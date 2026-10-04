//! 项目树的数据层：`workspace.snapshot` 的 `data.root` 树 + `git.status` 状态装饰。
//!
//! 只依赖 `SharedString` / `TreeItem` / `IconName` 这些数据结构，**不依赖任何 gpui UI
//! 组件**，所以建树与抓取逻辑可以脱离渲染单独读。`Explorer` 实体、`LoadState` 与所有渲染
//! 辅助在 `explorer_view.rs`；crate 的模块文档（规格出处、图标对照、未实现清单）在 `lib.rs`。
//!
//! 唯一的例外是 [`icon_for_file`] 与 [`GitTone::color`]：它们要按**当前主题**取字形 / 颜色
//! （`&App`），所以签名里有一个只读的 `&App`。它们仍然不做任何渲染，返回的
//! [`lithe_gpui_shared::icons::FileIcon`] 与 [`Hsla`] 由 `explorer_view.rs` 画出来。
//!
//! 信封拼装已收敛到 `lithe_gpui_shared::core_json`（见 [`load_snapshot`] / [`load_git_status`]）。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::tree::TreeItem;
use gpui_kit::{App, Hsla, SharedString};
use lithe_gpui_shared::icons::{FileIcon, file_icon};
use lithe_gpui_shared::{core_json, tr};
// [`core_json`] 的 payload 需要一个 `serde_json::Value`：直接依赖 `serde_json`
// （与 `lithe-gpui/crates/git` 同一口径），不借 gpui 的内部重导出。
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// 常量：上限（界面文案与度量在 explorer_view.rs）
// ---------------------------------------------------------------------------

/// 项目树最多渲染**多少行**（目录行 + 文件行 + 空目录占位行）。
///
/// 本仓库快照约 4.8 千个节点，这个上限等于"全部显示"；设它是为了防止把巨型工作区
/// （几十万节点）一次性铺进 `TreeState` —— `TreeState::set_items` 会重建整棵扁平表
/// （`gpui-base-0.7.0/src/tree.rs:221-226`）。**超出时不静默**：加载完成后打一行
/// `S1_EXPLORER ... rows=… truncated=…` 诊断（与 `workspace.rs` 的 `S1_VIEWPORT` 是同一套
/// 探针口径）。
pub(crate) const RENDER_LIMIT: usize = 5_000;

/// 工作区根那一行的 id：空前缀 `dir:`（根工作区相对路径 = 空串）。
///
/// 真源：macOS 的项目树根行是 `depth == 0` 的那一条，标签 600 粗体
/// （`macos/Sources/Lithe/Views/Workspace/ProjectSidebarView.swift:483`），也是
/// `Copy Project Path` / `Show Project in Finder` 那一支的判据（同文件 `:603-617`）。
pub(crate) const ROOT_ID: &str = "dir:";

// ---------------------------------------------------------------------------
// 数据：Core 快照 → 多层树
// ---------------------------------------------------------------------------

/// 行的种类。由 `TreeItem` 的 id 前缀解出来（`file:` / `dir:` / `empty:`）。
///
/// 用枚举而不是 `&str` 是因为渲染闭包是 `move` 的：如果 `kind`/`relative` 是借用了
/// 本地 `id` 的切片，闭包就会同时持有 `id` 和指向它的引用（自引用，编译不过）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RowKind {
    File,
    Directory,
    /// 空目录的禁用占位行，见 [`empty_placeholder`]。
    EmptyPlaceholder,
}

impl RowKind {
    /// 从 `TreeItem` 的 id 解出（种类, 工作区相对路径）。
    pub(crate) fn parse(id: &str) -> (Self, String) {
        match id.split_once(':') {
            Some(("file", rest)) => (Self::File, rest.to_string()),
            Some(("empty", rest)) => (Self::EmptyPlaceholder, rest.to_string()),
            Some(("dir", rest)) => (Self::Directory, rest.to_string()),
            _ => (Self::Directory, id.to_string()),
        }
    }
}

