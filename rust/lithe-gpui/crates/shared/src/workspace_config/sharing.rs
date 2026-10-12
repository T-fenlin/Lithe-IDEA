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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnsureExcluded {
    /// 排除规则已经写入 `.git/info/exclude` 并回读确认。
    Ensured,
    /// 当前目录不是 Git 工作区，没有排除目标。
    NotARepository,
    /// 尝试写入 `.git/info/exclude` 但被拒绝，目录没有被排除。
    /// 调用方应当继续保存业务数据，而不是让整次保存失败。
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareOutcome {
    Staged { files: Vec<String> },
    NothingToShare,
    NotARepository,
}

#[derive(Debug)]
pub enum SharingError {
    Git(CoreError),
    NotPersisted {
        operation: &'static str,
        pattern: String,
        path: PathBuf,
        reason: String,
    },
}

impl SharingError {
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

pub fn pattern_is_present(exclude_file: &Path, pattern: &str) -> io::Result<bool> {
    let text = match std::fs::read_to_string(exclude_file) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let wanted = pattern.trim();
    Ok(text
        .lines()
        .map(str::trim)
        .any(|line| !line.is_empty() && !line.starts_with('#') && line == wanted))
}

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

pub fn ensure_project_dir_excluded(root: &Path) -> Result<EnsureExcluded, SharingError> {
    let pattern = workspace_config_exclude_pattern();

    let Some(exclude_file) = local_exclude_path(root).map_err(SharingError::Git)? else {
        return Ok(EnsureExcluded::NotARepository);
    };

    match write_ignore_patterns(root, "excludePatterns", &pattern) {
        Ok(()) => {}
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

/// 把 `ensure_project_dir_excluded` 的失败降级成 [`EnsureExcluded::Failed`]。
///
/// `.lithe/` 没有写进 `.git/info/exclude` 只会让工作区在 `git status` 里变脏，
/// 不该让会话、项目清单这类业务数据的保存整体失败。失败仍然打诊断日志，
/// 便于在 Core 补齐 git 写能力后定位哪些仓库受影响。
///
/// `scope` 用于区分调用来源（例如 `session`、`project`），让日志能指回具体写入方。
pub fn degrade_exclusion_failure(
    result: Result<EnsureExcluded, SharingError>,
    root: &Path,
    scope: &str,
) -> EnsureExcluded {
    match result {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!(
                "{DIAGNOSTIC_PREFIX} exclusion_degraded scope={scope} root={} error={error}",
                root.display()
            );
            EnsureExcluded::Failed
        }
    }
}

pub const LOCAL_IGNORE_RULES: &[&str] = &["*.local.json", "run/classes/", "**/*.tmp"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreFileOutcome {
    Created,
    Appended { added: usize },
    Unchanged,
}

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

const MAX_WALK_DEPTH: usize = 6;

const MAX_MEMBERS: usize = 512;

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

fn write_ignore_patterns(root: &Path, operation: &str, pattern: &str) -> Result<(), CoreError> {
    write_git_paths(root, operation, &[pattern.to_string()])
}

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

const GIT_WRITE_TIMEOUT_MILLIS: u64 = 60_000;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_config::test_repo::{TempRepo, is_inside_work_tree};

    #[test]
    fn ensure_excluded_hides_the_config_directory_from_git_status() {
        let repo = TempRepo::new("ensure");
        repo.write(".lithe/project.json", "{\"id\":\"x\",\"version\":1}");

        let before = repo.status();
        assert!(before.contains(".lithe"), "前置条件不成立：{before}");

        let exclude = repo.git_path("info/exclude");
        std::fs::write(&exclude, "# 别人的规则\n*.log\n").expect("写初始排除文件");

        assert_eq!(
            ensure_project_dir_excluded(repo.root()).expect("守卫失败"),
            EnsureExcluded::Ensured
        );
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

    #[test]
    fn sharing_exposes_only_shareable_members() {
        let repo = TempRepo::new("share");
        repo.write(".lithe/settings.json", "{}");
        repo.write(".lithe/.gitignore", "*.local.json\n");
        repo.write(".lithe/run/configurations.json", "{}");
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

        let exclude = repo.git_path("info/exclude");
        assert!(
            !repo
                .exclude_lines_at(&exclude)
                .iter()
                .any(|line| line == ".lithe/"),
            "共享之后排除行必须被移除"
        );

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

    #[test]
    fn nothing_to_share_leaves_the_exclusion_in_place() {
        let repo = TempRepo::new("nothing");
        assert_eq!(
            ensure_project_dir_excluded(repo.root()).expect("守卫失败"),
            EnsureExcluded::Ensured
        );

        assert_eq!(
            share_project_config(repo.root()).expect("共享失败"),
            ShareOutcome::NothingToShare
        );
        assert!(
            repo.exclude_lines().iter().any(|line| line == ".lithe/"),
            "没东西可共享时不得移除排除行"
        );

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

    #[test]
    fn pattern_is_present_reads_the_documented_cases() {
        let dir =
            std::env::temp_dir().join(format!("lithe-sharing-readback-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        let file = dir.join("exclude");

        assert!(!pattern_is_present(&file, ".lithe/").expect("读失败"));

        std::fs::write(&file, "# 别人的规则\n*.log\n   .lithe/   \n").expect("写文件");
        assert!(pattern_is_present(&file, ".lithe/").expect("读失败"));

        std::fs::write(&file, "# .lithe/\n*.log\n").expect("写文件");
        assert!(!pattern_is_present(&file, ".lithe/").expect("读失败"));

        std::fs::write(&file, ".lithe\n.lithe/run/\n").expect("写文件");
        assert!(!pattern_is_present(&file, ".lithe/").expect("读失败"));

        std::fs::write(&file, "\n   \n").expect("写文件");
        assert!(!pattern_is_present(&file, ".lithe/").expect("读失败"));

        let _ = std::fs::remove_dir_all(&dir);
    }

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
        let text = std::fs::read_to_string(&exclude).expect("读排除文件");
        assert!(
            text.contains("# 别人的规则") && text.contains("*.log"),
            "{text}"
        );
    }

    fn plain_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lithe-workspace-config-ignore-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        dir
    }

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
        let expected: Vec<String> = LOCAL_IGNORE_RULES
            .iter()
            .map(|rule| rule.to_string())
            .collect();
        assert_eq!(
            rules, expected,
            "第二道闸的内容必须与 LOCAL_IGNORE_RULES 一字对应"
        );

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

    #[test]
    fn exclusion_failure_degrades_instead_of_blocking_the_save() {
        let root = std::env::temp_dir();

        let ok = degrade_exclusion_failure(Ok(EnsureExcluded::Ensured), &root, "session");
        assert_eq!(ok, EnsureExcluded::Ensured, "成功时必须原样透传");

        let error = SharingError::NotPersisted {
            operation: "excludePatterns",
            pattern: ".lithe/".to_string(),
            path: root.join(".git/info/exclude"),
            reason: "测试注入的失败".to_string(),
        };
        let degraded = degrade_exclusion_failure(Err(error), &root, "session");
        assert_eq!(
            degraded,
            EnsureExcluded::Failed,
            "排除失败必须降级成 Failed，让会话 / 项目清单 / 工具链配置的保存继续完成"
        );

        let not_repo =
            degrade_exclusion_failure(Ok(EnsureExcluded::NotARepository), &root, "session");
        assert_eq!(not_repo, EnsureExcluded::NotARepository);
    }
}
