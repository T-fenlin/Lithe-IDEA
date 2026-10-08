use std::path::Path;

use gpui_kit::component::ActiveTheme as _;
use gpui::{App, Hsla, SharedString};

use crate::model::{
    discover_repository_root, execute_core, parse_references, repository_root_of,
    resolve_repository_root,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
}

impl ChangeKind {
    pub fn parse(status: &str, untracked: bool) -> Self {
        if untracked || status.contains('?') {
            return Self::Untracked;
        }
        let mut chars = status.chars();
        let index = chars.next().unwrap_or(' ');
        let worktree = chars.next().unwrap_or(' ');
        if index == 'A' {
            return Self::Added;
        }
        if worktree == 'D' || index == 'D' {
            return Self::Deleted;
        }
        if [index, worktree]
            .iter()
            .any(|code| *code == 'R' || *code == 'C')
        {
            return Self::Renamed;
        }
        if worktree == 'A' {
            return Self::Added;
        }
        Self::Modified
    }

    pub fn group_order(self) -> usize {
        match self {
            Self::Added => 0,
            Self::Modified => 1,
            Self::Deleted => 2,
            Self::Renamed => 3,
            Self::Untracked => 4,
        }
    }

    pub fn color(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        match self {
            Self::Added => theme.green_light,
            Self::Modified => theme.blue_light,
            Self::Deleted => theme.muted_foreground,
            Self::Renamed => theme.magenta_light,
            Self::Untracked => theme.danger,
        }
    }

    pub fn is_untracked(self) -> bool {
        self == Self::Untracked
    }
}

#[derive(Clone, Debug)]
pub struct ChangeRow {
    pub path: SharedString,
    pub original_path: Option<SharedString>,
    pub raw_status: SharedString,
    pub kind: ChangeKind,
    pub staged: bool,
    pub worktree: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationKind {
    Merge,
    Rebase,
    CherryPick,
    Revert,
}

impl OperationKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "merge" => Some(Self::Merge),
            "rebase" => Some(Self::Rebase),
            "cherryPick" => Some(Self::CherryPick),
            "revert" => Some(Self::Revert),
            _ => None,
        }
    }

    pub fn title_key(self) -> &'static str {
        match self {
            Self::Merge => "lithe.git.mergeInProgress",
            Self::Rebase => "lithe.git.rebaseInProgress",
            Self::CherryPick => "lithe.git.cherryPickInProgress",
            Self::Revert => "lithe.git.revertInProgress",
        }
    }

    pub fn continue_key(self) -> &'static str {
        match self {
            Self::Merge => "lithe.git.continueMerge",
            Self::Rebase => "lithe.git.continueRebase",
            Self::CherryPick => "lithe.git.continueCherryPick",
            Self::Revert => "lithe.git.continueRevert",
        }
    }

    pub fn can_skip(self) -> bool {
        self == Self::Rebase
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Rebase => "rebase",
            Self::CherryPick => "cherryPick",
            Self::Revert => "revert",
        }
    }
}

#[derive(Clone, Debug)]
pub struct OperationState {
    pub kind: OperationKind,
    pub reference: Option<SharedString>,
    pub step: Option<usize>,
    pub total: Option<usize>,
    pub conflicted_paths: Vec<SharedString>,
}

impl OperationState {
    pub fn has_conflicts(&self) -> bool {
        !self.conflicted_paths.is_empty()
    }
}

#[derive(Clone, Debug, Default)]
pub struct ChangesSnapshot {
    pub repository_root: Option<SharedString>,
    pub branch: Option<SharedString>,
    pub ahead: usize,
    pub behind: usize,
    pub changes: Vec<ChangeRow>,
    pub operation: Option<OperationState>,
    pub has_commits: Option<bool>,
    pub failures: Vec<String>,
}

impl ChangesSnapshot {
    pub fn staged_count(&self) -> usize {
        self.changes.iter().filter(|row| row.staged).count()
    }

    pub fn unstaged_count(&self) -> usize {
        self.changes.iter().filter(|row| !row.staged).count()
    }

    pub fn tracked_rows(&self) -> Vec<&ChangeRow> {
        self.changes
            .iter()
            .filter(|row| !row.kind.is_untracked())
            .collect()
    }

    pub fn untracked_rows(&self) -> Vec<&ChangeRow> {
        self.changes
            .iter()
            .filter(|row| row.kind.is_untracked())
            .collect()
    }

