//! 跨 Feature 的**设计规格常量**：同一个设计量被两个以上视图消费时，只在这里定义一次。
//!
//! 判据（上游《设计指南》`docs/gpui kit/0.7.0/docs/design-guides.md:186,188`，§「对齐细节」）：
//! 「精确对齐与重复 gap 是质量 invariant…相等关系应来自同一个 `rem` helper、spacing token、
//! grid definition 或 shared component inset；发生偏差时修复共同 owner」。
//! 两个 crate 各自硬写一份**字面相同**的数字不构成相等关系 —— 值相同只是巧合，任一侧被改动
//! 就会静默分叉，而「差 1 个 rendered pixel 也是 defect」。本模块是这些值的共同 owner。
//!
//! 准入与 crate 根文档同一条：只收「已有清晰名称 + 两个以上真实使用方」的常量（本模块五条
//! 各有 2 个使用方）。值是**规格像素值**（`f32`），消费方式仍走 rem —— 能吃 `AbsoluteLength`
//! 的调用点写 `rems(C / 16.)`，只吃 `Pixels` 的固有方法走 [`crate::rem_px`]（1 rem = 16px，
//! 不是 4px）。放在宿主侧 `shared` 而不进 `lithe-core`：它们是界面规格，不是确定性业务语义。

/// 列表行 / 树的选中面圆角 **4**。
///
/// 真源（macOS 界面规格）：`LitheTheme.Metrics.projectTreeSelectionCornerRadius = 4`
/// （`macos/Sources/Lithe/Theme/LitheTheme.swift:474`）。项目树行用这个命名 token
/// （`ProjectSidebarView.swift:497,545`），Git 更改列表行的选中面用同一个值
/// （`ChangesSidebarView.swift:818` 的 `RoundedRectangle(cornerRadius: 4)`）。
///
/// 使用方：`explorer`（项目树行）、`git`（更改列表行 + 分类头计数徽章）。
/// 注意它**不是** `--radius × k` 那一族的成员（`rounded-md` 是 6.4），调用点不要互换。
pub const ROW_RADIUS: f32 = 4.;

/// 列表行行高倍数 **1.35** —— 是**倍数**不是长度，调用点写 `line_height(relative(..))`。
///
/// 真源：`--leading-row: 1.35`。两条使用方在本仓库的注释里此前各自引用过同一个 token
/// （`theme.css:4`）；macOS 侧没有等价的命名倍数（`LitheTheme.Metrics` 只给像素行高），
/// 所以这一条的「同源」依据是两处引用的是同一个 token，而不是 macOS 复核。
///
/// 使用方：`explorer`（项目树行）、`git`（更改列表行）。
pub const ROW_LINE_HEIGHT: f32 = 1.35;

/// 单个页签最大宽 **200**。
///
/// 真源：`ui/tab-bar.tsx:257-267`（编辑器页签与终端页签共用同一条页签规格；200 不在 gpui 的
/// 固定 rem 档位上 —— 档位里 48 → 192、56 → 224）。
///
/// 消费方式：`TabBar::max_width(impl Into<Pixels>)` **只吃 `Pixels`**，所以两处调用点都走
/// [`crate::rem_px`]（= `rems(200 / 16.)`），按当帧 rem 基准求值。
/// 使用方：`editor`（编辑器页签）、`terminal`（终端页签）。
pub const TAB_MAX_WIDTH: f32 = 200.;

