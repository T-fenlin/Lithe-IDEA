use std::sync::Mutex;
use std::sync::OnceLock;

static DISABLED: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

fn cell() -> &'static Mutex<Vec<String>> {
    DISABLED.get_or_init(|| Mutex::new(Vec::new()))
}

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

#[must_use]
pub fn is_language_disabled(language: &str) -> bool {
    cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .any(|id| id == language)
}

#[must_use]
pub fn disabled_languages() -> Vec<String> {
    cell()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}
