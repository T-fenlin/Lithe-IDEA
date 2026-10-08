use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crate::buffer::{Buffer, display_names, icon_for_file, language_for_file, read_body};
use crate::navigation::{JumpEntry, JumpHistory, NavTarget, editor_position, resolve_target};
use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::input::{
    Editor, EditorState, Input, InputEvent, InputState, Position, Rope, RopeExt as _, TabSize,
};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::tab::{Tab, TabBar, TabVariant};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _};
use gpui::{AnyElement, App, AppContext as _, ClipboardItem, Context, Entity, InteractiveElement as _, IntoElement, MouseButton, MouseDownEvent, ParentElement as _, Pixels, Render, ScrollWheelEvent, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Task, WeakEntity, Window, div, point, rems};
use gpui_kit::{Div, TestSupportExt as _, relative};
use lithe_gpui_java::JavaLanguageService;
use lithe_gpui_shared::design::TAB_MAX_WIDTH;
use lithe_gpui_shared::icons::{idea, idea_icon_mono};
use lithe_gpui_shared::{tr, tr_args};
use lithe_ui::overlays::Dialog;
use lithe_ui::shell::{WindowRootExt as _, dismiss_hosted};

pub(crate) use lithe_gpui_shared::rem_px;

const FALLBACK_LINE_HEIGHT: f32 = 20.;

pub const EDITOR_HOST_ELEMENT_ID: &str = "lithe-editor-host";

const AUTO_SAVE_DEBOUNCE: Duration = Duration::from_millis(150);

const AUTO_SAVE_DEFAULT: bool = true;

const SOFT_WRAP_DEFAULT: bool = true;

const BREAKPOINT_GUTTER_WIDTH_SPEC: f32 = 14.;

const BREAKPOINT_MARKER_SIZE_SPEC: f32 = 14.;