/// `workspace.snapshot` 的 `data.root` 解析出来的一棵节点（目录或文件）。
///
/// 这是 `data.root` 的形状（`rust/lithe-core/src/protocol/contracts.rs:58-67` 的
/// `WorkspaceNode`：`{ path, name, isDirectory, children? }`），**目录的 `children` 恒存在**
/// （`rust/lithe-core/src/project/files.rs:633-638` 的 `scan_node` 对目录总是
/// `children: Some(visible_children)`，空目录就是 `Some([])`）—— 那条 `Some([])` 正是
/// "空目录"在契约里的表达。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TreeNode {
    /// 工作区相对路径，`/` 分隔；工作区根是**空串**（`project/files.rs:571-580` 的 `relative_path`）。
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) is_directory: bool,
    /// 目录的子项（文件恒为空）。顺序照 Core（目录在前 + 小写名升序）。
    pub(crate) children: Vec<TreeNode>,
}

impl TreeNode {
    /// 这棵子树有多少个节点（含自身）。
    pub(crate) fn count(&self) -> usize {
        1 + self.children.iter().map(Self::count).sum::<usize>()
    }

    /// 收集这棵子树里所有**非目录**节点的相对路径。
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

/// 一次 `workspace.snapshot` 的解析结果。
///
/// 两条数据都留着：`root` 是**建树唯一用的那一份**（它才带得动空目录），`files` 只用于
/// "两份数据不一致"的诊断（见 [`Snapshot::files_missing_from_root`]）—— 真机契约里
/// `files` 是 `root` 的扁平投影（`protocol/contracts.rs:69-75`），不一致就说明 Core 侧
/// 有 bug，值得留一行证据。
#[derive(Debug)]
pub(crate) struct Snapshot {
    pub(crate) root: TreeNode,
    pub(crate) files: Vec<String>,
}

impl Snapshot {
    /// `data.files` 里有多少条路径**不在** `data.root` 的树里。
    ///
    /// 建树以 `root` 为准（U24 的取舍），所以这个数只用于诊断：非 0 时
    /// `explorer_view.rs` 会打一行 `S1_EXPLORER ... files_not_in_root=…`。
    pub(crate) fn files_missing_from_root(&self) -> usize {
        let mut in_tree = BTreeSet::new();
        self.root.collect_file_paths(&mut in_tree);
        self.files
            .iter()
            .filter(|path| !in_tree.contains(path.as_str()))
            .count()
    }
}

/// 空目录的占位行（禁用 + 文案「文件夹为空」）。
///
/// **为什么需要它**：`TreeItem::is_folder()` 的定义是 `!children.is_empty()`
/// （`gpui-base-0.7.0/src/tree.rs:132-134`），不是文件系统概念 —— 一个没有可见子项的目录会被
/// 整棵树当作**文件行**处理：单击不会展开，而且我们的点击回调会拿它的路径去调 `on_open`
/// （等于把目录当文件打开）。挂一条禁用的子项同时解决两件事：这一行重新成为可展开的目录行，
/// 展开后显示真机已有的文案「文件夹为空」（`locale.ts:7461`，走
/// `lithe.fileExplorer.folderIsEmpty`）。
///
/// **U24 起这条分支真的可达了**：改用 `data.root` 之后，磁盘上的空目录会作为
/// `children: Some([])` 的目录节点出现（`project/files.rs:633-638`），于是 `dir:<路径>` 这一行
/// 第一次有了"零个可见子项"的真实输入。旧的扁平 `data.files` 聚树路径永远造不出这种行。
pub(crate) fn empty_placeholder(path: &str) -> TreeItem {
    TreeItem::new(
        SharedString::from(format!("empty:{path}")),
        tr("lithe.fileExplorer.folderIsEmpty"),
    )
    .disabled(true)
}

/// 一次建树的结果：行、行数，以及"是否因为 [`RENDER_LIMIT`] 被截断"。
pub(crate) struct BuiltTree {
    pub(crate) items: Vec<TreeItem>,
    /// 实际铺进树里的行数（含根行与占位行）。
    pub(crate) rows: usize,
    /// 是否还有没铺进来的节点（**不静默**：调用点会打诊断）。
    pub(crate) truncated: bool,
}

/// 行预算：把"最多 [`RENDER_LIMIT`] 行"写成一处，避免每个递归分支各判一次。
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

