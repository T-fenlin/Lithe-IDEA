use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::service::{JavaPosition, parse_position};

const PUMP_SLICE: Duration = Duration::from_millis(1_000);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaDiagnostic {
    pub range: JavaDiagnosticRange,
    pub severity: Option<i64>,
    pub message: String,
    pub source: Option<String>,
    pub code: Option<String>,
    pub tags: Vec<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JavaDiagnosticRange {
    pub start: JavaPosition,
    pub end: JavaPosition,
}

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

#[derive(Debug)]
pub(crate) struct SessionEvents {
    pending: Mutex<HashMap<String, Sender<Result<Value, String>>>>,
    lifecycle: Mutex<Lifecycle>,
    signal: Condvar,
    diagnostics: Mutex<HashMap<String, Vec<JavaDiagnostic>>>,
    counts: Mutex<BTreeMap<String, usize>>,
    last_log: Mutex<Option<String>>,
    log_events: AtomicUsize,
    events_seen: AtomicUsize,
    unclaimed: AtomicUsize,
    stopping: AtomicBool,
}

#[derive(Debug, Default)]
struct Lifecycle {
    queued: Vec<Value>,
    terminal: Option<String>,
}

impl SessionEvents {
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

    pub(crate) fn register(
        &self,
        operation_id: &str,
    ) -> Result<Receiver<Result<Value, String>>, String> {
        let mut pending = lock(&self.pending);
        if let Some(reason) = lock(&self.lifecycle).terminal.clone() {
            return Err(reason);
        }
        if pending.contains_key(operation_id) {
            return Err(format!("operationId 重复：{operation_id}"));
        }
        let (sender, receiver) = channel();
        pending.insert(operation_id.to_string(), sender);
        Ok(receiver)
    }

    pub(crate) fn unregister(&self, operation_id: &str) {
        lock(&self.pending).remove(operation_id);
    }

    pub(crate) fn terminal_reason(&self) -> Option<String> {
        lock(&self.lifecycle).terminal.clone()
    }

    pub(crate) fn events_seen(&self) -> usize {
        self.events_seen.load(Ordering::Relaxed)
    }

    pub(crate) fn log_event_count(&self) -> usize {
        self.log_events.load(Ordering::Relaxed)
    }

    pub(crate) fn unclaimed_count(&self) -> usize {
        self.unclaimed.load(Ordering::Relaxed)
    }

    pub(crate) fn latest_diagnostics(&self, uri: &str) -> Vec<JavaDiagnostic> {
        lock(&self.diagnostics)
            .get(uri)
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn diagnostics_by_uri(&self) -> Vec<(String, Vec<JavaDiagnostic>)> {
        let mut snapshot: Vec<(String, Vec<JavaDiagnostic>)> = lock(&self.diagnostics)
            .iter()
            .filter(|(_, diagnostics)| !diagnostics.is_empty())
            .map(|(uri, diagnostics)| (uri.clone(), diagnostics.clone()))
            .collect();
        snapshot.sort_by(|left, right| left.0.cmp(&right.0));
        snapshot
    }

    pub(crate) fn dispatch(&self, event: &Value) {
        self.events_seen.fetch_add(1, Ordering::Relaxed);
        let kind = event
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let first_of_kind = self.note_kind(kind);
        match kind {
            "requestCompleted" => self.dispatch_completion(event),
            "diagnostics" => self.store_diagnostics(event),
            "log" => self.report_log(event),
            "stateChanged" | "serverInfoChanged" => self.push_lifecycle(event),
            other => {
                if first_of_kind {
                    println!("S1_JAVA_PUMP ignored type={other}");
                }
            }
        }
    }

    fn dispatch_completion(&self, event: &Value) {
        let Some(operation_id) = event.get("operationId").and_then(Value::as_str) else {
            self.unclaimed.fetch_add(1, Ordering::Relaxed);
            println!("S1_JAVA_PUMP unclaimed operationId=- reason=missing-operation-id");
            return;
        };
        let waiter = lock(&self.pending).remove(operation_id);
        match waiter {
            Some(sender) => {
                let _ = sender.send(completion_outcome(event));
            }
            None => {
                self.unclaimed.fetch_add(1, Ordering::Relaxed);
                println!("S1_JAVA_PUMP unclaimed operationId={operation_id} reason=no-waiter");
            }
        }
    }

    fn store_diagnostics(&self, event: &Value) {
        let uri = event.get("uri").and_then(Value::as_str).unwrap_or_default();
        if uri.is_empty() {
            println!("S1_JAVA_DIAGNOSTICS ignored reason=missing-uri");
            return;
        }
        let items = event.get("diagnostics").and_then(Value::as_array);
        let parsed: Vec<JavaDiagnostic> = items
            .map(|items| items.iter().filter_map(parse_diagnostic).collect())
            .unwrap_or_default();
        let dropped = items.map(Vec::len).unwrap_or_default() - parsed.len();
        let version = event.get("version").and_then(Value::as_i64);

        lock(&self.diagnostics).insert(uri.to_string(), parsed.clone());
        println!(
            "S1_JAVA_DIAGNOSTICS uri={uri} version={} count={} dropped={dropped}",
            version
                .map(|version| version.to_string())
                .unwrap_or_else(|| "-".to_string()),
            parsed.len(),
        );
    }

    fn push_lifecycle(&self, event: &Value) {
        if event.get("type").and_then(Value::as_str) == Some("stateChanged") {
            if let Some(reason) = terminal_reason_for(event) {
                self.mark_terminal(reason);
            }
        }
        lock(&self.lifecycle).queued.push(event.clone());
        self.signal.notify_all();
    }

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

    fn note_kind(&self, kind: &str) -> bool {
        let mut counts = lock(&self.counts);
        let entry = counts.entry(kind.to_string()).or_insert(0);
        *entry += 1;
        *entry == 1
    }

    pub(crate) fn mark_terminal(&self, reason: String) {
        {
            let mut lifecycle = lock(&self.lifecycle);
            if lifecycle.terminal.is_none() {
                lifecycle.terminal = Some(reason);
            }
        }
        self.signal.notify_all();

        let waiters: Vec<Sender<Result<Value, String>>> = lock(&self.pending)
            .drain()
            .map(|(_, sender)| sender)
            .collect();
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

    pub(crate) fn request_stop(&self) {
        self.stopping.store(true, Ordering::Relaxed);
    }

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

    pub(crate) fn stop(&mut self) {
        self.events.request_stop();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for EventPump {
    fn drop(&mut self) {
        self.stop();
    }
}

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
                if events.terminal_reason().is_some() {
                    break "terminal".to_string();
                }
            }
            Err(error) => {
                break format!("waitEvents: {error}");
            }
        }
    };

    events.mark_terminal(format!("事件泵已退出（{reason}）"));
    println!(
        "S1_JAVA_PUMP exit sessionId={session_id} reason={reason} eventsSeen={} unclaimed={}",
        events.events_seen(),
        events.unclaimed_count()
    );
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

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
        assert_eq!(
            diagnostic.range.start,
            JavaPosition {
                line: 4,
                utf16_column: 8
            }
        );
        assert_eq!(
            diagnostic.range.end,
            JavaPosition {
                line: 4,
                utf16_column: 16
            }
        );
        assert!(
            events
                .latest_diagnostics("file:///ws/src/demo/Greeter.java")
                .is_empty()
        );
    }

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
        assert_eq!(diagnostics[0].message, "");
        assert_eq!(diagnostics[0].code.as_deref(), Some("12345"));
        assert_eq!(diagnostics[0].severity, None);

        events.dispatch(&json!({ "type": "diagnostics" }));
        events.dispatch(&json!({ "type": "diagnostics", "uri": uri }));
        assert!(
            events.latest_diagnostics(uri).is_empty(),
            "第二次发布是空数组 = 清空"
        );

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

    #[test]
    fn request_completed_routes_by_operation_id() {
        let events = SessionEvents::new();
        let first = events.register("op-1").expect("注册 op-1");
        let second = events.register("op-2").expect("注册 op-2");

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
            first
                .recv_timeout(Duration::from_secs(1))
                .expect("op-1 有结果"),
            Ok(json!({ "locations": [{ "uri": "file:///ws/Greeter.java" }] })),
        );
        assert_eq!(
            second
                .recv_timeout(Duration::from_secs(1))
                .expect("op-2 有结果"),
            Ok(json!({ "items": [] })),
        );
        assert_eq!(events.unclaimed_count(), 0, "两个请求都被认领了");
    }

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
        assert_eq!(
            events.unclaimed_count(),
            2,
            "连 operationId 都没有的也算无人认领"
        );

        events.dispatch(&json!({
            "type": "requestCompleted",
            "operationId": "op-1",
            "result": { "items": [] },
        }));
        assert_eq!(
            waiter
                .recv_timeout(Duration::from_secs(1))
                .expect("op-1 有结果"),
            Ok(json!({ "items": [] })),
        );
    }

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

    #[test]
    fn duplicate_operation_ids_are_rejected() {
        let events = SessionEvents::new();
        let _first = events.register("op-1").expect("第一次注册");
        assert!(events.register("op-1").is_err());
        events.unregister("op-1");
        assert!(events.register("op-1").is_ok(), "注销之后可以复用这个 id");
    }

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
        assert_eq!(
            queued[0].get("state").and_then(Value::as_str),
            Some("initializing")
        );
        assert_eq!(events.events_seen(), 6);
    }

    #[test]
    fn terminal_state_wakes_waiters_and_rejects_new_requests() {
        let events = SessionEvents::new();
        let waiter = events.register("op-1").expect("注册 op-1");
        assert!(
            !events.wait_terminal(Instant::now()),
            "还没终态、且死线已过 → false"
        );

        events.dispatch(
            &json!({ "type": "stateChanged", "state": "failed", "detail": "JVM 起不来" }),
        );
        assert!(
            events.wait_terminal(Instant::now()),
            "终态已记录 → 立刻 true"
        );
        let outcome = waiter
            .recv_timeout(Duration::from_secs(1))
            .expect("在飞请求必须被叫醒，而不是等满自己的死线");
        assert!(
            outcome
                .expect_err("会话失败时不能给出结果")
                .contains("JVM 起不来"),
            "终态原因要带到调用方"
        );

        let rejected = events.register("op-2").expect_err("终态之后不再接受新请求");
        assert!(rejected.contains("JVM 起不来"));
    }
}
