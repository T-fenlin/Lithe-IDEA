use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use lithe_gpui_shared::{cancel_operation, core_json};
use serde_json::{Value, json};

pub const DEBUG_TRANSPORT_TAG: &str = "S1_DEBUG_TRANSPORT";

const CREATE_SESSION_COMMAND: &str = "debug.createSession";
const LAUNCH_COMMAND: &str = "debug.launch";
const RECEIVE_COMMAND: &str = "debug.receive";
const EXECUTE_COMMAND: &str = "debug.execute";
const INSPECT_COMMAND: &str = "debug.inspect";
const DISCONNECT_COMMAND: &str = "debug.disconnect";
const DESTROY_SESSION_COMMAND: &str = "debug.destroySession";

const STDERR_TAIL_BYTES: usize = 4096;

const LSP_REQUEST_COMMAND: &str = "lsp.request";
const EXECUTE_COMMAND_OPERATION: &str = "executeCommand";
const JAVA_START_DEBUG_SESSION: &str = "vscode.java.startDebugSession";

pub fn java_start_debug_session_payload(session_id: &str, operation_id: &str) -> Value {
    json!({
        "sessionId": session_id,
        "operationId": operation_id,
        "operation": EXECUTE_COMMAND_OPERATION,
        "command": { "command": JAVA_START_DEBUG_SESSION },
    })
}

pub fn parse_debug_port(value: &Value) -> Option<u16> {
    match value {
        Value::Number(number) => number.as_u64().and_then(valid_port),
        Value::String(text) => text
            .trim()
            .parse::<u64>()
            .ok()
            .and_then(valid_port)
            .or_else(|| {
                serde_json::from_str::<Value>(text)
                    .ok()
                    .and_then(|inner| parse_debug_port(&inner))
            }),
        Value::Object(map) => {
            for key in ["value", "port", "debugPort", "debug_port"] {
                if let Some(found) = map.get(key).and_then(parse_debug_port) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(parse_debug_port),
        Value::Null | Value::Bool(_) => None,
    }
}

fn valid_port(port: u64) -> Option<u16> {
    u16::try_from(port).ok().filter(|port| *port > 0)
}

pub fn java_debug_session_port(
    session_id: &str,
    operation_id: &str,
    timeout: Duration,
) -> Result<u16, String> {
    let payload = java_start_debug_session_payload(session_id, operation_id);
    let data = match core_json(LSP_REQUEST_COMMAND, payload) {
        Ok(Some(data)) => data,
        Ok(None) => {
            return Err(format!(
                "{DEBUG_TRANSPORT_TAG} start_debug_session_no_data session={session_id}"
            ));
        }
        Err(error) => {
            return Err(format!(
                "{DEBUG_TRANSPORT_TAG} start_debug_session_failed session={session_id} error={error}"
            ));
        }
    };
    parse_debug_port(&data).ok_or_else(|| {
        format!(
            "{DEBUG_TRANSPORT_TAG} start_debug_session_no_port session={session_id} data={data}"
        )
    })
    .map(|port| {
        println!(
            "{DEBUG_TRANSPORT_TAG} start_debug_session session={session_id} port={port} timeout_ms={}",
            timeout.as_millis()
        );
        port
    })
}

pub fn connect_java_debug_session(
    session_id: &str,
    operation_id: &str,
    timeout: Duration,
) -> Result<TcpDebugAdapter, String> {
    let port = java_debug_session_port(session_id, operation_id, timeout)?;
    TcpDebugAdapter::connect(&format!("127.0.0.1:{port}"), timeout)
}

pub fn start_tcp_session(
    session_id: &str,
    root_path: &str,
    port: u16,
    connect_timeout: Duration,
) -> Result<DebugTransport, String> {
    let adapter = TcpDebugAdapter::connect(&format!("127.0.0.1:{port}"), connect_timeout)?;
    DebugTransport::start(session_id, "java", root_path, false, Box::new(adapter))
}

pub fn start_java_session(
    session_id: &str,
    operation_id: &str,
    lsp_session_id: &str,
    root_path: &str,
    timeout: Duration,
) -> Result<DebugTransport, String> {
    let port = java_debug_session_port(lsp_session_id, operation_id, timeout)?;
    start_tcp_session(session_id, root_path, port, timeout)
}

pub struct DebugAdapterSpec {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: Vec<(String, String)>,
}

pub trait DebugAdapterPort: Send {
    fn write_frame(&mut self, frame: &[u8]) -> Result<(), String>;

    fn try_recv(&mut self) -> Option<Vec<u8>> {
        None
    }

    fn kill(&mut self);
}

pub struct StdioDebugAdapter {
    child: Child,
    stdin: Option<ChildStdin>,
    chunks: Receiver<Vec<u8>>,
    stderr_tail: std::sync::Arc<std::sync::Mutex<String>>,
    exited: bool,
}

impl StdioDebugAdapter {
    pub fn spawn(spec: &DebugAdapterSpec) -> Result<Self, String> {
        let mut command = Command::new(&spec.executable);
        command
            .args(&spec.arguments)
            .current_dir(&spec.working_directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in &spec.environment {
            command.env(key, value);
        }
        let mut child = command.spawn().map_err(|error| {
            format!(
                "{DEBUG_TRANSPORT_TAG} spawn_failed executable={} error={error}",
                spec.executable.display()
            )
        })?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (sender, chunks) = channel();
        if let Some(mut stdout) = stdout {
            std::thread::spawn(move || {
                let mut buffer = [0_u8; 8192];
                loop {
                    match stdout.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => {
                            if sender.send(buffer[..read].to_vec()).is_err() {
                                break;
                            }
                        }
                    }
                }
            });
        }
        let stderr_tail = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        if let Some(mut stderr) = stderr {
            let tail = stderr_tail.clone();
            std::thread::spawn(move || {
                let mut text = String::new();
                let _ = stderr.read_to_string(&mut text);
                if let Ok(mut guard) = tail.lock() {
                    let start = text.len().saturating_sub(STDERR_TAIL_BYTES);
                    *guard = text[start..].to_string();
                }
            });
        }

        println!(
            "{DEBUG_TRANSPORT_TAG} adapter_spawned executable={} pid={}",
            spec.executable.display(),
            child.id()
        );
        Ok(Self {
            child,
            stdin,
            chunks,
            stderr_tail,
            exited: false,
        })
    }

    #[must_use]
    pub fn process_id(&self) -> u32 {
        self.child.id()
    }

    #[must_use]
    pub fn stderr_tail(&self) -> String {
        self.stderr_tail
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn try_recv(&self) -> Option<Vec<u8>> {
        match self.chunks.try_recv() {
            Ok(chunk) => Some(chunk),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }

    pub fn recv_bounded(&self, timeout: Duration) -> Option<Vec<u8>> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(chunk) = self.try_recv() {
                return Some(chunk);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn wait_exit(&mut self, timeout: Duration) -> Option<i32> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.exited = true;
                    return status.code();
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Ok(None) => return None,
                Err(error) => {
                    eprintln!("{DEBUG_TRANSPORT_TAG} wait_failed error={error}");
                    return None;
                }
            }
        }
    }

    #[must_use]
    pub fn is_exited(&self) -> bool {
        self.exited
    }
}

impl DebugAdapterPort for StdioDebugAdapter {
    fn write_frame(&mut self, frame: &[u8]) -> Result<(), String> {
        let Some(stdin) = self.stdin.as_mut() else {
            return Err(format!(
                "{DEBUG_TRANSPORT_TAG} write_failed reason=stdin_closed pid={}",
                self.child.id()
            ));
        };
        stdin.write_all(frame).map_err(|error| {
            format!(
                "{DEBUG_TRANSPORT_TAG} write_failed pid={} error={error}",
                self.child.id()
            )
        })?;
        stdin.flush().map_err(|error| {
            format!(
                "{DEBUG_TRANSPORT_TAG} flush_failed pid={} error={error}",
                self.child.id()
            )
        })
    }

    fn try_recv(&mut self) -> Option<Vec<u8>> {
        StdioDebugAdapter::try_recv(self)
    }

    fn kill(&mut self) {
        self.stdin = None;
        if !self.exited {
            match self.child.kill() {
                Ok(()) => {
                    let _ = self.child.wait();
                    self.exited = true;
                    println!(
                        "{DEBUG_TRANSPORT_TAG} adapter_killed pid={}",
                        self.child.id()
                    );
                }
                Err(error) => {
                    eprintln!(
                        "{DEBUG_TRANSPORT_TAG} adapter_kill_failed pid={} error={error}",
                        self.child.id()
                    );
                }
            }
        }
    }
}

impl Drop for StdioDebugAdapter {
    fn drop(&mut self) {
        self.kill();
    }
}

pub fn stdio_spec(
    executable: impl Into<PathBuf>,
    arguments: impl IntoIterator<Item = impl Into<String>>,
    working_directory: impl AsRef<Path>,
) -> DebugAdapterSpec {
    DebugAdapterSpec {
        executable: executable.into(),
        arguments: arguments.into_iter().map(Into::into).collect(),
        working_directory: working_directory.as_ref().to_path_buf(),
        environment: Vec::new(),
    }
}

pub fn java_launch_configuration(name: &str, java_launch: &Value) -> Result<Value, String> {
    let main_class = java_launch
        .get("mainClass")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            format!("{DEBUG_TRANSPORT_TAG} launch_configuration_missing_main_class name={name}")
        })?;
    let mut arguments = serde_json::Map::new();
    arguments.insert("type".to_string(), json!("java"));
    arguments.insert("mainClass".to_string(), json!(main_class));
    for key in ["projectName", "classPaths", "modulePaths", "vmArgs", "cwd"] {
        if let Some(value) = java_launch.get(key) {
            arguments.insert(key.to_string(), value.clone());
        }
    }
    Ok(json!({
        "name": name,
        "request": "launch",
        "arguments": Value::Object(arguments),
    }))
}

