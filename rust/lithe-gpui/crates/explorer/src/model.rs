use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::tree::TreeItem;
use gpui_kit::{App, Hsla, SharedString};
use lithe_gpui_shared::icons::{FileIcon, file_icon};
use lithe_gpui_shared::{core_json, tr};
use serde_json::{Value, json};

pub(crate) const RENDER_LIMIT: usize = 5_000;

pub(crate) const ROOT_ID: &str = "dir:";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RowKind {
    File,
    Directory,
    EmptyPlaceholder,
}

impl RowKind {
    pub(crate) fn parse(id: &str) -> (Self, String) {
        match id.split_once(':') {
            Some(("file", rest)) => (Self::File, rest.to_string()),
            Some(("empty", rest)) => (Self::EmptyPlaceholder, rest.to_string()),
            Some(("dir", rest)) => (Self::Directory, rest.to_string()),
            _ => (Self::Directory, id.to_string()),
        }
    }
}

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
}

pub(crate) fn empty_placeholder(path: &str) -> TreeItem {
    TreeItem::new(
        SharedString::from(format!("empty:{path}")),
        tr("lithe.fileExplorer.folderIsEmpty"),
    )
    .disabled(true)
}

pub(crate) struct BuiltTree {
    pub(crate) items: Vec<TreeItem>,
    pub(crate) rows: usize,
    pub(crate) truncated: bool,
}

struct RowBudget {
    remaining: usize,
    truncated: bool,
}

impl RowBudget {
    fn new() -> Self {
        Self {
            remaining: RENDER_LIMIT,
            truncated: false,
        }
    }

    fn take(&mut self) -> bool {
        if self.remaining == 0 {
            self.truncated = true;
            false
        } else {
            self.remaining -= 1;
            true
        }
    }

    fn rows(&self) -> usize {
        RENDER_LIMIT - self.remaining
    }
}

pub(crate) fn build_tree_items(
    root_label: &str,
    root: &TreeNode,
    expanded: &BTreeSet<SharedString>,
    filter: Option<&str>,
) -> BuiltTree {
    let query = filter.map(str::to_lowercase);
    let force_expanded = query.is_some();
    let mut budget = RowBudget::new();

    // 根行同样占一行预算（`rows` 是"materialize 了多少行"，含根）。
    if !budget.take() {
        return BuiltTree {
            items: Vec::new(),
            rows: budget.rows(),
            truncated: budget.truncated,
        };
    }

    // 广度优先：行预算先买**浅层**的行。原先的"深度优先 + 每个目录无条件递归"会让根下
    // 第一个大目录（本仓库的 `.artifacts` 有上万节点）把 `RENDER_LIMIT` 吃光，根的同级
    // 目录在 `break` 处被整段丢掉 —— 用户看到"顶层目录只剩前两个"。
    //
    // 折叠目录的子项仍然要物化：`TreeState` 用"有没有子项"判定能不能展开，不带子项的
    // 目录会变成不能展开的死行。所以只能改**取用顺序**，不能改成"只建可见行"。
    let mut nodes: Vec<Option<&TreeNode>> = Vec::new();
    let mut items: Vec<Option<TreeItem>> = Vec::new();
    let mut kids: Vec<Vec<usize>> = Vec::new();
    let mut queue: VecDeque<usize> = VecDeque::new();

    let root_id = SharedString::new_static(ROOT_ID);
    let root_expanded = force_expanded || expanded.contains(&root_id);
    nodes.push(Some(root));
    items.push(Some(
        TreeItem::new(root_id, SharedString::from(root_label.to_string())).expanded(root_expanded),
    ));
    kids.push(Vec::new());
    queue.push_back(0);

    while let Some(slot) = queue.pop_front() {
        let Some(node) = nodes[slot] else { continue };
        let visible = visible_children(node, query.as_deref());
        if visible.is_empty() {
            // 空目录，或过滤后一条都不命中：给一条禁用占位行，让它画得出展开箭头。
            if budget.take() {
                let ix = items.len();
                nodes.push(None);
                items.push(Some(empty_placeholder(&node.path)));
                kids.push(Vec::new());
                kids[slot].push(ix);
            }
            continue;
        }
        for child in visible {
            if !budget.take() {
                break;
            }
            let id = node_id(child);
            let item = TreeItem::new(id.clone(), SharedString::from(child.name.clone()))
                .expanded(force_expanded || expanded.contains(&id));
            let ix = items.len();
            nodes.push(Some(child));
            items.push(Some(item));
            kids.push(Vec::new());
            kids[slot].push(ix);
            if child.is_directory {
                queue.push_back(ix);
            }
        }
    }

    // 组装：子树在 arena 里永远排在父之后，所以倒着收一次就能先把孩子收好。
    for slot in (0..items.len()).rev() {
        if kids[slot].is_empty() {
            continue;
        }
        let children: Vec<TreeItem> = kids[slot]
            .iter()
            .map(|&ix| items[ix].take().expect("每个槽只组装一次"))
            .collect();
        let mut item = items[slot].take().expect("每个槽只组装一次");
        item.children = children;
        items[slot] = Some(item);
    }

    BuiltTree {
        items: vec![items[0].take().expect("根槽一定在")],
        rows: budget.rows(),
        truncated: budget.truncated,
    }
}

