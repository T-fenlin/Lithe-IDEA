//! JDT LS **工作区索引缓存**的宿主侧：指纹输入、缓存键、目录、过期回收。
//!
//! 移植自 Windows host 的 `windows/tauri/src-tauri/src/lsp/jdt_workspace.rs`。
//!
//! ## 分工（契约 `shared/contracts/rust-core-api.md:1239-1301`）
//!
//! | 谁 | 做什么 |
//! | --- | --- |
//! | 平台（本模块） | 观察根构建描述符的时间戳/大小与直接 Maven 模块名；枚举缓存目录；读写 last-used 标记；删除目录 |
//! | Rust Core | `java.jdtWorkspaceFingerprint` 归一化 + 哈希；`lsp.jdtWorkspaceKey` 算目录名；`java.jdtCacheRetention` 选过期项（固定 30 天） |
//!
//! Core 里那句"Omitting the fingerprint preserves the legacy path-only key for older clients"
//! 说的是兼容路径：**我们总是带指纹**，否则同一路径换了项目结构会复用一份陈旧的项目模型。
//!
//! ## 目录形状
//!
//! ```text
//! <cacheDirectory>/jdtls/<workspaceKey>/      ← Core 自己拼的后两级（jdt.rs 的 JDTLS_DATA_DIRECTORY）
//! ```
//!
//! 所以宿主只需要给出 `cacheDirectory`，`lsp.startServer` 会把 `-data` 指到
//! `cacheDirectory/jdtls/<workspaceKey>`。本模块额外**自己算一遍**同一个键，用途只有两个：
//! 回收时排除"正在用的那个"，以及给验证链路一个可以 `dir` 的稳定路径。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lithe_gpui_shared::core_json;
use serde_json::{Value, json};

/// 根构建描述符（与 Windows 同一份列表；**不递归**遍历，递归是 JDT 自己的事）。
const WORKSPACE_BUILD_FILES: &[&str] = &["pom.xml", "build.gradle", "build.gradle.kts"];
/// last-used 标记文件名（Windows 的 `JDT_CACHE_LAST_USED_MARKER`）。
pub(crate) const LAST_USED_MARKER: &str = ".lithe-last-used";
/// 缓存根目录的显式覆盖（验证链路用；与 `LITHE_GPUI_SETTINGS_FILE` 同一口径）。
pub(crate) const CACHE_DIR_ENV: &str = "LITHE_GPUI_CACHE_DIR";

/// 一次启动要用的 JDT 索引缓存计划。
#[derive(Clone, Debug)]
pub(crate) struct JdtIndexCache {
    /// 送进 `lsp.startServer` 的 `cacheDirectory`（**不含** `jdtls/<key>` 两级）。
    pub(crate) cache_directory: PathBuf,
    /// `lsp.jdtWorkspaceKey` 返回的 64 位小写十六进制键。
    pub(crate) workspace_key: String,
    /// `java.jdtWorkspaceFingerprint` 返回的不透明指纹，原样回传、**不解析**。
    pub(crate) workspace_fingerprint: String,
    /// 本次启动前该工作区状态目录是否已经存在（"重开复用"的直接证据）。
    pub(crate) state_directory_existed: bool,
}

impl JdtIndexCache {
    /// Core 为这次会话选择的状态目录（`cacheDirectory/jdtls/<workspaceKey>`）。
    pub(crate) fn state_directory(&self) -> PathBuf {
        self.cache_directory
            .join("jdtls")
            .join(&self.workspace_key)
    }
}

/// 算这次启动的缓存计划：观察 → Core 指纹 → Core 键 → 目录。
pub(crate) fn plan(workspace_root: &Path, jdtls_version: &str) -> Result<JdtIndexCache, String> {
    let inputs = collect_fingerprint_inputs(workspace_root)?;
    let workspace_fingerprint = resolve_fingerprint(&inputs, jdtls_version)?;
    let workspace_key = resolve_workspace_key(workspace_root, &workspace_fingerprint)?;
    let cache_directory = language_server_cache(&cache_root().ok_or_else(|| {
        "推导不出应用缓存目录（Windows 上是 %LOCALAPPDATA%）。".to_string()
    })?);
    let state_directory_existed = cache_directory
        .join("jdtls")
        .join(&workspace_key)
        .is_dir();
    Ok(JdtIndexCache {
        cache_directory,
        workspace_key,
        workspace_fingerprint,
        state_directory_existed,
    })
}

