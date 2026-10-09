use std::{cell::Cell, cell::RefCell, rc::Rc, time::Duration};

use gpui::{
    ClipboardItem, Context, Entity, EntityInputHandler, KeyUpEvent, Keystroke, MouseMoveEvent,
    Subscription, TestAppContext, VisualTestContext, point, px,
};

use super::{
    Buffer, CodeEditor, CompletionItem, EditorEvent, GhostText, HoverInfo, InlayHint, LineNumbers,
    SymbolKind, layout::Row, syntax,
};
use crate::theme::Theme;

fn editor<'a>(
    text: &str,
    cx: &'a mut TestAppContext,
) -> (Entity<CodeEditor>, &'a mut VisualTestContext) {
    cx.update(Theme::init);
    let text = text.to_string();
    cx.add_window_view(move |window, cx| CodeEditor::new(text, window, cx))
}

/// Watches the editor's events and collects the ones a test cares about: events are the only trigger surface an
/// owner has, so they are what a test pins down.
///
/// The subscription is handed back with the collection (`let (_watching, asked) = ...`): dropping it would leave
/// nobody listening.
fn watch<E: Clone + 'static>(
    editor: &Entity<CodeEditor>,
    cx: &mut TestAppContext,
    pick: impl Fn(&EditorEvent) -> Option<E> + 'static,
) -> (Subscription, Rc<RefCell<Vec<E>>>) {
    let seen: Rc<RefCell<Vec<E>>> = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    let subscription = cx.update(|cx| {
        cx.subscribe(editor, move |_, event: &EditorEvent, _| {
            if let Some(found) = pick(event) {
                log.borrow_mut().push(found);
            }
        })
    });
    (subscription, seen)
}

#[gpui::test]
fn typed_pairs_close_and_one_undo_takes_back_the_burst(cx: &mut TestAppContext) {
    let (editor, cx) = editor("let x = ", cx);
    editor.update(cx, |editor, cx| {
        let end = 8..8;
        editor.select([end], cx);
        for ch in ["(", "1", ")"] {
            editor.type_text(ch, cx);
        }
        assert_eq!(
            editor.text(),
            "let x = (1)",
            "the typed close steps over its pair"
        );
        editor.undo(cx);
        assert_eq!(editor.text(), "let x = ");
        editor.redo(cx);
        assert_eq!(editor.text(), "let x = (1)");
    });
}

#[gpui::test]
fn select_next_takes_the_word_then_its_next_match(cx: &mut TestAppContext) {
    let (editor, cx) = editor("a foo b foo c", cx);
    editor.update(cx, |editor, cx| {
        let inside = 3..3;
        editor.select([inside], cx);
        editor.select_next(cx);
        editor.select_next(cx);
        let ranges: Vec<_> = editor.selections().iter().map(|s| s.range()).collect();
        assert_eq!(ranges, [2..5, 8..11]);
        editor.type_text("bar", cx);
        assert_eq!(editor.text(), "a bar b bar c", "every cursor types");
    });
}

#[gpui::test]
fn toggling_comments_twice_restores_the_lines(cx: &mut TestAppContext) {
    let (editor, cx) = editor("fn a() {\n    b();\n}", cx);
    editor.update(cx, |editor, cx| {
        editor.select_all(cx);
        editor.toggle_comment(cx);
        assert_eq!(editor.text(), "// fn a() {\n//     b();\n// }");
        editor.toggle_comment(cx);
        assert_eq!(editor.text(), "fn a() {\n    b();\n}");
    });
}

#[gpui::test]
fn a_multi_line_ghost_moves_the_rest_of_its_line_below_it(cx: &mut TestAppContext) {
    let (editor, cx) = editor("call(x)\nnext", cx);
    editor.update(cx, |editor, cx| {
        editor.set_ghost_text(
            Some(GhostText {
                offset: 5,
                text: "a,\n  b, ".into(),
            }),
            cx,
        );
        assert_eq!(
            editor.row_text(0),
            "call(",
            "the row stops where the ghost starts"
        );
        assert_eq!(editor.row_text(1), "next");
        let caret = 5..5;
        editor.select([caret], cx);
        editor.indent(cx);
        assert_eq!(
            editor.text(),
            "call(a,\n  b, x)\nnext",
            "Tab takes the ghost"
        );
        assert_eq!(editor.row_text(0), "call(a,", "a taken ghost cuts nothing");
    });
}

