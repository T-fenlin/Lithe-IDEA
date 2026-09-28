//! 通知中心面板：铃铛角标的**内容**那一面。
//!
//! 这个面板是「消息发生过」的主要呈现面（另一面是状态栏那条可点的入口，见
//! [`crate::status_bar`])。gpui-kit 的角落 toast（`component::Notification`）与状态栏那条
//! 4 秒自动消失的小字都已废掉 —— 决策见
//! `.agents/notes/implemented/architecture/2026-09-27-gpui-notification-center.md`。
//!
//! ## 为什么是独立实体，而不是 `right_tool_window` 的一个自由函数
//!
//! 面板要持有**自己的**视图状态：搜索框（`Entity<InputState>` + 当前查询串）、严重性
//! 筛选、哪一条展开了。这些都不属于外壳 —— 换根时它们本来就该一起没，而通知**数据**
//! 由 [`Store`] 按那条决策独立于外壳存活。
//!
//! 顺带的好处：`ShellWorkspace` 已经是 287KB，再塞 4 个字段会让「哪部分状态归谁」更难看。
//!
//! ## 为什么不用 gpui-kit 的 `List`
//!
//! `ListDelegate` 要求**所有行等高**（`gpui-component-0.6.6/src/list/delegate.rs:41` 的
//! "Every item should have same height"，`list.rs:446-451` 只测量一行高度后复用）。
//! 本面板的行要能**展开详情**（IDEA 的详情是「描述 + 跳转链接」），展开后高度可变，
//! 放进 `List` 会错位。所以用 `v_flex().overflow_y_scrollbar()` 自己列行 —— 滚动条由
//! `ScrollableElement::overflow_y_scrollbar` 挂（`scroll/scrollable.rs:60-62`），不是
//! 「要自己挂」的那种。
//!
//! ## 搜索搜三样
//!
//! 稳定码（ASCII 大小写不敏感）、参数值、渲染后的文案。参数值那一路最关键：路径、行号、
//! 扩展名基本只出现在参数里，文案里只有「打开失败」四个字，搜不出来。判据在
//! `lithe_db_gpui_notify::Entry::matches_query` 与 `lithe_db_gpui_notify::matches_text`。
//!
//! ## 文案在渲染时拼
//!
//! 条目存 `(code, params)`，本模块用 [`tr_args`] 拼出那一行 —— 依据是
//! `shared/contracts/application-boundary.md:227-228`（领域层返回稳定原因，文案归展示层）。
//! 占位符配错时 `interpolate` 会把 `{detail}` 原样留在界面上（`i18n.rs:87-88`），是可见
//! 的失败而不是静默的空串。

use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, Selectable as _,
};
// `StyledExt` 提供 `font_semibold` 这类语义化样式方法（gpui-component 的 crate 根，
// `gpui_kit` 再导出到 `component`）。注意 `align_items_center` / `items_center` **不在**
// 这里 —— 前者在 0.6.6 里根本不存在，后者是 `Styled` 的固有方法（`gpui-pre-0.3.6/src/
// styled.rs:301`）。
use gpui_kit::component::StyledExt as _;
use gpui_kit::prelude::FluentBuilder as _;
// `Context` / `Entity` / `EventEmitter` / `Render` / `Subscription` / `WeakEntity` /
// `AppContext` 全部来自 `gpui_kit` 根（`gpui-kit-0.6.6/src/lib.rs:95` 的 `pub use ::gpui::*`），
// 与 `branch_panel.rs:111-115` 同一取法。
//
// ⚠️ `on_click` **不在** `InteractiveElement` 上而在 `StatefulInteractiveElement`
// （`gpui-pre-0.3.6/src/elements/div.rs:1300`），而 `Div` 只实现前者
// （`:1871`）—— 所以可点的 `div()` 必须先 `.id(..)` 变成 `Stateful<Div>`
// （`Stateful` 的实现在 `:4074`）。忘了 `.id` 的表现是「no method named on_click」。
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, Hsla, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, WeakEntity, Window, div,
};

use lithe_db_gpui_notify::{Entry, EntryId, Severity, Store, matches_text};
use lithe_db_gpui_shared::{tr, tr_args};

