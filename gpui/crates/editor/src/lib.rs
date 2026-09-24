//! 编辑区 Feature：**标签栏 + 正文 + 空状态**。
//!
//! ## 职责
//!
//! 对应 Windows 前端的 pane / tab-bar 结构：上面一条常驻标签栏，下面是正文区；
//! 没有活动 buffer 时标签栏仍在，只有正文位置换成空状态。一个打开的 buffer 对应
//! 一个独立的 `EditorState`，打开 / 切换 / 关闭都由 [`EditorPane`] 持有。
//!
//! ## 拆分来源
//!
//! 从 `gpui/shell/src/bin/shell_probe/editor.rs`（833 行，单个 bin crate 里的扁平模块）
//! 逐字拆出，只调整可见性与 import：**不改**布局、尺寸、颜色、文案、交互与数据接线。
//! 下一节是该文件原来的文件头，整体原样搬进本模块文档。
//!
//! ## 文件分工
//!
//! - `buffer.rs`：`Buffer`（打开的文件 + 它的正文状态）与读盘（`read_body` / `notice`）、
//!   文件类型图标（`icon_for_file`）、标签显示名（`display_names` 等 path-shortener 的同名区分）；
//! - `editor_view.rs`：`EditorPane` 的结构体与字段、`new` / `open` / `close`、
//!   全部 `render_*` 与 `impl Render for EditorPane`，以及界面用的度量常量与文案。
//!
//! ## 公开边界
//!
//! 只有 [`EditorPane`]（`new` / `open`）是公开 API，由本文件 `pub use` 发布；
//! `Buffer` 与两个实现模块都留在 crate 内部（`mod buffer; mod editor_view;`），
//! 不出现在 import 路径里（《编码指南》「内部重组时保持 public module path」的反面用法：
//! 新建 crate 直接发布 `pub use`，不让 `buffer` / `editor_view` 这类实现路径变成契约）。
//!
//! ## 原 `editor.rs` 文件头（逐字保留）
//!
//! 编辑区：**标签栏 + 正文 + 空状态**。
//!
//! 形状照 Windows 前端的 `windows/tauri/src/features/panes/components/pane-container.tsx:1094-1100`：
//! 上面一条标签栏、下面是 `relative min-h-0 flex-1 overflow-hidden` 的正文区；
//! **没有活动 buffer 时标签栏仍在**，只有正文位置换成空状态
//! （`windows/tauri/src/features/panes/components/pane-container.tsx:1100` →
//! `windows/tauri/src/features/panes/components/empty-editor-state.tsx`）。
//!
//! 界面规格来源是 Windows 前端（`gpui/UI-MAP.md` 顶部横幅），
//! 观感用 gpui-kit 0.6.6 真实存在的组件与主题 token 实现，不逐层翻译 Tailwind class：
//!
//! > 引用约定：Windows 侧路径一律写全（相对仓库根）；gpui-kit 侧的行号相对
//! > `gpui-component-0.6.6/src/`（`base::` 的少量引用相对 `gpui-base-0.6.6/src/`），
//! > 两类根目录见 `gpui/UI-MAP.md` §1.1 第 7 条。
//!
//! - 标签条 = `component::tab::TabBar` 的 `TabVariant::Underline`
//!   （Windows 活动标签是"透明底 + 底边主色条"= IntelliJ 风格，
//!   `windows/tauri/src/ui/tab-bar.tsx:204-209`）；
//! - 标签 = `Tab::child(...)`（**不是** `Tab::icon()`，后者会丢掉 label）；
//! - 空状态 = `component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription}`；
//! - 正文 = `component::input::Editor`（`EditorState` 每个标签一份，对应真机"一个 buffer 一个
//!   Monaco model"）。
//!
//! 本轮**明确没做**的两件事（都只做外观/位置，行为留待后续接线）：
//!
//! 1. **脏标记（未保存圆点）恒为干净**：`Buffer::is_dirty` 永远写不进去
//!    （没有编辑事件订阅），所以圆点不会出现；圆点的位置与颜色按规格写在
//!    [`EditorPane::render_tab`] 里，接上 `InputEvent` 后把它改成真实值即可；
//! 2. **`← →` 只有外观 + 禁用态**：探针里没有 jump list（历史栈），
//!    两个按钮恒为 `disabled(true)`，见 [`EditorPane::nav_button`]。
//!
//! 还有几处**组件写死、公开 API 改不动**的尺寸偏差（细节见交付报告）：
//! `TabVariant::Underline` 的标签高度是 36px（真机标签 28px 居中在 36px 条里，
//! `tab/tab.rs:38-41`）、标签字号默认 14px（真机 `--ui-text-chrome` 是 13px，
//! `tab/tab.rs:803-807` 按 `Size` 档位给字号）、标签之间的间距写死 16px
//! （真机 `--lithe-chrome-gap` 是 4px；`tab/tab_bar.rs:393-403` 只把这个 gap 给
//! 内部的 `tabs-inner`，`TabBar` 没有公开的 gap setter，`.gap()` 设的是外层容器）。
//! 这些值都在组件自己的 `render` 里最后写入，外层 `.h()` / `.text_size()` 覆盖不掉
//! （`Tab::style()` 与组件自己写的字段是同一个 `StyleRefinement`，后写的赢）。
//!
//! 本轮**范围外**（真机有、这里没有）还有：标签悬停才显示关闭按钮的那一档
//! （`TabBar` 不暴露每个标签的悬停状态）、标签拖拽重排 / 拖出成新窗格、
//! 标签右键菜单、面包屑栏、查找替换、分屏与轮播、外部冲突横幅与大文件「仍然启用」降级、
//! 状态栏的光标位置/编码/缩进等项（那些属于外壳的状态栏区域，不在编辑区里）。

mod buffer;
mod editor_view;

pub use editor_view::EditorPane;
