use std::path::PathBuf;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_image, idea_icon_svg_px};
use lithe_gpui_shared::{CoreClient, CoreError, CoreRequest, tr, tr_args};
use lithe_ui::buttons::Button;
use lithe_ui::forms::{Input, InputEvent, InputState};
use lithe_ui::layout::{h_stack, v_stack};
use lithe_ui::motion::Spinner;
use lithe_ui::overlays::Dialog;
use lithe_ui::shell::{WindowRootExt as _, dismiss_hosted};
use lithe_ui::theme::{ActiveTheme as _, Size};
use serde_json::Value;

use crate::search_index::SearchIndexReadiness;
use crate::search_model::{
    QueryTimeline, REPLACE_PREVIEW_COMMAND, ReplaceOptions, ReplaceRow, ReplaceState,
    parse_replace_preview, replace_payload,
};

const TAG: &str = "S1_REPLACE_PREVIEW";

const PANEL_WIDTH: f32 = 650.;
const PANEL_HEIGHT: f32 = 614.;

const ROW_HEIGHT_SPEC: f32 = 46.;
const ROW_ICON_SPEC: f32 = 14.;
const FOOTER_HEIGHT_SPEC: f32 = 52.;

const OVERLAY_ID: &str = "lithe-replace-in-files";
const LIST_ID: &str = "lithe-replace-preview-results";
const PREVIEW_BUTTON_ID: &str = "lithe-replace-preview-run";

pub const REPLACE_KEY: &str = "ctrl-shift-r";

pub fn install_actions(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(REPLACE_KEY, OpenReplaceInFiles, None)]);
    cx.on_action(|_: &OpenReplaceInFiles, cx: &mut App| {
        let Some(window_handle) = cx.active_window() else {
            return;
        };
        let _ = window_handle.update(cx, |_, window, cx| {
            let Some(shell) = crate::command_palette::shell().and_then(|shell| shell.upgrade())
            else {
                return;
            };
            let _ = shell.update(cx, |shell, cx| shell.open_replace_in_files(window, cx));
        });
    });
}

gpui::actions!(lithe_workbench, [OpenReplaceInFiles]);

pub(crate) fn open_dialog(panel: Entity<ReplaceInFiles>, window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        ReplaceInFiles::diagnose("open", "false reason=active_dialog");
        return;
    }
    ReplaceInFiles::diagnose("open", "true");
    let handle = panel.clone();
    window.open_dialog(
        "lithe-replace-in-files",
        move |id, window, _cx| {
            let rem = window.rem_size();
            let viewport = window.viewport_size();
            let height = crate::rem_px(rem, PANEL_HEIGHT)
                .min((viewport.height - rem * 2.).max(gpui::Pixels::ZERO));
            let handle = handle.clone();
            let teardown = handle.clone();
            Dialog::new(id)
                // 居中卡片（上游的 `margin_top((viewport-height)/2)` 等价于默认居中），
                // 高度给死；正文外面套一层定高容器，面板里的 `h_full()` 才有确定父级。
                .on_close(move |window, cx| {
                    let _ = teardown.update(cx, |panel, cx| panel.close(window, cx));
                    dismiss_hosted(window, cx);
                })
                .w(crate::rem_px(rem, PANEL_WIDTH))
                .h(height)
                .p_0()
                .gap_0()
                .child(div().h(height).child(handle))
                .into_any_element()
        },
        cx,
    );
    let _ = panel.update(cx, |panel, cx| panel.focus_query(window, cx));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReplaceOption {
    CaseSensitive,
    WholeWords,
    RegularExpression,
    PreserveCase,
}

impl ReplaceOption {
    #[cfg(test)]
    const ALL: [ReplaceOption; 4] = [
        Self::CaseSensitive,
        Self::WholeWords,
        Self::RegularExpression,
        Self::PreserveCase,
    ];

    const QUERY_ROW: [ReplaceOption; 3] = [
        Self::CaseSensitive,
        Self::WholeWords,
        Self::RegularExpression,
    ];

