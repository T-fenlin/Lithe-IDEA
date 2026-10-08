//! 过渡桥：`gpui-kit` 的图标槽只收它**自己**的 `Icon`。
//!
//! `shared::icons::idea_icon_mono` 已随这次迁移改成组件库的元素
//! （`lithe_ui::primitives::Icon`），而本 crate 这几处的图标要交给上游
//! `PopupMenuItem::icon(impl Into<gpui_kit::component::Icon>)` —— 元素进不去。
//! 这里按**同一份资源路径**就地把 IDEA 的 SVG 包成上游 `Icon`：画出来的东西与迁移前逐字节相同
//! （同一个 `icons/idea/**` 路径、同一套上游默认尺寸与配色）。
//!
//! 这批菜单（菜单栏、项目菜单、命令面板、调试菜单）整片迁到 `lithe_ui::menus` 时，
//! **删掉本模块**，那些调用点改用 `shared` 的元素版——那时它们才拿得到组件库的图标槽。

use gpui::SharedString;

use lithe_gpui_shared::icons::idea;

/// 单色的 IDEA 图标，形状与迁移前的 `shared::icons::idea_icon_mono` 一致。
#[must_use]
pub(crate) fn idea_icon_mono(icon: &idea::IdeaIcon, is_dark: bool) -> gpui_kit::component::Icon {
    let path: SharedString = icon.path(is_dark).into();
    gpui_kit::component::Icon::empty().path(path)
}
