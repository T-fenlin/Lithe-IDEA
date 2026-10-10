use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, Hsla, IntoElement, Length, ParentElement as _, Pixels,
    SharedString, Styled as _, Window, div, rems,
};
use lithe_gpui_shared::design;
use lithe_gpui_shared::icons::{idea, idea_icon_image};
use lithe_ui::buttons::{Button, ButtonCustomVariant};
use lithe_ui::data_display::DotBadge;
use lithe_ui::theme::ActiveTheme as _;

const COLLAPSED_WIDTH: f32 = design::ACTIVITY_RAIL_WIDTH_SPEC;
const WORKBENCH_GAP: f32 = 4.;
const ITEM_SIZE: f32 = 30.;
const ITEM_RADIUS: f32 = 6.;
const SLOT_WIDTH: f32 = 37.;
const SLOT_HEIGHT: f32 = 40.;

const HOVER_OPACITY: f32 = 0.7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivitySide {
    Left,
    Right,
}

const fn rail_width_spec(side: ActivitySide) -> f32 {
    match side {
        ActivitySide::Left => COLLAPSED_WIDTH + WORKBENCH_GAP,
        ActivitySide::Right => COLLAPSED_WIDTH,
    }
}

#[non_exhaustive]
pub struct ActivityItem {
    icon: &'static idea::IdeaIcon,
    label: SharedString,
    bottom: bool,
    badge: Option<Hsla>,
}

impl ActivityItem {
    /// A rail item drawn from the app's own icon set (`assets/icons/idea/**`). There is no
    /// library-icon constructor: an app icon the repo does not carry is a same-size blank, not a
    /// borrowed component icon.
    pub fn idea(icon: &'static idea::IdeaIcon, label: impl Into<SharedString>) -> Self {
        Self {
            icon,
            label: label.into(),
            bottom: false,
            badge: None,
        }
    }

    pub fn bottom(mut self, bottom: bool) -> Self {
        self.bottom = bottom;
        self
    }

    pub fn badge(mut self, color: Hsla) -> Self {
        self.badge = Some(color);
        self
    }
}

pub fn activity_bar(
    side: ActivitySide,
    items: &[ActivityItem],
    is_active: impl Fn(usize) -> bool + 'static,
    on_select: impl Fn(usize, &ClickEvent, &mut Window, &mut App) + 'static,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    let rem = window.rem_size();
    let on_select: Rc<dyn Fn(usize, &ClickEvent, &mut Window, &mut App)> = Rc::new(on_select);
    let is_active: Rc<dyn Fn(usize) -> bool> = Rc::new(is_active);

    let top_items: Vec<AnyElement> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.bottom)
        .map(|(index, item)| {
            item_button(side, index, item, &*is_active, on_select.clone(), rem, cx)
                .into_any_element()
        })
        .collect();
    let bottom_items: Vec<AnyElement> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.bottom)
        .map(|(index, item)| {
            item_button(side, index, item, &*is_active, on_select.clone(), rem, cx)
                .into_any_element()
        })
        .collect();
    let has_bottom_group = !bottom_items.is_empty();

    let column_width = rail_width_spec(side);
    let slot_inset = rems(((column_width - SLOT_WIDTH) / 2.) / 16.);
    let outer_pad_end = match side {
        ActivitySide::Left => rems(0.),
        ActivitySide::Right => rems(0.25),
    };

    let top_group = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .w_full()
        .px(slot_inset)
        .overflow_hidden()
        .children(top_items);

    let bottom_group = div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .w_full()
        .px(slot_inset)
        .children(bottom_items);

    let mut rail = div()
        .flex()
        .flex_col()
        .items_center()
        .h_full()
        .w(rems(column_width / 16.))
        .flex_shrink_0()
        .overflow_hidden()
        .child(top_group);
    if has_bottom_group {
        rail = rail.child(bottom_group);
    }

    div()
        .flex()
        .flex_row()
        .h_full()
        .flex_shrink_0()
        .pr(outer_pad_end)
        .child(rail)
}

fn item_button(
    side: ActivitySide,
    index: usize,
    item: &ActivityItem,
    is_active: &dyn Fn(usize) -> bool,
    on_select: Rc<dyn Fn(usize, &ClickEvent, &mut Window, &mut App)>,
    rem: Pixels,
    cx: &App,
) -> AnyElement {
    let colors = &cx.theme().colors;
    let accent = colors.accent;

    let (foreground, hover_background) = match side {
        ActivitySide::Left => (colors.fg, accent.opacity(HOVER_OPACITY)),
        ActivitySide::Right => (colors.fg_muted, accent),
    };
    let id_name = match side {
        ActivitySide::Left => "lithe-activity-bar-left",
        ActivitySide::Right => "lithe-activity-bar-right",
    };
    let (width, height, radius): (Length, Length, Pixels) = (
        rems(ITEM_SIZE / 16.).into(),
        rems(ITEM_SIZE / 16.).into(),
        crate::rem_px(rem, ITEM_RADIUS),
    );

    let is_active = is_active(index);
    let button = Button::new(ElementId::named_usize(id_name, index))
        .icon_element(idea_icon_image(item.icon, cx, rems(1.25)))
        .custom(
            ButtonCustomVariant::new(cx)
                .foreground(foreground)
                .hover(hover_background)
                .active(colors.primary),
        )
        .selected(is_active)
        .toggled(is_active)
        .tooltip(item.label.clone())
        .aria_label(item.label.clone())
        .debug_selector(move || format!("lithe-activity-bar-item-{index}"))
        .rounded(radius)
        .w(width)
        .h(height)
        .px_0()
        .on_click(move |event, window, cx| on_select(index, event, window, cx));

    let slot = div()
        .w(rems(SLOT_WIDTH / 16.))
        .h(rems(SLOT_HEIGHT / 16.))
        .flex()
        .items_center()
        .justify_center();

    match item.badge {
        None => slot.child(button).into_any_element(),
        Some(color) => slot
            .child(DotBadge::new().color(color).over(button))
            .into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_rails_take_their_width_from_the_shared_owner() {
        assert_eq!(design::ACTIVITY_RAIL_WIDTH_SPEC, 40.);
        assert_eq!(
            COLLAPSED_WIDTH,
            design::ACTIVITY_RAIL_WIDTH_SPEC,
            "rail 宽只能来自 shared::design 的 owner，不得在本文件另写一份数字"
        );
        assert_eq!(rail_width_spec(ActivitySide::Right), 40.);
        assert_eq!(rail_width_spec(ActivitySide::Left), 44.);
        assert_eq!(
            rail_width_spec(ActivitySide::Left),
            rail_width_spec(ActivitySide::Right) + WORKBENCH_GAP
        );
    }
}
