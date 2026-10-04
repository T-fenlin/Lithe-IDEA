use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

const EXECUTABLE_NAMES: &[&str] = &["jdtls.bat", "jdtls.cmd", "jdtls.exe", "jdtls"];
const MAX_EXE_WALK_DEPTH: usize = 12;
const CORE_PLUGIN_PREFIX: &str = "org.eclipse.jdt.ls.core_";
const EQUINOX_LAUNCHER_PREFIX: &str = "org.eclipse.equinox.launcher_";
const JAVA_DEBUG_BUNDLE_PREFIX: &str = "com.microsoft.java.debug.plugin-";
const JAVA_TEST_BUNDLE_LIST: &str = "extensions.txt";
const BUNDLED_MANIFEST: &str = include_str!("../../../../../third_party/jdtls/manifest.json");

pub(crate) const JAVA_EXECUTABLE_ENV: &str = "LITHE_JDTLS_JAVA";

const JAVA_HOME_ENV: &str = "JAVA_HOME";

const PATH_ENV: &str = "PATH";

#[derive(Clone, Debug)]
pub(crate) struct JdtlsInstallation {
    pub(crate) executable: PathBuf,
    pub(crate) launcher_jar_path: PathBuf,
    pub(crate) configuration_directory: PathBuf,
    pub(crate) lombok_agent_path: PathBuf,
    pub(crate) java_debug_bundle_path: Option<PathBuf>,
    pub(crate) java_extension_bundle_paths: Vec<PathBuf>,
    pub(crate) version: String,
}

#[derive(Clone, Debug)]
pub(crate) struct JavaRuntime {
    pub(crate) home: PathBuf,
    pub(crate) executable: PathBuf,
    pub(crate) version: String,
}

const MINIMUM_JAVA_MAJOR: u64 = 21;

fn configuration_directory_name() -> &'static str {
    if cfg!(windows) {
        "config_win"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "config_mac_arm"
        } else {
            "config_mac"
        }
    } else {
        "config_linux"
    }
}

fn java_executable_name() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

pub(crate) fn resolve(workspace_root: &Path) -> Result<JdtlsInstallation, String> {
    let executable = find_executable(workspace_root).ok_or_else(|| {
        "未找到 jdtls。请安装 Eclipse JDT Language Server 并加入 PATH，或使用内置 Java 语言服务的 Lithe 构建。"
            .to_string()
    })?;

    let mut roots = installation_roots(&executable);
    roots.dedup();
    let mut inspected = Vec::new();
    let mut resolved = None;
    for root in &roots {
        inspected.push(root.clone());
        match resources_for_root(root) {
            Ok(Some(resources)) => {
                resolved = Some((root.clone(), resources));
                break;
            }
            Ok(None) => {}
            Err(error) => return Err(error),
        }
    }

    let Some((root, resources)) = resolved else {
        let inspected = inspected
            .iter()
            .map(|root| root.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "选中的 JDTLS 安装缺少 Equinox 启动 JAR、平台配置目录、lombok/lombok.jar、\
             Java Debug Server 包，或 java-test/{JAVA_TEST_BUNDLE_LIST} 列出的 Java Test 扩展包。\
             已检查：{inspected}"
        ));
    };

    Ok(JdtlsInstallation {
        executable,
        launcher_jar_path: resources.launcher_jar_path,
        configuration_directory: resources.configuration_directory,
        lombok_agent_path: resources.lombok_agent_path,
        java_debug_bundle_path: resources.java_debug_bundle_path,
        java_extension_bundle_paths: resources.java_extension_bundle_paths,
        version: version_for_root(&root)?
            .or_else(manifest_version)
            .ok_or_else(|| format!("读不出 JDTLS 版本：{}", root.display()))?,
    })
}

struct LaunchResources {
    launcher_jar_path: PathBuf,
    configuration_directory: PathBuf,
    lombok_agent_path: PathBuf,
    java_debug_bundle_path: Option<PathBuf>,
    java_extension_bundle_paths: Vec<PathBuf>,
}

