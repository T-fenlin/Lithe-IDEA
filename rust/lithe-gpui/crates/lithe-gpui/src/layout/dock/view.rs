use gpui::{
    AnyElement, Axis, Context, DragMoveEvent, EmptyView, EntityId, FontWeight, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Pixels, Point, Rems, Render, SharedString,
    StatefulInteractiveElement, Styled, Window, div, point, prelude::*, relative,
};

use super::{Dock, DockSide, Spot};
use crate::{
    layout::{FloatingPanel, resize_handle},
    primitives::{DragGhost, Icon},
    theme::{ActiveTheme, IconSize, Radius, Size, TextSize},
};

struct DraggedPanel {
    dock: EntityId,
    id: SharedString,
}

struct DockResize {
    dock: EntityId,
    side: DockSide,
}

impl Dock {
    /// A side's heading: its panels as tabs, dragged to move them.
    fn tabs(&self, side: DockSide, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let active = self.active[side.index()];
        div()
            .flex()
            .items_center()
            .gap_0p5()
            .h(theme.control_height(Size::Large))
            .px_1()
            .border_b_1()
            .border_color(theme.colors.border)
            .children(self.shown_in(side).into_iter().map(|ix| {
                let panel = &self.panels[ix];
                let selected = active == Some(ix);
                let (id, title, icon) = (panel.id.clone(), panel.title.resolve(cx), panel.icon);
                let drag = DraggedPanel {
                    dock: cx.entity_id(),
                    id: id.clone(),
                };
                div()
                    .id(SharedString::from(format!("tab-{side:?}-{id}")))
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .px_2()
                    .h(theme.control_height(Size::Small))
                    .rounded(theme.radius(Radius::Md))
                    .cursor_pointer()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(if selected {
                        theme.colors.fg
                    } else {
                        theme.colors.fg_subtle
                    })
                    .when(selected, |tab| {
                        tab.bg(theme.colors.hover).font_weight(FontWeight::MEDIUM)
                    })
                    .hover(|style| style.text_color(theme.colors.fg))
                    .child(Icon::new(icon).size(IconSize::Sm).color(if selected {
                        theme.colors.fg
                    } else {
                        theme.colors.fg_subtle
                    }))
                    .child(title.clone())
                    .on_click(cx.listener(move |dock, _, _, cx| dock.activate(&id, cx)))
                    .on_drag(drag, move |_, _, _, cx| {
                        DragGhost::new(title.clone(), Some(icon), cx)
                    })
            }))
            .into_any_element()
    }

    /// A side: the panel it shows, with a divider to size it. `None` when the
    /// side has nothing open, which is how an empty side draws nothing.
    fn side(
        &self,
        side: DockSide,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let shown = self.shown_in(side);
        let active = shown
            .iter()
            .copied()
            .find(|ix| Some(*ix) == self.active[side.index()])
            .or_else(|| shown.first().copied())?;
        let panel = &self.panels[active];
        let header = panel.header;
        let panel_id = panel.id.clone();
        let body = (self.content)(&panel_id, window, cx);
        let theme = cx.theme();
        let pane = div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(theme.colors.surface)
            .when(header, |pane| pane.child(self.tabs(side, cx)))
            .child(div().flex_1().min_h_0().flex().flex_col().child(body));
        let size = self.sizes[side.index()];
        let axis = match side {
            DockSide::Left | DockSide::Right => Axis::Horizontal,
            DockSide::Top | DockSide::Bottom => Axis::Vertical,
        };
        let pane = match axis {
            Axis::Horizontal => pane.w(size).h_full(),
            Axis::Vertical => pane.h(size).w_full(),
        };
        let resize = DockResize {
            dock: cx.entity_id(),
            side,
        };
        let handle = resize_handle(
            SharedString::from(format!("dock-handle-{side:?}")),
            axis,
            cx,
        )
        .on_drag(resize, |_, _, _, cx| cx.new(|_| EmptyView));
        let row = div().flex().flex_none();
        Some(
            match side {
                DockSide::Left => row.h_full().child(pane).child(handle),
                DockSide::Right => row.h_full().child(handle).child(pane),
                DockSide::Top => row.w_full().flex_col().child(pane).child(handle),
                DockSide::Bottom => row.w_full().flex_col().child(handle).child(pane),
            }
            .into_any_element(),
        )
    }

