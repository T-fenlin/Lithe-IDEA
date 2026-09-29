//! 一次运行会话：起进程、收输出、记退出码、能停。
//!
//! ## 机制为什么与终端那套同形
//!
//! `lithe-gpui/crates/terminal/src/session.rs` 是本仓库唯一一处长期存活子进程的先例：
//! `std::process::Command` + 三个管道 + 每条管道一条读线程 + `std::sync::mpsc`
//! + 前台任务 `background_spawn(async { blocking_recv(..) })` 的泵。那套机制已经被真机跑过，
//! 所以这里**照抄形状**而不是另发明一套（重复造子进程泵只会多一份要调的生命周期）。
//!
//! 与终端的差异只有两条，都是运行场景的语义：
//!
//! 1. **没有 stdin**：运行配置不是交互式 shell，没有输入框（真机的运行控制台也不给输入）；
//! 2. **输出是"一整份文本 + 有界行数"**：终端要按 ANSI 解析成网格，运行只要原样的文本流
//!    （真机的运行输出同样只是 `run-output { sessionId, chunk }` 的文本）。
//!
//! ## 生命周期硬要求：`kill()` 之后必须 `wait()`
//!
//! 与终端同一条（`terminal/src/session.rs` 的 `shutdown` 文档）：只 `kill()` 不回收会在进程表里
//! 留僵尸进程。所以 [`RunSession::stop`] 与 `Drop` 都走同一个 `kill` + `wait`。
//!
//! ## 上限
//!
//! 输出留最后 [`MAX_OUTPUT_LINES`] 行（超出的从头部丢，并在诊断行里报一次 `truncated`）：
//! 一次跑得久的服务（`spring-boot:run` 能刷几万行）不该把宿主的内存吃光。

use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::plan::CommandSpec;

/// 输出保留的行数上限（超出的从头部丢）。
pub const MAX_OUTPUT_LINES: usize = 5_000;
/// 读线程一次读多少字节。
const READ_BUFFER: usize = 4_096;
/// 前置步骤（`javac` 一类）的等待上限：编译大工程可能要几十秒，但不该无限等。
const STEP_TIMEOUT: Duration = Duration::from_secs(300);

/// 一次运行的状态（真机 `debugger.status*` 的那几档，运行侧只用得到前四档）。
///
/// ⚠️ `WaitingForDebugger` 的判据是**计划里真的有 JWDP 参数**（`-agentlib:jdwp=…`），
/// 不是"用户按了 🐞"：只有 JVM 配置（`java.main` / `spring-boot.maven`）的计划才会带它，
/// 一条 `npm` 脚本按 🐞 起来就是普通进程。按意图写状态会说成"在等调试器"而它其实在跑 ——
/// 那是假状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunState {
    /// 起进程之前（含前置步骤）。
    Starting,
    /// 主进程在跑。
    Running,
    /// 已退出（退出码）。
    Exited(i32),
    /// 起不来，或前置步骤失败。存的是给人看的原因。
    Failed(String),
    /// 被用户停止。
    Stopped,
    /// `🐞` 起的那一种：进程已起、JVM 挂在 `-agentlib:jdwp=…,suspend=y` 上等调试器。
    /// **没有** DAP 会话（见 `crate::lib` 的未做清单），所以状态如实描述这件事。
    WaitingForDebugger {
        /// JDWP 监听端口。
        port: u16,
    },
}

impl RunState {
    /// 进程还在跑吗（含"等调试器接入"——那时进程也是活的）。
    pub fn is_live(&self) -> bool {
        matches!(
            self,
            Self::Starting | Self::Running | Self::WaitingForDebugger { .. }
        )
    }

    /// 状态文案的 i18n 键（`debugger.status*` 一族，真源已有的键）。
    pub fn label_key(&self) -> &'static str {
        match self {
            Self::Starting => "lithe.debugger.statusIdle",
            Self::Running => "lithe.debugger.statusRunning",
            Self::Exited(_) => "lithe.debugger.statusExited",
            Self::Failed(_) => "lithe.debugger.statusFailed",
            Self::Stopped => "lithe.debugger.statusStopped",
            Self::WaitingForDebugger { .. } => "lithe.debugger.statusPaused",
        }
    }
}

