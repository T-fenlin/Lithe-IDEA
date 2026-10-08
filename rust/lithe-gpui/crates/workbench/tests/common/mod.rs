#![allow(dead_code)]

use gpui::{App, AppContext, Bounds, Context, Entity, Pixels, Point, Render, Size, TestAppContext, Window, WindowBounds, WindowHandle, WindowOptions};
use gpui_kit::{AnyWindowHandle};
// 窗口根已换成自建的 `WindowRoot`（门 G1）：测试宿主与生产组合根同形，
// 否则集成测试跑的是另外一套根，浮层这条路根本没被覆盖。
use lithe_ui::shell::{WindowRoot, open_rooted_window};

pub fn open_window<V: Render>(
    cx: &mut TestAppContext,
    size: Option<Size<Pixels>>,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> (WindowHandle<WindowRoot>, Entity<V>) {
    cx.update(|cx| {
        // 窗口根与浮层要用组件库的主题全局。不调 `lithe_ui::init`：它会校验
        // `Application::with_assets`，无头测试里没有资产源，会直接 panic。
        if !cx.has_global::<lithe_ui::theme::Theme>() {
            lithe_ui::theme::Theme::init(cx);
        }
        let bounds = size
            .map(|size| Bounds {
                origin: Point::default(),
                size,
            })
            .unwrap_or_else(|| Bounds::maximized(None, cx));
        let (window, content) = open_rooted_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            build,
        )
        .expect("open test window");
        (
            window.downcast::<WindowRoot>().expect("WindowRoot"),
            content,
        )
    })
}

pub fn update_content<V: Render, R>(
    window: impl Into<AnyWindowHandle>,
    content: &Entity<V>,
    cx: &mut TestAppContext,
    update: impl FnOnce(&mut V, &mut Window, &mut Context<V>) -> R,
) -> gpui_kit::Result<R> {
    cx.update_window(window.into(), |_, window, cx| {
        content.update(cx, |view, cx| update(view, window, cx))
    })
}