pub fn java_attach_configuration(
    name: &str,
    port: u16,
    project_name: Option<&str>,
) -> Result<Value, String> {
    if port == 0 {
        return Err(format!(
            "{DEBUG_TRANSPORT_TAG} attach_configuration_invalid_port name={name} port=0"
        ));
    }
    let mut arguments = serde_json::Map::new();
    arguments.insert("type".to_string(), json!("java"));
    arguments.insert("hostName".to_string(), json!("127.0.0.1"));
    arguments.insert("port".to_string(), json!(port));
    if let Some(project_name) = project_name.filter(|value| !value.trim().is_empty()) {
        arguments.insert("projectName".to_string(), json!(project_name));
    }
    Ok(json!({
        "name": name,
        "request": "attach",
        "arguments": Value::Object(arguments),
    }))
}

pub struct TcpDebugAdapter {
    stream: std::net::TcpStream,
    chunks: Receiver<Vec<u8>>,
    child: Option<Child>,
    exited: bool,
}

impl TcpDebugAdapter {
    pub fn connect(address: &str, timeout: Duration) -> Result<Self, String> {
        let socket: std::net::SocketAddr = address.parse().map_err(|error| {
            format!("{DEBUG_TRANSPORT_TAG} bad_address address={address} error={error}")
        })?;
        let stream = std::net::TcpStream::connect_timeout(&socket, timeout).map_err(|error| {
            format!("{DEBUG_TRANSPORT_TAG} connect_failed address={address} error={error}")
        })?;
        let read_half = stream.try_clone().map_err(|error| {
            format!("{DEBUG_TRANSPORT_TAG} clone_failed address={address} error={error}")
        })?;
        let (sender, chunks) = channel();
        std::thread::spawn(move || {
            let mut half = read_half;
            let mut buffer = [0_u8; 8192];
            loop {
                match half.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => {
                        if sender.send(buffer[..read].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        println!("{DEBUG_TRANSPORT_TAG} adapter_connected address={address}");
        Ok(Self {
            stream,
            chunks,
            child: None,
            exited: false,
        })
    }

    #[must_use]
    pub fn is_exited(&self) -> bool {
        self.exited
    }

    pub fn try_recv(&self) -> Option<Vec<u8>> {
        match self.chunks.try_recv() {
            Ok(chunk) => Some(chunk),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }

    pub fn recv_bounded(&self, timeout: Duration) -> Option<Vec<u8>> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(chunk) = self.try_recv() {
                return Some(chunk);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl DebugAdapterPort for TcpDebugAdapter {
    fn write_frame(&mut self, frame: &[u8]) -> Result<(), String> {
        self.stream.write_all(frame).map_err(|error| {
            format!("{DEBUG_TRANSPORT_TAG} write_failed transport=tcp error={error}")
        })?;
        self.stream.flush().map_err(|error| {
            format!("{DEBUG_TRANSPORT_TAG} flush_failed transport=tcp error={error}")
        })
    }

    fn try_recv(&mut self) -> Option<Vec<u8>> {
        TcpDebugAdapter::try_recv(self)
    }

    fn kill(&mut self) {
        if self.exited {
            return;
        }
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
        self.exited = true;
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        println!("{DEBUG_TRANSPORT_TAG} adapter_killed transport=tcp");
    }
}

impl Drop for TcpDebugAdapter {
    fn drop(&mut self) {
        self.kill();
    }
}

pub enum DebugDriverEvent {
    Events(Vec<Value>),
    Exited { reason: String },
}

pub enum DebugDriverCommand {
    Execute {
        command: String,
        thread_id: Option<i64>,
        timeout: Duration,
        reply: std::sync::mpsc::Sender<Result<Option<Value>, String>>,
    },
    Inspect {
        kind: String,
        thread_id: Option<i64>,
        frame_id: Option<i64>,
        variables_reference: Option<i64>,
        timeout: Duration,
        reply: std::sync::mpsc::Sender<Result<Option<Value>, String>>,
    },
}

pub const DEBUG_REPLY_TIMEOUT: Duration = Duration::from_secs(5);

pub fn is_reply_event(event: &Value) -> bool {
    matches!(
        event.get("type").and_then(Value::as_str),
        Some("stopped" | "continued" | "terminated" | "operationFailed")
    )
}

pub fn is_inspection_reply(event: &Value) -> bool {
    matches!(
        event.get("type").and_then(Value::as_str),
        Some("operationCompleted" | "operationFailed")
    )
}

pub struct DebugSessionDriver {
    stop: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
    events: Receiver<DebugDriverEvent>,
    commands: std::sync::mpsc::Sender<DebugDriverCommand>,
}

impl DebugSessionDriver {
    pub fn spawn(mut transport: DebugTransport, poll_interval: Duration) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let running = Arc::new(AtomicBool::new(true));
        let (sender, events) = channel();
        let (commands, command_queue) = channel();
        let stop_flag = stop.clone();
        let running_flag = running.clone();
        println!(
            "{DEBUG_TRANSPORT_TAG} driver_started session={} poll_ms={}",
            transport.session_id(),
            poll_interval.as_millis()
        );
        let handle = std::thread::spawn(move || {
            let mut reason = String::from("terminated");
            let mut pending: Option<(
                String,
                bool,
                std::time::Instant,
                std::sync::mpsc::Sender<Result<Option<Value>, String>>,
            )> = None;
            while !stop_flag.load(Ordering::Relaxed) && !transport.is_terminated() {
                if pending.is_none() {
                    if let Ok(command) = command_queue.try_recv() {
                        match command {
                            DebugDriverCommand::Execute {
                                command,
                                thread_id,
                                timeout,
                                reply,
                            } => {
                                let operation_id = transport.take_operation_id(&command);
                                match transport.execute_with_operation(
                                    &operation_id,
                                    &command,
                                    thread_id,
                                ) {
                                    Ok(immediate) => {
                                        if !immediate.is_empty()
                                            && sender
                                                .send(DebugDriverEvent::Events(immediate))
                                                .is_err()
                                        {
                                            reason = String::from("consumer_closed");
                                            break;
                                        }
                                        pending = Some((
                                            operation_id,
                                            false,
                                            Instant::now() + timeout,
                                            reply,
                                        ));
                                    }
                                    Err(error) => {
                                        eprintln!(
                                            "{DEBUG_TRANSPORT_TAG} execute_failed command={command} session={} error={error}",
                                            transport.session_id()
                                        );
                                        let _ = reply.send(Err(error));
                                    }
                                }
                            }
                            DebugDriverCommand::Inspect {
                                kind,
                                thread_id,
                                frame_id,
                                variables_reference,
                                timeout,
                                reply,
                            } => {
                                let operation_id = transport.take_operation_id(&kind);
                                match transport.inspect_with_operation(
                                    &operation_id,
                                    &kind,
                                    thread_id,
                                    frame_id,
                                    variables_reference,
                                ) {
                                    Ok(immediate) => {
                                        if !immediate.is_empty()
                                            && sender
                                                .send(DebugDriverEvent::Events(immediate))
                                                .is_err()
                                        {
                                            reason = String::from("consumer_closed");
                                            break;
                                        }
                                        pending = Some((
                                            operation_id,
                                            true,
                                            Instant::now() + timeout,
                                            reply,
                                        ));
                                    }
                                    Err(error) => {
                                        eprintln!(
                                            "{DEBUG_TRANSPORT_TAG} inspect_failed kind={kind} session={} error={error}",
                                            transport.session_id()
                                        );
                                        let _ = reply.send(Err(error));
                                    }
                                }
                            }
                        }
                    }
                }

                let outcome = match pending.take() {
                    Some((command, inspection, deadline, reply)) => {
                        let remaining = deadline.saturating_duration_since(Instant::now());
                        let slice = std::cmp::min(Duration::from_millis(20), remaining);
                        let matches: fn(&Value) -> bool = if inspection {
                            is_inspection_reply
                        } else {
                            is_reply_event
                        };
                        let waited = transport.poll_until(slice, &matches);
                        match waited {
                            Ok((events, matched)) => {
                                if !events.is_empty()
                                    && sender.send(DebugDriverEvent::Events(events)).is_err()
                                {
                                    reason = String::from("consumer_closed");
                                    break;
                                }
                                if matched {
                                    let _ = reply.send(Ok(None));
                                } else if Instant::now() >= deadline {
                                    transport.cancel(&command);
                                    let _ = reply.send(Err(format!(
                                        "{DEBUG_TRANSPORT_TAG} command_timeout session={} operation={command}",
                                        transport.session_id()
                                    )));
                                } else {
                                    pending = Some((command, inspection, deadline, reply));
                                }
                                continue;
                            }
                            Err(error) => {
                                let _ = reply.send(Err(error.clone()));
                                reason = error;
                                break;
                            }
                        }
                    }
                    None => transport.poll_bounded(poll_interval),
                };
                match outcome {
                    Ok(new_events) => {
                        if !new_events.is_empty()
                            && sender.send(DebugDriverEvent::Events(new_events)).is_err()
                        {
                            reason = String::from("consumer_closed");
                            break;
                        }
                    }
                    Err(error) => {
                        reason = error;
                        break;
                    }
                }
            }
            if stop_flag.load(Ordering::Relaxed) {
                reason = String::from("stopped");
            }
            if !transport.is_terminated() {
                let _ = transport.disconnect();
            }
            let _ = transport.destroy();
            running_flag.store(false, Ordering::Relaxed);
            println!("{DEBUG_TRANSPORT_TAG} driver_exited reason={reason}");
            let _ = sender.send(DebugDriverEvent::Exited { reason });
        });
        Self {
            stop,
            running,
            handle: Some(handle),
            events,
            commands,
        }
    }

    pub fn execute(
        &self,
        command: &str,
        thread_id: Option<i64>,
        timeout: Duration,
    ) -> Result<(), String> {
        let (reply, answer) = channel();
        self.commands
            .send(DebugDriverCommand::Execute {
                command: command.to_string(),
                thread_id,
                timeout,
                reply,
            })
            .map_err(|error| {
                format!("{DEBUG_TRANSPORT_TAG} command_send_failed command={command} error={error}")
            })?;
        match answer.recv_timeout(timeout + DEBUG_REPLY_TIMEOUT) {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(error)) => Err(error),
            Err(error) => Err(format!(
                "{DEBUG_TRANSPORT_TAG} command_reply_failed command={command} error={error}"
            )),
        }
    }

    pub fn request(
        &self,
        command: &str,
        thread_id: Option<i64>,
        timeout: Duration,
    ) -> Result<(), String> {
        let (reply, _answer) = channel();
        self.commands
            .send(DebugDriverCommand::Execute {
                command: command.to_string(),
                thread_id,
                timeout,
                reply,
            })
            .map_err(|error| {
                format!("{DEBUG_TRANSPORT_TAG} command_send_failed command={command} error={error}")
            })
    }

    pub fn request_inspect(
        &self,
        kind: &str,
        thread_id: Option<i64>,
        frame_id: Option<i64>,
        variables_reference: Option<i64>,
        timeout: Duration,
    ) -> Result<(), String> {
        let (reply, _answer) = channel();
        self.commands
            .send(DebugDriverCommand::Inspect {
                kind: kind.to_string(),
                thread_id,
                frame_id,
                variables_reference,
                timeout,
                reply,
            })
            .map_err(|error| {
                format!("{DEBUG_TRANSPORT_TAG} command_send_failed kind={kind} error={error}")
            })
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn try_recv(&self) -> Option<DebugDriverEvent> {
        self.events.try_recv().ok()
    }

    pub fn recv_bounded(&self, timeout: Duration) -> Option<DebugDriverEvent> {
        self.events.recv_timeout(timeout).ok()
    }

    pub fn stop(&mut self, timeout: Duration) -> bool {
        self.stop.store(true, Ordering::Relaxed);
        let deadline = Instant::now() + timeout;
        while self.running.load(Ordering::Relaxed) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        if self.running.load(Ordering::Relaxed) {
            eprintln!(
                "{DEBUG_TRANSPORT_TAG} driver_stop_timeout timeout_ms={}",
                timeout.as_millis()
            );
            return false;
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        true
    }
}

pub struct DebugTransport {
    session_id: String,
    adapter: Box<dyn DebugAdapterPort>,
    generation: u64,
    terminated: bool,
    dropped_after_termination: u64,
    next_operation: u64,
    cancelled_operations: u64,
}

impl DebugTransport {
    pub fn start(
        session_id: &str,
        adapter_id: &str,
        root_path: &str,
        supports_run_in_terminal_request: bool,
        adapter: Box<dyn DebugAdapterPort>,
    ) -> Result<Self, String> {
        let update = command(
            CREATE_SESSION_COMMAND,
            json!({
                "sessionId": session_id,
                "adapterId": adapter_id,
                "rootPath": root_path,
                "supportsRunInTerminalRequest": supports_run_in_terminal_request,
            }),
        )?;
        let mut transport = Self {
            session_id: session_id.to_string(),
            adapter,
            generation: 1,
            terminated: false,
            dropped_after_termination: 0,
            next_operation: 1,
            cancelled_operations: 0,
        };
        transport.deliver(&update)?;
        println!(
            "{DEBUG_TRANSPORT_TAG} start session={} adapter={adapter_id} generation={}",
            transport.session_id, transport.generation
        );
        Ok(transport)
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub fn is_terminated(&self) -> bool {
        self.terminated
    }

    pub fn launch(&mut self, configuration: Value) -> Result<Vec<Value>, String> {
        self.command(LAUNCH_COMMAND, configuration)
    }

    pub fn launch_with_configuration(
        &mut self,
        configuration: Value,
    ) -> Result<Vec<Value>, String> {
        let operation_id = self.take_operation_id("launch");
        self.command(
            LAUNCH_COMMAND,
            json!({
                "sessionId": self.session_id,
                "operationId": operation_id,
                "configuration": configuration,
            }),
        )
    }

    pub fn execute_with_operation(
        &mut self,
        operation_id: &str,
        execution: &str,
        thread_id: Option<i64>,
    ) -> Result<Vec<Value>, String> {
        self.command(
            EXECUTE_COMMAND,
            json!({
                "sessionId": self.session_id,
                "operationId": operation_id,
                "command": execution,
                "threadId": thread_id,
            }),
        )
    }

    pub fn execute(
        &mut self,
        execution: &str,
        thread_id: Option<i64>,
    ) -> Result<Vec<Value>, String> {
        let operation_id = self.take_operation_id(execution);
        self.execute_with_operation(&operation_id, execution, thread_id)
    }

    pub fn inspect_with_operation(
        &mut self,
        operation_id: &str,
        kind: &str,
        thread_id: Option<i64>,
        frame_id: Option<i64>,
        variables_reference: Option<i64>,
    ) -> Result<Vec<Value>, String> {
        self.command(
            INSPECT_COMMAND,
            json!({
                "sessionId": self.session_id,
                "operationId": operation_id,
                "kind": kind,
                "threadId": thread_id,
                "frameId": frame_id,
                "variablesReference": variables_reference,
            }),
        )
    }

    pub fn inspect(
        &mut self,
        kind: &str,
        thread_id: Option<i64>,
        frame_id: Option<i64>,
        variables_reference: Option<i64>,
    ) -> Result<Vec<Value>, String> {
        let operation_id = self.take_operation_id(kind);
        self.inspect_with_operation(
            &operation_id,
            kind,
            thread_id,
            frame_id,
            variables_reference,
        )
    }

    pub fn execute_and_wait(
        &mut self,
        execution: &str,
        thread_id: Option<i64>,
        timeout: Duration,
        matches: impl Fn(&Value) -> bool,
    ) -> Result<Option<Value>, String> {
        let operation_id = self.take_operation_id(execution);
        self.command(
            EXECUTE_COMMAND,
            json!({
                "sessionId": self.session_id,
                "operationId": operation_id,
                "command": execution,
                "threadId": thread_id,
            }),
        )?;
        match self.wait_for_event(timeout, &matches)? {
            Some(event) => Ok(Some(event)),
            None => {
                self.cancel(operation_id.as_str());
                Err(format!(
                    "{DEBUG_TRANSPORT_TAG} execute_timeout session={} command={execution} operation={operation_id} timeout_ms={}",
                    self.session_id,
                    timeout.as_millis()
                ))
            }
        }
    }

    pub fn wait_for_event(
        &mut self,
        timeout: Duration,
        matches: &dyn Fn(&Value) -> bool,
    ) -> Result<Option<Value>, String> {
        let deadline = Instant::now() + timeout;
        loop {
            let slice = std::cmp::min(
                Duration::from_millis(20),
                deadline.saturating_duration_since(Instant::now()),
            );
            if let Some(found) = self
                .poll_bounded(slice)?
                .into_iter()
                .find(|event| matches(event))
            {
                return Ok(Some(found));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
        }
    }

    pub fn poll_until(
        &mut self,
        timeout: Duration,
        matches: &dyn Fn(&Value) -> bool,
    ) -> Result<(Vec<Value>, bool), String> {
        let deadline = Instant::now() + timeout;
        let mut events = Vec::new();
        loop {
            let slice = std::cmp::min(
                Duration::from_millis(20),
                deadline.saturating_duration_since(Instant::now()),
            );
            let batch = self.poll_bounded(slice)?;
            let matched = batch.iter().any(matches);
            events.extend(batch);
            if matched {
                return Ok((events, true));
            }
            if Instant::now() >= deadline {
                return Ok((events, false));
            }
        }
    }

    #[must_use]
    pub fn cancelled_operations(&self) -> u64 {
        self.cancelled_operations
    }

    pub fn cancel(&mut self, operation_id: &str) {
        self.cancelled_operations += 1;
        let accepted = cancel_operation(operation_id);
        println!(
            "{DEBUG_TRANSPORT_TAG} cancel_operation operation={operation_id} accepted={accepted} total={}",
            self.cancelled_operations
        );
    }

    pub fn take_operation_id(&mut self, execution: &str) -> String {
        let sequence = self.next_operation;
        self.next_operation += 1;
        format!(
            "{}-{}-{sequence}-{execution}",
            self.session_id, self.generation
        )
    }

    pub fn feed(&mut self, chunk: &[u8]) -> Result<Vec<Value>, String> {
        if self.terminated {
            return Err(format!(
                "{DEBUG_TRANSPORT_TAG} feed=rejected session={} reason=terminated",
                self.session_id
            ));
        }
        let update = command(
            RECEIVE_COMMAND,
            json!({
                "sessionId": self.session_id,
                "dataBase64": BASE64.encode(chunk),
            }),
        )?;
        self.deliver(&update)
    }

    pub fn poll_bounded(&mut self, timeout: Duration) -> Result<Vec<Value>, String> {
        if self.terminated {
            if self.adapter.try_recv().is_some() {
                self.dropped_after_termination += 1;
                println!(
                    "{DEBUG_TRANSPORT_TAG} poll_dropped session={} reason=terminated total={}",
                    self.session_id, self.dropped_after_termination
                );
            }
            return Ok(Vec::new());
        }

        let deadline = Instant::now() + timeout;
        let mut events = Vec::new();
        let mut received = false;
        loop {
            match self.adapter.try_recv() {
                Some(chunk) => {
                    received = true;
                    events.extend(self.feed(&chunk)?);
                }
                None if received || Instant::now() >= deadline => break,
                None => std::thread::sleep(Duration::from_millis(5)),
            }
        }
        Ok(events)
    }

    #[must_use]
    pub fn dropped_after_termination(&self) -> u64 {
        self.dropped_after_termination
    }

    pub fn disconnect(&mut self) -> Result<Vec<Value>, String> {
        let events = self.command(DISCONNECT_COMMAND, json!({ "sessionId": self.session_id }))?;
        self.terminated = true;
        self.adapter.kill();
        println!(
            "{DEBUG_TRANSPORT_TAG} disconnect session={} generation={}",
            self.session_id, self.generation
        );
        Ok(events)
    }

    pub fn destroy(&mut self) -> Result<(), String> {
        if self.terminated {
            return Ok(());
        }
        let session_id = self.session_id.clone();
        let _ = command(DESTROY_SESSION_COMMAND, json!({ "sessionId": session_id }))?;
        self.terminated = true;
        self.adapter.kill();
        println!("{DEBUG_TRANSPORT_TAG} destroy session={}", self.session_id);
        Ok(())
    }

    fn command(&mut self, command: &str, payload: Value) -> Result<Vec<Value>, String> {
        if self.terminated {
            return Err(format!(
                "{DEBUG_TRANSPORT_TAG} command={command} session={} reason=terminated",
                self.session_id
            ));
        }
        let update = command_call(command, payload)?;
        self.deliver(&update)
    }

    fn deliver(&mut self, update: &Value) -> Result<Vec<Value>, String> {
        let frames = update
            .get("outboundFrames")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for frame in &frames {
            let frame = frame.as_str().ok_or_else(|| {
                format!(
                    "{DEBUG_TRANSPORT_TAG} outbound_frame_not_a_string session={}",
                    self.session_id
                )
            })?;
            let bytes = BASE64.decode(frame).map_err(|error| {
                format!(
                    "{DEBUG_TRANSPORT_TAG} outbound_frame_decode_failed session={} error={error}",
                    self.session_id
                )
            })?;
            self.adapter.write_frame(&bytes)?;
        }

        if let Some(state) = update.get("state").and_then(Value::as_str) {
            if state == "terminated" {
                self.terminated = true;
            }
        }

        let events = update
            .get("events")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(events)
    }
}

impl Drop for DebugTransport {
    fn drop(&mut self) {
        self.adapter.kill();
    }
}

fn command(command: &str, payload: Value) -> Result<Value, String> {
    Ok(command_call(command, payload)?.into())
}

fn command_call(command: &str, payload: Value) -> Result<Value, String> {
    match core_json(command, payload) {
        Ok(Some(data)) => Ok(data),
        Ok(None) => Err(format!(
            "{DEBUG_TRANSPORT_TAG} command={command} error=no_data"
        )),
        Err(error) => Err(format!(
            "{DEBUG_TRANSPORT_TAG} command={command} error={error}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct ScriptedAdapter {
        frames: Vec<Vec<u8>>,
        incoming: Vec<Vec<u8>>,
        killed: bool,
        fail_with: Option<io::ErrorKind>,
    }

    impl ScriptedAdapter {
        fn written(&self) -> String {
            self.frames
                .iter()
                .map(|frame| String::from_utf8_lossy(frame).to_string())
                .collect::<Vec<_>>()
                .join("\n")
        }

        fn push_incoming(&mut self, chunk: Vec<u8>) {
            self.incoming.push(chunk);
        }
    }

    impl DebugAdapterPort for ScriptedAdapter {
        fn write_frame(&mut self, frame: &[u8]) -> Result<(), String> {
            if self.fail_with.is_some() {
                return Err(format!("{DEBUG_TRANSPORT_TAG} scripted_write_failed"));
            }
            self.frames.push(frame.to_vec());
            Ok(())
        }

        fn try_recv(&mut self) -> Option<Vec<u8>> {
            if self.incoming.is_empty() {
                None
            } else {
                Some(self.incoming.remove(0))
            }
        }

        fn kill(&mut self) {
            self.killed = true;
        }
    }

    fn start(
        session: &str,
    ) -> (
        DebugTransport,
        std::sync::Arc<std::sync::Mutex<ScriptedAdapter>>,
    ) {
        let adapter = std::sync::Arc::new(std::sync::Mutex::new(ScriptedAdapter::default()));
        let handle = adapter.clone();
        let transport = DebugTransport::start(
            session,
            "java",
            r"D:\proj",
            false,
            Box::new(SharedAdapter(adapter)),
        )
        .expect("起会话应当成功");
        (transport, handle)
    }

    struct SharedAdapter(std::sync::Arc<std::sync::Mutex<ScriptedAdapter>>);

    fn locked(
        adapter: &std::sync::Mutex<ScriptedAdapter>,
    ) -> std::sync::MutexGuard<'_, ScriptedAdapter> {
        adapter
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    impl DebugAdapterPort for SharedAdapter {
        fn write_frame(&mut self, frame: &[u8]) -> Result<(), String> {
            locked(&self.0).write_frame(frame)
        }

        fn try_recv(&mut self) -> Option<Vec<u8>> {
            locked(&self.0).try_recv()
        }

        fn kill(&mut self) {
            locked(&self.0).kill();
        }
    }

    #[test]
    fn start_writes_the_initialize_frame_to_the_adapter() {
        let (transport, adapter) = start("session-1");
        assert_eq!(transport.session_id(), "session-1");
        assert_eq!(transport.generation(), 1);
        assert!(!transport.is_terminated());

        let written = adapter.lock().expect("锁").written();
        assert!(
            written.contains(r#""command":"initialize""#),
            "start 必须把 Core 生成的 initialize 帧写给 adapter，实得：{written}"
        );
        assert!(
            written.contains(r#""adapterID":"java""#),
            "initialize 帧里必须带 adapterID（DAP 规范就是大写 ID）：{written}"
        );
        assert!(
            written.starts_with("Content-Length: "),
            "写出去的必须是 DAP 分帧字节（Core 已经分好帧），实得：{written}"
        );
    }

    #[test]
    fn adapter_write_failure_surfaces_as_an_error() {
        let adapter = std::sync::Arc::new(std::sync::Mutex::new(ScriptedAdapter {
            fail_with: Some(io::ErrorKind::BrokenPipe),
            ..ScriptedAdapter::default()
        }));
        let result = DebugTransport::start(
            "session-broken",
            "java",
            r"D:\proj",
            false,
            Box::new(SharedAdapter(adapter)),
        );
        assert!(
            result.is_err(),
            "adapter 写失败必须变成错误，而不是静默丢掉 initialize 帧"
        );
    }

    #[test]
    fn invalid_session_identifiers_are_rejected_before_any_write() {
        let adapter = std::sync::Arc::new(std::sync::Mutex::new(ScriptedAdapter::default()));
        let result = DebugTransport::start(
            "",
            "java",
            r"D:\proj",
            false,
            Box::new(SharedAdapter(adapter.clone())),
        );
        assert!(result.is_err(), "空 sessionId 必须由 Core 拒绝");
        let written = adapter.lock().expect("锁").frames.len();
        assert_eq!(written, 0, "会话没建起来时不许往 adapter 写任何字节");
    }

    fn dap_frame(message: Value) -> Vec<u8> {
        let body = serde_json::to_vec(&message).expect("序列化 DAP 消息");
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend(body);
        frame
    }

    #[test]
    fn feed_after_disconnect_is_rejected_and_stops_writing() {
        let (mut transport, adapter) = start("session-2");
        transport
            .feed(&dap_frame(json!({
                "seq": 1,
                "type": "response",
                "request_seq": 1,
                "success": true,
                "command": "initialize",
                "body": { "supportsConfigurationDoneRequest": true }
            })))
            .expect("initialize 响应应当被 Core 接受");
        transport
            .feed(&dap_frame(
                json!({ "seq": 2, "type": "event", "event": "initialized" }),
            ))
            .expect("initialized 事件应当被 Core 接受");

        transport.disconnect().expect("握手完成后应当能断开");
        assert!(transport.is_terminated());
        assert!(
            adapter.lock().expect("锁").killed,
            "disconnect 必须杀 adapter 进程"
        );

        let after_disconnect = adapter.lock().expect("锁").frames.len();
        let error = transport
            .feed(b"Content-Length: 2\r\n\r\n{}")
            .expect_err("断开之后的字节必须被拒绝");
        assert!(
            error.contains("terminated"),
            "错误里要能看出是会话已终止：{error}"
        );
        assert_eq!(
            adapter.lock().expect("锁").frames.len(),
            after_disconnect,
            "断开之后的字节被拒绝，也不许再往 adapter 写任何帧"
        );
        transport
            .destroy()
            .expect("terminated 之后 destroy 应当幂等成功");
    }

    #[test]
    fn receive_on_an_unknown_session_reports_a_core_error() {
        struct Discard;
        impl DebugAdapterPort for Discard {
            fn write_frame(&mut self, _frame: &[u8]) -> Result<(), String> {
                Ok(())
            }
            fn kill(&mut self) {}
        }

        let (mut transport, _adapter) = start("session-3");
        let mut foreign = DebugTransport {
            session_id: "not-a-session".to_string(),
            adapter: Box::new(Discard),
            generation: 1,
            terminated: false,
            dropped_after_termination: 0,
            next_operation: 1,
            cancelled_operations: 0,
        };
        let error = foreign
            .feed(b"Content-Length: 2\r\n\r\n{}")
            .expect_err("未知会话必须报错");
        assert!(
            error.contains("debug.receive"),
            "错误里要带上是哪条命令失败的：{error}"
        );
        transport.disconnect().expect("收尾");
    }

    fn current_test_binary() -> PathBuf {
        std::env::current_exe().expect("当前测试可执行文件路径")
    }

    #[cfg(windows)]
    fn live_child_spec(dir: &Path) -> DebugAdapterSpec {
        stdio_spec("cmd", ["/C", "more"], dir)
    }

    #[cfg(unix)]
    fn live_child_spec(dir: &Path) -> DebugAdapterSpec {
        stdio_spec("sh", ["-c", "cat"], dir)
    }

    #[test]
    fn stdio_adapter_spawns_reads_and_reports_exit() {
        let dir = std::env::temp_dir();
        let mut adapter =
            StdioDebugAdapter::spawn(&stdio_spec(current_test_binary(), ["--list"], &dir))
                .expect("应当能起子进程");

        assert!(adapter.process_id() > 0, "子进程必须有 pid");
        let chunk = adapter
            .recv_bounded(Duration::from_secs(20))
            .expect("有界等待内应当读得到子进程 stdout");
        assert!(!chunk.is_empty(), "读到的 stdout 块不能是空的");
        let code = adapter
            .wait_exit(Duration::from_secs(20))
            .expect("子进程应当自己退出");
        assert_eq!(code, 0, "`--list` 应当以 0 退出");
        assert!(adapter.is_exited(), "退出之后 is_exited 必须为真");
    }

    #[test]
    fn stdio_adapter_reports_a_missing_executable() {
        let dir = std::env::temp_dir();
        let error = StdioDebugAdapter::spawn(&stdio_spec(
            dir.join("lithe-no-such-debug-adapter"),
            Vec::<String>::new(),
            &dir,
        ))
        .err()
        .expect("不存在的可执行文件必须报错");
        assert!(
            error.contains("spawn_failed"),
            "错误要能看出是起进程失败：{error}"
        );
    }

    #[test]
    fn kill_is_idempotent_and_marks_the_child_exited() {
        let dir = std::env::temp_dir();
        let mut adapter = StdioDebugAdapter::spawn(&live_child_spec(&dir)).expect("起长驻子进程");
        assert!(!adapter.is_exited(), "刚起来的长驻子进程不该已经被判定退出");

        adapter.kill();
        assert!(adapter.is_exited(), "kill 之后必须标记为已退出");
        adapter.kill();
        assert!(adapter.is_exited(), "第二次 kill 必须幂等，不许 panic");
    }

    #[test]
    fn dropping_a_live_child_does_not_hang() {
        let dir = std::env::temp_dir();
        let adapter = StdioDebugAdapter::spawn(&live_child_spec(&dir)).expect("起长驻子进程");
        assert!(!adapter.is_exited());
        drop(adapter);
    }

    fn loopback_server() -> (String, Receiver<Vec<u8>>, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("绑回环端口");
        let address = listener.local_addr().expect("读本地地址").to_string();
        let (sender, received) = channel();
        let handle = std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut buffer = [0_u8; 4096];
            if let Ok(read) = stream.read(&mut buffer) {
                let _ = sender.send(buffer[..read].to_vec());
            }
            let _ = stream.write_all(b"adapter-says-hi");
            let _ = stream.flush();
            std::thread::sleep(Duration::from_millis(200));
        });
        (address, received, handle)
    }

    #[test]
    fn tcp_adapter_writes_frames_and_reads_adapter_output() {
        let (address, received, server) = loopback_server();
        let mut adapter =
            TcpDebugAdapter::connect(&address, Duration::from_secs(5)).expect("连上回环 adapter");
        assert!(!adapter.is_exited());
        adapter
            .write_frame(b"Content-Length: 2\r\n\r\n{}")
            .expect("写帧应当成功");

        let written = received
            .recv_timeout(Duration::from_secs(10))
            .expect("服务端应当收到写出的帧");
        assert_eq!(
            String::from_utf8_lossy(&written),
            "Content-Length: 2\r\n\r\n{}",
            "TCP 传输必须原样搬运 Core 分好的帧"
        );
        let read = adapter
            .recv_bounded(Duration::from_secs(10))
            .expect("应当读得到 adapter 发来的字节");
        assert_eq!(String::from_utf8_lossy(&read), "adapter-says-hi");
        server.join().expect("服务端线程应当结束");
    }

    #[test]
    fn launch_with_configuration_sends_the_dap_launch_request() {
        let (mut transport, adapter) = responding_transport("session-launch");
        transport
            .feed(&dap_frame(json!({
                "seq": 1,
                "type": "response",
                "request_seq": 1,
                "success": true,
                "command": "initialize",
                "body": { "supportsConfigurationDoneRequest": true }
            })))
            .expect("initialize 响应");

        transport
            .launch_with_configuration(json!({
                "name": "App",
                "request": "launch",
                "arguments": {
                    "type": "java",
                    "mainClass": "com.example.App"
                }
            }))
            .expect("握手之后必须能下发 launch");

        let written = adapter.lock().expect("锁").frames.concat();
        let written = String::from_utf8_lossy(&written);
        assert!(
            written.contains(r#""command":"launch""#),
            "Core 必须把 launch 请求写到 adapter，实得：{written}"
        );
        assert!(
            written.contains("com.example.App"),
            "launch 配置要原样透传给 adapter：{written}"
        );
    }

    #[test]
    fn java_launch_configuration_maps_core_fields_and_rejects_a_missing_main_class() {
        let configuration = java_launch_configuration(
            "App",
            &json!({
                "mainClass": "com.example.App",
                "classPaths": ["/workspace/target/classes"],
                "modulePaths": [],
                "vmArgs": ["-Xmx256m"]
            }),
        )
        .expect("有 mainClass 就应当能生成配置");
        assert_eq!(configuration["name"], json!("App"));
        assert_eq!(configuration["request"], json!("launch"));
        assert_eq!(configuration["arguments"]["type"], json!("java"));
        assert_eq!(
            configuration["arguments"]["mainClass"],
            json!("com.example.App")
        );
        assert_eq!(
            configuration["arguments"]["classPaths"],
            json!(["/workspace/target/classes"])
        );
        assert_eq!(configuration["arguments"]["vmArgs"], json!(["-Xmx256m"]));

        let error = java_launch_configuration("App", &json!({ "classPaths": [] }))
            .err()
            .expect("缺 mainClass 必须报错");
        assert!(
            error.contains("missing_main_class"),
            "错误要能看出缺的是 mainClass：{error}"
        );
    }

    #[test]
    fn inspect_returns_the_operation_result_for_threads() {
        let (mut transport, _adapter) = responding_transport("session-inspect");
        drive_to_paused(&mut transport);

        let immediate = transport
            .inspect("threads", Some(1), None, None)
            .expect("paused 之后应当能查线程");
        assert!(
            immediate.is_empty(),
            "inspect 只是把请求发出去，结果要等 adapter 回包后由下一轮 poll 归约"
        );
        let matched = transport
            .wait_for_event(Duration::from_secs(5), &|event| {
                event.get("type").and_then(Value::as_str) == Some("operationCompleted")
            })
            .expect("等 inspect 结果不该报错")
            .expect("adapter 应答之后必须归约出 operationCompleted 事件");
        let text = serde_json::to_string(&matched).unwrap_or_default();
        assert!(text.contains("main"), "线程名要出现在结果里，实得：{text}");
    }

    #[test]
    fn java_attach_configuration_points_at_the_lockback_debug_port() {
        let configuration =
            java_attach_configuration("Attach App", 5005, Some("demo")).expect("端口合法");
        assert_eq!(configuration["request"], json!("attach"));
        assert_eq!(configuration["arguments"]["hostName"], json!("127.0.0.1"));
        assert_eq!(configuration["arguments"]["port"], json!(5005));
        assert_eq!(configuration["arguments"]["projectName"], json!("demo"));

        let without_project =
            java_attach_configuration("Attach App", 5005, None).expect("端口合法");
        assert!(
            without_project["arguments"].get("projectName").is_none(),
            "没有项目名时不许塞空字符串进配置"
        );

        let error = java_attach_configuration("Attach App", 0, None)
            .err()
            .expect("端口 0 必须报错");
        assert!(error.contains("invalid_port"), "{error}");
    }

    #[test]
    fn start_tcp_session_connects_and_writes_the_initialize_frame() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("绑回环端口");
        let address = listener.local_addr().expect("读本地地址");
        let (sender, received) = channel();
        let server = std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut buffer = [0_u8; 4096];
            if let Ok(read) = stream.read(&mut buffer) {
                let _ = sender.send(buffer[..read].to_vec());
            }
        });

        let mut transport = start_tcp_session(
            "session-tcp-start",
            r"D:\proj",
            address.port(),
            Duration::from_secs(5),
        )
        .expect("按端口起 TCP 会话");
        assert_eq!(transport.session_id(), "session-tcp-start");
        let written = received
            .recv_timeout(Duration::from_secs(10))
            .expect("adapter 侧应当收到 initialize 帧");
        assert!(
            String::from_utf8_lossy(&written).contains(r#""command":"initialize""#),
            "起会话必须把 initialize 帧写出去"
        );
        transport.disconnect().expect("断开");
        server.join().expect("服务端线程应当结束");
    }

    #[test]
    fn tcp_adapter_reports_a_refused_connection() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("绑回环端口");
        let address = listener.local_addr().expect("读本地地址").to_string();
        drop(listener);

        let error = TcpDebugAdapter::connect(&address, Duration::from_secs(5))
            .err()
            .expect("没人监听时必须报错");
        assert!(
            error.contains("connect_failed"),
            "错误要能看出是建连失败：{error}"
        );
    }

    #[test]
    fn tcp_kill_closes_the_socket_and_is_idempotent() {
        let (address, _received, server) = loopback_server();
        let mut adapter =
            TcpDebugAdapter::connect(&address, Duration::from_secs(5)).expect("连上回环 adapter");
        adapter.kill();
        assert!(adapter.is_exited());
        adapter.kill();
        assert!(adapter.is_exited(), "第二次 kill 必须幂等");
        let error = adapter
            .write_frame(b"after-kill")
            .err()
            .expect("关掉之后写帧必须失败");
        assert!(
            error.contains("write_failed"),
            "错误要能看出是写失败：{error}"
        );
        server.join().expect("服务端线程应当结束");
    }

    #[test]
    fn java_start_debug_session_payload_pins_the_core_contract() {
        let payload = java_start_debug_session_payload("session-9", "op-1");
        assert_eq!(payload["sessionId"], json!("session-9"));
        assert_eq!(payload["operationId"], json!("op-1"));
        assert_eq!(
            payload["operation"],
            json!("executeCommand"),
            "Core 的 LspSemanticOperation 是 camelCase 反序列化，字符串必须是 executeCommand"
        );
        assert_eq!(
            payload["command"]["command"],
            json!("vscode.java.startDebugSession"),
            "executeCommand 的载荷就是要交给 workspace/executeCommand 的 params"
        );
    }

    #[test]
    fn parse_debug_port_accepts_the_shapes_jdt_can_return() {
        assert_eq!(parse_debug_port(&json!(56789)), Some(56789));
        assert_eq!(parse_debug_port(&json!({ "value": 56789 })), Some(56789));
        assert_eq!(parse_debug_port(&json!({ "port": "56789" })), Some(56789));
        assert_eq!(parse_debug_port(&json!({ "value": "56789" })), Some(56789));
        assert_eq!(
            parse_debug_port(&json!({ "value": "{\"port\":56789}" })),
            Some(56789),
            "java-debug 有时把结果再包一层 JSON 字符串"
        );
    }

    #[test]
    fn parse_debug_port_rejects_values_that_are_not_a_port() {
        assert_eq!(parse_debug_port(&json!(0)), None, "端口 0 不是可连的端口");
        assert_eq!(parse_debug_port(&json!(70000)), None, "超出 u16 的不是端口");
        assert_eq!(parse_debug_port(&json!({})), None);
        assert_eq!(parse_debug_port(&json!(null)), None);
        assert_eq!(parse_debug_port(&json!("not-a-port")), None);
    }

    #[test]
    fn poll_bounded_feeds_adapter_bytes_and_returns_core_events() {
        let (mut transport, adapter) = start("session-poll");
        transport
            .feed(&dap_frame(json!({
                "seq": 1,
                "type": "response",
                "request_seq": 1,
                "success": true,
                "command": "initialize",
                "body": { "supportsConfigurationDoneRequest": true }
            })))
            .expect("initialize 响应");
        transport
            .feed(&dap_frame(
                json!({ "seq": 2, "type": "event", "event": "initialized" }),
            ))
            .expect("initialized 事件");

        adapter.lock().expect("锁").push_incoming(dap_frame(json!({
            "seq": 3,
            "type": "event",
            "event": "stopped",
            "body": { "reason": "breakpoint", "threadId": 1 }
        })));

        let events = transport
            .poll_bounded(Duration::from_secs(5))
            .expect("读一轮 adapter 输出应当成功");
        assert!(
            !events.is_empty(),
            "adapter 发来 stopped 之后必须归约出事件，实得：{events:?}"
        );
        let text = serde_json::to_string(&events).unwrap_or_default();
        assert!(
            text.contains("stopped") || text.contains("Stopped"),
            "事件里应当能看出是 stopped：{text}"
        );
        assert!(
            adapter.lock().expect("锁").frames.len() >= 2,
            "initialize 与 initialized 都应当已经写给 adapter"
        );
    }

    #[test]
    fn poll_bounded_returns_empty_when_the_adapter_is_silent() {
        let (mut transport, _adapter) = start("session-silent");
        let started = Instant::now();
        let events = transport
            .poll_bounded(Duration::from_millis(120))
            .expect("静默 adapter 不算错误");
        let elapsed = started.elapsed();
        assert!(events.is_empty(), "没有输入时不该产生事件");
        assert!(
            elapsed >= Duration::from_millis(120),
            "必须真的等满 deadline，实得：{elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "等待必须是有界的，实得：{elapsed:?}"
        );
    }

    #[test]
    fn poll_after_termination_drops_input_and_counts_it() {
        let (mut transport, adapter) = start("session-stale");
        transport.disconnect().expect("断开");
        adapter
            .lock()
            .expect("锁")
            .push_incoming(b"Content-Length: 2\r\n\r\n{}".to_vec());

        let events = transport
            .poll_bounded(Duration::from_millis(50))
            .expect("终止后的 poll 不该报错，只是丢掉输入");
        assert!(events.is_empty(), "终止之后的 adapter 输入不许再产生事件");
        assert_eq!(
            transport.dropped_after_termination(),
            1,
            "丢掉的输入必须计数，便于诊断"
        );
    }

    #[derive(Default)]
    struct RespondingAdapter {
        incoming: Vec<Vec<u8>>,
        frames: Vec<Vec<u8>>,
        killed: bool,
    }

    impl RespondingAdapter {
        fn respond_to(&mut self, frame: &[u8]) {
            let text = String::from_utf8_lossy(frame).to_string();
            let Some(body_start) = text.find("\r\n\r\n").map(|index| index + 4) else {
                return;
            };
            let Ok(message) = serde_json::from_str::<Value>(&text[body_start..]) else {
                return;
            };
            let command = message
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let seq = message.get("seq").and_then(Value::as_i64).unwrap_or(1);
            match command.as_str() {
                "initialize" => {
                    self.incoming.push(dap_frame(json!({
                        "seq": 100,
                        "type": "response",
                        "request_seq": seq,
                        "success": true,
                        "command": "initialize",
                        "body": { "supportsConfigurationDoneRequest": true }
                    })));
                    self.incoming.push(dap_frame(
                        json!({ "seq": 101, "type": "event", "event": "initialized" }),
                    ));
                }
                "continue" | "next" | "stepIn" | "stepOut" | "launch" | "configurationDone" => {
                    self.incoming.push(dap_frame(json!({
                        "seq": 102,
                        "type": "response",
                        "request_seq": seq,
                        "success": true,
                        "command": command
                    })));
                    self.incoming.push(dap_frame(json!({
                        "seq": 103,
                        "type": "event",
                        "event": "stopped",
                        "body": { "reason": "step", "threadId": 1 }
                    })));
                }
                "threads" => {
                    self.incoming.push(dap_frame(json!({
                        "seq": 104,
                        "type": "response",
                        "request_seq": seq,
                        "success": true,
                        "command": "threads",
                        "body": { "threads": [ { "id": 1, "name": "main" } ] }
                    })));
                }
                _ => {}
            }
        }
    }

    impl DebugAdapterPort for RespondingAdapter {
        fn write_frame(&mut self, frame: &[u8]) -> Result<(), String> {
            self.frames.push(frame.to_vec());
            self.respond_to(frame);
            Ok(())
        }

        fn try_recv(&mut self) -> Option<Vec<u8>> {
            if self.incoming.is_empty() {
                None
            } else {
                Some(self.incoming.remove(0))
            }
        }

        fn kill(&mut self) {
            self.killed = true;
        }
    }

    struct SharedResponder(Arc<std::sync::Mutex<RespondingAdapter>>);

    impl SharedResponder {
        fn locked(&self) -> std::sync::MutexGuard<'_, RespondingAdapter> {
            self.0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }
    }

    impl DebugAdapterPort for SharedResponder {
        fn write_frame(&mut self, frame: &[u8]) -> Result<(), String> {
            self.locked().write_frame(frame)
        }

        fn try_recv(&mut self) -> Option<Vec<u8>> {
            self.locked().try_recv()
        }

        fn kill(&mut self) {
            self.locked().kill();
        }
    }

    fn events_mention(events: &[Value], needle: &str) -> bool {
        serde_json::to_string(events)
            .map(|text| text.contains(needle))
            .unwrap_or(false)
    }

    #[test]
    fn driver_runs_a_scripted_session_and_stops_within_the_deadline() {
        let adapter = Arc::new(std::sync::Mutex::new(RespondingAdapter::default()));
        let handle = adapter.clone();
        let transport = DebugTransport::start(
            "session-driver",
            "java",
            r"D:\proj",
            false,
            Box::new(SharedResponder(adapter)),
        )
        .expect("起会话");

        let mut driver = DebugSessionDriver::spawn(transport, Duration::from_millis(20));
        assert!(driver.is_running());

        let deadline = Instant::now() + Duration::from_secs(15);
        let mut saw_stopped = false;
        while Instant::now() < deadline && !saw_stopped {
            match driver.recv_bounded(Duration::from_millis(200)) {
                Some(DebugDriverEvent::Events(events)) => {
                    saw_stopped = events_mention(&events, "stopped");
                }
                Some(DebugDriverEvent::Exited { reason }) => {
                    panic!("会话提前退出：{reason}");
                }
                None => {}
            }
        }
        assert!(
            saw_stopped,
            "脚本化 adapter 应答 initialized 之后应当送达 stopped 事件"
        );

        assert!(
            driver.stop(Duration::from_secs(10)),
            "stop 必须在有界时间内把读循环停住"
        );
        assert!(!driver.is_running());
        assert!(
            handle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .killed,
            "驱动器收尾必须杀掉 adapter"
        );
    }

    fn responding_transport(
        session: &str,
    ) -> (DebugTransport, Arc<std::sync::Mutex<RespondingAdapter>>) {
        let adapter = Arc::new(std::sync::Mutex::new(RespondingAdapter::default()));
        let handle = adapter.clone();
        let transport = DebugTransport::start(
            session,
            "java",
            r"D:\proj",
            false,
            Box::new(SharedResponder(adapter)),
        )
        .expect("起会话");
        (transport, handle)
    }

    fn handshake(transport: &mut DebugTransport) {
        transport
            .feed(&dap_frame(json!({
                "seq": 1,
                "type": "response",
                "request_seq": 1,
                "success": true,
                "command": "initialize",
                "body": { "supportsConfigurationDoneRequest": true }
            })))
            .expect("initialize 响应");
        transport
            .feed(&dap_frame(
                json!({ "seq": 2, "type": "event", "event": "initialized" }),
            ))
            .expect("initialized 事件");
    }

    fn is_stopped(event: &Value) -> bool {
        serde_json::to_string(event)
            .map(|text| text.contains("stopped"))
            .unwrap_or(false)
    }

    #[test]
    fn execute_and_wait_returns_the_matching_event() {
        let (mut transport, _adapter) = responding_transport("session-exec-ok");
        handshake(&mut transport);
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut found = None;
        while Instant::now() < deadline && found.is_none() {
            found = transport
                .wait_for_event(Duration::from_secs(2), &is_stopped)
                .expect("等待事件不该报错");
        }
        assert!(found.is_some(), "脚本化 adapter 应当送来 stopped 事件");

        let matched = transport
            .execute_and_wait("continue", Some(1), Duration::from_secs(5), is_stopped)
            .expect("单步请求在有 stopped 应答时不该超时");
        assert!(matched.is_some(), "continue 之后应当等来新的 stopped");
        assert_eq!(transport.cancelled_operations(), 0);
    }

    fn drive_to_paused(transport: &mut DebugTransport) {
        handshake(transport);
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let stopped = transport
                .wait_for_event(Duration::from_millis(200), &is_stopped)
                .expect("等待 stopped 不该报错");
            if stopped.is_some() {
                return;
            }
        }
        panic!("脚本化 adapter 没能在有界时间内送达 stopped");
    }

    #[test]
    fn execute_and_wait_times_out_and_cancels_the_operation() {
        let (mut transport, _adapter) = responding_transport("session-exec-timeout");
        drive_to_paused(&mut transport);

        let started = Instant::now();
        let error = transport
            .execute_and_wait("next", Some(1), Duration::from_millis(120), |_| false)
            .expect_err("没有任何事件能满足谓词时必须超时");
        let elapsed = started.elapsed();
        assert!(
            error.contains("execute_timeout"),
            "错误要能看出是请求超时：{error}"
        );
        assert!(
            error.contains("operation=session-exec-timeout-1-"),
            "错误里要带上 operationId，便于对照 Core 的取消：{error}"
        );
        assert_eq!(
            transport.cancelled_operations(),
            1,
            "超时必须尝试取消那次在途请求（错误：{error}）"
        );
        assert!(
            elapsed >= Duration::from_millis(120) && elapsed < Duration::from_secs(5),
            "等待必须真有界，实得：{elapsed:?}"
        );
    }

    #[test]
    fn operation_ids_are_unique_per_request() {
        let (mut transport, _adapter) = responding_transport("session-op-ids");
        drive_to_paused(&mut transport);

        let first = transport
            .execute_and_wait("next", Some(1), Duration::from_millis(60), |_| false)
            .expect_err("第一次必须超时");
        let second = transport
            .execute_and_wait("next", Some(1), Duration::from_millis(60), |_| false)
            .expect_err("第二次必须超时");
        assert!(
            first.contains("operation=session-op-ids-1-1-next"),
            "第一次的 operationId 形如 session-generation-序号-命令：{first}"
        );
        assert!(
            second.contains("operation=session-op-ids-1-2-next"),
            "第二次的 operationId 必须与第一次不同：{second}"
        );
        assert_eq!(transport.cancelled_operations(), 2);
    }

    #[test]
    fn poll_until_returns_every_event_plus_whether_the_predicate_matched() {
        let (mut transport, _adapter) = responding_transport("session-poll-until");
        drive_to_paused(&mut transport);

        transport.execute("next", Some(1)).expect("单步请求");
        let (events, matched) = transport
            .poll_until(Duration::from_secs(5), &is_stopped)
            .expect("等回执不该报错");
        assert!(matched, "adapter 应答了 stopped，谓词必须命中");
        assert!(
            !events.is_empty(),
            "命中谓词时也要把这一轮收到的全部事件交出来，UI 才不会丢中间态"
        );
        assert!(
            events.iter().any(is_stopped),
            "返回的事件里必须包含那条命中的 stopped"
        );

        let started = Instant::now();
        let (silent, matched) = transport
            .poll_until(Duration::from_millis(120), &|_| false)
            .expect("静默等待不该报错");
        let elapsed = started.elapsed();
        assert!(!matched, "没有事件能满足谓词时不该报命中");
        assert!(silent.is_empty(), "静默 adapter 不该凭空产生事件");
        assert!(
            elapsed >= Duration::from_millis(120) && elapsed < Duration::from_secs(5),
            "等待必须真有界，实得：{elapsed:?}"
        );
    }
}
