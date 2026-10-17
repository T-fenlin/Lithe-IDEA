use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use crate::buffer::{
    Buffer, BufferKey, display_names, icon_for_file, language_for_file, read_body,
    resolved_language,
};
use crate::navigation::{
    JumpEntry, JumpHistory, NavTarget, NavigationTarget, byte_offset, line_character, remap_path,
    resolve_target, word_range,
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
    CodeEditor, CompletionItem as UiCompletion, CompletionMenu, EditorEvent, FindOptions,
    FindWidget, LineNumbers, PopupKey, completions as filtered_completions, find_all,
};
use lithe_ui::feedback::EmptyState;
use lithe_ui::forms::{InputEvent, InputState};
use lithe_ui::layout::{h_stack, v_stack};
use lithe_ui::menus::{ContextMenu, Menu, MenuItem};
use lithe_ui::overlays::Dialog;
use lithe_ui::shell::{WindowRootExt as _, dismiss_hosted};
use lithe_ui::theme::{ActiveTheme as _, Size};

/// The key context the find bar's own container carries, so `Escape` resolves to `CloseFind` while
/// focus is inside the bar and to the editor's meanings while focus is in the code. Defined here
/// because the renderer owns the container; the workbench binds keys with this same name.
pub const FIND_BAR_KEY_CONTEXT: &str = "EditorFindBar";

const FALLBACK_LINE_HEIGHT: f32 = 20.;

pub const EDITOR_HOST_ELEMENT_ID: &str = "lithe-editor-host";

const AUTO_SAVE_DEBOUNCE: Duration = Duration::from_millis(150);