#[gpui::test]
fn a_selection_inside_a_fold_opens_it(cx: &mut TestAppContext) {
    let (editor, cx) = editor("fn a() {\n    b();\n}\nc", cx);
    editor.update(cx, |editor, cx| {
        editor.toggle_fold(0, cx);
        assert!(!editor.frame_shows(1), "the body folds away");
        let body = 13..13;
        editor.select([body], cx);
        assert!(editor.frame_shows(1), "a cursor inside unfolds it");
    });
}

#[gpui::test]
fn hidden_numbers_draw(cx: &mut TestAppContext) {
    let (editor, cx) = editor("a\nb", cx);
    editor.update(cx, |editor, cx| {
        editor.set_line_numbers(LineNumbers::Hidden, cx)
    });
    cx.run_until_parked();
    assert_eq!(editor.read_with(cx, |editor, _| editor.gutter_columns()), 5);
}

#[gpui::test]
fn edits_with_nothing_to_change_leave_the_text(cx: &mut TestAppContext) {
    let (editor, cx) = editor("a\n\nb", cx);
    editor.update(cx, |editor, cx| {
        editor.outdent(cx);
        let blank = 2..2;
        editor.select([blank], cx);
        editor.toggle_comment(cx);
        assert_eq!(editor.text(), "a\n\nb");
    });
}

#[gpui::test]
fn a_read_only_editor_ignores_typing_and_undo(cx: &mut TestAppContext) {
    let (editor, cx) = editor("x", cx);
    editor.update(cx, |editor, cx| {
        let end = 1..1;
        editor.select([end], cx);
        editor.type_text("y", cx);
        editor.set_read_only(true, cx);
        editor.type_text("(", cx);
        assert_eq!(editor.primary().head, 2, "the caret stays");
        editor.undo(cx);
        assert_eq!(editor.text(), "xy");
    });
}

#[gpui::test]
fn enter_presses_the_banner_button_inside_the_editor(cx: &mut TestAppContext) {
    let (editor, cx) = editor("x", cx);
    cx.update(|window, cx| {
        super::keys::bind_keys(cx);
        editor.update(cx, |editor, cx| editor.set_read_only(true, cx));
        window.refresh();
    });
    cx.run_until_parked();
    let focus = editor.read_with(cx, |editor, _| editor.focus.clone());
    cx.update(|window, cx| {
        window.focus(&focus, cx);
        window.focus_next(cx);
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").expect("a key"),
    });
    cx.run_until_parked();
    assert!(!editor.read_with(cx, |editor, _| editor.is_read_only()));
}

#[gpui::test]
fn a_hint_past_a_multi_line_ghost_stays_in_its_row(cx: &mut TestAppContext) {
    let (editor, cx) = editor("call(x)\nnext", cx);
    editor.update(cx, |editor, cx| {
        editor.set_inlay_hints(
            vec![InlayHint {
                offset: 6,
                text: ": i32".into(),
            }],
            cx,
        );
        editor.set_ghost_text(
            Some(GhostText {
                offset: 5,
                text: "a,\n".into(),
            }),
            cx,
        );
        let row = editor.row_text(0).len();
        assert!(editor.notes(0, cx).iter().all(|(at, ..)| *at <= row));
    });
    cx.run_until_parked();
}

#[gpui::test]
fn cursors_added_above_keep_climbing(cx: &mut TestAppContext) {
    let (editor, cx) = editor("a\nb\nc\nd", cx);
    editor.update(cx, |editor, cx| {
        let last = 6..6;
        editor.select([last], cx);
        editor.add_cursor(false, cx);
        editor.add_cursor(false, cx);
        assert_eq!(editor.selections().len(), 3);
        assert_eq!(editor.position().0, 2, "the newest cursor leads");
    });
}

#[gpui::test]
fn a_slow_composition_undoes_in_one_step(cx: &mut TestAppContext) {
    let (editor, cx) = editor("", cx);
    for pinyin in ["n", "ni"] {
        editor.update_in(cx, |editor, window, cx| {
            editor.replace_and_mark_text_in_range(None, pinyin, None, window, cx)
        });
        std::thread::sleep(Duration::from_millis(950));
    }
    editor.update_in(cx, |editor, window, cx| {
        editor.replace_text_in_range(None, "你", window, cx);
        assert_eq!(editor.text(), "你");
        editor.undo(cx);
        assert_eq!(editor.text(), "", "the pinyin was never a step");
    });
}

