//! 侧栏 · 项目树（资源管理器）。
//!
//! 这一块对应 Windows 前端的 `FileExplorerPane` → `FileExplorerTree`：头部（标题 + 搜索 +
//! 偏好）+ 树体（虚拟滚动、任意深度、目录展开/折叠、点击文件打开）。规格一律取
//! `windows/tauri/src/features/file-explorer/` 的源码，每个数值都带 `文件:行号`。
//!
//! 三层结构（照真机，`windows/tauri/src/features/file-explorer/components/file-explorer-pane.tsx:32-61`）：
//!
//! 1. **头部** `SidebarHeader.file-explorer-header`：高 32、水平内边距 8、下边框 1px、
//!    标题「项目」（13px / 600）+ 搜索按钮 + 清空按钮（有查询时）+ 偏好按钮
//!    （`file-explorer-tree.tsx:1309-1518`）；
//! 2. **树体**：gpui 的 `Tree`（表体是 `uniform_list`）—— 行高固定 24、缩进 `10 + depth × 16`、
//!    目录优先 + 名字小写升序，工作区根是一行 600 粗体的根行（真机默认显示根目录）；
//! 3. **状态**：加载中顶部胶囊（`file-explorer-pane.tsx:54-60`）、空态 / 失败态用
//!    `Empty`（`ui/empty.tsx:111-130` 的 `EmptyState`）。
//!
//! 数据来自 Rust Core 的 `workspace.snapshot`（`rust/lithe-core/src/project/files.rs:36-45,160-167`），
//! **在后台线程**取，回前台再建树（见 [`Explorer::load`]）。
//!
//! 已知取舍（各自的理由写在使用处）：
//! - 树内搜索用**内联一整行输入框**代替 Windows 的 `SidebarSearchPopover` 浮层；
//! - 空目录挂一条禁用占位行「文件夹为空」，因为 `TreeItem::is_folder()` 是"有没有子节点"
//!   推导出来的（`gpui-base-0.6.6/src/tree.rs:131-134`），没有子节点就会被当成文件行；
//! - 未能实现的项逐条登记在文件末尾。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex, TreeState};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tree::{Tree, TreeEvent, TreeItem};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, Role, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, WeakEntity, Window, div, px, relative,
};

// ---------------------------------------------------------------------------
// 文案（全部逐字取 `windows/tauri/src/i18n/locale.ts`，不自己编中文）
// ---------------------------------------------------------------------------

/// `workbench.project` → 「项目」（`locale.ts:4605`）：文件树头部标题。
const TITLE: &str = "项目";
/// `fileExplorer.searchFiles` → 「搜索文件」（`locale.ts:7446`）：搜索按钮的提示 + 输入框占位。
const SEARCH_FILES: &str = "搜索文件";
/// `search.clear` → 「清除搜索」（`locale.ts:5895`）。
const CLEAR_SEARCH: &str = "清除搜索";
/// `fileExplorer.preferences` → 「文件资源管理器偏好设置」（`locale.ts:7445`）。
const PREFERENCES: &str = "文件资源管理器偏好设置";
/// `fileExplorer.ariaLabel` → 「文件资源管理器」（`locale.ts:7447`；调研文档 §5.4 漏了这一条）。
const TREE_ARIA_LABEL: &str = "文件资源管理器";
/// `quickOpen.loadingFiles` → 「正在加载文件」（`locale.ts:7835`）。
const LOADING_FILES: &str = "正在加载文件";
/// `ui.retry` → 「重试」（`locale.ts:4628`）。
const RETRY: &str = "重试";
/// `fileExplorer.noFolderOpen` → 「未打开文件夹」（`locale.ts:7458`）。
const NO_FOLDER_OPEN: &str = "未打开文件夹";
/// `welcome.openFolder` → 「打开文件夹」（`locale.ts:8740`）。
const OPEN_FOLDER: &str = "打开文件夹";
/// `fileExplorer.noMatchingFiles` → 「没有匹配的文件」（`locale.ts:7460`）。
const NO_MATCHING_FILES: &str = "没有匹配的文件";
/// `fileExplorer.folderIsEmpty` → 「文件夹为空」（`locale.ts:7461`）。
/// 真机只在"整棵树为空"时用它；本项目还给**空目录**挂一条禁用占位行（原因见 [`empty_placeholder`]）。
const FOLDER_IS_EMPTY: &str = "文件夹为空";

// ---------------------------------------------------------------------------
// 度量（全部取 Windows 源码；`px()` 直搬，`gpui/UI-MAP.md` §1.1 第 1 条）
// ---------------------------------------------------------------------------

/// 侧栏头部高度 32px（`styles/theme.css:124` `--lithe-sidebar-header-height: 2rem`）。
const HEADER_HEIGHT: f32 = 32.;
/// 头部水平内边距 8px（`file-explorer-tree.tsx:1310` 的 `px-2`）。
const HEADER_PADDING_INLINE: f32 = 8.;
/// 头部纵向内边距 4px（`ui/sidebar.tsx:93` 的 `py-1`）。
const HEADER_PADDING_BLOCK: f32 = 4.;
/// 头部图标按钮 24×24（`ui/button.tsx:27` `"icon-xs": "size-6 p-0"`）。
const HEADER_BUTTON_SIZE: f32 = 24.;
/// 头部图标按钮圆角 6.4px（`ui/button.tsx:9` 的 `rounded-md` → `--radius-md = --radius × 0.8`，
/// `theme.css:7,134`）。
const HEADER_BUTTON_RADIUS: f32 = 6.4;
/// 头部标题行高 16px（`file-explorer/styles/file-explorer-tree.css:151-156`：
/// `font-size: var(--ui-text-chrome)` / `font-weight: 600` / `line-height: var(--lithe-chrome-line-height)`，
/// 后者的值 1rem 见 `theme.css:128`）。
const TITLE_LINE_HEIGHT: f32 = 16.;
/// 搜索输入框高 28px（`h-7`；树内搜索框与全局搜索工具栏是同一套 Chrome 控件，
/// `global-search-toolbar.tsx:101-102`）+ 圆角 8px（`rounded-lg`）。
const SEARCH_INPUT_HEIGHT: f32 = 28.;
const SEARCH_INPUT_RADIUS: f32 = 8.;
/// 搜索行内边距 8px（`global-search-toolbar.tsx:93` 的 `py-2`、compact `px-2`）。
const SEARCH_ROW_PADDING: f32 = 8.;

