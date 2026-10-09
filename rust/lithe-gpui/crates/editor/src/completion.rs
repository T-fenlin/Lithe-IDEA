//! 补全：向 JDTLS 取数、换算成组件库的补全项，并算出"接受一条"要落的那一次文本编辑。
//!
//! 分工：组件库只**报时机**（`EditorEvent::CompletionRequested`，`ctrl-space` 与自动补全两条路都
//! 汇到它）与**存数据**（`CodeEditor::set_completions`）；取数、弹层的画与收、接受时怎么改文本
//! 都归宿主。所以本文件里没有"provider trait"——请求由宿主发起，答案由宿主回填。

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use gpui::BackgroundExecutor;
use lithe_gpui_java::{JavaCompletionItem, JavaLanguageService};
use lithe_gpui_shared::core_json;
use lithe_ui::editor::{CompletionItem as UiCompletion, SymbolKind};
use serde_json::{Value, json};

use crate::navigation::{core_position, offset_for_utf16};

const BUILTIN_COMPLETIONS: &str = "lsp.builtinCompletions";

/// 每次请求先等这么久再打服务端：连打几个字符只问最后一次。
const COMPLETION_DEBOUNCE: Duration = Duration::from_millis(120);

pub(crate) struct Fetched {
    pub(crate) items: Vec<JavaCompletionItem>,
    pub(crate) source: &'static str,
    pub(crate) prefix: String,
    pub(crate) elapsed_ms: u128,
}

/// 取一次补全；返回 `None` 表示这次请求已经被更晚的一次顶掉（**不要**把空列表当成"没有补全"）。
pub(crate) async fn fetch(
    service: Option<Arc<JavaLanguageService>>,
    file_path: PathBuf,
    text: String,
    offset: usize,
    executor: BackgroundExecutor,
    generation: Arc<AtomicU64>,
    mine: u64,
) -> Option<Fetched> {
    let started = Instant::now();
    let prefix = typed_prefix(&text, offset);
    executor.timer(COMPLETION_DEBOUNCE).await;
    if generation.load(Ordering::SeqCst) != mine {
        return None;
    }

    let (line, utf16_column) = core_position(&text, offset);
    if let Some(service) = service {
        match service.completion(&file_path, &text, line as u32, utf16_column as u32) {
            Ok(items) => {
                println!(
                    "S1_JAVA_COMPLETION source=jdtls prefix={prefix:?} items={} ms={}",
                    items.len(),
                    started.elapsed().as_millis()
                );
                return Some(Fetched {
                    items,
                    source: "jdtls",
                    prefix,
                    elapsed_ms: started.elapsed().as_millis(),
                });
            }
            Err(error) => println!(
                "S1_JAVA_COMPLETION source=jdtls result=failed error={error} fallback=builtin"
            ),
        }
    }

    let items = builtin_completions(&file_path, &text, line, utf16_column);
    println!(
        "S1_JAVA_COMPLETION source=builtin prefix={prefix:?} items={} ms={}",
        items.len(),
        started.elapsed().as_millis()
    );
    Some(Fetched {
        items,
        source: "builtin",
        prefix,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

/// What the diagnostic line reports as "what was typed": letters, digits, `_`, `$`, `@` and `.` right before
/// a byte offset. The dots are kept because `System.out.pri` is what the user actually has in front of the
/// caret, and that is worth seeing in the log.
pub(crate) fn typed_prefix(text: &str, offset: usize) -> String {
    walk_back(text, offset, true)
}

/// The **last** identifier segment before a byte offset: the text a list is filtered by, and the span an item
/// without a `textEdit` of its own replaces. `.` and `@` end it, so accepting `println` after `System.out.`
/// replaces `pri` rather than the whole dotted path.
pub(crate) fn word_prefix(text: &str, offset: usize) -> String {
    walk_back(text, offset, false)
}

fn walk_back(text: &str, offset: usize, dotted: bool) -> String {
    let mut end = offset.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut begin = end;
    while begin > 0 {
        let Some(previous) = text[..begin].chars().next_back() else {
            break;
        };
        if is_identifier_char(previous) || (dotted && (previous == '.' || previous == '@')) {
            begin -= previous.len_utf8();
        } else {
            break;
        }
    }
    text[begin..end].to_string()
}

fn is_identifier_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

/// One server item as the library's row: label, kind's mark, detail and documentation.
///
/// The library has eleven symbol kinds where LSP has twenty-five, so the mark is a coarse one
/// (`CLASS`/`STRUCT`/`INTERFACE` share `Struct`/`Trait`, and so on); the label, the type detail and the
/// documentation are the server's own words, unchanged.
pub(crate) fn to_ui(item: &JavaCompletionItem) -> UiCompletion {
    let mut ui = UiCompletion::new(item.label.clone(), symbol_kind(item.kind));
    if let Some(detail) = &item.detail {
        ui = ui.detail(detail.clone());
    }
    if let Some(docs) = &item.documentation {
        ui = ui.docs(docs.clone());
    }
    ui
}

/// Every item's row, in the server's own order — the same order the library filters and draws.
pub(crate) fn rows(items: &[JavaCompletionItem]) -> Vec<UiCompletion> {
    items.iter().map(to_ui).collect()
}

/// The server item a picked row means.
///
/// The library filters and re-orders the rows it draws (best score first), so a picked row's number is an
/// index into *that* order and cannot be used on the server's list. The pairing walks the picked rows back
/// to the first server item with the same value that no earlier row has claimed: two items equal in label,
/// kind, detail and documentation are the same row to the user, so either one is the right answer.
pub(crate) fn pick<'a>(
    items: &'a [JavaCompletionItem],
    rows: &[UiCompletion],
    at: usize,
) -> Option<&'a JavaCompletionItem> {
    rows.get(at)?;
    let mut by_label: HashMap<&str, Vec<usize>> = HashMap::new();
    for (ix, item) in items.iter().enumerate() {
        by_label.entry(item.label.as_str()).or_default().push(ix);
    }
    let mut claimed: HashSet<usize> = HashSet::new();
    for (number, fit) in rows.iter().enumerate() {
        let Some(candidates) = by_label.get(fit.label.as_str()) else {
            continue;
        };
        let found = candidates
            .iter()
            .copied()
            .find(|ix| !claimed.contains(ix) && to_ui(&items[*ix]) == *fit);
        let Some(found) = found else {
            continue;
        };
        claimed.insert(found);
        if number == at {
            return items.get(found);
        }
    }
    None
}

/// The server item a **clicked** row means: the first item whose row is that row.
///
/// A click hands back the row itself rather than a number, so the pairing is by value; two items equal in
/// label, kind, detail and documentation are the same row to the user, so either one answers it.
pub(crate) fn pick_clicked<'a>(
    items: &'a [JavaCompletionItem],
    row: &UiCompletion,
) -> Option<&'a JavaCompletionItem> {
    items.iter().find(|item| to_ui(item) == *row)
}

