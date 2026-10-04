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

const SHUTDOWN_SETTLE_TIMEOUT: Duration = Duration::from_secs(5);

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

    pub(crate) fn shutdown(mut self) {
        let payload = json!({ "sessionId": self.id });
        if let Err(error) = execute(
            CoreRequest::command("lsp.stopServer")
                .with_payload(payload.clone())
                .with_timeout_millis(20_000),
        ) {
            println!(
                "S1_JAVA_SESSION stop_failed sessionId={} error={}",
                self.id, error
            );
        }

        let terminal = self
            .events
            .wait_terminal(Instant::now() + SHUTDOWN_SETTLE_TIMEOUT);
        if !terminal {
            println!(
                "S1_JAVA_SESSION destroy_skipped sessionId={} reason=not-terminal elapsedMs={}",
                self.id,
                SHUTDOWN_SETTLE_TIMEOUT.as_millis()
            );
        }

        if let Err(error) = core_json("lsp.destroyServer", payload) {
            println!(
                "S1_JAVA_SESSION destroy_failed sessionId={} error={error}",
                self.id
            );
        } else {
            println!("S1_JAVA_SESSION stopped sessionId={}", self.id);
        }

        self.pump.stop();
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
