use std::ops::Range;

use gpui::{
    AnyElement, App, Bounds, Context, ElementInputHandler, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, Pixels, Point, Render, Styled,
    StyledText, Window, canvas, div, fill, font, point, size, uniform_list,
};

use super::{
    buffer::clamped_range,
    decor::ReadOnlyBanner,
    keys::{CONTEXT, listen},
    layout::{Row, Shown, around, hidden, rows},
    state::{CodeEditor, EditorEvent},
    syntax::{self, Bracket, brackets, folds},
    wrap::{self, Wrap},
};
use crate::theme::{ActiveTheme, TextSize};

/// What one render worked out, for the rows the list builds after it.
#[derive(Default)]
pub(crate) struct Frame {
    pub rows: Vec<Row>,
    /// What each line wraps onto, by line: one sub-row spanning it whole while soft wrap is off, and the
    /// width the rows were cut to. A runtime switch or a resize leaves this stale until the next render.
    pub wrap: Vec<Vec<Wrap>>,
    /// Which lines a closed fold hid when this frame was built. The wrap table and the rows are both read
    /// through it, so a fold that goes in between two renders cannot make one of them disagree with the other.
    pub hidden: Vec<bool>,
    pub folds: Vec<(usize, usize)>,
    pub brackets: Vec<Bracket>,
    pub advance: Pixels,
    pub line: Pixels,
    pub matched: Option<(usize, usize)>,
    pub occurrences: Vec<Range<usize>>,
    /// The minimap's box, from its last paint.
    pub minimap: Bounds<Pixels>,
}

/// Lines of code a row's height holds, as leading.
const LEADING: f32 = 1.6;
/// Blocks the sticky header shows at most.
const STICKY: usize = 3;
/// Columns the minimap spans, in code characters.
const MINIMAP: f32 = 12.0;

impl CodeEditor {
    /// The text the primary selection or its word gives, for highlighting where else it appears.
    fn needle(&self) -> Option<String> {
        let primary = self.primary();
        let range = if primary.is_empty() {
            let word = self.buffer.word_at(primary.head);
            let text = &self.buffer.text()[word.clone()];
            text.chars()
                .all(|ch| ch.is_alphanumeric() || ch == '_')
                .then_some(word)?
        } else {
            primary.range()
        };
        (!range.is_empty()).then(|| self.buffer.text()[range].to_string())
    }

    /// The first line the list shows, after scrolling.
    fn top_line(&self) -> Option<usize> {
        let scrolled = -self.scroll.0.borrow().base_handle.offset().y;
        let first = (scrolled / self.frame.line).floor().max(0.0) as usize;
        self.frame.rows[first.min(self.frame.rows.len().saturating_sub(1))..]
            .iter()
            .find_map(|row| match row {
                Row::Line(line, _) => Some(*line),
                _ => None,
            })
    }

