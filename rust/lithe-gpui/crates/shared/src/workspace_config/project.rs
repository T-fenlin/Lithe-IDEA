use std::fmt;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::core_client::{CoreError, core_json};
use crate::document;
use crate::workspace_config::DIAGNOSTIC_PREFIX;
use crate::workspace_config::paths::WorkspaceConfigPaths;
use crate::workspace_config::sharing::{self, EnsureExcluded, IgnoreFileOutcome, SharingError};

pub const PROJECT_MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectManifest {
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectManifestState {
    pub manifest: ProjectManifest,
    pub previous: Option<Value>,
    pub read_only: bool,
    pub file_existed: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestSaved {
    pub bytes: usize,
    pub exclusion: EnsureExcluded,
    pub ignore: IgnoreFileOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectIdentity {
    Manifest(String),
    PathDerived(String),
}

impl ProjectIdentity {
    pub fn value(&self) -> &str {
        match self {
            Self::Manifest(id) | Self::PathDerived(id) => id,
        }
    }

    pub fn is_manifest_backed(&self) -> bool {
        matches!(self, Self::Manifest(_))
    }
}

#[derive(Debug)]
pub enum WorkspaceConfigError {
    Exclude(SharingError),
    PathIdentity(CoreError),
    Io(io::Error),
    Json(serde_json::Error),
    MissingId,
    ReadOnly {
        path: String,
        declared: u64,
        supported: u32,
    },
}

impl fmt::Display for WorkspaceConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exclude(error) => write!(formatter, "确保本机排除失败：{error}"),
            Self::PathIdentity(error) => write!(formatter, "解析路径身份失败：{error}"),
            Self::Io(error) => write!(formatter, "清单文件读写失败：{error}"),
            Self::Json(error) => write!(formatter, "清单 JSON 生成失败：{error}"),
            Self::MissingId => write!(formatter, "项目清单缺少 id，拒绝写入（会抹掉已有的身份）"),
            Self::ReadOnly {
                path,
                declared,
                supported,
            } => write!(
                formatter,
                "文档版本 {declared} 高于本程序支持的 {supported}，拒绝覆盖：{path}"
            ),
        }
    }
}

impl std::error::Error for WorkspaceConfigError {}

impl From<io::Error> for WorkspaceConfigError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for WorkspaceConfigError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub fn load_project_manifest(root: &Path) -> ProjectManifestState {
    let path = WorkspaceConfigPaths::new(root).project_manifest();
    match document::read_document_text(&path) {
        Ok(Some(text)) => {
            let parsed = document::parse::<ProjectManifest>(&text, PROJECT_MANIFEST_VERSION);
            ProjectManifestState {
                manifest: parsed.value,
                previous: parsed.previous,
                read_only: parsed.read_only,
                file_existed: true,
                diagnostics: parsed.diagnostics,
            }
        }
        Ok(None) => ProjectManifestState {
            manifest: ProjectManifest::default(),
            previous: None,
            read_only: false,
            file_existed: false,
            diagnostics: Vec::new(),
        },
        Err(error) => ProjectManifestState {
            manifest: ProjectManifest::default(),
            previous: None,
            read_only: false,
            file_existed: false,
            diagnostics: vec![format!("read_failed error={error}")],
        },
    }
}

pub fn save_project_manifest(
    root: &Path,
    manifest: &ProjectManifest,
) -> Result<ManifestSaved, WorkspaceConfigError> {
    let Some(id) = declared_id(manifest) else {
        eprintln!(
            "{DIAGNOSTIC_PREFIX} manifest_write_rejected root={} reason=missing_id",
            root.display()
        );
        return Err(WorkspaceConfigError::MissingId);
    };

    let path = WorkspaceConfigPaths::new(root).project_manifest();
    // 与会话保存同理：排除失败只影响工作区是否变脏，不该让项目清单写不进去。
    let exclusion = sharing::degrade_exclusion_failure(
        sharing::ensure_project_dir_excluded(root),
        root,
        "project",
    );
    let previous = document::previous_object(&path);
    let bytes = document::save_document(
        &path,
        &ProjectManifest { id: Some(id) },
        previous.as_ref(),
        PROJECT_MANIFEST_VERSION,
    )?;
    let ignore = sharing::ensure_local_ignore_file(root).map_err(WorkspaceConfigError::Io)?;
    println!(
        "{DIAGNOSTIC_PREFIX} manifest_saved root={} bytes={bytes} exclusion={exclusion:?} ignore={ignore:?}",
        root.display()
    );
    Ok(ManifestSaved {
        bytes,
        exclusion,
        ignore,
    })
}