/// 一次运行的会话：主进程 + 输出 + 状态。
pub struct RunSession {
    /// 子进程。**一直留在这里**，`stop` / `Drop` 才有 `kill()` + `wait()` 的能力。
    child: Option<Child>,
    /// 已经出来的输出（按行切好，头部超出上限的丢掉）。
    output: Arc<Mutex<Vec<String>>>,
    /// 输出的行数是否被截断过（诊断行 + 界面上如实标注）。
    truncated: Arc<Mutex<bool>>,
    /// 泵侧收到的唤醒。`Option`：起完之后由面板取走交给泵（`Receiver` 不可克隆）。
    receiver: Option<Receiver<()>>,
    state: RunState,
    /// 这次运行的那条配置（诊断行与面板标题用）。
    pub(crate) configuration_id: String,
    pub(crate) configuration_name: String,
    /// 计划里那条命令行（诊断行用，便于核对"到底跑了什么"）。
    pub(crate) command_line: String,
    /// 计划里带没带 JWDP 参数（见 [`RunState::WaitingForDebugger`] 的文档）。
    suspends_for_debugger: bool,
}

impl RunSession {
    /// 按启动计划起进程：先跑完前置步骤，再起主进程。
    ///
    /// `debug_port` 是 `Some` 时状态进入 [`RunState::WaitingForDebugger`]
    /// （Core 已经在计划里给 JVM 加了 `suspend=y`，所以进程起来就停在那里等接入）。
    ///
    /// **阻塞**（前置步骤要等完成）：调用方必须放进 `background_spawn`。
    pub fn start(
        root: &Path,
        configuration_id: &str,
        configuration_name: &str,
        plan: &serde_json::Value,
        resolve: &crate::plan::ToolchainResolver,
        debug_port: Option<u16>,
    ) -> Result<Self, String> {
        let (steps, main) =
            crate::plan::launch_command(root, plan, resolve).map_err(|error| error.0)?;
        let command_line = describe(&main);
        // 计划里带没带 JWDP 参数（Core 只在 `debugPort` 存在且是 JVM 配置时前置
        // `-agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address=127.0.0.1:<port>`）。
        let suspends_for_debugger = main
            .args
            .iter()
            .any(|argument| argument.starts_with("-agentlib:jdwp"));

        // 前置步骤：按序跑完，任一非零退出就中止（契约明说这是宿主的责任）。
        for step in &steps {
            run_step(step)?;
        }

        let mut command = Command::new(&main.program);
        command
            .args(&main.args)
            .current_dir(&main.cwd)
            // 运行配置**不给输入**：不是交互式 shell，没有输入框（真机的运行控制台也不给）。
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in &main.env {
            command.env(key, value);
        }

        let mut child = command
            .spawn()
            .map_err(|error| format!("{}: {error}", main.program.display()))?;

        let output = Arc::new(Mutex::new(Vec::<String>::new()));
        let truncated = Arc::new(Mutex::new(false));
        // 发送端只交给两条读线程：它们都结束后通道自行关闭，泵的 `recv` 随之返回 `None`——
        // 会话自己**不留**一份，否则通道永远不会关，泵要靠"最后一条唤醒"才发现进程没了。
        let (signal, receiver) = mpsc::channel::<()>();
        if let Some(stdout) = child.stdout.take() {
            spawn_reader(stdout, output.clone(), truncated.clone(), signal.clone());
        }
        if let Some(stderr) = child.stderr.take() {
            spawn_reader(stderr, output.clone(), truncated.clone(), signal.clone());
        }

        println!(
            "S1_RUN_START id={configuration_id} name={configuration_name} pid={} debug_port={} jdwp={} command={command_line}",
            child.id(),
            debug_port
                .map(|port| port.to_string())
                .unwrap_or_else(|| "-".to_string()),
            if suspends_for_debugger {
                "suspended"
            } else {
                "none"
            },
        );

        Ok(Self {
            child: Some(child),
            output,
            truncated,
            receiver: Some(receiver),
            state: match (debug_port, suspends_for_debugger) {
                (Some(port), true) => RunState::WaitingForDebugger { port },
                _ => RunState::Running,
            },
            configuration_id: configuration_id.to_string(),
            configuration_name: configuration_name.to_string(),
            command_line,
            suspends_for_debugger,
        })
    }

