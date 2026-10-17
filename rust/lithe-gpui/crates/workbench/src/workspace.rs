use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Entity, Hsla, InteractiveElement as _,
    IntoElement, KeyBinding, ParentElement as _, Render, SharedString, Styled as _, Window, div,
    rems,
};
use gpui::{AnyWindowHandle, Global, PathPromptOptions, ScrollHandle};

use lithe_ui::buttons::Button;
use lithe_ui::forms::{Checkbox, InputState};
use lithe_ui::layout::{Dock, DockEvent, DockPanel, DockSide, h_stack, v_stack};
// 窗口根宿主（门 G1）：本文件里的 `open_dialog` / `close_dialog` / `has_active_dialog` /
// `push_notification` 已整片切到 `WindowRootExt`。
use lithe_ui::feedback::Toast;
use lithe_ui::overlays::{ConfirmDialog, Dialog, PromptDialog};
use lithe_ui::primitives::Severity as NoticeSeverity;
use lithe_ui::shell::{WindowRoot, WindowRootExt as _, dismiss_hosted};
use lithe_ui::theme::ActiveTheme as _;

use crate::activity_bar::{ActivityItem, ActivitySide, activity_bar};
use crate::branch_panel::{BranchPanel, open_branch_panel, trigger as branch_trigger};
use crate::command_palette::{
    Category, CommandAction, CommandId, install_actions as install_command_palette, set_shell,
    set_shell_focus,
};
use crate::debug::{DEBUG_DIAGNOSTIC_TAG as DEBUG_TAG, DebugPane, DebugPaneEvent};
use crate::menu_bar::{
    EditorContext, MenuAction, MenuBar, MenuBarMode, MenuRequest, Missing,
    diagnose_run as diagnose_menu_run, set_menu_bar,
};
use crate::project_menu::{
    ProjectEntry, ProjectMenu, render as render_project_menu, set_project_menu,
};
use crate::project_tabs::{ProjectTab, project_tabs};
use crate::right_tool_window::{
    ProbePoint, RightToolWindowView, diagnose as diagnose_right_panel, diagnose_extensions_page,
    resolve_click as resolve_right_click, right_tool_window,
};
use crate::search_index::SearchIndexWiring;
use crate::search_panel::SearchPanel;
use crate::session::{SessionPlan, SessionTracker};
use crate::status_bar::{StatusEntry, status_bar};
use crate::title_bar::title_bar;
use lithe_gpui_editor::{
    BufferWriteHost, DebugGutterHost, EditorPane, JavaServicePhase, SaveBuffer, TabMenuHostActions,
};
use lithe_gpui_explorer::Explorer;
use lithe_gpui_git::{BottomPane, ChangesView};
use lithe_gpui_run::{RunPane, RunPaneEvent, ToolchainPath, ToolchainResolver};
use lithe_gpui_settings::{
    Category as SettingsCategory, GitIdentityHost, IdentityField, IdentityScope, Overrides,
    RecentProjects, resolve_overrides,
};
use lithe_gpui_shared::icons::idea;
use lithe_gpui_shared::workspace_config::{ToolchainPaths, load_local_toolchain};
use lithe_gpui_shared::{tr, tr_args};
use lithe_gpui_terminal::{TerminalPane, TerminalPaneEvent};

// 命令表的图标走本仓 idea 集，而且是「画的时候才问主题」的工厂形式（见 `shared::icons`）。
use lithe_gpui_shared::icons::idea_icon_element;

pub struct ShellStartup {
    compact_menu: bool,
    /// 四态里选了哪一档。`None` = 没给，退回 `compact_menu` 那条旧通道。
    menu_bar_mode: Option<MenuBarMode>,
    right_view: Option<RightToolWindowView>,
    extensions_page: bool,
    left_view: Option<usize>,
}

impl Global for ShellStartup {}

#[allow(clippy::too_many_arguments)]
pub fn set_shell_startup(
    cx: &mut App,
    compact_menu: bool,
    menu_bar_mode: Option<MenuBarMode>,
    right_view: Option<RightToolWindowView>,
    extensions_page: bool,
    left_view: Option<usize>,
) {
    println!(
        "S1_WORKSPACE startup mode={} compact_menu={compact_menu} right_view={:?} extensions_page={extensions_page} left_view={left_view:?}",
        startup_menu_bar_mode_of(menu_bar_mode, compact_menu).id(),
        right_view.as_ref().map(|view| view.id())
    );
    cx.set_global(ShellStartup {
        compact_menu,
        menu_bar_mode,
        right_view,
        extensions_page,
        left_view,
    });
}

/// 新的四态参数优先；没给就退回旧的 `compact_menu: bool`（它只能表达 `Compact` 一档）。
fn startup_menu_bar_mode_of(mode: Option<MenuBarMode>, compact_menu: bool) -> MenuBarMode {
    mode.or(if compact_menu {
        Some(MenuBarMode::Compact)
    } else {
        None
    })
    .unwrap_or(MenuBarMode::DEFAULT)
}

fn startup_menu_bar_mode(cx: &App) -> MenuBarMode {
    cx.try_global::<ShellStartup>()
        .map(|startup| startup_menu_bar_mode_of(startup.menu_bar_mode, startup.compact_menu))
        .unwrap_or(MenuBarMode::DEFAULT)
}

fn startup_views(cx: &App) -> (Option<RightToolWindowView>, bool, Option<usize>) {
    cx.try_global::<ShellStartup>()
        .map(|startup| {
            (
                startup.right_view,
                startup.extensions_page,
                startup.left_view,
            )
        })
        .unwrap_or((None, false, None))
}

pub(crate) fn replace_root_with_workspace(
    target: &Path,
    window: &mut Window,
    cx: &mut App,
) -> Option<Entity<ShellWorkspace>> {
    let mut created: Option<Entity<ShellWorkspace>> = None;
    let new_root = window.replace_root(cx, |window, cx| {
        let workspace =
            cx.new(|cx| ShellWorkspace::new(target.to_path_buf(), false, false, false, window, cx));
        created = Some(workspace.clone());
        WindowRoot::new(workspace, window, cx)
    });
    let _ = new_root;
    created
}

/// File ▸ Close Folder 的落点：把这一窗换回欢迎页。
///
/// 与 [`replace_root_with_workspace`] 正好相反：那里把欢迎页换成工作台，这里把工作台换回
/// 欢迎页。旧工作台的标签、缓冲、dock、终端与语言服务客户端都挂在那个实体上，随根视图一起
/// 释放；逐个手工关会漏掉其中几条清理路径，所以这里只换根。
pub(crate) fn replace_root_with_welcome(window: &mut Window, cx: &mut App) {
    let loaded = lithe_gpui_settings::load_recent_projects();
    window.replace_root(cx, |window, cx| {
        let welcome = cx.new(|cx| {
            crate::welcome::WelcomeView::new(loaded.projects, loaded.path, window, cx)
        });
        WindowRoot::new(welcome, window, cx)
    });
}

