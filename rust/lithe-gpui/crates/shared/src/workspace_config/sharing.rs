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

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::core_client::{CoreClient, CoreError, CoreRequest, core_json};
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

/// 本模块的失败。
///
/// ## 为什么不能直接用 [`CoreError`]
///
/// **回读校验**发现的失败不是 Core 报的错：Core 说"写成功了"，是我们在回读时发现预期状态
/// 没出现。那是本层的事实，必须有自己的变体 —— 否则调用方无法把"写入被丢弃"与"Core 调用
/// 本身失败"分开处理，也没法在诊断里如实说明到底发生了什么。
#[derive(Debug)]
pub enum SharingError {
    /// Core 调用本身失败（信封错误、进程失败……）。
    Git(CoreError),
    /// 写/删之后**回读发现预期状态没有出现**。
    ///
    /// 真机成因是"写入被静默丢弃"：`fs` 不报错，但文件里没有那一条。也可能是文件读不出来。
    /// 两种情况都必须**报失败** —— 没有验证就宣称成功就是谎报，用户读回文件时会发现
    /// 目录根本没被排除，而日志里却明明写着 `excluded`。
    NotPersisted {
        /// 做了哪一步（`excludePatterns` / `unexcludePatterns`）。
        operation: &'static str,
        /// 那一条 literal。
        pattern: String,
        /// 回读的那个文件。
        path: PathBuf,
        /// 人话原因（"回读时文件里没有该行" / "回读失败：…"）。
        reason: String,
    },
}

impl SharingError {
    /// 这次失败是不是"这个根不是 Git 仓库"（那种情况必须静默跳过，不是错误）。
    pub fn is_not_a_repository(&self) -> bool {
        matches!(self, Self::Git(error) if error.is_not_a_repository())
    }
}

impl fmt::Display for SharingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Git(error) => write!(formatter, "{error}"),
            Self::NotPersisted {
                operation,
                pattern,
                path,
                reason,
            } => write!(
                formatter,
                "{operation} 之后回读 {path} 时没能确认 `{pattern}` 的状态（{reason}）——\
                 写入可能被静默丢弃，拒绝宣称成功",
                path = path.display()
            ),
        }
    }
}

impl std::error::Error for SharingError {}

/// 回读判定：`exclude_file` 里是否已经有与 `pattern` 相同的那一条 literal。
///
/// 单独抽出来是因为**它是本模块最该被直测的那条规则**：真机上的"写入被静默丢弃"没法确定性
/// 构造（`fs` 不报错但不落盘），所以判据本身必须能拿临时文件直接测。
///
/// 三条语义：
///
/// - **文件不存在 → `Ok(false)`**：没有那个文件就绝不可能有那一条；
/// - 逐行 `trim` 之后**逐字比较**：Core 写入时也只 trim（不 root-anchor、不转义 pathspec），
///   所以比较口径必须与它一致，否则"写进去的形态"与"我们找的形态"会错开；
/// - **注释行不参与比较**：`# .lithe/` 是注释，不是规则。把它当成命中会让回读在
///   "用户把规则注释掉了"时报成功。
pub fn pattern_is_present(exclude_file: &Path, pattern: &str) -> io::Result<bool> {
    let text = match std::fs::read_to_string(exclude_file) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let wanted = pattern.trim();
    Ok(text.lines().map(str::trim).any(|line| {
        !line.is_empty() && !line.starts_with('#') && line == wanted
    }))
}

/// 本机排除文件的绝对路径；`Ok(None)` = 这个根不是 Git 仓库。
///
/// **必须问 Core**（`git.watchContext`）而不是自己拼 `<root>/.git/info/exclude`：仓库可能是
/// linked worktree / submodule / bare，那时 Git 目录在别处，而且 `info/exclude` 落在**共享的
/// common 目录**里（Core 自己的测试断言过 linked worktree 下 `.git/info/exclude` 并不存在）。
fn local_exclude_path(root: &Path) -> Result<Option<PathBuf>, CoreError> {
    let data = core_json(
        "git.watchContext",
        json!({ "root": root.to_string_lossy() }),
    )?;
    let Some(data) = data else {
        return Ok(None);
    };
    let common = data
        .get("gitCommonDirectory")
        .and_then(|value| value.as_str())
        .ok_or_else(|| CoreError::MissingData {
            command: "git.watchContext".to_string(),
        })?;
    Ok(Some(Path::new(common).join("info").join("exclude")))
}

