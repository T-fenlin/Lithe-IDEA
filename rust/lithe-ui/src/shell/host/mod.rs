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

pub use ext::WindowRootExt;
pub use root::WindowRoot;

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
