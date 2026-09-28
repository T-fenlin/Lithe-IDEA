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

use std::ffi::{OsStr, OsString};
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
const BUNDLED_MANIFEST: &str = include_str!("../../../../../third_party/jdtls/manifest.json");

/// 显式指定运行 JDT LS 的 `java` 可执行文件（验证链路与"装在非常规位置"的用户用）。
///
/// 与仓库既有的 `LITHE_JDTLS_ROOT` / `LITHE_GPUI_SETTINGS_FILE` 同一口径：**只覆盖推导**，
/// 不是常规入口。
pub(crate) const JAVA_EXECUTABLE_ENV: &str = "LITHE_JDTLS_JAVA";

/// JDK 主目录的环境变量名（`resolve_runtime` 的第 5 条判据）。
const JAVA_HOME_ENV: &str = "JAVA_HOME";

/// `PATH` 的环境变量名（第 6 条判据：逐目录找 `java`）。
const PATH_ENV: &str = "PATH";

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

/// 一个 JDK 候选**来自哪一条判据**。枚举顺序即判据顺序（见 [`runtime_candidates`]）。
///
/// 单列成类型而不是"按顺序 push 一串路径"，是因为判据顺序正是这里最容易被顺手改坏的东西
/// （改了它，用户选的 JDK 就可能被某个环境变量盖掉 —— 也就是这次要修的"存了不生效"），
/// 而显式数据让这条顺序可以被单测逐项钉住。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeSource {
    /// 设置页的覆盖值（用户显式选择）。
    Override,
    /// `LITHE_JDTLS_JAVA`。
    ExplicitEnv,
    /// 捆绑 JDK（exe 旁的 `LanguageServers/jdk`、向上走的 `.artifacts/jdk`）。
    Bundled,
    /// JDT LS 安装根自带的 `jre`（官方载荷自带）。
    JdtlsBundled,
    /// `JAVA_HOME`。
    JavaHome,
    /// `PATH` 上的目录。
    Path,
    /// 本机常见安装根（枚举一层子目录）。
    InstalledRoot,
}

/// 一次 JDK 发现要用的**全部外部输入**（环境变量 + 两条枚举结果）。
///
/// 抽成纯数据是为了让判据顺序可以脱离真实环境单测：`runtime_candidates` 拿它当入参，
/// 测试就能构造"`PATH` 上是 1.8、覆盖值是 21"这种组合，而**不用改进程环境变量**
/// —— 改环境变量会让同一个测试二进制里并发跑的用例互相污染。
struct RuntimeInputs {
    /// [`JAVA_EXECUTABLE_ENV`] 的原文。
    explicit: Option<String>,
    /// `JAVA_HOME` 的原文。
    java_home: Option<String>,
    /// `PATH` 的原文（`;` / `:` 分隔，交给 `std::env::split_paths`）。
    path: Option<OsString>,
    /// 捆绑 JDK 的候选（[`bundled_jdk_roots`] 的结果）。
    bundled: Vec<PathBuf>,
    /// 常见安装根下枚举出来的候选（[`installed_jdk_roots`] 的结果）。
    installed: Vec<PathBuf>,
}

