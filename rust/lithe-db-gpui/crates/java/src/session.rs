//! Core LSP 会话的**同步信封**：启动 / 等就绪 / 同步文档 / 语义请求 / 虚拟文档 / 关闭。
//!
//! 这里**只有** `core_json` 调用与事件读取 —— 没有 JSON-RPC 组帧、没有 request id、
//! 没有文档版本号、没有 `$/progress` 解析。那些全在 Rust Core 里
//! （契约 `shared/contracts/rust-core-api.md:1162-1220`：进程、stdin/stdout/stderr、
//! 组帧缓冲、JSON-RPC id、文档版本、超时、能力、诊断与优雅/强制终止都归 Core）。
//!
//! ## 事件：会话里**只有一条**队列，也**只有一个**消费者
//!
//! `lsp.waitEvents` 是排空语义，两个消费者会互相偷事件（`crate::events` 的模块文档把
//! 原因与分派表写全了）。所以本文件里没有"等自己的结果时顺便排空事件"这种事：
//! [`Session::start`] 顺手起 [`crate::events::EventPump`]，`request` / `wait_ready` /
//! `shutdown` 全部通过泵的分派结果说话，`lsp.waitEvents` 只由泵调用一次。
//!
//! ## 为什么用阻塞调用 + `waitEvents` 而不是轮询
//!
//! 契约 `:1419-1422` 明写"Hosts should use `waitEvents` so idle sessions do not poll"。
//! 所以就绪、请求结果、关闭终态都走"阻塞等事件"（泵阻塞在 Core 的条件变量上，有事立刻
//! 返回），**不 sleep 轮询**。
//!
//! ## 这一步是本 crate 唯一打 JDTLS 自身日志的地方
//!
//! `lsp.waitEvents` 是唯一能看见 JDTLS 生命周期的入口（`stateChanged` / `log` /
//! `serverInfoChanged`），所以 `S1_JAVA_LOG` / `S1_JAVA_SESSION` 两行在这里打；
//! 其余诊断行（载荷解析、缓存、跳转）在 [`crate::service`]。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use lithe_db_gpui_shared::{CoreClient, CoreError, CoreRequest, core_json};
use serde_json::{Value, json};

use crate::events::{EventPump, JavaDiagnostic, SessionEvents};
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
    /// Maven 项目上下文（`maven.scan` 的结论，见 `crate::workspace::maven_context`）。
    ///
    /// `None` = 这个工作区没有可读的 `pom.xml`：**不带**这个字段
    /// （带空的 reactor 只会让 Core 做无意义的校验）。Core 拿到它会归一化生成源根、
    /// 下发 Maven profile 与 `settingsPath`（契约 `:1170-1178`）。
    pub(crate) maven_context: Option<Value>,
    pub(crate) timeouts: SessionTimeouts,
}

/// 一个已启动的 Core LSP 会话。
#[derive(Debug)]
pub(crate) struct Session {
    id: String,
    /// 事件账本 + 诊断存储（泵是唯一的写者，见 [`crate::events`]）。
    events: Arc<SessionEvents>,
    /// 事件泵：本会话**唯一**的 `lsp.waitEvents` 消费者。
    pump: EventPump,
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
            // Maven 项目模型（可选）：Core 会把 reactor 的生成源根并进
            // `java.project.sourcePaths`、下发 profile 与 settings（契约 `:1170-1178`）。
            // 字段名与形状逐字照 `rust/lithe-core/src/project/maven.rs:63-78` 的
            // `MavenLaunchContextRequest`（camelCase、`version` 必填）。
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

        // 泵在 `startServer` 成功之后就起：握手期间的 `stateChanged` / `log` /
        // `serverInfoChanged` 也在这条事件队列上，晚起就会漏掉就绪信号。
        // 泵的失败等于"这个会话拿不到任何事件"，所以直接让启动失败（Core 会话由调用方的
        // `shutdown` 收尾，与 `wait_ready` 失败时同一条路径）。
        let events = Arc::new(SessionEvents::new());
        let pump = EventPump::start(id.clone(), Arc::clone(&events))?;

