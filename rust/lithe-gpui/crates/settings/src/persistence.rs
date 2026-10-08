use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::schema::Settings;

pub use lithe_gpui_shared::{
    DOCUMENT_VERSION_KEY, declared_version, preserve_unknown, save_json, tmp_path,
};

pub const SAVE_DEBOUNCE_MS: u64 = 300;

pub const DOCUMENT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub settings: Settings,
    pub previous: Option<Value>,
    pub read_only: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub settings: Settings,
    pub path: Option<PathBuf>,
    pub file_existed: bool,
    pub diagnostics: Vec<String>,
    pub previous: Option<Value>,
    pub read_only: bool,
}

pub fn load() -> Loaded {
    let path = crate::paths::settings_file_path();
    load_from(path)
}

pub fn load_from(path: Option<PathBuf>) -> Loaded {
    let Some(path) = path else {
        return Loaded {
            settings: normalized_default(),
            path: None,
            file_existed: false,
            diagnostics: vec![
                "no_settings_path (推导不出设置文件路径，本次会话不落盘)".to_string(),
            ],
            previous: None,
            read_only: false,
        };
    };

    match lithe_gpui_shared::read_document_text(&path) {
        Ok(Some(text)) => {
            let parsed = parse(&text);
            Loaded {
                settings: parsed.settings,
                path: Some(path),
                file_existed: true,
                diagnostics: parsed.diagnostics,
                previous: parsed.previous,
                read_only: parsed.read_only,
            }
        }
        Ok(None) => Loaded {
            settings: normalized_default(),
            path: Some(path),
            file_existed: false,
            diagnostics: Vec::new(),
            previous: None,
            read_only: false,
        },
        Err(error) => Loaded {
            settings: normalized_default(),
            path: Some(path),
            file_existed: false,
            diagnostics: vec![format!("read_failed error={error}")],
            previous: None,
            read_only: false,
        },
    }
}

pub fn parse(text: &str) -> Parsed {
    let parsed = lithe_gpui_shared::parse::<Settings>(text, DOCUMENT_VERSION);
    let mut settings = parsed.value;
    settings.normalize();
    Parsed {
        settings,
        previous: parsed.previous,
        read_only: parsed.read_only,
        diagnostics: parsed.diagnostics,
    }
}

pub fn known_keys() -> BTreeSet<String> {
    lithe_gpui_shared::known_keys::<Settings>()
}

pub fn merge_document(
    previous: Option<&Value>,
    settings: &Settings,
) -> Result<Value, serde_json::Error> {
    lithe_gpui_shared::merge_document(previous, settings, DOCUMENT_VERSION)
}

pub fn save_document(
    path: &Path,
    settings: &Settings,
    previous: Option<&Value>,
) -> io::Result<usize> {
    lithe_gpui_shared::save_document(path, settings, previous, DOCUMENT_VERSION)
}

pub fn previous_object(path: &Path) -> Option<Value> {
    lithe_gpui_shared::previous_object(path)
}

fn normalized_default() -> Settings {
    let mut settings = Settings::default();
    settings.normalize();
    settings
}

#[derive(Debug, Default, Clone)]
pub struct DebounceState {
    revision: u64,
    flushed: u64,
}

impl DebounceState {
    pub fn arm(&mut self) -> u64 {
        self.revision += 1;
        self.revision
    }

    pub fn should_flush(&mut self, armed: u64) -> bool {
        if armed != self.revision || self.flushed >= armed {
            return false;
        }
        self.flushed = armed;
        true
    }

    pub fn has_pending(&self) -> bool {
        self.flushed < self.revision
    }

    pub fn mark_flushed(&mut self) {
        self.flushed = self.revision;
    }
}

