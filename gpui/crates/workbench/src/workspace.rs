//! 工作台骨架：把外壳各区域组装成 Windows 的三层结构。
//!
//! 结构（逐值照 `windows/tauri/src/features/layout/components/main-layout.tsx`，度量见
//! `gpui/UI-MAP-WINDOWS.md` §1.1 与 `gpui/research/windows/01-shell.md` §2）：
//!
//! ```text
//! v_flex
//! ├─ 标题栏 40            title-bar.tsx:337          40 = --lithe-title-bar-height（theme.css:118）
//! ├─ 项目标签条 32        project-tab-bar.tsx:41     h-8
//! ├─ 工作区 h_flex flex-1  main-layout.tsx:299       行尾 pr = 4（--lithe-workbench-gap）
//! │  ├─ 左活动栏 38+4      main-sidebar.tsx:97,588   折叠态图标竖条
//! │  ├─ 左侧栏 320        default-settings.ts:138   settings.sidebarWidth
//! │  ├─ 中央列 flex-1      main-layout.tsx:312       v_flex[ 编辑器岛, 底部工具窗 ]
//! │  ├─ 右侧工具窗 400    default-settings.ts:139   settings.rightToolWindowWidth（**可收起**）
//! │  └─ 右活动栏 38+4     plugin-activity-rail.tsx:34
//! └─ 状态栏 24            footer.tsx:45-49          24 = --lithe-footer-height（theme.css:119）
//! ```
//!
//! **底部窗不横跨工作台**（维护者 2026-09-25 拍板）：Windows 默认
//! `terminalWidthMode === "editor"`（`features/terminal/store.ts:29`），`BottomPane` 就是
//! 中央列里编辑器岛的下一个 flex 兄弟（`main-layout.tsx:318-322`），默认高 **320**
//! （`bottom-pane/bottom-pane.tsx:47`），上面还有一条 4px 的拖拽热区（同文件 `:222-232`）。
//! 本版用普通 `v_flex` + 固定高度表达这个分栏，**不用** gpui-kit 的 Dock：Windows 的
//! `MainLayout` 本身就是 flex + `ResizablePane`，没有 dock 系统，用 Dock 反而会多出
//! 一条 Windows 没有的标签头。
//!
//! 三个区域模块（[`super::shell`]）是**无状态渲染函数**；需要持有 `Entity` 的区域
//! （项目树 / 编辑区）各自是一个 `Entity`。状态归属只有两个：本结构体（外壳）+ 各区域自己。
//!
//! ## 活动栏选中态：**左栏两组独立可同时高亮；右栏一组，面板收起时全灭**
//!
//! 真机把「顶部组选中哪个视图」与「底部工具窗显示什么」当成**两个独立状态**
//! （`features/window/stores/workspace-ui-defaults.ts:3-9`：`activeSidebarView` 与
//! `isBottomPaneVisible` + `bottomPaneActiveTab`），高亮分别在
//! `features/layout/components/sidebar/main-sidebar.tsx:635-652` 算：
//! 顶部组看 `activeSidebarView`，底部组看 `isBottomPaneVisible && bottomPaneActiveTab === 该项`。
//! 所以本结构体也**不**共用一条 `Option<usize>`（那会让点终端时「项目」的高亮被顶掉），
//! 而是 [`ShellWorkspace::top_activity_view`] + [`ShellWorkspace::bottom_visible`] /
//! [`ShellWorkspace::bottom_kind`] 三个字段，再由
//! [`ShellWorkspace::is_activity_active`] 合成逐项谓词交给 [`crate::activity_bar`]。
//!
//! **右活动栏**是第三条、也是独立的一条：高亮 = `isRightSidebarVisible &&
//! activeRightSidebarView === 该项`（`plugin-activity-rail.tsx:24-26`、
//! `notifications-trigger.tsx:18-21`），所以右工具窗**收起时三项都不亮**，与左栏的
//! 「顶部组照常亮」互不影响（两边本来就是两套状态）。判据见
//! [`ShellWorkspace::is_right_activity_active`]。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::{ActiveTheme as _, Root};
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Div, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Pixels, Render, SharedString, Styled as _, Window, div, px,
    rems,
};

use lithe_gpui_editor::{EditorPane, SaveBuffer, TabMenuHostActions};
use lithe_gpui_explorer::Explorer;
use lithe_gpui_git::{BottomPane, ChangesView};
use lithe_gpui_settings::{Category as SettingsCategory, GitIdentityHost, IdentityField, IdentityScope};
use lithe_gpui_shared::icons::idea;
use lithe_gpui_shared::tr;
use lithe_gpui_terminal::{TerminalPane, TerminalPaneEvent};
use crate::activity_bar::{ActivityItem, ActivitySide, activity_bar};
use crate::branch_panel::{BranchPanel, open_branch_panel, trigger as branch_trigger};
use crate::command_palette::{
    Category, CommandAction, CommandId, install_actions as install_command_palette, set_shell,
    set_shell_focus,
};
use crate::menu_bar::{
    MenuAction, MenuBar, diagnose_run as diagnose_menu_run, mode_for, set_menu_bar,
    set_shell as set_menu_bar_shell,
};
use crate::project_menu::{
    ProjectEntry, ProjectMenu, render as render_project_menu, set_project_menu,
};
use crate::project_tabs::{ProjectTab, project_tabs};
use crate::right_tool_window::{
    RightToolWindowView, diagnose as diagnose_right_panel, resolve_click as resolve_right_click,
    right_tool_window,
};
use crate::status_bar::{StatusEntry, status_bar};
use crate::title_bar::title_bar;

