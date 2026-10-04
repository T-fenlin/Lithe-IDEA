//! 工作台骨架：把外壳各区域组装成 Windows 的三层结构。
//!
//! 结构（逐值照 `windows/tauri/src/features/layout/components/main-layout.tsx`，度量见
//! `lithe-gpui/UI-MAP-WINDOWS.md` §1.1 与 `lithe-gpui/research/windows/01-shell.md` §2）：
//!
//! ```text
//! v_flex
//! ├─ 标题栏单行 40        LitheTheme.swift:478       2026-09-30 由"两行 30+38"回到真源单行
//! ├─ 项目标签条 32        project-tab-bar.tsx:41     h-8
//! ├─ 工作区 h_flex flex-1  main-layout.tsx:299       行尾 pr = 4（--lithe-workbench-gap）
//! │  ├─ 左活动栏 40+4      真源 WorkbenchView.swift:12  折叠态图标竖条
//! │  ├─ 左侧栏 320         真源 WorkbenchLayoutStore.swift:21  可拖：30…min(520, 容器 − 400)
//! │  ├─ 中央列 flex-1      本侧 v_flex 结构           v_flex[ 编辑器岛, 底部工具窗 260 ]
//! │  ├─ 右侧工具窗 360    真源 WorkbenchLayoutStore.swift:5    默认宽（**可收起**）
//! │  └─ 右活动栏 40+4     真源 WorkbenchView.swift:7,13,1415
//! └─ 状态栏 24            footer.tsx:45-49          24 = --lithe-footer-height（theme.css:119）
//! ```
//!
//! ⚠️ **顶部标题栏是单行 40**（真源 `LitheTheme.Metrics.toolbarHeight`，见 `crate::title_bar`）；
//! 2026-09-27 那版"两行 30 + 38"已按真源回退。
//!
//! **底部窗不横跨工作台**（维护者 2026-09-25 拍板）：它挂在**中央列**里，是编辑器岛的下一个
//! flex 兄弟。高度真源是**可拖的纵向分割**：最小 **260**
//! （`macos/.../Views/Workbench/WorkbenchView.swift:2019` 的 `minimumGitPaneHeight = 260`，
//! 与同处 `:2018` 的 `minimumTopPaneHeight = 220` 一起定义可拖区间）。
//! **本侧取 260**（[`BOTTOM_PANE_HEIGHT_SPEC`]）：2026-10-03 维护者裁决"U25 遗留 4 条全部按真源改"，
//! 把原先那个 240 对齐到真源最小值 —— **视觉无变化**，因为 clamp 早就把它强制到 260
//! （理由与证据写在 [`BOTTOM_PANE_HEIGHT_SPEC`] 的文档里）。
//! 本版用普通 `v_flex` + 固定高度表达这个分栏，**不用** gpui-kit 的 Dock：本侧的中央列
//! 就是 flex + 可拖手柄，没有 dock 系统，用 Dock 反而会多出一条真源没有的标签头。
//! 唯一的例外是**分隔条**：三条可拖分界线用的都是
//! Dock 同款的上游手柄（[`drag_handle`]，2026-09-30，`docs/gpui kit/0.7.0/component/dock.md`）。
//!
//! 四个区域模块（[`crate::title_bar`] / [`crate::project_tabs`] / [`crate::activity_bar`] /
//! [`crate::status_bar`]）是**无状态渲染函数**；需要持有 `Entity` 的区域
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
//! `App::update` 收尾的 `flush_effects` 里级联 drop（侦察 `lithe-gpui/research/menu-open-prereqs.md`
//! §2），`EditorPane::drop` / `TerminalPane::drop` 各自收尾。⚠️ 但 `java::Session` 的 `Drop`
//! **只停事件泵、不关 JVM** —— 所以换根后"无残留进程"必须**实测**（B4 验收线），
//! 不能只看代码路径。

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
    /// `--right-view extensions`：本次会话**打开中央扩展页**（**验证/诊断用**）。
    ///
    /// 扩展页不是右工具窗视图（`RightToolWindowView::from_id("extensions")` 是 `None`），
    /// 所以单独一个开关而不是复用 `right_view`；应用时机与换根语义与 `right_view` 相同
    /// （每个重建出来的外壳都应用一次）。
    extensions_page: bool,
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

/// 启动期形态里的菜单栏模式（重建路径读它）。
fn startup_compact_menu(cx: &App) -> bool {
    cx.try_global::<ShellStartup>()
        .map(|startup| startup.compact_menu)
        .unwrap_or(false)
}

/// 启动期请求的右 / 左栏视图与扩展页（重建路径也要应用，理由见 [`ShellStartup`] 的字段文档）。
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

/// 把窗口根换成 `target` 上的新外壳：`window.replace_root` + 新的 `Root`（**窗口根必须是
/// `Root`** —— 它是对话框 / 浮层 / 通知的宿主，`crates/app/src/main.rs` 同一句）。
///
/// 两个调用点共用它，被绕开的差别只在调用方那一侧：
///
/// - [`ShellWorkspace::rebuild_project_window`]（换项目）：旧根要先"放掉索引缓存 + 写会话"；
/// - [`crate::welcome::WorkspaceOpener`]（欢迎页点一条项目）：**还没有**旧外壳，所以没有
///   那两步 —— 换根之后要补的那两步（记最近项目 / 写「不再询问」偏好）也各自在调用方。
///
/// 菜单栏形态在 `ShellStartup` 这个 `App::Global` 里（应用级启动参数，`main.rs` 的局部
/// 变量活不到这里），所以本函数自己取一次（`startup_compact_menu`）。
///
/// 返回 `None` 只可能发生在 `replace_root` 的实现被换掉时（它今天同步执行 `build_view`，
/// `gpui-pre-0.3.6/src/window.rs:2242-2254`，所以闭包**一定**跑过）。调用方按既有口径打
/// 一行 `state=rebuild_failed reason=no_view` 的诊断，不 panic。
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
    // `replace_root` 已经把新 `Root` 装成窗口根；这里只留一行回执，
    // 免得读者以为返回值没用上。
    let _ = new_root;
    created
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
/// | ② `!hasOpenWorkspace` → `this-window`，不询问 | **只在欢迎页成立**（U10 之前本侧 `ShellWorkspace` 恒有一个工作区根，所以这一支一直是"不适用"）。欢迎页那一侧不经过本函数：`crate::welcome::WorkspaceOpener` 直接照这一段定死"此窗口、不询问"（宿主又没有多窗口，见 `report_new_window_not_wired`） |
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
/// ⚠️ 落点在这里（外壳）而不是编辑区，与 `lithe-gpui/crates/explorer/src/lib.rs:93-94`
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
            vec![path.parent().unwrap_or(path).display().to_string()],
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
/// 真机里「设置」打开的是**模态对话框**，不是底部工具窗（`lithe-gpui/research/windows/07-settings-ui.md`
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
/// 选中它时左栏画 [`crate::search_panel::SearchPanel`]（U17 第一件，2026-10-03 起；
/// 在此之前点它只切选中态、内容仍是项目树）。
const SEARCH_ACTIVITY_IX: usize = 2;

/// 右侧活动栏「扩展」拼图项的下标 = 0。
///
/// ⚠️ 2026-09-29 起这一项**不是右工具窗视图**：它开/关**中央扩展页**
/// （[`ShellWorkspace::toggle_extensions_page`]，对齐真机 `openExtensionsBuffer` 的口径）。
/// [`RightToolWindowView::from_rail_index`] 因此对下标 0 返回 `None`，点击处理在
/// 进入右栏状态机**之前**先拦截这个下标。
const EXTENSIONS_RAIL_IX: usize = 0;

/// 默认选中的顶部组视图：第 0 项「项目」。
///
/// 真源 `features/window/stores/workspace-ui-defaults.ts:7` 的 `activeSidebarView: "files"`
/// —— 启动时左活动栏顶部第一项就是选中态。
const DEFAULT_TOP_ACTIVITY: usize = 0;

/// 左侧栏面板的**初始**宽度（规格像素，320 = 真源 `sidebarWidth`）。
///
/// ⚠️ 这只是**初值**：2026-09-27 起运行期可以拖（[`drag_handle`] 开始拖拽会话，
/// [`drag_tracker`] 写 [`ShellWorkspace::left_sidebar_width`]，范围
/// [`SIDEBAR_MIN_WIDTH_SPEC`] … [`SIDEBAR_MAX_WIDTH_SPEC`]）。维护者口径：
/// 「workspace.rs 我还是改为 320 了，到时候弄成拖动改宽窄」—— 即恢复真源默认值，宽度交给用户拖。
///
/// **不持久化**：会话文档（`workspace_config::session`）只有可见性字段，还没有宽度字段；
/// 加字段要动那边手写的 `Deserialize`，属独立一批（登记在 `lithe-gpui/docs/ui-mockup-idea.md`）。
///
/// **320 在 gpui 的 rem 档位上**（`w_80()`），但仍走 `rems(SIDEBAR_WIDTH_SPEC / 16.)`
/// 表达：宽度现在是**变量**（拖拽会改），用 helper 只会把常量与变量两种写法混在一起。
const SIDEBAR_WIDTH_SPEC: f32 = 320.;

// 工作区各岛之间的间距是 **4**（真源 `--lithe-workbench-gap: 4px`，`styles/theme.css:125`），
// 在 render 里由 `gap_1()` / `pr_1()` 表达 —— 度量是 rem 档位上的 1，不需要另立常量。
//
// ⚠️ **`WORKBENCH_GAP_SPEC` / 负外边距（2026-09-30 删除）**：此前左侧栏与编辑区之间那 4px
// 要按维护者口径"吃成直接连接"，靠的是 `rems(-WORKBENCH_GAP_SPEC / 16.)` 经 `side_pane` 的
// `.ml()/.mr()` 抵消父级 `gap_1()`。那违反了《设计指南》
// （`docs/gpui kit/0.7.0/docs/design-guides.md:188`「不能用 raw pixel offset 修补」、
// `:196`「Hairline 由边界所有者绘制，相邻区域不能各画一次同一条分隔线」）。
// 现在"直接连接"由 render 里那个 **`gap_0` 的内层组**（共同 owner）表达，边界只由
// `editor_island` 的左边框**一个所有者**画，`side_pane` 也不再收水平外边距参数。

/// 左栏宽度的**下界** 30。
///
/// 真源（macOS）：`macos/Sources/Lithe/Services/Workbench/WorkbenchLayoutStore.swift:4`
/// 的 `static let minimumSidebarWidth: Double = 30`；应用点是
/// `macos/Sources/Lithe/Views/Workbench/WorkbenchView.swift:2006` 的
/// `let minimumSidebarWidth = CGFloat(WorkbenchLayout.minimumSidebarWidth)`，
/// 它同时是 `:2014` 的 `constrained(minimum:)` 与 `:2037` 分栏的 `minimum`。
///
/// 改前本侧是 **200**（2026-09-27 维护者口径："再窄就连 `domain` 这样的目录名都放不下"）。
/// **2026-10-03 维护者裁决**：`U25` 遗留的 4 条全部按真源改 → 改成 30。这是**可见改动**
/// （能拖的范围从 200 起变到 30 起），依据就是上面两行真源。
const SIDEBAR_MIN_WIDTH_SPEC: f32 = 30.;

/// 左栏宽度的**静网上界** 520。
///
/// 真源（macOS）：`WorkbenchView.swift:2010` 的 `min(520, availableTopWidth - minimumEditorWidth)`；
/// `WorkbenchLayoutStore.swift:32` 的 `layout.sidebarWidth <= 520` 同样以 520 为合法上界
/// （越界文档直接判为无效、回落默认布局）。真正的上界按分栏容器宽收敛，见
/// [`left_sidebar_max_width`]。
///
/// 改前本侧是静网 **720**（`pane_span_range` 直接拿它当上界、没有自适应）→ 2026-10-03
/// 按真源改成 520（与 `pane_span_range` 的自适应一起落地）。
const SIDEBAR_MAX_WIDTH_SPEC: f32 = 520.;

/// 拖左栏时给**同一横向分栏里编辑区那一半**留的最小宽度 400。
///
/// 真源（macOS）：`WorkbenchView.swift:2007` 的 `let minimumEditorWidth: CGFloat = 400`，
/// 用在 `:2010` 的 `availableTopWidth - minimumEditorWidth`。
///
/// 这条回答"400 是什么"：**不是"给右侧栏保留的宽"**，而是"给分栏里 `flexible` 的那半
/// （编辑区）保留的宽" —— 它的近邻是 `LitheSplitPaneView` 的 `sized:`（侧栏）与
/// `flexible:`（编辑区），`WorkbenchView.swift:2033-2062`。
const SIDEBAR_MIN_EDITOR_WIDTH_SPEC: f32 = 400.;

// ---------------------------------------------------------------------------
// 度量：应用布局一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// rem base = 主题字号 16px，所以 helper 后缀 `N` = `N × 4px`，与 Windows 规格逐像素相等：
//
// | 规格（macOS 真源） | 值 | 用到的 helper |
// | --- | --- | --- |
// | `sidebarWidth: 320`（`WorkbenchLayoutStore.swift:21` 的 `WorkbenchLayout(sidebarWidth: 320, …)`） | 320 | `rems(SIDEBAR_WIDTH_SPEC / 16.)` |
// | `--lithe-workbench-gap: 4px`（`styles/theme.css:125`） | 4 | `gap_1()` / `pr_1()` / `h_1()` |
// | 底部工具窗高 **260**（真源**最小高**，`WorkbenchView.swift:2019` 的 `minimumGitPaneHeight`） | 260 | `rems(BOTTOM_PANE_HEIGHT_SPEC / 16.)` |
// | 区域占位块内边距 8 | 8 | `p_2()` |
// | 占位块字号 13（`--ui-text-chrome`） | 13 | `text_sm()`（14px，见下） |
//
// **左侧栏默认宽 320**：2026-09-27 一度改到 380，同日后按维护者口径
// 「workspace.rs 我还是改为 320 了」回退 —— 320 也正是 macOS 真源的默认
// （`WorkbenchLayoutStore.swift:21`），见 [`SIDEBAR_WIDTH_SPEC`]。
//
// 字号：13 不在 gpui 的档位（`text_xs()`=12 / `text_sm()`=14）上，按《编码指南》用
// `text_sm()`（14px）——13 → 14 是经维护者确认的**有意**视觉改动，不是等价换算。

/// 右侧工具窗的**默认宽度 360**（真源 `WorkbenchLayoutStore.swift:5` 的
/// `static let defaultMavenPaneWidth: Double = 360`）。
///
/// ⚠️ **它取代的 400 出自已作废的 `windows/tauri`**：400 的原出处是
/// `features/settings/config/default-settings.ts:139` 的 `rightToolWindowWidth: 400`
/// （区间 140–600 见 `features/settings/lib/settings-normalization.ts:114-115`），
/// 而那份 Windows 前端**已从仓库删除**，`develop-lithe` 明令不许再引用它的任何数值。
/// **2026-10-03 维护者裁决**：既然 400 没有有效出处、真源是 360，就按真源改成 360。
/// 这是**可见改动**（右工具窗第一次展开时的宽度 400 → 360）。
///
/// ⚠️ **默认值与持久化值的关系**：这个常量只在**没有持久化值**时生效 ——
/// `right_tool_window_width` 的初值取它，而 [`ShellWorkspace::restore_session`] 只在
/// 会话文档里 `rightPaneWidth` 为 `Some` 时才覆盖（`Option<f32>`，缺键 = `None` =
/// 保留外壳默认值，语义见 `crates/shared/src/workspace_config/session.rs:117-125`）。
/// 所以**用户已经存过的宽度不会被 360 覆盖**。
///
/// ⚠️ **不用 `px(...)`**：360 不在 gpui 的 rem 档位上（档位里 88 → 352、96 → 384，
/// `gpui-pre-macros-0.3.6/src/styles.rs:1063-1077`），也不能自己发明一个 `w_90()`。
/// 按《编码指南》"档位外的值用 helper 底层的 `rems(P / 16.)`"写成 rem：基准 16px 时
/// `rems(360. / 16.)` 与 `px(360.)` 逐像素相等（22.5rem × 16 = 360），但会随主题字号缩放。
const RIGHT_TOOL_WINDOW_WIDTH_SPEC: f32 = 360.;

/// 右侧工具窗的**下界** 300（真源 `WorkbenchLayoutStore.swift:6` 的
/// `minimumMavenPaneWidth = 300`）。
///
/// 拖拽与恢复越界值时都钳到它 —— 再窄就放不下 Maven 树的"GroupId/ArtifactId"两级缩进。
const RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC: f32 = 300.;

/// 右侧工具窗的**静网上界** 760（真源 `WorkbenchLayoutStore.swift:7` 的
/// `maximumMavenPaneWidth = 760`）。真正的上界还要按可用宽度收敛，见
/// [`right_tool_window_max_width`]。
const RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC: f32 = 760.;

/// 拖右侧工具窗时给**中间那片工作区**留的最小宽度 640（真源
/// `macos/.../Views/Workbench/WorkbenchRightToolSplitView.swift:5` 的
/// `minimumWorkspaceWidth = 640`，`maximumWidth(in:)` 用 `availableWidth - 640 - 热区厚`）。
///
/// 本侧量的是**窗口内宽度**而不是真源那个分栏容器（见 [`right_tool_window_max_width`]），
/// 所以留白比真源略松 —— 是近似，不是真源。
const RIGHT_TOOL_WINDOW_MIN_WORKSPACE_WIDTH_SPEC: f32 = 640.;

/// 底部工具窗的**下界** 260（真源 `macos/.../Views/Workbench/WorkbenchView.swift:2019` 的
/// `minimumGitPaneHeight = 260` —— 纵向分割里它正是留给底部工具窗的那一段）。
const BOTTOM_PANE_MIN_HEIGHT_SPEC: f32 = 260.;

/// 拖底部工具窗时给**中央列上面那片**留的最小高度 220（真源
/// `WorkbenchView.swift:2018` 的 `minimumTopPaneHeight = 220`，`:2020-2025` 的
/// `maximumTopPaneHeight = height - 间距 - 260` 就是"两边各留自己的下界"）。
const BOTTOM_PANE_MIN_CENTER_HEIGHT_SPEC: f32 = 220.;

// `ISLAND_RADIUS_SPEC = 11.2`（真源 `theme.css:9,134` 的 `rounded-xl = --radius × 1.4`）**2026-09-30 删除**。
//
// 真源那一层 `lithe-glass-island` 有自己的底色，圆角是个看得见的形体；本侧的岛底色与外层同色
// （都是 `theme.background`），圆角就只剩边框上的一段弧 —— 它和相邻的直线接不上，维护者截图
// 指认为"竖线顶端多出来的那一小截弯线"，要求改成直角。三个岛（`side_pane` / `editor_island` /
// `bottom_pane`）现在都不写 `.rounded(..)`，四条边框都是直线。
//
// 真源仍是 `rounded-xl`；要恢复圆角，先让岛的底色与外层不同（真源那层是 `lithe-glass-island`），
// 否则回来还是同一段弧。

/// 底部工具窗（终端 / Git 提交记录 / 占位）的**默认高度 260**。
///
/// ## 2026-10-03 维护者裁决：240 → 260（按真源；**视觉无变化**）
///
/// 这个值现在**等于真源的最小高**：`macos/.../Views/Workbench/WorkbenchView.swift:2019`
/// 的 `let minimumGitPaneHeight: CGFloat = 260`（同处 `:2018` 的 `minimumTopPaneHeight = 220`
/// 是"上面那片"的下界，两份一起定义可拖区间）。维护者裁决 `U25` 遗留的 4 条全部按真源改，
/// 于是默认值取真源的最小值 —— 即"默认就停在它最小能到的位置"，不再自造第三个数字。
///
/// ⚠️ **为什么这条是视觉无变化的**：`BOTTOM_PANE_HEIGHT_SPEC` 与
/// [`BOTTOM_PANE_MIN_HEIGHT_SPEC`] 是**同一个值**（260），而后者早就被
/// [`pane_span_range`] 当作下界、在**恢复会话**与**每一帧 render** 上各钳一次 ——
/// 改前那个 240 一次都没能画出来过（首帧就被钳到 260）。改后 clamp 在默认值上
/// **恒等**（`clamp_pane_span(260, 260, …) == 260`，有单测钉住），所以"只剩一个真源值
/// 在起作用"：240 那条墓碑到此为止。
///
/// ## 240 的来历（墓碑，已作废）
///
/// 2026-09-27 标题栏按效果图口径改成两行 30 + 38 = 68（比单行 40 净增 28），窗口总高不变的
/// 前提下从底部工具窗让出 320 → 240。那 28px 的挤压前提已随标题栏 2026-09-30 回到单行 40
/// 而消失；2026-10-03 维护者裁决按真源改到 260，**240 不再有任何作用**（只在
/// `U25` 的报告与本节留下这段记录）。原先留下的"暂不还给底部工具窗"那段话因此删除 ——
/// 它已经是自相矛盾的墓碑：既说"没有判据可抄"，又说"维持 240"。
///
/// ⚠️ 260 不在 gpui 的 4px 档位上（档位里 56 → 224、64 → 256，没有 65，
/// `gpui-pre-macros-0.3.7/src/styles.rs`），所以不用 `h_65()`（它不存在），
/// 按《编码指南》写 `rems(BOTTOM_PANE_HEIGHT_SPEC / 16.)`（16px 基准下 = 260，且随字号缩放）。
const BOTTOM_PANE_HEIGHT_SPEC: f32 = 260.;

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
    /// **调试工具窗（U19）**：断点模型 + 调用栈 / 变量树 / 监视 / 求值 / 单步控制。
    ///
    /// 为什么进底部工具窗而不是右工具窗：真源的调试面板是**自己一个工具窗**
    /// （`Views/Debug/GenericDebugView.swift` 的 header 带「隐藏」，`:151-155`），
    /// 而宿主里"带隐藏按钮、由活动栏切换、高度可拖"的那一套只有底部工具窗有；
    /// 右工具窗的三个视图（扩展 / 通知 / Maven）都是**窄栏**形态（默认 360 宽），
    /// 放不下真源那个左右两栏（min 240 + min 300，`:611-617`）的检查器。
    Debug,
}

