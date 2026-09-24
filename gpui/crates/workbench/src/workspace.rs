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
//! │  ├─ 右侧工具窗 400    default-settings.ts:139   settings.rightToolWindowWidth
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

use std::path::PathBuf;

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::{ActiveTheme as _, Root};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Div, Entity, IntoElement, ParentElement as _,
    Pixels, Render, SharedString, Styled as _, Window, div, px,
};

use lithe_gpui_editor::EditorPane;
use lithe_gpui_explorer::Explorer;
use lithe_gpui_git::BottomPane;
use lithe_gpui_shared::tr;
use lithe_gpui_terminal::TerminalPane;

use crate::activity_bar::{ActivityItem, ActivitySide, activity_bar};
use crate::project_tabs::{ProjectTab, project_tabs};
use crate::status_bar::{StatusEntry, status_bar};
use crate::title_bar::title_bar;

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

/// 右侧工具窗宽度。真源 `default-settings.ts:139`（`rightToolWindowWidth: 400`）。
///
/// ⚠️ **保留 `px(...)`**：400 不在 gpui 的固定 rem 档位上（档位里 96 → 384、112 → 448，
/// `gpui-pre-macros-0.3.6/src/styles.rs:1063-1077`），不能自己发明一个 `w_100()`。
const RIGHT_TOOL_WINDOW_WIDTH: Pixels = px(400.);

/// 编辑器岛 / 侧栏外壳圆角。真源 `theme.css:9,134`：`rounded-xl = calc(--radius * 1.4) = 11.2`。
///
/// ⚠️ **保留 `px(...)`**：11.2 **不是** gpui 的 rem 档位（gpui 的 `rounded_xl()` 是 12px，
/// `gpui-pre-macros-0.3.6/src/styles.rs:1255-1259`）；也不能从主题读 —— `ThemeConfig.radius`
/// 是 `usize`（`gpui-component-0.6.6/src/theme/schema.rs:67-68`），装不下 Lithe 的
/// `--radius × k` 阶梯（4.8 / 6.4 / 11.2），而把主题半径改大又会连带改掉所有 gpui-kit 组件。
const ISLAND_RADIUS: Pixels = px(11.2);

/// 底部工具窗当前显示哪一个内容。
///
/// Windows 的底部窗**没有自己的标签条**，由活动栏 / 命令面板切换一个单值
/// `bottomPaneActiveTab`（默认 `"terminal"`，
/// `features/window/stores/workspace-ui-defaults.ts:6`、`stores/ui-state/panel-slice.ts:31`）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomPaneKind {
    /// 终端（阶段 5）。
    Terminal,
    /// Git 提交记录（阶段 4）。
    Git,
    /// 这一组活动栏项还没有内容（Maven / 运行 / 诊断）。
    Pending(&'static str),
}

/// 活动栏第 `index` 项对应的底部窗内容；`None` = 该项不换底部窗。
///
/// ⚠️ 下标必须与 [`activity_items`] 的顺序一致（0 项目 / 1 更改 / 2 搜索 / 3 Maven /
/// 4 运行 / 5 终端 / 6 诊断 / 7 提交记录 / 8 设置）。「设置」在真机是对话框，
/// 不是底部窗，所以这里是 `None`。
fn bottom_pane_for(index: usize) -> Option<BottomPaneKind> {
    match index {
        3 => Some(BottomPaneKind::Pending("Maven")),
        4 => Some(BottomPaneKind::Pending("运行")),
        5 => Some(BottomPaneKind::Terminal),
        6 => Some(BottomPaneKind::Pending("诊断")),
        7 => Some(BottomPaneKind::Git),
        _ => None,
    }
}

/// 工作台根视图。
pub struct ShellWorkspace {
    /// 工作区根目录，来自命令行参数（阶段 4 的 `git.*` 与后面的设置都要用）。
    root: PathBuf,
    /// 项目标签条的数据。
    projects: Vec<ProjectTab>,
    /// 当前选中的项目标签（`None` = 一个都没选中）。
    active_project: Option<usize>,
    /// 两条活动栏共用的图标项。
    activity_items: Vec<ActivityItem>,
    /// 当前选中的活动栏项（`items` 的下标）。
    active_activity: Option<usize>,
    /// 状态栏左组（前导项）。
    footer_left: Vec<StatusEntry>,
    /// 状态栏右组（尾随项）。
    footer_right: Vec<StatusEntry>,
    /// 左侧栏内容：项目树（真实 `workspace.snapshot` 数据）。
    explorer: Entity<Explorer>,
    /// 中央列内容：编辑区（标签栏 + 正文 / 空状态）。
    editor: Entity<EditorPane>,
    /// 底部窗里的终端（真机默认显示的就是它）。
    terminal: Entity<TerminalPane>,
    /// 底部窗里的 Git 提交记录。
    bottom_git: Entity<BottomPane>,
    /// 底部窗当前显示的内容。
    bottom_kind: BottomPaneKind,
    /// 底部窗是否展开（再点一次当前活动栏项可以收起）。
    bottom_visible: bool,
}