    fn id(self) -> &'static str {
        match self {
            Self::CaseSensitive => "case-sensitive",
            Self::WholeWords => "whole-words",
            Self::RegularExpression => "regular-expression",
            Self::PreserveCase => "preserve-case",
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            Self::CaseSensitive => "lithe.search.matchCase",
            Self::WholeWords => "lithe.search.matchWholeWord",
            Self::RegularExpression => "lithe.search.useRegex",
            Self::PreserveCase => "lithe.search.gpui.preserveCase",
        }
    }

    fn get(self, options: &ReplaceOptions) -> bool {
        match self {
            Self::CaseSensitive => options.search.case_sensitive,
            Self::WholeWords => options.search.whole_words,
            Self::RegularExpression => options.search.regular_expression,
            Self::PreserveCase => options.preserve_case,
        }
    }

    fn set(self, options: &mut ReplaceOptions, value: bool) -> bool {
        let slot = match self {
            Self::CaseSensitive => &mut options.search.case_sensitive,
            Self::WholeWords => &mut options.search.whole_words,
            Self::RegularExpression => &mut options.search.regular_expression,
            Self::PreserveCase => &mut options.preserve_case,
        };
        let changed = *slot != value;
        *slot = value;
        changed
    }

    fn enabled(self, options: &ReplaceOptions) -> bool {
        match self {
            Self::PreserveCase => !options.search.regular_expression,
            Self::CaseSensitive | Self::WholeWords | Self::RegularExpression => true,
        }
    }
}

pub struct ReplaceInFiles {
    root: PathBuf,
    query: Entity<InputState>,
    replacement: Entity<InputState>,
    mask: Entity<InputState>,
    query_text: SharedString,
    replacement_text: SharedString,
    mask_text: SharedString,
    options: ReplaceOptions,
    state: ReplaceState,
    timeline: QueryTimeline,
    _subscriptions: Vec<Subscription>,
    search_index_readiness: SearchIndexReadiness,
}