/// 现在到 Unix 毫秒。
///
/// 通知时间戳要显示给用户，所以是墙钟而不是单调时钟。`SystemTime` 在 1970 之前会返回
/// `Err`，这里退化成 0 —— 显示成一个时间戳，而不是让整帧 render 失败。
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// 相对时间文案。
///
/// **只到分钟**：秒级相对时间每帧都在变，会让整列表每帧重排；更细的粒度在这个面板约
/// 350px 的宽度里也读不出来。四个键对应 IDEA 时间线的四档，与 macOS 中心用绝对时刻
/// （`WorkbenchView.swift:1900` 的 `date: .omitted, time: .shortened`）是同一层信息。
fn format_age(now: u64, then: u64) -> SharedString {
    let elapsed = now.saturating_sub(then);
    let minutes = elapsed / 60_000;
    if minutes == 0 {
        tr("lithe.notifications.justNow")
    } else if minutes < 60 {
        tr_args(
            "lithe.notifications.minutesAgo",
            &[("count", &minutes.to_string())],
        )
    } else {
        let hours = minutes / 60;
        if hours < 24 {
            tr_args(
                "lithe.notifications.hoursAgo",
                &[("count", &hours.to_string())],
            )
        } else {
            tr_args(
                "lithe.notifications.daysAgo",
                &[("count", &(elapsed / 86_400_000).to_string())],
            )
        }
    }
}

/// 渲染一条通知的文案。`code` 是 `tr` 的键，`params` 是插值参数。
///
/// `pub` 是因为状态栏那条入口也要显示同一句话（`ShellWorkspace::footer_left`）—— 两处
/// 必须显示**同一句**，否则状态栏与中心会各说各话。
///
/// 这里是**唯一**拼文案的地方，所以「文案不存进条目」这条决定落在代码里而不只是文档里。
pub fn render_message(entry: &Entry) -> SharedString {
    let args: Vec<(&str, &str)> = entry
        .params()
        .iter()
        .map(|(name, value)| (name.as_ref(), value.as_ref()))
        .collect();
    tr_args(entry.code().as_ref(), &args)
}

/// 严重性 → 主题色。
///
/// 与 gpui-kit 自己的映射一致（`gpui-component-0.6.6/src/notification.rs:41-46`：
/// Info→`info` / Warning→`warning` / Error→`danger`），不另发明一套。
fn severity_color(severity: Severity, cx: &App) -> Hsla {
    match severity {
        Severity::Info => cx.theme().info,
        Severity::Warning => cx.theme().warning,
        Severity::Error => cx.theme().danger,
    }
}

/// 严重性 → 图标。
///
/// 同上，与 `notification.rs:41-46` 一致。`IconName` 从 `component` **平铺**导出
/// （`component::icon` 是私有模块，`lib.rs:7,112`），所以路径是
/// `gpui_kit::component::IconName`，没有 `component::icon::IconName` 这条路。
///
/// ⚠️ 中文文档镜像里写的 `IconName::AlertTriangle` / `CheckCircle` / `XCircle`
/// **在 0.6.6 里都不存在**（`gpui-kit-assets-0.6.6/default-icons.txt` 只有 101 个默认
/// 字形），别照抄那一页。
fn severity_icon(severity: Severity) -> IconName {
    match severity {
        Severity::Info => IconName::Info,
        Severity::Warning => IconName::TriangleAlert,
        Severity::Error => IconName::CircleX,
    }
}

/// 严重性筛选。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Info,
    Warning,
    Error,
}