fn resources_for_root(root: &Path) -> Result<Option<LaunchResources>, String> {
    let Some(launcher_jar_path) =
        first_regular_file(&root.join("plugins"), EQUINOX_LAUNCHER_PREFIX)
    else {
        return Ok(None);
    };
    let java_debug_bundle_path =
        first_regular_file(&root.join("java-debug"), JAVA_DEBUG_BUNDLE_PREFIX);
    let Some(java_extension_bundle_paths) = java_test_extension_bundles(root)? else {
        return Ok(None);
    };
    let configuration_directory = root.join(configuration_directory_name());
    let lombok_agent_path = root.join("lombok").join("lombok.jar");
    if !configuration_directory.is_dir() || !lombok_agent_path.is_file() {
        return Ok(None);
    }
    Ok(Some(LaunchResources {
        launcher_jar_path,
        configuration_directory,
        lombok_agent_path,
        java_debug_bundle_path,
        java_extension_bundle_paths,
    }))
}

fn first_regular_file(directory: &Path, prefix: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(directory).ok()?;
    let mut matching = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".jar"))
        })
        .collect::<Vec<_>>();
    matching.sort();
    matching.into_iter().next()
}

fn java_test_extension_bundles(root: &Path) -> Result<Option<Vec<PathBuf>>, String> {
    let directory = root.join("java-test");
    let list = match std::fs::read_to_string(directory.join(JAVA_TEST_BUNDLE_LIST)) {
        Ok(list) => list,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "读取 Java Test 扩展包清单失败 {}：{error}",
                directory.join(JAVA_TEST_BUNDLE_LIST).display()
            ));
        }
    };
    let extensions = directory.join("extensions");
    let mut bundles = Vec::new();
    for name in list.lines().map(str::trim).filter(|name| !name.is_empty()) {
        if name.contains(['/', '\\']) || name == ".." {
            return Err(format!(
                "Java Test 扩展包清单里出现的是路径而不是文件名：{name}"
            ));
        }
        let bundle = extensions.join(name);
        if !bundle.is_file() {
            return Err(format!(
                "Java Test 扩展包 {name} 不在 {} 下",
                extensions.display()
            ));
        }
        bundles.push(bundle);
    }
    Ok((!bundles.is_empty()).then_some(bundles))
}

fn installation_roots(executable: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    push_installation_root(&mut roots, executable);
    if let Ok(canonical) = std::fs::canonicalize(executable) {
        push_installation_root(&mut roots, &canonical);
    }
    roots
}

fn push_installation_root(roots: &mut Vec<PathBuf>, executable: &Path) {
    let Some(parent) = executable.parent() else {
        return;
    };
    if parent
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("bin"))
    {
        if let Some(root) = parent.parent() {
            roots.push(root.to_path_buf());
        }
    }
    roots.push(parent.to_path_buf());
}

fn version_for_root(root: &Path) -> Result<Option<String>, String> {
    let manifest_path = root.join("manifest.json");
    match std::fs::read(&manifest_path) {
        Ok(data) => {
            let manifest: serde_json::Value = serde_json::from_slice(&data)
                .map_err(|error| format!("JDTLS 清单无效 {}：{error}", manifest_path.display()))?;
            let version = manifest
                .get("version")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|version| !version.is_empty())
                .ok_or_else(|| format!("JDTLS 清单没有有效版本号 {}。", manifest_path.display()))?;
            return Ok(Some(version.to_string()));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "读取 JDTLS 清单失败 {}：{error}",
                manifest_path.display()
            ));
        }
    }

    let plugins_path = root.join("plugins");
    let plugins = match std::fs::read_dir(&plugins_path) {
        Ok(plugins) => plugins,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "无法枚举 JDTLS 插件目录 {}：{error}",
                plugins_path.display()
            ));
        }
    };
    let mut names = plugins
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<Vec<_>>();
    names.sort();
    Ok(names.into_iter().find_map(|name| {
        let suffix = name
            .strip_prefix(CORE_PLUGIN_PREFIX)?
            .strip_suffix(".jar")?;
        let version = suffix.split('.').take(3).collect::<Vec<_>>().join(".");
        (!version.is_empty()).then_some(version)
    }))
}

fn find_executable(workspace_root: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    for root in bundled_roots() {
        push_root(&mut candidates, &root);
    }
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            push_names(&mut candidates, &directory);
        }
    }
    for root in search_roots(workspace_root) {
        push_root(&mut candidates, &root);
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn bundled_roots() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("LanguageServers").join("jdtls"));
            let mut cursor = exe_dir.to_path_buf();
            for _ in 0..MAX_EXE_WALK_DEPTH {
                let artifact = cursor.join(".artifacts").join("jdtls");
                if !candidates.contains(&artifact) {
                    candidates.push(artifact);
                }
                if !cursor.pop() {
                    break;
                }
            }
        }
    }
    candidates
        .into_iter()
        .filter(|root| is_root(root))
        .collect()
}

