//! 组件画廊（dev-only）：`cargo run -p lithe-gpui --example gallery`。
//!
//! 左侧是章节切换侧栏，右侧内容区单章展示、可滚动。内嵌了最小 SVG 资产
//! 集合，演示 Icon 的注入式图标约定——库本体不内嵌图标，正式应用走
//! app crate 的 `LitheAssets`。分章 story 见同目录 `stories.rs`。

mod stories;

use std::borrow::Cow;

use gpui_kit::prelude::*;

use gpui_kit::{
    AssetSource, Bounds, Context, Point, Render, SharedString, Window, WindowBounds, WindowOptions,
    div, px, rems, size,
};

use lithe_gpui::{
    ActiveTheme as _, Button, Entity, InputState, ResizableState, Sidebar, Sizable as _, Theme,
    ThemeMode, init, open_window,
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
                size: size(px(1040.), px(860.)),
            };
            open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| GalleryView::new(window, cx)),
            )
            .expect("failed to open gallery window");
        });
}

/// 画廊状态：当前章节 + 受控组件（表单/标签页/分页）的值。
struct GalleryView {
    chapter: usize,
    dock_state: Entity<ResizableState>,
    input: Entity<InputState>,
    checked: bool,
    switch_on: bool,
    radio_ix: usize,
    tab_ix: usize,
    page: usize,
    selected_commit: Option<usize>,
}

impl GalleryView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            chapter: 0,
            dock_state: cx.new(|_| ResizableState::default()),
            input: cx.new(|cx| InputState::new(window, cx).placeholder("Search files…")),
            checked: false,
            switch_on: true,
            radio_ix: 1,
            tab_ix: 0,
            page: 2,
            selected_commit: None,
        }
    }
}

impl Render for GalleryView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let toggle_label = if theme.is_dark() { "Light" } else { "Dark" };
        let chapter = self.chapter;
        let state = cx.entity().downgrade();
        let sidebar_state = state.clone();

        div()
            .size_full()
            .flex()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                // 章节切换侧栏
                Sidebar::new("chapter-sidebar").w(rems(11.0)).child(
                    div().flex().flex_col().gap_1().p_3().pt_4().children(
                        stories::CHAPTERS.iter().enumerate().map(move |(ix, name)| {
                            let sidebar_state = sidebar_state.clone();
                            let name = *name;
                            Button::new(("chapter", ix))
                                .label(name)
                                .ghost()
                                .small()
                                .selected(chapter == ix)
                                .on_click(move |_, _, cx| {
                                    let _ = sidebar_state.update(cx, |v, cx| {
                                        v.chapter = ix;
                                        cx.notify();
                                    });
                                })
                        }),
                    ),
                ),
            )
            .child(
                // 右列：画廊头 + 可滚动内容区（Stateful div 才能滚）
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_6()
                            .py_3()
                            .border_b_1()
                            .border_color(theme.border)
                            .child(div().text_lg().child(format!(
                                "lithe-gpui gallery ({})",
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
                    .child(
                        div()
                            .id("gallery-content")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .px_6()
                            .py_5()
                            .flex()
                            .flex_col()
                            .gap_5()
                            .child(stories::chapter(chapter, self, &state, cx)),
                    ),
            )
    }
}
