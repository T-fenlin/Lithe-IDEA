//! 项目树的视图层：`Explorer` 实体、`LoadState` 与各区域渲染。
//!
//! 建树与取数据在 `model.rs`（那个文件不依赖 gpui UI 组件）；crate 的模块文档（规格出处、
//! 图标对照、未实现清单）在 `lib.rs`。本文件只放宽 `Explorer` 为 `pub`（由 `lib.rs`
//! re-export），其余项都留在模块内。
//!
//! **现状（2026-09-27 按维护者要求删除）**：侧栏顶部**不再有**「项目」头部行（原 32px）与
//! 树内搜索行（原 28px），本模块渲染出的第一个元素就是文件树本身；搜索入口改由左活动栏的
//! 「搜索」项承担（`lithe-gpui/crates/workbench/src/activity_bar.rs`）。这两条与真机的差异
//! 原先登记在 `lib.rs` 未实现清单的第 10 条（偏好下拉菜单）与第 11 条（搜索浮层），随实现一并
//! 删除。搜索状态（`search` / `search_open` / `query`）与 `toggle_search` / `clear_search` /
//! `set_query` / `rebuild` 的过滤通路**保留**，等左活动栏与树内 `Mod+F` / `/` 键位接上，
//! 保留理由写在各声明处。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{TreeState, h_flex, v_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tree::{Tree, TreeEvent};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, ClipboardItem, Context, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, Role, ScrollStrategy,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, WeakEntity, Window,
    div, relative, rems,
};

use lithe_gpui_shared::design::{ROW_LINE_HEIGHT, ROW_RADIUS};
use lithe_gpui_shared::icons::{FileIcon, idea, idea_icon_image};
use lithe_gpui_shared::tr;

use crate::model::{
    BuiltTree, GitTreeStatus, RENDER_LIMIT, ROOT_ID, RowKind, Snapshot, TreeNode, build_tree_items,
    icon_for_file, load_git_status, load_snapshot,
};

// ---------------------------------------------------------------------------
// 文案：全部走 `lithe_gpui_shared::tr`（key 逐字取 `crates/shared/locales/source/locale.ts`
// 生成的 locale，不自己编中文）。下面每一条注释保留原文与成 key 的对应关系。
// ---------------------------------------------------------------------------

/// `fileExplorer.searchFiles` → 「搜索文件」（`locale.ts:7446`）：树内搜索输入框的占位文案。
///
/// 原先它同时是头部搜索按钮的 tooltip；头部行与树内搜索行已在 **2026-09-27 按维护者要求**删除，
/// 现在只剩 [`Explorer::new`] 里 `InputState::set_placeholder` 这一处消费。
const SEARCH_FILES_KEY: &str = "lithe.fileExplorer.searchFiles";
/// `fileExplorer.ariaLabel` → 「文件资源管理器」（`locale.ts:7447`；调研文档 §5.4 漏了这一条）。
const TREE_ARIA_LABEL_KEY: &str = "lithe.fileExplorer.ariaLabel";
/// `quickOpen.loadingFiles` → 「正在加载文件」（`locale.ts:7835`）。
const LOADING_FILES_KEY: &str = "lithe.quickOpen.loadingFiles";
/// `ui.retry` → 「重试」（`locale.ts:4628`）。
const RETRY_KEY: &str = "lithe.ui.retry";
/// `fileExplorer.noFolderOpen` → 「未打开文件夹」（`locale.ts:7458`）。
const NO_FOLDER_OPEN_KEY: &str = "lithe.fileExplorer.noFolderOpen";
/// `titleProject.openFolder` → 「打开文件夹」（`locale.ts:8740` 所属的
/// `welcome.openFolder` 同值；这里取标题栏项目菜单里的那一条）。
const OPEN_FOLDER_KEY: &str = "lithe.titleProject.openFolder";
/// `fileExplorer.noMatchingFiles` → 「没有匹配的文件」（`locale.ts:7460`）。
const NO_MATCHING_FILES_KEY: &str = "lithe.fileExplorer.noMatchingFiles";

// 右键菜单的五个动作：**全部复 catalog 既有键**（U24 零新增文案，除了编辑器那条
// 「Reveal in Project Tree」，见 `lithe-gpui/tools/extract-locale.mjs` 的 `GPUI_ONLY_KEYS`）。
/// `files.open` → 「打开」（`locale.ts:7490`）。
const OPEN_KEY: &str = "lithe.files.open";
/// `files.reveal` → 「在资源管理器中显示」（`locale.ts:7522`）：真源 macOS 是
/// `Show in Finder` / `Show Project in Finder`（`ProjectSidebarView.swift:605,620,734`），
/// 同一个动作。
const REVEAL_KEY: &str = "lithe.files.reveal";
/// `files.copyPath` → 「复制路径」（`locale.ts:7498`）。
const COPY_PATH_KEY: &str = "lithe.files.copyPath";
/// `files.copyRelativePath` → 「复制相对路径」（`locale.ts:7506`）。
const COPY_RELATIVE_PATH_KEY: &str = "lithe.files.copyRelativePath";
/// `files.refresh` → 「刷新」（`locale.ts:7564`）：真源目录行右键菜单末尾那条
/// （`ProjectSidebarView.swift:642-647`）。
const REFRESH_KEY: &str = "lithe.files.refresh";

// ---------------------------------------------------------------------------
// 度量：一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// 全部取自界面规格真源（macOS 的 `ProjectSidebarView.swift` 与它引用的 `LitheTheme.Metrics`）。
// rem base = 主题字号 16px，所以 helper 后缀 `N` = `N × 4px`。
//
// | 规格 | 值 | 用到的 helper | 出处 |
// | --- | --- | --- | --- |
// | 树行高 | 24 | `h_6()` | `ProjectSidebarView.swift:493` 的 `frame(height: rowHeight)` |
// | 状态字母 | 9 | `rems(9. / 16.)` | 同文件 `:531` 的 `.font(.system(size: 9, ...))` |
// | 行内图标 | 16 | `size_4()` | 同文件 `:481,521` |
//
// ⚠️ `line_height` 没有 rem 档位 helper（`gpui-pre-0.3.7/src/styled.rs` 只有取值形式），
// 需要定长行高时用 helper 底层的 `rems(P / 16.)`；倍数行高仍走 `relative(..)`
// （见 [`ROW_LINE_HEIGHT`]）。

/// 树的**设计常量**（不是测量几何）：逐层缩进的基准与步长。
///
/// 真源 macOS 一行公式：`padding(.leading, CGFloat(depth * 14 + 8))`
/// （`ProjectSidebarView.swift:490,536`）⇒ 基准 8、步长 14。它与旧实现（Windows 的
/// `10 + depth × 16`）不同，本任务按"规格真源是 macOS"改为真源那一对值。
/// 两者都不在 gpui 的 rem 档位上（档位里 8 有、14 没有），正是该写 helper 底层
/// `rems(P / 16.)` 的场合：它们是**设计常量**、不是测量几何，所以 rem 化后缩进随界面字号缩放。
const BASE_INDENT: f32 = 8.;
const INDENT_STEP: f32 = 14.;
// 行圆角与行高倍数的**共同 owner 不在这里**：`ROW_RADIUS`（4）与 `ROW_LINE_HEIGHT`（1.35）
// 同时被 `git` 的更改列表行消费，两条原先各自硬写一份字面相同的数字（值相同纯属巧合）。
// 现按《设计指南》§「对齐细节」（`design-guides.md:188`）"相等关系来自同一个 spacing token"
// 收敛到 `lithe_gpui_shared::design`，本文件只 import。
//
// 消费方式仍是 rem：`div` 上的 `Styled::rounded` 吃 `AbsoluteLength`，调用点写
// `rems(ROW_RADIUS / 16.)`（rem base = 16px）——「不在 gpui 档位上」不是保留 `px(...)` 的理由；
// 也不能从主题读（`ThemeConfig.radius` 是 `Option<usize>`，装不下 Lithe 的 `--radius × k` 阶梯）。

// ---------------------------------------------------------------------------
// 宿主钩子
// ---------------------------------------------------------------------------