#[gpui::test]
fn a_composition_cleared_to_nothing_ends(cx: &mut TestAppContext) {
    let (editor, cx) = editor("", cx);
    editor.update_in(cx, |editor, window, cx| {
        editor.replace_and_mark_text_in_range(None, "n", None, window, cx);
        editor.replace_and_mark_text_in_range(None, "", None, window, cx);
        editor.type_text("x", cx);
        editor.undo(cx);
        assert_eq!(editor.text(), "", "typing after the cleared mark undoes");
    });
}

impl CodeEditor {
    /// Whether line `line` has a row now.
    fn frame_shows(&self, line: usize) -> bool {
        let hide = super::layout::hidden(
            self.buffer.lines(),
            &super::syntax::folds(&self.buffer),
            &self.folded,
        );
        super::layout::rows(&self.buffer, &hide, &self.marks).contains(&Row::Line(line))
    }
}

#[gpui::test]
fn a_browser_paste_gives_each_cursor_its_line(cx: &mut TestAppContext) {
    let (editor, cx) = editor("a\nb", cx);
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| {
            let (first, second) = (1..1, 3..3);
            editor.select([first, second], cx);
            EntityInputHandler::paste(editor, ClipboardItem::new_string("1\n2".into()), window, cx);
            assert_eq!(editor.text(), "a1\nb2");
        })
    });
}

/// 没有语言时的回归基线：同一段文本，逐行取色的老入口与缓冲区感知的新入口必须给出同一结果。
///
/// `set_language(None)` 就是编辑器接上语言注册表之前的全部行为，所以这条断言等价于"接线没有
/// 改变没有语言时的观感"。
#[gpui::test]
fn a_buffer_without_a_language_keeps_the_lexer_colors(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let buffer = Buffer::new("fn a() {\n    let s = \"x\"; // note\n}");
    assert!(buffer.language().is_none());
    assert!(
        !buffer.has_highlighter(),
        "没有语言就没有 tree-sitter 高亮器"
    );
    for line in 0..buffer.lines() {
        let range = buffer.line_range(line);
        let expected = cx.update(|cx| syntax::colors(buffer.line(line), cx));
        let found = cx.update(|cx| syntax::colors_in(&buffer, range.clone(), cx));
        assert_eq!(found, expected, "第 {line} 行必须与逐行词法器一致");
        assert!(
            found
                .iter()
                .all(|(span, _)| span.start <= span.end && span.end <= range.end - range.start),
            "第 {line} 行的 span 必须落在该行之内：{found:?}"
        );
    }
    // 片段（而不是整行）也一样：取色入口按调用方给的绝对区间切片，答案属于那一段文本。
    let tail = buffer.line_range(1).start + 4;
    let whole = buffer.len();
    let found = cx.update(|cx| syntax::colors_in(&buffer, tail..whole, cx));
    let expected = cx.update(|cx| syntax::colors(&buffer.text()[tail..whole], cx));
    assert_eq!(found, expected);
}

/// Automatic completion after an identifier character: switched on, one letter typed raises the event, while
/// punctuation and several characters at once (a paste) do not.
#[gpui::test]
fn auto_completion_asks_after_one_identifier_character(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (editor, cx) =
        cx.add_window_view(|window, cx| CodeEditor::new("let ", window, cx).auto_completion(true));
    let (_watching, asked) = watch(&editor, cx, |event| match event {
        EditorEvent::CompletionRequested(at) => Some(*at),
        _ => None,
    });
    editor.update(cx, |editor, cx| {
        editor.select([4..4], cx);
        editor.type_text("x", cx);
    });
    assert_eq!(
        *asked.borrow(),
        vec![5],
        "the request names the primary cursor after x was typed"
    );
    editor.update(cx, |editor, cx| {
        editor.type_text("(", cx);
        editor.type_text("name", cx);
    });
    assert_eq!(
        asked.borrow().len(),
        1,
        "an opening bracket and several characters at once are not typing one by one"
    );
    // The real input path (a keystroke arrives at `replace_text_in_range`) asks just the same.
    editor.update_in(cx, |editor, window, cx| {
        editor.replace_text_in_range(None, "y", window, cx)
    });
    assert_eq!(asked.borrow().len(), 2, "the next letter asks again");
    assert_eq!(
        editor.read_with(cx, |editor, _| editor.text().to_string()),
        "let x(namey)",
        "the input path types inside the paired bracket"
    );
}

