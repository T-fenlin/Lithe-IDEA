//! `.lithe/project.json`：工作区身份清单的读写。
//!
//! ## 身份为什么需要落盘
//!
//! 在此之前，"这个项目是谁"在仓库里有六种算法（本地历史用 `stableIdentifier(工作区路径)`、
//! Maven 用 `sha256(工作区路径 + reactor 路径)`、索引用 JDT 指纹键…），全是**路径派生**的：
//! 目录改名、仓库搬家、换台机器 clone 之后，同一个项目的本地历史、断点、会话就对不上了。
//!
//! 所以真源是清单里的 `id`（UUID v4，首次生成时写入、**随项目提交进 Git**），
//! 而 Core 已有的路径身份（`normalized_workspace_identity` + SHA-256，
//! `rust/lithe-core/src/lsp/languages/jdt.rs`）降级为**回落**：清单还没有 id、
//! 或文件版本比本程序新（能读不能写）时用它。
//!
//! ## 只建模 gpui 真正读写的字段
//!
//! `project.json` 是跨端共享文件：Core 会读它的 `defaultRunConfiguration`，现役 Windows/
//! macOS 也会写这个字段。本模块**不建模**它 —— 靠 [`crate::document`] 的"未知键原样保留"，
//! 那些字段会被逐字带过，既不会丢也不会被改。为没有消费方的字段建类型只会造出假契约
//! （写进去、没人读、还得出测试）。
//!
//! ## `id` 为什么不能 `skip_serializing_if`
//!
//! [`crate::document::known_keys`] 用 `T::default()` 的序列化结果当已知键集。`id` 一旦
//! 在默认值里被跳过，它就不在键集里，于是"另一个键坏掉"时它会被当成未知键读不回来 ——
//! 那正是派生机制要消灭的静默失效。代价是默认值会序列化成 `{"id": null}`，可以接受。

use std::fmt;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::core_client::{CoreError, core_json};
use crate::document;
use crate::workspace_config::DIAGNOSTIC_PREFIX;
use crate::workspace_config::paths::WorkspaceConfigPaths;
use crate::workspace_config::sharing::{self, EnsureExcluded};

/// `project.json` 当前的文档版本。
pub const PROJECT_MANIFEST_VERSION: u32 = 1;

/// 工作区身份清单。
///
/// 字段与写回规则见模块文档。**不要**给字段加 `skip_serializing_if`；容器级的
/// `#[serde(default)]` 是 [`crate::document`] 的硬约束（缺键要能逐字段回落）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectManifest {
    /// 稳定项目身份（UUID v4）。空串与 `None` 都当作"还没生成"。
    #[serde(default)]
    pub id: Option<String>,
}

/// 一次读取的结论。
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectManifestState {
    /// 解析出来的清单。任何失败路径都给出可用的值（最差是全默认）。
    pub manifest: ProjectManifest,
    /// 文件里的原始对象；写回时用它保留未知键。
    pub previous: Option<Value>,
    /// 文件声明的版本高于本程序支持：能读，**不要覆盖**。
    pub read_only: bool,
    /// 文件是否真的存在且被读到了。
    pub file_existed: bool,
    /// 逐键诊断，调用方负责打印。
    pub diagnostics: Vec<String>,
}

/// 一次落盘的结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestSaved {
    /// 写入的字节数。
    pub bytes: usize,
    /// 写之前"确保不共享"的结论（非 Git 项目会是 [`EnsureExcluded::NotARepository`]）。
    pub exclusion: EnsureExcluded,
}

/// 工作区身份。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectIdentity {
    /// 清单里声明的稳定 id（真源）。
    Manifest(String),
    /// 回落的路径身份（Core 的 `lsp.jdtWorkspaceKey`，不含构建指纹）。
    PathDerived(String),
}

impl ProjectIdentity {
    /// 身份的字符串值。
    pub fn value(&self) -> &str {
        match self {
            Self::Manifest(id) | Self::PathDerived(id) => id,
        }
    }

    /// 这个身份是从清单里来的（而不是路径推导的）。
    pub fn is_manifest_backed(&self) -> bool {
        matches!(self, Self::Manifest(_))
    }
}

