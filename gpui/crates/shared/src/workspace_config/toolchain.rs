//! 工具链五个值的**项目本机层**：`.lithe/run.local.json` 的 `toolchain` 对象
//! 与 `.lithe/maven.local.json`。
//!
//! ## 优先级
//!
//! ```text
//! 项目本机（本模块）> 全局默认（全局 settings.json）> 自动发现
//! ```
//!
//! 设计真源：`.agents/notes/proposed/architecture/2026-09-26-workspace-configuration-layers.md`
//! 的「三、工作区层」与「四、覆盖顺序」。为什么保留"全局默认"这一级：多数开发者在这台机器上
//! 主用一个 JDK，新建项目不该要求重新手填一遍。
//!
//! ## 五个值分别在哪个文件
//!
//! | 值 | 文件 | 键 |
//! | --- | --- | --- |
//! | `javaHomePath` | `run.local.json` | `toolchain.java.homePath` |
//! | `mavenExecutablePath` | `run.local.json` | `toolchain.maven.executablePath` |
//! | `mavenJavaHomePath` | `run.local.json` | `toolchain.maven.javaHomePath` |
//! | `mavenSettingsPath` | `maven.local.json` | `settingsPath` |
//! | `mavenLocalRepositoryPath` | `maven.local.json` | `localRepositoryPath` |
//!
//! 前三个的形状**不是本模块定的**：`.lithe/run.local.json` 就是一份运行配置文档（契约
//! `shared/contracts/run-configuration-v2.schema.json`，`version: 2`），它的根级 `toolchain`
//! 对象由那份 schema 定义（`toolchain.java.homePath` / `toolchain.maven.{executablePath,javaHomePath}`），
//! 键名与 Core 契约逐字相同。后两个是本模块新开的文件，schema 在
//! `shared/contracts/maven-local-v1.schema.json`。
//!
//! ## 空串 = 这一层没设（也是"清除"的写法）
//!
//! 五个值一律按**空串表示未设置**处理（与全局 `settings.json` 的口径一致：真源也是 `""` 表示
//! "用自动检测的值"）。所以"清除项目覆盖"就是把空串写进去 —— 不能用"省略字段"表示清除，
//! 因为省略的字段会被 [`crate::document`] 的未知键保留机制从上一版文档里**原样带回来**。
//!
//! ## 旧路径双读（设计改名后的过渡）
//!
//! 设计把本机层从 `.lithe/run/local.json` 改名成 `.lithe/run.local.json`。现役 macOS / Windows
//! 产品写的是**旧路径**，所以这里读的时候会回落到旧路径；写只写新路径。
//!
//! ⚠️ 回落是**文件级**的，不是逐字段的：新文件一旦存在，就以它为准（哪怕它的某个值是空串）。
//! 逐字段回落会让"在新文件里清除某个值"被旧文件里的老值顶回来，用户看到"清不掉"。
//!
//! ## 不碰 `configurations`
//!
//! `run.local.json` 里还有运行配置数组（`configurations`，契约要求必填），但**它不归本模块**。
//! 本模块只在文件里还没有这个键时补一个空数组把契约补齐，其余情况原样带过 —— 否则一次"改 JDK"
//! 就会把别的生产者写进去的运行配置覆盖掉。

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

/// `.lithe/run.local.json` 的文档版本。与运行配置契约一致（`run-configuration-v2.schema.json`）。
pub const RUN_LOCAL_DOCUMENT_VERSION: u32 = 2;

/// `.lithe/maven.local.json` 的文档版本。
pub const MAVEN_LOCAL_DOCUMENT_VERSION: u32 = 1;

/// 五个工具链覆盖值（空串 = 这一层没设）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolchainPaths {
    /// JDK 主目录（`toolchain.java.homePath` / `javaHomePath`）。
    pub java_home_path: String,
    /// Maven 可执行文件（`toolchain.maven.executablePath` / `mavenExecutablePath`）。
    pub maven_executable_path: String,
    /// Maven 使用的 JDK 主目录（`toolchain.maven.javaHomePath` / `mavenJavaHomePath`）。
    pub maven_java_home_path: String,
    /// Maven 用户 `settings.xml`（`maven.local.json` 的 `settingsPath`）。
    pub maven_settings_path: String,
    /// Maven 本地仓库（`maven.local.json` 的 `localRepositoryPath`）。
    pub maven_local_repository_path: String,
}

/// 一个值最终来自哪一层（界面要如实标注，见设计 Note 第四节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverrideOrigin {
    /// 来自项目本机层（`.lithe/*.local.json`）。
    Project,
    /// 来自全局默认（全局 `settings.json`）。
    Global,
    /// 两层都没有：用自动发现。
    Unset,
}

impl OverrideOrigin {
    /// 诊断行里的稳定 token。
    pub fn id(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Global => "global",
            Self::Unset => "unset",
        }
    }
}

