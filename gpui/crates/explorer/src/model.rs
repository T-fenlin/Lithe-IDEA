//! 项目树的数据层：`workspace.snapshot` 拉取 + 扁平路径聚成多层树。
//!
//! 只依赖 `SharedString` / `TreeItem` / `IconName` 这些数据结构，**不依赖任何 gpui UI
//! 组件**，所以建树与抓取逻辑可以脱离渲染单独读。`Explorer` 实体、`LoadState` 与所有渲染
//! 辅助在 `explorer_view.rs`；crate 的模块文档（规格出处、图标对照、未实现清单）在 `lib.rs`。
//!
//! 信封拼装已收敛到 `lithe_gpui_shared::core_json`（见 [`load_snapshot`]）。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use gpui_kit::SharedString;
use gpui_kit::assets::IconName;
use gpui_kit::component::tree::TreeItem;
use lithe_gpui_shared::core_json;
// [`core_json`] 的 payload 需要一个 `serde_json::Value`：直接依赖 `serde_json`
// （与 `gpui/crates/git` 同一口径），不借 gpui 的内部重导出。
use serde_json::json;

// ---------------------------------------------------------------------------
// 常量：文案与上限（界面文案与度量在 explorer_view.rs）
// ---------------------------------------------------------------------------

/// `fileExplorer.folderIsEmpty` → 「文件夹为空」（`locale.ts:7461`）。
/// 真机只在"整棵树为空"时用它；本项目还给**空目录**挂一条禁用占位行（原因见 [`empty_placeholder`]）。
pub(crate) const FOLDER_IS_EMPTY: &str = "文件夹为空";

/// 项目树最多渲染多少个文件路径。
///
/// 本仓库快照约 4.8 千个文件，这个上限等于"全部显示"；设它是为了防止把巨型工作区
/// （几十万文件）一次性建成树 —— 建树是同步操作，`TreeState::set_items` 会重建整棵扁平表。
/// **超出时不静默丢弃**：加载完成后打一行 `S1_EXPLORER` 诊断（与 `workspace.rs` 的
/// `S1_VIEWPORT` 是同一套探针口径）。
pub(crate) const RENDER_LIMIT: usize = 5_000;

/// 工作区根那一行的 id：空前缀 `dir:`（根工作区相对路径 = 空串）。
///
/// 真机默认**显示**根目录（`hideRootFolderInFileTree: false`，
/// `features/settings/config/default-settings.ts:184`）：根行标签 600 粗体
/// （`file-explorer/styles/file-explorer-tree.css:133-135`）、带 `data-root`
/// （`file-explorer-tree-item.tsx:218`）、且"根目录只自动展开一次"
/// （`file-explorer-tree.tsx:235-238`）。
pub(crate) const ROOT_ID: &str = "dir:";

// ---------------------------------------------------------------------------
// 数据：Core 快照 → 多层树
// ---------------------------------------------------------------------------

/// 行的种类。由 `TreeItem` 的 id 前缀解出来（`file:` / `dir:` / `empty:`）。
///
/// 用枚举而不是 `&str` 是因为渲染闭包是 `move` 的：如果 `kind`/`relative` 是借用了
/// 本地 `id` 的切片，闭包就会同时持有 `id` 和指向它的引用（自引用，编译不过）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowKind {
    File,
    Directory,
    /// 空目录的禁用占位行，见 [`empty_placeholder`]。
    EmptyPlaceholder,
}

/// 先把路径按 `/` 拆成层级，再转成 `TreeItem`。
///
/// 用 `BTreeMap` 只是为了取"目录名去重"，真正的排序在 [`to_items`] 里做。
#[derive(Default)]
pub(crate) struct PathTree {
    directories: BTreeMap<String, PathTree>,
    files: Vec<String>,
}