/// The LSP completion-kind number as the library's symbol kind; anything unrecognised reads as a variable
/// rather than losing the row.
fn symbol_kind(kind: Option<i64>) -> SymbolKind {
    match kind {
        Some(1 | 6 | 18) => SymbolKind::Variable,
        Some(2 | 4) => SymbolKind::Method,
        Some(3 | 24) => SymbolKind::Function,
        Some(5 | 10 | 23) => SymbolKind::Field,
        Some(7 | 22 | 25) => SymbolKind::Struct,
        Some(8) => SymbolKind::Trait,
        Some(9 | 17 | 19) => SymbolKind::Module,
        Some(11 | 12 | 16 | 21) => SymbolKind::Constant,
        Some(13 | 20) => SymbolKind::Enum,
        Some(14) => SymbolKind::Keyword,
        Some(15) => SymbolKind::Snippet,
        _ => SymbolKind::Variable,
    }
}

/// The edit that accepts an item: the replacement range in **byte offsets** of the text as it is now, and
/// the text to put there.
///
/// A server that sent a `textEdit` knows its own replacement range, so that one is used (its columns are
/// UTF-16 and are converted here). Otherwise the last identifier segment before the cursor is what the item
/// replaces — the same span the list was filtered by.
pub(crate) fn edit_for(
    item: &JavaCompletionItem,
    text: &str,
    cursor: usize,
) -> (Range<usize>, String) {
    let from_server = item.text_edit.as_ref().map(|edit| {
        offset_for_utf16(
            text,
            edit.start.line as usize,
            edit.start.utf16_column as usize,
        )..offset_for_utf16(text, edit.end.line as usize, edit.end.utf16_column as usize)
    });
    let range = from_server.unwrap_or_else(|| {
        let end = cursor.min(text.len());
        let start = end.saturating_sub(word_prefix(text, end).len());
        start..end
    });
    (range, item.insert_text.clone())
}