/// 把一个工程根从「工程根 → 窗口」表里摘掉。
///
/// 关闭文件夹之后这个窗口已经回到欢迎页。若表里还留着「这个工程属于这个窗口」，下次从欢迎页
/// 打开同一工程只会把欢迎页抬到前面（`route_project_window` 的 `Focused` 分支），工程永远
/// 打不开。关窗那一条不必显式摘：死窗口会被同一条路由的存活裁剪顺手清掉，这里摘掉是为了
/// 不留下一条指向已关窗口的记录。
fn unregister_project_window(root: &Path, window: &Window) {
    let key = project_window_key(root);
    let handle = window.window_handle();
    PROJECT_WINDOWS.with(|slot| {
        slot.borrow_mut()
            .retain(|(path, existing)| *path != key || *existing != handle)
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenDestination {
    NewWindow,
    ThisWindow,
}

/// Opens another IDE window for a project root. The composition root (`app`) owns window bounds and
/// options, so it installs this; the shell only decides *when* a project goes to a new window
/// (真源 `ProjectOpenPlacement.newWindow`). It returns the new window's handle, because the
/// 工程根 → 窗口 归属表（下）必须在这一刻登记，否则同一个工程会被开第二次。
pub type OpenWindowHost =
    std::sync::Arc<dyn Fn(&Path, &mut Window, &mut App) -> Option<AnyWindowHandle> + 'static>;

thread_local! {
    static OPEN_WINDOW_HOST: std::cell::RefCell<Option<OpenWindowHost>> =
        const { std::cell::RefCell::new(None) };
    /// 工程根 → 窗口句柄。真源里同一件事分在两处：macOS 是 `ProjectSessionManager` 的窗口-会话
    /// 归属（`ProjectWindowScope` / `noteWindowBecameKey`），Windows 前端是
    /// `ProjectWindowRegistry`（`identity` / `claim` / `retain_windows`）。语义只有一条：
    /// **一个工程只开一个窗口**，再打开同一个工程就把已有窗口抬到前面。
    static PROJECT_WINDOWS: std::cell::RefCell<Vec<(PathBuf, AnyWindowHandle)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Installed once by the composition root, before the first window opens.
pub fn set_open_window_host(host: OpenWindowHost) {
    OPEN_WINDOW_HOST.with(|slot| *slot.borrow_mut() = Some(host));
}

pub(crate) fn open_window_host() -> Option<OpenWindowHost> {
    OPEN_WINDOW_HOST.with(|slot| slot.borrow().clone())
}

/// 首窗口不是经 `OpenWindowHost` 开的，所以组合根在 `open_window` 之后自己登记一次。
pub fn register_project_window(root: &Path, handle: AnyWindowHandle) {
    let key = project_window_key(root);
    PROJECT_WINDOWS.with(|slot| {
        let mut windows = slot.borrow_mut();
        // 一个窗口同时只显示一个工程：原地换根（OpenDestination::ThisWindow）之后，
        // 这个窗口原来那条记录必须跟着换，否则它会替旧工程占着位置。
        windows.retain(|(path, existing)| *path != key && *existing != handle);
        windows.push((key, handle));
    });
}

/// 工程身份。真源 Windows 前端用 `ProjectWindowRegistry::identity` 做同样的事：同一个目录用不同
/// 写法（相对路径、`\\?` 前缀、大小写）打开时必须落到同一个窗口上。
fn project_window_key(root: &Path) -> PathBuf {
    root.canonicalize().unwrap_or_else(|_| root.to_path_buf())
}

/// 「打开工程 → 新窗口」这一次到底发生了什么。诊断和测试都按它判读。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectWindowRoute {
    /// 新开了一个窗口。
    Opened,
    /// 这个工程已经有一个窗口，抬到了前面，没有再开第二个。
    Focused,
    /// 组合根没有装 `OpenWindowHost`（headless / 单测）。
    NotWired,
    /// 装了宿主但开窗失败。
    Failed,
}

pub(crate) fn route_project_window(
    target: &Path,
    window: &mut Window,
    cx: &mut App,
) -> ProjectWindowRoute {
    // 关掉的窗口不会自己从表里消失（真源 `ProjectWindowRegistry::retain_windows`）。
    let live: Vec<gpui::WindowId> = cx.windows().into_iter().map(|h| h.window_id()).collect();
    PROJECT_WINDOWS.with(|slot| {
        slot.borrow_mut()
            .retain(|(_, handle)| live.contains(&handle.window_id()))
    });

    let key = project_window_key(target);
    let existing = PROJECT_WINDOWS.with(|slot| {
        slot.borrow()
            .iter()
            .find(|(path, _)| *path == key)
            .map(|(_, handle)| *handle)
    });
    if let Some(handle) = existing {
        if handle.update(cx, |_, window, _| window.activate_window()).is_ok() {
            return ProjectWindowRoute::Focused;
        }
        // 还在当前窗口集合里却更新失败：按已关闭处理，别再复用它。
        PROJECT_WINDOWS.with(|slot| slot.borrow_mut().retain(|(path, _)| *path != key));
    }

    let Some(host) = open_window_host() else {
        return ProjectWindowRoute::NotWired;
    };
    match host(target, window, cx) {
        Some(handle) => {
            PROJECT_WINDOWS.with(|slot| slot.borrow_mut().push((key, handle)));
            ProjectWindowRoute::Opened
        }
        None => ProjectWindowRoute::Failed,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProjectOpenDecision {
    pub(crate) destination: OpenDestination,
    pub(crate) remember: bool,
}

pub(crate) fn resolve_project_open_destination(
    ask: bool,
    open_in_new_window: bool,
    explicit: Option<OpenDestination>,
) -> Option<ProjectOpenDecision> {
    if let Some(destination) = explicit {
        return Some(ProjectOpenDecision {
            destination,
            remember: false,
        });
    }
    if !ask {
        return Some(ProjectOpenDecision {
            destination: if open_in_new_window {
                OpenDestination::NewWindow
            } else {
                OpenDestination::ThisWindow
            },
            remember: false,
        });
    }
    None
}

pub fn left_activity_index(selector: &str) -> Option<usize> {
    match selector {
        "files" | "project" => Some(DEFAULT_TOP_ACTIVITY),
        "changes" => Some(CHANGES_ACTIVITY_IX),
        "search" => Some(SEARCH_ACTIVITY_IX),
        _ => None,
    }
}

type GitIdentityDeliver =
    Box<dyn FnOnce(Option<String>, &mut gpui::Window, &mut gpui::App) + 'static>;

trait IntoGitIdentity {
    type Target;
    fn to_git_identity(&self) -> Self::Target;
}

impl IntoGitIdentity for IdentityScope {
    type Target = lithe_gpui_git::IdentityScope;

    fn to_git_identity(&self) -> Self::Target {
        match self {
            Self::Local => lithe_gpui_git::IdentityScope::Local,
            Self::Global => lithe_gpui_git::IdentityScope::Global,
        }
    }
}

impl IntoGitIdentity for IdentityField {
    type Target = lithe_gpui_git::IdentityField;

    fn to_git_identity(&self) -> Self::Target {
        match self {
            Self::Name => lithe_gpui_git::IdentityField::Name,
            Self::Email => lithe_gpui_git::IdentityField::Email,
        }
    }
}

fn reveal_in_file_manager(path: &Path) -> Result<(), String> {
    use std::process::{Command, Stdio};

    let attempts: [(&str, Vec<String>); 3] = [
        ("explorer.exe", vec![format!("/select,{}", path.display())]),
        ("open", vec!["-R".to_string(), path.display().to_string()]),
        (
            "xdg-open",
            vec![path.parent().unwrap_or(path).display().to_string()],
        ),
    ];

    let mut last = String::from("no known file manager");
    for (program, args) in attempts {
        match Command::new(program)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_child) => return Ok(()),
            Err(error) => last = format!("{program}: {error}"),
        }
    }
    Err(last)
}

const SETTINGS_ACTIVITY_IX: usize = 7;

const TOP_ACTIVITY_ITEMS: std::ops::RangeInclusive<usize> = 0..=2;

const CHANGES_ACTIVITY_IX: usize = 1;

const SEARCH_ACTIVITY_IX: usize = 2;

const EXTENSIONS_RAIL_IX: usize = 0;

const DEFAULT_TOP_ACTIVITY: usize = 0;

const SIDEBAR_WIDTH_SPEC: f32 = 320.;

const SIDEBAR_MIN_WIDTH_SPEC: f32 = 30.;

const SIDEBAR_MAX_WIDTH_SPEC: f32 = 520.;

const SIDEBAR_MIN_EDITOR_WIDTH_SPEC: f32 = 400.;

const RIGHT_TOOL_WINDOW_WIDTH_SPEC: f32 = 360.;

const RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC: f32 = 300.;

const RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC: f32 = 760.;

const RIGHT_TOOL_WINDOW_MIN_WORKSPACE_WIDTH_SPEC: f32 = 640.;

const BOTTOM_PANE_MIN_HEIGHT_SPEC: f32 = 260.;

const BOTTOM_PANE_MIN_CENTER_HEIGHT_SPEC: f32 = 220.;

const BOTTOM_PANE_HEIGHT_SPEC: f32 = 260.;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomPaneKind {
    Terminal,
    Git,
    Run,
    Diagnostics,
    Debug,
}

impl BottomPaneKind {
    fn id(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Git => "git",
            Self::Run => "run",
            Self::Diagnostics => "diagnostics",
            Self::Debug => "debug",
        }
    }
}

fn bottom_kind_from_id(id: &str) -> Option<BottomPaneKind> {
    match id {
        "terminal" => Some(BottomPaneKind::Terminal),
        "git" => Some(BottomPaneKind::Git),
        "run" => Some(BottomPaneKind::Run),
        "diagnostics" => Some(BottomPaneKind::Diagnostics),
        "debug" => Some(BottomPaneKind::Debug),
        _ => None,
    }
}

fn bottom_pane_for(index: usize) -> Option<BottomPaneKind> {
    match index {
        3 => Some(BottomPaneKind::Run),
        4 => Some(BottomPaneKind::Terminal),
        5 => Some(BottomPaneKind::Diagnostics),
        6 => Some(BottomPaneKind::Git),
        _ => None,
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ActionFlags {
    pub(crate) terminal_shown: bool,
    pub(crate) maven_shown: bool,
    pub(crate) status_bar_shown: bool,
    pub(crate) dark_theme: bool,
}

/// 命令表的一行。
///
/// 图标一律走本仓的 idea 集（`shared::icons` 的口径）：我们自己没有那张图的几条**留空**
/// （`ToggleStatusBar` 没有状态栏/底部面板的图、`ToggleMenuBar` 用三点代替 —— 见下）。
/// （这一栏早先借的是外部图标集的那几个名字，现按同一份资产目录换成 idea 图。）
pub(crate) fn command_action(id: CommandId, flags: ActionFlags) -> CommandAction {
    use CommandId as C;
    let (category, icon) = match id {
        C::OpenSettings | C::OpenAppearanceSettings => (
            Category::Settings,
            Some(idea_icon_element(&idea::GEAR_ICON)),
        ),
        C::SwitchToLightTheme | C::SwitchToDarkTheme => (
            Category::Settings,
            Some(idea_icon_element(&idea::PALETTE_ICON)),
        ),
        C::ToggleTerminal => (
            Category::View,
            Some(idea_icon_element(&idea::TERMINAL_ICON)),
        ),
        C::ToggleMaven => (Category::View, Some(idea_icon_element(&idea::MAVEN_ICON))),
        // 本仓没有「状态栏 / 底部面板」这张 idea 图 ⇒ 留空，不拿组件库图标顶替。
        C::ToggleStatusBar => (Category::View, None),
        // 主菜单的开合就是那三条横线。
        C::ToggleMenuBar => (
            Category::View,
            Some(idea_icon_element(&idea::DOTS_THREE_ICON)),
        ),
        C::SaveBuffer => (
            Category::View,
            Some(idea_icon_element(&idea::FLOPPY_DISK_ICON)),
        ),
        C::OpenCommandPalette | C::OpenQuickOpen => (
            Category::View,
            Some(idea_icon_element(&idea::MAGNIFYING_GLASS_ICON)),
        ),
        C::OpenSearchEverywhere => (
            Category::View,
            Some(idea_icon_element(&idea::GLOBE_HEMISPHERE_WEST_ICON)),
        ),
        // 替换在 IDEA 里就落在「查找」工具窗里（`expui/toolwindows/find@20x20`）。
        C::OpenReplaceInFiles => (
            Category::View,
            Some(idea_icon_element(&idea::FIND_TOOL_WINDOW_ICON)),
        ),
        C::NavigateToDefinition => (
            Category::View,
            Some(idea_icon_element(&idea::ARROW_RIGHT_ICON)),
        ),
        // 本仓 idea 资产里没有这两张图 ⇒ 留空槽，不拿语义不同的图顶替。
        C::NavigateToImplementation | C::NavigateToTypeDefinition => (Category::View, None),
        C::GoToReferences => (
            Category::View,
            Some(idea_icon_element(&idea::TOOL_WINDOW_FIND_ICON)),
        ),
        // 重命名没有对应图（留空槽）；格式化沿用「整篇排版」的行图。
        C::RenameSymbol => (Category::View, None),
        C::FormatDocument => (Category::View, Some(idea_icon_element(&idea::ROWS_ICON))),
        C::OpenFile => (Category::View, Some(idea_icon_element(&idea::FOLDER_ICON))),
        C::CloseTab
        | C::CloseOtherTabs
        | C::CloseAllTabs
        | C::CloseSavedTabs
        | C::CloseTabsToLeft
        | C::CloseTabsToRight => (Category::View, Some(idea_icon_element(&idea::X_ICON))),
        C::ReopenClosedTab => (Category::View, Some(idea_icon_element(&idea::REFRESH_ICON))),
        C::NewWindow => (Category::View, Some(idea_icon_element(&idea::MONITOR_ICON))),
    };

    let (label, description) = match id {
        C::OpenSettings => (
            tr("lithe.commandPalette.actions.open-settings.label"),
            tr("lithe.settings.tabs.general"),
        ),
        C::OpenAppearanceSettings => (
            tr("lithe.commandPalette.actions.open-settings.label"),
            tr("lithe.settings.tabs.appearance"),
        ),
        C::SwitchToLightTheme => (
            tr("lithe.appearance.gpui.switchThemeLight"),
            tr("lithe.appearance.gpui.switchThemeLightDescription"),
        ),
        C::SwitchToDarkTheme => (
            tr("lithe.appearance.gpui.switchThemeDark"),
            tr("lithe.appearance.gpui.switchThemeDarkDescription"),
        ),
        C::ToggleTerminal => (
            if flags.terminal_shown {
                tr("lithe.commandPalette.actions.toggle-terminal.disableLabel")
            } else {
                tr("lithe.commandPalette.actions.toggle-terminal.enableLabel")
            },
            tr("lithe.workbench.terminal"),
        ),
        C::ToggleMaven => (
            if flags.maven_shown {
                tr("lithe.maven.gpui.toggleToolWindowHide")
            } else {
                tr("lithe.maven.gpui.toggleToolWindowShow")
            },
            tr("lithe.maven.gpui.toggleToolWindowDescription"),
        ),
        C::ToggleStatusBar => (
            if flags.status_bar_shown {
                tr("lithe.appearance.gpui.toggleStatusBarHide")
            } else {
                tr("lithe.appearance.gpui.toggleStatusBarShow")
            },
            tr("lithe.settings.appearance.showStatusBarDescription"),
        ),
        C::ToggleMenuBar => (
            tr("lithe.menu.toggleMenuBar"),
            tr("lithe.settings.appearance.showStatusBarDescription"),
        ),
        C::SaveBuffer => (tr("lithe.menu.save"), tr("lithe.workbench.project")),
        C::OpenCommandPalette => (
            tr("lithe.menu.commandPalette"),
            tr("lithe.commandPalette.placeholder"),
        ),
        C::OpenQuickOpen => (tr("lithe.menu.quickOpen"), tr("lithe.menu.go")),
        C::OpenSearchEverywhere => (tr("lithe.menu.globalSearch"), tr("lithe.menu.view")),
        C::OpenReplaceInFiles => (tr("lithe.search.gpui.replaceInFiles"), tr("lithe.menu.go")),
        C::NavigateToDefinition => (
            tr("lithe.menu.goToDefinition"),
            tr("lithe.navigation.definition"),
        ),
        C::NavigateToImplementation => (
            tr("lithe.menu.goToImplementation"),
            tr("lithe.navigation.implementation"),
        ),
        C::NavigateToTypeDefinition => (
            tr("lithe.menu.goToTypeDefinition"),
            tr("lithe.navigation.typeDefinition"),
        ),
        // 后两个没有 `navigation.*` 文案（那是三个 Go-to 的目标名），
        // 描述复用各自的菜单文案所属的分组，不新造 i18n。
        C::GoToReferences => (
            tr("lithe.menu.goToReferences"),
            tr("lithe.menu.go"),
        ),
        C::RenameSymbol => (tr("lithe.menu.renameSymbol"), tr("lithe.menu.go")),
        C::FormatDocument => (
            tr("lithe.menu.formatDocument"),
            tr("lithe.menu.edit"),
        ),
        C::OpenFile => (tr("lithe.outline.openFile"), tr("lithe.menu.file")),
        C::CloseTab => (tr("lithe.menu.closeTab"), tr("lithe.menu.file")),
        C::CloseOtherTabs => (tr("lithe.menu.closeOtherTabs"), tr("lithe.menu.file")),
        C::CloseAllTabs => (tr("lithe.menu.closeAllTabs"), tr("lithe.menu.file")),
        C::CloseSavedTabs => (tr("lithe.menu.closeSavedTabs"), tr("lithe.menu.file")),
        C::CloseTabsToLeft => (tr("lithe.menu.closeTabsToLeft"), tr("lithe.menu.file")),
        C::CloseTabsToRight => (tr("lithe.menu.closeTabsToRight"), tr("lithe.menu.file")),
        C::ReopenClosedTab => (tr("lithe.menu.reopenClosedTab"), tr("lithe.menu.file")),
        C::NewWindow => (tr("lithe.menu.newWindow"), tr("lithe.menu.file")),
    };

    CommandAction {
        id,
        category,
        label,
        description,
        icon,
    }
}

const COMMAND_ORDER: [CommandId; 7] = [
    CommandId::OpenSettings,
    CommandId::OpenAppearanceSettings,
    CommandId::SwitchToLightTheme,
    CommandId::SwitchToDarkTheme,
    CommandId::ToggleTerminal,
    CommandId::ToggleMaven,
    CommandId::ToggleStatusBar,
];

fn diagnose_run(id: CommandId, state: &str) {
    println!("S1_COMMAND_RUN id={} state={state}", id.id());
}

fn visible_commands(flags: ActionFlags) -> Vec<CommandId> {
    COMMAND_ORDER
        .into_iter()
        .filter(|id| match id {
            CommandId::SwitchToLightTheme => flags.dark_theme,
            CommandId::SwitchToDarkTheme => !flags.dark_theme,
            _ => true,
        })
        .collect()
}

pub struct ShellWorkspace {
    projects: Vec<ProjectTab>,
    project_tabs_scroll: ScrollHandle,
    root: PathBuf,
    pub(crate) search_index: SearchIndexWiring,
    active_project: Option<usize>,
    activity_items: Vec<ActivityItem>,
    right_activity_items: Vec<ActivityItem>,
    top_activity_view: Option<usize>,
    left_sidebar_visible: bool,
    left_pane_visible: bool,
    left_sidebar_width: f32,
    right_tool_window_width: f32,
    bottom_pane_height: f32,
    /// The frame, the dividers and the sizes of the three tool windows. The
    /// shell still chooses *what* each one shows and whether it is open; the
    /// dock owns how they are laid out, and a drag on a divider is the only way
    /// a person sizes them.
    dock: Entity<Dock>,
    _dock_subscription: Option<gpui::Subscription>,
    /// The shell's own handle, for the closures the dock calls back into.
    self_handle: gpui::WeakEntity<ShellWorkspace>,
    right_view: RightToolWindowView,
    right_visible: bool,
    maven_project: Option<crate::maven::MavenProjectView>,
    spring_index: Option<crate::spring::SpringIndexView>,
    right_scan: RightScanState,
    explorer: Entity<Explorer>,
    changes: Entity<ChangesView>,
    search: Entity<crate::search_panel::SearchPanel>,
    quick_open_overlay: Option<gpui::WeakEntity<crate::quick_open::QuickOpen>>,
    search_everywhere_overlay: Option<gpui::WeakEntity<crate::search_everywhere::SearchEverywhere>>,
    replace_in_files_overlay: Option<gpui::WeakEntity<crate::replace_in_files::ReplaceInFiles>>,
    editor: Entity<EditorPane>,
    extensions_page: Option<Entity<crate::extensions_page::ExtensionsPage>>,
    extensions_page_open: bool,
    terminal: Entity<TerminalPane>,
    bottom_git: Entity<BottomPane>,
    run: Entity<RunPane>,
    _run_subscription: Option<gpui::Subscription>,
    debug: Entity<DebugPane>,
    _debug_subscription: Option<gpui::Subscription>,
    bottom_kind: BottomPaneKind,
    bottom_visible: bool,
    _settings_subscription: Option<gpui::Subscription>,
    _terminal_subscription: Option<gpui::Subscription>,
    _editor_subscription: Option<gpui::Subscription>,
    focus: gpui::FocusHandle,
    menu_bar: Entity<MenuBar>,
    project_menu: Entity<ProjectMenu>,
    _project_menu_subscription: Option<gpui::Subscription>,
    branch_panel: Entity<BranchPanel>,
    _branch_panel_subscription: Option<gpui::Subscription>,
    branch_panel_probe: bool,
    recent_projects: RecentProjects,
    recent_projects_path: Option<PathBuf>,
    project_open_do_not_ask: Rc<Cell<bool>>,
    notifications: Entity<lithe_gpui_notify::Store>,
    notification_panel: Entity<crate::notifications::NotificationPanel>,
    spring_panel: Entity<crate::spring_panel::SpringPanel>,
    /// The one-line field the tree's New File / New Folder prompts share. `PromptDialog` borrows an
    /// `Entity<InputState>` that must outlive the dialog's per-frame rebuild, so it lives here and
    /// is reset before each open instead of being created inside the dialog builder.
    new_entry_field: Entity<InputState>,
    _notification_toast_subscription: Option<gpui::Subscription>,
    notification_toast_seq: u64,
    session: SessionTracker,
    session_save_task: Option<gpui::Task<()>>,
    _quit_subscription: Option<gpui::Subscription>,
}

impl ShellWorkspace {
    pub fn new(
        root: PathBuf,
        branch_panel_probe: bool,
        run_probe: bool,
        run_probe_debug: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        lithe_gpui_editor::install_actions(cx);
        lithe_gpui_git::install_actions(cx);
        install_command_palette(cx);
        crate::quick_open::install_actions(cx);
        crate::search_everywhere::install_actions(cx);
        crate::replace_in_files::install_actions(cx);

        // 键位重绑引擎的快照点**必须在这里**：上面这 6 个 `install_actions` 装的绑定
        // （保存 / 跳转定义 / 菜单条开关 / 命令面板 / 快速打开 / 文件内替换）是窗口构造期
        // 才进 keymap 的，晚于 `main.rs` 的启动段。快照早于此处就会漏掉它们，第一次重放会
        // 把它们清掉。第二个窗口再跑一次无害：装的是同一批，快照只做一次。
        // 设置页要知道哪几条命令能绑键（其余灰掉），判定口由引擎注入。
        crate::keymap_engine::register_bindable_resolver();
        crate::keymap_engine::sync(cx);

        register_java_toolchain(&root, cx);

        prepare_workspace_config(root.clone(), cx);

        if let Some(store) = lithe_gpui_settings::try_store(cx) {
            let settings_root = root.clone();
            store.update(cx, |store, cx| {
                store.set_workspace_root(Some(settings_root), cx)
            });
            // 键位改了要重放整张 keymap。`settings` 不能反向依赖 `workbench`，所以订阅挂在
            // 这一侧；引擎自己比指纹，改字号、换主题这类无关改动不会触发重放。
            cx.observe(&store, |_, _, cx| crate::keymap_engine::sync(cx))
                .detach();
        }

        let notifications = cx.new(|_| lithe_gpui_notify::Store::new());

        let notification_toast_subscription = Some(cx.observe_in(
            &notifications,
            window,
            |this, store, window, cx| this.dispatch_notification_toasts(&store, window, cx),
        ));

        let editor = cx.new(|cx| EditorPane::new(Some(notifications.downgrade()), window, cx));
        editor.update(cx, |pane, cx| pane.prepare_java(root.clone(), cx));
        // 本地历史的仓库放在**设置文件所在目录**（平台的应用数据目录）下，不写安装目录：
        // 运行期可写状态写进安装目录会让增量更新失去基线（见 AGENTS.md 的运行时只读边界）。
        if let Some(dir) = lithe_gpui_settings::settings_file_path()
            .and_then(|path| path.parent().map(Path::to_path_buf))
        {
            editor.update(cx, |pane, _| pane.set_local_history_root(dir));
        }
        let editor_handle = editor.downgrade();
        let shell_for_open = cx.entity().downgrade();

        let on_open: Box<dyn Fn(PathBuf, &mut Window, &mut App) + 'static> =
            Box::new(move |path: PathBuf, window: &mut Window, cx: &mut App| {
                let _ = shell_for_open.update(cx, |this, cx| {
                    this.close_extensions_page(None, cx);
                });
                let _ = editor_handle.update(cx, |pane, cx| pane.open(&path, window, cx));
            });

        let explorer_root = root.clone();
        let explorer = cx.new({
            let window = &mut *window;
            move |cx| Explorer::new(explorer_root, on_open, window, cx)
        });
        explorer.update(cx, |this, cx| this.refresh(window, cx));
        explorer.update(cx, |this, _cx| {
            this.set_reveal_in_file_manager_host(Arc::new(|path: &Path, _cx: &mut App| {
                if let Err(error) = reveal_in_file_manager(path) {
                    eprintln!(
                        "S1_EXPLORER reveal_in_file_manager result=failed path={} error={error}",
                        path.display()
                    );
                }
            }));
        });
        // 面板标题行的「隐藏」：收起整块左工具窗（与再点一次活动栏同一效果），
        // 活动栏本身留着 —— 收起是"这次不看"，不是"关掉这个视图"。
        let shell_for_hide = cx.entity().downgrade();
        explorer.update(cx, |this, _cx| {
            this.set_hide_host(Arc::new(move |_window: &mut Window, cx: &mut App| {
                let _ = shell_for_hide.update(cx, |shell, cx| {
                    shell.left_pane_visible = false;
                    cx.notify();
                });
            }));
        });
        // The tree's own rename / delete / new ports: validation, the disk write, the cascade into
        // open buffers and the refresh all live in the shell; the row only reports what to act on.
        {
            let shell_for_rename = cx.entity().downgrade();
            explorer.update(cx, |this, _cx| {
                this.set_rename_host(Arc::new(
                    move |path: &Path, new_name: &str, window: &mut Window, cx: &mut App| {
                        let _ = shell_for_rename.update(cx, |shell, cx| {
                            shell.perform_rename(path, new_name, window, cx);
                        });
                    },
                ));
            });
            let shell_for_delete = cx.entity().downgrade();
            explorer.update(cx, |this, _cx| {
                this.set_delete_host(Arc::new(
                    move |path: &Path, window: &mut Window, cx: &mut App| {
                        let _ = shell_for_delete.update(cx, |shell, cx| {
                            shell.prompt_delete(path, window, cx);
                        });
                    },
                ));
            });
            let shell_for_new = cx.entity().downgrade();
            explorer.update(cx, |this, _cx| {
                this.set_new_host(Arc::new(
                    move |parent: &Path, is_folder: bool, window: &mut Window, cx: &mut App| {
                        let _ = shell_for_new.update(cx, |shell, cx| {
                            shell.prompt_new_entry(parent, is_folder, window, cx);
                        });
                    },
                ));
            });
        }
        let new_entry_field = cx.new(|cx| InputState::new(window, cx));

        let changes_root = root.clone();
        let changes = cx.new({
            let window = &mut *window;
            let editor_handle = editor.downgrade();
            let shell_for_open = cx.entity().downgrade();
            let on_open: Box<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static> =
                Box::new(
                    move |path: PathBuf, line: Option<u32>, window: &mut Window, cx: &mut App| {
                        let _ = shell_for_open.update(cx, |this, cx| {
                            this.close_extensions_page(None, cx);
                        });
                        let _ = editor_handle.update(cx, |pane, cx| match line {
                            Some(line) => {
                                pane.open_at(&path, line, 0, "source-control", window, cx);
                            }
                            None => {
                                pane.open(&path, window, cx);
                            }
                        });
                    },
                );
            move |cx| ChangesView::new(changes_root, on_open, window, cx)
        });

        let terminal = cx.new(|cx| TerminalPane::new(window, cx));

        let search = {
            let editor_handle = editor.downgrade();
            let shell_for_open = cx.entity().downgrade();
            let on_open: Box<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static> =
                Box::new(
                    move |path: PathBuf, line: Option<u32>, window: &mut Window, cx: &mut App| {
                        let _ = shell_for_open.update(cx, |this, cx| {
                            this.close_extensions_page(None, cx);
                        });
                        let _ = editor_handle.update(cx, |pane, cx| match line {
                            Some(line) => {
                                pane.open_at(&path, line, 0, "search", window, cx);
                            }
                            None => pane.open(&path, window, cx),
                        });
                    },
                );
            let search_root = root.clone();
            cx.new({
                let window = &mut *window;
                move |cx| SearchPanel::new(search_root, on_open, window, cx)
            })
        };

        let debug = cx.new(|_cx| DebugPane::new());
        debug.update(cx, |pane, cx| pane.load_stepping_filters(cx));
        let debug_subscription =
            cx.subscribe(
                &debug,
                |this, _pane, event: &DebugPaneEvent, cx| match event {
                    DebugPaneEvent::BreakpointsChanged => {
                        this.push_gutter_breakpoints(cx);
                    }
                    DebugPaneEvent::StartRequested => {
                        this.start_debug_session(cx);
                    }
                    DebugPaneEvent::Hide => {
                        this.bottom_visible = false;
                        this.schedule_session_save(cx);
                        cx.notify();
                    }
                },
            );
        {
            let shell = cx.entity().downgrade();
            let shell_for_write = shell.clone();
            let shell_for_debug_gutter = shell.clone();
            let terminal_handle = terminal.downgrade();
            let workspace_root = root.clone();
            editor.update(cx, |pane, _cx| {
                pane.set_workspace_root(workspace_root);
                pane.set_tab_menu_host_actions(TabMenuHostActions {
                    reveal: Arc::new(|path, _window, _cx| {
                        if let Err(error) = reveal_in_file_manager(path) {
                            eprintln!(
                                "S1_TAB_MENU run=reveal result=failed path={} error={error}",
                                path.display()
                            );
                        }
                    }),
                    reveal_in_tree: {
                        let shell_for_reveal = shell.clone();
                        Arc::new(move |path: &Path, cx: &mut App| {
                            let _ = shell_for_reveal.update(cx, |shell, cx| {
                                shell.reveal_in_project_tree(path, cx);
                            });
                        })
                    },
                    open_terminal: Arc::new(move |path, window, cx| {
                        let directory = path
                            .parent()
                            .map(Path::to_path_buf)
                            .unwrap_or_else(|| path.to_path_buf());
                        let opened = terminal_handle
                            .update(cx, |pane, cx| {
                                pane.new_tab_in(directory.clone(), window, cx)
                            })
                            .is_ok();
                        if opened {
                            let _ = shell.update(cx, |shell, cx| {
                                shell.bottom_kind = BottomPaneKind::Terminal;
                                shell.bottom_visible = true;
                                cx.notify();
                            });
                        }
                        eprintln!(
                            "S1_TAB_MENU run=openInTerminal dir={} opened={opened}",
                            directory.display()
                        );
                    }),
                });
                let write_host: BufferWriteHost = Arc::new(move |path: &Path, cx: &mut App| {
                    let _ =
                        shell_for_write.update(cx, |shell, cx| shell.note_buffer_written(path, cx));
                });
                pane.set_buffer_write_host(write_host);

                let shell_for_toggle = shell_for_debug_gutter.clone();
                let shell_for_enable = shell_for_debug_gutter.clone();
                let shell_for_remove = shell_for_debug_gutter.clone();
                let shell_for_edit = shell_for_debug_gutter;
                pane.set_debug_gutter_host(DebugGutterHost {
                    toggle: Arc::new(move |path: &Path, line: usize, cx: &mut App| {
                        let _ = shell_for_toggle.update(cx, |shell, cx| {
                            shell.toggle_breakpoint(path, line, cx);
                        });
                    }),
                    set_enabled: Arc::new(
                        move |path: &Path, line: usize, enabled: bool, cx: &mut App| {
                            let _ = shell_for_enable.update(cx, |shell, cx| {
                                shell.set_breakpoint_enabled(path, line, enabled, cx);
                            });
                        },
                    ),
                    remove: Arc::new(move |path: &Path, line: usize, cx: &mut App| {
                        let _ = shell_for_remove.update(cx, |shell, cx| {
                            shell.remove_breakpoint(path, line, cx);
                        });
                    }),
                    source_edit: Arc::new(
                        move |path: &Path, before: &str, after: &str, cx: &mut App| {
                            let _ = shell_for_edit.update(cx, |shell, cx| {
                                shell.apply_debug_source_edit(path, before, after, cx);
                            });
                        },
                    ),
                });
            });
        }

        let bottom_git = cx.new(|cx| BottomPane::new(root.clone(), window, cx));

        let run = cx.new(|_| RunPane::new());
        {
            let global = lithe_gpui_settings::try_store(cx)
                .map(|store| Overrides::from_settings(store.read(cx).settings()))
                .unwrap_or_default();
            let local = load_local_toolchain(&root);
            for diagnostic in &local.diagnostics {
                eprintln!("S1_RUN_TOOLCHAIN local_diagnostic {diagnostic}");
            }
            let (resolved, origins) = resolve_overrides(&local.paths, &global);
            println!(
                "S1_RUN_TOOLCHAIN {}",
                ToolchainPaths::diagnostic_suffix(&origins)
            );
            let resolve: ToolchainResolver = Arc::new(move |reference: &str| {
                let environment = lithe_gpui_settings::project::discover(&resolved);
                let effective = match reference {
                    "java" | "project-jdk" => environment.jdk.effective(),
                    "maven" | "project-maven" => environment.maven.effective(),
                    _ => return None,
                };
                match effective {
                    lithe_gpui_settings::project::EffectiveToolchain::Resolved { path, .. } => {
                        let path = PathBuf::from(path);
                        Some(match reference {
                            "java" | "project-jdk" => ToolchainPath::JavaHome(path),
                            _ => ToolchainPath::Program(path),
                        })
                    }
                    _ => None,
                }
            });
            run.update(cx, |pane, cx| {
                pane.set_probe(run_probe, run_probe_debug);
                pane.set_workspace(root.clone(), resolve, cx);
            });
        }
        let run_subscription = cx.subscribe(&run, |this, _pane, event: &RunPaneEvent, cx| {
            if *event == RunPaneEvent::Started {
                this.bottom_kind = BottomPaneKind::Run;
                this.bottom_visible = true;
                this.schedule_session_save(cx);
                if this.run.read(cx).debug_port().is_some() {
                    this.start_debug_session(cx);
                }
                cx.notify();
            }
        });

        {
            let identity_root = root.clone();
            lithe_gpui_settings::set_git_identity_host(Some(GitIdentityHost {
                workspace_root: identity_root.clone(),
                load: {
                    let root_text = identity_root.to_string_lossy().to_string();
                    Box::new(
                        move |scope: &str,
                              deliver: GitIdentityDeliver,
                              async_cx: &mut gpui::AsyncWindowContext| {
                            let scope = IdentityScope::from_id(scope);
                            let root = root_text.clone();
                            async_cx
                            .spawn(async move |async_cx: &mut gpui::AsyncWindowContext| {
                                let result = async_cx
                                    .background_spawn(async move {
                                        lithe_gpui_git::inspect_identity(
                                            &root,
                                            scope.to_git_identity(),
                                        )
                                    })
                                    .await;
                                let _ = async_cx.update(move |window, cx| match result {
                                    Ok(json) => deliver(Some(json), window, cx),
                                    Err(error) => {
                                        eprintln!(
                                            "S1_GIT_IDENTITY run=load scope={} result=failed error={error}",
                                            scope.id()
                                        );
                                        deliver(None, window, cx);
                                    }
                                });
                            })
                            .detach();
                        },
                    )
                },
                save: {
                    let root_text = identity_root.to_string_lossy().to_string();
                    Box::new(
                        move |scope: &str,
                              key: &str,
                              value: Option<String>,
                              deliver: GitIdentityDeliver,
                              async_cx: &mut gpui::AsyncWindowContext| {
                            let scope = IdentityScope::from_id(scope);
                            let field = IdentityField::from_id(key);
                            let root = root_text.clone();
                            async_cx
                            .spawn(async move |async_cx: &mut gpui::AsyncWindowContext| {
                                let result = async_cx
                                    .background_spawn(async move {
                                        lithe_gpui_git::configure_identity(
                                            &root,
                                            scope.to_git_identity(),
                                            field.to_git_identity(),
                                            value.as_deref(),
                                        )
                                    })
                                    .await;
                                let _ = async_cx.update(move |window, cx| match result {
                                    Ok(json) => deliver(Some(json), window, cx),
                                    Err(error) => {
                                        eprintln!(
                                            "S1_GIT_IDENTITY run=save scope={} field={} result=failed error={error}",
                                            scope.id(),
                                            field.id()
                                        );
                                        deliver(None, window, cx);
                                    }
                                });
                            })
                            .detach();
                        },
                    )
                },
            }));
        }

        let changes_for_settings = changes.clone();
        let settings_subscription = lithe_gpui_settings::try_store(cx).map(|store| {
            let initial = store.read(cx).settings().clone();
            println!(
                "S1_SETTINGS wiring=workbench tab_size={} terminal_default_shell_id={:?} terminal_font_size={} confirm_before_discard={} auto_completion={}",
                initial.tab_size,
                initial.terminal_default_shell_id,
                initial.terminal_font_size,
                initial.confirm_before_discard,
                initial.auto_completion
            );
            editor.update(cx, |pane, cx| {
                pane.set_tab_size(initial.tab_size as usize, cx);
                pane.set_auto_completion(initial.auto_completion, cx);
                pane.set_auto_save(initial.auto_save, cx);
                pane.set_soft_wrap(initial.word_wrap, window, cx);
                pane.set_minimap(initial.minimap, cx);
                pane.set_line_numbers(initial.line_numbers, cx);
                pane.set_render_whitespace(initial.render_whitespace, cx);
                pane.set_indent_guides(initial.indent_guides, cx);
                pane.set_rainbow_brackets(initial.rainbow_brackets, cx);
                pane.set_sticky_scroll(initial.sticky_scroll, cx);
            });
            terminal.update(cx, |pane, cx| {
                pane.set_default_shell(&initial.terminal_default_shell_id, cx);
                pane.set_font_size(terminal_font_size_override(initial.terminal_font_size), cx);
            });
            changes_for_settings.update(cx, |view, cx| {
                view.set_confirm_before_discard(initial.confirm_before_discard, cx);
            });
            cx.observe_in(&store, window, |this, store, window, cx| {
                let settings = store.read(cx).settings().clone();
                this.editor.update(cx, |pane, cx| {
                    pane.set_tab_size(settings.tab_size as usize, cx);
                    pane.set_auto_completion(settings.auto_completion, cx);
                    pane.set_auto_save(settings.auto_save, cx);
                    pane.set_soft_wrap(settings.word_wrap, window, cx);
                    pane.set_minimap(settings.minimap, cx);
                    pane.set_line_numbers(settings.line_numbers, cx);
                    pane.set_render_whitespace(settings.render_whitespace, cx);
                    pane.set_indent_guides(settings.indent_guides, cx);
                    pane.set_rainbow_brackets(settings.rainbow_brackets, cx);
                    pane.set_sticky_scroll(settings.sticky_scroll, cx);
                });
                this.terminal.update(cx, |pane, cx| {
                    pane.set_default_shell(&settings.terminal_default_shell_id, cx);
                    pane.set_font_size(terminal_font_size_override(settings.terminal_font_size), cx);
                });
                this.changes.update(cx, |view, cx| {
                    view.set_confirm_before_discard(settings.confirm_before_discard, cx);
                });
                // 菜单栏布局是「立刻生效」的那一类：4 档只是摆法，不重启任何服务。
                // 认不出的值在这里回落默认（写入侧的 Settings::normalize 也会兜一次）。
                this.menu_bar.update(cx, |bar, cx| {
                    bar.set_mode(
                        MenuBarMode::from_id(&settings.menu_bar_mode)
                            .unwrap_or(MenuBarMode::DEFAULT),
                        cx,
                    )
                });
                let current_root = this.root.clone();
                register_java_toolchain(&current_root, cx);
                cx.notify();
            })
        });

        let terminal_subscription = Some(cx.subscribe_in(
            &terminal,
            window,
            |this: &mut Self, _pane, event: &TerminalPaneEvent, _window, cx| {
                if matches!(event, TerminalPaneEvent::LastTabClosed) {
                    this.bottom_visible = false;
                    cx.notify();
                }
            },
        ));

        let editor_subscription = Some(cx.observe(&editor, |shell: &mut Self, _, cx| {
            shell.schedule_session_save(cx);
            cx.notify();
        }));

        let project_name: SharedString = root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| root.display().to_string())
            .into();

        let focus = cx.focus_handle();
        let menu_bar = MenuBar::new(startup_menu_bar_mode(cx), focus.clone(), cx);
        set_menu_bar(menu_bar.downgrade());
        // 菜单一变（开 / 关 / 换模式 / 选中一行）外壳就重画。走 `observe` 而不是让 MenuBar
        // 攥一个 `WeakEntity<ShellWorkspace>` 反戳回来：菜单的关闭回调跑在外壳自己的更新栈
        // 里，反戳就是第二次可变借用；`observe` 的回调由 gpui 保证在 MenuBar 的 update 结束
        // 之后才派发。与下面 `project_menu` / `branch_panel` 同一写法。
        cx.observe(&menu_bar, |_, _, cx| cx.notify()).detach();

        let loaded_recent = lithe_gpui_settings::load_recent_projects();
        loaded_recent.report();
        let recent_projects = loaded_recent.projects.clone();
        let recent_projects_path = loaded_recent.path.clone();
        let project_menu = ProjectMenu::new(project_name.as_ref(), recent_projects.len(), cx);
        set_project_menu(project_menu.downgrade());
        let project_menu_subscription = Some(cx.observe(&project_menu, |_, _, cx| cx.notify()));

        let branch_panel = BranchPanel::new(root.clone(), window, cx);
        let branch_panel_subscription = Some(cx.observe(&branch_panel, |_, _, cx| cx.notify()));

        let notification_panel =
            cx.new(|cx| crate::notifications::NotificationPanel::new(&notifications, window, cx));

        let spring_panel = cx.new(|cx| crate::spring_panel::SpringPanel::new(window, cx));
        cx.observe(&spring_panel, |_, _, cx| cx.notify()).detach();

        let self_handle = cx.weak_entity();

        // 语言服务阶段变化 → 弹一条通知（状态栏读的是同一个阶段，不需要回调）。
        // 以前只有 S1_JAVA_* 日志在文件里，界面上分不清「没启动」和「正在启动」。
        {
            let shell = self_handle.clone();
            editor.update(cx, |pane, _| {
                pane.set_java_phase_host(Rc::new(move |phase, cx| {
                    let Some(shell) = shell.upgrade() else {
                        return;
                    };
                    let phase = phase.clone();
                    let _ = shell.update(cx, |shell, cx| shell.on_java_phase(phase, cx));
                }));
            });
        }

        // The dock wraps the middle column: the editor at its center, a tool
        // window on each side. Its two closures read the shell back, so a panel
        // always draws the view the shell has chosen for it.
        let dock = {
            let center = {
                let shell = self_handle.clone();
                move |window: &mut Window, cx: &mut App| match shell.upgrade() {
                    Some(shell) => shell.read(cx).center_content(window, cx),
                    None => div().into_any_element(),
                }
            };
            let content = {
                let shell = self_handle.clone();
                move |id: &SharedString, window: &mut Window, cx: &mut App| match shell.upgrade() {
                    Some(shell) => shell.read(cx).panel_content(id.as_ref(), window, cx),
                    None => div().into_any_element(),
                }
            };
            cx.new(|cx| dock_with_panels(center, content, cx))
        };

        let mut workspace = Self {
            projects: vec![ProjectTab::new(project_name.clone())],
            project_tabs_scroll: ScrollHandle::new(),
            root: root.clone(),
            search_index: SearchIndexWiring::new(root.clone()),
            active_project: Some(0),
            activity_items: activity_items(),
            right_activity_items: right_activity_items(&notifications, cx),
            top_activity_view: Some(DEFAULT_TOP_ACTIVITY),
            left_sidebar_visible: true,
            left_pane_visible: true,
            left_sidebar_width: SIDEBAR_WIDTH_SPEC,
            right_tool_window_width: RIGHT_TOOL_WINDOW_WIDTH_SPEC,
            bottom_pane_height: BOTTOM_PANE_HEIGHT_SPEC,
            dock,
            _dock_subscription: None,
            self_handle,
            right_view: RightToolWindowView::Maven,
            right_visible: false,
            maven_project: None,
            spring_index: None,
            right_scan: RightScanState::default(),
            explorer,
            changes,
            search,
            quick_open_overlay: None,
            search_everywhere_overlay: None,
            replace_in_files_overlay: None,
            editor,
            extensions_page: None,
            extensions_page_open: false,
            terminal,
            bottom_git,
            run,
            _run_subscription: Some(run_subscription),
            debug,
            _debug_subscription: Some(debug_subscription),
            bottom_kind: BottomPaneKind::Terminal,
            bottom_visible: false,
            _settings_subscription: settings_subscription,
            _terminal_subscription: terminal_subscription,
            _editor_subscription: editor_subscription,
            focus,
            menu_bar,
            project_menu,
            _project_menu_subscription: project_menu_subscription,
            branch_panel,
            _branch_panel_subscription: branch_panel_subscription,
            branch_panel_probe,
            recent_projects,
            recent_projects_path,
            project_open_do_not_ask: Rc::new(Cell::new(false)),
            notifications,
            notification_panel,
            spring_panel,
            new_entry_field,
            notification_toast_seq: 0,
            _notification_toast_subscription: notification_toast_subscription,
            session: SessionTracker::new(&root),
            session_save_task: None,
            _quit_subscription: None,
        };
        let dock_subscription = cx.subscribe(
            &workspace.dock,
            |shell: &mut ShellWorkspace, _dock, event: &DockEvent, cx| {
                shell.on_dock_event(event, cx)
            },
        );
        workspace._dock_subscription = Some(dock_subscription);
        println!("S1_WORKSPACE root={}", root.display());
        workspace.record_opened_project(&root, None, cx);
        workspace.search_index.schedule_warm(cx);
        diagnose_right_panel(workspace.right_view, workspace.right_visible, None);
        let (startup_right_view, startup_extensions, startup_left_view) = startup_views(cx);
        if let Some(view) = startup_right_view {
            workspace.right_view = view;
            workspace.right_visible = true;
            workspace.schedule_right_view_scan(view, true, cx);
            diagnose_right_panel(view, true, None);
        }
        if startup_extensions {
            workspace.open_extensions_page(None, window, cx);
        }
        if let Some(index) = startup_left_view {
            workspace.select_top_activity(index, false, cx);
            println!("S1_LEFT_VIEW index={index}");
        }
        let shell_handle = cx.entity().downgrade();
        set_shell(shell_handle.clone());
        workspace
            .project_menu
            .clone()
            .update(cx, |menu, _cx| menu.set_shell(shell_handle));
        set_shell_focus(workspace.focus.clone());
        window.focus(&workspace.focus, cx);

        workspace.restore_session(window, cx);

        let quit_shell = cx.entity().downgrade();
        let _quit_subscription = cx.on_app_quit(move |_shell, cx| {
            let _ = quit_shell.update(cx, |shell, cx| shell.flush_session(true, cx));
            async {}
        });
        workspace._quit_subscription = Some(_quit_subscription);

        workspace
    }

    fn restore_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let outcome = crate::session::restore_into(&self.root, &self.editor, window, cx);
        let session = outcome.session;

        if let Some(visible) = session.left_sidebar_visible {
            self.left_sidebar_visible = visible;
        }
        // Widths come back through the dock: it clamps them against its own
        // limits, and the shell stores what it settled on. The extent here is
        // the window's, a little looser than the dock's own box (the window less
        // the two activity rails) — the approximation the right tool window has
        // always used, now for all three sides.
        let rem = window.rem_size();
        let viewport = window.viewport_size();
        for (side, stored) in [
            (DockSide::Left, session.left_pane_width),
            (DockSide::Right, session.right_pane_width),
            (DockSide::Bottom, session.bottom_pane_height),
        ] {
            let Some(stored) = stored else { continue };
            let extent = match side {
                DockSide::Bottom => rems(f32::from(viewport.height) / 16.),
                _ => rems(f32::from(viewport.width) / 16.),
            };
            let limits = self.dock.read(cx).side_limits(side);
            let resolved = limits.clamp_in(rems(stored / 16.), extent, rems(f32::MAX), rem);
            self.dock
                .update(cx, |dock, cx| dock.set_size(side, resolved, cx));
            self.set_pane_span(side, resolved.0 * 16.);
        }
        println!(
            "S1_SESSION_SIZES left={} right={} bottom={}",
            self.left_sidebar_width, self.right_tool_window_width, self.bottom_pane_height
        );
        if let Some(index) = session.top_activity_view {
            if index < self.activity_items.len() {
                self.select_top_activity(index, false, cx);
            }
        }
        if let Some(kind) = session.bottom_kind.as_deref().and_then(bottom_kind_from_id) {
            self.bottom_kind = kind;
        }
        if let Some(visible) = session.bottom_visible {
            self.bottom_visible = visible;
        }
        if let Some(view) = session
            .right_view
            .as_deref()
            .and_then(RightToolWindowView::from_id)
        {
            self.right_view = view;
        }
        if let Some(visible) = session.right_visible {
            self.right_visible = visible;
        }

        println!(
            "S1_WORKSPACE session_applied left_sidebar_visible={} top_activity_view={:?} bottom_kind={} bottom_visible={} right_view={} right_visible={}",
            self.left_sidebar_visible,
            self.top_activity_view,
            self.bottom_kind.id(),
            self.bottom_visible,
            self.right_view.id(),
            self.right_visible
        );
        if self.right_visible {
            self.schedule_right_view_scan(self.right_view, true, cx);
        }
        cx.notify();
    }

    pub fn session_probe(
        &mut self,
        paths: &[PathBuf],
        active: Option<usize>,
        hide_sidebar: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for path in paths {
            let _ = self
                .editor
                .update(cx, |pane, cx| pane.open(path, window, cx));
        }
        if let Some(index) = active {
            if let Some(path) = paths.get(index) {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.open(path, window, cx));
            }
        }
        if hide_sidebar {
            self.left_sidebar_visible = false;
        }
        println!(
            "S1_SESSION_PROBE opened={} active={:?} hide_sidebar={hide_sidebar}",
            paths.len(),
            active
        );
        self.flush_session(true, cx);
        cx.notify();
    }

    pub fn session_assert(&self, cx: &App) {
        let files = self.editor.read(cx).open_disk_files();
        let active = self.editor.read(cx).active_disk_file();
        let active_index = active
            .as_ref()
            .and_then(|active| files.iter().position(|path| path == active));
        let headline = format!(
            "S1_SESSION_ASSERT root={} open={} active_index={:?} active={:?} left_sidebar_visible={} bottom_kind={} bottom_visible={} right_view={} right_visible={} top_activity_view={:?}",
            self.root.display(),
            files.len(),
            active_index,
            active.as_ref().map(|path| path.display().to_string()),
            self.left_sidebar_visible,
            self.bottom_kind.id(),
            self.bottom_visible,
            self.right_view.id(),
            self.right_visible,
            self.top_activity_view
        );
        println!("{headline}");
        let mut report = format!("{headline}\n");
        for path in &files {
            let relative =
                lithe_gpui_shared::workspace_config::workspace_relative(&self.root, path)
                    .unwrap_or_else(|| path.display().to_string());
            println!("S1_SESSION_ASSERT file={relative}");
            report.push_str(&format!("S1_SESSION_ASSERT file={relative}\n"));
        }
        if let Some(target) = std::env::var_os("S1_SESSION_ASSERT_FILE") {
            let target = PathBuf::from(target);
            if let Err(error) = std::fs::write(&target, report) {
                eprintln!(
                    "S1_SESSION_ASSERT write_failed path={} error={error}",
                    target.display()
                );
            }
        }
    }

    pub(crate) fn schedule_session_save(&mut self, cx: &mut Context<Self>) {
        let ui = self.session_ui_state();
        let plan = {
            let editor = self.editor.clone();
            self.session.plan(&editor, ui, cx)
        };
        match plan {
            SessionPlan::Unchanged => {}
            SessionPlan::ReadOnly => eprintln!(
                "S1_WORKSPACE_CONFIG session_save_skipped root={} reason=document_version_newer",
                self.root.display()
            ),
            SessionPlan::Debounced(armed) => {
                self.session_save_task = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(
                            crate::session::SESSION_SAVE_DEBOUNCE_MS,
                        ))
                        .await;
                    let _ = this.update(cx, |shell, _cx| {
                        shell.session.flush_if_current(armed);
                    });
                }));
            }
        }
    }

    pub(crate) fn flush_session(&mut self, force: bool, cx: &mut Context<Self>) {
        let ui = self.session_ui_state();
        let editor = self.editor.clone();
        let plan = self.session.plan(&editor, ui, cx);
        if matches!(plan, SessionPlan::ReadOnly) {
            eprintln!(
                "S1_WORKSPACE_CONFIG session_save_skipped root={} reason=document_version_newer",
                self.root.display()
            );
            return;
        }
        let wrote = self.session.flush(force);
        println!(
            "S1_WORKSPACE_CONFIG session_flush root={} force={force} wrote={wrote}",
            self.root.display()
        );
    }

    pub(crate) fn note_buffer_written(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.search_index.schedule_update(path, cx);
    }

    fn debug_breakpoint_key(&self, path: &Path) -> String {
        crate::debug::debug_key(&self.root, path)
    }

    pub(crate) fn push_gutter_breakpoints(&mut self, cx: &mut Context<Self>) {
        let (snapshot, muted) = {
            let pane = self.debug.read(cx);
            (pane.gutter_snapshot(&self.root), pane.breakpoints().muted())
        };
        self.editor.update(cx, |pane, _cx| {
            pane.set_gutter_breakpoints(snapshot, muted);
        });
        cx.notify();
    }

    pub(crate) fn start_debug_session(&mut self, cx: &mut Context<Self>) {
        let service = self.editor.read(cx).java_service();
        let Some(service) = service else {
            eprintln!("{DEBUG_TAG} start status=blocked reason=no-java-service");
            return;
        };
        let Some(lsp_session) = service.lsp_session_id() else {
            eprintln!("{DEBUG_TAG} start status=blocked reason=no-lsp-session");
            return;
        };
        let root = self.root.to_string_lossy().to_string();
        let session_id = "java-debug".to_string();
        let operation_id = format!("{session_id}-port");
        match crate::debug_transport::start_java_session(
            &session_id,
            &operation_id,
            &lsp_session,
            &root,
            std::time::Duration::from_secs(30),
        ) {
            Ok(mut transport) => {
                let seeded = self
                    .debug
                    .update(cx, |pane, _| pane.seed_probe_breakpoints());
                let breakpoints_sent = self.debug.read(cx).resend_breakpoints(&session_id);
                println!("{DEBUG_TAG} setBreakpoints seeded={seeded}");
                println!(
                    "{DEBUG_TAG} setBreakpoints_before_attach session={session_id} count={breakpoints_sent}"
                );
                let jdwp_port = self.run.read(cx).debug_port();
                match jdwp_port {
                    Some(port) => {
                        let project_name = std::env::var("LITHE_DEBUG_PROJECT_NAME")
                            .ok()
                            .map(|value| value.trim().to_string())
                            .filter(|value| !value.is_empty());
                        match crate::debug_transport::java_attach_configuration(
                            "Java",
                            port,
                            project_name.as_deref(),
                        ) {
                            Ok(configuration) => {
                                match transport.launch_with_configuration(configuration) {
                                    Ok(_) => println!(
                                        "{DEBUG_TAG} launch status=sent session={session_id} jdwp={port}"
                                    ),
                                    Err(error) => eprintln!(
                                        "{DEBUG_TAG} launch status=failed jdwp={port} error={error}"
                                    ),
                                }
                            }
                            Err(error) => {
                                eprintln!("{DEBUG_TAG} launch status=invalid error={error}");
                            }
                        }
                    }
                    None => eprintln!(
                        "{DEBUG_TAG} launch status=pending-jdwp-port session={session_id}"
                    ),
                }
                let handle = crate::debug_session::DebugSessionHandle::start(
                    transport,
                    std::time::Duration::from_millis(20),
                );
                self.bottom_kind = BottomPaneKind::Debug;
                self.bottom_visible = true;
                self.debug.update(cx, |pane, cx| {
                    pane.attach_session(handle);
                    cx.notify();
                });
                println!("{DEBUG_TAG} start status=ok session={session_id} lsp={lsp_session}");
            }
            Err(error) => {
                eprintln!("{DEBUG_TAG} start status=failed error={error}");
            }
        }
    }

    pub(crate) fn toggle_breakpoint(&mut self, path: &Path, line: usize, cx: &mut Context<Self>) {
        let key = self.debug_breakpoint_key(path);
        let Ok(line) = i64::try_from(line) else {
            return;
        };
        self.debug
            .update(cx, |pane, cx| pane.toggle_breakpoint_at(&key, line, cx));
    }

    pub(crate) fn set_breakpoint_enabled(
        &mut self,
        path: &Path,
        line: usize,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        let key = self.debug_breakpoint_key(path);
        let Ok(line) = i64::try_from(line) else {
            return;
        };
        self.debug.update(cx, |pane, cx| {
            pane.set_breakpoint_enabled(&key, line, enabled, cx)
        });
    }

    pub(crate) fn remove_breakpoint(&mut self, path: &Path, line: usize, cx: &mut Context<Self>) {
        let key = self.debug_breakpoint_key(path);
        let Ok(line) = i64::try_from(line) else {
            return;
        };
        self.debug
            .update(cx, |pane, cx| pane.remove_breakpoint(&key, line, cx));
    }

    pub(crate) fn toggle_breakpoint_at_caret(&mut self, cx: &mut Context<Self>) {
        let target = {
            let pane = self.editor.read(cx);
            pane.active_disk_file().map(|path| {
                let position = pane.cursor_position(cx);
                (path, position.line)
            })
        };
        let Some((path, line)) = target else {
            eprintln!("{DEBUG_TAG} toggleAtCaret status=skipped reason=no-file");
            return;
        };
        eprintln!(
            "{DEBUG_TAG} toggleAtCaret path={} line={line}",
            path.display()
        );
        self.toggle_breakpoint(&path, line, cx);
    }

    pub(crate) fn apply_debug_source_edit(
        &mut self,
        path: &Path,
        before: &str,
        after: &str,
        cx: &mut Context<Self>,
    ) {
        let key = self.debug_breakpoint_key(path);
        self.debug.update(cx, |pane, cx| {
            pane.apply_source_edit(&key, before, after, cx)
        });
    }

    #[allow(dead_code)]
    pub(crate) fn clear_breakpoints(&mut self, cx: &mut Context<Self>) {
        self.debug.update(cx, |pane, cx| pane.clear_breakpoints(cx));
    }

    fn session_ui_state(&self) -> lithe_gpui_shared::workspace_config::SessionUiState {
        lithe_gpui_shared::workspace_config::SessionUiState {
            left_sidebar_visible: Some(self.left_sidebar_visible),
            top_activity_view: self.top_activity_view,
            bottom_kind: Some(self.bottom_kind.id().to_string()),
            bottom_visible: Some(self.bottom_visible),
            right_view: Some(self.right_view.id().to_string()),
            right_visible: Some(self.right_visible),
            left_pane_width: Some(self.left_sidebar_width),
            right_pane_width: Some(self.right_tool_window_width),
            bottom_pane_height: Some(self.bottom_pane_height),
        }
    }

    fn set_pane_span(&mut self, side: DockSide, span: f32) {
        match side {
            DockSide::Left => self.left_sidebar_width = span,
            DockSide::Right => self.right_tool_window_width = span,
            DockSide::Bottom => self.bottom_pane_height = span,
            DockSide::Top => {}
        }
    }

    /// A person dragged a divider: the dock owns the size from here, and the
    /// shell records it so the session saves it. The write is debounced, so a
    /// drag still touches no disk until the hand stops.
    fn on_dock_event(&mut self, event: &DockEvent, cx: &mut Context<Self>) {
        if let DockEvent::Resized { side, size } = event {
            self.set_pane_span(*side, size.0 * 16.);
            eprintln!(
                "S1_PANE_RESIZE pane={} span={}",
                side_id(*side),
                size.0 * 16.
            );
            self.schedule_session_save(cx);
            cx.notify();
        }
    }

    /// The middle column the dock wraps: the extensions page while it is open,
    /// the editor otherwise.
    fn center_content(&self, window: &mut Window, cx: &App) -> AnyElement {
        if self.extensions_page_open {
            if let Some(page) = &self.extensions_page {
                return page.clone().into_any_element();
            }
        }
        let _ = window;
        editor_island(self.editor.clone(), cx).into_any_element()
    }

    /// One dock panel's body. The shell chooses what each tool window shows;
    /// the dock owns the frame around it, the divider and the size.
    fn panel_content(&self, id: &str, _window: &mut Window, cx: &App) -> AnyElement {
        match id {
            PANEL_LEFT => {
                if self.top_activity_view == Some(CHANGES_ACTIVITY_IX) {
                    self.changes.clone().into_any_element()
                } else if self.top_activity_view == Some(SEARCH_ACTIVITY_IX) {
                    self.search.clone().into_any_element()
                } else {
                    self.explorer.clone().into_any_element()
                }
            }
            PANEL_RIGHT => {
                let on_close = {
                    let handle = self.self_handle.clone();
                    move |event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                        let _ = handle.update(cx, |this, cx| {
                            this.right_visible = false;
                            diagnose_right_panel(
                                this.right_view,
                                false,
                                event.mouse_position().map(ProbePoint::from),
                            );
                            this.schedule_session_save(cx);
                            cx.notify();
                        });
                    }
                };
                right_tool_window(
                    self.right_view,
                    self.maven_project.as_ref(),
                    self.spring_index.as_ref(),
                    &self.notification_panel,
                    &self.spring_panel,
                    on_close,
                    cx,
                )
            }
            PANEL_BOTTOM => match self.bottom_kind {
                BottomPaneKind::Terminal => self.terminal.clone().into_any_element(),
                BottomPaneKind::Git => self.bottom_git.clone().into_any_element(),
                BottomPaneKind::Diagnostics => {
                    self.editor.read(cx).problems_pane().into_any_element()
                }
                BottomPaneKind::Run => self.run.clone().into_any_element(),
                BottomPaneKind::Debug => self.debug.clone().into_any_element(),
            },
            _ => div().into_any_element(),
        }
    }

    /// Brings the shell's tool-window visibility and the dock's together: the
    /// shell decides, the dock draws. Called once a frame, so no path that flips
    /// a flag has to remember to tell the dock.
    fn sync_dock(&mut self, cx: &mut Context<Self>) {
        let git_self_hidden =
            self.bottom_kind == BottomPaneKind::Git && !self.bottom_git.read(cx).visible();
        let wanted = wanted_panels(
            self.left_sidebar_visible,
            self.left_pane_visible,
            self.right_visible,
            self.bottom_visible,
            git_self_hidden,
        );
        self.dock.update(cx, |dock, cx| {
            for (id, visible) in wanted {
                let hidden = dock.is_hidden(id);
                if visible && hidden {
                    dock.show(id, cx);
                } else if !visible && !hidden {
                    dock.hide(id, cx);
                }
            }
        });
    }

    fn reveal_in_project_tree(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.top_activity_view = Some(DEFAULT_TOP_ACTIVITY);
        self.left_pane_visible = true;
        self.left_sidebar_visible = true;
        let explorer = self.explorer.clone();
        let _ = explorer.update(cx, |explorer, cx| explorer.reveal(path, cx));
        cx.notify();
    }

    /// 选中左侧视图。`by_click` 为真表示这是一次活动栏点击 —— 再点同一项就
    /// 收起面板（IntelliJ 的手感）；恢复会话、应用启动视图时传假，因为那恢复的
    /// 是"选中的是哪一项"，不是一次点击。两者混用会让第二次启动起左侧面板一直
    /// 收着：初始 `top_activity_view` 与恢复出来的值都是同一项，切换分支一进就把
    /// `left_pane_visible` 翻成了假。
    fn select_top_activity(&mut self, index: usize, by_click: bool, cx: &mut Context<Self>) {
        if by_click && self.top_activity_view == Some(index) {
            self.left_pane_visible = !self.left_pane_visible;
        } else {
            self.top_activity_view = Some(index);
            self.left_pane_visible = true;
        }
        if index == CHANGES_ACTIVITY_IX {
            let changes = self.changes.clone();
            let _ = changes.update(cx, |view, cx| view.activate(cx));
        }
    }

    fn project_entries(&self) -> Vec<ProjectEntry> {
        let path: SharedString = self.root.display().to_string().into();
        self.projects
            .iter()
            .enumerate()
            .map(|(index, tab)| ProjectEntry {
                name: tab.name().clone(),
                path: path.clone(),
                active: self.active_project == Some(index),
            })
            .collect()
    }

    pub(crate) fn command_actions(&self, cx: &App) -> Vec<CommandAction> {
        let flags = self.action_flags(cx);
        visible_commands(flags)
            .into_iter()
            .map(|id| command_action(id, flags))
            .collect()
    }

    pub(crate) fn action_catalog(&self, cx: &App) -> Vec<CommandAction> {
        let flags = self.action_flags(cx);
        CommandId::ALL
            .into_iter()
            .map(|id| command_action(id, flags))
            .collect()
    }

    fn action_flags(&self, cx: &App) -> ActionFlags {
        let (status_bar_shown, dark_theme) = lithe_gpui_settings::try_store(cx)
            .map(|store| {
                let store = store.read(cx);
                let settings = store.settings();
                (
                    settings.show_status_bar,
                    !store.applied_theme().to_lowercase().contains("light"),
                )
            })
            .unwrap_or((true, true));
        ActionFlags {
            terminal_shown: self.bottom_visible && self.bottom_kind == BottomPaneKind::Terminal,
            maven_shown: self.right_visible && self.right_view == RightToolWindowView::Maven,
            status_bar_shown,
            dark_theme,
        }
    }

    fn toggle_left_sidebar(&mut self) {
        self.left_sidebar_visible = !self.left_sidebar_visible;
        diagnose_menu_run(
            MenuAction::ToggleActivitySidebar,
            if self.left_sidebar_visible {
                "visible"
            } else {
                "hidden"
            },
        );
    }

    pub(crate) fn open_quick_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.root.clone();
        let editor = self.editor.clone();
        let on_open: Rc<dyn Fn(PathBuf, &mut Window, &mut App) + 'static> =
            Rc::new(move |path, window, cx| {
                let _ = editor.update(cx, |pane, cx| pane.open(&path, window, cx));
            });
        let readiness = self.search_index.readiness();
        let quick_open =
            cx.new(|cx| crate::quick_open::QuickOpen::new(root, on_open, readiness, window, cx));
        self.quick_open_overlay = Some(quick_open.downgrade());
        crate::quick_open::open_dialog(quick_open, window, cx);
    }

    pub(crate) fn open_search_everywhere(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.root.clone();
        let editor = self.editor.clone();
        let on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static> =
            Rc::new(move |path, line, window, cx| {
                let _ = editor.update(cx, |pane, cx| match line {
                    Some(line) => {
                        pane.open_at(&path, line, 0, "search-everywhere", window, cx);
                    }
                    None => pane.open(&path, window, cx),
                });
            });
        let on_action: Rc<dyn Fn(CommandId, &mut Window, &mut App) + 'static> =
            Rc::new(|id, window, cx| {
                let Some(shell) =
                    crate::command_palette::shell().and_then(|handle| handle.upgrade())
                else {
                    return;
                };
                shell.update(cx, |shell, cx| shell.run_command_id(id, window, cx));
            });
        let readiness = self.search_index.readiness();
        let overlay = cx.new(|cx| {
            crate::search_everywhere::SearchEverywhere::new(
                root, on_open, on_action, readiness, window, cx,
            )
        });
        self.search_everywhere_overlay = Some(overlay.downgrade());
        crate::search_everywhere::open_dialog(overlay, window, cx);
    }

    pub(crate) fn open_replace_in_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.root.clone();
        let readiness = self.search_index.readiness();
        let panel =
            cx.new(|cx| crate::replace_in_files::ReplaceInFiles::new(root, readiness, window, cx));
        self.replace_in_files_overlay = Some(panel.downgrade());
        crate::replace_in_files::open_dialog(panel, window, cx);
    }

    pub(crate) fn broadcast_search_index_readiness(&mut self, cx: &mut Context<Self>) {
        let readiness = self.search_index.readiness();
        self.search.update(cx, |panel, cx| {
            panel.set_search_index_readiness(readiness, cx)
        });
        if let Some(overlay) = &self.quick_open_overlay {
            let _ = overlay.update(cx, |view, cx| {
                view.set_search_index_readiness(readiness, cx)
            });
        }
        if let Some(overlay) = &self.search_everywhere_overlay {
            let _ = overlay.update(cx, |view, cx| {
                view.set_search_index_readiness(readiness, cx)
            });
        }
        if let Some(overlay) = &self.replace_in_files_overlay {
            let _ = overlay.update(cx, |view, cx| {
                view.set_search_index_readiness(readiness, cx)
            });
        }
    }

    pub fn apply_menu_action(
        &mut self,
        action: MenuAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(id) = action.command_id() {
            diagnose_menu_run(action, "delegated");
            self.run_command_id(id, window, cx);
            return;
        }
        match action {
            MenuAction::OpenFolder => {
                diagnose_menu_run(action, "prompt");
                self.open_project_picker(window, cx);
            }
            MenuAction::NewTerminalTab => {
                self.bottom_kind = BottomPaneKind::Terminal;
                self.bottom_visible = true;
                let terminal = self.terminal.clone();
                let _ = terminal.update(cx, |pane, cx| pane.new_tab(window, cx));
                diagnose_menu_run(action, "applied");
            }
            MenuAction::CloseTerminalTab => {
                let terminal = self.terminal.clone();
                let closed = terminal.update(cx, |pane, cx| pane.close_active_tab(cx));
                diagnose_menu_run(action, if closed { "applied" } else { "no_tab" });
            }
            MenuAction::Preferences => {
                diagnose_menu_run(action, "open");
                lithe_gpui_settings::open_settings_dialog_at(window, cx, SettingsCategory::General);
            }
            MenuAction::Minimize => {
                window.minimize_window();
                diagnose_menu_run(action, "applied");
            }
            MenuAction::Maximize => {
                window.zoom_window();
                diagnose_menu_run(action, "applied");
            }
            MenuAction::ToggleFullscreen => {
                window.toggle_fullscreen();
                diagnose_menu_run(action, "applied");
            }
            // 窗口 / 工程生命周期三条：都换掉整个窗口（关窗、退进程、换根），所以做完直接
            // 返回——后面那句 `schedule_session_save` + `cx.notify()` 是对着已经不在的窗口。
            MenuAction::CloseWindow => {
                self.flush_session(true, cx);
                unregister_project_window(&self.root, window);
                diagnose_menu_run(action, "closedWindow");
                window.remove_window();
                return;
            }
            MenuAction::Quit => {
                diagnose_menu_run(action, "quit");
                // 会话由 `cx.on_app_quit` 那条订阅统一落盘（见 `ShellWorkspace::new`），
                // 这里再写一次只会白写一遍。
                cx.quit();
                return;
            }
            MenuAction::CloseFolder => {
                self.close_project(window, cx);
                return;
            }
            MenuAction::Undo
            | MenuAction::Redo
            | MenuAction::Cut
            | MenuAction::Copy
            | MenuAction::Paste
            | MenuAction::SelectAll
            | MenuAction::ToggleComment
            | MenuAction::DuplicateLine
            | MenuAction::DeleteLine
            | MenuAction::MoveLineUp
            | MenuAction::MoveLineDown => {
                self.dispatch_editor_action(action, window, cx);
            }
            // 查找 / 替换直接叫编辑器开查找条：`dispatch_action(OpenFind)` 会先经过元素级冒泡
            // （`EditorPane` 自己的 `.on_action`），那条路已 `stop_propagation`，全局观察者
            // 就不再跑——所以在这里直调 `open_find`，与菜单点击是同一条落点（§1.6）。
            MenuAction::Find | MenuAction::FindAndReplace => {
                let with_replace = action == MenuAction::FindAndReplace;
                self.editor
                    .update(cx, |pane, cx| pane.open_find(with_replace, window, cx));
                diagnose_menu_run(action, "find");
            }
            // 快速修复：代码动作菜单是编辑器自己那一块，
            // 外壳直接叫它开（`EditorPane::show_code_actions`）。
            MenuAction::QuickFix => {
                self.editor
                    .update(cx, |pane, cx| pane.show_code_actions(window, cx));
                diagnose_menu_run(action, "codeActions");
            }
            // 文件生命周期：执行分支都在这里，无活动缓冲时由 `EditorPane` 自己 no-op + 诊断。
            MenuAction::NewTab => {
                self.editor
                    .update(cx, |pane, cx| pane.new_untitled(window, cx));
                diagnose_menu_run(action, "newTab");
            }
            MenuAction::NewFile => {
                self.prompt_new_file(window, cx);
            }
            MenuAction::SaveAs => {
                self.editor.update(cx, |pane, cx| pane.save_as(window, cx));
                diagnose_menu_run(action, "saveAs");
            }
            MenuAction::SaveAll => {
                self.editor.update(cx, |pane, cx| pane.save_all(window, cx));
                diagnose_menu_run(action, "saveAll");
            }
            MenuAction::RevertFile => {
                self.editor
                    .update(cx, |pane, cx| pane.revert_active(window, cx));
                diagnose_menu_run(action, "revertFile");
            }
            MenuAction::ToggleActivitySidebar => {
                self.toggle_left_sidebar();
            }
            MenuAction::ToggleSecondarySidebar => {
                self.right_visible = !self.right_visible;
                self.schedule_right_view_scan(self.right_view, self.right_visible, cx);
                diagnose_menu_run(
                    action,
                    if self.right_visible {
                        "visible"
                    } else {
                        "hidden"
                    },
                );
            }
            MenuAction::StartDebugging => {
                self.run.update(cx, |pane, cx| pane.launch(true, cx));
                diagnose_menu_run(action, "debug");
            }
            MenuAction::StopDebugging => {
                self.run.update(cx, |pane, cx| pane.stop(cx));
                diagnose_menu_run(action, "stop");
            }
            MenuAction::ToggleBreakpoint => {
                self.toggle_breakpoint_at_caret(cx);
                diagnose_menu_run(action, "toggleBreakpoint");
            }
            MenuAction::GoToLine => {
                self.editor
                    .update(cx, |pane, cx| pane.open_go_to_line(window, cx));
                diagnose_menu_run(action, "goToLine");
            }
            MenuAction::ToggleWordWrap => match lithe_gpui_settings::try_store(cx) {
                Some(store) => {
                    let next = !store.read(cx).settings().word_wrap;
                    store.update(cx, |store, cx| store.set_word_wrap(next, cx));
                    diagnose_menu_run(action, "toggleWordWrap");
                }
                None => eprintln!(
                    "S1_MENU notWired id=lithe.menu.toggleWordWrap missing=no_settings_store"
                ),
            },
            MenuAction::ToggleMinimap
            | MenuAction::ToggleLineNumbers
            | MenuAction::ToggleRenderWhitespace => {
                self.toggle_editor_display(action, cx);
            }
            MenuAction::GoBack => {
                self.editor.update(cx, |pane, cx| pane.go_back(window, cx));
                diagnose_menu_run(action, "goBack");
            }
            MenuAction::GoForward => {
                self.editor
                    .update(cx, |pane, cx| pane.go_forward(window, cx));
                diagnose_menu_run(action, "goForward");
            }
            MenuAction::ShowKeymapSettings => {
                lithe_gpui_settings::open_settings_dialog_at(
                    window,
                    cx,
                    lithe_gpui_settings::Category::Keyboard,
                );
                diagnose_menu_run(action, "keymap");
            }
            MenuAction::NextTab => {
                self.editor.update(cx, |pane, cx| pane.next_tab(cx));
                diagnose_menu_run(action, "nextTab");
            }
            MenuAction::PreviousTab => {
                self.editor.update(cx, |pane, cx| pane.previous_tab(cx));
                diagnose_menu_run(action, "previousTab");
            }
            MenuAction::ShowHover => {
                self.editor
                    .update(cx, |pane, cx| pane.show_hover_at_caret(window, cx));
                diagnose_menu_run(action, "showHover");
            }
            // 符号级语义动作：三个都是异步取数，派发这一电只负责把请求发出去。
            MenuAction::GoToReferences => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.go_to_references(window, cx));
                diagnose_menu_run(action, "goToReferences");
            }
            MenuAction::RenameSymbol => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.rename_symbol(window, cx));
                diagnose_menu_run(action, "renameSymbol");
            }
            MenuAction::FormatDocument => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.format_document(window, cx));
                diagnose_menu_run(action, "formatDocument");
            }
            MenuAction::ShowLocalHistory => {
                diagnose_menu_run(action, "localHistory");
                self.prompt_local_history(window, cx);
            }
            MenuAction::ToggleMarkdownPreview => {
                self.editor
                    .update(cx, |pane, cx| pane.toggle_markdown_preview(cx));
                diagnose_menu_run(action, "markdownPreview");
            }
            MenuAction::ShowDiagnostics | MenuAction::ShowRunAndDebug => {
                self.bottom_kind = match action {
                    MenuAction::ShowRunAndDebug => BottomPaneKind::Debug,
                    _ => BottomPaneKind::Diagnostics,
                };
                self.bottom_visible = true;
                self.refresh_problems_if_visible(cx);
                diagnose_menu_run(action, "visible");
            }
            MenuAction::ShowFileExplorer | MenuAction::ShowSourceControl => {
                let index = if action == MenuAction::ShowSourceControl {
                    CHANGES_ACTIVITY_IX
                } else {
                    DEFAULT_TOP_ACTIVITY
                };
                self.left_sidebar_visible = true;
                self.select_top_activity(index, false, cx);
                diagnose_menu_run(action, "applied");
            }
            MenuAction::ZoomIn | MenuAction::ZoomOut | MenuAction::ResetZoom => {
                self.apply_zoom(action, cx);
            }
            MenuAction::OpenFile
            | MenuAction::Save
            | MenuAction::CloseTab
            | MenuAction::CloseOtherTabs
            | MenuAction::CloseAllTabs
            | MenuAction::CloseSavedTabs
            | MenuAction::CloseTabsToLeft
            | MenuAction::CloseTabsToRight
            | MenuAction::ReopenClosedTab
            | MenuAction::NewWindow
            | MenuAction::CommandPalette
            | MenuAction::ToggleTerminal
            | MenuAction::ToggleMaven
            | MenuAction::ToggleStatusBar
            | MenuAction::OpenAppearanceSettings
            | MenuAction::ToggleMenuBar
            | MenuAction::GoToDefinition
            | MenuAction::GoToImplementation
            | MenuAction::GoToTypeDefinition
            | MenuAction::QuickOpen
            | MenuAction::SearchEverywhere
            | MenuAction::ReplaceInFiles => {
                diagnose_menu_run(action, "unreachable");
            }
        }
        self.schedule_session_save(cx);
        cx.notify();
    }

    /// File ▸ Close Folder：先落盘会话，再把这一窗换回欢迎页。
    ///
    /// 会话必须先落盘：换根之后旧壳连同 `session` 一起没了，等不到关闭时的 `force` 写盘。
    /// 落盘的目的是下次从欢迎页重新打开这个工程时标签还能回来（与「关窗」的行为一致）。
    fn close_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        println!(
            "S1_PROJECT_CLOSE state=closing root={}",
            self.root.display()
        );
        self.flush_session(true, cx);
        unregister_project_window(&self.root, window);
        replace_root_with_welcome(window, cx);
        diagnose_menu_run(MenuAction::CloseFolder, "closed");
    }

    /// File ▸ 显示本地历史：列出当前文件的历史快照，点一条以只读缓冲打开。
    ///
    /// 列表里只放「这一份快照」的数据（文件名主干里的 unix 毫秒 + 字节数），不放散文：
    /// 时间与体积都是数字，不需要翻译，也不会因为语言不同而排序错乱。
    fn prompt_local_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let entries = self.editor.read(cx).local_history_entries(cx);
        println!(
            "S1_LOCAL_HISTORY list count={} file={}",
            entries.len(),
            self.editor
                .read(cx)
                .active_text_format(cx)
                .map(|_| "active")
                .unwrap_or("none")
        );
        let title = tr("lithe.menu.showLocalHistory");
        let empty = entries.is_empty();
        let editor = self.editor.clone();
        window.open_dialog(
            "lithe-local-history",
            move |id, _window, _cx| {
                let mut dialog = Dialog::new(id).title(title.clone()).on_close(dismiss_hosted);
                if empty {
                    dialog = dialog
                        .child(div().text_sm().child(tr("lithe.gpui.localHistoryEmpty")));
                } else {
                    let mut list = v_stack().gap_1();
                    for (index, snapshot) in entries.iter().enumerate() {
                        let label = SharedString::from(snapshot.label());
                        let snapshot = snapshot.clone();
                        let editor = editor.clone();
                        list = list.child(
                            Button::new(SharedString::from(format!("local-history-{index}")))
                                .label(label)
                                .on_click(move |_, window, cx| {
                                    editor.update(cx, |pane, cx| {
                                        pane.open_local_history_snapshot(&snapshot, window, cx)
                                    });
                                    window.close_dialog(cx);
                                }),
                        );
                    }
                    dialog = dialog.child(list);
                }
                dialog.into_any_element()
            },
            cx,
        );
    }

    fn dispatch_editor_action(
        &mut self,
        action: MenuAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(bound) = action.keybind_action() else {
            diagnose_menu_run(action, "no_action");
            return;
        };
        let Some(target) = crate::menu_bar::action_target() else {
            diagnose_menu_run(action, "no_target");
            return;
        };
        target.focus(window, cx);
        window.dispatch_action(bound, cx);
        diagnose_menu_run(action, "dispatched");
    }

    // ------------------------------------------------------------------
    // 文件生命周期（B2 / B1）：新建、另存为、全部保存、还原，以及 explorer 的增删改名
    // ------------------------------------------------------------------

    /// File ▸ New File: pick a path under the workspace root, then hand `(parent, name)` to the
    /// editor's single create entry point — the same one the tree's New File uses (裁决 #11). With
    /// no workspace root there is nowhere to create, so say so instead of opening a picker.
    fn prompt_new_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.root.as_os_str().is_empty() {
            eprintln!("S1_EDITOR_NEW_FILE create=false reason=no-workspace-root via=menu");
            self.notify(
                lithe_gpui_notify::Input::new(
                    "lithe.files.openFolderFirst",
                    lithe_gpui_notify::Severity::Warning,
                )
                .key("lithe.files.openFolderFirst"),
                cx,
            );
            return;
        }
        let root = self.root.clone();
        let picked = cx.prompt_for_new_path(&root, Some("NewFile"));
        cx.spawn_in(window, async move |shell, cx| {
            let path = match picked.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) => {
                    eprintln!("S1_EDITOR_NEW_FILE create=false reason=cancelled via=menu");
                    return;
                }
                Ok(Err(error)) => {
                    eprintln!("S1_EDITOR_NEW_FILE create=false reason=error error={error}");
                    return;
                }
                Err(_) => {
                    eprintln!("S1_EDITOR_NEW_FILE create=false reason=channel-closed");
                    return;
                }
            };
            let _ = shell.update_in(cx, |shell, window, cx| {
                shell.create_at_picked_path(path, window, cx)
            });
        })
        .detach();
    }

    /// Splits a picked path into `(parent, name)` and creates through the editor, so the menu and
    /// the tree share one on-disk entry point; then reveals whatever appeared.
    fn create_at_picked_path(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
        else {
            eprintln!(
                "S1_EDITOR_NEW_FILE create=false reason=no-file-name path={}",
                path.display()
            );
            return;
        };
        let Some(parent) = path.parent().map(Path::to_path_buf) else {
            eprintln!(
                "S1_EDITOR_NEW_FILE create=false reason=no-parent path={}",
                path.display()
            );
            return;
        };
        self.editor.update(cx, |pane, cx| {
            pane.create_file(Some(parent), name, window, cx)
        });
        self.reveal_created(path, window, cx);
    }

    /// The tree row menu's Delete: confirm first (no recycle bin yet, 裁决 #9), then remove. The
    /// confirm dialog has no input field, so Enter is structurally not "delete".
    fn prompt_delete(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let is_dir = path.is_dir();
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        let (title_key, body_key) = if is_dir {
            (
                "lithe.fileExplorer.deleteFolder",
                "lithe.fileExplorer.deleteFolderMessage",
            )
        } else {
            (
                "lithe.fileExplorer.deleteFile",
                "lithe.fileExplorer.deleteFileMessage",
            )
        };
        let title = tr(title_key);
        let body = tr_args(body_key, &[("name", &name)]);
        let target = path.to_path_buf();
        let shell = cx.entity().downgrade();
        let delete_label = tr("lithe.ui.delete");
        window.open_dialog(
            "lithe-explorer-delete",
            move |id, _window, _cx| {
                let shell = shell.clone();
                let target = target.clone();
                ConfirmDialog::new(id, title.clone(), body.clone(), dismiss_hosted)
                    .confirm(delete_label.clone())
                    .destructive()
                    .on_confirm(move |window, cx| {
                        let _ = shell.update(cx, |shell, cx| {
                            shell.delete_confirmed(target.clone(), window, cx)
                        });
                    })
                    .into_any_element()
            },
            cx,
        );
    }

    /// Deletes a confirmed target: remove it, forget every buffer under it, then reload the tree
    /// and land on the parent row. A failure notifies and leaves the explorer alone (B1-19).
    fn delete_confirmed(&mut self, target: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let is_dir = target.is_dir();
        let removed = if is_dir {
            std::fs::remove_dir_all(&target)
        } else {
            std::fs::remove_file(&target)
        };
        if let Err(error) = removed {
            eprintln!(
                "S1_EXPLORER delete result=failed path={} error={error}",
                target.display()
            );
            let name = target
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            self.notify(
                lithe_gpui_notify::Input::new(
                    "lithe.files.deleteFailed",
                    lithe_gpui_notify::Severity::Error,
                )
                .param("name", name)
                .param("error", error.to_string())
                .key("lithe.files.deleteFailed"),
                cx,
            );
            return;
        }
        eprintln!(
            "S1_EXPLORER delete result=applied path={}",
            target.display()
        );
        self.editor
            .update(cx, |pane, cx| pane.forget_paths(&target, window, cx));
        // B1-17: pick the parent row; a top-level child falls back to the workspace root.
        let parent = target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.root.clone());
        self.explorer.update(cx, |explorer, cx| {
            explorer.refresh_and_reveal(parent, window, cx)
        });
        self.schedule_session_save(cx);
    }

    /// The tree's New File / New Folder: one `PromptDialog` asks for the name and refuses a bad one
    /// before submitting (F5). File goes through the editor; a folder is a plain `create_dir`.
    fn prompt_new_entry(
        &mut self,
        parent: &Path,
        is_folder: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (title_key, label_key, placeholder_key, submit_key) = if is_folder {
            (
                "lithe.files.nameNewFolder",
                "lithe.files.enterNewFolderName",
                "lithe.files.folderNamePlaceholder",
                "lithe.files.newFolder",
            )
        } else {
            (
                "lithe.files.nameNewFile",
                "lithe.files.enterNewFileName",
                "lithe.files.fileNamePlaceholder",
                "lithe.files.newFile",
            )
        };
        let parent = parent.to_path_buf();
        self.new_entry_field.update(cx, |field, cx| {
            field.set_text(String::new(), cx);
            field.set_placeholder(tr(placeholder_key), cx);
        });
        let field = self.new_entry_field.clone();
        let shell = cx.entity().downgrade();
        let title = tr(title_key);
        let label = tr(label_key);
        let submit = tr(submit_key);
        let check_parent = parent.clone();
        let check = Arc::new(move |text: &str| check_new_name(&check_parent, text, is_folder));
        let submit_parent = parent.clone();
        window.open_dialog(
            "lithe-explorer-new-entry",
            move |id, _window, _cx| {
                let check = check.clone();
                let shell = shell.clone();
                let parent = submit_parent.clone();
                PromptDialog::new(id, title.clone(), &field, dismiss_hosted)
                    .label(label.clone())
                    .submit(submit.clone())
                    .check(move |text| check(text))
                    .on_submit(move |text, window, cx| {
                        let name = text.to_string();
                        let _ = shell.update(cx, |shell, cx| {
                            shell.finish_new_entry(parent.clone(), is_folder, name, window, cx)
                        });
                    })
                    .into_any_element()
            },
            cx,
        );
    }

    /// Creates the named entry under `parent`: a file through the editor's `create_file` (the menu
    /// path shares it), a folder through `create_dir`. Both reveal the new row afterwards.
    fn finish_new_entry(
        &mut self,
        parent: PathBuf,
        is_folder: bool,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = parent.join(&name);
        if is_folder {
            match std::fs::create_dir(&target) {
                Ok(()) => {
                    println!(
                        "S1_EXPLORER new result=applied kind=folder path={}",
                        target.display()
                    );
                    self.explorer.update(cx, |explorer, cx| {
                        explorer.refresh_and_reveal(target, window, cx)
                    });
                }
                Err(error) => {
                    eprintln!(
                        "S1_EXPLORER new result=failed kind=folder path={} error={error}",
                        target.display()
                    );
                    self.notify(
                        lithe_gpui_notify::Input::new(
                            "lithe.files.createFileFailedMessage",
                            lithe_gpui_notify::Severity::Error,
                        )
                        .param("error", error.to_string())
                        .key("lithe.files.createFileFailedMessage"),
                        cx,
                    );
                }
            }
            return;
        }
        self.editor.update(cx, |pane, cx| {
            pane.create_file(Some(parent), name, window, cx)
        });
        // The check already refused an existing name, so a file at `target` is the one just made.
        self.reveal_created(target, window, cx);
    }

    /// The tree's inline rename, committed: re-check, rename on disk, move every pane-owned path
    /// table, reload the tree, and land on the new row. The inline check already ran; the rules
    /// are re-run here because the check and the write are not one transaction.
    fn perform_rename(
        &mut self,
        path: &Path,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(key) = invalid_name_key(name) {
            eprintln!(
                "S1_EXPLORER rename result=invalid path={} name={name}",
                path.display()
            );
            self.notify(
                lithe_gpui_notify::Input::new(key, lithe_gpui_notify::Severity::Warning).key(key),
                cx,
            );
            return;
        }
        let Some(parent) = path.parent().map(Path::to_path_buf) else {
            eprintln!(
                "S1_EXPLORER rename result=no-parent path={}",
                path.display()
            );
            return;
        };
        let target = parent.join(name);
        if target != path && target.exists() {
            // Case-only rename on a case-insensitive volume points at the same file: that is a
            // legal rename, not a clash.
            let same_file = matches!(
                (target.canonicalize(), path.canonicalize()),
                (Ok(a), Ok(b)) if a == b
            );
            if !same_file {
                let key = if target.is_dir() {
                    "lithe.fileExplorer.folderAlreadyExistsMessage"
                } else {
                    "lithe.fileExplorer.fileAlreadyExistsMessage"
                };
                eprintln!(
                    "S1_EXPLORER rename result=exists path={} target={}",
                    path.display(),
                    target.display()
                );
                self.notify(
                    lithe_gpui_notify::Input::new(key, lithe_gpui_notify::Severity::Warning)
                        .key(key),
                    cx,
                );
                return;
            }
        }
        if let Err(error) = std::fs::rename(path, &target) {
            eprintln!(
                "S1_EXPLORER rename result=failed path={} target={} error={error}",
                path.display(),
                target.display()
            );
            self.notify(
                lithe_gpui_notify::Input::new(
                    "lithe.fileExplorer.moveFailed",
                    lithe_gpui_notify::Severity::Error,
                )
                .key("lithe.fileExplorer.moveFailed"),
                cx,
            );
            return;
        }
        eprintln!(
            "S1_EXPLORER rename result=applied from={} to={}",
            path.display(),
            target.display()
        );
        self.editor
            .update(cx, |pane, cx| pane.remap_paths(path, &target, window, cx));
        self.explorer.update(cx, |explorer, cx| {
            explorer.refresh_and_reveal(target, window, cx)
        });
        self.schedule_session_save(cx);
    }

    /// Reveals a path the shell just created, once the explorer has reloaded.
    fn reveal_created(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if path.exists() {
            self.explorer.update(cx, |explorer, cx| {
                explorer.refresh_and_reveal(path, window, cx)
            });
        }
    }

    /// 翻一个编辑器显示开关（缩略图 / 行号 / 空白渲染）：设置 store 是唯一真源，
    /// 翻它 → 订阅把新档位同步给 `EditorPane`（与 `ToggleWordWrap` 同一条链）。
    fn toggle_editor_display(&mut self, action: MenuAction, cx: &mut Context<Self>) {
        let Some(store) = lithe_gpui_settings::try_store(cx) else {
            eprintln!(
                "S1_MENU notWired id={} missing=no_settings_store",
                action.label_key().trim_start_matches("lithe.")
            );
            return;
        };
        let settings = store.read(cx).settings().clone();
        let next = match action {
            MenuAction::ToggleMinimap => !settings.minimap,
            MenuAction::ToggleLineNumbers => !settings.line_numbers,
            MenuAction::ToggleRenderWhitespace => !settings.render_whitespace,
            _ => return,
        };
        store.update(cx, |store, cx| match action {
            MenuAction::ToggleMinimap => store.set_minimap(next, cx),
            MenuAction::ToggleLineNumbers => store.set_line_numbers(next, cx),
            MenuAction::ToggleRenderWhitespace => store.set_render_whitespace(next, cx),
            _ => {}
        });
        diagnose_menu_run(action, if next { "on" } else { "off" });
    }

    fn apply_zoom(&mut self, action: MenuAction, cx: &mut Context<Self>) {
        let Some(store) = lithe_gpui_settings::try_store(cx) else {
            diagnose_menu_run(action, "unavailable");
            return;
        };
        let current = store.read(cx).settings().font_size;
        let next = match action {
            MenuAction::ZoomIn => current + 1.0,
            MenuAction::ZoomOut => current - 1.0,
            _ => lithe_gpui_settings::Settings::default().font_size,
        };
        store.update(cx, |store, cx| store.set_editor_font_size(next, cx));
        let applied = store.read(cx).settings().font_size;
        eprintln!("S1_ZOOM font_size={applied} requested={next}");
        diagnose_menu_run(action, "applied");
    }

    /// 跑一条命令。命令面板给的就是 `CommandId` 本身：行由它按查询裁出来，
    /// 索引与动作同一份列表，不再有「按下的行」与「跑起来的命令」错位那类事。
    pub(crate) fn run_command_id(
        &mut self,
        id: CommandId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match id {
            CommandId::SaveBuffer => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.save_active(window, cx));
                diagnose_run(id, "saved");
            }
            CommandId::OpenCommandPalette => {
                diagnose_run(id, "open");
                crate::command_palette::open_command_palette(window, cx);
            }
            CommandId::OpenQuickOpen => {
                diagnose_run(id, "open");
                self.open_quick_open(window, cx);
            }
            CommandId::OpenSearchEverywhere => {
                diagnose_run(id, "open");
                self.open_search_everywhere(window, cx);
            }
            CommandId::OpenReplaceInFiles => {
                diagnose_run(id, "open");
                self.open_replace_in_files(window, cx);
            }
            CommandId::OpenFile => {
                diagnose_run(id, "prompt");
                let picked = cx.prompt_for_paths(PathPromptOptions {
                    files: true,
                    directories: false,
                    multiple: true,
                    prompt: None,
                });
                let editor = self.editor.clone();
                let shell_for_open = cx.entity().downgrade();
                let opened = cx.spawn_in(window, async move |_this, async_cx| {
                    let result = match picked.await {
                        Ok(Ok(result)) => result,
                        Ok(Err(error)) => {
                            eprintln!("S1_EDITOR_OPEN_FILE state=failed error={error}");
                            return;
                        }
                        Err(_) => {
                            eprintln!("S1_EDITOR_OPEN_FILE state=cancelled reason=channel-closed");
                            return;
                        }
                    };
                    let Some(paths) = result else {
                        eprintln!("S1_EDITOR_OPEN_FILE state=cancelled");
                        return;
                    };
                    if paths.is_empty() {
                        eprintln!("S1_EDITOR_OPEN_FILE state=empty");
                        return;
                    }
                    eprintln!(
                        "S1_EDITOR_OPEN_FILE state=picked count={} first={}",
                        paths.len(),
                        paths[0].display()
                    );
                    let _ = async_cx.update(move |window, cx| {
                        let _ = shell_for_open.update(cx, |this, cx| {
                            this.close_extensions_page(None, cx);
                        });
                        for path in &paths {
                            eprintln!("S1_EDITOR_OPEN path={}", path.display());
                            let _ = editor.update(cx, |pane, cx| pane.open(path, window, cx));
                        }
                        let active = editor.read(cx).active_buffer_name();
                        eprintln!("S1_EDITOR_OPEN_FILE state=opened active={active}");
                    });
                });
                opened.detach();
            }
            CommandId::CloseTab => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.close_active(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::CloseOtherTabs => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.close_other_tabs(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::CloseAllTabs => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.close_all_tabs(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::CloseSavedTabs => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.close_saved_tabs(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::CloseTabsToLeft => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.close_tabs_to_left(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::CloseTabsToRight => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.close_tabs_to_right(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::ReopenClosedTab => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.reopen_closed_tab(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::NewWindow => {
                diagnose_run(id, "requested");
                self.open_current_project_in_new_window(window, cx);
            }
            CommandId::NavigateToDefinition => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.navigate_to_definition(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::NavigateToImplementation => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.navigate_to_implementation(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::NavigateToTypeDefinition => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.navigate_to_type_definition(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::GoToReferences => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.go_to_references(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::RenameSymbol => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.rename_symbol(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::FormatDocument => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.format_document(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::ToggleTerminal => {
                let terminal_handle = self.terminal.clone();
                let (visible, ensure_session) =
                    if self.bottom_visible && self.bottom_kind == BottomPaneKind::Terminal {
                        self.bottom_visible = false;
                        (false, false)
                    } else {
                        self.bottom_kind = BottomPaneKind::Terminal;
                        self.bottom_visible = true;
                        (true, true)
                    };
                diagnose_run(id, if visible { "visible" } else { "hidden" });
                if ensure_session {
                    let _ = terminal_handle.update(cx, |pane, cx| pane.ensure_session(window, cx));
                }
            }
            CommandId::ToggleMaven => {
                let (view, visible) = resolve_right_click(
                    RightToolWindowView::Maven,
                    self.right_view,
                    self.right_visible,
                );
                self.right_view = view;
                self.right_visible = visible;
                self.schedule_right_view_scan(view, visible, cx);
                diagnose_right_panel(view, visible, None);
                diagnose_run(id, if visible { "visible" } else { "hidden" });
            }
            CommandId::ToggleStatusBar => {
                let next = lithe_gpui_settings::try_store(cx)
                    .map(|store| !store.read(cx).settings().show_status_bar)
                    .unwrap_or(false);
                if let Some(store) = lithe_gpui_settings::try_store(cx) {
                    store.update(cx, |store, cx| store.set_show_status_bar(next, cx));
                    diagnose_run(id, if next { "visible" } else { "hidden" });
                } else {
                    diagnose_run(id, "unavailable");
                }
            }
            CommandId::OpenSettings => {
                diagnose_run(id, "open");
                lithe_gpui_settings::open_settings_dialog_at(window, cx, SettingsCategory::General);
            }
            CommandId::OpenAppearanceSettings => {
                diagnose_run(id, "open");
                lithe_gpui_settings::open_settings_dialog_at(
                    window,
                    cx,
                    SettingsCategory::Appearance,
                );
            }
            CommandId::SwitchToLightTheme | CommandId::SwitchToDarkTheme => {
                self.apply_theme_command(id == CommandId::SwitchToDarkTheme, cx);
            }
            CommandId::ToggleMenuBar => crate::menu_bar::toggle_menu_bar(cx),
        }
        self.schedule_session_save(cx);
        cx.notify();
    }

    fn apply_theme_command(&mut self, dark: bool, cx: &mut Context<Self>) {
        let id = if dark {
            CommandId::SwitchToDarkTheme
        } else {
            CommandId::SwitchToLightTheme
        };
        let Some(store) = lithe_gpui_settings::try_store(cx) else {
            diagnose_run(id, "unavailable");
            return;
        };
        let settings = store.read(cx).settings().clone();
        let name = if dark {
            settings.auto_theme_dark.clone()
        } else {
            settings.auto_theme_light.clone()
        };
        store.update(cx, |store, cx| {
            store.set_theme_explicit(name.into(), cx);
        });
        diagnose_run(id, "applied");
    }

    fn footer_left(&mut self, cx: &mut Context<Self>) -> Vec<StatusEntry> {
        let mut entries: Vec<StatusEntry> = Vec::new();

        let project_name: SharedString = self
            .projects
            .first()
            .map(|tab| tab.name().clone())
            .unwrap_or_default();
        let active_file = self.editor.read(cx).active_buffer_name();
        {
            let active_path = self.editor.read(cx).active_buffer_path();
            self.explorer.update(cx, |explorer, cx| {
                explorer.follow_active_document_path(active_path.as_deref(), cx);
            });
        }
        let branch = self
            .changes
            .read(cx)
            .branch()
            .map(|branch| branch.to_string())
            .unwrap_or_else(|| "—".to_string());

        entries.push(StatusEntry::new(project_name).with_file_icon(&active_file, cx));
        entries.push(
            StatusEntry::new(SharedString::from(branch)).with_idea_icon(&idea::GIT_BRANCH_ICON, cx),
        );
        entries
    }

    fn dispatch_notification_toasts(
        &mut self,
        store: &Entity<lithe_gpui_notify::Store>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pending = store.read(cx).entries_since(self.notification_toast_seq);
        if pending.is_empty() {
            return;
        }
        self.notification_toast_seq = pending.last().map(|entry| entry.seq()).unwrap_or_default();
        for entry in &pending {
            self.show_notification_toast(entry, window, cx);
        }
        cx.notify();
    }

    fn show_notification_toast(
        &self,
        entry: &lithe_gpui_notify::Entry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 通知的落地形态是 lithe-ui 的 `Toast`（挂在 `WindowRoot` 的 `Toaster` 上）。
        // 三处语义：
        // 1. 位置：`ToastViewport` 固定右下角（要换角落是给 `ToastViewport` 加参数，
        //    不是在宿主里做）。
        // 2. 上限：同屏 3 条（`lithe_ui::feedback::toast` 的 `MOST`），与 macOS 真源
        //    `WorkbenchNotificationFeatureModel.maximumVisibleCount` 一致。
        // 3. 溢出：丢最旧那条（与上游通知列表语义相同）。
        let toast = Toast::new(crate::notifications::render_message(entry)).severity(
            match entry.severity() {
                lithe_gpui_notify::Severity::Info => NoticeSeverity::Info,
                lithe_gpui_notify::Severity::Warning => NoticeSeverity::Warning,
                // lithe-ui 的严重度档位叫 `Danger`（不是 `Error`）：颜色/图标由它自己的
                // 主题表给。
                lithe_gpui_notify::Severity::Error => NoticeSeverity::Danger,
            },
        );
        if !window.push_notification(toast, cx) {
            // 没挂宿主的窗口：如实报出来，不假装通知已经发出去了。
            eprintln!("S1_NOTIFICATION toast=false reason=no-window-root");
        }
    }
    fn note_notifications_opened(&mut self, cx: &mut Context<Self>) {
        self.notifications
            .update(cx, |store, _| store.mark_all_read());
        let read_upto = self.notifications.read(cx).read_upto_seq();
        eprintln!("S1_NOTIFICATION action=open readUpToSeq={read_upto}");
    }

    /// 语言服务就绪/失败时弹一条通知。「启动中」不弹：那一条由状态栏常驻显示，
    /// 每次打开工程都弹一次只会变成噪音。
    fn on_java_phase(&mut self, phase: JavaServicePhase, cx: &mut Context<Self>) {
        match phase {
            JavaServicePhase::Ready { elapsed } => {
                let elapsed = format!("{:.1}", elapsed.as_secs_f64());
                self.notify(
                    lithe_gpui_notify::Input::new(
                        "lithe.gpui.javaServiceReady",
                        lithe_gpui_notify::Severity::Info,
                    )
                    .param("elapsed", elapsed.as_str())
                    .key("java-service-phase"),
                    cx,
                );
            }
            JavaServicePhase::Failed(reason) => {
                self.notify(
                    lithe_gpui_notify::Input::new(
                        "lithe.gpui.javaServiceFailed",
                        lithe_gpui_notify::Severity::Error,
                    )
                    .param("reason", reason.as_str())
                    .key("java-service-phase"),
                    cx,
                );
            }
            JavaServicePhase::Starting | JavaServicePhase::Idle => {}
        }
    }

    fn footer_right(&self, cx: &App) -> Vec<StatusEntry> {
        let cursor = self.editor.read(cx).cursor_position(cx);
        let tab_size = self.editor.read(cx).tab_size().to_string();

        let mut entries: Vec<StatusEntry> = Vec::new();
        // 语言服务那一档：启动中 / 就绪 / 失败都常驻显示，用户不必去翻日志。
        match self.editor.read(cx).java_service_phase() {
            JavaServicePhase::Starting => {
                entries.push(StatusEntry::new(tr("lithe.gpui.javaServiceStatusStarting")));
            }
            JavaServicePhase::Ready { elapsed } => {
                let elapsed = format!("{:.1}", elapsed.as_secs_f64());
                entries.push(StatusEntry::new(tr_args(
                    "lithe.gpui.javaServiceStatusReady",
                    &[("elapsed", elapsed.as_str())],
                )));
            }
            JavaServicePhase::Failed(_) => {
                entries.push(StatusEntry::new(tr("lithe.gpui.javaServiceStatusFailed")));
            }
            JavaServicePhase::Idle => {}
        }

        // 编码与行尾符读真实缓冲区。没有活动文件（或正文只是只读说明）时整两条不画：
        // 那时候说「UTF-8」是没有根据的，而空胶囊只是个空方块。
        let mut format_entries: Vec<StatusEntry> = Vec::new();
        if let Some((encoding, line_ending)) = self.editor.read(cx).active_text_format(cx) {
            let encoding_editor = self.editor.clone();
            format_entries.push(
                StatusEntry::new(SharedString::from(encoding)).on_click(move |_, window, cx| {
                    encoding_editor.update(cx, |pane, cx| pane.cycle_active_encoding(window, cx));
                }),
            );
            let eol_editor = self.editor.clone();
            format_entries.push(
                StatusEntry::new(SharedString::from(line_ending)).on_click(move |_, _, cx| {
                    eol_editor.update(cx, |pane, cx| pane.toggle_active_line_ending(cx));
                }),
            );
        }

        entries.push(StatusEntry::new(SharedString::from(format!(
            "{}:{}",
            cursor.line, cursor.column
        ))));
        entries.extend(format_entries);
        entries.extend(vec![
            StatusEntry::new(lithe_gpui_shared::tr_args(
                "lithe.footer.spaces",
                &[("count", tab_size.as_str())],
            )),
            if self.editor.read(cx).active_buffer_is_readonly() {
                StatusEntry::new(SharedString::from("")).with_idea_icon(&idea::LOCK_ICON, cx)
            } else {
                StatusEntry::new(SharedString::from("")).with_idea_icon(&idea::LOCK_OPEN_ICON, cx)
            },
            StatusEntry::new(SharedString::from("")).with_idea_icon(&idea::CHECK_CIRCLE_ICON, cx),
        ]);
        entries
    }

    fn is_activity_active(&self, index: usize) -> bool {
        if TOP_ACTIVITY_ITEMS.contains(&index) {
            return self.top_activity_view == Some(index);
        }
        match bottom_pane_for(index) {
            Some(kind) => self.bottom_visible && self.bottom_kind == kind,
            None => false,
        }
    }

    fn refresh_problems_if_visible(&mut self, cx: &mut Context<Self>) {
        if self.bottom_visible && self.bottom_kind == BottomPaneKind::Diagnostics {
            self.editor.update(cx, |pane, cx| pane.refresh_problems(cx));
        }
    }

    fn is_right_activity_active(&self, index: usize) -> bool {
        if index == EXTENSIONS_RAIL_IX {
            return self.extensions_page_open;
        }
        match RightToolWindowView::from_rail_index(index) {
            Some(view) => self.right_visible && self.right_view == view,
            None => false,
        }
    }

    fn schedule_right_view_scan(
        &mut self,
        view: RightToolWindowView,
        visible: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(target) = self.right_scan.request(view, visible) else {
            return;
        };
        let root = self.root.clone();
        eprintln!("S1_RIGHT_SCAN view={} root={}", view.id(), root.display());
        cx.spawn(async move |this, cx| match target {
            RightToolWindowView::Maven => {
                let project = crate::maven::scan(&root);
                let _ = this.update(cx, |this, cx| {
                    this.maven_project = project;
                    if right_scan_should_notify(target, this.right_view, this.right_visible) {
                        cx.notify();
                    }
                });
            }
            RightToolWindowView::Spring => {
                let index = crate::spring::index(&root);
                let _ = this.update(cx, |this, cx| {
                    this.spring_index = index;
                    if right_scan_should_notify(target, this.right_view, this.right_visible) {
                        cx.notify();
                    }
                });
            }
            RightToolWindowView::Notifications => {}
        })
        .detach();
    }

    pub fn show_right_view_probe(&mut self, view: RightToolWindowView, cx: &mut Context<Self>) {
        self.right_view = view;
        self.right_visible = true;
        if view == RightToolWindowView::Notifications {
            self.note_notifications_opened(cx);
        }
        self.schedule_right_view_scan(view, true, cx);
        diagnose_right_panel(view, true, None);
        cx.notify();
    }

    fn open_extensions_page(
        &mut self,
        point: Option<ProbePoint>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.extensions_page_open = true;
        if self.extensions_page.is_none() {
            self.extensions_page =
                Some(cx.new(|cx| crate::extensions_page::ExtensionsPage::new(window, cx)));
        }
        diagnose_extensions_page(true, point);
        cx.notify();
    }

    pub(crate) fn close_extensions_page(
        &mut self,
        point: Option<ProbePoint>,
        cx: &mut Context<Self>,
    ) {
        self.extensions_page_open = false;
        diagnose_extensions_page(false, point);
        cx.notify();
    }

    pub(crate) fn toggle_extensions_page(
        &mut self,
        point: Option<ProbePoint>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.extensions_page_open {
            self.close_extensions_page(point, cx);
        } else {
            self.open_extensions_page(point, window, cx);
        }
    }

    pub(crate) fn open_project_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            eprintln!("S1_OPEN_PROJECT state=skipped reason=active_dialog");
            return;
        }
        eprintln!("S1_OPEN_PROJECT state=prompt");
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        let shell = cx.entity().downgrade();
        let task = cx.spawn_in(window, async move |_this, async_cx| {
            let result = match picked.await {
                Ok(Ok(result)) => result,
                Ok(Err(error)) => {
                    eprintln!("S1_OPEN_PROJECT state=failed error={error}");
                    return;
                }
                Err(_) => {
                    eprintln!("S1_OPEN_PROJECT state=cancelled reason=channel-closed");
                    return;
                }
            };
            let Some(paths) = result else {
                eprintln!("S1_OPEN_PROJECT state=cancelled");
                return;
            };
            let Some(target) = paths.into_iter().next() else {
                eprintln!("S1_OPEN_PROJECT state=empty");
                return;
            };
            eprintln!("S1_OPEN_PROJECT state=picked path={}", target.display());
            let _ = async_cx.update(move |window, cx| {
                let _ = shell.update(cx, |shell, cx| {
                    shell.request_open_project(target, None, window, cx)
                });
            });
        });
        task.detach();
    }

    pub(crate) fn request_open_project(
        &mut self,
        target: PathBuf,
        explicit: Option<OpenDestination>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path_text = target.to_string_lossy().to_string();

        if !target.is_dir() {
            let marked = self.recent_projects.set_missing(&path_text, true);
            if marked {
                self.save_recent_projects();
            }
            eprintln!("S1_OPEN_PROJECT state=missing path={path_text} marked={marked}");
            let key = if target.exists() {
                "lithe.fileSystem.recentProjectNotFolder"
            } else {
                "lithe.fileSystem.recentProjectUnavailable"
            };
            self.notify(
                lithe_gpui_notify::Input::new(key, lithe_gpui_notify::Severity::Error)
                    .param("path", path_text.as_str())
                    .key("recent-project-missing"),
                cx,
            );
            return;
        }

        let (ask, open_in_new_window) = self.project_open_preference(cx);
        match resolve_project_open_destination(ask, open_in_new_window, explicit) {
            None => self.open_project_where_dialog(target, window, cx),
            Some(decision) => self.execute_project_open(target, decision, window, cx),
        }
    }

    fn project_open_preference(&self, cx: &App) -> (bool, bool) {
        lithe_gpui_settings::try_store(cx)
            .map(|store| {
                let settings = store.read(cx).settings();
                (
                    settings.ask_where_to_open_projects,
                    settings.open_folders_in_new_window,
                )
            })
            .unwrap_or((true, true))
    }

    fn execute_project_open(
        &mut self,
        target: PathBuf,
        decision: ProjectOpenDecision,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match decision.destination {
            OpenDestination::NewWindow => match route_project_window(&target, window, cx) {
                ProjectWindowRoute::Opened => {
                    eprintln!(
                        "S1_OPEN_PROJECT destination=new-window state=opened path={}",
                        target.display()
                    );
                    self.record_opened_project(&target, Some(true), cx);
                    if decision.remember {
                        self.remember_open_destination(true, cx);
                    }
                }
                ProjectWindowRoute::Focused => eprintln!(
                    "S1_OPEN_PROJECT destination=new-window state=focused path={}",
                    target.display()
                ),
                ProjectWindowRoute::NotWired => self.report_new_window_not_wired(cx),
                ProjectWindowRoute::Failed => eprintln!(
                    "S1_OPEN_PROJECT destination=new-window state=failed path={}",
                    target.display()
                ),
            },
            OpenDestination::ThisWindow => {
                self.rebuild_project_window(target, decision.remember, window, cx)
            }
        }
    }

    fn report_new_window_not_wired(&mut self, cx: &mut Context<Self>) {
        eprintln!(
            "S1_OPEN_PROJECT destination=new-window state=not_wired missing=window_handle_routing"
        );
        self.notify_new_window_not_wired(cx);
    }

    /// File ▸ New Window（真源 `LitheApp.swift:492` 的 `CommandGroup(replacing: .newItem)`）。
    /// 真源那一步是"把主窗口再呈现一次"（`ensurePrimaryWindowAvailable` + `presentPrimary`），
    /// 而本侧窗口尺寸与选项归组合根，所以这里只把**当前工程根**交给 `OpenWindowHost`；新窗口是
    /// 自己一份 `ShellWorkspace`。真源里"两个窗口共享同一份会话"记在「按窗口的会话」那一片。
    /// 没有装端口时维持未接线提示（headless / 单测不假装成功）。
    fn open_current_project_in_new_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.root.clone();
        eprintln!(
            "S1_OPEN_WINDOW source=menu state=requested root={}",
            root.display()
        );
        match route_project_window(&root, window, cx) {
            ProjectWindowRoute::Opened => {
                eprintln!("S1_OPEN_WINDOW source=menu state=opened root={}", root.display());
            }
            // 真源那一步是 `ensurePrimaryWindowAvailable` + `presentPrimary`：这个工程已经有窗口时
            // 只把它抬到前面，不再开第二个。
            ProjectWindowRoute::Focused => {
                eprintln!(
                    "S1_OPEN_WINDOW source=menu state=focused existing={}",
                    root.display()
                );
            }
            ProjectWindowRoute::NotWired => {
                eprintln!(
                    "S1_OPEN_WINDOW source=menu state=not_wired missing=window_handle_routing"
                );
                self.notify_new_window_not_wired(cx);
            }
            ProjectWindowRoute::Failed => {
                eprintln!("S1_OPEN_WINDOW source=menu state=failed root={}", root.display());
                self.notify_new_window_not_wired(cx);
            }
        }
    }

    /// 「打开工程 → 新窗口」与「File ▸ New Window」共用同一条用户提示，诊断锚点各自打印。
    fn notify_new_window_not_wired(&mut self, cx: &mut Context<Self>) {
        self.notify_not_wired("lithe.gpui.newWindowNotWired", "New Window", cx);
    }

    fn report_menu_not_wired(
        &mut self,
        label_key: &'static str,
        missing: Missing,
        cx: &mut Context<Self>,
    ) {
        missing.diagnose(label_key);
        self.notify_not_wired(missing.text_key, label_key, cx);
    }

    pub(crate) fn report_panel_action_not_wired(
        &mut self,
        action_id: &'static str,
        missing: Missing,
        cx: &mut Context<Self>,
    ) {
        eprintln!(
            "S1_PROJECT_MENU action={action_id} state=not_wired missing={}",
            missing.id
        );
        self.notify_not_wired(missing.text_key, action_id, cx);
    }

    /// 「打开工程」对话框里勾了「记住我的选择」后写回设置。真源字段：
    /// `openFoldersInNewWindow` / `askWhereToOpenProjects`。
    fn remember_open_destination(&mut self, open_in_new_window: bool, cx: &mut Context<Self>) {
        match lithe_gpui_settings::try_store(cx) {
            Some(store) => {
                store.update(cx, |store, cx| {
                    store.remember_project_open_destination(open_in_new_window, cx)
                });
                println!(
                    "S1_OPEN_PROJECT remember=true openFoldersInNewWindow={open_in_new_window} askWhereToOpenProjects=false"
                );
            }
            None => eprintln!("S1_OPEN_PROJECT remember=skipped reason=no_settings_store"),
        }
    }

    fn rebuild_project_window(
        &mut self,
        target: PathBuf,
        remember: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        eprintln!(
            "S1_OPEN_PROJECT destination=this-window path={} state=rebuilding",
            target.display()
        );
        let started = std::time::Instant::now();
        let old_root = self.root.display().to_string();
        self.search_index.schedule_invalidate(cx);
        eprintln!(
            "S1_OPEN_PROJECT phase=search_invalidate ms={}",
            started.elapsed().as_millis()
        );
        self.flush_session(true, cx);
        eprintln!(
            "S1_OPEN_PROJECT phase=flush_session ms={}",
            started.elapsed().as_millis()
        );
        let Some(workspace) = replace_root_with_workspace(&target, window, cx) else {
            eprintln!("S1_OPEN_PROJECT state=rebuild_failed reason=no_view");
            return;
        };
        // 这一段以前是 14 秒的整体黑箱：换根会**同步**拆掉旧工作区的全部子系统
        // （编辑器缓冲、资源树、终端、git、改动视图、搜索索引、语言服务），耗时都在这里。
        eprintln!(
            "S1_OPEN_PROJECT phase=replace_root ms={}",
            started.elapsed().as_millis()
        );
        // 原地换根之后，这个窗口从此代表新工程：归属表必须跟着换，否则旧工程还以为自己
        // 有一个窗口（再打开旧工程就会聚焦到这个已经换了内容的窗口上）。
        register_project_window(&target, window.window_handle());

        let _ = workspace.update(cx, |shell, cx| {
            shell.record_opened_project(&target, Some(false), cx)
        });

        if remember {
            self.remember_open_destination(false, cx);
        }
        println!(
            "S1_OPEN_PROJECT state=opened path={} from={old_root}",
            target.display()
        );
    }

    fn open_project_where_dialog(
        &mut self,
        target: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let shell = cx.entity().downgrade();
        let do_not_ask = self.project_open_do_not_ask.clone();
        let name = lithe_gpui_settings::recent_project_name(&target.to_string_lossy());
        let body = tr_args("lithe.projectOpen.where", &[("project", &name)]);
        eprintln!(
            "S1_OPEN_PROJECT state=dialog path={} project={name}",
            target.display()
        );

        window.open_dialog(
            "lithe-project-open-where",
            move |id, _window, cx| {
                let checkbox_cell = do_not_ask.clone();
                let checkbox_shell = shell.clone();
                let click_cell = do_not_ask.clone();
                let new_window_cell = do_not_ask.clone();
                let cancel_label = tr("lithe.projectOpen.cancel");
                let new_window_label = tr("lithe.projectOpen.newWindow");
                let this_window_label = tr("lithe.projectOpen.thisWindow");
                let new_window_shell = shell.clone();
                let this_window_shell = shell.clone();
                let this_window_target = target.clone();
                let new_window_target = target.clone();
                let (cancel_id, new_window_id, this_window_id) = (
                    (id.clone(), "cancel"),
                    (id.clone(), "new-window"),
                    (id.clone(), "this-window"),
                );

                Dialog::new(id)
                    .title(tr("lithe.projectOpen.title"))
                    .on_close(dismiss_hosted)
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().colors.fg_muted)
                            .child(body.clone()),
                    )
                    .child(
                        Checkbox::new("lithe-project-open-do-not-ask")
                            .label(tr("lithe.projectOpen.doNotAskAgain"))
                            .checked(do_not_ask.get())
                            .on_change(move |value, _window, cx| {
                                checkbox_cell.set(value);
                                let _ = checkbox_shell.update(cx, |_shell, cx| cx.notify());
                            }),
                    )
                    .action(move |close| {
                        Button::new(cancel_id)
                            .label(cancel_label)
                            .on_click(move |_, window, cx| {
                                eprintln!("S1_OPEN_PROJECT state=cancelled reason=dialog-cancel");
                                close(window, cx);
                            })
                    })
                    .action(move |close| {
                        let new_window_shell = new_window_shell.clone();
                        let new_window_target = new_window_target.clone();
                        Button::new(new_window_id)
                            .primary()
                            .label(new_window_label)
                            .on_click(
                            move |_, window, cx| {
                                let remember = new_window_cell.get();
                                close(window, cx);
                                let _ = new_window_shell.update(cx, |shell, cx| {
                                    shell.execute_project_open(
                                        new_window_target.clone(),
                                        ProjectOpenDecision {
                                            destination: OpenDestination::NewWindow,
                                            remember,
                                        },
                                        window,
                                        cx,
                                    )
                                });
                            },
                        )
                    })
                    .action(move |close| {
                        let this_window_shell = this_window_shell.clone();
                        let this_window_target = this_window_target.clone();
                        Button::new(this_window_id)
                            .label(this_window_label)
                            .on_click(move |_, window, cx| {
                                let remember = click_cell.get();
                                close(window, cx);
                                let _ = this_window_shell.update(cx, |shell, cx| {
                                    shell.execute_project_open(
                                        this_window_target.clone(),
                                        ProjectOpenDecision {
                                            destination: OpenDestination::ThisWindow,
                                            remember,
                                        },
                                        window,
                                        cx,
                                    )
                                });
                            })
                    })
                    .into_any_element()
            },
            cx,
        );
    }

    pub(crate) fn record_opened_project(
        &mut self,
        target: &Path,
        open_in_new_window: Option<bool>,
        cx: &mut Context<Self>,
    ) {
        let path_text = target.to_string_lossy().to_string();
        if !self.recent_projects.record_open(
            &path_text,
            lithe_gpui_settings::now_unix_ms(),
            open_in_new_window,
        ) {
            eprintln!("S1_SETTINGS_RECENT record=skipped path={path_text} reason=empty_path");
            return;
        }
        println!(
            "S1_SETTINGS_RECENT record=ok path={path_text} count={}",
            self.recent_projects.len()
        );
        self.save_recent_projects();
        cx.notify();
    }

    fn save_recent_projects(&self) {
        let Some(path) = self.recent_projects_path.as_deref() else {
            eprintln!("S1_SETTINGS_RECENT save=skipped reason=no_path");
            return;
        };
        match lithe_gpui_settings::save_recent_projects(path, &self.recent_projects) {
            Ok(bytes) => println!(
                "S1_SETTINGS_RECENT saved path={} bytes={bytes} count={}",
                path.display(),
                self.recent_projects.len()
            ),
            Err(error) => eprintln!(
                "S1_SETTINGS_RECENT save_failed path={} error={error}",
                path.display()
            ),
        }
    }

    pub(crate) fn notify(&mut self, input: lithe_gpui_notify::Input, cx: &mut Context<Self>) {
        let code = input.code().to_string();
        let severity = input.severity();
        let outcome = self.notifications.update(cx, |store, _| {
            store.record(input, crate::notifications::now_ms())
        });
        match outcome {
            lithe_gpui_notify::RecordOutcome::Inserted => eprintln!(
                "S1_NOTIFICATION action=record code={code} severity={severity:?} state=inserted"
            ),
            lithe_gpui_notify::RecordOutcome::Replaced { occurrences } => eprintln!(
                "S1_NOTIFICATION action=record code={code} severity={severity:?} state=replaced occurrences={occurrences}"
            ),
        }
        cx.notify();
    }

    pub(crate) fn notify_not_wired(
        &mut self,
        code: &'static str,
        item: &str,
        cx: &mut Context<Self>,
    ) {
        self.notify(
            lithe_gpui_notify::Input::new(code, lithe_gpui_notify::Severity::Info)
                .param("item", item)
                .key("not-wired"),
            cx,
        );
    }

    pub fn open_project_probe(
        &mut self,
        target: PathBuf,
        explicit: Option<OpenDestination>,
        remember: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        eprintln!(
            "S1_OPEN_PROJECT_PROBE path={} explicit={:?} remember={remember}",
            target.display(),
            explicit
        );
        self.project_open_do_not_ask.set(remember);
        match explicit {
            Some(destination) => self.execute_project_open(
                target,
                ProjectOpenDecision {
                    destination,
                    remember,
                },
                window,
                cx,
            ),
            None => self.request_open_project(target, None, window, cx),
        }
    }
}

gpui::actions!(lithe_workbench, [OpenProjectFolder]);

pub fn install_open_project_action(cx: &mut App) {
    // `secondary-o` = macOS ⌘O / 其他 Ctrl+O，与 gpui 的平台归一化口径一致；旧写的 `ctrl-o`
    // 在 macOS 上错位（F2）。
    cx.bind_keys([KeyBinding::new("secondary-o", OpenProjectFolder, None)]);
    cx.on_action(|_: &OpenProjectFolder, cx: &mut App| {
        let bar = crate::menu_bar::try_handle();
        let Some(bar) = bar else {
            eprintln!("S1_MENU_KEY id=lithe.menu.openFolder source=secondary-o state=no_menu_bar");
            return;
        };
        let _ = bar.update(cx, |bar, cx| bar.push_run(MenuAction::OpenFolder, cx));
        eprintln!("S1_MENU_KEY id=lithe.menu.openFolder source=secondary-o state=pushed");
    });
}

/// F3 / Shift+F3 arrive as global listener callbacks (they carry no `MenuAction`): find the shell's
/// open editor and step its find bar. With no live shell there is nothing to step — the find bar
/// never opened, and `find_step` reports the closed case itself (B3-14).
pub fn route_find_step(forward: bool, cx: &mut App) {
    let Some(shell) = crate::command_palette::shell().and_then(|shell| shell.upgrade()) else {
        eprintln!("S1_EDITOR_FIND step={forward} result=no-shell");
        return;
    };
    let Some(window) = cx.active_window() else {
        eprintln!("S1_EDITOR_FIND step={forward} result=no-window");
        return;
    };
    let _ = window.update(cx, |_view, window, cx| {
        shell.update(cx, |shell, cx| {
            shell
                .editor
                .update(cx, |pane, cx| pane.find_step(forward, window, cx));
        });
    });
}

/// The syntactic half of the name rules, shared by the shell's own checks (the tree carries its
/// own copy because it must answer synchronously while typing, inside the component library).
/// `None` means the name is at least shaped like a legal child.
///
/// `trim()` 只用来判「是不是空的」：名字**原样**参与其余规则 —— 尾随空格 / 尾随点是真源里
/// 「名称非法」的判据（交互规范 §1.6 规则表），先 trim 会把它们抹掉，永远拦不住。
fn invalid_name_key(name: &str) -> Option<&'static str> {
    if name.trim().is_empty() {
        return Some("lithe.files.invalidFileName");
    }
    if name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name.ends_with('.')
        || name.ends_with(' ')
        || lithe_gpui_explorer::is_windows_reserved(name)
    {
        return Some("lithe.files.invalidFileNameCharacters");
    }
    None
}

/// `PromptDialog::check` for the tree's New File / New Folder: the name must be shaped legally and
/// must not already exist in `parent`. The reason comes back to be shown under the field.
fn check_new_name(parent: &Path, name: &str, is_dir: bool) -> Result<(), SharedString> {
    // 名字**原样**进校验（`invalid_name_key` 内部只对判空用 trim）：与行内改名同源，
    // 尾随空格 / 尾随点都必须被拒。
    if let Some(key) = invalid_name_key(name) {
        return Err(tr(key));
    }
    if parent.join(name).exists() {
        let key = if is_dir {
            "lithe.fileExplorer.folderAlreadyExistsMessage"
        } else {
            "lithe.fileExplorer.fileAlreadyExistsMessage"
        };
        return Err(tr(key));
    }
    Ok(())
}

impl Drop for ShellWorkspace {
    fn drop(&mut self) {
        eprintln!("S1_WORKSPACE_DROP root={}", self.root.display());
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RightScanState {
    maven: bool,
    spring: bool,
}

impl RightScanState {
    fn request(&mut self, view: RightToolWindowView, visible: bool) -> Option<RightToolWindowView> {
        if !visible {
            return None;
        }
        match view {
            RightToolWindowView::Maven if !self.maven => {
                self.maven = true;
                Some(view)
            }
            RightToolWindowView::Spring if !self.spring => {
                self.spring = true;
                Some(view)
            }
            _ => None,
        }
    }
}

fn right_scan_should_notify(
    target: RightToolWindowView,
    current: RightToolWindowView,
    visible: bool,
) -> bool {
    visible && current == target
}

impl ShellWorkspace {
    /// 菜单启用判定的那一份状态快照（见 [`crate::menu_bar::EditorContext`]）。
    ///
    /// **必须只读**：调用点在 `Render::render` 里，那期间外壳正被更新着，再去 `update`
    /// 编辑窗格会撞 gpui 的借用 panic（`cannot update X while it is already being
    /// updated`）。五个问句全是 `EditorPane` 上的 `&self` 纯读算式。
    ///
    /// 每次重画都现取一份，不做缓存：外壳已经 observe 了编辑窗格
    /// （`ShellWorkspace::new` 里的 `editor_subscription`），标签页增删改躲不开一次重画。
    fn menu_context(&self, cx: &App) -> EditorContext {
        let pane = self.editor.read(cx);
        EditorContext {
            has_active_buffer: pane.active_tab_index().is_some(),
            tab_count: pane.tab_count(),
            active_tab_index: pane.active_tab_index(),
            has_active_disk_file: pane.has_active_disk_file(),
            // `active_buffer_is_readonly` 在没有活动缓冲时返回 `true`，取反后自然是
            // `false`，与 `EditorContext` 的第 3 条不变量一致。
            active_writable: !pane.active_buffer_is_readonly(),
            can_go_back: pane.can_go_back(),
            can_go_forward: pane.can_go_forward(),
            has_closed_tabs: pane.has_closed_tabs(),
        }
    }
}

impl Render for ShellWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The dock's own state is derived: bring it in line with the shell's
        // flags before anything is drawn, so no other path has to remember to.
        self.sync_dock(cx);
        let menu = crate::menu_bar::render_bar(&self.menu_bar, self.menu_context(cx), window, cx);
        let project_entries = self.project_entries();
        let project_menu = render_project_menu(
            &self.project_menu,
            &project_entries,
            self.recent_projects.entries(),
            cx,
        );
        let branch_item =
            branch_trigger(&self.branch_panel, cx).unwrap_or_else(|| div().into_any_element());
        let probe = self.branch_panel_probe;
        self.branch_panel_probe = false;
        if probe {
            open_branch_panel(&self.branch_panel, window, cx);
        }
        for request in crate::menu_bar::take_pending_runs(cx) {
            match request {
                MenuRequest::Run(action) => self.apply_menu_action(action, window, cx),
                MenuRequest::NotWired { label_key, missing } => {
                    self.report_menu_not_wired(label_key, missing, cx)
                }
            }
        }

        let show_status_bar = lithe_gpui_settings::try_store(cx)
            .map(|store| store.read(cx).settings().show_status_bar)
            .unwrap_or(true);

        let handle = cx.entity();
        let on_activate = {
            let handle = handle.clone();
            move |index: usize, _window: &mut Window, cx: &mut App| {
                handle.update(cx, |this, cx| {
                    this.active_project = Some(index);
                    this.project_tabs_scroll.scroll_to_item(index);
                    cx.notify();
                });
            }
        };
        let on_close = {
            let handle = handle.clone();
            move |index: usize, _window: &mut Window, cx: &mut App| {
                handle.update(cx, |this, cx| {
                    if this.active_project == Some(index) {
                        this.active_project = this.projects.len().checked_sub(2);
                        cx.notify();
                    }
                });
            }
        };
        let terminal_handle = self.terminal.downgrade();
        let on_select_activity = {
            let handle = handle.clone();
            move |index: usize, _event: &ClickEvent, window: &mut Window, cx: &mut App| {
                if index == SETTINGS_ACTIVITY_IX {
                    lithe_gpui_settings::open_settings_dialog(window, cx);
                    return;
                }
                let ensure_terminal = handle.update(cx, |this, cx| {
                    match bottom_pane_for(index) {
                        Some(kind) => {
                            if this.bottom_visible && this.bottom_kind == kind {
                                this.bottom_visible = false;
                            } else {
                                this.bottom_kind = kind;
                                this.bottom_visible = true;
                            }
                        }
                        None => this.select_top_activity(index, true, cx),
                    }
                    this.schedule_session_save(cx);
                    this.refresh_problems_if_visible(cx);
                    if index == SEARCH_ACTIVITY_IX
                        && this.top_activity_view == Some(SEARCH_ACTIVITY_IX)
                        && this.left_pane_visible
                    {
                        let search = this.search.clone();
                        search.update(cx, |panel, cx| panel.activate(window, cx));
                    }
                    cx.notify();
                    this.bottom_visible && this.bottom_kind == BottomPaneKind::Terminal
                });
                if ensure_terminal {
                    let _ = terminal_handle.update(cx, |pane, cx| pane.ensure_session(window, cx));
                }
            }
        };

        let on_select_right_activity = {
            let handle = handle.clone();
            move |index: usize, event: &ClickEvent, window: &mut Window, cx: &mut App| {
                let point = event.mouse_position().map(ProbePoint::from);
                if index == EXTENSIONS_RAIL_IX {
                    handle.update(cx, |this, cx| {
                        this.toggle_extensions_page(point, window, cx);
                    });
                    return;
                }
                let Some(clicked) = RightToolWindowView::from_rail_index(index) else {
                    return;
                };
                handle.update(cx, |this, cx| {
                    let (view, visible) =
                        resolve_right_click(clicked, this.right_view, this.right_visible);
                    this.right_view = view;
                    this.right_visible = visible;
                    if view == RightToolWindowView::Notifications && visible {
                        this.note_notifications_opened(cx);
                    }
                    this.schedule_right_view_scan(view, visible, cx);
                    diagnose_right_panel(view, visible, point);
                    this.schedule_session_save(cx);
                    cx.notify();
                });
            }
        };

        let activity_flags: Vec<bool> = (0..self.activity_items.len())
            .map(|index| self.is_activity_active(index))
            .collect();
        let right_activity_flags: Vec<bool> = (0..self.right_activity_items.len())
            .map(|index| self.is_right_activity_active(index))
            .collect();
        let left_rail = activity_bar(
            ActivitySide::Left,
            &self.activity_items,
            move |index| activity_flags.get(index).copied().unwrap_or(false),
            on_select_activity,
            window,
            cx,
        );
        let right_activity_items = right_activity_items(&self.notifications, cx);
        let right_rail = activity_bar(
            ActivitySide::Right,
            &right_activity_items,
            move |index| right_activity_flags.get(index).copied().unwrap_or(false),
            on_select_right_activity,
            window,
            cx,
        );

        // The middle column and the tool windows are drawn by the dock, from the
        // shell's own state (`center_content` / `panel_content`).
        let left_sidebar_visible = self.left_sidebar_visible;
        let root = v_stack()
            .size_full()
            .bg(cx.theme().colors.bg)
            .text_color(cx.theme().colors.fg)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &SaveBuffer, window, cx| {
                let _ = this
                    .editor
                    .update(cx, |pane, cx| pane.save_active(window, cx));
            }))
            // 标题栏由组件库画（含平台相关的窗口键与拖拽），这里只递内容。菜单那一段摆在哪
            // 一层取决于当前档位：Above / AutoHide 是标题栏**上方**的独立一行。
            .child(title_bar(
                menu.unwrap_or_else(|| div().into_any_element()),
                project_menu.unwrap_or_else(|| div().into_any_element()),
                branch_item,
                self.run.clone(),
                self.menu_bar.read(cx).mode(),
                cx,
            ))
            .children((self.projects.len() > 1).then(|| {
                project_tabs(
                    &self.projects,
                    self.active_project,
                    &self.project_tabs_scroll,
                    on_activate,
                    on_close,
                    window,
                    cx,
                )
            }))
            .child(
                h_stack()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .gap_1()
                    .pr_1()
                    .children(left_sidebar_visible.then_some(left_rail))
                    // `h_stack()` is `align-items: center`, so a child is not stretched
                    // to the row's height: the dock needs an explicit `h_full()`, or its
                    // `size_full()` root resolves against an indefinite height and the
                    // whole layout — editor included — collapses to nothing.
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .min_w_0()
                            .min_h_0()
                            .child(self.dock.clone()),
                    )
                    .child(right_rail),
            )
            .children(show_status_bar.then(|| {
                let left = self.footer_left(cx);
                let right = self.footer_right(cx);
                status_bar(&left, &right, window, cx)
            }));

        root
    }
}

/// The three tool windows the dock wraps, by panel id.
const PANEL_LEFT: &str = "left";
const PANEL_RIGHT: &str = "right";
const PANEL_BOTTOM: &str = "bottom";

/// Which tool windows the shell wants open, from its own flags. The left pane
/// has two switches — the sidebar itself and whether a view is chosen for it —
/// and the Git tool window hides itself while it has nothing to show.
fn wanted_panels(
    left_sidebar_visible: bool,
    left_pane_visible: bool,
    right_visible: bool,
    bottom_visible: bool,
    git_self_hidden: bool,
) -> [(&'static str, bool); 3] {
    [
        (PANEL_LEFT, left_sidebar_visible && left_pane_visible),
        (PANEL_RIGHT, right_visible),
        (PANEL_BOTTOM, bottom_visible && !git_self_hidden),
    ]
}

fn side_id(side: DockSide) -> &'static str {
    match side {
        DockSide::Left => PANEL_LEFT,
        DockSide::Right => PANEL_RIGHT,
        DockSide::Bottom => PANEL_BOTTOM,
        DockSide::Top => "top",
    }
}

/// The dock the shell lives in: a tool window on the left, the editor in the
/// middle, one on the right and one below. Every panel is `plain` — the shell's
/// own chrome (activity bar, panel headers) already names them, so the dock
/// draws no tab strip and the divider is the only handle.
fn dock_with_panels(
    center: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    content: impl Fn(&SharedString, &mut Window, &mut App) -> AnyElement + 'static,
    cx: &mut Context<Dock>,
) -> Dock {
    let mut dock = Dock::new(center, content, cx);
    dock.add(DockPanel::plain(PANEL_LEFT, "Project"), DockSide::Left);
    dock.add(
        DockPanel::plain(PANEL_RIGHT, "Tool Window"),
        DockSide::Right,
    );
    dock.add(DockPanel::plain(PANEL_BOTTOM, "Terminal"), DockSide::Bottom);
    // The ranges the macOS specification lays down, as of 2026-10-03 (see
    // `UI-MAP.md`): each side keeps what the center must not lose ("reserve"),
    // and the left and right also have a fixed ceiling.
    for (side, min, max, reserve) in [
        (
            DockSide::Left,
            SIDEBAR_MIN_WIDTH_SPEC,
            Some(SIDEBAR_MAX_WIDTH_SPEC),
            Some(SIDEBAR_MIN_EDITOR_WIDTH_SPEC),
        ),
        (
            DockSide::Right,
            RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC,
            Some(RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC),
            Some(RIGHT_TOOL_WINDOW_MIN_WORKSPACE_WIDTH_SPEC),
        ),
        (
            DockSide::Bottom,
            BOTTOM_PANE_MIN_HEIGHT_SPEC,
            None,
            Some(BOTTOM_PANE_MIN_CENTER_HEIGHT_SPEC),
        ),
    ] {
        dock.set_side_limits_with(
            side,
            rems(min / 16.),
            max.map(|width| rems(width / 16.)),
            reserve.map(|width| rems(width / 16.)),
            cx,
        );
    }
    dock
}

fn editor_island(content: impl IntoElement, cx: &App) -> impl IntoElement {
    div()
        .relative()
        .w_full()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .border_1()
        .border_color(cx.theme().colors.border)
        .bg(cx.theme().colors.bg)
        .child(content)
}

fn activity_items() -> Vec<ActivityItem> {
    vec![
        ActivityItem::idea(&idea::FILES_ICON, tr("lithe.workbench.project")),
        ActivityItem::idea(&idea::GIT_BRANCH_ICON, tr("lithe.workbench.changes")),
        ActivityItem::idea(&idea::MAGNIFYING_GLASS_ICON, tr("lithe.workbench.search")),
        ActivityItem::idea(&idea::PLAY_ICON, tr("lithe.workbench.run")).bottom(true),
        ActivityItem::idea(&idea::TERMINAL_ICON, tr("lithe.workbench.terminal")).bottom(true),
        ActivityItem::idea(
            &idea::WARNING_CIRCLE_ICON,
            tr("lithe.workbench.diagnostics"),
        )
        .bottom(true),
        ActivityItem::idea(&idea::VCS_ICON, tr("lithe.workbench.gitLog")).bottom(true),
        ActivityItem::idea(&idea::GEAR_ICON, tr("lithe.workbench.settings")).bottom(true),
    ]
}

fn notification_badge_color(store: &Entity<lithe_gpui_notify::Store>, cx: &App) -> Option<Hsla> {
    match store.read(cx).badge() {
        lithe_gpui_notify::Badge::None => None,
        lithe_gpui_notify::Badge::Info => Some(cx.theme().colors.info),
        lithe_gpui_notify::Badge::Error => Some(cx.theme().colors.danger),
    }
}

fn right_activity_items(store: &Entity<lithe_gpui_notify::Store>, cx: &App) -> Vec<ActivityItem> {
    let bell = {
        let item = ActivityItem::idea(&idea::BELL_ICON, tr("lithe.notifications.title"));
        match notification_badge_color(store, cx) {
            Some(color) => item.badge(color),
            None => item,
        }
    };
    vec![
        ActivityItem::idea(&idea::PLUGIN_ICON, tr("lithe.extensions.title")),
        bell,
        ActivityItem::idea(&idea::MAVEN_ICON, tr("lithe.workbench.maven")),
    ]
}

fn java_toolchain_override_from(
    paths: &ToolchainPaths,
) -> Option<lithe_gpui_java::JavaToolchainOverride> {
    let trimmed = paths.java_home_path.trim();
    (!trimmed.is_empty()).then(|| lithe_gpui_java::JavaToolchainOverride {
        java_home: PathBuf::from(trimmed),
    })
}

fn maven_settings_override_from(paths: &ToolchainPaths) -> Option<PathBuf> {
    let trimmed = paths.maven_settings_path.trim();
    (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
}

fn terminal_font_size_override(value: f64) -> Option<f32> {
    (value > 0.0).then_some(value as f32)
}

fn prepare_workspace_config(root: PathBuf, cx: &mut Context<ShellWorkspace>) {
    cx.spawn(async move |this, cx| {
        let task_root = root.clone();
        let (identity, settings_tracked) = cx
            .background_spawn(async move {
                let identity = lithe_gpui_shared::workspace_config::resolve_project_id(&task_root);
                let tracked = workspace_settings_tracked(&task_root);
                (identity, tracked)
            })
            .await;

        let _ = this.update(cx, |_shell, cx| {
            if let Some(store) = lithe_gpui_settings::try_store(cx) {
                store.update(cx, |store, cx| {
                    store.set_workspace_settings_tracked(Some(settings_tracked), cx)
                });
            }
        });

        match identity {
            Ok(identity) => println!(
                "S1_WORKSPACE_CONFIG shell_identity root={} id={} source={}",
                root.display(),
                identity.value(),
                if identity.is_manifest_backed() {
                    "manifest"
                } else {
                    "path"
                }
            ),
            Err(error) => {
                eprintln!(
                    "S1_WORKSPACE_CONFIG shell_identity_failed root={} error={error}",
                    root.display()
                );
                let reason = error.to_string();
                let _ = this.update(cx, |shell, cx| {
                    shell.notify(
                        lithe_gpui_notify::Input::new(
                            "lithe.gpui.workspaceConfigFailed",
                            lithe_gpui_notify::Severity::Error,
                        )
                        .param("reason", reason)
                        .key("workspace-config"),
                        cx,
                    );
                });
            }
        }
    })
    .detach();
}

fn workspace_settings_tracked(root: &std::path::Path) -> bool {
    let paths = lithe_gpui_shared::workspace_config::WorkspaceConfigPaths::new(root);
    let Some(relative) =
        lithe_gpui_shared::workspace_config::workspace_relative(root, &paths.settings())
    else {
        return false;
    };
    let payload = serde_json::json!({
        "root": root.display().to_string(),
        "arguments": ["ls-files", "--error-unmatch", "--", relative],
    });
    match lithe_gpui_shared::core_json("git.command", payload) {
        Ok(Some(data)) => {
            let tracked = data.get("exitCode").and_then(serde_json::Value::as_i64) == Some(0);
            println!(
                "S1_WORKSPACE_CONFIG settings_tracked root={} tracked={} exit_code={}",
                root.display(),
                tracked,
                data.get("exitCode")
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "(none)".to_string())
            );
            tracked
        }
        Ok(None) => false,
        Err(error) => {
            eprintln!(
                "S1_WORKSPACE_CONFIG settings_tracked_unknown root={} error={error}",
                root.display()
            );
            false
        }
    }
}

fn register_java_toolchain(root: &Path, cx: &App) {
    let global = lithe_gpui_settings::try_store(cx)
        .map(|store| Overrides::from_settings(store.read(cx).settings()))
        .unwrap_or_default();
    let local = load_local_toolchain(root);
    for diagnostic in &local.diagnostics {
        eprintln!("S1_SETTINGS wiring=java_toolchain local_diagnostic {diagnostic}");
    }
    let (resolved, origins) = resolve_overrides(&local.paths, &global);
    let paths = resolved.to_local();

    let overridden = java_toolchain_override_from(&paths);
    let maven_settings = maven_settings_override_from(&paths);
    println!(
        "S1_SETTINGS wiring=java_toolchain java_home_path={} maven_settings_path={} {}",
        overridden
            .as_ref()
            .map(|overridden| overridden.java_home.display().to_string())
            .unwrap_or_else(|| "-".to_string()),
        maven_settings
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "-".to_string()),
        ToolchainPaths::diagnostic_suffix(&origins)
    );
    lithe_gpui_java::set_java_toolchain_override(overridden);
    lithe_gpui_java::set_maven_settings_override(maven_settings);
}

