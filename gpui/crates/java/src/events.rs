//! 会话事件泵：`lsp.waitEvents` 的**唯一**消费者，以及按事件类型 / `operationId` 的分派表。
//!
//! ## 为什么必须只有一个消费者
//!
//! `lsp.waitEvents` 是**排空**语义（契约 `shared/contracts/rust-core-api.md:1419-1422`；
//! Core 的实现 `rust/lithe-core/src/lsp/interface/engine.rs:1936-1978` 就是 `events.drain(..)`）：
//! 它把队列里的事件一次全部取走。两个线程各调一次就是"谁先到谁全拿"，后到的那次什么也看不到。
//!
//! 改造前 `Session::request` 自己就是那个消费者（旧 `session.rs:329-357`）：它只认自己的
//! `operationId`，把其余事件**丢掉**。于是只要再挂一个诊断订阅，两边就会互相偷事件 ——
//! 这正是 `gpui/research/editor-lsp-completion.md` §4.7 点名的那条硬前置。现在：
//!
//! - [`EventPump`] 是**纯 std 线程**（本 crate 没有 gpui 依赖，泵也不该有：调研 §6 第 3 步
//!   的方案 (A)），它以有界切片循环调 `lsp.waitEvents`，把每个事件交给
//!   [`SessionEvents::dispatch`]；
//! - 想要事件的人（`Session::request` / `Session::wait_ready` / `Session::shutdown` /
//!   诊断存储）只跟分派表打交道，谁都不再自己碰 `lsp.waitEvents`。
//!
//! ## 为什么诊断和请求结果在同一个流里
//!
//! Core 每个会话只有**一条**事件队列（`engine.rs:586` 的 `event_signal` + `events`），
//! `stateChanged` / `log` / `diagnostics` / `requestCompleted` 全排在里面。
//! `publishDiagnostics` 不是"我们能去拉的东西"，而是服务端推送（`client.rs:343-377`），
//! 落点就是这条队列。所以分派只能按事件的 `type` 分流，不能按"谁在等"分队列。
//!
//! ## 死线怎么保持
//!
//! `Session::request(operation, payload, deadline)` 的外部签名不变，内部改成
//! "先注册 `operationId`，再阻塞在自己的通道上"。`deadline` 仍然是调用方给的 `Instant`：
//! `recv_timeout(deadline - now)` 直接尊重它，超时与失败的文本也照旧（见 `Session::request`）。
//! 唯一的顺序要求是**注册必须早于 `lsp.request` 发出** —— Core 会对"文档变旧"的请求主动推
//! `requestCompleted`（`engine.rs:4289-4346`），响应窗口可以极短。
//!
//! ## 诊断为什么要**等**事件而不是缓存后重放
//!
//! 这里只存"最近一次快照"。上游编辑器每次文本编辑都会清空自己的诊断集合
//! （`gpui-base-0.6.6/src/input/editor/state.rs:3551-3553` 等），波浪线必须在"编辑后
//! 由 JDT 重新 publish"时回来；宿主侧缓存旧诊断再重放只会画出对不上正文的波浪线。

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::service::{JavaPosition, parse_position};

/// 泵单次 `lsp.waitEvents` 的阻塞上限。
///
/// 它**不是**轮询周期：泵阻塞在 Core 的条件变量上（`engine.rs:1964-1976`），有事立刻返回。
/// 这个值只决定两件事：① 空闲时多久问一次 Core；② `EventPump::stop` 置停止标志后，
/// 泵最多多久退出 —— 因此它同时是那个 `join` 的实际上界（不能无限等）。
const PUMP_SLICE: Duration = Duration::from_millis(1_000);

// ---------------------------------------------------------------------------
// 诊断（Core `LspClientDiagnostic` 的宿主侧镜像）
// ---------------------------------------------------------------------------

