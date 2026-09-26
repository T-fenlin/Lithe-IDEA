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
use std::sync::Arc;

use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::component::{Theme, ThemeConfig, ThemeMode, ThemeRegistry};
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
    stamp_builtin_highlight_if_absent(cx, &theme, mode);
    Theme::global_mut(cx).apply_config(&theme);
    Theme::sync_base(cx);
    cx.refresh_windows();
    println!("S1_THEME applied={name} dark={}", mode.is_dark());
    true
}

/// 目标主题**没有** `highlight` 段时，把 `highlight_theme` 复位成该明暗的内置那一份。
///
/// 为什么必须有这一步：`apply_config` 只在 `config.highlight` 是 `Some` 时整段替换
/// `highlight_theme`（`gpui-component-0.6.6/src/theme/schema.rs:1066-1073`），**没有**按字段合并；
/// 而 `Theme::change(mode)` 重放的是"该明暗槽里存着的那个配置"（`theme/mod.rs:671-686`），
/// 对没有 `highlight` 的主题（`gpui/themes/lithe-*.json`）同样不会清掉上一份。于是：
///
/// `Lithe Dark`（无 `highlight`）→ `Gruvbox Light`（有）→ 切回 `Lithe Dark`
/// 会把 **Gruvbox Light 的浅色语法色留在深色底上**（深底深字）。这不是"主题没生效"，
/// 而是编辑器那一层沿用了上一个主题的高亮表。
///
/// 复位源是注册表的 `default_themes()`：它就是内置的 `Default Light` / `Default Dark`
/// （`theme/registry.rs:139-149`），两者都带 `highlight`（`theme/default-theme.json`）。
/// 复位发生在 `apply_config` **之前**，所以带 `highlight` 的主题不会被这里影响。
///
/// ⚠️ 覆盖不到的地方：`Theme::change` 内部会把当时的 `highlight_theme` 拷进
/// `install_text_view_defaults`（`src/text/mod.rs:64-75`，供 TextView 的 markdown 代码块用），
/// 这一步在复位之前，所以那些代码块仍可能拿着上一份。本仓库目前没有任何 TextView / markdown
/// 代码块消费方（`crates/**` 里 `TextView` 零命中），编辑器正文每帧读 `cx.theme().highlight_theme`
/// （`gpui-component-0.6.6/src/input/input.rs:510`），所以这条不影响当前界面；将来若有消费方，
/// 需要把复位挪到 `Theme::change` 之前（那要求先修正槽配置，见 `schema.rs:1060-1065` 的写入点）。
fn stamp_builtin_highlight_if_absent(cx: &mut App, theme: &ThemeConfig, mode: ThemeMode) {
    if theme.highlight.is_some() {
        return;
    }

    let builtin = ThemeRegistry::global(cx)
        .default_themes()
        .get(&mode)
        .cloned();
    let Some(builtin) = builtin else {
        return;
    };
    let Some(style) = builtin.highlight.clone() else {
        return;
    };

    Theme::global_mut(cx).highlight_theme = Arc::new(HighlightTheme {
        name: builtin.name.to_string(),
        appearance: mode,
        style,
    });
    println!(
        "S1_THEME highlight=builtin theme={} dark={} source={}",
        theme.name,
        mode.is_dark(),
        builtin.name
    );
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

/// 把「编辑器字号」写进主题的等宽字号并刷新窗口。
///
/// 落点是 `Theme::mono_font_size`：gpui-kit 的编辑器正文就用它
/// （`gpui-component-0.6.6/src/input/editor.rs:137-143` 的
/// `.text_size(cx.theme().mono_font_size)`），所以改这一个 token 就是"编辑器字号立即生效"。
///
/// 与 [`apply_theme_font_size`] 同一口径：**必须在 [`apply_theme_by_name`] 之后调用**
/// （主题文件里若写了 `font.mono_size`，`apply_config` 会先覆盖一次）。
/// `sync_base` 不是可选项：不推给 Base 层的话滚动条等基础件会继续用旧字号
/// （`gpui-component-0.6.6/src/theme/mod.rs:349-372`）。
pub fn apply_editor_font_size(cx: &mut App, editor_font_size: f64) {
    let font_size = px(editor_font_size as f32);
    Theme::global_mut(cx).mono_font_size = font_size;
    Theme::sync_base(cx);
    cx.refresh_windows();
    println!("S1_THEME mono_font_size={font_size:?} editor_font_size={editor_font_size}");
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

    /// `apply_config` **没有 fallback** 的 13 个颜色 key（`theme/schema.rs:685`、`:717-719` 的宏）。
    ///
    /// 少写这些 key 不会报错，但缺失时会去读编译进二进制的 shadcn 默认值，把中性灰/蓝
    /// 漏进主题（`schema.rs:740`、`:774-777`、`:802`、`:819`、`:1016`、`:743-772`）。
    /// 与 `gpui/themes/README.md` §1.5 是同一份清单。
    const REQUIRED_COLOR_KEYS: [&str; 13] = [
        "background",
        "border",
        "foreground",
        "muted.background",
        "primary.background",
        "secondary.background",
        "overlay",
        "base.red",
        "base.green",
        "base.blue",
        "base.yellow",
        "base.magenta",
        "base.cyan",
    ];

    /// 主题目录里每一份 JSON 都必须能被**真实的** `ThemeSet` 反序列化，并且
    /// 主题名非空且全局唯一、13 个无 fallback 的 key 齐全、每个颜色值 gpui 都解析得了。
    ///
    /// 这四件事全是**静默失效**，只会在界面上表现为"主题没出现"或"某个角还是 shadcn 的灰"：
    /// 1. 解析失败的文件被整份忽略，只打一行日志（`theme/registry.rs:252-258`）；
    /// 2. 同名主题条目被跳过（`registry.rs:270-273`），所以重名就是"少一个主题"；
    /// 3. 无 fallback 的 key 缺失会漏进内置默认色（见 [`REQUIRED_COLOR_KEYS`]）；
    /// 4. 颜色解析器只认 `#RRGGBB` / `#RRGGBBAA` / Tailwind 色名，**不认** `rgba(...)`
    ///    （`theme/color.rs:693-697`、`:763`）。
    ///
    /// 走的是真类型（`ThemeSet` / 真解析器），不是另写一份 schema，所以上游改了字段形状
    /// 这里会一起失败。纯同步读文件 + 反序列化，没有等待、没有全局状态。
    #[test]
    fn every_theme_file_is_a_loadable_theme_set() {
        use gpui_kit::component::{ThemeSet, try_parse_background, try_parse_color};
        use std::collections::BTreeMap;

        let dir = themes_dir();
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("read_dir failed dir={} error={error}", dir.display()))
            .map(|entry| entry.expect("read_dir entry").path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect();
        files.sort();
        assert!(!files.is_empty(), "no theme json in {}", dir.display());

        // 主题名 -> 定义它的文件：注册表按名字去重，先把重名找出来。
        let mut owners: BTreeMap<String, String> = BTreeMap::new();
        let mut theme_count = 0usize;

        for path in &files {
            let file = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let text = std::fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("{file}: not readable/UTF-8: {error}"));
            let set: ThemeSet = serde_json::from_str(&text)
                .unwrap_or_else(|error| panic!("{file}: not a loadable ThemeSet: {error}"));
            assert!(!set.themes.is_empty(), "{file}: themes[] is empty");

            for theme in &set.themes {
                let name = theme.name.to_string();
                assert!(!name.trim().is_empty(), "{file}: a theme has an empty name");
                if let Some(owner) = owners.insert(name.clone(), file.clone()) {
                    panic!("{file}: theme name {name:?} is already defined in {owner}");
                }

                // 序列化回 JSON 是为了按 **JSON key** 取值：`None` 会出现成 `null`，
                // 于是"这个 key 生效了没有"和文件的字面量对得上（`schema.rs` 用 `rename`）。
                let colors = serde_json::to_value(&theme.colors)
                    .unwrap_or_else(|error| panic!("{file}/{name}: colors re-encode: {error}"));
                let colors = colors
                    .as_object()
                    .unwrap_or_else(|| panic!("{file}/{name}: colors is not an object"));

                for key in REQUIRED_COLOR_KEYS {
                    match colors.get(key) {
                        Some(serde_json::Value::String(_)) => {}
                        other => panic!(
                            "{file}/{name}: required color {key:?} is missing/not a string ({other:?})"
                        ),
                    }
                }

                for (key, value) in colors {
                    // 没写的 key 反序列化成 `None`、再序列化回来就是 `null`：那是合法的
                    // （走 `apply_config` 的 fallback），只有"写了但 gpui 解析不了"才是错。
                    let Some(value) = value.as_str() else {
                        assert!(
                            value.is_null(),
                            "{file}/{name}: color {key:?} is not a string ({value:?})"
                        );
                        continue;
                    };
                    assert!(
                        try_parse_color(value).is_ok() || try_parse_background(value).is_ok(),
                        "{file}/{name}: color {key:?} = {value:?} is not parseable by gpui"
                    );
                }

                theme_count += 1;
            }
        }

        assert!(theme_count > 0, "no theme defined in {}", dir.display());
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
