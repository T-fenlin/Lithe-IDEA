//! 工作区配置目录 `.lithe/` 的路径真源。
//!
//! ## 为什么要有这个模块
//!
//! 审计确认：`.lithe` 这个字面量在**整个仓库里没有任何具名常量**（十几处裸字面量：
//! `root.join(".lithe")`、`appendingPathComponent(".lithe/run/local.json")` …），
//! 这违反仓库自己的"稳定常量集中命名"规则。本模块给 gpui 侧一个**唯一**的命名与构造点。
//!
//! 本模块只定常量与路径构造，**不替换**既有实现里的裸字面量（那会把改动面摊到无关文件上）；
//! 新写的代码必须走这里。Core / macOS / Windows 各自的 `.lithe` 解析暂时保持原样，
//! 收敛它们属于跨端契约的后续工作。
//!
//! ## 层的划分决定了文件在哪（见设计 Note 第三节）
//!
//! ```text
//! .lithe/
//! ├── project.json                共享  version / id
//! ├── settings.json               共享  工作区对全局键的覆盖
//! ├── git.json                    共享  vcs / roots / submodules
//! ├── run/{configurations,generated}.json   共享
//! ├── toolchains/requirements.json          共享
//! ├── maven/config.json                     共享
//! ├── lsp/language-providers.json           共享
//! ├── settings.local.json         本机
//! ├── run.local.json              本机
//! ├── maven.local.json            本机
//! ├── toolchains.local.json       本机
//! └── session.local.json          本机
//! ```
//!
//! **本机层用后缀（`.local.json`）而不是目录位置表达归属**：这样"这个文件该不该提交"
//! 看文件名就知道，`.gitignore` 也只需要一条 `*.local.json`。

use std::path::{Path, PathBuf};

/// 工作区配置目录名。
pub const WORKSPACE_CONFIG_DIRECTORY: &str = ".lithe";

/// 本机层文件的后缀。带这个后缀的文件**绝不能**进版本控制。
pub const LOCAL_LAYER_SUFFIX: &str = ".local.json";

/// 派生物目录名（独立 Java 编译产物）：它在本机层之上，连共享层都不是。
pub const RUN_CLASSES_DIRECTORY: &str = "classes";

/// 本机排除文件里的**唯一**条目：整棵工作区配置目录。
///
/// 由 [`WORKSPACE_CONFIG_DIRECTORY`] 派生，不写第二遍字面量。
///
/// ⚠️ **故意不做 root-anchor**（不是 `/.lithe/`）：`/`-锚定要求先知道仓库根，而工作区根
/// 可能是仓库的子目录（那时正确的锚定形态是 `/backend/.lithe/`）。不锚定的 `.lithe/`
/// 在两种情况下都成立，代价是同一个仓库里嵌套的其它 `.lithe` 目录也会被隐藏 ——
/// 对"工作区配置默认不共享"这条口径来说这是可接受的，而且用户随时可以让
/// `sharing::share_project_config` 把这一行精确移除。
pub fn workspace_config_exclude_pattern() -> String {
    format!("{WORKSPACE_CONFIG_DIRECTORY}/")
}

/// 一个工作区根的配置路径集合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceConfigPaths {
    root: PathBuf,
}

impl WorkspaceConfigPaths {
    /// 以工作区根（用户打开的那个目录）构造。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 工作区根。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<root>/.lithe`
    pub fn directory(&self) -> PathBuf {
        self.root.join(WORKSPACE_CONFIG_DIRECTORY)
    }

    /// `<root>/.lithe/project.json`：工作区身份清单（共享层）。
    pub fn project_manifest(&self) -> PathBuf {
        self.directory().join("project.json")
    }

    /// `<root>/.lithe/settings.json`：工作区对全局键的覆盖（共享层）。
    pub fn settings(&self) -> PathBuf {
        self.directory().join("settings.json")
    }

    /// `<root>/.lithe/settings.local.json`：个人的工作区覆盖（本机层）。
    pub fn settings_local(&self) -> PathBuf {
        self.directory().join("settings.local.json")
    }

    /// `<root>/.lithe/git.json`：团队共享的仓库结构事实（共享层）。
    pub fn git(&self) -> PathBuf {
        self.directory().join("git.json")
    }

