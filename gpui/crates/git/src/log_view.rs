//! Git Feature 的**表现层**：`BottomPane` 的 `Render` 与引用 / 提交 / 提交详情三栏。
//!
//! 从 `shell_probe/bottom_panel.rs` 的「面板」一节起原样拆出（只调整可见性）。
//! 数据、纯函数、度量常量与 Core 调用都在 `model.rs`；本文件只负责画。
//!
//! 对外公开的是 [`BottomPane`]（`new` / `set_visible` / `visible` / `refresh`），字段全部私有。
//!
//! 规格出处（度量速查表、Core 命令表、与 Windows 源码的 9 条刻意偏差）逐条列在 crate 根
//! `lib.rs` 的模块文档里，那是唯一真源；本文件只在渲染函数旁写具体引用。

use std::collections::BTreeSet;
use std::path::PathBuf;

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Div, Entity, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, WeakEntity, Window, div, px,
    relative, rems,
};

use lithe_gpui_shared::{tr, tr_args};

use crate::model::{
    AUTHOR_COLUMN_WIDTH, ButtonHandler, COMMIT_CONTENT_MIN_WIDTH, COMMIT_FILE_INDENT_BASE,
    COMMIT_FILE_INDENT_STEP, COMMIT_FILE_STATUS_FONT_SIZE, COMMIT_PANE_FRACTION, COMMIT_ROW_HEIGHT,
    Commit, CommitFileRow, DATE_FONT_SIZE, Detail, FILTER_INPUT_MAX_WIDTH, FILTER_INPUT_MIN_WIDTH,
    FilesState, FilterScope, FirstLoad, GRAPH_LANE_GAP, GRAPH_LINE_WIDTH, GRAPH_MIN_WIDTH,
    GRAPH_NODE_RADIUS, GraphRow, HistoryCursor, ICON_BUTTON_RADIUS, ICON_BUTTON_SIZE,
    INSPECTOR_DETAILS_FRACTION, INSPECTOR_FILES_FRACTION, INSPECTOR_FILES_MIN_HEIGHT,
    INSPECTOR_HASH_FONT_SIZE, INSPECTOR_MONO_FONT_SIZE, INSPECTOR_PANE_FRACTION,
    INSPECTOR_PANE_MIN_WIDTH, LABEL_FONT_SIZE, LABEL_MAX_WIDTH, LABEL_PADDING_Y, LABEL_RADIUS,
    LOAD_MORE_BUTTON_HEIGHT, Label, LoadState, MoreLoad, Panel, REFERENCE_BADGE_FONT_SIZE,
    REFERENCE_HEAD_ROW_RADIUS, REFERENCE_INDENT_BASE, REFERENCE_INDENT_STEP,
    REFERENCE_PANE_FRACTION, REFERENCE_PANE_MIN_WIDTH, REFERENCE_PILL_MAX_WIDTH,
    REFERENCE_PILL_RADIUS, REFERENCE_ROW_RADIUS, REFERENCE_SECTION_RADIUS,
    REFERENCE_TOOLBAR_BUTTON_RADIUS, RefKind, Reference, TRACKING_COUNT_FONT_SIZE,
    build_commit_files, build_reference_rows, handler, label_color, lane_color, layout_graph,
    load_commit_files, load_first, load_more, matches_filter, tracking_count,
};
// ---------------------------------------------------------------------------
// 稳定标识（ElementId）
// ---------------------------------------------------------------------------
//
// `ElementId` 是元素局部状态（hover / focus / 滚动）的键，必须是**稳定 identity**。
// 下标只是数据在当帧列表里的位置：筛选、翻页或重排之后同一个下标会指向另一条数据，
// 元素状态就会串到别的行上 —— 《编码指南》「稳定标识」与「精确区分领域词汇」都要求
// index 只表示位置、id 才表示 identity，可重排的数据不能用 index 当 key。

/// 页签的 ElementId：只由 [`Panel`] 决定，与它在页签行里的位置无关。
fn panel_element_id(panel: Panel) -> &'static str {
    // 取值里带领域名（`log` / `console`）而不是编号：两个页签是**不同**的领域对象，
    // 键一旦相同也会互相覆盖状态。
    match panel {
        Panel::Log => "bottom-git-tab:log",
        Panel::Console => "bottom-git-tab:console",
    }
}

/// 提交行的 ElementId：由提交自己的哈希决定（Core `GitCommitResponse.hash`，
/// `protocol/contracts.rs:602-611`）。
///
/// 不用下标：`visible_commits` 随筛选变化、`load_more` 往列表尾部追加，同一个下标在不同帧
/// 指向的提交可能完全不同 —— 那时 hover / 元素状态会跟着"第 N 行"而不是"这个提交"走。
fn commit_element_id(commit: &Commit) -> String {
    format!("bottom-git-commit:{}", commit.hash)
}

// ---------------------------------------------------------------------------
// 面板
// ---------------------------------------------------------------------------

/// 底部工具窗「提交记录」。
///
/// 对外只暴露 [`BottomPane::new`] 与 [`BottomPane::refresh`] / [`BottomPane::set_visible`] /
/// [`BottomPane::visible`]；字段全部私有。本面板不开浮层（对话框 / 抽屉 / 通知的挂层由窗口
/// 根视图负责）。
pub struct BottomPane {
    /// 工作区根，来自命令行参数（`git.*` 命令都要求带 `root`）。
    root: PathBuf,
    /// `git.status.repositoryRoot` 解析成绝对路径后的仓库根（见 [`resolve_repository_root`]）。
    repository_root: Option<String>,
    /// `git.status.branch`：没有选中引用时标题栏的兜底。
    branch: Option<SharedString>,
    load_state: LoadState,
    /// 当前页签（局部状态，`git-log-tool-window.tsx:80`）。
    panel: Panel,
    /// 引用（`git.references`）。
    references: Vec<Reference>,
    /// 提交（`git.historyPage`，可能有多页）。
    commits: Vec<Commit>,
    /// 持有的分页游标；`None` = 读完了 / 没在读。
    cursor: Option<HistoryCursor>,
    has_more: bool,
    loading_more: bool,
    /// 选中的引用（[`Self::references`] 的下标）。
    selected_reference: Option<usize>,
    /// 选中的提交（[`Self::commits`] 的下标）。
    selected_commit: Option<usize>,
    /// 折叠的分区（`local / remote / tag`）。
    collapsed_sections: BTreeSet<&'static str>,
    /// 折叠的引用分组 id。
    collapsed_groups: BTreeSet<String>,
    /// 「只显示我的分支」（`git-log-preferences.store.ts:64`）。
    show_my_branches_only: bool,
    /// 引用树是否画装饰（`git-log-preferences.store.ts:63`，默认 true）。
    show_decorations: bool,
    /// 提交文件列表状态。
    files_state: FilesState,
    files: Vec<CommitFileRow>,
    /// 提交详情（选中提交后填）。
    detail: Option<Detail>,
    /// 筛选输入框（`git-commit-table.tsx:201` 的 `<input>`）。
    filter: Entity<InputState>,
    filter_query: SharedString,
    filter_scope: FilterScope,
    /// 输入事件订阅：不存下来会被立刻丢掉，输入框就不再触发重绘。
    _filter_subscription: Subscription,
    /// 可见性（Windows 由外部 `setIsBottomPaneVisible` 控制；这里给父级一个钩子）。
    visible: bool,
    /// 请求代次：晚到的旧回包直接丢掉。
    request_serial: u64,
}

