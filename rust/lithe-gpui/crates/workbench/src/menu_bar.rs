use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::{
    App, AppContext as _, Bounds, ClickEvent, Context, Entity, FocusHandle, Focusable as _,
    InteractiveElement as _, IntoElement, KeyBinding, Keystroke, MouseButton, ParentElement as _,
    Pixels, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, div,
    rems,
};
use gpui::{DismissEvent, deferred};
use gpui_kit::assets::IconName;
use gpui_kit::base::{Align, ElementExt as _, Placement, Positioner, h_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _};

use lithe_gpui_shared::icons::{idea, idea_icon_mono};
use lithe_gpui_shared::tr;

use crate::command_palette::CommandId;

const ITEM_RADIUS_SPEC: f32 = lithe_gpui_shared::design::CONTROL_RADIUS;

const FLOAT_RADIUS_SPEC: f32 = 14.4;

const PANEL_MIN_WIDTH_SPEC: f32 = 240.;

const PANEL_OFFSET_SPEC: f32 = 4.;

const PANEL_WINDOW_MARGIN_SPEC: f32 = 8.;

fn is_escape(keystroke: &Keystroke) -> bool {
    keystroke.key == "escape" && keystroke.modifiers.number_of_modifiers() == 0
}

pub struct TopMenu {
    pub id: &'static str,
    pub label_key: &'static str,
    pub items: &'static [MenuItem],
}

