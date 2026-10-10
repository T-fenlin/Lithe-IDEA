use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, FontWeight,
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Task, Window, div, rems,
};
use lithe_gpui_shared::icons::{FileIcon, file_icon, idea, idea_icon_image, idea_icon_svg_px};
use lithe_gpui_shared::{CoreClient, CoreError, CoreRequest, tr};
use lithe_ui::forms::{InputEvent, InputState};
use lithe_ui::layout::{h_stack, v_stack};
use lithe_ui::navigation::{Palette, PaletteRow};
use lithe_ui::overlays::Dialog;
use lithe_ui::shell::{WindowRootExt as _, dismiss_hosted};
use lithe_ui::theme::ActiveTheme as _;
use serde_json::Value;

use crate::command_palette::{CommandAction, CommandId};
use crate::search_index::SearchIndexReadiness;
use crate::search_model::{
    COMMANDS_EMPTY_KEY, EVERYWHERE_COMMAND, EVERYWHERE_MAX_RESULTS, EverywhereRow, EverywhereState,
    QueryTimeline, SearchEverywhereScope, SearchOptions, absolute, action_rows, command_query,
    is_command_query, issues_core_request, parse_matches, payload_with_limit, uses_command_catalog,
};

const TAG: &str = "S1_SEARCH_EVERYWHERE";

const DEBOUNCE_MS: u64 = 180;

const PANEL_WIDTH: f32 = 860.;
const PANEL_MAX_HEIGHT: f32 = 560.;
const PANEL_TOP_INSET: f32 = 84.;

/// 空查询时说「敲 `/` 找命令」，一旦有字就换成普通提示（与上游 `Command` 每帧现算的一致）。
const PLACEHOLDER_EMPTY_KEY: &str = "lithe.searchEverywhere.typeSlash";
const PLACEHOLDER_TYPED_KEY: &str = "lithe.search.gpui.queryPlaceholder";

const TAB_HEIGHT_SPEC: f32 = 38.;
const TAB_PADDING_SPEC: f32 = 11.;
const TAB_UNDERLINE_SPEC: f32 = 2.;

const ROW_ICON_SPEC: f32 = 14.;

const ACTION_ICON_SPEC: f32 = 11.;
const ACTION_ICON_SLOT_SPEC: f32 = 16.;

const OVERLAY_ID: &str = "search-everywhere";
const SCOPE_ROW_ID: &str = "search-everywhere-scopes";

/// `Palette::new` 的 id 与 `open_dialog` 那个层名刻意不同：前者是元素/键控状态的 key，
/// 后者是宿主浮层栈里的层名。
const PALETTE_ID: &str = "search-everywhere-body";

const SCOPE_KEY_CONTEXT: &str = "SearchEverywhere";

pub fn install_actions(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", NextScope, Some(SCOPE_KEY_CONTEXT)),
        KeyBinding::new("shift-tab", PreviousScope, Some(SCOPE_KEY_CONTEXT)),
    ]);
}

gpui::actions!(lithe_workbench, [NextScope, PreviousScope]);

pub(crate) fn open_dialog(overlay: Entity<SearchEverywhere>, window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        SearchEverywhere::diagnose("open", "false reason=active_dialog");
        return;
    }
    SearchEverywhere::diagnose("open", "true");
    window.open_dialog(
        "lithe-search-everywhere",
        move |id, window, _cx| {
            let rem = window.rem_size();
            let overlay = overlay.clone();
            let teardown = overlay.clone();
            Dialog::new(id)
                .panel()
                .on_close(move |window, cx| {
                    let _ = teardown.update(cx, |overlay, cx| overlay.close(window, cx));
                    dismiss_hosted(window, cx);
                })
                .w(crate::rem_px(rem, PANEL_WIDTH))
                .p_0()
                .gap_0()
                .top_inset(crate::rem_px(rem, PANEL_TOP_INSET))
                .child(overlay)
                .into_any_element()
        },
        cx,
    );
}

pub struct SearchEverywhere {
    root: PathBuf,
    on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static>,
    on_action: Rc<dyn Fn(CommandId, &mut Window, &mut App) + 'static>,
    query: Entity<InputState>,
    /// 查询框一动就重排一次 —— 行是本面板自己算的（库的 `Palette` 不做过滤）。
    _query_changed: Subscription,
    scope: SearchEverywhereScope,
    query_text: SharedString,
    state: EverywhereState,
    timeline: QueryTimeline,
    debounce_task: Option<Task<()>>,
    /// 有一轮在途：行首转圈，且不说「没匹配」。
    loading: bool,
    /// 面板是每次打开现建的：第一帧把光标送进查询框。
    focus_pending: bool,
    search_index_readiness: SearchIndexReadiness,
}

