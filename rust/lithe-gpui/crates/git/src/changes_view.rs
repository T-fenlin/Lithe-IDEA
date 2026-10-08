use std::collections::BTreeSet;
use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia};
use gpui_kit::component::input::{InputEvent, Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _};
use gpui::{AnyElement, App, AppContext as _, ClickEvent, Context, Entity, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, WeakEntity, Window, div, rems};
use gpui_kit::{Role, relative};

use lithe_gpui_shared::design::{ROW_LINE_HEIGHT, ROW_RADIUS};
use lithe_gpui_shared::icons::{idea, idea_icon_image};
use lithe_gpui_shared::{tr, tr_args};
use lithe_ui::overlays::Dialog;
use lithe_ui::shell::{WindowRootExt as _, dismiss_hosted};

use crate::CommitChanges;
use crate::changes::{
    ChangeRow, ChangesSnapshot, OperationState, commit as write_commit, discard, load,
    operation_action, stage, stage_all, unstage,
};
use crate::log_view::rem_px;
use crate::model::{ICON_BUTTON_RADIUS, RADIUS_MD, RADIUS_XL};
use crate::remote::{self, ConflictGate, Preflight};

const COMMIT_PANEL_RADIUS: f32 = RADIUS_XL;
const COMMIT_ERROR_RADIUS: f32 = RADIUS_MD;
const COMMIT_INPUT_HEIGHT: f32 = 64.;
const CONFLICT_PATH_LIMIT: usize = 3;

const TITLE_KEY: &str = "lithe.workbench.sourceControl";
const REFRESH_KEY: &str = "lithe.git.refreshStatus";
const ACTIONS_KEY: &str = "lithe.git.actions";
const LOAD_FAILED_KEY: &str = "lithe.git.statusLoadFailed";
const RETRY_KEY: &str = "lithe.git.log.retry";
const LOADING_KEY: &str = "lithe.git.loadingGitStatus";
const TRACKED_KEY: &str = "lithe.git.tracked";
const UNTRACKED_KEY: &str = "lithe.git.untracked";
const CLEAN_KEY: &str = "lithe.git.workingTreeClean";
const NOT_REPOSITORY_KEY: &str = "lithe.git.setup.notRepository";
const NOT_REPOSITORY_HINT_KEY: &str = "lithe.git.setup.initializeHint";
const INITIALIZE_KEY: &str = "lithe.git.setup.initialize";
const NO_COMMITS_KEY: &str = "lithe.git.setup.noCommits";
const NO_COMMITS_HINT_KEY: &str = "lithe.git.setup.firstCommit";
const STAGE_ALL_KEY: &str = "lithe.git.stageAllChanges";
const UNSTAGE_ALL_KEY: &str = "lithe.git.unstageAllChanges";
const TRACKED_FILES_KEY: &str = "lithe.git.trackedFiles";
const UNTRACKED_FILES_KEY: &str = "lithe.git.untrackedFiles";
const COMMIT_PLACEHOLDER_KEY: &str = "lithe.git.commitMessagePlaceholder";
const COMMIT_KEY: &str = "lithe.git.commit";
const COMMITTING_KEY: &str = "lithe.git.committing";
const FILES_STAGED_KEY: &str = "lithe.git.filesStaged";
const NO_FILES_STAGED_KEY: &str = "lithe.git.noFilesStaged";
const SELECT_FILES_KEY: &str = "lithe.git.selectFilesToCommit";
const ENTER_MESSAGE_KEY: &str = "lithe.git.enterCommitMessage";
const COMMITTED_KEY: &str = "lithe.git.changesCommitted";
const FINISH_OPERATION_KEY: &str = "lithe.git.finishOperationBeforeCommit";
const RESOLVE_CONFLICTS_KEY: &str = "lithe.git.resolveConflictsFirst";
const OPERATION_STEP_KEY: &str = "lithe.git.operationStep";
const RESOLVE_CONFLICTS_HINT_KEY: &str = "lithe.git.resolveConflicts";
const CONFLICTS_RESOLVED_KEY: &str = "lithe.git.conflictsResolved";
const SKIP_KEY: &str = "lithe.git.skipCommit";
const ABORT_KEY: &str = "lithe.git.abort";
const DISCARD_ALL_KEY: &str = "lithe.git.discardAllChanges";
const DISCARD_CONFIRM_KEY: &str = "lithe.git.discardChangesConfirm";
const ROLLBACK_KEY: &str = "lithe.git.rollback";
const CANCEL_KEY: &str = "lithe.ui.cancel";
const CONFLICT_MARKERS_KEY: &str = "lithe.git.conflictMarkersRemain";
const VERIFY_MARKERS_KEY: &str = "lithe.git.verifyConflictMarkersFailed";
const OPEN_FILE_KEY: &str = "lithe.git.openFile";

enum LoadState {
    Loading,
    Ready,
    Failed(String),
}

enum CommitBlocker {
    Conflicts(SharedString),
    Operation,
    NoStagedFiles,
    EmptyMessage,
}