fn node_id(node: &TreeNode) -> SharedString {
    SharedString::from(if node.is_directory {
        format!("dir:{}", node.path)
    } else {
        format!("file:{}", node.path)
    })
}

/// `node` 的子项里，按显示顺序留下命中过滤的那些。
fn visible_children<'a>(node: &'a TreeNode, query: Option<&str>) -> Vec<&'a TreeNode> {
    let mut visible: Vec<&TreeNode> = node
        .children
        .iter()
        .filter(|child| query.is_none_or(|needle| subtree_matches(child, needle)))
        .collect();
    sort_for_display(&mut visible);
    visible
}

fn subtree_matches(node: &TreeNode, query: &str) -> bool {
    node.path.to_lowercase().contains(query)
        || node
            .children
            .iter()
            .any(|child| subtree_matches(child, query))
}

fn sort_for_display(nodes: &mut [&TreeNode]) {
    nodes.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
}

#[allow(dead_code)]
pub(crate) fn displayed_siblings(root: &TreeNode, focused: Option<&str>) -> Vec<String> {
    let parent = focused
        .and_then(|path| displayed_parent_of(root, path))
        .unwrap_or(root);
    let mut children: Vec<&TreeNode> = parent.children.iter().collect();
    sort_for_display(&mut children);
    children.into_iter().map(|node| node.path.clone()).collect()
}

fn displayed_parent_of<'a>(node: &'a TreeNode, path: &str) -> Option<&'a TreeNode> {
    for child in &node.children {
        if child.path == path {
            return Some(node);
        }
        if let Some(parent) = displayed_parent_of(child, path) {
            return Some(parent);
        }
    }
    None
}

#[allow(dead_code)]
pub(crate) fn visible_nodes<'a>(
    root: &'a TreeNode,
    expanded: &BTreeSet<SharedString>,
) -> Vec<&'a TreeNode> {
    fn walk<'a>(
        node: &'a TreeNode,
        expanded: &BTreeSet<SharedString>,
        is_root: bool,
        out: &mut Vec<&'a TreeNode>,
    ) {
        out.push(node);
        if !node.is_directory {
            return;
        }
        let id = if is_root {
            SharedString::new_static(ROOT_ID)
        } else {
            SharedString::from(format!("dir:{}", node.path))
        };
        if !expanded.contains(&id) {
            return;
        }
        for child in &node.children {
            walk(child, expanded, false, out);
        }
    }

    let mut out = Vec::new();
    walk(root, expanded, true, &mut out);
    out
}

#[allow(dead_code)]
pub(crate) fn selected_in_display_order(
    root: &TreeNode,
    expanded: &BTreeSet<SharedString>,
    selected: &BTreeSet<String>,
) -> Vec<String> {
    visible_nodes(root, expanded)
        .into_iter()
        .filter(|node| !node.path.is_empty() && selected.contains(&node.path))
        .map(|node| node.path.clone())
        .collect()
}

#[allow(dead_code)]
pub(crate) fn paste_destination(
    root: &TreeNode,
    expanded: &BTreeSet<SharedString>,
    focused: Option<&str>,
) -> String {
    let Some(focused) = focused else {
        return String::new();
    };
    let Some(node) = visible_nodes(root, expanded)
        .into_iter()
        .find(|node| node.path == focused)
    else {
        return String::new();
    };
    if node.is_directory {
        return node.path.clone();
    }
    match node.path.rsplit_once('/') {
        Some((parent, _)) => parent.to_string(),
        None => String::new(),
    }
}

#[cfg(test)]
mod displayed_siblings_tests {
    use super::*;

    fn node(path: &str, is_directory: bool, children: Vec<TreeNode>) -> TreeNode {
        let name = match path.rsplit('/').next() {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => "proj".to_string(),
        };
        TreeNode {
            path: path.to_string(),
            name,
            is_directory,
            children,
        }
    }

