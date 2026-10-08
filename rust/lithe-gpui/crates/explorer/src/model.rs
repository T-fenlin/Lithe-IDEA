//! The Core reads behind the project tree, and the icons its rows draw.
//!
//! What stays here: the workspace snapshot (`workspace.snapshot`), the git porcelain that decorates
//! it (`git.status`), and the owner's own file-type icons. Everything the rows show themselves —
//! what is open, what is picked, the row budget, the git letters, the row menu — belongs to
//! `lithe_ui::lists::ProjectTree`.

use std::collections::BTreeSet;
use std::path::Path;

use gpui_kit::App;
use gpui_kit::SharedString;
use gpui_kit::assets::IconName;
use lithe_gpui_shared::core_json;
use lithe_gpui_shared::icons::file_icon;
use lithe_ui::lists::{GitStatus, ProjectIcon, ProjectIconRequest, ProjectNode, ProjectRowKind};
use serde_json::{Value, json};

/// One folder or file of the workspace snapshot, as Core sends it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TreeNode {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) is_directory: bool,
    pub(crate) children: Vec<TreeNode>,
}

impl TreeNode {
    pub(crate) fn count(&self) -> usize {
        1 + self.children.iter().map(Self::count).sum::<usize>()
    }

    fn collect_file_paths(&self, out: &mut BTreeSet<String>) {
        if !self.is_directory {
            out.insert(self.path.clone());
            return;
        }
        for child in &self.children {
            child.collect_file_paths(out);
        }
    }
}

#[derive(Debug)]
pub(crate) struct Snapshot {
    pub(crate) root: TreeNode,
    pub(crate) files: Vec<String>,
}

impl Snapshot {
    pub(crate) fn files_missing_from_root(&self) -> usize {
        let mut in_tree = BTreeSet::new();
        self.root.collect_file_paths(&mut in_tree);
        self.files
            .iter()
            .filter(|path| !in_tree.contains(path.as_str()))
            .count()
    }

    /// The workspace's top level as the tree component takes it; the root row itself is the label.
    pub(crate) fn project_nodes(&self) -> Vec<ProjectNode> {
        project_nodes(&self.root.children)
    }
}

/// A host node as a library node; folders keep their children, so an empty folder still shows.
fn to_project_node(node: &TreeNode) -> ProjectNode {
    match node.is_directory {
        true => ProjectNode::folder(node.path.as_str(), node.name.as_str())
            .children(node.children.iter().map(to_project_node)),
        false => ProjectNode::file(node.path.as_str(), node.name.as_str()),
    }
}

fn project_nodes(nodes: &[TreeNode]) -> Vec<ProjectNode> {
    nodes.iter().map(to_project_node).collect()
}

fn core_error_text(error: lithe_gpui_shared::CoreError) -> String {
    match error.code() {
        Some(code) => code.to_string(),
        None => error.to_string(),
    }
}

pub(crate) fn load_snapshot(root: &Path) -> Result<Snapshot, String> {
    let root_text = root
        .to_str()
        .ok_or_else(|| "工作区根不是合法 UTF-8 路径".to_string())?;

    let data =
        core_json("workspace.snapshot", json!({ "root": root_text })).map_err(core_error_text)?;
    parse_snapshot(data.as_ref().ok_or("响应缺少 data")?)
}

pub(crate) fn parse_snapshot(data: &Value) -> Result<Snapshot, String> {
    let root = data.get("root").ok_or("响应缺少 data.root")?;
    let files = data
        .get("files")
        .and_then(Value::as_array)
        .ok_or("响应缺少 data.files")?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    Ok(Snapshot {
        root: parse_node(root),
        files,
    })
}