/// 一条诊断。
///
/// 字段与 Core 归一化后的 `LspClientDiagnostic`（`rust/lithe-core/src/lsp/interface/types.rs:175-185`）
/// 一一对应：`range` / `severity` / `message` / `source` / `code` / `tags`。
/// **不保留 `relatedInformation`**：Core 会归一化它（`client.rs:824-827`），但诊断 UI 的第一版
/// 只画一段波浪线，需要时再按同一个形状补（本轮不接编辑器 UI）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaDiagnostic {
    /// 覆盖范围（0 基行 + 0 基 UTF-16 码元列）。
    pub range: JavaDiagnosticRange,
    /// LSP `DiagnosticSeverity` 的原始数值（1=Error 2=Warning 3=Information 4=Hint）；
    /// 缺字段 = `None`（服务端没说，宿主不猜）。
    pub severity: Option<i64>,
    /// 消息正文。
    pub message: String,
    /// 诊断来源（JDT 通常是 `Java`）。
    pub source: Option<String>,
    /// 服务端错误码。LSP 允许 `string | number`，这里取 Core 归一化后的字符串。
    pub code: Option<String>,
    /// LSP `DiagnosticTag` 列表（1=Unnecessary 2=Deprecated）。
    pub tags: Vec<i64>,
}

/// 诊断覆盖的范围。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JavaDiagnosticRange {
    /// 起点。
    pub start: JavaPosition,
    /// 终点（不含）。
    pub end: JavaPosition,
}