/// 造一个 [`SharingError::NotPersisted`]，同时打一条可 grep 的诊断。
fn not_persisted(
    operation: &'static str,
    pattern: &str,
    path: &Path,
    reason: &str,
) -> SharingError {
    eprintln!(
        "{DIAGNOSTIC_PREFIX} exclude_not_persisted operation={operation} pattern={pattern} \
         path={} reason={reason}",
        path.display()
    );
    SharingError::NotPersisted {
        operation,
        pattern: pattern.to_string(),
        path: path.to_path_buf(),
        reason: reason.to_string(),
    }
}

/// 确保 `.lithe/` 进本机排除文件：**在做任何 `.lithe/` 写入之前调用**。
///
/// 幂等：条目已存在时 Core 判定无需写入。不是 Git 仓库时静默跳过。
///
/// ⚠️ **写了之后一定回读**：Core 返回成功只代表它没报错，不代表那一条真的落盘了（真机上
/// 出现过"`fs` 不报错但文件里没有"的静默丢弃）。回读不通过时返回
/// [`SharingError::NotPersisted`]，**绝不**打 `excluded`。
pub fn ensure_project_dir_excluded(root: &Path) -> Result<EnsureExcluded, SharingError> {
    let pattern = workspace_config_exclude_pattern();

    // 先问 Git：这个根在不在仓库里、排除文件在哪。不是仓库就静默跳过（非 Git 项目必须
    // 完全无副作用，也不打扰用户）。
    let Some(exclude_file) = local_exclude_path(root).map_err(SharingError::Git)? else {
        return Ok(EnsureExcluded::NotARepository);
    };

    match write_ignore_patterns(root, "excludePatterns", &pattern) {
        Ok(()) => {}
        // 兜底：万一在两次调用之间仓库没了，仍然按"不是仓库"处理。
        Err(error) if error.is_not_a_repository() => return Ok(EnsureExcluded::NotARepository),
        Err(error) => {
            eprintln!(
                "{DIAGNOSTIC_PREFIX} exclude_failed root={} pattern={pattern} error={error}",
                root.display()
            );
            return Err(SharingError::Git(error));
        }
    }

    match pattern_is_present(&exclude_file, &pattern) {
        Ok(true) => {
            println!(
                "{DIAGNOSTIC_PREFIX} excluded root={} pattern={pattern}",
                root.display()
            );
            Ok(EnsureExcluded::Ensured)
        }
        Ok(false) => Err(not_persisted(
            "excludePatterns",
            &pattern,
            &exclude_file,
            "回读时文件里没有该行",
        )),
        Err(error) => Err(not_persisted(
            "excludePatterns",
            &pattern,
            &exclude_file,
            &format!("回读失败：{error}"),
        )),
    }
}

/// `.lithe/.gitignore` 里**我们负责**的那几行（顺序 = 追加顺序）。
///
/// 与设计 Note 第三节一字对应：`*.local.json`（本机层）、`run/classes/`（编译产物）、
/// `**/*.tmp`（原子写的中间产物）。
///
/// **新增一条规则时只改这一处** —— 补齐逻辑（[`ensure_local_ignore_file`]）与测试都读它，
/// 不需要在别处再登记一遍。
pub const LOCAL_IGNORE_RULES: &[&str] = &["*.local.json", "run/classes/", "**/*.tmp"];

/// 一次补齐 `.lithe/.gitignore` 的结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreFileOutcome {
    /// 文件本来不存在，这次建了出来。
    Created,
    /// 文件存在，补了 `added` 行。
    Appended {
        /// 这次追加的规则条数。
        added: usize,
    },
    /// 规则都在：**一个字都没写**（幂等，重复调用逐字不变）。
    Unchanged,
}