fn search_roots(workspace_root: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(home) = std::env::var("JDTLS_HOME") {
        roots.push(PathBuf::from(home));
    }
    if let Ok(root) = std::env::var("LITHE_JDTLS_ROOT") {
        let trimmed = root.trim();
        if !trimmed.is_empty() {
            roots.push(PathBuf::from(trimmed));
        }
    }
    for key in [
        "LOCALAPPDATA",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "XDG_DATA_HOME",
    ] {
        if let Ok(base) = std::env::var(key) {
            let base = PathBuf::from(base);
            roots.push(base.join("jdtls"));
            roots.push(base.join("Eclipse JDT Language Server"));
            roots.push(base.join("Programs").join("jdtls"));
        }
    }
    if let Ok(profile) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let profile = PathBuf::from(profile);
        roots.push(profile.join(".jdtls"));
        roots.push(
            profile
                .join("scoop")
                .join("apps")
                .join("jdtls")
                .join("current"),
        );
        roots.push(profile.join("scoop").join("shims"));
    }
    roots.push(
        workspace_root
            .join(".lithe")
            .join("toolchains")
            .join("jdtls"),
    );
    roots
}

fn is_root(root: &Path) -> bool {
    EXECUTABLE_NAMES
        .iter()
        .any(|name| root.join(name).is_file() || root.join("bin").join(name).is_file())
}

fn push_root(candidates: &mut Vec<PathBuf>, root: &Path) {
    push_names(candidates, root);
    push_names(candidates, &root.join("bin"));
}

