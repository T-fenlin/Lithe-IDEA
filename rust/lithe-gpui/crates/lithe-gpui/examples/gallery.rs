//! 组件画廊（dev-only）：`cargo run -p lithe-gpui --example gallery`。
//!
//! 内嵌了一个最小 SVG 资产集合，演示 Icon 的注入式图标约定——库本体不内嵌
//! 图标，正式应用走 app crate 的 `LitheAssets`。第 1 章 Primitives 的全部
//! 变体在这里以 light/dark 双模式展示。

use std::borrow::Cow;

use gpui_kit::prelude::*;
use gpui_kit::{
    AssetSource, Bounds, Point, Render, SharedString, Window, WindowBounds, WindowOptions, div, px,
    size,
};

use lithe_gpui::{
    ActiveTheme as _, Avatar, Badge, BadgeVariant, Icon, Kbd, Label, Progress, Separator, Severity,
    Sizable as _, Spinner, Tag, Theme, ThemeMode, init, open_window,
};

/// 最小图标集合：手绘几何路径，专供 gallery。
static GALLERY_ASSETS: &[(&str, &str)] = &[
    (
        "icons/check.svg",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 12.5l5 5L20 6.5"/></svg>"##,
    ),
    (
        "icons/x.svg",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="2.5" stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18"/></svg>"##,
    ),
    (
        "icons/chevron-down.svg",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6"/></svg>"##,
    ),
    (
        "icons/plus.svg",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="2.5" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>"##,
    ),
];

struct GalleryAssets;

impl AssetSource for GalleryAssets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        Ok(GALLERY_ASSETS
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, svg)| Cow::Borrowed(svg.as_bytes())))
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        Ok(GALLERY_ASSETS
            .iter()
            .map(|(name, _)| SharedString::from(*name))
            .filter(|name| name.starts_with(path))
            .collect())
    }
}

fn main() {
    lithe_gpui::application()
        .with_assets(GalleryAssets)
        .run(|cx| {
            init(cx);
            let bounds = Bounds {
                origin: Point::default(),
                size: size(px(960.), px(760.)),
            };
            open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                cx,
                |_, cx| cx.new(|_| GalleryView),
            )
            .expect("failed to open gallery window");
        });
}

struct GalleryView;

impl Render for GalleryView {
    fn render(&mut self, _: &mut Window, cx: &mut gpui_kit::Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let toggle_label = if theme.is_dark() { "Light" } else { "Dark" };
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_5()
            .p_6()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_lg().child(format!(
                        "lithe-gpui · Chapter 1 Primitives ({})",
                        if theme.is_dark() { "dark" } else { "light" }
                    )))
                    .child(
                        div()
                            .id("toggle-mode")
                            .px_3()
                            .h_8()
                            .flex()
                            .items_center()
                            .rounded_full()
                            .border_1()
                            .border_color(theme.border_strong)
                            .bg(theme.surface)
                            .text_sm()
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover))
                            .child(format!("Switch to {toggle_label}"))
                            .on_click(|_, _, cx| {
                                let target = if cx.theme().is_dark() {
                                    ThemeMode::Light
                                } else {
                                    ThemeMode::Dark
                                };
                                Theme::set_mode(target, cx);
                            }),
                    ),
            )
            .child(section(
                "Icon",
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_3()
                    .child(Icon::new("icon-check", "icons/check.svg"))
                    .child(Icon::new("icon-x-small", "icons/x.svg").xsmall())
                    .child(Icon::new("icon-plus-large", "icons/plus.svg").large())
                    .child(
                        Icon::new("icon-chevron-primary", "icons/chevron-down.svg")
                            .color(theme.primary),
                    )
                    .child(Icon::new("icon-check-danger", "icons/check.svg").color(theme.danger))
                    .child(Icon::new("icon-plus-custom", "icons/plus.svg").with_size(px(28.))),
            ))
            .child(section(
                "Separator",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .w_full()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.subtle_foreground)
                            .child("above"),
                    )
                    .child(Separator::new("separator-h"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.subtle_foreground)
                            .child("below"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .h_8()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.subtle_foreground)
                                    .child("left"),
                            )
                            .child(Separator::new("separator-v").vertical())
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.subtle_foreground)
                                    .child("right"),
                            ),
                    ),
            ))
            .child(section(
                "Avatar",
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(Avatar::new("avatar-xsmall", "ZK").xsmall())
                    .child(Avatar::new("avatar-small", "LT").small())
                    .child(Avatar::new("avatar-medium", "LI").medium())
                    .child(Avatar::new("avatar-large", "UI").large()),
            ))
            .child(section(
                "Badge",
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(Badge::new("badge-neutral", "Neutral"))
                    .child(Badge::new("badge-primary", "Primary").variant(BadgeVariant::Primary))
                    .child(Badge::new("badge-success", "Success").variant(BadgeVariant::Success))
                    .child(Badge::new("badge-warning", "Warning").variant(BadgeVariant::Warning))
                    .child(Badge::new("badge-danger", "Danger").variant(BadgeVariant::Danger))
                    .child(Badge::new("badge-info", "Info").variant(BadgeVariant::Info))
                    .child(Badge::new("badge-small", "Small").small())
                    .child(Badge::new("badge-large", "Large").large()),
            ))
            .child(section(
                "Tag",
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(Tag::new("tag-plain", "Plain"))
                    .child(Tag::new("tag-closable", "Closable").on_remove(|_, _| {
                        // gallery 不持有状态：仅演示回调通路（集成测试覆盖真实点击）。
                    })),
            ))
            .child(section(
                "Label",
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(Label::new("label-name", "Project name"))
                    .child(Label::new("label-required", "Java home").required(true)),
            ))
            .child(section(
                "Kbd",
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(Kbd::new("kbd-cmd", "⌘K"))
                    .child(Kbd::new("kbd-combo", "Ctrl+Shift+P")),
            ))
            .child(section(
                "Spinner",
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(Spinner::new("spinner-medium"))
                    .child(Spinner::new("spinner-large").large()),
            ))
            .child(section(
                "Progress",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .w_full()
                    .child(Progress::new("progress-25").value(0.25))
                    .child(Progress::new("progress-60").value(0.6))
                    .child(Progress::new("progress-100").value(1.0)),
            ))
            .child(section(
                "Severity",
                div().flex().items_center().gap_3().children(
                    [
                        (Severity::Neutral, "Neutral"),
                        (Severity::Info, "Info"),
                        (Severity::Success, "Success"),
                        (Severity::Warning, "Warning"),
                        (Severity::Danger, "Danger"),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(ix, (severity, name))| {
                        div()
                            .id(("severity", ix))
                            .px_2()
                            .h_6()
                            .flex()
                            .items_center()
                            .rounded_full()
                            .bg(theme.severity_subtle(severity))
                            .text_color(theme.severity(severity))
                            .text_xs()
                            .child(name)
                    }),
                ),
            ))
    }
}

fn section(title: &'static str, content: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .w_full()
        .child(div().text_sm().child(title))
        .child(content)
}
