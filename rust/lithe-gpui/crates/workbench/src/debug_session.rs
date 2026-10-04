use serde_json::Value;

pub const DEBUG_SESSION_TAG: &str = "S1_DEBUG_SESSION";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DebugSessionSnapshot {
    pub state: Option<String>,
    pub paused_reason: Option<String>,
    pub paused_thread_id: Option<i64>,
    pub paused_description: Option<String>,
    pub output: Vec<String>,
    pub terminated: bool,
    pub exit_code: Option<i64>,
    pub last_error: Option<String>,
    pub results: std::collections::BTreeMap<String, Value>,
    pub applied_events: u64,
}

impl DebugSessionSnapshot {
    pub const MAX_OUTPUT_LINES: usize = 500;

    pub fn apply(&mut self, events: &[Value]) -> bool {
        let mut changed = false;
        for event in events {
            changed |= self.apply_one(event);
        }
        if changed {
            println!(
                "{DEBUG_SESSION_TAG} snapshot_changed state={:?} terminated={} output_lines={} error={:?}",
                self.state,
                self.terminated,
                self.output.len(),
                self.last_error
            );
        }
        changed
    }

    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.state.as_deref() == Some("paused")
    }

    fn apply_one(&mut self, event: &Value) -> bool {
        self.applied_events += 1;
        let Some(kind) = text(event, "type") else {
            return false;
        };
        match kind.as_str() {
            "stateChanged" => {
                let Some(state) = text(event, "state") else {
                    return false;
                };
                let terminated = state == "terminated" || self.terminated;
                let changed =
                    self.state.as_deref() != Some(state.as_str()) || self.terminated != terminated;
                self.terminated = terminated;
                self.state = Some(state);
                changed
            }
            "breakpoint" => {
                println!(
                    "{} breakpoint_event verified={} reason={} source={}",
                    DEBUG_SESSION_TAG,
                    event
                        .pointer("/body/breakpoint/verified")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    event
                        .pointer("/body/reason")
                        .and_then(Value::as_str)
                        .unwrap_or("-"),
                    event
                        .pointer("/body/breakpoint/source/path")
                        .and_then(Value::as_str)
                        .unwrap_or("-"),
                );
                false
            }
            "stopped" => {
                let reason = text(event, "reason");
                let thread_id = event.get("threadId").and_then(Value::as_i64);
                let description = text(event, "description");
                let changed = self.state.as_deref() != Some("paused")
                    || self.paused_reason != reason
                    || self.paused_thread_id != thread_id
                    || self.paused_description != description;
                self.state = Some("paused".to_string());
                self.paused_reason = reason;
                self.paused_thread_id = thread_id;
                self.paused_description = description;
                changed
            }
            "continued" => {
                let changed = self.state.as_deref() != Some("running")
                    || self.paused_reason.is_some()
                    || self.paused_thread_id.is_some()
                    || self.paused_description.is_some();
                self.state = Some("running".to_string());
                self.paused_reason = None;
                self.paused_thread_id = None;
                self.paused_description = None;
                changed
            }
            "terminated" => {
                let exit_code = event.get("exitCode").and_then(Value::as_i64);
                let changed = !self.terminated
                    || self.state.as_deref() != Some("terminated")
                    || self.exit_code != exit_code;
                self.terminated = true;
                self.state = Some("terminated".to_string());
                self.exit_code = exit_code;
                changed
            }
            "output" => {
                let Some(line) = text(event, "output") else {
                    return false;
                };
                self.output.push(line);
                let overflow = self.output.len().saturating_sub(Self::MAX_OUTPUT_LINES);
                if overflow > 0 {
                    self.output.drain(..overflow);
                }
                true
            }
            "operationCompleted" => {
                let Some(result) = event.get("result") else {
                    return false;
                };
                let Some(kind) = result.get("kind").and_then(Value::as_str) else {
                    return false;
                };
                let key = match result.get("reference").and_then(Value::as_i64) {
                    Some(reference) if reference > 0 => format!("{kind}#{reference}"),
                    _ => kind.to_string(),
                };
                let changed = self.results.get(&key) != Some(result);
                self.results.insert(key, result.clone());
                changed
            }
            "operationFailed" => {
                let command = text(event, "command").unwrap_or_default();
                let message = text(event, "message").unwrap_or_default();
                let error = format!("{command}: {message}");
                let changed = self.last_error.as_deref() != Some(error.as_str());
                self.last_error = Some(error);
                changed
            }
            _ => false,
        }
    }
}

