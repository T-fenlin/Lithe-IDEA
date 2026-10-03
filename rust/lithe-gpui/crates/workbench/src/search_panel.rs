//! 查找面板（Find in Files）的**视图层**：左活动栏「搜索」项那块侧栏。
//!
//! ## 这一件做了什么（U17 第一件）
//!
//! 查询输入 + 文件掩码 + 三个查询选项（区分大小写 / 全字匹配 / 正则表达式）+
//! 结果列表（文件头 + 命中行）+ 点击结果打开文件并跳到那一行 + 四态呈现。
//! **纯逻辑**（payload / 解析 / 分组 / 代次）在 [`crate::search_model`]，本模块只做
//! "把状态画出来"与"把事件接到 Core 上"。
//!
//! ## 这一件**没做**（交给后续任务，这里不顺手开个头）
//!
//! - **Search Everywhere**（双击 Shift 的全局搜索浮层，`workspace.searchEverywhere`）；
//! - **Quick Open**（`Ctrl+P` 的文件快速打开）；
//! - **Replace in Files**（`workspace.replacePreview` 与替换应用）。
//!
//! 三者与本面板共用 `crate::search_model` 的一部分（payload / 解析），但都需要各自的
//! 界面与状态，不在本件范围。
//!
//! ## 索引：本模块**不碰**索引
//!
//! `warm` / `update` / `invalidate` 三条命令的 owner 是 [`crate::search_index`]（U18）：
//! 项目打开时后台 warm、保存时增量 update、换根时 invalidate。本面板只发
//! `workspace.search`（Core 自己在 `get_or_build` 里复用那份索引，`project/search_index.rs:80-93`）。
//!
//! ⚠️ **如实登记一条 U18 的缺口**：宿主今天**没有**"索引就绪"这个状态，也没有把它显示出来。
//! 所以"项目刚打开、warm 还没跑完就立刻搜索"这一次会在后台等索引锁
//! （`get_or_build` 持锁建索引）—— 界面不会卡（Core 调用在 `background_spawn` 上），
//! 但第一下搜索会慢。**本面板不另建一套索引状态**（那会是第二个真相源），
//! 只在 UI-MAP §2.2 ⑬ 记成缺口。
//!
//! ## 异步边界：UI 线程不做阻塞工作
//!
//! `CoreClient::execute` 是**同步**的（`shared/src/core_client.rs:27-28`），所以它只出现在
//! `cx.background_spawn` 里；结果的回写走 `WeakEntity::update`。形状照
//! `crate::search_index`（U18 刚落地的接线）。
//!
//! ## 长操作的三件事：`operationID` / 取消 / stale 保护
//!
//! 1. **`operationId`**：每次查询由 [`QueryTimeline::schedule`] 按代次生成
//!    （`lithe-gpui-search-<n>`），经 `CoreRequest::with_operation_id` 进信封
//!    （`shared/src/core_client.rs:154-158,169-183`）；
//! 2. **取消**：换代时把上一条在飞请求的 `operationId` 交给
//!    `lithe_gpui_shared::cancel_operation`（**在后台任务里调**），Core 的协作式取消
//!    会让那次扫描尽早停下（`rust/lithe-core/src/protocol/cancellation.rs:96` 的 `check()`
//!    在搜索循环里逐文件 / 逐行被调）；
//! 3. **stale 保护**：结果回来时先过代次闸门（[`QueryTimeline::accepts`]），
//!    旧代次一律丢弃并留一行 `stale=1`。**这一条不依赖"取消真的生效"** ——
//!    取消是尽力而为的优化，代次才是正确性保证。
//!
//! ## 防抖：250ms，照 macOS
//!
//! 真机 `SearchSidebarView.swift:141-145` 是 `.task(id: query|options)` + `Task.sleep(250ms)`
//! + `guard !Task.isCancelled`，即"输入停 250ms 才真的搜"。本侧用同一招的 gpui 形式：
//! 把防抖任务存在 [`SearchPanel::debounce_task`] 里，**换掉旧的 = 取消旧的唤醒**
//! （gpui 的 `Task` 一 drop 就取消，同一个 crate 的 `session.rs:1971-1985` 是既有先例）。
//! `Enter` 跳过防抖立即搜。
//!
//! ## 为什么选项是三个内联 `Checkbox`，而不是 macOS 那个 `Menu`
//!
//! 真机把三个开关收在一颗滑块图标的下拉菜单里（`SearchSidebarView.swift:181-195`）。
//! 本侧刻意不建浮层：侧栏宽度**可以拖得很窄**（真源下界 30，见 `workspace.rs` 的
//! `SIDEBAR_MIN_WIDTH_SPEC`），
//! 三个开关的文案（`使用正则表达式` 这类）在一行里放不下，而浮层要处理定位与焦点归还。
//! 纵向三个 `Checkbox` 是"窄侧栏里能读、且不引入浮层"的选择，属于**有意偏离**，
//! 记在 UI-MAP §2.2 ⑬。
//!
//! ## 诊断（stderr）
//!
//! `S1_SEARCH` 与 `S1_SEARCH_INDEX` / `S1_NOTIFICATION` 同族：交互类改动在本仓库只能靠
//! 维护者手动核对，所以每一次"真的发了查询 / 结果落地 / 结果过期 / 打开了一条结果"
//! 都留一行可 grep 的锚点。走 `eprintln!`（stdout 重定向到文件是块缓冲，进程还在跑时
//! 可能一行都没落盘）。

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, Focusable as _,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Task, Window, div, rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_image};
use lithe_gpui_shared::{CoreClient, CoreError, CoreRequest, tr, tr_args};
use serde_json::Value;

