//! 语言高亮的**启用状态**：哪些语言因为所属扩展被卸载而不再高亮。
//!
//! ## 为什么放在 shared
//!
//! 写它的是设置存储（`settings → shared` 是允许的方向），读它的是编辑器
//! （`editor → shared` 同样允许）。两侧都只依赖 `shared`，谁也不需要依赖对方 ——
//! 与图标主题的"生效包"（`icons::file_icon::set_active_icons`）同一条口径。
//!
//! ## 语义
//!
//! 集合里的是**语言 id**（`yaml` / `toml` / `xml` …，与目录契约里
//! `languageSupports[].id` 同一套取值），而不是文件扩展名：按扩展名判会漏掉
//! 无扩展名的入口（`mvnw` / `gradlew` 走文件名分支映射到 `bash`）。
//!
//! 未设置过（启动极早期）时**什么都不禁用** —— 默认是"宿主内置的语言都能高亮"。

use std::sync::Mutex;
use std::sync::OnceLock;

/// 被禁用的语言 id 集合（进程内一份）。
static DISABLED: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

fn cell() -> &'static Mutex<Vec<String>> {
    DISABLED.get_or_init(|| Mutex::new(Vec::new()))
}

/// 设置被禁用的语言（排序去重后存）。调用方是设置存储。
pub fn set_disabled_languages(languages: &[String]) {
    let mut normalized: Vec<String> = languages
        .iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    normalized.sort();
    normalized.dedup();
    let mut guard = cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = normalized;
}

/// 这个语言现在被禁用了吗（被禁用的语言不该设进编辑器，文件按纯文本打开）。
#[must_use]
pub fn is_language_disabled(language: &str) -> bool {
    cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .any(|id| id == language)
}

/// 当前被禁用的语言（诊断用；排序去重后的快照）。
#[must_use]
pub fn disabled_languages() -> Vec<String> {
    cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}