/// 五个值各自的来源（字段名与 [`ToolchainPaths`] 一一对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverrideOrigins {
    /// 见 [`ToolchainPaths::java_home_path`]。
    pub java_home_path: OverrideOrigin,
    /// 见 [`ToolchainPaths::maven_executable_path`]。
    pub maven_executable_path: OverrideOrigin,
    /// 见 [`ToolchainPaths::maven_java_home_path`]。
    pub maven_java_home_path: OverrideOrigin,
    /// 见 [`ToolchainPaths::maven_settings_path`]。
    pub maven_settings_path: OverrideOrigin,
    /// 见 [`ToolchainPaths::maven_local_repository_path`]。
    pub maven_local_repository_path: OverrideOrigin,
}

impl ToolchainPaths {
    /// 这一层有没有设任何值（空白不算设）。
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 非空值的个数（诊断行用）。
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

    /// 去掉每个值的首尾空白（写盘前统一走一次）。
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

    /// 逐字段解析生效值：**本机层非空就用它，否则用全局那一个**。
    ///
    /// 这是"项目本机 > 全局默认"这条优先级的唯一实现点（界面与语言服务登记都走它）。
    pub fn resolve(local: &Self, global: &Self) -> (Self, OverrideOrigins) {
        let mut resolved = Self::default();
        let mut origins = OverrideOrigins {
            java_home_path: OverrideOrigin::Unset,
            maven_executable_path: OverrideOrigin::Unset,
            maven_java_home_path: OverrideOrigin::Unset,
            maven_settings_path: OverrideOrigin::Unset,
            maven_local_repository_path: OverrideOrigin::Unset,
        };

        // 五个字段走同一条规则，所以用一个闭包逐位处理，避免五段复制粘贴各自漂移。
        let pick = |local_value: &str, global_value: &str, target: &mut String, origin: &mut OverrideOrigin| {
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

    /// 一行可 grep 的诊断（五个值的来源）。
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

/// 一次读取的结论。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalToolchainState {
    /// 项目本机层里的五个值（空串 = 没设）。
    pub paths: ToolchainPaths,
    /// 项目本机层是否真的存在（新文件或旧文件之一可读）。界面据此区分"项目层没设"与"没有项目层"。
    pub file_existed: bool,
    /// 逐文件诊断（坏版本、读失败等）。
    pub diagnostics: Vec<String>,
}

/// 读项目本机层。**不创建文件**；任何失败都退化成"没设任何值 + 诊断"。
pub fn load(root: &Path) -> LocalToolchainState {
    let paths = WorkspaceConfigPaths::new(root);
    let mut state = LocalToolchainState::default();

    // 前三个值：新路径优先，**新文件不存在时**才回落到旧路径（文件级回落，见模块文档）。
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
            state
                .diagnostics
                .push(format!("run_local {diagnostic}"));
        }
        state.paths.java_home_path = parsed.value.toolchain.java.home_path;
        state.paths.maven_executable_path = parsed.value.toolchain.maven.executable_path;
        state.paths.maven_java_home_path = parsed.value.toolchain.maven.java_home_path;
    }

    // 后两个值：`.lithe/maven.local.json`（新文件，没有旧路径可回落）。
    let maven_local = paths.maven_local();
    if let Some(text) = document::read_document_text(&maven_local).ok().flatten() {
        state.file_existed = true;
        let parsed = document::parse::<MavenLocalDocument>(&text, MAVEN_LOCAL_DOCUMENT_VERSION);
        for diagnostic in &parsed.diagnostics {
            state
                .diagnostics
                .push(format!("maven_local {diagnostic}"));
        }
        state.paths.maven_settings_path = parsed.value.settings_path;
        state.paths.maven_local_repository_path = parsed.value.local_repository_path;
    }

    state.paths = state.paths.normalized();
    state
}

/// 一次落盘的结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainSaved {
    /// `run.local.json` 写入的字节数。
    pub run_local_bytes: usize,
    /// `maven.local.json` 写入的字节数。
    pub maven_local_bytes: usize,
    /// 写 `.lithe/` 之前"确保不共享"的结论。
    pub exclusion: EnsureExcluded,
    /// `.lithe/.gitignore` 的补齐结论（第二道闸）。
    pub ignore: IgnoreFileOutcome,
}