    pub fn state_label(&self) -> &'static str {
        if self.repository_root.is_none() {
            "norepo"
        } else if !self.changes.is_empty() {
            "ready"
        } else if self.has_commits == Some(false) {
            "nocommits"
        } else {
            "clean"
        }
    }

    pub fn diagnose(&self) -> String {
        format!(
            "S1_SOURCE_CONTROL files={} staged={} unstaged={} state={} root={} branch={} op={}",
            self.changes.len(),
            self.staged_count(),
            self.unstaged_count(),
            self.state_label(),
            self.repository_root.as_deref().unwrap_or("-"),
            self.branch.as_deref().unwrap_or("-"),
            self.operation
                .as_ref()
                .map(|operation| operation.kind.id())
                .unwrap_or("none"),
        )
    }
}

pub fn load(root: &Path) -> ChangesSnapshot {
    let mut snapshot = ChangesSnapshot::default();
    let root_text = root.to_string_lossy().to_string();

    match execute_core("git.status", serde_json::json!({ "root": root_text })) {
        Ok(data) => {
            snapshot.repository_root =
                repository_root_of(&data).map(|found| resolve_repository_root(root, &found).into());
            snapshot.branch = data
                .get("branch")
                .and_then(serde_json::Value::as_str)
                .map(SharedString::from);
            snapshot.ahead = data
                .get("ahead")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as usize;
            snapshot.behind = data
                .get("behind")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as usize;
            snapshot.changes = parse_changes(&data);
        }
        Err(code) => snapshot.failures.push(format!("git.status（{code}）")),
    }

    if snapshot.repository_root.is_none() {
        snapshot.repository_root = discover_repository_root(root).map(SharedString::from);
    }

    let Some(repository_root) = snapshot.repository_root.clone() else {
        return snapshot;
    };

    match execute_core(
        "git.operationState",
        serde_json::json!({ "root": repository_root.as_ref() }),
    ) {
        Ok(data) => snapshot.operation = parse_operation(&data),
        Err(code) => snapshot
            .failures
            .push(format!("git.operationState（{code}）")),
    }

    if snapshot.changes.is_empty() {
        snapshot.has_commits = match execute_core(
            "git.references",
            serde_json::json!({ "root": repository_root.as_ref() }),
        ) {
            Ok(data) => Some(!parse_references(&data).is_empty()),
            Err(code) => {
                snapshot.failures.push(format!("git.references（{code}）"));
                None
            }
        };
    }

    snapshot
}

pub fn parse_changes(data: &serde_json::Value) -> Vec<ChangeRow> {
    let mut rows: Vec<ChangeRow> = Vec::new();

    let Some(changes) = data.get("changes").and_then(serde_json::Value::as_array) else {
        return rows;
    };

    for change in changes {
        let Some(path) = change.get("path").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let raw_status = change
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let untracked = change
            .get("untracked")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let row = ChangeRow {
            path: SharedString::from(path.to_string()),
            original_path: change
                .get("originalPath")
                .and_then(serde_json::Value::as_str)
                .map(SharedString::from),
            raw_status: SharedString::from(raw_status.to_string()),
            kind: ChangeKind::parse(raw_status, untracked),
            staged: change
                .get("staged")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            worktree: change
                .get("worktree")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        };

        match rows.iter_mut().find(|existing| existing.path == row.path) {
            Some(existing) => {
                if !existing.staged && row.staged {
                    *existing = row;
                }
            }
            None => rows.push(row),
        }
    }

    rows.sort_by(|left, right| {
        left.kind
            .is_untracked()
            .cmp(&right.kind.is_untracked())
            .then_with(|| left.kind.group_order().cmp(&right.kind.group_order()))
            .then_with(|| left.path.cmp(&right.path))
    });
    rows
}

