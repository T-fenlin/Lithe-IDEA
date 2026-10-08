use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::command::{Command, CommandItem, CommandState};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Icon, WindowExt as _};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, DefiniteLength, Entity, Focusable as _, FontWeight,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_image};

use lithe_gpui_git::{
    BranchInfo, BranchSnapshot, CheckoutGate, CheckoutPreflight, Preflight, PullGate,
    PullPreflight, PullStrategy, checkout, checkout_gate, checkout_preflight, fetch, pull,
    pull_gate, pull_preflight, push,
};
use lithe_gpui_shared::{tr, tr_args};

const PANEL_WIDTH_SPEC: f32 = 704.;

const PANEL_MAX_HEIGHT_SPEC: f32 = 512.;

const PANEL_TOP_INSET_SPEC: f32 = 64.;

const SEARCH_ROW_HEIGHT_SPEC: f32 = 28.;

const TAB_HEIGHT_SPEC: f32 = 32.;

const BRANCH_ROW_HEIGHT_SPEC: f32 = 36.;

const ROW_RADIUS_SPEC: f32 = 8.;

const BRANCH_NAME_MAX_WIDTH_SPEC: f32 = 320.;

const BADGE_MAX_WIDTH_SPEC: f32 = 160.;

const FOOTER_BUTTON_HEIGHT_SPEC: f32 = 32.;

const TRIGGER_RADIUS_SPEC: f32 = lithe_gpui_shared::design::CONTROL_RADIUS;

const TRIGGER_HEIGHT_SPEC: f32 = 32.;

const TRIGGER_HORIZONTAL_INSET_SPEC: f32 = 9.;

const ACTION_ROW_HEIGHT_SPEC: f32 = 30.;

const ACTION_ICON_SLOT_SPEC: f32 = 18.;

const ACTION_ROW_HORIZONTAL_INSET_SPEC: f32 = 8.;

const ACTION_ROW_RADIUS_SPEC: f32 = 6.;

const ACTION_SHORTCUT_FONT_SPEC: f32 = 11.5;

const ACTION_ICON_SIZE_SPEC: f32 = 12.;

const CHECKOUT_CONFIRM_PATH_LIMIT: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RemoteAction {
    Fetch,
    Pull,
    Push,
    Checkout,
}

