//! 「项目 · JDK 与 Maven」页的**数据层**：本机 JDK / Maven 环境的发现 + 纯解析 + 纯判定。
//!
//! ## 这一页的数据从哪来（每一格都有出处，绝无编造）
//!
//! | 界面上的格子 | 来源 | 依据 |
//! | --- | --- | --- |
//! | JDK 主目录 / 版本 / 来源 | 本模块的发现链：显式环境变量 → `JAVA_HOME` → `PATH` → 本机常见安装目录 | 发现顺序逐条照 `gpui/crates/java/src/jdtls.rs:440-495` 的 `resolve_runtime` 与 `:519-563` 的 `installed_jdk_roots`（**只读参考**：那些函数是 `pub(crate)`，本 crate 既不该也不能调它们；这里只沿用**顺序与判据语义**，不复制代码） |
//! | Maven 可执行文件 / 版本 / 安装目录 / 运行时 JDK | `MAVEN_HOME` → `M2_HOME` → `PATH` 找 `mvn*`，再跑 `mvn -version` | 候选名与顺序照 `windows/tauri/src-tauri/src/run.rs:1206-1240`，输出解析照同文件 `:1822-1841`（`Apache Maven <版本>` / `Maven home:` / `Java version:`） |
//! | settings.xml | Maven 自己的两个既有位置：用户级 `~/.m2/settings.xml`、安装级 `<maven home>/conf/settings.xml` | 安装级的推导逐条照 Core `rust/lithe-core/src/project/maven.rs:456-481` 的 `installation_settings_path`（同样是"只有 `<home>/bin/mvn*` 才算安装根"） |
//! | 本地仓库 | 生效 settings.xml 里的 `<localRepository>`；没写就是 Maven 自己的默认位置 `~/.m2/repository` | `localRepository` 只从**用户级** settings.xml 读（Maven 文档：全局 settings 里这条不生效） |
//!
//! ⚠️ **这里没有新探测逻辑的发明**：三级顺序、`mvn` 候选名、"安装根 = `bin` 的父目录"、
//! 版本串的取法全部来自上面几处现成实现，本模块只做"读环境变量 + 跑一次 `-version` + 解析文本"。
//!
//! ## Core 有没有专门报告 JDK / Maven 环境的命令？**没有**
//!
//! 查过 `shared/contracts/rust-core-api.md` 的命令表（`:100-169`）：`maven.*` 六条全是
//! **项目描述符解析与启动计划**（`maven.scan` 返回 reactor / modules / profiles / sourceRoots，
//! **不含** Maven 安装位置、版本、`settings.xml`、本地仓库），`runConfig.*` 读的是 `.lithe` 文档层，
//! 而契约 `:316` 明写「runtime discovery remain platform adapters」——**运行时发现归平台适配器**。
//! 所以这一页的"检测到的安装"只能由本侧探测，这一点在汇报里如实登记。
//!
//! ## 与真源（`components/project-environment-settings.tsx`）的关系
//!
//! 真源那三个字段（`javaHomePath` / `mavenExecutablePath` / `mavenJavaHomePath`）在这里是
//! **同一套键名**的**全局**覆盖值（见 [`Overrides`] 与 `schema.rs`），每个字段下面那行
//! 「生效值」的语义与真源 `features/run/utils/effective-toolchain.ts:23-58` 的
//! `describeEffectiveToolchain` 逐条对齐：**模式 → 名字（+版本） · 路径 · 来源**，
//! 另外两种状态是"你用不了"（`toolchain.invalid`）与"没找到"（`toolchain.*NotFound`）。
//!
//! ## 为什么路径按**原生分隔符**显示
//!
//! 同一个设置对话框的「Git」页已经用 `host.workspace_root.display()`（原生 `\`）展示路径，
//! 这一页跟着它走：Windows 用户看到的 `C:\Users\x\.m2\settings.xml` 可以直接粘进资源管理器。
//! 送进 JSON / JVM 的那份归一化（`\\?\` → `/`）是 `jdtls.rs` 自己的口径，与本页的显示无关。

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// 本页诊断行的前缀（走 stdout，与 `S1_MAVEN` / `S1_SETTINGS` 一族同口径）。
///
/// 页面上每一个显示出来的事实都能在这一行里核对（见
/// [`ProjectEnvironment::diagnostic_line`]），所以"界面上显示的值"与"日志里的值"可以逐项对照。
pub const PROJECT_DIAGNOSTIC_TAG: &str = "S1_SETTINGS_PROJECT";

/// 显式指定 `java` 可执行文件的环境变量名。**与 `jdtls.rs:40` 的 `JAVA_EXECUTABLE_ENV` 同名**：
/// 它就是本仓库"这台机器上到底用哪个 JDK"的既有覆写入口（`jdtls.rs:431` 的注释写明了它存在的
/// 理由：本机 `PATH` 上的 `java` 是 1.8，验证链路靠它）。
pub const JAVA_EXECUTABLE_ENV: &str = "LITHE_JDTLS_JAVA";
/// JDK 主目录的环境变量名。
pub const JAVA_HOME_ENV: &str = "JAVA_HOME";
/// Maven 安装目录的环境变量名（首选）。
pub const MAVEN_HOME_ENV: &str = "MAVEN_HOME";
/// Maven 安装目录的环境变量名（旧名，Maven 自己仍认）。
pub const M2_HOME_ENV: &str = "M2_HOME";
/// `PATH` 环境变量名。
pub const PATH_ENV: &str = "PATH";
/// 用户主目录的环境变量名（Windows 优先，其余平台回落到 `HOME`，照 `jdtls.rs:521`）。
pub const USER_PROFILE_ENV: &str = "USERPROFILE";
/// 见 [`USER_PROFILE_ENV`]。
pub const HOME_ENV: &str = "HOME";
/// 常见 JDK 安装根用到的环境变量（枚举顺序照 `jdtls.rs:519-549`）。
pub const INSTALL_ROOT_ENVS: [&str; 5] = [
    "ProgramFiles",
    "ProgramFiles(x86)",
    "LOCALAPPDATA",
    "ProgramData",
    "XDG_DATA_HOME",
];
/// 每个"常见安装根"下面枚举一层子目录的候选上限。
///
/// `jdtls.rs` 那边不需要上限：它在第一个满足 `>= 21` 的候选上就返回了；本页要把
/// **检测到的安装全列出来**，所以自己收口 —— 否则一个堆了几十个 JDK 的目录会让每次刷新
/// 变成几十次 `java -version`（每次都是一次进程创建，Windows 上尤其贵）。
/// 收口只影响"列不列得全"，不影响发现顺序（被截掉的永远是列表最末尾的那些）。
pub const MAX_INSTALLED_JDK_CANDIDATES: usize = 24;
/// 用户级 Maven 配置的目录名（`~/.m2`）。
pub const M2_DIRECTORY: &str = ".m2";
/// 用户级 Maven 配置文件名。
pub const SETTINGS_FILE_NAME: &str = "settings.xml";
/// Maven 默认本地仓库目录名（`~/.m2/repository`）。
pub const REPOSITORY_DIRECTORY: &str = "repository";
/// Maven 安装目录下配置目录名（`<maven home>/conf`）。
pub const CONF_DIRECTORY: &str = "conf";
/// 安装根的 `bin` 目录名（Core 用它判断"上面那一层是不是安装根"）。
pub const BIN_DIRECTORY: &str = "bin";

/// JDT LS 允许的最低 Java **主版本**。
///
/// ## 数值与判据的唯一真源：`gpui/crates/java/src/jdtls.rs`
///
/// 这一个 `21` **不是本模块自己定的**，它是 `jdtls.rs` 里同一条判据的镜像（那个 crate
/// 对本 crate 是**只读参考**：`settings` 不认识 `lithe-gpui-java`，反向依赖会成环）：
///
/// - 数值本身：`jdtls.rs:80` 的 `const MINIMUM_JAVA_MAJOR: u64 = 21;`，
///   它的文档写着来源是 `third_party/jdtls/manifest.json` 的 `minimumJavaVersion`
///   （即 **JDT LS 载荷自己声明的运行要求**，不是我们拍脑袋选的版本）；
/// - 判据落点：`jdtls.rs:820-837` 的 `probe_java` —— 跑 `java -version`、解析版本，
///   `major_version(&version) < MINIMUM_JAVA_MAJOR` 就返回
///   `Err("版本 {version} 低于 {MINIMUM_JAVA_MAJOR}")`；
/// - 主版本解析口径：`jdtls.rs:848-859` 的 `major_version`
///   （`1.8.0_221` → 8、`21.0.2` → 21、`25` → 25），与下文的 [`jdk_major_version`] 逐条对齐。
///
/// ⚠️ **为什么这个闸门必须出现在设置页**：`resolve_runtime`（`jdtls.rs:440-495`）会把
/// 低于闸门的候选**丢掉并继续往下找**，一个都不满足时整条链路失败
/// （`jdtls.rs:713-715` 的"未找到 Java 21 或更新版本的 JDK，JDT LS 无法启动"）。
/// 页面如果只报"自动 → JDK 1.8.0_221"而不说这一层，用户就无法预判
/// **语言服务到底能不能起来** —— 那正是"显示 ≠ 实际"。
///
/// 两条路用的是同一个闸门与同一个解析函数（见 [`JdkDiscovery::effective`]）：
/// 自动发现（`LITHE_JDTLS_JAVA` / `JAVA_HOME` / `PATH` / 常见安装根）与
/// 设置页的覆盖值（`javaHomePath`）。
pub const MINIMUM_JAVA_MAJOR: u64 = 21;

