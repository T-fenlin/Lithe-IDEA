use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use gpui_kit::component::input::{CompletionProvider, Rope};
use gpui::{App, AppContext as _, Task, Window};
use lithe_gpui_java::{JavaCompletionItem, JavaLanguageService};
use lithe_gpui_shared::core_json;
use lsp_types::{
    CompletionContext, CompletionItem, CompletionItemKind, CompletionResponse, CompletionTextEdit,
    Documentation, Position, TextEdit,
};
use serde_json::{Value, json};

const BUILTIN_COMPLETIONS: &str = "lsp.builtinCompletions";

const COMPLETION_MENU_WIDTH_PX: f32 = 480.;

const COMPLETION_DEBOUNCE_MS: u64 = 120;

pub(crate) fn menu_width(rem: gpui_kit::Pixels) -> gpui_kit::Pixels {
    crate::editor_view::rem_px(rem, COMPLETION_MENU_WIDTH_PX)
}

pub(crate) fn set_menu_width(
    state: &mut gpui_kit::component::input::EditorState,
    width: gpui_kit::Pixels,
) -> bool {
    let lsp = state.lsp_mut();
    if lsp.completion_provider.is_none() || lsp.completion_menu.max_width == width {
        return false;
    }
    lsp.completion_menu.max_width = width;
    true
}

pub(crate) fn install(
    state: &mut gpui_kit::component::input::EditorState,
    provider: Option<Rc<dyn CompletionProvider>>,
    rem: gpui_kit::Pixels,
) {
    let Some(provider) = provider else {
        return;
    };
    let lsp = state.lsp_mut();
    lsp.completion_provider = Some(provider);
    lsp.completion_menu.max_width = menu_width(rem);
}

pub(crate) struct JavaCompletionProvider {
    service: Option<Arc<JavaLanguageService>>,
    file_path: PathBuf,
    generation: Arc<AtomicU64>,
    auto_completion: Rc<AtomicBool>,
}

impl JavaCompletionProvider {
    pub(crate) fn new(
        service: Option<Arc<JavaLanguageService>>,
        file_path: PathBuf,
        auto_completion: Rc<AtomicBool>,
    ) -> Rc<dyn CompletionProvider> {
        Rc::new(Self {
            service,
            file_path,
            generation: Arc::new(AtomicU64::new(0)),
            auto_completion,
        })
    }

    fn prefix(body: &str, offset: usize) -> String {
        let mut end = offset.min(body.len());
        while end > 0 && !body.is_char_boundary(end) {
            end -= 1;
        }
        let mut begin = end;
        while begin > 0 {
            let Some(previous) = body[..begin].chars().next_back() else {
                break;
            };
            if is_identifier_char(previous) || previous == '.' || previous == '@' {
                begin -= previous.len_utf8();
            } else {
                break;
            }
        }
        body[begin..end].to_string()
    }
}

impl CompletionProvider for JavaCompletionProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<gpui_kit::Result<CompletionResponse>> {
        let (line, utf16_column) = crate::navigation::core_position(text, offset);
        let position = Position::new(line as u32, utf16_column as u32);
        let body = text.to_string();
        let prefix = Self::prefix(&body, offset);
        let rope = text.clone();
        let file_path = self.file_path.clone();
        let service = self.service.clone();
        let executor = cx.background_executor().clone();

        let mine = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = self.generation.clone();

        cx.background_spawn(async move {
            let started = Instant::now();
            executor
                .timer(std::time::Duration::from_millis(COMPLETION_DEBOUNCE_MS))
                .await;
            if generation.load(Ordering::SeqCst) != mine {
                return Ok(CompletionResponse::Array(Vec::new()));
            }

            if let Some(service) = service {
                match service.completion(&file_path, &body, position.line, position.character) {
                    Ok(items) => {
                        let count = items.len();
                        println!(
                            "S1_JAVA_COMPLETION source=jdtls prefix={prefix:?} items={count} ms={}",
                            started.elapsed().as_millis()
                        );
                        return Ok(deferred(generation, mine, items, &rope));
                    }
                    Err(error) => {
                        println!(
                            "S1_JAVA_COMPLETION source=jdtls result=failed error={error} fallback=builtin"
                        );
                    }
                }
            }

            let items = builtin_completions(&file_path, &body, position);
            println!(
                "S1_JAVA_COMPLETION source=builtin prefix={prefix:?} items={} ms={}",
                items.len(),
                started.elapsed().as_millis()
            );
            Ok(deferred(generation, mine, items, &rope))
        })
    }

    fn is_completion_trigger(&self, _offset: usize, new_text: &str, _cx: &mut App) -> bool {
        completion_triggered(self.auto_completion.load(Ordering::Acquire), new_text)
    }
}

fn completion_triggered(auto_completion: bool, new_text: &str) -> bool {
    auto_completion && triggers_on(new_text)
}

