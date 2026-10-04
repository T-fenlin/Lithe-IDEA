use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::BackgroundExecutor;
use gpui_kit::base::input::{Diagnostic, DiagnosticSet, DiagnosticSeverity};
use gpui_kit::component::input::{EditorState, Rope};
use lithe_gpui_java::{JavaDiagnostic, JavaLanguageService};

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

pub(crate) fn apply(state: &mut EditorState, diagnostics: &[JavaDiagnostic]) -> usize {
    let text = state.text().clone();
    let Some(set) = state.diagnostics_mut() else {
        return 0;
    };
    write(set, &text, diagnostics)
}

fn write(set: &mut DiagnosticSet, text: &Rope, diagnostics: &[JavaDiagnostic]) -> usize {
    set.clear();
    for diagnostic in diagnostics {
        set.push(to_upstream(diagnostic, text));
    }
    diagnostics.len()
}

fn to_upstream(diagnostic: &JavaDiagnostic, text: &Rope) -> Diagnostic {
    let start = crate::navigation::editor_position(
        text,
        diagnostic.range.start.line as usize,
        diagnostic.range.start.utf16_column as usize,
    );
    let end = crate::navigation::editor_position(
        text,
        diagnostic.range.end.line as usize,
        diagnostic.range.end.utf16_column as usize,
    );

    let mut mapped = Diagnostic::new(start..end, diagnostic.message.clone())
        .with_severity(severity(diagnostic.severity));
    if let Some(source) = &diagnostic.source {
        mapped = mapped.with_source(source.clone());
    }
    if let Some(code) = &diagnostic.code {
        mapped = mapped.with_code(code.clone());
    }
    mapped
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

fn severity(value: Option<i64>) -> DiagnosticSeverity {
    match Level::from_lsp(value) {
        Level::Error => DiagnosticSeverity::Error,
        Level::Warning => DiagnosticSeverity::Warning,
        Level::Info => DiagnosticSeverity::Info,
        Level::Hint => DiagnosticSeverity::Hint,
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
    use gpui_kit::component::input::Position;
    use lithe_gpui_java::{JavaDiagnostic, JavaDiagnosticRange, JavaPosition};

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
        assert_eq!(severity(Some(1)), DiagnosticSeverity::Error);
        assert_eq!(severity(Some(2)), DiagnosticSeverity::Warning);
        assert_eq!(severity(Some(3)), DiagnosticSeverity::Info);
        assert_eq!(severity(Some(4)), DiagnosticSeverity::Hint);
        assert_eq!(severity(None), DiagnosticSeverity::Info);
        assert_eq!(severity(Some(0)), DiagnosticSeverity::Info);
        assert_eq!(severity(Some(99)), DiagnosticSeverity::Info);
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
    fn mapping_keeps_message_severity_source_and_code() {
        let rope = Rope::from_str("int x = \"nope\";\n");
        let mapped = to_upstream(&diagnostic(0, 8, 0, 14, Some(1)), &rope);

        assert_eq!(mapped.severity, DiagnosticSeverity::Error);
        assert_eq!(
            mapped.message.as_str(),
            "Type mismatch: cannot convert from String to int"
        );
        assert_eq!(mapped.source.as_ref().map(|s| s.as_str()), Some("Java"));
        assert_eq!(
            mapped.code.as_ref().map(|s| s.as_str()),
            Some("TypeMismatch")
        );
        assert_eq!(mapped.range.start, Position::new(0, 8));
        assert_eq!(mapped.range.end, Position::new(0, 14));
    }

    #[test]
    fn utf16_columns_are_converted_to_character_columns_for_the_underline() {
        let text = Rope::from_str("// 🙂 int y\n");
        let mapped = to_upstream(&diagnostic(0, 10, 0, 11, Some(1)), &text);
        assert_eq!(
            mapped.range.start,
            Position::new(0, 9),
            "UTF-16 列 10 → 字符列 9"
        );

        let mut set = DiagnosticSet::new(&text);
        set.push(mapped);
        assert_eq!(set.len(), 1);
        let entry = set.for_offset(12).expect("第 12 字节应当落在诊断范围内");
        assert_eq!(
            entry.range,
            12..13,
            "波浪线必须落在 `y` 这一个字节上（把 UTF-16 当字符解会落到别的字节）"
        );
    }

    #[test]
    fn ascii_columns_are_unchanged() {
        let text = Rope::from_str("// int y\n");
        let mut set = DiagnosticSet::new(&text);
        set.push(to_upstream(&diagnostic(0, 7, 0, 8, Some(1)), &text));
        let entry = set.for_offset(7).expect("第 7 字节应当落在诊断范围内");
        assert_eq!(entry.range, 7..8);
    }

    #[test]
    fn ranges_on_later_lines_use_that_line_slice() {
        let text = Rope::from_str("int a = 1;\n// 🙂 oops\nint b = 2;\n");
        let mut set = DiagnosticSet::new(&text);
        set.push(to_upstream(&diagnostic(1, 6, 1, 10, Some(2)), &text));

        let entry = set.for_offset(19).expect("诊断必须落在行 1 的 oops 上");
        assert_eq!(entry.range, 19..23);
        assert_eq!(
            &text.to_string()[entry.range.clone()],
            "oops",
            "字节范围必须正好覆盖 oops（差一列就会切到别的字符）"
        );
    }

    #[test]
    fn out_of_range_positions_do_not_panic() {
        let text = Rope::from_str("int x = 1;\n");
        let mut set = DiagnosticSet::new(&text);
        set.push(to_upstream(&diagnostic(99, 99, 99, 100, Some(1)), &text));
        assert_eq!(set.len(), 1, "越界的诊断仍然入集合（范围落在文本末尾）");
    }

    #[test]
    fn writing_a_smaller_snapshot_drops_the_previous_entries() {
        let text = Rope::from_str("int a = 1;\nint b = 2;\n");
        let mut set = DiagnosticSet::new(&text);
        let written = write(
            &mut set,
            &text,
            &[
                diagnostic(0, 0, 0, 3, Some(1)),
                diagnostic(1, 0, 1, 3, Some(1)),
            ],
        );
        assert_eq!(written, 2);
        assert_eq!(set.len(), 2);

        let written = write(&mut set, &text, &[diagnostic(0, 0, 0, 3, Some(2))]);
        assert_eq!(written, 1);
        assert_eq!(set.len(), 1, "旧的那一条必须被整份替换掉");
        assert_eq!(
            set.for_offset(0).expect("新的那条还在").severity,
            DiagnosticSeverity::Warning
        );
        assert!(
            set.for_offset(11).is_none(),
            "第二行（字节 11 起）的诊断必须已经被清掉"
        );

        write(&mut set, &text, &[]);
        assert_eq!(set.len(), 0, "空快照必须把波浪线全部清掉");
    }

    #[test]
    fn upstream_reset_clears_diagnostics_on_text_change() {
        let text = Rope::from_str("int a = 1;\n");
        let mut set = DiagnosticSet::new(&text);
        set.push(Diagnostic::new(
            Position::new(0, 0)..Position::new(0, 3),
            "boom",
        ));
        assert_eq!(set.len(), 1);

        set.reset(&Rope::from_str("int a = 2;\n"));
        assert_eq!(
            set.len(),
            0,
            "上游在每次正文变化时 reset：缓存旧快照再重放会让波浪线回到旧位置"
        );
    }
}