/// 解析一条 `LspClientDiagnostic`。
///
/// 宽容度与 Core 的 `parse_diagnostics`（`client.rs:795-832`）一致：**`range` 缺失就丢掉这一条**
/// （没有范围的波浪线无处可画），`message` 缺失按空串，`code` 支持字符串与数字。
/// 绝不 panic：语言服务器的输出本来就随版本变化，一条脏诊断不该把整个快照或整个会话掀掉。
fn parse_diagnostic(value: &Value) -> Option<JavaDiagnostic> {
    let range = value.get("range")?;
    Some(JavaDiagnostic {
        range: JavaDiagnosticRange {
            start: parse_position(range.get("start")?)?,
            end: parse_position(range.get("end")?)?,
        },
        severity: value.get("severity").and_then(Value::as_i64),
        message: value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        source: value
            .get("source")
            .and_then(Value::as_str)
            .map(str::to_string),
        code: value.get("code").and_then(|code| match code {
            Value::String(code) => Some(code.clone()),
            Value::Number(code) => Some(code.to_string()),
            _ => None,
        }),
        tags: value
            .get("tags")
            .and_then(Value::as_array)
            .map(|tags| tags.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default(),
    })
}

// ---------------------------------------------------------------------------
// 共享账本
// ---------------------------------------------------------------------------

/// 泵与请求方共享的**会话事件账本**。
///
/// 泵是唯一的写者，其余线程只读自己那一格：
/// `pending` 是"在飞请求 → 等待者"，`lifecycle` 是给 `wait_ready` / `shutdown` 的生命周期队列，
/// `diagnostics` 是按 `uri` 归档的最近一次快照。
#[derive(Debug)]
pub(crate) struct SessionEvents {
    /// `operationId` → 等待者。注册与注销都由请求方做，完成时由泵取走。
    pending: Mutex<HashMap<String, Sender<Result<Value, String>>>>,
    /// 生命周期事件（`stateChanged` / `serverInfoChanged`）与终态标志。
    lifecycle: Mutex<Lifecycle>,
    /// `lifecycle` 变化的唤醒源（`wait_ready` / `shutdown` / `wait_terminal` 等它）。
    signal: Condvar,
    /// `uri` → 最近一次 `diagnostics` 快照。
    diagnostics: Mutex<HashMap<String, Vec<JavaDiagnostic>>>,
    /// 事件类型 → 条数。用于"某类事件从来没到过"这种问题的排查，也只用来打印未知类型的首条。
    counts: Mutex<BTreeMap<String, usize>>,
    /// 连续重复的 `log` 只打一次（原 `Session::last_log`）：JDT 的进度流会把 stdout 刷满。
    /// 去重状态跟着账本走，因为现在只有泵在读事件。
    last_log: Mutex<Option<String>>,
    /// 泵观察到的 `log` 事件条数（`ReadyReport::log_events`）。
    log_events: AtomicUsize,
    /// 泵观察到的**全部**事件条数。超时诊断行要一个"它到底在动吗"的数字。
    events_seen: AtomicUsize,
    /// 无人认领的 `requestCompleted` 条数（请求方已超时/已注销）。
    unclaimed: AtomicUsize,
    /// 停止标志：会话收尾时置位，泵在下一次循环边界退出。
    stopping: AtomicBool,
}

/// `lifecycle` 的那一格状态。
#[derive(Debug, Default)]
struct Lifecycle {
    /// 还没被取走的生命周期事件。
    ///
    /// 用**队列**而不是"最新状态"单元格：会话可能在调用方开始等之前就已经就绪，
    /// 单元格会把那个 `stateChanged: ready` 盖掉，于是 `wait_ready` 一直等到绝对死线。
    queued: Vec<Value>,
    /// 会话终态的原因（`stopped` / `failed` / 泵退出）。`Some` 之后不再接受新的在飞请求。
    terminal: Option<String>,
}

impl SessionEvents {
    /// 空账本。
    pub(crate) fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            lifecycle: Mutex::new(Lifecycle::default()),
            signal: Condvar::new(),
            diagnostics: Mutex::new(HashMap::new()),
            counts: Mutex::new(BTreeMap::new()),
            last_log: Mutex::new(None),
            log_events: AtomicUsize::new(0),
            events_seen: AtomicUsize::new(0),
            unclaimed: AtomicUsize::new(0),
            stopping: AtomicBool::new(false),
        }
    }

    // -----------------------------------------------------------------------
    // 请求方
    // -----------------------------------------------------------------------

    /// 注册一个等待者，返回它自己的结果通道。
    ///
    /// **调用顺序有要求**：必须早于 `lsp.request` 发出（见模块文档）。会话已经到终态时
    /// 直接报错，而不是让调用方等满自己的死线 —— 死掉的会话不会再产生任何结果。
    ///
    /// 锁序：`pending` → `lifecycle`。泵侧取 `lifecycle` 时会**先放开**再取 `pending`
    /// （`mark_terminal`），不存在反向嵌套。
    pub(crate) fn register(
        &self,
        operation_id: &str,
    ) -> Result<Receiver<Result<Value, String>>, String> {
        let mut pending = lock(&self.pending);
        if let Some(reason) = lock(&self.lifecycle).terminal.clone() {
            return Err(reason);
        }
        if pending.contains_key(operation_id) {
            // Core 对同一个会话里重复挂起的 `operationId` 也是 `invalid_request`
            // （`engine.rs:1491-1498`），在这里拦住能给出更直接的原因。
            return Err(format!("operationId 重复：{operation_id}"));
        }
        let (sender, receiver) = channel();
        pending.insert(operation_id.to_string(), sender);
        Ok(receiver)
    }

    /// 注销等待者（请求超时、失败、或结果已被取走）。幂等。
    pub(crate) fn unregister(&self, operation_id: &str) {
        lock(&self.pending).remove(operation_id);
    }

    /// 会话终态的原因（还没有就是 `None`）。
    pub(crate) fn terminal_reason(&self) -> Option<String> {
        lock(&self.lifecycle).terminal.clone()
    }

    /// 泵观察到的全部事件条数。
    pub(crate) fn events_seen(&self) -> usize {
        self.events_seen.load(Ordering::Relaxed)
    }

    /// 泵观察到的 `log` 事件条数。
    pub(crate) fn log_event_count(&self) -> usize {
        self.log_events.load(Ordering::Relaxed)
    }

    /// 无人认领的 `requestCompleted` 条数。
    pub(crate) fn unclaimed_count(&self) -> usize {
        self.unclaimed.load(Ordering::Relaxed)
    }

    /// 某个 `uri` 的**最近一次**诊断快照。
    ///
    /// 返回空有两种含义，取值上不区分：① 服务端明确报过"这个文件没有诊断"（空数组也是
    /// 一次发布，LSP 就是这么表达"清空"）；② 还没收到过这个 `uri` 的任何发布。
    /// 需要区分时看 `S1_JAVA_DIAGNOSTICS` 诊断行。
    pub(crate) fn latest_diagnostics(&self, uri: &str) -> Vec<JavaDiagnostic> {
        lock(&self.diagnostics)
            .get(uri)
            .cloned()
            .unwrap_or_default()
    }

    // -----------------------------------------------------------------------
    // 泵侧
    // -----------------------------------------------------------------------

    /// 分派一个事件。**只有泵调它**（也就是 `lsp.waitEvents` 的唯一出口）。
    pub(crate) fn dispatch(&self, event: &Value) {
        self.events_seen.fetch_add(1, Ordering::Relaxed);
        let kind = event.get("type").and_then(Value::as_str).unwrap_or("unknown");
        let first_of_kind = self.note_kind(kind);
        match kind {
            "requestCompleted" => self.dispatch_completion(event),
            "diagnostics" => self.store_diagnostics(event),
            "log" => self.report_log(event),
            // 生命周期事件要给 `wait_ready` / `shutdown` 看，所以进队列而不是丢弃。
            "stateChanged" | "serverInfoChanged" => self.push_lifecycle(event),
            // 其余（`featuresChanged` / `semanticTokensRefresh` / Maven 相关…，见
            // `engine.rs:415-429`）：**记录**类型名再丢弃。静默丢弃会让"某个事件从来没到过"
            // 这种问题无从查起，而每类只打一次不会刷屏。
            other => {
                if first_of_kind {
                    println!("S1_JAVA_PUMP ignored type={other}");
                }
            }
        }
    }

    /// `requestCompleted` → 对应 `operationId` 的等待者。
    fn dispatch_completion(&self, event: &Value) {
        let Some(operation_id) = event.get("operationId").and_then(Value::as_str) else {
            self.unclaimed.fetch_add(1, Ordering::Relaxed);
            println!("S1_JAVA_PUMP unclaimed operationId=- reason=missing-operation-id");
            return;
        };
        // 每个请求最多完成一次（契约 `:1425-1429`），所以取走就删。
        let waiter = lock(&self.pending).remove(operation_id);
        match waiter {
            Some(sender) => {
                // 等待者可能已经因为自己的死线走人：`send` 失败不是错误，丢掉即可。
                let _ = sender.send(completion_outcome(event));
            }
            None => {
                self.unclaimed.fetch_add(1, Ordering::Relaxed);
                // 典型来源：请求方的死线（补全 15s）比 Core 的 `requestTimeoutMilliseconds`
                // （60s）短，超时后 Core 仍会把结果/`requestTimeout` 推回来。
                println!("S1_JAVA_PUMP unclaimed operationId={operation_id} reason=no-waiter");
            }
        }
    }

    /// `diagnostics` → 按 `uri` 归档最近一次快照。
    ///
    /// 版本不用我们筛：Core 在收到 `publishDiagnostics` 时已经丢掉"版本与当前文档不符"
    /// 的那些（`client.rs:353-356`），事件带的是它认可过的版本。
    fn store_diagnostics(&self, event: &Value) {
        let uri = event.get("uri").and_then(Value::as_str).unwrap_or_default();
        if uri.is_empty() {
            // Core 的形状里 `uri` 一定有（`client.rs:345-349` 会先校验），真缺了就没法归档。
            println!("S1_JAVA_DIAGNOSTICS ignored reason=missing-uri");
            return;
        }
        let items = event.get("diagnostics").and_then(Value::as_array);
        let parsed: Vec<JavaDiagnostic> = items
            .map(|items| items.iter().filter_map(parse_diagnostic).collect())
            .unwrap_or_default();
        let dropped = items.map(Vec::len).unwrap_or_default() - parsed.len();
        let version = event.get("version").and_then(Value::as_i64);

        // 空数组也是有效快照（= 服务端说"这个文件现在没有诊断"），照存。
        lock(&self.diagnostics).insert(uri.to_string(), parsed.clone());
        println!(
            "S1_JAVA_DIAGNOSTICS uri={uri} version={} count={} dropped={dropped}",
            version
                .map(|version| version.to_string())
                .unwrap_or_else(|| "-".to_string()),
            parsed.len(),
        );
    }

    /// `stateChanged` / `serverInfoChanged` → 生命周期队列；终态同时叫醒所有等待者。
    fn push_lifecycle(&self, event: &Value) {
        if event.get("type").and_then(Value::as_str) == Some("stateChanged") {
            if let Some(reason) = terminal_reason_for(event) {
                self.mark_terminal(reason);
            }
        }
        lock(&self.lifecycle).queued.push(event.clone());
        self.signal.notify_all();
    }

    /// JDT 的日志（项目导入 / 进度）→ `S1_JAVA_LOG`，连续重复只打一次。
    fn report_log(&self, event: &Value) {
        self.log_events.fetch_add(1, Ordering::Relaxed);
        let message = event
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let detail = event
            .get("detail")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let line = format!("{message} {detail}");
        let mut last_log = lock(&self.last_log);
        if last_log.as_deref() == Some(line.as_str()) {
            return;
        }
        let level = event.get("level").and_then(Value::as_str).unwrap_or("info");
        println!("S1_JAVA_LOG level={level} message={line}");
        *last_log = Some(line);
    }

    /// 记下事件类型并返回"是不是第一次见到它"。
    fn note_kind(&self, kind: &str) -> bool {
        let mut counts = lock(&self.counts);
        let entry = counts.entry(kind.to_string()).or_insert(0);
        *entry += 1;
        *entry == 1
    }

    /// 标记会话终态并叫醒所有等待者。
    ///
    /// 之后 `register` 会直接失败：泵要么已经退出，要么因为会话已死而马上退出，
    /// 让在飞请求白等自己的死线（补全 15s、定义 60s）没有任何意义。
    ///
    /// 锁序：先取 `lifecycle`、**放开**、再取 `pending`。与 `register` 的
    /// `pending → lifecycle` 不构成嵌套环。
    pub(crate) fn mark_terminal(&self, reason: String) {
        {
            let mut lifecycle = lock(&self.lifecycle);
            if lifecycle.terminal.is_none() {
                lifecycle.terminal = Some(reason);
            }
        }
        self.signal.notify_all();

        let waiters: Vec<Sender<Result<Value, String>>> =
            lock(&self.pending).drain().map(|(_, sender)| sender).collect();
        if waiters.is_empty() {
            return;
        }
        let reason = self
            .terminal_reason()
            .unwrap_or_else(|| "会话已终止".to_string());
        for sender in waiters {
            let _ = sender.send(Err(reason.clone()));
        }
    }

    /// 置停止标志。泵在下一次循环边界退出（≤ [`PUMP_SLICE`]）。
    pub(crate) fn request_stop(&self) {
        self.stopping.store(true, Ordering::Relaxed);
    }

    /// 等生命周期事件（不超过 `timeout`），返回并取走队列里的那些。
    ///
    /// 队列非空时立刻返回（`Duration::ZERO` 也一样），所以调用方可以先"取一批"再决定等多久。
    pub(crate) fn wait_lifecycle(&self, timeout: Duration) -> Vec<Value> {
        let mut lifecycle = lock(&self.lifecycle);
        if !lifecycle.queued.is_empty() {
            return std::mem::take(&mut lifecycle.queued);
        }
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Vec::new();
            }
            lifecycle = self
                .signal
                .wait_timeout(lifecycle, remaining)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
            if !lifecycle.queued.is_empty() {
                return std::mem::take(&mut lifecycle.queued);
            }
        }
    }

    /// 等到会话终态（有界）。返回是否真的到了终态。
    pub(crate) fn wait_terminal(&self, deadline: Instant) -> bool {
        let mut lifecycle = lock(&self.lifecycle);
        loop {
            if lifecycle.terminal.is_some() {
                return true;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return false;
            }
            lifecycle = self
                .signal
                .wait_timeout(lifecycle, remaining)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
    }
}

