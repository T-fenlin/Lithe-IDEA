//! The project tree's nodes, the flat arena they are read through, and the visible rows.

use std::collections::{HashMap, HashSet, VecDeque};

use gpui::SharedString;

/// One node the owner hands the tree: a folder with its children in display order, or a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectNode {
    pub(crate) path: SharedString,
    pub(crate) name: SharedString,
    pub(crate) is_dir: bool,
    pub(crate) children: Vec<ProjectNode>,
}

impl ProjectNode {
    pub fn folder(path: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
            is_dir: true,
            children: Vec::new(),
        }
    }

    pub fn file(path: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
            is_dir: false,
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: ProjectNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = ProjectNode>) -> Self {
        self.children.extend(children);
        self
    }

    /// The workspace-relative path the tree names this row by.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// What the row reads.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn nodes(&self) -> &[ProjectNode] {
        &self.children
    }
}

/// What a row shows, by the kind of node behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectRowKind {
    Directory,
    File,
    /// A folder that is open and empty (or holds nothing the filter keeps): a note, not a row to pick.
    EmptyFolder,
}

impl ProjectRowKind {
    /// The kind named by a row key, `dir:`/`file:`/`empty:` with the path after it.
    pub fn parse(key: &str) -> (Self, &str) {
        match key.split_once(':') {
            Some(("file", rest)) => (Self::File, rest),
            Some(("empty", rest)) => (Self::EmptyFolder, rest),
            Some(("dir", rest)) => (Self::Directory, rest),
            _ => (Self::Directory, key),
        }
    }
}

/// One node in the arena: its path, its name, and whether it holds children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub path: SharedString,
    pub name: SharedString,
    pub is_dir: bool,
}

/// The workspace flattened once, root row first: node 0 is the workspace itself (path `""`),
/// every node keeps its key, its parent and its depth, so rows can be materialized without
/// borrowing the owner's nodes again.
#[derive(Default)]
pub(crate) struct Arena {
    entries: Vec<Entry>,
    keys: Vec<SharedString>,
    children: Vec<Vec<usize>>,
    parents: Vec<Option<usize>>,
    depths: Vec<usize>,
    by_path: HashMap<SharedString, usize>,
}

impl Arena {
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The root row, then every node under it in display order: folders first, then by lowercase name.
    pub(crate) fn build(
        root_label: SharedString,
        forest: impl IntoIterator<Item = ProjectNode>,
    ) -> Self {
        let mut arena = Self::default();
        arena.push(
            Entry {
                path: SharedString::default(),
                name: root_label,
                is_dir: true,
            },
            None,
            0,
        );
        for node in sorted(forest) {
            arena.add(node, 0, 1);
        }
        arena
    }

    fn add(&mut self, node: ProjectNode, parent: usize, depth: usize) {
        let ix = self.push(
            Entry {
                path: node.path,
                name: node.name,
                is_dir: node.is_dir,
            },
            Some(parent),
            depth,
        );
        self.children[parent].push(ix);
        for child in sorted(node.children) {
            self.add(child, ix, depth + 1);
        }
    }

    fn push(&mut self, entry: Entry, parent: Option<usize>, depth: usize) -> usize {
        let ix = self.entries.len();
        let key: SharedString = if entry.is_dir {
            format!("dir:{}", entry.path).into()
        } else {
            format!("file:{}", entry.path).into()
        };
        self.by_path.insert(entry.path.clone(), ix);
        self.entries.push(entry);
        self.keys.push(key);
        self.children.push(Vec::new());
        self.parents.push(parent);
        self.depths.push(depth);
        ix
    }

    pub(crate) fn entry(&self, ix: usize) -> &Entry {
        &self.entries[ix]
    }

    pub(crate) fn key(&self, ix: usize) -> SharedString {
        self.keys[ix].clone()
    }

    pub(crate) fn children(&self, ix: usize) -> &[usize] {
        &self.children[ix]
    }

    pub(crate) fn parent(&self, ix: usize) -> Option<usize> {
        self.parents[ix]
    }

    pub(crate) fn depth(&self, ix: usize) -> usize {
        self.depths[ix]
    }

    pub(crate) fn is_root(&self, ix: usize) -> bool {
        ix == 0
    }

    pub(crate) fn find(&self, path: &str) -> Option<usize> {
        self.by_path.get(path).copied()
    }

    /// Whether this node or anything under it has `needle` in its path.
    pub(crate) fn subtree_matches(&self, ix: usize, needle: &str) -> bool {
        self.entries[ix].path.to_lowercase().contains(needle)
            || self.children[ix]
                .iter()
                .any(|child| self.subtree_matches(*child, needle))
    }
}

