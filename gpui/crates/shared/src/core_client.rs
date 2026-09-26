//! `lithe-core` 的**信封层**：一次 Core 命令调用的全部 JSON 拼装与解析都收敛在这里。
//!
//! ## 为什么它属于 `shared`
//!
//! 信封契约（`{id, operationId, timeoutMilliseconds, command, payload}` →
//! `{ok, data?, error?}`，`rust/lithe-core/src/lib.rs`）由 `lithe-core` 拥有；在本次分层重构前，
//! 这段拼装被**逐字复制了两份**：
//!
//! - `explorer`（`workspace.snapshot`，`id` 固定 `shell-explorer-snapshot`）；
//! - `git`（6 条 `git.*`，`id` 是 `lithe-gpui-<command>-<n>` 自增序号）。
//!
//! 这正是《编码指南》「只有一项能力已有清晰名称和两个以上真实使用方时，才提取共享 crate」
//! 描述的正面案例：有名字（信封 / 信封层）、有两个真实使用方、且两边**已经漂移**
//! （错误处理与 `data` 缺失的判定不同）。收敛到一处之后：
//!
//! - 请求字段名、超时单位、`operationId` 的生成口径只有一份；
//! - `ok=false` → 错误码、`ok=true` 但没有 `data` → [`CoreError::MissingData`]
//!   这两种失败在所有 feature 里表现一致。
//!
//! ## 调用形态
//!
//! ```ignore
//! let data = CoreClient::new().execute("git.status", json!({ "root": root }))?;
//! let name = data.get("branch").and_then(Value::as_str);
//! ```
//!
//! `lithe_core::execute_json` 是**同步**调用，所以本层的调用方（feature 的 model）必须把它
//! 放进 `cx.background_spawn`，不要在 `render` 或 UI 线程上直接调。

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

/// 默认超时。当前两个使用方都用 120 s（`workspace.snapshot` 要扫全工作区，
/// `git.*` 要起 `git` 子进程），所以把它作为 [`CoreRequest::DEFAULT_TIMEOUT_MILLIS`]。
pub const DEFAULT_TIMEOUT_MILLIS: u64 = 120_000;

/// `operationId` 的进程内自增序号，保证同一个命令的多次调用 id 不重复。
static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);

/// 信封级失败。
///
/// 只表达"这条命令没能给出可用的 `data`"这一类失败；**命令自己的领域语义**
/// （例如 `git.status` 用 `ok:true` + `repositoryRoot:null` 表示"不是仓库"）
/// 不属于错误，留给调用方判字段 —— 这是 `lithe-core` 的既有契约，不要在这里改写。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// `lithe_core::execute_json` 的返回值不是合法 JSON。
    InvalidResponse {
        /// 出错的命令名，便于在诊断行里定位。
        command: String,
        /// JSON 解析器的错误原文。
        message: String,
    },
    /// 响应 `ok:false`。`code` / `message` 逐字取自 `/error/code` 与 `/error/message`。
    Reported {
        /// Core 给出的稳定错误码（缺字段时为 `unknown`）。
        code: String,
        /// Core 给出的可读消息（缺字段时为 `(无消息)`）。
        message: String,
    },
    /// 响应 `ok:true` 但**没有** `data` 字段。
    MissingData {
        /// 出错的命令名。
        command: String,
    },
}

impl CoreError {
    /// Core 的错误码；不是 `ok:false` 一类失败时为 `None`。
    ///
    /// 供调用方在诊断行里原样显示（例如 `git.status（process_failed）`），
    /// 不要用它做分支判断 —— 需要分支时应当看字段语义。
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::Reported { code, .. } => Some(code),
            _ => None,
        }
    }

    /// 这条失败是不是"这个根不是 Git 仓库"。
    ///
    /// **这是本类型唯一为分支判断开的入口**，理由：这一种失败**没有字段可看**。
    /// Core 的 `require_git_repository`（`rust/lithe-core/src/git/mod.rs`）对非仓库根返回
    /// 错误码 `invalid_request` + 稳定消息 `Not a Git repository`，它的注释明确写着这条稳定
    /// 消息就是给宿主"跳过 Git 副作用"用的。所以这里匹配它，而不是让每个调用方各自
    /// 猜、或者再发一次 `git.status` 去重复检测仓库状态。
    pub fn is_not_a_repository(&self) -> bool {
        matches!(
            self,
            Self::Reported { code, message }
                if code == "invalid_request" && message == NOT_A_GIT_REPOSITORY_MESSAGE
        )
    }
}

