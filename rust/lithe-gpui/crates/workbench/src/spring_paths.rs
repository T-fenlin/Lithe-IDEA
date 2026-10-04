use std::path::{Component, Path, PathBuf};

pub const MAX_SPRING_PATHS: usize = 5_000;

pub const MAX_SCANNED_ENTRIES: usize = 200_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpringSourcePaths {
    pub paths: Vec<String>,
    pub truncated: bool,
}

pub fn collect_spring_source_paths(root: &Path) -> SpringSourcePaths {
    collect_bounded(root, MAX_SPRING_PATHS, MAX_SCANNED_ENTRIES)
}

fn collect_bounded(root: &Path, max_paths: usize, max_entries: usize) -> SpringSourcePaths {
    let mut candidates: Vec<String> = Vec::new();
    let mut truncated = false;
    let mut scanned = 0usize;
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    'walk: while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        let mut children: Vec<(PathBuf, bool, bool)> = Vec::new();
        for entry in entries {
            scanned += 1;
            if scanned > max_entries {
                truncated = true;
                break 'walk;
            }
            let Ok(entry) = entry else { continue };
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            children.push((entry.path(), kind.is_dir(), kind.is_file()));
        }

        children.sort_by(|left, right| left.0.cmp(&right.0));
        let mut directories: Vec<PathBuf> = Vec::new();
        for (child, is_dir, is_file) in children {
            if is_dir {
                if !is_excluded_directory(&child) {
                    directories.push(child);
                }
                continue;
            }
            if !is_file {
                continue;
            }
            let Some(relative) = workspace_relative_path(root, &child) else {
                continue;
            };
            if !is_spring_index_path(&relative) {
                continue;
            }
            candidates.push(relative);
            if candidates.len() > max_paths {
                truncated = true;
                break 'walk;
            }
        }
        for directory in directories.into_iter().rev() {
            stack.push(directory);
        }
    }

    candidates.sort();
    candidates.truncate(max_paths);
    SpringSourcePaths {
        paths: candidates,
        truncated,
    }
}

fn is_spring_index_path(relative: &str) -> bool {
    let name = relative
        .rsplit('/')
        .next()
        .unwrap_or(relative)
        .to_ascii_lowercase();
    if name.ends_with(".java") {
        return true;
    }
    if name == "spring-configuration-metadata.json"
        || name == "additional-spring-configuration-metadata.json"
    {
        return true;
    }
    is_spring_configuration_file(&name)
}

fn is_spring_configuration_file(name: &str) -> bool {
    if name == "application.properties" || name == "application.yml" || name == "application.yaml" {
        return true;
    }
    match name.strip_prefix("application-") {
        Some(profile) => {
            profile.ends_with(".properties")
                || profile.ends_with(".yml")
                || profile.ends_with(".yaml")
        }
        None => false,
    }
}

fn is_excluded_directory(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    EXCLUDED_DIRECTORIES
        .iter()
        .any(|excluded| name.eq_ignore_ascii_case(excluded))
}

const EXCLUDED_DIRECTORIES: &[&str] = &[
    ".git",
    ".svn",
    ".hg",
    ".bzr",
    "target",
    "build",
    "out",
    "dist",
    "bin",
    "obj",
    "node_modules",
    "vendor",
    "coverage",
    ".gradle",
    ".mvn",
    "__pycache__",
    ".venv",
    "venv",
    ".next",
    ".nuxt",
    ".cache",
    ".idea",
    ".vscode",
    ".vs",
];