/// 树行高 24px。
///
/// 真机公式 `max(24, uiFontSize × 1.35 + 6)`，`uiFontSize = 13` → `max(24, 23.55) = 24`
/// （`file-explorer/lib/file-tree-row.ts:1-14`；13px 见 `theme.css:112`）。
///
/// 表体是 `uniform_list`，**行高只取第 0 行的测量值**再按固定值铺满
/// （`gpui-pre-0.3.6/src/elements/uniform_list.rs:658-680,371,397`），所以行内容必须正好是这个高度，
/// 行内纵向内边距一律清零（`ListItem` 默认 `py_1`，见 `gpui-component-0.6.6/src/list/list_item.rs:186`）。
const ROW_HEIGHT: f32 = 24.;
/// 行基准缩进 10px（`file-explorer/lib/file-tree-row.ts:1` `FILE_TREE_BASE_INDENT = 10`）。
const BASE_INDENT: f32 = 10.;
/// 缩进步长 16px（默认 `fileTreeIndentSize: 16`，`features/settings/config/default-settings.ts:182`；
/// 可选 12/16/20/24 见 `file-explorer-tree.tsx:1482-1493`）。
const INDENT_STEP: f32 = 16.;
/// 树体水平内缩 6px（`file-explorer/styles/file-explorer-tree.css:6,36` `--file-tree-row-inline-inset`）。
///
/// 真机给每一行（`.file-tree-virtual-row`）加 `padding-inline`，于是行底色左右各缩 6px。
/// `uniform_list` 的行没法加外边距，等价做法是把整棵树套一层 `px(6.)`（见 [`Explorer::render_tree`]）。
const ROW_INLINE_INSET: f32 = 6.;
/// 行内水平内边距 6px（`features/sidebar/components/sidebar-tree.tsx:241` 的 `px-1.5`）。
/// 左内边距被行内样式覆写成 `10 + depth × 16`（`sidebar-tree.tsx:246`），右内边距保持 6。
const ROW_PADDING_INLINE: f32 = 6.;
/// 行内列间距 4px（`file-explorer-tree.css:56` `column-gap: var(--lithe-chrome-gap) !important`，
/// `--lithe-chrome-gap: 4px` 见 `theme.css:130`）。
const ROW_GAP: f32 = 4.;
/// 展开箭头槽 16×16（`sidebar-tree.tsx:301` 的 `size-4`），右侧另加 2px（同行 `mr-0.5`）。
const DISCLOSURE_SIZE: f32 = 16.;
const DISCLOSURE_MARGIN: f32 = 2.;
/// 箭头字形 12×12（`file-explorer-tree.css:121-124` 的 `svg { width: 12px; height: 12px }`）。
const CARET_SIZE: f32 = 12.;
/// 文件/目录图标 16×16（`file-explorer-tree.css:5,126-131` `--file-tree-icon-size: 16px`）。
const ICON_SIZE: f32 = 16.;
/// 行圆角 4px（`file-explorer-tree.css:7` `--file-tree-row-radius: 4px`）。
const ROW_RADIUS: f32 = 4.;
/// 树行字号 13px（`file-explorer-tree.css:100` `font-size: var(--ui-text-sm)`；
/// `--ui-text-sm = --app-ui-font-size = 13px`，`theme.css:112,116`）。
const ROW_TEXT_SIZE: f32 = 13.;
/// 行高倍数 1.35（`theme.css:4` `--leading-row: 1.35`；`sidebar-tree.tsx:241` 的 `leading-row`）。
const ROW_LINE_HEIGHT: f32 = 1.35;
/// 加载胶囊：`p-3`(12) 定位、`px-3 py-1.5`(12/6)、`rounded-full`
/// （`file-explorer-pane.tsx:55-56`）；间距 8 与 13px 字号取 `ui/spinner.tsx:44` 的 `gap-2 … ui-text-sm`。
const LOADING_PILL_OFFSET: f32 = 12.;
const LOADING_PILL_PADDING_INLINE: f32 = 12.;
const LOADING_PILL_PADDING_BLOCK: f32 = 6.;
const LOADING_PILL_GAP: f32 = 8.;

/// 项目树最多渲染多少个文件路径。
///
/// 本仓库快照约 4.8 千个文件，这个上限等于"全部显示"；设它是为了防止把巨型工作区
/// （几十万文件）一次性建成树 —— 建树是同步操作，`TreeState::set_items` 会重建整棵扁平表。
/// **超出时不静默丢弃**：加载完成后打一行 `S1_EXPLORER` 诊断（与 `workspace.rs` 的
/// `S1_VIEWPORT` 是同一套探针口径）。
const RENDER_LIMIT: usize = 5_000;

/// 工作区根那一行的 id：空前缀 `dir:`（根工作区相对路径 = 空串）。
///
/// 真机默认**显示**根目录（`hideRootFolderInFileTree: false`，
/// `features/settings/config/default-settings.ts:184`）：根行标签 600 粗体
/// （`file-explorer/styles/file-explorer-tree.css:133-135`）、带 `data-root`
/// （`file-explorer-tree-item.tsx:218`）、且"根目录只自动展开一次"
/// （`file-explorer-tree.tsx:235-238`）。
const ROOT_ID: &str = "dir:";

// ---------------------------------------------------------------------------
// 数据：Core 快照 → 多层树
// ---------------------------------------------------------------------------

/// 行的种类。由 `TreeItem` 的 id 前缀解出来（`file:` / `dir:` / `empty:`）。
///
/// 用枚举而不是 `&str` 是因为渲染闭包是 `move` 的：如果 `kind`/`relative` 是借用了
/// 本地 `id` 的切片，闭包就会同时持有 `id` 和指向它的引用（自引用，编译不过）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum RowKind {
    File,
    Directory,
    /// 空目录的禁用占位行，见 [`empty_placeholder`]。
    EmptyPlaceholder,
}

/// 项目树的加载状态。
enum LoadState {
    /// 正在后台取 `workspace.snapshot`。
    Loading,
    /// 快照到手（或工作区根为空，此时没有可加载的东西）。
    Ready,
    /// 取快照失败：存 **Core 的错误码原文**（例如 `WorkspaceNotFound`）。
    ///
    /// 这里刻意不编中文：`locale.ts` 里没有"文件树加载失败"这类键，最近的
    /// `files.unableToDetermineRootPath`（「无法确定根文件夹路径」，`locale.ts:7413`）语义只覆盖
    /// "根路径不合法"，套到任意失败上是错的。显示稳定错误码对排查更有用，也不暴露环境细节。
    Failed(String),
}

