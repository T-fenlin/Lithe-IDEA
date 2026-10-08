use gpui::{AnyElement, App, IntoElement as _, Length, SharedString, Styled as _, Svg, svg};
use lithe_ui::primitives::{Icon, IconName};
use lithe_ui::theme::ActiveTheme as _;

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
        let theme = cx.theme();
        svg()
            .path(SharedString::from(self.fallback.path()))
            .flex_shrink_0()
            .text_color(theme.colors.fg)
            .w(side)
            .h(side)
            .into_any_element()
    }
}

pub fn idea_icon_svg(icon: &idea::IdeaIcon, cx: &App) -> Svg {
    let theme = cx.theme();
    let path: SharedString = icon.path(theme.is_dark()).into();
    svg()
        .path(path)
        .flex_shrink_0()
        .text_color(theme.colors.fg)
        .size_4()
}

pub fn idea_icon_image(icon: &idea::IdeaIcon, cx: &App, side: impl Into<Length>) -> AnyElement {
    let side: Length = side.into();
    let is_dark = cx.theme().is_dark();
    match file_icon::theme_file_icon(icon.path(is_dark), side, cx) {
        Some(element) => element,
        None => idea_icon_svg_px(icon, cx, side).into_any_element(),
    }
}

/// 单色的 IDEA 图标元素（组件库的 [`Icon`]），给按钮、菜单项这类收元素的地方用。
///
/// 上游那条 `Icon::empty().path(path)` 在组件库里就是 [`Icon::from_path`]：同一件事
/// （空图标 + 一个资源路径），只是库这一版还会在渲染时确认路径**确实在宿主的资源源里**
/// ——`icons/idea/**` 全部随 `LitheAssets` 内嵌（见 `app::assets` 的同名测试）。
#[must_use]
pub fn idea_icon_mono(icon: &idea::IdeaIcon, is_dark: bool) -> Icon {
    let path: SharedString = icon.path(is_dark).into();
    Icon::from_path(path)
}

pub fn idea_icon_svg_px(icon: &idea::IdeaIcon, cx: &App, side: impl Into<Length>) -> Svg {
    let theme = cx.theme();
    let path: SharedString = icon.path(theme.is_dark()).into();
    let side: Length = side.into();
    svg()
        .path(path)
        .flex_shrink_0()
        .text_color(theme.colors.fg)
        .w(side)
        .h(side)
}
