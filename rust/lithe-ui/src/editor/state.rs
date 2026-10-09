use std::{collections::BTreeSet, ops::Range, time::Duration};

use gpui::{
    App, AppContext as _, Context, EventEmitter, FocusHandle, Focusable, Pixels, Point,
    ScrollStrategy, SharedString, Subscription, Task, UniformListScrollHandle, Window,
};

use super::{
    buffer::Buffer,
    cursor::{Selection, merged},
    decor::{CodeLens, Diagnostic, DiffHunk, GhostText, GitMark, InlayHint},
    hints::CompletionItem,
    hover::HoverInfo,
    view::Frame,
};
use crate::{
    forms::{History, InputEvent, InputState},
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

/// A navigation key pressed while a popup owned by the owner has the keyboard.
///
/// The editor did **not** act on it: a popup that keeps the keyboard while the caret still belongs to the
/// editor (a completion menu, a code-action list) would otherwise never see its own navigation keys, because
/// the editor's bindings take them first. The owner decides what each key means for its popup — moving the
/// highlight, accepting, closing — and the library keeps no state beyond whether a popup is active
/// ([`CodeEditor::set_popup_active`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupKey {
    /// `up`.
    Up,
    /// `down`.
    Down,
    /// `pageup`.
    PageUp,
    /// `pagedown`.
    PageDown,
    /// `enter`.
    Enter,
    /// `tab`.
    Tab,
    /// `escape`.
    Escape,
    /// `backspace`.
    Backspace,
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
    /// The owner should ask for completions: the primary cursor's byte offset.
    ///
    /// The library reports the moment and nothing else: the **owner** queries its language server and pushes
    /// the answer back through [`CodeEditor::set_completions`]. Explicit `ctrl-space`, automatic completion
    /// and the owner's own entry points all raise this one event.
    CompletionRequested(usize),
    /// The owner should ask for a hover: the byte offset where the symbol under the pointer starts.
    ///
    /// The same split as [`EditorEvent::CompletionRequested`] — the library asks, the owner fetches the content
    /// and pushes it back with [`CodeEditor::present_hover`]. Leaving a symbol raises nothing: when to take a
    /// hover down is the owner's business.
    HoverRequested(usize),
    /// A navigation key the owner's popup owns, which the editor left alone.
    ///
    /// Raised only while [`CodeEditor::set_popup_active`] says a popup is active; the popup's selection,
    /// highlight, acceptance and dismissal are all the owner's business.
    PopupKey(PopupKey),
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
    /// Whether typing an identifier character asks for completions automatically; off by default (see
    /// [`CodeEditor::auto_completion`]).
    pub auto_completion: bool,
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

/// The hover content the owner pushed back, and the offset it hangs from.
pub(crate) struct Hover {
    pub info: HoverInfo,
    /// The start of the symbol the last [`EditorEvent::HoverRequested`] named, or the primary cursor when none was asked.
    pub offset: usize,
}

/// The text and cursors one undo step returns to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Snapshot {
    pub text: String,
    pub selections: Vec<Selection>,
}

