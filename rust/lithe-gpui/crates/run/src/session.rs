use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::plan::CommandSpec;

pub const MAX_OUTPUT_LINES: usize = 5_000;
const READ_BUFFER: usize = 4_096;
const STEP_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunState {
    Starting,
    Running,
    Exited(i32),
    Failed(String),
    Stopped,
    WaitingForDebugger { port: u16 },
}

impl RunState {
    pub fn is_live(&self) -> bool {
        matches!(
            self,
            Self::Starting | Self::Running | Self::WaitingForDebugger { .. }
        )
    }

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

pub struct RunSession {
    child: Option<Child>,
    output: Arc<Mutex<Vec<String>>>,
    truncated: Arc<Mutex<bool>>,
    receiver: Option<Receiver<()>>,
    state: RunState,
    pub(crate) configuration_id: String,
    pub(crate) configuration_name: String,
    pub(crate) command_line: String,
    suspends_for_debugger: bool,
}

pub struct PreparedStart {
    pub spec: CommandSpec,
    pub command_line: String,
}

pub fn prepare(
    root: &Path,
    plan: &Value,
    resolve: &crate::plan::ToolchainResolver,
) -> Result<PreparedStart, String> {
    let (steps, main) =
        crate::plan::launch_command(root, plan, resolve).map_err(|error| error.0)?;
    let command_line = describe(&main);
    run_steps(&steps)?;
    Ok(PreparedStart {
        spec: main,
        command_line,
    })
}

pub fn run_steps(steps: &[CommandSpec]) -> Result<(), String> {
    for step in steps {
        run_step(step)?;
    }
    Ok(())
}

impl RunSession {
    pub fn start(
        configuration_id: &str,
        configuration_name: &str,
        prepared: PreparedStart,
        debug_port: Option<u16>,
    ) -> Result<Self, String> {
        let PreparedStart { spec, command_line } = prepared;
        let main = spec;
        let suspends_for_debugger = main
            .args
            .iter()
            .any(|argument| argument.starts_with("-agentlib:jdwp"));

        let mut command = Command::new(&main.program);
        command
            .args(&main.args)
            .current_dir(&main.cwd)
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

    pub fn take_receiver(&mut self) -> Option<Receiver<()>> {
        self.receiver.take()
    }

    pub fn clear_output(&self) {
        lock(&self.output).clear();
        *lock(&self.truncated) = false;
    }

    pub fn suspends_for_debugger(&self) -> bool {
        self.suspends_for_debugger
    }

    pub fn configuration_id(&self) -> &str {
        &self.configuration_id
    }

    pub fn configuration_name(&self) -> &str {
        &self.configuration_name
    }

    pub fn command_line(&self) -> &str {
        &self.command_line
    }

    pub fn output_lines(&self) -> Vec<String> {
        lock(&self.output).clone()
    }

    pub fn output_truncated(&self) -> bool {
        *lock(&self.truncated)
    }

    pub fn state(&self) -> &RunState {
        &self.state
    }

    pub fn exit_code(&self) -> Option<i32> {
        match self.state {
            RunState::Exited(code) => Some(code),
            _ => None,
        }
    }

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
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

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
                        if signal.send(()).is_err() {
                            break;
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        if !pending.is_empty() {
            let mut output = lock(&output);
            output.push(pending);
        }
        let _ = signal.send(());
    });
}

fn lock<T>(value: &Arc<Mutex<T>>) -> std::sync::MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
