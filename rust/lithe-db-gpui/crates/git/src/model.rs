//! Git Feature **底部提交记录面板**的数据层：5 条 `git.*` 命令的请求 / 解析、分页游标生命周期、
//! 行数据与纯函数。
//!
//! > ⚠️ **数目纠正**：本行原先写作「6 条 `git.*` 命令」，实际只有 5 条
//! > （`git.status` / `git.references` / `git.historyPage` / `git.historyCursorClose` /
//! > `git.commitFiles`）；第 6 条数据调用是**非 Git 命名空间**的 `workspace.snapshot`
//! > （[`discover_repository_root`]，用于"工作区根不是仓库"时的一级子目录探测）。
//!
//! 从 `shell_probe/bottom_panel.rs` 的「度量」常量区与「数据模型」「Core 调用与解析」
//! 「纯函数：泳道布局 / 引用树 / 文件树 / 过滤」四节逐字拆出（只调整可见性）。
//! 本文件不渲染任何东西：`log_view.rs` 只从这里取数据、常量和纯函数，所以度量常量
//! 也集中定义在这里（单一真源），由 `log_view.rs` 导入使用。
//!
//! 规格出处（Core 命令的 payload / 响应字段、三条硬约束、与 Windows 的 9 条刻意偏差）
//! 全部在 crate 根 `lib.rs` 的模块文档里，那是唯一真源；这里不重复。

use std::collections::BTreeSet;
use std::path::Path;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{App, ClickEvent, Hsla, SharedString, Window};

use lithe_db_gpui_shared::{CoreClient, CoreRequest, tr, tr_args};

// ---------------------------------------------------------------------------
// 度量
// ---------------------------------------------------------------------------
//
// 出处见模块文档的表格，行号一一对应。**布局度量一律用 gpui 的 rem-based helper，不再直接写
// `px(...)`**：rem base = 主题字号 16px，所以 helper 后缀 `N` = `N × 4px`，与 Windows 规格逐像素
// 相等。已换算的项（出处行号与常量区原注释一一对应）：
//
// - 标题栏（`git-log-title-bar.tsx:29-31`、`ui/button.tsx:9,27`）：32(`h-8`) → `h_8()`、
//   8 → `px_2()` / `gap_2()`、24 → `size_6()`、14(`size-3.5`) → `size_3p5()`；
// - 页签行（`git-log-tool-window.tsx:582`）：24 → `h_6()`、12 → `px_3()`、16 → `gap_4()`；
// - 刷新失败横幅（`:590`）：28(`h-7`) → `h_7()`、8 → `px_2()` / `gap_2()`；
// - 筛选条（`git-commit-table.tsx:200-232`）：32(`h-8`) → `h_8()`、8 → `px_2()` / `gap_2()`、
//   6(`px-1.5`) → `px_1p5()`；
// - 提交表（`:255,302,321,324,431`）：24(`h-6`) → `h_6()`、4(`px-1`) → `px_1()`、
//   8 → `px_2()`、128(`w-32`) → `w_32()`、36(`h-9`) → `h_9()`；
// - 泳道图（`git-graph-row.tsx:4-6,27,87-88`）：8 → `pl_2()` / `pr_2()`、
//   2(描边) → `border_2()`、6 → `px_1p5()` / `gap_1p5()`、14(`size-3.5`) → `size_3p5()`；
// - 三栏与引用栏（`git-reference-tree.tsx:146,357,371,840-919`）：320 → `min_w_80()`、
//   36 → `w_9()`、4 → `py_1()` / `gap_1()` / `my_1()` / `mb_1()`、32 → `size_8()` / `h_8()`、
//   16 → `size_4()`、20 → `w_5()`、1 → `h_px()`、6 → `p_1p5()` / `px_1p5()`、
//   28(`h-7`) → `h_7()`、24 → `h_6()`、4 → `px_1()` / `gap_1()`、14 → `size_3p5()`、
//   32(`pl-8`) → `pl_8()`；
// - Inspector（`git-commit-inspector.tsx:96-185`）：80 → `min_h_20()`、32 → `h_8()`、
//   8 → `px_2()` / `gap_2()`、12 → `p_3()`；
// - 提交文件树（`git-commit-file-tree.tsx:181-184,127`）：6 → `p_1p5()` / `pr_1p5()`、
//   24 → `h_6()`；
// - 控制台（`git-execution-console.tsx:68-103`）：32 → `w_8()`、4 → `py_1()` / `gap_1()`、
//   12 → `p_3()`。
//
// 字号：`ui-text-sm` 是 13px（`git-log-title-bar.tsx:31`、`git-commit-table.tsx:199`），不在 gpui 的
// 档位（`text_xs()`=12 / `text_sm()`=14）上 → 按《编码指南》用 `text_sm()`（14px，**有意**改动）；
// 12 → `text_xs()`（等价）；10 / 11（徽章、日期列、等宽字号）不在档位上 → 调用点写
// `rems(字号 / 16.)`（rem base = 16px，写 `/ 4.` 就错；`rems(10. / 16.)` 与原来的 `px(10.)`
// 在默认基准下逐像素相等，但会随界面字号缩放）。
//
// ⚠️ 下面每个常量的注释给出调用点**怎么消费**它，而不是"为什么可以保留 `px(...)`"：
// 「不在 gpui 的固定 rem 档位上」不是《编码指南》`coding-guides.md:288` 的例外类别
// —— 档位外本来就有 `rems(P / 16.)` 这条正路。只有确实需要 `Pixels` 类型值的槽
// （`Sizable::with_size(impl Into<Size>)`、`TabBar::max_width`、`lsp.completion_menu.max_width`
// 这类**固有方法/字段**，它们只接受 `Pixels`）才走 `rem_px(rem, C)` 换算，`rem` 取运行时基准。
//
// ⚠️ 圆角一律走应用层具名常量、不用 gpui 的 `rounded_sm()`/`rounded_md()`：Lithe 的圆角阶梯来自
// `--radius`（`sm`=4.8、`md`=6.4、`lg`=8、`xl`=11.2），与 gpui 的同名档位语义不同；
// 也不能从主题读 —— `ThemeConfig.radius` 是 `usize`
// （`gpui-component-0.6.6/src/theme/schema.rs:67-68`），装不下 4.8 / 6.4。
// 这条只约束"用哪个数"，不约束"怎么写"：消费方式仍然是 `rems(半径 / 16.)`。