const EDITOR_CONTENT_TOP_PADDING_SPEC: f32 = 8.;

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
    origin: Position,
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
    input: Entity<InputState>,
    _enter: Subscription,
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
    _theme_subscription: Subscription,
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
        let theme_subscription = cx.observe_global::<gpui_kit::component::Theme>(|pane, cx| {
            pane.sync_completion_menu_width(cx);
        });
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
            hovered_tab: None,
            closed: Vec::new(),
            _theme_subscription: theme_subscription,
        }
    }

    fn sync_completion_menu_width(&mut self, cx: &mut Context<Self>) {
        let rem = cx.theme().font_size;
        let width = crate::completion::menu_width(rem);
        let mut refreshed = 0usize;
        for buffer in &self.buffers {
            buffer.editor.update(cx, |state, _cx| {
                if crate::completion::set_menu_width(state, width) {
                    refreshed += 1;
                }
            });
        }
        if refreshed > 0 {
            eprintln!(
                "S1_EDITOR_COMPLETION_WIDTH max_width={width:?} rem={rem:?} buffers={refreshed}"
            );
        }
    }

    pub fn set_auto_completion(&mut self, enabled: bool, cx: &mut Context<Self>) {
        use std::sync::atomic::Ordering;
        let changed = self.auto_completion.load(Ordering::Acquire) != enabled;
        self.auto_completion.store(enabled, Ordering::Release);
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

    pub fn set_soft_wrap(&mut self, enabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        let changed = self.soft_wrap != enabled;
        self.soft_wrap = enabled;
        let mut applied = 0usize;
        for buffer in &self.buffers {
            buffer
                .editor
                .update(cx, |state, cx| state.set_soft_wrap(enabled, window, cx));
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
        let tab = TabSize {
            tab_size,
            hard_tabs: false,
        };
        for buffer in &self.buffers {
            buffer
                .editor
                .update(cx, |state, cx| state.set_tab_size(tab, cx));
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

        let mut patched = 0usize;
        for buffer in &self.buffers {
            if crate::buffer::language_for_file(&buffer.name) != Some("java") {
                continue;
            }
            let provider = crate::completion::JavaCompletionProvider::new(
                Some(service.clone()),
                buffer.path.clone(),
                self.auto_completion.clone(),
            );
            let code_actions = crate::code_actions::JavaCodeActionProvider::new(
                service.clone(),
                buffer.path.clone(),
            );
            let hover =
                crate::hover::LitheHoverProvider::new(Some(service.clone()), buffer.path.clone());
            let rem = cx.theme().font_size;
            buffer.editor.update(cx, |state, _cx| {
                crate::completion::install(state, Some(provider), rem);
                crate::code_actions::install(state, Some(code_actions));
                crate::hover::install(state, Some(hover));
            });
            patched += 1;
        }
        if patched > 0 {
            println!("S1_JAVA_COMPLETION reattached buffers={patched}");
        }

        let java_buffers: Vec<usize> = self
            .buffers
            .iter()
            .enumerate()
            .filter(|(_, buffer)| crate::buffer::language_for_file(&buffer.name) == Some("java"))
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
            || crate::buffer::language_for_file(&buffer.name) != Some("java")
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
            let written = crate::diagnostics::apply(state, &fetched.diagnostics);
            cx.notify();
            written
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
            .map(|buffer| JumpEntry {
                path: buffer.path.clone(),
                position: buffer.editor.read(cx).cursor_position(),
            });

        if !self.buffers.iter().any(|buffer| buffer.path == path) {
            self.open(path, window, cx);
        }
        let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) else {
            return false;
        };

        let text = self.buffers[index].editor.read(cx).text().clone();
        let position = editor_position(&text, line as usize, utf16_column as usize);
        if let Some(origin) = origin {
            self.history.record(origin);
        }
        let to = self.buffers[index].editor.update(cx, |state, cx| {
            state.set_cursor_position(position, window, cx);
            state.cursor_position()
        });
        self.active = Some(index);

        println!(
            "S1_NAV_JUMP to={}:{} kind={} file={}",
            to.line + 1,
            to.character + 1,
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

    fn tab(&self) -> TabSize {
        TabSize {
            tab_size: self.tab_size,
            hard_tabs: false,
        }
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
        let language = language_for_file(&name);
        crate::languages::ensure_extra_languages();
        let tab = self.tab();
        let completion_provider = (language == Some("java")).then(|| {
            crate::completion::JavaCompletionProvider::new(
                self.java.clone(),
                path.clone(),
                self.auto_completion.clone(),
            )
        });

        let code_action_provider = (language == Some("java"))
            .then(|| self.java.clone())
            .flatten()
            .map(|service| crate::code_actions::JavaCodeActionProvider::new(service, path.clone()));

        let hover_provider = Some(crate::hover::LitheHoverProvider::new(
            (language == Some("java"))
                .then(|| self.java.clone())
                .flatten(),
            path.clone(),
        ));

        let editor = cx.new(|cx| {
            let mut state = EditorState::new(window, cx).searchable(true);
            if let Some(language) = language {
                state = state.language(language);
            }
            state.set_tab_size(tab, cx);
            crate::completion::install(&mut state, completion_provider, window.rem_size());
            crate::code_actions::install(&mut state, code_action_provider);
            crate::hover::install(&mut state, hover_provider);
            if !writable {
                state.set_readonly(true, cx);
            }
            state
        });
        editor.update(cx, |state, cx| state.set_value(body.text, window, cx));
        if !self.soft_wrap {
            editor.update(cx, |state, cx| state.set_soft_wrap(false, window, cx));
        }
        println!(
            "S1_EDITOR_LANG name={} file={name}",
            editor.read(cx).language_name()
        );

        let subscriptions = vec![
            cx.subscribe_in(
                &editor,
                window,
                |pane: &mut Self,
                 editor: &gpui_kit::Entity<EditorState>,
                 event: &InputEvent,
                 window,
                 cx| {
                    if matches!(event, InputEvent::Change) {
                        pane.on_input_change(editor, window, cx);
                    }
                },
            ),
            cx.observe(&editor, |pane: &mut Self, _editor, cx| pane.sync_cursor(cx)),
        ];

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
        if language == Some("java") {
            let opened = self.buffers.len() - 1;
            self.schedule_diagnostics(opened, Duration::ZERO, cx);
        }
        cx.notify();
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
                let position = buffer.editor.read(cx).cursor_position();
                CursorPosition {
                    line: position.line as usize + 1,
                    column: position.character as usize + 1,
                }
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
            (
                state.cursor(),
                state.cursor_position(),
                state.text().clone(),
            )
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

        let (index, position, via) = match target {
            NavTarget::Local(position) => (origin_index, position, "builtin"),
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
                let text = self.buffers[index].editor.read(cx).text().clone();
                (
                    index,
                    editor_position(&text, line as usize, utf16_column as usize),
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
                let position = editor_position(
                    &Rope::from(text.as_str()),
                    line as usize,
                    utf16_column as usize,
                );
                let index = self.open_virtual(&uri, &display_path, text, window, cx);
                (index, position, "jdtls-virtual")
            }
        };

        self.history.record(JumpEntry {
            path: origin_path,
            position: origin,
        });
        let to = self.buffers[index].editor.update(cx, |state, cx| {
            state.set_cursor_position(position, window, cx);
            state.cursor_position()
        });
        self.active = Some(index);

        println!(
            "S1_NAV_JUMP from={}:{} to={}:{} kind=definition via={via} file={}",
            origin.line + 1,
            origin.character + 1,
            to.line + 1,
            to.character + 1,
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

        let inside = self
            .active
            .and_then(|index| self.buffers.get(index))
            .is_some_and(|buffer| {
                buffer
                    .editor
                    .read(cx)
                    .input_bounds()
                    .contains(&event.position)
            });
        if !inside {
            return;
        }

        self.navigate_to_definition(window, cx);
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
        let language = language_for_file(&name);
        crate::languages::ensure_extra_languages();
        let tab = self.tab();
        let hover_provider = Some(crate::hover::LitheHoverProvider::new(
            (language == Some("java"))
                .then(|| self.java.clone())
                .flatten(),
            path.clone(),
        ));

        let editor = cx.new(|cx| {
            let mut state = EditorState::new(window, cx).searchable(true);
            if let Some(language) = language {
                state = state.language(language);
            }
            state.set_readonly(true, cx);
            state.set_tab_size(tab, cx);
            crate::hover::install(&mut state, hover_provider);
            state
        });
        editor.update(cx, |state, cx| state.set_value(text, window, cx));
        if !self.soft_wrap {
            editor.update(cx, |state, cx| state.set_soft_wrap(false, window, cx));
        }
        println!(
            "S1_EDITOR_LANG name={} file={name} virtual=true",
            editor.read(cx).language_name()
        );

        let subscriptions = vec![
            cx.subscribe_in(
                &editor,
                window,
                |pane: &mut Self,
                 editor: &gpui_kit::Entity<EditorState>,
                 event: &InputEvent,
                 window,
                 cx| {
                    if matches!(event, InputEvent::Change) {
                        pane.on_input_change(editor, window, cx);
                    }
                },
            ),
            cx.observe(&editor, |pane: &mut Self, _editor, cx| pane.sync_cursor(cx)),
        ];

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
            state.set_cursor_position(entry.position, window, cx);
            state.cursor_position()
        });
        self.active = Some(index);

        println!(
            "{log_tag} from={}:{} to={}:{}",
            from.position.line + 1,
            from.position.character + 1,
            to.line + 1,
            to.character + 1
        );
        self.sync_cursor(cx);
        cx.notify();
    }

    fn current_entry(&self, cx: &App) -> Option<JumpEntry> {
        let index = self.active?;
        let buffer = self.buffers.get(index)?;
        Some(JumpEntry {
            path: buffer.path.clone(),
            position: buffer.editor.read(cx).cursor_position(),
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

    fn on_input_change(
        &mut self,
        editor: &gpui_kit::Entity<EditorState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self
            .buffers
            .iter()
            .position(|buffer| buffer.editor.entity_id() == editor.entity_id())
        else {
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
        let value = buffer.editor.read(cx).value();

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

    fn reload_buffer(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
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
            state.set_value(body.text, window, cx);
            state.set_readonly(!writable, cx);
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
                            .text_color(cx.theme().muted_foreground)
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

        let text = self.buffers[index].editor.read(cx).text().clone();
        let offset = self.buffers[index].editor.read(cx).cursor();
        let (line, utf16_column) = crate::navigation::core_position(&text, offset);
        let initial = crate::go_to_line::GoToLineInput::prefill(line as usize, utf16_column);

        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial)
                .placeholder(tr("lithe.navigation.goToLinePrompt"))
        });
        let enter = cx.subscribe_in(
            &input,
            window,
            |pane: &mut Self, _input: &Entity<InputState>, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
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
                    crate::go_to_line::GoToLineInput::parse(input.read(cx).value().as_ref())
                        .is_some();
                let confirm_pane = pane.clone();
                let (cancel_id, ok_id) = ((id.clone(), "cancel"), (id.clone(), "ok"));
                Dialog::new(id)
                    .title(tr("lithe.editor.goToLineColumn"))
                    .on_close(dismiss_hosted)
                    .child(Input::new(&input))
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
        let text = prompt.input.read(cx).value().to_string();
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
        let text = self.buffers[index].editor.read(cx).text().clone();
        let target = crate::go_to_line::GoToLineInput::clamped(
            line,
            column,
            has_explicit_column,
            &text.to_string(),
        );
        let origin = JumpEntry {
            path: self.buffers[index].path.clone(),
            position: self.buffers[index].editor.read(cx).cursor_position(),
        };
        let position = editor_position(&text, target.line, target.column);
        let to = self.buffers[index].editor.update(cx, |state, cx| {
            state.set_cursor_position(position, window, cx);
            state.cursor_position()
        });
        self.history.record(origin);

        self.buffers[index].editor.update(cx, |state, cx| {
            state.focus(window, cx);
        });

        println!(
            "S1_EDITOR_GO_TO_LINE jump=true requested={}:{} explicit={} to={}:{}",
            target.line + 1,
            target.column + 1,
            target.has_explicit_column,
            to.line + 1,
            to.character + 1
        );
        self.sync_cursor(cx);
        cx.notify();
        true
    }

    pub fn show_hover_at_caret(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            eprintln!("S1_EDITOR_HOVER menu=false reason=no-active-buffer");
            return;
        };
        let editor = self.buffers[index].editor.clone();
        let java = (language_for_file(&self.buffers[index].name) == Some("java"))
            .then(|| self.java.clone())
            .flatten();
        let provider =
            crate::hover::LitheHoverProvider::new(java, self.buffers[index].path.clone());
        let text = editor.read(cx).text().clone();
        let offset = editor.read(cx).cursor();
        println!("S1_EDITOR_HOVER menu=true offset={offset}");
        let task = provider.hover(&text, offset, window, cx);

        cx.spawn_in(window, async move |pane, cx| {
            let Ok(Some(hover)) = task.await else {
                return;
            };
            let _ = pane.update_in(cx, |_pane, _window, cx| {
                let text = editor.read(cx).text().clone();
                let range = hover
                    .range
                    .as_ref()
                    .map(|range| {
                        text.position_to_offset(&range.start)..text.position_to_offset(&range.end)
                    })
                    .unwrap_or_else(|| text.word_range(offset).unwrap_or(offset..offset));
                editor.update(cx, |state, cx| state.present_hover(range, hover, cx));
            });
        })
        .detach();
    }

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let names = display_names(&self.buffers);

        let mut bar = TabBar::new("editor-tab-bar")
            .with_variant(TabVariant::Underline)
            .bg(cx.theme().tab_bar)
            .h_9()
            .px_2()
            .max_width(rem_px(cx.theme().font_size, TAB_MAX_WIDTH))
            .on_click(cx.listener(|pane, index: &usize, _window, cx| {
                pane.activate(*index, cx);
            }));

        for (index, (buffer, name)) in self.buffers.iter().zip(names).enumerate() {
            bar = bar.child(self.render_tab(index, buffer, name, cx));
        }

        if let Some(active) = self.active {
            bar = bar.selected_index(active);
        }

        bar.into_any_element()
    }

    fn with_tab_menu(
        &self,
        index: usize,
        buffer: &Buffer,
        content: Div,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pane: Entity<Self> = cx.entity();
        let path = buffer.path.clone();
        let name = buffer.name.clone();
        let on_disk = !is_virtual_source_path(&path);

        content
            .id(tab_element_id(&buffer.path))
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
            .context_menu(
                move |mut menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>| {
                    let mut items = 0usize;
                    let mut separators = 0usize;

                    menu = menu.item(
                        PopupMenuItem::new(tr("lithe.files.copyPath"))
                            .icon(idea_icon_mono(&idea::COPY_ICON, cx.theme().is_dark()))
                            .on_click(window.listener_for(&pane, move |pane, _, _window, cx| {
                                pane.copy_path(index, cx);
                            })),
                    );
                    items += 1;

                    menu = menu.item(
                        PopupMenuItem::new(tr("lithe.files.copyRelativePath"))
                            .icon(idea_icon_mono(&idea::COPY_ICON, cx.theme().is_dark()))
                            .on_click(window.listener_for(&pane, move |pane, _, _window, cx| {
                                pane.copy_relative_path(index, cx);
                            })),
                    );
                    items += 1;

                    menu = menu.item(
                        PopupMenuItem::new(tr("lithe.editor.gpui.revealInProjectTree"))
                            .icon(idea_icon_mono(&idea::FILES_ICON, cx.theme().is_dark()))
                            .on_click(window.listener_for(&pane, move |pane, _, _window, cx| {
                                pane.reveal_in_project_tree(index, cx);
                            })),
                    );
                    items += 1;

                    menu = menu.item(
                        PopupMenuItem::new(tr("lithe.files.reveal"))
                            .icon(idea_icon_mono(&idea::FOLDER_ICON, cx.theme().is_dark()))
                            .on_click(window.listener_for(&pane, move |pane, _, window, cx| {
                                pane.reveal_in_explorer(index, window, cx);
                            })),
                    );
                    items += 1;

                    if on_disk {
                        menu = menu.item(
                            PopupMenuItem::new(tr("lithe.files.openInTerminal"))
                                .icon(IconName::Terminal)
                                .on_click(window.listener_for(
                                    &pane,
                                    move |pane, _, window, cx| {
                                        pane.open_in_terminal(index, window, cx);
                                    },
                                )),
                        );
                        items += 1;
                    }

                    if on_disk {
                        menu = menu.item(
                            PopupMenuItem::new(tr("lithe.tabs.reload"))
                                .icon(idea_icon_mono(&idea::REFRESH_ICON, cx.theme().is_dark()))
                                .on_click(window.listener_for(
                                    &pane,
                                    move |pane, _, window, cx| {
                                        pane.request_reload(index, window, cx);
                                    },
                                )),
                        );
                        items += 1;
                    }

                    menu = menu.separator();
                    separators += 1;

                    menu = menu.item(
                        PopupMenuItem::new(tr("lithe.tabs.close"))
                            .icon(idea_icon_mono(&idea::X_ICON, cx.theme().is_dark()))
                            .on_click(window.listener_for(&pane, move |pane, _, window, cx| {
                                pane.request_close(index, window, cx);
                            })),
                    );
                    items += 1;

                    menu = menu.item(PopupMenuItem::new(tr("lithe.tabs.closeOthers")).on_click(
                        window.listener_for(&pane, {
                            let anchor = path.clone();
                            move |pane, _, window, cx| {
                                pane.close_scope(CloseScope::Others(anchor.clone()), window, cx);
                            }
                        }),
                    ));
                    items += 1;

                    menu = menu.item(PopupMenuItem::new(tr("lithe.tabs.closeToRight")).on_click(
                        window.listener_for(&pane, {
                            let anchor = path.clone();
                            move |pane, _, window, cx| {
                                pane.close_scope(CloseScope::ToRight(anchor.clone()), window, cx);
                            }
                        }),
                    ));
                    items += 1;

                    menu = menu.item(PopupMenuItem::new(tr("lithe.tabs.closeAll")).on_click(
                        window.listener_for(&pane, move |pane, _, window, cx| {
                            pane.close_scope(CloseScope::All, window, cx);
                        }),
                    ));
                    items += 1;

                    eprintln!(
                        "S1_TAB_MENU opened=true tab={name} items={items} separators={separators}"
                    );
                    menu
                },
            )
            .into_any_element()
    }

    fn render_tab(
        &self,
        index: usize,
        buffer: &Buffer,
        name: SharedString,
        cx: &mut Context<Self>,
    ) -> Tab {
        let is_active = self.active == Some(index);
        let label_color = if is_active {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };

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
                .bg(cx.theme().primary)
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

        let content = h_flex()
            .items_center()
            .pl_2()
            .pr_6()
            .gap_1p5()
            .min_w_0()
            .child(buffer.icon.render(rems(0.75), cx))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_color(label_color)
                    .child(name),
            )
            .children(dirty_dot)
            .children(close);

        let content = self.with_tab_menu(index, buffer, content.h_full(), cx);

        Tab::new().aria_label(aria_label).child(content)
    }

    fn close_button(path: &Path, index: usize, cx: &mut Context<Self>) -> Button {
        Button::new(tab_close_element_id(path))
            .icon(idea_icon_mono(&idea::X_ICON, cx.theme().is_dark()))
            .ghost()
            .small()
            .tab_stop(false)
            .tooltip(tr("lithe.tabs.close"))
            .accessibility_label(tr("lithe.tabs.close"))
            .on_click(cx.listener(move |pane, _event, window, cx| {
                cx.stop_propagation();
                pane.request_close(index, window, cx);
            }))
    }

    fn render_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(buffer) = self.active.and_then(|index| self.buffers.get(index)) else {
            return Self::render_empty_state(cx);
        };
        let editor = buffer.editor.clone();
        let scroll_editor = editor.clone();
        let path = buffer.path.clone();

        h_flex()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(self.breakpoint_gutter(&path, cx))
            .child(
                div()
                    .id(EDITOR_HOST_ELEMENT_ID)
                    .test_support()
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
                    .child(Editor::new(&editor).h(relative(1.)).bordered(false)),
            )
            .into_any_element()
    }

    fn breakpoint_gutter(&self, path: &Path, cx: &Context<Self>) -> AnyElement {
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
        let top = rem_px(cx.theme().font_size, EDITOR_CONTENT_TOP_PADDING_SPEC)
            + line_height * visible.start as f32
            - scroll_y;
        let marks = self.gutter_breakpoints.get(path);
        let host = self.debug_gutter_host.as_ref();
        let path_for_rows = path.to_path_buf();
        let rows: Vec<AnyElement> = (visible.start..visible.end)
            .map(|row| {
                let line = row + 1;
                let breakpoint = marks
                    .and_then(|marks| marks.iter().find(|mark| mark.line == line))
                    .copied();
                let mut row_element = h_flex()
                    .id(SharedString::from(format!(
                        "bp-{}-{line}",
                        path_for_rows.display()
                    )))
                    .w_full()
                    .h(line_height)
                    .flex_shrink_0()
                    .items_center()
                    .justify_center();
                if let Some(breakpoint) = breakpoint {
                    let active = breakpoint.enabled && !self.breakpoints_muted;
                    row_element = row_element.child(
                        div()
                            .size(rems(BREAKPOINT_MARKER_SIZE_SPEC / 16.))
                            .rounded_full()
                            .bg(if active {
                                cx.theme().danger
                            } else {
                                cx.theme().muted_foreground
                            }),
                    );
                }
                match host {
                    Some(host) => {
                        let toggle = host.toggle.clone();
                        let set_enabled = host.set_enabled.clone();
                        let remove = host.remove.clone();
                        let path_for_click = path_for_rows.clone();
                        row_element
                            .cursor_pointer()
                            .on_click(cx.listener(
                                move |_pane, event: &gpui_kit::ClickEvent, _, cx| {
                                    if event.modifiers().alt {
                                        (remove)(&path_for_click, line, cx);
                                    } else {
                                        (toggle)(&path_for_click, line, cx);
                                    }
                                    let _ = &set_enabled;
                                },
                            ))
                            .into_any_element()
                    }
                    None => row_element.into_any_element(),
                }
            })
            .collect();
        column
            .child(v_flex().w_full().relative().top(-top).children(rows))
            .into_any_element()
    }

    fn render_empty_state(cx: &App) -> AnyElement {
        v_flex()
            .flex_1()
            .min_h_0()
            .context_menu(|menu, _window, _cx| {
                menu.item(PopupMenuItem::new(tr("lithe.ui.noActionsHere")).disabled(true))
            })
            .child(
                Empty::new()
                    .border_color(cx.theme().transparent)
                    .gap_3()
                    .px_6()
                    .py_8()
                    .header(
                        EmptyHeader::new()
                            .max_w_112()
                            .gap_3()
                            .media(
                                EmptyMedia::new().child(
                                    div()
                                        .relative()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .size_12()
                                        .child(
                                            idea_icon_mono(&idea::FILE_ICON, cx.theme().is_dark())
                                                .size_10()
                                                .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(
                                            idea_icon_mono(
                                                &idea::MAGNIFYING_GLASS_ICON,
                                                cx.theme().is_dark(),
                                            )
                                            .absolute()
                                            .right_0()
                                            .bottom_0()
                                            .size_5()
                                            .text_color(cx.theme().muted_foreground),
                                        ),
                                ),
                            )
                            .title(
                                EmptyTitle::new()
                                    .text_sm()
                                    .child(tr("lithe.workbench.emptyEditorTitle")),
                            )
                            .description(
                                EmptyDescription::new()
                                    .text_sm()
                                    .child(tr("lithe.workbench.emptyEditorDescription")),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn tab_element_id(path: &Path) -> String {
    format!("editor-tab:{}", path.display())
}

fn tab_close_element_id(path: &Path) -> String {
    format!("editor-tab-close:{}", path.display())
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .on_action(
                cx.listener(|pane, _: &crate::NavigateToDefinition, window, cx| {
                    pane.navigate_to_definition(window, cx);
                }),
            )
            .on_action(cx.listener(|pane, _: &crate::CloseActiveTab, window, cx| {
                pane.close_active(window, cx);
            }))
            .child(self.render_tab_bar(cx))
            .child(self.render_body(cx))
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