impl LoadState {
    /// 是否正在取快照（渲染时用来决定要不要画加载胶囊）。
    fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }
}

/// 先把路径按 `/` 拆成层级，再转成 `TreeItem`。
///
/// 用 `BTreeMap` 只是为了取"目录名去重"，真正的排序在 [`to_items`] 里做。
#[derive(Default)]
struct PathTree {
    directories: BTreeMap<String, PathTree>,
    files: Vec<String>,
}

impl PathTree {
    /// 把一条相对路径插进树里。
    ///
    /// `path` 用 `/` 分隔（Core 返回的就是这种形式，`rust/lithe-core/src/project/files.rs:576`）；
    /// 空段会被忽略，避免建出空名字的节点。
    fn insert(&mut self, path: &str) {
        let mut segments = path.split('/').filter(|segment| !segment.is_empty()).peekable();
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
    fn parse(id: &str) -> (Self, String) {
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
fn empty_placeholder(path: &str) -> TreeItem {
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
fn build_tree_items(
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
fn to_items(
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
///   （后两个带 `#[serde(default)]`，可以不传）；信封还要求
///   `id` / `operationId` / `timeoutMilliseconds` / `command` / `payload`；
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
/// 失败返回 Core 的错误码原文（`/error/code`）；JSON 解析失败也当失败。
fn load_snapshot(root: &Path) -> Result<Vec<String>, String> {
    let root_text = root
        .to_str()
        .ok_or_else(|| "工作区根不是合法 UTF-8 路径".to_string())?;

    let request = serde_json::json!({
        "id": "shell-explorer-snapshot",
        "operationId": "shell-explorer-snapshot",
        "timeoutMilliseconds": 120_000,
        "command": "workspace.snapshot",
        "payload": { "root": root_text },
    })
    .to_string();

    let raw = lithe_core::execute_json(&request);
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|error| format!("响应不是合法 JSON：{error}"))?;

    if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        let code = value
            .pointer("/error/code")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        return Err(code.to_string());
    }

    let files = value
        .pointer("/data/files")
        .and_then(serde_json::Value::as_array)
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
/// 唯一例外已登记在文件末尾的替代表里：Lucide 目录**没有 `file-json.svg`**，
/// 所以 JSON 用语义最近的 `FileBraces`（`icons/file-braces.svg`，
/// Lucide 对 `braces` 的定位就是 JSON）。
///
/// 真机用的是按语言/类型区分的主题图标集（`ThemedFileIcon`，`file-explorer-tree-item.tsx:233-240`），
/// 那一套是 Windows 自己的图标资源，这里只用 Lucide 里语义最接近的字形。
fn icon_for_file(name: &str) -> IconName {
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

// ---------------------------------------------------------------------------
// 组件
// ---------------------------------------------------------------------------

/// 侧栏的项目树（资源管理器）。
///
/// 对外只暴露 [`Explorer::new`]、[`Explorer::refresh`] 与 `Render`；字段全部私有。
/// 点击文件时通过构造时传入的 `on_open` 回调把**绝对路径**抛给外壳（主代理接到编辑区）；
/// 点击目录只做展开/折叠 —— 由 `Tree` 自己在 `mouse_down` 里处理
/// （`gpui-base-0.6.6/src/tree.rs:413-417,442-456`）。
pub struct Explorer {
    /// 工作区根（外壳给出；Core 不回传它，所以自己留着拼绝对路径）。
    root: PathBuf,
    /// 最近一次成功加载的**工作区相对**路径（已截断到 [`RENDER_LIMIT`]）。
    paths: Vec<String>,
    /// 加载状态。
    state: LoadState,
    /// 树的交互状态（选中、滚动、展开后的扁平表）。
    tree: Entity<TreeState>,
    /// 用户展开过的目录 id（`dir:<相对路径>`）。
    ///
    /// 从 `TreeEvent::Expanded/Collapsed` 攒出来（`gpui-base-0.6.6/src/tree.rs:91-96,320-338`）。
    expanded: BTreeSet<SharedString>,
    /// 最后一次点击打开的文件（相对路径），用来给那一行加底色。
    active: Option<SharedString>,
    /// 树内搜索的输入状态。
    search: Entity<InputState>,
    /// 搜索输入框是否展开（真机是 `SidebarSearchPopover` 的 `open`，`file-explorer-tree.tsx:1321`）。
    search_open: bool,
    /// 当前查询词（小写比较在 [`build_tree_items`] 里做）。
    query: SharedString,
    /// 点击文件时往外抛的回调。
    ///
    /// 存 `Rc` 而不是 `Box`：渲染闭包要求 `'static`，没法借用 `&self.on_open`。
    on_open: Rc<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>,
    /// `TreeEvent` 订阅（攒 [`Explorer::expanded`]）。订阅器一 drop 就失效，所以要存住。
    _tree_events: Subscription,
    /// `InputEvent` 订阅（查询变化 → 重建树）。同上。
    _search_events: Subscription,
}

impl Explorer {
    /// `root` = 工作区根；`on_open` = 点击文件时往外抛的回调（主代理会接到编辑区）。
    ///
    /// 构造期只做两件轻活：建两个 `Entity`、挂两个订阅，**不取数据** —— 快照是后台任务，
    /// 见 [`Explorer::load`]。
    pub fn new(
        root: PathBuf,
        on_open: Box<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let tree = cx.new(|cx| TreeState::new(cx));
        let search = cx.new(|cx| InputState::new(window, cx));
        search.update(cx, |state, cx| {
            state.set_placeholder(SEARCH_FILES, window, cx);
        });

        // 攒展开状态。`TreeEvent` 只有 `Expanded` / `Collapsed` 两个变体
        // （`gpui-base-0.6.6/src/tree.rs:91-96`），所以这里能直接取里面的 id。
        let tree_events = cx.subscribe(
            &tree,
            |this, _tree, event: &TreeEvent, _cx: &mut Context<Self>| match event {
                TreeEvent::Expanded(id) => {
                    this.expanded.insert(id.clone());
                }
                TreeEvent::Collapsed(id) => {
                    this.expanded.remove(id);
                }
            },
        );

        // 查询变化 → 重建树。`InputEvent` 见 `gpui-base-0.6.6/src/input/base/state.rs:121-127`。
        let search_events = cx.subscribe(
            &search,
            |this, _input, event: &InputEvent, cx: &mut Context<Self>| {
                if matches!(event, InputEvent::Change) {
                    // ⚠️ `&**cx`：`Entity::read` 要 `&App`，而这里是 `&mut Context<Self>`。
                    // 显式走两级 Deref（Context<Self> → App），不依赖多级 deref 强制转换。
                    let query = this.search.read(&**cx).value();
                    this.set_query(query, cx);
                }
            },
        );

        let mut explorer = Self {
            root,
            paths: Vec::new(),
            state: LoadState::Loading,
            tree,
            expanded: BTreeSet::new(),
            active: None,
            search,
            search_open: false,
            query: SharedString::default(),
            on_open: Rc::from(on_open),
            _tree_events: tree_events,
            _search_events: search_events,
        };
        // 根行默认展开一次（真机 `file-explorer-tree.tsx:235-238`）。
        explorer.expanded.insert(SharedString::new_static(ROOT_ID));
        explorer.load(cx);
        explorer
    }

    /// 重新取一次工作区快照。
    ///
    /// 真机的「刷新」在右键菜单里（`files.refresh`「刷新」，`locale.ts:7421`；
    /// `use-file-explorer-context-menu.tsx:221`），本模块没有画右键菜单，所以把它开成公开方法
    /// 交给外壳接线。
    ///
    /// `window` 现在用不上（后台任务不需要窗口），保留参数是为了调用形态与 `new` 一致。
    pub fn refresh(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.load(cx);
    }

    /// 取工作区快照：**在后台线程调 Core**，回到前台才建树。
    ///
    /// `lithe_core::execute_json` 是同步调用（`rust/lithe-core/src/lib.rs:25`），直接放在 UI 线程上
    /// 会把整棵树扫描（本仓库约 4.8 千个文件）挡在渲染前面，所以走 `cx.background_spawn`
    /// （`gpui-pre-0.3.6/src/app.rs:3073-3078`，`AppContext` trait 上的方法）再 `await` 回前台；
    /// 调用形态与上一轮跑通的 `workspace.rs` 加载一致。
    fn load(&mut self, cx: &mut Context<Self>) {
        self.state = LoadState::Loading;
        cx.notify();

        // 工作区根为空：没有可加载的东西，直接进空态（真机的 `!rootFolderPath`，
        // `file-explorer-tree.tsx:1532-1538`）。
        if self.root.as_os_str().is_empty() {
            self.paths.clear();
            self.state = LoadState::Ready;
            self.rebuild(cx);
            return;
        }

        let root = self.root.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { load_snapshot(&root) })
                .await;

            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(paths) => {
                        // 截断不静默：把工作区根与渲染量打一行诊断（探针口径同 `workspace.rs`
                        // 的 `S1_VIEWPORT`）；总量超出 `RENDER_LIMIT` 时这一行就是证据。
                        println!(
                            "S1_EXPLORER root={} rendered={} limit={}",
                            this.root.display(),
                            paths.len(),
                            RENDER_LIMIT,
                        );
                        this.paths = paths;
                        this.state = LoadState::Ready;
                    }
                    Err(error) => {
                        println!("S1_EXPLORER root={} error={error}", this.root.display());
                        this.paths.clear();
                        this.state = LoadState::Failed(error);
                    }
                }
                this.rebuild(cx);
            });
        })
        .detach();
    }