fn triggers_on(new_text: &str) -> bool {
    !new_text.is_empty()
        && new_text
            .chars()
            .all(|character| is_identifier_char(character) || character == '.' || character == '@')
}

fn deferred(
    generation: Arc<AtomicU64>,
    mine: u64,
    items: Vec<JavaCompletionItem>,
    text: &Rope,
) -> CompletionResponse {
    if generation.load(Ordering::SeqCst) != mine {
        return CompletionResponse::Array(Vec::new());
    }
    CompletionResponse::Array(
        items
            .into_iter()
            .map(|item| to_lsp_item(item, text))
            .collect(),
    )
}

fn to_lsp_item(item: JavaCompletionItem, text: &Rope) -> CompletionItem {
    CompletionItem {
        label: item.label,
        kind: item.kind.and_then(completion_kind),
        detail: item.detail,
        documentation: item.documentation.map(Documentation::String),
        sort_text: item.sort_text,
        filter_text: item.filter_text,
        insert_text: Some(item.insert_text),
        insert_text_format: Some(lsp_types::InsertTextFormat::PLAIN_TEXT),
        text_edit: item.text_edit.map(|edit| {
            CompletionTextEdit::Edit(TextEdit {
                range: lsp_types::Range {
                    start: crate::navigation::editor_position(
                        text,
                        edit.start.line as usize,
                        edit.start.utf16_column as usize,
                    ),
                    end: crate::navigation::editor_position(
                        text,
                        edit.end.line as usize,
                        edit.end.utf16_column as usize,
                    ),
                },
                new_text: edit.new_text,
            })
        }),
        ..Default::default()
    }
}

fn completion_kind(kind: i64) -> Option<CompletionItemKind> {
    Some(match kind {
        1 => CompletionItemKind::TEXT,
        2 => CompletionItemKind::METHOD,
        3 => CompletionItemKind::FUNCTION,
        4 => CompletionItemKind::CONSTRUCTOR,
        5 => CompletionItemKind::FIELD,
        6 => CompletionItemKind::VARIABLE,
        7 => CompletionItemKind::CLASS,
        8 => CompletionItemKind::INTERFACE,
        9 => CompletionItemKind::MODULE,
        10 => CompletionItemKind::PROPERTY,
        11 => CompletionItemKind::UNIT,
        12 => CompletionItemKind::VALUE,
        13 => CompletionItemKind::ENUM,
        14 => CompletionItemKind::KEYWORD,
        15 => CompletionItemKind::SNIPPET,
        16 => CompletionItemKind::COLOR,
        17 => CompletionItemKind::FILE,
        18 => CompletionItemKind::REFERENCE,
        19 => CompletionItemKind::FOLDER,
        20 => CompletionItemKind::ENUM_MEMBER,
        21 => CompletionItemKind::CONSTANT,
        22 => CompletionItemKind::STRUCT,
        23 => CompletionItemKind::EVENT,
        24 => CompletionItemKind::OPERATOR,
        25 => CompletionItemKind::TYPE_PARAMETER,
        _ => return None,
    })
}

fn builtin_completions(
    file_path: &std::path::Path,
    text: &str,
    position: Position,
) -> Vec<JavaCompletionItem> {
    let payload = json!({
        "filePath": file_path.to_string_lossy(),
        "text": text,
        "position": { "line": position.line, "utf16Column": position.character },
    });
    let Ok(Some(response)) = core_json(BUILTIN_COMPLETIONS, payload) else {
        return Vec::new();
    };
    response
        .get("items")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(builtin_item).collect())
        .unwrap_or_default()
}

fn builtin_item(value: &Value) -> Option<JavaCompletionItem> {
    let label = value.get("label").and_then(Value::as_str)?.to_string();
    let edit = value.get("textEdit")?;
    let range = edit.get("range")?;
    let start = range.get("start")?;
    let end = range.get("end")?;
    let read_position = |value: &Value| {
        Some((
            value.get("line").and_then(Value::as_u64)? as u32,
            value.get("utf16Column").and_then(Value::as_u64)? as u32,
        ))
    };
    let (start_line, start_column) = read_position(start)?;
    let (end_line, end_column) = read_position(end)?;
    let new_text = edit
        .get("newText")
        .and_then(Value::as_str)
        .unwrap_or(label.as_str())
        .to_string();

    Some(JavaCompletionItem {
        label,
        insert_text: new_text.clone(),
        kind: value.get("kind").and_then(Value::as_i64),
        detail: None,
        documentation: None,
        sort_text: None,
        filter_text: None,
        text_edit: Some(lithe_gpui_java::JavaTextEdit {
            start: lithe_gpui_java::JavaPosition {
                line: start_line,
                utf16_column: start_column,
            },
            end: lithe_gpui_java::JavaPosition {
                line: end_line,
                utf16_column: end_column,
            },
            new_text,
        }),
    })
}

