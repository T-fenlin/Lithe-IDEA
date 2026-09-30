//! IntelliJ `expui` 图标的**资源路径清单**（`lithe-gpui/assets/ui-icons/idea/**`）+ 渲染 helper。
//!
//! 本模块回答一个问题："`lithe-gpui/assets/` 里这套从 Windows 前端搬进来的图标，在 gpui 里怎么引用？"
//!
//! ## 项目应该怎么引用这些图标（四步）
//!
//! 1. **注册**（App Shell 做一次）：`gpui_kit::application().with_assets(LitheAssets)`。
//!    `LitheAssets`（`lithe-gpui-app/src/assets.rs`）用 `rust-embed` 内嵌整个 `lithe-gpui/assets/**`，
//!    未命中时回落 `gpui_kit::assets::AllAssets` 的 Lucide 字形。`with_assets` 只收**一个**
//!    资源源（`gpui-pre-0.3.6/src/app.rs:198-206`），所以两者必须由包装层组合，不能注册两次。
//! 2. **清单**（本模块）：[`idea::ALL`] 是全部 79 个图标（每个带 `light` / `dark` 两个资源路径），
//!    [`idea::ALIASES`] 保留旧前端 TS 清单里的别名显示名。
//!    文件 `src/icons/idea.rs` 由 `lithe-gpui/tools/generate-idea-icons.mjs` **生成**，不要手改。
//!    **文件类型图标走另一条路**：[`file_icon`] 按文件名 / 后缀查
//!    `lithe-gpui/assets/icons/idea/extension.json`，再用 [`FileIcon::render`] 画出来。
//! 3. **helper**：[`idea_icon_svg`] 按当前主题明暗挑路径，并设好 16×16 的尺寸与前景色。
//!    **调用点只写图标名**，不要自己再调 `.size_4()` / `.text_color(..)`（理由见该函数的文档）。
//! 4. **调用点**：`.child(idea_icon_svg(&idea::GEAR_ICON, cx))`。
//!
//! ```ignore
//! use lithe_gpui_shared::icons::{idea, idea_icon_svg};
//!
//! // 设置对话框头部（`crates/settings/src/dialog.rs` 的真实用法）：
//! h_flex()
//!     .text_color(cx.theme().foreground)
//!     .child(idea_icon_svg(&idea::GEAR_ICON, cx))
//!     .child(div().child(tr("lithe.workbench.settings")))
//! ```
//!
//! ## 为什么 helper 住在 `shared` 而不是某个 feature
//!
//! 依赖方向是 `app → settings → shared`、`workbench → explorer → shared`（见《编码指南》
//! 「依赖方向」）：**feature 不能依赖 app**，所以 helper 放 `crates/app/src/assets.rs` 的话，
//! `settings` / `explorer` / `workbench` 的调用点都拿不到它。`shared` 是唯一同时满足
//! "被所有 feature 依赖"与"已经依赖 gpui-kit"的位置。
//!
//! 本模块只多用了 `gpui_kit` 的三个公开项（`App` / `Svg` / `svg()` 与
//! `component::Theme`），没有引入新依赖 —— `shared` 的 `Cargo.toml` 本来就依赖 `gpui-kit`。
//! 生成的 [`idea`] 子模块本身**只有 `&'static str` 与 `bool`**，连 gpui-kit 都用不到。
//!
//! ## 还没接的部分
//!
//! - `lithe-gpui/assets/icons/**`（4 套文件类型图标包，1 027 个 SVG）走**另一条路**
//!   （"按文件名 / 后缀查 `extension.json` 再取 SVG"，不是一个名字一张图），实现在
//!   [`file_icon`] 子模块；`explorer` / `editor` 的 `icon_for_file` 已经接上它。
//! - 因此 [`idea`] 子模块只覆盖 `ui-icons/idea/**` 的 157 个文件（79 个图标 × 明/暗变体）。

use gpui_kit::assets::IconName;
use gpui_kit::component::Theme;
use gpui_kit::{AnyElement, App, IntoElement as _, Length, SharedString, Styled as _, Svg, svg};

pub mod file_icon;
pub mod idea;

