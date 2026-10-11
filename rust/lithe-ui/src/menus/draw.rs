use gpui::{
    AnyElement, App, Div, ElementId, Entity, FontWeight, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Stateful, Styled, div, prelude::*,
};

use super::{
    menu::{Open, at_depth, choose, mark, measure},
    model::{Entry, Kind, Menu, MenuItem},
};
use crate::{
    forms::{Run, surface},
    navigation::marked,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, Size, TextSize},
    typography::KbdCombo,
};

fn row(item: &MenuItem, ix: usize, current: bool, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let colors = &theme.colors;
    let fg = if item.disabled {
        colors.fg_disabled
    } else {
        colors.fg
    };
    // 富行：外壳照旧（高度下限、内边距、圆角、hover 底、整行可点），内容全由调用方画。
    if let Some(build) = &item.element {
        return div()
            .id(("row", ix))
            .relative()
            .flex()
            .items_center()
            .min_h(theme.control_height(Size::Medium))
            .px_2()
            .rounded(theme.radius(Radius::Md))
            .text_color(fg)
            .when(current && !item.disabled, |row| row.bg(colors.chrome_hover))
            .when(!item.disabled, |row| row.cursor_pointer())
            .child(build(cx));
    }
    let (lead, lead_color) = match &item.kind {
        Kind::Check(true) => (Some(IconName::Check), fg),
        Kind::Radio(true) => (Some(IconName::Dot), fg),
        _ => (item.icon, if item.disabled { fg } else { colors.fg_muted }),
    };
    // 图标槽：调用方自画的元素优先（宿主自己的图标集，见 `MenuItem::icon_element`），
    // 其次是上面的库图标；槽位尺寸两种都一样，行不会跳。
    let lead = match &item.leading {
        Some(build) => Some(build(cx)),
        None => lead.map(|icon| {
            Icon::new(icon)
                .size(IconSize::Sm)
                .color(lead_color)
                .into_any_element()
        }),
    };
    div()
        .id(("row", ix))
        .relative()
        .flex()
        .items_center()
        .gap_2()
        .h(theme.control_height(Size::Medium))
        .px_2()
        .rounded(theme.radius(Radius::Md))
        .text_color(fg)
        .when(current && !item.disabled, |row| row.bg(colors.chrome_hover))
        .when(!item.disabled, |row| row.cursor_pointer())
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(theme.icon_size(IconSize::Sm))
                .children(lead),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(if item.hits.is_empty() {
                    item.label.clone().into_any_element()
                } else {
                    marked(item.label.clone(), item.hits.clone(), cx)
                }),
        )
        .when_some(item.keys.clone(), |row, keys| {
            row.child(
                div()
                    .flex_none()
                    .pl_4()
                    .text_color(colors.fg_subtle)
                    .child(KbdCombo::new(&keys)),
            )
        })
        .when(matches!(item.kind, Kind::Sub(_)), |row| {
            row.child(
                Icon::new(IconName::ChevronRight)
                    .size(IconSize::Xs)
                    .color(colors.fg_subtle),
            )
        })
}

pub(super) fn panel(
    id: &ElementId,
    menu: &Menu,
    state: &Entity<Open>,
    depth: usize,
    close: &Run,
    head: Option<AnyElement>,
    cx: &App,
) -> Stateful<Div> {
    let open = state.read(cx);
    let here = at_depth(menu, &open.levels, depth).expect("settled levels follow submenu rows");
    let at = open.levels[depth].at;
    let theme = cx.theme();
    let colors = &theme.colors;
    let rows = here
        .entries
        .iter()
        .enumerate()
        .map(|(ix, entry)| match entry {
            Entry::Separator => div().h_px().my_1().bg(colors.border).into_any_element(),
            Entry::Heading(title) => div()
                .px_2()
                .pt_1p5()
                .pb_1()
                .text_size(theme.text_size(TextSize::Xs))
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors.fg_subtle)
                .child(title.clone())
                .into_any_element(),
            Entry::Item(item) => {
                let marked = at == Some(ix);
                let sub = matches!(item.kind, Kind::Sub(_));
                let (hover, click, close, menu) =
                    (state.clone(), state.clone(), close.clone(), menu.clone());
                let hover_menu = menu.clone();
                row(item, ix, marked, cx)
                    .when(!item.disabled, |row| {
                        row.on_mouse_move(move |_, _, cx| {
                            let open = hover.read(cx);
                            let Some(level) = open.levels.get(depth) else {
                                return;
                            };
                            let shown = open.levels.len() > depth + 1;
                            if level.at != Some(ix) || (sub && !shown) {
                                mark(&hover, &hover_menu, depth, ix, true, cx);
                            }
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            move |_, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                                choose(&click, &menu, depth, ix, &close, window, cx);
                            },
                        )
                    })
                    .when(marked && sub, |row| {
                        row.child(measure(state.clone(), move |open, bounds| {
                            let level = &mut open.levels[depth];
                            let changed = level.row != bounds;
                            level.row = bounds;
                            changed
                        }))
                    })
                    .into_any_element()
            }
        });
    let panel = surface((id.clone(), format!("panel-{depth}")), cx)
        .relative()
        .p_1()
        .flex()
        .flex_col();
    // 面板宽度：定宽（最小=最大）或主题的菜单宽度；超高的面板自己滚。
    let panel = match here.width {
        Some(width) => panel.min_w(width).max_w(width),
        None => panel.min_w(theme.menu_width()),
    };
    let panel = match here.max_height {
        Some(height) => panel.max_h(height).overflow_y_scroll(),
        None => panel,
    };
    panel
        .children(head)
        .children(rows)
        .child(measure(state.clone(), move |open, bounds| {
            let level = &mut open.levels[depth];
            let changed = level.panel != bounds;
            level.panel = bounds;
            changed
        }))
}