impl SearchEverywhere {
    pub(crate) fn new(
        root: PathBuf,
        on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static>,
        on_action: Rc<dyn Fn(CommandId, &mut Window, &mut App) + 'static>,
        search_index_readiness: SearchIndexReadiness,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr(PLACEHOLDER_EMPTY_KEY)));
        let query_changed = cx.subscribe_in(
            &query,
            window,
            |overlay: &mut Self, field: &Entity<InputState>, event: &InputEvent, window, cx| {
                if *event != InputEvent::Changed {
                    return;
                }
                let text = field.read(cx).text().to_string();
                overlay.on_query(&text, window, cx);
            },
        );
        Self {
            root,
            on_open,
            on_action,
            query,
            _query_changed: query_changed,
            scope: SearchEverywhereScope::All,
            query_text: SharedString::default(),
            state: EverywhereState::Idle,
            timeline: QueryTimeline::default(),
            debounce_task: None,
            loading: false,
            focus_pending: true,
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

    fn action_query(&self) -> Option<String> {
        if !uses_command_catalog(self.scope, &self.query_text) {
            return None;
        }
        Some(
            command_query(&self.query_text)
                .unwrap_or(&self.query_text)
                .to_string(),
        )
    }

    fn items(&self, cx: &App) -> Vec<EverywhereItem> {
        match self.action_query() {
            Some(query) if !self.query_text.trim().is_empty() => {
                action_rows(Self::action_catalog(cx), &query)
                    .into_iter()
                    .map(EverywhereItem::Action)
                    .collect()
            }
            Some(_) => Vec::new(),
            None => self
                .state
                .rows(self.scope)
                .into_iter()
                .map(EverywhereItem::Result)
                .collect(),
        }
    }

    fn action_catalog(cx: &App) -> Vec<CommandAction> {
        crate::command_palette::shell()
            .and_then(|handle| handle.upgrade())
            .map(|shell| shell.read(cx).action_catalog(cx))
            .unwrap_or_default()
    }

    fn abandon(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        self.debounce_task = None;
        self.set_loading(false, window, cx);
    }

    fn on_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.query_text.as_ref() == query {
            return;
        }
        self.query_text = SharedString::from(query);
        // 上游 `Command` 每帧按当前串现算提示语；查询框归调用方之后，提示语是输入框的状态，
        // 得自己跟着串走。置空只 `notify`，不会再发 `Changed`，不会打回这里。
        let placeholder = if self.query_text.trim().is_empty() {
            tr(PLACEHOLDER_EMPTY_KEY)
        } else {
            tr(PLACEHOLDER_TYPED_KEY)
        };
        self.query
            .update(cx, |input, cx| input.set_placeholder(placeholder, cx));
        if uses_command_catalog(self.scope, &self.query_text) {
            self.abandon(window, cx);
            let rows = self.items(cx).len();
            Self::diagnose(
                "actions",
                &format!(
                    "commands={} scope={} query={query} rows={rows} core=false",
                    is_command_query(&self.query_text),
                    self.scope.token()
                ),
            );
            cx.notify();
            return;
        }
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        if self.query_text.trim().is_empty() {
            self.debounce_task = None;
            self.state = EverywhereState::Idle;
            self.set_loading(false, window, cx);
            Self::diagnose("idle", "query=empty");
            cx.notify();
            return;
        }
        self.schedule(window, cx);
        cx.notify();
    }

    fn schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.debounce_task = Some(cx.spawn_in(window, async move |this, async_cx| {
            async_cx
                .background_executor()
                .timer(Duration::from_millis(DEBOUNCE_MS))
                .await;
            let _ = this.update_in(async_cx, |overlay, window, cx| overlay.issue(window, cx));
        }));
    }

    fn issue(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !issues_core_request(self.scope, &self.query_text) {
            Self::diagnose(
                "issue",
                &format!(
                    "skipped=true core=false scope={} commands={}",
                    self.scope.token(),
                    is_command_query(&self.query_text)
                ),
            );
            return;
        }
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
        let previous = self.state.matches().to_vec();
        self.state = EverywhereState::loading(query.clone(), &previous);
        self.set_loading(true, window, cx);
        Self::diagnose(
            "issue",
            &format!(
                "generation={generation} operation={operation_id} scope={} query={query}",
                self.scope.token()
            ),
        );
        cx.notify();

        let background_query = query.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let request = CoreRequest::command(EVERYWHERE_COMMAND)
                        .with_payload(payload_with_limit(
                            &root,
                            &background_query,
                            &SearchOptions::default(),
                            EVERYWHERE_MAX_RESULTS,
                        ))
                        .with_operation_id(operation_id);
                    CoreClient::new().execute(&request)
                })
                .await;
            let _ = this.update_in(cx, |overlay, window, cx| {
                overlay.adopt(generation, &query, result, window, cx)
            });
        })
        .detach();
    }

    fn adopt(
        &mut self,
        generation: u64,
        query: &str,
        result: Result<Option<Value>, CoreError>,
        window: &mut Window,
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
                Ok(matches) => EverywhereState::ready(query, &matches),
                Err(error) => EverywhereState::failed(query, error.token()),
            },
            Err(error) => EverywhereState::failed(query, error.to_string()),
        };
        match &next {
            EverywhereState::Failed { reason, .. } => Self::diagnose(
                "failed",
                &format!("generation={generation} query={query} reason={reason}"),
            ),
            _ => {
                let counts = SearchEverywhereScope::ALL
                    .iter()
                    .map(|scope| format!("{}={}", scope.token(), next.rows(*scope).len()))
                    .collect::<Vec<_>>()
                    .join(" ");
                Self::diagnose(
                    "complete",
                    &format!(
                        "generation={generation} query={query} matches={} core=true {counts}",
                        next.matches().len()
                    ),
                );
            }
        }
        self.state = next;
        self.timeline.settle(generation);
        self.set_loading(false, window, cx);
        cx.notify();
    }

    fn set_scope(
        &mut self,
        scope: SearchEverywhereScope,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.scope == scope {
            return;
        }
        let previous = self.scope;
        self.scope = scope;
        if !is_command_query(&self.query_text) {
            if !scope.is_core_backed() {
                self.abandon(window, cx);
            } else if previous == SearchEverywhereScope::Actions
                && issues_core_request(scope, &self.query_text)
                && !self.state.is_ready_for(&self.query_text)
            {
                self.issue(window, cx);
            }
        }
        let rows = self.items(cx).len();
        Self::diagnose(
            "scope",
            &format!(
                "scope={} rows={rows} core={} commands={}",
                scope.token(),
                scope.is_core_backed(),
                is_command_query(&self.query_text)
            ),
        );
        cx.notify();
    }

    fn on_scope_next(&mut self, _: &NextScope, window: &mut Window, cx: &mut Context<Self>) {
        let next = self.scope.moved(1);
        self.set_scope(next, window, cx);
    }

    fn on_scope_prev(&mut self, _: &PreviousScope, window: &mut Window, cx: &mut Context<Self>) {
        let previous = self.scope.moved(-1);
        self.set_scope(previous, window, cx);
    }

    fn confirm(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.items(cx).into_iter().nth(row) else {
            return;
        };
        // 关面板由 `Palette::on_close` 收（它先叫这里、再收尾）：这一行得在收尾之前读，
        // 收尾（`close`）会把 `state` 复位成 `Idle`、行就没了。
        self.abandon(window, cx);
        match item {
            EverywhereItem::Action(action) => {
                Self::diagnose(
                    "run_action",
                    &format!(
                        "id={} scope={} commands={}",
                        action.id.id(),
                        self.scope.token(),
                        is_command_query(&self.query_text)
                    ),
                );
                (self.on_action)(action.id, window, cx);
            }
            EverywhereItem::Result(target) => {
                let path = absolute(&self.root, &target.path);
                let line = target.line.map(|line| line as u32);
                Self::diagnose(
                    "open_result",
                    &format!(
                        "scope={} file={} line={}",
                        self.scope.token(),
                        path.display(),
                        line.map(|line| line.to_string())
                            .unwrap_or_else(|| "none".to_string())
                    ),
                );
                (self.on_open)(path, line, window, cx);
            }
        }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.abandon(window, cx);
        self.state = EverywhereState::Idle;
        self.scope = SearchEverywhereScope::All;
        Self::diagnose("close", "reason=dialog");
        cx.notify();
    }

    fn set_loading(&mut self, loading: bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
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

    fn scope_tabs(&self, cx: &Context<Self>) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let mut row = h_stack()
            .id(SCOPE_ROW_ID)
            .w_full()
            .h_10()
            .px_1p5()
            .gap_0p5()
            .bg(colors.tab_bar)
            .border_b_1()
            .border_color(colors.border);
        for scope in SearchEverywhereScope::ALL {
            row = row.child(self.scope_tab(scope, cx));
        }
        row
    }

    fn scope_tab(&self, scope: SearchEverywhereScope, cx: &Context<Self>) -> impl IntoElement {
        let active = self.scope == scope;
        let colors = &cx.theme().colors;
        let foreground = if active { colors.fg } else { colors.fg_subtle };
        v_stack()
            .id(ElementId::from(format!(
                "search-everywhere-scope-{}",
                scope.token()
            )))
            .relative()
            .h(rems(TAB_HEIGHT_SPEC / 16.))
            .justify_center()
            .items_center()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| this.set_scope(scope, window, cx)))
            .child(
                h_stack()
                    .px(rems(TAB_PADDING_SPEC / 16.))
                    .text_xs()
                    .font_weight(if active {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(foreground)
                    .child(tr(scope.label_key())),
            )
            .when(active, |this| {
                this.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(rems(TAB_UNDERLINE_SPEC / 16.))
                        .bg(colors.primary),
                )
            })
    }
}