    /// 占一行。预算用尽返回 `false`（并把 `truncated` 立起来）。
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

/// 把 Core 的 `data.root` 树铺成 gpui 的 `TreeItem` 列表（**根行 + 任意深度**）。
///
/// 任意深度照真源：macOS 的 `FileNodeRow` 递归渲染 `node.children`
/// （`macos/Sources/Lithe/Views/Workspace/ProjectSidebarView.swift:440-455`），目录能一直点开到
/// 最里层。
///
/// `expanded` 是**用户当前的展开集合**（id 形如 `dir:<相对路径>`），由 `Explorer` 从
/// `TreeEvent` 里攒出来：搜索、刷新、Reveal 都会重建整棵树，不带上它就会把用户展开的目录全收起来。
/// `filter` 有值时强制展开所有层级 —— 过滤后只剩命中路径及其祖先目录，不展开就什么也看不到。
pub(crate) fn build_tree_items(
    root_label: &str,
    root: &TreeNode,
    expanded: &BTreeSet<SharedString>,
    filter: Option<&str>,
) -> BuiltTree {
    let query = filter.map(str::to_lowercase);
    let force_expanded = query.is_some();
    let mut budget = RowBudget::new();
    let root_id = SharedString::new_static(ROOT_ID);

    let mut items = Vec::new();
    if budget.take() {
        let mut root_children = to_items(
            &root.children,
            expanded,
            force_expanded,
            query.as_deref(),
            &mut budget,
        );
        if root_children.is_empty() && budget.take() {
            // 根行没有可见子项时（空工作区，或过滤后没有命中）挂一条禁用占位行，理由见
            // [`empty_placeholder`]。
            root_children = vec![empty_placeholder("")];
        }
        items.push(
            TreeItem::new(root_id.clone(), SharedString::from(root_label.to_string()))
                .expanded(force_expanded || expanded.contains(&root_id))
                .children(root_children),
        );
    }

    BuiltTree {
        items,
        rows: budget.rows(),
        truncated: budget.truncated,
    }
}

/// 节点自身或**任一后代**命中查询词。
///
/// 祖先目录必须留在树里，否则命中的文件无处可挂（真机过滤后同样保留路径上的每一级）。
fn subtree_matches(node: &TreeNode, query: &str) -> bool {
    node.path.to_lowercase().contains(query)
        || node
            .children
            .iter()
            .any(|child| subtree_matches(child, query))
}

/// 显示顺序：目录在前、两边都按**小写**比较。
///
/// 排序照 Core 自己的 `folders-first` 规则（`rust/lithe-core/src/project/files.rs:608-624`；
/// 界面口径同 macOS 的 `ForEach(node.children ?? [])`）—— 这里再排一次是为了不依赖"Core 恰好
/// 排好了"这个隐含前提。[`to_items`] 与 [`displayed_siblings`] **共用**它，所以"树里铺出来的
/// 顺序"与"⌘A 选中的兄弟顺序"不会打架。
fn sort_for_display(nodes: &mut [&TreeNode]) {
    nodes.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
}

/// 真源 `selectAll`（`ProjectTreeSelection.swift:59-70`）里"焦点行的**同级兄弟**"那一半。
///
/// 用**显示树**找父级（真源 `displayedParent`，`:62-69`）而不是文件系统父级 —— 压缩的 Java 包
/// 在显示上挂在并非文件系统父级的祖先下。没有焦点、焦点就是根、或焦点在树里找不到时，取根的子项
/// （真源 `?? root` 同）。
///
/// ⚠️ 返回的是**全部**直接子项（不按展开态过滤）：焦点行既然可见，它的父级必是展开的，所以它的
/// 兄弟此刻也都可见 —— 真源同样直接取 `parent.children`。
///
/// 顺序与树里铺出来的一致（走 [`sort_for_display`]），这样"选中集的顺序"与用户看到的对得上。
///
/// 接线（焦点 + action 管道）还没做，所以先 `allow(dead_code)` —— 与 `workspace.rs` 里
/// `clear_breakpoints`「实现了、入口未接」同一处置。
#[allow(dead_code)]
pub(crate) fn displayed_siblings(root: &TreeNode, focused: Option<&str>) -> Vec<String> {
    let parent = focused
        .and_then(|path| displayed_parent_of(root, path))
        .unwrap_or(root);
    let mut children: Vec<&TreeNode> = parent.children.iter().collect();
    sort_for_display(&mut children);
    children.into_iter().map(|node| node.path.clone()).collect()
}

/// 真源 `displayedParent`（`:62-69`）：按**树结构**递归找 `path` 的父节点。
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

    /// 固定形状：`src/`（两个子项）+ `a.rs` + `B.rs`，构造顺序故意不是显示顺序。
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

