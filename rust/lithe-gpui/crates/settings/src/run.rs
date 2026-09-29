//! 设置「运行配置」页的数据层 —— **模型与取数都在 `lithe-gpui-run`**。
//!
//! 这一页从 `crate::run` 起家的（2026-09-30 之前），运行工具窗（`lithe_gpui_run::RunPane`）
//! 需要同一份配置列表，于是模型与解析**整体搬到那个 crate**，这里只留重导出：
//!
//! - 两个消费者读的是**同一份** `RunProjectView`（一条解析、一处兜底口径），
//!   不会出现"设置页说 3 条、运行工具窗说 2 条"；
//! - 依赖方向是 `settings -> lithe-gpui-run -> shared`：`run` 不认识设置，
//!   设置也不碰运行工具窗的实体（它只读模型）。
//!
//! 页面本身（列表、重新识别按钮、说明文案）仍在 `crate::dialog`。

pub use lithe_gpui_run::{
    MAX_PATHS, RUN_DIAGNOSTIC_TAG, RunConfigurationView, RunExecution, RunLayer, RunProjectView,
    load, parse_generate, parse_snapshot, provider_title,
};