#[cfg(test)]
mod tests {
    use gpui::prelude::*;
    use gpui::{TestAppContext, div, rems};
    use lithe_gpui_shared::tr;
    use lithe_ui::layout::DockSide;

    use super::{
        ActionFlags, BOTTOM_PANE_HEIGHT_SPEC, BOTTOM_PANE_MIN_CENTER_HEIGHT_SPEC,
        BOTTOM_PANE_MIN_HEIGHT_SPEC, COMMAND_ORDER, CommandId, OpenDestination, Overrides,
        PANEL_BOTTOM, PANEL_LEFT, PANEL_RIGHT, ProjectOpenDecision,
        RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC, RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC,
        RIGHT_TOOL_WINDOW_MIN_WORKSPACE_WIDTH_SPEC, RIGHT_TOOL_WINDOW_WIDTH_SPEC, RightScanState,
        RightToolWindowView, SIDEBAR_MAX_WIDTH_SPEC, SIDEBAR_MIN_EDITOR_WIDTH_SPEC,
        SIDEBAR_MIN_WIDTH_SPEC, ToolchainPaths, check_new_name, dock_with_panels, invalid_name_key,
        left_activity_index, resolve_overrides, resolve_project_open_destination,
        right_scan_should_notify, visible_commands, wanted_panels,
    };
    use std::path::{Path, PathBuf};

