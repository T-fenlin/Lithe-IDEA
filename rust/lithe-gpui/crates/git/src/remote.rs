use gpui::SharedString;
use serde_json::Value;

use crate::changes::{self, WriteOutcome};
use crate::model::execute_core;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteOperation {
    Fetch,
    Pull,
    Push,
    Checkout,
    CheckoutRevision,
}

impl WriteOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fetch => "fetch",
            Self::Pull => "pull",
            Self::Push => "push",
            Self::Checkout => "checkout",
            Self::CheckoutRevision => "checkoutRevision",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullStrategy {
    FfOnly,
    Merge,
    Rebase,
}

impl PullStrategy {
    pub fn mode(self) -> &'static str {
        match self {
            Self::FfOnly => "ffOnly",
            Self::Merge => "merge",
            Self::Rebase => "rebase",
        }
    }
}

pub enum IntegrationOperation {
    Merge,
    Rebase,
    CherryPick,
    Revert,
}

impl IntegrationOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Rebase => "rebase",
            Self::CherryPick => "cherryPick",
            Self::Revert => "revert",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceKind {
    Local,
    Remote,
    Tag,
}

impl ReferenceKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
            Self::Tag => "tag",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WriteFields {
    pub paths: Vec<String>,
    pub reference: Option<String>,
    pub reference_kind: Option<ReferenceKind>,
    pub revision: Option<String>,
    pub mode: Option<String>,
    pub force: bool,
    pub auto_stash: bool,
    pub fetch_options: Option<Value>,
    pub check_submodules: bool,
    pub expected_commit_state: Option<Value>,
    pub gitlink_updates: Vec<Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteFieldError {
    FetchOptionsRequireFetch,
    SubmoduleChecksRequirePush,
    CommitStateRequiresCommitOrPush,
}

impl WriteFieldError {
    pub fn message(self) -> &'static str {
        match self {
            Self::FetchOptionsRequireFetch => "Fetch options require a fetch operation",
            Self::SubmoduleChecksRequirePush => {
                "Submodule publication checks require a push operation"
            }
            Self::CommitStateRequiresCommitOrPush => {
                "Commit state preconditions require a commit or push operation"
            }
        }
    }
}

pub fn validate_fields(
    operation: WriteOperation,
    fields: &WriteFields,
) -> Result<(), WriteFieldError> {
    if fields.fetch_options.is_some() && operation != WriteOperation::Fetch {
        return Err(WriteFieldError::FetchOptionsRequireFetch);
    }
    if fields.check_submodules && operation != WriteOperation::Push {
        return Err(WriteFieldError::SubmoduleChecksRequirePush);
    }
    if !fields.gitlink_updates.is_empty()
        || (fields.expected_commit_state.is_some() && operation != WriteOperation::Push)
    {
        return Err(WriteFieldError::CommitStateRequiresCommitOrPush);
    }
    Ok(())
}

impl WriteFields {
    pub fn to_payload(&self) -> Value {
        let mut payload = serde_json::Map::new();
        if !self.paths.is_empty() {
            payload.insert("paths".into(), serde_json::json!(self.paths));
        }
        if let Some(reference) = &self.reference {
            payload.insert("reference".into(), serde_json::json!(reference));
        }
        if let Some(kind) = self.reference_kind {
            payload.insert("referenceKind".into(), serde_json::json!(kind.id()));
        }
        if let Some(revision) = &self.revision {
            payload.insert("revision".into(), serde_json::json!(revision));
        }
        if let Some(mode) = &self.mode {
            payload.insert("mode".into(), serde_json::json!(mode));
        }
        if self.force {
            payload.insert("force".into(), serde_json::json!(true));
        }
        if self.auto_stash {
            payload.insert("autoStash".into(), serde_json::json!(true));
        }
        if let Some(options) = &self.fetch_options {
            payload.insert("fetchOptions".into(), options.clone());
        }
        if self.check_submodules {
            payload.insert("checkSubmodules".into(), serde_json::json!(true));
        }
        if let Some(state) = &self.expected_commit_state {
            payload.insert("expectedCommitState".into(), state.clone());
        }
        if !self.gitlink_updates.is_empty() {
            payload.insert(
                "gitlinkUpdates".into(),
                serde_json::json!(self.gitlink_updates),
            );
        }
        Value::Object(payload)
    }
}

