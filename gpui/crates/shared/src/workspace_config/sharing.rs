//! "默认不共享"的守卫，以及显式的"共享此项目的配置"动作。
//!
//! ## 为什么要复用 Core 的 `git.write`
//!
//! 本机排除文件的位置**必须由 Git 自己解析**：仓库可能是 worktree / submodule / bare，
//! 那时 `.git` 是文件而不是目录，硬编码 `<root>/.git/info/exclude` 直接写错地方。
//! Core 的 `git.write` 已经处理了这件事（`GitIgnoreTarget::LocalExclude` 走
//! `git_path(root, "info/exclude")`，`rust/lithe-core/src/git/mod.rs`），所以这里只发命令。
//!
//! ## 为什么用 `excludePatterns` 而不是 `exclude`
//!
//! 两个 operation 都写本机排除文件，差别在"写进去的是不是调用方的原文"：
//!
//! | operation | 走哪个函数 | 写进去的形态 |
//! | --- | --- | --- |
//! | `exclude` | `append_git_ignore_patterns` → `git_ignore_patterns` | 会 root-anchor、会转义 pathspec 字符 |
//! | `excludePatterns` | `mutate_literal_git_ignore_patterns(adding=true)` | **保留调用方原文**（只 trim） |
//!
//! 判据只有一条：**我能不能用同一个 literal 把我加的那一行精确删掉**。
//! `unexcludePatterns` 按"存储的原始字节"逐行比对删除，只有 `excludePatterns` 写进去的
//! `.lithe/` 才能被同一个 `.lithe/` 精确删掉。用 `exclude` 的话写进去的是锚定/转义后的形态，
//! 想删就得复现 Core 的转义逻辑 —— 那正是"发明第二个实现"。
//!
//! `excludePatterns` 还自带两个我们要的性质：**已经存在时不写**（no-op），以及
//! 只追加、从不重写文件（别人的规则逐字保留）。
//!
//! ## "不是 Git 仓库"怎么判
//!
//! Core 的 `require_git_repository` 对非仓库根返回**错误码 `invalid_request` + 稳定消息
//! `Not a Git repository`**，它的注释明确说这条稳定消息就是给宿主"跳过 Git 副作用"用的。
//! 这里通过 [`CoreError::is_not_a_repository`] 使用它，而不是自己再探测一遍仓库状态 ——
//! 那会多一次 `git.status` 往返，而且要在本层复制仓库检测。

use std::path::Path;

use serde_json::json;

use crate::core_client::{CoreClient, CoreError, CoreRequest};
use crate::workspace_config::DIAGNOSTIC_PREFIX;
use crate::workspace_config::paths::{
    WorkspaceConfigPaths, is_pruned_directory, is_shareable_member,
    workspace_config_exclude_pattern, workspace_relative,
};

/// 一次"确保排除"的结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnsureExcluded {
    /// 排除条目已经在那里（含重复调用：Core 判定已存在就不写）。
    Ensured,
    /// 这个根不是 Git 仓库：静默跳过（不报错、不创建任何文件）。
    NotARepository,
}

/// 一次"共享此项目的配置"的结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareOutcome {
    /// 这些工作区相对路径进了暂存区。
    Staged {
        /// 被暂存的可共享成员（工作区相对路径，`/` 分隔，已排序）。
        files: Vec<String>,
    },
    /// 没有任何可共享成员（`.lithe/` 不存在，或里面只有本机层文件）。
    /// 这种情况下**不动**排除条目：没东西可共享就不该把目录暴露出来。
    NothingToShare,
    /// 不是 Git 仓库：什么都不做。
    NotARepository,
}