/// 「在资源管理器中显示」的宿主钩子（Windows = `explorer.exe /select,`）。
///
/// 形状照 `lithe-gpui/crates/editor/src/editor_view.rs:260-265` 的 `TabMenuHostActions`：
/// **低层 crate 定义注入点，外壳填实现**。探索器自己不起进程 —— 起系统文件管理器属平台层，
/// 不该由这个 UI 模块做（`lib.rs` 未实现清单第 2 条登记过同一条约束）。
pub type RevealInFileManagerHost = Arc<dyn Fn(&Path, &mut App) + 'static>;

// ---------------------------------------------------------------------------
// 右键菜单：条目是纯数据，落点在 `Explorer::run_row_action`
// ---------------------------------------------------------------------------

/// 右键菜单里的一条动作。
///
/// **每个变体都必须有真实落点**：落点写在 [`Explorer::run_row_action`] 的 `match` 里，
/// 而那条 `match` 是穷尽的 —— 于是"画了不接"在编译期就不可能发生。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RowAction {
    /// 打开文件（走既有的 `on_open` 回调，与单击同一条路）。
    Open,
    /// 在系统文件管理器里定位这一行（宿主钩子）。
    RevealInFileManager,
    /// 复制绝对路径。
    CopyPath,
    /// 复制工作区相对路径（工作区根那一行按真源拷 `.`）。
    CopyRelativePath,
    /// 重新取一次 `workspace.snapshot`（+ `git.status`）。
    Refresh,
}

/// 某一行的右键菜单条目；`None` 是一条分隔线。
///
/// 真源：`macos/Sources/Lithe/Views/Workspace/ProjectSidebarView.swift` 的
/// `directoryContextMenuItems`（`:574-648`）与 `fileContextMenuItems`（`:704-745`）。
/// **只画本版真有落点的项**（维护者口径「操作行只列真能执行的」）：
///
/// | 真源条目 | 本版 | 为什么 |
/// | --- | --- | --- |
/// | `Open`（`:706`） | ✅ [`RowAction::Open`] | 与单击同一条 `on_open` 通路 |
/// | `Show in Finder` / `Show Project in Finder`（`:605`、`:620`、`:734`） | ✅ [`RowAction::RevealInFileManager`] | 外壳登记的宿主钩子（`workbench` 已有的 `reveal_in_file_manager`） |
/// | `Copy Path` / `Copy Relative Path`（`:611`、`:623`、`:737`、`:740`） | ✅ | 写剪贴板，纯本地 |
/// | `Refresh`（`:644`，目录行末尾） | ✅ [`RowAction::Refresh`] | 真的重取快照（不是画死的按钮） |
/// | `New File…` / `New Directory…`（`:579`、`:582`） | ❌ 不画 | Core 没有文件写命令（**C34**） |
/// | `Duplicate` / `Rename…` / `Move to Trash`（`:630`、`:633`、`:636`、`:721`、`:724`、`:730`） | ❌ 不画 | 同上：重命名/删除/移动在 macOS 是**宿主端口**，gpui 侧没有对应实现也不许自己写文件系统语义 |
/// | `Show Git Diff`（`:591`、`:713`） | ❌ 不画 | 差异视图族是 **U15**，本模块没有落地通路 |
/// | `Local History…` / `Show Project Local History…`（`:608`、`:727`） | ❌ 不画 | 本地历史记录触发点契约是 **C35**，gpui 零调用点 |
/// | `Mark Target As`（`:599`，六个子项） | ❌ 不画 | 目录标记（sources root / excluded…）在 gpui 侧没有状态、也没有消费者（图标分派同样没接） |
///
/// 目录行与根行**形状相同**：真源对 `depth == 0` 只是把文案从 `Show in Finder` 换成
/// `Show Project in Finder`（同一个动作、同一个落点），而本侧用的是 catalog 里那条通用文案
/// `lithe.files.reveal`（「在资源管理器中显示」/ "Reveal in File Explorer"），所以不需要分叉。
pub(crate) fn row_menu_entries(kind: RowKind) -> Vec<Option<RowAction>> {
    match kind {
        // 空目录占位行是禁用行：`gpui-component` 的 `Tree::context_menu` 会跳过禁用项
        // （`gpui-component-0.7.0/src/tree.rs:101-105` 的 `.filter(|entry| !entry.is_disabled())`），
        // 这里返回空表只是把同一件事说清楚。
        RowKind::EmptyPlaceholder => Vec::new(),
        RowKind::File => vec![
            Some(RowAction::Open),
            None,
            Some(RowAction::RevealInFileManager),
            Some(RowAction::CopyPath),
            Some(RowAction::CopyRelativePath),
        ],
        RowKind::Directory => vec![
            Some(RowAction::RevealInFileManager),
            Some(RowAction::CopyPath),
            Some(RowAction::CopyRelativePath),
            None,
            Some(RowAction::Refresh),
        ],
    }
}

/// 菜单条目的文案（**每个动作一个既有键**，没有硬编码中英文）。
fn row_action_label(action: RowAction) -> SharedString {
    match action {
        RowAction::Open => tr(OPEN_KEY),
        RowAction::RevealInFileManager => tr(REVEAL_KEY),
        RowAction::CopyPath => tr(COPY_PATH_KEY),
        RowAction::CopyRelativePath => tr(COPY_RELATIVE_PATH_KEY),
        RowAction::Refresh => tr(REFRESH_KEY),
    }
}

// ---------------------------------------------------------------------------
// 状态：树体的加载状态（本 Entity 自己保留）
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 组件
// ---------------------------------------------------------------------------

/// 侧栏的项目树（资源管理器）。
///
/// 对外只暴露 [`Explorer::new`]、[`Explorer::refresh`]、[`Explorer::reveal`]、
/// [`Explorer::set_reveal_in_file_manager_host`] 与 `Render`；字段全部私有。
/// 点击文件时通过构造时传入的 `on_open` 回调把**绝对路径**抛给外壳（主代理接到编辑区）；
/// 点击目录只做展开/折叠 —— 由 `Tree` 自己在 `mouse_down` 里处理
/// （`gpui-base-0.7.0/src/tree.rs:439-456`）。
pub struct Explorer {
    /// 工作区根（外壳给出；Core 不回传它，所以自己留着拼绝对路径）。
    root: PathBuf,
    /// 最近一次成功加载的快照（`data.root` 树 + `data.files` 诊断用）。
    ///
    /// **U24：建树用它、不再用扁平的 `data.files`**，因为只有 `root` 带得动空目录。
    snapshot: Option<Snapshot>,
    /// 最近一次建树的**行数**与是否被 [`RENDER_LIMIT`] 截断（诊断用）。
    rows: usize,
    truncated: bool,
    /// 加载状态。
    state: LoadState,
    /// 树的交互状态（选中、滚动、展开后的扁平表）。
    tree: Entity<TreeState>,
    /// 用户展开过的目录 id（`dir:<相对路径>`）。
    ///
    /// 从 `TreeEvent::Expanded/Collapsed` 攒出来
    /// （`gpui-base-0.7.0/src/tree.rs:93-96,346-364`），[`Explorer::reveal`] 也会往里写祖先目录。
    expanded: BTreeSet<SharedString>,
    /// 最后一次点击打开 / 定位的文件（相对路径），用来给那一行加底色。
    active: Option<SharedString>,
    /// 多选状态（选择集 + Shift 锚点 + 焦点），真源 `ProjectTreeSelection`。
    ///
    /// ⚠️ 它**不**喂 `TreeState`：上游只有单个 `selected_ix`
    /// （`gpui-base-0.7.0/src/tree.rs:195`），而 `Tree::new` 的 wrapper 又会在 `render_item`
    /// 返回后覆盖调用方设的 `.selected(..)`（`gpui-component-0.7.0/src/tree.rs:89-92`），
    /// 所以行的选中态由本文件**自己画**（见 `render_tree` 里的选中底色）。
    selection: crate::selection::ProjectTreeSelection,
    /// 树的**焦点句柄**：`⌘A` 这类键要先让树拿到焦点才生效。
    ///
    /// 真源的规矩是"**树内发生过指针交互之后**才接管快捷键"
    /// （`Platform/MacOS/UI/ProjectTreeKeyboardCommands.swift:4,44-62`），所以行被点击时
    /// 主动 `window.focus(..)`（见 `render_tree` 的点击处理）。
    focus_handle: gpui_kit::FocusHandle,
    /// `git.status` 投影到树行上的状态装饰（工作区不是 git 仓库时是空的）。
    decorations: GitTreeStatus,
    /// 树内搜索的输入状态（placeholder 在 [`Explorer::new`] 里设置）。
    ///
    /// ⚠️ **保留项**：承载它的树内搜索行已在 **2026-09-27 按维护者要求**删除（见文件顶部模块
    /// 文档），搜索入口改由左活动栏的「搜索」项承担。它现在仍被生产路径读写（`new` 里的
    /// placeholder 设置与 `InputEvent` 订阅），`#[allow(dead_code)]` 是**显式的保留说明**：
    /// 后续把左活动栏那一项与树内 `Mod+F` / `/` 键位接到这里时，整个搜索簇（本字段 +
    /// [`Explorer::toggle_search`] / [`Explorer::clear_search`] / [`Explorer::set_query`]）
    /// 一起复活，别让任何一次局部调整把它判成死代码删掉。
    #[allow(dead_code)]
    search: Entity<InputState>,
    /// 搜索输入框是否展开（真机是 `SidebarSearchPopover` 的 `open`，`file-explorer-tree.tsx:1321`）。
    ///
    /// ⚠️ **保留项**：唯一读它的渲染函数（原来的 `render_search_row`）随搜索行删除，现在生产路径上
    /// 只剩 [`Explorer::toggle_search`] 在读写它，所以 `#[allow(dead_code)]` 是必须的显式保留说明
    /// （删除原因是 **2026-09-27 按维护者要求**去掉了那两行；见文件顶部模块文档）。
    #[allow(dead_code)]
    search_open: bool,
    /// 当前查询词（小写比较在 `model::build_tree_items` 里做）。
    query: SharedString,
    /// 点击文件时往外抛的回调。
    ///
    /// 存 `Rc` 而不是 `Box`：渲染闭包要求 `'static`，没法借用 `&self.on_open`。
    on_open: Rc<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>,
    /// 「在资源管理器中显示」的宿主钩子（由外壳在 [`Explorer::new`] 之后登记）。
    ///
    /// `None` 时那条菜单项不是"画了不接"：它会打一行 `S1_EXPLORER ... result=no-host` 诊断，
    /// 与编辑区「在资源管理器中显示」的 `S1_TAB_MENU run=reveal result=no-host`
    /// （`editor_view.rs:2456-2458`）同一口径。
    reveal_host: Option<RevealInFileManagerHost>,
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
        // ⚠️ placeholder 这一处是搜索簇**保留**下来的活代码之一：树内搜索行已在 **2026-09-27 按
        // 维护者要求**删除，所以这个 `InputState` 目前没有可见的输入框；等左活动栏的「搜索」项与
        // 树内 `Mod+F` / `/` 键位接上后复用（见 `search` 字段的注释）。
        let search = cx.new(|cx| InputState::new(window, cx));
        search.update(cx, |state, cx| {
            state.set_placeholder(tr(SEARCH_FILES_KEY), window, cx);
        });