/// 平台缓存根（**不含** `language-servers`）。
///
/// gpui 没有 `app_cache_dir`（Windows host 用的是 Tauri 的），所以平台分支在这里：
///
/// | 平台 | 路径 |
/// | --- | --- |
/// | Windows | `%LOCALAPPDATA%\Lithe\cache`（Tauri `app_cache_dir` 的同口径） |
/// | macOS | `~/Library/Caches/Lithe` |
/// | Linux | `$XDG_CACHE_HOME/lithe`，回落 `~/.cache/lithe` |
fn cache_root() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(CACHE_DIR_ENV) {
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    platform_cache_root()
}

#[cfg(target_os = "windows")]
fn platform_cache_root() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|base| PathBuf::from(base).join("Lithe").join("cache"))
}

#[cfg(target_os = "macos")]
fn platform_cache_root() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join("Library")
            .join("Caches")
            .join("Lithe")
    })
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn platform_cache_root() -> Option<PathBuf> {
    if let Some(base) = std::env::var_os("XDG_CACHE_HOME") {
        if !base.is_empty() {
            return Some(PathBuf::from(base).join("lithe"));
        }
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache").join("lithe"))
}

/// 语言服务器的缓存根：Windows host 用 `language-servers` 这一层，逐字照搬。
pub(crate) fn language_server_cache(cache_root: &Path) -> PathBuf {
    cache_root.join("language-servers")
}

/// 指纹输入：根构建描述符 + 直接含 `pom.xml` 的子目录名（**不递归**）。
struct FingerprintInputs {
    build_files: Vec<Value>,
    direct_maven_modules: Vec<String>,
}

fn collect_fingerprint_inputs(workspace_root: &Path) -> Result<FingerprintInputs, String> {
    let mut build_files = Vec::new();
    for name in WORKSPACE_BUILD_FILES {
        let path = workspace_root.join(name);
        let metadata = match std::fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!("无法读取 Java 构建文件 {}：{error}", path.display()))
            }
        };
        let modified = metadata
            .modified()
            .map_err(|error| format!("无法读取构建文件时间戳 {}：{error}", path.display()))?
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("构建文件时间戳早于 Unix 纪元 {}：{error}", path.display()))?
            .as_millis();
        let modified_unix_milliseconds = u64::try_from(modified)
            .map_err(|_| format!("构建文件时间戳超出范围：{}", path.display()))?;
        build_files.push(json!({
            "path": name,
            "modifiedUnixMilliseconds": modified_unix_milliseconds,
            "sizeBytes": metadata.len(),
        }));
    }

    let entries = std::fs::read_dir(workspace_root)
        .map_err(|error| format!("无法枚举 Java 工作区 {}：{error}", workspace_root.display()))?;
    let mut direct_maven_modules = Vec::new();
    for entry in entries {
        let entry = entry
            .map_err(|error| format!("无法读取 Java 工作区条目 {}：{error}", workspace_root.display()))?;
        let path = entry.path();
        let is_directory = entry
            .file_type()
            .map_err(|error| format!("无法判定 Java 工作区条目类型 {}：{error}", path.display()))?
            .is_dir();
        if !is_directory {
            continue;
        }
        match std::fs::metadata(path.join("pom.xml")) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!(
                    "无法读取 Maven 模块描述符 {}：{error}",
                    path.join("pom.xml").display()
                ))
            }
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("Java 工作区模块名不是合法 UTF-8：{}", path.display()))?;
        direct_maven_modules.push(name);
    }

    Ok(FingerprintInputs {
        build_files,
        direct_maven_modules,
    })
}