impl ReplaceInFiles {
    pub(crate) fn new(
        root: PathBuf,
        search_index_readiness: SearchIndexReadiness,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx.new(|cx| {
            InputState::new(window, cx).placeholder(tr("lithe.search.gpui.queryPlaceholder"))
        });
        let replacement =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr("lithe.search.replaceWith")));
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
        let subscriptions = vec![
            subscribe(&query, cx),
            subscribe(&replacement, cx),
            subscribe(&mask, cx),
        ];

        Self {
            root,
            query,
            replacement,
            mask,
            query_text: SharedString::default(),
            replacement_text: SharedString::default(),
            mask_text: SharedString::default(),
            options: ReplaceOptions::default(),
            state: ReplaceState::Idle,
            timeline: QueryTimeline::default(),
            _subscriptions: subscriptions,
            search_index_readiness,
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

    fn focus_query(&self, window: &mut Window, cx: &mut App) {
        let focus = self.query.read(cx).focus().clone();
        window.focus(&focus, cx);
    }

    fn on_input_event(&mut self, event: &InputEvent, cx: &mut Context<Self>) {
        match event {
            InputEvent::Changed => {
                self.query_text = SharedString::from(self.query.read(cx).text().to_string());
                self.replacement_text =
                    SharedString::from(self.replacement.read(cx).text().to_string());
                self.mask_text = SharedString::from(self.mask.read(cx).text().to_string());
                self.options.search.file_mask = self.mask_text.to_string();
                self.discard_preview(cx);
            }
            InputEvent::Submit => self.issue(cx),
            InputEvent::Focus | InputEvent::Blur => {}
        }
    }

    fn discard_preview(&mut self, cx: &mut Context<Self>) {
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        if !matches!(self.state, ReplaceState::Idle) {
            self.state = ReplaceState::Idle;
            cx.notify();
        }
    }

    fn issue(&mut self, cx: &mut Context<Self>) {
        let query = self.query_text.trim().to_string();
        if query.is_empty() {
            self.state = ReplaceState::Idle;
            Self::diagnose("idle", "query=empty");
            cx.notify();
            return;
        }
        let scheduled = self.timeline.schedule();
        if let Some(superseded) = scheduled.superseded.clone() {
            Self::cancel_in_background(superseded, cx);
        }
        let generation = scheduled.generation;
        let operation_id = scheduled.operation_id.clone();
        let root = self.root.to_string_lossy().to_string();
        let replacement = self.replacement_text.to_string();
        let options = self.options.clone();
        let previous = self.state.files().to_vec();
        self.state = ReplaceState::loading(query.clone(), &previous);
        Self::diagnose(
            "issue",
            &format!(
                "generation={generation} operation={operation_id} query={query} \
                 replacement_len={} case={} whole={} regex={} preserve={} mask={}",
                replacement.len(),
                options.search.case_sensitive,
                options.search.whole_words,
                options.search.regular_expression,
                options.preserve_case,
                options.search.file_mask,
            ),
        );
        cx.notify();

        let background_query = query.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let request = CoreRequest::command(REPLACE_PREVIEW_COMMAND)
                        .with_payload(replace_payload(
                            &root,
                            &background_query,
                            &replacement,
                            &options,
                        ))
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
            Ok(data) => match parse_replace_preview(data.as_ref()) {
                Ok(files) => ReplaceState::ready(query, &files),
                Err(error) => ReplaceState::failed(query, error.token()),
            },
            Err(error) => ReplaceState::failed(query, error.to_string()),
        };
        let (files, occurrences) = next.counts();
        match &next {
            ReplaceState::Failed { reason, .. } => Self::diagnose(
                "failed",
                &format!("generation={generation} query={query} reason={reason}"),
            ),
            ReplaceState::Ready { .. } => Self::diagnose(
                "complete",
                &format!(
                    "generation={generation} query={query} files={files} matches={occurrences} state={}",
                    if files == 0 { "empty" } else { "ready" }
                ),
            ),
            _ => Self::diagnose("unexpected_state", &format!("generation={generation}")),
        }
        self.state = next;
        self.timeline.settle(generation);
        cx.notify();
    }

    fn set_option(&mut self, option: ReplaceOption, value: bool, cx: &mut Context<Self>) {
        if !option.enabled(&self.options) {
            Self::diagnose("option", &format!("id={} ignored=disabled", option.id()));
            return;
        }
        if !option.set(&mut self.options, value) {
            return;
        }
        Self::diagnose("option", &format!("id={} value={value}", option.id()));
        self.discard_preview(cx);
    }

    fn close(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        Self::diagnose("close", "true");
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

    fn header(&self, cx: &App) -> AnyElement {
        h_stack()
            .w_full()
            .flex_shrink_0()
            .h_10()
            .px_5()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().colors.fg)
                    .child(tr("lithe.search.gpui.replaceInFiles")),
            )
            .child(div().flex_1())
            // 库里的 `Input` 是个 `IntoElement` 的框，不吃 `Styled`；宽度得外包一层给。
            .child(
                div().w(rems(96. / 16.)).child(
                    Input::new(&self.mask)
                        .size(Size::Small)
                        .clearable()
                        .aria_label(tr("lithe.search.filesToInclude")),
                ),
            )
            .into_any_element()
    }

    fn controls(&self, cx: &mut Context<Self>) -> AnyElement {
        v_stack()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .px_3()
            .pb_2p5()
            .child(
                Input::new(&self.query)
                    .size(Size::Small)
                    .clearable()
                    .prefix(idea_icon_image(
                        &idea::MAGNIFYING_GLASS_ICON,
                        cx,
                        rems(ROW_ICON_SPEC / 16.),
                    ))
                    .aria_label(tr("lithe.search.gpui.queryPlaceholder")),
            )
            .child(self.option_row(ReplaceOption::QUERY_ROW.as_slice(), cx))
            .child(
                Input::new(&self.replacement)
                    .size(Size::Small)
                    .clearable()
                    .aria_label(tr("lithe.search.replaceWith")),
            )
            .child(self.option_row(&[ReplaceOption::PreserveCase], cx))
            .into_any_element()
    }

    fn option_row(&self, options: &[ReplaceOption], cx: &mut Context<Self>) -> AnyElement {
        let mut row = h_stack().w_full().flex_shrink_0().gap_1();
        for option in options {
            let option = *option;
            row = row.child(
                Button::new(ElementId::from(format!(
                    "lithe-replace-option-{}",
                    option.id()
                )))
                .size(Size::Small)
                .selected(option.get(&self.options))
                .disabled(!option.enabled(&self.options))
                .label(tr(option.label_key()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let next = !option.get(&this.options);
                    this.set_option(option, next, cx);
                })),
            );
        }
        row.into_any_element()
    }

    fn results(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match &self.state {
            ReplaceState::Idle => Self::empty_state(
                Some(&idea::MAGNIFYING_GLASS_ICON),
                tr("lithe.search.gpui.replaceInFiles"),
                Some(tr("lithe.search.gpui.replaceEmptyDescription")),
                cx,
            ),
            ReplaceState::Loading { .. } => h_stack()
                .w_full()
                .flex_1()
                .min_h_0()
                .justify_center()
                .gap_2()
                .child(
                    Spinner::new("lithe-replace-spinner")
                        .color(cx.theme().colors.fg_muted),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(tr("lithe.fileExplorer.searchingFiles")),
                )
                .into_any_element(),
            ReplaceState::Failed { reason, .. } => Self::empty_state(
                Some(&idea::WARNING_CIRCLE_ICON),
                tr("lithe.search.failed"),
                Some(SharedString::from(reason.clone())),
                cx,
            ),
            ReplaceState::Ready { files, .. } if files.is_empty() => Self::empty_state(
                None,
                tr_args(
                    "lithe.search.noResultsFor",
                    &[("query", self.state.query())],
                ),
                None,
                cx,
            ),
            ReplaceState::Ready { .. } => {
                let rows = self.state.rows();
                self.list(&rows, cx)
            }
        }
    }

    fn list(&self, rows: &[ReplaceRow], cx: &mut Context<Self>) -> AnyElement {
        v_stack()
            .id(LIST_ID)
            .w_full()
            .flex_1()
            .min_h_0()
            .gap_0()
            .overflow_y_scroll()
            .children(rows.iter().map(|row| self.row(row, cx)))
            .into_any_element()
    }

    fn row(&self, row: &ReplaceRow, cx: &App) -> AnyElement {
        match row {
            ReplaceRow::File { path, occurrences } => h_stack()
                .id(ElementId::from(format!("replace-file-{path}")))
                .w_full()
                .gap_2()
                .px_2()
                .py_1()
                .child(idea_icon_image(
                    &idea::FILE_ICON,
                    cx,
                    rems(ROW_ICON_SPEC / 16.),
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_color(cx.theme().colors.fg)
                        .child(SharedString::from(path.clone())),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(SharedString::from(occurrences.to_string())),
                )
                .into_any_element(),
            ReplaceRow::Line {
                path,
                line,
                before,
                after,
                ..
            } => h_stack()
                .id(ElementId::from(format!("replace-line-{path}-{line}")))
                .w_full()
                .h(rems(ROW_HEIGHT_SPEC / 16.))
                .gap_2()
                .pl_4()
                .pr_2()
                .py_1()
                .child(
                    div()
                        .w_8()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(SharedString::from(line.to_string())),
                )
                .child(
                    v_stack()
                        .flex_1()
                        .min_w_0()
                        .gap_0()
                        .child(Self::text_pair(
                            tr("lithe.search.gpui.before"),
                            before,
                            cx.theme().colors.fg_muted,
                            cx,
                        ))
                        .child(Self::text_pair(
                            tr("lithe.search.gpui.after"),
                            after,
                            cx.theme().colors.fg,
                            cx,
                        )),
                )
                .into_any_element(),
        }
    }

    fn text_pair(label: SharedString, text: &str, color: Hsla, cx: &App) -> AnyElement {
        h_stack()
            .w_full()
            .min_w_0()
            .gap_2()
            .child(
                div()
                    .w(rems(48. / 16.))
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .font_family(cx.theme().mono_family.clone())
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_color(color)
                    .child(SharedString::from(text.to_string())),
            )
            .into_any_element()
    }

    fn footer(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let (files, matches) = self.state.counts();
        let loading = matches!(self.state, ReplaceState::Loading { .. });
        let can_preview = preview_enabled(&self.state, &self.query_text);
        h_stack()
            .w_full()
            .flex_shrink_0()
            .h(rems(FOOTER_HEIGHT_SPEC / 16.))
            .px_5()
            .gap_2()
            .border_t_1()
            .border_color(cx.theme().colors.border)
            .child(
                Button::new(PREVIEW_BUTTON_ID)
                    .primary()
                    .label(tr("lithe.search.gpui.preview"))
                    .disabled(!can_preview)
                    .on_click(cx.listener(|this, _, _, cx| this.issue(cx))),
            )
            .child(div().flex_1())
            .child(div().w_4().flex_shrink_0().when(loading, |slot| {
                slot.child(
                    Spinner::new("lithe-replace-footer-spinner")
                        .color(cx.theme().colors.fg_muted),
                )
            }))
            .child(
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(tr_args(
                        "lithe.search.gpui.replacePreviewCounts",
                        &[
                            ("files", &files.to_string()),
                            ("matches", &matches.to_string()),
                        ],
                    )),
            )
            .into_any_element()
    }

    fn empty_state(
        icon: Option<&'static idea::IdeaIcon>,
        title: SharedString,
        description: Option<SharedString>,
        cx: &App,
    ) -> AnyElement {
        lithe_gpui_shared::empty_state::unavailable(
            icon.map(|icon| {
                idea_icon_svg_px(
                    icon,
                    cx,
                    rems(lithe_gpui_shared::empty_state::UNAVAILABLE_ICON_SIZE_SPEC / 16.),
                )
                .text_color(cx.theme().colors.fg_muted)
            }),
            title,
            description,
            None,
            cx,
        )
    }
}

