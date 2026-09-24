//! 底部工具窗：macOS 的 `GitLogView`（Log / Worktrees / Console 三页签）。
//!
//! 规格真源：`.artifacts/ui-map/05-git.md`（下称「规格」），实现真源
//! `macos/Sources/Lithe/Views/Git/`。本文件此前照 **Windows 端**
//! （`windows/tauri/src/features/git/components/log/git-log-tool-window.tsx`）做的是
//! `提交记录` 工具窗；本轮改为 macOS 口径，行号证据写在每处实现旁。
//!
//! ## 与 macOS 的对应关系
//!
//! | 元素 | macOS 出处 | 实现 |
//! |---|---|---|
//! | 标题栏：icon 14 + `Git` 13.5 semibold + 页签 + 菜单 + 隐藏，高 32，左右内边距 12 / 7 | `GitLogView.swift:380-461` | [`BottomPanel::render_header`] |
//! | 页签：高 27、圆角 5、选中 `subtleSelection` 底 + 1px `inputFocusBorder`（**不是**彩色下划线） | `GitLogView.swift:463-519` | [`BottomPanel::tab_button`] |
//! | Log 页签标题 `日志：<引用>`（没选中引用时是 `日志：<当前分支>`，两者都没有才 `日志：所有引用`） | `GitLogView.swift:394-397`、zh-Hans `"Log: %@"` | [`BottomPanel::render_header`] |
//! | Worktrees 页签的 `· <仓库根>` 补充信息 | `GitLogView.swift:398-402 / :479-485` | [`BottomPanel::tab_button`] |
//! | 三栏：引用 220 / min 180，提交 弹性 / min 340，提交文件 350 / min 250 | `GitLogView.swift:2832-2904` | [`BottomPanel::render_log`] |
//! | 栏间分隔条可见 5 | `Views/Workbench/SplitHandleView.swift:13` | [`split_handle_appearance`] |
//! | 详情栏上下再分：文件栏 min 90、提交详情默认 156 / min 110 | `GitLogView.swift:1096-1108` | [`BottomPanel::render_detail_pane`] |
//! | 提交行 22、作者列 104 左对齐、日期列 110 右对齐等宽 | `GitGraphGeometry.swift:8`、`GitGraphView.swift:559-577` | [`commit_row`] |
//! | 引用徽标：高 16、圆角 4、水平间距 5、从右往左堆叠 | `GitGraphView.swift:578-580 / :612-626` | [`commit_row`]、[`decoration_badge`] |
//! | 引用树行 / 分组行 28、缩进 `depth*16`、left 8 / right 8、图标槽 16 | `GitLogView.swift:74 / :2674-2732` | [`reference_row`] |
//! | 提交文件树行 28 | `GitCommitFileTreeView.swift:120-121` | [`commit_file_row`] |
//! | 空态 / 提示文案 | `macos/Resources/zh-Hans.lproj/Localizable.strings` | 全文 |
//!
//! ## 提交行为什么是自绘行（本项目实测的坑）
//!
//! 22 行高的提交行走的是**自绘行 + 手铺 `v_flex()`**，不是 `DataTable`：
//!
//! 1. `DataTable` 的表体是 `uniform_list`，**行高取第 0 行内容的测量高度**；表级
//!    `with_size(Size::Size(px(22.)))` 压不住它 —— 带 `border_1` 的徽标自然高度 30，
//!    实测得到「行高 18 / 内容 30」，相邻行互相覆盖（`.artifacts/p1/git-table-22.png`）。
//! 2. macOS 的提交表**没有表头**，列宽是「从右边缘往回量」的偏移
//!    （作者 `maxX-222` 宽 104、日期 `maxX-118` 宽 110，`GitGraphView.swift:565-577`）；
//!    `DataTable` 自带表头、且只接受固定像素列宽，无法一比一。
//!
//! 自绘行的两条硬约束（改这个文件时不要破坏）：
//! - 行高显式 `min_h(px(22.))`（[`COMMIT_ROW_HEIGHT`]）；
//! - 单元格内容的**自然高度必须 ≤ 22**：徽标 `h(px(16.))` 且不带 `border_1`、文字
//!   `text_size(px(11.))` / `px(12.5)`、不加纵向 `py`。否则内容会溢出到相邻行。
//!
//! ### 提交信息「字顶被切」的结论（本轮定案）
//!
//! **选方向 ②：去掉提交信息单元格的 `overflow_hidden`（只留 `text_ellipsis()`），
//! 并把该单元格的高度下限钉成 `min_h(px(22.))`**。`line_height(22)` 保留。
//!
//! 为什么：
//!
//! - gpui 的裁剪发生在**元素盒**上（`overflow_hidden` → 绘制时套 content mask），所以
//!   只要留着它，字形只要有一个像素画到盒外就会被切掉 —— 而字形画在哪里由
//!   `padding_top = (line_height - ascent - descent) / 2` 决定
//!   （`gpui-pre-0.3.6/src/text_system.rs:518-528`、`text_system/line.rs:547-548`）。
//!   去掉裁剪后，**任何**「盒高 / 行盒 / 字体度量」的出入都不会再切字顶。
//! - `min_h(px(22.))` 补掉「盒比行盒矮」这条路径：盒高与行盒同为 22 时，基线落在
//!   `padding_top + ascent`，字形整个在盒内；这也是「行盒 ≧ 字形」之外唯一还需要钉住的事。
//! - 方向 ①（把 `line_height` 给到 26–28）**不采用**：`min_h(22)` 的行会跟着内容长高到
//!   26–28，直接破坏 macOS 的 22pt 行高与 22 的行间距（`GitGraphGeometry.swift:8`）；
//!   若同时把行高写死成 `h(22)`，26 的行盒又会被 22 的盒裁掉，等于把问题搬回来。
//! - 水平方向不受影响：`text_ellipsis()` 是在**排版阶段**把文本截断成 `…` 的
//!   （`elements/text.rs:697-735` 把截断后的串交给 shaping），不依赖 `overflow_hidden`。
//!
//! ## 数据：Core 命令（本轮接线）
//!
//! `new` 时在后台跑一遍下面三条读命令；失败就**保留占位数据 + 显示一行提示**
//! （不空白、不 panic）。所有字段名都在源码里核对过，不猜：
//!
//! | 界面 | 命令 | 请求 payload | 响应（`data` 下） | 出处 |
//! |---|---|---|---|---|
//! | 引用树 | `git.references` | `{ root }` | `references[] { fullName, shortName, kind, isCurrent, upstreamShortName, ahead, behind }`、`recentReferences`、`userName`、`userEmail` | 请求 `rust/lithe-core/src/git/history.rs:49-51`；响应 `rust/lithe-core/src/protocol/contracts.rs:583-597 / :629-635`；种类判定 `git/mod.rs:5896-5902` |
//! | 提交列表 | `git.historyPage` | `{ root, order, limit }`（`order: "date"`、`limit: 100`） | `commits[] { hash, shortHash, parentHashes, authorName, authorEmail, date, subject, decorations }`、`nextCursor`、`hasMore` | 请求 `git/history.rs:56-71`；响应 `contracts.rs:602-611 / :640-649`；macOS 传 `order: "date"` + 页大小 100（`RustGitOperations.swift:594-601`、`GitFeatureModel.swift:298`）；`%D` 装饰与 `%ad` 日期格式 `git/history.rs:301-302` |
//! | 头部分支 / 仓库根 | `git.status` | `{ root }` | `repositoryRoot`（非仓库为 `null`）、`branch`、`ahead`、`behind`、`changes[]` | 请求 `git/mod.rs:94-96`；响应 `contracts.rs:518-538`；非仓库返回 `repositoryRoot: null` 见 `git/mod.rs:6535-6544` |
//! | 提交文件 | `git.commitFiles` | `{ root, commit }` | `files[] { status, path }` | 请求 `git/mod.rs:650-653`；响应 `contracts.rs:703-714`；实现 `git/mod.rs:2013-2038` |
//! | 关分页游标 | `git.historyCursorClose` | `{ root, cursor }` | `closed` | 请求 `git/history.rs:96-99`；契约要求放弃未读完的流时关掉游标（`shared/contracts/rust-core-api.md:894-895`） |
//!
//! 调用方式照 `shell_probe/files.rs:64-100`：拼 `{id, operationId, timeoutMilliseconds,
//! command, payload}` → `lithe_core::execute_json` → 判 `ok` → 取 `data`；整段放进
//! `cx.background_spawn(...)`，回前台写状态再 `cx.notify()`。
//!
//! 工作区根：本轮**不改 `BottomPanel::new` 的签名**（只允许改本文件，接线点是
//! `workspace.rs` 里的 `BottomPanel::new(window, cx)`，不归本步），用 [`workspace_root`]
//! 的 `std::env::current_dir()` 兜底。
//!
//! 真实可用的交互：页签切换（Console 选中后还能用 `xmark` 回到 Log）、引用行选中、
//! **提交行单击选中（加载该提交的文件 + 填写详情）**、提交栏刷新按钮、装饰开关
//! （macOS `showCommitDecorations`，`GitLogView.swift:1008-1012`）与搜索框
//! （引用栏的放大镜会清空它，但它本身只接收输入、不参与过滤）。
//!
//! ## 缺口（必须自研或后续步骤，本步未做）
//!
//! 1. **提交图泳道**（节点 / 箭头 / 虚线 lane）：gpui-kit 没有对应能力（规格 §5.6.1），
//!    需自定义 `Element` + `canvas`，移植 `GitGraphGeometry.swift:5-45` 与
//!    `GitGraphView.swift:871-917`。
//! 2. **并排 diff / 行内 diff**（规格 §5.6.2）：kit 里没有任何 diff 组件，是最大的一块。
//! 3. **提交行多选**（Cmd / Shift 区间 + 多选集合）：`TableState` / `ListState` 都只有单选
//!    （规格 §5.6.3）。连带未做：**悬停**背景（`toolHeader.opacity(0.55)`，`:555-558`；
//!    选中背景已做，见 [`commit_row`]）、120 ms 防抖（`:1243-1253`，本轮是立即加载）、
//!    ↑/↓ 键盘移动（`:1070-1079`）、行右键菜单（`:25-42`）、`Load more commits` 行
//!    （高 32，`:1045-1063`，本轮只取第一页并在 `hasMore` 时关游标）。
//! 4. **`primaryActionBar`**（高 38：Fetch / Fetch Options… / Compare │ Checkout / Cherry-pick
//!    + 右侧比较描述，`:526-594`）与提交栏的 `gitLogFilterBar`（Branch / User / Date / Path，
//!    `:1408-1546`）都没做。
//! 5. **栏宽的动态百分比上限**（引用栏 `avail*0.35`、详情栏 `avail*0.5`，`:2856-2880`）：
//!    需要按容器宽在 layout 回调里夹取，当前只声明了最小宽（规格 §5.1 也标为「需自行组合」）。
//! 6. **引用行右键菜单**（`:2733-2818`，完整项与顺序见规格 §3.4）与删除引用后的可恢复横幅
//!    （`:598-636`）；**选中引用后按该引用重查历史**（macOS `historyReference`，
//!    `GitFeatureModel.swift:1740-1746`）也没接，本轮只把引用树点亮。
//! 7. **提交文件树**：根标题行（仓库路径末两段 + `N files`，`:1671-1676`）、目录折叠
//!    （`:1171-1178`）、以及自绘 `NSView` 的全部命中逻辑（`GitCommitFileTreeView.swift:448-679`）；
//!    本轮已按真实 `status` + `path` 建出**多层目录树**（[`build_commit_files`]），
//!    状态字也按 macOS 的 `30 + max(depth-1,0)*16` 排（`GitCommitFileTreeView.swift:362`），
//!    但文件名的基线仍是「状态字 + 12」固定间距，没搬 macOS 由 `RowPresentation`
//!    按层级算出的 `textX`（`:364-368`）。
//! 8. **控制台**：`git.consolePresentation` 的折叠片段 / 匹配定位 / 清空 / 复制输出
//!    （`GitConsoleView.swift`）。
//! 9. **Worktrees 页签**：左列表默认 360、快速信息栏 282（`GitWorktreesView.swift:15-25 / :120-137`）。
//! 10. **滚动条**：提交栏与提交文件栏手铺 `v_flex()`（`uniform_list` 在这个容器里视口高为 0，
//!     见 [`BottomPanel::render_commit_list`]），没接 `ScrollableMask`，能滚但看不到滚动条
//!     （引用树由 `Tree` 自带）。
//! 11. **Git 专属字形**：默认图标集（`gpui-kit-assets-0.6.6/default-icons.txt` 的 101 个）没有
//!     分支 / cloud / tag 字形，替代关系写在 [`reference_icon`] 与 [`decoration_badge`] 旁。

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use gpui_kit::base::{
    ResizeHandleContext, ResizeHandleRenderer, TreeEntry, TreeItem, TreeState, h_resizable,
    resizable_panel, v_resizable,
};
use gpui_kit::component::button::Button;
use gpui_kit::component::dock::{BasePanel, Panel, PanelEvent};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tree::Tree;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    App, AppContext as _, Axis, Context, Div, Entity, EventEmitter, FocusHandle, Focusable,
    FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, WeakEntity, Window, div, px,
    uniform_list,
};

