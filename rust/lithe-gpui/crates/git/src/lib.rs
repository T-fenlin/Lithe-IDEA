mod branch_info;
mod changes;
mod changes_view;
mod identity;
mod log_view;
mod model;
mod remote;

pub use branch_info::{BranchInfo, BranchSnapshot, TrackingCounts};
pub use changes::WriteOutcome;
pub use changes_view::ChangesView;
pub use identity::{
    IdentityField, IdentityScope, IdentitySetup, configure_identity, inspect_identity,
};
pub use log_view::BottomPane;
pub use remote::{
    CheckoutGate, CheckoutPreflight, ConflictGate, IntegrationGate, IntegrationOperation,
    IntegrationPreflight, Preflight, Probe, PullGate, PullPreflight, PullStop, PullStrategy,
    ReferenceKind, WriteFieldError, WriteFields, WriteOperation, checkout, checkout_gate,
    checkout_preflight, conflict_gate, conflict_markers, fetch, integration_gate,
    integration_preflight, join_paths, parse_checkout_preflight, parse_conflict_markers,
    parse_integration_preflight, parse_pull_preflight, pull, pull_gate, pull_preflight, push,
    validate_fields,
};

use gpui::{App, KeyBinding};

gpui::actions!(lithe_git, [CommitChanges]);

pub fn install_actions(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-enter", CommitChanges, None)]);
}