impl TopMenu {
    pub fn label(&self) -> SharedString {
        tr(self.label_key)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Missing {
    pub id: &'static str,
    pub text_key: &'static str,
}

impl Missing {
    pub fn text(self) -> SharedString {
        tr(self.text_key)
    }

    pub fn diagnose(self, label_key: &str) {
        eprintln!(
            "S1_MENU notWired id={} missing={}",
            label_key.trim_start_matches("lithe."),
            self.id
        );
    }
}

pub const MISSING_EDITOR_ENTRY: Missing = Missing {
    id: "editor_entry",
    text_key: "lithe.gpui.menuMissing.editorEntry",
};

pub const MISSING_EDITOR_FEATURES: Missing = Missing {
    id: "editor_features",
    text_key: "lithe.gpui.menuMissing.editorFeatures",
};

pub const MISSING_LSP_REQUESTS: Missing = Missing {
    id: "lsp_requests",
    text_key: "lithe.gpui.menuMissing.lspRequests",
};

pub const MISSING_SUBSYSTEM: Missing = Missing {
    id: "subsystem",
    text_key: "lithe.gpui.menuMissing.subsystem",
};

pub const MISSING_GITHUB: Missing = Missing {
    id: "github_integration",
    text_key: "lithe.gpui.menuMissing.github",
};

pub const MISSING_UPDATER: Missing = Missing {
    id: "updater",
    text_key: "lithe.gpui.menuMissing.updater",
};

pub const MISSING_CLONE_UI: Missing = Missing {
    id: "clone_ui",
    text_key: "lithe.gpui.menuMissing.cloneUi",
};

pub const MISSING_NEW_PROJECT_SCAFFOLDING: Missing = Missing {
    id: "new_project_scaffolding",
    text_key: "lithe.gpui.menuMissing.newProjectScaffolding",
};

pub const MISSING_FILE_LIFECYCLE: Missing = Missing {
    id: "file_lifecycle",
    text_key: "lithe.gpui.menuMissing.fileLifecycle",
};

pub const MISSING_PROJECT_LIFECYCLE: Missing = Missing {
    id: "project_lifecycle",
    text_key: "lithe.gpui.menuMissing.projectLifecycle",
};

pub const MISSING_LOCAL_HISTORY: Missing = Missing {
    id: "local_history",
    text_key: "lithe.gpui.menuMissing.localHistory",
};

pub const MISSING_WINDOW_ROUTING: Missing = Missing {
    id: "window_routing",
    text_key: "lithe.gpui.menuMissing.windowRouting",
};

pub const MISSING_HELP_ABOUT: Missing = Missing {
    id: "help_about",
    text_key: "lithe.gpui.menuMissing.helpAbout",
};

pub const MISSING_SHORTCUTS_VIEW: Missing = Missing {
    id: "shortcuts_view",
    text_key: "lithe.gpui.menuMissing.shortcutsView",
};

pub const MISSING_TERMINAL_SPLIT: Missing = Missing {
    id: "terminal_split",
    text_key: "lithe.gpui.menuMissing.terminalSplit",
};

pub const MISSING_GROUPS: [Missing; 15] = [
    MISSING_EDITOR_ENTRY,
    MISSING_EDITOR_FEATURES,
    MISSING_LSP_REQUESTS,
    MISSING_SUBSYSTEM,
    MISSING_GITHUB,
    MISSING_UPDATER,
    MISSING_CLONE_UI,
    MISSING_NEW_PROJECT_SCAFFOLDING,
    MISSING_FILE_LIFECYCLE,
    MISSING_PROJECT_LIFECYCLE,
    MISSING_LOCAL_HISTORY,
    MISSING_WINDOW_ROUTING,
    MISSING_HELP_ABOUT,
    MISSING_SHORTCUTS_VIEW,
    MISSING_TERMINAL_SPLIT,
];

pub const NOT_WIRED_OUTSIDE_MENUS: [(&str, Missing); 2] = [
    ("lithe.titleProject.cloneRepository", MISSING_CLONE_UI),
    (
        "lithe.titleProject.newProject",
        MISSING_NEW_PROJECT_SCAFFOLDING,
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItem {
    Action(MenuAction),
    NotWired {
        label_key: &'static str,
        missing: Missing,
        icon: MenuIcon,
    },
    Separator,
    Theme,
}

impl MenuItem {
    pub const fn missing(&self) -> Option<Missing> {
        match self {
            Self::NotWired { missing, .. } => Some(*missing),
            Self::Action(_) | Self::Separator | Self::Theme => None,
        }
    }

    pub const fn label_key(&self) -> Option<&'static str> {
        match self {
            Self::Action(action) => Some(action.label_key()),
            Self::NotWired { label_key, .. } => Some(label_key),
            Self::Separator | Self::Theme => None,
        }
    }
}

pub const KEY_REOPEN_CLOSED_TAB: &str = "ctrl-shift-t";
pub const KEY_TOGGLE_ACTIVITY_SIDEBAR: &str = "ctrl-b";
pub const KEY_TOGGLE_TERMINAL: &str = "ctrl-j";
pub const KEY_ZOOM_IN: &str = "ctrl-=";
pub const KEY_ZOOM_OUT: &str = "ctrl--";
pub const KEY_RESET_ZOOM: &str = "ctrl-0";
pub const KEY_GO_TO_LINE: &str = "ctrl-g";

gpui_kit::actions!(
    lithe_workbench,
    [
        ReopenTab,
        ToggleSidebar,
        ToggleTerminalPane,
        ZoomIn,
        ZoomOut,
        ResetZoom,
        OpenGoToLine
    ]
);

fn push_menu_key(menu_action: MenuAction, key: &'static str, cx: &mut App) {
    let id = menu_action.label_key();
    let Some(bar) = try_handle() else {
        eprintln!("S1_MENU_KEY id={id} source={key} state=no_menu_bar");
        return;
    };
    let _ = bar.update(cx, |bar, cx| bar.push_run(menu_action, cx));
    eprintln!("S1_MENU_KEY id={id} source={key} state=pushed");
}

fn install_menu_key<A: gpui_kit::Action>(
    cx: &mut App,
    key: &'static str,
    action: A,
    menu_action: MenuAction,
) {
    cx.bind_keys([KeyBinding::new(key, action, None)]);
    cx.on_action(move |_: &A, cx: &mut App| push_menu_key(menu_action, key, cx));
}

pub fn install_key_actions(cx: &mut App) {
    install_menu_key(
        cx,
        KEY_REOPEN_CLOSED_TAB,
        ReopenTab,
        MenuAction::ReopenClosedTab,
    );
    install_menu_key(
        cx,
        KEY_TOGGLE_ACTIVITY_SIDEBAR,
        ToggleSidebar,
        MenuAction::ToggleActivitySidebar,
    );
    install_menu_key(
        cx,
        KEY_TOGGLE_TERMINAL,
        ToggleTerminalPane,
        MenuAction::ToggleTerminal,
    );
    install_menu_key(cx, KEY_ZOOM_IN, ZoomIn, MenuAction::ZoomIn);
    install_menu_key(cx, KEY_ZOOM_OUT, ZoomOut, MenuAction::ZoomOut);
    install_menu_key(cx, KEY_RESET_ZOOM, ResetZoom, MenuAction::ResetZoom);
    install_menu_key(cx, KEY_GO_TO_LINE, OpenGoToLine, MenuAction::GoToLine);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    OpenFolder,
    OpenFile,
    Save,
    CloseTab,
    CloseOtherTabs,
    CloseAllTabs,
    CloseSavedTabs,
    CloseTabsToLeft,
    CloseTabsToRight,
    ReopenClosedTab,
    CommandPalette,
    ToggleTerminal,
    ToggleMaven,
    ToggleStatusBar,
    OpenAppearanceSettings,
    GoToDefinition,
    QuickOpen,
    SearchEverywhere,
    ReplaceInFiles,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Find,
    FindAndReplace,
    QuickFix,
    ToggleActivitySidebar,
    ToggleSecondarySidebar,
    ShowDiagnostics,
    ShowFileExplorer,
    ShowSourceControl,
    ShowRunAndDebug,
    StartDebugging,
    StopDebugging,
    ToggleBreakpoint,
    GoToLine,
    ToggleWordWrap,
    ShowHover,
    ZoomIn,
    ZoomOut,
    ResetZoom,
    ToggleMenuBar,
    NewTerminalTab,
    CloseTerminalTab,
    NewWindow,
    Preferences,
    Minimize,
    Maximize,
    ToggleFullscreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuIcon {
    Idea(&'static idea::IdeaIcon),
    Lucide(IconName),
}

impl MenuAction {
    pub const fn label_key(self) -> &'static str {
        match self {
            Self::OpenFolder => "lithe.menu.openFolder",
            Self::OpenFile => "lithe.outline.openFile",
            Self::Save => "lithe.menu.save",
            Self::CloseTab => "lithe.menu.closeTab",
            Self::CloseOtherTabs => "lithe.menu.closeOtherTabs",
            Self::CloseAllTabs => "lithe.menu.closeAllTabs",
            Self::CloseSavedTabs => "lithe.menu.closeSavedTabs",
            Self::CloseTabsToLeft => "lithe.menu.closeTabsToLeft",
            Self::CloseTabsToRight => "lithe.menu.closeTabsToRight",
            Self::ReopenClosedTab => "lithe.menu.reopenClosedTab",
            Self::CommandPalette => "lithe.menu.commandPalette",
            Self::ToggleTerminal => "lithe.menu.toggleTerminal",
            Self::ToggleMaven => "lithe.workbench.maven",
            Self::ToggleStatusBar => "lithe.settings.appearance.showStatusBar",
            Self::OpenAppearanceSettings => "lithe.settings.tabs.appearance",
            Self::GoToDefinition => "lithe.menu.goToDefinition",
            Self::QuickOpen => "lithe.menu.quickOpen",
            Self::SearchEverywhere => "lithe.menu.globalSearch",
            Self::ReplaceInFiles => "lithe.search.gpui.replaceInFiles",
            Self::NewTerminalTab => "lithe.menu.newTerminal",
            Self::CloseTerminalTab => "lithe.menu.closeTerminal",
            Self::NewWindow => "lithe.menu.newWindow",
            Self::Preferences => "lithe.menu.preferences",
            Self::Minimize => "lithe.menu.minimize",
            Self::Maximize => "lithe.menu.maximize",
            Self::ToggleFullscreen => "lithe.menu.toggleFullscreen",
            Self::Undo => "lithe.menu.undo",
            Self::Redo => "lithe.menu.redo",
            Self::Cut => "lithe.menu.cut",
            Self::Copy => "lithe.menu.copy",
            Self::Paste => "lithe.menu.paste",
            Self::SelectAll => "lithe.menu.selectAll",
            Self::Find => "lithe.menu.find",
            Self::FindAndReplace => "lithe.menu.findAndReplace",
            Self::QuickFix => "lithe.menu.quickFix",
            Self::ToggleActivitySidebar => "lithe.menu.toggleActivitySidebar",
            Self::ToggleSecondarySidebar => "lithe.menu.toggleSecondarySidebar",
            Self::ShowDiagnostics => "lithe.menu.diagnostics",
            Self::ShowFileExplorer => "lithe.menu.fileExplorer",
            Self::ShowSourceControl => "lithe.menu.sourceControl",
            Self::ShowRunAndDebug => "lithe.menu.runAndDebug",
            Self::StartDebugging => "lithe.menu.startDebugging",
            Self::StopDebugging => "lithe.menu.stopDebugging",
            Self::ToggleBreakpoint => "lithe.menu.toggleBreakpoint",
            Self::GoToLine => "lithe.menu.goToLine",
            Self::ToggleWordWrap => "lithe.menu.toggleWordWrap",
            Self::ShowHover => "lithe.menu.showHover",
            Self::ZoomIn => "lithe.menu.zoomIn",
            Self::ZoomOut => "lithe.menu.zoomOut",
            Self::ResetZoom => "lithe.menu.resetZoom",
            Self::ToggleMenuBar => "lithe.menu.toggleMenuBar",
        }
    }

    pub fn label(self) -> SharedString {
        tr(self.label_key())
    }

    pub fn icon(self, is_dark: bool) -> gpui_kit::component::Icon {
        match self {
            Self::OpenFolder => idea_icon_mono(&idea::FOLDER_ICON, is_dark),
            Self::OpenFile => idea_icon_mono(&idea::FILE_ICON, is_dark),
            Self::Save => idea_icon_mono(&idea::FLOPPY_DISK_ICON, is_dark),
            Self::CloseTab => idea_icon_mono(&idea::X_ICON, is_dark),
            Self::CloseOtherTabs => idea_icon_mono(&idea::FILES_ICON, is_dark),
            Self::CloseAllTabs => Icon::new(IconName::SquareX),
            Self::CloseSavedTabs => Icon::new(IconName::SaveCheck),
            Self::CloseTabsToLeft => Icon::new(IconName::PanelLeft),
            Self::CloseTabsToRight => Icon::new(IconName::PanelRight),
            Self::ReopenClosedTab => idea_icon_mono(&idea::REFRESH_ICON, is_dark),
            Self::CommandPalette => idea_icon_mono(&idea::MAGNIFYING_GLASS_ICON, is_dark),
            Self::ToggleTerminal | Self::NewTerminalTab | Self::CloseTerminalTab => {
                Icon::new(IconName::SquareTerminal)
            }
            Self::ToggleStatusBar => Icon::new(IconName::PanelBottom),
            Self::ToggleMaven => Icon::new(IconName::Package),
            Self::OpenAppearanceSettings | Self::Preferences => {
                idea_icon_mono(&idea::GEAR_ICON, is_dark)
            }
            Self::GoToDefinition => idea_icon_mono(&idea::ARROW_RIGHT_ICON, is_dark),
            Self::QuickOpen => idea_icon_mono(&idea::MAGNIFYING_GLASS_ICON, is_dark),
            Self::SearchEverywhere => idea_icon_mono(&idea::GLOBE_HEMISPHERE_WEST_ICON, is_dark),
            Self::ReplaceInFiles => Icon::new(IconName::Replace),
            Self::NewWindow => Icon::new(IconName::AppWindow),
            Self::Minimize => Icon::new(IconName::WindowMinimize),
            Self::Maximize => Icon::new(IconName::WindowMaximize),
            Self::ToggleFullscreen => Icon::new(IconName::Maximize),
            Self::Undo => idea_icon_mono(&idea::ARROW_COUNTER_CLOCKWISE_ICON, is_dark),
            Self::Redo => idea_icon_mono(&idea::ARROW_CLOCKWISE_ICON, is_dark),
            Self::Cut => idea_icon_mono(&idea::SCISSORS_ICON, is_dark),
            Self::Copy => idea_icon_mono(&idea::COPY_ICON, is_dark),
            Self::Paste => Icon::new(IconName::Clipboard),
            Self::SelectAll => Icon::new(IconName::SquareDashed),
            Self::Find => idea_icon_mono(&idea::MAGNIFYING_GLASS_ICON, is_dark),
            Self::FindAndReplace => Icon::new(IconName::Replace),
            Self::QuickFix => idea_icon_mono(&idea::LIGHTBULB_ICON, is_dark),
            Self::ToggleActivitySidebar => Icon::new(IconName::PanelLeft),
            Self::ToggleSecondarySidebar => Icon::new(IconName::PanelRight),
            Self::ShowDiagnostics => Icon::new(IconName::SquareActivity),
            Self::ShowFileExplorer => idea_icon_mono(&idea::FILES_ICON, is_dark),
            Self::ShowSourceControl => idea_icon_mono(&idea::GIT_BRANCH_ICON, is_dark),
            Self::ShowRunAndDebug | Self::StartDebugging => {
                idea_icon_mono(&idea::PLAY_ICON, is_dark)
            }
            Self::StopDebugging => idea_icon_mono(&idea::STOP_ICON, is_dark),
            Self::ToggleBreakpoint => Icon::new(IconName::CircleDot),
            Self::GoToLine => idea_icon_mono(&idea::HASH_ICON, is_dark),
            Self::ToggleWordWrap => Icon::new(IconName::TextWrap),
            Self::ShowHover => idea_icon_mono(&idea::INFO_ICON, is_dark),
            Self::ZoomIn => idea_icon_mono(&idea::MAGNIFYING_GLASS_PLUS_ICON, is_dark),
            Self::ZoomOut => idea_icon_mono(&idea::MAGNIFYING_GLASS_MINUS_ICON, is_dark),
            Self::ResetZoom => idea_icon_mono(&idea::REFRESH_ICON, is_dark),
            Self::ToggleMenuBar => idea_icon_mono(&idea::FILES_ICON, is_dark),
        }
    }

    pub const fn menu_key(self) -> Option<&'static str> {
        match self {
            Self::ReopenClosedTab => Some(KEY_REOPEN_CLOSED_TAB),
            Self::ToggleActivitySidebar => Some(KEY_TOGGLE_ACTIVITY_SIDEBAR),
            Self::ToggleTerminal => Some(KEY_TOGGLE_TERMINAL),
            Self::ZoomIn => Some(KEY_ZOOM_IN),
            Self::ZoomOut => Some(KEY_ZOOM_OUT),
            Self::ResetZoom => Some(KEY_RESET_ZOOM),
            Self::GoToLine => Some(KEY_GO_TO_LINE),
            _ => None,
        }
    }

    pub fn keybind_action(self) -> Option<Box<dyn gpui_kit::Action>> {
        use crate::command_palette::{OpenCommandPalette, ToggleMenuBar as ToggleMenuBarAction};

        Some(match self {
            Self::OpenFolder => Box::new(crate::workspace::OpenProjectFolder),
            Self::Save => Box::new(lithe_gpui_editor::SaveBuffer),
            Self::CloseTab => Box::new(lithe_gpui_editor::CloseActiveTab),
            Self::GoToDefinition => Box::new(lithe_gpui_editor::NavigateToDefinition),
            Self::Preferences => Box::new(lithe_gpui_settings::OpenSettings),
            Self::CommandPalette => Box::new(OpenCommandPalette),
            Self::ToggleMenuBar => Box::new(ToggleMenuBarAction),
            Self::QuickOpen => Box::new(crate::quick_open::OpenQuickOpen),
            Self::SearchEverywhere => return None,
            Self::ReplaceInFiles => Box::new(crate::replace_in_files::OpenReplaceInFiles),
            Self::ReopenClosedTab => Box::new(ReopenTab),
            Self::ToggleActivitySidebar => Box::new(ToggleSidebar),
            Self::ToggleTerminal => Box::new(ToggleTerminalPane),
            Self::ZoomIn => Box::new(ZoomIn),
            Self::ZoomOut => Box::new(ZoomOut),
            Self::ResetZoom => Box::new(ResetZoom),
            Self::Undo => Box::new(gpui_kit::base::input::Undo),
            Self::Redo => Box::new(gpui_kit::base::input::Redo),
            Self::Cut => Box::new(gpui_kit::base::input::Cut),
            Self::Copy => Box::new(gpui_kit::base::input::Copy),
            Self::Paste => Box::new(gpui_kit::base::input::Paste),
            Self::SelectAll => Box::new(gpui_kit::base::input::SelectAll),
            Self::Find => Box::new(gpui_kit::base::input::Search),
            Self::FindAndReplace => Box::new(gpui_kit::base::input::Replace),
            Self::QuickFix => Box::new(gpui_kit::base::input::ToggleCodeActions),
            Self::OpenFile
            | Self::CloseOtherTabs
            | Self::CloseAllTabs
            | Self::CloseSavedTabs
            | Self::CloseTabsToLeft
            | Self::CloseTabsToRight
            | Self::ToggleMaven
            | Self::ToggleStatusBar
            | Self::OpenAppearanceSettings
            | Self::NewTerminalTab
            | Self::CloseTerminalTab
            | Self::NewWindow
            | Self::Minimize
            | Self::Maximize
            | Self::ToggleFullscreen => return None,
            Self::ToggleSecondarySidebar
            | Self::ShowDiagnostics
            | Self::ShowFileExplorer
            | Self::ShowSourceControl
            | Self::ShowRunAndDebug
            | Self::StartDebugging
            | Self::StopDebugging
            | Self::ToggleBreakpoint => return None,
            Self::GoToLine => Box::new(OpenGoToLine),
            Self::ToggleWordWrap | Self::ShowHover => return None,
        })
    }

    pub fn command_id(self) -> Option<CommandId> {
        match self {
            Self::OpenFolder => None,
            Self::OpenFile => Some(CommandId::OpenFile),
            Self::Save => Some(CommandId::SaveBuffer),
            Self::CloseTab => Some(CommandId::CloseTab),
            Self::CloseOtherTabs => Some(CommandId::CloseOtherTabs),
            Self::CloseAllTabs => Some(CommandId::CloseAllTabs),
            Self::CloseSavedTabs => Some(CommandId::CloseSavedTabs),
            Self::CloseTabsToLeft => Some(CommandId::CloseTabsToLeft),
            Self::CloseTabsToRight => Some(CommandId::CloseTabsToRight),
            Self::ReopenClosedTab => Some(CommandId::ReopenClosedTab),
            Self::CommandPalette => Some(CommandId::OpenCommandPalette),
            Self::ToggleTerminal => Some(CommandId::ToggleTerminal),
            Self::ToggleMaven => Some(CommandId::ToggleMaven),
            Self::ToggleStatusBar => Some(CommandId::ToggleStatusBar),
            Self::OpenAppearanceSettings => Some(CommandId::OpenAppearanceSettings),
            Self::GoToDefinition => Some(CommandId::NavigateToDefinition),
            Self::QuickOpen => Some(CommandId::OpenQuickOpen),
            Self::SearchEverywhere => Some(CommandId::OpenSearchEverywhere),
            Self::ReplaceInFiles => Some(CommandId::OpenReplaceInFiles),
            Self::NewTerminalTab
            | Self::CloseTerminalTab
            | Self::Preferences
            | Self::Minimize
            | Self::Maximize
            | Self::ToggleFullscreen => None,
            Self::NewWindow => Some(CommandId::NewWindow),
            Self::ToggleMenuBar => Some(CommandId::ToggleMenuBar),
            Self::Undo
            | Self::Redo
            | Self::Cut
            | Self::Copy
            | Self::Paste
            | Self::SelectAll
            | Self::Find
            | Self::FindAndReplace
            | Self::QuickFix
            | Self::ToggleActivitySidebar
            | Self::ToggleSecondarySidebar
            | Self::ShowDiagnostics
            | Self::ShowFileExplorer
            | Self::ShowSourceControl
            | Self::ShowRunAndDebug
            | Self::StartDebugging
            | Self::StopDebugging
            | Self::ToggleBreakpoint
            | Self::ZoomIn
            | Self::ZoomOut
            | Self::ResetZoom
            | Self::GoToLine
            | Self::ToggleWordWrap
            | Self::ShowHover => None,
        }
    }
}

pub static MENUS: &[TopMenu] = &[
    TopMenu {
        id: "file",
        label_key: "lithe.menu.file",
        items: &[
            MenuItem::NotWired {
                label_key: "lithe.menu.newTab",
                missing: MISSING_FILE_LIFECYCLE,
                icon: MenuIcon::Idea(&idea::FILE_PLUS_ICON),
            },
            MenuItem::Action(MenuAction::NewWindow),
            MenuItem::NotWired {
                label_key: "lithe.menu.newFile",
                missing: MISSING_FILE_LIFECYCLE,
                icon: MenuIcon::Lucide(IconName::FilePlusCorner),
            },
            MenuItem::Action(MenuAction::OpenFolder),
            MenuItem::Action(MenuAction::OpenFile),
            MenuItem::NotWired {
                label_key: "lithe.menu.closeFolder",
                missing: MISSING_PROJECT_LIFECYCLE,
                icon: MenuIcon::Lucide(IconName::FolderX),
            },
            MenuItem::Separator,
            MenuItem::Action(MenuAction::Save),
            MenuItem::NotWired {
                label_key: "lithe.menu.saveAs",
                missing: MISSING_EDITOR_ENTRY,
                icon: MenuIcon::Lucide(IconName::SavePen),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.saveAll",
                missing: MISSING_EDITOR_ENTRY,
                icon: MenuIcon::Lucide(IconName::SaveAll),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.revertFile",
                missing: MISSING_EDITOR_ENTRY,
                icon: MenuIcon::Idea(&idea::REFRESH_ICON),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.showLocalHistory",
                missing: MISSING_LOCAL_HISTORY,
                icon: MenuIcon::Idea(&idea::CLOCK_COUNTER_CLOCKWISE_ICON),
            },
            MenuItem::Separator,
            MenuItem::Action(MenuAction::CloseTab),
            MenuItem::NotWired {
                label_key: "lithe.menu.closeWindow",
                missing: MISSING_WINDOW_ROUTING,
                icon: MenuIcon::Lucide(IconName::WindowClose),
            },
            MenuItem::Separator,
            MenuItem::Action(MenuAction::CloseOtherTabs),
            MenuItem::Action(MenuAction::CloseAllTabs),
            MenuItem::Action(MenuAction::CloseSavedTabs),
            MenuItem::Action(MenuAction::CloseTabsToLeft),
            MenuItem::Action(MenuAction::CloseTabsToRight),
            MenuItem::Action(MenuAction::ReopenClosedTab),
            MenuItem::NotWired {
                label_key: "lithe.menu.quit",
                missing: MISSING_WINDOW_ROUTING,
                icon: MenuIcon::Lucide(IconName::Power),
            },
        ],
    },
    TopMenu {
        id: "edit",
        label_key: "lithe.menu.edit",
        items: &[
            MenuItem::Action(MenuAction::Undo),
            MenuItem::Action(MenuAction::Redo),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::Cut),
            MenuItem::Action(MenuAction::Copy),
            MenuItem::Action(MenuAction::Paste),
            MenuItem::Action(MenuAction::SelectAll),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::Find),
            MenuItem::Action(MenuAction::FindAndReplace),
            MenuItem::Separator,
            MenuItem::NotWired {
                label_key: "lithe.menu.toggleComment",
                missing: MISSING_EDITOR_FEATURES,
                icon: MenuIcon::Lucide(IconName::Code),
            },
            MenuItem::Action(MenuAction::QuickFix),
            MenuItem::NotWired {
                label_key: "lithe.menu.triggerParameterHints",
                missing: MISSING_LSP_REQUESTS,
                icon: MenuIcon::Lucide(IconName::Parentheses),
            },
            MenuItem::Action(MenuAction::ShowHover),
            MenuItem::Separator,
            MenuItem::NotWired {
                label_key: "lithe.menu.duplicateLine",
                missing: MISSING_EDITOR_FEATURES,
                icon: MenuIcon::Lucide(IconName::CopyPlus),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.deleteLine",
                missing: MISSING_EDITOR_FEATURES,
                icon: MenuIcon::Idea(&idea::TRASH_ICON),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.moveLineUp",
                missing: MISSING_EDITOR_FEATURES,
                icon: MenuIcon::Idea(&idea::ARROW_UP_ICON),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.moveLineDown",
                missing: MISSING_EDITOR_FEATURES,
                icon: MenuIcon::Idea(&idea::ARROW_DOWN_ICON),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.formatDocument",
                missing: MISSING_LSP_REQUESTS,
                icon: MenuIcon::Lucide(IconName::Pilcrow),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.formatSelection",
                missing: MISSING_LSP_REQUESTS,
                icon: MenuIcon::Lucide(IconName::SquarePilcrow),
            },
            MenuItem::Action(MenuAction::CommandPalette),
        ],
    },
    TopMenu {
        id: "view",
        label_key: "lithe.menu.view",
        items: &[
            MenuItem::Action(MenuAction::ToggleActivitySidebar),
            MenuItem::Action(MenuAction::ToggleSecondarySidebar),
            MenuItem::Action(MenuAction::ToggleTerminal),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::SearchEverywhere),
            MenuItem::Action(MenuAction::ShowDiagnostics),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::ShowFileExplorer),
            MenuItem::Action(MenuAction::ShowSourceControl),
            MenuItem::NotWired {
                label_key: "lithe.menu.github",
                missing: MISSING_GITHUB,
                icon: MenuIcon::Lucide(IconName::Github),
            },
            MenuItem::Action(MenuAction::ShowRunAndDebug),
            MenuItem::Separator,
            MenuItem::NotWired {
                label_key: "lithe.menu.splitEditor",
                missing: MISSING_EDITOR_FEATURES,
                icon: MenuIcon::Idea(&idea::COLUMNS_ICON),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.toggleMinimap",
                missing: MISSING_EDITOR_FEATURES,
                icon: MenuIcon::Lucide(IconName::Map),
            },
            MenuItem::Action(MenuAction::ToggleWordWrap),
            MenuItem::NotWired {
                label_key: "lithe.menu.toggleLineNumbers",
                missing: MISSING_EDITOR_FEATURES,
                icon: MenuIcon::Lucide(IconName::ListOrdered),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.toggleRenderWhitespace",
                missing: MISSING_EDITOR_FEATURES,
                icon: MenuIcon::Lucide(IconName::Space),
            },
            MenuItem::Separator,
            MenuItem::Action(MenuAction::ZoomIn),
            MenuItem::Action(MenuAction::ZoomOut),
            MenuItem::Action(MenuAction::ResetZoom),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::ToggleStatusBar),
            MenuItem::Separator,
            MenuItem::Theme,
        ],
    },
    TopMenu {
        id: "go",
        label_key: "lithe.menu.go",
        items: &[
            MenuItem::Action(MenuAction::QuickOpen),
            MenuItem::Action(MenuAction::ReplaceInFiles),
            MenuItem::Action(MenuAction::GoToLine),
            MenuItem::Separator,
            MenuItem::NotWired {
                label_key: "lithe.menu.goBack",
                missing: MISSING_EDITOR_ENTRY,
                icon: MenuIcon::Idea(&idea::ARROW_LEFT_ICON),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.goForward",
                missing: MISSING_EDITOR_ENTRY,
                icon: MenuIcon::Idea(&idea::ARROW_RIGHT_ICON),
            },
            MenuItem::Separator,
            MenuItem::Action(MenuAction::GoToDefinition),
            MenuItem::NotWired {
                label_key: "lithe.menu.goToImplementation",
                missing: MISSING_LSP_REQUESTS,
                icon: MenuIcon::Lucide(IconName::ArrowUpRight),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.goToTypeDefinition",
                missing: MISSING_LSP_REQUESTS,
                icon: MenuIcon::Lucide(IconName::FileType),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.goToReferences",
                missing: MISSING_LSP_REQUESTS,
                icon: MenuIcon::Lucide(IconName::Network),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.renameSymbol",
                missing: MISSING_LSP_REQUESTS,
                icon: MenuIcon::Lucide(IconName::Signature),
            },
            MenuItem::Separator,
            MenuItem::NotWired {
                label_key: "lithe.menu.nextTab",
                missing: MISSING_EDITOR_ENTRY,
                icon: MenuIcon::Idea(&idea::CARET_RIGHT_ICON),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.previousTab",
                missing: MISSING_EDITOR_ENTRY,
                icon: MenuIcon::Idea(&idea::CARET_LEFT_ICON),
            },
        ],
    },
    TopMenu {
        id: "terminal",
        label_key: "lithe.menu.terminal",
        items: &[
            MenuItem::Action(MenuAction::NewTerminalTab),
            MenuItem::NotWired {
                label_key: "lithe.menu.splitTerminalRight",
                missing: MISSING_TERMINAL_SPLIT,
                icon: MenuIcon::Idea(&idea::COLUMNS_ICON),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.splitTerminalDown",
                missing: MISSING_TERMINAL_SPLIT,
                icon: MenuIcon::Idea(&idea::ROWS_ICON),
            },
            MenuItem::Action(MenuAction::CloseTerminalTab),
        ],
    },
    TopMenu {
        id: "run",
        label_key: "lithe.menu.run",
        items: &[
            MenuItem::Action(MenuAction::StartDebugging),
            MenuItem::Action(MenuAction::StopDebugging),
            MenuItem::Action(MenuAction::ToggleBreakpoint),
        ],
    },
    TopMenu {
        id: "tools",
        label_key: "lithe.menu.tools",
        items: &[
            MenuItem::NotWired {
                label_key: "lithe.menu.databases",
                missing: MISSING_SUBSYSTEM,
                icon: MenuIcon::Idea(&idea::DATABASE_ICON),
            },
            MenuItem::Separator,
            MenuItem::NotWired {
                label_key: "lithe.menu.webInspector",
                missing: MISSING_SUBSYSTEM,
                icon: MenuIcon::Idea(&idea::MONITOR_ICON),
            },
            MenuItem::Separator,
            MenuItem::Action(MenuAction::Preferences),
            MenuItem::NotWired {
                label_key: "lithe.menu.keyboardShortcuts",
                missing: MISSING_SHORTCUTS_VIEW,
                icon: MenuIcon::Idea(&idea::KEYBOARD_ICON),
            },
            MenuItem::Action(MenuAction::OpenAppearanceSettings),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::ToggleMaven),
        ],
    },
    TopMenu {
        id: "window",
        label_key: "lithe.menu.window",
        items: &[
            MenuItem::Action(MenuAction::Minimize),
            MenuItem::Action(MenuAction::Maximize),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::ToggleMenuBar),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::ToggleFullscreen),
        ],
    },
    TopMenu {
        id: "help",
        label_key: "lithe.menu.help",
        items: &[
            MenuItem::NotWired {
                label_key: "lithe.menu.documentation",
                missing: MISSING_HELP_ABOUT,
                icon: MenuIcon::Idea(&idea::BOOK_OPEN_ICON),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.keyboardShortcuts",
                missing: MISSING_SHORTCUTS_VIEW,
                icon: MenuIcon::Idea(&idea::KEYBOARD_ICON),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.whatsNew",
                missing: MISSING_HELP_ABOUT,
                icon: MenuIcon::Lucide(IconName::Sparkles),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.changelog",
                missing: MISSING_HELP_ABOUT,
                icon: MenuIcon::Lucide(IconName::ScrollText),
            },
            MenuItem::Separator,
            MenuItem::NotWired {
                label_key: "lithe.menu.reportBug",
                missing: MISSING_HELP_ABOUT,
                icon: MenuIcon::Idea(&idea::BUG_ICON),
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.requestFeature",
                missing: MISSING_HELP_ABOUT,
                icon: MenuIcon::Idea(&idea::LIGHTBULB_ICON),
            },
            MenuItem::Separator,
            MenuItem::NotWired {
                label_key: "lithe.menu.checkForUpdates",
                missing: MISSING_UPDATER,
                icon: MenuIcon::Idea(&idea::DOWNLOAD_ICON),
            },
        ],
    },
];