impl Filter {
    /// 四个筛选项，`tr` 键与判据放在一起 —— 加一档只改这一处。
    ///
    /// 键取自旧 Windows 的 `notifications.filter*`（`locale.ts:2996-3015`），
    /// **不含** `filterSuccess` —— 中心没有 success 档，见 `lithe_db_gpui_notify::Severity`。
    const OPTIONS: [(Filter, &'static str); 4] = [
        (Filter::All, "lithe.notifications.filterAll"),
        (Filter::Info, "lithe.notifications.filterInfo"),
        (Filter::Warning, "lithe.notifications.filterWarnings"),
        (Filter::Error, "lithe.notifications.filterErrors"),
    ];

    /// 遍历四个筛选项（画筛选条用）。
    pub fn options() -> impl Iterator<Item = (Filter, &'static str)> {
        Self::OPTIONS.into_iter()
    }

    /// 该筛选项的文案。
    pub fn label(self) -> SharedString {
        let key = Self::OPTIONS
            .iter()
            .find(|(option, _)| *option == self)
            .map(|(_, key)| *key)
            .unwrap_or("lithe.notifications.filterAll");
        tr(key)
    }

    /// 这一档放行哪些严重性。
    fn admits(self, severity: Severity) -> bool {
        match self {
            Filter::All => true,
            Filter::Info => severity == Severity::Info,
            Filter::Warning => severity == Severity::Warning,
            Filter::Error => severity == Severity::Error,
        }
    }
}

/// 画一行要用的数据。
///
/// 单独一个结构是为了让 `store` 的读借用能在建元素树**之前**结束：`Entity::read` 返回的
/// 守卫借用 `cx`，而元素树要拿 `&mut Context`。所以先把要用的都拷成自有值。
struct Row {
    id: EntryId,
    severity: Severity,
    message: SharedString,
    age: SharedString,
    unread: bool,
    occurrences: u32,
    /// 展开时显示的 `名字: 值` 列表。
    details: Vec<(SharedString, SharedString)>,
}

/// 通知中心面板。
pub struct NotificationPanel {
    /// 通知数据。**弱引用**：面板不该 prolong store 的寿命，store 由比外壳更长命的
    /// 持有者管（见 `lithe-db-gpui-notify` 的模块文档）。store 没了就退回空态。
    store: WeakEntity<Store>,
    search: Entity<InputState>,
    /// 当前查询串。`Input` **没有** `on_change`（`input/input.rs` 全文无此方法），
    /// 只能 `cx.subscribe_in` 听 `InputEvent::Change` 把值取出来存一份 —— 与
    /// `branch_panel.rs:618-620,654-658` 同一做法。
    query: SharedString,
    filter: Filter,
    /// 展开了哪一条（`None` = 全折叠）。只展开一条，与 IDEA 一致。
    expanded: Option<EntryId>,
    /// 盯住 store：它一变就重画。**必须**持有，掉了就取消订阅
    /// （`branch_panel.rs:626` 记过这条）。
    _store_subscription: Subscription,
    /// 同理，盯住搜索框。搜不到东西时面板要立刻改画「没有匹配」。
    _search_subscription: Subscription,
}

impl NotificationPanel {
    /// 建面板。`store` 是弱引用，所以 store 被销毁时 `store()` 返回 `None`。
    pub fn new(store: &Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx
            .new(|cx| InputState::new(window, cx).placeholder(tr("lithe.notifications.search")));

        let store_subscription = cx.observe(store, |_, _, cx| cx.notify());

        let search_subscription = cx.subscribe_in(
            &search,
            window,
            |this: &mut Self, state: &Entity<InputState>, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.query = state.read(cx).value();
                    cx.notify();
                }
            },
        );