fn push_names(candidates: &mut Vec<PathBuf>, directory: &Path) {
    for name in EXECUTABLE_NAMES {
        candidates.push(directory.join(name));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeSource {
    Override,
    ExplicitEnv,
    Bundled,
    JdtlsBundled,
    JavaHome,
    Path,
    InstalledRoot,
}

struct RuntimeInputs {
    explicit: Option<String>,
    java_home: Option<String>,
    path: Option<OsString>,
    bundled: Vec<PathBuf>,
    installed: Vec<PathBuf>,
}

impl RuntimeInputs {
    fn from_process() -> Self {
        Self {
            explicit: std::env::var(JAVA_EXECUTABLE_ENV).ok(),
            java_home: std::env::var(JAVA_HOME_ENV).ok(),
            path: std::env::var_os(PATH_ENV),
            bundled: bundled_jdk_roots(),
            installed: installed_jdk_roots(),
        }
    }
}

fn runtime_candidates(
    jdtls_root: &Path,
    inputs: &RuntimeInputs,
    overridden: Option<&Path>,
) -> Vec<(RuntimeSource, PathBuf)> {
    let mut candidates: Vec<(RuntimeSource, PathBuf)> = Vec::new();
    if let Some(overridden) = overridden {
        candidates.push((RuntimeSource::Override, overridden.to_path_buf()));
    }
    if let Some(explicit) = non_empty(inputs.explicit.as_deref()) {
        candidates.push((RuntimeSource::ExplicitEnv, PathBuf::from(explicit)));
    }
    candidates.extend(
        inputs
            .bundled
            .iter()
            .cloned()
            .map(|path| (RuntimeSource::Bundled, path)),
    );
    candidates.push((RuntimeSource::JdtlsBundled, jdtls_root.join("jre")));
    if let Some(home) = non_empty(inputs.java_home.as_deref()) {
        candidates.push((RuntimeSource::JavaHome, PathBuf::from(home)));
    }
    if let Some(path) = inputs.path.as_deref() {
        for directory in std::env::split_paths(path) {
            candidates.push((RuntimeSource::Path, directory));
        }
    }
    candidates.extend(
        inputs
            .installed
            .iter()
            .cloned()
            .map(|path| (RuntimeSource::InstalledRoot, path)),
    );
    candidates
}

enum OverrideOutcome<'a> {
    Absent {
        runtime: Option<&'a JavaRuntime>,
    },
    Used {
        path: &'a Path,
        runtime: &'a JavaRuntime,
    },
    Rejected {
        path: &'a Path,
        reason: &'a str,
        runtime: Option<&'a JavaRuntime>,
    },
}

fn toolchain_override_line(outcome: OverrideOutcome<'_>) -> String {
    match outcome {
        OverrideOutcome::Absent {
            runtime: Some(runtime),
        } => format!(
            "S1_JAVA_TOOLCHAIN override=- state=absent result=automatic java={} javaVersion={}",
            runtime.executable.display(),
            runtime.version
        ),
        OverrideOutcome::Absent { runtime: None } => {
            "S1_JAVA_TOOLCHAIN override=- state=absent result=failed".to_string()
        }
        OverrideOutcome::Used { path, runtime } => format!(
            "S1_JAVA_TOOLCHAIN override={} state=used result=used java={} javaVersion={}",
            path.display(),
            runtime.executable.display(),
            runtime.version
        ),
        OverrideOutcome::Rejected {
            path,
            reason,
            runtime,
        } => match runtime {
            Some(runtime) => format!(
                "S1_JAVA_TOOLCHAIN override={} state=invalid fallback=automatic result=automatic java={} javaVersion={} reason={reason}",
                path.display(),
                runtime.executable.display(),
                runtime.version
            ),
            None => format!(
                "S1_JAVA_TOOLCHAIN override={} state=invalid fallback=automatic result=failed reason={reason}",
                path.display()
            ),
        },
    }
}

pub(crate) fn resolve_runtime(jdtls_root: &Path) -> Result<JavaRuntime, String> {
    resolve_runtime_in(
        jdtls_root,
        crate::toolchain::java_home_override().as_deref(),
        &RuntimeInputs::from_process(),
    )
}

fn resolve_runtime_in(
    jdtls_root: &Path,
    overridden: Option<&Path>,
    inputs: &RuntimeInputs,
) -> Result<JavaRuntime, String> {
    let mut rejected = Vec::new();
    let mut override_rejection: Option<String> = None;

    for (source, candidate) in runtime_candidates(jdtls_root, inputs, overridden) {
        let Some(executable) = java_executable_in(&candidate, source) else {
            if source == RuntimeSource::Override {
                override_rejection = Some(format!("找不到 {}", java_executable_name()));
            }
            continue;
        };
        match probe_java(&executable) {
            Ok(version) => {
                let runtime = JavaRuntime {
                    home: executable
                        .parent()
                        .and_then(Path::parent)
                        .unwrap_or_else(|| Path::new(""))
                        .to_path_buf(),
                    executable,
                    version,
                };
                let outcome = match (overridden, source == RuntimeSource::Override) {
                    (Some(path), true) => OverrideOutcome::Used {
                        path,
                        runtime: &runtime,
                    },
                    (Some(path), false) => OverrideOutcome::Rejected {
                        path,
                        reason: override_rejection.as_deref().unwrap_or("未通过判据"),
                        runtime: Some(&runtime),
                    },
                    (None, _) => OverrideOutcome::Absent {
                        runtime: Some(&runtime),
                    },
                };
                println!("{}", toolchain_override_line(outcome));
                return Ok(runtime);
            }
            Err(error) => {
                if source == RuntimeSource::Override {
                    override_rejection = Some(error.clone());
                }
                rejected.push(format!("{}（{error}）", executable.display()));
            }
        }
    }

    println!(
        "{}",
        toolchain_override_line(match overridden {
            Some(path) => OverrideOutcome::Rejected {
                path,
                reason: override_rejection.as_deref().unwrap_or("未通过判据"),
                runtime: None,
            },
            None => OverrideOutcome::Absent { runtime: None },
        })
    );
    Err(format!(
        "未找到 Java {MINIMUM_JAVA_MAJOR} 或更新版本的 JDK，JDT LS 无法启动。\
         在设置「项目 · JDK 与 Maven」里指定 JDK 主目录，或用 {JAVA_EXECUTABLE_ENV} 指定 \
         java 可执行文件，或安装 JDK {MINIMUM_JAVA_MAJOR}+。\
         已排除：{}",
        rejected.join("；")
    ))
}

fn java_executable_in(candidate: &Path, source: RuntimeSource) -> Option<PathBuf> {
    if candidate.is_file() {
        return Some(candidate.to_path_buf());
    }
    let names: &[&str] = if source == RuntimeSource::Override {
        &[java_executable_name(), "java"]
    } else {
        &[java_executable_name()]
    };
    names
        .iter()
        .map(|name| candidate.join("bin").join(name))
        .find(|path| path.is_file())
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn bundled_jdk_roots() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("LanguageServers").join("jdk"));
            let mut cursor = exe_dir.to_path_buf();
            for _ in 0..MAX_EXE_WALK_DEPTH {
                let artifact = cursor.join(".artifacts").join("jdk");
                if !candidates.contains(&artifact) {
                    candidates.push(artifact);
                }
                if !cursor.pop() {
                    break;
                }
            }
        }
    }
    candidates
}