fn is_identifier_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_policy_follows_java_input() {
        for text in ["a", "Z", "_", "$", "1", ".", "@", "System.out"] {
            assert!(triggers_on(text), "{text:?} 应当触发补全");
        }
        for text in ["", " ", ";", ")", "\n", "()", "a ", "= ", "//"] {
            assert!(!triggers_on(text), "{text:?} 不该触发补全");
        }
    }

    #[test]
    fn the_auto_completion_switch_gates_the_trigger_only() {
        let switch = Rc::new(AtomicBool::new(true));
        let provider = JavaCompletionProvider {
            service: None,
            file_path: PathBuf::from("Greeter.java"),
            generation: Arc::new(AtomicU64::new(0)),
            auto_completion: switch.clone(),
        };
        let engaged = |provider: &JavaCompletionProvider| {
            completion_triggered(provider.auto_completion.load(Ordering::Acquire), ".")
        };
        assert!(engaged(&provider), "默认必须是开（真源默认 true）");
        assert!(triggers_on("."), "纯函数本身不受设置影响");

        switch.store(false, Ordering::Release);
        assert!(!engaged(&provider), "关掉之后 '.' 也不许触发");
        switch.store(true, Ordering::Release);
        assert!(engaged(&provider));
    }

    #[test]
    fn prefix_stops_at_non_identifier_characters() {
        let source = "System.out.pri";
        assert_eq!(JavaCompletionProvider::prefix(source, source.len()), source);

        let source = "foo(ba";
        assert_eq!(JavaCompletionProvider::prefix(source, source.len()), "ba");

        let source = "中文 System.ou";
        assert_eq!(
            JavaCompletionProvider::prefix(source, source.len()),
            "System.ou"
        );

        assert_eq!(JavaCompletionProvider::prefix(source, 999), "System.ou");
        assert_eq!(JavaCompletionProvider::prefix(source, 1), "");
    }

    #[test]
    fn completion_kinds_map_and_unknown_values_are_dropped() {
        assert_eq!(completion_kind(2), Some(CompletionItemKind::METHOD));
        assert_eq!(completion_kind(7), Some(CompletionItemKind::CLASS));
        assert_eq!(completion_kind(21), Some(CompletionItemKind::CONSTANT));
        assert_eq!(completion_kind(0), None);
        assert_eq!(completion_kind(99), None);
    }

    #[test]
    fn item_mapping_keeps_the_replacement_range() {
        let rope = Rope::from_str("line0\nline1\nline2\n    greet(na)\n");
        let mapped = to_lsp_item(sample_item(3, 10, 3, 12, "greet(name)"), &rope);
        assert_eq!(mapped.label, "greet(String)");
        assert_eq!(mapped.sort_text.as_deref(), Some("0001"));
        assert_eq!(
            mapped.insert_text_format,
            Some(lsp_types::InsertTextFormat::PLAIN_TEXT)
        );
        match mapped.text_edit.expect("必须带 textEdit") {
            CompletionTextEdit::Edit(edit) => {
                assert_eq!(edit.range.start, Position::new(3, 10));
                assert_eq!(edit.range.end, Position::new(3, 12));
                assert_eq!(edit.new_text, "greet(name)");
            }
            other => panic!("必须是普通 Edit：{other:?}"),
        }
    }

    #[test]
    fn item_mapping_converts_utf16_columns_to_character_columns() {
        let rope = Rope::from_str("x\n🙂    na\n");
        let mapped = to_lsp_item(sample_item(1, 6, 1, 8, "name"), &rope);
        match mapped.text_edit.expect("必须带 textEdit") {
            CompletionTextEdit::Edit(edit) => {
                assert_eq!(
                    edit.range.start,
                    Position::new(1, 5),
                    "UTF-16 列 6 → 字符列 5"
                );
                assert_eq!(
                    edit.range.end,
                    Position::new(1, 7),
                    "UTF-16 列 8 → 字符列 7"
                );
            }
            other => panic!("必须是普通 Edit：{other:?}"),
        }
    }

    fn sample_item(
        start_line: u32,
        start_column: u32,
        end_line: u32,
        end_column: u32,
        new_text: &str,
    ) -> JavaCompletionItem {
        JavaCompletionItem {
            label: "greet(String)".to_string(),
            insert_text: new_text.to_string(),
            kind: Some(2),
            detail: None,
            documentation: None,
            sort_text: Some("0001".to_string()),
            filter_text: None,
            text_edit: Some(lithe_gpui_java::JavaTextEdit {
                start: lithe_gpui_java::JavaPosition {
                    line: start_line,
                    utf16_column: start_column,
                },
                end: lithe_gpui_java::JavaPosition {
                    line: end_line,
                    utf16_column: end_column,
                },
                new_text: new_text.to_string(),
            }),
        }
    }
}
