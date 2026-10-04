use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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

    #[serde(rename = "autoSave")]
    pub auto_save: bool,

    #[serde(rename = "askWhereToOpenProjects")]
    pub ask_where_to_open_projects: bool,

    #[serde(rename = "openFoldersInNewWindow")]
    pub open_folders_in_new_window: bool,

    #[serde(rename = "icons")]
    pub icons: String,

    #[serde(rename = "uninstalledExtensions")]
    pub uninstalled_extensions: Vec<String>,
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
            auto_save: true,
            ask_where_to_open_projects: true,
            open_folders_in_new_window: true,
            icons: DEFAULT_ICONS.to_string(),
            uninstalled_extensions: Vec::new(),
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