    /// 没有焦点、或焦点在树里找不到 ⇒ 取**根的子项**（真源 `?? root`）；
    /// 顺序是"目录在前 + 小写名升序"。
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

    /// 焦点在子项上 ⇒ 取**它的同级兄弟**（不是它的子项、也不是整棵可见树）—— 真源 ⌘A 的判据。
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

    /// 根行自己的焦点（`path` 是**空串**）⇒ 顶层的项（根没有兄弟）。
    #[test]
    fn focus_on_the_root_takes_the_top_level_items() {
        let root = tree();
        assert_eq!(displayed_siblings(&root, Some("")), ["src", "a.rs", "B.rs"]);
    }
}

/// 把一层节点转成 `TreeItem` 列表。
fn to_items(
    nodes: &[TreeNode],
    expanded: &BTreeSet<SharedString>,
    force_expanded: bool,
    query: Option<&str>,
    budget: &mut RowBudget,
) -> Vec<TreeItem> {
    let mut visible: Vec<&TreeNode> = nodes
        .iter()
        .filter(|node| query.is_none_or(|needle| subtree_matches(node, needle)))
        .collect();
    sort_for_display(&mut visible);

    let mut items = Vec::new();
    for node in visible {
        if !budget.take() {
            break;
        }
        let id = SharedString::from(if node.is_directory {
            format!("dir:{}", node.path)
        } else {
            format!("file:{}", node.path)
        });
        let mut item = TreeItem::new(id.clone(), SharedString::from(node.name.clone()));
        if node.is_directory {
            let mut children = to_items(&node.children, expanded, force_expanded, query, budget);
            if children.is_empty() && budget.take() {
                children = vec![empty_placeholder(&node.path)];
            }
            item = item.children(children);
        }
        items.push(item.expanded(force_expanded || expanded.contains(&id)));
    }
    items
}

/// 失败的统一取词：有 Core 错误码就用错误码原文，信封级失败（响应不是合法 JSON）没有错误码，
/// 退回 [`core_json`] 自己的说明性文案 —— 与收敛前本地文案的取舍逐字一致。
fn core_error_text(error: lithe_gpui_shared::CoreError) -> String {
    match error.code() {
        Some(code) => code.to_string(),
        None => error.to_string(),
    }
}

/// 用真实 Core 调用取工作区快照。
///
/// 契约真源（不要照抄调研文档里的揣测）：
/// - **命令名**：`"workspace.snapshot"`（`rust/lithe-core/src/protocol/command.rs:320`）；
/// - **请求 payload**：`WorkspaceSnapshotRequest`（`rust/lithe-core/src/project/files.rs:36-45`，
///   `#[serde(rename_all = "camelCase")]`）→
///   `{ root: String, hiddenDirectoryNames: String[] = [], hiddenFilePatterns: String[] = [] }`
///   （后两个带 `#[serde(default)]`，可以不传）；信封（`id` / `operationId` /
///   `timeoutMilliseconds`）由 [`core_json`] 拼；
/// - **响应 `data`**：`WorkspaceSnapshotResponse`（`rust/lithe-core/src/protocol/contracts.rs:69-75`）
///   → `{ root: WorkspaceNode, files: String[] }`；`WorkspaceNode`（同文件 `:58-67`）是
///   `{ path, name, isDirectory, children?: WorkspaceNode[] }`，**`path` 也是工作区相对路径**
///   （`project/files.rs:576,583-588`）。
///
/// **U24：建树改用 `data.root`，不再用 `data.files`。** 原因见 [`TreeNode`]：`files` 里只有
/// 非目录项（`project/files.rs:582-583` 只对非目录 `files.push`），所以"磁盘上的空目录"在扁平
/// 路径里没有任何痕迹，永远聚不出那一行。`root` 是同一份扫描的另一半投影，带 `isDirectory`
/// 与 `children`，因此**空目录与文件共用同一条真相源**，不需要在 gpui 侧再扫一次盘
/// （那是第二份真相源，更是第二份忽略规则）。
///
/// `root` 的**绝对路径不来自 Core**（`existing_root` 内部会 `canonicalize`，
/// `project/files.rs:641-652`，但返回值里没有回传），所以拼绝对路径用的是构造时拿到的
/// `Explorer::root`。
pub(crate) fn load_snapshot(root: &Path) -> Result<Snapshot, String> {
    let root_text = root
        .to_str()
        .ok_or_else(|| "工作区根不是合法 UTF-8 路径".to_string())?;

    let data =
        core_json("workspace.snapshot", json!({ "root": root_text })).map_err(core_error_text)?;
    parse_snapshot(data.as_ref().ok_or("响应缺少 data")?)
}

