//! 「运行」工具窗：**配置列表 + 一条真进程的输出控制台**。
//!
//! ## 面板里有什么（以及刻意没有什么）
//!
//! ```text
//! ┌ 工具栏：当前配置名 · 状态 ──────────────── ▶ 运行   🐞 调试   ■ 停止   ⌫ 清空 ┐
//! ├ 正文：还没有会话 → 配置列表（点一行选中；选中行有底色）                     │
//! │        有会话     → 输出控制台（等宽字体、自动跟到底部、超上限时如实标注）    │
//! ```
//!
//! **没有**的东西（如实登记，不画假控件）：配置编辑器（改参数 / 环境变量 / JVM 参数）、
//! 多配置并行与输出页签切换、断点与 DAP（单步 / 变量 / 调用栈）、运行配置筛选与搜索。
//!
//! ## 状态与事件
//!
//! 面板向宿主播报两件事（[`RunPaneEvent`]）：一次运行**开始了**（外壳据此把底部工具窗切到
//! 「运行」并显示 —— 点 `▶` 之后看不到输出是最糟的体验），以及运行**结束了**
//! （外壳据此决定要不要保留面板）。这与终端 `TerminalPaneEvent` 是同一形状：
//! 面板只知道自己有没有会话，不知道底部工具窗是否可见。
//!
//! ## 线程与任务
//!
//! | 任务 | 内容 | 为什么 |
//! | --- | --- | --- |
//! | 取配置 | `workspace.snapshot` + `runConfig.generate` | 两步都是同步读盘 / 遍历，必须离开 UI 线程 |
//! | 起进程 | `runConfig.createLaunchPlan` + 前置步骤（`javac`）+ `spawn` | 前置步骤要等完成，最长几分钟 |
//! | 泵 | `background_spawn(blocking_recv)` 的循环（借 `terminal/src/session.rs` 的形状） | 每来一块输出才醒一次，不轮询、不阻塞 UI |
//!
//! 三者都在 `.update(..)` 的失败分支上静默收工（gpui 的实体没了就是 `Err`）：
//! 面板比任务活得短的正常情形不该 panic。

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
    Window, div, rems,
};

use lithe_gpui_shared::icons::{idea, idea_icon_mono};
use lithe_gpui_shared::{core_json, tr, tr_args};

use crate::model::{RunConfigurationView, load};
use crate::plan::ToolchainResolver;
use crate::session::{RunSession, RunState};

/// 面板向宿主播报的两件事（宿主拿它做**与面板内容无关的布局反应**）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunPaneEvent {
    /// 一次运行开始了：外壳该把底部工具窗切到「运行」并显示。
    Started,
    /// 一次运行结束了（正常退出 / 失败 / 被停止）。
    Finished,
}

/// 一次启动请求的结果（后台任务 → 前台的载荷）。
struct LaunchOutcome {
    root: PathBuf,
    id: String,
    name: String,
    plan: serde_json::Value,
    debug_port: Option<u16>,
}

/// 「运行」工具窗的实体。
pub struct RunPane {
    /// 工作区根（`runConfig.*` 的 `root`，也是启动计划里相对路径的基准）。
    root: Option<PathBuf>,
    /// 「工具链在哪」的解析器，由外壳登记（见 `crate::lib` 的模块文档）。
    resolve: Option<ToolchainResolver>,
    /// 识别到的配置（顺序照 Core 给的，不重排）。
    configurations: Vec<RunConfigurationView>,
    /// 当前选中的配置下标。
    selected: Option<usize>,
    /// 正在跑（或刚跑完）的那一次会话。
    session: Option<RunSession>,
    /// 输出与状态变化的泵（`Task` drop 即取消，与仓库其它后台任务同一约定）。
    pump: Option<Task<()>>,
    /// 在后台取配置的那次任务。
    load: Option<Task<()>>,
    /// 在后台起进程的那次任务。
    launch: Option<Task<()>>,
    /// 一次性的失败说明（取配置失败 / 起不来 / 前置步骤失败）。画在工具栏下面一行。
    notice: Option<SharedString>,
    /// 这次 `🐞` 用的 JDWP 端口（`None` = 普通运行）。
    debug_port: Option<u16>,
    /// 启动态探针（`--run-probe`）：**识别完一批配置就自动跑第一条**，跑完就清掉。
    ///
    /// 与点击 `▶` 走**同一段代码**（[`RunPane::launch`]），所以它证明的是"真实路径真的能跑"，
    /// 而不是一条只有探针才走的旁路。无人值守的验证里点不动按钮（注入不可靠），
    /// 这个开关是那台机器上唯一能拿到"进程真的起了、输出真的收了、退出码真的记了"的证据。
    probe: bool,
    /// 探针用不用调试方式起（`--run-probe-debug`）。
    probe_debug: bool,
}

