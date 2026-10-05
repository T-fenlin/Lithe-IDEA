use std::{collections::BTreeSet, ops::Range, time::Duration};

use gpui::{
    App, AppContext as _, Context, EventEmitter, FocusHandle, Focusable, Pixels, ScrollStrategy,
    SharedString, Subscription, Task, UniformListScrollHandle, Window,
};

use super::{
    buffer::Buffer,
    cursor::{Selection, merged},
    decor::{CodeLens, Diagnostic, DiffHunk, GhostText, GitMark, InlayHint},
    view::Frame,
};
use crate::{
    forms::{History, InputEvent, TextInput},
    theme::ActiveTheme,
};

/// How the gutter numbers lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineNumbers {
    #[default]
    Absolute,
    /// Distances from the cursor's line, which shows its own number.
    Relative,
    Hidden,
}

/// How the cursor draws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorShape {
    #[default]
    Line,
    Block,
    Underline,
}

/// What the editor tells its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorEvent {
    Changed,
    /// A press on a code lens: its line and the item's place.
    Lens(usize, usize),
    /// The inline chat's prompt, sent.
    Asked(SharedString),
    ChatClosed,
    /// The read-only banner's action.
    Unlock,
}

/// The editor's look and manners.
#[derive(Clone, Debug)]
pub(crate) struct Options {
    pub language: SharedString,
    pub numbers: LineNumbers,
    pub cursor: CursorShape,
    pub whitespace: bool,
    pub guides: bool,
    pub rulers: Vec<usize>,
    pub rainbow: bool,
    pub minimap: bool,
    pub sticky: bool,
    pub read_only: bool,
    pub tab: usize,
}

/// What the owner lays over the code.
#[derive(Clone, Debug, Default)]
pub(crate) struct Marks {
    pub diagnostics: Vec<Diagnostic>,
    pub backgrounds: Vec<(Range<usize>, gpui::Hsla)>,
    pub hints: Vec<InlayHint>,
    pub lenses: Vec<CodeLens>,
    pub ghost: Option<GhostText>,
    pub hunks: Vec<DiffHunk>,
    pub git: Vec<(usize, GitMark)>,
    pub breakpoints: BTreeSet<usize>,
    pub chat: Option<usize>,
}

/// The text and cursors one undo step returns to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Snapshot {
    pub text: String,
    pub selections: Vec<Selection>,
}

/// Where the code sat when last painted, in window pixels: its left edge, the top of the first row, a character's width and a row's height.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Metrics {
    pub left: Pixels,
    pub top: Pixels,
    pub advance: Pixels,
    pub line: Pixels,
}

const BLINK: Duration = Duration::from_millis(530);

/// Code to read and edit: syntax colors, many cursors, folds, a gutter, a minimap, and what tools lay over it.
pub struct CodeEditor {
    pub(crate) buffer: Buffer,
    pub(crate) selections: Vec<Selection>,
    pub(crate) history: History<Snapshot>,
    pub(crate) focus: FocusHandle,
    pub(crate) scroll: UniformListScrollHandle,
    pub(crate) folded: BTreeSet<usize>,
    pub(crate) options: Options,
    pub(crate) marks: Marks,
    pub(crate) marked: Option<Range<usize>>,
    /// The text before an input method's composition began, while it runs.
    pub(crate) composing: Option<Snapshot>,
    pub(crate) metrics: Metrics,
    pub(crate) dragging: bool,
    pub(crate) caret_on: bool,
    /// Blink runs only while focused, or unseen cursors redraw forever.
    focused: bool,
    pub(crate) chat_field: gpui::Entity<TextInput>,
    pub(crate) frame: Frame,
    blink_epoch: u64,
    _blink: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<EditorEvent> for CodeEditor {}

impl Focusable for CodeEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl CodeEditor {
    pub fn new(text: impl Into<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        let chat_field =
            cx.new(|cx| TextInput::new(window, cx).placeholder("Ask to change this code"));
        let subscriptions = vec![
            cx.on_focus(&focus, window, |editor, _, cx| {
                editor.focused = true;
                editor.restart_blink(cx);
            }),
            cx.on_blur(&focus, window, |editor, _, cx| {
                editor.focused = false;
                editor.dragging = false;
                editor._blink = None;
                cx.notify();
            }),
            cx.subscribe(&chat_field, |_, field, event, cx| {
                if *event != InputEvent::Submit {
                    return;
                }
                let prompt = field.read(cx).text().trim().to_string();
                if prompt.is_empty() {
                    return;
                }
                log::info!("code editor: asked {prompt:?}");
                field.update(cx, |field, cx| field.set_text("", cx));
                cx.emit(EditorEvent::Asked(prompt.into()));
            }),
        ];
        Self {
            buffer: Buffer::new(text),
            selections: vec![Selection::caret(0)],
            history: History::default(),
            focus,
            scroll: UniformListScrollHandle::new(),
            folded: BTreeSet::new(),
            options: Options {
                language: "Plain text".into(),
                numbers: LineNumbers::Absolute,
                cursor: CursorShape::Line,
                whitespace: false,
                guides: true,
                rulers: Vec::new(),
                rainbow: false,
                minimap: false,
                sticky: false,
                read_only: false,
                tab: 4,
            },
            marks: Marks::default(),
            marked: None,
            composing: None,
            metrics: Metrics::default(),
            dragging: false,
            caret_on: true,
            focused: false,
            chat_field,
            frame: Frame::default(),
            blink_epoch: 0,
            _blink: None,
            _subscriptions: subscriptions,
        }
    }