/// 观察 → Core：归一化、排序、去重、哈希全在 Core（`java.jdtWorkspaceFingerprint`）。
fn resolve_fingerprint(inputs: &FingerprintInputs, jdtls_version: &str) -> Result<String, String> {
    let data = core_json(
        "java.jdtWorkspaceFingerprint",
        json!({
            "buildFiles": inputs.build_files,
            "directMavenModules": inputs.direct_maven_modules,
            "jdtlsVersion": jdtls_version,
        }),
    )
    .map_err(core_error)?;
    data.as_ref()
        .and_then(|data| data.get("workspaceFingerprint"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "java.jdtWorkspaceFingerprint 的响应缺少 workspaceFingerprint".to_string())
}

/// `lsp.jdtWorkspaceKey`：与 `lsp.startServer` 内部**同一个**归一化 + SHA-256。
pub(crate) fn resolve_workspace_key(
    workspace_root: &Path,
    workspace_fingerprint: &str,
) -> Result<String, String> {
    let data = core_json(
        "lsp.jdtWorkspaceKey",
        json!({
            "workspaceRoot": workspace_root.to_string_lossy(),
            "workspaceFingerprint": workspace_fingerprint,
        }),
    )
    .map_err(core_error)?;
    let key = data
        .as_ref()
        .and_then(|data| data.get("workspaceKey"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "lsp.jdtWorkspaceKey 的响应缺少 workspaceKey".to_string())?;
    if !is_workspace_key(&key) {
        return Err(format!("lsp.jdtWorkspaceKey 返回了非法目录名：{key}"));
    }
    Ok(key)
}

/// 回收结果，只用于诊断行。
#[derive(Clone, Debug, Default)]
pub(crate) struct CleanupOutcome {
    /// Core 给出的固定保留天数（30）。
    pub(crate) retention_days: u64,
    /// 真正删掉的目录数。
    pub(crate) removed: usize,
}

/// 按 `java.jdtCacheRetention` 回收过期的工作区状态目录。
///
/// 判据照 Windows：只处理 64 位小写十六进制的目录名、拒绝符号链接、**永不删活动键**、
/// 删除前重新校验、活动目录写一次 last-used 标记（`metadata.modified` 会被 `-data` 里的
/// 索引写坏，所以标记时间是权威的"最后一次使用"）。
///
/// 失败不致命（回收只是省磁盘）：调用方把它降级成一条 `S1_JAVA_CACHE_RECLAIM` 警告。
pub(crate) fn cleanup_expired(
    cache_directory: &Path,
    active_workspace_key: &str,
) -> Result<CleanupOutcome, String> {
    if !is_workspace_key(active_workspace_key) {
        return Err("活动 JDTLS 工作区键非法，拒绝回收。".to_string());
    }
    let jdtls_cache = cache_directory.join("jdtls");
    let entries = match std::fs::read_dir(&jdtls_cache) {
        Ok(entries) => Some(entries),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "无法枚举 JDTLS 缓存目录 {}：{error}",
                jdtls_cache.display()
            ))
        }
    };

    let now_unix_seconds = now_unix_seconds()?;
    let mut observed = BTreeSet::new();
    let mut candidates = Vec::new();
    if let Some(entries) = entries {
        for entry in entries {
            let entry = entry.map_err(|error| {
                format!("无法读取 {} 中的条目：{error}", jdtls_cache.display())
            })?;
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(|error| format!("无法检查 {}：{error}", path.display()))?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                continue;
            }
            let Some(workspace_key) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if !is_workspace_key(&workspace_key) {
                continue;
            }

            let modified = if workspace_key == active_workspace_key {
                std::fs::write(
                    path.join(LAST_USED_MARKER),
                    now_unix_seconds.to_string(),
                )
                .map_err(|error| {
                    format!("无法标记 JDTLS 缓存 {} 为活动：{error}", path.display())
                })?;
                now_unix_seconds
            } else {
                let directory_modified = system_time_unix_seconds(
                    metadata
                        .modified()
                        .map_err(|error| format!("无法读取 {} 的时间戳：{error}", path.display()))?,
                )?;
                let marker_modified = match std::fs::metadata(path.join(LAST_USED_MARKER)) {
                    Ok(marker) => Some(system_time_unix_seconds(marker.modified().map_err(
                        |error| format!("无法读取 {} 标记的时间戳：{error}", path.display()),
                    )?)?),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => {
                        return Err(format!(
                            "无法检查 {} 的 last-used 标记：{error}",
                            path.display()
                        ))
                    }
                };
                marker_modified
                    .map(|value| value.max(directory_modified))
                    .unwrap_or(directory_modified)
            };
            observed.insert(workspace_key.clone());
            candidates.push(json!({
                "workspaceKey": workspace_key,
                "lastModifiedUnixSeconds": modified,
            }));
        }
    }

    let plan = core_json(
        "java.jdtCacheRetention",
        json!({
            "nowUnixSeconds": now_unix_seconds,
            "activeWorkspaceKey": active_workspace_key,
            "entries": candidates,
        }),
    )
    .map_err(core_error)?;
    let retention_days = plan
        .as_ref()
        .and_then(|plan| plan.get("retentionDays"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let expired = plan
        .as_ref()
        .and_then(|plan| plan.get("expiredWorkspaceKeys"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut removed = 0usize;
    for key in expired.iter().filter_map(Value::as_str) {
        // Core 已经保证"不选活动键"，这里再判一次是**纵深防御**：一个坏掉的响应不该删掉
        // 正在被 JDTLS 写着的目录（Windows host 同样这么做）。
        if key == active_workspace_key || !is_workspace_key(key) || !observed.contains(key) {
            return Err(format!("Core 返回了未观察到的回收目标：{key}"));
        }
        let target = jdtls_cache.join(key);
        let metadata = std::fs::symlink_metadata(&target)
            .map_err(|error| format!("无法复核 {}：{error}", target.display()))?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(format!(
                "JDTLS 回收目标不是可删除目录：{}",
                target.display()
            ));
        }
        std::fs::remove_dir_all(&target)
            .map_err(|error| format!("无法删除过期 JDTLS 缓存 {}：{error}", target.display()))?;
        removed += 1;
    }

    Ok(CleanupOutcome {
        retention_days,
        removed,
    })
}

/// 契约里的目录名判据：64 位小写十六进制（`rust-core-api.md:1294-1296`）。
fn is_workspace_key(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn now_unix_seconds() -> Result<u64, String> {
    system_time_unix_seconds(SystemTime::now())
}

fn system_time_unix_seconds(value: SystemTime) -> Result<u64, String> {
    value
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("系统时钟早于 Unix 纪元：{error}"))
}

/// 信封级失败统一成一条可读文本 —— 诊断行要的是原因，不是分层。
fn core_error(error: lithe_gpui_shared::CoreError) -> String {
    match error.code() {
        Some(code) => format!("{code}：{error}"),
        None => error.to_string(),
    }
}

/// **Maven 项目上下文**（`lsp.startServer` 的 `mavenContext`）。
///
/// 为什么值得做：Core **已经**接受这个字段（`rust/lithe-core/src/project/maven.rs:63-78` 的结构体，
/// 契约 `shared/contracts/rust-core-api.md:1170-1178`），拿到之后它会
/// ① 把 reactor 的 main / test / generated 四类源根归一成 `java.project.sourcePaths`
/// （**生成源根因此生效**：注解处理器产出的代码也能被补全与跳转看到）；
/// ② 在 `ServiceReady` 之后按项目下发 `org.eclipse.m2e.core.selectedProfiles`；
/// ③ 把 `settingsPath` 发布成 `java.configuration.maven.userSettings`。
///
/// gpui 侧此前**完全没传**（`session.rs` 的 startServer payload 里没有这个字段），这些能力一直空着。
/// **而且完全不需要我们解析 pom.xml**：`maven.scan` 直接给出 `relativePath` 与 `profiles`。
///
/// 返回 `None` = 工作区里没有可读的 `pom.xml`（`maven.scan` 明确返回 `null`）：
/// 那就**不带**这个字段 —— 带一个空 reactor 只会让 Core 去做无意义的校验。
pub(crate) fn maven_context(root: &Path) -> Option<Value> {
    let scan = match core_json("maven.scan", json!({ "root": root.to_string_lossy() })) {
        Ok(scan) => scan?,
        Err(error) => {
            // 失败**不静默**，也**不挡启动**：Maven 上下文只是"更完整的项目模型"，
            // 缺了它补全/跳转照样能用（只是看不到生成源根与 profile）。
            println!("S1_JAVA_MAVEN could not scan: error={error}");
            return None;
        }
    };
    maven_context_from_scan(&scan)
}

/// 从 `maven.scan` 的响应构造 `mavenContext`（纯函数，便于单测）。
///
/// 只带**有证据的**字段：`reactorPath`（scan 的 `relativePath`）、
/// `profiles`（scan 的 `profiles[].id`）、`settingsPath`（scan 给了才带）。
/// `localRepositoryPath` / `mavenExecutablePath` / `javaHomePath` **留空**：
/// 那属于「项目环境设置」的范围，gpui 侧还没有那个数据源 —— 宁可不传，也不猜一个值。
fn maven_context_from_scan(scan: &Value) -> Option<Value> {
    if scan.is_null() {
        return None;
    }
    let reactor_path = scan
        .get("relativePath")
        .and_then(Value::as_str)
        .unwrap_or(".");
    let profiles: Vec<Value> = scan
        .get("profiles")
        .and_then(Value::as_array)
        .map(|profiles| {
            profiles
                .iter()
                .filter_map(|profile| profile.get("id").and_then(Value::as_str))
                .map(|id| json!(id))
                .collect()
        })
        .unwrap_or_default();

    let mut context = json!({
        "version": 1,
        "reactorPath": reactor_path,
        "profiles": profiles,
        "skipTests": false,
    });
    if let Some(settings_path) = scan
        .get("settingsPath")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
    {
        context["settingsPath"] = json!(settings_path);
    }
    Some(context)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 没有 Maven 项目时**不传**这个字段（`maven.scan` 给 `null`）。
    #[test]
    fn no_scan_result_means_no_context() {
        assert!(maven_context_from_scan(&Value::Null).is_none());
    }

    /// 根 pom：`reactorPath` 是 `.`，profiles 取 `id`，`settingsPath` 原样带过去。
    #[test]
    fn root_project_maps_to_a_versioned_context() {
        let context = maven_context_from_scan(&json!({
            "relativePath": ".",
            "groupId": "demo",
            "artifactId": "lite-fixture",
            "profiles": [{ "id": "dev" }, { "id": "prod" }],
            "settingsPath": "/home/u/.m2/settings.xml",
            "modules": []
        }))
        .expect("有 relativePath 就是 Maven 项目");
        assert_eq!(context["version"], 1);
        assert_eq!(context["reactorPath"], ".");
        assert_eq!(context["profiles"], json!(["dev", "prod"]));
        assert_eq!(context["settingsPath"], "/home/u/.m2/settings.xml");
        assert_eq!(context["skipTests"], false);
        // 没有数据源的字段**不许出现**（宁可不传，也不猜一个值）。
        assert!(context.get("localRepositoryPath").is_none());
        assert!(context.get("mavenExecutablePath").is_none());
    }

    /// 子模块 reactor：`relativePath` 原样作为 `reactorPath`；没有 profiles 时给空表。
    #[test]
    fn nested_reactor_and_empty_profiles() {
        let context = maven_context_from_scan(&json!({
            "relativePath": "projects/demo",
            "profiles": []
        }))
        .expect("子模块也是 Maven 项目");
        assert_eq!(context["reactorPath"], "projects/demo");
        assert_eq!(context["profiles"], json!([]));
        assert!(context.get("settingsPath").is_none());
    }

    #[test]
    fn workspace_keys_must_be_lowercase_sha256() {
        assert!(is_workspace_key(&"a".repeat(64)));
        assert!(is_workspace_key(&"0123456789abcdef".repeat(4)));
        assert!(!is_workspace_key(&"A".repeat(64)));
        assert!(!is_workspace_key(&"g".repeat(64)));
        assert!(!is_workspace_key(&"a".repeat(63)));
    }

    /// 状态目录必须是 `cacheDirectory/jdtls/<key>`：Core 自己拼的就是这两级，
    /// 对不上就会"报告一个目录、实际用另一个目录"。
    #[test]
    fn state_directory_is_cache_directory_plus_two_levels() {
        let cache = JdtIndexCache {
            cache_directory: PathBuf::from("/tmp/cache/language-servers"),
            workspace_key: "a".repeat(64),
            workspace_fingerprint: "fingerprint".to_string(),
            state_directory_existed: false,
        };
        assert_eq!(
            cache.state_directory(),
            PathBuf::from("/tmp/cache/language-servers")
                .join("jdtls")
                .join("a".repeat(64))
        );
    }

    /// 语言服务器缓存根就是宿主缓存下加一层 `language-servers`（Windows 同款）。
    #[test]
    fn language_server_cache_adds_one_level() {
        assert_eq!(
            language_server_cache(Path::new("/tmp/lithe")),
            PathBuf::from("/tmp/lithe").join("language-servers")
        );
    }
}
