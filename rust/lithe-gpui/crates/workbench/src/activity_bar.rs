use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, Hsla, IntoElement, Length, ParentElement as _, Pixels,
    SharedString, Styled as _, Window, div, rems,
};
use gpui_kit::assets::IconName;
use gpui_kit::component::badge::Badge;
use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Placement, Selectable as _};
use lithe_gpui_shared::design;
use lithe_gpui_shared::icons::{idea, idea_icon_image};

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

#[derive(Clone, Debug)]
pub enum ActivityIcon {
    Lucide(IconName),
    Idea(&'static idea::IdeaIcon),
}

impl ActivityIcon {
    fn apply(&self, button: Button, cx: &App) -> Button {
        match self {
            Self::Lucide(name) => button.icon(*name),
            Self::Idea(icon) => button.child(idea_icon_image(icon, cx, rems(1.25))),
        }
    }
}

#[non_exhaustive]
pub struct ActivityItem {
    icon: ActivityIcon,
    label: SharedString,
    bottom: bool,
    badge: Option<Hsla>,
}

impl ActivityItem {
    pub fn new(icon: IconName, label: impl Into<SharedString>) -> Self {
        Self {
            icon: ActivityIcon::Lucide(icon),
            label: label.into(),
            bottom: false,
            badge: None,
        }
    }

    pub fn idea(icon: &'static idea::IdeaIcon, label: impl Into<SharedString>) -> Self {
        Self {
            icon: ActivityIcon::Idea(icon),
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
    let theme = cx.theme();
    let accent = theme.accent;
    let selected_background = theme.primary;

    let (foreground, hover_background) = match side {
        ActivitySide::Left => (theme.foreground, accent.opacity(HOVER_OPACITY)),
        ActivitySide::Right => (theme.muted_foreground, accent),
    };
    let (id_name, placement): (&str, Placement) = match side {
        ActivitySide::Left => ("lithe-activity-bar-left", Placement::Right),
        ActivitySide::Right => ("lithe-activity-bar-right", Placement::Left),
    };
    let (width, height, radius): (Length, Length, Pixels) = (
        rems(ITEM_SIZE / 16.).into(),
        rems(ITEM_SIZE / 16.).into(),
        crate::rem_px(rem, ITEM_RADIUS),
    );

    let is_active = is_active(index);
    let button = item.icon.apply(
        Button::new(ElementId::named_usize(id_name, index))
            .custom(
                ButtonCustomVariant::new(cx)
                    .foreground(foreground)
                    .hover(hover_background)
                    .active(selected_background),
            )
            .selected(is_active)
            .toggled(is_active),
        cx,
    );
    let button = button
        .tooltip(item.label.clone())
        .tooltip_placement(placement)
        .accessibility_label(item.label.clone())
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
            .child(Badge::new().dot().color(color).child(button))
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