/// 工作区配置文档的失败。
#[derive(Debug)]
pub enum WorkspaceConfigError {
    /// 写 `.lithe/` 之前"确保本机排除"失败（不是"非 Git 仓库"那一类，那种已经被静默跳过）。
    Exclude(CoreError),
    /// 读写清单文件失败。
    Io(io::Error),
    /// 生成文档 JSON 失败。
    Json(serde_json::Error),
    /// 要写的清单没有 `id`。
    ///
    /// 清单当前**只拥有** `id` 这一个字段，所以没有 id 就没有可写内容；更要紧的是
    /// typed 的 `id: null` 会在合并时盖掉文件里已有的 id —— 那是静默的数据丢失。
    MissingId,
}

impl fmt::Display for WorkspaceConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exclude(error) => write!(formatter, "确保本机排除失败：{error}"),
            Self::Io(error) => write!(formatter, "清单文件读写失败：{error}"),
            Self::Json(error) => write!(formatter, "清单 JSON 生成失败：{error}"),
            Self::MissingId => write!(formatter, "项目清单缺少 id，拒绝写入（会抹掉已有的身份）"),
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

/// 读清单（带路径推导），**不创建文件、不 panic**。
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

/// 写清单：**先确保 `.lithe/` 不被共享，再原子落盘**。
///
/// 这是"要写 `.lithe` 之前先确保排除"的落地形式 —— 顺序由本函数保证，调用方不需要记得
/// 先调守卫。未知键靠 [`crate::document::previous_object`] 从磁盘现读，所以本函数
/// 不需要调用方传上一版文档。
pub fn save_project_manifest(
    root: &Path,
    manifest: &ProjectManifest,
) -> Result<ManifestSaved, WorkspaceConfigError> {
    // 没有 id 就不写：见 [`WorkspaceConfigError::MissingId`]。这一步必须在守卫**之前**，
    // 免得为一个根本不会发生的写入去动排除文件。
    let Some(id) = declared_id(manifest) else {
        eprintln!(
            "{DIAGNOSTIC_PREFIX} manifest_write_rejected root={} reason=missing_id",
            root.display()
        );
        return Err(WorkspaceConfigError::MissingId);
    };

    let path = WorkspaceConfigPaths::new(root).project_manifest();
    // 写 `.lithe/` 之前的唯一入口：先让"默认不共享"落在磁盘上，否则第一次运行就会往
    // 用户的 `git status` 里撒文件。
    let exclusion =
        sharing::ensure_project_dir_excluded(root).map_err(WorkspaceConfigError::Exclude)?;
    let previous = document::previous_object(&path);
    let bytes = document::save_document(
        &path,
        &ProjectManifest { id: Some(id) },
        previous.as_ref(),
        PROJECT_MANIFEST_VERSION,
    )?;
    println!(
        "{DIAGNOSTIC_PREFIX} manifest_saved root={} bytes={bytes} exclusion={exclusion:?}",
        root.display()
    );
    Ok(ManifestSaved { bytes, exclusion })
}

