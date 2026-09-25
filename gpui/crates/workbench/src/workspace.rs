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
//!
//! ## 「打开其他文件夹」（换项目）：**重建整个 `ShellWorkspace`**，不逐个 reset（B4）
//!
//! 入口有三个，都落到同一个执行点：
//!
//! | 入口 | 走哪条 |
//! | --- | --- |
//! | 文件菜单「打开文件夹」（+ `Ctrl+O`） | [`MenuAction::OpenFolder`] → [`ShellWorkspace::open_project_picker`] |
//! | 标题栏项目下拉「打开…」 | 同一段（面板只负责路由到 `MenuAction` 那条执行分支） |
//! | 项目下拉「最近项目」某一行 | [`ShellWorkspace::request_open_project`]（路径已知，跳过选择器） |
//!
//! 选中文件夹之后照真源的 `chooseProjectOpenDestination` 决定"在哪儿打开"
//! （判据抽成纯函数 [`resolve_project_open_destination`]，可单测）：需要询问时弹
//! [`ShellWorkspace::open_project_where_dialog`]，选「此窗口」则
//! [`ShellWorkspace::rebuild_project_window`] —— `window.replace_root` + 新的 `Root`。
//!
//! ⚠️ **为什么不能逐个 reset**（两条硬证据，不是取舍）：
//!
//! 1. `EditorPane::prepare_java` 是**幂等早退**（`if self.java.is_some() { return; }`，
//!    `editor/src/editor_view.rs:436-438`）—— 同一个编辑区**永远换不了 JDTLS 工作区**；
//! 2. 根在**构造期**被烘进 6 个实体 + 1 个跨 crate 钩子（`Explorer` / `ChangesView` /
//!    `BottomPane` / `BranchPanel` / `EditorPane` 的 `workspace_root` / `GIT_IDENTITY_HOST`），
//!    它们**都没有** `set_root` 之类的重指入口。
//!
//! 换根的连带释放（JDTLS 会话、终端进程）**不需要**显式 `shutdown`：旧 `Root` 在外层
//! `App::update` 收尾的 `flush_effects` 里级联 drop（侦察 `gpui/research/menu-open-prereqs.md`
//! §2），`EditorPane::drop` / `TerminalPane::drop` 各自收尾。⚠️ 但 `java::Session` 的 `Drop`
//! **只停事件泵、不关 JVM** —— 所以换根后"无残留进程"必须**实测**（B4 验收线），
//! 不能只看代码路径。

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::{ActiveTheme as _, Root, WindowExt as _};
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Div, Entity, Global,
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, PathPromptOptions,
    Pixels, Render, SharedString, Styled as _, Window, div, px, rems,
};

use lithe_gpui_editor::{EditorPane, SaveBuffer, TabMenuHostActions};
use lithe_gpui_explorer::Explorer;
use lithe_gpui_git::{BottomPane, ChangesView};
use lithe_gpui_settings::{
    Category as SettingsCategory, GitIdentityHost, IdentityField, IdentityScope, RecentProjects,
};
use lithe_gpui_shared::icons::idea;
use lithe_gpui_shared::{tr, tr_args};
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
    ProbePoint, RightToolWindowView, diagnose as diagnose_right_panel,
    resolve_click as resolve_right_click, right_tool_window,
};
use crate::status_bar::{StatusEntry, status_bar};
use crate::title_bar::title_bar;

// ---------------------------------------------------------------------------
// 启动期形态（`App::Global`）+ 换项目决策（纯函数）
// ---------------------------------------------------------------------------

/// 启动参数里"决定外壳形态"的那几项。
///
/// ## 为什么必须是 `Global`（B4 的硬需求）
///
/// 换项目要**重建整个 `ShellWorkspace`**（见模块头的"不能逐个 reset"），而重建发生在
/// `window.replace_root` 的闭包里 —— 那里离 `main.rs` 的启动参数（`compact_menu_bar`）
/// 有十几层调用，**捕获不到局部变量**。
///
/// 本仓库唯一的同类先例是设置句柄 `SettingsHandle`
/// （`settings/src/store.rs:105,171` 的 `impl Global` / `set_global`），这里照它写。
/// ⚠️ 这是**应用级**值（一次启动一份，与"哪个窗口"无关），所以放 `Global` 是对的
/// —— 与 `crate::command_palette::SHELL` 那种"窗口级句柄"不是一类东西（B5 才需要拆那些）。
pub struct ShellStartup {
    /// 菜单栏走"左上角图标 + 浮动胶囊"形态（`--compact-menu-bar`）。
    compact_menu: bool,
    /// `--right-view <id>`：本次会话把右工具窗开在这个视图上（**验证/诊断用**）。
    ///
    /// ⚠️ 与 `compact_menu` 一起放 `Global` 有一个额外好处：**换根重建的外壳也会应用它**。
    /// 这正是"换根后右栏 Maven / Spring 面板指向新根"这条验收线需要的 —— 面板不打开就不会
    /// 触发懒扫（`ShellWorkspace::schedule_right_view_scan`），也就没有 `S1_RIGHT_SCAN root=`
    /// 那行证据。
    right_view: Option<RightToolWindowView>,
    /// `--left-view <id>`：本次会话把左栏切到这个视图（**验证/诊断用**，理由同上：
    /// 「更改」不激活就不会重读 `git.status`，拿不到 `S1_SOURCE_CONTROL root=`）。
    left_view: Option<usize>,
}

impl Global for ShellStartup {}

/// 登记启动期形态（`main.rs` 在建窗口之前调一次）。
///
/// 不登记时 [`ShellWorkspace::new`] 的调用方仍然显式传 `compact_menu`，重建走
/// [`startup_compact_menu`] 的默认值 `false`（常驻形态，本侧默认）——
/// 也就是"没登记 = 用默认形态"，而不是 panic。
pub fn set_shell_startup(
    cx: &mut App,
    compact_menu: bool,
    right_view: Option<RightToolWindowView>,
    left_view: Option<usize>,
) {
    println!(
        "S1_WORKSPACE startup compact_menu={compact_menu} right_view={:?} left_view={left_view:?}",
        right_view.map(|view| view.id())
    );
    cx.set_global(ShellStartup {
        compact_menu,
        right_view,
        left_view,
    });
}

/// 启动期形态里的菜单栏模式（重建路径读它）。
fn startup_compact_menu(cx: &App) -> bool {
    cx.try_global::<ShellStartup>()
        .map(|startup| startup.compact_menu)
        .unwrap_or(false)
}

/// 启动期请求的右 / 左栏视图（重建路径也要应用，理由见 [`ShellStartup`] 的字段文档）。
fn startup_views(cx: &App) -> (Option<RightToolWindowView>, Option<usize>) {
    cx.try_global::<ShellStartup>()
        .map(|startup| (startup.right_view, startup.left_view))
        .unwrap_or((None, None))
}

/// 「这次项目在哪儿打开」。真源 `ProjectOpenDestination`
/// （`windows/tauri/src/features/file-system/controllers/project-open-destination.ts:6`）。
///
/// `pub` 是给 `--open-project-probe --open-project-destination <取值>` 用的
/// （**验证/诊断用**；产品路径的取值由对话框的按钮决定，不走这个类型）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenDestination {
    /// 「新窗口」（真源 `new-window`）。
    NewWindow,
    /// 「此窗口」（真源 `this-window`）。
    ThisWindow,
}

/// 「这次是怎么决定的」：目的地 + 要不要在打开成功之后记住这个选择。
///
/// `remember = true` 只可能来自对话框里勾了「不再询问」（真源 `rememberAfterOpen`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProjectOpenDecision {
    pub(crate) destination: OpenDestination,
    /// 打开成功之后把「不再询问」与本次目的地一起写进设置。
    pub(crate) remember: bool,
}

