use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use lithe_gpui_shared::core_json;
use serde_json::{Value, json};

pub const DEBUG_TRANSPORT_TAG: &str = "S1_DEBUG_TRANSPORT";

const CREATE_SESSION_COMMAND: &str = "debug.createSession";
const LAUNCH_COMMAND: &str = "debug.launch";
const RECEIVE_COMMAND: &str = "debug.receive";
const EXECUTE_COMMAND: &str = "debug.execute";
const DISCONNECT_COMMAND: &str = "debug.disconnect";
const DESTROY_SESSION_COMMAND: &str = "debug.destroySession";

const STDERR_TAIL_BYTES: usize = 4096;

pub struct DebugAdapterSpec {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: Vec<(String, String)>,
}

pub trait DebugAdapterPort: Send {
    fn write_frame(&mut self, frame: &[u8]) -> Result<(), String>;

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

pub struct DebugTransport {
    session_id: String,
    adapter: Box<dyn DebugAdapterPort>,
    generation: u64,
    terminated: bool,
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

    pub fn execute(
        &mut self,
        execution: &str,
        thread_id: Option<i64>,
    ) -> Result<Vec<Value>, String> {
        self.command(
            EXECUTE_COMMAND,
            json!({
                "sessionId": self.session_id,
                "operationId": format!("{}-{}", self.session_id, self.generation),
                "command": execution,
                "threadId": thread_id,
            }),
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
    }

    impl DebugAdapterPort for ScriptedAdapter {
        fn write_frame(&mut self, frame: &[u8]) -> Result<(), String> {
            if self.fail_with.is_some() {
                return Err(format!("{DEBUG_TRANSPORT_TAG} scripted_write_failed"));
            }
            self.frames.push(frame.to_vec());
            Ok(())
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
}
