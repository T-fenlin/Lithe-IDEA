//! 诊断：把 JDTLS 的 UTF-16 坐标换算成编辑器字节偏移，再交给组件库画波浪线。
//!
//! 分工与补全/悬停同一条口径：**取数**在语言服务里、**画**在组件库里（`CodeEditor::set_diagnostics`
//! 收一份完整快照），本文件只做协议坐标到编辑器坐标的换算与严重度映射。
//!
//! 快照语义按真源走：服务端每次给的都是"这份文件的全部诊断"，所以每次推送都是**整份替换**；
//! 正文一变（`EditorEvent::Changed`）之前的偏移就全过期了，宿主会把它们清掉再等下一份
//! （上游 `DiagnosticSet` 也是在正文变化时 reset 的，同一条行为）。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{BackgroundExecutor, Context};
use lithe_gpui_java::{JavaDiagnostic, JavaLanguageService};
use lithe_ui::editor::{CodeEditor, Diagnostic};
use lithe_ui::primitives::Severity;

use crate::navigation::offset_for_utf16;

const RETRY_BACKOFF_MS: [u64; 5] = [250, 500, 1_000, 2_000, 4_000];

pub(crate) const CHANGE_DEBOUNCE: Duration = Duration::from_millis(400);

pub(crate) struct Fetched {
    pub(crate) diagnostics: Vec<JavaDiagnostic>,
    pub(crate) attempts: usize,
    pub(crate) sync_error: Option<String>,
    pub(crate) elapsed_ms: u128,
}

pub(crate) async fn fetch(
    service: Arc<JavaLanguageService>,
    path: PathBuf,
    text: String,
    executor: BackgroundExecutor,
    debounce: Duration,
) -> Fetched {
    let started = Instant::now();
    if !debounce.is_zero() {
        executor.timer(debounce).await;
    }

    let sync_error = service.sync_document(&path, &text).err();

    let mut attempts = 0;
    let mut diagnostics: Vec<JavaDiagnostic> = Vec::new();
    let mut previous: Option<Vec<JavaDiagnostic>> = None;
    for delay in RETRY_BACKOFF_MS {
        executor.timer(Duration::from_millis(delay)).await;
        attempts += 1;
        let snapshot = service.diagnostics(&path);

        let settled = !snapshot.is_empty() && previous.as_ref() == Some(&snapshot);
        previous = Some(snapshot.clone());
        diagnostics = snapshot;
        if settled {
            break;
        }
    }

    Fetched {
        diagnostics,
        attempts,
        sync_error,
        elapsed_ms: started.elapsed().as_millis(),
    }
}

/// Hands one snapshot of server diagnostics to the editor, replacing whatever was drawn before; returns how
/// many marks were written.
pub(crate) fn apply(
    state: &mut CodeEditor,
    diagnostics: &[JavaDiagnostic],
    cx: &mut Context<CodeEditor>,
) -> usize {
    let text = state.text().to_string();
    let marks = marks(diagnostics, &text);
    let written = marks.len();
    state.set_diagnostics(marks, cx);
    written
}

/// The squiggles a snapshot becomes, in the editor's own byte offsets and severities.
pub(crate) fn marks(diagnostics: &[JavaDiagnostic], text: &str) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| to_editor(diagnostic, text))
        .collect()
}