/// 工具窗标题栏高度。出处：`GitLogView.swift:456`。
const HEADER_HEIGHT: f32 = 32.;
/// 标题栏左 / 右内边距。出处：`GitLogView.swift:454-455`。
const HEADER_PADDING_LEADING: f32 = 12.;
const HEADER_PADDING_TRAILING: f32 = 7.;
/// 标题栏内间距（`HStack(spacing: 4)`）。出处：`GitLogView.swift:381`。
/// 引用栏工具行也是 `spacing: 4`（`:640`），所以两处共用。
const HEADER_GAP: f32 = 4.;
/// 页签高度 / 圆角 / 左右内边距 / 内容间距。出处：`GitLogView.swift:477 / :490-492 / :514`。
const TAB_HEIGHT: f32 = 27.;
const TAB_RADIUS: f32 = 5.;
const TAB_PADDING: f32 = 9.;
const TAB_GAP: f32 = 5.;
/// 页签在选中态（且是 Console）时的右内边距 —— 关闭按钮占位后收紧到 4。
/// 出处：`GitLogView.swift:491`。
const TAB_PADDING_WITH_CLOSE: f32 = 4.;
/// 页签关闭按钮宽 20 × 高 27。出处：`GitLogView.swift:502-505`。
const TAB_CLOSE_WIDTH: f32 = 20.;
/// 工具行统一高度（`GitVisual.toolbarHeight`）。出处：`GitLogView.swift:75`。
const TOOLBAR_HEIGHT: f32 = 38.;
/// 栏间分隔条厚度（`SplitHandleView.thickness`）。出处：`Views/Workbench/SplitHandleView.swift:13`。
const SPLIT_HANDLE_THICKNESS: f32 = 5.;
/// 三栏默认宽。出处：`GitLogView.swift:2886（引用 220）/ :2895（详情 350）`。
const REFERENCE_PANE_DEFAULT_WIDTH: f32 = 220.;
const DETAIL_PANE_DEFAULT_WIDTH: f32 = 350.;
/// 三栏最小宽（`GitLogThreePaneMetrics`）。出处：`GitLogView.swift:2833-2835`。
const REFERENCE_PANE_MIN_WIDTH: f32 = 180.;
const COMMIT_PANE_MIN_WIDTH: f32 = 340.;
const DETAIL_PANE_MIN_WIDTH: f32 = 250.;
/// 详情栏上下再分：文件栏最小高 90、提交详情默认高 156（= 容器高 − 5 − 文件栏）、最小高 110。
/// 出处：`GitLogView.swift:1096-1108`。
const COMMIT_FILES_MIN_HEIGHT: f32 = 90.;
const COMMIT_DETAIL_DEFAULT_HEIGHT: f32 = 156.;
const COMMIT_DETAIL_MIN_HEIGHT: f32 = 110.;
/// 提交行高（`GitGraphGeometry.rowHeight`，对齐 IntelliJ 原生 22pt）。出处：`GitGraphGeometry.swift:8`。
const COMMIT_ROW_HEIGHT: f32 = 22.;
/// 提交行行尾内边距。出处：`GitGraphView.swift:737`。
const COMMIT_ROW_TRAILING_PADDING: f32 = 8.;
/// 提交行作者列宽 104（左对齐）与日期列宽 110（右对齐）。出处：`GitGraphView.swift:565-577`。
const COMMIT_AUTHOR_WIDTH: f32 = 104.;
const COMMIT_DATE_WIDTH: f32 = 110.;
/// 提交表字号：正文 12.5 / 元数据 11.5 / 等宽元数据 11.5。出处：`GitGraphView.swift:657-660`。
const COMMIT_BODY_FONT_SIZE: f32 = 12.5;
const COMMIT_META_FONT_SIZE: f32 = 11.5;
/// 引用徽标：高 16、圆角 4、水平间距 5、上限宽 130、文字 11、文字左内边距 7。
/// 出处：`GitGraphView.swift:612-626`，文字字号 `:764`。
const DECORATION_BADGE_HEIGHT: f32 = 16.;
const DECORATION_BADGE_RADIUS: f32 = 4.;
const DECORATION_BADGE_GAP: f32 = 5.;
const DECORATION_BADGE_MAX_WIDTH: f32 = 130.;
const DECORATION_BADGE_FONT_SIZE: f32 = 11.;
const DECORATION_BADGE_LEADING_PADDING: f32 = 7.;
/// 引用树 / 引用分组行高（`GitVisual.treeRowHeight`）。出处：`GitLogView.swift:74`，
/// 行上的 `minHeight: 28` 在 `:2693 / :2722`。
const REFERENCE_ROW_HEIGHT: f32 = 28.;
/// 引用行缩进步长与左右内边距。出处：`GitLogView.swift:2691-2692 / :2720-2721`。
const REFERENCE_INDENT_STEP: f32 = 16.;
const REFERENCE_ROW_PADDING: f32 = 8.;
/// 引用行内间距（`HStack(spacing: 7)`）与图标槽宽（`frame(width: 16)`）。
/// 出处：`GitLogView.swift:2678-2681 / :2705-2708`。
const REFERENCE_GAP: f32 = 7.;
const REFERENCE_ICON_SLOT_WIDTH: f32 = 16.;
/// 引用栏工具行左右内边距 6。出处：`GitLogView.swift:659`。
const REFERENCE_TOOLBAR_PADDING: f32 = 6.;
/// 引用树容器的左右 8 / 上下 9 内边距。出处：`GitLogView.swift:691-692`。
const REFERENCE_TREE_PADDING_X: f32 = 8.;
const REFERENCE_TREE_PADDING_Y: f32 = 9.;
/// 提交栏工具行：左右内边距 10、行内间距 8、右侧按钮组内间距 2。
/// 出处：`GitLogView.swift:1024 / :965 / :997`。
const COMMIT_TOOLBAR_PADDING: f32 = 10.;
const COMMIT_TOOLBAR_GAP: f32 = 8.;
const COMMIT_TOOLBAR_BUTTON_GAP: f32 = 2.;
/// 提交栏搜索框 236 × 29、圆角 5、内边距 8。出处：`GitLogView.swift:966-991`。
const LOG_SEARCH_WIDTH: f32 = 236.;
const LOG_SEARCH_HEIGHT: f32 = 29.;
const LOG_SEARCH_RADIUS: f32 = 5.;
const LOG_SEARCH_PADDING: f32 = 8.;
/// 提交文件栏工具行内间距 5。出处：`GitLogView.swift:1120`。
const COMMIT_FILES_TOOLBAR_GAP: f32 = 5.;
/// 提交文件树行高 28。出处：`GitCommitFileTreeView.swift:120-121`。
const COMMIT_FILE_ROW_HEIGHT: f32 = 28.;
/// 提交文件行的状态字左起 30（macOS `x = 30`）。出处：`GitCommitFileTreeView.swift:362`。
const COMMIT_FILE_STATUS_X: f32 = 30.;
/// 提交详情内边距 11、行间距 9。出处：`GitLogView.swift:1190 / :1210`。
const COMMIT_DETAIL_PADDING: f32 = 11.;
const COMMIT_DETAIL_GAP: f32 = 9.;
/// `git.historyPage` 的页大小（macOS `GitFeatureModel.gitHistoryPageSize`）。出处：`GitFeatureModel.swift:298`。
const HISTORY_PAGE_LIMIT: usize = 100;
/// 调用 Core 时递增的 `operationId` 序列（每次调用取新值，见 [`execute_core`]）。
static NEXT_OPERATION_ID: AtomicUsize = AtomicUsize::new(1);
/// Git 读命令的超时（毫秒）。口径照 `shell_probe/files.rs:71` 的 `workspace.snapshot`（120 s），
/// 这里取 60 s：`git.historyPage` 只读第一页 100 条，正常远快于此。
const GIT_TIMEOUT_MILLIS: u32 = 60_000;
/// Core 读取失败 / 非仓库时那行提示的高度与字号（**自定，macOS 规格里没有这一行**）。
const NOTICE_HEIGHT: f32 = 20.;
const NOTICE_FONT_SIZE: f32 = 11.5;

/// 三栏 / 两栏之间的分隔条外观。
///
/// macOS 的分隔条是**可见 5pt 的实心条**（`Views/Workbench/SplitHandleView.swift:13`），
/// 而 gpui-kit 内建的分隔条只有 1px 线（`gpui-base/src/resizable/resize_handle.rs:12`
/// 的 `HANDLE_SIZE = px(1.)`）。用 `ResizablePanelGroup::with_handle_appearance`
/// （`gpui-base/src/resizable/panel.rs:59`）只换掉**画出来的部分**：
/// 命中区域、光标与拖拽仍由 kit 自己的 handle 负责。
fn split_handle_appearance() -> ResizeHandleRenderer {
    Rc::new(
        |handle: &ResizeHandleContext, _window: &mut Window, cx: &mut App| {
            let color = if handle.is_active() {
                cx.theme().primary
            } else {
                cx.theme().border
            };

            Some(match handle.axis() {
                // 横向 resizable = 竖着的分隔条。
                Axis::Horizontal => div()
                    .flex_none()
                    .w(px(SPLIT_HANDLE_THICKNESS))
                    .h_full()
                    .bg(color)
                    .into_any_element(),
                Axis::Vertical => div()
                    .flex_none()
                    .w_full()
                    .h(px(SPLIT_HANDLE_THICKNESS))
                    .bg(color)
                    .into_any_element(),
            })
        },
    )
}

/// 底部工具窗的页签。对应 macOS `GitToolTab`（`GitLogView.swift:81-85`）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomTab {
    /// `日志：<引用>`：引用栏 + 提交栏 + 提交文件栏（macOS `logTabContent`，`:297-335`）。
    Log,
    /// `工作树`（macOS `GitWorktreesView`，`:290-291`）。
    Worktrees,
    /// `控制台`（macOS `GitConsoleView`，`:292-293`）。
    Console,
}

/// 提交表的一行。
///
/// 字段对应 Core 的 `GitCommitResponse`（`rust/lithe-core/src/protocol/contracts.rs:602-611`，
/// UI 侧模型 `macos/Sources/LitheGitModule/Models/GitModels.swift:263-275`）：
/// `hash` / `shortHash` / `subject` / `authorName` / `authorEmail` / `date` / `decorations`。
/// 数据来自 `git.historyPage`，失败时退回 [`placeholder_commits`]。
#[derive(Clone)]
struct CommitRow {
    /// `commit.hash`：单击选中后拿它去查 `git.commitFiles`。
    hash: SharedString,
    /// `commit.shortHash`：提交详情里的短 hash。
    short_hash: SharedString,
    /// 引用装饰（`commit.decorations`，`%D` 字符串已按 [`parse_decorations`] 拆成徽标），
    /// **从右往左**堆叠渲染。
    decorations: Vec<Decoration>,
    /// 提交说明（`commit.subject`）。
    subject: SharedString,
    /// 作者（`commit.authorName`）。
    author: SharedString,
    /// 作者邮箱（`commit.authorEmail`，提交详情用）。
    author_email: SharedString,
    /// 日期（`commit.date`，Core 用 `--date=format:%Y/%m/%d %H:%M` 格式化好，UI 不再加工：
    /// `rust/lithe-core/src/git/history.rs:301`）。
    date: SharedString,
}

/// 提交文件树的一行。
///
/// 度量出处：`GitCommitFileTreeView.swift:120-121`（行高 28）、`:338 / :362 / :441`（缩进与状态字）。
/// 行数据来自 `git.commitFiles` 的 `files[] { status, path }`
/// （`contracts.rs:703-714`），由 [`build_commit_files`] 按路径聚成目录树；
/// 失败时退回 [`placeholder_commit_files`]。
struct CommitFileRow {
    /// 层级：根下的目录 / 文件为 0，逐层 +1。缩进步长 16。
    depth: usize,
    /// 名称（目录名或文件名，已按 `/` 拆开，只留自己那一段）。
    name: SharedString,
    /// 目录行：画文件夹图标 + 右侧 `N 个文件`。
    is_folder: bool,
    /// 文件行的状态字（Core 的 `status`，如 `M` / `A` / `D` / `R100`），目录行忽略。
    status: char,
    /// 目录行的文件数（含子目录里的文件，递归计数）。
    file_count: usize,
}

/// 提交详情（详情栏的下半部分）。出处：`GitLogView.swift:1187-1220`。
/// 内容来自选中的那一行 [`CommitRow`]（macOS `feature.selectedGitCommit`，
/// `GitFeatureModel.swift:1776-1782`），没有选中时是 [`placeholder_commit_detail`]。
struct CommitDetail {
    subject: SharedString,
    short_hash: SharedString,
    author_name: SharedString,
    author_email: SharedString,
    date: SharedString,
    decorations: SharedString,
}

/// 一条引用（Core `GitReferenceResponse`，`contracts.rs:583-597`）。
///
/// 引用树的输入；分组顺序与文案照 macOS（`GitLogView.swift:664-697`）。
struct RefEntry {
    /// `fullName`（`refs/heads/main` / `refs/remotes/origin/main` / `refs/tags/v1.0.0`）：
    /// 引用节点的稳定 id 与后续按引用查历史时的入参。
    full_name: String,
    /// `shortName`：引用行的标题（macOS `GitLogView.swift:2456`）。
    short_name: String,
    /// `kind`：`local` / `remote` / `tag`（`git/mod.rs:5896-5902`）。
    kind: String,
    /// `isCurrent`：当前分支（绿勾 + 空选中时点亮，`GitLogView.swift:758-764 / :2713-2717`）。
    is_current: bool,
    /// `upstreamShortName`：有 upstream 才画 ahead/behind。
    upstream_short_name: Option<String>,
    /// `ahead` / `behind`：相对 upstream 的领先 / 落后提交数。
    ahead: usize,
    behind: usize,
}

