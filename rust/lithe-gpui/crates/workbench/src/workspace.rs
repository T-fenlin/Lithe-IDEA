use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{ElementExt as _, HandleEdge, h_flex, resize_handle, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::notification::{Notification, NotificationType};
use gpui_kit::component::{ActiveTheme as _, Icon, Root, WindowExt as _, resize_handle_appearance};
use gpui_kit::{
    Anchor, AnyElement, App, AppContext as _, Axis, Bounds, ClickEvent, Context, Div, Empty,
    Entity, Global, Hsla, InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _,
    PathPromptOptions, Pixels, Point, Render, ScrollHandle, SharedString, Size, Styled as _,
    Window, div, rems,
};

use crate::activity_bar::{ActivityItem, ActivitySide, activity_bar};
use crate::branch_panel::{BranchPanel, open_branch_panel, trigger as branch_trigger};
use crate::command_palette::{
    Category, CommandAction, CommandId, install_actions as install_command_palette, set_shell,
    set_shell_focus,
};
use crate::debug::{DEBUG_DIAGNOSTIC_TAG as DEBUG_TAG, DebugPane, DebugPaneEvent};
use crate::menu_bar::{
    MenuAction, MenuBar, MenuRequest, Missing, diagnose_run as diagnose_menu_run, mode_for,
    set_menu_bar, set_shell as set_menu_bar_shell,
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
    BufferWriteHost, DebugGutterHost, EditorPane, SaveBuffer, TabMenuHostActions,
};
use lithe_gpui_explorer::Explorer;
use lithe_gpui_git::{BottomPane, ChangesView};
use lithe_gpui_run::{RunPane, RunPaneEvent, ToolchainPath, ToolchainResolver};
use lithe_gpui_settings::{
    Category as SettingsCategory, GitIdentityHost, IdentityField, IdentityScope, Overrides,
    RecentProjects, resolve_overrides,
};
use lithe_gpui_shared::icons::{idea, idea_icon_mono};
use lithe_gpui_shared::workspace_config::{ToolchainPaths, load_local_toolchain};
use lithe_gpui_shared::{tr, tr_args};
use lithe_gpui_terminal::{TerminalPane, TerminalPaneEvent};

pub struct ShellStartup {
    compact_menu: bool,
    right_view: Option<RightToolWindowView>,
    extensions_page: bool,
    left_view: Option<usize>,
}

impl Global for ShellStartup {}

pub fn set_shell_startup(
    cx: &mut App,
    compact_menu: bool,
    right_view: Option<RightToolWindowView>,
    extensions_page: bool,
    left_view: Option<usize>,
) {
    println!(
        "S1_WORKSPACE startup compact_menu={compact_menu} right_view={:?} extensions_page={extensions_page} left_view={left_view:?}",
        right_view.map(|view| view.id())
    );
    cx.set_global(ShellStartup {
        compact_menu,
        right_view,
        extensions_page,
        left_view,
    });
}

fn startup_compact_menu(cx: &App) -> bool {
    cx.try_global::<ShellStartup>()
        .map(|startup| startup.compact_menu)
        .unwrap_or(false)
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
    let compact_menu = startup_compact_menu(cx);
    let mut created: Option<Entity<ShellWorkspace>> = None;
    let new_root = window.replace_root(cx, |window, cx| {
        let workspace = cx.new(|cx| {
            ShellWorkspace::new(
                target.to_path_buf(),
                compact_menu,
                false,
                false,
                false,
                window,
                cx,
            )
        });
        created = Some(workspace.clone());
        Root::new(workspace, window, cx)
    });
    let _ = new_root;
    created
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenDestination {
    NewWindow,
    ThisWindow,
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
    Box<dyn FnOnce(Option<String>, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static>;

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

pub(crate) fn command_action(id: CommandId, flags: ActionFlags, is_dark: bool) -> CommandAction {
    use CommandId as C;
    let (category, icon) = match id {
        C::OpenSettings | C::OpenAppearanceSettings => (
            Category::Settings,
            idea_icon_mono(&idea::GEAR_ICON, is_dark),
        ),
        C::SwitchToLightTheme | C::SwitchToDarkTheme => (
            Category::Settings,
            idea_icon_mono(&idea::PALETTE_ICON, is_dark),
        ),
        C::ToggleTerminal => (Category::View, Icon::new(IconName::SquareTerminal)),
        C::ToggleMaven => (Category::View, Icon::new(IconName::Package)),
        C::ToggleStatusBar => (Category::View, Icon::new(IconName::PanelBottom)),
        C::ToggleMenuBar => (Category::View, Icon::new(IconName::List)),
        C::SaveBuffer => (
            Category::View,
            idea_icon_mono(&idea::FLOPPY_DISK_ICON, is_dark),
        ),
        C::OpenCommandPalette => (
            Category::View,
            idea_icon_mono(&idea::MAGNIFYING_GLASS_ICON, is_dark),
        ),
        C::OpenQuickOpen => (
            Category::View,
            idea_icon_mono(&idea::MAGNIFYING_GLASS_ICON, is_dark),
        ),
        C::OpenSearchEverywhere => (
            Category::View,
            idea_icon_mono(&idea::GLOBE_HEMISPHERE_WEST_ICON, is_dark),
        ),
        C::OpenReplaceInFiles => (Category::View, Icon::new(IconName::Replace)),
        C::NavigateToDefinition => (
            Category::View,
            idea_icon_mono(&idea::ARROW_RIGHT_ICON, is_dark),
        ),
        C::OpenFile => (Category::View, idea_icon_mono(&idea::FOLDER_ICON, is_dark)),
        C::CloseTab
        | C::CloseOtherTabs
        | C::CloseAllTabs
        | C::CloseSavedTabs
        | C::CloseTabsToLeft
        | C::CloseTabsToRight => (Category::View, idea_icon_mono(&idea::X_ICON, is_dark)),
        C::ReopenClosedTab => (Category::View, idea_icon_mono(&idea::REFRESH_ICON, is_dark)),
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
        C::OpenFile => (tr("lithe.outline.openFile"), tr("lithe.menu.file")),
        C::CloseTab => (tr("lithe.menu.closeTab"), tr("lithe.menu.file")),
        C::CloseOtherTabs => (tr("lithe.menu.closeOtherTabs"), tr("lithe.menu.file")),
        C::CloseAllTabs => (tr("lithe.menu.closeAllTabs"), tr("lithe.menu.file")),
        C::CloseSavedTabs => (tr("lithe.menu.closeSavedTabs"), tr("lithe.menu.file")),
        C::CloseTabsToLeft => (tr("lithe.menu.closeTabsToLeft"), tr("lithe.menu.file")),
        C::CloseTabsToRight => (tr("lithe.menu.closeTabsToRight"), tr("lithe.menu.file")),
        C::ReopenClosedTab => (tr("lithe.menu.reopenClosedTab"), tr("lithe.menu.file")),
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
    split_container: Rc<Cell<Option<Size<Pixels>>>>,
    pane_resize_drag: Option<PaneResizeDrag>,
    right_view: RightToolWindowView,
    right_visible: bool,
    maven_project: Option<crate::maven::MavenProjectView>,
    spring_index: Option<crate::spring::SpringIndexView>,
    right_scan: RightScanState,
    explorer: Entity<Explorer>,
    changes: Entity<ChangesView>,
    search: Entity<crate::search_panel::SearchPanel>,
    quick_open_overlay: Option<gpui_kit::WeakEntity<crate::quick_open::QuickOpen>>,
    search_everywhere_overlay:
        Option<gpui_kit::WeakEntity<crate::search_everywhere::SearchEverywhere>>,
    replace_in_files_overlay: Option<gpui_kit::WeakEntity<crate::replace_in_files::ReplaceInFiles>>,
    editor: Entity<EditorPane>,
    extensions_page: Option<Entity<crate::extensions_page::ExtensionsPage>>,
    extensions_page_open: bool,
    terminal: Entity<TerminalPane>,
    bottom_git: Entity<BottomPane>,
    run: Entity<RunPane>,
    _run_subscription: Option<gpui_kit::Subscription>,
    debug: Entity<DebugPane>,
    _debug_subscription: Option<gpui_kit::Subscription>,
    bottom_kind: BottomPaneKind,
    bottom_visible: bool,
    _settings_subscription: Option<gpui_kit::Subscription>,
    _terminal_subscription: Option<gpui_kit::Subscription>,
    _editor_subscription: Option<gpui_kit::Subscription>,
    focus: gpui_kit::FocusHandle,
    menu_bar: Entity<MenuBar>,
    project_menu: Entity<ProjectMenu>,
    _project_menu_subscription: Option<gpui_kit::Subscription>,
    branch_panel: Entity<BranchPanel>,
    _branch_panel_subscription: Option<gpui_kit::Subscription>,
    branch_panel_probe: bool,
    recent_projects: RecentProjects,
    recent_projects_path: Option<PathBuf>,
    project_open_do_not_ask: Rc<Cell<bool>>,
    notifications: Entity<lithe_gpui_notify::Store>,
    notification_panel: Entity<crate::notifications::NotificationPanel>,
    spring_panel: Entity<crate::spring_panel::SpringPanel>,
    _notification_toast_subscription: Option<gpui_kit::Subscription>,
    notification_toast_seq: u64,
    session: SessionTracker,
    session_save_task: Option<gpui_kit::Task<()>>,
    _quit_subscription: Option<gpui_kit::Subscription>,
}

impl ShellWorkspace {
    pub fn new(
        root: PathBuf,
        compact_menu: bool,
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

        register_java_toolchain(&root, cx);

        prepare_workspace_config(root.clone(), cx);

        if let Some(store) = lithe_gpui_settings::try_store(cx) {
            let settings_root = root.clone();
            store.update(cx, |store, cx| {
                store.set_workspace_root(Some(settings_root), cx)
            });
        }

        let notifications = cx.new(|_| lithe_gpui_notify::Store::new());

        let notification_toast_subscription = Some(cx.observe_in(
            &notifications,
            window,
            |this, store, window, cx| this.dispatch_notification_toasts(&store, window, cx),
        ));

        let editor = cx.new(|cx| EditorPane::new(Some(notifications.downgrade()), window, cx));
        editor.update(cx, |pane, cx| pane.prepare_java(root.clone(), cx));
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
                              async_cx: &mut gpui_kit::AsyncWindowContext| {
                            let scope = IdentityScope::from_id(scope);
                            let root = root_text.clone();
                            async_cx
                            .spawn(async move |async_cx: &mut gpui_kit::AsyncWindowContext| {
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
                              async_cx: &mut gpui_kit::AsyncWindowContext| {
                            let scope = IdentityScope::from_id(scope);
                            let field = IdentityField::from_id(key);
                            let root = root_text.clone();
                            async_cx
                            .spawn(async move |async_cx: &mut gpui_kit::AsyncWindowContext| {
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
                });
                this.terminal.update(cx, |pane, cx| {
                    pane.set_default_shell(&settings.terminal_default_shell_id, cx);
                    pane.set_font_size(terminal_font_size_override(settings.terminal_font_size), cx);
                });
                this.changes.update(cx, |view, cx| {
                    view.set_confirm_before_discard(settings.confirm_before_discard, cx);
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
        set_menu_bar_shell(cx.entity().downgrade());
        let menu_bar = MenuBar::new(mode_for(compact_menu), focus.clone(), cx);
        set_menu_bar(menu_bar.downgrade());

        let loaded_recent = lithe_gpui_settings::load_recent_projects();
        loaded_recent.report();
        let recent_projects = loaded_recent.projects.clone();
        let recent_projects_path = loaded_recent.path.clone();
        let project_menu = ProjectMenu::new(
            project_name.as_ref(),
            recent_projects.len(),
            focus.clone(),
            cx,
        );
        set_project_menu(project_menu.downgrade());
        let project_menu_subscription = Some(cx.observe(&project_menu, |_, _, cx| cx.notify()));

        let branch_panel = BranchPanel::new(root.clone(), window, cx);
        let branch_panel_subscription = Some(cx.observe(&branch_panel, |_, _, cx| cx.notify()));

        let notification_panel =
            cx.new(|cx| crate::notifications::NotificationPanel::new(&notifications, window, cx));

        let spring_panel = cx.new(|cx| crate::spring_panel::SpringPanel::new(window, cx));
        cx.observe(&spring_panel, |_, _, cx| cx.notify()).detach();

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
            split_container: Rc::new(Cell::new(None)),
            pane_resize_drag: None,
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
            notification_toast_seq: 0,
            _notification_toast_subscription: notification_toast_subscription,
            session: SessionTracker::new(&root),
            session_save_task: None,
            _quit_subscription: None,
        };
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
            workspace.select_top_activity(index, cx);
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
        let viewport = window.viewport_size();
        for (pane, stored) in [
            (ResizablePane::LeftSidebar, session.left_pane_width),
            (ResizablePane::RightToolWindow, session.right_pane_width),
            (ResizablePane::BottomPane, session.bottom_pane_height),
        ] {
            let (min, max) = self.pane_span_range(pane, viewport);
            let resolved = pane_span_on_restore(stored, self.pane_span(pane), min, max);
            self.set_pane_span(pane, resolved);
        }
        println!(
            "S1_SESSION_SIZES left={} right={} bottom={}",
            self.left_sidebar_width, self.right_tool_window_width, self.bottom_pane_height
        );
        if let Some(index) = session.top_activity_view {
            if index < self.activity_items.len() {
                self.select_top_activity(index, cx);
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
                        match crate::debug_transport::java_attach_configuration("Java", port, None)
                        {
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

    fn pane_span(&self, pane: ResizablePane) -> f32 {
        match pane {
            ResizablePane::LeftSidebar => self.left_sidebar_width,
            ResizablePane::RightToolWindow => self.right_tool_window_width,
            ResizablePane::BottomPane => self.bottom_pane_height,
        }
    }

    fn set_pane_span(&mut self, pane: ResizablePane, span: f32) {
        match pane {
            ResizablePane::LeftSidebar => self.left_sidebar_width = span,
            ResizablePane::RightToolWindow => self.right_tool_window_width = span,
            ResizablePane::BottomPane => self.bottom_pane_height = span,
        }
    }

    fn pane_span_range(&self, pane: ResizablePane, viewport: Size<Pixels>) -> (f32, f32) {
        let container = self.split_container.get().unwrap_or(viewport);
        match pane {
            ResizablePane::LeftSidebar => (
                SIDEBAR_MIN_WIDTH_SPEC,
                left_sidebar_max_width(f32::from(container.width)),
            ),
            ResizablePane::RightToolWindow => (
                RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC,
                right_tool_window_max_width(f32::from(viewport.width)),
            ),
            ResizablePane::BottomPane => (
                BOTTOM_PANE_MIN_HEIGHT_SPEC,
                bottom_pane_max_height(f32::from(container.height)),
            ),
        }
    }

    fn begin_pane_resize(&mut self, pane: ResizablePane) {
        self.pane_resize_drag = Some(PaneResizeDrag { pane, origin: None });
    }

    fn drag_pane_resize(
        &mut self,
        pane: ResizablePane,
        along: f32,
        viewport: Size<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let current = self.pane_span(pane);
        let (min, max) = self.pane_span_range(pane, viewport);
        let direction = pane.drag_direction();
        let Some(session) = self.pane_resize_drag.as_mut() else {
            return;
        };
        if session.pane != pane {
            return;
        }
        let next = match session.origin {
            Some((press, start)) => clamped_drag_span(press, start, along, min, max, direction),
            None => {
                session.origin = Some((along, current));
                return;
            }
        };
        if next != current {
            self.set_pane_span(pane, next);
            cx.notify();
        }
    }

    fn end_pane_resize(
        &mut self,
        pane: ResizablePane,
        viewport: Size<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.pane_resize_drag.as_ref().map(|session| session.pane) != Some(pane) {
            return;
        }
        let (min, max) = self.pane_span_range(pane, viewport);
        let resolved = clamp_pane_span(self.pane_span(pane), min, max);
        self.pane_resize_drag = None;
        self.set_pane_span(pane, resolved);
        eprintln!("S1_PANE_RESIZE pane={} span={resolved}", pane.id());
        self.schedule_session_save(cx);
        cx.notify();
    }

    fn reveal_in_project_tree(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.top_activity_view = Some(DEFAULT_TOP_ACTIVITY);
        self.left_pane_visible = true;
        self.left_sidebar_visible = true;
        let explorer = self.explorer.clone();
        let _ = explorer.update(cx, |explorer, cx| explorer.reveal(path, cx));
        cx.notify();
    }

    fn select_top_activity(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.top_activity_view == Some(index) {
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
        let is_dark = cx.theme().is_dark();
        visible_commands(flags)
            .into_iter()
            .map(|id| command_action(id, flags, is_dark))
            .collect()
    }

    pub(crate) fn action_catalog(&self, cx: &App) -> Vec<CommandAction> {
        let flags = self.action_flags(cx);
        let is_dark = cx.theme().is_dark();
        CommandId::ALL
            .into_iter()
            .map(|id| command_action(id, flags, is_dark))
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

    pub(crate) fn run_command(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = visible_commands(self.action_flags(cx)).get(row).copied() else {
            return;
        };
        self.run_command_id(id, window, cx);
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
            MenuAction::Undo
            | MenuAction::Redo
            | MenuAction::Cut
            | MenuAction::Copy
            | MenuAction::Paste
            | MenuAction::SelectAll
            | MenuAction::Find
            | MenuAction::FindAndReplace
            | MenuAction::QuickFix => {
                self.dispatch_editor_action(action, window, cx);
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
            MenuAction::ShowHover => {
                self.editor
                    .update(cx, |pane, cx| pane.show_hover_at_caret(window, cx));
                diagnose_menu_run(action, "showHover");
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
                self.select_top_activity(index, cx);
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
            | MenuAction::CommandPalette
            | MenuAction::ToggleTerminal
            | MenuAction::ToggleMaven
            | MenuAction::ToggleStatusBar
            | MenuAction::OpenAppearanceSettings
            | MenuAction::ToggleMenuBar
            | MenuAction::GoToDefinition
            | MenuAction::QuickOpen
            | MenuAction::SearchEverywhere
            | MenuAction::ReplaceInFiles => {
                diagnose_menu_run(action, "unreachable");
            }
        }
        self.schedule_session_save(cx);
        cx.notify();
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

    fn run_command_id(&mut self, id: CommandId, window: &mut Window, cx: &mut Context<Self>) {
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
            CommandId::NavigateToDefinition => {
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.navigate_to_definition(window, cx));
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
        let note = Notification::from(crate::notifications::render_message(entry))
            .with_type(match entry.severity() {
                lithe_gpui_notify::Severity::Info => NotificationType::Info,
                lithe_gpui_notify::Severity::Warning => NotificationType::Warning,
                lithe_gpui_notify::Severity::Error => NotificationType::Error,
            })
            .placement(Anchor::BottomRight);
        window.push_notification(note, cx);
    }

    fn note_notifications_opened(&mut self, cx: &mut Context<Self>) {
        self.notifications
            .update(cx, |store, _| store.mark_all_read());
        let read_upto = self.notifications.read(cx).read_upto_seq();
        eprintln!("S1_NOTIFICATION action=open readUpToSeq={read_upto}");
    }

    fn footer_right(&self, cx: &App) -> Vec<StatusEntry> {
        let cursor = self.editor.read(cx).cursor_position(cx);
        let tab_size = self.editor.read(cx).tab_size().to_string();

        vec![
            StatusEntry::new(SharedString::from(format!(
                "{}:{}",
                cursor.line, cursor.column
            ))),
            StatusEntry::new(SharedString::from("UTF-8")),
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
        ]
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
            OpenDestination::NewWindow => self.report_new_window_not_wired(cx),
            OpenDestination::ThisWindow => {
                self.rebuild_project_window(target, decision.remember, window, cx)
            }
        }
    }

    fn report_new_window_not_wired(&mut self, cx: &mut Context<Self>) {
        eprintln!(
            "S1_OPEN_PROJECT destination=new-window state=not_wired missing=window_handle_routing"
        );
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
        let old_root = self.root.display().to_string();
        self.search_index.schedule_invalidate(cx);
        self.flush_session(true, cx);
        let Some(workspace) = replace_root_with_workspace(&target, window, cx) else {
            eprintln!("S1_OPEN_PROJECT state=rebuild_failed reason=no_view");
            return;
        };

        let _ = workspace.update(cx, |shell, cx| {
            shell.record_opened_project(&target, Some(false), cx)
        });

        if remember {
            match lithe_gpui_settings::try_store(cx) {
                Some(store) => {
                    store.update(cx, |store, cx| {
                        store.remember_project_open_destination(false, cx)
                    });
                    println!(
                        "S1_OPEN_PROJECT remember=true openFoldersInNewWindow=false askWhereToOpenProjects=false"
                    );
                }
                None => eprintln!("S1_OPEN_PROJECT remember=skipped reason=no_settings_store"),
            }
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

        window.open_dialog(cx, move |dialog, _window, cx| {
            let checkbox_cell = do_not_ask.clone();
            let checkbox_shell = shell.clone();
            let click_cell = do_not_ask.clone();
            let cancel_label = tr("lithe.projectOpen.cancel");
            let new_window_label = tr("lithe.projectOpen.newWindow");
            let this_window_label = tr("lithe.projectOpen.thisWindow");
            let new_window_shell = shell.clone();
            let this_window_shell = shell.clone();
            let this_window_target = target.clone();

            dialog
                .title(tr("lithe.projectOpen.title"))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(body.clone()),
                )
                .child(
                    Checkbox::new("lithe-project-open-do-not-ask")
                        .label(tr("lithe.projectOpen.doNotAskAgain"))
                        .checked(do_not_ask.get())
                        .on_change(move |&value, _window, cx| {
                            checkbox_cell.set(value);
                            let _ = checkbox_shell.update(cx, |_shell, cx| cx.notify());
                        }),
                )
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        .child(
                            Button::new("lithe-project-open-cancel")
                                .label(cancel_label)
                                .on_click(|_, window, cx| {
                                    eprintln!(
                                        "S1_OPEN_PROJECT state=cancelled reason=dialog-cancel"
                                    );
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("lithe-project-open-new-window")
                                .label(new_window_label)
                                .on_click(move |_, window, cx| {
                                    let _ = new_window_shell.update(cx, |shell, cx| {
                                        shell.report_new_window_not_wired(cx)
                                    });
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("lithe-project-open-this-window")
                                .primary()
                                .label(this_window_label)
                                .on_click(move |_, window, cx| {
                                    let remember = click_cell.get();
                                    window.close_dialog(cx);
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
                                }),
                        ),
                )
        });
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

gpui_kit::actions!(lithe_workbench, [OpenProjectFolder]);

pub fn install_open_project_action(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-o", OpenProjectFolder, None)]);
    cx.on_action(|_: &OpenProjectFolder, cx: &mut App| {
        let bar = crate::menu_bar::try_handle();
        let Some(bar) = bar else {
            eprintln!("S1_MENU_KEY id=lithe.menu.openFolder source=ctrl-o state=no_menu_bar");
            return;
        };
        let _ = bar.update(cx, |bar, cx| bar.push_run(MenuAction::OpenFolder, cx));
        eprintln!("S1_MENU_KEY id=lithe.menu.openFolder source=ctrl-o state=pushed");
    });
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

impl Render for ShellWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu = crate::menu_bar::render_bar(&self.menu_bar, window, cx);
        let project_entries = self.project_entries();
        let project_menu = render_project_menu(
            &self.project_menu,
            &project_entries,
            self.recent_projects.entries(),
            window,
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
                        None => this.select_top_activity(index, cx),
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

        let on_close_right_activity = {
            let handle = handle.clone();
            move |event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                handle.update(cx, |this, cx| {
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

        let explorer = self.explorer.clone();
        let editor = self.editor.clone();
        let left_content: AnyElement = if self.top_activity_view == Some(CHANGES_ACTIVITY_IX) {
            self.changes.clone().into_any_element()
        } else if self.top_activity_view == Some(SEARCH_ACTIVITY_IX) {
            self.search.clone().into_any_element()
        } else {
            explorer.into_any_element()
        };
        let center: AnyElement = if self.extensions_page_open {
            match &self.extensions_page {
                Some(page) => page.clone().into_any_element(),
                None => editor_island(editor, cx).into_any_element(),
            }
        } else {
            editor_island(editor, cx).into_any_element()
        };
        let right_tool_window = right_tool_window(
            self.right_view,
            self.maven_project.as_ref(),
            self.spring_index.as_ref(),
            &self.notification_panel,
            &self.spring_panel,
            on_close_right_activity,
            cx,
        );
        let right_tool_window_visible = self.right_visible;
        let left_sidebar_visible = self.left_sidebar_visible;
        let left_pane_visible = self.left_pane_visible;
        let viewport = window.viewport_size();
        let split_container = self.split_container.clone();
        let left_sidebar_width = {
            let (min, max) = self.pane_span_range(ResizablePane::LeftSidebar, viewport);
            clamp_pane_span(self.left_sidebar_width, min, max)
        };
        let right_tool_window_width = {
            let (min, max) = self.pane_span_range(ResizablePane::RightToolWindow, viewport);
            clamp_pane_span(self.right_tool_window_width, min, max)
        };
        let bottom_pane_height = {
            let (min, max) = self.pane_span_range(ResizablePane::BottomPane, viewport);
            clamp_pane_span(self.bottom_pane_height, min, max)
        };

        let pane_self_hidden =
            self.bottom_kind == BottomPaneKind::Git && !self.bottom_git.read(cx).visible();
        let bottom = (self.bottom_visible && !pane_self_hidden).then(|| {
            let content: AnyElement = match self.bottom_kind {
                BottomPaneKind::Terminal => self.terminal.clone().into_any_element(),
                BottomPaneKind::Git => self.bottom_git.clone().into_any_element(),
                BottomPaneKind::Diagnostics => {
                    self.editor.read(cx).problems_pane().into_any_element()
                }
                BottomPaneKind::Run => self.run.clone().into_any_element(),
                BottomPaneKind::Debug => self.debug.clone().into_any_element(),
            };
            bottom_pane(
                content,
                bottom_pane_height,
                handle.downgrade(),
                ResizablePane::BottomPane,
                cx,
            )
        });

        let root = v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &SaveBuffer, window, cx| {
                let _ = this
                    .editor
                    .update(cx, |pane, cx| pane.save_active(window, cx));
            }))
            .child(title_bar(
                menu.unwrap_or_else(|| div().into_any_element()),
                project_menu.unwrap_or_else(|| div().into_any_element()),
                branch_item,
                self.run.clone(),
                window,
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
                h_flex()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .gap_1()
                    .pr_1()
                    .children(left_sidebar_visible.then_some(left_rail))
                    .child(
                        h_flex()
                            .id(SPLIT_CONTAINER_ID)
                            .flex_1()
                            .h_full()
                            .min_w_0()
                            .on_prepaint({
                                let container = split_container;
                                move |bounds, window, _cx| {
                                    measure_split_container(&container, bounds, window);
                                }
                            })
                            .children((left_sidebar_visible && left_pane_visible).then(|| {
                                let pane = div()
                                    .relative()
                                    .size_full()
                                    .child(left_content)
                                    .child(drag_handle(
                                        handle.downgrade(),
                                        ResizablePane::LeftSidebar,
                                    ))
                                    .child(drag_tracker(
                                        handle.downgrade(),
                                        ResizablePane::LeftSidebar,
                                    ));
                                side_pane(
                                    div().w(rems(left_sidebar_width / 16.)),
                                    pane,
                                    SidePaneEdge::WithoutTrailing,
                                    cx,
                                )
                                .into_any_element()
                            }))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .h_full()
                                    .min_w_0()
                                    .min_h_0()
                                    .child(center)
                                    .children(bottom),
                            ),
                    )
                    .children(right_tool_window_visible.then(|| {
                        let pane = div()
                            .relative()
                            .size_full()
                            .child(right_tool_window)
                            .child(drag_handle(
                                handle.downgrade(),
                                ResizablePane::RightToolWindow,
                            ))
                            .child(drag_tracker(
                                handle.downgrade(),
                                ResizablePane::RightToolWindow,
                            ));
                        side_pane(
                            div().w(rems(right_tool_window_width / 16.)),
                            pane,
                            SidePaneEdge::All,
                            cx,
                        )
                        .into_any_element()
                    }))
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum SidePaneEdge {
    All,
    WithoutTrailing,
}

fn side_pane(
    outer: Div,
    content: impl IntoElement,
    edge: SidePaneEdge,
    cx: &App,
) -> impl IntoElement {
    let border_color = cx.theme().border;
    let outer = outer
        .h_full()
        .flex_shrink_0()
        .relative()
        .bg(cx.theme().background);
    let outer = match edge {
        SidePaneEdge::All => outer.border_1().border_color(border_color),
        SidePaneEdge::WithoutTrailing => outer
            .border_t_1()
            .border_b_1()
            .border_l_1()
            .border_color(border_color),
    };
    outer.child(content)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResizablePane {
    LeftSidebar,
    RightToolWindow,
    BottomPane,
}

impl ResizablePane {
    fn axis(self) -> Axis {
        match self {
            Self::LeftSidebar | Self::RightToolWindow => Axis::Horizontal,
            Self::BottomPane => Axis::Vertical,
        }
    }

    fn handle_edge(self) -> HandleEdge {
        match self {
            Self::LeftSidebar => HandleEdge::Trailing,
            Self::RightToolWindow | Self::BottomPane => HandleEdge::Leading,
        }
    }

    fn handle_id(self) -> &'static str {
        match self {
            Self::LeftSidebar => "lithe-left-pane-resize",
            Self::RightToolWindow => "lithe-right-pane-resize",
            Self::BottomPane => "lithe-bottom-pane-resize",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::LeftSidebar => "left",
            Self::RightToolWindow => "right",
            Self::BottomPane => "bottom",
        }
    }

    fn drag_direction(self) -> f32 {
        match self {
            Self::LeftSidebar => 1.,
            Self::RightToolWindow | Self::BottomPane => -1.,
        }
    }

    fn pointer_along(self, position: Point<Pixels>) -> f32 {
        f32::from(match self.axis() {
            Axis::Horizontal => position.x,
            Axis::Vertical => position.y,
        })
    }
}

struct PaneResizeDrag {
    pane: ResizablePane,
    origin: Option<(f32, f32)>,
}

fn clamp_pane_span(value: f32, min: f32, max: f32) -> f32 {
    if value.is_nan() {
        return min;
    }
    value.clamp(min, max.max(min))
}

fn pane_span_on_restore(stored: Option<f32>, current: f32, min: f32, max: f32) -> f32 {
    match stored {
        Some(value) => clamp_pane_span(value, min, max),
        None => current,
    }
}

fn clamped_drag_span(
    press: f32,
    start: f32,
    current: f32,
    min: f32,
    max: f32,
    direction: f32,
) -> f32 {
    clamp_pane_span(start + direction * (current - press), min, max)
}

const SPLIT_CONTAINER_ID: &str = "lithe-workspace-split-container";

fn measure_split_container(
    cache: &Rc<Cell<Option<Size<Pixels>>>>,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    let size = bounds.size;
    let previous = cache.get();
    cache.set(Some(size));
    if previous.is_none() {
        window.request_animation_frame();
    }
    if previous != Some(size) {
        eprintln!(
            "S1_SPLIT_CONTAINER w={} h={}",
            f32::from(size.width),
            f32::from(size.height)
        );
    }
}

fn left_sidebar_max_width(available_width: f32) -> f32 {
    if !available_width.is_finite() || available_width <= 0. {
        return SIDEBAR_MAX_WIDTH_SPEC;
    }
    (available_width - SIDEBAR_MIN_EDITOR_WIDTH_SPEC)
        .min(SIDEBAR_MAX_WIDTH_SPEC)
        .max(SIDEBAR_MIN_WIDTH_SPEC)
}

fn right_tool_window_max_width(available_width: f32) -> f32 {
    if !available_width.is_finite() || available_width <= 0. {
        return RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC;
    }
    (available_width - RIGHT_TOOL_WINDOW_MIN_WORKSPACE_WIDTH_SPEC)
        .min(RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC)
        .max(RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC)
}

fn bottom_pane_max_height(available_height: f32) -> f32 {
    if !available_height.is_finite() || available_height <= 0. {
        return f32::INFINITY;
    }
    (available_height - BOTTOM_PANE_MIN_CENTER_HEIGHT_SPEC).max(BOTTOM_PANE_MIN_HEIGHT_SPEC)
}

fn drag_handle(
    this: gpui_kit::WeakEntity<ShellWorkspace>,
    pane: ResizablePane,
) -> impl IntoElement {
    resize_handle(pane.handle_id(), pane.axis())
        .with_appearance(resize_handle_appearance())
        .inside(pane.handle_edge())
        .on_drag(PaneResizeHandle, move |_, _, _, cx| {
            cx.stop_propagation();
            let _ = this.update(cx, |shell, _| shell.begin_pane_resize(pane));
            cx.new(|_| PaneResizeHandle)
        })
}

struct PaneResizeHandle;

impl Render for PaneResizeHandle {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

fn drag_tracker(
    this: gpui_kit::WeakEntity<ShellWorkspace>,
    pane: ResizablePane,
) -> impl IntoElement {
    let drag = this.clone();
    let release = this;
    gpui_kit::canvas(
        |_bounds, _window, _cx| {},
        move |_bounds, _prepaint, window, _cx| {
            window.on_mouse_event(move |event: &gpui_kit::MouseMoveEvent, phase, window, cx| {
                if !phase.bubble() {
                    return;
                }
                let viewport = window.viewport_size();
                let along = pane.pointer_along(event.position);
                let released = event.pressed_button != Some(gpui_kit::MouseButton::Left);
                let _ = drag.update(cx, |shell, cx| {
                    if shell.pane_resize_drag.as_ref().map(|session| session.pane) != Some(pane) {
                        return;
                    }
                    if released {
                        shell.end_pane_resize(pane, viewport, cx);
                    } else {
                        shell.drag_pane_resize(pane, along, viewport, cx);
                    }
                });
            });
            window.on_mouse_event(move |_: &gpui_kit::MouseUpEvent, phase, window, cx| {
                if !phase.bubble() {
                    return;
                }
                let viewport = window.viewport_size();
                let _ = release.update(cx, |shell, cx| {
                    shell.end_pane_resize(pane, viewport, cx);
                });
            });
        },
    )
    .absolute()
    .size_full()
}

fn editor_island(content: impl IntoElement, cx: &App) -> impl IntoElement {
    div()
        .relative()
        .w_full()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .child(content)
}

fn bottom_pane(
    content: AnyElement,
    height: f32,
    handle: gpui_kit::WeakEntity<ShellWorkspace>,
    pane: ResizablePane,
    cx: &App,
) -> impl IntoElement {
    let body = div()
        .relative()
        .w_full()
        .h(rems(height / 16.))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .child(content)
        .child(drag_handle(handle.clone(), pane))
        .child(drag_tracker(handle, pane));

    v_flex().w_full().flex_shrink_0().child(body)
}

fn activity_items() -> Vec<ActivityItem> {
    vec![
        ActivityItem::idea(&idea::FILES_ICON, tr("lithe.workbench.project")),
        ActivityItem::idea(&idea::GIT_BRANCH_ICON, tr("lithe.workbench.changes")),
        ActivityItem::idea(&idea::MAGNIFYING_GLASS_ICON, tr("lithe.workbench.search")),
        ActivityItem::idea(&idea::PLAY_ICON, tr("lithe.workbench.run")).bottom(true),
        ActivityItem::new(IconName::SquareTerminal, tr("lithe.workbench.terminal")).bottom(true),
        ActivityItem::idea(
            &idea::WARNING_CIRCLE_ICON,
            tr("lithe.workbench.diagnostics"),
        )
        .bottom(true),
        ActivityItem::new(IconName::GitGraph, tr("lithe.workbench.gitLog")).bottom(true),
        ActivityItem::idea(&idea::GEAR_ICON, tr("lithe.workbench.settings")).bottom(true),
    ]
}

fn notification_badge_color(store: &Entity<lithe_gpui_notify::Store>, cx: &App) -> Option<Hsla> {
    match store.read(cx).badge() {
        lithe_gpui_notify::Badge::None => None,
        lithe_gpui_notify::Badge::Info => Some(cx.theme().info),
        lithe_gpui_notify::Badge::Error => Some(cx.theme().danger),
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
        ActivityItem::new(IconName::Puzzle, tr("lithe.extensions.title")),
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
    use super::{
        ActionFlags, Axis, BOTTOM_PANE_HEIGHT_SPEC, BOTTOM_PANE_MIN_CENTER_HEIGHT_SPEC,
        BOTTOM_PANE_MIN_HEIGHT_SPEC, COMMAND_ORDER, CommandId, HandleEdge, OpenDestination,
        Overrides, ProjectOpenDecision, RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC,
        RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC, RIGHT_TOOL_WINDOW_MIN_WORKSPACE_WIDTH_SPEC,
        RIGHT_TOOL_WINDOW_WIDTH_SPEC, ResizablePane, RightScanState, RightToolWindowView,
        SIDEBAR_MAX_WIDTH_SPEC, SIDEBAR_MIN_EDITOR_WIDTH_SPEC, SIDEBAR_MIN_WIDTH_SPEC,
        ToolchainPaths, bottom_pane_max_height, clamp_pane_span, clamped_drag_span,
        left_activity_index, left_sidebar_max_width, pane_span_on_restore, resolve_overrides,
        resolve_project_open_destination, right_scan_should_notify, right_tool_window_max_width,
        visible_commands,
    };

    #[test]
    fn drag_width_follows_the_pointer_and_stays_in_range() {
        let container_width = 1280.;
        let left = |press: f32, start: f32, current: f32| {
            clamped_drag_span(
                press,
                start,
                current,
                SIDEBAR_MIN_WIDTH_SPEC,
                left_sidebar_max_width(container_width),
                ResizablePane::LeftSidebar.drag_direction(),
            )
        };
        assert_eq!(left(100., 320., 200.), 420.);
        assert_eq!(left(300., 320., 200.), 220.);
        assert_eq!(left(0., 320., -10_000.), SIDEBAR_MIN_WIDTH_SPEC);
        assert_eq!(
            left(0., 320., 10_000.),
            left_sidebar_max_width(container_width)
        );
        assert_eq!(left(500., 320., 500.), 320.);
    }

    #[test]
    fn left_pane_max_width_is_the_macos_formula() {
        assert_eq!(SIDEBAR_MIN_EDITOR_WIDTH_SPEC, 400., "400 本身是真源常量");
        assert_eq!(
            SIDEBAR_MIN_WIDTH_SPEC, 30.,
            "下界 = 真源 minimumSidebarWidth"
        );
        assert_eq!(
            SIDEBAR_MAX_WIDTH_SPEC, 520.,
            "静网上界 = 真源那个 min(520, …)"
        );

        assert_eq!(left_sidebar_max_width(400.), SIDEBAR_MIN_WIDTH_SPEC);
        assert_eq!(left_sidebar_max_width(400. - 1.), SIDEBAR_MIN_WIDTH_SPEC);
        assert_eq!(left_sidebar_max_width(30.), SIDEBAR_MIN_WIDTH_SPEC);
        assert_eq!(left_sidebar_max_width(1920.), SIDEBAR_MAX_WIDTH_SPEC);
        assert_eq!(left_sidebar_max_width(10_000.), SIDEBAR_MAX_WIDTH_SPEC);
        assert_eq!(
            left_sidebar_max_width(920.),
            SIDEBAR_MAX_WIDTH_SPEC,
            "正好封顶"
        );
        assert_eq!(left_sidebar_max_width(700.), 300.);
        assert_eq!(left_sidebar_max_width(919.), 519.);
        assert_eq!(left_sidebar_max_width(0.), SIDEBAR_MAX_WIDTH_SPEC);
        assert_eq!(left_sidebar_max_width(-1.), SIDEBAR_MAX_WIDTH_SPEC);
        assert_eq!(left_sidebar_max_width(f32::NAN), SIDEBAR_MAX_WIDTH_SPEC);
        assert_eq!(
            left_sidebar_max_width(f32::INFINITY),
            SIDEBAR_MAX_WIDTH_SPEC
        );
    }

    #[test]
    fn bottom_pane_default_matches_the_macos_minimum_so_the_clamp_is_identity() {
        assert_eq!(
            BOTTOM_PANE_HEIGHT_SPEC, 260.,
            "默认高 = 真源 minimumGitPaneHeight（WorkbenchView.swift:2019）"
        );
        assert_eq!(
            BOTTOM_PANE_HEIGHT_SPEC, BOTTOM_PANE_MIN_HEIGHT_SPEC,
            "默认值必须等于下界，否则 clamp 会在首帧改掉它（改前 240 vs 260 就是那个不自洽）"
        );
        let (min, max) = (BOTTOM_PANE_MIN_HEIGHT_SPEC, bottom_pane_max_height(800.));
        assert_eq!(
            clamp_pane_span(BOTTOM_PANE_HEIGHT_SPEC, min, max),
            BOTTOM_PANE_HEIGHT_SPEC
        );
    }

    #[test]
    fn right_pane_default_is_macos_and_never_overrides_a_stored_width() {
        assert_eq!(
            RIGHT_TOOL_WINDOW_WIDTH_SPEC, 360.,
            "400 出自已作废的 windows/tauri（default-settings.ts:139），真源是 360"
        );
        let (min, max) = (
            RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC,
            right_tool_window_max_width(1920.),
        );
        assert_eq!(
            pane_span_on_restore(None, RIGHT_TOOL_WINDOW_WIDTH_SPEC, min, max),
            RIGHT_TOOL_WINDOW_WIDTH_SPEC
        );
        assert_eq!(pane_span_on_restore(Some(413.25), 360., min, max), 413.25);
        assert_eq!(pane_span_on_restore(Some(500.), 360., min, max), 500.);
        assert_eq!(pane_span_on_restore(Some(9_999.), 360., min, max), max);
        assert_eq!(pane_span_on_restore(Some(-5.), 360., min, max), min);
        assert_eq!(pane_span_on_restore(Some(f32::NAN), 360., min, max), min);
        assert_eq!(pane_span_on_restore(None, 320., 30., 520.), 320.);
        assert_eq!(pane_span_on_restore(None, 260., 260., 580.), 260.);
    }

    #[test]
    fn leading_edge_panes_grow_towards_the_window_edge() {
        let right = |press: f32, start: f32, current: f32| {
            clamped_drag_span(
                press,
                start,
                current,
                RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC,
                RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC,
                ResizablePane::RightToolWindow.drag_direction(),
            )
        };
        assert_eq!(right(500., 400., 440.), 460.);
        assert_eq!(right(500., 400., 900.), RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC);

        let bottom = |press: f32, start: f32, current: f32| {
            clamped_drag_span(
                press,
                start,
                current,
                BOTTOM_PANE_MIN_HEIGHT_SPEC,
                bottom_pane_max_height(800.),
                ResizablePane::BottomPane.drag_direction(),
            )
        };
        assert_eq!(bottom(600., 300., 500.), 400., "往上拖 100 ⇒ 高 400");
        assert_eq!(bottom(600., 300., 100.), bottom_pane_max_height(800.));
    }

    #[test]
    fn the_three_resize_handles_have_distinct_element_ids() {
        let ids = [
            ResizablePane::LeftSidebar.handle_id(),
            ResizablePane::RightToolWindow.handle_id(),
            ResizablePane::BottomPane.handle_id(),
        ];
        for (index, id) in ids.iter().enumerate() {
            assert!(!id.is_empty(), "ElementId 不能是空串");
            assert!(
                !ids[..index].contains(id),
                "ElementId 撞车：{id}（按住一个手柄可能触发另一个的拖动）"
            );
        }
        assert_eq!(
            ResizablePane::LeftSidebar.handle_edge(),
            HandleEdge::Trailing
        );
        assert_eq!(
            ResizablePane::RightToolWindow.handle_edge(),
            HandleEdge::Leading
        );
        assert_eq!(ResizablePane::BottomPane.handle_edge(), HandleEdge::Leading);
        assert_eq!(ResizablePane::BottomPane.axis(), Axis::Vertical);
        assert_eq!(ResizablePane::RightToolWindow.axis(), Axis::Horizontal);
    }

    #[test]
    fn the_right_pane_max_width_converges_on_a_narrow_window() {
        assert_eq!(
            right_tool_window_max_width(1920.),
            RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC
        );
        assert_eq!(
            right_tool_window_max_width(1280.),
            1280. - RIGHT_TOOL_WINDOW_MIN_WORKSPACE_WIDTH_SPEC
        );
        assert_eq!(
            right_tool_window_max_width(760.),
            RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC,
            "可用宽刚够放下 640 + 300 时，上界正好是下界"
        );
        assert_eq!(
            right_tool_window_max_width(200.),
            RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC,
            "窗口比两边的下界之和还窄：上界收敛到下界，不产生 min > max"
        );
        assert_eq!(
            right_tool_window_max_width(0.),
            RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC
        );
        assert_eq!(
            right_tool_window_max_width(f32::NAN),
            RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC
        );
    }

    #[test]
    fn spans_converge_for_narrow_windows_and_out_of_range_restores() {
        assert_eq!(
            bottom_pane_max_height(1200.),
            1200. - BOTTOM_PANE_MIN_CENTER_HEIGHT_SPEC
        );
        assert_eq!(
            bottom_pane_max_height(300.),
            BOTTOM_PANE_MIN_HEIGHT_SPEC,
            "窗口太矮：上界收到下界"
        );
        assert!(
            bottom_pane_max_height(0.).is_infinite(),
            "窗口尺寸还不可知时这一帧不设上界（别误吃恢复出来的值）"
        );

        let restored = clamp_pane_span(
            760.,
            BOTTOM_PANE_MIN_HEIGHT_SPEC,
            bottom_pane_max_height(800.),
        );
        assert_eq!(restored, bottom_pane_max_height(800.));

        assert_eq!(clamp_pane_span(-5., 260., 580.), 260.);
        assert_eq!(clamp_pane_span(f32::NAN, 260., 580.), 260.);
        assert_eq!(clamp_pane_span(f32::NEG_INFINITY, 260., 580.), 260.);
        assert_eq!(clamp_pane_span(f32::INFINITY, 260., 580.), 580.);
        assert_eq!(clamp_pane_span(500., 300., 120.), 300.);
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
            gpui_kit::Keystroke::parse(key)
                .unwrap_or_else(|error| panic!("{action:?} 的键位 {key:?} 解析不了：{error}"));
            assert!(listed.contains(&action), "{action:?} 没有画在任何菜单里");
            assert!(
                action.keybind_action().is_some(),
                "{action:?} 有键位却没有 Action —— 菜单上会显示一颗按不出的键"
            );
        }

        let with_key: Vec<&'static str> = listed
            .iter()
            .filter_map(|action| action.menu_key())
            .collect();
        assert_eq!(
            with_key,
            vec![
                crate::menu_bar::KEY_REOPEN_CLOSED_TAB,
                crate::menu_bar::KEY_TOGGLE_ACTIVITY_SIDEBAR,
                crate::menu_bar::KEY_TOGGLE_TERMINAL,
                crate::menu_bar::KEY_ZOOM_IN,
                crate::menu_bar::KEY_ZOOM_OUT,
                crate::menu_bar::KEY_RESET_ZOOM,
                crate::menu_bar::KEY_GO_TO_LINE,
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
            22,
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
                "open-file",
                "close-tab",
                "close-other-tabs",
                "close-all-tabs",
                "close-saved-tabs",
                "close-tabs-to-left",
                "close-tabs-to-right",
                "reopen-closed-tab",
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
            let action = super::command_action(id, dark(), true);
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
}
