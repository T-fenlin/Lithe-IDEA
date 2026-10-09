//! A virtualized transcript: only the rows in view, following its tail, with a jump to the latest.

use std::{cell::Cell, ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, Context, ElementId, Entity, FollowMode, Hsla, InteractiveElement, IntoElement,
    ListAlignment, ListOffset, ListState, ParentElement, Pixels, Point, RenderOnce, SharedString,
    Styled, Window, div, list, prelude::*, px, rems,
};

use crate::{
    buttons::Button,
    theme::{ActiveTheme, Radius, Size},
};

/// How far past the viewport rows are measured. A transcript's rows are about
/// one line tall, and a large overdraw lets the scrollbar's reach settle before
/// the reader ever scrolls.
const LIST_OVERDRAW: Pixels = px(400.);

/// The scrolling state behind a [`MessageScroller`], owned by its view.
///
/// It holds only the virtual list's bookkeeping. The rows themselves stay with
/// the caller, which reads them back one at a time from the builder passed to
/// [`MessageScroller::new`].
pub struct MessageScrollerState {
    list: ListState,
}

impl MessageScrollerState {
    /// A state of `item_count` rows that follows the tail as rows arrive.
    ///
    /// The context is taken so the list's scroll handler can notify this entity
    /// once GPUI has released the list's own borrow.
    pub fn new(item_count: usize, cx: &mut Context<Self>) -> Self {
        let list = ListState::new(item_count, ListAlignment::Top, LIST_OVERDRAW);
        let weak = cx.weak_entity();
        list.set_scroll_handler(move |_, _, cx| {
            let weak = weak.clone();
            cx.defer(move |cx| {
                let _ = weak.update(cx, |_, cx| cx.notify());
            });
        });
        list.set_follow_mode(FollowMode::Tail);
        Self { list }
    }

    /// How many rows the virtual list knows about.
    pub fn item_count(&self) -> usize {
        self.list.item_count()
    }

    /// Whether the reader has scrolled away from the latest row.
    pub fn is_scrolled_up(&self) -> bool {
        self.list.max_offset_for_scrollbar().y > px(0.)
            && !self.list.is_following_tail()
            && !self.list.is_scrolled_to_end().unwrap_or(false)
    }

    /// Whether the list is pinned to its last row.
    pub fn is_following_tail(&self) -> bool {
        self.list.is_following_tail()
    }

    /// Reset the list to exactly `item_count` rows and follow the tail again.
    pub fn reset(&mut self, item_count: usize, cx: &mut Context<Self>) {
        self.list.reset(item_count);
        self.list.set_follow_mode(FollowMode::Tail);
        cx.notify();
    }

    /// Replace `old_range` with `count` new rows.
    ///
    /// Returns `false` and changes nothing when the range is outside the list.
    pub fn splice(
        &mut self,
        old_range: Range<usize>,
        count: usize,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.holds(&old_range) {
            return false;
        }
        let neighbor = old_range.start.checked_sub(1);
        self.list.splice(old_range, count);
        // A row that stopped being last changed height, so remeasure it and the
        // survivor beside the splice.
        if let Some(last) = self.list.item_count().checked_sub(1) {
            self.list.remeasure_items(last..last + 1);
            if let Some(neighbor) = neighbor.filter(|neighbor| *neighbor != last) {
                self.list.remeasure_items(neighbor..neighbor + 1);
            }
        }
        cx.notify();
        true
    }

    /// Append `count` rows to the end.
    pub fn append(&mut self, count: usize, cx: &mut Context<Self>) -> bool {
        let item_count = self.list.item_count();
        self.splice(item_count..item_count, count, cx)
    }

    /// Insert `count` rows at the start, keeping the reader's row in place.
    pub fn prepend(&mut self, count: usize, cx: &mut Context<Self>) -> bool {
        self.splice(0..0, count, cx)
    }

    /// Mark every row for remeasurement.
    pub fn remeasure(&mut self, cx: &mut Context<Self>) {
        self.list.remeasure();
        cx.notify();
    }