impl BottomPane {
    /// 建立面板并立刻在后台跑一遍首屏读取。
    ///
    /// `root` 是工作区根；`git.status` 会在它下面解析出真正的仓库根。
    pub fn new(root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter_scope = FilterScope::Text;
        let placeholder = filter_scope.placeholder();
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));

        let filter_subscription = cx.subscribe_in(
            &filter,
            window,
            |pane: &mut Self, state: &Entity<InputState>, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    pane.filter_query = state.read(cx).value();
                    cx.notify();
                }
            },
        );

        let mut pane = Self {
            root: root.clone(),
            repository_root: None,
            branch: None,
            load_state: LoadState::Loading,
            panel: Panel::Log,
            references: Vec::new(),
            commits: Vec::new(),
            cursor: None,
            has_more: false,
            loading_more: false,
            selected_reference: None,
            selected_commit: None,
            collapsed_sections: BTreeSet::new(),
            collapsed_groups: BTreeSet::new(),
            show_my_branches_only: false,
            show_decorations: true,
            files_state: FilesState::Idle,
            files: Vec::new(),
            detail: None,
            filter,
            filter_query: SharedString::from(""),
            filter_scope,
            _filter_subscription: filter_subscription,
            visible: true,
            request_serial: 0,
        };

        pane.spawn_first_load(cx);
        pane
    }

    /// 可见性钩子（Windows 的 `setIsBottomPaneVisible(false)`，`git-log-tool-window.tsx:579`）。
    /// 父级在布局里应先问 [`Self::visible`]，否则隐藏后只会留一块空白。
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    /// 重新跑一遍首屏读取（标题栏刷新按钮 / 引用选中变化都走这里）。
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.spawn_first_load(cx);
    }

    /// 起一次首屏读取：把当前游标交给后台任务去归还，重排状态，再 spawn。
    fn spawn_first_load(&mut self, cx: &mut Context<Self>) {
        let root = self.root.clone();
        let reference = self.selected_reference_full_name();
        let cursor = self.cursor.take();
        self.has_more = false;
        self.loading_more = false;
        // 已有数据时保持原状态（失败会切到 `Stale`，画横幅而不是把列表清空）。
        if self.commits.is_empty() {
            self.load_state = LoadState::Loading;
        }
        self.request_serial = self.request_serial.wrapping_add(1);
        let serial = self.request_serial;
        let this = cx.entity().downgrade();
        cx.notify();

        cx.spawn(async move |_this, cx| {
            let load = cx
                .background_spawn(async move { load_first(&root, reference, cursor) })
                .await;
            let _ = this.update(cx, |pane, cx| pane.apply_first(serial, load, cx));
        })
        .detach();
    }

    /// 把首屏结果写进面板。
    fn apply_first(&mut self, serial: u64, load: FirstLoad, cx: &mut Context<Self>) {
        if serial != self.request_serial {
            return;
        }

        self.references = load.references;
        if let Some(repository_root) = load.repository_root {
            self.repository_root = Some(repository_root);
        }
        if load.branch.is_some() {
            self.branch = load.branch;
        }

        if self.repository_root.is_none() {
            self.load_state = LoadState::NoRepository;
            self.commits = Vec::new();
            self.has_more = false;
            self.cursor = None;
            self.selected_commit = None;
            self.selected_reference = None;
            self.detail = None;
            self.files = Vec::new();
            self.files_state = FilesState::Idle;
            cx.notify();
            return;
        }

        let cursor_root = self.repository_root.clone().unwrap_or_default();
        if load.failures.is_empty() {
            self.commits = load.commits;
            self.cursor = load.next_cursor.map(|cursor| HistoryCursor {
                root: cursor_root,
                cursor,
            });
            self.has_more = load.has_more;
            self.load_state = LoadState::Ready;
        } else if self.commits.is_empty() {
            self.load_state = LoadState::Failed;
            self.has_more = false;
            self.cursor = None;
        } else {
            // 有旧数据 → 横幅（`git-log-tool-window.tsx:589-600`），列表保持不动。
            self.load_state = LoadState::Stale;
            self.has_more = false;
            self.cursor = None;
        }

        // 换仓库 / 换引用后旧的选中下标可能越界，统一清掉再选第一条。
        self.selected_commit = None;
        self.selected_reference = self
            .selected_reference
            .filter(|index| *index < self.references.len());
        cx.notify();

        if !self.commits.is_empty() {
            self.select_commit(0, cx);
        }
    }

    /// 追加一页（「加载更多提交」）。
    fn load_more(&mut self, cx: &mut Context<Self>) {
        let Some(cursor_text) = self.cursor.as_ref().map(|cursor| cursor.cursor.clone()) else {
            return;
        };
        let Some(root) = self.repository_root.clone() else {
            return;
        };
        if self.loading_more {
            return;
        }

        let reference = self.selected_reference_full_name();
        self.loading_more = true;
        let this = cx.entity().downgrade();
        cx.notify();

        cx.spawn(async move |_this, cx| {
            let more = cx
                .background_spawn(async move { load_more(&root, reference, &cursor_text) })
                .await;
            let _ = this.update(cx, |pane, cx| pane.apply_more(more, cx));
        })
        .detach();
    }

    fn apply_more(&mut self, more: MoreLoad, cx: &mut Context<Self>) {
        self.loading_more = false;
        match more.failure {
            // 失败时 Core 已经停掉 session（`git/history.rs:258-261`），游标作废。
            Some(_) => {
                self.has_more = false;
                self.cursor = None;
            }
            None => {
                let cursor_root = self.repository_root.clone().unwrap_or_default();
                self.commits.extend(more.commits);
                self.cursor = more.next_cursor.map(|cursor| HistoryCursor {
                    root: cursor_root,
                    cursor,
                });
                self.has_more = more.has_more;
            }
        }
        cx.notify();
    }

    /// 选中一行提交：填详情 + 后台读 `git.commitFiles`。
    /// Windows 的选中会同时驱动 Inspector（`git-log-tool-window.tsx:685-704`）。
    fn select_commit(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(commit) = self.commits.get(index).cloned() else {
            return;
        };

        self.selected_commit = Some(index);
        self.detail = Some(Detail {
            subject: commit.subject.clone(),
            short_hash: commit.short_hash.clone(),
            author: commit.author.clone(),
            email: commit.email.clone(),
            date: commit.date.clone(),
            decorations: SharedString::from(
                commit
                    .labels
                    .iter()
                    .map(|label| label.title.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            ),
            hash: commit.hash.clone(),
        });
        self.files = Vec::new();
        self.files_state = FilesState::Loading;
        cx.notify();

        let root = self
            .repository_root
            .clone()
            .unwrap_or_else(|| self.root.to_string_lossy().to_string());
        let hash = commit.hash.to_string();
        let this = cx.entity().downgrade();

        cx.spawn(async move |_this, cx| {
            let loaded = cx
                .background_spawn(async move { load_commit_files(&root, &hash) })
                .await;
            let _ = this.update(cx, |pane, cx| pane.apply_commit_files(loaded, cx));
        })
        .detach();
    }

    fn apply_commit_files(
        &mut self,
        loaded: Result<Vec<(String, String)>, String>,
        cx: &mut Context<Self>,
    ) {
        match loaded {
            Ok(files) => {
                self.files = build_commit_files(&files);
                self.files_state = FilesState::Ready;
            }
            Err(_) => {
                self.files = Vec::new();
                self.files_state = FilesState::Failed;
            }
        }
        cx.notify();
    }

    /// 选中/清空引用。选中后按该引用重查历史（Windows `selectReference` + 刷新）。
    fn select_reference(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        if self.selected_reference == index {
            return;
        }
        self.selected_reference = index;
        self.selected_commit = None;
        self.detail = None;
        self.files = Vec::new();
        self.files_state = FilesState::Idle;
        self.spawn_first_load(cx);
    }

    fn selected_reference_full_name(&self) -> Option<String> {
        self.selected_reference
            .and_then(|index| self.references.get(index))
            .map(|reference| reference.full_name.to_string())
    }

    fn toggle_section(&mut self, kind: RefKind, cx: &mut Context<Self>) {
        if !self.collapsed_sections.remove(kind.id()) {
            self.collapsed_sections.insert(kind.id());
        }
        cx.notify();
    }

    fn toggle_group(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.collapsed_groups.remove(id) {
            self.collapsed_groups.insert(id.to_string());
        }
        cx.notify();
    }

    /// 可见的提交下标（筛选之后）。Windows 的 `visibleRows`（`git-commit-table.tsx:105-108`）。
    fn visible_commits(&self) -> Vec<usize> {
        self.commits
            .iter()
            .enumerate()
            .filter(|(_, commit)| matches_filter(commit, &self.filter_query, self.filter_scope))
            .map(|(index, _)| index)
            .collect()
    }

    /// 可见引用：`showMyBranchesOnly` 只保留当前分支，其余原样
    /// （`filterGitLogReferences`，`git-reference-tree.tsx:756-762`）。
    fn visible_reference(&self, index: usize) -> bool {
        if !self.show_my_branches_only {
            return true;
        }
        self.references
            .get(index)
            .is_some_and(|reference| reference.kind == RefKind::Local && reference.is_current)
    }

    // -----------------------------------------------------------------------
    // 自绘基元
    // -----------------------------------------------------------------------

    /// 一个自绘图标按钮：24×24 命中区、14px 图标、悬停 `accent` 底。
    ///
    /// 偏差 5：不用 `Button`（它的图标会被算成 18px），所以悬停提示得自己挂 ——
    /// `div` 上的 `.tooltip(..)` 收的是"构造 tooltip 的闭包"而不是文案
    /// （`gpui-pre-0.3.6/src/elements/div.rs:1676-1685`），这里用组件自带的文本 tooltip
    /// [`Tooltip`] 包成 `AnyView`；无障碍名继续用同一份文案（`aria_label`）。
    fn icon_button(
        id: (&'static str, usize),
        icon: IconName,
        label: SharedString,
        enabled: bool,
        handler: ButtonHandler,
        cx: &App,
    ) -> impl IntoElement {
        let color = if enabled {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };
        let tooltip = label.clone();

        div()
            .id(id)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size_6()
            .rounded(px(ICON_BUTTON_RADIUS))
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .aria_label(label)
            .when(enabled, |this| {
                this.hover(|style| style.bg(cx.theme().accent))
                    .on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when(!enabled, |this| this.opacity(0.4))
            .child(Icon::new(icon).size_3p5().text_color(color))
    }

    /// 引用树工具栏按钮：32×32、图标 16、圆角 4.8（`git-reference-tree.tsx:146`）。
    ///
    /// 悬停提示与无障碍名的挂法与 [`Self::icon_button`] 相同（同样是自绘 `div`，不是 `Button`）。
    fn toolbar_button(
        id: (&'static str, usize),
        icon: IconName,
        label: SharedString,
        enabled: bool,
        handler: ButtonHandler,
        cx: &App,
    ) -> impl IntoElement {
        let color = if enabled {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };
        let tooltip = label.clone();
        div()
            .id(id)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size_8()
            .rounded(px(REFERENCE_TOOLBAR_BUTTON_RADIUS))
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .aria_label(label)
            .when(enabled, |this| {
                this.hover(|style| style.bg(cx.theme().accent))
                    .on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when(!enabled, |this| this.opacity(0.3))
            .child(Icon::new(icon).size_4().text_color(color))
    }

    /// 标题栏。`git-log-title-bar.tsx:28-72`。
    fn title_bar(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        // 引用名：选中引用 → 该引用短名；没有选中但 `git.status` 给了分支 → 分支名；
        // 两者都没有才是 `全部`（`git.log.all`，`locale.ts:7188`）。
        let reference_name = self
            .selected_reference
            .and_then(|index| self.references.get(index))
            .map(|reference| reference.short_name.clone())
            .or_else(|| self.branch.clone())
            .unwrap_or_else(|| tr("lithe.git.log.all"));
        // `日志：{name}`（`git.log.logLabel`，`locale.ts:7189`）。
        let pill_label = tr_args(
            "lithe.git.log.logLabel",
            &[("name", reference_name.as_ref())],
        );

        let show_all: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| pane.select_reference(None, cx));
            })
        };
        let refresh: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| pane.refresh(cx));
            })
        };
        let hide: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| pane.set_visible(false, cx));
            })
        };
        let settings: ButtonHandler = handler(|_event, _window, _cx| {});

        h_flex()
            .w_full()
            .flex_shrink_0()
            .h_8()
            .items_center()
            .gap_2()
            .px_2()
            .border_b_1()
            .border_color(cx.theme().border)
            // 底色 `bg-surface`（`git-log-title-bar.tsx:28`）；主题里最接近的是 `tab_bar`。
            .bg(cx.theme().tab_bar)
            .text_sm()
            .child(
                Icon::new(IconName::GitBranch)
                    .size_3p5()
                    .text_color(cx.theme().muted_foreground),
            )
            // `workbench.gitLog` = 提交记录（`locale.ts:5910`）。
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .child(tr("lithe.workbench.gitLog")),
            )
            // 「引用名」胶囊：点击 = 显示全部引用（`git.log.showAll`，`locale.ts:7190`）。
            .child(
                div()
                    .id("bottom-git-reference-pill")
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .h_6()
                    .max_w(px(REFERENCE_PILL_MAX_WIDTH))
                    .px_2()
                    .rounded(px(REFERENCE_PILL_RADIUS))
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .font_weight(FontWeight::MEDIUM)
                    .hover(|style| style.bg(cx.theme().accent))
                    .aria_label(tr("lithe.git.log.showAll"))
                    .on_click(move |event, window, cx| show_all(event, window, cx))
                    .child(div().min_w_0().text_ellipsis().child(pill_label)),
            )
            // 刷新（`git.log.refresh`，`locale.ts:7191`）。
            .child(Self::icon_button(
                ("bottom-git-refresh", 0),
                IconName::RotateCw,
                tr("lithe.git.log.refresh"),
                self.load_state != LoadState::Loading,
                refresh,
                cx,
            ))
            // 设置（`git.log.settings`，`locale.ts:7219`）：设置面板不属本步范围 → 禁用。
            .child(Self::icon_button(
                ("bottom-git-settings", 0),
                IconName::Settings,
                tr("lithe.git.log.settings"),
                false,
                settings,
                cx,
            ))
            .child(div().flex_1())
            // `footer.readOnly` = 只读（`locale.ts:7309`）。
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.footer.readOnly")),
            )
            // 隐藏（`git.log.hide`，`locale.ts:7192`）。
            .child(Self::icon_button(
                ("bottom-git-hide", 0),
                IconName::Minus,
                tr("lithe.git.log.hide"),
                true,
                hide,
                cx,
            ))
    }

    /// 页签行。`git-log-tool-window.tsx:582-587`（12px、gap 16、px 12、py 4，**没有**选中底色）。
    fn tab_row(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let mut row = h_flex()
            .w_full()
            .flex_shrink_0()
            .h_6()
            .items_center()
            .gap_4()
            .px_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .text_xs();

        // `git.console.log` = 日志（`locale.ts:4556`）、`git.console.title` = 控制台（`locale.ts:4555`）。
        for (panel, label) in [
            (Panel::Log, tr("lithe.git.console.log")),
            (Panel::Console, tr("lithe.git.console.title")),
        ] {
            let selected = self.panel == panel;
            let this = this.clone();
            row = row.child(
                div()
                    // 稳定 id 由 `Panel` 决定（见 [`panel_element_id`]），不是循环下标。
                    .id(panel_element_id(panel))
                    .flex_shrink_0()
                    .aria_selected(selected)
                    // 偏差 4：源码没有选中视觉，这里只补前景色区分。
                    .text_color(if selected {
                        cx.theme().foreground
                    } else {
                        cx.theme().muted_foreground
                    })
                    .hover(|style| style.text_color(cx.theme().foreground))
                    .on_click(move |_event, _window, cx: &mut App| {
                        let _ = this.update(cx, |pane, cx| {
                            if pane.panel != panel {
                                pane.panel = panel;
                                cx.notify();
                            }
                        });
                    })
                    .child(label),
            );
        }

        row
    }

    /// 字段下拉按钮。`git-commit-table.tsx:229-238`（h 24、圆角 6.4、px 6）。
    ///
    /// 不用 `cx`：外观全部来自 `Button` 自己的主题样式（所以这里收不到 `&App`）。
    fn scope_button(&self, this: &WeakEntity<Self>) -> impl IntoElement {
        let current = self.filter_scope;
        let this = this.clone();

        Button::new("bottom-git-filter-field")
            .ghost()
            .with_size(px(ICON_BUTTON_SIZE))
            .h_6()
            .px_1p5()
            .label(current.label())
            // `git.log.filterField` = 「Git 日志筛选字段」（`locale.ts:7278`）：曾经硬编码中文，
            // 现在与其它文案一样走 `tr(..)`（键早就在 locale 与 WIRED 表里）。
            .tooltip(tr("lithe.git.log.filterField"))
            .dropdown_menu(move |menu, _window, _cx| {
                let mut menu = menu;
                for scope in FilterScope::all() {
                    let this = this.clone();
                    menu = menu.item(
                        PopupMenuItem::new(scope.label())
                            .checked(scope == current)
                            .on_click(move |_event, window: &mut Window, cx: &mut App| {
                                let _ = this.update(cx, |pane, cx| {
                                    pane.filter_scope = scope;
                                    // 占位文案跟着字段变（`git.log.filterPlaceholder`，`locale.ts:7282`）。
                                    let placeholder = scope.placeholder();
                                    pane.filter.update(cx, |state, cx| {
                                        state.set_placeholder(placeholder, window, cx)
                                    });
                                    cx.notify();
                                });
                            }),
                    );
                }
                menu
            })
    }

    /// 筛选条。`git-commit-table.tsx:200-253`。
    fn filter_row(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let visible = self.visible_commits().len();
        let total = self.commits.len();

        // 装饰开关（`git.log.showDecorations` / `hideDecorations`，`locale.ts:7283-7284`）。
        let toggle_decorations: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| {
                    pane.show_decorations = !pane.show_decorations;
                    cx.notify();
                });
            })
        };

        h_flex()
            .w_full()
            .flex_shrink_0()
            .h_8()
            .items_center()
            .gap_2()
            .px_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .child(
                Input::new(&self.filter)
                    .cleanable(true)
                    .small()
                    .flex_1()
                    .min_w(px(FILTER_INPUT_MIN_WIDTH))
                    .max_w(px(FILTER_INPUT_MAX_WIDTH))
                    .prefix(
                        Icon::new(IconName::Search)
                            .size_3p5()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            .child(self.scope_button(this))
            .child(Self::icon_button(
                ("bottom-git-decorations", 0),
                if self.show_decorations {
                    IconName::Eye
                } else {
                    IconName::EyeOff
                },
                if self.show_decorations {
                    tr("lithe.git.log.hideDecorations")
                } else {
                    tr("lithe.git.log.showDecorations")
                },
                true,
                toggle_decorations,
                cx,
            ))
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(cx.theme().muted_foreground)
                    .child(SharedString::from(format!("{visible}/{total}"))),
            )
    }

    /// 表头行：`提交 / 作者 / 日期`。出处：`git-commit-table.tsx:255-259`。
    fn commit_header(cx: &App) -> Div {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .h_6()
            .items_center()
            .px_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            // `git.log.commit` / `author` / `date`（`locale.ts:7285-7287`）。
            // 表头「提交」只占 `flex-1`，**没有**预留泳道图宽度（源码就是不对齐的，
            // 见 `research/windows/03-git-and-bottom.md` §2.3 的注）。
            .child(div().min_w_0().flex_1().child(tr("lithe.git.log.commit")))
            .child(
                div()
                    .w(px(AUTHOR_COLUMN_WIDTH))
                    .flex_shrink_0()
                    .child(tr("lithe.git.log.author")),
            )
            .child(
                h_flex()
                    .w_32()
                    .flex_shrink_0()
                    .justify_end()
                    .child(tr("lithe.git.log.date")),
            )
    }

    /// 泳道图单元格（简化版，偏差 2）。
    fn graph_cell(row: &GraphRow, cx: &App) -> Div {
        let half = COMMIT_ROW_HEIGHT / 2.;

        h_flex()
            .h_full()
            .flex_shrink_0()
            .min_w(px(GRAPH_MIN_WIDTH))
            .pl_2()
            .pr_2()
            .children(row.lanes.iter().enumerate().map(move |(lane, color)| {
                let lane_width = px(GRAPH_LANE_GAP);
                let line_width = px(GRAPH_LINE_WIDTH);
                let edge_here = row.edges.iter().find(|(target, _, _)| *target == lane);

                match (lane == row.lane, color) {
                    // 本提交所在泳道：上半段线 + 节点 + 下半段线。
                    (true, _) => v_flex()
                        .w(lane_width)
                        .h_full()
                        .items_center()
                        .child(
                            div()
                                .w(line_width)
                                .h(px(half - GRAPH_NODE_RADIUS))
                                .bg(lane_color(row.node_color, cx)),
                        )
                        .child(
                            div()
                                .size(px(GRAPH_NODE_RADIUS * 2.))
                                .flex_shrink_0()
                                .rounded_full()
                                .bg(cx.theme().background)
                                .border_2()
                                .border_color(lane_color(row.node_color, cx)),
                        )
                        .child(
                            div()
                                .w(line_width)
                                .flex_1()
                                .when(!row.edges.is_empty(), |this| {
                                    this.bg(lane_color(row.node_color, cx))
                                }),
                        )
                        .into_any_element(),
                    // 过路线：整条竖线。
                    (false, Some(index)) => v_flex()
                        .w(lane_width)
                        .h_full()
                        .items_center()
                        .child(div().w(line_width).h_full().bg(lane_color(*index, cx)))
                        .into_any_element(),
                    // 只被父边指到、本行上半没有线的泳道：补下半段。
                    (false, None) => v_flex()
                        .w(lane_width)
                        .h_full()
                        .items_center()
                        .child(div().flex_1())
                        .when_some(edge_here, |this, (_, index, missing)| {
                            this.child(
                                div()
                                    .w(line_width)
                                    .h(px(half))
                                    .opacity(if *missing { 0.7 } else { 1.0 })
                                    .bg(lane_color(*index, cx)),
                            )
                        })
                        .into_any_element(),
                }
            }))
    }

    /// 一个标签徽章（`git-graph-row.tsx:87-88`）。
    fn label_badge(label: &Label, cx: &App) -> Div {
        let color = label_color(label.kind, cx);
        h_flex()
            .h(px(LABEL_FONT_SIZE + LABEL_PADDING_Y * 2.))
            .max_w(px(LABEL_MAX_WIDTH))
            .flex_shrink_0()
            .items_center()
            .px_1p5()
            .rounded(px(LABEL_RADIUS))
            .border_1()
            .border_color(color.opacity(0.45))
            .bg(color.opacity(0.2))
            .text_size(px(LABEL_FONT_SIZE))
            .text_color(color)
            .whitespace_nowrap()
            .child(div().min_w_0().text_ellipsis().child(label.title.clone()))
    }

    /// 一行提交（自绘，`git-commit-table.tsx:301-326`）。
    ///
    /// ⚠️ **不要给提交信息加 `overflow_hidden`**（`gpui/BLOCKERS.md` B4：提交信息文字顶部被切掉约
    /// 1/4 行高）。这里只用 `text_ellipsis()` + `min_h`，与上一轮实现的定案一致。
    fn commit_row(
        &self,
        index: usize,
        graph: &GraphRow,
        commit: &Commit,
        selected: bool,
        this: &WeakEntity<Self>,
        cx: &App,
    ) -> impl IntoElement {
        let hover_bg = if selected {
            cx.theme().primary.opacity(0.28)
        } else {
            cx.theme().accent.opacity(0.7)
        };
        let this = this.clone();

        h_flex()
            // 稳定 id 由提交哈希决定（见 [`commit_element_id`]）；`index` 只用于"点这一行选中
            // 它"这条位置语义的交互，不参与元素身份。
            .id(commit_element_id(commit))
            .w_full()
            .min_w(px(COMMIT_CONTENT_MIN_WIDTH))
            .min_h(px(COMMIT_ROW_HEIGHT))
            .items_center()
            .px_1()
            .border_b_1()
            .border_color(cx.theme().border.opacity(0.5))
            .whitespace_nowrap()
            .when(selected, |row| row.bg(cx.theme().primary.opacity(0.22)))
            .hover(move |style| style.bg(hover_bg))
            .on_click(move |_event, _window, cx: &mut App| {
                let _ = this.update(cx, |pane, cx| pane.select_commit(index, cx));
            })
            .child(Self::graph_cell(graph, cx))
            // 标签 + 提交说明：`flex min-w-0 flex-1 items-center gap-1.5`。
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap_1p5()
                    .when(self.show_decorations, |labels| {
                        labels.children(
                            commit
                                .labels
                                .iter()
                                .map(|label| Self::label_badge(label, cx)),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .min_h(px(COMMIT_ROW_HEIGHT))
                            .text_sm()
                            .line_height(px(COMMIT_ROW_HEIGHT))
                            .child(commit.subject.clone()),
                    ),
            )
            .child(
                div()
                    .w(px(AUTHOR_COLUMN_WIDTH))
                    .flex_shrink_0()
                    .px_2()
                    .text_ellipsis()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(commit.author.clone()),
            )
            .child(
                h_flex()
                    .w_32()
                    .flex_shrink_0()
                    .justify_end()
                    .text_size(px(DATE_FONT_SIZE))
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(cx.theme().muted_foreground)
                    .child(commit.date.clone()),
            )
    }

    /// 提交栏。`git-commit-table.tsx:198-446`。
    fn commit_pane(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let visible = self.visible_commits();
        let total = self.commits.len();
        let graphs = layout_graph(&self.commits);

        let mut list = v_flex().w_full().min_w(px(COMMIT_CONTENT_MIN_WIDTH));

        if visible.is_empty() {
            let message = if total == 0 {
                // `git.log.noCommits` = 此视图中没有提交（`locale.ts:7289`）。
                tr("lithe.git.log.noCommits")
            } else {
                // `git.log.noMatch` = 没有符合筛选条件的提交（`locale.ts:7288`）。
                tr("lithe.git.log.noMatch")
            };
            list = list.child(
                h_flex()
                    .w_full()
                    .min_h(px(COMMIT_ROW_HEIGHT * 4.))
                    .items_center()
                    .justify_center()
                    .text_color(cx.theme().muted_foreground)
                    .child(message),
            );
        } else {
            let empty_graph = GraphRow {
                lanes: Vec::new(),
                lane: 0,
                node_color: 0,
                edges: Vec::new(),
            };
            for index in visible {
                let Some(commit) = self.commits.get(index) else {
                    continue;
                };
                let graph = graphs.get(index).unwrap_or(&empty_graph);
                list = list.child(self.commit_row(
                    index,
                    graph,
                    commit,
                    self.selected_commit == Some(index),
                    this,
                    cx,
                ));
            }
        }

        // 「加载更多提交」行（`git-commit-table.tsx:428-443`）。
        if self.has_more {
            let this = this.clone();
            list = list.child(
                h_flex()
                    .w_full()
                    .min_w(px(COMMIT_CONTENT_MIN_WIDTH))
                    .h_9()
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("bottom-git-load-more")
                            .ghost()
                            .with_size(px(LOAD_MORE_BUTTON_HEIGHT))
                            .label(if self.loading_more {
                                // `git.log.loadingCommits` = 正在加载提交…（`locale.ts:7294`）。
                                tr("lithe.git.log.loadingCommits")
                            } else {
                                // `git.log.loadMore` = 加载更多提交（`locale.ts:7295`）。
                                tr("lithe.git.log.loadMore")
                            })
                            .disabled(self.loading_more)
                            .on_click(move |_event, _window, cx: &mut App| {
                                let _ = this.update(cx, |pane, cx| pane.load_more(cx));
                            }),
                    ),
            );
        }

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_sm()
            .child(self.filter_row(this, cx))
            .child(Self::commit_header(cx))
            .child(
                // 滚动容器必须有 `id`：`overflow_y_scroll` 是 `StatefulInteractiveElement` 的方法
                // （`gpui-pre-0.3.6/src/elements/div.rs:1300,1529`），而 `StatefulInteractiveElement`
                // 只对 `Stateful<Div>` 实现（`:4074`）；`id` 同时让滚动偏移跨帧保留。
                div()
                    .id("bottom-git-commit-scroll")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scroll()
                    .child(list),
            )
    }

    /// 引用树左侧竖排工具栏。`git-reference-tree.tsx:118-361`。
    ///
    /// 偏差 8：Windows 的 10 个动作里只有「全部展开 / 全部折叠 / 只显示我的分支」是纯 UI，
    /// 这里只画这三个（另一个「新建分支」按禁用态保留位置）。
    fn reference_toolbar(pane: &Self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let has_references = !pane.references.is_empty();
        let has_my_branches = pane
            .references
            .iter()
            .any(|reference| reference.kind == RefKind::Local && reference.is_current);
        let show_my_branches_only = pane.show_my_branches_only;

        let expand: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| {
                    pane.collapsed_sections.clear();
                    pane.collapsed_groups.clear();
                    cx.notify();
                });
            })
        };
        let collapse: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| {
                    let mut groups = Vec::new();
                    for kind in RefKind::sections() {
                        pane.collapsed_sections.insert(kind.id());
                        for row in build_reference_rows(&pane.references, kind) {
                            if row.reference.is_none() {
                                groups.push(row.id);
                            }
                        }
                    }
                    pane.collapsed_groups.extend(groups);
                    cx.notify();
                });
            })
        };
        let toggle_mine: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| {
                    pane.show_my_branches_only = !pane.show_my_branches_only;
                    cx.notify();
                });
            })
        };
        let create_branch: ButtonHandler = handler(|_event, _window, _cx| {});

        v_flex()
            .w_9()
            .h_full()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .py_1()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar.opacity(0.6))
            .child(Self::toolbar_button(
                ("bottom-git-ref-expand", 0),
                IconName::UnfoldVertical,
                tr("lithe.git.expandAll"),
                has_references,
                expand,
                cx,
            ))
            .child(Self::toolbar_button(
                ("bottom-git-ref-collapse", 0),
                IconName::FoldVertical,
                tr("lithe.git.collapseAll"),
                has_references,
                collapse,
                cx,
            ))
            .child(
                div()
                    .w_5()
                    .h_px()
                    .flex_shrink_0()
                    .my_1()
                    .bg(cx.theme().border),
            )
            .child(Self::toolbar_button(
                ("bottom-git-ref-mine", 0),
                IconName::ListFilter,
                if show_my_branches_only {
                    tr("lithe.git.log.toolbar.showAllBranches")
                } else {
                    tr("lithe.git.log.toolbar.showMyBranches")
                },
                has_my_branches || show_my_branches_only,
                toggle_mine,
                cx,
            ))
            .child(div().flex_1())
            // 新建分支要 `git.write`（不在 6 条命令里）→ 禁用占位。
            .child(Self::toolbar_button(
                ("bottom-git-ref-create", 0),
                IconName::Plus,
                tr("lithe.git.log.toolbar.newBranch"),
                false,
                create_branch,
                cx,
            ))
    }

    /// 引用树的一行（`git-reference-tree.tsx:596-660`）。
    #[allow(clippy::too_many_arguments)]
    fn reference_row(
        row_id: String,
        depth: usize,
        name: SharedString,
        is_group: bool,
        selected: bool,
        disclosure: Option<AnyElement>,
        icon: Option<IconName>,
        reference: Option<(usize, Option<(usize, usize)>, bool)>,
        this: &WeakEntity<Self>,
        cx: &App,
    ) -> impl IntoElement {
        let this = this.clone();
        let index = reference.map(|(index, _, _)| index);
        let tracking = reference.and_then(|(_, tracking, _)| tracking);
        let is_current = reference.is_some_and(|(_, _, current)| current);

        let row = h_flex()
            .id(SharedString::from(format!("bottom-git-ref-row:{row_id}")))
            .w_full()
            .min_w_0()
            .h_6()
            .items_center()
            .gap_1p5()
            .pl(px(
                REFERENCE_INDENT_BASE + depth as f32 * REFERENCE_INDENT_STEP
            ))
            .rounded(px(REFERENCE_ROW_RADIUS))
            .when(selected, |row| row.bg(cx.theme().accent))
            .when(is_current, |row| {
                row.font_weight(FontWeight::SEMIBOLD)
                    .text_color(cx.theme().yellow_light)
            })
            .hover(|style| style.bg(cx.theme().accent.opacity(0.8)))
            .when_some(index, |row, index| {
                row.on_click(move |_event, _window, cx: &mut App| {
                    let _ = this.update(cx, |pane, cx| pane.select_reference(Some(index), cx));
                })
            });

        let row = match disclosure {
            Some(disclosure) => row.child(disclosure),
            None => row.child(div().size_3p5().flex_shrink_0()),
        };

        let row = match icon {
            Some(icon) => row.child(Icon::new(icon).size_3p5().text_color(if is_current {
                cx.theme().yellow_light
            } else {
                cx.theme().muted_foreground
            })),
            None if is_group => row.child(
                Icon::new(IconName::Folder)
                    .size_3p5()
                    .text_color(cx.theme().muted_foreground),
            ),
            None => row,
        };

        let mut row = row
            .child(div().min_w_0().text_ellipsis().child(name))
            .child(div().flex_1());

        // ahead / behind 计数（`git-reference-tree.tsx:641-650`、`git-tracking-counts.tsx:36-53`）。
        if let Some((ahead, behind)) = tracking {
            let mut counts = h_flex()
                .flex_shrink_0()
                .items_center()
                .gap_1()
                .text_size(px(TRACKING_COUNT_FONT_SIZE));
            if behind > 0 {
                counts = counts.child(
                    div()
                        .text_color(cx.theme().info)
                        .child(SharedString::from(format!("↙{}", tracking_count(behind)))),
                );
            }
            if ahead > 0 {
                counts = counts.child(
                    div()
                        .text_color(cx.theme().success)
                        .child(SharedString::from(format!("↗{}", tracking_count(ahead)))),
                );
            }
            row = row.child(counts);
        }

        if is_current {
            // `git.current` = 当前（`locale.ts:6948`）。
            row = row.child(
                h_flex()
                    .flex_shrink_0()
                    .px_1()
                    .rounded(px(REFERENCE_SECTION_RADIUS))
                    .bg(cx.theme().yellow_light.opacity(0.12))
                    .text_size(px(REFERENCE_BADGE_FONT_SIZE))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().yellow_light)
                    .child(tr("lithe.git.current")),
            );
        }

        row
    }

    /// 引用栏。`git-reference-tree.tsx:803-928`。
    fn reference_pane(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let current = self
            .references
            .iter()
            .find(|reference| reference.is_current)
            .cloned();
        let visible_count = self
            .references
            .iter()
            .enumerate()
            .filter(|(index, _)| self.visible_reference(*index))
            .count();

        let mut body = v_flex().w_full().gap_1();

        // HEAD 行（`git-reference-tree.tsx:849-865`）。
        let current_index = self
            .references
            .iter()
            .position(|reference| reference.is_current);
        let head_selected =
            self.selected_reference.is_some() && self.selected_reference == current_index;
        let head = {
            let this = this.clone();
            h_flex()
                .id("bottom-git-head-row")
                .w_full()
                .h_7()
                .mb_1()
                .items_center()
                .gap_2()
                .px_2()
                .rounded(px(REFERENCE_HEAD_ROW_RADIUS))
                .font_weight(FontWeight::MEDIUM)
                .when(head_selected, |row| row.bg(cx.theme().accent))
                .hover(|style| style.bg(cx.theme().accent.opacity(0.8)))
                .on_click(move |_event, _window, cx: &mut App| {
                    let _ = this.update(cx, |pane, cx| pane.select_reference(current_index, cx));
                })
                .child(div().text_color(cx.theme().primary).child("→"))
                // `git.log.headCurrentBranch` = HEAD（当前分支）（`locale.ts:7220`）。
                .child(
                    div()
                        .min_w_0()
                        .text_ellipsis()
                        .child(tr("lithe.git.log.headCurrentBranch")),
                )
                .when_some(current, |row, reference| {
                    row.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .text_color(cx.theme().muted_foreground)
                            .child(reference.short_name),
                    )
                })
        };
        body = body.child(head);

        // 三个分区（本地 / 远程 / 标签）。
        for kind in RefKind::sections() {
            let collapsed = self.collapsed_sections.contains(kind.id());
            let rows = build_reference_rows(&self.references, kind);

            let mut section = v_flex().w_full();
            let header = {
                let this = this.clone();
                h_flex()
                    .id(("bottom-git-ref-section", kind as usize))
                    .w_full()
                    .h_6()
                    .items_center()
                    .gap_1p5()
                    .px_1p5()
                    .rounded(px(REFERENCE_SECTION_RADIUS))
                    .font_weight(FontWeight::MEDIUM)
                    .hover(|style| style.bg(cx.theme().accent.opacity(0.8)))
                    .on_click(move |_event, _window, cx: &mut App| {
                        let _ = this.update(cx, |pane, cx| pane.toggle_section(kind, cx));
                    })
                    .child(
                        Icon::new(if collapsed {
                            IconName::ChevronRight
                        } else {
                            IconName::ChevronDown
                        })
                        .size_3()
                        .text_color(cx.theme().muted_foreground),
                    )
                    .child(kind.title())
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_color(cx.theme().muted_foreground)
                            .child(SharedString::from(
                                self.references
                                    .iter()
                                    .enumerate()
                                    .filter(|(index, reference)| {
                                        reference.kind == kind && self.visible_reference(*index)
                                    })
                                    .count()
                                    .to_string(),
                            )),
                    )
            };
            section = section.child(header);

            if !collapsed {
                if rows.is_empty() {
                    // `git.log.none` = 无（`locale.ts:7255`）。
                    section = section.child(
                        div()
                            .h_6()
                            .pl_8()
                            // 行高 = 24（`h-6`）：`line_height` 没有档位 helper，
                            // 用 helper 底层的 `rems()`，`rems(1.5)` = 24px，与 `h_6()` 同值。
                            .line_height(rems(1.5))
                            .text_color(cx.theme().muted_foreground)
                            .child(tr("lithe.git.log.none")),
                    );
                } else {
                    for row in rows {
                        if let Some(index) = row.reference {
                            if !self.visible_reference(index) {
                                continue;
                            }
                        }
                        let is_group = row.reference.is_none();
                        let selected =
                            row.reference.is_some() && row.reference == self.selected_reference;
                        let reference = row
                            .reference
                            .and_then(|index| self.references.get(index))
                            .cloned();
                        let collapsed_group = self.collapsed_groups.contains(&row.id);

                        let disclosure = if is_group {
                            let this = this.clone();
                            let id = row.id.clone();
                            Some(
                                div()
                                    .id(SharedString::from(format!("bottom-git-ref-group:{id}")))
                                    .size_3p5()
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .on_click(move |_event, _window, cx: &mut App| {
                                        let id = id.clone();
                                        let _ =
                                            this.update(cx, |pane, cx| pane.toggle_group(&id, cx));
                                    })
                                    .child(
                                        Icon::new(if collapsed_group {
                                            IconName::ChevronRight
                                        } else {
                                            IconName::ChevronDown
                                        })
                                        .size_3()
                                        .text_color(cx.theme().muted_foreground),
                                    )
                                    .into_any_element(),
                            )
                        } else {
                            None
                        };

                        if is_group && collapsed_group {
                            section = section.child(Self::reference_row(
                                row.id, row.depth, row.name, true, selected, disclosure, None,
                                None, this, cx,
                            ));
                            continue;
                        }
                        if is_group {
                            section = section.child(Self::reference_row(
                                row.id, row.depth, row.name, true, selected, disclosure, None,
                                None, this, cx,
                            ));
                            continue;
                        }

                        let reference_index = row.reference;
                        let is_current = reference
                            .as_ref()
                            .is_some_and(|reference| reference.is_current);
                        let tracking = reference.as_ref().and_then(|reference| {
                            reference
                                .upstream_short_name
                                .as_ref()
                                .map(|_| (reference.ahead, reference.behind))
                        });
                        let icon = if is_current {
                            IconName::Check
                        } else if let Some(reference) = reference.as_ref() {
                            match reference.kind {
                                RefKind::Tag => IconName::Tag,
                                RefKind::Remote => IconName::Network,
                                RefKind::Local => IconName::GitBranch,
                            }
                        } else {
                            IconName::GitBranch
                        };

                        section = section.child(Self::reference_row(
                            row.id,
                            row.depth,
                            row.name,
                            false,
                            selected,
                            None,
                            Some(icon),
                            reference_index.map(|index| (index, tracking, is_current)),
                            this,
                            cx,
                        ));
                    }
                }
            }

            body = body.child(section);
        }

        let reference_label = self
            .selected_reference
            .and_then(|index| self.references.get(index))
            .map(|reference| reference.short_name.clone());

        h_flex()
            .size_full()
            .bg(cx.theme().tab_bar.opacity(0.45))
            .text_sm()
            .child(Self::reference_toolbar(self, this, cx))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    // 列表头：`git.log.references` = 引用（`locale.ts:7207`）。
                    .child(
                        h_flex()
                            .w_full()
                            .flex_shrink_0()
                            .h_8()
                            .items_center()
                            .px_2()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .text_color(cx.theme().muted_foreground)
                            .child(tr("lithe.git.log.references"))
                            .child(div().flex_1())
                            .when_some(reference_label, |row, label| {
                                row.child(div().min_w_0().text_ellipsis().child(label))
                            })
                            .child(SharedString::from(visible_count.to_string())),
                    )
                    .child(
                        div()
                            .id("bottom-git-reference-scroll")
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .overflow_y_scroll()
                            .p_1p5()
                            .child(body),
                    ),
            )
    }

    /// Inspector。`git-commit-inspector.tsx:95-195`（上 62% 文件 / 下 38% 详情）。
    fn inspector_pane(&self, cx: &App) -> impl IntoElement {
        let file_count = self.files.iter().filter(|row| !row.is_folder).count();

        v_flex()
            .size_full()
            .bg(cx.theme().tab_bar.opacity(0.35))
            .text_sm()
            .child(
                v_flex()
                    .w_full()
                    .h(relative(INSPECTOR_FILES_FRACTION))
                    .min_h(px(INSPECTOR_FILES_MIN_HEIGHT))
                    .min_w_0()
                    // 文件区表头（`git-commit-inspector.tsx:106-130`）。
                    .child(
                        h_flex()
                            .w_full()
                            .flex_shrink_0()
                            .h_8()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().tab_bar)
                            .text_color(cx.theme().muted_foreground)
                            // `git.log.commitFiles` = 提交文件（`locale.ts:7296`）。
                            .child(tr("lithe.git.log.commitFiles"))
                            .child(div().flex_1())
                            .child(if self.files_state == FilesState::Loading {
                                // `git.log.loadingShort` = 加载中…（`locale.ts:7303`）。
                                tr("lithe.git.log.loadingShort")
                            } else {
                                // `git.log.filesCount` = {count} 个文件（`locale.ts:7297`）。
                                tr_args(
                                    "lithe.git.log.filesCount",
                                    &[("count", &file_count.to_string())],
                                )
                            })
                            // 「打开提交差异」要 `git.diff`（不在 6 条命令里）→ 禁用。
                            .child(Self::icon_button(
                                ("bottom-git-open-diff", 0),
                                IconName::GitCompare,
                                tr("lithe.git.log.openCommitDiff"),
                                false,
                                handler(|_event, _window, _cx| {}),
                                cx,
                            )),
                    )
                    .child(self.commit_files_body(cx)),
            )
            .child(
                v_flex()
                    .id("bottom-git-detail-scroll")
                    .w_full()
                    .h(relative(INSPECTOR_DETAILS_FRACTION))
                    .min_h_20()
                    .min_w_0()
                    .overflow_y_scroll()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .p_3()
                    .gap_2()
                    .child(self.commit_detail_body(cx)),
            )
    }

    /// 提交文件区正文（空 / 加载 / 失败 / 无文件 / 文件树）。`git-commit-inspector.tsx:131-160`。
    fn commit_files_body(&self, cx: &App) -> impl IntoElement {
        let centered = |message: SharedString, color: Hsla| {
            h_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .items_center()
                .justify_center()
                .text_color(color)
                .child(message)
                .into_any_element()
        };

        match self.files_state {
            // `git.log.selectCommit` = 选择一个提交（`locale.ts:7298`）。
            FilesState::Idle => centered(
                tr("lithe.git.log.selectCommit"),
                cx.theme().muted_foreground,
            ),
            // `git.log.loadingChangedFiles` = 正在加载更改的文件…（`locale.ts:7299`）。
            FilesState::Loading => centered(
                tr("lithe.git.log.loadingChangedFiles"),
                cx.theme().muted_foreground,
            ),
            // `git.log.unableToLoadFiles` = 无法加载更改的文件（`locale.ts:7300`）。
            FilesState::Failed => {
                centered(tr("lithe.git.log.unableToLoadFiles"), cx.theme().danger)
            }
            // `git.log.noChangedFiles` = 没有更改的文件（`locale.ts:7301`）。
            FilesState::Ready if self.files.is_empty() => centered(
                tr("lithe.git.log.noChangedFiles"),
                cx.theme().muted_foreground,
            ),
            FilesState::Ready => {
                let mut tree = v_flex().w_full();
                for row in &self.files {
                    tree = tree.child(Self::commit_file_row(row, cx));
                }
                div()
                    .id("bottom-git-files-scroll")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scroll()
                    .p_1p5()
                    .child(tree)
                    .into_any_element()
            }
        }
    }

    /// 提交文件树的一行。`git-commit-file-tree.tsx:120-134,77,127`。
    fn commit_file_row(row: &CommitFileRow, cx: &App) -> Div {
        let status = row.status.chars().next().unwrap_or(' ');
        let status_color = match status {
            'A' => cx.theme().success,
            'D' => cx.theme().danger,
            'R' => cx.theme().primary,
            _ => cx.theme().warning,
        };

        let row_element = h_flex()
            .w_full()
            .min_w_0()
            .h_6()
            .items_center()
            .gap_1p5()
            .pl(px(
                COMMIT_FILE_INDENT_BASE + row.depth as f32 * COMMIT_FILE_INDENT_STEP
            ))
            .pr_1p5()
            .whitespace_nowrap();

        if row.is_folder {
            row_element
                .child(
                    Icon::new(IconName::Folder)
                        .size_3p5()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_ellipsis()
                        .child(row.name.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(COMMIT_FILE_STATUS_FONT_SIZE))
                        .text_color(cx.theme().muted_foreground)
                        .child(tr_args(
                            "lithe.git.log.filesCount",
                            &[("count", &row.file_count.to_string())],
                        )),
                )
        } else {
            row_element
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_ellipsis()
                        .child(row.name.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_size(px(COMMIT_FILE_STATUS_FONT_SIZE))
                        .text_color(status_color)
                        // Core 的 `status` 是 name-status 码（可能是 `R100`），Windows 原样渲染
                        // （`git-commit-file-tree.tsx:127`）；这里取首字母，与 macOS 的
                        // `statusColor` 同一口径（`GitCommitFileTreeView.swift:459-464`）。
                        .child(SharedString::from(status.to_string())),
                )
        }
    }

    /// 提交详情区正文。`git-commit-inspector.tsx:165-190`。
    fn commit_detail_body(&self, cx: &App) -> impl IntoElement {
        let mono = cx.theme().mono_font_family.clone();

        let Some(detail) = self.detail.as_ref() else {
            // `git.log.commitDetails` = 提交详情（`locale.ts:7302`）。
            return h_flex()
                .w_full()
                .flex_1()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child(tr("lithe.git.log.commitDetails"))
                .into_any_element();
        };

        v_flex()
            .w_full()
            .gap_2()
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .child(detail.subject.clone()),
            )
            .child(
                div()
                    .font_family(mono.clone())
                    .text_size(px(INSPECTOR_MONO_FONT_SIZE))
                    .text_color(cx.theme().muted_foreground)
                    .child(SharedString::from(if detail.email.is_empty() {
                        format!("{} · {}", detail.short_hash, detail.author)
                    } else {
                        format!(
                            "{} · {} <{}>",
                            detail.short_hash, detail.author, detail.email
                        )
                    })),
            )
            .child(
                div()
                    .font_family(mono.clone())
                    .text_size(px(INSPECTOR_MONO_FONT_SIZE))
                    .text_color(cx.theme().muted_foreground)
                    .child(detail.date.clone()),
            )
            .when(!detail.decorations.is_empty(), |this| {
                this.child(
                    div()
                        .text_color(cx.theme().primary)
                        .child(detail.decorations.clone()),
                )
            })
            // 完整哈希：Windows 是 `break-all`，gpui 没有这个属性，这里让它自然换行。
            .child(
                div()
                    .font_family(mono)
                    .text_size(px(INSPECTOR_HASH_FONT_SIZE))
                    .text_color(cx.theme().muted_foreground)
                    .child(detail.hash.clone()),
            )
            .into_any_element()
    }

    /// 控制台。`git-execution-console.tsx:68-111`。
    ///
    /// 偏差 6：输出数据源（Git 执行事件 + `git.consolePresentation`）不在本步的 6 条命令里，
    /// 所以左栏按钮全部禁用、正文只画空态。
    fn console_pane(cx: &App) -> impl IntoElement {
        let mut toolbar = v_flex()
            .w_8()
            .h_full()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .py_1()
            .border_r_1()
            .border_color(cx.theme().border);

        // 文案逐字取自 `git.console.find/wrap/scrollToEnd/cancel/clear/copy`
        // （`locale.ts:4473,4493,4494,4557,4558,4559`）。
        for (index, (icon, label)) in [
            (IconName::Search, tr("lithe.git.console.find")),
            (IconName::TextWrap, tr("lithe.git.console.wrap")),
            (IconName::ArrowDown, tr("lithe.git.console.scrollToEnd")),
            (IconName::CircleSlash, tr("lithe.git.console.cancel")),
            (IconName::Trash, tr("lithe.git.console.clear")),
            (IconName::Copy, tr("lithe.git.console.copy")),
        ]
        .into_iter()
        .enumerate()
        {
            toolbar = toolbar.child(Self::icon_button(
                ("bottom-git-console-action", index),
                icon,
                label,
                false,
                handler(|_event, _window, _cx| {}),
                cx,
            ));
        }

        h_flex()
            .size_full()
            .font_family(cx.theme().mono_font_family.clone())
            .text_xs()
            .child(toolbar)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .items_center()
                    .justify_center()
                    .p_3()
                    .text_color(cx.theme().muted_foreground)
                    // `git.console.empty` = Git 命令及其输出将显示在这里。（`locale.ts:4560`）。
                    .child(tr("lithe.git.console.empty")),
            )
    }

    /// 三栏（引用 / 提交 / 提交详情）。比例与最小宽见模块文档的表格。
    ///
    /// 偏差 1：用 `relative()` 百分比而不是 `h_resizable`（理由见模块文档），所以栏间不可拖拽。
    fn log_body(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        h_flex()
            .w_full()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .w(relative(REFERENCE_PANE_FRACTION))
                    .min_w(px(REFERENCE_PANE_MIN_WIDTH))
                    .h_full()
                    .min_h_0()
                    .child(self.reference_pane(this, cx)),
            )
            .child(div().w_px().h_full().flex_shrink_0().bg(cx.theme().border))
            .child(
                div()
                    .w(relative(COMMIT_PANE_FRACTION))
                    .min_w_80()
                    .h_full()
                    .min_h_0()
                    .child(self.commit_pane(this, cx)),
            )
            .child(div().w_px().h_full().flex_shrink_0().bg(cx.theme().border))
            .child(
                div()
                    .w(relative(INSPECTOR_PANE_FRACTION))
                    .min_w(px(INSPECTOR_PANE_MIN_WIDTH))
                    .h_full()
                    .min_h_0()
                    .child(self.inspector_pane(cx)),
            )
    }

    /// 刷新失败横幅。`git-log-tool-window.tsx:589-600`。
    fn banner(text: SharedString, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let this = this.clone();
        h_flex()
            .w_full()
            .flex_shrink_0()
            .h_7()
            .items_center()
            .gap_2()
            .px_2()
            .border_b_1()
            .border_color(cx.theme().danger.opacity(0.3))
            .bg(cx.theme().danger.opacity(0.1))
            .text_sm()
            .text_color(cx.theme().danger)
            .child(div().min_w_0().flex_1().text_ellipsis().child(text))
            // `git.log.retry` = 重试（`locale.ts:7197`）。
            .child(
                Button::new("bottom-git-retry")
                    .ghost()
                    .with_size(px(LOAD_MORE_BUTTON_HEIGHT))
                    .label(tr("lithe.git.log.retry"))
                    .on_click(move |_event, _window, cx: &mut App| {
                        let _ = this.update(cx, |pane, cx| pane.refresh(cx));
                    }),
            )
    }

    /// 居中占位（无仓库 / 加载中 / 失败，`git-log-tool-window.tsx:602-612`）。
    fn centered_notice(
        title: SharedString,
        detail: Option<SharedString>,
        retry: Option<&WeakEntity<Self>>,
        cx: &App,
    ) -> Div {
        let mut block = v_flex()
            .w_full()
            .flex_1()
            .min_h_0()
            .items_center()
            .justify_center()
            .gap_2()
            .text_color(cx.theme().muted_foreground)
            .child(div().text_color(cx.theme().foreground).child(title));

        if let Some(detail) = detail {
            block = block.child(div().child(detail));
        }

        if let Some(this) = retry {
            let this = this.clone();
            block = block.child(
                Button::new("bottom-git-empty-retry")
                    .ghost()
                    .with_size(px(LOAD_MORE_BUTTON_HEIGHT))
                    .label(tr("lithe.git.log.retry"))
                    .on_click(move |_event, _window, cx: &mut App| {
                        let _ = this.update(cx, |pane, cx| pane.refresh(cx));
                    }),
            );
        }

        block
    }
}