/// 控件圆角 **6.4** —— Lithe 圆角阶梯里的 `rounded-md`（`--radius × 0.8` = `8 × 0.8`）。
///
/// 判据同模块头：这条值此前由 **7 处**各自硬写一份字面量 `6.4`（`workbench` 的
/// `title_bar.rs` / `project_menu.rs` / `branch_panel.rs` / `menu_bar.rs` / `status_bar.rs` /
/// `right_tool_window.rs` 六个具名常量 + `git/src/model.rs` 的 `RADIUS_MD`），
/// 值相同只是巧合，任一侧被改动就会静默分叉。现在**只在这里写一次**；各处保留自己的具名
/// 常量作为「锚定调用点」的别名（`const X: f32 = design::CONTROL_RADIUS;`），
/// 名字仍说明它用在哪一处，数字不再重复。
///
/// ⚠️ 6.4 **不是** gpui 的 rem 档位（`rounded_md()` 是 6，
/// `gpui-pre-macros-0.3.7/src/styles.rs`），也不能从主题读 —— `ThemeConfig.radius` 是
/// `usize`，装不下 6.4；而把主题半径调成 8 会让**所有** gpui-kit 组件的圆角从 6 变成 8，
/// 离 6.4 反而更远。`Styled::rounded` 收 `impl Into<AbsoluteLength>`，所以能写 `rems(..)`
/// 的调用点写 `rems(CONTROL_RADIUS / 16.)`；`Button::rounded` 这类只吃 `Pixels` 的固有方法
/// 走 [`crate::rem_px`]。
///
/// 使用方：`workbench` 的六个区域（上述六个具名常量各自 `= CONTROL_RADIUS`）。
///
/// ⚠️ **残留一处重复，未合并**：`git/src/model.rs` 的 `RADIUS_MD` **数值相同（也是 6.4）但仍是
/// 它自己 `--radius × k` 阶梯里的独立定义**（`RADIUS_BASE × 0.8`），并不引用本条。没合成一条是因为
/// `git` 那一族要保住"整族从同一个基准派生"（`SM` / `MD` / `XL` 三条都由 `RADIUS_BASE` 算出），
/// 而本条服务的是"跨 crate 的同一个 6.4"。**要不要让其中一侧引用另一侧，连同下面这条冲突，
/// 留给维护者裁决**（`D23` 已登记）：macOS `LitheTheme.swift:484` 的 `controlCornerRadius = 6`
/// 与 Lithe 阶梯的 `6.4` **规格本身就不一致**，`settings` 又走第三条路（`cx.theme().radius` = 6px 且
/// 是固定 `Pixels`、不随 rem 缩放）—— 三处口径没统一之前，**不要**擅自把任一处的数字改成另一处。
pub const CONTROL_RADIUS: f32 = 6.4;

/// 活动栏 rail 宽 **40** —— 真源里左右两条 rail **各自**都是这个值。
///
/// 真源（macOS 界面规格）出处两条，各自对应一条 rail（复核时**补正**了 `BLOCKERS.md` B14 原记的
/// 「真源只定义右侧」—— 真源左侧也有一条 40）：
///
/// - **左 / leading**：`macos/Sources/Lithe/Views/Workbench/WorkbenchView.swift:12`
///   `static let width: CGFloat = 40`（类型是 `:11` 的 `ActivityBarMetrics`）—— 由 `:1263` 的
///   `activityBar`（`:257` 那个 `HStack` 的**首个子视图** ⇒ 左侧）在 `:1342` / `:1345`
///   以 `.frame(width: ActivityBarMetrics.width)` 使用；
/// - **右 / trailing**：同文件 `:7`
///   `static let rightActivityBarWidth: CGFloat = 40`（类型是 `WorkbenchLayoutMetrics`）——
///   由 `:13` 的 `ActivityBarMetrics.rightWidth` 转发、在 `:1415` 的 `.frame(width:)` 使用
///   （`:264` 的 `pluginActivityBar` 挂 `.overlay(alignment: .trailing)` ⇒ 右侧），
///   `:553` 的通知 trailing 内衬 `rightActivityBarWidth + 12` 也用它。
///
/// **宿主侧只有一条常量管两侧**（`crates/workbench/src/activity_bar.rs` 的 `COLLAPSED_WIDTH`），
/// 所以维护者 2026-10-03 裁决（`BLOCKERS.md` B14 选项 (a)）：承认宿主"左右同宽"，把原值
/// **38 对齐到本条的 40**。改前的 38 出自**已作废**的 `windows/tauri` 前端
/// （`main-sidebar.tsx:97`，与 `38 + 4 gap = 42` 一起进来）—— **本条出处不再是它**；
/// 而 40 在真源里对**两条** rail 都成立，所以不需要给左侧另定值。
///
/// 使用方（引用方只写一次，不再各写一份数字）：
/// `workbench` 的 `activity_bar`（`COLLAPSED_WIDTH` = 本条；左轨盒宽 = 本条 + `WORKBENCH_GAP`、
/// 右轨盒宽 = 本条）、`settings` 的通知右内衬（`NOTIFICATION_MARGIN_RIGHT_SPEC` = 本条 + 12，
/// 照真源 `:553` 的算式）。
pub const ACTIVITY_RAIL_WIDTH_SPEC: f32 = 40.;

#[cfg(test)]
mod tests {
    use super::*;

    /// 活动栏 rail 宽 = 真源 `WorkbenchView.swift:7`（右）/`:12`（左）的 **40**。
    ///
    /// 钉住的是 owner 的**值**本身：宿主两条 rail 与通知右内衬都从它派生，所以这里被改成别的数
    /// 会同时移动两处可见外观。`settings` 侧另有一条「内衬 = 本条 + 12」的断言
    /// （`crates/settings/src/theme.rs`），两条一起才构成"跨 crate 同源"。
    ///
    /// 纯常量断言：不开窗口、不碰主题、无后台任务。
    #[test]
    fn activity_rail_width_matches_the_macos_source() {
        assert_eq!(ACTIVITY_RAIL_WIDTH_SPEC, 40.);
    }
}
