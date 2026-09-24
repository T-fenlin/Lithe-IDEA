//! Core LSP 会话的**同步信封**：启动 / 等就绪 / 同步文档 / 语义请求 / 虚拟文档 / 关闭。
//!
//! 这里**只有** `core_json` 调用与事件读取 —— 没有 JSON-RPC 组帧、没有 request id、
//! 没有文档版本号、没有 `$/progress` 解析。那些全在 Rust Core 里
//! （契约 `shared/contracts/rust-core-api.md:1162-1220`：进程、stdin/stdout/stderr、
//! 组帧缓冲、JSON-RPC id、文档版本、超时、能力、诊断与优雅/强制终止都归 Core）。
//!
//! ## 为什么用阻塞调用 + `waitEvents` 而不是轮询
//!
//! 契约 `:1419-1422` 明写"Hosts should use `waitEvents` so idle sessions do not poll"。
//! 所以等待就绪与等待请求结果都走 `lsp.waitEvents`（Core 在会话事件通道上阻塞，
//! 到点或有事就返回），**不 sleep 轮询**。
//!
//! ## 这一步是本 crate 唯一打 JDTLS 自身日志的地方
//!
//! `lsp.waitEvents` 是唯一能看见 JDTLS 生命周期的入口（`stateChanged` / `log` /
//! `serverInfoChanged`），所以 `S1_JAVA_LOG` / `S1_JAVA_SESSION` 两行在这里打；
//! 其余诊断行（载荷解析、缓存、跳转）在 [`crate::service`]。

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lithe_gpui_shared::{CoreClient, CoreError, CoreRequest, core_json};
use serde_json::{Value, json};

use crate::jdtls::JdtlsInstallation;

/// 执行一条已拼好的请求并解开信封（`CoreClient` 是无状态值，随手建一个即可）。
fn execute(request: CoreRequest) -> Result<Option<Value>, String> {
    CoreClient::new().execute(&request).map_err(core_error)
}

/// 等待会话进入终态的上限：它决定窗口关闭时最多卡多久（见 `crate::service`）。
const SHUTDOWN_SETTLE_TIMEOUT: Duration = Duration::from_secs(5);

/// 一次会话超时参数（毫秒）。默认值与 Core 的默认值同源，显式传是为了让超时是
/// **产品决定**而不是某天 Core 改默认值就悄悄变了行为。
#[derive(Clone, Copy, Debug)]
pub(crate) struct SessionTimeouts {
    /// 只界定标准 LSP `initialize` 握手（契约 `:1197-1198`）。
    pub(crate) initialize_milliseconds: u64,
    /// 等待 JDT 专有就绪信号的空闲上限。
    pub(crate) service_ready_idle_milliseconds: u64,
    /// 等待就绪的绝对上限（最后的兜底）。
    pub(crate) service_ready_absolute_milliseconds: u64,
    /// 普通语义请求的超时。
    pub(crate) request_milliseconds: u64,
}

impl Default for SessionTimeouts {
    fn default() -> Self {
        Self {
            // JDTLS 首次启动要起一个 JVM 再握手，10s（Core 默认）偏紧；真机 JDT 的
            // `initializeTimeoutMilliseconds` 也是放宽的。
            initialize_milliseconds: 90_000,
            service_ready_idle_milliseconds: 45_000,
            service_ready_absolute_milliseconds: 600_000,
            request_milliseconds: 60_000,
        }
    }
}

/// `lsp.startServer` 要的全部输入。
pub(crate) struct SessionSpec<'a> {
    /// 工作区根（同时是 `workingDirectory`）。
    pub(crate) workspace_root: &'a Path,
    /// `file://` 形式的 `rootUri`。
    pub(crate) root_uri: &'a str,
    pub(crate) installation: &'a JdtlsInstallation,
    /// 跑 JDTLS 的 `java`。
    pub(crate) java_executable: &'a Path,
    /// 同一个 JDK 的根（`JAVA_HOME`）。
    pub(crate) java_home: &'a Path,
    /// 供 JDT 给项目绑定 JDK 的候选（`javaRuntimes`）。
    pub(crate) java_runtimes: Vec<(PathBuf, String)>,
    /// JDT 状态目录的父目录（Core 会拼 `jdtls/<workspaceKey>`）。
    pub(crate) cache_directory: &'a Path,
    /// `java.jdtWorkspaceFingerprint` 的不透明结果。
    pub(crate) workspace_fingerprint: &'a str,
    pub(crate) timeouts: SessionTimeouts,
}

/// 一个已启动的 Core LSP 会话。
#[derive(Debug)]
pub(crate) struct Session {
    id: String,
    /// 连续重复的 `log` 消息只打一次，避免 JDT 的进度流把 stdout 刷满。
    ///
    /// 用 `Mutex` 而不是 `&mut self`：会话被 `JavaLanguageService` 的状态锁保护，
    /// 请求路径只需要 `&Session`，为一行日志把整条链改成可变借用不值得。
    last_log: Mutex<Option<String>>,
}