    /// The drop zones a dragging tab reveals: one per side, and a middle one that
    /// floats the panel.
    fn zones(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (lit, edge, idle) = (
            theme.colors.focus.opacity(0.12),
            theme.colors.focus.opacity(0.7),
            theme.colors.border_strong,
        );
        let radius = theme.radius(Radius::Lg);
        let zone = |id: &'static str, spot: Option<DockSide>, cx: &mut Context<Self>| {
            div()
                .id(id)
                .absolute()
                .rounded(radius)
                .border_2()
                .border_dashed()
                .border_color(idle)
                .drag_over::<DraggedPanel>(move |style, _, _, _| style.bg(lit).border_color(edge))
                .on_drop(
                    cx.listener(move |dock, dragged: &DraggedPanel, window, cx| {
                        if dragged.dock != cx.entity_id() {
                            return;
                        }
                        let target = match spot {
                            Some(side) => Spot::Docked(side),
                            None => {
                                let at = window.mouse_position();
                                Spot::Floating {
                                    x: f32::from(at.x),
                                    y: f32::from(at.y),
                                }
                            }
                        };
                        dock.dragging = false;
                        dock.move_panel(&dragged.id, target, cx);
                    }),
                )
        };
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .p_2()
            .child(
                zone("zone-left", Some(DockSide::Left), cx)
                    .top_2()
                    .bottom_2()
                    .left_2()
                    .w(relative(0.2)),
            )
            .child(
                zone("zone-right", Some(DockSide::Right), cx)
                    .top_2()
                    .bottom_2()
                    .right_2()
                    .w(relative(0.2)),
            )
            .child(
                zone("zone-top", Some(DockSide::Top), cx)
                    .top_2()
                    .left(relative(0.22))
                    .right(relative(0.22))
                    .h(relative(0.25)),
            )
            .child(
                zone("zone-bottom", Some(DockSide::Bottom), cx)
                    .bottom_2()
                    .left(relative(0.22))
                    .right(relative(0.22))
                    .h(relative(0.25)),
            )
            .child(
                zone("zone-float", None, cx)
                    .top(relative(0.3))
                    .left(relative(0.3))
                    .right(relative(0.3))
                    .h(relative(0.4)),
            )
            .into_any_element()
    }
}

impl Render for Dock {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let left = self.side(DockSide::Left, window, cx);
        let right = self.side(DockSide::Right, window, cx);
        let top = self.side(DockSide::Top, window, cx);
        let bottom = self.side(DockSide::Bottom, window, cx);
        let center = (self.center)(window, cx);
        let floating: Vec<AnyElement> = (0..self.panels.len())
            .filter_map(|ix| match self.spots[ix] {
                // A closed panel is gone from the window too, not only from its side.
                Spot::Floating { x, y } if !self.hidden[ix] => {
                    Some((ix, point(Pixels::from(x), Pixels::from(y))))
                }
                _ => None,
            })
            .map(|(ix, start)| self.floating(ix, start, window, cx))
            .collect();
        let showing_zones = self.dragging && cx.has_active_drag();
        let rem = window.rem_size();
        let own = cx.entity_id();
        div()
            .id("dock")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .on_drag_move(cx.listener(
                move |dock, event: &DragMoveEvent<DockResize>, window, cx| {
                    let resize = event.drag(cx);
                    if resize.dock != own {
                        return;
                    }
                    let side = resize.side;
                    let bounds = event.bounds;
                    let pointer = event.event.position;
                    let span = match side {
                        DockSide::Left => pointer.x - bounds.left(),
                        DockSide::Right => bounds.right() - pointer.x,
                        DockSide::Top => pointer.y - bounds.top(),
                        DockSide::Bottom => bounds.bottom() - pointer.y,
                    };
                    let fallback = Rems::from_pixels(
                        match side {
                            DockSide::Left | DockSide::Right => bounds.size.width * 0.45,
                            DockSide::Top | DockSide::Bottom => bounds.size.height * 0.45,
                        },
                        window,
                    );
                    let extent = Rems::from_pixels(
                        match side {
                            DockSide::Left | DockSide::Right => bounds.size.width,
                            DockSide::Top | DockSide::Bottom => bounds.size.height,
                        },
                        window,
                    );
                    // The side's own range: its maximum, tighter of the space its
                    // reserve leaves the center, and never under its minimum.
                    let limits = dock.limits[side.index()];
                    let wanted = Rems::from_pixels(span, window);
                    let size = limits.clamp_in(wanted, extent, fallback, rem);
                    if dock.sizes[side.index()] != size {
                        dock.sizes[side.index()] = size;
                        cx.emit(super::DockEvent::Resized { side, size });
                        cx.notify();
                    }
                },
            ))
            .on_drag_move(
                cx.listener(|dock, event: &DragMoveEvent<DraggedPanel>, _, cx| {
                    if event.drag(cx).dock == cx.entity_id() && !dock.dragging {
                        dock.dragging = true;
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|dock, _, _, _| dock.dragging = false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|dock, _, _, _| dock.dragging = false),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .children(left)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .children(top)
                            .child(div().flex_1().min_h_0().flex().flex_col().child(center))
                            .children(bottom),
                    )
                    .children(right)
                    .children(floating)
                    .when(showing_zones, |dock| dock.child(self.zones(cx))),
            )
            .into_any_element()
    }
}

impl Dock {
    fn floating(
        &self,
        ix: usize,
        start: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let panel = &self.panels[ix];
        let id = panel.id.clone();
        let body = (self.content)(&id, window, cx);
        let dock_id = id.clone();
        FloatingPanel::new(
            SharedString::from(format!("float-{id}")),
            panel.title.resolve(cx),
            start,
        )
        .w(cx.theme().sidebar_width(false))
        .h(cx.theme().sheet_size())
        .on_move(cx.listener(move |dock, at: &Point<Pixels>, _, _| {
            let ix = dock.find(&dock_id).expect("dock panels are never removed");
            dock.spots[ix] = Spot::Floating {
                x: f32::from(at.x),
                y: f32::from(at.y),
            };
        }))
        .child(body)
        .into_any_element()
    }
}