/// 提交文件树的中间节点：按路径把 `files[]` 聚成多层目录。
///
/// macOS 侧是 `GitCommitFileTreeNode.build(from:rootName:)`（`GitModels.swift:285-323`），
/// 这里只保留建树需要的部分：子目录（`BTreeMap` 保证同名目录只有一份、顺序稳定）
/// 与自己的文件列表。
#[derive(Default)]
struct CommitFileNode {
    dirs: std::collections::BTreeMap<String, CommitFileNode>,
    /// `(文件名, 状态字)`。
    files: Vec<(String, String)>,
}

impl CommitFileNode {
    /// 把一条仓库相对路径插进树：中间的目录按需建节点，最后一段是文件名。
    fn insert(&mut self, path: &str, status: &str) {
        let segments: Vec<&str> = path.split('/').collect();
        let Some((name, directories)) = segments.split_last() else {
            return;
        };

        let mut node = self;
        for directory in directories {
            node = node.dirs.entry((*directory).to_string()).or_default();
        }
        node.files.push(((*name).to_string(), status.to_string()));
    }

    /// 该节点下的文件总数（含子目录，递归）。
    fn file_count(&self) -> usize {
        self.files.len() + self.dirs.values().map(CommitFileNode::file_count).sum::<usize>()
    }

    /// 深度优先摊平成行：目录在前、文件在后，`depth` 从 0 起。
    ///
    /// macOS 的行序来自 `GitCommitFileTreeNode.build`（`GitModels.swift:285-323`），
    /// 同样是「目录 → 子目录 → 文件」的稳定顺序。
    fn flatten(&self, depth: usize, rows: &mut Vec<CommitFileRow>) {
        for (name, child) in &self.dirs {
            rows.push(CommitFileRow {
                depth,
                name: SharedString::from(name.clone()),
                is_folder: true,
                status: ' ',
                file_count: child.file_count(),
            });
            child.flatten(depth + 1, rows);
        }

        for (name, status) in &self.files {
            rows.push(CommitFileRow {
                depth,
                name: SharedString::from(name.clone()),
                is_folder: false,
                // Core 的 `status` 是 name-status 码（可能是 `R100` 这样的多字符），
                // macOS 的 `statusColor` 只看首字母（`GitCommitFileTreeView.swift:459-464`）。
                status: status.chars().next().unwrap_or(' '),
                file_count: 0,
            });
        }
    }
}

/// 提交文件栏的四种状态（macOS `commitFilesPane` 的空 / 加载 / 失败三态 + 本轮的占位态，
/// `GitLogView.swift:1118-1185`）。
enum CommitFilesState {
    /// 还没选中提交（或历史没接上）：画占位文件 + 未接入提示。
    Placeholder,
    /// 正在加载，文案 `正在加载更改的文件…`（zh-Hans `Localizable.strings:1748`）。
    Loading,
    /// 已加载（可能是 0 个文件）。
    Loaded,
    /// 加载失败，文案 `无法加载更改的文件`（`Localizable.strings:1749`），文件列表退回占位。
    Failed,
}

/// 一次 Core 读取的结果：`git.status` + `git.references` + `git.historyPage` 第一页。
///
/// 三个命令在**同一个后台任务**里顺序跑（都要用同一个 `root`，且引用与历史在
/// 真机上也几乎是同时刷新的：`GitFeatureModel.swift:1736-1750`）。它必须能跨线程
/// 送回前台，所以只装 `String` / `SharedString` 这些 `Send` 数据 ——
/// **不能装 `TreeItem`**：`TreeItem` 内部有 `Rc<RefCell<..>>`
/// （`gpui-base-0.6.6/src/tree.rs:41-46`），不是 `Send`，引用树因此在
/// `apply_snapshot`（前台）里用 [`build_reference_tree`] 现拼。
struct GitSnapshot {
    /// `git.status.repositoryRoot` 是否存在 → 是否 Git 仓库（非仓库时 Core 返回 `null`，
    /// `git/mod.rs:6535-6544`）。
    is_repository: bool,
    /// `git.status.branch`：标题栏 `日志：<当前分支>` 的兜底（`GitLogView.swift:396`）。
    branch: Option<SharedString>,
    /// 解析成绝对路径的仓库根：`git.commitFiles` 用它当 `root`（`relative_or_absolute`
    /// 可能给出相对路径，`git/mod.rs:6574`）。
    repository_root: Option<String>,
    /// `git.references` 的结果（还没拼成树）。
    references: Option<Vec<RefEntry>>,
    /// `git.historyPage` 第一页的提交行。
    commits: Option<Vec<CommitRow>>,
    /// 失败的命令与错误码，拼成提示行；空表示全部成功。
    failures: Vec<String>,
}

/// 引用装饰的种类。macOS 用它决定徽标强调色（`GitGraphView.swift:775-782`）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum DecorationKind {
    /// `.head` → `LitheTheme.accent`。
    Head,
    /// `.branch` → `LitheTheme.success`。
    Local,
    /// `.remote` → `rgb(0.55,0.70,0.96)`。
    Remote,
    /// `.tag` → `LitheTheme.warning`。
    Tag,
}

/// 一个引用装饰（提交行右侧的徽标）。
///
/// `kind` 在真机上来自 `GitGraphView.swift:775-782` 的 `kind` 枚举；这里由
/// [`parse_decorations`] 从 Core 的 `%D` 字符串（`commit.decorations` 字段）判定并**显式带上**，
/// 不再靠名字形状猜（占位数据才走 [`decoration_kind`]）。
#[derive(Clone)]
struct Decoration {
    name: SharedString,
    kind: DecorationKind,
}

/// 装饰名的种类判定（**只有占位数据用**）。
///
/// 真机上每种装饰有明确的种类（`GitGraphView.swift:775-782`），真数据的种类由
/// [`parse_decorations`] 按 `%D` 的写法判定（`HEAD -> x` / `tag: x` / 含 `/` 的是远端）；
/// 这个函数只服务于 [`placeholder_commits`] 的假装饰名。
fn decoration_kind(name: &str) -> DecorationKind {
    if name == "HEAD" {
        DecorationKind::Head
    } else if name.contains('/') {
        DecorationKind::Remote
    } else if name.starts_with('v') && name[1..].starts_with(|c: char| c.is_ascii_digit()) {
        DecorationKind::Tag
    } else {
        DecorationKind::Local
    }
}

/// 提交行里的引用徽标。
///
/// 度量按 macOS 自绘版（`GitGraphView.swift:612-626`）：高 16、圆角 4、上限宽
/// `min(130, max(28, 文字宽 + 14))`、文字 11、文字左内边距 7、背景 `toolHeader`、
/// 文字 `primaryText`。
///
/// 两处刻意偏差（已登记）：
/// - **宽度不做测量**：macOS 用文字宽度算 `min(130, max(28, 宽 + 14))`，gpui 侧拿不到
///   文字测量值，这里用「`pl(7)` + `max_w(130)`」等价近似。
/// - **种类色落在文字上**：macOS 的种类强调色画在徽标里的 12×12 图标上（`:761-762`），
///   而默认图标集没有 Git 字形（见模块注释缺口 11），所以把种类色落到文字，
///   否则 head / branch / remote / tag 四类引用在视觉上无法区分。
fn decoration_badge(decoration: &Decoration, cx: &App) -> impl IntoElement {
    // 种类 → 强调色。remote 的真机值是硬编码 `rgb(0.55,0.70,0.96)`（`GitGraphView.swift:779`），
    // 规格要求颜色一律走 `cx.theme()`，这里取最接近的主题 token `blue_light`。
    let foreground = match decoration.kind {
        DecorationKind::Head => cx.theme().primary,
        DecorationKind::Local => cx.theme().success,
        DecorationKind::Remote => cx.theme().blue_light,
        DecorationKind::Tag => cx.theme().warning,
    };

    h_flex()
        .h(px(DECORATION_BADGE_HEIGHT))
        .max_w(px(DECORATION_BADGE_MAX_WIDTH))
        .pl(px(DECORATION_BADGE_LEADING_PADDING))
        .pr(px(3.))
        .rounded(px(DECORATION_BADGE_RADIUS))
        // macOS 自绘版徽标底色是 `toolHeader`（`:614-620`），等价于标题栏底色。
        .bg(cx.theme().tab_bar)
        .text_size(px(DECORATION_BADGE_FONT_SIZE))
        .text_color(foreground)
        .whitespace_nowrap()
        .overflow_hidden()
        .child(decoration.name.clone())
}

/// 提交表的一行（**自绘**，理由见模块注释）。
///
/// 列布局对齐 macOS 自绘版（`GitGraphView.swift:541-584`）：
/// `[提交图（未做）][提交信息 弹性][引用徽标 从右往左][间隙 8][作者 104 左对齐][日期 110 右对齐]`，
/// 行尾再留 8pt（`:737`）；行底部 1px `divider`（`:581-582`）。
///
/// `selected` 是选中背景：macOS 是整行 `accent.withAlphaComponent(0.16)`
/// （`GitGraphView.swift:552-554`），这里用主题 `primary` 压低透明度近似
/// （`accent` 在 gpui 里是悬停底色，见 `gpui/UI-MAP.md` §1.2 的 token 映射）。
/// 行的 `id` 与点击由 [`BottomPanel::render_commit_list`] 挂（那里拿得到 `cx`）。
fn commit_row(row: &CommitRow, show_decorations: bool, selected: bool, cx: &App) -> Div {
    let mut element = h_flex()
        // `min_h` 而不是 `h`：macOS 的行高是 22pt（`GitGraphGeometry.swift:8`），
        // 但内容（徽标 16 + 文字行盒）自然高度可能略高，写死高度就会裁内容。
        // `min_h(22)` 让它至少 22、内容更高时自然撑开，先保证不裁字。
        .min_h(px(COMMIT_ROW_HEIGHT))
        .w_full()
        .min_w_0()
        .items_center()
        .pr(px(COMMIT_ROW_TRAILING_PADDING))
        .whitespace_nowrap()
        .border_b_1()
        .border_color(cx.theme().border)
        // 提交信息列：左对齐、尾部截断。macOS 的宽度是 `rect.width - textStart - 230`，
        // 这里让它吃掉「提交图 + 徽标 + 作者 + 日期」之外的全部剩余宽度 —— 提交图未做，
        // 所以当前就等于全部剩余宽度（缺口 1）。
        .child(
            div()
                .flex_1()
                .min_w_0()
                // ⚠️ **不要在这里加 `overflow_hidden`**（本轮定案，理由见模块注释
                // 「提交信息『字顶被切』的结论」）：gpui 的裁剪套在元素盒上，只要盒与
                // 字形度量有一点出入，字顶就会被切掉约 1/4 行高
                // （实测证据 `.artifacts/p1/zoom-commit-final.png`）。
                // 水平方向不需要它：`text_ellipsis()` 在排版阶段就把文本截断成 `…`
                // （`gpui-pre-0.3.6/src/elements/text.rs:697-735`）。
                .text_ellipsis()
                // 行盒下限钉成行高：盒高 22 = 行盒 22 时，基线落在
                // `padding_top + ascent`、字形整个在盒内；盒若被压到字体自然行高
                // （约 1.3em）就会把字形往盒外推。
                .min_h(px(COMMIT_ROW_HEIGHT))
                .text_size(px(COMMIT_BODY_FONT_SIZE))
                // 显式给行高：gpui 用 `padding_top = (line_height - ascent - descent) / 2`
                // 算基线（`src/text_system.rs:518-528`），行盒 22 时 12.5pt 的字形在
                // 22pt 的行里垂直居中；不给就落到默认 `phi() = 1.618 × 字号`
                // （`src/geometry.rs:3722`、`src/style.rs:485-494`），与行高对不上。
                .line_height(px(COMMIT_ROW_HEIGHT))
                .child(row.subject.clone()),
        );

    // 选中背景：macOS 是整行 `accent.withAlphaComponent(0.16)`（`GitGraphView.swift:552-554`），
    // 这里用主题 `primary` 压低透明度近似（`accent` 在 gpui 主题里是悬停底色，
    // 见 `gpui/UI-MAP.md` §1.2 的 token 映射）。未选中时不画底色。
    if selected {
        element = element.bg(cx.theme().primary.opacity(0.16));
    }

    // 引用徽标：macOS 从 `maxX - 230` 起**从右向左**排布（`GitGraphView.swift:578-580`），
    // 即数组第 0 项在最右边；所以倒序渲染，再补 8pt 间隙接作者列（`:565` 的 `maxX-222`）。
    // 开关本身是 macOS 的 `showCommitDecorations`（`GitLogView.swift:1008-1012`）。
    if show_decorations {
        element = element.child(
            h_flex()
                .flex_shrink_0()
                .gap(px(DECORATION_BADGE_GAP))
                .pr(px(COMMIT_ROW_TRAILING_PADDING))
                .children(
                    row.decorations
                        .iter()
                        .rev()
                        .map(|decoration| decoration_badge(decoration, cx)),
                ),
        );
    }

    element
        // 作者列：宽 104、左对齐、`secondaryText`（`GitGraphView.swift:565-570`）。
        .child(
            div()
                .w(px(COMMIT_AUTHOR_WIDTH))
                .flex_shrink_0()
                .overflow_hidden()
                .text_ellipsis()
                .text_size(px(COMMIT_META_FONT_SIZE))
                .text_color(cx.theme().muted_foreground)
                .child(row.author.clone()),
        )
        // 日期列：宽 110、右对齐、等宽 11.5（`GitGraphView.swift:571-577 / :635-640`）。
        .child(
            h_flex()
                .w(px(COMMIT_DATE_WIDTH))
                .flex_shrink_0()
                .justify_end()
                .overflow_hidden()
                .text_ellipsis()
                .font_family(cx.theme().mono_font_family.clone())
                .text_size(px(COMMIT_META_FONT_SIZE))
                .text_color(cx.theme().muted_foreground)
                .child(row.date.clone()),
        )
}

