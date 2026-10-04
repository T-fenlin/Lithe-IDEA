use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::Receiver;

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AppContext as _, Context, EventEmitter, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Task,
    Window, div,
};

use lithe_gpui_shared::icons::{idea, idea_icon_mono};
use lithe_gpui_shared::{core_json, tr, tr_args};

use crate::model::{RunConfigurationView, load};
use crate::plan::ToolchainResolver;
use crate::session::{PreparedStart, RunSession, RunState, prepare};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunPaneEvent {
    Started,
    Finished,
}

struct LaunchOutcome {
    id: String,
    name: String,
    prepared: PreparedStart,
    debug_port: Option<u16>,
}

pub struct RunPane {
    root: Option<PathBuf>,
    resolve: Option<ToolchainResolver>,
    configurations: Vec<RunConfigurationView>,
    selected: Option<usize>,
    session: Option<RunSession>,
    pump: Option<Task<()>>,
    load: Option<Task<()>>,
    launch: Option<Task<()>>,
    notice: Option<SharedString>,
    debug_port: Option<u16>,
    probe: bool,
    probe_debug: bool,
}

impl EventEmitter<RunPaneEvent> for RunPane {}

impl RunPane {
    pub fn new() -> Self {
        Self {
            root: None,
            resolve: None,
            configurations: Vec::new(),
            selected: None,
            session: None,
            pump: None,
            load: None,
            launch: None,
            notice: None,
            debug_port: None,
            probe: false,
            probe_debug: false,
        }
    }

    pub fn set_probe(&mut self, probe: bool, debug: bool) {
        self.probe = probe;
        self.probe_debug = debug;
    }

    pub fn set_workspace(
        &mut self,
        root: PathBuf,
        resolve: ToolchainResolver,
        cx: &mut Context<Self>,
    ) {
        let changed = self.root.as_ref() != Some(&root);
        self.root = Some(root);
        self.resolve = Some(resolve);
        if changed {
            self.stop_session(cx);
            self.selected = None;
            self.configurations.clear();
            self.notice = None;
        }
        self.reload(cx);
    }

    pub fn configurations(&self) -> &[RunConfigurationView] {
        &self.configurations
    }

    pub fn selected(&self) -> Option<&RunConfigurationView> {
        self.selected
            .and_then(|index| self.configurations.get(index))
    }

    pub fn debug_without_jdwp(&self) -> bool {
        self.debug_port.is_some()
            && self
                .session
                .as_ref()
                .is_some_and(|session| !session.suspends_for_debugger())
    }

    pub fn debug_port(&self) -> Option<u16> {
        self.debug_port
    }