fn installed_jdk_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(profile) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let profile = PathBuf::from(profile);
        roots.push(profile.join(".jdks"));
        roots.push(profile.join(".sdkman").join("candidates").join("java"));
        roots.push(profile.join("scoop").join("apps"));
    }
    for key in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Ok(base) = std::env::var(key) {
            let base = PathBuf::from(base);
            for vendor in [
                "Java",
                "Eclipse Adoptium",
                "Microsoft",
                "Amazon Corretto",
                "Zulu",
                "BellSoft",
                "Programs",
            ] {
                roots.push(base.join(vendor));
            }
        }
    }
    if let Ok(base) = std::env::var("ProgramData") {
        roots.push(PathBuf::from(base).join("java"));
    }
    if let Ok(base) = std::env::var("XDG_DATA_HOME") {
        roots.push(PathBuf::from(base).join("jdks"));
    }
    let mut candidates = Vec::new();
    for root in roots {
        candidates.push(root.clone());
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.filter_map(Result::ok) {
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    candidates.push(entry.path());
                }
            }
        }
    }
    candidates
}

fn probe_java(executable: &Path) -> Result<String, String> {
    let output = Command::new(executable)
        .arg("-version")
        .output()
        .map_err(|error| format!("无法执行：{error}"))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let version = parse_release(&text).ok_or_else(|| "读不出版本号".to_string())?;
    if major_version(&version) < MINIMUM_JAVA_MAJOR {
        return Err(format!("版本 {version} 低于 {MINIMUM_JAVA_MAJOR}"));
    }
    Ok(version)
}

fn parse_release(text: &str) -> Option<String> {
    let start = text.find('"')? + 1;
    let rest = &text[start..];
    let end = rest.find('"')?;
    let version = rest[..end].trim();
    (!version.is_empty()).then(|| version.to_string())
}