/// 解析这个工作区的项目身份：清单里的 `id` 优先，否则生成一个写回，写不了就用路径身份。
///
/// `id` 是**首次生成**的：清单里已经有 id 时本函数不写文件（不做无意义的落盘）。
pub fn resolve_project_id(root: &Path) -> Result<ProjectIdentity, WorkspaceConfigError> {
    let state = load_project_manifest(root);
    for diagnostic in &state.diagnostics {
        eprintln!("{DIAGNOSTIC_PREFIX} manifest_diagnostic {diagnostic}");
    }
    if let Some(id) = declared_id(&state.manifest) {
        return Ok(ProjectIdentity::Manifest(id));
    }

    // 文件版本比本程序新：能读不能写，退到路径身份，**不覆盖**用户的文件。
    if state.read_only {
        println!(
            "{DIAGNOSTIC_PREFIX} project_id_path_derived root={} reason=document_version_newer",
            root.display()
        );
        return path_identity(root)
            .map(ProjectIdentity::PathDerived)
            .map_err(WorkspaceConfigError::Exclude);
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

/// Core 的**路径身份**：`normalized_workspace_identity` + SHA-256，不含构建指纹。
///
/// 走既有 Core 命令而不是在 gpui 里再实现一遍归一化：`lsp.jdtWorkspaceKey` 的
/// `workspaceFingerprint` 是可选的（`rust/lithe-core/src/lsp/languages/jdt.rs` 的
/// `JdtWorkspaceKeyRequest`），省略它拿到的就是**纯路径**键 —— 与 `lsp.startServer`
/// 内部同一个归一化。**不要**给它传指纹：那会让身份随 `pom.xml` 变化而失效。
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

/// 清单里声明的 id；`None` 与空串都算"还没生成"。
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

    /// 独立的临时目录（**不含 Git 仓库**），用完删掉。
    ///
    /// 只用在不调用排除守卫的用例上（读清单、只读分支）。任何调用 `save_project_manifest`
    /// 的用例都必须用 [`TempRepo`]：`TEMP` 落在仓库内时，裸临时目录会让守卫把
    /// `.lithe/` 写进**外层真实仓库**的 `.git/info/exclude`（见 `test_repo` 的模块文档）。
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lithe-project-manifest-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        dir
    }

    /// 读不存在的清单：给默认值、**不创建文件**。
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

    /// 落盘：**先让"默认不共享"落到磁盘上**，再写出带 `version` 与 `id` 的清单。
    ///
    /// 这是守卫的真实调用路径：调用方不需要记得先调它。
    #[test]
    fn saving_a_manifest_ensures_the_config_directory_is_not_shared() {
        let repo = TempRepo::new("save");
        let manifest = ProjectManifest {
            id: Some("3f1d2c6a-0000-4000-8000-000000000000".to_string()),
        };

        let saved = save_project_manifest(repo.root(), &manifest).expect("写清单失败");
        assert_eq!(saved.exclusion, EnsureExcluded::Ensured);
        assert!(saved.bytes > 0);

        // 守卫生效：`.lithe/` 不出现在 git status，排除文件里恰好一行。
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

    /// 写回保留别的工具写的字段（例如 Core / 现役产品写的 `defaultRunConfiguration`）。
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

    /// 没有 id 时**拒绝写入**：否则 typed 的 `id: null` 会盖掉文件里已有的身份。
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

        // 空串与空白同样算"没有 id"。
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

        // 文件里的 id 一个字符都没动，而且守卫也没被调用（不该动排除文件）。
        let text = repo.read(".lithe/project.json");
        assert!(text.contains("\"id\":\"kept\""), "{text}");
        assert!(
            !repo.exclude_lines().iter().any(|line| line == ".lithe/"),
            "被拒绝的写入不该动排除文件"
        );
    }

    /// 更高的文档版本 → 只读：`resolve_project_id` 回落到路径身份，**并改写文件**。
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

        // 已经有 id 时直接用清单里的（不写文件）。
        let identity = resolve_project_id(&dir).expect("解析身份失败");
        assert_eq!(identity, ProjectIdentity::Manifest("theirs".to_string()));
        assert_eq!(
            std::fs::read_to_string(&path).expect("读失败"),
            original,
            "只读的清单不得被改写"
        );

        // 没有 id 且文件只读：回落到 Core 的路径身份（64 位小写 hex），同样不写文件。
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

    /// 全新的工作区：生成一个 UUID v4 当身份，并写进清单。
    #[test]
    fn a_new_workspace_gets_a_generated_uuid_identity() {
        let repo = TempRepo::new("identity");

        let identity = resolve_project_id(repo.root()).expect("生成身份失败");
        let ProjectIdentity::Manifest(id) = &identity else {
            panic!("新工作区应当生成清单身份，实际 {identity:?}");
        };
        assert!(identity.is_manifest_backed());
        // UUID v4 的形态：36 字符、第 15 位是版本号 4、第 20 位是 variant。
        assert_eq!(id.len(), 36, "{id}");
        assert_eq!(&id[14..15], "4", "必须是 UUID v4：{id}");
        assert!(
            matches!(&id[19..20], "8" | "9" | "a" | "b"),
            "variant 位不对：{id}"
        );

        // 第二次解析必须给出**同一个** id（不再生成新的、也不重写文件）。
        let again = resolve_project_id(repo.root()).expect("再次解析身份失败");
        assert_eq!(again, identity, "同一个工作区的身份必须稳定");

        let manifest = load_project_manifest(repo.root()).manifest;
        assert_eq!(manifest.id.as_deref(), Some(id.as_str()));
    }
}
