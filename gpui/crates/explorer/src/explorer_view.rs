//! 项目树的视图层：`Explorer` 实体、`LoadState` 与各区域渲染。
//!
//! 建树与取数据在 `model.rs`（那个文件不依赖 gpui UI 组件）；crate 的模块文档（规格出处、
//! 图标对照、未实现清单）在 `lib.rs`。本文件只放宽 `Explorer` 为 `pub`（由 `lib.rs`
//! re-export），其余项都留在模块内。
//!
//! **现状（2026-09-27 按维护者要求删除）**：侧栏顶部**不再有**「项目」头部行（原 32px）与
//! 树内搜索行（原 28px），本模块渲染出的第一个元素就是文件树本身；搜索入口改由左活动栏的
//! 「搜索」项承担（`gpui/crates/workbench/src/activity_bar.rs`）。这两条与 Windows 真机的差异
//! 原先登记在 `lib.rs` 未实现清单的第 10 条（偏好下拉菜单）与第 11 条（搜索浮层），随实现一并
//! 删除。搜索状态（`search` / `search_open` / `query`）与 `toggle_search` / `clear_search` /
//! `set_query` / `rebuild` 的过滤通路**保留**，等左活动栏与树内 `Mod+F` / `/` 键位接上，
//! 保留理由写在各声明处。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{TreeState, h_flex, v_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tree::{Tree, TreeEvent};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, Role, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, WeakEntity, Window, div, relative, rems,
};

use lithe_gpui_shared::icons::FileIcon;
use lithe_gpui_shared::tr;

use crate::model::{RENDER_LIMIT, ROOT_ID, RowKind, build_tree_items, icon_for_file, load_snapshot};

// ---------------------------------------------------------------------------
// 文案：全部走 `lithe_gpui_shared::tr`（key 逐字取 `windows/tauri/src/i18n/locale.ts`
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

// ---------------------------------------------------------------------------
// 度量：一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// 全部取自 Windows 源码（规格真源）。rem base = 主题字号 16px，所以 helper 后缀 `N` = `N × 4px`，
// 与规格逐像素相等。表里只列**仍在渲染路径上**的项：
//
// | 规格 | 值 | 用到的 helper | 出处 |
// | --- | --- | --- | --- |
// | 树行高 | 24 | `h_6()` | 见下方 `BASE_INDENT` 注释 |
// | 行水平内缩 / 右内边距 `px-1.5` | 6 | `px_1p5()` / `pr_1p5()` | `file-explorer-tree.css:6`、`sidebar-tree.tsx:241` |
// | 行内列间距 `--lithe-chrome-gap` | 4 | `gap_1()` | `theme.css:130` |
// | 展开箭头槽 `size-4` / 右侧 `mr-0.5` | 16 / 2 | `size_4()` / `mr_0p5()` | `sidebar-tree.tsx:301` |
// | 箭头字形 / 文件图标 | 12 / 16 | `size_3()` / `size_4()` | `file-explorer-tree.css:121-124,126-131` |
// | 加载胶囊 `p-3` / `px-3 py-1.5` / `gap-2` | 12 / 12 / 6 / 8 | `top_3()` / `px_3()` / `py_1p5()` / `gap_2()` | `file-explorer-pane.tsx:55-56`、`ui/spinner.tsx:44` |
//
// **2026-09-27 按维护者要求删除**的那两行（「项目」头部行 32px、树内搜索行 28px）曾经用到的度量，
// 现在文件里已无调用点，一并列出以免后来者以为漏了：头部高 32 → `h_8()`（`styles/theme.css:124`）、
// 头部 `px-2 py-1`（`file-explorer-tree.tsx:1310`、`ui/sidebar.tsx:93`）、头部图标按钮 `icon-xs`
// 24 → `size_6()`（`ui/button.tsx:27`）、搜索框高 28 → `h_7()`（`global-search-toolbar.tsx:101-102`）、
// 搜索行 `px-2 py-2`（同文件 `:93`）。随它们删除的具名常量是 `HEADER_BUTTON_RADIUS`（6.4）、
// `TITLE_LINE_HEIGHT`（16）、`SEARCH_INPUT_RADIUS`（8）与那个只给 `Button::rounded` 用的
// `rem_px`；这两个与真机的差异原先登记在 `lib.rs` 未实现清单第 10 条（偏好下拉菜单）与第 11 条
// （搜索浮层）。
//
// 字号：`--ui-text-sm` = 13px（`theme.css:116`）不在 gpui 的档位（`text_xs()`=12 / `text_sm()`=14）
// 上，按《编码指南》用 **`text_sm()`（14px）**——13 → 14 是经维护者确认的**有意**视觉改动。
//
// ⚠️ `line_height` 没有 rem 档位 helper（`gpui-pre-0.3.6/src/styled.rs:740` 只有取值形式），
// 需要定长行高时用 helper 底层的 `rems(P / 16.)` 表达；倍数行高仍走 `relative(..)`
// （见 [`ROW_LINE_HEIGHT`]）。