        // 攒展开状态。`TreeEvent` 只有 `Expanded` / `Collapsed` 两个变体
        // （`gpui-base-0.7.0/src/tree.rs:93-96`），所以这里能直接取里面的 id。
        let tree_events = cx.subscribe(
            &tree,
            |this, _tree, event: &TreeEvent, cx: &mut Context<Self>| match event {
                TreeEvent::Expanded(id) => {
                    this.expanded.insert(id.clone());
                }
                TreeEvent::Collapsed(id) => {
                    this.expanded.remove(id);
                    // 折叠会让被折叠子树的行**不再可见**：选择集与可见集求交，锚点/焦点不可见就清掉
                    // （真源 `ProjectTreeSelection.retain`，`:51-56`）—— 否则"选中的东西看不见了，
                    // 批量操作却还作用在它身上"。
                    // `&**cx`：读实体要 `&App`，而这里是 `&mut Context<Self>`（同 `:350` 的取法）。
                    let visible = this.visible_paths(&**cx);
                    this.selection.retain(&visible);
                    cx.notify();
                }
            },
        );

        // 查询变化 → 重建树。`InputEvent` 见 `gpui-base-0.7.0/src/input/base/state.rs`。
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
            snapshot: None,
            rows: 0,
            truncated: false,
            state: LoadState::Loading,
            tree,
            expanded: BTreeSet::new(),
            active: None,
            selection: crate::selection::ProjectTreeSelection::default(),
            focus_handle: cx.focus_handle(),
            decorations: GitTreeStatus::default(),
            search,
            search_open: false,
            query: SharedString::default(),
            on_open: Rc::from(on_open),
            reveal_host: None,
            _tree_events: tree_events,
            _search_events: search_events,
        };
        // 根行默认展开一次（真源 macOS 的根行 `depth == 0` 且初态展开）。
        explorer.expanded.insert(SharedString::new_static(ROOT_ID));
        explorer.load(cx);
        explorer
    }

    /// 登记「在资源管理器中显示」的宿主钩子（由外壳在构造之后立刻调用）。
    ///
    /// 用 setter 而不是给 [`Explorer::new`] 再加一个参数：形状与
    /// `EditorPane::set_buffer_write_host` / `set_tab_menu_host_actions`
    /// （`editor_view.rs:1150-1158`）一致 —— 低层 crate 定形状，外壳填实现。
    pub fn set_reveal_in_file_manager_host(&mut self, host: RevealInFileManagerHost) {
        self.reveal_host = Some(host);
    }

    /// 重新取一次工作区快照（+ `git.status` 装饰）。
    ///
    /// 真机有**两处**「刷新」：侧栏头部那颗 `arrow.clockwise` 按钮
    /// （`ProjectSidebarView.swift:242-249`）与目录行右键菜单末尾那条
    /// （同文件 `:642-647`）。侧栏头部行在 gpui 侧已于 **2026-09-27 按维护者要求**删除
    /// （删的是 Windows 那一版「项目」头部行 + 搜索/清空/偏好三个按钮），所以本任务把刷新落在
    /// **右键菜单**那一处 —— 两处是同一个动作、同一个落点，菜单那一处在真源里也存在，
    /// 不需要为了它把头部行画回来（那会与"侧栏顶部就是文件树本身"直接冲突）。
    ///
    /// `window` 现在用不上（后台任务不需要窗口），保留参数是为了调用形态与 `new` 一致。
    pub fn refresh(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.load(cx);
    }

    /// 把**绝对路径**指向的那一项在树里选中并滚动到可见（真源「Reveal in Project Tree」）。
    ///
    /// 真源有两处入口：编辑区标签的右键菜单（`macos/Sources/Lithe/Views/Editor/EditorAreaView.swift:1206-1213`）
    /// 与侧栏头部那颗 `scope` 按钮（`ProjectSidebarView.swift:218-228`，文案
    /// "Reveal Active File in Project Tree"）。gpui 侧的侧栏头部行已删除，所以当前入口是
    /// 编辑区标签右键菜单（外壳登记 `TabMenuHostActions::reveal_in_tree`），
    /// 这一条落在**探索器自己的口径**上：复用既有的展开集合与树选中/滚动
    /// （`TreeState::reveal_item` + `set_selected_index`，`gpui-base-0.7.0/src/tree.rs:237-248,275-285`）。
    ///
    /// 路径不在工作区根之下时**什么都不做**，只打一行诊断 —— 与真源
    /// `canRevealInProjectTree`（`AppModel+FeatureState.swift:48`）同一个取舍。
    pub fn reveal(&mut self, path: &Path, cx: &mut Context<Self>) {
        let Some(relative) = relative_to_root(&self.root, path) else {
            eprintln!(
                "S1_EXPLORER reveal=outside-root root={} path={}",
                self.root.display(),
                path.display()
            );
            return;
        };

        // 祖先目录要先记进**我们自己的**展开集合：`TreeState::reveal_item` 只改它内部那份
        // 展开态（`expand_ancestors`），而本模块每次重建都会按 `self.expanded` 重新铺树，
        // 不记的话下一次重建（刷新 / 搜索 / 再次 reveal）就把祖先收回去、目标行随即消失。
        for ancestor in ancestor_dir_ids(&relative) {
            self.expanded.insert(SharedString::from(ancestor));
        }

        let id = SharedString::from(format!("file:{relative}"));
        self.active = Some(SharedString::from(relative.clone()));
        self.rebuild(cx);
        let mut found = false;
        self.tree.update(cx, |state, cx| {
            state.reveal_item(&id, ScrollStrategy::Center, cx);
            found = state.index_of(&id).is_some();
            let index = state.index_of(&id);
            state.set_selected_index(index, cx);
        });
        // `workspace.snapshot` 与树是两份时点不同的数据：文件可能是刚建/刚删的。
        // **不静默**（与同族诊断口径一致），但不报错、不假装选中了什么。
        eprintln!("S1_EXPLORER reveal path={relative} found={found}");
    }

    /// 取工作区快照 + `git.status` 装饰：**在后台线程调 Core**，回到前台才建树。
    ///
    /// `shared::core_client` 的 Core 调用是同步的（`lithe_core::execute_json`，
    /// `rust/lithe-core/src/lib.rs:25`），直接放在 UI 线程上
    /// 会把整棵树扫描（本仓库约 4.8 千个节点）挡在渲染前面，所以走 `cx.background_spawn`
    /// （`gpui-pre-0.3.7/src/app.rs`，`AppContext` trait 上的方法）再 `await` 回前台；
    /// 调用形态与上一轮跑通的 `workspace.rs` 加载一致。
    ///
    /// 两路 Core 调用**共用同一个后台任务**：先建树、再上色，少一次前台往返。
    /// `git.status` 失败**不影响树**（装饰退化成空），与真源同取舍：没装 git / 不是仓库时
    /// 项目树照常可用。
    fn load(&mut self, cx: &mut Context<Self>) {
        self.state = LoadState::Loading;
        cx.notify();

        // 工作区根为空：没有可加载的东西，直接进空态（真机的 `!rootFolderPath`）。
        if self.root.as_os_str().is_empty() {
            self.snapshot = None;
            self.decorations = GitTreeStatus::default();
            self.state = LoadState::Ready;
            self.rebuild(cx);
            return;
        }

        let root = self.root.clone();
        cx.spawn(async move |this, cx| {
            let snapshot_root = root.clone();
            let (snapshot, status) = cx
                .background_spawn(async move {
                    (
                        load_snapshot(&snapshot_root),
                        load_git_status(&root),
                    )
                })
                .await;

            let _ = this.update(cx, |this, cx| {
                match snapshot {
                    Ok(snapshot) => {
                        // 截断不静默：把工作区根、节点数、铺出来的行数与两份数据的分歧
                        // 打一行诊断（探针口径同 `workspace.rs` 的 `S1_VIEWPORT`）。
                        let nodes = snapshot.root.count();
                        let missing = snapshot.files_missing_from_root();
                        this.snapshot = Some(snapshot);
                        this.state = LoadState::Ready;
                        this.rebuild(cx);
                        eprintln!(
                            "S1_EXPLORER root={} nodes={} rows={} limit={} truncated={} files_not_in_root={missing}",
                            this.root.display(),
                            nodes,
                            this.rows,
                            RENDER_LIMIT,
                            this.truncated,
                        );
                    }
                    Err(error) => {
                        eprintln!("S1_EXPLORER root={} error={error}", this.root.display());
                        this.snapshot = None;
                        this.decorations = GitTreeStatus::default();
                        this.state = LoadState::Failed(error);
                        this.rebuild(cx);
                    }
                }
                match status {
                    Ok(status) => {
                        let (files, directories) = status.counts();
                        // 不是 git 仓库 / 没有任何变更时三样都是 0（`empty=true`）—— 这是
                        // **正常结果**，不是错误（`git/mod.rs:6575-6581` 返回 `ok: true`）。
                        let empty = status.is_empty();
                        this.decorations = status;
                        eprintln!(
                            "S1_EXPLORER_GIT root={} files={files} directories={directories} empty={empty}",
                            this.root.display()
                        );
                    }
                    Err(error) => {
                        this.decorations = GitTreeStatus::default();
                        eprintln!("S1_EXPLORER_GIT root={} error={error}", this.root.display());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 按当前的 `snapshot` / `expanded` / `query` 铺一次树。
    ///
    /// [`Explorer::rebuild`] 与 [`Explorer::visible_paths`] 共用它 —— 这样"真正铺进树里的行"与
    /// "选择用的可见顺序"**永远同源**（真源也是同一份 `visibleNodes`，
    /// `ProjectTreeSelection.swift:72-77`）。
    fn built_tree(&self) -> BuiltTree {
        let filter: Option<String> = if self.query.is_empty() {
            None
        } else {
            Some(self.query.to_lowercase())
        };
        let label = root_label(&self.root);
        // 还没加载完（或加载失败）时给一个空根，铺出来就是「根行 + 占位行」。
        let empty = TreeNode {
            path: String::new(),
            name: label.clone(),
            is_directory: true,
            children: Vec::new(),
        };
        let root = self
            .snapshot
            .as_ref()
            .map(|snapshot| &snapshot.root)
            .unwrap_or(&empty);
        build_tree_items(&label, root, &self.expanded, filter.as_deref())
    }

    /// **显示顺序**的可见路径表（真源 `ProjectTreeSelection.visibleNodes`，`:72-77`）。
    ///
    /// 区间选择按这个顺序切片，**不是**路径字典序；含根行（真源同样把根节点算作可见）。
    ///
    /// ⚠️ 取自 `TreeState` **自己的**扁平可见表：用 `entry(ix)` 从 0 向后逐项取到 `None`
    /// （`gpui-base-0.7.0/src/tree.rs:259`，那份表由 `rebuild_entries` 摊平，`:321-331`）。
    /// 它**就是**渲染那份顺序，所以不会出现"我算一份、树铺另一份"的偏差。
    ///
    /// 改前这里走 `built_tree().items`，而那是**嵌套**结构（`items = vec![根 TreeItem]`，
    /// 见 `model.rs:228-232`）⇒ 那份"可见顺序"里只有根一项，Shift 区间实际上是**失效**的。
    fn visible_paths(&self, cx: &App) -> Vec<String> {
        let state = self.tree.read(cx);
        let mut paths = Vec::new();
        let mut ix = 0;
        while let Some(entry) = state.entry(ix) {
            let (_, relative_path) = RowKind::parse(entry.item().id.as_ref());
            paths.push(relative_path);
            ix += 1;
        }
        paths
    }

    /// 按当前的 `snapshot` / `expanded` / `query` 重建树。
    ///
    /// `TreeState::set_items` 会**清空选中**（`gpui-base-0.7.0/src/tree.rs:221-226`），这是刻意的：
    /// 可见行被换掉之后旧行号已经没有意义（真机搜索时也是重新定位；[`Explorer::reveal`]
    /// 在重建之后自己把选中放回去）。
    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let built = self.built_tree();
        self.rows = built.rows;
        self.truncated = built.truncated;
        self.tree
            .update(cx, |state, cx| state.set_items(built.items, cx));
    }

    /// 真源 ⌘A（`ProjectTreeSelection.selectAll`，`ProjectTreeSelection.swift:59-70`）：
    /// 选**焦点行的同级兄弟**，不是整棵可见树（文件夹的子孙由 `covers` 覆盖）。
    ///
    /// 兄弟表由**显示树**算（[`crate::model::displayed_siblings`]）—— 父级不是文件系统父级，
    /// 因为压缩的 Java 包在显示上挂在并非文件系统父级的祖先下（真源 `:62-69`）。
    fn select_all_in_tree(
        &mut self,
        _: &crate::SelectAllInTree,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 没有快照（还没加载完 / 加载失败）时没有树可言。
        let Some(snapshot) = self.snapshot.as_ref() else {
            return;
        };
        let siblings = crate::model::displayed_siblings(&snapshot.root, self.selection.focused());
        self.selection.select_all_in_siblings(&siblings);
        cx.notify();
    }

    /// 当前多选集（**确定性有序**）。
    ///
    /// 给"批量操作要处理哪些条目"与 UI 测试读（真源里对应的就是
    /// `ProjectTreeSelection.paths`）。返回 owned `Vec` —— `selection.rs` 的内部类型不出公开 API。
    pub fn selected_paths(&self) -> Vec<String> {
        self.selection.paths().iter().cloned().collect()
    }

    /// 工作区里**有没有**任何可见项（决定画树还是画空态）。
    fn has_entries(&self) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|snapshot| !snapshot.root.children.is_empty())
    }

    /// 写入查询词并重建树（`InputEvent::Change` 与「清除搜索」都走这里）。
    ///
    /// ⚠️ **保留项**：搜索入口（头部行的搜索按钮、树内搜索行）已在 **2026-09-27 按维护者要求**
    /// 删除，但这条通路本身还是活代码 —— [`Explorer::new`] 里 `InputEvent::Change` 的订阅仍会调它。
    /// 保留给左活动栏的「搜索」项与树内 `Mod+F` / `/` 键位接上；`#[allow(dead_code)]` 是显式的
    /// 保留说明（同 `search` 字段的注释：整个搜索簇一起复活，别被逐项判成死代码删掉）。
    #[allow(dead_code)]
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
    /// 收起时清空查询：真机的 `Escape` 也是"关闭搜索"，留着过滤词会让下次打开看到一棵
    /// "少了东西"的树。
    ///
    /// ⚠️ **暂时不可达**：唯一调用点（头部行的搜索按钮）已在 **2026-09-27 按维护者要求**删除，
    /// 所以这里必须 `#[allow(dead_code)]`。保留它是因为左活动栏「搜索」项与树内 `Mod+F` / `/`
    /// 键位接上时，这个开关就是入口；维护者明确要求不要为了让 warning 消失而删掉搜索状态。
    #[allow(dead_code)]
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

    /// 清空查询并重建（对应 `search.clear`「清除搜索」按钮；那个按钮本身已随头部行在
    /// **2026-09-27 按维护者要求**删除）。
    ///
    /// ⚠️ **暂时不可达**：现在只有 [`Explorer::toggle_search`] 调它，而那个入口本身也暂时不可达
    /// （原因见那里），所以这里必须 `#[allow(dead_code)]`；保留它是为了接上搜索入口后直接复用
    /// 「Escape / 关闭搜索要连过滤词一起清掉」这条语义。
    ///
    /// ⚠️ **不能只靠 `InputEvent::Change`**：`InputState::set_value` 内部把 `emit_events` 置成
    /// `false`，**不会**发 `Change` 事件，所以这里必须自己调 [`Explorer::set_query`]。
    #[allow(dead_code)]
    fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let search = self.search.clone();
        search.update(cx, |state, cx| {
            state.set_value(SharedString::default(), window, cx);
        });
        self.set_query(SharedString::default(), cx);
    }

    /// 执行一条右键菜单动作。**这是每个菜单条目的唯一落点**
    /// （`match` 穷尽 = 编译期保证没有"画了不接"的条目）。
    fn run_row_action(
        &mut self,
        action: RowAction,
        relative: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            RowAction::Open => {
                let path = join_relative(&self.root, relative);
                (self.on_open.clone())(path, window, cx);
                self.active = Some(SharedString::from(relative.to_string()));
                cx.notify();
            }
            RowAction::RevealInFileManager => {
                let path = join_relative(&self.root, relative);
                match self.reveal_host.clone() {
                    Some(host) => {
                        host(&path, cx);
                        eprintln!("S1_EXPLORER reveal_in_file_manager path={}", path.display());
                    }
                    // 外壳没登记钩子：**不静默**（与编辑区同一条诊断口径）。
                    None => eprintln!(
                        "S1_EXPLORER reveal_in_file_manager result=no-host path={}",
                        path.display()
                    ),
                }
            }
            RowAction::CopyPath => {
                let path = join_relative(&self.root, relative);
                let text = path.to_string_lossy().to_string();
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                eprintln!("S1_EXPLORER copyPath value={text}");
            }
            RowAction::CopyRelativePath => {
                // 工作区根的相对路径在 Core 的约定里是空串，真源对这种情况拷 `.`
                // （`macos/Sources/Lithe/Models/AppModel/AppModel.swift:1067` 的
                // `relativeValue.isEmpty ? "." : relativeValue`），逐字照做。
                let text = if relative.is_empty() {
                    ".".to_string()
                } else {
                    relative.to_string()
                };
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                eprintln!("S1_EXPLORER copyRelativePath value={text}");
            }
            RowAction::Refresh => self.load(cx),
        }
    }

    // ---- 渲染 ----

    // ⚠️ **2026-09-27 按维护者要求删除**：这里原有 `render_header`（「项目」标题 32px 头部行：
    // 标题 + 搜索按钮 + 清空按钮 + 偏好按钮）与 `render_search_row`（28px 树内搜索行：内联
    // 一整行 `Input`）两个函数，以及 [`Explorer::new`] 之外的调用点、只服务它们的辅助函数
    // `header_button` 与常量 `TITLE_KEY` / `CLEAR_SEARCH_KEY` / `PREFERENCES_KEY` /
    // `HEADER_BUTTON_RADIUS` / `TITLE_LINE_HEIGHT` / `SEARCH_INPUT_RADIUS`、`rem_px`。
    // 删掉它们是为了让侧栏顶部就是文件树本身（对齐 IntelliJ IDEA 的观感），搜索入口改由左活动栏
    // 的「搜索」项承担。这两条与 Windows 真机的差异原先登记在 `lib.rs` 未实现清单第 10 条
    // （偏好下拉菜单）与第 11 条（搜索浮层），随实现一并删除；搜索状态与查询入口按维护者要求保留，
    // 理由写在字段与方法的注释里。图标、颜色、主题一律没动。

    /// 主体：加载中的胶囊 +（树体 / 空态 / 失败态）。
    fn render_body(&self, cx: &mut Context<Self>) -> AnyElement {
        // 首次加载时真机**不挂载树**，只显示顶部居中的加载胶囊
        // （`ProjectSidebarView.swift:237-241` 的 `ProgressView` 分支是同一个意思）；
        // 已有数据时刷新则保留树体、胶囊浮在上面（真机靠一层 `absolute inset-0` 做同一件事）。
        let content: AnyElement = if self.state.is_loading() && !self.has_entries() {
            div().flex_1().into_any_element()
        } else {
            match &self.state {
                LoadState::Failed(error) => self.empty_failed(error.clone(), cx),
                _ if !self.has_entries() => self.empty_no_rows(cx),
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
                        .top_3()
                        .left_0()
                        .right_0()
                        .justify_center()
                        .child(
                            // 真机胶囊：`rounded-full border border-border/60 bg-surface/92
                            // px-3 py-1.5 shadow-popover backdrop-blur-sm`
                            // （`file-explorer-pane.tsx:55-56`，Microsoft 版真机；macOS 版是
                            // `ProgressView` + `.help("Refreshing project")`）。`bg-surface/92`
                            // 取 `theme.muted`；阴影与背景模糊没有对应 token，不画
                            // （登记在未实现清单里）。
                            h_flex()
                                .gap_2()
                                .px_3()
                                .py_1p5()
                                .rounded_full()
                                .bg(cx.theme().muted)
                                .border_1()
                                .border_color(cx.theme().border)
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                // 转圈图标 16 + 文案 13（→ `text_sm()` 14）+ 间距 8。
                                // `Spinner` 默认 `Size::Medium`
                                // → 16px（`gpui-component-0.7.0/src/spinner.rs`），
                                // 自带 `with_animation` 常转。
                                .child(Spinner::new().color(cx.theme().muted_foreground))
                                .child(tr(LOADING_FILES_KEY)),
                        ),
                )
            })
            .child(content)
            .into_any_element()
    }

    /// 树体。
    ///
    /// - 外面套 `px_1p5()`（6）：真机给每一行加 `padding(.horizontal, projectTreeContentHorizontalInset)`
    ///   （`ProjectSidebarView.swift:504,552`），行底色因此左右各缩 6px；`uniform_list` 的行不能加
    ///   外边距，等价做法是把整棵树缩进 6px。所以行内的左内边距只需真源那一份
    ///   `depth × 14 + 8`（同文件 `:490,536`）。
    /// - 行内容按真机的 `FileNodeRow`（同文件 `:463-509` 目录行 / `:511-572` 文件行）。
    /// - `ListItem` 的 children 是**竖排**的（它内部装 children 的是普通块级 `div`），
    ///   所以整行必须自己套一层 `h_flex()`。
    /// - 行高由内容决定、`uniform_list` 只量第 0 行（`lithe-gpui/UI-MAP.md` §1.3），所以行内容按
    ///   24px 定死，`ListItem` 默认的纵向内边距用 `p_0()` 清掉。
    fn render_tree(&self, cx: &mut Context<Self>) -> AnyElement {
        let on_open = self.on_open.clone();
        let active = self.active.clone();
        let root = self.root.clone();
        // 行渲染要读当前 git 装饰：渲染闭包是 `Fn`，借用 `&self` 活不过这一次调用，所以这里
        // 把装饰表整体按值捕获一次（`Rc` 共享一份不可变快照，不是每行查一次实体）。
        let decorations = Rc::new(self.decorations.clone());
        // 多选集同样按值捕获一次（渲染闭包是 `Fn`，借用 `&self` 活不过这一次调用）。
        // `covers` 让"被选中的文件夹**连带其子孙**"都显示为选中（真源 `:32-41`）。
        let selection = self.selection.clone();
        // 点击时要把"当前打开的文件"记回来给行加底色，但 `ListItem::on_click` 只给 `&mut App`，
        // 拿不到 `Context<Explorer>`，所以带一个弱引用进来。
        let explorer: WeakEntity<Self> = cx.entity().downgrade();
        let menu_explorer = explorer.clone();
        // 行点击要主动把焦点交给树（真源：**树内发生过指针交互之后**才接管快捷键，
        // `ProjectTreeKeyboardCommands.swift:4,44-62`）。
        let focus_handle = self.focus_handle.clone();

        let tree = Tree::new(&self.tree, move |_index, entry, selected, _window, cx| {
            let id = entry.item().id.clone();
            let label = entry.item().label.clone();
            let is_folder = entry.is_folder();
            let expanded = entry.is_expanded();
            let (kind, relative_path) = RowKind::parse(id.as_ref());
            let is_active =
                kind == RowKind::File && active.as_deref() == Some(relative_path.as_str());

            // 展开箭头：只有目录行有；其它行留一个**等宽空槽**，同层的图标才能左对齐
            // （真机的文件行就是一个 `Color.clear.frame(width: 10)`，`ProjectSidebarView.swift:519`）。
            let caret: AnyElement = if is_folder {
                h_flex()
                    .size_4()
                    .mr_0p5()
                    .flex_shrink_0()
                    .justify_center()
                    .child(idea_icon_image(
                        if expanded {
                            &idea::CARET_DOWN_ICON
                        } else {
                            &idea::CARET_RIGHT_ICON
                        },
                        cx,
                        rems(0.75),
                    ))
                    .into_any_element()
            } else {
                div().size_4().mr_0p5().flex_shrink_0().into_any_element()
            };

            let icon = match kind {
                RowKind::EmptyPlaceholder => FileIcon::lucide(IconName::File),
                RowKind::Directory => FileIcon::lucide(if expanded {
                    IconName::FolderOpen
                } else {
                    IconName::FolderClosed
                })
                .with_theme_folder(&label, expanded, cx),
                RowKind::File => icon_for_file(&label, cx),
            };

            // git 状态装饰（真源 `gitStatusColor` + 文件行右侧的字母，
            // `ProjectSidebarView.swift:484,524,529-534`）：
            // - 目录行（含根行）按**最紧急的后代档**染色，不画字母；
            // - 文件行按自己的档染色，并在行尾画状态字母。
            let decoration = decorations.file(&relative_path);
            let tone = if kind == RowKind::File {
                decoration.map(|decoration| decoration.tone)
            } else {
                decorations.directory(&relative_path)
            };

            let mut label_el = div().flex_1().min_w_0().truncate().child(label);
            // 根行标签 600 粗体（真源 `font(..., weight: depth == 0 ? .semibold : .regular)`，
            // `ProjectSidebarView.swift:483`）。整棵树只有一条 `depth == 0` 的行，就是工作区根
            // （`TreeEntry::is_root()`，`gpui-base-0.7.0/src/tree.rs:71-73`）。
            if entry.is_root() {
                label_el = label_el.font_semibold();
            }
            if let Some(tone) = tone {
                label_el = label_el.text_color(tone.color(cx));
            }

            let mut row = h_flex()
                .w_full()
                .h_6()
                .gap_1()
                // 逐层缩进 = 8 + depth × 14：**设计常量**的算术
                // （`ProjectSidebarView.swift:490,536` 的 `depth * 14 + 8`），不是测量出来的
                // 几何 —— 用 helper 底层的 `rems()` 表达后，缩进会随界面字号一起缩放。
                .pl(rems(
                    (BASE_INDENT + entry.depth() as f32 * INDENT_STEP) / 16.,
                ))
                .pr_1p5()
                .rounded(rems(ROW_RADIUS / 16.))
                .child(caret)
                .child(icon.render(rems(1.), cx))
                .child(label_el);

            // 状态字母：真源 `Text(status.displayStatus)` + `.font(.system(size: 9, weight: .bold,
            // design: .monospaced))`（`ProjectSidebarView.swift:529-534`）—— 只出现在**文件行**。
            if let Some(decoration) = decoration {
                row = row.child(
                    div()
                        .flex_shrink_0()
                        .text_size(rems(9. / 16.))
                        .font_weight(FontWeight::BOLD)
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_color(decoration.tone.color(cx))
                        .child(decoration.letter.clone()),
                );
            }

            // 「当前打开的文件」与「被选中的行」都加底色。真机把它们分成**两个** token
            // （活动文件 `subtleSelection`、选中 `selection`，`ProjectSidebarView.swift:495-500,541-548`；
            // 两者在真机上不可区分 —— `lithe-gpui/UI-MAP.md` §2.3 的"优化"一条），而本侧只有
            // `list_active` 一条（同一条缺口），所以先用同一个色，待主题 role 收敛那一批再分开。
            if is_active || selection.covers(&relative_path) {
                row = row.bg(cx.theme().list_active);
            }

            // 目录行的展开/折叠由 `Tree` 在 `mouse_down` 里处理；空目录占位行是禁用的，
            // 连 `mouse_down` 都不会挂上（`gpui-base-0.7.0/src/tree.rs:439-456`）。
            let opening: Option<PathBuf> = if kind == RowKind::File {
                Some(join_relative(&root, &relative_path))
            } else {
                None
            };

            // ⚠️ 这个外层渲染闭包是 `Fn`（`Tree::new` 要求，`gpui-component-0.7.0/src/tree.rs:41-43`），
            // 所以**不能**把捕获到的 `on_open` / `explorer` 移进下面那个 `move` 点击闭包 ——
            // 每行都要现克隆一份（`relative_path` / `opening` 是本次调用新建的局部量，可以直接移）。
            let row_on_open = on_open.clone();
            let row_explorer = explorer.clone();
            let row_focus = focus_handle.clone();
            let row_relative_path = relative_path;
            let row_path = opening;

            ListItem::new(SharedString::from(explorer_row_element_id(id.as_ref())))
                .selected(selected)
                // 行高必须可控：`ListItem` 默认 `py_1() px_3()`，纵向内边距会让
                // 24px 的行装不下内容（`lithe-gpui/UI-MAP.md` §1.3 第一条）。
                .p_0()
                .h_6()
                .text_sm()
                .line_height(relative(ROW_LINE_HEIGHT))
                .whitespace_nowrap()
                .overflow_hidden()
                .child(row)
                .on_click(
                    move |event: &ClickEvent, window: &mut Window, cx: &mut App| {
                        // 选择**先于**打开：目录行不打开文件，但照样要能被选中
                        // （真源 `ProjectTreeSelection` 对目录与文件一视同仁）。
                        // 修饰键沿用仓库既有取法（`editor_view.rs:1785` 的 `modifiers.secondary()`）：
                        // 在 macOS 是 ⌘、其它平台是 Ctrl —— 正好就是真源 `:11-28` 的 `toggling`。
                        let extending = event.modifiers().shift;
                        let toggling = event.modifiers().secondary();
                        // 焦点交给树：此后 ⌘A 才落在树这个 key context 上（真源的规矩同）。
                        window.focus(&row_focus, cx);
                        let is_file_row = row_path.is_some();
                        let _ = row_explorer.update(cx, |this, cx| {
                            // 可见顺序取 `TreeState` 自己那份扁平表（见 `visible_paths`）——
                            // 与真正铺进树里的行**同源**，区间选择按它切片。
                            let visible = this.visible_paths(&**cx);
                            this.selection.select(
                                row_relative_path.as_str(),
                                &visible,
                                extending,
                                toggling,
                            );
                            if is_file_row {
                                // 记住这次打开的文件，给这一行加底色（目录行不设，免得"点一下目录"
                                // 看起来像打开了它）。
                                this.active = Some(SharedString::from(row_relative_path.clone()));
                            }
                            cx.notify();
                        });
                        // 闭包是 `Fn`（可被多次调用），所以路径必须**克隆**出来再用，不能 move。
                        if let Some(path) = row_path.clone() {
                            row_on_open(path, window, cx);
                        }
                    },
                )
        })
        .size_full()
        // 右键菜单：条目表是纯数据（[`row_menu_entries`]），落点统一走
        // [`Explorer::run_row_action`]。`context_menu` 的 builder 拿到的是
        // `&mut Context<TreeState>`（`gpui-component-0.7.0/src/tree.rs:26-27`），够不到
        // `Context<Explorer>`，所以照 `changes_view.rs:905-907` 的现成取法用弱引用。
        .context_menu(move |_index, entry, menu, _window, cx| {
            let (kind, relative_path) = RowKind::parse(entry.item().id.as_ref());
            // 右键**先**按真源把选择切到这一行（`selectForContextMenu`，`:43-49`）：
            // 该行已在选中集内、或（多选且被 `covers`）⇒ **保留**整份选择，否则只选该行。
            // 真机是在菜单**弹出时**做这件事，所以这里也在构建菜单之前做，而不是等某个菜单项被点。
            // `cx` 是 `&mut Context<TreeState>`，`&mut *cx` 退到 `App` 才能更新 `Explorer` 实体
            // （与下面菜单项里那份弱引用是同一条路）。
            let _ = menu_explorer.update(&mut *cx, |this, cx| {
                this.selection.select_for_context_menu(&relative_path);
                cx.notify();
            });
            let mut menu: PopupMenu = menu;
            for action in row_menu_entries(kind) {
                let Some(action) = action else {
                    menu = menu.separator();
                    continue;
                };
                let menu_explorer = menu_explorer.clone();
                let relative_path = relative_path.clone();
                menu = menu.item(PopupMenuItem::new(row_action_label(action)).on_click(
                    move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
                        let _ = menu_explorer.update(cx, |this, cx| {
                            this.run_row_action(action, &relative_path, window, cx);
                        });
                    },
                ));
            }
            menu
        });

        // 树的 a11y：`gpui_base::Tree` 自己会挂 `role(Tree)`，但不会设 aria-label，
        // 所以在包裹层补一次（真机 `aria-label` 取 `fileExplorer.ariaLabel`）。
        // ⚠️ `.role()` / `.aria_label()` 在 `StatefulInteractiveElement` 上，而它只对
        // `Stateful<E>` 实现，所以必须先 `.id(...)` 把 `Div` 变成 `Stateful<Div>`。
        v_flex()
            .id("explorer-tree-viewport")
            .flex_1()
            .min_h_0()
            .w_full()
            .px_1p5()
            .role(Role::Tree)
            .aria_label(tr(TREE_ARIA_LABEL_KEY))
            // 键位：`⌘A` 的**绑定**登记在 `crate::install_actions`（应用级、只登记一次），
            // **处理**放这里 —— "当前在不在树里"由 key context 判定
            // （与 `search_everywhere.rs:861-862` 同一条约定）。
            .track_focus(&self.focus_handle)
            .key_context(crate::EXPLORER_KEY_CONTEXT)
            .on_action(cx.listener(Self::select_all_in_tree))
            .child(tree)
            .into_any_element()
    }

    /// 树体为空时的空态。分支照真机的三段文案：
    /// 没有工作区 →「未打开文件夹」；有查询无命中 →「没有匹配的文件」；否则 →「文件夹为空」。
    fn empty_no_rows(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.root.as_os_str().is_empty() {
            // 真机的动作按钮是「打开文件夹」（`welcome.openFolder`，`locale.ts:8740`）。
            // 打开目录需要一个系统目录选择器、而且换工作区根要重建整个外壳布局，
            // 本模块拿不到这条通路，所以按"宁可禁用也不画假按钮"的约定渲染成**禁用态**并登记。
            return self.empty(
                tr(NO_FOLDER_OPEN_KEY),
                Some((tr(OPEN_FOLDER_KEY), true)),
                cx,
            );
        }

        let message = if self.query.is_empty() {
            tr("lithe.fileExplorer.folderIsEmpty")
        } else {
            tr(NO_MATCHING_FILES_KEY)
        };
        self.empty(message, None, cx)
    }

    /// 加载失败：显示 Core 错误码 + 「重试」（真的会重新取快照）。
    fn empty_failed(&self, error: String, cx: &mut Context<Self>) -> AnyElement {
        self.empty(SharedString::from(error), Some((tr(RETRY_KEY), false)), cx)
    }

    /// 空态的公共骨架：`Empty`（居中、`gap-2`、`p-3`、虚线边，真源关闭虚线）
    /// + `EmptyDescription` 承载文案 + 可选 `EmptyContent` 里的一个 `small` 按钮。
    fn empty(
        &self,
        message: SharedString,
        action: Option<(SharedString, bool)>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut empty = Empty::new()
            // `Empty` 的 render 里**硬编码**了 `.border_dashed().border_color(cx.theme().border)`，
            // 真机界面没有这圈虚线；把边框色改成透明来关掉它（编辑器空态用的是同一招）。
            .border_color(cx.theme().transparent)
            .header(
                EmptyHeader::new().description(EmptyDescription::new().text_sm().child(message)),
            );

        if let Some((label, disabled)) = action {
            // 密度走组件 `Size` 档位，不手写外框高度（`design-guides.md:115`「优先使用组件尺寸
            // （`xsmall`、`small`、默认 `medium`、`large`），不要单独写高度」、`:208`「使用组件
            // `Size` API，让字体、图标、padding 和 hit target 一起变化」）。
            // `.small().compact()`：外框 `h_6()`(24px) + 内衬 `px_1p5()`(6px)，字号随档位落到
            // `text_sm()`(14px)，与上一行 `EmptyDescription` 的 `.text_sm()` 一致
            // （13 → 14 是本文件「度量」段已确认的有意改动）。口径同 Git 的同一形状按钮。
            let button = Button::new("explorer-empty-action")
                .label(label)
                .small()
                .compact();
            let button = if disabled {
                button.disabled(true)
            } else {
                button.on_click(cx.listener(
                    |this: &mut Self,
                     _event: &ClickEvent,
                     _window: &mut Window,
                     cx: &mut Context<Self>| {
                        this.load(cx);
                    },
                ))
            };
            empty = empty.content(EmptyContent::new().child(button));
        }

        // `Empty` 的根是 `v_flex().flex_1()`，会吃掉剩余高度并把内容居中；外面这层必须是
        // flex 容器（`v_flex`），否则 `flex_1` 不生效、空态会贴在顶部。
        v_flex().size_full().child(empty).into_any_element()
    }
}

