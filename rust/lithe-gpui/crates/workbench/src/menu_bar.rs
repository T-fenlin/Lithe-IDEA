use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext as _, Bounds, ClickEvent, Context, Entity, FocusHandle, Hsla,
    IntoElement, KeyBinding, Keystroke, MouseButton, ParentElement as _, Pixels, SharedString,
    Styled as _, Subscription, Window, div, prelude::*, rems, transparent_black,
};
use lithe_gpui_shared::icons::{idea, idea_icon_mono};
use lithe_gpui_shared::tr;
use lithe_ui::buttons::{Button, ButtonCustomVariant, ButtonVariant};
use lithe_ui::layout::h_stack;
use lithe_ui::menus::{Menu, MenuItem as MenuRow, TriggerMenu};
use lithe_ui::primitives::Measure;
use lithe_ui::theme::{ActiveTheme as _, IconSize, Palette, Size};

use crate::command_palette::CommandId;

const ITEM_RADIUS_SPEC: f32 = lithe_gpui_shared::design::CONTROL_RADIUS;

const BAR_HEIGHT_SPEC: f32 = 24.;

const BAR_GAP_SPEC: f32 = 2.;

const BAR_PADDING_SPEC: f32 = 2.;

/// `AutoHide` 藏起来时留在窗口顶上那条热区的高度（pt）。
///
/// 要够窄——它平时占着窗口顶部，太宽会把内容往下顶；也要够宽——窄到 1px 时鼠标划过去
/// 容易漏掉。4pt 是「看不出来但划得到」的量级。
const AUTOHIDE_HOT_ZONE_SPEC: f32 = 4.;

const TRIGGER_HEIGHT_SPEC: f32 = 20.;

const TRIGGER_PADDING_SPEC: f32 = 6.;

/// 面板最小宽度（pt）：窄菜单也别挤成一条。
const PANEL_MIN_WIDTH_SPEC: f32 = 240.;

/// 未接线那一行行尾的角标文案；为什么整机没有一个「还没做」的短标签见
/// `lithe-gpui/tools/extract-locale.mjs` 里 `gpui.menuNotWiredBadge` 那条的理由。
const NOT_WIRED_BADGE_KEY: &str = "lithe.gpui.menuNotWiredBadge";

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

pub const MISSING_LOCAL_HISTORY: Missing = Missing {
    id: "local_history",
    text_key: "lithe.gpui.menuMissing.localHistory",
};

pub const MISSING_HELP_ABOUT: Missing = Missing {
    id: "help_about",
    text_key: "lithe.gpui.menuMissing.helpAbout",
};

pub const MISSING_TERMINAL_SPLIT: Missing = Missing {
    id: "terminal_split",
    text_key: "lithe.gpui.menuMissing.terminalSplit",
};