    /// 按当前的 `paths` / `expanded` / `query` 重建树。
    ///
    /// `TreeState::set_items` 会**清空选中**（`gpui-base-0.6.6/src/tree.rs:214-219`），这是刻意的：
    /// 可见行被换掉之后旧行号已经没有意义（真机搜索时也是重新定位）。
    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let filter: Option<String> = if self.query.is_empty() {
            None
        } else {
            Some(self.query.to_lowercase())
        };
        let label = root_label(&self.root);
        let items = build_tree_items(&label, &self.paths, &self.expanded, filter.as_deref());
        self.tree.update(cx, |state, cx| state.set_items(items, cx));
    }

    /// 写入查询词并重建树（`InputEvent::Change` 与「清除搜索」都走这里）。
    fn set_query(&mut self, query: SharedString, cx: &mut Context<Self>) {
        if self.query == query {
            return;
        }
        self.query = query;
        self.rebuild(cx);
        cx.notify();
    }

    /// 展开 / 收起树内搜索。
    ///
    /// 收起时清空查询：真机的 `Escape` 也是"关闭搜索"（`file-explorer-tree.tsx:1329-1335`），
    /// 留着过滤词会让下次打开看到一棵"少了东西"的树。
    fn toggle_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = !self.search_open;
        if self.search_open {
            let search = self.search.clone();
            search.update(cx, |state, cx| state.focus(window, cx));
        } else {
            self.clear_search(window, cx);
        }
        cx.notify();
    }

    /// 清空查询并重建（对应 `search.clear`「清除搜索」按钮，`file-explorer-tree.tsx:1344-1356`）。
    ///
    /// ⚠️ **不能只靠 `InputEvent::Change`**：`InputState::set_value` 内部把 `emit_events` 置成
    /// `false`（`gpui-base-0.6.6/src/input/base/state.rs:903-907`），**不会**发 `Change` 事件，
    /// 所以这里必须自己调 [`Explorer::set_query`]。
    fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let search = self.search.clone();
        search.update(cx, |state, cx| {
            state.set_value(SharedString::default(), window, cx);
        });
        self.set_query(SharedString::default(), cx);
    }

    // ---- 渲染 ----

    /// 头部：标题「项目」+ 搜索按钮 + 清空按钮（有查询时）+ 偏好按钮。
    ///
    /// 组成与度量照 `file-explorer-tree.tsx:1309-1518`：`SidebarHeader` 高 32
    /// （`ui/sidebar.tsx:93` 的 `h-(--lithe-sidebar-header-height)`）、`px-2 py-1`、
    /// 间距 4（`gap-(--lithe-chrome-gap)`）、下边框 1px（`file-explorer-tree.css:146`）。
    ///
    /// 三个按钮都是 `SidebarHeaderIconButton`（`ui/sidebar.tsx:127-141`）：ghost 变体
    /// （前景 `--subtle-foreground`、悬停底色 `--accent`，`ui/button.tsx:17`）、
    /// `icon-xs` 24×24（`ui/button.tsx:27`）、`rounded-md` 6.4px（`ui/button.tsx:9`）。
    fn render_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut header = h_flex()
            .w_full()
            .flex_shrink_0()
            .h(px(HEADER_HEIGHT))
            .gap(px(ROW_GAP))
            .px(px(HEADER_PADDING_INLINE))
            .py(px(HEADER_PADDING_BLOCK))
            .bg(cx.theme().background)
            .border_b_1()
            // 真机是 `color-mix(var(--border) 72%, transparent)`（`file-explorer-tree.css:146`）。
            // gpui-kit 没有"72% 透明度的边框"token（`--border-strong` 是混进前景色，语义不同），
            // 取最接近的 `theme.border` —— 差一档不透明度，登记在未实现清单里。
            .border_color(cx.theme().border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(ROW_TEXT_SIZE))
                    .font_semibold()
                    .line_height(px(TITLE_LINE_HEIGHT))
                    .text_color(cx.theme().foreground)
                    .child(SharedString::from(TITLE)),
            )
            .child(header_button(
                "explorer-search",
                IconName::Search,
                SEARCH_FILES,
                Some(Box::new(cx.listener(
                    |this: &mut Self, _event: &ClickEvent, window: &mut Window, cx: &mut Context<Self>| {
                        this.toggle_search(window, cx);
                    },
                ))),
            ));

        if !self.query.is_empty() {
            header = header.child(header_button(
                "explorer-search-clear",
                IconName::X,
                CLEAR_SEARCH,
                Some(Box::new(cx.listener(
                    |this: &mut Self, _event: &ClickEvent, window: &mut Window, cx: &mut Context<Self>| {
                        this.clear_search(window, cx);
                    },
                ))),
            ));
        }

        // 偏好下拉：真机是 `DropdownMenu`（可见性 / 外观 / 排序 / 缩进 / 自动定位 / 删除确认，
        // `file-explorer-tree.tsx:1357-1517`）。这些设置项在探针里**没有落点**（没有设置存储，
        // 也没有 `showHiddenFilesInFileTree` 这类可见性过滤的数据源），所以按"宁可禁用也不画假按钮"
        // 的约定渲染成**禁用态**并登记，而不是画一个点了没反应的按钮。
        header
            .child(header_button(
                "explorer-preferences",
                IconName::Settings,
                PREFERENCES,
                None,
            ))
            .into_any_element()
    }

    /// 树内搜索行。展开时插在头部**下面**。
    ///
    /// ⚠️ **与真机的差异**：真机把它做成锚在搜索按钮上的浮层 `SidebarSearchPopover`
    /// （`file-explorer-tree.tsx:1317-1343` → `ui/sidebar.tsx:143-...`），本模块渲染成
    /// 一整行内联输入框：gpui-kit 0.6.6 的 `Popover` 必须自带 trigger 并锚在 trigger 上
    /// （`gpui/UI-MAP.md` §1.3 浮层一节），而这一行的高度预算（28 + 8 + 8 = 44）在侧栏里够用，
    /// 内联更容易点中、也少一层焦点陷阱。度量仍照真机：输入框高 28（`h-7`）、圆角 8
    /// （`rounded-lg`）、整行水平内边距 8（compact `px-2`）、下边框 1px、底色 `--surface/55`
    /// （`global-search-toolbar.tsx:93,101-102`；树内搜索框与全局搜索工具栏是同一套 Chrome 控件）。
    fn render_search_row(&self, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.search_open {
            return None;
        }

        // ⚠️ **必须用 `Styled::h` 的全限定写法**：`Input` 有一个**同名固有方法**
        // `Input::h(impl Into<DefiniteLength>)`，它只写 `self.height`，而那个字段只在
        // **多行**输入里生效（`gpui-component-0.6.6/src/input/input.rs:256-260,706-709`）。
        // 单行输入的实际高度来自 `input_h(self.size)` —— `Size::Medium` → `h_8()` = 32px
        // （`input.rs:703`、`sizing.rs:261-269`）。方法调用语法会优先挑固有方法，所以这里
        // 走 `Styled::h(...)` 直接给样式表写高 28px（`refine_style` 在 `input_h` 之后执行，
        // `input.rs:703,719`，能覆写掉它）。
        let input = gpui_kit::Styled::h(
            Input::new(&self.search),
            px(SEARCH_INPUT_HEIGHT),
        )
        .w_full()
        .rounded(px(SEARCH_INPUT_RADIUS))
        .text_size(px(ROW_TEXT_SIZE));

        Some(
            h_flex()
                .w_full()
                .flex_shrink_0()
                .px(px(SEARCH_ROW_PADDING))
                .py(px(SEARCH_ROW_PADDING))
                .bg(cx.theme().muted)
                .border_b_1()
                .border_color(cx.theme().border)
                .child(input)
                .into_any_element(),
        )
    }

    /// 主体：加载中的胶囊 +（树体 / 空态 / 失败态）。
    fn render_body(&self, cx: &mut Context<Self>) -> AnyElement {
        // 首次加载时真机**不挂载树**，只显示顶部居中的加载胶囊（`file-explorer-pane.tsx:34-60`）；
        // 已有数据时刷新则保留树体、胶囊浮在上面（真机靠一层 `absolute inset-0` 做同一件事）。
        let content: AnyElement = if self.state.is_loading() && self.paths.is_empty() {
            div().flex_1().into_any_element()
        } else {
            match &self.state {
                LoadState::Failed(error) => self.empty_failed(error.clone(), cx),
                _ if self.paths.is_empty() => self.empty_no_rows(cx),
                _ => self.render_tree(cx),
            }
        };

        let loading = self.state.is_loading();
        v_flex()
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .when(loading, |this| {
                this.child(
                    h_flex()
                        .absolute()
                        .top(px(LOADING_PILL_OFFSET))
                        .left_0()
                        .right_0()
                        .justify_center()
                        .child(
                            // 真机胶囊：`rounded-full border border-border/60 bg-surface/92
                            // px-3 py-1.5 shadow-popover backdrop-blur-sm`
                            // （`file-explorer-pane.tsx:55-56`）。`bg-surface/92` 取 `theme.muted`
                            // （`--surface` → `muted`）；阴影与背景模糊没有对应 token，不画
                            // （登记在未实现清单里）。
                            h_flex()
                                .gap(px(LOADING_PILL_GAP))
                                .px(px(LOADING_PILL_PADDING_INLINE))
                                .py(px(LOADING_PILL_PADDING_BLOCK))
                                .rounded_full()
                                .bg(cx.theme().muted)
                                .border_1()
                                .border_color(cx.theme().border)
                                .text_size(px(ROW_TEXT_SIZE))
                                .text_color(cx.theme().muted_foreground)
                                // 转圈图标 16 + 文案 13 + 间距 8：`ui/spinner.tsx:20-51` 的
                                // `size-4` / `ui-text-sm` / `gap-2`。`Spinner` 默认 `Size::Medium`
                                // → 16px（`gpui-component-0.6.6/src/spinner.rs:22`、`icon.rs:185`），
                                // 自带 `with_animation` 常转（`spinner.rs:60-74`）。
                                .child(Spinner::new().color(cx.theme().muted_foreground))
                                .child(SharedString::from(LOADING_FILES)),
                        ),
                )
            })
            .child(content)
            .into_any_element()
    }

    /// 树体。
    ///
    /// - 外面套 `px(6.)`：真机给每一行加 `padding-inline: 6px`
    ///   （`file-explorer-tree.css:6,36`），行底色因此左右各缩 6px；`uniform_list` 的行不能加
    ///   外边距，等价做法是把整棵树缩进 6px。所以行内的左内边距只需
    ///   `10 + depth × 16`（真机的 `paddingLeft`，`sidebar-tree.tsx:246`）。
    /// - 行内容按真机的 `SidebarTreeRow`（`features/sidebar/components/sidebar-tree.tsx:231-278`）。
    /// - `ListItem` 的 children 是**竖排**的（它内部装 children 的是普通块级 `div`，
    ///   `gpui-component-0.6.6/src/list/list_item.rs:215-221`），所以整行必须自己套一层 `h_flex()`。
    /// - 行高由内容决定、`uniform_list` 只量第 0 行（`gpui/UI-MAP.md` §1.3），所以行内容按
    ///   [`ROW_HEIGHT`] 定死，`ListItem` 默认的纵向内边距用 `p_0()` 清掉。
    fn render_tree(&self, cx: &mut Context<Self>) -> AnyElement {
        let on_open = self.on_open.clone();
        let active = self.active.clone();
        let root = self.root.clone();
        // 点击时要把"当前打开的文件"记回来给行加底色，但 `ListItem::on_click` 只给 `&mut App`，
        // 拿不到 `Context<Explorer>`，所以带一个弱引用进来。
        let explorer: WeakEntity<Self> = cx.entity().downgrade();

        let tree = Tree::new(&self.tree, move |index, entry, selected, _window, cx| {
            let id = entry.item().id.clone();
            let label = entry.item().label.clone();
            let is_folder = entry.is_folder();
            let expanded = entry.is_expanded();
            let (kind, relative_path) = RowKind::parse(id.as_ref());
            let is_active =
                kind == RowKind::File && active.as_deref() == Some(relative_path.as_str());

            // 展开箭头：只有目录行有；其它行留一个**等宽空槽**，同层的图标才能左对齐
            // （真机的 `reserveDisclosureSpace`，`file-explorer-tree-item.tsx:211-213`）。
            let caret: AnyElement = if is_folder {
                h_flex()
                    .w(px(DISCLOSURE_SIZE))
                    .h(px(DISCLOSURE_SIZE))
                    .mr(px(DISCLOSURE_MARGIN))
                    .flex_shrink_0()
                    .justify_center()
                    .child(
                        Icon::new(if expanded {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .w(px(CARET_SIZE))
                        .h(px(CARET_SIZE))
                        .text_color(cx.theme().muted_foreground),
                    )
                    .into_any_element()
            } else {
                div()
                    .w(px(DISCLOSURE_SIZE))
                    .h(px(DISCLOSURE_SIZE))
                    .mr(px(DISCLOSURE_MARGIN))
                    .flex_shrink_0()
                    .into_any_element()
            };

            let icon = match kind {
                RowKind::EmptyPlaceholder => IconName::File,
                RowKind::Directory => {
                    if expanded {
                        IconName::FolderOpen
                    } else {
                        IconName::FolderClosed
                    }
                }
                RowKind::File => icon_for_file(&label),
            };

            let mut label_el = div().flex_1().min_w_0().truncate().child(label);
            // 根行标签 600 粗体（`file-explorer-tree.css:133-135`
            // `.file-tree-row[data-root="true"] .file-tree-node-label { font-weight: 600 }`）。
            // 整棵树只有一条 `depth == 0` 的行，就是工作区根
            // （`TreeEntry::is_root()`，`gpui-base-0.6.6/src/tree.rs:71-73`）。
            if entry.is_root() {
                label_el = label_el.font_semibold();
            }

            let row = h_flex()
                .w_full()
                .h(px(ROW_HEIGHT))
                .gap(px(ROW_GAP))
                .pl(px(BASE_INDENT + entry.depth() as f32 * INDENT_STEP))
                .pr(px(ROW_PADDING_INLINE))
                .rounded(px(ROW_RADIUS))
                .child(caret)
                .child(
                    Icon::new(icon)
                        .w(px(ICON_SIZE))
                        .h(px(ICON_SIZE))
                        .text_color(cx.theme().muted_foreground),
                )
                .child(label_el)
                // 「当前打开的文件」加一层选中底色。真机把"活动文件"与"悬停"都用
                // `subtleSelection`，两者在真机上不可区分（`gpui/UI-MAP.md` §2.3 的"优化"一条），
                // 这里直接用选中色 `list_active`。
                .when(is_active, |this| this.bg(cx.theme().list_active));

            // 目录行的展开/折叠由 `Tree` 在 `mouse_down` 里处理；空目录占位行是禁用的，
            // 连 `mouse_down` 都不会挂上（`gpui-base-0.6.6/src/tree.rs:413-417,442-456`）。
            let opening: Option<PathBuf> = if kind == RowKind::File {
                Some(join_relative(&root, &relative_path))
            } else {
                None
            };

            // ⚠️ 这个外层渲染闭包是 `Fn`（`Tree::new` 要求，`gpui-component-0.6.6/src/tree.rs:41-43`），
            // 所以**不能**把捕获到的 `on_open` / `explorer` 移进下面那个 `move` 点击闭包 ——
            // 每行都要现克隆一份（`relative_path` / `opening` 是本次调用新建的局部量，可以直接移）。
            let row_on_open = on_open.clone();
            let row_explorer = explorer.clone();
            let row_relative_path = relative_path;
            let row_path = opening;

            ListItem::new(SharedString::from(format!("explorer-row-{index}")))
                .selected(selected)
                // 行高必须可控：`ListItem` 默认 `py_1() px_3()`
                // （`gpui-component-0.6.6/src/list/list_item.rs:186-187`），纵向内边距会让
                // 24px 的行装不下内容（`gpui/UI-MAP.md` §1.3 第一条）。
                .p_0()
                .h(px(ROW_HEIGHT))
                .text_size(px(ROW_TEXT_SIZE))
                .line_height(relative(ROW_LINE_HEIGHT))
                .whitespace_nowrap()
                .overflow_hidden()
                .child(row)
                .on_click(
                    move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
                        let Some(path) = row_path.clone() else {
                            return;
                        };
                        row_on_open(path, window, cx);
                        // 记住这次打开的文件，给这一行加底色；实体已经没了就跳过（不影响打开）。
                        let _ = row_explorer.update(cx, |this, cx| {
                            this.active = Some(SharedString::from(row_relative_path.clone()));
                            cx.notify();
                        });
                    },
                )
        })
        .size_full();

        // 树的 a11y：`gpui_base::Tree` 自己会挂 `role(Tree)`（`gpui-base-0.6.6/src/tree.rs:517-522`），
        // 但不会设 aria-label，所以在包裹层补一次（真机 `aria-label` 取
        // `fileExplorer.ariaLabel`，`file-explorer-tree.tsx:1524`）。
        // ⚠️ `.role()` / `.aria_label()` 在 `StatefulInteractiveElement` 上，而它只对
        // `Stateful<E>` 实现（`gpui-pre-0.3.6/src/elements/div.rs:1300-1326,4074`），
        // 所以必须先 `.id(...)` 把 `Div` 变成 `Stateful<Div>`。
        v_flex()
            .id("explorer-tree-viewport")
            .flex_1()
            .min_h_0()
            .w_full()
            .px(px(ROW_INLINE_INSET))
            .role(Role::Tree)
            .aria_label(TREE_ARIA_LABEL)
            .child(tree)
            .into_any_element()
    }

    /// 树体为空时的空态。分支照真机的三段文案（`file-explorer-tree.tsx:1531-1551`）：
    /// 没有工作区 →「未打开文件夹」；有查询无命中 →「没有匹配的文件」；否则 →「文件夹为空」。
    fn empty_no_rows(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.root.as_os_str().is_empty() {
            // 真机的动作按钮是「打开文件夹」（`welcome.openFolder`，`locale.ts:8740`）。
            // 打开目录需要一个系统目录选择器、而且换工作区根要重建整个外壳布局，
            // 本模块拿不到这条通路，所以按"宁可禁用也不画假按钮"的约定渲染成**禁用态**并登记。
            return self.empty(SharedString::from(NO_FOLDER_OPEN), Some((OPEN_FOLDER, true)), cx);
        }

        let message = if self.query.is_empty() {
            FOLDER_IS_EMPTY
        } else {
            NO_MATCHING_FILES
        };
        self.empty(SharedString::from(message), None, cx)
    }

    /// 加载失败：显示 Core 错误码 + 「重试」（真的会重新取快照）。
    fn empty_failed(&self, error: String, cx: &mut Context<Self>) -> AnyElement {
        self.empty(SharedString::from(error), Some((RETRY, false)), cx)
    }

    /// 空态的公共骨架，照 `ui/empty.tsx:111-130` 的 `EmptyState`：
    /// `Empty`（居中、`gap-2`、`p-3`、`rounded-lg`、虚线边）+ `EmptyDescription` 承载文案
    /// + 可选 `EmptyContent` 里的一个 `xs` 按钮（高 24、`px-1.5`，`ui/button.tsx:23`）。
    fn empty(
        &self,
        message: SharedString,
        action: Option<(&'static str, bool)>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut empty = Empty::new()
            .header(EmptyHeader::new().description(
                EmptyDescription::new()
                    .text_size(px(ROW_TEXT_SIZE))
                    .child(message),
            ));

        if let Some((label, disabled)) = action {
            let button = Button::new("explorer-empty-action")
                .label(label)
                .h(px(HEADER_BUTTON_SIZE))
                .px(px(6.));
            let button = if disabled {
                button.disabled(true)
            } else {
                button.on_click(cx.listener(
                    |this: &mut Self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                        this.load(cx);
                    },
                ))
            };
            empty = empty.content(EmptyContent::new().child(button));
        }

        // `Empty` 的根是 `v_flex().flex_1()`，会吃掉剩余高度并把内容居中
        // （`gpui-component-0.6.6/src/empty.rs:63-82`）；外面这层必须是 flex 容器
        // （`v_flex`），否则 `flex_1` 不生效、空态会贴在顶部。
        v_flex()
            .size_full()
            .child(empty)
            .into_any_element()
    }
}