pub fn execute(root: &str, operation: WriteOperation, fields: WriteFields) -> WriteOutcome {
    match validate_fields(operation, &fields) {
        Ok(()) => changes::write(operation.as_str(), root, fields.to_payload()),
        Err(error) => WriteOutcome {
            failure: Some(error.message().to_string()),
        },
    }
}

pub fn fetch(root: &str) -> WriteOutcome {
    execute(root, WriteOperation::Fetch, WriteFields::default())
}

pub fn pull(root: &str, strategy: PullStrategy) -> WriteOutcome {
    execute(
        root,
        WriteOperation::Pull,
        WriteFields {
            mode: Some(strategy.mode().to_string()),
            ..WriteFields::default()
        },
    )
}

pub fn push(root: &str, reference: Option<&str>) -> WriteOutcome {
    execute(
        root,
        WriteOperation::Push,
        WriteFields {
            reference: reference.map(str::to_string),
            ..WriteFields::default()
        },
    )
}

pub fn checkout(root: &str, branch: &str, force: bool, auto_stash: bool) -> WriteOutcome {
    execute(
        root,
        WriteOperation::Checkout,
        WriteFields {
            reference: Some(branch.to_string()),
            reference_kind: Some(ReferenceKind::Local),
            force,
            auto_stash,
            ..WriteFields::default()
        },
    )
}

#[derive(Clone, Debug, PartialEq)]
pub enum Probe<T> {
    Idle,
    Loading,
    Ready(T),
    Failed(SharedString),
}

