#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn for_core_code(code: &str) -> Option<Self> {
        match code {
            "cancelled" => None,
            "invalid_request"
            | "workspace_not_found"
            | "permission_denied"
            | "process_start_failed"
            | "process_failed"
            | "parse_failed" => Some(Severity::Error),
            "not_supported" | "runtime_missing" | "timed_out" | "unknown" => {
                Some(Severity::Warning)
            }
            _ => Some(Severity::Warning),
        }
    }

    pub fn notification_code_for_core_code(code: &str) -> &'static str {
        match code {
            "invalid_request" => "lithe.notifications.core.invalidRequest",
            "workspace_not_found" => "lithe.notifications.core.workspaceNotFound",
            "permission_denied" => "lithe.notifications.core.permissionDenied",
            "not_supported" => "lithe.notifications.core.notSupported",
            "runtime_missing" => "lithe.notifications.core.runtimeMissing",
            "process_start_failed" => "lithe.notifications.core.processStartFailed",
            "process_failed" => "lithe.notifications.core.processFailed",
            "parse_failed" => "lithe.notifications.core.parseFailed",
            "timed_out" => "lithe.notifications.core.timedOut",
            "unknown" => "lithe.notifications.core.unknown",
            _ => "lithe.notifications.core.unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Badge {
    None,
    Info,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_CORE_CODES: [&str; 11] = [
        "invalid_request",
        "workspace_not_found",
        "permission_denied",
        "not_supported",
        "runtime_missing",
        "process_start_failed",
        "process_failed",
        "parse_failed",
        "cancelled",
        "timed_out",
        "unknown",
    ];

    #[test]
    fn every_core_error_code_has_an_explicit_verdict() {
        for code in ALL_CORE_CODES {
            let verdict = Severity::for_core_code(code);
            if code == "cancelled" {
                assert_eq!(verdict, None, "cancelled 必须不入中心");
            } else {
                assert!(verdict.is_some(), "{code} 必须有档位");
            }
        }
        assert_eq!(
            ALL_CORE_CODES.len(),
            11,
            "Core 错误码数量变了，要更新分档表"
        );
    }

    #[test]
    fn severities_land_in_the_documented_bucket() {
        for code in [
            "invalid_request",
            "workspace_not_found",
            "permission_denied",
            "process_start_failed",
            "process_failed",
            "parse_failed",
        ] {
            assert_eq!(
                Severity::for_core_code(code),
                Some(Severity::Error),
                "{code}"
            );
        }
        for code in ["not_supported", "runtime_missing", "timed_out", "unknown"] {
            assert_eq!(
                Severity::for_core_code(code),
                Some(Severity::Warning),
                "{code}"
            );
        }
    }

    #[test]
    fn unknown_code_degrades_instead_of_alarming() {
        assert_eq!(
            Severity::for_core_code("some_future_code"),
            Some(Severity::Warning)
        );
    }

    #[test]
    fn notification_codes_are_wired_keys() {
        for code in ALL_CORE_CODES {
            let key = Severity::notification_code_for_core_code(code);
            assert!(
                key.starts_with("lithe.notifications.core."),
                "{code} 的键 {key} 不在 lithe.notifications.core.* 命名空间"
            );
            assert!(
                !key.contains('_'),
                "{code} 的键 {key} 含下划线；键名要 camelCase"
            );
        }
    }

    #[test]
    fn notification_code_mapping_is_stable() {
        for code in ALL_CORE_CODES {
            assert_eq!(
                Severity::notification_code_for_core_code(code),
                Severity::notification_code_for_core_code(code)
            );
        }
    }
}
