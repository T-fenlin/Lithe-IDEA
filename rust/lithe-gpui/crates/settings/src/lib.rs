pub mod command_catalog;
pub mod dialog;
pub mod extensions;
pub mod fonts;
pub mod identity;
pub mod keymap;
pub mod keymap_io;
pub mod paths;
pub mod persistence;
pub mod project;
pub mod recent_projects;
pub mod restart;
pub mod row;
pub mod run;
pub mod schema;
pub mod store;
pub mod theme;
pub mod watch;
pub mod workspace;

/// `label_key` 这个名字在 crate 根太泛（`row`、`schema` 都有自己的 key 概念），
/// 带个前缀带出「这是命令的文案」。
pub use command_catalog::label_key as command_label_key;
pub use dialog::{
    Category, OpenSettings, install_actions, open_settings_dialog, open_settings_dialog_at,
};
pub use extensions::prepare_bundled_extensions;
pub use fonts::register_fonts;
pub use identity::{
    GitIdentityHost, GitIdentityPage, IDENTITY_MAX_BYTES, IdentityField, IdentityScope,
    IdentitySetup, git_identity_page, host_workspace_root, identity_value_is_valid,
    set_git_identity_host,
};
pub use keymap::{
    BindableResolver, DEFAULT_BINDINGS, ECLIPSE_BINDINGS, IDEA_CLASSIC_BINDINGS,
    KEYMAP_SETTINGS_VERSION, KeyOverride, KeySource, KeymapPreset, KeymapSettings, PresetBinding,
    ResolvedBinding, context_for, is_bindable, parse_keymap, preset_bindings,
    set_bindable_resolver,
};
pub use keymap_io::{
    EXPORT_FILE_NAME, EXPORT_FORMAT, EXPORT_VERSION, ImportEntry, ImportPlan, ImportResult,
    export_payload, parse_import,
};
pub use paths::{SETTINGS_FILE_ENV, recent_projects_file_path, settings_file_path};
pub use persistence::{
    DOCUMENT_VERSION, DOCUMENT_VERSION_KEY, DebounceState, Loaded, Parsed, SAVE_DEBOUNCE_MS, load,
    load_from, merge_document, parse as parse_settings_document, save_document,
};
pub use project::{
    EffectiveToolchain, MavenConfiguration, MavenDiscovery, Overrides, PROJECT_DIAGNOSTIC_TAG,
    ProjectEnvironment, ToolMode, ToolSource, discover as discover_project_environment,
    resolve_overrides,
};
pub use recent_projects::{
    LaunchRootPick, LoadedRecentProjects, MAX_RECENT_PROJECTS, RECENT_PROJECTS_DIAGNOSTIC_TAG,
    RECENT_PROJECTS_KEY, RecentProject, RecentProjects, folder_name as recent_project_name,
    load as load_recent_projects, load_from as load_recent_projects_from, now_unix_ms,
    save as save_recent_projects,
};
pub use restart::restart_application;
pub use schema::Settings;
pub use store::{AppearanceMode, Init, SettingsStore, init_store, store, try_store};
pub use theme::watch_lithe_themes;
pub use workspace::{
    AppearanceKey, AppearanceKeySource, AppearanceOverlay, AppearanceSource, AppearanceValue,
    WORKSPACE_SETTINGS_VERSION,
};