impl EventEmitter<RunPaneEvent> for RunPane {}

impl RunPane {
    /// 建一个空的运行面板（还没有工作区，因此还没有配置）。
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

    /// 打开启动态探针（见 [`Self::probe`] 的文档）。外壳在建面板时登记。
    pub fn set_probe(&mut self, probe: bool, debug: bool) {
        self.probe = probe;
        self.probe_debug = debug;
    }

    /// 登记工作区根与工具链解析器，并立刻在后台识别一次配置。
    ///
    /// 由外壳在 `ShellWorkspace::new` 里调（与 `prepare_java` / `register_java_toolchain`
    /// 同一位置）。换项目时再调一次即可 —— 旧会话会被停掉。
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
            // 换根：上一次的会话与列表都不再属于这个工作区。
            self.stop_session(cx);
            self.selected = None;
            self.configurations.clear();
            self.notice = None;
        }
        self.reload(cx);
    }

    /// 当前这一份配置列表（外壳的标题栏下拉要它）。
    pub fn configurations(&self) -> &[RunConfigurationView] {
        &self.configurations
    }

    /// 当前选中的配置（没有列表时是 `None`）。
    pub fn selected(&self) -> Option<&RunConfigurationView> {
        self.selected
            .and_then(|index| self.configurations.get(index))
    }

    /// 这次请求调试、但计划里没有调试参数吗（对本侧的非 JVM 配置按 `🐞` 就是这种情况）。
    ///
    /// 用来决定要不要给一句解释：状态会如实显示「运行中」，而用户按的是 `🐞` ——
    /// 不解释一句，看起来就像那个按钮坏了。
    pub fn debug_without_jdwp(&self) -> bool {
        self.debug_port.is_some()
            && self
                .session
                .as_ref()
                .is_some_and(|session| !session.suspends_for_debugger())
    }

    /// 有没有一条在跑的会话。
    pub fn is_running(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.state().is_live())
    }

    /// 选一条配置（`▶` / `🐞` 跑的就是它）。
    pub fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.configurations.len() {
            self.selected = Some(index);
            cx.notify();
        }
    }

    /// 选中下标（标题栏下拉勾选用）。
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    /// 重新识别一次配置（后台；换根与「重新识别」按钮都走这里）。
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
                        // 选中项按**原来的下标**保留（Core 的顺序是确定的，所以下标稳定）；
                        // 越界（列表变短）或还没选过时落到第一条。
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
                        // 探针：识别完就按真实路径跑第一条（只跑一次，见 `Self::probe`）。
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

    /// 启动选中的配置。`debug` 为真时按调试方式起（JDWP 等调试器接入，见模块文档）。
    ///
    /// 立即返回：取计划（Core 调用）与起进程（含前置步骤）都在后台任务里，结果通过
    /// [`RunPaneEvent`] 与 `cx.notify()` 回到前台。
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

        // 已经有一条在跑：先停掉再起新的（真机是"同一配置重跑"，本轮一次只跑一条）。
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
                cx.background_spawn(async move {
                    launch_plan(&task_root, &task_id, debug_port).map(|plan| LaunchOutcome {
                        root: task_root,
                        id: task_id,
                        name,
                        plan,
                        debug_port,
                    })
                })
                .await
            };

            let _ = this.update(cx, |this, cx| {
                match planned {
                    Ok(outcome) => {
                        match RunSession::start(
                            &outcome.root,
                            &outcome.id,
                            &outcome.name,
                            &outcome.plan,
                            &resolve,
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

    /// 停掉当前会话（没在跑时什么都不做）。
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

    /// 清空输出（留住会话本身：退出码还在，用户只是不想看滚动了一屏的日志）。
    pub fn clear_output(&mut self, cx: &mut Context<Self>) {
        if let Some(session) = self.session.as_ref() {
            session.clear_output();
        }
        cx.notify();
    }

    /// 这次真的跑了什么（`程序 + 参数`）；没有会话时 `None`。
    fn command_line(&self) -> Option<String> {
        self.session
            .as_ref()
            .map(|session| session.command_line().to_string())
    }

    /// 输出行是否被截断过（界面上如实标注）。
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

/// 起一次泵：每来一块输出就唤醒面板重绘，两条流都关了就去收退出码。
///
/// 形状借 `lithe-gpui/crates/terminal/src/session.rs` 的 `spawn_pump`：
/// `std::sync::mpsc` 没有异步接收端，所以把阻塞 `recv()` 放进 `background_spawn`
/// （要求 `Send + 'static`），前台任务 `await` 它的结果。`Mutex<Receiver>` 只是把 `!Sync`
/// 的 `Receiver` 搬进那次 `background_spawn`（同一时刻只有一个等待者）。
fn spawn_pump(receiver: Receiver<()>, cx: &mut Context<RunPane>) -> Task<()> {
    let receiver = Arc::new(Mutex::new(receiver));
    cx.spawn(async move |this, cx| {
        loop {
            let receiver = receiver.clone();
            let event = cx
                .background_spawn(async move { blocking_recv(&receiver) })
                .await;
            // 通道断开（会话被丢弃）就是收工。
            let Some(()) = event else { break };
            let settled = this
                .update(cx, |this, cx| {
                    if let Some(session) = this.session.as_mut() {
                        let settled = session.settle_exit();
                        cx.notify();
                        if settled {
                            // 证据行：退出码 + 收了多少行输出（+ 是否被截断）。
                            // "进程真的起了、输出真的收了、退出码真的记了"这三件事
                            // 在无人值守时只能从这一行读出来。
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

/// 阻塞地在后台线程上取一条唤醒（理由同终端的 `blocking_recv`）。
fn blocking_recv(receiver: &Arc<Mutex<Receiver<()>>>) -> Option<()> {
    let guard = match receiver.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.recv().ok()
}

/// 一次 `runConfig.createLaunchPlan`：拿平台无关的启动计划（`data`）。
///
/// `debug_port` 为真时把它交给 Core —— Core 会给 JVM 加上
/// `-agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address=127.0.0.1:<port>`，
/// 于是进程起来就停在"等调试器接入"那一档（契约 `rust-core-api.md` 的 debug 一段）。
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

    // ⚠️ `core_json` 返回的**已经是信封里的 `data`**（`core_client.rs:214-219`），
    // 不要再 `.get("data")` 一次 —— 那样只会拿到 `null`，报出"响应里没有 data"这种
    // 自己造出来的错（探针实测踩过）。
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

/// Core 失败 → 可排查的一句话（有错误码时带上码原文）。
fn core_error_text(error: &lithe_gpui_shared::CoreError) -> String {
    match error.code() {
        Some(code) => format!("{code}: {error}"),
        None => error.to_string(),
    }
}

/// 让操作系统分一个空闲端口给 JDWP（绑 0 再释放：标准做法，存在极小的竞争窗口）。
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
    /// 工具栏：当前配置名 + 状态 + 四个动作（运行 / 调试 / 停止 / 清空输出）。
    fn toolbar(&self, cx: &Context<Self>) -> AnyElement {
        let running = self.is_running();
        // 显示**这次会话**的配置名（运行期间用户可能又点了列表里别的行，标题不该跟着变）；
        // 还没跑过时显示当前选中项，一个都没有时显示工具窗自己的名字。
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

        // 动作按钮：真机那一排就是这四个（运行 / 调试 / 停止 / 清空）。
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

    /// 状态那一小段文字：`运行中` / `已退出 code=0` / `等待调试器接入（127.0.0.1:port）`。
    fn status_text(&self, _cx: &Context<Self>) -> Option<SharedString> {
        let session = self.session.as_ref()?;
        // `tr_args` 收 `&str`，所以数字要先落到局部变量上（临时值的借用活不到调用结束）。
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

    /// 配置列表：一行一条，点一行选中（选中行有底色 + 主色文字）。
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

    /// 输出控制台：等宽字体、逐行、最后一行贴底（滚动容器让新内容自然在下方）。
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
