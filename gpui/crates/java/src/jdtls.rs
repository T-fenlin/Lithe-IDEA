//! JDT LS 载荷的**平台发现**：可执行文件、直启资源、运行用 JDK、版本。
//!
//! 逐条移植 Windows host 的 `windows/tauri/src-tauri/src/lsp.rs`
//! （`find_jdtls_executable` / `jdtls_candidates` / `jdtls_search_roots` /
//! `resolve_jdtls_launch_resources` / `jdtls_version_for_executable` / `select_bundled_jdtls_root` /
//! `resolve_java_home`）。**判据不重新设计**：两边各有一套发现规则就会得到两套 JDTLS 安装，
//! 而契约（`shared/contracts/rust-core-api.md:1231-1237`）把这件事明确划给平台适配器：
//!
//! > Platform adapters own filesystem discovery and validate that packaged JDT LS contains the
//! > Equinox launcher, platform configuration directory, Lombok agent, Java Debug Server, and
//! > bundled Java. … They do not construct JVM commands.
//!
//! 最后一句是这里的硬边界：**不拼 JVM 参数**。`jdtlsLaunchResources` 交给
//! `lsp.startServer` 之后，JVM 命令行由 Rust Core 构造（`rust/lithe-core/src/lsp/languages/jdt.rs`
//! 的 `direct_java_arguments`）。

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 可执行文件候选名，顺序即优先级（Windows 的 `JDTLS_EXECUTABLE_NAMES` 逐字照搬）。
const EXECUTABLE_NAMES: &[&str] = &["jdtls.bat", "jdtls.cmd", "jdtls.exe", "jdtls"];
/// 从当前 exe 向上找捆绑载荷的层数上限（Windows 的 `MAX_CURRENT_EXE_JDTLS_WALK_DEPTH`）。
const MAX_EXE_WALK_DEPTH: usize = 12;
/// JDT LS 核心插件名前缀，它的版本号后缀是"没有 manifest.json 时"的版本来源。
const CORE_PLUGIN_PREFIX: &str = "org.eclipse.jdt.ls.core_";
/// Equinox 启动 JAR 的插件名前缀。
const EQUINOX_LAUNCHER_PREFIX: &str = "org.eclipse.equinox.launcher_";
/// Java Debug Server 包名前缀。
const JAVA_DEBUG_BUNDLE_PREFIX: &str = "com.microsoft.java.debug.plugin-";
/// Java Test 扩展包清单（`prepare-jdtls` 按扩展声明的顺序逐行写出）。
const JAVA_TEST_BUNDLE_LIST: &str = "extensions.txt";
/// 捆绑载荷的嵌入清单（与 Windows host 同一份文件）。
const BUNDLED_MANIFEST: &str = include_str!("../../../../third_party/jdtls/manifest.json");

/// 显式指定运行 JDT LS 的 `java` 可执行文件（验证链路与"装在非常规位置"的用户用）。
///
/// 与仓库既有的 `LITHE_JDTLS_ROOT` / `LITHE_GPUI_SETTINGS_FILE` 同一口径：**只覆盖推导**，
/// 不是常规入口。
pub(crate) const JAVA_EXECUTABLE_ENV: &str = "LITHE_JDTLS_JAVA";

/// 一套可以直接交给 `lsp.startServer` 的 JDT LS 安装。
#[derive(Clone, Debug)]
pub(crate) struct JdtlsInstallation {
    /// 启动器脚本（Windows 上是 `bin/jdtls.bat`）。结构化直启下 Core 不执行它，
    /// 但 `lsp.startServer` 的 `executablePath` 是必填字段，且它也是"用户装在哪"的证据。
    pub(crate) executable: PathBuf,
    /// Equinox 启动 JAR（`plugins/org.eclipse.equinox.launcher_*.jar`）。
    pub(crate) launcher_jar_path: PathBuf,
    /// 与当前平台匹配的 Eclipse 配置目录（`config_win` / `config_mac[_arm]` / `config_linux`）。
    pub(crate) configuration_directory: PathBuf,
    /// Lombok agent。
    pub(crate) lombok_agent_path: PathBuf,
    /// Java Debug Server 包（旧字段，排在扩展包之前加载）。
    pub(crate) java_debug_bundle_path: Option<PathBuf>,
    /// Java Test 扩展包，**按扩展声明的顺序**（OSGi 解析顺序有意义）。
    pub(crate) java_extension_bundle_paths: Vec<PathBuf>,
    /// 送进 `java.jdtWorkspaceFingerprint` 的 `jdtlsVersion`。
    pub(crate) version: String,
}