impl Drop for BottomPane {
    fn drop(&mut self) {
        // 面板销毁（底部窗被拆掉 / 应用退出）时把游标还给 Core，别让 `git log` 子进程一直挂着
        // （`rust/lithe-core/src/git/history.rs:25-29`：空闲 120 s 才回收、每根最多 8 条）。
        if let Some(cursor) = self.cursor.take() {
            cursor.close();
        }
    }
}

impl Render for BottomPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = v_flex()
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground);

        if !self.visible {
            return root;
        }

        let this = cx.entity().downgrade();

        root = root
            .child(self.title_bar(&this, cx))
            .child(self.tab_row(&this, cx));

        if self.panel == Panel::Console {
            return root.child(Self::console_pane(cx));
        }

        match self.load_state {
            // 错误横幅 + 三栏（`git-log-tool-window.tsx:589-621`）。
            LoadState::Stale => {
                // `git.log.unableToRefresh` = 无法刷新 Git 日志。（`locale.ts:7196`）。
                root = root.child(Self::banner(tr("lithe.git.log.unableToRefresh"), &this, cx));
                root.child(self.log_body(&this, cx))
            }
            // `git.log.noRepository` / `git.log.openWorkspace`（`locale.ts:7198-7199`）。
            LoadState::NoRepository => root.child(Self::centered_notice(
                tr("lithe.git.noRepositoryOpen"),
                Some(tr("lithe.git.log.openWorkspace")),
                None,
                cx,
            )),
            // `git.log.loading` = 正在加载 Git 日志…（`locale.ts:7200`）。
            LoadState::Loading => root.child(Self::centered_notice(
                tr("lithe.git.log.loading"),
                None,
                None,
                cx,
            )),
            LoadState::Failed => root.child(Self::centered_notice(
                tr("lithe.git.log.unableToRefresh"),
                None,
                Some(&this),
                cx,
            )),
            LoadState::Ready => root.child(self.log_body(&this, cx)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{commit_element_id, panel_element_id};
    use crate::model::{Commit, Panel};
    use gpui_kit::SharedString;

    /// 一行提交的最小 fixture：只填渲染与身份用得到的字段，其余固定值，保持测试确定性
    /// （不读时钟、不读环境、不起窗口）。
    fn commit(hash: &str) -> Commit {
        Commit {
            hash: SharedString::from(hash),
            short_hash: SharedString::from(hash),
            parent_hashes: Vec::new(),
            subject: SharedString::from("subject"),
            author: SharedString::from("author"),
            email: SharedString::from("author@example.invalid"),
            date: SharedString::from("2026-01-01"),
            labels: Vec::new(),
        }
    }

    /// 回归判据（《编码指南》「稳定标识」）：同一批页签数据在两种顺序下算出的 id 必须相同，
    /// 不同页签的 id 必须不同。用下标当 id 时，倒序的每一项都会拿到对方的 id，这条会失败。
    #[test]
    fn tab_element_ids_do_not_depend_on_the_tab_row_order() {
        let forward = [Panel::Log, Panel::Console];
        let reversed = [Panel::Console, Panel::Log];

        let forward_ids: Vec<&str> = forward.into_iter().map(panel_element_id).collect();
        let reversed_ids: Vec<&str> = reversed.into_iter().map(panel_element_id).collect();

        assert_eq!(forward_ids[0], reversed_ids[1], "Panel::Log 的 id 必须与位置无关");
        assert_eq!(forward_ids[1], reversed_ids[0], "Panel::Console 的 id 必须与位置无关");
        assert_ne!(forward_ids[0], forward_ids[1], "两个页签的 id 必须互不相同");
    }

    /// 同一批复现在两种顺序下算出的 id 必须相同（提交哈希 vs 行下标）。
    #[test]
    fn commit_element_ids_do_not_depend_on_the_commit_row_order() {
        let commits = [
            commit("1111111111111111111111111111111111111111"),
            commit("2222222222222222222222222222222222222222"),
        ];

        let forward: Vec<String> = commits.iter().map(commit_element_id).collect();
        let reversed: Vec<String> = commits.iter().rev().map(commit_element_id).collect();

        assert_eq!(forward[0], reversed[1], "同一提交在不同顺序下必须算出同一个 id");
        assert_eq!(forward[1], reversed[0], "同一提交在不同顺序下必须算出同一个 id");
        assert_ne!(forward[0], forward[1], "不同提交的 id 必须互不相同");
        assert_eq!(
            forward[0],
            "bottom-git-commit:1111111111111111111111111111111111111111"
        );
        // 身份是哈希、不是内容：同一提交换了主题行（比如 rebase 改写）也还是同一行。
        let mut rewritten = commit("1111111111111111111111111111111111111111");
        rewritten.subject = SharedString::from("另一个主题行");
        assert_eq!(commit_element_id(&rewritten), forward[0]);
    }
}