/// 「引用名」胶囊最大宽 240（`git-log-title-bar.tsx:32-39` 的 `max-w-60`）。
///
/// 240 不在 gpui 的固定 rem 档位上（档位里 56 → 224、64 → 256），没有 `max_w_60()`
/// —— 这正是「档位外写 helper 底层的 `rems(P / 16.)`」的场合（`coding-guides.md:288` 的例外里
/// **没有**"值不在档位上"这一类）。调用点：`.max_w(rems(REFERENCE_PILL_MAX_WIDTH / 16.))`。
pub(crate) const REFERENCE_PILL_MAX_WIDTH: f32 = 240.;
/// 「引用名」胶囊圆角 6.4（`git-log-title-bar.tsx:32-39`）—— 见上方圆角说明。
pub(crate) const REFERENCE_PILL_RADIUS: f32 = 6.4;
/// 图标按钮 24×24（`git-log-title-bar.tsx:40-71`、`ui/button.tsx:9,27` 的 `icon-xs`）。
///
/// 该值走 `Sizable::with_size(impl Into<Size>)`，而 `Size` 只有 `From<Pixels>`
/// （`gpui-component-0.6.6/src/sizing.rs:169-183`），**没有** `From<Rems>`；换成 `Size::XSmall`
/// 会连带改掉按钮的内边距与图标尺寸，不是逐像素等价。所以调用点写
/// `rem_px(cx.theme().font_size, ICON_BUTTON_SIZE)`（`log_view.rs` 的 `rem_px`）——
/// 仍按当前 rem 基准求值，不是写死 16。
pub(crate) const ICON_BUTTON_SIZE: f32 = 24.;
/// 图标按钮圆角 6.4（`ui/button.tsx:9`）—— 见上方圆角说明。
pub(crate) const ICON_BUTTON_RADIUS: f32 = 6.4;

/// 筛选输入框最小宽 144（`git-commit-table.tsx:201` 的 `min-w-36`）。
///
/// 144 不在 gpui 的固定 rem 档位上（档位里 32 → 128、40 → 160），没有 `min_w_36()`；
/// 调用点：`.min_w(rems(FILTER_INPUT_MIN_WIDTH / 16.))`。
pub(crate) const FILTER_INPUT_MIN_WIDTH: f32 = 144.;
/// 筛选输入框最大宽 288（`git-commit-table.tsx:201` 的 `max-w-72`）。
///
/// 288 不在 gpui 的固定 rem 档位上（档位里 64 → 256、80 → 320），没有 `max_w_72()`；
/// 调用点：`.max_w(rems(FILTER_INPUT_MAX_WIDTH / 16.))`。
pub(crate) const FILTER_INPUT_MAX_WIDTH: f32 = 288.;

/// 提交行高 30（`git-commit-table.tsx:43` 的 `const ROW_HEIGHT = 30`）。
///
/// 30 不在 gpui 的固定 rem 档位上（档位里 28 / 32），且同时用于 `.min_h(..)` 与
/// `.line_height(..)`，后者本来也没有档位 helper —— 两处都写
/// `rems(COMMIT_ROW_HEIGHT / 16.)`（`line_height` 吃 `DefiniteLength`，`Rems` 有 `From`）。
pub(crate) const COMMIT_ROW_HEIGHT: f32 = 30.;
/// 作者列宽 112（`git-commit-table.tsx:321` 的 `w-28`）。
///
/// 112 不在 gpui 的固定 rem 档位上（档位里 24 → 96、32 → 128），没有 `w_28()`；
/// 调用点：`.w(rems(AUTHOR_COLUMN_WIDTH / 16.))`。
pub(crate) const AUTHOR_COLUMN_WIDTH: f32 = 112.;
/// 日期列字号 11、等宽（`git-commit-table.tsx:324`）。
///
/// gpui 的字号档位只有 12 / 14 / 16 / 18 …，11 不在档位上 → 调用点写
/// `.text_size(rems(DATE_FONT_SIZE / 16.))`（`text_size` 吃 `AbsoluteLength`，`Rems` 有 `From`；
/// 这就是"档位外用 helper 底层的 rems"这条正路，不是"保留 px 的理由"）。
pub(crate) const DATE_FONT_SIZE: f32 = 11.;
/// 提交行内容最小宽 520（`git-commit-table.tsx:276` 的 `min-w-130`）。
///
/// 520 不在 gpui 的固定 rem 档位上（最大档 128 → 512）；调用点：
/// `.min_w(rems(COMMIT_CONTENT_MIN_WIDTH / 16.))`。
pub(crate) const COMMIT_CONTENT_MIN_WIDTH: f32 = 520.;
/// 「加载更多提交」按钮高 24（`git-commit-table.tsx:434` 的 `size="xs"`）。
///
/// 与 [`ICON_BUTTON_SIZE`] 同因：它走 `Sizable::with_size(impl Into<Size>)`，而 `Size` 只有
/// `From<Pixels>`（`gpui-component-0.6.6/src/sizing.rs:169-183`），**没有** `From<Rems>`。
/// 调用点（三处）都写 `rem_px(cx.theme().font_size, LOAD_MORE_BUTTON_HEIGHT)`。
pub(crate) const LOAD_MORE_BUTTON_HEIGHT: f32 = 24.;

