use lithe_gpui_shared::CoreError;

use crate::severity::Severity;
use crate::store::Input;

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
        CoreError::MissingData { .. } => None,
    }
}

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

    #[test]
    fn cancelled_is_not_reported() {
        assert!(from_core_error(&reported("cancelled"), "git.status").is_none());
        assert!(!is_reportable(&reported("cancelled")));
    }

    #[test]
    fn unknown_code_still_reports() {
        let input = from_core_error(&reported("some_future_code"), "maven.scan")
            .expect("未知码按 warning 报");
        assert_eq!(input.severity(), Severity::Warning);
    }

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

    #[test]
    fn missing_data_is_not_reported() {
        let error = CoreError::MissingData {
            command: "git.status".to_string(),
        };
        assert!(from_core_error(&error, "git.status").is_none());
        assert!(!is_reportable(&error));
    }

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
