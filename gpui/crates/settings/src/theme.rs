//! 主题：加载目录、「按名字应用」的唯一入口、以及 UI 字号到 rem 基准的映射。
//!
//! ## 为什么这一个函数同时服务启动与设置界面
//!
//! 改设置里的「配色主题」与启动时应用 `--theme` 必须是**同一条路**，否则两条路各自
//! `Theme::change` / `apply_config` 迟早会漂移（顺序错一次就是"主题看起来没生效"）。
//! 所以 `gpui/crates/app/src/main.rs` 原来那份 `apply_lithe_theme` 被拆成
//! [`watch_lithe_themes`]（只负责"装载 + 监听 + 重新加载后复原当前主题"）与
//! [`apply_theme_by_name`]（真正应用一个主题），设置界面的 setter 直接调后者。
//!
//! ## 两个必须记住的坑（沿用原来 `main.rs:39-48,64-71` 的结论）
//!
//! 1. 主题文件的根是 **`ThemeSet`**（`{ name, author?, themes: [ThemeConfig] }`）而不是
//!    `ThemeConfig`（`gpui-component-0.6.6/src/theme/schema.rs:22-34`）；解析失败的文件会被
//!    **整份静默忽略**（`theme/registry.rs:252-258`），所以"主题没生效"往往一点报错都没有。
//! 2. **必须先切 `ThemeMode` 再 `apply_config`**：`apply_config` 只把配置写进
//!    `light_theme` / `dark_theme` 两个槽里按 `config.mode` 对应的那一个
//!    （`theme/schema.rs:1060-1065`），而实际渲染用的是**当前 `ThemeMode`** 对应的槽。

use std::path::PathBuf;

use gpui_kit::component::{Theme, ThemeRegistry};
use gpui_kit::{App, SharedString, WindowAppearance, px};

use crate::schema::theme_font_size_for;

/// Lithe 主题目录（`gpui/themes`）。
///
/// 用 `CARGO_MANIFEST_DIR`（= `gpui/crates/settings`）拼路径，不依赖进程的工作目录：
/// `watch_dir` 收的是真实路径，而"从哪个目录启动 exe"是会变的。本 crate 与
/// `gpui/crates/app` 都在 `crates/<name>` 这一层，所以 `../../themes` 都指向 `gpui/themes`。
pub fn themes_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../themes"))
}

/// 装载 `gpui/themes/` 并监听它；**目录重新加载后按"当前生效主题"复原**。
///
/// `watch_dir` 自己会先 `reload_themes` 再调回调（`theme/registry.rs:104-115`），所以回调里
/// `themes()` 已经是新的一份。回调不记住某一次的主题名，而是问 [`crate::store::SettingsStore`]
/// "现在应该是什么主题" —— 这样设置界面改过主题之后再热改主题文件也不会退回旧主题。
pub fn watch_lithe_themes(cx: &mut App) {
    let themes_dir = themes_dir();
    let callback_dir = themes_dir.clone();

    let result = ThemeRegistry::watch_dir(themes_dir.clone(), cx, move |cx| {
        // 注册表在这一刻可能还没有 `SettingsStore`（启动顺序见 `main.rs`），
        // 拿不到就退回默认主题名，至少让主题目录里的主题能被应用一次。
        match crate::store::try_store(cx) {
            Some(store) => {
                store.update(cx, |store, cx| store.reapply_theme(cx));
            }
            None => {
                let name = crate::schema::DEFAULT_THEME;
                if !apply_theme_by_name(cx, name) {
                    eprintln!(
                        "S1_THEME missing name={name} dir={}",
                        callback_dir.display()
                    );
                }
            }
        }
    });

    if let Err(error) = result {
        eprintln!(
            "S1_THEME watch_failed dir={} error={error}",
            themes_dir.display()
        );
    }
}