impl RemoteAction {
    fn id(self) -> &'static str {
        match self {
            Self::Fetch => "fetch",
            Self::Pull => "pull",
            Self::Push => "push",
            Self::Checkout => "checkout",
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            Self::Fetch => "lithe.git.fetch",
            Self::Pull => "lithe.git.log.updateBranch",
            Self::Push => "lithe.git.push",
            Self::Checkout => "lithe.git.checkout",
        }
    }

    fn running_key(self) -> &'static str {
        match self {
            Self::Fetch => "lithe.git.fetchingChanges",
            Self::Pull => "lithe.git.pulling",
            Self::Push => "lithe.git.pushingChanges",
            Self::Checkout => "lithe.git.checkout",
        }
    }

    fn success_key(self) -> &'static str {
        match self {
            Self::Fetch => "lithe.git.changesFetched",
            Self::Pull => "lithe.git.pulledChanges",
            Self::Push => "lithe.git.changesPushed",
            Self::Checkout => "lithe.git.checkout",
        }
    }

    fn failure_key(self) -> &'static str {
        match self {
            Self::Fetch => "lithe.git.fetchFailed",
            Self::Pull => "lithe.git.pullResult.gitRejectedPull",
            Self::Push => "lithe.git.pushFailed",
            Self::Checkout => "lithe.git.operationFailed",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum RemoteRun {
    Idle,
    Running(RemoteAction),
    Done {
        action: RemoteAction,
        failure: Option<SharedString>,
    },
}

impl RemoteRun {
    fn is_busy(&self) -> bool {
        matches!(self, Self::Running(_))
    }

    fn running_action(&self) -> Option<RemoteAction> {
        match self {
            Self::Running(action) => Some(*action),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PullDivergence {
    upstream: SharedString,
    ahead: usize,
    behind: usize,
    has_local_changes: bool,
}

fn remote_actions_disabled(snapshot: &BranchSnapshot, run: &RemoteRun) -> bool {
    snapshot.repository_root.is_none() || run.is_busy()
}

fn update_project_disabled(snapshot: &BranchSnapshot, run: &RemoteRun) -> bool {
    remote_actions_disabled(snapshot, run) || snapshot.branch.is_none()
}

fn checkout_confirm_message(paths: &[SharedString], limit: usize) -> SharedString {
    let (listed, more) = lithe_gpui_git::join_paths(paths, limit);
    let more = if more == 0 {
        String::new()
    } else {
        format!(" (+{more})")
    };
    tr_args(
        "lithe.git.uncommittedWouldOverwrite",
        &[("listed", &listed), ("more", &more)],
    )
}

fn pull_diverged_message(divergence: &PullDivergence) -> SharedString {
    tr_args(
        "lithe.git.pullDivergedMessage",
        &[("upstream", divergence.upstream.as_ref())],
    )
}

fn pull_divergence_counts(divergence: &PullDivergence) -> SharedString {
    SharedString::from(format!("↑{} ↓{}", divergence.ahead, divergence.behind))
}

fn remote_message(run: &RemoteRun) -> Option<(bool, SharedString)> {
    match run {
        RemoteRun::Idle | RemoteRun::Running(_) => None,
        RemoteRun::Done {
            action,
            failure: Some(failure),
        } => Some((
            true,
            SharedString::from(format!("{}：{failure}", tr(action.failure_key()))),
        )),
        RemoteRun::Done {
            action,
            failure: None,
        } => Some((false, tr(action.success_key()))),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanelTab {
    Repositories,
    Branches,
    Worktrees,
}

impl PanelTab {
    const ALL: [Self; 3] = [Self::Branches, Self::Repositories, Self::Worktrees];

    const ACTIVE: Self = Self::Branches;

    fn label_key(self) -> &'static str {
        match self {
            Self::Repositories => "lithe.git.repositories",
            Self::Branches => "lithe.git.branches",
            Self::Worktrees => "lithe.git.worktrees",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Repositories => IconName::FolderOpen,
            Self::Branches => IconName::GitBranch,
            Self::Worktrees => IconName::FolderTree,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Repositories => "repositories",
            Self::Branches => "branches",
            Self::Worktrees => "worktrees",
        }
    }

    fn precondition(self) -> &'static str {
        match self {
            Self::Branches => "read_only_v1",
            Self::Repositories => "no_workspace_repositories_call_site",
            Self::Worktrees => "no_git_worktrees_call_site",
        }
    }
}

fn render_tabs(cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let active_bg = theme.accent;
    let active_fg = theme.foreground;
    let muted = theme.muted_foreground;

    let tabs = PanelTab::ALL.into_iter().map(move |tab| {
        let selected = tab == PanelTab::ACTIVE;
        let (background, foreground) = match selected {
            true => (Some(active_bg), active_fg),
            false => (None, muted),
        };
        let mut pill = h_flex()
            .id(SharedString::from(format!("branch-panel-tab-{}", tab.id())))
            .flex_shrink_0()
            .items_center()
            .min_h(rems(TAB_HEIGHT_SPEC / 16.))
            .px_3()
            .gap_1p5()
            .rounded_full()
            .text_sm()
            .text_color(foreground)
            .child(Icon::new(tab.icon()).size_3p5())
            .child(tr(tab.label_key()))
            .aria_label(tr(tab.label_key()));

        if selected {
            pill = pill.bg(background.unwrap_or(active_bg));
        } else {
            pill = pill.cursor_default().aria_label(SharedString::from(format!(
                "{} — {}",
                tr(tab.label_key()),
                tab.precondition()
            )));
        }

        pill
    });

    h_flex()
        .id("branch-panel-tabs")
        .flex_shrink_0()
        .items_center()
        .gap_1()
        .px_2()
        .pt_2()
        .aria_label(tr("lithe.git.selectorSections"))
        .children(tabs)
}

fn render_search(input: &Entity<InputState>, total: usize, cx: &App) -> impl IntoElement {
    let theme = cx.theme();

    h_flex()
        .flex_shrink_0()
        .items_center()
        .gap_2()
        .px_4()
        .py_3()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .h(rems(SEARCH_ROW_HEIGHT_SPEC / 16.))
                .child(
                    Input::new(input)
                        .appearance(false)
                        .p_0()
                        .h_full()
                        .aria_label(tr("lithe.git.searchBranchesAria")),
                ),
        )
        .child(
            h_flex()
                .flex_shrink_0()
                .items_center()
                .min_h(rems(SEARCH_ROW_HEIGHT_SPEC / 16.))
                .max_w(rems(BADGE_MAX_WIDTH_SPEC / 16.))
                .px_2()
                .rounded_full()
                .bg(theme.muted.opacity(0.7))
                .text_sm()
                .text_color(theme.muted_foreground)
                .truncate()
                .child(tr_args(
                    match total {
                        1 => "lithe.git.branchCount",
                        _ => "lithe.git.branchesCount",
                    },
                    &[("count", &total.to_string())],
                )),
        )
}

fn render_actions(panel: &Entity<BranchPanel>, cx: &App) -> impl IntoElement {
    let (snapshot, run) = {
        let panel = panel.read(cx);
        (panel.branches.clone(), panel.remote.clone())
    };
    let disabled = remote_actions_disabled(&snapshot, &run);
    let pull_disabled = update_project_disabled(&snapshot, &run);

    let mut section = v_flex().w_full().flex_shrink_0().gap_0p5().px_2().py_1();

    for action in [RemoteAction::Fetch, RemoteAction::Pull, RemoteAction::Push] {
        let running = run.running_action() == Some(action);
        let label = if running {
            tr(action.running_key())
        } else {
            tr(action.label_key())
        };
        let row_disabled = if action == RemoteAction::Pull {
            pull_disabled
        } else {
            disabled
        };
        let icon = match action {
            RemoteAction::Fetch => IconName::ArrowDownToLine,
            RemoteAction::Pull => IconName::ArrowDownLeft,
            RemoteAction::Push => IconName::ArrowUpRight,
            RemoteAction::Checkout => IconName::GitBranch,
        };
        let shortcut = match action {
            RemoteAction::Pull => Some("⌘T"),
            RemoteAction::Push => Some("⇧⌘K"),
            _ => None,
        };
        let handle = panel.downgrade();
        let mut row = div()
            .id(SharedString::from(format!("branch-action-{}", action.id())))
            .flex()
            .w_full()
            .items_center()
            .gap_2p5()
            .px(rems(ACTION_ROW_HORIZONTAL_INSET_SPEC / 16.))
            .h(rems(ACTION_ROW_HEIGHT_SPEC / 16.))
            .rounded(rems(ACTION_ROW_RADIUS_SPEC / 16.))
            .text_sm()
            .text_color(if row_disabled {
                cx.theme().muted_foreground
            } else {
                cx.theme().foreground
            });
        if !row_disabled {
            row = row.hover(|style| style.bg(cx.theme().accent));
        }
        row = row.child(
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .w(rems(ACTION_ICON_SLOT_SPEC / 16.))
                .child(
                    Icon::new(icon)
                        .text_color(cx.theme().muted_foreground)
                        .w(rems(ACTION_ICON_SIZE_SPEC / 16.))
                        .h(rems(ACTION_ICON_SIZE_SPEC / 16.)),
                ),
        );
        row = row.child(div().min_w_0().truncate().child(label));
        if let Some(shortcut) = shortcut {
            row = row.child(
                div()
                    .ml_auto()
                    .flex_shrink_0()
                    .text_color(cx.theme().muted_foreground)
                    .text_size(rems(ACTION_SHORTCUT_FONT_SPEC / 16.))
                    .child(SharedString::from(shortcut)),
            );
        }
        row = row.aria_label(tr(action.label_key()));
        if !row_disabled {
            row = row.on_click(move |_event, window, cx| {
                let _ = handle.update(cx, |panel, cx| match action {
                    RemoteAction::Fetch => panel.run_fetch(cx),
                    RemoteAction::Push => panel.run_push(cx),
                    RemoteAction::Pull => panel.update_project(window, cx),
                    RemoteAction::Checkout => {}
                });
            });
        }
        section = section.child(row);
    }

    section
}

fn render_remote_message(message: &(bool, SharedString), cx: &App) -> impl IntoElement {
    let (is_error, text) = message;
    let color = if *is_error {
        cx.theme().danger
    } else {
        cx.theme().success
    };
    h_flex()
        .w_full()
        .flex_shrink_0()
        .items_start()
        .gap_2()
        .px_2()
        .py_1p5()
        .text_sm()
        .text_color(color)
        .bg(color.opacity(0.12))
        .child(idea_icon_image(
            if *is_error {
                &idea::WARNING_CIRCLE_ICON
            } else {
                &idea::CHECK_ICON
            },
            cx,
            rems(14. / 16.),
        ))
        .child(div().flex_1().min_w_0().child(text.clone()))
}

fn render_footer(panel: &Entity<BranchPanel>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let handle = panel.downgrade();
    let idle_fg = theme.muted_foreground;
    let hover_bg = theme.accent;
    let hover_fg = theme.foreground;

    h_flex()
        .flex_shrink_0()
        .items_center()
        .justify_end()
        .gap_2()
        .px_3()
        .py_3()
        .border_t_1()
        .border_color(theme.border)
        .child(
            div()
                .id("branch-panel-refresh")
                .flex()
                .flex_shrink_0()
                .items_center()
                .h(rems(FOOTER_BUTTON_HEIGHT_SPEC / 16.))
                .px_3()
                .gap_1p5()
                .rounded(rems(ROW_RADIUS_SPEC / 16.))
                .bg(theme.accent)
                .text_sm()
                .text_color(theme.foreground)
                .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
                .on_click(move |_event, window, cx| {
                    let _ = handle.update(cx, |panel, cx| panel.refresh(window, cx));
                })
                .child(Icon::new(IconName::RefreshCw).size_4().text_color(idle_fg))
                .child(tr("lithe.git.refresh")),
        )
}

trait IntoBranchItem {
    fn into_branch_item(self) -> CommandItem;
}

impl IntoBranchItem for BranchInfo {
    fn into_branch_item(self) -> CommandItem {
        let name = self.name.clone();
        let search = self.name.clone();
        let is_current = self.is_current;

        CommandItem::new().label(search).child(move |_window, cx| {
            let theme = cx.theme();
            let name_color = if is_current {
                theme.foreground
            } else {
                theme.muted_foreground
            };
            let leading = h_flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .size_5()
                .child(match is_current {
                    true => idea_icon_image(&idea::CHECK_ICON, cx, rems(14. / 16.)),
                    false => idea_icon_image(&idea::GIT_BRANCH_ICON, cx, rems(14. / 16.)),
                });

            let mut row = h_flex()
                .w_full()
                .min_w_0()
                .min_h(rems(BRANCH_ROW_HEIGHT_SPEC / 16.))
                .gap_2p5()
                .child(leading)
                .child(
                    div()
                        .min_w_0()
                        .max_w(rems(BRANCH_NAME_MAX_WIDTH_SPEC / 16.))
                        .truncate()
                        .text_sm()
                        .text_color(name_color)
                        .child(name.clone()),
                );

            if is_current {
                row = row.child(
                    div()
                        .ml_auto()
                        .flex_shrink_0()
                        .px_1p5()
                        .rounded_full()
                        .bg(theme.success.opacity(0.1))
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.success)
                        .child(tr("lithe.git.current")),
                );
            }

            row.into_any_element()
        })
    }
}

fn diagnose(opened: bool, branches: usize, current: Option<&str>) {
    eprintln!(
        "S1_BRANCH_PANEL opened={opened} branches={branches} current={}",
        current.unwrap_or("—")
    );
}

fn diagnose_structure() {
    eprintln!(
        "S1_BRANCH_PANEL panel_w={} max_h={} top={} tabs={} tab={} search_h={} row_h={} \
         badge_max_w={} name_max_w={} refresh=true new_branch=absent rows_menu=absent",
        PANEL_WIDTH_SPEC as i32,
        PANEL_MAX_HEIGHT_SPEC as i32,
        PANEL_TOP_INSET_SPEC as i32,
        PanelTab::ALL.len(),
        PanelTab::ACTIVE.id(),
        SEARCH_ROW_HEIGHT_SPEC as i32,
        BRANCH_ROW_HEIGHT_SPEC as i32,
        BADGE_MAX_WIDTH_SPEC as i32,
        BRANCH_NAME_MAX_WIDTH_SPEC as i32,
    );
}

fn diagnose_tabs() {
    for tab in PanelTab::ALL {
        eprintln!(
            "S1_BRANCH_PANEL tab={} state={} precondition={}",
            tab.id(),
            match tab {
                PanelTab::Branches => "active",
                _ => "disabled",
            },
            tab.precondition()
        );
    }
}

fn diagnose_failures(failures: &[String]) {
    if failures.is_empty() {
        return;
    }
    eprintln!(
        "S1_BRANCH_PANEL failures={} detail={}",
        failures.len(),
        failures.join("+")
    );
}

pub struct BranchPanel {
    root: std::path::PathBuf,
    open: bool,
    command: Entity<CommandState>,
    search: Entity<InputState>,
    query: SharedString,
    branches: BranchSnapshot,
    _search_subscription: Option<Subscription>,
    remote: RemoteRun,
    pull: Preflight<PullPreflight>,
    checkout: Preflight<CheckoutPreflight>,
    remote_message: Option<(bool, SharedString)>,
    pending_pull: Option<PullDivergence>,
}

impl BranchPanel {
    pub fn new(root: std::path::PathBuf, window: &mut Window, cx: &mut App) -> Entity<Self> {
        let branches = BranchSnapshot::load(&root);
        diagnose(false, branches.branches.len(), branches.branch.as_deref());
        diagnose_failures(&branches.failures);

        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr("lithe.git.searchBranches")));
        cx.new(|cx| {
            let command = cx.new(|cx| CommandState::new(window, cx));
            let subscription = cx.subscribe_in(
                &search,
                window,
                |panel: &mut Self, state: &Entity<InputState>, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        panel.query = state.read(cx).value();
                        cx.notify();
                    }
                },
            );
            Self {
                root,
                open: false,
                command,
                search,
                query: SharedString::default(),
                branches,
                _search_subscription: Some(subscription),
                remote: RemoteRun::Idle,
                pull: Preflight::new(),
                checkout: Preflight::new(),
                remote_message: None,
                pending_pull: None,
            }
        })
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn branches(&self) -> &BranchSnapshot {
        &self.branches
    }

    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.branches = BranchSnapshot::load(&self.root);
        self.open = true;
        diagnose(
            true,
            self.branches.branches.len(),
            self.branches.branch.as_deref(),
        );
        diagnose_structure();
        diagnose_tabs();
        diagnose_failures(&self.branches.failures);
        self.query = SharedString::default();
        self.search
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    fn close(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        cx.notify();
    }

    fn refresh(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.branches = BranchSnapshot::load(&self.root);
        eprintln!(
            "S1_BRANCH_PANEL refresh=true branches={} current={}",
            self.branches.branches.len(),
            self.branches.branch.as_deref().unwrap_or("—")
        );
        diagnose_failures(&self.branches.failures);
        cx.notify();
    }

    fn run_fetch(&mut self, cx: &mut Context<Self>) {
        let Some(repository_root) = self.repository_root_text() else {
            return;
        };
        if self.remote.is_busy() {
            return;
        }
        self.begin_remote(RemoteAction::Fetch, cx);
        self.spawn_remote(
            repository_root,
            RemoteAction::Fetch,
            move |root| fetch(&root),
            cx,
        );
    }

    fn run_push(&mut self, cx: &mut Context<Self>) {
        let Some(repository_root) = self.repository_root_text() else {
            return;
        };
        if self.remote.is_busy() {
            return;
        }
        let Some(branch) = self.branches.branch.clone() else {
            return;
        };
        self.begin_remote(RemoteAction::Push, cx);
        let reference = branch.to_string();
        self.spawn_remote(
            repository_root,
            RemoteAction::Push,
            move |root| push(&root, Some(&reference)),
            cx,
        );
    }

    fn update_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(repository_root) = self.repository_root_text() else {
            return;
        };
        if self.remote.is_busy() {
            return;
        }
        if self.branches.branch.is_none() {
            return;
        }
        self.begin_remote(RemoteAction::Pull, cx);
        let serial = self.pull.begin();
        let workspace_root = self.root.clone();

        cx.spawn_in(window, async move |this, cx| {
            let (fetch_failure, probe, snapshot) = cx
                .background_spawn(async move {
                    let fetch_failure = fetch(&repository_root).failure;
                    let probe = match fetch_failure {
                        Some(_) => None,
                        None => Some(pull_preflight(&repository_root)),
                    };
                    (fetch_failure, probe, BranchSnapshot::load(&workspace_root))
                })
                .await;

            let _ = this.update_in(cx, |panel, window, cx| {
                panel.branches = snapshot;
                if let Some(failure) = fetch_failure {
                    eprintln!(
                        "S1_BRANCH_PANEL remote=pull step=fetch result=failed error={failure}"
                    );
                    panel.remote = RemoteRun::Done {
                        action: RemoteAction::Pull,
                        failure: Some(SharedString::from(failure.clone())),
                    };
                    panel.remote_message = Some((
                        true,
                        tr_args("lithe.git.pullResult.fetchFailed", &[("error", &failure)]),
                    ));
                    cx.notify();
                    return;
                }
                let probe = probe.unwrap_or_else(|| Err("pull preflight was not read".to_string()));
                if !panel.pull.apply(serial, probe) {
                    eprintln!("S1_BRANCH_PANEL remote=pull step=preflight result=stale_reply");
                    return;
                }
                panel.finish_pull_gate(window, cx);
            });
        })
        .detach();
    }

    fn finish_pull_gate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let action = RemoteAction::Pull;
        match pull_gate(self.pull.state()) {
            PullGate::Proceed => {
                let Some(repository_root) = self.repository_root_text() else {
                    return;
                };
                eprintln!("S1_BRANCH_PANEL remote=pull step=preflight state=proceed");
                self.spawn_remote(
                    repository_root,
                    action,
                    move |root| pull(&root, PullStrategy::FfOnly),
                    cx,
                );
            }
            PullGate::Stop(stop) => {
                eprintln!("S1_BRANCH_PANEL remote=pull step=preflight state=stop reason={stop:?}");
                self.remote = RemoteRun::Idle;
                self.remote_message = Some((false, tr(stop.message_key())));
                cx.notify();
            }
            PullGate::AskStrategy {
                upstream,
                ahead,
                behind,
                has_local_changes,
            } => {
                eprintln!(
                    "S1_BRANCH_PANEL remote=pull step=preflight state=ask_strategy ahead={ahead} behind={behind}"
                );
                self.remote = RemoteRun::Idle;
                self.pending_pull = Some(PullDivergence {
                    upstream,
                    ahead,
                    behind,
                    has_local_changes,
                });
                self.open_pull_strategy_dialog(window, cx);
            }
            PullGate::Unavailable { reason } => {
                eprintln!(
                    "S1_BRANCH_PANEL remote=pull step=preflight state=unavailable error={reason}"
                );
                self.remote = RemoteRun::Done {
                    action,
                    failure: Some(reason.clone()),
                };
                self.remote_message = Some((
                    true,
                    tr_args(
                        "lithe.git.pullResult.preflightFailed",
                        &[("error", reason.as_ref())],
                    ),
                ));
                cx.notify();
            }
            PullGate::Wait => {}
        }
    }

    fn open_pull_strategy_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(divergence) = self.pending_pull.clone() else {
            return;
        };
        let handle = cx.entity().downgrade();
        let body = pull_diverged_message(&divergence);
        let counts = pull_divergence_counts(&divergence);
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let cancel_handle = handle.clone();
            let merge_handle = handle.clone();
            let rebase_handle = handle.clone();
            dialog
                .title(tr("lithe.git.choosePullStrategy"))
                .child(
                    v_flex()
                        .gap_2()
                        .text_sm()
                        .child(body.clone())
                        .child(counts.clone())
                        .child(
                            div()
                                .text_color(_cx.theme().muted_foreground)
                                .child(tr("lithe.git.pullStrategyHint")),
                        ),
                )
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        .child(
                            Button::new("branch-pull-cancel")
                                .label(tr("lithe.ui.cancel"))
                                .on_click(move |_, window, cx| {
                                    if let Some(panel) = cancel_handle.upgrade() {
                                        let _ = panel.update(cx, |panel, cx| {
                                            panel.pending_pull = None;
                                            panel.remote_message = Some((
                                                false,
                                                tr("lithe.git.pullResult.strategyRequired"),
                                            ));
                                            cx.notify();
                                        });
                                    }
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("branch-pull-merge")
                                .label(tr("lithe.git.merge"))
                                .on_click(move |_, window, cx| {
                                    if let Some(panel) = merge_handle.upgrade() {
                                        let _ = panel.update(cx, |panel, cx| {
                                            panel.pending_pull = None;
                                            panel.resolve_pull(PullStrategy::Merge, cx);
                                        });
                                    }
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("branch-pull-rebase")
                                .label(tr("lithe.git.rebase"))
                                .on_click(move |_, window, cx| {
                                    if let Some(panel) = rebase_handle.upgrade() {
                                        let _ = panel.update(cx, |panel, cx| {
                                            panel.pending_pull = None;
                                            panel.resolve_pull(PullStrategy::Rebase, cx);
                                        });
                                    }
                                    window.close_dialog(cx);
                                }),
                        ),
                )
        });
    }

    fn resolve_pull(&mut self, strategy: PullStrategy, cx: &mut Context<Self>) {
        let Some(repository_root) = self.repository_root_text() else {
            return;
        };
        self.begin_remote(RemoteAction::Pull, cx);
        self.spawn_remote(
            repository_root,
            RemoteAction::Pull,
            move |root| pull(&root, strategy),
            cx,
        );
    }

    fn request_checkout(
        &mut self,
        name: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(repository_root) = self.repository_root_text() else {
            eprintln!("S1_BRANCH_PANEL action=checkout state=no_repository");
            return;
        };
        if self.remote.is_busy() {
            return;
        }
        if self.branches.branch.as_ref() == Some(&name) {
            eprintln!("S1_BRANCH_PANEL action=checkout state=already_current branch={name}");
            self.remote_message = Some((
                false,
                SharedString::from(format!("{name} · {}", tr("lithe.git.current"))),
            ));
            cx.notify();
            return;
        }

        let serial = self.checkout.begin();
        let workspace_root = self.root.clone();
        let branch = name.clone();
        let probe_branch = branch.clone();
        cx.spawn_in(window, async move |this, cx| {
            let (probe, snapshot) = cx
                .background_spawn(async move {
                    let probe = checkout_preflight(&repository_root, probe_branch.as_ref());
                    (probe, BranchSnapshot::load(&workspace_root))
                })
                .await;
            let _ = this.update_in(cx, |panel, window, cx| {
                panel.branches = snapshot;
                if !panel.checkout.apply(serial, probe) {
                    eprintln!("S1_BRANCH_PANEL action=checkout step=preflight result=stale_reply");
                    return;
                }
                match checkout_gate(panel.checkout.state()) {
                    CheckoutGate::Proceed => {
                        eprintln!("S1_BRANCH_PANEL action=checkout step=preflight state=proceed");
                        panel.perform_checkout(branch.clone(), false, false, cx);
                    }
                    CheckoutGate::Confirm { blocking_paths } => {
                        eprintln!(
                            "S1_BRANCH_PANEL action=checkout step=preflight state=confirm paths={}",
                            blocking_paths.len()
                        );
                        panel.ask_checkout(branch.clone(), blocking_paths, window, cx);
                    }
                    CheckoutGate::Unavailable { reason } => {
                        eprintln!(
                            "S1_BRANCH_PANEL action=checkout step=preflight state=unavailable error={reason}"
                        );
                        panel.remote_message = Some((
                            true,
                            SharedString::from(format!(
                                "{}：{reason}",
                                tr_args(
                                    "lithe.git.actionFailed",
                                    &[("action", tr("lithe.git.checkout").as_ref())]
                                )
                            )),
                        ));
                        cx.notify();
                    }
                    CheckoutGate::Wait => {}
                }
            });
        })
        .detach();
    }

    fn ask_checkout(
        &mut self,
        name: SharedString,
        blocking_paths: Vec<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let handle = cx.entity().downgrade();
        let body = checkout_confirm_message(&blocking_paths, CHECKOUT_CONFIRM_PATH_LIMIT);
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let cancel_handle = handle.clone();
            let stash_handle = handle.clone();
            let force_handle = handle.clone();
            let stash_branch = name.clone();
            let force_branch = name.clone();
            dialog
                .title(tr("lithe.git.checkout"))
                .child(
                    v_flex().gap_2().text_sm().child(body.clone()).child(
                        div()
                            .text_color(_cx.theme().muted_foreground)
                            .child(tr("lithe.git.pullResult.dirty")),
                    ),
                )
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        .child(
                            Button::new("branch-checkout-cancel")
                                .label(tr("lithe.ui.cancel"))
                                .on_click(move |_, window, cx| {
                                    if let Some(panel) = cancel_handle.upgrade() {
                                        let _ = panel.update(cx, |_panel, cx| {
                                            eprintln!(
                                                "S1_BRANCH_PANEL action=checkout state=cancelled"
                                            );
                                            cx.notify();
                                        });
                                    }
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("branch-checkout-stash")
                                .label(tr("lithe.git.stashChanges"))
                                .on_click(move |_, window, cx| {
                                    if let Some(panel) = stash_handle.upgrade() {
                                        let branch = stash_branch.clone();
                                        let _ = panel.update(cx, |panel, cx| {
                                            panel.perform_checkout(branch, false, true, cx);
                                        });
                                    }
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("branch-checkout-force")
                                .label(tr("lithe.git.checkout"))
                                .on_click(move |_, window, cx| {
                                    if let Some(panel) = force_handle.upgrade() {
                                        let branch = force_branch.clone();
                                        let _ = panel.update(cx, |panel, cx| {
                                            panel.perform_checkout(branch, true, false, cx);
                                        });
                                    }
                                    window.close_dialog(cx);
                                }),
                        ),
                )
        });
    }

    fn perform_checkout(
        &mut self,
        name: SharedString,
        force: bool,
        auto_stash: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(repository_root) = self.repository_root_text() else {
            return;
        };
        let workspace_root = self.root.clone();
        self.remote = RemoteRun::Running(RemoteAction::Checkout);
        self.remote_message = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let branch_text = name.to_string();
            let (outcome, snapshot) = cx
                .background_spawn(async move {
                    let outcome = checkout(&repository_root, &branch_text, force, auto_stash);
                    (outcome, BranchSnapshot::load(&workspace_root))
                })
                .await;
            let _ = this.update(cx, |panel, cx| {
                panel.branches = snapshot;
                panel.remote = RemoteRun::Idle;
                match outcome.failure {
                    Some(failure) => {
                        eprintln!("S1_BRANCH_PANEL action=checkout result=failed error={failure}");
                        panel.remote_message = Some((true, SharedString::from(failure)));
                    }
                    None => {
                        eprintln!("S1_BRANCH_PANEL action=checkout result=ok branch={name}");
                        panel.remote_message = Some((
                            false,
                            tr_args("lithe.git.switchingTo", &[("branch", name.as_ref())]),
                        ));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn begin_remote(&mut self, action: RemoteAction, cx: &mut Context<Self>) {
        self.remote = RemoteRun::Running(action);
        self.remote_message = None;
        eprintln!("S1_BRANCH_PANEL remote={} state=running", action.id());
        cx.notify();
    }

    fn spawn_remote(
        &mut self,
        repository_root: String,
        action: RemoteAction,
        run: impl FnOnce(String) -> lithe_gpui_git::WriteOutcome + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        let workspace_root = self.root.clone();
        cx.spawn(async move |this, cx| {
            let (outcome, snapshot) = cx
                .background_spawn(async move {
                    let outcome = run(repository_root);
                    (outcome, BranchSnapshot::load(&workspace_root))
                })
                .await;
            let _ = this.update(cx, |panel, cx| {
                panel.branches = snapshot;
                panel.remote = RemoteRun::Done {
                    action,
                    failure: outcome.failure.clone().map(SharedString::from),
                };
                if let Some(failure) = &outcome.failure {
                    eprintln!(
                        "S1_BRANCH_PANEL remote={} result=failed error={failure}",
                        action.id()
                    );
                } else {
                    eprintln!("S1_BRANCH_PANEL remote={} result=ok", action.id());
                }
                panel.remote_message = remote_message(&panel.remote);
                cx.notify();
            });
        })
        .detach();
    }

    fn repository_root_text(&self) -> Option<String> {
        self.branches
            .repository_root
            .as_ref()
            .map(|root| root.to_string())
    }

    fn focus_search(&self, window: &mut Window, cx: &mut App) {
        let focus = self.search.read(cx).focus_handle(cx);
        focus.focus(window, cx);
    }

    pub fn open_by_probe(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        eprintln!("S1_BRANCH_PANEL_PROBE open=true");
        self.open(window, cx);
    }
}

impl Render for BranchPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn normalize(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut last_was_space = true;
    for character in text.chars() {
        if character.is_alphanumeric() {
            normalized.extend(character.to_lowercase());
            last_was_space = false;
        } else if !last_was_space {
            normalized.push(' ');
            last_was_space = true;
        }
    }
    while normalized.ends_with(' ') {
        normalized.pop();
    }
    normalized
}

fn matches_query(name: &str, query: &str) -> bool {
    let query = normalize(query);
    if query.is_empty() {
        return true;
    }
    let name = normalize(name);
    let compact_query: String = query.chars().filter(|c| *c != ' ').collect();
    if name.contains(&query) {
        return true;
    }
    let compact_name: String = name.chars().filter(|c| *c != ' ').collect();
    compact_name.contains(&compact_query)
}

fn visible_rows(branches: &[BranchInfo], query: &str) -> Vec<BranchInfo> {
    branches
        .iter()
        .filter(|branch| matches_query(branch.name.as_ref(), query))
        .cloned()
        .collect()
}

fn render_empty(query: &str, cx: &App) -> impl IntoElement {
    let key = match query.trim().is_empty() {
        true => "lithe.git.noBranchesFound",
        false => "lithe.git.noMatchingBranches",
    };
    div()
        .p_3()
        .w_full()
        .text_center()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(tr(key))
}

fn render_error_bar(failures: &[String], cx: &App) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .mx_2()
        .mb_2()
        .px_2()
        .py_1p5()
        .rounded(rems(ROW_RADIUS_SPEC / 16.))
        .bg(cx.theme().danger.opacity(0.1))
        .text_xs()
        .text_color(cx.theme().danger)
        .child(SharedString::from(failures.join("；")))
}

fn render_panel(panel: &Entity<BranchPanel>, cx: &App) -> AnyElement {
    let (command, query, search, snapshot) = {
        let panel = panel.read(cx);
        (
            panel.command.clone(),
            panel.query.clone(),
            panel.search.clone(),
            panel.branches.clone(),
        )
    };

    let rows = visible_rows(&snapshot.branches, query.as_ref());
    let total = snapshot.branches.len();
    let failures = snapshot.failures.clone();
    let empty_query = query.clone();
    let confirm_handle = panel.downgrade();
    let visible_names: Vec<SharedString> = rows.iter().map(|row| row.name.clone()).collect();

    let command_element = Command::new(&command)
        .bordered(false)
        .searchable(false)
        .filterable(false)
        .w_full()
        .max_h(DefiniteLength::from(rems(PANEL_MAX_HEIGHT_SPEC / 16.)))
        .items(rows.into_iter().map(IntoBranchItem::into_branch_item))
        .empty(move |_, _, cx| render_empty(empty_query.as_ref(), cx).into_any_element())
        .on_confirm(move |index, window, cx| {
            eprintln!(
                "S1_BRANCH_PANEL action=checkout row={} state=preflight",
                index.row
            );
            let Some(branch) = visible_names.get(index.row).cloned() else {
                eprintln!(
                    "S1_BRANCH_PANEL action=checkout row={} state=missing_row",
                    index.row
                );
                return;
            };
            let _ =
                confirm_handle.update(cx, |panel, cx| panel.request_checkout(branch, window, cx));
        })
        .footer({
            let panel = panel.clone();
            move |_, _, cx| render_footer(&panel, cx).into_any_element()
        });

    let mut container = v_flex().w_full();
    if !failures.is_empty() {
        container = container.child(render_error_bar(&failures, cx));
    }
    let remote_message = panel.read(cx).remote_message.clone();
    container = container
        .child(render_tabs(cx))
        .child(render_search(&search, total, cx))
        .child(render_actions(panel, cx));
    if let Some(message) = &remote_message {
        container = container.child(render_remote_message(message, cx).into_any_element());
    }
    container.child(command_element).into_any_element()
}

pub(crate) fn open_branch_panel(panel: &Entity<BranchPanel>, window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        return;
    }
    let _ = panel.update(cx, |panel, cx| panel.open(window, cx));
    let _ = panel.update(cx, |panel, cx| panel.focus_search(window, cx));

    let handle = panel.clone();
    window.open_dialog(cx, move |dialog, window, _cx| {
        let rem = window.rem_size();
        dialog
            .close_button(false)
            .overlay(true)
            .overlay_closable(true)
            .keyboard(true)
            .width(crate::rem_px(rem, PANEL_WIDTH_SPEC))
            .margin_top(crate::rem_px(rem, PANEL_TOP_INSET_SPEC))
            .p_0()
            .on_close({
                let handle = handle.clone();
                move |_event, window, cx| {
                    let _ = handle.update(cx, |panel, cx| panel.close(window, cx));
                }
            })
            .content({
                let handle = handle.clone();
                move |content, _window, cx| content.p_0().gap_0().child(render_panel(&handle, cx))
            })
    });
}

pub(crate) fn trigger(panel: &Entity<BranchPanel>, cx: &App) -> Option<AnyElement> {
    let branch = panel.read(cx).branches().branch.clone()?;
    let tracking = panel.read(cx).branches().tracking;
    let theme = cx.theme();
    let idle = theme.muted_foreground;
    let hover_bg = theme.accent;
    let hover_fg = theme.foreground;
    let open_handle = panel.clone();
    let name = branch.clone();

    Some(
        div()
            .id("lithe-title-branch-trigger")
            .flex()
            .flex_shrink_0()
            .items_center()
            .h(rems(TRIGGER_HEIGHT_SPEC / 16.))
            .px(rems(TRIGGER_HORIZONTAL_INSET_SPEC / 16.))
            .gap_1()
            .max_w(rems(BRANCH_NAME_MAX_WIDTH_SPEC / 16.))
            .rounded(rems(TRIGGER_RADIUS_SPEC / 16.))
            .text_sm()
            .text_color(idle)
            .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
            .on_mouse_down(MouseButton::Left, |_event, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .on_click(move |_event, window, cx| {
                if open_handle.read(cx).is_open() {
                    return;
                }
                open_branch_panel(&open_handle, window, cx);
            })
            .child(Icon::new(IconName::GitBranch).size_3p5())
            .child(div().min_w_0().truncate().child(branch))
            .children(render_tracking(tracking, cx))
            .aria_label(SharedString::from(format!(
                "{} {}",
                tr("lithe.git.searchBranchesAria"),
                name
            )))
            .into_any_element(),
    )
}

fn render_tracking(tracking: lithe_gpui_git::TrackingCounts, cx: &App) -> Vec<AnyElement> {
    if tracking.is_empty() {
        return Vec::new();
    }
    let mut arrows = Vec::new();
    if tracking.behind > 0 {
        arrows.push(
            div()
                .flex_shrink_0()
                .text_size(rems(10. / 16.))
                .text_color(cx.theme().info)
                .child(SharedString::from("↙"))
                .into_any_element(),
        );
    }
    if tracking.ahead > 0 {
        arrows.push(
            div()
                .flex_shrink_0()
                .text_size(rems(10. / 16.))
                .text_color(cx.theme().success)
                .child(SharedString::from("↗"))
                .into_any_element(),
        );
    }
    arrows
}

#[cfg(test)]
use gpui_kit::Pixels;

#[cfg(test)]
fn branch_row_height(rem: Pixels) -> Pixels {
    crate::rem_px(rem, BRANCH_ROW_HEIGHT_SPEC)
}

#[cfg(test)]
fn trigger_icon_names() -> [IconName; 1] {
    [IconName::GitBranch]
}

#[cfg(test)]
mod tests {
    use super::{
        BRANCH_ROW_HEIGHT_SPEC, IconName, PANEL_MAX_HEIGHT_SPEC, PANEL_TOP_INSET_SPEC,
        PANEL_WIDTH_SPEC, PanelTab, PullDivergence, RemoteAction, RemoteRun,
        SEARCH_ROW_HEIGHT_SPEC, branch_row_height, checkout_confirm_message, matches_query,
        pull_diverged_message, pull_divergence_counts, remote_actions_disabled, remote_message,
        trigger_icon_names, update_project_disabled, visible_rows,
    };
    use gpui_kit::{SharedString, px};
    use lithe_gpui_git::{BranchInfo, BranchSnapshot};
    use lithe_gpui_shared::tr;

    fn branch(name: &str, is_current: bool) -> BranchInfo {
        BranchInfo {
            name: SharedString::from(name),
            is_current,
            upstream: None,
        }
    }

    #[test]
    fn panel_geometry_matches_the_source() {
        assert_eq!(PANEL_WIDTH_SPEC, 704.);
        assert_eq!(PANEL_MAX_HEIGHT_SPEC, 512.);
        assert_eq!(PANEL_TOP_INSET_SPEC, 64.);
        assert_eq!(SEARCH_ROW_HEIGHT_SPEC, 28.);
        assert_eq!(BRANCH_ROW_HEIGHT_SPEC, 36.);
    }

    #[test]
    fn branch_row_height_scales_with_the_rem_base() {
        assert_eq!(f32::from(branch_row_height(px(16.))), 36.);
        assert_eq!(f32::from(branch_row_height(px(32.))), 72.);
    }

    #[test]
    fn tabs_are_ordered_and_branches_is_active() {
        assert_eq!(PanelTab::ALL.len(), 3);
        assert_eq!(PanelTab::ACTIVE, PanelTab::Branches);
        let ids: Vec<&str> = PanelTab::ALL.iter().map(|tab| tab.id()).collect();
        assert_eq!(ids, vec!["branches", "repositories", "worktrees"]);
        let mut preconditions: Vec<&str> =
            PanelTab::ALL.iter().map(|tab| tab.precondition()).collect();
        preconditions.sort_unstable();
        preconditions.dedup();
        assert_eq!(preconditions.len(), 3);
    }

    #[test]
    fn trigger_has_no_chevron() {
        let icons = trigger_icon_names();
        assert_eq!(icons, [IconName::GitBranch]);
        assert!(!icons.contains(&IconName::ChevronDown));
        assert!(!icons.contains(&IconName::ChevronUp));
    }

    #[test]
    fn search_matches_like_the_source() {
        assert!(matches_query("feature/login", ""));
        assert!(matches_query("feature/login", "LOGIN"));
        assert!(matches_query("feature/login", "feature"));
        assert!(matches_query("fix/order-list", "fix order"));
        assert!(matches_query("fix/order-list", "order list"));
        assert!(matches_query("feature/login", "feature log"));
        assert!(!matches_query("main", "dev"));
        assert!(!matches_query("feature/login", "zzz"));
        assert!(!matches_query("feature/recent", "featrecent"));
    }

    #[test]
    fn filtering_keeps_order_and_only_reads_the_name() {
        let branches = vec![
            branch("main", true),
            branch("feature/login", false),
            branch("fix/order-list", false),
        ];
        let all = visible_rows(&branches, "");
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].name.as_ref(), "main");

        let filtered = visible_rows(&branches, "fix");
        let names: Vec<&str> = filtered.iter().map(|b| b.name.as_ref()).collect();
        assert_eq!(names, vec!["fix/order-list"]);

        let none = visible_rows(&branches, "zzz");
        assert!(none.is_empty());
    }

    fn snapshot_with_repo() -> BranchSnapshot {
        BranchSnapshot {
            repository_root: Some(SharedString::from("/tmp/lithe-repo")),
            branch: Some(SharedString::from("main")),
            ..BranchSnapshot::default()
        }
    }

    #[test]
    fn remote_actions_are_disabled_without_a_repository() {
        let empty = BranchSnapshot::default();
        assert!(remote_actions_disabled(&empty, &RemoteRun::Idle));
        assert!(update_project_disabled(&empty, &RemoteRun::Idle));
    }

    #[test]
    fn remote_actions_are_disabled_while_a_branch_operation_runs() {
        let snapshot = snapshot_with_repo();
        assert!(!remote_actions_disabled(&snapshot, &RemoteRun::Idle));
        assert!(!update_project_disabled(&snapshot, &RemoteRun::Idle));

        let running = RemoteRun::Running(RemoteAction::Fetch);
        assert!(remote_actions_disabled(&snapshot, &running));
        assert!(update_project_disabled(&snapshot, &running));
    }

    #[test]
    fn update_project_needs_a_current_branch() {
        let mut snapshot = snapshot_with_repo();
        snapshot.branch = None;
        assert!(!remote_actions_disabled(&snapshot, &RemoteRun::Idle));
        assert!(update_project_disabled(&snapshot, &RemoteRun::Idle));
    }

    #[test]
    fn a_finished_run_is_not_busy() {
        let done = RemoteRun::Done {
            action: RemoteAction::Push,
            failure: Some(SharedString::from("boom")),
        };
        assert!(!done.is_busy());
        assert_eq!(done.running_action(), None);
        assert_eq!(
            RemoteRun::Running(RemoteAction::Pull).running_action(),
            Some(RemoteAction::Pull)
        );
    }

    #[test]
    fn remote_messages_cover_success_failure_and_in_flight() {
        assert_eq!(remote_message(&RemoteRun::Idle), None);
        assert_eq!(
            remote_message(&RemoteRun::Running(RemoteAction::Fetch)),
            None
        );

        let success = remote_message(&RemoteRun::Done {
            action: RemoteAction::Fetch,
            failure: None,
        })
        .expect("成功后必须有一句话");
        assert!(!success.0, "成功不该标成错误");
        assert_eq!(success.1, tr("lithe.git.changesFetched"));

        let failure = remote_message(&RemoteRun::Done {
            action: RemoteAction::Push,
            failure: Some(SharedString::from("fatal: no upstream")),
        })
        .expect("失败后必须有一句话");
        assert!(failure.0, "失败必须标成错误");
        assert!(failure.1.contains("fatal: no upstream"), "{}", failure.1);
        assert!(!failure.1.starts_with("lithe."), "{}", failure.1);
        assert!(
            failure
                .1
                .starts_with(tr(RemoteAction::Push.failure_key()).as_ref()),
            "结果里应当以动作名开头：{}",
            failure.1
        );
    }

    #[test]
    fn the_checkout_confirm_message_lists_paths_and_counts_the_rest() {
        let paths = vec![
            SharedString::from("src/a.rs"),
            SharedString::from("src/b.rs"),
            SharedString::from("src/c.rs"),
        ];
        let message = checkout_confirm_message(&paths, 2);
        assert!(message.contains("src/a.rs, src/b.rs"), "{message}");
        assert!(message.contains("(+1)"), "{message}");
        assert!(!message.contains("{listed}"), "{message}");
        assert!(!message.contains("{more}"), "{message}");

        let short = checkout_confirm_message(&paths, 9);
        assert!(!short.contains("(+0)"), "{short}");
        assert!(short.contains("src/c.rs"), "{short}");
    }

    #[test]
    fn the_pull_strategy_message_names_the_upstream() {
        let divergence = PullDivergence {
            upstream: SharedString::from("origin/main"),
            ahead: 2,
            behind: 3,
            has_local_changes: true,
        };
        let message = pull_diverged_message(&divergence);
        assert!(message.contains("origin/main"), "{message}");
        assert!(!message.contains("{upstream}"), "{message}");
        assert_eq!(pull_divergence_counts(&divergence).as_ref(), "↑2 ↓3");
    }
}
