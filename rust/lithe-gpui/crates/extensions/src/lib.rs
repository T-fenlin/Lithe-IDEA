//! 扩展目录的状态层（扩展页的数据源）。
//!
//! ## 分层
//!
//! - [`model`]：目录里一个扩展条目的展示快照（`plugins.listCatalog` 的响应 →
//!   [`ExtensionSnapshot`]），以及搜索 / 分类过滤的纯函数。
//! - [`core_bridge`]：调用 Core 的 `plugins.listCatalog` 并把响应解析成快照。
//!
//! ## 本 crate 不做什么
//!
//! - **不渲染。** 没有任何 `impl IntoElement`。扩展页视图在
//!   `lithe-gpui/crates/workbench/src/extensions_page.rs`。
//! - **不读时钟、不起进程。** Core 调用是同步的，调用方（workbench 的后台任务）
//!   负责放到非 UI 线程执行。
//! - **不校验目录。** 兼容性与形状校验真源在 `lithe-core::plugins`；本 crate 只做
//!   展示归一（如字体家族去重），解析失败按字符串错误返回给展示层。
//!
//! ## 为什么独立成一个 crate
//!
//! 与 `notify` 同一逻辑：`workbench` 依赖全部 feature crate，而 feature 只依赖
//! `shared`。扩展的数据模型 + Core 桥放 `workbench` 会让它继续膨胀；独立成
//! `lithe-gpui-extensions`（压在 `shared` 之上）后依赖方向不变：
//! `workbench -> extensions -> shared`。

mod core_bridge;
mod model;

pub use core_bridge::load_catalog;
pub use model::{
    CATEGORIES, CatalogSnapshot, ExtensionSnapshot, LanguageSupportSnapshot, matches_category,
    matches_query,
};
