use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use crate::buffer::{
    Buffer, display_names, icon_for_file, language_for_file, read_body, resolved_language,
};
use crate::navigation::{
    JumpEntry, JumpHistory, NavTarget, byte_offset, line_character, resolve_target, word_range,
};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext as _, ClipboardItem, Context, Entity, EntityId, Focusable as _,
    InteractiveElement as _, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    ParentElement as _, Pixels, Point, Render, Role, ScrollWheelEvent, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Task, WeakEntity, Window, canvas,
    div, point, rems,
};

use lithe_gpui_java::JavaLanguageService;
use lithe_gpui_shared::design::TAB_MAX_WIDTH;
use lithe_gpui_shared::icons::{idea, idea_icon_mono};
use lithe_gpui_shared::rem_px;
use lithe_gpui_shared::{tr, tr_args};
use lithe_ui::buttons::{Button, ButtonVariant};
use lithe_ui::editor::{
    CodeEditor, CompletionItem as UiCompletion, CompletionMenu, EditorEvent, PopupKey,
    completions as filtered_completions,
};
use lithe_ui::feedback::EmptyState;
use lithe_ui::layout::{h_stack, v_stack};
use lithe_ui::menus::{ContextMenu, Menu, MenuItem};
use lithe_ui::overlays::Dialog;
use lithe_ui::shell::{WindowRootExt as _, dismiss_hosted};
use lithe_ui::theme::{ActiveTheme as _, Size};

const FALLBACK_LINE_HEIGHT: f32 = 20.;

pub const EDITOR_HOST_ELEMENT_ID: &str = "lithe-editor-host";

const AUTO_SAVE_DEBOUNCE: Duration = Duration::from_millis(150);

const AUTO_SAVE_DEFAULT: bool = true;

const SOFT_WRAP_DEFAULT: bool = true;

const BREAKPOINT_GUTTER_WIDTH_SPEC: f32 = 14.;

const BREAKPOINT_MARKER_SIZE_SPEC: f32 = 14.;

const EDITOR_CONTENT_TOP_PADDING_SPEC: f32 = 8.;

/// 自动补全在哪些字符上开列表：标识符字符由库自己认，这两个是 Java 特有的入口
/// （`.` 成员访问、`@` 注解）——策略是产品设置的一部分，所以由宿主点名（见 `completion_triggers`）。
const COMPLETION_TRIGGERS: &str = ".@";

/// 一次翻页键在补全列表里走多少行。
const COMPLETION_PAGE: isize = 10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CursorPosition {
    pub line: usize,
    pub column: usize,
}

impl CursorPosition {
    const INITIAL: Self = Self { line: 1, column: 1 };
}

struct NavRequest {
    generation: u64,
    path: PathBuf,
    revision: u64,
    origin: lsp_types::Position,
}

pub struct TabMenuHostActions {
    pub reveal: Arc<dyn Fn(&Path, &mut Window, &mut App) + 'static>,
    pub reveal_in_tree: Arc<dyn Fn(&Path, &mut App) + 'static>,
    pub open_terminal: Arc<dyn Fn(&Path, &mut Window, &mut App) + 'static>,
}

pub type BufferWriteHost = Arc<dyn Fn(&Path, &mut App) + 'static>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GutterBreakpoint {
    pub line: usize,
    pub enabled: bool,
    pub verified: bool,
}

pub struct DebugGutterHost {
    pub toggle: Arc<dyn Fn(&Path, usize, &mut App) + 'static>,
    pub set_enabled: Arc<dyn Fn(&Path, usize, bool, &mut App) + 'static>,
    pub remove: Arc<dyn Fn(&Path, usize, &mut App) + 'static>,
    pub source_edit: Arc<dyn Fn(&Path, &str, &str, &mut App) + 'static>,
}

#[derive(Clone)]
enum CloseScope {
    Others(PathBuf),
    ToLeft(PathBuf),
    ToRight(PathBuf),
    All,
    Saved,
}

impl CloseScope {
    fn tag(&self) -> &'static str {
        match self {
            Self::Others(_) => "closeOthers",
            Self::ToLeft(_) => "closeLeft",
            Self::ToRight(_) => "closeRight",
            Self::All => "closeAll",
            Self::Saved => "closeSaved",
        }
    }

    fn targets(&self, buffers: &[Buffer]) -> Vec<usize> {
        match self {
            Self::Others(anchor) => buffers
                .iter()
                .enumerate()
                .filter(|(_, buffer)| &buffer.path != anchor)
                .map(|(index, _)| index)
                .collect(),
            Self::ToLeft(anchor) => {
                let Some(position) = buffers.iter().position(|buffer| &buffer.path == anchor)
                else {
                    return Vec::new();
                };
                (0..position).collect()
            }
            Self::ToRight(anchor) => {
                let Some(position) = buffers.iter().position(|buffer| &buffer.path == anchor)
                else {
                    return Vec::new();
                };
                (position + 1..buffers.len()).collect()
            }
            Self::All => (0..buffers.len()).collect(),
            Self::Saved => buffers
                .iter()
                .enumerate()
                .filter(|(_, buffer)| !buffer.is_dirty)
                .map(|(index, _)| index)
                .collect(),
        }
    }
}

#[derive(Clone)]
enum PendingConfirm {
    Single(usize),
    Batch(CloseScope),
    Reload(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRestore {
    pub opened: Vec<PathBuf>,
    pub active_index: Option<usize>,
}

struct GoToLinePrompt {
    input: Entity<lithe_ui::forms::InputState>,
    _enter: Subscription,
}

/// 正在显示的那一份补全列表。
///
/// `offset` 是请求发出时光标所在的字节偏移（弹层贴着它画），`selected` 是**过滤之后**的行号
/// （与组件库画出来的顺序一致），`items` 是服务端原样的那批（接受时按值回配）。
struct Completion {
    editor: EntityId,
    items: Vec<lithe_gpui_java::JavaCompletionItem>,
    rows: Vec<UiCompletion>,
    offset: usize,
    selected: usize,
    revision: u64,
}

pub struct EditorPane {
    notifications: Option<WeakEntity<lithe_gpui_notify::Store>>,
    buffers: Vec<Buffer>,
    active: Option<usize>,
    cursor: Option<CursorPosition>,
    history: JumpHistory,
    nav_request: Option<NavRequest>,
    nav_task: Option<Task<()>>,
    nav_generation: u64,
    diagnostics_generation: u64,
    java: Option<Arc<JavaLanguageService>>,
    java_task: Option<Task<()>>,
    problems: Entity<crate::problems::ProblemsPane>,
    workspace_root: Option<PathBuf>,
    tab_menu_actions: Option<TabMenuHostActions>,
    buffer_write_host: Option<BufferWriteHost>,
    gutter_breakpoints: BTreeMap<PathBuf, Vec<GutterBreakpoint>>,
    breakpoints_muted: bool,
    debug_gutter_host: Option<DebugGutterHost>,
    last_text: BTreeMap<PathBuf, String>,
    tab_size: usize,
    auto_completion: Rc<AtomicBool>,
    hovered_tab: Option<usize>,
    closed: Vec<PathBuf>,
    auto_save: bool,
    soft_wrap: bool,
    go_to_line: Option<GoToLinePrompt>,
    /// 补全：当前这一份、请求的代次（晚到的答案据此丢掉）、以及正在跑的任务。
    completion: Option<Completion>,
    completion_generation: Arc<AtomicU64>,
    completion_task: Option<Task<()>>,
    completion_revision: u64,
    /// 悬停：当前挂着的锚点（`None` 表示没画）、代次与任务。
    hover_offset: Option<usize>,
    hover_generation: u64,
    hover_task: Option<Task<()>>,
    /// 代码动作：服务端给的这一批、以及"请在第 N 次请求时于这个窗口坐标开菜单"。
    code_actions: Vec<lithe_gpui_java::JavaCodeAction>,
    code_action_request: Option<(u64, Point<Pixels>)>,
    code_action_generation: Arc<AtomicU64>,
    code_action_task: Option<Task<()>>,
    /// 正文容器（弹层的父容器）在窗口里的原点，由它自己的 canvas 量出来：`bounds_for_offset` 给的是
    /// **窗口**坐标，要画成这个容器的子元素就得减去它。
    body_origin: Option<Point<Pixels>>,
}

const DEFAULT_TAB_SIZE: usize = 2;
const MAX_TAB_SIZE: usize = 64;

const MAX_CLOSED_TABS: usize = 20;

impl EditorPane {
    pub fn new(
        notifications: Option<WeakEntity<lithe_gpui_notify::Store>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            notifications,
            buffers: Vec::new(),
            active: None,
            cursor: None,
            history: JumpHistory::default(),
            nav_request: None,
            nav_task: None,
            nav_generation: 0,
            diagnostics_generation: 0,
            java: None,
            java_task: None,
            problems: cx.new(|_| crate::problems::ProblemsPane::new()),
            workspace_root: None,
            tab_menu_actions: None,
            buffer_write_host: None,
            gutter_breakpoints: BTreeMap::new(),
            breakpoints_muted: false,
            debug_gutter_host: None,
            last_text: BTreeMap::new(),
            tab_size: DEFAULT_TAB_SIZE,
            auto_completion: Rc::new(AtomicBool::new(true)),
            auto_save: AUTO_SAVE_DEFAULT,
            soft_wrap: SOFT_WRAP_DEFAULT,
            go_to_line: None,
            completion: None,
            completion_generation: Arc::new(AtomicU64::new(0)),
            completion_task: None,
            completion_revision: 0,
            hover_offset: None,
            hover_generation: 0,
            hover_task: None,
            code_actions: Vec::new(),
            code_action_request: None,
            code_action_generation: Arc::new(AtomicU64::new(0)),
            code_action_task: None,
            body_origin: None,
            hovered_tab: None,
            closed: Vec::new(),
        }
    }

    /// 打开列表之前那一段（也就是光标所在缓冲区，若它在这个窗格里）。
    fn index_of(&self, editor: &Entity<CodeEditor>) -> Option<usize> {
        self.buffers
            .iter()
            .position(|buffer| buffer.editor.entity_id() == editor.entity_id())
    }

    fn index_of_id(&self, editor: EntityId) -> Option<usize> {
        self.buffers
            .iter()
            .position(|buffer| buffer.editor.entity_id() == editor)
    }

    /// 这个文件用不用 Java 语言服务：`.java` 且注册表认得它。
    fn java_for(&self, name: &str) -> Option<Arc<JavaLanguageService>> {
        (language_for_file(name) == Some("java"))
            .then(|| self.java.clone())
            .flatten()
    }

    pub fn set_auto_completion(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let changed = self.auto_completion.load(Ordering::Acquire) != enabled;
        self.auto_completion.store(enabled, Ordering::Release);
        for buffer in &self.buffers {
            buffer
                .editor
                .update(cx, |state, cx| state.set_auto_completion(enabled, cx));
        }
        println!(
            "S1_EDITOR_AUTO_COMPLETION enabled={} changed={} buffers={}",
            enabled,
            changed,
            self.buffers.len()
        );
        if changed {
            cx.notify();
        }
    }

    pub fn set_soft_wrap(&mut self, enabled: bool, _window: &mut Window, cx: &mut Context<Self>) {
        let changed = self.soft_wrap != enabled;
        self.soft_wrap = enabled;
        let mut applied = 0usize;
        for buffer in &self.buffers {
            buffer
                .editor
                .update(cx, |state, cx| state.set_soft_wrap(enabled, cx));
            applied += 1;
        }
        println!("S1_EDITOR_SOFT_WRAP enabled={enabled} changed={changed} buffers={applied}");
        if changed {
            cx.notify();
        }
    }

    pub fn soft_wrap_enabled(&self) -> bool {
        self.soft_wrap
    }

    pub fn set_auto_save(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let changed = self.auto_save != enabled;
        self.auto_save = enabled;
        let mut cancelled = 0usize;
        if !enabled {
            for buffer in &mut self.buffers {
                if buffer.auto_save_task.take().is_some() {
                    cancelled += 1;
                }
            }
        }
        println!(
            "S1_EDITOR_AUTO_SAVE enabled={enabled} changed={changed} cancelled={cancelled} buffers={}",
            self.buffers.len()
        );
        if changed {
            cx.notify();
        }
    }

    pub fn auto_save_enabled(&self) -> bool {
        self.auto_save
    }

    pub fn set_tab_size(&mut self, tab_size: usize, cx: &mut Context<Self>) {
        let tab_size = tab_size.clamp(1, MAX_TAB_SIZE);
        if self.tab_size == tab_size {
            return;
        }
        self.tab_size = tab_size;
        for buffer in &self.buffers {
            buffer
                .editor
                .update(cx, |state, cx| state.set_tab_size(tab_size, cx));
        }
        println!(
            "S1_EDITOR_TAB_SIZE size={} buffers={}",
            self.tab_size,
            self.buffers.len()
        );
        cx.notify();
    }

    pub fn prepare_java(&mut self, workspace_root: PathBuf, cx: &mut Context<Self>) {
        if self.java.is_some() {
            return;
        }
        let service = Arc::new(JavaLanguageService::new(workspace_root.clone()));
        self.java = Some(service.clone());

        let view = cx.entity().downgrade();
        let problems = self.problems.clone();
        problems.update(cx, |pane, cx| {
            pane.set_service(service.clone(), workspace_root.clone(), cx);
            pane.set_open(Arc::new(move |path, line, utf16_column, window, cx| {
                let Some(view) = view.upgrade() else {
                    return;
                };
                view.update(cx, |pane, cx| {
                    pane.open_at(path, line, utf16_column, "problems", window, cx);
                });
            }));
        });

        // Java 缓冲区一开就接上了语言服务（见 `open`），这里只补一件事：服务就绪之后把诊断要一遍。
        let java_buffers: Vec<usize> = self
            .buffers
            .iter()
            .enumerate()
            .filter(|(_, buffer)| language_for_file(&buffer.name) == Some("java"))
            .map(|(index, _)| index)
            .collect();
        for index in java_buffers {
            self.schedule_diagnostics(index, Duration::ZERO, cx);
        }

        self.java_task = Some(cx.background_spawn(async move {
            match service.prepare() {
                Ok(true) => println!("S1_JAVA_PREPARED"),
                Ok(false) => {}
                Err(reason) => println!("S1_JAVA_PREPARE_FAILED reason={reason}"),
            }
        }));
    }

    fn schedule_diagnostics(&mut self, index: usize, debounce: Duration, cx: &mut Context<Self>) {
        let Some(service) = self.java.clone() else {
            return;
        };
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        if !buffer.writable
            || is_virtual_source_path(&buffer.path)
            || language_for_file(&buffer.name) != Some("java")
        {
            return;
        }

        let path = buffer.path.clone();
        let revision = buffer.revision;
        let text = buffer.editor.read(cx).text().to_string();

        let generation = self.diagnostics_generation.wrapping_add(1);
        self.diagnostics_generation = generation;
        self.buffers[index].diagnostics_generation = generation;

        let executor = cx.background_executor().clone();
        let task_path = path.clone();
        let task = cx.spawn(async move |pane, cx| {
            let fetched = cx
                .background_spawn(crate::diagnostics::fetch(
                    service,
                    task_path.clone(),
                    text,
                    executor,
                    debounce,
                ))
                .await;
            let _ = pane.update(cx, |pane, cx| {
                pane.apply_diagnostics(generation, revision, task_path, fetched, cx)
            });
        });
        self.buffers[index].diagnostics_task = Some(task);
    }

    fn apply_diagnostics(
        &mut self,
        generation: u64,
        revision: u64,
        path: PathBuf,
        fetched: crate::diagnostics::Fetched,
        cx: &mut Context<Self>,
    ) {
        let file = path.display();
        let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) else {
            println!("S1_EDITOR_DIAGNOSTICS file={file} result=stale reason=buffer-closed");
            return;
        };
        if self.buffers[index].diagnostics_generation != generation {
            println!("S1_EDITOR_DIAGNOSTICS file={file} result=stale reason=superseded");
            return;
        }
        if self.buffers[index].revision != revision {
            println!("S1_EDITOR_DIAGNOSTICS file={file} result=stale reason=content-changed");
            return;
        }

        let written = self.buffers[index].editor.update(cx, |state, cx| {
            crate::diagnostics::apply(state, &fetched.diagnostics, cx)
        });

        if let Some(error) = &fetched.sync_error {
            println!("S1_EDITOR_DIAGNOSTICS sync_failed file={file} error={error}");
        }
        println!(
            "S1_EDITOR_DIAGNOSTICS file={file} count={written} severity_max={} attempts={} ms={} revision={revision}",
            crate::diagnostics::severity_max(&fetched.diagnostics),
            fetched.attempts,
            fetched.elapsed_ms,
        );

        self.refresh_problems(cx);
    }