impl Render for SearchEverywhere {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_pending {
            self.focus_pending = false;
            window.focus(&self.query.read(cx).focus().clone(), cx);
        }
        let failure: Option<SharedString> = match &self.state {
            EverywhereState::Failed { reason, .. } => Some(SharedString::from(reason.clone())),
            _ => None,
        };
        let rows: Vec<EverywhereItem> = self.items(cx);
        let empty_query = if uses_command_catalog(self.scope, &self.query_text) {
            self.query_text.trim().is_empty()
        } else {
            self.state.query().trim().is_empty()
        };
        let empty_key = if is_command_query(&self.query_text) {
            COMMANDS_EMPTY_KEY
        } else {
            self.scope.empty_key()
        };
        let index_notice = crate::search_index::readiness_notice(self.search_index_readiness, cx);
        let (danger, quiet) = {
            let colors = &cx.theme().colors;
            (colors.danger, colors.fg_subtle)
        };

        // 空态三态：查询失败（danger 色）、查询为空（什么都不说）、查了但没匹配（quiet 色）。
        let empty: AnyElement = match (&failure, empty_query) {
            (Some(reason), _) => div()
                .py_6()
                .w_full()
                .text_center()
                .text_sm()
                .text_color(danger)
                .child(reason.clone())
                .into_any_element(),
            (None, true) => div().into_any_element(),
            (None, false) => div()
                .py_6()
                .w_full()
                .text_center()
                .text_sm()
                .text_color(quiet)
                .child(tr(empty_key))
                .into_any_element(),
        };