/// 树行高 24px。
///
/// 真机公式 `max(24, uiFontSize × 1.35 + 6)`，`uiFontSize = 13` → `max(24, 23.55) = 24`
/// （`file-explorer/lib/file-tree-row.ts:1-14`；13px 见 `theme.css:112`）。调用点直接 `h_6()`。
///
/// 表体是 `uniform_list`，**行高只取第 0 行的测量值**再按固定值铺满
/// （`gpui-pre-0.3.6/src/elements/uniform_list.rs:658-680,371,397`），所以行内容必须正好是这个高度，
/// 行内纵向内边距一律清零（`ListItem` 默认 `py_1`，见 `gpui-component-0.6.6/src/list/list_item.rs:186`）。
///
/// 行基准缩进 10px（`file-explorer/lib/file-tree-row.ts:1` `FILE_TREE_BASE_INDENT = 10`）与
/// 缩进步长 16px（默认 `fileTreeIndentSize: 16`，`features/settings/config/default-settings.ts:182`；
/// 可选 12/16/20/24 见 `file-explorer-tree.tsx:1482-1493`）参与逐层缩进的算术：10 不在 rem 档位上
/// （档位里 8 / 12），但那正是该写 helper 底层 `rems(P / 16.)` 的场合 —— 调用点
/// `.pl(rems((BASE_INDENT + depth × INDENT_STEP) / 16.))`。它们是**设计常量**（真源 TS 里的
/// `10 + depth × 16`），不是测量几何，所以 rem 化后缩进随界面字号缩放。
const BASE_INDENT: f32 = 10.;
const INDENT_STEP: f32 = 16.;
/// 行圆角 4px（`file-explorer-tree.css:7` `--file-tree-row-radius: 4px`）。
///
/// 与已随头部行删除的 `HEADER_BUTTON_RADIUS`（6.4）同因保留应用层具名常量：它不在 gpui 的
/// rem 档位上，也不能从主题读 —— `ThemeConfig.radius` 是 `Option<usize>`
/// （`gpui-component-0.6.6/src/theme/schema.rs:66-68`），装不下 Lithe 的 `--radius × k` 阶梯
/// （4.8 / 6.4 / 11.2）。消费方式仍是 rem：`div` 上的 `Styled::rounded` 吃 `AbsoluteLength`，
/// 调用点直接写 `rems(ROW_RADIUS / 16.)` ——「不在档位上」不是保留 `px(...)` 的理由。
const ROW_RADIUS: f32 = 4.;
/// 行高倍数 1.35（`theme.css:4` `--leading-row: 1.35`；`sidebar-tree.tsx:241` 的 `leading-row`）——
/// 是倍数不是长度，`line_height(relative(..))` 原样保留。
const ROW_LINE_HEIGHT: f32 = 1.35;

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
        // ⚠️ placeholder 这一处是搜索簇**保留**下来的活代码之一：树内搜索行已在 **2026-09-27 按
        // 维护者要求**删除，所以这个 `InputState` 目前没有可见的输入框；等左活动栏的「搜索」项与
        // 树内 `Mod+F` / `/` 键位接上后复用（见 `search` 字段的注释）。
        let search = cx.new(|cx| InputState::new(window, cx));
        search.update(cx, |state, cx| {
            state.set_placeholder(tr(SEARCH_FILES_KEY), window, cx);
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
    /// `shared::core_client` 的 Core 调用是同步的（`lithe_core::execute_json`，
    /// `rust/lithe-core/src/lib.rs:25`），直接放在 UI 线程上
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
    /// 收起时清空查询：真机的 `Escape` 也是"关闭搜索"（`file-explorer-tree.tsx:1329-1335`），
    /// 留着过滤词会让下次打开看到一棵"少了东西"的树。
    ///
    /// ⚠️ **暂时不可达**：唯一调用点（头部行的搜索按钮）已在 **2026-09-27 按维护者要求**删除，
    /// 所以这里必须 `#[allow(dead_code)]`。保留它是因为左活动栏「搜索」项与树内 `Mod+F` / `/`
    /// 键位（真机键位见 `lib.rs` 未实现清单第 9 条）接上时，这个开关就是入口；维护者明确要求
    /// 不要为了让 warning 消失而删掉搜索状态。
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

    /// 清空查询并重建（对应 `search.clear`「清除搜索」按钮，`file-explorer-tree.tsx:1344-1356`；
    /// 那个按钮本身已随头部行在 **2026-09-27 按维护者要求**删除）。
    ///
    /// ⚠️ **暂时不可达**：现在只有 [`Explorer::toggle_search`] 调它，而那个入口本身也暂时不可达
    /// （原因见那里），所以这里必须 `#[allow(dead_code)]`；保留它是为了接上搜索入口后直接复用
    /// 「Escape / 关闭搜索要连过滤词一起清掉」这条语义。
    ///
    /// ⚠️ **不能只靠 `InputEvent::Change`**：`InputState::set_value` 内部把 `emit_events` 置成
    /// `false`（`gpui-base-0.6.6/src/input/base/state.rs:903-907`），**不会**发 `Change` 事件，
    /// 所以这里必须自己调 [`Explorer::set_query`]。
    #[allow(dead_code)]
    fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let search = self.search.clone();
        search.update(cx, |state, cx| {
            state.set_value(SharedString::default(), window, cx);
        });
        self.set_query(SharedString::default(), cx);
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
                        .top_3()
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
                                .gap_2()
                                .px_3()
                                .py_1p5()
                                .rounded_full()
                                .bg(cx.theme().muted)
                                .border_1()
                                .border_color(cx.theme().border)
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                // 转圈图标 16 + 文案 13（→ `text_sm()` 14）+ 间距 8：
                                // `ui/spinner.tsx:20-51` 的 `size-4` / `ui-text-sm` / `gap-2`。`Spinner` 默认 `Size::Medium`
                                // → 16px（`gpui-component-0.6.6/src/spinner.rs:22`、`icon.rs:185`），
                                // 自带 `with_animation` 常转（`spinner.rs:60-74`）。
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

        let tree = Tree::new(&self.tree, move |_index, entry, selected, _window, cx| {
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
                    .size_4()
                    .mr_0p5()
                    .flex_shrink_0()
                    .justify_center()
                    .child(
                        Icon::new(if expanded {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .size_3()
                        .text_color(cx.theme().muted_foreground),
                    )
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
                .h_6()
                .gap_1()
                // 逐层缩进 = 10 + depth × 16：**设计常量**的算术（`file-explorer/lib/file-tree-row.ts:1`
                // 的 `FILE_TREE_BASE_INDENT = 10` + 默认 `fileTreeIndentSize: 16`），不是测量出来的
                // 几何 —— 用 helper 底层的 `rems()` 表达后，缩进会随界面字号一起缩放。
                .pl(rems((BASE_INDENT + entry.depth() as f32 * INDENT_STEP) / 16.))
                .pr_1p5()
                .rounded(rems(ROW_RADIUS / 16.))
                .child(caret)
                .child(icon.render(rems(1.), cx))
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

            ListItem::new(SharedString::from(explorer_row_element_id(id.as_ref())))
                .selected(selected)
                // 行高必须可控：`ListItem` 默认 `py_1() px_3()`
                // （`gpui-component-0.6.6/src/list/list_item.rs:186-187`），纵向内边距会让
                // 24px 的行装不下内容（`gpui/UI-MAP.md` §1.3 第一条）。
                .p_0()
                .h_6()
                .text_sm()
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
            .px_1p5()
            .role(Role::Tree)
            .aria_label(tr(TREE_ARIA_LABEL_KEY))
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

    /// 空态的公共骨架，照 `ui/empty.tsx:111-130` 的 `EmptyState`：
    /// `Empty`（居中、`gap-2`、`p-3`、`rounded-lg`、虚线边）+ `EmptyDescription` 承载文案
    /// + 可选 `EmptyContent` 里的一个 `xs` 按钮（高 24、`px-1.5`，`ui/button.tsx:23`）。
    fn empty(
        &self,
        message: SharedString,
        action: Option<(SharedString, bool)>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut empty = Empty::new().header(
            EmptyHeader::new().description(EmptyDescription::new().text_sm().child(message)),
        );

        if let Some((label, disabled)) = action {
            let button = Button::new("explorer-empty-action")
                .label(label)
                .h_6()
                .px_1p5();
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

        // `Empty` 的根是 `v_flex().flex_1()`，会吃掉剩余高度并把内容居中
        // （`gpui-component-0.6.6/src/empty.rs:63-82`）；外面这层必须是 flex 容器
        // （`v_flex`），否则 `flex_1` 不生效、空态会贴在顶部。
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
    use super::explorer_row_element_id;

    /// 回归判据（《编码指南》「稳定标识」）：同一棵树的节点在两种展示顺序下算出的 id 必须
    /// 相同，不同节点的 id 必须不同。用行下标当 id 时，倒序的每一行都会拿到对方的 id，
    /// 这条会失败。
    ///
    /// 纯函数测试（「测试策略」第 1 层）：不起窗口、不建 `Tree`，直接喂 `TreeItem` 的 id
    /// —— 那正是建树时写死的稳定身份。
    #[test]
    fn row_element_ids_do_not_depend_on_the_visible_row_order() {
        let nodes = ["dir:", "dir:src", "file:src/main.rs", "empty:src/empty-folder"];

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
}