    /// 窗口尺寸规格仍写在宿主侧，但上下限现在由 dock 持有：三块工具窗的
    /// 区间必须与 macOS 真源一致（见 `UI-MAP.md`）。
    #[gpui::test]
    fn the_dock_carries_the_spec_ranges_for_each_tool_window(cx: &mut TestAppContext) {
        cx.update(|cx| {
            lithe_ui::theme::Theme::init(cx);
            let dock = cx.new(|cx| {
                dock_with_panels(
                    |_, _| div().into_any_element(),
                    |_, _, _| div().into_any_element(),
                    cx,
                )
            });
            dock.read_with(cx, |dock, _| {
                // 三块都在，且各自在自己的边上（`is_hidden` 对未知 id 会 panic）。
                assert!(!dock.is_hidden(PANEL_LEFT));
                assert!(!dock.is_hidden(PANEL_RIGHT));
                assert!(!dock.is_hidden(PANEL_BOTTOM));

                let left = dock.side_limits(DockSide::Left);
                assert_eq!(left.min, rems(SIDEBAR_MIN_WIDTH_SPEC / 16.));
                assert_eq!(left.max, Some(rems(SIDEBAR_MAX_WIDTH_SPEC / 16.)));
                assert_eq!(
                    left.reserve,
                    Some(rems(SIDEBAR_MIN_EDITOR_WIDTH_SPEC / 16.)),
                    "左栏必须给编辑区留 400"
                );

                let right = dock.side_limits(DockSide::Right);
                assert_eq!(right.min, rems(RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC / 16.));
                assert_eq!(
                    right.max,
                    Some(rems(RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC / 16.))
                );
                assert_eq!(
                    right.reserve,
                    Some(rems(RIGHT_TOOL_WINDOW_MIN_WORKSPACE_WIDTH_SPEC / 16.)),
                    "右栏必须给中央列留 640"
                );

                let bottom = dock.side_limits(DockSide::Bottom);
                assert_eq!(bottom.min, rems(BOTTOM_PANE_MIN_HEIGHT_SPEC / 16.));
                assert_eq!(bottom.max, None, "底部没有固定上界，只有保留");
                assert_eq!(
                    bottom.reserve,
                    Some(rems(BOTTOM_PANE_MIN_CENTER_HEIGHT_SPEC / 16.)),
                    "底部必须给上方留 220"
                );
            });
        });
    }