/// 会话启动后的状态，给调用方做诊断与"是否复用"判断。
#[derive(Debug)]
pub(crate) struct ReadyReport {
    /// 从 `startServer` 返回到 `state: ready` 的墙钟耗时。
    pub(crate) elapsed: Duration,
    /// 期间观察到的日志事件条数。
    pub(crate) log_events: usize,
    /// `serverInfoChanged` 报出来的服务名与版本（有就报）。
    pub(crate) server_info: Option<String>,
}

impl Session {
    /// 启动一个 Core 拥有的 JDTLS 进程并**不等待**就绪（`state` 会是 `initializing`）。
    ///
    /// `lsp.startServer` 的响应是同步的、很快：Core 建进程与管道、写完 `initialize`
    /// 就返回（`rust/lithe-core/src/lsp/interface/engine.rs:1198-1202`），
    /// 握手与 JDT 的项目导入都在 Core 的后台线程里继续。所以这个调用的超时只需覆盖
    /// "起一个进程 + 写一帧"，不需要覆盖 `initialize`；用 `initialize` 的死线留足余量。
    pub(crate) fn start(spec: &SessionSpec<'_>) -> Result<Self, String> {
        let installation = spec.installation;
        let request = json!({
            "providerId": "java",
            "executablePath": path_text(&installation.executable),
            // 结构化直启下 Core 用 `runtimeExecutablePath` 作为进程、自己拼 JVM 参数
            // （契约 `:1208-1217`），所以这里**不留**老的 wrapper 参数。
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
            // Java 项目构建（`vscode.java.buildWorkspace`）本轮不触发，用 Core 的默认量级。
            "javaBuildTimeoutMilliseconds": 600_000,
            // 优雅关闭的上限。`shutdown()` 是在窗口关闭路径上同步调的（见
            // `crate::service` 的说明），所以这个值直接决定退出时最多卡多久。
            "shutdownTimeoutMilliseconds": 2_000,
        });

        // 信封自身的超时：进程创建 + 校验 + 一帧 initialize，30s 足够；
        // 真出问题时应当报"启动失败"而不是把 UI 的后台任务挂到握手死线上。
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

        Ok(Self {
            id,
            last_log: Mutex::new(None),
        })
    }

