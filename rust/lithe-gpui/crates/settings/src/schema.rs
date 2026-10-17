use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::keymap::KeymapSettings;

pub const UI_FONT_SIZE_MIN: f64 = 10.0;
pub const UI_FONT_SIZE_MAX: f64 = 24.0;
pub const UI_FONT_SIZE_STEP: f64 = 0.5;
pub const UI_FONT_SIZE_DEFAULT: f64 = 13.0;

pub const DEFAULT_DISPLAY_LANGUAGE: &str = "zh-CN";

pub const EDITOR_FONT_SIZE_MIN: f64 = 10.0;
pub const EDITOR_FONT_SIZE_MAX: f64 = 22.0;
pub const EDITOR_FONT_SIZE_DEFAULT: f64 = 14.0;

pub const TAB_SIZES: [u32; 3] = [2, 4, 8];
pub const TAB_SIZE_DEFAULT: u32 = 2;

pub const SHELL_SYSTEM_DEFAULT: &str = "";
pub const TERMINAL_SHELL_IDS: [&str; 4] = [SHELL_SYSTEM_DEFAULT, "powershell", "cmd", "wsl"];

pub fn normalize_editor_font_size(value: f64) -> f64 {
    if !value.is_finite() {
        return EDITOR_FONT_SIZE_DEFAULT;
    }
    value
        .round()
        .clamp(EDITOR_FONT_SIZE_MIN, EDITOR_FONT_SIZE_MAX)
}

pub const TERMINAL_FONT_SIZE_UNSET: f64 = 0.0;
pub const TERMINAL_FONT_SIZE_MIN: f64 = EDITOR_FONT_SIZE_MIN;
pub const TERMINAL_FONT_SIZE_MAX: f64 = EDITOR_FONT_SIZE_MAX;

pub fn normalize_terminal_font_size(value: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 {
        return TERMINAL_FONT_SIZE_UNSET;
    }
    value
        .round()
        .clamp(TERMINAL_FONT_SIZE_MIN, TERMINAL_FONT_SIZE_MAX)
}

pub const DISPLAY_LANGUAGES: [&str; 2] = ["en-US", DEFAULT_DISPLAY_LANGUAGE];

pub const DEFAULT_THEME: &str = "darcula";
pub const DEFAULT_AUTO_THEME_LIGHT: &str = "intellij-light";
pub const DEFAULT_AUTO_THEME_DARK: &str = "darcula";

pub const DEFAULT_ICONS: &str = "idea";

/// 菜单栏布局的 4 档。字面量的真源是
/// `rust/lithe-gpui/crates/workbench/src/menu_bar.rs` 的 `MenuBarMode::id()`；
/// settings 不能反依赖 workbench，所以这里存同一组字面量，认不出的值两侧都会回落默认。
pub const MENU_BAR_MODES: [&str; 4] = ["merged", "above", "autohide", "compact"];
pub const MENU_BAR_MODE_DEFAULT: &str = MENU_BAR_MODES[0];

pub const DEFAULT_MONO_FONT_FAMILY: &str = "JetBrains Mono";

