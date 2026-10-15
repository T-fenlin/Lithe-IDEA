//! 代码动作（含 Quick Fix）：向 JDTLS 要"光标/选区这里能做什么"，把选中的那条落到文本上。
//!
//! 与补全、悬停同一条分工：**取数**在这里、**改文本**走组件库的 `CodeEditor::apply_edits`
//! （整批一次撤销步、只读拒绝），**弹层**用组件库的菜单（`ContextMenu::manual` 就是"由宿主在
//! 某个窗口坐标上开一份菜单"的接缝：键盘、选中、点击外部关闭、关掉后交还焦点都归它）。
//!
//! 入口是宿主自己的键（`secondary-.` = macOS ⌘. / 其他 Ctrl+.，与上游 gpui-base 的
//! `ToggleCodeActions` 同一个键）：组件库没有"请求代码动作"的事件——它不认识语言服务，
//! 所以这次取数的时机由宿主决定，不是库报上来的。

use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use gpui::Window;
use lithe_gpui_java::{JavaCodeAction, JavaLanguageService, JavaTextEdit};
use lithe_ui::editor::ActionKind;
use lithe_ui::menus::{Menu, MenuItem};

use crate::navigation::{core_position, offset_for_utf16};

/// A picked row: its place in the actions the menu was built from.
pub(crate) type OnPick = Rc<dyn Fn(usize, &mut Window, &mut gpui::App)>;

/// 取一次代码动作；返回 `None` 表示这次请求已被更晚的一次顶掉。
///
/// `range` 是编辑器口径的**字节**区间（选区，或一个空区间代表光标处）；给服务端时换算成 UTF-16 列。
pub(crate) async fn fetch(
    service: Arc<JavaLanguageService>,
    file_path: PathBuf,
    text: String,
    range: Range<usize>,
    generation: Arc<AtomicU64>,
    mine: u64,
) -> Option<Vec<JavaCodeAction>> {
    let started = Instant::now();
    let (start_line, start_column) = core_position(&text, range.start);
    let (end_line, end_column) = core_position(&text, range.end);
    let start = lithe_gpui_java::JavaPosition {
        line: start_line as u32,
        utf16_column: start_column as u32,
    };
    let end = lithe_gpui_java::JavaPosition {
        line: end_line as u32,
        utf16_column: end_column as u32,
    };
    let file = file_path.display().to_string();

    let actions = match service.code_actions(&file_path, &text, start, end) {
        Ok(actions) => actions,
        Err(error) => {
            println!("S1_EDITOR_CODE_ACTION file={file} actions=0 result=failed error={error}");
            return Some(Vec::new());
        }
    };

    if generation.load(Ordering::SeqCst) != mine {
        println!(
            "S1_EDITOR_CODE_ACTION file={file} actions={} result=stale reason=superseded",
            actions.len()
        );
        return None;
    }

    println!(
        "S1_EDITOR_CODE_ACTION file={file} actions={} kind={} line={} col={} ms={}",
        actions.len(),
        kind_summary(&actions),
        start.line,
        start.utf16_column,
        started.elapsed().as_millis()
    );
    Some(actions)
}

/// The menu of a batch of actions: one group per kind, each row the server's own title.
///
/// The three group titles are the component library's own (`CodeActionsMenu` groups them the same way), so
/// the words are the library's and no product copy is invented here.
pub(crate) fn menu(actions: &[JavaCodeAction], pick: OnPick) -> Menu {
    let groups = [
        (ActionKind::QuickFix, "Quick fixes"),
        (ActionKind::Refactor, "Refactor"),
        (ActionKind::Source, "Source"),
    ];
    let mut menu = Menu::new();
    for (kind, title) in groups {
        let items = actions
            .iter()
            .enumerate()
            .filter(|(_, action)| group_of(action.kind.as_deref()) == kind)
            .map(|(ix, action)| {
                let pick = pick.clone();
                MenuItem::new(action.title.clone()).on_click(move |window, cx| {
                    pick(ix, window, cx);
                })
            })
            .collect::<Vec<MenuItem>>();
        if !items.is_empty() {
            menu = menu.group(title, items);
        }
    }
    menu
}

/// Which group an action's LSP kind belongs to; a kind the server did not give (or one this side does not
/// know) is offered as a quick fix, which is the only group that is always shown.
fn group_of(kind: Option<&str>) -> ActionKind {
    match kind {
        Some(kind) if kind.starts_with("refactor") => ActionKind::Refactor,
        Some(kind) if kind.starts_with("source") => ActionKind::Source,
        _ => ActionKind::QuickFix,
    }
}

/// One action's edits in the editor's own byte offsets, ready for `CodeEditor::apply_edits` (which sorts and
/// clamps them itself, so the order here is the server's).
///
/// Edits that belong to another file are **not** applied — this editor holds one file, and its owner
/// (`lithe-gpui-java` through Core) reports how many of those there are so the log can say so.
pub(crate) fn edits_for(action: &JavaCodeAction, text: &str) -> Vec<(Range<usize>, String)> {
    ranges_for(&action.edits, text)
}