/// 从 `java -version` 报出的版本串里取主版本（`1.8.0_221` → 8、`21.0.2` → 21、`25` → 25）。
///
/// 逐条对齐 `jdtls.rs:848-859` 的 `major_version`（那里是唯一真源，本函数只是它的镜像）：
/// 先按 `.` / `_` / `-` / `+` 切开；首段是 `1` 时（`1.8.0_221` 这种 2006 年前的旧命名）
/// 主版本是**第二段**；否则首段自己就是主版本。读不出来时返回 `0`
/// （`0 < MINIMUM_JAVA_MAJOR` ⇒ 判定为"不满足"，与 `jdtls.rs` 的 `unwrap_or(0)` 同一条口径）。
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

/// 这个版本串是否满足 JDT LS 的最低要求（`>= MINIMUM_JAVA_MAJOR`）。
pub fn meets_language_service_requirement(version: &str) -> bool {
    jdk_major_version(version) >= MINIMUM_JAVA_MAJOR
}

/// 一个工具是**从哪一级**找出来的。界面上的「来源」一栏就是它。
///
/// `label_key` 指向的文案键里，`ToolSource::EnvOverride` / `ToolSource::MavenHome` 带一个
/// `{name}` 占位符（变量名），调用方用 `tr_args` 填 [`Self::label_arg`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSource {
    /// 显式环境变量 `LITHE_JDTLS_JAVA`（第 1 级）。
    EnvOverride,
    /// `JAVA_HOME`（第 2 级）。
    JavaHome,
    /// `MAVEN_HOME` / `M2_HOME`（Maven 的第 1 级）。
    MavenHome,
    /// `PATH`（第 3 级）。
    Path,
    /// 本机常见安装目录里枚举出来的 JDK（`jdtls.rs:519-563` 的第 5 级）。
    ///
    /// ⚠️ 它排在 `PATH` **之后**：`PATH` 上的那个才是"命令行走 `java` 时拿到的那个"，
    /// 枚举出来的只是"这台机器上还装着什么"。界面上两者的区别就是这一栏。
    InstalledRoot,
}

impl ToolSource {
    /// 诊断行 `source=…` / `mavenSource=…` 的取值（可 grep 的稳定 token）。
    pub fn id(self) -> &'static str {
        match self {
            Self::EnvOverride => "env",
            Self::JavaHome => "java_home",
            Self::MavenHome => "maven_home",
            Self::Path => "path",
            Self::InstalledRoot => "installed_root",
        }
    }

    /// 「来源」那一栏的文案键。
    pub fn label_key(self) -> &'static str {
        match self {
            // `toolchain.source.javaHome`（来自 JAVA_HOME）、`toolchain.source.path`
            // （来自 PATH）与 `toolchain.source.detected`（本机检测到）都是真源既有键，逐字复用。
            Self::JavaHome => "lithe.toolchain.source.javaHome",
            Self::Path => "lithe.toolchain.source.path",
            Self::InstalledRoot => "lithe.toolchain.source.detected",
            // 这两个来源是"某个环境变量"，真源没有对应措辞（Windows 只报 javaHome / path /
            // detected / mavenWrapper），所以用 gpui 侧新增的带 `{name}` 占位符键。
            Self::EnvOverride => "lithe.settings.gpui.sourceFromEnv",
            Self::MavenHome => "lithe.settings.gpui.sourceFromEnv",
        }
    }

    /// [`Self::label_key`] 里 `{name}` 占位符的取值；不需要占位符的来源返回 `None`。
    pub fn label_arg(self) -> Option<&'static str> {
        match self {
            Self::EnvOverride => Some(JAVA_EXECUTABLE_ENV),
            Self::MavenHome => Some(MAVEN_HOME_ENV),
            Self::JavaHome | Self::Path | Self::InstalledRoot => None,
        }
    }
}

/// 一行「生效值」的模式（真源 `EffectiveToolchainMode`，`effective-toolchain.ts:5`）。
///
/// 真源那一版还有 `inherit`（运行配置继承项目环境）；本页是**项目环境的落点本身**，
/// 没有"再往上继承一层"的东西，所以只有这三档。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMode {
    /// 没设覆盖值，用的是自动检测到的那个。
    Automatic,
    /// 用的是设置文件里的覆盖值。
    Configured,
    /// Maven 用的 JDK 没单独设置，跟随 JDK 那一行的生效值。
    ProjectJdk,
}

impl ToolMode {
    /// 诊断行 / 界面模式文案的键（真源 `toolchain.mode.*`）。
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Automatic => "lithe.toolchain.mode.automatic",
            Self::Configured => "lithe.toolchain.mode.configured",
            Self::ProjectJdk => "lithe.toolchain.mode.projectJdk",
        }
    }

    /// 诊断行里的稳定 token。
    pub fn id(self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Configured => "configured",
            Self::ProjectJdk => "project_jdk",
        }
    }
}

/// 一个探测成功的 JDK。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedJdk {
    /// JDK 主目录（`<java>` 可执行文件的上一级的上一级）。跑不起来或路径太浅时是空串。
    pub home: String,
    /// `java` 可执行文件的完整路径。
    pub executable: String,
    /// `java -version` 里的版本串（**原样**，不转成数字：`1.8.0_402` 与 `21.0.2` 都要能显示）。
    pub version: String,
    /// 这个版本能不能跑 JDT LS（`>= MINIMUM_JAVA_MAJOR`，判据见 [`MINIMUM_JAVA_MAJOR`]）。
    ///
    /// ⚠️ **低于闸门的 JDK 仍然是一条"检测到的安装"**（它确实装在这台机器上、版本也读出来了），
    /// 所以它留在候选列表里；但一旦它成为**生效值**，页面必须标出"语言服务起不来"
    /// —— 见 [`EffectiveToolchain::Unusable`] 的 `language_service_rejection`。
    pub meets_language_service: bool,
    /// 从哪一级找出来的。
    pub source: ToolSource,
}

/// 一个探测成功的 Maven。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedMaven {
    /// `mvn` 启动器（或 `mvn.cmd` / `mvn.bat`）的完整路径。
    pub executable: String,
    /// `mvn -version` 第一行 `Apache Maven <版本>` 里的版本串；读不出时是空串。
    pub version: String,
    /// `mvn -version` 的 `Maven home:`；读不出时 `None`。
    pub home: Option<String>,
    /// `mvn -version` 的 `Java version:`（`21.0.2` 这种）；读不出时 `None`。
    pub java_runtime: Option<String>,
    /// 从哪一级找出来的。
    pub source: ToolSource,
}

/// 覆盖值（设置文件里的路径）探测成功后需要的那点事实。
///
/// 覆盖值**不带来源**：真源的口径是"你选的就是你选的"，模式那一栏已经说明白了
/// （`effective-toolchain.ts:51-54`：`source` 只在 `configured` / `projectJdk` 之外才拼）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedTool {
    /// JDK 主目录，或 Maven 可执行文件路径（都按生效口径给出）。
    pub path: String,
    /// 版本串。
    pub version: String,
    /// 见 [`DetectedJdk::meets_language_service`]（Maven 那条探测恒为 `true`：
    /// Maven 的版本与 JDT LS 的 Java 门槛无关，这里不假造一个判定）。
    pub meets_language_service: bool,
}

/// 一个覆盖值的探测结论（连同用户填的原文，用不了时要原样回显）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverrideProbe {
    /// 设置文件里填的原文。
    pub path: String,
    /// `Err` 里是"为什么用不了"的可读原因（路径不存在 / 跑不起来 / 读不出版本）。
    pub result: Result<DetectedTool, String>,
}

