use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use lithe_gpui_shared::{CoreClient, CoreError, CoreRequest, core_json};
use serde_json::{Value, json};

use crate::events::{EventPump, JavaDiagnostic, SessionEvents};
use crate::jdtls::JdtlsInstallation;

fn execute(request: CoreRequest) -> Result<Option<Value>, String> {
    CoreClient::new().execute(&request).map_err(core_error)
}

/// 会话销毁的总重试预算（后台线程里用，不占 UI 线程）。
const SHUTDOWN_SETTLE_TIMEOUT: Duration = Duration::from_secs(8);
/// 销毁重试步进。
const DESTROY_RETRY_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug)]
pub(crate) struct SessionTimeouts {
    pub(crate) initialize_milliseconds: u64,
    pub(crate) service_ready_idle_milliseconds: u64,
    pub(crate) service_ready_absolute_milliseconds: u64,
    pub(crate) request_milliseconds: u64,
}

impl Default for SessionTimeouts {
    fn default() -> Self {
        Self {
            initialize_milliseconds: 90_000,
            service_ready_idle_milliseconds: 45_000,
            service_ready_absolute_milliseconds: 600_000,
            request_milliseconds: 60_000,
        }
    }
}

pub(crate) struct SessionSpec<'a> {
    pub(crate) workspace_root: &'a Path,
    pub(crate) root_uri: &'a str,
    pub(crate) installation: &'a JdtlsInstallation,
    pub(crate) java_executable: &'a Path,
    pub(crate) java_home: &'a Path,
    pub(crate) java_runtimes: Vec<(PathBuf, String)>,
    pub(crate) cache_directory: &'a Path,
    pub(crate) workspace_fingerprint: &'a str,
    pub(crate) maven_context: Option<Value>,
    pub(crate) timeouts: SessionTimeouts,
}

#[derive(Debug)]
pub(crate) struct Session {
    id: String,
    events: Arc<SessionEvents>,
    pump: EventPump,
}

impl Session {
    pub(crate) fn session_id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug)]
pub(crate) struct ReadyReport {
    pub(crate) elapsed: Duration,
    pub(crate) log_events: usize,
    pub(crate) server_info: Option<String>,
}

impl Session {
    pub(crate) fn start(spec: &SessionSpec<'_>) -> Result<Self, String> {
        let installation = spec.installation;
        let request = json!({
            "providerId": "java",
            "executablePath": path_text(&installation.executable),
            "arguments": [],
            "environment": { "JAVA_HOME": path_text(spec.java_home) },
            "rootUri": spec.root_uri,
            "workingDirectory": path_text(spec.workspace_root),
            "runtimeExecutablePath": path_text(spec.java_executable),
            "jdtlsLaunchResources": {
                "launcherJarPath": path_text(&installation.launcher_jar_path),
                "configurationDirectory": path_text(&installation.configuration_directory),
                "lombokAgentPath": path_text(&installation.lombok_agent_path),
                "javaDebugBundlePath": installation
                    .java_debug_bundle_path
                    .as_deref()
                    .map(path_text),
                "javaExtensionBundlePaths": installation
                    .java_extension_bundle_paths
                    .iter()
                    .map(|path| path_text(path))
                    .collect::<Vec<_>>(),
            },
            "cacheDirectory": path_text(spec.cache_directory),
            "workspaceFingerprint": spec.workspace_fingerprint,
            "mavenContext": spec.maven_context,
            "javaRuntimes": spec
                .java_runtimes
                .iter()
                .map(|(home, version)| json!({
                    "homePath": path_text(home),
                    "version": version,
                }))
                .collect::<Vec<_>>(),
            "initializeTimeoutMilliseconds": spec.timeouts.initialize_milliseconds,
            "serviceReadyIdleTimeoutMilliseconds": spec.timeouts.service_ready_idle_milliseconds,
            "serviceReadyAbsoluteTimeoutMilliseconds": spec.timeouts.service_ready_absolute_milliseconds,
            "requestTimeoutMilliseconds": spec.timeouts.request_milliseconds,
            "javaBuildTimeoutMilliseconds": 600_000,
            "shutdownTimeoutMilliseconds": 2_000,
        });

        let data = execute(
            CoreRequest::command("lsp.startServer")
                .with_payload(request)
                .with_timeout_millis(30_000),
        )?;

        let id = data
            .as_ref()
            .and_then(|data| data.get("sessionId"))
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .ok_or_else(|| "lsp.startServer 的响应缺少 sessionId".to_string())?;

        let process_id = data
            .as_ref()
            .and_then(|data| data.get("processId"))
            .and_then(Value::as_u64);
        println!(
            "S1_JAVA_SESSION started sessionId={id} processId={}",
            process_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "-".to_string())
        );

