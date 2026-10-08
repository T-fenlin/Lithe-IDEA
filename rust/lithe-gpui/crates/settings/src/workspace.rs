use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::schema::Settings;

pub const WORKSPACE_SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppearanceKey {
    Theme,
    SyncSystemTheme,
    AutoThemeLight,
    AutoThemeDark,
    UiFontSize,
    EditorFontSize,
    FontFamily,
    MonoFontFamily,
    TerminalFontSize,
}

impl AppearanceKey {
    pub const ALL: [AppearanceKey; 9] = [
        AppearanceKey::Theme,
        AppearanceKey::SyncSystemTheme,
        AppearanceKey::AutoThemeLight,
        AppearanceKey::AutoThemeDark,
        AppearanceKey::UiFontSize,
        AppearanceKey::EditorFontSize,
        AppearanceKey::FontFamily,
        AppearanceKey::MonoFontFamily,
        AppearanceKey::TerminalFontSize,
    ];

    pub fn json_key(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::SyncSystemTheme => "syncSystemTheme",
            Self::AutoThemeLight => "autoThemeLight",
            Self::AutoThemeDark => "autoThemeDark",
            Self::UiFontSize => "uiFontSize",
            Self::EditorFontSize => "fontSize",
            Self::FontFamily => "fontFamily",
            Self::MonoFontFamily => "monoFontFamily",
            Self::TerminalFontSize => "terminalFontSize",
        }
    }

    pub fn as_str(self) -> &'static str {
        self.json_key()
    }

    pub fn from_json_key(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|key| key.json_key() == value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppearanceValue {
    Text(String),
    Number(f64),
    Flag(bool),
}

impl AppearanceValue {
    pub fn from_settings(settings: &Settings, key: AppearanceKey) -> Self {
        match key {
            AppearanceKey::Theme => Self::Text(settings.theme.clone()),
            AppearanceKey::SyncSystemTheme => Self::Flag(settings.sync_system_theme),
            AppearanceKey::AutoThemeLight => Self::Text(settings.auto_theme_light.clone()),
            AppearanceKey::AutoThemeDark => Self::Text(settings.auto_theme_dark.clone()),
            AppearanceKey::UiFontSize => Self::Number(settings.ui_font_size),
            AppearanceKey::EditorFontSize => Self::Number(settings.font_size),
            AppearanceKey::FontFamily => Self::Text(settings.font_family.clone()),
            AppearanceKey::MonoFontFamily => Self::Text(settings.mono_font_family.clone()),
            AppearanceKey::TerminalFontSize => Self::Number(settings.terminal_font_size),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceOverlay {
    #[serde(rename = "theme")]
    pub theme: Option<String>,
    #[serde(rename = "syncSystemTheme")]
    pub sync_system_theme: Option<bool>,
    #[serde(rename = "autoThemeLight")]
    pub auto_theme_light: Option<String>,
    #[serde(rename = "autoThemeDark")]
    pub auto_theme_dark: Option<String>,
    #[serde(rename = "uiFontSize")]
    pub ui_font_size: Option<f64>,
    #[serde(rename = "fontSize")]
    pub font_size: Option<f64>,
    #[serde(rename = "fontFamily")]
    pub font_family: Option<String>,
    #[serde(rename = "monoFontFamily")]
    pub mono_font_family: Option<String>,
    #[serde(rename = "terminalFontSize")]
    pub terminal_font_size: Option<f64>,
}

impl AppearanceOverlay {
    pub fn has(&self, key: AppearanceKey) -> bool {
        self.get(key).is_some()
    }

    pub fn get(&self, key: AppearanceKey) -> Option<AppearanceValue> {
        match key {
            AppearanceKey::Theme => self.theme.clone().map(AppearanceValue::Text),
            AppearanceKey::SyncSystemTheme => self.sync_system_theme.map(AppearanceValue::Flag),
            AppearanceKey::AutoThemeLight => {
                self.auto_theme_light.clone().map(AppearanceValue::Text)
            }
            AppearanceKey::AutoThemeDark => self.auto_theme_dark.clone().map(AppearanceValue::Text),
            AppearanceKey::UiFontSize => self.ui_font_size.map(AppearanceValue::Number),
            AppearanceKey::EditorFontSize => self.font_size.map(AppearanceValue::Number),
            AppearanceKey::FontFamily => self.font_family.clone().map(AppearanceValue::Text),
            AppearanceKey::MonoFontFamily => {
                self.mono_font_family.clone().map(AppearanceValue::Text)
            }
            AppearanceKey::TerminalFontSize => self.terminal_font_size.map(AppearanceValue::Number),
        }
    }

    pub fn set(&mut self, key: AppearanceKey, value: AppearanceValue) {
        match (key, value) {
            (AppearanceKey::Theme, AppearanceValue::Text(value)) => self.theme = Some(value),
            (AppearanceKey::SyncSystemTheme, AppearanceValue::Flag(value)) => {
                self.sync_system_theme = Some(value)
            }
            (AppearanceKey::AutoThemeLight, AppearanceValue::Text(value)) => {
                self.auto_theme_light = Some(value)
            }
            (AppearanceKey::AutoThemeDark, AppearanceValue::Text(value)) => {
                self.auto_theme_dark = Some(value)
            }
            (AppearanceKey::UiFontSize, AppearanceValue::Number(value)) => {
                self.ui_font_size = Some(value)
            }
            (AppearanceKey::EditorFontSize, AppearanceValue::Number(value)) => {
                self.font_size = Some(value)
            }
            (AppearanceKey::FontFamily, AppearanceValue::Text(value)) => {
                self.font_family = Some(value)
            }
            (AppearanceKey::MonoFontFamily, AppearanceValue::Text(value)) => {
                self.mono_font_family = Some(value)
            }
            (AppearanceKey::TerminalFontSize, AppearanceValue::Number(value)) => {
                self.terminal_font_size = Some(value)
            }
            _ => {}
        }
    }

    pub fn clear(&mut self, key: AppearanceKey) {
        match key {
            AppearanceKey::Theme => self.theme = None,
            AppearanceKey::SyncSystemTheme => self.sync_system_theme = None,
            AppearanceKey::AutoThemeLight => self.auto_theme_light = None,
            AppearanceKey::AutoThemeDark => self.auto_theme_dark = None,
            AppearanceKey::UiFontSize => self.ui_font_size = None,
            AppearanceKey::EditorFontSize => self.font_size = None,
            AppearanceKey::FontFamily => self.font_family = None,
            AppearanceKey::MonoFontFamily => self.mono_font_family = None,
            AppearanceKey::TerminalFontSize => self.terminal_font_size = None,
        }
    }

    pub fn apply_to(&self, settings: &mut Settings) -> Vec<AppearanceKey> {
        let mut applied = Vec::new();
        for key in AppearanceKey::ALL {
            let Some(value) = self.get(key) else {
                continue;
            };
            match (key, value) {
                (AppearanceKey::Theme, AppearanceValue::Text(value)) => settings.theme = value,
                (AppearanceKey::SyncSystemTheme, AppearanceValue::Flag(value)) => {
                    settings.sync_system_theme = value
                }
                (AppearanceKey::AutoThemeLight, AppearanceValue::Text(value)) => {
                    settings.auto_theme_light = value
                }
                (AppearanceKey::AutoThemeDark, AppearanceValue::Text(value)) => {
                    settings.auto_theme_dark = value
                }
                (AppearanceKey::UiFontSize, AppearanceValue::Number(value)) => {
                    settings.ui_font_size = value
                }
                (AppearanceKey::EditorFontSize, AppearanceValue::Number(value)) => {
                    settings.font_size = value
                }
                (AppearanceKey::FontFamily, AppearanceValue::Text(value)) => {
                    settings.font_family = value
                }
                (AppearanceKey::MonoFontFamily, AppearanceValue::Text(value)) => {
                    settings.mono_font_family = value
                }
                (AppearanceKey::TerminalFontSize, AppearanceValue::Number(value)) => {
                    settings.terminal_font_size = value
                }
                _ => continue,
            }
            applied.push(key);
        }
        applied
    }

    pub fn clamp_with(&mut self, settings: &Settings) {
        for key in AppearanceKey::ALL {
            if self.has(key) {
                self.set(key, AppearanceValue::from_settings(settings, key));
            }
        }
    }

    fn unset_json_keys(&self) -> Vec<&'static str> {
        AppearanceKey::ALL
            .iter()
            .filter(|key| !self.has(**key))
            .map(|key| key.json_key())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceSource {
    Global,
    ProjectShared { tracked: bool },
    ProjectLocal,
}

impl AppearanceSource {
    pub fn locale_key(self) -> &'static str {
        match self {
            Self::Global => "settings.gpui.appearanceSourceGlobal",
            Self::ProjectShared { tracked: true } => "settings.gpui.appearanceSourceTeam",
            Self::ProjectShared { tracked: false } => "settings.gpui.appearanceSourceProject",
            Self::ProjectLocal => "settings.gpui.appearanceSourceLocal",
        }
    }

    pub fn is_workspace_override(self) -> bool {
        !matches!(self, Self::Global)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppearanceKeySource {
    pub key: AppearanceKey,
    pub source: AppearanceSource,
}

pub fn resolve_effective(
    global: &Settings,
    shared: &AppearanceOverlay,
    local: &AppearanceOverlay,
) -> Settings {
    let mut effective = global.clone();
    shared.apply_to(&mut effective);
    local.apply_to(&mut effective);
    effective.normalize();
    effective
}

pub fn source_for(
    key: AppearanceKey,
    shared: &AppearanceOverlay,
    local: &AppearanceOverlay,
    shared_tracked: Option<bool>,
) -> AppearanceSource {
    if local.has(key) {
        return AppearanceSource::ProjectLocal;
    }
    if shared.has(key) {
        return AppearanceSource::ProjectShared {
            tracked: shared_tracked.unwrap_or(false),
        };
    }
    AppearanceSource::Global
}

pub fn appearance_sources(
    shared: &AppearanceOverlay,
    local: &AppearanceOverlay,
    shared_tracked: Option<bool>,
) -> Vec<AppearanceKeySource> {
    AppearanceKey::ALL
        .iter()
        .map(|key| AppearanceKeySource {
            key: *key,
            source: source_for(*key, shared, local, shared_tracked),
        })
        .collect()
}

pub fn effective_theme_key(settings: &Settings, system_is_dark: bool) -> AppearanceKey {
    if !settings.sync_system_theme {
        return AppearanceKey::Theme;
    }
    if system_is_dark {
        AppearanceKey::AutoThemeDark
    } else {
        AppearanceKey::AutoThemeLight
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedOverlay {
    pub overlay: AppearanceOverlay,
    pub previous: Option<Value>,
    pub read_only: bool,
    pub file_existed: bool,
    pub diagnostics: Vec<String>,
}

pub fn load_overlay(path: &Path) -> LoadedOverlay {
    match lithe_gpui_shared::read_document_text(path) {
        Ok(Some(text)) => {
            let parsed =
                lithe_gpui_shared::parse::<AppearanceOverlay>(&text, WORKSPACE_SETTINGS_VERSION);
            LoadedOverlay {
                overlay: parsed.value,
                previous: parsed.previous,
                read_only: parsed.read_only,
                file_existed: true,
                diagnostics: parsed.diagnostics,
            }
        }
        Ok(None) => LoadedOverlay {
            overlay: AppearanceOverlay::default(),
            previous: None,
            read_only: false,
            file_existed: false,
            diagnostics: Vec::new(),
        },
        Err(error) => LoadedOverlay {
            overlay: AppearanceOverlay::default(),
            previous: None,
            read_only: false,
            file_existed: false,
            diagnostics: vec![format!("workspace_settings_read_failed error={error}")],
        },
    }
}

pub fn merged_document(
    previous: Option<&Value>,
    overlay: &AppearanceOverlay,
) -> Result<Value, serde_json::Error> {
    let typed = serde_json::to_value(overlay)?;
    let mut merged = lithe_gpui_shared::preserve_unknown(previous, &typed);
    if let Value::Object(object) = &mut merged {
        for key in overlay.unset_json_keys() {
            object.remove(key);
        }
        object.insert(
            lithe_gpui_shared::DOCUMENT_VERSION_KEY.to_string(),
            Value::from(WORKSPACE_SETTINGS_VERSION),
        );
    }
    Ok(merged)
}

pub fn write_overlay(
    path: &Path,
    previous: Option<&Value>,
    overlay: &AppearanceOverlay,
) -> io::Result<Value> {
    let document = merged_document(previous, overlay)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    lithe_gpui_shared::save_json(path, &document)?;
    Ok(document)
}

pub fn overlay_known_keys() -> std::collections::BTreeSet<String> {
    lithe_gpui_shared::known_keys::<AppearanceOverlay>()
}

pub fn describe_keys(keys: &[AppearanceKey]) -> String {
    keys.iter()
        .map(|key| key.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

pub fn keys_in_document(object: &Map<String, Value>) -> Vec<AppearanceKey> {
    AppearanceKey::ALL
        .iter()
        .copied()
        .filter(|key| object.contains_key(key.json_key()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{EDITOR_FONT_SIZE_MAX, TERMINAL_FONT_SIZE_UNSET, UI_FONT_SIZE_MAX};

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("lithe-ws-settings-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn global_settings() -> Settings {
        let mut settings = Settings::default();
        settings.theme = "lithe-light".to_string();
        settings.sync_system_theme = false;
        settings.auto_theme_light = "lithe-light".to_string();
        settings.auto_theme_dark = "lithe-dark".to_string();
        settings.ui_font_size = 13.0;
        settings.font_size = 14.0;
        settings.font_family = "Global UI".to_string();
        settings.mono_font_family = "Global Mono".to_string();
        settings.terminal_font_size = 0.0;
        settings.display_language = "en-US".to_string();
        settings.tab_size = 8;
        settings.java_home_path = "D:\\global-jdk".to_string();
        settings.normalize();
        settings
    }

    #[test]
    fn precedence_is_global_then_shared_then_local() {
        let global = global_settings();

        let mut shared = AppearanceOverlay::default();
        shared.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(18.0));
        shared.set(
            AppearanceKey::FontFamily,
            AppearanceValue::Text("Shared UI".to_string()),
        );
        let mut local = AppearanceOverlay::default();
        local.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(20.0));

        let effective = resolve_effective(&global, &shared, &local);
        assert_eq!(effective.font_size, 20.0, "本机层赢");
        assert_eq!(
            effective.font_family, "Shared UI",
            "本机层没设的键由共享层决定"
        );
        assert_eq!(effective.theme, global.theme, "两层都没设 → 回落到全局");
        assert_eq!(effective.ui_font_size, global.ui_font_size);
    }

    #[test]
    fn clearing_a_layer_key_falls_back_to_the_global_value() {
        let global = global_settings();
        let mut shared = AppearanceOverlay::default();
        shared.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(18.0));
        let mut local = AppearanceOverlay::default();
        local.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(20.0));

        assert_eq!(resolve_effective(&global, &shared, &local).font_size, 20.0);

        local.clear(AppearanceKey::EditorFontSize);
        assert_eq!(
            resolve_effective(&global, &shared, &local).font_size,
            18.0,
            "清掉本机层 → 回落到共享层"
        );

        shared.clear(AppearanceKey::EditorFontSize);
        assert_eq!(
            resolve_effective(&global, &shared, &local).font_size,
            global.font_size,
            "两层都清掉 → 回落到全局值（不是空值、不是默认值）"
        );
    }

    #[test]
    fn non_appearance_keys_are_not_overridable() {
        let global = global_settings();
        let text = r#"{
          "fontSize": 18,
          "displayLanguage": "zh-CN",
          "tabSize": 2,
          "javaHomePath": "D:\\project-jdk",
          "terminalDefaultShellId": "cmd",
          "mavenSettingsPath": "D:\\project-settings.xml"
        }"#;
        let parsed =
            lithe_gpui_shared::parse::<AppearanceOverlay>(text, WORKSPACE_SETTINGS_VERSION);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let effective = resolve_effective(&global, &parsed.value, &AppearanceOverlay::default());

        assert_eq!(effective.font_size, 18.0, "外观键生效");
        assert_eq!(
            effective.display_language, global.display_language,
            "语言不是外观键，仍然只有全局层"
        );
        assert_eq!(effective.tab_size, global.tab_size, "缩进只有全局层");
        assert_eq!(
            effective.java_home_path, global.java_home_path,
            "工具链只有全局层"
        );
        assert_eq!(
            effective.terminal_default_shell_id, global.terminal_default_shell_id,
            "终端 shell 只有全局层"
        );
        assert_eq!(
            effective.maven_settings_path, global.maven_settings_path,
            "Maven 路径只有全局层"
        );
    }

    #[test]
    fn clamping_is_reused_from_the_settings_normalizer() {
        let global = global_settings();
        let mut overlay = AppearanceOverlay::default();
        overlay.set(AppearanceKey::UiFontSize, AppearanceValue::Number(99.0));
        overlay.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(99.0));
        overlay.set(
            AppearanceKey::TerminalFontSize,
            AppearanceValue::Number(-3.0),
        );

        let effective = resolve_effective(&global, &overlay, &AppearanceOverlay::default());
        assert_eq!(effective.ui_font_size, UI_FONT_SIZE_MAX);
        assert_eq!(effective.font_size, EDITOR_FONT_SIZE_MAX);
        assert_eq!(
            effective.terminal_font_size, TERMINAL_FONT_SIZE_UNSET,
            "负数必须归一到「不覆盖」，而不是被夹到最小字号"
        );

        let mut clamped = overlay.clone();
        clamped.clamp_with(&effective);
        assert_eq!(
            clamped.get(AppearanceKey::UiFontSize),
            Some(AppearanceValue::Number(UI_FONT_SIZE_MAX))
        );
        assert_eq!(
            clamped.get(AppearanceKey::TerminalFontSize),
            Some(AppearanceValue::Number(TERMINAL_FONT_SIZE_UNSET))
        );
    }

    #[test]
    fn sources_report_the_highest_priority_layer() {
        let mut shared = AppearanceOverlay::default();
        shared.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(18.0));
        let mut local = AppearanceOverlay::default();
        local.set(
            AppearanceKey::FontFamily,
            AppearanceValue::Text("Local".to_string()),
        );

        assert_eq!(
            source_for(AppearanceKey::EditorFontSize, &shared, &local, Some(false)),
            AppearanceSource::ProjectShared { tracked: false }
        );
        assert_eq!(
            source_for(AppearanceKey::EditorFontSize, &shared, &local, Some(true)),
            AppearanceSource::ProjectShared { tracked: true }
        );
        assert_eq!(
            source_for(AppearanceKey::FontFamily, &shared, &local, Some(true)),
            AppearanceSource::ProjectLocal,
            "本机层优先于共享层，与它对不对得上 Git 无关"
        );
        assert_eq!(
            source_for(AppearanceKey::UiFontSize, &shared, &local, Some(false)),
            AppearanceSource::Global
        );
        assert_eq!(
            source_for(AppearanceKey::EditorFontSize, &shared, &local, None),
            AppearanceSource::ProjectShared { tracked: false }
        );

        let sources = appearance_sources(&shared, &local, Some(true));
        assert_eq!(sources.len(), AppearanceKey::ALL.len());
        assert!(sources[0].source.is_workspace_override() == false);
    }

    #[test]
    fn the_three_sources_have_distinct_locale_keys() {
        let keys = [
            AppearanceSource::Global.locale_key(),
            AppearanceSource::ProjectShared { tracked: true }.locale_key(),
            AppearanceSource::ProjectShared { tracked: false }.locale_key(),
            AppearanceSource::ProjectLocal.locale_key(),
        ];
        let unique: std::collections::BTreeSet<&str> = keys.iter().copied().collect();
        assert_eq!(unique.len(), keys.len(), "四种来源的文案键必须互不相同");
        assert_ne!(
            AppearanceSource::ProjectShared { tracked: true }.locale_key(),
            AppearanceSource::ProjectShared { tracked: false }.locale_key()
        );
    }

    #[test]
    fn the_decisive_theme_key_follows_the_sync_flag() {
        let mut settings = Settings::default();
        settings.sync_system_theme = false;
        assert_eq!(effective_theme_key(&settings, true), AppearanceKey::Theme);
        settings.sync_system_theme = true;
        assert_eq!(
            effective_theme_key(&settings, true),
            AppearanceKey::AutoThemeDark
        );
        assert_eq!(
            effective_theme_key(&settings, false),
            AppearanceKey::AutoThemeLight
        );
    }

    #[test]
    fn json_keys_come_from_the_overlay_schema() {
        let known = overlay_known_keys();
        for key in AppearanceKey::ALL {
            assert!(
                known.contains(key.json_key()),
                "{} 不在 AppearanceOverlay 的键集里（改了 serde rename 却忘了改 json_key？）",
                key.json_key()
            );
            assert_eq!(
                AppearanceKey::from_json_key(key.json_key()),
                Some(key),
                "反查必须回到同一个键"
            );
        }
        assert_eq!(known.len(), AppearanceKey::ALL.len(), "键集规模必须相等");
        assert_eq!(AppearanceKey::from_json_key("displayLanguage"), None);
    }

    #[test]
    fn a_layer_round_trips_through_the_file() {
        let dir = temp_dir("round-trip");
        let path = dir.join("settings.json");

        let mut overlay = AppearanceOverlay::default();
        overlay.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(18.0));
        overlay.set(
            AppearanceKey::MonoFontFamily,
            AppearanceValue::Text("JetBrains Mono".to_string()),
        );
        let document = write_overlay(&path, None, &overlay).expect("写入失败");
        assert!(
            !lithe_gpui_shared::tmp_path(&path).exists(),
            "临时文件必须被 rename 掉"
        );

        let loaded = load_overlay(&path);
        assert!(loaded.file_existed);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(loaded.overlay, overlay);
        assert_eq!(
            document.get(lithe_gpui_shared::DOCUMENT_VERSION_KEY),
            Some(&Value::from(WORKSPACE_SETTINGS_VERSION))
        );

        let mut next = loaded.overlay.clone();
        next.clear(AppearanceKey::MonoFontFamily);
        let document = write_overlay(&path, loaded.previous.as_ref(), &next).expect("覆盖写失败");
        assert!(
            document.get("monoFontFamily").is_none(),
            "清掉的键必须从文档里删掉，而不是留一个 null：{document}"
        );
        assert_eq!(load_overlay(&path).overlay, next);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_keys_survive_a_layer_write() {
        let dir = temp_dir("preserve");
        let path = dir.join("settings.json");
        std::fs::create_dir_all(&dir).expect("建目录失败");
        std::fs::write(
            &path,
            r#"{
  "version": 1,
  "fontSize": 18,
  "thirdParty": { "keepMe": [1, 2, 3], "alsoKeep": "yes" },
  "futureKey": true
}"#,
        )
        .expect("写初始文件失败");

        let loaded = load_overlay(&path);
        assert_eq!(
            loaded.overlay.get(AppearanceKey::EditorFontSize),
            Some(AppearanceValue::Number(18.0))
        );

        let mut next = loaded.overlay.clone();
        next.set(AppearanceKey::UiFontSize, AppearanceValue::Number(15.0));
        let document = write_overlay(&path, loaded.previous.as_ref(), &next).expect("写入失败");

        assert_eq!(
            document.get("thirdParty"),
            Some(&serde_json::json!({ "keepMe": [1, 2, 3], "alsoKeep": "yes" })),
            "未知的嵌套对象必须整体保留：{document}"
        );
        assert_eq!(document.get("futureKey"), Some(&Value::from(true)));
        assert_eq!(document.get("fontSize"), Some(&Value::from(18.0)));
        assert_eq!(document.get("uiFontSize"), Some(&Value::from(15.0)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_single_broken_key_falls_back_alone() {
        let parsed = lithe_gpui_shared::parse::<AppearanceOverlay>(
            r#"{"fontSize": 18, "uiFontSize": "big", "theme": "lithe-light"}"#,
            WORKSPACE_SETTINGS_VERSION,
        );
        assert_eq!(
            parsed.value.get(AppearanceKey::EditorFontSize),
            Some(AppearanceValue::Number(18.0))
        );
        assert_eq!(
            parsed.value.get(AppearanceKey::Theme),
            Some(AppearanceValue::Text("lithe-light".to_string()))
        );
        assert_eq!(parsed.value.get(AppearanceKey::UiFontSize), None);
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|line| line.contains("bad_key key=uiFontSize")),
            "{:?}",
            parsed.diagnostics
        );
    }

    #[test]
    fn reading_a_missing_or_broken_file_degrades_safely() {
        let dir = temp_dir("missing");
        let path = dir.join("settings.json");
        let loaded = load_overlay(&path);
        assert!(!loaded.file_existed);
        assert_eq!(loaded.overlay, AppearanceOverlay::default());
        assert!(!path.exists(), "读取不得创建文件");

        std::fs::create_dir_all(&dir).expect("建目录失败");
        std::fs::write(&path, "{ not json ").expect("写文件失败");
        let loaded = load_overlay(&path);
        assert!(loaded.file_existed);
        assert_eq!(loaded.overlay, AppearanceOverlay::default());
        assert!(!loaded.diagnostics.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_newer_document_version_is_read_only() {
        let parsed = lithe_gpui_shared::parse::<AppearanceOverlay>(
            r#"{"version": 99, "fontSize": 18}"#,
            WORKSPACE_SETTINGS_VERSION,
        );
        assert!(parsed.read_only);
        assert_eq!(
            parsed.value.get(AppearanceKey::EditorFontSize),
            Some(AppearanceValue::Number(18.0))
        );
        assert!(
            !lithe_gpui_shared::parse::<AppearanceOverlay>(
                r#"{"version": 1}"#,
                WORKSPACE_SETTINGS_VERSION
            )
            .read_only
        );
    }

    #[test]
    fn keys_in_the_document_are_reported_in_ui_order() {
        let Value::Object(object) = serde_json::json!({
            "fontSize": 18,
            "theme": "lithe-light",
            "unknownKey": 1
        }) else {
            panic!("必须是对象");
        };
        assert_eq!(describe_keys(&keys_in_document(&object)), "theme,fontSize");
    }
}
