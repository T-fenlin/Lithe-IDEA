use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

pub const DEFAULT_TIMEOUT_MILLIS: u64 = 120_000;

static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    InvalidResponse { command: String, message: String },
    Reported { code: String, message: String },
    MissingData { command: String },
}

impl CoreError {
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::Reported { code, .. } => Some(code),
            _ => None,
        }
    }

    pub fn is_not_a_repository(&self) -> bool {
        matches!(
            self,
            Self::Reported { code, message }
                if code == "invalid_request" && message == NOT_A_GIT_REPOSITORY_MESSAGE
        )
    }
}

const NOT_A_GIT_REPOSITORY_MESSAGE: &str = "Not a Git repository";

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidResponse { message, .. } => {
                write!(formatter, "响应不是合法 JSON：{message}")
            }
            Self::Reported { code, message } => write!(formatter, "{code}: {message}"),
            Self::MissingData { command } => write!(formatter, "{command} 的响应缺少 data"),
        }
    }
}

impl std::error::Error for CoreError {}

#[derive(Debug, Clone)]
pub struct CoreRequest {
    command: String,
    payload: Value,
    timeout_millis: u64,
    operation_id: Option<String>,
}

impl CoreRequest {
    pub const DEFAULT_TIMEOUT_MILLIS: u64 = DEFAULT_TIMEOUT_MILLIS;

    pub fn command(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            payload: Value::Null,
            timeout_millis: DEFAULT_TIMEOUT_MILLIS,
            operation_id: None,
        }
    }

    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = payload;
        self
    }

    pub fn with_timeout_millis(mut self, timeout_millis: u64) -> Self {
        self.timeout_millis = timeout_millis;
        self
    }

    pub fn with_operation_id(mut self, operation_id: impl Into<String>) -> Self {
        self.operation_id = Some(operation_id.into());
        self
    }

    pub fn command_name(&self) -> &str {
        &self.command
    }

    pub fn to_request_json(&self) -> String {
        let operation_id = self.operation_id.clone().unwrap_or_else(|| {
            let call = NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed);
            format!("lithe-gpui-{}-{call}", self.command)
        });

        serde_json::json!({
            "id": operation_id,
            "operationId": operation_id,
            "timeoutMilliseconds": self.timeout_millis,
            "command": self.command,
            "payload": self.payload,
        })
        .to_string()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CoreClient;

impl CoreClient {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, request: &CoreRequest) -> Result<Option<Value>, CoreError> {
        let raw = lithe_core::execute_json(&request.to_request_json());
        let value: Value =
            serde_json::from_str(&raw).map_err(|error| CoreError::InvalidResponse {
                command: request.command.clone(),
                message: error.to_string(),
            })?;

        if value.get("ok").and_then(Value::as_bool) == Some(true) {
            return Ok(match value.get("data") {
                None | Some(Value::Null) => None,
                Some(data) => Some(data.clone()),
            });
        }

        Err(CoreError::Reported {
            code: value
                .pointer("/error/code")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            message: value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("(无消息)")
                .to_string(),
        })
    }
}

pub fn core_json(command: &str, payload: Value) -> Result<Option<Value>, CoreError> {
    CoreClient::new().execute(&CoreRequest::command(command).with_payload(payload))
}

pub fn cancel_operation(operation_id: &str) -> bool {
    lithe_core::cancel_operation(operation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_a_repository_follows_the_documented_signal() {
        let reported = |code: &str, message: &str| CoreError::Reported {
            code: code.to_string(),
            message: message.to_string(),
        };

        assert!(reported("invalid_request", "Not a Git repository").is_not_a_repository());
        assert!(
            !reported(
                "invalid_request",
                "Git ignore operation contains an invalid pattern"
            )
            .is_not_a_repository()
        );
        assert!(!reported("process_failed", "Not a Git repository").is_not_a_repository());
        assert!(
            !CoreError::MissingData {
                command: "git.write".to_string()
            }
            .is_not_a_repository()
        );
        assert!(
            !CoreError::InvalidResponse {
                command: "git.write".to_string(),
                message: "not json".to_string(),
            }
            .is_not_a_repository()
        );
    }

    #[test]
    fn cancelling_an_unknown_operation_reports_false_instead_of_failing() {
        assert!(!cancel_operation("lithe-gpui-test-never-registered"));
    }
}