    pub fn problems_pane(&self) -> Entity<crate::problems::ProblemsPane> {
        self.problems.clone()
    }

    pub fn refresh_problems(&mut self, cx: &mut Context<Self>) {
        self.problems.update(cx, |pane, cx| pane.refresh(cx));
    }

    pub fn open_at(
        &mut self,
        path: &Path,
        line: u32,
        utf16_column: u32,
        origin_kind: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let origin = self
            .active
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| {
                let state = buffer.editor.read(cx);
                JumpEntry {
                    path: buffer.path.clone(),
                    position: line_character(state.text(), state.cursor_offset()),
                }
            });

        if !self.buffers.iter().any(|buffer| buffer.path == path) {
            self.open(path, window, cx);
        }
        let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) else {
            return false;
        };

        let text = self.buffers[index].editor.read(cx).text().to_string();
        let to = self.buffers[index].editor.update(cx, |state, cx| {
            let offset =
                crate::navigation::offset_for_utf16(&text, line as usize, utf16_column as usize);
            state.set_cursor_offset(offset, window, cx);
            state.position()
        });
        if let Some(origin) = origin {
            self.history.record(origin);
        }
        self.active = Some(index);

        println!(
            "S1_NAV_JUMP to={}:{} kind={} file={}",
            to.0,
            to.1,
            origin_kind,
            self.buffers[index].path.display()
        );
        self.sync_cursor(cx);
        cx.notify();
        true
    }

    pub fn set_tab_menu_host_actions(&mut self, actions: TabMenuHostActions) {
        self.tab_menu_actions = Some(actions);
    }

    pub fn set_buffer_write_host(&mut self, host: BufferWriteHost) {
        self.buffer_write_host = Some(host);
    }

    pub fn java_service(&self) -> Option<Arc<JavaLanguageService>> {
        self.java.clone()
    }

    pub fn set_gutter_breakpoints(
        &mut self,
        breakpoints: BTreeMap<PathBuf, Vec<GutterBreakpoint>>,
        muted: bool,
    ) {
        if self.gutter_breakpoints == breakpoints && self.breakpoints_muted == muted {
            return;
        }
        self.gutter_breakpoints = breakpoints;
        self.breakpoints_muted = muted;
    }

    pub fn set_debug_gutter_host(&mut self, host: DebugGutterHost) {
        self.debug_gutter_host = Some(host);
    }

    pub fn set_workspace_root(&mut self, root: PathBuf) {
        self.workspace_root = Some(root);
    }

    pub fn open(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.to_path_buf();

        if let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) {
            self.activate(index, cx);
            return;
        }

        let name: SharedString = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string())
            .into();
        let body = read_body(&path, &name);
        let writable = body.writable;
        let language = resolved_language(&name);
        let tab_size = self.tab_size;
        let auto_completion = self.auto_completion.load(Ordering::Acquire);
        let text = body.text;

        let editor = cx.new(|cx| {
            let mut state = CodeEditor::new(text, window, cx)
                .tab_size(tab_size)
                .auto_completion(auto_completion)
                .completion_triggers(COMPLETION_TRIGGERS);
            if !writable {
                state = state.read_only();
            }
            if !self.soft_wrap {
                state = state.soft_wrap(false);
            }
            state.set_language(language, cx);
            state
        });
        println!(
            "S1_EDITOR_LANG name={} file={name}",
            language.map_or("plain-text", |language| language.id)
        );

        let subscriptions = self.subscribe_to(&editor, window, cx);
        let icon = icon_for_file(&name, cx);
        let _ = self
            .last_text
            .insert(path.clone(), editor.read(cx).text().to_string());
        self.buffers.push(Buffer::new(
            path,
            name,
            icon,
            editor,
            writable,
            subscriptions,
        ));
        self.active = Some(self.buffers.len() - 1);
        self.sync_cursor(cx);
        if language_for_file(&self.buffers[self.buffers.len() - 1].name) == Some("java") {
            let opened = self.buffers.len() - 1;
            self.schedule_diagnostics(opened, Duration::ZERO, cx);
        }
        cx.notify();
    }

    /// 编辑器 → 宿主的那两条订阅：事件（改变了、要补全、要悬停、弹层按键）与状态变化（光标、滚动）。
    fn subscribe_to(
        &self,
        editor: &Entity<CodeEditor>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<Subscription> {
        vec![
            cx.subscribe_in(
                editor,
                window,
                |pane: &mut Self, editor: &Entity<CodeEditor>, event: &EditorEvent, window, cx| {
                    pane.on_editor_event(editor, event, window, cx)
                },
            ),
            cx.observe(editor, |pane: &mut Self, editor, cx| {
                pane.on_editor_changed(&editor, cx)
            }),
        ]
    }

    fn on_editor_event(
        &mut self,
        editor: &Entity<CodeEditor>,
        event: &EditorEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            EditorEvent::Changed => self.on_text_changed(editor, window, cx),
            EditorEvent::CompletionRequested(offset) => {
                self.ask_for_completions(editor, *offset, cx);
            }
            EditorEvent::HoverRequested(offset) => self.ask_for_hover(editor, *offset, cx),
            EditorEvent::PopupKey(key) => self.on_popup_key(editor, *key, cx),
            // 组件库的只读横幅带一个"Make editable"按钮，而本产品的只读是**文件属性**（见 `refuse_unlock`）。
            EditorEvent::Unlock => self.refuse_unlock(editor, cx),
            _ => {}
        }
    }

    /// 编辑器的状态变了（光标动了、滚了、重画了）：把状态栏的光标同步一下，并收拾掉"锚点已经看不见
    /// 的弹层"——库不会自己收，滚动之后挂在视口外的补全列表必须由这里关掉。
    fn on_editor_changed(&mut self, editor: &Entity<CodeEditor>, cx: &mut Context<Self>) {
        self.sync_cursor(cx);
        let Some(completion) = self.completion.as_ref() else {
            return;
        };
        if editor
            .read(cx)
            .bounds_for_offset(completion.offset, cx)
            .is_some()
        {
            return;
        }
        println!("S1_EDITOR_COMPLETION open=false reason=anchor-out-of-view");
        self.dismiss_completion(editor, cx);
    }

    pub fn active_buffer_name(&self) -> String {
        self.active
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| buffer.name.to_string())
            .unwrap_or_default()
    }

    pub fn active_buffer_path(&self) -> Option<PathBuf> {
        self.active
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| buffer.path.clone())
    }

    pub fn tab_size(&self) -> usize {
        self.tab_size
    }

    pub fn active_buffer_is_readonly(&self) -> bool {
        self.active
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| !buffer.writable)
            .unwrap_or(true)
    }

    pub fn open_disk_files(&self) -> Vec<PathBuf> {
        self.buffers
            .iter()
            .filter(|buffer| !is_virtual_source_path(&buffer.path))
            .map(|buffer| buffer.path.clone())
            .collect()
    }

    pub fn active_disk_file(&self) -> Option<PathBuf> {
        self.active
            .and_then(|index| self.buffers.get(index))
            .filter(|buffer| !is_virtual_source_path(&buffer.path))
            .map(|buffer| buffer.path.clone())
    }

    pub fn restore_session(
        &mut self,
        paths: &[PathBuf],
        active: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> SessionRestore {
        for path in paths {
            self.open(path, window, cx);
        }

        let opened = self
            .buffers
            .iter()
            .filter(|buffer| paths.contains(&buffer.path))
            .map(|buffer| buffer.path.clone())
            .collect::<Vec<_>>();
        let mut active_index = active.filter(|index| *index < opened.len()).or_else(|| {
            self.active_disk_file()
                .and_then(|path| opened.iter().position(|candidate| *candidate == path))
        });
        if active_index.is_none() && active.is_none() && !opened.is_empty() {
            active_index = Some(0);
        }
        if let Some(index) = active_index {
            if let Some(path) = opened.get(index) {
                if let Some(position) = self.buffers.iter().position(|buffer| buffer.path == *path)
                {
                    self.activate(position, cx);
                }
            }
        }

        SessionRestore {
            opened,
            active_index,
        }
    }

    pub fn cursor_position(&self, cx: &App) -> CursorPosition {
        self.active
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| {
                let (line, column) = buffer.editor.read(cx).position();
                CursorPosition { line, column }
            })
            .unwrap_or(CursorPosition::INITIAL)
    }

    pub fn save_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            return;
        };
        if !self
            .buffers
            .get(index)
            .is_some_and(|buffer| buffer.writable)
        {
            return;
        }
        self.write_buffer(index, "lithe.editor.saveFailed", window, cx);
    }

    pub fn navigate_to_definition(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            println!("S1_NAV_FAILED reason=no-buffer");
            return;
        };
        let Some(buffer) = self.buffers.get(index) else {
            println!("S1_NAV_FAILED reason=no-buffer");
            return;
        };

        let path = buffer.path.clone();
        let revision = buffer.revision;
        let file_path = path.to_string_lossy().to_string();
        let (offset, origin, text) = {
            let state = buffer.editor.read(cx);
            let text = state.text().to_string();
            let offset = state.cursor_offset();
            (offset, line_character(&text, offset), text)
        };
        let java = self.java.clone();

        let generation = self.nav_generation.wrapping_add(1);
        self.nav_generation = generation;
        self.nav_request = Some(NavRequest {
            generation,
            path: path.clone(),
            revision,
            origin,
        });

        let task = cx.spawn_in(window, async move |pane, cx| {
            let result = cx
                .background_spawn(async move {
                    resolve_target(java.as_deref(), &file_path, &text, offset)
                })
                .await;
            let _ = pane.update_in(cx, |pane, window, cx| {
                pane.apply_definition(generation, result, window, cx)
            });
        });
        self.nav_task = Some(task);
    }

    fn apply_definition(
        &mut self,
        generation: u64,
        result: Result<Option<NavTarget>, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((origin_path, revision, origin)) = self
            .nav_request
            .as_ref()
            .filter(|request| request.generation == generation)
            .map(|request| (request.path.clone(), request.revision, request.origin))
        else {
            println!("S1_NAV_FAILED reason=stale");
            return;
        };

        let Some(origin_index) = self
            .buffers
            .iter()
            .position(|buffer| buffer.path == origin_path)
        else {
            println!("S1_NAV_FAILED reason=buffer-closed");
            return;
        };
        if self.buffers[origin_index].revision != revision {
            println!("S1_NAV_FAILED reason=content-changed");
            return;
        }

        let target = match result {
            Ok(Some(target)) => target,
            Ok(None) => {
                println!("S1_NAV_FAILED reason=no-target");
                self.notify_no_target(cx);
                return;
            }
            Err(error) => {
                println!("S1_NAV_FAILED reason={error}");
                self.notify_no_target(cx);
                return;
            }
        };

        let (index, offset, via) = match target {
            NavTarget::Local(position) => {
                let text = self.buffers[origin_index]
                    .editor
                    .read(cx)
                    .text()
                    .to_string();
                (origin_index, byte_offset(&text, position), "builtin")
            }
            NavTarget::File {
                path,
                line,
                utf16_column,
            } => {
                if !self.buffers.iter().any(|buffer| buffer.path == path) {
                    self.open(&path, window, cx);
                }
                let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) else {
                    println!("S1_NAV_FAILED reason=no-target");
                    self.notify_no_target(cx);
                    return;
                };
                let text = self.buffers[index].editor.read(cx).text().to_string();
                (
                    index,
                    crate::navigation::offset_for_utf16(
                        &text,
                        line as usize,
                        utf16_column as usize,
                    ),
                    "jdtls",
                )
            }
            NavTarget::Virtual {
                uri,
                display_path,
                text,
                line,
                utf16_column,
            } => {
                let offset = crate::navigation::offset_for_utf16(
                    &text,
                    line as usize,
                    utf16_column as usize,
                );
                let index = self.open_virtual(&uri, &display_path, text, window, cx);
                (index, offset, "jdtls-virtual")
            }
        };

        self.history.record(JumpEntry {
            path: origin_path,
            position: origin,
        });
        let to = self.buffers[index].editor.update(cx, |state, cx| {
            state.set_cursor_offset(offset, window, cx);
            state.position()
        });
        self.active = Some(index);

        println!(
            "S1_NAV_JUMP from={}:{} to={}:{} kind=definition via={via} file={}",
            origin.line + 1,
            origin.character + 1,
            to.0,
            to.1,
            self.buffers[index].path.display()
        );
        self.sync_cursor(cx);
        cx.notify();
    }

    fn on_editor_click(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !event.modifiers.secondary() || event.modifiers.alt {
            return;
        }

        // 点在正文里（而不是行号槽、标签栏或空白）：`offset_at` 给不出偏移就说明这一点不在代码行上。
        let inside = self
            .active
            .and_then(|index| self.buffers.get(index))
            .is_some_and(|buffer| {
                buffer
                    .editor
                    .read(cx)
                    .offset_at(event.position, cx)
                    .is_some()
            });
        if !inside {
            return;
        }

        self.navigate_to_definition(window, cx);
    }

    /// 指针在正文里移动：离开当前悬停的那个词就把面板收掉（库不报"离开"，只报"进了新的符号"）。
    fn on_editor_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let Some(anchor) = self.hover_offset else {
            return;
        };
        let Some(index) = self.active else {
            return;
        };
        let editor = self.buffers[index].editor.clone();
        let still = {
            let state = editor.read(cx);
            state.offset_at(event.position, cx).is_some_and(|at| {
                word_range(state.text(), at).is_some_and(|word| word.start == anchor)
            })
        };
        if !still {
            self.clear_hover(&editor, cx);
        }
    }

    /// 指针离开正文容器（整个编辑器）：挂着的悬停内容同样收掉。
    fn on_pointer_left_editor(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            return;
        };
        let editor = self.buffers[index].editor.clone();
        self.clear_hover(&editor, cx);
    }

    fn open_virtual(
        &mut self,
        uri: &str,
        display_path: &str,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        let path = PathBuf::from(uri);
        if let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) {
            return index;
        }

        let name: SharedString = display_path
            .rsplit('/')
            .find(|segment| !segment.is_empty())
            .unwrap_or(display_path)
            .to_string()
            .into();
        let language = resolved_language(&name);
        let tab_size = self.tab_size;

        let editor = cx.new(|cx| {
            let mut state = CodeEditor::new(text, window, cx)
                .tab_size(tab_size)
                .read_only();
            if !self.soft_wrap {
                state = state.soft_wrap(false);
            }
            state.set_language(language, cx);
            state
        });
        println!(
            "S1_EDITOR_LANG name={} file={name} virtual=true",
            language.map_or("plain-text", |language| language.id)
        );

        let subscriptions = self.subscribe_to(&editor, window, cx);
        let icon = icon_for_file(&name, cx);
        self.buffers
            .push(Buffer::new(path, name, icon, editor, false, subscriptions));
        self.buffers.len() - 1
    }

    #[allow(dead_code)]
    fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(present) = self.current_entry(cx) else {
            return;
        };
        let Some(entry) = self.history.go_back(present.clone()) else {
            return;
        };
        self.restore_entry(present, entry, "S1_NAV_BACK", window, cx);
    }

    #[allow(dead_code)]
    fn go_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(present) = self.current_entry(cx) else {
            return;
        };
        let Some(entry) = self.history.go_forward() else {
            return;
        };
        self.restore_entry(present, entry, "S1_NAV_FORWARD", window, cx);
    }

    fn restore_entry(
        &mut self,
        from: JumpEntry,
        entry: JumpEntry,
        log_tag: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.buffers.iter().any(|buffer| buffer.path == entry.path) {
            if is_virtual_source_path(&entry.path) {
                println!("S1_NAV_FAILED reason=buffer-closed");
                return;
            }
            self.open(&entry.path, window, cx);
        }
        let Some(index) = self
            .buffers
            .iter()
            .position(|buffer| buffer.path == entry.path)
        else {
            println!("S1_NAV_FAILED reason=buffer-closed");
            return;
        };

        let to = self.buffers[index].editor.update(cx, |state, cx| {
            let offset = byte_offset(state.text(), entry.position);
            state.set_cursor_offset(offset, window, cx);
            state.position()
        });
        self.active = Some(index);

        println!(
            "{log_tag} from={}:{} to={}:{}",
            from.position.line + 1,
            from.position.character + 1,
            to.0,
            to.1
        );
        self.sync_cursor(cx);
        cx.notify();
    }

    fn current_entry(&self, cx: &App) -> Option<JumpEntry> {
        let index = self.active?;
        let buffer = self.buffers.get(index)?;
        let state = buffer.editor.read(cx);
        Some(JumpEntry {
            path: buffer.path.clone(),
            position: line_character(state.text(), state.cursor_offset()),
        })
    }

    fn notify_no_target(&self, cx: &mut Context<Self>) {
        let target = tr("lithe.navigation.definition");
        let code = "lithe.navigation.noTargetFound";
        if !self.record_notification(
            cx,
            lithe_gpui_notify::Input::new(code, lithe_gpui_notify::Severity::Info)
                .param("target", target.as_ref())
                .key("navigation-no-target"),
        ) {
            eprintln!("S1_NOTIFICATION action=record state=dropped reason=no_store code={code}");
        }
        cx.notify();
    }

    fn record_notification(&self, cx: &mut Context<Self>, input: lithe_gpui_notify::Input) -> bool {
        match self.notifications.as_ref().and_then(WeakEntity::upgrade) {
            Some(store) => {
                store.update(cx, |store, _| {
                    store.record(input, lithe_gpui_notify::now_ms());
                });
                true
            }
            None => false,
        }
    }

    fn activate(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.buffers.len() {
            return;
        }
        self.active = Some(index);
        eprintln!(
            "S1_EDITOR_ACTIVE index={index} file={}",
            self.buffers[index].name
        );
        self.sync_cursor(cx);
        cx.notify();
    }

    fn sync_cursor(&mut self, cx: &mut Context<Self>) {
        let position = self.cursor_position(cx);
        if self.cursor == Some(position) {
            return;
        }
        self.cursor = Some(position);
        println!(
            "S1_EDITOR_CURSOR line={} col={}",
            position.line, position.column
        );
        cx.notify();
    }

    /// 正文变了：脏标记、修订号、诊断、自动保存、调试宿主的回调，再加上"库不会替我们清的"那两样
    /// ——过期的诊断波浪线（上游的 `DiagnosticSet` 也是在正文变化时 reset 的）与悬停内容，
    /// 以及补全列表要跟着新前缀收窄。
    fn on_text_changed(
        &mut self,
        editor: &Entity<CodeEditor>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.index_of(editor) else {
            return;
        };
        if !self.buffers[index].writable {
            return;
        }

        if !self.buffers[index].is_dirty {
            self.buffers[index].is_dirty = true;
            cx.notify();
        }
        self.buffers[index].revision = self.buffers[index].revision.wrapping_add(1);

        self.schedule_diagnostics(index, crate::diagnostics::CHANGE_DEBOUNCE, cx);

        if self.auto_save {
            self.schedule_auto_save(index, window, cx);
        }

        let path = self.buffers[index].path.clone();
        let after = self.buffers[index].editor.read(cx).text().to_string();
        match self.last_text.insert(path.clone(), after.clone()) {
            Some(before) if before != after => {
                if let Some(host) = self
                    .debug_gutter_host
                    .as_ref()
                    .map(|host| host.source_edit.clone())
                {
                    (host)(&path, &before, &after, cx);
                }
            }
            _ => {}
        }

        self.clear_hover(editor, cx);
        editor.update(cx, |state, cx| state.set_diagnostics(Vec::new(), cx));
        self.narrow_completion(editor, cx);
    }

    fn schedule_auto_save(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let generation = self.buffers[index].save_generation.wrapping_add(1);
        self.buffers[index].save_generation = generation;

        let task = cx.spawn_in(window, async move |pane, cx| {
            cx.background_executor().timer(AUTO_SAVE_DEBOUNCE).await;
            let _ = pane.update_in(cx, |pane, window, cx| {
                pane.run_auto_save(index, generation, window, cx)
            });
        });
        self.buffers[index].auto_save_task = Some(task);
    }

    fn run_auto_save(
        &mut self,
        index: usize,
        generation: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        if buffer.save_generation != generation || !buffer.is_dirty {
            return;
        }
        self.write_buffer(index, "lithe.editor.autoSaveFailed", window, cx);
    }

    fn write_buffer(
        &mut self,
        index: usize,
        failure_key: &'static str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(buffer) = self.buffers.get(index) else {
            return false;
        };
        let path = buffer.path.clone();
        let name = buffer.name.clone();
        let value = buffer.editor.read(cx).text().to_string();

        match std::fs::write(&path, value.as_bytes()) {
            Ok(()) => {
                if let Some(buffer) = self.buffers.get_mut(index) {
                    buffer.is_dirty = false;
                    buffer.auto_save_task = None;
                }
                println!(
                    "S1_EDITOR_SAVED path={} bytes={}",
                    path.display(),
                    value.len()
                );
                match self.buffer_write_host.as_ref().map(|host| host.clone()) {
                    Some(host) => (host)(&path, cx),
                    None => eprintln!("S1_EDITOR_WRITE_HOST missing=1 path={}", path.display()),
                }
                cx.notify();
                true
            }
            Err(error) => {
                eprintln!(
                    "S1_EDITOR_SAVE_FAILED path={} error={error}",
                    path.display()
                );
                if !self.record_notification(
                    cx,
                    lithe_gpui_notify::Input::new(failure_key, lithe_gpui_notify::Severity::Error)
                        .param("name", name.as_ref())
                        .param("error", error.to_string())
                        .key(failure_key),
                ) {
                    eprintln!(
                        "S1_NOTIFICATION action=record state=dropped reason=no_store code={failure_key}"
                    );
                }
                cx.notify();
                false
            }
        }
    }

    fn close(&mut self, index: usize) {
        if index >= self.buffers.len() {
            return;
        }

        let closed = self.buffers[index].path.clone();
        if !is_virtual_source_path(&closed) {
            self.closed.push(closed);
            if self.closed.len() > MAX_CLOSED_TABS {
                self.closed.remove(0);
            }
        }

        let shift = |active: usize| if active > index { active - 1 } else { active };
        self.buffers.remove(index);

        self.active = if self.buffers.is_empty() {
            None
        } else {
            Some(
                self.active
                    .map(shift)
                    .unwrap_or(index)
                    .min(self.buffers.len() - 1),
            )
        };
    }

    fn request_close(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };

        if !buffer.is_dirty {
            eprintln!(
                "S1_TAB_MENU run=close tab={} dirty=false",
                buffer.name.as_ref()
            );
            self.close(index);
            self.sync_cursor(cx);
            cx.notify();
            return;
        }

        let body = tr_args(
            "lithe.editor.gpui.unsavedChangesBody",
            &[("name", buffer.name.as_ref())],
        );
        self.open_unsaved_dialog(PendingConfirm::Single(index), body, window, cx);
    }

    pub fn close_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            return;
        };
        self.request_close(index, window, cx);
    }

    fn active_path(&self) -> Option<PathBuf> {
        self.active
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| buffer.path.clone())
    }

    fn close_scope_at_active(
        &mut self,
        make: impl FnOnce(PathBuf) -> CloseScope,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(anchor) = self.active_path() else {
            eprintln!("S1_TAB_MENU run=close reason=no-active");
            return;
        };
        self.close_scope(make(anchor), window, cx);
    }

    pub fn close_other_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_scope_at_active(CloseScope::Others, window, cx);
    }

    pub fn close_tabs_to_left(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_scope_at_active(CloseScope::ToLeft, window, cx);
    }

    pub fn close_tabs_to_right(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_scope_at_active(CloseScope::ToRight, window, cx);
    }

    pub fn close_all_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_scope(CloseScope::All, window, cx);
    }

    pub fn close_saved_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_scope(CloseScope::Saved, window, cx);
    }

    pub fn reopen_closed_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.closed.pop() else {
            eprintln!("S1_TAB_MENU run=reopenClosed result=empty");
            return;
        };
        eprintln!("S1_TAB_MENU run=reopenClosed path={}", path.display());
        self.open(&path, window, cx);
    }

    fn copy_path(&self, index: usize, cx: &mut App) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        let text = buffer.path.to_string_lossy().to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        eprintln!("S1_TAB_MENU run=copyPath value={text}");
    }

    fn copy_relative_path(&self, index: usize, cx: &mut App) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        let text = relative_to_workspace(&buffer.path, self.workspace_root.as_deref());
        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        eprintln!("S1_TAB_MENU run=copyRelativePath value={text}");
    }

    fn reveal_in_project_tree(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        let path = buffer.path.clone();
        let Some(reveal) = self
            .tab_menu_actions
            .as_ref()
            .map(|actions| actions.reveal_in_tree.clone())
        else {
            eprintln!("S1_TAB_MENU run=revealInTree result=no-host");
            return;
        };
        (reveal)(&path, cx);
        eprintln!("S1_TAB_MENU run=revealInTree path={}", path.display());
    }

    fn reveal_in_explorer(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        let path = buffer.path.clone();
        let Some(reveal) = self
            .tab_menu_actions
            .as_ref()
            .map(|actions| actions.reveal.clone())
        else {
            eprintln!("S1_TAB_MENU run=reveal result=no-host");
            return;
        };
        (reveal)(&path, window, cx);
        eprintln!("S1_TAB_MENU run=reveal path={}", path.display());
    }

    fn open_in_terminal(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        let path = buffer.path.clone();
        let Some(open_terminal) = self
            .tab_menu_actions
            .as_ref()
            .map(|actions| actions.open_terminal.clone())
        else {
            eprintln!("S1_TAB_MENU run=openInTerminal result=no-host");
            return;
        };
        (open_terminal)(&path, window, cx);
    }

    fn request_reload(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        if !buffer.is_dirty {
            self.reload_buffer(index, window, cx);
            return;
        }
        let body = tr_args(
            "lithe.editor.gpui.unsavedChangesBody",
            &[("name", buffer.name.as_ref())],
        );
        self.open_unsaved_dialog(PendingConfirm::Reload(index), body, window, cx);
    }

    fn reload_buffer(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get_mut(index) else {
            return;
        };
        let path = buffer.path.clone();
        let name = buffer.name.clone();
        let body = read_body(&path, &name);
        let writable = body.writable;
        let bytes = body.text.len();

        buffer.is_dirty = false;
        buffer.writable = writable;
        buffer.revision = buffer.revision.wrapping_add(1);
        buffer.save_generation = buffer.save_generation.wrapping_add(1);
        buffer.auto_save_task = None;

        let editor = buffer.editor.clone();
        editor.update(cx, |state, cx| {
            state.set_text(body.text, cx);
            state.set_read_only(!writable, cx);
        });

        eprintln!(
            "S1_TAB_MENU run=reload path={} bytes={bytes}",
            path.display()
        );
        self.sync_cursor(cx);
        self.schedule_diagnostics(index, Duration::ZERO, cx);
        cx.notify();
    }

    fn close_scope(&mut self, scope: CloseScope, window: &mut Window, cx: &mut Context<Self>) {
        let targets = scope.targets(&self.buffers);
        if targets.is_empty() {
            eprintln!("S1_TAB_MENU run={} closed=0 reason=empty", scope.tag());
            return;
        }

        let mut dirty = 0usize;
        for index in &targets {
            if self.buffers[*index].is_dirty {
                dirty += 1;
            }
        }
        if dirty == 0 {
            let closed = self.apply_close_scope(&scope, cx);
            eprintln!("S1_TAB_MENU run={} closed={closed} dirty=0", scope.tag());
            return;
        }

        eprintln!(
            "S1_TAB_MENU run={} confirm=true dirty={dirty} targets={}",
            scope.tag(),
            targets.len()
        );
        let count = dirty.to_string();
        let body = tr_args(
            "lithe.editor.gpui.unsavedChangesBatchBody",
            &[("count", &count)],
        );
        self.open_unsaved_dialog(PendingConfirm::Batch(scope), body, window, cx);
    }

    fn apply_close_scope(&mut self, scope: &CloseScope, cx: &mut Context<Self>) -> usize {
        let targets = scope.targets(&self.buffers);
        for index in targets.iter().rev() {
            self.close(*index);
        }
        self.sync_cursor(cx);
        cx.notify();
        targets.len()
    }

    fn open_unsaved_dialog(
        &self,
        pending: PendingConfirm,
        body: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = cx.weak_entity();
        let save_label = match &pending {
            PendingConfirm::Batch(_) => tr("lithe.menu.saveAll"),
            _ => tr("lithe.ui.save"),
        };

        window.open_dialog(
            "lithe-editor-unsaved",
            move |id, _window, cx| {
                let discard_pane = pane.clone();
                let save_pane = pane.clone();
                let discard_pending = pending.clone();
                let save_pending = pending.clone();
                let body = body.clone();
                let save_label = save_label.clone();
                let (cancel_id, discard_id, save_id) = (
                    (id.clone(), "cancel"),
                    (id.clone(), "discard"),
                    (id.clone(), "save"),
                );
                Dialog::new(id)
                    .title(tr("lithe.unsavedChanges.title"))
                    .on_close(dismiss_hosted)
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().colors.fg_muted)
                            .child(body),
                    )
                    .action(move |close| {
                        Button::new(cancel_id)
                            .label(tr("lithe.ui.cancel"))
                            .on_click(move |_, window, cx| close(window, cx))
                    })
                    .action(move |close| {
                        Button::new(discard_id)
                            .label(tr("lithe.editor.gpui.discardChanges"))
                            .on_click(move |_, window, cx| {
                                let pending = discard_pending.clone();
                                let _ = discard_pane
                                    .update(cx, |pane, cx| pane.apply_confirm(pending, window, cx));
                                close(window, cx);
                            })
                    })
                    .action(move |close| {
                        Button::new(save_id).primary().label(save_label).on_click(
                            move |_, window, cx| {
                                let pending = save_pending.clone();
                                let saved = save_pane
                                    .update(cx, |pane, cx| pane.save_confirm(pending, window, cx))
                                    .unwrap_or(false);
                                if saved {
                                    close(window, cx);
                                }
                            },
                        )
                    })
                    .into_any_element()
            },
            cx,
        );
    }

    fn apply_confirm(
        &mut self,
        pending: PendingConfirm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match pending {
            PendingConfirm::Single(index) => {
                self.close(index);
                self.sync_cursor(cx);
                cx.notify();
            }
            PendingConfirm::Batch(scope) => {
                let closed = self.apply_close_scope(&scope, cx);
                eprintln!(
                    "S1_TAB_MENU run={} closed={closed} via=discard",
                    scope.tag()
                );
            }
            PendingConfirm::Reload(index) => self.reload_buffer(index, window, cx),
        }
    }

    fn save_confirm(
        &mut self,
        pending: PendingConfirm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        match pending {
            PendingConfirm::Single(index) => {
                let saved = self.write_buffer(index, "lithe.editor.saveFailed", window, cx);
                if saved {
                    self.close(index);
                    self.sync_cursor(cx);
                    cx.notify();
                }
                saved
            }
            PendingConfirm::Batch(scope) => {
                let targets = scope.targets(&self.buffers);
                for index in &targets {
                    let dirty = self
                        .buffers
                        .get(*index)
                        .is_some_and(|buffer| buffer.is_dirty);
                    if dirty && !self.write_buffer(*index, "lithe.editor.saveFailed", window, cx) {
                        return false;
                    }
                }
                let closed = self.apply_close_scope(&scope, cx);
                eprintln!("S1_TAB_MENU run={} closed={closed} via=save", scope.tag());
                true
            }
            PendingConfirm::Reload(index) => {
                let saved = self.write_buffer(index, "lithe.editor.saveFailed", window, cx);
                if saved {
                    self.reload_buffer(index, window, cx);
                }
                saved
            }
        }
    }

    pub fn open_go_to_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            eprintln!("S1_EDITOR_GO_TO_LINE open=false reason=no-active-buffer");
            return;
        };
        if window.has_active_dialog(cx) {
            eprintln!("S1_EDITOR_GO_TO_LINE open=false reason=active-dialog");
            return;
        }

        let (text, offset) = {
            let state = self.buffers[index].editor.read(cx);
            (state.text().to_string(), state.cursor_offset())
        };
        let (line, utf16_column) = crate::navigation::core_position(&text, offset);
        let initial = crate::go_to_line::GoToLineInput::prefill(line as usize, utf16_column);

        let input = cx.new(|cx| lithe_ui::forms::InputState::new(window, cx));
        input.update(cx, |state, cx| {
            state.set_placeholder(tr("lithe.navigation.goToLinePrompt"), cx);
            state.set_text(initial, cx);
        });
        let enter = cx.subscribe_in(
            &input,
            window,
            |pane: &mut Self,
             _input: &Entity<lithe_ui::forms::InputState>,
             event: &lithe_ui::forms::InputEvent,
             window,
             cx| {
                if matches!(event, lithe_ui::forms::InputEvent::Submit) {
                    pane.submit_go_to_line(window, cx);
                }
            },
        );
        self.go_to_line = Some(GoToLinePrompt {
            input: input.clone(),
            _enter: enter,
        });

        println!(
            "S1_EDITOR_GO_TO_LINE open=true line={} col={}",
            line + 1,
            utf16_column + 1
        );

        let pane = cx.weak_entity();
        window.open_dialog(
            "lithe-go-to-line",
            move |id, _window, cx| {
                let valid =
                    crate::go_to_line::GoToLineInput::parse(input.read(cx).text()).is_some();
                let confirm_pane = pane.clone();
                let (cancel_id, ok_id) = ((id.clone(), "cancel"), (id.clone(), "ok"));
                Dialog::new(id)
                    .title(tr("lithe.editor.goToLineColumn"))
                    .on_close(dismiss_hosted)
                    .child(lithe_ui::forms::Input::new(&input))
                    .action(move |close| {
                        Button::new(cancel_id)
                            .label(tr("lithe.ui.cancel"))
                            .on_click(move |_, window, cx| close(window, cx))
                    })
                    .action(move |_close| {
                        // 确认由 `submit_go_to_line` 自己收口（它同时服务 Enter 那条路），
                        // 这里再调一次 close 会把**下面**那一层也一起摘掉。
                        Button::new(ok_id)
                            .primary()
                            .label(tr("lithe.ui.ok"))
                            .disabled(!valid)
                            .on_click(move |_, window, cx| {
                                let _ = confirm_pane
                                    .update(cx, |pane, cx| pane.submit_go_to_line(window, cx));
                            })
                    })
                    .into_any_element()
            },
            cx,
        );
    }

    pub fn submit_go_to_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(prompt) = self.go_to_line.as_ref() else {
            return;
        };
        let text = prompt.input.read(cx).text().to_string();
        let Some(parsed) = crate::go_to_line::GoToLineInput::parse(&text) else {
            eprintln!("S1_EDITOR_GO_TO_LINE submit=false reason=unparsable input={text:?}");
            return;
        };
        let jumped = self.go_to_line_column(
            parsed.line,
            parsed.column,
            parsed.has_explicit_column,
            window,
            cx,
        );
        if jumped {
            self.go_to_line = None;
            window.close_dialog(cx);
        }
    }

    pub fn go_to_line_column(
        &mut self,
        line: usize,
        column: usize,
        has_explicit_column: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(index) = self.active else {
            eprintln!("S1_EDITOR_GO_TO_LINE jump=false reason=no-active-buffer");
            return false;
        };
        let text = self.buffers[index].editor.read(cx).text().to_string();
        let target =
            crate::go_to_line::GoToLineInput::clamped(line, column, has_explicit_column, &text);
        let origin = JumpEntry {
            path: self.buffers[index].path.clone(),
            position: line_character(&text, self.buffers[index].editor.read(cx).cursor_offset()),
        };
        // `go_to_line` 的列是 **UTF-16** 口径（提示串里那一列），与 LSP 一致。
        let offset = crate::navigation::offset_for_utf16(&text, target.line, target.column);
        let to = self.buffers[index].editor.update(cx, |state, cx| {
            state.set_cursor_offset(offset, window, cx);
            state.position()
        });
        self.history.record(origin);
        self.buffers[index].editor.update(cx, |state, cx| {
            let handle = state.focus_handle(cx);
            window.focus(&handle, cx);
        });

        println!(
            "S1_EDITOR_GO_TO_LINE jump=true requested={}:{} explicit={} to={}:{}",
            target.line + 1,
            target.column + 1,
            target.has_explicit_column,
            to.0,
            to.1
        );
        self.sync_cursor(cx);
        cx.notify();
        true
    }

    // ------------------------------------------------------------------
    // 悬停
    // ------------------------------------------------------------------

    pub fn show_hover_at_caret(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            eprintln!("S1_EDITOR_HOVER menu=false reason=no-active-buffer");
            return;
        };
        let editor = self.buffers[index].editor.clone();
        let offset = editor.read(cx).cursor_offset();
        println!("S1_EDITOR_HOVER menu=true offset={offset}");
        self.ask_for_hover(&editor, offset, cx);
    }

    fn ask_for_hover(
        &mut self,
        editor: &Entity<CodeEditor>,
        offset: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.index_of(editor) else {
            return;
        };
        let path = self.buffers[index].path.clone();
        let text = editor.read(cx).text().to_string();
        let java = self.java_for(&self.buffers[index].name);

        let generation = self.hover_generation.wrapping_add(1);
        self.hover_generation = generation;
        let requested = crate::hover::request(java, path, text, offset, cx);
        let editor = editor.clone();
        let task = cx.spawn(async move |pane, cx| {
            let content = requested.await;
            let _ = pane.update(cx, |pane, cx| {
                pane.present_hover(&editor, offset, generation, content, cx)
            });
        });
        self.hover_task = Some(task);
    }

    fn present_hover(
        &mut self,
        editor: &Entity<CodeEditor>,
        offset: usize,
        generation: u64,
        content: Option<crate::hover::Content>,
        cx: &mut Context<Self>,
    ) {
        if self.hover_generation != generation {
            return;
        }
        let Some(index) = self.index_of(editor) else {
            return;
        };
        let info = content.map(|content| {
            crate::hover::info(content, ("lithe-editor-hover", generation as usize))
        });
        if info.is_none() {
            eprintln!(
                "S1_EDITOR_HOVER file={} result=none",
                self.buffers[index].path.display()
            );
        }
        self.hover_offset = info.as_ref().map(|_| offset);
        editor.update(cx, |state, cx| state.present_hover_at(offset, info, cx));
    }

    fn clear_hover(&mut self, editor: &Entity<CodeEditor>, cx: &mut Context<Self>) {
        if self.hover_offset.take().is_none() {
            return;
        }
        editor.update(cx, |state, cx| state.present_hover(None, cx));
    }

    // ------------------------------------------------------------------
    // 补全
    // ------------------------------------------------------------------

    fn ask_for_completions(
        &mut self,
        editor: &Entity<CodeEditor>,
        offset: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.index_of(editor) else {
            return;
        };
        // 新的一次请求顶掉旧的列表（也把它的高亮与接管一起收掉）。
        self.dismiss_completion(editor, cx);

        let mine = self.completion_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let path = self.buffers[index].path.clone();
        let text = editor.read(cx).text().to_string();
        let java = self.java_for(&self.buffers[index].name);
        let executor = cx.background_executor().clone();
        let generation = self.completion_generation.clone();
        let requested =
            crate::completion::fetch(java, path, text, offset, executor, generation, mine);

        let editor = editor.clone();
        let task = cx.spawn(async move |pane, cx| {
            let Some(fetched) = requested.await else {
                return;
            };
            let _ = pane.update(cx, |pane, cx| {
                pane.present_completions(&editor, offset, mine, fetched, cx)
            });
        });
        self.completion_task = Some(task);
    }

    fn present_completions(
        &mut self,
        editor: &Entity<CodeEditor>,
        offset: usize,
        mine: u64,
        fetched: crate::completion::Fetched,
        cx: &mut Context<Self>,
    ) {
        if self.completion_generation.load(Ordering::SeqCst) != mine {
            return;
        }
        let Some(index) = self.index_of(editor) else {
            return;
        };
        let file = self.buffers[index].path.display().to_string();
        if fetched.items.is_empty() {
            println!(
                "S1_EDITOR_COMPLETION open=false reason=no-items file={file} source={} prefix={:?} ms={}",
                fetched.source, fetched.prefix, fetched.elapsed_ms
            );
            editor.update(cx, |state, cx| state.clear_completions(cx));
            return;
        }

        self.completion_revision = self.completion_revision.wrapping_add(1);
        let rows = crate::completion::rows(&fetched.items);
        let count = fetched.items.len();
        println!(
            "S1_EDITOR_COMPLETION open=true file={file} source={} prefix={:?} items={count} offset={offset} ms={}",
            fetched.source, fetched.prefix, fetched.elapsed_ms
        );
        editor.update(cx, |state, cx| {
            state.set_completions(rows.clone(), cx);
            state.set_popup_active(true, cx);
        });
        self.completion = Some(Completion {
            editor: editor.entity_id(),
            items: fetched.items,
            rows,
            offset,
            selected: 0,
            revision: self.completion_revision,
        });
        cx.notify();
    }

    fn on_popup_key(&mut self, editor: &Entity<CodeEditor>, key: PopupKey, cx: &mut Context<Self>) {
        if self.completion.is_none() {
            return;
        }
        match key {
            PopupKey::Up => self.step_completion(editor, -1, cx),
            PopupKey::Down => self.step_completion(editor, 1, cx),
            PopupKey::PageUp => self.step_completion(editor, -COMPLETION_PAGE, cx),
            PopupKey::PageDown => self.step_completion(editor, COMPLETION_PAGE, cx),
            PopupKey::Enter | PopupKey::Tab => self.accept_completion(editor, cx),
            PopupKey::Escape => {
                println!("S1_EDITOR_COMPLETION open=false reason=escape");
                self.dismiss_completion(editor, cx);
            }
            // 库把 Backspace 交给了弹层（它自己不删），所以删除由这里做；删完正文一变，
            // `narrow_completion` 会按新前缀收窄或收掉列表。
            PopupKey::Backspace => self.backspace_in_completion(editor, cx),
        }
    }

    fn step_completion(&mut self, editor: &Entity<CodeEditor>, by: isize, cx: &mut Context<Self>) {
        if self.completion.is_none() {
            return;
        }
        let count = self.completion_len(editor, cx);
        if count == 0 {
            return;
        }
        let step = by.rem_euclid(count as isize);
        if let Some(completion) = self.completion.as_mut() {
            let moved = (completion.selected as isize + step).rem_euclid(count as isize);
            completion.selected = moved as usize;
        }
        self.completion_moved(editor, cx);
    }

    fn completion_moved(&mut self, editor: &Entity<CodeEditor>, cx: &mut Context<Self>) {
        println!(
            "S1_EDITOR_COMPLETION selected={}",
            self.completion.as_ref().map_or(0, |each| each.selected)
        );
        editor.update(cx, |state, cx| state.set_popup_active(true, cx));
        cx.notify();
    }

    /// 当前前缀过滤之后还剩多少行（组件库画的就是这个数）。
    fn completion_len(&self, editor: &Entity<CodeEditor>, cx: &App) -> usize {
        let Some(completion) = self.completion.as_ref() else {
            return 0;
        };
        let state = editor.read(cx);
        let typed = crate::completion::word_prefix(state.text(), state.cursor_offset());
        filtered_completions(&completion.rows, &typed).len()
    }

    fn accept_completion(&mut self, editor: &Entity<CodeEditor>, cx: &mut Context<Self>) {
        let Some(completion) = self.completion.as_ref() else {
            return;
        };
        let text = editor.read(cx).text().to_string();
        let cursor = editor.read(cx).cursor_offset();
        let typed = crate::completion::word_prefix(&text, cursor);
        let rows: Vec<UiCompletion> = filtered_completions(&completion.rows, &typed)
            .into_iter()
            .map(|(row, _)| row)
            .collect();
        let Some(item) = crate::completion::pick(&completion.items, &rows, completion.selected)
        else {
            self.dismiss_completion(editor, cx);
            return;
        };
        let (range, insert) = crate::completion::edit_for(item, &text, cursor);
        let label = item.label.clone();
        let file = self.active_path().map(|path| path.display().to_string());

        // 先收列表再改文本：`apply_edits` 会同步发 `Changed`，那时列表若还在，就会按新前缀
        // 把"刚接受的那一条"又留下（一模一样的一行看着像没接受成功）。
        self.dismiss_completion(editor, cx);
        editor.update(cx, |state, cx| state.apply_edits(&[(range, insert)], cx));
        println!(
            "S1_EDITOR_COMPLETION accepted={label:?} file={}",
            file.unwrap_or_default()
        );
        cx.notify();
    }

    fn backspace_in_completion(&mut self, editor: &Entity<CodeEditor>, cx: &mut Context<Self>) {
        let (text, cursor) = {
            let state = editor.read(cx);
            (state.text().to_string(), state.cursor_offset())
        };
        if cursor == 0 {
            return;
        }
        let mut start = cursor - 1;
        while start > 0 && !text.is_char_boundary(start) {
            start -= 1;
        }
        editor.update(cx, |state, cx| {
            state.apply_edits(&[(start..cursor, String::new())], cx)
        });
    }

    /// 接受一条之后、或按下 Esc、或点到别处：把列表收掉，并把键盘接管交还。
    fn dismiss_completion(&mut self, editor: &Entity<CodeEditor>, cx: &mut Context<Self>) {
        if self.completion.take().is_none() {
            return;
        }
        editor.update(cx, |state, cx| {
            state.clear_completions(cx);
            state.set_popup_active(false, cx);
        });
        cx.notify();
    }

    /// 正文变了：前缀空了、或者一条都匹配不上，就把列表收掉；否则把高亮夹回剩下的行里。
    fn narrow_completion(&mut self, editor: &Entity<CodeEditor>, cx: &mut Context<Self>) {
        if self.completion.is_none() {
            return;
        }
        let (typed, count) = {
            let state = editor.read(cx);
            let typed = crate::completion::word_prefix(state.text(), state.cursor_offset());
            let count = self
                .completion
                .as_ref()
                .map(|completion| filtered_completions(&completion.rows, &typed).len())
                .unwrap_or(0);
            (typed, count)
        };
        if typed.is_empty() || count == 0 {
            println!(
                "S1_EDITOR_COMPLETION open=false reason={}",
                if typed.is_empty() {
                    "empty-prefix"
                } else {
                    "no-match"
                }
            );
            self.dismiss_completion(editor, cx);
            return;
        }
        if let Some(completion) = self.completion.as_mut() {
            completion.selected = completion.selected.min(count - 1);
        }
        cx.notify();
    }

    /// 组件库里那一行被点中：按值回配服务端那一条，再走与键盘接受同样的一步。
    fn accept_picked(&mut self, row: UiCompletion, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            return;
        };
        let editor = self.buffers[index].editor.clone();
        let Some(completion) = self.completion.as_ref() else {
            return;
        };
        let Some(item) = crate::completion::pick_clicked(&completion.items, &row) else {
            return;
        };
        let item = item.clone();
        let text = editor.read(cx).text().to_string();
        let cursor = editor.read(cx).cursor_offset();
        let (range, insert) = crate::completion::edit_for(&item, &text, cursor);
        let label = item.label.clone();
        self.dismiss_completion(&editor, cx);
        editor.update(cx, |state, cx| state.apply_edits(&[(range, insert)], cx));
        println!("S1_EDITOR_COMPLETION accepted={label:?} via=click");
        cx.notify();
    }

    // ------------------------------------------------------------------
    // 代码动作
    // ------------------------------------------------------------------

    /// `ctrl-.`（macOS 还有 `cmd-.`，与上游 gpui-base 的 `ToggleCodeActions` 同一个键）。
    pub fn show_code_actions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            eprintln!("S1_EDITOR_CODE_ACTION menu=false reason=no-active-buffer");
            return;
        };
        if language_for_file(&self.buffers[index].name) != Some("java") {
            eprintln!(
                "S1_EDITOR_CODE_ACTION menu=false reason=not-java file={}",
                self.buffers[index].path.display()
            );
            return;
        }
        let Some(service) = self.java.clone() else {
            eprintln!(
                "S1_EDITOR_CODE_ACTION menu=false reason=no-java-service file={}",
                self.buffers[index].path.display()
            );
            return;
        };

        let path = self.buffers[index].path.clone();
        let text = self.buffers[index].editor.read(cx).text().to_string();
        let range = self.buffers[index].editor.read(cx).primary().range();
        let caret = self.buffers[index].editor.read(cx).cursor_offset();

        let mine = self.code_action_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = self.code_action_generation.clone();
        let requested = crate::code_actions::fetch(service, path, text, range, generation, mine);
        let task = cx.spawn_in(window, async move |pane, cx| {
            let Some(actions) = requested.await else {
                return;
            };
            let _ = pane.update_in(cx, |pane, window, cx| {
                pane.present_code_actions(caret, mine, actions, window, cx)
            });
        });
        self.code_action_task = Some(task);
    }

    fn present_code_actions(
        &mut self,
        caret: usize,
        mine: u64,
        actions: Vec<lithe_gpui_java::JavaCodeAction>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.code_action_generation.load(Ordering::SeqCst) != mine {
            return;
        }
        let Some(index) = self.active else {
            return;
        };
        let file = self.buffers[index].path.display().to_string();
        if actions.is_empty() {
            println!("S1_EDITOR_CODE_ACTION menu=false reason=no-actions file={file}");
            return;
        }
        let Some(bounds) = self.buffers[index]
            .editor
            .read(cx)
            .bounds_for_offset(caret, cx)
        else {
            println!("S1_EDITOR_CODE_ACTION menu=false reason=anchor-out-of-view file={file}");
            return;
        };

        let count = actions.len();
        self.code_actions = actions;
        self.code_action_request = Some((
            mine,
            point(bounds.origin.x, bounds.origin.y + bounds.size.height),
        ));
        println!("S1_EDITOR_CODE_ACTION menu=true file={file} actions={count}");
        cx.notify();
    }

    /// 菜单里选中的那一条：把它的编辑落进缓冲区（一批一次撤销步、只读拒绝）。
    fn apply_code_action(&mut self, at: usize, cx: &mut Context<Self>) {
        let Some(action) = self.code_actions.get(at).cloned() else {
            return;
        };
        let Some(index) = self.active else {
            return;
        };
        let editor = self.buffers[index].editor.clone();
        let file = self.buffers[index].path.display().to_string();
        let edits = {
            let state = editor.read(cx);
            crate::code_actions::edits_for(&action, state.text())
        };
        if edits.is_empty() {
            println!(
                "S1_EDITOR_CODE_ACTION file={file} result=skipped reason=no-edits-for-this-file title={:?}",
                action.title
            );
            return;
        }
        let count = edits.len();
        editor.update(cx, |state, cx| state.apply_edits(&edits, cx));
        println!(
            "S1_EDITOR_CODE_ACTION file={file} result=applied edits={count} title={:?}",
            action.title
        );
        if action.other_file_edits > 0 {
            println!(
                "S1_EDITOR_CODE_ACTION file={file} result=not-applied other_files={} title={:?}",
                action.other_file_edits, action.title
            );
        }
        cx.notify();
    }

    /// 组件库的只读横幅带着一个"Make editable"按钮，而本产品的只读是**文件属性**（二进制、有损解码、
    /// 超过上限、读失败）：编辑出来的东西写不回磁盘（`save_active` 也会拒），所以这里立刻按回去，
    /// 并留下一条诊断——不假装这次解锁成功了。
    fn refuse_unlock(&mut self, editor: &Entity<CodeEditor>, cx: &mut Context<Self>) {
        let Some(index) = self.index_of(editor) else {
            return;
        };
        if self.buffers[index].writable {
            return;
        }
        let file = self.buffers[index].path.display().to_string();
        editor.update(cx, |state, cx| state.set_read_only(true, cx));
        eprintln!("S1_EDITOR_UNLOCK_REFUSED file={file} reason=not-writable");
    }

    fn render_tab_bar(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let names = display_names(&self.buffers);
        let colors = cx.theme().colors.clone();

        let mut bar = h_stack()
            .flex_shrink_0()
            .h_9()
            .px_2()
            .overflow_hidden()
            .max_w(rem_px(window.rem_size(), TAB_MAX_WIDTH))
            .border_b_1()
            .border_color(colors.border)
            .bg(colors.tab_bar)
            .id("editor-tab-bar");

        for (index, (buffer, name)) in self.buffers.iter().zip(names).enumerate() {
            bar = bar.child(self.render_tab(index, buffer, name, cx));
        }

        bar.into_any_element()
    }

    /// 一个标签：图标 + 名字 + 未保存圆点（悬停或选中时变成关闭按钮），右键是标签菜单。
    ///
    /// 布局由宿主给（选中态用 `accent` 划一条底线）：组件库的标签栏组件只收 `IconName`，
    /// 而这里的图标是产品自己的文件类型图标（`FileIcon`），换成库内图标就是可见的降级。
    fn render_tab(
        &self,
        index: usize,
        buffer: &Buffer,
        name: SharedString,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_active = self.active == Some(index);
        let (fg, muted) = (cx.theme().colors.fg, cx.theme().colors.fg_muted);
        let accent = cx.theme().colors.accent;
        let hover = cx.theme().colors.hover;
        let label_color = if is_active { fg } else { muted };

        let aria_label = if buffer.is_dirty {
            SharedString::from(format!("{name}（未保存）"))
        } else {
            name.clone()
        };

        let dirty_dot = buffer.is_dirty.then(|| {
            div()
                .flex_shrink_0()
                .size_2()
                .rounded_full()
                .bg(cx.theme().colors.accent)
        });

        let close = (is_active || self.hovered_tab == Some(index)).then(|| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right_1()
                .flex()
                .items_center()
                .child(Self::close_button(&buffer.path, index, cx))
        });

        let content = h_stack()
            .relative()
            .h_full()
            .items_center()
            .pl_2()
            .pr_6()
            .gap_1p5()
            .min_w_0()
            .when(is_active, |tab| tab.bg(cx.theme().colors.bg))
            .child(buffer.icon.render(rems(0.75), cx))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_color(label_color)
                    .child(name),
            )
            .children(dirty_dot)
            .children(close)
            .children(is_active.then(|| {
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(rems(2. / 16.))
                    .bg(accent)
            }))
            .id(tab_element_id(&buffer.path))
            .role(Role::Tab)
            .aria_label(aria_label.clone())
            .aria_selected(is_active)
            .cursor_pointer()
            .hover(move |tab| tab.bg(hover))
            .on_hover(cx.listener(move |pane, hovered: &bool, _window, cx| {
                let next = if *hovered {
                    Some(index)
                } else if pane.hovered_tab == Some(index) {
                    None
                } else {
                    pane.hovered_tab
                };
                if pane.hovered_tab != next {
                    pane.hovered_tab = next;
                    cx.notify();
                }
            }))
            .on_click(
                cx.listener(move |pane, _event: &gpui::ClickEvent, _window, cx| {
                    pane.activate(index, cx);
                }),
            );

        ContextMenu::new(
            tab_menu_element_id(&buffer.path),
            self.tab_menu(index, buffer, cx),
        )
        .child(content.into_any_element())
        .into_any_element()
    }

    /// 标签的右键菜单：除了图标（组件库的菜单行只收库内图标名），条目的文案与动作与迁移前一一对应。
    fn tab_menu(&self, index: usize, buffer: &Buffer, cx: &mut Context<Self>) -> Menu {
        let pane: Entity<Self> = cx.entity();
        let path = buffer.path.clone();
        let on_disk = !is_virtual_source_path(&path);
        let mut menu = Menu::new();
        // 迁移前那条 `S1_TAB_MENU opened=…` 诊断报的是行数与分隔符数，菜单本身读不出这些，所以在建的时候数。
        let mut items = 0usize;
        let mut separators = 0usize;

        type Row = Rc<dyn Fn(&mut EditorPane, &mut Window, &mut Context<EditorPane>)>;
        let entry = |label: SharedString, run: Row| {
            let pane = pane.clone();
            MenuItem::new(label).on_click(move |window, cx| {
                let _ = pane.update(cx, |pane, cx| run(pane, window, cx));
            })
        };

        menu = menu.item(entry(
            tr("lithe.files.copyPath"),
            Rc::new(move |pane: &mut Self, _window, cx| pane.copy_path(index, cx)),
        ));
        items += 1;
        menu = menu.item(entry(
            tr("lithe.files.copyRelativePath"),
            Rc::new(move |pane: &mut Self, _window, cx| pane.copy_relative_path(index, cx)),
        ));
        items += 1;
        menu = menu.item(entry(
            tr("lithe.editor.gpui.revealInProjectTree"),
            Rc::new(move |pane: &mut Self, _window, cx| pane.reveal_in_project_tree(index, cx)),
        ));
        items += 1;
        menu = menu.item(entry(
            tr("lithe.files.reveal"),
            Rc::new(move |pane: &mut Self, window, cx| pane.reveal_in_explorer(index, window, cx)),
        ));
        items += 1;

        if on_disk {
            menu = menu.item(entry(
                tr("lithe.files.openInTerminal"),
                Rc::new(move |pane: &mut Self, window, cx| {
                    pane.open_in_terminal(index, window, cx)
                }),
            ));
            items += 1;
            menu = menu.item(entry(
                tr("lithe.tabs.reload"),
                Rc::new(move |pane: &mut Self, window, cx| pane.request_reload(index, window, cx)),
            ));
            items += 1;
        }

        menu = menu.separator();
        separators += 1;
        menu = menu.item(entry(
            tr("lithe.tabs.close"),
            Rc::new(move |pane: &mut Self, window, cx| pane.request_close(index, window, cx)),
        ));
        items += 1;
        menu = menu.item(entry(
            tr("lithe.tabs.closeOthers"),
            Rc::new({
                let anchor = path.clone();
                move |pane: &mut Self, window, cx| {
                    pane.close_scope(CloseScope::Others(anchor.clone()), window, cx)
                }
            }),
        ));
        items += 1;
        menu = menu.item(entry(
            tr("lithe.tabs.closeToRight"),
            Rc::new({
                let anchor = path.clone();
                move |pane: &mut Self, window, cx| {
                    pane.close_scope(CloseScope::ToRight(anchor.clone()), window, cx)
                }
            }),
        ));
        items += 1;
        menu = menu.item(entry(
            tr("lithe.tabs.closeAll"),
            Rc::new(move |pane: &mut Self, window, cx| {
                pane.close_scope(CloseScope::All, window, cx)
            }),
        ));
        items += 1;

        eprintln!(
            "S1_TAB_MENU opened=true tab={} items={items} separators={separators}",
            buffer.name
        );
        menu
    }

    fn close_button(path: &Path, index: usize, cx: &mut Context<Self>) -> Button {
        Button::new(tab_close_element_id(path))
            .icon_element(idea_icon_mono(&idea::X_ICON, cx.theme().is_dark()))
            .variant(ButtonVariant::Ghost)
            .size(Size::Small)
            .tooltip(tr("lithe.tabs.close"))
            .on_click(
                cx.listener(move |pane, _event: &gpui::ClickEvent, window, cx| {
                    cx.stop_propagation();
                    pane.request_close(index, window, cx);
                }),
            )
    }

    fn render_body(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(buffer) = self.active.and_then(|index| self.buffers.get(index)) else {
            return Self::render_empty_state(cx);
        };
        let editor = buffer.editor.clone();
        let scroll_editor = editor.clone();
        let path = buffer.path.clone();
        let measured = cx.entity().downgrade();
        let popup_owner = measured.clone();

        h_stack()
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(self.breakpoint_gutter(window, &path, cx))
            .child(
                div()
                    .id(EDITOR_HOST_ELEMENT_ID)
                    .debug_selector(|| EDITOR_HOST_ELEMENT_ID.to_string())
                    .relative()
                    .flex_1()
                    .h_full()
                    .min_h_0()
                    .overflow_hidden()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|pane, event: &MouseDownEvent, window, cx| {
                            pane.on_editor_click(event, window, cx);
                        }),
                    )
                    .on_mouse_move(cx.listener(|pane, event: &MouseMoveEvent, _window, cx| {
                        pane.on_editor_move(event, cx)
                    }))
                    .on_hover(cx.listener(|pane, hovered: &bool, _window, cx| {
                        if !*hovered {
                            pane.on_pointer_left_editor(cx);
                        }
                    }))
                    .on_scroll_wheel(cx.listener(
                        move |_pane, event: &ScrollWheelEvent, window, cx| {
                            let line_height = scroll_editor
                                .read(cx)
                                .line_height()
                                .unwrap_or(rem_px(window.rem_size(), FALLBACK_LINE_HEIGHT));
                            let delta = event.delta.pixel_delta(line_height).y;
                            if delta == Pixels::ZERO {
                                return;
                            }
                            let current = scroll_editor.read(cx).scroll_offset();
                            scroll_editor.update(cx, |state, cx| {
                                state.set_scroll_offset(point(current.x, current.y + delta), cx);
                            });
                        },
                    ))
                    .child(editor)
                    // 正文容器的原点：`bounds_for_offset` 给的是窗口坐标，弹层要画成这个容器的子元素，
                    // 所以差这一份原点（与组件库内部 `metrics.body` 同一个做法）。
                    .child(
                        canvas(
                            move |bounds, _, cx| {
                                let _ = measured
                                    .update(cx, |pane, _| pane.body_origin = Some(bounds.origin));
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    ),
            )
            .children(self.completion_popup(&popup_owner, cx))
            .into_any_element()
    }

    /// 补全列表：贴着光标所在的字符画在正文容器里。
    ///
    /// 与迁移前同一个落点（上游的补全菜单也画在编辑器正文里，同样被正文容器的 `overflow` 裁掉），
    /// 所以观感不会变成"库能出界、宿主不能"。锚点看不见（滚出视口、折起来）时这里什么也不画，
    /// 而 `on_editor_changed` 会在同一帧把列表整个收掉，不会留下一个"看不见却还吃着方向键"的弹层。
    fn completion_popup(&self, pane: &WeakEntity<Self>, cx: &App) -> Option<AnyElement> {
        let completion = self.completion.as_ref()?;
        let origin = self.body_origin?;
        let index = self.index_of_id(completion.editor)?;
        let editor = self.buffers.get(index)?.editor.clone();
        let bounds = editor.read(cx).bounds_for_offset(completion.offset, cx)?;
        let (text, cursor) = {
            let state = editor.read(cx);
            (state.text().to_string(), state.cursor_offset())
        };
        let typed = crate::completion::word_prefix(&text, cursor);
        let pick = pane.clone();

        Some(
            div()
                .absolute()
                .left(bounds.origin.x - origin.x)
                .top(bounds.origin.y - origin.y + bounds.size.height)
                .child(
                    CompletionMenu::new(
                        ("lithe-editor-completions", completion.revision as usize),
                        completion.rows.clone(),
                    )
                    .typed(typed)
                    .selected(completion.selected)
                    .on_pick(move |row: &UiCompletion, _window, cx| {
                        let row = row.clone();
                        let _ = pick.update(cx, |pane, cx| pane.accept_picked(row, cx));
                    }),
                )
                .into_any_element(),
        )
    }

    /// 断点槽：每个可视**行**一格（高度与编辑器行高一致），行号由 `line_at_row` 给。
    ///
    /// 不写 `row + 1`：折叠体、ghost、代码透镜与软换行都会让行号不等于行下标，而软换行之后
    /// 同一行的每个可视子行都指向该行（`line_at_row` 的语义）。
    fn breakpoint_gutter(&self, window: &Window, path: &Path, cx: &Context<Self>) -> AnyElement {
        let column = div()
            .w(rems(BREAKPOINT_GUTTER_WIDTH_SPEC / 16.))
            .flex_shrink_0()
            .h_full()
            .overflow_hidden()
            .relative();
        let Some(buffer) = self.active.and_then(|index| self.buffers.get(index)) else {
            return column.into_any_element();
        };
        let (visible, line_height, scroll_y) = {
            let editor = buffer.editor.read(cx);
            (
                editor.visible_row_range(),
                editor.line_height(),
                editor.scroll_offset().y,
            )
        };
        let (Some(visible), Some(line_height)) = (visible, line_height) else {
            return column.into_any_element();
        };
        let top = rem_px(window.rem_size(), EDITOR_CONTENT_TOP_PADDING_SPEC)
            + line_height * visible.start as f32
            - scroll_y;
        let marks = self.gutter_breakpoints.get(path);
        let host = self.debug_gutter_host.as_ref();
        let path_for_rows = path.to_path_buf();
        let rows: Vec<AnyElement> = (visible.start..visible.end)
            .map(|row| {
                // 不是整行的行（透镜、ghost、被删掉的 diff 行）也要占住同样高度，否则下面的行会错位。
                let line = buffer.editor.read(cx).line_at_row(row);
                let breakpoint = line
                    .and_then(|line| {
                        marks.and_then(|marks| marks.iter().find(|mark| mark.line == line))
                    })
                    .copied();
                let mut row_element = h_stack()
                    .id(SharedString::from(format!(
                        "bp-{}-{row}",
                        path_for_rows.display()
                    )))
                    .w_full()
                    .h(line_height)
                    .flex_shrink_0()
                    .justify_center();
                if let Some(breakpoint) = breakpoint {
                    let active = breakpoint.enabled && !self.breakpoints_muted;
                    row_element = row_element.child(
                        div()
                            .size(rems(BREAKPOINT_MARKER_SIZE_SPEC / 16.))
                            .rounded_full()
                            .bg(if active {
                                cx.theme().colors.danger
                            } else {
                                cx.theme().colors.fg_muted
                            }),
                    );
                }
                match (host, line) {
                    (Some(host), Some(line)) => {
                        let toggle = host.toggle.clone();
                        let remove = host.remove.clone();
                        let path_for_click = path_for_rows.clone();
                        row_element
                            .cursor_pointer()
                            .on_click(cx.listener(move |_pane, event: &gpui::ClickEvent, _, cx| {
                                if event.modifiers().alt {
                                    (remove)(&path_for_click, line, cx);
                                } else {
                                    (toggle)(&path_for_click, line, cx);
                                }
                            }))
                            .into_any_element()
                    }
                    _ => row_element.into_any_element(),
                }
            })
            .collect();
        column
            .child(v_stack().w_full().relative().top(-top).children(rows))
            .into_any_element()
    }

    fn render_empty_state(cx: &mut Context<Self>) -> AnyElement {
        ContextMenu::new(
            "lithe-editor-empty-menu",
            Menu::new().item(MenuItem::new(tr("lithe.ui.noActionsHere")).disabled(true)),
        )
        .child(
            v_stack()
                .flex_1()
                .min_h_0()
                .child(
                    EmptyState::slots("lithe-editor-empty")
                        .media_element(
                            div()
                                .relative()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size_12()
                                .child(
                                    idea_icon_mono(&idea::FILE_ICON, cx.theme().is_dark())
                                        .size(lithe_ui::theme::IconSize::Lg),
                                )
                                .child(
                                    idea_icon_mono(
                                        &idea::MAGNIFYING_GLASS_ICON,
                                        cx.theme().is_dark(),
                                    )
                                    .absolute()
                                    .right_0()
                                    .bottom_0(),
                                ),
                        )
                        .title(tr("lithe.workbench.emptyEditorTitle"))
                        .body(tr("lithe.workbench.emptyEditorDescription")),
                )
                .into_any_element(),
        )
        .into_any_element()
    }

    /// 代码动作的菜单：组件库的 `ContextMenu::manual` 就是"宿主在某个窗口坐标上开一份菜单"，
    /// 键盘、选中、点到别处关闭、关掉后交还焦点都归它。
    fn code_action_menu(&self, pane: &WeakEntity<Self>, cx: &App) -> Option<AnyElement> {
        if self.code_action_request.is_none() || self.code_actions.is_empty() {
            return None;
        }
        let pick = pane.clone();
        let menu = crate::code_actions::menu(
            &self.code_actions,
            Rc::new(move |at: usize, _window, cx| {
                let _ = pick.update(cx, |pane, cx| pane.apply_code_action(at, cx));
            }),
        );
        let _ = cx;
        Some(
            ContextMenu::new("lithe-editor-code-actions", menu)
                .manual(self.code_action_request)
                .into_any_element(),
        )
    }
}