/// 工作区根那一行显示的名字：根目录名；根是盘符根（`D:\`）这类没有文件名的情况退回整条路径。
fn root_label(root: &Path) -> String {
    root.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| root.to_string_lossy().to_string())
}

/// 树行的 ElementId：由 `TreeItem` 自己的 id 决定（`dir:<相对路径>` / `file:<相对路径>` /
/// `empty:<相对路径>`，见 [`crate::model::RowKind::parse`]）—— 那是建树时按路径写死的稳定
/// identity，展开 / 折叠 / 搜索过滤都改不了它。
///
/// 不用 `ListItem` 的下标：树是 `uniform_list`，行的下标随"展开 / 折叠一个目录、输入一次搜索"
/// 整体变化，用下标当 key 会让展开态与选中底色串到别的行上（《编码指南》「稳定标识」与
/// 「精确区分领域词汇」：index 是位置，id 是 identity，可重排数据不能拿 index 当 key）。
fn explorer_row_element_id(row_id: &str) -> String {
    format!("explorer-row:{row_id}")
}

/// 把工作区相对路径（Core 约定用 `/` 分隔）拼成绝对路径。
///
/// 逐段 `join` 而不是整串 `join`：这样结果用的是平台自己的分隔符，空段与 `..` 的行为也更可控。
fn join_relative(root: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .filter(|segment| !segment.is_empty())
        .fold(root.to_path_buf(), |path, segment| path.join(segment))
}