use crate::search_index::SearchIndexReadiness;
use crate::search_model::{
    QueryTimeline, SEARCH_COMMAND, SearchOptions, SearchRow, SearchState, absolute, parse_matches,
    payload,
};

/// 诊断前缀（`eprintln!`，stderr）。与 `S1_SEARCH_INDEX` 一族同口径。
const TAG: &str = "S1_SEARCH";

/// 输入防抖窗口（毫秒）。
///
/// 值与真机一致：`SearchSidebarView.swift:142` 的 `Task.sleep(for: .milliseconds(250))`。
/// 不发明第二个值 —— 这个数字决定"打一个字会不会发一次全盘搜索"。
const SEARCH_DEBOUNCE_MS: u64 = 250;

/// 三个查询选项。
///
/// 做成 enum 而不是"三处字符串 match"：`id` / 文案键 / 取哪一格 / 写哪一格**只有这一份**，
/// 所以"表里加一档、忘了改 match"这种漂移编译期就挡住（`render` 与 `set` 都只认这个类型）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QueryOption {
    /// `caseSensitive`。
    CaseSensitive,
    /// `wholeWords`。
    WholeWords,
    /// `regularExpression`。
    RegularExpression,
}

impl QueryOption {
    /// 界面上从上到下的顺序（也是 `SearchOptions` 的字段顺序）。
    const ALL: [QueryOption; 3] = [
        Self::CaseSensitive,
        Self::WholeWords,
        Self::RegularExpression,
    ];

    /// 元素 id 的片段（`ElementId` 由它拼出来，所以必须互不相同）。
    fn id(self) -> &'static str {
        match self {
            Self::CaseSensitive => "case-sensitive",
            Self::WholeWords => "whole-words",
            Self::RegularExpression => "regular-expression",
        }
    }

    /// 文案键。取旧 Windows 的 `search.*`（`locales/source/locale.ts:1462-1464`），
    /// 与 macOS 的 `Match Case` / `Whole Words` / `Regular Expression`
    /// （`SearchSidebarView.swift:183-185`）是同一件事、同一个中文。
    fn label_key(self) -> &'static str {
        match self {
            Self::CaseSensitive => "lithe.search.matchCase",
            Self::WholeWords => "lithe.search.matchWholeWord",
            Self::RegularExpression => "lithe.search.useRegex",
        }
    }

    /// 读这一格。
    fn get(self, options: &SearchOptions) -> bool {
        match self {
            Self::CaseSensitive => options.case_sensitive,
            Self::WholeWords => options.whole_words,
            Self::RegularExpression => options.regular_expression,
        }
    }

    /// 写这一格；返回**是否真的变了**（没变就不用重搜）。
    fn set(self, options: &mut SearchOptions, value: bool) -> bool {
        let slot = match self {
            Self::CaseSensitive => &mut options.case_sensitive,
            Self::WholeWords => &mut options.whole_words,
            Self::RegularExpression => &mut options.regular_expression,
        };
        let changed = *slot != value;
        *slot = value;
        changed
    }
}