        let close = {
            let overlay = cx.entity();
            move |window: &mut Window, cx: &mut App| {
                let _ = overlay.update(cx, |overlay, cx| overlay.close(window, cx));
                dismiss_hosted(window, cx);
            }
        };
        let pick = {
            let overlay = cx.entity();
            move |row: usize, _: &SharedString, window: &mut Window, cx: &mut App| {
                let _ = overlay.update(cx, |overlay, cx| overlay.confirm(row, window, cx));
            }
        };

        v_stack()
            .id(OVERLAY_ID)
            .key_context(SCOPE_KEY_CONTEXT)
            .w_full()
            .on_action(cx.listener(Self::on_scope_next))
            .on_action(cx.listener(Self::on_scope_prev))
            .child(self.scope_tabs(cx))
            .children(index_notice)
            .child(
                Palette::new(PALETTE_ID, self.query.clone(), close)
                    .rows(rows.into_iter().map(|item| item.palette_row(cx)))
                    .empty(empty)
                    .loading(self.loading)
                    .on_pick(pick)
                    .w_full()
                    .h_auto()
                    .max_h(rems(PANEL_MAX_HEIGHT / 16.))
                    .card(window, cx),
            )
    }
}

enum EverywhereItem {
    Result(EverywhereRow),
    Action(CommandAction),
}