fn workspace_relative_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut text = String::new();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            continue;
        };
        let part = part.to_str()?;
        if !text.is_empty() {
            text.push('/');
        }
        text.push_str(part);
    }
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TempTree(PathBuf);

    static NEXT_TEMP_TREE: AtomicU64 = AtomicU64::new(0);

    impl TempTree {
        fn new(label: &str) -> Self {
            let sequence = NEXT_TEMP_TREE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "lithe-gpui-spring-paths-{}-{sequence}-{label}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("临时工作区根");
            Self(root)
        }

        fn root(&self) -> &Path {
            &self.0
        }

        fn write(&self, relative: &str, content: &str) {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("临时工作区父目录");
            }
            std::fs::write(&path, content).expect("临时工作区文件");
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn spring_index_path_names_match_the_reference() {
        for accepted in [
            "Demo.java",
            "a/b/APPLICATION.JAVA",
            "application.properties",
            "application.yml",
            "application.yaml",
            "application-prod.properties",
            "application-prod.yml",
            "application-prod.yaml",
            "META-INF/spring-configuration-metadata.json",
            "META-INF/additional-spring-configuration-metadata.json",
        ] {
            assert!(is_spring_index_path(accepted), "{accepted} 应当被收集");
        }

        for rejected in [
            "Demo.java.bak",
            "Demo.kt",
            "application.json",
            "application-prod.txt",
            "spring-configuration-metadata.json.bak",
            "spring-metadata.json",
            "index.html",
        ] {
            assert!(!is_spring_index_path(rejected), "{rejected} 不应当被收集");
        }
    }

    #[test]
    fn exclusion_matches_directory_names_case_insensitively() {
        for excluded in [
            "target",
            "Target",
            "TARGET",
            "build",
            "out",
            ".git",
            "node_modules",
        ] {
            assert!(
                is_excluded_directory(Path::new(excluded)),
                "{excluded} 应当被排除"
            );
        }
        for kept in [
            "src",
            "main",
            "java",
            "targets",
            "node_modules_",
            "resource",
        ] {
            assert!(
                !is_excluded_directory(Path::new(kept)),
                "{kept} 不是构建产物目录，不该被排除"
            );
        }
    }

    #[test]
    fn collects_relative_sorted_paths_and_skips_unrelated_files() {
        let tree = TempTree::new("extensions");
        tree.write(
            "src/main/java/demo/Application.java",
            "class Application {}\n",
        );
        tree.write(
            "src/main/resources/application.properties",
            "server.port=8080\n",
        );
        tree.write(
            "src/main/resources/application-prod.yml",
            "server:\n  port: 9090\n",
        );
        tree.write(
            "src/main/resources/META-INF/spring-configuration-metadata.json",
            "{}\n",
        );
        tree.write("notes.md", "# 不是 Spring 事实\n");
        tree.write(
            "src/main/java/demo/Application.java.bak",
            "class Application {}\n",
        );

        let collected = collect_spring_source_paths(tree.root());

        assert_eq!(
            collected.paths,
            vec![
                "src/main/java/demo/Application.java".to_string(),
                "src/main/resources/META-INF/spring-configuration-metadata.json".to_string(),
                "src/main/resources/application-prod.yml".to_string(),
                "src/main/resources/application.properties".to_string(),
            ],
            "升序 + 工作区相对路径 + `/` 分隔"
        );
        assert!(
            collected.paths.iter().all(|path| !path.contains('\\')),
            "必须是 `/` 分隔"
        );
        assert!(!collected.truncated, "没有到上限就不该报截断");
    }

    #[test]
    fn build_artifact_and_vcs_directories_are_not_scanned() {
        let tree = TempTree::new("excluded");
        tree.write("src/main/java/demo/Real.java", "class Real {}\n");
        for directory in ["target", "build", "out", ".git", "node_modules"] {
            tree.write(
                &format!("{directory}/generated/Generated.java"),
                "class Generated {}\n",
            );
            tree.write(
                &format!("{directory}/application.properties"),
                "generated=true\n",
            );
        }

        let collected = collect_spring_source_paths(tree.root());

        assert_eq!(
            collected.paths,
            vec!["src/main/java/demo/Real.java".to_string()],
            "被排除目录里的源码与配置都不该出现"
        );
    }

    #[test]
    fn the_path_cap_truncates_deterministically_and_reports_it() {
        let tree = TempTree::new("cap");
        for index in 0..5 {
            tree.write(&format!("src/File{index}.java"), "class Generated {}\n");
        }

        let capped = collect_bounded(tree.root(), 2, MAX_SCANNED_ENTRIES);
        assert_eq!(
            capped.paths,
            vec!["src/File0.java".to_string(), "src/File1.java".to_string()],
            "截断点必须落在排序序上，否则 Core 的输入不可复现"
        );
        assert!(capped.truncated, "截断了就要如实上报");

        let complete = collect_bounded(tree.root(), 5, MAX_SCANNED_ENTRIES);
        assert_eq!(complete.paths.len(), 5, "命中数等于上限时不算截断");
        assert!(!complete.truncated);
    }

    #[test]
    fn the_scan_budget_stops_a_tree_without_any_hits() {
        let tree = TempTree::new("budget");
        for index in 0..5 {
            tree.write(&format!("assets/asset-{index}.bin"), "x\n");
        }

        let collected = collect_bounded(tree.root(), MAX_SPRING_PATHS, 2);

        assert!(collected.paths.is_empty(), "没有合规文件就应当是空清单");
        assert!(collected.truncated, "预算用尽代表清单不完整，必须如实上报");
    }

    #[test]
    fn an_empty_or_missing_workspace_yields_no_paths() {
        let tree = TempTree::new("empty");
        tree.write("notes.md", "no java here\n");

        let empty = collect_spring_source_paths(tree.root());
        assert!(empty.paths.is_empty());
        assert!(!empty.truncated, "没有命中不等于被截断");

        let missing = tree.root().join("does-not-exist");
        let absent = collect_spring_source_paths(&missing);
        assert!(
            absent.paths.is_empty(),
            "不存在的根不该 panic，也不该编出路径"
        );
        assert!(!absent.truncated);
    }
}
