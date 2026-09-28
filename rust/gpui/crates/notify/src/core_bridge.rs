//! `CoreError` → 通知条目的桥。
//!
//! 存在理由：[`Input::for_core_code`] 只吃 Core 的**稳定错误码**那个形态，而
//! [`CoreError`] 有三个变体，其中两个**没有码**。让每个调用点自己判会漏 —— 而漏掉的
//! 表现是「失败静默消失」，那正是本仓库最不能接受的一种。
//!
//! 三个变体各自的去处：
//!
//! | 变体 | 去处 | 为什么 |
//! | --- | --- | --- |
//! | `Reported { code, .. }` | `Severity::for_core_code(code)` | Core 自己给的稳定分类，一比一透传 |
//! | `InvalidResponse { .. }` | 归成 `parse_failed` | 「Core 的返回值不是合法 JSON」就是一次解析失败（`error.rs:26-27` 对 `parse_failed` 的定义就是「输入或工具输出无法解码成稳定契约」） |
//! | `MissingData { .. }` | **不入中心**，返回 `None` | 「`ok:true` 但没有 `data`」多数时候不是失败而是「没有东西可显示」——`core_client.rs:44-46` 明写领域语义走字段而不是错误。把它报成通知会刷出一堆「成功但没数据」的噪音 |
//!
//! `cancelled` 同样返回 `None`（那是用户自己取消的，见 [`Severity::for_core_code`]）。
//!
//! ## 调用点怎么用
//!
//! ```ignore
//! match core_json("git.status", payload) {
//!     Ok(value) => { /* … */ }
//!     Err(error) => {
//!         // 桥只在该报的时候返回 Some；`None` = 不进中心（cancelled / MissingData）。
//!         if let Some(input) = lithe_gpui_notify::from_core_error(&error, "git.status") {
//!             lithe_gpui_notify::record(cx, input);
//!         }
//!     }
//! }
//! ```
//!
//! `command` 那个参数会进条目的 `command` 参数，于是**详情里能看到是哪条命令失败的**，
//! 而且它可搜（`Entry::matches_query` 匹配参数值）。

use lithe_gpui_shared::CoreError;

use crate::severity::Severity;
use crate::store::Input;

/// 把一次 Core 失败变成一条通知；`None` = 不该进中心。
///
/// 诊断行请照旧打自己那一条（`S1_*`）—— 这个函数**不替代**诊断行，只决定要不要多一条
/// 用户可见的记录。两条都打是故意的：机器判据给人看，通知给用户看。
pub fn from_core_error(error: &CoreError, command: &str) -> Option<Input> {
    match error {
        CoreError::Reported { code, message } => {
            let input = Input::for_core_code(code)?;
            Some(
                input
                    .param("detail", message.as_str())
                    .param("command", command),
            )
        }
        CoreError::InvalidResponse { message, .. } => Some(
            Input::for_core_code("parse_failed")?
                .param("detail", message.as_str())
                .param("command", command),
        ),
        // 见模块文档的表：不入中心，避免「成功但没数据」刷屏。
        CoreError::MissingData { .. } => None,
    }
}

/// 只要「这条失败值不值得进中心」，不要条目（调用点自己已经有 `CoreError` 的展示面时用）。
///
/// 与 [`from_core_error`] 的判据完全一致，只是不构造条目 —— 避免为了拿一个布尔值而
/// 造出一整条没人读的条目。
pub fn is_reportable(error: &CoreError) -> bool {
    match error {
        CoreError::Reported { code, .. } => Severity::for_core_code(code).is_some(),
        CoreError::InvalidResponse { .. } => true,
        CoreError::MissingData { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{from_core_error, is_reportable};
    use crate::Severity;
    use lithe_gpui_shared::CoreError;

    fn reported(code: &str) -> CoreError {
        CoreError::Reported {
            code: code.to_string(),
            message: "boom".to_string(),
        }
    }

    /// `Reported` 的码一比一透传：档位由 `Severity::for_core_code` 决定，`command` 与
    /// `detail` 都进参数（于是可搜、可显示在详情里）。
    #[test]
    fn reported_carries_code_detail_and_command() {
        let input = from_core_error(&reported("process_failed"), "git.status")
            .expect("process_failed 有档位");
        assert_eq!(input.severity(), Severity::Error);
        assert_eq!(
            input.code().as_ref(),
            "lithe.notifications.core.processFailed"
        );
        let params: Vec<(&str, &str)> = input
            .params()
            .iter()
            .map(|(name, value)| (name.as_ref(), value.as_ref()))
            .collect();
        assert_eq!(
            params,
            vec![
                ("coreCode", "process_failed"),
                ("detail", "boom"),
                ("command", "git.status"),
            ],
            "`command` 必须进参数，否则详情里看不出是哪条命令失败"
        );
    }

    /// `cancelled` 不进中心 —— 用户自己取消的，报成通知只会让中心被正常操作刷满。
    #[test]
    fn cancelled_is_not_reported() {
        assert!(from_core_error(&reported("cancelled"), "git.status").is_none());
        assert!(!is_reportable(&reported("cancelled")));
    }

    /// 未知码**要**进中心（降级为 warning），因为 Core 以后新增码时我们不知道它要不要
    /// 用户处理，先按降级处理而不是静默丢弃。
    #[test]
    fn unknown_code_still_reports() {
        let input = from_core_error(&reported("some_future_code"), "maven.scan")
            .expect("未知码按 warning 报");
        assert_eq!(input.severity(), Severity::Warning);
    }

    /// `InvalidResponse` 归成 `parse_failed`：Core 的返回值不是合法 JSON 就是一次解析失败。
    #[test]
    fn invalid_response_maps_to_parse_failed() {
        let error = CoreError::InvalidResponse {
            command: "spring.index".to_string(),
            message: "not json".to_string(),
        };
        let input = from_core_error(&error, "spring.index").expect("应报");
        assert_eq!(
            input.code().as_ref(),
            "lithe.notifications.core.parseFailed"
        );
        assert!(is_reportable(&error));
    }

    /// `MissingData` 不进中心：`core_client.rs:44-46` 明写「`ok:true` 但没有 data」多数
    /// 时候是「没有东西可显示」，报成通知会刷噪音。
    #[test]
    fn missing_data_is_not_reported() {
        let error = CoreError::MissingData {
            command: "git.status".to_string(),
        };
        assert!(from_core_error(&error, "git.status").is_none());
        assert!(!is_reportable(&error));
    }

    /// `is_reportable` 与 `from_core_error().is_some()` 必须**永远一致** ——
    /// 两个入口给出相反答案的话，调用点按哪个写就会出 bug。
    #[test]
    fn is_reportable_agrees_with_from_core_error() {
        let cases = [
            reported("invalid_request"),
            reported("permission_denied"),
            reported("not_supported"),
            reported("cancelled"),
            reported("timed_out"),
            reported("unknown"),
            reported("brand_new_code"),
            CoreError::InvalidResponse {
                command: "c".to_string(),
                message: "m".to_string(),
            },
            CoreError::MissingData {
                command: "c".to_string(),
            },
        ];
        for case in cases {
            assert_eq!(
                is_reportable(&case),
                from_core_error(&case, "cmd").is_some(),
                "两个入口对 {case:?} 给出相反答案"
            );
        }
    }
}