/// 确保 `.lithe/` 进本机排除文件：**在做任何 `.lithe/` 写入之前调用**。
///
/// 幂等：条目已存在时 Core 判定无需写入。不是 Git 仓库时静默跳过。
pub fn ensure_project_dir_excluded(root: &Path) -> Result<EnsureExcluded, CoreError> {
    let pattern = workspace_config_exclude_pattern();
    match write_ignore_patterns(root, "excludePatterns", &pattern) {
        Ok(()) => {
            println!(
                "{DIAGNOSTIC_PREFIX} excluded root={} pattern={pattern}",
                root.display()
            );
            Ok(EnsureExcluded::Ensured)
        }
        // "不是仓库"不是失败：这条守卫对非 Git 项目必须完全无副作用，也不打扰用户。
        Err(error) if error.is_not_a_repository() => Ok(EnsureExcluded::NotARepository),
        Err(error) => {
            eprintln!(
                "{DIAGNOSTIC_PREFIX} exclude_failed root={} pattern={pattern} error={error}",
                root.display()
            );
            Err(error)
        }
    }
}

/// 共享这个项目的工作区配置：**先移除本机排除行，再把可共享成员放进暂存区**。
///
/// 顺序不能反：被排除的文件用普通 `git add` 提交会被拒绝（要 `-f`），而"让用户自己发现
/// 需要 `-f`"是设计缺陷。移除用的是与写入完全相同的 literal，所以是精确的一对一。
///
/// 不做界面，调用方留给下一批。
pub fn share_project_config(root: &Path) -> Result<ShareOutcome, CoreError> {
    let members = shareable_members(root);
    if members.is_empty() {
        println!(
            "{DIAGNOSTIC_PREFIX} share_skipped root={} reason=nothing_to_share",
            root.display()
        );
        return Ok(ShareOutcome::NothingToShare);
    }

    let pattern = workspace_config_exclude_pattern();
    match write_ignore_patterns(root, "unexcludePatterns", &pattern) {
        Ok(()) => {}
        Err(error) if error.is_not_a_repository() => return Ok(ShareOutcome::NotARepository),
        Err(error) => {
            eprintln!(
                "{DIAGNOSTIC_PREFIX} share_failed root={} step=unexclude error={error}",
                root.display()
            );
            return Err(error);
        }
    }

    if let Err(error) = write_git_paths(root, "stage", &members) {
        eprintln!(
            "{DIAGNOSTIC_PREFIX} share_failed root={} step=stage error={error}",
            root.display()
        );
        return Err(error);
    }

    println!(
        "{DIAGNOSTIC_PREFIX} shared root={} files={}",
        root.display(),
        members.len()
    );
    Ok(ShareOutcome::Staged { files: members })
}

/// 遍历深度上限。
///
/// `.lithe/` 是我们自己的目录，正常只有两三层；这个上限只是防止一个被手工塞满的目录
/// 让扫描变成无界操作。
const MAX_WALK_DEPTH: usize = 6;

/// 一次扫描返回的成员数上限（同上，防无界）。
const MAX_MEMBERS: usize = 512;

/// 工作区根下**当前存在**的可共享成员（工作区相对路径，`/` 分隔，已排序）。
///
/// 只列常规文件：暂存用显式路径，所以即使 `.lithe/.gitignore` 还不存在也不会误把本机层
/// 文件带进暂存区。`run/classes` 整棵子树被剪掉（它可能很大）。
pub fn shareable_members(root: &Path) -> Vec<String> {
    let paths = WorkspaceConfigPaths::new(root);
    let config_dir = paths.directory();
    let mut found = Vec::new();
    collect_shareable(root, &config_dir, &config_dir, 0, &mut found);
    found.sort();
    found.dedup();
    found.truncate(MAX_MEMBERS);
    found
}

fn collect_shareable(
    root: &Path,
    config_dir: &Path,
    current: &Path,
    depth: usize,
    out: &mut Vec<String>,
) {
    if depth > MAX_WALK_DEPTH || out.len() >= MAX_MEMBERS {
        return;
    }
    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };
    // 目录项顺序由文件系统给，不稳定；排序后再走，结论才是可复现的。
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let Ok(config_relative) = path.strip_prefix(config_dir) else {
            continue;
        };
        let config_relative = config_relative.to_string_lossy().replace('\\', "/");
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            if is_pruned_directory(&config_relative) {
                continue;
            }
            collect_shareable(root, config_dir, &path, depth + 1, out);
            continue;
        }
        if !file_type.is_file() || !is_shareable_member(&config_relative) {
            continue;
        }
        if let Some(relative) = workspace_relative(root, &path) {
            out.push(relative);
        }
    }
}