impl BottomPaneKind {
    /// 稳定 id：会话文档（`.lithe/session.local.json` 的 `bottomKind`）与诊断行用它。
    ///
    /// 为什么不直接 `serde` 派生到 `BottomPaneKind` 上：那个枚举是**界面私有**类型
    /// （`BottomPaneKind` 不上任何对外契约），而文档层要的是"一个不随 Rust 标识符改名的
    /// 字符串"—— 把这两件事绑在一起，改个枚举名就会悄悄改掉磁盘上的取值。
    /// 两个方向的映射由 [`bottom_kind_from_id`] 与这里**同处一个文件**，改一处必须改两处。
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

/// 会话文档里的 `bottomKind` 字符串 → 底部窗内容；认不出的取值返回 `None`。
///
/// **认不出就返回 `None`**（调用方保留当前值）而不是落到某一项：会话文件是可丢弃的本机文档，
/// 里面可能是别的版本写的 id，静默落到「终端」会让用户看到一个自己没开过的面板。
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

/// 活动栏第 `index` 项对应的底部窗内容；`None` = 该项不换底部窗。
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
/// 在 gpui 侧还不存在，理由见 `lithe-gpui/PLAN.md` 的阶段 6 第二半。
pub(crate) fn command_action(id: CommandId, flags: ActionFlags, is_dark: bool) -> CommandAction {
    use CommandId as C;
    // 只有命令面板里真的会出现的动作才需要分类与图标。
    // [`C::SaveBuffer`] / [`C::OpenCommandPalette`] / [`C::NavigateToDefinition`] /
    // [`C::ToggleMenuBar`] 这四条**不在面板里**（它们的入口是菜单项与快捷键，
    // 见 `COMMAND_ORDER` 的说明），所以给它们一个中性分类与图标而不是把它们排除在
    // 动作表之外 —— 否则菜单项就得自己再写一份 label / description。
    let (category, icon) = match id {
        C::OpenSettings | C::OpenAppearanceSettings => (
            Category::Settings,
            idea_icon_mono(&idea::GEAR_ICON, is_dark),
        ),
        // 主题动作在两套界面里都用「调色板」字形（真源 `settings-actions.tsx` 的 `PaletteIcon`）。
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
        // 「快速打开」同样**不在面板里**（`COMMAND_ORDER` 没有它，入口是 `Ctrl+P` 与主菜单）。
        // 图标与菜单那一份同源（`MenuAction::icon` 也是放大镜）。
        C::OpenQuickOpen => (
            Category::View,
            idea_icon_mono(&idea::MAGNIFYING_GLASS_ICON, is_dark),
        ),
        // 「全局搜索」同样**不在面板里**（`COMMAND_ORDER` 没有它，入口只有主菜单）。
        // 图标与菜单那一份同源（`MenuAction::SearchEverywhere` 是地球）。
        C::OpenSearchEverywhere => (
            Category::View,
            idea_icon_mono(&idea::GLOBE_HEMISPHERE_WEST_ICON, is_dark),
        ),
        // 「在文件中替换」同样**不在面板里**（`COMMAND_ORDER` 没有它，入口是菜单与
        // `Ctrl+Shift+R`）。图标与菜单那一份同源（`MenuAction::ReplaceInFiles` 用的是
        // 与「编辑 → 查找并替换」同一个替换字形）。
        C::OpenReplaceInFiles => (Category::View, Icon::new(IconName::Replace)),
        C::NavigateToDefinition => (
            Category::View,
            idea_icon_mono(&idea::ARROW_RIGHT_ICON, is_dark),
        ),
        // B1 的八条同样**不在面板里**（`COMMAND_ORDER` 没有它们，入口只有主菜单「文件」）。
        // 它们的可见图标在 `MenuAction::icon`（菜单那一份才是用户看得到的），
        // 这里给同一批字形只为保持"两处入口视觉同源"这条既有口径。
        C::OpenFile => (Category::View, idea_icon_mono(&idea::FOLDER_ICON, is_dark)),
        C::CloseTab
        | C::CloseOtherTabs
        | C::CloseAllTabs
        | C::CloseSavedTabs
        | C::CloseTabsToLeft
        | C::CloseTabsToRight => (Category::View, idea_icon_mono(&idea::X_ICON, is_dark)),
        C::ReopenClosedTab => (Category::View, idea_icon_mono(&idea::REFRESH_ICON, is_dark)),
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
        // 「快速打开」：label 与菜单项**同一个键**（`lithe.menu.quickOpen`），description 取
        // 顶级菜单名「转到」—— 与上面四条同一条口径（它也不会被画进面板）。
        C::OpenQuickOpen => (tr("lithe.menu.quickOpen"), tr("lithe.menu.go")),
        // 「全局搜索」：label 与菜单项**同一个键**（`lithe.menu.globalSearch`），description 取
        // 顶级菜单名「视图」—— 与上面几条同一口径（它也不会被画进面板）。
        C::OpenSearchEverywhere => (tr("lithe.menu.globalSearch"), tr("lithe.menu.view")),
        // 「在文件中替换」：label 与菜单项**同一个键**（`lithe.search.gpui.replaceInFiles`，
        // gpui 自有键，中文逐字取自 `macos/Resources/zh-Hans.lproj/Localizable.strings:65`），
        // description 取它所在的顶级菜单名「转到」—— 与上面几条同一口径（它也不会被画进面板）。
        C::OpenReplaceInFiles => (tr("lithe.search.gpui.replaceInFiles"), tr("lithe.menu.go")),
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
    /// 项目标签条的**横向滚动**状态（`ScrollHandle`）。
    ///
    /// 归外壳持有而不是 `crate::project_tabs`：那个模块是无状态的，而滚动状态必须跨帧活着，
    /// 而且切换项目后要由这里（回调里）把活动项滚进可视区 —— 真源
    /// `WorkbenchView.swift:640-650` 的 `proxy.scrollTo(.., anchor: .center)` 就是这个动作。
    /// ⚠️ **不在 `render` 里滚**：`render` 只登记 `track_scroll`，滚动请求只从事件回调发出。
    project_tabs_scroll: ScrollHandle,
    /// 工作区根目录。
    ///
    /// ⚠️ 状态栏的 Git 分支项**不再**从这里现读（D11 之前每帧读 `.git/HEAD`）：分支取自
    /// [`ChangesView::branch`] 的 `git.status` 快照，见 [`ShellWorkspace::footer_left`]。
    /// 根目录仍是本视图的其它用途（打开项目、相对路径换算）所需要的。
    root: PathBuf,
    /// 搜索索引的三条 Core 接线（`workspace.searchIndex.warm` / `.update` / `.invalidate`）。
    ///
    /// 与外壳同生命周期：换根＝重建外壳＝换一份接线（所以代次天然从新根的 0 起）。
    /// 时机、请求形状、代次与诊断口径全在 [`crate::search_index`] 的模块文档里，这里只持有。
    ///
    /// `pub(crate)` 而不是私有：`crate::search_index` 排出的后台任务回来时要**读当前代次**
    /// （`accepts`）决定采纳还是丢弃，而这件事必须看到外壳**当下**的那一份接线 ——
    /// 排任务时克隆一份带走正是代次保护要防的错误。
    ///
    /// 它还持有**索引就绪状态**（`U18` 收尾）：`warm` 的结果被采纳后，外壳从这里读一次并
    /// 推给四个搜索界面（[`ShellWorkspace::broadcast_search_index_readiness`]）——
    /// 界面不轮询、不另建状态，见 `crate::search_index` 的模块文档「就绪状态」。
    pub(crate) search_index: SearchIndexWiring,
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
    /// **左侧整条栏**（活动栏 + 左面板）是否显示。**默认 `true`**。
    ///
    /// B2 加的：真源「视图 → 切换活动侧栏」（`menu.toggleActivitySidebar`，`mod+b`）与
    /// VS Code 的 `workbench.action.toggleSidebarVisibility` 同义 —— 收起的是
    /// **活动栏 + 面板**这一整块，不是只收起面板。本侧在此之前左栏**常驻、没有收起态**
    /// （`command_action` 的文档曾把 `toggle-sidebar` 明确排除在外），B2 把它补上：
    /// 状态只有这一个 bool，渲染时整块不画（与右工具窗 `right_visible` 同一套做法）。
    left_sidebar_visible: bool,
    /// **左侧栏那块面板**（项目树 / 更改）单独是否显示。**默认 `true`**。
    ///
    /// 2026-09-27 新增，把"活动栏"和"面板"拆成两个状态 —— 维护者口径是 **IDEA 语义**：
    ///
    /// - **活动栏常驻**（只要 [`ShellWorkspace::left_sidebar_visible`] 为真就画，不随面板收起而消失）；
    /// - **点活动栏当前高亮的那一项 = 只收/放面板**（`left_pane_visible` 取反），点**别的**项 = 切视图
    ///   并把面板放出来。判据都落在 [`ShellWorkspace::select_top_activity`] 一处；
    /// - `Ctrl+B` / 菜单「视图 → 切换活动侧栏」仍然是**整块收**（活动栏 + 面板），
    ///   那是给"想要最大编辑区"的人用的，语义不变。
    ///
    /// 真机的对应关系：`isSidebarVisible`（整块）与 `activePaneId`（哪一页）之间并没有"面板收起"
    /// 这个第三态；IDEA 是有的（工具窗按钮按下/弹起）。这是本侧**有意照 IDEA 而非 Windows 真源**
    /// 的一处，理由就是维护者那句"点击图标无法关闭项目的这个界面"。
    left_pane_visible: bool,
    /// 左侧栏面板宽度（规格像素，见 [`SIDEBAR_WIDTH_SPEC`] 与 [`SIDEBAR_MIN_WIDTH_SPEC`] /
    /// [`SIDEBAR_MAX_WIDTH_SPEC`]）。
    ///
    /// 允许区间 = `pane_span_range`：下界 30、上界 `min(520, 分栏容器宽 − 400)`
    /// （真源 `WorkbenchLayoutStore.swift:4` / `WorkbenchView.swift:2008-2011`）。
    ///
    /// 运行期可改：拖面板右边界那条分隔条（[`drag_handle`] 开始会话、[`drag_tracker`] 写入）。
    ///
    /// **持久化**：会话文档的 `leftPaneWidth`（写见 [`ShellWorkspace::session_ui_state`]，
    /// 读回见 [`ShellWorkspace::restore_session`]）。落盘时机是**手势结束**那一次、
    /// 且走既有的 300ms 去抖保存路径 —— 拖动中的每一帧都不落盘，理由见
    /// `.agents/notes/implemented/architecture/2026-09-13-resizable-ui-performance-boundaries.md`。
    left_sidebar_width: f32,
    /// **右侧**工具窗宽度（规格像素）。
    ///
    /// 默认 [`RIGHT_TOOL_WINDOW_WIDTH_SPEC`]（**360**，真源 `WorkbenchLayoutStore.swift:5`；
    /// 改前的 400 出自已作废的 `windows/tauri`）；拖它的**左**边界那条分隔条可改（右栏是被右
    /// 活动栏顶住的定宽面板，它的可变边界在前缘）。允许区间见 [`right_tool_window_max_width`] ——
    /// 真源给的是最小 300 / 最大 760（`WorkbenchLayout.minimumMavenPaneWidth` /
    /// `maximumMavenPaneWidth`），并且**按可用宽度收敛**（真源 `WorkbenchRightToolSplitView`
    /// 的 `minimumWorkspaceWidth = 640`）。
    ///
    /// 持久化：会话文档的 `rightPaneWidth`（`Option<f32>`，缺键 = 保留上面这个默认值；
    /// 判据见 [`pane_span_on_restore`]）。
    right_tool_window_width: f32,
    /// **底部**工具窗高度（规格像素）。
    ///
    /// 默认 [`BOTTOM_PANE_HEIGHT_SPEC`]；拖它**上**边界那条分隔条可改（垂直轴，热区在面板顶边）。
    /// 下界 [`BOTTOM_PANE_MIN_HEIGHT_SPEC`]，上界见 [`bottom_pane_max_height`]。
    ///
    /// 持久化：会话文档的 `bottomPaneHeight`。
    bottom_pane_height: f32,
    /// **分栏容器**的**实测尺寸**（逻辑 px，上一帧 prepaint 量的；`None` = 还没量到）。
    ///
    /// ## 为什么需要它（第 4 条遗留：上界用错了容器）
    ///
    /// 真源算上界用的是**分栏容器**的实测尺寸：`WorkbenchView.swift:1999` 的 `GeometryReader`
    /// 给出 `geometry.size.width` / `.height`，`:2000-2005` 与 `:2022-2025` 再减掉内衬 / 间距。
    /// 本侧原来拿 `window.viewport_size()`（**窗口内框**）代替容器 —— 那个盒子把标题栏、
    /// 项目标签条、状态栏和两条活动栏全算进去了，于是上界一律**偏松**（实测差额见
    /// [`Self::pane_span_range`]）。
    ///
    /// ## 为什么是"上一帧"（`on_prepaint` 而不是 render 里现读）
    ///
    /// gpui 的布局发生在 `render` **之后**，`render` 里没有任何 API 能读到某个子容器的
    /// 已布局尺寸。本仓既有的做法是 `on_prepaint`（`project_menu.rs:1312` / `menu_bar.rs:2617`
    /// 量触发器盒子同一套）：prepaint 闭包拿得到当帧 `Bounds<Pixels>`，存进 `Rc<Cell>` 供
    /// **下一帧** render 读。量的是 `render` 里那个 **`gap_0` 内层组**
    /// （`h_flex().flex_1().h_full().min_w_0()`，装 [左侧面板, 中央列]）——它正是真源那个
    /// `GeometryReader` 的对应物：宽 = 窗口内宽减两条活动栏 / 右工具窗 / 各处 4px 间距，
    /// 高 = 窗口内高减标题栏 / 标签条 / 状态栏。
    ///
    /// ⚠️ **只读、不改状态、不 notify**：prepaint 闭包里只写这个 `Rc<Cell>`（与
    /// `project_menu` 同形），仅**第一次**量到时补一帧（`window.request_animation_frame()`，
    /// 否则首帧没有可用的上界）。
    ///
    /// ⚠️ **右工具窗不用它**：真源给右工具窗的容器是 `WorkbenchRightToolSplitView` 的
    /// `geometry`（= 窗口减两条活动栏，**包含**右工具窗自己），而本侧的两条活动栏是同一行
    /// 的兄弟、不是那一层的外框 —— 那个容器在本侧的树里**没有对应元素**，所以右栏仍按
    /// 窗口内宽近似（理由与差额写在 [`right_tool_window_max_width`]）。
    split_container: Rc<Cell<Option<Size<Pixels>>>>,
    /// **正在被拖的面板** + 这次会话的锚点；`None` = 没在拖。
    ///
    /// 三块可拖面板共用**这一个**槽：同一时刻只有一个指针，物理上不可能同时拖两块，于是
    /// "这次移动事件算不算我这次拖拽"只有一处判据 —— 不用给每块面板各留一对
    /// `dragging` / `origin` 字段（那样三份状态迟早会漂移）。
    ///
    /// 会话开关由 [`drag_handle`] 的 `.on_drag` 构造器置上，[`drag_tracker`] 在
    /// mouse up（或发现左键已不在按下态）时清掉**并落盘**。语义与 gpui-kit Dock 皮肤
    /// `SkinShared.resizing_dock` 同源 —— 窗口级移动监听靠它区分"这次拖拽"与
    /// "别处按住左键的普通拖动"（编辑器选文本等）。
    pane_resize_drag: Option<PaneResizeDrag>,
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
    /// 左侧栏的**第三个**内容视图：查找面板（活动栏「搜索」项，下标 [`SEARCH_ACTIVITY_IX`]）。
    ///
    /// 与上面两个同一个左栏槽位三选一渲染。面板的**状态**（查询串 / 选项 / 结果 / 代次）
    /// 全在它自己那边（`crate::search_panel` 的模块文档），外壳只负责"什么时候把它画出来"
    /// 与"把打开文件的回调接上编辑区"——`workspace.rs` 已经 4000+ 行，这是最后一处新增。
    ///
    /// ⚠️ 面板**不**进会话文档：查询与结果不入 `.lithe/session.local.json`
    /// （搜索历史持久化不在本件范围）。
    search: Entity<crate::search_panel::SearchPanel>,
    /// 三个搜索浮层的句柄（**只在它们开着时**能 `upgrade` 成功）。
    ///
    /// 为什么外壳要持有它们：索引就绪状态的真源是 [`ShellWorkspace::search_index`]，而浮层是
    /// **每次打开新建**的 `Entity`（`Ctrl+P` / 菜单 → `cx.new` → `window.open_dialog`），
    /// 外壳要把状态变化**推**进那个**当下存在**的实例，否则"打开浮层时还在建索引、建完了
    /// 提示不消失"。
    ///
    /// 存 `WeakEntity` 而不是 `Entity`：浮层关掉后最后一批强引用（对话框的 `content` /
    /// `on_close` 闭包）一起 drop，`upgrade` 就从 `Some` 变 `None` —— **不需要**任何"关闭时
    /// 清句柄"的生命周期记账，也不会把浮层留住。三个字段与 `search` 同一处置：外壳只为
    /// "把宿主状态登记给子视图"而持有句柄（与 `BufferWriteHost` / `TabMenuHostActions`
    /// 同一个口径），打开 / 渲染 / 关闭仍全归各自的模块。
    quick_open_overlay: Option<gpui_kit::WeakEntity<crate::quick_open::QuickOpen>>,
    search_everywhere_overlay:
        Option<gpui_kit::WeakEntity<crate::search_everywhere::SearchEverywhere>>,
    replace_in_files_overlay: Option<gpui_kit::WeakEntity<crate::replace_in_files::ReplaceInFiles>>,
    /// 中央列内容：编辑区（标签栏 + 正文 / 空状态）。
    editor: Entity<EditorPane>,
    /// 中央扩展页（`crate::extensions_page`，Zed 式独立页，2026-09-29 起）。
    ///
    /// **懒创建**：第一次打开时才建实体（它构造后会自己拉一次目录数据）；创建后
    /// **常驻**——关掉只翻 [`ShellWorkspace::extensions_page_open`]，重开不丢搜索与
    /// 筛选状态（与左栏双实体同一口径）。中央列在「本页 ↔ 编辑器岛」之间二选一渲染，
    /// 与左栏的「项目树 ↔ 更改」同一套模式。
    extensions_page: Option<Entity<crate::extensions_page::ExtensionsPage>>,
    /// 中央扩展页是否占据中央列（取代编辑器岛）。
    ///
    /// **不入会话文档**（2026-09-29 口径）：会话持久化只存"今天真的有宿主"的字段
    /// （`feature/2026-09-27-session-state-persistence.md`），扩展页是新形态，等它有
    /// 更多宿主能力（安装 / 启停）再决定要不要存。旧会话里的 `rightView: "extensions"`
    /// 在恢复时解不出视图（[`RightToolWindowView::from_id`] 已不认它），按"未知 id 保留
    /// 当前值"的既有规则静默落到默认视图。
    extensions_page_open: bool,
    /// 底部窗里的终端。**构造期不建会话**，第一次可见时由
    /// [`TerminalPane::ensure_session`] 懒创建。
    terminal: Entity<TerminalPane>,
    /// 底部窗里的 Git 提交记录。
    bottom_git: Entity<BottomPane>,
    /// 底部窗里的「运行」面板（配置列表 + 输出控制台，见 `lithe_gpui_run::RunPane`）。
    ///
    /// 进程归它自己起（与终端同一口径："进程归 feature crate"），外壳只负责把它放进底部
    /// 工具窗、并在它报告"一次运行开始了"时把工具窗切过来。
    run: Entity<RunPane>,
    /// 订阅运行面板的 [`RunPaneEvent`]：一次运行开始时自动显示底部工具窗并切到「运行」
    /// （点 `▶` 之后看不到输出是最糟的体验）。同样是 RAII，必须持有。
    _run_subscription: Option<gpui_kit::Subscription>,
    /// 底部窗里的「调试」面板（U19：断点模型 + 调用栈 / 变量树 / 监视 / 求值 / 单步控制）。
    ///
    /// 断点表的 **owner 是它**（不是本外壳、也不是编辑区）：本外壳只做三件它做不了的事 ——
    /// 把工作区根给它的键换算成绝对路径、把快照推给编辑区的行号槽、把它的动作接到菜单。
    debug: Entity<DebugPane>,
    /// 订阅调试面板的 [`DebugPaneEvent`]：断点集一变就把快照重推给行号槽。
    /// 与 `_run_subscription` 同理，RAII，必须持有。
    _debug_subscription: Option<gpui_kit::Subscription>,
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
    /// ## 为什么是 `Rc<Cell<bool>>`
    ///
    /// 勾选态必须**跨帧**存在（`window.open_dialog` 的 builder 每帧都会被重跑，
    /// gpui-kit 0.7 里由窗口根 `Root` 的 plugin 层渲染时重跑），所以不能是闭包里的
    /// 局部变量。它也**不能**是"builder 每帧 `shell.read(..)` 去读实体字段"的形状：
    /// builder 跑在渲染期，而外壳随时可能正被更新（0.6 时代 dialog 层由外壳自己画，
    /// `shell.read` 必 panic，实测记录保留在这里防止回退），读一个正在被更新的实体会崩：
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
    /// 通知中心的**数据**（`lithe-gpui-notify::Store`）。
    ///
    /// 这是「消息发生过」的唯一落点：角落 toast（gpui-kit `component::Notification`）与状态栏
    /// 那条 4 秒小字都已废掉。决策与被否方案见
    /// `.agents/notes/implemented/architecture/2026-09-27-gpui-notification-center.md`。
    ///
    /// 为什么住在 `App` 的全局状态而不是本结构体的字段：换根会**重建整个外壳**，而
    /// `.agents/notes/implemented/bug-fix/2026-09-27-exclude-write-must-be-read-back.md:73-77`
    /// 已经把这条技术理由写死了 —— 异步回填时只保证外壳实体还活着，通知/对话框要求
    /// 窗口与 `Root` 就位。store 挂在外壳上就会跟着换根一起没。
    ///
    /// 代价是它比外壳活得久，所以**换根必须显式清空**：见
    /// [`ShellWorkspace::clear_notifications_on_root_swap`]，否则旧项目的通知会挂到新项目上。
    notifications: Entity<lithe_gpui_notify::Store>,
    /// 通知中心的**面板**（搜索 / 筛选 / 展开都在它自己身上，见 `crate::notifications`）。
    ///
    /// 刻意不做成 `right_tool_window` 的自由函数参数：面板要持有自己的视图状态，而
    /// `ShellWorkspace` 已经 287KB 了。
    notification_panel: Entity<crate::notifications::NotificationPanel>,
    /// 订阅通知 store：**每记一条就弹一条左下角 toast**。
    ///
    /// 订阅必须**被持有**：`Subscription` 一 drop 就取消（gpui 的 RAII 语义），
    /// 所以放在结构体里而不是丢在 `new()` 的局部变量里。回调在
    /// [`ShellWorkspace::dispatch_notification_toasts`]，它把「刚记进 store 的条目」逐条
    /// 转成 gpui-kit 的 `component::Notification`（2026-09-29 产品决定：通知的唯一即时
    /// 反馈是这条 toast，状态栏回显与常驻红条都废掉了）。
    _notification_toast_subscription: Option<gpui_kit::Subscription>,
    /// 已经弹过 toast 的最大 `seq`（[`lithe_gpui_notify::Entry::seq`]）。
    ///
    /// store 的观察回调每次都会跑，但它分不清「哪几条是这次新进的」—— 这个水位线补上
    /// 这一半：[`lithe_gpui_notify::Store::entries_since`] 拿到水位线之后的条目，弹完推进。
    /// 回调可能在一次批量写入后只跑一次（gpui 合并 notify），所以必须是"逐条查询"
    /// 而不是"弹最新那条"。换根重建外壳时归零，新 store 的 seq 也从 1 起，两边天然对齐。
    notification_toast_seq: u64,
    /// **会话状态**（`.lithe/session.local.json`）：打开的文件 + 当前文件 + 侧栏可见性 +
    /// 左右面板的当前视图 id。
    ///
    /// 一个工作区一个实例：换根会重建整个外壳，新外壳的 [`SessionTracker::new`] 读的是新根的
    /// 会话，旧实例随旧外壳一起 drop —— 所以换根**前**必须先
    /// [`ShellWorkspace::flush_session`]（见 `crate::session` 的模块文档）。
    ///
    /// 装载期的诊断同样在这里（构造期读的，与渲染无关）。
    session: SessionTracker,
    /// 在飞的那次会话防抖写的**任务**。
    ///
    /// 必须被持有：gpui 的 `Task` 一 drop 就**取消**（同 `editor` 的 `auto_save_task`）。
    /// 更要紧的是"换掉旧任务就是取消旧的防抖唤醒"—— 连点标签时只留最后一次计时器，
    /// 这正是 [`SessionTracker`] 里 `DebounceState` 的代数机制要表达的语义。
    session_save_task: Option<gpui_kit::Task<()>>,
    /// 退出前补写会话的钩子（`on_app_quit`）。
    ///
    /// 必须被持有：`Subscription` 一 drop 就取消（gpui 的 RAII 语义），丢在 `new` 的局部
    /// 变量里等于没装 —— 同一个陷阱在 `_settings_subscription` 那边已经记过一次。
    _quit_subscription: Option<gpui_kit::Subscription>,
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
        run_probe: bool,
        // `--run-probe-debug`：探针按 `🐞` 起（见 `app::main` 的同名开关）。
        run_probe_debug: bool,
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
        // Quick Open 的 `Ctrl+P`（U17 第二件）：同样是**全局 action**，理由与上一条逐字相同
        // （挂在根元素上会有"启动后没点过任何地方时按不出来"的死角）。
        crate::quick_open::install_actions(cx);
        // Search Everywhere 的 `Tab` / `Shift+Tab`（U17 第三件）：与前两条不同，它不是
        // **开浮层**的键 —— 而是一条**只在浮层焦点路径上生效**的键（key context 判定，
        // 见 `crate::search_everywhere::install_actions`），所以不存在"启动后按不出来"的死角。
        crate::search_everywhere::install_actions(cx);
        // Replace in Files 的 `Ctrl+Shift+R`（U17 第四件）：与 Quick Open 同一条 —— 它是
        // **开浮层**的键，所以必须登记成全局 action（挂在根元素上会有"启动后没点过任何地方
        // 时按不出来"的死角）。键位有真源依据：真源的 Windows 平台表就是 `Ctrl+Shift+R`
        // （`macos/Sources/Lithe/Models/Keymap/LitheCommandCatalog.swift:61`）。
        crate::replace_in_files::install_actions(cx);

        // 设置里的「项目 · JDK 与 Maven」页把 JDK 覆盖值（`javaHomePath`）存进设置文件，
        // 而这条值**唯一的消费方是语言服务**（JDT LS 用哪个 JDK 起）：这条链路只有本 crate
        // 拼得起来 —— `lithe-gpui-java` 不许依赖 `settings`（那会把设置文件的键灌进语言服务），
        // 而 `settings` 不认识 JDT LS 会话。照 `set_git_identity_host` 同一口径：
        // 低层 crate 定义注入点（`lithe_gpui_java::toolchain`），外壳在这里登记。
        //
        // ⚠️ **必须早于下面那一步 `prepare_java`**：它会在后台起 JDTLS 并当场读这个值，
        // 登记晚了这一轮读到的还是"自动发现"的那个 JDK —— 也就是页面上写着「已选择」
        // 却没有生效的那副假象。
        register_java_toolchain(&root, cx);

        // 建立这个工作区的 `.lithe/project.json` 身份（首次打开时生成 UUID 并落盘；
        // 之后每次打开只读不写）。**打开即建、无条件**是一条产品决策，而
        // `ShellWorkspace::new` 是所有"打开"路径的公共收口 —— 理由、失败语义、
        // 以及"非 Git 目录也会多出一个 `.lithe/`"这个明确接受的代价，都写在
        // [`prepare_workspace_config`] 的文档里。
        prepare_workspace_config(root.clone(), cx);

        // 把工作区根交给设置状态：`.lithe/settings.json`（共享层）与
        // `.lithe/settings.local.json`（本机层）里的**外观覆盖**从这一刻起生效。
        //
        // 位置有要求：必须**早于**下面那条 `try_store(..)` 的设置订阅块 —— 那一条读设置算
        // 「工作台启动证据」（缩进 / 终端 shell / 终端字号…），根登记晚了它读到的还是全局值
        // （随后会被订阅回调纠正，但那一行证据就变成误导性的）。读两份很小的 JSON、
        // 不碰 Core；**覆盖顺序的唯一实现**在 `lithe_gpui_settings::workspace::resolve_effective`。
        if let Some(store) = lithe_gpui_settings::try_store(cx) {
            let settings_root = root.clone();
            store.update(cx, |store, cx| {
                store.set_workspace_root(Some(settings_root), cx)
            });
        }

        // 通知 store 每次建外壳都新建一份：换根 = 换项目，所以下一个项目的通知中心天然是
        // 空的（新 store、水位线归零），不需要显式清空。理由与代价见
        // `lithe-gpui-notify` 的模块文档「store 归谁持有」。
        //
        // 位置在编辑区**之前**：编辑区要拿它的弱引用（保存失败 / 跳转无目标要往里记）。
        let notifications = cx.new(|_| lithe_gpui_notify::Store::new());

        // toast 派发：store 一变就把新条目弹成左下角 toast（见
        // [`ShellWorkspace::dispatch_notification_toasts`]）。观察要 `&mut Window`（弹 toast
        // 走 `window.push_notification`），所以用 `observe_in`；订阅是 RAII，字段持有。
        let notification_toast_subscription = Some(cx.observe_in(
            &notifications,
            window,
            |this, store, window, cx| this.dispatch_notification_toasts(&store, window, cx),
        ));

        // 先建编辑区，再把它的弱引用交给项目树：点文件 → 打开到编辑区。
        let editor = cx.new(|cx| {
            // 通知中心的弱引用注入（见 `EditorPane::new` 的参数文档）。弱引用而不是强引用：
            // 所有权在 `ShellWorkspace`，编辑区不该 prolong store 的寿命。
            EditorPane::new(Some(notifications.downgrade()), window, cx)
        });
        // 阶段 10 第二批：打开项目时在后台起 Java 语言服务（= 生成 / 复用 JDT 索引缓存）。
        // 放在这里而不是编辑区自己：工作区根是外壳的参数，编辑区不认识它。
        // 整段是后台任务，失败只留 `S1_JAVA_*` 诊断，跳转退回第一批的轻量链路。
        editor.update(cx, |pane, cx| pane.prepare_java(root.clone(), cx));
        let editor_handle = editor.downgrade();
        // 「打开文件 → 中央区让位回编辑器」要够得着外壳：扩展页若开着就收掉。
        let shell_for_open = cx.entity().downgrade();

        let on_open: Box<dyn Fn(PathBuf, &mut Window, &mut App) + 'static> =
            Box::new(move |path: PathBuf, window: &mut Window, cx: &mut App| {
                // 中央区让位回编辑器：资源管理器点文件时扩展页若开着就收掉
                // （与命令面板「打开文件」同一条；诊断行同族可 grep）。
                let _ = shell_for_open.update(cx, |this, cx| {
                    this.close_extensions_page(None, cx);
                });
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
        // U24：右键菜单「在资源管理器中显示」的宿主钩子。起系统文件管理器属平台层，
        // 落点在外壳（与下面编辑区那条 `TabMenuHostActions::reveal` 同一个函数、同一条约束：
        // `lithe-gpui/crates/explorer/src/lib.rs` 未实现清单里登记过"reveal 不该由 UI 模块做"）。
        explorer.update(cx, |this, _cx| {
            this.set_reveal_in_file_manager_host(Arc::new(|path: &Path, _cx: &mut App| {
                if let Err(error) = reveal_in_file_manager(path) {
                    // 失败不静默：与 `S1_TAB_MENU run=reveal result=failed` 同口径走 stderr。
                    eprintln!(
                        "S1_EXPLORER reveal_in_file_manager result=failed path={} error={error}",
                        path.display()
                    );
                }
            }));
        });

        // 左栏「更改」（源代码管理）。**构造期不取数据**：顶部组默认选「项目」
        // （`DEFAULT_TOP_ACTIVITY`），所以这个视图在第一帧不可见；它的第一次读取发生在
        // 用户点活动栏「更改」时（`on_select_activity` 调 `ChangesView::activate`），
        // 之后由「手动刷新 + 写后刷新 + 再次激活」维持（本侧刻意不做 watcher，见 crate 文档）。
        let changes_root = root.clone();
        let changes = cx.new({
            let window = &mut *window;
            // 「更改」视图里冲突文件列表的打开通路：与 `explorer` / 搜索面板**同一个落地函数**
            // （`pane.open_at`），只是本路没有行号可跳（Core 的 `git.conflictMarkers` 只回路径）。
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

        // 左栏「搜索」（查找面板，U17 第一件）。**构造期不发 Core 调用**：面板默认是
        // `idle` 空态（没有查询串就不搜），第一次 `workspace.search` 发生在用户输入之后。
        //
        // 打开通路的接线与资源管理器同源（`on_open` 那个回调）：外壳是唯一够得到编辑区的那层，
        // 面板自己不 import `lithe-gpui-editor`。两条分支对应真机的两种结果：
        // `Some(line)` → 跳到那一行（`open_at`，与问题列表同一个落地函数，只是 `kind=search`）；
        // `None` → 只打开文件（按文件名命中的结果没有行可跳，`SearchSidebarView.swift:99-103`）。
        let search = {
            let editor_handle = editor.downgrade();
            let shell_for_open = cx.entity().downgrade();
            let on_open: Box<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static> =
                Box::new(
                    move |path: PathBuf, line: Option<u32>, window: &mut Window, cx: &mut App| {
                        // 中央区让位回编辑器：与资源管理器点文件同一条（扩展页若开着就收掉）。
                        let _ = shell_for_open.update(cx, |this, cx| {
                            this.close_extensions_page(None, cx);
                        });
                        // 编辑区可能已经被关掉：`update` 返回 `Err` 时静默忽略，不 panic。
                        let _ = editor_handle.update(cx, |pane, cx| match line {
                            Some(line) => {
                                // 列口径：Core 的搜索结果**只有行号没有列**
                                // （`contracts.rs:101-102` 的 `SearchMatch`），所以列给 0
                                // —— 与"落到行首"一致，也与问题列表的调用形状相同。
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

        // 标签右键菜单（`lithe-gpui/research/windows/09-tab-context-menu.md`）：把工作区根与
        // 两件"只有外壳做得了"的动作登记给编辑区。
        //
        // ⚠️ **必须排在 `terminal` 之后**：两个回调都要捕获终端实体的弱引用（「在终端中打开」
        // 要新开一个带工作目录的页签，并把底部工具窗切到终端且显示）。
        // 被绕开的只有"操作系统把这次点击送进窗口"那一段 —— 回调里走的都是与主菜单
        // 「终端 → 新建终端」同一个 `TerminalPane::new_tab_in` / 同一个 `bottom_visible`。
        //
        // ⚠️ **U19 起还要排在自己的 `debug` 实体之后**：编辑区的两个调试回调
        // （点行号槽 / 正文变了）要捕获外壳的弱引用，而外壳的 `toggle_breakpoint` 要
        // `debug.update(..)`。
        let debug = cx.new(|_cx| DebugPane::new());
        // 步进过滤器那一块的数据来自 Core 的 `debug.steppingFilters`
        // （**唯一无会话依赖的 debug 命令之一**，见 `debug.rs` 的模块文档）：
        // 建面板时就取一次，Core 调用在 `background_spawn` 上，不在 UI 线程等。
        debug.update(cx, |pane, cx| pane.load_stepping_filters(cx));
        let debug_subscription = cx.subscribe(&debug, |this, _pane, event: &DebugPaneEvent, cx| {
            match event {
                DebugPaneEvent::BreakpointsChanged => {
                    // 断点表的 owner 是调试面板；行号槽只画。所以"谁改了断点"由这一个事件说出来，
                    // 快照的重推也只有这一处。
                    this.push_gutter_breakpoints(cx);
                }
                DebugPaneEvent::Hide => {
                    // 面板头部那颗「隐藏」（真源 `GenericDebugView.swift:151-155`）：
                    // 面板不知道底部工具窗是否可见，所以由外壳改这两个字段。
                    this.bottom_visible = false;
                    this.schedule_session_save(cx);
                    cx.notify();
                }
            }
        });
        {
            let shell = cx.entity().downgrade();
            // 「文件写盘成功」的宿主钩子要自己一份弱引用：下面 `open_terminal` 那个闭包会把
            // `shell` 整个搬走（`Arc<dyn Fn>` 是 `'static` 的），不能再共用一份。
            let shell_for_write = shell.clone();
            // U19 的四个调试回调各要自己一份弱引用（下面 `open_terminal` 那个闭包会把 `shell`
            // 整个搬走），所以在这一层先克隆一份给它们。
            let shell_for_debug_gutter = shell.clone();
            let terminal_handle = terminal.downgrade();
            let workspace_root = root.clone();
            editor.update(cx, |pane, _cx| {
                // 「复制相对路径」要工作区根（无根时拷全路径，与真机同口径）。
                pane.set_workspace_root(workspace_root);
                pane.set_tab_menu_host_actions(TabMenuHostActions {
                    // 「在资源管理器中显示」：起系统文件管理器的定位进程 ——
                    // 这与 `lithe-gpui/crates/explorer/src/lib.rs:93-94` 登记的"reveal 属平台层、
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
                    // U24：编辑区标签右键菜单的「在项目树中显示」（真源
                    // `macos/Sources/Lithe/Views/Editor/EditorAreaView.swift:1206-1213`）。
                    // 项目树归 `explorer` crate，编辑区不认识它，所以落点在这里：切到项目视图 +
                    // 在树里定位。外壳自己一份弱引用（下面 `open_terminal` 会把 `shell` 整个搬走）。
                    reveal_in_tree: {
                        let shell_for_reveal = shell.clone();
                        Arc::new(move |path: &Path, cx: &mut App| {
                            let _ = shell_for_reveal.update(cx, |shell, cx| {
                                shell.reveal_in_project_tree(path, cx);
                            });
                        })
                    },
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
                // 「文件写盘成功」→ 搜索索引增量更新（`workspace.searchIndex.update`）。
                //
                // 为什么必须经外壳：索引是**工作区级**资源，三条 `searchIndex.*` 命令的时机 /
                // 请求形状 / 代次 / 诊断口径只有 `crate::search_index` 一份
                // （编辑区自己调会变成第二份真相源）。编辑区只报"哪个文件落盘了"。
                // 驱动只有"保存"这一条：宿主没有文件系统 watcher（`H12`），本任务不新建。
                let write_host: BufferWriteHost = Arc::new(move |path: &Path, cx: &mut App| {
                    // 外壳可能已经被换根丢掉：`update` 返回 `Err` 时静默忽略，不 panic。
                    let _ =
                        shell_for_write.update(cx, |shell, cx| shell.note_buffer_written(path, cx));
                });
                pane.set_buffer_write_host(write_host);

                // U19：行号槽的两个调试回调（点断点 + 正文变了）。三个回调都要捕获外壳
                // （断点表的 owner 是 `debug` 实体，而键的换算要工作区根 —— 两样都在外壳手里）。
                let shell_for_toggle = shell_for_debug_gutter.clone();
                let shell_for_enable = shell_for_debug_gutter.clone();
                let shell_for_remove = shell_for_debug_gutter.clone();
                let shell_for_edit = shell_for_debug_gutter;
                pane.set_debug_gutter_host(DebugGutterHost {
                    // 点行号槽：切换该行断点（真源 `toggleBreakpoint`，`AppModel+Debugging.swift:292-313`）。
                    toggle: Arc::new(move |path: &Path, line: usize, cx: &mut App| {
                        let _ = shell_for_toggle.update(cx, |shell, cx| {
                            shell.toggle_breakpoint(path, line, cx);
                        });
                    }),
                    // 右键「启用 / 禁用断点」（真源 `CodeEditorView.swift:5380-5382`）。
                    set_enabled: Arc::new(
                        move |path: &Path, line: usize, enabled: bool, cx: &mut App| {
                            let _ = shell_for_enable.update(cx, |shell, cx| {
                                shell.set_breakpoint_enabled(path, line, enabled, cx);
                            });
                        },
                    ),
                    // 右键「移除断点」（真源 `:5383-5385`）。
                    remove: Arc::new(move |path: &Path, line: usize, cx: &mut App| {
                        let _ = shell_for_remove.update(cx, |shell, cx| {
                            shell.remove_breakpoint(path, line, cx);
                        });
                    }),
                    // 正文变了：`debug.relocateBreakpoints` 的输入（形参见 `DebugGutterHost`）。
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

        // 「运行」面板（底部工具窗的「运行」页）：配置列表 + 真进程 + 输出控制台。
        //
        // 「工具链在哪」按仓库既有的"低层 crate 定义钩子、外壳填实现"口径注入
        // （与 `TabMenuHostActions` / `GitIdentityHost` 同一形状）：`run` 只依赖 `shared`，
        // 不认识设置，也不该自己去找 JDK。
        //
        // ⚠️ 解析器**懒探测**：`discover` 会跑 `java -version` / `mvn -version`，
        // 所以只在真的点 ▶ / 🐞 时才算（`register_java_toolchain` 那条路是给语言服务用的，
        // 必须早算；这里不能跟着它在启动路径上付探测的钱）。
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
                // 与设置页「项目 · JDK 与 Maven」用的是**同一个** `discover`
                // （`settings/src/project.rs`），所以运行用的 JDK / Maven
                // 与设置页显示的那个值必然一致。
                let environment = lithe_gpui_settings::project::discover(&resolved);
                let effective = match reference {
                    "java" | "project-jdk" => environment.jdk.effective(),
                    "maven" | "project-maven" => environment.maven.effective(),
                    _ => return None,
                };
                match effective {
                    lithe_gpui_settings::project::EffectiveToolchain::Resolved { path, .. } => {
                        // JDK 给的是**主目录**，Maven 给的是**可执行文件**
                        // （`EffectiveToolchain::Resolved::path` 的文档），两种形态分开传。
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
                // `--run-probe`（验证/诊断用）：识别完配置就自动跑第一条，走的是与点 ▶
                // 完全相同的那段代码（见 `RunPane::set_probe` 的文档）。
                pane.set_probe(run_probe, run_probe_debug);
                pane.set_workspace(root.clone(), resolve, cx);
            });
        }
        let run_subscription = cx.subscribe(&run, |this, _pane, event: &RunPaneEvent, cx| {
            if *event == RunPaneEvent::Started {
                // 一次运行开始了：把底部工具窗切到「运行」并显示（同 `MenuAction::ShowRunAndDebug`
                // 那两个字段，真机也是"运行 → 输出面板自动出现"）。
                this.bottom_kind = BottomPaneKind::Run;
                this.bottom_visible = true;
                this.schedule_session_save(cx);
                cx.notify();
            }
        });

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
                // U20 的两个开关：`autoSave` 与 `tabSize` 同一形状，`wordWrap` 多要一个窗口
                // （`EditorState::set_soft_wrap` 要拿当帧的换行宽度）—— 这里在 `new()` 里，
                // `window` 在手上。
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
            // ⚠️ 用 `observe_in` 而**不是** `observe`：U20 的「软换行」是唯一一个
            // **必须拿到窗口**才推得动的值型设置 —— `EditorState::set_soft_wrap` 要当帧的
            // 换行宽度（`gpui-base-0.7.0/src/input/base/state.rs:9971-9989`），而
            // `cx.observe` 的回调拿不到 `Window`（只有 `(this, observed, cx)`）。
            // 本视图与它观察的设置实体在同一个窗口里（`new()` 的 `window` 就是它），
            // 所以 `observe_in` 的窗口绑定在这里是成立的、不是权宜之计。
            cx.observe_in(&store, window, |this, store, window, cx| {
                let settings = store.read(cx).settings().clone();
                this.editor.update(cx, |pane, cx| {
                    pane.set_tab_size(settings.tab_size as usize, cx);
                    // 「自动补全」（阶段 18，「LSP」页）：与 `tabSize` 同一条"值型设置经外壳
                    // 转发"的路子。落点是编辑区那份共享的原子开关，所以**已打开的** Java
                    // buffer 与之后新开的都立刻跟上（见 `EditorPane::set_auto_completion`）。
                    pane.set_auto_completion(settings.auto_completion, cx);
                    // U20「自动保存」：与上面两条同一条路子，落点是编辑区自己的闸门
                    // （`EditorPane::set_auto_save`，关掉时顺手取消待执行的自动保存任务）。
                    pane.set_auto_save(settings.auto_save, cx);
                    // U20「软换行」：逐 buffer 推（它是每个 `EditorState` 自己的渲染属性）。
                    pane.set_soft_wrap(settings.word_wrap, window, cx);
                });
                this.terminal.update(cx, |pane, cx| {
                    pane.set_default_shell(&settings.terminal_default_shell_id, cx);
                    // 「终端字号」（本批）：与 `terminalDefaultShellId` 同一条"值型设置经外壳
                    // 转发"的路子。差别是它作用在**所有已打开的页签**上（字号是渲染属性）。
                    pane.set_font_size(terminal_font_size_override(settings.terminal_font_size), cx);
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
                //
                // ⚠️ 这里也读**项目本机层**（`.lithe/*.local.json`）：用户在设置页把覆盖值
                // 存进项目本机层之后，全局设置其实没变，但这一行必须重新解析两层才能拿到新值。
                let current_root = this.root.clone();
                register_java_toolchain(&current_root, cx);
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
        //
        // 同一条观察也是**会话内容的触发点**：开 / 关标签、切标签都会让 `EditorPane`
        // `notify`，而"哪几个文件开着 + 当前是哪个"正是会话文档里的核心内容。
        // 每个打开动作都去调一次 `schedule_session_save` 会长出六七个调用点，而这里一个就够
        // —— `SessionTracker::plan` 自己判"内容没变"，所以重绘不会变成写盘。
        let editor_subscription = Some(cx.observe(&editor, |shell: &mut Self, _, cx| {
            shell.schedule_session_save(cx);
            cx.notify();
        }));

        let project_name: SharedString = root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| root.display().to_string())
            .into();

        // 分支的启动期证据**不再由这里单独读一次**（原先是直接读 `.git/HEAD`，D11 已删）：
        // 分支现在来自 Core `git.status` 的快照，而 `changes_view` 的
        // `S1_SOURCE_CONTROL … branch=…` 诊断行已经把它打出来了
        // （见 `changes_view.rs` 的 `println!`），所以无人值守验证仍旧 grep 得到，
        // 而且那一行来自 Core 的判定 —— worktree / 分离 HEAD / 仓库根≠工作区根都不会读错。

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

        // 通知 store 已在编辑区之前建好（那里要它的弱引用）。面板与字段必须观察**同一个**
        // 实体，否则面板会盯住一个没人往里写的 store（表现为「铃铛一直不亮」）。
        let notification_panel =
            cx.new(|cx| crate::notifications::NotificationPanel::new(&notifications, window, cx));

        let mut workspace = Self {
            projects: vec![ProjectTab::new(project_name.clone())],
            // 滚动状态从 0 偏移开始；真源在 `onAppear` 时把活动项滚到中间
            // （`WorkbenchView.swift:640-642`），本侧等价动作在切项目的回调里做。
            project_tabs_scroll: ScrollHandle::new(),
            root: root.clone(),
            // 索引接线绑定这个根；`warm` 在下面 `S1_WORKSPACE` 之后立刻排（见那一行）。
            search_index: SearchIndexWiring::new(root.clone()),
            // 单项目时 Windows 会隐藏整条标签条（`project-tab-bar-model.ts:12-13`）；
            // 阶段 1 只有一个项目，仍然把 `Some(0)` 选中，方便验收外观。
            active_project: Some(0),
            activity_items: activity_items(),
            // 构造期这一份的角点必然是「无未读」（store 刚建、还是空的）；`render` 每帧
            // 会用 `right_activity_items(&self.notifications, cx)` 重算一份带角点的。
            right_activity_items: right_activity_items(&notifications, cx),
            // 默认选中顶部第 0 项「项目」（`workspace-ui-defaults.ts:7` 的 `activeSidebarView: "files"`）。
            top_activity_view: Some(DEFAULT_TOP_ACTIVITY),
            left_sidebar_visible: true,
            // IDEA 语义（2026-09-27）：活动栏常驻，面板默认展开、点当前项可单独收起。
            left_pane_visible: true,
            left_sidebar_width: SIDEBAR_WIDTH_SPEC,
            // 右栏与底部窗的初值 = 各自的规格默认值；恢复会话若存了尺寸，`restore_session`
            // 会按当时的窗口尺寸钳过之后覆盖掉（见那里的 `S1_SESSION_SIZES` 诊断）。
            right_tool_window_width: RIGHT_TOOL_WINDOW_WIDTH_SPEC,
            bottom_pane_height: BOTTOM_PANE_HEIGHT_SPEC,
            // 分栏容器尺寸：构造期还没有任何一帧，`None` = 上界先按窗口内尺寸近似
            // （与"窗口尺寸不可知时给静网上界"同一条口径，见 `pane_span_range`）。
            split_container: Rc::new(Cell::new(None)),
            pane_resize_drag: None,
            // 右工具窗默认**隐藏**（`panel-slice.ts:28` 的 `isRightSidebarVisible: false`），
            // 视图字段的默认值理由见字段文档。
            right_view: RightToolWindowView::Maven,
            right_visible: false,
            maven_project: None,
            spring_index: None,
            right_scan: RightScanState::default(),
            explorer,
            changes,
            search,
            // 三个搜索浮层都还没开（句柄在各自 `open_*` 里登记，见字段文档）。
            quick_open_overlay: None,
            search_everywhere_overlay: None,
            replace_in_files_overlay: None,
            editor,
            // 中央扩展页：懒创建（首次打开才建实体），启动时关闭。
            extensions_page: None,
            extensions_page_open: false,
            terminal,
            bottom_git,
            run,
            _run_subscription: Some(run_subscription),
            debug,
            _debug_subscription: Some(debug_subscription),
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
            notifications,
            notification_panel,
            // toast 水位线从 0 起：新 store 的第一条 seq 是 1，必然 > 0，必然被弹。
            notification_toast_seq: 0,
            _notification_toast_subscription: notification_toast_subscription,
            // 会话装载：**只读文件，不创建**（`.lithe/session.local.json` 在第一次真的产生
            // 内容之前不存在）。恢复（打开文件 + 还原侧栏 / 视图）排在下面 `Self` 建好之后 ——
            // 它要 `window` 才能建编辑器 buffer，也要本结构体自己的字段才能恢复界面状态。
            session: SessionTracker::new(&root),
            session_save_task: None,
            // 下面 `new` 的收尾里装上（见 `on_app_quit` 那一段）。
            _quit_subscription: None,
        };
        // 一行锚点诊断：整个外壳（项目树 / 编辑区 / JDTLS / 右栏 / Git 面板）都挂在**这一个**
        // 根上。换项目会重建整个外壳，所以换根前后各有且只有一行 `S1_WORKSPACE root=…`
        // —— 它是"换根真的发生了"的第一条证据，后面那些 `S1_EXPLORER` / `S1_JAVA_START`
        // / `S1_BRANCH_PANEL` 都应当指向同一个根。
        println!("S1_WORKSPACE root={}", root.display());
        // 搜索索引：项目一打开就**后台**建好（`workspace.searchIndex.warm`）。
        //
        // 不排这一步的后果不是"少一个功能"，而是用户**第一次搜索会在调用线程上同步建全量
        // 索引**。Core 调用是同步阻塞的，所以整段在 `background_spawn` 上（见
        // `crate::search_index`）；这里只排任务、不等结果 —— 建索引失败也不影响开项目
        // （降级为"第一次搜索时再建"，诊断行照常留痕）。
        //
        // 位置在这里而不是别处：`ShellWorkspace::new` 是**所有"打开"路径的公共收口**
        // （argv 启动 + `replace_root`，与下面 `prepare_workspace_config` 同一处论证），
        // 所以"每个根恰好 warm 一次"是结构性的，不需要额外的"已排过"状态。
        workspace.search_index.schedule_warm(cx);
        // 启动期也留一行状态证据：右工具窗**默认隐藏**这件事要能被机器验证，
        // 而不是只靠截图比对（`S1_RIGHT_PANEL`，可 grep）。构造期不是指针输入，所以不带坐标。
        diagnose_right_panel(workspace.right_view, workspace.right_visible, None);
        // `--right-view` / `--left-view`：**每个外壳都应用一次**（含换根重建出来的那个）。
        //
        // 为什么放在构造期而不是 `main.rs` 的"首帧之后"：换根会重建整个外壳，而
        // `main.rs` 那段 `on_next_frame` 是一次性的、捕获的是**第一个**外壳 —— 不做这一步，
        // "换根后右栏 Maven / Spring 面板与 Git 变更视图都指向新根"就取不到证据
        // （视图不打开就不会去扫 / 去读）。构造期不是 render，改状态是安全的。
        let (startup_right_view, startup_extensions, startup_left_view) = startup_views(cx);
        if let Some(view) = startup_right_view {
            workspace.right_view = view;
            workspace.right_visible = true;
            workspace.schedule_right_view_scan(view, true, cx);
            diagnose_right_panel(view, true, None);
        }
        if startup_extensions {
            // `--right-view extensions`：打开的是**中央扩展页**（不是右工具窗视图）。
            workspace.open_extensions_page(None, window, cx);
        }
        if let Some(index) = startup_left_view {
            workspace.select_top_activity(index, cx);
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

        // 会话恢复：把上次打开的文件、当前文件与侧栏 / 视图状态装回来。**排在最后**：
        // ① 它要 `window`（建 buffer 要窗口）；② 它要覆盖本结构体自己的字段（侧栏与视图），
        // 排在 `--right-view` / `--left-view` 那些启动参数**之后**，于是"显式启动参数"
        // 仍然赢过会话（自动化里 `--left-view` 与恢复的侧栏态同时存在时，参数是更明确的意图）。
        workspace.restore_session(window, cx);

        // 会话的落盘时机有两处硬保证 + 一段防抖（见 `crate::session` 的模块文档）：
        // 换项目前 `flush_session`（在 `rebuild_project_window` 里），退出前装这个钩子。
        // `on_app_quit` 与设置文档那条补写同一条口径（`SettingsStore::install_quit_flush`）。
        let quit_shell = cx.entity().downgrade();
        let _quit_subscription = cx.on_app_quit(move |_shell, cx| {
            let _ = quit_shell.update(cx, |shell, cx| shell.flush_session(true, cx));
            // 回调要求返回一个 future；补写本身是同步的，所以这里立即完成。
            async {}
        });
        // ⚠️ `Subscription` 一 drop 就取消（gpui 的 RAII 语义），所以挂进字段。
        // 同一个陷阱在 `_settings_subscription` 那边已经记过一次。
        workspace._quit_subscription = Some(_quit_subscription);

        workspace
    }

    /// 把会话状态装回界面（构造期调一次）：打开文件、切活动标签、恢复侧栏与视图。
    ///
    /// 文件被删 / 改名 / 在根外时**跳过它**（诊断逐条打 `S1_WORKSPACE_CONFIG session_skipped`），
    /// 不报错、不崩溃 —— 判定与过滤在 `crate::session::restore_into` 里。
    fn restore_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let outcome = crate::session::restore_into(&self.root, &self.editor, window, cx);
        let session = outcome.session;

        // 侧栏：会话里没记（第一次打开这个项目）时保留默认值 `true`。
        if let Some(visible) = session.left_sidebar_visible {
            self.left_sidebar_visible = visible;
        }
        // 三块面板的尺寸：**缺字段 = 保留外壳默认值**（旧 `session.local.json` 没有这三个键；
        // 文档层刻意不做上下界判定，钳制归这里）。存了值就按**当时**的窗口尺寸钳一遍 ——
        // 在一个 1920 宽的窗口里拖到 760 的右栏，换到 1000 宽的窗口必须收敛，不能把中央列压没。
        //
        // ⚠️ **默认值只在"没有持久化值"时生效**：判据是 `Option::is_some`（提取成
        // [`pane_span_on_restore`]，有单测钉住 —— 右栏默认 360 不得覆盖用户存过的宽度）。
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
        // 尺寸恢复成一行可 grep 的诊断（拖动交互本身没法确定性单测，最终值必须能被机器看到）。
        println!(
            "S1_SESSION_SIZES left={} right={} bottom={}",
            self.left_sidebar_width, self.right_tool_window_width, self.bottom_pane_height
        );
        // 左右面板的当前视图：稳定 id 认不出时保留当前值（不静默落到某一项）。
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
        // 恢复出来的右栏视图若已可见，照 `--right-view` 同一条路径排一次懒扫
        // （不排的话面板会画空态，看起来像"恢复没生效"）。
        if self.right_visible {
            self.schedule_right_view_scan(self.right_view, true, cx);
        }
        cx.notify();
    }

    /// `--session-probe`：把"打开若干文件 → 把其中一个设为当前 → 收起侧栏"这段迁移真的
    /// 走一遍，然后**立刻写一次会话**（**验证/诊断用**）。
    ///
    /// ## 为什么必须自己 flush
    ///
    /// 真机上"关窗退出"会走 [`ShellWorkspace::flush_session`]（挂在 `on_app_quit` 上），
    /// 而无人值守环境只能强杀进程 —— 强杀不走那个钩子，防抖窗口里那次改动就丢了。
    /// 所以探针在打开动作之后显式 `flush_session(true, ..)`：
    /// 这样"磁盘上那份会话 == 这一段操作的结果"是**当场成立**的，外部脚本可以直接读文件断言。
    ///
    /// 走的是与"资源管理器里点文件"完全相同的那条路（[`EditorPane::open`]），
    /// 被绕开的只有鼠标点击那一段 —— 与 `--menu-probe` / `--right-view` 同一条口径。
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
        // 当前文件：与用户点标签同一个迁移（`EditorPane::open` 每次都会把新开的设成活动），
        // 所以"把第 N 个设为当前"只要**再 open 一次**那一个（同路径只切活动标签、不重读盘）。
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
        // 探针必须显式落盘（理由见方法文档）。
        self.flush_session(true, cx);
        cx.notify();
    }

    /// `--session-assert`：把**恢复之后**的会话状态打成一行可 grep 的证据（**验证/诊断用**）。
    ///
    /// 恢复本身在 `ShellWorkspace::new`（构造期）结束时就完成了，所以这一行是对
    /// "三个文件回来了、当前文件是第二个、侧栏仍是隐藏"这件事的**直接断言**，
    /// 而不是"再恢复一次"。读的也是**界面上的真实字段**（不是磁盘文件）——
    /// 这才证明恢复真的落到了界面上。
    ///
    /// ## 为什么还要写一个文件
    ///
    /// 这两行同时写一份到 `S1_SESSION_ASSERT_FILE` 指到的路径（环境变量，没设就不写）。
    /// 理由是本轮实测：无人值守脚本用 `Start-Process -RedirectStandardOutput/Error` 抓
    /// 子进程输出时，**进程被强杀会把块缓冲里的内容一起丢掉** —— 而"窗口开着、脚本读结论、
    /// 再杀掉"正是这个探针的用法。写文件是唯一不受缓冲影响的取证方式。
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
            // 写失败只留一行诊断：这是验证辅助路径，不该让界面因此出问题。
            if let Err(error) = std::fs::write(&target, report) {
                eprintln!(
                    "S1_SESSION_ASSERT write_failed path={} error={error}",
                    target.display()
                );
            }
        }
    }

    /// 会话状态变了：排一次 300ms 防抖写（同窗口里的多次改动合并成一次）。
    /// 由"开 / 关标签、切标签、切视图、收起侧栏"这些**真的改了会话内容**的路径调用，
    /// 不是每帧调用：`SessionTracker::plan` 自己会判"内容没变"，没变时连计时器都不排。
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
                // 换掉旧任务 = 取消旧的防抖唤醒（gpui 的 `Task` 一 drop 就取消），
                // 于是"连点只写一次"这条语义由任务槽与代数**共同**保证。
                self.session_save_task = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(
                            crate::session::SESSION_SAVE_DEBOUNCE_MS,
                        ))
                        .await;
                    // 实体可能已经销毁：`update` 返回 `Err` 时静默忽略，不 panic。
                    let _ = this.update(cx, |shell, _cx| {
                        shell.session.flush_if_current(armed);
                    });
                }));
            }
        }
    }

    /// 立刻落盘（切换项目前 / 退出前）。`force = true` 时内容没变也写。
    ///
    /// ⚠️ **先 `plan` 再 `flush`**：`plan` 才是"把编辑区此刻的状态收进文档"的那一步，
    /// 少了它写出去的会是上一次 [`Self::schedule_session_save`] 时的快照。
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

    /// 编辑区报告"这个文件刚写回磁盘"（宿主钩子，见 [`BufferWriteHost`]）。
    ///
    /// 这是 `workspace.searchIndex.update` 的**唯一**驱动：宿主没有文件系统 watcher（`H12`），
    /// 所以索引的增量更新只能由"已有的写盘路径"带动，而编辑器的 `write_buffer` 是全仓唯一的
    /// "文件真的落盘了"收口（`Ctrl+S` / 去抖后的自动保存 / 关闭前的确认保存都汇到它）。
    /// 传的是**这一条**路径，绝不退化成"每次保存就 invalidate 全量"。
    pub(crate) fn note_buffer_written(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.search_index.schedule_update(path, cx);
    }

    // -----------------------------------------------------------------------
    // U19 调试：断点表的键换算、快照推送、以及三个"只有外壳做得了"的落点
    // -----------------------------------------------------------------------
    //
    // 分工（与 `search_index` 那条链同一形状）：
    //
    // | 谁 | 做什么 | 不做什么 |
    // | --- | --- | --- |
    // | `crate::debug::DebugPane` | 持有断点表、算请求、调 Core、决定"能不能启动" | 不认识绝对路径、不认识工作区根 |
    // | 本外壳 | 把编辑区给的绝对路径换算成断点表的键、把快照推回行号槽、接菜单 | 不自己存断点、不构造请求 |
    // | `editor` | 画标记、把"点了哪一行/正文变了"报出来 | 不存断点、不算位移 |

    /// 断点表的键：工作区根下的文件取工作区相对路径（`/` 分隔）。
    ///
    /// 换算本体是纯函数 [`crate::debug::debug_key`]（有单测），这里只补一个"还没有工作区根"
    /// 的退化：那时用绝对路径 —— 与 Core 的 `sourcePath` 两种都收（`validate_path`）一致，
    /// 而且比"丢掉这次点击"诚实。
    fn debug_breakpoint_key(&self, path: &Path) -> String {
        crate::debug::debug_key(&self.root, path)
    }

    /// 把断点表的当前快照推给编辑区的行号槽（键还原成绝对路径）。
    ///
    /// 只在"断点集变了"时调（订阅 [`DebugPaneEvent::BreakpointsChanged`] 那一条路，
    /// 以及换根之后补推一次）。快照是每文件一份、量级几十条，整份推的代价可忽略。
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

    /// 点行号槽：切换该行断点（真源 `AppModel+Debugging.swift:292-313`）。
    ///
    /// `path` 是编辑区给的**绝对路径**；本外壳负责换算成断点表的键。
    pub(crate) fn toggle_breakpoint(&mut self, path: &Path, line: usize, cx: &mut Context<Self>) {
        let key = self.debug_breakpoint_key(path);
        let Ok(line) = i64::try_from(line) else {
            return;
        };
        self.debug
            .update(cx, |pane, cx| pane.toggle_breakpoint_at(&key, line, cx));
    }

    /// 右键「启用 / 禁用断点」（真源 `CodeEditorView.swift:5380-5382`）。
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

    /// 右键「移除断点」（真源 `CodeEditorView.swift:5383-5385`）。
    pub(crate) fn remove_breakpoint(&mut self, path: &Path, line: usize, cx: &mut Context<Self>) {
        let key = self.debug_breakpoint_key(path);
        let Ok(line) = i64::try_from(line) else {
            return;
        };
        self.debug
            .update(cx, |pane, cx| pane.remove_breakpoint(&key, line, cx));
    }

    /// 「运行 → 切换当前行断点」（真源 `debugger.toggleCurrentLineBreakpoint`）：
    /// 光标所在行翻转断点。
    ///
    /// 落点是编辑区的 [`lithe_gpui_editor::EditorPane::active_disk_file`] +
    /// [`lithe_gpui_editor::EditorPane::cursor_position`]（两样都是编辑区的只读公开面），
    /// 走的是与点行号槽**完全相同**的 [`Self::toggle_breakpoint`]。
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

    /// 正文变了：把这一次编辑交给 `debug.relocateBreakpoints`（Core）。
    ///
    /// 编辑区已经保证了"只在真的有基线且真的变了"时才报（见 `on_input_change`），
    /// 所以这里不再重复判据。
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

    /// 「清空断点」（真源 `debugger.clearBreakpoints`）。
    ///
    /// ⚠️ **保留项**：外壳侧这个入口本身是好的 —— 它只是把动作交给断点表的 owner
    /// （`debug` 实体；键的换算要工作区根，两样都在外壳手里），与
    /// `toggle_breakpoint` / `remove_breakpoint` 是同一个形状。它现在没有调用方，
    /// 是因为**界面还没有这个入口**：工具栏只有「查看断点 / 静音断点」，断点面板、
    /// 行号槽右键、运行菜单（`menu_bar.rs` 只有 `ToggleBreakpoint` 一项）都没有「清空断点」。
    /// 真源那一项在断点编辑器里，本侧还没有对应控件 —— 补控件是新行为，不属于"收警告"，
    /// 所以这里按既有先例保留并写明原因，而不是现造一个假入口或删掉已完成的动作实现。
    ///
    /// ⚠️ 顺带钉住一个容易搞错的边界：**清空本身不需要会话**。
    /// `DebugPane::clear_breakpoints` 先把宿主断点表清掉、让行号槽重画，只有"每个文件发一条
    /// 空集"的 DAP 通知走派发闸门；无会话时那一步只留诊断（`reason=no-session (H10/H11)`，
    /// 没有 transport 就没有会话 ⇒ `debug.setBreakpoints` 的 `with_session` 路径必然
    /// `session_not_found`），清空在界面上仍然是生效的。
    #[allow(dead_code)]
    pub(crate) fn clear_breakpoints(&mut self, cx: &mut Context<Self>) {
        self.debug.update(cx, |pane, cx| pane.clear_breakpoints(cx));
    }

    /// 会话文档里的界面部分（外壳那六个可见性/视图值 + 三块面板的尺寸 →
    /// [`lithe_gpui_shared::workspace_config::SessionUiState`]）。
    ///
    /// ⚠️ 尺寸这里给的是**内存里的当前值**，不是钳制以后的值：调用点只在"真的改了会话内容"
    /// 的路径上（含手势结束那一次），而手势结束时 [`Self::end_pane_resize`] 已经按当时的窗口
    /// 钳过一遍。给原始值不会写出越界文档。
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

    /// 这块面板的当前长度（规格像素）。
    fn pane_span(&self, pane: ResizablePane) -> f32 {
        match pane {
            ResizablePane::LeftSidebar => self.left_sidebar_width,
            ResizablePane::RightToolWindow => self.right_tool_window_width,
            ResizablePane::BottomPane => self.bottom_pane_height,
        }
    }

    /// 写回这块面板的长度。**只改内存值**，调用方决定要不要 `notify` / 落盘。
    fn set_pane_span(&mut self, pane: ResizablePane, span: f32) {
        match pane {
            ResizablePane::LeftSidebar => self.left_sidebar_width = span,
            ResizablePane::RightToolWindow => self.right_tool_window_width = span,
            ResizablePane::BottomPane => self.bottom_pane_height = span,
        }
    }

    /// 这块面板在给定窗口尺寸下允许的长度区间 `(min, max)`。
    ///
    /// ## 左栏（2026-10-03 按真源改）
    ///
    /// 下界 30（`WorkbenchLayoutStore.swift:4` 的 `minimumSidebarWidth`）、上界
    /// `min(520, 分栏容器宽 − 400)`（`WorkbenchView.swift:2008-2011`，公式见
    /// [`left_sidebar_max_width`]）。改前是静网 200…720（2026-09-27 维护者口径），
    /// **这条会改可见行为**：能拖的范围从 200 起变到 30 起、上限也会随窗口宽收敛。
    ///
    /// ## 容器尺寸的取法（第 4 条遗留）
    ///
    /// 左右两个轴的上界都应该相对**分栏容器**，而不是 `window.viewport_size()`（窗口内框）。
    /// 本侧能拿到容器的只有一处：`render` 里量到的 [`ShellWorkspace::split_container`]
    /// （上一帧 prepaint 的实测尺寸），它是真源那个 `GeometryReader` 的对应物。
    ///
    /// - **左栏**用它的 `width`：真源同一个 `availableTopWidth`（`WorkbenchView.swift:2000-2010`）；
    /// - **底部窗**用它的 `height`：真源同一个 `geometry.size.height`（`:2022-2025`）；
    /// - **右工具窗**仍只能用窗口内宽，**是近似**：真源给它的容器
    ///   （`WorkbenchRightToolSplitView` 的 `geometry` = 窗口减两条活动栏、**包含**右工具窗）
    ///   在本侧的树里没有对应元素（两条活动栏是本侧那一行的**兄弟**，不是外框），
    ///   而若改用 `split_container.width` 会**重复扣掉右工具窗自己**。差额与后果写在
    ///   [`right_tool_window_max_width`]。
    ///
    /// ⚠️ `viewport` 仍是**兜底输入**：容器还没量到（构造期 / 恢复会话 / 首帧）时回落到它，
    /// 与"窗口尺寸不可知时给静网上界"同一条口径 —— 宁可那几帧不收敛，也不能按 0 把用户
    /// 存下来的尺寸钳到下界。
    fn pane_span_range(&self, pane: ResizablePane, viewport: Size<Pixels>) -> (f32, f32) {
        // 容器实测尺寸（逻辑 px）；`None` = 还没 prepaint 过 ⇒ 回落窗口内尺寸。
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

    /// 手柄按下：开一次拖拽会话。锚点等**首个移动事件**再记（理由见 [`PaneResizeDrag::origin`]）。
    ///
    /// 与拆分"按下"和"移动"两件事的理由：`.on_drag` 构造器只拿到相对手柄原点的 offset，
    /// 而窗口级移动监听拿到的是窗口坐标 —— 后者才是可直接算的。
    fn begin_pane_resize(&mut self, pane: ResizablePane) {
        self.pane_resize_drag = Some(PaneResizeDrag { pane, origin: None });
    }

    /// 拖动中：改**内存值** + `notify`。**不落盘**（落盘在 [`Self::end_pane_resize`]）。
    ///
    /// 三条纪律（`.agents/notes/implemented/architecture/2026-09-13-resizable-ui-performance-boundaries.md`）：
    ///
    /// 1. 位移按"起点 + 位移"算，不逐帧累加（累加会随事件频率漂移，快拖与慢拖结果不同）；
    /// 2. 每帧只改这一个 `f32` 并 `notify` 一次，**不重建任何实体、不重新布局整棵树**——
    ///    面板长度是 render 里现读的局部变量，重绘范围就是这几块面板；
    /// 3. **值没变就不 `notify`**：拖到上下界之后继续拖，指针仍在动而长度已经不动了，
    ///    这时一次重绘都不该发生。
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
                // 会话的首个移动事件：只记锚点，这一帧不改长度（起点本身已经钳过）。
                session.origin = Some((along, current));
                return;
            }
        };
        if next != current {
            self.set_pane_span(pane, next);
            cx.notify();
        }
    }

    /// 手势结束（mouse up，或自愈发现左键已经松开）：收尾 + **在这里落盘**。
    ///
    /// ## 落盘点为什么在这里
    ///
    /// 那份 Note 的硬要求是"只在拖动结束时保存最终尺寸，不要在拖动过程中持续写入持久化存储"。
    /// 这个函数是拖动链路上**唯一**碰持久化的地方：拖动中的每一帧只走
    /// [`Self::drag_pane_resize`]（改内存值 + `notify`），一次文件操作都不做。
    ///
    /// 落盘本身也**不是同步写文件**：`schedule_session_save` 走既有的 [`SessionTracker`]
    /// 通路（300ms 去抖 + `plan` 判"内容没变就不排计时器"），真正的 `std::fs` 调用发生在
    /// 后台计时器醒来之后或进程退出前 —— UI 线程不做阻塞 IO。三块面板共用这一条通路，
    /// 没有第二个写文件的地方。
    ///
    /// 收尾时按**当时**的窗口尺寸再钳一次：拖到一半把窗口缩小 / 把手柄拖出窗口，
    /// 存下去的必须是"这个窗口放得下"的值（恢复时才不会越界）。
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
        // 诊断锚点（stderr）：拖动交互本身没法确定性单测（要真窗口 + 真指针），
        // 所以把"手势结束时的最终值"留成可 grep 的一行。
        eprintln!("S1_PANE_RESIZE pane={} span={resolved}", pane.id());
        self.schedule_session_save(cx);
        cx.notify();
    }

    /// 把一个文件在**项目树**里定位（选中那一行并滚动到可见）。
    ///
    /// 真源 `AppModel+FeatureState.swift:61` 的 `revealInProjectTree` —— 它只管发一个带 id 的
    /// 定位请求，真正做事的项目树在**当前内容必须是「项目」**的前提下消费它
    /// （`ProjectSidebarView.swift:218-228` 那颗 `scope` 按钮就带 `selectedContent == .project` 这个
    /// 前提）。所以这里先把左栏切到项目视图并放出来：否则"定位"会发生在一个看不见的视图里。
    ///
    /// 不调 [`ShellWorkspace::select_top_activity`]：那个方法的 IDEA 语义是"点当前项 = 收/放面板"
    /// （`top_activity_view` 不变、`left_pane_visible` 取反），对"程序化地切到项目视图"是错的。
    fn reveal_in_project_tree(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.top_activity_view = Some(DEFAULT_TOP_ACTIVITY);
        self.left_pane_visible = true;
        // 整块侧栏（活动栏 + 面板）也要放出来：真源只切 `selectedSidebar = .project`
        // （`AppModel+FeatureState.swift:66`），而本侧把活动栏算在那块可见性里 —— 不放开的话
        // `Ctrl+B` 收掉侧栏之后"定位"会落在一个谁也看不见的地方。
        self.left_sidebar_visible = true;
        let explorer = self.explorer.clone();
        let _ = explorer.update(cx, |explorer, cx| explorer.reveal(path, cx));
        cx.notify();
    }

    /// 选中左栏**顶部组**的第 `index` 项（项目 / 更改 / 搜索）。
    ///
    /// 两处入口**共用这一段**，不各写一遍：活动栏顶部组的点击
    /// （`on_select_activity` 的 `None` 分支）与 B2 的主菜单
    /// 「视图 → 文件资源管理器 / 源代码管理」。两处改的是同一份状态
    /// （[`ShellWorkspace::top_activity_view`]），而"切到「更改」要顺手重读一次工作区状态"
    /// 这条副作用（真源 `use-git-data-controller.ts:276-281`，本侧没有 watcher）必须跟着走，
    /// 否则从菜单进去会看到一个不刷新的列表。
    ///
    /// ## IDEA 语义（2026-09-27，维护者口径）
    ///
    /// - 点**当前已经高亮**的那一项 → **只收/放面板**（[`ShellWorkspace::left_pane_visible`] 取反），
    ///   活动栏自己**不动**（它常驻）；
    /// - 点**别的**项 → 切视图，并把面板放出来（切视图时面板不该还是收着的）。
    ///
    /// 这与 `Ctrl+B` / 菜单「视图 → 切换活动侧栏」**不是同一件事**：那两条仍然整块收
    /// （活动栏 + 面板），见 [`ShellWorkspace::toggle_left_sidebar`]。
    /// 判据写成"下标是否相等"而不是"面板此前是否可见"：后者在"面板收着、点了另一个项"时会
    /// 反向动作（切过去却把面板关掉），与直觉相反。
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
        let is_dark = cx.theme().is_dark();
        visible_commands(flags)
            .into_iter()
            .map(|id| command_action(id, flags, is_dark))
            .collect()
    }

    /// Search Everywhere 的 `Actions` 档要列的**宿主命令目录**（顺序 = [`CommandId::ALL`]）。
    ///
    /// 与 [`Self::command_actions`] 的唯一区别是"取哪些 id"：命令面板要的是
    /// [`visible_commands`] 那份 7 条短名单（主题那一对还要按当前配色二选一），
    /// 而 `Actions` 档是**整个可执行命令目录**（22 条，含只由菜单 / 快捷键进入的那批）——
    /// 真源的 Actions 档同样列 `LitheActionRegistry.actions(for:)` 的全部 35 条动作
    /// （`macos/Sources/Lithe/Models/LitheAction.swift:56-102`），不是命令面板的短名单。
    ///
    /// 朝向快照、主题深浅、以及每一条的 label / description / icon 全部走同一个
    /// [`command_action`]，所以**同一个动作在两个入口里的字面与字形逐字相同** ——
    /// 不是第二份动作表，只是同一份表取不同的行集合。
    ///
    /// 纯读：不改状态、不生成界面元素（与 [`Self::command_actions`] 同一口径）。
    pub(crate) fn action_catalog(&self, cx: &App) -> Vec<CommandAction> {
        let flags = self.action_flags(cx);
        let is_dark = cx.theme().is_dark();
        CommandId::ALL
            .into_iter()
            .map(|id| command_action(id, flags, is_dark))
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

    /// 「切换活动侧栏」唯一的状态改动点（左边一条 rail + 它那块面板一起收放）。
    ///
    /// 入口只有两条、都走这里：菜单「视图 → 切换活动侧栏」与键位 `Ctrl+B`
    /// （[`MenuAction::ToggleActivitySidebar`]）。
    ///
    /// ⚠️ **活动栏图标的点击不在这里**（2026-09-27 起）：那条路走
    /// [`ShellWorkspace::select_top_activity`] 的 IDEA 语义 —— 点当前高亮项**只收面板**
    /// （[`ShellWorkspace::left_pane_visible`]），活动栏常驻。原来还有一个「编辑区左边界收起侧栏」
    /// 按钮也调这里，那个按钮按维护者口径删掉了（见它的墓碑注释）。
    ///
    /// 真源语义：`menu.toggleActivitySidebar`（`mod+b`）= VS Code 的 `toggleSidebarVisibility`，
    /// 收起的是**活动栏 + 面板**整块（不是只收面板）。
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

    /// 打开 Quick Open 浮层（`Ctrl+P` / 菜单「转到 → 快速打开」）。
    ///
    /// 两个入口落到**同一个方法**：键位经 `crate::quick_open::install_actions` 的全局处理器
    /// 调它，菜单经 [`CommandId::OpenQuickOpen`] 在 [`Self::run_command_id`] 里调它
    /// ——"一个逻辑命令只建模一次"。
    ///
    /// 外壳只做三件事：把 `root` 与"打开文件"的通路交给浮层（`EditorPane::open`，
    /// Core 的路径命中 `line` 恒为 `null`，所以走 `open` 而不是 `open_at`）、把**当刻的**
    /// 索引就绪状态推进去（并登记弱句柄，之后状态真的变了时继续推）、再
    /// `window.open_dialog`。查询 / 结果 / 键盘全在 `crate::quick_open`。
    ///
    /// ⚠️ **只能从事件回调或任务里调用**：`render` 阶段调 `open_dialog` 会 panic。
    pub(crate) fn open_quick_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.root.clone();
        let editor = self.editor.clone();
        let on_open: Rc<dyn Fn(PathBuf, &mut Window, &mut App) + 'static> =
            Rc::new(move |path, window, cx| {
                // 与资源管理器点文件 / 查找面板点结果同一条通路（`EditorPane::open`）。
                let _ = editor.update(cx, |pane, cx| pane.open(&path, window, cx));
            });
        // 就绪状态**在打开这一刻读一次**（真源仍是 `search_index`），浮层不做任何"索引怎么样"
        // 的判断；之后的变化由外壳推（见 `broadcast_search_index_readiness`）。
        let readiness = self.search_index.readiness();
        let quick_open =
            cx.new(|cx| crate::quick_open::QuickOpen::new(root, on_open, readiness, window, cx));
        self.quick_open_overlay = Some(quick_open.downgrade());
        crate::quick_open::open_dialog(quick_open, window, cx);
    }

    /// 打开 Search Everywhere 浮层（菜单「视图 → 全局搜索」）。
    ///
    /// 外壳只做四件事：把 `root` 与"打开结果"的通路交给浮层、把"执行动作"的通路也交给它、
    /// 把**当刻的**索引就绪状态推进去（并登记弱句柄，之后变化继续推）、再 `window.open_dialog`。
    /// 查询 / 结果 / scope / 键盘全在 `crate::search_everywhere`。
    ///
    /// ⚠️ **只能从事件回调或任务里调用**：`render` 阶段调 `open_dialog` 会 panic。
    ///
    /// ⚠️ 这条动作**没有键位**（理由见 `CommandId::OpenSearchEverywhere` 的文档），
    /// 所以它是本 crate 里唯一"只由主菜单触发"的浮层入口。
    pub(crate) fn open_search_everywhere(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.root.clone();
        let editor = self.editor.clone();
        // 与资源管理器 / 查找面板**同一段**逻辑：有行号（符号 / 类型 / 正文命中）就跳那一行，
        // 没有（文件路径命中）就只打开文件。`origin_kind` 用 `"search-everywhere"`，
        // 与问题列表的 `"problems"` / 查找面板的 `"search"` 分开（列给 0，理由同查找面板：
        // Core 的 `SearchMatch` 只有行号没有列）。
        let on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static> =
            Rc::new(move |path, line, window, cx| {
                let _ = editor.update(cx, |pane, cx| match line {
                    Some(line) => {
                        pane.open_at(&path, line, 0, "search-everywhere", window, cx);
                    }
                    None => pane.open(&path, window, cx),
                });
            });
        // `Actions` 档与 `/` 命令模式按 `Enter` 时要执行的**动作 id**：浮层只报"用户选了这条"，
        // 执行点仍是唯一的那一个 [`Self::run_command_id`]（菜单与命令面板也走它）——
        // 与 `crate::command_palette::CommandPalette::confirm` 完全同一处置
        // （`'static` 回调捕获不了 `&self`，所以同样经 [`crate::command_palette::shell`] 拿句柄）。
        //
        // ⚠️ 浮层**先关自己再调它**：被选中的动作里就有「打开命令面板 / 全局搜索」这类
        // 会再开一个对话框的动作，而 `open_*` 都有"已经开着对话框就不叠第二层"的保护，
        // 留着本浮层就会静默什么都不发生（同 `CommandPalette::confirm` 的顺序理由）。
        let on_action: Rc<dyn Fn(CommandId, &mut Window, &mut App) + 'static> =
            Rc::new(|id, window, cx| {
                let Some(shell) =
                    crate::command_palette::shell().and_then(|handle| handle.upgrade())
                else {
                    // 工作台已经销毁（窗口正在关）：浮层已经关了，什么都不用做。
                    return;
                };
                shell.update(cx, |shell, cx| shell.run_command_id(id, window, cx));
            });
        // 与 Quick Open 同一处置：打开这一刻读一次就绪状态（真源是 `search_index`）。
        let readiness = self.search_index.readiness();
        let overlay = cx.new(|cx| {
            crate::search_everywhere::SearchEverywhere::new(
                root, on_open, on_action, readiness, window, cx,
            )
        });
        self.search_everywhere_overlay = Some(overlay.downgrade());
        crate::search_everywhere::open_dialog(overlay, window, cx);
    }

    /// 打开 Replace in Files 浮层（`Ctrl+Shift+R` / 菜单「转到 → 在文件中替换」）。
    ///
    /// 两个入口落到**同一个方法**：键位经 `crate::replace_in_files::install_actions` 的全局
    /// 处理器调它，菜单经 [`CommandId::OpenReplaceInFiles`] 在 [`Self::run_command_id`] 里调它
    /// ——"一个逻辑命令只建模一次"。
    ///
    /// 外壳只做三件事：把 `root` 交给浮层、把**当刻的**索引就绪状态推进去（并登记弱句柄，
    /// 之后变化继续推）、再 `window.open_dialog`。查询 / 选项 / 预览结果 / 键盘全在
    /// `crate::replace_in_files`。
    ///
    /// ⚠️ **这条界面只读、不写盘**：Core 在本分支**没有** apply / replace 命令
    /// （`protocol/command.rs:77-78` 只有 `workspace.replacePreview`），所以这里没有
    /// "点一下真的替换"的通路 —— 理由与它为什么不能在宿主里补，写在
    /// `crate::replace_in_files` 的模块文档上。
    ///
    /// ⚠️ **只能从事件回调或任务里调用**：`render` 阶段调 `open_dialog` 会 panic。
    pub(crate) fn open_replace_in_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.root.clone();
        // 与另两个浮层同一处置：打开这一刻读一次就绪状态（真源是 `search_index`）。
        let readiness = self.search_index.readiness();
        let panel =
            cx.new(|cx| crate::replace_in_files::ReplaceInFiles::new(root, readiness, window, cx));
        self.replace_in_files_overlay = Some(panel.downgrade());
        crate::replace_in_files::open_dialog(panel, window, cx);
    }

    /// 把索引的**就绪状态**推给四个搜索界面（索引链唯一的对外通知口）。
    ///
    /// 调用点只有一个：`crate::search_index` 的 `warm` 任务回到前台、`adopt` 说"真的变了"
    /// 的那一处（`search_index.rs` 的 `schedule_warm`）。所以：
    ///
    /// - **不轮询**：没有定时器、没有"每帧问一次"，只有一次真实的 `warm` 结果回来时走这里；
    /// - **不在 `render` 里做事**：本方法在任务回调里被调，四个界面的 `render` 只读自己那份值；
    /// - **单一 owner**：值来自 [`ShellWorkspace::search_index`]，界面那四个字段是**只读的展示
    ///   副本**（各自 `set_..` 只在真的变了时 `cx.notify()`），没有任何界面自己判"索引好了没"；
    /// - 打开浮层时的那一次推**不在这里**：浮层构造时就拿到了当刻的值（见三个 `open_*`），
    ///   这里只负责"开着的那一个"在状态变化后跟着变 —— 关掉的浮层 `upgrade` 失败，直接跳过。
    ///
    /// 左栏面板是常驻实体（与外壳同生命周期），所以它总是被推；三个浮层是**每次打开新建**的，
    /// 所以只推当下还活着的那个（弱句柄一 upgrade 就失败＝没开着）。
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
                diagnose_menu_run(action, if closed { "applied" } else { "no_tab" });
            }
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
            // -------------------------------------------------------------------
            // B2：编辑那九条 —— 派发给"打开菜单那一刻窗口的焦点元素"
            // -------------------------------------------------------------------
            //
            // 为什么不是"自己实现一遍"：撤销 / 复制 / 查找… 全部是上游 `Input` 上下文里
            // 已经绑好、且处理器已经挂在编辑器元素上的动作（`gpui-base-0.6.6/src/input/base/state.rs:246-302`
            // 绑键、`:4191` 一带挂处理器）。重写一遍等于把编辑器引擎的撤销栈 / 剪贴板语义抄一份，
            // 那是《编码指南》明确禁止的"平行实现一个成熟上游子系统"。
            //
            // 机制与它**如实**的语义（焦点目标从哪来、拿不到时怎么办）写在
            // `crate::menu_bar::ACTION_TARGET` 的文档上。
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
            // -------------------------------------------------------------------
            // B2：视图那九条
            // -------------------------------------------------------------------
            MenuAction::ToggleActivitySidebar => {
                // 真源 `menu.toggleActivitySidebar`（`mod+b`）= VS Code 的
                // `toggleSidebarVisibility`：收起的是**活动栏 + 面板**整块。
                // 2026-09-27：编辑区左边界那个「收起侧栏」按钮也走这一条（同一个状态、同一句
                // 诊断），不另立第二套状态 —— 契约与 `activity_bar.rs:61-67` 的
                // 「左栏顶部组/底部组各自独立高亮」同源。
                self.toggle_left_sidebar();
            }
            MenuAction::ToggleSecondarySidebar => {
                // 「辅助侧栏」= 右侧那块 400px 工具窗（真源 `menu.toggleSecondarySidebar`）。
                // 与 [`CommandId::ToggleMaven`] 的差别：这里只改**可见性**，不换视图
                // （真源 `right-tool-window-actions.ts:26-31` 的 toggle 分支同样不动
                // `activeRightSidebarView`）。懒扫照走，否则第一次展开会是一块空面板。
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
                // 「运行 → 开始调试」与标题栏的 🐞 落点相同（`lithe_gpui_run::RunPane::launch`）：
                // 同一条启动计划 + `debugPort`。面板自己会在起进程后让底部工具窗切到「运行」。
                self.run.update(cx, |pane, cx| pane.launch(true, cx));
                diagnose_menu_run(action, "debug");
            }
            MenuAction::StopDebugging => {
                self.run.update(cx, |pane, cx| pane.stop(cx));
                diagnose_menu_run(action, "stop");
            }
            MenuAction::ToggleBreakpoint => {
                // 「运行 → 切换断点」：U19 起**真的能用**（改前是
                // `NotWired { missing: MISSING_DEBUGGER_HOST }` 占位项）。
                // 落点与点行号槽完全同一条（`toggle_breakpoint_at_caret` → `toggle_breakpoint`）。
                self.toggle_breakpoint_at_caret(cx);
                diagnose_menu_run(action, "toggleBreakpoint");
            }
            MenuAction::GoToLine => {
                // 「转到 → 转到行…」：U20 起**真的能用**（改前声明缺编辑区公开入口）。
                // 对话框与落地都在编辑区（`EditorPane::open_go_to_line`）：它同时要
                // "当前光标"（预填）与"按当时的正文收敛"，两样都在编辑区手上。
                // `Ctrl+G` 走 `menu_bar::install_key_actions` → 压进本队列 → 落到**这里**，
                // 所以键与菜单是逐字同一条链。
                self.editor
                    .update(cx, |pane, cx| pane.open_go_to_line(window, cx));
                diagnose_menu_run(action, "goToLine");
            }
            MenuAction::ToggleWordWrap => {
                // 「视图 → 切换自动换行」：U20 起**真的能用**（改前声明缺编辑区公开入口）。
                //
                // 写的是**设置里的同一个键** `wordWrap`，不是一份会话内的临时状态：
                // 设置页那个开关与这一项必须共用一个真相源（两个值会互相覆盖 ——
                // 理由逐条写在 `lithe_gpui_settings::schema::Settings::word_wrap` 的字段文档里）。
                // 设置一变，上面的 `observe_in` 订阅立刻把它推给编辑区（含**已打开的**每个 buffer），
                // 所以这里**不**再自己调 `set_soft_wrap`（那会变成两处写入）。
                match lithe_gpui_settings::try_store(cx) {
                    Some(store) => {
                        let next = !store.read(cx).settings().word_wrap;
                        store.update(cx, |store, cx| store.set_word_wrap(next, cx));
                        diagnose_menu_run(action, "toggleWordWrap");
                    }
                    // 没有设置实体（测试宿主 / 将来别的宿主）：**不静默**，留一行诊断。
                    None => eprintln!("S1_MENU notWired id=lithe.menu.toggleWordWrap missing=no_settings_store"),
                }
            }
            MenuAction::ShowHover => {
                // 「编辑 → 显示悬停信息」：U20 起**真的能用**（改前声明缺 LSP 请求）。
                //
                // ⚠️ 这不是"鼠标悬停"那条路（那条由上游在 `handle_mouse_move` 里自动触发，
                // 不需要菜单项）：本项是**键盘**入口 —— 把光标处的悬停文档弹出来，
                // 落点 `EditorPane::show_hover_at_caret`（用上游公开的 `present_hover`）。
                self.editor
                    .update(cx, |pane, cx| pane.show_hover_at_caret(window, cx));
                diagnose_menu_run(action, "showHover");
            }
            MenuAction::ShowDiagnostics | MenuAction::ShowRunAndDebug => {
                // 接线点：底部工具窗的**同一份状态**（`bottom_kind` + `bottom_visible`）——
                // 活动栏底部组的「诊断 / 运行」两项点下去改的就是这两个字段
                // （`on_select_activity` 的 `Some(kind)` 分支）。
                //
                // ⚠️ **U19 改了这一条的落点**：「视图 → 运行和调试」从「运行」面板改到
                // **调试工具窗**。理由：真源里"运行和调试"就是**调试工具窗**
                // （`Views/Debug/GenericDebugView.swift` 自带 `Debugger | Console` 两个
                // content tab，`:96-133`，Console 就是运行输出），而 U19 之前宿主没有调试面板、
                // 只能临时指到运行面板。运行面板本身仍从活动栏「运行」项和标题栏 ▶ 进。
                self.bottom_kind = match action {
                    MenuAction::ShowRunAndDebug => BottomPaneKind::Debug,
                    _ => BottomPaneKind::Diagnostics,
                };
                self.bottom_visible = true;
                self.refresh_problems_if_visible(cx);
                diagnose_menu_run(action, "visible");
            }
            MenuAction::ShowFileExplorer | MenuAction::ShowSourceControl => {
                // 「文件资源管理器」= 左栏顶部组的第 0 项、「源代码管理」= 第 1 项
                // （`activity_items` 的顺序，与活动栏图标同源）。
                let index = if action == MenuAction::ShowSourceControl {
                    CHANGES_ACTIVITY_IX
                } else {
                    DEFAULT_TOP_ACTIVITY
                };
                // 顺带把整块左栏显示出来：侧栏被 `Ctrl+B` 收起时，"打开文件资源管理器"
                // 却什么都没出现会像死项。活动栏点击路径不会遇到这个（收起时它自己也点不到）。
                self.left_sidebar_visible = true;
                self.select_top_activity(index, cx);
                diagnose_menu_run(action, "applied");
            }
            MenuAction::ZoomIn | MenuAction::ZoomOut | MenuAction::ResetZoom => {
                self.apply_zoom(action, cx);
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
            | MenuAction::ToggleMenuBar
            | MenuAction::GoToDefinition
            // 「快速打开」走 `command_id()` 那一支（`CommandId::OpenQuickOpen`），
            // 这里的臂与上面几条一样**不可达**，列出来只为让 match 穷尽。
            | MenuAction::QuickOpen
            // 「全局搜索」同理（`CommandId::OpenSearchEverywhere`）。
            | MenuAction::SearchEverywhere
            // 「在文件中替换」同理（`CommandId::OpenReplaceInFiles`）。
            | MenuAction::ReplaceInFiles => {
                diagnose_menu_run(action, "unreachable");
            }
        }
        // 菜单里那几条视图动作（切换活动侧栏 / 辅助侧栏 / 诊断 / 运行 / 资源管理器…）改的是
        // 会话文档里的字段，所以在这里统一排一次。**不是每一条都会真的排**：
        // `SessionTracker::plan` 判"内容没变"就直接返回，缩放、编辑类动作都落在那一支。
        self.schedule_session_save(cx);
        cx.notify();
    }

    /// 「编辑」菜单那九条的执行：把它们**派发给打开菜单那一刻窗口的焦点元素**。
    ///
    /// 机制、以及"为什么不能更直接"（编辑器元素的 `FocusHandle` 在 editor crate 内部，
    /// 本批写域不含它）写在 `crate::menu_bar::ACTION_TARGET` 的文档上。这里只做三件事：
    ///
    /// 1. 取目标；没有（启动后什么都没点过就点菜单）就**如实**打一行 `state=no_target`
    ///    并返回 —— 不假装成功，也不静默；
    /// 2. 把焦点交给它（`dispatch_action` 走"焦点节点 → 祖先"，焦点不在它身上就什么都碰不到）；
    /// 3. `Window::dispatch_action(上游动作)` —— 与用户在编辑器里按 `Ctrl+Z` 命中的
    ///    **是同一个 action、同一个处理器**。
    ///
    /// ⚠️ `dispatch_action` 是**延后**执行的（`gpui-pre-0.3.6/src/window.rs:2442-2454` 里
    /// `cx.defer`），但它**当场**就把当前焦点句柄记了下来，所以本函数之后菜单栏把焦点
    /// 还给外壳根不影响这次派发。
    fn dispatch_editor_action(
        &mut self,
        action: MenuAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(bound) = action.keybind_action() else {
            // 只有"编辑"那九条会走到这里，而它们每一条都有上游动作；真出了这种情况
            // 说明 `keybind_action` 与调用点漂移了 —— 打一行诊断而不是 panic（界面不该崩）。
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

    /// 「视图 → 放大 / 缩小 / 重置缩放」（`Ctrl+=` / `Ctrl+-` / `Ctrl+0`）。
    ///
    /// ⚠️ **语义映射（有意偏离真源，逐条写清）**：真源这三条是 Monaco 的**视图级** fontZoom
    /// （`editor.action.fontZoomIn/Out/Reset`，只改编辑器字号、不落盘）。gpui 侧没有视图级字号：
    /// 正文字号唯一的落点是设置项 `fontSize`（`settings/src/schema.rs` 的 10..=22，
    /// 经 `theme::apply_editor_font_size` 写到主题的 `mono_font_size`，编辑器与终端正文共用）。
    /// 所以这里改的就是它：放大 / 缩小 ±1，重置回到默认值 —— 代价是**会落盘**
    /// （真源不落盘），收益是零新增机制、且与设置页里那一格是同一个值。
    /// 越界钳制不在这里做：`SettingsStore::set_editor_font_size` 自己会归一
    /// （`normalize_editor_font_size`），本函数读回**归一之后**的值打诊断。
    fn apply_zoom(&mut self, action: MenuAction, cx: &mut Context<Self>) {
        let Some(store) = lithe_gpui_settings::try_store(cx) else {
            // 没有设置状态（测试宿主）：不装作做成了。
            diagnose_menu_run(action, "unavailable");
            return;
        };
        let current = store.read(cx).settings().font_size;
        let next = match action {
            MenuAction::ZoomIn => current + 1.0,
            MenuAction::ZoomOut => current - 1.0,
            // 重置 = 真源 `fontZoomReset` 的等价物。真源回到"设置里配置的值"，而本侧的
            // 缩放本身就是改那个设置，所以"重置"= **默认值**（不写死 14：从 `Settings::default()` 取）。
            _ => lithe_gpui_settings::Settings::default().font_size,
        };
        store.update(cx, |store, cx| store.set_editor_font_size(next, cx));
        let applied = store.read(cx).settings().font_size;
        // 字号是"这条动作真的改到了东西"的唯一机器可读证据（界面上只是字变大变小）。
        eprintln!("S1_ZOOM font_size={applied} requested={next}");
        diagnose_menu_run(action, "applied");
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
            CommandId::OpenQuickOpen => {
                // 接线点：[`Self::open_quick_open`] —— 与 `Ctrl+P` 的全局处理器
                // （`crate::quick_open::install_actions`）落到**同一个方法**。
                diagnose_run(id, "open");
                self.open_quick_open(window, cx);
            }
            CommandId::OpenSearchEverywhere => {
                // 接线点：[`Self::open_search_everywhere`] —— 菜单「视图 → 全局搜索」的
                // 唯一执行点（它没有键位，见 `CommandId::OpenSearchEverywhere` 的文档）。
                diagnose_run(id, "open");
                self.open_search_everywhere(window, cx);
            }
            CommandId::OpenReplaceInFiles => {
                // 接线点：[`Self::open_replace_in_files`] —— 与 `Ctrl+Shift+R` 的全局处理器
                // （`crate::replace_in_files::install_actions`）落到**同一个方法**。
                diagnose_run(id, "open");
                self.open_replace_in_files(window, cx);
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
                // 打开文件时中央区让位回编辑器：扩展页开着就收掉（与资源管理器点文件同一条）。
                let shell_for_open = cx.entity().downgrade();
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
                        // 中央区让位回编辑器：文件真的要开了，扩展页不再占着。
                        let _ = shell_for_open.update(cx, |this, cx| {
                            this.close_extensions_page(None, cx);
                        });
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
        // 命令面板里的视图动作（终端 / Maven 工具窗）同样改会话文档里的字段；
        // 与 `apply_menu_action` 同一处置（内容没变时 `plan` 不排计时器）。
        self.schedule_session_save(cx);
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

    /// 状态栏左组（前导项）：**通知中心入口**（有未读时）+ 项目名 + 分支。
    ///
    /// 2026-09-27 改过。顺序真源仍是
    /// `FOOTER_LEADING_ITEM_IDS = ["filePath", "branch"]`（`item-order.ts:20-32`），但前面
    /// 多了一格「通知」—— 见下面「为什么它不是另一条临时提示」。
    ///
    /// 两格老改动与上一轮不同，仍然成立：
    ///
    /// - 第一项（项目名）的图标从写死的 `IconName::FileText` 换成
    ///   [`StatusEntry::with_file_icon`] —— 按**当前活动文件名**查真机默认图标主题
    ///   （`idea-icons`），查不到才落 Lucide。真机这一格就是 `ThemedFileIcon`
    ///   （`file-path-breadcrumb.tsx:196-202`），所以现在与文件树 / 标签条同一套美术。
    ///   没有活动文件时传空名字，落到 `defaultFile`（`text`）。
    /// - 第二项（分支）取自 Git 视图的 `git.status` 快照（[`ChangesView::branch`]）——
    ///   每帧只做一次**内存读取**，不再有文件 I/O（D11；此前是每帧读 `.git/HEAD`）。
    ///   分支名的更新时机跟着快照走：Git 视图刷新一次（激活 / 手动刷新 / 写后刷新）
    ///   这一格就跟着变（真机是 git 状态推送）。
    /// 状态栏左组（前导项）：**通知中心入口**（有未读时）+ 项目名 + 分支。
    ///
    /// 2026-09-27 改过。顺序真源仍是
    /// `FOOTER_LEADING_ITEM_IDS = ["filePath", "branch"]`（`item-order.ts:20-32`），但前面
    /// 多了一格「通知」—— 见下面「为什么它不是另一条临时提示」。
    ///
    /// 原来第一格是 `status_notice`（一条 4 秒后自己消失的临时消息，带代数防串），随通知
    /// 中心一起废掉：说完就没了，用户事后无从查证。
    ///
    /// 现在这一格是**通知中心里最新那一条的文案，点它打开右工具窗的「通知」**。
    ///
    /// ## 为什么它不是「另一条临时提示」
    ///
    /// 它和中心里那一条是**同一个数据的第二个视图**，不是第二次记录。IDEA 的口径正是
    /// 这样：官方文档写状态栏里能看到的消息「点它会在工具窗里打开」——
    /// `https://www.jetbrains.com/help/idea/notifications.html` 的 Notifications tool
    /// window 一节。所以不存在「哪条通知只进了状态栏没进中心」的分裂。
    ///
    /// ## 为什么有未读才占这一格
    ///
    /// 它占了左组第一格，**有未读时**才出现，所以界面上是「暂时多一条」而不是
    /// 「少了项目名和分支」。仍然有未读就一直显示 —— 与 IDEA 一致。
    ///
    /// ## 为什么参数是 `&mut Context<Self>`
    ///
    /// 因为「点它打开中心」要拿到 `&mut ShellWorkspace`（切 `right_view` + 推水位线），
    /// 而 [`StatusEntry::on_click`] 的处理器只有 `(&ClickEvent, &mut Window, &mut App)`。
    /// 桥是 `cx.listener` —— 它把 `&mut Self` 闭包适配成那个签名（`status_bar.rs` 的
    /// `entry_chip` 用 `when_some(..)` 挂上去）。
    fn footer_left(&mut self, cx: &mut Context<Self>) -> Vec<StatusEntry> {
        let mut entries: Vec<StatusEntry> = Vec::new();

        // 通知**不再**回显到状态栏（2026-09-29 产品决定：即时反馈只有左下角 toast，
        // 记录在通知中心）—— 这里原来挂着"最新一条 + 点它开中心"的入口，已随红框
        // 一并移除，水位线推进挪到 [`ShellWorkspace::note_notifications_opened`]。

        let project_name: SharedString = self
            .projects
            .first()
            .map(|tab| tab.name().clone())
            .unwrap_or_default();
        let active_file = self.editor.read(cx).active_buffer_name();
        // 分支：走 Git 视图已经取到的 `git.status` 快照（`ChangesView::branch`），
        // **不再每帧读 `.git/HEAD`**（D11）。那三处读错的场景 —— worktree（`.git` 是文件）、
        // 分离 HEAD（没有 `ref: refs/heads/` 前缀）、仓库根≠工作区根 —— 都由 Core 的
        // `git.status` 判定，前端只负责显示。这是一次内存读取，没有 I/O。
        // 还没读到（首帧 / 非仓库）时显示 `—`，与原来"读不到就 `—`"同一口径。
        let branch = self
            .changes
            .read(cx)
            .branch()
            .map(|branch| branch.to_string())
            .unwrap_or_else(|| "—".to_string());

        entries.push(StatusEntry::new(project_name).with_file_icon(&active_file, cx));
        // 真源：`icons/idea/icons/ui/branch.svg(+_dark)` —— 真机 `GitBranchIcon`。
        entries.push(
            StatusEntry::new(SharedString::from(branch)).with_idea_icon(&idea::GIT_BRANCH_ICON, cx),
        );
        entries
    }

    /// 把 store 里**还没弹过**的条目逐条弹成左下角 toast。
    ///
    /// [`ShellWorkspace::_notification_toast_subscription`] 的回调体。store 一次批量写入
    /// 可能只触发一次 notify，所以不能用"弹最新一条"：[`lithe_gpui_notify::Store::entries_since`]
    /// 按水位线（[`ShellWorkspace::notification_toast_seq`]）做增量查询，这里按发生顺序
    /// 逐条弹完，再把水位线推进到最后一条。
    ///
    /// 弹窗口径（2026-09-29 产品决定）：**左下角**、**5 秒自动消失**。左下角用
    /// `Notification::placement(Anchor::BottomLeft)` 逐条指定；5 秒是 gpui-kit 0.7 的
    /// `autohide` 默认时长（`notification.rs` 的 `timeout: autohide.then_some(from_secs(5))`），
    /// 库里没有调时长的公开 API，所以不显式设置、靠默认值兑现需求。
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
        // 顺带驱动外壳重绘：铃铛角标读 store（`right_activity_items`），而编辑区是
        // 直接 `store.update` 记通知、不经过外壳的 —— 外壳原本不观察 store，角标要等
        // 下一次无关重绘才刷新。这里在弹 toast 的同一帧把它一起刷掉。
        cx.notify();
    }

    /// 单条 toast：文案与通知中心**同一份**（[`crate::notifications::render_message`]），
    /// 类型按严重性档位映射（`Severity` 没有"成功"档，`NotificationType::Success` 用不到）。
    ///
    /// 位置：**右下角**（`Anchor::BottomRight`）。真源是 `.overlay(alignment: .bottomTrailing)`
    /// （`macos/Sources/Lithe/Views/Workbench/WorkbenchView.swift:517`）—— 改前是
    /// `BottomLeft`，与真源左右相反。
    ///
    /// ⚠️ **两条内衬已在 `settings` 侧接上（`U25`）**：真源那一摞还有
    /// `padding(.trailing, rightActivityBarWidth + 12)`（= 52，现派生自
    /// `shared::design::ACTIVITY_RAIL_WIDTH_SPEC` + 12，见 `BLOCKERS.md` B14）、
    /// `padding(.bottom, 38)`、
    /// 单条 `minWidth 280 / maxWidth 360`、圆角 7、堆叠间距 8（`WorkbenchView.swift:519,543-554`）。
    /// 上游 `NotificationSettings` 的 `margins` 与 `width` **都是 pub**
    /// （`gpui-component-0.7.0/src/notification.rs:543,547`），经
    /// `Theme::global_mut(cx).notification`（`theme/mod.rs:184`）宿主**可设** —— 所以内衬不是
    /// "没有入口"，它已经接在 `crates/settings/src/theme.rs` 的 `apply_theme_by_name` 里。
    /// 真正缺的是 **280–360 的自适应宽度**（上游 `width` 是单值），
    /// 以及堆叠间距：上游 `expanded_gap` 固定 14（`gpui-base-0.7.0/src/toast.rs:51`，真源是 8），
    /// 而建栈处 `ToastStack::new(stack_id, stack.state.clone())`
    /// （`gpui-component-0.7.0/src/notification.rs:1048`）**没把 motion 传进去**，宿主改不到。
    /// 同一落点还接了"同时可见条数上限 = 3"。
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

    /// 打开通知面板时把未读水位线推上去。
    ///
    /// 「推水位线」是这个动作的**一半** —— IDEA 的红点是「有未读」，而用户点开铃铛
    /// 就是在说「我知道了」。不推的话角标永远不消。原来挂在状态栏回显的点击上，
    /// 回显废掉后这是唯一的推点（右活动栏点铃铛 / 菜单与探针的打开路径都调它）。
    fn note_notifications_opened(&mut self, cx: &mut Context<Self>) {
        self.notifications
            .update(cx, |store, _| store.mark_all_read());
        let read_upto = self.notifications.read(cx).read_upto_seq();
        eprintln!("S1_NOTIFICATION action=open readUpToSeq={read_upto}");
    }

    /// 状态栏右组（尾随项）：顺序与内容都照真机，**每次重绘时按当前状态算**。
    ///
    /// 顺序真源：`features/layout/config/item-order.ts:20-32` 的
    /// `FOOTER_TRAILING_ITEM_IDS = ["cursor", "encoding", "indent", "readOnly", "memory", "gitChanges"]`。
    ///
    /// 与上一轮的差别：光标位置原来是写死的 `"1:1"`，现在是**真实值**
    /// （[`EditorPane::cursor_position`]）；编码与缩进这两格按当前编辑器/读盘口径核过，
    /// 写出来的就是真值（理由见下面各自的注释），不再是"随手填的占位"。
    /// 只读态取的是真实值；**更改数**仍是占位（只有图标，理由写在旁边）。
    ///
    /// ⚠️ **内存格已撤除，不是修好**：它原先写死 `"总计 0.0 MB · Lithe 0.0 MB"`，
    /// 而真值要平台侧 10s 轮询的内存读数（真源 `WorkbenchStatusViews.swift:31-41`）。
    /// 显示一个编造的数字比不显示更糟，所以这一格先不画；平台端口落地后再接回
    /// （缺口登记在 `lithe-gpui/UI-MAP.md` 的校正专章）。
    fn footer_right(&self, cx: &App) -> Vec<StatusEntry> {
        // 没有任何 buffer 时编辑区给 `1:1`（真机同样：拿不到位置时用
        // `INITIAL_CURSOR_POSITION = {0,0}`，显示成 `1:1`）。
        let cursor = self.editor.read(cx).cursor_position(cx);
        // 缩进格的文案要一个 `&str`：`tr_args` 借的是字符串，所以先绑成局部量，
        // 不能在 `vec![..]` 的实参里现借一个马上析构的临时 `String`。
        let tab_size = self.editor.read(cx).tab_size().to_string();

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
            // 缩进：编辑器的制表宽度**取真值**（`EditorPane::tab_size`），
            // 它由外壳在启动期与每次设置变化后经 `set_tab_size` 转发进编辑区，
            // 所以设置页改 `tabSize` 会立刻反映到这一格（不再写死 2）。
            // 文案形如 `{count} 个空格`（`i18n/locale.ts` 的 `footer.spaces`）。
            StatusEntry::new(lithe_gpui_shared::tr_args(
                "lithe.footer.spaces",
                &[("count", tab_size.as_str())],
            )),
            // 只读态：取 `Buffer::writable` 的取反（`EditorPane::active_buffer_is_readonly`），
            // 与建 `EditorState` 时 `set_readonly(!writable)` 用的是同一个值，所以状态栏
            // 显示的和编辑器实际是否接受输入**同源**。
            // 真源 `footer-editor-status.tsx:102` 用 `LockIcon` / `LockOpenIcon` 区分两态。
            // 真源图标：`icons/idea/icons/ui/locked.svg(+_dark)` / `unlocked.svg(+_dark)`。
            if self.editor.read(cx).active_buffer_is_readonly() {
                StatusEntry::new(SharedString::from("")).with_idea_icon(&idea::LOCK_ICON, cx)
            } else {
                StatusEntry::new(SharedString::from("")).with_idea_icon(&idea::LOCK_OPEN_ICON, cx)
            },
            // ⚠️ Git 更改数：要等 `git.*` 提供工作区状态，本轮仍是空占位（只有图标）。
            // 真源：`icons/idea/icons/ui/successDialog.svg(+_dark)` —— 真机 `CheckCircleIcon`。
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

    /// 底部工具窗当前显示的是「诊断」页时，让问题列表重新问一次服务端。
    ///
    /// 为什么外壳要管这件事：面板自己不知道"用户什么时候打开了我" —— 那是外壳的布局状态
    /// （[`Self::bottom_visible`] / [`Self::bottom_kind`]）。而诊断可能在面板不可见的
    /// 那段时间里变多了（另一个文件报错、项目构建完成），所以切过去时补一次刷新，
    /// 与"编辑区每次诊断落地后刷新"（`editor::problems` 的表）一起构成完整的刷新时机。
    ///
    /// 不满足条件时**什么都不做**：没打开的面板刷不刷新都看不见，白花一次查询。
    fn refresh_problems_if_visible(&mut self, cx: &mut Context<Self>) {
        if self.bottom_visible && self.bottom_kind == BottomPaneKind::Diagnostics {
            self.editor.update(cx, |pane, cx| pane.refresh_problems(cx));
        }
    }

    /// 右活动栏第 `index` 项是否画选中底色。
    ///
    /// 判据照真机 `plugin-activity-rail.tsx:24-26`（Maven）与
    /// `notifications-trigger.tsx:18-21`（通知）：
    /// `isRightSidebarVisible && activeRightSidebarView === "<该项>"`。
    /// **扩展拼图项例外**（2026-09-29 起）：它的真机判据是「扩展视图是不是打开着」，
    /// 这里就是"中央扩展页开着"（[`ShellWorkspace::extensions_page_open`]）——
    /// 它不是右工具窗视图，不参与 `right_visible` / `right_view` 那套状态。
    ///
    /// 推论（都是有意行为）：**面板收起时通知与 Maven 不亮**；点另一项时旧项立刻灭、新项亮。
    fn is_right_activity_active(&self, index: usize) -> bool {
        if index == EXTENSIONS_RAIL_IX {
            return self.extensions_page_open;
        }
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
    /// 同一段观测窗口（`lithe-gpui/research/click-double-trigger-dpi.md` §5 末尾单独登记过这条）。
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
        eprintln!("S1_RIGHT_SCAN view={} root={}", view.id(), root.display());
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
                // 扩展 / 通知没有数据层（扩展页在 `extensions_page` 自己的构造任务里拉
                // 目录，通知是推送写入）：`RightScanState::request` 不会为它们返回
                // `Some`，这里只是把匹配写穷尽。
                RightToolWindowView::Notifications => {}
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
    /// （`S1_RIGHT_PANEL … visible=true` 紧跟一行 `visible=false`，见 `lithe-gpui/HANDOFF.md` §2），
    /// 面板随即被自己收起，于是"面板里的内容长什么样"这件事在无人值守环境里取不到证。
    /// 与 `--open-settings` / `--menu-probe` 同一条口径：被绕开的只有"操作系统把那一下点击
    /// 送进窗口"这一段，它写的两个字段与点击回调写的是**同一个**（不是另一套状态）。
    pub fn show_right_view_probe(&mut self, view: RightToolWindowView, cx: &mut Context<Self>) {
        self.right_view = view;
        self.right_visible = true;
        // 与右栏点击同一段迁移的一部分：打开通知面板同样推水位线。
        if view == RightToolWindowView::Notifications {
            self.note_notifications_opened(cx);
        }
        self.schedule_right_view_scan(view, true, cx);
        // 探针不是指针输入：没有"本次点击的坐标"，所以不带 `x=` / `y=`
        // （判据要求"坐标相同"，编一个坐标出来会污染证据）。
        diagnose_right_panel(view, true, None);
        cx.notify();
    }

    // -----------------------------------------------------------------------
    // 中央扩展页（`crate::extensions_page`，2026-09-29 起）
    // -----------------------------------------------------------------------

    /// 打开中央扩展页（右活动栏拼图项 / `--right-view extensions` 探针共用）。
    ///
    /// 实体**懒创建**：首次打开才建（它构造后会自己拉一次目录数据）；创建后常驻，
    /// 重开不丢搜索与筛选状态（与左栏双实体同一口径）。
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

    /// 关闭中央扩展页（中央列让位回编辑器）。
    ///
    /// 「打开文件时收起」走这里：资源管理器点文件（`on_open`）与命令面板
    /// 「打开文件」两条路径都在开文件前调它 —— 用户既然要编辑器，页面就该让位。
    pub(crate) fn close_extensions_page(
        &mut self,
        point: Option<ProbePoint>,
        cx: &mut Context<Self>,
    ) {
        self.extensions_page_open = false;
        diagnose_extensions_page(false, point);
        cx.notify();
    }

    /// 拼图项的开关语义：再点一次关闭（与右栏 toggle 同一口径）。
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
            eprintln!("S1_OPEN_PROJECT state=missing path={path_text} marked={marked}");
            // 两句真源既有文案（`fileSystem.recentProjectNotFolder` /
            // `fileSystem.recentProjectUnavailable`，`locale.ts:650-653`）：路径**在**但不是
            // 文件夹 vs 路径**不在**（或读不了）—— 真源也是这么分的（`stores/recent-folders.store.ts:106-120`）。
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
        self.notify_not_wired("lithe.gpui.newWindowNotWired", "New Window", cx);
    }

    /// **占位项**（B3）被点时的落点：一行可 grep 诊断 ＋ 通知中心里一条"尚未接入：缺 X"。
    ///
    /// 两件事都在这里做（不在菜单栏那一侧）：通知 store 是**外壳的一个字段**
    /// （[`ShellWorkspace::notifications`]，换根时显式清空），而菜单栏只有 `&mut App`
    /// ——它够不到外壳的 `&mut self`。所以菜单栏只把"哪一项 × 缺什么"塞进队列
    /// （[`MenuRequest::NotWired`]），执行落在这里。
    ///
    /// 原来这一族走状态栏那条 4 秒小字（`show_status_notice`，带**代数防串**），2026-09-27
    /// 随通知中心一起废掉。去重键固定是 `not-wired`：用户连点 20 个占位项时，中心里是
    /// **一条**「尚未接入」且次数变成 20，而不是 20 行几乎一样的话。
    fn report_menu_not_wired(
        &mut self,
        label_key: &'static str,
        missing: Missing,
        cx: &mut Context<Self>,
    ) {
        // 诊断格式的唯一拼装点（`crate::menu_bar::Missing::diagnose`），验证脚本按它 grep：
        // `S1_MENU notWired id=menu.newTab missing=file_lifecycle`。
        missing.diagnose(label_key);
        self.notify_not_wired(missing.text_key, label_key, cx);
    }

    /// 项目下拉里**占位项**被点时的落点（B3 的「克隆仓库…」、B3 之后的「新建项目…」）。
    ///
    /// ⚠️ 与菜单栏那 51 条占位项**同一个口径**（都是 [`Missing`] 声明 → "尚未接入：缺 X"），
    /// 但诊断前缀是 `S1_PROJECT_MENU`（那一行的 `action=<id>` 与 `diagnose_actions` 的打开期
    /// 诊断同一格式）。这几条不在 89 条菜单项里（真源把它们放在**标题栏项目下拉**），
    /// 所以不经过 [`MenuRequest`] 那条队列。
    ///
    /// `action_id` 是 [`crate::project_menu`] 的 `PanelAction::id()`；`missing` 是这一条缺的
    /// **能力组**（「克隆仓库…」是 [`crate::menu_bar::MISSING_CLONE_UI`]，新建项目是
    /// [`crate::menu_bar::MISSING_NEW_PROJECT_SCAFFOLDING`]）—— 一句话对应一种能力，
    /// 与菜单栏那张表同一条规矩。
    ///
    /// `pub(crate)`：调用点是 `crate::project_menu` 那几行动作的 `on_click`
    /// （面板自己只有 `WeakEntity<ShellWorkspace>`，够不到 `&mut self`）。
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

    /// 「此窗口」：**重建整个外壳**（见模块头的"为什么不能逐个 reset"）。
    ///
    /// 顺序（每一步都有理由，不能调换）：
    ///
    /// 0. `search_index.schedule_invalidate`：把**旧根**的搜索索引缓存放掉。必须在
    ///    `replace_root` **之前** —— 之后 `self` 已经不是窗口持有的那个外壳了。
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
        // 不能捕获 `main.rs` 的局部变量（理由见该 Global 的文档）。取回那一步在
        // [`replace_root_with_workspace`] 里（欢迎页那条路也要它）。
        eprintln!(
            "S1_OPEN_PROJECT destination=this-window path={} state=rebuilding",
            target.display()
        );
        let old_root = self.root.display().to_string();
        // ① 换根之前先把**旧根**的索引缓存放掉（Core `workspace.searchIndex.invalidate`）。
        //
        // 这是 `invalidate` 在全仓**唯一**的挂点：gpui 今天没有"关闭项目"这条生命周期
        // （窗口关闭＝进程退出，内存里的索引跟着没），所以"离开一个工作区根"只剩换根这一条。
        // 这个外壳随后被 `replace_root` 丢掉，所以调用只发出去、结果不回写实体（见
        // `crate::search_index::SearchIndexWiring::schedule_invalidate`）。
        self.search_index.schedule_invalidate(cx);
        // ⚠️ **先保存会话，再换根**：`replace_root` 会丢掉旧 `Root` → 旧 `ShellWorkspace`，
        // 连带丢掉它的 `SessionTracker`（内存里那份会话）。防抖窗口里还没落盘的那次改动
        // 就在这一步永久消失 —— 所以这里是 `crate::session` 文档里那条"换根前必须先写"的落点。
        //
        // 必须在 `replace_root` **之前**（不能在闭包之后）：闭包一执行，`self` 指向的外壳
        // 已经不再被窗口持有，它的字段随时可能被 drop。
        self.flush_session(true, cx);
        // 通知中心**不需要**在这里清空：store 是 `ShellWorkspace` 的字段，换根会重建整个
        // 外壳，于是新外壳拿到的是一个空 store（水位线也是 0）。见 `lithe-gpui-notify`
        // 的模块文档「store 归谁持有」。
        //
        // 换根前到达的通知会跟着旧 store 一起没 —— 但那一刻旧 `EditorPane` 的异步任务本身
        // 也被 drop 了，原来那条 toast 同样发不出来，所以没有回归。
        // 闭包是 `FnOnce`（`replace_root` 的 build_view），新建出来的外壳句柄由
        // [`replace_root_with_workspace`] 带出来（重建之后它才是所有者）。
        let Some(workspace) = replace_root_with_workspace(&target, window, cx) else {
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
                None => eprintln!("S1_OPEN_PROJECT remember=skipped reason=no_settings_store"),
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
    /// ⚠️ 两处上游限制决定了拼法（侦察 `lithe-gpui/research/menu-open-prereqs.md` §1）：
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
                        // panic 记录：builder 跑在窗口根 `Root` 的渲染期）。
                        .checked(do_not_ask.get())
                        .on_change(move |&value, _window, cx| {
                            checkbox_cell.set(value);
                            // 写 cell 不够，还要触发下一帧：新帧重跑 dialog builder，
                            // 新值才会画出来。
                            let _ = checkbox_shell.update(cx, |_shell, cx| cx.notify());
                        }),
                )
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        // 按钮顺序照指南「取消 + 结果词」，主操作（此窗口）最右且 primary。
                        //
                        // ⚠️ 诊断（B2 补的真缺口）：这条路径原来**只有** `window.close_dialog`，
                        // 而同文件另外四条（`request_open_project` 的取消 / 空选择 / 通道关闭，
                        // 以及 `--open-project-probe`）都各有一行 `S1_OPEN_PROJECT state=…`。
                        // 于是"用户点了取消"在日志里跟"什么都没发生"完全一样 —— 验证脚本只能
                        // 靠"没有出现换根行"来间接推断。现在它自己有一行可 grep 的结论。
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
                                    // 「新窗口」现在是提示（规格 §5）：先让状态栏那句出现，
                                    // 再关对话框 —— 反过来的话提示会被浮层的遮罩挡在后面。
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
    ///
    /// `pub(crate)`：欢迎页的打开链路（[`crate::welcome::WorkspaceOpener`]）也要在换根
    /// **之后**记这一次打开 —— 那时它手里只有新外壳的 `Entity`，走的正是这个方法。
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

    /// 往通知中心记一条通知。
    ///
    /// 这是外壳侧**唯一**的「事情发生了」出口。原来这里是 `show_status_notice`（状态栏
    /// 一条 4 秒后自己消失的小字），2026-09-27 随通知中心一起废掉 —— 理由是它说完就没了，
    /// 用户事后无从查证，而通知中心是唯一的记录面。决策与被否方案见
    /// `.agents/notes/implemented/architecture/2026-09-27-gpui-notification-center.md`。
    ///
    /// 诊断行从 `S1_STATUS_NOTICE` 换成 `S1_NOTIFICATION`：交互类改动在本仓库只能靠维护者
    /// 手动验证（`lithe-gpui/docs/grill.md` D1a），这行可 grep 的证据是唯一的机器判据，所以
    /// **必须**跟着迁移，否则这批改动没法按现有方式验收。
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

    /// 「尚未接入：缺 X」这一族通知的落点（菜单栏 / 项目下拉的占位项，以及多窗口那句）。
    ///
    /// 三个调用点共用它，因为它们是**同一类**事情：用户点了一个还没接线的能力。`code` 直接
    /// 用 `Missing::text_key` / `lithe.gpui.newWindowNotWired` —— 那些键本来就在 catalog 里
    /// （`extract-locale.mjs` 的 `GPUI_ONLY_KEYS`），所以这一族迁移**不需要新增任何文案键**，
    /// 而条目仍然是 `(code, params)` 形状（`item` 让详情里能看到点的到底是哪一项）。
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

    /// 建立工作区配置失败时的**常驻红条**（可手动关闭）已删（2026-09-29）。
    ///
    /// 原来它刻意与通知中心分开（"要用户处理的失败走常驻条"），本轮产品决定收拢：
    /// 失败也走通知中心记录 + 左下角 toast（[`prepare_workspace_config`] 的失败分支），
    /// 消息面上不再有常驻条。Git 的红/绿条（`changes_view` 的写失败/写成功）**不在**
    /// 本轮范围内，维持原样。

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
//
// ⚠️ B2 又加了六条同类的键位入口（重新打开已关闭标签页 / 侧栏 / 终端 / 缩放三条），
// 它们**不在这里**：它们的登记与映射都跟着菜单项走，落在
// `crate::menu_bar::install_key_actions`（同一个 crate，`pub mod menu_bar` 已经公开，
// 于是 `main.rs` 不需要为了它们多一条 `lib.rs` 的 re-export）。
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
/// （侦察 `lithe-gpui/research/menu-open-prereqs.md` §2）。整条链上**只有这一行**能证明
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
        // gpui-kit 0.7 起对话框 / 抽屉 / 通知三层由窗口根 `Root` 的 plugin 层（WindowState）
        // 每帧统一渲染，外壳不再手动调 `Root::render_*_layer`（0.6 API 已删除）；
        // 这里只画工作区自己的内容。

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
        let branch_item =
            branch_trigger(&self.branch_panel, cx).unwrap_or_else(|| div().into_any_element());
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
        // 菜单项的点击由上一帧写进队列，在本帧分流执行（B3 起队列里有两类：
        // 已接线的 [`MenuRequest::Run`] 与占位项 [`MenuRequest::NotWired`]）。
        //
        // ⚠️ 两类**必须**在这里都处理：占位项只收起菜单、什么都不做的话，
        // 用户看到的就是"点了没反应"——那正是 Q2 与"占位项不许假装能用"要挡的。
        for request in crate::menu_bar::take_pending_runs(cx) {
            match request {
                MenuRequest::Run(action) => self.apply_menu_action(action, window, cx),
                MenuRequest::NotWired { label_key, missing } => {
                    self.report_menu_not_wired(label_key, missing, cx)
                }
            }
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
                    // 切项目后把活动标签滚进可视区（真源
                    // `WorkbenchView.swift:643-650` 的 `proxy.scrollTo(.., anchor: .center)`）。
                    // ⚠️ 放在**事件回调**里而不是 `render` 里：`render` 只登记 `track_scroll`。
                    this.project_tabs_scroll.scroll_to_item(index);
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
        // ⚠️ 左栏「更改」视图的弱引用**不再需要**：切到它的那条副作用已经收进
        // [`ShellWorkspace::select_top_activity`]（B2：活动栏与主菜单两处入口共用一段）。
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
                        //
                        // ⚠️ 复用 [`ShellWorkspace::select_top_activity`]（B2）：主菜单的
                        // 「文件资源管理器 / 源代码管理」走的是同一段 —— "切到「更改」就重读一次"
                        // 那条副作用只有这样才不会漏。
                        None => this.select_top_activity(index, cx),
                    }
                    // 活动栏点击同样改会话文档里的字段（顶部组选中 / 底部窗页签与可见性）。
                    this.schedule_session_save(cx);
                    // 切到「诊断」页时补一次刷新：面板自己不知道"用户什么时候打开了我"，
                    // 而诊断可能是在它不可见的那段时间里变多的（见 `editor::problems` 的表）。
                    this.refresh_problems_if_visible(cx);
                    // 切到「搜索」时把焦点交给查询框（真机 `SearchSidebarView.swift:146`
                    // 的 `onAppear { searchFocused = true }`）。**只有这一条事件回调能调**：
                    // 焦点要 `&mut Window`，而 `render` 里不许 mutate / 起任务。
                    //
                    // 判据与渲染**同源**：既要选中「搜索」这一项，也要面板真的可见
                    // （`left_pane_visible` 为假时面板没画出来，把焦点交给一个不可见的
                    // 输入框会让用户"按键打到了看不见的地方"）。
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
                    // 幂等：会话已存在就什么都不做（所以"再点一次隐藏、再点回来"不会重开 shell）。
                    // 首次创建时由它自己把焦点延到帧末交给输入行。
                    let _ = terminal_handle.update(cx, |pane, cx| pane.ensure_session(window, cx));
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
            move |index: usize, event: &ClickEvent, window: &mut Window, cx: &mut App| {
                // 本次点击的位置：取 `ClickEvent::mouse_position()`（`activity_bar` 把事件原样传下来），
                // 它是**这次松手**的位置（`gpui-pre-0.3.6/src/interactive.rs:336-347`）。
                // ⚠️ 不用 `Window::mouse_position()`：键盘激活按钮（Enter / Space）时那个值是
                // 上一次指针位置，会给"两行坐标相同 ⇒ 同一次派发"这条判据塞进假证据；
                // `ClickEvent` 在那种情况下给 `None`，于是这一行干脆不带坐标。
                let point = event.mouse_position().map(ProbePoint::from);
                // 扩展拼图项（下标 0）不是右工具窗视图：它开/关**中央扩展页**（2026-09-29
                // 起对齐真机 `openExtensionsBuffer` 的口径，见 `right_tool_window` 模块文档
                // 与 [`EXTENSIONS_RAIL_IX`]）。在进入右栏状态机**之前**拦截。
                if index == EXTENSIONS_RAIL_IX {
                    handle.update(cx, |this, cx| {
                        this.toggle_extensions_page(point, window, cx);
                    });
                    return;
                }
                // 越界下标什么也不做：右栏除拼图项外只有通知与 Maven，多出来的下标不该静默落到某一项上。
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
                    // 打开通知面板 = 用户看了（原来在状态栏回显的点击里，回显废掉后
                    // 「打开」就是唯一入口）。收起（visible=false）不算。
                    if view == RightToolWindowView::Notifications && visible {
                        this.note_notifications_opened(cx);
                    }
                    // 第一次真正显示这个视图时才去取它的数据：这里只**登记**，
                    // 同步读盘被排到派发栈之外（判据与理由都在
                    // [`ShellWorkspace::schedule_right_view_scan`] 里，菜单项与 `--right-view` 同源）。
                    this.schedule_right_view_scan(view, visible, cx);
                    diagnose_right_panel(view, visible, point);
                    // 右栏当前视图 / 可见性也在会话文档里。
                    this.schedule_session_save(cx);
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
                    this.schedule_session_save(cx);
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
        // 右活动栏这一份**每帧重算**：铃铛的角点颜色跟着通知中心的未读状态变，而未读
        // 状态在「一条通知进来」和「用户打开中心」两个时刻都会改。构造期存的那一份永远
        // 是「无未读」，所以不能直接用字段。
        let right_activity_items = right_activity_items(&self.notifications, cx);
        let right_rail = activity_bar(
            ActivitySide::Right,
            &right_activity_items,
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
        //
        // 三个视图（项目树 / 更改 / 搜索）三选一：搜索面板是 U17 第一件接上的第三个，
        // 判据与活动栏高亮**同源**（`top_activity_view`）—— 两处不同源会出现
        // "图标亮着但内容不对"（真机 `main-sidebar.tsx:777-816` 的 `activePaneId` 单选）。
        let left_content: AnyElement = if self.top_activity_view == Some(CHANGES_ACTIVITY_IX) {
            self.changes.clone().into_any_element()
        } else if self.top_activity_view == Some(SEARCH_ACTIVITY_IX) {
            self.search.clone().into_any_element()
        } else {
            explorer.into_any_element()
        };
        // 中央列的内容：扩展页开着时是**扩展页实体**，否则是编辑器岛 —— 与左栏
        // 「项目树 ↔ 更改」同一套"两实体二选一"模式（2026-09-29 起，Zed 式扩展页）。
        // `extensions_page_open` 为真而实体缺失只可能出现在构造路径之外（理论上不可达），
        // 防御式回落到编辑器而不是 panic。
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
            on_close_right_activity,
            cx,
        );
        let right_tool_window_visible = self.right_visible;
        // 左栏**两个**状态（2026-09-27，IDEA 语义）：整块可见性（`Ctrl+B` / 菜单那条能收它）
        // 与面板可见性（点活动栏当前高亮项只收它）。活动栏画不画只看前者。
        let left_sidebar_visible = self.left_sidebar_visible;
        let left_pane_visible = self.left_pane_visible;
        // 三块面板的长度是**变量**（拖拽会改），闭包按值捕获，所以先按**这一帧**的尺寸算出来。
        //
        // 为什么在 render 里再钳一次：拖拽中与恢复时都已经钳过，但**窗口本身会变窄**（用户拖窗口
        // 边缘、换显示器）。render 是唯一"每帧都会跑"的地方，上界跟着当帧尺寸走，于是
        // "在一个宽屏里拖到 760 的右栏、窗口缩到 1000 之后仍然不会把中央列压没"是结构性的，
        // 不需要任何 resize 回调。这里只读 + 算局部变量，**不改状态、不 notify、不起任务**。
        //
        // ⚠️ 上界的输入是 `pane_span_range`：左栏与底部窗读**分栏容器**的实测尺寸
        // （[`ShellWorkspace::split_container`]，上一帧 prepaint 量的；还没量到时回落
        // `viewport`），右栏只能读窗口内尺寸（理由见 `right_tool_window_max_width`）。
        let viewport = window.viewport_size();
        // 分栏容器实测点的缓存句柄（理由与量法见 [`ShellWorkspace::split_container`] 与
        // [`measure_split_container`]）：`on_prepaint` 闭包要 `'static`，所以克隆 `Rc` 再捕获。
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
                // 诊断页：**真的问题列表**（实体归编辑区，见 `editor::problems` 的模块文档）。
                // 外壳只把它塞进来：数据（哪些文件在报什么错）与"点一行"的落地都在那边。
                BottomPaneKind::Diagnostics => {
                    self.editor.read(cx).problems_pane().into_any_element()
                }
                // 运行页：**真的运行面板**（配置列表 / 输出控制台 / ▶ 🐞 ■ ⌫）。
                // 进程与输出都在那个实体里，外壳只把它放进来。
                BottomPaneKind::Run => self.run.clone().into_any_element(),
                // 调试页（U19）：**真的调试面板**（断点 / 调用栈 / 变量 / 监视 / 求值 / 单步）。
                // ⚠️ 它自己**没有**活会话可显示，也**不假装有** —— 空态写着为什么
                // （H10 transport / H11 JDT 调试会话启动都没做），见 `crate::debug` 的模块文档。
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

        // ① 标题栏**单行 40**（真源 macOS：`LitheTheme.Metrics.toolbarHeight = 40`，
        // `macos/Sources/Lithe/Theme/LitheTheme.swift:478` → `WorkbenchView.swift:829`）。
        // 菜单栏 / 项目下拉 / 分支项 / 拖拽区 / 运行控件 / 账号槽位 / 窗口三键全在这一行里，
        // 顺序与理由见 `crate::title_bar` 的模块头。
        //
        // 行内除 `drag_region` 之外的可点元素都是它的**兄弟节点**（由 `title_bar` 挂在行容器
        // 与左段上）：祖先的 `Drag` 会赢下 Windows 的命中测试，把菜单 / 运行控件放进拖拽区
        // 就会变成"点菜单只拖窗口"（见 `crate::title_bar` 的 `title_bar` 文档）。
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
            // （gpui 的 action 冒泡沿"焦点节点 → 祖先"走）。
            // 唯一够不到的情形是"窗口里一个焦点元素都没有"（启动后没点过任何地方），
            // 那时也没有可保存的内容。
            .on_action(cx.listener(|this, _: &SaveBuffer, window, cx| {
                let _ = this
                    .editor
                    .update(cx, |pane, cx| pane.save_active(window, cx));
            }))
            .child(title_bar(
                menu.unwrap_or_else(|| div().into_any_element()),
                project_menu.unwrap_or_else(|| div().into_any_element()),
                branch_item,
                // 运行面板实体：标题栏的 `运行配置 ▾` 显示**真实**的当前配置名，`▶` / `🐞`
                // 直接让它起一次（不再走菜单队列），见 `crate::title_bar` 的 `run_controls`。
                self.run.clone(),
                window,
                cx,
            ))
            // ② 项目标签条 38 —— **只有一个项目时整条隐藏**（真源
            // `macos/.../WorkbenchView.swift:253-255` 的 `count > 1` 判据）。
            // 度量与横向滚动见 `crate::project_tabs` 的模块头；滚动状态归外壳持有。
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
            // ②.5 **建立工作区配置失败**的常驻红条已删（2026-09-29）：配置失败改走通知
            // 中心 + 左下角 toast（见 [`prepare_workspace_config`] 的失败分支）。
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
                    // 左栏：**活动栏与面板是两个状态**（IDEA 语义，2026-09-27）。
                    // - 活动栏：只要 `left_sidebar_visible` 为真就画（`Ctrl+B` / 菜单那条才整块收）；
                    // - 面板：还要 `left_pane_visible`（点当前高亮的活动栏项只收这一个）。
                    // 可见性判据与 [`ShellWorkspace::is_activity_active`] 同源，见字段文档。
                    .children(left_sidebar_visible.then_some(left_rail))
                    // ★ 侧栏 + 中央列是**同一个 `gap_0` 组**：维护者 2026-09-27 的口径是
                    // 「这个中间可以直接连接吗？现在这样还有空细」——"直接连接"这件事由
                    // **这个共同 owner** 表达（组内 gap = 0），不再靠 `-WORKBENCH_GAP_SPEC`
                    // 的负外边距去吃掉父级的 `gap_1()`（那正是《设计指南》
                    // `docs/gpui kit/0.7.0/docs/design-guides.md:188` 禁止的"raw pixel offset
                    // 修补"）。组外的 `gap_1()`（左活动栏 ↔ 侧栏、中央列 ↔ 右工具窗与右活动栏）
                    // 一概不动 —— 那几处 4px 是另外的观感，不该被这次改动连坐。
                    .child(
                        // ⚠️ **这一格就是"分栏容器"的实测点**（第 4 条遗留）：`.id()` +
                        // `.on_prepaint` 把它的当帧盒子写进 [`ShellWorkspace::split_container`]，
                        // 供**下一帧**的上界计算读（`pane_span_range`）。它是真源
                        // `WorkbenchView.swift:1999` 那个 `GeometryReader` 的对应物：装
                        // [左侧面板, 中央列]，宽 = 窗口内宽减两条活动栏 / 右工具窗 / 各处间距，
                        // 高 = 窗口内高减标题栏 / 标签条 / 状态栏。
                        // 只写 `Rc<Cell>`、不改实体状态、不 notify（仅首次量到补一帧），
                        // 见 [`measure_split_container`]。
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
                            // 组内**没有** gap：相邻区域的边界线由**唯一所有者**画
                            // （`design-guides.md:196`「Hairline 由边界所有者绘制，相邻区域
                            // 不能各画一次同一条分隔线」）——那条线归中央列
                            // （`editor_island` 的左边框），所以左侧面板走
                            // [`SidePaneEdge::WithoutTrailing`] **不画右边框**。
                            // 改前两侧各画一条 1px、中间还有 4px 缝。
                            .children((left_sidebar_visible && left_pane_visible).then(|| {
                                // 面板内容 = 视图 Entity（填满）+ 右边界那条分隔条
                                // （[`drag_handle`]，绝对定位不占布局）+ 拖拽跟踪器
                                // （[`drag_tracker`]，同样不占布局）。
                                //
                                // 包装是 `relative().size_full()`：内容 Entity
                                // （`Explorer` / `ChangesView`）的根是 `size_full` 的 `v_flex`，
                                // 父级给满尺寸它就铺开；分隔条与跟踪器都走绝对定位所以
                                // **不挤压**内容。
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
                                // 中央列 = 中央页（扩展页，开着时）/ 编辑器岛 + 底部工具窗
                                // （默认口径：嵌在中央列内）。
                                v_flex()
                                    .flex_1()
                                    .h_full()
                                    // `min_*_0()` 是"窗口窄了要能收敛"那条：中央列可以收到 0，
                                    // 于是左右面板被窗口挤到时，溢出的是可收缩的这一列而不是把
                                    // 状态栏 / 右活动栏顶出窗口。
                                    .min_w_0()
                                    .min_h_0()
                                    .child(center)
                                    .children(bottom),
                            ),
                    )
                    // 右工具窗 —— **可收起**（真机整块 `ResizablePane` 的 `hidden` 由
                    // `isRightToolWindowVisible`（= 通知或 Maven 可见）决定，收起时这一栏完全不占位），
                    // **可拖宽**：手柄贴它的**左**边界（`HandleEdge::Leading`），往左拖变宽 ——
                    // 见 [`ResizablePane::RightToolWindow`] 与 [`drag_handle`]。
                    .children(right_tool_window_visible.then(|| {
                        // 与左侧面板同一套包装：内容 + 手柄（绝对定位不占布局）+ 跟踪器。
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
                            // 右工具窗与右活动栏之间那 4px 是另一处观感：它自己四条边都画。
                            SidePaneEdge::All,
                            cx,
                        )
                        .into_any_element()
                    }))
                    .child(right_rail),
            )
            // ④ 状态栏 24 —— 设置里的「显示状态栏」关掉时整条不渲染（真机是根元素上的
            // `data-status-bar` + CSS，见 `07-settings-ui.md` §4.5 的 `lib/ui-preferences.ts:5-11`）。
            // 尾随组每次重绘都按当前光标/活动 buffer 重算，所以用局部变量接一下返回值。
            .children(show_status_bar.then(|| {
                // `footer_left` 要 `&mut Context<Self>`：通知中心那格的点击处理器得拿到
                // `&mut ShellWorkspace` 才能切右工具窗（见它的文档）。
                let left = self.footer_left(cx);
                let right = self.footer_right(cx);
                status_bar(&left, &right, window, cx)
            }));

        root
    }
}

/// 侧栏外壳**画哪几条竖边**。
///
/// 存在的唯一理由（2026-09-30）：侧栏与中央列是"直接连接"的邻居（见 `render` 里那个
/// `gap_0` 内层组），而《设计指南》`docs/gpui kit/0.7.0/docs/design-guides.md:196` 要求
/// 「Hairline 由边界所有者绘制，相邻区域不能各画一次同一条分隔线」。那条边界的所有者是
/// **中央列**（[`editor_island`] 的 `border_1()` 左边框），所以左侧面板不再画自己的右边框。
///
/// 改前是"两侧各画一条 1px + 中间 4px 缝"，再用 `-WORKBENCH_GAP_SPEC` 的负外边距把缝吃掉
/// —— 那既重复画了同一条线，又是 raw pixel offset 修补（`:188` 明令禁止）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum SidePaneEdge {
    /// 四条边都画。用于右侧工具窗：它的右邻是 4px 间距 + 右活动栏，那条边界归它自己。
    All,
    /// 不画右边框。用于左侧面板：右邻中央列，那条线归 [`editor_island`]。
    WithoutTrailing,
}

/// 左右两侧的面板外壳：固定宽、不参与收缩、`bg-background`、直角边框。
///
/// 本侧**不画圆角**（四个角是直角），理由见文件开头 `ISLAND_RADIUS_SPEC` 的墓碑注释。
/// 左侧栏**可以拖拽改宽**：右边界那条分隔条是 gpui-kit Dock 同款的 [`drag_handle`]，
/// 跟踪在 [`drag_tracker`]；右侧工具窗目前定宽不可拖（真源的 `ResizablePane` 热区还没
/// 对应过来）。
///
/// `outer` 由调用方给：宽度是布局度量，调用点用 gpui 的 rem 档位 helper 或档位外的
/// `rems(P / 16.)` 各自表达，这里只负责外壳剩下的部分。
///
/// `edge` 决定四条边画几条 —— 见 [`SidePaneEdge`]（相邻边界的 hairline 只能有一个所有者）。
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
        // `relative()` 是拖拽热区（[`drag_handle`]）绝对定位的定位上下文 ——
        // 热区贴在面板右边界上，而**不**参与布局（见那里"不套 flex 包装"的理由）。
        .relative()
        .bg(cx.theme().background);
    let outer = match edge {
        // 四条边一起给：`border_1()` + `border_color(..)`。
        SidePaneEdge::All => outer.border_1().border_color(border_color),
        // 三条边（不含右）：逐边给，**不用** `border_1()` 再清右 —— 那样会写出
        // `border_r(0)` 这类"先画再抹"的修补，与本次修掉的负外边距是同一类坏味道。
        SidePaneEdge::WithoutTrailing => outer
            .border_t_1()
            .border_b_1()
            .border_l_1()
            .border_color(border_color),
    };
    outer.child(content)
}

/// 编辑区左边界那个「收起侧栏」按钮 —— **2026-09-27 按维护者口径删除**。
///
/// 维护者原话：「侧栏折叠按钮（那个 ‹ 与编辑区那对 ← → 重叠了，也去掉」。
/// 它当时绝对定位贴在编辑器岛左上角（`left_1() + top_1()`），与标签栏左侧那对导航箭头
/// 落在同一个角上，视觉上重叠；两个都删掉之后那个角是干净的。
///
/// **不删能力**：收起侧栏本身还留着三个入口 —— 左活动栏点当前高亮项
/// （IDEA 语义，见 [`ShellWorkspace::select_top_activity`]）、菜单
/// 「视图 → 切换活动侧栏」、键位 `Ctrl+B`（后两者走
/// [`MenuAction::ToggleActivitySidebar`] → [`ShellWorkspace::toggle_left_sidebar`]）。
///
/// 记一笔当时的实现细节，免得将来重新发明：绝对定位盒**必须显式给宽高且外层要
/// `items_center()`**，否则内层 `flex()` 会被默认的 `align-items: stretch` 拉成竖长条
/// （维护者截图复现过两次）。

/// 与 [`crate::title_bar`] 的运行控件同一取法（`title_bar.rs:516-549`）：`Button` 的 ghost 变体
/// 自带一组 `icon-xs` 内边距与圆角，要覆盖成这里的圆角反而得绕 `Button::rounded(Pixels)`；
/// 自绘 `div` 能直接用 rem helper，也与标题栏那两个控件保持同一种观感与代码形状。
/// 圆角取 `cx.theme().radius`（不写裸值）。
///
/// ## 点击行为与那两行"防回归"
///
/// 编辑区左边界那个「收起侧栏」按钮 —— **2026-09-27 按维护者口径删除**（墓碑注释）。
///
/// 维护者原话：「侧栏折叠按钮（那个 ‹ 与编辑区那对 ← → 重叠了，也去掉」。
/// 它当时绝对定位贴在编辑器岛左上角（`left_1() + top_1()`），与标签栏左侧那对导航箭头
/// 落在同一个角上、视觉重叠；两个都删掉之后那个角是干净的。
///
/// **能力没删**：收起侧栏还留着三个入口 —— 左活动栏点当前高亮项（IDEA 语义，见
/// [`ShellWorkspace::select_top_activity`]）、菜单「视图 → 切换活动侧栏」、键位 `Ctrl+B`
/// （后两者走 [`MenuAction::ToggleActivitySidebar`] / [`ShellWorkspace::toggle_left_sidebar`]）。
///
/// 实现细节记一笔，免得重新发明时再踩：绝对定位盒**必须显式给宽高、且外层要
/// `items_center()`**，否则内层 `flex()` 会被默认的 `align-items: stretch` 拉成竖长条
/// （维护者截图复现过两次）。

/// 一块**可以拖拽改尺寸**的面板。
///
/// 三个变体就是外壳里三块有独立尺寸的面板。用枚举而不是三个 bool：拖拽会话、算术、手柄的
/// 轴与贴边方式都按它分支，一处 `match` 就能看清三块面板的差别在哪。
#[derive(Clone, Copy, PartialEq, Eq)]
enum ResizablePane {
    /// 左侧栏（项目树 / 更改 / 搜索），拖**右**边界，水平轴。
    LeftSidebar,
    /// 右侧工具窗（Maven / 通知 / 扩展），拖**左**边界，水平轴。
    RightToolWindow,
    /// 底部工具窗（终端 / Git / 运行 / 诊断），拖**上**边界，**垂直**轴。
    BottomPane,
}

impl ResizablePane {
    /// 手柄的轴。`Axis::Horizontal` = 竖直的那条分隔条（左右拖），
    /// `Axis::Vertical` = 水平的那条分隔条（上下拖）—— 上游 `resize_handle` 的命名口径。
    ///
    /// 垂直轴**上游支持**（`gpui-base-0.7.0/src/resizable/resize_handle.rs:277-290`：
    /// `(Some(HandleEdge::Leading), Axis::Vertical)` 给出 `top_0() + pb(HANDLE_PADDING)` 与
    /// `cursor_row_resize()`），所以底部工具窗不需要自己画热区。
    fn axis(self) -> Axis {
        match self {
            Self::LeftSidebar | Self::RightToolWindow => Axis::Horizontal,
            Self::BottomPane => Axis::Vertical,
        }
    }

    /// 手柄贴在面板的哪条边。左栏的可变边界是**后缘**（右边界），右栏与底部窗是**前缘**
    /// （左边界 / 上边界）。
    ///
    /// 贴边不只是观感：面板壳有裁剪与 1px 边框，跨边界手柄的外半段会被裁掉、命中不到，
    /// 只有整体收进面板内才是完整可抓的（上游 [`HandleEdge`] 文档）。
    fn handle_edge(self) -> HandleEdge {
        match self {
            Self::LeftSidebar => HandleEdge::Trailing,
            Self::RightToolWindow | Self::BottomPane => HandleEdge::Leading,
        }
    }

    /// 手柄的 `ElementId`，**按面板（业务身份）取**，不按渲染顺序或下标。
    ///
    /// ⚠️ 三个**必须互不相同**：gpui-kit 官方 `element_id` 文档专门点了 Dock 那个坑 ——
    /// 三个 `resize_handle` 落在同一个带 key 的祖先下、都叫 `"resize-handle"` 时会拿到同一个
    /// `GlobalElementId`，"按住一个 handle 可能触发另一个的拖动"；上游自己的实现因此按方位取
    /// `resize-handle-left/-right/-bottom`。这里沿用同一口径（`lithe-gpui/UI-MAP.md` §1.4 那条
    /// 官方条文也写着"动手改分栏时应照此检查"）。
    fn handle_id(self) -> &'static str {
        match self {
            Self::LeftSidebar => "lithe-left-pane-resize",
            Self::RightToolWindow => "lithe-right-pane-resize",
            Self::BottomPane => "lithe-bottom-pane-resize",
        }
    }

    /// 诊断行里用的稳定名字（`S1_PANE_RESIZE pane=…`）。
    fn id(self) -> &'static str {
        match self {
            Self::LeftSidebar => "left",
            Self::RightToolWindow => "right",
            Self::BottomPane => "bottom",
        }
    }

    /// 拖拽要**加**还是**减**沿轴的位移。
    ///
    /// 手柄在**后缘**（左栏右边界）时，指针沿轴正方向移动 = 面板变长，取 `+1`；手柄在**前缘**
    /// （右栏左边界 / 底部窗上边界）时反过来 —— 往左 / 往上拖才是变长，取 `-1`。写成常数而不是
    /// "两次取反"，三块面板因此共用下面一段算术。
    fn drag_direction(self) -> f32 {
        match self {
            Self::LeftSidebar => 1.,
            Self::RightToolWindow | Self::BottomPane => -1.,
        }
    }

    /// 指针**沿轴**的窗口坐标（水平轴取 x，垂直轴取 y）。
    fn pointer_along(self, position: Point<Pixels>) -> f32 {
        f32::from(match self.axis() {
            Axis::Horizontal => position.x,
            Axis::Vertical => position.y,
        })
    }
}

/// 一次拖拽会话：哪块面板 + 锚点。
struct PaneResizeDrag {
    pane: ResizablePane,
    /// 会话**首个移动事件**记下的 `(指针沿轴坐标, 起始长度)`；`None` = 还没记。
    ///
    /// 为什么不按下那刻记：`.on_drag` 构造器只拿到相对手柄原点的 offset，换算窗口坐标要多测
    /// 一次手柄盒，不值得；首帧锚点与按下锚点只差 gpui 的 `DRAG_THRESHOLD` 几个像素。
    /// 为什么存**起点**而不是每帧累加 delta：累加会随事件频率漂移（一次快拖与一次慢拖结果
    /// 不同），而 `起点 + 位移` 与事件频率无关。
    origin: Option<(f32, f32)>,
}

/// 把一个长度钳到 `[min, max]`，并做两条"坏输入不许变成坏布局"的硬化。
///
/// 1. `max < min` 时**先收敛到下界**：`f32::clamp` 在 `min > max` 时**会 panic**，而
///    "窗口比两端下界之和还窄"是 `available − reserve` 能算出来的真实结果（把一个在宽屏上
///    存下来的尺寸恢复到窄窗口上也会走到这里）；
/// 2. `NaN` 落回下界：尺寸字段是跨端文档里的值，`clamp(NaN, ..)` 返回 `NaN`，那一帧的布局会
///    整片塌掉。真源 `WorkbenchLayoutStore.load` 同样按 `isFinite` 判一次（本侧只挡 `NaN`：
///    `±inf` 交给 `clamp` 自己处理就是"顶到上/下界"，比落回下界更符合直觉）。
fn clamp_pane_span(value: f32, min: f32, max: f32) -> f32 {
    if value.is_nan() {
        return min;
    }
    value.clamp(min, max.max(min))
}

/// 恢复会话时这块面板该取哪个长度：**有持久化值就用它**（钳到区间内），
/// **没有就保留外壳默认值**（`current`）。
///
/// 抽成纯函数是为了把"默认值不得覆盖用户存过的值"这条语义变成机器判据（见单测）：
/// 三个尺寸字段在会话文档里都是 `Option<f32>`（`crates/shared/src/workspace_config/session.rs`
/// 的 `left_pane_width` / `right_pane_width` / `bottom_pane_height`，缺键 = `None`），
/// 文档层刻意不判上下界，`None` 与"越界值"的处置都归这里。
///
/// ⚠️ 这条与右栏默认值 400 → 360 的改动直接相关：360 是**初值**，用户一旦拖过、会话里存了
/// `rightPaneWidth`，下次启动读到的就是存过的值（本函数 `Some` 分支），不会被 360 顶掉。
fn pane_span_on_restore(stored: Option<f32>, current: f32, min: f32, max: f32) -> f32 {
    match stored {
        Some(value) => clamp_pane_span(value, min, max),
        None => current,
    }
}

/// 把一次拖动换算成新的面板长度：`起点长 + 方向 × 位移`，再钳到 `[min, max]`。
///
/// 抽成纯函数是为了**能单测**（本 crate 拿不到 `TestAppContext`，起不了窗口，所以拖动链路里
/// 唯一可测的就是这段算术）。三块面板共用这一段，差别只有 `direction`（见
/// [`ResizablePane::drag_direction`]）。
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

/// 分栏容器实测点的 `ElementId`（业务身份：这个窗口里只有一处分栏容器）。
const SPLIT_CONTAINER_ID: &str = "lithe-workspace-split-container";

/// 把当帧量到的分栏容器盒子写进缓存（`on_prepaint` 的回调体）。
///
/// 只做两件事：
///
/// 1. `Rc<Cell>` 覆盖成当帧的 `size`。**只改这个缓存、不改任何实体状态、不 notify** ——
///    与 `project_menu.rs:1312` / `menu_bar.rs:2617` 量触发器盒子同一套写法（那两处也是
///    "prepaint 写 `Rc<Cell>`，下一帧 render 读"）。
/// 2. **第一次**量到时补一帧（`window.request_animation_frame()`）：上界读的是**上一帧**
///    的值，不补这一帧的话首帧没有可用容器尺寸、只能按兜底输入（窗口内尺寸）算一次。
///
/// 值真的变了才打一行 `S1_SPLIT_CONTAINER` 到 **stderr**（真机核对"容器宽到底是多少"
/// 的唯一证据，且不逐帧刷屏；`eprintln!` 是本仓的诊断通路）。
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

/// 左栏在给定**分栏容器宽**下的上界：`max(30, min(520, 容器宽 − 400))`。
///
/// ## 出处（真源 macOS，逐句对得上）
///
/// - `macos/Sources/Lithe/Views/Workbench/WorkbenchView.swift:2000-2005`：
///   `let availableTopWidth = max(0, geometry.size.width - (paneInset * 2) - paneSpacing)`
///   —— 本侧传进来的 `available_width` 就是这一项（`paneInset = 0`，见 `WorkbenchView.swift:23`；
///   容器实测宽见 [`ShellWorkspace::split_container`]）。
/// - `:2006`：`let minimumSidebarWidth = CGFloat(WorkbenchLayout.minimumSidebarWidth)` = 30。
/// - `:2007`：`let minimumEditorWidth: CGFloat = 400`
///   （[`SIDEBAR_MIN_EDITOR_WIDTH_SPEC`]）。
/// - `:2008-2011`：
///   `let maximumSidebarWidth = max(minimumSidebarWidth, min(520, availableTopWidth - minimumEditorWidth))`
///   —— 本函数**逐字**照抄这个形状（`.max(SIDEBAR_MIN_WIDTH_SPEC)` 就是那个外层的 `max`，
///   所以"容器比 400 还窄"时返回下界 30 而不是负数）。
///
/// ⚠️ **容器尺寸还不可知时返回静网上界 520**（`available_width` 非有限或 `<= 0`：
/// 构造期、恢复会话、以及第一帧还没 prepaint 过）。那几帧宁可"不收敛"，也不能按
/// "容器只有 0 宽"把用户存下来的宽度钳到下界 30 —— 后者会让存过的尺寸在启动瞬间被吃掉。
/// 下一帧 `render` 会按真实容器宽收敛。
///
/// ⚠️ **与真源的已知差额**：真源的 `geometry` 是分栏容器的**精确**宽度；本侧量的是
/// "装 [左侧面板, 中央列] 的那个 `gap_0` 组"的 prepaint 盒（`Bounds<Pixels>`），
/// 它不含自己的 padding / border，所以等于该组的布局宽 —— 但本侧各处 `gap_1()` 的 4px
/// 在真源里是 0（真源只有一条 `SplitHandleView.thickness`），因此仍差几个像素。
/// 差额方向是"本侧容器略小"，即上界比真源**略紧**几像素，不是更松。
fn left_sidebar_max_width(available_width: f32) -> f32 {
    if !available_width.is_finite() || available_width <= 0. {
        return SIDEBAR_MAX_WIDTH_SPEC;
    }
    (available_width - SIDEBAR_MIN_EDITOR_WIDTH_SPEC)
        .min(SIDEBAR_MAX_WIDTH_SPEC)
        .max(SIDEBAR_MIN_WIDTH_SPEC)
}

/// 右侧工具窗在给定**可用宽度**下的上界：`min(760, 可用宽 − 640)`，且不低过下界。
///
/// 形状照真源 `WorkbenchRightToolSplitView.swift:7-16`（`maximumWidth(in:)` =
/// `min(maximumMavenPaneWidth, availableWidth − minimumWorkspaceWidth − 热区厚)`，
/// `minimumWidth(in:)` = `min(300, maximumWidth)`）—— 这就是"窗口窄了要收敛"那条规则。
///
/// ⚠️ **`available` 仍是近似（2026-10-03 复核后保留）**：本侧量的是**窗口内宽度**
/// （`window.viewport_size()`），真源量的是 `WorkbenchRightToolSplitView` 的 `geometry`
/// （= 窗口内宽 − 左活动栏 − 右活动栏）。**换不成容器**：那个容器在本侧的树里没有对应元素
/// —— 真源把两条活动栏放在工作区容器**外面**做兄弟（`WorkbenchView.swift:257-264` 的
/// `HStack { activityBar; workspaceArea; pluginActivityBar }`），本侧两条活动栏是**同一行**
/// 的兄弟（`render` 里那个 `h_flex`）。而 `ShellWorkspace::split_container` 那一格
/// **已经**扣掉了右工具窗自己，拿它算会重复扣。
/// 差额 = 两条活动栏 + 本侧 4px 间距 = **92**（左 44 + 右 44 + 4；两条 rail 的盒宽见
/// `activity_bar.rs` 的 `rail_width_spec`，rail 宽本身 = `shared::design::ACTIVITY_RAIL_WIDTH_SPEC`
/// 的 40 —— B14 落地前这里记的是 38+4 = 42/条、差额 88），
/// 方向是这里比真源**略松**；不会溢出：中央列至少还剩 640 − 活动栏宽。
/// 要精确就得把这个差额从钳制里扣掉（会动可见的右栏上界）—— **本轮不接**：B14 只裁决了 rail 宽，
/// 没有裁决"右栏上界要不要扣掉两条活动栏"。
///
/// ⚠️ 窗口尺寸还不可知时（`viewport_size()` 在极端情况下会给 0，例如构造期还没量过一帧）
/// 返回**静网上界**：那几帧宁可"不收敛"，也不能把恢复出来的宽度按"窗口只有 0 宽"钳到下界 ——
/// 后者会让用户存下来的尺寸在启动瞬间被吃掉。下一帧 `render` 会按真实尺寸收敛。
fn right_tool_window_max_width(available_width: f32) -> f32 {
    if !available_width.is_finite() || available_width <= 0. {
        return RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC;
    }
    (available_width - RIGHT_TOOL_WINDOW_MIN_WORKSPACE_WIDTH_SPEC)
        .min(RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC)
        .max(RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC)
}

/// 底部工具窗在给定**可用高度**下的上界：`可用高 − 220`，且不低过下界。
///
/// 形状照真源 `WorkbenchView.swift:2018-2025`（`maximumTopPaneHeight` =
/// `height − 间距 − minimumGitPaneHeight`，反过来读就是"底部窗最多占到上面只剩 220"）。
/// `available` 现在来自 [`ShellWorkspace::split_container`] 的 `height`（真源同一个
/// `geometry.size.height`）—— 改前取窗口内高，那会把标题栏 / 标签条 / 状态栏一起算进去，
/// 比真源松；容器还没量到时才回落窗口内高。
/// 尺寸还不可知时返回 `INFINITY`（= 这一帧不设上界，同样为了不误吃恢复出来的值）。
fn bottom_pane_max_height(available_height: f32) -> f32 {
    if !available_height.is_finite() || available_height <= 0. {
        return f32::INFINITY;
    }
    (available_height - BOTTOM_PANE_MIN_CENTER_HEIGHT_SPEC).max(BOTTOM_PANE_MIN_HEIGHT_SPEC)
}

/// 三块可拖面板的分隔条 —— gpui-kit **Dock 同款**（`docs/gpui kit/0.7.0/component/dock.md`；
/// 2026-09-30 按 dock 文档把旧的自拼热区换成上游组件；2026-10-01 从"只有左栏"扩到三块面板）。
///
/// ## 组件与观感
///
/// 直接用 `gpui_kit::base::resize_handle` + 组件库的 `resize_handle_appearance()`，
/// 与 Dock 组件每条边缘/分割手柄是同一份实现：
///
/// - 静止时常显一条 1px `border` 色细线，落在面板可变边界那一像素上
///   （`inside(HandleEdge::Trailing)` 用于左栏；右栏与底部窗的可变边界在前缘，用
///   `HandleEdge::Leading`）—— Dock 的左缘手柄同款贴法：面板壳有裁剪与 1px 边框，
///   跨边界手柄的外半段会被裁掉、命中不到，只有整体收进面板内才是完整可抓的；
/// - 悬停/按下/拖动时细线上浮出一粒**圆角胶囊**：muted 前景色，长度 20/28/44、
///   不透明度 0.35/0.6/0.9 随投入程度渐进，带过渡动画 —— 取代旧版"整条 4px 染主色"的
///   悬停态；
/// - 命中带宽 1px 线 + 4px 内侧 padding（upstream `HANDLE_SIZE`/`HANDLE_PADDING`），
///   光标 col-resize / row-resize（由 `Axis` 决定）。手柄 `.occlude()`，热区那几像素上的
///   点击让给它 —— 与 Dock 自身"手柄住在 dock 里"的取舍一致。
///
/// ## 与旧自拼热区的差别
///
/// 旧版（底部工具窗那条，2026-09-30 之前）是手写的绝对定位 `div`：平时隐形、`on_mouse_down`
/// 记起点、**没有任何会话开关**，于是它实际上什么都拖不动。换组件后**这里不记起点**：
/// `.on_drag` 构造器在一次手势里只触发一次（gpui 以 `active_drag` 记住会话归属），只把
/// [`ShellWorkspace::pane_resize_drag`] 开起来；起点与跟踪在 [`drag_tracker`]，
/// 与 Dock 的 `DockResizeTracker` 同一分层。
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

/// [`drag_handle`] 拖拽会话的载荷：什么都不画 —— 手柄本身就是视觉反馈。与 Dock 的
/// `ResizePanel` 同款：存在意义是让 gpui 的 `active_drag` 有类型可归属，
/// [`drag_tracker`] 靠 [`ShellWorkspace::pane_resize_drag`] 里"哪块面板"决定这次拖拽算谁的。
///
/// 不带字段是**有意**的：一次会话的面板身份在 [`PaneResizeDrag::pane`]，两边各存一份只会漂移
/// （载荷这份没人读）。
struct PaneResizeHandle;

impl Render for PaneResizeHandle {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// 一块面板的分隔条**跟踪与结束**：一块不画任何东西的 canvas，借 paint 闭包注册窗口级
/// 鼠标监听（拖动期间长度每次变化都重绘，监听逐帧续上）。
///
/// ## 为什么是窗口级监听，而不是手柄上的元素级 `on_mouse_move`
///
/// 元素级监听内部要求 `hitbox.is_hovered(window)` —— 光标一离开手柄就断线，维护者实测的
/// 症状是「鼠标过快就不能拖动」。Dock 的 `DockResizeTracker` 注释给出同一理由：
/// resize 由落在窗口**任何位置**的指针移动驱动，没法表达成手柄自己的监听。
///
/// ## 为什么必须在 paint 期注册
///
/// `Window::on_mouse_event` 有 `debug_assert_paint`，文档明说 *"should only be called
/// as part of the paint phase"*；在事件回调里调它会 `this method can only be called
/// during paint` 直接崩（旧版踩过）。canvas 的第二个闭包就是 paint 闭包、签名带
/// `&mut Window`，是合法且每帧重跑的注册点 —— 与 gpui-component 自己的
/// `carousel/scroll_mask.rs` 同一套做法。paint 期不调 `cx.notify()`。
///
/// ## 会话归属与清理
///
/// 三块面板各有一块 tracker，但只认 [`ShellWorkspace::pane_resize_drag`] 里 `pane` 与**自己**
/// 相同的那条会话：会话的首个移动记锚点，之后每次移动用 [`clamped_drag_span`] 改长度；
/// mouse up 清掉会话 —— **这是 2026-09-30 顺手修掉的旧 bug**：旧版只在 mouse down 记起点、
/// 从不清理，拖过一次之后任何按住左键的窗口内拖动（编辑器选文本等）都会拿旧起点继续改宽。
/// 另有一道自愈：会话开着却侦到左键已不在按下态（up 事件被吞，例如拖拽中途面板被收起、
/// 监听随帧丢失），同样当场收尾，不留悬挂会话。
///
/// ⚠️ **每帧最多重绘、绝不落盘**：拖动中只改内存值 + `notify`（且值没变就连 `notify` 都省）；
/// 写盘只在手势结束那一次，见 [`ShellWorkspace::end_pane_resize`]。
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
                // 先读窗口尺寸再 `update`：两个借用不能同时活着（`cx` 是 `&mut App`）。
                let viewport = window.viewport_size();
                let along = pane.pointer_along(event.position);
                let released = event.pressed_button != Some(gpui_kit::MouseButton::Left);
                let _ = drag.update(cx, |shell, cx| {
                    if shell.pane_resize_drag.as_ref().map(|session| session.pane) != Some(pane) {
                        return;
                    }
                    if released {
                        // 左键已经不在了（up 事件被吞）：当场收尾 + 落盘，不留悬挂会话。
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

/// 中央编辑器岛：直角边框（理由见文件开头 `ISLAND_RADIUS_SPEC` 的墓碑注释）。
///
/// ⚠️ **它的左边框是"侧栏 ↔ 中央列"那条 hairline 的唯一所有者**（2026-09-30）：侧栏走
/// [`SidePaneEdge::WithoutTrailing`] 不画右边框，两个区域之间也**没有** gap（render 里那个
/// `gap_0` 内层组）。所以这里必须保持 `border_1()`（四条边）——把左边框去掉会让那条边界
/// 一条线都不剩。
///
/// 只收内容：原来还收一个「编辑区左边界收起侧栏」按钮（`collapse_sidebar_button:
/// Option<AnyElement>`），2026-09-27 按维护者口径删除，见上方那段墓碑注释。
///
/// 仍然保留 `.relative() + .overflow_hidden()`：岛是绝对定位子元素的定位上下文。
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

/// 底部工具窗外框：高度由调用方给（默认 [`BOTTOM_PANE_HEIGHT_SPEC`] = 260，可拖改），
/// **上边界**是一条真正的垂直轴分隔条（[`drag_handle`] + [`HandleEdge::Leading`]）。
///
/// ⚠️ **2026-10-03：默认高按真源改成 260**（= 真源最小高，`WorkbenchView.swift:2019`），
/// 改前那个 240 从来没画出来过（首帧就被下界 260 钳掉）—— 理由与证据写在
/// [`BOTTOM_PANE_HEIGHT_SPEC`] 上。
///
/// ⚠️ **2026-10-01 接上拖拽**：改前这里是一条"死"热区 —— 一个 `div().w_full().h_1()`，
/// 画了一条 4px 的带子但**没有会话开关、没有跟踪器**，所以拖不动（`lithe-gpui/HANDOFF.md` §4
/// 登记过）。现在换成上游 `resize_handle`（`Axis::Vertical` + `HandleEdge::Leading`：
/// 热区整体收在面板**顶边内侧**，光标 `row-resize`，静止是一条 1px 细线、悬停浮出胶囊），
/// 会话与跟踪复用与左栏完全相同的那一套（[`drag_handle`] / [`drag_tracker`]）。
///
/// 内容由调用方给（终端 / Git 提交记录 / 占位 / 运行 / 诊断），与真机"底部窗只有一个、
/// 内容由 `bottomPaneActiveTab` 切换"的结构一致。
fn bottom_pane(
    content: AnyElement,
    height: f32,
    handle: gpui_kit::WeakEntity<ShellWorkspace>,
    pane: ResizablePane,
    cx: &App,
) -> impl IntoElement {
    // 面板本体：`relative()` 是手柄与跟踪器的定位上下文（两者都绝对定位、**不占布局**）。
    // 高度走 `rems(P / 16.)`：260（以及拖出来的任意值）不在 4px 档位上，
    // 见 [`BOTTOM_PANE_HEIGHT_SPEC`]。
    let body = div()
        .relative()
        .w_full()
        .h(rems(height / 16.))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .child(content)
        // `HandleEdge::Leading` + `Axis::Vertical` ⇒ 热区（1px 细线 + 4px 内侧 padding）
        // 整体收在**顶边内侧**，细线正落在与中央列相接的那一像素上。
        .child(drag_handle(handle.clone(), pane))
        .child(drag_tracker(handle, pane));

    v_flex()
        .w_full()
        // `flex_shrink_0`：长度是用户给定的（拖拽改），不接受 flex 再压一次 —— 收敛由
        // [`bottom_pane_max_height`] 与中央列的 `min_h_0()` 一起保证，不靠隐式收缩。
        .flex_shrink_0()
        .child(body)
}

/// 左侧活动栏的图标项。
///
/// 图标来源逐项见 `activity_bar.rs` 模块文档的对照表：**有真源 SVG 的用
/// `lithe-gpui/assets/icons/idea/icons/**`（`ActivityItem::idea`），真源只有内联 React 组件或
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
        // 真源：`icons/idea/icons/ui/listFiles.svg(+_dark)` —— 真机 `FilesIcon`
        // 的同一个文件（`windows/tauri/src/ui/icons/idea-assets.generated.ts:FilesIcon`）。
        ActivityItem::idea(&idea::FILES_ICON, tr("lithe.workbench.project")),
        // 真源：`icons/idea/icons/ui/branch.svg(+_dark)` —— 真机 `GitBranchIcon`。
        ActivityItem::idea(&idea::GIT_BRANCH_ICON, tr("lithe.workbench.changes")),
        // 真源：`icons/idea/icons/ui/search.svg(+_dark)` —— 真机 `MagnifyingGlassIcon`。
        ActivityItem::idea(&idea::MAGNIFYING_GLASS_ICON, tr("lithe.workbench.search")),
        // 真源：`icons/idea/icons/ui/run.svg(+_dark)` —— 真机 `RunIcon`
        // （`features/run/components/run-icon.tsx`）是**内联 React 组件**，没有 SVG 文件可搬；
        // 但图标包里有 `run/run.svg`（彩色运行三角），2026-09-30 起改用它 —— 与 IDEA 的运行
        // 工具窗图标一致，也和终端 / 标题栏上那枚运行按钮同物。
        ActivityItem::idea(&idea::PLAY_ICON, tr("lithe.workbench.run")).bottom(true),
        // ⚠️ 保持 Lucide：真机 `TerminalWindowIcon` 走 `Nucleo.IconSquareTerminalOutline18`
        // → `lucide-react` 的 `square-terminal`，**本来就是 Lucide，已经是 1:1**。
        ActivityItem::new(IconName::SquareTerminal, tr("lithe.workbench.terminal")).bottom(true),
        // 真源：`icons/idea/icons/ui/warningDialog.svg(+_dark)` —— 真机 `WarningIcon`。
        ActivityItem::idea(
            &idea::WARNING_CIRCLE_ICON,
            tr("lithe.workbench.diagnostics"),
        )
        .bottom(true),
        // ⚠️ 保持 Lucide：真机 `GitGraphIcon` 走 `Nucleo.IconGitGraphOutline18` → Lucide
        // `git-graph`，同样是 1:1。
        ActivityItem::new(IconName::GitGraph, tr("lithe.workbench.gitLog")).bottom(true),
        // 真源：`icons/idea/icons/ui/settings.svg(+_dark)` —— 真机 `GearIcon`
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
/// 顺序（0 扩展拼图 / 1 通知 / 2 Maven）：**拼图项不是右工具窗视图**（2026-09-29 起
/// 它开/关中央扩展页，[`RightToolWindowView::from_rail_index`] 对 0 返回 `None`），
/// 通知与 Maven 仍按下标 1 / 2 查表。
///
/// Windows 只在探测到 Maven 项目时才渲染 Maven 按钮（`plugin-activity-rail.tsx:21-23,51` 的
/// `isMavenAvailable`），我们还没有 Maven 项目探测，先固定渲染三项；Maven 视图的空态
/// 如实写"未检测到 Maven 项目"（`lithe.maven.notDetected`）。
/// 右栏三项都没有 `bottom` 分组（Windows 的右栏是单列）。
///
/// ## 2026-09-27：按维护者要求**收敛为三项**，Spring 项从活动栏入口删掉
///
/// 本侧曾在这个列表末尾加过第四项 Spring
/// （`ActivityItem::new(IconName::Leaf, tr("lithe.spring.title"))`）。维护者 2026-09-27 的定稿
/// 是右栏**只留三项：扩展 / 通知 / Maven**（`lithe-gpui/docs/ui-mockup-idea.md` §1.3 的表格），
/// 所以那一项从**活动栏入口**删掉 —— 上面那张表因此与真源 `plugin-activity-rail.tsx:31-67`
/// 逐项相同。
///
/// ## 2026-09-29：扩展项改为**中央扩展页**入口，右窗只剩通知与 Maven
///
/// 拼图项的**入口与组合不变**（仍是右栏三项之一，2026-09-27 的决策不动），变的是**行为**：
/// 点击它开/关中央扩展页（[`crate::extensions_page`]，对齐真机 `openExtensionsBuffer` 的
/// 口径），不再落到右工具窗。连带核对过三处，结论是"行为变更不产生映射死角"：
///
/// 1. [`RightToolWindowView`] 枚举删掉了 `Extensions` 变体 —— 右窗视图、空态、诊断行
///    都不再有"扩展"取值；[`RightToolWindowView::from_rail_index(0)`] 因此返回 `None`；
/// 2. 右栏点击分流（`on_select_right_activity`）在进入右栏状态机**之前**按下标 0 拦截
///    （[`EXTENSIONS_RAIL_IX`]），走 [`ShellWorkspace::toggle_extensions_page`]；
///    高亮判据（[`ShellWorkspace::is_right_activity_active`]）对下标 0 读
///    `extensions_page_open`，对其余下标仍读右栏状态；
/// 3. Spring 视图仍由 `--right-view spring` 探针到达（[`RightToolWindowView::from_id`]）；
///    旧会话里的 `rightView: "extensions"` 恢复时解不出视图，按"未知 id 保留当前值"
///    的既有规则落到默认视图（不会 panic，也不会静默打开别的面板）。
/// 铃铛角点的颜色（`None` = 不画）。
///
/// 红点优先于蓝点，与 IDEA 一致（红点标错误、蓝点标常规事件）。三态判据在
/// [`lithe_gpui_notify::Store::badge`]，这里只把它翻译成主题色 —— 取色要 `&App`，
/// 而 `right_activity_items()` 是无状态函数。
fn notification_badge_color(store: &Entity<lithe_gpui_notify::Store>, cx: &App) -> Option<Hsla> {
    match store.read(cx).badge() {
        lithe_gpui_notify::Badge::None => None,
        lithe_gpui_notify::Badge::Info => Some(cx.theme().info),
        lithe_gpui_notify::Badge::Error => Some(cx.theme().danger),
    }
}

/// 右活动栏三项：扩展 / 通知 / Maven。顺序即下标 —— 拼图项（0）开中央扩展页，
/// 通知（1）与 Maven（2）按下标查右工具窗视图（`RightToolWindowView::from_rail_index`）。
fn right_activity_items(store: &Entity<lithe_gpui_notify::Store>, cx: &App) -> Vec<ActivityItem> {
    // 铃铛的角点：红 = 有未读错误、蓝 = 有未读其它、无未读不画。
    //
    // 现算而不是存字段：角点状态每次通知进来 / 用户打开中心都会变，而这个函数每帧被
    // `render` 调一次。
    let bell = {
        let item = ActivityItem::idea(&idea::BELL_ICON, tr("lithe.notifications.title"));
        match notification_badge_color(store, cx) {
            Some(color) => item.badge(color),
            None => item,
        }
    };
    vec![
        // ⚠️ 保持 Lucide：真机 `PuzzlePieceIcon` 经 `Nucleo` 代理解析到
        // `legacyIconCompatibility.PuzzlePiece = lucideIcons.Puzzle`，**真源物就是 Lucide
        // `puzzle`**，没有 SVG 文件可搬；gpui 用的 `IconName::Puzzle` 是同一个字形。
        ActivityItem::new(IconName::Puzzle, tr("lithe.extensions.title")),
        // 真源：`icons/idea/icons/ui/notifications.svg(+_dark)` —— 真机
        // `BellIcon`（`features/notifications/components/notifications-trigger.tsx:43`）。
        bell,
        // 真源：`icons/idea/icons/files/maven.svg(+_dark)` —— Maven 的蓝色标志（`#3574F0`），
        // 与 IDEA 工具窗 / 轨道上的 Maven 图标同物（维护者 2026-09-30 对照 IDEA 后要求换掉立方体）。
        // 此前用 Lucide `package`，理由是"真源 `MavenIcon` 只是内联 React 组件、没有 SVG 可搬"——
        // 那条理由只说明**真源**没有，没注意到**图标包里早就有**这枚标志（它原先只按"文件类型图标"
        // 用在 `pom.xml` 上），所以借口不再成立。
        ActivityItem::idea(&idea::MAVEN_ICON, tr("lithe.workbench.maven")),
    ]
}

/// 生效的 `javaHomePath` → `lithe-gpui-java` 要的 JDK 覆盖值（**纯函数**，便于单测）。
///
/// 收的是**五个生效值**（[`ToolchainPaths`]，已经按 `项目本机 > 全局默认` 解析过），不是
/// `Settings`：这两个来源的优先级只有一份实现（`lithe_gpui_settings::resolve_overrides`），
/// 这里只负责"空 = 没选"的判据。
///
/// **只读 `javaHomePath`**：另外两个工具链键（`mavenExecutablePath` / `mavenJavaHomePath`）
/// 在 gpui 侧没有任何消费方 —— `maven.scan` 是 Core 进程内的项目描述符解析，本侧不执行 `mvn`
/// （见 `lithe_gpui_java::toolchain` 的模块文档）。登记了没人读只会把"存了不生效"从设置页
/// 搬到这一层，所以这里不登记它们。`mavenSettingsPath` 有真消费方，走它自己那条
/// [`maven_settings_override_from`]（同一个登记函数里一起交给语言服务）。
///
/// 空串 / 全空白等于"没选"（设置页的「清空」写的就是空串，照 `settings/src/project.rs`
/// 的 `non_empty_opt` 同一条口径）；有值时去掉前后空白再转成路径。
fn java_toolchain_override_from(
    paths: &ToolchainPaths,
) -> Option<lithe_gpui_java::JavaToolchainOverride> {
    let trimmed = paths.java_home_path.trim();
    (!trimmed.is_empty()).then(|| lithe_gpui_java::JavaToolchainOverride {
        java_home: PathBuf::from(trimmed),
    })
}

/// 生效的 `mavenSettingsPath` → 语言服务要的 Maven 用户 `settings.xml`（**纯函数**，便于单测）。
///
/// 这个键与 JDK 那一条**同等真实**：它进 `mavenContext.settingsPath`，Core 发布成
/// `java.configuration.maven.userSettings`（`shared/contracts/rust-core-api.md:1170-1178`）。
/// 判据与 [`java_toolchain_override_from`] 逐字相同（空 / 全空白 = 没选）。
///
/// `mavenLocalRepositoryPath` 仍不在这里：它只在 `maven.launchPlan` 里用，今天是死值
/// （见 `lithe_gpui_java::toolchain` 的模块文档）。
fn maven_settings_override_from(paths: &ToolchainPaths) -> Option<PathBuf> {
    let trimmed = paths.maven_settings_path.trim();
    (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
}

/// 设置里的终端字号 → 终端面板要的 `Option<f32>`。
///
/// `terminalFontSize` 用 `0` 表示**不覆盖**（见 `lithe_gpui_settings::schema`），
/// 而终端面板的 `None` 才是"用默认档"（`.text_sm()` 的 14px）。这个转换只做一次，
/// 免得两处调用各自写一遍 `> 0.0` 的判据。
fn terminal_font_size_override(value: f64) -> Option<f32> {
    (value > 0.0).then_some(value as f32)
}

/// 解析（并在首次需要时落盘）本工作区的 `.lithe/project.json` 项目身份。调用点见 `ShellWorkspace::new`。
///
/// ## 为什么接在 `ShellWorkspace::new`
///
/// 产品决策是**打开即建、无条件**（决策与验证证据：
/// `.agents/notes/implemented/architecture/2026-09-27-config-document-semantics-and-workspace-config.md`）：
/// 打开任何一个目录就建立它的 `.lithe/project.json`（带稳定 UUID 身份），写之前先确保
/// `.lithe/` 进了本机排除文件。而 `ShellWorkspace::new` 是**所有"打开"路径的公共收口** ——
/// 它全仓库只有两个调用点：`main.rs` 的启动（argv 给的根）与
/// `rebuild_project_window` 的 `replace_root`（选择器、项目下拉里的最近项目、探针三条入口
/// 都经 `request_open_project` → `execute_project_open` 汇到这里）。接在这一处，就不需要
/// 给每个入口各接一次（多处接会重复调用）。
///
/// ⚠️ **唯一覆盖不到的路径**：`OpenDestination::NewWindow`。它今天**什么都不打开**
/// （多窗口整批暂缓，`execute_project_open` 里只报一句"尚未接入"），所以没有漏。
/// 将来多窗口落地时它会新建一个窗口、同样走 `ShellWorkspace::new`，那时自动被覆盖。
///
/// ## 顺序与失败语义
///
/// - **先确保不共享、再写清单**这条顺序由 `resolve_project_id` 内部保证（它走
///   `save_project_manifest`），所以这里**不**自己去拼 `ensure_project_dir_excluded` ——
///   那样会重实现一遍顺序保证。
/// - ⚠️ **绝不能让打开项目失败**：`.lithe` 建不出来、排除写不了、Core 报错，都只留诊断
///   （`S1_WORKSPACE_CONFIG` 前缀），窗口照常打开、项目照常可用。
/// - **非 Git 目录是正常情况**：守卫返回 `NotARepository`，`.lithe/project.json` 照样建。
///   代价是打开任何目录都会多出一个 `.lithe/`（含非 Git 目录），这是明确接受的产品选择。
///
/// ## 为什么不阻塞 UI 线程
///
/// 这条链会调 Core（`git.write` 内部要 `git rev-parse`，是子进程），所以阻塞的那一段走
/// `background_spawn`；任务 detach 掉，外壳被换根丢掉时也不影响它（它不碰实体）。
/// ## 失败必须**对用户可见**（不只是 stderr）
///
/// 失败是异步到达的（这条链在后台执行器上），所以它经 [`ShellWorkspace::notify`] 记进
/// 通知中心 —— 弹一条**左下角 toast**（5 秒）并入档，不再画常驻红条（2026-09-29 收拢，
/// 消息面只剩 toast 一层）。只打 stderr 是不够的：双击启动的用户看不到它，只会发现
/// "`.lithe` 目录没出现"。走 store 而不是在后台任务里直接 `push_notification` 的理由
/// 不变：后台任务只保证"实体活着"，而 toast 要求窗口与 `Root` 就位 —— 外壳持有 store，
/// `this.update` 之后由外壳的订阅（窗口在场）派发 toast，异步边界两侧都稳。
fn prepare_workspace_config(root: PathBuf, cx: &mut Context<ShellWorkspace>) {
    cx.spawn(async move |this, cx| {
        let task_root = root.clone();
        let (identity, settings_tracked) = cx
            .background_spawn(async move {
                let identity = lithe_gpui_shared::workspace_config::resolve_project_id(&task_root);
                // 顺手问一次 Git：`.lithe/settings.json` 是否已被跟踪。设置页用它把外观来源
                // 显示成「团队设置」还是「本项目的设置」（判据见
                // `settings/src/workspace.rs` 的 `AppearanceSource`）。
                let tracked = workspace_settings_tracked(&task_root);
                (identity, tracked)
            })
            .await;

        // 回填"共享层文件有没有被跟踪"（`None` 不写：那表示"还没问到"，界面按尚未提交显示）。
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
                // 通知中心 + toast：条目存 `(code, params)`，文案键就是原来红条用的
                // `lithe.gpui.workspaceConfigFailed`，`reason` 填 `WorkspaceConfigError` 的
                // Display 原文（"确保本机排除失败：…" / "清单文件读写失败：…"），
                // 用户据此知道该查什么。toast 与中心显示**同一句**（都走 `render_message`）。
                let reason = error.to_string();
                // 实体可能已经销毁（换根丢掉外壳）：`update` 返回 `Err` 时静默忽略，不 panic。
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

/// `.lithe/settings.json` 是否已被 Git 跟踪。
///
/// ## 为什么需要问 Git
///
/// 外观被工作区覆盖时，界面要说清"来自哪一层"，而 `.lithe/settings.json` 这一层有**两个**
/// 状态：已被提交（团队设置）与还没提交（本项目的设置）—— 判据只能是 Git 的索引，
/// 不能靠"文件在不在"（那两种情况文件都在）。设计 Note 第七节的表就是这么定的。
///
/// ## 判据与失败语义
///
/// - `git ls-files --error-unmatch -- .lithe/settings.json` 的**退出码**：`0` = 在索引里。
///   用 `--error-unmatch` 而不是 `ls-files` 的裸输出，是因为要的是"已跟踪"这个是非判断，
///   而不是一份可能很长的清单。
/// - **不是仓库、没有这个文件、Git 不在、Core 报错**，全部收敛成 `false`（= 尚未提交）。
///   这是**更弱也更安全**的那个承诺：把未跟踪的文件说成"团队设置"会让用户以为同事也看得到它。
/// - 走 `background_spawn`（Core 要起 Git 子进程），**不允许落在 UI 线程**。
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
        // `Ok(None)` = Core 返回空载荷（理论上不会），按未跟踪处理。
        Ok(None) => false,
        Err(error) => {
            // 非 Git 目录是**正常情况**（打开即建那一条已经接受它），所以只在 stderr 留一行，
            // 不挡任何流程、也不冒充成"已跟踪"。
            eprintln!(
                "S1_WORKSPACE_CONFIG settings_tracked_unknown root={} error={error}",
                root.display()
            );
            false
        }
    }
}

/// 把生效的 JDK 与 Maven `settings.xml` 覆盖值登记给语言服务（调用点见 `ShellWorkspace::new`）。
///
/// **优先级 `项目本机 > 全局默认 > 自动发现`**：项目本机层是 `.lithe/run.local.json` 的
/// `toolchain` 对象与 `.lithe/maven.local.json`（[`load_local_toolchain`]），全局默认是设置文件里
/// 那五个键（`Overrides::from_settings`）。两层怎么合只有一份实现
/// （`lithe_gpui_settings::resolve_overrides`），这里只做"读两层 → 登记"与一行启动证据。
///
/// 判据全在两个 `*_from` 里。设置状态不存在时（测试或别的宿主里没有 `SettingsStore`）全局层
/// 按全空处理，与 `try_store` 在别处"没有设置就回落默认值"的口径一致，不 panic。
///
/// ⚠️ 这里同步读两份很小的 JSON（不碰 Core、不起子进程）。**不要**在这里调
/// `ensure_project_dir_excluded` 之类会起 `git` 子进程的东西：本函数在 `ShellWorkspace::new`
/// 与设置变化回调里被调用，都属于 UI 线程的路径。
///
/// ⚠️ **改了项目本机层的值，语言服务要重启才吃到新值**（与"换 JDK 必须重建 JDT 索引"同一条
/// 既有口径：`EditorPane::prepare_java` 一个工作区一个会话、幂等）。所以这里不尝试热重启会话。
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
        // 五个值各自的来源（可 grep）：证明"项目本机压过全局"这条优先级真的生效。
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

    /// 侧栏拖动改宽的算术：起点 + 位移，并钳在允许范围内。
    ///
    /// 守的是 2026-09-27 那轮的教训：拖拽链路里**只有这段算术可以纯函数验证**
    /// （本 crate 拿不到 `TestAppContext`、起不了窗口，鼠标事件注入只能由维护者在真机做），
    /// 而"宽度算错"正是那一轮出过问题的地方（当时还因为误用只能在 paint 期调用的
    /// `Window::on_mouse_event` 当场 panic 过）。
    ///
    /// 三条判据：**向右拖变宽**、**向左拖变窄**、**两端都被钳住**。
    #[test]
    fn drag_width_follows_the_pointer_and_stays_in_range() {
        // 左栏走 [`clamped_drag_span`]：手柄在**后缘**（`drag_direction = +1`），区间是
        // **真源**的下界 30 与上界 `min(520, 容器宽 − 400)`（`pane_span_range`；这里取一个
        // 1280 宽的容器 ⇒ 上界 = min(520, 880) = 520）。
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
        // 从 320 起拖：鼠标右移 100 → 420。
        assert_eq!(left(100., 320., 200.), 420.);
        // 鼠标左移 100 → 220。
        assert_eq!(left(300., 320., 200.), 220.);
        // 一路左拖到负无穷也只会停在最小宽（真源 30）。
        assert_eq!(left(0., 320., -10_000.), SIDEBAR_MIN_WIDTH_SPEC);
        // 一路右拖也只会停在最大宽（这个容器宽下 = 静网上界 520）。
        assert_eq!(
            left(0., 320., 10_000.),
            left_sidebar_max_width(container_width)
        );
        // 起点宽本身就是钳过的值：不动鼠标 ⇒ 结果等于起点宽（拖动不会自己漂移）。
        assert_eq!(left(500., 320., 500.), 320.);
    }

    /// 左栏上界 = 真源 `max(30, min(520, 容器宽 − 400))`（`WorkbenchView.swift:2008-2011`）。
    ///
    /// 逐条钉住四个边界（这是第 2 条遗留"按真源改"的机器判据）：
    ///
    /// 1. **容器宽刚好等于 400**（= 编辑区保留宽）⇒ `available − 400 = 0` ⇒ 收到下界 30；
    /// 2. **容器宽小于 400** ⇒ 仍是 30（真源外层那个 `max(minimumSidebarWidth, …)`，不能出现负数）；
    /// 3. **容器很宽** ⇒ 封顶静网上界 520；
    /// 4. 中间区间线性 = `容器宽 − 400`。
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

        // ① 容器宽刚好 = 400：上界收到下界（不是 0，也不是负数）。
        assert_eq!(left_sidebar_max_width(400.), SIDEBAR_MIN_WIDTH_SPEC);
        // ② 容器比 400 还窄（真源 max(0, …) 之后仍要保证 min <= max）。
        assert_eq!(left_sidebar_max_width(400. - 1.), SIDEBAR_MIN_WIDTH_SPEC);
        assert_eq!(left_sidebar_max_width(30.), SIDEBAR_MIN_WIDTH_SPEC);
        // ③ 很宽：封顶 520（1920 的窗口也只会给 520，不会给 1520）。
        assert_eq!(left_sidebar_max_width(1920.), SIDEBAR_MAX_WIDTH_SPEC);
        assert_eq!(left_sidebar_max_width(10_000.), SIDEBAR_MAX_WIDTH_SPEC);
        assert_eq!(
            left_sidebar_max_width(920.),
            SIDEBAR_MAX_WIDTH_SPEC,
            "正好封顶"
        );
        // ④ 中间区间就是 容器宽 − 400。
        assert_eq!(left_sidebar_max_width(700.), 300.);
        assert_eq!(left_sidebar_max_width(919.), 519.);
        // ⑤ 容器尺寸还不可知（构造期 / 首帧 / 恢复会话）：给静网上界，
        //    不能按"容器只有 0 宽"把用户存下来的宽度钳到下界。
        assert_eq!(left_sidebar_max_width(0.), SIDEBAR_MAX_WIDTH_SPEC);
        assert_eq!(left_sidebar_max_width(-1.), SIDEBAR_MAX_WIDTH_SPEC);
        assert_eq!(left_sidebar_max_width(f32::NAN), SIDEBAR_MAX_WIDTH_SPEC);
        assert_eq!(
            left_sidebar_max_width(f32::INFINITY),
            SIDEBAR_MAX_WIDTH_SPEC
        );
    }

    /// 第 1 条遗留：底部工具窗的常量与 clamp **自洽**，所以"改常量"是**视觉无变化**的。
    ///
    /// 三条判据：
    ///
    /// 1. 默认常量 == 真源最小高（`WorkbenchView.swift:2019` 的 `minimumGitPaneHeight = 260`）；
    /// 2. 默认值等于下界 ⇒ `pane_span_range` 的钳制在默认值上**恒等**（改前 240 < 260，
    ///    所以首帧一渲染就被钳到 260 —— 这正是"240 从来没画出来过"的证明）；
    /// 3. 默认值不低过下界这种"墓碑式自相矛盾"不会再出现。
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
        // 默认值经过完整区间（上界取一个正常窗口高度）钳制后不变 —— 视觉无变化的机器证据。
        let (min, max) = (BOTTOM_PANE_MIN_HEIGHT_SPEC, bottom_pane_max_height(800.));
        assert_eq!(
            clamp_pane_span(BOTTOM_PANE_HEIGHT_SPEC, min, max),
            BOTTOM_PANE_HEIGHT_SPEC
        );
    }

    /// 第 3 条遗留：右栏默认 **360**（真源 `WorkbenchLayoutStore.swift:5`），
    /// 并且**只在没有持久化值时生效** —— 用户存过的宽度不被默认值覆盖。
    ///
    /// 会话文档的三个尺寸都是 `Option<f32>`（缺键 = `None`），`pane_span_on_restore` 是
    /// `restore_session` 的唯一判据：`None` ⇒ 保留初值；`Some` ⇒ 用存过的值（按**当前**
    /// 容器/窗口尺寸钳一遍）。
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
        // 没有持久化值 ⇒ 用默认值（360 落在 300…760 内，不被钳）。
        assert_eq!(
            pane_span_on_restore(None, RIGHT_TOOL_WINDOW_WIDTH_SPEC, min, max),
            RIGHT_TOOL_WINDOW_WIDTH_SPEC
        );
        // 存过值 ⇒ 用存过的值，不被 360 顶掉。
        assert_eq!(pane_span_on_restore(Some(413.25), 360., min, max), 413.25);
        assert_eq!(pane_span_on_restore(Some(500.), 360., min, max), 500.);
        // 存过的值越界 ⇒ 收敛到区间（文档层刻意不判上下界，钳制在这里）。
        assert_eq!(pane_span_on_restore(Some(9_999.), 360., min, max), max);
        assert_eq!(pane_span_on_restore(Some(-5.), 360., min, max), min);
        assert_eq!(pane_span_on_restore(Some(f32::NAN), 360., min, max), min);
        // 左栏 / 底部窗同一判据（缺键保留初值）。
        assert_eq!(pane_span_on_restore(None, 320., 30., 520.), 320.);
        assert_eq!(pane_span_on_restore(None, 260., 260., 580.), 260.);
    }

    /// **方向**：右栏与底部窗的手柄在**前缘**，往左 / 往上拖才是变长。
    ///
    /// 守的是最容易写反的一处：把右栏的 `drag_direction` 也写成 `+1`，拖右栏就会"往右拖变宽"——
    /// 看上去像手柄没反应 / 反着走，而这类错误在纯算术里一眼能看穿，真机上要试半天。
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
        // 指针往**左**移 60（400 → 340）⇒ 右栏变宽 60。
        assert_eq!(right(500., 400., 440.), 460.);
        // 指针往右移回去 ⇒ 变窄，且停在 300 下界。
        assert_eq!(right(500., 400., 900.), RIGHT_TOOL_WINDOW_MIN_WIDTH_SPEC);

        // 底部窗同一条判据，只是轴换成 y：往上拖（y 变小）才是变高。
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

    /// 三个手柄的 `ElementId` **必须互不相同**（gpui-kit 官方 `element_id` 那条坑）。
    ///
    /// 上游文档的原话是：三个 `resize_handle` 落在同一个带 key 的祖先下、都叫
    /// `"resize-handle"` 时会拿到同一个 `GlobalElementId`，"按住一个 handle 可能触发另一个的
    /// 拖动"。这条用例把它变成机器判据 —— 三个手柄现在确实住在同一个祖先下
    /// （`render` 里最外层那个 `h_flex`），正是那个坑的形状。
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
        // 贴边与轴：左栏在后缘 + 水平轴；右栏在前缘 + 水平轴；底部窗在前缘 + **垂直**轴。
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

    /// 右栏的上界**按可用宽度收敛**（真源 `WorkbenchRightToolSplitView.swift:7-16`）。
    ///
    /// 宽窗口里是静网上界 760；窗口窄到放不下 640 的中央列时，上界一路收到下界 300 ——
    /// 这就是"窗口变窄时要能收敛"，而且**永远不会**返回小于下界的上界
    /// （`min > max` 会让 `f32::clamp` 直接 panic）。
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
        // 窗口尺寸还不可知（构造期 `viewport_size()` 可能是 0）：给静网上界，
        // 不能按"窗口 0 宽"把恢复出来的宽度钳到下界。
        assert_eq!(
            right_tool_window_max_width(0.),
            RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC
        );
        assert_eq!(
            right_tool_window_max_width(f32::NAN),
            RIGHT_TOOL_WINDOW_MAX_WIDTH_SPEC
        );
    }

    /// 底部窗的上界同理，且恢复一个**越界值**时收敛到区间内。
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

        // 恢复一个在宽屏上存下来的 760、当前窗口只有 800 高：收敛到上界而不是溢出。
        let restored = clamp_pane_span(
            760.,
            BOTTOM_PANE_MIN_HEIGHT_SPEC,
            bottom_pane_max_height(800.),
        );
        assert_eq!(restored, bottom_pane_max_height(800.));

        // 恢复越界 / 坏值：负值、NaN、±inf 一律落回下界 / 顶到界内（`f32::clamp(NaN, ..)` 会
        // 返回 NaN，那一帧布局会整片塌掉，所以 NaN 单独挡）。
        assert_eq!(clamp_pane_span(-5., 260., 580.), 260.);
        assert_eq!(clamp_pane_span(f32::NAN, 260., 580.), 260.);
        assert_eq!(clamp_pane_span(f32::NEG_INFINITY, 260., 580.), 260.);
        assert_eq!(clamp_pane_span(f32::INFINITY, 260., 580.), 580.);
        // 区间本身反了（窗口比两端下界之和还窄）：收敛到下界，不 panic。
        assert_eq!(clamp_pane_span(500., 300., 120.), 300.);
    }

    /// B2：`menu_bar::install_key_actions` 绑的六条键位，逐条对着 `MENUS` 与 keymap 语法定一遍。
    ///
    /// 这条守的是 Q13 的两面：
    ///
    /// 1. **键位字面量必须是 keymap 认得的**（`Keystroke::parse` 失败 = 那颗键永远按不出，
    ///    而菜单上照样会显示胶囊 —— 正是 Q13 禁止的"显示了却按不出效果"）；
    /// 2. **每一条都必须真的画在某个菜单里**（否则"按了这颗键会改状态"没有可见入口，
    ///    用户永远不知道它存在；对照 `install_open_project_action` 的 `Ctrl+O`
    ///    就对应「文件 → 打开文件夹」这一项）。
    ///
    /// ⚠️ 它**不能**断言"回调真的被登记了"（`App::on_action` 要一个 `App`，
    /// 而本 crate 拿不到 `TestAppContext`，理由见 `menu_bar.rs` 里那条说明）。
    /// 那一半由实机键盘注入覆盖（`.artifacts/p24/`）。
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
            // U20：「转到行」的 `Ctrl+G`（真源 Windows 平台表，见 `MenuAction::GoToLine`）。
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

        // 反向：`MENUS` 里**只有**这七条带键位字面量（B2 的六条 + U20 的 `Ctrl+G`；
        // 改它就是改验证脚本）。
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

    /// `mavenSettingsPath` 同样"空 = 没选"，且**与 JDK 那一条互不依赖**：
    /// 只填 Maven 配置（不填 JDK）时它照样要送到语言服务 —— 合成一个 `Option`
    /// 会让"选了 settings.xml 但没选 JDK"这条最常见的用法静默失效。
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
        // 同一份生效值里 JDK 还是"没选"：两条判据互不影响。
        assert_eq!(super::java_toolchain_override_from(&paths), None);
    }

    /// **优先级 `项目本机 > 全局默认 > 自动发现`** —— 在"登记给语言服务"这个边界上验一次。
    ///
    /// 这一层只搬运五个值，优先级实现只有一份（`lithe_gpui_settings::resolve_overrides` →
    /// `lithe_gpui_shared::workspace_config::ToolchainPaths::resolve`），但"登记确实用了生效值"
    /// 这件事必须在**调用边界**上钉住：否则界面显示一套、语言服务拿另一套。
    #[test]
    fn project_local_toolchain_wins_over_the_global_default_when_registering() {
        use lithe_gpui_shared::workspace_config::OverrideOrigin;

        let global = Overrides {
            java_home: "C:\\global\\jdk".to_string(),
            maven_settings: "C:\\global\\settings.xml".to_string(),
            ..Overrides::default()
        };
        // 项目本机只设了 JDK：Maven 的 settings.xml 应当回落到全局那一个。
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
        // 与本表之外的 id 也不能撞（面板那七条 + 几条只由快捷键/菜单触发的：
        // 切换菜单栏 / 保存 / 命令面板 / 快速打开 / 全局搜索 / 在文件中替换 / 转到定义）。
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

    /// Search Everywhere 的 `Actions` 档列的是 [`CommandId::ALL`]（**整个**命令目录 22 条），
    /// 不是命令面板那份 7 条短名单。这条测试钉住三件事：
    ///
    /// 1. `ALL` 覆盖全部 22 个变体、顺序 = 声明顺序、id 互不相同；
    /// 2. 它是 `COMMAND_ORDER` 的**超集**（面板里有的每一条都必须能在 Actions 档里搜到）；
    /// 3. 每一条都能被 [`command_action`] 投影出非空文案（Actions 档的行就是这么来的）——
    ///    顺带证明这套投影在纯单测里可跑（不依赖 `Window` / `App`）。
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

        // 每一条都能投影出可画的文案（label / description 非空）。
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

        // 没有数据层的视图永远不排任务：通知是推送写入；扩展页在自己的构造任务里拉目录
        // （2026-09-29 起它根本不是右窗视图，连 `RightScanState` 都不经过）。
        assert_eq!(
            state.request(RightToolWindowView::Notifications, true),
            None
        );
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