pub fn parse_operation(data: &serde_json::Value) -> Option<OperationState> {
    let kind = data
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .and_then(OperationKind::parse)?;

    Some(OperationState {
        kind,
        reference: data
            .get("reference")
            .and_then(serde_json::Value::as_str)
            .map(SharedString::from),
        step: data
            .get("step")
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize),
        total: data
            .get("total")
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize),
        conflicted_paths: data
            .get("conflictedPaths")
            .and_then(serde_json::Value::as_array)
            .map(|paths| {
                paths
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(SharedString::from)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

#[derive(Debug)]
pub struct WriteOutcome {
    pub failure: Option<String>,
}

pub fn write(operation: &str, root: &str, payload: serde_json::Value) -> WriteOutcome {
    let mut request = serde_json::json!({ "root": root, "operation": operation });
    if let (Some(target), Some(extra)) = (request.as_object_mut(), payload.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }

    let data = match execute_core("git.write", request) {
        Ok(data) => data,
        Err(code) => {
            return WriteOutcome {
                failure: Some(code),
            };
        }
    };

    if let Some(error) = data.get("operationError") {
        let message = error
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("operation failed");
        let code = error
            .get("code")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("operation_failed");
        return WriteOutcome {
            failure: Some(format!("{code}: {}", redact(message, root))),
        };
    }

    let exit_code = data
        .get("exitCode")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    if exit_code == 0 {
        return WriteOutcome { failure: None };
    }

    let stderr = data
        .get("stderr")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let stdout = data
        .get("stdout")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let detail = first_meaningful_line(stderr)
        .or_else(|| first_meaningful_line(stdout))
        .unwrap_or_else(|| format!("git exited with {exit_code}"));
    WriteOutcome {
        failure: Some(redact(&detail, root)),
    }
}

fn first_meaningful_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn redact(message: &str, root: &str) -> String {
    if root.is_empty() {
        return message.to_string();
    }
    let forward = root.replace('\\', "/");
    let backward = root.replace('/', "\\");
    let mut text = message
        .replace(root, "")
        .replace(&forward, "")
        .replace(&backward, "");
    text = text.replace("''", "").trim().to_string();
    if text.is_empty() {
        "git command failed".to_string()
    } else {
        text
    }
}

pub fn stage(root: &str, paths: &[String]) -> WriteOutcome {
    write("stage", root, serde_json::json!({ "paths": paths }))
}

pub fn unstage(root: &str, paths: &[String]) -> WriteOutcome {
    write("unstage", root, serde_json::json!({ "paths": paths }))
}

pub fn stage_all(root: &str) -> WriteOutcome {
    write("stageAll", root, serde_json::json!({}))
}

pub fn commit(root: &str, message: &str) -> WriteOutcome {
    write(
        "commit",
        root,
        serde_json::json!({ "message": message, "paths": Vec::<String>::new() }),
    )
}

pub fn discard(root: &str, paths: &[String]) -> WriteOutcome {
    write("discard", root, serde_json::json!({ "paths": paths }))
}

pub fn operation_action(root: &str, operation: &str) -> WriteOutcome {
    write(operation, root, serde_json::json!({}))
}

#[cfg(test)]
mod tests {
    use super::{ChangeKind, ChangeRow, OperationKind, parse_changes, parse_operation};
    use gpui::SharedString;

    fn row(path: &str, status: &str, untracked: bool) -> ChangeRow {
        ChangeRow {
            path: SharedString::from(path),
            original_path: None,
            raw_status: SharedString::from(status),
            kind: ChangeKind::parse(status, untracked),
            staged: !status.starts_with(' ') && !untracked,
            worktree: !status.ends_with(' ') && !untracked,
        }
    }

    #[test]
    fn status_codes_normalize_like_the_windows_adapter() {
        assert_eq!(ChangeKind::parse("??", true), ChangeKind::Untracked);
        assert_eq!(ChangeKind::parse(" M", false), ChangeKind::Modified);
        assert_eq!(ChangeKind::parse("M ", false), ChangeKind::Modified);
        assert_eq!(ChangeKind::parse("MM", false), ChangeKind::Modified);
        assert_eq!(ChangeKind::parse("A ", false), ChangeKind::Added);
        assert_eq!(ChangeKind::parse(" A", false), ChangeKind::Added);
        assert_eq!(ChangeKind::parse(" D", false), ChangeKind::Deleted);
        assert_eq!(ChangeKind::parse("D ", false), ChangeKind::Deleted);
        assert_eq!(ChangeKind::parse("R ", false), ChangeKind::Renamed);
        assert_eq!(ChangeKind::parse("RM", false), ChangeKind::Renamed);
        assert_eq!(ChangeKind::parse("AD", false), ChangeKind::Added);
    }

    #[test]
    fn untracked_is_its_own_category() {
        assert!(ChangeKind::Untracked.is_untracked());
        for kind in [
            ChangeKind::Added,
            ChangeKind::Modified,
            ChangeKind::Deleted,
            ChangeKind::Renamed,
        ] {
            assert!(!kind.is_untracked());
        }
        assert!(ChangeKind::Added.group_order() < ChangeKind::Modified.group_order());
        assert!(ChangeKind::Modified.group_order() < ChangeKind::Deleted.group_order());
        assert!(ChangeKind::Deleted.group_order() < ChangeKind::Renamed.group_order());
    }

    #[test]
    fn same_path_records_coalesce_and_staged_wins() {
        let data = serde_json::json!({
            "changes": [
                { "path": "a.txt", "status": " M", "staged": false, "worktree": true, "untracked": false },
                { "path": "a.txt", "status": "M ", "staged": true, "worktree": false, "untracked": false },
                { "path": "b.txt", "status": "??", "staged": false, "worktree": false, "untracked": true }
            ]
        });
        let rows = parse_changes(&data);
        assert_eq!(rows.len(), 2);
        let a = rows.iter().find(|row| row.path == "a.txt").unwrap();
        assert!(a.staged, "已暂存的那条必须胜出");
        assert_eq!(a.raw_status.as_ref(), "M ");
    }

    #[test]
    fn rows_sort_by_category_then_status_then_path() {
        let data = serde_json::json!({
            "changes": [
                { "path": "z.txt", "status": "??", "staged": false, "worktree": false, "untracked": true },
                { "path": "m.txt", "status": " M", "staged": false, "worktree": true, "untracked": false },
                { "path": "a.txt", "status": "A ", "staged": true, "worktree": false, "untracked": false },
                { "path": "b.txt", "status": " M", "staged": false, "worktree": true, "untracked": false }
            ]
        });
        let rows = parse_changes(&data);
        let paths: Vec<&str> = rows.iter().map(|row| row.path.as_ref()).collect();
        assert_eq!(paths, vec!["a.txt", "b.txt", "m.txt", "z.txt"]);
    }

    #[test]
    fn rename_keeps_original_path() {
        let data = serde_json::json!({
            "changes": [
                { "path": "new.txt", "originalPath": "old.txt", "status": "R ", "staged": true, "worktree": false, "untracked": false }
            ]
        });
        let rows = parse_changes(&data);
        assert_eq!(rows[0].kind, ChangeKind::Renamed);
        assert_eq!(rows[0].original_path.as_deref(), Some("old.txt"));
    }

    #[test]
    fn operation_state_is_none_without_an_active_operation() {
        assert!(parse_operation(&serde_json::json!({ "kind": "" })).is_none());
        assert!(parse_operation(&serde_json::json!({ "kind": "bisect" })).is_none());
    }

    #[test]
    fn rebase_operation_reads_progress_and_conflicts() {
        let operation = parse_operation(&serde_json::json!({
            "kind": "rebase",
            "reference": "abcdef1234567890",
            "step": 2,
            "total": 5,
            "conflictedPaths": ["src/a.rs"]
        }))
        .expect("rebase 必须被识别");
        assert_eq!(operation.kind, OperationKind::Rebase);
        assert_eq!(operation.step, Some(2));
        assert_eq!(operation.total, Some(5));
        assert!(operation.has_conflicts());
        assert!(operation.kind.can_skip());
        assert!(!OperationKind::Merge.can_skip());
    }

    #[test]
    fn merge_without_conflicts_can_continue() {
        let operation = parse_operation(&serde_json::json!({
            "kind": "merge", "reference": null, "step": null, "total": null, "conflictedPaths": []
        }))
        .expect("merge 必须被识别");
        assert!(!operation.has_conflicts());
        assert_eq!(operation.reference, None);
    }

    #[test]
    fn empty_state_labels_follow_the_snapshot() {
        let mut snapshot = super::ChangesSnapshot::default();
        assert_eq!(snapshot.state_label(), "norepo");

        snapshot.repository_root = Some(SharedString::from("/repo"));
        snapshot.has_commits = Some(true);
        assert_eq!(snapshot.state_label(), "clean");

        snapshot.has_commits = Some(false);
        assert_eq!(snapshot.state_label(), "nocommits");

        snapshot.changes = vec![row("a.txt", " M", false), row("b.txt", "??", true)];
        assert_eq!(snapshot.state_label(), "ready");
        assert_eq!(snapshot.tracked_rows().len(), 1);
        assert_eq!(snapshot.untracked_rows().len(), 1);
        assert_eq!(snapshot.unstaged_count(), 2);
        assert_eq!(snapshot.staged_count(), 0);
        assert!(
            snapshot
                .diagnose()
                .starts_with("S1_SOURCE_CONTROL files=2 staged=0 unstaged=2")
        );
    }
}