pub struct ChangesView {
    root: PathBuf,
    snapshot: ChangesSnapshot,
    state: LoadState,
    error: Option<SharedString>,
    notice: Option<SharedString>,
    pending: BTreeSet<SharedString>,
    busy: bool,
    message: Entity<TextareaState>,
    _message_subscription: Subscription,
    request_serial: u64,
    confirm_before_discard: bool,
    conflict: Preflight<Vec<SharedString>>,
    on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static>,
}

impl ChangesView {
    pub fn new(
        root: PathBuf,
        on_open: Box<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let message =
            cx.new(|cx| TextareaState::new(window, cx).placeholder(tr(COMMIT_PLACEHOLDER_KEY)));
        let message_subscription = cx.subscribe_in(
            &message,
            window,
            |_view: &mut Self, _state: &Entity<TextareaState>, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            },
        );

        Self {
            root,
            snapshot: ChangesSnapshot::default(),
            state: LoadState::Loading,
            error: None,
            notice: None,
            pending: BTreeSet::new(),
            busy: false,
            message,
            _message_subscription: message_subscription,
            request_serial: 0,
            confirm_before_discard: true,
            conflict: Preflight::new(),
            on_open: Rc::from(on_open),
        }
    }

    pub fn set_confirm_before_discard(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.confirm_before_discard == enabled {
            return;
        }
        self.confirm_before_discard = enabled;
        eprintln!("S1_SOURCE_CONTROL confirm_before_discard={enabled}");
        cx.notify();
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.request_serial = self.request_serial.wrapping_add(1);
        let serial = self.request_serial;
        if self.snapshot.repository_root.is_none() {
            self.state = LoadState::Loading;
        }
        let conflict_serial = self.conflict.begin();
        let root = self.root.clone();
        cx.notify();

        cx.spawn(async move |this, cx| {
            let (snapshot, markers) = cx
                .background_spawn(async move {
                    let snapshot = load(&root);
                    let markers = match snapshot.repository_root.as_ref() {
                        Some(repository_root) => remote::conflict_markers(repository_root.as_ref()),
                        None => Ok(Vec::new()),
                    };
                    (snapshot, markers)
                })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.apply(serial, snapshot, cx);
                view.conflict.apply(conflict_serial, markers);
                cx.notify();
            });
        })
        .detach();
    }

    pub fn branch(&self) -> Option<&SharedString> {
        self.snapshot.branch.as_ref()
    }

    pub fn activate(&mut self, cx: &mut Context<Self>) {
        self.refresh(cx);
    }

    fn apply(&mut self, serial: u64, snapshot: ChangesSnapshot, cx: &mut Context<Self>) {
        if serial != self.request_serial {
            return;
        }

        eprintln!("{}", snapshot.diagnose());
        if !snapshot.failures.is_empty() {
            eprintln!("S1_SOURCE_CONTROL failures={}", snapshot.failures.join(" "));
        }

        self.state = if snapshot.repository_root.is_none() && !snapshot.failures.is_empty() {
            LoadState::Failed(snapshot.failures.join(" "))
        } else {
            LoadState::Ready
        };
        self.snapshot = snapshot;
        cx.notify();
    }

    fn staged_count(&self) -> usize {
        self.snapshot.staged_count()
    }

    fn message_text(&self, cx: &App) -> SharedString {
        self.message.read(cx).value()
    }

    fn commit_blocker(&self, cx: &App) -> Option<CommitBlocker> {
        if let Some(operation) = &self.snapshot.operation {
            if operation.has_conflicts() {
                let paths = operation
                    .conflicted_paths
                    .iter()
                    .take(CONFLICT_PATH_LIMIT)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("、");
                return Some(CommitBlocker::Conflicts(tr_args(
                    RESOLVE_CONFLICTS_KEY,
                    &[("paths", &paths)],
                )));
            }
            return Some(CommitBlocker::Operation);
        }
        if self.snapshot.staged_count() == 0 {
            return Some(CommitBlocker::NoStagedFiles);
        }
        if self.message_text(cx).trim().is_empty() {
            return Some(CommitBlocker::EmptyMessage);
        }
        None
    }

    fn set_staged(&mut self, paths: Vec<SharedString>, staged: bool, cx: &mut Context<Self>) {
        if paths.is_empty() || self.busy {
            return;
        }
        let Some(root) = self.repository_root_text() else {
            return;
        };

        for path in &paths {
            self.pending.insert(path.clone());
        }
        self.error = None;
        self.notice = None;
        cx.notify();

        let target: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
        let touched = paths.clone();
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(async move {
                    if staged {
                        stage(&root, &target)
                    } else {
                        unstage(&root, &target)
                    }
                })
                .await;

            let _ = this.update(cx, |view, cx| {
                for path in &touched {
                    view.pending.remove(path);
                }
                let verb = if staged { "stage" } else { "unstage" };
                view.record_write(verb, touched.len(), outcome.failure, None, cx);
            });
        })
        .detach();
    }

    fn set_all_staged(&mut self, staged: bool, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if !staged {
            let paths: Vec<SharedString> = self
                .snapshot
                .changes
                .iter()
                .filter(|row| row.staged)
                .map(|row| row.path.clone())
                .collect();
            self.set_staged(paths, false, cx);
            return;
        }

        let Some(root) = self.repository_root_text() else {
            return;
        };
        self.busy = true;
        self.error = None;
        self.notice = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let outcome = cx.background_spawn(async move { stage_all(&root) }).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.record_write("stage_all", 0, outcome.failure, None, cx);
            });
        })
        .detach();
    }

    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if let Some(blocker) = self.commit_blocker(cx) {
            self.error = Some(match blocker {
                CommitBlocker::Conflicts(message) => message,
                CommitBlocker::Operation => tr(FINISH_OPERATION_KEY),
                CommitBlocker::NoStagedFiles => tr(SELECT_FILES_KEY),
                CommitBlocker::EmptyMessage => tr(ENTER_MESSAGE_KEY),
            });
            cx.notify();
            return;
        }

        let Some(root) = self.repository_root_text() else {
            return;
        };
        let staged = self.staged_count();
        let text = self.message_text(cx).to_string();
        self.busy = true;
        self.error = None;
        self.notice = None;
        cx.notify();

        let serial = self.conflict.begin();
        let gate_root = root.clone();
        cx.spawn_in(window, async move |this, cx| {
            let markers = cx
                .background_spawn(async move { remote::conflict_markers(&gate_root) })
                .await;
            let _ = this.update_in(cx, |view, window, cx| {
                if !view.conflict.apply(serial, markers) {
                    eprintln!("S1_SOURCE_CONTROL commit_gate=stale_reply action=drop");
                    return;
                }
                match remote::conflict_gate(view.conflict.state()) {
                    ConflictGate::Clear => {
                        eprintln!("S1_SOURCE_CONTROL commit_gate=conflict_markers state=clear");
                        view.run_commit(root, staged, text, window, cx);
                    }
                    ConflictGate::Present { paths } => {
                        let (listed, more) = remote::join_paths(&paths, CONFLICT_PATH_LIMIT);
                        let more = if more == 0 {
                            String::new()
                        } else {
                            format!(" (+{more})")
                        };
                        eprintln!(
                            "S1_SOURCE_CONTROL commit_gate=conflict_markers state=blocked paths={}",
                            paths.len()
                        );
                        view.busy = false;
                        view.error = Some(tr_args(
                            CONFLICT_MARKERS_KEY,
                            &[("paths", &format!("{listed}{more}"))],
                        ));
                        cx.notify();
                    }
                    ConflictGate::Unavailable { reason } => {
                        eprintln!(
                            "S1_SOURCE_CONTROL commit_gate=conflict_markers state=unavailable error={reason}"
                        );
                        view.busy = false;
                        view.error = Some(tr(VERIFY_MARKERS_KEY));
                        cx.notify();
                    }
                    ConflictGate::Wait => {
                        eprintln!("S1_SOURCE_CONTROL commit_gate=conflict_markers state=wait");
                        view.busy = false;
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn run_commit(
        &mut self,
        root: String,
        staged: usize,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(window, async move |this, cx| {
            let outcome = cx
                .background_spawn(async move { write_commit(&root, &text) })
                .await;
            let _ = this.update_in(cx, |view, window, cx| {
                view.busy = false;
                let committed = outcome.failure.is_none();
                view.record_write("commit", staged, outcome.failure, None, cx);
                if committed {
                    view.notice = Some(tr(COMMITTED_KEY));
                    view.message.update(cx, |state, cx| {
                        state.set_value(SharedString::default(), window, cx)
                    });
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn run_operation_action(&mut self, operation: &'static str, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(root) = self.repository_root_text() else {
            return;
        };
        self.busy = true;
        self.error = None;
        self.notice = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(async move { operation_action(&root, operation) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.record_write(operation, 0, outcome.failure, None, cx);
            });
        })
        .detach();
    }

    fn discard_unstaged(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let paths: Vec<SharedString> = self
            .snapshot
            .changes
            .iter()
            .filter(|row| !row.kind.is_untracked() && row.worktree)
            .map(|row| row.path.clone())
            .collect();
        if paths.is_empty() {
            return;
        }

        if !self.confirm_before_discard {
            eprintln!(
                "S1_SOURCE_CONTROL discard_confirm=off paths={} action=immediate",
                paths.len()
            );
            self.confirm_discard(paths, cx);
            return;
        }

        eprintln!(
            "S1_SOURCE_CONTROL discard_confirm=on paths={} action=dialog",
            paths.len()
        );
        let view = cx.entity().downgrade();
        window.open_dialog(
            "lithe-source-control-discard",
            move |id, _window, cx| {
                let confirm_view = view.clone();
                let confirm_paths = paths.clone();
                let (cancel_id, confirm_id) = ((id.clone(), "cancel"), (id.clone(), "confirm"));
                Dialog::new(id)
                    .title(tr(DISCARD_ALL_KEY))
                    .on_close(dismiss_hosted)
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(tr(DISCARD_CONFIRM_KEY)),
                    )
                    .action(move |close| {
                        Button::new(cancel_id)
                            .label(tr(CANCEL_KEY))
                            .on_click(move |_, window, cx| close(window, cx))
                    })
                    .action(move |close| {
                        Button::new(confirm_id)
                            .danger()
                            .label(tr(ROLLBACK_KEY))
                            .on_click(move |_, window, cx| {
                                let paths = confirm_paths.clone();
                                let _ = confirm_view
                                    .update(cx, |view, cx| view.confirm_discard(paths, cx));
                                close(window, cx);
                            })
                    })
                    .into_any_element()
            },
            cx,
        );
    }

    fn confirm_discard(&mut self, paths: Vec<SharedString>, cx: &mut Context<Self>) {
        let Some(root) = self.repository_root_text() else {
            return;
        };
        let target: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
        let count = target.len();
        self.busy = true;
        self.error = None;
        self.notice = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(async move { discard(&root, &target) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.record_write("discard", count, outcome.failure, None, cx);
            });
        })
        .detach();
    }

    fn record_write(
        &mut self,
        verb: &str,
        paths: usize,
        failure: Option<String>,
        notice: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        match failure {
            Some(failure) => {
                eprintln!(
                    "S1_SOURCE_CONTROL write={verb} paths={paths} result=failed error={failure}"
                );
                self.error = Some(SharedString::from(failure));
            }
            None => {
                eprintln!("S1_SOURCE_CONTROL write={verb} paths={paths} result=ok");
                if notice.is_some() {
                    self.notice = notice;
                }
            }
        }
        self.refresh(cx);
    }

    fn repository_root_text(&self) -> Option<String> {
        self.snapshot
            .repository_root
            .as_ref()
            .map(|root| root.to_string())
    }

    fn diagnose_drop(&self) {
        eprintln!(
            "S1_SOURCE_CONTROL run=drop files={} staged={}",
            self.snapshot.changes.len(),
            self.snapshot.staged_count(),
        );
    }

    fn render_title_bar(&self, this: &WeakEntity<Self>, cx: &mut Context<Self>) -> AnyElement {
        let busy = self.busy;
        let discard_enabled = !busy
            && self
                .snapshot
                .changes
                .iter()
                .any(|row| !row.kind.is_untracked() && row.worktree);

        let menu_this = this.clone();
        let more = Button::new("changes-more")
            .ghost()
            .child(idea_icon_image(&idea::DOTS_THREE_ICON, cx, rems(1.)))
            .px_0()
            .tab_stop(false)
            .size_6()
            .rounded(rem_px(cx.theme().font_size, ICON_BUTTON_RADIUS))
            .tooltip(tr(ACTIONS_KEY))
            .accessibility_label(tr(ACTIONS_KEY))
            .dropdown_menu(move |menu, _window, _cx| {
                let refresh_this = menu_this.clone();
                let discard_this = menu_this.clone();
                menu.item(PopupMenuItem::new(tr(REFRESH_KEY)).on_click(
                    move |_event, _window, cx| {
                        let _ = refresh_this.update(cx, |view, cx| view.refresh(cx));
                    },
                ))
                .item(
                    PopupMenuItem::new(tr(DISCARD_ALL_KEY))
                        .disabled(!discard_enabled)
                        .on_click(move |_event, window, cx| {
                            let _ = discard_this
                                .update(cx, |view, cx| view.discard_unstaged(window, cx));
                        }),
                )
            });

        h_flex()
            .w_full()
            .flex_shrink_0()
            .h_9()
            .items_center()
            .gap_2()
            .px_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .line_height(rems(1.))
                    .text_color(cx.theme().foreground)
                    .child(tr(TITLE_KEY)),
            )
            .child(self.icon_button(
                "changes-refresh",
                idea_icon_image(&idea::REFRESH_ICON, cx, rems(1.)),
                tr(REFRESH_KEY),
                !matches!(self.state, LoadState::Loading),
                cx.theme().font_size,
                cx.listener(|view: &mut Self, _event, _window, cx| view.refresh(cx)),
            ))
            .child(more)
            .into_any_element()
    }

    fn icon_button(
        &self,
        id: &'static str,
        icon: AnyElement,
        label: SharedString,
        enabled: bool,
        rem: Pixels,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> AnyElement {
        let button = Button::new(id)
            .ghost()
            .child(icon)
            .px_0()
            .tab_stop(false)
            .size_6()
            .rounded(rem_px(rem, ICON_BUTTON_RADIUS))
            .tooltip(label.clone())
            .accessibility_label(label);
        if enabled {
            button.on_click(on_click).into_any_element()
        } else {
            button.disabled(true).into_any_element()
        }
    }

    fn render_load_error(&self, message: SharedString, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .p_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().danger.opacity(0.12))
            .text_sm()
            .text_color(cx.theme().danger)
            .child(message)
            .child(
                Button::new("changes-retry")
                    .label(tr(RETRY_KEY))
                    .small()
                    .compact()
                    .on_click(cx.listener(|view: &mut Self, _event, _window, cx| view.refresh(cx))),
            )
            .into_any_element()
    }

    fn render_write_error(&self, message: SharedString, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .items_start()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().danger.opacity(0.12))
            .text_sm()
            .text_color(cx.theme().danger)
            .child(idea_icon_image(
                &idea::WARNING_CIRCLE_ICON,
                cx,
                rems(14. / 16.),
            ))
            .child(div().flex_1().min_w_0().child(message))
            .child(self.icon_button(
                "changes-error-dismiss",
                idea_icon_image(&idea::X_ICON, cx, rems(1.)),
                tr(CANCEL_KEY),
                true,
                cx.theme().font_size,
                cx.listener(|view: &mut Self, _event, _window, cx| {
                    view.error = None;
                    cx.notify();
                }),
            ))
            .into_any_element()
    }

    fn render_notice(&self, message: SharedString, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().success.opacity(0.12))
            .text_sm()
            .text_color(cx.theme().success)
            .child(idea_icon_image(&idea::CHECK_ICON, cx, rems(14. / 16.)))
            .child(div().flex_1().min_w_0().child(message))
            .child(self.icon_button(
                "changes-notice-dismiss",
                idea_icon_image(&idea::X_ICON, cx, rems(1.)),
                tr(CANCEL_KEY),
                true,
                cx.theme().font_size,
                cx.listener(|view: &mut Self, _event, _window, cx| {
                    view.notice = None;
                    cx.notify();
                }),
            ))
            .into_any_element()
    }

    fn render_operation_banner(
        &self,
        operation: &OperationState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut banner = v_flex()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .px_3()
            .py_2p5()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .text_sm()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(idea_icon_image(
                        &idea::WARNING_CIRCLE_ICON,
                        cx,
                        rems(14. / 16.),
                    ))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .child(tr(operation.kind.title_key())),
                    )
                    .children(operation.reference.as_ref().map(|reference| {
                        div()
                            .min_w_0()
                            .truncate()
                            .text_color(cx.theme().muted_foreground)
                            .child(SharedString::from(format!(
                                "— {}",
                                reference.chars().take(7).collect::<String>()
                            )))
                    })),
            );

        if let (Some(step), Some(total)) = (operation.step, operation.total) {
            banner = banner.child(div().text_color(cx.theme().muted_foreground).child(tr_args(
                OPERATION_STEP_KEY,
                &[("step", &step.to_string()), ("total", &total.to_string())],
            )));
        }

        if operation.has_conflicts() {
            let paths = operation
                .conflicted_paths
                .iter()
                .take(CONFLICT_PATH_LIMIT)
                .map(|path| path.to_string())
                .collect::<Vec<_>>()
                .join("、");
            banner = banner.child(
                v_flex()
                    .gap_1()
                    .child(div().text_color(cx.theme().danger).child(tr_args(
                        RESOLVE_CONFLICTS_HINT_KEY,
                        &[("count", &operation.conflicted_paths.len().to_string())],
                    )))
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child(SharedString::from(paths)),
                    ),
            );
        } else {
            banner = banner.child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr(CONFLICTS_RESOLVED_KEY)),
            );
        }

        let can_continue = !self.busy && !operation.has_conflicts();
        let mut actions = h_flex().items_center().gap_2().child(
            Button::new("changes-operation-continue")
                .label(tr(operation.kind.continue_key()))
                .small()
                .compact()
                .disabled(!can_continue)
                .on_click(cx.listener(|view: &mut Self, _event, _window, cx| {
                    view.run_operation_action("operationContinue", cx)
                })),
        );

        if operation.kind.can_skip() {
            actions = actions.child(
                Button::new("changes-operation-skip")
                    .label(tr(SKIP_KEY))
                    .small()
                    .compact()
                    .disabled(self.busy)
                    .on_click(cx.listener(|view: &mut Self, _event, _window, cx| {
                        view.run_operation_action("operationSkip", cx)
                    })),
            );
        }

        actions = actions.child(
            Button::new("changes-operation-abort")
                .label(tr(ABORT_KEY))
                .ghost()
                .small()
                .compact()
                .disabled(self.busy)
                .on_click(cx.listener(|view: &mut Self, _event, _window, cx| {
                    view.run_operation_action("operationAbort", cx)
                })),
        );

        banner.child(actions).into_any_element()
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let unstaged = self.snapshot.unstaged_count();
        let staged = self.staged_count();

        let mut row = h_flex()
            .w_full()
            .flex_shrink_0()
            .h_9()
            .items_center()
            .gap_2()
            .px_2()
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap_1()
                    .text_color(cx.theme().muted_foreground)
                    .children(
                        self.snapshot
                            .branch
                            .as_ref()
                            .map(|branch| div().min_w_0().truncate().child(branch.clone())),
                    )
                    .children((self.snapshot.ahead > 0).then(|| {
                        div().child(SharedString::from(format!("↑{}", self.snapshot.ahead)))
                    }))
                    .children((self.snapshot.behind > 0).then(|| {
                        div().child(SharedString::from(format!("↓{}", self.snapshot.behind)))
                    })),
            );

        if unstaged > 0 {
            row = row.child(self.icon_button(
                "changes-stage-all",
                idea_icon_image(&idea::PLUS_ICON, cx, rems(1.)),
                tr(STAGE_ALL_KEY),
                !self.busy,
                cx.theme().font_size,
                cx.listener(|view: &mut Self, _event, _window, cx| view.set_all_staged(true, cx)),
            ));
        }
        if staged > 0 {
            row = row.child(self.icon_button(
                "changes-unstage-all",
                idea_icon_image(&idea::MINUS_ICON, cx, rems(1.)),
                tr(UNSTAGE_ALL_KEY),
                !self.busy,
                cx.theme().font_size,
                cx.listener(|view: &mut Self, _event, _window, cx| view.set_all_staged(false, cx)),
            ));
        }

        row.into_any_element()
    }

    fn render_section_header(label: SharedString, count: usize, cx: &App) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .h_8()
            .items_center()
            .gap_2()
            .px_2()
            .pb_0p5()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .text_color(cx.theme().foreground)
            .child(label)
            .child(
                div()
                    .flex_shrink_0()
                    .px_1p5()
                    .rounded(rems(ROW_RADIUS / 16.))
                    .bg(cx.theme().muted)
                    .text_color(cx.theme().muted_foreground)
                    .child(SharedString::from(count.to_string())),
            )
            .into_any_element()
    }

    fn render_row(&self, row: &ChangeRow, cx: &mut Context<Self>) -> AnyElement {
        let file_name = row
            .path
            .rsplit('/')
            .next()
            .map(SharedString::from)
            .unwrap_or_else(|| row.path.clone());
        let directory = row
            .path
            .rsplit_once('/')
            .map(|(directory, _)| SharedString::from(directory.to_string()));

        let pending = self.pending.contains(&row.path) || self.busy;
        let staged = row.staged;
        let label = tr_args(
            if staged {
                "lithe.git.unstageFileNamed"
            } else {
                "lithe.git.stageFileNamed"
            },
            &[("name", file_name.as_ref())],
        );

        let path = row.path.clone();
        let action = if pending {
            div()
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .size_5()
                .child(Spinner::new().color(cx.theme().muted_foreground))
                .into_any_element()
        } else {
            Button::new(stage_button_element_id(row))
                .ghost()
                .child(if staged {
                    idea_icon_image(&idea::MINUS_ICON, cx, rems(1.))
                } else {
                    idea_icon_image(&idea::PLUS_ICON, cx, rems(1.))
                })
                .tab_stop(false)
                .size_5()
                .rounded(rem_px(cx.theme().font_size, ROW_RADIUS))
                .tooltip(label.clone())
                .accessibility_label(label)
                .on_click(cx.listener(move |view: &mut Self, _event, _window, cx| {
                    view.set_staged(vec![path.clone()], !staged, cx);
                }))
                .into_any_element()
        };

        h_flex()
            .id(change_row_element_id(row))
            .w_full()
            .flex_shrink_0()
            .h_6()
            .items_center()
            .gap_1()
            .px_1p5()
            .rounded(rems(ROW_RADIUS / 16.))
            .text_sm()
            .line_height(relative(ROW_LINE_HEIGHT))
            .whitespace_nowrap()
            .overflow_hidden()
            .aria_label(row_aria_label(row))
            .hover(|style| style.bg(cx.theme().list_hover))
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(row.kind.color(cx))
                    .child(file_name),
            )
            .children(directory.map(|directory| {
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(cx.theme().muted_foreground)
                    .child(directory)
            }))
            .child(div().flex_1())
            .child(action)
            .into_any_element()
    }

    fn render_conflict_markers(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let ConflictGate::Present { paths } = remote::conflict_gate(self.conflict.state()) else {
            return None;
        };
        let (listed, more) = remote::join_paths(&paths, CONFLICT_PATH_LIMIT);
        let more = if more == 0 {
            String::new()
        } else {
            format!(" (+{more})")
        };
        let root = self.repository_root_text().map(PathBuf::from);
        let on_open = self.on_open.clone();
        let open_label = tr(OPEN_FILE_KEY);

        let mut rows: Vec<AnyElement> = Vec::with_capacity(paths.len().min(CONFLICT_PATH_LIMIT));
        for path in paths.iter().take(CONFLICT_PATH_LIMIT) {
            let absolute = root.as_ref().map(|root| root.join(path.as_ref()));
            let open = on_open.clone();
            let label = open_label.clone();
            let path_text = path.clone();
            rows.push(
                h_flex()
                    .id(SharedString::from(format!("changes-conflict-open-{path}")))
                    .w_full()
                    .h_6()
                    .items_center()
                    .gap_1()
                    .px_1p5()
                    .rounded(rems(RADIUS_MD / 16.))
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .hover(|style| style.bg(cx.theme().accent))
                    .child(div().min_w_0().truncate().child(path.clone()))
                    .aria_label(SharedString::from(format!(
                        "{} {}",
                        label,
                        path_text.as_ref()
                    )))
                    .on_click(
                        move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
                            let Some(absolute) = absolute.clone() else {
                                return;
                            };
                            eprintln!(
                                "S1_SOURCE_CONTROL conflict_open path={} line=none",
                                path_text.as_ref()
                            );
                            open(absolute, None, window, cx);
                        },
                    )
                    .into_any_element(),
            );
        }

        Some(
            v_flex()
                .w_full()
                .flex_shrink_0()
                .gap_1()
                .px_1p5()
                .pb_2()
                .child(
                    h_flex()
                        .w_full()
                        .items_start()
                        .gap_2()
                        .px_1p5()
                        .py_1()
                        .rounded(rems(RADIUS_MD / 16.))
                        .bg(cx.theme().danger.opacity(0.12))
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(idea_icon_image(
                            &idea::WARNING_CIRCLE_ICON,
                            cx,
                            rems(14. / 16.),
                        ))
                        .child(div().flex_1().min_w_0().child(tr_args(
                            CONFLICT_MARKERS_KEY,
                            &[("paths", &format!("{listed}{more}"))],
                        ))),
                )
                .children(rows)
                .into_any_element(),
        )
    }

    fn render_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let tracked = self.snapshot.tracked_rows();
        let untracked = self.snapshot.untracked_rows();
        let tree_label = SharedString::from(format!(
            "{} / {}",
            tr(TRACKED_FILES_KEY),
            tr(UNTRACKED_FILES_KEY)
        ));
        let mut children: Vec<AnyElement> = Vec::new();

        if !tracked.is_empty() {
            children.push(Self::render_section_header(
                tr(TRACKED_KEY),
                tracked.len(),
                cx,
            ));
            for row in &tracked {
                children.push(self.render_row(row, cx));
            }
        }

        if !untracked.is_empty() {
            if !tracked.is_empty() {
                children.push(div().w_full().flex_shrink_0().pb_2().into_any_element());
            }
            children.push(Self::render_section_header(
                tr(UNTRACKED_KEY),
                untracked.len(),
                cx,
            ));
            for row in &untracked {
                children.push(self.render_row(row, cx));
            }
        }

        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(self.render_toolbar(cx))
            .children(self.render_conflict_markers(cx))
            .child(
                v_flex()
                    .id("changes-scroll")
                    .role(Role::Tree)
                    .aria_label(tree_label)
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .px_1p5()
                    .py_2()
                    .overflow_y_scroll()
                    .children(children),
            )
            .into_any_element()
    }

    fn render_body(&self, cx: &mut Context<Self>) -> AnyElement {
        match &self.state {
            LoadState::Loading => self.centered(
                Some(
                    Spinner::new()
                        .color(cx.theme().muted_foreground)
                        .into_any_element(),
                ),
                tr(LOADING_KEY),
                None,
                cx,
            ),
            LoadState::Failed(_) => self.centered(None, tr(LOAD_FAILED_KEY), None, cx),
            LoadState::Ready => {
                if self.snapshot.repository_root.is_none() {
                    return self.centered(
                        None,
                        tr(NOT_REPOSITORY_KEY),
                        Some((tr(NOT_REPOSITORY_HINT_KEY), tr(INITIALIZE_KEY), false)),
                        cx,
                    );
                }
                if self.snapshot.changes.is_empty() {
                    if self.snapshot.has_commits == Some(false) {
                        return self.centered(
                            None,
                            tr(NO_COMMITS_KEY),
                            Some((tr(NO_COMMITS_HINT_KEY), SharedString::default(), true)),
                            cx,
                        );
                    }
                    return self.centered(
                        Some(idea_icon_image(&idea::CHECK_ICON, cx, rems(14. / 16.))),
                        tr(CLEAN_KEY),
                        None,
                        cx,
                    );
                }
                self.render_list(cx)
            }
        }
    }

    fn centered(
        &self,
        icon: Option<AnyElement>,
        message: SharedString,
        action: Option<(SharedString, SharedString, bool)>,
        cx: &App,
    ) -> AnyElement {
        let mut header = EmptyHeader::new().gap_2();
        if let Some(icon) = icon {
            header = header.media(EmptyMedia::new().child(icon));
        }
        header = header.description(
            EmptyDescription::new()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(message),
        );

        let mut empty = Empty::new()
            .border_color(cx.theme().transparent)
            .p_3()
            .header(header);
        if let Some((hint, label, enabled)) = action {
            let mut content = v_flex().gap_2().items_center().child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(hint),
            );
            if !label.is_empty() {
                content = content.child(
                    Button::new("changes-empty-action")
                        .label(label)
                        .small()
                        .compact()
                        .disabled(!enabled),
                );
            }
            empty = empty.content(EmptyContent::new().child(content));
        }

        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(empty)
            .into_any_element()
    }

    fn render_commit_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let staged = self.staged_count();
        let panel_error: Option<SharedString> =
            self.error
                .clone()
                .or_else(|| match &self.snapshot.operation {
                    Some(operation) if operation.has_conflicts() => Some(tr_args(
                        RESOLVE_CONFLICTS_KEY,
                        &[(
                            "paths",
                            &operation
                                .conflicted_paths
                                .iter()
                                .take(CONFLICT_PATH_LIMIT)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join("、"),
                        )],
                    )),
                    Some(_) => Some(tr(FINISH_OPERATION_KEY)),
                    None => None,
                });

        let mut panel = v_flex()
            .w_full()
            .flex_shrink_0()
            .mx_2()
            .mb_2()
            .rounded(rems(COMMIT_PANEL_RADIUS / 16.))
            .border_1()
            .border_color(cx.theme().border)
            .overflow_hidden();

        if let Some(error) = panel_error {
            panel = panel.child(
                div()
                    .mx_2()
                    .mt_2()
                    .px_2()
                    .py_1p5()
                    .rounded(rems(COMMIT_ERROR_RADIUS / 16.))
                    .border_1()
                    .border_color(cx.theme().danger.opacity(0.3))
                    .bg(cx.theme().danger.opacity(0.1))
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(error),
            );
        }

        panel = panel.child(
            Textarea::new(&self.message)
                .h(rems(COMMIT_INPUT_HEIGHT / 16.))
                .disabled(self.busy)
                .w_full()
                .text_sm(),
        );

        let count_label = if staged == 0 {
            tr(NO_FILES_STAGED_KEY)
        } else {
            tr_args(FILES_STAGED_KEY, &[("count", &staged.to_string())])
        };
        let can_commit = !self.busy && self.commit_blocker(cx).is_none();

        panel
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .px_1()
                    .pt_1p5()
                    .pb_1()
                    .text_sm()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(cx.theme().muted_foreground)
                            .child(count_label),
                    )
                    .child(
                        Button::new("changes-commit")
                            .primary()
                            .label(if self.busy {
                                tr(COMMITTING_KEY)
                            } else {
                                tr(COMMIT_KEY)
                            })
                            .h_7()
                            .px_3()
                            .disabled(!can_commit)
                            .on_click(cx.listener(|view: &mut Self, _event, window, cx| {
                                view.commit(window, cx)
                            })),
                    ),
            )
            .into_any_element()
    }

    fn shows_commit_panel(&self) -> bool {
        matches!(self.state, LoadState::Ready)
            && self.snapshot.repository_root.is_some()
            && self.snapshot.has_commits != Some(false)
    }
}

