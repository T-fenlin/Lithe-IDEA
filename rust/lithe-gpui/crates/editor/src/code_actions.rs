use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use gpui::{App, AppContext as _, Entity, SharedString, Task, Window};
use gpui_kit::component::input::{CodeActionProvider, EditorState};
use lithe_gpui_java::{JavaCodeAction, JavaLanguageService, JavaPosition};
use lsp_types::{CodeAction, CodeActionKind, Position, TextEdit, Uri, WorkspaceEdit};

pub(crate) fn install(state: &mut EditorState, provider: Option<Rc<dyn CodeActionProvider>>) {
    let Some(provider) = provider else {
        return;
    };
    state.lsp_mut().code_action_providers = vec![provider];
}

pub(crate) struct JavaCodeActionProvider {
    service: Arc<JavaLanguageService>,
    file_path: PathBuf,
    generation: Arc<AtomicU64>,
}

impl JavaCodeActionProvider {
    pub(crate) fn new(
        service: Arc<JavaLanguageService>,
        file_path: PathBuf,
    ) -> Rc<dyn CodeActionProvider> {
        Rc::new(Self {
            service,
            file_path,
            generation: Arc::new(AtomicU64::new(0)),
        })
    }

    fn file_uri(&self) -> Option<Uri> {
        let uri = lithe_gpui_java::file_uri(&self.file_path).ok()?;
        Uri::from_str(&uri).ok()
    }
}

impl CodeActionProvider for JavaCodeActionProvider {
    fn id(&self) -> SharedString {
        "lithe-java-code-actions".into()
    }