fn text(event: &Value, key: &str) -> Option<String> {
    event.get(key).and_then(Value::as_str).map(str::to_string)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DrainOutcome {
    pub changed: bool,
    pub exited: Option<String>,
    pub events: u64,
}

pub fn copy_new_output(
    snapshot: &DebugSessionSnapshot,
    seen: &mut usize,
    target: &mut Vec<String>,
) -> usize {
    if *seen > snapshot.output.len() {
        *seen = snapshot.output.len();
        return 0;
    }
    let start = *seen;
    target.extend(snapshot.output[start..].iter().cloned());
    *seen = snapshot.output.len();
    snapshot.output.len() - start
}

pub fn drain_driver(
    driver: &crate::debug_transport::DebugSessionDriver,
    snapshot: &mut DebugSessionSnapshot,
) -> DrainOutcome {
    use crate::debug_transport::DebugDriverEvent;

    let mut outcome = DrainOutcome::default();
    while let Some(event) = driver.try_recv() {
        match event {
            DebugDriverEvent::Events(events) => {
                outcome.events += events.len() as u64;
                outcome.changed |= snapshot.apply(&events);
            }
            DebugDriverEvent::Exited { reason } => {
                outcome.exited = Some(reason);
                break;
            }
        }
    }
    outcome
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InspectStage {
    pub threads_requested: bool,
    pub stack_requested: bool,
    pub scopes_requested: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectSpec {
    pub kind: &'static str,
    pub thread_id: Option<i64>,
    pub frame_id: Option<i64>,
    pub variables_reference: Option<i64>,
}

pub fn next_inspect(snapshot: &DebugSessionSnapshot, stage: &InspectStage) -> Option<InspectSpec> {
    if snapshot.state.as_deref() != Some("paused") {
        return None;
    }
    if !stage.threads_requested {
        return Some(InspectSpec {
            kind: "threads",
            thread_id: None,
            frame_id: None,
            variables_reference: None,
        });
    }
    let thread_id = snapshot.paused_thread_id;
    if !stage.stack_requested {
        return Some(InspectSpec {
            kind: "stackTrace",
            thread_id,
            frame_id: None,
            variables_reference: None,
        });
    }
    if !stage.scopes_requested {
        let frame_id = first_frame_id(snapshot)?;
        return Some(InspectSpec {
            kind: "scopes",
            thread_id,
            frame_id: Some(frame_id),
            variables_reference: None,
        });
    }
    None
}

pub fn first_frame_id(snapshot: &DebugSessionSnapshot) -> Option<i64> {
    let result = snapshot.results.get("stackTrace")?;
    for key in ["frames", "stackFrames"] {
        if let Some(id) = result
            .get(key)
            .and_then(Value::as_array)
            .and_then(|frames| frames.first())
            .and_then(|frame| frame.get("id"))
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
        {
            return Some(id);
        }
    }
    None
}

pub const DEBUG_INSPECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub fn result_for<'a>(snapshot: &'a DebugSessionSnapshot, kind: &str) -> Option<&'a Value> {
    if let Some(result) = snapshot.results.get(kind) {
        return Some(result);
    }
    let prefix = format!("{kind}#");
    snapshot
        .results
        .iter()
        .find(|(key, _)| key.starts_with(&prefix))
        .map(|(_, result)| result)
}

pub fn result_for_reference<'a>(
    snapshot: &'a DebugSessionSnapshot,
    kind: &str,
    reference: i64,
) -> Option<&'a Value> {
    snapshot.results.get(&format!("{kind}#{reference}"))
}

pub fn result_array<'a>(snapshot: &'a DebugSessionSnapshot, kind: &str, key: &str) -> &'a [Value] {
    result_for(snapshot, kind)
        .and_then(|result| result.get(key))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

pub fn variable_labels_for(snapshot: &DebugSessionSnapshot, reference: i64) -> Vec<String> {
    result_for_reference(snapshot, "variables", reference)
        .and_then(|result| result.get("variables"))
        .and_then(Value::as_array)
        .map(|variables| variables.iter().map(variable_label).collect())
        .unwrap_or_default()
}

pub fn thread_labels(snapshot: &DebugSessionSnapshot) -> Vec<String> {
    result_array(snapshot, "threads", "threads")
        .iter()
        .map(|thread| {
            let name = thread
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let id = thread.get("id").and_then(Value::as_i64).unwrap_or_default();
            if name.is_empty() {
                format!("#{id}")
            } else {
                format!("{name} (#{id})")
            }
        })
        .collect()
}