    fn tree() -> TreeNode {
        node(
            "",
            true,
            vec![
                node(
                    "src",
                    true,
                    vec![
                        node("src/lib.rs", false, vec![]),
                        node("src/main.rs", false, vec![]),
                    ],
                ),
                node("a.rs", false, vec![]),
                node("B.rs", false, vec![]),
            ],
        )
    }

    #[test]
    fn without_focus_the_top_level_items_are_taken() {
        let root = tree();
        assert_eq!(displayed_siblings(&root, None), ["src", "a.rs", "B.rs"]);
        assert_eq!(
            displayed_siblings(&root, Some("does/not/exist")),
            ["src", "a.rs", "B.rs"],
            "焦点找不到时回落根"
        );
    }

    #[test]
    fn focus_inside_a_folder_takes_that_folder_siblings() {
        let root = tree();
        assert_eq!(
            displayed_siblings(&root, Some("src/lib.rs")),
            ["src/lib.rs", "src/main.rs"],
            "焦点在 src/lib.rs ⇒ 兄弟是 src 的两个子项"
        );
        assert_eq!(
            displayed_siblings(&root, Some("src")),
            ["src", "a.rs", "B.rs"],
            "焦点在顶层项 src ⇒ 兄弟是顶层那一层"
        );
    }

    #[test]
    fn focus_on_the_root_takes_the_top_level_items() {
        let root = tree();
        assert_eq!(displayed_siblings(&root, Some("")), ["src", "a.rs", "B.rs"]);
    }

    fn expanded_set(paths: &[&str]) -> BTreeSet<SharedString> {
        let mut set = BTreeSet::new();
        set.insert(SharedString::new_static(ROOT_ID));
        for path in paths {
            set.insert(SharedString::from(format!("dir:{path}")));
        }
        set
    }

    fn selected(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|path| (*path).to_string()).collect()
    }

    #[test]
    fn copy_payload_takes_exactly_the_selected_visible_items() {
        let root = tree();
        let expanded = expanded_set(&["src"]);
        assert_eq!(
            selected_in_display_order(&root, &expanded, &selected(&["src"])),
            ["src"],
            "选中文件夹 ⇒ 载荷只有文件夹本身，不含 src/lib.rs、src/main.rs"
        );
        let collapsed = expanded_set(&[]);
        assert_eq!(
            selected_in_display_order(&root, &collapsed, &selected(&["src/lib.rs"])),
            Vec::<String>::new(),
            "不可见的项不进载荷"
        );
        assert_eq!(
            selected_in_display_order(&root, &expanded, &selected(&["a.rs", "src", "B.rs"])),
            ["src", "a.rs", "B.rs"],
            "混合选择按显示顺序（目录在前）"
        );
        assert_eq!(
            selected_in_display_order(&root, &expanded, &selected(&[""])),
            Vec::<String>::new(),
            "工作区根永远不进载荷（真源 `$0.url != root.url`）"
        );
    }

    #[test]
    fn paste_destination_follows_the_focused_node() {
        let root = tree();
        let expanded = expanded_set(&["src"]);
        assert_eq!(paste_destination(&root, &expanded, Some("src")), "src");
        assert_eq!(
            paste_destination(&root, &expanded, Some("src/lib.rs")),
            "src",
            "文件 ⇒ 它所在的目录"
        );
        assert_eq!(
            paste_destination(&root, &expanded, Some("a.rs")),
            "",
            "顶层文件 ⇒ 工作区根"
        );
        assert_eq!(paste_destination(&root, &expanded, None), "");
        assert_eq!(
            paste_destination(&root, &expanded, Some("nope/x")),
            "",
            "焦点在可见树里找不到 ⇒ 根"
        );
    }
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum GitTone {
    Conflicted,
    Added,
    Deleted,
    Moved,
    Copied,
    Modified,
}

impl GitTone {
    fn priority(self) -> u8 {
        match self {
            Self::Modified => 0,
            Self::Copied => 1,
            Self::Moved => 2,
            Self::Added => 3,
            Self::Deleted => 4,
            Self::Conflicted => 5,
        }
    }