    fn code_actions(
        &self,
        state: Entity<EditorState>,
        range: std::ops::Range<usize>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<gpui::Result<Vec<CodeAction>>> {
        let text = state.read(cx).text().clone();
        let body = text.to_string();
        let (start_line, start_column) = crate::navigation::core_position(&text, range.start);
        let (end_line, end_column) = crate::navigation::core_position(&text, range.end);
        let start = JavaPosition {
            line: start_line as u32,
            utf16_column: start_column as u32,
        };
        let end = JavaPosition {
            line: end_line as u32,
            utf16_column: end_column as u32,
        };

        let Some(uri) = self.file_uri() else {
            let file = self.file_path.display().to_string();
            println!("S1_EDITOR_CODE_ACTION file={file} actions=0 reason=no-file-uri");
            return Task::ready(Ok(Vec::new()));
        };
        let file = self.file_path.display().to_string();
        let file_path = self.file_path.clone();
        let service = self.service.clone();
        let mine = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = self.generation.clone();

        cx.background_spawn(async move {
            let started = Instant::now();
            let actions = match service.code_actions(&file_path, &body, start, end) {
                Ok(actions) => actions,
                Err(error) => {
                    println!(
                        "S1_EDITOR_CODE_ACTION file={file} actions=0 result=failed error={error}"
                    );
                    return Ok(Vec::new());
                }
            };

            if generation.load(Ordering::SeqCst) != mine {
                println!(
                    "S1_EDITOR_CODE_ACTION file={file} actions={} result=stale reason=superseded",
                    actions.len()
                );
                return Ok(Vec::new());
            }

            let kinds = kind_summary(&actions);
            let mapped: Vec<CodeAction> = actions
                .into_iter()
                .map(|action| to_lsp_action(action, &uri))
                .collect();
            println!(
                "S1_EDITOR_CODE_ACTION file={file} actions={} kind={kinds} line={} col={} ms={}",
                mapped.len(),
                start.line,
                start.utf16_column,
                started.elapsed().as_millis()
            );
            Ok(mapped)
        })
    }

    fn perform_code_action(
        &self,
        state: Entity<EditorState>,
        action: CodeAction,
        _push_to_history: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<gpui::Result<()>> {
        let file = self.file_path.display().to_string();
        let title = action.title.clone();
        let Some(uri) = self.file_uri() else {
            println!(
                "S1_EDITOR_CODE_ACTION file={file} result=skipped reason=no-file-uri title={title:?}"
            );
            return Task::ready(Ok(()));
        };
        let Some(edits) = action
            .edit
            .as_ref()
            .and_then(|edit| edit.changes.as_ref())
            .and_then(|changes| changes.get(&uri))
        else {
            println!(
                "S1_EDITOR_CODE_ACTION file={file} result=skipped reason=no-edits-for-this-file title={title:?}"
            );
            return Task::ready(Ok(()));
        };
        let edits = edits.clone();
        let handle = window.window_handle();

        cx.spawn(async move |cx| {
            let outcome = cx.update_window(handle, |_, window, cx| {
                state.update(cx, |state, cx| {
                    let text = state.text().clone();
                    let converted = to_editor_edits(&text, &edits);
                    let count = converted.len();
                    state.apply_lsp_edits(&converted, window, cx);
                    count
                })
            });

            match outcome {
                Ok(count) => println!(
                    "S1_EDITOR_CODE_ACTION file={file} result=applied edits={count} title={title:?}"
                ),
                Err(error) => println!(
                    "S1_EDITOR_CODE_ACTION file={file} result=skipped reason=window-gone error={error}"
                ),
            }
            Ok(())
        })
    }
}

fn to_editor_edits(text: &gpui_kit::component::input::Rope, edits: &[TextEdit]) -> Vec<TextEdit> {
    let mut converted: Vec<TextEdit> = edits
        .iter()
        .map(|edit| TextEdit {
            range: lsp_types::Range {
                start: crate::navigation::editor_position(
                    text,
                    edit.range.start.line as usize,
                    edit.range.start.character as usize,
                ),
                end: crate::navigation::editor_position(
                    text,
                    edit.range.end.line as usize,
                    edit.range.end.character as usize,
                ),
            },
            new_text: edit.new_text.clone(),
        })
        .collect();
    converted
        .sort_by_key(|edit| std::cmp::Reverse((edit.range.start.line, edit.range.start.character)));
    converted
}

fn to_lsp_action(action: JavaCodeAction, uri: &Uri) -> CodeAction {
    let edits: Vec<TextEdit> = action
        .edits
        .iter()
        .map(|edit| TextEdit {
            range: lsp_types::Range {
                start: Position::new(edit.start.line, edit.start.utf16_column),
                end: Position::new(edit.end.line, edit.end.utf16_column),
            },
            new_text: edit.new_text.clone(),
        })
        .collect();
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), edits);

    CodeAction {
        title: action.title,
        kind: action.kind.map(CodeActionKind::from),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        ..Default::default()
    }
}