    pub fn is_running(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.state().is_live())
    }

    pub fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.configurations.len() {
            self.selected = Some(index);
            cx.notify();
        }
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.root.clone() else {
            return;
        };
        self.load = Some(cx.spawn(async move |this, cx| {
            let loaded = cx.background_spawn(async move { load(&root) }).await;
            let _ = this.update(cx, |this, cx| {
                match loaded {
                    Ok(view) => {
                        this.configurations = view.configurations;
                        let selected = this
                            .selected
                            .filter(|index| *index < this.configurations.len())
                            .or((!this.configurations.is_empty()).then_some(0));
                        this.selected = selected;
                        this.notice = None;
                        println!(
                            "S1_RUN_CONFIGS configs={} selected={}",
                            this.configurations.len(),
                            selected
                                .map(|index| index.to_string())
                                .unwrap_or_else(|| "-".to_string())
                        );
                        if this.probe && !this.configurations.is_empty() {
                            this.probe = false;
                            let debug = this.probe_debug;
                            println!("S1_RUN_PROBE launching=first-configuration debug={debug}");
                            this.launch(debug, cx);
                        }
                    }
                    Err(error) => {
                        this.configurations.clear();
                        this.selected = None;
                        this.notice = Some(tr_args(
                            "lithe.run.gpui.planFailed",
                            &[("reason", error.as_str())],
                        ));
                        println!("S1_RUN_CONFIGS result=failed error={error}");
                    }
                }
                cx.notify();
            });
        }));
    }

    pub fn launch(&mut self, debug: bool, cx: &mut Context<Self>) {
        let (Some(root), Some(resolve)) = (self.root.clone(), self.resolve.clone()) else {
            self.notice = Some(tr("lithe.run.gpui.noWorkspace"));
            cx.notify();
            return;
        };
        let Some(configuration) = self.selected().cloned() else {
            self.notice = Some(tr("lithe.run.gpui.noConfiguration"));
            cx.notify();
            return;
        };

        self.stop_session(cx);
        self.notice = None;
        self.debug_port = debug.then(free_local_port).flatten();

        let id = configuration.id.clone();
        let name = configuration.name.clone();
        let debug_port = self.debug_port;
        self.launch = Some(cx.spawn(async move |this, cx| {
            let planned = {
                let task_root = root.clone();
                let task_id = id.clone();
                let task_resolve = resolve.clone();
                cx.background_spawn(async move {
                    prepare_startup(&task_root, &task_id, debug_port, &task_resolve).map(
                        |prepared| LaunchOutcome {
                            id: task_id,
                            name,
                            prepared,
                            debug_port,
                        },
                    )
                })
                .await
            };

            let _ = this.update(cx, |this, cx| {
                match planned {
                    Ok(outcome) => {
                        match RunSession::start(
                            &outcome.id,
                            &outcome.name,
                            outcome.prepared,
                            outcome.debug_port,
                        ) {
                            Ok(mut session) => {
                                this.pump = session
                                    .take_receiver()
                                    .map(|receiver| spawn_pump(receiver, cx));
                                this.session = Some(session);
                                this.notice = None;
                                cx.emit(RunPaneEvent::Started);
                            }
                            Err(error) => {
                                this.notice = Some(tr_args(
                                    "lithe.run.gpui.planFailed",
                                    &[("reason", error.as_str())],
                                ));
                                println!("S1_RUN_START result=failed error={error}");
                            }
                        }
                    }
                    Err(error) => {
                        this.notice = Some(tr_args(
                            "lithe.run.gpui.planFailed",
                            &[("reason", error.as_str())],
                        ));
                        println!("S1_RUN_PLAN result=failed error={error}");
                    }
                }
                cx.notify();
            });
        }));
    }

    pub fn stop(&mut self, cx: &mut Context<Self>) {
        self.stop_session(cx);
        cx.notify();
    }

    fn stop_session(&mut self, cx: &mut Context<Self>) {
        let Some(mut session) = self.session.take() else {
            return;
        };
        session.stop();
        self.pump = None;
        cx.emit(RunPaneEvent::Finished);
    }

    pub fn clear_output(&mut self, cx: &mut Context<Self>) {
        if let Some(session) = self.session.as_ref() {
            session.clear_output();
        }
        cx.notify();
    }

    fn command_line(&self) -> Option<String> {
        self.session
            .as_ref()
            .map(|session| session.command_line().to_string())
    }

    fn output_truncated(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.output_truncated())
    }
}

impl Default for RunPane {
    fn default() -> Self {
        Self::new()
    }
}

fn spawn_pump(receiver: Receiver<()>, cx: &mut Context<RunPane>) -> Task<()> {
    let receiver = Arc::new(Mutex::new(receiver));
    cx.spawn(async move |this, cx| {
        loop {
            let receiver = receiver.clone();
            let event = cx
                .background_spawn(async move { blocking_recv(&receiver) })
                .await;
            let Some(()) = event else { break };
            let settled = this
                .update(cx, |this, cx| {
                    if let Some(session) = this.session.as_mut() {
                        let settled = session.settle_exit();
                        cx.notify();
                        if settled {
                            println!(
                                "S1_RUN_EXIT id={} code={} lines={} truncated={}",
                                session.configuration_id(),
                                session
                                    .exit_code()
                                    .map(|code| code.to_string())
                                    .unwrap_or_else(|| "-".to_string()),
                                session.output_lines().len(),
                                session.output_truncated(),
                            );
                            cx.emit(RunPaneEvent::Finished);
                        }
                        settled
                    } else {
                        true
                    }
                })
                .unwrap_or(true);
            if settled {
                break;
            }
        }
    })
}