/// 一次 JDK 发现的全部结论。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JdkDiscovery {
    /// 覆盖值的探测结论；`None` = 设置里没填覆盖值。
    pub overridden: Option<OverrideProbe>,
    /// 三级发现里**探测成功**的候选，顺序 = 发现顺序（第一项就是自动生效的那个）。
    pub candidates: Vec<DetectedJdk>,
    /// 试过但用不了的候选，形如 `路径（原因）`；界面在"没找到"时把它显示出来。
    pub rejected: Vec<String>,
}

/// 一次 Maven 发现的全部结论。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MavenDiscovery {
    /// 见 [`JdkDiscovery::overridden`]。
    pub overridden: Option<OverrideProbe>,
    /// 探测成功的候选，顺序 = 发现顺序。
    pub candidates: Vec<DetectedMaven>,
    /// 见 [`JdkDiscovery::rejected`]。
    pub rejected: Vec<String>,
}

/// 本地仓库是怎么定下来的。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalRepositorySource {
    /// 生效的**用户级** settings.xml 里写了 `<localRepository>`。
    SettingsXml,
    /// settings.xml 没写（或没有 settings.xml），用 Maven 自己的默认位置 `~/.m2/repository`。
    MavenDefault,
}

impl LocalRepositorySource {
    /// 诊断行 `localRepoSource=…` 的取值。
    pub fn id(self) -> &'static str {
        match self {
            Self::SettingsXml => "settings_xml",
            Self::MavenDefault => "maven_default",
        }
    }

    /// 「来源」那一栏的文案键。
    pub fn label_key(self) -> &'static str {
        match self {
            Self::SettingsXml => "lithe.settings.gpui.mavenLocalRepositoryFromSettings",
            Self::MavenDefault => "lithe.settings.gpui.mavenLocalRepositoryDefault",
        }
    }
}

/// 本地仓库路径 + 它是怎么来的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRepository {
    /// 仓库目录路径（settings.xml 里 `~` / `${...}` 这类表达式**原样**显示，不做展开）。
    pub path: String,
    /// 见 [`LocalRepositorySource`]。
    pub source: LocalRepositorySource,
}

/// Maven 自己的两个 settings.xml 位置、用户选的那一份与本地仓库。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MavenConfiguration {
    /// 用户级 `~/.m2/settings.xml`（**存在**时才有值）。
    pub user_settings: Option<String>,
    /// 安装级 `<maven home>/conf/settings.xml`（存在时才有值）。
    pub installation_settings: Option<String>,
    /// 用户在设置页选的 `settings.xml`（`mavenSettingsPath` 的原文；空串 = 没选）。
    ///
    /// **原样保留**（不做存在性过滤）：这一行是"你选的是哪一个"的事实，界面要能显示出来
    /// 并标出它不存在；`override_settings_missing` 单独记存在性。
    pub override_settings: Option<String>,
    /// 选的 `settings.xml` **不是一个存在的文件**（拼错 / 被删 / 指到目录）。
    /// 界面据此把这一行画成失败色，而不是让一个不存在的路径看起来"已生效"。
    pub override_settings_missing: bool,
    /// 本地仓库；`None` = 连用户主目录都拿不到（此时界面显示"未知"）。
    pub local_repository: Option<LocalRepository>,
}

impl MavenConfiguration {
    /// 生效的 settings.xml：**选的那一份优先**，其次用户级，最后安装级。
    ///
    /// Maven 自己只读用户级与安装级（用户级覆盖安装级）；本侧多一层"用户明确选的那一份"，
    /// 因为那一份会被交给语言服务（`java.configuration.maven.userSettings`）去**替代**
    /// 自动检测的结果 —— 所以它必须排在生效值的第一位，页面上的"生效值"也只能指它。
    pub fn effective_settings(&self) -> Option<&str> {
        self.override_settings
            .as_deref()
            .or(self.user_settings.as_deref())
            .or(self.installation_settings.as_deref())
    }

    /// 生效的 settings.xml 是**用户选的那一份**（而不是自动检测到的）。
    pub fn settings_is_overridden(&self) -> bool {
        self.override_settings.is_some()
    }
}

/// 页面上的五个覆盖值（都是**全局**设置键，见 `schema.rs` 的说明）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overrides {
    /// JDK 主目录（`javaHomePath`）。
    pub java_home: String,
    /// Maven 主目录 / 可执行文件（`mavenExecutablePath`）。
    pub maven_executable: String,
    /// Maven 用的 JDK 主目录（`mavenJavaHomePath`）。
    pub maven_java_home: String,
    /// 用户选的 Maven `settings.xml`（`mavenSettingsPath`）。
    pub maven_settings: String,
    /// 用户选的 Maven 本地仓库（`mavenLocalRepositoryPath`）。
    pub maven_local_repository: String,
}

impl Overrides {
    /// 五个覆盖值全为空 = 全自动（真源也是 `""` 表示"用自动检测的值"）。
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 非空覆盖值的个数（诊断行用）。
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
}

/// 一行的「生效值」。界面只按这里的字段画，判定逻辑全在这里（因此可以脱离 GPUI 单测）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectiveToolchain {
    /// 有可用值。`source` 只在自动选出时有值（覆盖值的来源由 [`ToolMode::Configured`] 表达）。
    Resolved {
        /// 自动 / 你选的 / 跟随项目 JDK。
        mode: ToolMode,
        /// JDK 主目录，或 Maven 可执行文件路径。
        path: String,
        /// 版本串；`None` = 读不出版本（界面退化成只显示名字 `JDK` / `Maven`）。
        version: Option<String>,
        /// 自动选出时的来源。
        source: Option<ToolSource>,
    },
    /// 覆盖值填了但**用不了**：界面用失败色 + [`Self::Unusable::reason`]。
    Unusable {
        /// 用户填的覆盖值原文（原样显示，方便对照）。
        path: String,
        /// 为什么用不了。
        reason: String,
        /// 这个"用不了"是不是**仅仅因为版本低于 JDT LS 的门槛**
        /// （= `jdtls.rs` 的 `probe_java` 那一条判据，见 [`MINIMUM_JAVA_MAJOR`]）。
        ///
        /// 为 `true` 时界面会在 [`Self::Unusable::reason`] 之外**再补一句**
        /// "低于语言服务最低要求 N，语言服务无法启动" + 这个版本串，让用户一眼能预判
        /// 语言服务起不来（而不是看到一个"因为版本低所以用不了"的笼统失败）。
        /// 为 `false` 的是路径不存在 / 跑不起来 / 读不出版本这一档，那时补一句关于版本的话
        /// 反而是编造（我们连版本都没读到）。
        language_service_rejection: bool,
    },
    /// 没有覆盖值，也没探测到任何东西。
    NotFound {
        /// 已经试过哪些位置、各自为什么不行。
        reason: String,
    },
}

impl EffectiveToolchain {
    /// 诊断行里这一格的取值（路径或 `none` / `unusable`）。
    pub fn diagnostic_value(&self) -> String {
        match self {
            Self::Resolved { path, .. } => path.clone(),
            Self::Unusable { path, .. } => format!("unusable({path})"),
            Self::NotFound { .. } => "none".to_string(),
        }
    }

    /// 诊断行里这一格的模式。
    pub fn diagnostic_mode(&self) -> &'static str {
        match self {
            Self::Resolved { mode, .. } => mode.id(),
            Self::Unusable { .. } => "unusable",
            Self::NotFound { .. } => "none",
        }
    }
}

impl JdkDiscovery {
    /// 自动生效的 JDK：三级里**第一个满足 JDT LS 最低版本**的候选。
    ///
    /// ⚠️ 判据必须与 `jdtls.rs` 的 `resolve_runtime`（`:440-495`）一致：那一条链路会
    /// 逐候选跑 `probe_java`，**低于 `MINIMUM_JAVA_MAJOR` 的直接丢弃并继续往下找**
    /// （本机实测：`PATH` 上的 1.8 被拒之后它继续找 `JAVA_HOME` 的 21）。所以页面上的
    /// "自动生效值"也只能是"第一个过得去闸门的那个" —— 否则页面报 1.8、语言服务实际用 21，
    /// 或者页面报"可用"而语言服务起不来。
    ///
    /// 一个候选都不满足时返回 `None`（由 [`Self::effective`] 翻成如实标注的"用不了"），
    /// 而**不是**把第一个候选假装成可用值。
    pub fn detected(&self) -> Option<&DetectedJdk> {
        self.candidates
            .iter()
            .find(|jdk| jdk.meets_language_service)
    }

    /// 一个候选都不过闸门时的那一条：候选列表的**第一项**（发现顺序最高的那个）。
    ///
    /// 用它来给出"这台机器上最接近可用"的那条事实（路径 + 版本），界面据此标出
    /// "低于语言服务最低要求 21"。
    pub fn closest_below_requirement(&self) -> Option<&DetectedJdk> {
        self.candidates.first()
    }

