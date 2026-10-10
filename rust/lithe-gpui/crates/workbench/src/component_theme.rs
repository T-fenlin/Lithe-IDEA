//! 过渡适配：把宿主主题喂给**仍在 gpui-kit 组件上的那一半**。
//!
//! 宿主主题的真源现在是组件库的 `lithe_ui::theme::Theme`——设置层
//! （`crates/settings/src/theme.rs`）每次换主题都往它写。但 workbench 里还有一批
//! gpui-kit 组件（`Icon` / `TitleBar` / `PopupMenuItem` / `Badge` / `Checkbox` / …），
//! 它们只认 gpui-kit 自己的 `Theme` 全局，所以这台桥**反过来装**：观察组件库主题，
//! 把当前主题喂给 gpui-kit。
//!
//! 喂法是**无损**的：不把组件库的 `Palette` 还原成 gpui-kit 的颜色（那会丢掉
//! `button.hover.background` / `tab.active.background` / `table.head.foreground` 这类
//! 组件专用键），而是拿设置层给出的**主题文件原文**交给 gpui-kit 自己的 `ThemeSet`
//! 反序列化，再 `apply_config`——与迁移前逐键一致。
//!
//! 本文件是**临时代码**：workbench 全部迁到 lithe-ui、gpui-kit 组件清零那一天，
//! 它连同 workbench 的 `gpui-kit` 依赖一起删。

use std::rc::Rc;

use gpui::{App, px};
use gpui_kit::component::{Theme as LegacyTheme, ThemeConfig as LegacyThemeConfig};
use lithe_ui::theme::{ActiveTheme as _, Theme as ComponentTheme};

/// 装上组件库主题，并让它带上 gpui-kit 的老消费者；由组合根（`app`）在启动时调用一次。
pub fn install_component_theme(cx: &mut App) {
    ComponentTheme::init(cx);
    // gpui-kit 的主题全局得先存在，下面的写入才有落点。正常路径里组合根已经调过
    // `gpui_kit::init`（它同时初始化 `ThemeRegistry`），这里只为「只装主题、不跑
    // 组合根」的集成测试兜底。
    if !cx.has_global::<LegacyTheme>() {
        gpui_kit::init(cx);
    }
    sync_legacy_theme(cx);
    cx.observe_global::<ComponentTheme>(|cx| sync_legacy_theme(cx))
        .detach();
}

/// 把当前宿主主题（含字体）喂给 gpui-kit 的主题全局。
fn sync_legacy_theme(cx: &mut App) {
    // 字体以组件库主题为准：它的 `font_size` 是尺寸缩放的唯一锚点，两套字体不能各说各话。
    let (font_family, mono_family, font_size, mono_font_size) = {
        let theme = cx.theme();
        (
            theme.font_family.clone(),
            theme.mono_family.clone(),
            theme.font_size,
            theme.mono_font_size,
        )
    };

    match legacy_config(cx) {
        Some(config) => {
            let (name, dark) = (config.name.to_string(), config.mode.is_dark());
            LegacyTheme::update(cx, |theme| {
                theme.apply_config(&config);
                theme.font_family = font_family;
                theme.mono_font_family = mono_family;
                theme.font_size = px(font_size);
                theme.mono_font_size = px(mono_font_size);
            });
            println!("S1_THEME legacy_synced name={name} dark={dark}");
        }
        // 还没有活动主题（只装主题、没走设置层的测试）：字体仍要对齐，颜色留给
        // `gpui_kit::init` 装的那套默认值。
        None => {
            LegacyTheme::update(cx, |theme| {
                theme.font_family = font_family;
                theme.mono_font_family = mono_family;
                theme.font_size = px(font_size);
                theme.mono_font_size = px(mono_font_size);
            });
        }
    }
}

/// 当前活动主题在 gpui-kit 眼里的那份配置：从设置层给的主题文件原文反序列化而来。
///
/// 走原文而不是走组件库调色板，是为了**逐键无损**：`ThemeConfig` 认得的键（包括
/// 渐变、`button.*`、`tab.*` 这些组件专用槽）全部原样落到 gpui-kit 的主题上。
fn legacy_config(cx: &App) -> Option<Rc<LegacyThemeConfig>> {
    let name = lithe_gpui_settings::theme::active_theme_name(cx)?;
    let document = lithe_gpui_settings::theme::theme_document_for(&name)?;
    let set: gpui_kit::component::ThemeSet = serde_json::from_str(&document).ok()?;
    let config = set.themes.into_iter().find(|theme| theme.name == name)?;
    Some(Rc::new(config))
}