/// 把 `workspace.snapshot` 的 `data` 解析成 [`Snapshot`]（纯函数，可脱离 Core 单测）。
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

/// 递归解析一个 `WorkspaceNode`。
///
/// 字段缺失一律按"最保守"的解释走（不是目录、没有子项），不 panic：`data.root` 是 Core 的
/// 序列化产物，形状不对时宁可少画几行也不要把界面打崩。
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

// ---------------------------------------------------------------------------
// Git 状态装饰（`git.status`）
// ---------------------------------------------------------------------------

/// 树行上的 git 状态档。
///
/// 逐字对应 macOS 的 `GitChangeKind`（`macos/Sources/LitheGitModule/Models/GitModels.swift:877-906`）
/// 的六个 case —— **注意"未跟踪"不是一档**：真源把它归进 `.added`
/// （同文件 `:857-861` 的 `if isUntracked || indexStatus == "A" ... { return .added }`），
/// 所以未跟踪文件是绿色的 `A`。
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
    /// 目录装饰的取色优先级（真源 `GitTreeStatusProjection.priority`，
    /// `GitModels.swift:961-970`）：**最紧急的后代档胜出**，冲突与删除永远不会被"修改"盖住。
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

    /// 行内文字（目录/文件名，以及右侧的状态字母）的颜色。
    ///
    /// 真源是 macOS 的 `gitStatusColor`（`ProjectSidebarView.swift:747-758`）：
    /// modified→`accent`、added/copied→`success`、deleted→`error`、moved→`skill`、
    /// conflicted→`warning`。`skill` 在 gpui 主题里没有同名字 tok，按语义取最近的一档紫色
    /// `magenta_light`（与 `lithe-gpui/crates/git/src/changes.rs:125-134` 对 renamed 的选择同源）。
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

/// 一行文件的状态装饰：右侧的字母 + 名字与字母的颜色档。
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct GitDecoration {
    /// 真源 `GitChange.displayStatus`（`GitModels.swift:869-874`）：冲突 `!`、未跟踪 `A`、
    /// 否则工作树字符优先、再退回索引字符。
    pub(crate) letter: SharedString,
    pub(crate) tone: GitTone,
}

/// `git.status` 的一条变更（Core `GitChange`，`protocol/contracts.rs:523-539`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StatusChange {
    /// **仓库相对**路径，`/` 分隔（Core 约定）。
    pub(crate) path: String,
    /// Core 的原始两字符 porcelain 码（`" M"` / `"M "` / `"??"` / `"R "`…）。
    pub(crate) status: String,
    pub(crate) untracked: bool,
}

/// `git.status` 投影到树行上的状态（真源 `GitTreeStatusProjection`，
/// `GitModels.swift:911-977`）。
///
/// 键是**工作区相对**路径（`/` 分隔），因为树行的 id 用的就是它：Core 的 `changes[].path` 是
/// **仓库相对**的，而仓库根可以是工作区根下面的一级（`git/mod.rs:6630` 的
/// `relative_or_absolute(&repository_root, &root)`），所以构造时要把仓库前缀拼回去
/// —— macOS 是拿绝对路径当键来回避同一个问题（`GitFeatureModel.swift:107` 的
/// `absolutePaths: true`），本侧用相对路径拼前缀，等价且不用做平台路径比较。
#[derive(Clone, Default)]
pub(crate) struct GitTreeStatus {
    /// 文件行：工作区相对路径 → 字母 + 色调。
    files: BTreeMap<String, GitDecoration>,
    /// 目录行（含工作区根 `""`）：工作区相对路径 → 最紧急的后代档。
    directories: BTreeMap<String, GitTone>,
}

impl GitTreeStatus {
    /// 从 `git.status` 的 `data` 构造（纯函数，可脱离 Core 单测）。
    ///
    /// `repositoryRoot` 为 `null` 表示**工作区不是 git 仓库**（`git/mod.rs:6575-6581` 返回
    /// `ok: true` + `repositoryRoot: null`），此时没有任何装饰，不是错误。
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