    /// 把唤醒通道交给泵（只有一次：`Receiver` 不可克隆）。
    pub fn take_receiver(&mut self) -> Option<Receiver<()>> {
        self.receiver.take()
    }

    /// 清空输出（留住会话本身：退出码与状态还在，用户只是不想看滚了一屏的日志）。
    pub fn clear_output(&self) {
        lock(&self.output).clear();
        *lock(&self.truncated) = false;
    }

    /// 这次运行的进程是不是"以调试方式"起的（计划里真的有 JWDP 参数）。
    pub fn suspends_for_debugger(&self) -> bool {
        self.suspends_for_debugger
    }

    /// 这次运行的配置 id（诊断行用它，可 grep 到具体是哪条配置）。
    pub fn configuration_id(&self) -> &str {
        &self.configuration_id
    }

    /// 这次运行的配置名（面板标题用它，比"当前选中项"更准：运行期间用户可能又点了别的行）。
    pub fn configuration_name(&self) -> &str {
        &self.configuration_name
    }

    /// 这次真的跑了什么（`程序 + 参数` 一行；面板上如实显示，便于核对）。
    pub fn command_line(&self) -> &str {
        &self.command_line
    }

    /// 现在这一份输出（整体替换语义：调用方拿它直接画）。
    pub fn output_lines(&self) -> Vec<String> {
        lock(&self.output).clone()
    }

    /// 输出是否被截断过（超过 [`MAX_OUTPUT_LINES`]）。
    pub fn output_truncated(&self) -> bool {
        *lock(&self.truncated)
    }

    /// 当前状态。
    pub fn state(&self) -> &RunState {
        &self.state
    }

    /// 进程退出码（没退出是 `None`）。
    pub fn exit_code(&self) -> Option<i32> {
        match self.state {
            RunState::Exited(code) => Some(code),
            _ => None,
        }
    }

    /// 如果进程已经退出，把状态推进到 `Exited`。
    ///
    /// 返回 `true` 表示状态已确定（泵可以收工）。**从管道 EOF 走到这里**是主要路径：
    /// 两条流都关了说明进程基本已经结束（与终端的 `settle_exit` 同一判据）。
    pub(crate) fn settle_exit(&mut self) -> bool {
        if !matches!(
            self.state,
            RunState::Running | RunState::WaitingForDebugger { .. }
        ) {
            return true;
        }
        let Some(child) = self.child.as_mut() else {
            return true;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.state = RunState::Exited(status.code().unwrap_or(-1));
                true
            }
            Ok(None) => false,
            Err(error) => {
                self.state = RunState::Failed(error.to_string());
                true
            }
        }
    }

    /// 用户点了「停止」：先杀进程再回收（`wait()` 不能省，否则留下僵尸进程）。
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let code = child.wait().ok().and_then(|status| status.code());
            self.state = code.map(RunState::Exited).unwrap_or(RunState::Stopped);
            println!(
                "S1_RUN_STOP id={} code={}",
                self.configuration_id,
                match self.state {
                    RunState::Exited(code) => code.to_string(),
                    _ => "-".to_string(),
                },
            );
        } else if matches!(self.state, RunState::Starting) {
            self.state = RunState::Stopped;
        }
    }
}

