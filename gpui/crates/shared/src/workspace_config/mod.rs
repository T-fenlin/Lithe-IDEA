//! 工作区配置：`.lithe/` 的路径真源、身份清单，以及"默认不共享"的守卫。
//!
//! ## 这里放什么
//!
//! - [`paths`]：`.lithe/` 目录及其成员的**唯一**命名与构造点（仓库里 `.lithe` 这个字面量
//!   在此之前没有任何具名常量）；
//! - [`project`]：`.lithe/project.json` 的读写 —— 稳定项目身份（UUID v4），
//!   没有它时回落到 Core 的路径身份；
//! - [`toolchain`]：工具链五个值的**项目本机层**（`.lithe/run.local.json` 的 `toolchain` 对象
//!   与 `.lithe/maven.local.json`），以及 `项目本机 > 全局默认 > 自动发现` 的解析；
//! - [`sharing`]：把"默认不共享"落到磁盘上的两道闸（本机排除文件 + `.lithe/.gitignore`），
//!   以及显式的"共享此项目的配置"动作。
//!
//! ## 与设计 Note 的关系
//!
//! 分层与文件形状的真源是
//! `.agents/notes/proposed/architecture/2026-09-26-workspace-configuration-layers.md`
//! （四层模型：全局 / 项目可共享 / 项目本机 / 派生物）。本模块只实现其中的**项目侧**：
//! 路径、身份、"默认不共享"。全局层仍归 `lithe-gpui-settings` 的 `paths`。
//!
//! ## 为什么在 `shared`
//!
//! 本模块有两类真实使用方，而不是"提前共享"：
//!
//! - `.lithe/` 的路径与身份会被多个 feature 读（写清单的调用方、按项目 id 分桶的会话/历史）；
//! - "默认不共享"这条守卫需要调 Core（`git.write`），而 Core 的信封层就在本 crate
//!   （[`crate::core_client`]）。放在别处就得复制信封拼装，或为它单开一个 crate。
//!
//! ## 这个模块**不做**什么
//!
//! - 不做界面：`sharing::share_project_config` 只提供可调用函数，入口留给下一批。
//! - 不替换既有实现里的 `.lithe` 裸字面量（Core / macOS / Windows 各自的解析保持原样），
//!   那属于跨端契约的后续收敛工作。
//! - 不支持多根工作区：路径都相对**用户打开的那个根**（与 `.idea` 同构）。

pub mod paths;
pub mod project;
pub mod sharing;
pub mod toolchain;
#[cfg(test)]
pub(crate) mod test_repo;

/// 本模块诊断行的前缀（与 `S1_SETTINGS` / `S1_THEME` 同一口径，可 grep）。
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
pub use sharing::{
    EnsureExcluded, IgnoreFileOutcome, LOCAL_IGNORE_RULES, ShareOutcome, SharingError,
    ensure_local_ignore_file, ensure_project_dir_excluded, pattern_is_present, share_project_config,
    shareable_members,
};
pub use toolchain::{
    LocalToolchainState, MAVEN_LOCAL_DOCUMENT_VERSION, OverrideOrigin, OverrideOrigins,
    RUN_LOCAL_DOCUMENT_VERSION, ToolchainPaths, ToolchainSaved, load as load_local_toolchain,
    save as save_local_toolchain,
};