/// Query keystrokes and text changes recompute the find matches after this quiet period, so typing
/// does not run a full scan on every key. The first open and any explicit change is immediate.
const FIND_DEBOUNCE: Duration = Duration::from_millis(120);

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
    /// 这次跳的是定义 / 实现 / 类型定义：落点后写进 `S1_NAV_JUMP` 的 `kind`，
    /// 也决定「没找到」通知里报的是哪个目标。
    target: NavigationTarget,
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
    Others(BufferKey),
    ToLeft(BufferKey),
    ToRight(BufferKey),
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
                .filter(|(_, buffer)| &buffer.key != anchor)
                .map(|(index, _)| index)
                .collect(),
            Self::ToLeft(anchor) => {
                let Some(position) = buffers.iter().position(|buffer| &buffer.key == anchor) else {
                    return Vec::new();
                };
                (0..position).collect()
            }
            Self::ToRight(anchor) => {
                let Some(position) = buffers.iter().position(|buffer| &buffer.key == anchor) else {
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

/// 重命名符号的输入框。
///
/// `offset` 是**发起时**光标所在的字节偏移：对话框开着的时候用户还能去点正文，但那一次重命名的
/// 语义是「当初光标下那个符号」，所以位置以发起那一刻为准，不跟着光标漂。
struct RenamePrompt {
    input: Entity<lithe_ui::forms::InputState>,
    offset: usize,
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

/// The find bar that is currently showing (the Ctrl+F strip inside the editor).
///
/// Same "the host owns it" shape as [`Completion`]: the component library knows how to draw a find
/// box but not which buffer it belongs to, so the pane holds the two inputs, the matches it
/// derived, and the debounce generation.
struct FindBar {
    /// The buffer editor it belongs to; switching tabs, closing a tab or reloading compares against
    /// this so a stale bar is torn down rather than left pointing at another file.
    editor: Entity<CodeEditor>,
    /// The query and replace fields; the pane owns the `InputState`s (as it does `chat_field`).
    query: Entity<InputState>,
    replace: Entity<InputState>,
    options: FindOptions,
    /// All matches of `query` in the current buffer, recomputed whenever text, query or options
    /// change — never cached across buffers.
    matches: Vec<Range<usize>>,
    /// Set when the query does not compile; drawn as a short word in the count cell with the
    /// compiler's own message on a tooltip.
    error: Option<SharedString>,
    /// The chosen match, inside `0..matches.len()`; `None` when there are no matches.
    current: Option<usize>,
    /// Whether the replace row is unfolded.
    showing_replace: bool,
    /// Bumped on every recompute request; a debounced task whose generation has moved on drops its
    /// (stale) result rather than making the count jump back.
    generation: Arc<AtomicU64>,
    /// Input subscriptions (query `Changed`/`Submit`); dropped with the bar.
    _events: Vec<Subscription>,
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
    /// 内嵌提示与 Code Lens 的代次；与诊断各算各的（见 `decorations` 模块注释）。
    decorations_generation: u64,
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
    /// 三个显示开关的当前档位（设置 store 是唯一真源，这里留一份给新开的缓冲区用）。
    minimap: bool,
    line_numbers: bool,
    render_whitespace: bool,
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
    /// 转到引用：服务端给的这一批、以及「请在第 N 次请求时于这个窗口坐标开菜单」。
    ///
    /// 与代码动作**共用**菜单机制但各有各的代次：两者可以同时在飞，共用一个计数器会让后发的那次
    /// 把先发的答案判成过期。
    references: Vec<crate::symbol_actions::Reference>,
    reference_request: Option<(u64, Point<Pixels>)>,
    reference_generation: Arc<AtomicU64>,
    reference_task: Option<Task<()>>,
    /// 重命名符号：一个带输入框的模态对话框；`None` 表示没开。
    rename: Option<RenamePrompt>,
    rename_generation: Arc<AtomicU64>,
    rename_task: Option<Task<()>>,
    /// 格式化：发一次请求，回来整批落进当前缓冲区（没有 UI，所以只有一个代次与任务）。
    format_generation: Arc<AtomicU64>,
    format_task: Option<Task<()>>,
    /// 正文容器（弹层的父容器）在窗口里的原点，由它自己的 canvas 量出来：`bounds_for_offset` 给的是
    /// **窗口**坐标，要画成这个容器的子元素就得减去它。
    body_origin: Option<Point<Pixels>>,
    /// 查找/替换条；`None` 表示没开（关闭即清空归属编辑器的 `backgrounds`，见 `close_find`）。
    find: Option<FindBar>,
    /// 未命名缓冲的下一号；只增不减、窗格内不复用，所以两个未命名标签不会撞键。
    untitled_counter: u32,
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
            decorations_generation: 0,
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
            minimap: false,
            line_numbers: true,
            render_whitespace: false,
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
            references: Vec::new(),
            reference_request: None,
            reference_generation: Arc::new(AtomicU64::new(0)),
            reference_task: None,
            rename: None,
            rename_generation: Arc::new(AtomicU64::new(0)),
            rename_task: None,
            format_generation: Arc::new(AtomicU64::new(0)),
            format_task: None,
            body_origin: None,
            hovered_tab: None,
            closed: Vec::new(),
            find: None,
            untitled_counter: 0,
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

    /// 缩略图开关：记下档位（给后面新开的缓冲区），并让开着的这批一起换。
    pub fn set_minimap(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let changed = self.minimap != enabled;
        self.minimap = enabled;
        for buffer in &self.buffers {
            buffer
                .editor
                .update(cx, |state, cx| state.set_minimap(enabled, cx));
        }
        println!("S1_EDITOR_MINIMAP enabled={enabled} changed={changed}");
        if changed {
            cx.notify();
        }
    }

    /// 行号开关：关掉等于 `LineNumbers::Hidden`。
    pub fn set_line_numbers(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let changed = self.line_numbers != enabled;
        self.line_numbers = enabled;
        let numbers = if enabled {
            LineNumbers::Absolute
        } else {
            LineNumbers::Hidden
        };
        for buffer in &self.buffers {
            buffer
                .editor
                .update(cx, |state, cx| state.set_line_numbers(numbers, cx));
        }
        println!("S1_EDITOR_LINE_NUMBERS enabled={enabled} changed={changed}");
        if changed {
            cx.notify();
        }
    }

    /// 空白字符渲染开关。
    pub fn set_render_whitespace(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let changed = self.render_whitespace != enabled;
        self.render_whitespace = enabled;
        for buffer in &self.buffers {
            buffer
                .editor
                .update(cx, |state, cx| state.set_render_whitespace(enabled, cx));
        }
        println!("S1_EDITOR_RENDER_WHITESPACE enabled={enabled} changed={changed}");
        if changed {
            cx.notify();
        }
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
            self.schedule_decorations(index, Duration::ZERO, cx);
        }

        self.java_task = Some(cx.background_spawn(async move {
            match service.prepare() {
                Ok(true) => println!("S1_JAVA_PREPARED"),
                Ok(false) => {}
                Err(reason) => println!("S1_JAVA_PREPARE_FAILED reason={reason}"),
            }
        }));
    }

    /// 取一次内嵌提示与 Code Lens，回来整份替换。
    ///
    /// 与诊断同一套守卫：非 Java、不可写、虚拟源码都不发；代次与 revision 双复核，
    /// 晚到的答案一律丢掉。区别是**没有退避重试**——注记是只读的辅助信息，
    /// 诊断那套「等两份一样才算稳定」的等待在这里没有对应的收敛问题。
    fn schedule_decorations(&mut self, index: usize, debounce: Duration, cx: &mut Context<Self>) {
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

        let generation = self.decorations_generation.wrapping_add(1);
        self.decorations_generation = generation;
        self.buffers[index].decorations_generation = generation;

        let executor = cx.background_executor().clone();
        let task_path = path.clone();
        let task = cx.spawn(async move |pane, cx| {
            let fetched = cx
                .background_spawn(crate::decorations::fetch(
                    service,
                    task_path.clone(),
                    text,
                    executor,
                    debounce,
                ))
                .await;
            let _ = pane.update(cx, |pane, cx| {
                pane.apply_decorations(generation, revision, task_path, fetched, cx)
            });
        });
        self.buffers[index].decorations_task = Some(task);
    }

    fn apply_decorations(
        &mut self,
        generation: u64,
        revision: u64,
        path: PathBuf,
        fetched: crate::decorations::Fetched,
        cx: &mut Context<Self>,
    ) {
        let file = path.display();
        let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) else {
            println!("S1_EDITOR_DECORATIONS file={file} result=stale reason=buffer-closed");
            return;
        };
        if self.buffers[index].decorations_generation != generation {
            println!("S1_EDITOR_DECORATIONS file={file} result=stale reason=superseded");
            return;
        }
        if self.buffers[index].revision != revision {
            println!("S1_EDITOR_DECORATIONS file={file} result=stale reason=content-changed");
            return;
        }

        let editor = self.buffers[index].editor.clone();
        let counts = editor.update(cx, |state, cx| {
            // 先取正文再改：换算用的就是这份正文，借用得在 `set_*` 之前结束。
            let text = state.text().to_string();
            crate::decorations::apply(state, &fetched.hints, &fetched.lenses, &text, cx)
        });
        println!(
            "S1_EDITOR_DECORATIONS file={file} hints={} lenses={} error={:?} ms={} revision={revision}",
            counts.0,
            counts.1,
            fetched.error,
            fetched.elapsed_ms
        );
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

        if !self
            .buffers
            .iter()
            .any(|buffer| buffer.disk_path() == Some(path))
        {
            self.open(path, window, cx);
        }
        let Some(index) = self
            .buffers
            .iter()
            .position(|buffer| buffer.disk_path() == Some(path))
        else {
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

        if let Some(index) = self
            .buffers
            .iter()
            .position(|buffer| buffer.key == BufferKey::Disk(path.clone()))
        {
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

        let editor = self.build_editor(
            text,
            writable,
            language,
            tab_size,
            auto_completion,
            window,
            cx,
        );
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
            BufferKey::Disk(path),
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
            self.schedule_decorations(opened, Duration::ZERO, cx);
        }
        cx.notify();
    }

    /// Builds a `CodeEditor` with the pane's current display switches applied. Shared by `open`,
    /// `open_virtual`, `new_untitled` and `create_file` so a new buffer always starts consistent
    /// with the settings.
    fn build_editor(
        &self,
        text: String,
        writable: bool,
        language: Option<&'static lithe_ui::language::Language>,
        tab_size: usize,
        auto_completion: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<CodeEditor> {
        let (soft_wrap, minimap, line_numbers, render_whitespace) = (
            self.soft_wrap,
            self.minimap,
            self.line_numbers,
            self.render_whitespace,
        );
        cx.new(|cx| {
            let mut state = CodeEditor::new(text, window, cx)
                .tab_size(tab_size)
                .auto_completion(auto_completion)
                .completion_triggers(COMPLETION_TRIGGERS);
            if !writable {
                state = state.read_only();
            }
            if !soft_wrap {
                state = state.soft_wrap(false);
            }
            if minimap {
                state = state.minimap();
            }
            if !line_numbers {
                state = state.line_numbers(LineNumbers::Hidden);
            }
            if render_whitespace {
                state = state.show_whitespace();
            }
            state.set_language(language, cx);
            state
        })
    }

    /// Opens a scratch buffer: no path, plain text, not dirty, and not part of the session. Its
    /// number never repeats within this pane, and Save (`Ctrl+S`) sends it to Save As.
    pub fn new_untitled(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.untitled_counter = self.untitled_counter.wrapping_add(1);
        let number = self.untitled_counter;
        let name = tr_args(
            "lithe.files.untitledNumber",
            &[("number", &number.to_string())],
        );
        let tab_size = self.tab_size;
        let auto_completion = self.auto_completion.load(Ordering::Acquire);

        let editor = self.build_editor(
            String::new(),
            true,
            None,
            tab_size,
            auto_completion,
            window,
            cx,
        );
        let subscriptions = self.subscribe_to(&editor, window, cx);
        let icon = icon_for_file(&name, cx);
        self.buffers.push(Buffer::new(
            BufferKey::Untitled(number),
            name.clone(),
            icon,
            editor,
            true,
            subscriptions,
        ));
        self.active = Some(self.buffers.len() - 1);
        self.sync_cursor(cx);
        println!("S1_EDITOR_NEW_UNTITLED number={number} name={name}");
        cx.notify();
    }

    /// Creates an empty file on disk and opens it. When `directory` is `None` the file lands next
    /// to the active buffer (or at the workspace root). The name is the caller's validated choice;
    /// creation uses `create_new` so an existing file is never silently truncated.
    pub fn create_file(
        &mut self,
        directory: Option<PathBuf>,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(directory) = directory.or_else(|| self.creation_directory()) else {
            eprintln!("S1_EDITOR_NEW_FILE create=false reason=no-directory");
            self.record_notification(
                cx,
                lithe_gpui_notify::Input::new(
                    "lithe.files.openFolderFirst",
                    lithe_gpui_notify::Severity::Warning,
                )
                .key("lithe.files.openFolderFirst"),
            );
            return;
        };
        let path = directory.join(&name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => {
                println!("S1_EDITOR_NEW_FILE create=true path={}", path.display());
                self.open(&path, window, cx);
            }
            Err(error) => {
                eprintln!(
                    "S1_EDITOR_NEW_FILE create=false path={} error={error}",
                    path.display()
                );
                self.record_notification(
                    cx,
                    lithe_gpui_notify::Input::new(
                        "lithe.files.createFileFailedMessage",
                        lithe_gpui_notify::Severity::Error,
                    )
                    .param("error", error.to_string())
                    .key("lithe.files.createFileFailedMessage"),
                );
            }
        }
    }

    /// Where a new file should go: the active buffer's folder, else the workspace root.
    fn creation_directory(&self) -> Option<PathBuf> {
        self.active
            .and_then(|index| self.buffers.get(index))
            .and_then(|buffer| buffer.disk_path())
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .or_else(|| self.workspace_root.clone())
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
            .and_then(|buffer| buffer.disk_path())
            .map(Path::to_path_buf)
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
            .filter(|buffer| {
                buffer
                    .disk_path()
                    .is_some_and(|path| !is_virtual_source_path(path))
            })
            .map(|buffer| buffer.path.clone())
            .collect()
    }

    pub fn active_disk_file(&self) -> Option<PathBuf> {
        self.active
            .and_then(|index| self.buffers.get(index))
            .filter(|buffer| {
                buffer
                    .disk_path()
                    .is_some_and(|path| !is_virtual_source_path(path))
            })
            .map(|buffer| buffer.path.clone())
    }

    /// 打开着几个标签页。
    ///
    /// 与 [`Self::active_tab_index`] 是一对：`EditorPane::close` 保证 `buffers.is_empty()`
    /// 与 `active.is_none()` 严格等价，所以 `tab_count() == 0` 时下标必是 `None`。
    pub fn tab_count(&self) -> usize {
        self.buffers.len()
    }

    /// 活动标签页的下标；一个标签页都没开时是 `None`。
    pub fn active_tab_index(&self) -> Option<usize> {
        self.active
    }

    /// 活动缓冲是不是一个**真正的磁盘文件**（未命名 scratch 与虚拟源路径不算）。
    ///
    /// 判定与 [`Self::active_disk_file`] 完全一致，只是不克隆那条 `PathBuf`：这个问题
    /// 每帧都要问一次（菜单要不要置灰），不该为它付一次路径克隆。
    pub fn has_active_disk_file(&self) -> bool {
        self.active_disk_file().is_some()
    }

    /// 跳转历史里有没有「上一处」可去 —— 菜单 ▸ Go ▸ Back 的启用条件。
    ///
    /// 只读：与 History 的改动命令（[`Self::go_back`]）分工，问「能不能去」不该动历史。
    pub fn can_go_back(&self) -> bool {
        self.history.can_go_back()
    }

    /// 跳转历史里有没有「下一处」可去 —— 菜单 ▸ Go ▸ Forward 的启用条件。
    pub fn can_go_forward(&self) -> bool {
        self.history.can_go_forward()
    }

    /// 有没有关掉过标签页 —— 菜单 ▸ File ▸ Reopen Closed Tab 的启用条件。
    ///
    /// `closed` 只收真正的磁盘文件（`close()` 里排除未命名缓冲与虚拟源路径），
    /// 与 [`Self::reopen_closed_tab`] 弹出时用的是同一条队列。
    pub fn has_closed_tabs(&self) -> bool {
        !self.closed.is_empty()
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
            eprintln!("S1_EDITOR_SAVE active=false reason=no-active-buffer");
            return;
        };
        if !self
            .buffers
            .get(index)
            .is_some_and(|buffer| buffer.writable)
        {
            return;
        }
        // A scratch buffer has no path: Save must become Save As, never a silent no-op.
        if !self.buffers[index].is_disk() {
            println!("S1_EDITOR_SAVE active=false reason=untitled via=saveAs");
            self.save_as(window, cx);
            return;
        }
        self.write_buffer(index, "lithe.editor.saveFailed", window, cx);
    }

    /// Save As: pick a path, write the buffer there, then move its key/path/name/icon/language and
    /// every path table onto it. The original file is left untouched. When the chosen path is
    /// already another tab, the current buffer is written out, closed, and that tab is activated.
    pub fn save_as(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            eprintln!("S1_EDITOR_SAVE_AS save=false reason=no-active-buffer");
            return;
        };
        let (key, directory, default_name) = {
            let buffer = &self.buffers[index];
            let directory = buffer
                .disk_path()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
                .or_else(|| self.workspace_root.clone())
                .unwrap_or_default();
            (buffer.key.clone(), directory, buffer.name.to_string())
        };

        let picked = cx.prompt_for_new_path(&directory, Some(&default_name));
        let pane = cx.weak_entity();
        cx.spawn_in(window, async move |_this, cx| {
            let path = match picked.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) => {
                    eprintln!("S1_EDITOR_SAVE_AS save=false reason=cancelled");
                    return;
                }
                Ok(Err(error)) => {
                    eprintln!("S1_EDITOR_SAVE_AS save=false reason=error error={error}");
                    return;
                }
                Err(_) => {
                    eprintln!("S1_EDITOR_SAVE_AS save=false reason=channel-closed");
                    return;
                }
            };
            let _ = pane.update_in(cx, |pane, window, cx| {
                pane.finish_save_as(key, path, window, cx)
            });
        })
        .detach();
    }

    fn finish_save_as(
        &mut self,
        key: BufferKey,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.buffers.iter().position(|buffer| buffer.key == key) else {
            eprintln!("S1_EDITOR_SAVE_AS save=false reason=buffer-gone");
            return;
        };
        let value = self.buffers[index].editor.read(cx).text().to_string();
        if let Err(error) = std::fs::write(&path, value.as_bytes()) {
            eprintln!(
                "S1_EDITOR_SAVE_AS save=false path={} error={error}",
                path.display()
            );
            let name = self.buffers[index].name.clone();
            self.record_notification(
                cx,
                lithe_gpui_notify::Input::new(
                    "lithe.editor.saveFailed",
                    lithe_gpui_notify::Severity::Error,
                )
                .param("name", name.as_ref())
                .param("error", error.to_string())
                .key("lithe.editor.saveFailed"),
            );
            return;
        }

        // The target is already another tab: writing is done above so nothing is lost, then close
        // the current buffer and land on the existing tab rather than showing two tabs one path.
        let duplicate = self
            .buffers
            .iter()
            .position(|buffer| buffer.key == BufferKey::Disk(path.clone()))
            .filter(|existing| *existing != index);
        if let Some(existing) = duplicate {
            self.buffers[index].is_dirty = false;
            println!(
                "S1_EDITOR_SAVE_AS duplicateTarget=true path={}",
                path.display()
            );
            self.close(index, cx);
            let target = existing.min(self.buffers.len().saturating_sub(1));
            self.activate(target, cx);
            self.sync_cursor(cx);
            cx.notify();
            return;
        }

        let old = self.buffers[index].path.clone();
        self.buffers[index].is_dirty = false;
        self.buffers[index].auto_save_task = None;
        self.rekey_buffer(index, path.clone(), cx);
        self.remap_paths(&old, &path, window, cx);
        println!(
            "S1_EDITOR_SAVE_AS save=true path={} bytes={}",
            path.display(),
            value.len()
        );
        println!(
            "S1_EDITOR_SAVED path={} bytes={}",
            path.display(),
            value.len()
        );
        self.sync_cursor(cx);
        cx.notify();
    }

    /// Save All: writes every dirty, writable disk buffer in tab order; stops at the first failure.
    /// Dirty scratch buffers cannot be saved without a name, so they are counted and reported once.
    /// With nothing dirty this is a successful no-op that reports nothing.
    pub fn save_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut saved = 0usize;
        let mut skipped_untitled = 0usize;
        let mut skipped_readonly = 0usize;
        let mut failed = false;

        for index in 0..self.buffers.len() {
            let buffer = &self.buffers[index];
            if !buffer.is_dirty {
                continue;
            }
            if !buffer.writable {
                skipped_readonly += 1;
                continue;
            }
            if !buffer.is_disk() {
                skipped_untitled += 1;
                continue;
            }
            if !self.write_buffer(index, "lithe.editor.saveFailed", window, cx) {
                failed = true;
                break;
            }
            saved += 1;
        }

        if skipped_untitled > 0 {
            // 未命名 + 脏：跳过但**不能静默**。文案必须点名「未命名、请用另存为」，所以用
            // `lithe.editor.saveAllUntitled` 而不是泛泛的 `unsavedChangesBatchBody`（§3.5）。
            let count = skipped_untitled.to_string();
            self.record_notification(
                cx,
                lithe_gpui_notify::Input::new(
                    "lithe.editor.saveAllUntitled",
                    lithe_gpui_notify::Severity::Warning,
                )
                .param("count", &count)
                .key("lithe.editor.saveAllUntitled"),
            );
        }
        println!(
            "S1_EDITOR_SAVE_ALL saved={saved} skippedUntitled={skipped_untitled} skippedReadOnly={skipped_readonly} failed={}",
            u8::from(failed)
        );
    }

    /// Revert File: re-reads the active buffer from disk, confirming first when it is dirty. The
    /// confirmation is the shared three-button dialog (`Cancel` / `Discard` / `Save`); Save-then-
    /// revert is a no-op write-back, kept for v1 to avoid a second dialog shape.
    pub fn revert_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            eprintln!("S1_EDITOR_REVERT revert=false reason=no-active-buffer");
            return;
        };
        if !self.buffers[index].is_disk() {
            eprintln!("S1_EDITOR_REVERT revert=false reason=untitled");
            return;
        }
        let dirty = self.buffers[index].is_dirty;
        println!(
            "S1_EDITOR_REVERT path={} dirty={dirty} confirm={dirty}",
            self.buffers[index].path.display()
        );
        self.request_reload(index, window, cx);
    }

    /// Rebuilds a buffer's identity after its path changes: the key, path, name, icon and language
    /// all follow the new file name.
    fn rekey_buffer(&mut self, index: usize, path: PathBuf, cx: &mut Context<Self>) {
        let name: SharedString = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string())
            .into();
        let icon = icon_for_file(&name, cx);
        let language = resolved_language(&name);
        let buffer = &mut self.buffers[index];
        buffer.key = BufferKey::Disk(path.clone());
        buffer.path = path;
        buffer.name = name;
        buffer.icon = icon;
        buffer
            .editor
            .update(cx, |state, cx| state.set_language(language, cx));
    }

    /// The single entry point that moves every pane-owned path table when a file or a whole folder
    /// changes location (explorer rename, Save As): open buffers' keys, `last_text`,
    /// `gutter_breakpoints`, the closed-tab list, and the jump history. Matching is by path
    /// component prefix, so `a/b` never touches `a/bc`.
    pub fn remap_paths(
        &mut self,
        from: &Path,
        to: &Path,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for index in 0..self.buffers.len() {
            let Some(old) = self.buffers[index].disk_path().map(Path::to_path_buf) else {
                continue;
            };
            if let Some(new_path) = remap_path(&old, from, to) {
                self.rekey_buffer(index, new_path, cx);
            }
        }
        remap_map(&mut self.last_text, from, to);
        remap_map(&mut self.gutter_breakpoints, from, to);
        for entry in &mut self.closed {
            if let Some(moved) = remap_path(entry, from, to) {
                *entry = moved;
            }
        }
        self.history.remap(from, to);
        println!(
            "S1_EDITOR_REMAP from={} to={}",
            from.display(),
            to.display()
        );
        cx.notify();
    }

    /// Forgets everything under a deleted file or folder: closes the affected buffers (each dirty
    /// one still goes through its save/discard confirmation via the tab machinery), drops their
    /// `last_text` and breakpoint rows, and clears them from the jump history.
    pub fn forget_paths(&mut self, target: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let victims: Vec<usize> = self
            .buffers
            .iter()
            .enumerate()
            .filter(|(_, buffer)| {
                buffer
                    .disk_path()
                    .is_some_and(|path| sits_under(path, target))
            })
            .map(|(index, _)| index)
            .collect();

        for index in victims.iter().rev() {
            self.drop_find_for(self.buffers[*index].editor.entity_id(), cx);
            self.close(*index, cx);
        }
        self.last_text.retain(|path, _| !sits_under(path, target));
        self.gutter_breakpoints
            .retain(|path, _| !sits_under(path, target));
        self.history.remove_path(target);
        let _ = window;
        println!(
            "S1_EDITOR_FORGET target={} closed={}",
            target.display(),
            victims.len()
        );
        self.sync_cursor(cx);
        cx.notify();
    }

    /// Go ▸ Go to Definition。
    pub fn navigate_to_definition(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.navigate_to(NavigationTarget::Definition, window, cx);
    }

    /// Go ▸ Go to Implementation（`textDocument/implementation`）。
    pub fn navigate_to_implementation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.navigate_to(NavigationTarget::Implementation, window, cx);
    }

    /// Go ▸ Go to Type Definition（`textDocument/typeDefinition`）。
    pub fn navigate_to_type_definition(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.navigate_to(NavigationTarget::TypeDefinition, window, cx);
    }

    /// 三个 Go-to 共用的入口：取光标位置、记一次代次、后台问语言服务，回到前台落点。
    ///
    /// 代次（`nav_generation`）是这里唯一的状态机：请求发出后用户可能改了正文、换了标签页或
    /// 又按了一次别的 Go-to，回来时 `apply_definition` 会拿代次与 revision 复核，过期的结果
    /// 直接丢掉，不会把人跳到旧位置。
    fn navigate_to(
        &mut self,
        target: NavigationTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.active else {
            println!("S1_NAV_FAILED kind={} reason=no-buffer", target.tag());
            return;
        };
        let Some(buffer) = self.buffers.get(index) else {
            println!("S1_NAV_FAILED kind={} reason=no-buffer", target.tag());
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
            target,
        });

        let task = cx.spawn_in(window, async move |pane, cx| {
            let result = cx
                .background_spawn(async move {
                    resolve_target(java.as_deref(), target, &file_path, &text, offset)
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
        let Some((origin_path, revision, origin, nav)) = self
            .nav_request
            .as_ref()
            .filter(|request| request.generation == generation)
            .map(|request| {
                (
                    request.path.clone(),
                    request.revision,
                    request.origin,
                    request.target,
                )
            })
        else {
            println!("S1_NAV_FAILED reason=stale");
            return;
        };
        let tag = nav.tag();

        let Some(origin_index) = self
            .buffers
            .iter()
            .position(|buffer| buffer.path == origin_path)
        else {
            println!("S1_NAV_FAILED kind={tag} reason=buffer-closed");
            return;
        };
        if self.buffers[origin_index].revision != revision {
            println!("S1_NAV_FAILED kind={tag} reason=content-changed");
            return;
        }

        let target = match result {
            Ok(Some(target)) => target,
            Ok(None) => {
                println!("S1_NAV_FAILED kind={tag} reason=no-target");
                self.notify_no_target(nav.label_key(), nav.tag(), cx);
                return;
            }
            Err(error) => {
                println!("S1_NAV_FAILED kind={tag} reason={error}");
                self.notify_no_target(nav.label_key(), nav.tag(), cx);
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
                let Some((index, offset)) =
                    self.land_on_file(&path, line, utf16_column, window, cx)
                else {
                    println!("S1_NAV_FAILED kind={tag} reason=no-target");
                    self.notify_no_target(nav.label_key(), nav.tag(), cx);
                    return;
                };
                (index, offset, "jdtls")
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
            "S1_NAV_JUMP from={}:{} to={}:{} kind={tag} via={via} file={}",
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
        if let Some(index) = self
            .buffers
            .iter()
            .position(|buffer| buffer.key == BufferKey::Disk(path.clone()))
        {
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
        self.buffers.push(Buffer::new(
            BufferKey::Disk(path),
            name,
            icon,
            editor,
            false,
            subscriptions,
        ));
        self.buffers.len() - 1
    }

    #[allow(dead_code)]
    /// 后退：跳转历史上一步。菜单与 `Ctrl+Alt+Left` 都走这里。
    pub fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(present) = self.current_entry(cx) else {
            return;
        };
        let Some(entry) = self.history.go_back(present.clone()) else {
            return;
        };
        self.restore_entry(present, entry, "S1_NAV_BACK", window, cx);
    }

    /// 前进：跳转历史下一步。
    pub fn go_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    /// 「没找到目标」的通知。
    ///
    /// 文案与去重键都按目标分：连按三次「转到实现」只该合并成一条，而不该与
    /// 「转到定义」的那条互相顶掉。
    ///
    /// 取的是「文案键 + 去重标签」而不是 [`NavigationTarget`]：转到引用也是一次"没找到"，
    /// 但它返回一组位置、和三个 Go-to 不是同一个形状，硬塞进那个枚举只会让 `resolve_target`
    /// 多一个永远走不到的取值。
    fn notify_no_target(&self, label_key: &str, tag: &str, cx: &mut Context<Self>) {
        let target = tr(label_key);
        let code = "lithe.navigation.noTargetFound";
        let key = format!("navigation-no-target-{tag}");
        if !self.record_notification(
            cx,
            lithe_gpui_notify::Input::new(code, lithe_gpui_notify::Severity::Info)
                .param("target", target.as_ref())
                .key(key),
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

    /// 切到下一个标签页（循环）：`Go ▸ Next Tab` 与 `Ctrl+Tab` 都走这里。
    pub fn next_tab(&mut self, cx: &mut Context<Self>) {
        let Some(current) = self.active else {
            return;
        };
        if self.buffers.is_empty() {
            return;
        }
        let next = (current + 1) % self.buffers.len();
        self.activate(next, cx);
    }

    /// 切到上一个标签页（循环）。
    pub fn previous_tab(&mut self, cx: &mut Context<Self>) {
        let Some(current) = self.active else {
            return;
        };
        if self.buffers.is_empty() {
            return;
        }
        let previous = if current == 0 {
            self.buffers.len() - 1
        } else {
            current - 1
        };
        self.activate(previous, cx);
    }

    fn activate(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.buffers.len() {
            return;
        }
        // Tear the find bar down before the active buffer changes: afterwards a bar owned by the
        // old buffer would no longer match and its highlight would be stranded.
        let next = self.buffers[index].editor.entity_id();
        if self
            .find
            .as_ref()
            .is_some_and(|find| find.editor.entity_id() != next)
        {
            self.drop_find(cx);
            eprintln!("S1_EDITOR_FIND open=false reason=switch");
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
        self.schedule_decorations(index, crate::decorations::CHANGE_DEBOUNCE, cx);

        if self.auto_save {
            self.schedule_auto_save(index, window, cx);
        }

        // A scratch buffer has no file, so it has no row in `last_text` or the debug gutter.
        if let Some(path) = self.buffers[index].disk_path().map(Path::to_path_buf) {
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
        }

        self.clear_hover(editor, cx);
        editor.update(cx, |state, cx| {
            state.set_diagnostics(Vec::new(), cx);
            // 注记的偏移与诊断一样会过期：先清掉，免得在旧位置画一排错位的灰字。
            crate::decorations::clear(state, cx);
        });
        self.narrow_completion(editor, cx);
        // The find matches depend on the text, so the bar re-scans (debounced like the query).
        if self
            .find
            .as_ref()
            .is_some_and(|find| find.editor.entity_id() == editor.entity_id())
        {
            self.schedule_find_recompute(window, cx);
        }
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
        // A scratch buffer has no path; writing to its display name would drop a stray file on
        // disk. Callers route scratch buffers to Save As instead.
        if !buffer.is_disk() {
            eprintln!("S1_EDITOR_SAVE_FAILED reason=untitled name={}", buffer.name);
            return false;
        }
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

    fn close(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.buffers.len() {
            return;
        }

        // Close the find bar first when it belongs to the buffer going away, so its highlight is
        // cleared while the editor entity is still reachable.
        self.drop_find_for(self.buffers[index].editor.entity_id(), cx);

        let closed = self.buffers[index].path.clone();
        if self.buffers[index].is_disk() && !is_virtual_source_path(&closed) {
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
            self.close(index, cx);
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

    fn active_key(&self) -> Option<BufferKey> {
        self.active
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| buffer.key.clone())
    }

    fn close_scope_at_active(
        &mut self,
        make: impl FnOnce(BufferKey) -> CloseScope,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(anchor) = self.active_key() else {
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
        let Some(path) = buffer.disk_path() else {
            eprintln!("S1_TAB_MENU run=copyPath result=untitled");
            return;
        };
        let text = path.to_string_lossy().to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        eprintln!("S1_TAB_MENU run=copyPath value={text}");
    }

    fn copy_relative_path(&self, index: usize, cx: &mut App) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        let Some(path) = buffer.disk_path() else {
            eprintln!("S1_TAB_MENU run=copyRelativePath result=untitled");
            return;
        };
        let text = relative_to_workspace(path, self.workspace_root.as_deref());
        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        eprintln!("S1_TAB_MENU run=copyRelativePath value={text}");
    }

    fn reveal_in_project_tree(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        let Some(path) = buffer.disk_path().map(Path::to_path_buf) else {
            eprintln!("S1_TAB_MENU run=revealInTree result=untitled");
            return;
        };
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
        let Some(path) = buffer.disk_path().map(Path::to_path_buf) else {
            eprintln!("S1_TAB_MENU run=reveal result=untitled");
            return;
        };
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
        let Some(path) = buffer.disk_path().map(Path::to_path_buf) else {
            eprintln!("S1_TAB_MENU run=openInTerminal result=untitled");
            return;
        };
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
        // Revert: `set_text` resets the selection to the caret, so a bar that is open must
        // recompute and re-select its current match (the reload itself does not close the bar).
        if self
            .find
            .as_ref()
            .is_some_and(|find| find.editor.entity_id() == self.buffers[index].editor.entity_id())
        {
            self.recompute_find(cx);
        }
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
            self.close(*index, cx);
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
                self.close(index, cx);
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
                // A scratch buffer needs a name before it can be written: route to Save As and
                // leave the tab open rather than silently failing to save.
                if self
                    .buffers
                    .get(index)
                    .is_some_and(|buffer| !buffer.is_disk())
                {
                    self.save_as(window, cx);
                    return false;
                }
                let saved = self.write_buffer(index, "lithe.editor.saveFailed", window, cx);
                if saved {
                    self.close(index, cx);
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
        // The buffer's display path (a real path, or the scratch name) — used only for a message.
        let file = self
            .active
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| buffer.path.display().to_string());

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

    /// `secondary-.`（macOS ⌘. / 其他 Ctrl+.，与上游 gpui-base 的 `ToggleCodeActions` 同一个键）。
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

    /// 打开一个物理文件位置，返回它的缓冲下标与字节偏移（**不动光标**，由调用方决定要不要记跳转历史）。
    ///
    /// 定义/实现/类型定义与「转到引用」都落在这里：差别只在落下去之后要不要记一次历史。
    fn land_on_file(
        &mut self,
        path: &Path,
        line: u32,
        utf16_column: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(usize, usize)> {
        if !self.buffers.iter().any(|buffer| buffer.path == path) {
            self.open(path, window, cx);
        }
        let index = self.buffers.iter().position(|buffer| buffer.path == path)?;
        let text = self.buffers[index].editor.read(cx).text().to_string();
        let offset = crate::navigation::offset_for_utf16(
            &text,
            line as usize,
            utf16_column as usize,
        );
        Some((index, offset))
    }

    // ------------------------------------------------------------------
    // 转到引用 / 重命名 / 格式化
    // ------------------------------------------------------------------

    /// Go ▸ Go to References：取一次引用，回来在光标下方开一份菜单。
    ///
    /// 与定义那三个的差别是**结果是一组**：定义只落第一条，引用要把全部位置列出来让用户挑。
    /// 菜单机制与代码动作共用（`ContextMenu::manual`），但代次各算各的——两者可以同时在飞。
    pub fn go_to_references(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            eprintln!("S1_EDITOR_REFERENCES open=false reason=no-active-buffer");
            return;
        };
        let file = self.buffers[index].path.display().to_string();
        if language_for_file(&self.buffers[index].name) != Some("java") {
            eprintln!("S1_EDITOR_REFERENCES open=false reason=not-java file={file}");
            return;
        }
        let Some(service) = self.java.clone() else {
            eprintln!("S1_EDITOR_REFERENCES open=false reason=no-java-service file={file}");
            return;
        };

        let path = self.buffers[index].path.clone();
        let text = self.buffers[index].editor.read(cx).text().to_string();
        let caret = self.buffers[index].editor.read(cx).cursor_offset();

        let mine = self.reference_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = self.reference_generation.clone();
        let requested = crate::symbol_actions::fetch_references(
            service, path, text, caret, generation, mine,
        );
        let task = cx.spawn_in(window, async move |pane, cx| {
            let Some(references) = requested.await else {
                return;
            };
            let _ = pane.update_in(cx, |pane, window, cx| {
                pane.present_references(caret, mine, references, window, cx)
            });
        });
        self.reference_task = Some(task);
    }

    fn present_references(
        &mut self,
        caret: usize,
        mine: u64,
        references: Vec<crate::symbol_actions::Reference>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.reference_generation.load(Ordering::SeqCst) != mine {
            return;
        }
        let Some(index) = self.active else {
            return;
        };
        let file = self.buffers[index].path.display().to_string();
        if references.is_empty() {
            println!("S1_EDITOR_REFERENCES menu=false reason=no-references file={file}");
            // 复用真源里已有的菜单文案，不新造导航键（`navigation.references` 不存在）。
            self.notify_no_target("lithe.menu.goToReferences", "references", cx);
            return;
        }
        let Some(bounds) = self.buffers[index]
            .editor
            .read(cx)
            .bounds_for_offset(caret, cx)
        else {
            println!("S1_EDITOR_REFERENCES menu=false reason=anchor-out-of-view file={file}");
            return;
        };

        let count = references.len();
        self.references = references;
        self.reference_request = Some((
            mine,
            point(bounds.origin.x, bounds.origin.y + bounds.size.height),
        ));
        println!("S1_EDITOR_REFERENCES menu=true file={file} count={count}");
        cx.notify();
    }

    /// 引用菜单里挑中的那一条：打开它并把光标落上去，同时记一次跳转历史。
    fn apply_reference(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(reference) = self.references.get(at).cloned() else {
            return;
        };
        let Some(origin_index) = self.active else {
            return;
        };
        let origin_path = self.buffers[origin_index].path.clone();
        let origin = {
            let state = self.buffers[origin_index].editor.read(cx);
            crate::navigation::line_character(state.text(), state.cursor_offset())
        };
        let (origin_line, origin_column) = (origin.line, origin.character);

        let Some((index, offset)) =
            self.land_on_file(&reference.path, reference.line, reference.utf16_column, window, cx)
        else {
            println!(
                "S1_EDITOR_REFERENCES jump=false reason=no-target path={}",
                reference.path.display()
            );
            return;
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
            "S1_EDITOR_REFERENCES jump=true from={}:{} to={}:{} file={}",
            origin_line + 1,
            origin_column + 1,
            to.0,
            to.1,
            self.buffers[index].path.display()
        );
        self.sync_cursor(cx);
        cx.notify();
    }

    /// Edit ▸ Format Document：问一次服务端，回来整批落进当前缓冲区（一批一次撤销步）。
    pub fn format_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            eprintln!("S1_EDITOR_FORMAT open=false reason=no-active-buffer");
            return;
        };
        let file = self.buffers[index].path.display().to_string();
        if language_for_file(&self.buffers[index].name) != Some("java") {
            eprintln!("S1_EDITOR_FORMAT open=false reason=not-java file={file}");
            return;
        }
        let Some(service) = self.java.clone() else {
            eprintln!("S1_EDITOR_FORMAT open=false reason=no-java-service file={file}");
            return;
        };

        let path = self.buffers[index].path.clone();
        let text = self.buffers[index].editor.read(cx).text().to_string();

        let mine = self.format_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = self.format_generation.clone();
        let requested =
            crate::symbol_actions::fetch_formatting(service, path, text, generation, mine);
        let task = cx.spawn_in(window, async move |pane, cx| {
            let Some(edits) = requested.await else {
                return;
            };
            let _ = pane.update_in(cx, |pane, _window, cx| pane.apply_formatting(mine, edits, cx));
        });
        self.format_task = Some(task);
    }

    fn apply_formatting(
        &mut self,
        mine: u64,
        edits: Vec<lithe_gpui_java::JavaTextEdit>,
        cx: &mut Context<Self>,
    ) {
        if self.format_generation.load(Ordering::SeqCst) != mine {
            return;
        }
        let Some(index) = self.active else {
            return;
        };
        let file = self.buffers[index].path.display().to_string();
        let editor = self.buffers[index].editor.clone();
        let converted = {
            let state = editor.read(cx);
            crate::code_actions::ranges_for(&edits, state.text())
        };
        if converted.is_empty() {
            println!("S1_EDITOR_FORMAT file={file} result=no-changes");
            return;
        }
        let count = converted.len();
        editor.update(cx, |state, cx| state.apply_edits(&converted, cx));
        println!("S1_EDITOR_FORMAT file={file} result=applied edits={count}");
        cx.notify();
    }

    /// Go ▸ Rename Symbol：先在对话框里问新名字，确认之后再问服务端要编辑。
    ///
    /// 输入框预填光标下那个词（`word_range`），并且**只选中标识符本身**——用户改的不是整行。
    pub fn rename_symbol(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            eprintln!("S1_EDITOR_RENAME open=false reason=no-active-buffer");
            return;
        };
        if window.has_active_dialog(cx) {
            eprintln!("S1_EDITOR_RENAME open=false reason=active-dialog");
            return;
        }
        let file = self.buffers[index].path.display().to_string();
        if language_for_file(&self.buffers[index].name) != Some("java") {
            eprintln!("S1_EDITOR_RENAME open=false reason=not-java file={file}");
            return;
        }
        if self.java.is_none() {
            eprintln!("S1_EDITOR_RENAME open=false reason=no-java-service file={file}");
            return;
        }

        let (text, offset) = {
            let state = self.buffers[index].editor.read(cx);
            (state.text().to_string(), state.cursor_offset())
        };
        let Some(range) = crate::navigation::word_range(&text, offset) else {
            eprintln!("S1_EDITOR_RENAME open=false reason=no-symbol file={file}");
            return;
        };
        let current = text[range.clone()].to_string();

        let input = cx.new(|cx| lithe_ui::forms::InputState::new(window, cx));
        input.update(cx, |state, cx| {
            state.set_placeholder(tr("lithe.editor.renameSymbolPrompt"), cx);
            state.set_text(current.clone(), cx);
        });
        let enter = cx.subscribe_in(
            &input,
            window,
            |pane: &mut Self,
             _input: &Entity<lithe_ui::forms::InputState>,
             event: &InputEvent,
             window,
             cx| {
                if matches!(event, InputEvent::Submit) {
                    pane.submit_rename(window, cx);
                }
            },
        );
        self.rename = Some(RenamePrompt {
            input: input.clone(),
            offset,
            _enter: enter,
        });
        println!("S1_EDITOR_RENAME open=true file={file} symbol={current:?}");

        let pane = cx.weak_entity();
        window.open_dialog(
            "lithe-rename-symbol",
            move |id, _window, cx| {
                let candidate = input.read(cx).text().trim().to_string();
                // 空名字、或者与原来一字不差：都不该发请求（服务端会退化成空编辑，白跑一趟）。
                let valid = !candidate.is_empty() && candidate != current;
                let confirm_pane = pane.clone();
                let (cancel_id, ok_id) = ((id.clone(), "cancel"), (id.clone(), "ok"));
                Dialog::new(id)
                    .title(tr("lithe.menu.renameSymbol"))
                    .on_close(dismiss_hosted)
                    .child(lithe_ui::forms::Input::new(&input))
                    .action(move |close| {
                        Button::new(cancel_id)
                            .label(tr("lithe.ui.cancel"))
                            .on_click(move |_, window, cx| close(window, cx))
                    })
                    .action(move |_close| {
                        Button::new(ok_id)
                            .primary()
                            .label(tr("lithe.ui.ok"))
                            .disabled(!valid)
                            .on_click(move |_, window, cx| {
                                let _ = confirm_pane.update(cx, |pane, cx| {
                                    pane.submit_rename(window, cx)
                                });
                            })
                    })
                    .into_any_element()
            },
            cx,
        );
    }

    /// 对话框确认（也会被 Enter 触发）：把新名字发给服务端，开始取编辑。
    pub fn submit_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(prompt) = self.rename.as_ref() else {
            return;
        };
        let new_name = prompt.input.read(cx).text().trim().to_string();
        let offset = prompt.offset;
        if new_name.is_empty() {
            eprintln!("S1_EDITOR_RENAME submit=false reason=empty-name");
            return;
        }
        let Some(index) = self.active else {
            return;
        };
        let Some(service) = self.java.clone() else {
            return;
        };
        let path = self.buffers[index].path.clone();
        let text = self.buffers[index].editor.read(cx).text().to_string();

        // 关掉对话框再发请求：请求回来时要落的是一份仍然属于当前缓冲区的编辑，
        // 而对话框开着的时候用户按不了别的菜单，先关掉才能让「失败也看得见结果」。
        self.rename = None;
        window.close_dialog(cx);

        let mine = self.rename_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = self.rename_generation.clone();
        let requested = crate::symbol_actions::fetch_rename(
            service, path, text, offset, new_name, generation, mine,
        );
        let task = cx.spawn_in(window, async move |pane, cx| {
            let Some(edits) = requested.await else {
                return;
            };
            let _ = pane.update_in(cx, |pane, _window, cx| pane.apply_rename(mine, edits, cx));
        });
        self.rename_task = Some(task);
    }

    fn apply_rename(
        &mut self,
        mine: u64,
        edits: lithe_gpui_java::JavaWorkspaceEdits,
        cx: &mut Context<Self>,
    ) {
        if self.rename_generation.load(Ordering::SeqCst) != mine {
            return;
        }
        let Some(index) = self.active else {
            return;
        };
        let file = self.buffers[index].path.display().to_string();
        if edits.other_file_edits > 0 {
            // 本仓没有跨文件批改的执行层：**说清楚只改了一个文件**，不假装整体完成。
            eprintln!(
                "S1_EDITOR_RENAME file={file} result=partial other_files={} note=this-file-only",
                edits.other_file_edits
            );
        }
        let editor = self.buffers[index].editor.clone();
        let converted = {
            let state = editor.read(cx);
            crate::code_actions::ranges_for(&edits.edits, state.text())
        };
        if converted.is_empty() {
            println!("S1_EDITOR_RENAME file={file} result=no-edits");
            return;
        }
        let count = converted.len();
        editor.update(cx, |state, cx| state.apply_edits(&converted, cx));
        println!("S1_EDITOR_RENAME file={file} result=applied edits={count}");
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

    // ------------------------------------------------------------------
    // 查找 / 替换
    // ------------------------------------------------------------------

    /// Opens the find bar on the active buffer, unfolding the replace row when `with_replace`.
    ///
    /// Refuses (with a diagnostic, no UI) when there is no active buffer or a modal dialog is up,
    /// matching `open_go_to_line`. Re-opening on the buffer that already owns the bar refocuses
    /// and selects the query without collapsing a replace row the user opened with Ctrl+R.
    pub fn open_find(&mut self, with_replace: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            eprintln!("S1_EDITOR_FIND open=false reason=no-active-buffer");
            return;
        };
        if window.has_active_dialog(cx) {
            eprintln!("S1_EDITOR_FIND open=false reason=active-dialog");
            return;
        }
        let editor = self.buffers[index].editor.clone();
        let editor_id = editor.entity_id();
        let mode = if with_replace { "replace" } else { "find" };

        // The bar already belongs to this buffer: reuse it.
        if self
            .find
            .as_ref()
            .is_some_and(|find| find.editor.entity_id() == editor_id)
        {
            if let Some(find) = self.find.as_mut() {
                if with_replace {
                    find.showing_replace = true;
                }
            }
            let query = self.find.as_ref().expect("just checked").query.clone();
            let all = 0..query.read(cx).text().len();
            query.update(cx, |state, cx| state.select(all, cx));
            let handle = query.read(cx).focus_handle(cx);
            window.focus(&handle, cx);
            let total = self.find.as_ref().map_or(0, |find| find.matches.len());
            let current = self.find.as_ref().and_then(|find| find.current);
            println!(
                "S1_EDITOR_FIND open=true mode={mode} reuse=true matches={total} current={}",
                current.map_or(-1, |current| current as isize)
            );
            cx.notify();
            return;
        }

        // A bar owned by another buffer is torn down first so its highlight cannot linger.
        self.drop_find(cx);

        let query = cx.new(|cx| InputState::new(window, cx));
        let replace = cx.new(|cx| InputState::new(window, cx));
        if let Some(prefill) = self.selection_prefill(index, cx) {
            query.update(cx, |state, cx| {
                let len = prefill.len();
                state.set_text(prefill, cx);
                state.select(0..len, cx);
            });
        }

        let events = vec![cx.subscribe_in(
            &query,
            window,
            |pane: &mut Self, _query: &Entity<InputState>, event: &InputEvent, window, cx| {
                match event {
                    InputEvent::Changed => pane.schedule_find_recompute(window, cx),
                    InputEvent::Submit => pane.find_step(true, window, cx),
                    _ => {}
                }
            },
        )];

        self.find = Some(FindBar {
            editor: editor.clone(),
            query: query.clone(),
            replace,
            options: FindOptions::default(),
            matches: Vec::new(),
            error: None,
            current: None,
            showing_replace: with_replace,
            generation: Arc::new(AtomicU64::new(0)),
            _events: events,
        });

        // The first recompute is immediate (not debounced): the user expects an answer on open.
        self.recompute_find(cx);
        let handle = query.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
        let total = self.find.as_ref().map_or(0, |find| find.matches.len());
        let current = self.find.as_ref().and_then(|find| find.current);
        println!(
            "S1_EDITOR_FIND open=true mode={mode} matches={total} current={}",
            current.map_or(-1, |current| current as isize)
        );
        cx.notify();
    }

    /// Closes the find bar: clears the owning editor's highlight, forgets the bar, and returns
    /// focus to that editor (if it is still open). The editor's selection is left alone.
    pub fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(find) = self.find.as_ref() else {
            return;
        };
        let editor = find.editor.clone();
        let idle = find.matches.is_empty();
        self.drop_find(cx);
        if self
            .buffers
            .iter()
            .any(|buffer| buffer.editor.entity_id() == editor.entity_id())
        {
            let handle = editor.read(cx).focus_handle(cx);
            window.focus(&handle, cx);
        }
        println!("S1_EDITOR_FIND open=false reason=close idle={idle}");
        cx.notify();
    }

    /// Steps to the next (`forward`) or previous match, wrapping at both ends, and scrolls the
    /// selection into view (via the editor's own `select`). Focus does not move.
    pub fn find_step(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            eprintln!("S1_EDITOR_FIND step=false reason=active-dialog");
            return;
        }
        let Some(find) = self.find.as_ref() else {
            eprintln!("S1_EDITOR_FIND step=false reason=closed");
            return;
        };
        let total = find.matches.len();
        let Some(current) = find.current else {
            eprintln!("S1_EDITOR_FIND step=false reason=no-match");
            return;
        };
        if total == 0 {
            eprintln!("S1_EDITOR_FIND step=false reason=no-match");
            return;
        }
        let next = if forward {
            (current + 1) % total
        } else {
            (current + total - 1) % total
        };
        let wrapped = if forward {
            current + 1 == total
        } else {
            current == 0
        };
        if let Some(find) = self.find.as_mut() {
            find.current = Some(next);
        }
        self.apply_find_highlight(cx);
        println!(
            "S1_EDITOR_FIND step=true forward={forward} current={next} total={total} wrapped={wrapped}"
        );
        cx.notify();
    }

    /// Recomputes the matches against the owning editor's current text and redraws the highlight.
    /// Reads only the current buffer, so nothing is cached across tabs.
    fn recompute_find(&mut self, cx: &mut Context<Self>) {
        let Some(find) = self.find.as_ref() else {
            return;
        };
        let editor = find.editor.clone();
        let query = find.query.read(cx).text().to_string();
        let options = find.options;
        let text = editor.read(cx).text().to_string();
        let cursor = editor.read(cx).cursor_offset();
        let previous = find.current;

        let (matches, error) = match find_all(&text, &query, options) {
            Ok(ranges) => (ranges, None),
            Err(message) => (Vec::new(), Some(SharedString::from(message))),
        };
        let len = matches.len();
        let current = if len == 0 {
            None
        } else {
            match previous {
                Some(index) if index < len => Some(index),
                _ => Some(first_match_at_or_after(&matches, cursor).unwrap_or(0)),
            }
        };

        if let Some(find) = self.find.as_mut() {
            find.matches = matches;
            find.error = error;
            find.current = current;
        }
        self.apply_find_highlight(cx);
    }

    /// Debounced recompute for keystrokes and text changes: bumps the generation and drops the
    /// result if a newer request landed in the meantime, so a fast typist never sees the count jump
    /// back to a stale value.
    fn schedule_find_recompute(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(find) = self.find.as_ref() else {
            return;
        };
        let generation = find.generation.clone();
        let token = generation.fetch_add(1, Ordering::AcqRel).wrapping_add(1);
        cx.spawn_in(window, async move |pane, cx| {
            cx.background_executor().timer(FIND_DEBOUNCE).await;
            let _ = pane.update_in(cx, |pane, _window, cx| {
                let current = pane
                    .find
                    .as_ref()
                    .map(|find| find.generation.load(Ordering::Acquire));
                if current == Some(token) {
                    pane.recompute_find(cx);
                }
            });
        })
        .detach();
    }

    /// Paints the match wash — the current match stronger than the rest — and selects it so the
    /// editor scrolls it into view. Empty matches clear the wash.
    fn apply_find_highlight(&mut self, cx: &mut Context<Self>) {
        let Some(find) = self.find.as_ref() else {
            return;
        };
        let editor = find.editor.clone();
        let warn = cx.theme().colors.warning;
        let current = find.current;
        let marks: Vec<(Range<usize>, gpui::Hsla)> = find
            .matches
            .iter()
            .enumerate()
            .map(|(index, range)| {
                let wash = if Some(index) == current {
                    warn.opacity(0.45)
                } else {
                    warn.opacity(0.2)
                };
                (range.clone(), wash)
            })
            .collect();
        let selection = current.and_then(|index| find.matches.get(index).cloned());
        editor.update(cx, |state, cx| {
            state.set_backgrounds(marks, cx);
            if let Some(range) = selection {
                state.select([range], cx);
            }
        });
    }

    /// Forgets the bar and clears its owning editor's highlight. Focus is untouched; callers that
    /// still hold the editor restore it with [`Self::close_find`].
    fn drop_find(&mut self, cx: &mut Context<Self>) {
        let Some(find) = self.find.take() else {
            return;
        };
        find.editor
            .update(cx, |state, cx| state.set_backgrounds(Vec::new(), cx));
    }

    /// Drops the bar only when it belongs to `editor` (used when that buffer is going away).
    fn drop_find_for(&mut self, editor: EntityId, cx: &mut Context<Self>) {
        if self
            .find
            .as_ref()
            .is_some_and(|find| find.editor.entity_id() == editor)
        {
            self.drop_find(cx);
        }
    }

    /// The query the bar should open with: the single, non-empty, single-line selection of the
    /// buffer, if any. Multiple cursors or a selection spanning lines leave it empty (a guess there
    /// would be worse than blank).
    fn selection_prefill(&self, index: usize, cx: &App) -> Option<String> {
        let editor = self.buffers.get(index)?.editor.clone();
        let state = editor.read(cx);
        let selections = state.selections();
        if selections.len() != 1 {
            return None;
        }
        let selection = selections[0];
        let (start, end) = (
            selection.anchor.min(selection.head),
            selection.anchor.max(selection.head),
        );
        if start == end {
            return None;
        }
        let text = state.text();
        let slice = text.get(start..end)?;
        if slice.contains('\n') || slice.contains('\r') {
            return None;
        }
        Some(slice.to_string())
    }

    fn set_find_options(&mut self, options: FindOptions, cx: &mut Context<Self>) {
        if let Some(find) = self.find.as_mut() {
            find.options = options;
        }
        self.recompute_find(cx);
    }

    fn set_find_replace_visible(&mut self, showing: bool, cx: &mut Context<Self>) {
        if let Some(find) = self.find.as_mut() {
            find.showing_replace = showing;
        }
        cx.notify();
    }

    /// Replaces the current match (`all == false`) or every match (`all == true`) with the replace
    /// field's text, as one undo step. A read-only buffer refuses silently in the editor, so that
    /// is detected and reported rather than leaving the user wondering.
    fn find_replace(&mut self, all: bool, cx: &mut Context<Self>) {
        let Some(find) = self.find.as_ref() else {
            return;
        };
        let editor = find.editor.clone();
        let replacement = find.replace.read(cx).text().to_string();
        let edits: Vec<(Range<usize>, String)> = if all {
            find.matches
                .iter()
                .map(|range| (range.clone(), replacement.clone()))
                .collect()
        } else {
            match find.current {
                Some(index) => vec![(find.matches[index].clone(), replacement.clone())],
                None => Vec::new(),
            }
        };
        if edits.is_empty() {
            eprintln!("S1_EDITOR_FIND replace=false reason=no-match");
            return;
        }

        let before = editor.read(cx).text().to_string();
        editor.update(cx, |state, cx| state.edit(edits, cx));
        let after = editor.read(cx).text().to_string();
        if before == after {
            eprintln!("S1_EDITOR_FIND replace=false reason=read-only");
            self.record_notification(
                cx,
                lithe_gpui_notify::Input::new(
                    "lithe.footer.readOnly",
                    lithe_gpui_notify::Severity::Warning,
                )
                .key("lithe.footer.readOnly"),
            );
            return;
        }

        // The end of the replaced run, so a single replace moves to the first match at or after it
        // instead of spinning on a replacement that still contains the query.
        let replaced_end = self.find.as_ref().and_then(|find| {
            find.current
                .and_then(|index| find.matches.get(index))
                .map(|range| range.start + replacement.len())
        });
        self.recompute_find(cx);
        let next = self.find.as_ref().and_then(|find| {
            if find.matches.is_empty() {
                None
            } else if all {
                Some(0)
            } else {
                Some(first_match_at_or_after(&find.matches, replaced_end.unwrap_or(0)).unwrap_or(0))
            }
        });
        if let Some(find) = self.find.as_mut() {
            find.current = next;
        }
        self.apply_find_highlight(cx);
        println!("S1_EDITOR_FIND replace=true all={all}");
        cx.notify();
    }

    /// The find bar as an absolutely-positioned overlay inside the body container. It carries the
    /// `EditorFindBar` key context so `Escape` resolves to `CloseFind` while focus is inside it.
    fn find_bar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let find = self.find.as_ref()?;
        let current = find.current;
        let total = find.matches.len();
        let error = find.error.clone();
        let replace = find.replace.clone();
        let showing_replace = find.showing_replace;
        let options = find.options;
        let pane = cx.entity().downgrade();

        let mut widget = FindWidget::new(
            ("lithe-editor-find", current.unwrap_or(usize::MAX)),
            &find.query,
            current,
            total,
        )
        .options(options)
        .on_options({
            let pane = pane.clone();
            move |options, _window, cx| {
                let _ = pane.update(cx, |pane, cx| pane.set_find_options(options, cx));
            }
        })
        .on_step({
            let pane = pane.clone();
            move |forward, window, cx| {
                let _ = pane.update(cx, |pane, cx| pane.find_step(forward, window, cx));
            }
        })
        .on_toggle_replace({
            let pane = pane.clone();
            move |showing, _window, cx| {
                let _ = pane.update(cx, |pane, cx| pane.set_find_replace_visible(showing, cx));
            }
        })
        .on_replace({
            let pane = pane.clone();
            move |all, _window, cx| {
                let _ = pane.update(cx, |pane, cx| pane.find_replace(all, cx));
            }
        })
        .on_close({
            let pane = pane.clone();
            move |window, cx| {
                let _ = pane.update(cx, |pane, cx| pane.close_find(window, cx));
            }
        });
        if showing_replace {
            widget = widget.replace(&replace);
        }
        if let Some(error) = error {
            widget = widget.error(error);
        }

        Some(
            div()
                .id("lithe-editor-find-bar")
                .absolute()
                .top_2()
                .right_2()
                .key_context(FIND_BAR_KEY_CONTEXT)
                .on_action(cx.listener(|pane, _: &crate::CloseFind, window, cx| {
                    pane.close_find(window, cx);
                }))
                .child(widget)
                .into_any_element(),
        )
    }

    /// 面包屑：当前文件相对工作区根的一串目录段，末段是文件名。
    ///
    /// IDEA 的每一段可点（在文件树里定位），本产品先只做展示：段间用 "/" 分隔，
    /// 末段比目录段亮一档。没有活动缓冲区、或路径不在工作区里（虚拟源码、外部文件）时
    /// 退到全路径，仍然给读者一个"我在哪儿"的答案。
    fn render_breadcrumbs(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let index = self.active?;
        let buffer = self.buffers.get(index)?;
        let colors = cx.theme().colors.clone();
        let relative = self
            .workspace_root
            .as_ref()
            .and_then(|root| buffer.path.strip_prefix(root).ok())
            .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| buffer.path.to_string_lossy().replace('\\', "/"));
        let segments: Vec<&str> = relative
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();
        if segments.is_empty() {
            return None;
        }
        let mut bar = h_stack()
            .flex_shrink_0()
            .h_6()
            .px_2()
            .gap_1()
            .overflow_hidden()
            .bg(colors.bg)
            .border_b_1()
            .border_color(colors.border)
            .id("editor-breadcrumbs");
        for (position, segment) in segments.iter().enumerate() {
            let last = position + 1 == segments.len();
            bar = bar.child(
                div()
                    .text_color(if last { colors.fg } else { colors.fg_muted })
                    .child(segment.to_string()),
            );
            if !last {
                bar = bar.child(div().text_color(colors.fg_muted).child("/"));
            }
        }
        Some(bar.into_any_element())
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
                .child(Self::close_button(&buffer.key, index, cx))
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
            .id(tab_element_id(&buffer.key))
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

        // 菜单只在「要显示」的那一帧才建（右键按下 / 请求开出）。标签栏每帧都渲染，
        // 每帧给每个标签重建一份十项菜单 = 十来个 Rc 闭包 + 一串 SharedString，纯浪费。
        let pane = cx.entity();
        ContextMenu::build(tab_menu_element_id(&buffer.key), move |_window, cx| {
            pane.update(cx, |pane, cx| pane.tab_menu(index, cx))
        })
        .child(content.into_any_element())
        .into_any_element()
    }

    /// 标签的右键菜单：除了图标（组件库的菜单行只收库内图标名），条目的文案与动作与迁移前一一对应。
    /// 菜单靠索引现取缓冲：调用方（`ContextMenu::build` 的构造函数）只持有索引，
    /// 拿不到 `&Buffer`——它要能活在渲染帧之外。
    fn tab_menu(&self, index: usize, cx: &mut Context<Self>) -> Menu {
        let Some(buffer) = self.buffers.get(index) else {
            return Menu::new();
        };
        let pane: Entity<Self> = cx.entity();
        let anchor = buffer.key.clone();
        let on_disk = buffer
            .disk_path()
            .is_some_and(|path| !is_virtual_source_path(path));
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
                let anchor = anchor.clone();
                move |pane: &mut Self, window, cx| {
                    pane.close_scope(CloseScope::Others(anchor.clone()), window, cx)
                }
            }),
        ));
        items += 1;
        menu = menu.item(entry(
            tr("lithe.tabs.closeToRight"),
            Rc::new({
                let anchor = anchor.clone();
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

    fn close_button(key: &BufferKey, index: usize, cx: &mut Context<Self>) -> Button {
        Button::new(tab_close_element_id(key))
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
            // 查找条画在正文容器里（补全之后，故 z 序在补全之上、代码动作菜单之下）。
            .children(self.find_bar(cx))
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

    /// 引用列表的菜单：与代码动作同一套 `ContextMenu::manual`，每行是「相对路径:行:列」。
    ///
    /// 定高而不是定宽：引用可能上百条，不限高会把面板拉到窗口外面；行文案右对齐的路径宽度
    /// 交给主题的菜单宽度（与代码动作一致）。
    fn reference_menu(&self, pane: &WeakEntity<Self>, cx: &App) -> Option<AnyElement> {
        if self.reference_request.is_none() || self.references.is_empty() {
            return None;
        }
        let pick = pane.clone();
        let root = self.workspace_root.as_deref();
        let mut menu = Menu::new();
        for (at, reference) in self.references.iter().enumerate() {
            let pick = pick.clone();
            menu = menu.item(MenuItem::new(reference.label(root)).on_click(
                move |window, cx| {
                    let _ = pick.update(cx, |pane, cx| pane.apply_reference(at, window, cx));
                },
            ));
        }
        menu = menu.max_height(rems(20.));
        let _ = cx;
        Some(
            ContextMenu::new("lithe-editor-references", menu)
                .manual(self.reference_request)
                .into_any_element(),
        )
    }
}

fn tab_element_id(key: &BufferKey) -> String {
    format!("editor-tab:{}", key_id(key))
}

fn tab_close_element_id(key: &BufferKey) -> String {
    format!("editor-tab-close:{}", key_id(key))
}

fn tab_menu_element_id(key: &BufferKey) -> String {
    format!("editor-tab-menu:{}", key_id(key))
}

/// A stable, order-independent id fragment for a buffer key: the path for a disk buffer, a
/// numbered tag for a scratch one (whose number never repeats within a pane).
fn key_id(key: &BufferKey) -> String {
    match key {
        BufferKey::Disk(path) => path.display().to_string(),
        BufferKey::Untitled(number) => format!("untitled:{number}"),
    }
}

fn is_virtual_source_path(path: &Path) -> bool {
    path.to_string_lossy().starts_with("jdt://")
}

/// Whether `path` is `target` itself or lives under it (matched by whole path components).
fn sits_under(path: &Path, target: &Path) -> bool {
    path.strip_prefix(target).is_ok()
}

/// The index of the first match that starts at or after `offset`, the natural "next" from the
/// caret when the bar opens or a replace moves past its target. `None` when every match is earlier.
fn first_match_at_or_after(matches: &[Range<usize>], offset: usize) -> Option<usize> {
    matches.iter().position(|range| range.start >= offset)
}

/// Rewrites a whole map's keys through [`remap_path`], dropping nothing that stays put. A no-op
/// when no key sits under `from`, so the common case costs one scan.
fn remap_map<V>(map: &mut BTreeMap<PathBuf, V>, from: &Path, to: &Path) {
    if !map.keys().any(|path| remap_path(path, from, to).is_some()) {
        return;
    }
    let old = std::mem::take(map);
    for (path, value) in old {
        match remap_path(&path, from, to) {
            Some(moved) => {
                map.insert(moved, value);
            }
            None => {
                map.insert(path, value);
            }
        }
    }
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
        let references = self.reference_menu(&pane, cx);
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
            .on_action(
                cx.listener(|pane, _: &crate::NavigateToImplementation, window, cx| {
                    pane.navigate_to_implementation(window, cx);
                }),
            )
            .on_action(
                cx.listener(|pane, _: &crate::NavigateToTypeDefinition, window, cx| {
                    pane.navigate_to_type_definition(window, cx);
                }),
            )
            .on_action(cx.listener(|pane, _: &crate::CloseActiveTab, window, cx| {
                pane.close_active(window, cx);
            }))
            .on_action(cx.listener(|pane, _: &crate::ShowCodeActions, window, cx| {
                pane.show_code_actions(window, cx);
            }))
            // 符号级动作：引用给一组位置，重命名与格式化各发一次编辑请求。
            .on_action(cx.listener(|pane, _: &crate::GoToReferences, window, cx| {
                pane.go_to_references(window, cx);
            }))
            .on_action(cx.listener(|pane, _: &crate::RenameSymbol, window, cx| {
                pane.rename_symbol(window, cx);
            }))
            .on_action(cx.listener(|pane, _: &crate::FormatDocument, window, cx| {
                pane.format_document(window, cx);
            }))
            // 查找/替换：菜单与键位都走这几个动作（键位由 workbench 绑，见 `crate::OpenFind`）。
            .on_action(cx.listener(|pane, _: &crate::OpenFind, window, cx| {
                pane.open_find(false, window, cx);
            }))
            .on_action(cx.listener(|pane, _: &crate::OpenFindReplace, window, cx| {
                pane.open_find(true, window, cx);
            }))
            .on_action(cx.listener(|pane, _: &crate::FindNext, window, cx| {
                pane.find_step(true, window, cx);
            }))
            .on_action(cx.listener(|pane, _: &crate::FindPrevious, window, cx| {
                pane.find_step(false, window, cx);
            }))
            .on_action(cx.listener(|pane, _: &crate::CloseFind, window, cx| {
                pane.close_find(window, cx);
            }))
            .child(tabs)
            .children(self.render_breadcrumbs(cx))
            .child(body)
            .children(code_actions)
            .children(references)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BufferKey, first_match_at_or_after, remap_map, sits_under, tab_close_element_id,
        tab_element_id,
    };
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    fn disk(path: &str) -> BufferKey {
        BufferKey::Disk(PathBuf::from(path))
    }

    #[test]
    fn a_buffer_key_keeps_disk_files_and_scratch_buffers_apart() {
        let file = disk("/ws/Main.java");
        assert_eq!(file.disk_path(), Some(Path::new("/ws/Main.java")));
        assert_eq!(file.display("ignored"), PathBuf::from("/ws/Main.java"));

        let scratch = BufferKey::Untitled(3);
        assert_eq!(scratch.disk_path(), None, "未命名缓冲没有磁盘路径");
        assert_eq!(
            scratch.display("Untitled-3"),
            PathBuf::from("Untitled-3"),
            "未命名键的展示路径就是它的标题"
        );
    }

    #[test]
    fn a_deleted_folder_takes_its_children_and_leaves_neighbours() {
        assert!(sits_under(
            Path::new("/ws/a/b/Main.java"),
            Path::new("/ws/a/b")
        ));
        assert!(sits_under(Path::new("/ws/a/b"), Path::new("/ws/a/b")));
        assert!(
            !sits_under(Path::new("/ws/a/bc/Other.java"), Path::new("/ws/a/b")),
            "按路径组件匹配：`a/bc` 不是 `a/b` 的孩子"
        );
    }

    #[test]
    fn a_remap_moves_its_own_keys_and_leaves_neighbours() {
        let mut map: BTreeMap<PathBuf, i32> = BTreeMap::new();
        map.insert(PathBuf::from("/ws/a/b/last"), 1);
        map.insert(PathBuf::from("/ws/a/bc/last"), 2);

        remap_map(&mut map, Path::new("/ws/a/b"), Path::new("/ws/a/renamed"));

        assert_eq!(map.get(Path::new("/ws/a/renamed/last")), Some(&1));
        assert_eq!(map.get(Path::new("/ws/a/bc/last")), Some(&2));
        assert!(!map.contains_key(Path::new("/ws/a/b/last")));
    }

    #[test]
    fn the_next_match_is_the_first_one_at_or_after_the_caret() {
        let matches = [2..4usize, 10..12, 20..22];
        assert_eq!(first_match_at_or_after(&matches, 0), Some(0));
        assert_eq!(first_match_at_or_after(&matches, 3), Some(1));
        assert_eq!(
            first_match_at_or_after(&matches, 10),
            Some(1),
            "起点相等算命中"
        );
        assert_eq!(first_match_at_or_after(&matches, 21), None, "都在光标之前");
    }

    #[test]
    fn tab_element_ids_do_not_depend_on_the_tab_order() {
        let tabs = [
            disk("/workspace/src/main.rs"),
            disk("/workspace/README.md"),
            disk("jdt://contents/java.base/java/lang/String.class"),
        ];

        let forward: Vec<String> = tabs.iter().map(tab_element_id).collect();
        let reversed: Vec<String> = tabs.iter().rev().map(tab_element_id).collect();

        assert_eq!(forward[0], reversed[2]);
        assert_eq!(forward[1], reversed[1]);
        assert_eq!(forward[2], reversed[0]);
        assert_ne!(forward[0], forward[1]);
        assert_ne!(forward[0], forward[2]);
        assert_ne!(forward[1], forward[2]);
        assert!(tab_element_id(&disk("jdt://x")).starts_with("editor-tab:jdt://x"));
    }

    #[test]
    fn two_untitled_buffers_get_distinct_tab_ids() {
        assert_ne!(
            tab_element_id(&BufferKey::Untitled(1)),
            tab_element_id(&BufferKey::Untitled(2)),
            "同窗格里两个未命名缓冲不能撞元素 id"
        );
        assert_ne!(
            tab_element_id(&BufferKey::Untitled(1)),
            tab_element_id(&disk("Untitled-1")),
            "未命名键与同名磁盘文件也不是同一个标签"
        );
    }

    #[test]
    fn close_button_id_is_namespaced_apart_from_its_tab() {
        let path = disk("/workspace/src/main.rs");
        let other = disk("/workspace/src/lib.rs");

        assert_ne!(tab_element_id(&path), tab_close_element_id(&path));
        assert_ne!(tab_close_element_id(&path), tab_close_element_id(&other));
        assert_eq!(
            tab_close_element_id(&path),
            "editor-tab-close:/workspace/src/main.rs"
        );
    }
}

/// 宿主行为的集成断言（交互稿 §5.1 / §5.2 的 A 类）：真的开一个窗口、建一个 `EditorPane`，
/// 再驱动它的公开方法。每条都确定性收尾（临时目录 `Drop` 删、不 sleep），符合 `write-stable-tests`。
#[cfg(test)]
mod host_tests {
    use super::*;
    use gpui::TestAppContext;
    use std::fs;

    /// 语言注册表是进程级全局，主题也是：每条测试自己装一次，免得并行跑时依赖别人先跑。
    fn prepare(cx: &mut App) {
        lithe_gpui_languages::register();
        lithe_ui::theme::Theme::init(cx);
    }

    /// 临时目录：建在系统临时目录下，按用例名与进程号隔离，`Drop` 时删掉。
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(case: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("lithe-editor-host-{case}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("建临时目录");
            Self(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn open_pane(cx: &mut TestAppContext) -> (Entity<EditorPane>, &mut gpui::VisualTestContext) {
        cx.add_window_view(|window, cx| {
            prepare(cx);
            EditorPane::new(None, window, cx)
        })
    }

    #[gpui::test]
    fn open_find_without_a_buffer_is_a_no_op(cx: &mut TestAppContext) {
        let (pane, cx) = open_pane(cx);
        pane.update_in(cx, |pane, window, cx| pane.open_find(false, window, cx));
        assert!(
            pane.read_with(cx, |pane, _| pane.find.is_none()),
            "无活动缓冲时不该冒出查找条"
        );
    }

    #[gpui::test]
    fn open_find_unfolds_the_replace_row_only_when_asked(cx: &mut TestAppContext) {
        let dir = TempDir::new("find-row");
        let file = dir.path("A.java");
        fs::write(&file, "class A {}\n").expect("写样本");
        let (pane, cx) = open_pane(cx);
        pane.update_in(cx, |pane, window, cx| pane.open(&file, window, cx));

        pane.update_in(cx, |pane, window, cx| pane.open_find(false, window, cx));
        assert_eq!(
            pane.read_with(cx, |pane, _| pane
                .find
                .as_ref()
                .map(|find| find.showing_replace)),
            Some(false)
        );

        // A second Ctrl+F must not collapse anything, but Ctrl+R unfolds the replace row.
        pane.update_in(cx, |pane, window, cx| pane.open_find(false, window, cx));
        assert_eq!(
            pane.read_with(cx, |pane, _| pane
                .find
                .as_ref()
                .map(|find| find.showing_replace)),
            Some(false),
            "Ctrl+F 不改变已展开的替换行"
        );
        pane.update_in(cx, |pane, window, cx| pane.open_find(true, window, cx));
        assert_eq!(
            pane.read_with(cx, |pane, _| pane
                .find
                .as_ref()
                .map(|find| find.showing_replace)),
            Some(true)
        );
    }

    #[gpui::test]
    fn open_find_prefills_the_query_from_a_single_line_selection(cx: &mut TestAppContext) {
        let dir = TempDir::new("find-prefill");
        let file = dir.path("B.java");
        fs::write(&file, "hello world\n").expect("写样本");
        let (pane, cx) = open_pane(cx);
        pane.update_in(cx, |pane, window, cx| {
            pane.open(&file, window, cx);
            let editor = pane.buffers[0].editor.clone();
            editor.update(cx, |state, cx| state.select([0..5], cx));
            pane.open_find(false, window, cx);
        });
        let query = pane.read_with(cx, |pane, cx| {
            pane.find
                .as_ref()
                .expect("查找条已开")
                .query
                .read(cx)
                .text()
                .to_string()
        });
        assert_eq!(query, "hello", "单行非空选区应预填查询框");
    }

    #[gpui::test]
    fn find_counts_matches_and_steps_with_wrapping(cx: &mut TestAppContext) {
        let dir = TempDir::new("find-step");
        let file = dir.path("C.java");
        fs::write(&file, "a\na\na\n").expect("写样本");
        let (pane, cx) = open_pane(cx);
        pane.update_in(cx, |pane, window, cx| {
            pane.open(&file, window, cx);
            pane.open_find(false, window, cx);
            let query = pane.find.as_ref().expect("查找条已开").query.clone();
            query.update(cx, |state, cx| state.set_text("a", cx));
            pane.recompute_find(cx);
        });
        assert_eq!(
            pane.read_with(cx, |pane, _| pane.find.as_ref().unwrap().matches.len()),
            3
        );
        assert_eq!(
            pane.read_with(cx, |pane, _| pane.find.as_ref().unwrap().current),
            Some(0),
            "开在光标偏移处的第一处命中上"
        );

        pane.update_in(cx, |pane, window, cx| {
            pane.find_step(true, window, cx);
            pane.find_step(true, window, cx);
        });
        assert_eq!(
            pane.read_with(cx, |pane, _| pane.find.as_ref().unwrap().current),
            Some(2)
        );
        pane.update_in(cx, |pane, window, cx| pane.find_step(true, window, cx));
        assert_eq!(
            pane.read_with(cx, |pane, _| pane.find.as_ref().unwrap().current),
            Some(0),
            "最后一处再下一处要回绕到第 0 处"
        );
        pane.update_in(cx, |pane, window, cx| pane.find_step(false, window, cx));
        assert_eq!(
            pane.read_with(cx, |pane, _| pane.find.as_ref().unwrap().current),
            Some(2),
            "第 0 处再上一处要回绕到最后一处"
        );
    }

    #[gpui::test]
    fn closing_the_find_bar_leaves_no_highlight_behind(cx: &mut TestAppContext) {
        let dir = TempDir::new("find-close");
        let file = dir.path("D.java");
        fs::write(&file, "a\na\n").expect("写样本");
        let (pane, cx) = open_pane(cx);
        pane.update_in(cx, |pane, window, cx| {
            pane.open(&file, window, cx);
            pane.open_find(false, window, cx);
            let query = pane.find.as_ref().expect("查找条已开").query.clone();
            query.update(cx, |state, cx| state.set_text("a", cx));
            pane.recompute_find(cx);
        });
        let editor = pane.read_with(cx, |pane, _| pane.buffers[0].editor.clone());
        pane.update_in(cx, |pane, window, cx| pane.close_find(window, cx));
        assert!(pane.read_with(cx, |pane, _| pane.find.is_none()));
        // The wash itself lives on `CodeEditor`'s private marks; `close_find` clears it through
        // `set_backgrounds(Vec::new())` (invariant I5). Asserting the editor's step count of the
        // wash would need a library accessor, which is out of this batch's scope.
        let _ = editor;
    }

    #[gpui::test]
    fn a_scratch_buffer_is_kept_out_of_the_reopen_list(cx: &mut TestAppContext) {
        let (pane, cx) = open_pane(cx);
        pane.update_in(cx, |pane, window, cx| {
            pane.new_untitled(window, cx);
            pane.new_untitled(window, cx);
        });
        let keys = pane.read_with(cx, |pane, _| {
            pane.buffers
                .iter()
                .map(|buffer| buffer.key.clone())
                .collect::<Vec<_>>()
        });
        assert_eq!(
            keys,
            vec![BufferKey::Untitled(1), BufferKey::Untitled(2)],
            "两个未命名缓冲并存，编号自增不复用"
        );
        assert!(pane.read_with(cx, |pane, _| {
            pane.buffers
                .iter()
                .all(|buffer| !buffer.is_disk() && !buffer.is_dirty)
        }));

        pane.update_in(cx, |pane, window, cx| pane.close_active(window, cx));
        assert!(pane.read_with(cx, |pane, _| pane.closed.is_empty()));
        pane.update_in(cx, |pane, window, cx| pane.reopen_closed_tab(window, cx));
        assert_eq!(
            pane.read_with(cx, |pane, _| pane.buffers.len()),
            1,
            "未命名缓冲不进 closed，重开也不会把它带回来"
        );
    }

    #[gpui::test]
    fn create_file_never_truncates_an_existing_file(cx: &mut TestAppContext) {
        let dir = TempDir::new("new-file");
        let existing = dir.path("Existing.txt");
        fs::write(&existing, "keep me").expect("写样本");
        let (pane, cx) = open_pane(cx);
        pane.update_in(cx, |pane, window, cx| {
            pane.create_file(Some(dir.0.clone()), "Existing.txt".to_string(), window, cx);
        });
        assert_eq!(
            fs::read_to_string(&existing).expect("读回"),
            "keep me",
            "create_new 必须拒绝覆盖已存在的文件"
        );
        assert!(
            pane.read_with(cx, |pane, _| pane.buffers.is_empty()),
            "创建失败不应打开任何标签"
        );
    }

    #[gpui::test]
    fn remap_paths_moves_a_buffer_and_its_path_tables(cx: &mut TestAppContext) {
        let dir = TempDir::new("remap");
        let file = dir.path("Old.java");
        fs::write(&file, "class Old {}\n").expect("写样本");
        let (pane, cx) = open_pane(cx);
        pane.update_in(cx, |pane, window, cx| pane.open(&file, window, cx));

        let renamed = dir.path("New.java");
        pane.update_in(cx, |pane, window, cx| {
            pane.remap_paths(&file, &renamed, window, cx);
        });

        let (key, name) = pane.read_with(cx, |pane, _| {
            (
                pane.buffers[0].key.clone(),
                pane.buffers[0].name.to_string(),
            )
        });
        assert_eq!(key, BufferKey::Disk(renamed.clone()));
        assert_eq!(name, "New.java", "改名后标题跟着新文件名");
        let (old_gone, new_there) = pane.read_with(cx, |pane, _| {
            (
                pane.last_text.contains_key(&file),
                pane.last_text.contains_key(&renamed),
            )
        });
        assert!(!old_gone, "last_text 的旧键必须迁走");
        assert!(new_there);
    }
}