fn preview_enabled(state: &ReplaceState, query: &str) -> bool {
    !query.trim().is_empty() && !matches!(state, ReplaceState::Loading { .. })
}

impl Render for ReplaceInFiles {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let index_notice = crate::search_index::readiness_notice(self.search_index_readiness, cx);
        v_stack()
            .id(OVERLAY_ID)
            .size_full()
            .child(self.header(cx))
            .child(self.controls(cx))
            .children(index_notice)
            .child(self.results(cx))
            .child(self.footer(cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_model::{ReplacementFile, ReplacementLine};

    #[test]
    fn option_ids_are_distinct_and_cover_every_option() {
        let mut ids: Vec<&str> = ReplaceOption::ALL
            .iter()
            .map(|option| option.id())
            .collect();
        assert_eq!(ids.len(), 4, "四颗开关一个不多一个不少");
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "选项 id 不能重复");
        assert_eq!(ReplaceOption::QUERY_ROW.len(), 3);
        assert!(!ReplaceOption::QUERY_ROW.contains(&ReplaceOption::PreserveCase));
    }

    #[test]
    fn preserve_case_is_disabled_only_while_the_regex_option_is_on() {
        let mut options = ReplaceOptions::default();
        assert!(ReplaceOption::PreserveCase.enabled(&options));
        assert!(ReplaceOption::CaseSensitive.enabled(&options));
        options.search.regular_expression = true;
        assert!(
            !ReplaceOption::PreserveCase.enabled(&options),
            "正则打开时 Core 根本不读 preserveCase —— 画一颗拨了没用的开关就是骗人"
        );
        assert!(
            ReplaceOption::CaseSensitive.enabled(&options),
            "大小写敏感在正则打开时仍然有意义"
        );
    }

    #[test]
    fn preview_button_needs_a_query_and_no_preview_in_flight() {
        let files = vec![ReplacementFile {
            path: "src/A.java".to_string(),
            lines: vec![ReplacementLine {
                line: 1,
                before: "a".to_string(),
                after: "b".to_string(),
                occurrences: 1,
            }],
        }];
        assert!(!preview_enabled(&ReplaceState::Idle, ""));
        assert!(
            !preview_enabled(&ReplaceState::Idle, "   "),
            "只有空白也算空查询（Core 会 trim，`files.rs:341`）"
        );
        assert!(preview_enabled(&ReplaceState::Idle, "needle"));
        assert!(
            !preview_enabled(&ReplaceState::loading("needle", &files), "needle"),
            "一次预览在路上时不许再点（真源同一判据）"
        );
        assert!(preview_enabled(
            &ReplaceState::ready("needle", &files),
            "needle"
        ));
        assert!(preview_enabled(
            &ReplaceState::failed("needle", "invalid_request"),
            "needle"
        ));
    }

    #[test]
    fn each_option_reads_and_writes_its_own_slot() {
        let mut options = ReplaceOptions::default();
        for option in ReplaceOption::ALL {
            assert!(!option.get(&options), "默认全关");
            assert!(option.set(&mut options, true), "第一次拨要报 changed");
            assert!(option.get(&options), "{} 必须真的写进它那一格", option.id());
            assert!(!option.set(&mut options, true), "同一个值不算 changed");
            assert!(option.set(&mut options, false), "拨回去也要报 changed");
            assert!(!option.get(&options));
        }
        assert_eq!(options, ReplaceOptions::default());
    }
}