impl Drop for ChangesView {
    fn drop(&mut self) {
        self.diagnose_drop();
    }
}

fn row_aria_label(row: &ChangeRow) -> SharedString {
    let mut label = format!("{} {}", row.raw_status, row.path);
    if let Some(original) = &row.original_path {
        label.push_str(&format!(" ← {original}"));
    }
    SharedString::from(label)
}

fn change_row_element_id(row: &ChangeRow) -> String {
    format!("changes-row:{}", row.path)
}

fn stage_button_element_id(row: &ChangeRow) -> String {
    format!("changes-stage-row:{}", row.path)
}

impl Render for ChangesView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let title_bar = self.render_title_bar(&this, cx);
        let load_error = match &self.state {
            LoadState::Failed(message) => Some(self.render_load_error(
                SharedString::from(format!("{} {message}", tr(LOAD_FAILED_KEY))),
                cx,
            )),
            _ => None,
        };
        let write_error = self
            .error
            .clone()
            .map(|message| self.render_write_error(message, cx));
        let notice = self
            .notice
            .clone()
            .map(|message| self.render_notice(message, cx));
        let banner = self
            .snapshot
            .operation
            .clone()
            .map(|operation| self.render_operation_banner(&operation, cx));
        let body = self.render_body(cx);
        let commit_panel = self
            .shows_commit_panel()
            .then(|| self.render_commit_panel(cx));

        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(title_bar)
            .children(load_error)
            .children(write_error)
            .children(notice)
            .children(banner)
            .child(body)
            .children(commit_panel)
            .on_action(
                cx.listener(|view: &mut Self, _: &CommitChanges, window, cx| {
                    view.commit(window, cx)
                }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{change_row_element_id, stage_button_element_id};
    use crate::changes::{ChangeKind, ChangeRow};
    use gpui_kit::SharedString;

    fn row(path: &str, status: &str, untracked: bool) -> ChangeRow {
        ChangeRow {
            path: SharedString::from(path),
            original_path: None,
            raw_status: SharedString::from(status),
            kind: ChangeKind::parse(status, untracked),
            staged: !status.starts_with(' ') && !untracked,
            worktree: !status.ends_with(' ') && !untracked,
        }
    }

    #[test]
    fn row_element_ids_do_not_depend_on_the_row_order() {
        let rows = [
            row("src/main.rs", " M", false),
            row("README.md", "??", true),
            row("src/lib.rs", "D ", false),
        ];

        let forward: Vec<String> = rows.iter().map(change_row_element_id).collect();
        let reversed: Vec<String> = rows.iter().rev().map(change_row_element_id).collect();

        assert_eq!(
            forward[0], reversed[2],
            "同一行在不同顺序下必须算出同一个 id"
        );
        assert_eq!(
            forward[1], reversed[1],
            "同一行在不同顺序下必须算出同一个 id"
        );
        assert_eq!(
            forward[2], reversed[0],
            "同一行在不同顺序下必须算出同一个 id"
        );
        assert_eq!(forward[0], "changes-row:src/main.rs", "id 必须由路径构成");
        for (left, right) in [(0, 1), (0, 2), (1, 2)] {
            assert_ne!(forward[left], forward[right], "不同文件的 id 必须互不相同");
        }
    }

    #[test]
    fn stage_button_id_is_namespaced_apart_from_its_row() {
        let target = row("src/main.rs", " M", false);
        let other = row("src/lib.rs", " M", false);

        assert_ne!(
            change_row_element_id(&target),
            stage_button_element_id(&target)
        );
        assert_ne!(
            stage_button_element_id(&target),
            stage_button_element_id(&other)
        );
        assert_eq!(
            stage_button_element_id(&target),
            "changes-stage-row:src/main.rs"
        );
    }
}