/// 跑 JDT LS 用的 JDK。
#[derive(Clone, Debug)]
pub(crate) struct JavaRuntime {
    /// JDK 安装根（`JAVA_HOME` 口径），送给 `lsp.startServer` 的 `environment.JAVA_HOME`。
    pub(crate) home: PathBuf,
    /// `bin/java(.exe)`，送给 `runtimeExecutablePath`。
    pub(crate) executable: PathBuf,
    /// `java -version` 报出来的版本，例如 `21.0.2`。
    pub(crate) version: String,
}

/// JDT LS 要求的最低 Java 主版本（`third_party/jdtls/manifest.json` 的 `minimumJavaVersion`）。
const MINIMUM_JAVA_MAJOR: u64 = 21;

/// 当前平台的配置目录名（Windows host 把 `config_win` 写死，这里补上另两个平台）。
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

/// 平台上的 `java` 可执行文件名。
fn java_executable_name() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

/// 发现一套 JDT LS 安装。`workspace_root` 只用来多看一眼项目本地的
/// `.lithe/toolchains/jdtls`（Windows 的同名规则）。
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
        // 版本回落到嵌入清单：安装根里既没有 `manifest.json` 也解析不出核心插件名时，
        // 用仓库里那一份（与 Windows `jdtls_version_for_executable` 的兜底同源）。
        // 它只影响缓存键，不会让一个版本号读不出来的安装完全不可用。
        version: version_for_root(&root)?
            .or_else(manifest_version)
            .ok_or_else(|| format!("读不出 JDTLS 版本：{}", root.display()))?,
    })
}

/// 一套校验通过的直启资源。
struct LaunchResources {
    launcher_jar_path: PathBuf,
    configuration_directory: PathBuf,
    lombok_agent_path: PathBuf,
    java_debug_bundle_path: Option<PathBuf>,
    java_extension_bundle_paths: Vec<PathBuf>,
}

/// 在 `root` 下按 Windows 的判据取齐直启资源；缺件返回 `Ok(None)`（继续看下一个 root），
/// 目录不可读之类的 I/O 失败返回 `Err`（调用方不该把它当成"换一个 root 就好"）。
fn resources_for_root(root: &Path) -> Result<Option<LaunchResources>, String> {
    let Some(launcher_jar_path) = first_regular_file(&root.join("plugins"), EQUINOX_LAUNCHER_PREFIX)
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

/// 目录下第一个 `prefix*.suffix` 常规文件（文件名排序取第一个，确定性）。
///
/// 目录不存在算"没有"，不算错误 —— 候选 root 里本来就有不含 `java-debug` 的目录
/// （例如 JDTLS 安装根本身）。
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

/// Java Test 扩展包，按 `extensions.txt` 的**声明顺序**返回。
///
/// 列了但缺失的包算错误而不是跳过：只加载一部分会让 OSGi 对整个扩展解析失败
/// （Windows host 的同名注释，`lsp.rs:294-296`）。
fn java_test_extension_bundles(root: &Path) -> Result<Option<Vec<PathBuf>>, String> {
    let directory = root.join("java-test");
    let list = match std::fs::read_to_string(directory.join(JAVA_TEST_BUNDLE_LIST)) {
        Ok(list) => list,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "读取 Java Test 扩展包清单失败 {}：{error}",
                directory.join(JAVA_TEST_BUNDLE_LIST).display()
            ))
        }
    };
    let extensions = directory.join("extensions");
    let mut bundles = Vec::new();
    for name in list.lines().map(str::trim).filter(|name| !name.is_empty()) {
        // 清单是**文件名**清单，不是路径清单：拒绝任何带分隔符的项，避免目录穿越。
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

/// 可执行文件的**安装根**候选：父目录是 `bin` 时根是它的父目录，否则就是父目录本身；
/// 再补一条 `canonicalize` 之后的（符号链接 / 大小写不同的路径都覆盖）。
///
/// ⚠️ **顺序有意义，不要排序**：`canonicalize` 在 Windows 上返回 `\\?\D:\...` 形式的
/// verbatim 路径，而 `\`（0x5C）排在 `D`（0x44）之前 —— 一旦排序，verbatim 变体就会
/// 排在前面并被选中，于是所有资源路径都带上 `\\?\` 前缀，JVM 直接找不到启动 JAR
/// （实测：`initialize` 立刻失败、进程秒退，stderr 里还有 `Unable to add \\?\...lombok.jar`）。
/// Windows host 同样是"先原文、后 canonical"，只是它还有 [`normalize_path`] 兜底。
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

/// JDT LS 版本：先看安装根的 `manifest.json`，没有就从核心插件名解析；
/// 两者都没有时返回 `None`，由调用方回落到嵌入清单。
fn version_for_root(root: &Path) -> Result<Option<String>, String> {
    let manifest_path = root.join("manifest.json");
    match std::fs::read(&manifest_path) {
        Ok(data) => {
            let manifest: serde_json::Value = serde_json::from_slice(&data).map_err(|error| {
                format!("JDTLS 清单无效 {}：{error}", manifest_path.display())
            })?;
            let version = manifest
                .get("version")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|version| !version.is_empty())
                .ok_or_else(|| {
                    format!("JDTLS 清单没有有效版本号 {}。", manifest_path.display())
                })?;
            return Ok(Some(version.to_string()));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "读取 JDTLS 清单失败 {}：{error}",
                manifest_path.display()
            ))
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
            ))
        }
    };
    let mut names = plugins
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<Vec<_>>();
    names.sort();
    // `org.eclipse.jdt.ls.core_1.61.0.202609031315.jar` → `1.61.0`
    // （Windows host 取前三个点分段，理由同源：后一段是构建时间戳，不属于发布版本）。
    Ok(names.into_iter().find_map(|name| {
        let suffix = name
            .strip_prefix(CORE_PLUGIN_PREFIX)?
            .strip_suffix(".jar")?;
        let version = suffix.split('.').take(3).collect::<Vec<_>>().join(".");
        (!version.is_empty()).then_some(version)
    }))
}

