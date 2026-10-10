use gpui::{
    App, ClickEvent, ElementId, FocusHandle, FontWeight, Hsla, IntoElement, MouseButton,
    Refineable, RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Window,
    div, prelude::*, transparent_black,
};

use crate::{
    i18n,
    motion::Spinner,
    primitives::{FocusRing, Icon, IconName, Tooltip},
    theme::{ActiveTheme, IconSize, Mix, Palette, Platform, Radius, Size, TextSize},
    typography::keys::{keystroke, keystroke_labels},
};

pub(crate) type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Colors a caller supplies for a control whose state is not one of the named tones — an
/// activity-rail button whose selected fill comes from the host theme, say.
///
/// Mirrors `gpui-component`'s `ButtonCustomVariant`: transparent background and the theme
/// foreground unless told otherwise, so it starts as a quiet ghost and the caller paints it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonCustomVariant {
    color: Hsla,
    foreground: Hsla,
    hover: Hsla,
    active: Hsla,
    shadow: bool,
}

impl ButtonCustomVariant {
    pub fn new(cx: &App) -> Self {
        Self {
            color: transparent_black(),
            foreground: cx.theme().colors.fg,
            hover: transparent_black(),
            active: transparent_black(),
            shadow: false,
        }
    }

    /// The resting background; transparent by default.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = color;
        self
    }

    /// The text and icon color, when it differs from the theme foreground.
    pub fn foreground(mut self, color: Hsla) -> Self {
        self.foreground = color;
        self
    }

    /// The background under the pointer; transparent by default.
    pub fn hover(mut self, color: Hsla) -> Self {
        self.hover = color;
        self
    }

    /// The background while pressed, and at rest when the button is selected.
    pub fn active(mut self, color: Hsla) -> Self {
        self.active = color;
        self
    }

    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ButtonVariant {
    Primary,
    #[default]
    Secondary,
    Outline,
    Ghost,
    Subtle,
    /// Confirms a gain, such as a purchase; green.
    Success,
    Danger,
    Link,
    /// Caller-supplied colors; see [`ButtonCustomVariant`].
    Custom(ButtonCustomVariant),
}

pub(crate) struct Tone {
    pub bg: Hsla,
    pub hover: Hsla,
    pub pressed: Hsla,
    pub fg: Hsla,
    pub border: Hsla,
}

pub(crate) fn tone(variant: ButtonVariant, colors: &Palette) -> Tone {
    let clear = transparent_black();
    match variant {
        ButtonVariant::Primary => Tone {
            bg: colors.accent,
            hover: colors.accent_hover,
            pressed: colors.accent_hover.mix(&colors.on_accent, 0.12),
            fg: colors.on_accent,
            border: clear,
        },
        ButtonVariant::Secondary => Tone {
            bg: colors.surface,
            hover: colors.hover,
            pressed: colors.active,
            fg: colors.fg,
            border: colors.border,
        },
        ButtonVariant::Outline => Tone {
            bg: clear,
            hover: colors.hover,
            pressed: colors.active,
            fg: colors.fg,
            border: colors.border_strong,
        },
        ButtonVariant::Ghost => Tone {
            bg: clear,
            hover: colors.hover,
            pressed: colors.active,
            fg: colors.fg,
            border: clear,
        },
        ButtonVariant::Subtle => Tone {
            bg: colors.hover,
            hover: colors.active,
            pressed: colors.active.mix(&colors.border_strong, 0.5),
            fg: colors.fg,
            border: clear,
        },
        ButtonVariant::Success => Tone {
            bg: colors.success,
            hover: colors.success.mix(&colors.fg, 0.14),
            pressed: colors.success.mix(&colors.fg, 0.24),
            fg: colors.on_accent,
            border: clear,
        },
        ButtonVariant::Danger => Tone {
            bg: colors.danger,
            hover: colors.danger.mix(&colors.fg, 0.14),
            pressed: colors.danger.mix(&colors.fg, 0.24),
            fg: colors.on_accent,
            border: clear,
        },
        ButtonVariant::Link => Tone {
            bg: clear,
            hover: clear,
            pressed: clear,
            fg: colors.link,
            border: clear,
        },
        ButtonVariant::Custom(style) => Tone {
            bg: style.color,
            hover: if style.hover == clear { style.color } else { style.hover },
            pressed: style.active,
            fg: style.foreground,
            border: clear,
        },
    }
}

/// The background a selected button rests on, per variant.
fn selected_background(variant: ButtonVariant, colors: &Palette) -> Hsla {
    match variant {
        ButtonVariant::Custom(style) => style.active,
        other => tone(other, colors).pressed,
    }
}