pub fn frame_labels(snapshot: &DebugSessionSnapshot) -> Vec<String> {
    let frames = if result_array(snapshot, "stackTrace", "frames").is_empty() {
        result_array(snapshot, "stackTrace", "stackFrames")
    } else {
        result_array(snapshot, "stackTrace", "frames")
    };
    frames
        .iter()
        .map(|frame| {
            let name = frame
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let source = frame
                .get("source")
                .and_then(|source| source.get("path").or_else(|| source.get("name")))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let line = frame.get("line").and_then(Value::as_i64);
            match (source.is_empty(), line) {
                (false, Some(line)) => format!("{name} — {source}:{line}"),
                (false, None) => format!("{name} — {source}"),
                (true, _) => name.to_string(),
            }
        })
        .collect()
}

pub fn scope_labels(snapshot: &DebugSessionSnapshot) -> Vec<(String, i64)> {
    result_array(snapshot, "scopes", "scopes")
        .iter()
        .filter_map(|scope| {
            let reference = scope.get("variablesReference").and_then(Value::as_i64)?;
            if reference <= 0 {
                return None;
            }
            let name = scope
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            Some((name.to_string(), reference))
        })
        .collect()
}

pub fn variable_labels(snapshot: &DebugSessionSnapshot) -> Vec<String> {
    result_array(snapshot, "variables", "variables")
        .iter()
        .map(variable_label)
        .collect()
}

fn variable_label(variable: &Value) -> String {
    let name = variable
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let value = variable
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let reference = variable
        .get("variablesReference")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    match (value.is_empty(), reference > 0) {
        (false, true) => format!("{name} = {value} ›"),
        (false, false) => format!("{name} = {value}"),
        (true, _) => name.to_string(),
    }
}

pub struct DebugSessionHandle {
    session_id: String,
    driver: crate::debug_transport::DebugSessionDriver,
    snapshot: DebugSessionSnapshot,
    last_exit: Option<String>,
    total_events: u64,
    inspect_stage: InspectStage,
    last_state: Option<String>,
}