pub fn slugify_theme_id(name: &str) -> String {
    let mut out = String::new();
    let mut pending_separator = false;
    for ch in name.chars() {
        if ch.is_alphanumeric() {
            if pending_separator && !out.is_empty() {
                out.push('-');
            }
            pending_separator = false;
            out.extend(ch.to_lowercase());
        } else if !out.is_empty() {
            pending_separator = true;
        }
    }
    if out.is_empty() {
        "theme".to_string()
    } else {
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeEntry {
    pub id: String,
    pub name: String,
    pub dark: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThemeIndex {
    entries: Vec<ThemeEntry>,
}

impl ThemeIndex {
    pub fn from_documents(documents: impl IntoIterator<Item = String>) -> Self {
        let mut entries: Vec<ThemeEntry> = Vec::new();

        for document in documents {
            let Ok(set) = serde_json::from_str::<RawThemeSet>(&document) else {
                continue;
            };
            for theme in set.themes {
                let name = theme.name.trim();
                if name.is_empty() {
                    continue;
                }
                let id = match theme.id.as_deref().map(str::trim) {
                    Some(id) if !id.is_empty() => id.to_string(),
                    _ => slugify_theme_id(name),
                };
                let duplicate = entries
                    .iter()
                    .any(|entry| entry.id == id || entry.name == name);
                if duplicate {
                    continue;
                }
                entries.push(ThemeEntry {
                    id,
                    name: name.to_string(),
                    dark: theme.mode.as_deref() == Some("dark"),
                });
            }
        }

        Self { entries }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[ThemeEntry] {
        &self.entries
    }

    pub fn id_for_name(&self, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.id.as_str())
    }

    pub fn name_for_id(&self, id: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.name.as_str())
    }

    pub fn canonicalize(&self, value: &str) -> Option<String> {
        let value = value.trim();
        if value.is_empty() {
            return None;
        }
        if let Some(entry) = self.entries.iter().find(|entry| entry.id == value) {
            return Some(entry.id.clone());
        }
        if let Some(entry) = self.entries.iter().find(|entry| entry.name == value) {
            return Some(entry.id.clone());
        }
        let lowered = value.to_lowercase();
        self.entries
            .iter()
            .find(|entry| {
                entry.id.to_lowercase() == lowered || entry.name.to_lowercase() == lowered
            })
            .map(|entry| entry.id.clone())
    }

    pub fn ids(&self) -> Vec<String> {
        self.entries.iter().map(|entry| entry.id.clone()).collect()
    }
}

#[derive(Deserialize)]
struct RawThemeSet {
    #[serde(default)]
    themes: Vec<RawTheme>,
}

#[derive(Deserialize)]
struct RawTheme {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(rename = "theme")]
    pub theme: String,

    #[serde(rename = "syncSystemTheme")]
    pub sync_system_theme: bool,

    #[serde(rename = "autoThemeLight")]
    pub auto_theme_light: String,

    #[serde(rename = "autoThemeDark")]
    pub auto_theme_dark: String,

    #[serde(rename = "uiFontSize")]
    pub ui_font_size: f64,

    #[serde(rename = "showStatusBar")]
    pub show_status_bar: bool,

    #[serde(rename = "displayLanguage")]
    pub display_language: String,

    #[serde(rename = "fontSize")]
    pub font_size: f64,

    #[serde(rename = "fontFamily")]
    pub font_family: String,

    #[serde(rename = "monoFontFamily")]
    pub mono_font_family: String,

    #[serde(rename = "tabSize")]
    pub tab_size: u32,

    #[serde(rename = "terminalDefaultShellId")]
    pub terminal_default_shell_id: String,

    #[serde(rename = "terminalFontSize")]
    pub terminal_font_size: f64,

    #[serde(rename = "confirmBeforeDiscard")]
    pub confirm_before_discard: bool,

    #[serde(rename = "javaHomePath")]
    pub java_home_path: String,

    #[serde(rename = "mavenExecutablePath")]
    pub maven_executable_path: String,

    #[serde(rename = "mavenJavaHomePath")]
    pub maven_java_home_path: String,

    #[serde(rename = "mavenSettingsPath")]
    pub maven_settings_path: String,

    #[serde(rename = "mavenLocalRepositoryPath")]
    pub maven_local_repository_path: String,

    #[serde(rename = "autoCompletion")]
    pub auto_completion: bool,

    #[serde(rename = "wordWrap")]
    pub word_wrap: bool,

    /// 缩略图开关。默认与组件库编辑器的出厂值一致（关），不为"像 IDEA"而改默认。
    #[serde(rename = "minimap")]
    pub minimap: bool,

    /// 行号开关。关掉时编辑器拿 `LineNumbers::Hidden`。
    #[serde(rename = "lineNumbers")]
    pub line_numbers: bool,

    /// 空白字符（空格 / 制表 / 换行标记）渲染开关。
    #[serde(rename = "renderWhitespace")]
    pub render_whitespace: bool,

    /// 缩进参考线开关。默认开 —— 与组件库 `Options::guides` 的默认值一致。
    #[serde(rename = "indentGuides")]
    pub indent_guides: bool,

    /// 括号按嵌套深度着色（真源里的「括号对着色」）。
    #[serde(rename = "rainbowBrackets")]
    pub rainbow_brackets: bool,

    /// 粘滞滚动：滚动时把包住顶行的块头钉在顶部。
    #[serde(rename = "stickyScroll")]
    pub sticky_scroll: bool,

    #[serde(rename = "autoSave")]
    pub auto_save: bool,

    #[serde(rename = "askWhereToOpenProjects")]
    pub ask_where_to_open_projects: bool,

    #[serde(rename = "openFoldersInNewWindow")]
    pub open_folders_in_new_window: bool,

    /// 菜单栏布局（`merged` / `above` / `autohide` / `compact`）。
    ///
    /// 真源里没有这个设置项：那 4 档以前只能靠 `--menu-bar-mode` 命令行开关选，改一次要重启。
    /// 落成持久化设置后，启动优先级仍是「命令行 > 设置 > 默认」。
    #[serde(rename = "menuBarMode")]
    pub menu_bar_mode: String,

    #[serde(rename = "icons")]
    pub icons: String,

    #[serde(rename = "uninstalledExtensions")]
    pub uninstalled_extensions: Vec<String>,

    /// 键位设置（预设 + 全部预设的用户覆盖），类型落在 `crate::keymap`。
    ///
    /// 老的设置文件里没有这个键：`Settings` 带容器级 `#[serde(default)]`，缺字段时整个
    /// `Settings` 走 `default()`，于是这里拿到 `KeymapSettings::default()`（Lithe 档、无覆盖）。
    #[serde(rename = "keymap")]
    pub keymap: KeymapSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.to_string(),
            sync_system_theme: false,
            auto_theme_light: DEFAULT_AUTO_THEME_LIGHT.to_string(),
            auto_theme_dark: DEFAULT_AUTO_THEME_DARK.to_string(),
            ui_font_size: UI_FONT_SIZE_DEFAULT,
            show_status_bar: true,
            display_language: DEFAULT_DISPLAY_LANGUAGE.to_string(),
            font_size: EDITOR_FONT_SIZE_DEFAULT,
            font_family: String::new(),
            mono_font_family: DEFAULT_MONO_FONT_FAMILY.to_string(),
            tab_size: TAB_SIZE_DEFAULT,
            terminal_default_shell_id: SHELL_SYSTEM_DEFAULT.to_string(),
            terminal_font_size: TERMINAL_FONT_SIZE_UNSET,
            confirm_before_discard: true,
            java_home_path: String::new(),
            maven_executable_path: String::new(),
            maven_java_home_path: String::new(),
            maven_settings_path: String::new(),
            maven_local_repository_path: String::new(),
            auto_completion: true,
            word_wrap: true,
            minimap: false,
            line_numbers: true,
            render_whitespace: false,
            indent_guides: true,
            rainbow_brackets: false,
            sticky_scroll: false,
            auto_save: true,
            ask_where_to_open_projects: true,
            open_folders_in_new_window: true,
            menu_bar_mode: MENU_BAR_MODE_DEFAULT.to_string(),
            icons: DEFAULT_ICONS.to_string(),
            uninstalled_extensions: Vec::new(),
            keymap: KeymapSettings::default(),
        }
    }
}