impl ShellWorkspace {
    /// 建立工作台视图。
    ///
    /// `Root::new` 之前不得打开任何浮层（那时窗口根还不是 `Root`，`window.open_dialog` 会 panic）。
    pub fn new(root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 先建编辑区，再把它的弱引用交给项目树：点文件 → 打开到编辑区。
        let editor = cx.new(|cx| EditorPane::new(window, cx));
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

        let terminal = cx.new(|cx| TerminalPane::new(window, cx));
        let bottom_git = cx.new(|cx| BottomPane::new(root.clone(), window, cx));

        let project_name: SharedString = root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| root.display().to_string())
            .into();

        let branch = read_branch(&root).unwrap_or_else(|| "—".to_string());

        Self {
            root,
            projects: vec![ProjectTab::new(project_name.clone())],
            // 单项目时 Windows 会隐藏整条标签条（`project-tab-bar-model.ts:12-13`）；
            // 阶段 1 只有一个项目，仍然把 `Some(0)` 选中，方便验收外观。
            active_project: Some(0),
            activity_items: activity_items(),
            active_activity: None,
            footer_left: vec![
                // 前导项顺序真源：`features/layout/config/item-order.ts:20-32`
                // `FOOTER_LEADING_ITEM_IDS = ["filePath", "branch"]`。
                StatusEntry::new(project_name).with_icon(IconName::FileText),
                // 真实字形 `git-branch`（`icons/git-branch.svg`，全量目录 `AllAssets` 已注册）。
                StatusEntry::new(SharedString::from(branch)).with_icon(IconName::GitBranch),
            ],
            footer_right: vec![
                // 尾随项顺序真源：`["cursor", "encoding", "indent", "readOnly", "memory", "gitChanges"]`。
                StatusEntry::new(SharedString::from("1:1")),
                StatusEntry::new(SharedString::from("UTF-8")),
                // 文案形如 `{count} 个空格`（`i18n/locale.ts` 的 `footer.spaces`），默认 2。
                StatusEntry::new(lithe_gpui_shared::tr_args(
                    "lithe.footer.spaces",
                    &[("count", "2")],
                )),
                // 只读态的真实锁字形（`icons/lock.svg`）；可写态同契约里的 `LockOpen`。
                StatusEntry::new(SharedString::from("")).with_icon(IconName::Lock),
                StatusEntry::new(SharedString::from("总计 0.0 MB · Lithe 0.0 MB"))
                    .with_icon(IconName::HardDrive),
                StatusEntry::new(SharedString::from("")).with_icon(IconName::CircleCheck),
            ],
            explorer,
            editor,
            terminal,
            bottom_git,
            // 真机默认就是终端（`workspace-ui-defaults.ts:6`）。
            bottom_kind: BottomPaneKind::Terminal,
            bottom_visible: true,
        }
    }

    /// 右侧工具窗占位内容（阶段 6 换成真实工具窗）。
    ///
    /// 顺带把工作区根打出来：这既是"命令行参数确实传进来了"的现场证据，也让 `root`
    /// 在阶段 4 的 `git.*` 接手之前不至于是个死字段。
    fn right_tool_window_placeholder(&self, cx: &App) -> impl IntoElement {
        placeholder(
            format!("右侧工具窗（阶段 6）· 工作区：{}", self.root.display()),
            cx,
        )
    }
}