/// 补齐 `.lithe/.gitignore` —— "默认不共享"的**第二道闸**。
///
/// ## 两道闸的分工
///
/// | 闸 | 文件 | 什么时候起作用 |
/// | --- | --- | --- |
/// | 本机排除（[`ensure_project_dir_excluded`]，走 Core） | `<repo>/.git/info/exclude` | 还没人共享 `.lithe/` 时，整目录被挡住 |
/// | 这个函数 | `.lithe/.gitignore`（仓库内的普通文件） | **一旦有人共享**，用它挡住本机层与派生物 |
///
/// 第一道闸在"没人共享"时是惰性的（Git 不会进入被排除的目录），所以第二道闸必须**始终**维护
/// —— 它生效的时刻正是第一道被移除的那一刻。设计 Note 第七节把这条写成硬要求。
///
/// ## 只追加、绝不重写
///
/// - 已经存在的行（trim 后与规则逐字相同）不再追加；
/// - 规则都在时**直接返回、不写文件**（所以重复调用逐字不变，mtime 也不动）；
/// - 用户自己的行、顺序、注释一律保留，我们只往末尾补缺失的规则；
/// - 用户想否定某条规则，在后面写 `!*.local.json` 即可 —— 我们只负责补齐缺失的规则，
///   不与用户争抢（这是普通 `.gitignore`，最后的匹配获胜）。
///
/// 写盘用**原子写**（临时文件 + rename），与这个模块其它写入同一条口径。
pub fn ensure_local_ignore_file(root: &Path) -> io::Result<IgnoreFileOutcome> {
    let path = WorkspaceConfigPaths::new(root).ignore_file();
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };

    let contents = existing.clone().unwrap_or_default();
    let missing: Vec<&str> = LOCAL_IGNORE_RULES
        .iter()
        .copied()
        .filter(|rule| !contents.lines().any(|line| line.trim() == *rule))
        .collect();
    if missing.is_empty() {
        return Ok(IgnoreFileOutcome::Unchanged);
    }

    let mut next = contents;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    for rule in &missing {
        next.push_str(rule);
        next.push('\n');
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = crate::document::tmp_path(&path);
    std::fs::write(&tmp, next.as_bytes())?;
    std::fs::rename(&tmp, &path)?;

    let outcome = match existing {
        None => IgnoreFileOutcome::Created,
        Some(_) => IgnoreFileOutcome::Appended {
            added: missing.len(),
        },
    };
    println!(
        "{DIAGNOSTIC_PREFIX} ignore_file root={} outcome={outcome:?}",
        root.display()
    );
    Ok(outcome)
}