    pub fn language(mut self, name: impl Into<SharedString>) -> Self {
        self.options.language = name.into();
        self
    }

    pub fn line_numbers(mut self, numbers: LineNumbers) -> Self {
        self.options.numbers = numbers;
        self
    }

    pub fn cursor_shape(mut self, shape: CursorShape) -> Self {
        self.options.cursor = shape;
        self
    }

    /// Dots for spaces and arrows for tabs.
    pub fn show_whitespace(mut self) -> Self {
        self.options.whitespace = true;
        self
    }

    pub fn hide_indent_guides(mut self) -> Self {
        self.options.guides = false;
        self
    }

    /// Thin lines at these columns.
    pub fn rulers(mut self, columns: impl IntoIterator<Item = usize>) -> Self {
        self.options.rulers = columns.into_iter().collect();
        self
    }

    /// Brackets colored by how deep they sit.
    pub fn rainbow_brackets(mut self) -> Self {
        self.options.rainbow = true;
        self
    }

    pub fn minimap(mut self) -> Self {
        self.options.minimap = true;
        self
    }

    /// The lines that open the blocks around the top line stay pinned above it.
    pub fn sticky_scroll(mut self) -> Self {
        self.options.sticky = true;
        self
    }

    pub fn read_only(mut self) -> Self {
        self.options.read_only = true;
        self
    }

    /// Spaces a Tab inserts and an indent guide spans.
    pub fn tab_size(mut self, size: usize) -> Self {
        assert!(size > 0, "a tab spans at least one column");
        self.options.tab = size;
        self
    }

    pub fn text(&self) -> &str {
        self.buffer.text()
    }

    pub fn selections(&self) -> &[Selection] {
        &self.selections
    }

    /// The last cursor placed, which scrolling follows.
    pub fn primary(&self) -> Selection {
        *self.selections.last().expect("an editor keeps a cursor")
    }

    /// Line and column of the primary cursor, both from one.
    pub fn position(&self) -> (usize, usize) {
        let (line, column) = self.buffer.point(self.primary().head);
        (line + 1, column + 1)
    }

    pub fn language_name(&self) -> &SharedString {
        &self.options.language
    }

    /// The line the inline chat sits under, while it is open.
    pub fn chat_line(&self) -> Option<usize> {
        self.marks.chat
    }

    pub fn is_read_only(&self) -> bool {
        self.options.read_only
    }

    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        let before = self.snapshot();
        self.buffer = Buffer::new(text);
        self.selections = vec![Selection::caret(0)];
        self.folded.clear();
        self.commit(before, false, cx);
    }

    pub fn set_cursor_shape(&mut self, shape: CursorShape, cx: &mut Context<Self>) {
        self.options.cursor = shape;
        cx.notify();
    }

    pub fn set_line_numbers(&mut self, numbers: LineNumbers, cx: &mut Context<Self>) {
        self.options.numbers = numbers;
        cx.notify();
    }

    pub fn set_read_only(&mut self, read_only: bool, cx: &mut Context<Self>) {
        log::info!("code editor: read only {read_only}");
        self.options.read_only = read_only;
        cx.notify();
    }

    /// Selects these ranges, the last one primary.
    pub fn select(
        &mut self,
        ranges: impl IntoIterator<Item = Range<usize>>,
        cx: &mut Context<Self>,
    ) {
        let chosen: Vec<Selection> = ranges
            .into_iter()
            .map(|range| {
                assert!(
                    range.end <= self.buffer.len()
                        && self.buffer.text().is_char_boundary(range.start)
                        && self.buffer.text().is_char_boundary(range.end),
                    "a selection covers whole characters of the text"
                );
                Selection {
                    anchor: range.start,
                    head: range.end,
                    goal: None,
                }
            })
            .collect();
        if merged(chosen.clone()) == self.selections {
            return;
        }
        self.set_selections(chosen, cx);
        self.scroll
            .scroll_to_item(self.primary_row(), ScrollStrategy::Center);
    }

    pub fn set_diagnostics(&mut self, diagnostics: Vec<Diagnostic>, cx: &mut Context<Self>) {
        if self.marks.diagnostics == diagnostics {
            return;
        }
        self.marks.diagnostics = diagnostics;
        cx.notify();
    }

