//! UI 集成测试的夹具：用**生产**的窗口入口在确定尺寸下开一个无头窗口。
//!
//! 形状逐字对照上游 `gpui-kit-0.7.0/tests/common/mod.rs`（`docs/gpui kit/0.7.0/docs/test.md`
//! §配置测试项目 要求这两个文件成对存在：只复制 `ui.rs` 会在 `mod common;` 处失败）。
//! 只改了一处：说明文字换成中文，行为与上游一致。
//!
//! 为什么用 `gpui_kit::open_window` 而不是 `cx.open_window`：前者内部已经 `Root::new`，
//! 所以返回的 `WindowHandle<Root>` 直接可以走 `window.find(..)` / `window.click(..)`
//! 这套观察接口；后者要求闭包自己返回 `Root` 包好的视图（见 `UI-MAP.md` §1.4 C 组的判据）。
#![allow(dead_code)]

use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, Context, Entity, Pixels, Point, Render, Size,
    TestAppContext, Window, WindowBounds, WindowHandle, WindowOptions, base::Root,
};

/// 开一个确定尺寸的无头窗口，并返回窗口句柄与其中的视图实体。
///
/// `size` 为 `None` 时用"最大化"边界（与上游同一口径）。
pub fn open_window<V: Render>(
    cx: &mut TestAppContext,
    size: Option<Size<Pixels>>,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> (WindowHandle<Root>, Entity<V>) {
    cx.update(|cx| {
        let bounds = size
            .map(|size| Bounds {
                origin: Point::default(),
                size,
            })
            .unwrap_or_else(|| Bounds::maximized(None, cx));
        let (window, content) = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            build,
        )
        .expect("open test window");
        (window.downcast::<Root>().expect("Base Root"), content)
    })
}

/// 在窗口已建好之后更新其中的视图实体（需要 `&mut Window` 的初始化写在这里）。
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