/// 侧栏「搜索」面板。
///
/// 生命周期与外壳相同：换根会重建整个 `ShellWorkspace`（见 `workspace.rs` 模块头），
/// 所以 `root` 是不可变字段，不需要"换根要不要取消"的分支。
pub struct SearchPanel {
    /// 工作区根。Core 的 `root` 与"把相对路径变成绝对路径"都用它。
    root: PathBuf,
    /// 查询框。
    query: Entity<InputState>,
    /// 文件掩码框（Core 的 `fileMask`，逗号分隔 glob）。
    mask: Entity<InputState>,
    /// 查询框的当前值。`Input` 没有 `on_change`，只能听 `InputEvent::Change` 存一份
    /// （与 `notifications.rs:228-230` 同一做法）。
    query_text: SharedString,
    /// 掩码框的当前值（同上）。
    mask_text: SharedString,
    /// 三个开关 + 掩码，拼成 Core 的请求选项。
    options: SearchOptions,
    /// 四态状态机（`idle` / `loading` / `ready` / `failed`）。
    state: SearchState,
    /// 代次与在飞 `operationId`（stale 保护 + 取消句柄）。
    timeline: QueryTimeline,
    /// 防抖任务。**换掉旧的 = 取消旧的唤醒**（`Task` 一 drop 就取消）。
    debounce_task: Option<Task<()>>,
    /// 「点一条结果」往外交的回调（外壳接到编辑区，见 [`SearchPanel::new`]）。
    ///
    /// 存 `Rc` 而不是 `Box`：渲染闭包要求 `'static`，借用不到 `&self.on_open`
    /// （`explorer_view.rs:196-197` 是既有的同一处取舍）。
    on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static>,
    /// 输入框订阅。**必须持有**，掉了就取消订阅。
    _subscriptions: Vec<Subscription>,
    /// 搜索索引的就绪状态（`U18` 收尾）。
    ///
    /// **只读的展示值，不是第二个真相源**：owner 是 `crate::search_index` 的
    /// `SearchIndexWiring`，外壳在它变化时**推**进来（[`Self::set_search_index_readiness`]）。
    /// 面板**不**据此禁用 / 排队 / 阻塞任何搜索，也不改请求形状 —— 只是未就绪时在结果区
    /// 上方多一行说明（`crate::search_index::readiness_notice`）。
    search_index_readiness: SearchIndexReadiness,
}