fn kind_summary(actions: &[JavaCodeAction]) -> String {
    let mut kinds: Vec<&str> = Vec::new();
    for action in actions {
        let kind = action.kind.as_deref().unwrap_or("none");
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    if kinds.is_empty() {
        return "none".to_string();
    }
    if kinds.len() > 4 {
        kinds.truncate(4);
        kinds.push("…");
    }
    kinds.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_action() -> JavaCodeAction {
        JavaCodeAction {
            title: "Import 'List' (java.util)".to_string(),
            kind: Some("quickfix".to_string()),
            edits: vec![lithe_gpui_java::JavaTextEdit {
                start: JavaPosition {
                    line: 0,
                    utf16_column: 0,
                },
                end: JavaPosition {
                    line: 2,
                    utf16_column: 0,
                },
                new_text: "package demo;\n\nimport java.util.List;\n\n".to_string(),
            }],
            other_file_edits: 0,
        }
    }

    #[test]
    fn action_mapping_keeps_title_kind_and_utf16_edit() {
        let uri = test_uri();
        let mapped = to_lsp_action(sample_action(), &uri);

        assert_eq!(mapped.title, "Import 'List' (java.util)");
        assert_eq!(
            mapped.kind.as_ref().map(CodeActionKind::as_str),
            Some("quickfix")
        );
        let changes = mapped
            .edit
            .expect("必须带 edit：上游不会替我们应用")
            .changes
            .expect("changes 是 Core 归一化后的形状");
        let edits = changes.get(&uri).expect("键必须是本文件的 URI");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].range.start, Position::new(0, 0));
        assert_eq!(edits[0].range.end, Position::new(2, 0));
        assert_eq!(
            edits[0].new_text,
            "package demo;\n\nimport java.util.List;\n\n"
        );
    }

    #[test]
    fn action_mapping_tolerates_a_missing_kind() {
        let uri = test_uri();
        let mut action = sample_action();
        action.kind = None;
        let mapped = to_lsp_action(action, &uri);
        assert!(mapped.kind.is_none());
        assert_eq!(
            kind_summary(&[JavaCodeAction {
                kind: None,
                ..sample_action()
            }]),
            "none"
        );
    }

    fn test_uri() -> Uri {
        let raw = lithe_gpui_java::file_uri(std::path::Path::new(
            "C:/ws/src/main/java/demo/ImportFix.java",
        ))
        .expect("路径 → file URI");
        assert!(raw.starts_with("file:///"), "{raw}");
        Uri::from_str(&raw).expect("lsp_types 必须解析得了这条 URI")
    }

    #[test]
    fn edit_key_round_trips_through_the_workspace_edit() {
        let uri = test_uri();
        let mapped = to_lsp_action(sample_action(), &uri);
        let changes = mapped.edit.expect("edit").changes.expect("changes");
        assert!(
            changes.contains_key(&uri),
            "键必须与查询用的 URI 逐字节相同：{:?}",
            changes.keys().map(|key| key.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn edits_are_converted_to_character_columns_and_sorted_back_to_front() {
        let rope = gpui_kit::component::input::Rope::from_str("x\n🙂    List<String> y;\n");
        let edits = vec![
            TextEdit {
                range: lsp_types::Range {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
                new_text: "import java.util.List;\n".to_string(),
            },
            TextEdit {
                range: lsp_types::Range {
                    start: Position::new(1, 6),
                    end: Position::new(1, 10),
                },
                new_text: "java.util.List".to_string(),
            },
        ];
        let converted = to_editor_edits(&rope, &edits);

        assert_eq!(converted.len(), 2);
        assert_eq!(
            converted[0].range.start,
            Position::new(1, 5),
            "UTF-16 列 6 → 字符列 5（倒序之后第一条是行 1 那条）"
        );
        assert_eq!(
            converted[0].range.end,
            Position::new(1, 9),
            "UTF-16 列 10 → 字符列 9"
        );
        assert_eq!(converted[0].new_text, "java.util.List");
        assert_eq!(converted[1].range.start, Position::new(0, 0), "行 0 在后面");
    }

    #[test]
    fn ascii_edits_are_unchanged() {
        let rope =
            gpui_kit::component::input::Rope::from_str("package demo;\n\npublic class A {}\n");
        let edits = vec![TextEdit {
            range: lsp_types::Range {
                start: Position::new(0, 0),
                end: Position::new(2, 0),
            },
            new_text: "package demo;\n\nimport java.util.List;\n\n".to_string(),
        }];
        let converted = to_editor_edits(&rope, &edits);
        assert_eq!(converted[0].range.start, Position::new(0, 0));
        assert_eq!(converted[0].range.end, Position::new(2, 0));
    }

    #[test]
    fn kind_summary_dedupes_and_truncates() {
        let make = |kind: &str| JavaCodeAction {
            kind: Some(kind.to_string()),
            ..sample_action()
        };
        assert_eq!(
            kind_summary(&[
                make("quickfix"),
                make("quickfix"),
                make("source.generate.accessors"),
            ]),
            "quickfix,source.generate.accessors"
        );
        assert_eq!(
            kind_summary(&[make("a"), make("b"), make("c"), make("d"), make("e"),]),
            "a,b,c,d,…"
        );
        assert_eq!(kind_summary(&[]), "none");
    }
}