    /// 恢复会话时宽度经 dock 夹取：超出上界的存量值落到上界，非法值落到下界。
    #[gpui::test]
    fn a_stored_width_is_clamped_through_the_dock(cx: &mut TestAppContext) {
        cx.update(|cx| {
            lithe_ui::theme::Theme::init(cx);
            let dock = cx.new(|cx| {
                dock_with_panels(
                    |_, _| div().into_any_element(),
                    |_, _, _| div().into_any_element(),
                    cx,
                )
            });
            dock.update(cx, |dock, cx| {
                dock.set_size(DockSide::Left, rems(9999.), cx);
                assert_eq!(
                    dock.size(DockSide::Left),
                    rems(SIDEBAR_MAX_WIDTH_SPEC / 16.)
                );
                dock.set_size(DockSide::Left, rems(f32::NAN), cx);
                assert_eq!(
                    dock.size(DockSide::Left),
                    rems(SIDEBAR_MIN_WIDTH_SPEC / 16.)
                );
            });
        });
    }

    /// 默认宽度就是真源值，且底部默认与下界同值（钳制在默认值上恒等）。
    #[test]
    fn the_spec_defaults_still_hold() {
        assert_eq!(RIGHT_TOOL_WINDOW_WIDTH_SPEC, 360.);
        assert_eq!(BOTTOM_PANE_HEIGHT_SPEC, BOTTOM_PANE_MIN_HEIGHT_SPEC);
    }