fn major_version(version: &str) -> u64 {
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

pub(crate) fn manifest_version() -> Option<String> {
    let manifest: serde_json::Value = serde_json::from_str(BUNDLED_MANIFEST).ok()?;
    manifest
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(str::to_string)
}

pub(crate) fn normalize_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    if let Some(network_path) = path.strip_prefix(r"\\?\UNC\") {
        return format!("//{}", network_path.replace('\\', "/"));
    }
    path.strip_prefix(r"\\?\")
        .unwrap_or(path.as_ref())
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_version_reads_legacy_and_modern_releases() {
        assert_eq!(major_version("1.8.0_402"), 8);
        assert_eq!(major_version("21.0.2"), 21);
        assert_eq!(major_version("25"), 25);
        assert_eq!(major_version("21.0.2+13-LTS"), 21);
    }

    #[test]
    fn parse_release_takes_the_quoted_version() {
        assert_eq!(
            parse_release("openjdk version \"21.0.2\" 2024-01-16\n").as_deref(),
            Some("21.0.2")
        );
        assert_eq!(
            parse_release("java version \"1.8.0_402\"\n").as_deref(),
            Some("1.8.0_402")
        );
        assert_eq!(parse_release("no version here"), None);
    }

    #[test]
    fn installation_roots_cover_bin_and_root_layouts() {
        let bin = Path::new("C:/tools/jdtls/bin/jdtls.bat");
        let roots = installation_roots(bin);
        assert!(roots.contains(&PathBuf::from("C:/tools/jdtls")));
        assert!(roots.contains(&PathBuf::from("C:/tools/jdtls/bin")));

        let root = Path::new("/opt/jdtls/jdtls");
        assert!(installation_roots(root).contains(&PathBuf::from("/opt/jdtls")));
    }

    #[test]
    fn bundled_manifest_exposes_a_version() {
        assert_eq!(manifest_version().as_deref(), Some("1.61.0"));
    }

    #[test]
    fn plugin_name_yields_the_release_version() {
        let suffix = "1.61.0.202609031315";
        assert_eq!(
            suffix.split('.').take(3).collect::<Vec<_>>().join("."),
            "1.61.0"
        );
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("系统时钟应在 Unix 纪元之后")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "lithe-gpui-jdtls-{tag}-{}-{stamp}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("临时目录");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn with_fake_java(&self, name: &str) -> PathBuf {
            let bin = self.0.join("bin");
            std::fs::create_dir_all(&bin).expect("bin 目录");
            let java = bin.join(name);
            std::fs::write(&java, b"not-a-real-java").expect("伪 java");
            java
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn inputs_with_every_source() -> RuntimeInputs {
        RuntimeInputs {
            explicit: Some(" jdk-from-env ".to_string()),
            java_home: Some("jdk-from-java-home".to_string()),
            path: Some(
                std::env::join_paths([Path::new("jdk-on-path-a"), Path::new("jdk-on-path-b")])
                    .expect("join_paths"),
            ),
            bundled: vec![PathBuf::from("jdk-bundled")],
            installed: vec![PathBuf::from("jdk-installed")],
        }
    }

    #[test]
    fn the_override_comes_first_and_the_rest_of_the_order_is_unchanged() {
        let inputs = inputs_with_every_source();
        let candidates = runtime_candidates(
            Path::new("jdtls-root"),
            &inputs,
            Some(Path::new("jdk-override")),
        );
        let order: Vec<RuntimeSource> = candidates.iter().map(|(source, _)| *source).collect();
        assert_eq!(
            order,
            vec![
                RuntimeSource::Override,
                RuntimeSource::ExplicitEnv,
                RuntimeSource::Bundled,
                RuntimeSource::JdtlsBundled,
                RuntimeSource::JavaHome,
                RuntimeSource::Path,
                RuntimeSource::Path,
                RuntimeSource::InstalledRoot,
            ]
        );
        assert_eq!(candidates[0].1, PathBuf::from("jdk-override"));
        assert_eq!(candidates[1].1, PathBuf::from("jdk-from-env"));
        assert_eq!(candidates[2].1, PathBuf::from("jdk-bundled"));
        assert_eq!(candidates[3].1, PathBuf::from("jdtls-root").join("jre"));
        assert_eq!(candidates[4].1, PathBuf::from("jdk-from-java-home"));
        assert_eq!(candidates[5].1, PathBuf::from("jdk-on-path-a"));
        assert_eq!(candidates[7].1, PathBuf::from("jdk-installed"));
    }

    #[test]
    fn without_an_override_the_first_judgement_is_still_the_explicit_env() {
        let candidates =
            runtime_candidates(Path::new("jdtls-root"), &inputs_with_every_source(), None);
        assert_eq!(candidates[0].0, RuntimeSource::ExplicitEnv);

        let blank = RuntimeInputs {
            explicit: Some("   ".to_string()),
            java_home: Some(String::new()),
            path: None,
            bundled: Vec::new(),
            installed: Vec::new(),
        };
        let candidates = runtime_candidates(Path::new("jdtls-root"), &blank, None);
        assert_eq!(
            candidates
                .iter()
                .map(|(source, _)| *source)
                .collect::<Vec<_>>(),
            vec![RuntimeSource::JdtlsBundled],
            "空白的 LITHE_JDTLS_JAVA / JAVA_HOME 不能被当成候选"
        );
    }

    #[test]
    fn the_override_accepts_a_home_or_an_executable() {
        let home = TempDir::new("override-shapes");
        let java = home.with_fake_java("java");

        assert_eq!(
            java_executable_in(home.path(), RuntimeSource::Override).as_deref(),
            Some(java.as_path()),
            "覆盖值是 JDK 主目录时必须找到 bin/java"
        );
        assert_eq!(
            java_executable_in(&java, RuntimeSource::Override).as_deref(),
            Some(java.as_path()),
            "覆盖值直接是 java 可执行文件时必须原样采用"
        );
        assert_eq!(
            java_executable_in(home.path(), RuntimeSource::Bundled),
            None,
            "自动发现那一路保持原样的候选名（不新认无扩展名的 java）"
        );
    }

    #[test]
    fn an_unusable_override_falls_back_instead_of_stopping_the_chain() {
        let override_dir = TempDir::new("override");
        let override_java = override_dir.with_fake_java("java");
        let fallback_dir = TempDir::new("fallback");
        let fallback_java = fallback_dir.with_fake_java(java_executable_name());

        let inputs = RuntimeInputs {
            explicit: None,
            java_home: None,
            path: None,
            bundled: Vec::new(),
            installed: vec![fallback_dir.path().to_path_buf()],
        };
        let error = resolve_runtime_in(
            Path::new("no-such-jdtls-root"),
            Some(override_dir.path()),
            &inputs,
        )
        .expect_err("两个候选都不可执行时必须失败");
        assert!(error.contains("未找到 Java"), "{error}");
        assert!(
            error.contains(&override_java.display().to_string()),
            "覆盖值必须被试过并记进'已排除'：{error}"
        );
        assert!(
            error.contains(&fallback_java.display().to_string()),
            "覆盖值失败后必须继续走自动发现的候选（降级）：{error}"
        );
    }

    #[test]
    fn the_toolchain_diagnostic_names_the_state_and_the_fallback() {
        let runtime = JavaRuntime {
            home: PathBuf::from("jdk-21"),
            executable: PathBuf::from("jdk-21").join("bin").join("java.exe"),
            version: "21.0.8".to_string(),
        };

        let absent = toolchain_override_line(OverrideOutcome::Absent {
            runtime: Some(&runtime),
        });
        assert!(
            absent.contains("override=- state=absent result=automatic"),
            "{absent}"
        );
        assert!(absent.contains("javaVersion=21.0.8"), "{absent}");

        let used = toolchain_override_line(OverrideOutcome::Used {
            path: Path::new("chosen-jdk"),
            runtime: &runtime,
        });
        assert!(
            used.contains("override=chosen-jdk state=used result=used"),
            "{used}"
        );
        assert!(used.contains("javaVersion=21.0.8"), "{used}");

        let fell_back = toolchain_override_line(OverrideOutcome::Rejected {
            path: Path::new("bad-jdk"),
            reason: "版本 1.8.0_221 低于 21",
            runtime: Some(&runtime),
        });
        assert!(
            fell_back
                .contains("override=bad-jdk state=invalid fallback=automatic result=automatic"),
            "{fell_back}"
        );
        assert!(
            fell_back.ends_with("reason=版本 1.8.0_221 低于 21"),
            "{fell_back}"
        );

        let failed = toolchain_override_line(OverrideOutcome::Rejected {
            path: Path::new("bad-jdk"),
            reason: "找不到 java.exe",
            runtime: None,
        });
        assert!(
            failed.contains("state=invalid fallback=automatic result=failed"),
            "{failed}"
        );

        assert_eq!(
            toolchain_override_line(OverrideOutcome::Absent { runtime: None }),
            "S1_JAVA_TOOLCHAIN override=- state=absent result=failed"
        );
    }

    #[test]
    fn real_java_home_override_beats_the_environment() {
        if std::env::var_os("LITHE_GPUI_JDTLS_SMOKE").is_none_or(|value| value.is_empty()) {
            return;
        }
        let Some(home) = std::env::var("LITHE_GPUI_JDTLS_SMOKE_JDK_HOME")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
        else {
            return;
        };
        let inputs = RuntimeInputs {
            explicit: Some("no-such-jdk-from-env".to_string()),
            java_home: Some("no-such-jdk-from-java-home".to_string()),
            path: None,
            bundled: vec![PathBuf::from("no-such-bundled-jdk")],
            installed: vec![PathBuf::from("no-such-installed-jdk")],
        };
        let runtime = resolve_runtime_in(Path::new("no-such-jdtls-root"), Some(&home), &inputs)
            .expect("覆盖值是可用 JDK 时必须采用它");
        assert_eq!(
            runtime.home, home,
            "覆盖值必须压过 LITHE_JDTLS_JAVA / JAVA_HOME / 捆绑 / PATH"
        );
        assert!(
            major_version(&runtime.version) >= MINIMUM_JAVA_MAJOR,
            "{}",
            runtime.version
        );
    }
}