/// 占位项用的能力分组表。
///
/// `editor_entry`、`file_lifecycle` 与 `shortcuts_view` 三个分组已随接线而**退场**：
/// B2（另存为 / 全部保存 / 还原文件）与 B1（新建标签 / 新建文件）让文件生命周期这批动作
/// 全部有了执行分支；`shortcuts_view` 唯一的占位项（Tools / Help 的「键盘快捷键」）也被
/// `MenuAction::ShowKeymapSettings` 接走 —— 三组一条占位项都不剩，空分组留在表里会被
/// `missing_declarations_are_grouped_by_capability` 判为「声明表虚胖」。对应的文案键
/// `lithe.gpui.menuMissing.editorEntry` / `.fileLifecycle` / `.shortcutsView` 仍留在真源里
/// （无人引用，未删），以便同名能力再出现时复用。
///
/// 同一轮里 `project_lifecycle` 与 `window_routing` 也退场了（C1）：关闭文件夹改成换根回
/// 欢迎页，关闭窗口 / 退出接成 `window.remove_window()` / `cx.quit()` —— 三条占位项都变成真
/// `MenuAction`，两组一条都不剩。文案键 `lithe.gpui.menuMissing.projectLifecycle` /
/// `.windowRouting` 同样留在真源里未删，理由与上面三条一致。
pub const MISSING_GROUPS: [Missing; 10] = [
    MISSING_EDITOR_FEATURES,
    MISSING_LSP_REQUESTS,
    MISSING_SUBSYSTEM,
    MISSING_GITHUB,
    MISSING_UPDATER,
    MISSING_CLONE_UI,
    MISSING_NEW_PROJECT_SCAFFOLDING,
    MISSING_LOCAL_HISTORY,
    MISSING_HELP_ABOUT,
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

// 字面量一律 `secondary-`：它是 gpui 的平台归一化开关（macOS → Command，其他 → Control），
// 也是 `settings/src/keymap.rs::DEFAULT_BINDINGS` 已经假设的形态。旧写的 `ctrl-` 在 macOS 上
// 把设置页显示的（⌘S）与真能按出来的（Ctrl+S）拆成了两套，那正是 F2 要收口的地方。
pub const KEY_REOPEN_CLOSED_TAB: &str = "secondary-shift-t";
pub const KEY_TOGGLE_ACTIVITY_SIDEBAR: &str = "secondary-b";
pub const KEY_TOGGLE_TERMINAL: &str = "secondary-j";
pub const KEY_ZOOM_IN: &str = "secondary-=";
pub const KEY_ZOOM_OUT: &str = "secondary--";
pub const KEY_RESET_ZOOM: &str = "secondary-0";
pub const KEY_GO_TO_LINE: &str = "secondary-g";
/// 查找 / 替换：IDEA-mac 的真源就是 ⌘F / ⌘R，`secondary-` 在 macOS 给 Command、Windows 给 Control。
pub const KEY_FIND: &str = "secondary-f";
pub const KEY_FIND_REPLACE: &str = "secondary-r";
/// 下一处 / 上一处：全窗绑定、三端同键，但**不建 `MenuAction`**（它们不出现在任何菜单行上，
/// 建了会挂「不在菜单里的 MenuAction」与「登记在 NON_COMMAND 却没进菜单」两条守卫）。
pub const KEY_FIND_NEXT: &str = "f3";
pub const KEY_FIND_PREVIOUS: &str = "shift-f3";
/// 后退 / 前进：IDEA 的 Navigate Back / Forward，三端同键（`secondary` 在 macOS 是 Command）。
pub const KEY_GO_BACK: &str = "secondary-alt-left";
pub const KEY_GO_FORWARD: &str = "secondary-alt-right";
/// 转到引用：真源自己写着「使用 Shift+F12 查找引用」（`references.findReferencesHint`）。
/// 绑定登记在 `lithe_gpui_editor::install_actions`（与 `f12` / 转到定义同一处）。
pub const KEY_GO_TO_REFERENCES: &str = "shift-f12";

gpui::actions!(
    lithe_workbench,
    [
        ReopenTab,
        ToggleSidebar,
        ToggleTerminalPane,
        ZoomIn,
        ZoomOut,
        ResetZoom,
        OpenGoToLine,
        GoBack,
        GoForward,
        NextTab,
        PreviousTab,
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

fn install_menu_key<A: gpui::Action>(
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
    install_menu_key(cx, KEY_GO_BACK, GoBack, MenuAction::GoBack);
    install_menu_key(cx, KEY_GO_FORWARD, GoForward, MenuAction::GoForward);
    // 切标签页：应用级惯例，与 `secondary-shift-t`（重开关掉的标签页）不冲突。这里**刻意保留
    // 字面 `ctrl-tab`**：`secondary-tab` 在 macOS 就是 ⌘Tab，被系统应用切换器吃掉，改用它会
    // 让 macOS 上的「下一个标签页」整条失效。IDEA-mac 自己也是 Ctrl+Tab。
    install_menu_key(cx, "ctrl-tab", NextTab, MenuAction::NextTab);
    install_menu_key(cx, "ctrl-shift-tab", PreviousTab, MenuAction::PreviousTab);

    // 查找 / 替换：与 GoToLine 同一范式 —— 绑键 + 把「跑这个菜单动作」推给外壳，菜单点击与
    // 键位走同一条 `apply_menu_action`。`Esc` 的上下文是查找条自己的，不在这里绑。
    install_menu_key(cx, KEY_FIND, lithe_gpui_editor::OpenFind, MenuAction::Find);
    install_menu_key(
        cx,
        KEY_FIND_REPLACE,
        lithe_gpui_editor::OpenFindReplace,
        MenuAction::FindAndReplace,
    );
    // 下一处 / 上一处：**不建 `MenuAction`**（见 `KEY_FIND_NEXT` 的注释）。全窗绑定的键在编辑器
    // 内有焦点时由 `EditorPane` 自己的 handler 接住；焦点不在编辑器里时下面的转发兜住，两步
    // 不会重叠 —— 元素级 handler 在冒泡阶段默认 `stop_propagation`，全局监听器就不会再跑。
    cx.bind_keys([
        KeyBinding::new(KEY_FIND_NEXT, lithe_gpui_editor::FindNext, None),
        KeyBinding::new(KEY_FIND_PREVIOUS, lithe_gpui_editor::FindPrevious, None),
        // `Esc` 只在查找条的上下文里关查找条；编辑器里的 `Esc` 仍归 `ElyEditor` 的 `SingleCursor`。
        KeyBinding::new(
            "escape",
            lithe_gpui_editor::CloseFind,
            Some(lithe_gpui_editor::FIND_BAR_KEY_CONTEXT),
        ),
    ]);
    cx.on_action(|_: &lithe_gpui_editor::FindNext, cx: &mut App| {
        crate::workspace::route_find_step(true, cx)
    });
    cx.on_action(|_: &lithe_gpui_editor::FindPrevious, cx: &mut App| {
        crate::workspace::route_find_step(false, cx)
    });
}

/// 菜单行「此刻能不能点」的判定上下文。
///
/// 只装**标量**：它每帧在 `ShellWorkspace::render` 里现取一次，然后按值传进菜单构建链
/// （`render_bar` → `menu_bar` → `menu_row` / `compact_menu` → `menu_item` → `build_menu`）。
/// 不放实体句柄、不放 `PathBuf`、不放任何需要 `&mut App` 才能问的东西 —— 构建链跑在
/// `ShellWorkspace` 自己的渲染栈里，那里不许再 `update` 别的实体（会撞 gpui 的
/// `cannot update X while it is already being updated`）。
///
/// 不变量（由 `EditorPane::close` 保证，`ShellWorkspace::menu_context` 是唯一生产者）：
///
///   1. `has_active_buffer == (tab_count > 0) == active_tab_index.is_some()`
///   2. `has_active_disk_file == true` ⇒ `has_active_buffer == true`
///   3. `active_writable` 只在 `has_active_buffer == true` 时有意义；没有活动缓冲时恒为 `false`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorContext {
    /// 有没有活动缓冲（= 有没有打开的标签页）。
    pub has_active_buffer: bool,
    /// 打开着几个标签页。
    pub tab_count: usize,
    /// 活动标签页的下标；没有标签页时是 `None`。
    pub active_tab_index: Option<usize>,
    /// 活动缓冲是一个**真正的磁盘文件**（未命名 scratch 与虚拟源路径不算）。
    pub has_active_disk_file: bool,
    /// 活动缓冲能写回磁盘（非二进制 / 非有损转码 / 非缺失 / 非超限）。
    pub active_writable: bool,
    /// 跳转历史里有没有「上一处」可去。
    pub can_go_back: bool,
    /// 跳转历史里有没有「下一处」可去。
    pub can_go_forward: bool,
    /// 有没有关掉过标签页（File ▸ Reopen Closed Tab 的启用条件）。
    ///
    /// 不并入 `tab_count`：关掉过的文件与当前打开着几个标签页是两件事，本批主场景
    /// （刚开项目、还没关过任何文件）下它就是 `false`。
    pub has_closed_tabs: bool,
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
    // 另外两个 Go-to 目标：与 `GoToDefinition` 同一条路（`navigate_to_*` + `NavTarget`），
    // 只有 `keybind_action()` 与 `command_id()` 的取值不同。
    GoToImplementation,
    GoToTypeDefinition,
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
    // 符号级语义动作：执行分支在 `apply_menu_action`，各自直接调 `EditorPane` 的同名方法
    // （与 `ShowHover` 一条路）。请求本身是异步的，菜单派发电是同步的，所以不经过 Action 派发。
    GoToReferences,
    RenameSymbol,
    FormatDocument,
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
    // 编辑器行编辑：动作本体在 `lithe_ui::editor`（与 `ElyEditor` 上下文里登记的同一批类型），
    // 菜单派发靠类型匹配，和 Undo 一条路。
    ToggleComment,
    DuplicateLine,
    DeleteLine,
    MoveLineUp,
    MoveLineDown,
    // 显示开关：状态在 settings store，执行分支在 `run_menu_action`。
    ToggleMinimap,
    ToggleLineNumbers,
    ToggleRenderWhitespace,
    // 后退 / 前进：跳转历史在 `EditorPane`，动作是工作台自己的 `gpui::actions!`。
    GoBack,
    GoForward,
    // 快捷键一览：设置对话框的 Keyboard 页（tools / help 两个菜单共用这一个动作）。
    ShowKeymapSettings,
    // 标签页上/下一个：循环切换，动作是工作台自己的 `gpui::actions!`。
    NextTab,
    PreviousTab,
    // 文件生命周期：执行分支在 `apply_menu_action`（无活动缓冲时 no-op + 诊断，见裁决 #7），
    // 所以都没有 `CommandId`（列进 `NON_COMMAND`）。文案键沿用 File 菜单里原本挂着的占位项。
    NewTab,
    NewFile,
    SaveAs,
    SaveAll,
    RevertFile,
    // 窗口 / 工程生命周期：执行分支在 `apply_menu_action`（`window.remove_window()` /
    // `cx.quit()` / 换根回欢迎页），所以没有可派发的 Action，也没有 `CommandId`。
    CloseFolder,
    CloseWindow,
    Quit,
}

/// 一行菜单的图标。
///
/// **口径**（见 `shared::icons::AppIcon` 的文档）：应用图标一律取自本仓自己的
/// `rust/lithe-gpui/assets/icons/idea/**`；自己**没画过**那张图的行就留一个空槽，
/// 不拿别的图标集顶替：没画过就是空槽，不找一个「差不多像的」换上。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuIcon {
    Idea(&'static idea::IdeaIcon),
    /// 我们没画过这张图。
    Empty,
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
            Self::GoToImplementation => "lithe.menu.goToImplementation",
            Self::GoToTypeDefinition => "lithe.menu.goToTypeDefinition",
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
            Self::GoToReferences => "lithe.menu.goToReferences",
            Self::RenameSymbol => "lithe.menu.renameSymbol",
            Self::FormatDocument => "lithe.menu.formatDocument",
            Self::ZoomIn => "lithe.menu.zoomIn",
            Self::ZoomOut => "lithe.menu.zoomOut",
            Self::ResetZoom => "lithe.menu.resetZoom",
            Self::ToggleMenuBar => "lithe.menu.toggleMenuBar",
            Self::ToggleComment => "lithe.menu.toggleComment",
            Self::DuplicateLine => "lithe.menu.duplicateLine",
            Self::DeleteLine => "lithe.menu.deleteLine",
            Self::MoveLineUp => "lithe.menu.moveLineUp",
            Self::MoveLineDown => "lithe.menu.moveLineDown",
            Self::ToggleMinimap => "lithe.menu.toggleMinimap",
            Self::ToggleLineNumbers => "lithe.menu.toggleLineNumbers",
            Self::ToggleRenderWhitespace => "lithe.menu.toggleRenderWhitespace",
            Self::GoBack => "lithe.menu.goBack",
            Self::GoForward => "lithe.menu.goForward",
            Self::ShowKeymapSettings => "lithe.menu.keyboardShortcuts",
            Self::NextTab => "lithe.menu.nextTab",
            Self::PreviousTab => "lithe.menu.previousTab",
            Self::NewTab => "lithe.menu.newTab",
            Self::NewFile => "lithe.menu.newFile",
            Self::SaveAs => "lithe.menu.saveAs",
            Self::SaveAll => "lithe.menu.saveAll",
            Self::RevertFile => "lithe.menu.revertFile",
            // 三条沿用 File 菜单里原本挂着的占位项文案键（文字未变，只是不再挂「尚未接入」）。
            Self::CloseFolder => "lithe.menu.closeFolder",
            Self::CloseWindow => "lithe.menu.closeWindow",
            Self::Quit => "lithe.menu.quit",
        }
    }

    pub fn label(self) -> SharedString {
        tr(self.label_key())
    }

    /// 这一行在 `ctx` 下能不能点 —— 菜单置灰的唯一真源。
    ///
    /// **只影响菜单置灰，不改变执行路径**：键位与命令面板仍会走到 `apply_menu_action`，
    /// 那里保持既有的「no-op + 诊断」双保险。所以这里判错了最坏也只是「该灰的没灰」，
    /// 不会执行出错误行为。
    ///
    /// 与 [`Self::menu_key`]（这一行**有没有**快捷键）刻意解耦：前者是静态事实（绑没绑键），
    /// 后者是动态事实（此刻能不能用）。
    ///
    /// 没有在下表里点名的变体一律**可用**（`_ => true`）：打开/新建类、界面开关、全局搜索、
    /// 窗口、帮助都不依赖活动缓冲；拿不到低成本判据的（已关闭标签页栈、终端标签页数）
    /// 也暂时维持常亮——宁可错亮，也不要因为拿不到数据把一行钉死成灰。
    pub const fn is_enabled(self, ctx: EditorContext) -> bool {
        match self {
            // —— 只要有活动缓冲就能说有意义的一批：对象都是「当前这个缓冲」——
            Self::CloseTab
            | Self::Undo
            | Self::Redo
            | Self::Cut
            | Self::Copy
            | Self::Paste
            | Self::SelectAll
            | Self::Find
            | Self::FindAndReplace
            | Self::ToggleComment
            | Self::DuplicateLine
            | Self::DeleteLine
            | Self::MoveLineUp
            | Self::MoveLineDown
            | Self::GoToLine
            | Self::QuickFix
            | Self::ShowHover
            | Self::GoToDefinition
            | Self::GoToImplementation
            | Self::GoToTypeDefinition
            | Self::GoToReferences
            | Self::RenameSymbol
            | Self::FormatDocument
            | Self::SaveAs => ctx.has_active_buffer,

            // —— 有活动缓冲 **且** 能写回：二进制 / 有损转码的缓冲写不回磁盘 ——
            Self::Save => ctx.has_active_buffer && ctx.active_writable,

            // —— 必须是一个磁盘文件：未命名 scratch 没有可还原的源头，也没有
            //    可落下断点的行（`ShellWorkspace::toggle_breakpoint_at_caret`
            //    无文件时走的正是 skipped/no-file 那条分支）——
            //
            //    `RevertFile` **不看 `active_writable`**：执行侧 `revert_active` 只判
            //    `active` 与 `is_disk()`，从不看可写性。有损 UTF-8 转码的缓冲
            //    `writable == false`，可它的正文恰恰是文件内容的近似转写 —— 用户改坏了
            //    最需要「还原」的就是这种文件，带上 `active_writable` 会把这条唯一的
            //    逃生通道灰掉。
            Self::RevertFile => ctx.has_active_disk_file,
            Self::ToggleBreakpoint => ctx.has_active_disk_file,

            // —— 按标签页数：不和当前是哪个标签页打交道 ——
            Self::SaveAll | Self::CloseAllTabs | Self::CloseSavedTabs => ctx.tab_count > 0,
            Self::CloseOtherTabs | Self::NextTab | Self::PreviousTab => ctx.tab_count > 1,

            // —— 按活动标签页的下标：左边至少得有一个邻居 ——
            Self::CloseTabsToLeft => match ctx.active_tab_index {
                Some(index) => index > 0,
                None => false,
            },
            Self::CloseTabsToRight => match ctx.active_tab_index {
                Some(index) => index + 1 < ctx.tab_count,
                None => false,
            },

            // —— 跳转历史：有没有可去的目标由历史自己答，但**必须**叠加活动缓冲 ——
            //
            //    `go_back` / `go_forward` 的第一句都是 `current_entry(cx)`，而它第一行是
            //    `self.active?`；`close()` 又不清跳转历史。于是「有跳转历史、一个标签页
            //    都没开」是可达状态 —— 只按 `can_go_*` 判，会画出一条亮着、带 ⌘[ / ⌘]、
            //    点了却完全没反应的死行，正是这一整单要消灭的东西。
            Self::GoBack => ctx.has_active_buffer && ctx.can_go_back,
            Self::GoForward => ctx.has_active_buffer && ctx.can_go_forward,

            // —— 关掉过的标签页：`reopen_closed_tab` 在 `self.closed.pop()` 为 `None` 时
            //    直接返回。这一行带 ⌘⇧T 且摆在 File 的关闭组里，在本批主场景（刚开项目、
            //    还没关过任何文件）下必然失效 —— 不判就是「亮着 + 有快捷键 + 点了无反应」。
            Self::ReopenClosedTab => ctx.has_closed_tabs,

            // —— 其余：新建/打开类、显示开关、全局搜索、窗口、帮助、未接线行 ——
            _ => true,
        }
    }

    /// 本动作的图标（见 [`MenuIcon`] 的口径：我们没画过那张图就是空槽）。
    pub fn icon(self) -> MenuIcon {
        match self {
            Self::OpenFolder => MenuIcon::Idea(&idea::FOLDER_ICON),
            Self::OpenFile => MenuIcon::Idea(&idea::FILE_ICON),
            Self::Save => MenuIcon::Idea(&idea::FLOPPY_DISK_ICON),
            Self::CloseTab => MenuIcon::Idea(&idea::X_ICON),
            Self::CloseOtherTabs => MenuIcon::Idea(&idea::FILES_ICON),
            Self::ReopenClosedTab => MenuIcon::Idea(&idea::REFRESH_ICON),
            Self::CommandPalette => MenuIcon::Idea(&idea::MAGNIFYING_GLASS_ICON),
            Self::ToggleTerminal | Self::NewTerminalTab | Self::CloseTerminalTab => {
                MenuIcon::Idea(&idea::TERMINAL_ICON)
            }
            Self::ToggleMaven => MenuIcon::Idea(&idea::MAVEN_ICON),
            Self::OpenAppearanceSettings | Self::Preferences => MenuIcon::Idea(&idea::GEAR_ICON),
            Self::GoToDefinition => MenuIcon::Idea(&idea::ARROW_RIGHT_ICON),
            // 本仓的 idea 资产里没有「转到实现 / 转到类型定义」这两张图 ⇒ 留空槽，
            // 不拿 `IMPLEMENTED_METHOD_ICON` 之类语义不同的图顶替（见 `MenuIcon` 的口径）。
            Self::GoToImplementation | Self::GoToTypeDefinition => MenuIcon::Empty,
            Self::GoToReferences => MenuIcon::Idea(&idea::TOOL_WINDOW_FIND_ICON),
            Self::FormatDocument => MenuIcon::Idea(&idea::ROWS_ICON),
            // 本仓 idea 资产里没有「重命名」那张图 ⇒ 空槽，不拿语义不同的图顶替。
            Self::RenameSymbol => MenuIcon::Empty,
            Self::QuickOpen => MenuIcon::Idea(&idea::MAGNIFYING_GLASS_ICON),
            Self::SearchEverywhere => MenuIcon::Idea(&idea::GLOBE_HEMISPHERE_WEST_ICON),
            Self::Undo => MenuIcon::Idea(&idea::ARROW_COUNTER_CLOCKWISE_ICON),
            Self::Redo => MenuIcon::Idea(&idea::ARROW_CLOCKWISE_ICON),
            Self::Cut => MenuIcon::Idea(&idea::SCISSORS_ICON),
            Self::Copy => MenuIcon::Idea(&idea::COPY_ICON),
            Self::Find => MenuIcon::Idea(&idea::MAGNIFYING_GLASS_ICON),
            Self::QuickFix => MenuIcon::Idea(&idea::LIGHTBULB_ICON),
            Self::ShowFileExplorer => MenuIcon::Idea(&idea::FILES_ICON),
            Self::ShowSourceControl => MenuIcon::Idea(&idea::GIT_BRANCH_ICON),
            Self::ShowRunAndDebug | Self::StartDebugging => MenuIcon::Idea(&idea::PLAY_ICON),
            Self::StopDebugging => MenuIcon::Idea(&idea::STOP_ICON),
            Self::GoToLine => MenuIcon::Idea(&idea::HASH_ICON),
            Self::ShowHover => MenuIcon::Idea(&idea::INFO_ICON),
            Self::ZoomIn => MenuIcon::Idea(&idea::MAGNIFYING_GLASS_PLUS_ICON),
            Self::ZoomOut => MenuIcon::Idea(&idea::MAGNIFYING_GLASS_MINUS_ICON),
            Self::ResetZoom => MenuIcon::Idea(&idea::REFRESH_ICON),
            Self::ToggleMenuBar => MenuIcon::Idea(&idea::FILES_ICON),
            Self::DeleteLine => MenuIcon::Idea(&idea::TRASH_ICON),
            Self::MoveLineUp => MenuIcon::Idea(&idea::ARROW_UP_ICON),
            Self::MoveLineDown => MenuIcon::Idea(&idea::ARROW_DOWN_ICON),
            Self::GoBack => MenuIcon::Idea(&idea::ARROW_LEFT_ICON),
            Self::GoForward => MenuIcon::Idea(&idea::ARROW_RIGHT_ICON),
            Self::ShowKeymapSettings => MenuIcon::Idea(&idea::KEYBOARD_ICON),
            Self::NextTab => MenuIcon::Idea(&idea::CARET_RIGHT_ICON),
            Self::PreviousTab => MenuIcon::Idea(&idea::CARET_LEFT_ICON),
            // 新建两个沿用占位项那枚「新文档」图；`SaveAs`/`SaveAll` 我们没画过（不给三个软盘
            // 动作共用同一枚图，那比空槽更难认，见视觉稿 §10 的 A3）；`RevertFile` 是一处登记
            // 在案的语义借用（刷新 ≠ 还原）。
            // 上游那批借来的图标（`CloseAllTabs` 的方框叉、`Paste` 的剪贴板、
            // 窗口那三个、断点、自动换行…）我们自己的图标集里没有，**留空**。
            Self::NewTab | Self::NewFile => MenuIcon::Idea(&idea::FILE_PLUS_ICON),
            Self::SaveAs | Self::SaveAll => MenuIcon::Empty,
            Self::RevertFile => MenuIcon::Idea(&idea::REFRESH_ICON),
            Self::CloseAllTabs
            | Self::CloseSavedTabs
            | Self::CloseTabsToLeft
            | Self::CloseTabsToRight
            | Self::ToggleStatusBar
            | Self::ReplaceInFiles
            | Self::NewWindow
            | Self::Minimize
            | Self::Maximize
            | Self::ToggleFullscreen
            | Self::Paste
            | Self::SelectAll
            | Self::FindAndReplace
            | Self::ToggleActivitySidebar
            | Self::ToggleSecondarySidebar
            | Self::ShowDiagnostics
            | Self::ToggleBreakpoint
            | Self::ToggleWordWrap
            | Self::ToggleComment
            | Self::DuplicateLine
            | Self::ToggleMinimap
            | Self::ToggleLineNumbers
            | Self::ToggleRenderWhitespace
            // 「关闭文件夹 / 关闭窗口 / 退出」三张图我们也没画过，按同一口径留空槽。
            | Self::CloseFolder
            | Self::CloseWindow
            | Self::Quit => MenuIcon::Empty,
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
            Self::GoBack => Some(KEY_GO_BACK),
            // 真源自己写着「使用 Shift+F12 查找引用」（`references.findReferencesHint`），
            // 这条键在 `lithe_gpui_editor::install_actions` 里真绑了，所以胶囊画得出、按得出生效。
            Self::GoToReferences => Some(KEY_GO_TO_REFERENCES),
            Self::GoForward => Some(KEY_GO_FORWARD),
            // 查找 / 替换：`install_key_actions` 里绑的正是这两个键（`secondary-f` / `secondary-r`），
            // 而它们的 `keybind_action()` 现在也真有值（见下面的注释），所以菜单画得出、按得出生效。
            Self::Find => Some(KEY_FIND),
            Self::FindAndReplace => Some(KEY_FIND_REPLACE),
            // 行编辑五个：键位登记在 `lithe_ui::editor` 的 `ElyEditor` 上下文里
            // （IDEA 默认键，三端同键），胶囊照旧从这些字面值画。
            // `DuplicateLine` 是 `secondary-shift-d`（**不是** `secondary-d`）：后者的多光标
            // `SelectNext` 已经占了，一个键不能命名两个动作（F1）。
            Self::ToggleComment => Some("secondary-/"),
            Self::DuplicateLine => Some("secondary-shift-d"),
            Self::DeleteLine => Some("secondary-y"),
            Self::MoveLineUp => Some("secondary-alt-shift-up"),
            Self::MoveLineDown => Some("secondary-alt-shift-down"),
            // 切标签页：应用级惯例（Ctrl+Tab 下一个 / Ctrl+Shift+Tab 上一个），三端同键，
            // 与 `ctrl-shift-t`（重开关掉的标签页）不冲突。
            Self::NextTab => Some("ctrl-tab"),
            Self::PreviousTab => Some("ctrl-shift-tab"),
            _ => None,
        }
    }

    /// 这行要派发给谁 —— 菜单上的键位胶囊也从它查（`highest_precedence_binding_for_action`）。
    ///
    /// 编辑类的六个走组件库编辑器自己的动作（`lithe_ui::editor`，与 `ElyEditor` 上下文里
    /// 真登记的键位是同一批类型）；工作台自己的动作是本地 `gpui::actions!` 声明的那几个。
    pub fn keybind_action(self) -> Option<Box<dyn gpui::Action>> {
        use crate::command_palette::{OpenCommandPalette, ToggleMenuBar as ToggleMenuBarAction};

        Some(match self {
            Self::OpenFolder => Box::new(crate::workspace::OpenProjectFolder),
            Self::Save => Box::new(lithe_gpui_editor::SaveBuffer),
            Self::CloseTab => Box::new(lithe_gpui_editor::CloseActiveTab),
            Self::GoToDefinition => Box::new(lithe_gpui_editor::NavigateToDefinition),
            // 实现 / 类型定义：菜单派发要有个真被登记的 Action，但今天**不绑键** ——
            // `menu_key()` 因此返回 `None`，菜单上不会画一颗按不出的胶囊。
            Self::GoToImplementation => Box::new(lithe_gpui_editor::NavigateToImplementation),
            Self::GoToTypeDefinition => Box::new(lithe_gpui_editor::NavigateToTypeDefinition),
            Self::GoToReferences => Box::new(lithe_gpui_editor::GoToReferences),
            // 重命名与格式化：菜单派发走 `apply_menu_action` 直接调宿主方法（重命名要先弹对话框、
            // 格式化没有 UI），所以没有可派发的 Action；`menu_key()` 因此返回 `None`。
            Self::RenameSymbol => Box::new(lithe_gpui_editor::RenameSymbol),
            Self::FormatDocument => Box::new(lithe_gpui_editor::FormatDocument),
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
            Self::Undo => Box::new(lithe_ui::editor::Undo),
            Self::Redo => Box::new(lithe_ui::editor::Redo),
            Self::Cut => Box::new(lithe_ui::editor::Cut),
            Self::Copy => Box::new(lithe_ui::editor::Copy),
            Self::Paste => Box::new(lithe_ui::editor::Paste),
            Self::SelectAll => Box::new(lithe_ui::editor::SelectAll),
            // 行编辑五个：与撤销/复制同一条路 —— 派发给聚焦的编辑器，
            // 动作类型就是 `ElyEditor` 上下文里登记的那几个。
            Self::ToggleComment => Box::new(lithe_ui::editor::ToggleComment),
            Self::DuplicateLine => Box::new(lithe_ui::editor::DuplicateLine),
            Self::DeleteLine => Box::new(lithe_ui::editor::DeleteLine),
            Self::MoveLineUp => Box::new(lithe_ui::editor::MoveLineUp),
            Self::MoveLineDown => Box::new(lithe_ui::editor::MoveLineDown),
            Self::GoBack => Box::new(GoBack),
            Self::GoForward => Box::new(GoForward),
            // 查找 / 替换：`lithe_gpui_editor` 的 `OpenFind` / `OpenFindReplace` 是宿主在
            // `install_key_actions` 里真绑了键的动作（`secondary-f` / `secondary-r`），菜单派发
            // 也走 `apply_menu_action`（`pane.open_find(...)`），所以键位胶囊能画、按得出生效。
            Self::Find => Box::new(lithe_gpui_editor::OpenFind),
            Self::FindAndReplace => Box::new(lithe_gpui_editor::OpenFindReplace),
            // 「快速修复」由工作台自己调编辑器的代码动作菜单（`pane.show_code_actions`），
            // 不走 `dispatch_action`，所以没有 Action 可查。
            Self::QuickFix => return None,
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
            // 显示开关没有可派发的动作：它们改的是 settings store 里的档位，
            // 由外壳自己在 `run_menu_action` 里翻（和 `ToggleWordWrap` 一条路）。
            Self::ToggleMinimap | Self::ToggleLineNumbers | Self::ToggleRenderWhitespace => {
                return None;
            }
            // 快捷键一览由外壳直接开设置对话框的 Keyboard 页，没有 Action 可派发。
            Self::ShowKeymapSettings => return None,
            Self::NextTab => Box::new(NextTab),
            Self::PreviousTab => Box::new(PreviousTab),
            // 文件生命周期五个：执行分支都在 `apply_menu_action`（无活动缓冲时 no-op + 诊断，
            // 见裁决 #7），没有可派发的 Action，菜单上也不画键位胶囊。
            Self::NewTab
            | Self::NewFile
            | Self::SaveAs
            | Self::SaveAll
            | Self::RevertFile
            | Self::CloseFolder
            | Self::CloseWindow
            | Self::Quit => {
                return None;
            }
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
            Self::GoToImplementation => Some(CommandId::NavigateToImplementation),
            Self::GoToTypeDefinition => Some(CommandId::NavigateToTypeDefinition),
            Self::GoToReferences => Some(CommandId::GoToReferences),
            Self::RenameSymbol => Some(CommandId::RenameSymbol),
            Self::FormatDocument => Some(CommandId::FormatDocument),
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
            | Self::ShowHover
            | Self::ToggleComment
            | Self::DuplicateLine
            | Self::DeleteLine
            | Self::MoveLineUp
            | Self::MoveLineDown
            | Self::ToggleMinimap
            | Self::ToggleLineNumbers
            | Self::ToggleRenderWhitespace
            | Self::GoBack
            | Self::GoForward
            | Self::ShowKeymapSettings
            | Self::NextTab
            | Self::PreviousTab
            // 文件生命周期五个：执行分支在 `apply_menu_action`，所以都没有 `CommandId`。
            | Self::NewTab
            | Self::NewFile
            | Self::SaveAs
            | Self::SaveAll
            | Self::RevertFile
            // 窗口 / 工程生命周期三条同理：没有 `CommandId`，也不进命令面板。
            | Self::CloseFolder
            | Self::CloseWindow
            | Self::Quit => None,
        }
    }
}

pub static MENUS: &[TopMenu] = &[
    TopMenu {
        id: "file",
        label_key: "lithe.menu.file",
        items: &[
            MenuItem::Action(MenuAction::NewTab),
            MenuItem::Action(MenuAction::NewWindow),
            MenuItem::Action(MenuAction::NewFile),
            MenuItem::Action(MenuAction::OpenFolder),
            MenuItem::Action(MenuAction::OpenFile),
            MenuItem::Action(MenuAction::CloseFolder),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::Save),
            MenuItem::Action(MenuAction::SaveAs),
            MenuItem::Action(MenuAction::SaveAll),
            MenuItem::Action(MenuAction::RevertFile),
            MenuItem::NotWired {
                label_key: "lithe.menu.showLocalHistory",
                missing: MISSING_LOCAL_HISTORY,
                icon: MenuIcon::Idea(&idea::CLOCK_COUNTER_CLOCKWISE_ICON),
            },
            MenuItem::Separator,
            MenuItem::Action(MenuAction::CloseTab),
            MenuItem::Action(MenuAction::CloseWindow),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::CloseOtherTabs),
            MenuItem::Action(MenuAction::CloseAllTabs),
            MenuItem::Action(MenuAction::CloseSavedTabs),
            MenuItem::Action(MenuAction::CloseTabsToLeft),
            MenuItem::Action(MenuAction::CloseTabsToRight),
            MenuItem::Action(MenuAction::ReopenClosedTab),
            MenuItem::Action(MenuAction::Quit),
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
            MenuItem::Action(MenuAction::ToggleComment),
            MenuItem::Action(MenuAction::QuickFix),
            MenuItem::NotWired {
                label_key: "lithe.menu.triggerParameterHints",
                missing: MISSING_LSP_REQUESTS,
                icon: MenuIcon::Empty,
            },
            MenuItem::Action(MenuAction::ShowHover),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::DuplicateLine),
            MenuItem::Action(MenuAction::DeleteLine),
            MenuItem::Action(MenuAction::MoveLineUp),
            MenuItem::Action(MenuAction::MoveLineDown),
            MenuItem::Action(MenuAction::FormatDocument),
            // 格式化**选区**仍未接线：Core 只有整篇的 `textDocument/formatting`，
            // 没有 `rangeFormatting`，宿主拿不到「只格式化这一段」的请求形态。
            MenuItem::NotWired {
                label_key: "lithe.menu.formatSelection",
                missing: MISSING_LSP_REQUESTS,
                icon: MenuIcon::Empty,
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
                icon: MenuIcon::Empty,
            },
            MenuItem::Action(MenuAction::ShowRunAndDebug),
            MenuItem::Separator,
            MenuItem::NotWired {
                label_key: "lithe.menu.splitEditor",
                missing: MISSING_EDITOR_FEATURES,
                icon: MenuIcon::Idea(&idea::COLUMNS_ICON),
            },
            MenuItem::Action(MenuAction::ToggleMinimap),
            MenuItem::Action(MenuAction::ToggleWordWrap),
            MenuItem::Action(MenuAction::ToggleLineNumbers),
            MenuItem::Action(MenuAction::ToggleRenderWhitespace),
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
            MenuItem::Action(MenuAction::GoBack),
            MenuItem::Action(MenuAction::GoForward),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::GoToDefinition),
            MenuItem::Action(MenuAction::GoToImplementation),
            MenuItem::Action(MenuAction::GoToTypeDefinition),
            MenuItem::Action(MenuAction::GoToReferences),
            MenuItem::Action(MenuAction::RenameSymbol),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::NextTab),
            MenuItem::Action(MenuAction::PreviousTab),
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
            MenuItem::Action(MenuAction::ShowKeymapSettings),
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
            MenuItem::Action(MenuAction::ShowKeymapSettings),
            MenuItem::NotWired {
                label_key: "lithe.menu.whatsNew",
                missing: MISSING_HELP_ABOUT,
                icon: MenuIcon::Empty,
            },
            MenuItem::NotWired {
                label_key: "lithe.menu.changelog",
                missing: MISSING_HELP_ABOUT,
                icon: MenuIcon::Empty,
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

/// 菜单栏的四种摆法。
///
/// 用户拍板（2026-10-10）要「完整版」：既能抬出独立一行（`Above`），也要「默认藏着、
/// 鼠标移到顶上才浮出来」（`AutoHide`）。
///
/// - `Merged` —— 合进标题栏那一行，与标题栏共用底色。**不要给它刷底**：它继承父级
///   `title_bar`，刷了同色零收益。分隔线画在 `title_bar()` 的 `v_stack()` 下沿。
/// - `Above` —— 抬出**独立的一行**，压在标题栏上方。因为是独立一行，需要自己的下边框
///   与标题栏分开（见 `above_bar`）。
/// - `AutoHide` —— 默认只留一条几像素高的热区；鼠标移进去浮出 `Above` 那一行。
///   菜单**开着的时候不能收**（收了等于把人正在用的东西抽走）。
/// - `Compact` —— 一个汉堡按钮，按下去展开「菜单的菜单」。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuBarMode {
    Merged,
    Above,
    AutoHide,
    Compact,
}