/// 共享这个项目的工作区配置：**先移除本机排除行，再把可共享成员放进暂存区**。
///
/// 顺序不能反：被排除的文件用普通 `git add` 提交会被拒绝（要 `-f`），而"让用户自己发现
/// 需要 `-f`"是设计缺陷。移除用的是与写入完全相同的 literal，所以是精确的一对一。
///
/// 不做界面，调用方留给下一批。
///
/// ⚠️ **移除之后同样回读**：与 [`ensure_project_dir_excluded`] 对称 —— 移除没生效却报
/// `shared`，用户会以为配置已经交给团队，实际那一条还挡着，`git add` 又会被拒绝。
pub fn share_project_config(root: &Path) -> Result<ShareOutcome, SharingError> {
    let members = shareable_members(root);
    if members.is_empty() {
        println!(
            "{DIAGNOSTIC_PREFIX} share_skipped root={} reason=nothing_to_share",
            root.display()
        );
        return Ok(ShareOutcome::NothingToShare);
    }

    let Some(exclude_file) = local_exclude_path(root).map_err(SharingError::Git)? else {
        return Ok(ShareOutcome::NotARepository);
    };

    let pattern = workspace_config_exclude_pattern();
    match write_ignore_patterns(root, "unexcludePatterns", &pattern) {
        Ok(()) => {}
        Err(error) if error.is_not_a_repository() => return Ok(ShareOutcome::NotARepository),
        Err(error) => {
            eprintln!(
                "{DIAGNOSTIC_PREFIX} share_failed root={} step=unexclude error={error}",
                root.display()
            );
            return Err(SharingError::Git(error));
        }
    }

    // 回读：这一条必须**已经不在**了，否则后面的 `stage` 会踩在"文件仍被排除"之上。
    match pattern_is_present(&exclude_file, &pattern) {
        Ok(false) => {}
        Ok(true) => {
            return Err(not_persisted(
                "unexcludePatterns",
                &pattern,
                &exclude_file,
                "回读时文件里仍有该行",
            ));
        }
        Err(error) => {
            return Err(not_persisted(
                "unexcludePatterns",
                &pattern,
                &exclude_file,
                &format!("回读失败：{error}"),
            ));
        }
    }

    if let Err(error) = write_git_paths(root, "stage", &members) {
        eprintln!(
            "{DIAGNOSTIC_PREFIX} share_failed root={} step=stage error={error}",
            root.display()
        );
        return Err(SharingError::Git(error));
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

    /// 回读判据本身的三条语义：文件不存在 / 逐行 trim 后逐字比较 / **注释不算命中**。
    ///
    /// 之所以单独直测它：真机上的"写入被静默丢弃"**没法确定性构造**（`fs` 不报错但文件里没有），
    /// 所以把判据抽成纯函数、用临时文件直接打，是唯一可靠的覆盖方式。
    #[test]
    fn pattern_is_present_reads_the_documented_cases() {
        let dir = std::env::temp_dir().join(format!("lithe-sharing-readback-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        let file = dir.join("exclude");

        // 1) 文件不存在 → 没有那一条（那个文件都没有，就不可能有规则）。
        assert!(!pattern_is_present(&file, ".lithe/").expect("读失败"));

        // 2) 别人的规则 + 被空白包裹的我们那一条 → 命中（Core 写入时也只 trim，口径必须一致）。
        std::fs::write(&file, "# 别人的规则\n*.log\n   .lithe/   \n").expect("写文件");
        assert!(pattern_is_present(&file, ".lithe/").expect("读失败"));

        // 3) 只有**被注释掉**的那一条 → 不算命中：注释不是规则。
        std::fs::write(&file, "# .lithe/\n*.log\n").expect("写文件");
        assert!(!pattern_is_present(&file, ".lithe/").expect("读失败"));

        // 4) 相似但不是同一条 → 不算命中（挡住前缀/包含式误判）。
        std::fs::write(&file, ".lithe\n.lithe/run/\n").expect("写文件");
        assert!(!pattern_is_present(&file, ".lithe/").expect("读失败"));

        // 5) 空文件与全空行 → 不算命中。
        std::fs::write(&file, "\n   \n").expect("写文件");
        assert!(!pattern_is_present(&file, ".lithe/").expect("读失败"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 正常仓库走完守卫之后，回读必须能确认那一条**确实在**。
    ///
    /// 这条把"校验真的接上了"钉死：如果实现里没有回读、或者判据写反，
    /// `EnsureExcluded::Ensured` 与 `pattern_is_present == true` 不可能同时成立。
    #[test]
    fn ensure_excluded_verifies_its_own_write_by_reading_back() {
        let repo = TempRepo::new("readback");
        let exclude = repo.git_path("info/exclude");
        std::fs::write(&exclude, "# 别人的规则\n*.log\n").expect("写初始排除文件");

        assert_eq!(
            ensure_project_dir_excluded(repo.root()).expect("守卫失败"),
            EnsureExcluded::Ensured
        );
        assert!(
            pattern_is_present(&exclude, ".lithe/").expect("读失败"),
            "守卫报成功之后，回读必须能在排除文件里找到那一条"
        );
        // 别人的规则仍然逐字保留（回读不该顺手改写文件）。
        let text = std::fs::read_to_string(&exclude).expect("读排除文件");
        assert!(text.contains("# 别人的规则") && text.contains("*.log"), "{text}");
    }

    /// 一个"进程内唯一、用后即删"的**普通**临时目录。
    ///
    /// 与 [`TempRepo`] 的唯一区别是不 `git init`：本模块的守卫测试必须用 `TempRepo`（否则
    /// `git rev-parse` 会找到外层真实仓库、把 `.lithe/` 写进真实检出），而 `.lithe/.gitignore`
    /// 的补齐是**仓库内的普通文件写入**、完全不碰 Git，所以普通目录就够、也更接近真实调用点。
    fn plain_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lithe-workspace-config-ignore-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        dir
    }

    /// 文件不存在时建出来：内容**恰好**是我们负责的那几条，不多不少、不残留临时文件。
    #[test]
    fn local_ignore_file_is_created_with_only_our_rules() {
        let dir = plain_dir("create");

        assert_eq!(
            ensure_local_ignore_file(&dir).expect("补齐失败"),
            IgnoreFileOutcome::Created
        );

        let path = dir.join(".lithe/.gitignore");
        let rules: Vec<String> = std::fs::read_to_string(&path)
            .expect("读 .lithe/.gitignore")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
        let expected: Vec<String> = LOCAL_IGNORE_RULES.iter().map(|rule| rule.to_string()).collect();
        assert_eq!(rules, expected, "第二道闸的内容必须与 LOCAL_IGNORE_RULES 一字对应");

        assert!(
            !dir.join(".git").exists(),
            "补齐 .lithe/.gitignore 不该创建任何 Git 文件"
        );
        assert!(
            !crate::document::tmp_path(&path).exists(),
            "原子写不得残留临时文件"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 已有内容时**只追加缺失的规则**：用户自己的行、顺序、注释逐字保留，已存在的那条不重复。
    #[test]
    fn local_ignore_file_appends_missing_rules_and_keeps_other_lines() {
        let dir = plain_dir("append");
        let path = dir.join(".lithe/.gitignore");
        std::fs::create_dir_all(path.parent().expect("要有父目录")).expect("建 .lithe");
        let original = "# 我自己写的注释\n*.log\n*.local.json\n";
        std::fs::write(&path, original).expect("写初始文件");

        assert_eq!(
            ensure_local_ignore_file(&dir).expect("补齐失败"),
            IgnoreFileOutcome::Appended { added: 2 }
        );

        let text = std::fs::read_to_string(&path).expect("读 .lithe/.gitignore");
        assert!(
            text.starts_with(original),
            "用户的既有内容必须逐字保留在前：{text}"
        );
        assert_eq!(
            text.matches("*.local.json").count(),
            1,
            "已经存在的那条不得重复追加：{text}"
        );
        let tail: Vec<String> = text
            .lines()
            .skip(original.lines().count())
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect();
        assert_eq!(
            tail,
            vec!["run/classes/".to_string(), "**/*.tmp".to_string()],
            "缺的规则按 LOCAL_IGNORE_RULES 的顺序补在末尾"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 规则齐了就不再写：返回 `Unchanged`、文件**逐字不变**；顺序不同也算齐（判据是"在不在"）。
    #[test]
    fn local_ignore_file_is_idempotent_once_complete() {
        let dir = plain_dir("idempotent");
        assert_eq!(
            ensure_local_ignore_file(&dir).expect("第一次失败"),
            IgnoreFileOutcome::Created
        );
        let path = dir.join(".lithe/.gitignore");
        let first = std::fs::read_to_string(&path).expect("读");

        assert_eq!(
            ensure_local_ignore_file(&dir).expect("第二次失败"),
            IgnoreFileOutcome::Unchanged
        );
        assert_eq!(
            first,
            std::fs::read_to_string(&path).expect("读"),
            "规则齐了之后不得再改动文件"
        );
        assert!(
            !crate::document::tmp_path(&path).exists(),
            "Unchanged 这条路径上不该写任何东西"
        );

        std::fs::write(&path, "**/*.tmp\n*.local.json\nrun/classes/\n").expect("写重排后的文件");
        assert_eq!(
            ensure_local_ignore_file(&dir).expect("第三次失败"),
            IgnoreFileOutcome::Unchanged
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 判据是"trim 之后逐字相同"：带空白的那一行算命中，不会因为空格而被重复追加。
    #[test]
    fn local_ignore_file_matches_trimmed_lines() {
        let dir = plain_dir("trim");
        let path = dir.join(".lithe/.gitignore");
        std::fs::create_dir_all(path.parent().expect("要有父目录")).expect("建 .lithe");
        std::fs::write(&path, "   *.local.json   \n**/*.tmp\n").expect("写初始文件");

        assert_eq!(
            ensure_local_ignore_file(&dir).expect("补齐失败"),
            IgnoreFileOutcome::Appended { added: 1 }
        );

        let text = std::fs::read_to_string(&path).expect("读 .lithe/.gitignore");
        assert_eq!(
            text.matches("*.local.json").count(),
            1,
            "带空白的既有行算命中，不该重复追加：{text}"
        );
        assert_eq!(
            text.lines()
                .filter(|line| line.trim() == "run/classes/")
                .count(),
            1
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