    /// JDK 这一行的生效值（真源 `resolved.java` 的等价物）。
    ///
    /// 三条路，与实测判据一一对应：
    ///
    /// 1. **覆盖值可用**（`javaHomePath` 指向的 JDK 过了 ≥ 21 闸门）→ `Resolved{Configured}`；
    /// 2. **覆盖值低于闸门** → `Unusable` + `language_service_rejection = true`
    ///    （`jdtls.rs` 的 `resolve_runtime` 会拒掉它，页面必须说出来）；
    /// 3. **没有覆盖值** → 取自动发现里**第一个过闸门**的候选；一个都不过闸门时，
    ///    用发现顺序里的第一项如实标出"低于最低要求"（而不是报 `NotFound`：JDK 确实找到了，
    ///    只是版本不够 —— 这两种事实的可排查方向完全不同）。
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
        // 候选都在，但没有一个能跑 JDT LS：如实报"是哪一个、为什么"。
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

    /// 一个都没探测到时，把"试过哪些、为什么不行"拼成一句（界面照原样显示，绝不静默空态）。
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
    /// 自动生效的 Maven（第一项）。
    pub fn detected(&self) -> Option<&DetectedMaven> {
        self.candidates.first()
    }

    /// Maven 这一行的生效值。
    ///
    /// ⚠️ Maven **没有** JDT LS 那种 Java 版本闸门：`mavenJavaHomePath` 指向的 JDK 用不了
    /// 就是路径/可执行文件本身的问题（见 [`Self::overridden`] 的 `Err`），所以这一行的
    /// `Unusable` 恒为 `language_service_rejection = false`（不把 Java 门槛混进 Maven 的失败）。
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

/// 一次完整探测的结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectEnvironment {
    /// 本次用的覆盖值（探测时就带上了，界面直接回显）。
    pub overrides: Overrides,
    /// JDK。
    pub jdk: JdkDiscovery,
    /// Maven。
    pub maven: MavenDiscovery,
    /// Maven 自己的配置与本地仓库。
    pub maven_config: MavenConfiguration,
    /// Maven 用的 JDK 的生效值判定所需的那份发现（覆盖值另算）。
    pub maven_jdk: JdkDiscovery,
}

impl ProjectEnvironment {
    /// **两行都落到「未找到」**（JDK 与 Maven 各一个可用值都没有，也没有填覆盖值）
    /// —— 这是页面上唯一会出现整页级提示的条件。
    ///
    /// ⚠️ "填了覆盖值但用不了"**不算**空：那一格要如实画出"无法使用 + 原因"，
    /// 这是可排查的信息，不该被一句整页提示盖掉。
    pub fn has_no_toolchain(&self) -> bool {
        matches!(self.jdk.effective(), EffectiveToolchain::NotFound { .. })
            && matches!(self.maven.effective(), EffectiveToolchain::NotFound { .. })
    }

    /// Maven 那一行的 JDK：单独设了就跟单独设的，否则跟随 JDK 那一行的生效值。
    ///
    /// 与真源一致（`project-environment-settings.tsx:177-193` 的 `mode: "projectJdk"`）：没设时
    /// 显示成**项目 JDK 的生效值**，而不是空。跟随值**不带来源** —— 真源对 `projectJdk`
    /// 这条模式同样不拼来源（`effective-toolchain.ts:51-54`）。
    pub fn maven_jdk_effective(&self) -> EffectiveToolchain {
        if self.maven_jdk.overridden.is_some() {
            return self.maven_jdk.effective();
        }
        match self.jdk.effective() {
            EffectiveToolchain::Resolved {
                path, version, ..
            } => EffectiveToolchain::Resolved {
                mode: ToolMode::ProjectJdk,
                path,
                version,
                source: None,
            },
            other => other,
        }
    }

    /// 可 grep 的一行诊断：页面上显示的每个值都能在这里核对。
    ///
    /// 形状（**顺序固定**，便于 `grep 'S1_SETTINGS_PROJECT'` 后逐项对照）：
    /// `jdk=… source=… version=… maven=… mavenVersion=… mavenHome=… mavenSource=… settings=… localRepo=… …`
    ///
    /// 其中 Maven 配置那一段的顺序是
    /// `settings=… settingsInstallation=… localRepo=… localRepoSource=… settingsOverridden=…
    /// overrideJdk=… overrideMaven=… overrideMavenJdk=… overrideSettings=… overrideLocalRepo=…`：
    /// 前四项是**检测/推导出来的事实**，后五项是**用户填的覆盖值原文**（空 = `-` 之外的原文，
    /// 由 `Overrides` 原样带出），`settingsOverridden` 说明生效的 settings.xml 是不是用户选的那份。
    pub fn diagnostic_line(&self) -> String {
        let jdk = self.jdk.effective();
        let maven = self.maven.effective();
        let maven_jdk = self.maven_jdk_effective();
        // 语言服务能不能起来（本批 A1 的核心判据）：`languageService=ready` 才代表
        // 页面显示的那个生效 JDK 满足 `jdtls.rs` 的 ≥ 21 闸门。
        // 与 `S1_JAVA_JDTLS javaVersion=…` 对照就能证明"页面显示"与"实际起服务用的 JDK"一致。
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
            // 生效的 settings.xml 是不是用户选的那份（`yes` 时下面那格 `overrideSettings=` 就是它）。
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
        // ⚠️ `Unusable` 那一档也要报版本：低于 ≥ 21 闸门时版本号**是读到了的**
        // （只是过不了闸门），页面上那句话就带它；诊断行漏掉它会让
        // `S1_SETTINGS_PROJECT` 与 `S1_JAVA_JDTLS javaVersion=…` 对不上，
        // 也无法用日志复核"页面报的那个版本是哪一个"。
        EffectiveToolchain::Unusable { reason, .. } => version_in_reason(reason),
        EffectiveToolchain::NotFound { .. } => None,
    }
}

/// 从 `版本 {X} 低于 {N}…` 这类原因串里把版本号抠出来（[`below_requirement_reason`] 的逆运算）。
///
/// 纯字符串处理、失败就返回 `None`（**不编一个版本**）：诊断行宁可少一格，
/// 也不能因为解析不出来而报一个错的版本。
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

/// 环境变量的一份**快照**（纯数据，所以候选路径的构造可以脱离真实环境单测）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvSnapshot {
    /// [`JAVA_EXECUTABLE_ENV`]。
    pub java_executable: Option<String>,
    /// [`JAVA_HOME_ENV`]。
    pub java_home: Option<String>,
    /// [`MAVEN_HOME_ENV`]。
    pub maven_home: Option<String>,
    /// [`M2_HOME_ENV`]。
    pub m2_home: Option<String>,
    /// [`PATH_ENV`] 的原文（`;` / `:` 分隔）。
    pub path: Option<String>,
    /// 用户主目录（`USERPROFILE` 或 `HOME`）。
    pub user_home: Option<String>,
    /// 常见 JDK 安装根的基目录（键名见 [`INSTALL_ROOT_ENVS`]），值按同名环境变量取。
    pub install_roots: Vec<(String, String)>,
}

impl EnvSnapshot {
    /// 读当前进程的环境变量。
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

/// 一个带来源的候选可执行文件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// 从哪一级来。
    pub source: ToolSource,
    /// 候选路径（可能是 JDK 主目录，也可能是 `java` 可执行文件本身）。
    pub path: PathBuf,
}

/// JDK 的候选顺序：`LITHE_JDTLS_JAVA` → `JAVA_HOME` → `PATH` → 本机常见安装目录。
///
/// 前三级逐条照 `jdtls.rs:440-460` 的 `resolve_runtime`；第 4 级照同文件 `:519-563` 的
/// `installed_jdk_roots`（枚举一层子目录）。
///
/// **本模块没有移植的两级**：`jdtls.rs` 在 `JAVA_HOME` 之前还有"exe 旁的 `LanguageServers/jdk`、
/// 向上走的 `.artifacts/jdk`、JDTLS 安装根下的 `jre`" —— 那三级是 **JDT LS 自己的启动资源**
/// （要先知道 jdtls 装在哪、自己从哪个 exe 启动），本页没有那个上下文。这是有意的收窄，
/// 登记在汇报的已知边界里。
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