/// 一组服务端文本编辑 → 编辑器字节区间，ready for `CodeEditor::apply_edits`（它自己会排序与夹取，
/// 所以这里保留服务端给的顺序）。
///
/// 代码动作、重命名、格式化三种结果都是这个形状，所以换算只有这一份：区间反了（服务端把 start/end
/// 发反）读成空的一段，而不是让 `apply_edits` 去切一个倒过来的区间。
pub(crate) fn ranges_for(edits: &[JavaTextEdit], text: &str) -> Vec<(Range<usize>, String)> {
    edits
        .iter()
        .map(|edit| {
            let start = offset_for_utf16(
                text,
                edit.start.line as usize,
                edit.start.utf16_column as usize,
            );
            let end =
                offset_for_utf16(text, edit.end.line as usize, edit.end.utf16_column as usize);
            (start..end.max(start), edit.new_text.clone())
        })
        .collect()
}

/// The distinct kinds in a batch, for the diagnostic line (at most four, so one line stays readable).
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
                start: lithe_gpui_java::JavaPosition {
                    line: 0,
                    utf16_column: 0,
                },
                end: lithe_gpui_java::JavaPosition {
                    line: 2,
                    utf16_column: 0,
                },
                new_text: "package demo;\n\nimport java.util.List;\n\n".to_string(),
            }],
            other_file_edits: 0,
        }
    }

    #[test]
    fn edits_are_converted_to_byte_offsets() {
        let text = "x\n🙂    List<String> y;\n";
        let action = JavaCodeAction {
            edits: vec![
                lithe_gpui_java::JavaTextEdit {
                    start: lithe_gpui_java::JavaPosition {
                        line: 0,
                        utf16_column: 0,
                    },
                    end: lithe_gpui_java::JavaPosition {
                        line: 0,
                        utf16_column: 0,
                    },
                    new_text: "import java.util.List;\n".to_string(),
                },
                lithe_gpui_java::JavaTextEdit {
                    start: lithe_gpui_java::JavaPosition {
                        line: 1,
                        utf16_column: 6,
                    },
                    end: lithe_gpui_java::JavaPosition {
                        line: 1,
                        utf16_column: 10,
                    },
                    new_text: "java.util.List".to_string(),
                },
            ],
            ..sample_action()
        };

        let converted = edits_for(&action, text);
        assert_eq!(converted.len(), 2);
        assert_eq!(
            converted[1].0,
            10..14,
            "UTF-16 列 6..10 落在第 2 行的 `List` 上（把 UTF-16 当字符解会切到别的字节）"
        );
        assert_eq!(&text[converted[1].0.clone()], "List");
        assert_eq!(converted[1].1, "java.util.List");
        assert_eq!(converted[0].0, 0..0);
    }

    #[test]
    fn ascii_edits_are_unchanged() {
        let text = "package demo;\n\npublic class A {}\n";
        let action = JavaCodeAction {
            edits: vec![lithe_gpui_java::JavaTextEdit {
                start: lithe_gpui_java::JavaPosition {
                    line: 0,
                    utf16_column: 0,
                },
                end: lithe_gpui_java::JavaPosition {
                    line: 2,
                    utf16_column: 0,
                },
                new_text: "package demo;\n\nimport java.util.List;\n\n".to_string(),
            }],
            ..sample_action()
        };
        let converted = edits_for(&action, text);
        assert_eq!(converted[0].0, 0..15, "第 2 行从第 15 个字节开始");
        assert_eq!(&text[converted[0].0.clone()], "package demo;\n\n");
    }

    /// 服务端把区间发反了：读成空的一段，而不是让 `apply_edits` 去切一个倒过来的区间。
    #[test]
    fn a_backwards_edit_reads_as_empty() {
        let text = "int x = 1;\n";
        let action = JavaCodeAction {
            edits: vec![lithe_gpui_java::JavaTextEdit {
                start: lithe_gpui_java::JavaPosition {
                    line: 0,
                    utf16_column: 9,
                },
                end: lithe_gpui_java::JavaPosition {
                    line: 0,
                    utf16_column: 4,
                },
                new_text: "y".to_string(),
            }],
            ..sample_action()
        };
        assert_eq!(edits_for(&action, text)[0].0, 9..9);
    }

    #[test]
    fn kinds_land_in_the_librarys_groups() {
        assert_eq!(group_of(Some("quickfix")), ActionKind::QuickFix);
        assert_eq!(
            group_of(Some("refactor.extract")),
            ActionKind::Refactor,
            "细化过的 kind 归它的大类"
        );
        assert_eq!(group_of(Some("source.organizeImports")), ActionKind::Source);
        assert_eq!(
            group_of(None),
            ActionKind::QuickFix,
            "没有 kind 的动作也要有地方出现"
        );
        assert_eq!(group_of(Some("something.else")), ActionKind::QuickFix);
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
        assert_eq!(
            kind_summary(&[JavaCodeAction {
                kind: None,
                ..sample_action()
            }]),
            "none"
        );
    }

    #[test]
    fn a_menu_carries_every_action_the_server_sent() {
        let actions = vec![
            sample_action(),
            JavaCodeAction {
                title: "Extract variable".to_string(),
                kind: Some("refactor.extract".to_string()),
                ..sample_action()
            },
        ];
        // 菜单本身只有库能读，这里守住"每一组都建得出来、行数对得上"这一步：
        // 两个动作分别落在 quickfix 与 refactor 两组里。
        let grouped = actions
            .iter()
            .map(|action| group_of(action.kind.as_deref()))
            .collect::<Vec<ActionKind>>();
        assert_eq!(grouped, vec![ActionKind::QuickFix, ActionKind::Refactor]);
        let _ = menu(&actions, Rc::new(|_, _, _| {}));
    }
}