/// 找 `jdtls` 可执行文件：捆绑载荷优先，然后 PATH，最后各类外部安装位置。
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

/// 仓库/安装目录自带的 JDTLS：资源目录旁，或从当前 exe 向上找到 `.artifacts/jdtls`。
///
/// 这一条是 gpui 侧对 Windows `select_bundled_jdtls_root` 的等价物：那边从 Tauri
/// `resource_dir()` 起，这里从 `current_exe` 起（开发时 `cargo run` 也能找到）。
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
    candidates.into_iter().filter(|root| is_root(root)).collect()
}

/// 外部安装位置（Windows `jdtls_search_roots` 的等价物，去掉 Tauri 专有项）。
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
    for key in ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)", "XDG_DATA_HOME"] {
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
        roots.push(profile.join("scoop").join("apps").join("jdtls").join("current"));
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

// ---------------------------------------------------------------------------
// 运行 JDT LS 的 JDK
// ---------------------------------------------------------------------------

/// 找跑 JDT LS 的 JDK（**必须 ≥ 21**，`third_party/jdtls/manifest.json` 的
/// `minimumJavaVersion`，实测 JDT LS 1.61 在 1.8 上直接起不来）。
///
/// 顺序（先到先用，第一个能跑且主版本够的胜出）：
///
/// 1. `LITHE_JDTLS_JAVA`（显式覆盖；本机 PATH 上的 java 是 1.8，验证链路靠它）；
/// 2. 捆绑位置：exe 旁的 `LanguageServers/jdk`、向上走的 `.artifacts/jdk`、
///    JDTLS 安装根下的 `jre`（官方载荷自带）；
/// 3. `JAVA_HOME`；
/// 4. PATH 上的 `java`；
/// 5. 常见安装根（`~/.jdks`、`%LOCALAPPDATA%\Programs\...`、`%ProgramFiles%\...`、scoop）。
///
/// 第 5 条不是"猜路径"：它和 Windows frontend 的 JDK 候选列表是同一类**安装位置枚举**，
/// 每个候选都要过 `java -version` 的版本闸门，不满足就继续。
pub(crate) fn resolve_runtime(jdtls_root: &Path) -> Result<JavaRuntime, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(explicit) = std::env::var(JAVA_EXECUTABLE_ENV) {
        let explicit = explicit.trim();
        if !explicit.is_empty() {
            candidates.push(PathBuf::from(explicit));
        }
    }
    candidates.extend(bundled_jdk_roots());
    candidates.push(jdtls_root.join("jre"));
    if let Ok(home) = std::env::var("JAVA_HOME") {
        if !home.trim().is_empty() {
            candidates.push(PathBuf::from(home));
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            candidates.push(directory.clone());
        }
    }
    candidates.extend(installed_jdk_roots());

    let mut rejected = Vec::new();
    for candidate in candidates {
        let executable = if candidate.is_file() {
            candidate
        } else {
            let java = candidate.join("bin").join(java_executable_name());
            if !java.is_file() {
                continue;
            }
            java
        };
        match probe_java(&executable) {
            Ok(version) => {
                return Ok(JavaRuntime {
                    home: executable
                        .parent()
                        .and_then(Path::parent)
                        .unwrap_or_else(|| Path::new(""))
                        .to_path_buf(),
                    executable,
                    version,
                })
            }
            Err(error) => rejected.push(format!("{}（{error}）", executable.display())),
        }
    }

    Err(format!(
        "未找到 Java {MINIMUM_JAVA_MAJOR} 或更新版本的 JDK，JDT LS 无法启动。\
         用 {JAVA_EXECUTABLE_ENV} 指定 java 可执行文件，或安装 JDK {MINIMUM_JAVA_MAJOR}+。\
         已排除：{}",
        rejected.join("；")
    ))
}

