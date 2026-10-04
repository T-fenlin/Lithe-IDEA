use std::path::{Path, PathBuf};

pub const WORKSPACE_CONFIG_DIRECTORY: &str = ".lithe";

pub const LOCAL_LAYER_SUFFIX: &str = ".local.json";

pub const RUN_CLASSES_DIRECTORY: &str = "classes";

pub fn workspace_config_exclude_pattern() -> String {
    format!("{WORKSPACE_CONFIG_DIRECTORY}/")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceConfigPaths {
    root: PathBuf,
}

impl WorkspaceConfigPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn directory(&self) -> PathBuf {
        self.root.join(WORKSPACE_CONFIG_DIRECTORY)
    }

    pub fn project_manifest(&self) -> PathBuf {
        self.directory().join("project.json")
    }

    pub fn settings(&self) -> PathBuf {
        self.directory().join("settings.json")
    }

    pub fn settings_local(&self) -> PathBuf {
        self.directory().join("settings.local.json")
    }

    pub fn git(&self) -> PathBuf {
        self.directory().join("git.json")
    }

    pub fn run_configurations(&self) -> PathBuf {
        self.directory().join("run").join("configurations.json")
    }

    pub fn run_generated(&self) -> PathBuf {
        self.directory().join("run").join("generated.json")
    }

    pub fn run_local(&self) -> PathBuf {
        self.directory().join("run.local.json")
    }

    pub fn run_local_legacy(&self) -> PathBuf {
        self.directory().join("run").join("local.json")
    }

    pub fn toolchain_requirements(&self) -> PathBuf {
        self.directory()
            .join("toolchains")
            .join("requirements.json")
    }

    pub fn toolchain_local(&self) -> PathBuf {
        self.directory().join("toolchains.local.json")
    }

    pub fn maven_config(&self) -> PathBuf {
        self.directory().join("maven").join("config.json")
    }

    pub fn maven_local(&self) -> PathBuf {
        self.directory().join("maven.local.json")
    }

    pub fn language_providers(&self) -> PathBuf {
        self.directory().join("lsp").join("language-providers.json")
    }

    pub fn session_local(&self) -> PathBuf {
        self.directory().join("session.local.json")
    }

    pub fn ignore_file(&self) -> PathBuf {
        self.directory().join(".gitignore")
    }
}

pub fn workspace_relative(root: &Path, absolute: &Path) -> Option<String> {
    let relative = absolute.strip_prefix(root).ok()?;
    let text = relative.to_string_lossy().replace('\\', "/");
    if text.is_empty() { None } else { Some(text) }
}

pub fn is_shareable_member(config_relative: &str) -> bool {
    let normalized = config_relative.replace('\\', "/");
    let normalized = normalized.trim_start_matches('/');
    if normalized.is_empty() || is_pruned_directory(normalized) {
        return false;
    }
    let file_name = normalized.rsplit('/').next().unwrap_or(normalized);
    !file_name.ends_with(LOCAL_LAYER_SUFFIX) && !file_name.ends_with(".tmp")
}

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

        assert!(is_shareable_member("run\\configurations.json"));
        assert!(!is_shareable_member("/run\\classes\\x.class"));
    }

    #[test]
    fn pruned_directories_are_whole_subtrees() {
        assert!(is_pruned_directory("run/classes"));
        assert!(is_pruned_directory("run/classes/java-main-Standalone"));
        assert!(is_pruned_directory("run\\classes\\deep\\nested"));
        assert!(!is_pruned_directory("run"));
        assert!(!is_pruned_directory("run/configurations.json"));
        assert!(!is_pruned_directory("maven/classes"));
    }

    #[test]
    fn exclude_pattern_comes_from_the_directory_name() {
        assert_eq!(workspace_config_exclude_pattern(), ".lithe/");
        assert_eq!(
            workspace_config_exclude_pattern(),
            format!("{WORKSPACE_CONFIG_DIRECTORY}/")
        );
    }

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
