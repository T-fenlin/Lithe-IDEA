//! 测试共用夹具：经生产入口 `gpui_kit::open_window` 开确定性尺寸的无头窗口。

use gpui_kit::{
    AnyWindowHandle, App, Bounds, Entity, Pixels, Point, Render, TestAppContext, Window,
    WindowBounds, WindowOptions, size,
};

pub fn open_window<V: Render>(
    cx: &mut TestAppContext,
    width: Pixels,
    height: Pixels,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> AnyWindowHandle {
    cx.update(move |cx| {
        let bounds = Bounds {
            origin: Point::default(),
            size: size(width, height),
        };
        let (handle, _) = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            build,
        )
        .expect("open test window");
        handle
    })
}