impl EverywhereItem {
    fn palette_row(self, cx: &App) -> PaletteRow {
        match self {
            Self::Result(row) => result_row(row, cx),
            Self::Action(action) => action_row(action, cx),
        }
    }
}

/// 一条动作：[`crate::command_palette::CommandAction`] 的标签 + 描述 + 分类。
///
/// 行首刻意用固定的闪电标（`warning` 色）当「这是命令、不是文件」的记号，
/// 而不是命令自己的图标 —— 与上游 `Command` 里动作行的口径一致。
fn action_row(action: CommandAction, cx: &App) -> PaletteRow {
    let colors = &cx.theme().colors;
    let group = action.category_label();
    let (label, description) = (action.label.clone(), action.description.clone());
    PaletteRow::new(action.id.id(), label.clone())
        .icon_element(
            div()
                .flex_shrink_0()
                .w(rems(ACTION_ICON_SLOT_SPEC / 16.))
                .items_center()
                .justify_center()
                .child(
                    idea_icon_svg_px(&idea::LIGHTNING_ICON, cx, rems(ACTION_ICON_SPEC / 16.))
                        .text_color(colors.warning),
                ),
        )
        .label(
            div()
                .flex_shrink_0()
                .text_xs()
                .text_color(colors.fg)
                .child(label),
        )
        .detail(
            div()
                .min_w_0()
                .truncate()
                .text_xs()
                .text_color(colors.fg_subtle)
                .child(description),
        )
        .end(
            div()
                .flex_shrink_0()
                .text_xs()
                .text_color(colors.fg_subtle)
                .child(group),
        )
}

/// 一条搜索结果：主题文件图标 + 名字（符号/内容还会带 `:行号`）+ 父路径（+ 内容预览）。
fn result_row(row: EverywhereRow, cx: &App) -> PaletteRow {
    let shows_line = row.shows_line();
    let shows_preview = row.shows_preview();
    let class_icon = row.kind.is_type();
    let java_icon = row.kind.is_symbol();
    let EverywhereRow {
        path,
        name,
        parent,
        line,
        preview,
        ..
    } = row;
    let colors = &cx.theme().colors;
    let item_name = SharedString::from(name.clone());
    let icon = if class_icon {
        idea_icon_image(&idea::CLASS_ICON, cx, rems(ROW_ICON_SPEC / 16.))
    } else if java_icon {
        idea_icon_image(&idea::JAVA_ICON, cx, rems(ROW_ICON_SPEC / 16.))
    } else {
        FileIcon::own(file_icon::fallback_icon(item_name.as_ref()))
            .with_theme_icon(item_name.as_ref(), cx)
            .render(rems(ROW_ICON_SPEC / 16.), cx)
    };
    let mut line_mark = h_stack()
        .flex_shrink_0()
        .text_xs()
        .text_color(colors.fg)
        .child(item_name);
    if shows_line {
        if let Some(line) = line {
            line_mark = line_mark.child(
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(colors.fg_subtle)
                    .child(SharedString::from(format!(":{line}"))),
            );
        }
    }
    let mut row = PaletteRow::new(path, name)
        .icon_element(icon)
        .label(line_mark)
        .detail(
            div()
                .min_w_0()
                .truncate()
                .text_xs()
                .text_color(colors.fg_subtle)
                .child(SharedString::from(parent)),
        );
    if shows_preview {
        row = row.end(
            div()
                .min_w_0()
                .truncate()
                .text_xs()
                .text_color(colors.fg_subtle)
                .child(SharedString::from(preview)),
        );
    }
    row
}

#[cfg(test)]
mod tests {

    use std::rc::Rc;

    use gpui::{
        AppContext as _, Bounds, Entity, Point, Size, TestAppContext, Window, WindowBounds,
        WindowHandle, WindowOptions, px,
    };
    use lithe_ui::shell::{WindowRoot, open_rooted_window};

    use super::*;

