mod core_bridge;
mod model;
mod severity;
mod store;

pub use core_bridge::{from_core_error, is_reportable};
pub use model::{Entry, EntryId, MAX_ENTRIES, Target, matches_text};
pub use severity::{Badge, Severity};
pub use store::{Input, RecordOutcome, Store};

pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn now_ms_does_not_go_backwards() {
        let first = super::now_ms();
        let second = super::now_ms();
        assert!(second >= first, "墙钟不许回退：{second} < {first}");
    }
}