impl RuntimeInputs {
    /// 读当前进程的环境变量，并按既有规则枚举捆绑位置与常见安装根。
    ///
    /// 只读环境、只读目录，**不起子进程**（真正的 `java -version` 在候选循环里）。
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

/// 候选顺序（先到先用，第一个能跑且主版本够的胜出）：
///
/// 1. **覆盖值**（设置页 `javaHomePath`，用户显式选择）；
/// 2. `LITHE_JDTLS_JAVA`（显式覆盖；本机 PATH 上的 java 是 1.8，验证链路靠它）；
/// 3. 捆绑位置：exe 旁的 `LanguageServers/jdk`、向上走的 `.artifacts/jdk`；
/// 4. JDT LS 安装根下的 `jre`（官方载荷自带）；
/// 5. `JAVA_HOME`；
/// 6. `PATH` 上的 `java`；
/// 7. 常见安装根（`~/.jdks`、`%LOCALAPPDATA%\Programs\...`、`%ProgramFiles%\...`、scoop）。
///
/// ## 覆盖值为什么排在 `LITHE_JDTLS_JAVA` **之前**
///
/// 那个变量按它自己的文档是"**只覆盖推导**，不是常规入口"（`jdtls.rs:36-40`，
/// 与 `LITHE_JDTLS_ROOT` 同一口径的验证/诊断开关）；而覆盖值是用户在设置页上做的一次
/// **显式选择**，页面把它的「生效值」标成「**已选择**」（`settings/src/project.rs:390-405`
/// 的 `EffectiveToolchain::Resolved { mode: Configured }`）。
/// 让一个验证开关盖掉用户的选择，会让设置页第三次变成"存了不生效"：页面写 21.0.8、
/// 实际起的是环境变量指的那个。两者都是"显式指定"，但产品里的用户选择优先。
///
/// `LITHE_JDTLS_JAVA` 仍然排在**所有自动发现之前**，所以"本机 PATH 上是 1.8、
/// 验证链路靠它指到 21"这条既有用法原样保留（`service.rs:1554-1560`）。
///
/// 第 7 条不是"猜路径"：它和 Windows frontend 的 JDK 候选列表是同一类**安装位置枚举**，
/// 每个候选都要过 `java -version` 的版本闸门，不满足就继续。
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

/// 一次覆盖值判定的结论 —— [`toolchain_override_line`] 的唯一输入。
///
/// 三种情形互斥且穷尽，所以诊断行不需要再看别处的状态。
enum OverrideOutcome<'a> {
    /// 设置里没填（或宿主没登记）覆盖值 ⇒ 走的就是纯自动发现。
    /// `runtime` = 自动发现拿到的那个（`None` = 自动发现也没找到，整条链就此失败）。
    Absent { runtime: Option<&'a JavaRuntime> },
    /// 覆盖值被采用（`path` = 用户填的原文路径，`runtime` = 判定通过的 JDK）。
    Used {
        path: &'a Path,
        runtime: &'a JavaRuntime,
    },
    /// 覆盖值填了但**用不了**（路径不存在 / 没有 `bin/java` / 版本低于 21）：
    /// `reason` 是"为什么"，`runtime` 是**降级之后**自动发现拿到的那个
    /// （`None` = 自动发现也没找到，整条链就此失败）。
    ///
    /// ⚠️ 这条分支**不是启动失败**：调用方必须继续走剩下的候选（见 [`resolve_runtime_in`]）。
    Rejected {
        path: &'a Path,
        reason: &'a str,
        runtime: Option<&'a JavaRuntime>,
    },
}

/// `S1_JAVA_TOOLCHAIN` 那一行的**唯一构造点**（纯函数 ⇒ 降级措辞可脱离真实环境单测）。
///
/// 形状固定、字段顺序固定，便于 `grep 'S1_JAVA_TOOLCHAIN'` 后逐项对照：
///
/// ```text
/// S1_JAVA_TOOLCHAIN override=- state=absent result=automatic java=… javaVersion=21.0.8
/// S1_JAVA_TOOLCHAIN override=<填的路径> state=used result=used java=… javaVersion=21.0.8
/// S1_JAVA_TOOLCHAIN override=<填的路径> state=invalid fallback=automatic result=automatic java=… javaVersion=1.8.0_221 reason=<为什么用不了>
/// S1_JAVA_TOOLCHAIN override=<填的路径> state=invalid fallback=automatic result=failed reason=<为什么用不了>
/// S1_JAVA_TOOLCHAIN override=- state=absent result=failed
/// ```
///
/// `reason=` 永远是最后一个字段：它是一句可读的中文（可能带空格），排在中间会让后面的
/// 字段没法逐项 grep。
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

/// 找跑 JDT LS 的 JDK（**必须 ≥ 21**，`third_party/jdtls/manifest.json` 的
/// `minimumJavaVersion`，实测 JDT LS 1.61 在 1.8 上直接起不来）。
///
/// 判据顺序与"覆盖值为什么排第一"写在 [`runtime_candidates`] 上；
/// 覆盖值的一次判定结论写成一行可 grep 的 `S1_JAVA_TOOLCHAIN`（[`toolchain_override_line`]）。
/// **覆盖值无效不会中断发现**：它只是第 1 条候选失败，循环继续走第 2 条往后，
/// 并在诊断行里说明"降级到了自动发现、最终用的是哪个"。
pub(crate) fn resolve_runtime(jdtls_root: &Path) -> Result<JavaRuntime, String> {
    resolve_runtime_in(
        jdtls_root,
        crate::toolchain::java_home_override().as_deref(),
        &RuntimeInputs::from_process(),
    )
}

/// [`resolve_runtime`] 的实现体：外部输入全部由入参给（因此可以确定性单测）。
fn resolve_runtime_in(
    jdtls_root: &Path,
    overridden: Option<&Path>,
    inputs: &RuntimeInputs,
) -> Result<JavaRuntime, String> {
    let mut rejected = Vec::new();
    // 覆盖值自己的失败原因要**单独留一份**：它要进 `S1_JAVA_TOOLCHAIN`
    // （"你选的那个为什么没用上"），而 `rejected` 那一串是"自动发现都试过什么"。
    let mut override_rejection: Option<String> = None;

    for (source, candidate) in runtime_candidates(jdtls_root, inputs, overridden) {
        let Some(executable) = java_executable_in(&candidate, source) else {
            // 覆盖值指名的地方没有 `java` 也要记：自动发现里"这个目录不是 JDK"是常态
            // （`PATH` 上绝大多数目录如此），但用户指名的地方没有就是问题。
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
                        // 走到这里说明覆盖值那条候选已经失败过（它是第 1 条），
                        // 所以原因一定在；真取不到也不静默，给一句通用说明。
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
            // 覆盖值这条候选是第 1 条：走到"没有可用 JDK"就说明它已经失败过，
            // 失败原因一定在 `override_rejection` 里。
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

/// 一个候选路径 → 它的 `java` 可执行文件。
///
/// 两种形状都认（与既有实现一致）：候选本身是**文件**时它就是 `java` 可执行文件；
/// 否则当 JDK 主目录看，找 `<candidate>/bin/<java 可执行文件名>`。
///
/// 覆盖值比自动发现**多认一个无扩展名的 `java`**：设置页对覆盖值用的就是
/// `java.exe` 与 `java` 两个名字（`settings/src/project.rs:985-997` 的 `java_executable_in`），
/// 那边说"可用"、这边却被判无效，就又是一处"页面说能用、实际不生效"。
/// 自动发现那一路**保持原样**（只认 `java.exe`）：它是一条已经稳定的发现链，
/// 改它的候选名会连带影响所有既有机器上的解析结果。
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

/// 去空白后非空才算"设置过"（与 `settings/src/project.rs` 的 `non_empty_opt` 同一口径：
/// 界面上把输入框清空写的是空串，空串等于没填，不是"当前目录"）。
fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
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

    // -----------------------------------------------------------------------
    // JDK 覆盖值（设置页 `javaHomePath`）与判据顺序
    // -----------------------------------------------------------------------

    /// 一次测试用完就删的临时目录（断言失败也会在 `Drop` 里删掉）。
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

        /// 造一个 `<dir>/bin/<name>`（不是可执行的 JDK，只是"这里有一个 java 名字的文件"）。
        ///
        /// 名字要显式给：覆盖值那一路多认无扩展名的 `java`，而自动发现那一路只认
        /// `java_executable_name()` —— 用例想验哪一条就得造对应的那个名字。
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

    /// 一份"环境变量全都在"的输入，用于把判据顺序逐项钉住。
    ///
    /// `PATH` 用 `join_paths` 拼而不是写死分隔符：Windows 是 `;`、其余平台是 `:`
    /// （写死会让这条用例在另一个平台上把整串当成一个目录）。
    fn inputs_with_every_source() -> RuntimeInputs {
        RuntimeInputs {
            // 前后空白必须被吃掉（设置/环境里粘贴进来的路径很容易带空白）。
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

    /// **判据顺序**：覆盖值（用户显式选择）→ `LITHE_JDTLS_JAVA` → 捆绑 → JDTLS 自带 `jre` →
    /// `JAVA_HOME` → `PATH` → 常见安装根。
    ///
    /// 这条顺序是本次修复的核心语义（页面把覆盖值标成「已选择」，那它就必须真的压过自动发现），
    /// 所以逐项断言而不是只看第一项。
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

    /// 没登记覆盖值时，第 1 条判据就是 `LITHE_JDTLS_JAVA`（既有用法原样保留）；
    /// 空白值等于没设置。
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

    /// 覆盖值的两种形状都认：JDK 主目录（`<home>/bin/java*`）与直接填 `java` 可执行文件；
    /// 并且**只有覆盖值**多认一个无扩展名的 `java`（自动发现的候选名保持原样）。
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

    /// **覆盖值无效 ⇒ 继续走自动发现**（不是启动失败、也不是静默跳过）。
    ///
    /// 用一个"有 `bin/java` 但不是 JDK"的目录当覆盖值：它会被 `java -version` 的判定拒掉，
    /// 于是它的路径必须出现在"已排除"里（= 确实被试过），**并且**它后面的自动发现候选也必须
    /// 被试过（= 真的降级了，没有在覆盖值这里中断）。覆盖值自己的原因则进 `S1_JAVA_TOOLCHAIN`。
    #[test]
    fn an_unusable_override_falls_back_instead_of_stopping_the_chain() {
        let override_dir = TempDir::new("override");
        let override_java = override_dir.with_fake_java("java");
        // 降级之后的那个候选必须用**自动发现认得的名字**，否则它连试都不会被试到，
        // 这条断言也就测不到"链继续往下走了"（`java` 这个名字只有覆盖值那一路认）。
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

    /// `S1_JAVA_TOOLCHAIN` 的四种形态：没填 / 生效 / 无效并降级成功 / 无效且自动发现也没找到。
    ///
    /// 断言的是**字段与稳定 token**（`state=` / `result=` / `fallback=` / `javaVersion=`），
    /// 而不是整行字面量：这行的用途就是被 grep 出来人工核对，字段不能漂。
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

    /// 选定式（opt-in）**真实 JDK** 检查：覆盖值必须压过环境自动发现。
    ///
    /// 开关与 `service.rs` 的真实 JDTLS 用例同一约定（CI 上不跑）：
    /// `LITHE_GPUI_JDTLS_SMOKE` 非空 **且** `LITHE_GPUI_JDTLS_SMOKE_JDK_HOME` 指出一个真实的
    /// JDK 21+ 主目录。第 2、3 条判据（`LITHE_JDTLS_JAVA`、捆绑位置）在这里**故意指向不存在的
    /// 路径**：覆盖值仍然必须被采用 —— 这正是"设置里选了 JDK 就真的用它"这条断言。
    #[test]
    fn real_java_home_override_beats_the_environment() {
        if std::env::var_os("LITHE_GPUI_JDTLS_SMOKE").is_none_or(|value| value.is_empty()) {
            return;
        }
        // 前后空白要去掉才能当路径用：`cmd` 里写 `set VAR=值 && ...` 会把那个空格算进
        // 值里（本轮实测过一次，结果是 `state=invalid reason=找不到 java.exe`）。
        // 生产路径上的空白由登记方去掉（`workbench/src/workspace.rs` 的
        // `java_toolchain_override_from` 会 `trim`），所以这里 trim 才是这条用例要的输入。
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