    fn open_overlay(
        cx: &mut TestAppContext,
    ) -> (WindowHandle<WindowRoot>, Entity<SearchEverywhere>) {
        let on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static> =
            Rc::new(|_, _, _, _| {});
        let on_action: Rc<dyn Fn(CommandId, &mut Window, &mut App) + 'static> =
            Rc::new(|_, _, _| {});
        cx.update(|cx| {
            // 窗口根已换成 lithe-ui 的 `WindowRoot`（门 G1），测试宿主跟着换。
            // `lithe_ui::init` 会校验 `Application::with_assets`，无头测试里没有，
            // 所以只装它需要的主题全局。
            if !cx.has_global::<lithe_ui::theme::Theme>() {
                lithe_ui::theme::Theme::init(cx);
            }
            let (window, overlay) = open_rooted_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: Size {
                            width: px(800.),
                            height: px(600.),
                        },
                    })),
                    ..Default::default()
                },
                cx,
                move |window, cx| {
                    cx.new(|cx| {
                        SearchEverywhere::new(
                            PathBuf::from("D:/proj"),
                            on_open,
                            on_action,
                            SearchIndexReadiness::Ready,
                            window,
                            cx,
                        )
                    })
                },
            )
            .expect("open test window");
            (
                window.downcast::<WindowRoot>().expect("WindowRoot"),
                overlay,
            )
        })
    }

    fn with_overlay(
        handle: WindowHandle<WindowRoot>,
        overlay: &Entity<SearchEverywhere>,
        cx: &mut TestAppContext,
        run: impl FnOnce(&mut SearchEverywhere, &mut Window, &mut Context<SearchEverywhere>),
    ) {
        cx.update_window(handle.into(), |_, window, cx| {
            overlay.update(cx, |overlay, cx| run(overlay, window, cx))
        })
        .expect("update overlay");
    }

    fn assert_no_core_query(
        overlay: &Entity<SearchEverywhere>,
        cx: &mut TestAppContext,
        what: &str,
    ) {
        overlay.read_with(cx, |overlay, _| {
            assert!(
                !matches!(overlay.state, EverywhereState::Loading { .. }),
                "{what}：Core 状态机不该进 `Loading`"
            );
            assert!(
                overlay.debounce_task.is_none(),
                "{what}：不该排防抖任务（那是通往 `issue` 的唯一一条路）"
            );
        });
    }

    #[gpui::test]
    fn the_actions_scope_and_command_mode_never_start_a_core_query(cx: &mut TestAppContext) {
        let (handle, overlay) = open_overlay(cx);

        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.set_scope(SearchEverywhereScope::Files, window, cx);
            overlay.on_query("alpha", window, cx);
        });
        overlay.read_with(cx, |overlay, _| {
            assert_eq!(overlay.query_text.as_ref(), "alpha");
            assert!(
                overlay.debounce_task.is_some(),
                "Core 档的非空查询必须排上防抖（否则本测试证明不了任何事）"
            );
        });
        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.abandon(window, cx);
        });

        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.set_scope(SearchEverywhereScope::Actions, window, cx);
            overlay.on_query("alpha", window, cx);
        });
        overlay.read_with(cx, |overlay, _| {
            assert_eq!(
                overlay.query_text.as_ref(),
                "alpha",
                "查询串本身要照常更新（动作列表每帧按它现算）"
            );
            assert!(matches!(overlay.state, EverywhereState::Idle));
        });
        assert_no_core_query(&overlay, cx, "Actions 档敲字");

        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.issue(window, cx);
        });
        assert_no_core_query(&overlay, cx, "Actions 档 + issue");

        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.query_text = SharedString::from("alpha");
            overlay.state = EverywhereState::ready("alpha", &[]);
            overlay.set_scope(SearchEverywhereScope::All, window, cx);
        });
        assert_no_core_query(&overlay, cx, "回到 Core 档（同一查询串已有 ready 结果）");
        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.on_query("/alpha", window, cx);
            overlay.issue(window, cx);
        });
        overlay.read_with(cx, |overlay, _| {
            assert_eq!(overlay.action_query().as_deref(), Some("alpha"));
            assert!(
                matches!(overlay.state, EverywhereState::Ready { .. }),
                "`/` 模式下 `issue` 必须被闸门挡住：否则它会把 Ready 覆盖成 Loading"
            );
        });
        assert_no_core_query(&overlay, cx, "`/` 命令模式");

        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.on_query("alpha", window, cx);
        });
        overlay.read_with(cx, |overlay, _| {
            assert!(
                overlay.debounce_task.is_some(),
                "离开 `/` 模式回到工作区查询时必须重新排队"
            );
        });
        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.abandon(window, cx);
        });
        assert_no_core_query(&overlay, cx, "收尾");
    }
}
