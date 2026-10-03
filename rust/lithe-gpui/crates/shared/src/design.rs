//! 跨 Feature 的**设计规格常量**：同一个设计量被两个以上视图消费时，只在这里定义一次。
//!
//! 判据（上游《设计指南》`docs/gpui kit/0.7.0/docs/design-guides.md:186,188`，§「对齐细节」）：
//! 「精确对齐与重复 gap 是质量 invariant…相等关系应来自同一个 `rem` helper、spacing token、
//! grid definition 或 shared component inset；发生偏差时修复共同 owner」。
//! 两个 crate 各自硬写一份**字面相同**的数字不构成相等关系 —— 值相同只是巧合，任一侧被改动
//! 就会静默分叉，而「差 1 个 rendered pixel 也是 defect」。本模块是这些值的共同 owner。
//!
//! 准入与 crate 根文档同一条：只收「已有清晰名称 + 两个以上真实使用方」的常量（本模块三条
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
