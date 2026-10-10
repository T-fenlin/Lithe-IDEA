use gpui::{
    AnyElement, App, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::*,
};

use super::InputState;
use crate::{
    buttons::Button,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, Size, TextSize},
};

/// A text field's frame: border, focus, and room for things before and after.
#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    size: Size,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    clearable: bool,
    invalid: bool,
    /// The name assistive tech announces; the placeholder stands in when it is unset.
    aria_label: Option<SharedString>,
}

impl Input {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::default(),
            prefix: None,
            suffix: None,
            clearable: false,
            invalid: false,
            aria_label: None,
        }
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Adds a clear button while there is text.
    pub fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }

    /// Draws the frame in the danger tone.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// The name assistive tech announces for the field. A placeholder is a hint, not a name, so a
    /// field whose purpose is not obvious from the placeholder alone should set this.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// The name assistive tech announces: the explicit [`Self::aria_label`], else the placeholder.
    pub(crate) fn spoken_name(&self, cx: &App) -> SharedString {
        self.aria_label
            .clone()
            .unwrap_or_else(|| self.state.read(cx).placeholder_text().clone())
    }
}

pub(crate) fn text_size(size: Size) -> TextSize {
    match size {
        // An explicit height maps onto the nearest named tier.
        Size::Size(height) => text_size(if f32::from(height) < 26.0 {
            Size::Small
        } else if f32::from(height) < 30.0 {
            Size::Medium
        } else {
            Size::Large
        }),
        Size::XSmall => TextSize::Sm,
        Size::Small => TextSize::Sm,
        Size::Medium => TextSize::Base,
        Size::Large => TextSize::Md,
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let input = self.state.read(cx);
        let focus = input.focus().clone();
        let (focused, empty, multi, disabled) = (
            focus.is_focused(window),
            input.is_empty(),
            input.is_multi_line(),
            input.is_disabled(),
        );
        let theme = cx.theme();
        let colors = &theme.colors;
        let border = if self.invalid {
            colors.danger
        } else if focused {
            colors.focus
        } else {
            colors.border_strong
        };
        let (state, clear_id) = (self.state.clone(), self.state.entity_id());
        let name = self.spoken_name(cx);
        div()
            .id(("input-frame", clear_id))
            .debug_selector(|| "input-root".into())
            .when(!name.is_empty(), |frame| frame.aria_label(name))
            .flex()
            .gap_2()
            .px(theme.control_padding(self.size))
            .map(|frame| {
                if multi {
                    frame.items_start().py_1p5()
                } else {
                    frame.items_center().h(theme.control_height(self.size))
                }
            })
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(border)
            .bg(if disabled {
                colors.sunken
            } else {
                colors.surface
            })
            .text_size(theme.text_size(text_size(self.size)))
            .text_color(if disabled {
                colors.fg_disabled
            } else {
                colors.fg
            })
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                window.focus(&focus, cx)
            })
            .children(self.prefix)
            .child(
                div()
                    .debug_selector(|| "input-text".into())
                    .flex_1()
                    .min_w_0()
                    .child(self.state)
                    // An empty strut keeps the frame's inset to type in.
                    .child(div().px(theme.control_padding(self.size)).border_x_1()),
            )
            .when(self.clearable && !empty && !disabled, |frame| {
                frame.child(
                    Button::new(("input-clear", clear_id))
                        .icon(IconName::CircleX)
                        .size(Size::Small)
                        .tooltip("Clear")
                        .on_click(move |_, window, cx| {
                            state.update(cx, |input, cx| input.set_text("", cx));
                            window.focus(&state.read(cx).focus().clone(), cx);
                        }),
                )
            })
            .children(self.suffix)
    }
}

/// A masked field with a lock before it and an eye that shows the text.
#[derive(IntoElement)]
pub struct PasswordInput {
    state: Entity<InputState>,
    size: Size,
}

impl PasswordInput {
    /// Build the state with `InputState::masked`.
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::default(),
        }
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for PasswordInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let masked = self.state.read(cx).is_masked();
        let (toggle, subtle) = (self.state.clone(), cx.theme().colors.fg_subtle);
        Input::new(&self.state)
            .size(self.size)
            .prefix(Icon::new(IconName::Lock).size(IconSize::Sm).color(subtle))
            .suffix(
                Button::new(("password-eye", self.state.entity_id()))
                    .icon(if masked {
                        IconName::Eye
                    } else {
                        IconName::EyeOff
                    })
                    .size(Size::Small)
                    .tooltip(if masked {
                        "Show password"
                    } else {
                        "Hide password"
                    })
                    .on_click(move |_, _, cx| {
                        toggle.update(cx, |input, cx| {
                            let masked = input.is_masked();
                            log::info!(
                                "password input: {}",
                                if masked { "shown" } else { "hidden" }
                            );
                            input.set_masked(!masked, cx);
                        })
                    }),
            )
    }
}

impl crate::theme::Sizable for Input {
    fn with_size(self, size: impl Into<crate::theme::Size>) -> Self {
        self.size(size.into())
    }
}

impl crate::theme::Sizable for PasswordInput {
    fn with_size(self, size: impl Into<crate::theme::Size>) -> Self {
        self.size(size.into())
    }
}
