use gpui::SharedString;

use crate::model::{Entry, EntryId, MAX_ENTRIES, Target};
use crate::severity::{Badge, Severity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordOutcome {
    Inserted,
    Replaced { occurrences: u32 },
}

#[derive(Clone, Debug)]
pub struct Input {
    code: SharedString,
    params: Vec<(SharedString, SharedString)>,
    severity: Severity,
    key: Option<SharedString>,
    target: Option<Target>,
}

impl Input {
    pub fn new(code: impl Into<SharedString>, severity: Severity) -> Self {
        Self {
            code: code.into(),
            params: Vec::new(),
            severity,
            key: None,
            target: None,
        }
    }

    pub fn for_core_code(core_code: &str) -> Option<Self> {
        let severity = Severity::for_core_code(core_code)?;
        Some(
            Self::new(
                Severity::notification_code_for_core_code(core_code),
                severity,
            )
            .param("coreCode", core_code),
        )
    }

    pub fn code(&self) -> &SharedString {
        &self.code
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn params(&self) -> &[(SharedString, SharedString)] {
        &self.params
    }

    pub fn param(mut self, name: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        let name = name.into();
        let value = value.into();
        self.params.retain(|(existing, _)| *existing != name);
        self.params.push((name, value));
        self
    }

    pub fn key(mut self, key: impl Into<SharedString>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn target(mut self, target: Target) -> Self {
        self.target = Some(target);
        self
    }
}

#[derive(Debug)]
pub struct Store {
    entries: Vec<Entry>,
    next_seq: u64,
    read_upto_seq: u64,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            next_seq: 1,
            read_upto_seq: 0,
        }
    }
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, input: Input, now_ms: u64) -> RecordOutcome {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);

        if let Some(key) = input.key.as_ref() {
            if let Some(position) = self
                .entries
                .iter()
                .position(|entry| entry.key() == Some(key))
            {
                self.entries[position].replace_with(
                    input.code,
                    input.params,
                    input.severity,
                    input.target,
                    now_ms,
                    seq,
                );
                let entry = self.entries.remove(position);
                self.entries.insert(0, entry);
                return RecordOutcome::Replaced {
                    occurrences: self.entries[0].occurrences(),
                };
            }
        }

        self.entries.insert(
            0,
            Entry::new(
                seq,
                input.code,
                input.params,
                input.severity,
                input.key,
                input.target,
                now_ms,
                seq,
            ),
        );
        if self.entries.len() > MAX_ENTRIES {
            self.entries.truncate(MAX_ENTRIES);
        }
        RecordOutcome::Inserted
    }

    pub fn clear(&mut self) -> usize {
        let removed = self.entries.len();
        self.entries.clear();
        self.read_upto_seq = 0;
        removed
    }

    pub fn remove(&mut self, id: EntryId) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id() != id);
        self.entries.len() != before
    }

    pub fn mark_all_read(&mut self) {
        self.read_upto_seq = self.next_seq.wrapping_sub(1);
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn badge(&self) -> Badge {
        let mut has_error = false;
        let mut has_other = false;
        for entry in &self.entries {
            if !entry.is_unread(self.read_upto_seq) {
                continue;
            }
            match entry.severity() {
                Severity::Error => has_error = true,
                Severity::Info | Severity::Warning => has_other = true,
            }
        }
        if has_error {
            Badge::Error
        } else if has_other {
            Badge::Info
        } else {
            Badge::None
        }
    }

    pub fn unread_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.is_unread(self.read_upto_seq))
            .count()
    }

    pub fn entries_since(&self, after_seq: u64) -> Vec<Entry> {
        self.entries
            .iter()
            .rev()
            .filter(|entry| entry.seq() > after_seq)
            .cloned()
            .collect()
    }

    pub fn read_upto_seq(&self) -> u64 {
        self.read_upto_seq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Clock(u64);

    impl Clock {
        fn next(&mut self) -> u64 {
            self.0 += 10;
            self.0
        }
    }

    fn input(code: &str, severity: Severity) -> Input {
        Input::new(code, severity)
    }

    #[test]
    fn records_newest_first() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("a", Severity::Info), clock.next());
        store.record(input("b", Severity::Info), clock.next());
        store.record(input("c", Severity::Info), clock.next());

        let codes: Vec<_> = store
            .entries()
            .iter()
            .map(|e| e.code().to_string())
            .collect();
        assert_eq!(codes, vec!["c", "b", "a"]);
    }

    #[test]
    fn same_key_replaces_in_place_and_moves_to_front() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("keep", Severity::Info).key("k"), clock.next());
        store.record(input("other", Severity::Info), clock.next());

        let first = store.record(input("hit-1", Severity::Error).key("k"), clock.next());
        assert_eq!(first, RecordOutcome::Replaced { occurrences: 2 });
        let second = store.record(input("hit-2", Severity::Warning).key("k"), clock.next());
        assert_eq!(second, RecordOutcome::Replaced { occurrences: 3 });

        assert_eq!(store.len(), 2, "同 key 不该新增行");
        let top = &store.entries()[0];
        assert_eq!(top.code().to_string(), "hit-2");
        assert_eq!(top.occurrences(), 3);
        assert_eq!(top.severity(), Severity::Warning, "档位要被替换成最新的");
    }

    #[test]
    fn replace_keeps_created_at_and_updates_updated_at() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("v1", Severity::Info).key("k"), clock.next());
        let before = store.entries()[0].clone();
        clock.next();
        store.record(input("v2", Severity::Info).key("k"), clock.next());
        let after = &store.entries()[0];

        assert_eq!(after.id(), before.id());
        assert_eq!(after.created_at_ms(), before.created_at_ms());
        assert!(after.updated_at_ms() > before.updated_at_ms());
    }

    #[test]
    fn no_key_always_appends() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("same", Severity::Info), clock.next());
        store.record(input("same", Severity::Info), clock.next());
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn caps_at_max_entries_dropping_oldest() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        for i in 0..(MAX_ENTRIES + 5) {
            store.record(input(&format!("n{i}"), Severity::Info), clock.next());
        }
        assert_eq!(store.len(), MAX_ENTRIES);
        assert_eq!(store.entries()[0].code().to_string(), "n104");
        assert_eq!(store.entries()[MAX_ENTRIES - 1].code().to_string(), "n5");
    }

    #[test]
    fn badge_prefers_error_over_info() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        assert_eq!(store.badge(), Badge::None, "空中心没有角标");

        store.record(input("i", Severity::Info), clock.next());
        assert_eq!(store.badge(), Badge::Info, "未读 info 是蓝点");

        store.record(input("w", Severity::Warning), clock.next());
        assert_eq!(store.badge(), Badge::Info, "warning 也是蓝点那一档");

        store.record(input("e", Severity::Error), clock.next());
        assert_eq!(store.badge(), Badge::Error, "有未读 error 就是红点");
    }

    #[test]
    fn first_record_is_unread() {
        let mut store = Store::new();
        store.record(input("first", Severity::Error), 1);
        assert_eq!(
            store.badge(),
            Badge::Error,
            "第一条通知必须是未读，否则角标永远不亮"
        );
        assert_eq!(store.unread_count(), 1);
    }

    #[test]
    fn mark_all_read_does_not_swallow_later_arrivals() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("before", Severity::Error), clock.next());
        assert_eq!(store.badge(), Badge::Error);

        store.mark_all_read();
        assert_eq!(store.badge(), Badge::None, "推进水位线后没有未读");
        assert_eq!(store.unread_count(), 0);

        store.record(input("after", Severity::Error), clock.next());
        assert_eq!(
            store.badge(),
            Badge::Error,
            "打开中心之后才发生的事，必须还是未读"
        );
        assert_eq!(store.unread_count(), 1);
    }

    #[test]
    fn removal_does_not_distort_the_watermark() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        let a = store.record(input("a", Severity::Error), clock.next());
        assert_eq!(a, RecordOutcome::Inserted);
        let a_id = store.entries()[0].id();
        store.record(input("b", Severity::Error), clock.next());
        store.mark_all_read();
        assert_eq!(store.badge(), Badge::None);

        assert!(store.remove(a_id));
        assert_eq!(store.len(), 1);
        assert_eq!(store.badge(), Badge::None, "删掉已读的条目不该冒出角标");

        store.record(input("c", Severity::Error), clock.next());
        assert_eq!(store.badge(), Badge::Error, "新来的仍是未读");
    }

    #[test]
    fn clear_resets_entries_and_watermark_together() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("old", Severity::Error), clock.next());
        store.mark_all_read();
        assert_eq!(store.badge(), Badge::None);

        let removed = store.clear();
        assert_eq!(removed, 1);
        assert!(store.is_empty());
        assert_eq!(store.read_upto_seq(), 0, "水位线必须一起归零");

        store.record(input("new-project", Severity::Error), clock.next());
        assert_eq!(store.badge(), Badge::Error);
    }

    #[test]
    fn clear_does_not_recycle_ids() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("a", Severity::Info), clock.next());
        let first_id = store.entries()[0].id();
        store.clear();
        store.record(input("b", Severity::Info), clock.next());
        assert_ne!(store.entries()[0].id(), first_id);
    }

    #[test]
    fn core_codes_map_through_and_cancelled_is_dropped() {
        assert!(
            Input::for_core_code("cancelled").is_none(),
            "cancelled 是用户自己取消的，不该进中心"
        );

        let input = Input::for_core_code("process_failed").expect("process_failed 有档位");
        let mut store = Store::new();
        store.record(input, 1);
        let entry = &store.entries()[0];
        assert_eq!(entry.severity(), Severity::Error);
        assert_eq!(
            entry.code().to_string(),
            "lithe.notifications.core.processFailed"
        );
        assert!(
            entry.matches_query("process_failed"),
            "coreCode 参数必须可搜"
        );
    }

    #[test]
    fn entries_since_returns_new_entries_oldest_first() {
        let mut store = Store::new();
        let mut clock = Clock(0);
        store.record(input("a", Severity::Info).key("k"), clock.next());
        store.record(input("b", Severity::Error), clock.next());
        store.record(input("c", Severity::Warning), clock.next());

        let none: Vec<_> = store
            .entries_since(3)
            .iter()
            .map(|e| e.code().to_string())
            .collect();
        assert!(none.is_empty(), "水位线之后没有条目时什么都不弹");

        let all: Vec<_> = store
            .entries_since(0)
            .iter()
            .map(|e| e.code().to_string())
            .collect();
        assert_eq!(all, vec!["a", "b", "c"], "旧→新，弹出的顺序是发生顺序");

        store.record(input("a2", Severity::Info).key("a"), clock.next());
        let again: Vec<_> = store
            .entries_since(2)
            .iter()
            .map(|e| e.code().to_string())
            .collect();
        assert_eq!(again, vec!["c", "a2"]);
    }
}