fn builtin_completions(
    file_path: &std::path::Path,
    text: &str,
    line: u64,
    utf16_column: usize,
) -> Vec<JavaCompletionItem> {
    let payload = json!({
        "filePath": file_path.to_string_lossy(),
        "text": text,
        "position": { "line": line, "utf16Column": utf16_column },
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

#[cfg(test)]
mod tests {
    use super::*;

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
            detail: Some("void".to_string()),
            documentation: Some("打招呼".to_string()),
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

    fn plain(label: &str, insert: &str) -> JavaCompletionItem {
        JavaCompletionItem {
            label: label.to_string(),
            insert_text: insert.to_string(),
            kind: Some(6),
            detail: None,
            documentation: None,
            sort_text: None,
            filter_text: None,
            text_edit: None,
        }
    }

    #[test]
    fn the_typed_prefix_keeps_the_dots_and_the_filter_prefix_does_not() {
        let source = "System.out.pri";
        assert_eq!(typed_prefix(source, source.len()), source);
        assert_eq!(
            word_prefix(source, source.len()),
            "pri",
            "列表按最后一段过滤，接受时也只替换这一段"
        );

        let source = "foo(ba";
        assert_eq!(word_prefix(source, source.len()), "ba");

        let source = "中文 System.ou";
        assert_eq!(typed_prefix(source, source.len()), "System.ou");
        assert_eq!(word_prefix(source, source.len()), "ou");

        assert_eq!(typed_prefix(source, 999), "System.ou");
        assert_eq!(typed_prefix(source, 1), "");
        assert_eq!(
            word_prefix("中文", 3),
            "中",
            "中日韩字符与词法器的口径一致，算标识符字符"
        );
    }

    #[test]
    fn completion_kinds_map_and_unknown_values_stay_a_row() {
        assert_eq!(symbol_kind(Some(2)), SymbolKind::Method);
        assert_eq!(symbol_kind(Some(7)), SymbolKind::Struct);
        assert_eq!(symbol_kind(Some(21)), SymbolKind::Constant);
        assert_eq!(
            symbol_kind(Some(0)),
            SymbolKind::Variable,
            "认不出的 kind 仍然要有行，不能把这一条丢了"
        );
        assert_eq!(symbol_kind(Some(99)), SymbolKind::Variable);
        assert_eq!(symbol_kind(None), SymbolKind::Variable);
    }

    #[test]
    fn a_row_keeps_the_label_the_detail_and_the_docs() {
        let row = to_ui(&sample_item(0, 0, 0, 0, "greet(name)"));
        assert_eq!(row.label, "greet(String)");
        assert_eq!(row.kind, SymbolKind::Method);
        assert_eq!(row.detail.as_deref(), Some("void"));
        assert_eq!(row.docs.as_deref(), Some("打招呼"));

        let bare = to_ui(&plain("total", "total"));
        assert!(bare.detail.is_none());
        assert!(bare.docs.is_none());
    }

    #[test]
    fn accepting_an_item_uses_the_servers_replacement_range() {
        let text = "line0\nline1\nline2\n    greet(na)\n";
        let (range, insert) = edit_for(&sample_item(3, 10, 3, 12, "greet(name)"), text, 0);
        assert_eq!(range, 28..30, "第 4 行第 10..12 个 **字节**");
        assert_eq!(&text[range], "na");
        assert_eq!(insert, "greet(name)");
    }

    #[test]
    fn utf16_columns_in_a_text_edit_become_byte_offsets() {
        let text = "x\n🙂    na\n";
        let (range, _) = edit_for(&sample_item(1, 6, 1, 8, "name"), text, 0);
        assert_eq!(
            range,
            10..12,
            "UTF-16 列 6..8 是 `na` 两个字节（把 UTF-16 当字符解会落到别的字节）"
        );
        assert_eq!(&text[range], "na");
    }

    #[test]
    fn without_a_text_edit_the_typed_prefix_is_what_gets_replaced() {
        let text = "System.out.pri";
        let cursor = text.len();
        let (range, insert) = edit_for(&plain("println", "println"), text, cursor);
        assert_eq!(range, 11..14, "只替换 `.` 之后手打的 `pri`");
        assert_eq!(&text[range], "pri");
        assert_eq!(insert, "println");

        // 光标落在非标识符字符上（例如刚打完 `(`）时替换的是空区间：插入而不是吃掉别的字。
        let text = "greet(";
        let (range, _) = edit_for(&plain("greet", "greet"), text, text.len());
        assert_eq!(range, 6..6);
    }

    /// 库画的是它自己过滤、重排过的行，所以"选中的行 → 服务端那一条"这条回路必须能对上，
    /// 包括标签重复的几条。
    #[test]
    fn a_picked_row_maps_back_to_its_server_item() {
        let items = vec![
            plain("total", "total"),
            plain("to_string", "to_string"),
            plain("total", "total()"),
        ];
        let rows = rows(&items);
        let fits: Vec<UiCompletion> = lithe_ui::editor::completions(&rows, "tot")
            .into_iter()
            .map(|(row, _)| row)
            .collect();
        assert!(!fits.is_empty(), "`tot` 至少匹配得上一条");

        // 每一行回配到的都必须正是"这一行本身"。
        for (at, row) in fits.iter().enumerate() {
            let picked = pick(&items, &fits, at).expect("每一行都有主");
            assert_eq!(to_ui(picked), *row, "第 {at} 行回配错了");
        }
        // 同标签的两条按出现顺序各归各行，不会被同一条占两次。
        let mut claimed: Vec<&str> = (0..fits.len())
            .filter_map(|at| pick(&items, &fits, at))
            .map(|item| item.insert_text.as_str())
            .collect();
        let rows = claimed.len();
        claimed.sort_unstable();
        claimed.dedup();
        assert_eq!(claimed.len(), rows, "同标签的两条必须被不同的行认领");

        assert!(
            pick(&items, &Vec::new(), 0).is_none(),
            "越界的行号给 None，而不是随手挑一条"
        );
    }
}