    /// `<root>/.lithe/run/configurations.json`：团队运行配置（共享层）。
    pub fn run_configurations(&self) -> PathBuf {
        self.directory().join("run").join("configurations.json")
    }

    /// `<root>/.lithe/run/generated.json`：识别产物（共享层）。
    pub fn run_generated(&self) -> PathBuf {
        self.directory().join("run").join("generated.json")
    }

    /// `<root>/.lithe/run.local.json`：运行配置的本机覆盖（本机层）。
    pub fn run_local(&self) -> PathBuf {
        self.directory().join("run.local.json")
    }

    /// `<root>/.lithe/toolchains/requirements.json`：这个项目需要什么工具链（共享层）。
    pub fn toolchain_requirements(&self) -> PathBuf {
        self.directory()
            .join("toolchains")
            .join("requirements.json")
    }

    /// `<root>/.lithe/toolchains.local.json`：本机实际用哪个可执行文件（本机层）。
    pub fn toolchain_local(&self) -> PathBuf {
        self.directory().join("toolchains.local.json")
    }

    /// `<root>/.lithe/maven/config.json`：团队 Maven 便携配置（共享层）。
    pub fn maven_config(&self) -> PathBuf {
        self.directory().join("maven").join("config.json")
    }

    /// `<root>/.lithe/maven.local.json`：`settings.xml` / 本地仓库的本机覆盖（本机层）。
    pub fn maven_local(&self) -> PathBuf {
        self.directory().join("maven.local.json")
    }

    /// `<root>/.lithe/lsp/language-providers.json`：手写的 provider 覆盖（共享层）。
    pub fn language_providers(&self) -> PathBuf {
        self.directory().join("lsp").join("language-providers.json")
    }

    /// `<root>/.lithe/session.local.json`：会话状态（本机层，可丢弃）。
    pub fn session_local(&self) -> PathBuf {
        self.directory().join("session.local.json")
    }

    /// `<root>/.lithe/.gitignore`：一旦共享，用它挡住本机成员（共享层）。
    pub fn ignore_file(&self) -> PathBuf {
        self.directory().join(".gitignore")
    }
}

/// 把绝对路径转成**工作区根相对**的路径，分隔符统一成 `/`（Core 的 `paths` 口径）。
///
/// 不在 `root` 之下时返回 `None`。
pub fn workspace_relative(root: &Path, absolute: &Path) -> Option<String> {
    let relative = absolute.strip_prefix(root).ok()?;
    let text = relative.to_string_lossy().replace('\\', "/");
    if text.is_empty() { None } else { Some(text) }
}

/// 工作区配置目录**内部**的相对路径是否属于"可共享层"。
///
/// 参数是相对 `.lithe/` 的路径（`run/configurations.json`、`settings.local.json` …），
/// 用 `/` 或 `\` 分隔都可以。规则只有三条，全部是"绝不能提交"的排除项：
///
/// - 带 `.local.json` 后缀 → 本机层，不能提交；
/// - `.tmp` → 原子写的中间产物；
/// - [`is_pruned_directory`] 之下的任何东西 → 编译产物。
pub fn is_shareable_member(config_relative: &str) -> bool {
    let normalized = config_relative.replace('\\', "/");
    let normalized = normalized.trim_start_matches('/');
    if normalized.is_empty() || is_pruned_directory(normalized) {
        return false;
    }
    let file_name = normalized.rsplit('/').next().unwrap_or(normalized);
    !file_name.ends_with(LOCAL_LAYER_SUFFIX) && !file_name.ends_with(".tmp")
}