/// 一个"文件类型图标"在**用户界面里实际怎么画**的完整答案。
///
/// ## 为什么要这个类型（而不是只返回 `IconName`）
///
/// 文件类型图标有**两个真源**，而且优先关系是固定的：
///
/// 1. **`lithe-gpui/assets/icons/idea/**`** —— 真机默认图标主题（`idea-icons`）的文件类型美术。
///    它按文件名 / 后缀查 `extension.json`，是**彩色**的（见 [`file_icon`]）。
/// 2. **全量 Lucide 目录**（`gpui_kit::assets::IconName`，`icons/*.svg`）—— 主题包里**没有**
///    对应条目时的回落。老的 `icon_for_file` 就是这一层。
///
/// 把两者放进一个结构体，是为了让"**回落这条路径没坏**"成为类型上的事实：每个调用点拿到的
/// 都是一个"有真源就用真源、没有就用 Lucide"的完整答案，而不是"先查真源、查不到就当作显示
/// 空白"或"到处 `unwrap_or_default`"。
///
/// ## 字段
///
/// - `themed`：主题包里的资源路径（相对 `lithe-gpui/assets/`，可直接交给
///   [`file_icon::theme_file_icon`]）。`None` = 这个文件 / 目录在主题包里没有专属图标。
/// - `fallback`：Lucide 字形，**永远有值**（`IconName` 是全量目录的封闭枚举）。
#[derive(Clone, Debug)]
pub struct FileIcon {
    /// 主题包（`icons/idea/**`）里的 SVG 资源路径；`None` 时用 `fallback`。
    pub themed: Option<&'static str>,
    /// 主题包没有对应条目时用的 Lucide 字形（`icons/*.svg`）。
    pub fallback: IconName,
}

impl FileIcon {
    /// 构造一个"主题包里没有专属条目"的答案（`themed = None`）。
    #[must_use]
    pub fn lucide(fallback: IconName) -> Self {
        Self {
            themed: None,
            fallback,
        }
    }

    /// 把查到的主题资源路径补进来（builder：`FileIcon::lucide(..).with_themed(opt)`）。
    #[must_use]
    pub fn with_themed(mut self, themed: Option<&'static str>) -> Self {
        self.themed = themed;
        self
    }

    /// 查一个**文件**在当前主题 + 当前明暗下的真源图标（查不到就保持 `None`）。
    ///
    /// 调用点一行就能拿到完整答案：
    ///
    /// ```ignore
    /// FileIcon::lucide(file_icon::lucide_fallback(name))
    ///     .with_theme_icon(name, cx)
    /// ```
    #[must_use]
    pub fn with_theme_icon(self, name: &str, cx: &App) -> Self {
        let themed = file_icon::theme_icon_path(name, false, false, cx);
        self.with_themed(themed)
    }

    /// 查一个**目录**在当前主题 + 当前明暗下的真源图标。
    #[must_use]
    pub fn with_theme_folder(self, name: &str, expanded: bool, cx: &App) -> Self {
        let themed = file_icon::theme_icon_path(name, true, expanded, cx);
        self.with_themed(themed)
    }