pub fn normalize_pattern_lines(text: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            out.push(trimmed.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        DEFAULT_AUTO_THEME_DARK, DEFAULT_AUTO_THEME_LIGHT, DEFAULT_DISPLAY_LANGUAGE, DEFAULT_THEME,
        TAB_SIZE_DEFAULT, UI_FONT_SIZE_DEFAULT, UI_FONT_SIZE_MAX, UI_FONT_SIZE_MIN,
    };

    fn all_non_default() -> Settings {
        Settings {
            theme: "Lithe Light".to_string(),
            sync_system_theme: true,
            auto_theme_light: "Lithe Light".to_string(),
            auto_theme_dark: "Lithe Dark".to_string(),
            ui_font_size: 16.5,
            show_status_bar: false,
            display_language: "en-US".to_string(),
            font_size: 20.0,
            tab_size: 8,
            terminal_default_shell_id: "cmd".to_string(),
            confirm_before_discard: false,
            java_home_path: "D:\\ProgramData\\java\\openjdk-21".to_string(),
            maven_executable_path: "D:\\tools\\apache-maven-3.9.9\\bin\\mvn.cmd".to_string(),
            maven_java_home_path: "C:\\Program Files\\Java\\jdk-21".to_string(),
            maven_settings_path: "C:\\Users\\u\\.m2\\settings.xml".to_string(),
            maven_local_repository_path: "D:\\m2\\repository".to_string(),
            auto_completion: false,
            ask_where_to_open_projects: false,
            open_folders_in_new_window: false,
            word_wrap: false,
            auto_save: false,
            font_family: "Inter".to_string(),
            mono_font_family: "JetBrains Mono".to_string(),
            terminal_font_size: 16.0,
            uninstalled_extensions: vec!["lithe.nordTheme".to_string()],
            icons: "pierre".to_string(),
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lithe-settings-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn missing_file_yields_defaults_without_creating_it() {
        let dir = temp_dir("missing");
        let path = dir.join("settings.json");

        let loaded = load_from(Some(path.clone()));
        assert_eq!(loaded.settings, Settings::default());
        assert!(!loaded.file_existed);
        assert!(loaded.diagnostics.is_empty());
        assert!(!path.exists(), "读取不得创建文件");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_path_degrades_to_defaults() {
        let loaded = load_from(None);
        assert_eq!(loaded.settings, Settings::default());
        assert!(!loaded.diagnostics.is_empty());
        assert!(loaded.previous.is_none());
        assert!(!loaded.read_only);
    }

    #[test]
    fn broken_json_falls_back_whole_file() {
        let parsed = parse("{ not json ");
        assert_eq!(parsed.settings, Settings::default());
        assert_eq!(parsed.diagnostics.len(), 1);
        assert!(parsed.diagnostics[0].starts_with("invalid_json"));
        assert!(parsed.previous.is_none(), "不可解析的文件没有可保留的文档");

        let parsed = parse("[1, 2, 3]");
        assert_eq!(parsed.settings, Settings::default());
        assert_eq!(parsed.diagnostics, vec!["not_an_object".to_string()]);
        assert!(parsed.previous.is_none());
    }

    #[test]
    fn broken_field_keeps_the_other_fields() {
        let parsed =
            parse(r#"{"theme": "Lithe Light", "uiFontSize": "big", "showStatusBar": false}"#);
        assert_eq!(parsed.settings.theme, "Lithe Light");
        assert_eq!(parsed.settings.ui_font_size, UI_FONT_SIZE_DEFAULT);
        assert!(!parsed.settings.show_status_bar);
        assert_eq!(parsed.diagnostics.len(), 1);
        assert!(
            parsed.diagnostics[0].contains("bad_key key=uiFontSize"),
            "{:?}",
            parsed.diagnostics
        );
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let parsed = parse(r#"{"theme": "Lithe Light", "icons": "idea-icons"}"#);
        assert_eq!(parsed.settings.theme, "Lithe Light");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    }

    #[test]
    fn values_are_normalized_on_load() {
        let parsed = parse(r#"{"uiFontSize": 99.0, "displayLanguage": "fr-FR"}"#);
        assert_eq!(parsed.settings.ui_font_size, UI_FONT_SIZE_MAX);
        assert_eq!(parsed.settings.display_language, DEFAULT_DISPLAY_LANGUAGE);

        let parsed = parse(r#"{"uiFontSize": -1.0}"#);
        assert_eq!(parsed.settings.ui_font_size, UI_FONT_SIZE_MIN);
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = temp_dir("rt");
        let path = dir.join("settings.json");

        let mut settings = Settings {
            theme: "Lithe Light".to_string(),
            auto_theme_light: "Lithe Light".to_string(),
            auto_theme_dark: DEFAULT_AUTO_THEME_DARK.to_string(),
            sync_system_theme: true,
            ui_font_size: 16.5,
            show_status_bar: false,
            display_language: "en-US".to_string(),
            ..Settings::default()
        };
        settings.normalize();

        let bytes = save_document(&path, &settings, None).expect("写入失败");
        assert!(bytes > 0);
        assert!(path.exists());
        assert!(!tmp_path(&path).exists(), "临时文件必须被 rename 掉");

        let loaded = load_from(Some(path.clone()));
        assert_eq!(loaded.settings, settings);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert!(loaded.file_existed);

        let mut second = settings.clone();
        second.theme = DEFAULT_THEME.to_string();
        save_document(&path, &second, loaded.previous.as_ref()).expect("覆盖写失败");
        assert_eq!(load_from(Some(path.clone())).settings, second);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn project_environment_overrides_survive_a_file_round_trip() {
        let dir = temp_dir("project-env");
        let path = dir.join("settings.json");

        let mut settings = Settings {
            java_home_path: "D:\\ProgramData\\java\\openjdk-21".to_string(),
            maven_executable_path: "D:\\tools\\apache-maven-3.9.9\\bin\\mvn.cmd".to_string(),
            maven_java_home_path: "C:\\Program Files\\Java\\jdk-21".to_string(),
            maven_settings_path: "C:\\Users\\u\\.m2\\settings.xml".to_string(),
            maven_local_repository_path: "D:\\m2\\repository".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        save_document(&path, &settings, None).expect("写入失败");

        let text = std::fs::read_to_string(&path).expect("读文件失败");
        for key in [
            "\"javaHomePath\"",
            "\"mavenExecutablePath\"",
            "\"mavenJavaHomePath\"",
            "\"mavenSettingsPath\"",
            "\"mavenLocalRepositoryPath\"",
        ] {
            assert!(text.contains(key), "落盘文本缺键 {key}：{text}");
        }

        let loaded = load_from(Some(path.clone()));
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(loaded.settings, settings);
        assert_eq!(
            loaded.settings.java_home_path,
            "D:\\ProgramData\\java\\openjdk-21"
        );
        assert_eq!(
            loaded.settings.maven_local_repository_path,
            "D:\\m2\\repository"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_key_survives_a_round_trip() {
        let settings = all_non_default();
        let mut copy = settings.clone();
        copy.normalize();
        assert_eq!(copy, settings, "测试自己的取值不合规");

        let json = serde_json::to_string(&settings).expect("序列化失败");
        let parsed = parse(&json);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(parsed.settings, settings, "有字段没被读回来");
    }

    #[test]
    fn word_wrap_and_auto_save_use_the_documented_keys_and_default_on() {
        let json = serde_json::to_value(Settings {
            word_wrap: false,
            auto_save: false,
            ..Settings::default()
        })
        .expect("序列化失败");
        assert_eq!(json["wordWrap"], serde_json::json!(false));
        assert_eq!(json["autoSave"], serde_json::json!(false));

        let parsed = parse(r#"{"theme": "Lithe Light"}"#);
        assert!(parsed.settings.word_wrap, "缺 wordWrap 应回落 true");
        assert!(parsed.settings.auto_save, "缺 autoSave 应回落 true");

        let parsed = parse(r#"{"wordWrap": false, "autoSave": false}"#);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert!(!parsed.settings.word_wrap);
        assert!(!parsed.settings.auto_save);
    }

    #[test]
    fn a_single_broken_key_falls_back_alone() {
        let parsed = parse(
            r#"{"theme": "Lithe Light", "tabSize": "eight", "terminalDefaultShellId": "wsl"}"#,
        );
        assert_eq!(parsed.settings.theme, "Lithe Light", "好键不受坏键连带");
        assert_eq!(parsed.settings.terminal_default_shell_id, "wsl");
        assert_eq!(parsed.settings.tab_size, TAB_SIZE_DEFAULT);
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|line| line.contains("bad_key")),
            "坏键必须留下诊断：{:?}",
            parsed.diagnostics
        );
    }

    #[test]
    fn any_single_broken_key_leaves_every_other_key_intact() {
        let settings = all_non_default();
        let Value::Object(baseline) = serde_json::to_value(&settings).expect("序列化失败")
        else {
            panic!("设置必须是 JSON 对象");
        };
        assert!(baseline.len() >= 19, "键集太小，这条测试失去意义");

        for key in known_keys() {
            let mut object = baseline.clone();
            object.insert(key.clone(), Value::Null);
            let text = serde_json::to_string(&Value::Object(object)).expect("序列化失败");
            let parsed = parse(&text);

            assert!(
                parsed
                    .diagnostics
                    .iter()
                    .any(|line| line.contains(&format!("bad_key key={key}"))),
                "键 {key} 传 null 没被判成坏键（它是不是其实接受 null？换一个非法值）：{:?}",
                parsed.diagnostics
            );

            let mut without_key = baseline.clone();
            without_key.remove(&key);
            let expected =
                parse(&serde_json::to_string(&Value::Object(without_key)).expect("序列化失败"))
                    .settings;
            assert_eq!(
                parsed.settings, expected,
                "坏键 {key} 连带影响了别的键（回落路径漏了某个字段？）"
            );
        }
    }

    #[test]
    fn unknown_keys_are_preserved_across_a_write() {
        let dir = temp_dir("preserve");
        let path = dir.join("settings.json");

        let original = r#"{
  "theme": "Lithe Light",
  "icons": "idea-icons",
  "thirdParty": { "keepMe": [1, 2, 3], "alsoKeep": "yes" },
  "version": 1
}"#;
        std::fs::create_dir_all(&dir).expect("建目录失败");
        std::fs::write(&path, original).expect("写初始文件失败");

        let loaded = load_from(Some(path.clone()));
        assert_eq!(loaded.settings.theme, "Lithe Light");
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);

        let mut settings = loaded.settings.clone();
        settings.ui_font_size = 17.0;
        settings.normalize();
        save_document(&path, &settings, loaded.previous.as_ref()).expect("写入失败");

        let text = std::fs::read_to_string(&path).expect("读文件失败");
        let Value::Object(written) = serde_json::from_str::<Value>(&text).expect("落盘必须是 JSON")
        else {
            panic!("落盘必须是对象");
        };
        assert_eq!(written.get("theme"), Some(&Value::from("Lithe Light")));
        assert_eq!(written.get("uiFontSize"), Some(&Value::from(17.0)));
        assert_eq!(
            written.get("icons"),
            Some(&Value::from("idea-icons")),
            "未知键被吃掉了：{text}"
        );
        assert_eq!(
            written.get("thirdParty"),
            Some(&serde_json::json!({ "keepMe": [1, 2, 3], "alsoKeep": "yes" })),
            "未知的嵌套对象必须整体保留：{text}"
        );

        let reloaded = load_from(Some(path.clone()));
        assert_eq!(reloaded.settings, settings);
        assert!(
            reloaded.diagnostics.is_empty(),
            "{:?}",
            reloaded.diagnostics
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn document_version_is_written_and_normalized() {
        let settings = Settings::default();

        let fresh = merge_document(None, &settings).expect("合并失败");
        assert_eq!(fresh.get(DOCUMENT_VERSION_KEY), Some(&Value::from(1)));

        let stale = serde_json::json!({ "version": 0, "theme": "Lithe Light" });
        let merged = merge_document(Some(&stale), &settings).expect("合并失败");
        assert_eq!(
            merged.get(DOCUMENT_VERSION_KEY),
            Some(&Value::from(DOCUMENT_VERSION)),
            "写回时必须写当前版本，而不是文件里那个旧值"
        );
        assert_eq!(
            merged.get("theme"),
            Some(&Value::from(DEFAULT_THEME)),
            "typed 设置覆盖它自己声明的键"
        );
    }

    #[test]
    fn newer_document_version_is_read_only() {
        let parsed = parse(r#"{"version": 99, "theme": "Lithe Light", "futureKey": true}"#);
        assert!(parsed.read_only, "更高版本必须转只读");
        assert_eq!(parsed.settings.theme, "Lithe Light", "仍然读取能读懂的部分");
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|line| line.contains("document_version_newer")),
            "{:?}",
            parsed.diagnostics
        );

        assert!(!parse(r#"{"version": 1}"#).read_only);
        assert!(!parse(r#"{"version": 0}"#).read_only);
        assert!(!parse(r#"{"theme": "Lithe Dark"}"#).read_only);
        assert!(!parse(r#"{"version": "one"}"#).read_only);
    }

    #[test]
    fn known_keys_come_from_the_schema() {
        let keys = known_keys();
        for key in ["theme", "tabSize", "terminalDefaultShellId", "javaHomePath"] {
            assert!(keys.contains(key), "缺键 {key}");
        }
        let Value::Object(defaults) =
            serde_json::to_value(Settings::default()).expect("序列化失败")
        else {
            panic!("设置必须是对象");
        };
        assert_eq!(keys.len(), defaults.len());
    }

    #[test]
    fn debounce_merges_changes_in_one_window() {
        let mut debounce = DebounceState::default();
        assert!(!debounce.has_pending());

        let first = debounce.arm();
        let _second = debounce.arm();
        let third = debounce.arm();
        assert!(debounce.has_pending());

        assert!(!debounce.should_flush(first));
        assert!(!debounce.should_flush(third - 1));
        assert!(debounce.has_pending());

        assert!(debounce.should_flush(third));
        assert!(!debounce.should_flush(third), "同一代不得重复写");
        assert!(!debounce.has_pending());
    }

    #[test]
    fn immediate_write_clears_pending() {
        let mut debounce = DebounceState::default();
        let armed = debounce.arm();
        debounce.mark_flushed();
        assert!(!debounce.has_pending());
        assert!(
            !debounce.should_flush(armed),
            "立即写之后，那次被替代的防抖唤醒不得再写一遍"
        );
    }

    #[test]
    fn pattern_lines_are_trimmed_and_deduplicated() {
        assert_eq!(
            normalize_pattern_lines("  .git \n\n.hg\r\n.git\n   \n__pycache__\n"),
            vec![".git", ".hg", "__pycache__"]
        );
        assert!(normalize_pattern_lines("   \n\n").is_empty());
    }

    #[test]
    fn theme_defaults_are_consistent() {
        let settings = Settings::default();
        assert_eq!(settings.theme, DEFAULT_THEME);
        assert_eq!(settings.auto_theme_light, DEFAULT_AUTO_THEME_LIGHT);
        assert_eq!(settings.auto_theme_dark, DEFAULT_AUTO_THEME_DARK);
    }
}