    pub fn set_backgrounds(
        &mut self,
        ranges: Vec<(Range<usize>, gpui::Hsla)>,
        cx: &mut Context<Self>,
    ) {
        if self.marks.backgrounds == ranges {
            return;
        }
        self.marks.backgrounds = ranges;
        cx.notify();
    }

    pub fn set_inlay_hints(&mut self, hints: Vec<InlayHint>, cx: &mut Context<Self>) {
        self.marks.hints = hints;
        self.marks.hints.sort_by_key(|hint| hint.offset);
        cx.notify();
    }

    pub fn set_code_lenses(&mut self, lenses: Vec<CodeLens>, cx: &mut Context<Self>) {
        self.marks.lenses = lenses;
        cx.notify();
    }

    /// Suggested text at an offset, in grey until Tab takes it.
    pub fn set_ghost_text(&mut self, ghost: Option<GhostText>, cx: &mut Context<Self>) {
        self.marks.ghost = ghost;
        cx.notify();
    }

    pub fn set_diff(&mut self, hunks: Vec<DiffHunk>, cx: &mut Context<Self>) {
        self.marks.hunks = hunks;
        cx.notify();
    }

    pub fn set_git_marks(&mut self, marks: Vec<(usize, GitMark)>, cx: &mut Context<Self>) {
        self.marks.git = marks;
        cx.notify();
    }

    pub fn toggle_breakpoint(&mut self, line: usize, cx: &mut Context<Self>) {
        if !self.marks.breakpoints.remove(&line) {
            self.marks.breakpoints.insert(line);
        }
        log::info!("code editor: breakpoints {:?}", self.marks.breakpoints);
        cx.notify();
    }

    pub fn breakpoints(&self) -> &BTreeSet<usize> {
        &self.marks.breakpoints
    }

    /// Opens the inline chat below a line, or closes it.
    pub fn set_chat(&mut self, line: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        self.marks.chat = line;
        if line.is_some() {
            let focus = self.chat_field.read(cx).focus().clone();
            window.focus(&focus, cx);
        } else {
            window.focus(&self.focus, cx);
            cx.emit(EditorEvent::ChatClosed);
        }
        cx.notify();
    }

    /// Replaces ranges of the text at once, as one undo step; read-only text refuses.
    pub fn edit(
        &mut self,
        edits: impl IntoIterator<Item = (Range<usize>, String)>,
        cx: &mut Context<Self>,
    ) {
        let edits: Vec<(Range<usize>, String)> = edits.into_iter().collect();
        assert!(
            edits.iter().all(|(range, _)| range.end <= self.buffer.len()
                && self.buffer.text().is_char_boundary(range.start)
                && self.buffer.text().is_char_boundary(range.end)),
            "an edit covers whole characters of the text"
        );
        self.apply(edits, false, cx);
    }

    pub(crate) fn set_selections(&mut self, selections: Vec<Selection>, cx: &mut Context<Self>) {
        self.selections = merged(selections);
        self.unfold_cursors();
        self.restart_blink(cx);
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.buffer.text().to_string(),
            selections: self.selections.clone(),
        }
    }

    pub(crate) fn restore(&mut self, snapshot: Snapshot, cx: &mut Context<Self>) {
        self.buffer = Buffer::new(snapshot.text);
        self.selections = snapshot.selections;
        self.marked = None;
        self.composing = None;
        cx.emit(EditorEvent::Changed);
        self.restart_blink(cx);
    }

    /// Makes `before` one undo step when the text moved on from it.
    pub(crate) fn commit(&mut self, before: Snapshot, typing: bool, cx: &mut Context<Self>) {
        if before.text != self.buffer.text() {
            if self.composing.is_none() {
                self.history.record(before, typing);
            }
            self.marks.ghost = None;
            self.folded.retain(|line| *line < self.buffer.lines());
            cx.emit(EditorEvent::Changed);
        }
        self.restart_blink(cx);
    }

    /// Shows the cursors now and restarts their blink; still under reduced motion.
    pub(crate) fn restart_blink(&mut self, cx: &mut Context<Self>) {
        self.caret_on = true;
        self.blink_epoch += 1;
        let epoch = self.blink_epoch;
        let still = cx.theme().reduced_motion;
        self._blink = (!still && self.focused).then(|| {
            cx.spawn(async move |editor, cx| {
                loop {
                    cx.background_executor().timer(BLINK).await;
                    let alive = editor.update(cx, |editor, cx| {
                        if editor.blink_epoch == epoch {
                            editor.caret_on = !editor.caret_on;
                            cx.notify();
                        }
                    });
                    if alive.is_err() {
                        return;
                    }
                }
            })
        });
        cx.notify();
    }

    pub(crate) fn editable(&self) -> bool {
        !self.options.read_only
    }
}