/// 提交文件树的一行。
///
/// macOS 度量（`GitCommitFileTreeView.swift`）：行高 28、目录行缩进 `8 + depth*16`、
/// 文件行状态字左起 `30 + max(depth-1,0)*16`（`:362`）、状态字等宽 11 bold、正文 13；
/// 状态色 `A`→success、`D`→error、`R`→accent、其余→warning（`:459-464`）。
///
/// **偏差**：目录行只按 `8 + depth*16` 排、文件行名字紧随状态字，没有搬 macOS 的
/// `presentation.textX`（`:364-368`，由 `RowPresentation` 按层级算出的名字基线），
/// 因为 `GitCommitFileTreeView` 是 498 行的自绘 NSView，完整对齐属于缺口 7。
fn commit_file_row(row: &CommitFileRow, cx: &App) -> Div {
    let status_color = match row.status {
        'A' => cx.theme().success,
        'D' => cx.theme().danger,
        'R' => cx.theme().primary,
        _ => cx.theme().warning,
    };

    let mut element = h_flex()
        .h(px(COMMIT_FILE_ROW_HEIGHT))
        .w_full()
        .items_center()
        .pr(px(REFERENCE_ROW_PADDING))
        .whitespace_nowrap()
        .text_ellipsis();

    if row.is_folder {
        element = element
            .gap(px(REFERENCE_GAP))
            .pl(px(REFERENCE_ROW_PADDING + row.depth as f32 * REFERENCE_INDENT_STEP))
            .child(
                Icon::new(IconName::FolderOpen)
                    .size(px(14.))
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(row.name.clone()),
            )
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child(SharedString::from(format!("{} 个文件", row.file_count))),
            );
    } else {
        element = element
            // 文件行状态字左起 `30 + max(depth-1,0)*16`：macOS `GitCommitFileTreeView.swift:362`。
            .pl(px(
                COMMIT_FILE_STATUS_X + row.depth.saturating_sub(1) as f32 * REFERENCE_INDENT_STEP
            ))
            .child(
                div()
                    .text_size(px(11.))
                    .font_weight(FontWeight::BOLD)
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(status_color)
                    .child(SharedString::from(row.status.to_string())),
            )
            .child(div().w(px(12.)).flex_shrink_0())
            .child(div().text_size(px(13.)).child(row.name.clone()))
            .child(div().flex_1());
    }

    element
}

/// 引用行的种类：从树节点 id 反解（本文件自己拼的 id，形状固定为
/// `ref:local:<fullName>` / `ref:remote:<fullName>` / `ref:tag:<fullName>` /
/// `ref:group:<kind>` / `ref:HEAD`，见 [`build_reference_tree`] 与 [`placeholder_references`]）。
///
/// 种类本身来自 Core 的 `kind` 字段（`local` / `remote` / `tag`，
/// `rust/lithe-core/src/git/mod.rs:5896-5902`），**不再靠名字里是否含 `tag` 猜**。
fn reference_kind(id: &str) -> &str {
    let rest = id.strip_prefix("ref:").unwrap_or(id);
    let rest = rest.strip_prefix("group:").unwrap_or(rest);
    match rest.split_once(':') {
        Some((kind, _)) => kind,
        None => "head",
    }
}

/// 引用行 / 引用分组行的图标。
///
/// macOS 用 SF Symbols（`GitLogView.swift:2679-2686 / :2706 / :2821-2827`）：
/// 分组 `folder` / `network` / `tag`，引用 `point.3.connected.trianglepath.dotted` / `cloud` / `tag`，
/// `HEAD (Current Branch)` 行是 `arrow.right`（`:668`，本文件用 `ref:HEAD` 这个 id 表示）。
/// `gpui_kit::component::IconName`（0.6.6）只有默认图标集里的 101 个字形
/// （`gpui-kit-assets-0.6.6/default-icons.txt`），没有 Git 专属字形，对应关系：
///
/// | macOS | 这里 | 说明 |
/// |---|---|---|
/// | `folder`（本地分组） | `Folder` | 一致 |
/// | `network`（远端分组） | `Network` | 一致 |
/// | `tag`（标签分组 / 标签引用） | `Star` | 默认集没有 tag 字形 |
/// | `point.3.connected.trianglepath.dotted`（本地分支） | `ArrowRight` | 默认集没有分支字形 |
/// | `cloud`（远端引用） | `Globe` | 默认集没有 cloud 字形 |
/// | `arrow.right`（HEAD 行） | `ArrowRight` | 一致 |
///
/// `kind` 取 [`reference_kind`] 的结果：`local` / `remote` / `tag` / `HEAD` 行是 `head`。
fn reference_icon(kind: &str, is_group: bool) -> IconName {
    match (is_group, kind) {
        (_, "tag") => IconName::Star,
        (true, "remote") => IconName::Network,
        (true, _) => IconName::Folder,
        (false, "remote") => IconName::Globe,
        // 本地分支与 `HEAD（当前分支）` 行：默认集没有分支字形，统一用 `ArrowRight`。
        (false, _) => IconName::ArrowRight,
    }
}

/// 某个节点是不是「当前分支」：`HEAD（当前分支）` 行与当前分支那一行都算
/// （macOS 两行用的是同一份判定，`GitLogView.swift:758-764 / :839`）。
fn is_current_reference(id: &str, current: Option<&SharedString>) -> bool {
    id == "ref:HEAD" || current.is_some_and(|current| current.as_ref() == id)
}

/// 引用树的一行（分组行与引用行**同高**）。
///
/// macOS：分组行与引用行都是 `minHeight: 28`、缩进 `depth * 16`、left 8 / right 8、
/// 圆角 4、内间距 7、图标槽 16（`GitLogView.swift:2674-2732`）；
/// 分组标题 13 medium，引用名 13，当前分支多一个 9 bold 的 `checkmark`（`:2713-2717`）；
/// `HEAD（当前分支）` 行是同一度量的另一份实现（`:818-833`）。
/// 行高统一取 `GitVisual.treeRowHeight = 28`（`:74`）。
///
/// `Tree` 内部是 `uniform_list`（`gpui-base/src/tree.rs:423`），一套树只能有一个行高；
/// macOS 的行间距（组内 1 / 组间 2，`GitLogView.swift:666 / :711`）在这里无法表达，
/// 行与行贴合 —— 偏差已登记在模块注释。
///
/// `highlight_current`：macOS 在「没有显式选中引用」时把当前分支行点亮
/// （`isReferenceRowSelected`，`:758-764`）；`current` 是当前分支节点的 id（`None` = detached）。
fn reference_row(
    index: usize,
    entry: &TreeEntry,
    selected: bool,
    highlight_current: bool,
    current: Option<&SharedString>,
    cx: &mut App,
) -> ListItem {
    let id = entry.item().id.as_ref();
    let is_group = entry.is_folder();
    // 种类来自本文件拼的 id（真数据的种类来自 Core 的 `GitReference.kind`，
    // `rust/lithe-core/src/git/mod.rs:5896-5902`）。
    let kind = reference_kind(id);
    let is_tag = kind == "tag";
    let is_current_branch = is_current_reference(id, current);
    let active = selected || (highlight_current && is_current_branch);

    let icon_color = if is_tag {
        cx.theme().warning
    } else {
        cx.theme().muted_foreground
    };

    // 分组行：chevron（8 bold，占 10 宽）+ 图标 + 13 medium 标题。
    // 引用行：16 宽的图标槽 + 13 标题 + （当前分支的）9 bold checkmark。
    let row = ListItem::new(SharedString::from(format!("ref-node-{index}")))
        .selected(active)
        .h(px(REFERENCE_ROW_HEIGHT))
        // `ListItem` 默认 `py_1() px_3()`（`list_item.rs:185-188`），紧凑行必须清掉，
        // 否则自然高度 32 > 28，会溢出到相邻行。
        .py_0()
        .px_0()
        .pl(px(entry.depth() as f32 * REFERENCE_INDENT_STEP))
        .pr(px(REFERENCE_ROW_PADDING))
        .rounded(px(4.))
        .text_size(px(13.))
        .whitespace_nowrap()
        .overflow_hidden();

    // **必须自己套一层 `h_flex()`**：`ListItem` 内部装 children 的是普通块级 `div()`，
    // 多个 child 直接挂上去会竖着排（图标 / 名字 / 对勾各占一行），行高随之失控。
    let mut content = h_flex().w_full().items_center().gap(px(REFERENCE_GAP));

    if is_group {
        content = content.child(
            Icon::new(if entry.is_expanded() {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            })
            .size(px(8.))
            .text_color(cx.theme().muted_foreground),
        );
    }

    content = content.child(
        h_flex()
            .w(px(REFERENCE_ICON_SLOT_WIDTH))
            .flex_shrink_0()
            .justify_center()
            .child(Icon::new(reference_icon(kind, is_group)).size(px(14.)).text_color(icon_color)),
    );

    if is_group {
        content = content.child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .child(entry.item().label.clone()),
        );
    } else {
        content = content.child(entry.item().label.clone());
    }

    if is_current_branch {
        content = content.child(
            Icon::new(IconName::Check)
                .size(px(9.))
                .text_color(cx.theme().primary),
        );
    }

    // macOS 的 `Spacer(minLength: 8)`：把剩余宽度吃掉，保证标题左对齐。
    content = content.child(div().flex_1());

    row.child(content)
}

/// 占位引用树：`git.references` 没接上（命令失败 / 不是 Git 仓库）时画它。
///
/// 结构照 macOS（`GitLogView.swift:664-697`）：`HEAD（当前分支）` 单独一行，下面是
/// `本地 / 远程 / 标签` 三个分组，分组内的引用缩进一级。
///
/// 节点 id 必须与 [`build_reference_tree`] 拼出来的形状一致（`ref:<kind>:<name>` /
/// `ref:group:<kind>` / `ref:HEAD`），否则 [`reference_icon`] / [`is_current_reference`]
/// 认不出种类与当前分支。
///
/// 标签分组放一个 `v1.0.0`：`TreeItem::is_folder()` 以「有子项」判定分组，
/// 空分组会退化成引用行；macOS 的空分组就是不画任何行（`:750-756`）。
fn placeholder_references() -> Vec<TreeItem> {
    vec![
        TreeItem::new("ref:HEAD", "HEAD（当前分支）"),
        TreeItem::new("ref:group:local", "本地")
            .expanded(true)
            .child(TreeItem::new("ref:local:refs/heads/main", "main")),
        TreeItem::new("ref:group:remote", "远程")
            .expanded(true)
            .child(TreeItem::new(
                "ref:remote:refs/remotes/origin/main",
                "origin/main",
            ))
            .child(TreeItem::new(
                "ref:remote:refs/remotes/origin/HEAD",
                "origin/HEAD",
            )),
        TreeItem::new("ref:group:tag", "标签")
            .expanded(true)
            .child(TreeItem::new("ref:tag:refs/tags/v1.0.0", "v1.0.0")),
    ]
}

/// 占位提交行：`git.historyPage` 没接上时画它（真实数据由 [`parse_commits`] 填）：
///
/// - `hash` / `short_hash` 是**占位 hash**，点这些行时 `git.commitFiles` 会失败，
///   面板会退回占位文件列表并显示失败提示（不 panic）；
/// - 日期字符串格式照 Core 的 `--date=format:%Y/%m/%d %H:%M`
///   （`rust/lithe-core/src/git/history.rs:301`），免得接线前后列宽表现不一致
///   （日期列宽 110，出处 `GitGraphView.swift:571-577`）。
fn placeholder_commits() -> Vec<CommitRow> {
    let row = |decorations: &[&str], subject: &str, author: &str, date: &str| CommitRow {
        hash: SharedString::from(format!("placeholder-{}", subject.len())),
        short_hash: SharedString::from("0000000"),
        decorations: decorations
            .iter()
            .map(|name| Decoration {
                name: SharedString::from(*name),
                kind: decoration_kind(name),
            })
            .collect(),
        subject: SharedString::from(subject),
        author: SharedString::from(author),
        author_email: SharedString::from(format!("{author}@lithe.dev")),
        date: SharedString::from(date),
    };

    vec![
        row(
            &["HEAD", "main", "origin/main", "v1.0.0"],
            "门急诊就诊记录查询接口",
            "fuchen",
            "2026/09/20 19:08",
        ),
        row(&[], "优化消息通知", "fuchen", "2026/09/19 16:42"),
        row(&[], "接口日志补充耗时字段", "fuchen", "2026/09/18 11:05"),
        row(&[], "HTTP 服务超时改成可配置", "lizhen", "2026/09/17 09:31"),
        row(
            &["origin/fix/order"],
            "修正订单查询空指针",
            "lizhen",
            "2026/09/16 20:14",
        ),
        row(&[], "同步数据库脚本", "wangqi", "2026/09/15 15:02"),
    ]
}