/// 捆绑 JDK：exe 旁与向上走的 `.artifacts/jdk`（Windows `bundled_jdk_root` 的等价物）。
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

/// 常见 JDK 安装根（每个下面按名字枚举 `*/bin/java`）。
fn installed_jdk_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(profile) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let profile = PathBuf::from(profile);
        // IntelliJ 自动下载的 JDK 与 SDKMAN 都在这里。
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
        // 常见的是 `<vendor>/<jdk-21.0.2+13>` 这种一层子目录。
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

/// 跑 `java -version`，解析主版本；低于 `MINIMUM_JAVA_MAJOR` 时返回错误说明。
fn probe_java(executable: &Path) -> Result<String, String> {
    let output = Command::new(executable)
        .arg("-version")
        .output()
        .map_err(|error| format!("无法执行：{error}"))?;
    // `java -version` 一律写 stderr（JDK 8 起都是），stdout 只有极少数发行版会用。
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

/// 从 `java -version` 的输出里取第一个引号里的版本串。
fn parse_release(text: &str) -> Option<String> {
    let start = text.find('"')? + 1;
    let rest = &text[start..];
    let end = rest.find('"')?;
    let version = rest[..end].trim();
    (!version.is_empty()).then(|| version.to_string())
}

/// `1.8.0_402` → 8、`21.0.2` → 21、`25` → 25。
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

/// 捆绑载荷清单里的版本，`jdtlsVersion` 的最后兜底（与 Windows host 同源）。
pub(crate) fn manifest_version() -> Option<String> {
    let manifest: serde_json::Value = serde_json::from_str(BUNDLED_MANIFEST).ok()?;
    manifest
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(str::to_string)
}

/// 送进 JSON 的路径文本。逐字移植 Windows host 的 `normalize_path`
/// （`windows/tauri/src-tauri/src/lsp.rs:658-669`）：
///
/// 1. `\\?\UNC\server\share` → `//server/share`；
/// 2. 去掉 verbatim 的 `\\?\` 前缀；
/// 3. `\` → `/`。
///
/// 第 1、2 条是**必须**的：`std::fs::canonicalize` 在 Windows 上给出 verbatim 路径，
/// 而 JVM 不认 `\\?\`（`-jar` 会直接报错、`-javaagent` 会 `AddToSystemClassLoaderSearch`
/// 失败）；第 3 条是为了与 Windows 的发送口径逐字一致（Java 在 Windows 上吃 `/`）。
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

    /// 安装根推导：`.../bin/jdtls.bat` 的根是它的父目录，根目录下的 `jdtls` 则是父目录本身。
    #[test]
    fn installation_roots_cover_bin_and_root_layouts() {
        let bin = Path::new("C:/tools/jdtls/bin/jdtls.bat");
        let roots = installation_roots(bin);
        assert!(roots.contains(&PathBuf::from("C:/tools/jdtls")));
        assert!(roots.contains(&PathBuf::from("C:/tools/jdtls/bin")));

        let root = Path::new("/opt/jdtls/jdtls");
        assert!(installation_roots(root).contains(&PathBuf::from("/opt/jdtls")));
    }

    /// 清单版本必须能被读出来：它是 `java.jdtWorkspaceFingerprint` 的输入之一，
    /// 读不出来会让缓存键随载荷升级静默复用（本测试守的是这条）。
    #[test]
    fn bundled_manifest_exposes_a_version() {
        assert_eq!(manifest_version().as_deref(), Some("1.61.0"));
    }

    /// 版本解析：`org.eclipse.jdt.ls.core_1.61.0.202609031315.jar` 取前三个点分段。
    #[test]
    fn plugin_name_yields_the_release_version() {
        let suffix = "1.61.0.202609031315";
        assert_eq!(
            suffix.split('.').take(3).collect::<Vec<_>>().join("."),
            "1.61.0"
        );
    }
}