    /// Mark `range` for remeasurement; `false` when it is outside the list.
    pub fn remeasure_items(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> bool {
        if !self.holds(&range) {
            return false;
        }
        self.list.remeasure_items(range);
        cx.notify();
        true
    }

    /// Scroll to the row at `index`; `false` when there is no such row.
    pub fn scroll_to_item(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        if index >= self.list.item_count() {
            return false;
        }
        self.list.scroll_to(ListOffset {
            item_ix: index,
            offset_in_item: px(0.),
        });
        cx.notify();
        true
    }

    /// Follow the tail again and scroll to the latest row.
    pub fn scroll_to_end(&mut self, cx: &mut Context<Self>) {
        self.list.set_follow_mode(FollowMode::Tail);
        self.list.scroll_to_end();
        cx.notify();
    }

    fn list(&self) -> ListState {
        self.list.clone()
    }

    fn holds(&self, range: &Range<usize>) -> bool {
        range.start <= range.end && range.end <= self.list.item_count()
    }
}

type Row = Rc<dyn Fn(usize, &mut Window, &mut App) -> AnyElement>;

/// A virtualized row list that follows its tail: a chat, a log, a terminal transcript.
///
/// Rows live with the caller and are read back one at a time by `row`, so a
/// transcript of any length builds only what is on screen. While the reader
/// sits at the last row, new rows pull the view along; once they scroll up the
/// view stays where it is and a jump-to-latest button appears over the bottom
/// edge. Give it a height.
#[derive(IntoElement)]
pub struct MessageScroller {
    id: ElementId,
    state: Entity<MessageScrollerState>,
    row: Row,
    scrollbar: bool,
    jump: bool,
    jump_label: SharedString,
    bottom_fade: Option<Hsla>,
}

impl MessageScroller {
    /// A transcript of `state`'s rows, each built by `row(index)`.
    pub fn new(
        id: impl Into<ElementId>,
        state: Entity<MessageScrollerState>,
        row: impl Fn(usize, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            state,
            row: Rc::new(row),
            scrollbar: true,
            jump: true,
            jump_label: "Jump to latest".into(),
            bottom_fade: None,
        }
    }

    /// Show or hide the overlay scrollbar. Shown by default.
    pub fn scrollbar(mut self, scrollbar: bool) -> Self {
        self.scrollbar = scrollbar;
        self
    }

    /// Show or hide the jump-to-latest button. Shown by default.
    pub fn jump_button(mut self, jump: bool) -> Self {
        self.jump = jump;
        self
    }

    /// The words the jump-to-latest button is named by.
    pub fn with_jump_button_label(mut self, label: impl Into<SharedString>) -> Self {
        self.jump_label = label.into();
        self
    }

    /// Fade the bottom edge into `color` while the reader is away from the last row.
    ///
    /// A half-visible row melts into the surface behind the transcript instead
    /// of being cut mid-line. Pass that surface's color; off by default, and
    /// drawn under the scrollbar and the jump button.
    pub fn with_bottom_fade(mut self, color: impl Into<Hsla>) -> Self {
        self.bottom_fade = Some(color.into());
        self
    }
}

impl RenderOnce for MessageScroller {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (scrolled, rows) = {
            let state = self.state.read(cx);
            (state.is_scrolled_up(), state.list())
        };
        let row = self.row;
        let fg = cx.theme().colors.fg;
        let jump_label = self.jump_label;
        let target = self.state.clone();
        let fade = self.bottom_fade;
        let scrollbar = self.scrollbar;

        let list_el = list(rows.clone(), move |ix, window, cx| {
            div()
                .w_full()
                .min_w_0()
                .child(row(ix, window, cx))
                .into_any_element()
        })
        .size_full()
        .min_h_0();

        let view = div()
            .relative()
            .size_full()
            .min_h_0()
            .min_w_0()
            .child(list_el)
            .when(scrollbar, |view| {
                view.child(ScrollThumb::new((self.id.clone(), "scrollbar"), &rows))
            })
            .when_some(fade, |view, color| {
                view.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .h(rems(3.))
                        .bg(color)
                        // At the live edge nothing is clipped, so a fade there
                        // would promise content that is not below.
                        .when(!scrolled, |edge| edge.invisible()),
                )
            });

