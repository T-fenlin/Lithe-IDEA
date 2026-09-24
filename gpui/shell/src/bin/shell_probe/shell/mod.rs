//! 外壳的区域模块（阶段 1）。
//!
//! 一个区域一个文件，互不冲突，便于并行改动：
//!
//! - [`title_bar`]：标题栏（40px）+ 自绘窗口三键；
//! - [`project_tabs`]：项目标签条；
//! - [`activity_bar`]：左右活动栏（38px）；
//! - [`status_bar`]：状态栏（24px）。
//!
//! 这四个区域都是**无状态渲染函数**（`-> impl IntoElement`），状态由
//! [`super::workspace::ShellWorkspace`] 持有并通过参数传进来 —— 这样区域文件不需要
//! 各自持有 `Entity`，组装与接线集中在 `workspace.rs` 一处。
//!
//! 可见性约定：区域模块本身保持私有，只在 `shell/mod.rs` 这一层用 `pub(super) use` 重新导出
//! （即"私有 `mod` + 受控 `pub use`"的门面写法），对外只暴露 `shell_probe` 这一层。
//! 区域文件里被重导出的项因此要写 `pub`，**不能**写 `pub(super)` —— 后者只在 `shell` 内可见，
//! 再往上一层重导出会被 rustc 判为"放大可见性"（E0365 一族）。
//!
//! ## ⚠️ 为什么这些渲染函数收 `&Window` / `&App`，而不是 `&mut`
//!
//! 本 crate 是 **edition 2024**，而 edition 2024 的 RPIT 规则会**捕获签名里所有在作用域的
//! 生命周期**（2021 及以前只捕获出现在 `impl Trait` 里的那些）。于是：
//!
//! - 参数写成 `&mut Window` / `&mut App` 时，返回的 `impl IntoElement` 会捕获这个**可变借用**；
//! - 同一个表达式里连续调用两次（例如"左右两条活动栏"共用 `cx`）就报
//!   `E0499 cannot borrow *cx as mutable more than once at a time`。
//!
//! 本项目实测：第一次构建时这一条一次报了 **12 个** E0499/E0502。修法不是拆表达式，而是
//! 让这些函数收 `&Window` / `&App` —— 它们本来就只读主题色与窗口状态，不需要可变借用，
//! 而不可变借用可以同时存在。**新增区域模块请遵守这条**。

mod activity_bar;
mod project_tabs;
mod status_bar;
mod title_bar;

pub(super) use activity_bar::{ActivityItem, ActivitySide, activity_bar};
pub(super) use project_tabs::{ProjectTab, project_tabs};
pub(super) use status_bar::{StatusEntry, status_bar};
pub(super) use title_bar::title_bar;