/// 泳道横向间距 13（`git-graph-row.tsx:4-6`）。调用点 `.w(rems(GRAPH_LANE_GAP / 16.))`。
///
/// ⚠️ "参与泳道坐标算术"**不是**保留 `px(...)` 的理由：13 是**设计常量**（真源 TS 的
/// `GRAPH_LANE_GAP`），不是 `prepaint` 测量出来的几何。rem 化之后泳道图会随界面字号缩放
/// —— 这正是想要的行为。算术里只有"与测量 bounds 直接相加"的那部分才属于
/// `coding-guides.md:288` 的 measured runtime geometry（本文件里没有这样的调用点）。
pub(crate) const GRAPH_LANE_GAP: f32 = 13.;
/// 泳道图最小宽 30（`git-graph-row.tsx:27`）。
///
/// 30 不在 gpui 的固定 rem 档位上，且参与泳道列宽算术（同 [`GRAPH_LANE_GAP`]：设计值不是测量值）；
/// 调用点：`.min_w(rems(GRAPH_MIN_WIDTH / 16.))`。
pub(crate) const GRAPH_MIN_WIDTH: f32 = 30.;
/// 泳道线宽 1.6（`git-graph-row.tsx:4-6`）：非整数，`border` 档位只有整数
/// → 调用点 `.w(rems(GRAPH_LINE_WIDTH / 16.))`（宽度走 `Styled::w`，不吃 border 档位）。
pub(crate) const GRAPH_LINE_WIDTH: f32 = 1.6;
/// 泳道节点半径 4.3（`git-graph-row.tsx:4-6`）：非整数。
/// 调用点：`.size(rems(GRAPH_NODE_RADIUS * 2. / 16.))`、`.h(rems((half - GRAPH_NODE_RADIUS) / 16.))`
/// —— 两个式子里的 `4.3` 都是设计常量，缩放后与 `half`（= `COMMIT_ROW_HEIGHT / 2.`）同步。
pub(crate) const GRAPH_NODE_RADIUS: f32 = 4.3;
/// 标签徽章最大宽 112（`git-graph-row.tsx:87-88`）。
///
/// 112 不在 gpui 的固定 rem 档位上（没有 `max_w_28()`）；调用点：
/// `.max_w(rems(LABEL_MAX_WIDTH / 16.))`。
pub(crate) const LABEL_MAX_WIDTH: f32 = 112.;
/// 标签徽章字号 10（`git-graph-row.tsx:87-88`）：不在 gpui 的字号档位上；
/// 调用点 `.text_size(rems(LABEL_FONT_SIZE / 16.))`。
pub(crate) const LABEL_FONT_SIZE: f32 = 10.;
/// 标签徽章圆角 6.4（`git-graph-row.tsx:87-88`）—— 见上方圆角说明。
pub(crate) const LABEL_RADIUS: f32 = 6.4;
/// 标签徽章纵向内边距 2（`git-graph-row.tsx:87-88`）—— 参与徽章高度的算术
/// （`LABEL_FONT_SIZE + LABEL_PADDING_Y * 2.`）；调用点
/// `.h(rems((LABEL_FONT_SIZE + LABEL_PADDING_Y * 2.) / 16.))`。
pub(crate) const LABEL_PADDING_Y: f32 = 2.;

/// 三栏比例。出处：`git-log-preferences.store.ts:43-47`、`git-log-tool-window.tsx:622,648,684`。
/// 比例不是长度，原样保留。
pub(crate) const REFERENCE_PANE_FRACTION: f32 = 0.19;
pub(crate) const COMMIT_PANE_FRACTION: f32 = 0.57;
pub(crate) const INSPECTOR_PANE_FRACTION: f32 = 0.24;
/// 引用栏最小宽 140（`git-log-preferences.store.ts:43-47`、`git-log-tool-window.tsx:622`）。
///
/// 140 不在 gpui 的固定 rem 档位上（档位里 32 → 128、40 → 160）；调用点：
/// `.min_w(rems(REFERENCE_PANE_MIN_WIDTH / 16.))`。
pub(crate) const REFERENCE_PANE_MIN_WIDTH: f32 = 140.;
/// Inspector 栏最小宽 220（同上）。
///
/// 220 不在 gpui 的固定 rem 档位上（档位里 48 → 192、56 → 224）；调用点：
/// `.min_w(rems(INSPECTOR_PANE_MIN_WIDTH / 16.))`。
pub(crate) const INSPECTOR_PANE_MIN_WIDTH: f32 = 220.;

/// 引用栏工具栏按钮圆角 4.8（`git-reference-tree.tsx:357,371`）—— 见上方圆角说明。
pub(crate) const REFERENCE_TOOLBAR_BUTTON_RADIUS: f32 = 4.8;
/// HEAD 行圆角 6.4（`git-reference-tree.tsx:853`）—— 见上方圆角说明。
pub(crate) const REFERENCE_HEAD_ROW_RADIUS: f32 = 6.4;
/// 分区头圆角 6.4（`git-reference-tree.tsx:876`）—— 见上方圆角说明。
pub(crate) const REFERENCE_SECTION_RADIUS: f32 = 6.4;
/// 引用行圆角 4.8（`git-reference-tree.tsx:599`）—— 见上方圆角说明。
pub(crate) const REFERENCE_ROW_RADIUS: f32 = 4.8;
/// 引用树缩进基准 10 / 步长 14（`git-reference-tree.tsx:594`）—— 逐层缩进的算术
/// （`.pl(rems((REFERENCE_INDENT_BASE + depth × REFERENCE_INDENT_STEP) / 16.))`）。
///
/// 10 / 14 都是**设计常量**（真源 TS 里的 `+ 10 + depth * 14`），不是测量几何：
/// rem 化之后引用树缩进随界面字号缩放。
pub(crate) const REFERENCE_INDENT_BASE: f32 = 10.;
pub(crate) const REFERENCE_INDENT_STEP: f32 = 14.;
/// 「当前」徽章字号 10（`git-reference-tree.tsx:652`）：不在 gpui 的字号档位上；
/// 调用点 `.text_size(rems(REFERENCE_BADGE_FONT_SIZE / 16.))`。
pub(crate) const REFERENCE_BADGE_FONT_SIZE: f32 = 10.;
/// ahead/behind 计数（`git-tracking-counts.tsx:36-53`）：字号 10 不在 gpui 的字号档位上；
/// 调用点 `.text_size(rems(TRACKING_COUNT_FONT_SIZE / 16.))`。
pub(crate) const TRACKING_COUNT_FONT_SIZE: f32 = 10.;
pub(crate) const TRACKING_COUNT_MAX: usize = 99;

