//! 严重性分档，以及 Core 稳定错误码到档位的映射。
//!
//! ## 三档，没有 `success`
//!
//! 中心只有 [`Severity::Info`] / [`Severity::Warning`] / [`Severity::Error`]。
//! 旧 Windows 产品有四档（`notifications.types.ts:3` 的
//! `"info" | "success" | "warning" | "error"`），本模块**不引入** `success`：成功不是
//! 事后还需要回看的信息，它只该占用现有的绿条
//! （`gpui/crates/git/src/changes_view.rs:913` 的 `render_notice`），而中心的容量是
//! 100 条、里面每一条都要占用户的注意力。旧产品的角标本来就排除 success
//! （`notifications-trigger.tsx:24`），这一档留着只会长期躺在列表里。
//!
//! ## 图标与颜色不在本模块
//!
//! 一个档位该配哪个图标、哪个主题色，是**展示层**的决定，所以本模块既不返回
//! `IconName` 也不碰 `Hsla` —— 那样才能让这一层在没有 GPUI 窗口的普通单测里直接跑。
//! 映射写在 `gpui/crates/workbench/src/right_tool_window.rs` 的 `severity_icon` /
//! `severity_color`。
//!
//! ## 11 个 Core 错误码
//!
//! 码来自 `rust/lithe-core/src/protocol/error.rs:11-34`，序列化形态是 **snake_case**
//! （同文件 `:6` 的 `#[serde(rename_all = "snake_case")]`）—— 不是 camelCase，这一点在
//! `gpui/crates/shared/src/core_client.rs:225` 的默认值 `"unknown"` 与 `:261` 的
//! `reported("invalid_request", …)` 判据上可以直接对上。
//!
//! 分档依据是「要不要用户处理」：
//!
//! | 档 | 码 | 为什么 |
//! | --- | --- | --- |
//! | `error` | `invalid_request`、`workspace_not_found`、`permission_denied`、`process_start_failed`、`process_failed`、`parse_failed` | 事情没做成，用户要么改输入、要么改环境 |
//! | `warning` | `not_supported`、`runtime_missing`、`timed_out`、`unknown` | 降级但没坏，IDE 还能用 |
//! | 不入中心 | `cancelled` | 调用方自己协作式取消的，记进去只会让中心被用户的正常操作刷满 |
//!
//! ## 未知码一律 `warning`
//!
//! 认不出来的码走 [`Severity::for_core_code`] 的兜底分支，判成 `warning` 而不是
//! `error`。方向是「不要谎报严重性」：Core 以后新增一个码时，我们不知道它到底要不要
//! 用户处理，先按降级处理；等这个码在 Core 侧有明确语义了再挪进 `error` 那张表。
//! 反过来（先判 `error`）会把一个可能无关紧要的码染红，而红点是用户唯一会盯着看的信号。

/// 通知的严重性档位。**故意不含 `success`**，理由见模块文档。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// 常规事件。
    Info,
    /// 降级但没坏。
    Warning,
    /// 事情没做成。
    Error,
}

impl Severity {
    /// Core 稳定错误码 → 档位。`None` 表示**不入中心**（目前只有 `cancelled`）。
    ///
    /// 参数是 `core_client::CoreError::Reported { code }` 里那个**字符串**形态
    /// （`shared/src/core_client.rs:222-226`），不是 Rust 的 `ErrorCode` 枚举 ——
    /// Core 响应是 JSON，枚举到不了这一层。
    pub fn for_core_code(code: &str) -> Option<Self> {
        match code {
            // 调用方自己取消的，不是通知。
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
            // 未知码按降级处理，见模块文档「未知码一律 warning」。
            _ => Some(Severity::Warning),
        }
    }

    /// 稳定的事件码（存在 [`crate::model::Entry`] 里的那个 `tr` 键）。
    ///
    /// Core 错误码 → 通知键的映射表。**不要**把 `error.rs` 的 11 个码直接当键用：键要
    /// 落在 `lithe.notifications.*` 命名空间下才能被 `tr_args` 解析到，而且键名要是
    /// 人能读的（`processStartFailed` 而不是 `process_start_failed`）。
    ///
    /// 两侧的对应关系是确定的：一个 Core 码固定映射到一个通知键，而那个键的严重性由
    /// [`Severity::for_core_code`] 决定 —— 所以这个表**不**重复存严重性，避免两处漂移。
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
            // `cancelled` 走不到这里（`for_core_code` 先把它挡掉了），但给出兜底而不是
            // `unreachable!()`：Core 新增码时这条会静默落到 unknown 键，而不是崩掉。
            _ => "lithe.notifications.core.unknown",
        }
    }
}

/// 铃铛角标的形状。
///
/// 决定与镜像 IDEA：官方文档写的是工具窗标题旁一个**点** —— 「a blue dot marks regular
/// events and unimportant suggestions. A red dot marks errors and important suggestions.」
/// 也就是**颜色表严重性、不表数量**。
///
/// 不选数字角标的理由：`windows/tauri/src/features/notifications/components/
/// notifications-trigger.tsx:45-47` 那个 `9+` 看起来更有信息量，但它要求一整套
/// read/unread 状态机，而那套状态机唯一的实际效果就是
/// `gpui/docs/archive/ui-map-macos.md:96` 点名的缺陷（未读红点形同虚设）。
/// `gpui-component-0.6.6/src/badge.rs:64,76` 的 `count` / `max` 随时能加回来。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Badge {
    /// 无未读，不画。
    None,
    /// 有未读，但不是错误 → 蓝点。
    Info,
    /// 有未读错误 → 红点。**红优先于蓝**：只要有一条未读错误就是红点。
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 11 个 Core 码必须全部有明确归属：新增/改名时这条会红，而不是静默走兜底分支。
    ///
    /// 码名逐字取自 `rust/lithe-core/src/protocol/error.rs:11-34`（snake_case 形态）。
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
        assert_eq!(ALL_CORE_CODES.len(), 11, "Core 错误码数量变了，要更新分档表");
    }

    /// 分档方向不许反：需要用户处理的进 `error`，降级的进 `warning`。
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
            assert_eq!(Severity::for_core_code(code), Some(Severity::Error), "{code}");
        }
        for code in ["not_supported", "runtime_missing", "timed_out", "unknown"] {
            assert_eq!(
                Severity::for_core_code(code),
                Some(Severity::Warning),
                "{code}"
            );
        }
    }

    /// 未知码按 `warning` 而不是 `error`：不谎报严重性。
    #[test]
    fn unknown_code_degrades_instead_of_alarming() {
        assert_eq!(
            Severity::for_core_code("some_future_code"),
            Some(Severity::Warning)
        );
    }

    /// 通知键必须落在 `lithe.notifications.*` 下、且是 camelCase —— `tr_args` 只认
    /// 注册过的键，键名写错的表现是界面上漏出花括号原文而不是报错。
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

    /// 同一个 Core 码每次都映射到同一个键（不能带随机或时间成分）。
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
