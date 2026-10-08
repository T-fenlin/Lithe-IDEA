pub mod paths;
pub mod project;
pub mod session;
pub mod sharing;
#[cfg(test)]
pub(crate) mod test_repo;
pub mod toolchain;

pub const DIAGNOSTIC_PREFIX: &str = "S1_WORKSPACE_CONFIG";

pub use paths::{
    LOCAL_LAYER_SUFFIX, RUN_CLASSES_DIRECTORY, WORKSPACE_CONFIG_DIRECTORY, WorkspaceConfigPaths,
    is_pruned_directory, is_shareable_member, workspace_config_exclude_pattern, workspace_relative,
};
pub use project::{
    PROJECT_MANIFEST_VERSION, ProjectIdentity, ProjectManifest, ProjectManifestState,
    WorkspaceConfigError, load_project_manifest, path_identity, resolve_project_id,
    save_project_manifest,
};
pub use session::{
    MAX_OPEN_FILES, SESSION_DOCUMENT_VERSION, SessionError, SessionFileOutcome, SessionLoaded,
    SessionSaved, SessionState, SessionUiState, load_session, resolve_session_files, save_session,
    session_from_paths, session_path,
};
pub use sharing::{
    EnsureExcluded, IgnoreFileOutcome, LOCAL_IGNORE_RULES, ShareOutcome, SharingError,
    ensure_local_ignore_file, ensure_project_dir_excluded, pattern_is_present,
    share_project_config, shareable_members,
};
pub use toolchain::{
    LocalToolchainState, MAVEN_LOCAL_DOCUMENT_VERSION, OverrideOrigin, OverrideOrigins,
    RUN_LOCAL_DOCUMENT_VERSION, ToolchainPaths, ToolchainSaved, load as load_local_toolchain,
    save as save_local_toolchain,
};