/// Inspector 上下两区的比例。出处：`git-log-preferences.store.ts:49-52`。比例不是长度，原样保留。
pub(crate) const INSPECTOR_FILES_FRACTION: f32 = 0.62;
pub(crate) const INSPECTOR_DETAILS_FRACTION: f32 = 0.38;
/// Inspector 文件区最小高 90（`git-commit-inspector.tsx:96-185`）。
///
/// 90 不在 gpui 的固定 rem 档位上（档位里 20 → 80、24 → 96）；调用点：
/// `.min_h(rems(INSPECTOR_FILES_MIN_HEIGHT / 16.))`。
pub(crate) const INSPECTOR_FILES_MIN_HEIGHT: f32 = 90.;
/// Inspector 等宽字号 11（`git-commit-inspector.tsx:96-185`）：不在档位上；
/// 调用点 `.text_size(rems(INSPECTOR_MONO_FONT_SIZE / 16.))`。
pub(crate) const INSPECTOR_MONO_FONT_SIZE: f32 = 11.;
/// Inspector 提交哈希字号 10：不在档位上；调用点
/// `.text_size(rems(INSPECTOR_HASH_FONT_SIZE / 16.))`。
pub(crate) const INSPECTOR_HASH_FONT_SIZE: f32 = 10.;
/// 提交文件树缩进基准 10 / 步长 14（`git-commit-file-tree.tsx:127`、
/// `file-explorer/lib/file-tree-row.ts:1-13`）—— 与 [`REFERENCE_INDENT_BASE`] 同口径的
/// 设计常量算术，调用点写 `rems((COMMIT_FILE_INDENT_BASE + depth × COMMIT_FILE_INDENT_STEP) / 16.)`。
pub(crate) const COMMIT_FILE_INDENT_BASE: f32 = 10.;
pub(crate) const COMMIT_FILE_INDENT_STEP: f32 = 14.;
/// 提交文件状态字号 10（`git-commit-file-tree.tsx:181-184`）：不在档位上；调用点
/// `.text_size(rems(COMMIT_FILE_STATUS_FONT_SIZE / 16.))`。
pub(crate) const COMMIT_FILE_STATUS_FONT_SIZE: f32 = 10.;

/// `git.historyPage` 的页大小。Core 默认 300（`git/history.rs:20`），上一轮实现与 macOS
/// 口径都用 100（`GitFeatureModel.swift:298`）；这里取 100：提交行是自绘的（见模块文档），
/// 一页 100 行在首帧的布局量可控。
pub(crate) const HISTORY_PAGE_LIMIT: usize = 100;
/// 不是仓库时，最多向上层目录探测多少个一级子目录（[`discover_repository_root`]）。
pub(crate) const REPOSITORY_PROBE_LIMIT: usize = 6;

// ---------------------------------------------------------------------------
// 数据模型
// ---------------------------------------------------------------------------

/// 面板内的两个页签。Windows 是局部 `useState<"log" | "console">("log")`
/// （`git-log-tool-window.tsx:80`）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Panel {
    Log,
    Console,
}

/// 仓库数据的加载状态。对应 `git-log-tool-window.tsx:602-612` 的分支：
/// 无仓库 / 加载中 / 失败且无数据 / （有数据时的）刷新失败横幅。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadState {
    /// 首屏还没回来（`git-log-tool-window.tsx:607-610`）。
    Loading,
    /// 有数据。
    Ready,
    /// **有**数据但最近一次刷新失败 → 画横幅（`:589-600`）。
    Stale,
    /// 失败且没有数据 → 居中错误 + 重试。
    Failed,
    /// `git.status` 说这不是 Git 仓库（`:602-606`）。
    NoRepository,
}

/// 筛选字段。对应 `git-log-preferences.store.ts:8` 的 `"text" | "author" | "branch"`。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterScope {
    Text,
    Author,
    Branch,
}

impl FilterScope {
    /// 字段名对应的文案 key，逐字取自 `i18n/locale.ts:7279-7281`。
    fn label_key(self) -> &'static str {
        match self {
            FilterScope::Text => "lithe.git.log.filterText",
            FilterScope::Author => "lithe.git.log.filterAuthor",
            FilterScope::Branch => "lithe.git.log.filterBranch",
        }
    }

    pub(crate) fn label(self) -> SharedString {
        tr(self.label_key())
    }

    /// 占位文案 `{field} 筛选`（`git.log.filterPlaceholder`，`locale.ts:7282`）。
    pub(crate) fn placeholder(self) -> SharedString {
        tr_args(
            "lithe.git.log.filterPlaceholder",
            &[("field", self.label().as_ref())],
        )
    }

    pub(crate) fn all() -> [FilterScope; 3] {
        [FilterScope::Text, FilterScope::Author, FilterScope::Branch]
    }
}

/// 引用种类。Core 的 `kind` 字段（`git/mod.rs:5896-5902`）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefKind {
    Local,
    Remote,
    Tag,
}

impl RefKind {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "local" => Some(RefKind::Local),
            "remote" => Some(RefKind::Remote),
            "tag" => Some(RefKind::Tag),
            _ => None,
        }
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            RefKind::Local => "local",
            RefKind::Remote => "remote",
            RefKind::Tag => "tag",
        }
    }

    /// 分区标题文案逐字取自 `i18n/locale.ts:7252-7254`。
    pub(crate) fn title(self) -> SharedString {
        tr(match self {
            RefKind::Local => "lithe.git.log.local",
            RefKind::Remote => "lithe.git.log.remote",
            RefKind::Tag => "lithe.git.log.tags",
        })
    }

    pub(crate) fn sections() -> [RefKind; 3] {
        [RefKind::Local, RefKind::Remote, RefKind::Tag]
    }
}

/// 一条引用（Core `GitReferenceResponse`，`protocol/contracts.rs:583-597`）。
#[derive(Clone)]
pub(crate) struct Reference {
    pub(crate) full_name: SharedString,
    pub(crate) short_name: SharedString,
    pub(crate) kind: RefKind,
    pub(crate) is_current: bool,
    pub(crate) upstream_short_name: Option<SharedString>,
    pub(crate) ahead: usize,
    pub(crate) behind: usize,
}

/// 提交标签的种类。Windows 的 `GitGraphLabel.kind`（`git-graph-row.tsx:13-24`）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LabelKind {
    /// `HEAD`（或 `HEAD -> x` 里的 HEAD 那一段）。
    Head,
    /// 远端分支（名字里含 `/`）。
    Remote,
    /// `tag: x`。
    Tag,
    /// 本地分支。
    Branch,
}

/// 一个标签徽章。
#[derive(Clone)]
pub(crate) struct Label {
    pub(crate) title: SharedString,
    pub(crate) kind: LabelKind,
}

/// 一行提交（Core `GitCommitResponse`，`protocol/contracts.rs:602-611`）。
#[derive(Clone)]
pub(crate) struct Commit {
    pub(crate) hash: SharedString,
    pub(crate) short_hash: SharedString,
    pub(crate) parent_hashes: Vec<SharedString>,
    /// Core 只给 `%s`（主题行，`git/history.rs:302`），没有正文。
    pub(crate) subject: SharedString,
    pub(crate) author: SharedString,
    pub(crate) email: SharedString,
    pub(crate) date: SharedString,
    pub(crate) labels: Vec<Label>,
}