impl DebugSessionHandle {
    pub fn start(
        transport: crate::debug_transport::DebugTransport,
        poll_interval: std::time::Duration,
    ) -> Self {
        let session_id = transport.session_id().to_string();
        Self {
            session_id,
            driver: crate::debug_transport::DebugSessionDriver::spawn(transport, poll_interval),
            snapshot: DebugSessionSnapshot::default(),
            last_exit: None,
            total_events: 0,
            inspect_stage: InspectStage::default(),
            last_state: None,
        }
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn snapshot(&self) -> &DebugSessionSnapshot {
        &self.snapshot
    }

    #[must_use]
    pub fn last_exit(&self) -> Option<&str> {
        self.last_exit.as_deref()
    }

    #[must_use]
    pub fn total_events(&self) -> u64 {
        self.total_events
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        self.driver.is_running()
    }

    pub fn drain(&mut self) -> DrainOutcome {
        let outcome = drain_driver(&self.driver, &mut self.snapshot);
        self.total_events += outcome.events;
        if outcome.exited.is_some() {
            self.last_exit = outcome.exited.clone();
        }
        if self.snapshot.state != self.last_state {
            println!(
                "{DEBUG_SESSION_TAG} inspect_stage_reset from={:?} to={:?}",
                self.last_state, self.snapshot.state
            );
            self.inspect_stage = InspectStage::default();
            self.last_state = self.snapshot.state.clone();
        }
        if let Some(spec) = next_inspect(&self.snapshot, &self.inspect_stage) {
            match spec.kind {
                "threads" => self.inspect_stage.threads_requested = true,
                "stackTrace" => self.inspect_stage.stack_requested = true,
                "scopes" => self.inspect_stage.scopes_requested = true,
                _ => {}
            }
            let kind = spec.kind;
            match self.request_inspect(
                kind,
                spec.thread_id,
                spec.frame_id,
                spec.variables_reference,
                DEBUG_INSPECT_TIMEOUT,
            ) {
                Ok(()) => println!("{DEBUG_SESSION_TAG} inspect_requested kind={kind}"),
                Err(error) => {
                    eprintln!(
                        "{DEBUG_SESSION_TAG} inspect_request_failed kind={kind} error={error}"
                    )
                }
            }
        }
        outcome
    }

    pub fn stop(&mut self, timeout: std::time::Duration) -> bool {
        let stopped = self.driver.stop(timeout);
        self.drain();
        stopped
    }

    pub fn execute_command(
        &mut self,
        command: &str,
        thread_id: Option<i64>,
        timeout: std::time::Duration,
    ) -> Result<(), String> {
        let result = self.driver.execute(command, thread_id, timeout);
        self.drain();
        result
    }

    pub fn request_variables(
        &mut self,
        variables_reference: i64,
        frame_id: Option<i64>,
        timeout: std::time::Duration,
    ) -> Result<(), String> {
        self.request_inspect(
            "variables",
            None,
            frame_id,
            Some(variables_reference),
            timeout,
        )
    }

    pub fn request_command(
        &mut self,
        command: &str,
        thread_id: Option<i64>,
        timeout: std::time::Duration,
    ) -> Result<(), String> {
        let result = self.driver.request(command, thread_id, timeout);
        self.drain();
        result
    }

    pub fn request_inspect(
        &mut self,
        kind: &str,
        thread_id: Option<i64>,
        frame_id: Option<i64>,
        variables_reference: Option<i64>,
        timeout: std::time::Duration,
    ) -> Result<(), String> {
        let result =
            self.driver
                .request_inspect(kind, thread_id, frame_id, variables_reference, timeout);
        self.drain();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stopped_moves_the_snapshot_to_paused_and_reports_a_change() {
        let mut snapshot = DebugSessionSnapshot::default();
        let changed = snapshot.apply(&[json!({
            "sequence": 1,
            "type": "stopped",
            "reason": "breakpoint",
            "threadId": 7,
            "description": "Main.java:12"
        })]);
        assert!(changed, "第一次 stopped 必须报告变化，UI 才会重绘");
        assert!(snapshot.is_paused());
        assert_eq!(snapshot.paused_reason.as_deref(), Some("breakpoint"));
        assert_eq!(snapshot.paused_thread_id, Some(7));
        assert_eq!(snapshot.paused_description.as_deref(), Some("Main.java:12"));

        let unchanged = snapshot.apply(&[json!({
            "sequence": 2,
            "type": "stopped",
            "reason": "breakpoint",
            "threadId": 7,
            "description": "Main.java:12"
        })]);
        assert!(!unchanged, "内容相同的 stopped 不该触发无谓重绘");
    }

    #[test]
    fn continued_clears_the_paused_frame() {
        let mut snapshot = DebugSessionSnapshot::default();
        snapshot.apply(&[json!({ "type": "stopped", "reason": "step", "threadId": 1 })]);
        snapshot.apply(&[json!({ "type": "continued", "threadId": 1 })]);
        assert_eq!(snapshot.state.as_deref(), Some("running"));
        assert!(snapshot.paused_reason.is_none());
        assert!(snapshot.paused_thread_id.is_none());
    }

    #[test]
    fn output_is_bounded_and_keeps_the_newest_lines() {
        let mut snapshot = DebugSessionSnapshot::default();
        let events = (0..DebugSessionSnapshot::MAX_OUTPUT_LINES + 10)
            .map(|index| json!({ "type": "output", "category": "stdout", "output": format!("line {index}") }))
            .collect::<Vec<_>>();
        snapshot.apply(&events);
        assert_eq!(
            snapshot.output.len(),
            DebugSessionSnapshot::MAX_OUTPUT_LINES
        );
        assert_eq!(
            snapshot.output.last().map(String::as_str),
            Some(format!("line {}", DebugSessionSnapshot::MAX_OUTPUT_LINES + 9).as_str())
        );
        assert_eq!(snapshot.output.first().map(String::as_str), Some("line 10"));
    }

    #[test]
    fn terminated_records_the_exit_code_and_operation_failures_surface_as_errors() {
        let mut snapshot = DebugSessionSnapshot::default();
        snapshot.apply(&[json!({ "type": "terminated", "exitCode": 3 })]);
        assert!(snapshot.terminated);
        assert_eq!(snapshot.exit_code, Some(3));
        assert_eq!(snapshot.state.as_deref(), Some("terminated"));

        snapshot.apply(&[json!({
            "type": "operationFailed",
            "operationId": "op-1",
            "command": "stackTrace",
            "code": "timedOut",
            "message": "The adapter did not answer."
        })]);
        assert_eq!(
            snapshot.last_error.as_deref(),
            Some("stackTrace: The adapter did not answer.")
        );
    }

    #[test]
    fn unknown_event_kinds_are_ignored_without_reporting_a_change() {
        let mut snapshot = DebugSessionSnapshot::default();
        let changed = snapshot.apply(&[json!({ "type": "capabilities", "capabilities": {} })]);
        assert!(!changed);
        assert_eq!(snapshot.applied_events, 1, "未知事件也要计数，便于诊断");
    }

    struct FrameQueue {
        incoming: Vec<Vec<u8>>,
    }

    impl crate::debug_transport::DebugAdapterPort for FrameQueue {
        fn write_frame(&mut self, frame: &[u8]) -> Result<(), String> {
            let text = String::from_utf8_lossy(frame).to_string();
            let Some(start) = text.find("\r\n\r\n").map(|index| index + 4) else {
                return Ok(());
            };
            let Ok(message) = serde_json::from_str::<Value>(&text[start..]) else {
                return Ok(());
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
                "launch" | "configurationDone" | "next" | "continue" | "stepIn" | "stepOut" => {
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
                "stackTrace" => {
                    self.incoming.push(dap_frame(json!({
                        "seq": 105,
                        "type": "response",
                        "request_seq": seq,
                        "success": true,
                        "command": "stackTrace",
                        "body": { "stackFrames": [ { "id": 42, "name": "main", "line": 3 } ] }
                    })));
                }
                "scopes" => {
                    self.incoming.push(dap_frame(json!({
                        "seq": 106,
                        "type": "response",
                        "request_seq": seq,
                        "success": true,
                        "command": "scopes",
                        "body": { "scopes": [ { "name": "Locals", "variablesReference": 9 } ] }
                    })));
                }
                _ => {}
            }
            Ok(())
        }

        fn try_recv(&mut self) -> Option<Vec<u8>> {
            if self.incoming.is_empty() {
                None
            } else {
                Some(self.incoming.remove(0))
            }
        }

        fn kill(&mut self) {}
    }

    fn dap_frame(message: Value) -> Vec<u8> {
        let body = serde_json::to_vec(&message).expect("序列化 DAP 消息");
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend(body);
        frame
    }

    #[test]
    fn drain_driver_applies_events_then_reports_exit() {
        use crate::debug_transport::{DebugSessionDriver, DebugTransport};
        use std::time::{Duration, Instant};

        let adapter = FrameQueue {
            incoming: vec![dap_frame(json!({
                "seq": 1,
                "type": "event",
                "event": "output",
                "body": { "category": "stdout", "output": "hello from the debuggee\n" }
            }))],
        };
        let transport = DebugTransport::start(
            "session-drain",
            "java",
            r"D:\proj",
            false,
            Box::new(adapter),
        )
        .expect("起会话");
        let mut driver = DebugSessionDriver::spawn(transport, Duration::from_millis(20));
        let mut snapshot = DebugSessionSnapshot::default();

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut outcome = DrainOutcome::default();
        while Instant::now() < deadline && !outcome.changed {
            outcome = drain_driver(&driver, &mut snapshot);
        }
        assert!(
            outcome.changed,
            "adapter 发来的 output 事件必须变成快照变化"
        );
        assert!(
            snapshot
                .output
                .iter()
                .any(|line| line.contains("hello from the debuggee")),
            "控制台输出应当进入快照，实得：{:?}",
            snapshot.output
        );

        let quiet = drain_driver(&driver, &mut snapshot);
        assert!(!quiet.changed, "没有新事件时不该报告变化");
        assert!(quiet.exited.is_none(), "会话还在跑时不该报退出");

        assert!(
            driver.stop(Duration::from_secs(10)),
            "stop 必须在有界时间内停住读循环"
        );
        let mut exited = DrainOutcome::default();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && exited.exited.is_none() {
            exited = drain_driver(&driver, &mut snapshot);
        }
        assert_eq!(
            exited.exited.as_deref(),
            Some("stopped"),
            "主动停止之后必须能收到带原因的退出事件"
        );
    }

    #[test]
    fn session_handle_owns_the_snapshot_and_reports_exit() {
        use crate::debug_transport::DebugTransport;
        use std::time::{Duration, Instant};

        let adapter = FrameQueue {
            incoming: vec![dap_frame(json!({
                "seq": 1,
                "type": "event",
                "event": "output",
                "body": { "category": "stdout", "output": "handle says hi\n" }
            }))],
        };
        let transport = DebugTransport::start(
            "session-handle",
            "java",
            r"D:\proj",
            false,
            Box::new(adapter),
        )
        .expect("起会话");
        let mut handle = DebugSessionHandle::start(transport, Duration::from_millis(20));
        assert_eq!(handle.session_id(), "session-handle");
        assert!(handle.is_running());
        assert!(handle.last_exit().is_none());

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut changed = false;
        while Instant::now() < deadline && !changed {
            changed = handle.drain().changed;
        }
        assert!(changed, "句柄必须把 adapter 事件变成快照变化");
        assert!(
            handle
                .snapshot()
                .output
                .iter()
                .any(|line| line.contains("handle says hi")),
            "快照里应当有控制台输出"
        );
        assert!(handle.total_events() >= 1);

        assert!(handle.stop(Duration::from_secs(10)), "stop 必须有界成功");
        assert!(!handle.is_running());
        assert_eq!(
            handle.last_exit(),
            Some("stopped"),
            "stop 之后句柄要记住退出原因"
        );
    }

    #[test]
    fn handle_sends_a_step_command_and_still_forwards_events() {
        use crate::debug_transport::DebugTransport;
        use std::time::{Duration, Instant};

        let adapter = FrameQueue {
            incoming: Vec::new(),
        };
        let transport =
            DebugTransport::start("session-step", "java", r"D:\proj", false, Box::new(adapter))
                .expect("起会话");
        let mut handle = DebugSessionHandle::start(transport, Duration::from_millis(20));

        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && !handle.snapshot().is_paused() {
            handle.drain();
        }
        assert!(
            handle.snapshot().is_paused(),
            "自动应答的 adapter 应当把会话推到 paused"
        );
        let events_before = handle.total_events();

        handle
            .execute_command("next", Some(1), Duration::from_secs(5))
            .expect("单步命令必须拿到回执");
        assert!(
            handle.total_events() > events_before,
            "命令期间的事件仍要被转发（不能只拿回执丢事件）"
        );
        assert!(
            handle.snapshot().is_paused(),
            "单步之后脚本化 adapter 又停在了断点，快照应仍是 paused"
        );

        assert!(handle.stop(Duration::from_secs(10)), "stop 必须有界成功");
        assert_eq!(handle.last_exit(), Some("stopped"));
    }

    #[test]
    fn request_inspect_returns_immediately_and_the_result_lands_in_the_snapshot() {
        use crate::debug_transport::DebugTransport;
        use std::time::{Duration, Instant};

        let adapter = FrameQueue {
            incoming: Vec::new(),
        };
        let transport = DebugTransport::start(
            "session-inspect-command",
            "java",
            r"D:\proj",
            false,
            Box::new(adapter),
        )
        .expect("起会话");
        let mut handle = DebugSessionHandle::start(transport, Duration::from_millis(20));

        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && !handle.snapshot().is_paused() {
            handle.drain();
        }
        assert!(handle.snapshot().is_paused(), "先把会话推到 paused");

        handle
            .request_inspect("threads", Some(1), None, None, Duration::from_secs(5))
            .expect("非阻塞 inspect 必须能发出");

        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && !handle.snapshot().results.contains_key("threads") {
            handle.drain();
        }
        assert!(
            handle.snapshot().results.contains_key("threads"),
            "inspect 结果必须按 kind 落进快照，实得 kinds={:?}",
            handle.snapshot().results.keys().collect::<Vec<_>>()
        );

        assert!(handle.stop(Duration::from_secs(10)), "stop 必须有界成功");
    }
    #[test]
    fn paused_session_fetches_threads_stack_and_scopes_without_being_asked() {
        use crate::debug_transport::DebugTransport;
        use std::time::{Duration, Instant};

        let adapter = FrameQueue {
            incoming: Vec::new(),
        };
        let transport = DebugTransport::start(
            "session-auto-inspect",
            "java",
            r"D:\proj",
            false,
            Box::new(adapter),
        )
        .expect("起会话");
        let mut handle = DebugSessionHandle::start(transport, Duration::from_millis(20));

        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline && handle.snapshot().results.len() < 2 {
            handle.drain();
        }
        let kinds: Vec<&str> = handle
            .snapshot()
            .results
            .keys()
            .map(String::as_str)
            .collect();
        assert!(
            kinds.contains(&"threads") && kinds.contains(&"stackTrace"),
            "暂停后应当自动拉到线程与调用栈，实得：{kinds:?}"
        );
        println!(
            "{DEBUG_SESSION_TAG} auto_inspect_kinds={kinds:?} first_frame={:?}",
            first_frame_id(handle.snapshot())
        );

        assert!(handle.stop(Duration::from_secs(10)), "stop 必须有界成功");
    }

    #[test]
    fn scope_and_variable_labels_feed_the_tree() {
        let mut snapshot = DebugSessionSnapshot::default();
        assert!(scope_labels(&snapshot).is_empty());
        assert!(variable_labels(&snapshot).is_empty());

        snapshot.results.insert(
            "scopes".to_string(),
            json!({ "kind": "scopes", "scopes": [
                { "name": "Locals", "variablesReference": 9 },
                { "name": "Statics", "variablesReference": 0 }
            ] }),
        );
        assert_eq!(
            scope_labels(&snapshot),
            vec![("Locals".to_string(), 9)],
            "variablesReference 为 0 的作用域没有子项，不该进树"
        );

        snapshot.results.insert(
            "variables".to_string(),
            json!({ "kind": "variables", "variables": [
                { "name": "count", "value": "3", "variablesReference": 0 },
                { "name": "user", "value": "User@1", "variablesReference": 12 },
                { "name": "hidden" }
            ] }),
        );
        assert_eq!(
            variable_labels(&snapshot),
            vec!["count = 3", "user = User@1 ›", "hidden"],
            "可展开的变量要带 › 提示，缺值只显示名字"
        );
    }

    #[test]
    fn variables_of_different_references_do_not_overwrite_each_other() {
        let mut snapshot = DebugSessionSnapshot::default();
        snapshot.apply(&[json!({
            "type": "operationCompleted",
            "result": { "kind": "variables", "reference": 9, "variables": [
                { "name": "outer", "value": "1", "variablesReference": 0 }
            ] }
        })]);
        snapshot.apply(&[json!({
            "type": "operationCompleted",
            "result": { "kind": "variables", "reference": 12, "variables": [
                { "name": "inner", "value": "2", "variablesReference": 0 }
            ] }
        })]);

        assert_eq!(snapshot.results.len(), 2, "两个引用各存一份，互不覆盖");
        assert_eq!(variable_labels_for(&snapshot, 9), vec!["outer = 1"]);
        assert_eq!(variable_labels_for(&snapshot, 12), vec!["inner = 2"]);
        assert!(
            variable_labels_for(&snapshot, 99).is_empty(),
            "没拉过的引用必须是空，不能拿别的引用的结果冒充"
        );
    }

    #[test]
    fn labels_render_threads_and_frames_for_the_ui() {
        let mut snapshot = DebugSessionSnapshot::default();
        assert!(thread_labels(&snapshot).is_empty());
        assert!(frame_labels(&snapshot).is_empty());

        snapshot.results.insert(
            "threads".to_string(),
            json!({ "kind": "threads", "threads": [
                { "id": 1, "name": "main" },
                { "id": 2 }
            ] }),
        );
        assert_eq!(thread_labels(&snapshot), vec!["main (#1)", "#2"]);

        snapshot.results.insert(
            "stackTrace".to_string(),
            json!({ "kind": "stackTrace", "frames": [
                { "id": 42, "name": "main", "source": { "path": "src/Main.java" }, "line": 12 },
                { "id": 43, "name": "run" }
            ] }),
        );
        assert_eq!(
            frame_labels(&snapshot),
            vec!["main — src/Main.java:12", "run"]
        );

        snapshot.results.insert(
            "stackTrace".to_string(),
            json!({ "kind": "stackTrace", "stackFrames": [
                { "id": 7, "name": "tick", "source": { "name": "Clock.java" }, "line": 3 }
            ] }),
        );
        assert_eq!(
            frame_labels(&snapshot),
            vec!["tick — Clock.java:3"],
            "DAP 原生 stackFrames + source.name 也要能渲染"
        );
    }

    #[test]
    fn next_inspect_walks_threads_stack_and_scopes_once_each() {
        let mut snapshot = DebugSessionSnapshot::default();
        let mut stage = InspectStage::default();
        assert!(
            next_inspect(&snapshot, &stage).is_none(),
            "没有暂停时不该拉任何东西"
        );

        snapshot.state = Some("paused".to_string());
        snapshot.paused_thread_id = Some(1);
        let first = next_inspect(&snapshot, &stage).expect("暂停后先拉线程");
        assert_eq!(first.kind, "threads");
        stage.threads_requested = true;

        let second = next_inspect(&snapshot, &stage).expect("再拉调用栈");
        assert_eq!(second.kind, "stackTrace");
        assert_eq!(second.thread_id, Some(1));
        stage.stack_requested = true;

        snapshot.results.insert(
            "stackTrace".to_string(),
            json!({ "kind": "stackTrace", "frames": [ { "id": 42, "name": "main" } ] }),
        );
        let third = next_inspect(&snapshot, &stage).expect("最后拉作用域");
        assert_eq!(third.kind, "scopes");
        assert_eq!(third.frame_id, Some(42), "作用域必须带可用帧 id");
        stage.scopes_requested = true;
        assert!(
            next_inspect(&snapshot, &stage).is_none(),
            "三步都拉过之后不该重复拉"
        );

        snapshot.results.insert(
            "stackTrace".to_string(),
            json!({ "kind": "stackTrace", "stackFrames": [ { "id": 7 } ] }),
        );
        assert_eq!(
            first_frame_id(&snapshot),
            Some(7),
            "DAP 原生字段名 stackFrames 也要认"
        );

        snapshot.results.insert(
            "stackTrace".to_string(),
            json!({ "kind": "stackTrace", "frames": [ { "id": 0, "name": "synthetic" } ] }),
        );
        assert_eq!(
            first_frame_id(&snapshot),
            None,
            "0/负数不是可用帧 id（Core 会拒 frameId<0），此时不许发 scopes"
        );
        stage.scopes_requested = false;
        assert!(
            next_inspect(&snapshot, &stage).is_none(),
            "拿不到正数帧 id 时应当停下，而不是发一个 Core 必然拒绝的请求"
        );

        snapshot.state = Some("running".to_string());
        stage = InspectStage::default();
        assert!(
            next_inspect(&snapshot, &stage).is_none(),
            "继续运行之后不该再拉"
        );
    }

    #[test]
    fn operation_completed_results_are_stored_per_kind() {
        let mut snapshot = DebugSessionSnapshot::default();
        let changed = snapshot.apply(&[json!({
            "sequence": 3,
            "type": "operationCompleted",
            "operationId": "op-1",
            "result": { "kind": "threads", "threads": [ { "id": 1, "name": "main" } ] }
        })]);
        assert!(changed, "第一次结果必须报告变化");
        assert_eq!(
            snapshot.results["threads"]["threads"][0]["name"],
            json!("main")
        );

        let unchanged = snapshot.apply(&[json!({
            "sequence": 4,
            "type": "operationCompleted",
            "operationId": "op-1",
            "result": { "kind": "threads", "threads": [ { "id": 1, "name": "main" } ] }
        })]);
        assert!(!unchanged, "同一个 kind 的相同结果不该触发无谓重绘");

        snapshot.apply(&[json!({
            "sequence": 5,
            "type": "operationCompleted",
            "operationId": "op-2",
            "result": { "kind": "variables", "reference": 7, "variables": [] }
        })]);
        assert_eq!(snapshot.results.len(), 2, "不同 kind 各留一份最新结果");
        assert!(
            snapshot.results.contains_key("variables#7"),
            "带引用的结果按键 kind#reference 存，多个引用才不会互相覆盖"
        );
        assert!(
            result_for(&snapshot, "variables").is_some(),
            "按 kind 也要能找到"
        );
        assert!(result_for_reference(&snapshot, "variables", 7).is_some());
        assert!(result_for_reference(&snapshot, "variables", 99).is_none());

        let ignored = snapshot.apply(&[json!({
            "sequence": 6,
            "type": "operationCompleted",
            "operationId": "op-3"
        })]);
        assert!(!ignored, "没有 result 的事件要忽略而不是 panic");
    }

    #[test]
    fn copy_new_output_only_copies_the_unseen_tail() {
        let mut snapshot = DebugSessionSnapshot::default();
        let mut seen = 0;
        let mut target = Vec::new();
        snapshot.apply(&[
            json!({ "type": "output", "output": "line 1" }),
            json!({ "type": "output", "output": "line 2" }),
        ]);
        assert_eq!(copy_new_output(&snapshot, &mut seen, &mut target), 2);
        assert_eq!(target, vec!["line 1".to_string(), "line 2".to_string()]);
        assert_eq!(
            copy_new_output(&snapshot, &mut seen, &mut target),
            0,
            "重复 drain 不许把已经搬走的行再搬一遍"
        );
        assert_eq!(target.len(), 2);

        snapshot.apply(&[json!({ "type": "output", "output": "line 3" })]);
        assert_eq!(copy_new_output(&snapshot, &mut seen, &mut target), 1);
        assert_eq!(target.last().map(String::as_str), Some("line 3"));

        seen = snapshot.output.len() + 10;
        assert_eq!(
            copy_new_output(&snapshot, &mut seen, &mut target),
            0,
            "快照丢过旧行时不许重放"
        );
        assert_eq!(seen, snapshot.output.len(), "游标只夹回快照末尾");
    }

    #[test]
    fn request_command_returns_immediately_and_events_still_arrive() {
        use crate::debug_transport::DebugTransport;
        use std::time::{Duration, Instant};

        let adapter = FrameQueue {
            incoming: Vec::new(),
        };
        let transport = DebugTransport::start(
            "session-request",
            "java",
            r"D:\proj",
            false,
            Box::new(adapter),
        )
        .expect("起会话");
        let mut handle = DebugSessionHandle::start(transport, Duration::from_millis(20));

        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && !handle.snapshot().is_paused() {
            handle.drain();
        }
        assert!(handle.snapshot().is_paused(), "先把会话推到 paused");

        let started = Instant::now();
        handle
            .request_command("next", Some(1), Duration::from_secs(5))
            .expect("非阻塞命令必须能发出");
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(1),
            "UI 线程调用的命令不许等回执，实得：{elapsed:?}"
        );

        let events_before = handle.total_events();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && handle.total_events() == events_before {
            handle.drain();
        }
        assert!(
            handle.total_events() > events_before,
            "命令发出后事件仍必须由驱动器转发过来"
        );

        assert!(handle.stop(Duration::from_secs(10)), "stop 必须有界成功");
    }
}