/// 本机常见 JDK 安装根下**枚举一层子目录**得到的候选（`jdtls.rs:519-563` 的等价物）。
///
/// 顺序是确定的：先按 [`INSTALL_ROOT_ENVS`] 里登记的顺序走各个根，每个根先放它自己、
/// 再按**目录名的字典序**放它的子目录（`read_dir` 的顺序是文件系统给的，不稳定，
/// 而界面与诊断行都要可复现）。只收目录：`%ProgramData%\java` 那种目录里同时躺着
/// 安装包（`jdk-8u45-windows-i586.exe`），把文件当候选去 `java -version` 只会白跑一次进程。
pub fn installed_jdk_roots(env: &EnvSnapshot) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(home) = non_empty_opt(env.user_home.as_deref()) {
        let home = Path::new(home);
        // IntelliJ 自动下载的 JDK、SDKMAN 与 scoop 都在这三个下面（`jdtls.rs:521-527`）。
        roots.push(home.join(".jdks"));
        roots.push(home.join(".sdkman").join("candidates").join("java"));
        roots.push(home.join("scoop").join("apps"));
    }
    for (name, base) in &env.install_roots {
        let base = Path::new(base);
        // 两个"常见根"在 `jdtls.rs` 里是按**变量名**分别处理的（`:544-549`），不是按值：
        // `%ProgramData%\java` 与 `$XDG_DATA_HOME/jdks` 的下一层布局与厂商目录不同。
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

/// 常见 JDK 厂商目录名（照 `jdtls.rs:531-539` 的列表，顺序也一致）。
pub const INSTALL_ROOT_VENDORS: [&str; 7] = [
    "Java",
    "Eclipse Adoptium",
    "Microsoft",
    "Amazon Corretto",
    "Zulu",
    "BellSoft",
    "Programs",
];

/// 一个安装根下的候选：根自己 + 它的子目录（按字典序）。
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

/// Maven 的候选（`MAVEN_HOME` → `M2_HOME` → `PATH`）。
///
/// ⚠️ 真源还有第 0 级：**项目自己的 Maven Wrapper**（`mvnw.cmd` / `mvnw`，
/// `windows/tauri/src-tauri/src/run.rs:1206-1208`）。本侧拿不到工作区根
/// （设置 crate 不依赖外壳，也没有项目级存储），所以这一级**没做** —— 见汇报的已知边界。
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

/// Maven 启动器的候选名（顺序照 `windows/tauri/src-tauri/src/run.rs:1234`）。
pub const MAVEN_LAUNCHER_NAMES: [&str; 4] = ["mvn.cmd", "mvn.bat", "mvn.exe", "mvn"];

/// `java` 可执行文件名（Windows 上是 `java.exe`，照 `jdtls.rs:92` 的口径）。
pub fn java_executable_name() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

/// 跑一次真实的发现：覆盖值 + 三级候选 + Maven 配置 + 本地仓库，并打一行诊断。
///
/// **会起子进程**（每个候选一次 `-version`），所以调用方必须放进后台任务，
/// 不要在 `render` 或 UI 线程上直接调（与 `lithe_gpui_shared::core_client` 的调用方同一条口径）。
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
    // 同一个 JDK 被 `JAVA_HOME` 与 `PATH` 同时指到时只留最先找到的那一条（顺序语义不变：
    // 留下来的那条就是**级别更高**的那条）。
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
    // 诊断走 stdout：这一行就是"界面上显示的值"的日志副本。
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

/// 版本低于 JDT LS 门槛时的**可读原因**（界面上那句话的 `{message}` 部分）。
///
/// 与 `jdtls.rs:834` 的错误文本同一条口径（`版本 {version} 低于 {MINIMUM_JAVA_MAJOR}`），
/// 但这里补上"这意味着什么"：`jdtls.rs:713-715` 在整条链路失败时说的是
/// "未找到 Java 21 或更新版本的 JDK，**JDT LS 无法启动**"。
/// 设置页要对齐的是后半句 —— 用户看到版本号不够，真正要预判的是"语言服务起不起得来"。
pub fn below_requirement_reason(version: &str) -> String {
    format!("版本 {version} 低于 {MINIMUM_JAVA_MAJOR}，语言服务无法启动")
}

/// 探测一个 JDK 主目录（或 `java` 可执行文件）：能用就给出 `home` + `version`。
///
/// ⚠️ **低于 ≥ 21 闸门的 JDK 仍然算"探测成功"**（`Ok`），只是
/// [`DetectedTool::meets_language_service`] 是 `false`：它确实装在这台机器上、版本也读得出来，
/// 界面上它应该出现在"检测到的安装"里，并且成为生效值时**如实标注**"语言服务无法启动"
/// （见 [`JdkDiscovery::effective`]）。把它直接当 `Err` 会丢掉版本号，
/// 页面就只能说"用不了"，用户无法预判。
///
/// 真正的 `Err` 只留给"根本用不了"：找不到 `java`、跑不起来、读不出版本
/// （与 `jdtls.rs:820-837` 的 `probe_java` 相比，这里**唯一**放宽的就是把版本闸门
/// 从"返回 Err"改成"带一个标志位"）。
fn probe_jdk(path: &str) -> Result<DetectedTool, String> {
    let path = Path::new(path);
    let executable = java_executable_in(path)
        .ok_or_else(|| format!("找不到 {}（查过 {}）", java_executable_name(), path.display()))?;
    let version = probe_java_version(&executable)?;
    Ok(DetectedTool {
        path: home_of(&executable)
            .map(|home| home.display().to_string())
            .unwrap_or_else(|| path.display().to_string()),
        meets_language_service: meets_language_service_requirement(&version),
        version,
    })
}

/// 探测一个 Maven 主目录（或 `mvn` 启动器）。
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
        // Maven 没有 JDT LS 的 Java 版本门槛（见 [`DetectedTool::meets_language_service`]）。
        meets_language_service: true,
    })
}

/// 一个候选的探测结果。
enum CandidateProbe {
    /// 探测成功。
    Found(DetectedJdk),
    /// 这个位置根本没有 `java`（`PATH` 上绝大多数目录就是这样）。
    Absent,
    /// 有 `java`，但它用不了（跑不起来 / 读不出版本）。
    Failed(String),
}

