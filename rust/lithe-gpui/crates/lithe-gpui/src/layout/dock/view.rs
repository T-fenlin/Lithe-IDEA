use gpui::{
    AnyElement, Axis, Context, DragMoveEvent, EmptyView, EntityId, FontWeight, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Pixels, Point, Render, SharedString,
    StatefulInteractiveElement, Styled, Window, div, point, prelude::*, relative,
};

use super::{Dock, DockSide, Spot};
use crate::{
    buttons::{Button, ButtonVariant},
    i18n::text,
    layout::{FloatingPanel, resize_handle},
    primitives::{DragGhost, Icon, IconName},
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
    fn tabs(&self, side: DockSide, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let active = self.active[side.index()];
        let zoom = active.map(|ix| self.panels[ix].id.clone());
        div()
            .flex()
            .items_center()
            .justify_between()
            .h(theme.control_height(Size::Large))
            .px_1()
            .border_b_1()
            .border_color(theme.colors.border)
            .child(div().flex().items_center().gap_0p5().children(
                self.shown_on_side(side).into_iter().map(|ix| {
                    let panel = &self.panels[ix];
                    let selected = active == Some(ix);
                    let (id, title, icon) = (panel.id.clone(), panel.title.resolve(cx), panel.icon);
                    let drag = DraggedPanel {
                        dock: cx.entity_id(),
                        id: id.clone(),
                    };
                    div()
                        .id(SharedString::from(format!("tab-{id}")))
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
                }),
            ))
            .children(zoom.map(|id| {
                let off = self.is_zoomed(&id);
                let close_id = id.clone();
                let float_id = id.clone();
                // One strip per side, so the controls.sc ids name the side: two sides
                // sharing an id would collide in the accessibility tree.
                let (close_el, float_el, zoom_el) = (
                    SharedString::from(format!("dock-close-{side:?}")),
                    SharedString::from(format!("dock-float-{side:?}")),
                    SharedString::from(format!("dock-zoom-{side:?}")),
                );
                // The heading of the panel a side is showing: close it, float it, or
                // fill the dock with it.
                div()
                    .flex()
                    .items_center()
                    .gap_0p5()
                    .child(
                        Button::new(close_el.clone())
                            .icon(IconName::X)
                            .variant(ButtonVariant::Ghost)
                            .size(Size::Small)
                            .tooltip(text(cx, "dock.close", &[]))
                            .on_click(cx.listener(move |dock, _, _, cx| dock.hide(&close_id, cx))),
                    )
                    .child(
                        Button::new(float_el.clone())
                            .icon(IconName::PictureInPicture2)
                            .variant(ButtonVariant::Ghost)
                            .size(Size::Small)
                            .tooltip(text(cx, "dock.float", &[]))
                            .on_click(
                                cx.listener(move |dock, _, _, cx| dock.toggle_float(&float_id, cx)),
                            ),
                    )
                    .child(
                        Button::new(zoom_el.clone())
                            .icon(if off {
                                IconName::Minimize2
                            } else {
                                IconName::Maximize2
                            })
                            .variant(ButtonVariant::Ghost)
                            .size(Size::Small)
                            .tooltip(text(
                                cx,
                                if off { "dock.restore" } else { "dock.zoom" },
                                &[],
                            ))
                            .on_click(cx.listener(move |dock, _, _, cx| dock.toggle_zoom(&id, cx))),
                    )
            }))
            .into_any_element()
    }

    fn side(
        &self,
        side: DockSide,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        // The side shows the chosen panel when it is open, else its first open
        // panel, and nothing at all once every panel on it is closed.
        let shown = self.shown_on_side(side);
        let active = shown
            .iter()
            .copied()
            .find(|ix| Some(*ix) == self.active[side.index()])
            .or_else(|| shown.first().copied())?;
        let size = self.sizes[side.index()];
        let panel_id = self.panels[active].id.clone();
        let body = (self.content)(&panel_id, window, cx);
        let theme = cx.theme();
        let pane = div()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(theme.colors.surface)
            .child(self.tabs(side, cx))
            .child(div().flex_1().min_h_0().child(body));
        let axis = match side {
            DockSide::Left | DockSide::Right => Axis::Horizontal,
            DockSide::Top | DockSide::Bottom => Axis::Vertical,
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
        let pane = match axis {
            Axis::Horizontal => pane.w(size).h_full(),
            Axis::Vertical => pane.h(size).w_full(),
        };
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
        // A zoomed panel takes the whole dock until it is restored, tabs and all.
        if let Some(ix) = self.zoomed.filter(|ix| !self.hidden[*ix]) {
            let id = self.panels[ix].id.clone();
            let body = (self.content)(&id, window, cx);
            let side = match self.spots[ix] {
                Spot::Docked(side) => side,
                Spot::Floating { .. } => DockSide::Left,
            };
            return div()
                .id("dock")
                .relative()
                .size_full()
                .flex()
                .flex_col()
                .overflow_hidden()
                .bg(cx.theme().colors.surface)
                .child(self.tabs(side, cx))
                .child(div().flex_1().min_h_0().child(body))
                .into_any_element();
        }
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
        let closed = self.closed(cx);
        let rem = window.rem_size();
        let own = cx.entity_id();
        div()
            .id("dock")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .on_drag_move(
                cx.listener(move |dock, event: &DragMoveEvent<DockResize>, _, cx| {
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
                    let cap = match side {
                        DockSide::Left | DockSide::Right => bounds.size.width * 0.45,
                        DockSide::Top | DockSide::Bottom => bounds.size.height * 0.45,
                    };
                    // The side.s own range, with the four-fifths cap standing in for a
                    // side that sets no maximum.
                    let limits = dock.limits[side.index()];
                    let low = limits.min.to_pixels(rem);
                    let high = limits
                        .max
                        .map(|max| max.to_pixels(rem))
                        .unwrap_or(cap)
                        .max(low);
                    let size = span.max(low).min(high);
                    dock.sizes[side.index()] = gpui::rems(f32::from(size) / f32::from(rem));
                    cx.notify();
                }),
            )
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
                            .child(div().flex_1().min_h_0().child(center))
                            .children(bottom),
                    )
                    .children(right)
                    .children(floating)
                    .when(showing_zones, |dock| dock.child(self.zones(cx))),
            )
            .children(closed)
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
        let back_id = id.clone();
        let close_id = id.clone();
        let actions = [
            Button::new(SharedString::from(format!("float-dock-{id}")))
                .icon(IconName::PictureInPicture2)
                .variant(ButtonVariant::Ghost)
                .size(Size::Small)
                .tooltip(text(cx, "dock.dock", &[]))
                .on_click(cx.listener(move |dock, _, _, cx| dock.toggle_float(&back_id, cx)))
                .into_any_element(),
            Button::new(SharedString::from(format!("float-close-{id}")))
                .icon(IconName::X)
                .variant(ButtonVariant::Ghost)
                .size(Size::Small)
                .tooltip(text(cx, "dock.close", &[]))
                .on_click(cx.listener(move |dock, _, _, cx| dock.hide(&close_id, cx)))
                .into_any_element(),
        ];
        FloatingPanel::new(
            SharedString::from(format!("float-{id}")),
            panel.title.resolve(cx),
            start,
        )
        .actions(actions)
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

impl Dock {
    /// The closed panels as buttons that open them again, when the strip is on.
    ///
    /// An application with an activity bar leaves this off: a toggle there is the
    /// usual way back, and a second one would be noise.
    fn closed(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let closed: Vec<usize> = (0..self.panels.len())
            .filter(|ix| self.hidden[*ix])
            .collect();
        if !self.closed_strip || closed.is_empty() {
            return None;
        }
        let theme = cx.theme();
        Some(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_1()
                .px_2()
                .py_1()
                .border_t_1()
                .border_color(theme.colors.border)
                .bg(theme.colors.sunken)
                .children(closed.into_iter().map(|ix| {
                    let panel = &self.panels[ix];
                    let (id, title, icon) = (panel.id.clone(), panel.title.resolve(cx), panel.icon);
                    Button::new(SharedString::from(format!("dock-closed-{id}")))
                        .icon(icon)
                        .label(title)
                        .variant(ButtonVariant::Ghost)
                        .size(Size::Small)
                        .on_click(cx.listener(move |dock, _, _, cx| dock.show(&id, cx)))
                }))
                .into_any_element(),
        )
    }
}