        let events = Arc::new(SessionEvents::new());
        let pump = EventPump::start(id.clone(), Arc::clone(&events))?;

        Ok(Self { id, events, pump })
    }

    pub(crate) fn wait_ready(&mut self, absolute_deadline: Instant) -> Result<ReadyReport, String> {
        let started_at = Instant::now();
        let mut server_info = None;

        loop {
            let remaining = absolute_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(format!(
                    "JDTLS 在 {:?} 内没有就绪（最后状态 {server_info:?}）",
                    started_at.elapsed()
                ));
            }
            let slice = remaining.min(Duration::from_secs(5));
            let events = self.events.wait_lifecycle(slice);
            if events.is_empty() {
                if let Some(reason) = self.events.terminal_reason() {
                    return Err(format!("JDTLS 在就绪前终止：{reason}"));
                }
                continue;
            }
            for event in &events {
                match event.get("type").and_then(Value::as_str) {
                    Some("stateChanged") => match event.get("state").and_then(Value::as_str) {
                        Some("ready") => {
                            return Ok(ReadyReport {
                                elapsed: started_at.elapsed(),
                                log_events: self.events.log_event_count(),
                                server_info,
                            });
                        }
                        Some("failed") => {
                            println!("S1_JAVA_SESSION failed stateChanged={event}");
                            return Err(format!(
                                "JDTLS 初始化失败：{}",
                                event
                                    .get("detail")
                                    .or_else(|| event.get("message"))
                                    .and_then(Value::as_str)
                                    .unwrap_or("（无详情，见 S1_JAVA_SESSION failed 那一行）")
                            ));
                        }
                        _ => {}
                    },
                    Some("serverInfoChanged") => {
                        server_info = event
                            .get("serverInfo")
                            .and_then(|info| info.get("name"))
                            .and_then(Value::as_str)
                            .map(|name| {
                                let version = event
                                    .get("serverInfo")
                                    .and_then(|info| info.get("version"))
                                    .and_then(Value::as_str)
                                    .unwrap_or_default();
                                format!("{name} {version}")
                            });
                    }
                    _ => {}
                }
            }
        }
    }

    pub(crate) fn sync_document(
        &self,
        uri: &str,
        language_id: &str,
        text: &str,
    ) -> Result<(i64, bool), String> {
        let data = core_json(
            "lsp.syncDocument",
            json!({
                "sessionId": self.id,
                "uri": uri,
                "languageId": language_id,
                "text": text,
            }),
        )
        .map_err(core_error)?;
        let version = data
            .as_ref()
            .and_then(|data| data.get("documentVersion"))
            .and_then(Value::as_i64)
            .ok_or_else(|| "lsp.syncDocument 的响应缺少 documentVersion".to_string())?;
        let changed = data
            .as_ref()
            .and_then(|data| data.get("changed"))
            .and_then(Value::as_bool)
            .unwrap_or(true);
        Ok((version, changed))
    }

    pub(crate) fn request(
        &self,
        operation: &str,
        payload: Value,
        deadline: Instant,
    ) -> Result<Value, String> {
        let operation_id = format!("lithe-gpui-java-{}", next_operation_id());
        let mut body = payload;
        let object = body
            .as_object_mut()
            .ok_or_else(|| "语义请求的参数必须是对象".to_string())?;
        object.insert("sessionId".to_string(), json!(self.id));
        object.insert("operationId".to_string(), json!(operation_id));
        object.insert("operation".to_string(), json!(operation));

        let waiter = self.events.register(&operation_id)?;
        if let Err(error) = core_json("lsp.request", body) {
            self.events.unregister(&operation_id);
            return Err(core_error(error));
        }

        let outcome = match waiter.recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            Ok(outcome) => outcome,
            Err(RecvTimeoutError::Timeout) => Err(format!(
                "{operation} 在超时前没有结果（会话已观察 {} 条事件）",
                self.events.events_seen()
            )),
            Err(RecvTimeoutError::Disconnected) => Err(format!(
                "{operation} 的事件泵已退出：{}",
                self.events
                    .terminal_reason()
                    .unwrap_or_else(|| "原因未知".to_string())
            )),
        };
        self.events.unregister(&operation_id);
        outcome
    }

    /// 收尾。两件事必须分开看：
    ///
    /// - `lsp.stopServer` 只是**发出** `shutdown`，真正的强杀由 Core 的
    ///   `shutdown_timeout_milliseconds`（2s）兜底，所以它立刻返回；
    /// - `lsp.destroyServer` 只接受已经到终态（Stopped/Failed）的会话。
    ///
    /// 以前这里在 UI 线程上死等 5 秒再销毁：切换工程/关窗口时卡的就是这一段，而且等到的
    /// 往往还是"S 秒内没变终态 → 销毁被拒"。等待和销毁都不需要占用 UI 线程，所以整段挪到
    /// 后台线程，用有界重试代替一次性等待。
    pub(crate) fn shutdown(mut self) {
        let payload = json!({ "sessionId": self.id });
        let started = Instant::now();
        match execute(
            CoreRequest::command("lsp.stopServer")
                .with_payload(payload.clone())
                .with_timeout_millis(20_000),
        ) {
            Ok(_) => println!(
                "S1_JAVA_SESSION stop_requested sessionId={} elapsedMs={}",
                self.id,
                started.elapsed().as_millis()
            ),
            Err(error) => println!(
                "S1_JAVA_SESSION stop_failed sessionId={} error={}",
                self.id, error
            ),
        }

        // 事件泵只置停止标志：它自己会在下一个 `lsp.waitEvents` 切片（1s）后退出，
        // 不必再为它等满 5 秒。
        self.events.request_stop();

        let id = self.id.clone();
        let spawned = std::thread::Builder::new()
            .name(format!("lithe-java-teardown-{id}"))
            .spawn(move || teardown_session(id, payload));
        if let Err(error) = spawned {
            println!(
                "S1_JAVA_SESSION teardown_unspawned sessionId={} error={error}",
                self.id
            );
        }
    }

    pub(crate) fn latest_diagnostics(&self, uri: &str) -> Vec<JavaDiagnostic> {
        self.events.latest_diagnostics(uri)
    }

    pub(crate) fn diagnostics_by_uri(&self) -> Vec<(String, Vec<JavaDiagnostic>)> {
        self.events.diagnostics_by_uri()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.pump.stop();
    }
}