impl Render for ShellWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // ⚠️ 顺序不可调换：这三个 `render_*_layer` 借用 `cx`，必须先把值取出来，
        // 再调用任何其它借用 `cx` 的方法（`cx.theme()` 等）。
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let sheet_layer = Root::render_sheet_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

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
        let on_select_activity = {
            let handle = handle.clone();
            move |index: usize, window: &mut Window, cx: &mut App| {
                // 用 `update` 的返回值带出判断，避免在闭包里捕获 `bool`：这个闭包要同时
                // 给左右两条活动栏用（`Fn`），捕获可变的局部量会让它退化成 `FnMut`。
                let shows_terminal = handle.update(cx, |this, cx| {
                    match bottom_pane_for(index) {
                        // 底部组：切底部窗内容；已经是它且可见 → 再点一次收起。
                        Some(kind) => {
                            if this.active_activity == Some(index) && this.bottom_visible {
                                this.bottom_visible = false;
                            } else {
                                this.bottom_visible = true;
                                this.bottom_kind = kind;
                            }
                            this.active_activity = Some(index);
                        }
                        // 顶部组（项目 / 更改 / 搜索）与「设置」不换底部窗，只切选中态。
                        None => {
                            this.active_activity =
                                (this.active_activity != Some(index)).then_some(index);
                        }
                    }
                    cx.notify();
                    this.bottom_visible && this.bottom_kind == BottomPaneKind::Terminal
                });
                // 终端输入行只能在**首次渲染之后**才拿得到焦点（构造期窗口根还不是 `Root`），
                // 所以在这里补一次。
                if shows_terminal {
                    let _ = terminal_handle.update(cx, |pane, cx| pane.focus_input(window, cx));
                }
            }
        };

        let project_name: SharedString = self
            .projects
            .first()
            .map(|tab| tab.name().clone())
            .unwrap_or_else(|| SharedString::from("Lithe"));

        let left_rail = activity_bar(
            ActivitySide::Left,
            &self.activity_items,
            self.active_activity,
            on_select_activity.clone(),
            window,
            cx,
        );
        let right_rail = activity_bar(
            ActivitySide::Right,
            &self.activity_items,
            None,
            on_select_activity,
            window,
            cx,
        );

        let explorer = self.explorer.clone();
        let editor = self.editor.clone();
        let right_tool_window = self.right_tool_window_placeholder(cx);

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
                BottomPaneKind::Pending(label) => {
                    placeholder(format!("{label} 工具窗（未实现）"), cx).into_any_element()
                }
            };
            bottom_pane(content, cx)
        });

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            // ① 标题栏 40（含自绘窗口三键 56×40）。
            .child(title_bar(&project_name, window, cx))
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
                    .child(side_pane(div().w_80(), explorer, cx))
                    .child(
                        // 中央列 = 编辑器岛 + 底部工具窗（默认口径：嵌在中央列内）。
                        v_flex()
                            .flex_1()
                            .h_full()
                            .min_w_0()
                            .child(editor_island(editor, cx))
                            .children(bottom),
                    )
                    .child(side_pane(
                        div().w(RIGHT_TOOL_WINDOW_WIDTH),
                        right_tool_window,
                        cx,
                    ))
                    .child(right_rail),
            )
            // ④ 状态栏 24。
            .child(status_bar(
                &self.footer_left,
                &self.footer_right,
                window,
                cx,
            ))
            // 浮层三层必须挂在最外层视图上，否则对话框 / 抽屉 / 通知静默不显示。
            .children(dialog_layer)
            .children(sheet_layer)
            .children(notification_layer)
    }
}

/// 左右两侧的面板外壳：固定宽、不参与收缩、`bg-background`、圆角 11.2 + 1px 边框。
///
/// Windows 的左右面板是 `lithe-glass-island rounded-xl border-border`（`resizable-pane.tsx:203-206`）。
/// **拖拽改宽还没接**（`ResizablePane` 的 4px 热区）。
///
/// `outer` 由调用方给：宽度是布局度量，调用点用 gpui 的 rem 档位 helper（`w_80()`）或
/// 档位外的 `px(...)` 各自表达，这里只负责外壳剩下的部分。
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

/// 两条活动栏的图标项。
///
/// 图标类型是全量目录的 `gpui_kit::assets::IconName`（1830 个 Lucide 字形，应用已注册
/// `AllAssets`），所以 git / 提交图 / Maven 这些位置都用**真实字形**，没有替代。
/// 逐项对照表见 `activity_bar.rs` 的模块文档「图标」一节。
fn activity_items() -> Vec<ActivityItem> {
    // 顶部组：`sidebar-pane-selector.tsx:311-319` 的 files / git / search。
    // 底部组：`features/layout/config/item-order.ts:12-19` 的
    // `SIDEBAR_BOTTOM_ACTIVITY_ITEM_IDS = [maven, run, terminal, diagnostics, gitLog, settings]`。
    vec![
        ActivityItem::new(IconName::FolderOpen, tr("lithe.workbench.project")),
        // 真实字形 `git-branch`（`icons/git-branch.svg`）。
        ActivityItem::new(IconName::GitBranch, tr("lithe.workbench.changes")),
        ActivityItem::new(IconName::Search, tr("lithe.workbench.search")),
        // Lucide 没有 Maven 字形，取"包 / 构建产物"语义的 `package`（`icons/package.svg`）。
        ActivityItem::new(IconName::Package, "Maven").bottom(true),
        ActivityItem::new(IconName::Play, tr("lithe.workbench.run")).bottom(true),
        ActivityItem::new(IconName::SquareTerminal, tr("lithe.workbench.terminal")).bottom(true),
        ActivityItem::new(IconName::TriangleAlert, tr("lithe.workbench.diagnostics")).bottom(true),
        // 真实字形 `git-graph`（`icons/git-graph.svg`）。
        ActivityItem::new(IconName::GitGraph, tr("lithe.workbench.gitLog")).bottom(true),
        ActivityItem::new(IconName::Settings, tr("lithe.workbench.settings")).bottom(true),
    ]
}