/// Core 用来表示"这个根不是 Git 仓库"的稳定消息（见 [`CoreError::is_not_a_repository`]）。
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

/// 发往 `lithe-core` 的一条请求（`command` + `payload` + 超时 + 操作 id）。
///
/// 字段私有：信封的键名是 `lithe-core` 的兼容面，不允许调用方自己拼。
/// 需要非默认值时用 builder：`CoreRequest::command("git.status").payload(payload)`
/// （`with_*` 造 non-boolean 值，见《编码指南》词汇表）。
#[derive(Debug, Clone)]
pub struct CoreRequest {
    command: String,
    payload: Value,
    timeout_millis: u64,
    operation_id: Option<String>,
}

impl CoreRequest {
    /// 默认超时（[`DEFAULT_TIMEOUT_MILLIS`]）。
    pub const DEFAULT_TIMEOUT_MILLIS: u64 = DEFAULT_TIMEOUT_MILLIS;

    /// 一条用默认超时、`payload` 为空的请求。
    pub fn command(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            payload: Value::Null,
            timeout_millis: DEFAULT_TIMEOUT_MILLIS,
            operation_id: None,
        }
    }

    /// 设置 payload（non-boolean builder 用 `with_` 命名）。
    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = payload;
        self
    }

    /// 覆盖超时（毫秒）。
    pub fn with_timeout_millis(mut self, timeout_millis: u64) -> Self {
        self.timeout_millis = timeout_millis;
        self
    }

    /// 固定 `operationId`（不设则自动生成 `lithe-gpui-<command>-<n>`）。
    pub fn with_operation_id(mut self, operation_id: impl Into<String>) -> Self {
        self.operation_id = Some(operation_id.into());
        self
    }

    /// 命令名。
    pub fn command_name(&self) -> &str {
        &self.command
    }

    /// 按 `lithe-core` 的信封契约拼出请求 JSON 文本。
    ///
    /// `id` 与 `operationId` 取同一个值：Core 用 `operationId` 做取消 / 陈旧结果校验，
    /// 用 `id` 做响应关联，本应用两者一一对应。
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

/// `lithe-core` 的信封客户端。
///
/// 无状态（`operationId` 序号是进程级 `AtomicU64`），所以可以随手 `CoreClient::new()`；
/// 保留成类型而不是自由函数，是为了让"Core 调用"在 feature 里有一个可搜索的名字。
#[derive(Debug, Default, Clone, Copy)]
pub struct CoreClient;

impl CoreClient {
    /// 建一个客户端。
    pub fn new() -> Self {
        Self
    }

    /// 执行一条命令并解开信封，返回 `data`。
    ///
    /// - `Ok(Some(data))`：`ok:true` 且有 `data`；
    /// - `Ok(None)`：`ok:true` 但 `data` 缺失或为 `null`（调用方按字段语义判空）；
    /// - `Err(CoreError)`：JSON 非法、`ok:false`。
    ///
    /// **同步**：调用方负责放进后台任务。
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

/// 便捷形态：`command` + `payload` + 默认超时执行一条命令。
///
/// ```ignore
/// let data = core_json("git.status", json!({ "root": root }))?;
/// ```
pub fn core_json(command: &str, payload: Value) -> Result<Option<Value>, CoreError> {
    CoreClient::new().execute(&CoreRequest::command(command).with_payload(payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "不是 Git 仓库"的判定跟着 Core 的**稳定信号**走：错误码 `invalid_request` +
    /// 消息 `Not a Git repository`（两个条件都要满足）。
    ///
    /// 这条单测是那个分支的确定性覆盖：真实路径需要"一个不在任何 Git 工作树里的可写目录"，
    /// 而那在验证环境里未必成立（见 `workspace_config::test_repo` 的模块文档）。
    #[test]
    fn not_a_repository_follows_the_documented_signal() {
        let reported = |code: &str, message: &str| CoreError::Reported {
            code: code.to_string(),
            message: message.to_string(),
        };

        assert!(reported("invalid_request", "Not a Git repository").is_not_a_repository());
        // 码对、消息不对：不是这一种失败。
        assert!(
            !reported(
                "invalid_request",
                "Git ignore operation contains an invalid pattern"
            )
            .is_not_a_repository()
        );
        // 消息对、码不对：不是这一种失败。
        assert!(!reported("process_failed", "Not a Git repository").is_not_a_repository());
        // 信封级失败与缺字段都不是"不是仓库"。
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
}
