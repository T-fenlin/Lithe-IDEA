//! 组件库主题与宿主主题的桥接。
//!
//! `lithe-gpui` 的组件读的是**它自己的**主题全局（`lithe_gpui::theme::Theme`），
//! 而宿主的主题是 gpui-kit 的 `Theme`。两者不是同一份数据，所以外壳在启动时
//! 装一次组件库主题，并把宿主的颜色映射进去，之后每次宿主换主题都重新映射。
//! 不装的话组件一渲染就会 panic（`cx.theme()` 读不到全局）。
//!
//! 映射是**保守**的：只覆盖组件真正读的、宿主也确实有的那些语义色；其余
//! （渐变、图表、ANSI、语法色等）留给组件库自己的调色板。

use gpui_kit::App;
use gpui_kit::component::ActiveTheme as _;
use lithe_gpui::theme::{Mode, Palette, Theme};

/// 装上组件库主题，并让它跟随宿主的主题；由组合根（`app`）在启动时调用一次。
pub fn install_component_theme(cx: &mut App) {
    Theme::init(cx);
    sync_component_theme(cx);
    cx.observe_global::<gpui_kit::component::Theme>(|cx| sync_component_theme(cx))
        .detach();
}

/// 把宿主当前的主题映射进组件库的主题。
fn sync_component_theme(cx: &mut App) {
    let host = cx.theme();
    let mode = if host.is_dark() {
        Mode::Dark
    } else {
        Mode::Light
    };
    let mut palette = match mode {
        Mode::Dark => Palette::dark(false),
        Mode::Light => Palette::light(false),
    };
    // 面板与中央列用的是同一个底色（外壳原先也是这么画的），分隔线用宿主的边框色。
    palette.bg = host.background;
    palette.surface = host.background;
    palette.sunken = host.muted;
    palette.overlay = host.popover;
    palette.hover = host.list_hover;
    palette.active = host.secondary;
    palette.border = host.border;
    palette.border_strong = host.border;
    palette.fg = host.foreground;
    palette.fg_muted = host.muted_foreground;
    palette.fg_subtle = host.muted_foreground;
    palette.fg_disabled = host.muted_foreground.opacity(0.6);
    palette.accent = host.accent;
    palette.accent_hover = host.accent;
    palette.focus = host.ring;
    palette.link = host.ring;
    palette.selection = host.sidebar_accent;
    palette.success = host.success;
    palette.warning = host.warning;
    palette.danger = host.danger;
    palette.info = host.info;
    Theme::apply(mode, Some(palette), cx);
}