/// 按名字应用一个主题；`false` = 注册表里没有这个名字（**不改动当前主题**）。
///
/// 顺序（缺一不可，逐条理由见模块文档）：
/// 1. `Theme::change(mode)` —— 先让当前 `ThemeMode` 指向要应用的那个槽；
/// 2. `apply_config` —— 把 Lithe 的配置写进该槽并应用到当前主题；
/// 3. 让调用方随后覆盖 `font_size`（`apply_config` 会把主题文件里的 `font.size` 写回来）；
/// 4. `sync_base` —— 把颜色/圆角/滚动条样式推给 Base 层（`theme/mod.rs:349-372` 明确要求：
///    直接改 `Theme::global_mut` 的字段后不调它，滚动条会继续用旧样式）；
/// 5. `refresh_windows` —— 让所有窗口重绘（`Root::render` 每帧会按 `font_size` 重设 rem，
///    `root.rs:582`）。
pub fn apply_theme_by_name(cx: &mut App, name: &str) -> bool {
    let theme = ThemeRegistry::global(cx)
        .themes()
        .get(&SharedString::from(name.to_string()))
        .cloned();

    let Some(theme) = theme else {
        eprintln!("S1_THEME missing name={name}");
        return false;
    };

    let mode = theme.mode;
    Theme::change(mode, None, cx);
    Theme::global_mut(cx).apply_config(&theme);
    Theme::sync_base(cx);
    cx.refresh_windows();
    println!("S1_THEME applied={name} dark={}", mode.is_dark());
    true
}

/// 把 UI 字号写进主题的 rem 基准并刷新窗口。
///
/// `font_size = 16 × (uiFontSize / 13)`，见 [`theme_font_size_for`]。**必须在
/// [`apply_theme_by_name`] 之后调用**，因为 `apply_config` 会用主题文件里的 `font.size` 覆盖它。
pub fn apply_theme_font_size(cx: &mut App, ui_font_size: f64) {
    let font_size = px(theme_font_size_for(ui_font_size));
    Theme::global_mut(cx).font_size = font_size;
    Theme::sync_base(cx);
    cx.refresh_windows();
    println!(
        "S1_THEME font_size={:?} ui_font_size={ui_font_size}",
        font_size
    );
}

/// 注册表里当前可选的**主题名**（按注册表的稳定序：默认主题在前、浅色在前、名字不区分大小写）。
///
/// 这就是设置界面「配色主题」下拉的候选集合 —— 不写死，因此以后往 `gpui/themes/` 里
/// 丢一个新主题文件，下拉里会自动多一项。
pub fn theme_names(cx: &App) -> Vec<String> {
    ThemeRegistry::global(cx)
        .sorted_themes()
        .into_iter()
        .map(|theme| theme.name.to_string())
        .collect()
}

/// 某个主题名在注册表里是不是深色主题；名字不在注册表里返回 `None`。
///
/// 设置界面在两个地方要用它：
/// 1. 「配色主题」下拉在**跟随系统**时要把选择写进 `autoThemeDark` 还是 `autoThemeLight`
///    （Windows 是 `theme.isDark ? "autoThemeDark" : "autoThemeLight"`，
///    `macos-settings-panels.tsx:122`）；
/// 2. [`crate::store::SettingsStore::appearance_mode`] 需要判断当前主题的明暗。
pub fn theme_is_dark(cx: &App, name: &str) -> Option<bool> {
    ThemeRegistry::global(cx)
        .themes()
        .get(&SharedString::from(name.to_string()))
        .map(|theme| theme.mode.is_dark())
}

/// 系统当前是否处于深色外观。
///
/// 用 `Window::appearance()`（`gpui-pre-0.3.6/src/window.rs:2765`）的同源 API
/// `App::window_appearance()`（`gpui-pre-0.3.6/src/app.rs:1508` 附近的窗口外观全局值），
/// 映射规则与 gpui-kit 自己的 `impl From<WindowAppearance> for ThemeMode`
/// （`gpui-component-0.6.6/src/theme/mod.rs:722-727`）逐字一致。
pub fn system_is_dark(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 主题目录必须指向 `gpui/themes`（拼错一层就会"一个主题都加载不出来"且不报错）。
    #[test]
    fn themes_dir_points_at_the_repo_themes_folder() {
        let dir = themes_dir();
        assert!(dir.ends_with("themes"), "{dir:?}");
        assert!(dir.join("lithe-dark.json").exists(), "{dir:?}");
        assert!(dir.join("lithe-light.json").exists(), "{dir:?}");
    }

    /// 系统外观 → 明暗：四个变体逐个钉住（Vibrant* 也属于 mac 的深浅两态）。
    #[test]
    fn appearance_maps_to_dark_and_light() {
        assert!(system_is_dark(WindowAppearance::Dark));
        assert!(system_is_dark(WindowAppearance::VibrantDark));
        assert!(!system_is_dark(WindowAppearance::Light));
        assert!(!system_is_dark(WindowAppearance::VibrantLight));
    }
}