pub fn resolve_project_id(root: &Path) -> Result<ProjectIdentity, WorkspaceConfigError> {
    let state = load_project_manifest(root);
    for diagnostic in &state.diagnostics {
        eprintln!("{DIAGNOSTIC_PREFIX} manifest_diagnostic {diagnostic}");
    }
    if let Some(id) = declared_id(&state.manifest) {
        return Ok(ProjectIdentity::Manifest(id));
    }

    if state.read_only {
        println!(
            "{DIAGNOSTIC_PREFIX} project_id_path_derived root={} reason=document_version_newer",
            root.display()
        );
        return path_identity(root)
            .map(ProjectIdentity::PathDerived)
            .map_err(WorkspaceConfigError::PathIdentity);
    }

    let id = uuid::Uuid::new_v4().to_string();
    save_project_manifest(
        root,
        &ProjectManifest {
            id: Some(id.clone()),
        },
    )?;
    println!(
        "{DIAGNOSTIC_PREFIX} project_id_created root={} id={id}",
        root.display()
    );
    Ok(ProjectIdentity::Manifest(id))
}

pub fn path_identity(root: &Path) -> Result<String, CoreError> {
    const COMMAND: &str = "lsp.jdtWorkspaceKey";
    let data = core_json(COMMAND, json!({ "workspaceRoot": root.to_string_lossy() }))?;
    data.as_ref()
        .and_then(|value| value.get("workspaceKey"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(CoreError::MissingData {
            command: COMMAND.to_string(),
        })
}

fn declared_id(manifest: &ProjectManifest) -> Option<String> {
    manifest
        .id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_config::WorkspaceConfigPaths;
    use crate::workspace_config::test_repo::TempRepo;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lithe-project-manifest-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        dir
    }

    #[test]
    fn loading_a_missing_manifest_does_not_create_it() {
        let dir = temp_dir("missing");
        let path = WorkspaceConfigPaths::new(&dir).project_manifest();

        let state = load_project_manifest(&dir);
        assert_eq!(state.manifest, ProjectManifest::default());
        assert!(!state.file_existed);
        assert!(state.diagnostics.is_empty());
        assert!(!path.exists(), "读取不得创建文件");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_a_manifest_ensures_the_config_directory_is_not_shared() {
        let repo = TempRepo::new("save");
        let manifest = ProjectManifest {
            id: Some("3f1d2c6a-0000-4000-8000-000000000000".to_string()),
        };

        let saved = save_project_manifest(repo.root(), &manifest).expect("写清单失败");
        assert_eq!(saved.exclusion, EnsureExcluded::Ensured);
        assert!(saved.bytes > 0);

        assert!(
            !repo.status().contains(".lithe"),
            "写清单之后 .lithe 不该出现在 git status：{}",
            repo.status()
        );
        assert_eq!(
            repo.exclude_lines()
                .iter()
                .filter(|line| *line == ".lithe/")
                .count(),
            1,
            "排除文件里应当恰好一行 .lithe/"
        );

        let text = repo.read(".lithe/project.json");
        assert!(text.contains("\"version\": 1"), "{text}");
        assert!(
            text.contains("\"id\": \"3f1d2c6a-0000-4000-8000-000000000000\""),
            "{text}"
        );

        let state = load_project_manifest(repo.root());
        assert_eq!(state.manifest, manifest);
        assert!(state.file_existed);
        assert!(state.diagnostics.is_empty(), "{:?}", state.diagnostics);
    }

    #[test]
    fn saving_preserves_fields_owned_by_other_producers() {
        let repo = TempRepo::new("preserve");
        repo.write(
            ".lithe/project.json",
            r#"{"version":1,"defaultRunConfiguration":"java-main","future":{"deep":true}}"#,
        );

        let state = load_project_manifest(repo.root());
        assert_eq!(state.manifest.id, None, "不认识的字段不参与解析");
        assert!(state.diagnostics.is_empty(), "{:?}", state.diagnostics);

        save_project_manifest(
            repo.root(),
            &ProjectManifest {
                id: Some("abc".to_string()),
            },
        )
        .expect("写清单失败");

        let text = repo.read(".lithe/project.json");
        let value: Value = serde_json::from_str(&text).expect("清单应是 JSON");
        assert_eq!(
            value.get("defaultRunConfiguration"),
            Some(&Value::from("java-main")),
            "别的生产者写的字段必须原样保留：{text}"
        );
        assert_eq!(
            value.get("future"),
            Some(&json!({ "deep": true })),
            "未知的嵌套对象必须整体保留：{text}"
        );
        assert_eq!(value.get("id"), Some(&Value::from("abc")));
        assert_eq!(
            value.get(crate::document::DOCUMENT_VERSION_KEY),
            Some(&Value::from(1))
        );
    }

    #[test]
    fn writing_without_an_id_is_rejected_instead_of_wiping_the_existing_one() {
        let repo = TempRepo::new("missing-id");
        repo.write(".lithe/project.json", r#"{"version":1,"id":"kept"}"#);

        let error = save_project_manifest(repo.root(), &ProjectManifest::default())
            .expect_err("没有 id 必须被拒绝");
        assert!(
            matches!(error, WorkspaceConfigError::MissingId),
            "{error:?}"
        );

        let error = save_project_manifest(
            repo.root(),
            &ProjectManifest {
                id: Some("   ".to_string()),
            },
        )
        .expect_err("空白 id 必须被拒绝");
        assert!(
            matches!(error, WorkspaceConfigError::MissingId),
            "{error:?}"
        );

        let text = repo.read(".lithe/project.json");
        assert!(text.contains("\"id\":\"kept\""), "{text}");
        assert!(
            !repo.exclude_lines().iter().any(|line| line == ".lithe/"),
            "被拒绝的写入不该动排除文件"
        );
    }

    #[test]
    fn newer_manifest_version_falls_back_to_the_path_identity() {
        let dir = temp_dir("newer");
        let path = WorkspaceConfigPaths::new(&dir).project_manifest();
        let original = r#"{"version":99,"id":"theirs"}"#;
        std::fs::create_dir_all(path.parent().expect("有父目录")).expect("建配置目录");
        std::fs::write(&path, original).expect("写初始清单失败");

        let state = load_project_manifest(&dir);
        assert!(state.read_only);
        assert_eq!(state.manifest.id.as_deref(), Some("theirs"));

        let identity = resolve_project_id(&dir).expect("解析身份失败");
        assert_eq!(identity, ProjectIdentity::Manifest("theirs".to_string()));
        assert_eq!(
            std::fs::read_to_string(&path).expect("读失败"),
            original,
            "只读的清单不得被改写"
        );

        std::fs::write(&path, r#"{"version":99}"#).expect("写初始清单失败");
        let identity = resolve_project_id(&dir).expect("解析路径身份失败");
        let ProjectIdentity::PathDerived(key) = &identity else {
            panic!("只读且没有 id 时必须回落到路径身份，实际 {identity:?}");
        };
        assert_eq!(key.len(), 64, "Core 的路径身份是 64 位 hex：{key}");
        assert!(
            key.bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "路径身份必须是小写 hex：{key}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).expect("读失败"),
            r#"{"version":99}"#,
            "只读的清单不得被改写"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_new_workspace_gets_a_generated_uuid_identity() {
        let repo = TempRepo::new("identity");

        let identity = resolve_project_id(repo.root()).expect("生成身份失败");
        let ProjectIdentity::Manifest(id) = &identity else {
            panic!("新工作区应当生成清单身份，实际 {identity:?}");
        };
        assert!(identity.is_manifest_backed());
        assert_eq!(id.len(), 36, "{id}");
        assert_eq!(&id[14..15], "4", "必须是 UUID v4：{id}");
        assert!(
            matches!(&id[19..20], "8" | "9" | "a" | "b"),
            "variant 位不对：{id}"
        );

        let again = resolve_project_id(repo.root()).expect("再次解析身份失败");
        assert_eq!(again, identity, "同一个工作区的身份必须稳定");

        let manifest = load_project_manifest(repo.root()).manifest;
        assert_eq!(manifest.id.as_deref(), Some(id.as_str()));
    }
}