fn tab_element_id(path: &Path) -> String {
    format!("editor-tab:{}", path.display())
}

fn tab_close_element_id(path: &Path) -> String {
    format!("editor-tab-close:{}", path.display())
}

fn tab_menu_element_id(path: &Path) -> String {
    format!("editor-tab-menu:{}", path.display())
}

fn is_virtual_source_path(path: &Path) -> bool {
    path.to_string_lossy().starts_with("jdt://")
}

fn relative_to_workspace(path: &Path, root: Option<&Path>) -> String {
    let full = path.to_string_lossy().to_string();
    let Some(root) = root else {
        return full;
    };

    let normalized_path = full.replace('\\', "/");
    let normalized_root = root.to_string_lossy().replace('\\', "/");
    let root_trimmed = normalized_root.trim_end_matches('/');

    normalized_path
        .strip_prefix(root_trimmed)
        .map(|rest| rest.trim_start_matches('/').to_string())
        .filter(|rest| !rest.is_empty())
        .unwrap_or(full)
}

impl Drop for EditorPane {
    fn drop(&mut self) {
        if let Some(service) = self.java.take() {
            service.shutdown();
        }
    }
}

impl Render for EditorPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pane = cx.entity().downgrade();
        let code_actions = self.code_action_menu(&pane, cx);
        let tabs = self.render_tab_bar(window, cx);
        let body = self.render_body(window, cx);
        v_stack()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().colors.bg)
            .on_action(
                cx.listener(|pane, _: &crate::NavigateToDefinition, window, cx| {
                    pane.navigate_to_definition(window, cx);
                }),
            )
            .on_action(cx.listener(|pane, _: &crate::CloseActiveTab, window, cx| {
                pane.close_active(window, cx);
            }))
            .on_action(cx.listener(|pane, _: &crate::ShowCodeActions, window, cx| {
                pane.show_code_actions(window, cx);
            }))
            .child(tabs)
            .child(body)
            .children(code_actions)
    }
}