        Self {
            store: store.downgrade(),
            search,
            query: SharedString::default(),
            filter: Filter::default(),
            expanded: None,
            _store_subscription: store_subscription,
            _search_subscription: search_subscription,
        }
    }

    /// 展开 / 折叠某一条。
    ///
    /// 「哪一条被展开了」这个事实由 [`Self::diagnose_toggle`] 那一行可 grep 的诊断行承担 ——
    /// 交互类改动在本仓库只能靠维护者手动验证（`lithe-db-gpui/docs/grill.md` D1a），所以不另建一套
    /// 事件机制。
    fn toggle(&mut self, id: EntryId, cx: &mut Context<Self>) {
        self.expanded = if self.expanded == Some(id) { None } else { Some(id) };
        Self::diagnose_toggle(id, self.expanded.is_some());
        cx.notify();
    }

    /// 诊断行：点行展开。
    ///
    /// `S1_NOTIFICATION` 与 `S1_STATUS_NOTICE` / `S1_RIGHT_PANEL` 同一族 —— 交互类改动在
    /// 本仓库只能靠维护者手动验证（`lithe-db-gpui/docs/grill.md` D1a），所以这行可 grep 的证据
    /// 是唯一的机器判据。被删掉的 `S1_STATUS_NOTICE` 正是由它接班的。
    fn diagnose_toggle(id: EntryId, expanded: bool) {
        eprintln!("S1_NOTIFICATION action=toggle id={id} expanded={expanded}");
    }

    /// 诊断行：清空。
    fn diagnose_clear(removed: usize) {
        eprintln!("S1_NOTIFICATION action=clear removed={removed}");
    }

    /// 当前筛选后的行数据。
    ///
    /// 「筛选」与「搜索」是**与**的关系：先按严重性挡掉，再按查询串过滤。空查询匹配一切
    /// （`Entry::matches_query` 的第一行）。
    fn rows(&self, now: u64, cx: &App) -> Vec<Row> {
        let Some(store) = self.store.upgrade() else {
            return Vec::new();
        };
        let guard = store.read(cx);
        let read_upto = guard.read_upto_seq();
        let query = self.query.as_ref();

        guard
            .entries()
            .iter()
            .filter_map(|entry| {
                if !self.filter.admits(entry.severity()) {
                    return None;
                }
                let message = render_message(entry);
                if !(entry.matches_query(query) || matches_text(&message, query)) {
                    return None;
                }
                Some(Row {
                    id: entry.id(),
                    severity: entry.severity(),
                    message,
                    age: format_age(now, entry.updated_at_ms()),
                    unread: entry.is_unread(read_upto),
                    occurrences: entry.occurrences(),
                    details: entry
                        .params()
                        .iter()
                        .map(|(name, value)| (name.clone(), value.clone()))
                        .collect(),
                })
            })
            .collect()
    }

    /// 搜索框 + 筛选条 + 「全部清除」那一行。
    fn toolbar(&mut self, total: usize, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .px_2()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Input::new(&self.search)
                    .small()
                    .cleanable(true)
                    .flex_1()
                    .prefix(
                        Icon::new(IconName::Search).text_color(cx.theme().muted_foreground),
                    )
                    .aria_label(tr("lithe.notifications.search")),
            )
            .child(
                Button::new("lithe-notifications-clear-all")
                    .ghost()
                    .small()
                    .icon(IconName::Delete)
                    .disabled(total == 0)
                    .tooltip(tr("lithe.notifications.clearAll"))
                    .accessibility_label(tr("lithe.notifications.clearAll"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        let Some(store) = this.store.upgrade() else {
                            return;
                        };
                        let removed = store.update(cx, |store, _| store.clear());
                        Self::diagnose_clear(removed);
                        this.expanded = None;
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    /// 筛选条：四档，选中那档给 `selected` 底色。
    fn filter_bar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .gap_1()
            .px_2()
            .pb_2()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.notifications.filter")),
            )
            .children(Filter::options().map(|(option, _)| {
                let selected = option == self.filter;
                // 元素 id 用拼出来的字符串而不是元组：`ElementId` 没有
                // `From<(&str, &str)>`（元组那个组合是 `ElementId::named_usize` 那条路，
                // 见 `activity_bar.rs:401`）。四个 id 必须互不相同，否则点击会串到别的按钮上。
                let id = format!("lithe-notifications-filter-{}", option_key(option));
                Button::new(id)
                    .ghost()
                    .small()
                    .selected(selected)
                    .toggled(selected)
                    .label(option.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.filter = option;
                        cx.notify();
                    }))
            }))
            .into_any_element()
    }

    /// 一行通知。
    ///
    /// 点整行展开 / 折叠详情。**行内没有嵌套按钮** —— GPUI 的点击默认冒泡
    /// （`gpui-pre-0.3.6/src/app.rs:2379`），有嵌套按钮就必须在它的处理器里
    /// `cx.stop_propagation()`，否则「删除」会连带把整行也展开。这里选择不嵌套。
    fn row(&self, row: &Row, cx: &mut Context<Self>) -> AnyElement {
        let expanded = self.expanded == Some(row.id);
        let color = severity_color(row.severity, cx);
        let id = row.id;

        let mut column = v_flex()
            .w_full()
            .gap_1()
            .py_2()
            .px_3()
            .cursor_pointer()
            .border_b_1()
            .border_color(cx.theme().border)
            .when(!row.unread, |this| this.bg(cx.theme().accent.opacity(0.35)))
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .items_center()
                    .child(
                        Icon::new(severity_icon(row.severity))
                            .size_4()
                            .flex_shrink_0()
                            .text_color(color),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(row.message.clone()),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(row.age.clone()),
                    ),
            );

        // 「×N」：去重命中过多次时给个提示。字段一直在模型里（`Entry::occurrences`），
        // 画出来只是让它不白存 —— 不画的话同一件事反复发生用户看不出区别。
        if row.occurrences > 1 {
            column = column.child(
                h_flex()
                    .w_full()
                    .pl_6()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("×{}", row.occurrences)),
            );
        }

        if expanded {
            column = column.child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .pl_6()
                    .pt_1()
                    .border_l_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child(tr("lithe.notifications.details")),
                    )
                    .children(row.details.iter().map(|(name, value)| {
                        h_flex()
                            .w_full()
                            .gap_2()
                            .text_xs()
                            .child(
                                div()
                                    .w_20()
                                    .flex_shrink_0()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(name.clone()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_color(cx.theme().foreground)
                                    .child(value.clone()),
                            )
                    })),
            );
        }

        // `.id(..)` 不是装饰：`on_click` 在 `StatefulInteractiveElement` 上（`div.rs:1300`），
        // 只有 `Stateful<Div>` 实现它（`:4074`），所以可点的 div 必须先有 id。
        column
            .id(ElementId::named_usize("lithe-notification-row", row.id as usize))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle(id, cx)))
            .into_any_element()
    }

    /// 空态：分「一条都没有」与「筛完没剩下」两种，文案不同。
    fn empty_state(&self, filtered: bool, cx: &App) -> AnyElement {
        let title = if filtered {
            tr("lithe.notifications.noMatch")
        } else {
            tr("lithe.notifications.empty")
        };
        v_flex()
            .w_full()
            .flex_1()
            .min_h_0()
            .child(
                Empty::new()
                    // `Empty` 硬编码了 `.border_dashed()`（`empty.rs:74-75`），真机界面没有
                    // 这圈虚线；改边框色为透明关掉（与本模块外那两个空态同一招）。
                    .border_color(cx.theme().transparent)
                    .header(
                        EmptyHeader::new()
                            .media(EmptyMedia::new().child(
                                Icon::new(IconName::Inbox).size_8(),
                            ))
                            .title(EmptyTitle::new().text_sm().child(title)),
                    ),
            )
            .into_any_element()
    }
}