/// Where the code sat when last painted, in window pixels: the text's left edge, the body's top-left corner, a character's width and a row's height.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Metrics {
    /// The first column of code, past the gutter.
    pub left: Pixels,
    /// The scrollable code area's top-left corner, which a host layers its own popups over.
    pub body: Point<Pixels>,
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
    /// Completions the owner pushed: the library only keeps the data; rendering, placing and driving the
    /// popup all stay with the owner.
    pub(crate) completions: Vec<CompletionItem>,
    /// Hover content the owner pushed: drawn inside the editor while it is `Some`.
    pub(crate) hover: Option<Hover>,
    /// The symbol the pointer is over now, so one symbol is asked for only once
    /// ([`EditorEvent::HoverRequested`]).
    pub(crate) hover_symbol: Option<Range<usize>>,
    /// While true, the navigation keys a popup uses are reported instead of acted on
    /// ([`CodeEditor::set_popup_active`]).
    pub(crate) popup_active: bool,
    pub(crate) dragging: bool,
    pub(crate) caret_on: bool,
    /// Blink runs only while focused, or unseen cursors redraw forever.
    focused: bool,
    pub(crate) chat_field: gpui::Entity<InputState>,
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
            cx.new(|cx| InputState::new(window, cx).placeholder("Ask to change this code"));
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
                auto_completion: false,
            },
            marks: Marks::default(),
            marked: None,
            composing: None,
            metrics: Metrics::default(),
            completions: Vec::new(),
            hover: None,
            hover_symbol: None,
            popup_active: false,
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

    /// Ask for completions after an identifier character (a letter, a digit or an underscore); **off** by
    /// default.
    ///
    /// While it is on, every such character typed raises [`EditorEvent::CompletionRequested`]; fetching the
    /// data and showing the popup still belong to the owner. The explicit `ctrl-space` trigger is independent
    /// of this switch — both entry points raise the same event.
    pub fn auto_completion(mut self, on: bool) -> Self {
        self.options.auto_completion = on;
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

    /// The primary cursor's **byte offset**, in the same coordinates as [`Buffer`].
    ///
    /// The way an owner hands a position to its language server (a completion or hover position, a code
    /// action's range); the editor knows nothing about LSP and asks for nothing itself.
    pub fn cursor_offset(&self) -> usize {
        self.primary().head
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

    /// Tells the editor that a popup owned by the owner (completion menu, code actions, …) has the keyboard.
    /// While active the editor stops acting on the navigation keys below and reports them
    /// ([`EditorEvent::PopupKey`]) instead, so the owner can drive its popup; everything else — typing, the input
    /// method, selection, scrolling — keeps working, and the editor keeps the focus.
    ///
    /// The keys handed over are Up, Down, PageUp, PageDown, Enter, Tab, Escape and Backspace: with the popup up
    /// they move its highlight, accept, close or step back instead of moving the caret, inserting, indenting or
    /// deleting. Their modified forms are **not** handed over — `shift-up`, `shift-tab`, `ctrl-home` and the
    /// editing keys (`delete`, `ctrl-backspace`) keep their editor behaviour, so an owner that wants one of those
    /// handles it in its own key context. The library reports only what was pressed: choosing, highlighting,
    /// accepting and closing stay the owner's, and no popup state beyond this switch is kept here.
    ///
    /// The popup is the owner's own surface, so the owner also clears this when it closes; a popup that is
    /// dismissed keeps the editor from acting on those keys until it does.
    pub fn set_popup_active(&mut self, active: bool, cx: &mut Context<Self>) {
        if self.popup_active == active {
            return;
        }
        self.popup_active = active;
        cx.notify();
    }

    /// Reports a navigation key to the owner when its popup owns the keyboard, and answers whether it did.
    ///
    /// The one place the hand-over is decided: every action a popup key is bound to asks here first, so the
    /// editor's own behaviour can only run while no popup is active.
    pub(crate) fn report_popup_key(&mut self, key: PopupKey, cx: &mut Context<Self>) -> bool {
        if !self.popup_active {
            return false;
        }
        cx.emit(EditorEvent::PopupKey(key));
        true
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

    /// The completions the owner fetched.
    ///
    /// The same shape as `set_diagnostics`/`set_inlay_hints`/`set_code_lenses`: the **owner** asks its language
    /// server and pushes the answer in, and the library only keeps the data (read it back with
    /// [`CodeEditor::completions`]). The popup's rendering, placement and keyboard navigation are not in the
    /// library: an owner anchors its own popup with [`CodeEditor::bounds_for_offset`].
    pub fn set_completions(&mut self, items: Vec<CompletionItem>, cx: &mut Context<Self>) {
        if self.completions == items {
            return;
        }
        log::info!("code editor: {} completions", items.len());
        self.completions = items;
        cx.notify();
    }

    /// Takes the completions down: the owner clears them when it closes its popup, or when a newer request
    /// replaces this one.
    pub fn clear_completions(&mut self, cx: &mut Context<Self>) {
        if self.completions.is_empty() {
            return;
        }
        self.completions.clear();
        cx.notify();
    }

    /// The completions the owner pushed; empty means this request suggested nothing.
    pub fn completions(&self) -> &[CompletionItem] {
        &self.completions
    }

    /// Hover content the owner fetched: while it is `Some` the **library** draws it in the editor, and `None`
    /// takes it down.
    ///
    /// The split is the other way round from completions — how the content is fetched (a language server,
    /// documentation, disk) is the owner's, how it is drawn is the library's. It hangs from the symbol the last
    /// [`EditorEvent::HoverRequested`] named, or from the primary cursor when none was asked. A text change
    /// makes the content stale without clearing it: the owner pushes again, or pushes `None`, when it moves on.
    pub fn present_hover(&mut self, info: Option<HoverInfo>, cx: &mut Context<Self>) {
        let offset = self
            .hover_symbol
            .as_ref()
            .map_or_else(|| self.cursor_offset(), |symbol| symbol.start);
        self.hover = info.map(|info| Hover { info, offset });
        cx.notify();
    }

    /// The hover content on show.
    pub fn hover(&self) -> Option<&HoverInfo> {
        self.hover.as_ref().map(|hover| &hover.info)
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

    /// Ask for completions now: report the primary cursor's byte offset to the owner
    /// ([`EditorEvent::CompletionRequested`]).
    ///
    /// The library makes no language-server request — `ctrl-space`, automatic completion and the owner's own
    /// entry points all arrive here, and the answer comes back through [`CodeEditor::set_completions`].
    /// Read-only text may ask too: asking changes nothing. Automatic completion reaches this on every
    /// identifier character, so it logs at `debug`.
    pub(crate) fn request_completion(&mut self, cx: &mut Context<Self>) {
        let offset = self.cursor_offset();
        log::debug!("code editor: completion asked at {offset}");
        cx.emit(EditorEvent::CompletionRequested(offset));
    }

    /// After text was typed: ask for completions when automatic completion is on, and this really was one
    /// identifier character.
    ///
    /// Several characters at once (a paste, a committed composition) are not typing character by character,
    /// so they ask for nothing.
    pub(crate) fn request_after_typing(&mut self, typed: &str, cx: &mut Context<Self>) {
        if !self.options.auto_completion {
            return;
        }
        let mut chars = typed.chars();
        let identifier = matches!(
            (chars.next(), chars.next()),
            (Some(ch), None) if ch.is_alphanumeric() || ch == '_'
        );
        if identifier {
            self.request_completion(cx);
        }
    }
}