    /// 阻塞等到 `state: ready`（JDT 的 `language/status: ServiceReady`）。
    ///
    /// 事件只来自 `lsp.waitEvents`；每次等待的片段取 `min(剩余, 5s)`，这样：
    /// 进程崩了 / 会话 `failed` 能**立刻**看到，而不会被一个很长的阻塞盖住。
    pub(crate) fn wait_ready(&mut self, absolute_deadline: Instant) -> Result<ReadyReport, String> {
        let started_at = Instant::now();
        let mut log_events = 0usize;
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
            let events = self.wait_events(slice)?;
            if events.is_empty() {
                continue;
            }
            for event in &events {
                match event.get("type").and_then(Value::as_str) {
                    Some("stateChanged") => match event.get("state").and_then(Value::as_str) {
                        Some("ready") => {
                            return Ok(ReadyReport {
                                elapsed: started_at.elapsed(),
                                log_events,
                                server_info,
                            });
                        }
                        Some("failed") => {
                            // 失败的 `stateChanged` 里没有 `detail` / `message` 时也要能定位，
                            // 所以把整条事件打出来（JDTLS 起不来时这是唯一的现场）。
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
                    Some("log") => {
                        log_events += 1;
                        self.report_log(event);
                    }
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

    /// `lsp.syncDocument`：首次是 `didOpen`（版本 1），之后是递增版本的 `didChange`。
    ///
    /// 返回 `(documentVersion, changed)`。Core 对"文本没变"的重复同步返回 `changed: false`
    /// 且版本不变（契约 `:1303-1310`），调用方据此跳过后续请求。
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

    /// 一次语义请求（`textDocument/definition` 等）的**终态结果**。
    ///
    /// `lsp.request` 本身是异步的：它只返回 `operationId`，结果通过 `requestCompleted`
    /// 事件回来（契约 `:1336-1338`、`:1419-1430`）。所以这里发完请求就等着，
    /// 且**只认带同一个 operationId 的事件**（会话里可能还有别的在飞请求）。
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

        core_json("lsp.request", body).map_err(core_error)?;

        let mut seen = 0usize;
        loop {
            if Instant::now() >= deadline {
                return Err(format!(
                    "{operation} 在超时前没有结果（已观察 {seen} 条事件）"
                ));
            }
            let slice = deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(5));
            let events = self.wait_events(slice)?;
            seen += events.len();
            for event in &events {
                match event.get("type").and_then(Value::as_str) {
                    Some("log") => self.report_log(event),
                    Some("requestCompleted")
                        if event.get("operationId").and_then(Value::as_str)
                            == Some(operation_id.as_str()) =>
                    {
                        if let Some(error) = event.get("error").filter(|error| !error.is_null()) {
                            let code = error
                                .get("code")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown");
                            let message = error
                                .get("message")
                                .and_then(Value::as_str)
                                .unwrap_or("（无消息）");
                            let stage = error
                                .get("stage")
                                .and_then(Value::as_str)
                                .unwrap_or("request");
                            return Err(format!("{code}@{stage}：{message}"));
                        }
                        return Ok(event.get("result").cloned().unwrap_or(Value::Null));
                    }
                    _ => {}
                }
            }
        }
    }

    /// `lsp.stopServer` + `lsp.destroyServer`（契约 `:1130-1141` 的收尾顺序）。
    ///
    /// 两步都要走，而且中间**必须等会话到终态**：`stopServer` 只发起有界优雅关闭、
    /// **保留会话记录**（`rust/lithe-core/src/lsp/interface/engine.rs:630-633`），
    /// 而 `destroyServer` 拒绝非终态会话（同文件 `:1212-1226`：
    /// "A running language-server session cannot be destroyed."）。实测直接连调会拿到
    /// `invalid_request`，于是 `<cacheDirectory>/jdtls/<key>` 里会留下一个写坏的索引，
    /// 更要紧的是 JVM 子进程变成孤儿。
    pub(crate) fn shutdown(self) {
        let payload = json!({ "sessionId": self.id });
        if let Err(error) = execute(
            CoreRequest::command("lsp.stopServer")
                .with_payload(payload.clone())
                .with_timeout_millis(20_000),
        ) {
            println!("S1_JAVA_SESSION stop_failed sessionId={} error={error}", self.id);
        }

        // 有界等待终态。每一步都用 `waitEvents`（Core 在会话事件通道上阻塞），
        // 不用 sleep 轮询；`stopped` / `failed` 之外的状态继续等。
        let deadline = Instant::now() + SHUTDOWN_SETTLE_TIMEOUT;
        let mut terminal = false;
        while Instant::now() < deadline {
            let slice = deadline
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(250));
            match self.wait_events(slice) {
                Ok(events) => {
                    terminal = events.iter().any(|event| {
                        event.get("type").and_then(Value::as_str) == Some("stateChanged")
                            && matches!(
                                event.get("state").and_then(Value::as_str),
                                Some("stopped") | Some("failed")
                            )
                    });
                    if terminal {
                        break;
                    }
                }
                Err(error) => {
                    // 会话已经没了（`sessionStopped`）也算终态。
                    println!("S1_JAVA_SESSION settle_failed sessionId={} error={error}", self.id);
                    break;
                }
            }
        }
        if !terminal {
            // 不静默：摘不掉句柄就是子进程可能还活着，必须在日志里留痕。
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
    }

    /// 排空事件（不超过 `timeout`）。
    fn wait_events(&self, timeout: Duration) -> Result<Vec<Value>, String> {
        let data = execute(
            CoreRequest::command("lsp.waitEvents")
                .with_payload(json!({
                    "sessionId": self.id,
                    "timeoutMilliseconds": timeout.as_millis().max(1) as u64,
                }))
                // 信封超时比会话超时多 5s：Core 在会话通道上阻塞，信封不能先把它掐了。
                .with_timeout_millis(timeout.as_millis() as u64 + 5_000),
        )?;
        Ok(data
            .as_ref()
            .and_then(|data| data.get("events"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    /// JDT 的日志（项目导入 / 进度）→ `S1_JAVA_LOG`，连续重复只打一次。
    fn report_log(&self, event: &Value) {
        let message = event
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let detail = event.get("detail").and_then(Value::as_str).unwrap_or_default();
        let line = format!("{message} {detail}");
        let mut last_log = self
            .last_log
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if last_log.as_deref() == Some(line.as_str()) {
            return;
        }
        let level = event.get("level").and_then(Value::as_str).unwrap_or("info");
        println!("S1_JAVA_LOG level={level} message={line}");
        *last_log = Some(line);
    }
}

/// 进程内自增的请求号（`operationId` 只需在会话内唯一，进程内唯一更省心）。
fn next_operation_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// 路径 → JSON 文本。
///
/// **必须过 [`crate::jdtls::normalize_path`]**：`canonicalize` / 平台 API 给出的
/// verbatim（`\\?\D:\...`）路径 JVM 不认，发出去就是"启动 JAR 找不到、进程秒退"。
fn path_text(path: &Path) -> String {
    crate::jdtls::normalize_path(path)
}

/// 信封级失败 → 可读文本（诊断行要原因，不要分层）。
fn core_error(error: CoreError) -> String {
    match error.code() {
        Some(code) => format!("{code}：{error}"),
        None => error.to_string(),
    }
}