    fn sticky(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let colors = cx.theme().colors.clone();
        let top = self.top_line()?;
        let headers = around(&self.frame.folds, top, STICKY);
        if headers.is_empty() {
            return None;
        }
        let rows = headers.into_iter().map(|line| {
            let text = self.buffer.line(line).to_string();
            let styles = syntax::colors_in(&self.buffer, self.buffer.line_range(line), cx);
            let jump = cx.entity();
            div()
                .id(("sticky", line))
                .h(self.frame.line)
                .flex()
                .items_center()
                .cursor_pointer()
                .hover(|style| style.bg(colors.hover))
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    jump.update(cx, |editor, cx| {
                        let at = editor.buffer.line_range(line).start + editor.buffer.indent(line);
                        let cursor = at..at;
                        editor.select([cursor], cx);
                        editor.reveal(false, cx);
                    });
                })
                .child(
                    div()
                        .flex_none()
                        .w(self.frame.advance * (self.gutter_columns() + 1) as f32),
                )
                .child(
                    div()
                        .line_height(self.frame.line)
                        .whitespace_nowrap()
                        .text_color(colors.syntax.variable)
                        .child(StyledText::new(text).with_highlights(styles)),
                )
        });
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bg(colors.surface)
                .border_b_1()
                .border_color(colors.border)
                .children(rows)
                .into_any_element(),
        )
    }

    fn minimap_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let (advance, line) = (self.frame.advance, self.frame.line);
        let tokens: Vec<Vec<(usize, usize, gpui::Hsla)>> = self
            .frame
            .rows
            .iter()
            .map(|row| match row {
                // The minimap draws a line, not its sub-rows, so a continuation adds nothing.
                Row::Line(at, 0) => {
                    let text = self.buffer.line(*at);
                    syntax::colors_in(&self.buffer, self.buffer.line_range(*at), cx)
                        .into_iter()
                        .filter_map(|(range, style)| {
                            let start = text[..range.start].chars().count();
                            let len = text[range].chars().count();
                            style.color.map(|color| (start, len, color))
                        })
                        .collect()
                }
                _ => Vec::new(),
            })
            .collect();
        let scroll = self.scroll.clone();
        let entity = cx.entity();
        let (drag, press) = (cx.entity(), cx.entity());
        let shade = colors.hover;
        div()
            .id("minimap")
            .relative()
            .flex_none()
            .w(advance * MINIMAP)
            .h_full()
            .border_l_1()
            .border_color(colors.border)
            .cursor_pointer()
            .child(
                canvas(
                    move |bounds, _, cx| {
                        entity.update(cx, |editor, _| editor.frame.minimap = bounds);
                    },
                    move |bounds, _, window, _| {
                        let rows = tokens.len().max(1) as f32;
                        let tall = (line * 0.15).min(bounds.size.height / rows);
                        let wide = advance * 0.12;
                        let state = scroll.0.borrow();
                        let viewport = state.base_handle.bounds().size.height;
                        let scrolled = -state.base_handle.offset().y;
                        let top = bounds.origin.y + scrolled / line * tall;
                        let extent = size(bounds.size.width, viewport / line * tall);
                        window.paint_quad(fill(
                            Bounds::new(point(bounds.origin.x, top), extent),
                            shade,
                        ));
                        for (ix, spans) in tokens.iter().enumerate() {
                            for (start, len, color) in spans {
                                let origin = point(
                                    bounds.origin.x + wide * (*start + 2) as f32,
                                    bounds.origin.y + tall * ix as f32,
                                );
                                let extent = size(wide * *len as f32, tall * 0.7);
                                window.paint_quad(fill(
                                    Bounds::new(origin, extent),
                                    color.opacity(0.7),
                                ));
                            }
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                press.update(cx, |editor, cx| editor.jump(event.position.y, cx));
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                if event.dragging() {
                    drag.update(cx, |editor, cx| editor.jump(event.position.y, cx));
                }
            })
            .into_any_element()
    }

    /// Scrolls so the row under a minimap point sits in the middle of the view.
    fn jump(&mut self, y: Pixels, cx: &mut Context<Self>) {
        let bounds = self.frame.minimap;
        let rows = self.frame.rows.len().max(1) as f32;
        let tall = (self.frame.line * 0.15).min(bounds.size.height / rows);
        let row = ((y - bounds.origin.y) / tall).max(0.0);
        let state = self.scroll.0.borrow();
        let viewport = state.base_handle.bounds().size.height;
        let top = (self.frame.line * row - viewport / 2.0).max(Pixels::ZERO);
        state.base_handle.set_offset(point(Pixels::ZERO, -top));
        drop(state);
        cx.notify();
    }
}

/// View metrics and anchors.
///
/// This group is for the **owner**: a gutter, breakpoints, a completion or hover popup all need to know
/// what is drawn right now and where an offset sits on screen. The library answers only for a frame it has
/// measured — a frame that has not been laid out yet gives `None`, so an owner falls back to its own
/// defaults instead of taking a wrong place for the right one.
impl CodeEditor {
    /// The rows the viewport draws now, indexed into this frame's row table (not line numbers).
    ///
    /// `None` before the first layout, while no line height or viewport height is known. Folded bodies,
    /// ghost rows and code lenses make a row index differ from a line number, so this reports no line
    /// numbers: ask [`CodeEditor::line_at_row`] for one.
    pub fn visible_row_range(&self) -> Option<Range<usize>> {
        let line = self.metrics.line;
        if line <= Pixels::ZERO {
            return None;
        }
        let state = self.scroll.0.borrow();
        let viewport = state.base_handle.bounds().size.height;
        if viewport <= Pixels::ZERO {
            return None;
        }
        let scrolled = -state.base_handle.offset().y;
        let count = self.frame.rows.len();
        let first = ((scrolled / line).floor().max(0.0) as usize).min(count);
        let last = (((scrolled + viewport) / line).ceil().max(0.0) as usize).min(count);
        Some(first..last.max(first))
    }

