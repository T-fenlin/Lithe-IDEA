use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, AppContext as _, Context, EventEmitter, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Task,
    Window, div, rems,
};

use crate::debug_session::{DebugSessionHandle, copy_new_output};
use lithe_gpui_editor::GutterBreakpoint;
use lithe_gpui_shared::icons::{idea, idea_icon_mono};
use lithe_gpui_shared::{core_json, tr, tr_args};
use serde_json::{Value, json};

pub const DEBUG_DIAGNOSTIC_TAG: &str = "S1_DEBUG";

const STEPPING_FILTERS_COMMAND: &str = "debug.steppingFilters";
const RELOCATE_BREAKPOINTS_COMMAND: &str = "debug.relocateBreakpoints";
#[allow(dead_code)]
const SET_BREAKPOINTS_COMMAND: &str = "debug.setBreakpoints";

const STEPPING_FILTERS_ADAPTER: &str = "java";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugBreakpoint {
    pub line: i64,
    pub column: Option<i64>,
    pub enabled: bool,
    pub verified: bool,
    pub message: Option<String>,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub log_message: Option<String>,
}

impl DebugBreakpoint {
    fn line(line: i64) -> Self {
        Self {
            line,
            column: None,
            enabled: true,
            verified: false,
            message: None,
            condition: None,
            hit_condition: None,
            log_message: None,
        }
    }

    fn to_payload(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("line".to_string(), json!(self.line));
        if let Some(column) = self.column {
            map.insert("column".to_string(), json!(column));
        }
        map.insert("enabled".to_string(), json!(self.enabled));
        insert_optional(&mut map, "condition", self.condition.as_deref());
        insert_optional(&mut map, "hitCondition", self.hit_condition.as_deref());
        insert_optional(&mut map, "logMessage", self.log_message.as_deref());
        Value::Object(map)
    }

    fn from_payload(value: &Value) -> Option<Self> {
        Some(Self {
            line: value.get("line")?.as_i64()?,
            column: value.get("column").and_then(Value::as_i64),
            enabled: value
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            verified: false,
            message: None,
            condition: string_field(value, "condition"),
            hit_condition: string_field(value, "hitCondition"),
            log_message: string_field(value, "logMessage"),
        })
    }
}

