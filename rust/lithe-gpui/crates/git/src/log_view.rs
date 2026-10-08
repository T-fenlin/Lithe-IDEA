use std::collections::BTreeSet;
use std::path::PathBuf;

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, Sizable as _};
use gpui::prelude::FluentBuilder as _;
use gpui::{AnyElement, App, AppContext as _, Context, Entity, FontWeight, Hsla, InteractiveElement as _, IntoElement, Length, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, WeakEntity, Window, div, rems};
use gpui_kit::{Div, relative};

use lithe_gpui_shared::icons::{idea, idea_icon_image};
use lithe_gpui_shared::{tr, tr_args};

enum LogIcon {
    Idea(&'static idea::IdeaIcon),
    Lucide(IconName),
}

impl LogIcon {
    fn element(self, cx: &App, side: impl Into<Length>, enabled: bool) -> AnyElement {
        match self {
            Self::Lucide(name) => {
                let color = if enabled {
                    cx.theme().foreground
                } else {
                    cx.theme().muted_foreground
                };
                let side = side.into();
                Icon::new(name)
                    .w(side)
                    .h(side)
                    .text_color(color)
                    .into_any_element()
            }
            Self::Idea(icon) => idea_icon_image(icon, cx, side),
        }
    }
}

use crate::model::{
    AUTHOR_COLUMN_WIDTH, ButtonHandler, COMMIT_CONTENT_MIN_WIDTH, COMMIT_FILE_INDENT_BASE,
    COMMIT_FILE_INDENT_STEP, COMMIT_FILE_STATUS_FONT_SIZE, COMMIT_PANE_FRACTION, COMMIT_ROW_HEIGHT,
    Commit, CommitFileRow, DATE_FONT_SIZE, Detail, FILTER_INPUT_MAX_WIDTH, FILTER_INPUT_MIN_WIDTH,
    FilesState, FilterScope, FirstLoad, GRAPH_LANE_GAP, GRAPH_LINE_WIDTH, GRAPH_MIN_WIDTH,
    GRAPH_NODE_RADIUS, GraphRow, HistoryCursor, ICON_BUTTON_RADIUS, INSPECTOR_DETAILS_FRACTION,
    INSPECTOR_FILES_FRACTION, INSPECTOR_FILES_MIN_HEIGHT, INSPECTOR_HASH_FONT_SIZE,
    INSPECTOR_MONO_FONT_SIZE, INSPECTOR_PANE_FRACTION, INSPECTOR_PANE_MIN_WIDTH, LABEL_FONT_SIZE,
    LABEL_MAX_WIDTH, LABEL_PADDING_Y, LABEL_RADIUS, LOAD_MORE_BUTTON_HEIGHT, Label, LoadState,
    MoreLoad, Panel, REFERENCE_BADGE_FONT_SIZE, REFERENCE_HEAD_ROW_RADIUS, REFERENCE_INDENT_BASE,
    REFERENCE_INDENT_STEP, REFERENCE_PANE_FRACTION, REFERENCE_PANE_MIN_WIDTH,
    REFERENCE_PILL_MAX_WIDTH, REFERENCE_PILL_RADIUS, REFERENCE_ROW_RADIUS,
    REFERENCE_SECTION_RADIUS, REFERENCE_TOOLBAR_BUTTON_RADIUS, RefKind, Reference,
    TRACKING_COUNT_FONT_SIZE, build_commit_files, build_reference_rows, handler, label_color,
    lane_color, layout_graph, load_commit_files, load_first, load_more, matches_filter,
    tracking_count,
};
pub(crate) use lithe_gpui_shared::rem_px;

fn panel_element_id(panel: Panel) -> &'static str {
    match panel {
        Panel::Log => "bottom-git-tab:log",
        Panel::Console => "bottom-git-tab:console",
    }
}

fn commit_element_id(commit: &Commit) -> String {
    format!("bottom-git-commit:{}", commit.hash)
}