pub fn action_count() -> usize {
    MENUS
        .iter()
        .flat_map(|menu| menu.items.iter())
        .filter(|item| matches!(item, MenuItem::Action(_)))
        .count()
}

pub fn not_wired_count() -> usize {
    MENUS
        .iter()
        .flat_map(|menu| menu.items.iter())
        .filter(|item| item.missing().is_some())
        .count()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuBarMode {
    Pinned,
    Compact,
}

impl MenuBarMode {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Pinned => "pinned",
            Self::Compact => "compact",
        }
    }

    pub fn diagnose(self) {
        eprintln!(
            "S1_MENU_BAR mode={} items={} actions={} notWired={}",
            self.id(),
            MENUS.len(),
            action_count(),
            not_wired_count()
        );
    }
}

pub fn diagnose_open(id: &str) {
    eprintln!("S1_MENU_OPEN id={id}");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseReason {
    Dismiss,
    Toggle,
    Escape,
    Select,
    Switch,
}

impl CloseReason {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Dismiss => "dismiss",
            Self::Toggle => "toggle",
            Self::Escape => "escape",
            Self::Select => "select",
            Self::Switch => "switch",
        }
    }
}

pub fn diagnose_close(reason: CloseReason) {
    eprintln!("S1_MENU_CLOSE reason={}", reason.id());
}