#[derive(Default, Clone, Debug)]
pub struct DebugBreakpointStore {
    by_file: BTreeMap<String, Vec<DebugBreakpoint>>,
    muted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebugBreakpointChange {
    Unchanged,
    File(String),
}

impl DebugBreakpointStore {
    pub fn file(&self, path: &str) -> &[DebugBreakpoint] {
        self.by_file.get(path).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn files(&self) -> impl Iterator<Item = &String> {
        self.by_file.keys()
    }

    pub fn total(&self) -> usize {
        self.by_file.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }

    pub fn muted(&self) -> bool {
        self.muted
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    pub fn toggle(&mut self, path: &str, line: i64) -> DebugBreakpointChange {
        if line < 1 {
            return DebugBreakpointChange::Unchanged;
        }
        let entry = self.by_file.entry(path.to_string()).or_default();
        match entry.iter().position(|breakpoint| breakpoint.line == line) {
            Some(index) => {
                entry.remove(index);
                if entry.is_empty() {
                    self.by_file.remove(path);
                }
            }
            None => {
                entry.push(DebugBreakpoint::line(line));
                entry.sort_by_key(|breakpoint| (breakpoint.line, breakpoint.column.unwrap_or(0)));
            }
        }
        DebugBreakpointChange::File(path.to_string())
    }

    pub fn set_enabled(&mut self, path: &str, line: i64, enabled: bool) -> DebugBreakpointChange {
        let Some(entry) = self.by_file.get_mut(path) else {
            return DebugBreakpointChange::Unchanged;
        };
        let Some(breakpoint) = entry.iter_mut().find(|item| item.line == line) else {
            return DebugBreakpointChange::Unchanged;
        };
        if breakpoint.enabled == enabled {
            return DebugBreakpointChange::Unchanged;
        }
        breakpoint.enabled = enabled;
        DebugBreakpointChange::File(path.to_string())
    }

    pub fn remove(&mut self, path: &str, line: i64) -> DebugBreakpointChange {
        let Some(entry) = self.by_file.get_mut(path) else {
            return DebugBreakpointChange::Unchanged;
        };
        let before = entry.len();
        entry.retain(|breakpoint| breakpoint.line != line);
        if entry.len() == before {
            return DebugBreakpointChange::Unchanged;
        }
        if entry.is_empty() {
            self.by_file.remove(path);
        }
        DebugBreakpointChange::File(path.to_string())
    }

    pub fn replace_file(
        &mut self,
        path: &str,
        breakpoints: Vec<DebugBreakpoint>,
    ) -> DebugBreakpointChange {
        let mut sorted = breakpoints;
        sorted.sort_by_key(|breakpoint| (breakpoint.line, breakpoint.column.unwrap_or(0)));
        sorted.dedup_by_key(|breakpoint| (breakpoint.line, breakpoint.column));
        if sorted.is_empty() {
            self.by_file.remove(path);
        } else {
            self.by_file.insert(path.to_string(), sorted);
        }
        DebugBreakpointChange::File(path.to_string())
    }

    pub fn clear(&mut self) -> DebugBreakpointChange {
        if self.by_file.is_empty() {
            return DebugBreakpointChange::Unchanged;
        }
        let paths: Vec<String> = self.by_file.keys().cloned().collect();
        self.by_file.clear();
        let _ = paths;
        DebugBreakpointChange::File(String::new())
    }

    pub fn set_breakpoints_payload(&self, session_id: &str, path: &str) -> Value {
        json!({
            "sessionId": session_id,
            "sourcePath": path,
            "breakpoints": self
                .file(path)
                .iter()
                .map(DebugBreakpoint::to_payload)
                .collect::<Vec<_>>(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugSourceEdit {
    pub start_utf16_offset: usize,
    pub end_utf16_offset: usize,
    pub replacement: String,
}

pub fn derive_source_edit(before: &str, after: &str) -> Option<DebugSourceEdit> {
    if before == after {
        return None;
    }
    let before_units: Vec<u16> = before.encode_utf16().collect();
    let after_units: Vec<u16> = after.encode_utf16().collect();

    let mut prefix = 0usize;
    while prefix < before_units.len()
        && prefix < after_units.len()
        && before_units[prefix] == after_units[prefix]
    {
        prefix += 1;
    }
    let mut suffix = 0usize;
    while suffix < before_units.len() - prefix
        && suffix < after_units.len() - prefix
        && before_units[before_units.len() - 1 - suffix]
            == after_units[after_units.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let replacement_units = &after_units[prefix..after_units.len() - suffix];
    Some(DebugSourceEdit {
        start_utf16_offset: prefix,
        end_utf16_offset: before_units.len() - suffix,
        replacement: String::from_utf16_lossy(replacement_units),
    })
}

pub fn relocate_breakpoints_payload(
    source_before: &str,
    edit: &DebugSourceEdit,
    breakpoints: &[DebugBreakpoint],
) -> Value {
    json!({
        "source": source_before,
        "edit": {
            "startUtf16Offset": edit.start_utf16_offset,
            "endUtf16Offset": edit.end_utf16_offset,
            "replacement": edit.replacement,
        },
        "breakpoints": breakpoints
            .iter()
            .map(DebugBreakpoint::to_payload)
            .collect::<Vec<_>>(),
    })
}

pub fn parse_relocate_result(value: &Value) -> Option<Vec<DebugBreakpoint>> {
    let items = value.get("breakpoints")?.as_array()?;
    Some(
        items
            .iter()
            .filter_map(DebugBreakpoint::from_payload)
            .collect(),
    )
}

pub fn set_exception_breakpoints_payload(session_id: &str, filters: &[String]) -> Value {
    json!({
        "sessionId": session_id,
        "breakpoints": filters
            .iter()
            .map(|filter| json!({ "filter": filter, "enabled": true }))
            .collect::<Vec<_>>(),
    })
}

pub fn set_function_breakpoints_payload(session_id: &str, names: &[String]) -> Value {
    json!({
        "sessionId": session_id,
        "breakpoints": names
            .iter()
            .map(|name| json!({ "name": name, "enabled": true }))
            .collect::<Vec<_>>(),
    })
}

pub fn stepping_filters_payload(adapter_id: &str) -> Value {
    json!({ "adapterId": adapter_id })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SteppingFiltersView {
    pub class_name_filters: Vec<String>,
    pub skip_synthetics: bool,
    pub skip_static_initializers: bool,
    pub skip_constructors: bool,
    pub hide_filtered_stack_frames: bool,
}

impl SteppingFiltersView {
    pub fn from_payload(value: &Value) -> Option<Self> {
        Some(Self {
            class_name_filters: value
                .get("classNameFilters")?
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
            skip_synthetics: value
                .get("skipSynthetics")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            skip_static_initializers: value
                .get("skipStaticInitializers")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            skip_constructors: value
                .get("skipConstructors")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            hide_filtered_stack_frames: value
                .get("hideFilteredStackFrames")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SteppingFilterToggle {
    SkipSynthetics,
    SkipConstructors,
    SkipStaticInitializers,
    HideFilteredStackFrames,
}

impl SteppingFilterToggle {
    pub fn value(self, filters: &SteppingFiltersView) -> bool {
        match self {
            Self::SkipSynthetics => filters.skip_synthetics,
            Self::SkipConstructors => filters.skip_constructors,
            Self::SkipStaticInitializers => filters.skip_static_initializers,
            Self::HideFilteredStackFrames => filters.hide_filtered_stack_frames,
        }
    }

    #[allow(dead_code)]
    fn set(self, filters: &mut SteppingFiltersView, value: bool) {
        match self {
            Self::SkipSynthetics => filters.skip_synthetics = value,
            Self::SkipConstructors => filters.skip_constructors = value,
            Self::SkipStaticInitializers => filters.skip_static_initializers = value,
            Self::HideFilteredStackFrames => filters.hide_filtered_stack_frames = value,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SteppingFiltersState {
    Idle,
    Loading,
    Ready(SteppingFiltersView),
    Failed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugTransportFacts {
    Absent,
    Present,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugSessionFacts {
    Absent,
    Present,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugStartBlocker {
    NoTransport,
    NoSession,
    NoTransportAndNoSession,
}

impl DebugStartBlocker {
    pub fn text_key(self) -> &'static str {
        match self {
            Self::NoTransport => "lithe.debugger.gpui.startBlockedNoTransport",
            Self::NoSession => "lithe.debugger.gpui.startBlockedNoSession",
            Self::NoTransportAndNoSession => "lithe.debugger.gpui.startBlockedBoth",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoTransport => "no-transport",
            Self::NoSession => "no-session",
            Self::NoTransportAndNoSession => "no-transport+no-session",
        }
    }
}

pub fn start_blocker(
    transport: DebugTransportFacts,
    session: DebugSessionFacts,
) -> Option<DebugStartBlocker> {
    match (transport, session) {
        (DebugTransportFacts::Present, DebugSessionFacts::Present) => None,
        (DebugTransportFacts::Absent, DebugSessionFacts::Absent) => {
            Some(DebugStartBlocker::NoTransportAndNoSession)
        }
        (DebugTransportFacts::Absent, DebugSessionFacts::Present) => {
            Some(DebugStartBlocker::NoTransport)
        }
        (DebugTransportFacts::Present, DebugSessionFacts::Absent) => {
            Some(DebugStartBlocker::NoSession)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugDispatchGate {
    pub session_id: Option<String>,
    pub transport: DebugTransportFacts,
}

impl DebugDispatchGate {
    pub fn disconnected() -> Self {
        Self {
            session_id: None,
            transport: DebugTransportFacts::Absent,
        }
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn can_dispatch(&self) -> bool {
        self.session_id.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugExecutionState {
    Idle,
    Running,
    Paused,
    Terminated,
}

impl DebugExecutionState {
    pub fn status_key(self) -> &'static str {
        match self {
            Self::Idle => "lithe.debugger.statusIdle",
            Self::Running => "lithe.debugger.statusRunning",
            Self::Paused => "lithe.debugger.statusPaused",
            Self::Terminated => "lithe.debugger.statusExited",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Terminated => "terminated",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugControl {
    Continue,
    Pause,
    StepOver,
    StepIn,
    StepOut,
    Restart,
    Terminate,
}

impl DebugControl {
    pub fn command(self) -> &'static str {
        match self {
            Self::Continue => "continue",
            Self::Pause => "pause",
            Self::StepOver => "next",
            Self::StepIn => "stepIn",
            Self::StepOut => "stepOut",
            Self::Restart => "restart",
            Self::Terminate => "terminate",
        }
    }

    pub fn payload(self, session_id: &str, operation_id: &str, thread_id: Option<i64>) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("sessionId".to_string(), json!(session_id));
        map.insert("operationId".to_string(), json!(operation_id));
        map.insert("command".to_string(), json!(self.command()));
        if let Some(thread_id) = thread_id {
            map.insert("threadId".to_string(), json!(thread_id));
        }
        map.insert("singleThread".to_string(), json!(false));
        Value::Object(map)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingExecution {
    pub operation_id: String,
    pub generation: u64,
    pub control: DebugControl,
}

#[derive(Clone, Debug, Default)]
pub struct DebugExecution {
    state: DebugExecutionState,
    generation: u64,
    pending: Option<PendingExecution>,
    stopped_reason: Option<String>,
    stopped_thread: Option<i64>,
}

impl Default for DebugExecutionState {
    fn default() -> Self {
        Self::Idle
    }
}

impl DebugExecution {
    pub fn state(&self) -> DebugExecutionState {
        self.state
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn pending(&self) -> Option<&PendingExecution> {
        self.pending.as_ref()
    }

    pub fn is_request_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub fn stopped_reason(&self) -> Option<&str> {
        self.stopped_reason.as_deref()
    }

    pub fn stopped_thread(&self) -> Option<i64> {
        self.stopped_thread
    }

    pub fn is_enabled(&self, control: DebugControl) -> bool {
        if self.is_request_pending() {
            return false;
        }
        match control {
            DebugControl::Restart | DebugControl::Terminate => {
                matches!(
                    self.state,
                    DebugExecutionState::Running | DebugExecutionState::Paused
                )
            }
            DebugControl::Continue => self.state == DebugExecutionState::Paused,
            DebugControl::Pause => self.state == DebugExecutionState::Running,
            DebugControl::StepOver | DebugControl::StepIn | DebugControl::StepOut => {
                self.state == DebugExecutionState::Paused && self.stopped_thread.is_some()
            }
        }
    }

    pub fn begin(&mut self, operation_id: &str, control: DebugControl) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some(PendingExecution {
            operation_id: operation_id.to_string(),
            generation: self.generation,
            control,
        });
        self.generation
    }

    pub fn accepts(&self, generation: u64) -> bool {
        self.generation == generation
    }

    pub fn on_stopped(&mut self, reason: &str, thread_id: Option<i64>) {
        self.state = DebugExecutionState::Paused;
        self.stopped_reason = (!reason.is_empty()).then(|| reason.to_string());
        self.stopped_thread = thread_id;
        self.pending = None;
    }

    pub fn on_continued(&mut self) {
        self.state = DebugExecutionState::Running;
        self.stopped_reason = None;
        self.stopped_thread = None;
        self.pending = None;
    }

    pub fn on_terminated(&mut self) {
        self.state = DebugExecutionState::Terminated;
        self.stopped_reason = None;
        self.stopped_thread = None;
        self.pending = None;
    }

    pub fn on_operation_failed(&mut self, operation_id: &str) -> bool {
        match self.pending.as_ref() {
            Some(pending) if pending.operation_id == operation_id => {
                self.pending = None;
                true
            }
            _ => false,
        }
    }

    pub fn status_text(&self) -> SharedString {
        match (self.state, self.stopped_reason.as_deref()) {
            (DebugExecutionState::Paused, Some(reason)) => {
                if let Some(label) = stop_reason_label(reason) {
                    let label = tr(label);
                    tr_args(
                        "lithe.debugger.gpui.pausedWithReason",
                        &[("reason", label.as_ref())],
                    )
                } else {
                    tr(self.state.status_key())
                }
            }
            _ => tr(self.state.status_key()),
        }
    }
}

fn stop_reason_label(reason: &str) -> Option<&'static str> {
    match reason.to_ascii_lowercase().as_str() {
        "breakpoint" => Some("lithe.debugger.breakpoints"),
        "pause" => Some("lithe.debugger.paused"),
        _ => None,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DebugUpdate {
    pub state: Option<DebugExecutionState>,
    pub events: Vec<DebugEventView>,
    pub outbound_frame_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebugEventView {
    StateChanged {
        state: DebugExecutionState,
    },
    Initialized,
    Capabilities,
    Output {
        category: Option<String>,
        output: String,
    },
    Stopped {
        reason: String,
        thread_id: Option<i64>,
    },
    Continued {
        thread_id: Option<i64>,
    },
    Terminated {
        exit_code: Option<i64>,
    },
    Breakpoint {
        verified: bool,
        message: Option<String>,
        line: Option<i64>,
    },
    OperationCompleted {
        operation_id: String,
        result: DebugOperationResultView,
    },
    OperationFailed {
        operation_id: String,
        message: String,
    },
    RunInTerminalRequested {
        request_id: String,
    },
    Unknown {
        kind: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebugOperationResultView {
    Threads(Vec<DebugThreadView>),
    StackTrace(Vec<DebugStackFrameView>),
    Scopes(Vec<DebugScopeView>),
    Variables(Vec<DebugVariableView>),
    Evaluate(Box<DebugVariableView>),
    ExceptionInfo {
        description: Option<String>,
        break_mode: String,
    },
    Acknowledged {
        command: String,
    },
    Other {
        kind: String,
        count: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugThreadView {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugStackFrameView {
    pub id: i64,
    pub name: String,
    pub source_path: Option<String>,
    pub line: i64,
    pub column: i64,
    pub is_filtered: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugScopeView {
    pub name: String,
    pub variables_reference: i64,
    pub expensive: bool,
    pub named_variables: i64,
    pub indexed_variables: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugVariableView {
    pub name: String,
    pub value: String,
    pub type_name: Option<String>,
    pub evaluate_name: Option<String>,
    pub variables_reference: i64,
    pub named_variables: i64,
    pub indexed_variables: i64,
}

impl DebugVariableView {
    pub fn is_expandable(&self) -> bool {
        self.variables_reference != 0
    }

    fn from_payload(value: &Value) -> Option<Self> {
        Some(Self {
            name: value.get("name")?.as_str()?.to_string(),
            value: value
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            type_name: string_field(value, "type"),
            evaluate_name: string_field(value, "evaluateName"),
            variables_reference: value
                .get("variablesReference")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            named_variables: value
                .get("namedVariables")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            indexed_variables: value
                .get("indexedVariables")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugVariableRow {
    pub depth: usize,
    pub variable: DebugVariableView,
}

#[derive(Default, Clone, Debug)]
pub struct DebugVariableTree {
    expanded: BTreeSet<i64>,
}

impl DebugVariableTree {
    pub fn is_expanded(&self, variables_reference: i64) -> bool {
        self.expanded.contains(&variables_reference)
    }

    pub fn toggle(&mut self, variables_reference: i64) -> bool {
        if self.expanded.contains(&variables_reference) {
            self.expanded.remove(&variables_reference);
            false
        } else {
            self.expanded.insert(variables_reference);
            true
        }
    }

    pub fn collapse_all(&mut self) {
        self.expanded.clear();
    }

    pub fn rows(&self, variables: &[DebugVariableView]) -> Vec<DebugVariableRow> {
        variables
            .iter()
            .map(|variable| DebugVariableRow {
                depth: 0,
                variable: variable.clone(),
            })
            .collect()
    }
}

pub fn parse_session_update(value: &Value) -> DebugUpdate {
    let mut update = DebugUpdate {
        state: value
            .get("state")
            .and_then(Value::as_str)
            .and_then(parse_execution_state),
        outbound_frame_count: value
            .get("outboundFrames")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        events: Vec::new(),
    };
    let Some(events) = value.get("events").and_then(Value::as_array) else {
        return update;
    };
    update.events = events.iter().filter_map(parse_event).collect();
    update
}

pub fn parse_execution_state(state: &str) -> Option<DebugExecutionState> {
    match state {
        "idle" => Some(DebugExecutionState::Idle),
        "running" | "launching" | "initializing" | "ready" => Some(DebugExecutionState::Running),
        "paused" => Some(DebugExecutionState::Paused),
        "terminated" | "terminating" | "failed" => Some(DebugExecutionState::Terminated),
        _ => None,
    }
}

fn parse_event(value: &Value) -> Option<DebugEventView> {
    let kind = value.get("type")?.as_str()?;
    Some(match kind {
        "stateChanged" => DebugEventView::StateChanged {
            state: value
                .get("state")
                .and_then(Value::as_str)
                .and_then(parse_execution_state)
                .unwrap_or(DebugExecutionState::Idle),
        },
        "initialized" => DebugEventView::Initialized,
        "capabilities" => DebugEventView::Capabilities,
        "output" => DebugEventView::Output {
            category: string_field(value, "category"),
            output: value
                .get("output")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        "stopped" => DebugEventView::Stopped {
            reason: value
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            thread_id: value.get("threadId").and_then(Value::as_i64),
        },
        "continued" => DebugEventView::Continued {
            thread_id: value.get("threadId").and_then(Value::as_i64),
        },
        "terminated" => DebugEventView::Terminated {
            exit_code: value.get("exitCode").and_then(Value::as_i64),
        },
        "breakpoint" => {
            let breakpoint = value.get("breakpoint")?;
            DebugEventView::Breakpoint {
                verified: breakpoint
                    .get("verified")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                message: string_field(breakpoint, "message"),
                line: breakpoint.get("line").and_then(Value::as_i64),
            }
        }
        "operationCompleted" => DebugEventView::OperationCompleted {
            operation_id: value
                .get("operationId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            result: value
                .get("result")
                .and_then(parse_operation_result)
                .unwrap_or(DebugOperationResultView::Other {
                    kind: "unknown".to_string(),
                    count: 0,
                }),
        },
        "operationFailed" => DebugEventView::OperationFailed {
            operation_id: value
                .get("operationId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            message: value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        "runInTerminalRequested" => DebugEventView::RunInTerminalRequested {
            request_id: value
                .get("requestId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        other => DebugEventView::Unknown {
            kind: other.to_string(),
        },
    })
}

fn parse_operation_result(value: &Value) -> Option<DebugOperationResultView> {
    let kind = value.get("kind")?.as_str()?;
    Some(match kind {
        "threads" => DebugOperationResultView::Threads(
            value
                .get("threads")?
                .as_array()?
                .iter()
                .filter_map(|thread| {
                    Some(DebugThreadView {
                        id: thread.get("id")?.as_i64()?,
                        name: thread
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    })
                })
                .collect(),
        ),
        "stackTrace" => DebugOperationResultView::StackTrace(
            value
                .get("stackFrames")?
                .as_array()?
                .iter()
                .filter_map(|frame| {
                    Some(DebugStackFrameView {
                        id: frame.get("id")?.as_i64()?,
                        name: frame
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        source_path: string_field(frame, "sourcePath"),
                        line: frame.get("line").and_then(Value::as_i64).unwrap_or(0),
                        column: frame.get("column").and_then(Value::as_i64).unwrap_or(0),
                        is_filtered: frame
                            .get("isFiltered")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect(),
        ),
        "scopes" => DebugOperationResultView::Scopes(
            value
                .get("scopes")?
                .as_array()?
                .iter()
                .filter_map(|scope| {
                    Some(DebugScopeView {
                        name: scope
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        variables_reference: scope
                            .get("variablesReference")
                            .and_then(Value::as_i64)
                            .unwrap_or(0),
                        expensive: scope
                            .get("expensive")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        named_variables: scope
                            .get("namedVariables")
                            .and_then(Value::as_i64)
                            .unwrap_or(0),
                        indexed_variables: scope
                            .get("indexedVariables")
                            .and_then(Value::as_i64)
                            .unwrap_or(0),
                    })
                })
                .collect(),
        ),
        "variables" => DebugOperationResultView::Variables(
            value
                .get("variables")?
                .as_array()?
                .iter()
                .filter_map(DebugVariableView::from_payload)
                .collect(),
        ),
        "evaluate" => DebugOperationResultView::Evaluate(Box::new(
            DebugVariableView::from_payload(value.get("variable")?)?,
        )),
        "exceptionInfo" => {
            let info = value.get("exceptionInfo")?;
            DebugOperationResultView::ExceptionInfo {
                description: string_field(info, "description"),
                break_mode: info
                    .get("breakMode")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            }
        }
        "acknowledged" => DebugOperationResultView::Acknowledged {
            command: value
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        other => DebugOperationResultView::Other {
            kind: other.to_string(),
            count: value
                .get(other)
                .and_then(Value::as_array)
                .map(Vec::len)
                .or_else(|| value.get("targets").and_then(Value::as_array).map(Vec::len))
                .unwrap_or(0),
        },
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DebugInspection {
    pub threads: Vec<DebugThreadView>,
    pub stack_frames: Vec<DebugStackFrameView>,
    pub scopes: Vec<DebugScopeView>,
    pub variables: Vec<DebugVariableView>,
    pub exception: Option<DebugExceptionView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugExceptionView {
    pub description: Option<String>,
    pub break_mode: String,
}

impl DebugInspection {
    pub fn adopt(&mut self, result: &DebugOperationResultView) {
        match result {
            DebugOperationResultView::Threads(threads) => self.threads = threads.clone(),
            DebugOperationResultView::StackTrace(frames) => self.stack_frames = frames.clone(),
            DebugOperationResultView::Scopes(scopes) => {
                self.scopes = scopes.clone();
                self.variables.clear();
            }
            DebugOperationResultView::Variables(variables) => self.variables = variables.clone(),
            DebugOperationResultView::ExceptionInfo {
                description,
                break_mode,
            } => {
                self.exception = Some(DebugExceptionView {
                    description: description.clone(),
                    break_mode: break_mode.clone(),
                });
            }
            _ => {}
        }
    }

    pub fn clear(&mut self) {
        self.threads.clear();
        self.stack_frames.clear();
        self.scopes.clear();
        self.variables.clear();
        self.exception = None;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugWatch {
    pub id: u64,
    pub expression: String,
    pub state: WatchState,
    pub value: Option<String>,
    pub type_name: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchState {
    NotEvaluated,
    Evaluating,
    Ready,
    Failed,
}

#[derive(Default, Clone, Debug)]
pub struct DebugWatches {
    items: Vec<DebugWatch>,
    next_id: u64,
    inflight: Vec<(String, u64)>,
}

impl DebugWatches {
    pub fn items(&self) -> &[DebugWatch] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn add(&mut self, expression: &str) -> Option<u64> {
        let expression = expression.trim();
        if expression.is_empty() {
            return None;
        }
        if self
            .items
            .iter()
            .any(|watch| watch.expression == expression)
        {
            return None;
        }
        self.next_id = self.next_id.wrapping_add(1);
        let id = self.next_id;
        self.items.push(DebugWatch {
            id,
            expression: expression.to_string(),
            state: WatchState::NotEvaluated,
            value: None,
            type_name: None,
            error: None,
        });
        Some(id)
    }

    pub fn remove(&mut self, id: u64) -> bool {
        let before = self.items.len();
        self.items.retain(|watch| watch.id != id);
        self.items.len() != before
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn mark_all_stale(&mut self) {
        for watch in &mut self.items {
            watch.state = WatchState::NotEvaluated;
            watch.value = None;
            watch.error = None;
        }
    }

    pub fn begin_evaluation(&mut self, id: u64, operation_id: &str) -> bool {
        let Some(watch) = self.items.iter_mut().find(|watch| watch.id == id) else {
            return false;
        };
        watch.state = WatchState::Evaluating;
        watch.error = None;
        self.inflight.push((operation_id.to_string(), id));
        true
    }

    pub fn adopt_evaluation(&mut self, operation_id: &str, variable: &DebugVariableView) -> bool {
        let Some(index) = self
            .inflight
            .iter()
            .position(|(pending, _)| pending == operation_id)
        else {
            return false;
        };
        let (_, id) = self.inflight.remove(index);
        if let Some(watch) = self.items.iter_mut().find(|watch| watch.id == id) {
            watch.state = WatchState::Ready;
            watch.value = Some(variable.value.clone());
            watch.type_name = variable.type_name.clone();
            return true;
        }
        false
    }

    pub fn adopt_failure(&mut self, operation_id: &str, message: &str) -> bool {
        let Some(index) = self
            .inflight
            .iter()
            .position(|(pending, _)| pending == operation_id)
        else {
            return false;
        };
        let (_, id) = self.inflight.remove(index);
        if let Some(watch) = self.items.iter_mut().find(|watch| watch.id == id) {
            watch.state = WatchState::Failed;
            watch.error = Some(message.to_string());
            return true;
        }
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugPaneEvent {
    Hide,
    BreakpointsChanged,
}

pub fn debug_key(root: &std::path::Path, path: &std::path::Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) => {
            let text = relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            if text.is_empty() {
                path.to_string_lossy().replace('\\', "/")
            } else {
                text
            }
        }
        Err(_) => path.to_string_lossy().replace('\\', "/"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugToolbarAction {
    RestartOrStart,
    Stop,
    Resume,
    Pause,
    StepOver,
    StepInto,
    StepOut,
    ViewBreakpoints,
    MuteBreakpoints,
}

pub const DEBUG_TOOLBAR_ACTIONS: [DebugToolbarAction; 9] = [
    DebugToolbarAction::RestartOrStart,
    DebugToolbarAction::Stop,
    DebugToolbarAction::Resume,
    DebugToolbarAction::Pause,
    DebugToolbarAction::StepOver,
    DebugToolbarAction::StepInto,
    DebugToolbarAction::StepOut,
    DebugToolbarAction::ViewBreakpoints,
    DebugToolbarAction::MuteBreakpoints,
];

pub fn debug_toolbar_has_separator_after(action: DebugToolbarAction) -> bool {
    matches!(
        action,
        DebugToolbarAction::Stop | DebugToolbarAction::StepOut
    )
}

pub const DEBUG_TOOLBAR_HEIGHT_SPEC: f32 = 36.;
pub const DEBUG_SESSION_HEADER_HEIGHT_SPEC: f32 = 34.;
pub const DEBUG_TOOLBAR_ICON_SPEC: f32 = 18.;
pub const DEBUG_CONTENT_TABS_HEIGHT_SPEC: f32 = 32.;
pub const DEBUG_SECTION_HEADER_HEIGHT_SPEC: f32 = 27.;
pub const DEBUG_EVALUATE_ROW_HEIGHT_SPEC: f32 = 32.;
pub const DEBUG_VARIABLE_INDENT_SPEC: f32 = 14.;
pub const DEBUG_STACK_COLUMN_MIN_WIDTH_SPEC: f32 = 240.;
pub const DEBUG_DATA_COLUMN_MIN_WIDTH_SPEC: f32 = 300.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugContent {
    Debugger,
    Console,
}

impl DebugContent {
    pub fn title_key(self) -> &'static str {
        match self {
            Self::Debugger => "lithe.debugger.threadsAndVariables",
            Self::Console => "lithe.debugger.console",
        }
    }
}

pub struct DebugPane {
    breakpoints: DebugBreakpointStore,
    session: Option<DebugSessionHandle>,
    session_output_seen: usize,
    execution: DebugExecution,
    inspection: DebugInspection,
    tree: DebugVariableTree,
    watches: DebugWatches,
    filters: SteppingFiltersState,
    evaluate_input: String,
    content: DebugContent,
    error_message: Option<String>,
    output: Vec<String>,
    filters_task: Option<Task<()>>,
    last_source: Option<(String, String)>,
    gate: DebugDispatchGate,
}

impl EventEmitter<DebugPaneEvent> for DebugPane {}

impl Default for DebugPane {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugPane {
    pub fn new() -> Self {
        Self {
            breakpoints: DebugBreakpointStore::default(),
            execution: DebugExecution::default(),
            inspection: DebugInspection::default(),
            tree: DebugVariableTree::default(),
            watches: DebugWatches::default(),
            filters: SteppingFiltersState::Idle,
            evaluate_input: String::new(),
            content: DebugContent::Debugger,
            session: None,
            session_output_seen: 0,
            error_message: None,
            output: Vec::new(),
            filters_task: None,
            last_source: None,
            gate: DebugDispatchGate::disconnected(),
        }
    }

    pub fn transport(&self) -> DebugTransportFacts {
        self.gate.transport
    }

    pub fn session(&self) -> DebugSessionFacts {
        if self.gate.can_dispatch() {
            DebugSessionFacts::Present
        } else {
            DebugSessionFacts::Absent
        }
    }

    pub fn start_blocker(&self) -> Option<DebugStartBlocker> {
        start_blocker(self.transport(), self.session())
    }

    pub fn breakpoints(&self) -> &DebugBreakpointStore {
        &self.breakpoints
    }

    pub fn gutter_snapshot(&self, root: &Path) -> BTreeMap<PathBuf, Vec<GutterBreakpoint>> {
        self.breakpoints
            .files()
            .map(|key| {
                let path = if Path::new(key.as_str()).is_absolute() {
                    PathBuf::from(key)
                } else {
                    root.join(key)
                };
                let marks = self
                    .breakpoints
                    .file(key)
                    .iter()
                    .map(|breakpoint| GutterBreakpoint {
                        line: usize::try_from(breakpoint.line.max(1)).unwrap_or(1),
                        enabled: breakpoint.enabled,
                        verified: breakpoint.verified,
                    })
                    .collect();
                (path, marks)
            })
            .collect()
    }

    pub fn execution(&self) -> &DebugExecution {
        &self.execution
    }

    pub fn inspection(&self) -> &DebugInspection {
        &self.inspection
    }

    pub fn watches(&self) -> &DebugWatches {
        &self.watches
    }

    pub fn filters(&self) -> &SteppingFiltersState {
        &self.filters
    }

    pub fn content(&self) -> DebugContent {
        self.content
    }

    pub fn has_content(&self) -> bool {
        self.session() == DebugSessionFacts::Present
            || !self.output.is_empty()
            || self.error_message.is_some()
    }

    pub fn load_stepping_filters(&mut self, cx: &mut Context<Self>) {
        if matches!(self.filters, SteppingFiltersState::Loading) {
            return;
        }
        let payload = stepping_filters_payload(STEPPING_FILTERS_ADAPTER);
        eprintln!("{DEBUG_DIAGNOSTIC_TAG} steppingFilters adapter={STEPPING_FILTERS_ADAPTER}");
        self.filters = SteppingFiltersState::Loading;
        self.filters_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { core_json(STEPPING_FILTERS_COMMAND, payload) })
                .await;
            let _ = this.update(cx, |pane, cx| {
                pane.filters = match result {
                    Ok(Some(data)) => match SteppingFiltersView::from_payload(&data) {
                        Some(filters) => {
                            eprintln!(
                                "{DEBUG_DIAGNOSTIC_TAG} steppingFilters status=ok classes={} synthetics={} constructors={} staticInit={} hide={}",
                                filters.class_name_filters.len(),
                                filters.skip_synthetics,
                                filters.skip_constructors,
                                filters.skip_static_initializers,
                                filters.hide_filtered_stack_frames,
                            );
                            SteppingFiltersState::Ready(filters)
                        }
                        None => {
                            eprintln!(
                                "{DEBUG_DIAGNOSTIC_TAG} steppingFilters status=shape_unknown"
                            );
                            SteppingFiltersState::Failed(tr("lithe.debugger.gpui.filtersShapeUnknown").to_string())
                        }
                    },
                    Ok(None) => {
                        eprintln!("{DEBUG_DIAGNOSTIC_TAG} steppingFilters status=missing_data");
                        SteppingFiltersState::Failed(
                            tr("lithe.debugger.gpui.filtersMissingData").to_string(),
                        )
                    }
                    Err(error) => {
                        eprintln!(
                            "{DEBUG_DIAGNOSTIC_TAG} steppingFilters status=failed error={error}"
                        );
                        SteppingFiltersState::Failed(error.to_string())
                    }
                };
                cx.notify();
            });
        }));
    }

    pub fn apply_source_edit(
        &mut self,
        path: &str,
        source_before: &str,
        source_after: &str,
        cx: &mut Context<Self>,
    ) {
        self.last_source = Some((path.to_string(), source_after.to_string()));
        let breakpoints = self.breakpoints.file(path);
        if breakpoints.is_empty() {
            return;
        }
        let Some(edit) = derive_source_edit(source_before, source_after) else {
            return;
        };
        let payload = relocate_breakpoints_payload(source_before, &edit, breakpoints);
        let path = path.to_string();
        eprintln!(
            "{DEBUG_DIAGNOSTIC_TAG} relocate path={path} start={} end={} breakpoints={}",
            edit.start_utf16_offset,
            edit.end_utf16_offset,
            breakpoints.len()
        );
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { core_json(RELOCATE_BREAKPOINTS_COMMAND, payload) })
                .await;
            let _ = this.update(cx, |pane, cx| match result {
                Ok(Some(data)) => match parse_relocate_result(&data) {
                    Some(relocated) => {
                        eprintln!(
                            "{DEBUG_DIAGNOSTIC_TAG} relocate path={path} status=ok breakpoints={}",
                            relocated.len()
                        );
                        pane.breakpoints.replace_file(&path, relocated);
                        cx.emit(DebugPaneEvent::BreakpointsChanged);
                        cx.notify();
                    }
                    None => eprintln!(
                        "{DEBUG_DIAGNOSTIC_TAG} relocate path={path} status=shape_unknown"
                    ),
                },
                Ok(None) => {
                    eprintln!("{DEBUG_DIAGNOSTIC_TAG} relocate path={path} status=missing_data")
                }
                Err(error) => {
                    eprintln!(
                        "{DEBUG_DIAGNOSTIC_TAG} relocate path={path} status=failed error={error}"
                    );
                }
            });
        })
        .detach();
    }

    pub fn toggle_breakpoint_at(&mut self, path: &str, line: i64, cx: &mut Context<Self>) {
        if line < 1 {
            return;
        }
        let change = self.breakpoints.toggle(path, line);
        if change == DebugBreakpointChange::Unchanged {
            return;
        }
        eprintln!(
            "{DEBUG_DIAGNOSTIC_TAG} breakpoint toggle path={path} line={line} total={}",
            self.breakpoints.total()
        );
        self.sync_breakpoints(path);
        cx.emit(DebugPaneEvent::BreakpointsChanged);
        cx.notify();
    }

    pub fn set_breakpoint_enabled(
        &mut self,
        path: &str,
        line: i64,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        if self.breakpoints.set_enabled(path, line, enabled) == DebugBreakpointChange::Unchanged {
            return;
        }
        eprintln!("{DEBUG_DIAGNOSTIC_TAG} breakpoint enabled={enabled} path={path} line={line}");
        self.sync_breakpoints(path);
        cx.emit(DebugPaneEvent::BreakpointsChanged);
        cx.notify();
    }

    pub fn remove_breakpoint(&mut self, path: &str, line: i64, cx: &mut Context<Self>) {
        if self.breakpoints.remove(path, line) == DebugBreakpointChange::Unchanged {
            return;
        }
        eprintln!("{DEBUG_DIAGNOSTIC_TAG} breakpoint remove path={path} line={line}");
        self.sync_breakpoints(path);
        cx.emit(DebugPaneEvent::BreakpointsChanged);
        cx.notify();
    }

    pub fn clear_breakpoints(&mut self, cx: &mut Context<Self>) {
        if self.breakpoints.is_empty() {
            return;
        }
        let paths: Vec<String> = self.breakpoints.files().cloned().collect();
        self.breakpoints.clear();
        eprintln!(
            "{DEBUG_DIAGNOSTIC_TAG} breakpoint clear files={}",
            paths.len()
        );
        for path in paths {
            self.sync_breakpoints(&path);
        }
        cx.emit(DebugPaneEvent::BreakpointsChanged);
        cx.notify();
    }

    pub fn toggle_mute_breakpoints(&mut self, cx: &mut Context<Self>) {
        self.breakpoints.toggle_mute();
        eprintln!(
            "{DEBUG_DIAGNOSTIC_TAG} breakpoint muted={}",
            self.breakpoints.muted()
        );
        for path in self.breakpoints.files().cloned().collect::<Vec<_>>() {
            self.sync_breakpoints(&path);
        }
        cx.emit(DebugPaneEvent::BreakpointsChanged);
        cx.notify();
    }

    fn sync_breakpoints(&mut self, path: &str) {
        match self.gate.session_id() {
            Some(session_id) => {
                let payload = self.breakpoints.set_breakpoints_payload(session_id, path);
                eprintln!(
                    "{DEBUG_DIAGNOSTIC_TAG} setBreakpoints session={session_id} path={path} status=dispatch"
                );
                let _ = payload;
            }
            None => eprintln!(
                "{DEBUG_DIAGNOSTIC_TAG} setBreakpoints path={path} status=skipped reason=no-session (H10/H11)"
            ),
        }
    }

    pub fn start_session(&mut self, cx: &mut Context<Self>) {
        match self.start_blocker() {
            Some(blocker) => {
                eprintln!(
                    "{DEBUG_DIAGNOSTIC_TAG} start status=blocked reason={}",
                    blocker.as_str()
                );
                cx.notify();
            }
            None => {
                eprintln!("{DEBUG_DIAGNOSTIC_TAG} start status=ready_to_dispatch");
            }
        }
    }

    pub fn add_watch(&mut self, expression: &str, cx: &mut Context<Self>) {
        if self.watches.add(expression).is_some() {
            eprintln!(
                "{DEBUG_DIAGNOSTIC_TAG} watch add total={}",
                self.watches.len()
            );
            cx.notify();
        }
    }

    pub fn remove_watch(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.watches.remove(id) {
            eprintln!(
                "{DEBUG_DIAGNOSTIC_TAG} watch remove total={}",
                self.watches.len()
            );
            cx.notify();
        }
    }

    pub fn refresh_watches(&mut self, cx: &mut Context<Self>) {
        self.watches.mark_all_stale();
        eprintln!("{DEBUG_DIAGNOSTIC_TAG} watch refresh");
        cx.notify();
    }

    pub fn evaluate(&mut self, cx: &mut Context<Self>) {
        let expression = self.evaluate_input.trim().to_string();
        if expression.is_empty() {
            return;
        }
        if self.execution.state() != DebugExecutionState::Paused {
            eprintln!("{DEBUG_DIAGNOSTIC_TAG} evaluate status=skipped reason=not-paused");
            return;
        }
        let blocker = self.start_blocker();
        eprintln!(
            "{DEBUG_DIAGNOSTIC_TAG} evaluate status=skipped reason={}",
            blocker
                .map(DebugStartBlocker::as_str)
                .unwrap_or("no-session")
        );
        self.add_watch(&expression, cx);
        self.evaluate_input.clear();
        cx.notify();
    }

    pub fn set_evaluate_input(&mut self, value: String, cx: &mut Context<Self>) {
        if self.evaluate_input != value {
            self.evaluate_input = value;
            cx.notify();
        }
    }

    pub fn select_content(&mut self, content: DebugContent, cx: &mut Context<Self>) {
        if self.content != content {
            self.content = content;
            eprintln!(
                "{DEBUG_DIAGNOSTIC_TAG} content={}",
                match content {
                    DebugContent::Debugger => "debugger",
                    DebugContent::Console => "console",
                }
            );
            cx.notify();
        }
    }

    pub fn adopt_update(&mut self, update: &DebugUpdate) {
        if let Some(state) = update.state {
            match state {
                DebugExecutionState::Paused => {}
                DebugExecutionState::Terminated => {
                    self.execution.on_terminated();
                    self.inspection.clear();
                }
                _ => {}
            }
        }
        for event in &update.events {
            match event {
                DebugEventView::StateChanged { state } => {
                    if *state == DebugExecutionState::Terminated {
                        self.execution.on_terminated();
                        self.inspection.clear();
                    }
                }
                DebugEventView::Stopped { reason, thread_id } => {
                    self.execution.on_stopped(reason, *thread_id)
                }
                DebugEventView::Continued { .. } => self.execution.on_continued(),
                DebugEventView::Terminated { .. } => {
                    self.execution.on_terminated();
                    self.inspection.clear();
                }
                DebugEventView::OperationCompleted {
                    operation_id,
                    result,
                } => {
                    self.inspection.adopt(result);
                    if let DebugOperationResultView::Evaluate(variable) = result {
                        self.watches.adopt_evaluation(operation_id, variable);
                    }
                }
                DebugEventView::OperationFailed {
                    operation_id,
                    message,
                } => {
                    self.execution.on_operation_failed(operation_id);
                    self.watches.adopt_failure(operation_id, message);
                    self.error_message = Some(message.clone());
                }
                DebugEventView::Output { output, .. } => {
                    if !output.is_empty() {
                        self.output.push(output.clone());
                    }
                }
                _ => {}
            }
        }
    }

    fn header(&self, cx: &Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .pl_3()
            .pr_2()
            .h(rems(DEBUG_SESSION_HEADER_HEIGHT_SPEC / 16.))
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .text_color(cx.theme().foreground)
                    .child(tr("lithe.debugger.gpui.toolWindowTitle")),
            )
            .child(self.fact_chip(
                tr("lithe.debugger.gpui.transportLabel"),
                self.transport(),
                cx,
            ))
            .child(self.fact_chip(tr("lithe.debugger.gpui.sessionLabel"), self.session(), cx))
            .child(div().flex_1())
            .child(
                Button::new("debug-pane-hide")
                    .small()
                    .ghost()
                    .icon(idea_icon_mono(&idea::STOP_ICON, cx.theme().is_dark()))
                    .tooltip(tr("lithe.debugger.hideRunAndDebug"))
                    .on_click(cx.listener(|_pane, _, _, cx| {
                        cx.emit(DebugPaneEvent::Hide);
                    })),
            )
            .into_any_element()
    }

    fn fact_chip(
        &self,
        label: SharedString,
        present: impl FactPresent,
        cx: &Context<Self>,
    ) -> AnyElement {
        let missing = present.missing_key();
        h_flex()
            .flex_shrink_0()
            .gap_1()
            .px_2()
            .rounded_md()
            .bg(cx.theme().muted)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(label)
            .child(div().text_color(cx.theme().danger).child(tr(missing)))
            .into_any_element()
    }

    fn toolbar(&self, cx: &Context<Self>) -> AnyElement {
        let mut row = h_flex()
            .w_full()
            .flex_shrink_0()
            .h(rems(DEBUG_TOOLBAR_HEIGHT_SPEC / 16.))
            .px_2()
            .gap_1()
            .border_b_1()
            .border_color(cx.theme().border);
        for action in DEBUG_TOOLBAR_ACTIONS {
            if let Some(control) = toolbar_control(action) {
                let enabled = self.execution.is_enabled(control);
                row = row.child(
                    Button::new(SharedString::from(format!(
                        "debug-toolbar-{}",
                        toolbar_action_id(action)
                    )))
                    .small()
                    .ghost()
                    .disabled(!enabled)
                    .tooltip(toolbar_tooltip(action))
                    .on_click(cx.listener(move |pane, _, _, cx| {
                        pane.press_toolbar_action(action, cx);
                    })),
                );
            } else {
                let disabled = match action {
                    DebugToolbarAction::ViewBreakpoints => false,
                    DebugToolbarAction::MuteBreakpoints => pane_is_empty(&self.breakpoints),
                    _ => false,
                };
                row = row.child(
                    Button::new(SharedString::from(format!(
                        "debug-toolbar-{}",
                        toolbar_action_id(action)
                    )))
                    .small()
                    .ghost()
                    .disabled(disabled)
                    .tooltip(toolbar_tooltip(action))
                    .on_click(cx.listener(move |pane, _, _, cx| {
                        pane.press_toolbar_action(action, cx);
                    })),
                );
            }
            if debug_toolbar_has_separator_after(action) {
                row = row.child(
                    div()
                        .w(rems(1. / 16.))
                        .h(rems(DEBUG_TOOLBAR_ICON_SPEC / 16.))
                        .mx_1()
                        .bg(cx.theme().border),
                );
            }
        }
        row.child(div().flex_1())
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .px_2()
                    .rounded_full()
                    .bg(cx.theme().muted)
                    .text_xs()
                    .text_color(match self.execution.state() {
                        DebugExecutionState::Paused => cx.theme().warning,
                        DebugExecutionState::Terminated => cx.theme().muted_foreground,
                        _ => cx.theme().success,
                    })
                    .child(self.execution.status_text()),
            )
            .into_any_element()
    }

    fn press_toolbar_action(&mut self, action: DebugToolbarAction, cx: &mut Context<Self>) {
        match action {
            DebugToolbarAction::RestartOrStart => self.start_session(cx),
            DebugToolbarAction::ViewBreakpoints => {
                self.select_content(DebugContent::Debugger, cx);
            }
            DebugToolbarAction::MuteBreakpoints => self.toggle_mute_breakpoints(cx),
            _ => {
                let Some(control) = toolbar_control(action) else {
                    return;
                };
                let blocker = self.start_blocker();
                eprintln!(
                    "{DEBUG_DIAGNOSTIC_TAG} execute control={} status=skipped reason={}",
                    control.command(),
                    blocker
                        .map(DebugStartBlocker::as_str)
                        .unwrap_or("no-session")
                );
            }
        }
    }

    fn content_tabs(&self, cx: &Context<Self>) -> AnyElement {
        let mut row = h_flex()
            .w_full()
            .flex_shrink_0()
            .h(rems(DEBUG_CONTENT_TABS_HEIGHT_SPEC / 16.))
            .px_2()
            .gap_1()
            .border_b_1()
            .border_color(cx.theme().border);
        for content in [DebugContent::Debugger, DebugContent::Console] {
            let selected = self.content == content;
            row = row.child(
                div()
                    .id(SharedString::from(format!(
                        "debug-content-{}",
                        match content {
                            DebugContent::Debugger => "debugger",
                            DebugContent::Console => "console",
                        }
                    )))
                    .px_2()
                    .h(rems(25. / 16.))
                    .flex()
                    .items_center()
                    .rounded_md()
                    .cursor_pointer()
                    .when(selected, |this| {
                        this.bg(cx.theme().secondary)
                            .text_color(cx.theme().foreground)
                    })
                    .when(!selected, |this| {
                        this.text_color(cx.theme().muted_foreground)
                    })
                    .text_xs()
                    .child(tr(content.title_key()))
                    .on_click(cx.listener(move |pane, _, _, cx| {
                        pane.select_content(content, cx);
                    })),
            );
        }
        row.into_any_element()
    }

    fn section_header(&self, title: SharedString, count: usize, cx: &Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .h(rems(DEBUG_SECTION_HEADER_HEIGHT_SPEC / 16.))
            .px_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(title)
            .child(div().flex_1())
            .child(SharedString::from(count.to_string()))
            .into_any_element()
    }

    fn stack_column(&self, cx: &Context<Self>) -> AnyElement {
        let mut column = v_flex()
            .flex_1()
            .min_w(rems(DEBUG_STACK_COLUMN_MIN_WIDTH_SPEC / 16.))
            .min_h_0()
            .child(self.section_header(
                tr("lithe.debugger.stack"),
                self.inspection.stack_frames.len(),
                cx,
            ));
        if self.inspection.stack_frames.is_empty() {
            column = column.child(self.placeholder(tr("lithe.debugger.stackEmpty"), cx));
        } else {
            for frame in &self.inspection.stack_frames {
                let location = frame
                    .source_path
                    .as_ref()
                    .map(|path| {
                        let name = path.rsplit('/').next().unwrap_or(path);
                        format!("{name}:{}", frame.line)
                    })
                    .unwrap_or_default();
                column = column.child(
                    v_flex()
                        .w_full()
                        .px_2()
                        .py_1()
                        .when(frame.is_filtered, |this| this.opacity(0.58))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().foreground)
                                .child(SharedString::from(frame.name.clone())),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(SharedString::from(location)),
                        ),
                );
            }
        }
        let mut breakpoint_section = v_flex().w_full().child(self.section_header(
            tr("lithe.debugger.breakpoints"),
            self.breakpoints.total(),
            cx,
        ));
        if self.breakpoints.is_empty() {
            breakpoint_section = breakpoint_section
                .child(self.placeholder(tr("lithe.debugger.breakpointsEmpty"), cx));
        } else {
            let rows: Vec<(String, i64, bool)> =
                self.breakpoints
                    .files()
                    .flat_map(|path| {
                        self.breakpoints.file(path).iter().map(move |breakpoint| {
                            (path.clone(), breakpoint.line, breakpoint.enabled)
                        })
                    })
                    .collect();
            for (path, line, enabled) in rows {
                let line_text = line.to_string();
                breakpoint_section = breakpoint_section.child(
                    h_flex()
                        .w_full()
                        .px_2()
                        .py_1()
                        .gap_2()
                        .text_xs()
                        .child(div().text_color(cx.theme().muted_foreground).child(tr_args(
                            "lithe.debugger.lineNumber",
                            &[("line", line_text.as_str())],
                        )))
                        .child(
                            div()
                                .flex_1()
                                .truncate()
                                .text_color(if enabled {
                                    cx.theme().foreground
                                } else {
                                    cx.theme().muted_foreground
                                })
                                .child(SharedString::from(path)),
                        ),
                );
            }
        }
        column.child(breakpoint_section).into_any_element()
    }

    fn data_column(&self, cx: &Context<Self>) -> AnyElement {
        let mut column = v_flex()
            .flex_1()
            .min_w(rems(DEBUG_DATA_COLUMN_MIN_WIDTH_SPEC / 16.))
            .min_h_0()
            .child(
                h_flex()
                    .w_full()
                    .flex_shrink_0()
                    .h(rems(DEBUG_EVALUATE_ROW_HEIGHT_SPEC / 16.))
                    .px_2()
                    .gap_2()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.debugger.variables"))
                    .child(div().flex_1())
                    .child(tr("lithe.debugger.evaluating")),
            )
            .child(self.section_header(
                tr("lithe.debugger.variables"),
                self.inspection.variables.len(),
                cx,
            ));
        if self.inspection.variables.is_empty() {
            column = column.child(self.placeholder(tr("lithe.debugger.variablesEmpty"), cx));
        } else {
            for row in self.tree.rows(&self.inspection.variables) {
                column = column.child(self.variable_row(&row, cx));
            }
        }
        column =
            column.child(self.section_header(tr("lithe.debugger.watch"), self.watches.len(), cx));
        if self.watches.is_empty() {
            column = column.child(self.placeholder(tr("lithe.debugger.watchEmpty"), cx));
        } else {
            for watch in self.watches.items() {
                let detail = match watch.state {
                    WatchState::Ready => watch.value.clone().unwrap_or_default(),
                    WatchState::Failed => watch.error.clone().unwrap_or_default(),
                    WatchState::Evaluating => tr("lithe.debugger.evaluating").to_string(),
                    WatchState::NotEvaluated => tr("lithe.debugger.notEvaluated").to_string(),
                };
                column = column.child(
                    v_flex()
                        .w_full()
                        .px_2()
                        .py_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().foreground)
                                .child(SharedString::from(watch.expression.clone())),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(match watch.state {
                                    WatchState::Failed => cx.theme().danger,
                                    WatchState::Ready => cx.theme().success,
                                    _ => cx.theme().muted_foreground,
                                })
                                .child(SharedString::from(detail)),
                        ),
                );
            }
        }
        column.child(self.filters_block(cx)).into_any_element()
    }

    fn variable_row(&self, row: &DebugVariableRow, cx: &Context<Self>) -> AnyElement {
        let indent = 10. + row.depth as f32 * DEBUG_VARIABLE_INDENT_SPEC;
        let name = format!(
            "{} = {}",
            row.variable.name,
            if row.variable.value.is_empty() {
                tr("lithe.debugger.empty").to_string()
            } else {
                row.variable.value.clone()
            }
        );
        h_flex()
            .w_full()
            .pr_2()
            .py_1()
            .pl(rems(indent / 16.))
            .gap_2()
            .text_xs()
            .child(
                div()
                    .text_color(cx.theme().foreground)
                    .child(SharedString::from(name)),
            )
            .when_some(row.variable.type_name.clone(), |this, type_name| {
                this.child(
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .child(SharedString::from(type_name)),
                )
            })
            .into_any_element()
    }

    fn filters_block(&self, cx: &Context<Self>) -> AnyElement {
        let mut block = v_flex().w_full().child(self.section_header(
            tr("lithe.debugger.gpui.steppingFilters"),
            match &self.filters {
                SteppingFiltersState::Ready(filters) => filters.class_name_filters.len(),
                _ => 0,
            },
            cx,
        ));
        match &self.filters {
            SteppingFiltersState::Idle => {
                block = block.child(self.placeholder(tr("lithe.debugger.gpui.filtersIdle"), cx))
            }
            SteppingFiltersState::Loading => {
                block = block.child(self.placeholder(tr("lithe.debugger.gpui.filtersLoading"), cx))
            }
            SteppingFiltersState::Failed(error) => {
                block = block.child(
                    v_flex()
                        .w_full()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .child(tr("lithe.debugger.gpui.filtersFailed"))
                        .child(SharedString::from(error.clone())),
                )
            }
            SteppingFiltersState::Ready(filters) => {
                block = block.child(
                    v_flex()
                        .w_full()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(SharedString::from(format!(
                            "{}: {}",
                            tr("lithe.debugger.gpui.filterClassPatterns"),
                            filters
                                .class_name_filters
                                .iter()
                                .take(4)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))),
                );
                for toggle in [
                    SteppingFilterToggle::SkipSynthetics,
                    SteppingFilterToggle::SkipConstructors,
                    SteppingFilterToggle::SkipStaticInitializers,
                    SteppingFilterToggle::HideFilteredStackFrames,
                ] {
                    block = block.child(
                        h_flex()
                            .w_full()
                            .px_2()
                            .py_1()
                            .text_xs()
                            .text_color(cx.theme().foreground)
                            .child(tr(toggle.label_key()))
                            .child(div().flex_1())
                            .child(SharedString::from(
                                if toggle.value(filters) { "✓" } else { "—" }.to_string(),
                            )),
                    );
                }
            }
        }
        block.into_any_element()
    }

    fn empty_state(&self, cx: &Context<Self>) -> AnyElement {
        let blocker = self.start_blocker();
        let mut column = v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .px_4()
            .child(
                div()
                    .text_lg()
                    .text_color(cx.theme().foreground)
                    .child(tr("lithe.debugger.gpui.emptyTitle")),
            )
            .child(
                div()
                    .max_w(rems(520. / 16.))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(match blocker {
                        Some(blocker) => tr(blocker.text_key()),
                        None => tr("lithe.debugger.statusIdle"),
                    }),
            );
        column = column
            .child(
                div()
                    .max_w(rems(520. / 16.))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.debugger.gpui.h10Note")),
            )
            .child(
                div()
                    .max_w(rems(520. / 16.))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.debugger.gpui.h11Note")),
            );
        let can_start = blocker.is_none();
        column = column.child(
            Button::new("debug-empty-start")
                .small()
                .disabled(!can_start)
                .tooltip(match blocker {
                    Some(blocker) => tr(blocker.text_key()),
                    None => tr("lithe.debugger.start"),
                })
                .label(tr("lithe.debugger.start"))
                .on_click(cx.listener(|pane, _, _, cx| pane.start_session(cx))),
        );
        column = column.child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(tr_args(
                    "lithe.debugger.gpui.breakpointSummary",
                    &[("count", self.breakpoints.total().to_string().as_str())],
                )),
        );
        column.into_any_element()
    }

    fn console(&self, cx: &Context<Self>) -> AnyElement {
        v_flex()
            .size_full()
            .child(
                div()
                    .px_2()
                    .py_1()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.debugger.adapterOutputAppearsHere")),
            )
            .children(
                self.output
                    .iter()
                    .map(|line| {
                        div()
                            .px_2()
                            .text_xs()
                            .child(SharedString::from(line.clone()))
                    })
                    .collect::<Vec<_>>(),
            )
            .into_any_element()
    }

    fn placeholder(&self, text: SharedString, cx: &Context<Self>) -> AnyElement {
        div()
            .w_full()
            .px_2()
            .py_2()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(text)
            .into_any_element()
    }

    fn debugger_content(&self, cx: &Context<Self>) -> AnyElement {
        h_flex()
            .size_full()
            .min_h_0()
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(self.stack_column(cx)),
            )
            .child(div().w(rems(1. / 16.)).h_full().bg(cx.theme().border))
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(self.data_column(cx)),
            )
            .into_any_element()
    }
}

trait FactPresent {
    fn missing_key(&self) -> &'static str;
}

impl FactPresent for DebugTransportFacts {
    fn missing_key(&self) -> &'static str {
        match self {
            Self::Absent => "lithe.debugger.gpui.absent",
            Self::Present => "lithe.debugger.gpui.present",
        }
    }
}

impl FactPresent for DebugSessionFacts {
    fn missing_key(&self) -> &'static str {
        match self {
            Self::Absent => "lithe.debugger.gpui.absent",
            Self::Present => "lithe.debugger.gpui.present",
        }
    }
}

impl SteppingFilterToggle {
    fn label_key(self) -> &'static str {
        match self {
            Self::SkipSynthetics => "lithe.debugger.gpui.skipSynthetics",
            Self::SkipConstructors => "lithe.debugger.gpui.skipConstructors",
            Self::SkipStaticInitializers => "lithe.debugger.gpui.skipStaticInitializers",
            Self::HideFilteredStackFrames => "lithe.debugger.gpui.hideFilteredStackFrames",
        }
    }
}

fn pane_is_empty(store: &DebugBreakpointStore) -> bool {
    store.is_empty()
}

fn toolbar_control(action: DebugToolbarAction) -> Option<DebugControl> {
    match action {
        DebugToolbarAction::Stop => Some(DebugControl::Terminate),
        DebugToolbarAction::Resume => Some(DebugControl::Continue),
        DebugToolbarAction::Pause => Some(DebugControl::Pause),
        DebugToolbarAction::StepOver => Some(DebugControl::StepOver),
        DebugToolbarAction::StepInto => Some(DebugControl::StepIn),
        DebugToolbarAction::StepOut => Some(DebugControl::StepOut),
        DebugToolbarAction::RestartOrStart => Some(DebugControl::Restart),
        DebugToolbarAction::ViewBreakpoints | DebugToolbarAction::MuteBreakpoints => None,
    }
}

fn toolbar_action_id(action: DebugToolbarAction) -> &'static str {
    match action {
        DebugToolbarAction::RestartOrStart => "restart-or-start",
        DebugToolbarAction::Stop => "stop",
        DebugToolbarAction::Resume => "resume",
        DebugToolbarAction::Pause => "pause",
        DebugToolbarAction::StepOver => "step-over",
        DebugToolbarAction::StepInto => "step-into",
        DebugToolbarAction::StepOut => "step-out",
        DebugToolbarAction::ViewBreakpoints => "view-breakpoints",
        DebugToolbarAction::MuteBreakpoints => "mute-breakpoints",
    }
}

fn toolbar_tooltip(action: DebugToolbarAction) -> SharedString {
    tr(match action {
        DebugToolbarAction::RestartOrStart => "lithe.debugger.start",
        DebugToolbarAction::Stop => "lithe.debugger.stop",
        DebugToolbarAction::Resume => "lithe.debugger.continueDebugging",
        DebugToolbarAction::Pause => "lithe.debugger.pauseDebugging",
        DebugToolbarAction::StepOver => "lithe.debugger.stepOver",
        DebugToolbarAction::StepInto => "lithe.debugger.stepInto",
        DebugToolbarAction::StepOut => "lithe.debugger.stepOut",
        DebugToolbarAction::ViewBreakpoints => "lithe.debugger.breakpoints",
        DebugToolbarAction::MuteBreakpoints => "lithe.debugger.breakpoints",
    })
}

impl DebugPane {
    pub fn attach_session(&mut self, session: DebugSessionHandle) {
        println!(
            "{} attach_session={}",
            DEBUG_DIAGNOSTIC_TAG,
            session.session_id()
        );
        self.session = Some(session);
    }

    pub fn drain_session(&mut self) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let outcome = session.drain();
        let snapshot = session.snapshot();
        copy_new_output(snapshot, &mut self.session_output_seen, &mut self.output);
        if let Some(error) = snapshot.last_error.clone() {
            self.error_message = Some(error);
        }
        if let Some(reason) = outcome.exited.clone() {
            println!("{DEBUG_DIAGNOSTIC_TAG} session_exited reason={reason}");
            self.session = None;
            self.session_output_seen = 0;
            true
        } else {
            outcome.changed
        }
    }

    pub fn stop_session(&mut self, timeout: std::time::Duration) -> bool {
        let Some(session) = self.session.as_mut() else {
            return true;
        };
        let stopped = session.stop(timeout);
        self.session = None;
        stopped
    }
}

impl Render for DebugPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.drain_session() {
            cx.notify();
        }
        v_flex()
            .size_full()
            .min_h_0()
            .bg(cx.theme().background)
            .child(self.header(cx))
            .child(self.toolbar(cx))
            .child(if self.has_content() {
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.content_tabs(cx))
                    .child(div().flex_1().min_h_0().child(match self.content {
                        DebugContent::Debugger => self.debugger_content(cx),
                        DebugContent::Console => self.console(cx),
                    }))
            } else {
                v_flex().flex_1().min_h_0().child(self.empty_state(cx))
            })
            .into_any_element()
    }
}

fn insert_optional(map: &mut serde_json::Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        map.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn toggle_adds_then_removes_a_line_and_keeps_lines_sorted() {
        let mut store = DebugBreakpointStore::default();
        assert_eq!(
            store.toggle("src/Main.java", 12),
            DebugBreakpointChange::File("src/Main.java".to_string())
        );
        store.toggle("src/Main.java", 4);
        store.toggle("src/Main.java", 9);

        let lines: Vec<i64> = store
            .file("src/Main.java")
            .iter()
            .map(|breakpoint| breakpoint.line)
            .collect();
        assert_eq!(lines, vec![4, 9, 12], "行号必须有序");
        assert_eq!(store.total(), 3);

        store.toggle("src/Main.java", 9);
        let lines: Vec<i64> = store
            .file("src/Main.java")
            .iter()
            .map(|breakpoint| breakpoint.line)
            .collect();
        assert_eq!(lines, vec![4, 12]);

        assert_eq!(
            store.toggle("src/Main.java", 0),
            DebugBreakpointChange::Unchanged
        );
        assert_eq!(store.total(), 2);
    }

    #[test]
    fn set_breakpoints_payload_is_a_whole_file_replacement() {
        let mut store = DebugBreakpointStore::default();
        store.toggle("src/Main.java", 7);
        store.toggle("src/Other.java", 3);
        store.set_enabled("src/Other.java", 3, false);

        let payload = store.set_breakpoints_payload("s-1", "src/Other.java");
        assert_eq!(payload["sessionId"], json!("s-1"));
        assert_eq!(payload["sourcePath"], json!("src/Other.java"));
        let items = payload["breakpoints"].as_array().expect("必须是数组");
        assert_eq!(items.len(), 1, "只发这一个文件的断点：{payload}");
        assert_eq!(items[0]["line"], json!(3));
        assert_eq!(items[0]["enabled"], json!(false));
        assert!(items[0].get("column").is_none());
        assert!(items[0].get("condition").is_none());

        let empty = store.set_breakpoints_payload("s-1", "src/Gone.java");
        assert_eq!(empty["breakpoints"], json!([]));
    }

    #[test]
    fn replace_file_sorts_dedups_and_drops_empty_files() {
        let mut store = DebugBreakpointStore::default();
        let marks = vec![
            DebugBreakpoint::line(9),
            DebugBreakpoint::line(2),
            DebugBreakpoint::line(9),
        ];
        store.replace_file("a.java", marks);
        let lines: Vec<i64> = store.file("a.java").iter().map(|b| b.line).collect();
        assert_eq!(lines, vec![2, 9], "排序 + 按 (行, 列) 去重");

        store.replace_file("a.java", Vec::new());
        assert!(store.file("a.java").is_empty());
        assert_eq!(store.total(), 0, "空集合要真的把文件键去掉");
    }

    #[test]
    fn a_missing_transport_is_never_startable_even_with_a_session() {
        assert_eq!(
            start_blocker(DebugTransportFacts::Present, DebugSessionFacts::Present),
            None,
            "两个事实都在才是可启动"
        );
        assert_eq!(
            start_blocker(DebugTransportFacts::Absent, DebugSessionFacts::Present),
            Some(DebugStartBlocker::NoTransport),
            "反证：没有 transport 就不可启动，哪怕会话存在"
        );
        assert_eq!(
            start_blocker(DebugTransportFacts::Present, DebugSessionFacts::Absent),
            Some(DebugStartBlocker::NoSession)
        );
        assert_eq!(
            start_blocker(DebugTransportFacts::Absent, DebugSessionFacts::Absent),
            Some(DebugStartBlocker::NoTransportAndNoSession),
            "当前运行期的真实状态"
        );
    }

    #[test]
    fn every_start_blocker_has_its_own_message_key() {
        let keys = [
            DebugStartBlocker::NoTransport.text_key(),
            DebugStartBlocker::NoSession.text_key(),
            DebugStartBlocker::NoTransportAndNoSession.text_key(),
        ];
        let unique: BTreeSet<&str> = keys.iter().copied().collect();
        assert_eq!(unique.len(), 3, "三个分支必须各有一句话：{keys:?}");
    }

    #[test]
    fn a_fresh_pane_reports_both_facts_missing() {
        let pane = DebugPane::new();
        assert_eq!(pane.transport(), DebugTransportFacts::Absent);
        assert_eq!(pane.session(), DebugSessionFacts::Absent);
        assert_eq!(
            pane.start_blocker(),
            Some(DebugStartBlocker::NoTransportAndNoSession)
        );
        assert!(!pane.has_content(), "没有会话 / 输出 / 错误 ⇒ 画空态");
    }

    #[test]
    fn the_dispatch_gate_stays_closed_without_a_session() {
        let gate = DebugDispatchGate::disconnected();
        assert!(gate.session_id().is_none());
        assert!(!gate.can_dispatch());
    }

    #[test]
    fn relocating_a_breakpoint_across_an_inserted_line_moves_it_down() {
        let before = "class Main {\n    void run() {}\n}\n";
        let after = "class Main {\n    int x = 1;\n    void run() {}\n}\n";
        let edit = derive_source_edit(before, after).expect("两段文本不同 ⇒ 必须有一个 edit");
        assert_eq!(apply_source_edit_for_test(before, &edit), after);
        assert!(edit.start_utf16_offset > 0, "前半段必须活下来：{edit:?}");
        assert!(
            edit.end_utf16_offset < before.encode_utf16().count(),
            "后半段必须活下来：{edit:?}"
        );

        let breakpoints = vec![DebugBreakpoint::line(2)];
        let payload = relocate_breakpoints_payload(before, &edit, &breakpoints);
        assert_eq!(payload["source"], json!(before), "必须给**编辑前**的全文");
        assert_eq!(
            payload["edit"]["startUtf16Offset"],
            json!(edit.start_utf16_offset)
        );
        assert_eq!(
            payload["edit"]["endUtf16Offset"],
            json!(edit.end_utf16_offset)
        );
        assert_eq!(payload["edit"]["replacement"], json!(edit.replacement));
        assert_eq!(payload["breakpoints"][0]["line"], json!(2));

        let response = json!({
            "breakpoints": [{ "line": 3, "enabled": true }]
        });
        let relocated = parse_relocate_result(&response).expect("形状要认得");
        assert_eq!(relocated.len(), 1);
        assert_eq!(relocated[0].line, 3, "断点跟着代码走");

        let mut store = DebugBreakpointStore::default();
        store.toggle("src/Main.java", 2);
        store.replace_file("src/Main.java", relocated);
        assert_eq!(store.file("src/Main.java")[0].line, 3);
    }

    #[test]
    fn derive_source_edit_handles_non_bmp_text_and_deletions() {
        let before = "a🚀b\nc\n";
        let after = "a🚀b\n\nc\n";
        let edit = derive_source_edit(before, after).expect("插入了一个换行");
        assert_eq!(edit.replacement, "\n");
        assert_eq!(edit.start_utf16_offset, 5);
        assert_eq!(edit.end_utf16_offset, 5);
        assert_eq!(edit.replacement, "\n");

        assert!(derive_source_edit("same", "same").is_none());

        let edit = derive_source_edit("abc", "").expect("整段删掉");
        assert_eq!(edit.start_utf16_offset, 0);
        assert_eq!(edit.end_utf16_offset, 3);
        assert_eq!(edit.replacement, "");
    }

    #[test]
    fn an_unrecognized_relocate_response_is_rejected_instead_of_clearing() {
        assert!(parse_relocate_result(&json!({ "nope": [] })).is_none());
        assert!(parse_relocate_result(&json!({ "breakpoints": "x" })).is_none());
        assert_eq!(
            parse_relocate_result(&json!({ "breakpoints": [] })),
            Some(Vec::new())
        );
    }

    #[test]
    fn debug_key_relativizes_inside_the_root_and_keeps_outsiders_absolute() {
        let root = Path::new("/work/project");
        assert_eq!(
            debug_key(root, Path::new("/work/project/src/Main.java")),
            "src/Main.java"
        );
        assert_eq!(
            debug_key(root, Path::new("/other/Thing.java")),
            "/other/Thing.java"
        );
        assert_eq!(debug_key(root, root), "/work/project");
    }

    #[test]
    fn a_synthetic_dap_update_projects_into_the_inspector_blocks() {
        let update = parse_session_update(&json!({
            "sessionId": "s-1",
            "state": "paused",
            "outboundFrames": ["ZnJhbWU=", "bW9yZQ=="],
            "events": [
                { "type": "stopped", "reason": "breakpoint", "threadId": 3 },
                {
                    "type": "operationCompleted",
                    "operationId": "op-1",
                    "result": {
                        "kind": "stackTrace",
                        "stackFrames": [
                            { "id": 11, "name": "Main.run", "sourcePath": "src/Main.java",
                              "line": 7, "column": 5, "isFiltered": false },
                            { "id": 12, "name": "java.lang.Thread.run", "sourcePath": null,
                              "line": 0, "column": 0, "isFiltered": true }
                        ]
                    }
                },
                {
                    "type": "operationCompleted",
                    "operationId": "op-2",
                    "result": {
                        "kind": "variables",
                        "variables": [
                            { "name": "this", "value": "Main@1", "type": "Main",
                              "variablesReference": 42, "namedVariables": 3 },
                            { "name": "count", "value": "7", "type": "int",
                              "variablesReference": 0 }
                        ]
                    }
                }
            ]
        }));

        assert_eq!(update.state, Some(DebugExecutionState::Paused));
        assert_eq!(update.outbound_frame_count, 2, "只留条数，不搬字节");
        assert_eq!(update.events.len(), 3);

        let mut inspection = DebugInspection::default();
        for event in &update.events {
            if let DebugEventView::OperationCompleted { result, .. } = event {
                inspection.adopt(result);
            }
        }
        assert_eq!(inspection.stack_frames.len(), 2);
        assert_eq!(inspection.stack_frames[0].line, 7);
        assert!(inspection.stack_frames[1].is_filtered, "被过滤的帧要标出来");
        assert_eq!(inspection.variables.len(), 2);
        assert_eq!(inspection.variables[0].named_variables, 3);
        assert!(inspection.variables[0].is_expandable());
        assert!(!inspection.variables[1].is_expandable());

        let tree = DebugVariableTree::default();
        let rows = tree.rows(&inspection.variables);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.depth == 0));
    }

    #[test]
    fn an_unknown_dap_event_keeps_its_kind() {
        let update = parse_session_update(&json!({
            "state": "running",
            "events": [{ "type": "somethingNew" }]
        }));
        assert_eq!(
            update.events,
            vec![DebugEventView::Unknown {
                kind: "somethingNew".to_string()
            }]
        );
        assert_eq!(update.state, Some(DebugExecutionState::Running));
        assert_eq!(
            parse_execution_state("initializing"),
            Some(DebugExecutionState::Running)
        );
        assert_eq!(
            parse_execution_state("terminated"),
            Some(DebugExecutionState::Terminated)
        );
        assert_eq!(parse_execution_state("no-such-state"), None);
    }

    #[test]
    fn watch_evaluation_is_claimed_by_operation_id_not_by_order() {
        let mut watches = DebugWatches::default();
        let first = watches.add("a + b").expect("非空表达式要能加");
        let second = watches.add(" c ".trim()).expect("去空白后非空");

        assert!(watches.begin_evaluation(first, "op-1"));
        assert!(watches.begin_evaluation(second, "op-2"));

        assert!(!watches.adopt_evaluation(
            "op-9",
            &DebugVariableView {
                name: "x".to_string(),
                value: "1".to_string(),
                type_name: None,
                evaluate_name: None,
                variables_reference: 0,
                named_variables: 0,
                indexed_variables: 0,
            }
        ));

        assert!(watches.adopt_evaluation(
            "op-2",
            &DebugVariableView {
                name: "c".to_string(),
                value: "42".to_string(),
                type_name: Some("int".to_string()),
                evaluate_name: None,
                variables_reference: 0,
                named_variables: 0,
                indexed_variables: 0,
            }
        ));
        assert_eq!(watches.items()[0].state, WatchState::Evaluating);
        assert_eq!(watches.items()[1].state, WatchState::Ready);
        assert_eq!(watches.items()[1].value.as_deref(), Some("42"));

        assert_eq!(watches.add("a + b"), None);
        assert_eq!(watches.len(), 2);

        assert!(watches.begin_evaluation(first, "op-3"));
        assert!(watches.adopt_failure("op-3", "cannot evaluate"));
        assert_eq!(watches.items()[0].state, WatchState::Failed);
        assert_eq!(watches.items()[0].error.as_deref(), Some("cannot evaluate"));
        assert!(!watches.adopt_failure("op-3", "again"));
    }

    fn apply_source_edit_for_test(before: &str, edit: &DebugSourceEdit) -> String {
        let units: Vec<u16> = before.encode_utf16().collect();
        let mut out: Vec<u16> = Vec::new();
        out.extend_from_slice(&units[..edit.start_utf16_offset]);
        out.extend(edit.replacement.encode_utf16());
        out.extend_from_slice(&units[edit.end_utf16_offset..]);
        String::from_utf16_lossy(&out)
    }

    #[test]
    fn the_execution_machine_gates_every_control_like_the_macos_source() {
        let mut execution = DebugExecution::default();
        assert_eq!(execution.state(), DebugExecutionState::Idle);
        assert!(!execution.is_enabled(DebugControl::Continue));
        assert!(!execution.is_enabled(DebugControl::Pause));
        assert!(!execution.is_enabled(DebugControl::StepOver));

        execution.on_continued();
        assert!(execution.is_enabled(DebugControl::Pause));
        assert!(!execution.is_enabled(DebugControl::Continue));

        execution.on_stopped("breakpoint", Some(3));
        assert!(execution.is_enabled(DebugControl::Continue));
        assert!(execution.is_enabled(DebugControl::StepIn));
        assert!(!execution.is_enabled(DebugControl::Pause));

        let mut no_thread = DebugExecution::default();
        no_thread.on_stopped("pause", None);
        assert!(!no_thread.is_enabled(DebugControl::StepOver));
        assert!(no_thread.is_enabled(DebugControl::Continue));

        let generation = execution.begin("op-1", DebugControl::Continue);
        assert!(execution.is_request_pending());
        assert!(!execution.is_enabled(DebugControl::StepOver));
        assert!(execution.accepts(generation));

        execution.on_continued();
        let next = execution.begin("op-2", DebugControl::Pause);
        assert!(next > generation);
        assert!(!execution.accepts(generation), "旧代次的结果必须被丢掉");
        assert!(execution.accepts(next));

        assert!(!execution.on_operation_failed("op-1"));
        assert!(execution.on_operation_failed("op-2"));
        assert!(!execution.is_request_pending());

        execution.on_terminated();
        assert_eq!(execution.state(), DebugExecutionState::Terminated);
        assert!(!execution.is_enabled(DebugControl::Restart));
        assert_eq!(execution.stopped_reason(), None);
    }

    #[test]
    fn the_status_capsule_only_names_stop_reasons_the_catalog_has_words_for() {
        let mut execution = DebugExecution::default();
        execution.on_stopped("breakpoint", Some(1));
        assert_eq!(
            stop_reason_label("breakpoint"),
            Some("lithe.debugger.breakpoints")
        );
        assert_eq!(stop_reason_label("PAUSE"), Some("lithe.debugger.paused"));
        assert_eq!(stop_reason_label("data breakpoint"), None);
        assert_eq!(stop_reason_label("entry"), None);
    }

    #[test]
    fn the_toolbar_order_and_separators_match_the_macos_source() {
        assert_eq!(DEBUG_TOOLBAR_ACTIONS.len(), 9);
        let ids: Vec<&str> = DEBUG_TOOLBAR_ACTIONS
            .iter()
            .copied()
            .map(toolbar_action_id)
            .collect();
        assert_eq!(
            ids,
            vec![
                "restart-or-start",
                "stop",
                "resume",
                "pause",
                "step-over",
                "step-into",
                "step-out",
                "view-breakpoints",
                "mute-breakpoints",
            ]
        );
        let separated: Vec<&str> = DEBUG_TOOLBAR_ACTIONS
            .iter()
            .copied()
            .filter(|action| debug_toolbar_has_separator_after(*action))
            .map(toolbar_action_id)
            .collect();
        assert_eq!(separated, vec!["stop", "step-out"]);

        assert!(toolbar_control(DebugToolbarAction::ViewBreakpoints).is_none());
        assert!(toolbar_control(DebugToolbarAction::MuteBreakpoints).is_none());
        assert_eq!(
            toolbar_control(DebugToolbarAction::StepOver).map(DebugControl::command),
            Some("next")
        );
        assert_eq!(
            toolbar_control(DebugToolbarAction::StepInto).map(DebugControl::command),
            Some("stepIn")
        );
        assert_eq!(DEBUG_TOOLBAR_HEIGHT_SPEC, 36.);
        assert_eq!(DEBUG_SESSION_HEADER_HEIGHT_SPEC, 34.);
        assert_eq!(DEBUG_TOOLBAR_ICON_SPEC, 18.);
    }

    #[test]
    fn execute_payload_matches_the_core_execute_request_shape() {
        let payload = DebugControl::StepOver.payload("s-1", "op-7", Some(3));
        assert_eq!(payload["sessionId"], json!("s-1"));
        assert_eq!(payload["operationId"], json!("op-7"));
        assert_eq!(payload["command"], json!("next"));
        assert_eq!(payload["threadId"], json!(3));
        assert_eq!(payload["singleThread"], json!(false));

        let payload = DebugControl::Continue.payload("s-1", "op-8", None);
        assert!(payload.get("threadId").is_none());
        assert_eq!(payload["command"], json!("continue"));
    }

    #[test]
    fn stepping_filters_payload_asks_for_the_adapter_defaults() {
        let payload = stepping_filters_payload("java");
        assert_eq!(payload["adapterId"], json!("java"));
        assert!(
            payload.get("filters").is_none(),
            "传一份宿主自造的默认值就是把 Core 的默认档复制一份：{payload}"
        );

        let filters = SteppingFiltersView::from_payload(&json!({
            "classNameFilters": ["$JDK", "$Libraries", "org.mockito.*"],
            "skipSynthetics": true,
            "skipStaticInitializers": true,
            "skipConstructors": false,
            "hideFilteredStackFrames": true
        }))
        .expect("Core 的默认档形状要认得");
        assert_eq!(filters.class_name_filters.len(), 3);
        assert!(filters.skip_synthetics);
        assert!(!filters.skip_constructors);
        assert!(SteppingFilterToggle::SkipConstructors.value(&filters) == false);
        assert!(SteppingFilterToggle::HideFilteredStackFrames.value(&filters));

        assert!(SteppingFiltersView::from_payload(&json!({ "skipSynthetics": true })).is_none());
    }

    #[test]
    fn exception_and_function_breakpoint_payloads_match_the_core_requests() {
        let payload = set_exception_breakpoints_payload("s-1", &["uncaught".to_string()]);
        assert_eq!(payload["sessionId"], json!("s-1"));
        assert_eq!(payload["breakpoints"][0]["filter"], json!("uncaught"));
        assert_eq!(payload["breakpoints"][0]["enabled"], json!(true));

        let payload = set_function_breakpoints_payload("s-1", &["Main.run".to_string()]);
        assert_eq!(payload["sessionId"], json!("s-1"));
        assert_eq!(payload["breakpoints"][0]["name"], json!("Main.run"));
    }
}