        div()
            .id(self.id.clone())
            .relative()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .child(view)
            .when(self.jump && scrolled, |root| {
                root.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom(rems(0.5))
                        .flex()
                        .justify_center()
                        .child(
                            Button::new((self.id, "jump-to-latest"))
                                .label(jump_label.clone())
                                .size(Size::Small)
                                .tooltip(jump_label)
                                .on_click(move |_, _, cx| {
                                    target.update(cx, |state, cx| state.scroll_to_end(cx));
                                }),
                        ),
                )
            })
            .text_color(fg)
    }
}

/// What the reader grabbed while dragging the transcript's scrollbar.
struct ThumbDrag {
    list: ListState,
    /// Where inside the thumb the grab landed, in the thumb's own pixels.
    grab: Pixels,
    /// How far the thumb may travel: the track minus the thumb.
    travel: Pixels,
    /// The list's full scrollable distance, which one thumb-travel maps onto.
    reach: Pixels,
}

/// The overlay thumb for a virtual list.
///
/// Dragging it moves the list through `set_offset_from_scrollbar`; the list's
/// own wheel handling is left alone, so the wheel keeps working while the thumb
/// only paints and follows.
#[derive(IntoElement)]
struct ScrollThumb {
    id: ElementId,
    list: ListState,
}

impl ScrollThumb {
    fn new(id: impl Into<ElementId>, list: &ListState) -> Self {
        Self {
            id: id.into(),
            list: list.clone(),
        }
    }
}

impl RenderOnce for ScrollThumb {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let reach = self.list.max_offset_for_scrollbar().y;
        let view = self.list.viewport_bounds().size.height;
        if reach <= Pixels::ZERO || view <= Pixels::ZERO {
            return div().into_any_element();
        }
        let theme = cx.theme();
        let rem = window.rem_size();
        let thickness = theme.scrollbar_thickness().to_pixels(rem);
        let min_thumb = theme.scrollbar_min_thumb().to_pixels(rem);
        let length = (view * (view / (view + reach))).max(min_thumb).min(view);
        let travel = view - length;
        let offset = self.list.scroll_px_offset_for_scrollbar().y;
        let start = travel * (offset / reach).clamp(0.0, 1.0);
        let list = self.list;
        let (idle, busy) = (theme.colors.fg.opacity(0.22), theme.colors.fg.opacity(0.42));
        let grabbed = Rc::new(Cell::new(Pixels::ZERO));
        let remember = grabbed.clone();

        div()
            .id(self.id.clone())
            .absolute()
            .top_0()
            .bottom_0()
            .right_0()
            .w(thickness * 2.0)
            .pl(thickness * 0.5)
            .child(
                div()
                    .id((self.id, "thumb"))
                    .absolute()
                    .top(start)
                    .right_0()
                    .w(thickness)
                    .h(length)
                    .rounded_full()
                    .bg(idle)
                    .hover(|style| style.bg(busy))
                    .on_drag(
                        ThumbDrag {
                            list: list.clone(),
                            grab: Pixels::ZERO,
                            travel,
                            reach,
                        },
                        move |drag, _, _, cx| {
                            remember.set(drag.grab);
                            cx.new(|_| Grip)
                        },
                    )
                    .on_drag_move(move |event: &gpui::DragMoveEvent<ThumbDrag>, window, cx| {
                        let drag = event.drag(cx);
                        let local = event.event.position - event.bounds.origin;
                        let ratio = ((start + local.y - drag.grab) / drag.travel).clamp(0.0, 1.0);
                        drag.list
                            .set_offset_from_scrollbar(Point::new(px(0.), -(drag.reach * ratio)));
                        cx.notify(window.current_view());
                    }),
            )
            .into_any_element()
    }
}

/// The empty element a thumb drag carries; the drag's own payload does the work.
struct Grip;

impl gpui::Render for Grip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

// `Radius` is named by this module's prose, not by its code.
#[allow(unused_imports)]
use Radius as _;