/// 头部的一个图标按钮。
///
/// `on_click` 为 `None` 就是禁用态：走 gpui-kit 的 `Disableable`
/// （`gpui-component-0.6.6/src/button/button.rs:503-508`；禁用后不响应指针，
/// 与 `ui/button.tsx:9` 的 `disabled:pointer-events-none disabled:opacity-50` 同语义）。
fn header_button(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
) -> Button {
    let button = Button::new(id)
        .ghost()
        .icon(icon)
        .tab_stop(false)
        .w(px(HEADER_BUTTON_SIZE))
        .h(px(HEADER_BUTTON_SIZE))
        .rounded(px(HEADER_BUTTON_RADIUS))
        .tooltip(label)
        .accessibility_label(label);

    match on_click {
        Some(handler) => button.on_click(handler),
        None => button.disabled(true),
    }
}

/// 工作区根那一行显示的名字：根目录名；根是盘符根（`D:\`）这类没有文件名的情况退回整条路径。
fn root_label(root: &Path) -> String {
    root.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| root.to_string_lossy().to_string())
}

/// 把工作区相对路径（Core 约定用 `/` 分隔，`rust/lithe-core/src/project/files.rs:576`）拼成绝对路径。
///
/// 逐段 `join` 而不是整串 `join`：这样结果用的是平台自己的分隔符，空段与 `..` 的行为也更可控。
fn join_relative(root: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .filter(|segment| !segment.is_empty())
        .fold(root.to_path_buf(), |path, segment| path.join(segment))
}