/// 一个候选 → 探测结论。
fn probe_java_candidate(candidate: &Candidate) -> CandidateProbe {
    let Some(executable) = java_executable_in(&candidate.path) else {
        return CandidateProbe::Absent;
    };
    match probe_java_version(&executable) {
        // ⚠️ 低于 ≥ 21 闸门的**也进候选列表**（版本读到了 = 这是一条可显示的事实），
        // 只是带一个 `meets_language_service = false` 的标志位；"自动生效值"的选取
        // （[`JdkDiscovery::detected`]）与"生效值标注"（[`JdkDiscovery::effective`]）都用它。
        // 这与 `jdtls.rs` 的差别只有一处：那边低于闸门直接丢弃候选，因为语言服务不需要它；
        // 设置页要把它**显示**出来，这正是本批要修的"显示 ≠ 实际"。
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

/// 这个候选的"没有 java"要不要记进 `rejected`（= 界面「已排除」那一句）。
///
/// `PATH` 上绝大多数目录里根本没有 `java` —— 那不是"一个失败的候选"，只是"这个目录不是 JDK"。
/// 全记下来的话，「未找到」那句话会变成几十条噪音（本机实测 68 条，把真正有用的"版本太低"
/// 淹掉了）。判据：
///
/// - **用户指名**的位置（显式覆写变量 / `JAVA_HOME`）必须记 —— 他指的地方没有 `java` 就是问题；
/// - 枚举出来的安装根：只有目录**存在**才记（不存在的根只说明这台机器没在那儿装 JDK）；
/// - `PATH` 上的目录：不记（那不是任何人的选择，`Missing` 这条每段路径都成立就没有信息量）。
fn absent_is_reportable(source: ToolSource, path: &Path) -> bool {
    match source {
        ToolSource::EnvOverride | ToolSource::JavaHome => true,
        ToolSource::InstalledRoot => path.is_dir(),
        ToolSource::MavenHome | ToolSource::Path => false,
    }
}

/// 一个 Maven 候选 → 一个探测成功的 Maven。
fn probe_maven_executable(
    executable: &str,
    source: ToolSource,
) -> Result<DetectedMaven, String> {
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

/// 在 `path`（JDK 主目录或 `java` 可执行文件本体）里找出 `java` 可执行文件。
fn java_executable_in(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    // Windows 上 `java.exe` 与无扩展名的 `java` 都试（照 `run.rs:1277-1285`）。
    for name in [java_executable_name(), "java"] {
        let candidate = path.join(BIN_DIRECTORY).join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// `java` 可执行文件的上一级的上一级 = JDK 主目录。
fn home_of(executable: &Path) -> Option<PathBuf> {
    executable.parent()?.parent().map(Path::to_path_buf)
}

/// 跑 `java -version`，取版本串。
fn probe_java_version(executable: &Path) -> Result<String, String> {
    let output = run_version(executable, &["-version"])?;
    java_version_text(&output).ok_or_else(|| "读不出版本号".to_string())
}

/// 跑一次 `-version` 并把 stdout + stderr 合起来。
///
/// `java -version` 一律写 stderr（JDK 8 起），`mvn -version` 写 stdout，
/// 所以两者都收（照 `jdtls.rs:571-576` 与 `run.rs` 的 `command_output`）。
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

/// 从 `java -version` 的输出里取版本串。
///
/// 两种写法都认（`run.rs:1822-1834` 与 `jdtls.rs:585-591` 的并集）：
/// `openjdk version "21.0.2" 2024-01-16`（带引号）与 `java version 1.8.0_402`（不带引号）。
pub fn java_version_text(output: &str) -> Option<String> {
    for line in output.lines() {
        let Some(index) = line.find("version") else {
            continue;
        };
        let rest = line[index + "version".len()..].trim();
        if let Some(quoted) = rest.strip_prefix('"') {
            return non_empty(quoted.split('"').next().unwrap_or_default());
        }
        return rest.split_whitespace().next().and_then(|value| non_empty(value));
    }
    None
}

/// 从 `mvn -version` 的输出里取版本号（`Apache Maven 3.9.9 (…)` → `3.9.9`）。
pub fn maven_version_text(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("Apache Maven")?;
        rest.split_whitespace().next().and_then(|value| non_empty(value))
    })
}

/// 从 `mvn -version` 的输出里取 `Maven home:`。
///
/// ⚠️ Maven 3.6 这一代把这一行打成 `<home>/bin/..`（真机实测：
/// `Maven home: D:\ProgramData\maven\apache-maven-3.6.3\bin\..`），所以过一遍
/// [`normalize_reported_path`]，否则界面上会出现一个带 `\..` 的路径。
pub fn maven_home_text(output: &str) -> Option<String> {
    labeled_line(output, "Maven home:").map(|value| normalize_reported_path(&value))
}

/// 去掉路径末尾的 `..` 段（`<home>/bin/..` → `<home>`）。
///
/// 只做字符串层面的归一化：不 `canonicalize`（那会在 Windows 上带出 `\\?\` verbatim 前缀，
/// 显示的路径会更难读），也不碰路径中间的部分。`..` 是 Maven 自己打出来的，剥掉它不改变含义。
pub fn normalize_reported_path(path: &str) -> String {
    let path = Path::new(path.trim());
    // 逐段走一遍：`..` 抵消上一层，抵消不掉就原样留着（相对路径的前导 `..`）。
    //
    // ⚠️ 不能写成"循环里剥两级父目录"：`X/bin/../..` 的第一个 `..` 抵消 `bin` 之后，
    // 第二个 `..` 要抵消的是 `X`，而"剥两级"在第二步会把 `..` 自己也算成一层。
    // 也不能用 `Path::file_name()` 判断结尾是不是 `..`：那条 API 在路径以 `..` 结尾时
    // 返回 `None`（`std::path` 的明确规定），会一段都剥不掉。
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

/// 从 `mvn -version` 的输出里取 `Java version:` 的版本号（逗号之前那一截）。
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

/// Maven 自己的两个 settings.xml 位置、用户选的那一份与本地仓库。
///
/// `settings_override` 是设置页选的那份 `settings.xml`（空串 / 全空白 = 没选）：
/// 选了就以它为**生效值**，本地仓库也从它那里读 `<localRepository>` —— 用户换了 settings.xml
/// 之后"本地仓库"那一行必须跟着变，否则页面会显示一份与实际生效文件无关的仓库路径。
///
/// `user_home` 拿不到时 `local_repository` 是 `None`（界面显示"未知"，不猜）。
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
        // 生效的那一份：用户选的优先（判据与 [`MavenConfiguration::effective_settings`] 同源）。
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

/// `<maven home>/conf/settings.xml`（存在时）。
///
/// 安装根的判据逐条照 Core `project/maven.rs:456-481`：配置的路径是目录就用它；
/// 否则**只有**它的父目录叫 `bin` 才算 `<home>/bin/mvn*` 布局，再往上一层才是安装根。
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

/// 本地仓库：用户级 settings.xml 里的 `<localRepository>`，否则 Maven 的默认位置。
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

/// 读一份 settings.xml 里的 `<localRepository>`（**纯文本扫描，不引 XML 依赖**）。
///
/// 规则只有三条，够用且不会读错：
/// 1. 只认元素文本 `<localRepository>…</localRepository>`；
/// 2. 内容里出现 `<` 视为"不是简单文本"→ 当作没写（嵌套内容不猜）；
/// 3. 自闭合 `<localRepository/>` 与空文本视为**没写**（Maven 会回落默认位置；
///    Core 自己的 `settings_with_local_repository` 测试 `tests/languages.rs:1369-1377` 是同一口径）。
///
/// ⚠️ **只读用户级 settings.xml**：Maven 文档里 `localRepository` 在**全局**（安装级）settings
/// 中不生效，所以安装级那份不参与判定。
pub fn read_local_repository(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    local_repository_in(&text)
}

/// [`read_local_repository`] 的纯解析部分（可直接单测）。
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

/// "一个都没探测到"时的那句话：把试过的位置与原因列出来，**不静默**。
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

/// 空串 / 只有空白 = 没设（有些 shell 会导出空值，照 `paths.rs` 的口径）。
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

    /// 三级候选的顺序与来源必须**逐条**固定：换顺序等于换"哪个 JDK 生效"。
    ///
    /// 这条是"来源那一栏说得对不对"的地基：本机 `PATH` 上的 `java` 是 1.8、
    /// `JAVA_HOME`（若设了）指向 openjdk-21，页面必须报后者。
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
        assert_eq!(candidates[0].path, PathBuf::from(r"D:\explicit\bin\java.exe"));
        assert_eq!(candidates[1].source, ToolSource::JavaHome);
        assert_eq!(
            candidates[1].path,
            PathBuf::from(r"D:\ProgramData\java\openjdk-21")
        );
        // PATH 有几段就有几个候选，且全部排在前两级之后（`split_paths` 是平台分隔符）。
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

    /// 常见安装根枚举：只收目录、每个根自己的候选在前、子目录按**字典序**（顺序必须确定，
    /// 因为"第一项就是生效值"）。
    ///
    /// 用临时目录造一棵假树（不碰真实机器上的 JDK 目录），所以这条测试在任何机器上都成立。
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
        // 安装包（文件）不能被当候选：那样会白跑一次 `java -version`。
        std::fs::write(program_data.join("java").join("jdk-8u45.exe"), "not a jdk")
            .expect("写文件失败");

        let env = EnvSnapshot {
            install_roots: vec![("ProgramData".to_string(), program_data.display().to_string())],
            ..EnvSnapshot::default()
        };
        let roots = installed_jdk_roots(&env);
        assert_eq!(
            roots,
            vec![
                // 根自己先来（`%ProgramData%\java` 本身就是个合法 JDK 家的情况）。
                program_data.join("java"),
                // 子目录按字典序：`jdk-11` 在 `openjdk-21` 前。
                program_data.join("java").join("jdk-11"),
                program_data.join("java").join("openjdk-21"),
            ]
        );

        // 候选要接在三级之后（`InstalledRoot` 是最后一级）。
        let candidates = java_candidates(&env);
        assert_eq!(candidates[0].source, ToolSource::InstalledRoot);
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.source == ToolSource::InstalledRoot),
            "这条 env 里没有前三级的任何一项"
        );

        // 目录不存在时安静地什么都不给（不是错误）。
        let env = EnvSnapshot {
            install_roots: vec![(
                "ProgramData".to_string(),
                base.join("nope").display().to_string(),
            )],
            ..EnvSnapshot::default()
        };
        assert_eq!(installed_jdk_roots(&env), vec![base.join("nope").join("java")]);

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 候选总数的收口：一个堆了几十个 JDK 的根不能让每次刷新变成几十次进程创建。
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
            install_roots: vec![("ProgramData".to_string(), base.join("ProgramData").display().to_string())],
            ..EnvSnapshot::default()
        };
        let roots = installed_jdk_roots(&env);
        assert_eq!(roots.len(), MAX_INSTALLED_JDK_CANDIDATES);
        // 截掉的永远是末尾那些（前面的顺序语义不变）。
        assert_eq!(roots[0], java);
        assert_eq!(roots[1], java.join("jdk-000"));

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 只有 PATH 时第一个候选就是 PATH 上的第一个目录（= 来源必须报 `来自 PATH`）。
    #[test]
    fn java_candidates_degrade_to_path_only() {
        let env = EnvSnapshot {
            path: Some(joined_path(&[r"C:\Windows", r"C:\jdk8\bin"])),
            ..EnvSnapshot::default()
        };
        let candidates = java_candidates(&env);
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().all(|item| item.source == ToolSource::Path));
        assert_eq!(candidates[0].path, PathBuf::from(r"C:\Windows"));
    }

    /// 用**平台分隔符**拼一份 `PATH`（`;` / `:` 的选择是平台的事，测试不该写死）。
    fn joined_path(parts: &[&str]) -> String {
        std::env::join_paths(parts.iter().map(PathBuf::from))
            .expect("测试用的 PATH 片段不含平台分隔符")
            .to_string_lossy()
            .into_owned()
    }

    /// 空串环境变量 = 没设（不许造出一个"空路径候选"）。
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

    /// 「未找到」那句话的 `rejected` 清单**不能是 PATH 上每个目录一条**
    /// （本机真机实测 68 条噪音，把"版本太低"这类真原因淹掉了）。
    ///
    /// 判据：用户指名的地方（覆写变量 / `JAVA_HOME`）要记；枚举出来的安装根只在**目录存在**时记；
    /// `PATH` 目录不记。
    #[test]
    fn absent_candidates_are_only_reported_where_it_means_something() {
        let path = Path::new(r"C:\Windows");
        assert!(absent_is_reportable(ToolSource::EnvOverride, path));
        assert!(absent_is_reportable(ToolSource::JavaHome, path));
        assert!(!absent_is_reportable(ToolSource::Path, path));
        assert!(!absent_is_reportable(ToolSource::MavenHome, path));

        // 枚举出来的根：存在的目录要记（"这儿有个装了一半的 JDK"是有用信息），
        // 不存在的根不记（只说明这台机器没在那儿装）。
        let existing = std::env::temp_dir();
        assert!(absent_is_reportable(ToolSource::InstalledRoot, &existing));
        assert!(!absent_is_reportable(
            ToolSource::InstalledRoot,
            &existing.join("lithe-no-such-root-ever")
        ));
    }

    /// Maven 3.6 的 `Maven home: ...\bin\..` 要被归一化，普通路径原样保留。
    ///
    /// 用**平台原生**的绝对路径拼输入（`/opt` 在 Windows 上会被渲染成 `\opt`，
    /// 写死 POSIX 字面量就不是跨平台测试了）。
    #[test]
    fn maven_home_drops_the_trailing_parent_segment() {
        // `temp_dir()` 在 Windows 上带尾部分隔符（`…\Temp\`），所以再 `join` 一层，
        // 让期望值与归一化后的形态一致（归一化会去掉尾部分隔符）。
        let base = std::env::temp_dir().join("lithe-normalize-base");
        let messy = base.join("bin").join("..");
        assert_eq!(
            normalize_reported_path(&messy.to_string_lossy()),
            base.display().to_string()
        );
        // 连续两个 `..`：第一个抵消 `bin`，第二个抵消 `base` 自己。
        let messy = base.join("bin").join("..").join("..");
        let expected = base.parent().expect("临时目录一定有父目录");
        assert_eq!(
            normalize_reported_path(&messy.to_string_lossy()),
            expected.display().to_string()
        );
        // 普通路径原样保留（用平台原生路径，不写死 POSIX 字面量）。
        let clean = base.join("apache-maven-3.9.9");
        assert_eq!(
            normalize_reported_path(&clean.to_string_lossy()),
            clean.display().to_string()
        );
        assert_eq!(
            normalize_reported_path(&format!("  {}  ", clean.display())),
            clean.display().to_string()
        );
        // 走到根就停（不 panic、不越界）；纯 `..` 保留成相对路径。
        assert_eq!(normalize_reported_path(".."), "..");

        // 端到端：真机那条 `mvn -version` 输出（Windows 上的真实形态）。
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

    /// 且候选名四个都在（Windows 上真正的启动器是 `mvn.cmd`）。
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

    /// `java -version` 的两种真实写法都要读出版本（1.8 那种不带引号）。
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
        // 读不出来就是 `None`（界面会显式说"读不出版本号"，不编一个）
        assert_eq!(java_version_text("no banner here"), None);
    }

    /// `mvn -version` 的三样事实（版本 / 安装目录 / 运行时 JDK）。
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

    /// `<localRepository>` 的三种写法：写了 / 自闭合 / 空元素。
    #[test]
    fn local_repository_parsing_handles_the_documented_shapes() {
        assert_eq!(
            local_repository_in(
                "<settings><localRepository>D:\\repo</localRepository></settings>"
            ),
            Some(r"D:\repo".to_string())
        );
        assert_eq!(
            local_repository_in("<settings><localRepository>  /opt/repo  </localRepository></settings>"),
            Some("/opt/repo".to_string())
        );
        // 自闭合与空元素都 = "没写"（回落 Maven 默认位置）。
        assert_eq!(local_repository_in("<settings><localRepository/></settings>"), None);
        assert_eq!(
            local_repository_in("<settings><localRepository></localRepository></settings>"),
            None
        );
        // 嵌套内容不猜。
        assert_eq!(
            local_repository_in("<settings><localRepository><a>x</a></localRepository></settings>"),
            None
        );
        assert_eq!(local_repository_in("<settings/>"), None);
    }

    /// 安装级 settings.xml 的推导：只有 `<home>/bin/mvn*` 才算安装根
    /// （逐条照 Core `project/maven.rs:465-481`）。
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

        // 目录形式（用户填了 Maven 主目录）同样成立。
        assert_eq!(
            installation_settings_path(&home.to_string_lossy()),
            Some(home.join("conf").join("settings.xml"))
        );

        // 不是安装布局（父目录不叫 bin）→ 不猜。
        let wrapper = home.join("mvnw.cmd");
        assert_eq!(installation_settings_path(&wrapper.to_string_lossy()), None);

        let _ = std::fs::remove_dir_all(&home);
    }

    /// 本地仓库：settings.xml 写了就报它的值（来源 = settings.xml），
    /// 没写就报 Maven 的默认位置（来源 = Maven 默认），主目录都没有时是 `None`（界面显示"未知"）。
    #[test]
    fn local_repository_falls_back_to_the_maven_default() {
        let directory = std::env::temp_dir().join(format!(
            "lithe-settings-localrepo-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("建临时目录失败");
        let home = directory.join("home");
        std::fs::create_dir_all(home.join(M2_DIRECTORY)).expect("建 .m2 失败");

        // ① 没有 settings.xml → 默认位置。
        let maven = MavenDiscovery::default();
        let configuration = maven_configuration(&maven, Some(&home.to_string_lossy()), None);
        assert_eq!(configuration.user_settings, None);
        assert!(!configuration.settings_is_overridden());
        let repository = configuration.local_repository.expect("必须给出默认位置");
        assert_eq!(
            repository.path,
            home.join(M2_DIRECTORY).join(REPOSITORY_DIRECTORY).display().to_string()
        );
        assert_eq!(repository.source, LocalRepositorySource::MavenDefault);

        // ② 有 settings.xml 且写了 `<localRepository>` → 用它的值。
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

        // ③ 连用户主目录都没有 → 不猜，`None`。
        let configuration = maven_configuration(&maven, None, None);
        assert!(configuration.local_repository.is_none());

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// 用户选了一份 settings.xml：它成为**生效值**，本地仓库也改从它那里读 ——
    /// 而且"用户级"那一格仍然报自动检测到的那份（两个事实不能互相冒充）。
    ///
    /// 保护的是"选了 settings.xml 但本地仓库那一行没跟着变"这类**看起来生效、实际没生效**：
    /// 那份文件会被交给语言服务去替代自动检测结果，所以本地仓库必须同源。
    #[test]
    fn a_selected_settings_file_wins_and_drives_the_local_repository() {
        let directory = std::env::temp_dir().join(format!(
            "lithe-settings-selected-settings-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let home = directory.join("home");
        std::fs::create_dir_all(home.join(M2_DIRECTORY)).expect("建 .m2 失败");
        // 自动检测到的那份：仓库是 auto-repo。
        std::fs::write(
            home.join(M2_DIRECTORY).join(SETTINGS_FILE_NAME),
            "<settings><localRepository>D:\\auto-repo</localRepository></settings>",
        )
        .expect("写用户级 settings.xml 失败");
        // 用户选的那份：仓库是 picked-repo。
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
        // 用户级那一格仍然是自动检测到的那份（不是被选中的那份顶替）。
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

        // 选了一份**不存在**的文件：生效值仍然是它（界面要如实显示"你选的是这个"），
        // 但存在性标志为真 → 界面画失败色；本地仓库退回 Maven 默认位置（读不到就是没写）。
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

        // 空 / 全空白 = 没选（与 `Overrides` 同一口径，否则探测层会拿空路径去查文件系统）。
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

    /// 覆盖值非空且可用 → 模式是「已选择」，**不带来源**（真源 `effective-toolchain.ts:52`）。
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

    /// 自动生效时来源与版本一起报（这是"来源那一栏说得对不对"的直接判据）。
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
        // 第二个候选也留在"检测到的安装"里（信息量：PATH 上还有 1.8）。
        assert_eq!(discovery.candidates.len(), 2);
    }

    /// 一条"检测到的 JDK"样例；`meets_language_service` 一律**按版本现算**
    /// （测试里不手写这个标志位，否则闸门与解析改动时测试会继续绿着说谎）。
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

    /// 覆盖值填了但跑不起来 → `Unusable` + 原因；**不会**悄悄退回自动值。
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
                // 路径不存在这一类**不是**版本闸门问题：界面不许补一句关于版本的话（那是编造）。
                assert!(!language_service_rejection);
            }
            other => panic!("覆盖值用不了时必须报 Unusable，实际 {other:?}"),
        }
    }

    /// 一个都没有 → `NotFound` + "试过哪些、为什么不行"（界面上是可排查的一句话）。
    #[test]
    fn nothing_detected_reports_what_was_tried() {
        let discovery = JdkDiscovery {
            overridden: None,
            candidates: Vec::new(),
            rejected: vec![
                r"C:\jdk8\bin\java.exe（版本 1.8.0_402 低于 21）".to_string(),
            ],
        };
        let EffectiveToolchain::NotFound { reason } = discovery.effective() else {
            panic!("没有候选时必须报 NotFound");
        };
        assert!(reason.contains(JAVA_EXECUTABLE_ENV), "{reason}");
        assert!(reason.contains(JAVA_HOME_ENV), "{reason}");
        assert!(reason.contains(PATH_ENV), "{reason}");
        assert!(reason.contains("已排除"), "{reason}");
    }

    /// Maven 那一行的 JDK：没单独设 → **跟随 JDK 那一行的生效值**，模式是「使用项目 JDK」。
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
    /// 诊断行必须含任务书要求的四个 token，且**每个值都能在行里找到**。
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
        // 任务书要求的四个 token 按顺序出现。
        let jdk_at = line.find("jdk=").expect("缺 jdk=");
        let source_at = line.find("source=").expect("缺 source=");
        let maven_at = line.find("maven=").expect("缺 maven=");
        let local_repo_at = line.find("localRepo=").expect("缺 localRepo=");
        assert!(jdk_at < source_at && source_at < maven_at && maven_at < local_repo_at, "{line}");
        // 界面上会显示的每个值都在行里。
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
            // 没选 settings.xml 时"生效值不是选来的"，且两个新覆盖值都是空串。
            "settingsOverridden=no",
            "overrideSettings= overrideLocalRepo=",
        ] {
            assert!(line.contains(expected), "诊断行缺 {expected}：{line}");
        }

        // 选了 settings.xml：生效值换成它、`settingsOverridden=yes`、本地仓库随之改写
        // （本地仓库的推导由 `a_selected_settings_file_wins_and_drives_the_local_repository` 覆盖，
        //  这里只钉诊断行确实把"谁在生效"说了出来）。
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

    /// 三个覆盖值全空 = 全自动；**两边都没有可用生效值**时才允许页面打那句整页提示。
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

        // 只有 Maven 可用 → 不打整页提示（Maven 那一格有真值）。
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

        // 覆盖值填了但**用不了** → 也不算空（页面要画出"用不了 + 原因"）。
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

    /// 来源标签：`JAVA_HOME` / `PATH` 用真源既有键；两个"环境变量"来源用带 `{name}` 的新键。
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
        assert_eq!(ToolSource::EnvOverride.label_arg(), Some(JAVA_EXECUTABLE_ENV));
        assert_eq!(ToolSource::MavenHome.label_arg(), Some(MAVEN_HOME_ENV));
        // 三个模式的文案键必须是真源既有的那三条。
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

    // ---- A1（≥ 21 闸门）：页面显示必须能预判"语言服务起不起得来" ----

    /// 主版本解析逐条对齐 `jdtls.rs:848-859` 的 `major_version`（本模块只做镜像）。
    ///
    /// 三种命名都要认：`1.8.0_402`（2006 年前的旧命名，主版本是**第二段**）、
    /// `21.0.2`（现代命名）、`25`（只有主版本）。
    #[test]
    fn jdk_major_version_mirrors_the_jdtls_reading() {
        assert_eq!(jdk_major_version("1.8.0_402"), 8);
        assert_eq!(jdk_major_version("1.8.0_221"), 8);
        assert_eq!(jdk_major_version("21.0.2"), 21);
        assert_eq!(jdk_major_version("21.0.2+13-LTS"), 21);
        assert_eq!(jdk_major_version("25"), 25);
        // 读不出来 → 0 ⇒ 判定为"不满足"（与 `jdtls.rs` 的 `unwrap_or(0)` 同一条口径）。
        assert_eq!(jdk_major_version("not a version"), 0);
        assert_eq!(jdk_major_version(""), 0);
    }

    /// 闸门数值与 `jdtls.rs:80` 的 `MINIMUM_JAVA_MAJOR` 必须相等，且正好卡在 21。
    ///
    /// 这条是"两处数值不许各走各的"的守卫：`jdtls.rs` 那边改了下限而这里没跟，
    /// 页面就会重新变成"显示 ≠ 实际"。
    #[test]
    fn the_gate_is_the_jdtls_minimum() {
        assert_eq!(MINIMUM_JAVA_MAJOR, 21);
        assert!(meets_language_service_requirement("21.0.8"));
        assert!(meets_language_service_requirement("22"));
        assert!(!meets_language_service_requirement("17.0.16"));
        assert!(!meets_language_service_requirement("1.8.0_221"));
    }

    /// **自动发现这条路**：低于闸门的 JDK 仍然是"检测到的安装"，但一旦没有过闸门的候选，
    /// 生效值必须如实标成"用不了 + 语言服务无法启动"，而**不是**显示成可用的 1.8
    /// （这正是实机取证到的 `自动 → JDK 1.8.0_221` 那条假显示）。
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
        // 一个都不过闸门 ⇒ 没有"自动生效值"。
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
        // 两条候选都还在"检测到的安装"里（版本是读到的事实，不许丢掉）。
        assert_eq!(discovery.candidates.len(), 2);
        // 诊断行要能把这件事说清楚，供 `S1_JAVA_JDTLS javaVersion=…` 逐项对照。
        assert!(discovery
            .candidates
            .iter()
            .all(|jdk| !jdk.meets_language_service));
    }

    /// **自动发现这条路（混合）**：候选里有一个过闸门的，生效值就是**那一个**
    /// （与 `jdtls.rs` 的 `resolve_runtime` "丢弃过不去的、继续往下找"逐条一致）。
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

    /// **覆盖值这条路**：填 `openjdk-17`（真实存在、能跑、只是低于门槛）时，
    /// 页面必须标出"低于语言服务最低要求 21，语言服务无法启动"，
    /// **不是**「已选择 → JDK 17.0.16」这种看起来可用的显示。
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
                // 用户填的原文要回显（否则"用不了"没说清是哪个值用不了）。
                assert_eq!(path, r"D:\ProgramData\java\openjdk-17");
                assert!(reason.contains("17.0.16"), "{reason}");
                assert!(reason.contains("21"), "{reason}");
                assert!(reason.contains("语言服务无法启动"), "{reason}");
                assert!(language_service_rejection);
            }
            other => panic!("低于闸门的覆盖值必须报 Unusable，实际 {other:?}"),
        }
        // ⚠️ **不回落**到那个可用的 21：用户明确选了 17，页面要报"你选的那个不行"，
        // 而不是悄悄换一个（与"覆盖值用不了不退回自动值"同一条既有口径）。
        assert_ne!(
            discovery.effective().diagnostic_value(),
            r"D:\ProgramData\java\openjdk-21"
        );
    }

    /// 覆盖值正好在闸门上（21）→ 照旧是「已选择」（闸门是 `>=` 不是 `>`）。
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

    /// 闸门数值与结论都要在那一行诊断里（否则"页面显示得对"这件事在日志里核不出来，
    /// 也就无法与 `S1_JAVA_JDTLS javaVersion=…` 逐项对照）。
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
        // ⚠️ 低于闸门时**也必须报出版本号**：它是页面那句话的一部分，也是与
        // `S1_JAVA_JDTLS javaVersion=…` 对照的锚点（`jdk_version` 从 `reason` 里抠出来）。
        assert!(line.contains("version=1.8.0_221"), "{line}");
        // `Unusable` 那一档的 `jdk=` 取值是 `unusable(<路径>)`（[`EffectiveToolchain::diagnostic_value`]）。
        assert!(line.contains(r"jdk=unusable(C:\jdk8)"), "{line}");

        // 过闸门时结论换成 `ready`。
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
