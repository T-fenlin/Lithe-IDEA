use gpui_kit::base::v_flex;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::{ActiveTheme as _, StyledExt as _};
use gpui_kit::{AnyElement, App, IntoElement, ParentElement as _, SharedString, Styled as _};

pub const UNAVAILABLE_ICON_SIZE_SPEC: f32 = 28.;

pub const UNAVAILABLE_DESCRIPTION_OPACITY: f32 = 0.72;

pub fn unavailable(
    icon: Option<impl IntoElement>,
    title: impl Into<SharedString>,
    description: Option<SharedString>,
    extra: Option<AnyElement>,
    cx: &App,
) -> AnyElement {
    let mut header = EmptyHeader::new();
    if let Some(icon) = icon {
        header = header.media(EmptyMedia::new().child(icon));
    }
    header = header.title(
        EmptyTitle::new()
            .text_sm()
            .font_semibold()
            .child(title.into()),
    );
    if let Some(description) = description {
        header = header.description(
            EmptyDescription::new()
                .text_xs()
                .opacity(UNAVAILABLE_DESCRIPTION_OPACITY)
                .child(description),
        );
    }

    v_flex()
        .w_full()
        .flex_1()
        .min_h_0()
        .child(
            Empty::new()
                .border_color(cx.theme().transparent)
                .gap_2()
                .p_6()
                .text_center()
                .header(header),
        )
        .children(extra)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_specs_match_the_macos_source() {
        assert_eq!(UNAVAILABLE_ICON_SIZE_SPEC, 28.);
        assert_eq!(UNAVAILABLE_DESCRIPTION_OPACITY, 0.72);
    }
}