/// `Filter` 的稳定短标识，用作元素 id 的一部分（gpui 的 `ElementId` 要能拼）。
fn option_key(filter: Filter) -> &'static str {
    match filter {
        Filter::All => "all",
        Filter::Info => "info",
        Filter::Warning => "warning",
        Filter::Error => "error",
    }
}

impl Render for NotificationPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.rows(now_ms(), cx);
        let total = self
            .store
            .upgrade()
            .map(|store| store.read(cx).len())
            .unwrap_or(0);

        match layout_for(total, rows.len()) {
            // 中心一条通知都没有 → 整块空态，不画工具条与筛选条（没有可筛的东西）。
            Layout::Empty => self.empty_state(false, cx).into_any_element(),
            // ⚠️ 只要中心还有通知，工具条与筛选条就**必须**画，哪怕筛选/搜索的结果是空的。
            // 少了这一句会出现「切到警告/错误后出不来」：筛到零结果时空态吞掉了整个面板，
            // 于是「全部」「信息」那两个按钮和搜索框一起消失，用户再也点不回去、也清不掉
            // 查询。空态只该占**列表区**。
            Layout::List { filtered } => v_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(self.toolbar(total, cx))
                .child(self.filter_bar(cx))
                .child(
                    v_flex()
                        .id("lithe-notification-list")
                        .w_full()
                        .flex_1()
                        .min_h_0()
                        .gap_0()
                        // 滚动容器**无条件**挂：空列表挂它没有副作用（不会画滚动条），
                        // 而把它放进 `when` 里会让两个分支类型不同（`child` 给
                        // `Stateful<Div>`、`overflow_y_scrollbar` 给 `Scrollable<…>`），
                        // 编译不过。分支只改**内容**。
                        .overflow_y_scrollbar()
                        .when(filtered, |this| this.child(self.empty_state(true, cx)))
                        .when(
                            !filtered,
                            |this| this.children(rows.iter().map(|row| self.row(row, cx))),
                        ),
                )
                .into_any_element(),
        }
    }
}

/// 面板这一帧画什么。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Layout {
    /// 整块空态「暂无通知」。
    Empty,
    /// 工具条 + 筛选条 + 列表区。`filtered` = 列表区该显示「没有匹配」而不是行。
    List {
        /// 筛选/搜索之后一条都不剩。
        filtered: bool,
    },
}