impl Drop for RunSession {
    fn drop(&mut self) {
        // 兜底：面板销毁 / 换项目时也要把子进程收干净（与终端 `Drop` 同一条要求）。
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// 命令行的一句话描述（诊断行用）：程序 + 参数，参数里的空格加引号便于人读。
fn describe(spec: &CommandSpec) -> String {
    let mut parts = vec![spec.program.display().to_string()];
    parts.extend(spec.args.iter().map(|argument| {
        if argument.contains(' ') {
            format!("\"{argument}\"")
        } else {
            argument.clone()
        }
    }));
    parts.join(" ")
}

/// 跑一个前置步骤：等它结束，非零退出就中止整次运行（原因带上退出码）。
fn run_step(step: &CommandSpec) -> Result<(), String> {
    let mut command = Command::new(&step.program);
    command
        .args(&step.args)
        .current_dir(&step.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in &step.env {
        command.env(key, value);
    }
    let child = command
        .spawn()
        .map_err(|error| format!("前置步骤 {} 起不来：{error}", step.program.display()))?;
    let started = Instant::now();
    let mut child = child;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    println!(
                        "S1_RUN_STEP program={} ok=true ms={}",
                        step.program.display(),
                        started.elapsed().as_millis()
                    );
                    return Ok(());
                }
                let detail = collect_failure(&mut child);
                println!(
                    "S1_RUN_STEP program={} ok=false code={:?} {}",
                    step.program.display(),
                    status.code(),
                    detail.replace('\n', " ⏎ ")
                );
                return Err(format!(
                    "前置步骤失败（{}）：{detail}",
                    step.program.display()
                ));
            }
            Ok(None) => {
                if started.elapsed() > STEP_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "前置步骤超时（{} 秒）：{}",
                        STEP_TIMEOUT.as_secs(),
                        step.program.display()
                    ));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(format!("前置步骤等待失败：{error}")),
        }
    }
}

/// 前置步骤失败时把它的 stderr 捞出来（那才是用户要看的：编译错误的原文）。
fn collect_failure(child: &mut Child) -> String {
    let mut text = String::new();
    if let Some(stderr) = child.stderr.as_mut() {
        let mut buffer = [0u8; READ_BUFFER];
        while let Ok(read) = stderr.read(&mut buffer) {
            if read == 0 {
                break;
            }
            text.push_str(&String::from_utf8_lossy(&buffer[..read]));
            if text.len() > 4_000 {
                break;
            }
        }
    }
    let trimmed = text.trim();
    if trimmed.is_empty() {
        "没有输出".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 一条管道读线程：把字节按**行**追加进输出缓冲，超上限时从头部丢。
///
/// 按行切而不是按字节存：面板要显示的是文本行，且"最后 N 行"这个上限天然以行为单位
/// （真机的运行输出也是逐行渲染的）。
fn spawn_reader<R: Read + Send + 'static>(
    mut reader: R,
    output: Arc<Mutex<Vec<String>>>,
    truncated: Arc<Mutex<bool>>,
    signal: Sender<()>,
) {
    std::thread::spawn(move || {
        let mut buffer = [0u8; READ_BUFFER];
        let mut pending = String::new();
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    pending.push_str(&String::from_utf8_lossy(&buffer[..read]));
                    // 只把完整的行切出去，半个行留在 `pending` 里等下一次读：
                    // 否则同一个逻辑行会被拆成两条，界面上看起来像两行输出。
                    let mut lines: Vec<String> = Vec::new();
                    while let Some(index) = pending.find('\n') {
                        let line = pending[..index].trim_end_matches('\r').to_string();
                        lines.push(line);
                        pending.drain(..=index);
                    }
                    if !lines.is_empty() {
                        {
                            let mut output = lock(&output);
                            output.extend(lines);
                            if output.len() > MAX_OUTPUT_LINES {
                                let excess = output.len() - MAX_OUTPUT_LINES;
                                output.drain(..excess);
                                *lock(&truncated) = true;
                            }
                        }
                        // `send` 失败 = 面板已经不在了，收工（不是错误）。
                        if signal.send(()).is_err() {
                            break;
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        // 结尾那半行（没有换行的最后一行）也要出去，否则最后一条输出会丢。
        if !pending.is_empty() {
            let mut output = lock(&output);
            output.push(pending);
        }
        // 流关闭也是一次唤醒：泵要据此去收退出码。
        let _ = signal.send(());
    });
}

/// 走上限保护地借出锁（poisoned 也照用：一个读线程 panic 不该让整个面板失效）。
fn lock<T>(value: &Arc<Mutex<T>>) -> std::sync::MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