/// 设置侧给宿主的那条"结果投递"回调的类型别名。
///
/// 写别名只为一件事：让两个 `Box<dyn Fn(..)>` 里的**闭包参数写得出显式类型**。
/// 不写类型标注时，`async_cx.spawn(async move |async_cx| …)` 的 `R` 推断不出来
/// （E0282：`AsyncFnOnce` 的返回类型是关联类型，推断会卡在未标注的闭包参数上）。
type GitIdentityDeliver =
    Box<dyn FnOnce(Option<String>, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static>;

/// 设置侧的 Git 身份枚举 → `lithe-gpui-git` 的同一枚举。
///
/// **为什么需要这一层**：两个 crate 各自拥有一份（设置侧不许依赖 git crate，
/// 见 `lithe-gpui-settings::identity` 的模块文档），类型不同名同形。
/// 转换写在一处、并且有单测钉住双向映射，比在两个调用点各写一次 `match` 可靠。
///
/// ⚠️ 不能写成 `impl From<lithe_gpui_settings::IdentityScope> for lithe_gpui_git::IdentityScope`：
/// 两个类型都是外部类型，违反孤儿规则（`E0117`）。所以这里定义一个**本地 trait**。
trait IntoGitIdentity {
    /// 目标类型（`lithe-gpui-git` 的对应枚举）。
    type Target;
    /// 转换。参数是 `&self` 而不是 `self`：两个枚举都是 `Copy`，但 `&self` 让
    /// `scope.to_git_identity()` 这种链式写法不必先把 `scope` 移走。
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

/// 在系统文件管理器里**定位**一个文件（标签右键菜单的「在资源管理器中显示」）。
///
/// 真源走的是 Tauri 插件 `revealItemInDir(path)`
/// （`windows/tauri/src/features/file-system/stores/file-system.store.ts:2689-2703`），
/// gpui 侧**没有等价物**（全量 grep 无 reveal 调用），所以只能自己起进程。
///
/// 平台差异用**运行时探测**表达，不写 `#[cfg(target_os)]`（与 `terminal/src/profile.rs`
/// 的 `command_exists` 同一条口径：Windows 先试 `explorer.exe`，它不存在时
/// `Command::spawn` 会返回 `NotFound`，再依次试 `open -R`（macOS）与 `xdg-open`（Linux））。
///
/// ⚠️ 落点在这里（外壳）而不是编辑区，与 `gpui/crates/explorer/src/lib.rs:86-87`
/// 已登记的"起 `explorer.exe` 属平台层，不该由 UI 模块做"是同一条约束。
fn reveal_in_file_manager(path: &Path) -> Result<(), String> {
    use std::process::{Command, Stdio};

    // `explorer.exe` 的 `/select,<path>` **必须**是同一个参数且不加引号外的转义
    // （`Command::arg` 会按 Windows 的参数规则给含空格的路径加引号，所以直接给整串）。
    let attempts: [(&str, Vec<String>); 3] = [
        ("explorer.exe", vec![format!("/select,{}", path.display())]),
        ("open", vec!["-R".to_string(), path.display().to_string()]),
        // `xdg-open` 只能开**目录**（没有"选中某个文件"的通用形式），
        // 所以父目录拿不到时退回文件本身所在的那一层。
        (
            "xdg-open",
            vec![path
                .parent()
                .unwrap_or(path)
                .display()
                .to_string()],
        ),
    ];

    let mut last = String::from("no known file manager");
    for (program, args) in attempts {
        match Command::new(program)
            .args(&args)
            // 三路 stdio 全部丢弃：文件管理器是长期存活的分离进程，接了管道等于把宿主的
            // 生命周期绑上去（终端会话那边同样刻意不接管道，见 `terminal/src/session.rs`）。
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

// ---------------------------------------------------------------------------
// 活动栏下标
// ---------------------------------------------------------------------------
/// 左侧活动栏「设置」项的下标（[`activity_items`] 的第 8 项、0 起第 7）。
///
/// 真机里「设置」打开的是**模态对话框**，不是底部工具窗（`gpui/research/windows/07-settings-ui.md`
/// §1.1），所以它既不在 [`bottom_pane_for`] 里，也不改活动栏的选中态：点它只把对话框打开。
///
/// ⚠️ 本项下标是 **7**（不是 8）：阶段 6 第一半把左栏的 Maven 项拿掉了（Maven 归右栏，
/// 见 [`activity_items`]），底部组从 6 项变成 5 项。
const SETTINGS_ACTIVITY_IX: usize = 7;

/// 左侧活动栏「顶部组」的下标范围（0 起、含首含尾）：项目 0 / 更改 1 / 搜索 2。
///
/// 真源 `features/layout/components/sidebar/sidebar-pane-selector.tsx:311-319`
/// （`files` / `git` / `search`）；底部组从 3 开始（`features/layout/config/item-order.ts:12-19`）。
const TOP_ACTIVITY_ITEMS: std::ops::RangeInclusive<usize> = 0..=2;

/// 左侧活动栏「更改」（源代码管理）项的下标 = 1。
///
/// 真源 `item-order.ts:3` 的顺序是 `files, git, search, …`，所以 `git` 是顶部组第 2 项
/// （0 起第 1）。左栏内容按这个下标在「项目树」与「更改列表」之间切换 —— 这是本侧左栏
/// 第一次出现**两个内容视图**（在此之前左栏恒为项目树）。
const CHANGES_ACTIVITY_IX: usize = 1;

/// 默认选中的顶部组视图：第 0 项「项目」。
///
/// 真源 `features/window/stores/workspace-ui-defaults.ts:7` 的 `activeSidebarView: "files"`
/// —— 启动时左活动栏顶部第一项就是选中态。
const DEFAULT_TOP_ACTIVITY: usize = 0;

// ---------------------------------------------------------------------------
// 度量：应用布局一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// rem base = 主题字号 16px，所以 helper 后缀 `N` = `N × 4px`，与 Windows 规格逐像素相等：
//
// | 规格（Windows 真源） | 值 | 用到的 helper |
// | --- | --- | --- |
// | `sidebarWidth: 320`（`features/settings/config/default-settings.ts:138`） | 320 | `w_80()` |
// | `--lithe-workbench-gap: 4px`（`styles/theme.css:125`） | 4 | `gap_1()` / `pr_1()` / `h_1()` |
// | 底部工具窗默认高 320（`bottom-pane/bottom-pane.tsx:47`） | 320 | `h_80()` |
// | 区域占位块内边距 8 | 8 | `p_2()` |
// | 占位块字号 13（`--ui-text-chrome`） | 13 | `text_sm()`（14px，见下） |
//
// 字号：13 不在 gpui 的档位（`text_xs()`=12 / `text_sm()`=14）上，按《编码指南》用
// `text_sm()`（14px）——13 → 14 是经维护者确认的**有意**视觉改动，不是等价换算。

/// 右侧工具窗宽度 400。真源 `default-settings.ts:139`（`rightToolWindowWidth: 400`），
/// 取值区间 140–600（`features/settings/lib/settings-normalization.ts:114-115`）。
///
/// ⚠️ **不用 `px(...)`**：400 不在 gpui 的 rem 档位上（档位里 96 → 384、112 → 448，
/// `gpui-pre-macros-0.3.6/src/styles.rs:1063-1077`），也不能自己发明一个 `w_100()`。
/// 按《编码指南》"档位外的值用 helper 底层的 `rems(P / 16.)`"写成 rem：基准 16px 时
/// `rems(400. / 16.)` 与 `px(400.)` 逐像素相等（25rem × 16 = 400），但会随主题字号缩放。
const RIGHT_TOOL_WINDOW_WIDTH: f32 = 400.;

/// 编辑器岛 / 侧栏外壳圆角。真源 `theme.css:9,134`：`rounded-xl = calc(--radius * 1.4) = 11.2`。
///
/// ⚠️ **保留 `px(...)`**：11.2 **不是** gpui 的 rem 档位（gpui 的 `rounded_xl()` 是 12px，
/// `gpui-pre-macros-0.3.6/src/styles.rs:1255-1259`）；也不能从主题读 —— `ThemeConfig.radius`
/// 是 `usize`（`gpui-component-0.6.6/src/theme/schema.rs:67-68`），装不下 Lithe 的
/// `--radius × k` 阶梯（4.8 / 6.4 / 11.2），而把主题半径改大又会连带改掉所有 gpui-kit 组件。
const ISLAND_RADIUS: Pixels = px(11.2);

/// 底部工具窗当前显示哪一个内容。
///
/// Windows 的底部窗**没有自己的标签条**，由活动栏 / 命令面板带一个单值
/// `bottomPaneActiveTab`（默认 `"terminal"`，
/// `features/window/stores/workspace-ui-defaults.ts:6`、`stores/ui-state/panel-slice.ts:31`）。
///
/// ⚠️ **可见性不在这个类型里**：`isBottomPaneVisible` 是独立字段（默认 `false`，
/// 同文件 `:5`），所以 `bottom_kind` 在面板隐藏时**保留最后显示过的页签** —— 这正是真机的行为：
/// 隐藏终端再做点别的、回来时 `bottomPaneActiveTab` 还是 `"terminal"`。
///
/// ⚠️ **没有 `Maven`**（阶段 6 第一半改）：真机的 Maven 工具窗开在**右侧栏**
/// （`features/maven/actions/maven-tool-window-actions.ts:58` 的 `toggleMavenToolWindow`
/// 走 `applyRightToolWindowIntent`，`main-layout.tsx:336-340` 把它画在右 `ResizablePane` 里），
/// 上一轮临时落到这里的那一项是错位的，已删除 —— 现在 Maven 只有右栏一处入口
/// （[`RightToolWindowView::Maven`]）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomPaneKind {
    /// 终端（阶段 5）。
    Terminal,
    /// Git 提交记录（阶段 4）。
    Git,
    /// 运行工具窗（阶段 6 前用占位内容）。
    Run,
    /// 诊断工具窗（阶段 6 前用占位内容）。
    Diagnostics,
}

impl BottomPaneKind {
    /// 工具窗标题里的名字，用于占位内容那句「{label} 工具窗（未实现）」。
    ///
    /// 走 `lithe_gpui_shared::tr`（界面不许出现中英文字面量）。真源键与原文：
    /// `lithe.workbench.run` = 「运行」、`lithe.workbench.diagnostics` = 「诊断」。
    fn label(self) -> SharedString {
        match self {
            Self::Run => tr("lithe.workbench.run"),
            Self::Diagnostics => tr("lithe.workbench.diagnostics"),
            // 终端与提交记录有自己的界面，永远不会走到占位分支；给同一个键只为不必返回 `Option`。
            Self::Terminal => tr("lithe.workbench.terminal"),
            Self::Git => tr("lithe.workbench.gitLog"),
        }
    }
}

/// 活动栏第 `index` 项对应的底部窗内容；`None` = 该项不换底部窗。
///
/// ⚠️ 下标必须与**左栏** [`activity_items`] 的顺序一致（0 项目 / 1 更改 / 2 搜索 / 3 运行 /
/// 4 终端 / 5 诊断 / 6 提交记录 / 7 设置）。「设置」在真机是对话框，不是底部窗，所以这里是 `None`。
/// **Maven 不在表里**：它归右活动栏（[`RightToolWindowView::Maven`]）。
fn bottom_pane_for(index: usize) -> Option<BottomPaneKind> {
    match index {
        3 => Some(BottomPaneKind::Run),
        4 => Some(BottomPaneKind::Terminal),
        5 => Some(BottomPaneKind::Diagnostics),
        6 => Some(BottomPaneKind::Git),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 命令面板的动作表（阶段 6 第二半）
// ---------------------------------------------------------------------------

/// 生成命令面板动作表时需要的三个**朝向**开关。
///
/// 为什么单独一个结构体而不是直接读 `ShellWorkspace`：动作表要能被两处以**只有 `&self`**
/// 的方式生成（`render` 里借不到 `&mut Context<Self>`），所以把"当前状态推出哪一条标签"
/// 这件事做成纯函数 [`command_action`]，开关由调用方先取成快照。
#[derive(Clone, Copy)]
pub(crate) struct ActionFlags {
    /// 底部工具窗当前显示的是不是终端（可见 && 页签 = 终端，判据同 [`ShellWorkspace::is_activity_active`]）。
    pub(crate) terminal_shown: bool,
    /// 右工具窗当前显示的是不是 Maven。
    pub(crate) maven_shown: bool,
    /// 状态栏当前是否显示（`Settings::show_status_bar`）。
    pub(crate) status_bar_shown: bool,
    /// 当前生效的主题是不是深色（判据见 [`ShellWorkspace::action_flags`]）：
    /// 只决定面板里给"切浅色"还是"切深色"。
    pub(crate) dark_theme: bool,
}

/// 一条动作的数据：分类 / 标签 / 描述 / 图标。**纯函数，没有副作用**。
///
/// ## 清单与真源逐条对应
///
/// | 本侧 id | 真源 | 真的改了什么 |
/// | --- | --- | --- |
/// | `open-settings` | `commandPalette.actions.open-settings.label`（`settings-actions.tsx:155-163`） | `SettingsStore` 对话框（常规页） |
/// | `open-appearance-settings` | `settingsTabCommands` 的 `appearance` 那一页（`settings-actions.tsx:127-138`） | 设置对话框且**停在外观页** |
/// | `switch-theme-light` / `-dark` | 真源是命令面板二级视图 `color-theme`（`command-palette.tsx:402-409`），执行 `handleThemeChange`（`:107-116`） | `SettingsStore::set_theme`（跟随系统时同时关掉 `syncSystemTheme`，与真源逐条一致） |
/// | `toggle-terminal` | `commandPalette.actions.toggle-terminal.*`（`view-actions.tsx:125-145`） | 底部工具窗的可见性 + 懒建终端会话 |
/// | `toggle-maven` | 左活动栏 maven 项 → `toggleMavenPane`（`maven-tool-window-actions.ts:58`） | 右工具窗 `resolve_click(Maven, …)` |
/// | `toggle-status-bar` | 设置项 `settings.appearance.showStatusBar`（`settings-actions.tsx` 没有对应动作） | `SettingsStore::set_show_status_bar` |
///
/// 没有放进来的（有意）：`toggle-sidebar`（左栏可见性）—— 本侧左栏是常驻的、没有收起态，
/// 放进来就是"点了没反应"；窗口 / 缩放 / pane / git / github / markdown 等动作对应的能力
/// 在 gpui 侧还不存在，理由见 `gpui/PLAN.md` 的阶段 6 第二半。
pub(crate) fn command_action(id: CommandId, flags: ActionFlags) -> CommandAction {
    use CommandId as C;
    // 只有命令面板里真的会出现的动作才需要分类与图标。
    // [`C::SaveBuffer`] / [`C::OpenCommandPalette`] / [`C::NavigateToDefinition`] /
    // [`C::ToggleMenuBar`] 这四条**不在面板里**（它们的入口是菜单项与快捷键，
    // 见 `COMMAND_ORDER` 的说明），所以给它们一个中性分类与图标而不是把它们排除在
    // 动作表之外 —— 否则菜单项就得自己再写一份 label / description。
    let (category, icon) = match id {
        C::OpenSettings | C::OpenAppearanceSettings => (Category::Settings, IconName::Settings),
        // 主题动作在两套界面里都用「调色板」字形（真源 `settings-actions.tsx` 的 `PaletteIcon`）。
        C::SwitchToLightTheme | C::SwitchToDarkTheme => (Category::Settings, IconName::Palette),
        C::ToggleTerminal => (Category::View, IconName::SquareTerminal),
        C::ToggleMaven => (Category::View, IconName::Package),
        C::ToggleStatusBar => (Category::View, IconName::PanelBottom),
        C::ToggleMenuBar => (Category::View, IconName::List),
        C::SaveBuffer => (Category::View, IconName::Save),
        C::OpenCommandPalette => (Category::View, IconName::Search),
        C::NavigateToDefinition => (Category::View, IconName::ArrowRight),
    };

    // 标签 / 描述：**能复用真源既有键就复用**（逐条的键与出处写在 `shared/src/i18n.rs`
    // 的 `WIRED` 清单里；剩下 9 条是 gpui 侧新增键，理由写在 `extract-locale.mjs`）。
    let (label, description) = match id {
        C::OpenSettings => (
            tr("lithe.commandPalette.actions.open-settings.label"),
            tr("lithe.settings.tabs.general"),
        ),
        C::OpenAppearanceSettings => (
            tr("lithe.commandPalette.actions.open-settings.label"),
            tr("lithe.settings.tabs.appearance"),
        ),
        // 朝向与实际生效的主题相反：跟着系统走时 `theme` 就是"首选深/浅主题"，
        // 与 `SettingsStore::set_theme` 写的那一支一致。
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
        // 下面四条**不在命令面板里**（见 `COMMAND_ORDER`），但菜单项会用到它们的文案，
        // 所以文案键与菜单那一份**同一个来源**（`MenuAction::label_key` 也是这些键），
        // 免得菜单与面板各写一份、迟早漂移。
        C::ToggleMenuBar => (
            tr("lithe.menu.toggleMenuBar"),
            tr("lithe.settings.appearance.showStatusBarDescription"),
        ),
        C::SaveBuffer => (tr("lithe.menu.save"), tr("lithe.workbench.project")),
        C::OpenCommandPalette => (
            tr("lithe.menu.commandPalette"),
            tr("lithe.commandPalette.placeholder"),
        ),
        C::NavigateToDefinition => (
            tr("lithe.menu.goToDefinition"),
            tr("lithe.navigation.definition"),
        ),
    };

    CommandAction {
        id,
        category,
        label,
        description,
        icon,
    }
}

/// 动作表顺序（= 面板里从上到下的顺序，也是 `run_command` 的行号含义）。
///
/// 按分类分块、块内按"用户最常找的"排：设置 → 主题 → 视图类。
/// 真源按 `createXxxActions` 的拼接顺序排（`command-palette.tsx:233-335`），本侧动作少，
/// 按上面的口径重排是**有意**的（真源那张表里有 200+ 条，顺序照搬没有意义）。
///
/// ⚠️ 主题那两条是**互斥**的（[`ShellWorkspace::command_actions`] 按当前主题只保留一条），
/// 所以面板里的实际行号与这里的下标**不总是一一对应** —— `run_command` 的 `row` 由
/// `CommandState` 的 `IndexPath` 给出，它数的是**渲染时那一份表**。
const COMMAND_ORDER: [CommandId; 7] = [
    CommandId::OpenSettings,
    CommandId::OpenAppearanceSettings,
    CommandId::SwitchToLightTheme,
    CommandId::SwitchToDarkTheme,
    CommandId::ToggleTerminal,
    CommandId::ToggleMaven,
    CommandId::ToggleStatusBar,
];

/// 打一行命令面板诊断（可 grep，与其他 `S1_*` 同一口径）。
fn diagnose_run(id: CommandId, state: &str) {
    println!("S1_COMMAND_RUN id={} state={state}", id.id());
}

/// 当前状态下**真的会出现在面板里**的动作，顺序 = 面板行序 = `run_command` 的行号。
///
/// 只做一件事：把主题那一对按当前配色二选一 —— 真机把两色都列出来是因为它走的是二级视图
/// （`command-palette.tsx:402-409` 的 `color-theme` 列出全部主题），本侧没有那一层，
/// 再同时列出"切浅色 / 切深色"就会出现一条按下去什么都不变的动作。
/// 当前是深色 → 只给"切浅色"，反之亦然（与真源 `handleThemeChange` 从当前值出发同一个意思）。
///
/// ⚠️ 这个函数是**动作表行序的唯一真源**：`command_actions`（画）与 `run_command`（执行）
/// 都必须经过它，否则两边会错位一行。
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

/// 工作台根视图。
pub struct ShellWorkspace {
    /// 项目标签条的数据。
    projects: Vec<ProjectTab>,
    /// 工作区根目录。
    ///
    /// 状态栏的 Git 分支项每帧要从 `.git/HEAD` 现读（见 [`ShellWorkspace::footer_left`]），
    /// 所以根目录要留着；`read_branch` 只在状态栏重绘时被调用（不是每帧定时器），
    /// 代价与真机前端的 `useGitBranch` 相当。
    root: PathBuf,
    /// 当前选中的项目标签（`None` = 一个都没选中）。
    active_project: Option<usize>,
    /// **左侧**活动栏的图标项（Windows `SidebarActivityRail`：顶部 3 + 底部 5 共 8 项）。
    ///
    /// ⚠️ 真机的左栏底部组是 6 项（含 maven），本侧是 5 项 —— Maven 只归右栏，
    /// 理由见 [`activity_items`]。
    activity_items: Vec<ActivityItem>,
    /// **右侧**活动栏的图标项（Windows `PluginActivityRail`：扩展 / 通知 / Maven 共 3 项）。
    ///
    /// 两条栏是**各自独立**的视图集合，不能共用一份列表：共用会让右栏变成左栏的镜像。
    right_activity_items: Vec<ActivityItem>,
    /// 左活动栏**顶部组**当前选中的视图（[`activity_items`] 的下标，默认第 0 项「项目」）。
    ///
    /// 与底部工具窗的状态**完全独立**：顶部组的选中来自真机的 `activeSidebarView`
    /// （`workspace-ui-defaults.ts:7`），底部组来自 `isBottomPaneVisible` + `bottomPaneActiveTab`
    /// （同文件 `:5-6`）。两者可以同时高亮。
    top_activity_view: Option<usize>,
    /// **右侧**工具窗当前显示的视图（真机 `activeRightSidebarView`，
    /// `stores/ui-state/view-slice.ts:12,26`）。
    ///
    /// 默认 [`RightToolWindowView::Maven`]：真机的默认值是 `"outline"`
    /// （同文件 `:26`），而右活动栏里没有 outline 这一项（本侧只有三项），所以这里取
    /// 「三个视图里唯一在真机真的开在右栏的那个」（`main-layout.tsx:104,336-340`，
    /// outline 是左栏视图）。面板默认隐藏（[`ShellWorkspace::right_visible`]），
    /// 所以这个取值在界面上不可见，只决定"第一次点别的项之前面板里是什么"。
    right_view: RightToolWindowView,
    /// 右侧工具窗是否展开。**默认 `false`（隐藏）**：真源
    /// `features/window/stores/ui-state/panel-slice.ts:28` 的 `isRightSidebarVisible: false`；
    /// `main-layout.tsx:101-105` 因此默认不渲染右工具窗。
    ///
    /// 与 [`ShellWorkspace::right_view`] 分开的理由同底部窗：收起时保留最后显示过的视图
    /// （`right-tool-window-actions.ts:26-31` 的 toggle 分支不改 `activeRightSidebarView`）。
    right_visible: bool,
    /// Maven 项目结构（`maven.scan` 的结论），**懒扫一次后缓存**。
    ///
    /// `None` 有三种情况，故意不区分：还没扫过、扫过但不是 Maven 项目、扫描失败 ——
    /// 三种都该显示真源的「未检测到 Maven 项目」空态（见 `crate::maven` 的模块文档）。
    /// 扫描时机是"右栏第一次切到 Maven 且可见"：`maven.scan` 要解析 pom，
    /// 放在启动路径上会让首帧为不相关的面板付钱。
    maven_project: Option<crate::maven::MavenProjectView>,
    /// 是否已经扫过（`None` 的三种情况靠它区分不了，但"扫过没有"必须能区分，
    /// 否则每次渲染都会重扫）。
    maven_scanned: bool,
    /// 左侧栏内容：项目树（真实 `workspace.snapshot` 数据）。
    ///
    /// 状态栏左组（前导项）**不再是字段**：它每帧现算（[`ShellWorkspace::footer_left`]），
    /// 因为第一项的文件类型图标要跟着"当前活动文件"变，存成字段就得在每个改活动文件的地方
    /// 同步（漏一处就显示错图标）。尾随组本来就是每帧现算的
    /// （[`ShellWorkspace::footer_right`]），这里对齐同一口径。
    explorer: Entity<Explorer>,
    /// 左侧栏的**第二个**内容视图：源代码管理（活动栏「更改」项，下标 [`CHANGES_ACTIVITY_IX`]）。
    ///
    /// 与 [`ShellWorkspace::explorer`] 并排存在、**同一个左栏槽位二选一渲染**：
    /// 真源是 `MainSidebar` 按 `activePaneId` 单选渲染一个 pane
    /// （`features/layout/components/sidebar/main-sidebar.tsx:778-816`），
    /// 本侧用 [`ShellWorkspace::top_activity_view`] 当下标做同一件事。
    /// 两个实体都常驻（不按需创建）：切换视图不该丢掉对方的滚动位置与展开状态。
    changes: Entity<ChangesView>,
    /// 中央列内容：编辑区（标签栏 + 正文 / 空状态）。
    editor: Entity<EditorPane>,
    /// 底部窗里的终端。**构造期不建会话**，第一次可见时由
    /// [`TerminalPane::ensure_session`] 懒创建。
    terminal: Entity<TerminalPane>,
    /// 底部窗里的 Git 提交记录。
    bottom_git: Entity<BottomPane>,
    /// 底部窗当前显示的内容。**与可见性分开**：隐藏时保留最后显示过的页签。
    bottom_kind: BottomPaneKind,
    /// 底部窗是否展开。**默认 `false`（隐藏）**：真源
    /// `features/window/stores/workspace-ui-defaults.ts:5` 的 `isBottomPaneVisible: false`；
    /// 同文件 `:6` 的 `bottomPaneActiveTab: "terminal"` 说的是**默认页签**是终端，
    /// 不是"启动就显示"（维护者 2026-09-25 实测反馈：终端默认应隐藏）。
    bottom_visible: bool,
    /// 订阅 `SettingsStore`：设置一变就重绘（状态栏的显示/隐藏就靠它）。
    ///
    /// 订阅必须**被持有**：`Subscription` 一 drop 就取消（gpui 的 RAII 语义），
    /// 所以放在结构体里而不是丢在 `new()` 的局部变量里。
    _settings_subscription: Option<gpui_kit::Subscription>,
    /// 订阅终端面板的 [`TerminalPaneEvent::LastTabClosed`]：关掉最后一个终端页签时自动收起
    /// 底部工具窗（真机 `features/terminal/utils/terminal-pane-visibility.ts:14-26`）。
    ///
    /// 与上面同理，必须持有；字段声明在 `terminal` **之后**，所以 drop 顺序是
    /// 先取消订阅、再销毁终端面板（先取消再被通知才不会被拆到一半的实体回调）。
    _terminal_subscription: Option<gpui_kit::Subscription>,
    /// 观察编辑区：光标位置变了就重绘（状态栏的 `行:列` 是它的下游）。
    ///
    /// 光标信息归编辑区所有，外壳只读；`EditorPane` 只在位置**真的变了**时
    /// `cx.notify()`，所以这里不会跟着编辑器的每一次重绘空转。
    /// 同样是 RAII，必须持有。
    _editor_subscription: Option<gpui_kit::Subscription>,
    /// 外壳根元素的焦点句柄，真机上由 `Ctrl+S` 的 `SaveBuffer` 处理器与
    /// 命令面板的全局快捷键共用。
    ///
    /// **为什么根元素要有焦点**：gpui 的按键派发路径由当前焦点节点决定
    /// （`gpui-pre-0.3.6/src/window.rs:5815-5816`），没有焦点节点时 keymap 一条都匹配不到
    /// —— 也就是说"启动后没点过任何地方"，`Ctrl+S` 与 `Ctrl+Shift+P` 都不会响。
    /// 根元素挂 `.track_focus(..)` 之后，只要没有后代元素抢走焦点，键盘事件就落在这一层，
    /// 全局 action 照常派发。理由与实测见 `crate::command_palette` 的 `SHELL_FOCUS`。
    focus: gpui_kit::FocusHandle,
    /// 主菜单栏句柄（Windows 规格，见 [`crate::menu_bar`]）。
    ///
    /// 菜单栏**不持有在这里的字段里**：它是 `Entity`（自己持有形态、当前打开的顶级菜单与
    /// "待执行的菜单动作"队列），句柄登记在 `crate::menu_bar` 的 `MENU_BAR` 里，
    /// 这样 [Render::render] 能借出 `&mut App` 去画它（见 `menu_bar::with_state` 的说明），
    /// 而 `Ctrl+M` 的应用级 action 也够得着它。
    ///
    /// ⚠️ 这里持有**强引用**：`Entity` 一 drop 菜单栏就被销毁（`WeakEntity` 在
    /// `ShellWorkspace` 侧就升级不到了）。字段本身不参与渲染，只负责生命周期。
    menu_bar: Entity<MenuBar>,
    /// 标题栏项目下拉的句柄（Windows 规格，见 [`crate::project_menu`]）。
    ///
    /// 与 [`ShellWorkspace::menu_bar`] 同一处置：面板的开关状态要跨帧存在，而这个实体
    /// **不是** `Render`（由本视图画），所以状态变化通过下面的观察订阅让外壳重绘。
    /// 强引用同样是必需的生命周期锚点（句柄登记在 `crate::project_menu` 的 `PROJECT_MENU` 里）。
    project_menu: Entity<ProjectMenu>,
    /// 观察项目下拉：**开合状态一变就重绘外壳**。
    ///
    /// 必须持有：`Subscription` 一 drop 就取消（gpui 的 RAII 语义）。
    /// 用 observe 而不是像菜单栏那样反向持有外壳句柄：项目下拉只需要"让外壳重画"这一件事，
    /// 订阅是这一件事最短的表达（`editor` / `SettingsStore` 也是这么接的）。
    _project_menu_subscription: Option<gpui_kit::Subscription>,
    /// 标题栏「分支项 + 分支弹窗」的句柄（Windows 规格，见 [`crate::branch_panel`]）。
    ///
    /// 与 [`ShellWorkspace::project_menu`] 同一处置：面板的开关与**数据**都要跨帧存在，
    /// 而这个实体也**不是**本视图的 `Render`，所以状态变化通过下面的观察订阅让外壳重绘。
    /// 强引用同样是必需的生命周期锚点（`Entity` 一 drop 面板就被销毁）。
    branch_panel: Entity<BranchPanel>,
    /// 观察分支弹窗：开合 / 刷新一变就重绘外壳（标题栏那一项与面板都归外壳画）。
    ///
    /// 必须持有：`Subscription` 一 drop 就取消（gpui 的 RAII 语义）。
    _branch_panel_subscription: Option<gpui_kit::Subscription>,
    /// `--branch-panel-probe`：**首帧之后**把分支弹窗打开（**验证/诊断用**）。
    ///
    /// 收成字段而不是在 `new` 里立刻打开：浮层只能在事件回调 / 任务 / 首帧之后的 render 里
    /// `open_dialog`，而 `Root` 还没建好时窗口根不是 `Root`（`main.rs` 的启动顺序表）。
    /// 消费点是 [`ShellWorkspace::render`] 的第一帧。
    branch_panel_probe: bool,
}

impl ShellWorkspace {
    /// 建立工作台视图。`compact_menu` = `true` 时菜单栏走"左上角图标 + 浮动胶囊"形态
    /// （真源默认值），`false` 走"与标题栏同一行的常驻形态"（**本侧默认**，维护者口径）。
    ///
    /// `branch_panel_probe` = `true` 时**首帧之后**把标题栏的分支弹窗打开（`--branch-panel-probe`，
    /// **验证/诊断用**，理由见字段文档）；产品路径恒为 `false`。
    ///
    /// `Root::new` 之前不得打开任何浮层（那时窗口根还不是 `Root`，`window.open_dialog` 会 panic）。
    pub fn new(
        root: PathBuf,
        compact_menu: bool,
        branch_panel_probe: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // 编辑区的应用级快捷键（`Ctrl+S`）在这里登记：编辑区没有自己的启动入口，
        // 而 workspace 是外壳的组合点。与 `lithe_gpui_settings::install_actions` 同一口径：
        // 绑定是应用级、只登记内容；真正的处理在本视图根元素的 `SaveBuffer` 处理器上。
        // `Ctrl+F` / `Ctrl+H` 不在这里 —— 它们归编辑器组件自己的 `Input` 上下文绑定
        // （见 `lithe_gpui_editor::install_actions` 的说明）。
        lithe_gpui_editor::install_actions(cx);
        // 左栏「更改」视图的 `Ctrl+Enter`（提交）同样登记成应用级 action：它的处理器在
        // `ChangesView` 的根元素上，而"启动后没点过任何地方"时那颗处理器够不着键盘事件
        // （理由与上面 `Ctrl+S` 完全一样）。
        lithe_gpui_git::install_actions(cx);
        // 命令面板的 `Ctrl+Shift+P`（真源 `cmd+shift+p` 在 Windows 上的归一化形式，
        // 出处见 `crate::command_palette` 的模块文档）。同样登记成**全局 action**：
        // 挂在根元素上会有"启动后没点过任何地方时按不出来"的死角。
        install_command_palette(cx);

        // 先建编辑区，再把它的弱引用交给项目树：点文件 → 打开到编辑区。
        let editor = cx.new(|cx| EditorPane::new(window, cx));
        // 阶段 10 第二批：打开项目时在后台起 Java 语言服务（= 生成 / 复用 JDT 索引缓存）。
        // 放在这里而不是编辑区自己：工作区根是外壳的参数，编辑区不认识它。
        // 整段是后台任务，失败只留 `S1_JAVA_*` 诊断，跳转退回第一批的轻量链路。
        editor.update(cx, |pane, cx| pane.prepare_java(root.clone(), cx));
        let editor_handle = editor.downgrade();

        let on_open: Box<dyn Fn(PathBuf, &mut Window, &mut App) + 'static> =
            Box::new(move |path: PathBuf, window: &mut Window, cx: &mut App| {
                // 编辑区可能已经被关掉：`update` 返回 `Err` 时静默忽略，不 panic。
                let _ = editor_handle.update(cx, |pane, cx| pane.open(&path, window, cx));
            });

        let explorer_root = root.clone();
        let explorer = cx.new({
            // 只在本块内借用 `window`：`cx.new` 的闭包是 `FnOnce`、用完即释放，
            // 块外还要用 `window` 调一次 `refresh`（构造期不取数据）。
            let window = &mut *window;
            move |cx| Explorer::new(explorer_root, on_open, window, cx)
        });
        // 构造期不取数据，第一帧之后立刻去拉 `workspace.snapshot`。
        explorer.update(cx, |this, cx| this.refresh(window, cx));

        // 左栏「更改」（源代码管理）。**构造期不取数据**：顶部组默认选「项目」
        // （`DEFAULT_TOP_ACTIVITY`），所以这个视图在第一帧不可见；它的第一次读取发生在
        // 用户点活动栏「更改」时（`on_select_activity` 调 `ChangesView::activate`），
        // 之后由「手动刷新 + 写后刷新 + 再次激活」维持（本侧刻意不做 watcher，见 crate 文档）。
        let changes_root = root.clone();
        let changes = cx.new({
            let window = &mut *window;
            move |cx| ChangesView::new(changes_root, window, cx)
        });

        let terminal = cx.new(|cx| TerminalPane::new(window, cx));

        // 标签右键菜单（`gpui/research/windows/09-tab-context-menu.md`）：把工作区根与
        // 两件"只有外壳做得了"的动作登记给编辑区。
        //
        // ⚠️ **必须排在 `terminal` 之后**：两个回调都要捕获终端实体的弱引用（「在终端中打开」
        // 要新开一个带工作目录的页签，并把底部工具窗切到终端且显示）。
        // 被绕开的只有"操作系统把这次点击送进窗口"那一段 —— 回调里走的都是与主菜单
        // 「终端 → 新建终端」同一个 `TerminalPane::new_tab_in` / 同一个 `bottom_visible`。
        {
            let shell = cx.entity().downgrade();
            let terminal_handle = terminal.downgrade();
            let workspace_root = root.clone();
            editor.update(cx, |pane, _cx| {
                // 「复制相对路径」要工作区根（无根时拷全路径，与真机同口径）。
                pane.set_workspace_root(workspace_root);
                pane.set_tab_menu_host_actions(TabMenuHostActions {
                    // 「在资源管理器中显示」：起系统文件管理器的定位进程 ——
                    // 这与 `gpui/crates/explorer/src/lib.rs:86-87` 登记的"reveal 属平台层、
                    // 不该由 UI 模块做"是同一条约束，所以落点在这里（外壳）而不是编辑区。
                    reveal: Arc::new(|path, _window, _cx| {
                        if let Err(error) = reveal_in_file_manager(path) {
                            // 失败不静默：`S1_TAB_MENU` 与 `S1_*` 一族同口径走 stderr。
                            eprintln!(
                                "S1_TAB_MENU run=reveal result=failed path={} error={error}",
                                path.display()
                            );
                        }
                    }),
                    // 「在终端中打开」：以文件**所在目录**新开一个终端页签 + 显示底部工具窗。
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
            });
        }

        let bottom_git = cx.new(|cx| BottomPane::new(root.clone(), window, cx));

        // 设置里的「Git」页要读写提交身份（`user.name` / `user.email`），而那条通路是
        // **本 crate 才拼得起来**的：Core 命令住在 `lithe-gpui-git`，设置 crate 不许依赖它
        // （依赖方向见 `lithe-gpui-settings` 的 `identity` 模块文档）。
        //
        // 所以照仓库里已有的同类口径（`TabMenuHostActions`：编辑区定义回调、外壳登记）：
        // 设置侧定义钩子，这里用 `lithe-gpui-git` 的实现填进去。
        // ⚠️ **必须比 `open_settings_dialog` 早**：对话框一开就会画内容页。
        {
            let identity_root = root.clone();
            lithe_gpui_settings::set_git_identity_host(Some(GitIdentityHost {
                workspace_root: identity_root.clone(),
                // 两条回调都：`background_spawn` 跑同步的 Core 调用 → 回前台把结果
                // 交给设置侧给的 delegate。与 `changes.rs` / `explorer` 的现成写法一致。
                //
                // 两条回调都要自己拿一份根的文本（闭包是 `'static` 的，借用逃不出去），
                // 所以各克隆一次、各自在调用时再克隆进后台任务。
                load: {
                    let root_text = identity_root.to_string_lossy().to_string();
                    Box::new(
                        move |scope: &str,
                              deliver: GitIdentityDeliver,
                              async_cx: &mut gpui_kit::AsyncWindowContext| {
                        let scope = IdentityScope::from_id(scope);
                        let root = root_text.clone();
                        // 任务由设置侧的 `Context::spawn_in` 起（那里才有窗口），
                        // 这里只用它跑 Core 并回前台投递。
                        // ⚠️ `detach()` 而不是 `let _ =`：gpui 的 `Task` 一 drop 就**取消**
                        // （`gpui-pre-scheduler-0.3.6/src/executor.rs:389-391`）。
                        // 本轮实测：写成 `let _ =` 时 `result=…` 永不出现、
                        // 界面卡在「正在检查 Git 仓库…」。宿主内部 detach，
                        // 所以本函数返回 `()`（见 `GitIdentityHost` 的文档）。
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
                                        // 失败**不静默**：与 `S1_*` 一族同口径走 stderr。
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
                        // `detach()` 的理由同 load 那条（`Task` 一 drop 就取消）。
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

        // 设置变了要重绘（「显示状态栏」立即生效）。`try_store` 而不是 `store`：
        // 工作台在测试或将来别的宿主里可能没有设置状态，那时回落"默认显示状态栏"，
        // 而不是 panic。
        //
        // 这条订阅同时是**三个"值"型设置**的转发点（阶段 14 / 15）：缩进宽度、终端默认 Shell
        // 与「丢弃前确认」都是"面板自己不认识设置 crate"的值（依赖方向：
        // `workbench` → `editor`/`terminal`/`git`），所以由外壳读出来、再喂给三个面板。
        // 启动时先各喂一次，之后每次设置变化再喂。
        let changes_for_settings = changes.clone();
        let settings_subscription = lithe_gpui_settings::try_store(cx).map(|store| {
            let initial = store.read(cx).settings().clone();
            // 启动态证据行：这行的值来自**设置文件读出来的**设置，所以它同时证明
            // "文件被读回来了"与"外壳确实拿到了这句设置"。阶段 14 就是靠它抓到
            // "`tabSize` 写得出、读不回"那个 bug 的（见 `PLAN.md` §14.2）。
            println!(
                "S1_SETTINGS wiring=workbench tab_size={} terminal_default_shell_id={:?} confirm_before_discard={}",
                initial.tab_size, initial.terminal_default_shell_id, initial.confirm_before_discard
            );
            editor.update(cx, |pane, cx| {
                pane.set_tab_size(initial.tab_size as usize, cx);
            });
            terminal.update(cx, |pane, cx| {
                pane.set_default_shell(&initial.terminal_default_shell_id, cx);
            });
            changes_for_settings.update(cx, |view, cx| {
                view.set_confirm_before_discard(initial.confirm_before_discard, cx);
            });
            cx.observe(&store, |this, store, cx| {
                let settings = store.read(cx).settings().clone();
                this.editor.update(cx, |pane, cx| {
                    pane.set_tab_size(settings.tab_size as usize, cx);
                });
                this.terminal.update(cx, |pane, cx| {
                    pane.set_default_shell(&settings.terminal_default_shell_id, cx);
                });
                // ⚠️ 用字段 `changes`（不是外层捕获的那个实体）：这条闭包要在
                // `ShellWorkspace` 上取本视图自己的引用，否则会和上面的 `cx.observe` 抢借用。
                this.changes.update(cx, |view, cx| {
                    view.set_confirm_before_discard(settings.confirm_before_discard, cx);
                });
                cx.notify();
            })
        });

        // 关掉最后一个终端页签 → 收起底部工具窗（真机
        // `features/terminal/utils/terminal-pane-visibility.ts:14-26`）。
        // 终端面板只知道"我还有几个页签"，可见性归本结构体，所以走事件而不是让
        // `TerminalPane` 直接改宿主的布局状态。
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

        // 编辑区 → 外壳：光标位置变了就重绘，状态栏的 `行:列` 才跟着走。
        let editor_subscription = Some(cx.observe(&editor, |_, _, cx| cx.notify()));

        let project_name: SharedString = root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| root.display().to_string())
            .into();

        // 启动期读一次分支只为**诊断**：状态栏那一格现在每帧现算
        // （见 `ShellWorkspace::footer_left`），但"这个工作区根本读不到 .git/HEAD"
        // 这件事在无人值守验证里必须能看见，所以启动时打一行可 grep 的证据。
        let branch = read_branch(&root).unwrap_or_else(|| "—".to_string());
        eprintln!("S1_BRANCH name={branch}");

        // 兜底焦点锚点：先建句柄，菜单栏与根元素**共用同一个**（菜单收起后焦点回到它，
        // 见 `crate::menu_bar` 的 `MenuBar::action_context`）。
        let focus = cx.focus_handle();
        // 菜单栏：形态来自启动参数，构造期自己打一行 `S1_MENU_BAR`。
        // 它要一个外壳句柄来通知重绘（菜单栏不是 `Render`，由本视图画），
        // 所以先把本视图的句柄登记进去，再建菜单栏。
        set_menu_bar_shell(cx.entity().downgrade());
        let menu_bar = MenuBar::new(mode_for(compact_menu), focus.clone(), cx);
        // 把它登记到 `crate::menu_bar` 的 `MENU_BAR`：渲染期取 `&mut App` 画它、
        // `Ctrl+M` 的全局 action 也走这个句柄（理由见 `menu_bar::set_menu_bar`）。
        set_menu_bar(menu_bar.downgrade());

        // 标题栏的项目下拉（`crate::project_menu`）。与菜单栏同一套：
        // 先建实体（构造期打一行 `S1_PROJECT_MENU opened=false` 的启动证据），
        // 再登记句柄给 `--project-menu-probe` 用，最后订阅它让自己重绘。
        let project_menu = ProjectMenu::new(project_name.as_ref(), focus.clone(), cx);
        set_project_menu(project_menu.downgrade());
        let project_menu_subscription = Some(cx.observe(&project_menu, |_, _, cx| cx.notify()));

        // 标题栏的分支项 + 分支弹窗（`crate::branch_panel`）。同一套：
        // 构造期自己读一次 `git.status` + `git.references`（标题栏那一项第一帧就要画对），
        // 打一行 `S1_BRANCH_PANEL opened=false …` 的启动证据，最后订阅它让自己重绘。
        let branch_panel = BranchPanel::new(root.clone(), window, cx);
        let branch_panel_subscription = Some(cx.observe(&branch_panel, |_, _, cx| cx.notify()));

        let workspace = Self {
            projects: vec![ProjectTab::new(project_name.clone())],
            root: root.clone(),
            // 单项目时 Windows 会隐藏整条标签条（`project-tab-bar-model.ts:12-13`）；
            // 阶段 1 只有一个项目，仍然把 `Some(0)` 选中，方便验收外观。
            active_project: Some(0),
            activity_items: activity_items(),
            right_activity_items: right_activity_items(),
            // 默认选中顶部第 0 项「项目」（`workspace-ui-defaults.ts:7` 的 `activeSidebarView: "files"`）。
            top_activity_view: Some(DEFAULT_TOP_ACTIVITY),
            // 右工具窗默认**隐藏**（`panel-slice.ts:28` 的 `isRightSidebarVisible: false`），
            // 视图字段的默认值理由见字段文档。
            right_view: RightToolWindowView::Maven,
            right_visible: false,
            maven_project: None,
            maven_scanned: false,
            explorer,
            changes,
            editor,
            terminal,
            bottom_git,
            // 默认页签是终端（`workspace-ui-defaults.ts:6` 的 `bottomPaneActiveTab: "terminal"`），
            // 但默认**不显示**（同文件 `:5` 的 `isBottomPaneVisible: false`）。
            bottom_kind: BottomPaneKind::Terminal,
            bottom_visible: false,
            _settings_subscription: settings_subscription,
            _terminal_subscription: terminal_subscription,
            _editor_subscription: editor_subscription,
            // 根元素的兜底焦点锚点（理由见字段文档）。
            focus,
            menu_bar,
            project_menu,
            _project_menu_subscription: project_menu_subscription,
            branch_panel,
            _branch_panel_subscription: branch_panel_subscription,
            branch_panel_probe,
        };
        // 启动期也留一行状态证据：右工具窗**默认隐藏**这件事要能被机器验证，
        // 而不是只靠截图比对（`S1_RIGHT_PANEL`，可 grep）。
        diagnose_right_panel(workspace.right_view, workspace.right_visible);
        // 命令面板的动作要改本视图的状态，而浮层的 builder / 回调都是 `'static`，
        // 够不着 `self`：所以在这里登记一个弱引用句柄（理由见 `crate::command_palette`
        // 的 `SHELL`）。登记在 `Self` 建好之后，句柄一定是可升级的。
        set_shell(cx.entity().downgrade());
        // 兜底焦点锚点的句柄（理由见 `crate::command_palette` 的 `SHELL_FOCUS`）。
        set_shell_focus(workspace.focus.clone());
        // 根元素**主动要一次焦点**：无人值守启动（验证脚本）时 gpui 不会自己给任何一个
        // 元素焦点，而"没有焦点节点"会让全部全局快捷键失效 —— 真机上用户点一下界面就有了，
        // 自动化里没有这一步。有后代元素持有焦点时，user 的点击会照常把焦点移走。
        window.focus(&workspace.focus, cx);
        workspace
    }

    /// 项目下拉要显示的项目条目（真源 `projectTabs` 的等价物，见 [`crate::project_menu`]）。
    ///
    /// ⚠️ 每一条的**路径**都取工作区根：本侧 [`ProjectTab`] 的契约只有 `name`
    /// （`project_tabs.rs:203-208`），而现在也只有一个工作区根，所以这一条是准确的。
    /// 等 `ProjectTab` 长出 `path` 之后这里改成逐条取（研究 §5.2-B 第 1 行登记了那个缺口）。
    ///
    /// `active` 与 [`ShellWorkspace::active_project`] **同源**：真源面板读的也是
    /// `projectTabs.find(p => p.isActive)`（`title-project-menu.tsx:112`），而标签条的高亮读的是
    /// 同一个下标 —— 两处不同源会让"面板里的当前项"和"标签条上的高亮"打架。
    /// 判据写成"下标相等"而不是"下标 0"，所以关掉当前项目（`active_project` 变 `None`）之后
    /// 面板里**不会**再有任何一行打勾。
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

    /// 当前状态下命令面板的动作表（顺序 = 面板里的行序 = [`ShellWorkspace::run_command`]
    /// 的行号）。纯读：不改任何状态，也不生成任何界面元素。
    ///
    /// 判据与活动栏高亮**同源**（[`ShellWorkspace::is_activity_active`]）：终端"是不是开着"
    /// 就是 `bottom_visible && bottom_kind == Terminal`，Maven 就是
    /// `right_visible && right_view == Maven` —— 这样面板里的标签朝向与界面上看到的一致。
    pub(crate) fn command_actions(&self, cx: &App) -> Vec<CommandAction> {
        let flags = self.action_flags(cx);
        visible_commands(flags)
            .into_iter()
            .map(|id| command_action(id, flags))
            .collect()
    }

    /// 动作表需要的朝向快照（理由见 [`ActionFlags`]）。
    fn action_flags(&self, cx: &App) -> ActionFlags {
        // 状态栏：没有设置状态时按默认值（显示）处理 —— 与 `render` 里那条判据一致，
        // 否则面板会说"隐藏状态栏"而界面上根本没有状态栏。
        let (status_bar_shown, dark_theme) = lithe_gpui_settings::try_store(cx)
            .map(|store| {
                let store = store.read(cx);
                let settings = store.settings();
                // 主题为什么看 `applied_theme()` 而不看 `settings.theme`：与设置对话框显示
                // "当前生效的主题"同一条口径（`settings/src/dialog.rs`）。跟随系统时
                // `applied_theme()` 按系统外观解析，面板因此不会给出一条"按了没变化"的动作。
                //
                // 判据用"主题名里有没有 light"，与 `AppearanceMode::from_settings`
                // 完全一致（`settings/src/store.rs:55-64`）—— 同一个问题只留一条判据。
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

    /// 执行命令面板的第 `row` 条动作（`row` = [`ShellWorkspace::command_actions`] 的行号）。
    ///
    /// **每一条都真的改到状态**：不是日志占位。越界行号直接返回（不 panic、不静默落到某项）。
    ///
    /// 与真源的对应关系与"为什么只有这几条"写在 [`command_action`] 的文档上。
    pub(crate) fn run_command(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        // 行号 → 动作 id 必须经过 `visible_commands`（面板渲染时用的同一条），
        // 否则主题那两条的互斥会把行号错开一位。
        let Some(id) = visible_commands(self.action_flags(cx)).get(row).copied() else {
            return;
        };
        // 执行逻辑只有一份：`run_command_id`（菜单也走它）。
        self.run_command_id(id, window, cx);
    }

    /// 执行一条菜单动作（见 [`crate::menu_bar::MenuAction`]）。
    ///
    /// `pub` 是给 `--menu-probe` 用的（诊断入口，见 `crate::menu_bar::open_by_id` 的说明）：
    /// 外壳才是"把动作落到 `&mut Window` 上"的那一层，菜单栏够不到窗口。
    ///
    /// **两条路**：
    ///
    /// 1. 能映射到 [`CommandId`] 的（[`MenuAction::command_id`] 有值）→ 直接调
    ///    [`ShellWorkspace::run_command`]，与命令面板**共用同一个执行点**；
    /// 2. 其余 6 条（新建/关闭终端页签、首选项、窗口三键）在这里各自接一段**已有实现**，
    ///    每一段的接线点写在下面。
    ///
    /// 不存在"点了没反应"的分支：能进这张表的动作在 [`crate::menu_bar::MENUS`] 的测试里
    /// 被逐条要求"有 CommandId 或在 NON_COMMAND 里登记过"。
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
            MenuAction::NewTerminalTab => {
                // 接线点：`TerminalPane::new_tab`（`terminal/src/session.rs:345`），
                // 真机入口是 `terminal.new`（`cmd+t`，`command-registry.ts:258-267`）与页签条
                // 右端的 `+`。菜单点击时输入行早就在树上，所以直接调即可（见该方法文档）。
                //
                // ⚠️ 顺带把底部工具窗切到终端并显示：菜单的「新建终端」不显示终端就等于没反应。
                self.bottom_kind = BottomPaneKind::Terminal;
                self.bottom_visible = true;
                let terminal = self.terminal.clone();
                let _ = terminal.update(cx, |pane, cx| pane.new_tab(window, cx));
                diagnose_menu_run(action, "applied");
            }
            MenuAction::CloseTerminalTab => {
                // 接线点：`TerminalPane::close_active_tab`（本模块本轮新增的公开入口，
                // 内部走既有的 `close_tab`，`session.rs:451`）。关掉最后一个页签时
                // `TerminalPaneEvent::LastTabClosed` 会把底部窗收起（既有订阅，`:490-499`）。
                let terminal = self.terminal.clone();
                let closed = terminal.update(cx, |pane, cx| pane.close_active_tab(cx));
                diagnose_menu_run(action, if closed { "applied" } else { "no_tab" });            }
            MenuAction::Preferences => {
                // 接线点：`lithe_gpui_settings::open_settings_dialog_at(.., General)`
                // （`settings/src/dialog.rs`；快捷键 `ctrl-,` 在 `install_actions` 里登记）。
                // 真源 `menu.preferences` 开的也是常规页（`settings-actions.tsx:155-163`）。
                diagnose_menu_run(action, "open");
                lithe_gpui_settings::open_settings_dialog_at(window, cx, SettingsCategory::General);
            }
            MenuAction::Minimize => {
                // 接线点：`Window::minimize_window`（`gpui-pre-0.3.6/src/window.rs:6416`）。
                // 标题栏右侧的三键走 `WindowControlArea`（由系统完成动作），菜单这一条
                // 是同一个能力在 gpui 侧的公开 API。
                window.minimize_window();
                diagnose_menu_run(action, "applied");
            }
            MenuAction::Maximize => {
                // 接线点：`Window::zoom_window`（`:2835`）+ `Window::is_maximized`（`:2374`）。
                // 已经是最大化时这个菜单项的含义是"还原"，而 gpui 侧没有 `restore_window`
                // （`Window` 上只有 `zoom_window` 会在两态之间来回切），所以文案照真源
                // 只写「最大化」—— 真源同样只有一个标签（`menu.maximize`，
                // `window-menu-bar.tsx:473-481`），不是本侧漏了一个「还原」。
                window.zoom_window();
                diagnose_menu_run(action, "applied");
            }
            MenuAction::ToggleFullscreen => {
                // 接线点：`Window::toggle_fullscreen`（`:6421`）。
                window.toggle_fullscreen();
                diagnose_menu_run(action, "applied");
            }
            // 走上一条 `if let` 的七条在这里不可达；写全分支是为了让"表里加一条动作"
            // 变成编译错误而不是运行时静默。
            MenuAction::Save
            | MenuAction::CommandPalette
            | MenuAction::ToggleTerminal
            | MenuAction::ToggleMaven
            | MenuAction::ToggleStatusBar
            | MenuAction::OpenAppearanceSettings
            | MenuAction::GoToDefinition => {
                diagnose_menu_run(action, "unreachable");
            }
        }
        cx.notify();
    }

    /// 按 [`CommandId`] 执行——[`ShellWorkspace::run_command`] 的 id 版本，也是**两条界面的
    /// 唯一执行点**：命令面板（行号 → id）与主菜单（[`MenuAction::command_id`]）都落到这里。
    ///
    /// ⚠️ 其中 [`CommandId::SaveBuffer`] / [`CommandId::OpenCommandPalette`] /
    /// [`CommandId::NavigateToDefinition`] / [`CommandId::ToggleMenuBar`] **不在命令面板里**
    /// （[`COMMAND_ORDER`] 没有它们），只由菜单项或快捷键触发 ——
    /// 各自的接线点写在分支上。
    fn run_command_id(&mut self, id: CommandId, window: &mut Window, cx: &mut Context<Self>) {
        match id {
            CommandId::SaveBuffer => {
                // 接线点：`EditorPane::save_active`（`editor_view.rs:371`），
                // 与 `Ctrl+S` 的 `SaveBuffer` 处理器（本文件 `render` 里）同一条路。
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.save_active(window, cx));
                diagnose_run(id, "saved");
            }
            CommandId::OpenCommandPalette => {
                // 接线点：`crate::command_palette::open_command_palette` —— 与 `Ctrl+Shift+P`
                // 的全局处理器里那一句**是同一个函数**。
                diagnose_run(id, "open");
                crate::command_palette::open_command_palette(window, cx);
            }
            CommandId::NavigateToDefinition => {
                // 接线点：`EditorPane::navigate_to_definition`（`F12` 的处理器在
                // `editor_view.rs:1480`，本句与它调的是同一个方法）。
                let _ = self
                    .editor
                    .update(cx, |pane, cx| pane.navigate_to_definition(window, cx));
                diagnose_run(id, "applied");
            }
            CommandId::ToggleTerminal => {
                // 与活动栏点「终端」同一条迁移（`is_activity_active` 的判据同源）：
                // 已经开着就收起，否则切到终端页签并显示。
                let terminal_handle = self.terminal.clone();
                let (visible, ensure_session) = if self.bottom_visible
                    && self.bottom_kind == BottomPaneKind::Terminal
                {
                    self.bottom_visible = false;
                    (false, false)
                } else {
                    self.bottom_kind = BottomPaneKind::Terminal;
                    self.bottom_visible = true;
                    (true, true)
                };
                diagnose_run(id, if visible { "visible" } else { "hidden" });
                if ensure_session {
                    // 幂等：会话已存在就什么都不做；首次创建由它自己把焦点延到帧末交给输入行。
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
                // 与右活动栏点击同一条懒扫口径（菜单与右栏改的是同一份状态，取数据也该一致）。
                if visible && view == RightToolWindowView::Maven && !self.maven_scanned {
                    self.maven_scanned = true;
                    self.maven_project = crate::maven::scan(&self.root);
                }
                // 与右活动栏点击走同一个诊断（`S1_RIGHT_PANEL view=maven visible=…`）：
                // 这样"菜单里的 Maven 项和右栏那一项改的是同一份状态"有机器证据。
                diagnose_right_panel(view, visible);
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
                lithe_gpui_settings::open_settings_dialog_at(
                    window,
                    cx,
                    SettingsCategory::General,
                );
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
        cx.notify();
    }

    /// 切配色主题：立即生效 + 防抖落盘。
    ///
    /// 照真源命令面板的 `handleThemeChange`（`command-palette.tsx:107-116`）：用户从命令面板
    /// 点名要一个主题，意思就是**别跟着系统了**，所以走
    /// `SettingsStore::set_theme_explicit`（它把 `syncSystemTheme` 一起关掉）。
    /// ⚠️ 与设置对话框里那个主题下拉的口径**不同**（那边跟随系统时改的是"首选深/浅主题"，
    /// `macos-settings-panels.tsx:115-123`）—— 这正是真源自身的两种写法，不是本侧的偏离。
    fn apply_theme_command(&mut self, dark: bool, cx: &mut Context<Self>) {
        let id = if dark {
            CommandId::SwitchToDarkTheme
        } else {
            CommandId::SwitchToLightTheme
        };
        let Some(store) = lithe_gpui_settings::try_store(cx) else {
            // 没有设置状态（测试宿主 / 别的宿主）：不装作做成了。
            diagnose_run(id, "unavailable");
            return;
        };
        let settings = store.read(cx).settings().clone();
        // 目标主题取"当前这一支"的既有值：面板在同一支上永远走反方向，所以
        // `Lithe Dark` / `Lithe Light` 这一对名字直接来自设置（默认值见
        // `settings/src/schema.rs:54,57`），不在这里另写一份字面量。
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

    /// 状态栏左组（前导项）：顺序与内容都照真机，**每次重绘时按当前状态算**。
    ///
    /// 顺序真源：`features/layout/config/item-order.ts:20-32` 的
    /// `FOOTER_LEADING_ITEM_IDS = ["filePath", "branch"]`。
    ///
    /// 两条都与上一轮不同：
    ///
    /// - 第一项（项目名）的图标从写死的 `IconName::FileText` 换成
    ///   [`StatusEntry::with_file_icon`] —— 按**当前活动文件名**查真机默认图标主题
    ///   （`idea-icons`），查不到才落 Lucide。真机这一格就是 `ThemedFileIcon`
    ///   （`file-path-breadcrumb.tsx:196-202`），所以现在与文件树 / 标签条同一套美术。
    ///   没有活动文件时传空名字，落到 `defaultFile`（`idea-text`）。
    /// - 第二项（分支）从构造期读一次改成每帧现读 `.git/HEAD`（[`read_branch`]）。
    ///   ⚠️ 分支名本身与工作区根目录无关的这一层没变：切分支后要等下一次
    ///   `cx.notify()` 才会更新（真机是 git 状态推送）。这是既有取舍，不是本轮引入的。
    fn footer_left(&self, cx: &App) -> Vec<StatusEntry> {
        let project_name: SharedString = self
            .projects
            .first()
            .map(|tab| tab.name().clone())
            .unwrap_or_default();
        let active_file = self.editor.read(cx).active_buffer_name();
        let branch = read_branch(&self.root).unwrap_or_else(|| "—".to_string());

        vec![
            StatusEntry::new(project_name).with_file_icon(&active_file, cx),
            // 真源：`ui-icons/idea/vcs/branch.svg(+_dark)` —— 真机 `GitBranchIcon`。
            StatusEntry::new(SharedString::from(branch)).with_idea_icon(&idea::GIT_BRANCH_ICON, cx),
        ]
    }

    /// 状态栏右组（尾随项）：顺序与内容都照真机，**每次重绘时按当前状态算**。
    ///
    /// 顺序真源：`features/layout/config/item-order.ts:20-32` 的
    /// `FOOTER_TRAILING_ITEM_IDS = ["cursor", "encoding", "indent", "readOnly", "memory", "gitChanges"]`。
    ///
    /// 与上一轮的差别：光标位置原来是写死的 `"1:1"`，现在是**真实值**
    /// （[`EditorPane::cursor_position`]）；编码与缩进这两格按当前编辑器/读盘口径核过，
    /// 写出来的就是真值（理由见下面各自的注释），不再是"随手填的占位"。
    /// 其余四项（只读态 / 内存 / 更改数）仍是占位值，理由各自写在旁边。
    fn footer_right(&self, cx: &App) -> Vec<StatusEntry> {
        // 没有任何 buffer 时编辑区给 `1:1`（真机同样：拿不到位置时用
        // `INITIAL_CURSOR_POSITION = {0,0}`，显示成 `1:1`）。
        let cursor = self.editor.read(cx).cursor_position(cx);

        vec![
            // 光标位置 chip。真机这一颗是**可点**的（点开变成输入框、回车跳行，
            // `features/editor/components/toolbar/editor-status-actions.tsx:29-93`），
            // 需要 `on_click` + 编辑态，见 `status_bar.rs` 的「未实现」清单第 3 条。
            StatusEntry::new(SharedString::from(format!(
                "{}:{}",
                cursor.line, cursor.column
            ))),
            // 编码：**真实值**就是 UTF-8 —— 编辑区读盘时把所有内容读成 UTF-8 字符串
            // （`buffer.rs::read_body`），保存也按 UTF-8 写回。`UTF-8` 是编码名而不是界面
            // 文案（真机这一格同样直接显示编码名，`footer-editor-status.tsx`），所以不走 i18n。
            // ⚠️ 非 UTF-8 的文件本轮**不打开成可写**（`read_body` 标成只读），所以不会出现
            // "显示 UTF-8 但保存时按别的编码写回"的不一致。
            StatusEntry::new(SharedString::from("UTF-8")),
            // 缩进：编辑器的制表宽度**固定 2**。真机取的是编辑器的 `tabSize`
            // （默认 2，`features/settings/config/default-settings.ts` 的 editor.tabSize），
            // 而 gpui-kit 的 `EditorState` 不暴露 `tab_size` 读接口（`TabSize` 只有写动作），
            // 本轮也没有"按语言取缩进"的语言配置，所以这里就是它真实的取值。
            // 文案形如 `{count} 个空格`（`i18n/locale.ts` 的 `footer.spaces`），默认 2。
            StatusEntry::new(lithe_gpui_shared::tr_args(
                "lithe.footer.spaces",
                &[("count", "2")],
            )),
            // ⚠️ 只读态：本轮所有可写 buffer 都是可编辑的，占位值仍是"只读"语义的
            // 闭锁字形。要变成真实值需要 `EditorState::is_readonly()` 接进来
            // （真源 `footer-editor-status.tsx:102` 用 `LockIcon` / `LockOpenIcon` 区分），
            // 属于阶段 9 范围外的状态栏打磨。
            // 真源：`ui-icons/idea/expui/general/locked.svg(+_dark)` —— 真机 `LockIcon`。
            StatusEntry::new(SharedString::from("")).with_idea_icon(&idea::LOCK_ICON, cx),
            // ⚠️ 内存条目：真值来自 10s 轮询的原生命令 `get_application_memory_usage`
            // （`footer-editor-status.tsx:24,45-67`），属数据层，本阶段不做。
            StatusEntry::new(SharedString::from("总计 0.0 MB · Lithe 0.0 MB"))
                .with_icon(IconName::HardDrive),
            // ⚠️ Git 更改数：要等 `git.*` 提供工作区状态，本轮仍是空占位（只有图标）。
            // 真源：`ui-icons/idea/expui/general/successDialog.svg(+_dark)` —— 真机 `CheckCircleIcon`。
            StatusEntry::new(SharedString::from("")).with_idea_icon(&idea::CHECK_CIRCLE_ICON, cx),
        ]
    }

    /// 左活动栏第 `index` 项是否画选中底色。
    ///
    /// 两组独立（契约见 [`crate::activity_bar`] 的模块文档「选中契约：同时可以亮多项」）：
    ///
    /// - **顶部组**（0..=2）：选中态是 [`ShellWorkspace::top_activity_view`] 那一个，
    ///   与真机 `activeSidebarView`（`sidebar-pane-selector.tsx:234-247,311-319`）对应；
    /// - **底部组**（3..=6）：选中态是 `bottom_visible && bottom_kind == 该项的 kind`，
    ///   与真机 `isBottomPaneVisible && bottomPaneActiveTab === "<该项>"`
    ///   （`main-sidebar.tsx:643-652`）逐条对应。**判据不是活动栏的选中项**，所以隐藏底部窗后
    ///   底部组一项都不亮，而顶部组不受影响；
    /// - 「设置」（7）：不是视图，永远不亮（点它开对话框，见 [`SETTINGS_ACTIVITY_IX`]）。
    ///
    /// 右活动栏的选中态**不在这个函数里**：它由 [`ShellWorkspace::is_right_activity_active`]
    /// 单独算（两套栏、两套视图集合）。
    fn is_activity_active(&self, index: usize) -> bool {
        if TOP_ACTIVITY_ITEMS.contains(&index) {
            return self.top_activity_view == Some(index);
        }
        match bottom_pane_for(index) {
            Some(kind) => self.bottom_visible && self.bottom_kind == kind,
            None => false,
        }
    }

    /// 右活动栏第 `index` 项是否画选中底色。
    ///
    /// 判据照真机 `plugin-activity-rail.tsx:24-26`（Maven）与
    /// `notifications-trigger.tsx:18-21`（通知）：
    /// `isRightSidebarVisible && activeRightSidebarView === "<该项>"`。
    /// 扩展项的真机判据是「扩展缓冲区是不是当前 buffer」（`plugin-activity-rail.tsx:14-20`），
    /// 本侧按本轮口径换成同一条右栏判据（见 [`crate::right_tool_window`] 的偏离说明）。
    ///
    /// 推论（都是有意行为）：**面板收起时三项都不亮**；点另一项时旧项立刻灭、新项亮。
    fn is_right_activity_active(&self, index: usize) -> bool {
        match RightToolWindowView::from_rail_index(index) {
            Some(view) => self.right_visible && self.right_view == view,
            None => false,
        }
    }
}

impl Render for ShellWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // ⚠️ 顺序不可调换：这三个 `render_*_layer` 借用 `cx`，必须先把值取出来，
        // 再调用任何其它借用 `cx` 的方法（`cx.theme()` 等）。
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let sheet_layer = Root::render_sheet_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        // 主菜单：先画菜单栏（它按"上一帧结束时的状态"画），再把用户上一帧点下的动作**真的执行掉**。
        //
        // 顺序为什么是"先画再执行"而不是反过来：画菜单栏要 `cx` 的不可变借用（读菜单栏实体），
        // 而执行动作要 `&mut self`；两者在同一段代码里同时成立会让借用检查失败。
        // 动作改动的是 `self` 的字段（`bottom_visible` / `right_visible` …），本帧的布局
        // 紧接着按新值画，所以用户看不到"慢一帧"。
        let menu = crate::menu_bar::render_bar(&self.menu_bar, window, cx);
        // 标题栏的项目下拉（同一段：先渲染，动作在下一帧被菜单项写进队列后由本帧收尾执行）。
        let project_entries = self.project_entries();
        let project_menu = render_project_menu(&self.project_menu, &project_entries, window, cx);
        // 标题栏的分支项（Windows 规格，见 `crate::branch_panel`）：
        // 没有仓库 / 没有分支名时真源整项不渲染（研究 §5），所以这里给 `None` →
        // 调用方传一块空 `div`，拖拽区宽度不受影响。
        let branch_item = branch_trigger(&self.branch_panel, cx)
            .unwrap_or_else(|| div().into_any_element());
        // `--branch-panel-probe`：**到这一帧才开**（`Root` 已就位，浮层只能在事件回调 /
        // 任务 / 首帧之后的 render 里开）。只消费一次，之后置回 `false` ——
        // 否则用户关掉面板后会在下一帧被重新打开。
        //
        // ⚠️ 位置很讲究：gpui 的区域渲染函数（本文件后面那些 `activity_bar` / `side_pane` /
        // `title_bar`）在 edition 2024 下都是 `-> impl IntoElement`，**捕获传入的
        // `&Window` / `&App` 生命周期**（`lib.rs` 模块头的"区域渲染函数必须收 `&Window` /
        // `&App`"）。只要把它们的结果绑进局部变量并且活到本函数末尾，`window` / `cx` 就被
        // 不可变借用到最后，再想 `&mut` 开浮层就报 E0502（本轮实测）。所以这一段必须放在
        // 第一个区域渲染函数**之前**。
        let probe = self.branch_panel_probe;
        self.branch_panel_probe = false;
        if probe {
            open_branch_panel(&self.branch_panel, window, cx);
        }
        for action in crate::menu_bar::take_pending_runs(cx) {
            self.apply_menu_action(action, window, cx);
        }

        // 设置项「显示状态栏」。没有设置状态时按默认值（显示）处理：工作台不应该因为
        // 宿主没装设置而少一块 UI。
        let show_status_bar = lithe_gpui_settings::try_store(cx)
            .map(|store| store.read(cx).settings().show_status_bar)
            .unwrap_or(true);

        // 回调不能借用 `self`（它们要 `'static`），所以通过实体句柄改自己的状态。
        let handle = cx.entity();
        let on_activate = {
            let handle = handle.clone();
            move |index: usize, _window: &mut Window, cx: &mut App| {
                handle.update(cx, |this, cx| {
                    this.active_project = Some(index);
                    cx.notify();
                });
            }
        };
        let on_close = {
            let handle = handle.clone();
            move |index: usize, _window: &mut Window, cx: &mut App| {
                // 关闭项目要销毁项目窗口 / 落盘项目列表，属项目生命周期，本轮不做。
                // 这里只把"关掉当前项目"表现为取消选中，避免出现点了没反应的按钮。
                handle.update(cx, |this, cx| {
                    if this.active_project == Some(index) {
                        this.active_project = this.projects.len().checked_sub(2);
                        cx.notify();
                    }
                });
            }
        };
        let terminal_handle = self.terminal.downgrade();
        // 左栏「更改」视图的弱引用：切到该项时刷新一次（本侧没有 watcher，
        // 「激活时刷新 + 手动刷新 + 写后刷新」是仅有的三条通路，见 `ChangesView::activate`）。
        let changes_handle = self.changes.downgrade();
        let on_select_activity = {
            let handle = handle.clone();
            move |index: usize, window: &mut Window, cx: &mut App| {
                // 「设置」打开的是模态对话框（真机如此），**不改**活动栏选中态、
                // 也不换底部窗：点它只是把对话框打开。
                if index == SETTINGS_ACTIVITY_IX {
                    lithe_gpui_settings::open_settings_dialog(window, cx);
                    return;
                }
                // 终端会话的懒创建：只有"这一次点完底部窗显示终端"才去建会话。判据与
                // `is_activity_active` 完全一致（可见 && 当前页签 == 该项的 kind）。
                let ensure_terminal = handle.update(cx, |this, cx| {
                    match bottom_pane_for(index) {
                        // 底部组：切底部窗内容；已经是它**且可见** → 再点一次收起。
                        //
                        // ⚠️ 判据是 `bottom_visible && bottom_kind == kind`，**不是**
                        // "活动栏这一项之前是不是选中项"：活动栏底部组的高亮本来就由这个表达式
                        // 算出来，两者必须同源（真机 `view-command-actions.ts:45-55`、
                        // `main-sidebar.tsx:643-652`）。
                        Some(kind) => {
                            if this.bottom_visible && this.bottom_kind == kind {
                                this.bottom_visible = false;
                            } else {
                                this.bottom_kind = kind;
                                this.bottom_visible = true;
                            }
                        }
                        // 顶部组（项目 / 更改 / 搜索）不换底部窗，也不碰它的可见性：只记下
                        // 「顶部组选中谁」。底部高亮由 `bottom_visible` / `bottom_kind` 自己决定，
                        // 所以两组可以同时亮（真机 `activeSidebarView` 与 `isBottomPaneVisible`
                        // 就是两个独立状态，`workspace-ui-defaults.ts:5-7`）。
                        None => this.top_activity_view = Some(index),
                    }
                    cx.notify();
                    this.bottom_visible && this.bottom_kind == BottomPaneKind::Terminal
                });
                if ensure_terminal {
                    // 幂等：会话已存在就什么都不做（所以"再点一次隐藏、再点回来"不会重开 shell）。
                    // 首次创建时由它自己把焦点延到帧末交给输入行。
                    let _ = terminal_handle.update(cx, |pane, cx| pane.ensure_session(window, cx));
                }
                // 顶部组点到「更改」：左栏内容换人（判据同 `is_activity_active`）之外，
                // 顺手让那个视图重读一次工作区状态 —— 真源也是"视图重新可见就后台整刷一次"
                // （`use-git-data-controller.ts:276-281`），本侧没有文件监听，这一步是兜底。
                if bottom_pane_for(index).is_none() && index == CHANGES_ACTIVITY_IX {
                    let _ = changes_handle.update(cx, |view, cx| view.activate(cx));
                }
            }
        };

        // 右活动栏的点击处理：**不能**复用左栏那个闭包，因为它的 `index` 是左栏的下标
        // （右栏第 0 项不是「项目」）。三项都落到**右侧工具窗**：真机三项里通知与 Maven
        // 本来就是右栏视图（`plugin-activity-rail.tsx:24-26`、`notifications-trigger.tsx:18-21`），
        // 扩展的真机行为是开编辑器缓冲区（`:46`），本侧按本轮口径统一走右栏（偏离说明见
        // `right_tool_window` 模块文档）。
        let on_select_right_activity = {
            let handle = handle.clone();
            move |index: usize, _window: &mut Window, cx: &mut App| {
                // 越界下标什么也不做：右栏只有三项，多出来的下标不该静默落到某一项上。
                let Some(clicked) = RightToolWindowView::from_rail_index(index) else {
                    return;
                };
                handle.update(cx, |this, cx| {
                    // 状态迁移是纯函数（[`resolve_right_click`]），与真机
                    // `resolveRightToolWindowUpdate` 逐条对应，单测直接覆盖它。
                    let (view, visible) =
                        resolve_right_click(clicked, this.right_view, this.right_visible);
                    this.right_view = view;
                    this.right_visible = visible;
                    // 第一次真正显示 Maven 面板时才扫项目（懒扫 + 缓存，理由见字段文档）。
                    if visible && view == RightToolWindowView::Maven && !this.maven_scanned {
                        this.maven_scanned = true;
                        this.maven_project = crate::maven::scan(&this.root);
                    }
                    diagnose_right_panel(view, visible);
                    cx.notify();
                });
            }
        };

        // 面板头部的关闭按钮：只关可见性，**不动** `right_view`（真机 toggle 分支同理）。
        let on_close_right_activity = {
            let handle = handle.clone();
            move |_: &ClickEvent, _window: &mut Window, cx: &mut App| {
                handle.update(cx, |this, cx| {
                    this.right_visible = false;
                    diagnose_right_panel(this.right_view, false);
                    cx.notify();
                });
            }
        };

        // ⚠️ 标题栏的项目名**不再在这里取一份**：它由 `project_entries()` 一起算出来，
        // 画在项目下拉的触发器里（真源 `title-project-menu.tsx:157`），拖拽区是空白。

        // 选中态先算成一份**不借用 `self`** 的小表：`activity_bar` 要求 `is_active` 是 `'static`，
        // 闭包直接捕获 `self` 会被判成 `E0521 borrowed data escapes outside of method`。
        // 每次重绘克隆一份 bool 换来闭包只捕获自己那份 Vec，读数与判据仍只有一个来源。
        let activity_flags: Vec<bool> = (0..self.activity_items.len())
            .map(|index| self.is_activity_active(index))
            .collect();
        let right_activity_flags: Vec<bool> = (0..self.right_activity_items.len())
            .map(|index| self.is_right_activity_active(index))
            .collect();

        let left_rail = activity_bar(
            ActivitySide::Left,
            &self.activity_items,
            // 逐项谓词：顶部组与底部组各自独立，可以同时亮（契约见 `activity_bar` 模块文档）。
            move |index| activity_flags.get(index).copied().unwrap_or(false),
            on_select_activity,
            window,
            cx,
        );
        let right_rail = activity_bar(
            ActivitySide::Right,
            &self.right_activity_items,
            // 右栏选中态 = `right_visible && right_view == 该项`，所以收起时三项全灭。
            move |index| right_activity_flags.get(index).copied().unwrap_or(false),
            on_select_right_activity,
            window,
            cx,
        );

        let explorer = self.explorer.clone();
        let editor = self.editor.clone();
        // 左栏槽位的内容：顶部组选中项决定画「项目树」还是「更改列表」（真源
        // `main-sidebar.tsx:777-816` 的 `activePaneId` 单选；本侧的下标来源是
        // `top_activity_view`，与活动栏高亮同源 —— 两处不同源会出现"图标亮着但内容不对"）。
        // 两个实体都常驻，切换只换渲染谁，不销毁对方的状态。
        let left_content: AnyElement = if self.top_activity_view == Some(CHANGES_ACTIVITY_IX) {
            self.changes.clone().into_any_element()
        } else {
            explorer.into_any_element()
        };
        let right_tool_window = right_tool_window(
            self.right_view,
            self.maven_project.as_ref(),
            on_close_right_activity,
            cx,
        );
        let right_tool_window_visible = self.right_visible;

        // 底部工具窗的内容由活动栏 / 命令面板切换的单值 `bottomPaneActiveTab` 决定。
        //
        // ⚠️ Git 工具窗的标题栏自带「隐藏」按钮（内部调 `BottomPane::set_visible(false)`），
        // 所以那个窗格的可见性**以它自己为准**。不读它就会出现"外框还在、内容已经空了"。
        let pane_self_hidden =
            self.bottom_kind == BottomPaneKind::Git && !self.bottom_git.read(cx).visible();
        let bottom = (self.bottom_visible && !pane_self_hidden).then(|| {
            let content: AnyElement = match self.bottom_kind {
                BottomPaneKind::Terminal => self.terminal.clone().into_any_element(),
                BottomPaneKind::Git => self.bottom_git.clone().into_any_element(),
                kind @ (BottomPaneKind::Run | BottomPaneKind::Diagnostics) => {
                    // 占位文案是临时脚手架（阶段 6 第二半会换成真界面）；里面的 {label} 是活动栏
                    // 项的名字，走 i18n（`lithe.workbench.run` / `.diagnostics`）。
                    placeholder(format!("{} 工具窗（未实现）", kind.label()), cx).into_any_element()
                }
            };
            bottom_pane(content, cx)
        });

        // ① 标题栏 40（含自绘窗口三键 56×40）+ 主菜单栏。
        //
        // 菜单栏与 `drag_region` 是**兄弟节点**（菜单栏由 `title_bar` 插在拖拽区之前）：
        // 祖先的 `Drag` 会赢下 Windows 的命中测试，把菜单放进拖拽区就会变成"点菜单只拖窗口"
        // （理由与源码行号见 `crate::title_bar` 的 `title_bar` 文档）。
        //
        // ⚠️ `cx.lease()` 是必须的：`menu_bar()` 要 `&mut App`（它要新造 `PopupMenu`
        // 实体），而 `cx` 在同一帧里还要用来画后面的区域；`lease` 把这一帧的 `&mut App`
        // 借出来，避免 "cannot borrow `*cx` as mutable more than once"。
        // ⚠️ 菜单栏元素必须在**这一帧的布局链**里，所以它由上面算好的 `menu` 给；
        // `render_bar` 拿不到时会返回 `None`（窗口正在关），那时标题栏里就没有菜单栏
        // （`Some` 恒为真：`ShellWorkspace::new` 建视图时就把句柄登记好了），
        // 其余部分照画。
        // `--branch-panel-probe` 与「待执行菜单动作」都在上面消费掉了（理由见那一段：
        // 它们要 `&mut Window` / `&mut App`，必须早于第一个区域渲染函数）。

        let root = v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            // 兜底焦点锚点：没有后代元素持有焦点时，键盘事件落在这里，
            // 全局 action（`Ctrl+S` / `Ctrl+Shift+P` / `Ctrl+,`）才派发得出去。
            // 理由与实测见结构体字段 `focus` 与 `crate::command_palette` 的 `SHELL_FOCUS`。
            .track_focus(&self.focus)
            // `Ctrl+S`：绑定登记在 `lithe_gpui_editor::install_actions`，动作转发给编辑区。
            // 放在**外壳根元素**而不是编辑区根元素：焦点落在资源管理器 / 终端 / 标签栏时
            // `Ctrl+S` 也应该保存当前文件，而它们在元素树里都是这个根元素的后代
            // （gpui 的 action 冒泡沿"焦点节点 → 祖先"走，`gpui-pre-0.3.6/src/window.rs:6333`）。
            // 唯一够不到的情形是"窗口里一个焦点元素都没有"（启动后没点过任何地方），
            // 那时也没有可保存的内容。
            .on_action(cx.listener(|this, _: &SaveBuffer, window, cx| {
                let _ = this
                    .editor
                    .update(cx, |pane, cx| pane.save_active(window, cx));
            }))
            // ① 标题栏 40（含自绘窗口三键 56×40）+ 主菜单栏。
            //
            // 菜单栏与 `drag_region` 是**兄弟节点**（菜单栏由 `title_bar` 插在拖拽区之前）：
            // 祖先的 `Drag` 会赢下 Windows 的命中测试，把菜单放进拖拽区就会变成"点菜单只拖窗口"
            // （理由与源码行号见 `crate::title_bar` 的 `title_bar` 文档）。
            //
            // ⚠️ `cx.lease()` 是必须的：`menu_bar()` 要 `&mut App`（它要新造 `PopupMenu`
            // 实体），而 `cx` 在同一帧里还要用来画后面的区域；`lease` 把这一帧的 `&mut App`
            // 借出来，避免 "cannot borrow `*cx` as mutable more than once"。
            // ⚠️ 菜单栏元素必须在**这一帧的布局链**里，所以它由上面算好的 `menu` 给；
            // `render_bar` 拿不到时会返回 `None`（窗口正在关），那时标题栏里就没有菜单栏
            // （`Some` 恒为真：`ShellWorkspace::new` 建视图时就把句柄登记好了），
            // 其余部分照画。
            .child(title_bar(
                menu.unwrap_or_else(|| div().into_any_element()),
                project_menu.unwrap_or_else(|| div().into_any_element()),
                branch_item,
                window,
                cx,
            ))
            // ② 项目标签条 32 —— **只有一个项目时整条隐藏**。
            // 真机规则：`count > 0 && (!hideWhenSingle || count > 1)`
            // （`features/window/components/project-tab-bar/utils/project-tab-bar-model.ts:12-13`，
            // `main-layout.tsx:293` 传 `hideWhenSingle`）。
            // 2026-09-25 与真机并排对照时确认：单个项目时看不到这条标签条。
            .children((self.projects.len() > 1).then(|| {
                project_tabs(
                    &self.projects,
                    self.active_project,
                    on_activate,
                    on_close,
                    window,
                    cx,
                )
            }))
            // ③ 工作区：左右活动栏 + 左右面板 + 中央列，间隔 4。
            .child(
                h_flex()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .gap_1()
                    // `main-layout.tsx:299` 的 `pr-(--lithe-workbench-gap)`：右端再留 4，
                    // 否则右活动栏会贴到窗口边缘。
                    .pr_1()
                    .child(left_rail)
                    .child(side_pane(div().w_80(), left_content, cx))
                    .child(
                        // 中央列 = 编辑器岛 + 底部工具窗（默认口径：嵌在中央列内）。
                        v_flex()
                            .flex_1()
                            .h_full()
                            .min_w_0()
                            .child(editor_island(editor, cx))
                            .children(bottom),
                    )
                    // 右工具窗 400 —— **可收起**：真机整块 `ResizablePane` 的 `hidden` 由
                    // `isRightToolWindowVisible`（= 通知或 Maven 可见）决定，
                    // 收起时这一栏完全不占位（`main-layout.tsx:101-105,328`）。
                    .children(right_tool_window_visible.then(|| {
                        side_pane(
                            div().w(rems(RIGHT_TOOL_WINDOW_WIDTH / 16.)),
                            right_tool_window,
                            cx,
                        )
                    }))
                    .child(right_rail),
            )
            // ④ 状态栏 24 —— 设置里的「显示状态栏」关掉时整条不渲染（真机是根元素上的
            // `data-status-bar` + CSS，见 `07-settings-ui.md` §4.5 的 `lib/ui-preferences.ts:5-11`）。
            // 尾随组每次重绘都按当前光标/活动 buffer 重算，所以用局部变量接一下返回值。
            .children(show_status_bar.then(|| {
                let left = self.footer_left(cx);
                let right = self.footer_right(cx);
                status_bar(&left, &right, window, cx)
            }))
            // 浮层三层必须挂在最外层视图上，否则对话框 / 抽屉 / 通知静默不显示。
            .children(dialog_layer)
            .children(sheet_layer)
            .children(notification_layer);

        root
    }
}

/// 左右两侧的面板外壳：固定宽、不参与收缩、`bg-background`、圆角 11.2 + 1px 边框。
///
/// Windows 的左右面板是 `lithe-glass-island rounded-xl border-border`（`resizable-pane.tsx:203-206`）。
/// **拖拽改宽还没接**（`ResizablePane` 的 4px 热区）。
///
/// `outer` 由调用方给：宽度是布局度量，调用点用 gpui 的 rem 档位 helper（`w_80()`）或
/// 档位外的 `rems(P / 16.)` 各自表达，这里只负责外壳剩下的部分。
fn side_pane(outer: Div, content: impl IntoElement, cx: &App) -> impl IntoElement {
    outer
        .h_full()
        .flex_shrink_0()
        .rounded(ISLAND_RADIUS)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .child(content)
}

/// 中央编辑器岛：`rounded-xl border-l bg-background`（`main-layout.tsx:313`）。
fn editor_island(content: impl IntoElement, cx: &App) -> impl IntoElement {
    div()
        .w_full()
        .flex_1()
        .min_h_0()
        .rounded(ISLAND_RADIUS)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .child(content)
}

/// 底部工具窗外框：默认 **320** 高，上面一条 **4px** 拖拽热区
/// （`bottom-pane/bottom-pane.tsx:47,222-232`）。
///
/// 内容由调用方给（终端 / Git 提交记录 / 占位），与真机"底部窗只有一个、内容由
/// `bottomPaneActiveTab` 切换"的结构一致。
fn bottom_pane(content: AnyElement, cx: &App) -> impl IntoElement {
    v_flex()
        .w_full()
        .flex_shrink_0()
        .child(
            // 拖拽热区：4px，悬停时中间那条 1px 主色线（`bottom-pane.tsx:226-231`）。
            div().w_full().h_1(),
        )
        .child(
            div()
                .w_full()
                .h_80()
                .rounded(ISLAND_RADIUS)
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().background)
                .child(content),
        )
}

/// 区域占位块的通用外观：弱化文字 + 内边距，用来核对位置与尺寸。
///
/// ⚠️ 收 `impl Into<SharedString>` 而**不是** `&str`：本 crate 是 edition 2024，`-> impl IntoElement`
/// 会捕获参数的生命周期，收 `&str` 时传一个临时 `String`（`&format!(..)`）会报
/// `E0716 temporary value dropped while borrowed`。
fn placeholder(text: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    div()
        .p_2()
        // 字号 `--ui-text-chrome` 原来是 13px，不在 gpui 的档位（12 / 14）上；
        // 按《编码指南》用 `text_sm()`（14px）——13 → 14 是经维护者确认的有意改动。
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text.into())
}

/// 读当前分支名。
///
/// ⚠️ **临时实现**：直接读 `.git/HEAD`。阶段 4 接 `git.status` 后应删掉
/// （`git.status` 才是在 Core 里解析 worktree / 分离 HEAD / 符号链接的那一份）。
fn read_branch(root: &std::path::Path) -> Option<String> {
    let head = std::fs::read_to_string(root.join(".git/HEAD")).ok()?;
    let head = head.trim().to_string();
    head.strip_prefix("ref: refs/heads/").map(str::to_string)
}

/// 左侧活动栏的图标项。
///
/// 图标来源逐项见 `activity_bar.rs` 模块文档的对照表：**有真源 SVG 的用
/// `gpui/assets/ui-icons/idea/**`（`ActivityItem::idea`），真源只有内联 React 组件或
/// 本来就是 Lucide 的保持 `IconName`（`ActivityItem::new`）**。
///
/// ⚠️ **左栏没有 Maven 项**（阶段 6 第一半改）：真机的左栏底部组第一项是 maven
/// （`features/layout/config/item-order.ts:12-19` 的
/// `SIDEBAR_BOTTOM_ACTIVITY_ITEM_IDS = [maven, run, terminal, diagnostics, gitLog, settings]`），
/// 但它点下去切的是**右侧栏**（`sidebar` 的 `toggleMavenPane` →
/// `features/maven/actions/maven-tool-window-actions.ts:58` → `applyRightToolWindowIntent`）。
/// 本侧已经有一个右栏 Maven 入口（[`right_activity_items`]），再留一个就是"两处都能开 Maven"，
/// 所以按阶段 6 的口径把左栏那一项删掉，Maven 只归右栏。底部组因此从 6 项变成 5 项，
/// 相关下标（[`bottom_pane_for`] / [`SETTINGS_ACTIVITY_IX`]）同步下移。
fn activity_items() -> Vec<ActivityItem> {
    // 顶部组：`sidebar-pane-selector.tsx:311-319` 的 files / git / search。
    // 底部组：上面的 `SIDEBAR_BOTTOM_ACTIVITY_ITEM_IDS` 去掉 maven。
    vec![
        // 真源：`ui-icons/idea/expui/general/listFiles.svg(+_dark)` —— 真机 `FilesIcon`
        // 的同一个文件（`windows/tauri/src/ui/icons/idea-assets.generated.ts:FilesIcon`）。
        ActivityItem::idea(&idea::FILES_ICON, tr("lithe.workbench.project")),
        // 真源：`ui-icons/idea/vcs/branch.svg(+_dark)` —— 真机 `GitBranchIcon`。
        ActivityItem::idea(&idea::GIT_BRANCH_ICON, tr("lithe.workbench.changes")),
        // 真源：`ui-icons/idea/expui/general/search.svg(+_dark)` —— 真机 `MagnifyingGlassIcon`。
        ActivityItem::idea(&idea::MAGNIFYING_GLASS_ICON, tr("lithe.workbench.search")),
        // ⚠️ 保持 Lucide：真机 `RunIcon`（`features/run/components/run-icon.tsx`）是**内联
        // React 组件**，只有一条 stroke path、没有 SVG 文件，搬不动（见
        // `gpui/research/icon-asset-inventory.md` 第 4 节）。
        ActivityItem::new(IconName::Play, tr("lithe.workbench.run")).bottom(true),
        // ⚠️ 保持 Lucide：真机 `TerminalWindowIcon` 走 `Nucleo.IconSquareTerminalOutline18`
        // → `lucide-react` 的 `square-terminal`，**本来就是 Lucide，已经是 1:1**。
        ActivityItem::new(IconName::SquareTerminal, tr("lithe.workbench.terminal")).bottom(true),
        // 真源：`ui-icons/idea/expui/general/warningDialog.svg(+_dark)` —— 真机 `WarningIcon`。
        ActivityItem::idea(&idea::WARNING_CIRCLE_ICON, tr("lithe.workbench.diagnostics"))
            .bottom(true),
        // ⚠️ 保持 Lucide：真机 `GitGraphIcon` 走 `Nucleo.IconGitGraphOutline18` → Lucide
        // `git-graph`，同样是 1:1。
        ActivityItem::new(IconName::GitGraph, tr("lithe.workbench.gitLog")).bottom(true),
        // 真源：`ui-icons/idea/expui/general/settings.svg(+_dark)` —— 真机 `GearIcon`
        // （与 `GearSixIcon` 同一张图，见生成器的别名表）。
        ActivityItem::idea(&idea::GEAR_ICON, tr("lithe.workbench.settings")).bottom(true),
    ]
}

/// 右侧活动栏的图标项。
///
/// 真源：`features/layout/components/plugin-activity-rail.tsx:31-67`。右栏与左栏是**两套不同**的
/// 视图集合——左栏是 `SIDEBAR_ACTIVITY_ITEM_IDS`（项目 / 更改 / 搜索 / 运行 / 终端 /
/// 诊断 / 提交记录 / 设置），右栏只有三个：扩展（`PuzzlePieceIcon`）、通知
/// （`NotificationsTrigger` 的铃铛，带未读徽标）、Maven（`MavenIcon`）。
///
/// 顺序必须与 [`RightToolWindowView::from_rail_index`] 的下标一致（0 扩展 / 1 通知 / 2 Maven）。
///
/// Windows 只在探测到 Maven 项目时才渲染 Maven 按钮（`plugin-activity-rail.tsx:21-23,51` 的
/// `isMavenAvailable`），我们还没有 Maven 项目探测，先固定渲染三项；Maven 视图的空态
/// 如实写"未检测到 Maven 项目"（`lithe.maven.notDetected`）。
/// 右栏三项都没有 `bottom` 分组（Windows 的右栏是单列）。
fn right_activity_items() -> Vec<ActivityItem> {
    vec![
        // ⚠️ 保持 Lucide：真机 `PuzzlePieceIcon` 经 `Nucleo` 代理解析到
        // `legacyIconCompatibility.PuzzlePiece = lucideIcons.Puzzle`，**真源物就是 Lucide
        // `puzzle`**，没有 SVG 文件可搬；gpui 用的 `IconName::Puzzle` 是同一个字形。
        ActivityItem::new(IconName::Puzzle, tr("lithe.extensions.title")),
        // 真源：`ui-icons/idea/expui/toolwindows/notifications.svg(+_dark)` —— 真机
        // `BellIcon`（`features/notifications/components/notifications-trigger.tsx:43`）。
        ActivityItem::idea(&idea::BELL_ICON, tr("lithe.notifications.title")),
        // ⚠️ 保持 Lucide：真机 `MavenIcon`（`features/maven/components/maven-icon.tsx`）是
        // **内联 React 组件**，只有一条 fill path、没有 SVG 文件。Lucide 没有 Maven 字形，
        // 与 `activity_bar.rs` 的对照表同一取舍：取「包 / 构建产物」语义的 `package`。
        // （注意：Maven **文件类型**图标是有真源的 —— `icon-themes/idea` 的
        // `pom.xml` → `icons/expui/fileTypes/maven.svg`，所以文件树里 `pom.xml` 是真源。）
        ActivityItem::new(IconName::Package, tr("lithe.workbench.maven")),
    ]
}

#[cfg(test)]
mod tests {
    use super::{ActionFlags, CommandId, COMMAND_ORDER, visible_commands};

    /// 设置侧 ↔ `lithe-gpui-git` 的两组枚举映射**双向**都要对（阶段 15）。
    ///
    /// 这两组映射是"Git 身份宿主钩子"唯一的翻译层：错了不会编译失败，只会静默写错作用域
    /// （把 `global` 写成 `local` = **改错文件**），所以必须逐值钉住。
    /// 同时校验 `id()` / `from_id()` 这一对（宿主钩子的线格式就是这两个字面量）。
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

    /// 深色配色下的朝向快照（其余字段取默认可见状态，避免测试里到处写一遍）。
    fn dark() -> ActionFlags {
        ActionFlags {
            terminal_shown: false,
            maven_shown: false,
            status_bar_shown: true,
            dark_theme: true,
        }
    }

    /// 主题那一对**互斥**：面板里永远只有"反方向"那一条。
    ///
    /// 这是"动作表行序"的回归测试：`command_actions`（画）与 `run_command`（执行）都必须
    /// 经过 `visible_commands`，一旦有人只改一边，面板点下去就会错行。
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

    /// 除主题那一条外，其余动作**总是**在表里（它们没有"按了没变化"的可能：
    /// 终端 / Maven / 状态栏都是双向开关，设置对话框总是能打开）。
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

    /// 动作表**没有重复项**，而且是 `COMMAND_ORDER` 的子序列（顺序 = 面板行序）。
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

    /// `CommandId::id()` 是诊断行 `S1_COMMAND_RUN id=…` 的取值，改它就是改可 grep 的契约；
    /// 同时它也是搜索关键词，所以要稳定且唯一。
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
}
