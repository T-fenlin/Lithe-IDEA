use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, FontWeight, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    forms::{Choice, Pick, step},
    motion::{self, Axis, Marker, glide, measure_item, measure_origin, slide},
    primitives::{Icon, tab_stop},
    theme::{ActiveTheme, IconSize, Radius, Size, TextSize},
};

/// Where a tab strip sits against its panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabPlacement {
    #[default]
    Top,
    Left,
    Bottom,
}

type OnValue = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnContext = Rc<dyn Fn(&SharedString, Point<Pixels>, &mut Window, &mut App)>;

/// A strip of tabs and the chosen tab's panel. A line slides to the chosen tab; arrows move it.
#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    tabs: Vec<Choice>,
    selected: SharedString,
    placement: TabPlacement,
    panel: Option<AnyElement>,
    on_change: Option<OnValue>,
    on_context: Option<OnContext>,
}

impl Tabs {
    pub fn new(
        id: impl Into<ElementId>,
        tabs: impl IntoIterator<Item = Choice>,
        selected: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            tabs: tabs.into_iter().collect(),
            selected: selected.into(),
            placement: TabPlacement::default(),
            panel: None,
            on_change: None,
            on_context: None,
        }
    }

    pub fn placement(mut self, placement: TabPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// The chosen tab's content.
    pub fn panel(mut self, panel: impl IntoElement) -> Self {
        self.panel = Some(panel.into_any_element());
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Tabs {
    /// A right click on a tab: its value and the pointer.
    pub fn on_context(
        mut self,
        handler: impl Fn(&SharedString, Point<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_context = Some(Rc::new(handler));
        self
    }
}

/// One tab of a [`TabBar`]: its own content, an optional trailing slot (a close
/// button) and the words screen readers announce it by.
///
/// 内容先收进一个 `Div`（[`Tab::new`] 取任意元素、[`Tab::label`] 取一句正文），
/// 所以 `Tab` 自己**不是**一个 `RenderOnce` 组件——它只是 [`TabBar`] 攒起来的一张表。
pub struct Tab {
    content: AnyElement,
    suffix: Option<AnyElement>,
    aria_label: Option<SharedString>,
}

impl Tab {
    /// A tab whose body is `content`.
    pub fn new(content: impl IntoElement) -> Self {
        Self {
            content: div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .child(content)
                .into_any_element(),
            suffix: None,
            aria_label: None,
        }
    }

    /// A tab whose body is a line of text.
    pub fn label(label: impl Into<SharedString>) -> Self {
        Self::new(label.into())
    }

    /// Its trailing slot, beside the content: a close button, a badge.
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// The words it is announced by, for a tab whose content does not read as its name.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }
}

impl From<&'static str> for Tab {
    fn from(label: &'static str) -> Self {
        Tab::label(label)
    }
}

impl From<SharedString> for Tab {
    fn from(label: SharedString) -> Self {
        Tab::label(label)
    }
}

/// A strip of tabs whose bodies belong to the caller. Each tab carries its own
/// content and an optional trailing slot, and the chosen one is drawn with the
/// accent underline.
///
/// Unlike [`super::Tabs`], which turns plain [`Choice`]s into labels, this one
/// takes whole elements, so a tab can hold a close button or any other control.
/// A `suffix` on the bar itself holds the controls that belong beside the strip
/// (new tab, clear, a profile menu).
#[derive(IntoElement)]
pub struct TabBar {
    id: ElementId,
    tabs: Vec<Tab>,
    selected: usize,
    suffix: Option<AnyElement>,
    max_width: Option<Pixels>,
    on_click: Option<OnIndex>,
}

type OnIndex = Rc<dyn Fn(usize, &mut Window, &mut App)>;

impl TabBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            tabs: Vec::new(),
            selected: 0,
            suffix: None,
            max_width: None,
            on_click: None,
        }
    }

    /// Which tab is drawn as chosen; out of range draws none of them chosen.
    pub fn selected_index(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    /// Add one tab.
    pub fn child(mut self, tab: impl Into<Tab>) -> Self {
        self.tabs.push(tab.into());
        self
    }

    /// Add several tabs.
    pub fn children(mut self, tabs: impl IntoIterator<Item = impl Into<Tab>>) -> Self {
        self.tabs.extend(tabs.into_iter().map(Into::into));
        self
    }

    /// The controls beside the strip, after the last tab.
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Cap each tab's width.
    pub fn max_width(mut self, width: impl Into<Pixels>) -> Self {
        self.max_width = Some(width.into());
        self
    }

    /// Runs with the pressed tab's index.
    pub fn on_click(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TabBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let height = theme.control_height(Size::Medium);
        let thickness = theme.underline_thickness();
        let radius = theme.radius(Radius::Md);
        let text = theme.text_size(TextSize::Sm);
        let max_width = self.max_width;
        let tabs = self
            .tabs
            .into_iter()
            .enumerate()
            .map(|(ix, tab)| {
                let chosen = ix == self.selected;
                let click = self.on_click.clone();
                let label = tab.aria_label.clone();
                let (fg, weight) = if chosen {
                    (colors.fg, FontWeight::MEDIUM)
                } else {
                    (colors.fg_muted, FontWeight::NORMAL)
                };
                let mut item = div()
                    .id(("tab-bar-item", ix))
                    .role(Role::Tab)
                    .aria_selected(chosen)
                    .when_some(label, |item, label| item.aria_label(label))
                    .relative()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_1p5()
                    .h(height)
                    .px_3()
                    .rounded(radius)
                    .text_size(text)
                    .font_weight(weight)
                    .text_color(fg)
                    .map(|item| match max_width {
                        Some(width) => item.max_w(width),
                        None => item,
                    })
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, move |_, window, _| {
                        window.prevent_default()
                    })
                    .on_click(move |_, window, cx| {
                        if let Some(click) = &click {
                            click(ix, window, cx);
                        }
                    })
                    .hover(|style| style.text_color(colors.fg));
                if chosen {
                    item = item.child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .bottom_0()
                            .h(thickness)
                            .bg(colors.accent),
                    );
                }
                item.child(tab.content).children(tab.suffix)
            })
            .collect::<Vec<_>>();

        div()
            .id(self.id)
            .flex()
            .items_center()
            .min_w_0()
            .border_b_1()
            .border_color(colors.border)
            .children(tabs)
            .children(
                self.suffix
                    .map(|suffix| div().flex_none().px_1().child(suffix)),
            )
    }
}

