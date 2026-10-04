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

pub struct DebugSessionHandle {
    session_id: String,
    driver: crate::debug_transport::DebugSessionDriver,
    snapshot: DebugSessionSnapshot,
    last_exit: Option<String>,
    total_events: u64,
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
}