/// 把五个值写进项目本机层。
///
/// 顺序（与 `save_project_manifest` 同一条约定）：**先确保 `.lithe/` 不进 Git，再写文件**；
/// 两个 `.local.json` 写完之后补 `.lithe/.gitignore`（共享之后靠它挡住本机层）。
pub fn save(root: &Path, paths: &ToolchainPaths) -> Result<ToolchainSaved, WorkspaceConfigError> {
    let paths = paths.normalized();
    let config = WorkspaceConfigPaths::new(root);

    // 写 `.lithe/` 之前的唯一入口：先让"默认不共享"落在磁盘上。
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
    let mut merged = document::merge_document(previous.as_ref(), &typed, RUN_LOCAL_DOCUMENT_VERSION)
        .map_err(WorkspaceConfigError::Json)?;
    if let Value::Object(object) = &mut merged {
        // 契约要求 `configurations` 必填，而它不归本模块（见模块文档）：只在文件里还没有它时
        // 补一个空数组，其余情况由未知键保留机制原样带过。
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
    document::save_document(&path, &typed, previous.as_ref(), MAVEN_LOCAL_DOCUMENT_VERSION)
        .map_err(WorkspaceConfigError::Io)
}

/// 写之前先确认这个文档**没有**声明比我们支持的更高的版本。
///
/// 为什么不只依赖 `merge_document`：它会把 `version` 写成我们支持的版本，于是"未来的文件"
/// 被无声降级 —— 那正是"更高版本转只读"这条规则要防的事（见设计 Note 第六节）。
/// 发现更高版本时**整次保存失败**（而不是只跳过这一个文件）：用户看到的是明确的一条错误，
/// 而不是"保存了但只保存了一半"。
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

/// `.lithe/run.local.json` 里**本模块拥有**的那部分。
///
/// 字段一律不加 `skip_serializing_if`：省略字段会被未知键保留机制从上一版文档里带回来，
/// 于是"清除某个值"就清不掉（见模块文档）。`configurations` 是**唯一**的例外，它不归本模块，
/// 所以既不声明也不序列化 —— 它由未知键保留机制原样带过，缺失时才由 [`save_run_local`] 补空数组。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RunLocalDocument {
    #[serde(default)]
    toolchain: ToolchainObject,
}

/// `toolchain` 对象（形状由 `run-configuration-v2.schema.json` 定义，键名与 Core 契约逐字相同）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ToolchainObject {
    #[serde(default)]
    java: ToolchainJava,
    #[serde(default)]
    maven: ToolchainMaven,
}

/// `toolchain.java`。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ToolchainJava {
    #[serde(default)]
    home_path: String,
}

/// `toolchain.maven`。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ToolchainMaven {
    #[serde(default)]
    executable_path: String,
    #[serde(default)]
    java_home_path: String,
}

/// `.lithe/maven.local.json` 的文档（schema：`shared/contracts/maven-local-v1.schema.json`）。
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

    /// 本机层非空就用本机层，否则用全局；两者都没有就是"自动"。
    #[test]
    fn project_layer_wins_over_the_global_default() {
        let local = ToolchainPaths {
            java_home_path: "D:\\project-jdk".to_string(),
            // 只设一个，其余应当落到全局。
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

    /// 空白与空串一样算"这一层没设"（否则一个空格就能顶掉全局值）。
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

    /// 写一次再读回来：五个值都在它们该在的文件里。
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

        // 落盘形状：前三个在 run.local.json 的 toolchain 对象里，后两个在 maven.local.json 里。
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
        // 契约要求 `configurations` 必填：新建文件时由我们补空数组。
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

    /// 清除某个值要真的清掉：写空串之后读回来是空串（而不是被旧文档带回来）。
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
        assert_eq!(loaded.paths.maven_executable_path, "D:\\maven\\bin\\mvn.cmd");
    }

    /// **不碰 `configurations`**：别的生产者写进去的运行配置必须逐字保留。
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
        // 未知键也照旧保留（document 层的职责，这里守一条端到端的）。
        assert_eq!(
            value.pointer("/thirdParty/keep"),
            Some(&Value::from(true))
        );
    }

    /// 旧路径双读：新文件不存在时读 `.lithe/run/local.json`；写只写新路径。
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

        // 写一次之后只认新文件（旧文件里的值不再被读回来 —— 文件级回落，见模块文档）。
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

        // 在新文件里清除该值之后，旧文件的老值**不得**顶回来。
        save(repo.root(), &ToolchainPaths::default()).expect("写入失败");
        assert_eq!(load(repo.root()).paths.java_home_path, "");
    }

    /// 写 `.lithe/` 之前必须先确保不共享：守卫没跑（不是 Git 仓库）时也要照写，但仓库里
    /// 必须看不到 `.lithe`。
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

    /// 写入必须顺便补上 `.lithe/.gitignore`（第二道闸）。
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

    /// 版本比程序新：**只读**——能读到的值照读，但不得覆盖。
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

        // 读得了不等于写得动：保存必须**拒绝**，而不是把 `version: 99` 降级成 2。
        let error = save(repo.root(), &full()).expect_err("更高版本的文档必须拒绝写入");
        assert!(
            matches!(error, WorkspaceConfigError::ReadOnly { declared, supported, .. } if declared == 99 && supported == RUN_LOCAL_DOCUMENT_VERSION),
            "应当是 ReadOnly：{error}"
        );
        let after = repo.read(".lithe/run.local.json");
        assert_eq!(after, future, "被拒绝的写入不得改动文件一个字");
    }
}
