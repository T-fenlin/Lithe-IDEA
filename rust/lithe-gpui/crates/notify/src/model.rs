use gpui_kit::SharedString;

use crate::severity::Severity;

pub const MAX_ENTRIES: usize = 100;

pub type EntryId = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    File {
        path: SharedString,
        line: Option<u32>,
    },
    ToolWindow {
        id: SharedString,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    id: EntryId,
    code: SharedString,
    params: Vec<(SharedString, SharedString)>,
    severity: Severity,
    key: Option<SharedString>,
    occurrences: u32,
    created_at_ms: u64,
    updated_at_ms: u64,
    seq: u64,
    target: Option<Target>,
}

impl Entry {
    pub(crate) fn new(
        id: EntryId,
        code: SharedString,
        params: Vec<(SharedString, SharedString)>,
        severity: Severity,
        key: Option<SharedString>,
        target: Option<Target>,
        now_ms: u64,
        seq: u64,
    ) -> Self {
        Self {
            id,
            code,
            params,
            severity,
            key,
            occurrences: 1,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
            seq,
            target,
        }
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn code(&self) -> &SharedString {
        &self.code
    }

    pub fn params(&self) -> &[(SharedString, SharedString)] {
        &self.params
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn key(&self) -> Option<&SharedString> {
        self.key.as_ref()
    }

    pub fn occurrences(&self) -> u32 {
        self.occurrences
    }

    pub fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }

    pub fn updated_at_ms(&self) -> u64 {
        self.updated_at_ms
    }

    pub fn seq(&self) -> u64 {
        self.seq
    }

    pub fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }

    pub fn with_target(mut self, target: Target) -> Self {
        self.target = Some(target);
        self
    }

    pub fn is_unread(&self, read_upto_seq: u64) -> bool {
        self.seq > read_upto_seq
    }

    pub(crate) fn replace_with(
        &mut self,
        code: SharedString,
        params: Vec<(SharedString, SharedString)>,
        severity: Severity,
        target: Option<Target>,
        now_ms: u64,
        seq: u64,
    ) {
        self.code = code;
        self.params = params;
        self.severity = severity;
        self.target = target;
        self.updated_at_ms = now_ms;
        self.seq = seq;
        self.occurrences = self.occurrences.saturating_add(1);
    }

    pub fn matches_query(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        if contains_ascii_ci(&self.code, query) {
            return true;
        }
        self.params.iter().any(|(_, value)| contains(value, query))
    }
}

pub fn matches_text(text: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    contains(text, query)
}

fn contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

fn contains_ascii_ci(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let haystack = haystack.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    haystack.contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(code: &str, params: &[(&str, &str)]) -> Entry {
        entry_with_seq(code, params, 1)
    }

    fn entry_with_seq(code: &str, params: &[(&str, &str)], seq: u64) -> Entry {
        Entry::new(
            seq,
            code.into(),
            params
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
            Severity::Info,
            None,
            None,
            0,
            seq,
        )
    }

    #[test]
    fn unread_is_decided_by_seq_not_by_position() {
        let older = entry_with_seq("lithe.notifications.demo.old", &[], 1);
        let newer = entry_with_seq("lithe.notifications.demo.new", &[], 2);
        assert!(older.is_unread(0));
        assert!(newer.is_unread(0));
        assert!(!older.is_unread(1));
        assert!(newer.is_unread(1));
    }

    #[test]
    fn code_matches_ascii_case_insensitively() {
        let item = entry("invalid_request", &[]);
        assert!(item.matches_query("INVALID_REQUEST"));
        assert!(item.matches_query("Invalid_Request"));
        assert!(item.matches_query("request"));
    }

    #[test]
    fn param_values_match_verbatim() {
        let item = entry("lithe.notifications.demo", &[("path", "/repo/POM.XML")]);
        assert!(item.matches_query("POM.XML"));
        assert!(!item.matches_query("pom.xml"));
    }

    #[test]
    fn param_names_are_not_searchable() {
        let item = entry("lithe.notifications.demo", &[("path", "value")]);
        assert!(item.matches_query("value"));
        assert!(!item.matches_query("path"));
    }

    #[test]
    fn empty_query_matches_everything() {
        let item = entry("lithe.notifications.demo", &[]);
        assert!(item.matches_query(""));
        assert!(matches_text("任意文案", ""));
    }

    #[test]
    fn text_matches_verbatim() {
        assert!(matches_text("打开失败", "失败"));
        assert!(!matches_text("打开失败", "失败：/repo"));
    }

    #[test]
    fn replace_keeps_identity_but_refreshes_recency() {
        let mut item = entry("lithe.notifications.demo", &[]);
        let id = item.id();
        let created = item.created_at_ms();

        item.replace_with(
            "lithe.notifications.demo2".into(),
            vec![],
            Severity::Error,
            None,
            500,
            9,
        );

        assert_eq!(item.id(), id, "id 必须不变，否则界面持有的 id 会失效");
        assert_eq!(item.created_at_ms(), created, "首次时间不该被替换改掉");
        assert_eq!(item.updated_at_ms(), 500);
        assert_eq!(item.occurrences(), 2);
        assert_eq!(item.seq(), 9);
        assert_eq!(item.severity(), Severity::Error);
        assert!(item.is_unread(8), "重新发生的事件必须重新变未读");
    }
}