    /// The 1-based line number a row-table index stands for; `None` when that row is not a whole document
    /// line (a code lens, a removed diff line, ghost text, the inline chat) or out of range.
    ///
    /// The companion of [`CodeEditor::visible_row_range`]: a folded body, a ghost row or a lens makes the row
    /// table shorter and coarser than the document, so `row + 1` is not a line number — ask here instead. With
    /// soft wrap on it is coarser still: every visual sub-row of a line stands for that line, so a line wider
    /// than the code area answers the same number for each of the rows it takes. It answers for the frame that
    /// is painted; before the first layout every index is out of range.
    pub fn line_at_row(&self, row: usize) -> Option<usize> {
        match self.frame.rows.get(row)? {
            Row::Line(line, _) => Some(line + 1),
            _ => None,
        }
    }

    /// One row's height; `None` before the first layout.
    pub fn line_height(&self) -> Option<Pixels> {
        (self.metrics.line > Pixels::ZERO).then_some(self.metrics.line)
    }

    /// The code area's scroll offset: a negative `y` means the content has scrolled up, the same sign as
    /// the upstream `EditorState::scroll_offset`.
    pub fn scroll_offset(&self) -> Point<Pixels> {
        self.scroll.0.borrow().base_handle.offset()
    }

    /// Scrolls the code area to `offset`, clamped to what the content allows (a negative `y` means the
    /// content has scrolled up, the same sign as [`CodeEditor::scroll_offset`]). The owner calls this from its
    /// wheel and scrollbar handling.
    ///
    /// The reading is the one an owner can compute before it calls — `-offset.y` is how far down the content
    /// stands — and it is clamped twice for safety: here against the last measured travel, so
    /// [`CodeEditor::scroll_offset`] answers a value that is already inside it, and again by the list on its next
    /// layout, so a frame that changed height in between (a resize, a fold, new text) still cannot be scrolled
    /// past its content. Before the first layout there is no measured content to scroll, so the offset stays at
    /// the top and nothing panics.
    pub fn set_scroll_offset(&mut self, offset: Point<Pixels>, cx: &mut Context<Self>) {
        // The list reports how far its content can travel with a **positive** sign — a distance rather than an
        // offset — so the range an offset may take is `-travel..0`, and content that fits has no travel at all.
        let reach = self.scroll.0.borrow().base_handle.max_offset();
        let clamped = point(
            offset.x.clamp(-reach.x, Pixels::ZERO),
            offset.y.clamp(-reach.y, Pixels::ZERO),
        );
        self.scroll.0.borrow().base_handle.set_offset(clamped);
        cx.notify();
    }

    /// The byte offset under a window point; `None` when that point is not on a row of code (the gutter,
    /// past the last row, a row that is not a whole line).
    ///
    /// The mouse-position lookup; for the other direction see [`CodeEditor::bounds_for_offset`]. `None`
    /// until a line height has been measured. A point below the first visual sub-row of a wrapped line is read
    /// against **that** sub-row, so a click lands on the characters it was aimed at.
    pub fn offset_at(&self, position: Point<Pixels>, cx: &App) -> Option<usize> {
        let metrics = self.metrics;
        if metrics.line <= Pixels::ZERO || metrics.advance <= Pixels::ZERO {
            return None;
        }
        let scrolled = -self.scroll.0.borrow().base_handle.offset().y;
        let row = ((position.y - metrics.body.y + scrolled) / metrics.line).floor();
        let row = self.frame.rows.get(row.max(0.0) as usize)?;
        let Row::Line(line, sub) = *row else {
            return None;
        };
        let wrap = self
            .line_wraps(line, self.frame_hidden())
            .get(sub)
            .cloned()?;
        Some(self.line_offset(line, &wrap, position.x, cx).0)
    }