    /// 可见性是外壳说了算：三个开关与 Git 面板的自隐藏合成出 dock 的面板开关。
    #[test]
    fn visibility_flags_combine_into_the_docks_panels() {
        let open = |left: bool, left_pane: bool, right: bool, bottom: bool, git_hidden: bool| {
            let mut wanted = wanted_panels(left, left_pane, right, bottom, git_hidden);
            wanted.sort_by_key(|(id, _)| *id);
            wanted
        };
        assert_eq!(
            open(true, true, true, true, false),
            [
                (PANEL_BOTTOM, true),
                (PANEL_LEFT, true),
                (PANEL_RIGHT, true)
            ]
        );
        assert_eq!(
            open(false, true, false, false, false),
            [
                (PANEL_BOTTOM, false),
                (PANEL_LEFT, false),
                (PANEL_RIGHT, false)
            ],
            "左栏收起、右栏与底部关闭"
        );
        assert_eq!(
            open(true, false, true, true, true),
            [
                (PANEL_BOTTOM, false),
                (PANEL_LEFT, false),
                (PANEL_RIGHT, true)
            ],
            "搜索页没开时左侧面板也收着；Git 面板自隐藏时底部跟着收"
        );
    }

    #[test]
    fn menu_key_actions_are_parseable_and_listed() {
        use crate::menu_bar::{MENUS, MenuAction, MenuItem};

        let listed: Vec<MenuAction> = MENUS
            .iter()
            .flat_map(|menu| menu.items.iter())
            .filter_map(|item| match item {
                MenuItem::Action(action) => Some(*action),
                _ => None,
            })
            .collect();

        for action in [
            MenuAction::ReopenClosedTab,
            MenuAction::ToggleActivitySidebar,
            MenuAction::ToggleTerminal,
            MenuAction::ZoomIn,
            MenuAction::ZoomOut,
            MenuAction::ResetZoom,
            MenuAction::GoToLine,
        ] {
            let key = action
                .menu_key()
                .unwrap_or_else(|| panic!("{action:?} 没有键位字面量"));
            gpui::Keystroke::parse(key)
                .unwrap_or_else(|error| panic!("{action:?} 的键位 {key:?} 解析不了：{error}"));
            assert!(listed.contains(&action), "{action:?} 没有画在任何菜单里");
            assert!(
                action.keybind_action().is_some(),
                "{action:?} 有键位却没有 Action —— 菜单上会显示一颗按不出的键"
            );
        }

        // 与 `menu_bar` 的 `b2_actions_are_wired_and_only_bound_ones_carry_a_key` 同一条契约：
        // 菜单里带键帽的动作，顺序与键位必须与真源一致（改它就是改验证脚本）。
        let with_key: Vec<&'static str> = listed
            .iter()
            .filter_map(|action| action.menu_key())
            .collect();
        assert_eq!(
            with_key,
            vec![
                crate::menu_bar::KEY_REOPEN_CLOSED_TAB,
                crate::menu_bar::KEY_FIND,
                crate::menu_bar::KEY_FIND_REPLACE,
                "secondary-/",
                "secondary-shift-d",
                "secondary-y",
                "secondary-alt-shift-up",
                "secondary-alt-shift-down",
                crate::menu_bar::KEY_TOGGLE_ACTIVITY_SIDEBAR,
                crate::menu_bar::KEY_TOGGLE_TERMINAL,
                crate::menu_bar::KEY_ZOOM_IN,
                crate::menu_bar::KEY_ZOOM_OUT,
                crate::menu_bar::KEY_RESET_ZOOM,
                crate::menu_bar::KEY_GO_TO_LINE,
                crate::menu_bar::KEY_GO_BACK,
                crate::menu_bar::KEY_GO_FORWARD,
                crate::menu_bar::KEY_GO_TO_REFERENCES,
                "ctrl-tab",
                "ctrl-shift-tab",
            ]
        );
    }