impl Render for Explorer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 三个 `render_*` 都返回 `AnyElement`（具体类型，不借用 self）：edition 2024 下
        // `-> impl IntoElement` 会捕获作用域内的生命周期，返回 `&self` 派生的类型会和下面
        // `cx.theme()` 的共享借用打架（E0502）。上一轮实现踩过同一个坑。
        let header = self.render_header(cx);
        let search_row = self.render_search_row(cx);
        let body = self.render_body(cx);

        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(header)
            .children(search_row)
            .child(body)
    }
}

// ---------------------------------------------------------------------------
// 图标真源：全量 Lucide 目录（`gpui_kit::assets::IconName`，1830 个变体）
//
// | 用途 | Windows 真机的字形 | 本项目用的字形 | 说明 |
// | --- | --- | --- | --- |
// | 目录（折叠） | 主题图标集 folder | `FolderClosed`（`folder-closed.svg`） | 同语义 |
// | 目录（展开） | 主题图标集 folder-open | `FolderOpen`（`folder-open.svg`） | 同语义 |
// | 展开/折叠箭头 | `chevron-down` / `chevron-right` | `ChevronDown` / `ChevronRight` | 同语义（12px） |
// | 源码文件 | Seti 等按语言区分的图标 | `FileCode`（`file-code.svg`） | 真实字形 |
// | JSON / JSONC | Seti 的 json 图标 | `FileBraces`（`file-braces.svg`） | **目录里没有 `file-json.svg`**，取语义最近的 `file-braces` |
// | Markdown / 文本 | Seti 的 markdown 图标 | `FileText`（`file-text.svg`） | 真实字形 |
// | 配置（toml/yaml/ini/conf） | Seti 的配置文件图标 | `Settings`（`settings.svg`） | 真实字形 |
// | 依赖锁 / 包文件 | Seti 的 lock 图标 | `Package`（`package.svg`） | 真实字形 |
// | 脚本（sh/ps1/bat） | 终端图标 | `Terminal`（`terminal.svg`） | 真实字形 |
// | 图片/图标文件 | 主题图标集 image | `Image`（`image.svg`） | 真实字形 |
// | 其它文件 | 主题图标集 file | `File`（`file.svg`） | 同语义 |
// | 搜索按钮 | `Search` | `Search`（`search.svg`） | 同语义 |
// | 清空搜索按钮 | `X` | `X`（`x.svg`） | 同语义 |
// | 偏好按钮 | `Preferences`（自定义齿轮） | `Settings`（`settings.svg`） | Windows 的 `Preferences` 是自有字形，Lucide 里取齿轮 |
// | 加载转圈 | 自绘 CSS 圆环 | `Spinner` 默认的 `Loader`（`loader.svg`） | 组件自带 |
//
// 未实现清单（本轮**不做**的，逐条写明卡在哪）：
//
// 1. **Git 状态装饰**（文件行状态字母 + 目录/文件名染色）：数据来自 Core 的 `git.status`，
//    本模块只调 `workspace.snapshot`。这不是 API 缺口（上一轮的 `panels.rs::load_git_marks`
//    已经跑通过，契约见 `shared/contracts/rust-core-api.md`），缺的是这里还没接第二路后台调用。
// 2. **右键菜单**（打开 / 新建文件 / 新建文件夹 / 重命名 / 删除 / 复制路径 / 在资源管理器中显示 /
//    刷新 / 全部折叠）：`Tree::context_menu` 现成可用（`gpui-component-0.6.6/src/tree.rs:54-62`），
//    卡住的是**动作落点** —— 新建/重命名/删除在 Windows 走 `#[tauri::command]`
//    （`move_file` / `rename_file` 等，见 `gpui/research/windows/02-editor-sidebar.md` §4.2），
//    Core 契约里没有文件写命令；「在资源管理器中显示」要起 `explorer.exe`，属于平台层，
//    不该由这个 UI 模块做。所以「刷新」以公开方法 [`Explorer::refresh`] 的形式提供。
// 3. **拖拽移动 / 拖到编辑区**：需要拖拽 payload + 落盘命令，同第 2 条。
// 4. **内联重命名 / 内联新建输入框**：依赖第 2 条的命令通路。
// 5. **缩进参考线**：真机是每行一套绝对定位的 1px 竖线（`file-explorer-tree.css:170-200`），
//    gpui 的 `Tree` 行没有任何"层级线"支持，要自己在行里画 `w(px(1.))` 的竖线；
//    而且真机规定它默认 `opacity: 0`、只有悬停/聚焦时才到 0.9（同文件 `:177-190`），
//    缺它不影响常驻观感，本轮不做。
// 6. **紧凑文件夹链**（`compactFoldersInFileTree: true` 默认开启，
//    `visible-file-tree-rows.ts:168-184` 把单子目录链合并成 `a.b.c`）：纯数据层算法不难，
//    但会改变行的 path↔id 映射（一行覆盖多级目录），要和展开状态一起改，本轮不做。
// 7. **根行自动定位**（`autoRevealActiveFileInFileTree: true`）：`TreeState::reveal_item`
//    现成可用（`gpui-base-0.6.6/src/tree.rs:268-278`），缺的是"当前活动文件"这个输入 ——
//    外壳还没把编辑区状态回传给侧栏（本模块只记自己点开过的那一个）。
// 8. **两档选中底色 / 边框 72% / 胶囊阴影**：真机的选中底色分"树未聚焦 `--border` / 聚焦
//    `--selected`"（`file-explorer-tree.css:69-78`），gpui 的 `Tree` 只给一档 `list_active`；
//    头部下边框的 72% 透明度没有对应 token（取了 `theme.border`）；
//    加载胶囊的 `shadow-popover` 与 `backdrop-blur-sm` 也没有对应 token。
// 9. **树容器级键盘**（`Mod+F` / `/` 开搜索、`Mod+C/X/V`、`F2`、`Home/End`）：`Tree` 内建
//    `↑↓←→` 与 `Enter`（`gpui-base-0.6.6/src/tree.rs:19-27,350-411`），其余要在外层
//    `track_focus` + 自己绑 action，本轮只保证内建那几条可用。
// 10. **偏好下拉菜单**：见 [`Explorer::render_header`] 的注释（渲染成禁用按钮）。
// 11. **搜索浮层**：见 [`Explorer::render_search_row`] 的注释（改成一整行内联输入框）。
// 12. **空态的「打开文件夹」按钮**：渲染成禁用态（原因见 [`Explorer::empty_no_rows`]）。
