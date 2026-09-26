//! 测试用的真实临时 Git 仓库（**只在 test 构建里编译**）。
//!
//! ## 为什么每个用守卫的测试都必须先 `git init`
//!
//! 验证脚本会把 `TEMP` 指到仓库内的 `.artifacts/alt-tmp`（这台机器的 `%TEMP%` 对 Rust
//! 测试进程返回 `PermissionDenied`）。于是"临时目录"其实落在 **Lithe 仓库自己的工作树里**，
//! 而 `ensure_project_dir_excluded` 走的是 `git rev-parse` —— 它会找到**外层**仓库，
//! 把 `.lithe/` 写进真实检出目录的 `.git/info/exclude`。
//!
//! 所以在自己的临时目录里 `git init` 不只是"造个测试环境"，它同时是把守卫的作用域
//! **限制在这个临时目录里**的手段。任何调用 [`crate::workspace_config::save_project_manifest`]
//! 或排除守卫的测试都要用它，不要直接用裸临时目录。
//!
//! ## 为什么用真 git 而不是打桩
//!
//! 本模块的价值就是"Core 真的把那行写进了 Git 自己认的排除文件"，而排除文件的位置由
//! `git rev-parse --git-path` 决定 —— 打桩只能验证我们自己拼的字符串。Core 的 `git` 测试
//! 也是这个口径（临时仓库 + 真命令）。命令全是本地操作（init / status / rev-parse / diff），
//! 不碰网络、不触发钩子。

use std::path::{Path, PathBuf};
use std::process::Command;

/// 一个真实的临时 Git 仓库，`Drop` 时整个删掉（含 `.git`）。
pub(crate) struct TempRepo(PathBuf);

impl TempRepo {
    /// 建一个独立的临时仓库。`tag` 用来避免同一进程内的用例互相踩。
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

    /// 仓库根。
    pub(crate) fn root(&self) -> &Path {
        &self.0
    }

    /// 跑一条 git 命令并要求成功。
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

    /// `git rev-parse --git-path <path>` 解析出的绝对路径。
    ///
    /// **不硬编码 `<root>/.git/info/exclude`**：worktree / submodule 下 `.git` 是文件，
    /// 那个位置是错的。测试要和产品走同一条解析路径。
    pub(crate) fn git_path(&self, path: &str) -> PathBuf {
        let resolved = self.git(&["rev-parse", "--git-path", path]);
        let candidate = PathBuf::from(resolved.trim());
        if candidate.is_absolute() {
            candidate
        } else {
            self.0.join(candidate)
        }
    }

    /// 在仓库里写一个文件（自动建父目录）。
    pub(crate) fn write(&self, relative: &str, contents: &str) {
        let path = self.0.join(relative);
        std::fs::create_dir_all(path.parent().expect("写入路径要有父目录")).expect("建父目录");
        std::fs::write(&path, contents).expect("写文件");
    }

    /// 读仓库里的一个文件。
    pub(crate) fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.0.join(relative)).expect("读文件")
    }

    /// 本机排除文件里去掉空行之后的**原始行**（顺序即文件顺序）。
    pub(crate) fn exclude_lines(&self) -> Vec<String> {
        self.exclude_lines_at(&self.git_path("info/exclude"))
    }

    /// 指定排除文件里去掉空行之后的原始行。
    pub(crate) fn exclude_lines_at(&self, path: &Path) -> Vec<String> {
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// `git status --porcelain` 的原文。
    pub(crate) fn status(&self) -> String {
        self.git(&["status", "--porcelain"])
    }

    /// 暂存区里的路径（已排序）。
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

/// 这个目录是否落在某个 Git 工作树里。
///
/// `TEMP` 指向仓库内时它会是 `true` —— 用于"必须避开外层仓库"的测试先判定前提。
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