/// 提交行左侧的泳道图（简化版，见模块文档偏差 2）。
pub(crate) struct GraphRow {
    /// 每条泳道的线色下标；`None` = 这条泳道本行上半部分没有线。
    pub(crate) lanes: Vec<Option<usize>>,
    /// 本提交所在的泳道。
    pub(crate) lane: usize,
    /// 本行的节点颜色下标。
    pub(crate) node_color: usize,
    /// 往下连的边：`(泳道, 颜色下标, 父提交不在本页)`。
    pub(crate) edges: Vec<(usize, usize, bool)>,
}

/// 提交文件树的一行。
#[derive(Clone)]
pub(crate) struct CommitFileRow {
    pub(crate) depth: usize,
    pub(crate) name: SharedString,
    pub(crate) is_folder: bool,
    /// 目录行的文件数（含子目录，递归）。
    pub(crate) file_count: usize,
    /// 文件行的 name-status 码（Core `GitFileResponse.status`，`contracts.rs:703-707`）。
    pub(crate) status: SharedString,
}

/// 文件列表的加载状态。对应 `git-commit-inspector.tsx:19` 的
/// `"idle" | "loading" | "ready" | "failed"`。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilesState {
    /// 还没选中提交。
    Idle,
    Loading,
    Ready,
    Failed,
}

/// Inspector 下半部分的提交详情。取选中的那一行的字段（Windows 也是同一个 `GitCommit`）。
pub(crate) struct Detail {
    pub(crate) subject: SharedString,
    pub(crate) short_hash: SharedString,
    pub(crate) author: SharedString,
    pub(crate) email: SharedString,
    pub(crate) date: SharedString,
    /// 装饰拼回一行文本（Windows 直接渲染 `commit.decorations`，`git-commit-inspector.tsx:180`）。
    pub(crate) decorations: SharedString,
    pub(crate) hash: SharedString,
}

/// 交给后台任务的游标句柄：`git.historyCursorClose` 需要 `root` 与 `cursor` 成对
/// （`git/history.rs:344` 会校验游标属于哪个 root）。
pub(crate) struct HistoryCursor {
    pub(crate) root: String,
    pub(crate) cursor: String,
}

impl HistoryCursor {
    /// 把游标还给 Core（同步、很快：只是摘掉注册表项并停掉子进程）。
    pub(crate) fn close(self) {
        let _ = execute_core(
            "git.historyCursorClose",
            serde_json::json!({ "root": self.root, "cursor": self.cursor }),
        );
    }
}

/// 一次「首屏」读取的结果（工作区探测 → `git.status` → `git.references` → `git.historyPage`）。
///
/// 必须在后台线程上构造再送回前台，所以只装 `String` / `SharedString` 这些 `Send` 数据。
pub(crate) struct FirstLoad {
    /// 解析成绝对路径的仓库根；`None` = 不是 Git 仓库。
    pub(crate) repository_root: Option<String>,
    pub(crate) branch: Option<SharedString>,
    pub(crate) references: Vec<Reference>,
    pub(crate) commits: Vec<Commit>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) has_more: bool,
    /// 失败的命令（空 = 全部成功）。
    pub(crate) failures: Vec<String>,
}

/// 追加一页的结果。
pub(crate) struct MoreLoad {
    pub(crate) commits: Vec<Commit>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) has_more: bool,
    pub(crate) failure: Option<String>,
}

