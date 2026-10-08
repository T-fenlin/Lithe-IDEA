//! 窗口根宿主（G1）：一个窗口只有一层浮层栈。
//!
//! 它替代过渡期上游 gpui-kit 的 `Root`。宿主自己是一个**每帧渲染**的 `Entity`，
//! 把窗口内容与所有浮层（抽屉 / 对话框 / 通知）挂到同一条
//! `raise(..).with_priority(..)` 链上——排序与绘制交给 gpui 的 `Window::defer_draw`
//! （见 `crate::primitives::layer`），本模块不重画遮罩、不自己管焦点。
//!
//! 两条纪律，违反任一条都会出现"看起来能用、实际会坏"的症状：
//!
//! 1. **浮层元素每帧重建**。`primitives::layer` 在 prepaint 时消费元素
//!    （`child.take().expect("a raised element prepaints once")`），所以宿主不能把
//!    `AnyElement` 存起来复用——`Overlay::build` 每次渲染都重新调用。
//! 2. **一个窗口只能有一个宿主**，整片切换、不逐控件混用。新旧两套根并存会双重渲染、
//!    互抢焦点；而且旧根那套 API 在找不到自己的根时会直接 panic。

mod ext;
mod root;
#[cfg(all(test, feature = "test-support"))]
mod tests;

use gpui::{AnyWindowHandle, App, AppContext as _, Entity, Render, Window, WindowOptions};

pub use ext::{Close, WindowRootExt};
pub use root::WindowRoot;

/// 浮层组件**自己**关掉时（Esc / 遮罩 / 关闭按钮），把这一层从宿主栈里摘掉。
///
/// 具名函数而不是每个调用点手写一遍闭包：漏掉其中一处，就留下一个"关不掉"的浮层——
/// 组件那边动画走完了、焦点归还了，可栈里那一层还在，下一次 Esc 会再关掉一个更下层的浮层。
pub fn dismiss_hosted(window: &mut Window, cx: &mut App) {
    window.close_top_layer(cx);
}

/// 开一个窗口并装上窗口根宿主；返回窗口句柄与**应用内容**实体（不是根）。
///
/// 对应上游 `gpui_kit::open_window`（`gpui-kit-0.7.1/src/lib.rs:149`）：把"开窗 + 包根 +
/// 回传内容实体"合成一步。调用方拿到的是内容实体，浮层入口走 [`WindowRootExt`]。
///
/// 句柄的 `downcast::<WindowRoot>()` 能命中：gpui 的 `open_window` 取的是**内容**类型
/// （`WindowHandle<V>` 记为 `TypeId::of::<V>()`），根实体由 `cx.new` 包出来。
pub fn open_rooted_window<V: Render>(
    options: WindowOptions,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> anyhow::Result<(AnyWindowHandle, Entity<V>)> {
    let mut built = None;
    let window = cx.open_window(options, |window, cx| {
        let view = build(window, cx);
        built = Some(view.clone());
        cx.new(|cx| WindowRoot::new(view, window, cx))
    })?;
    Ok((
        window.into(),
        built.expect("open_rooted_window ran its build closure"),
    ))
}

/// 浮层优先级：数字越大画得越晚、盖在越上面。
///
/// 档位之间留出间隔，宿主内部再按"后压入的更高"在同类里递增。
pub mod priority {
    /// 抽屉 / 侧滑：在对话框之下。
    pub const DRAWER: usize = 100;
    /// 对话框 / 弹窗：盖住抽屉。
    pub const DIALOG: usize = 200;
    /// 通知 / toast：永远最上。
    pub const NOTIFICATION: usize = 300;
}