impl MenuBarMode {
    pub const ALL: [MenuBarMode; 4] = [
        MenuBarMode::Merged,
        MenuBarMode::Above,
        MenuBarMode::AutoHide,
        MenuBarMode::Compact,
    ];

    /// `Merged` 是默认：与改造前 `--compact-menu-bar` 没给时的行为一致。
    pub const DEFAULT: MenuBarMode = MenuBarMode::Merged;

    pub const fn id(self) -> &'static str {
        match self {
            Self::Merged => "merged",
            Self::Above => "above",
            Self::AutoHide => "autohide",
            Self::Compact => "compact",
        }
    }

    /// 解析持久化里的那个字符串。认不出的（含旧版的 `pinned`）一律回落 `Merged`——
    /// 宁可用默认摆法，也不要因为一个认不出的值把整份设置读失败。
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|mode| mode.id() == id)
    }

    /// 只有 `AutoHide` 是「平时不占地方」的那一档：外壳要按它决定标题栏上面留不留空。
    pub const fn reserves_no_space(self) -> bool {
        matches!(self, Self::AutoHide)
    }

    /// 切到下一档。循环顺序按「占的地方从多到少」排，与 `ALL` 一致。
    pub const fn next(self) -> Self {
        match self {
            Self::Merged => Self::Above,
            Self::Above => Self::AutoHide,
            Self::AutoHide => Self::Compact,
            Self::Compact => Self::Merged,
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
    /// `AutoHide` 下鼠标是否正压在热区/菜单行上。别的模式下恒为 `false`。
    autohide_revealed: bool,
    restore_focus: bool,
    pending: Vec<MenuRequest>,
    action_context: FocusHandle,
    escape_interceptor: Option<Subscription>,
    /// 每个顶层菜单触发器的位置，由 [`Measure`] 报上来；`S1_MENU_BOUNDS` 诊断要用。
    trigger_bounds: Vec<Option<Bounds<Pixels>>>,
    /// 刚打开、几何还没报出去的那个菜单的下标。
    bounds_pending: Option<usize>,
}

impl MenuBar {
    pub fn new(mode: MenuBarMode, action_context: FocusHandle, cx: &mut App) -> Entity<Self> {
        mode.diagnose();
        let bar = cx.new(|_| Self {
            mode,
            selected: None,
            compact_visible: false,
            autohide_revealed: false,
            restore_focus: false,
            pending: Vec::new(),
            action_context,
            escape_interceptor: None,
            trigger_bounds: vec![None; MENUS.len()],
            bounds_pending: None,
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

    /// 标记自己变了，让外壳跟着重画一次。
    ///
    /// 这里**只通知自己**，不去戳 `ShellWorkspace`：菜单的关闭回调（dismiss / Escape /
    /// 选中一行）是跑在外壳的更新栈里的，那条链是 `ShellWorkspace` 在 update -> 菜单还给
    /// 中间层 -> `MenuBar::close` -> 这里。此刻外壳还在「正在被 update」的借用里，反过来
    /// `shell.update` 就是第二次可变借用，gpui 会报
    /// `cannot update ShellWorkspace while it is already being updated`。
    /// `App::defer` 也救不了它 —— defer 只挪到本轮 effect 之后，借用的边界不受它控制。
    ///
    /// 正确的做法是 `ShellWorkspace::new` 里 `cx.observe(&menu_bar, ...)`（与
    /// `project_menu` / `branch_panel` 同一排）：gpui 保证 observer 在被观察实体的 update
    /// 结束、实体归还**之后**才派发，重入由框架兜住，不需要我们再猜时机。
    fn notify_self(&mut self, cx: &mut Context<Self>) {
        cx.notify();
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

    /// 直接设成某一档（设置页用）。切走时把展开态收干净，否则旧模式的面板会残留。
    pub fn set_mode(&mut self, mode: MenuBarMode, cx: &mut Context<Self>) {
        if mode == self.mode {
            return;
        }
        self.mode = mode;
        self.reset_open_state();
        self.mode.diagnose();
        self.notify_self(cx);
    }

    pub fn toggle_mode(&mut self, cx: &mut Context<Self>) {
        self.mode = self.mode.next();
        self.reset_open_state();
        self.mode.diagnose();
        self.notify_self(cx);
    }

    /// 换模式必然要收的东西：开着的菜单、紧凑面板、待报的几何、以及 AutoHide 的浮出态。
    fn reset_open_state(&mut self) {
        let had_open = self.selected.take().is_some() || self.compact_visible;
        self.selected = None;
        self.compact_visible = false;
        self.autohide_revealed = false;
        self.bounds_pending = None;
        if had_open {
            self.restore_focus = true;
        }
    }

    /// `AutoHide` 的浮出开关。
    ///
    /// **菜单开着的时候不收**：那一档下鼠标移出热区是常事（比如划到某一级子菜单上），
    /// 收掉等于把人正在用的东西抽走。
    pub fn set_autohide_revealed(&mut self, revealed: bool, cx: &mut Context<Self>) {
        if self.mode != MenuBarMode::AutoHide {
            return;
        }
        if revealed == self.autohide_revealed {
            return;
        }
        if !revealed && self.selected.is_some() {
            return;
        }
        self.autohide_revealed = revealed;
        self.notify_self(cx);
    }

    pub fn toggle(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(menu) = MENUS.get(index) else {
            return;
        };
        let next = resolve_menu_toggle(self.nav_state(), index);
        match next {
            NavState::Closed => {
                self.close(CloseReason::Toggle, cx);
            }
            NavState::Open(opened) => {
                self.selected = Some(opened);
                self.compact_visible = false;
                self.bounds_pending = Some(opened);
                diagnose_open(menu.id);
                self.notify_self(cx);
            }
        }
    }

    fn nav_state(&self) -> NavState {
        match self.selected {
            Some(index) => NavState::Open(index),
            None => NavState::Closed,
        }
    }

    pub fn close(&mut self, reason: CloseReason, cx: &mut Context<Self>) -> bool {
        if self.selected.is_none() && !self.compact_visible {
            return false;
        }
        let had_open = self.selected.take().is_some() || self.compact_visible;
        self.selected = None;
        self.compact_visible = false;
        self.bounds_pending = None;
        if had_open {
            self.restore_focus = true;
        }
        diagnose_close(reason);
        self.notify_self(cx);
        true
    }

    /// 库自己把菜单关了（点外面、选中一行之后）：宿主这边跟着收。
    ///
    /// 起因一律记 `dismiss` —— 库只报「关了」这一件事，分不出是点了外面还是又点了
    /// 自己那块触发器（后者在开合全交给库之前是 `toggle`）。Esc 由拦截器先关掉，
    /// 轮不到这里，所以那条诊断仍是 `escape`。
    fn menu_closed(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.selected == Some(index) {
            self.close(CloseReason::Dismiss, cx);
        }
    }

    fn trigger_bounds_of(&self, index: usize) -> Option<Bounds<Pixels>> {
        self.trigger_bounds.get(index).copied().flatten()
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

    pub fn toggle_compact(&mut self, cx: &mut Context<Self>) {
        if self.compact_visible {
            self.close(CloseReason::Toggle, cx);
        } else {
            self.compact_visible = true;
            self.selected = None;
            diagnose_open("compact_bar");
            self.notify_self(cx);
        }
    }

    /// 入队一条「跑这个菜单动作」的请求。
    ///
    /// 关菜单与入队**必须都通知自己**，通知是无条件的：`close` 只在「确实有东西开着」时才
    /// 通知（没开就 `return false`，不通知），而入队不是重画问题——队列从空变非空，宿主靠
    /// `cx.observe` 感知，observe 只在实体被 notify 之后才派发。搭 `close` 的顺风车的话，
    /// 只要这一路进来时菜单已经关了（例如别的路径先收了面板），请求就会**静默地**躺在
    /// 队列里不被排空——设置页上点了没反应，比崩溃更难查。
    pub(crate) fn push_run(&mut self, action: MenuAction, cx: &mut Context<Self>) {
        self.pending.push(MenuRequest::Run(action));
        self.close(CloseReason::Select, cx);
        self.notify_self(cx);
    }

    /// 入队一条「这项还没接线」的提示，理由同 [`Self::push_run`]。
    pub(crate) fn push_not_wired(
        &mut self,
        label_key: &'static str,
        missing: Missing,
        cx: &mut Context<Self>,
    ) {
        self.pending
            .push(MenuRequest::NotWired { label_key, missing });
        self.close(CloseReason::Select, cx);
        self.notify_self(cx);
    }

    pub fn drain_runs(&mut self) -> Vec<MenuRequest> {
        std::mem::take(&mut self.pending)
    }

    pub(crate) fn remember_action_target(&mut self, focused: Option<FocusHandle>) {
        let target = focused.filter(|handle| handle != &self.action_context);
        ACTION_TARGET.with(|slot| *slot.borrow_mut() = target);
    }

    pub fn open_by_id(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let Some(index) = MENUS.iter().position(|menu| menu.id == id) else {
            return false;
        };
        self.selected = None;
        self.compact_visible = false;
        self.bounds_pending = None;
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

    pub fn run_action(&mut self, action: MenuAction, cx: &mut Context<Self>) {
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

/// 从持久化里的字符串取模式；认不出（包括没给）一律回落 [`MenuBarMode::DEFAULT`]。
pub fn mode_from_id(id: Option<&str>) -> MenuBarMode {
    id.and_then(MenuBarMode::from_id)
        .unwrap_or(MenuBarMode::DEFAULT)
}

/// 旧的 `compact_menu: bool` 兼容通道：它只能表达两档，所以 `true` → `Compact`、
/// `false` → 默认档。新代码一律走 [`mode_from_id`]。
pub fn mode_for(compact: bool) -> MenuBarMode {
    if compact {
        MenuBarMode::Compact
    } else {
        MenuBarMode::DEFAULT
    }
}

thread_local! {
    static MENU_BAR: std::cell::RefCell<Option<gpui::WeakEntity<MenuBar>>> =
        const { std::cell::RefCell::new(None) };

    static ACTION_TARGET: std::cell::RefCell<Option<FocusHandle>> =
        const { std::cell::RefCell::new(None) };
}

pub fn action_target() -> Option<FocusHandle> {
    ACTION_TARGET.with(|slot| slot.borrow().clone())
}

pub(crate) fn set_menu_bar(bar: gpui::WeakEntity<MenuBar>) {
    MENU_BAR.with(|slot| *slot.borrow_mut() = Some(bar));
}

pub fn handle() -> gpui::WeakEntity<MenuBar> {
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

pub(crate) fn try_handle() -> Option<gpui::WeakEntity<MenuBar>> {
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

/// 画菜单栏。`ctx` 是外壳现取的那份启用上下文（见 [`EditorContext`]），按值一路传到
/// [`build_menu`]；之所以显式传而不是放 thread-local，是因为多窗口时后渲染的窗会覆盖
/// 前一个窗的全局槽。
pub fn menu_bar(
    bar: &mut MenuBar,
    ctx: EditorContext,
    window: &mut Window,
    cx: &mut Context<MenuBar>,
) -> impl IntoElement {
    if bar.restore_focus {
        bar.restore_focus = false;
        window.focus(&bar.action_context, cx);
    }
    let _ = window;
    match bar.mode {
        MenuBarMode::Merged => merged_bar(bar, ctx, cx).into_any_element(),
        MenuBarMode::Above => above_bar(bar, ctx, cx).into_any_element(),
        MenuBarMode::AutoHide => autohide_bar(bar, ctx, cx).into_any_element(),
        MenuBarMode::Compact => compact_bar(bar, ctx, cx).into_any_element(),
    }
}

pub(crate) fn render_bar(
    bar: &Entity<MenuBar>,
    ctx: EditorContext,
    window: &mut Window,
    cx: &mut App,
) -> Option<gpui::AnyElement> {
    bar.downgrade()
        .update(cx, |bar, cx| {
            menu_bar(bar, ctx, window, cx).into_any_element()
        })
        .ok()
}

/// 九个顶层菜单横排。三种「把菜单摆出来」的模式共用这一排，差别只在容器。
fn menu_row(bar: &mut MenuBar, ctx: EditorContext, cx: &mut App) -> impl IntoElement {
    let mut row = h_stack()
        .flex_shrink_0()
        .items_center()
        .gap(rems(BAR_GAP_SPEC / 16.));
    for index in 0..MENUS.len() {
        row = row.child(menu_item(bar, index, ctx, cx));
    }
    row
}

/// `Merged`：合进标题栏那一行，**不刷底**（继承父级 `title_bar`，刷了同色零收益）。
fn merged_bar(bar: &mut MenuBar, ctx: EditorContext, cx: &mut App) -> impl IntoElement {
    h_stack()
        .id("lithe-menu-bar-merged")
        .flex_shrink_0()
        .items_center()
        .h(rems(BAR_HEIGHT_SPEC / 16.))
        .p(rems(BAR_PADDING_SPEC / 16.))
        .child(menu_row(bar, ctx, cx))
}

/// `Above`：抬出**独立的一行**，压在标题栏上方。
///
/// 与 `Merged` 的差别不只是位置——它是独立的一层，得自己跟下面分开，否则看起来就是
/// 标题栏被劈成了两截。这里用下边框而不是底色：标题栏与页面都是 `title_bar` /
/// `bg` 那一档，换成别的底色会在窗口顶部切出一条异色带。
fn above_bar(bar: &mut MenuBar, ctx: EditorContext, cx: &mut App) -> impl IntoElement {
    let colors = &cx.theme().colors;
    h_stack()
        .id("lithe-menu-bar-above")
        .flex_shrink_0()
        .items_center()
        .h(rems(BAR_HEIGHT_SPEC / 16.))
        .px(rems(BAR_PADDING_SPEC / 16.))
        .border_b_1()
        .border_color(colors.border)
        .child(menu_row(bar, ctx, cx))
}

/// `AutoHide`：平时只留一条热区，鼠标压上去才浮出 `Above` 那一行。
///
/// 为什么不干脆用 `on_hover` 挂在那一行自己身上——**藏起来之后那一行就不在场景里了**，
/// 没有元素能收 hover。所以必须留一条常驻的热区：它不吃地方（几像素高），但一直能
/// 接到鼠标进出。浮出之后热区与菜单行一起在，鼠标在两者之间移动不会触发收起。
fn autohide_bar(bar: &mut MenuBar, ctx: EditorContext, cx: &mut App) -> impl IntoElement {
    let revealed = bar.autohide_revealed;
    let handle = handle();
    let zone = div()
        .id("lithe-menu-bar-autohide")
        .w_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        // 藏起来时这条热区自己撑出 `AUTOHIDE_HOT_ZONE_SPEC`；浮出后高度交给菜单行。
        .when(!revealed, |zone| zone.h(rems(AUTOHIDE_HOT_ZONE_SPEC / 16.)))
        .on_hover(move |hovered, _, cx| {
            let _ = handle.update(cx, |bar, cx| bar.set_autohide_revealed(*hovered, cx));
        });

    if revealed {
        zone.child(above_bar(bar, ctx, cx))
    } else {
        zone
    }
}

/// 紧凑模式：一个按钮，按下去展开**菜单的菜单** —— 每个顶层菜单是一张子菜单。
///
/// 原来那套是「按钮下面浮一排顶层菜单触发器，鼠标划过去换一个」，那排浮层要靠上游
/// 的 `Positioner`（贴边、限窗、外挂 `deferred`）才放得稳；库这边没有对应的外挂定位，
/// 于是换成子菜单：面板挂哪儿、怎么走键盘都由 [`TriggerMenu`] 自己管。
fn compact_bar(bar: &mut MenuBar, ctx: EditorContext, cx: &mut App) -> impl IntoElement {
    let handle = handle();
    let button_handle = handle.clone();
    let open = bar.compact_visible;
    // 面板只在开着的时候建：关着的时候库也不画它。
    let panel = if open {
        compact_menu(ctx, cx)
    } else {
        Menu::new()
    };

    let colors = &cx.theme().colors;
    let (open_bg, open_fg) = (colors.active, colors.fg);
    TriggerMenu::new("lithe-menu-bar-compact", panel, move |open, _window, cx| {
        // 展开态要留在按钮上：汉堡按钮按下去之后面板顶掉了整条菜单栏，按钮是唯一还在
        // 屏幕上的、能说明「菜单是从这儿开的」的东西。
        //
        // 底色光写 `.bg(active)` 不够：Ghost 那档 hover 底压在外层自定义样式之上，而点完
        // 之后指针本来就还停在按钮上 —— 那一档会被盖成 hover，"开着"和"只是指着"就又分不
        // 出来了。所以展开时换成 `Custom` variant，把静置 / 悬停 / 按下三档一起锁到 `active`
        // （与 Pinned 的展开态同槽）。
        let variant = if open {
            ButtonVariant::Custom(
                ButtonCustomVariant::new(cx)
                    .color(open_bg)
                    .hover(open_bg)
                    .active(open_bg)
                    .foreground(open_fg),
            )
        } else {
            ButtonVariant::Ghost
        };
        Button::new("lithe-menu-bar-trigger")
            .variant(variant)
            .size(Size::Small)
            .icon_element(idea_icon_mono(&idea::FILES_ICON, cx.theme().is_dark()))
            .tooltip(tr("lithe.window.menu"))
            .aria_label(tr("lithe.window.menu"))
            .on_click(move |_: &ClickEvent, _window: &mut Window, cx: &mut App| {
                let _ = button_handle.update(cx, |bar, cx| bar.toggle_compact(cx));
            })
            .into_any_element()
    })
    .open(Some(open))
    .on_toggle(move |open, cx| {
        let _ = handle.update(cx, |bar, cx| {
            if open {
                bar.compact_visible = true;
            } else {
                bar.close(CloseReason::Dismiss, cx);
            }
        });
    })
    .into_any_element()
}

/// 紧凑模式那张面板：一个顶层菜单一行，行里是它的子菜单。
///
/// 九个行全是子菜单，**父项一律不置灰**：禁用的子菜单是可达性黑洞（`Menu::step` 会跳过
/// 禁用行），而每个顶层菜单里都还有常亮项（File 有 OpenFile、Edit 有 CommandPalette…）。
fn compact_menu(ctx: EditorContext, cx: &mut App) -> Menu {
    let mut menu = Menu::new();
    for index in 0..MENUS.len() {
        menu = menu.item(MenuRow::submenu(
            MENUS[index].label(),
            build_menu(index, ctx, cx),
        ));
    }
    menu
}

/// 顶层菜单触发器的一张脸：文字色、底、内描边，全在这一张表里。
///
/// 五个状态的配色是照 IDEA 的菜单栏对出来的（2026-10-11）：
///
/// | 状态 | 文字 | 底 | 内描边 |
/// | --- | --- | --- | --- |
/// | idle | `fg` | 无 | 无 |
/// | hover | `fg` | `hover` | 无 |
/// | open | `fg` | `active` | 无 |
/// | active（键盘焦点落在这一格） | `fg` | `active` | `fg` 1px |
/// | disabled | `fg_disabled` | 无 | 无 |
///
/// 两条硬规矩：
///
/// - 文字一律 `fg` 而不用 `fg_muted`：浅色 `title_bar` 上 `fg_muted` 只剩 4.56:1，菜单栏
///   天天盯着看，余量不够。
/// - 内描边**不用 `focus` token**：那一个落在 `title_bar`（深 `#313335`）上只有 2.49:1，
///   蹲不到 WCAG 的 3:1；用 `fg` 自己画 1px 反而稳过。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TriggerFace {
    Idle,
    Hover,
    Open,
    Active,
    Disabled,
}

impl TriggerFace {
    /// 三件事合成一张脸：`disabled` 最大，其次 `focused`，最后才是开合。
    fn of(open: bool, disabled: bool, focused: bool) -> Self {
        if disabled {
            return Self::Disabled;
        }
        if focused {
            return Self::Active;
        }
        if open { Self::Open } else { Self::Idle }
    }

    /// 文字色、底（`None` = 不画底）、内描边（`None` = 透明）。
    ///
    /// 底盘取 `chrome_hover` / `chrome_active` 而不是 `hover` / `active`：触发器画在
    /// `title_bar` 这块 chrome 面上，而 `hover`（`list.hover.background`）是贴着**列表白底**
    /// 调出来的那一档——IntelliJ Light 里它与 `title_bar` 同为 `#f2f2f2`，铺上去等于没画。
    /// `chrome_*` 是主题自己给出的「在 chrome 面上悬停/选中用什么」那一档（浅 `#d9e5f2` /
    /// 深 `#3c3f41`、`#214283`），与 2026-10-10 那篇笔记里的其余调用点同一口径。
    fn paint(self, colors: &Palette) -> (Hsla, Option<Hsla>, Option<Hsla>) {
        match self {
            Self::Idle => (colors.fg, None, None),
            Self::Hover => (colors.fg, Some(colors.chrome_hover), None),
            // 展开态取 `chrome_active`（selection 蓝，浅 `#b3d7ff` / 深 `#214283`）而不是
            // `chrome_hover`：面板开着时鼠标多半还停在触发器上，两态同色就分不清「指着」和
            // 「开着」。
            Self::Open => (colors.fg, Some(colors.chrome_active), None),
            Self::Active => (colors.fg, Some(colors.chrome_active), Some(colors.fg)),
            Self::Disabled => (colors.fg_disabled, None, None),
        }
    }
}

/// 顶层菜单那块触发器：一行文字，开的那个高亮。
///
/// 开合归库的 [`TriggerMenu`] 管，这张脸只负责长相和两件宿主自己的事：
///
/// - 按下时记住「菜单打开前谁握着焦点」——选中的动作派发给它（见 `action_target`）；
/// - 鼠标划过来、且已经开了别的菜单时，换到这一个。
///
/// `disabled` / `focused` 这两个入参今天恒为假：九个顶层菜单都不能禁用，键盘也还没走到
/// 触发器上（顶层导航的键位方案另排期）。配色先按 [`TriggerFace`] 定死，接线的活归那一批
/// ——到时候只要把这两个入参换成真来源，不用再动颜色。
fn trigger(index: usize, open: bool, cx: &App) -> AnyElement {
    let colors = &cx.theme().colors;
    // 两个 `false` 只许出现在这一处（调用点收口）：`disabled` 等菜单行真正具备禁用来源，
    // `focused` 等顶层键盘导航接上 `FocusHandle::is_focused`（gpui-pre 0.3.8 的
    // `src/window.rs:507` 有这个句柄）。在此之前它们不是 bug，是**尚未接线**——别在别处
    // 再抄一份 `false`，接线时改这一行就够了。
    let face = TriggerFace::of(open, false, false);
    let (text, background, inner) = face.paint(colors);
    // 内描边恒留 1px 的槽：聚焦那一刻才上色，免得整行宽高跳两个像素。
    let inner = inner.unwrap_or(transparent_black());
    // 悬停底跟着状态走：展开着的时候鼠标还在触发器上，底就该停在 `chrome_active` 上；
    // 禁用态连悬停都不给——那一行什么都不该反应。
    let hover = match face {
        TriggerFace::Disabled => None,
        TriggerFace::Open | TriggerFace::Active => Some(colors.chrome_active),
        _ => TriggerFace::Hover.paint(colors).1,
    };
    let handle = handle();

    div()
        .id(("lithe-menu-face", index))
        .flex()
        .items_center()
        .h(rems(TRIGGER_HEIGHT_SPEC / 16.))
        .px(rems(TRIGGER_PADDING_SPEC / 16.))
        .rounded(rems(ITEM_RADIUS_SPEC / 16.))
        .text_sm()
        .text_color(text)
        .border_1()
        .border_color(inner)
        .when_some(background, |face, background| face.bg(background))
        .when_some(hover, |face, hover| {
            face.hover(move |style| style.bg(hover).text_color(text))
        })
        .on_mouse_down(MouseButton::Left, {
            let handle = handle.clone();
            move |_, window, cx| {
                let focused = window.focused(cx);
                let _ = handle.update(cx, |bar, _| bar.remember_action_target(focused));
            }
        })
        .on_hover(move |hovered: &bool, _window, cx| {
            if *hovered {
                let _ = handle.update(cx, |bar, cx| bar.hover_switch(index, cx));
            }
        })
        .child(MENUS[index].label())
        .into_any_element()
}

/// 一个顶层菜单：触发器 + 挂在它下面的面板，外加量它的位置给诊断用。
///
/// 开合是**受控**的（[`TriggerMenu::open`]）：宿主说开哪个，库就开哪个；库自己关掉时
/// 由 [`TriggerMenu::on_toggle`] 回报回来。
fn menu_item(
    bar: &mut MenuBar,
    index: usize,
    ctx: EditorContext,
    cx: &mut App,
) -> impl IntoElement {
    let open = bar.selected == Some(index);
    // 关着的时候面板是空的：库反正不画它，省下每帧建九份菜单。
    let panel = if open {
        build_menu(index, ctx, cx)
    } else {
        Menu::new()
    };
    let handle = handle();
    let measure_handle = handle.clone();

    // 刚打开的那一次，把触发器的几何报出去。
    if bar.bounds_pending == Some(index) {
        if let Some(bounds) = bar.trigger_bounds_of(index) {
            eprintln!(
                "S1_MENU_BOUNDS id={} x={} y={} w={} h={}",
                MENUS[index].id,
                f32::from(bounds.origin.x) as i32,
                f32::from(bounds.origin.y) as i32,
                f32::from(bounds.size.width) as i32,
                f32::from(bounds.size.height) as i32,
            );
            bar.bounds_pending = None;
        }
    }

    let item = TriggerMenu::new(
        ("lithe-menu-item", index),
        panel,
        move |open, _window, cx| trigger(index, open, cx),
    )
    .open(Some(open))
    .on_toggle(move |open, cx| {
        let _ = handle.update(cx, |bar, cx| {
            if open {
                if bar.selected != Some(index) {
                    bar.selected = Some(index);
                    bar.compact_visible = false;
                    bar.bounds_pending = Some(index);
                    diagnose_open(MENUS[index].id);
                    bar.notify_self(cx);
                }
            } else {
                bar.menu_closed(index, cx);
            }
        });
    })
    .aria_label(MENUS[index].label());

    // 触发器的位置：库只在绘制期知道，宿主得靠它量出来（`Measure` 报一次变一次）。
    Measure::new(
        ("lithe-menu-measure", index),
        move |bounds, _window, cx: &mut App| {
            let _ = measure_handle.update(cx, |bar, _| {
                if let Some(slot) = bar.trigger_bounds.get_mut(index) {
                    *slot = Some(bounds);
                }
            });
        },
    )
    .flex_shrink_0()
    .child(item)
    .into_any_element()
}

/// 一个顶层菜单的面板。主题那一行是子菜单，没有主题可选时整行不出现。
///
/// `ctx` 只用来给每一行算 [`MenuAction::is_enabled`]；菜单每次展开都现建一次
/// （见 [`menu_item`] / [`compact_menu`]），所以状态变了下一次重画就跟上。
fn build_menu(index: usize, ctx: EditorContext, cx: &mut App) -> Menu {
    let items = MENUS[index].items;
    let handle = handle();
    let names = lithe_gpui_settings::theme::theme_names(cx);
    let themes = (!names.is_empty()).then(|| theme_menu(names));

    let mut menu = Menu::new().width(rems(PANEL_MIN_WIDTH_SPEC / 16.));
    for item in items {
        menu = match item {
            MenuItem::Separator => menu.separator(),
            MenuItem::Action(action) => {
                let action = *action;
                let disabled = !action.is_enabled(ctx);
                let row_handle = handle.clone();
                let mut row = MenuRow::new(action.label()).disabled(disabled);
                if let MenuIcon::Idea(icon) = action.icon() {
                    // 图标槽是宿主自画的，库不会替它上禁用色，颜色得自己按状态给。
                    row = row.icon_element(icon_element(icon, disabled));
                }
                // 键位胶囊**照画**：「这个动作绑定了什么键」是静态事实，不因为当前不可用
                // 就不存在，抽掉它反而让用户失去学键位的机会（IDEA / VS Code 的禁用菜单项
                // 也照常显示快捷键）。降档交给库：见 `lithe_ui::typography::KbdCombo` 的
                // `.disabled()`，键帽文字与「+」会跟到 `fg_disabled`，与行文字同档。
                //
                // 这里会置灰、且真的带键位的共 **12** 行：GoToLine、Find、FindAndReplace、
                // ToggleComment、DuplicateLine、DeleteLine、MoveLineUp、MoveLineDown、
                // NextTab、PreviousTab、GoBack、GoForward —— 数漏了就会低估这条的收益。
                if let Some(keys) = action.menu_key() {
                    row = row.keys(keys);
                }
                // 不用判 disabled：库已经在两处拦过（绘制时不挂 mouse_down、键盘
                // `choose()` 直接 return），宿主再判一次就成了第二个真源。
                menu.item(row.on_click(move |_window, cx| {
                    let _ = row_handle.update(cx, |bar, cx| bar.push_run(action, cx));
                }))
            }
            MenuItem::NotWired {
                label_key,
                missing,
                icon,
            } => {
                let label_key = *label_key;
                let missing = *missing;
                let row_handle = handle.clone();
                let (label, badge) = (tr(label_key), tr(NOT_WIRED_BADGE_KEY));
                let lead: Option<Rc<dyn Fn(&App) -> AnyElement>> = match *icon {
                    // 未接线行永远不被置灰（下面的 `not_wired_row` 写了理由），传 `false`。
                    MenuIcon::Idea(icon) => Some(Rc::new(icon_element(icon, false))),
                    MenuIcon::Empty => None,
                };
                menu.item(
                    MenuRow::element(move |cx| {
                        not_wired_row(label.clone(), badge.clone(), lead.as_ref(), cx)
                    })
                    .on_click(move |_window, cx| {
                        let _ = row_handle
                            .update(cx, |bar, cx| bar.push_not_wired(label_key, missing, cx));
                    }),
                )
            }
            MenuItem::Theme => match &themes {
                Some(submenu) => {
                    menu.item(MenuRow::submenu(tr("lithe.menu.theme"), submenu.clone()))
                }
                None => menu,
            },
        };
    }
    menu
}

/// 菜单行里的图标槽内容：本仓那张图，深浅等画的时候再问主题。
type Slot = Rc<dyn Fn(&App) -> AnyElement>;

/// 未接线那一行的行内容：图标槽 + 标题 + 行尾的「未接入」角标。
///
/// **降权用 `fg_subtle` 而不是 `fg_disabled`，并且保持可点**：这类行是「还没做」不是
/// 「现在不能用」，置灰会把两件事画成一个样；再就是 `Menu::step()` 跳过 `disabled` 的行，
/// 置灰等于把它们的键盘可达性一起丢掉。点击照旧走到 `push_not_wired`（那句才说清缺什么）。
fn not_wired_row(
    label: SharedString,
    badge: SharedString,
    lead: Option<&Slot>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    h_stack()
        .flex_1()
        .min_w_0()
        .gap_2()
        .text_color(colors.fg_subtle)
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(theme.icon_size(IconSize::Sm))
                .children(lead.map(|paint| paint(cx))),
        )
        .child(div().flex_1().min_w_0().truncate().child(label))
        .child(div().flex_none().text_xs().child(badge))
        .into_any_element()
}

fn theme_menu(names: Vec<String>) -> Menu {
    let mut menu = Menu::new().width(rems(PANEL_MIN_WIDTH_SPEC / 16.));
    for name in names {
        let label: SharedString = name.clone().into();
        let value: SharedString = name.into();
        menu = menu.item(
            MenuRow::new(label)
                // 主题子菜单没有禁用态：主题列表空的时候整行都不出现。
                .icon_element(icon_element(&idea::PALETTE_ICON, false))
                .on_click(move |_window, cx| apply_theme_choice(value.to_string(), cx)),
        );
    }
    menu
}

/// 菜单行里的图标槽：本仓那张图，深浅与禁用色等画的时候再问主题。
///
/// 禁用行的颜色得**宿主**自己给：库那条 [`MenuRow::icon`] 会在行禁用时把图标画成
/// `fg_disabled`，而自画的元素拿不到这个状态（`lithe_ui::menus` 里 `icon_element`
/// 的注释就是这么说的）。`Icon::color()` 对 IDEA 图标同样生效 —— gpui 把 SVG 当
/// alpha 蒙版画，文件里自带的颜色不参与。
fn icon_element(
    icon: &'static idea::IdeaIcon,
    disabled: bool,
) -> impl Fn(&App) -> AnyElement + 'static {
    move |cx: &App| {
        let theme = cx.theme();
        // 非禁用时显式给 `fg`：这正是 `Icon::render` 原本回落的值，视觉零变化，
        // 只是免得读代码的人还要去猜「没给色会落到哪儿」。
        let color = if disabled {
            theme.colors.fg_disabled
        } else {
            theme.colors.fg
        };
        idea_icon_mono(icon, theme.is_dark())
            .color(color)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CloseReason, EditorContext, KEY_FIND, KEY_FIND_REPLACE, KEY_GO_BACK, KEY_GO_FORWARD,
        KEY_GO_TO_LINE, KEY_GO_TO_REFERENCES, KEY_REOPEN_CLOSED_TAB, KEY_RESET_ZOOM,
        KEY_TOGGLE_ACTIVITY_SIDEBAR,
        KEY_TOGGLE_TERMINAL, KEY_ZOOM_IN, KEY_ZOOM_OUT, MENUS, MISSING_GROUPS, MenuAction,
        MenuBarMode, MenuItem, NOT_WIRED_OUTSIDE_MENUS, NavState, TriggerFace, action_count,
        mode_for, mode_from_id, not_wired_count, resolve_menu_toggle,
    };
    use lithe_gpui_shared::tr;
    use lithe_ui::theme::Palette;
    use std::collections::BTreeSet;

    const SOURCE_MENU_ITEMS: usize = 89;
    const LOCAL_ONLY_ITEMS: usize = 5;

    /// 回归：菜单栏曾经只有「闲置 / 悬停」两态，展开着的那个菜单没有底；补四态时又把
    /// 展开态和悬停态设成了同一个 `hover`，鼠标还压在触发器上就分不清「指着」和「开着」。
    ///
    /// 第二次回归（2026-10-11）：这两档用的是 `hover` / `active`，那是贴着**列表白底**调的
    /// 一档，IntelliJ Light 下与触发器所在的 `title_bar`（`#f2f2f2`）同值 —— 闭合态悬停看不出
    /// 反馈。画在 chrome 面上的高亮必须读 `chrome_hover` / `chrome_active`。
    #[test]
    fn trigger_faces_follow_the_spec_table() {
        for palette in [Palette::light(false), Palette::dark(false)] {
            let face = |state: TriggerFace| state.paint(&palette);

            assert_eq!(face(TriggerFace::Idle), (palette.fg, None, None));
            assert_eq!(face(TriggerFace::Hover).1, Some(palette.chrome_hover));
            assert_eq!(face(TriggerFace::Open).1, Some(palette.chrome_active));
            assert_eq!(face(TriggerFace::Active).1, Some(palette.chrome_active));
            // 内描边取 `fg` 自己：`focus` token 落在 `title_bar` 上只有 2.49:1，不够 WCAG。
            assert_eq!(face(TriggerFace::Active).2, Some(palette.fg));
            assert_eq!(
                face(TriggerFace::Disabled),
                (palette.fg_disabled, None, None)
            );

            // 悬停底必须**在它所在的那一块底色上看得见**：触发器画在 `title_bar` 上，取
            // `hover`（贴着列表白底调的那一档）会在 IntelliJ Light 下与它同值。
            assert_ne!(
                palette.chrome_hover, palette.title_bar,
                "chrome_hover 与 title_bar 同色，触发器悬停等于没画"
            );
            assert_ne!(
                face(TriggerFace::Hover).1,
                Some(palette.hover),
                "触发器画在 chrome 面上，不能读列表的 hover 档"
            );
            // 展开态与悬停态必须分得开，否则鼠标停在触发器上就等于没有高亮。
            assert_ne!(
                face(TriggerFace::Open).1,
                face(TriggerFace::Hover).1,
                "展开态与悬停态同色，看不出菜单开着"
            );
            assert_ne!(
                palette.chrome_hover, palette.chrome_active,
                "调色板本身就撞色了"
            );
            // 除禁用态之外文字都是 `fg`：浅色下 `fg_muted` 在 header 上只剩 4.56:1。
            for state in [
                TriggerFace::Idle,
                TriggerFace::Hover,
                TriggerFace::Open,
                TriggerFace::Active,
            ] {
                assert_eq!(face(state).0, palette.fg, "{state:?}");
            }
        }
    }

    #[test]
    fn trigger_face_picks_the_strongest_state() {
        assert_eq!(TriggerFace::of(false, false, false), TriggerFace::Idle);
        assert_eq!(TriggerFace::of(true, false, false), TriggerFace::Open);
        assert_eq!(TriggerFace::of(false, false, true), TriggerFace::Active);
        assert_eq!(
            TriggerFace::of(true, false, true),
            TriggerFace::Active,
            "聚焦压过开合"
        );
        assert_eq!(
            TriggerFace::of(true, true, true),
            TriggerFace::Disabled,
            "禁用压过一切"
        );
    }

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
                MenuAction::NewTab,
                MenuAction::NewWindow,
                MenuAction::NewFile,
                MenuAction::OpenFolder,
                MenuAction::OpenFile,
                MenuAction::CloseFolder,
                MenuAction::Save,
                MenuAction::SaveAs,
                MenuAction::SaveAll,
                MenuAction::RevertFile,
                MenuAction::CloseTab,
                MenuAction::CloseWindow,
                MenuAction::CloseOtherTabs,
                MenuAction::CloseAllTabs,
                MenuAction::CloseSavedTabs,
                MenuAction::CloseTabsToLeft,
                MenuAction::CloseTabsToRight,
                MenuAction::ReopenClosedTab,
                MenuAction::Quit,
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
            // 查找 / 替换：宿主在 `install_key_actions` 里为它们绑了 `secondary-f` / `secondary-r`，
            // 并派发 `lithe_gpui_editor::OpenFind` / `OpenFindReplace`（`keybind_action()` 有值），
            // 所以菜单画得出键位胶囊、按下去也真生效。「快速修复」不同 —— 它由外壳直接调
            // `EditorPane::show_code_actions`，不走 `dispatch_action`，没有 Action，故仍不画胶囊。
            (MenuAction::Find, "lithe.menu.find", true),
            (
                MenuAction::FindAndReplace,
                "lithe.menu.findAndReplace",
                true,
            ),
            (MenuAction::QuickFix, "lithe.menu.quickFix", false),
            // 转到实现 / 转到类型定义：有真被登记的 Action（菜单派发走得通），
            // 但出厂**不绑键**，所以 `menu_key()` 是 `None`、菜单上不画胶囊。
            (
                MenuAction::GoToImplementation,
                "lithe.menu.goToImplementation",
                true,
            ),
            (
                MenuAction::GoToTypeDefinition,
                "lithe.menu.goToTypeDefinition",
                true,
            ),
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
            // 行编辑五个：动作在 `lithe_ui::editor` 的 `ElyEditor` 上下文里真绑着键
            // （IDEA 默认键，三端同键），所以菜单画胶囊、派发也走得通。
            (MenuAction::ToggleComment, "lithe.menu.toggleComment", true),
            (MenuAction::DuplicateLine, "lithe.menu.duplicateLine", true),
            (MenuAction::DeleteLine, "lithe.menu.deleteLine", true),
            (MenuAction::MoveLineUp, "lithe.menu.moveLineUp", true),
            (MenuAction::MoveLineDown, "lithe.menu.moveLineDown", true),
            // 后退/前进：工作台自己的动作，`install_key_actions` 绑了键。
            (MenuAction::GoBack, "lithe.menu.goBack", true),
            (MenuAction::GoForward, "lithe.menu.goForward", true),
            // 显示开关三个：翻的是 settings store 的档位，没有可派发动作，不画胶囊。
            (MenuAction::ToggleMinimap, "lithe.menu.toggleMinimap", false),
            (
                MenuAction::ToggleLineNumbers,
                "lithe.menu.toggleLineNumbers",
                false,
            ),
            (
                MenuAction::ToggleRenderWhitespace,
                "lithe.menu.toggleRenderWhitespace",
                false,
            ),
            // 快捷键一览：两处菜单（Tools / Help）共用，外壳直接开设置对话框的 Keyboard 页。
            (
                MenuAction::ShowKeymapSettings,
                "lithe.menu.keyboardShortcuts",
                false,
            ),
            // 标签页上/下一个：循环切换，`ctrl-tab` / `ctrl-shift-tab` 在 `install_key_actions` 里绑着。
            (MenuAction::NextTab, "lithe.menu.nextTab", true),
            (MenuAction::PreviousTab, "lithe.menu.previousTab", true),
            // 文件生命周期五个：执行分支在 `apply_menu_action`（无活动缓冲时 no-op + 诊断），
            // 没有可派发的 Action，也不绑键，所以菜单上不画键位胶囊。
            (MenuAction::NewTab, "lithe.menu.newTab", false),
            (MenuAction::NewFile, "lithe.menu.newFile", false),
            (MenuAction::SaveAs, "lithe.menu.saveAs", false),
            (MenuAction::SaveAll, "lithe.menu.saveAll", false),
            (MenuAction::RevertFile, "lithe.menu.revertFile", false),
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
            gpui::Keystroke::parse(key)
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
                KEY_FIND,
                KEY_FIND_REPLACE,
                "secondary-/",
                "secondary-shift-d",
                "secondary-y",
                "secondary-alt-shift-up",
                "secondary-alt-shift-down",
                KEY_TOGGLE_ACTIVITY_SIDEBAR,
                KEY_TOGGLE_TERMINAL,
                KEY_ZOOM_IN,
                KEY_ZOOM_OUT,
                KEY_RESET_ZOOM,
                KEY_GO_TO_LINE,
                KEY_GO_BACK,
                KEY_GO_FORWARD,
                KEY_GO_TO_REFERENCES,
                "ctrl-tab",
                "ctrl-shift-tab",
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
        assert_eq!(KEY_GO_TO_LINE, "secondary-g");
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
        // 同一个能力默认只许一个菜单入口。双入口要在白名单里说清理由 ——
        // 「快捷键一览」按 IDEA 的布局在 Tools 和 Help 各有一条，这是上游布局不是重复。
        const TWO_ENTRIES: [MenuAction; 1] = [MenuAction::ShowKeymapSettings];
        const NON_COMMAND: [MenuAction; 52] = [
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
            MenuAction::ToggleComment,
            MenuAction::DuplicateLine,
            MenuAction::DeleteLine,
            MenuAction::MoveLineUp,
            MenuAction::MoveLineDown,
            MenuAction::ToggleMinimap,
            MenuAction::ToggleLineNumbers,
            MenuAction::ToggleRenderWhitespace,
            MenuAction::GoBack,
            MenuAction::GoForward,
            MenuAction::ShowKeymapSettings,
            MenuAction::NextTab,
            MenuAction::PreviousTab,
            // 文件生命周期五个：执行分支在 `apply_menu_action`，都没有 `CommandId`。
            MenuAction::NewTab,
            MenuAction::NewFile,
            MenuAction::SaveAs,
            MenuAction::SaveAll,
            MenuAction::RevertFile,
            // 窗口 / 工程生命周期三条同样由 `apply_menu_action` 执行。
            MenuAction::CloseFolder,
            MenuAction::CloseWindow,
            MenuAction::Quit,
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
                            !wired.contains(action) || TWO_ENTRIES.contains(action),
                            "{action:?} 在菜单里出现了两次（同一个能力不该两个入口，白名单除外）"
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
        // 这个下限只防「有人把没能力的项顺手删掉」。2026-10-16 接通转到引用 / 重命名 /
        // 格式化文档三条后从 21 降到 18；C1 接通关闭文件夹 / 关闭窗口 / 退出三条后
        // 18 → 15；再降就得先解释清楚又接了什么。
        assert!(
            declared.len() >= 15,
            "占位项突然少了一截（现在 {}）？查一下是不是有人把没能力的项又删掉了",
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
            "secondary-shift-r",
            "`secondary-` 才是 gpui 的平台归一化口径（macOS ⌘⇧R / 其他 Ctrl+Shift+R）"
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
            texts.len() <= 10,
            "占位文案有 {} 句，超过了'按能力分组'的上限 10（= MISSING_GROUPS 的组数）",
            texts.len()
        );
        assert_eq!(
            MISSING_GROUPS.len(),
            10,
            "能力组数变了就必须同步改上面的上界与 `MISSING_GROUPS` 的文档"
        );
    }

    #[test]
    fn keyboard_shortcuts_appears_in_tools_and_help() {
        // 「键盘快捷键」入口已经接线（Tools / Help 各画一条 `Action(ShowKeymapSettings)`），
        // 不再是 `NotWired` 占位项。这条测试盯的是**入口出现在哪两个菜单里**，与接线与否无关。
        assert_eq!(
            MenuAction::ShowKeymapSettings.label_key(),
            "lithe.menu.keyboardShortcuts",
            "快捷键入口的文案键变了"
        );
        let places = MENUS
            .iter()
            .flat_map(|menu| {
                menu.items.iter().filter_map(|item| match item {
                    MenuItem::Action(MenuAction::ShowKeymapSettings) => Some(menu.id),
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
    /// 旧的 `compact_menu: bool` 只能表达两档，`false` 必须还落在默认档上。
    fn mode_defaults_to_pinned() {
        assert_eq!(mode_for(false), MenuBarMode::DEFAULT);
        assert_eq!(mode_for(true), MenuBarMode::Compact);
    }

    #[test]
    fn the_four_modes_round_trip_through_their_ids() {
        for mode in MenuBarMode::ALL {
            assert_eq!(MenuBarMode::from_id(mode.id()), Some(mode));
        }
        // 认不出的（含改造前存的旧值 `pinned`）一律回落默认档，不能让整份设置读失败。
        assert_eq!(MenuBarMode::from_id("pinned"), None);
        assert_eq!(mode_from_id(Some("pinned")), MenuBarMode::DEFAULT);
        assert_eq!(mode_from_id(None), MenuBarMode::DEFAULT);
        assert_eq!(mode_from_id(Some("autohide")), MenuBarMode::AutoHide);
        assert_eq!(mode_from_id(Some("above")), MenuBarMode::Above);
    }

    #[test]
    fn the_mode_cycle_visits_every_mode_before_repeating() {
        let mut mode = MenuBarMode::DEFAULT;
        let mut seen = Vec::new();
        for _ in 0..MenuBarMode::ALL.len() {
            seen.push(mode);
            mode = mode.next();
        }
        assert_eq!(mode, MenuBarMode::DEFAULT, "转一圈要回到原点");
        assert_eq!(seen.len(), 4);
        for each in MenuBarMode::ALL {
            assert!(seen.contains(&each), "{each:?} 没出现在循环里");
        }
    }

    #[test]
    fn only_autohide_reserves_no_space() {
        // 外壳靠这个决定标题栏上面留不留空：只有 AutoHide 是平时不占地方的那一档。
        assert!(MenuBarMode::AutoHide.reserves_no_space());
        for mode in MenuBarMode::ALL {
            if mode != MenuBarMode::AutoHide {
                assert!(!mode.reserves_no_space(), "{mode:?} 是要占地方的");
            }
        }
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

    // —— 菜单启用判定（S5）——
    //
    // 这里的用例全是**纯函数**测试：不建 `App`、不起定时器、不碰 IO，只把
    // `EditorContext` 构造出来喂给 `MenuAction::is_enabled`。贪慢等待与共享状态的坑天然
    // 不存在，所以不必给它们加任何超时或清理。

    /// 一份「符合不变量」的上下文坯子：`EditorContext` 的三条不变量由这里一次性兜住，
    /// 用例只用 struct 更新语法挑自己要改的那一两个字段。
    fn context(tab_count: usize, active_tab_index: Option<usize>) -> EditorContext {
        let context = EditorContext {
            has_active_buffer: active_tab_index.is_some(),
            tab_count,
            active_tab_index,
            has_active_disk_file: false,
            active_writable: false,
            can_go_back: false,
            can_go_forward: false,
            has_closed_tabs: false,
        };
        assert!(
            context.has_active_buffer == (context.tab_count > 0)
                && context.has_active_buffer == context.active_tab_index.is_some(),
            "上下文坯子自己就违反了第 1 条不变量：{context:?}"
        );
        assert!(
            !context.has_active_disk_file || context.has_active_buffer,
            "上下文坯子违反了第 2 条不变量：{context:?}"
        );
        context
    }

    /// 一个都没开 —— 本次改造真正的触发场景。
    fn no_tabs() -> EditorContext {
        context(0, None)
    }

    /// 开着一个健康的磁盘文件。
    fn one_disk_tab() -> EditorContext {
        EditorContext {
            has_active_disk_file: true,
            active_writable: true,
            ..context(1, Some(0))
        }
    }

    fn is_enabled(action: MenuAction, ctx: EditorContext) -> bool {
        action.is_enabled(ctx)
    }

    #[test]
    fn no_open_tab_greys_out_every_buffer_bound_row() {
        let ctx = no_tabs();

        // 认的是「当前这个缓冲」的那一批：文件生命周期、编辑、查找、跳转。
        for action in [
            MenuAction::Save,
            MenuAction::SaveAs,
            MenuAction::SaveAll,
            MenuAction::RevertFile,
            MenuAction::CloseTab,
            MenuAction::CloseOtherTabs,
            MenuAction::CloseAllTabs,
            MenuAction::CloseSavedTabs,
            MenuAction::CloseTabsToLeft,
            MenuAction::CloseTabsToRight,
            MenuAction::Undo,
            MenuAction::Redo,
            MenuAction::Cut,
            MenuAction::Copy,
            MenuAction::Paste,
            MenuAction::SelectAll,
            MenuAction::Find,
            MenuAction::FindAndReplace,
            MenuAction::ToggleComment,
            MenuAction::DuplicateLine,
            MenuAction::DeleteLine,
            MenuAction::MoveLineUp,
            MenuAction::MoveLineDown,
            MenuAction::GoToLine,
            MenuAction::QuickFix,
            MenuAction::ShowHover,
            MenuAction::GoToDefinition,
            MenuAction::GoToImplementation,
            MenuAction::GoToTypeDefinition,
            MenuAction::ToggleBreakpoint,
            MenuAction::NextTab,
            MenuAction::PreviousTab,
            MenuAction::GoBack,
            MenuAction::GoForward,
        ] {
            assert!(
                !is_enabled(action, ctx),
                "{action:?} 一个标签页都没开时不该可点"
            );
        }

        // 与缓冲无关的必须照旧可点：File 的五个入口、Edit 的 CommandPalette、
        // View / Window / Help 全组 —— 这也是「九个顶层菜单里没有一个会全灰」的兜底。
        for action in [
            MenuAction::NewTab,
            MenuAction::NewWindow,
            MenuAction::NewFile,
            MenuAction::OpenFolder,
            MenuAction::OpenFile,
            MenuAction::CommandPalette,
            MenuAction::QuickOpen,
            MenuAction::SearchEverywhere,
            MenuAction::ReplaceInFiles,
            MenuAction::StartDebugging,
            MenuAction::StopDebugging,
            MenuAction::ToggleWordWrap,
            MenuAction::ZoomIn,
            MenuAction::ZoomOut,
            MenuAction::ResetZoom,
            MenuAction::ToggleMenuBar,
            MenuAction::ToggleMinimap,
            MenuAction::ToggleLineNumbers,
            MenuAction::ToggleRenderWhitespace,
            MenuAction::Preferences,
            MenuAction::ShowKeymapSettings,
            MenuAction::ToggleActivitySidebar,
            MenuAction::ToggleSecondarySidebar,
            MenuAction::ToggleStatusBar,
            MenuAction::ToggleTerminal,
            MenuAction::NewTerminalTab,
            MenuAction::CloseTerminalTab,
            MenuAction::ShowDiagnostics,
            MenuAction::ShowFileExplorer,
            MenuAction::ShowSourceControl,
            MenuAction::ShowRunAndDebug,
        ] {
            assert!(
                is_enabled(action, ctx),
                "{action:?} 不需要活动缓冲，不该跟着一起灰"
            );
        }
    }

    /// 本批主场景里唯一一条「带 ⌘⇧T、亮着、点了必然无反应」的行。
    ///
    /// `reopen_closed_tab` 的第一句是 `self.closed.pop()`，为 `None` 时直接返回。刚开一个
    /// 项目、还没关过任何文件时 `closed` 就是空的 —— 不判它，这一行会精确复刻本单
    /// 要消灭的那种死行。
    #[test]
    fn reopen_closed_tab_follows_the_closed_queue_not_the_open_tabs() {
        // 零标签页（= 主场景）：有没有关过文件说了算，与当前开着几个无关。
        assert!(!is_enabled(MenuAction::ReopenClosedTab, no_tabs()));
        assert!(is_enabled(
            MenuAction::ReopenClosedTab,
            EditorContext {
                has_closed_tabs: true,
                ..no_tabs()
            }
        ));

        // 开着三个标签页也一样：它是「关掉过的」清单，不是「打开着的」清单。
        let three = context(3, Some(1));
        assert!(!is_enabled(MenuAction::ReopenClosedTab, three));
    }

    /// 有跳转历史、但一个标签页都没开 —— `can_go_back` 会撒谎，`has_active_buffer` 得兜住。
    ///
    /// 复现路径：打开 A.java → ⌘点击跳到 B.java → 关掉所有标签页（`close()` 不清跳转历史）
    /// → 此时 `can_go_back == true`，而 `go_back` 第一句的 `current_entry` 会因
    /// `self.active?` 直接返回 `None`，点了毫无反应。
    #[test]
    fn jump_history_alone_does_not_light_up_navigation() {
        let history_but_no_tab = EditorContext {
            can_go_back: true,
            can_go_forward: true,
            ..no_tabs()
        };

        assert!(
            !is_enabled(MenuAction::GoBack, history_but_no_tab),
            "没有活动缓冲时 Back 必须置灰：执行侧会在 `current_entry` 直接返回"
        );
        assert!(
            !is_enabled(MenuAction::GoForward, history_but_no_tab),
            "没有活动缓冲时 Forward 必须置灰，理由同上"
        );

        // 有历史、有缓冲 —— 两条都该亮。
        let navigated = EditorContext {
            can_go_back: true,
            can_go_forward: true,
            ..one_disk_tab()
        };
        assert!(is_enabled(MenuAction::GoBack, navigated));
        assert!(is_enabled(MenuAction::GoForward, navigated));
    }

    /// 有损 UTF-8 转码的缓冲 `writable == false`，但「还原」恰恰是它唯一需要的出口。
    #[test]
    fn a_lossy_buffer_can_still_be_reverted_though_not_saved() {
        let lossy = EditorContext {
            has_active_disk_file: true,
            // 转码有损 ⇒ 写不回磁盘，但可以原样读回来。
            active_writable: false,
            ..context(1, Some(0))
        };

        assert!(
            is_enabled(MenuAction::RevertFile, lossy),
            "revert 是从磁盘读回，不看可写性；有损转码文件改坏了只能靠它恢复"
        );
        assert!(
            !is_enabled(MenuAction::Save, lossy),
            "把有损转码的内容写回去会损坏原文件，这一行必须灰"
        );
    }

    #[test]
    fn a_dirty_untitled_scratch_can_be_saved_but_not_reverted() {
        // 只点「New Tab」：`save_active` 会转成另存为，所以这一行**可用**；
        // 但还原与断点都需要一条磁盘路径，没有就是`false`。
        let ctx = EditorContext {
            active_writable: true,
            ..context(1, Some(0))
        };

        assert!(is_enabled(MenuAction::Save, ctx));
        assert!(is_enabled(MenuAction::SaveAs, ctx));
        assert!(!is_enabled(MenuAction::RevertFile, ctx));
        assert!(!is_enabled(MenuAction::ToggleBreakpoint, ctx));
    }

    #[test]
    fn one_tab_disables_tab_cycling_but_not_closing() {
        let ctx = one_disk_tab();

        assert!(is_enabled(MenuAction::CloseTab, ctx));
        assert!(is_enabled(MenuAction::CloseAllTabs, ctx));
        assert!(is_enabled(MenuAction::CloseSavedTabs, ctx));
        assert!(is_enabled(MenuAction::SaveAll, ctx));

        // 只有一个标签页时「关掉其它 / 上一个 / 下一个」没有意义。
        assert!(!is_enabled(MenuAction::CloseOtherTabs, ctx));
        assert!(!is_enabled(MenuAction::NextTab, ctx));
        assert!(!is_enabled(MenuAction::PreviousTab, ctx));
        // 左右两边都排不到邻居。
        assert!(!is_enabled(MenuAction::CloseTabsToLeft, ctx));
        assert!(!is_enabled(MenuAction::CloseTabsToRight, ctx));
    }

    #[test]
    fn the_active_tabs_position_decides_both_close_to_the_side_rows() {
        // 三个标签页，逐个走一遍活动下标 —— 这是最有代表性的自变量切片。
        for (index, left_ok, right_ok) in [(0, false, true), (1, true, true), (2, true, false)] {
            let ctx = EditorContext {
                has_active_disk_file: true,
                active_writable: true,
                ..context(3, Some(index))
            };
            assert_eq!(
                is_enabled(MenuAction::CloseTabsToLeft, ctx),
                left_ok,
                "活动在 ix={index} 时「关闭左侧标签页」"
            );
            assert_eq!(
                is_enabled(MenuAction::CloseTabsToRight, ctx),
                right_ok,
                "活动在 ix={index} 时「关闭右侧标签页」"
            );
            // 只要有两个以上的标签页，这三个就都是活的。
            assert!(is_enabled(MenuAction::CloseOtherTabs, ctx));
            assert!(is_enabled(MenuAction::NextTab, ctx));
            assert!(is_enabled(MenuAction::PreviousTab, ctx));
        }
    }

    #[test]
    fn a_read_only_buffer_disables_saving_but_not_reverting() {
        // 二进制 / 有损转码的缓冲：`writable` 是假。
        let ctx = EditorContext {
            has_active_disk_file: true,
            ..context(1, Some(0))
        };

        // 写回磁盘的那一个，灰。
        assert!(!is_enabled(MenuAction::Save, ctx));
        // 另存为写到别处去，不受当前缓冲能不能写回影响。
        assert!(is_enabled(MenuAction::SaveAs, ctx));
        // 断点不写文件，读了字节就拿得下来。
        assert!(is_enabled(MenuAction::ToggleBreakpoint, ctx));

        // **还原不在此列**：它是从磁盘读回，执行侧 `revert_active` 只判 `active` 与
        // `is_disk()`。有损 UTF-8 转码的文件 `writable == false`，可它的正文恰恰是文件
        // 内容的近似转写 —— 用户改坏了最需要的就是这一行，灰掉等于堵死唯一的出口。
        assert!(
            is_enabled(MenuAction::RevertFile, ctx),
            "revert 读回磁盘内容，不看可写性；有损转码文件改坏了只能靠它恢复"
        );
    }

    #[test]
    fn the_jump_history_decides_back_and_forward() {
        let present = context(2, Some(1));
        let navigated = EditorContext {
            can_go_back: true,
            can_go_forward: true,
            ..present
        };

        assert!(!is_enabled(MenuAction::GoBack, present));
        assert!(!is_enabled(MenuAction::GoForward, present));
        assert!(is_enabled(MenuAction::GoBack, navigated));
        assert!(is_enabled(MenuAction::GoForward, navigated));

        // 只走到一半：能后退但还没得前进 —— 这是 `JumpHistory` 自己的语义。
        let halfway = EditorContext {
            can_go_back: true,
            ..present
        };
        assert!(is_enabled(MenuAction::GoBack, halfway));
        assert!(!is_enabled(MenuAction::GoForward, halfway));
    }

    /// 判定表的**逐变体**交叉验证。
    ///
    /// 下面这个 `match` 必须穷尽：给 `MenuAction` 加一个新成员，这里会先编译不过，
    /// 逼着实现同学给新行挑一个出处。它是一份刻意维护的镜像——用例 1~6 用的是写死的
    /// 期望值（这才是真正的验收口径），这里负责补上「一个变体都不漏」那件事。
    fn assert_row_follows(action: MenuAction, ctx: EditorContext) {
        let expected = match action {
            // —— 认的是「当前这个缓冲」的那一批 ——
            MenuAction::SaveAs
            | MenuAction::CloseTab
            | MenuAction::Undo
            | MenuAction::Redo
            | MenuAction::Cut
            | MenuAction::Copy
            | MenuAction::Paste
            | MenuAction::SelectAll
            | MenuAction::Find
            | MenuAction::FindAndReplace
            | MenuAction::ToggleComment
            | MenuAction::DuplicateLine
            | MenuAction::DeleteLine
            | MenuAction::MoveLineUp
            | MenuAction::MoveLineDown
            | MenuAction::GoToLine
            | MenuAction::QuickFix
            | MenuAction::ShowHover
            | MenuAction::GoToDefinition
            | MenuAction::GoToImplementation
            | MenuAction::GoToTypeDefinition
            | MenuAction::GoToReferences
            | MenuAction::RenameSymbol
            | MenuAction::FormatDocument => ctx.has_active_buffer,

            // —— 要把字节写回磁盘的那个 ——
            //
            //    `RevertFile` **不在此列**：它是从磁盘**读回**，执行侧 `revert_active`
            //    只判 `active` 与 `is_disk()`。有损 UTF-8 转码的缓冲 `writable == false`，
            //    可还原恰恰是它最需要的出口。
            MenuAction::Save => ctx.has_active_buffer && ctx.active_writable,

            // —— 必须有一条磁盘路径的两个 ——
            MenuAction::RevertFile | MenuAction::ToggleBreakpoint => ctx.has_active_disk_file,

            // —— 按标签页数 ——
            MenuAction::SaveAll | MenuAction::CloseAllTabs | MenuAction::CloseSavedTabs => {
                ctx.tab_count > 0
            }
            MenuAction::CloseOtherTabs | MenuAction::NextTab | MenuAction::PreviousTab => {
                ctx.tab_count > 1
            }

            // —— 按活动标签页的位置 ——
            MenuAction::CloseTabsToLeft => match ctx.active_tab_index {
                Some(index) => index > 0,
                None => false,
            },
            MenuAction::CloseTabsToRight => match ctx.active_tab_index {
                Some(index) => index + 1 < ctx.tab_count,
                None => false,
            },

            // —— 跳转历史：必须叠加活动缓冲，否则「有历史、没标签页」会画出死行
            //    （`go_back` / `go_forward` 的第一句都是 `current_entry`，它第一行是
            //    `self.active?`；`close()` 又不清历史）——
            MenuAction::GoBack => ctx.has_active_buffer && ctx.can_go_back,
            MenuAction::GoForward => ctx.has_active_buffer && ctx.can_go_forward,

            // —— 关掉过的标签页：`reopen_closed_tab` 在 `closed.pop()` 为空时直接返回 ——
            MenuAction::ReopenClosedTab => ctx.has_closed_tabs,

            // —— 其余三十四个：新建/打开、界面开关、全局搜索、窗口、帮助 ——
            MenuAction::NewTab
            | MenuAction::NewWindow
            | MenuAction::NewFile
            | MenuAction::OpenFolder
            | MenuAction::OpenFile
            | MenuAction::CommandPalette
            | MenuAction::QuickOpen
            | MenuAction::SearchEverywhere
            | MenuAction::ReplaceInFiles
            | MenuAction::ToggleActivitySidebar
            | MenuAction::ToggleSecondarySidebar
            | MenuAction::ToggleStatusBar
            | MenuAction::ToggleTerminal
            | MenuAction::ToggleMaven
            | MenuAction::ToggleMenuBar
            | MenuAction::ToggleWordWrap
            | MenuAction::ToggleMinimap
            | MenuAction::ToggleLineNumbers
            | MenuAction::ToggleRenderWhitespace
            | MenuAction::OpenAppearanceSettings
            | MenuAction::ShowDiagnostics
            | MenuAction::ShowFileExplorer
            | MenuAction::ShowSourceControl
            | MenuAction::ShowRunAndDebug
            | MenuAction::StartDebugging
            | MenuAction::StopDebugging
            | MenuAction::ZoomIn
            | MenuAction::ZoomOut
            | MenuAction::ResetZoom
            | MenuAction::ShowKeymapSettings
            | MenuAction::Preferences
            | MenuAction::Minimize
            | MenuAction::Maximize
            | MenuAction::ToggleFullscreen
            | MenuAction::NewTerminalTab
            | MenuAction::CloseTerminalTab
            // 窗口 / 工程生命周期三条不依赖活动缓冲：关文件夹、关窗口、退出随时可点。
            | MenuAction::CloseFolder
            | MenuAction::CloseWindow
            | MenuAction::Quit => true,
        };

        assert_eq!(
            is_enabled(action, ctx),
            expected,
            "{action:?} 在 {ctx:?} 下的启用判定与逐变体表不符"
        );
    }

    #[test]
    fn the_decision_table_covers_every_menu_action_in_the_bar() {
        let cases = [
            no_tabs(),
            one_disk_tab(),
            EditorContext {
                active_writable: true,
                ..context(1, Some(0))
            },
            context(2, Some(0)),
            context(2, Some(1)),
            EditorContext {
                can_go_back: true,
                ..context(3, Some(1))
            },
            EditorContext {
                can_go_forward: true,
                ..context(3, Some(2))
            },
        ];

        // 只跑 `MENUS` 里真出现的那些：菜单看不见的行不参与置灰，多判没有意义。
        for item in MENUS.iter().flat_map(|menu| menu.items.iter()) {
            let MenuItem::Action(action) = item else {
                continue;
            };
            for ctx in cases {
                assert_row_follows(*action, ctx);
            }
        }
    }
}