fn parse_node(value: &Value) -> TreeNode {
    let is_directory = value
        .get("isDirectory")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let children = if is_directory {
        value
            .get("children")
            .and_then(Value::as_array)
            .map(|children| children.iter().map(parse_node).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    TreeNode {
        path: value
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        name: value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        is_directory,
        children,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StatusChange {
    path: String,
    status: String,
    untracked: bool,
}

/// The workspace's file states from a `git.status` payload, ready for the tree: workspace-relative
/// paths, one state per path (the first change of a path wins, as it always has).
pub(crate) fn parse_git_status(
    data: &Value,
    workspace_root: &Path,
) -> Vec<(SharedString, GitStatus)> {
    let repository_root = data.get("repositoryRoot").and_then(Value::as_str);
    let Some(prefix) = repository_prefix(repository_root, workspace_root) else {
        return Vec::new();
    };
    let changes: Vec<StatusChange> = data
        .get("changes")
        .and_then(Value::as_array)
        .map(|changes| changes.iter().filter_map(parse_change).collect())
        .unwrap_or_default();
    let mut files: Vec<(SharedString, GitStatus)> = Vec::new();
    for change in &changes {
        let path = join_prefix(&prefix, &change.path);
        if path.is_empty()
            || files
                .iter()
                .any(|(known, _)| known.as_ref() == path.as_str())
        {
            continue;
        }
        files.push((path.into(), classify(&change.status, change.untracked)));
    }
    files
}

/// The porcelain's two letters as the one vocabulary the tree and the file tree share: a conflict
/// first, then what the change is, and `Modified` when nothing more specific was said.
fn classify(status: &str, untracked: bool) -> GitStatus {
    let mut chars = status.chars();
    let index = chars.next().unwrap_or(' ');
    let worktree = chars.next().unwrap_or(' ');
    if is_conflicted(index, worktree) {
        return GitStatus::Conflicted;
    }
    if untracked || index == 'A' || worktree == 'A' {
        return GitStatus::Added;
    }
    if index == 'D' || worktree == 'D' {
        return GitStatus::Deleted;
    }
    if index == 'R' || worktree == 'R' {
        return GitStatus::Renamed;
    }
    if index == 'C' || worktree == 'C' {
        return GitStatus::Copied;
    }
    GitStatus::Modified
}

fn repository_prefix(repository_root: Option<&str>, workspace_root: &Path) -> Option<String> {
    let text = repository_root?;
    if text.is_empty() {
        return Some(String::new());
    }
    let path = Path::new(text);
    if path.is_absolute() {
        let relative = path.strip_prefix(workspace_root).ok()?;
        return Some(normalize_path(&relative.to_string_lossy()));
    }
    Some(normalize_path(text))
}

fn normalize_path(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let normalized = normalized.trim_matches('/');
    if normalized == "." {
        return String::new();
    }
    normalized.to_string()
}

fn join_prefix(prefix: &str, path: &str) -> String {
    let path = normalize_path(path);
    if prefix.is_empty() {
        return path;
    }
    if path.is_empty() {
        return prefix.to_string();
    }
    format!("{prefix}/{path}")
}

fn parse_change(value: &Value) -> Option<StatusChange> {
    Some(StatusChange {
        path: value.get("path").and_then(Value::as_str)?.to_string(),
        status: value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        untracked: value
            .get("untracked")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn is_conflicted(index: char, worktree: char) -> bool {
    index == 'U'
        || worktree == 'U'
        || (index == 'A' && worktree == 'A')
        || (index == 'D' && worktree == 'D')
}

pub(crate) fn load_git_status(root: &Path) -> Result<Vec<(SharedString, GitStatus)>, String> {
    let root_text = root
        .to_str()
        .ok_or_else(|| "工作区根不是合法 UTF-8 路径".to_string())?;
    let data = core_json("git.status", json!({ "root": root_text })).map_err(core_error_text)?;
    Ok(match data.as_ref() {
        Some(data) => parse_git_status(data, root),
        None => Vec::new(),
    })
}

/// The icon a row draws: the IDE's own file-type drawing when we have one, else the Lucide glyph
/// for that kind. An open, empty folder shows the plain file glyph it always has.
pub(crate) fn icon_for(request: &ProjectIconRequest, cx: &App) -> Option<ProjectIcon> {
    if request.kind == ProjectRowKind::EmptyFolder {
        return Some(ProjectIcon::Glyph(IconName::File.path()));
    }
    let is_dir = request.kind == ProjectRowKind::Directory;
    if let Some(path) =
        file_icon::theme_icon_path(request.name.as_str(), is_dir, request.expanded, cx)
        && file_icon::theme_icon_bytes(path, cx).is_some()
    {
        return Some(ProjectIcon::Image(path.into()));
    }
    let fallback = match is_dir {
        true if request.expanded => IconName::FolderOpen,
        true => IconName::FolderClosed,
        false => file_icon::lucide_fallback(request.name.as_str()),
    };
    Some(ProjectIcon::Glyph(fallback.path()))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use lithe_ui::lists::GitStatus;
    use serde_json::json;

    use super::{parse_git_status, parse_snapshot, project_nodes};

    fn snapshot_root() -> serde_json::Value {
        json!({
            "root": {
                "path": "",
                "name": "workspace",
                "isDirectory": true,
                "children": [
                    { "path": "src", "name": "src", "isDirectory": true, "children": [
                        { "path": "src/main.rs", "name": "main.rs", "isDirectory": false }
                    ]},
                    { "path": "logs", "name": "logs", "isDirectory": true, "children": [] }
                ]
            },
            "files": ["src/main.rs", "ghost.txt"]
        })
    }

    #[test]
    fn parse_snapshot_rejects_incomplete_payloads() {
        assert_eq!(
            parse_snapshot(&json!({ "files": [] })).unwrap_err(),
            "响应缺少 data.root"
        );
        assert_eq!(
            parse_snapshot(&json!({ "root": {} })).unwrap_err(),
            "响应缺少 data.files"
        );
    }

    #[test]
    fn empty_directories_from_data_root_become_nodes() {
        let snapshot = parse_snapshot(&snapshot_root()).expect("快照应当解析成功");
        let nodes = snapshot.project_nodes();
        let logs = nodes
            .iter()
            .find(|node| node.path() == "logs")
            .expect("空目录也要成为节点");
        assert!(logs.is_dir(), "空目录是文件夹");
        assert!(logs.nodes().is_empty(), "空目录没有子节点");
        assert_eq!(logs.name(), "logs");
    }

    #[test]
    fn flat_files_cannot_express_empty_directories() {
        let snapshot = parse_snapshot(&json!({
            "root": {
                "path": "",
                "name": "workspace",
                "isDirectory": true,
                "children": [
                    { "path": "src", "name": "src", "isDirectory": true, "children": [
                        { "path": "src/main.rs", "name": "main.rs", "isDirectory": false }
                    ]}
                ]
            },
            "files": ["src/main.rs"]
        }))
        .expect("快照应当解析成功");
        // 反证：只用 `files` 聚树时，`logs` 这样的空目录根本不存在。
        let from_files: Vec<&str> = snapshot.files.iter().map(String::as_str).collect();
        assert!(!from_files.iter().any(|path| path.starts_with("logs")));
        let nodes = project_nodes(&snapshot.root.children);
        assert_eq!(nodes.len(), 1, "只有 src 一个顶层节点");
    }

    #[test]
    fn root_wins_when_files_and_root_disagree() {
        let snapshot = parse_snapshot(&snapshot_root()).expect("快照应当解析成功");
        let nodes = snapshot.project_nodes();
        assert!(
            !nodes.iter().any(|node| node.path() == "ghost.txt"),
            "只出现在 `files` 里的路径不许成为行"
        );
        assert_eq!(snapshot.files_missing_from_root(), 1);
    }

    fn status(data: serde_json::Value) -> Vec<(String, GitStatus)> {
        parse_git_status(&data, Path::new("/workspace"))
            .into_iter()
            .map(|(path, status)| (path.to_string(), status))
            .collect()
    }

    #[test]
    fn the_porcelain_classifies_every_state() {
        let cases = [
            (" M", false, GitStatus::Modified),
            ("M ", false, GitStatus::Modified),
            ("??", true, GitStatus::Added),
            (" A", false, GitStatus::Added),
            ("A ", false, GitStatus::Added),
            (" D", false, GitStatus::Deleted),
            ("R ", false, GitStatus::Renamed),
            ("C ", false, GitStatus::Copied),
            ("UU", false, GitStatus::Conflicted),
            ("AA", false, GitStatus::Conflicted),
            ("DD", false, GitStatus::Conflicted),
        ];
        for (code, untracked, expected) in cases {
            let decorated = status(json!({
                "repositoryRoot": "",
                "changes": [{ "path": "src/a.rs", "status": code, "untracked": untracked }]
            }));
            assert_eq!(
                decorated,
                vec![("src/a.rs".to_string(), expected)],
                "{code} (untracked={untracked})"
            );
        }
    }

    #[test]
    fn repository_prefix_is_joined_back_onto_workspace_relative_paths() {
        let decorated = status(json!({
            "repositoryRoot": "app",
            "changes": [{ "path": "src/a.txt", "status": "A ", "untracked": false }]
        }));
        assert_eq!(
            decorated,
            vec![("app/src/a.txt".to_string(), GitStatus::Added)]
        );
    }

    #[test]
    fn repository_root_equal_to_workspace_root_adds_no_prefix() {
        for root in ["", "."] {
            let decorated = status(json!({
                "repositoryRoot": root,
                "changes": [{ "path": "a.txt", "status": "A ", "untracked": false }]
            }));
            assert_eq!(
                decorated,
                vec![("a.txt".to_string(), GitStatus::Added)],
                "repositoryRoot = {root:?}"
            );
        }
    }

    #[test]
    fn no_repository_and_no_changes_produce_no_decorations() {
        assert!(
            status(json!({ "repositoryRoot": null, "changes": [] })).is_empty(),
            "workspace 自己不是仓库 ⇒ 没有装饰"
        );
        assert!(status(json!({ "repositoryRoot": "", "changes": [] })).is_empty());
    }

    #[test]
    fn absolute_repository_root_maps_only_when_under_the_workspace() {
        // 用当前目录拼出绝对路径，Windows 与 Unix 上的 `is_absolute()` 判定才都成立。
        let workspace = std::env::current_dir().expect("当前目录").join("workspace");
        let inside = parse_git_status(
            &json!({
                "repositoryRoot": workspace.join("app").to_str().expect("UTF-8"),
                "changes": [{ "path": "a.txt", "status": "A ", "untracked": false }]
            }),
            &workspace,
        );
        assert_eq!(
            inside
                .into_iter()
                .map(|(path, status)| (path.to_string(), status))
                .collect::<Vec<_>>(),
            vec![("app/a.txt".to_string(), GitStatus::Added)]
        );

        let outside = parse_git_status(
            &json!({
                "repositoryRoot": std::env::current_dir()
                    .expect("当前目录")
                    .join("elsewhere/app")
                    .to_str()
                    .expect("UTF-8"),
                "changes": [{ "path": "a.txt", "status": "A ", "untracked": false }]
            }),
            &workspace,
        );
        assert!(outside.is_empty(), "仓库在 workspace 之外 ⇒ 整份状态作废");
    }

    #[test]
    fn duplicate_and_empty_paths_are_handled() {
        let decorated = status(json!({
            "repositoryRoot": "",
            "changes": [
                { "path": "src/a.rs", "status": " M", "untracked": false },
                { "path": "src/a.rs", "status": " D", "untracked": false },
                { "path": "", "status": " M", "untracked": false }
            ]
        }));
        assert_eq!(
            decorated,
            vec![("src/a.rs".to_string(), GitStatus::Modified)],
            "同一路径的第一条胜出，空路径不成行"
        );
    }

    #[test]
    fn deep_paths_keep_their_workspace_relative_form() {
        let decorated = status(json!({
            "repositoryRoot": "app",
            "changes": [{ "path": "src/main/java/App.java", "status": " M", "untracked": false }]
        }));
        assert_eq!(
            decorated,
            vec![(
                "app/src/main/java/App.java".to_string(),
                GitStatus::Modified
            )]
        );
    }
}