    /// The rectangle (in window pixels) an offset occupies on screen, which an owner anchors its completion
    /// or hover popup to.
    ///
    /// Only **already drawn and currently visible** places are answered: not laid out yet
    /// ([`CodeEditor::visible_row_range`] gives `None`), hidden in a fold, or scrolled out of view all give
    /// `None` — so an owner shows nothing rather than something in the wrong place. An offset out of bounds
    /// or inside a multi-byte character is pulled onto a whole character first; it never panics.
    pub fn bounds_for_offset(&self, offset: usize, cx: &App) -> Option<Bounds<Pixels>> {
        let (row, bounds) = self.placed(offset, cx)?;
        self.visible_row_range()?.contains(&row).then_some(bounds)
    }

    /// Which row an offset is drawn on, and where in the window; it does **not** judge visibility (the input
    /// method wants to know where a composition would go, even off screen).
    ///
    /// Answers the row index and its rectangle; `None` when a fold hides that line, or while the row is not
    /// in this frame's row table yet. On a wrapped line the row is the one its **visual sub-row** takes, and
    /// the rectangle stands on that sub-row — so an owner anchoring a popup inside a continuation gets the
    /// rectangle it is drawn in rather than the first sub-row's.
    pub(crate) fn placed(&self, offset: usize, cx: &App) -> Option<(usize, Bounds<Pixels>)> {
        let offset = clamped_range(self.buffer.text(), offset..offset).start;
        let line = self.buffer.line_of(offset);
        let text = self.row_text(line);
        let shown = Shown::new(text, self.notes(line, cx), Vec::new());
        let range = self.row_range(line);
        // Which visual sub-row draws these bytes, and how far into that sub-row's own drawing they stand. The
        // column is the sub-row's drawn start plus what the offset adds inside it, so an offset on the line's
        // first sub-row draws where it always did and one past a break draws under the sub-row it fell in.
        let (sub, drawn) = match self.sub_row(line, offset - range.start) {
            Some((sub, wrap)) => {
                let within = (offset - range.start).min(wrap.range.end);
                let inside = shown.drawn_range(&(wrap.range.start..within));
                let drawn = wrap.column + shown.text[inside].chars().count();
                (sub, drawn)
            }
            None => {
                let column = shown.column(text, self.buffer.point(offset).1.min(text.len()));
                (0, column)
            }
        };
        let row = self
            .frame
            .rows
            .iter()
            .position(|row| *row == Row::Line(line, sub))?;
        let metrics = self.metrics;
        let scrolled = -self.scroll.0.borrow().base_handle.offset().y;
        let origin = point(
            metrics.left + metrics.advance * drawn as f32,
            metrics.body.y + metrics.line * row as f32 - scrolled,
        );
        Some((
            row,
            Bounds::new(origin, size(metrics.advance, metrics.line)),
        ))
    }
}

