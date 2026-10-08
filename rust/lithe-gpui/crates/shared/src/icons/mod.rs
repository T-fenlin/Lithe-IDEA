use gpui_kit::assets::IconName;
use gpui_kit::component::Theme;
use gpui::{AnyElement, App, IntoElement as _, Length, SharedString, Styled as _, Svg, svg};

pub mod file_icon;
pub mod idea;

#[derive(Clone, Debug)]
pub struct FileIcon {
    pub themed: Option<&'static str>,
    pub fallback: IconName,
}

impl FileIcon {
    #[must_use]
    pub fn lucide(fallback: IconName) -> Self {
        Self {
            themed: None,
            fallback,
        }
    }

    #[must_use]
    pub fn with_themed(mut self, themed: Option<&'static str>) -> Self {
        self.themed = themed;
        self
    }

    #[must_use]
    pub fn with_theme_icon(self, name: &str, cx: &App) -> Self {
        let themed = file_icon::theme_icon_path(name, false, false, cx);
        self.with_themed(themed)
    }

    #[must_use]
    pub fn with_theme_folder(self, name: &str, expanded: bool, cx: &App) -> Self {
        let themed = file_icon::theme_icon_path(name, true, expanded, cx);
        self.with_themed(themed)
    }

    #[must_use]
    pub fn render(&self, side: impl Into<Length>, cx: &App) -> AnyElement {
        let side: Length = side.into();
        if let Some(themed) = self.themed {
            if let Some(element) = file_icon::theme_file_icon(themed, side, cx) {
                return element;
            }
        }
        let theme = Theme::global(cx);
        svg()
            .path(SharedString::from(self.fallback.path()))
            .flex_shrink_0()
            .text_color(theme.foreground)
            .w(side)
            .h(side)
            .into_any_element()
    }
}

pub fn idea_icon_svg(icon: &idea::IdeaIcon, cx: &App) -> Svg {
    let theme = Theme::global(cx);
    let path: SharedString = icon.path(theme.is_dark()).into();
    svg()
        .path(path)
        .flex_shrink_0()
        .text_color(theme.foreground)
        .size_4()
}

pub fn idea_icon_image(icon: &idea::IdeaIcon, cx: &App, side: impl Into<Length>) -> AnyElement {
    let side: Length = side.into();
    let is_dark = Theme::global(cx).is_dark();
    match file_icon::theme_file_icon(icon.path(is_dark), side, cx) {
        Some(element) => element,
        None => idea_icon_svg_px(icon, cx, side).into_any_element(),
    }
}

#[must_use]
pub fn idea_icon_mono(icon: &idea::IdeaIcon, is_dark: bool) -> gpui_kit::component::Icon {
    let path: SharedString = icon.path(is_dark).into();
    gpui_kit::component::Icon::empty().path(path)
}

pub fn idea_icon_svg_px(icon: &idea::IdeaIcon, cx: &App, side: impl Into<Length>) -> Svg {
    let theme = Theme::global(cx);
    let path: SharedString = icon.path(theme.is_dark()).into();
    let side: Length = side.into();
    svg()
        .path(path)
        .flex_shrink_0()
        .text_color(theme.foreground)
        .w(side)
        .h(side)
}
