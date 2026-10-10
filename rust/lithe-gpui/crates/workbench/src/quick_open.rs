use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, IntoElement, KeyBinding,
    ParentElement as _, Render, SharedString, Styled as _, Subscription, Task, Window, div, rems,
};
use lithe_gpui_shared::icons::{FileIcon, file_icon};
use lithe_gpui_shared::{CoreClient, CoreError, CoreRequest, tr};
use lithe_ui::forms::{InputEvent, InputState};
use lithe_ui::layout::v_stack;
use lithe_ui::navigation::{Palette, PaletteRow};
use lithe_ui::overlays::Dialog;
use lithe_ui::shell::{WindowRootExt as _, dismiss_hosted};
use lithe_ui::theme::ActiveTheme as _;
use serde_json::Value;

use crate::search_index::SearchIndexReadiness;
use crate::search_model::{
    EVERYWHERE_COMMAND, QueryTimeline, QuickOpenRow, QuickOpenState, SearchOptions, absolute,
    parse_matches, payload,
};

const TAG: &str = "S1_QUICK_OPEN";

const DEBOUNCE_MS: u64 = 180;

const PANEL_WIDTH: f32 = 860.;
const PANEL_MAX_HEIGHT: f32 = 560.;
const PANEL_TOP_INSET: f32 = 84.;

const ROW_ICON_SPEC: f32 = 14.;

pub fn install_actions(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-p", OpenQuickOpen, None)]);
    cx.on_action(|_: &OpenQuickOpen, cx: &mut App| {
        let Some(window_handle) = cx.active_window() else {
            return;
        };
        let _ = window_handle.update(cx, |_, window, cx| {
            let Some(shell) = crate::command_palette::shell().and_then(|shell| shell.upgrade())
            else {
                return;
            };
            let _ = shell.update(cx, |shell, cx| shell.open_quick_open(window, cx));
        });
    });
}

gpui::actions!(lithe_workbench, [OpenQuickOpen]);

pub(crate) fn open_dialog(quick_open: Entity<QuickOpen>, window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        QuickOpen::diagnose("open", "false reason=active_dialog");
        return;
    }
    QuickOpen::diagnose("open", "true");
    window.open_dialog(
        "lithe-quick-open",
        move |id, window, _cx| {
            let rem = window.rem_size();
            let quick_open = quick_open.clone();
            let teardown = quick_open.clone();
            Dialog::new(id)
                .panel()
                // Esc / 遮罩 / 关闭按钮都走这里：先让面板收自己的状态（放弃在途查询、
                // 撤掉防抖任务），再把这一层从宿主栈里摘掉。
                .on_close(move |window, cx| {
                    let _ = teardown.update(cx, |quick, cx| quick.close(window, cx));
                    dismiss_hosted(window, cx);
                })
                .w(crate::rem_px(rem, PANEL_WIDTH))
                .p_0()
                .gap_0()
                .top_inset(crate::rem_px(rem, PANEL_TOP_INSET))
                .child(quick_open)
                .into_any_element()
        },
        cx,
    );
}

pub struct QuickOpen {
    root: PathBuf,
    on_open: Rc<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>,
    query: Entity<InputState>,
    /// 查询框一动就重新起一轮（防抖 + 在途取消都在 `on_query` 里）。
    _query_changed: Subscription,
    query_text: SharedString,
    state: QuickOpenState,
    timeline: QueryTimeline,
    debounce_task: Option<Task<()>>,
    /// 有一轮在途：行首转圈，且不说「没匹配」。
    loading: bool,
    /// 面板是每次打开现建的：第一帧把光标送进查询框。
    focus_pending: bool,
    search_index_readiness: SearchIndexReadiness,
}