#[cfg(test)]
mod tests {
    use super::{tab_close_element_id, tab_element_id};
    use std::path::{Path, PathBuf};

    #[test]
    fn tab_element_ids_do_not_depend_on_the_tab_order() {
        let tabs = [
            PathBuf::from("/workspace/src/main.rs"),
            PathBuf::from("/workspace/README.md"),
            PathBuf::from("jdt://contents/java.base/java/lang/String.class"),
        ];

        let forward: Vec<String> = tabs.iter().map(|path| tab_element_id(path)).collect();
        let reversed: Vec<String> = tabs.iter().rev().map(|path| tab_element_id(path)).collect();

        assert_eq!(forward[0], reversed[2]);
        assert_eq!(forward[1], reversed[1]);
        assert_eq!(forward[2], reversed[0]);
        assert_ne!(forward[0], forward[1]);
        assert_ne!(forward[0], forward[2]);
        assert_ne!(forward[1], forward[2]);
        assert!(tab_element_id(Path::new("jdt://x")).starts_with("editor-tab:jdt://x"));
    }

    #[test]
    fn close_button_id_is_namespaced_apart_from_its_tab() {
        let path = Path::new("/workspace/src/main.rs");
        let other = Path::new("/workspace/src/lib.rs");

        assert_ne!(tab_element_id(path), tab_close_element_id(path));
        assert_ne!(tab_close_element_id(path), tab_close_element_id(other));
        assert_eq!(
            tab_close_element_id(path),
            "editor-tab-close:/workspace/src/main.rs"
        );
    }
}