pub(crate) fn wait_events(session_id: &str, timeout: Duration) -> Result<Vec<Value>, String> {
    let data = execute(
        CoreRequest::command("lsp.waitEvents")
            .with_payload(json!({
                "sessionId": session_id,
                "timeoutMilliseconds": timeout.as_millis().max(1) as u64,
            }))
            .with_timeout_millis(timeout.as_millis() as u64 + 5_000),
    )?;
    Ok(data
        .as_ref()
        .and_then(|data| data.get("events"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

/// 等会话落到终态再销毁。Core 的强杀在 2s 后发生，进程退出还要一个监控周期，所以按
/// 250ms 步进重试：第一次成功即返回，超时只记一行诊断，不阻塞任何人。
fn teardown_session(id: String, payload: Value) {
    let started = Instant::now();
    let deadline = started + SHUTDOWN_SETTLE_TIMEOUT;
    let mut attempts = 0u32;
    loop {
        attempts += 1;
        let last = Instant::now() + DESTROY_RETRY_INTERVAL >= deadline;
        match core_json("lsp.destroyServer", payload.clone()) {
            Ok(_) => {
                println!(
                    "S1_JAVA_SESSION stopped sessionId={id} attempts={attempts} waitedMs={}",
                    started.elapsed().as_millis()
                );
                return;
            }
            // 预算用尽前不打印：重试期间的"未到终态"是正常现象，只记最后一次的真实拒绝原因。
            Err(_) if !last => {}
            Err(error) => {
                println!(
                    "S1_JAVA_SESSION destroy_unconfirmed sessionId={id} attempts={attempts} elapsedMs={} error={error}",
                    started.elapsed().as_millis()
                );
                return;
            }
        }
        std::thread::sleep(DESTROY_RETRY_INTERVAL);
    }
}

fn next_operation_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn path_text(path: &Path) -> String {
    crate::jdtls::normalize_path(path)
}

fn core_error(error: CoreError) -> String {
    match error.code() {
        Some(code) => format!("{code}：{error}"),
        None => error.to_string(),
    }
}