/// Folders first, then files, each by lowercase name.
fn sorted(nodes: impl IntoIterator<Item = ProjectNode>) -> Vec<ProjectNode> {
    let mut nodes: Vec<ProjectNode> = nodes.into_iter().collect();
    nodes.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    nodes
}

/// One visible row: the node it shows, how deep it sits, its parent's row, and whether it is
/// the note an open-but-empty folder shows instead of children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Row {
    pub node: usize,
    pub depth: usize,
    pub parent: Option<usize>,
    pub placeholder: bool,
}

impl Row {
    /// The key the row is named by; the placeholder takes the `empty:` form.
    pub(crate) fn key(&self, arena: &Arena) -> SharedString {
        match self.placeholder {
            true => format!("empty:{}", arena.entry(self.node).path).into(),
            false => arena.key(self.node),
        }
    }

    pub(crate) fn kind(&self, arena: &Arena) -> ProjectRowKind {
        match self.placeholder {
            true => ProjectRowKind::EmptyFolder,
            false if arena.entry(self.node).is_dir => ProjectRowKind::Directory,
            false => ProjectRowKind::File,
        }
    }
}

/// The rows in view, at most `budget` of them: a folder that is shut costs one row, and a folder
/// that is open and holds nothing shows one note row instead.
///
/// Two walks, because the budget and the display order are different questions: the **budget** is
/// spent breadth first, so a huge first folder cannot eat the rows its shallow neighbours need
/// (that was the B20 regression); the **rows** are then emitted depth first, so a folder's children
/// sit right under it. `true` comes back when the budget cut the tree short.
pub(crate) fn materialize(
    arena: &Arena,
    expanded: &HashSet<SharedString>,
    query: &str,
    budget: usize,
) -> (Vec<Row>, bool) {
    if arena.is_empty() || budget == 0 {
        return (Vec::new(), !arena.is_empty());
    }
    let needle = query.to_lowercase();
    let forced = !needle.is_empty();
    let open =
        |node: usize| arena.entry(node).is_dir && (forced || expanded.contains(&arena.key(node)));
    let visible = |node: usize| -> Vec<usize> {
        arena
            .children(node)
            .iter()
            .copied()
            .filter(|child| needle.is_empty() || arena.subtree_matches(*child, &needle))
            .collect()
    };

    // What the budget buys: the shallow rows first.
    let mut bought: HashSet<usize> = HashSet::new();
    let mut notes: HashSet<usize> = HashSet::new();
    let mut spent = 0usize;
    let mut queue: VecDeque<usize> = VecDeque::new();
    bought.insert(0);
    spent += 1;
    if open(0) {
        queue.push_back(0);
    }
    let mut truncated = false;
    'buying: while let Some(node) = queue.pop_front() {
        let children = visible(node);
        if children.is_empty() {
            if spent >= budget {
                truncated = true;
                break;
            }
            notes.insert(node);
            spent += 1;
            continue;
        }
        for child in children {
            if spent >= budget {
                truncated = true;
                break 'buying;
            }
            bought.insert(child);
            spent += 1;
            if open(child) {
                queue.push_back(child);
            }
        }
    }

    // The order they show in: each folder's children right under it.
    let mut rows = Vec::new();
    let mut stack: Vec<(usize, usize, Option<usize>)> = vec![(0, 0, None)];
    while let Some((node, depth, parent)) = stack.pop() {
        let row = rows.len();
        rows.push(Row {
            node,
            depth,
            parent,
            placeholder: false,
        });
        if notes.contains(&node) {
            rows.push(Row {
                node,
                depth: depth + 1,
                parent: Some(row),
                placeholder: true,
            });
            continue;
        }
        push_children(
            &mut stack,
            &bought,
            &open,
            &visible,
            node,
            depth + 1,
            Some(row),
        );
    }
    (rows, truncated)
}

/// Pushes the children a folder will show, in reverse so the first one pops first; only rows the
/// budget bought go on the stack.
fn push_children(
    stack: &mut Vec<(usize, usize, Option<usize>)>,
    bought: &HashSet<usize>,
    open: &impl Fn(usize) -> bool,
    visible: &impl Fn(usize) -> Vec<usize>,
    node: usize,
    depth: usize,
    parent: Option<usize>,
) {
    if !open(node) {
        return;
    }
    for child in visible(node).into_iter().rev() {
        if bought.contains(&child) {
            stack.push((child, depth, parent));
        }
    }
}