/// 占位提交文件树：`git.commitFiles` 没接上（或失败）时画它。
///
/// 真实数据由 [`load_commit_files`] + [`build_commit_files`] 生成：用 Core 的
/// `files[] { status, path }`（`contracts.rs:703-714`）按路径建多层目录树
/// （macOS 是 `GitCommitFileTreeNode.build(from:rootName:)`，`GitModels.swift:285-323`）。
fn placeholder_commit_files() -> Vec<CommitFileRow> {
    let folder = |depth: usize, name: &str, file_count: usize| CommitFileRow {
        depth,
        name: SharedString::from(name),
        is_folder: true,
        status: ' ',
        file_count,
    };
    let file = |depth: usize, name: &str, status: char| CommitFileRow {
        depth,
        name: SharedString::from(name),
        is_folder: false,
        status,
        file_count: 0,
    };

    vec![
        folder(0, "src", 3),
        file(1, "main.rs", 'M'),
        file(1, "protocol.rs", 'A'),
        file(1, "settings.rs", 'M'),
        file(0, "README.md", 'D'),
    ]
}

/// 占位提交详情：还没选中提交（或历史没接上）时画它。
///
/// 真数据由 [`BottomPanel::select_commit`] 从选中的 [`CommitRow`] 填
/// （macOS `feature.selectedGitCommit`，`GitFeatureModel.swift:1776-1782`；
/// 版面 `GitLogView.swift:1187-1220`）。
fn placeholder_commit_detail() -> CommitDetail {
    CommitDetail {
        subject: SharedString::from("门急诊就诊记录查询接口"),
        short_hash: SharedString::from("0000000"),
        author_name: SharedString::from("fuchen"),
        author_email: SharedString::from("fuchen@lithe.dev"),
        date: SharedString::from("2026/09/20 19:08"),
        decorations: SharedString::from("HEAD -> main, origin/main, tag: v1.0.0"),
    }
}

/// 面板要用的工作区根由 `workspace.rs` 通过 [`BottomPanel::new`] 的参数传入，
/// 不再用 `std::env::current_dir()` 兜底（那会随启动目录漂移）。
///
/// 语义与 `workspace.snapshot` 的 `root` 一致：可以是仓库里的子目录，
/// `git.*` 命令会自己解析出真正的仓库根（`git.status` 的 `repositoryRoot`，
/// 见 [`load_git_snapshot`]）。

/// 执行一条 Core 命令并返回响应里的 `data`。
///
/// 请求信封照 `shell_probe/files.rs:68-75`：`{id, operationId, timeoutMilliseconds,
/// command, payload}`；响应是 `CoreResponse`（`rust/lithe-core/src/protocol/contracts.rs:7-21`）：
/// `ok: bool` + 成功时 `data` / 失败时 `error { code, … }`。
///
/// `operationId` 每次调用都取新值（[`NEXT_OPERATION_ID`]）：契约要求按
/// 「repository / reference / 归属的 operationId」判过期页并关掉
/// （`shared/contracts/rust-core-api.md:894-897`），所以刷新时不能沿用上一次的 id。
///
/// 返回 `Err(code)` 时只带 Core 的错误码（不是完整消息）：面板只把它拼进一行提示，
/// 不弹窗、不 panic。
fn execute_core(command: &str, payload: serde_json::Value) -> Result<serde_json::Value, String> {
    let call = NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed);
    let operation_id = format!("shell-probe-{command}-{call}");
    let request = serde_json::json!({
        "id": operation_id,
        "operationId": operation_id,
        "timeoutMilliseconds": GIT_TIMEOUT_MILLIS,
        "command": command,
        "payload": payload,
    })
    .to_string();

    let raw = lithe_core::execute_json(&request);
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|error| format!("响应不是合法 JSON：{error}"))?;

    if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        let code = value
            .pointer("/error/code")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        return Err(code.to_string());
    }

    value
        .get("data")
        .cloned()
        .ok_or_else(|| "响应缺少 data".to_string())
}

/// 把 `git.status.repositoryRoot` 变成可用的绝对根。
///
/// Core 给的是 `relative_or_absolute(&repository_root, &root)`
/// （`rust/lithe-core/src/git/mod.rs:6574`）：仓库根就在工作区里时会返回相对路径，
/// 直接拿去当 `git.commitFiles` 的 `root` 会按**进程工作目录**解析，可能指错地方。
fn resolve_repository_root(root: &Path, repository_root: &str) -> String {
    let path = Path::new(repository_root);
    if path.is_absolute() {
        repository_root.to_string()
    } else {
        root.join(path).to_string_lossy().to_string()
    }
}

