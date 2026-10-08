use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use lithe_gpui_shared::workspace_config::{OverrideOrigins, ToolchainPaths};

pub const PROJECT_DIAGNOSTIC_TAG: &str = "S1_SETTINGS_PROJECT";

pub const JAVA_EXECUTABLE_ENV: &str = "LITHE_JDTLS_JAVA";
pub const JAVA_HOME_ENV: &str = "JAVA_HOME";
pub const MAVEN_HOME_ENV: &str = "MAVEN_HOME";
pub const M2_HOME_ENV: &str = "M2_HOME";
pub const PATH_ENV: &str = "PATH";
pub const USER_PROFILE_ENV: &str = "USERPROFILE";
pub const HOME_ENV: &str = "HOME";
pub const INSTALL_ROOT_ENVS: [&str; 5] = [
    "ProgramFiles",
    "ProgramFiles(x86)",
    "LOCALAPPDATA",
    "ProgramData",
    "XDG_DATA_HOME",
];
pub const MAX_INSTALLED_JDK_CANDIDATES: usize = 24;
pub const M2_DIRECTORY: &str = ".m2";
pub const SETTINGS_FILE_NAME: &str = "settings.xml";
pub const REPOSITORY_DIRECTORY: &str = "repository";
pub const CONF_DIRECTORY: &str = "conf";
pub const BIN_DIRECTORY: &str = "bin";

pub const MINIMUM_JAVA_MAJOR: u64 = 21;

pub fn jdk_major_version(version: &str) -> u64 {
    let mut parts = version.split(['.', '_', '-', '+']);
    let first = parts.next().unwrap_or_default();
    if first == "1" {
        return parts
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
    }
    first.parse().unwrap_or(0)
}