/// 换项目的**决策**：逐条照真源 `chooseProjectOpenDestination`
/// （`project-open-destination.ts:78-110`）。
///
/// 返回 `None` = **需要弹对话框**（真源的第 ④ 段）。四段判据与真源的对应关系：
///
/// | 真源 | 这里 |
/// | --- | --- |
/// | ① `explicitDestination` 直接用 | `explicit` 参数（`--open-project-destination` / 菜单里点名的那一支） |
/// | ② `!hasOpenWorkspace` → `this-window`，不询问 | **不适用**：本侧 `ShellWorkspace` 恒有一个工作区根（启动参数给的），不会有"没有工作区"这一态 |
/// | ③ `!askWhereToOpenProjects` → 按 `openFoldersInNewWindow` 直接定 | `ask == false` 那一支 |
/// | ④ 否则弹对话框 | `None` |
///
/// 做成**纯函数**是为了能单测（本 crate 拿不到 `TestAppContext`，理由见 `menu_bar.rs`）。
pub(crate) fn resolve_project_open_destination(
    ask: bool,
    open_in_new_window: bool,
    explicit: Option<OpenDestination>,
) -> Option<ProjectOpenDecision> {
    if let Some(destination) = explicit {
        return Some(ProjectOpenDecision {
            destination,
            // 真源：显式指定目的地时**不**改偏好（那是"这一次"的意思，不是"以后都这样"）。
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

/// 状态栏左侧临时消息的存活时间（毫秒）。
///
/// "几秒后自动消失"（规格 Q2）。4 秒：够读到一句话，又不至于一直占着状态栏。
const STATUS_NOTICE_MS: u64 = 4_000;

/// `--left-view` 的取值 → 左栏下标（**验证/诊断用**的解析，不是产品能力）。
///
/// 只认**顶部组**那三个视图（项目 / 更改 / 搜索，`sidebar-pane-selector.tsx:311-319`）：
/// 底部组那四项（运行 / 终端 / 诊断 / 提交记录）切的是**底部工具窗**，不是"左栏显示谁"，
/// 把它们混进来会让 `--left-view terminal` 看起来像"左栏没有变化"。
/// 未知取值返回 `None`（调用方报一行错，不静默落到某一项）。
pub fn left_activity_index(selector: &str) -> Option<usize> {
    match selector {
        "files" | "project" => Some(DEFAULT_TOP_ACTIVITY),
        "changes" => Some(CHANGES_ACTIVITY_IX),
        "search" => Some(SEARCH_ACTIVITY_IX),
        _ => None,
    }
}


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

/// 左侧活动栏「搜索」项的下标 = 2（顶部组第 3 项，`item-order.ts:3` 的 `files, git, search`）。
///
/// 只给 `--left-view search` 用：本侧搜索视图还没有内容（点它只切选中态），
/// 所以 `--left-view search` 是"证明探针能切到顶部组任意一项"的对照，不是产品功能。
const SEARCH_ACTIVITY_IX: usize = 2;

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
        // B1 的八条同样**不在面板里**（`COMMAND_ORDER` 没有它们，入口只有主菜单「文件」）。
        // 它们的可见图标在 `MenuAction::icon`（菜单那一份才是用户看得到的），
        // 这里给同一批字形只为保持"两处入口视觉同源"这条既有口径。
        C::OpenFile => (Category::View, IconName::FolderOpen),
        C::CloseTab
        | C::CloseOtherTabs
        | C::CloseAllTabs
        | C::CloseSavedTabs
        | C::CloseTabsToLeft
        | C::CloseTabsToRight => (Category::View, IconName::X),
        C::ReopenClosedTab => (Category::View, IconName::RotateCcw),
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
        // B1 的八条：label 与菜单项**同一个键**（两处入口同一句话）；description 取它们所在的
        // 顶级菜单名（真源既有键「文件」）—— 这八条**永远不会被画出来**（不在 `COMMAND_ORDER`），
        // 所以这里不编新句、也不假装它们是命令面板的动作。
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
    ///
    /// ⚠️ 扫描**不是**在派发栈里跑的（[`ShellWorkspace::schedule_right_view_scan`]），
    /// 所以在"已登记、任务还没跑完"这段窗口里它同样是 `None`，面板画空态。
    maven_project: Option<crate::maven::MavenProjectView>,
    /// Spring 索引（`spring.index` 的结论），与 Maven 同一套懒扫 + 缓存口径。
    ///
    /// `spring.index` 会读依赖 JAR 里的 `spring-configuration-metadata.json`（契约说
    /// `refreshDependencyMetadata: true` 只该在"打开项目"时置真），所以它比 `maven.scan` 更贵，
    /// **更不能**挂在启动路径或每次按键上。
    spring_index: Option<crate::spring::SpringIndexView>,
    /// 两个视图"扫过没有"的登记状态（原 `maven_scanned` / `spring_scanned` 的语义）。
    ///
    /// 为什么与数据分开存：数据在**延后的任务**里写回，而"要不要排这一次扫描"必须在派发栈内
    /// 就定下来 —— 否则同一项连点两次（或点击与菜单同时来）会排出两个任务，把"每个视图只扫
    /// 一次"变成"每次入口都扫一次"。纯值 + 纯函数，所以这条语义可以直接单测
    /// （[`RightScanState::request`]）。
    right_scan: RightScanState,
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
    /// 「最近项目」的内存真源（B4）。落盘是它的投影。
    ///
    /// 构造期从 `%APPDATA%\Lithe\recent-projects.json`（或 `LITHE_GPUI_SETTINGS_FILE`
    /// 指到的同目录）读一次，之后只在"打开项目成功"与"路径失效"两处改它并落盘。
    /// 换项目会**重建整个外壳**，新外壳的构造期会重新读一遍 —— 所以上一条记录一定在新
    /// 外壳的列表里（写入发生在换根之后，见 [`ShellWorkspace::rebuild_project_window`]）。
    recent_projects: RecentProjects,
    /// 最近项目文件的路径（`None` = 推导不出，只影响落盘）。
    recent_projects_path: Option<PathBuf>,
    /// 换项目对话框里「不再询问」的**当前勾选态**。
    ///
    /// ## 为什么是 `Rc<Cell<bool>>`（实测踩过 panic）
    ///
    /// 勾选态必须**跨帧**存在（`window.open_dialog` 的 builder 每帧都会被重跑，
    /// `gpui-component-0.6.6/src/root.rs:256-262`），所以不能是闭包里的局部变量。
    /// 但它也**不能**像一开始那样"builder 每帧 `shell.read(..)` 去读实体字段"：
    /// dialog 层是**外壳自己画的**（本文件 `render` 第一行的 `Root::render_dialog_layer`），
    /// 于是 builder 运行时外壳正在被更新，再去 `read` 它会 panic：
    ///
    /// ```text
    /// cannot read lithe_gpui_workbench::workspace::ShellWorkspace while it is already being updated
    /// ```
    ///
    /// 所以真源是**一个共享的 cell**：外壳持有它（唯一真源），builder 捕获一份克隆后**直接读值**
    /// （不碰实体），勾选回调写 cell **并**让外壳重绘 —— 重绘才会重跑 builder，新值于是画出来。
    /// 同一形状的先例是本文件项目下拉的 `trigger_bounds`（`Rc<Cell<..>>`，理由也一样：
    /// "在 `'static` 回调里写、在 `render` 里读"）。
    ///
    /// 它是**这一次的待写入值**，不是设置本身：真源要求"先打开成功、后写偏好"
    /// （`project-open-destination.ts:112-131`），所以勾上之后不立刻落盘，
    /// 由 [`ShellWorkspace::rebuild_project_window`] 在换根成功之后写。
    project_open_do_not_ask: Rc<Cell<bool>>,
    /// 状态栏左侧的**临时消息**（规格 Q2/Q6：未接线的能力在这里给一句"尚未接入：缺 X"）。
    ///
    /// 为什么放在外壳而不是 `crate::status_bar`：状态栏是**无状态渲染函数**
    /// （`status_bar` 的模块文档），条目由 [`ShellWorkspace::footer_left`] 每帧现算 ——
    /// 所以"临时消息"自然就是外壳的一个字段 + 一条 [`STATUS_NOTICE_MS`] 后清掉它的任务。
    status_notice: Option<SharedString>,
    /// 临时消息的代数：定时器醒来时只有"自己那一代还是最新的"才清空。
    ///
    /// 没有它就会出现"后一条提示被前一条的定时器提前清掉"（4 秒内连开两次项目时可见）。
    status_notice_generation: u64,
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

        // 设置里的「项目 · JDK 与 Maven」页把 JDK 覆盖值（`javaHomePath`）存进设置文件，
        // 而这条值**唯一的消费方是语言服务**（JDT LS 用哪个 JDK 起）：这条链路只有本 crate
        // 拼得起来 —— `lithe-gpui-java` 不许依赖 `settings`（那会把设置文件的键灌进语言服务），
        // 而 `settings` 不认识 JDT LS 会话。照 `set_git_identity_host` 同一口径：
        // 低层 crate 定义注入点（`lithe_gpui_java::toolchain`），外壳在这里登记。
        //
        // ⚠️ **必须早于下面那一步 `prepare_java`**：它会在后台起 JDTLS 并当场读这个值，
        // 登记晚了这一轮读到的还是"自动发现"的那个 JDK —— 也就是页面上写着「已选择」
        // 却没有生效的那副假象。
        register_java_toolchain(cx);

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
                "S1_SETTINGS wiring=workbench tab_size={} terminal_default_shell_id={:?} confirm_before_discard={} auto_completion={}",
                initial.tab_size,
                initial.terminal_default_shell_id,
                initial.confirm_before_discard,
                initial.auto_completion
            );
            editor.update(cx, |pane, cx| {
                pane.set_tab_size(initial.tab_size as usize, cx);
                pane.set_auto_completion(initial.auto_completion, cx);
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
                    // 「自动补全」（阶段 18，「LSP」页）：与 `tabSize` 同一条"值型设置经外壳
                    // 转发"的路子。落点是编辑区那份共享的原子开关，所以**已打开的** Java
                    // buffer 与之后新开的都立刻跟上（见 `EditorPane::set_auto_completion`）。
                    pane.set_auto_completion(settings.auto_completion, cx);
                });
                this.terminal.update(cx, |pane, cx| {
                    pane.set_default_shell(&settings.terminal_default_shell_id, cx);
                });
                // ⚠️ 用字段 `changes`（不是外层捕获的那个实体）：这条闭包要在
                // `ShellWorkspace` 上取本视图自己的引用，否则会和上面的 `cx.observe` 抢借用。
                this.changes.update(cx, |view, cx| {
                    view.set_confirm_before_discard(settings.confirm_before_discard, cx);
                });
                // JDK 覆盖值也在这里跟着走：登记本身是幂等的，但**它只影响下一次语言服务
                // 启动**（JDT LS 会话是一个工作区一个、`EditorPane::prepare_java` 幂等），
                // 所以改了设置之后要重启应用才真的换 JVM —— 换 JDK 必须重建 JDT 索引，
                // 静默重启会话会让索引与 JVM 的对应关系断掉。
                register_java_toolchain(cx);
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
        //
        // ⚠️ 最近项目在这里**读一次**（B4）：面板每次打开时用的是内存里的那一份，
        // 顺序就是数据层的顺序（`pinned` 优先 + `lastOpenedAt` 降序）。读取的容错口径
        // （文件不在 / 坏 JSON / 坏条目）都在 `recent_projects` 模块里，这里只打印它的诊断。
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

        // 标题栏的分支项 + 分支弹窗（`crate::branch_panel`）。同一套：
        // 构造期自己读一次 `git.status` + `git.references`（标题栏那一项第一帧就要画对），
        // 打一行 `S1_BRANCH_PANEL opened=false …` 的启动证据，最后订阅它让自己重绘。
        let branch_panel = BranchPanel::new(root.clone(), window, cx);
        let branch_panel_subscription = Some(cx.observe(&branch_panel, |_, _, cx| cx.notify()));

        let mut workspace = Self {
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
            spring_index: None,
            right_scan: RightScanState::default(),
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
            recent_projects,
            recent_projects_path,
            // 对话框每次打开都从"没勾"开始：勾选态只活在**这一次**对话框里，
            // 落盘的是"以后不再问"这件事本身（写进设置）。
            project_open_do_not_ask: Rc::new(Cell::new(false)),
            status_notice: None,
            status_notice_generation: 0,
        };
        // 一行锚点诊断：整个外壳（项目树 / 编辑区 / JDTLS / 右栏 / Git 面板）都挂在**这一个**
        // 根上。换项目会重建整个外壳，所以换根前后各有且只有一行 `S1_WORKSPACE root=…`
        // —— 它是"换根真的发生了"的第一条证据，后面那些 `S1_EXPLORER` / `S1_JAVA_START`
        // / `S1_BRANCH_PANEL` 都应当指向同一个根。
        println!("S1_WORKSPACE root={}", root.display());
        // 启动期也留一行状态证据：右工具窗**默认隐藏**这件事要能被机器验证，
        // 而不是只靠截图比对（`S1_RIGHT_PANEL`，可 grep）。构造期不是指针输入，所以不带坐标。
        diagnose_right_panel(workspace.right_view, workspace.right_visible, None);
        // `--right-view` / `--left-view`：**每个外壳都应用一次**（含换根重建出来的那个）。
        //
        // 为什么放在构造期而不是 `main.rs` 的"首帧之后"：换根会重建整个外壳，而
        // `main.rs` 那段 `on_next_frame` 是一次性的、捕获的是**第一个**外壳 —— 不做这一步，
        // "换根后右栏 Maven / Spring 面板与 Git 变更视图都指向新根"就取不到证据
        // （视图不打开就不会去扫 / 去读）。构造期不是 render，改状态是安全的。
        let (startup_right_view, startup_left_view) = startup_views(cx);
        if let Some(view) = startup_right_view {
            workspace.right_view = view;
            workspace.right_visible = true;
            workspace.schedule_right_view_scan(view, true, cx);
            diagnose_right_panel(view, true, None);
        }
        if let Some(index) = startup_left_view {
            workspace.top_activity_view = Some(index);
            if index == CHANGES_ACTIVITY_IX {
                let changes = workspace.changes.clone();
                let _ = changes.update(cx, |view, cx| view.activate(cx));
            }
            println!("S1_LEFT_VIEW index={index}");
        }
        // 命令面板的动作要改本视图的状态，而浮层的 builder / 回调都是 `'static`，
        // 够不着 `self`：所以在这里登记一个弱引用句柄（理由见 `crate::command_palette`
        // 的 `SHELL`）。登记在 `Self` 建好之后，句柄一定是可升级的。
        // 先取外壳句柄（`cx.entity()` 只要 `&App`），再分别登记给命令面板与项目下拉。
        let shell_handle = cx.entity().downgrade();
        set_shell(shell_handle.clone());
        // 项目下拉里**可点的那些行**（「打开…」与最近项目）要回来落到本外壳上：面板实体
        // 自己没有外壳句柄，所以在 `Self` 建好之后补登记一次（理由见
        // `crate::project_menu::ProjectMenu` 的 `shell` 字段文档）。
        workspace
            .project_menu
            .clone()
            .update(cx, |menu, _cx| menu.set_shell(shell_handle));
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
            MenuAction::OpenFolder => {
                // 接线点：gpui 自带的 `App::prompt_for_paths`（directories: true），
                // 与标题栏项目下拉的「打开…」走**同一个**函数 —— Q12 说这两条同义。
                // 整段是异步的（oneshot 回来再弹对话框 / 换根），所以这里不会挡住本帧。
                diagnose_menu_run(action, "prompt");
                self.open_project_picker(window, cx);
            }
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
            // 走上一条 `if let` 的动作在这里不可达；写全分支是为了让"表里加一条动作"
            // 变成编译错误而不是运行时静默。
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
    /// [`CommandId::NavigateToDefinition`] / [`CommandId::ToggleMenuBar`] 与 B1 新增的
    /// [`CommandId::OpenFile`] ＋ 七条关闭系 **不在命令面板里**
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
            CommandId::OpenFile => {
                // 接线点：gpui 自带的 `App::prompt_for_paths`
                // （`gpui-pre-0.3.6/src/app.rs:1687-1692`；Windows 实现是真的
                // `IFileOpenDialog`，`gpui-pre-windows-0.3.6/src/platform.rs:673-684` →
                // `:1374-1400` 的 `file_open_dialog`，取消时回 `Ok(None)`）。
                //
                // ⚠️ `project_menu.rs:108-113` 当年写"gpui 侧没有任何文件对话框依赖"是**不完整**的
                // （它只查了 `rfd` / `tinyfiledialogs` / `native-dialog` 三个 crate，漏掉了 gpui
                // 自带的这一条），并因此把「打开…」判成"做不到"。这里用的是同一台机器上
                // **零新增依赖**的通路。
                //
                // 选中的路径**异步**回来（`oneshot::Receiver`，不是回调），所以整段起一个
                // 前台任务：`spawn_in` 给的 `AsyncWindowContext` 能在 await 之后拿回
                // `&mut Window` —— 落盘到编辑区要它（`EditorPane::open` 要 `window`）。
                //
                // ⚠️ 本函数是**在渲染里**被调用的（`ShellWorkspace::render` 的
                // `for action in take_pending_runs(cx)`），所以"调 `prompt_for_paths`
                // 会不会把这一帧挡住"必须说清楚：Windows 侧的实现把 `IFileOpenDialog`
                // 放到**专用线程**上跑（`gpui-pre-windows-0.3.6/src/dialog.rs:85-133` 的
                // `show_dialog` 里 `std::thread::Builder::…spawn`），本线程只拿一个
                // `Receiver` 就返回 —— 实测这一帧照常画完（面板在点击后立刻收起）。
                diagnose_run(id, "prompt");
                let picked = cx.prompt_for_paths(PathPromptOptions {
                    files: true,
                    directories: false,
                    // 多选：真源的文件选择器同样允许一次开多个
                    // （`windows/tauri/src/features/file-system/controllers/platform.ts:126-127`
                    // 的 `open({ multiple: true })`）。选多个时**逐个打开**，
                    // 最后一个成为活动标签（`EditorPane::open` 每次都会把它设成活动）。
                    multiple: true,
                    prompt: None,
                });
                let editor = self.editor.clone();
                let opened = cx.spawn_in(window, async move |_this, async_cx| {
                    // ⚠️ **两层 `Result`**：外层是 oneshot 通道（送信端被丢），内层是平台侧
                    // 的错误（Linux 打开选择器失败时会给）。两者都不是"用户取消"，
                    // 所以各自打一行、各自与 `cancelled` 区分开。
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
                    // 诊断先打（在开文件之前）：`count` 是"选择器回了几个路径"的直接证据，
                    // 而每个文件开成功与否另有 `S1_EDITOR_LANG` / `S1_EDITOR_OPEN` 一行。
                    eprintln!(
                        "S1_EDITOR_OPEN_FILE state=picked count={} first={}",
                        paths.len(),
                        paths[0].display()
                    );
                    // 不在这里 `cx.notify()`：那是 `Context<T>` 的方法，`AsyncWindowContext::update`
                    // 给的是 `&mut App`（只有一个收 `EntityId` 的 `App::notify`）。也不需要 ——
                    // `pane.open` 自己收尾会 `cx.notify()`，而外壳订阅了编辑区实体
                    // （`_editor_subscription`），会跟着重绘。
                    let _ = async_cx.update(move |window, cx| {
                        for path in &paths {
                            eprintln!("S1_EDITOR_OPEN path={}", path.display());
                            let _ = editor.update(cx, |pane, cx| pane.open(path, window, cx));
                        }
                        // 活动标签是谁：`pane.open` 每次都把新开的那个设成活动，
                        // 所以这里读到的就是"切到该文件"的证据（状态栏那一格读的是同一个值）。
                        let active = editor.read(cx).active_buffer_name();
                        eprintln!("S1_EDITOR_OPEN_FILE state=opened active={active}");
                    });
                });
                // `detach()` 而不是 `let _ =`：gpui 的 `Task` 一 drop 就**取消**
                // （与 `GitIdentityHost` 那两段同一条实测教训），丢掉它等于"选择器刚打开
                // 就被取消"。
                opened.detach();
            }
            CommandId::CloseTab => {
                // 接线点：`EditorPane::close_active`（`Ctrl+W` 的同一个方法）。
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
                // 与右活动栏点击同一条懒扫口径（菜单与右栏改的是同一份状态，取数据也该一致）：
                // 懒扫只有一份，见 [`ShellWorkspace::schedule_right_view_scan`]。
                self.schedule_right_view_scan(view, visible, cx);
                // 与右活动栏点击走同一个诊断（`S1_RIGHT_PANEL view=maven visible=…`）：
                // 这样"菜单里的 Maven 项和右栏那一项改的是同一份状态"有机器证据。
                //
                // 不带坐标：菜单动作是从菜单栏的点击回调排进队列、再由 render 执行的，
                // 到这一层已经拿不到那一次点击的坐标；`window.mouse_position()` 这时可能是
                // 键盘激活菜单前留下的旧位置，拿它冒充"本次点击位置"会造出假的坐标证据。
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
        // **临时消息在的时候它占满左组**（Q2/Q6：未接线的能力在这儿给一句"尚未接入：缺 X"）。
        //
        // ⚠️ 为什么不是"插在最前面、其余照排"：实测（`.artifacts/p22/p22-new-window-notice.png`
        // 的第一版）那样会把项目名与分支**压在提示底下**——状态栏的条目自己不做文字截断
        // （截断能力在 `status_bar.rs`，而本轮写域不含它），一条 40 字的提示会盖住后面两项。
        // 提示 4 秒后自己消失，左组照常恢复成"项目名 + 分支"，所以这个取舍在界面上是
        // "暂时换一句话"而不是"少了两项"。
        if let Some(notice) = &self.status_notice {
            return vec![StatusEntry::new(notice.clone())];
        }

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

    /// 右工具窗的**懒扫**：某视图第一次真的显示出来时才去取它的数据（各扫一次，结果缓存）。
    ///
    /// 三条入口（右活动栏点击 / 「视图 → Maven」菜单项 / `--right-view` 启动探针）都调它，
    /// 所以"点开面板"和"启动就打开面板"拿到的是**同一份数据**。不抽出来的话同一段判据会有三份，
    /// 一份漏改就会出现"探针打开的面板是空的、点开的不是"这类只在一条路径上的偏差。
    ///
    /// 为什么懒扫而不是构造期扫：`maven.scan` 要解析 pom，`spring.index` 还要读依赖 JAR 里的
    /// 元数据（契约说 `refreshDependencyMetadata: true` 只该在"打开项目"时置真，见
    /// [`ShellWorkspace::spring_index`]），挂在启动路径上会让首帧为不相关的面板付钱。
    ///
    /// ## 为什么真正的读盘要**延后出派发栈**
    ///
    /// 这个方法的三条入口都跑在**点击派发栈**里（右活动栏的 `on_click`、菜单动作、
    /// `--right-view` 探针），而 `maven.scan` / `spring.index` 是同步读盘：同步跑会让"点了这一下"
    /// 一直不返回，后续输入只能排在队列里、随后被立刻处理完 —— 也就是把"第二次输入"和第一次挤进
    /// 同一段观测窗口（`gpui/research/click-double-trigger-dpi.md` §5 末尾单独登记过这条）。
    ///
    /// 所以这里只做两件事：**登记**（[`RightScanState::request`]，两个 bool 里至多改一个）+
    /// 把读盘排到任务里。选 `cx.spawn`（前台任务）而不是 `cx.defer`：
    ///
    /// - `cx.defer` 的回调在**同一轮** `App::update` 结尾的 `flush_effects` 里执行
    ///   （`gpui-pre-0.3.6/src/app.rs:1781-1843` 的 `Effect::Defer`、`:2064-2070` 的 `App::defer`
    ///   文档原文 "at the end of the current effect cycle"）。它出了 `dispatch_event`，但**没出
    ///   这条输入消息** —— 消息循环里排着的下一条输入仍要等它跑完；
    /// - `cx.spawn` 在 Windows 上经 `dispatcher.rs:118-129` 的 `PostMessageW(
    ///   WM_GPUI_TASK_DISPATCHED_ON_MAIN_THREAD)` 投递，任务的第一次 poll 发生在当前输入消息
    ///   处理**返回消息循环之后**，而且排在队列尾部 —— 已经在队列里的输入消息会被先处理。
    ///   这既是"出派发栈"，也是"不占这一轮消息"；
    /// - 不用 `background_spawn`：扫描经 Core 的 JSON 边界取数据（`crate::maven` / `crate::spring`），
    ///   本 crate 里这些调用都还没有跨线程的先例，换成后台线程是另一件事（本轮不做，也不引入
    ///   自己的线程池/轮询）。
    ///
    /// **语义保持不变**：每个视图只扫一次、结果缓存。登记在派发栈内就落下（所以延后不会退化成
    /// "再次点击又排一个任务"）；数据没回来之前 `maven_project` / `spring_index` 仍是 `None`，
    /// 面板画的是空态 —— 这一点可接受：没有数据本来就没有内容可画，而且重绘本身是消息循环里的
    /// 另一条消息（`flush_effects` 最后只是 `schedule_frame()`，`app.rs:1839`），任务排得早时
    /// 首帧通常已经带上数据（只是不保证，所以按空态兜底）。
    fn schedule_right_view_scan(
        &mut self,
        view: RightToolWindowView,
        visible: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(target) = self.right_scan.request(view, visible) else {
            return;
        };
        // 工作区根在外面拷一份带走：任务全程在主线程（`dispatch_on_main_thread`），
        // 拿快照与"任务里再 `update` 一次去读"等价，但少一次 `update` 与一个失败分支。
        let root = self.root.clone();
        // 扫的**是哪个根**要能 grep：`maven.rs` / `spring.rs` 自己的诊断只说结论
        // （artifactId / endpoints），而换项目之后"右栏两个面板指向新根"这条验收线
        // 需要的正是这一行（B4）。放在这里而不是那两个模块里：这一层才知道根。
        eprintln!(
            "S1_RIGHT_SCAN view={} root={}",
            view.id(),
            root.display()
        );
        // 故意只捕获**弱引用**（`cx.spawn` 给的 `this`）：窗口/工作区在任务跑完前被销毁时，
        // `this.update(..)` 直接返回 `Err` 被忽略 —— "面板已关闭"这一支的竞态兜底。
        cx.spawn(async move |this, cx| {
            // 先扫到局部变量，再一次性写回：写回时才可能发现"用户已经切走/收起了"，
            // 所以竞态判断放在写回处（[`right_scan_should_notify`]）。
            match target {
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
                // 扩展 / 通知没有数据层（纯空态）：`RightScanState::request` 不会为它们返回
                // `Some`，这里只是把匹配写穷尽。
                RightToolWindowView::Extensions | RightToolWindowView::Notifications => {}
            }
        })
        .detach();
    }

    /// `--right-view <id>` 的接线点（**验证/诊断用**，不是产品能力）。
    ///
    /// 做的是与"点右活动栏那一项"**同一段**状态迁移：`right_view = view`、`right_visible = true`、
    /// 走 [`ShellWorkspace::schedule_right_view_scan`] 的懒扫、打同一行
    /// `S1_RIGHT_PANEL view=… visible=true` 诊断。
    ///
    /// **为什么需要这个入口**：本机（125% DPI）右活动栏一次点击会被处理成两次
    /// （`S1_RIGHT_PANEL … visible=true` 紧跟一行 `visible=false`，见 `gpui/HANDOFF.md` §2），
    /// 面板随即被自己收起，于是"面板里的内容长什么样"这件事在无人值守环境里取不到证。
    /// 与 `--open-settings` / `--menu-probe` 同一条口径：被绕开的只有"操作系统把那一下点击
    /// 送进窗口"这一段，它写的两个字段与点击回调写的是**同一个**（不是另一套状态）。
    pub fn show_right_view_probe(&mut self, view: RightToolWindowView, cx: &mut Context<Self>) {
        self.right_view = view;
        self.right_visible = true;
        self.schedule_right_view_scan(view, true, cx);
        // 探针不是指针输入：没有"本次点击的坐标"，所以不带 `x=` / `y=`
        // （判据要求"坐标相同"，编一个坐标出来会污染证据）。
        diagnose_right_panel(view, true, None);
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 「打开其他文件夹」＋「最近项目」（B4）
    // -----------------------------------------------------------------------

    /// 打开"选一个文件夹"的系统选择器（文件菜单「打开文件夹」/ 项目下拉「打开…」/ `Ctrl+O`）。
    ///
    /// 用的是 gpui **自带的** `App::prompt_for_paths`
    /// （`gpui-pre-0.3.6/src/app.rs:1687-1692`；Windows 侧是真 `IFileOpenDialog` +
    /// `FOS_PICKFOLDERS`）—— **零新增依赖**。`project_menu.rs` 早年写"gpui 侧没有任何文件
    /// 对话框依赖"时只查了三个第三方 crate，漏掉了这一条（B1 的「打开文件」已经订正过一次）。
    ///
    /// ⚠️ 返回的是 **oneshot `Receiver`，不是回调**（取消 = `Ok(None)`），所以整段起一个
    /// 前台任务：`spawn_in` 给的 `AsyncWindowContext` 能在 `await` 之后拿回 `&mut Window`
    /// —— 弹对话框 / 换根都要它。选择器的**实现**在专用线程上（Windows 侧
    /// `gpui-pre-windows-0.3.6/src/dialog.rs` 的 `show_dialog`），本线程只拿一个 `Receiver`
    /// 就返回，所以从 `render` 里调它不会挡住这一帧（与 B1 的「打开文件」同一条实测）。
    pub(crate) fn open_project_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 已经开着浮层（设置 / 命令面板 / 换项目对话框）时不叠第二层：与
        // `open_settings_dialog` / `open_command_palette` 同一条口径。`Ctrl+O` 的全局
        // 处理器可能被登记多次（每次重建外壳都会登记一次，见 `install_open_project_action`），
        // 这道闸门同时挡住"同一次按键被两个处理器各开一个选择器"。
        if window.has_active_dialog(cx) {
            eprintln!("S1_OPEN_PROJECT state=skipped reason=active_dialog");
            return;
        }
        eprintln!("S1_OPEN_PROJECT state=prompt");
        let picked = cx.prompt_for_paths(PathPromptOptions {
            // 选**文件夹**：`directories: true` + `files: false`。Windows 的
            // `can_select_mixed_files_and_dirs()` 返回 false（`FOS_PICKFOLDERS` 在"只文件"
            // 与"只文件夹"之间切换），所以两者必须二选一。
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        let shell = cx.entity().downgrade();
        let task = cx.spawn_in(window, async move |_this, async_cx| {
            // ⚠️ **两层 `Result`**：外层是 oneshot 通道（送信端被丢），内层是平台侧错误
            // （Linux 打不开选择器时会给）。两者都不是"用户取消"，各自打一行区分开。
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
        // `detach()` 而不是 `let _ =`：gpui 的 `Task` 一 drop 就**取消**
        // （与 `GitIdentityHost` 那两段同一条实测教训），丢掉它等于"选择器刚打开就被取消"。
        task.detach();
    }

    /// 请求打开 `target` 这个项目：**先探测路径，再决定在哪儿打开**。
    ///
    /// 三个入口共用它：选择器选完（[`Self::open_project_picker`]）、项目下拉里的最近项目行、
    /// 诊断入口 [`Self::open_project_probe`]。`explicit` = 真源的 `explicitDestination`
    /// （只有诊断/探针会传，产品路径传 `None`）。
    pub(crate) fn request_open_project(
        &mut self,
        target: PathBuf,
        explicit: Option<OpenDestination>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path_text = target.to_string_lossy().to_string();

        // 第一步：路径探测（照真源 `openRecentFolder` 的 `getSymlinkInfo` 那一步，
        // `stores/recent-folders.store.ts:106-120`）。不是目录就不换根 ——
        // 换根会把当前工作区整个丢掉，为一个点不开的路径付这个代价是不对的。
        // 最近项目列表里命中这条时**标 missing + 落盘**（真源 `updateRecentFolder(.., {missing:true})`），
        // 于是列表下次打开就能看出这条失效了。
        if !target.is_dir() {
            let marked = self.recent_projects.set_missing(&path_text, true);
            if marked {
                self.save_recent_projects();
            }
            eprintln!(
                "S1_OPEN_PROJECT state=missing path={path_text} marked={marked}"
            );
            // 两句真源既有文案（`fileSystem.recentProjectNotFolder` /
            // `fileSystem.recentProjectUnavailable`，`locale.ts:650-653`）：路径**在**但不是
            // 文件夹 vs 路径**不在**（或读不了）—— 真源也是这么分的（`stores/recent-folders.store.ts:106-120`）。
            let key = if target.exists() {
                "lithe.fileSystem.recentProjectNotFolder"
            } else {
                "lithe.fileSystem.recentProjectUnavailable"
            };
            self.show_status_notice(tr_args(key, &[("path", &path_text)]), cx);
            return;
        }

        let (ask, open_in_new_window) = self.project_open_preference(cx);
        match resolve_project_open_destination(ask, open_in_new_window, explicit) {
            // 需要询问：弹换项目对话框（勾选态归本实体，见 [`Self::project_open_do_not_ask`]）。
            None => self.open_project_where_dialog(target, window, cx),
            Some(decision) => self.execute_project_open(target, decision, window, cx),
        }
    }

    /// 读「打开其他项目」的两个判据（设置文件里的值；没有设置状态时按真源默认 `true`）。
    ///
    /// 默认值必须是**真源默认**而不是"最省事的那个"：没有设置状态（测试 / 别的宿主）时
    /// 装作"用户已经选了不再询问"会让对话框永远不出现，那是把默认值当成了用户选择。
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

    /// 执行一个已经定下来的决定（真源 `executeProjectOpenDecision` 的前半段）。
    ///
    /// ⚠️ **`remember` 的写入顺序照真源**：先打开、成功之后才写偏好
    /// （`project-open-destination.ts:117-128`）—— 见 [`Self::rebuild_project_window`]。
    fn execute_project_open(
        &mut self,
        target: PathBuf,
        decision: ProjectOpenDecision,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match decision.destination {
            // 「新窗口」：**多窗口整批暂缓**（维护者决定，规格 §5）。这里不假装能开：
            // 给一句"尚未接入：缺 X"的状态栏消息 + 一行可 grep 的诊断，也**不**记最近项目
            // （什么都没打开，记了就是假记录）。
            OpenDestination::NewWindow => self.report_new_window_not_wired(cx),
            OpenDestination::ThisWindow => {
                self.rebuild_project_window(target, decision.remember, window, cx)
            }
        }
    }

    /// 「新窗口」那颗按钮（以及"记住的偏好就是新窗口"那条决策）的落点。
    ///
    /// 真源这里真的会开第二个窗口（`createAppWindow`），而 gpui 侧维护者已拍板
    /// **多窗口暂缓**：菜单栏 / 项目下拉 / 命令面板 / Git 身份宿主那几个 `thread_local`
    /// 句柄还是"一个进程一份"，第二个 `ShellWorkspace` 会把第一个的句柄覆盖掉（串台）。
    /// 所以本侧只能给 Q2 口径的那句提示 —— 文案见 `GPUI_ONLY_KEYS`。
    fn report_new_window_not_wired(&mut self, cx: &mut Context<Self>) {
        eprintln!(
            "S1_OPEN_PROJECT destination=new-window state=not_wired missing=window_handle_routing"
        );
        self.show_status_notice(tr("lithe.gpui.newWindowNotWired"), cx);
    }

    /// 「此窗口」：**重建整个外壳**（见模块头的"为什么不能逐个 reset"）。
    ///
    /// 顺序（每一步都有理由，不能调换）：
    ///
    /// 1. `replace_root`：新根上的 `ShellWorkspace` + 新的 `Root`（**窗口根必须是 `Root`**，
    ///    它是对话框 / 浮层 / 通知的宿主，`main.rs:709` 同一句）。旧 `Root` 及其整棵视图树
    ///    （含旧 JDTLS 会话与终端进程）在外层 `App::update` 的 `flush_effects` 里级联 drop。
    /// 2. **打开成功之后**才：① 把这次打开记进最近项目（含落盘）；② 勾了「不再询问」时
    ///    写两个偏好键（真源 `:117-128` 的顺序）。
    ///
    /// ⚠️ **调用方必须先 `window.close_dialog`**：换项目对话框挂在旧 `Root` 的
    /// `active_dialogs` 上，而 dialog 层由旧外壳画 —— 不先关掉就会出现"点了按钮对话框
    /// 凭空消失"（浮层挂在被换掉的那棵树上）。
    ///
    /// "打开成功"的判据：`replace_root` 没有 `Result`（`gpui-pre-0.3.6/src/window.rs:2242-2254`），
    /// 它会同步建出新外壳并装成窗口根 —— 所以**能走到下一步就是成功**。真源的 `open()` 返回
    /// 一个 bool 是因为它可能被 `createAppWindow` 拒绝（新窗口那条路），本侧这条路没有失败态。
    fn rebuild_project_window(
        &mut self,
        target: PathBuf,
        remember: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 菜单栏形态在重建时要取回来：它是应用级启动参数（`ShellStartup`），
        // 不能捕获 `main.rs` 的局部变量（理由见该 Global 的文档）。
        let compact_menu = startup_compact_menu(cx);
        eprintln!(
            "S1_OPEN_PROJECT destination=this-window path={} state=rebuilding",
            target.display()
        );
        let old_root = self.root.display().to_string();
        // 闭包是 `FnOnce`，用这个槽把新建出来的外壳句柄带出来（重建之后它才是所有者）。
        let mut created: Option<Entity<ShellWorkspace>> = None;
        let new_root = window.replace_root(cx, |window, cx| {
            let workspace = cx.new(|cx| {
                ShellWorkspace::new(target.clone(), compact_menu, false, window, cx)
            });
            created = Some(workspace.clone());
            Root::new(workspace, window, cx)
        });
        // `replace_root` 已经把新 `Root` 装成窗口根；这里只留一行回执，
        // 免得读者以为返回值没用上。
        let _ = new_root;

        let Some(workspace) = created else {
            // 只可能发生在 `replace_root` 的实现被换掉时；如实登记，不静默。
            eprintln!("S1_OPEN_PROJECT state=rebuild_failed reason=no_view");
            return;
        };

        // ① 最近项目：换根之后写（真源 `addToRecents` 也在打开成功之后），
        //    于是新外壳的列表里立刻有这一条，不需要再读一次文件。
        let _ = workspace.update(cx, |shell, cx| {
            shell.record_opened_project(&target, Some(false), cx)
        });

        // ② 「不再询问」：两个键一起写（真源写两次 `updateSetting`，本侧合成一次 commit）。
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
                None => eprintln!(
                    "S1_OPEN_PROJECT remember=skipped reason=no_settings_store"
                ),
            }
        }
        println!(
            "S1_OPEN_PROJECT state=opened path={} from={old_root}",
            target.display()
        );
    }

    /// 换项目对话框（真源 `projectOpen.*` 六键，**零新增真源文案**）。
    ///
    /// 形态照真源 `showChoiceDialogWithCheckbox`：标题 + 一行说明 + `□ 不再询问` +
    /// `取消 / 新窗口 / 此窗口`。
    ///
    /// ⚠️ 两处上游限制决定了拼法（侦察 `gpui/research/menu-open-prereqs.md` §1）：
    ///
    /// 1. **`Dialog` 只有 ok / cancel 两个按钮槽**（`DialogButtonProps`），三按钮必须走
    ///    `.footer(..)` —— 上游写死"设了 footer 就忽略 `button_props`"，
    ///    这正是 `editor_view.rs` 的未保存确认框的写法；
    /// 2. **没有内置 checkbox 槽**：`Checkbox` 组件存在，但"对话框里的 checkbox"要自己
    ///    当 child 塞进正文区。
    ///
    /// 勾选态存本实体的一个共享 cell（理由见 [`Self::project_open_do_not_ask`] 的字段文档：
    /// builder 每帧重跑，而它**不能**在外壳正在被更新的渲染期反过来读外壳实体）。
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
            // ⚠️ 每个捕获值都在闭包体内 `clone`：`open_dialog` 收的是 `Fn`（可被多次调用），
            // 内层 `move` 闭包不能把外层捕获的变量整个搬走（E0507）。
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
                // 标题 = 决策，正文 = 说明（含项目名），按钮 = 结果词：与设计指南
                // `design-guides.md:427-434` 的确认对话框口径一致。
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
                        // 受控值：勾选态来自那个共享 cell（**不读实体** —— 见字段文档里的
                        // panic 记录：builder 跑在外壳自己的 `render` 里）。
                        .checked(do_not_ask.get())
                        .on_change(move |&value, _window, cx| {
                            checkbox_cell.set(value);
                            // 写 cell 不够，还要让**外壳重绘**：只有重绘才会重跑 builder，
                            // 新值才会画出来（`render_dialog_layer` 是渲染期跑的）。
                            let _ = checkbox_shell.update(cx, |_shell, cx| cx.notify());
                        }),
                )
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        // 按钮顺序照指南「取消 + 结果词」，主操作（此窗口）最右且 primary。
                        .child(Button::new("lithe-project-open-cancel").label(cancel_label).on_click(
                            |_, window, cx| window.close_dialog(cx),
                        ))
                        .child(
                            Button::new("lithe-project-open-new-window")
                                .label(new_window_label)
                                .on_click(move |_, window, cx| {
                                    // 「新窗口」现在是提示（规格 §5）：先让状态栏那句出现，
                                    // 再关对话框 —— 反过来的话提示会被浮层的遮罩挡在后面。
                                    let _ = new_window_shell
                                        .update(cx, |shell, cx| shell.report_new_window_not_wired(cx));
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("lithe-project-open-this-window")
                                .primary()
                                .label(this_window_label)
                                .on_click(move |_, window, cx| {
                                    // 勾选态在**点击这一刻**读一次（共享 cell，不碰实体）：
                                    // 换根之后旧外壳连同它的 cell 一起没了。
                                    let remember = click_cell.get();
                                    // ⚠️ 先关对话框再换根（理由见 `rebuild_project_window`）。
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

    /// 记一次"打开项目成功"：写进最近项目列表 + 落盘。
    ///
    /// 规则（上限 12 / `pinned` 不占额度 / 按 `path` 精确去重 / 最新在前）**全在数据层**
    /// （`lithe_gpui_settings::recent_projects`，已单测），这里只负责"给时间戳 + 落盘 + 打诊断"
    /// —— 与那条"数据层不取时钟"的分工一致。
    ///
    /// `open_in_new_window` 传的总是 `Some(false)`：本侧唯一真的打开成功的形态就是「此窗口」
    /// （新窗口还没做）。真源记的是"用户选了哪一边"，这里如实记 `false`。
    fn record_opened_project(
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
        // 下拉下次打开就是新列表（面板每次打开现取本实体的这一份）。
        cx.notify();
    }

    /// 把最近项目列表原子写回文件（复用 `persistence::save_json` 的"临时文件 + rename"）。
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
            // 写失败不 panic：内存里的列表仍然是权威的，下一次打开项目会再试一次。
            Err(error) => eprintln!(
                "S1_SETTINGS_RECENT save_failed path={} error={error}",
                path.display()
            ),
        }
    }

    /// 状态栏左侧的**临时消息**：置位 → 重绘 → [`STATUS_NOTICE_MS`] 之后自己清掉。
    ///
    /// 出口只有 [`ShellWorkspace::footer_left`]（它是左组的第一条），因为状态栏本身是
    /// **无状态渲染函数**（`crate::status_bar` 的模块文档）。
    /// 消息本身走 `tr` / `tr_args`（与别的界面文案同一条规矩），并且额外打一行
    /// `S1_STATUS_NOTICE` —— 无人值守验证时那句提示是亮过的，可以 grep 到。
    fn show_status_notice(&mut self, text: SharedString, cx: &mut Context<Self>) {
        eprintln!("S1_STATUS_NOTICE text={text}");
        self.status_notice = Some(text);
        self.status_notice_generation = self.status_notice_generation.wrapping_add(1);
        let generation = self.status_notice_generation;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(STATUS_NOTICE_MS))
                .await;
            // 实体可能已经销毁（换根 / 关窗）：`update` 返回 `Err` 时静默忽略。
            let _ = this.update(cx, |shell, cx| {
                // 只有"这一条还是最新的"才清：否则 4 秒内连来两条提示时，
                // 前一条的定时器会把后一条提前抹掉。
                if shell.status_notice_generation == generation && shell.status_notice.is_some() {
                    shell.status_notice = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    // -----------------------------------------------------------------------
    // B4 的诊断入口（`--open-project-probe`）
    // -----------------------------------------------------------------------

    /// `--open-project-probe <目录>` 的接线点（**验证/诊断用**，不是产品能力）。
    ///
    /// 两种形态，**都没有**一条自己的换项目实现：
    ///
    /// 1. 不给 `explicit`：走 [`Self::request_open_project`] —— 与"在选择器里选完一个文件夹"
    ///    完全同一条链路（默认设置下会弹换项目对话框）。被绕开的只有"操作系统把这次选择
    ///    送回来"那一段；
    /// 2. 给了 `explicit`（`--open-project-destination`）：直接把决定交给
    ///    [`Self::execute_project_open`] —— 与"在对话框里点了那一颗按钮"写的是**同一个**
    ///    [`ProjectOpenDecision`]、同一个执行点，被绕开的只有"点按钮"那一下。
    ///    `remember`（`--open-project-remember`）也就是对话框里那个
    ///    [`Self::project_open_do_not_ask`] 字段 —— 于是"勾了「不再询问」→ 换根 → 两个偏好键
    ///    落盘 → 下一次不再弹"这条链能在本机锁屏（点不了 checkbox）的条件下端到端跑一遍。
    ///
    /// ⚠️ 形态 2 **不能**改成"把 explicit 交给 `request_open_project`"：那里显式目的地
    /// 会照真源把 `remember` 定为 `false`（真源 `:82-84`："这一次"不改偏好）——
    /// 那正是本探针要验的那条链，会被它吃掉（实测踩过：偏好一直没有落盘）。
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

    // `--left-view <id>` 的取值在 `ShellStartup` 里登记，由 `ShellWorkspace::new`
    // **在构造期应用**（换根重建出来的外壳也要应用，理由见那个 Global 的字段文档）——
    // 所以这里没有"首帧之后"的入口，处置与 `--right-view` 统一。
}

// `Ctrl+O` 的 action（「文件 → 打开文件夹」的键位入口，Q18）。
//
// 定义在**本模块**而不是 `crate::command_palette`：那是"命令面板的 action 表"，
// 而这条动作根本不是命令面板的一条（它不在 `COMMAND_ORDER` 里，只由菜单与键位触发）。
gpui_kit::actions!(lithe_workbench, [OpenProjectFolder]);

/// 登记 `Ctrl+O`（真源 `file.open` = "Open Project"，`command-registry.ts:236-242`；
/// 本侧照 Q18 把它给「打开文件夹」而不是「打开文件」）。
///
/// **在 `main.rs` 里登记一次**（App 级），不在 `ShellWorkspace::new` 里：那是每次重建
/// 外壳都会跑的地方，而 `App::on_action` 是**累加**的 —— 换一次项目就多一个处理器，
/// 同一个键会被处理多次（本方法里的 `has_active_dialog` 闸门只是第二道保险）。
///
/// 执行**不复用**另一条路径：它把 `MenuAction::OpenFolder` 压进菜单栏的待执行队列，
/// 下一帧由 `ShellWorkspace::render` 的同一个 `apply_menu_action` 收尾执行 ——
/// 也就是与"点菜单里那一项"逐字同一条链（只是绕开了"把这一项画出来再点它"）。
pub fn install_open_project_action(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-o", OpenProjectFolder, None)]);
    cx.on_action(|_: &OpenProjectFolder, cx: &mut App| {
        // 句柄没登记（窗口还没建出来）就什么都不做：菜单栏是**每个窗口一个**，
        // 没有窗口时没有"当前菜单栏"可压。这里不能 panic（`handle()` 会 expect）。
        let bar = crate::menu_bar::try_handle();
        let Some(bar) = bar else {
            eprintln!("S1_MENU_KEY id=lithe.menu.openFolder source=ctrl-o state=no_menu_bar");
            return;
        };
        let _ = bar.update(cx, |bar, cx| bar.push_run(MenuAction::OpenFolder, cx));
        eprintln!("S1_MENU_KEY id=lithe.menu.openFolder source=ctrl-o state=pushed");
    });
}

/// 外壳被释放时打一行**锚点诊断**（B4 的换根验收线要它）。
///
/// 为什么需要：换根的连带释放（JDTLS 会话、终端进程）是**级联 drop** 做的 ——
/// `window.replace_root` 丢掉旧 `Root`，旧 `Root` 丢掉旧 `ShellWorkspace`，
/// 再由 `ShellWorkspace` 的字段 drop 触发 `EditorPane::drop` / `TerminalPane::drop`
/// （侦察 `gpui/research/menu-open-prereqs.md` §2）。整条链上**只有这一行**能证明
/// "旧外壳真的被释放了"；没有它，日志里"旧 JVM 还在"到底是"没释放"还是"释放了但
/// `Session::shutdown` 还在跑"就分不开 —— 而 `java::Session` 的 `Drop` **只停事件泵、
/// 不关 JVM**（`java/src/session.rs:434-440`），正是这一批必须实测的风险点。
///
/// 打印用 `eprintln!`（无缓冲）：`println!` 在重定向到文件时是块缓冲的，
/// 进程还在跑的时候可能一行都看不到（本仓库已多次踩过，见 `main.rs` 的 `S1_ASSETS` 注释）。
impl Drop for ShellWorkspace {
    fn drop(&mut self) {
        eprintln!("S1_WORKSPACE_DROP root={}", self.root.display());
    }
}

/// 右工具窗懒扫的**登记状态**：哪个视图已经排过一次扫描。///
/// 只记"排过没有"，不记数据 —— 数据（`maven_project` / `spring_index`）由
/// [`ShellWorkspace::schedule_right_view_scan`] 排出的任务写回。
///
/// 为什么单独成类型：延后之后，留在派发栈里的就只剩 [`RightScanState::request`] 这一个决定，
/// 而它是纯值 + 纯函数，能用普通 `#[test]` 钉住"每个视图只扫一次"。另一半（读盘真的不在派发栈
/// 里）没有 gpui 宿主就没法有界地测：本 crate 没有 `TestAppContext` / `#[gpui::test]`
/// （理由见 `menu_bar.rs` 里那两处说明），所以它只由代码结构保证 —— 生产路径上
/// `crate::maven::scan` / `crate::spring::index` 各只有一处调用点，都在 `cx.spawn` 的任务体内
/// （`maven.rs` / `spring.rs` 自己的单测另有直接调用，那不是产品路径）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RightScanState {
    /// 是否已经**登记**过 Maven 扫描（登记 ≠ 已完成）。
    maven: bool,
    /// 同上，Spring。
    spring: bool,
}

impl RightScanState {
    /// 登记一次"请求显示 `view`"，返回**需要真的去扫**的那个视图（`None` = 不用排任务）。
    ///
    /// 判据与"延后之前"逐条相同：
    ///
    /// - `visible == false` ⇒ `None`，且**不消费**登记额度（收起面板不算"显示过"）；
    /// - 这个视图已经登记过 ⇒ `None`（"各扫一次并缓存"里那个"一次"）；
    /// - 没有数据层的两个视图（扩展 / 通知）⇒ 永远 `None`。
    ///
    /// 副作用只有至多一个 bool：不读盘、不分配、不阻塞，所以它可以留在派发栈里。
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

/// 懒扫任务写回数据之后"要不要重画"的决策（纯函数）。
///
/// 需要它是因为任务排出去之后状态**可能已经变了**：用户完全可能在扫描跑完前切到别的视图
/// （右活动栏点另一项 / 菜单里 toggle 到别处）或者把面板收起（再点同一项 / 头部关闭按钮）。
/// 那时这一帧画的是别的视图，`cx.notify()` 只是白排一次重绘，还会让"这次重绘是谁触发的"
/// 难以归因 —— 所以只在"当前显示的仍然是这次要扫的视图"时才通知。
///
/// 数据**不**受这个判断影响，照写缓存：`RightScanState::request` 里那个"一次"说的是扫描次数，
/// 不该因为用户在任务跑完前切走就丢掉；用户切回来时点击 / 菜单那条路自己会 `cx.notify()`，
/// 缓存里的数据照样会被画出来。
fn right_scan_should_notify(
    target: RightToolWindowView,
    current: RightToolWindowView,
    visible: bool,
) -> bool {
    visible && current == target
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
        // 段③ 的最近项目读的是**本实体**里的那一份（每次打开项目成功时更新 + 落盘）。
        let project_entries = self.project_entries();
        let project_menu = render_project_menu(
            &self.project_menu,
            &project_entries,
            self.recent_projects.entries(),
            window,
            cx,
        );
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
            move |index: usize, _event: &ClickEvent, window: &mut Window, cx: &mut App| {
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
            move |index: usize, event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                // 越界下标什么也不做：右栏只有三项，多出来的下标不该静默落到某一项上。
                let Some(clicked) = RightToolWindowView::from_rail_index(index) else {
                    return;
                };
                // 本次点击的位置：取 `ClickEvent::mouse_position()`（`activity_bar` 把事件原样传下来），
                // 它是**这次松手**的位置（`gpui-pre-0.3.6/src/interactive.rs:336-347`）。
                // ⚠️ 不用 `Window::mouse_position()`：键盘激活按钮（Enter / Space）时那个值是
                // 上一次指针位置，会给"两行坐标相同 ⇒ 同一次派发"这条判据塞进假证据；
                // `ClickEvent` 在那种情况下给 `None`，于是这一行干脆不带坐标。
                let point = event.mouse_position().map(ProbePoint::from);
                handle.update(cx, |this, cx| {
                    // 状态迁移是纯函数（[`resolve_right_click`]），与真机
                    // `resolveRightToolWindowUpdate` 逐条对应，单测直接覆盖它。
                    let (view, visible) =
                        resolve_right_click(clicked, this.right_view, this.right_visible);
                    this.right_view = view;
                    this.right_visible = visible;
                    // 第一次真正显示这个视图时才去取它的数据：这里只**登记**，
                    // 同步读盘被排到派发栈之外（判据与理由都在
                    // [`ShellWorkspace::schedule_right_view_scan`] 里，菜单项与 `--right-view` 同源）。
                    this.schedule_right_view_scan(view, visible, cx);
                    diagnose_right_panel(view, visible, point);
                    cx.notify();
                });
            }
        };

        // 面板头部的关闭按钮：只关可见性，**不动** `right_view`（真机 toggle 分支同理）。
        let on_close_right_activity = {
            let handle = handle.clone();
            move |event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                handle.update(cx, |this, cx| {
                    this.right_visible = false;
                    // 关闭按钮的坐标取 `ClickEvent::mouse_position()`：键盘激活时它是 `None`
                    // （gpui 只给了命中矩形，`interactive.rs:336-347`），那种情况就不打坐标 ——
                    // 编一个出来会污染"两行坐标相同 ⇒ 同一次派发"这条判据。
                    diagnose_right_panel(
                        this.right_view,
                        false,
                        event.mouse_position().map(ProbePoint::from),
                    );
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
            self.spring_index.as_ref(),
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
        // Spring（**本侧新增加的第四项**）：真机右栏没有这一项（Spring 能力留在语言服务里），
        // 图标取语义最近的 Lucide `leaf`（真源 `spring-icon.tsx` 是内联品牌 path，没有 SVG 文件）。
        ActivityItem::new(IconName::Leaf, tr("lithe.spring.title")),
    ]
}

/// 设置里的 `javaHomePath` → `lithe-gpui-java` 要的 JDK 覆盖值（**纯函数**，便于单测）。
///
/// **只读 `javaHomePath`**：另外两个键（`mavenExecutablePath` / `mavenJavaHomePath`）在 gpui 侧
/// 没有任何消费方 —— `maven.scan` 是 Core 进程内的项目描述符解析，本侧不执行 `mvn`
/// （见 `lithe_gpui_java::toolchain` 的模块文档）。登记了没人读只会把"存了不生效"从设置页
/// 搬到这一层，所以这里不登记它们。
///
/// 空串 / 全空白等于"没选"（设置页的「清空」写的就是空串，照 `settings/src/project.rs`
/// 的 `non_empty_opt` 同一条口径）；有值时去掉前后空白再转成路径。
fn java_toolchain_override_from(
    settings: &lithe_gpui_settings::Settings,
) -> Option<lithe_gpui_java::JavaToolchainOverride> {
    let trimmed = settings.java_home_path.trim();
    (!trimmed.is_empty()).then(|| lithe_gpui_java::JavaToolchainOverride {
        java_home: PathBuf::from(trimmed),
    })
}

/// 把设置里的 JDK 覆盖值登记给语言服务（调用点见 `ShellWorkspace::new`）。
///
/// 判据全在 [`java_toolchain_override_from`] 里；这里只做"读设置 → 登记"与一行启动证据。
/// 设置状态不存在时（测试或别的宿主里没有 `SettingsStore`）登记 `None` = 纯自动发现，
/// 与 `try_store` 在别处"没有设置就回落默认值"的口径一致，不 panic。
fn register_java_toolchain(cx: &App) {
    let overridden = lithe_gpui_settings::try_store(cx)
        .and_then(|store| java_toolchain_override_from(store.read(cx).settings()));
    println!(
        "S1_SETTINGS wiring=java_toolchain java_home_path={}",
        overridden
            .as_ref()
            .map(|overridden| overridden.java_home.display().to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    lithe_gpui_java::set_java_toolchain_override(overridden);
}

#[cfg(test)]
mod tests {
    use super::{
        ActionFlags, CommandId, COMMAND_ORDER, OpenDestination, ProjectOpenDecision, RightScanState,
        RightToolWindowView, left_activity_index, resolve_project_open_destination,
        right_scan_should_notify, visible_commands,
    };

    /// 换项目的**决策**逐条照真源 `chooseProjectOpenDestination`
    /// （`project-open-destination.ts:78-110`）：显式目的地优先 → 不询问时按偏好 → 否则弹对话框。
    ///
    /// 这条是"第一次打开项目要不要问""勾了不再询问之后还会不会弹"的唯一机器判据 ——
    /// 它错了不会编译失败，只会让对话框在不该出现的时候出现（或者永远不出现），
    /// 而两者都只有手动点才能发现。
    #[test]
    fn project_open_destination_follows_the_source_decision_table() {
        // ① 显式目的地直接用，且**不**改偏好（真源 `:82-84`）。
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

        // ③ 不询问：按 `openFoldersInNewWindow` 直接定（真源 `:90-96`）。
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

        // ④ 默认（询问 + 没给目的地）= 弹对话框。
        assert_eq!(resolve_project_open_destination(true, false, None), None);
        assert_eq!(resolve_project_open_destination(true, true, None), None);

        // 真源第 ② 段（`!hasOpenWorkspace`）在本侧不适用：外壳恒有一个工作区根 ——
        // 所以"询问"这一支永远走到对话框，不会被那条分支吃掉。
    }

    /// `--left-view` 的取值解析：只认顶部组那三个，未知取值返回 `None`（不静默落到某一项）。
    #[test]
    fn left_view_selectors_map_to_top_group_indices() {
        assert_eq!(left_activity_index("files"), Some(0));
        assert_eq!(left_activity_index("project"), Some(0));
        assert_eq!(left_activity_index("changes"), Some(1));
        assert_eq!(left_activity_index("search"), Some(2));
        // 底部组（运行 / 终端 / 诊断 / 提交记录）切的是底部工具窗，不是"左栏显示谁"。
        assert_eq!(left_activity_index("terminal"), None);
        assert_eq!(left_activity_index("nope"), None);
    }

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

    /// 设置里的 `javaHomePath` → 交给语言服务的 JDK 覆盖值。
    ///
    /// 这是"设置 → JDT LS 用哪个 JDK"这条注入链在本 crate 里的唯一判据，所以逐种写法钉住：
    /// 空串 / 全空白 = 没选（`None`，语言服务退回自动发现），有值 = 去空白后的路径。
    /// 判据的另一半（覆盖值真的压过自动发现）由 `lithe-gpui-java` 的单测与实机 `S1_JAVA_JDTLS`
    /// 日志守。
    #[test]
    fn java_toolchain_override_follows_java_home_path() {
        use lithe_gpui_settings::Settings;

        let mut settings = Settings::default();
        assert_eq!(
            super::java_toolchain_override_from(&settings),
            None,
            "默认的空串 = 没选覆盖值"
        );

        settings.java_home_path = "   ".to_string();
        assert_eq!(
            super::java_toolchain_override_from(&settings),
            None,
            "全空白同样等于没选（不能变成 `PathBuf::from(\"\")` 这种「当前目录」）"
        );

        settings.java_home_path = " C:\\tools\\jdk-21 ".to_string();
        let overridden =
            super::java_toolchain_override_from(&settings).expect("填了值就必须给出覆盖值");
        assert_eq!(
            overridden.java_home,
            std::path::PathBuf::from("C:\\tools\\jdk-21"),
            "前后空白必须去掉（带空白的路径会被 java 侧判成无效）"
        );
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
    ///
    /// ⚠️ 表里**只列命令面板里的那七条**（它们才有"面板行序"这层含义）；B1 新增的
    /// 「打开文件」与七条关闭系也有 `id()`，但它们的入口只有主菜单 ——
    /// 那八条的 id 由 [`menu_only_command_ids_are_stable`] 钉住。
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

    /// B1 新增的**只走主菜单**的八条 `CommandId`：id 必须稳定（诊断契约）、互不相同，
    /// 而且**一条都不在命令面板的可见表里** —— 那正是"它们不占面板行序"的机器判据
    /// （面板的行号含义由 [`visible_commands`] 算，多进去一条就会让别处错位一行）。
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
        // 与本表之外的 id 也不能撞（面板那七条 + 三条只由快捷键/菜单触发的）。
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
            CommandId::NavigateToDefinition,
        ] {
            ids.push(other.id());
        }
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "全部 CommandId 的 id 必须互不相同");
    }

    /// 懒扫**延后之后**仍然"每个视图只扫一次"：登记在派发栈内就落下，所以同一项连点两次
    /// （或任务还没跑完就切回来）也不会排第二个任务。
    ///
    /// ⚠️ 这条覆盖的是 [`RightScanState::request`]（纯值），也就是
    /// `ShellWorkspace::schedule_right_view_scan` 在点击回调里**唯一**还会做的事。
    /// 它**不能**证明"读盘不在派发栈里" —— 那一半要 gpui 宿主，而本 crate 拿不到
    /// `TestAppContext` / `#[gpui::test]`（同一处限制在 `menu_bar.rs` 里已经记过两次），
    /// 所以只由代码结构保证：生产路径上 `crate::maven::scan` / `crate::spring::index`
    /// 各只有一处调用点，都在 `cx.spawn` 的任务体内。
    #[test]
    fn scan_registration_happens_once_and_only_while_visible() {
        let mut state = RightScanState::default();

        // 收起时点一项：不算"显示过"，不消费登记额度。
        assert_eq!(state.request(RightToolWindowView::Maven, false), None);
        // 第一次真正显示 → 排一次扫描。
        assert_eq!(
            state.request(RightToolWindowView::Maven, true),
            Some(RightToolWindowView::Maven)
        );
        // 再点同一项（收起后再点开 / 延后的任务还没跑完就切回来）→ 不排第二次。
        assert_eq!(state.request(RightToolWindowView::Maven, true), None);
        // 收起也不影响"已经登记过"这个事实。
        assert_eq!(state.request(RightToolWindowView::Maven, false), None);

        // Spring 是**另一格**：Maven 扫过不影响它，且它自己同样只排一次。
        assert_eq!(
            state.request(RightToolWindowView::Spring, true),
            Some(RightToolWindowView::Spring)
        );
        assert_eq!(state.request(RightToolWindowView::Spring, true), None);

        // 没有数据层的两个视图永远不排任务（面板是纯空态）。
        assert_eq!(state.request(RightToolWindowView::Extensions, true), None);
        assert_eq!(state.request(RightToolWindowView::Notifications, true), None);
    }

    /// 延后任务的收尾策略：**当前显示的仍是这次要扫的视图**才重画；切走 / 收起之后不通知
    /// （数据照写缓存，那一步在 `schedule_right_view_scan` 里，与这个判断无关）。
    #[test]
    fn scan_completion_notifies_only_while_the_target_view_is_shown() {
        // 用户就停在这一项上 → 数据到了要重画。
        assert!(right_scan_should_notify(
            RightToolWindowView::Maven,
            RightToolWindowView::Maven,
            true,
        ));
        // 已经切到别的视图 → 这一帧画的是别人，重画没有意义。
        assert!(!right_scan_should_notify(
            RightToolWindowView::Maven,
            RightToolWindowView::Spring,
            true,
        ));
        // 面板已经收起 → 同样不通知（`right_view` 仍保留 Maven 也不算"显示"）。
        assert!(!right_scan_should_notify(
            RightToolWindowView::Maven,
            RightToolWindowView::Maven,
            false,
        ));
    }
}