pub struct BottomPane {
    root: PathBuf,
    repository_root: Option<String>,
    branch: Option<SharedString>,
    load_state: LoadState,
    panel: Panel,
    references: Vec<Reference>,
    commits: Vec<Commit>,
    cursor: Option<HistoryCursor>,
    has_more: bool,
    loading_more: bool,
    selected_reference: Option<usize>,
    selected_commit: Option<usize>,
    collapsed_sections: BTreeSet<&'static str>,
    collapsed_groups: BTreeSet<String>,
    show_my_branches_only: bool,
    show_decorations: bool,
    files_state: FilesState,
    files: Vec<CommitFileRow>,
    detail: Option<Detail>,
    filter: Entity<InputState>,
    filter_query: SharedString,
    filter_scope: FilterScope,
    _filter_subscription: Subscription,
    visible: bool,
    request_serial: u64,
}

impl BottomPane {
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

    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.spawn_first_load(cx);
    }

    fn spawn_first_load(&mut self, cx: &mut Context<Self>) {
        let root = self.root.clone();
        let reference = self.selected_reference_full_name();
        let cursor = self.cursor.take();
        self.has_more = false;
        self.loading_more = false;
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
            self.load_state = LoadState::Stale;
            self.has_more = false;
            self.cursor = None;
        }

        self.selected_commit = None;
        self.selected_reference = self
            .selected_reference
            .filter(|index| *index < self.references.len());
        cx.notify();

        if !self.commits.is_empty() {
            self.select_commit(0, cx);
        }
    }

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

    fn visible_commits(&self) -> Vec<usize> {
        self.commits
            .iter()
            .enumerate()
            .filter(|(_, commit)| matches_filter(commit, &self.filter_query, self.filter_scope))
            .map(|(index, _)| index)
            .collect()
    }

    fn visible_reference(&self, index: usize) -> bool {
        if !self.show_my_branches_only {
            return true;
        }
        self.references
            .get(index)
            .is_some_and(|reference| reference.kind == RefKind::Local && reference.is_current)
    }

    fn icon_button(
        id: (&'static str, usize),
        icon: LogIcon,
        label: SharedString,
        enabled: bool,
        handler: ButtonHandler,
        cx: &App,
    ) -> impl IntoElement {
        let tooltip = label.clone();

        div()
            .id(id)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size_6()
            .rounded(rems(ICON_BUTTON_RADIUS / 16.))
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .aria_label(label)
            .when(enabled, |this| {
                this.hover(|style| style.bg(cx.theme().accent))
                    .on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when(!enabled, |this| this.opacity(0.4))
            .child(icon.element(cx, rems(14. / 16.), enabled))
    }

    fn toolbar_button(
        id: (&'static str, usize),
        icon: LogIcon,
        label: SharedString,
        enabled: bool,
        handler: ButtonHandler,
        cx: &App,
    ) -> impl IntoElement {
        let tooltip = label.clone();
        div()
            .id(id)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size_8()
            .rounded(rems(REFERENCE_TOOLBAR_BUTTON_RADIUS / 16.))
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .aria_label(label)
            .when(enabled, |this| {
                this.hover(|style| style.bg(cx.theme().accent))
                    .on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when(!enabled, |this| this.opacity(0.3))
            .child(icon.element(cx, rems(1.), enabled))
    }

    fn title_bar(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let reference_name = self
            .selected_reference
            .and_then(|index| self.references.get(index))
            .map(|reference| reference.short_name.clone())
            .or_else(|| self.branch.clone())
            .unwrap_or_else(|| tr("lithe.git.log.all"));
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
            .bg(cx.theme().tab_bar)
            .text_sm()
            .child(idea_icon_image(&idea::GIT_BRANCH_ICON, cx, rems(14. / 16.)))
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .child(tr("lithe.workbench.gitLog")),
            )
            .child(
                div()
                    .id("bottom-git-reference-pill")
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .h_6()
                    .max_w(rems(REFERENCE_PILL_MAX_WIDTH / 16.))
                    .px_2()
                    .rounded(rems(REFERENCE_PILL_RADIUS / 16.))
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .font_weight(FontWeight::MEDIUM)
                    .hover(|style| style.bg(cx.theme().accent))
                    .aria_label(tr("lithe.git.log.showAll"))
                    .on_click(move |event, window, cx| show_all(event, window, cx))
                    .child(div().min_w_0().text_ellipsis().child(pill_label)),
            )
            .child(Self::icon_button(
                ("bottom-git-refresh", 0),
                LogIcon::Idea(&idea::REFRESH_ICON),
                tr("lithe.git.log.refresh"),
                self.load_state != LoadState::Loading,
                refresh,
                cx,
            ))
            .child(Self::icon_button(
                ("bottom-git-settings", 0),
                LogIcon::Idea(&idea::GEAR_ICON),
                tr("lithe.git.log.settings"),
                false,
                settings,
                cx,
            ))
            .child(div().flex_1())
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.footer.readOnly")),
            )
            .child(Self::icon_button(
                ("bottom-git-hide", 0),
                LogIcon::Idea(&idea::MINUS_ICON),
                tr("lithe.git.log.hide"),
                true,
                hide,
                cx,
            ))
    }

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

        for (panel, label) in [
            (Panel::Log, tr("lithe.git.console.log")),
            (Panel::Console, tr("lithe.git.console.title")),
        ] {
            let selected = self.panel == panel;
            let this = this.clone();
            row = row.child(
                div()
                    .id(panel_element_id(panel))
                    .flex_shrink_0()
                    .aria_selected(selected)
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

    fn scope_button(&self, this: &WeakEntity<Self>) -> impl IntoElement {
        let current = self.filter_scope;
        let this = this.clone();

        Button::new("bottom-git-filter-field")
            .ghost()
            .small()
            .compact()
            .label(current.label())
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

    fn filter_row(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let visible = self.visible_commits().len();
        let total = self.commits.len();

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
                    .min_w(rems(FILTER_INPUT_MIN_WIDTH / 16.))
                    .max_w(rems(FILTER_INPUT_MAX_WIDTH / 16.))
                    .prefix(idea_icon_image(
                        &idea::MAGNIFYING_GLASS_ICON,
                        cx,
                        rems(14. / 16.),
                    )),
            )
            .child(self.scope_button(this))
            .child(Self::icon_button(
                ("bottom-git-decorations", 0),
                if self.show_decorations {
                    LogIcon::Idea(&idea::EYE_ICON)
                } else {
                    LogIcon::Idea(&idea::EYE_SLASH_ICON)
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
            .child(div().min_w_0().flex_1().child(tr("lithe.git.log.commit")))
            .child(
                div()
                    .w(rems(AUTHOR_COLUMN_WIDTH / 16.))
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

    fn graph_cell(row: &GraphRow, cx: &App) -> Div {
        let half = COMMIT_ROW_HEIGHT / 2.;

        h_flex()
            .h_full()
            .flex_shrink_0()
            .min_w(rems(GRAPH_MIN_WIDTH / 16.))
            .pl_2()
            .pr_2()
            .children(row.lanes.iter().enumerate().map(move |(lane, color)| {
                let lane_width = rems(GRAPH_LANE_GAP / 16.);
                let line_width = rems(GRAPH_LINE_WIDTH / 16.);
                let edge_here = row.edges.iter().find(|(target, _, _)| *target == lane);

                match (lane == row.lane, color) {
                    (true, _) => v_flex()
                        .w(lane_width)
                        .h_full()
                        .items_center()
                        .child(
                            div()
                                .w(line_width)
                                .h(rems((half - GRAPH_NODE_RADIUS) / 16.))
                                .bg(lane_color(row.node_color, cx)),
                        )
                        .child(
                            div()
                                .size(rems(GRAPH_NODE_RADIUS * 2. / 16.))
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
                    (false, Some(index)) => v_flex()
                        .w(lane_width)
                        .h_full()
                        .items_center()
                        .child(div().w(line_width).h_full().bg(lane_color(*index, cx)))
                        .into_any_element(),
                    (false, None) => v_flex()
                        .w(lane_width)
                        .h_full()
                        .items_center()
                        .child(div().flex_1())
                        .when_some(edge_here, |this, (_, index, missing)| {
                            this.child(
                                div()
                                    .w(line_width)
                                    .h(rems(half / 16.))
                                    .opacity(if *missing { 0.7 } else { 1.0 })
                                    .bg(lane_color(*index, cx)),
                            )
                        })
                        .into_any_element(),
                }
            }))
    }

    fn label_badge(label: &Label, cx: &App) -> Div {
        let color = label_color(label.kind, cx);
        h_flex()
            .h(rems((LABEL_FONT_SIZE + LABEL_PADDING_Y * 2.) / 16.))
            .max_w(rems(LABEL_MAX_WIDTH / 16.))
            .flex_shrink_0()
            .items_center()
            .px_1p5()
            .rounded(rems(LABEL_RADIUS / 16.))
            .border_1()
            .border_color(color.opacity(0.45))
            .bg(color.opacity(0.2))
            .text_size(rems(LABEL_FONT_SIZE / 16.))
            .text_color(color)
            .whitespace_nowrap()
            .child(div().min_w_0().text_ellipsis().child(label.title.clone()))
    }

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
            .id(commit_element_id(commit))
            .w_full()
            .min_w(rems(COMMIT_CONTENT_MIN_WIDTH / 16.))
            .min_h(rems(COMMIT_ROW_HEIGHT / 16.))
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
                            .min_h(rems(COMMIT_ROW_HEIGHT / 16.))
                            .text_sm()
                            .line_height(rems(COMMIT_ROW_HEIGHT / 16.))
                            .child(commit.subject.clone()),
                    ),
            )
            .child(
                div()
                    .w(rems(AUTHOR_COLUMN_WIDTH / 16.))
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
                    .text_size(rems(DATE_FONT_SIZE / 16.))
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(cx.theme().muted_foreground)
                    .child(commit.date.clone()),
            )
    }

    fn commit_pane(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let visible = self.visible_commits();
        let total = self.commits.len();
        let graphs = layout_graph(&self.commits);

        let mut list = v_flex()
            .w_full()
            .min_w(rems(COMMIT_CONTENT_MIN_WIDTH / 16.));

        if visible.is_empty() {
            let message = if total == 0 {
                tr("lithe.git.log.noCommits")
            } else {
                tr("lithe.git.log.noMatch")
            };
            list = list.child(
                h_flex()
                    .w_full()
                    .min_h(rems(COMMIT_ROW_HEIGHT * 4. / 16.))
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

        if self.has_more {
            let this = this.clone();
            list = list.child(
                h_flex()
                    .w_full()
                    .min_w(rems(COMMIT_CONTENT_MIN_WIDTH / 16.))
                    .h_9()
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("bottom-git-load-more")
                            .ghost()
                            .with_size(rem_px(cx.theme().font_size, LOAD_MORE_BUTTON_HEIGHT))
                            .label(if self.loading_more {
                                tr("lithe.git.log.loadingCommits")
                            } else {
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
                div()
                    .id("bottom-git-commit-scroll")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scroll()
                    .child(list),
            )
    }

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
                LogIcon::Lucide(IconName::UnfoldVertical),
                tr("lithe.git.expandAll"),
                has_references,
                expand,
                cx,
            ))
            .child(Self::toolbar_button(
                ("bottom-git-ref-collapse", 0),
                LogIcon::Lucide(IconName::FoldVertical),
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
                LogIcon::Idea(&idea::FUNNEL_ICON),
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
            .child(Self::toolbar_button(
                ("bottom-git-ref-create", 0),
                LogIcon::Idea(&idea::PLUS_ICON),
                tr("lithe.git.log.toolbar.newBranch"),
                false,
                create_branch,
                cx,
            ))
    }

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
            .pl(rems(
                (REFERENCE_INDENT_BASE + depth as f32 * REFERENCE_INDENT_STEP) / 16.,
            ))
            .rounded(rems(REFERENCE_ROW_RADIUS / 16.))
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
            None if is_group => row.child(idea_icon_image(&idea::FOLDER_ICON, cx, rems(14. / 16.))),
            None => row,
        };

        let mut row = row
            .child(div().min_w_0().text_ellipsis().child(name))
            .child(div().flex_1());

        if let Some((ahead, behind)) = tracking {
            let mut counts = h_flex()
                .flex_shrink_0()
                .items_center()
                .gap_1()
                .text_size(rems(TRACKING_COUNT_FONT_SIZE / 16.));
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
            row = row.child(
                h_flex()
                    .flex_shrink_0()
                    .px_1()
                    .rounded(rems(REFERENCE_SECTION_RADIUS / 16.))
                    .bg(cx.theme().yellow_light.opacity(0.12))
                    .text_size(rems(REFERENCE_BADGE_FONT_SIZE / 16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().yellow_light)
                    .child(tr("lithe.git.current")),
            );
        }

        row
    }

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
                .rounded(rems(REFERENCE_HEAD_ROW_RADIUS / 16.))
                .font_weight(FontWeight::MEDIUM)
                .when(head_selected, |row| row.bg(cx.theme().accent))
                .hover(|style| style.bg(cx.theme().accent.opacity(0.8)))
                .on_click(move |_event, _window, cx: &mut App| {
                    let _ = this.update(cx, |pane, cx| pane.select_reference(current_index, cx));
                })
                .child(div().text_color(cx.theme().primary).child("→"))
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
                    .rounded(rems(REFERENCE_SECTION_RADIUS / 16.))
                    .font_weight(FontWeight::MEDIUM)
                    .hover(|style| style.bg(cx.theme().accent.opacity(0.8)))
                    .on_click(move |_event, _window, cx: &mut App| {
                        let _ = this.update(cx, |pane, cx| pane.toggle_section(kind, cx));
                    })
                    .child(idea_icon_image(
                        if collapsed {
                            &idea::CARET_RIGHT_ICON
                        } else {
                            &idea::CARET_DOWN_ICON
                        },
                        cx,
                        rems(0.75),
                    ))
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
                    section = section.child(
                        div()
                            .h_6()
                            .pl_8()
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
                                    .child(idea_icon_image(
                                        if collapsed_group {
                                            &idea::CARET_RIGHT_ICON
                                        } else {
                                            &idea::CARET_DOWN_ICON
                                        },
                                        cx,
                                        rems(0.75),
                                    ))
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
                    .min_h(rems(INSPECTOR_FILES_MIN_HEIGHT / 16.))
                    .min_w_0()
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
                            .child(tr("lithe.git.log.commitFiles"))
                            .child(div().flex_1())
                            .child(if self.files_state == FilesState::Loading {
                                tr("lithe.git.log.loadingShort")
                            } else {
                                tr_args(
                                    "lithe.git.log.filesCount",
                                    &[("count", &file_count.to_string())],
                                )
                            })
                            .child(Self::icon_button(
                                ("bottom-git-open-diff", 0),
                                LogIcon::Idea(&idea::GIT_DIFF_ICON),
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
            FilesState::Idle => centered(
                tr("lithe.git.log.selectCommit"),
                cx.theme().muted_foreground,
            ),
            FilesState::Loading => centered(
                tr("lithe.git.log.loadingChangedFiles"),
                cx.theme().muted_foreground,
            ),
            FilesState::Failed => {
                centered(tr("lithe.git.log.unableToLoadFiles"), cx.theme().danger)
            }
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
            .pl(rems(
                (COMMIT_FILE_INDENT_BASE + row.depth as f32 * COMMIT_FILE_INDENT_STEP) / 16.,
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
                        .text_size(rems(COMMIT_FILE_STATUS_FONT_SIZE / 16.))
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
                        .text_size(rems(COMMIT_FILE_STATUS_FONT_SIZE / 16.))
                        .text_color(status_color)
                        .child(SharedString::from(status.to_string())),
                )
        }
    }

    fn commit_detail_body(&self, cx: &App) -> impl IntoElement {
        let mono = cx.theme().mono_font_family.clone();

        let Some(detail) = self.detail.as_ref() else {
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
                    .text_size(rems(INSPECTOR_MONO_FONT_SIZE / 16.))
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
                    .text_size(rems(INSPECTOR_MONO_FONT_SIZE / 16.))
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
            .child(
                div()
                    .font_family(mono)
                    .text_size(rems(INSPECTOR_HASH_FONT_SIZE / 16.))
                    .text_color(cx.theme().muted_foreground)
                    .child(detail.hash.clone()),
            )
            .into_any_element()
    }

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

        for (index, (icon, label)) in [
            (
                LogIcon::Idea(&idea::MAGNIFYING_GLASS_ICON),
                tr("lithe.git.console.find"),
            ),
            (
                LogIcon::Lucide(IconName::TextWrap),
                tr("lithe.git.console.wrap"),
            ),
            (
                LogIcon::Idea(&idea::ARROW_DOWN_ICON),
                tr("lithe.git.console.scrollToEnd"),
            ),
            (
                LogIcon::Lucide(IconName::CircleSlash),
                tr("lithe.git.console.cancel"),
            ),
            (
                LogIcon::Idea(&idea::TRASH_ICON),
                tr("lithe.git.console.clear"),
            ),
            (
                LogIcon::Idea(&idea::COPY_ICON),
                tr("lithe.git.console.copy"),
            ),
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
                    .child(tr("lithe.git.console.empty")),
            )
    }

    fn log_body(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        h_flex()
            .w_full()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .w(relative(REFERENCE_PANE_FRACTION))
                    .min_w(rems(REFERENCE_PANE_MIN_WIDTH / 16.))
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
                    .min_w(rems(INSPECTOR_PANE_MIN_WIDTH / 16.))
                    .h_full()
                    .min_h_0()
                    .child(self.inspector_pane(cx)),
            )
    }

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
            .child(
                Button::new("bottom-git-retry")
                    .ghost()
                    .with_size(rem_px(cx.theme().font_size, LOAD_MORE_BUTTON_HEIGHT))
                    .label(tr("lithe.git.log.retry"))
                    .on_click(move |_event, _window, cx: &mut App| {
                        let _ = this.update(cx, |pane, cx| pane.refresh(cx));
                    }),
            )
    }

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
                    .with_size(rem_px(cx.theme().font_size, LOAD_MORE_BUTTON_HEIGHT))
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
            LoadState::Stale => {
                root = root.child(Self::banner(tr("lithe.git.log.unableToRefresh"), &this, cx));
                root.child(self.log_body(&this, cx))
            }
            LoadState::NoRepository => root.child(Self::centered_notice(
                tr("lithe.git.noRepositoryOpen"),
                Some(tr("lithe.git.log.openWorkspace")),
                None,
                cx,
            )),
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

    #[test]
    fn tab_element_ids_do_not_depend_on_the_tab_row_order() {
        let forward = [Panel::Log, Panel::Console];
        let reversed = [Panel::Console, Panel::Log];

        let forward_ids: Vec<&str> = forward.into_iter().map(panel_element_id).collect();
        let reversed_ids: Vec<&str> = reversed.into_iter().map(panel_element_id).collect();

        assert_eq!(
            forward_ids[0], reversed_ids[1],
            "Panel::Log 的 id 必须与位置无关"
        );
        assert_eq!(
            forward_ids[1], reversed_ids[0],
            "Panel::Console 的 id 必须与位置无关"
        );
        assert_ne!(forward_ids[0], forward_ids[1], "两个页签的 id 必须互不相同");
    }

    #[test]
    fn commit_element_ids_do_not_depend_on_the_commit_row_order() {
        let commits = [
            commit("1111111111111111111111111111111111111111"),
            commit("2222222222222222222222222222222222222222"),
        ];

        let forward: Vec<String> = commits.iter().map(commit_element_id).collect();
        let reversed: Vec<String> = commits.iter().rev().map(commit_element_id).collect();

        assert_eq!(
            forward[0], reversed[1],
            "同一提交在不同顺序下必须算出同一个 id"
        );
        assert_eq!(
            forward[1], reversed[0],
            "同一提交在不同顺序下必须算出同一个 id"
        );
        assert_ne!(forward[0], forward[1], "不同提交的 id 必须互不相同");
        assert_eq!(
            forward[0],
            "bottom-git-commit:1111111111111111111111111111111111111111"
        );
        let mut rewritten = commit("1111111111111111111111111111111111111111");
        rewritten.subject = SharedString::from("另一个主题行");
        assert_eq!(commit_element_id(&rewritten), forward[0]);
    }
}