/// 解析 `git.references` 的 `references[]`（`contracts.rs:583-597`）。
fn parse_references(data: &serde_json::Value) -> Vec<RefEntry> {
    data.get("references")
        .and_then(serde_json::Value::as_array)
        .map(|references| {
            references
                .iter()
                .filter_map(|reference| {
                    Some(RefEntry {
                        full_name: reference.get("fullName")?.as_str()?.to_string(),
                        short_name: reference.get("shortName")?.as_str()?.to_string(),
                        kind: reference.get("kind")?.as_str()?.to_string(),
                        is_current: reference
                            .get("isCurrent")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false),
                        upstream_short_name: reference
                            .get("upstreamShortName")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string),
                        ahead: reference
                            .get("ahead")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0) as usize,
                        behind: reference
                            .get("behind")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0) as usize,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 引用行的标题：`shortName`（macOS `GitLogView.swift:2456`）。
///
/// **偏差**：本地分支有 upstream 且领先/落后时这里追加 ` ↑ahead ↓behind`。
/// macOS 的引用行**不显示** ahead/behind（只在 Update 确认框里显示，
/// `GitLogView.swift:2348`），本步按任务要求把它补进引用树，颜色仍是主题色
/// （`reference_row` 里用 `muted_foreground` 画整行标题）。
fn reference_label(entry: &RefEntry) -> SharedString {
    if entry.kind == "local"
        && entry.upstream_short_name.is_some()
        && (entry.ahead > 0 || entry.behind > 0)
    {
        SharedString::from(format!(
            "{} ↑{} ↓{}",
            entry.short_name, entry.ahead, entry.behind
        ))
    } else {
        SharedString::from(entry.short_name.clone())
    }
}

/// 把引用列表拼成 macOS 的引用树（`GitLogView.swift:664-697`）。
///
/// 结构：`HEAD（当前分支）`（只有存在当前引用时才画，detached 时不画）→
/// `本地` / `远程` / `标签` 三个分组（**空分组不画**，`:750-756`）→ 组内引用。
/// 返回 `(items, 当前分支节点 id)`：后者给 [`reference_row`] 画绿勾与点亮用。
///
/// 节点 id 形状（[`reference_kind`] 依赖它）：`ref:HEAD`、`ref:group:<kind>`、
/// `ref:<kind>:<fullName>`。
fn build_reference_tree(entries: &[RefEntry]) -> (Vec<TreeItem>, Option<SharedString>) {
    let current = entries
        .iter()
        .find(|entry| entry.is_current)
        .map(|entry| SharedString::from(format!("ref:{}:{}", entry.kind, entry.full_name)));

    let mut items: Vec<TreeItem> = Vec::new();
    if current.is_some() {
        // `HEAD（当前分支）` 单独一行（zh-Hans `Localizable.strings:1742`），
        // 指向的就是当前引用本身（macOS `GitLogView.swift:667-670`）。
        items.push(TreeItem::new("ref:HEAD", "HEAD（当前分支）"));
    }

    // 分组顺序固定：本地 → 远程 → 标签（`:672-689`）；文案取 zh-Hans
    // `Localizable.strings:364-366`。
    for (kind, title) in [("local", "本地"), ("remote", "远程"), ("tag", "标签")] {
        let children: Vec<TreeItem> = entries
            .iter()
            .filter(|entry| entry.kind == kind)
            .map(|entry| {
                TreeItem::new(
                    SharedString::from(format!("ref:{}:{}", entry.kind, entry.full_name)),
                    reference_label(entry),
                )
            })
            .collect();
        if children.is_empty() {
            continue;
        }
        items.push(
            // 三个分组都默认展开（macOS `localExpanded / remoteExpanded / tagsExpanded`，`:23-25`）。
            TreeItem::new(SharedString::from(format!("ref:group:{kind}")), title)
                .expanded(true)
                .children(children),
        );
    }

    (items, current)
}

/// 解析 Core 的 `%D` 装饰串（`commit.decorations`，`contracts.rs:610`）。
///
/// Core 用 `--pretty=format:…%x1f%D`（`rust/lithe-core/src/git/history.rs:302`），
/// `%D` 的形状是逗号分隔的引用名，例如
/// `HEAD -> main, origin/main, tag: v1.0.0`。这里按 macOS 的种类规则拆分
/// （`GitGraphView.swift:775-782` 的 `.head` / `.branch` / `.remote` / `.tag`）：
/// `HEAD -> x` → HEAD 徽标 + 本地徽标 `x`；`tag: x` → 标签徽标 `x`；
/// 含 `/` → 远端；其余 → 本地。
///
/// **偏差（已登记）**：`%D` 不带种类信息，含 `/` 就判远端；名字里带 `/` 的**本地**分支
/// （如 `fix/order`）会被画成远端色。真机是拿 `GitReference.kind` 判定的
/// （`GitModels.swift:204-238`），要接准就得把 `git.references` 的 remote 名单一起传进来，
/// 本轮没做。
fn parse_decorations(value: &str) -> Vec<Decoration> {
    let mut decorations = Vec::new();

    for part in value.split(',').map(str::trim).filter(|part| !part.is_empty()) {
        // `HEAD -> main`：HEAD 自己是一个 `.head` 徽标，箭头后面那个是本地分支徽标。
        if let Some(branch) = part.strip_prefix("HEAD -> ") {
            decorations.push(Decoration {
                name: SharedString::from("HEAD"),
                kind: DecorationKind::Head,
            });
            decorations.push(Decoration {
                name: SharedString::from(branch.to_string()),
                kind: DecorationKind::Local,
            });
            continue;
        }

        if let Some(tag) = part.strip_prefix("tag: ") {
            decorations.push(Decoration {
                name: SharedString::from(tag.to_string()),
                kind: DecorationKind::Tag,
            });
            continue;
        }

        let (name, kind) = if part.contains('/') {
            (part, DecorationKind::Remote)
        } else if part == "HEAD" {
            (part, DecorationKind::Head)
        } else {
            (part, DecorationKind::Local)
        };
        decorations.push(Decoration {
            name: SharedString::from(name.to_string()),
            kind,
        });
    }

    decorations
}

/// 解析 `git.historyPage` 的 `commits[]`（`contracts.rs:602-611 / :640-649`）。
fn parse_commits(data: &serde_json::Value) -> Vec<CommitRow> {
    data.get("commits")
        .and_then(serde_json::Value::as_array)
        .map(|commits| {
            commits
                .iter()
                .filter_map(|commit| {
                    Some(CommitRow {
                        hash: SharedString::from(commit.get("hash")?.as_str()?),
                        short_hash: SharedString::from(
                            commit
                                .get("shortHash")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        decorations: parse_decorations(
                            commit
                                .get("decorations")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        subject: SharedString::from(
                            commit
                                .get("subject")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        author: SharedString::from(
                            commit
                                .get("authorName")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        author_email: SharedString::from(
                            commit
                                .get("authorEmail")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        date: SharedString::from(
                            commit
                                .get("date")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 把 `git.commitFiles` 的 `files[] { status, path }` 聚成多层目录树
/// （`contracts.rs:703-714`；macOS 的 `GitCommitFileTreeNode.build`，`GitModels.swift:285-323`）。
fn build_commit_files(files: &[(String, String)]) -> Vec<CommitFileRow> {
    let mut root = CommitFileNode::default();
    for (status, path) in files {
        root.insert(path, status);
    }

    let mut rows = Vec::new();
    root.flatten(0, &mut rows);
    rows
}

/// 读一个提交的文件列表：`git.commitFiles { root, commit }`
/// （请求 `git/mod.rs:650-653`，实现 `git/mod.rs:2013-2038`）。
///
/// 失败返回 Core 的错误码字符串，由 [`BottomPanel::apply_commit_files`] 换成
/// 「无法加载更改的文件」+ 占位数据。
fn load_commit_files(root: &str, commit: &str) -> Result<Vec<CommitFileRow>, String> {
    let data = execute_core(
        "git.commitFiles",
        serde_json::json!({ "root": root, "commit": commit }),
    )?;

    let files = data
        .get("files")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "响应缺少 files".to_string())?;

    let files: Vec<(String, String)> = files
        .iter()
        .filter_map(|file| {
            let path = file.get("path")?.as_str()?;
            let status = file
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("M");
            (!path.is_empty()).then(|| (status.to_string(), path.to_string()))
        })
        .collect();

    Ok(build_commit_files(&files))
}

/// 跑一遍 `git.status` → `git.references` → `git.historyPage`（第一页）。
///
/// **同步**函数，由 [`BottomPanel::spawn_git_load`] 放进 `cx.background_spawn(...)`：
/// 三个命令都要用同一个根，且引用与历史在真机上也是同时刷新的
/// （`GitFeatureModel.swift:1736-1750`）。
///
/// 失败/非仓库的处理（对应「不要空白、不要 panic」）：
/// - `git.status` 成功但 `repositoryRoot == null` → **不是 Git 仓库**
///   （`git/mod.rs:6535-6544`），不再白跑另外两条命令，只记一条 macOS 原文
///   「当前项目不是 Git 仓库」（`Localizable.strings:698`）；
/// - 某条命令失败 → 记 `命令（错误码）`，它的数据留 `None`，面板**保留占位数据**；
/// - `git.historyPage` 有 `nextCursor` → 本步只取第一页，按契约立刻
///   `git.historyCursorClose` 释放 Core 的 `git log` 流
///   （`shared/contracts/rust-core-api.md:894-895`）。
fn load_git_snapshot(root: &Path) -> GitSnapshot {
    let mut snapshot = GitSnapshot {
        is_repository: false,
        branch: None,
        repository_root: None,
        references: None,
        commits: None,
        failures: Vec::new(),
    };

    let root_text = root.to_string_lossy().to_string();

    // 1) git.status：仓库根 + 当前分支。
    let mut repository_root_text = root_text.clone();
    match execute_core(
        "git.status",
        serde_json::json!({ "root": root_text.as_str() }),
    ) {
        Ok(data) => {
            snapshot.is_repository = data
                .get("repositoryRoot")
                .is_some_and(|value| !value.is_null());
            snapshot.branch = data
                .get("branch")
                .and_then(serde_json::Value::as_str)
                .map(SharedString::from);
            if let Some(repository_root) = data
                .get("repositoryRoot")
                .and_then(serde_json::Value::as_str)
            {
                repository_root_text = resolve_repository_root(root, repository_root);
                snapshot.repository_root = Some(repository_root_text.clone());
            }
        }
        Err(code) => snapshot.failures.push(format!("git.status（{code}）")),
    }

    if !snapshot.is_repository {
        // 非仓库（或工作区不存在）：Core 的 `git.references` / `git.historyPage` 只会
        // 再失败一次，这里直接返回，让面板走「保留占位 + 提示」。
        if snapshot.failures.is_empty() {
            snapshot
                .failures
                .push("当前项目不是 Git 仓库".to_string());
        }
        return snapshot;
    }

    // 2) git.references：引用树。
    match execute_core(
        "git.references",
        serde_json::json!({ "root": repository_root_text.as_str() }),
    ) {
        Ok(data) => {
            snapshot.references = Some(parse_references(&data));
        }
        Err(code) => snapshot
            .failures
            .push(format!("git.references（{code}）")),
    }

    // 3) git.historyPage：第一页提交（macOS 传 order: "date"、limit 100，
    //    `RustGitOperations.swift:594-601`、`GitFeatureModel.swift:298`）。
    match execute_core(
        "git.historyPage",
        serde_json::json!({
            "root": repository_root_text.as_str(),
            "order": "date",
            "limit": HISTORY_PAGE_LIMIT,
        }),
    ) {
        Ok(data) => {
            let next_cursor = data
                .get("nextCursor")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            snapshot.commits = Some(parse_commits(&data));
            if let Some(cursor) = next_cursor {
                // 「加载更多」没做（缺口 3），所以把游标还回去，别占住 Core 的
                // 增量 `git log` 流（每根最多 8 条，空闲 120 s 才回收：
                // `rust/lithe-core/src/git/history.rs:25-29`）。
                let _ = execute_core(
                    "git.historyCursorClose",
                    serde_json::json!({
                        "root": repository_root_text.as_str(),
                        "cursor": cursor,
                    }),
                );
            }
        }
        Err(code) => snapshot
            .failures
            .push(format!("git.historyPage（{code}）")),
    }

    snapshot
}

/// 底部工具窗面板。
///
/// 它自带头部与页签，所以不复用 `panels.rs` 的 `ShellPanel`：那种面板靠
/// `PanelKind` 分支渲染，而这里需要一条完全不同的 chrome，独立成类型才不会
/// 和其他区域互相牵动。
pub(super) struct BottomPanel {
    /// 稳定名字，用于布局持久化与恢复注册。
    name: &'static str,
    /// 工作区根（本轮是 [`workspace_root`] 的兜底值，`new` 的签名没变）。
    root: PathBuf,
    /// `git.status.repositoryRoot` 解析成绝对路径后的仓库根：`git.commitFiles` 用它当 `root`
    /// （相对路径的来源见 [`resolve_repository_root`]）；`None` = 还没接上 / 不是仓库。
    repository_root: Option<String>,
    /// `git.status.branch`：标题栏 `日志：<当前分支>` 的兜底（`GitLogView.swift:396`）。
    branch: Option<SharedString>,
    /// 引用树里「当前分支」节点的 id（画绿勾 + 空选中时点亮，`:758-764 / :2713-2717`）。
    current_reference: Option<SharedString>,
    /// 当前页签（macOS `selectedGitToolTab`，`GitLogView.swift:42`）。
    tab: BottomTab,
    /// 引用树（macOS `GitReferenceRowsBuilder` 的扁平化结果，`GitReferenceRows.swift`）。
    /// 真数据来自 `git.references`，失败时是 [`placeholder_references`]。
    references: Entity<TreeState>,
    /// 提交行。真数据来自 `git.historyPage` 第一页，失败时是 [`placeholder_commits`]。
    commits: Rc<Vec<CommitRow>>,
    /// 选中的提交在 [`Self::commits`] 里的下标（macOS `selectedGitCommit`，
    /// `GitLogView.swift:1286-1290`）。`None` = 没有选中。
    selected_commit: Option<usize>,
    /// 提交文件树的行。真数据来自 `git.commitFiles`，失败 / 未选中时是
    /// [`placeholder_commit_files`]。
    commit_files: Rc<Vec<CommitFileRow>>,
    /// 提交文件栏的状态（占位 / 加载中 / 已加载 / 失败）。
    commit_files_state: CommitFilesState,
    /// 提交详情。选中提交后从那一行 [`CommitRow`] 填，否则是 [`placeholder_commit_detail`]。
    commit_detail: CommitDetail,
    /// Core 读取失败 / 非仓库时的一行提示（`None` = 全部成功，不画这一行）。
    notice: Option<SharedString>,
    /// 是否画引用装饰（macOS `showCommitDecorations`，`GitLogView.swift:1008-1012`）。
    show_decorations: bool,
    /// 提交栏的日志搜索框。当前只接收输入、不参与过滤（未接 `GitLogQuery`，`:1383-1391`）。
    search: Entity<InputState>,
    focus_handle: FocusHandle,
}

impl BottomPanel {
    /// `root` 是工作区根，由 `workspace.rs` 传入（`Core` 的 `git.*` 命令都要求请求里带
    /// `root`；之前用 `std::env::current_dir()` 兜底，会随启动目录漂移）。
    pub(super) fn new(window: &mut Window, cx: &mut Context<Self>, root: PathBuf) -> Self {
        // `new` 里也能拿实体弱引用（`workspace.rs:200` 同一写法），初载就不用等第一帧。
        let this = cx.entity().downgrade();

        let panel = Self {
            name: "BottomToolWindow",
            root: root.clone(),
            repository_root: None,
            branch: None,
            current_reference: None,
            tab: BottomTab::Log,
            // 三个分组都默认展开（macOS `localExpanded / remoteExpanded / tagsExpanded`，`:23-25`）；
            // 初值是占位树，`git.references` 回来后在 `apply_snapshot` 里整棵换掉。
            references: cx.new(|cx| TreeState::new(cx).items(placeholder_references())),
            commits: Rc::new(placeholder_commits()),
            selected_commit: None,
            commit_files: Rc::new(placeholder_commit_files()),
            commit_files_state: CommitFilesState::Placeholder,
            commit_detail: placeholder_commit_detail(),
            notice: None,
            show_decorations: true,
            // macOS 的 placeholder：`Text, me, author:, branch:, path:`（中文资源同键，
            // `GitLogView.swift:969`）。
            search: cx.new(|cx| {
                InputState::new(window, cx).placeholder("搜索文本，或使用 me、author:、branch:、path:")
            }),
            focus_handle: cx.focus_handle(),
        };

        Self::spawn_git_load(this, root, cx);

        panel
    }

    /// 后台跑 [`load_git_snapshot`]，回前台写状态。
    ///
    /// 写法照 `workspace.rs:199-231`（`ShellWorkspace::new` 里的初载）：`cx.spawn` + `cx.background_spawn`（同步 Core 调用
    /// 不能阻塞 UI 线程）+ `WeakEntity::update`（面板可能已经不在布局里）。
    fn spawn_git_load(this: WeakEntity<Self>, root: PathBuf, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let snapshot = cx
                .background_spawn(async move { load_git_snapshot(&root) })
                .await;
            let _ = this.update(cx, |panel, cx| panel.apply_snapshot(snapshot, cx));
        })
        .detach();
    }

    /// 把一次 Core 读取的结果写进面板状态。
    ///
    /// 分数据源处理，**失败的那一块保持原样（占位）**，只把失败拼成一行提示：
    /// - 引用树：`git.references` 成功才换树（失败时保留占位树，选中状态也不动）；
    /// - 提交行：`git.historyPage` 成功才换（失败时保留上一次的列表 / 占位列表）；
    /// - 分支 / 仓库根：只在成功时有值，失败时保留上一次的值（不会把已知分支擦掉）。
    ///
    /// 历史页回来后会选中第一条提交并加载它的文件（macOS 同样这么做：
    /// `GitFeatureModel.swift:1773-1782` 的 `historyPage.commits.first`）。
    fn apply_snapshot(&mut self, snapshot: GitSnapshot, cx: &mut Context<Self>) {
        let loaded_commits = snapshot.commits.is_some();

        // 引用树在前台拼（`TreeItem` 不是 `Send`，见 [`GitSnapshot`]）。
        if let Some(entries) = snapshot.references {
            let (items, current) = build_reference_tree(&entries);
            self.references
                .update(cx, |state, cx| state.set_items(items, cx));
            self.current_reference = current;
        }

        if let Some(commits) = snapshot.commits {
            self.commits = Rc::new(commits);
            self.selected_commit = None;
        }

        // 分支 / 仓库根只在成功时有值：失败时保留上一次的值，不把已知信息擦掉。
        if snapshot.repository_root.is_some() {
            self.repository_root = snapshot.repository_root;
        }
        if snapshot.branch.is_some() {
            self.branch = snapshot.branch;
        }

        self.notice = (!snapshot.failures.is_empty()).then(|| {
            SharedString::from(format!(
                "Git 数据未接入：{}（下方为占位数据）",
                snapshot.failures.join("；")
            ))
        });
        cx.notify();

        if loaded_commits && self.selected_commit.is_none() && !self.commits.is_empty() {
            self.select_commit(0, cx);
        }
    }

    /// 选中一行提交：填提交详情 + 后台加载该提交的文件（macOS `selectGitCommit` →
    /// `loadGitCommitFiles`，`GitFeatureModel.swift:1776-1782`）。
    ///
    /// 详情直接取选中行自己的字段（同一个 `GitCommitResponse`），不再多发一条
    /// `git.commit`（macOS 也只在按需刷新单条时才查 `git.commit`）。
    ///
    /// **偏差**：macOS 选中后有 120 ms 防抖（`GitLogView.swift:1243-1253`），本轮立即加载。
    fn select_commit(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(row) = self.commits.get(index).cloned() else {
            return;
        };

        self.selected_commit = Some(index);
        self.commit_detail = CommitDetail {
            subject: row.subject.clone(),
            short_hash: row.short_hash.clone(),
            author_name: row.author.clone(),
            author_email: row.author_email.clone(),
            date: row.date.clone(),
            // 详情里的装饰是**一行文本**（macOS `Text(commit.decorations)`，
            // `GitLogView.swift:1213-1218`），所以把徽标名拼回 `, ` 分隔。
            decorations: SharedString::from(
                row.decorations
                    .iter()
                    .map(|decoration| decoration.name.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            ),
        };
        // 文件列表先进加载态：清空旧行，免得显示上一个提交的文件。
        self.commit_files = Rc::new(Vec::new());
        self.commit_files_state = CommitFilesState::Loading;
        cx.notify();

        let root = self
            .repository_root
            .clone()
            .unwrap_or_else(|| self.root.to_string_lossy().to_string());
        let commit = row.hash.to_string();
        let this = cx.entity().downgrade();

        cx.spawn(async move |_this, cx| {
            let loaded = cx
                .background_spawn(async move { load_commit_files(&root, &commit) })
                .await;
            let _ = this.update(cx, |panel, cx| panel.apply_commit_files(loaded, cx));
        })
        .detach();
    }

    /// 把 `git.commitFiles` 的结果写进面板。
    ///
    /// 失败时**回到占位文件列表**（不空白）并把失败原因写进 [`Self::notice`]
    /// （这一行会覆盖更早的提示：最近一次失败更值得看）。
    fn apply_commit_files(
        &mut self,
        loaded: Result<Vec<CommitFileRow>, String>,
        cx: &mut Context<Self>,
    ) {
        match loaded {
            Ok(rows) => {
                self.commit_files = Rc::new(rows);
                self.commit_files_state = CommitFilesState::Loaded;
            }
            Err(code) => {
                self.commit_files = Rc::new(placeholder_commit_files());
                self.commit_files_state = CommitFilesState::Failed;
                self.notice = Some(SharedString::from(format!(
                    "无法加载更改的文件（git.commitFiles：{code}），下方为占位数据"
                )));
            }
        }
        cx.notify();
    }

    /// 重新跑一遍三个 Core 读命令（提交栏工具行的刷新按钮，macOS `:997-1022`）。
    fn reload(&mut self, cx: &mut Context<Self>) {
        let root = self.root.clone();
        let this = cx.entity().downgrade();
        Self::spawn_git_load(this, root, cx);
    }

    /// 当前选中的引用名；没有选中就是 `None`（= macOS 的 `isShowingAllGitReferences`）。
    ///
    /// 分组行被选中不算选中引用（macOS 的引用行与分组行是两类行，`:2666-2672`）。
    fn selected_reference_label(&self, cx: &App) -> Option<SharedString> {
        self.references
            .read(cx)
            .selected_entry()
            .filter(|entry| !entry.is_folder())
            .map(|entry| entry.item().label.clone())
    }

    /// 标题栏。macOS `toolWindowHeader`（`GitLogView.swift:380-461`）：
    /// `[VCS 图标 14][Git 13.5 semibold][Log 页签][Worktrees 页签][Console 页签]
    ///  [＋（仅选中引用时）][⋯ 菜单 28×28] …… [− 隐藏]`，
    /// 高 32、左右内边距 12 / 7、间距 4、底色 `toolHeader`、底部 1px `divider`。
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_reference = self.selected_reference_label(cx);
        // Log 页签标题：`Log: <短名>`（`GitLogView.swift:394-397`）。没有显式选中引用时
        // macOS 用的是**当前分支名**（`feature.currentBranch`，`:396`），分支名来自
        // `git.status.branch`；两者都没有才退回 `All References`
        // （zh-Hans `Localizable.strings:1817` → 所有引用）。
        let log_tab_label = match selected_reference.clone() {
            Some(name) => SharedString::from(format!("日志：{name}")),
            None => match self.branch.clone() {
                Some(branch) => SharedString::from(format!("日志：{branch}")),
                None => SharedString::from("日志：所有引用"),
            },
        };

        let mut header = h_flex()
            .h(px(HEADER_HEIGHT))
            .pl(px(HEADER_PADDING_LEADING))
            .pr(px(HEADER_PADDING_TRAILING))
            .gap(px(HEADER_GAP))
            .bg(cx.theme().tab_bar)
            .border_b_1()
            .border_color(cx.theme().border)
            // macOS 是 `toolwindows/toolWindowVcs.svg`（14pt，`:382-387`）；默认图标集没有
            // VCS 字形（`gpui-kit-assets-0.6.6/default-icons.txt`），用 `BookOpen` 代替。
            .child(
                Icon::new(IconName::BookOpen)
                    .size(px(14.))
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .pr(px(4.))
                    .text_size(px(13.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(cx.theme().foreground)
                    .child(SharedString::from("Git")),
            )
            .child(self.tab_button(BottomTab::Log, log_tab_label, None, cx))
            // `Worktrees · <repoPath>`：detail 是**仓库根路径**（`gitRepositoryRoot?.path`，
            // `GitLogView.swift:398-402`，`gitToolTabButton` 的 `detail` 分支在 `:479-485`：
            // `·` 用 `tertiaryText`、路径用 `secondaryText`、`truncationMode(.middle)`）。
            // 仓库根来自 `git.status.repositoryRoot`；拿不到时只画页签名。
            .child(self.tab_button(
                BottomTab::Worktrees,
                SharedString::from("工作树"),
                self.repository_root.clone().map(SharedString::from),
                cx,
            ))
            .child(self.tab_button(
                BottomTab::Console,
                SharedString::from("控制台"),
                None,
                cx,
            ));

        // `＋` 只在「选中了某个引用」时出现，作用是回到显示全部引用
        // （`GitLogView.swift:405-414`）。这里把「清空树的选中」当作等价动作。
        if selected_reference.is_some() {
            header = header.child(
                Button::new("bottom-show-all-refs")
                    .compact()
                    .icon(IconName::Plus)
                    .tooltip("显示所有引用")
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.references
                            .update(cx, |state, cx| state.set_selected_index(None, cx));
                        cx.notify();
                    })),
            );
        }

        header
            // 菜单（macOS 是 28×28 的 `⋯`：Fetch All Remotes / Fetch Options… /
            // Update Current Branch / Refresh Log / Show Changes，`:416-442`）。
            // 菜单项属于缺口 4，这里只落按钮与提示。
            .child(
                Button::new("bottom-git-actions")
                    .compact()
                    .icon(IconName::Ellipsis)
                    .tooltip("Git 工具窗口操作"),
            )
            .child(div().flex_1().min_w(px(12.)))
            // `−` = 隐藏工具窗（`:446-452`）。
            .child(
                Button::new("bottom-hide")
                    .compact()
                    .icon(IconName::Minus)
                    .tooltip("隐藏 Git 工具窗口"),
            )
    }

    /// 一个页签。macOS `gitToolTabButton`（`GitLogView.swift:463-519`）：
    /// 高 27、圆角 5、左右内边距 9（选中 Console 时右内边距收紧到 4 以容纳关闭按钮）、
    /// 底色选中为 `subtleSelection`、描边选中为 `inputFocusBorder.opacity(0.72)` 1px，
    /// **没有彩色下划线**；文字 `GitVisual.toolbar` 12.5，选中 `primaryText`、未选中 `secondaryText`；
    /// 选中 Console 时右侧多一个 20×27 的 `xmark`（8.5 semibold）回到 Log。
    ///
    /// `detail`：页签的补充信息（只有 Worktrees 用，值是仓库根路径）。macOS 是
    /// `HStack(spacing: 5)` 里的 `·`（`tertiaryText`）+ 内容（`secondaryText`）+ 中间截断
    /// （`:479-485`）；这里用同一个间距与两个色的近似 token（`muted_foreground`），
    /// 截断交给 `text_ellipsis_middle`（gpui 侧不需要 `overflow_hidden`，见模块注释）。
    fn tab_button(
        &self,
        tab: BottomTab,
        label: SharedString,
        detail: Option<SharedString>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.tab == tab;
        let shows_close = selected && tab == BottomTab::Console;

        // `subtleSelection` 在 gpui 主题里取 `secondary`；`inputFocusBorder` 取 `primary`。
        let background = if selected {
            cx.theme().secondary
        } else {
            cx.theme().secondary.opacity(0.0)
        };
        let stroke = if selected {
            cx.theme().primary.opacity(0.72)
        } else {
            cx.theme().primary.opacity(0.0)
        };
        let text_color = if selected {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };

        let mut button = h_flex()
            .id(("bottom-tab", tab as usize))
            .h(px(TAB_HEIGHT))
            .gap(px(TAB_GAP))
            .pl(px(TAB_PADDING))
            .pr(px(if shows_close {
                TAB_PADDING_WITH_CLOSE
            } else {
                TAB_PADDING
            }))
            .rounded(px(TAB_RADIUS))
            .border_1()
            .border_color(stroke)
            .bg(background)
            .text_size(px(12.5))
            .text_color(text_color)
            .whitespace_nowrap()
            .child(label.clone());

        if let Some(detail) = detail {
            button = button
                .child(
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .child(SharedString::from("·")),
                )
                .child(
                    div()
                        // 路径可能很长：macOS 用 `.truncationMode(.middle)`，
                        // gpui 的等价物是 `text_ellipsis_middle()`（排版阶段截断）。
                        .max_w(px(220.))
                        .text_ellipsis_middle()
                        .text_color(cx.theme().muted_foreground)
                        .child(detail),
                );
        }

        let mut button = button.on_click(cx.listener(move |this, _event, _window, cx| {
            this.tab = tab;
            cx.notify();
        }));

        if shows_close {
            button = button.child(
                h_flex()
                    .id(("bottom-tab-close", tab as usize))
                    .w(px(TAB_CLOSE_WIDTH))
                    .h(px(TAB_HEIGHT))
                    .justify_center()
                    .child(
                        Icon::new(IconName::Close)
                            .size(px(9.))
                            .text_color(cx.theme().muted_foreground),
                    )
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.tab = BottomTab::Log;
                        cx.notify();
                    })),
            );
        }

        button
    }

    /// `Log` 页签的内容：macOS `logTabContent` 的三栏
    /// （`GitLogView.swift:2838-2904` 的 `GitLogThreePaneLayout`）。
    ///
    /// macOS 是两处嵌套（外层 `leading` 固定引用栏、`flexible` 是「提交栏 + 详情栏」，
    /// 内层 `trailing` 固定详情栏），gpui-kit 的一个 `ResizablePanelGroup` 里
    /// 三个 panel 互相让位就能表达同一件事，所以这里不嵌套。
    fn render_log(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_resizable("bottom-git-panes")
            .with_handle_appearance(split_handle_appearance())
            .child(
                resizable_panel()
                    .size(px(REFERENCE_PANE_DEFAULT_WIDTH))
                    // 上限：真机是按容器宽算的 35%（`:2856-2867`），当前只声明最小宽，
                    // 动态上限属于缺口 5。
                    .size_range(px(REFERENCE_PANE_MIN_WIDTH)..Pixels::MAX)
                    .flex_none()
                    .child(self.render_reference_pane(cx)),
            )
            .child(
                resizable_panel()
                    .size_range(px(COMMIT_PANE_MIN_WIDTH)..Pixels::MAX)
                    .child(self.render_commit_pane(cx)),
            )
            .child(
                resizable_panel()
                    .size(px(DETAIL_PANE_DEFAULT_WIDTH))
                    // 上限：真机是 50%（`:2869-2880`），同上属于缺口 5。
                    .size_range(px(DETAIL_PANE_MIN_WIDTH)..Pixels::MAX)
                    .flex_none()
                    .child(self.render_detail_pane(cx)),
            )
    }

    /// 引用栏。macOS `referencePane`（`GitLogView.swift:638-703`）：
    /// 工具行 38（`⌃` 返回所有引用 + 放大镜清除日志搜索，左右内边距 6）→ 1px divider →
    /// 引用树（容器左右 8 / 上下 9）。
    fn render_reference_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // 没有显式选中引用时，当前分支行被点亮（`GitLogView.swift:758-764`）。
        let highlight_current = self.references.read(cx).selected_index().is_none();
        // 当前分支节点 id 来自 `git.references` 的 `isCurrent`（`build_reference_tree`）；
        // 它是 `None`（detached / 没接上）时 `HEAD（当前分支）` 行也画绿勾（macOS 同款判定）。
        let current = self.current_reference.clone();

        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .h(px(TOOLBAR_HEIGHT))
                    .px(px(REFERENCE_TOOLBAR_PADDING))
                    .gap(px(HEADER_GAP))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("git-ref-back")
                            .compact()
                            .icon(IconName::ChevronLeft)
                            .tooltip("返回所有引用")
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.references
                                    .update(cx, |state, cx| state.set_selected_index(None, cx));
                                cx.notify();
                            })),
                    )
                    .child(
                        // macOS 的放大镜 = 清空日志搜索（`:649-655`），这里接到搜索框上，是真的能用。
                        Button::new("git-ref-clear-search")
                            .compact()
                            .icon(IconName::Search)
                            .tooltip("清除日志搜索")
                            .on_click(cx.listener(|this, _event, window, cx| {
                                this.search.update(cx, |state, cx| state.set_value("", window, cx));
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1()),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .px(px(REFERENCE_TREE_PADDING_X))
                    .py(px(REFERENCE_TREE_PADDING_Y))
                    // 树根元素自己不裁剪：不套一层 `overflow_hidden` 的话，
                    // 过长的引用名会横向画到中间的提交栏上。
                    .overflow_hidden()
                    .child(Tree::new(
                        &self.references,
                        move |index, entry, selected, _window, cx| {
                            reference_row(index, entry, selected, highlight_current, current.as_ref(), cx)
                        },
                    )),
            )
    }

    /// 提交栏。macOS `commitPane`（`GitLogView.swift:963-1092`）：
    /// 工具行 38（搜索框 236×29 + 过滤器条 + 右侧 6 个工具图标，左右内边距 10、间距 8）
    /// → 1px divider → 提交列表。
    ///
    /// 过滤器条（Branch / User / Date / Path）与其中 4 个图标属于缺口 4；装饰开关是真的能用。
    fn render_commit_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let show_decorations = self.show_decorations;

        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .h(px(TOOLBAR_HEIGHT))
                    .px(px(COMMIT_TOOLBAR_PADDING))
                    .gap(px(COMMIT_TOOLBAR_GAP))
                    .bg(cx.theme().tab_bar)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        // 搜索框：`frame(width: 236, height: 29)`、圆角 5、
                        // `inputBackground` 底 + `inputBorder` 1px 描边（`:966-991`）。
                        h_flex()
                            .w(px(LOG_SEARCH_WIDTH))
                            .h(px(LOG_SEARCH_HEIGHT))
                            .flex_shrink_0()
                            .gap(px(6.))
                            .px(px(LOG_SEARCH_PADDING))
                            .rounded(px(LOG_SEARCH_RADIUS))
                            .border_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().background)
                            .child(
                                Icon::new(IconName::Search)
                                    .size(px(14.))
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(
                                div().flex_1().min_w_0().child(
                                    // `appearance(false)`：box 的底与描边由外面这层画，
                                    // 免得和 kit 的输入框 chrome 叠成双层边框。
                                    // `xsmall()`：单行 `Input` 的高度由 `Size` 决定
                                    // （`input.rs:703` 的 `input_h`，Medium 是 32 > 搜索框内容高 27），
                                    // XSmall 是 20 高 / 12pt 字，与 macOS 的 12.5pt 最接近。
                                    Input::new(&self.search)
                                        .appearance(false)
                                        .cleanable(true)
                                        .xsmall(),
                                ),
                            ),
                    )
                    .child(div().flex_1())
                    // 右侧按钮组：组内间距 2、与搜索框之间仍是 8（`HStack(spacing: 2)`，`:997-1022`）。
                    .child(
                        h_flex()
                            .gap(px(COMMIT_TOOLBAR_BUTTON_GAP))
                            .child(
                                Button::new("git-log-compare")
                                    .compact()
                                    .icon(IconName::Replace)
                                    .tooltip("比较当前分支与工作区"),
                            )
                            .child(
                                // macOS 的这个按钮本身没有 action
                                // （`gitToolbarIcon`，`:1003`，规格 §6.1.7）。
                                Button::new("git-log-details")
                                    .compact()
                                    .icon(IconName::Calendar)
                                    .tooltip("显示提交详情"),
                            )
                            .child(
                                Button::new("git-log-refresh")
                                    .compact()
                                    .icon(IconName::RotateCw)
                                    .tooltip("刷新 Git 日志")
                                    // macOS 的 Refresh Log（`:416-442` 菜单里的同一动作，
                                    // 工具行第 3 个图标 `:1008-1012`）：重跑三个读命令。
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.reload(cx);
                                    })),
                            )
                            .child(
                                Button::new("git-log-decorations")
                                    .compact()
                                    .icon(if show_decorations {
                                        IconName::Eye
                                    } else {
                                        IconName::EyeOff
                                    })
                                    .tooltip(if show_decorations {
                                        "隐藏提交装饰"
                                    } else {
                                        "显示提交装饰"
                                    })
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.show_decorations = !this.show_decorations;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("git-log-find")
                                    .compact()
                                    .icon(IconName::Search)
                                    .tooltip("在日志中查找"),
                            )
                            .child(
                                Button::new("git-log-long-edges")
                                    .compact()
                                    .icon(IconName::Frame)
                                    .tooltip("显示完整长连线"),
                            ),
                    ),
            )
            // Core 读取失败 / 非仓库的一行提示（macOS 没有这一行，是本项目为「不空白、
            // 不 panic」加的）：提示行不占列表的高度预算（`flex_shrink_0`）。
            .children(self.notice.clone().map(|notice| {
                div()
                    .flex_shrink_0()
                    .h(px(NOTICE_HEIGHT))
                    .px(px(COMMIT_TOOLBAR_PADDING))
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(NOTICE_FONT_SIZE))
                    .text_color(cx.theme().warning)
                    .child(notice)
            }))
            .child(div().flex_1().min_h_0().child(self.render_commit_list(cx)))
    }

    /// 提交列表。自绘行 + 手铺 `v_flex()` 的理由见模块注释。
    ///
    /// ⚠️ **这里不能用 `uniform_list`**（2026-09-24 实测）：把它放进
    /// `h_resizable` 的中间栏后，列表拿到的视口高度是 0（诊断输出显示闭包被调用时
    /// `visible_range` 是 `0..0`/`0..1`，屏幕上一个像素都不画）。改用固定行高的
    /// `v_flex()` 直接铺行；等容器高度问题解决、数据量大到需要虚拟化时再回来换
    /// （届时也可以考虑 `DataTable`，它自带 `ListSizingBehavior::Auto` 的处理，
    /// 见 `table/state.rs:2508`）。
    ///
    /// 行是可点的：macOS 单击选中 + 加载该提交的文件（`GitLogView.swift:1286-1290`），
    /// 判定在 [`BottomPanel::select_commit`]。行的 `id` 用下标（列表每次整批替换，
    /// 下标在同一批数据里稳定）。
    fn render_commit_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let show_decorations = self.show_decorations;
        let selected_commit = self.selected_commit;

        let mut list = v_flex().w_full();

        for (index, row) in self.commits.iter().enumerate() {
            list = list.child(
                commit_row(row, show_decorations, selected_commit == Some(index), cx)
                    .id(("bottom-commit-row", index))
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.select_commit(index, cx);
                    })),
            );
        }

        list
    }

    /// 详情栏（第三栏）。macOS `detailPane`（`GitLogView.swift:1094-1116`）：
    /// 上半是提交文件栏（默认高 = 容器高 − 5 − 156、最小 90），下半是提交详情（最小 110），
    /// 中间同样是 5pt 分隔条。这里把提交详情固定成默认的 156，文件栏吃掉剩余高度。
    fn render_detail_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_resizable("bottom-git-detail-split")
            .with_handle_appearance(split_handle_appearance())
            .child(
                resizable_panel()
                    .size_range(px(COMMIT_FILES_MIN_HEIGHT)..Pixels::MAX)
                    .child(self.render_commit_files(cx)),
            )
            .child(
                resizable_panel()
                    .size(px(COMMIT_DETAIL_DEFAULT_HEIGHT))
                    .size_range(px(COMMIT_DETAIL_MIN_HEIGHT)..Pixels::MAX)
                    .flex_none()
                    .child(self.render_commit_detail(cx)),
            )
    }

    /// 提交文件栏。macOS `commitFilesPane`（`GitLogView.swift:1118-1185`）：
    /// 工具行 38（比较更改 / 显示文件历史 / 切换预览 + 右对齐的 `N files`，
    /// 左右内边距 10、间距 5）→ 1px divider → 文件树。
    ///
    /// 四个状态按 macOS 的 `selectedGitCommitFilesLoadState`（`:1135-1183`）落：
    ///
    /// | 状态 | macOS | 这里 |
    /// |---|---|---|
    /// | `.idle`（没选中提交） | 居中 `Select a commit` | 画占位文件行（本步要求「不要空白」，偏差已登记） |
    /// | `.loading` | 转圈 + `Loading changed files…` | 顶部一行 `正在加载更改的文件…`（没搬转圈，见缺口 7） |
    /// | `.failed` | 居中 `Could not load changed files` + `Retry` | 顶部一行 `无法加载更改的文件` + **保留占位文件行**（`Retry` 没做，缺口 7） |
    /// | `.ready` 且空 | 居中 `No changed files` | 同左（居中一行） |
    /// | `.ready` | 文件树 | 真实 `status` + `path` 建出的多层目录树 |
    ///
    /// `N 个文件` 数的是**文件行**（macOS 数 `selectedGitCommitFiles.count`，`:1125`），
    /// 所以目录行不计入。
    fn render_commit_files(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let files = self.commit_files.clone();
        // 目录行不算文件数（macOS `selectedGitCommitFiles` 里只有文件）。
        let file_count = files.iter().filter(|row| !row.is_folder).count();

        // 状态行（加载中 / 失败）+ 列表。两者可以同时出现：失败时列表是**占位数据**。
        let status: Option<SharedString> = match self.commit_files_state {
            CommitFilesState::Placeholder | CommitFilesState::Loaded => None,
            CommitFilesState::Loading => Some(SharedString::from("正在加载更改的文件…")),
            CommitFilesState::Failed => Some(SharedString::from("无法加载更改的文件")),
        };
        // `.ready` 且没有任何更改：macOS 居中画 `No changed files`（zh-Hans
        // `Localizable.strings:856`）。
        let empty_ready =
            matches!(self.commit_files_state, CommitFilesState::Loaded) && file_count == 0;

        v_flex()
            .size_full()
            .min_h_0()
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .h(px(TOOLBAR_HEIGHT))
                    .px(px(COMMIT_TOOLBAR_PADDING))
                    .gap(px(COMMIT_FILES_TOOLBAR_GAP))
                    .bg(cx.theme().tab_bar)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("git-files-compare")
                            .compact()
                            .icon(IconName::Replace)
                            .tooltip("比较更改"),
                    )
                    .child(
                        Button::new("git-files-history")
                            .compact()
                            .icon(IconName::Calendar)
                            .tooltip("显示文件历史"),
                    )
                    .child(
                        Button::new("git-files-preview")
                            .compact()
                            .icon(IconName::Eye)
                            .tooltip("切换预览"),
                    )
                    .child(div().flex_1())
                    // `Text("\(count) files")`，中文资源 `"files" = "个文件"`（`:1125`）。
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(cx.theme().muted_foreground)
                            .child(SharedString::from(format!("{file_count} 个文件"))),
                    ),
            )
            .children(status.map(|status| {
                div()
                    .flex_shrink_0()
                    .px(px(COMMIT_TOOLBAR_PADDING))
                    .py(px(4.))
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child(status)
            }))
            .child(if empty_ready {
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(13.))
                    .text_color(cx.theme().muted_foreground)
                    .child(SharedString::from("没有更改的文件"))
                    .into_any_element()
            } else {
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(
                        uniform_list(
                            "bottom-git-commit-files",
                            files.len(),
                            move |range, _window, cx| {
                                range
                                    .filter_map(|ix| {
                                        files.get(ix).map(|row| commit_file_row(row, cx))
                                    })
                                    .collect()
                            },
                        )
                        .size_full(),
                    )
                    .into_any_element()
            })
    }

    /// 提交详情。macOS `commitDetail`（`GitLogView.swift:1187-1220`）：
    /// 内边距 11、行间距 9，依次是 subject 13.5 semibold、`短 hash  作者 <邮箱>` 12、
    /// 日期等宽 12、装饰 `accent` 12。
    fn render_commit_detail(&self, cx: &App) -> impl IntoElement {
        let detail = &self.commit_detail;

        v_flex()
            .size_full()
            .min_h_0()
            .p(px(COMMIT_DETAIL_PADDING))
            .gap(px(COMMIT_DETAIL_GAP))
            .bg(cx.theme().background)
            .child(
                div()
                    .text_size(px(13.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(cx.theme().foreground)
                    .child(detail.subject.clone()),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child(SharedString::from(format!(
                        "{}  {} <{}>",
                        detail.short_hash, detail.author_name, detail.author_email
                    ))),
            )
            .child(
                div()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child(detail.date.clone()),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(cx.theme().primary)
                    .child(detail.decorations.clone()),
            )
    }

    /// `Worktrees` 页签的内容。
    ///
    /// **占位空态**：文案取 macOS 的 `No worktrees` / `Create a checkout to get started.`；
    /// 真正的 `GitWorktreesView`（左列表默认 360、快速信息栏 282，`GitWorktreesView.swift:15-25`）
    /// 属于缺口 9，数据待 `git.worktrees` 接线（`RustCoreBridge.swift:3060`）。
    fn render_worktrees(cx: &App) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .items_center()
            .justify_center()
            .gap(px(9.))
            .bg(cx.theme().background)
            .child(
                Icon::new(IconName::Folder)
                    .size(px(30.))
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(cx.theme().foreground)
                    .child(SharedString::from("没有工作树")),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(cx.theme().muted_foreground)
                    .child(SharedString::from("新建一个检出目录以开始使用。")),
            )
    }

    /// `Console` 页签的内容。macOS 的 `GitConsoleView` 空态文案是
    /// `Git command output will appear here.`（中文资源同键）。
    /// 折叠片段 / 匹配定位 / 清空 / 复制属于缺口 8，数据待 `git.consolePresentation` 接线。
    fn render_console(cx: &App) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .p(px(12.))
            .gap(px(4.))
            .text_size(px(13.))
            .font_family(cx.theme().mono_font_family.clone())
            .text_color(cx.theme().muted_foreground)
            .bg(cx.theme().background)
            .child(SharedString::from("Git 命令输出将显示在这里。"))
            .child(SharedString::from(
                "（占位：控制台的执行记录、清空 / 复制输出 / 折叠片段 / 搜索都还没做）",
            ))
    }
}