impl<T> Probe<T> {
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    pub fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            _ => None,
        }
    }

    pub fn failure(&self) -> Option<&SharedString> {
        match self {
            Self::Failed(reason) => Some(reason),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Preflight<T> {
    serial: u64,
    state: Probe<T>,
}

impl<T> Default for Preflight<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Preflight<T> {
    pub fn new() -> Self {
        Self {
            serial: 0,
            state: Probe::Idle,
        }
    }

    pub fn begin(&mut self) -> u64 {
        self.serial = self.serial.wrapping_add(1);
        self.state = Probe::Loading;
        self.serial
    }

    pub fn apply(&mut self, serial: u64, result: Result<T, String>) -> bool {
        if serial != self.serial {
            return false;
        }
        self.state = match result {
            Ok(value) => Probe::Ready(value),
            Err(reason) => Probe::Failed(SharedString::from(reason)),
        };
        true
    }

    pub fn state(&self) -> &Probe<T> {
        &self.state
    }

    pub fn reset(&mut self) {
        self.state = Probe::Idle;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckoutPreflight {
    pub blocking_paths: Vec<SharedString>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IntegrationPreflight {
    pub blocking_paths: Vec<SharedString>,
    pub blocks_entirely: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PullPreflight {
    pub upstream: Option<SharedString>,
    pub ahead: usize,
    pub behind: usize,
    pub diverged: bool,
    pub has_local_changes: bool,
}

pub fn parse_checkout_preflight(data: &Value) -> CheckoutPreflight {
    CheckoutPreflight {
        blocking_paths: string_list(data, "blockingPaths"),
    }
}

pub fn parse_integration_preflight(data: &Value) -> IntegrationPreflight {
    IntegrationPreflight {
        blocking_paths: string_list(data, "blockingPaths"),
        blocks_entirely: data
            .get("blocksEntirely")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

pub fn parse_pull_preflight(data: &Value) -> PullPreflight {
    PullPreflight {
        upstream: data
            .get("upstream")
            .and_then(Value::as_str)
            .map(SharedString::from),
        ahead: count_of(data, "ahead"),
        behind: count_of(data, "behind"),
        diverged: data
            .get("diverged")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        has_local_changes: data
            .get("hasLocalChanges")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

pub fn parse_conflict_markers(data: &Value) -> Vec<SharedString> {
    let mut paths = string_list(data, "paths");
    paths.sort();
    paths.dedup();
    paths
}

fn string_list(data: &Value, key: &str) -> Vec<SharedString> {
    data.get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(SharedString::from)
                .collect()
        })
        .unwrap_or_default()
}

fn count_of(data: &Value, key: &str) -> usize {
    data.get(key)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(0)
}

pub fn checkout_preflight(root: &str, reference: &str) -> Result<CheckoutPreflight, String> {
    let data = execute_core(
        "git.checkoutPreflight",
        serde_json::json!({ "root": root, "reference": reference }),
    )?;
    Ok(parse_checkout_preflight(&data))
}

pub fn pull_preflight(root: &str) -> Result<PullPreflight, String> {
    let data = execute_core("git.pullPreflight", serde_json::json!({ "root": root }))?;
    Ok(parse_pull_preflight(&data))
}

pub fn integration_preflight(
    root: &str,
    reference: &str,
    operation: IntegrationOperation,
) -> Result<IntegrationPreflight, String> {
    let data = execute_core(
        "git.integrationPreflight",
        serde_json::json!({
            "root": root,
            "reference": reference,
            "operation": operation.as_str(),
        }),
    )?;
    Ok(parse_integration_preflight(&data))
}

pub fn conflict_markers(root: &str) -> Result<Vec<SharedString>, String> {
    let data = execute_core("git.conflictMarkers", serde_json::json!({ "root": root }))?;
    Ok(parse_conflict_markers(&data))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckoutGate {
    Proceed,
    Confirm { blocking_paths: Vec<SharedString> },
    Wait,
    Unavailable { reason: SharedString },
}

pub fn checkout_gate(probe: &Probe<CheckoutPreflight>) -> CheckoutGate {
    match probe {
        Probe::Idle | Probe::Loading => CheckoutGate::Wait,
        Probe::Failed(reason) => CheckoutGate::Unavailable {
            reason: reason.clone(),
        },
        Probe::Ready(preflight) if preflight.blocking_paths.is_empty() => CheckoutGate::Proceed,
        Probe::Ready(preflight) => CheckoutGate::Confirm {
            blocking_paths: preflight.blocking_paths.clone(),
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullStop {
    NoUpstream,
    UpToDate,
}

impl PullStop {
    pub fn message_key(self) -> &'static str {
        match self {
            Self::NoUpstream => "lithe.git.pullResult.noUpstream",
            Self::UpToDate => "lithe.git.pullResult.upToDate",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PullGate {
    Proceed,
    AskStrategy {
        upstream: SharedString,
        ahead: usize,
        behind: usize,
        has_local_changes: bool,
    },
    Stop(PullStop),
    Wait,
    Unavailable {
        reason: SharedString,
    },
}

pub fn pull_gate(probe: &Probe<PullPreflight>) -> PullGate {
    match probe {
        Probe::Idle | Probe::Loading => PullGate::Wait,
        Probe::Failed(reason) => PullGate::Unavailable {
            reason: reason.clone(),
        },
        Probe::Ready(preflight) => match &preflight.upstream {
            None => PullGate::Stop(PullStop::NoUpstream),
            Some(_) if preflight.behind == 0 && !preflight.diverged => {
                PullGate::Stop(PullStop::UpToDate)
            }
            Some(upstream) if preflight.diverged => PullGate::AskStrategy {
                upstream: upstream.clone(),
                ahead: preflight.ahead,
                behind: preflight.behind,
                has_local_changes: preflight.has_local_changes,
            },
            Some(_) => PullGate::Proceed,
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegrationGate {
    Proceed,
    Confirm {
        blocking_paths: Vec<SharedString>,
        blocks_entirely: bool,
    },
    Wait,
    Unavailable {
        reason: SharedString,
    },
}

pub fn integration_gate(probe: &Probe<IntegrationPreflight>) -> IntegrationGate {
    match probe {
        Probe::Idle | Probe::Loading => IntegrationGate::Wait,
        Probe::Failed(reason) => IntegrationGate::Unavailable {
            reason: reason.clone(),
        },
        Probe::Ready(preflight) if preflight.blocking_paths.is_empty() => IntegrationGate::Proceed,
        Probe::Ready(preflight) => IntegrationGate::Confirm {
            blocking_paths: preflight.blocking_paths.clone(),
            blocks_entirely: preflight.blocks_entirely,
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictGate {
    Clear,
    Present { paths: Vec<SharedString> },
    Wait,
    Unavailable { reason: SharedString },
}

pub fn conflict_gate(probe: &Probe<Vec<SharedString>>) -> ConflictGate {
    match probe {
        Probe::Idle | Probe::Loading => ConflictGate::Wait,
        Probe::Failed(reason) => ConflictGate::Unavailable {
            reason: reason.clone(),
        },
        Probe::Ready(paths) if paths.is_empty() => ConflictGate::Clear,
        Probe::Ready(paths) => ConflictGate::Present {
            paths: paths.clone(),
        },
    }
}

pub fn join_paths(paths: &[SharedString], limit: usize) -> (String, usize) {
    let listed = paths
        .iter()
        .take(limit)
        .map(SharedString::as_ref)
        .collect::<Vec<_>>()
        .join(", ");
    (listed, paths.len().saturating_sub(limit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn path(value: &str) -> SharedString {
        SharedString::from(value)
    }

    #[test]
    fn fetch_options_are_rejected_outside_fetch() {
        let fields = WriteFields {
            fetch_options: Some(json!({ "remote": "origin" })),
            ..WriteFields::default()
        };
        assert_eq!(
            validate_fields(WriteOperation::Pull, &fields),
            Err(WriteFieldError::FetchOptionsRequireFetch)
        );
        assert_eq!(validate_fields(WriteOperation::Fetch, &fields), Ok(()));
    }

    #[test]
    fn submodule_checks_are_rejected_outside_push() {
        let fields = WriteFields {
            check_submodules: true,
            ..WriteFields::default()
        };
        assert_eq!(
            validate_fields(WriteOperation::Fetch, &fields),
            Err(WriteFieldError::SubmoduleChecksRequirePush)
        );
        assert_eq!(validate_fields(WriteOperation::Push, &fields), Ok(()));
    }

    #[test]
    fn commit_state_preconditions_are_rejected_outside_push() {
        let expected = WriteFields {
            expected_commit_state: Some(json!({ "head": "abc" })),
            ..WriteFields::default()
        };
        assert_eq!(
            validate_fields(WriteOperation::Checkout, &expected),
            Err(WriteFieldError::CommitStateRequiresCommitOrPush)
        );
        assert_eq!(validate_fields(WriteOperation::Push, &expected), Ok(()));

        let gitlinks = WriteFields {
            gitlink_updates: vec![json!({ "path": "sub" })],
            ..WriteFields::default()
        };
        assert_eq!(
            validate_fields(WriteOperation::Push, &gitlinks),
            Err(WriteFieldError::CommitStateRequiresCommitOrPush)
        );
        assert_eq!(
            validate_fields(WriteOperation::Fetch, &gitlinks),
            Err(WriteFieldError::CommitStateRequiresCommitOrPush)
        );
    }

    #[test]
    fn default_fields_produce_an_empty_payload() {
        let payload = WriteFields::default().to_payload();
        assert_eq!(payload, json!({}));
    }

    #[test]
    fn checkout_fields_serialize_with_the_local_kind() {
        let fields = WriteFields {
            reference: Some("main".into()),
            reference_kind: Some(ReferenceKind::Local),
            force: true,
            auto_stash: false,
            ..WriteFields::default()
        };
        assert_eq!(
            fields.to_payload(),
            json!({ "reference": "main", "referenceKind": "local", "force": true })
        );
    }

    #[test]
    fn pull_strategies_use_the_core_modes() {
        assert_eq!(PullStrategy::FfOnly.mode(), "ffOnly");
        assert_eq!(PullStrategy::Merge.mode(), "merge");
        assert_eq!(PullStrategy::Rebase.mode(), "rebase");
    }

    #[test]
    fn write_operation_names_match_core() {
        assert_eq!(WriteOperation::Fetch.as_str(), "fetch");
        assert_eq!(WriteOperation::Pull.as_str(), "pull");
        assert_eq!(WriteOperation::Push.as_str(), "push");
        assert_eq!(WriteOperation::Checkout.as_str(), "checkout");
        assert_eq!(
            WriteOperation::CheckoutRevision.as_str(),
            "checkoutRevision"
        );
        assert_eq!(IntegrationOperation::CherryPick.as_str(), "cherryPick");
    }

    #[test]
    fn a_rejected_field_combination_becomes_a_visible_failure() {
        let outcome = execute(
            "/lithe-u11-does-not-exist",
            WriteOperation::Fetch,
            WriteFields {
                check_submodules: true,
                ..WriteFields::default()
            },
        );
        assert_eq!(
            outcome.failure.as_deref(),
            Some(WriteFieldError::SubmoduleChecksRequirePush.message())
        );
    }

    #[test]
    fn parsers_tolerate_missing_and_mistyped_fields() {
        assert_eq!(
            parse_checkout_preflight(&json!({})),
            CheckoutPreflight::default()
        );
        assert_eq!(
            parse_checkout_preflight(&json!({ "blockingPaths": "not-an-array" })),
            CheckoutPreflight::default()
        );
        assert_eq!(
            parse_integration_preflight(&json!({ "blockingPaths": ["a"], "blocksEntirely": 1 })),
            IntegrationPreflight {
                blocking_paths: vec![path("a")],
                blocks_entirely: false,
            }
        );
        assert_eq!(
            parse_pull_preflight(&json!({ "ahead": "two", "behind": null })),
            PullPreflight::default()
        );
        assert!(parse_conflict_markers(&json!({ "paths": null })).is_empty());
    }

    #[test]
    fn an_absent_upstream_is_a_missing_upstream_not_a_failure() {
        let absent = parse_pull_preflight(&json!({ "ahead": 0, "behind": 0, "diverged": false }));
        let null = parse_pull_preflight(&json!({ "upstream": null }));
        assert_eq!(absent, null);
        assert_eq!(absent.upstream, None);
    }

    #[test]
    fn conflict_paths_are_sorted_and_deduplicated() {
        let parsed = parse_conflict_markers(&json!({ "paths": ["b.txt", "a.txt", "b.txt"] }));
        assert_eq!(parsed, vec![path("a.txt"), path("b.txt")]);
    }

    #[test]
    fn a_stale_reply_is_dropped_without_disturbing_the_new_read() {
        let mut probe: Preflight<PullPreflight> = Preflight::new();
        let stale = probe.begin();
        assert!(probe.state().is_loading());

        let fresh = probe.begin();
        assert!(probe.state().is_loading());

        assert!(!probe.apply(stale, Ok(PullPreflight::default())));
        assert!(
            probe.state().is_loading(),
            "旧回包不该把状态改成 Ready：{:?}",
            probe.state()
        );

        let value = PullPreflight {
            upstream: Some(path("origin/main")),
            behind: 2,
            ..PullPreflight::default()
        };
        assert!(probe.apply(fresh, Ok(value.clone())));
        assert_eq!(probe.state().ready(), Some(&value));
    }

    #[test]
    fn a_failed_read_is_not_a_ready_read() {
        let mut probe: Preflight<CheckoutPreflight> = Preflight::new();
        let serial = probe.begin();
        assert!(probe.apply(serial, Err("process_failed".into())));
        assert!(probe.state().is_failed());
        assert_eq!(probe.state().ready(), None);
        assert_eq!(
            probe.state().failure().map(SharedString::as_ref),
            Some("process_failed")
        );
    }

    #[test]
    fn idle_and_loading_are_their_own_states() {
        let mut probe: Preflight<Vec<SharedString>> = Preflight::new();
        assert!(probe.state().is_idle());
        probe.begin();
        assert!(probe.state().is_loading());
        probe.reset();
        assert!(probe.state().is_idle());
    }

    #[test]
    fn checkout_gate_covers_all_four_outcomes() {
        assert_eq!(checkout_gate(&Probe::Idle), CheckoutGate::Wait);
        assert_eq!(checkout_gate(&Probe::Loading), CheckoutGate::Wait);
        assert_eq!(
            checkout_gate(&Probe::Failed(path("process_failed"))),
            CheckoutGate::Unavailable {
                reason: path("process_failed")
            }
        );
        assert_eq!(
            checkout_gate(&Probe::Ready(CheckoutPreflight::default())),
            CheckoutGate::Proceed
        );
        assert_eq!(
            checkout_gate(&Probe::Ready(CheckoutPreflight {
                blocking_paths: vec![path("a.rs")],
            })),
            CheckoutGate::Confirm {
                blocking_paths: vec![path("a.rs")]
            }
        );
    }

    #[test]
    fn pull_gate_follows_the_source_order() {
        assert_eq!(
            pull_gate(&Probe::Ready(PullPreflight::default())),
            PullGate::Stop(PullStop::NoUpstream)
        );
        assert_eq!(
            pull_gate(&Probe::Ready(PullPreflight {
                upstream: Some(path("origin/main")),
                ..PullPreflight::default()
            })),
            PullGate::Stop(PullStop::UpToDate)
        );
        assert_eq!(
            pull_gate(&Probe::Ready(PullPreflight {
                upstream: Some(path("origin/main")),
                behind: 3,
                ..PullPreflight::default()
            })),
            PullGate::Proceed
        );
        assert_eq!(
            pull_gate(&Probe::Ready(PullPreflight {
                upstream: Some(path("origin/main")),
                ahead: 1,
                behind: 3,
                diverged: true,
                has_local_changes: true,
            })),
            PullGate::AskStrategy {
                upstream: path("origin/main"),
                ahead: 1,
                behind: 3,
                has_local_changes: true,
            }
        );
    }

    #[test]
    fn pull_gate_never_pretends_success_when_the_read_failed() {
        assert_eq!(pull_gate(&Probe::Idle), PullGate::Wait);
        assert_eq!(pull_gate(&Probe::Loading), PullGate::Wait);
        assert_eq!(
            pull_gate(&Probe::Failed(path("timed_out"))),
            PullGate::Unavailable {
                reason: path("timed_out")
            }
        );
    }

    #[test]
    fn integration_gate_reports_the_shape_of_the_block() {
        assert_eq!(
            integration_gate(&Probe::Ready(IntegrationPreflight::default())),
            IntegrationGate::Proceed
        );
        assert_eq!(
            integration_gate(&Probe::Ready(IntegrationPreflight {
                blocking_paths: vec![path("a.rs"), path("b.rs")],
                blocks_entirely: true,
            })),
            IntegrationGate::Confirm {
                blocking_paths: vec![path("a.rs"), path("b.rs")],
                blocks_entirely: true,
            }
        );
        assert_eq!(
            integration_gate(&Probe::Failed(path("process_failed"))),
            IntegrationGate::Unavailable {
                reason: path("process_failed")
            }
        );
    }

    #[test]
    fn conflict_gate_blocks_on_markers_and_on_unknown() {
        assert_eq!(
            conflict_gate(&Probe::Ready(Vec::new())),
            ConflictGate::Clear
        );
        assert_eq!(
            conflict_gate(&Probe::Ready(vec![path("a.rs")])),
            ConflictGate::Present {
                paths: vec![path("a.rs")]
            }
        );
        assert_eq!(conflict_gate(&Probe::Idle), ConflictGate::Wait);
        assert_eq!(
            conflict_gate(&Probe::Failed(path("parse_failed"))),
            ConflictGate::Unavailable {
                reason: path("parse_failed")
            }
        );
    }

    #[test]
    fn join_paths_reports_the_hidden_count() {
        let paths = vec![path("a"), path("b"), path("c")];
        assert_eq!(join_paths(&paths, 2), ("a, b".to_string(), 1));
        assert_eq!(join_paths(&paths, 9), ("a, b, c".to_string(), 0));
        assert_eq!(join_paths(&[], 3), (String::new(), 0));
    }

    #[test]
    fn the_field_rules_match_core_over_the_real_envelope() {
        let root = "/lithe-u11-field-rules";

        let rejected = execute_core(
            "git.write",
            json!({ "root": root, "operation": "pull", "fetchOptions": { "remote": "origin" } }),
        )
        .expect_err("Core 必须拒掉 fetch 之外的 fetchOptions");
        assert_eq!(rejected, "invalid_request");

        let rejected = execute_core(
            "git.write",
            json!({ "root": root, "operation": "fetch", "checkSubmodules": true }),
        )
        .expect_err("Core 必须拒掉 push 之外的 checkSubmodules");
        assert_eq!(rejected, "invalid_request");

        let rejected = execute_core(
            "git.write",
            json!({ "root": root, "operation": "checkout", "expectedCommitState": { "head": "0" } }),
        )
        .expect_err("Core 必须拒掉 commit/push 之外的 expectedCommitState");
        assert_eq!(rejected, "invalid_request");

        let deeper = execute_core(
            "git.write",
            json!({ "root": root, "operation": "fetch", "fetchOptions": { "remote": "origin" } }),
        )
        .expect_err("这个根不存在，取回不该成功");
        assert_ne!(
            deeper, "invalid_request",
            "合法的取回组合不该被字段约束拦下"
        );
    }

    #[test]
    fn the_preflight_commands_are_accepted_by_core() {
        let root = "/lithe-u11-preflight-shape";

        let checkout = checkout_preflight(root, "main").expect_err("根不存在，不该成功");
        assert_ne!(
            checkout, "invalid_request",
            "checkoutPreflight 的请求形状被拒了"
        );

        let pull = pull_preflight(root).expect_err("根不存在，不该成功");
        assert_ne!(pull, "invalid_request", "pullPreflight 的请求形状被拒了");

        let integration = integration_preflight(root, "main", IntegrationOperation::Rebase)
            .expect_err("根不存在，不该成功");
        assert_ne!(
            integration, "invalid_request",
            "integrationPreflight 的请求形状被拒了"
        );

        let markers = conflict_markers(root).expect_err("根不存在，不该成功");
        assert_ne!(
            markers, "invalid_request",
            "conflictMarkers 的请求形状被拒了"
        );
    }
}