    /// 按这份答案画一个图标。
    ///
    /// - 有真源（`themed` 有值且内嵌资源里确实有那个文件）→ **彩色**的
    ///   [`file_icon::theme_file_icon`]；
    /// - 否则 → Lucide 字形，颜色照 `Icon` 的口径取 `theme.foreground`。
    ///
    /// 两个分支都按 `side` 设边长，所以同一个位置换源不会改变布局。
    #[must_use]
    pub fn render(&self, side: impl Into<Length>, cx: &App) -> AnyElement {
        // 先收敛成 `Length`（`Copy`）：两个分支都要用它，而 `impl Into<Length>` 本身不 `Copy`。
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

/// 按**当前主题明暗**挑一张 IntelliJ `expui` 图标，返回可直接 `.child(..)` 的元素。
///
/// 调用点只写图标名，不用自己判断主题，也不用设尺寸/颜色：
///
/// ```ignore
/// use lithe_gpui_shared::icons::{idea, idea_icon_svg};
///
/// h_flex().child(idea_icon_svg(&idea::GEAR_ICON, cx))
/// ```
///
/// ## 明暗从哪里读
///
/// `cx.theme()` 返回 `&gpui_kit::component::Theme`，明暗是 `ThemeMode`（`Light` / `Dark`）：
/// - `Theme::is_dark(&self) -> bool`（`gpui-component-0.6.6/src/theme/mod.rs:212-216`，
///   内部就是 `self.mode.is_dark()`）；
/// - `ThemeMode::is_dark(&self) -> bool`（同文件 `:707-711`）。
///
/// 本函数走的是 `Theme::global(cx).is_dark()`（`theme/mod.rs:196-200` 的全局读取）。
/// `Theme::change(..)`（同文件 `:261`）是改变它的唯一入口 —— `main.rs` 启动时切深色、
/// 设置「外观」页切主题、以及跟随系统外观时的回调都走它，所以调用点在 render 里读到的
/// 就是当前帧的真实明暗。
///
/// ## 颜色：用主题前景色，不用 SVG 自带的 `fill`
///
/// expui 的 SVG 自带 `fill="#6C707E"`（浅色）/ `fill="#CED0D6"`（深色），但 gpui 渲染
/// `svg().path(..)` 时走的是 **alpha mask**：`Window::paint_svg` 把它渲成 `MonochromeSprite`
/// （`gpui-pre-0.3.6/src/window.rs:4858-4866`），而 `svg_renderer.rs:231-249` 的
/// `render_alpha_mask` **只取 `pixel.alpha()`**。也就是说 **SVG 里的颜色会被丢掉**，
/// 实际颜色只来自元素自己的 `text_color`，而它又是 `Svg::paint` 的**必需前置条件**
/// （为 `None` 时整段不画，见下）。
///
/// 所以本函数显式设 `text_color(theme.foreground)` —— 与 `IconName` 的 `Icon` 同口径
/// （`Icon` 取的是 `window.text_style().color`），也让调用点不必依赖父容器的继承行为。
/// 这也解释了为什么"按主题换路径"仍然必要：78 对变体里有 **12 对几何形状也不同**（实测），
/// 纯换颜色在 gpui 这条渲染路径上根本不起作用。
///
/// ## 返回具体类型 `Svg`（不是 `impl IntoElement`）
///
/// gpui 的 `.size_4()` / `.text_color(..)` 是 `Styled` 上实现给**具体类型**的方法，
/// 返回 `impl IntoElement` 会让调用点的 `.size_4()` 报
/// `E0599: no method named size_4 found for opaque type impl IntoElement`（本项目实测）。
/// `Svg` 自己实现了 `IntoElement`（`gpui-pre-0.3.6/src/elements/svg.rs:192-198`），
/// 所以返回具体类型不损失任何调用方式。
///
/// ## 尺寸与颜色由本函数设，默认 16×16
///
/// 调用点只写图标名。要别的尺寸用 [`idea_icon_svg_px`]。
///
/// ## 为什么必须自己设尺寸与颜色（本项目实测踩过：白板）
///
/// 裸 `svg().path(..)` 直接放进 `h_flex` 里**画不出任何东西**，两个独立原因
/// （缺任何一个都是白板）：
///
/// 1. **没有 `text.color` 就整段不画**：`Svg::paint` 里三条绘制分支都被
///    `style.text.color` 的 `Option` 挡住（`gpui-pre-0.3.6/src/elements/svg.rs:149-186`），
///    为 `None` 时**什么也不做**。gpui-component 自己的 `Icon` 就是显式设 `text_color`
///    的（`gpui-component-0.6.6/src/icon.rs:179`，值取自 `window.text_style().color`）。
/// 2. **flex 子项会塌成 0 宽**：`Svg` 没有固有尺寸（`Svg::request_layout` 只是
///    `window.request_layout(style, None, cx)`，`svg.rs:84-98`），flex 子项默认
///    `flex-shrink: 1` + `basis: auto` 会把它压到 0。`Icon` 同样显式
///    `.flex_shrink_0()`（`icon.rs:178`）。
///
/// 本函数采用与 `Icon` 一致的三件套：`flex_shrink_0()` + `size_4()` + `text_color(..)`。
pub fn idea_icon_svg(icon: &idea::IdeaIcon, cx: &App) -> Svg {
    let theme = Theme::global(cx);
    let path: SharedString = icon.path(theme.is_dark()).into();
    svg()
        .path(path)
        .flex_shrink_0()
        .text_color(theme.foreground)
        .size_4()
}

/// 把一张 expui **UI 图标**画成**保留原色**的元素（`img()` 管线，与文件图标同一条）。
///
/// ## 与 [`idea_icon_svg`] 的区别：颜色保不保留
///
/// | helper | 管线 | 结果 |
/// | --- | --- | --- |
/// | [`idea_icon_svg`] / [`idea_icon_svg_px`] | `svg()` → alpha 遮罩 + 单一 `text_color` | **单色**（颜色 = 主题前景色） |
/// | 本函数 | `file_icon::theme_file_icon` → `img()` → 彩色 RGBA 栅格化 | **原色** |
///
/// 两条管线的源码依据与"为什么 `img()` 才是彩色那条"写在
/// [`file_icon::theme_file_icon`] 的文档里（`svg()` 把 SVG 当遮罩用，`img()` 走
/// `SvgRenderer::render_single_frame` 出彩色 RGBA）。这里**复用同一个实现**而不是再写一份：
/// 判别"资源在不在"的那一步（`theme_icon_bytes`）也正是本函数的前置条件。
///
/// ## 为什么 expui 的 UI 图标该走这条
///
/// 这批图标把**颜色烘进了文件**：157 个 SVG 里没有一个用 `currentColor`，色值全部硬编码，
/// 而且每个都有明暗两套变体（深色下另一组色值）。走单色管线 = 把这份信息抹掉、
/// 一律按主题前景色重画；走 `img()` = 原样保留作者定的那份颜色。
///
/// ⚠️ **但"保留原色"不等于"张张都是彩色的"**（这条 2026-09-30 核过一遍，别想当然）：
/// expui 分两族，颜色口径完全不同 ——
///
/// | 族 | 例子 | 颜色 |
/// | --- | --- | --- |
/// | **内容类**（对象 / 状态） | `general/warningDialog`（诊断）、`general/successDialog`（成功）、`fileTypes/**`（文件类型） | **真有颜色**：`#FFAF0F` 琥珀、`#55A76A` 绿、`#3574F0` 蓝… |
/// | **镶边类**（工具窗 / 轨道 / 状态栏那一族灰） | `general/listFiles`、`general/settings`、`toolwindows/notifications`（铃铛）、`toolwindows/**` | **灰阶、按主题两档**：浅色 `#6C707E`、深色 `#CED0D6` —— IDEA 自己就是这么画的（轨道图标是"安静的灰"，不是彩色） |
///
/// 所以换到 `img()` 之后：内容类那几张**会明显变彩**（诊断的警告三角从灰变成琥珀），
/// 镶边类那几张**看着几乎没变** —— 它们本来就该是灰的，现在只是换成 IDEA 自己那两档灰，
/// 而不是主题前景色。**要让某个镶边图标变彩，得换图标本身，换管线是换不出来的。**
///
/// ## 状态反馈不再靠图标变色
///
/// 彩色图标**不跟随 `text_color`**，所以悬停 / 选中的反馈只能靠**底色**（活动的轨道项仍由
/// `Button::selected` 出底色、悬停由 `ButtonCustomVariant::hover` 出底色）。
/// 这与 IDEA New UI 一致：轨道图标保持自己的颜色，高亮靠背景块。
/// Lucide 那些单色字形仍走 `.icon(..)`，照旧跟主题前景色。
///
/// ## 兜底
///
/// 资源取不到（例如只注册了部分资产源）时回落到 [`idea_icon_svg_px`] 的单色版本 ——
/// **不画空白**。所以调用点可以直接 `.child(idea_icon_image(..))`，不需要自己判空。
pub fn idea_icon_image(icon: &idea::IdeaIcon, cx: &App, side: impl Into<Length>) -> AnyElement {
    let side: Length = side.into();
    let is_dark = Theme::global(cx).is_dark();
    match file_icon::theme_file_icon(icon.path(is_dark), side, cx) {
        Some(element) => element,
        None => idea_icon_svg_px(icon, cx, side).into_any_element(),
    }
}

/// [`idea_icon_svg`] 的显式尺寸版本。
///
/// `side` 接受任何 `Into<Length>`：布局规范要求"用 rem 档位 helper，档位外的值走
/// `rems(P / 16.)`"，所以调用点传 `rems(1.)`（16）/ `rems(14. / 16.)`（14）/ `rems(0.75)`（12）
/// 都可以直接过，本函数不做换算也不四舍五入到档位。
///
/// ⚠️ 传 `rems(..)` 而不是 `px(..)`：`rems` 会在布局时按**当时的 rem 基准**解析，
/// 所以主题字号变化时图标跟着缩放（`px` 不会）。这也是"不要自己设 size"的另一面 ——
/// 尺寸单位的选择同样属于本 helper 的契约。
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