fn blocking_recv(receiver: &Arc<Mutex<Receiver<()>>>) -> Option<()> {
    let guard = match receiver.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.recv().ok()
}

fn prepare_startup(
    root: &Path,
    configuration_id: &str,
    debug_port: Option<u16>,
    resolve: &ToolchainResolver,
) -> Result<PreparedStart, String> {
    let plan = launch_plan(root, configuration_id, debug_port)?;
    prepare(root, &plan, resolve)
}

/// Workspace-relative Java file a probe run should treat as the current file.
///
/// The launch path otherwise takes the current file from the editor, which a
/// headless probe does not have; naming it through the environment lets the
/// run/debug probe exercise the real launch plan.
fn current_file_for_probe() -> Option<String> {
    std::env::var("LITHE_RUN_CURRENT_FILE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn launch_plan(
    root: &Path,
    configuration_id: &str,
    debug_port: Option<u16>,
) -> Result<serde_json::Value, String> {
    let root_text = root
        .to_str()
        .ok_or_else(|| "工作区根不是合法 UTF-8 路径".to_string())?;
    let mut request = serde_json::json!({
        "root": root_text,
        "configurationId": configuration_id,
    });
    if let Some(port) = debug_port {
        request["debugPort"] = serde_json::json!(port);
    }
    if let Some(current_file) = current_file_for_probe() {
        request["currentFile"] = serde_json::json!(current_file);
    }

    let plan = core_json("runConfig.createLaunchPlan", request)
        .map_err(|error| core_error_text(&error))?
        .ok_or_else(|| "启动计划的响应是空的".to_string())?;
    println!(
        "S1_RUN_PLAN id={configuration_id} debug_port={}",
        debug_port
            .map(|port| port.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    Ok(plan)
}

fn core_error_text(error: &lithe_gpui_shared::CoreError) -> String {
    match error.code() {
        Some(code) => format!("{code}: {error}"),
        None => error.to_string(),
    }
}

fn free_local_port() -> Option<u16> {
    std::net::TcpListener::bind("127.0.0.1:0")
        .ok()
        .and_then(|listener| listener.local_addr().ok())
        .map(|address| address.port())
}

impl Render for RunPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(self.toolbar(cx))
            .when_some(self.command_line(), |this, command| {
                this.child(
                    div()
                        .w_full()
                        .px_2()
                        .pb_1()
                        .text_xs()
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_color(cx.theme().muted_foreground)
                        .child(SharedString::from(command)),
                )
            })
            .when(self.debug_without_jdwp(), |this| {
                this.child(
                    div()
                        .w_full()
                        .px_2()
                        .pb_1()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("lithe.run.gpui.debugUnsupported")),
                )
            })
            .when_some(self.notice.clone(), |this, notice| {
                this.child(
                    div()
                        .w_full()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .child(notice),
                )
            })
            .child(
                v_flex()
                    .id("lithe-run-body")
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(if self.session.is_some() {
                        self.console(cx)
                    } else {
                        self.configuration_list(cx)
                    }),
            )
            .into_any_element()
    }
}

