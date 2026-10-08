use gpui::{AnyElement, App, IntoElement as _, Length, SharedString, Styled as _, Svg, div, svg};
use lithe_ui::primitives::Icon;
use lithe_ui::theme::ActiveTheme as _;

pub mod file_icon;
pub mod idea;

/// 我们自己图标集里的一个图标。
///
/// **口径**（维护者，2026-10-08，见
/// `.agents/notes/proposed/architecture/2026-10-08-unified-gpui-frontend-selection-and-layer-boundaries.md`）：
/// 应用图标一律取自**本仓库自己的**图标集 `rust/lithe-gpui/assets/icons/idea/**`（302 个文件）；
/// 我们自己没有那张图就**留空、不画**，不许拿 `gpui-kit` / `lithe-ui` 的图标顶替。
///
/// **这个类型把口径变成类型事实**：它只装资源路径 `&'static str`，装不下
/// `lithe_ui::primitives::IconName` 这类组件库图标名 —— 所以「用库图标」不再可表达。
///
/// **查找规则**：路径只可能来自 `assets/icons/idea/**`，取法有三种 ——
/// [`idea::*`](idea) 常量（用 [`AppIcon::from_idea`] 取）、[`file_icon`] 的主题清单，
/// 以及 [`file_icon::fallback_icon`] 那张显式表。
///
/// **「我们没有」不由本类型表达**，而由 `Option<AppIcon>` 的 `None` 表达：
/// 见 [`FileIcon::fallback`] 与 [`file_icon::fallback_icon`]。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppIcon {
    /// 深色与浅色用同一张图。
    Same(&'static str),
    /// 深色与浅色各一张图（[`idea::IdeaIcon`] 的常态）。
    Themed {
        light: &'static str,
        dark: &'static str,
    },
}

impl AppIcon {
    /// 深浅色统一用 `light` 那一张。
    #[must_use]
    pub const fn same(light: &'static str) -> Self {
        Self::Same(light)
    }

    /// 深浅色各一张；路径必须都指向我们自己的资源。
    #[must_use]
    pub const fn themed(light: &'static str, dark: &'static str) -> Self {
        Self::Themed { light, dark }
    }

    /// 由 [`idea::*`](idea) 常量取，深浅变体跟着常量走。
    #[must_use]
    pub const fn from_idea(icon: &'static idea::IdeaIcon) -> Self {
        Self::Themed {
            light: icon.light,
            dark: icon.dark,
        }
    }

    /// 当前主题该画哪张图。
    #[must_use]
    pub const fn path(&self, is_dark: bool) -> &'static str {
        match self {
            Self::Same(path) => path,
            Self::Themed { light, dark } => {
                if is_dark {
                    *dark
                } else {
                    *light
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileIcon {
    /// 我们自己的主题清单（`assets/icons/idea/extension.json`）按文件名命中的图；
    /// 构造时已按调用点的主题解析成路径。`None` = 清单里没有这一条。
    pub themed: Option<&'static str>,
    /// 清单没命中时的兜底：**我们自己的**图标（见 [`file_icon::fallback_icon`]）。
    /// `None` = 我们自己的图标集里也没有 ⇒ 渲染时**留空**（占住同一个图标位，不画图形）。
    pub fallback: Option<AppIcon>,
}

impl FileIcon {
    /// 只有兜底图、没有清单主题图；`None` 就是「我们没有这张图 ⇒ 留空」。
    #[must_use]
    pub fn own(fallback: Option<AppIcon>) -> Self {
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
        let Some(icon) = self.fallback else {
            // 我们自己的图标集里没有这张图：留空——占住同一个图标位（行不跳），但不画任何图形。
            return div().flex_none().w(side).h(side).into_any_element();
        };
        let theme = cx.theme();
        svg()
            .path(SharedString::from(icon.path(theme.is_dark())))
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