        Ok(Self { id, events, pump })
    }

    /// 阻塞等到 `state: ready`（JDT 的 `language/status: ServiceReady`）。
    ///
    /// 事件**不在这里读**：泵是唯一的 `lsp.waitEvents` 消费者（见 [`crate::events`]），
    /// 它把 `stateChanged` / `serverInfoChanged` 放进生命周期队列，这里只等队列。
    /// 每次等待的片段取 `min(剩余, 5s)`：片段本身不影响"进程崩了能立刻看到" —— 泵一收到
    /// `stateChanged` 就会叫醒这里，片段只决定我们多久复查一次绝对死线。
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
                // 队列空有两种可能：还没事件，或者泵已经退出（会话死在外面）。
                // 后者不该让我们等到绝对死线，所以直接报出来。
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
                                // 日志由泵计数与打印（`report_log`），这里只取数字。
                                log_events: self.events.log_event_count(),
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
    /// 事件回来（契约 `:1336-1338`、`:1419-1430`）。外部签名（`operation` / `payload` /
    /// `deadline`）没有变，但内部不再自己读事件：**先注册 `operationId`，再等自己的通道**，
    /// 由事件泵按 `operationId` 分派（见 [`crate::events`]）。`deadline` 仍然由调用方决定，
    /// 直接交给 `recv_timeout`；超时与错误的文本形状也没有变。
    pub(crate) fn request(
        &self,
        operation: &str,
        payload: Value,
        deadline: Instant,
    ) -> Result<Value, String> {
        let operation_id = format!("lithe-db-gpui-java-{}", next_operation_id());
        let mut body = payload;
        let object = body
            .as_object_mut()
            .ok_or_else(|| "语义请求的参数必须是对象".to_string())?;
        object.insert("sessionId".to_string(), json!(self.id));
        object.insert("operationId".to_string(), json!(operation_id));
        object.insert("operation".to_string(), json!(operation));

        // ⚠️ 注册必须**早于** `lsp.request` 发出：Core 会主动终结"文档变旧"的在飞请求
        // （`engine.rs:4289-4346`），响应窗口可以极短，晚注册就会把结果当成"无人认领"丢掉。
        let waiter = self.events.register(&operation_id)?;
        if let Err(error) = core_json("lsp.request", body) {
            self.events.unregister(&operation_id);
            return Err(core_error(error));
        }

        let outcome = match waiter
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            Ok(outcome) => outcome,
            Err(RecvTimeoutError::Timeout) => Err(format!(
                "{operation} 在超时前没有结果（会话已观察 {} 条事件）",
                self.events.events_seen()
            )),
            // 泵退出时会叫醒所有等待者（`SessionEvents::mark_terminal`），所以这里
            // 报的是"事件出口没了"而不是让调用方等满自己的死线。
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

    /// `lsp.stopServer` + `lsp.destroyServer`（契约 `:1130-1141` 的收尾顺序）。
    ///
    /// 两步都要走，而且中间**必须等会话到终态**：`stopServer` 只发起有界优雅关闭、
    /// **保留会话记录**（`rust/lithe-core/src/lsp/interface/engine.rs:630-633`），
    /// 而 `destroyServer` 拒绝非终态会话（同文件 `:1212-1226`：
    /// "A running language-server session cannot be destroyed."）。实测直接连调会拿到
    /// `invalid_request`，于是 `<cacheDirectory>/jdtls/<key>` 里会留下一个写坏的索引，
    /// 更要紧的是 JVM 子进程变成孤儿。
    pub(crate) fn shutdown(mut self) {
        let payload = json!({ "sessionId": self.id });
        if let Err(error) = execute(
            CoreRequest::command("lsp.stopServer")
                .with_payload(payload.clone())
                .with_timeout_millis(20_000),
        ) {
            println!("S1_JAVA_SESSION stop_failed sessionId={} error={}", self.id, error);
        }

        // 有界等待终态。**泵这时还活着**：`stopServer` 之后到达的 `stateChanged: stopped`
        // 由泵排进生命周期队列，这里等的就是这个队列（旧实现自己再调一次 `waitEvents` ——
        // 那正是"两个消费者互相偷事件"的隐患，现在会话里只剩泵一个消费者）。
        let terminal = self
            .events
            .wait_terminal(Instant::now() + SHUTDOWN_SETTLE_TIMEOUT);
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

        // 收工：置停止标志 + join，**不能**留一个永远等 `waitEvents` 的任务
        // （`lithe-db-gpui/PLAN.md` §15.1 / 调研 §7.2 第 1 条）。`destroyServer` 之后
        // `waitEvents` 会立刻报"会话已不在运行"（`engine.rs:1944-1958`），
        // 所以泵通常已经自行退出，join 只走个形式。
        self.pump.stop();
    }

    /// 某个 `uri` 的**最近一次**诊断快照（`type: "diagnostics"` 事件，由泵归档）。
    ///
    /// 只读接口，不发起任何 Core 调用：诊断是服务端推送，没同步过文档就还没有诊断
    /// （Core 会丢掉"文档不在 `open_documents` 里"的 `publishDiagnostics`，
    /// `rust/lithe-core/src/lsp/interface/client.rs:350-352`）。
    /// 本轮**不接编辑器 UI**，接口先立在这里，下一批做波浪线时直接用。
    pub(crate) fn latest_diagnostics(&self, uri: &str) -> Vec<JavaDiagnostic> {
        self.events.latest_diagnostics(uri)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // 兜底：`wait_ready` 失败等路径上 `Session` 会被直接丢掉，不会走 `shutdown()`。
        // 少了这一步就会留一个永远卡在 `lsp.waitEvents` 上的线程。
        self.pump.stop();
    }
}

/// 排空一个会话的事件（不超过 `timeout`）。
///
/// ⚠️ **只有事件泵调它**：`lsp.waitEvents` 是排空语义，第二个调用方就会把事件偷走
/// （见 [`crate::events`] 的模块文档）。所以这里做成自由函数 —— 它需要的是 `sessionId`
/// 而不是 `&Session`，免得别处顺手又拿到一个"会话自己的"事件读取口。
pub(crate) fn wait_events(session_id: &str, timeout: Duration) -> Result<Vec<Value>, String> {
    let data = execute(
        CoreRequest::command("lsp.waitEvents")
            .with_payload(json!({
                "sessionId": session_id,
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