    /// 从"仓库前缀 + 仓库相对变更列表"构造。
    pub(crate) fn from_changes(prefix: &str, changes: &[StatusChange]) -> Self {
        let mut files: BTreeMap<String, GitDecoration> = BTreeMap::new();
        let mut directories: BTreeMap<String, GitTone> = BTreeMap::new();
        for change in changes {
            let path = join_prefix(prefix, &change.path);
            if path.is_empty() {
                continue;
            }
            let tone = git_tone(&change.status, change.untracked);
            // 同一路径只留第一条（真源 `GitModels.swift:921-923` 的
            // `if changesByPath[path] == nil`）；Core 侧已经把索引/工作树两条合并过了
            // （`lithe-gpui/crates/git/src/changes.rs:387-392`）。
            files.entry(path.clone()).or_insert_with(|| GitDecoration {
                letter: SharedString::from(display_status(&change.status, change.untracked)),
                tone,
            });
            bump_ancestors(&mut directories, &path, tone);
            // 工作区根那一行也要染色（真源 `GitModels.swift:935-943` 给 `""` 单独记一次）。
            bump_directory(&mut directories, "", tone);
        }
        Self { files, directories }
    }

    /// 文件行的装饰（真源 `kind(relativePath:isDirectory:false)` + `change(for:)`）。
    pub(crate) fn file(&self, path: &str) -> Option<&GitDecoration> {
        self.files.get(path)
    }

    /// 目录行的色调（真源 `kind(relativePath:isDirectory:true)`）。
    pub(crate) fn directory(&self, path: &str) -> Option<GitTone> {
        self.directories.get(path).copied()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.files.is_empty() && self.directories.is_empty()
    }

    /// 诊断用：文件行 / 目录行的条目数。
    pub(crate) fn counts(&self) -> (usize, usize) {
        (self.files.len(), self.directories.len())
    }
}

/// 仓库根相对工作区根的前缀。
///
/// - `None` → 不是 git 仓库（无装饰）；
/// - 相对路径（Core 在仓库根位于工作区根之下时给相对形式，`git/mod.rs:6630`）→ 直接用；
/// - 绝对路径 → 相对工作区根求前缀；不在工作区根之下时返回 `None`（无法映射到树行，
///   宁可不装饰也不猜一个错误的位置）。
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

/// `/` 分隔、无首尾 `/`（`.` 与空串都归一成工作区根的空串）。
fn normalize_path(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let normalized = normalized.trim_matches('/');
    if normalized == "." {
        return String::new();
    }
    normalized.to_string()
}

/// 把仓库前缀与仓库相对路径拼成工作区相对路径。
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

/// 把 `tone` 记到 `path` 的**每一个祖先目录**上（不含 `path` 自身）。
fn bump_ancestors(map: &mut BTreeMap<String, GitTone>, path: &str, tone: GitTone) {
    let mut remainder = path;
    while let Some(slash) = remainder.rfind('/') {
        remainder = &remainder[..slash];
        bump_directory(map, remainder, tone);
    }
}

/// 按优先级写入一个目录档（只在更紧急时覆盖）。
fn bump_directory(map: &mut BTreeMap<String, GitTone>, directory: &str, tone: GitTone) {
    match map.get(directory) {
        Some(current) if current.priority() >= tone.priority() => {}
        _ => {
            map.insert(directory.to_string(), tone);
        }
    }
}

/// 解析一条 `GitChange`（形状见 `protocol/contracts.rs:523-539`）。
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

/// macOS 的 `GitChange.isConflicted`（`GitModels.swift:847-851`）：任一侧 `U`，或 `AA` / `DD`。
fn is_conflicted(index: char, worktree: char) -> bool {
    index == 'U'
        || worktree == 'U'
        || (index == 'A' && worktree == 'A')
        || (index == 'D' && worktree == 'D')
}

/// macOS 的 `GitChange.displayStatus`（`GitModels.swift:869-874`）。
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

/// macOS 的 `GitChange.kind`（`GitModels.swift:853-862`）。
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

