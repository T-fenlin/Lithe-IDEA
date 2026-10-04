use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::checkbox::Checkbox;
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

const TAG: &str = "S1_SEARCH";

const SEARCH_DEBOUNCE_MS: u64 = 250;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QueryOption {
    CaseSensitive,
    WholeWords,
    RegularExpression,
}

impl QueryOption {
    const ALL: [QueryOption; 3] = [
        Self::CaseSensitive,
        Self::WholeWords,
        Self::RegularExpression,
    ];

    fn id(self) -> &'static str {
        match self {
            Self::CaseSensitive => "case-sensitive",
            Self::WholeWords => "whole-words",
            Self::RegularExpression => "regular-expression",
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            Self::CaseSensitive => "lithe.search.matchCase",
            Self::WholeWords => "lithe.search.matchWholeWord",
            Self::RegularExpression => "lithe.search.useRegex",
        }
    }

    fn get(self, options: &SearchOptions) -> bool {
        match self {
            Self::CaseSensitive => options.case_sensitive,
            Self::WholeWords => options.whole_words,
            Self::RegularExpression => options.regular_expression,
        }
    }

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

pub struct SearchPanel {
    root: PathBuf,
    query: Entity<InputState>,
    mask: Entity<InputState>,
    query_text: SharedString,
    mask_text: SharedString,
    options: SearchOptions,
    state: SearchState,
    timeline: QueryTimeline,
    debounce_task: Option<Task<()>>,
    on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static>,
    _subscriptions: Vec<Subscription>,
    search_index_readiness: SearchIndexReadiness,
}

impl SearchPanel {
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
            search_index_readiness: SearchIndexReadiness::default(),
        }
    }

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

    pub fn activate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.query.read(cx).focus_handle(cx).focus(window, cx);
    }

    fn on_input_event(&mut self, event: &InputEvent, cx: &mut Context<Self>) {
        match event {
            InputEvent::Change => {
                self.query_text = self.query.read(cx).value();
                self.mask_text = self.mask.read(cx).value();
                self.options.file_mask = self.mask_text.to_string();
                self.restart(cx);
            }
            InputEvent::PressEnter { .. } => self.issue(cx),
            InputEvent::Focus | InputEvent::Blur => {}
        }
    }

    fn restart(&mut self, cx: &mut Context<Self>) {
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        if self.query_text.trim().is_empty() {
            self.debounce_task = None;
            self.state = SearchState::Idle;
            Self::diagnose("idle", "query=empty");
            cx.notify();
            return;
        }
        self.debounce_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(SEARCH_DEBOUNCE_MS))
                .await;
            let _ = this.update(cx, |panel, cx| panel.issue(cx));
        }));
        cx.notify();
    }

    fn issue(&mut self, cx: &mut Context<Self>) {
        let query = self.query_text.trim().to_string();
        if query.is_empty() {
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

    fn adopt(
        &mut self,
        generation: u64,
        query: &str,
        result: Result<Option<Value>, CoreError>,
        cx: &mut Context<Self>,
    ) {
        if !self.timeline.accepts(generation) {
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
                Err(error) => SearchState::failed(query, error.token()),
            },
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
                    if files == 0 && hits == 0 {
                        "empty"
                    } else {
                        "ready"
                    }
                ),
            ),
            _ => Self::diagnose("unexpected_state", &format!("generation={generation}")),
        }
        self.state = next;
        self.timeline.settle(generation);
        cx.notify();
    }

    fn set_option(&mut self, option: QueryOption, value: bool, cx: &mut Context<Self>) {
        if !option.set(&mut self.options, value) {
            return;
        }
        Self::diagnose("option", &format!("id={} value={value}", option.id()));
        self.restart(cx);
    }

    fn cancel_in_background(operation_id: String, cx: &mut Context<Self>) {
        cx.background_spawn(async move {
            let requested = lithe_gpui_shared::cancel_operation(&operation_id);
            Self::diagnose(
                "cancel",
                &format!("operation={operation_id} requested={requested}"),
            );
        })
        .detach();
    }

    fn diagnose(event: &str, fields: &str) {
        eprintln!("{TAG} {event} {fields}");
    }

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
                        rems(14. / 16.),
                    ))
                    .aria_label(tr("lithe.search.gpui.queryPlaceholder")),
            )
            .child(
                Input::new(&self.mask)
                    .small()
                    .cleanable(true)
                    .w_full()
                    .aria_label(tr("lithe.search.filesToInclude")),
            )
            .child(options)
            .into_any_element()
    }

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
            SearchState::Failed { reason, .. } => Self::empty_state(
                Some(IconName::TriangleAlert),
                tr("lithe.search.failed"),
                Some(SharedString::from(reason.clone())),
                cx,
            ),
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

    fn row(&self, row: &SearchRow, cx: &mut Context<Self>) -> AnyElement {
        let open = self.on_open.clone();
        match row {
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
                            .child(SharedString::from(hits.to_string())),
                    )
                    .on_click(cx.listener(move |_this, _, window, cx| {
                        Self::diagnose("open", &format!("file={} line=None", open_path.display()));
                        open(open_path.clone(), None, window, cx);
                    }))
                    .into_any_element()
            }
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

    fn empty_state(
        icon: Option<IconName>,
        title: SharedString,
        description: Option<SharedString>,
        cx: &App,
    ) -> AnyElement {
        lithe_gpui_shared::empty_state::unavailable(
            icon.map(|icon| {
                Icon::new(icon).size(rems(
                    lithe_gpui_shared::empty_state::UNAVAILABLE_ICON_SIZE_SPEC / 16.,
                ))
            }),
            title,
            description,
            None,
            cx,
        )
    }
}

impl Render for SearchPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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

    #[test]
    fn option_ids_are_distinct_and_cover_every_option() {
        let mut ids: Vec<&str> = QueryOption::ALL.iter().map(|option| option.id()).collect();
        assert_eq!(ids.len(), 3, "三个开关一个不多一个不少");
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "选项 id 不能重复");
    }

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

            option.set(&mut options, true);
            for other in QueryOption::ALL {
                if other != option {
                    assert!(!other.get(&options), "{other:?} 被 {option:?} 连坐了");
                }
            }
        }
    }

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

    #[test]
    fn the_debounce_window_matches_the_macos_spec() {
        assert_eq!(SEARCH_DEBOUNCE_MS, 250);
    }

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

    #[test]
    fn opening_a_result_joins_the_workspace_root() {
        let root = std::path::Path::new("proj-root");
        assert_eq!(
            absolute(root, "src/A.java"),
            root.join("src").join("A.java")
        );
    }
}
