use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) struct TempRepo(PathBuf);

impl TempRepo {
    pub(crate) fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "lithe-workspace-config-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        let repo = Self(dir);
        repo.git(&["init", "--quiet"]);
        repo
    }

    pub(crate) fn root(&self) -> &Path {
        &self.0
    }

    pub(crate) fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap_or_else(|error| {
                panic!("跑 git {args:?} 失败（git 是否在 PATH 上？）：{error}")
            });
        assert!(
            output.status.success(),
            "git {args:?} 退出码 {:?}：{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    pub(crate) fn git_path(&self, path: &str) -> PathBuf {
        let resolved = self.git(&["rev-parse", "--git-path", path]);
        let candidate = PathBuf::from(resolved.trim());
        if candidate.is_absolute() {
            candidate
        } else {
            self.0.join(candidate)
        }
    }

    pub(crate) fn write(&self, relative: &str, contents: &str) {
        let path = self.0.join(relative);
        std::fs::create_dir_all(path.parent().expect("写入路径要有父目录")).expect("建父目录");
        std::fs::write(&path, contents).expect("写文件");
    }

    pub(crate) fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.0.join(relative)).expect("读文件")
    }

    pub(crate) fn exclude_lines(&self) -> Vec<String> {
        self.exclude_lines_at(&self.git_path("info/exclude"))
    }

    pub(crate) fn exclude_lines_at(&self, path: &Path) -> Vec<String> {
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect()
    }

    pub(crate) fn status(&self) -> String {
        self.git(&["status", "--porcelain"])
    }

    pub(crate) fn staged_paths(&self) -> Vec<String> {
        let raw = self.git(&["diff", "--cached", "--name-only"]);
        let mut staged: Vec<String> = raw
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
        staged.sort();
        staged
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn is_inside_work_tree(dir: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output()
        .map(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true"
        })
        .unwrap_or(false)
}