impl Render for CodeEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sticky = if self.options.sticky {
            self.sticky(cx)
        } else {
            None
        };
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let size = theme.text_size(TextSize::Sm);
        let pixels = size.to_pixels(window.rem_size());
        let mono = window
            .text_system()
            .resolve_font(&font(theme.mono_family.clone()));
        let advance = window
            .text_system()
            .advance(mono, pixels, 'm')
            .expect("the code font has an m")
            .width;
        let line = (pixels * LEADING).round();
        let found = folds(&self.buffer);
        let hide = hidden(self.buffer.lines(), &found, &self.folded);
        let found_brackets = brackets(&self.buffer);
        let matched = syntax::matched(&found_brackets, self.primary().head);
        let occurrences = match self.needle() {
            Some(needle) if self.focus.is_focused(window) => {
                syntax::occurrences(&self.buffer, &needle, 0..self.buffer.lines())
            }
            _ => Vec::new(),
        };
        let minimap = self.frame.minimap;
        // Where every line wraps, at the width the last layout measured; soft wrap off leaves each line the
        // one sub-row that spans it whole, so the table reads exactly as it did before soft wrap existed.
        // Cutting the text is what the rows are counted from, so it happens before the row table.
        let columns = self.wrap_columns();
        let segments: Vec<Vec<Wrap>> = (0..self.buffer.lines())
            .map(|line| {
                // A line a closed fold hides draws no rows, so there is nothing to cut it for; the fold's
                // header draws to the end of that header line, which is the whole line either way.
                if self.options.soft_wrap && !hide[line] {
                    wrap::segments(self.buffer.line(line), columns)
                } else {
                    vec![Wrap {
                        range: 0..self.row_range(line).len(),
                        column: 0,
                    }]
                }
            })
            .collect();
        self.frame = Frame {
            rows: rows(&self.buffer, &hide, &self.marks, |line| {
                segments.get(line).map_or(1, Vec::len)
            }),
            wrap: segments,
            hidden: hide,
            folds: found,
            brackets: found_brackets,
            advance,
            line,
            matched,
            occurrences,
            minimap,
        };
        self.metrics.advance = advance;
        self.metrics.line = line;
        // The first column of code, past the gutter. The row's own box says the same thing, but a row is only
        // painted when it is in view, so the frame reads it from the body the rows sit in and the gutter they
        // all leave: one answer for every row, wrapped or not.
        self.metrics.left = self.metrics.body.x + advance * self.gutter_columns() as f32;
        let hover = self.hover_panel(cx);
        let count = self.frame.rows.len();
        let list = uniform_list(
            "editor-rows",
            count,
            cx.processor(|editor, range: Range<usize>, window, cx| {
                range.map(|ix| editor.row(ix, window, cx)).collect()
            }),
        )
        .track_scroll(&self.scroll)
        .size_full();
        let (top, focus, input) = (cx.entity(), self.focus.clone(), cx.entity());
        let unlock = cx.entity();
        let banner = self.options.read_only.then(|| {
            ReadOnlyBanner::new(
                ("read-only", cx.entity_id().as_u64()),
                "This file is read-only.",
            )
            .action("Make editable", move |_, cx| {
                unlock.update(cx, |editor, cx| {
                    editor.set_read_only(false, cx);
                    cx.emit(EditorEvent::Unlock);
                })
            })
        });
        let body = div()
            .relative()
            .flex_1()
            .h_full()
            .child(list)
            .children(sticky)
            .children(hover)
            .child(
                canvas(
                    move |bounds, _, cx| {
                        top.update(cx, |editor, cx| {
                            editor.metrics.body = bounds.origin;
                            // The code area's width is what soft wrap cuts a line to, and it is only known
                            // here, so a change to it asks for the frame that rewraps at it.
                            if editor.metrics.body_size != bounds.size {
                                editor.metrics.body_size = bounds.size;
                                cx.notify();
                            }
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        let root = div()
            .id(("code-editor", cx.entity_id().as_u64()))
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            // Ctrl/Cmd held ⇒ the symbol under the pointer is drawn as a jump target (`styles` adds the
            // underline, `line_row` the hand cursor). The window reports modifier changes to the focus
            // path, so this listener on the focus-owning root is where the state stays current; without
            // the notify the underline would only appear after the next unrelated repaint.
            .on_modifiers_changed(cx.listener(
                |editor, event: &gpui::ModifiersChangedEvent, _, cx| {
                    let held = event.modifiers.secondary();
                    if editor.secondary_held != held {
                        editor.secondary_held = held;
                        cx.notify();
                    }
                },
            ))
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(colors.surface)
            .font_family(theme.mono_family.clone())
            .text_size(size)
            .text_color(colors.fg)
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|editor, _, _, _| editor.dragging = false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|editor, _, _, _| editor.dragging = false),
            )
            .children(banner)
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(body)
                    .children(self.options.minimap.then(|| self.minimap_view(cx))),
            )
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, cx| {
                        window.handle_input(
                            &focus,
                            ElementInputHandler::new(bounds, input.clone()),
                            cx,
                        );
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        listen(root, cx)
    }
}