pub fn normalize_ui_font_size(value: f64) -> f64 {
    if !value.is_finite() {
        return UI_FONT_SIZE_DEFAULT;
    }
    let snapped = (value / UI_FONT_SIZE_STEP).round() * UI_FONT_SIZE_STEP;
    let clamped = snapped.clamp(UI_FONT_SIZE_MIN, UI_FONT_SIZE_MAX);
    (clamped * 100.0).round() / 100.0
}

pub fn theme_font_size_for(ui_font_size: f64) -> f32 {
    16.0 * (normalize_ui_font_size(ui_font_size) / UI_FONT_SIZE_DEFAULT) as f32
}

impl Settings {
    pub fn gpui_locale(&self) -> &'static str {
        match self.display_language.as_str() {
            "en-US" => "en",
            _ => "zh-CN",
        }
    }

    pub fn normalize(&mut self) {
        self.ui_font_size = normalize_ui_font_size(self.ui_font_size);
        self.font_size = normalize_editor_font_size(self.font_size);
        self.terminal_font_size = normalize_terminal_font_size(self.terminal_font_size);

        self.font_family = self.font_family.trim().to_string();
        self.mono_font_family = self.mono_font_family.trim().to_string();

        if !TAB_SIZES.contains(&self.tab_size) {
            self.tab_size = TAB_SIZE_DEFAULT;
        }

        if !TERMINAL_SHELL_IDS.contains(&self.terminal_default_shell_id.as_str()) {
            self.terminal_default_shell_id = SHELL_SYSTEM_DEFAULT.to_string();
        }

        if !DISPLAY_LANGUAGES.contains(&self.display_language.as_str()) {
            self.display_language = DEFAULT_DISPLAY_LANGUAGE.to_string();
        }

        if self.theme.trim().is_empty() {
            self.theme = DEFAULT_THEME.to_string();
        }
        if self.auto_theme_light.trim().is_empty() {
            self.auto_theme_light = DEFAULT_AUTO_THEME_LIGHT.to_string();
        }
        if self.auto_theme_dark.trim().is_empty() {
            self.auto_theme_dark = DEFAULT_AUTO_THEME_DARK.to_string();
        }

        self.java_home_path = self.java_home_path.trim().to_string();
        self.maven_executable_path = self.maven_executable_path.trim().to_string();
        self.maven_java_home_path = self.maven_java_home_path.trim().to_string();
        self.maven_settings_path = self.maven_settings_path.trim().to_string();
        self.maven_local_repository_path = self.maven_local_repository_path.trim().to_string();

        // 认不出的档位回落默认：手改设置文件不该让外壳拿到一个不存在的布局。
        // 外壳那边读的时候还会再回落一次，这里保证**写回**的值也是干净的。
        if !MENU_BAR_MODES.contains(&self.menu_bar_mode.as_str()) {
            self.menu_bar_mode = MENU_BAR_MODE_DEFAULT.to_string();
        }

        if self.icons.trim().is_empty() {
            self.icons = DEFAULT_ICONS.to_string();
        } else {
            self.icons = self.icons.trim().to_string();
        }

        self.uninstalled_extensions = self
            .uninstalled_extensions
            .iter()
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        // 键位覆盖按 `(preset, command)` 去重（后写的赢）。**这里是 `dedup_overrides` 而不是
        // `KeymapSettings::normalize`** —— 后者会连认不出的命令 id 一起丢掉，而 `normalize()`
        // 会被 `SettingsStore::commit_global` 反复触发：用户每改一个无关设置（调字号、换主题）
        // 都会跑一次，那种频率下删用户数据是副作用不是校验。见 `dedup_overrides` 的注释。
        self.keymap.dedup_overrides();
    }

    pub fn normalize_with_themes(&mut self, index: &ThemeIndex) {
        if index.is_empty() {
            return;
        }
        self.theme = index
            .canonicalize(&self.theme)
            .unwrap_or_else(|| DEFAULT_THEME.to_string());
        self.auto_theme_light = index
            .canonicalize(&self.auto_theme_light)
            .unwrap_or_else(|| DEFAULT_AUTO_THEME_LIGHT.to_string());
        self.auto_theme_dark = index
            .canonicalize(&self.auto_theme_dark)
            .unwrap_or_else(|| DEFAULT_AUTO_THEME_DARK.to_string());
    }

    pub fn effective_theme(&self, system_is_dark: bool) -> &str {
        if !self.sync_system_theme {
            return &self.theme;
        }
        if system_is_dark {
            &self.auto_theme_dark
        } else {
            &self.auto_theme_light
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::{KeySource, KeymapPreset, KeymapSettings};
    use lithe_gpui_shared::commands::CommandId;

    #[test]
    fn defaults_match_the_windows_truth() {
        let settings = Settings::default();
        assert_eq!(settings.theme, "darcula");
        assert!(!settings.sync_system_theme);
        assert_eq!(settings.auto_theme_light, "intellij-light");
        assert_eq!(settings.auto_theme_dark, "darcula");
        assert_eq!(settings.ui_font_size, 13.0);
        assert!(settings.show_status_bar);
        assert_eq!(settings.display_language, "zh-CN");
        assert_eq!(settings.font_size, 14.0);
        assert_eq!(settings.tab_size, 2);
        assert_eq!(settings.terminal_default_shell_id, "");
        assert!(settings.confirm_before_discard);
        assert_eq!(settings.java_home_path, "");
        assert_eq!(settings.maven_executable_path, "");
        assert_eq!(settings.maven_java_home_path, "");
        assert_eq!(settings.maven_settings_path, "");
        assert_eq!(settings.maven_local_repository_path, "");
        assert!(settings.auto_completion);
        assert!(settings.word_wrap);
        assert!(settings.auto_save);
        assert!(settings.ask_where_to_open_projects);
        assert!(settings.open_folders_in_new_window);
        assert_eq!(settings.font_family, "");
        assert_eq!(settings.mono_font_family, DEFAULT_MONO_FONT_FAMILY);
        assert_eq!(settings.terminal_font_size, TERMINAL_FONT_SIZE_UNSET);
    }

    #[test]
    fn terminal_font_size_normalizes_to_unset_or_clamped() {
        assert_eq!(normalize_terminal_font_size(0.0), TERMINAL_FONT_SIZE_UNSET);
        assert_eq!(normalize_terminal_font_size(-5.0), TERMINAL_FONT_SIZE_UNSET);
        assert_eq!(
            normalize_terminal_font_size(f64::NAN),
            TERMINAL_FONT_SIZE_UNSET
        );
        assert_eq!(
            normalize_terminal_font_size(f64::INFINITY),
            TERMINAL_FONT_SIZE_UNSET
        );

        assert_eq!(normalize_terminal_font_size(14.0), 14.0);
        assert_eq!(normalize_terminal_font_size(14.4), 14.0);
        assert_eq!(normalize_terminal_font_size(14.6), 15.0);
        assert_eq!(normalize_terminal_font_size(1.0), TERMINAL_FONT_SIZE_MIN);
        assert_eq!(normalize_terminal_font_size(999.0), TERMINAL_FONT_SIZE_MAX);
    }

    #[test]
    fn font_family_overrides_are_trimmed() {
        let mut settings = Settings {
            font_family: "  Inter \n".to_string(),
            mono_font_family: "\tJetBrains Mono ".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.font_family, "Inter");
        assert_eq!(settings.mono_font_family, "JetBrains Mono");

        let mut blank = Settings {
            font_family: "   ".to_string(),
            mono_font_family: "\n".to_string(),
            ..Settings::default()
        };
        blank.normalize();
        assert_eq!(blank.font_family, "");
        assert_eq!(blank.mono_font_family, "");
    }

    #[test]
    fn missing_keys_fall_back_per_field() {
        let settings: Settings = serde_json::from_str("{}").expect("空对象应当能反序列化");
        assert_eq!(settings, Settings::default());

        let settings: Settings =
            serde_json::from_str(r#"{"uiFontSize": 18.0}"#).expect("部分键应当能反序列化");
        assert_eq!(settings.ui_font_size, 18.0);
        assert_eq!(settings.theme, DEFAULT_THEME);
        assert_eq!(settings.display_language, DEFAULT_DISPLAY_LANGUAGE);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let settings: Settings = serde_json::from_str(
            r#"{"theme": "Lithe Light", "icons": "idea-icons", "nope": {"a": 1}}"#,
        )
        .expect("未知键不该让解析失败");
        assert_eq!(settings.theme, "Lithe Light");
    }

    #[test]
    fn serializes_with_windows_key_names() {
        let json = serde_json::to_string(&Settings::default()).expect("序列化失败");
        for key in [
            "\"theme\"",
            "\"syncSystemTheme\"",
            "\"autoThemeLight\"",
            "\"autoThemeDark\"",
            "\"uiFontSize\"",
            "\"showStatusBar\"",
            "\"displayLanguage\"",
            "\"fontSize\"",
            "\"tabSize\"",
            "\"terminalDefaultShellId\"",
            "\"confirmBeforeDiscard\"",
            "\"javaHomePath\"",
            "\"mavenExecutablePath\"",
            "\"mavenJavaHomePath\"",
            "\"mavenSettingsPath\"",
            "\"mavenLocalRepositoryPath\"",
            "\"autoCompletion\"",
            "\"askWhereToOpenProjects\"",
            "\"openFoldersInNewWindow\"",
            "\"fontFamily\"",
            "\"monoFontFamily\"",
            "\"terminalFontSize\"",
        ] {
            assert!(json.contains(key), "缺少键 {key}：{json}");
        }
    }

    #[test]
    fn toolchain_overrides_are_trimmed() {
        let mut settings = Settings {
            java_home_path: "  D:\\ProgramData\\java\\openjdk-21 \n".to_string(),
            maven_executable_path: "\tD:\\tools\\apache-maven-3.9.9\\bin\\mvn.cmd ".to_string(),
            maven_java_home_path: "   ".to_string(),
            maven_settings_path: " D:\\m2\\settings.xml ".to_string(),
            maven_local_repository_path: "\t \n".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.java_home_path, "D:\\ProgramData\\java\\openjdk-21");
        assert_eq!(
            settings.maven_executable_path,
            "D:\\tools\\apache-maven-3.9.9\\bin\\mvn.cmd"
        );
        assert_eq!(settings.maven_java_home_path, "");
        assert_eq!(settings.maven_settings_path, "D:\\m2\\settings.xml");
        assert_eq!(settings.maven_local_repository_path, "");
        let mut settings = Settings {
            java_home_path: "C:\\Program Files\\Java\\jdk-21".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.java_home_path, "C:\\Program Files\\Java\\jdk-21");
    }

    #[test]
    fn ui_font_size_is_snapped_and_clamped() {
        assert_eq!(normalize_ui_font_size(13.0), 13.0);
        assert_eq!(normalize_ui_font_size(13.24), 13.0);
        assert_eq!(normalize_ui_font_size(13.26), 13.5);
        assert_eq!(normalize_ui_font_size(0.0), UI_FONT_SIZE_MIN);
        assert_eq!(normalize_ui_font_size(999.0), UI_FONT_SIZE_MAX);
        assert_eq!(normalize_ui_font_size(-5.0), UI_FONT_SIZE_MIN);
        assert_eq!(normalize_ui_font_size(f64::NAN), UI_FONT_SIZE_DEFAULT);
        assert_eq!(normalize_ui_font_size(f64::INFINITY), UI_FONT_SIZE_DEFAULT);
        assert_eq!(normalize_ui_font_size(16.5), 16.5);
    }

    #[test]
    fn default_ui_font_size_maps_to_the_theme_rem_base() {
        assert_eq!(theme_font_size_for(13.0), 16.0);
        assert_eq!(theme_font_size_for(19.5), 24.0);
        assert!((theme_font_size_for(99.0) - 16.0 * 24.0 / 13.0).abs() < 1e-4);
        assert!((theme_font_size_for(0.0) - 16.0 * 10.0 / 13.0).abs() < 1e-4);
    }

    #[test]
    fn editor_font_size_is_rounded_and_clamped() {
        assert_eq!(normalize_editor_font_size(14.0), 14.0);
        assert_eq!(normalize_editor_font_size(14.4), 14.0);
        assert_eq!(normalize_editor_font_size(14.6), 15.0);
        assert_eq!(normalize_editor_font_size(10.0), EDITOR_FONT_SIZE_MIN);
        assert_eq!(normalize_editor_font_size(22.0), EDITOR_FONT_SIZE_MAX);
        assert_eq!(normalize_editor_font_size(0.0), EDITOR_FONT_SIZE_MIN);
        assert_eq!(normalize_editor_font_size(999.0), EDITOR_FONT_SIZE_MAX);
        assert_eq!(
            normalize_editor_font_size(f64::NAN),
            EDITOR_FONT_SIZE_DEFAULT
        );
    }

    #[test]
    fn tab_size_and_shell_id_are_whitelisted() {
        for value in TAB_SIZES {
            let mut settings = Settings {
                tab_size: value,
                ..Settings::default()
            };
            settings.normalize();
            assert_eq!(settings.tab_size, value);
        }
        let mut settings = Settings {
            tab_size: 3,
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.tab_size, TAB_SIZE_DEFAULT);

        for id in TERMINAL_SHELL_IDS {
            let mut settings = Settings {
                terminal_default_shell_id: id.to_string(),
                ..Settings::default()
            };
            settings.normalize();
            assert_eq!(settings.terminal_default_shell_id, id);
        }
        let mut settings = Settings {
            terminal_default_shell_id: "fish".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.terminal_default_shell_id, SHELL_SYSTEM_DEFAULT);
    }

    #[test]
    fn display_language_is_whitelisted() {
        let mut settings = Settings {
            display_language: "fr-FR".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.display_language, DEFAULT_DISPLAY_LANGUAGE);

        let mut settings = Settings {
            display_language: "en-US".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.display_language, "en-US");
        assert_eq!(settings.gpui_locale(), "en");
    }

    #[test]
    fn theme_ids_fall_back_to_defaults_and_accept_legacy_names() {
        let mut settings = Settings {
            theme: "  ".to_string(),
            auto_theme_light: String::new(),
            auto_theme_dark: String::new(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.theme, DEFAULT_THEME);
        assert_eq!(settings.auto_theme_light, DEFAULT_AUTO_THEME_LIGHT);
        assert_eq!(settings.auto_theme_dark, DEFAULT_AUTO_THEME_DARK);

        let mut settings = Settings {
            theme: "No Such Theme".to_string(),
            ..Settings::default()
        };
        settings.normalize_with_themes(&ThemeIndex::default());
        assert_eq!(settings.theme, "No Such Theme");
        settings.normalize_with_themes(&index_of(&["lithe-dark"]));
        assert_eq!(settings.theme, DEFAULT_THEME);
    }

    #[test]
    fn effective_theme_respects_sync_flag() {
        let mut settings = Settings {
            theme: "lithe-light".to_string(),
            auto_theme_light: "lithe-light".to_string(),
            auto_theme_dark: "lithe-dark".to_string(),
            sync_system_theme: false,
            ..Settings::default()
        };
        assert_eq!(settings.effective_theme(true), "lithe-light");
        assert_eq!(settings.effective_theme(false), "lithe-light");

        settings.sync_system_theme = true;
        assert_eq!(settings.effective_theme(true), "lithe-dark");
        assert_eq!(settings.effective_theme(false), "lithe-light");
    }

    fn document(id: Option<&str>, name: &str, mode: &str) -> String {
        let id = match id {
            Some(id) => format!(r#""id": "{id}","#),
            None => String::new(),
        };
        format!(r#"{{"name":"test","themes":[{{{id}"name":"{name}","mode":"{mode}"}}]}}"#)
    }

    fn index_of(names: &[&str]) -> ThemeIndex {
        ThemeIndex::from_documents(
            names
                .iter()
                .map(|name| document(None, name, "dark"))
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn theme_index_prefers_the_explicit_id_and_falls_back_to_the_name() {
        let index = ThemeIndex::from_documents(vec![
            document(Some("lithe-dark"), "Lithe Dark", "dark"),
            document(None, "VS Code Light+", "light"),
        ]);

        assert_eq!(index.entries().len(), 2);
        assert_eq!(index.id_for_name("Lithe Dark"), Some("lithe-dark"));
        assert_eq!(index.name_for_id("lithe-dark"), Some("Lithe Dark"));
        assert_eq!(index.id_for_name("VS Code Light+"), Some("vs-code-light"));
        assert!(index.entries()[0].dark);
        assert!(!index.entries()[1].dark, "light 主题不是深色");
        assert_eq!(index.ids(), vec!["lithe-dark", "vs-code-light"]);
    }

    #[test]
    fn canonicalize_accepts_both_ids_and_display_names() {
        let index = ThemeIndex::from_documents(vec![
            document(Some("lithe-dark"), "Lithe Dark", "dark"),
            document(Some("lithe-light"), "Lithe Light", "light"),
            document(Some("darcula"), "Darcula", "dark"),
        ]);

        assert_eq!(
            index.canonicalize("lithe-dark").as_deref(),
            Some("lithe-dark")
        );
        assert_eq!(
            index.canonicalize("  Lithe Dark  ").as_deref(),
            Some("lithe-dark")
        );
        assert_eq!(
            index.canonicalize("lithe LIGHT").as_deref(),
            Some("lithe-light")
        );
        assert_eq!(index.canonicalize("DARCULA").as_deref(), Some("darcula"));
        assert_eq!(
            index.canonicalize("lItHe LiGhT").as_deref(),
            Some("lithe-light")
        );
        assert_eq!(index.canonicalize("No Such Theme"), None);
        assert_eq!(index.canonicalize("   "), None);
    }

    #[test]
    fn normalize_with_themes_rewrites_display_names_to_ids() {
        let index = ThemeIndex::from_documents(vec![
            document(Some("lithe-dark"), "Lithe Dark", "dark"),
            document(Some("lithe-light"), "Lithe Light", "light"),
            document(Some("gruvbox-dark"), "Gruvbox Dark", "dark"),
        ]);

        let mut settings = Settings {
            theme: "Gruvbox Dark".to_string(),
            auto_theme_light: "Lithe Light".to_string(),
            auto_theme_dark: "Lithe Dark".to_string(),
            ..Settings::default()
        };
        settings.normalize_with_themes(&index);
        assert_eq!(settings.theme, "gruvbox-dark");
        assert_eq!(settings.auto_theme_light, "lithe-light");
        assert_eq!(settings.auto_theme_dark, "lithe-dark");

        let mut settings = Settings {
            theme: "Uninstalled Theme".to_string(),
            ..Settings::default()
        };
        settings.normalize_with_themes(&index);
        assert_eq!(settings.theme, DEFAULT_THEME);
    }

    #[test]
    fn theme_index_keeps_the_first_of_each_id_and_name() {
        let index = ThemeIndex::from_documents(vec![
            document(Some("dup"), "First", "dark"),
            document(Some("dup"), "Second", "dark"),
            document(Some("other"), "First", "light"),
            document(None, "   ", "light"),
        ]);
        assert_eq!(index.entries().len(), 1);
        assert_eq!(index.ids(), vec!["dup"]);
        assert_eq!(index.name_for_id("dup"), Some("First"));
    }

    #[test]
    fn a_legacy_document_without_the_keymap_key_gets_the_default() {
        let settings: Settings =
            serde_json::from_str(r#"{"theme": "darcula"}"#).expect("老文档必须还能读出来");
        assert_eq!(settings.keymap, KeymapSettings::default());
        assert_eq!(settings.keymap.preset, KeymapPreset::Lithe);
        assert!(settings.keymap.overrides.is_empty());
    }

    #[test]
    fn the_keymap_section_survives_a_round_trip() {
        let mut settings = Settings::default();
        settings.keymap.preset = KeymapPreset::IdeaClassic;
        settings.keymap.set_override(
            KeymapPreset::IdeaClassic,
            CommandId::ToggleTerminal,
            vec!["alt-f12".to_string()],
        );

        let json = serde_json::to_string(&settings).expect("序列化失败");
        let back: Settings = serde_json::from_str(&json).expect("反序列化失败");
        assert_eq!(back, settings, "带 keymap 的文档 round-trip 必须不变");
        assert_eq!(
            back.keymap.resolve(CommandId::ToggleTerminal).keystrokes,
            vec!["alt-f12".to_string()]
        );
    }

    #[test]
    fn duplicate_overrides_collapse_to_the_last_one() {
        let json = r#"{
          "keymap": {
            "preset": "lithe",
            "overrides": [
              {"preset": "eclipse", "command": "save-buffer", "keystrokes": ["secondary-a"]},
              {"preset": "eclipse", "command": "save-buffer", "keystrokes": ["secondary-b"]}
            ]
          }
        }"#;
        let mut settings: Settings = serde_json::from_str(json).expect("解析失败");
        settings.normalize();

        assert_eq!(
            settings.keymap.overrides.len(),
            1,
            "同一 (preset, command) 只留一条"
        );
        assert_eq!(
            settings.keymap.overrides[0].keystrokes,
            vec!["secondary-b".to_string()],
            "后写的赢（与「设置页先删旧的再追加」一致）"
        );
    }

    #[test]
    fn an_unknown_command_id_survives_normalization() {
        // 全局设置文件不删用户数据：这一版的 `CommandId` 认不出某个 id，不代表它永远是错的
        // （降级、改 id 都会遇到）。认不出的覆盖对解析无害，将来命令回来了它还在。
        let json = r#"{
          "keymap": {
            "preset": "lithe",
            "overrides": [
              {"preset": "lithe", "command": "ghost-command", "keystrokes": ["secondary-y"]}
            ]
          }
        }"#;
        let mut settings: Settings = serde_json::from_str(json).expect("解析失败");
        settings.normalize();

        assert_eq!(
            settings.keymap.overrides.len(),
            1,
            "认不出的 id 不许被静默删掉"
        );
        assert_eq!(settings.keymap.overrides[0].command, "ghost-command");
        assert_eq!(
            settings.keymap.resolve(CommandId::SaveBuffer).keystrokes,
            vec!["secondary-s".to_string()],
            "未知覆盖不干扰已知命令"
        );
    }

    #[test]
    fn an_empty_override_stays_empty_through_a_round_trip() {
        // 数据层那条硬约束在持久化层的验证：空 `keystrokes` =「这条命令在此预设下没有键位」，
        // 存下来读回去必须还是空的，**不许被默认值回填**（macOS 端 Eclipse 档 `Find` 为空、
        // 不能回退成 ⌘F 就是这条规则）。
        let mut settings = Settings::default();
        settings
            .keymap
            .set_override(KeymapPreset::Lithe, CommandId::SaveBuffer, Vec::new());

        let json = serde_json::to_string(&settings).expect("序列化失败");
        assert!(
            json.contains(r#""keystrokes":[]"#),
            "空列表要写成 [] 而不是被省略：{json}"
        );

        let mut back: Settings = serde_json::from_str(&json).expect("反序列化失败");
        back.normalize();

        let entry = back
            .keymap
            .override_for(KeymapPreset::Lithe, CommandId::SaveBuffer)
            .expect("空覆盖必须还在，不能被当成「没设置」丢掉");
        assert!(entry.keystrokes.is_empty(), "读回来必须还是空的");

        let resolved = back.keymap.resolve(CommandId::SaveBuffer);
        assert!(
            resolved.keystrokes.is_empty(),
            "不许回落到默认的 secondary-s：{resolved:?}"
        );
        assert_eq!(resolved.source, KeySource::User, "空覆盖也是用户意图");
    }

    #[test]
    fn clearing_an_override_falls_back_to_the_layer_below_it() {
        // 「恢复默认」的语义是撤掉覆盖、让这条命令回落到预设表或默认值，**不是**把键位清空。
        // 所以断言必须走到合成逻辑（`resolve`）而不是只看 `overrides` 数组变短了。
        let mut settings = Settings::default();
        settings.keymap.preset = KeymapPreset::IdeaClassic;
        settings.keymap.set_override(
            KeymapPreset::IdeaClassic,
            CommandId::ToggleTerminal,
            vec!["secondary-k".to_string()],
        );
        assert_eq!(
            settings.keymap.resolve(CommandId::ToggleTerminal).source,
            KeySource::User
        );

        // 预设表里有这一条（IDEA Classic 给 `alt-f12`）→ 清掉覆盖后回到预设层。
        settings
            .keymap
            .clear_override(KeymapPreset::IdeaClassic, CommandId::ToggleTerminal);
        let resolved = settings.keymap.resolve(CommandId::ToggleTerminal);
        assert_eq!(resolved.keystrokes, vec!["alt-f12".to_string()]);
        assert_eq!(resolved.source, KeySource::Preset);

        // 预设表里没有的那一条 → 回到注册表默认值，而不是变成「没有键位」。
        settings.keymap.set_override(
            KeymapPreset::IdeaClassic,
            CommandId::SaveBuffer,
            vec!["secondary-q".to_string()],
        );
        settings
            .keymap
            .clear_override(KeymapPreset::IdeaClassic, CommandId::SaveBuffer);
        let resolved = settings.keymap.resolve(CommandId::SaveBuffer);
        assert_eq!(resolved.keystrokes, vec!["secondary-s".to_string()]);
        assert_eq!(resolved.source, KeySource::Default);
        assert!(
            resolved.is_bound(),
            "清掉覆盖是回落到下一层，不是让命令变成未绑定"
        );
    }

    #[test]
    fn slugify_folds_punctuation_and_never_returns_empty() {
        assert_eq!(slugify_theme_id("Lithe Dark"), "lithe-dark");
        assert_eq!(slugify_theme_id("VS Code Light+"), "vs-code-light");
        assert_eq!(slugify_theme_id("  One   Dark  "), "one-dark");
        assert_eq!(slugify_theme_id("gruvbox-dark"), "gruvbox-dark");
        assert_eq!(slugify_theme_id("深色主题"), "深色主题");
        assert_eq!(slugify_theme_id("+++"), "theme");
        assert_eq!(slugify_theme_id(""), "theme");
    }
}