impl EventEmitter<PanelEvent> for BottomPanel {}

impl Focusable for BottomPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for BottomPanel {
    fn panel_name(&self) -> &'static str {
        self.name
    }

    /// 工具窗是常驻区域；macOS 只有「隐藏」按钮（`GitLogView.swift:446-452`），没有关闭。
    fn closable(&self, _: &App) -> bool {
        false
    }
}

impl Panel for BottomPanel {
    fn tab_name(&self, _: &App) -> Option<SharedString> {
        // 工具窗名是 `Git`（`GitLogView.swift:389`）。
        Some(SharedString::from("Git"))
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // 只有标签组里出现多个面板时 Dock 才会画这条标题栏（见 `title_bar`）。
        SharedString::from("Git")
    }

    /// `false`：面板自己画标题栏与页签，Dock 不要再画一条。
    fn title_bar(&self, _: &App) -> bool {
        false
    }

    /// `false`：面板自己控制内边距，Dock 不要在内容上方再加留白。
    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

impl Render for BottomPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            // macOS 把页签放进 32 高的标题栏里（不是 Windows 那条独立的标签行）。
            .child(self.render_header(cx))
            .child(match self.tab {
                BottomTab::Log => self.render_log(cx).into_any_element(),
                BottomTab::Worktrees => Self::render_worktrees(cx).into_any_element(),
                BottomTab::Console => Self::render_console(cx).into_any_element(),
            })
    }
}
