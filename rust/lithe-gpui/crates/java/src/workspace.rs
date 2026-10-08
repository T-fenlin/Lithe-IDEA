use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lithe_gpui_shared::core_json;
use serde_json::{Value, json};

const WORKSPACE_BUILD_FILES: &[&str] = &["pom.xml", "build.gradle", "build.gradle.kts"];
pub(crate) const LAST_USED_MARKER: &str = ".lithe-last-used";
pub(crate) const CACHE_DIR_ENV: &str = "LITHE_GPUI_CACHE_DIR";

#[derive(Clone, Debug)]
pub(crate) struct JdtIndexCache {
    pub(crate) cache_directory: PathBuf,
    pub(crate) workspace_key: String,
    pub(crate) workspace_fingerprint: String,
    pub(crate) state_directory_existed: bool,
}

impl JdtIndexCache {
    pub(crate) fn state_directory(&self) -> PathBuf {
        self.cache_directory.join("jdtls").join(&self.workspace_key)
    }
}

pub(crate) fn plan(workspace_root: &Path, jdtls_version: &str) -> Result<JdtIndexCache, String> {
    let inputs = collect_fingerprint_inputs(workspace_root)?;
    let workspace_fingerprint = resolve_fingerprint(&inputs, jdtls_version)?;
    let workspace_key = resolve_workspace_key(workspace_root, &workspace_fingerprint)?;
    let cache_directory = language_server_cache(
        &cache_root()
            .ok_or_else(|| "推导不出应用缓存目录（Windows 上是 %LOCALAPPDATA%）。".to_string())?,
    );
    let state_directory_existed = cache_directory.join("jdtls").join(&workspace_key).is_dir();
    Ok(JdtIndexCache {
        cache_directory,
        workspace_key,
        workspace_fingerprint,
        state_directory_existed,
    })
}

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
    std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join("Lithe").join("cache"))
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

pub(crate) fn language_server_cache(cache_root: &Path) -> PathBuf {
    cache_root.join("language-servers")
}

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
                return Err(format!(
                    "无法读取 Java 构建文件 {}：{error}",
                    path.display()
                ));
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
        let entry = entry.map_err(|error| {
            format!(
                "无法读取 Java 工作区条目 {}：{error}",
                workspace_root.display()
            )
        })?;
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
                ));
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

#[derive(Clone, Debug, Default)]
pub(crate) struct CleanupOutcome {
    pub(crate) retention_days: u64,
    pub(crate) removed: usize,
}

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
            ));
        }
    };

    let now_unix_seconds = now_unix_seconds()?;
    let mut observed = BTreeSet::new();
    let mut candidates = Vec::new();
    if let Some(entries) = entries {
        for entry in entries {
            let entry = entry
                .map_err(|error| format!("无法读取 {} 中的条目：{error}", jdtls_cache.display()))?;
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
                std::fs::write(path.join(LAST_USED_MARKER), now_unix_seconds.to_string()).map_err(
                    |error| format!("无法标记 JDTLS 缓存 {} 为活动：{error}", path.display()),
                )?;
                now_unix_seconds
            } else {
                let directory_modified =
                    system_time_unix_seconds(metadata.modified().map_err(|error| {
                        format!("无法读取 {} 的时间戳：{error}", path.display())
                    })?)?;
                let marker_modified = match std::fs::metadata(path.join(LAST_USED_MARKER)) {
                    Ok(marker) => Some(system_time_unix_seconds(marker.modified().map_err(
                        |error| format!("无法读取 {} 标记的时间戳：{error}", path.display()),
                    )?)?),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => {
                        return Err(format!(
                            "无法检查 {} 的 last-used 标记：{error}",
                            path.display()
                        ));
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

fn core_error(error: lithe_gpui_shared::CoreError) -> String {
    match error.code() {
        Some(code) => format!("{code}：{error}"),
        None => error.to_string(),
    }
}

pub(crate) fn maven_context(root: &Path) -> Option<Value> {
    let scan = match core_json("maven.scan", json!({ "root": root.to_string_lossy() })) {
        Ok(scan) => scan?,
        Err(error) => {
            println!("S1_JAVA_MAVEN could not scan: error={error}");
            return None;
        }
    };
    let overridden = crate::toolchain::maven_settings_override();
    maven_context_from_scan(&scan, overridden.as_deref())
}

fn maven_context_from_scan(scan: &Value, settings_override: Option<&Path>) -> Option<Value> {
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
    let settings_path = settings_override
        .map(|path| path.to_string_lossy().to_string())
        .filter(|path| !path.is_empty())
        .or_else(|| {
            scan.get("settingsPath")
                .and_then(Value::as_str)
                .filter(|path| !path.is_empty())
                .map(str::to_string)
        });
    if let Some(settings_path) = settings_path {
        context["settingsPath"] = json!(settings_path);
    }
    Some(context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_scan_result_means_no_context() {
        assert!(maven_context_from_scan(&Value::Null, None).is_none());
        assert!(maven_context_from_scan(&Value::Null, Some(Path::new("ci.xml"))).is_none());
    }

    #[test]
    fn root_project_maps_to_a_versioned_context() {
        let context = maven_context_from_scan(
            &json!({
                "relativePath": ".",
                "groupId": "demo",
                "artifactId": "lite-fixture",
                "profiles": [{ "id": "dev" }, { "id": "prod" }],
                "settingsPath": "/home/u/.m2/settings.xml",
                "modules": []
            }),
            None,
        )
        .expect("有 relativePath 就是 Maven 项目");
        assert_eq!(context["version"], 1);
        assert_eq!(context["reactorPath"], ".");
        assert_eq!(context["profiles"], json!(["dev", "prod"]));
        assert_eq!(context["settingsPath"], "/home/u/.m2/settings.xml");
        assert_eq!(context["skipTests"], false);
        assert!(context.get("localRepositoryPath").is_none());
        assert!(context.get("mavenExecutablePath").is_none());
    }

    #[test]
    fn a_selected_settings_file_overrides_the_scanned_one() {
        let scanned = json!({
            "relativePath": ".",
            "profiles": [],
            "settingsPath": "/home/u/.m2/settings.xml"
        });
        let context = maven_context_from_scan(&scanned, Some(Path::new("D:/ci/settings.xml")))
            .expect("有 relativePath 就是 Maven 项目");
        assert_eq!(
            context["settingsPath"],
            json!(Path::new("D:/ci/settings.xml").to_string_lossy())
        );

        let without = json!({ "relativePath": "projects/demo", "profiles": [] });
        let context = maven_context_from_scan(&without, Some(Path::new("D:/ci/settings.xml")))
            .expect("子模块也是 Maven 项目");
        assert_eq!(
            context["settingsPath"],
            json!(Path::new("D:/ci/settings.xml").to_string_lossy())
        );
    }

    #[test]
    fn nested_reactor_and_empty_profiles() {
        let context = maven_context_from_scan(
            &json!({
                "relativePath": "projects/demo",
                "profiles": []
            }),
            None,
        )
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

    #[test]
    fn language_server_cache_adds_one_level() {
        assert_eq!(
            language_server_cache(Path::new("/tmp/lithe")),
            PathBuf::from("/tmp/lithe").join("language-servers")
        );
    }
}