/// 往本机排除文件写/删一个 literal 条目。
fn write_ignore_patterns(root: &Path, operation: &str, pattern: &str) -> Result<(), CoreError> {
    write_git_paths(root, operation, &[pattern.to_string()])
}

/// 发一条带 `paths` 的 `git.write`。
///
/// 超时显式取 **60 s**（照 `lithe-gpui-git` 的 `GIT_TIMEOUT_MILLIS` 口径），不跟随信封层
/// 默认的 120 s：这几条写操作只改一个文本文件，超时口径放宽属于可观察的行为变化。
fn write_git_paths(root: &Path, operation: &str, paths: &[String]) -> Result<(), CoreError> {
    CoreClient::new()
        .execute(
            &CoreRequest::command("git.write")
                .with_payload(json!({
                    "root": root.to_string_lossy(),
                    "operation": operation,
                    "paths": paths,
                }))
                .with_timeout_millis(GIT_WRITE_TIMEOUT_MILLIS),
        )
        .map(|_| ())
}

/// 本模块 Git 写操作的超时（毫秒）。
const GIT_WRITE_TIMEOUT_MILLIS: u64 = 60_000;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_config::test_repo::{TempRepo, is_inside_work_tree};

    /// 守卫的全部意义：`.lithe/` 不再出现在 `git status` 里，且排除文件里**恰好**只有我们那一行。
    ///
    /// 同时守住三件事：只追加（不动别人的规则）、幂等（重复调用不产生第二行）、
    /// 位置由 Git 自己解析（用 `rev-parse --git-path` 定位）。
    #[test]
    fn ensure_excluded_hides_the_config_directory_from_git_status() {
        let repo = TempRepo::new("ensure");
        repo.write(".lithe/project.json", "{\"id\":\"x\",\"version\":1}");

        // 先证明"没有守卫时它确实是可见的" —— 否则下面的断言可能是假阳性。
        let before = repo.status();
        assert!(before.contains(".lithe"), "前置条件不成立：{before}");

        // 排除文件里先放一条**别人的**规则，验证守卫不重写、不动它。
        let exclude = repo.git_path("info/exclude");
        std::fs::write(&exclude, "# 别人的规则\n*.log\n").expect("写初始排除文件");

        assert_eq!(
            ensure_project_dir_excluded(repo.root()).expect("守卫失败"),
            EnsureExcluded::Ensured
        );
        // 幂等：再来一次不该产生第二行。
        assert_eq!(
            ensure_project_dir_excluded(repo.root()).expect("守卫失败"),
            EnsureExcluded::Ensured
        );

        let after = repo.status();
        assert!(
            !after.contains(".lithe"),
            "排除之后 git status 不该出现 .lithe：{after}"
        );

        let lines = repo.exclude_lines_at(&exclude);
        assert_eq!(
            lines.iter().filter(|line| *line == ".lithe/").count(),
            1,
            "排除文件里应当恰好只有一行 .lithe/：{lines:?}"
        );
        assert!(lines.contains(&"# 别人的规则".to_string()), "{lines:?}");
        assert!(lines.contains(&"*.log".to_string()), "{lines:?}");
    }

    /// 显式共享：移除排除行，**只有可共享成员**进暂存区。
    #[test]
    fn sharing_exposes_only_shareable_members() {
        let repo = TempRepo::new("share");
        repo.write(".lithe/settings.json", "{}");
        repo.write(".lithe/.gitignore", "*.local.json\n");
        repo.write(".lithe/run/configurations.json", "{}");
        // 本机层与派生物：绝不能进暂存区。
        repo.write(".lithe/settings.local.json", "{}");
        repo.write(".lithe/run.local.json", "{}");
        repo.write(".lithe/maven.local.json", "{}");
        repo.write(".lithe/run/classes/java-main-Standalone/App.class", "x");

        assert_eq!(
            ensure_project_dir_excluded(repo.root()).expect("守卫失败"),
            EnsureExcluded::Ensured
        );

        let outcome = share_project_config(repo.root()).expect("共享失败");
        let ShareOutcome::Staged { files } = outcome else {
            panic!("应当暂存了成员");
        };
        assert_eq!(
            files,
            vec![
                ".lithe/.gitignore".to_string(),
                ".lithe/run/configurations.json".to_string(),
                ".lithe/settings.json".to_string(),
            ],
            "可共享成员清单不对"
        );

        // 排除行被精确移除（与写入用的是同一个 literal）。
        let exclude = repo.git_path("info/exclude");
        assert!(
            !repo
                .exclude_lines_at(&exclude)
                .iter()
                .any(|line| line == ".lithe/"),
            "共享之后排除行必须被移除"
        );

        // 只有可共享成员进了暂存区。
        let staged = repo.staged_paths();
        assert_eq!(staged, files, "暂存区内容与可共享成员不一致");
        assert!(
            !staged.iter().any(|path| path.ends_with(".local.json")),
            "本机层文件绝不能进暂存区：{staged:?}"
        );
        assert!(
            !staged.iter().any(|path| path.contains("run/classes")),
            "编译产物绝不能进暂存区：{staged:?}"
        );
    }

    /// 没有可共享成员时，**不动**排除条目：没东西可共享就不该把目录暴露出来。
    #[test]
    fn nothing_to_share_leaves_the_exclusion_in_place() {
        let repo = TempRepo::new("nothing");
        assert_eq!(
            ensure_project_dir_excluded(repo.root()).expect("守卫失败"),
            EnsureExcluded::Ensured
        );

        // `.lithe/` 不存在。
        assert_eq!(
            share_project_config(repo.root()).expect("共享失败"),
            ShareOutcome::NothingToShare
        );
        assert!(
            repo.exclude_lines().iter().any(|line| line == ".lithe/"),
            "没东西可共享时不得移除排除行"
        );

        // 只有本机层文件时同样没东西可共享。
        repo.write(".lithe/settings.local.json", "{}");
        assert_eq!(
            share_project_config(repo.root()).expect("共享失败"),
            ShareOutcome::NothingToShare
        );
        assert!(
            repo.exclude_lines().iter().any(|line| line == ".lithe/"),
            "只有本机层文件时不得移除排除行"
        );
    }

    /// 不是 Git 仓库：静默跳过，不报错、不创建任何文件。
    ///
    /// ⚠️ 前提是选一个**不在任何 Git 工作树里**的目录（见 `test_repo` 的模块文档：
    /// `TEMP` 常常落在某个仓库内，那时 `git rev-parse` 会找到外层仓库，这条测试就会去改
    /// 真实仓库的排除文件）。前提不满足就明确跳过 —— 宁可少跑一次，也不能让测试动到真实仓库。
    #[test]
    fn non_repository_is_skipped_silently() {
        let dir = std::env::temp_dir().join(format!(
            "lithe-workspace-config-no-repo-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");

        if is_inside_work_tree(&dir) {
            println!(
                "skip: {} 落在某个 Git 工作树里，不能用来验证'非仓库'（会动到外层仓库）",
                dir.display()
            );
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }

        assert_eq!(
            ensure_project_dir_excluded(&dir).expect("非仓库不该是失败"),
            EnsureExcluded::NotARepository
        );
        assert_eq!(
            share_project_config(&dir).expect("非仓库不该是失败"),
            ShareOutcome::NotARepository
        );
        assert!(
            !dir.join(".git").exists(),
            "非仓库路径上不得创建任何 Git 文件"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 可共享成员扫描：排序确定、剪掉派生物目录、不误收本机层。
    #[test]
    fn shareable_members_are_deterministic_and_pruned() {
        let repo = TempRepo::new("members");
        repo.write(".lithe/settings.json", "{}");
        repo.write(".lithe/settings.local.json", "{}");
        repo.write(".lithe/maven/config.json", "{}");
        repo.write(".lithe/run/classes/deep/deeper/App.class", "x");
        repo.write(".lithe/lsp/language-providers.json", "{}");

        assert_eq!(
            shareable_members(repo.root()),
            vec![
                ".lithe/lsp/language-providers.json".to_string(),
                ".lithe/maven/config.json".to_string(),
                ".lithe/settings.json".to_string(),
            ]
        );
    }
}