    pub(crate) fn color(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        match self {
            Self::Modified => theme.accent,
            Self::Added | Self::Copied => theme.success,
            Self::Deleted => theme.danger,
            Self::Moved => theme.magenta_light,
            Self::Conflicted => theme.warning,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct GitDecoration {
    pub(crate) letter: SharedString,
    pub(crate) tone: GitTone,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StatusChange {
    pub(crate) path: String,
    pub(crate) status: String,
    pub(crate) untracked: bool,
}

#[derive(Clone, Default)]
pub(crate) struct GitTreeStatus {
    files: BTreeMap<String, GitDecoration>,
    directories: BTreeMap<String, GitTone>,
}

impl GitTreeStatus {
    pub(crate) fn from_status(data: &Value, workspace_root: &Path) -> Self {
        let repository_root = data.get("repositoryRoot").and_then(Value::as_str);
        let Some(prefix) = repository_prefix(repository_root, workspace_root) else {
            return Self::default();
        };
        let changes = data
            .get("changes")
            .and_then(Value::as_array)
            .map(|changes| changes.iter().filter_map(parse_change).collect::<Vec<_>>())
            .unwrap_or_default();
        Self::from_changes(&prefix, &changes)
    }

    pub(crate) fn from_changes(prefix: &str, changes: &[StatusChange]) -> Self {
        let mut files: BTreeMap<String, GitDecoration> = BTreeMap::new();
        let mut directories: BTreeMap<String, GitTone> = BTreeMap::new();
        for change in changes {
            let path = join_prefix(prefix, &change.path);
            if path.is_empty() {
                continue;
            }
            let tone = git_tone(&change.status, change.untracked);
            files.entry(path.clone()).or_insert_with(|| GitDecoration {
                letter: SharedString::from(display_status(&change.status, change.untracked)),
                tone,
            });
            bump_ancestors(&mut directories, &path, tone);
            bump_directory(&mut directories, "", tone);
        }
        Self { files, directories }
    }

    pub(crate) fn file(&self, path: &str) -> Option<&GitDecoration> {
        self.files.get(path)
    }

    pub(crate) fn directory(&self, path: &str) -> Option<GitTone> {
        self.directories.get(path).copied()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.files.is_empty() && self.directories.is_empty()
    }

    pub(crate) fn counts(&self) -> (usize, usize) {
        (self.files.len(), self.directories.len())
    }
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

fn bump_ancestors(map: &mut BTreeMap<String, GitTone>, path: &str, tone: GitTone) {
    let mut remainder = path;
    while let Some(slash) = remainder.rfind('/') {
        remainder = &remainder[..slash];
        bump_directory(map, remainder, tone);
    }
}

fn bump_directory(map: &mut BTreeMap<String, GitTone>, directory: &str, tone: GitTone) {
    match map.get(directory) {
        Some(current) if current.priority() >= tone.priority() => {}
        _ => {
            map.insert(directory.to_string(), tone);
        }
    }
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

fn display_status(status: &str, untracked: bool) -> String {
    let mut chars = status.chars();
    let index = chars.next().unwrap_or(' ');
    let worktree = chars.next().unwrap_or(' ');
    if is_conflicted(index, worktree) {
        return "!".to_string();
    }
    if untracked {
        return "A".to_string();
    }
    if worktree != ' ' {
        return worktree.to_string();
    }
    index.to_string()
}

fn git_tone(status: &str, untracked: bool) -> GitTone {
    let mut chars = status.chars();
    let index = chars.next().unwrap_or(' ');
    let worktree = chars.next().unwrap_or(' ');
    if is_conflicted(index, worktree) {
        return GitTone::Conflicted;
    }
    if untracked || index == 'A' || worktree == 'A' {
        return GitTone::Added;
    }
    if index == 'D' || worktree == 'D' {
        return GitTone::Deleted;
    }
    if index == 'R' || worktree == 'R' {
        return GitTone::Moved;
    }
    if index == 'C' || worktree == 'C' {
        return GitTone::Copied;
    }
    GitTone::Modified
}

pub(crate) fn load_git_status(root: &Path) -> Result<GitTreeStatus, String> {
    let root_text = root
        .to_str()
        .ok_or_else(|| "工作区根不是合法 UTF-8 路径".to_string())?;
    let data = core_json("git.status", json!({ "root": root_text })).map_err(core_error_text)?;
    Ok(match data.as_ref() {
        Some(data) => GitTreeStatus::from_status(data, root),
        None => GitTreeStatus::default(),
    })
}

pub(crate) fn icon_for_file(name: &str, cx: &App) -> FileIcon {
    FileIcon::lucide(file_icon::lucide_fallback(name)).with_theme_icon(name, cx)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_json() -> Value {
        json!({
            "root": {
                "path": "",
                "name": "demo",
                "isDirectory": true,
                "children": [
                    {
                        "path": "logs",
                        "name": "logs",
                        "isDirectory": true,
                        "children": []
                    },
                    {
                        "path": "src",
                        "name": "src",
                        "isDirectory": true,
                        "children": [
                            {
                                "path": "src/empty-folder",
                                "name": "empty-folder",
                                "isDirectory": true,
                                "children": []
                            },
                            {
                                "path": "src/main.rs",
                                "name": "main.rs",
                                "isDirectory": false
                            }
                        ]
                    }
                ]
            },
            "files": ["src/main.rs"]
        })
    }

    fn snapshot() -> Snapshot {
        parse_snapshot(&snapshot_json()).expect("夹具必须是合法的 data")
    }

    fn row_ids(built: &BuiltTree) -> Vec<String> {
        fn walk(items: &[TreeItem], out: &mut Vec<String>) {
            for item in items {
                out.push(item.id.to_string());
                walk(&item.children, out);
            }
        }
        let mut out = Vec::new();
        walk(&built.items, &mut out);
        out
    }

    fn disabled_ids(built: &BuiltTree) -> Vec<String> {
        fn walk(items: &[TreeItem], out: &mut Vec<String>) {
            for item in items {
                if item.is_disabled() {
                    out.push(item.id.to_string());
                }
                walk(&item.children, out);
            }
        }
        let mut out = Vec::new();
        walk(&built.items, &mut out);
        out
    }

    #[test]
    fn empty_directories_from_data_root_become_rows() {
        let snapshot = snapshot();
        let expanded = BTreeSet::new();
        let built = build_tree_items("demo", &snapshot.root, &expanded, None);

        let ids = row_ids(&built);
        assert!(
            ids.contains(&"dir:logs".to_string()),
            "根下的空目录必须成行，实际 = {ids:?}"
        );
        assert!(
            ids.contains(&"dir:src/empty-folder".to_string()),
            "二级空目录必须成行，实际 = {ids:?}"
        );
        assert!(
            ids.contains(&"file:src/main.rs".to_string()),
            "实际 = {ids:?}"
        );
    }

    #[test]
    fn empty_directory_rows_are_expandable_directories_with_a_disabled_placeholder() {
        let snapshot = snapshot();
        let built = build_tree_items("demo", &snapshot.root, &BTreeSet::new(), None);

        let logs = built
            .items
            .iter()
            .flat_map(|item| item.children.iter())
            .find(|item| item.id.as_ref() == "dir:logs")
            .expect("dir:logs 必须存在");
        assert!(
            logs.is_folder(),
            "空目录行必须是可展开的目录行（is_folder = 有子项）"
        );
        assert_eq!(
            logs.children
                .iter()
                .map(|child| child.id.to_string())
                .collect::<Vec<_>>(),
            vec!["empty:logs".to_string()],
            "空目录下应只有那一条占位行"
        );
        assert_eq!(
            logs.children[0].label.as_ref(),
            tr("lithe.fileExplorer.folderIsEmpty").as_ref(),
            "占位行文案必须走 catalog 既有键 lithe.fileExplorer.folderIsEmpty"
        );
        assert_eq!(
            disabled_ids(&built),
            vec![
                "empty:logs".to_string(),
                "empty:src/empty-folder".to_string()
            ]
        );
    }

    #[test]
    fn flat_files_cannot_express_empty_directories() {
        let snapshot = snapshot();
        assert_eq!(
            snapshot.files,
            vec!["src/main.rs".to_string()],
            "扁平列表里只有文件"
        );
        let synthesized = TreeNode {
            path: String::new(),
            name: "demo".to_string(),
            is_directory: true,
            children: vec![TreeNode {
                path: "src".to_string(),
                name: "src".to_string(),
                is_directory: true,
                children: vec![TreeNode {
                    path: "src/main.rs".to_string(),
                    name: "main.rs".to_string(),
                    is_directory: false,
                    children: Vec::new(),
                }],
            }],
        };
        let built = build_tree_items("demo", &synthesized, &BTreeSet::new(), None);
        let ids = row_ids(&built);
        assert!(!ids.contains(&"dir:logs".to_string()));
        assert!(!ids.contains(&"dir:src/empty-folder".to_string()));
    }

    #[test]
    fn root_wins_when_files_and_root_disagree() {
        let mut data = snapshot_json();
        data["files"] = json!(["src/main.rs", "ghost.txt"]);
        let snapshot = parse_snapshot(&data).expect("夹具必须是合法的 data");

        let built = build_tree_items("demo", &snapshot.root, &BTreeSet::new(), None);
        let ids = row_ids(&built);
        assert!(
            ids.contains(&"dir:logs".to_string()),
            "root 里的空目录要成行"
        );
        assert!(
            !ids.contains(&"file:ghost.txt".to_string()),
            "只出现在 files 里的路径不许成行（以 root 为准），实际 = {ids:?}"
        );
        assert_eq!(
            snapshot.files_missing_from_root(),
            1,
            "不一致要如实计数（诊断用），不许静默"
        );
    }

    #[test]
    fn rows_are_folders_first_then_lowercase_name_order() {
        let root = TreeNode {
            path: String::new(),
            name: "demo".to_string(),
            is_directory: true,
            children: vec![
                TreeNode {
                    path: "zeta.txt".to_string(),
                    name: "zeta.txt".to_string(),
                    is_directory: false,
                    children: Vec::new(),
                },
                TreeNode {
                    path: "Alpha".to_string(),
                    name: "Alpha".to_string(),
                    is_directory: true,
                    children: Vec::new(),
                },
                TreeNode {
                    path: "beta".to_string(),
                    name: "beta".to_string(),
                    is_directory: true,
                    children: Vec::new(),
                },
            ],
        };
        let built = build_tree_items("demo", &root, &BTreeSet::new(), None);
        assert_eq!(
            row_ids(&built),
            vec![
                "dir:".to_string(),
                "dir:Alpha".to_string(),
                "empty:Alpha".to_string(),
                "dir:beta".to_string(),
                "empty:beta".to_string(),
                "file:zeta.txt".to_string(),
            ]
        );
    }

    #[test]
    fn filter_keeps_matching_paths_and_their_ancestors() {
        let snapshot = snapshot();
        let built = build_tree_items("demo", &snapshot.root, &BTreeSet::new(), Some("main"));

        assert_eq!(
            row_ids(&built),
            vec![
                "dir:".to_string(),
                "dir:src".to_string(),
                "file:src/main.rs".to_string(),
            ],
            "命中 `main` 时只留 src 与其祖先，且 logs 不该出现"
        );
        assert!(built.items[0].is_expanded(), "过滤时必须强制展开根行");
        let src = &built.items[0].children[0];
        assert!(src.is_expanded(), "过滤时必须强制展开命中的祖先目录");
    }

    #[test]
    fn filter_without_match_leaves_only_the_root_placeholder() {
        let snapshot = snapshot();
        let built = build_tree_items("demo", &snapshot.root, &BTreeSet::new(), Some("nope"));
        assert_eq!(
            row_ids(&built),
            vec!["dir:".to_string(), "empty:".to_string()]
        );
    }

    #[test]
    fn expanded_directories_survive_a_rebuild() {
        let snapshot = snapshot();
        let mut expanded = BTreeSet::new();
        expanded.insert(SharedString::from("dir:src"));
        let built = build_tree_items("demo", &snapshot.root, &expanded, None);

        let src = built.items[0]
            .children
            .iter()
            .find(|item| item.id.as_ref() == "dir:src")
            .expect("dir:src 必须存在");
        assert!(src.is_expanded(), "dir:src 在展开集合里，重建后仍要展开");
        let logs = built.items[0]
            .children
            .iter()
            .find(|item| item.id.as_ref() == "dir:logs")
            .expect("dir:logs 必须存在");
        assert!(!logs.is_expanded(), "没在展开集合里的目录不该展开");
    }

    #[test]
    fn row_budget_truncates_and_reports_it() {
        let children: Vec<TreeNode> = (0..RENDER_LIMIT + 10)
            .map(|index| TreeNode {
                path: format!("f{index:05}.txt"),
                name: format!("f{index:05}.txt"),
                is_directory: false,
                children: Vec::new(),
            })
            .collect();
        let root = TreeNode {
            path: String::new(),
            name: "demo".to_string(),
            is_directory: true,
            children,
        };
        let built = build_tree_items("demo", &root, &BTreeSet::new(), None);
        assert_eq!(built.rows, RENDER_LIMIT, "行数必须正好卡在上限");
        assert!(built.truncated, "截断必须被报出来");
        assert_eq!(row_ids(&built).len(), RENDER_LIMIT);
    }

    #[test]
    fn row_budget_does_not_report_truncation_when_everything_fits() {
        let snapshot = snapshot();
        let built = build_tree_items("demo", &snapshot.root, &BTreeSet::new(), None);
        assert!(!built.truncated);
        assert_eq!(built.rows, 7);
    }

    /// 第一个目录自己就超过行预算时，它的**同级**目录不能被挤掉：行预算按广度取用。
    /// 回归：旧实现深度优先，根下第一个大目录吃光预算后在 `break` 处丢掉同级目录，
    /// 用户看到的顶层列表只剩它一个。
    #[test]
    fn a_deep_first_child_does_not_starve_its_siblings() {
        let deep: Vec<TreeNode> = (0..RENDER_LIMIT + 10)
            .map(|index| TreeNode {
                path: format!("big/f{index:05}.txt"),
                name: format!("f{index:05}.txt"),
                is_directory: false,
                children: Vec::new(),
            })
            .collect();
        let root = TreeNode {
            path: String::new(),
            name: "demo".to_string(),
            is_directory: true,
            children: vec![
                TreeNode {
                    path: "big".to_string(),
                    name: "big".to_string(),
                    is_directory: true,
                    children: deep,
                },
                TreeNode {
                    path: "zeta".to_string(),
                    name: "zeta".to_string(),
                    is_directory: true,
                    children: Vec::new(),
                },
            ],
        };
        let built = build_tree_items("demo", &root, &BTreeSet::new(), None);

        let top: Vec<String> = built.items[0]
            .children
            .iter()
            .map(|item| item.id.to_string())
            .collect();
        assert!(
            top.contains(&"dir:big".to_string()) && top.contains(&"dir:zeta".to_string()),
            "同一层的目录必须一起拿到预算，实际 = {top:?}"
        );
        assert!(built.truncated, "超过上限仍要如实报截断");
    }

    #[test]
    fn row_kinds_round_trip_through_the_ids_we_build() {
        let snapshot = snapshot();
        let built = build_tree_items("demo", &snapshot.root, &BTreeSet::new(), None);
        let kinds: Vec<RowKind> = row_ids(&built)
            .iter()
            .map(|id| RowKind::parse(id).0)
            .collect();
        assert_eq!(
            kinds,
            vec![
                RowKind::Directory,
                RowKind::Directory,
                RowKind::EmptyPlaceholder,
                RowKind::Directory,
                RowKind::Directory,
                RowKind::EmptyPlaceholder,
                RowKind::File,
            ]
        );
    }

    #[test]
    fn status_letters_match_the_macos_display_status() {
        assert_eq!(display_status(" M", false), "M", "工作树改动");
        assert_eq!(display_status("M ", false), "M", "只暂存");
        assert_eq!(display_status("??", true), "A", "未跟踪显示 A");
        assert_eq!(display_status("UU", false), "!", "冲突显示 !");
        assert_eq!(display_status("AA", false), "!");
        assert_eq!(display_status("DD", false), "!");
        assert_eq!(display_status("R ", false), "R", "索引位重命名");
        assert_eq!(display_status(" A", false), "A", "工作树新增");
    }

    #[test]
    fn tones_match_the_macos_change_kind() {
        assert_eq!(git_tone("??", true), GitTone::Added);
        assert_eq!(git_tone("A ", false), GitTone::Added);
        assert_eq!(git_tone(" M", false), GitTone::Modified);
        assert_eq!(git_tone(" D", false), GitTone::Deleted);
        assert_eq!(git_tone("R ", false), GitTone::Moved);
        assert_eq!(git_tone("C ", false), GitTone::Copied);
        assert_eq!(git_tone("UU", false), GitTone::Conflicted);
    }

    #[test]
    fn directory_tone_uses_the_most_urgent_descendant() {
        let prefix = "";
        let changes = vec![
            StatusChange {
                path: "src/a.txt".to_string(),
                status: " M".to_string(),
                untracked: false,
            },
            StatusChange {
                path: "src/deep/b.txt".to_string(),
                status: " D".to_string(),
                untracked: false,
            },
            StatusChange {
                path: "src/deep/c.txt".to_string(),
                status: "??".to_string(),
                untracked: true,
            },
        ];
        let status = GitTreeStatus::from_changes(prefix, &changes);

        assert_eq!(status.directory("src"), Some(GitTone::Deleted));
        assert_eq!(status.directory("src/deep"), Some(GitTone::Deleted));
        assert_eq!(status.directory(""), Some(GitTone::Deleted));
        assert_eq!(status.directory("missing"), None);

        let mut with_conflict = changes.clone();
        with_conflict.push(StatusChange {
            path: "src/x.txt".to_string(),
            status: "UU".to_string(),
            untracked: false,
        });
        let status = GitTreeStatus::from_changes(prefix, &with_conflict);
        assert_eq!(status.directory("src"), Some(GitTone::Conflicted));
    }

    #[test]
    fn file_rows_get_letters_and_directory_rows_do_not() {
        let changes = vec![StatusChange {
            path: "src/a.txt".to_string(),
            status: " M".to_string(),
            untracked: false,
        }];
        let status = GitTreeStatus::from_changes("", &changes);

        let decoration = status.file("src/a.txt").expect("文件行必须有装饰");
        assert_eq!(decoration.letter.as_ref(), "M");
        assert_eq!(decoration.tone, GitTone::Modified);
        assert_eq!(status.file("src"), None, "目录不是文件行");
        assert_eq!(status.directory("src"), Some(GitTone::Modified));
        assert_eq!((status.counts()), (1, 2), "一个文件行 + src 与根两个目录行");
    }

    #[test]
    fn repository_prefix_is_joined_back_onto_workspace_relative_paths() {
        let data = json!({
            "repositoryRoot": "app",
            "changes": [
                { "path": "src/a.txt", "status": " M", "staged": false, "worktree": true, "untracked": false }
            ]
        });
        let status = GitTreeStatus::from_status(&data, Path::new("/work"));
        assert!(status.file("app/src/a.txt").is_some(), "前缀要拼回去");
        assert!(status.file("src/a.txt").is_none());
        assert_eq!(status.directory("app/src"), Some(GitTone::Modified));
        assert_eq!(status.directory("app"), Some(GitTone::Modified));
        assert_eq!(status.directory(""), Some(GitTone::Modified));
    }

    #[test]
    fn repository_root_equal_to_workspace_root_adds_no_prefix() {
        for repository_root in ["", "."] {
            let data = json!({
                "repositoryRoot": repository_root,
                "changes": [
                    { "path": "a.txt", "status": "??", "staged": false, "worktree": false, "untracked": true }
                ]
            });
            let status = GitTreeStatus::from_status(&data, Path::new("/work"));
            let decoration = status.file("a.txt").expect("必须是工作区相对路径");
            assert_eq!(decoration.letter.as_ref(), "A");
            assert_eq!(decoration.tone, GitTone::Added);
        }
    }

    #[test]
    fn no_repository_and_no_changes_produce_no_decorations() {
        let no_repo = json!({ "repositoryRoot": null, "changes": [] });
        assert!(
            GitTreeStatus::from_status(&no_repo, Path::new("/work")).is_empty(),
            "repositoryRoot 为 null 时不该有任何装饰"
        );

        let no_changes = json!({ "repositoryRoot": "", "changes": [] });
        assert!(
            GitTreeStatus::from_status(&no_changes, Path::new("/work")).is_empty(),
            "没有变更时不该有任何装饰"
        );
    }

    #[test]
    fn absolute_repository_root_maps_only_when_under_the_workspace() {
        let (workspace, inside_root, outside_root) = if cfg!(windows) {
            ("C:/work", "C:/work/app", "C:/elsewhere")
        } else {
            ("/work", "/work/app", "/elsewhere")
        };
        let workspace = Path::new(workspace);

        let inside = json!({
            "repositoryRoot": inside_root,
            "changes": [{ "path": "a.txt", "status": " M", "untracked": false }]
        });
        let status = GitTreeStatus::from_status(&inside, workspace);
        assert!(
            status.file("app/a.txt").is_some(),
            "工作区根之下的绝对仓库根要折算成相对前缀"
        );

        let outside = json!({
            "repositoryRoot": outside_root,
            "changes": [{ "path": "a.txt", "status": " M", "untracked": false }]
        });
        assert!(
            GitTreeStatus::from_status(&outside, workspace).is_empty(),
            "工作区之外的仓库根无法映射到树行，必须放弃装饰"
        );
    }

    #[test]
    fn duplicate_and_empty_paths_are_handled() {
        let changes = vec![
            StatusChange {
                path: "a.txt".to_string(),
                status: " M".to_string(),
                untracked: false,
            },
            StatusChange {
                path: "a.txt".to_string(),
                status: " D".to_string(),
                untracked: false,
            },
            StatusChange {
                path: String::new(),
                status: " M".to_string(),
                untracked: false,
            },
        ];
        let status = GitTreeStatus::from_changes("", &changes);
        assert_eq!(
            status.file("a.txt").map(|d| d.letter.to_string()),
            Some("M".to_string()),
            "第一条胜出"
        );
        assert_eq!(status.counts().0, 1, "空路径不该造出文件行");
    }

    #[test]
    fn parse_snapshot_rejects_incomplete_payloads() {
        assert_eq!(
            parse_snapshot(&json!({ "files": [] })).unwrap_err(),
            "响应缺少 data.root"
        );
        assert_eq!(
            parse_snapshot(&json!({ "root": { "path": "" } })).unwrap_err(),
            "响应缺少 data.files"
        );
    }
}