    #[test]
    fn project_open_destination_follows_the_source_decision_table() {
        assert_eq!(
            resolve_project_open_destination(true, true, Some(OpenDestination::ThisWindow)),
            Some(ProjectOpenDecision {
                destination: OpenDestination::ThisWindow,
                remember: false,
            })
        );
        assert_eq!(
            resolve_project_open_destination(false, true, Some(OpenDestination::NewWindow)),
            Some(ProjectOpenDecision {
                destination: OpenDestination::NewWindow,
                remember: false,
            }),
            "显式目的地压过偏好"
        );

        assert_eq!(
            resolve_project_open_destination(false, false, None),
            Some(ProjectOpenDecision {
                destination: OpenDestination::ThisWindow,
                remember: false,
            })
        );
        assert_eq!(
            resolve_project_open_destination(false, true, None),
            Some(ProjectOpenDecision {
                destination: OpenDestination::NewWindow,
                remember: false,
            })
        );

        assert_eq!(resolve_project_open_destination(true, false, None), None);
        assert_eq!(resolve_project_open_destination(true, true, None), None);
    }

    #[test]
    fn project_window_key_normalizes_one_directory_to_one_identity() {
        // 真源 ProjectWindowRegistry::identity 的同一件事：同一个目录换个写法打开，
        // 必须落到同一个窗口上，否则「再打开同一个工程」会开出第二个。
        let root = std::env::temp_dir();
        let canonical = root.canonicalize().expect("临时目录应当能解析成规范路径");
        assert_eq!(super::project_window_key(&root), canonical);

        let detour = canonical.join("..").join(
            canonical
                .file_name()
                .expect("临时目录一定带最后一段目录名"),
        );
        assert_eq!(super::project_window_key(&detour), canonical);

        // 不存在的路径没有规范形式，退回原样：宁可错开一个键，也不要 panic。
        let missing = root.join("lithe-project-window-key-does-not-exist");
        assert_eq!(super::project_window_key(&missing), missing);
    }