/// 目录遍历时要**整个跳过**的子树。
///
/// 目前只有 `run/classes`：它是独立 Java 编译产物，可能很大，而且里面没有一个可共享成员。
/// 单独抽出来是因为"过滤成员"和"剪枝遍历"是两件事 —— 只在成员判定里排除它，遍历仍然会走进去。
pub fn is_pruned_directory(config_relative: &str) -> bool {
    let normalized = config_relative.replace('\\', "/");
    let normalized = normalized.trim_start_matches('/');
    let mut components = normalized.split('/');
    components.next() == Some("run") && components.next() == Some(RUN_CLASSES_DIRECTORY)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths() -> WorkspaceConfigPaths {
        WorkspaceConfigPaths::new(PathBuf::from("D:\\work\\demo"))
    }

    /// 每个成员都落在 `.lithe/` 之下，且本机层用的就是 `.local.json` 后缀。
    #[test]
    fn members_live_under_the_config_directory() {
        let paths = paths();
        let dir = paths.directory();
        assert_eq!(dir, PathBuf::from("D:\\work\\demo\\.lithe"));

        for member in [
            paths.project_manifest(),
            paths.settings(),
            paths.git(),
            paths.run_configurations(),
            paths.run_generated(),
            paths.toolchain_requirements(),
            paths.maven_config(),
            paths.language_providers(),
            paths.ignore_file(),
        ] {
            assert!(member.starts_with(&dir), "{member:?} 不在配置目录下");
            assert!(
                is_shareable_member(
                    member
                        .strip_prefix(&dir)
                        .expect("应在配置目录下")
                        .to_string_lossy()
                        .as_ref()
                ),
                "{member:?} 应当是共享层成员"
            );
        }

        for member in [
            paths.settings_local(),
            paths.run_local(),
            paths.toolchain_local(),
            paths.maven_local(),
            paths.session_local(),
        ] {
            assert!(member.starts_with(&dir), "{member:?} 不在配置目录下");
            assert!(
                !is_shareable_member(
                    member
                        .strip_prefix(&dir)
                        .expect("应在配置目录下")
                        .to_string_lossy()
                        .as_ref()
                ),
                "{member:?} 属于本机层，不能共享"
            );
        }
    }

    /// 可共享判定：本机层 / 临时文件 / 编译产物一律不可共享，其余可以。
    #[test]
    fn shareable_members_exclude_local_and_derived_files() {
        for shareable in [
            "project.json",
            "settings.json",
            "git.json",
            ".gitignore",
            "run/configurations.json",
            "run/generated.json",
            "toolchains/requirements.json",
            "maven/config.json",
            "lsp/language-providers.json",
        ] {
            assert!(is_shareable_member(shareable), "{shareable} 应当可共享");
        }

        for private in [
            "settings.local.json",
            "run.local.json",
            "maven.local.json",
            "toolchains.local.json",
            "session.local.json",
            "run/classes/java-main-Standalone/App.class",
            "settings.json.tmp",
            "run/configurations.json.tmp",
            "",
        ] {
            assert!(!is_shareable_member(private), "{private} 绝不能共享");
        }

        // Windows 分隔符与开头斜杠都要能处理。
        assert!(is_shareable_member("run\\configurations.json"));
        assert!(!is_shareable_member("/run\\classes\\x.class"));
    }

    /// 剪枝目录判定：它管的是"遍历要不要走进去"，比成员判定更宽（整棵子树）。
    #[test]
    fn pruned_directories_are_whole_subtrees() {
        assert!(is_pruned_directory("run/classes"));
        assert!(is_pruned_directory("run/classes/java-main-Standalone"));
        assert!(is_pruned_directory("run\\classes\\deep\\nested"));
        assert!(!is_pruned_directory("run"));
        assert!(!is_pruned_directory("run/configurations.json"));
        assert!(!is_pruned_directory("maven/classes"));
    }

    /// 排除条目由目录名派生，不写第二遍字面量。
    #[test]
    fn exclude_pattern_comes_from_the_directory_name() {
        assert_eq!(workspace_config_exclude_pattern(), ".lithe/");
        assert_eq!(
            workspace_config_exclude_pattern(),
            format!("{WORKSPACE_CONFIG_DIRECTORY}/")
        );
    }

    /// 相对路径统一用 `/`，且不在根之下时返回 `None`。
    #[test]
    fn workspace_relative_normalizes_separators() {
        let root = PathBuf::from("D:\\work\\demo");
        assert_eq!(
            workspace_relative(
                &root,
                &PathBuf::from("D:\\work\\demo\\.lithe\\project.json")
            ),
            Some(".lithe/project.json".to_string())
        );
        assert_eq!(
            workspace_relative(&root, &PathBuf::from("D:\\work\\other\\.lithe")),
            None
        );
        assert_eq!(workspace_relative(&root, &root), None);
    }
}