impl RunPane {
    fn toolbar(&self, cx: &Context<Self>) -> AnyElement {
        let running = self.is_running();
        let name = self
            .session
            .as_ref()
            .map(|session| session.configuration_name().to_string())
            .or_else(|| {
                self.selected()
                    .map(|configuration| configuration.name.clone())
            })
            .unwrap_or_else(|| tr("lithe.run.configurations").to_string());
        let status = self.status_text(cx);

        let mut row = h_flex()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .font_medium()
                    .text_color(cx.theme().foreground)
                    .child(name),
            )
            .when_some(status, |this, status| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(status),
                )
            });

        row = row
            .child(div().flex_1())
            .child(
                Button::new("run-pane-run")
                    .small()
                    .ghost()
                    .icon(idea_icon_mono(&idea::PLAY_ICON, cx.theme().is_dark()))
                    .tooltip(tr("lithe.run.run"))
                    .disabled(running)
                    .on_click(cx.listener(|this, _, _, cx| this.launch(false, cx))),
            )
            .child(
                Button::new("run-pane-debug")
                    .small()
                    .ghost()
                    .icon(idea_icon_mono(&idea::BUG_ICON, cx.theme().is_dark()))
                    .tooltip(tr("lithe.debugger.start"))
                    .disabled(running)
                    .on_click(cx.listener(|this, _, _, cx| this.launch(true, cx))),
            )
            .child(
                Button::new("run-pane-stop")
                    .small()
                    .ghost()
                    .icon(idea_icon_mono(&idea::STOP_ICON, cx.theme().is_dark()))
                    .tooltip(tr("lithe.run.stop"))
                    .disabled(!running)
                    .on_click(cx.listener(|this, _, _, cx| this.stop(cx))),
            )
            .child(
                Button::new("run-pane-clear")
                    .small()
                    .ghost()
                    .icon(idea_icon_mono(&idea::TRASH_ICON, cx.theme().is_dark()))
                    .tooltip(tr("lithe.run.clearOutput"))
                    .disabled(self.session.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.clear_output(cx))),
            );
        row.into_any_element()
    }

    fn status_text(&self, _cx: &Context<Self>) -> Option<SharedString> {
        let session = self.session.as_ref()?;
        match session.state() {
            RunState::WaitingForDebugger { port } => {
                let port = port.to_string();
                Some(tr_args(
                    "lithe.run.gpui.debugWaiting",
                    &[("port", port.as_str())],
                ))
            }
            RunState::Exited(code) => {
                let code = code.to_string();
                Some(tr_args(
                    "lithe.run.gpui.exitCode",
                    &[("code", code.as_str())],
                ))
            }
            state => Some(tr(state.label_key())),
        }
    }

    fn configuration_list(&self, cx: &Context<Self>) -> AnyElement {
        if self.configurations.is_empty() {
            return v_flex()
                .w_full()
                .items_center()
                .justify_center()
                .gap_1()
                .py_6()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().foreground)
                        .child(tr("lithe.settings.gpui.runNothingDetected")),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("lithe.run.emptyOutput")),
                )
                .into_any_element();
        }

        let mut column = v_flex().w_full();
        for (index, configuration) in self.configurations.iter().enumerate() {
            let selected = self.selected == Some(index);
            column = column.child(
                h_flex()
                    .id(SharedString::from(format!(
                        "run-configuration-{index}-{}",
                        configuration.id
                    )))
                    .w_full()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .cursor_pointer()
                    .when(selected, |this| this.bg(cx.theme().accent))
                    .on_click(cx.listener(move |this, _, _, cx| this.select(index, cx)))
                    .child(
                        idea_icon_mono(&idea::PLAY_ICON, cx.theme().is_dark())
                            .size_3()
                            .text_color(if selected {
                                cx.theme().primary
                            } else {
                                cx.theme().muted_foreground
                            }),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(configuration.name.clone()),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(tr(configuration.execution.label_key())),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(configuration.provider_title.clone()),
                    ),
            );
        }
        column.into_any_element()
    }

    fn console(&self, cx: &Context<Self>) -> AnyElement {
        let Some(session) = self.session.as_ref() else {
            return div()
                .p_2()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(tr("lithe.run.selectConfiguration"))
                .into_any_element();
        };
        let lines = session.output_lines();
        let mono = cx.theme().mono_font_family.clone();
        let mut column = v_flex().w_full().p_2().gap_0();
        if self.output_truncated() {
            column = column.child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().warning)
                    .child(tr("lithe.run.gpui.outputTruncated")),
            );
        }
        if lines.is_empty() {
            column = column.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.run.emptyOutput")),
            );
        }
        column = column.children(lines.into_iter().map(move |line| {
            div()
                .w_full()
                .text_xs()
                .font_family(mono.clone())
                .text_color(cx.theme().foreground)
                .child(SharedString::from(line))
        }));
        column.into_any_element()
    }
}
