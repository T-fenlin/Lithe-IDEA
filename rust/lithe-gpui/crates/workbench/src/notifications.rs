use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Selectable as _, Sizable as _,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, WeakEntity, Window, div, rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_image};

use lithe_gpui_notify::{Entry, EntryId, Severity, Store, matches_text};
use lithe_gpui_shared::{tr, tr_args};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

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

pub fn render_message(entry: &Entry) -> SharedString {
    let args: Vec<(&str, &str)> = entry
        .params()
        .iter()
        .map(|(name, value)| (name.as_ref(), value.as_ref()))
        .collect();
    tr_args(entry.code().as_ref(), &args)
}

fn severity_icon(severity: Severity) -> &'static idea::IdeaIcon {
    match severity {
        Severity::Info => &idea::INFO_ICON,
        Severity::Warning => &idea::WARNING_CIRCLE_ICON,
        Severity::Error => &idea::XCIRCLE_ICON,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Info,
    Warning,
    Error,
}

impl Filter {
    const OPTIONS: [(Filter, &'static str); 4] = [
        (Filter::All, "lithe.notifications.filterAll"),
        (Filter::Info, "lithe.notifications.filterInfo"),
        (Filter::Warning, "lithe.notifications.filterWarnings"),
        (Filter::Error, "lithe.notifications.filterErrors"),
    ];

    pub fn options() -> impl Iterator<Item = (Filter, &'static str)> {
        Self::OPTIONS.into_iter()
    }

    pub fn label(self) -> SharedString {
        let key = Self::OPTIONS
            .iter()
            .find(|(option, _)| *option == self)
            .map(|(_, key)| *key)
            .unwrap_or("lithe.notifications.filterAll");
        tr(key)
    }

    fn admits(self, severity: Severity) -> bool {
        match self {
            Filter::All => true,
            Filter::Info => severity == Severity::Info,
            Filter::Warning => severity == Severity::Warning,
            Filter::Error => severity == Severity::Error,
        }
    }
}

struct Row {
    id: EntryId,
    severity: Severity,
    message: SharedString,
    age: SharedString,
    unread: bool,
    occurrences: u32,
    details: Vec<(SharedString, SharedString)>,
}

pub struct NotificationPanel {
    store: WeakEntity<Store>,
    search: Entity<InputState>,
    query: SharedString,
    filter: Filter,
    expanded: Option<EntryId>,
    _store_subscription: Subscription,
    _search_subscription: Subscription,
}

impl NotificationPanel {
    pub fn new(store: &Entity<Store>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr("lithe.notifications.search")));

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

    fn toggle(&mut self, id: EntryId, cx: &mut Context<Self>) {
        self.expanded = if self.expanded == Some(id) {
            None
        } else {
            Some(id)
        };
        Self::diagnose_toggle(id, self.expanded.is_some());
        cx.notify();
    }

    fn diagnose_toggle(id: EntryId, expanded: bool) {
        eprintln!("S1_NOTIFICATION action=toggle id={id} expanded={expanded}");
    }

    fn diagnose_clear(removed: usize) {
        eprintln!("S1_NOTIFICATION action=clear removed={removed}");
    }

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
                    .prefix(idea_icon_image(
                        &idea::MAGNIFYING_GLASS_ICON,
                        cx,
                        rems(14. / 16.),
                    ))
                    .aria_label(tr("lithe.notifications.search")),
            )
            .child(
                Button::new("lithe-notifications-clear-all")
                    .ghost()
                    .small()
                    .child(idea_icon_image(&idea::TRASH_ICON, cx, rems(1.)))
                    .px_0()
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

    fn row(&self, row: &Row, cx: &mut Context<Self>) -> AnyElement {
        let expanded = self.expanded == Some(row.id);
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
                    .child(idea_icon_image(severity_icon(row.severity), cx, rems(1.)))
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

        column
            .id(ElementId::named_usize(
                "lithe-notification-row",
                row.id as usize,
            ))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle(id, cx)))
            .into_any_element()
    }

    fn empty_state(&self, filtered: bool, cx: &App) -> AnyElement {
        let title = if filtered {
            tr("lithe.notifications.noMatch")
        } else {
            tr("lithe.notifications.empty")
        };
        lithe_gpui_shared::empty_state::unavailable(
            Some(Icon::new(IconName::Inbox).size(rems(
                lithe_gpui_shared::empty_state::UNAVAILABLE_ICON_SIZE_SPEC / 16.,
            ))),
            title,
            None,
            None,
            cx,
        )
    }
}

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
            Layout::Empty => self.empty_state(false, cx).into_any_element(),
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
                        .overflow_y_scrollbar()
                        .when(filtered, |this| this.child(self.empty_state(true, cx)))
                        .when(!filtered, |this| {
                            this.children(rows.iter().map(|row| self.row(row, cx)))
                        }),
                )
                .into_any_element(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Layout {
    Empty,
    List { filtered: bool },
}

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
    use lithe_gpui_shared::tr;

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

    #[test]
    fn chrome_survives_an_empty_filtered_result() {
        assert_eq!(
            layout_for(3, 0),
            Layout::List { filtered: true },
            "筛到零结果不能吞掉筛选条，否则用户出不来"
        );
        assert_eq!(layout_for(3, 2), Layout::List { filtered: false });
        assert_eq!(layout_for(0, 0), Layout::Empty);
    }

    #[test]
    fn filter_option_ids_are_distinct() {
        let ids: Vec<_> = Filter::options()
            .map(|(option, _)| option_key(option))
            .collect();
        assert_eq!(ids, vec!["all", "info", "warning", "error"]);

        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "元素 id 不能重复");
    }

    #[test]
    fn every_filter_label_resolves() {
        for (option, key) in Filter::options() {
            let label = option.label();
            assert!(!label.is_empty(), "{key} 解析出空文案");
            assert_ne!(key, "lithe.notifications.filterSuccess");
        }
    }

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

    #[test]
    fn age_tolerates_a_future_timestamp() {
        let now = 1_000u64;
        assert_eq!(
            format_age(now, now + 5_000),
            tr("lithe.notifications.justNow")
        );
    }

    fn tr_args_count(key: &str, count: u64) -> SharedStringAlias {
        let count = count.to_string();
        lithe_gpui_shared::tr_args(key, &[("count", count.as_str())])
    }

    type SharedStringAlias = gpui_kit::SharedString;
}