/// Automatic completion is **off** by default: without switching it on, typing must not disturb the owner.
#[gpui::test]
fn auto_completion_stays_off_by_default(cx: &mut TestAppContext) {
    let (editor, cx) = editor("let ", cx);
    let (_watching, asked) = watch(&editor, cx, |event| match event {
        EditorEvent::CompletionRequested(at) => Some(*at),
        _ => None,
    });
    editor.update(cx, |editor, cx| {
        editor.select([4..4], cx);
        editor.type_text("x", cx);
        assert!(
            asked.borrow().is_empty(),
            "off by default, typing asks for nothing"
        );
    });
}

/// The explicit trigger: `ctrl-space` goes through the key bindings and reports the primary cursor's byte
/// offset; fetching the data and showing the popup are the owner's.
#[gpui::test]
fn ctrl_space_asks_the_owner_for_completions(cx: &mut TestAppContext) {
    let (editor, cx) = editor("let x", cx);
    let (_watching, asked) = watch(&editor, cx, |event| match event {
        EditorEvent::CompletionRequested(at) => Some(*at),
        _ => None,
    });
    cx.update(|window, cx| {
        super::keys::bind_keys(cx);
        window.refresh();
    });
    editor.update(cx, |editor, cx| editor.select([5..5], cx));
    let focus = editor.read_with(cx, |editor, _| editor.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("ctrl-space");
    cx.run_until_parked();
    assert_eq!(*asked.borrow(), vec![5], "ctrl-space reports the cursor");
}

/// An owner placing its cursor: an offset inside a multi-byte character or out of bounds is pulled back onto a
/// whole character, many cursors collapse into one, and nothing panics.
#[gpui::test]
fn a_cursor_offset_is_pulled_onto_a_character(cx: &mut TestAppContext) {
    let (editor, cx) = editor("aπb\ncdef", cx);
    editor.update(cx, |editor, cx| {
        editor.select([0..0, 6..6], cx);
        assert_eq!(editor.selections().len(), 2);
        assert_eq!(editor.cursor_offset(), 6, "the last cursor is primary");
    });
    editor.update_in(cx, |editor, window, cx| {
        editor.set_cursor_offset(2, window, cx);
        assert_eq!(
            editor.cursor_offset(),
            1,
            "π sits at 1..3, so an offset inside it falls back to its start"
        );
        assert_eq!(editor.selections().len(), 1, "many cursors collapse");
        assert!(editor.primary().is_empty(), "the selection becomes a caret");
        assert_eq!(editor.position(), (1, 2), "line and column follow");
        editor.set_cursor_offset(3, window, cx);
        assert_eq!(editor.cursor_offset(), 3, "a boundary after π stays");
        editor.set_cursor_offset(usize::MAX, window, cx);
        assert_eq!(
            editor.cursor_offset(),
            editor.text().len(),
            "out of bounds lands at the end"
        );
        assert_eq!(
            editor.text(),
            "aπb\ncdef",
            "placing a cursor changes no text"
        );
    });
    cx.run_until_parked();
}

/// Placing a cursor brings its line into view, and the anchor lookup answers only for places in the viewport —
/// the old position, scrolled away, gives `None`.
#[gpui::test]
fn a_cursor_jump_scrolls_its_line_into_view(cx: &mut TestAppContext) {
    let mut text = String::new();
    for line in 0..200 {
        text.push_str(&format!("line {line}; // some code\n"));
    }
    let (editor, cx) = editor(&text, cx);
    cx.run_until_parked();
    let last = editor.read_with(cx, |editor, _| editor.text().len());
    editor.update_in(cx, |editor, window, cx| {
        editor.set_cursor_offset(last, window, cx)
    });
    cx.run_until_parked();
    editor.read_with(cx, |editor, cx| {
        let visible = editor.visible_row_range().expect("a frame was drawn");
        let row = editor.primary_row();
        assert!(
            visible.contains(&row),
            "placing the cursor brings it into view: {visible:?} misses row {row}"
        );
        assert!(
            editor.scroll_offset().y < px(0.),
            "so the view really scrolled: {:?}",
            editor.scroll_offset()
        );
        assert!(
            editor.bounds_for_offset(0, cx).is_none(),
            "an anchor scrolled out of view answers nothing"
        );
        assert!(
            editor.bounds_for_offset(last, cx).is_some(),
            "the anchor at the cursor is in view"
        );
        assert!(editor.line_height().is_some(), "a line height was measured");
    });
}

/// Anchors: what comes out is a character's rectangle in the window — a multi-byte character takes one column;
/// a line in a fold, or one not in the row table yet, gives `None`.
#[gpui::test]
fn bounds_for_offset_measures_characters_not_bytes(cx: &mut TestAppContext) {
    let (editor, cx) = editor("aπb\ncdef", cx);
    cx.run_until_parked();
    editor.read_with(cx, |editor, cx| {
        let first = editor
            .bounds_for_offset(0, cx)
            .expect("the first character is laid out");
        let pi = editor.bounds_for_offset(1, cx).expect("the start of π");
        let after = editor
            .bounds_for_offset(3, cx)
            .expect("the character after π");
        // Positions accumulate column by column in f32, so a column's width needs a tolerance rather than bit equality.
        let step = f32::from(pi.origin.x - first.origin.x);
        let wide = f32::from(first.size.width);
        assert!(
            (step - wide).abs() < 0.01,
            "π takes one column: {step} against {wide}"
        );
        let next = f32::from(after.origin.x - pi.origin.x);
        assert!((next - wide).abs() < 0.01, "and one more after π: {next}");
        assert_eq!(after.origin.y, first.origin.y, "the same row");
        assert_eq!(
            first.size.height,
            editor.line_height().expect("a line height")
        );
        let inside = point(
            first.origin.x + first.size.width / 2.,
            first.size.height / 2.,
        );
        assert_eq!(
            editor.offset_at(inside, cx),
            Some(0),
            "the offset the mouse points back at"
        );
    });
}

/// A fold hides its lines, so there is no anchor inside it: an owner shows nothing instead of pinning a popup
/// to where the body used to be.
#[gpui::test]
fn bounds_for_offset_stays_silent_inside_a_fold(cx: &mut TestAppContext) {
    let (editor, cx) = editor("fn a() {\n    b();\n}\nc", cx);
    cx.run_until_parked();
    let body = editor.read_with(cx, |editor, _| editor.text().find("b();").expect("b();"));
    assert!(
        editor
            .read_with(cx, |editor, cx| editor.bounds_for_offset(body, cx))
            .is_some()
    );
    editor.update(cx, |editor, cx| editor.toggle_fold(0, cx));
    cx.run_until_parked();
    editor.read_with(cx, |editor, cx| {
        assert!(
            editor.bounds_for_offset(body, cx).is_none(),
            "an anchor inside the fold answers nothing"
        );
        assert!(
            editor.bounds_for_offset(0, cx).is_some(),
            "the fold's own header is still there"
        );
    });
}

/// The row table mixes document lines with folded bodies, ghost rows and the like, so a row index is not a line
/// number: each index is asked for its own line, and a row that is not a whole line answers `None`.
#[gpui::test]
fn line_at_row_names_the_line_a_row_stands_for(cx: &mut TestAppContext) {
    let (editor, cx) = editor("fn a() {\n    b();\n    c();\n}\nd", cx);
    cx.run_until_parked();
    // Every row of the table, then one past it: the whole mapping is read rather than one index at a time.
    let names = |editor: &CodeEditor| {
        (0..6)
            .map(|row| editor.line_at_row(row))
            .collect::<Vec<_>>()
    };
    editor.read_with(cx, |editor, _| {
        assert_eq!(
            editor.visible_row_range(),
            Some(0..5),
            "five lines, five rows"
        );
        assert_eq!(
            names(editor),
            [Some(1), Some(2), Some(3), Some(4), Some(5), None],
            "each index names its own 1-based line, and past the table there is none"
        );
    });
    // A folded body has no rows of its own, so the indices it used to hold name later lines instead.
    editor.update(cx, |editor, cx| editor.toggle_fold(0, cx));
    cx.run_until_parked();
    editor.read_with(cx, |editor, _| {
        assert_eq!(
            editor.visible_row_range(),
            Some(0..3),
            "the fold takes two rows away"
        );
        assert_eq!(
            names(editor),
            [Some(1), Some(4), Some(5), None, None, None],
            "row 1 named line 2 before the fold, so row + 1 would name the folded body"
        );
    });
    // Ghost text is drawn on rows of its own, and such a row is not a document line.
    editor.update(cx, |editor, cx| {
        editor.set_ghost_text(
            Some(GhostText {
                offset: 0,
                text: "one\ntwo".into(),
            }),
            cx,
        )
    });
    cx.run_until_parked();
    editor.read_with(cx, |editor, _| {
        assert_eq!(
            names(editor),
            [Some(1), None, Some(4), Some(5), None, None],
            "the ghost's own second row is no line, and the lines after it keep their numbers"
        );
    });
}

/// A batch of language-server edits: applied bottom-up, one undo step, and the caret left where the edits put
/// it; `Changed` only when the text really moved.
#[gpui::test]
fn lsp_edits_land_in_one_undo_step_and_place_the_caret(cx: &mut TestAppContext) {
    let (editor, cx) = editor("foo(1)\nbar(2)\n", cx);
    let changes = Rc::new(Cell::new(0usize));
    let counted = changes.clone();
    let (_watching, _seen) = watch(&editor, cx, move |event| {
        if *event == EditorEvent::Changed {
            counted.set(counted.get() + 1);
        }
        None::<()>
    });
    editor.update(cx, |editor, cx| {
        editor.select([8..8], cx);
        editor.apply_edits(
            &[(0..3, "print".to_string()), (7..10, "baz".to_string())],
            cx,
        );
        assert_eq!(editor.text(), "print(1)\nbaz(2)\n", "both edits landed");
        assert_eq!(
            editor.cursor_offset(),
            12,
            "the caret stops at the end of the replacement of the edit it was inside"
        );
    });
    // Events are dispatched as the entity's update finishes, so the assertion waits for it.
    assert_eq!(changes.get(), 1, "one batch raises one Changed");
    editor.update(cx, |editor, cx| editor.undo(cx));
    assert_eq!(
        editor.read_with(cx, |editor, _| editor.text().to_string()),
        "foo(1)\nbar(2)\n",
        "one batch is one undo step"
    );
    assert_eq!(changes.get(), 2, "undoing raises Changed too");
    cx.run_until_parked();
}

/// Out of bounds, splitting a multi-byte character, or overlapping: every language-server range lands safely
/// and nothing panics.
#[gpui::test]
fn clipped_lsp_edits_land_without_panicking(cx: &mut TestAppContext) {
    let (editor, cx) = editor("aπb", cx);
    editor.update(cx, |editor, cx| {
        editor.apply_edits(&[(1..2, "-".to_string())], cx);
        assert_eq!(
            editor.text(),
            "a-πb",
            "a range that splits π is pulled onto whole characters"
        );
        editor.apply_edits(&[(usize::MAX - 1..usize::MAX, "!".to_string())], cx);
        assert_eq!(
            editor.text(),
            "a-πb!",
            "an out-of-bounds range lands at the end of the text"
        );
        editor.apply_edits(&[(0..4, "xy".to_string())], cx);
        assert_eq!(editor.text(), "xyb!", "a whole-range replacement as usual");
        editor.apply_edits(&[(0..2, "X".to_string()), (1..3, "Y".to_string())], cx);
        assert_eq!(
            editor.text(),
            "X!",
            "overlapping edits are applied bottom-up, so the later one (the earlier range) wins"
        );
        editor.apply_edits(&[], cx);
        assert_eq!(editor.text(), "X!", "an empty batch does nothing");
    });
    cx.run_until_parked();
}

/// Read-only text refuses the edits an owner pushes in, exactly as it refuses typing.
#[gpui::test]
fn a_read_only_editor_refuses_lsp_edits(cx: &mut TestAppContext) {
    let (editor, cx) = editor("let x", cx);
    editor.update(cx, |editor, cx| {
        editor.set_read_only(true, cx);
        editor.apply_edits(&[(0..3, "var".to_string())], cx);
        assert_eq!(editor.text(), "let x", "read-only text does not move");
    });
}

/// Hover requests: the pointer resting on an identifier asks once, the same symbol is not asked again,
/// punctuation asks for nothing, and coming back to a symbol asks again.
#[gpui::test]
fn a_pointer_over_a_symbol_asks_for_its_hover_once(cx: &mut TestAppContext) {
    let (editor, cx) = editor("let total = 2;", cx);
    let (_watching, asked) = watch(&editor, cx, |event| match event {
        EditorEvent::HoverRequested(at) => Some(*at),
        _ => None,
    });
    cx.run_until_parked();
    // The pointer moves column by column (its width comes from the editor's own metrics, no pixels are written
    // down); events are dispatched as the entity's update finishes.
    let ask = |editor: &mut CodeEditor, cx: &mut Context<CodeEditor>, column: f32| {
        let (left, advance, line) = (
            editor.metrics.left,
            editor.metrics.advance,
            editor.metrics.line,
        );
        let moved = MouseMoveEvent {
            position: point(left + advance * column, line / 2.),
            ..MouseMoveEvent::default()
        };
        editor.track_hover(0, &moved, cx);
    };
    editor.update(cx, |editor, cx| ask(editor, cx, 5.));
    assert_eq!(*asked.borrow(), vec![4], "total starts at 4");
    editor.update(cx, |editor, cx| ask(editor, cx, 6.));
    assert_eq!(
        asked.borrow().len(),
        1,
        "the same symbol is not asked again"
    );
    editor.update(cx, |editor, cx| ask(editor, cx, 10.));
    assert_eq!(asked.borrow().len(), 1, "punctuation asks for nothing");
    editor.update(cx, |editor, cx| ask(editor, cx, 5.));
    assert_eq!(
        asked.borrow().len(),
        2,
        "leaving and coming back asks again"
    );
}

/// What the owner pushes back: completions are only kept (the library reads them out), hover content is drawn
/// by the library — and either can be taken down at any time.
#[gpui::test]
fn pushed_completions_and_hover_are_kept_for_the_host(cx: &mut TestAppContext) {
    let (editor, cx) = editor("let x", cx);
    cx.run_until_parked();
    editor.update(cx, |editor, cx| {
        assert!(editor.completions().is_empty());
        assert!(editor.hover().is_none());
        editor.set_completions(
            vec![
                CompletionItem::new("total", SymbolKind::Field).detail("i32"),
                CompletionItem::new("to_string", SymbolKind::Method),
            ],
            cx,
        );
        assert_eq!(editor.completions().len(), 2, "the library kept the batch");
        assert_eq!(editor.completions()[0].label, "total");
        editor.select([5..5], cx);
        editor.present_hover(
            Some(HoverInfo::new("hover", "let x: i32").docs("the x")),
            cx,
        );
        assert!(editor.hover().is_some(), "the hover content was kept");
        assert!(
            editor.hover_panel(cx).is_some(),
            "an anchor at the cursor draws it"
        );
    });
    // Let the hover panel really enter a frame: layout and paint must not blow up.
    cx.run_until_parked();
    editor.update(cx, |editor, cx| {
        editor.clear_completions(cx);
        editor.present_hover(None, cx);
        assert!(
            editor.completions().is_empty(),
            "the owner takes a layer down and it is cleared"
        );
        assert!(editor.hover().is_none());
        assert!(
            editor.hover_panel(cx).is_none(),
            "no content, nothing drawn"
        );
    });
    cx.run_until_parked();
}

#[gpui::test]
fn only_a_focused_editor_blinks(cx: &mut TestAppContext) {
    let (editor, cx) = editor("a\nb", cx);
    let redraws = Rc::new(Cell::new(0));
    let counted = redraws.clone();
    let _count = cx.update(|_, cx| cx.observe(&editor, move |_, _| counted.set(counted.get() + 1)));
    editor.update(cx, |editor, cx| {
        let second = 2..2;
        editor.select([second], cx);
    });
    cx.run_until_parked();
    redraws.set(0);
    cx.executor().advance_clock(Duration::from_secs(3));
    cx.run_until_parked();
    assert_eq!(redraws.get(), 0, "an unfocused editor keeps still");
    cx.update(|window, cx| {
        window.activate_window();
        let focus = editor.read(cx).focus.clone();
        focus.focus(window, cx);
    });
    cx.run_until_parked();
    redraws.set(0);
    cx.executor().advance_clock(Duration::from_secs(3));
    cx.run_until_parked();
    assert!(redraws.get() > 2, "a focused one blinks");
}