pub fn diagnose_run(action: MenuAction, state: &str) {
    eprintln!(
        "S1_MENU_RUN id={} state={state}",
        action.label_key().trim_start_matches("lithe.")
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavState {
    Closed,
    Open(usize),
}

pub fn resolve_menu_toggle(prev: NavState, index: usize) -> NavState {
    if index >= MENUS.len() {
        return prev;
    }
    match prev {
        NavState::Open(current) if current == index => NavState::Closed,
        _ => NavState::Open(index),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuRequest {
    Run(MenuAction),
    NotWired {
        label_key: &'static str,
        missing: Missing,
    },
}

pub struct MenuBar {
    mode: MenuBarMode,
    selected: Option<usize>,
    compact_visible: bool,
    popup: Option<Entity<PopupMenu>>,
    dismiss_subscription: Option<Subscription>,
    trigger_bounds: Rc<RefCell<Vec<Option<Bounds<Pixels>>>>>,
    compact_trigger_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    restore_focus: bool,
    pending: Vec<MenuRequest>,
    action_context: FocusHandle,
    shell: Option<gpui_kit::WeakEntity<crate::workspace::ShellWorkspace>>,
    escape_interceptor: Option<Subscription>,
}

impl MenuBar {
    pub fn new(mode: MenuBarMode, action_context: FocusHandle, cx: &mut App) -> Entity<Self> {
        mode.diagnose();
        let shell = MENU_BAR_SHELL.with(|slot| slot.borrow().clone());
        let bar = cx.new(|_| Self {
            mode,
            selected: None,
            compact_visible: false,
            popup: None,
            dismiss_subscription: None,
            trigger_bounds: Rc::new(RefCell::new(vec![None; MENUS.len()])),
            compact_trigger_bounds: Rc::new(Cell::new(None)),
            restore_focus: false,
            pending: Vec::new(),
            action_context,
            shell,
            escape_interceptor: None,
        });
        let handle = bar.downgrade();
        bar.update(cx, |bar, cx| {
            bar.escape_interceptor = Some(cx.intercept_keystrokes(move |event, _window, cx| {
                if !is_escape(&event.keystroke) {
                    return;
                }
                let closed = handle
                    .update(cx, |bar, cx| bar.close(CloseReason::Escape, cx))
                    .unwrap_or(false);
                if closed {
                    cx.stop_propagation();
                }
            }));
        });
        bar
    }

    fn notify_shell(&self, cx: &mut App) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let _ = shell.update(cx, |_, cx| cx.notify());
    }

    pub fn mode(&self) -> MenuBarMode {
        self.mode
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn compact_visible(&self) -> bool {
        self.compact_visible
    }

    pub fn toggle_mode(&mut self, cx: &mut App) {
        self.mode = match self.mode {
            MenuBarMode::Pinned => MenuBarMode::Compact,
            MenuBarMode::Compact => MenuBarMode::Pinned,
        };
        let had_popup = self.popup.take().is_some();
        self.dismiss_subscription = None;
        self.selected = None;
        self.compact_visible = false;
        if had_popup {
            self.restore_focus = true;
        }
        self.mode.diagnose();
        self.notify_shell(cx);
    }

    pub fn toggle(&mut self, index: usize, cx: &mut App) {
        let Some(menu) = MENUS.get(index) else {
            return;
        };
        let next = resolve_menu_toggle(self.nav_state(), index);
        match next {
            NavState::Closed => {
                self.close(CloseReason::Toggle, cx);
            }
            NavState::Open(opened) => {
                self.popup = None;
                self.dismiss_subscription = None;
                self.compact_visible = true;
                self.selected = Some(opened);
                diagnose_open(menu.id);
                self.notify_shell(cx);
            }
        }
    }

    fn nav_state(&self) -> NavState {
        match self.selected {
            Some(index) => NavState::Open(index),
            None => NavState::Closed,
        }
    }

    pub fn close(&mut self, reason: CloseReason, cx: &mut App) -> bool {
        if self.selected.is_none() && !self.compact_visible {
            return false;
        }
        let had_popup = self.popup.take().is_some();
        self.dismiss_subscription = None;
        self.selected = None;
        self.compact_visible = false;
        if had_popup {
            self.restore_focus = true;
        }
        diagnose_close(reason);
        self.notify_shell(cx);
        true
    }

    fn handle_dismiss(
        &mut self,
        _popup: &Entity<PopupMenu>,
        _event: &DismissEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.selected {
            let on_own_trigger = self
                .trigger_bounds_of(index)
                .is_some_and(|bounds| bounds.contains(&window.mouse_position()));
            if on_own_trigger {
                return;
            }
        }
        self.close(CloseReason::Dismiss, cx);
    }

    fn trigger_bounds_of(&self, index: usize) -> Option<Bounds<Pixels>> {
        self.trigger_bounds.borrow().get(index).copied().flatten()
    }

    fn hover_switch(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(current) = self.selected else {
            return;
        };
        if current == index || MENUS.get(index).is_none() {
            return;
        }
        self.close(CloseReason::Switch, cx);
        self.toggle(index, cx);
    }

    pub fn toggle_compact(&mut self, cx: &mut App) {
        if self.compact_visible {
            self.close(CloseReason::Toggle, cx);
        } else {
            self.compact_visible = true;
            self.selected = None;
            diagnose_open("compact_bar");
            self.notify_shell(cx);
        }
    }

    pub(crate) fn push_run(&mut self, action: MenuAction, cx: &mut App) {
        self.pending.push(MenuRequest::Run(action));
        self.close(CloseReason::Select, cx);
    }

    pub(crate) fn push_not_wired(
        &mut self,
        label_key: &'static str,
        missing: Missing,
        cx: &mut App,
    ) {
        self.pending
            .push(MenuRequest::NotWired { label_key, missing });
        self.close(CloseReason::Select, cx);
    }

    pub fn drain_runs(&mut self) -> Vec<MenuRequest> {
        std::mem::take(&mut self.pending)
    }

    pub(crate) fn remember_action_target(&mut self, focused: Option<FocusHandle>) {
        let target = focused.filter(|handle| handle != &self.action_context);
        ACTION_TARGET.with(|slot| *slot.borrow_mut() = target);
    }

    pub fn open_by_id(&mut self, id: &str, cx: &mut App) -> bool {
        let Some(index) = MENUS.iter().position(|menu| menu.id == id) else {
            return false;
        };
        self.popup = None;
        self.dismiss_subscription = None;
        self.selected = None;
        self.compact_visible = false;
        self.toggle(index, cx);
        true
    }

    pub fn run_theme_by(&mut self, selector: &str, cx: &mut App) -> bool {
        let names = lithe_gpui_settings::theme::theme_names(cx);
        let picked = match selector.parse::<usize>() {
            Ok(index) => names.get(index).cloned(),
            Err(_) => names.iter().find(|name| name.as_str() == selector).cloned(),
        };
        let Some(name) = picked else {
            return false;
        };
        apply_theme_choice(name, cx);
        true
    }

    pub fn run_action(&mut self, action: MenuAction, cx: &mut App) {
        self.push_run(action, cx);
    }
}

pub fn apply_theme_choice(name: String, cx: &mut App) {
    match lithe_gpui_settings::try_store(cx) {
        Some(store) => {
            let value: SharedString = name.clone().into();
            store.update(cx, |store, cx| store.set_theme_explicit(value, cx));
            eprintln!("S1_MENU_RUN id=lithe.menu.theme state=applied name={name}");
        }
        None => eprintln!("S1_MENU_RUN id=lithe.menu.theme state=unavailable"),
    }
}

pub fn lookup_action(id: &str) -> Option<MenuAction> {
    MENUS
        .iter()
        .flat_map(|menu| menu.items.iter())
        .find_map(|item| match item {
            MenuItem::Action(action) if action.label_key() == id => Some(*action),
            _ => None,
        })
}

pub fn mode_for(compact: bool) -> MenuBarMode {
    if compact {
        MenuBarMode::Compact
    } else {
        MenuBarMode::Pinned
    }
}

thread_local! {
    static MENU_BAR: std::cell::RefCell<Option<gpui_kit::WeakEntity<MenuBar>>> =
        const { std::cell::RefCell::new(None) };

    static MENU_BAR_SHELL: std::cell::RefCell<Option<gpui_kit::WeakEntity<crate::workspace::ShellWorkspace>>> =
        const { std::cell::RefCell::new(None) };

    static ACTION_TARGET: std::cell::RefCell<Option<FocusHandle>> =
        const { std::cell::RefCell::new(None) };
}

pub fn action_target() -> Option<FocusHandle> {
    ACTION_TARGET.with(|slot| slot.borrow().clone())
}

pub(crate) fn set_shell(shell: gpui_kit::WeakEntity<crate::workspace::ShellWorkspace>) {
    MENU_BAR_SHELL.with(|slot| *slot.borrow_mut() = Some(shell));
}

pub(crate) fn set_menu_bar(bar: gpui_kit::WeakEntity<MenuBar>) {
    MENU_BAR.with(|slot| *slot.borrow_mut() = Some(bar));
}

pub fn handle() -> gpui_kit::WeakEntity<MenuBar> {
    MENU_BAR
        .with(|slot| slot.borrow().clone())
        .expect("菜单栏句柄没有登记：`ShellWorkspace::new` 必须在 `render` 之前调 `set_menu_bar`")
}

pub fn push_show_run_pane(cx: &mut App) {
    let Some(bar) = try_handle() else {
        eprintln!("S1_TITLE_BAR_RUN id=lithe.run.configurations state=no_menu_bar");
        return;
    };
    let _ = bar.update(cx, |bar, cx| bar.push_run(MenuAction::ShowRunAndDebug, cx));
    eprintln!("S1_TITLE_BAR_RUN id=lithe.run.configurations state=pushed");
}

pub(crate) fn try_handle() -> Option<gpui_kit::WeakEntity<MenuBar>> {
    MENU_BAR.with(|slot| slot.borrow().clone())
}

pub(crate) fn take_pending_runs(cx: &mut App) -> Vec<MenuRequest> {
    let Some(bar) = MENU_BAR.with(|slot| slot.borrow().clone()) else {
        return Vec::new();
    };
    bar.update(cx, |bar, _| bar.drain_runs())
        .unwrap_or_default()
}

pub fn toggle_menu_bar(cx: &mut App) {
    let Some(bar) = MENU_BAR.with(|slot| slot.borrow().clone()) else {
        eprintln!("S1_MENU_BAR mode=unavailable");
        return;
    };
    let _ = bar.update(cx, |bar, cx| bar.toggle_mode(cx));
}

pub fn menu_bar(
    bar: &mut MenuBar,
    window: &mut Window,
    cx: &mut Context<MenuBar>,
) -> impl IntoElement {
    if bar.restore_focus {
        bar.restore_focus = false;
        window.focus(&bar.action_context, cx);
    }
    match bar.mode {
        MenuBarMode::Pinned => pinned_bar(bar, window, cx).into_any_element(),
        MenuBarMode::Compact => compact_bar(bar, window, cx).into_any_element(),
    }
}

pub(crate) fn render_bar(
    bar: &Entity<MenuBar>,
    window: &mut Window,
    cx: &mut App,
) -> Option<gpui_kit::AnyElement> {
    bar.downgrade()
        .update(cx, |bar, cx| menu_bar(bar, window, cx).into_any_element())
        .ok()
}

fn pinned_bar(
    bar: &mut MenuBar,
    window: &mut Window,
    cx: &mut Context<MenuBar>,
) -> impl IntoElement {
    let mut bar_element = h_flex()
        .id("lithe-menu-bar-pinned")
        .flex_shrink_0()
        .items_center()
        .h_6()
        .gap_0p5()
        .p_0p5();
    for index in 0..MENUS.len() {
        bar_element = bar_element.child(menu_item(bar, index, window, cx));
    }
    bar_element
}

fn compact_bar(
    bar: &mut MenuBar,
    window: &mut Window,
    cx: &mut Context<MenuBar>,
) -> impl IntoElement {
    let handle = handle();
    let toggle_handle = handle.clone();
    let button = Button::new("lithe-menu-bar-trigger")
        .ghost()
        .small()
        .icon(idea_icon_mono(&idea::FILES_ICON, cx.theme().is_dark()))
        .tooltip(tr("lithe.window.menu"))
        .accessibility_label(tr("lithe.window.menu"))
        .on_click(move |_: &ClickEvent, _window: &mut Window, cx: &mut App| {
            let _ = toggle_handle.update(cx, |bar, cx| bar.toggle_compact(cx));
        });

    let mut floating = None;
    if bar.compact_visible {
        let mut capsule = h_flex()
            .id("lithe-menu-bar-floating")
            .items_center()
            .h_6()
            .gap_0p5()
            .p_1()
            .mt_1()
            .rounded(rems(FLOAT_RADIUS_SPEC / 16.))
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background);
        for index in 0..MENUS.len() {
            capsule = capsule.child(menu_item(bar, index, window, cx));
        }
        if let Some(bounds) = bar.compact_trigger_bounds.get() {
            let rem = window.rem_size();
            floating = Some(
                deferred(
                    Positioner::side(bounds)
                        .placement(Placement::Bottom)
                        .align(Align::Start)
                        .offset(crate::rem_px(rem, PANEL_OFFSET_SPEC))
                        .margin(crate::rem_px(rem, PANEL_WINDOW_MARGIN_SPEC))
                        .occlude()
                        .child(capsule),
                )
                .with_priority(1)
                .into_any_element(),
            );
        }
    }

    let bounds_cell = bar.compact_trigger_bounds.clone();
    div()
        .id("lithe-menu-bar-compact")
        .relative()
        .flex_shrink_0()
        .on_prepaint(move |bounds, window, _cx| {
            let first = bounds_cell.get().is_none();
            bounds_cell.set(Some(bounds));
            if first {
                window.request_animation_frame();
            }
        })
        .child(button)
        .children(floating)
}

fn trigger(
    bar: &MenuBar,
    index: usize,
    window: &mut Window,
    cx: &mut Context<MenuBar>,
) -> impl IntoElement {
    let _ = window;
    let menu = &MENUS[index];
    let is_open = bar.selected == Some(index);
    let theme = cx.theme();
    let (idle, active_text, hover_bg) = (theme.muted_foreground, theme.foreground, theme.accent);
    let handle = handle();

    div()
        .id(("lithe-menu-trigger", index))
        .flex()
        .items_center()
        .h_5()
        .px_1p5()
        .rounded(rems(ITEM_RADIUS_SPEC / 16.))
        .text_sm()
        .text_color(if is_open { active_text } else { idle })
        .hover(move |style| style.bg(hover_bg).text_color(active_text))
        .on_hover(cx.listener(move |bar, hovered: &bool, _window, cx| {
            if *hovered {
                bar.hover_switch(index, cx);
            }
        }))
        .on_mouse_down(MouseButton::Left, {
            let handle = handle.clone();
            move |_, window, cx| {
                cx.stop_propagation();
                let focused = window.focused(cx);
                let _ = handle.update(cx, |bar, cx| {
                    bar.remember_action_target(focused);
                    bar.toggle(index, cx);
                });
            }
        })
        .child(menu.label())
}

fn menu_item(
    bar: &mut MenuBar,
    index: usize,
    window: &mut Window,
    cx: &mut Context<MenuBar>,
) -> impl IntoElement {
    let is_open = bar.selected == Some(index);

    let bounds_cell = bar.trigger_bounds.clone();
    let mut wrapper = div()
        .id(("lithe-menu-item", index))
        .relative()
        .flex_shrink_0()
        .on_prepaint(move |bounds, window, _cx| {
            let first = bounds_cell.borrow()[index].is_none();
            bounds_cell.borrow_mut()[index] = Some(bounds);
            if first {
                window.request_animation_frame();
            }
        })
        .child(trigger(bar, index, window, cx));

    if is_open {
        if bar.popup.is_none() {
            let popup = build_popup(bar, index, window, cx);
            bar.dismiss_subscription =
                Some(cx.subscribe_in(&popup, window, MenuBar::handle_dismiss));
            let focus = popup.read(cx).focus_handle(cx);
            if !focus.contains_focused(window, cx) {
                focus.focus(window, cx);
            }
            match bar.trigger_bounds_of(index) {
                Some(bounds) => eprintln!(
                    "S1_MENU_BOUNDS id={} x={} y={} w={} h={}",
                    MENUS[index].id,
                    f32::from(bounds.origin.x) as i32,
                    f32::from(bounds.origin.y) as i32,
                    f32::from(bounds.size.width) as i32,
                    f32::from(bounds.size.height) as i32,
                ),
                None => eprintln!("S1_MENU_BOUNDS id={} state=pending", MENUS[index].id),
            }
            bar.popup = Some(popup);
        }
        if let Some(popup) = bar.popup.clone() {
            let rem = window.rem_size();
            wrapper = wrapper.children(popup_for(popup, bar.trigger_bounds_of(index), rem));
        }
    }
    wrapper
}

fn popup_for(
    popup: Entity<PopupMenu>,
    bounds: Option<Bounds<Pixels>>,
    rem: Pixels,
) -> Option<impl IntoElement> {
    let bounds = bounds?;
    Some(
        deferred(
            Positioner::side(bounds)
                .placement(Placement::Bottom)
                .align(Align::Start)
                .offset(crate::rem_px(rem, PANEL_OFFSET_SPEC))
                .margin(crate::rem_px(rem, PANEL_WINDOW_MARGIN_SPEC))
                .occlude()
                .child(popup),
        )
        .with_priority(1),
    )
}

fn build_popup(
    bar: &MenuBar,
    index: usize,
    window: &mut Window,
    cx: &mut Context<MenuBar>,
) -> Entity<PopupMenu> {
    let items = MENUS[index].items;
    let action_context = bar.action_context.clone();
    let handle = handle();

    PopupMenu::build(window, cx, move |popup, window, cx| {
        let rem = window.rem_size();
        let popup = popup
            .action_context(action_context.clone())
            .min_w(crate::rem_px(rem, PANEL_MIN_WIDTH_SPEC));

        let names = lithe_gpui_settings::theme::theme_names(cx);
        let theme_menu = (!names.is_empty()).then(|| theme_menu(names, window, cx));

        let mut popup = popup;
        for item in items {
            popup = match item {
                MenuItem::Separator => popup.separator(),
                MenuItem::Action(action) => {
                    let action = *action;
                    let item_handle = handle.clone();
                    let mut item =
                        PopupMenuItem::new(action.label()).icon(action.icon(cx.theme().is_dark()));
                    if let Some(bound) = action.keybind_action() {
                        item = item.action(bound);
                    }
                    popup.item(item.on_click(
                        move |_: &ClickEvent, _window: &mut Window, cx: &mut App| {
                            let _ = item_handle.update(cx, |bar, cx| bar.push_run(action, cx));
                        },
                    ))
                }
                MenuItem::NotWired {
                    label_key,
                    missing,
                    icon,
                } => {
                    let label_key = *label_key;
                    let missing = *missing;
                    let icon = match *icon {
                        MenuIcon::Idea(i) => idea_icon_mono(i, cx.theme().is_dark()),
                        MenuIcon::Lucide(name) => Icon::new(name),
                    };
                    let item_handle = handle.clone();
                    popup.item(PopupMenuItem::new(tr(label_key)).icon(icon).on_click(
                        move |_: &ClickEvent, _window: &mut Window, cx: &mut App| {
                            let _ = item_handle
                                .update(cx, |bar, cx| bar.push_not_wired(label_key, missing, cx));
                        },
                    ))
                }
                MenuItem::Theme => match &theme_menu {
                    Some(submenu) => popup.item(PopupMenuItem::submenu(
                        tr("lithe.menu.theme"),
                        submenu.clone(),
                    )),
                    None => popup,
                },
            };
        }
        popup
    })
}

fn theme_menu(names: Vec<String>, window: &mut Window, cx: &mut App) -> Entity<PopupMenu> {
    PopupMenu::build(window, cx, move |popup, window, cx| {
        let rem = window.rem_size();
        let mut popup = popup.min_w(crate::rem_px(rem, PANEL_MIN_WIDTH_SPEC));
        for name in names {
            let label: SharedString = name.clone().into();
            let value: SharedString = name.into();
            popup = popup.item(
                PopupMenuItem::new(label)
                    .icon(idea_icon_mono(&idea::PALETTE_ICON, cx.theme().is_dark()))
                    .on_click(move |_: &ClickEvent, _window: &mut Window, cx: &mut App| {
                        apply_theme_choice(value.to_string(), cx);
                    }),
            );
        }
        popup
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CloseReason, KEY_GO_TO_LINE, KEY_REOPEN_CLOSED_TAB, KEY_RESET_ZOOM,
        KEY_TOGGLE_ACTIVITY_SIDEBAR, KEY_TOGGLE_TERMINAL, KEY_ZOOM_IN, KEY_ZOOM_OUT, MENUS,
        MISSING_GROUPS, MenuAction, MenuBarMode, MenuItem, NOT_WIRED_OUTSIDE_MENUS, NavState,
        action_count, mode_for, not_wired_count, resolve_menu_toggle,
    };
    use lithe_gpui_shared::tr;
    use std::collections::BTreeSet;

    const SOURCE_MENU_ITEMS: usize = 89;
    const LOCAL_ONLY_ITEMS: usize = 5;

    #[test]
    fn close_reasons_match_the_spec_vocabulary() {
        let reasons = [
            (CloseReason::Dismiss, "dismiss"),
            (CloseReason::Toggle, "toggle"),
            (CloseReason::Escape, "escape"),
            (CloseReason::Select, "select"),
            (CloseReason::Switch, "switch"),
        ];
        let ids: Vec<&str> = reasons.iter().map(|(reason, _)| reason.id()).collect();
        assert_eq!(ids, vec!["dismiss", "toggle", "escape", "select", "switch"]);
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "五个起因必须互不相同");
    }

    #[test]
    fn file_menu_close_entries_are_wired() {
        use crate::command_palette::CommandId;

        let expected = [
            (
                MenuAction::OpenFile,
                "lithe.outline.openFile",
                CommandId::OpenFile,
            ),
            (
                MenuAction::CloseTab,
                "lithe.menu.closeTab",
                CommandId::CloseTab,
            ),
            (
                MenuAction::CloseOtherTabs,
                "lithe.menu.closeOtherTabs",
                CommandId::CloseOtherTabs,
            ),
            (
                MenuAction::CloseAllTabs,
                "lithe.menu.closeAllTabs",
                CommandId::CloseAllTabs,
            ),
            (
                MenuAction::CloseSavedTabs,
                "lithe.menu.closeSavedTabs",
                CommandId::CloseSavedTabs,
            ),
            (
                MenuAction::CloseTabsToLeft,
                "lithe.menu.closeTabsToLeft",
                CommandId::CloseTabsToLeft,
            ),
            (
                MenuAction::CloseTabsToRight,
                "lithe.menu.closeTabsToRight",
                CommandId::CloseTabsToRight,
            ),
            (
                MenuAction::ReopenClosedTab,
                "lithe.menu.reopenClosedTab",
                CommandId::ReopenClosedTab,
            ),
        ];
        for (action, key, id) in expected {
            assert_eq!(action.label_key(), key, "{action:?} 的文案键变了");
            assert_eq!(action.command_id(), Some(id), "{action:?} 没有执行分支");
        }

        assert_eq!(MenuAction::OpenFolder.label_key(), "lithe.menu.openFolder");
        assert_eq!(
            MenuAction::OpenFolder.command_id(),
            None,
            "「打开文件夹」不该有 CommandId（那会让命令面板的行序多一条）"
        );

        let file = MENUS
            .iter()
            .find(|menu| menu.id == "file")
            .expect("顶级菜单里必须有 file");
        let actions: Vec<MenuAction> = file
            .items
            .iter()
            .filter_map(|item| match item {
                MenuItem::Action(action) => Some(*action),
                _ => None,
            })
            .collect();
        assert_eq!(
            actions,
            vec![
                MenuAction::NewWindow,
                MenuAction::OpenFolder,
                MenuAction::OpenFile,
                MenuAction::Save,
                MenuAction::CloseTab,
                MenuAction::CloseOtherTabs,
                MenuAction::CloseAllTabs,
                MenuAction::CloseSavedTabs,
                MenuAction::CloseTabsToLeft,
                MenuAction::CloseTabsToRight,
                MenuAction::ReopenClosedTab,
            ]
        );
    }

    #[test]
    fn new_window_menu_item_is_wired_to_the_window_host_command() {
        use crate::command_palette::CommandId;

        let listed = MENUS
            .iter()
            .flat_map(|menu| menu.items.iter())
            .filter(|item| matches!(item, MenuItem::Action(MenuAction::NewWindow)))
            .count();
        assert_eq!(
            listed, 1,
            "「新建窗口」在菜单里必须恰好一条（不许两个入口两个动作）"
        );

        assert_eq!(MenuAction::NewWindow.label_key(), "lithe.menu.newWindow");
        assert_ne!(
            tr(MenuAction::NewWindow.label_key()).as_ref(),
            "lithe.menu.newWindow",
            "菜单文案解析不出来（键名拼错了？）"
        );
        assert_eq!(
            MenuAction::NewWindow.command_id(),
            Some(CommandId::NewWindow),
            "菜单项必须落到「新窗口」那一个 CommandId（执行分支在 run_command_id 里）"
        );
        assert_eq!(
            CommandId::NewWindow.id(),
            "new-window",
            "诊断 id 是 `S1_COMMAND_RUN id=…` 的可 grep 契约"
        );
        assert!(
            MenuAction::NewWindow.menu_key().is_none()
                && MenuAction::NewWindow.keybind_action().is_none(),
            "真源 File ▸ New Window（LitheApp.swift:492）没有键位，菜单上不许画键位胶囊"
        );
    }

    #[test]
    fn nine_top_level_menus_in_source_order() {
        let ids: Vec<&str> = MENUS.iter().map(|menu| menu.id).collect();
        assert_eq!(
            ids,
            vec![
                "file", "edit", "view", "go", "terminal", "run", "tools", "window", "help"
            ]
        );
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "顶级菜单 id 必须互不相同");
    }

    #[test]
    fn top_level_labels_use_the_menu_namespace() {
        for menu in MENUS {
            assert_eq!(
                menu.label_key,
                format!("lithe.menu.{}", menu.id).as_str(),
                "顶级菜单 {} 的文案键不是 lithe.menu.{}",
                menu.id,
                menu.id
            );
        }
    }

    #[test]
    fn b2_actions_are_wired_and_only_bound_ones_carry_a_key() {
        let table: &[(MenuAction, &str, bool)] = &[
            (MenuAction::Undo, "lithe.menu.undo", true),
            (MenuAction::Redo, "lithe.menu.redo", true),
            (MenuAction::Cut, "lithe.menu.cut", true),
            (MenuAction::Copy, "lithe.menu.copy", true),
            (MenuAction::Paste, "lithe.menu.paste", true),
            (MenuAction::SelectAll, "lithe.menu.selectAll", true),
            (MenuAction::Find, "lithe.menu.find", true),
            (
                MenuAction::FindAndReplace,
                "lithe.menu.findAndReplace",
                true,
            ),
            (MenuAction::QuickFix, "lithe.menu.quickFix", true),
            (
                MenuAction::ToggleActivitySidebar,
                "lithe.menu.toggleActivitySidebar",
                true,
            ),
            (
                MenuAction::ToggleTerminal,
                "lithe.menu.toggleTerminal",
                true,
            ),
            (MenuAction::ZoomIn, "lithe.menu.zoomIn", true),
            (MenuAction::ZoomOut, "lithe.menu.zoomOut", true),
            (MenuAction::ResetZoom, "lithe.menu.resetZoom", true),
            (MenuAction::ToggleMenuBar, "lithe.menu.toggleMenuBar", true),
            (
                MenuAction::ReopenClosedTab,
                "lithe.menu.reopenClosedTab",
                true,
            ),
            (
                MenuAction::ToggleSecondarySidebar,
                "lithe.menu.toggleSecondarySidebar",
                false,
            ),
            (MenuAction::ShowDiagnostics, "lithe.menu.diagnostics", false),
            (
                MenuAction::ShowFileExplorer,
                "lithe.menu.fileExplorer",
                false,
            ),
            (
                MenuAction::ShowSourceControl,
                "lithe.menu.sourceControl",
                false,
            ),
            (MenuAction::ShowRunAndDebug, "lithe.menu.runAndDebug", false),
        ];

        for (action, key, has_key_hint) in table {
            assert_eq!(action.label_key(), *key, "{action:?} 的文案键变了");
            assert!(
                action.label_key().starts_with("lithe.menu."),
                "{action:?} 的文案键不在真源的菜单段里"
            );
            assert_eq!(
                action.keybind_action().is_some(),
                *has_key_hint,
                "{action:?} 的键位胶囊与真实绑定不一致（显示了却按不出，或能按却不显示）"
            );
        }

        let listed: Vec<MenuAction> = MENUS
            .iter()
            .flat_map(|menu| menu.items.iter())
            .filter_map(|item| match item {
                MenuItem::Action(action) => Some(*action),
                _ => None,
            })
            .collect();
        for (action, _, _) in table {
            assert!(listed.contains(action), "{action:?} 没有出现在任何菜单里");
        }

        for action in listed.iter().copied() {
            let Some(key) = action.menu_key() else {
                continue;
            };
            assert!(
                action.keybind_action().is_some(),
                "{action:?} 声明了键位 {key} 却没有对应的 Action —— 它会显示一颗按不出的键"
            );
            gpui_kit::Keystroke::parse(key)
                .unwrap_or_else(|error| panic!("{action:?} 的键位 {key:?} 解析不了：{error}"));
        }

        let bound: Vec<&str> = MENUS
            .iter()
            .flat_map(|menu| menu.items.iter())
            .filter_map(|item| match item {
                MenuItem::Action(action) => action.menu_key(),
                _ => None,
            })
            .collect();
        assert_eq!(
            bound,
            vec![
                KEY_REOPEN_CLOSED_TAB,
                KEY_TOGGLE_ACTIVITY_SIDEBAR,
                KEY_TOGGLE_TERMINAL,
                KEY_ZOOM_IN,
                KEY_ZOOM_OUT,
                KEY_RESET_ZOOM,
                KEY_GO_TO_LINE,
            ],
            "键位清单变了（改它就是改验证脚本）"
        );
    }

    #[test]
    fn u20_items_are_wired_and_left_no_placeholder_behind() {
        let table: &[(MenuAction, &str)] = &[
            (MenuAction::GoToLine, "lithe.menu.goToLine"),
            (MenuAction::ToggleWordWrap, "lithe.menu.toggleWordWrap"),
            (MenuAction::ShowHover, "lithe.menu.showHover"),
        ];

        for (action, label) in table {
            assert_eq!(action.label_key(), *label, "{action:?} 的文案键变了");
            assert_ne!(
                tr(label).as_ref(),
                *label,
                "{label} 的菜单文案解析不出来（键名拼错了？）"
            );

            let listed = MENUS
                .iter()
                .flat_map(|menu| menu.items.iter())
                .filter(|item| matches!(item, MenuItem::Action(a) if a == action))
                .count();
            assert_eq!(listed, 1, "{action:?} 在菜单里必须恰好一条 Action");

            let placeholders = MENUS
                .iter()
                .flat_map(|menu| menu.items.iter())
                .filter(|item| item.label_key() == Some(*label))
                .count();
            assert_eq!(
                placeholders, 1,
                "{label} 还挂在别的项上（占位项没撤干净，或同一文案键有两条）"
            );

            assert!(
                action.command_id().is_none(),
                "{action:?} 的执行分支在 apply_menu_action 里，不该同时有 CommandId"
            );
        }
    }

    #[test]
    fn only_go_to_line_of_the_u20_items_carries_a_key() {
        assert_eq!(KEY_GO_TO_LINE, "ctrl-g");
        assert_eq!(MenuAction::GoToLine.menu_key(), Some(KEY_GO_TO_LINE));
        assert!(
            MenuAction::GoToLine.keybind_action().is_some(),
            "菜单上要画 `Ctrl+G` 胶囊，这条动作就必须有一个真被登记的 Action"
        );

        for action in [MenuAction::ToggleWordWrap, MenuAction::ShowHover] {
            assert_eq!(action.menu_key(), None, "{action:?} 不该有键位");
            assert!(
                action.keybind_action().is_none(),
                "{action:?} 没有键位，就不许画一颗按不出的胶囊"
            );
        }
    }

    #[test]
    fn every_listed_item_is_wired_or_declares_what_is_missing() {
        const NON_COMMAND: [MenuAction; 31] = [
            MenuAction::OpenFolder,
            MenuAction::NewTerminalTab,
            MenuAction::CloseTerminalTab,
            MenuAction::Preferences,
            MenuAction::Minimize,
            MenuAction::Maximize,
            MenuAction::ToggleFullscreen,
            MenuAction::Undo,
            MenuAction::Redo,
            MenuAction::Cut,
            MenuAction::Copy,
            MenuAction::Paste,
            MenuAction::SelectAll,
            MenuAction::Find,
            MenuAction::FindAndReplace,
            MenuAction::QuickFix,
            MenuAction::ToggleActivitySidebar,
            MenuAction::ToggleSecondarySidebar,
            MenuAction::ShowDiagnostics,
            MenuAction::ShowFileExplorer,
            MenuAction::ShowSourceControl,
            MenuAction::ShowRunAndDebug,
            MenuAction::StartDebugging,
            MenuAction::StopDebugging,
            MenuAction::ToggleBreakpoint,
            MenuAction::GoToLine,
            MenuAction::ToggleWordWrap,
            MenuAction::ShowHover,
            MenuAction::ZoomIn,
            MenuAction::ZoomOut,
            MenuAction::ResetZoom,
        ];

        let mut wired: Vec<MenuAction> = Vec::new();
        let mut declared: Vec<(&'static str, super::Missing)> = Vec::new();
        for menu in MENUS {
            for item in menu.items {
                match item {
                    MenuItem::Action(action) => {
                        assert!(
                            item.missing().is_none(),
                            "{action:?} 已经接线了，却又带了一条 missing 声明"
                        );
                        assert!(
                            action.command_id().is_some() || NON_COMMAND.contains(action),
                            "{action:?} 既没有 CommandId，也没登记在 NON_COMMAND 里 —— 它就是一条死项"
                        );
                        assert!(
                            action.label_key().starts_with("lithe."),
                            "{action:?} 的文案键不在 lithe.* 命名空间里"
                        );
                        assert!(
                            !wired.contains(action),
                            "{action:?} 在菜单里出现了两次（同一个能力不该两个入口）"
                        );
                        wired.push(*action);
                    }
                    MenuItem::NotWired {
                        label_key, missing, ..
                    } => {
                        assert!(
                            label_key.starts_with("lithe."),
                            "{label_key} 的文案键不在 lithe.* 命名空间里"
                        );
                        assert_ne!(
                            tr(label_key).as_ref(),
                            *label_key,
                            "{label_key} 的菜单文案解析不出来（键名拼错了？）"
                        );
                        assert!(
                            !missing.id.is_empty(),
                            "{label_key} 既没有接线、也没有声明缺什么（missing.id 是空串）"
                        );
                        assert!(
                            !missing.text_key.is_empty(),
                            "{label_key} 的 missing 声明是空的 —— 每句提示都必须回答'缺什么'"
                        );
                        let text = tr(missing.text_key);
                        assert_ne!(
                            text.as_ref(),
                            missing.text_key,
                            "{label_key} 的 missing 文案键解析不出来：{}",
                            missing.text_key
                        );
                        assert!(
                            text.starts_with("尚未接入：") || text.starts_with("Not wired yet:"),
                            "{label_key} 的提示必须一眼看出'尚未接入'：{text}"
                        );
                        declared.push((label_key, *missing));
                    }
                    MenuItem::Separator | MenuItem::Theme => {}
                }
            }
        }

        for action in NON_COMMAND {
            assert!(
                wired.contains(&action),
                "{action:?} 登记在 NON_COMMAND 里，但菜单里没有它"
            );
        }
        assert_eq!(action_count(), wired.len());
        assert_eq!(not_wired_count(), declared.len());
        assert_eq!(
            action_count() + not_wired_count() + 1,
            SOURCE_MENU_ITEMS + LOCAL_ONLY_ITEMS
        );
        assert!(
            declared.len() >= 42,
            "占位项突然少了一半（现在 {}）？查一下是不是有人把没能力的项又删掉了",
            declared.len()
        );
    }

    #[test]
    fn quick_open_menu_item_dispatches_the_same_command_as_the_shortcut() {
        use crate::command_palette::CommandId;

        let listed = MENUS
            .iter()
            .flat_map(|menu| menu.items.iter())
            .filter(|item| matches!(item, MenuItem::Action(MenuAction::QuickOpen)))
            .count();
        assert_eq!(
            listed, 1,
            "「快速打开」在菜单里必须恰好一条（不许两个入口两个动作）"
        );

        assert_eq!(MenuAction::QuickOpen.label_key(), "lithe.menu.quickOpen");
        assert_ne!(
            tr(MenuAction::QuickOpen.label_key()).as_ref(),
            "lithe.menu.quickOpen",
            "菜单文案解析不出来（键名拼错了？）"
        );
        assert_eq!(
            MenuAction::QuickOpen.command_id(),
            Some(CommandId::OpenQuickOpen),
            "菜单项必须与 `Ctrl+P` 落到同一个 CommandId"
        );
        assert_eq!(
            CommandId::OpenQuickOpen.id(),
            "open-quick-open",
            "诊断 id 是 `S1_COMMAND_RUN id=…` 的可 grep 契约"
        );
        assert!(
            MenuAction::QuickOpen.keybind_action().is_some(),
            "菜单上要显示 `Ctrl+P` 胶囊，这条动作就必须有一个真被登记的 Action"
        );
    }

    #[test]
    fn search_everywhere_menu_item_is_wired_without_a_keybinding() {
        use crate::command_palette::CommandId;

        let listed = MENUS
            .iter()
            .flat_map(|menu| menu.items.iter())
            .filter(|item| matches!(item, MenuItem::Action(MenuAction::SearchEverywhere)))
            .count();
        assert_eq!(
            listed, 1,
            "「全局搜索」在菜单里必须恰好一条（不许两个入口两个动作）"
        );

        assert_eq!(
            MenuAction::SearchEverywhere.label_key(),
            "lithe.menu.globalSearch"
        );
        assert_ne!(
            tr(MenuAction::SearchEverywhere.label_key()).as_ref(),
            "lithe.menu.globalSearch",
            "菜单文案解析不出来（键名拼错了？）"
        );
        assert_eq!(
            MenuAction::SearchEverywhere.command_id(),
            Some(CommandId::OpenSearchEverywhere),
            "菜单项必须落到 Search Everywhere 那一个 CommandId"
        );
        assert_eq!(
            CommandId::OpenSearchEverywhere.id(),
            "open-search-everywhere",
            "诊断 id 是 `S1_COMMAND_RUN id=…` 的可 grep 契约"
        );
        assert!(
            MenuAction::SearchEverywhere.keybind_action().is_none(),
            "本切片没有为它找出一条有真源依据的空闲键位，所以菜单上不许画键位胶囊"
        );
    }

    #[test]
    fn replace_in_files_menu_item_dispatches_the_same_command_as_the_shortcut() {
        use crate::command_palette::CommandId;

        let listed = MENUS
            .iter()
            .flat_map(|menu| menu.items.iter())
            .filter(|item| matches!(item, MenuItem::Action(MenuAction::ReplaceInFiles)))
            .count();
        assert_eq!(
            listed, 1,
            "「在文件中替换」在菜单里必须恰好一条（不许两个入口两个动作）"
        );

        assert_eq!(
            MenuAction::ReplaceInFiles.label_key(),
            "lithe.search.gpui.replaceInFiles"
        );
        assert_ne!(
            tr(MenuAction::ReplaceInFiles.label_key()).as_ref(),
            "lithe.search.gpui.replaceInFiles",
            "菜单文案解析不出来（键名拼错了？或没进 GPUI_ONLY_KEYS / WIRED）"
        );
        assert_eq!(
            MenuAction::ReplaceInFiles.command_id(),
            Some(CommandId::OpenReplaceInFiles),
            "菜单项必须落到 Replace in Files 那一个 CommandId"
        );
        assert_eq!(
            CommandId::OpenReplaceInFiles.id(),
            "open-replace-in-files",
            "诊断 id 是 `S1_COMMAND_RUN id=…` 的可 grep 契约"
        );
        assert!(
            MenuAction::ReplaceInFiles.keybind_action().is_some(),
            "有键位的项必须给出 Action，否则菜单上会画一颗按不出效果的键（Q3 禁止）"
        );
        assert_eq!(
            crate::replace_in_files::REPLACE_KEY,
            "ctrl-shift-r",
            "真源 Windows 平台表给的就是它（LitheCommandCatalog.swift:61）"
        );
    }

    #[test]
    fn missing_declarations_are_grouped_by_capability() {
        let mut ids = BTreeSet::new();
        for group in MISSING_GROUPS {
            assert!(!group.id.is_empty(), "能力组 id 不许为空");
            assert!(
                group.text_key.starts_with("lithe.gpui.menuMissing."),
                "{} 的文案键不在 lithe.gpui.menuMissing.* 里",
                group.id
            );
            assert!(ids.insert(group.id), "能力组 id 重复：{}", group.id);
        }

        let mut used: BTreeSet<&'static str> = BTreeSet::new();
        let mut texts: BTreeSet<&'static str> = BTreeSet::new();
        for menu in MENUS {
            for item in menu.items {
                if let Some(missing) = item.missing() {
                    assert!(
                        MISSING_GROUPS.contains(&missing),
                        "有一条占位项用了登记表之外的声明：{}",
                        missing.id
                    );
                    used.insert(missing.id);
                    texts.insert(missing.text_key);
                }
            }
        }
        for (label_key, missing) in NOT_WIRED_OUTSIDE_MENUS {
            assert!(
                MISSING_GROUPS.contains(&missing),
                "{label_key} 用了登记表之外的声明：{}",
                missing.id
            );
            assert_ne!(
                tr(label_key).as_ref(),
                label_key,
                "{label_key} 的文案键解析不出来（键名拼错了？）"
            );
            used.insert(missing.id);
            texts.insert(missing.text_key);
        }
        for group in MISSING_GROUPS {
            assert!(
                used.contains(group.id),
                "能力组 {} 一条占位项都没用（声明表虚胖）",
                group.id
            );
        }
        assert!(
            texts.len() <= 15,
            "占位文案有 {} 句，超过了'按能力分组'的上限 15（= MISSING_GROUPS 的组数）",
            texts.len()
        );
        assert_eq!(
            MISSING_GROUPS.len(),
            15,
            "能力组数变了就必须同步改上面的上界与 `MISSING_GROUPS` 的文档"
        );
    }

    #[test]
    fn keyboard_shortcuts_appears_in_tools_and_help() {
        let places = MENUS
            .iter()
            .flat_map(|menu| {
                menu.items.iter().filter_map(|item| match item {
                    MenuItem::NotWired { label_key, .. }
                        if *label_key == "lithe.menu.keyboardShortcuts" =>
                    {
                        Some(menu.id)
                    }
                    _ => None,
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(places, vec!["tools", "help"]);
    }

    #[test]
    fn separators_are_between_items() {
        for menu in MENUS {
            let items = menu.items;
            if items.is_empty() {
                continue;
            }
            assert!(
                !matches!(items[0], MenuItem::Separator),
                "{} 以分隔线开头",
                menu.id
            );
            assert!(
                !matches!(items[items.len() - 1], MenuItem::Separator),
                "{} 以分隔线结尾",
                menu.id
            );
            for pair in items.windows(2) {
                assert!(
                    !(matches!(pair[0], MenuItem::Separator)
                        && matches!(pair[1], MenuItem::Separator)),
                    "{} 里有连着两条分隔线",
                    menu.id
                );
            }
        }
    }

    #[test]
    fn mode_defaults_to_pinned() {
        assert_eq!(mode_for(false), MenuBarMode::Pinned);
        assert_eq!(mode_for(true), MenuBarMode::Compact);
        assert_eq!(MenuBarMode::Pinned.id(), "pinned");
        assert_eq!(MenuBarMode::Compact.id(), "compact");
    }

    #[test]
    fn theme_submenu_appears_exactly_once() {
        let count = MENUS
            .iter()
            .flat_map(|menu| menu.items.iter())
            .filter(|item| matches!(item, MenuItem::Theme))
            .count();
        assert_eq!(count, 1, "主题子菜单只应出现在「视图」里一次");
    }

    #[test]
    fn opening_another_menu_closes_the_previous_one() {
        assert_eq!(MENUS[2].id, "view");
        assert_eq!(MENUS[5].id, "run");

        let mut state = NavState::Closed;
        state = resolve_menu_toggle(state, 2);
        assert_eq!(state, NavState::Open(2));

        let switched = resolve_menu_toggle(state, 5);
        assert_eq!(switched, NavState::Open(5));
        assert_ne!(
            switched, state,
            "切到别的顶级菜单必须换打开项，不是两个都开"
        );

        assert_eq!(resolve_menu_toggle(switched, 5), NavState::Closed);

        let opened = resolve_menu_toggle(NavState::Closed, 0);
        assert_eq!(opened, NavState::Open(0));
        assert_eq!(resolve_menu_toggle(opened, MENUS.len()), opened);

        assert!(MENUS.iter().any(|menu| menu.id == "terminal"));
        assert!(!MENUS.iter().any(|menu| menu.id == "nope"));
    }
}