    #[test]
    fn left_view_selectors_map_to_top_group_indices() {
        assert_eq!(left_activity_index("files"), Some(0));
        assert_eq!(left_activity_index("project"), Some(0));
        assert_eq!(left_activity_index("changes"), Some(1));
        assert_eq!(left_activity_index("search"), Some(2));
        assert_eq!(left_activity_index("terminal"), None);
        assert_eq!(left_activity_index("nope"), None);
    }

    #[test]
    fn identity_enums_map_both_ways() {
        use super::IntoGitIdentity as _;
        use lithe_gpui_settings::{IdentityField, IdentityScope};

        for scope in IdentityScope::ALL {
            let git = scope.to_git_identity();
            assert_eq!(git.id(), scope.id(), "作用域映射改变了线格式字面量");
            assert_eq!(IdentityScope::from_id(scope.id()), scope);
        }
        assert_eq!(IdentityScope::from_id("global"), IdentityScope::Global);
        assert_eq!(
            IdentityScope::from_id("anything-else"),
            IdentityScope::Local,
            "未知作用域必须退回 Core 的缺省值 local"
        );

        for field in [IdentityField::Name, IdentityField::Email] {
            let git = field.to_git_identity();
            assert_eq!(git.id(), field.id(), "字段映射改变了线格式字面量");
            assert_eq!(IdentityField::from_id(field.id()), field);
        }
        assert_eq!(IdentityField::from_id("email"), IdentityField::Email);
        assert_eq!(IdentityField::from_id("nope"), IdentityField::Name);
    }

    #[test]
    fn java_toolchain_override_follows_java_home_path() {
        let mut paths = ToolchainPaths::default();
        assert_eq!(
            super::java_toolchain_override_from(&paths),
            None,
            "默认的空串 = 没选覆盖值"
        );

        paths.java_home_path = "   ".to_string();
        assert_eq!(
            super::java_toolchain_override_from(&paths),
            None,
            "全空白同样等于没选（不能变成 `PathBuf::from(\"\")` 这种「当前目录」）"
        );

        paths.java_home_path = " C:\\tools\\jdk-21 ".to_string();
        let overridden =
            super::java_toolchain_override_from(&paths).expect("填了值就必须给出覆盖值");
        assert_eq!(
            overridden.java_home,
            std::path::PathBuf::from("C:\\tools\\jdk-21"),
            "前后空白必须去掉（带空白的路径会被 java 侧判成无效）"
        );
    }

    #[test]
    fn maven_settings_override_follows_its_own_key() {
        let mut paths = ToolchainPaths::default();
        assert_eq!(super::maven_settings_override_from(&paths), None);

        paths.maven_settings_path = "  \t".to_string();
        assert_eq!(
            super::maven_settings_override_from(&paths),
            None,
            "全空白等于没选（否则会拿空路径去当 settings.xml）"
        );

        paths.maven_settings_path = " D:\\ci\\settings.xml ".to_string();
        assert_eq!(
            super::maven_settings_override_from(&paths),
            Some(std::path::PathBuf::from("D:\\ci\\settings.xml")),
            "前后空白必须去掉"
        );
        assert_eq!(super::java_toolchain_override_from(&paths), None);
    }

    #[test]
    fn project_local_toolchain_wins_over_the_global_default_when_registering() {
        use lithe_gpui_shared::workspace_config::OverrideOrigin;

        let global = Overrides {
            java_home: "C:\\global\\jdk".to_string(),
            maven_settings: "C:\\global\\settings.xml".to_string(),
            ..Overrides::default()
        };
        let local = ToolchainPaths {
            java_home_path: "D:\\project\\jdk".to_string(),
            ..ToolchainPaths::default()
        };

        let (resolved, origins) = resolve_overrides(&local, &global);
        let paths = resolved.to_local();
        assert_eq!(
            super::java_toolchain_override_from(&paths)
                .expect("项目本机填了值")
                .java_home,
            std::path::PathBuf::from("D:\\project\\jdk"),
            "项目本机必须压过全局"
        );
        assert_eq!(
            super::maven_settings_override_from(&paths),
            Some(std::path::PathBuf::from("C:\\global\\settings.xml")),
            "项目本机没设的值回落到全局默认"
        );
        assert_eq!(origins.java_home_path, OverrideOrigin::Project);
        assert_eq!(origins.maven_settings_path, OverrideOrigin::Global);
        assert_eq!(origins.maven_executable_path, OverrideOrigin::Unset);
    }

    fn dark() -> ActionFlags {
        ActionFlags {
            terminal_shown: false,
            maven_shown: false,
            status_bar_shown: true,
            dark_theme: true,
        }
    }

    #[test]
    fn theme_actions_are_mutually_exclusive() {
        let on_dark = visible_commands(dark());
        assert!(on_dark.contains(&CommandId::SwitchToLightTheme));
        assert!(!on_dark.contains(&CommandId::SwitchToDarkTheme));

        let on_light = visible_commands(ActionFlags {
            dark_theme: false,
            ..dark()
        });
        assert!(on_light.contains(&CommandId::SwitchToDarkTheme));
        assert!(!on_light.contains(&CommandId::SwitchToLightTheme));
    }

    #[test]
    fn the_rest_of_the_table_is_always_present() {
        for flags in [
            dark(),
            ActionFlags {
                dark_theme: false,
                ..dark()
            },
        ] {
            let ids = visible_commands(flags);
            for id in COMMAND_ORDER {
                if matches!(
                    id,
                    CommandId::SwitchToLightTheme | CommandId::SwitchToDarkTheme
                ) {
                    continue;
                }
                assert!(ids.contains(&id), "{} 不该被过滤掉", id.id());
            }
        }
    }

    #[test]
    fn visible_order_follows_the_table_order() {
        let ids = visible_commands(dark());
        assert_eq!(ids.len(), COMMAND_ORDER.len() - 1, "主题只留一条");
        let mut rest = COMMAND_ORDER.iter();
        for id in &ids {
            assert!(
                rest.any(|candidate| candidate == id),
                "{} 不在 COMMAND_ORDER 的剩余部分里（顺序被打乱或重复）",
                id.id()
            );
        }
    }

    #[test]
    fn command_ids_are_stable_and_unique() {
        let all = [
            CommandId::OpenSettings,
            CommandId::OpenAppearanceSettings,
            CommandId::SwitchToLightTheme,
            CommandId::SwitchToDarkTheme,
            CommandId::ToggleTerminal,
            CommandId::ToggleMaven,
            CommandId::ToggleStatusBar,
        ];
        assert_eq!(CommandId::OpenSettings.id(), "open-settings");
        assert_eq!(CommandId::SwitchToLightTheme.id(), "switch-theme-light");
        assert_eq!(CommandId::ToggleTerminal.id(), "toggle-terminal");
        let mut ids: Vec<&str> = all.iter().map(|id| id.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all.len(), "诊断 id 必须互不相同");
    }

    #[test]
    fn menu_only_command_ids_are_stable() {
        let menu_only = [
            (CommandId::OpenFile, "open-file"),
            (CommandId::CloseTab, "close-tab"),
            (CommandId::CloseOtherTabs, "close-other-tabs"),
            (CommandId::CloseAllTabs, "close-all-tabs"),
            (CommandId::CloseSavedTabs, "close-saved-tabs"),
            (CommandId::CloseTabsToLeft, "close-tabs-to-left"),
            (CommandId::CloseTabsToRight, "close-tabs-to-right"),
            (CommandId::ReopenClosedTab, "reopen-closed-tab"),
            (CommandId::NewWindow, "new-window"),
        ];
        let visible = visible_commands(dark());
        let mut ids: Vec<&str> = menu_only.iter().map(|(id, _)| id.id()).collect();
        for (id, expected) in menu_only {
            assert_eq!(id.id(), expected, "{} 的诊断 id 变了", expected);
            assert!(
                !visible.contains(&id),
                "{} 不该出现在命令面板里（它只由主菜单触发）",
                expected
            );
            assert!(
                !COMMAND_ORDER.contains(&id),
                "{} 不该进 COMMAND_ORDER（那会改掉面板的行序）",
                expected
            );
        }
        for other in [
            CommandId::OpenSettings,
            CommandId::OpenAppearanceSettings,
            CommandId::SwitchToLightTheme,
            CommandId::SwitchToDarkTheme,
            CommandId::ToggleTerminal,
            CommandId::ToggleMaven,
            CommandId::ToggleStatusBar,
            CommandId::ToggleMenuBar,
            CommandId::SaveBuffer,
            CommandId::OpenCommandPalette,
            CommandId::OpenQuickOpen,
            CommandId::OpenSearchEverywhere,
            CommandId::OpenReplaceInFiles,
            CommandId::NavigateToDefinition,
            CommandId::NavigateToImplementation,
            CommandId::NavigateToTypeDefinition,
            CommandId::GoToReferences,
            CommandId::RenameSymbol,
            CommandId::FormatDocument,
        ] {
            ids.push(other.id());
        }
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "全部 CommandId 的 id 必须互不相同");
    }

    #[test]
    fn the_actions_catalog_covers_the_whole_command_table() {
        assert_eq!(
            CommandId::ALL.len(),
            28,
            "宿主命令目录的条数变了要同步改注释与 UI-MAP"
        );
        let ids: Vec<&str> = CommandId::ALL.iter().map(|id| id.id()).collect();
        assert_eq!(
            ids,
            [
                "open-settings",
                "open-appearance-settings",
                "switch-theme-light",
                "switch-theme-dark",
                "toggle-terminal",
                "toggle-maven",
                "toggle-status-bar",
                "toggle-menu-bar",
                "save-buffer",
                "open-command-palette",
                "open-quick-open",
                "open-search-everywhere",
                "open-replace-in-files",
                "navigate-to-definition",
                "navigate-to-implementation",
                "navigate-to-type-definition",
                "go-to-references",
                "rename-symbol",
                "format-document",
                "open-file",
                "close-tab",
                "close-other-tabs",
                "close-all-tabs",
                "close-saved-tabs",
                "close-tabs-to-left",
                "close-tabs-to-right",
                "reopen-closed-tab",
                "new-window",
            ],
            "顺序 = 枚举声明顺序（Actions 档的行序），id 来自 `CommandId::id()`"
        );
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "目录里不许有重复 id");

        for id in COMMAND_ORDER {
            assert!(
                CommandId::ALL.contains(&id),
                "命令面板里的 {} 必须也能在 Actions 档里搜到",
                id.id()
            );
        }

        for id in CommandId::ALL {
            let action = super::command_action(id, dark());
            assert_eq!(action.id, id);
            assert!(!action.label.is_empty(), "{} 的 label 为空", id.id());
            assert!(
                !action.description.is_empty(),
                "{} 的 description 为空",
                id.id()
            );
        }
    }

    #[test]
    fn scan_registration_happens_once_and_only_while_visible() {
        let mut state = RightScanState::default();

        assert_eq!(state.request(RightToolWindowView::Maven, false), None);
        assert_eq!(
            state.request(RightToolWindowView::Maven, true),
            Some(RightToolWindowView::Maven)
        );
        assert_eq!(state.request(RightToolWindowView::Maven, true), None);
        assert_eq!(state.request(RightToolWindowView::Maven, false), None);

        assert_eq!(
            state.request(RightToolWindowView::Spring, true),
            Some(RightToolWindowView::Spring)
        );
        assert_eq!(state.request(RightToolWindowView::Spring, true), None);

        assert_eq!(
            state.request(RightToolWindowView::Notifications, true),
            None
        );
    }

    #[test]
    fn scan_completion_notifies_only_while_the_target_view_is_shown() {
        assert!(right_scan_should_notify(
            RightToolWindowView::Maven,
            RightToolWindowView::Maven,
            true,
        ));
        assert!(!right_scan_should_notify(
            RightToolWindowView::Maven,
            RightToolWindowView::Spring,
            true,
        ));
        assert!(!right_scan_should_notify(
            RightToolWindowView::Maven,
            RightToolWindowView::Maven,
            false,
        ));
    }

    /// 一个用完自删的临时目录：断言失败也会跑 `Drop`，名字带时间戳与序号避免并行测试撞名
    /// （`write-stable-tests`：每个资源有且仅有一个拥有者与一条清理路径）。
    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicU32, Ordering};
            static SEQ: AtomicU32 = AtomicU32::new(0);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let seq = SEQ.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("lithe-workbench-{tag}-{nanos}-{seq}"));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// 新建名的语法校验：空串、路径分隔符、尾随点/空格、Windows 保留名全部拒绝，其余放行。
    /// 规则与 `explorer` 里行内改名同源（两边各自持有，因为分了层）。
    #[test]
    fn new_names_refuse_the_shapes_the_disk_refuses() {
        assert_eq!(invalid_name_key(""), Some("lithe.files.invalidFileName"));
        for bad in [
            "a/b", "a\\b", "a\0b", "name.", "name ", "CON", "con.txt", "LPT3", "nul",
        ] {
            assert_eq!(
                invalid_name_key(bad),
                Some("lithe.files.invalidFileNameCharacters"),
                "{bad:?} 应当被拒绝"
            );
        }
        for good in ["Foo.java", "a b", "COM0", "LPT10", "conifer", "name..d"] {
            assert_eq!(invalid_name_key(good), None, "{good:?} 应当放行");
        }
    }

    /// 新建名的存在性校验：同目录已存在同名文件/目录时拦下，并把「哪一类重名」说清楚；
    /// 名字**不做 trim**（只判空时用），尾随空格按非法字符拒绝。走临时的真实目录，不依赖工作区。
    #[test]
    fn a_new_child_must_not_already_be_there() {
        let dir = TestDir::new("new-child");
        std::fs::write(dir.path().join("taken.txt"), b"").expect("seed file");
        std::fs::create_dir(dir.path().join("taken-dir")).expect("seed dir");

        assert_eq!(
            check_new_name(dir.path(), "taken.txt", false),
            Err(tr("lithe.fileExplorer.fileAlreadyExistsMessage")),
        );
        assert_eq!(
            check_new_name(dir.path(), "taken-dir", true),
            Err(tr("lithe.fileExplorer.folderAlreadyExistsMessage")),
        );
        // 没人占用就放行；全空白名按「空」拒绝；尾随空格按「非法字符」拒绝（名字不做 trim）。
        assert!(check_new_name(dir.path(), "free.txt", false).is_ok());
        assert_eq!(
            check_new_name(dir.path(), "   ", false),
            Err(tr("lithe.files.invalidFileName")),
        );
        assert_eq!(
            check_new_name(dir.path(), "free.txt ", false),
            Err(tr("lithe.files.invalidFileNameCharacters")),
        );
    }
}