impl SearchPanel {
    /// 建面板。
    ///
    /// `on_open(path, line, window, cx)`：`line` 为 `Some` 时跳到那一行（命中行），
    /// `None` 时只打开文件（路径命中得到的文件头 —— 真机同一个语义，
    /// `SearchSidebarView.swift:99-103` 只有 `result.line` 存在时才显示 `:line`）。
    /// 打开逻辑**不在这里**：那是编辑区的通路（`EditorPane::open` / `open_at`），
    /// 外壳是唯一够得到编辑区的那一层。
    pub fn new(
        root: PathBuf,
        on_open: Box<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx.new(|cx| {
            InputState::new(window, cx).placeholder(tr("lithe.search.gpui.queryPlaceholder"))
        });
        let mask =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr("lithe.search.filesToInclude")));

        // 两个框共用一个处理器：它一次读两个框的当前值，所以"改了哪个"不影响结果；
        // `Enter` 也一样（在掩码框里回车同样立即搜）。
        let subscribe = |state: &Entity<InputState>, cx: &mut Context<Self>| {
            cx.subscribe_in(
                state,
                window,
                |this: &mut Self, _state: &Entity<InputState>, event: &InputEvent, _, cx| {
                    this.on_input_event(event, cx);
                },
            )
        };
        let subscriptions = vec![subscribe(&query, cx), subscribe(&mask, cx)];

        let mut options = SearchOptions::default();
        // 防御：`file_mask` 初值必须与掩码框的初值同源（掩码框是空串）。
        options.file_mask = String::new();

        Self {
            root,
            query,
            mask,
            query_text: SharedString::default(),
            mask_text: SharedString::default(),
            options,
            state: SearchState::Idle,
            timeline: QueryTimeline::default(),
            debounce_task: None,
            on_open: Rc::from(on_open),
            _subscriptions: subscriptions,
            // 面板与外壳的索引接线同一次构造：此刻 `warm` 还没排出去，所以起于 `Pending`
            // （与 `SearchIndexWiring::new` 同一个默认值，别写第二个字面量）。
            search_index_readiness: SearchIndexReadiness::default(),
        }
    }

    /// 外壳推来一个新的索引就绪状态（`U18` 收尾；唯一写者仍是 `search_index`）。
    ///
    /// **只在真的变了时 `cx.notify()`**：否则一次多余的推会让面板白重绘一次。
    pub(crate) fn set_search_index_readiness(
        &mut self,
        readiness: SearchIndexReadiness,
        cx: &mut Context<Self>,
    ) {
        if self.search_index_readiness == readiness {
            return;
        }
        self.search_index_readiness = readiness;
        cx.notify();
    }

    /// 面板被选中时调（外壳在活动栏点击那条事件回调里调）。
    ///
    /// 只做一件事：把焦点交给查询框。真机同一条：`SearchSidebarView.swift:146` 的
    /// `.onAppear { searchFocused = true }`。
    ///
    /// ⚠️ **不能在 `render` 里做**（`render` 不许 mutate / 起任务），所以外壳从事件回调调。
    pub fn activate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.query.read(cx).focus_handle(cx).focus(window, cx);
    }

    /// 输入事件（两个框共用）。
    fn on_input_event(&mut self, event: &InputEvent, cx: &mut Context<Self>) {
        match event {
            InputEvent::Change => {
                self.query_text = self.query.read(cx).value();
                self.mask_text = self.mask.read(cx).value();
                self.options.file_mask = self.mask_text.to_string();
                self.restart(cx);
            }
            // 回车 = 不等防抖，立刻搜。真机没有这一条，但它是"按了没反应"的解药：
            // 250ms 窗口里回车是最自然的"就是现在"。
            InputEvent::PressEnter { .. } => self.issue(cx),
            InputEvent::Focus | InputEvent::Blur => {}
        }
    }

    /// 输入变了：旧查询**立刻作废**（换代 + 尽力取消），再排一次防抖。
    ///
    /// 状态在防抖窗口里**不动**：旧结果继续留在屏幕上，直到新查询真的发出去 ——
    /// 打一个字就闪一下 loading 是更差的观感（真机同样保留 `searchResults`，
    /// `SearchFeatureModel.swift:125-141`）。
    fn restart(&mut self, cx: &mut Context<Self>) {
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        // 空查询**不发 Core 调用**：`workspace.search` 对空查询直接回空
        // （`project/files.rs:172-177`），发过去只是白跑一趟信封。
        if self.query_text.trim().is_empty() {
            self.debounce_task = None;
            self.state = SearchState::Idle;
            Self::diagnose("idle", "query=empty");
            cx.notify();
            return;
        }
        // 排新的防抖；换掉旧任务 = 取消旧唤醒。
        self.debounce_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(SEARCH_DEBOUNCE_MS))
                .await;
            // 实体可能已经销毁：`update` 返回 `Err` 时静默忽略，不 panic。
            let _ = this.update(cx, |panel, cx| panel.issue(cx));
        }));
        cx.notify();
    }

    /// 真的发一次查询（防抖到期 / 回车）。
    fn issue(&mut self, cx: &mut Context<Self>) {
        let query = self.query_text.trim().to_string();
        if query.is_empty() {
            // 防御：防抖窗口里被清空（那时 `restart` 已经把状态打回 `Idle`）。
            return;
        }
        let scheduled = self.timeline.schedule();
        if let Some(superseded) = scheduled.superseded.clone() {
            Self::cancel_in_background(superseded, cx);
        }
        let generation = scheduled.generation;
        let operation_id = scheduled.operation_id.clone();
        let root = self.root.to_string_lossy().to_string();
        let options = self.options.clone();
        self.state = SearchState::loading(query.clone());
        Self::diagnose(
            "issue",
            &format!("generation={generation} operation={operation_id} query={query}"),
        );
        cx.notify();

        // 同步的 Core 调用只在 `background_spawn` 里跑（UI 线程不做阻塞工作）。
        // 任务 detach 掉：代次闸门才是采纳判据，外壳被换掉时 `WeakEntity::update` 自然失败。
        let background_query = query.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let request = CoreRequest::command(SEARCH_COMMAND)
                        .with_payload(payload(&root, &background_query, &options))
                        .with_operation_id(operation_id);
                    CoreClient::new().execute(&request)
                })
                .await;
            let _ = this.update(cx, |panel, cx| panel.adopt(generation, &query, result, cx));
        })
        .detach();
    }

    /// 后台结果回到前台：**先过代次闸门**，再进 `ready` / `failed`。
    fn adopt(
        &mut self,
        generation: u64,
        query: &str,
        result: Result<Option<Value>, CoreError>,
        cx: &mut Context<Self>,
    ) {
        if !self.timeline.accepts(generation) {
            // 过期结果：**不采纳**（契约要求），也不改任何状态。留一行证据。
            Self::diagnose(
                "stale",
                &format!(
                    "generation={generation} current_query_len={}",
                    self.query_text.len()
                ),
            );
            return;
        }
        let next = match result {
            Ok(data) => match parse_matches(data.as_ref()) {
                Ok(matches) => SearchState::ready(query, &matches),
                // 契约异常（缺 data / 形状不对）：给稳定 token，不给句子 ——
                // 宿主不自己编界面文案（见 `search_model::ParseError`）。
                Err(error) => SearchState::failed(query, error.token()),
            },
            // Core 的失败原文是**数据**：`CoreError` 的 Display 给 `code: message`，
            // 与其它 feature 显示 Core 失败同一口径。不吞掉、不换成"搜索失败"了事。
            Err(error) => SearchState::failed(query, error.to_string()),
        };
        let (files, hits) = next.counts();
        match &next {
            SearchState::Failed { reason, .. } => Self::diagnose(
                "failed",
                &format!("generation={generation} query={query} reason={reason}"),
            ),
            SearchState::Ready { .. } => Self::diagnose(
                "complete",
                &format!(
                    "generation={generation} query={query} files={files} hits={hits} state={}",
                    // 空结果与"有命中"必须是两个可区分的状态词（契约要求分开）：
                    // 0 条 = `empty`，有命中 = `ready`。
                    if files == 0 && hits == 0 {
                        "empty"
                    } else {
                        "ready"
                    }
                ),
            ),
            // `Loading` / `Idle` 不会从这条路径产生：`issue` 先置 `Loading`，
            // 这里只会把它推进到 `Ready` 或 `Failed`。
            _ => Self::diagnose("unexpected_state", &format!("generation={generation}")),
        }
        self.state = next;
        self.timeline.settle(generation);
        cx.notify();
    }

    /// 把一个查询选项拨到 `value`；变了就立刻作废旧结果并重搜。
    fn set_option(&mut self, option: QueryOption, value: bool, cx: &mut Context<Self>) {
        if !option.set(&mut self.options, value) {
            // 值没变（gpui 的 `Checkbox` 可能重复发同一个值）：不重搜。
            return;
        }
        Self::diagnose("option", &format!("id={} value={value}", option.id()));
        // 选项变了 = 结果集变了：立刻作废旧结果（换代）并重搜。
        self.restart(cx);
    }

    /// 把 `operationId` 交给后台取消。
    ///
    /// **不在 UI 线程调**：它要拿 Core 的操作注册表锁（`cancellation.rs:84`）。
    fn cancel_in_background(operation_id: String, cx: &mut Context<Self>) {
        cx.background_spawn(async move {
            let requested = lithe_gpui_shared::cancel_operation(&operation_id);
            // `false` = 这个 id 没有在飞的操作（已经跑完了）—— 正常情况，不是失败。
            Self::diagnose(
                "cancel",
                &format!("operation={operation_id} requested={requested}"),
            );
        })
        .detach();
    }

    /// 一行 stderr 诊断（`S1_SEARCH <event> <fields>`）。
    fn diagnose(event: &str, fields: &str) {
        eprintln!("{TAG} {event} {fields}");
    }

    // ---- 渲染 ----

    /// 控件区：查询框 + 掩码框 + 三个开关。`flex_shrink_0`：它们不参与结果区的伸缩。
    fn controls(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let mut options = v_flex().w_full().gap_1();
        for option in QueryOption::ALL {
            let checked = option.get(&self.options);
            let id = option.id();
            options = options.child(
                Checkbox::new(ElementId::from(format!("lithe-search-option-{id}")))
                    .label(tr(option.label_key()))
                    .checked(checked)
                    .on_change(cx.listener(move |this, checked: &bool, _, cx| {
                        this.set_option(option, *checked, cx);
                    })),
            );
        }

        v_flex()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .p_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Input::new(&self.query)
                    .small()
                    .cleanable(true)
                    .w_full()
                    .prefix(idea_icon_image(
                        &idea::MAGNIFYING_GLASS_ICON,
                        cx,
                        // 14px：与通知中心搜索框同一个值（`notifications.rs:356-360`），
                        // 档位外写 `rems(P / 16.)`（rem base = 16px）。
                        rems(14. / 16.),
                    ))
                    // 无障碍名：`placeholder` 不是可访问名，而真机那颗放大镜是纯装饰的，
                    // 所以这里显式给一次。
                    .aria_label(tr("lithe.search.gpui.queryPlaceholder")),
            )
            .child(
                // placeholder 在 `InputState::new(..)` 上设（`Input` **没有**
                // `.placeholder(..)`，见 `notifications.rs:246` 的同一写法）。
                Input::new(&self.mask)
                    .small()
                    .cleanable(true)
                    .w_full()
                    .aria_label(tr("lithe.search.filesToInclude")),
            )
            .child(options)
            .into_any_element()
    }

    /// 结果区：四态各画各的。
    ///
    /// 判据只看 [`SearchState`]，**不重算任何东西**（`render` 里不做匹配 / 排序，
    /// 那是 Core 与 `search_model` 的事）。
    fn results(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match &self.state {
            SearchState::Idle => Self::empty_state(
                Some(IconName::Search),
                tr("lithe.search.emptyTitle"),
                Some(tr("lithe.search.emptyDescription")),
                cx,
            ),
            SearchState::Loading { .. } => h_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .justify_center()
                .gap_2()
                .child(Spinner::new().color(cx.theme().muted_foreground))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("lithe.fileExplorer.searchingFiles")),
                )
                .into_any_element(),
            // 失败：标题一句固定的、原因一行 Core 给的原文（或契约异常的稳定 token）。
            // **空结果不走这一支** —— 两者必须分得开。
            SearchState::Failed { reason, .. } => Self::empty_state(
                Some(IconName::TriangleAlert),
                tr("lithe.search.failed"),
                Some(SharedString::from(reason.clone())),
                cx,
            ),
            // 空结果 = `Ready` 但一行都没有：**不是失败**，标题带上查询串
            // （真源 `search.noResultsFor` 的 `{query}` 占位符）。
            SearchState::Ready { rows, .. } if rows.is_empty() => Self::empty_state(
                None,
                tr_args(
                    "lithe.search.noResultsFor",
                    &[("query", self.state.query())],
                ),
                None,
                cx,
            ),
            SearchState::Ready { rows, .. } => self.list(rows, cx),
        }
    }

    /// 结果列表：文件头 + 命中行，可滚动。
    ///
    /// ⚠️ 不用 gpui-kit 的 `List`：它要求**所有行等高**，而文件头与命中行高度不同
    /// （命中行要放预览正文）。`notifications.rs:16-23` 记录了同一条取舍。
    fn list(&self, rows: &[SearchRow], cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .id("lithe-search-results")
            .w_full()
            .flex_1()
            .min_h_0()
            .gap_0()
            .overflow_y_scrollbar()
            .children(rows.iter().map(|row| self.row(row, cx)))
            .into_any_element()
    }

    /// 列表里的一行。
    ///
    /// ⚠️ 元素 id 用**业务身份**（路径 + 行号），不用下标：下标会在结果变化时把点击串到
    /// 别的行上（`notifications.rs:516-522` 同一口径）。文件头与命中行前缀不同，不会撞。
    fn row(&self, row: &SearchRow, cx: &mut Context<Self>) -> AnyElement {
        let open = self.on_open.clone();
        match row {
            // 文件头：路径（工作区相对、`/` 分隔）+ 这个文件的命中数。
            // 点它 = 打开这个文件（没有行可跳，`line = None`）。
            SearchRow::File { path, hits } => {
                let open_path = absolute(&self.root, path);
                let open = open.clone();
                h_flex()
                    .id(ElementId::from(format!("search-file-{path}")))
                    .w_full()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .cursor_pointer()
                    .child(idea_icon_image(&idea::FILE_ICON, cx, rems(14. / 16.)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .font_medium()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_color(cx.theme().foreground)
                            .child(SharedString::from(path.clone())),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            // 命中数是数字（数据，不是文案），不走 `tr`。它的用处是把
                            // "只有文件名命中（0）"与"正文里有命中"分开。
                            .child(SharedString::from(hits.to_string())),
                    )
                    .on_click(cx.listener(move |_this, _, window, cx| {
                        Self::diagnose("open", &format!("file={} line=None", open_path.display()));
                        open(open_path.clone(), None, window, cx);
                    }))
                    .into_any_element()
            }
            // 命中行：行号（1 基，Core 的口径）+ 预览正文。点它 = 打开并跳到这一行。
            SearchRow::Hit {
                path,
                line,
                preview,
            } => {
                let open_path = absolute(&self.root, path);
                let target_line = line.map(|line| line as u32);
                let open = open.clone();
                h_flex()
                    .id(ElementId::from(format!(
                        "search-result-{path}-{}",
                        line.unwrap_or(0)
                    )))
                    .w_full()
                    .gap_2()
                    .pl_4()
                    .pr_2()
                    .py_1()
                    .cursor_pointer()
                    .child(
                        div()
                            .w_8()
                            .flex_shrink_0()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            // 行号是数据（1 基），不是文案。
                            .child(SharedString::from(match line {
                                Some(line) => line.to_string(),
                                None => String::new(),
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            // 真机的预览是等宽 11.5px（`SearchSidebarView.swift:105`）；
                            // 11.5 不在 gpui 的字号档位上，按仓库口径取最近的 `text_xs()`
                            // （12px），字族走主题的等宽族。
                            .font_family(cx.theme().mono_font_family.clone())
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_color(cx.theme().muted_foreground)
                            .child(SharedString::from(preview.clone())),
                    )
                    .on_click(cx.listener(move |_this, _, window, cx| {
                        Self::diagnose(
                            "open",
                            &format!("file={} line={:?}", open_path.display(), target_line),
                        );
                        open(open_path.clone(), target_line, window, cx);
                    }))
                    .into_any_element()
            }
        }
    }

    /// 空态 / 失败态（同一个壳：可选的图标 + 标题 + 可选的一行说明）。
    ///
    /// `Empty` 硬编码了 `.border_dashed()`，真机没有那圈虚线，所以边框色改成透明
    /// （与 `notifications.rs:540-542` 同一招）。
    fn empty_state(
        icon: Option<IconName>,
        title: SharedString,
        description: Option<SharedString>,
        cx: &App,
    ) -> AnyElement {
        let mut header = EmptyHeader::new();
        if let Some(icon) = icon {
            header = header.media(EmptyMedia::new().child(Icon::new(icon).size_8()));
        }
        header = header.title(EmptyTitle::new().text_sm().child(title));
        if let Some(description) = description {
            header = header.description(EmptyDescription::new().text_sm().child(description));
        }
        v_flex()
            .w_full()
            .flex_1()
            .min_h_0()
            .child(
                Empty::new()
                    .border_color(cx.theme().transparent)
                    .header(header),
            )
            .into_any_element()
    }
}

impl Render for SearchPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // ⚠️ 这里**只读**状态：不 mutate、不 notify、不起任务、不开浮层。
        // 进 `loading` / 换代 / 取消都发生在事件回调与后台任务里（见模块文档）。
        // 索引就绪那一行同理：判据与元素都归 `crate::search_index`，这里只把它画出来
        // （`Ready` 时它是 `None`，`.children(..)` 收 0 个孩子 ⇒ 整行消失）。
        let index_notice = crate::search_index::readiness_notice(self.search_index_readiness, cx);
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.controls(cx))
            .children(index_notice)
            .child(self.results(cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_model::{Match, MatchKind, group};

    /// 三个选项的 `id` 必须互不相同。重复会让两个 `Checkbox` 抢同一个 `ElementId`
    /// —— gpui 会把点击串到另一个上（`notifications.rs:677-689` 记过这条）。
    #[test]
    fn option_ids_are_distinct_and_cover_every_option() {
        let mut ids: Vec<&str> = QueryOption::ALL.iter().map(|option| option.id()).collect();
        assert_eq!(ids.len(), 3, "三个开关一个不多一个不少");
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "选项 id 不能重复");
    }

    /// `get` / `set` 必须落在**同一格**上，而且 `set` 只在真的变了时返回 `true`
    /// （返回值决定"要不要重搜"，所以它也是行为而不只是优化）。
    #[test]
    fn every_option_reads_and_writes_its_own_slot() {
        for option in QueryOption::ALL {
            let mut options = SearchOptions::default();
            assert!(
                !option.get(&options),
                "默认三个开关都关（与 Core 的默认一致）"
            );

            assert!(
                option.set(&mut options, true),
                "从一个默认值拨到 true 必须算变"
            );
            assert!(option.get(&options), "{option:?} 写进去读不出来");
            assert!(!option.set(&mut options, true), "值没变不该算变");
            assert!(option.set(&mut options, false), "拨回去也算变");
            assert!(!option.get(&options));

            // 只动自己那一格：把它拨到 true 之后，另外两个仍然是 false。
            option.set(&mut options, true);
            for other in QueryOption::ALL {
                if other != option {
                    assert!(!other.get(&options), "{other:?} 被 {option:?} 连坐了");
                }
            }
        }
    }

    /// 三个文案键都必须在 `lithe.search.*` 里（配错就是界面上出现键名原文，
    /// `tr` 对缺键是**原样回显**，所以这条同时守住"键存在"这一半）。
    #[test]
    fn every_option_label_key_is_in_the_search_namespace() {
        for option in QueryOption::ALL {
            let key = option.label_key();
            assert!(key.starts_with("lithe.search."), "{option:?}：{key}");
            let label = tr(key);
            assert!(!label.is_empty(), "{key} 解析出空文案");
            assert_ne!(
                label.as_ref(),
                key,
                "{key} 没有被 locale 解析（tr 原样回显了键名）"
            );
        }
    }

    /// 防抖窗口与真机同值（250ms）。这个数字决定"打一个字会不会发一次全盘搜索"，
    /// 所以用一条断言钉住它，而不是让它散落在文档里。
    #[test]
    fn the_debounce_window_matches_the_macos_spec() {
        assert_eq!(SEARCH_DEBOUNCE_MS, 250);
    }

    /// 结果行的元素 id 是**业务身份**（路径 + 行号）而不是下标：这里钉住拼出来的形状
    /// 与唯一性（下标在结果变化时会把点击串到别的行上）。
    #[test]
    fn row_element_ids_are_business_identities() {
        let rows = group(&[
            Match {
                kind: MatchKind::Content,
                path: "src/A.java".to_string(),
                line: Some(12),
                preview: "x".to_string(),
                symbol_name: None,
            },
            Match {
                kind: MatchKind::Content,
                path: "src/B.java".to_string(),
                line: Some(1),
                preview: "y".to_string(),
                symbol_name: None,
            },
        ]);
        let ids: Vec<String> = rows
            .iter()
            .map(|row| match row {
                SearchRow::File { path, .. } => format!("search-file-{path}"),
                SearchRow::Hit { path, line, .. } => {
                    format!("search-result-{path}-{}", line.unwrap_or(0))
                }
            })
            .collect();
        assert_eq!(
            ids,
            vec![
                "search-file-src/A.java",
                "search-result-src/A.java-12",
                "search-file-src/B.java",
                "search-result-src/B.java-1",
            ]
        );
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "元素 id 不能重复");
    }

    /// 打开一条结果用的绝对路径：根 + 工作区相对路径（拼接口径只有 `search_model::absolute`
    /// 一份实现，这里钉住"面板用的是它"）。
    #[test]
    fn opening_a_result_joins_the_workspace_root() {
        let root = std::path::Path::new("proj-root");
        assert_eq!(
            absolute(root, "src/A.java"),
            root.join("src").join("A.java")
        );
    }
}