impl RenderOnce for Tabs {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let values: Vec<SharedString> = self.tabs.iter().map(|tab| tab.value.clone()).collect();
        let chosen = values
            .iter()
            .position(|value| *value == self.selected)
            .unwrap_or_else(|| panic!("tabs {:?} have no tab {}", self.id, self.selected));
        let placement = self.placement;
        let axis = match placement {
            TabPlacement::Left => Axis::Vertical,
            TabPlacement::Top | TabPlacement::Bottom => Axis::Horizontal,
        };
        let (state, marker) = slide(
            (self.id.clone(), "slide"),
            &values,
            &self.selected,
            window,
            cx,
        );
        let turn = motion::changes(
            (self.id.clone(), "panel"),
            self.selected.clone(),
            window,
            cx,
        );
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let focused = focus.is_focused(window);
        let tabs = Rc::new(self.tabs);
        let pick: Pick = {
            let (id, tabs, on_change) = (self.id.clone(), tabs.clone(), self.on_change);
            Rc::new(move |ix, window, cx| {
                let value = &tabs[ix].value;
                log::info!("tabs {id:?}: {value}");
                if let Some(on_change) = &on_change {
                    on_change(value, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let (thickness, duration) = (theme.tab_indicator(), motion::duration(motion::SLOW, cx));
        let indicator = marker.map(
            |Marker {
                 from,
                 to,
                 generation,
             }| {
                let bar = div().absolute().bg(colors.accent);
                let bar = match placement {
                    TabPlacement::Top => bar.bottom_0().h(thickness),
                    TabPlacement::Bottom => bar.top_0().h(thickness),
                    TabPlacement::Left => bar.right_0().w(thickness),
                };
                bar.with_animation(
                    ("tab-indicator", generation),
                    Animation::new(duration),
                    move |bar, t| {
                        let (start, length) = glide(from, to, t);
                        match axis {
                            Axis::Horizontal => bar.left(start).w(length),
                            Axis::Vertical => bar.top(start).h(length),
                        }
                    },
                )
            },
        );
        let items = tabs
            .iter()
            .enumerate()
            .map(|(ix, tab)| {
                let on = ix == chosen;
                let fg = match (tab.disabled, on) {
                    (true, _) => colors.fg_disabled,
                    (false, true) => colors.fg,
                    (false, false) => colors.fg_muted,
                };
                let pick = pick.clone();
                let context = self.on_context.clone().filter(|_| !tab.disabled);
                let value = tab.value.clone();
                div()
                    .id(("tab", ix))
                    .role(Role::Tab)
                    .aria_selected(on)
                    .aria_label(tab.label.clone())
                    .relative()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .h(theme.control_height(Size::Medium))
                    .px_3()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(if focused && on {
                        colors.focus
                    } else {
                        gpui::transparent_black()
                    })
                    .text_size(theme.text_size(TextSize::Sm))
                    .font_weight(if on {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(fg)
                    .when(!tab.disabled, |item| {
                        item.cursor_pointer()
                            .hover(|style| style.text_color(colors.fg))
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| {
                                if !on {
                                    pick(ix, window, cx);
                                }
                            })
                    })
                    .when_some(context, |item, context| {
                        item.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                            log::info!("tabs: context on {value}");
                            context(&value, event.position, window, cx);
                        })
                    })
                    .when_some(tab.icon, |item, icon| {
                        item.child(Icon::new(icon).size(IconSize::Sm).color(fg))
                    })
                    .child(tab.label.clone())
                    .when_some(tab.note.clone(), |item, note| {
                        item.child(div().text_color(colors.fg_subtle).child(note))
                    })
                    .child(measure_item(state.clone(), ix, axis))
            })
            .collect::<Vec<_>>();
        let (keys, key_pick) = (tabs.clone(), pick.clone());
        let strip = div()
            .id((self.id.clone(), "strip"))
            .role(Role::TabList)
            .track_focus(&focus)
            .relative()
            .flex()
            .when(axis == Axis::Vertical, |strip| strip.flex_col())
            .gap_1()
            .border_color(colors.border)
            .map(|strip| match placement {
                TabPlacement::Top => strip.border_b_1(),
                TabPlacement::Bottom => strip.border_t_1(),
                TabPlacement::Left => strip.border_r_1().pr_1(),
            })
            .on_key_down(move |event, window, cx| {
                let back = if axis == Axis::Vertical { "up" } else { "left" };
                let ahead = if axis == Axis::Vertical {
                    "down"
                } else {
                    "right"
                };
                let key = event.keystroke.key.as_str();
                let to = match key {
                    key if key == back => step(&keys, chosen, -1),
                    key if key == ahead => step(&keys, chosen, 1),
                    "home" => step(&keys, keys.len() - 1, 1),
                    "end" => step(&keys, 0, -1),
                    _ => return,
                };
                cx.stop_propagation();
                if to != chosen {
                    key_pick(to, window, cx);
                }
            })
            .child(measure_origin(state, axis))
            .children(indicator)
            .children(items);
        let panel = self.panel.map(|panel| {
            div().flex_1().min_w_0().child(panel).with_animation(
                ("tab-panel", turn),
                Animation::new(motion::duration(motion::BASE, cx))
                    .with_easing(motion::ease_out_cubic),
                move |panel, t| if turn == 0 { panel } else { panel.opacity(t) },
            )
        });
        let frame = div().id(self.id).flex().gap_4();
        match placement {
            TabPlacement::Top => frame.flex_col().child(strip).children(panel),
            TabPlacement::Bottom => frame.flex_col().children(panel).child(strip),
            TabPlacement::Left => frame.child(strip).children(panel),
        }
    }
}