/// 用真实 Core 调用取 `git.status` 并投影成树行装饰。
///
/// 契约真源：命令 `"git.status"`（`rust/lithe-core/src/git/mod.rs:6565`）、请求
/// `GitStatusRequest { root, includeIndexOnlyChanges?, repositoryRoots? }`
/// （同文件 `:102`，`camelCase`；本侧只传 `root`，与 `lithe-gpui/crates/git/src/changes.rs:328`
/// 同口径）、响应 `GitStatusResponse { repositoryRoot?, branch?, ahead, behind, changes[] }`
/// （`protocol/contracts.rs:540-560`）。
///
/// 失败返回 Core 的错误码原文（与 [`load_snapshot`] 同一取舍）。
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

// ---------------------------------------------------------------------------
// 图标
// ---------------------------------------------------------------------------

/// 按文件名挑图标：**先查真机默认图标主题（`idea-icons`）的文件类型图标，查不到再回落 Lucide**。
///
/// ## 两个真源与优先关系
///
/// 1. **真源**：`lithe-gpui/assets/icons/idea/**` —— 真机默认图标主题
///    （`windows/tauri/src/features/settings/config/default-settings.ts:100` 的
///    `iconTheme: "idea-icons"`）。查表在 `lithe_gpui_shared::icons::file_icon`：
///    文件名 → `filenames`、后缀 → `fileExtensions`、都没有 → `defaultFile`；
///    明暗由 `Theme::global(cx).is_dark()` 决定（真机同口径，见
///    `extension-contribution-runtime.ts:81-91`）。这些图标是**彩色**的
///    （`.java` 是橙色的杯子、`.gitignore` 是红色的 git 字形、`pom.xml` 是蓝色的 Maven `m`），
///    渲染走 `img()`（见 [`lithe_gpui_shared::icons::FileIcon::render`]）。
/// 2. **回落**：`gpui_kit::assets::IconName`（全量 Lucide 目录，1830 个字形），
///    表在 [`lithe_gpui_shared::icons::file_icon::lucide_fallback`]。主题包没有对应条目时用它。
///
/// 真机对照：macOS 的 `LitheIcon(kind: resolvedFileIconKind ?? node.iconKind, ...)`
/// （`macos/Sources/Lithe/Views/Workspace/ProjectSidebarView.swift:520`）。
pub(crate) fn icon_for_file(name: &str, cx: &App) -> FileIcon {
    FileIcon::lucide(file_icon::lucide_fallback(name)).with_theme_icon(name, cx)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一个两级工作区：`src/main.rs` + **空目录** `src/empty-folder` + 根下的空目录 `logs`。
    ///
    /// 形状逐字照 Core `scan_node` 的产物（`project/files.rs:633-638`）：目录恒有 `children`
    /// （空目录是 `[]`），文件没有 `children`。
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

    /// 把树铺平成一串 `id`（顺序 = 可见行顺序），断言建树结果时读起来更直接。
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

    /// 行 id 是否被禁用（空目录占位行是禁用的）。
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

    /// **U24 的主判据**：`data.root` 里那些 `children: []` 的空目录必须有行。
    ///
    /// 反证在 `flat_files_cannot_express_empty_directories`：同一份数据只取 `files` 时，
    /// 这两个目录在树里**一个都不存在** —— 这正是 U24 要修的那条缺口。
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

    /// 空目录行必须带一条**禁用**的占位行（文案走「文件夹为空」）。
    ///
    /// 没有它 `TreeItem::is_folder()` 会判成文件行（`gpui-base-0.7.0/src/tree.rs:132-134`），
    /// 于是点它会把目录当文件交给 `on_open`。
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

    /// **反证**：同一份快照只取 `data.files`（扁平**文件**路径）聚树时，空目录一个都不出现。
    ///
    /// 这条不是"测试旧实现"，而是把 U24 的取舍钉在数据上：`data.files` 里没有目录信息
    /// （`project/files.rs:582-583` 只对非目录 `files.push`），所以任何从它出发的建树都
    /// 造不出 `dir:logs`。若有人把 `load_snapshot` 改回读 `files`，本测试与
    /// `empty_directories_from_data_root_become_rows` 会一起失败。
    #[test]
    fn flat_files_cannot_express_empty_directories() {
        let snapshot = snapshot();
        assert_eq!(
            snapshot.files,
            vec!["src/main.rs".to_string()],
            "扁平列表里只有文件"
        );
        // 用 `files` 里的路径建一棵"只有文件"的树（等价于旧实现的输入），空目录无从出现。
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

    /// **取舍判据**：`data.files` 与 `data.root` 不一致时**以 root 为准**。
    ///
    /// 两个方向都判：root 里有而 files 里没有的（空目录 `logs`）必须成行；files 里有而
    /// root 里没有的（`ghost.txt`）**不许**成行。同时 `files_missing_from_root` 要如实报出后者
    /// —— 不静默，留给调用点打诊断。
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

    /// 父目录在前、目录优先、小写名升序 —— 与 Core 自己的 `folders-first` 同规则
    /// （`project/files.rs:608-624`）。
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

    /// 过滤：只剩命中路径与它的祖先目录，且过滤时强制展开（否则什么都看不到）。
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

    /// 过滤不命中 → 根行只挂一条占位行（真机的「没有匹配的文件」空态在视图层，
    /// 但树本身不能是空的，否则根行会被 `TreeItem::is_folder()` 当成文件行）。
    #[test]
    fn filter_without_match_leaves_only_the_root_placeholder() {
        let snapshot = snapshot();
        let built = build_tree_items("demo", &snapshot.root, &BTreeSet::new(), Some("nope"));
        assert_eq!(
            row_ids(&built),
            vec!["dir:".to_string(), "empty:".to_string()]
        );
    }

    /// 用户展开过的目录在重建后仍然展开（搜索 / 刷新 / Reveal 都走重建）。
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

    /// 行预算：超过 [`RENDER_LIMIT`] 时**停止铺行并立起 `truncated`**（诊断不静默），
    /// 且行数恰好等于上限。
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

    /// 行数够用时不许误报截断。
    #[test]
    fn row_budget_does_not_report_truncation_when_everything_fits() {
        let snapshot = snapshot();
        let built = build_tree_items("demo", &snapshot.root, &BTreeSet::new(), None);
        assert!(!built.truncated);
        // 根 + logs + 占位 + src + src/empty-folder + 占位 + main.rs = 7 行。
        assert_eq!(built.rows, 7);
    }

    /// `RowKind::parse` 与建树写下的 id 前缀必须对得上（渲染层靠它区分点击行为）。
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

    /// `displayStatus`：冲突 `!`、未跟踪 `A`、工作树优先、再退回索引位
    /// （真源 `GitModels.swift:869-874`）。
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

    /// 状态档：未跟踪归进 `Added`（真源 `GitModels.swift:857-861`），
    /// 顺序判据（冲突 → 新增 → 删除 → 重命名 → 复制 → 修改）逐条对上。
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

    /// 目录装饰按"最紧急的后代"取档（真源 `GitModels.swift:961-970`）：
    /// 冲突压过删除、删除压过新增、新增压过修改。
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

        // 冲突一进来就把上面的删除盖住。
        let mut with_conflict = changes.clone();
        with_conflict.push(StatusChange {
            path: "src/x.txt".to_string(),
            status: "UU".to_string(),
            untracked: false,
        });
        let status = GitTreeStatus::from_changes(prefix, &with_conflict);
        assert_eq!(status.directory("src"), Some(GitTone::Conflicted));
    }

    /// 文件行拿字母 + 档；目录行**不挂字母**（真源只在文件行画
    /// `Text(status.displayStatus)`，`ProjectSidebarView.swift:529-534`，目录行只用颜色
    /// `:484`）。
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

    /// 仓库根在工作区根**之下**时，`changes[].path`（仓库相对）要拼回工作区相对
    /// （真源靠绝对路径键回避同一问题，`GitFeatureModel.swift:107`）。
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

    /// 仓库根就是工作区根（Core 给相对形式，常见是空串或 `.`）时，键就是仓库相对路径本身。
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

    /// 不是仓库 / 没有变更 → 空装饰，**不是错误**（`git/mod.rs:6575-6581` 是 `ok: true`）。
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

    /// 绝对形式的仓库根：在工作区根之下就折算成相对前缀，在之外就**放弃装饰**
    /// （宁可不染色也不猜一个错误的位置）。
    ///
    /// 绝对路径的**形状因平台而异**（Windows 必须带盘符），所以用例按平台取一组假路径，
    /// 不读文件系统、不看环境变量 —— 确定性只依赖编译目标。
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

    /// 同一路径出现两条时只留第一条（真源 `GitModels.swift:921-923`），
    /// 且空路径的变更被丢掉（它没有对应的行）。
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

    /// `parse_snapshot` 对残缺载荷给出稳定错误，不 panic（Core 形状变了也不会打崩界面）。
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