/// 一个 Icon 按钮的点击回调。
pub(crate) type ButtonHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// 把闭包包成 [`ButtonHandler`]。
///
/// 走一层泛型函数而不是直接 `Box::new(..)`：闭包的参数类型由 `Fn(&ClickEvent, &mut Window,
/// &mut App)` 这个 bound 推出来，不必在每个调用点手写三处参数类型。
pub(crate) fn handler(f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> ButtonHandler {
    Box::new(f)
}

// ---------------------------------------------------------------------------
// Core 调用与解析
// ---------------------------------------------------------------------------

/// Git 读命令超时（毫秒）。
///
/// 照 `shell_probe/files.rs` 的 `workspace.snapshot` 口径取 **60 s**：这是本 Feature 原来的值，
/// 收敛到 `shared::core_client` 时用 `CoreRequest::with_timeout_millis` 显式保留，
/// 不跟随信封层的默认值（那会把超时悄悄放宽到 120 s，属可观察的行为变化）。
pub(crate) const GIT_TIMEOUT_MILLIS: u64 = 60_000;

/// 调一次 Core 命令：拼请求、判 `ok`、取 `data`。
///
/// 请求形态（`id` / `operationId` / `timeoutMilliseconds` / `command` / `payload`）由
/// `lithe_db_gpui_shared::core_client` 的信封层负责 —— 本函数只保留本 Feature 自己的两件事：
/// **超时口径**（[`GIT_TIMEOUT_MILLIS`]）与**失败语义**。失败返回错误码字符串
/// （`/error/code`）；非仓库那种「`ok:true` + 字段为 null」由调用方判字段。
pub(crate) fn execute_core(
    command: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    CoreClient::new()
        .execute(
            &CoreRequest::command(command)
                .with_payload(payload)
                .with_timeout_millis(GIT_TIMEOUT_MILLIS),
        )
        .map(|data| data.unwrap_or(serde_json::Value::Null))
        .map_err(|error| match error.code() {
            Some(code) => code.to_string(),
            // 信封级失败（响应不是合法 JSON）没有 Core 错误码，保留 `CoreError` 自己的原文。
            None => error.to_string(),
        })
}

/// 把 `git.status.repositoryRoot` 变成可用的绝对根。
///
/// Core 给的是 `relative_or_absolute(&repository_root, &root)`（`rust/lithe-core/src/git/mod.rs:6574`）：
/// 仓库根就在工作区里时会返回**相对路径**，直接拿去当后面几条命令的 `root` 会按进程工作目录解析。
pub(crate) fn resolve_repository_root(root: &Path, repository_root: &str) -> String {
    let path = Path::new(repository_root);
    if path.is_absolute() {
        repository_root.to_string()
    } else {
        root.join(path).to_string_lossy().to_string()
    }
}

/// `git.status` 的 `repositoryRoot`（`contracts.rs:533`；非仓库为 `null`）。
pub(crate) fn repository_root_of(data: &serde_json::Value) -> Option<String> {
    data.get("repositoryRoot")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// 读 `git.status`，返回 `(仓库根, 分支)`；仓库根为 `None` 表示不是仓库。
pub(crate) fn read_status(root: &str) -> Result<(Option<String>, Option<SharedString>), String> {
    let data = execute_core("git.status", serde_json::json!({ "root": root }))?;
    let branch = data
        .get("branch")
        .and_then(serde_json::Value::as_str)
        .map(SharedString::from);
    Ok((repository_root_of(&data), branch))
}

/// 工作区根不是仓库时的兜底：用 `workspace.snapshot` 拿一级子目录，逐个探 `git.status`。
///
/// Windows 用 `git_discover_workspace_repos` 找子目录里的仓库，那不在本步拍板的 6 条命令里；
/// 这里用 `workspace.snapshot`（`project/files.rs:160-167`）+ `git.status` 复现同一件事，
/// 探测**有上限**（[`REPOSITORY_PROBE_LIMIT`]），且只探一级目录：`.git` 本身被
/// `BUILT_IN_HIDDEN_DIRECTORIES` 过滤掉（`files.rs:15-30`），所以看的是「哪个子目录是仓库」。
pub(crate) fn discover_repository_root(root: &Path) -> Option<String> {
    let data = execute_core(
        "workspace.snapshot",
        serde_json::json!({ "root": root.to_string_lossy() }),
    )
    .ok()?;

    let children = data.pointer("/root/children")?.as_array()?;
    let directories = children.iter().filter_map(|child| {
        let is_directory = child
            .get("isDirectory")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let path = child.get("path").and_then(serde_json::Value::as_str)?;
        is_directory.then(|| root.join(path).to_string_lossy().to_string())
    });

    for candidate in directories.take(REPOSITORY_PROBE_LIMIT) {
        if let Ok(data) = execute_core("git.status", serde_json::json!({ "root": candidate })) {
            if let Some(found) = repository_root_of(&data) {
                return Some(resolve_repository_root(root, &found));
            }
        }
    }

    None
}

/// 解析 `git.references` 的 `references[]`（`contracts.rs:583-597`）。
pub(crate) fn parse_references(data: &serde_json::Value) -> Vec<Reference> {
    data.get("references")
        .and_then(serde_json::Value::as_array)
        .map(|references| {
            references
                .iter()
                .filter_map(|reference| {
                    Some(Reference {
                        full_name: SharedString::from(reference.get("fullName")?.as_str()?),
                        short_name: SharedString::from(reference.get("shortName")?.as_str()?),
                        kind: RefKind::parse(reference.get("kind")?.as_str()?)?,
                        is_current: reference
                            .get("isCurrent")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false),
                        upstream_short_name: reference
                            .get("upstreamShortName")
                            .and_then(serde_json::Value::as_str)
                            .map(SharedString::from),
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

/// 解析 `%D` 装饰串（`commit.decorations`，`contracts.rs:610`）。
///
/// Core 用 `--pretty=…%x1f%D`（`git/history.rs:302`），`%D` 是逗号分隔的引用名，例如
/// `HEAD -> main, origin/main, tag: v1.0.0`。种类判定同 Windows 的四档
/// （`git-graph-row.tsx:13-24`）：`HEAD -> x` → head + branch；`tag: x` → tag；
/// 含 `/` → remote；其余 → branch。
///
/// **偏差（已登记）**：`%D` 不带种类信息，名字里含 `/` 的**本地**分支（如 `fix/order`）会被判成
/// remote；要判准得把 `git.references` 的远端名单一起传进来。
pub(crate) fn parse_decorations(value: &str) -> Vec<Label> {
    let mut labels = Vec::new();

    for part in value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some(branch) = part.strip_prefix("HEAD -> ") {
            labels.push(Label {
                title: SharedString::from("HEAD"),
                kind: LabelKind::Head,
            });
            labels.push(Label {
                title: SharedString::from(branch.to_string()),
                kind: LabelKind::Branch,
            });
            continue;
        }

        if let Some(tag) = part.strip_prefix("tag: ") {
            labels.push(Label {
                title: SharedString::from(tag.to_string()),
                kind: LabelKind::Tag,
            });
            continue;
        }

        let (title, kind) = if part == "HEAD" {
            (part, LabelKind::Head)
        } else if part.contains('/') {
            (part, LabelKind::Remote)
        } else {
            (part, LabelKind::Branch)
        };
        labels.push(Label {
            title: SharedString::from(title.to_string()),
            kind,
        });
    }

    labels
}

/// 解析 `git.historyPage` 的 `commits[]`（`contracts.rs:602-611`）。
pub(crate) fn parse_commits(data: &serde_json::Value) -> Vec<Commit> {
    data.get("commits")
        .and_then(serde_json::Value::as_array)
        .map(|commits| {
            commits
                .iter()
                .filter_map(|commit| {
                    Some(Commit {
                        hash: SharedString::from(commit.get("hash")?.as_str()?),
                        short_hash: SharedString::from(
                            commit
                                .get("shortHash")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        parent_hashes: commit
                            .get("parentHashes")
                            .and_then(serde_json::Value::as_array)
                            .map(|parents| {
                                parents
                                    .iter()
                                    .filter_map(serde_json::Value::as_str)
                                    .map(SharedString::from)
                                    .collect()
                            })
                            .unwrap_or_default(),
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
                        email: SharedString::from(
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
                        labels: parse_decorations(
                            commit
                                .get("decorations")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 一页历史的公共解析：`(commits, nextCursor, hasMore)`。
pub(crate) fn parse_history_page(data: &serde_json::Value) -> (Vec<Commit>, Option<String>, bool) {
    let commits = parse_commits(data);
    let next_cursor = data
        .get("nextCursor")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let has_more = data
        .get("hasMore")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(next_cursor.is_some());
    (commits, next_cursor, has_more)
}

/// 读一页提交：`git.historyPage { root, reference, order, cursor, limit }`。
///
/// `order: "date"`（`git/history.rs:80-82` 的 `HistoryOrder::Date`，IDEA 的 Normal 排序）；
/// `reference` / `order` 必须与创建 session 时一致，否则 Core 返回 `invalidCursor`
/// （`git/history.rs:228-235`）。
pub(crate) fn read_history_page(
    root: &str,
    reference: Option<&str>,
    cursor: Option<&str>,
) -> Result<(Vec<Commit>, Option<String>, bool), String> {
    let data = execute_core(
        "git.historyPage",
        serde_json::json!({
            "root": root,
            "reference": reference,
            "order": "date",
            "cursor": cursor,
            "limit": HISTORY_PAGE_LIMIT,
        }),
    )?;
    Ok(parse_history_page(&data))
}

/// 首屏读取：工作区探测 → `git.status` → `git.references` → `git.historyPage`。
///
/// 非仓库时**不再白跑**后面三条（约束 2）。`old_cursor` 是上一次持有的游标，进函数第一件事就是
/// 把它还回去，免得新 session 建起来后旧 `git log` 进程还挂着。
pub(crate) fn load_first(
    root: &Path,
    reference: Option<String>,
    old_cursor: Option<HistoryCursor>,
) -> FirstLoad {
    if let Some(cursor) = old_cursor {
        cursor.close();
    }

    let mut load = FirstLoad {
        repository_root: None,
        branch: None,
        references: Vec::new(),
        commits: Vec::new(),
        next_cursor: None,
        has_more: false,
        failures: Vec::new(),
    };

    let root_text = root.to_string_lossy().to_string();

    // 1) git.status：仓库根 + 当前分支。
    match read_status(&root_text) {
        Ok((repository_root, branch)) => {
            load.repository_root =
                repository_root.map(|found| resolve_repository_root(root, &found));
            load.branch = branch;
        }
        Err(code) => load.failures.push(format!("git.status（{code}）")),
    }

    if load.repository_root.is_none() {
        // 不是仓库：用工作区快照的一级目录再找一次（有上限），仍找不到就走「未打开仓库」。
        load.repository_root = discover_repository_root(root);
    }

    let Some(repository_root) = load.repository_root.clone() else {
        return load;
    };

    // 2) git.references：引用树。
    match execute_core(
        "git.references",
        serde_json::json!({ "root": repository_root.clone() }),
    ) {
        Ok(data) => load.references = parse_references(&data),
        Err(code) => load.failures.push(format!("git.references（{code}）")),
    }

    // 3) git.historyPage：第一页。
    match read_history_page(&repository_root, reference.as_deref(), None) {
        Ok((commits, next_cursor, has_more)) => {
            load.commits = commits;
            load.next_cursor = next_cursor;
            load.has_more = has_more;
        }
        Err(code) => load.failures.push(format!("git.historyPage（{code}）")),
    }

    load
}

/// 追加一页：`git.historyPage` 带上游标继续读同一个 `git log` 流。
///
/// 成功时**不关游标**（session 还在，`nextCursor` 与传入的游标是同一个串，`git/history.rs:244`）；
/// 失败时 Core 已经 `session.stop()`（`:258-261`），游标作废。
pub(crate) fn load_more(root: &str, reference: Option<String>, cursor: &str) -> MoreLoad {
    match read_history_page(root, reference.as_deref(), Some(cursor)) {
        Ok((commits, next_cursor, has_more)) => MoreLoad {
            commits,
            next_cursor,
            has_more,
            failure: None,
        },
        Err(code) => MoreLoad {
            commits: Vec::new(),
            next_cursor: None,
            has_more: false,
            failure: Some(code),
        },
    }
}

/// 读一个提交的文件列表：`git.commitFiles { root, commit }`（`git/mod.rs:2013-2038`）。
///
/// 返回 `(status, path)` 对，由 [`build_commit_files`] 聚成多层目录树。
pub(crate) fn load_commit_files(root: &str, commit: &str) -> Result<Vec<(String, String)>, String> {
    let data = execute_core(
        "git.commitFiles",
        serde_json::json!({ "root": root, "commit": commit }),
    )?;

    let files = data
        .get("files")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "响应缺少 files".to_string())?;

    Ok(files
        .iter()
        .filter_map(|file| {
            let path = file.get("path")?.as_str()?;
            let status = file
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("M");
            (!path.is_empty()).then(|| (status.to_string(), path.to_string()))
        })
        .collect())
}

// ---------------------------------------------------------------------------
// 纯函数：泳道布局 / 引用树 / 文件树 / 过滤
// ---------------------------------------------------------------------------

/// 泳道颜色下标 → 主题色（偏差 3）。
pub(crate) fn lane_color(index: usize, cx: &App) -> Hsla {
    let theme = cx.theme();
    match index % 6 {
        0 => theme.success,
        1 => theme.blue_light,
        2 => theme.magenta_light,
        3 => theme.warning,
        4 => theme.danger,
        _ => theme.cyan_light,
    }
}

/// 标签四档的颜色（偏差 3）。
pub(crate) fn label_color(kind: LabelKind, cx: &App) -> Hsla {
    let theme = cx.theme();
    match kind {
        LabelKind::Head => theme.cyan_light,
        LabelKind::Remote => theme.blue_light,
        LabelKind::Tag => theme.yellow_light,
        LabelKind::Branch => theme.green_light,
    }
}

/// 简化泳道布局（模块文档偏差 2）。
///
/// 规则与 `git-graph-layout.ts` 同构：每条泳道记住「在等哪个提交」；本提交进到自己那条泳道，
/// 第一父提交留在原泳道，其余父提交塞进空泳道（没有空位就新开一条）；行尾回收尾部空泳道。
/// 颜色按泳道下标取 6 色循环，跨泳道父边只画目标泳道的竖直段。
pub(crate) fn layout_graph(commits: &[Commit]) -> Vec<GraphRow> {
    let known: BTreeSet<&SharedString> = commits.iter().map(|commit| &commit.hash).collect();
    let mut lanes: Vec<Option<SharedString>> = Vec::new();
    let mut rows = Vec::with_capacity(commits.len());

    for commit in commits {
        // 本提交在哪条泳道：等它的那条；没有就是新开一条。
        let lane = match lanes
            .iter()
            .position(|expected| expected.as_ref() == Some(&commit.hash))
        {
            Some(lane) => lane,
            None => {
                lanes.push(None);
                lanes.len() - 1
            }
        };

        // 入线：本行上半部分经过的泳道。
        let mut lane_colors: Vec<Option<usize>> = lanes
            .iter()
            .enumerate()
            .map(|(index, expected)| expected.as_ref().map(|_| index % 6))
            .collect();
        let node_color = lane_colors.get(lane).copied().flatten().unwrap_or(lane % 6);

        // 出线：把父提交排进泳道，再按结果画下半段。
        let mut edges: Vec<(usize, usize, bool)> = Vec::new();
        for (position, parent) in commit.parent_hashes.iter().enumerate() {
            let target = if position == 0 {
                lane
            } else if let Some(existing) = lanes
                .iter()
                .position(|expected| expected.as_ref() == Some(parent))
            {
                existing
            } else if let Some(free) = lanes.iter().position(Option::is_none) {
                free
            } else {
                lanes.push(None);
                lanes.len() - 1
            };

            let missing = !known.contains(parent);
            edges.push((target, target % 6, missing));
            lanes[target] = Some(parent.clone());
            while lane_colors.len() <= target {
                lane_colors.push(None);
            }
        }
        if commit.parent_hashes.is_empty() {
            lanes[lane] = None;
        }

        // 跨泳道父边要在目标泳道的下半部分补一段线，所以泳道数要覆盖到最远的边。
        if let Some((widest, _, _)) = edges.iter().max_by_key(|(lane, _, _)| *lane) {
            while lane_colors.len() <= *widest {
                lane_colors.push(None);
            }
        }

        rows.push(GraphRow {
            lanes: lane_colors,
            lane,
            node_color,
            edges,
        });

        while lanes.last().is_some_and(|lane| lane.is_none()) {
            lanes.pop();
        }
    }

    rows
}

/// 引用树的一个节点（Windows `buildGitReferenceTree` 的目录分组）。
struct RefNode {
    /// 这一段的名字（目录名或引用短名）。
    name: SharedString,
    /// 分组 id（`"<kind>/<path>"`），用于折叠状态。
    id: String,
    /// 叶子节点指向 `BottomPane::references` 的下标。
    reference: Option<usize>,
    children: Vec<RefNode>,
}

/// 摊平后的一行。
pub(crate) struct RefRow {
    pub(crate) depth: usize,
    pub(crate) name: SharedString,
    pub(crate) id: String,
    pub(crate) reference: Option<usize>,
}

/// 把一类引用按 `/` 分组（顺序保持 Core 给的顺序，Core 已保证确定性）。
pub(crate) fn build_reference_rows(references: &[Reference], kind: RefKind) -> Vec<RefRow> {
    let mut roots: Vec<RefNode> = Vec::new();

    for (index, reference) in references.iter().enumerate() {
        if reference.kind != kind {
            continue;
        }

        let segments: Vec<&str> = reference
            .short_name
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        let Some((leaf, directories)) = segments.split_last() else {
            continue;
        };

        let mut path = String::new();
        let mut level = &mut roots;
        for directory in directories {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(directory);
            let id = format!("{}/{}", kind.id(), path);
            let position = match level.iter().position(|node| node.id == id) {
                Some(position) => position,
                None => {
                    level.push(RefNode {
                        name: SharedString::from((*directory).to_string()),
                        id: id.clone(),
                        reference: None,
                        children: Vec::new(),
                    });
                    level.len() - 1
                }
            };
            level = &mut level[position].children;
        }

        let mut leaf_path = path.clone();
        if !leaf_path.is_empty() {
            leaf_path.push('/');
        }
        leaf_path.push_str(leaf);
        level.push(RefNode {
            name: SharedString::from((*leaf).to_string()),
            id: format!("{}/{}", kind.id(), leaf_path),
            reference: Some(index),
            children: Vec::new(),
        });
    }

    let mut rows = Vec::new();
    flatten_reference_nodes(&roots, 0, &mut rows);
    rows
}

fn flatten_reference_nodes(nodes: &[RefNode], depth: usize, rows: &mut Vec<RefRow>) {
    for node in nodes {
        rows.push(RefRow {
            depth,
            name: node.name.clone(),
            id: node.id.clone(),
            reference: node.reference,
        });
        flatten_reference_nodes(&node.children, depth + 1, rows);
    }
}

/// 提交文件树的中间节点（Windows `GitCommitFileTreeNode.build`）。
#[derive(Default)]
struct FileNode {
    dirs: Vec<(String, FileNode)>,
    files: Vec<(String, String)>,
}

impl FileNode {
    fn insert(&mut self, path: &str, status: &str) {
        let segments: Vec<&str> = path.split('/').collect();
        let Some((name, directories)) = segments.split_last() else {
            return;
        };

        let mut node = self;
        for directory in directories {
            let position = match node.dirs.iter().position(|(key, _)| key == directory) {
                Some(position) => position,
                None => {
                    node.dirs
                        .push(((*directory).to_string(), FileNode::default()));
                    node.dirs.len() - 1
                }
            };
            node = &mut node.dirs[position].1;
        }
        node.files.push(((*name).to_string(), status.to_string()));
    }

    fn file_count(&self) -> usize {
        self.files.len()
            + self
                .dirs
                .iter()
                .map(|(_, child)| child.file_count())
                .sum::<usize>()
    }

    fn flatten(&self, depth: usize, rows: &mut Vec<CommitFileRow>) {
        for (name, child) in &self.dirs {
            rows.push(CommitFileRow {
                depth,
                name: SharedString::from(name.clone()),
                is_folder: true,
                file_count: child.file_count(),
                status: SharedString::from(""),
            });
            child.flatten(depth + 1, rows);
        }

        for (name, status) in &self.files {
            rows.push(CommitFileRow {
                depth,
                name: SharedString::from(name.clone()),
                is_folder: false,
                file_count: 0,
                status: SharedString::from(status.clone()),
            });
        }
    }
}

/// 把 `git.commitFiles` 的 `(status, path)` 聚成多层目录树。
pub(crate) fn build_commit_files(files: &[(String, String)]) -> Vec<CommitFileRow> {
    let mut root = FileNode::default();
    for (status, path) in files {
        root.insert(path, status);
    }

    let mut rows = Vec::new();
    root.flatten(0, &mut rows);
    rows
}

/// 提交过滤。逐字照 `features/git/utils/git-log-filter.ts:4-19`。
pub(crate) fn matches_filter(commit: &Commit, query: &str, scope: FilterScope) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }

    match scope {
        FilterScope::Author => contains(&commit.author, &query) || contains(&commit.email, &query),
        FilterScope::Branch => commit
            .labels
            .iter()
            .any(|label| contains(&label.title, &query)),
        FilterScope::Text => {
            contains(&commit.subject, &query)
                || contains(&commit.hash, &query)
                || contains(&commit.short_hash, &query)
        }
    }
}

pub(crate) fn contains(value: &SharedString, query: &str) -> bool {
    value.to_lowercase().contains(query)
}

/// ahead / behind 计数，上限 [`TRACKING_COUNT_MAX`] → `99+`
/// （`git-tracking-counts.tsx:9-14`）。
pub(crate) fn tracking_count(value: usize) -> SharedString {
    if value > TRACKING_COUNT_MAX {
        SharedString::from(format!("{TRACKING_COUNT_MAX}+"))
    } else {
        SharedString::from(value.to_string())
    }
}

// ---------------------------------------------------------------------------
// 面板
