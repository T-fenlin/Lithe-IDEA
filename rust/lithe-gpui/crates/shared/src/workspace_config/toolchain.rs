use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::document;
use crate::workspace_config::DIAGNOSTIC_PREFIX;
use crate::workspace_config::paths::WorkspaceConfigPaths;
use crate::workspace_config::project::WorkspaceConfigError;
use crate::workspace_config::sharing::{
    self, EnsureExcluded, IgnoreFileOutcome, ensure_local_ignore_file,
};

pub const RUN_LOCAL_DOCUMENT_VERSION: u32 = 2;

pub const MAVEN_LOCAL_DOCUMENT_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolchainPaths {
    pub java_home_path: String,
    pub maven_executable_path: String,
    pub maven_java_home_path: String,
    pub maven_settings_path: String,
    pub maven_local_repository_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverrideOrigin {
    Project,
    Global,
    Unset,
}

impl OverrideOrigin {
    pub fn id(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Global => "global",
            Self::Unset => "unset",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverrideOrigins {
    pub java_home_path: OverrideOrigin,
    pub maven_executable_path: OverrideOrigin,
    pub maven_java_home_path: OverrideOrigin,
    pub maven_settings_path: OverrideOrigin,
    pub maven_local_repository_path: OverrideOrigin,
}

impl ToolchainPaths {
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn count(&self) -> usize {
        [
            &self.java_home_path,
            &self.maven_executable_path,
            &self.maven_java_home_path,
            &self.maven_settings_path,
            &self.maven_local_repository_path,
        ]
        .iter()
        .filter(|value| !value.trim().is_empty())
        .count()
    }

    pub fn normalized(&self) -> Self {
        let clean = |value: &str| value.trim().to_string();
        Self {
            java_home_path: clean(&self.java_home_path),
            maven_executable_path: clean(&self.maven_executable_path),
            maven_java_home_path: clean(&self.maven_java_home_path),
            maven_settings_path: clean(&self.maven_settings_path),
            maven_local_repository_path: clean(&self.maven_local_repository_path),
        }
    }

    pub fn resolve(local: &Self, global: &Self) -> (Self, OverrideOrigins) {
        let mut resolved = Self::default();
        let mut origins = OverrideOrigins {
            java_home_path: OverrideOrigin::Unset,
            maven_executable_path: OverrideOrigin::Unset,
            maven_java_home_path: OverrideOrigin::Unset,
            maven_settings_path: OverrideOrigin::Unset,
            maven_local_repository_path: OverrideOrigin::Unset,
        };

        let pick = |local_value: &str,
                    global_value: &str,
                    target: &mut String,
                    origin: &mut OverrideOrigin| {
            let local_value = local_value.trim();
            if !local_value.is_empty() {
                *target = local_value.to_string();
                *origin = OverrideOrigin::Project;
                return;
            }
            let global_value = global_value.trim();
            if !global_value.is_empty() {
                *target = global_value.to_string();
                *origin = OverrideOrigin::Global;
            }
        };

        pick(
            &local.java_home_path,
            &global.java_home_path,
            &mut resolved.java_home_path,
            &mut origins.java_home_path,
        );
        pick(
            &local.maven_executable_path,
            &global.maven_executable_path,
            &mut resolved.maven_executable_path,
            &mut origins.maven_executable_path,
        );
        pick(
            &local.maven_java_home_path,
            &global.maven_java_home_path,
            &mut resolved.maven_java_home_path,
            &mut origins.maven_java_home_path,
        );
        pick(
            &local.maven_settings_path,
            &global.maven_settings_path,
            &mut resolved.maven_settings_path,
            &mut origins.maven_settings_path,
        );
        pick(
            &local.maven_local_repository_path,
            &global.maven_local_repository_path,
            &mut resolved.maven_local_repository_path,
            &mut origins.maven_local_repository_path,
        );

        (resolved, origins)
    }

    pub fn diagnostic_suffix(origins: &OverrideOrigins) -> String {
        format!(
            "project={} global={} unset={}",
            [
                origins.java_home_path,
                origins.maven_executable_path,
                origins.maven_java_home_path,
                origins.maven_settings_path,
                origins.maven_local_repository_path,
            ]
            .iter()
            .filter(|origin| **origin == OverrideOrigin::Project)
            .count(),
            [
                origins.java_home_path,
                origins.maven_executable_path,
                origins.maven_java_home_path,
                origins.maven_settings_path,
                origins.maven_local_repository_path,
            ]
            .iter()
            .filter(|origin| **origin == OverrideOrigin::Global)
            .count(),
            [
                origins.java_home_path,
                origins.maven_executable_path,
                origins.maven_java_home_path,
                origins.maven_settings_path,
                origins.maven_local_repository_path,
            ]
            .iter()
            .filter(|origin| **origin == OverrideOrigin::Unset)
            .count(),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalToolchainState {
    pub paths: ToolchainPaths,
    pub file_existed: bool,
    pub diagnostics: Vec<String>,
}

pub fn load(root: &Path) -> LocalToolchainState {
    let paths = WorkspaceConfigPaths::new(root);
    let mut state = LocalToolchainState::default();

    let run_local = paths.run_local();
    let mut run_path = run_local.clone();
    if document::read_document_text(&run_local)
        .ok()
        .flatten()
        .is_none()
    {
        let legacy = paths.run_local_legacy();
        if document::read_document_text(&legacy)
            .ok()
            .flatten()
            .is_some()
        {
            run_path = legacy;
        }
    }
    if let Some(text) = document::read_document_text(&run_path).ok().flatten() {
        state.file_existed = true;
        let parsed = document::parse::<RunLocalDocument>(&text, RUN_LOCAL_DOCUMENT_VERSION);
        for diagnostic in &parsed.diagnostics {
            state.diagnostics.push(format!("run_local {diagnostic}"));
        }
        state.paths.java_home_path = parsed.value.toolchain.java.home_path;
        state.paths.maven_executable_path = parsed.value.toolchain.maven.executable_path;
        state.paths.maven_java_home_path = parsed.value.toolchain.maven.java_home_path;
    }

    let maven_local = paths.maven_local();
    if let Some(text) = document::read_document_text(&maven_local).ok().flatten() {
        state.file_existed = true;
        let parsed = document::parse::<MavenLocalDocument>(&text, MAVEN_LOCAL_DOCUMENT_VERSION);
        for diagnostic in &parsed.diagnostics {
            state.diagnostics.push(format!("maven_local {diagnostic}"));
        }
        state.paths.maven_settings_path = parsed.value.settings_path;
        state.paths.maven_local_repository_path = parsed.value.local_repository_path;
    }

    state.paths = state.paths.normalized();
    state
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainSaved {
    pub run_local_bytes: usize,
    pub maven_local_bytes: usize,
    pub exclusion: EnsureExcluded,
    pub ignore: IgnoreFileOutcome,
}

pub fn save(root: &Path, paths: &ToolchainPaths) -> Result<ToolchainSaved, WorkspaceConfigError> {
    let paths = paths.normalized();
    let config = WorkspaceConfigPaths::new(root);

    let exclusion =
        sharing::ensure_project_dir_excluded(root).map_err(WorkspaceConfigError::Exclude)?;

    let run_local_bytes = save_run_local(&config, &paths)?;
    let maven_local_bytes = save_maven_local(&config, &paths)?;
    let ignore = ensure_local_ignore_file(root).map_err(WorkspaceConfigError::Io)?;

    println!(
        "{DIAGNOSTIC_PREFIX} toolchain_saved root={} set={} exclusion={exclusion:?} ignore={ignore:?}",
        root.display(),
        paths.count()
    );
    Ok(ToolchainSaved {
        run_local_bytes,
        maven_local_bytes,
        exclusion,
        ignore,
    })
}

fn save_run_local(
    config: &WorkspaceConfigPaths,
    paths: &ToolchainPaths,
) -> Result<usize, WorkspaceConfigError> {
    let path = config.run_local();
    ensure_writable(&path, RUN_LOCAL_DOCUMENT_VERSION)?;
    let previous = document::previous_object(&path);
    let typed = RunLocalDocument {
        toolchain: ToolchainObject {
            java: ToolchainJava {
                home_path: paths.java_home_path.clone(),
            },
            maven: ToolchainMaven {
                executable_path: paths.maven_executable_path.clone(),
                java_home_path: paths.maven_java_home_path.clone(),
            },
        },
    };
    let mut merged =
        document::merge_document(previous.as_ref(), &typed, RUN_LOCAL_DOCUMENT_VERSION)
            .map_err(WorkspaceConfigError::Json)?;
    if let Value::Object(object) = &mut merged {
        if !object.contains_key("configurations") {
            object.insert("configurations".to_string(), Value::Array(Vec::new()));
        }
    }
    document::save_json(&path, &merged).map_err(WorkspaceConfigError::Io)
}

fn save_maven_local(
    config: &WorkspaceConfigPaths,
    paths: &ToolchainPaths,
) -> Result<usize, WorkspaceConfigError> {
    let path = config.maven_local();
    ensure_writable(&path, MAVEN_LOCAL_DOCUMENT_VERSION)?;
    let previous = document::previous_object(&path);
    let typed = MavenLocalDocument {
        settings_path: paths.maven_settings_path.clone(),
        local_repository_path: paths.maven_local_repository_path.clone(),
    };
    document::save_document(
        &path,
        &typed,
        previous.as_ref(),
        MAVEN_LOCAL_DOCUMENT_VERSION,
    )
    .map_err(WorkspaceConfigError::Io)
}

fn ensure_writable(path: &Path, supported: u32) -> Result<(), WorkspaceConfigError> {
    let Some(previous) = document::previous_object(path) else {
        return Ok(());
    };
    let Some(object) = previous.as_object() else {
        return Ok(());
    };
    let Some(declared) = document::declared_version(object) else {
        return Ok(());
    };
    if declared <= u64::from(supported) {
        return Ok(());
    }
    eprintln!(
        "{DIAGNOSTIC_PREFIX} write_skipped path={} reason=document_version_newer declared={declared} supported={supported}",
        path.display()
    );
    Err(WorkspaceConfigError::ReadOnly {
        path: path.display().to_string(),
        declared,
        supported,
    })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RunLocalDocument {
    #[serde(default)]
    toolchain: ToolchainObject,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ToolchainObject {
    #[serde(default)]
    java: ToolchainJava,
    #[serde(default)]
    maven: ToolchainMaven,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ToolchainJava {
    #[serde(default)]
    home_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ToolchainMaven {
    #[serde(default)]
    executable_path: String,
    #[serde(default)]
    java_home_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct MavenLocalDocument {
    #[serde(default)]
    settings_path: String,
    #[serde(default)]
    local_repository_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_config::test_repo::TempRepo;

    fn full() -> ToolchainPaths {
        ToolchainPaths {
            java_home_path: "D:\\jdk-21".to_string(),
            maven_executable_path: "D:\\maven\\bin\\mvn.cmd".to_string(),
            maven_java_home_path: "D:\\jdk-17".to_string(),
            maven_settings_path: "D:\\m2\\settings.xml".to_string(),
            maven_local_repository_path: "D:\\m2\\repository".to_string(),
        }
    }

    #[test]
    fn project_layer_wins_over_the_global_default() {
        let local = ToolchainPaths {
            java_home_path: "D:\\project-jdk".to_string(),
            ..ToolchainPaths::default()
        };
        let global = ToolchainPaths {
            java_home_path: "D:\\global-jdk".to_string(),
            maven_settings_path: "D:\\global\\settings.xml".to_string(),
            ..ToolchainPaths::default()
        };

        let (resolved, origins) = ToolchainPaths::resolve(&local, &global);
        assert_eq!(resolved.java_home_path, "D:\\project-jdk");
        assert_eq!(origins.java_home_path, OverrideOrigin::Project);
        assert_eq!(resolved.maven_settings_path, "D:\\global\\settings.xml");
        assert_eq!(origins.maven_settings_path, OverrideOrigin::Global);
        assert_eq!(resolved.maven_executable_path, "");
        assert_eq!(origins.maven_executable_path, OverrideOrigin::Unset);
    }

    #[test]
    fn blank_values_do_not_override() {
        let local = ToolchainPaths {
            java_home_path: "   ".to_string(),
            ..ToolchainPaths::default()
        };
        let global = ToolchainPaths {
            java_home_path: "D:\\global-jdk".to_string(),
            ..ToolchainPaths::default()
        };
        let (resolved, origins) = ToolchainPaths::resolve(&local, &global);
        assert_eq!(resolved.java_home_path, "D:\\global-jdk");
        assert_eq!(origins.java_home_path, OverrideOrigin::Global);
    }

    #[test]
    fn save_then_load_round_trips_all_five_values() {
        let repo = TempRepo::new("toolchain-rt");
        let saved = save(repo.root(), &full()).expect("写入失败");
        assert!(saved.run_local_bytes > 0);
        assert!(saved.maven_local_bytes > 0);

        let loaded = load(repo.root());
        assert!(loaded.file_existed);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(loaded.paths, full());

        let run_text = repo.read(".lithe/run.local.json");
        let run: Value = serde_json::from_str(&run_text).expect("run.local.json 必须是 JSON");
        assert_eq!(run.get("version"), Some(&Value::from(2)));
        assert_eq!(
            run.pointer("/toolchain/java/homePath"),
            Some(&Value::from("D:\\jdk-21"))
        );
        assert_eq!(
            run.pointer("/toolchain/maven/executablePath"),
            Some(&Value::from("D:\\maven\\bin\\mvn.cmd"))
        );
        assert_eq!(
            run.pointer("/toolchain/maven/javaHomePath"),
            Some(&Value::from("D:\\jdk-17"))
        );
        assert_eq!(run.get("configurations"), Some(&serde_json::json!([])));

        let maven_text = repo.read(".lithe/maven.local.json");
        let maven: Value = serde_json::from_str(&maven_text).expect("maven.local.json 必须是 JSON");
        assert_eq!(maven.get("version"), Some(&Value::from(1)));
        assert_eq!(
            maven.get("settingsPath"),
            Some(&Value::from("D:\\m2\\settings.xml"))
        );
        assert_eq!(
            maven.get("localRepositoryPath"),
            Some(&Value::from("D:\\m2\\repository"))
        );
    }

    #[test]
    fn clearing_a_value_actually_clears_it() {
        let repo = TempRepo::new("toolchain-clear");
        save(repo.root(), &full()).expect("写入失败");
        assert_eq!(load(repo.root()).paths.java_home_path, "D:\\jdk-21");

        let cleared = ToolchainPaths {
            java_home_path: String::new(),
            ..full()
        };
        save(repo.root(), &cleared).expect("写入失败");
        let loaded = load(repo.root());
        assert_eq!(loaded.paths.java_home_path, "", "空串必须真的清掉旧值");
        assert_eq!(
            loaded.paths.maven_executable_path,
            "D:\\maven\\bin\\mvn.cmd"
        );
    }

    #[test]
    fn existing_configurations_are_preserved() {
        let repo = TempRepo::new("toolchain-configurations");
        repo.write(
            ".lithe/run.local.json",
            r#"{
  "version": 2,
  "configurations": [
    { "id": "current-file", "name": "My File", "provider": "java.current-file" }
  ],
  "thirdParty": { "keep": true }
}"#,
        );

        save(repo.root(), &full()).expect("写入失败");

        let text = std::fs::read_to_string(repo.root().join(".lithe/run.local.json"))
            .expect("读 run.local.json");
        let value: Value = serde_json::from_str(&text).expect("必须是 JSON");
        let configurations = value
            .get("configurations")
            .and_then(Value::as_array)
            .expect("configurations 必须还在");
        assert_eq!(configurations.len(), 1);
        assert_eq!(
            configurations[0].get("id"),
            Some(&Value::from("current-file"))
        );
        assert_eq!(value.pointer("/thirdParty/keep"), Some(&Value::from(true)));
    }

    #[test]
    fn legacy_run_local_is_read_until_the_new_file_exists() {
        let repo = TempRepo::new("toolchain-legacy");
        repo.write(
            ".lithe/run/local.json",
            r#"{"version": 2, "configurations": [], "toolchain": { "java": { "homePath": "D:\\legacy-jdk" } }}"#,
        );

        let loaded = load(repo.root());
        assert_eq!(loaded.paths.java_home_path, "D:\\legacy-jdk");
        assert!(loaded.file_existed);

        save(
            repo.root(),
            &ToolchainPaths {
                java_home_path: "D:\\new-jdk".to_string(),
                ..ToolchainPaths::default()
            },
        )
        .expect("写入失败");
        assert_eq!(load(repo.root()).paths.java_home_path, "D:\\new-jdk");
        assert!(
            repo.root().join(".lithe/run.local.json").exists(),
            "写必须落到新路径"
        );

        save(repo.root(), &ToolchainPaths::default()).expect("写入失败");
        assert_eq!(load(repo.root()).paths.java_home_path, "");
    }

    #[test]
    fn saving_keeps_the_config_directory_out_of_git_status() {
        let repo = TempRepo::new("toolchain-exclude");
        save(repo.root(), &full()).expect("写入失败");

        let exclude = repo.exclude_lines();
        assert_eq!(
            exclude
                .iter()
                .filter(|line| line.as_str() == ".lithe/")
                .count(),
            1,
            "本机排除里应当恰好一行 .lithe/：{exclude:?}"
        );
        assert!(
            repo.status().trim().is_empty(),
            "git status 不该看到 .lithe：{}",
            repo.status()
        );
    }

    #[test]
    fn saving_also_writes_the_local_ignore_file() {
        let repo = TempRepo::new("toolchain-ignore");
        save(repo.root(), &full()).expect("写入失败");
        let text = repo.read(".lithe/.gitignore");
        assert_eq!(
            text,
            format!("{}\n", sharing::LOCAL_IGNORE_RULES.join("\n")),
            "首次写入必须是那三条规则、一行一条"
        );
    }

    #[test]
    fn newer_document_version_is_not_overwritten() {
        let repo = TempRepo::new("toolchain-newer");
        let future = r#"{"version": 99, "configurations": [], "toolchain": { "java": { "homePath": "D:\\future-jdk" } }, "futureKey": 1}"#;
        repo.write(".lithe/run.local.json", future);

        let loaded = load(repo.root());
        assert_eq!(loaded.paths.java_home_path, "D:\\future-jdk");
        assert!(
            loaded
                .diagnostics
                .iter()
                .any(|line| line.contains("document_version_newer")),
            "{:?}",
            loaded.diagnostics
        );

        let error = save(repo.root(), &full()).expect_err("更高版本的文档必须拒绝写入");
        assert!(
            matches!(error, WorkspaceConfigError::ReadOnly { declared, supported, .. } if declared == 99 && supported == RUN_LOCAL_DOCUMENT_VERSION),
            "应当是 ReadOnly：{error}"
        );
        let after = repo.read(".lithe/run.local.json");
        assert_eq!(after, future, "被拒绝的写入不得改动文件一个字");
    }
}