pub fn meets_language_service_requirement(version: &str) -> bool {
    jdk_major_version(version) >= MINIMUM_JAVA_MAJOR
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSource {
    EnvOverride,
    JavaHome,
    MavenHome,
    Path,
    InstalledRoot,
}

impl ToolSource {
    pub fn id(self) -> &'static str {
        match self {
            Self::EnvOverride => "env",
            Self::JavaHome => "java_home",
            Self::MavenHome => "maven_home",
            Self::Path => "path",
            Self::InstalledRoot => "installed_root",
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::JavaHome => "lithe.toolchain.source.javaHome",
            Self::Path => "lithe.toolchain.source.path",
            Self::InstalledRoot => "lithe.toolchain.source.detected",
            Self::EnvOverride => "lithe.settings.gpui.sourceFromEnv",
            Self::MavenHome => "lithe.settings.gpui.sourceFromEnv",
        }
    }

    pub fn label_arg(self) -> Option<&'static str> {
        match self {
            Self::EnvOverride => Some(JAVA_EXECUTABLE_ENV),
            Self::MavenHome => Some(MAVEN_HOME_ENV),
            Self::JavaHome | Self::Path | Self::InstalledRoot => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMode {
    Automatic,
    Configured,
    ProjectJdk,
}

impl ToolMode {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Automatic => "lithe.toolchain.mode.automatic",
            Self::Configured => "lithe.toolchain.mode.configured",
            Self::ProjectJdk => "lithe.toolchain.mode.projectJdk",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Configured => "configured",
            Self::ProjectJdk => "project_jdk",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedJdk {
    pub home: String,
    pub executable: String,
    pub version: String,
    pub meets_language_service: bool,
    pub source: ToolSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedMaven {
    pub executable: String,
    pub version: String,
    pub home: Option<String>,
    pub java_runtime: Option<String>,
    pub source: ToolSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedTool {
    pub path: String,
    pub version: String,
    pub meets_language_service: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverrideProbe {
    pub path: String,
    pub result: Result<DetectedTool, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JdkDiscovery {
    pub overridden: Option<OverrideProbe>,
    pub candidates: Vec<DetectedJdk>,
    pub rejected: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MavenDiscovery {
    pub overridden: Option<OverrideProbe>,
    pub candidates: Vec<DetectedMaven>,
    pub rejected: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalRepositorySource {
    SettingsXml,
    MavenDefault,
}

impl LocalRepositorySource {
    pub fn id(self) -> &'static str {
        match self {
            Self::SettingsXml => "settings_xml",
            Self::MavenDefault => "maven_default",
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::SettingsXml => "lithe.settings.gpui.mavenLocalRepositoryFromSettings",
            Self::MavenDefault => "lithe.settings.gpui.mavenLocalRepositoryDefault",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRepository {
    pub path: String,
    pub source: LocalRepositorySource,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MavenConfiguration {
    pub user_settings: Option<String>,
    pub installation_settings: Option<String>,
    pub override_settings: Option<String>,
    pub override_settings_missing: bool,
    pub local_repository: Option<LocalRepository>,
}

impl MavenConfiguration {
    pub fn effective_settings(&self) -> Option<&str> {
        self.override_settings
            .as_deref()
            .or(self.user_settings.as_deref())
            .or(self.installation_settings.as_deref())
    }

    pub fn settings_is_overridden(&self) -> bool {
        self.override_settings.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overrides {
    pub java_home: String,
    pub maven_executable: String,
    pub maven_java_home: String,
    pub maven_settings: String,
    pub maven_local_repository: String,
}

impl Overrides {
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn count(&self) -> usize {
        [
            &self.java_home,
            &self.maven_executable,
            &self.maven_java_home,
            &self.maven_settings,
            &self.maven_local_repository,
        ]
        .iter()
        .filter(|value| !value.trim().is_empty())
        .count()
    }

    pub fn from_settings(settings: &crate::schema::Settings) -> Self {
        Self {
            java_home: settings.java_home_path.clone(),
            maven_executable: settings.maven_executable_path.clone(),
            maven_java_home: settings.maven_java_home_path.clone(),
            maven_settings: settings.maven_settings_path.clone(),
            maven_local_repository: settings.maven_local_repository_path.clone(),
        }
    }

    pub fn from_local(local: &ToolchainPaths) -> Self {
        Self {
            java_home: local.java_home_path.clone(),
            maven_executable: local.maven_executable_path.clone(),
            maven_java_home: local.maven_java_home_path.clone(),
            maven_settings: local.maven_settings_path.clone(),
            maven_local_repository: local.maven_local_repository_path.clone(),
        }
    }

    pub fn to_local(&self) -> ToolchainPaths {
        ToolchainPaths {
            java_home_path: self.java_home.clone(),
            maven_executable_path: self.maven_executable.clone(),
            maven_java_home_path: self.maven_java_home.clone(),
            maven_settings_path: self.maven_settings.clone(),
            maven_local_repository_path: self.maven_local_repository.clone(),
        }
    }
}

pub fn resolve_overrides(
    local: &ToolchainPaths,
    global: &Overrides,
) -> (Overrides, OverrideOrigins) {
    let global_paths = global.to_local();
    let (resolved, origins) = ToolchainPaths::resolve(local, &global_paths);
    (Overrides::from_local(&resolved), origins)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectiveToolchain {
    Resolved {
        mode: ToolMode,
        path: String,
        version: Option<String>,
        source: Option<ToolSource>,
    },
    Unusable {
        path: String,
        reason: String,
        language_service_rejection: bool,
    },
    NotFound {
        reason: String,
    },
}

impl EffectiveToolchain {
    pub fn diagnostic_value(&self) -> String {
        match self {
            Self::Resolved { path, .. } => path.clone(),
            Self::Unusable { path, .. } => format!("unusable({path})"),
            Self::NotFound { .. } => "none".to_string(),
        }
    }

    pub fn diagnostic_mode(&self) -> &'static str {
        match self {
            Self::Resolved { mode, .. } => mode.id(),
            Self::Unusable { .. } => "unusable",
            Self::NotFound { .. } => "none",
        }
    }
}

impl JdkDiscovery {
    pub fn detected(&self) -> Option<&DetectedJdk> {
        self.candidates
            .iter()
            .find(|jdk| jdk.meets_language_service)
    }

    pub fn closest_below_requirement(&self) -> Option<&DetectedJdk> {
        self.candidates.first()
    }

    pub fn effective(&self) -> EffectiveToolchain {
        if let Some(overridden) = &self.overridden {
            return match &overridden.result {
                Ok(tool) => {
                    if tool.meets_language_service {
                        EffectiveToolchain::Resolved {
                            mode: ToolMode::Configured,
                            path: tool.path.clone(),
                            version: non_empty(&tool.version),
                            source: None,
                        }
                    } else {
                        EffectiveToolchain::Unusable {
                            path: overridden.path.clone(),
                            reason: below_requirement_reason(&tool.version),
                            language_service_rejection: true,
                        }
                    }
                }
                Err(reason) => EffectiveToolchain::Unusable {
                    path: overridden.path.clone(),
                    reason: reason.clone(),
                    language_service_rejection: false,
                },
            };
        }
        if let Some(jdk) = self.detected() {
            return EffectiveToolchain::Resolved {
                mode: ToolMode::Automatic,
                path: jdk.home.clone(),
                version: non_empty(&jdk.version),
                source: Some(jdk.source),
            };
        }
        if let Some(jdk) = self.closest_below_requirement() {
            let path = if jdk.home.is_empty() {
                jdk.executable.clone()
            } else {
                jdk.home.clone()
            };
            return EffectiveToolchain::Unusable {
                path,
                reason: below_requirement_reason(&jdk.version),
                language_service_rejection: true,
            };
        }
        EffectiveToolchain::NotFound {
            reason: self.rejection_summary(),
        }
    }

    pub fn rejection_summary(&self) -> String {
        rejection_summary(
            &self.rejected,
            &[
                format!("{JAVA_EXECUTABLE_ENV}=未设置"),
                format!("{JAVA_HOME_ENV}=未设置"),
                format!("{PATH_ENV}=已逐目录查找 {}", java_executable_name()),
            ],
        )
    }
}

impl MavenDiscovery {
    pub fn detected(&self) -> Option<&DetectedMaven> {
        self.candidates.first()
    }

    pub fn effective(&self) -> EffectiveToolchain {
        if let Some(overridden) = &self.overridden {
            return match &overridden.result {
                Ok(tool) => EffectiveToolchain::Resolved {
                    mode: ToolMode::Configured,
                    path: tool.path.clone(),
                    version: non_empty(&tool.version),
                    source: None,
                },
                Err(reason) => EffectiveToolchain::Unusable {
                    path: overridden.path.clone(),
                    reason: reason.clone(),
                    language_service_rejection: false,
                },
            };
        }
        match self.detected() {
            Some(maven) => EffectiveToolchain::Resolved {
                mode: ToolMode::Automatic,
                path: maven.executable.clone(),
                version: non_empty(&maven.version),
                source: Some(maven.source),
            },
            None => EffectiveToolchain::NotFound {
                reason: rejection_summary(
                    &self.rejected,
                    &[
                        format!("{MAVEN_HOME_ENV}={M2_HOME_ENV}=未设置"),
                        format!("{PATH_ENV}=已逐目录查找 mvn"),
                    ],
                ),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectEnvironment {
    pub overrides: Overrides,
    pub jdk: JdkDiscovery,
    pub maven: MavenDiscovery,
    pub maven_config: MavenConfiguration,
    pub maven_jdk: JdkDiscovery,
}

impl ProjectEnvironment {
    pub fn has_no_toolchain(&self) -> bool {
        matches!(self.jdk.effective(), EffectiveToolchain::NotFound { .. })
            && matches!(self.maven.effective(), EffectiveToolchain::NotFound { .. })
    }

    pub fn maven_jdk_effective(&self) -> EffectiveToolchain {
        if self.maven_jdk.overridden.is_some() {
            return self.maven_jdk.effective();
        }
        match self.jdk.effective() {
            EffectiveToolchain::Resolved { path, version, .. } => EffectiveToolchain::Resolved {
                mode: ToolMode::ProjectJdk,
                path,
                version,
                source: None,
            },
            other => other,
        }
    }

    pub fn diagnostic_line(&self) -> String {
        let jdk = self.jdk.effective();
        let maven = self.maven.effective();
        let maven_jdk = self.maven_jdk_effective();
        let language_service = if matches!(jdk, EffectiveToolchain::Resolved { .. }) {
            "ready"
        } else {
            "unavailable"
        };
        let jdk_source = match &jdk {
            EffectiveToolchain::Resolved {
                source: Some(source),
                ..
            } => source.id(),
            EffectiveToolchain::Resolved { mode, .. } => mode.id(),
            other => other.diagnostic_mode(),
        };
        let maven_source = match &maven {
            EffectiveToolchain::Resolved {
                source: Some(source),
                ..
            } => source.id(),
            EffectiveToolchain::Resolved { mode, .. } => mode.id(),
            other => other.diagnostic_mode(),
        };
        format!(
            "{PROJECT_DIAGNOSTIC_TAG} jdk={} source={} version={} mode={} maven={} mavenVersion={} mavenHome={} mavenSource={} mavenJdk={} mavenJdkMode={} settings={} settingsInstallation={} localRepo={} localRepoSource={} settingsOverridden={} overrideJdk={} overrideMaven={} overrideMavenJdk={} overrideSettings={} overrideLocalRepo={} candidates={} rejected={} minimumJava={} languageService={}",
            jdk.diagnostic_value(),
            jdk_source,
            jdk_version(&jdk).unwrap_or_else(|| "-".to_string()),
            jdk.diagnostic_mode(),
            maven.diagnostic_value(),
            maven_version(&maven).unwrap_or_else(|| "-".to_string()),
            self.maven
                .detected()
                .and_then(|maven| maven.home.clone())
                .unwrap_or_else(|| "-".to_string()),
            maven_source,
            maven_jdk.diagnostic_value(),
            maven_jdk.diagnostic_mode(),
            self.maven_config
                .effective_settings()
                .unwrap_or("-")
                .to_string(),
            self.maven_config
                .installation_settings
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            self.maven_config
                .local_repository
                .as_ref()
                .map(|repository| repository.path.clone())
                .unwrap_or_else(|| "-".to_string()),
            self.maven_config
                .local_repository
                .as_ref()
                .map(|repository| repository.source.id())
                .unwrap_or("none"),
            if self.maven_config.settings_is_overridden() {
                "yes"
            } else {
                "no"
            },
            self.overrides.java_home,
            self.overrides.maven_executable,
            self.overrides.maven_java_home,
            self.overrides.maven_settings,
            self.overrides.maven_local_repository,
            self.jdk.candidates.len() + self.maven.candidates.len(),
            self.jdk.rejected.len() + self.maven.rejected.len(),
            MINIMUM_JAVA_MAJOR,
            language_service,
        )
    }
}

fn jdk_version(effective: &EffectiveToolchain) -> Option<String> {
    match effective {
        EffectiveToolchain::Resolved { version, .. } => version.clone(),
        EffectiveToolchain::Unusable { reason, .. } => version_in_reason(reason),
        EffectiveToolchain::NotFound { .. } => None,
    }
}

fn version_in_reason(reason: &str) -> Option<String> {
    let rest = reason.strip_prefix("版本 ")?;
    let version = rest.split_whitespace().next()?;
    (!version.is_empty()).then(|| version.to_string())
}

fn maven_version(effective: &EffectiveToolchain) -> Option<String> {
    match effective {
        EffectiveToolchain::Resolved { version, .. } => version.clone(),
        _ => None,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvSnapshot {
    pub java_executable: Option<String>,
    pub java_home: Option<String>,
    pub maven_home: Option<String>,
    pub m2_home: Option<String>,
    pub path: Option<String>,
    pub user_home: Option<String>,
    pub install_roots: Vec<(String, String)>,
}

impl EnvSnapshot {
    pub fn from_env() -> Self {
        Self {
            java_executable: read_env(JAVA_EXECUTABLE_ENV),
            java_home: read_env(JAVA_HOME_ENV),
            maven_home: read_env(MAVEN_HOME_ENV),
            m2_home: read_env(M2_HOME_ENV),
            path: read_env(PATH_ENV),
            user_home: read_env(USER_PROFILE_ENV).or_else(|| read_env(HOME_ENV)),
            install_roots: INSTALL_ROOT_ENVS
                .iter()
                .filter_map(|name| read_env(name).map(|value| (name.to_string(), value)))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub source: ToolSource,
    pub path: PathBuf,
}

pub fn java_candidates(env: &EnvSnapshot) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    push_candidate(
        &mut candidates,
        ToolSource::EnvOverride,
        env.java_executable.as_deref(),
    );
    push_candidate(
        &mut candidates,
        ToolSource::JavaHome,
        env.java_home.as_deref(),
    );
    if let Some(path) = env.path.as_deref() {
        for directory in std::env::split_paths(path) {
            candidates.push(Candidate {
                source: ToolSource::Path,
                path: directory,
            });
        }
    }
    for root in installed_jdk_roots(env) {
        candidates.push(Candidate {
            source: ToolSource::InstalledRoot,
            path: root,
        });
    }
    candidates
}

pub fn installed_jdk_roots(env: &EnvSnapshot) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(home) = non_empty_opt(env.user_home.as_deref()) {
        let home = Path::new(home);
        roots.push(home.join(".jdks"));
        roots.push(home.join(".sdkman").join("candidates").join("java"));
        roots.push(home.join("scoop").join("apps"));
    }
    for (name, base) in &env.install_roots {
        let base = Path::new(base);
        match name.as_str() {
            "ProgramData" => roots.push(base.join("java")),
            "XDG_DATA_HOME" => roots.push(base.join("jdks")),
            _ => {
                for vendor in INSTALL_ROOT_VENDORS {
                    roots.push(base.join(vendor));
                }
            }
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    for root in roots {
        push_root_candidates(&mut candidates, &root);
        if candidates.len() >= MAX_INSTALLED_JDK_CANDIDATES {
            break;
        }
    }
    candidates.truncate(MAX_INSTALLED_JDK_CANDIDATES);
    candidates
}

pub const INSTALL_ROOT_VENDORS: [&str; 7] = [
    "Java",
    "Eclipse Adoptium",
    "Microsoft",
    "Amazon Corretto",
    "Zulu",
    "BellSoft",
    "Programs",
];

fn push_root_candidates(candidates: &mut Vec<PathBuf>, root: &Path) {
    candidates.push(root.to_path_buf());
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut directories: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect();
    directories.sort();
    candidates.extend(directories);
}

pub fn maven_candidates(env: &EnvSnapshot) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    for (source, home) in [
        (ToolSource::MavenHome, env.maven_home.as_deref()),
        (ToolSource::MavenHome, env.m2_home.as_deref()),
    ] {
        let Some(home) = non_empty_opt(home) else {
            continue;
        };
        for name in MAVEN_LAUNCHER_NAMES {
            candidates.push(Candidate {
                source,
                path: Path::new(home).join(BIN_DIRECTORY).join(name),
            });
        }
    }
    if let Some(path) = env.path.as_deref() {
        for directory in std::env::split_paths(path) {
            for name in MAVEN_LAUNCHER_NAMES {
                candidates.push(Candidate {
                    source: ToolSource::Path,
                    path: directory.join(name),
                });
            }
        }
    }
    candidates
}

pub const MAVEN_LAUNCHER_NAMES: [&str; 4] = ["mvn.cmd", "mvn.bat", "mvn.exe", "mvn"];

pub fn java_executable_name() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

pub fn discover(overrides: &Overrides) -> ProjectEnvironment {
    let env = EnvSnapshot::from_env();

    let mut jdk = JdkDiscovery::default();
    if let Some(path) = non_empty_opt(Some(&overrides.java_home)) {
        jdk.overridden = Some(OverrideProbe {
            path: path.to_string(),
            result: probe_jdk(path),
        });
    }
    for candidate in java_candidates(&env) {
        match probe_java_candidate(&candidate) {
            CandidateProbe::Found(jdk_found) => jdk.candidates.push(jdk_found),
            CandidateProbe::Absent => {
                if absent_is_reportable(candidate.source, &candidate.path) {
                    jdk.rejected.push(format!(
                        "{}（找不到 {}）",
                        candidate.path.display(),
                        java_executable_name()
                    ));
                }
            }
            CandidateProbe::Failed(reason) => jdk
                .rejected
                .push(format!("{}（{reason}）", candidate.path.display())),
        }
    }
    deduplicate_jdks(&mut jdk.candidates);

    let mut maven = MavenDiscovery::default();
    if let Some(path) = non_empty_opt(Some(&overrides.maven_executable)) {
        maven.overridden = Some(OverrideProbe {
            path: path.to_string(),
            result: probe_maven(path),
        });
    }
    for candidate in maven_candidates(&env) {
        if !candidate.path.is_file() {
            continue;
        }
        match probe_maven_executable(&candidate.path.to_string_lossy(), candidate.source) {
            Ok(found) => maven.candidates.push(found),
            Err(reason) => maven
                .rejected
                .push(format!("{}（{reason}）", candidate.path.display())),
        }
    }
    deduplicate_maven(&mut maven.candidates);

    let mut maven_jdk = JdkDiscovery::default();
    if let Some(path) = non_empty_opt(Some(&overrides.maven_java_home)) {
        maven_jdk.overridden = Some(OverrideProbe {
            path: path.to_string(),
            result: probe_jdk(path),
        });
    }

    let maven_config = maven_configuration(
        &maven,
        env.user_home.as_deref(),
        Some(&overrides.maven_settings),
    );
    let environment = ProjectEnvironment {
        overrides: overrides.clone(),
        jdk,
        maven,
        maven_config,
        maven_jdk,
    };
    println!("{}", environment.diagnostic_line());
    environment
}

fn deduplicate_jdks(candidates: &mut Vec<DetectedJdk>) {
    let mut seen = BTreeSet::new();
    candidates.retain(|jdk| seen.insert(jdk.executable.clone()));
}

fn deduplicate_maven(candidates: &mut Vec<DetectedMaven>) {
    let mut seen = BTreeSet::new();
    candidates.retain(|maven| seen.insert(maven.executable.clone()));
}

pub fn below_requirement_reason(version: &str) -> String {
    format!("版本 {version} 低于 {MINIMUM_JAVA_MAJOR}，语言服务无法启动")
}

fn probe_jdk(path: &str) -> Result<DetectedTool, String> {
    let path = Path::new(path);
    let executable = java_executable_in(path).ok_or_else(|| {
        format!(
            "找不到 {}（查过 {}）",
            java_executable_name(),
            path.display()
        )
    })?;
    let version = probe_java_version(&executable)?;
    Ok(DetectedTool {
        path: home_of(&executable)
            .map(|home| home.display().to_string())
            .unwrap_or_else(|| path.display().to_string()),
        meets_language_service: meets_language_service_requirement(&version),
        version,
    })
}

fn probe_maven(path: &str) -> Result<DetectedTool, String> {
    let path = Path::new(path);
    let executable = if path.is_file() {
        path.to_path_buf()
    } else {
        let mut found = None;
        for name in MAVEN_LAUNCHER_NAMES {
            let candidate = path.join(BIN_DIRECTORY).join(name);
            if candidate.is_file() {
                found = Some(candidate);
                break;
            }
            let candidate = path.join(name);
            if candidate.is_file() {
                found = Some(candidate);
                break;
            }
        }
        found.ok_or_else(|| format!("找不到 mvn 启动器（查过 {}）", path.display()))?
    };
    let probe = probe_maven_executable(&executable.to_string_lossy(), ToolSource::Path)?;
    Ok(DetectedTool {
        path: executable.display().to_string(),
        version: probe.version,
        meets_language_service: true,
    })
}

enum CandidateProbe {
    Found(DetectedJdk),
    Absent,
    Failed(String),
}

fn probe_java_candidate(candidate: &Candidate) -> CandidateProbe {
    let Some(executable) = java_executable_in(&candidate.path) else {
        return CandidateProbe::Absent;
    };
    match probe_java_version(&executable) {
        Ok(version) => CandidateProbe::Found(DetectedJdk {
            home: home_of(&executable)
                .map(|home| home.display().to_string())
                .unwrap_or_default(),
            executable: executable.display().to_string(),
            meets_language_service: meets_language_service_requirement(&version),
            version,
            source: candidate.source,
        }),
        Err(reason) => CandidateProbe::Failed(reason),
    }
}

fn absent_is_reportable(source: ToolSource, path: &Path) -> bool {
    match source {
        ToolSource::EnvOverride | ToolSource::JavaHome => true,
        ToolSource::InstalledRoot => path.is_dir(),
        ToolSource::MavenHome | ToolSource::Path => false,
    }
}

fn probe_maven_executable(executable: &str, source: ToolSource) -> Result<DetectedMaven, String> {
    let output = run_version(Path::new(executable), &["-version"])?;
    let version = maven_version_text(&output).unwrap_or_default();
    Ok(DetectedMaven {
        executable: executable.to_string(),
        version,
        home: maven_home_text(&output),
        java_runtime: java_runtime_text(&output),
        source,
    })
}

fn java_executable_in(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    for name in [java_executable_name(), "java"] {
        let candidate = path.join(BIN_DIRECTORY).join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn home_of(executable: &Path) -> Option<PathBuf> {
    executable.parent()?.parent().map(Path::to_path_buf)
}

fn probe_java_version(executable: &Path) -> Result<String, String> {
    let output = run_version(executable, &["-version"])?;
    java_version_text(&output).ok_or_else(|| "读不出版本号".to_string())
}

fn run_version(executable: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(executable)
        .args(args)
        .output()
        .map_err(|error| format!("无法执行：{error}"))?;
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

pub fn java_version_text(output: &str) -> Option<String> {
    for line in output.lines() {
        let Some(index) = line.find("version") else {
            continue;
        };
        let rest = line[index + "version".len()..].trim();
        if let Some(quoted) = rest.strip_prefix('"') {
            return non_empty(quoted.split('"').next().unwrap_or_default());
        }
        return rest
            .split_whitespace()
            .next()
            .and_then(|value| non_empty(value));
    }
    None
}

pub fn maven_version_text(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("Apache Maven")?;
        rest.split_whitespace()
            .next()
            .and_then(|value| non_empty(value))
    })
}

pub fn maven_home_text(output: &str) -> Option<String> {
    labeled_line(output, "Maven home:").map(|value| normalize_reported_path(&value))
}

pub fn normalize_reported_path(path: &str) -> String {
    let path = Path::new(path.trim());
    let mut stack: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match stack.last() {
                Some(Component::Normal(_)) => {
                    stack.pop();
                }
                _ => stack.push(component),
            },
            other => stack.push(other),
        }
    }
    let mut normalized = PathBuf::new();
    for component in stack {
        normalized.push(component.as_os_str());
    }
    if normalized.as_os_str().is_empty() {
        return path.display().to_string();
    }
    normalized.display().to_string()
}

pub fn java_runtime_text(output: &str) -> Option<String> {
    let value = labeled_line(output, "Java version:")?;
    non_empty(value.split(',').next().unwrap_or_default())
}

fn labeled_line(output: &str, label: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix(label))
        .map(str::trim)
        .and_then(|value| non_empty(value))
}

pub fn maven_configuration(
    maven: &MavenDiscovery,
    user_home: Option<&str>,
    settings_override: Option<&str>,
) -> MavenConfiguration {
    let m2 = user_home.map(|home| Path::new(home).join(M2_DIRECTORY));
    let user_settings = m2
        .as_ref()
        .map(|directory| directory.join(SETTINGS_FILE_NAME))
        .filter(|path| path.is_file())
        .map(|path| path.display().to_string());
    let installation_settings = maven
        .detected()
        .and_then(|maven| installation_settings_path(&maven.executable))
        .map(|path| path.display().to_string());

    let override_settings = non_empty_opt(settings_override).map(str::to_string);
    let override_settings_missing = override_settings
        .as_deref()
        .is_some_and(|path| !Path::new(path).is_file());

    let local_repository = local_repository(
        override_settings
            .as_deref()
            .or(user_settings.as_deref())
            .map(Path::new),
        m2.as_deref(),
    );

    MavenConfiguration {
        user_settings,
        installation_settings,
        override_settings,
        override_settings_missing,
        local_repository,
    }
}

pub fn installation_settings_path(maven_executable: &str) -> Option<PathBuf> {
    let configured = Path::new(maven_executable);
    let home = if configured.is_dir() {
        configured.to_path_buf()
    } else {
        let bin = configured.parent()?;
        if !bin
            .file_name()?
            .to_str()?
            .eq_ignore_ascii_case(BIN_DIRECTORY)
        {
            return None;
        }
        bin.parent()?.to_path_buf()
    };
    let settings = home.join(CONF_DIRECTORY).join(SETTINGS_FILE_NAME);
    settings.is_file().then_some(settings)
}

fn local_repository(user_settings: Option<&Path>, m2: Option<&Path>) -> Option<LocalRepository> {
    if let Some(settings) = user_settings {
        if let Some(declared) = read_local_repository(settings) {
            return Some(LocalRepository {
                path: declared,
                source: LocalRepositorySource::SettingsXml,
            });
        }
    }
    let m2 = m2?;
    Some(LocalRepository {
        path: m2.join(REPOSITORY_DIRECTORY).display().to_string(),
        source: LocalRepositorySource::MavenDefault,
    })
}

pub fn read_local_repository(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    local_repository_in(&text)
}

pub fn local_repository_in(text: &str) -> Option<String> {
    let start = text.find("<localRepository>")? + "<localRepository>".len();
    let rest = &text[start..];
    let end = rest.find("</localRepository>")?;
    let value = rest[..end].trim();
    if value.is_empty() || value.contains('<') {
        return None;
    }
    Some(value.to_string())
}

fn rejection_summary(rejected: &[String], levels: &[String]) -> String {
    let mut parts = levels.to_vec();
    if !rejected.is_empty() {
        parts.push(format!("已排除：{}", rejected.join("；")));
    }
    parts.join("；")
}

fn push_candidate(candidates: &mut Vec<Candidate>, source: ToolSource, path: Option<&str>) {
    if let Some(path) = non_empty_opt(path) {
        candidates.push(Candidate {
            source,
            path: PathBuf::from(path),
        });
    }
}

fn read_env(name: &str) -> Option<String> {
    let value = std::env::var(name).ok();
    non_empty_opt(value.as_deref()).map(str::to_string)
}

fn non_empty_opt(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_map_field_by_field_to_the_shared_toolchain_paths() {
        let settings = crate::schema::Settings {
            java_home_path: "java-home".to_string(),
            maven_executable_path: "maven-executable".to_string(),
            maven_java_home_path: "maven-java-home".to_string(),
            maven_settings_path: "maven-settings".to_string(),
            maven_local_repository_path: "maven-local-repository".to_string(),
            ..crate::schema::Settings::default()
        };

        let overrides = Overrides::from_settings(&settings);
        let paths = overrides.to_local();
        assert_eq!(paths.java_home_path, "java-home");
        assert_eq!(paths.maven_executable_path, "maven-executable");
        assert_eq!(paths.maven_java_home_path, "maven-java-home");
        assert_eq!(paths.maven_settings_path, "maven-settings");
        assert_eq!(paths.maven_local_repository_path, "maven-local-repository");

        let back = Overrides::from_local(&paths);
        assert_eq!(back, overrides);
    }

    #[test]
    fn resolve_overrides_prefers_the_project_layer() {
        let global = Overrides {
            java_home: "global-jdk".to_string(),
            maven_settings: "global-settings".to_string(),
            ..Overrides::default()
        };
        let local = ToolchainPaths {
            java_home_path: "project-jdk".to_string(),
            ..ToolchainPaths::default()
        };

        let (resolved, origins) = resolve_overrides(&local, &global);
        assert_eq!(resolved.java_home, "project-jdk");
        assert_eq!(resolved.maven_settings, "global-settings");
        assert_eq!(resolved.maven_executable, "");
        assert_eq!(
            origins.java_home_path,
            lithe_gpui_shared::workspace_config::OverrideOrigin::Project
        );
        assert_eq!(
            origins.maven_settings_path,
            lithe_gpui_shared::workspace_config::OverrideOrigin::Global
        );
    }

    #[test]
    fn java_candidates_follow_the_three_level_order() {
        let env = EnvSnapshot {
            java_executable: Some(r"D:\explicit\bin\java.exe".to_string()),
            java_home: Some(r"D:\ProgramData\java\openjdk-21".to_string()),
            path: Some(joined_path(&[r"C:\Windows", r"C:\jdk1.8\bin"])),
            ..EnvSnapshot::default()
        };
        let candidates = java_candidates(&env);
        assert_eq!(candidates[0].source, ToolSource::EnvOverride);
        assert_eq!(
            candidates[0].path,
            PathBuf::from(r"D:\explicit\bin\java.exe")
        );
        assert_eq!(candidates[1].source, ToolSource::JavaHome);
        assert_eq!(
            candidates[1].path,
            PathBuf::from(r"D:\ProgramData\java\openjdk-21")
        );
        let path_candidates = &candidates[2..];
        assert_eq!(
            path_candidates,
            &[
                Candidate {
                    source: ToolSource::Path,
                    path: PathBuf::from(r"C:\Windows"),
                },
                Candidate {
                    source: ToolSource::Path,
                    path: PathBuf::from(r"C:\jdk1.8\bin"),
                },
            ]
        );
    }

    #[test]
    fn installed_roots_enumerate_directories_in_a_deterministic_order() {
        let base = std::env::temp_dir().join(format!(
            "lithe-settings-installed-roots-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let program_data = base.join("ProgramData");
        std::fs::create_dir_all(program_data.join("java").join("openjdk-21")).expect("建目录失败");
        std::fs::create_dir_all(program_data.join("java").join("jdk-11")).expect("建目录失败");
        std::fs::write(program_data.join("java").join("jdk-8u45.exe"), "not a jdk")
            .expect("写文件失败");

        let env = EnvSnapshot {
            install_roots: vec![(
                "ProgramData".to_string(),
                program_data.display().to_string(),
            )],
            ..EnvSnapshot::default()
        };
        let roots = installed_jdk_roots(&env);
        assert_eq!(
            roots,
            vec![
                program_data.join("java"),
                program_data.join("java").join("jdk-11"),
                program_data.join("java").join("openjdk-21"),
            ]
        );

        let candidates = java_candidates(&env);
        assert_eq!(candidates[0].source, ToolSource::InstalledRoot);
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.source == ToolSource::InstalledRoot),
            "这条 env 里没有前三级的任何一项"
        );

        let env = EnvSnapshot {
            install_roots: vec![(
                "ProgramData".to_string(),
                base.join("nope").display().to_string(),
            )],
            ..EnvSnapshot::default()
        };
        assert_eq!(
            installed_jdk_roots(&env),
            vec![base.join("nope").join("java")]
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn installed_root_candidates_are_capped() {
        let base = std::env::temp_dir().join(format!(
            "lithe-settings-installed-cap-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let java = base.join("ProgramData").join("java");
        for index in 0..MAX_INSTALLED_JDK_CANDIDATES + 8 {
            std::fs::create_dir_all(java.join(format!("jdk-{index:03}"))).expect("建目录失败");
        }
        let env = EnvSnapshot {
            install_roots: vec![(
                "ProgramData".to_string(),
                base.join("ProgramData").display().to_string(),
            )],
            ..EnvSnapshot::default()
        };
        let roots = installed_jdk_roots(&env);
        assert_eq!(roots.len(), MAX_INSTALLED_JDK_CANDIDATES);
        assert_eq!(roots[0], java);
        assert_eq!(roots[1], java.join("jdk-000"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn java_candidates_degrade_to_path_only() {
        let env = EnvSnapshot {
            path: Some(joined_path(&[r"C:\Windows", r"C:\jdk8\bin"])),
            ..EnvSnapshot::default()
        };
        let candidates = java_candidates(&env);
        assert_eq!(candidates.len(), 2);
        assert!(
            candidates
                .iter()
                .all(|item| item.source == ToolSource::Path)
        );
        assert_eq!(candidates[0].path, PathBuf::from(r"C:\Windows"));
    }

    fn joined_path(parts: &[&str]) -> String {
        std::env::join_paths(parts.iter().map(PathBuf::from))
            .expect("测试用的 PATH 片段不含平台分隔符")
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn empty_env_values_are_not_candidates() {
        let env = EnvSnapshot {
            java_home: Some("   ".to_string()),
            java_executable: Some(String::new()),
            ..EnvSnapshot::default()
        };
        assert!(java_candidates(&env).is_empty());
        assert!(maven_candidates(&env).is_empty());
    }

    #[test]
    fn absent_candidates_are_only_reported_where_it_means_something() {
        let path = Path::new(r"C:\Windows");
        assert!(absent_is_reportable(ToolSource::EnvOverride, path));
        assert!(absent_is_reportable(ToolSource::JavaHome, path));
        assert!(!absent_is_reportable(ToolSource::Path, path));
        assert!(!absent_is_reportable(ToolSource::MavenHome, path));

        let existing = std::env::temp_dir();
        assert!(absent_is_reportable(ToolSource::InstalledRoot, &existing));
        assert!(!absent_is_reportable(
            ToolSource::InstalledRoot,
            &existing.join("lithe-no-such-root-ever")
        ));
    }

    #[test]
    fn maven_home_drops_the_trailing_parent_segment() {
        let base = std::env::temp_dir().join("lithe-normalize-base");
        let messy = base.join("bin").join("..");
        assert_eq!(
            normalize_reported_path(&messy.to_string_lossy()),
            base.display().to_string()
        );
        let messy = base.join("bin").join("..").join("..");
        let expected = base.parent().expect("临时目录一定有父目录");
        assert_eq!(
            normalize_reported_path(&messy.to_string_lossy()),
            expected.display().to_string()
        );
        let clean = base.join("apache-maven-3.9.9");
        assert_eq!(
            normalize_reported_path(&clean.to_string_lossy()),
            clean.display().to_string()
        );
        assert_eq!(
            normalize_reported_path(&format!("  {}  ", clean.display())),
            clean.display().to_string()
        );
        assert_eq!(normalize_reported_path(".."), "..");

        #[cfg(windows)]
        {
            let output = "Apache Maven 3.6.3 (cecedd343002696d0abb50b32b541b8a6ba2883f)\n\
                          Maven home: D:\\ProgramData\\maven\\apache-maven-3.6.3\\bin\\..\n\
                          Java version: 1.8.0_221, vendor: Oracle Corporation, runtime: D:\\ProgramData\\java\\jdk1.8.0_221\\jre\n";
            assert_eq!(
                maven_home_text(output),
                Some(r"D:\ProgramData\maven\apache-maven-3.6.3".to_string())
            );
            assert_eq!(java_runtime_text(output), Some("1.8.0_221".to_string()));
        }
    }

    #[test]
    fn maven_candidates_prefer_the_installation_home() {
        let env = EnvSnapshot {
            maven_home: Some(r"D:\tools\apache-maven-3.9.9".to_string()),
            path: Some(r"C:\Windows".to_string()),
            ..EnvSnapshot::default()
        };
        let candidates = maven_candidates(&env);
        assert_eq!(candidates[0].source, ToolSource::MavenHome);
        assert_eq!(
            candidates[0].path,
            PathBuf::from(r"D:\tools\apache-maven-3.9.9\bin\mvn.cmd")
        );
        let path_candidate = candidates
            .iter()
            .find(|candidate| candidate.source == ToolSource::Path)
            .expect("PATH 上的候选必须也在");
        assert_eq!(path_candidate.path, PathBuf::from(r"C:\Windows\mvn.cmd"));
    }

    #[test]
    fn java_version_reads_both_banner_shapes() {
        assert_eq!(
            java_version_text("openjdk version \"21.0.2\" 2024-01-16\nOpenJDK Runtime Environment"),
            Some("21.0.2".to_string())
        );
        assert_eq!(
            java_version_text("java version \"1.8.0_402\"\nJava(TM) SE Runtime Environment"),
            Some("1.8.0_402".to_string())
        );
        assert_eq!(
            java_version_text("java version 1.8.0_402"),
            Some("1.8.0_402".to_string())
        );
        assert_eq!(java_version_text("no banner here"), None);
    }

    #[test]
    fn maven_version_reads_the_three_facts() {
        let output = "Apache Maven 3.9.9 (8e8579a9e76f7d015ee5ec7bfcdc97d260186937)\n\
                      Maven home: D:\\tools\\apache-maven-3.9.9\n\
                      Java version: 21.0.2, vendor: Eclipse Adoptium, runtime: D:\\ProgramData\\java\\openjdk-21\n\
                      Default locale: zh_CN, platform encoding: UTF-8\n";
        assert_eq!(maven_version_text(output), Some("3.9.9".to_string()));
        assert_eq!(
            maven_home_text(output),
            Some(r"D:\tools\apache-maven-3.9.9".to_string())
        );
        assert_eq!(java_runtime_text(output), Some("21.0.2".to_string()));
        assert_eq!(maven_version_text("not maven output"), None);
        assert_eq!(maven_home_text("not maven output"), None);
        assert_eq!(java_runtime_text("not maven output"), None);
    }

    #[test]
    fn local_repository_parsing_handles_the_documented_shapes() {
        assert_eq!(
            local_repository_in("<settings><localRepository>D:\\repo</localRepository></settings>"),
            Some(r"D:\repo".to_string())
        );
        assert_eq!(
            local_repository_in(
                "<settings><localRepository>  /opt/repo  </localRepository></settings>"
            ),
            Some("/opt/repo".to_string())
        );
        assert_eq!(
            local_repository_in("<settings><localRepository/></settings>"),
            None
        );
        assert_eq!(
            local_repository_in("<settings><localRepository></localRepository></settings>"),
            None
        );
        assert_eq!(
            local_repository_in("<settings><localRepository><a>x</a></localRepository></settings>"),
            None
        );
        assert_eq!(local_repository_in("<settings/>"), None);
    }

    #[test]
    fn installation_settings_only_apply_to_an_installation_layout() {
        let home = std::env::temp_dir().join(format!(
            "lithe-settings-installation-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("bin")).expect("建临时目录失败");
        std::fs::create_dir_all(home.join("conf")).expect("建临时目录失败");
        std::fs::write(home.join("conf").join("settings.xml"), "<settings/>")
            .expect("写临时文件失败");

        let launcher = home.join("bin").join("mvn.cmd");
        let resolved = installation_settings_path(&launcher.to_string_lossy())
            .expect("`<home>/bin/mvn*` 布局必须能推出 conf/settings.xml");
        assert_eq!(resolved, home.join("conf").join("settings.xml"));

        assert_eq!(
            installation_settings_path(&home.to_string_lossy()),
            Some(home.join("conf").join("settings.xml"))
        );

        let wrapper = home.join("mvnw.cmd");
        assert_eq!(installation_settings_path(&wrapper.to_string_lossy()), None);

        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn local_repository_falls_back_to_the_maven_default() {
        let directory =
            std::env::temp_dir().join(format!("lithe-settings-localrepo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("建临时目录失败");
        let home = directory.join("home");
        std::fs::create_dir_all(home.join(M2_DIRECTORY)).expect("建 .m2 失败");

        let maven = MavenDiscovery::default();
        let configuration = maven_configuration(&maven, Some(&home.to_string_lossy()), None);
        assert_eq!(configuration.user_settings, None);
        assert!(!configuration.settings_is_overridden());
        let repository = configuration.local_repository.expect("必须给出默认位置");
        assert_eq!(
            repository.path,
            home.join(M2_DIRECTORY)
                .join(REPOSITORY_DIRECTORY)
                .display()
                .to_string()
        );
        assert_eq!(repository.source, LocalRepositorySource::MavenDefault);

        std::fs::write(
            home.join(M2_DIRECTORY).join(SETTINGS_FILE_NAME),
            "<settings><localRepository>D:\\maven-repo</localRepository></settings>",
        )
        .expect("写 settings.xml 失败");
        let configuration = maven_configuration(&maven, Some(&home.to_string_lossy()), None);
        assert_eq!(
            configuration.effective_settings(),
            configuration.user_settings.as_deref()
        );
        let repository = configuration.local_repository.expect("必须给出仓库路径");
        assert_eq!(repository.path, r"D:\maven-repo");
        assert_eq!(repository.source, LocalRepositorySource::SettingsXml);

        let configuration = maven_configuration(&maven, None, None);
        assert!(configuration.local_repository.is_none());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_selected_settings_file_wins_and_drives_the_local_repository() {
        let directory = std::env::temp_dir().join(format!(
            "lithe-settings-selected-settings-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let home = directory.join("home");
        std::fs::create_dir_all(home.join(M2_DIRECTORY)).expect("建 .m2 失败");
        std::fs::write(
            home.join(M2_DIRECTORY).join(SETTINGS_FILE_NAME),
            "<settings><localRepository>D:\\auto-repo</localRepository></settings>",
        )
        .expect("写用户级 settings.xml 失败");
        let picked = directory.join("ci-settings.xml");
        std::fs::write(
            &picked,
            "<settings><localRepository>D:\\picked-repo</localRepository></settings>",
        )
        .expect("写选中的 settings.xml 失败");

        let maven = MavenDiscovery::default();
        let configuration = maven_configuration(
            &maven,
            Some(&home.to_string_lossy()),
            Some(&picked.to_string_lossy()),
        );
        assert!(configuration.settings_is_overridden());
        assert!(!configuration.override_settings_missing);
        assert_eq!(
            configuration.effective_settings(),
            Some(picked.to_string_lossy().as_ref())
        );
        assert_eq!(
            configuration.user_settings.as_deref(),
            Some(
                home.join(M2_DIRECTORY)
                    .join(SETTINGS_FILE_NAME)
                    .to_string_lossy()
                    .as_ref()
            )
        );
        let repository = configuration.local_repository.expect("必须给出仓库路径");
        assert_eq!(repository.path, r"D:\picked-repo");
        assert_eq!(repository.source, LocalRepositorySource::SettingsXml);

        let missing = directory.join("nope.xml");
        let configuration = maven_configuration(
            &maven,
            Some(&home.to_string_lossy()),
            Some(&missing.to_string_lossy()),
        );
        assert!(configuration.settings_is_overridden());
        assert!(configuration.override_settings_missing);
        assert_eq!(
            configuration.local_repository.map(|r| r.path),
            Some(
                home.join(M2_DIRECTORY)
                    .join(REPOSITORY_DIRECTORY)
                    .display()
                    .to_string()
            )
        );

        for blank in ["", "   "] {
            let configuration =
                maven_configuration(&maven, Some(&home.to_string_lossy()), Some(blank));
            assert!(!configuration.settings_is_overridden(), "{blank:?}");
            assert_eq!(
                configuration.effective_settings(),
                configuration.user_settings.as_deref()
            );
        }

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn an_override_reports_the_configured_mode_without_a_source() {
        let discovery = JdkDiscovery {
            overridden: Some(OverrideProbe {
                path: r"D:\ProgramData\java\openjdk-21".to_string(),
                result: Ok(DetectedTool {
                    path: r"D:\ProgramData\java\openjdk-21".to_string(),
                    version: "21.0.2".to_string(),
                    meets_language_service: true,
                }),
            }),
            candidates: vec![detected_jdk(
                r"C:\jdk8",
                r"C:\jdk8\bin\java.exe",
                "1.8.0_402",
                ToolSource::Path,
            )],
            rejected: Vec::new(),
        };
        assert_eq!(
            discovery.effective(),
            EffectiveToolchain::Resolved {
                mode: ToolMode::Configured,
                path: r"D:\ProgramData\java\openjdk-21".to_string(),
                version: Some("21.0.2".to_string()),
                source: None,
            }
        );
    }

    #[test]
    fn the_automatic_choice_reports_its_source() {
        let discovery = JdkDiscovery {
            overridden: None,
            candidates: vec![
                detected_jdk(
                    r"D:\ProgramData\java\openjdk-21",
                    r"D:\ProgramData\java\openjdk-21\bin\java.exe",
                    "21.0.2",
                    ToolSource::JavaHome,
                ),
                detected_jdk(
                    r"C:\jdk8",
                    r"C:\jdk8\bin\java.exe",
                    "1.8.0_402",
                    ToolSource::Path,
                ),
            ],
            rejected: Vec::new(),
        };
        assert_eq!(
            discovery.effective(),
            EffectiveToolchain::Resolved {
                mode: ToolMode::Automatic,
                path: r"D:\ProgramData\java\openjdk-21".to_string(),
                version: Some("21.0.2".to_string()),
                source: Some(ToolSource::JavaHome),
            }
        );
        assert_eq!(discovery.candidates.len(), 2);
    }

    fn detected_jdk(
        home: &str,
        executable: &str,
        version: &str,
        source: ToolSource,
    ) -> DetectedJdk {
        DetectedJdk {
            home: home.to_string(),
            executable: executable.to_string(),
            meets_language_service: meets_language_service_requirement(version),
            version: version.to_string(),
            source,
        }
    }

    #[test]
    fn an_unusable_override_is_reported_instead_of_falling_back() {
        let discovery = JdkDiscovery {
            overridden: Some(OverrideProbe {
                path: r"D:\nope".to_string(),
                result: Err("找不到 java.exe（查过 D:\\nope）".to_string()),
            }),
            candidates: vec![detected_jdk(
                r"C:\jdk8",
                r"C:\jdk8\bin\java.exe",
                "1.8.0_402",
                ToolSource::Path,
            )],
            rejected: Vec::new(),
        };
        match discovery.effective() {
            EffectiveToolchain::Unusable {
                reason,
                language_service_rejection,
                ..
            } => {
                assert!(reason.contains("找不到 java.exe"), "{reason}");
                assert!(!language_service_rejection);
            }
            other => panic!("覆盖值用不了时必须报 Unusable，实际 {other:?}"),
        }
    }

    #[test]
    fn nothing_detected_reports_what_was_tried() {
        let discovery = JdkDiscovery {
            overridden: None,
            candidates: Vec::new(),
            rejected: vec![r"C:\jdk8\bin\java.exe（版本 1.8.0_402 低于 21）".to_string()],
        };
        let EffectiveToolchain::NotFound { reason } = discovery.effective() else {
            panic!("没有候选时必须报 NotFound");
        };
        assert!(reason.contains(JAVA_EXECUTABLE_ENV), "{reason}");
        assert!(reason.contains(JAVA_HOME_ENV), "{reason}");
        assert!(reason.contains(PATH_ENV), "{reason}");
        assert!(reason.contains("已排除"), "{reason}");
    }

    #[test]
    fn maven_jdk_follows_the_project_jdk() {
        let environment = ProjectEnvironment {
            overrides: Overrides::default(),
            jdk: JdkDiscovery {
                overridden: None,
                candidates: vec![detected_jdk(
                    r"D:\ProgramData\java\openjdk-21",
                    r"D:\ProgramData\java\openjdk-21\bin\java.exe",
                    "21.0.2",
                    ToolSource::JavaHome,
                )],
                rejected: Vec::new(),
            },
            maven: MavenDiscovery::default(),
            maven_config: MavenConfiguration::default(),
            maven_jdk: JdkDiscovery::default(),
        };
        assert_eq!(
            environment.maven_jdk_effective(),
            EffectiveToolchain::Resolved {
                mode: ToolMode::ProjectJdk,
                path: r"D:\ProgramData\java\openjdk-21".to_string(),
                version: Some("21.0.2".to_string()),
                source: None,
            }
        );
    }
    #[test]
    fn the_diagnostic_line_carries_every_displayed_value() {
        let environment = ProjectEnvironment {
            overrides: Overrides {
                java_home: r"D:\ProgramData\java\openjdk-21".to_string(),
                maven_executable: String::new(),
                maven_java_home: String::new(),
                maven_settings: String::new(),
                maven_local_repository: String::new(),
            },
            jdk: JdkDiscovery {
                overridden: Some(OverrideProbe {
                    path: r"D:\ProgramData\java\openjdk-21".to_string(),
                    result: Ok(DetectedTool {
                        path: r"D:\ProgramData\java\openjdk-21".to_string(),
                        version: "21.0.2".to_string(),
                        meets_language_service: true,
                    }),
                }),
                candidates: vec![detected_jdk(
                    r"C:\jdk8",
                    r"C:\jdk8\bin\java.exe",
                    "1.8.0_402",
                    ToolSource::Path,
                )],
                rejected: Vec::new(),
            },
            maven: MavenDiscovery {
                overridden: None,
                candidates: vec![DetectedMaven {
                    executable: r"D:\tools\apache-maven-3.9.9\bin\mvn.cmd".to_string(),
                    version: "3.9.9".to_string(),
                    home: Some(r"D:\tools\apache-maven-3.9.9".to_string()),
                    java_runtime: Some("21.0.2".to_string()),
                    source: ToolSource::MavenHome,
                }],
                rejected: Vec::new(),
            },
            maven_config: MavenConfiguration {
                user_settings: Some(r"C:\Users\x\.m2\settings.xml".to_string()),
                installation_settings: None,
                override_settings: None,
                override_settings_missing: false,
                local_repository: Some(LocalRepository {
                    path: r"D:\maven-repo".to_string(),
                    source: LocalRepositorySource::SettingsXml,
                }),
            },
            maven_jdk: JdkDiscovery::default(),
        };
        let line = environment.diagnostic_line();
        assert!(line.starts_with(PROJECT_DIAGNOSTIC_TAG), "{line}");
        let jdk_at = line.find("jdk=").expect("缺 jdk=");
        let source_at = line.find("source=").expect("缺 source=");
        let maven_at = line.find("maven=").expect("缺 maven=");
        let local_repo_at = line.find("localRepo=").expect("缺 localRepo=");
        assert!(
            jdk_at < source_at && source_at < maven_at && maven_at < local_repo_at,
            "{line}"
        );
        for expected in [
            r"jdk=D:\ProgramData\java\openjdk-21",
            "source=configured",
            "version=21.0.2",
            "maven=D:\\tools\\apache-maven-3.9.9\\bin\\mvn.cmd",
            "mavenVersion=3.9.9",
            "mavenHome=D:\\tools\\apache-maven-3.9.9",
            "mavenSource=maven_home",
            "settings=C:\\Users\\x\\.m2\\settings.xml",
            "localRepo=D:\\maven-repo",
            "localRepoSource=settings_xml",
            "settingsOverridden=no",
            "overrideSettings= overrideLocalRepo=",
        ] {
            assert!(line.contains(expected), "诊断行缺 {expected}：{line}");
        }

        let mut selected = environment.clone();
        selected.overrides.maven_settings = r"D:\ci\settings.xml".to_string();
        selected.overrides.maven_local_repository = r"D:\repo".to_string();
        selected.maven_config.override_settings = Some(r"D:\ci\settings.xml".to_string());
        let line = selected.diagnostic_line();
        for expected in [
            r"settings=D:\ci\settings.xml",
            "settingsOverridden=yes",
            r"overrideSettings=D:\ci\settings.xml",
            r"overrideLocalRepo=D:\repo",
        ] {
            assert!(line.contains(expected), "诊断行缺 {expected}：{line}");
        }
    }

    #[test]
    fn the_page_level_notice_requires_no_usable_toolchain() {
        let empty = ProjectEnvironment {
            overrides: Overrides::default(),
            jdk: JdkDiscovery::default(),
            maven: MavenDiscovery::default(),
            maven_config: MavenConfiguration::default(),
            maven_jdk: JdkDiscovery::default(),
        };
        assert!(empty.has_no_toolchain());
        assert!(empty.overrides.is_empty());
        assert_eq!(empty.overrides.count(), 0);

        let maven_only = ProjectEnvironment {
            maven: MavenDiscovery {
                overridden: None,
                candidates: vec![DetectedMaven {
                    executable: r"D:\tools\apache-maven-3.9.9\bin\mvn.cmd".to_string(),
                    version: "3.9.9".to_string(),
                    home: None,
                    java_runtime: None,
                    source: ToolSource::MavenHome,
                }],
                rejected: Vec::new(),
            },
            ..empty.clone()
        };
        assert!(!maven_only.has_no_toolchain());

        let unusable = ProjectEnvironment {
            overrides: Overrides {
                java_home: r"D:\nope".to_string(),
                maven_executable: String::new(),
                maven_java_home: String::new(),
                maven_settings: String::new(),
                maven_local_repository: String::new(),
            },
            jdk: JdkDiscovery {
                overridden: Some(OverrideProbe {
                    path: r"D:\nope".to_string(),
                    result: Err("找不到 java.exe".to_string()),
                }),
                candidates: Vec::new(),
                rejected: Vec::new(),
            },
            ..empty.clone()
        };
        assert!(!unusable.has_no_toolchain());
        assert_eq!(unusable.overrides.count(), 1);
    }

    #[test]
    fn source_labels_point_at_the_right_keys() {
        assert_eq!(
            ToolSource::JavaHome.label_key(),
            "lithe.toolchain.source.javaHome"
        );
        assert_eq!(ToolSource::Path.label_key(), "lithe.toolchain.source.path");
        assert_eq!(
            ToolSource::InstalledRoot.label_key(),
            "lithe.toolchain.source.detected"
        );
        assert_eq!(ToolSource::JavaHome.label_arg(), None);
        assert_eq!(ToolSource::Path.label_arg(), None);
        assert_eq!(ToolSource::InstalledRoot.label_arg(), None);
        assert_eq!(
            ToolSource::EnvOverride.label_arg(),
            Some(JAVA_EXECUTABLE_ENV)
        );
        assert_eq!(ToolSource::MavenHome.label_arg(), Some(MAVEN_HOME_ENV));
        assert_eq!(
            ToolMode::Configured.label_key(),
            "lithe.toolchain.mode.configured"
        );
        assert_eq!(
            ToolMode::ProjectJdk.label_key(),
            "lithe.toolchain.mode.projectJdk"
        );
        assert_eq!(
            LocalRepositorySource::SettingsXml.label_key(),
            "lithe.settings.gpui.mavenLocalRepositoryFromSettings"
        );
    }

    #[test]
    fn jdk_major_version_mirrors_the_jdtls_reading() {
        assert_eq!(jdk_major_version("1.8.0_402"), 8);
        assert_eq!(jdk_major_version("1.8.0_221"), 8);
        assert_eq!(jdk_major_version("21.0.2"), 21);
        assert_eq!(jdk_major_version("21.0.2+13-LTS"), 21);
        assert_eq!(jdk_major_version("25"), 25);
        assert_eq!(jdk_major_version("not a version"), 0);
        assert_eq!(jdk_major_version(""), 0);
    }

    #[test]
    fn the_gate_is_the_jdtls_minimum() {
        assert_eq!(MINIMUM_JAVA_MAJOR, 21);
        assert!(meets_language_service_requirement("21.0.8"));
        assert!(meets_language_service_requirement("22"));
        assert!(!meets_language_service_requirement("17.0.16"));
        assert!(!meets_language_service_requirement("1.8.0_221"));
    }

    #[test]
    fn a_below_gate_automatic_choice_is_marked_unusable() {
        let discovery = JdkDiscovery {
            overridden: None,
            candidates: vec![
                detected_jdk(
                    r"C:\jdk8",
                    r"C:\jdk8\bin\java.exe",
                    "1.8.0_221",
                    ToolSource::Path,
                ),
                detected_jdk(
                    r"D:\ProgramData\java\openjdk-17",
                    r"D:\ProgramData\java\openjdk-17\bin\java.exe",
                    "17.0.16",
                    ToolSource::InstalledRoot,
                ),
            ],
            rejected: Vec::new(),
        };
        assert!(discovery.detected().is_none());
        match discovery.effective() {
            EffectiveToolchain::Unusable {
                path,
                reason,
                language_service_rejection,
            } => {
                assert_eq!(path, r"C:\jdk8");
                assert!(reason.contains("1.8.0_221"), "{reason}");
                assert!(reason.contains("21"), "{reason}");
                assert!(reason.contains("语言服务无法启动"), "{reason}");
                assert!(language_service_rejection);
            }
            other => panic!("低于闸门的自动值必须报 Unusable，实际 {other:?}"),
        }
        assert_eq!(discovery.candidates.len(), 2);
        assert!(
            discovery
                .candidates
                .iter()
                .all(|jdk| !jdk.meets_language_service)
        );
    }

    #[test]
    fn the_automatic_choice_skips_candidates_below_the_gate() {
        let discovery = JdkDiscovery {
            overridden: None,
            candidates: vec![
                detected_jdk(
                    r"C:\jdk8",
                    r"C:\jdk8\bin\java.exe",
                    "1.8.0_221",
                    ToolSource::Path,
                ),
                detected_jdk(
                    r"D:\ProgramData\java\openjdk-21",
                    r"D:\ProgramData\java\openjdk-21\bin\java.exe",
                    "21.0.8",
                    ToolSource::JavaHome,
                ),
            ],
            rejected: Vec::new(),
        };
        assert_eq!(
            discovery.effective(),
            EffectiveToolchain::Resolved {
                mode: ToolMode::Automatic,
                path: r"D:\ProgramData\java\openjdk-21".to_string(),
                version: Some("21.0.8".to_string()),
                source: Some(ToolSource::JavaHome),
            }
        );
    }

    #[test]
    fn a_below_gate_override_is_marked_unusable() {
        let discovery = JdkDiscovery {
            overridden: Some(OverrideProbe {
                path: r"D:\ProgramData\java\openjdk-17".to_string(),
                result: Ok(DetectedTool {
                    path: r"D:\ProgramData\java\openjdk-17".to_string(),
                    version: "17.0.16".to_string(),
                    meets_language_service: false,
                }),
            }),
            candidates: vec![detected_jdk(
                r"D:\ProgramData\java\openjdk-21",
                r"D:\ProgramData\java\openjdk-21\bin\java.exe",
                "21.0.8",
                ToolSource::InstalledRoot,
            )],
            rejected: Vec::new(),
        };
        match discovery.effective() {
            EffectiveToolchain::Unusable {
                path,
                reason,
                language_service_rejection,
            } => {
                assert_eq!(path, r"D:\ProgramData\java\openjdk-17");
                assert!(reason.contains("17.0.16"), "{reason}");
                assert!(reason.contains("21"), "{reason}");
                assert!(reason.contains("语言服务无法启动"), "{reason}");
                assert!(language_service_rejection);
            }
            other => panic!("低于闸门的覆盖值必须报 Unusable，实际 {other:?}"),
        }
        assert_ne!(
            discovery.effective().diagnostic_value(),
            r"D:\ProgramData\java\openjdk-21"
        );
    }

    #[test]
    fn an_override_exactly_at_the_gate_is_usable() {
        let discovery = JdkDiscovery {
            overridden: Some(OverrideProbe {
                path: r"D:\ProgramData\java\openjdk-21".to_string(),
                result: Ok(DetectedTool {
                    path: r"D:\ProgramData\java\openjdk-21".to_string(),
                    version: "21.0.8".to_string(),
                    meets_language_service: true,
                }),
            }),
            candidates: Vec::new(),
            rejected: Vec::new(),
        };
        assert_eq!(
            discovery.effective(),
            EffectiveToolchain::Resolved {
                mode: ToolMode::Configured,
                path: r"D:\ProgramData\java\openjdk-21".to_string(),
                version: Some("21.0.8".to_string()),
                source: None,
            }
        );
    }

    #[test]
    fn the_diagnostic_line_reports_the_gate_verdict() {
        let below = ProjectEnvironment {
            overrides: Overrides::default(),
            jdk: JdkDiscovery {
                overridden: None,
                candidates: vec![detected_jdk(
                    r"C:\jdk8",
                    r"C:\jdk8\bin\java.exe",
                    "1.8.0_221",
                    ToolSource::Path,
                )],
                rejected: Vec::new(),
            },
            maven: MavenDiscovery::default(),
            maven_config: MavenConfiguration::default(),
            maven_jdk: JdkDiscovery::default(),
        };
        let line = below.diagnostic_line();
        assert!(line.contains("minimumJava=21"), "{line}");
        assert!(line.contains("languageService=unavailable"), "{line}");
        assert!(line.contains("version=1.8.0_221"), "{line}");
        assert!(line.contains(r"jdk=unusable(C:\jdk8)"), "{line}");

        let ready = ProjectEnvironment {
            jdk: JdkDiscovery {
                overridden: None,
                candidates: vec![detected_jdk(
                    r"D:\ProgramData\java\openjdk-21",
                    r"D:\ProgramData\java\openjdk-21\bin\java.exe",
                    "21.0.8",
                    ToolSource::InstalledRoot,
                )],
                rejected: Vec::new(),
            },
            ..below
        };
        let line = ready.diagnostic_line();
        assert!(line.contains("languageService=ready"), "{line}");
        assert!(line.contains("version=21.0.8"), "{line}");
    }
}