/// 绝对路径 → 工作区相对路径（`/` 分隔）；不在工作区根之下时返回 `None`。
///
/// 逐段比对交给 `Path::strip_prefix`（按**路径分量**比较，不是字符串前缀），
/// 所以 `…/ws-other` 这种"字符串前缀相同但不在工作区里"的路径不会被误判成工作区内。
fn relative_to_root(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let text = relative.to_string_lossy().replace('\\', "/");
    let text = text.trim_matches('/').to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// 定位 `relative` 所指那一行需要展开的**祖先目录 id**（含工作区根行 `dir:`）。
///
/// 目标自身不在其中：文件行不需要展开（目录行作为目标的情况在 U24 里不出现 ——
/// 入口是"把当前编辑的活动**文件**在树里定位"）。
fn ancestor_dir_ids(relative: &str) -> Vec<String> {
    let segments: Vec<&str> = relative
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let mut ids = vec![ROOT_ID.to_string()];
    let mut prefix = String::new();
    for segment in segments.iter().take(segments.len().saturating_sub(1)) {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(segment);
        ids.push(format!("dir:{prefix}"));
    }
    ids
}

impl Render for Explorer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // `render_body` 返回 `AnyElement`（具体类型，不借用 self）：edition 2024 下
        // `-> impl IntoElement` 会捕获作用域内的生命周期，返回 `&self` 派生的类型会和下面
        // `cx.theme()` 的共享借用打架（E0502）。上一轮实现踩过同一个坑。
        //
        // 根容器下**只有主体**：顶部的「项目」头部行与树内搜索行已在 **2026-09-27 按维护者要求**
        // 删除（见上面「渲染」小节开头的说明），所以侧栏顶部就是文件树本身。
        let body = self.render_body(cx);

        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 回归判据（《编码指南》「稳定标识」）：同一棵树的节点在两种展示顺序下算出的 id 必须
    /// 相同，不同节点的 id 必须不同。用行下标当 id 时，倒序的每一行都会拿到对方的 id，
    /// 这条会失败。
    ///
    /// 纯函数测试（「测试策略」第 1 层）：不起窗口、不建 `Tree`，直接喂 `TreeItem` 的 id
    /// —— 那正是建树时写死的稳定身份。
    #[test]
    fn row_element_ids_do_not_depend_on_the_visible_row_order() {
        let nodes = [
            "dir:",
            "dir:src",
            "file:src/main.rs",
            "empty:src/empty-folder",
        ];

        let forward: Vec<String> = nodes.iter().map(|id| explorer_row_element_id(id)).collect();
        let reversed: Vec<String> = nodes
            .iter()
            .rev()
            .map(|id| explorer_row_element_id(id))
            .collect();

        assert_eq!(forward[0], reversed[3], "根行的 id 必须与行号无关");
        assert_eq!(forward[3], reversed[0], "空目录占位行的 id 必须与行号无关");
        assert_eq!(forward[1], "explorer-row:dir:src", "id 必须由节点 id 构成");
        // 两两不同：`dir:src` 与 `file:src/main.rs` 是不同节点（目录行与文件行各有一份状态）。
        for left in 0..nodes.len() {
            for right in (left + 1)..nodes.len() {
                assert_ne!(forward[left], forward[right]);
            }
        }
    }

    /// 右键菜单的**条目表**：文件行、目录行各自与真源逐条对上，且空目录占位行没有菜单
    /// （占位行是禁用行，`Tree::context_menu` 会跳过它）。
    ///
    /// 每个 `Some(..)` 的落点由 [`Explorer::run_row_action`] 的穷尽 `match` 保证 ——
    /// 那条 match 少一个分支就编译不过，所以"画了不接"不可能发生。
    #[test]
    fn row_menu_entries_match_the_macos_context_menus() {
        assert_eq!(
            row_menu_entries(RowKind::File),
            vec![
                Some(RowAction::Open),
                None,
                Some(RowAction::RevealInFileManager),
                Some(RowAction::CopyPath),
                Some(RowAction::CopyRelativePath),
            ],
            "文件行照真源 fileContextMenuItems（ProjectSidebarView.swift:704-745）"
        );
        assert_eq!(
            row_menu_entries(RowKind::Directory),
            vec![
                Some(RowAction::RevealInFileManager),
                Some(RowAction::CopyPath),
                Some(RowAction::CopyRelativePath),
                None,
                Some(RowAction::Refresh),
            ],
            "目录行照真源 directoryContextMenuItems（同文件 :574-648），末尾那条刷新是真的重取快照"
        );
        assert!(
            row_menu_entries(RowKind::EmptyPlaceholder).is_empty(),
            "空目录占位行是禁用行，不该有右键菜单"
        );
    }

    /// 不画的那几项**不许**悄悄回来：真源里有、本版没有落点的条目（新建/重命名/删除/复制副本/
    /// 本地历史/Show Git Diff/目录标记）在条目表里一个都不出现。
    ///
    /// 反证口径：`RowAction` 只有 5 个变体，所以"菜单里没有写文件的动作"是**类型层面**的事实；
    /// 这条测试把"当前就是这 5 个"写成断言，任何人加变体都会先看到它。
    #[test]
    fn row_menu_has_no_write_actions() {
        let mut actions: Vec<RowAction> = row_menu_entries(RowKind::File)
            .into_iter()
            .chain(row_menu_entries(RowKind::Directory))
            .flatten()
            .collect();
        actions.sort_by_key(|action| format!("{action:?}"));
        actions.dedup();
        assert_eq!(
            actions,
            vec![
                RowAction::CopyPath,
                RowAction::CopyRelativePath,
                RowAction::Open,
                RowAction::Refresh,
                RowAction::RevealInFileManager,
            ],
            "菜单里的动作集合必须正好是这 5 条（新增变体前先确认真源有落点）"
        );
    }

    /// 菜单文案全部走 catalog 既有键，且**五条互不相同**（两条「路径」文案指同一个键就会漏配）。
    #[test]
    fn row_action_labels_are_distinct_catalog_keys() {
        let labels: Vec<String> = [
            RowAction::Open,
            RowAction::RevealInFileManager,
            RowAction::CopyPath,
            RowAction::CopyRelativePath,
            RowAction::Refresh,
        ]
        .iter()
        .map(|action| row_action_label(*action).to_string())
        .collect();
        for label in &labels {
            assert!(
                !label.starts_with("lithe."),
                "文案必须是译文而不是键名（键漏配时 rust-i18n 会原样回显），实际 = {label}"
            );
        }
        for left in 0..labels.len() {
            for right in (left + 1)..labels.len() {
                assert_ne!(labels[left], labels[right], "两条动作的文案不许重复");
            }
        }
    }

    /// 相对路径归一：`/` 分隔、去掉首尾斜杠；不在工作区根之下的路径返回 `None`。
    #[test]
    fn relative_to_root_normalizes_and_rejects_outside_paths() {
        let root = Path::new("workspace");
        assert_eq!(
            relative_to_root(root, &Path::new("workspace").join("src").join("main.rs")),
            Some("src/main.rs".to_string())
        );
        assert_eq!(
            relative_to_root(root, Path::new("workspace")),
            None,
            "工作区根自己没有可定位的行"
        );
        assert_eq!(
            relative_to_root(root, &Path::new("elsewhere").join("a.txt")),
            None
        );
        // 字符串前缀相同但**不在**工作区里：`strip_prefix` 按分量比较，必须判出来。
        assert_eq!(
            relative_to_root(root, &Path::new("workspace-other").join("a.txt")),
            None
        );
    }

    /// Reveal 要展开的祖先目录：从根行一路到目标所在目录，目标自身不在其中。
    #[test]
    fn ancestor_dir_ids_walk_from_the_root_row_to_the_parent_directory() {
        assert_eq!(
            ancestor_dir_ids("src/main/java/App.java"),
            vec![
                "dir:".to_string(),
                "dir:src".to_string(),
                "dir:src/main".to_string(),
                "dir:src/main/java".to_string(),
            ]
        );
        assert_eq!(
            ancestor_dir_ids("main.rs"),
            vec!["dir:".to_string()],
            "根下的文件只需要展开根行"
        );
    }

    /// 缩进是真源那一对设计常量（`ProjectSidebarView.swift:490` 的 `depth * 14 + 8`），
    /// 不是旧实现的 `10 + depth × 16`。
    #[test]
    fn indentation_follows_the_macos_metrics() {
        assert_eq!(BASE_INDENT, 8.);
        assert_eq!(INDENT_STEP, 14.);
        assert_eq!(BASE_INDENT + 2. * INDENT_STEP, 36.);
    }
}