impl PathTree {
    /// 把一条相对路径插进树里。
    ///
    /// `path` 用 `/` 分隔（Core 返回的就是这种形式，`rust/lithe-core/src/project/files.rs:576`）；
    /// 空段会被忽略，避免建出空名字的节点。
    pub(crate) fn insert(&mut self, path: &str) {
        let mut segments = path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .peekable();
        let mut node = self;

        while let Some(segment) = segments.next() {
            if segments.peek().is_some() {
                node = node.directories.entry(segment.to_string()).or_default();
            } else {
                node.files.push(segment.to_string());
            }
        }
    }
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

/// 空目录的占位行（禁用 + 文案「文件夹为空」）。
///
/// **为什么需要它**：`TreeItem::is_folder()` 的定义是 `!children.is_empty()`
/// （`gpui-base-0.6.6/src/tree.rs:131-134`），不是文件系统概念 —— 一个没有可见子项的目录会被
/// 整棵树当作**文件行**处理：单击不会展开，而且我们的点击回调会拿它的路径去调 `on_open`
/// （等于把目录当文件打开）。挂一条禁用的子项同时解决两件事：这一行重新成为可展开的目录行，
/// 展开后显示真机已有的文案「文件夹为空」（`locale.ts:7461`）。
pub(crate) fn empty_placeholder(path: &str) -> TreeItem {
    TreeItem::new(
        SharedString::from(format!("empty:{path}")),
        SharedString::from(FOLDER_IS_EMPTY),
    )
    .disabled(true)
}

/// 把"工作区根 + 扁平路径列表"聚成**任意深度**的树。
///
/// 任意深度照真机：Windows 的 `buildVisibleFileTreeRows` 用 `walk` 递归展开 `children`
/// （`file-explorer/lib/visible-file-tree-rows.ts:154-...`），目录能一直点开到最里层。
///
/// `expanded` 是**用户当前的展开集合**（id 形如 `dir:<相对路径>`），由 [`Explorer`] 从
/// `TreeEvent` 里攒出来：搜索与刷新都会重建整棵树，不带上它就会把用户展开的目录全收起来。
/// `filter` 有值时强制展开所有层级 —— 过滤后只剩命中路径及其祖先目录，不展开就什么也看不到。
pub(crate) fn build_tree_items(
    root_label: &str,
    paths: &[String],
    expanded: &BTreeSet<SharedString>,
    filter: Option<&str>,
) -> Vec<TreeItem> {
    let mut tree = PathTree::default();
    for path in paths {
        if let Some(query) = filter
            && !path.to_lowercase().contains(query)
        {
            continue;
        }
        tree.insert(path);
    }

    let force_expanded = filter.is_some();
    let root_id = SharedString::new_static(ROOT_ID);
    let mut root_children = to_items("", &tree, expanded, force_expanded);
    if root_children.is_empty() {
        root_children = vec![empty_placeholder("")];
    }

    vec![
        TreeItem::new(root_id.clone(), SharedString::from(root_label.to_string()))
            .expanded(force_expanded || expanded.contains(&root_id))
            .children(root_children),
    ]
}

/// 把 [`PathTree`] 的一层转成 `TreeItem` 列表。
///
/// 排序照 Windows 的默认 `fileTreeSortOrder: "folders-first"`
/// （`features/settings/config/default-settings.ts:181`；比较规则见
/// `visible-file-tree-rows.ts:124-135`：目录在前，两边都按**小写**比较）—— 与 Core 自己的
/// 排列一致（`rust/lithe-core/src/project/files.rs:608-624`），所以两边的顺序不会打架。
pub(crate) fn to_items(
    prefix: &str,
    node: &PathTree,
    expanded: &BTreeSet<SharedString>,
    force_expanded: bool,
) -> Vec<TreeItem> {
    let join = |name: &str| {
        if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}/{name}")
        }
    };

    let mut directories: Vec<&String> = node.directories.keys().collect();
    directories.sort_by_key(|name| name.to_lowercase());

    let mut items: Vec<TreeItem> = directories
        .into_iter()
        .map(|name| {
            let path = join(name);
            let id = SharedString::from(format!("dir:{path}"));
            let mut children = to_items(&path, &node.directories[name], expanded, force_expanded);
            if children.is_empty() {
                children = vec![empty_placeholder(&path)];
            }

            TreeItem::new(id.clone(), SharedString::from(name.clone()))
                .expanded(force_expanded || expanded.contains(&id))
                .children(children)
        })
        .collect();

    let mut files = node.files.clone();
    files.sort_by_key(|name| name.to_lowercase());
    items.extend(files.into_iter().map(|name| {
        let path = join(&name);
        TreeItem::new(
            SharedString::from(format!("file:{path}")),
            SharedString::from(name),
        )
    }));

    items
}