/// `requestCompleted` → 等待者要的结果。
///
/// ⚠️ **错误文本的形状是兼容面**：`"{code}@{stage}：{message}"`。`service.rs::is_superseded`
/// 靠 `staleDocumentVersion@` / `requestCancelled@` 前缀把"文档变旧"（快速打字时的常态，
/// 必须静默丢弃）与真失败分开。改这里必须同时改那边。
fn completion_outcome(event: &Value) -> Result<Value, String> {
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
    Ok(event.get("result").cloned().unwrap_or(Value::Null))
}

/// `stateChanged` 是不是终态；是就给出可读原因。
fn terminal_reason_for(event: &Value) -> Option<String> {
    match event.get("state").and_then(Value::as_str) {
        Some("stopped") => Some("JDTLS 会话已停止（stateChanged: stopped）".to_string()),
        Some("failed") => Some(format!(
            "JDTLS 会话失败（stateChanged: failed）：{}",
            event
                .get("detail")
                .or_else(|| event.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("（无详情，见 S1_JAVA_SESSION failed 那一行）")
        )),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 泵
// ---------------------------------------------------------------------------

/// 事件泵的句柄。`Session` 持有它。
///
/// 为什么用 `std::thread` 而不是 gpui 的 `Task`：gpui 的 `Task` 一 drop 就取消
/// （`gpui/PLAN.md` §15.1 踩过），而这个泵必须活到会话收尾；同时本 crate 刻意不依赖 gpui
/// （见 `lib.rs`），泵也不该成为第一个理由。`std::thread` 不会因为句柄被 drop 就停，
/// 所以**必须**在收尾时置停止标志并 join，否则会留一个永远卡在 `waitEvents` 上的线程
/// （调研 §7.2 第 1 条）。
pub(crate) struct EventPump {
    events: Arc<SessionEvents>,
    handle: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for EventPump {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EventPump")
            .field("running", &self.handle.is_some())
            .finish()
    }
}

impl EventPump {
    /// 起泵。`session_id` 只用于拼 `lsp.waitEvents` 的载荷与诊断行。
    pub(crate) fn start(session_id: String, events: Arc<SessionEvents>) -> Result<Self, String> {
        let worker_events = Arc::clone(&events);
        let label = session_id.clone();
        let handle = thread::Builder::new()
            .name(format!("lithe-java-pump-{label}"))
            .spawn(move || pump_loop(&session_id, &worker_events))
            .map_err(|error| format!("事件泵线程创建失败：{error}"))?;
        println!(
            "S1_JAVA_PUMP started sessionId={label} sliceMs={}",
            PUMP_SLICE.as_millis()
        );
        Ok(Self {
            events,
            handle: Some(handle),
        })
    }

    /// 置停止标志并 join。
    ///
    /// 有界：泵只在循环边界看标志，而它每次阻塞都有 [`PUMP_SLICE`] 的上限；`destroyServer`
    /// 之后 `waitEvents` 还会立刻报"会话已不在运行"（`engine.rs:1944-1958`），泵通常早就退出了。
    pub(crate) fn stop(&mut self) {
        self.events.request_stop();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for EventPump {
    fn drop(&mut self) {
        // 兜底：`Session` 在 `wait_ready` 失败等路径上可能没走 `shutdown()` 就被丢掉。
        // 少了这一步就会留一个永远等 `waitEvents` 的线程。
        self.stop();
    }
}

/// 泵的主体：**唯一**调 `lsp.waitEvents` 的地方。
fn pump_loop(session_id: &str, events: &Arc<SessionEvents>) {
    let reason = loop {
        if events.stopping.load(Ordering::Relaxed) {
            break "stopping".to_string();
        }
        match crate::session::wait_events(session_id, PUMP_SLICE) {
            Ok(batch) => {
                if batch.is_empty() {
                    continue;
                }
                for event in &batch {
                    events.dispatch(event);
                }
                // 终态之后 Core 会持续对 `waitEvents` 报错，泵没有继续转的理由。
                if events.terminal_reason().is_some() {
                    break "terminal".to_string();
                }
            }
            Err(error) => {
                // 任何 `waitEvents` 失败都视为会话不可用：它要么是终态
                // （`process_failed`，`engine.rs:1944-1958`），要么是信封超时/会话不存在。
                // 继续转下去只会把日志刷满，并让每个请求都等满自己的死线。
                break format!("waitEvents: {error}");
            }
        }
    };

    // 泵一退出就没有任何事件出口了：把终态钉死，让在飞请求立刻失败、新请求直接拒绝。
    events.mark_terminal(format!("事件泵已退出（{reason}）"));
    println!(
        "S1_JAVA_PUMP exit sessionId={session_id} reason={reason} eventsSeen={} unclaimed={}",
        events.events_seen(),
        events.unclaimed_count()
    );
}

/// 锁中毒只说明某个持有者 panic 过，数据本身仍可用：照仓库既有写法取内层值。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---------------------------------------------------------------------------
// 测试：分派与解析（纯函数 + 共享账本，不起线程、不等时钟）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// 一次诊断发布能被解析并归档，字段与 Core 的归一化输出一一对应。
    #[test]
    fn diagnostics_event_is_parsed_and_stored() {
        let events = SessionEvents::new();
        events.dispatch(&json!({
            "type": "diagnostics",
            "sequence": 7,
            "providerId": "java",
            "sessionId": "s-1",
            "uri": "file:///ws/src/demo/App.java",
            "version": 12,
            "diagnostics": [{
                "range": {
                    "start": { "line": 4, "utf16Column": 8 },
                    "end": { "line": 4, "utf16Column": 16 }
                },
                "severity": 1,
                "message": "Greeter 无法解析为类型",
                "source": "Java",
                "code": "268435846",
                "tags": [1],
            }],
        }));

        let diagnostics = events.latest_diagnostics("file:///ws/src/demo/App.java");
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.severity, Some(1));
        assert_eq!(diagnostic.message, "Greeter 无法解析为类型");
        assert_eq!(diagnostic.source.as_deref(), Some("Java"));
        assert_eq!(diagnostic.code.as_deref(), Some("268435846"));
        assert_eq!(diagnostic.tags, vec![1]);
        assert_eq!(diagnostic.range.start, JavaPosition { line: 4, utf16_column: 8 });
        assert_eq!(diagnostic.range.end, JavaPosition { line: 4, utf16_column: 16 });
        // 别的 uri 不受影响（分派是按 uri 归档的，不是"最近一条"）。
        assert!(events.latest_diagnostics("file:///ws/src/demo/Greeter.java").is_empty());
    }

    /// 缺字段不 panic：没有 `range` 的条目被丢掉，`code` 支持数字，空数组 = 一次"清空"发布。
    #[test]
    fn diagnostics_tolerate_missing_fields_and_clear_on_empty() {
        let events = SessionEvents::new();
        let uri = "file:///ws/src/demo/App.java";
        events.dispatch(&json!({
            "type": "diagnostics",
            "uri": uri,
            "diagnostics": [
                { "message": "没有 range，丢掉", "severity": 2 },
                { "range": { "start": { "line": 0, "utf16Column": 0 } }, "message": "没有 end，丢掉" },
                {
                    "range": {
                        "start": { "line": 1, "utf16Column": 0 },
                        "end": { "line": 1, "utf16Column": 3 }
                    },
                    "code": 12345,
                },
            ],
        }));
        let diagnostics = events.latest_diagnostics(uri);
        assert_eq!(diagnostics.len(), 1, "只有最后一条有完整的 range");
        // `message` 缺失按空串（与 Core 的 `parse_diagnostics` 同宽容度），`code` 数字转字符串。
        assert_eq!(diagnostics[0].message, "");
        assert_eq!(diagnostics[0].code.as_deref(), Some("12345"));
        assert_eq!(diagnostics[0].severity, None);

        // 事件形态不对（没有 uri / 没有 diagnostics 数组）也不许 panic。
        events.dispatch(&json!({ "type": "diagnostics" }));
        events.dispatch(&json!({ "type": "diagnostics", "uri": uri }));
        assert!(events.latest_diagnostics(uri).is_empty(), "第二次发布是空数组 = 清空");

        // 空数组发布确实会把上一份快照替换掉。
        events.dispatch(&json!({
            "type": "diagnostics",
            "uri": uri,
            "diagnostics": [{
                "range": {
                    "start": { "line": 2, "utf16Column": 0 },
                    "end": { "line": 2, "utf16Column": 1 }
                },
                "message": "又有了"
            }],
        }));
        assert_eq!(events.latest_diagnostics(uri).len(), 1);
        events.dispatch(&json!({ "type": "diagnostics", "uri": uri, "diagnostics": [] }));
        assert!(events.latest_diagnostics(uri).is_empty());
    }

    /// **核心回归**：两个在飞请求各自只拿到自己的 `requestCompleted`，第三方订阅者的事件
    /// 不会打断它们，也不会被谁偷走。
    #[test]
    fn request_completed_routes_by_operation_id() {
        let events = SessionEvents::new();
        let first = events.register("op-1").expect("注册 op-1");
        let second = events.register("op-2").expect("注册 op-2");

        // 顺序故意反着来（op-2 先完成），确保分派认的是 operationId 而不是到达顺序。
        // 中间还插一条诊断发布：第三方订阅者的事件不许打断、也不许被谁偷走。
        events.dispatch(&json!({
            "type": "diagnostics",
            "uri": "file:///ws/App.java",
            "diagnostics": [],
        }));
        events.dispatch(&json!({
            "type": "requestCompleted",
            "operationId": "op-2",
            "result": { "items": [] },
        }));
        events.dispatch(&json!({
            "type": "requestCompleted",
            "operationId": "op-1",
            "result": { "locations": [{ "uri": "file:///ws/Greeter.java" }] },
        }));

        assert_eq!(
            first.recv_timeout(Duration::from_secs(1)).expect("op-1 有结果"),
            Ok(json!({ "locations": [{ "uri": "file:///ws/Greeter.java" }] })),
        );
        assert_eq!(
            second.recv_timeout(Duration::from_secs(1)).expect("op-2 有结果"),
            Ok(json!({ "items": [] })),
        );
        assert_eq!(events.unclaimed_count(), 0, "两个请求都被认领了");
    }

    /// 无人认领的结果（请求方已超时）只记一条诊断，不会串到别的等待者上。
    #[test]
    fn request_completed_for_an_unknown_operation_is_not_stolen() {
        let events = SessionEvents::new();
        let waiter = events.register("op-1").expect("注册 op-1");

        events.dispatch(&json!({
            "type": "requestCompleted",
            "operationId": "op-already-timed-out",
            "result": { "items": [{ "label": "别人的结果" }] },
        }));
        assert_eq!(events.unclaimed_count(), 1);
        events.dispatch(&json!({ "type": "requestCompleted", "result": Value::Null }));
        assert_eq!(events.unclaimed_count(), 2, "连 operationId 都没有的也算无人认领");

        events.dispatch(&json!({
            "type": "requestCompleted",
            "operationId": "op-1",
            "result": { "items": [] },
        }));
        assert_eq!(
            waiter.recv_timeout(Duration::from_secs(1)).expect("op-1 有结果"),
            Ok(json!({ "items": [] })),
        );
    }

    /// 错误文本形状是兼容面：`service.rs::is_superseded` 靠这两个前缀判定"被取代"。
    #[test]
    fn request_error_keeps_the_code_at_stage_prefix() {
        let events = SessionEvents::new();
        let waiter = events.register("op-1").expect("注册 op-1");
        events.dispatch(&json!({
            "type": "requestCompleted",
            "operationId": "op-1",
            "error": {
                "code": "staleDocumentVersion",
                "stage": "document",
                "message": "文档版本已变旧",
            },
        }));
        let error = waiter
            .recv_timeout(Duration::from_secs(1))
            .expect("op-1 有结果")
            .expect_err("带 error 的完成必须是 Err");
        assert_eq!(error, "staleDocumentVersion@document：文档版本已变旧");
        assert!(error.starts_with("staleDocumentVersion@"));
    }

    /// 同一 `operationId` 重复注册要被拦住（Core 侧同样是 `invalid_request`）。
    #[test]
    fn duplicate_operation_ids_are_rejected() {
        let events = SessionEvents::new();
        let _first = events.register("op-1").expect("第一次注册");
        assert!(events.register("op-1").is_err());
        events.unregister("op-1");
        assert!(events.register("op-1").is_ok(), "注销之后可以复用这个 id");
    }

    /// 日志去重只影响打印，计数照记；生命周期事件进队列，其余类型只记一次。
    #[test]
    fn logs_are_counted_and_lifecycle_events_are_queued() {
        let events = SessionEvents::new();
        for _ in 0..3 {
            events.dispatch(&json!({
                "type": "log",
                "level": "info",
                "message": "导入中",
                "detail": "",
            }));
        }
        assert_eq!(events.log_event_count(), 3);

        events.dispatch(&json!({ "type": "stateChanged", "state": "initializing" }));
        events.dispatch(&json!({
            "type": "serverInfoChanged",
            "serverInfo": { "name": "Eclipse JDT LS", "version": "1.61.0" },
        }));
        events.dispatch(&json!({ "type": "featuresChanged" }));
        let queued = events.wait_lifecycle(Duration::ZERO);
        assert_eq!(queued.len(), 2, "只有生命周期事件进队列：{queued:?}");
        assert_eq!(queued[0].get("state").and_then(Value::as_str), Some("initializing"));
        assert_eq!(events.events_seen(), 6);
    }

    /// 终态会叫醒 `wait_terminal`、并把在飞请求与后续注册一起拒掉。
    #[test]
    fn terminal_state_wakes_waiters_and_rejects_new_requests() {
        let events = SessionEvents::new();
        let waiter = events.register("op-1").expect("注册 op-1");
        assert!(!events.wait_terminal(Instant::now()), "还没终态、且死线已过 → false");

        events.dispatch(&json!({ "type": "stateChanged", "state": "failed", "detail": "JVM 起不来" }));
        assert!(events.wait_terminal(Instant::now()), "终态已记录 → 立刻 true");
        let outcome = waiter
            .recv_timeout(Duration::from_secs(1))
            .expect("在飞请求必须被叫醒，而不是等满自己的死线");
        assert!(
            outcome.expect_err("会话失败时不能给出结果").contains("JVM 起不来"),
            "终态原因要带到调用方"
        );

        let rejected = events.register("op-2").expect_err("终态之后不再接受新请求");
        assert!(rejected.contains("JVM 起不来"));
    }
}