/// 布局判据。
///
/// `total` 是**筛选前**的条数，`visible` 是筛选后的。判据只看 `total` —— 这正是那个 bug
/// 的要害：拿 `visible` 去判的话，筛到零结果就会走 `Empty` 分支，把工具条和筛选条一起
/// 吞掉，用户点不回「全部」。
fn layout_for(total: usize, visible: usize) -> Layout {
    if total == 0 {
        return Layout::Empty;
    }
    Layout::List {
        filtered: visible == 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{Filter, Layout, Severity, format_age, layout_for, option_key};
    use lithe_db_gpui_shared::tr;

    /// 筛选的放行判据：`All` 全放，其余只放自己那一档。
    ///
    /// 这条守住「筛选不是按文案前缀匹配」—— 中心没有 `success` 档，所以三档互斥。
    #[test]
    fn filter_admits_exactly_its_own_severity() {
        assert!(Filter::All.admits(Severity::Info));
        assert!(Filter::All.admits(Severity::Warning));
        assert!(Filter::All.admits(Severity::Error));

        assert!(Filter::Info.admits(Severity::Info));
        assert!(!Filter::Info.admits(Severity::Warning));
        assert!(!Filter::Info.admits(Severity::Error));

        assert!(Filter::Warning.admits(Severity::Warning));
        assert!(!Filter::Warning.admits(Severity::Error));

        assert!(Filter::Error.admits(Severity::Error));
        assert!(!Filter::Error.admits(Severity::Info));
    }

    /// 回归：筛到零结果时**工具条与筛选条必须还在**。
    ///
    /// 曾经的 bug：布局判据用的是筛选**后**的条数，于是切到「警告」/「错误」而一条都
    /// 不匹配时，整块空态把工具条与筛选条一起吞掉 —— 用户点不回「全部」，搜索框也跟着
    /// 消失（搜到零结果时同样出不来）。这条把「判据只看筛选前的条数」钉住。
    #[test]
    fn chrome_survives_an_empty_filtered_result() {
        // 中心有 3 条，但当前筛选一条都不匹配 → 仍要画工具条与筛选条。
        assert_eq!(
            layout_for(3, 0),
            Layout::List { filtered: true },
            "筛到零结果不能吞掉筛选条，否则用户出不来"
        );
        // 有匹配 → 同样画chrome，只是列表区显示行。
        assert_eq!(layout_for(3, 2), Layout::List { filtered: false });
        // 中心真的空了 → 整块空态（此时没有可筛的东西，不画筛选条是对的）。
        assert_eq!(layout_for(0, 0), Layout::Empty);
    }

    /// 筛选条的四个元素 id 必须互不相同 —— gpui 的 `ElementId` 撞了会让点击串到别的按钮上。
    #[test]
    fn filter_option_ids_are_distinct() {
        let ids: Vec<_> = Filter::options().map(|(option, _)| option_key(option)).collect();
        assert_eq!(ids, vec!["all", "info", "warning", "error"]);

        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "元素 id 不能重复");
    }

    /// 每一档的文案键都要真的解析得出，且 `filterSuccess` 不在其中
    /// （中心没有 success 档）。
    #[test]
    fn every_filter_label_resolves() {
        for (option, key) in Filter::options() {
            let label = option.label();
            assert!(!label.is_empty(), "{key} 解析出空文案");
            assert_ne!(key, "lithe.notifications.filterSuccess");
        }
    }

    /// 相对时间的四档边界。时钟由参数传入，所以这条不需要真等
    /// （`write-stable-tests` 的「确定性时间」要求）。
    #[test]
    fn age_buckets_cover_the_four_ranges() {
        let now = 1_000_000_000u64;
        assert_eq!(format_age(now, now), tr("lithe.notifications.justNow"));
        assert_eq!(
            format_age(now, now - 60_000),
            tr_args_count("lithe.notifications.minutesAgo", 1)
        );
        assert_eq!(
            format_age(now, now - 59 * 60_000),
            tr_args_count("lithe.notifications.minutesAgo", 59)
        );
        assert_eq!(
            format_age(now, now - 60 * 60_000),
            tr_args_count("lithe.notifications.hoursAgo", 1)
        );
        assert_eq!(
            format_age(now, now - 24 * 60 * 60_000),
            tr_args_count("lithe.notifications.daysAgo", 1)
        );
    }

    /// 时间戳在「未来」时不能 panic 也不能变成天文数字 —— `saturating_sub` 那一层。
    #[test]
    fn age_tolerates_a_future_timestamp() {
        let now = 1_000u64;
        assert_eq!(format_age(now, now + 5_000), tr("lithe.notifications.justNow"));
    }

    /// 复现 `tr_args` 的调用形状，让上两条断言不依赖内部实现。
    fn tr_args_count(key: &str, count: u64) -> SharedStringAlias {
        let count = count.to_string();
        lithe_db_gpui_shared::tr_args(key, &[("count", count.as_str())])
    }

    type SharedStringAlias = gpui_kit::SharedString;
}