/// 用真实 Core 调用取工作区快照，返回**工作区相对**的文件路径（`/` 分隔）。
///
/// 契约真源（不要照抄调研文档里的揣测）：
/// - **命令名**：`"workspace.snapshot"`（`rust/lithe-core/src/protocol/command.rs:320`）；
/// - **请求 payload**：`WorkspaceSnapshotRequest`（`rust/lithe-core/src/project/files.rs:36-45`，
///   `#[serde(rename_all = "camelCase")]`）→
///   `{ root: String, hiddenDirectoryNames: String[] = [], hiddenFilePatterns: String[] = [] }`
///   （后两个带 `#[serde(default)]`，可以不传）；信封（`id` / `operationId` /
///   `timeoutMilliseconds`）由 [`core_json`] 拼，见下；
/// - **响应 `data`**：`WorkspaceSnapshotResponse`（`rust/lithe-core/src/protocol/contracts.rs:69-75`）
///   → `{ root: WorkspaceNode, files: String[] }`；`WorkspaceNode`（同文件 `:58-67`）是
///   `{ path, name, isDirectory, children?: WorkspaceNode[] }`，**`path` 也是工作区相对路径**
///   （`project/files.rs:576,583-588`）。
///
/// 本模块只用 `data.files`（扁平相对路径）自己聚树：它是确定的 DFS 顺序，且与 `data.root` 的
/// `path` 是同一套相对路径。`root` 的**绝对路径不来自 Core**（`existing_root` 内部会
/// `canonicalize`，`project/files.rs:641-652`，但返回值里没有回传），所以拼绝对路径用的是
/// 构造时拿到的 `Explorer::root`。
///
/// 信封拼装收敛到 [`core_json`]：共享层自带自增 `operationId` 与 120 s 默认超时
/// （`gpui/crates/shared/src/core_client.rs`），所以不再自己拼 `id` / `operationId` /
/// `timeoutMilliseconds`，原先固定的 `shell-explorer-snapshot` 也随之下线（已确认无别处引用）。
///
/// 失败返回 Core 的错误码原文（`CoreError::code`）；信封级失败（响应不是合法 JSON）没有错误码，
/// 退回 `CoreError` 自己的说明性文案 —— 与收敛前的本地文案逐字一致。
pub(crate) fn load_snapshot(root: &Path) -> Result<Vec<String>, String> {
    let root_text = root
        .to_str()
        .ok_or_else(|| "工作区根不是合法 UTF-8 路径".to_string())?;

    let data = core_json("workspace.snapshot", json!({ "root": root_text })).map_err(|error| {
        match error.code() {
            Some(code) => code.to_string(),
            // 信封级失败（响应不是合法 JSON）没有 Core 错误码，保留原来的说明性文案。
            None => error.to_string(),
        }
    })?;

    let files = data
        .as_ref()
        .and_then(|data| data.get("files"))
        .and_then(|files| files.as_array())
        .ok_or("响应缺少 data.files")?;

    Ok(files
        .iter()
        .filter_map(serde_json::Value::as_str)
        .take(RENDER_LIMIT)
        .map(str::to_string)
        .collect())
}

/// 按文件名后缀挑一个**全量 Lucide 目录里确实存在**的真实字形。
///
/// 应用注册的是 `gpui_kit::assets::AllAssets`（`shell_probe/mod.rs:73`），它嵌入
/// `gpui-kit-assets-0.6.6/assets/icons/` 下的全部 1830 个 SVG（`src/native_assets.rs:8-11`），
/// 而 `gpui_kit::assets::IconName` 就是按这份目录生成的完整枚举（`build.rs:20-51`，
/// 变体名 = svg 文件名的 PascalCase）—— 所以这里可以按后缀给**真实**字形，
/// 不再需要"默认图标集里没有图片/代码字形"的替代。
///
/// 唯一例外已登记在 `lib.rs` 模块文档的替代表里：Lucide 目录**没有 `file-json.svg`**，
/// 所以 JSON 用语义最近的 `FileBraces`（`icons/file-braces.svg`，
/// Lucide 对 `braces` 的定位就是 JSON）。
///
/// 真机用的是按语言/类型区分的主题图标集（`ThemedFileIcon`，`file-explorer-tree-item.tsx:233-240`），
/// 那一套是 Windows 自己的图标资源，这里只用 Lucide 里语义最接近的字形。
pub(crate) fn icon_for_file(name: &str) -> IconName {
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());

    match extension.as_deref() {
        // 图片/图标：`Image`（`icons/image.svg`）。
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "svg" | "bmp" | "avif") => {
            IconName::Image
        }
        // JSON / JSONC：目录里**没有 `file-json.svg`**，取 `FileBraces`
        // （`icons/file-braces.svg`）—— Lucide 里花括号就是 JSON 的通用符号。
        Some("json" | "jsonc") => IconName::FileBraces,
        // Markdown / 纯文本：`FileText`（`icons/file-text.svg`）。
        Some("md" | "markdown" | "txt" | "rst" | "adoc") => IconName::FileText,
        // 配置：`Settings`（`icons/settings.svg`）。
        Some("toml" | "yaml" | "yml" | "ini" | "conf" | "cfg" | "properties" | "editorconfig") => {
            IconName::Settings
        }
        // 依赖锁 / 包文件：`Package`（`icons/package.svg`）。
        Some("lock") => IconName::Package,
        // 脚本 / 终端入口：`Terminal`（`icons/terminal.svg`）。
        Some("sh" | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "bat" | "cmd") => IconName::Terminal,
        // 源码：`FileCode`（`icons/file-code.svg`）。
        Some(
            "rs" | "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" | "py" | "java"
            | "kt" | "kts" | "swift" | "go" | "c" | "h" | "cc" | "cpp" | "hpp" | "cs" | "rb"
            | "php" | "vue" | "svelte" | "sql" | "css" | "scss" | "less" | "html" | "xml"
            | "gradle" | "lua" | "dart" | "scala" | "ex" | "exs" | "dockerfile",
        ) => IconName::FileCode,
        // 其余（含没有扩展名的）：`File`（`icons/file.svg`）。
        _ => IconName::File,
    }
}