impl QuickOpen {
    pub(crate) fn new(
        root: PathBuf,
        on_open: Rc<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>,
        search_index_readiness: SearchIndexReadiness,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx.new(|cx| {
            InputState::new(window, cx).placeholder(tr("lithe.quickOpen.searchFiles"))
        });
        let query_changed = cx.subscribe_in(
            &query,
            window,
            |quick: &mut Self, field: &Entity<InputState>, event: &InputEvent, window, cx| {
                if *event != InputEvent::Changed {
                    return;
                }
                let text = field.read(cx).text().to_string();
                quick.on_query(&text, window, cx);
            },
        );
        Self {
            root,
            on_open,
            query,
            _query_changed: query_changed,
            query_text: SharedString::default(),
            state: QuickOpenState::Idle,
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

    fn on_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.query_text.as_ref() == query {
            return;
        }
        self.query_text = SharedString::from(query);
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        if self.query_text.trim().is_empty() {
            self.debounce_task = None;
            self.state = QuickOpenState::Idle;
            self.set_loading(false, window, cx);
            Self::diagnose("idle", "query=empty");
            cx.notify();
            return;
        }
        self.debounce_task = Some(cx.spawn_in(window, async move |this, async_cx| {
            async_cx
                .background_executor()
                .timer(Duration::from_millis(DEBOUNCE_MS))
                .await;
            let _ = this.update_in(async_cx, |quick, window, cx| quick.issue(window, cx));
        }));
        cx.notify();
    }

    fn issue(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
        let previous = self.state.rows().to_vec();
        self.state = QuickOpenState::loading(query.clone(), &previous);
        self.set_loading(true, window, cx);
        Self::diagnose(
            "issue",
            &format!("generation={generation} operation={operation_id} query={query}"),
        );
        cx.notify();

        let background_query = query.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let request = CoreRequest::command(EVERYWHERE_COMMAND)
                        .with_payload(payload(&root, &background_query, &SearchOptions::default()))
                        .with_operation_id(operation_id);
                    CoreClient::new().execute(&request)
                })
                .await;
            let _ = this.update_in(cx, |quick, window, cx| {
                quick.adopt(generation, &query, result, window, cx)
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
                Ok(matches) => QuickOpenState::ready(query, &matches),
                Err(error) => QuickOpenState::failed(query, error.token()),
            },
            Err(error) => QuickOpenState::failed(query, error.to_string()),
        };
        match &next {
            QuickOpenState::Failed { reason, .. } => Self::diagnose(
                "failed",
                &format!("generation={generation} query={query} reason={reason}"),
            ),
            _ => Self::diagnose(
                "complete",
                &format!(
                    "generation={generation} query={query} files={} state={}",
                    next.rows().len(),
                    if next.rows().is_empty() {
                        "empty"
                    } else {
                        "ready"
                    }
                ),
            ),
        }
        self.state = next;
        self.timeline.settle(generation);
        self.set_loading(false, window, cx);
        cx.notify();
    }

    fn confirm(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(target) = self.state.rows().get(row).cloned() else {
            return;
        };
        // 关面板由 `Palette::on_close` 收（它先叫这里、再收尾）：这一行得在收尾之前读，
        // 收尾会把 `state` 复位成 `Idle`、行就没了。
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        self.debounce_task = None;
        self.set_loading(false, window, cx);
        let path = absolute(&self.root, &target.path);
        Self::diagnose("open_file", &format!("file={}", path.display()));
        (self.on_open)(path, window, cx);
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        self.debounce_task = None;
        self.state = QuickOpenState::Idle;
        self.set_loading(false, window, cx);
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
}

impl Render for QuickOpen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_pending {
            self.focus_pending = false;
            window.focus(&self.query.read(cx).focus().clone(), cx);
        }
        let failure: Option<SharedString> = match &self.state {
            QuickOpenState::Failed { reason, .. } => Some(SharedString::from(reason.clone())),
            _ => None,
        };
        let rows: Vec<QuickOpenRow> = self.state.rows().to_vec();
        let empty_query = self.state.query().trim().is_empty();
        let index_notice = crate::search_index::readiness_notice(self.search_index_readiness, cx);
        let theme = cx.theme();
        let (danger, quiet) = (theme.colors.danger, theme.colors.fg_subtle);

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
                .child(tr("lithe.quickOpen.noMatch"))
                .into_any_element(),
        };

        let close = {
            let quick = cx.entity();
            move |window: &mut Window, cx: &mut App| {
                let _ = quick.update(cx, |quick, cx| quick.close(window, cx));
                dismiss_hosted(window, cx);
            }
        };
        let pick = {
            let quick = cx.entity();
            move |row: usize, _: &SharedString, window: &mut Window, cx: &mut App| {
                let _ = quick.update(cx, |quick, cx| quick.confirm(row, window, cx));
            }
        };

        v_stack()
            .w_full()
            .children(index_notice)
            .child(
                Palette::new(QUICK_OPEN_ID, self.query.clone(), close)
                    .rows(rows.into_iter().map(|row| palette_row(row, cx)))
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

/// `Palette::new` 的 id 与 `open_dialog` 那个层名刻意不同：前者是元素/键控状态的 key，
/// 后者是宿主浮层栈里的层名。
const QUICK_OPEN_ID: &str = "lithe-quick-open-body";

/// A result row: the theme file icon, then the name, then where it lives.
fn palette_row(row: QuickOpenRow, cx: &App) -> PaletteRow {
    let QuickOpenRow { path, name, parent } = row;
    let colors = &cx.theme().colors;
    let item_name = SharedString::from(name.clone());
    let icon = FileIcon::own(file_icon::fallback_icon(item_name.as_ref()))
        .with_theme_icon(item_name.as_ref(), cx)
        .render(rems(ROW_ICON_SPEC / 16.), cx);
    PaletteRow::new(path, name)
        .icon_element(icon)
        .label(
            div()
                .flex_shrink_0()
                .text_xs()
                .text_color(colors.fg)
                .child(item_name),
        )
        .detail(
            div()
                .min_w_0()
                .truncate()
                .text_xs()
                .text_color(colors.fg_subtle)
                .child(SharedString::from(parent)),
        )
}