fn to_editor(diagnostic: &JavaDiagnostic, text: &str) -> Diagnostic {
    let start = offset_for_utf16(
        text,
        diagnostic.range.start.line as usize,
        diagnostic.range.start.utf16_column as usize,
    );
    let end = offset_for_utf16(
        text,
        diagnostic.range.end.line as usize,
        diagnostic.range.end.utf16_column as usize,
    );

    Diagnostic {
        // 服务端把范围发反了也只是画成空的一小段，不至于让渲染去切片一个倒过来的区间。
        range: start..end.max(start),
        severity: severity(diagnostic.severity),
        message: diagnostic.message.clone().into(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Level {
    Error,
    Warning,
    Info,
    Hint,
}

impl Level {
    pub(crate) fn from_lsp(value: Option<i64>) -> Self {
        match value {
            Some(1) => Self::Error,
            Some(2) => Self::Warning,
            Some(4) => Self::Hint,
            _ => Self::Info,
        }
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
        }
    }
}

/// LSP 的严重度数字映射到组件库的四档：`1` 错误、`2` 警告、`4` 提示，其余（含缺省）算信息。
fn severity(value: Option<i64>) -> Severity {
    match Level::from_lsp(value) {
        Level::Error => Severity::Danger,
        Level::Warning => Severity::Warning,
        Level::Info | Level::Hint => Severity::Info,
    }
}

pub(crate) fn severity_max(diagnostics: &[JavaDiagnostic]) -> &'static str {
    diagnostics
        .iter()
        .map(|diagnostic| Level::from_lsp(diagnostic.severity))
        .min()
        .map(Level::id)
        .unwrap_or("none")
}

#[cfg(test)]
mod tests {
    use lithe_gpui_java::{JavaDiagnosticRange, JavaPosition};

    use super::*;

    fn diagnostic(
        start_line: u32,
        start_column: u32,
        end_line: u32,
        end_column: u32,
        severity: Option<i64>,
    ) -> JavaDiagnostic {
        JavaDiagnostic {
            range: JavaDiagnosticRange {
                start: JavaPosition {
                    line: start_line,
                    utf16_column: start_column,
                },
                end: JavaPosition {
                    line: end_line,
                    utf16_column: end_column,
                },
            },
            severity,
            message: "Type mismatch: cannot convert from String to int".to_string(),
            source: Some("Java".to_string()),
            code: Some("TypeMismatch".to_string()),
            tags: Vec::new(),
        }
    }

    #[test]
    fn severity_follows_lsp_numbers_and_defaults_to_info() {
        assert_eq!(severity(Some(1)), Severity::Danger);
        assert_eq!(severity(Some(2)), Severity::Warning);
        assert_eq!(severity(Some(3)), Severity::Info);
        assert_eq!(severity(Some(4)), Severity::Info);
        assert_eq!(severity(None), Severity::Info);
        assert_eq!(severity(Some(0)), Severity::Info);
        assert_eq!(severity(Some(99)), Severity::Info);
    }

    #[test]
    fn severity_max_reports_the_most_severe_entry() {
        assert_eq!(severity_max(&[]), "none");
        assert_eq!(severity_max(&[diagnostic(0, 0, 0, 1, Some(2))]), "warning");
        assert_eq!(
            severity_max(&[
                diagnostic(0, 0, 0, 1, Some(3)),
                diagnostic(1, 0, 1, 1, Some(1)),
                diagnostic(2, 0, 2, 1, Some(2)),
            ]),
            "error",
            "一档 Error 就该报 error，与顺序无关"
        );
        assert_eq!(
            severity_max(&[
                diagnostic(0, 0, 0, 1, None),
                diagnostic(1, 0, 1, 1, Some(4)),
            ]),
            "info"
        );
    }

    #[test]
    fn mapping_keeps_message_and_severity() {
        let text = "int x = \"nope\";\n";
        let mapped = to_editor(&diagnostic(0, 8, 0, 14, Some(1)), text);

        assert_eq!(mapped.severity, Severity::Danger);
        assert_eq!(
            mapped.message,
            "Type mismatch: cannot convert from String to int"
        );
        assert_eq!(mapped.range, 8..14);
    }

    #[test]
    fn utf16_columns_are_converted_to_byte_offsets_for_the_underline() {
        let text = "// 🙂 int y\n";
        let mapped = to_editor(&diagnostic(0, 10, 0, 11, Some(1)), text);
        assert_eq!(
            mapped.range,
            12..13,
            "波浪线必须落在 `y` 这一个字节上（把 UTF-16 当字符解会落到别的字节）"
        );
        assert_eq!(&text[mapped.range], "y");
    }

    #[test]
    fn ascii_columns_are_unchanged() {
        let text = "// int y\n";
        let mapped = to_editor(&diagnostic(0, 7, 0, 8, Some(1)), text);
        assert_eq!(mapped.range, 7..8);
    }

    #[test]
    fn ranges_on_later_lines_use_that_line_slice() {
        let text = "int a = 1;\n// 🙂 oops\nint b = 2;\n";
        let mapped = to_editor(&diagnostic(1, 6, 1, 10, Some(2)), text);

        assert_eq!(mapped.range, 19..23);
        assert_eq!(
            &text[mapped.range], "oops",
            "字节范围必须正好覆盖 oops（差一列就会切到别的字符）"
        );
    }

    #[test]
    fn out_of_range_positions_do_not_panic() {
        let text = "int x = 1;\n";
        let mapped = to_editor(&diagnostic(99, 99, 99, 100, Some(1)), text);
        assert_eq!(
            mapped.range,
            text.len()..text.len(),
            "越界的诊断仍然入集合（范围落在文本末尾）"
        );
    }

    /// 服务端把范围发反了：读成空的一段，而不是让渲染去切一个倒过来的区间。
    #[test]
    fn a_backwards_range_reads_as_empty() {
        let text = "int x = 1;\n";
        let mapped = to_editor(&diagnostic(0, 9, 0, 4, Some(1)), text);
        assert_eq!(mapped.range, 9..9);
    }

    /// 每次推送都是整份快照：给的小了就只剩小的那些条目，给空的就一条不剩。
    #[test]
    fn a_snapshot_replaces_the_one_before_it() {
        let text = "int a = 1;\nint b = 2;\n";
        let two = marks(
            &[
                diagnostic(0, 0, 0, 3, Some(1)),
                diagnostic(1, 0, 1, 3, Some(1)),
            ],
            text,
        );
        assert_eq!(two.len(), 2);

        let one = marks(&[diagnostic(0, 0, 0, 3, Some(2))], text);
        assert_eq!(one.len(), 1, "旧的那一条必须被整份替换掉");
        assert_eq!(one[0].severity, Severity::Warning);

        assert!(marks(&[], text).is_empty(), "空快照必须把波浪线全部清掉");
    }
}