pub(crate) fn label_size(size: Size) -> (TextSize, IconSize) {
    match size {
        // An explicit height maps onto the nearest named tier.
        Size::Size(height) => label_size(if f32::from(height) < 26.0 {
            Size::Small
        } else if f32::from(height) < 30.0 {
            Size::Medium
        } else {
            Size::Large
        }),
        Size::XSmall => (TextSize::Sm, IconSize::Xs),
        Size::Small => (TextSize::Sm, IconSize::Xs),
        Size::Medium => (TextSize::Base, IconSize::Sm),
        Size::Large => (TextSize::Md, IconSize::Md),
    }
}

/// Where a button sits inside a `ButtonGroup`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    First,
    Middle,
    Last,
}

/// Fails on a keystroke gpui cannot read, when a builder takes it.
pub(crate) fn keystroke_of(source: &str) {
    keystroke(source, Platform::current());
}

/// A keystroke in gpui key syntax as `platform` spells it: `⌘S`, or `Ctrl+S`.
pub(crate) fn shortcut_text(source: &str, platform: Platform) -> String {
    let glue = if platform == Platform::Mac { "" } else { "+" };
    keystroke_labels(&keystroke(source, platform), platform).join(glue)
}

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    /// The name assistive tech announces when it is not the visible label — an icon-only button
    /// otherwise has no name at all.
    aria_label: Option<SharedString>,
    icon: Option<IconName>,
    /// 以**元素**形式给出的前置图标（宿主自己的图标体系，如 Lithe 的 IDEA 风格单色图标）。
    icon_element: Option<gpui::AnyElement>,
    trailing_icon: Option<IconName>,
    shortcut: Option<SharedString>,
    variant: ButtonVariant,
    size: Size,
    disabled: bool,
    loading: bool,
    /// Paints the resting background as a chosen state (an active rail item).
    selected: bool,
    /// Announced to assistive tech as the pressed/selected state; painted by [`Self::selected`].
    toggled: bool,
    /// The caller's own style, refined over the button's last.
    style: StyleRefinement,
    full_width: bool,
    focus: Option<FocusHandle>,
    /// `false` keeps it out of the Tab order; unset leaves the default (a Tab stop).
    tab_stop: Option<bool>,
    tooltip: Option<SharedString>,
    /// A stable name the UI test lane looks the button up by; see gpui's `debug_selector`.
    debug_name: Option<Box<dyn Fn() -> String + 'static>>,
    pub(crate) slot: Option<Slot>,
    on_click: Option<ClickHandler>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: SharedString::default(),
            aria_label: None,
            icon: None,
            icon_element: None,
            trailing_icon: None,
            shortcut: None,
            variant: ButtonVariant::default(),
            size: Size::default(),
            disabled: false,
            loading: false,
            selected: false,
            toggled: false,
            style: StyleRefinement::default(),
            full_width: false,
            focus: None,
            tab_stop: None,
            tooltip: None,
            debug_name: None,
            slot: None,
            on_click: None,
        }
    }

    /// The button's text. A builder rather than a `new` argument, matching the
    /// gpui-kit shape (`Button::new(id).label("Save")`).
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    /// The name assistive tech announces, when it differs from the visible label — the case of an
    /// icon-only button, which draws no text to name itself with. Prefer it over [`Self::label`]:
    /// a label also paints.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Whether Tab reaches it. `false` takes it out of the tab order, for a control reached by
    /// pointer or by the focus an owner restores itself.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = Some(tab_stop);
        self
    }

    /// The name assistive tech announces: the explicit [`Self::aria_label`], else the visible label.
    pub(crate) fn accessible_name(&self) -> SharedString {
        self.aria_label
            .clone()
            .unwrap_or_else(|| self.label.clone())
    }

    /// A hover tooltip; names an icon-only button that has no visible label.
    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Colors the button from a caller-supplied set; see [`ButtonCustomVariant`].
    pub fn custom(mut self, style: ButtonCustomVariant) -> Self {
        self.variant = ButtonVariant::Custom(style);
        self
    }

    /// Paints the resting background as a chosen state — a rail item that is the current one.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// The pressed/selected state assistive tech announces. Pairs with [`Self::selected`].
    pub fn toggled(mut self, toggled: bool) -> Self {
        self.toggled = toggled;
        self
    }

    pub fn primary(self) -> Self {
        self.variant(ButtonVariant::Primary)
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// 前置图标，但以**元素**传入——给"图标来自宿主自己的图标体系"的场景用。
    ///
    /// 上游 `gpui-base` 的 `Button::icon` 收 `impl Into<ButtonIcon>`，一个方法同时覆盖
    /// 库内图标与外部元素；我们的 [`Button::icon`] 只收 [`IconName`]，所以宿主图标元素走这里。
    /// 两者都给时 `icon_element` 优先。
    pub fn icon_element(mut self, icon: impl IntoElement) -> Self {
        self.icon_element = Some(icon.into_any_element());
        self
    }

    pub fn trailing_icon(mut self, icon: IconName) -> Self {
        self.trailing_icon = Some(icon);
        self
    }

    /// Shows the keystroke that also runs it, e.g. `"cmd-s"`. Panics on a bad keystroke.
    pub fn shortcut(mut self, keystroke: &str) -> Self {
        keystroke_of(keystroke);
        self.shortcut = Some(SharedString::from(keystroke.to_string()));
        self
    }

    /// Takes focus through the owner's handle, one that outlives a mode where the button changes; a Tab stop when built with `tab_stop`.
    pub fn focus_handle(mut self, handle: &FocusHandle) -> Self {
        self.focus = Some(handle.clone());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Spins in place of its content, at the same width, and ignores presses. Tab still reaches it.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// A stable name the UI test lane finds it by (`debug_bounds`); see gpui's `debug_selector`.
    pub fn debug_selector(mut self, selector: impl Fn() -> String + 'static) -> Self {
        self.debug_name = Some(Box::new(selector));
        self
    }
}

impl RenderOnce for Button {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let mut tone = tone(self.variant, &theme.colors);
        if self.selected {
            let resting = selected_background(self.variant, &theme.colors);
            tone.bg = resting;
            tone.hover = resting;
            tone.pressed = resting;
        }
        let (text, icon_size) = label_size(self.size);
        let link = self.variant == ButtonVariant::Link;
        // The caller's style refines the button last.
        let style = self.style.clone();
        // An icon with no label is a square button: the shape Button used to draw.
        let icon_element = self.icon_element.take();
        let debug_name = self.debug_name.take();
        let square = self.label.is_empty() && (self.icon.is_some() || icon_element.is_some());
        let tooltip = self.tooltip.clone();
        let icon = |name| Icon::new(name).size(icon_size).color(tone.fg);
        let radius = theme.radius(Radius::Md);
        let platform = cx.theme().platform;
        // Before the label moves into `parts` below.
        let name = self.accessible_name();
        let hint = self.shortcut.map(|source| shortcut_text(&source, platform));
        let spinner = (self.id.clone(), "spinner");
        let tab_stop = self.tab_stop;
        let parts = div()
            .flex()
            .items_center()
            .gap_1p5()
            .when_some(icon_element, |el, icon| el.child(icon))
            .when_some(self.icon, |el, name| el.child(icon(name)))
            .when(!self.label.is_empty(), |el| el.child(self.label))
            .when_some(self.trailing_icon, |el, name| el.child(icon(name)))
            .when_some(hint, |el, hint| {
                el.child(
                    div()
                        .pl_1()
                        .font_weight(FontWeight::NORMAL)
                        .text_color(tone.fg.opacity(0.55))
                        .child(hint),
                )
            });

        let mut el = div()
            .id(self.id)
            .when_some(debug_name, |el, selector| el.debug_selector(selector))
            .role(Role::Button)
            .when(!name.is_empty(), |el| el.aria_label(name))
            .when(self.toggled, |el| el.aria_selected(true))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap_1p5()
            .when(square, |el| el.size(theme.control_height(self.size)))
            .when(!square, |el| el.h(theme.control_height(self.size)))
            .when(!square && !link, |el| {
                el.px(theme.control_padding(self.size))
            })
            .border_1()
            .border_color(tone.border)
            .map(|el| match self.slot {
                None => el.rounded(radius),
                Some(Slot::First) => el.rounded_l(radius),
                Some(Slot::Middle) => el.border_l_0(),
                Some(Slot::Last) => el.rounded_r(radius).border_l_0(),
            })
            .text_size(theme.text_size(text))
            .font_weight(FontWeight::MEDIUM)
            .text_color(tone.fg)
            .bg(tone.bg)
            .when_some(tooltip, |el, text| el.tooltip(Tooltip::text(text)))
            .when(self.full_width, |el| el.w_full())
            .map(|el| {
                if !self.loading {
                    return el.child(parts);
                }
                el.relative().child(parts.invisible()).child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Spinner::new(spinner).size(icon_size).color(tone.fg)),
                )
            })
            .map(|el| {
                if self.disabled {
                    return el
                        .aria_description(i18n::text(cx, "state.unavailable", &[]))
                        .opacity(0.45)
                        .cursor_not_allowed();
                }
                let el = match &self.focus {
                    Some(handle) => el.track_focus(handle),
                    None => el.tab_index(0),
                };
                let el = match tab_stop {
                    Some(stop) => el.tab_stop(stop),
                    None => el,
                };
                if self.loading {
                    return el
                        .focus_ring(cx)
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default());
                }
                el.cursor_pointer()
                    .hover(|style| {
                        let style = style.bg(tone.hover);
                        if link { style.underline() } else { style }
                    })
                    .active(|style| style.bg(tone.pressed))
                    .focus_ring(cx)
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .when_some(self.on_click, |el, handler| el.on_click(handler))
            });
        el.style().refine(&style);
        el
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl crate::theme::Sizable for Button {
    fn with_size(self, size: impl Into<crate::theme::Size>) -> Self {
        self.size(size.into())
    }
}
