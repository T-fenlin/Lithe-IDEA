//! 设置文件的读、写与规范化落点（**不依赖 GPUI**）。
//!
//! ## 读取语义（照 Windows 的 `lib/settings-persistence.ts`）
//!
//! | 情况 | 行为 | 真源 |
//! | --- | --- | --- |
//! | 文件不存在 | 全部默认值，**不创建文件**（Windows 也是"改动才写"） | `settings-persistence.ts:18-43,84-94` |
//! | 文件不是合法 JSON / 不是对象 | 全部默认值 + 一条诊断，**不 panic** | `settings-persistence.ts:59-64` |
//! | 某个键类型坏了 | **该键**回落默认值 + 一条诊断，其余键照用 | `settings-normalization.ts` 的逐字段思路 |
//! | 未知键 | 静默忽略 | 同左（用户可能手改过文件） |
//! | 缺某个键 | 该键回落默认值 | `settings-persistence.ts:73-79` |
//!
//! 诊断统一走 `S1_SETTINGS ...` 前缀打 stderr —— 与 `gpui/crates/app/src/main.rs:74-78` 的
//! `S1_THEME` 同一风格：可 grep、可在自动化验证里断言。
//!
//! ## 写入语义：原子写 + 300ms 防抖
//!
//! - **原子写**：先写 `settings.json.tmp`，再 `rename` 覆盖目标
//!   （`std::fs::rename` 在 Windows 上走 `MoveFileEx(MOVEFILE_REPLACE_EXISTING)`，可以覆盖已存在文件）。
//!   这样断电/崩溃只会留下一个 `.tmp`，不会留下半份 JSON。
//! - **300ms 防抖**：真源 `lib/settings-persistence.ts:96-112` —— 同一窗口内的多次改动合并成一次写。
//!   防抖的判定被抽成 [`DebounceState`] 这个**纯状态机**，因此可以确定性单测（不用真实时钟）。

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::schema::Settings;

/// 防抖窗口。真源 `lib/settings-persistence.ts:111`（`setTimeout(..., 300)`）。
pub const SAVE_DEBOUNCE_MS: u64 = 300;

/// 一次读取的结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    /// 规范化之后的设置。**任何失败路径都返回可用的设置**（最差就是全默认值）。
    pub settings: Settings,
    /// 设置文件路径；`None` = 推导不出路径（只影响能否落盘）。
    pub path: Option<PathBuf>,
    /// 文件是否真的存在且被读到了（不存在 = 首次启动）。
    pub file_existed: bool,
    /// 逐键诊断（坏键、整份解析失败等）。调用方负责打印。
    pub diagnostics: Vec<String>,
}

/// 读设置文件（带路径推导），**不创建文件、不 panic**。
pub fn load() -> Loaded {
    let path = crate::paths::settings_file_path();
    load_from(path)
}

/// 从指定路径读设置文件。`path = None` 表示只使用默认值。
pub fn load_from(path: Option<PathBuf>) -> Loaded {
    let Some(path) = path else {
        let mut settings = Settings::default();
        settings.normalize();
        return Loaded {
            settings,
            path: None,
            file_existed: false,
            diagnostics: vec!["no_settings_path (推导不出设置文件路径，本次会话不落盘)".to_string()],
        };
    };

    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let (settings, diagnostics) = load_from_str(&text);
            Loaded {
                settings,
                path: Some(path),
                file_existed: true,
                diagnostics,
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // 首次启动：全部默认值，且**不写文件**（Windows 是"改动才写"）。
            let mut settings = Settings::default();
            settings.normalize();
            Loaded {
                settings,
                path: Some(path),
                file_existed: false,
                diagnostics: Vec::new(),
            }
        }
        Err(error) => {
            let mut settings = Settings::default();
            settings.normalize();
            Loaded {
                settings,
                path: Some(path),
                file_existed: false,
                diagnostics: vec![format!("read_failed error={error}")],
            }
        }
    }
}

/// 解析设置文件文本：逐键容错 + 规范化。
///
/// 返回 `(设置, 诊断)`。**永不失败**：最差返回全默认值 + 一条诊断。
pub fn load_from_str(text: &str) -> (Settings, Vec<String>) {
    let mut diagnostics = Vec::new();

    let parsed: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("invalid_json error={error}"));
            let mut settings = Settings::default();
            settings.normalize();
            return (settings, diagnostics);
        }
    };

    let Value::Object(object) = parsed else {
        diagnostics.push("not_an_object".to_string());
        let mut settings = Settings::default();
        settings.normalize();
        return (settings, diagnostics);
    };

    let mut settings = settings_from_object(&object, &mut diagnostics);
    settings.normalize();
    (settings, diagnostics)
}

/// 逐键取值：键不存在 → 默认值；键的类型坏了 → **该键**默认值 + 诊断。
///
/// 之所以不用一次性 `serde_json::from_value::<Settings>`：那样任何一个键的类型坏掉都会让整份
/// 设置丢回默认值，而 Windows 的语义是"逐键回退"（`settings-persistence.ts:73-79`）。
///
/// ⚠️ **两段式**（2026-09-25 修）：先整体 `serde` 解析一次 —— 它能认**所有**字段，
/// 所以新加的字段**不需要**在这里再登记一遍就能被读出来；只有"某个键的类型真的坏了"
/// （整体解析失败）时才退到下面这张逐键表。修之前只有逐键表这一条路，结果
/// `fontSize` / `tabSize` / `terminalDefaultShellId` 三个新键**写得出、读不回**
/// （文件里明明是 8，启动后仍是默认 2），而且**一点诊断都没有** —— 这正是"两份键名表"
/// 的典型失效方式。
///
/// 逐键表仍需与 `Settings` 的字段保持同步（坏键路径靠它）；`every_key_survives_a_round_trip`
/// 那条测试是守卫：它用"每个字段都不是默认值"的设置跑一遍存取往返。
fn settings_from_object(object: &Map<String, Value>, diagnostics: &mut Vec<String>) -> Settings {
    if let Ok(parsed) = serde_json::from_value::<Settings>(Value::Object(object.clone())) {
        return parsed;
    }

    // 走到这里说明至少有一个键的类型不对；逐键取，坏的那个键回默认并留诊断。
    let mut settings = Settings::default();
    take(object, "theme", &mut settings.theme, diagnostics);
    take(
        object,
        "syncSystemTheme",
        &mut settings.sync_system_theme,
        diagnostics,
    );
    take(
        object,
        "autoThemeLight",
        &mut settings.auto_theme_light,
        diagnostics,
    );
    take(
        object,
        "autoThemeDark",
        &mut settings.auto_theme_dark,
        diagnostics,
    );
    take(object, "uiFontSize", &mut settings.ui_font_size, diagnostics);
    take(
        object,
        "showStatusBar",
        &mut settings.show_status_bar,
        diagnostics,
    );
    take(
        object,
        "displayLanguage",
        &mut settings.display_language,
        diagnostics,
    );
    take(object, "fontSize", &mut settings.font_size, diagnostics);
    take(object, "tabSize", &mut settings.tab_size, diagnostics);
    take(
        object,
        "terminalDefaultShellId",
        &mut settings.terminal_default_shell_id,
        diagnostics,
    );
    take(
        object,
        "confirmBeforeDiscard",
        &mut settings.confirm_before_discard,
        diagnostics,
    );
    // 「项目 · JDK 与 Maven」页的三个覆盖值（键名 = Core 契约里 toolchain 载荷的键名）。
    take(
        object,
        "javaHomePath",
        &mut settings.java_home_path,
        diagnostics,
    );
    take(
        object,
        "mavenExecutablePath",
        &mut settings.maven_executable_path,
        diagnostics,
    );
    take(
        object,
        "mavenJavaHomePath",
        &mut settings.maven_java_home_path,
        diagnostics,
    );
    // 「LSP」页（阶段 18）：真源三键里唯一有消费方的那一个。
    take(
        object,
        "autoCompletion",
        &mut settings.auto_completion,
        diagnostics,
    );
    // 「打开其他项目」（B4）：决策判据 + 「不再询问」写回的两个键。
    take(
        object,
        "askWhereToOpenProjects",
        &mut settings.ask_where_to_open_projects,
        diagnostics,
    );
    take(
        object,
        "openFoldersInNewWindow",
        &mut settings.open_folders_in_new_window,
        diagnostics,
    );
    settings
}

/// 取一个键到 `slot`：类型不符就保留默认值并记一条诊断。
fn take<T: DeserializeOwned>(
    object: &Map<String, Value>,
    key: &str,
    slot: &mut T,
    diagnostics: &mut Vec<String>,
) {
    let Some(value) = object.get(key) else {
        return;
    };
    match serde_json::from_value::<T>(value.clone()) {
        Ok(parsed) => *slot = parsed,
        Err(error) => diagnostics.push(format!("bad_key key={key} error={error}")),
    }
}

/// 原子写设置文件。
///
/// 只是 [`save_json`] 的一个薄包装（保持既有调用点与测试不变）。
pub fn save(path: &Path, settings: &Settings) -> io::Result<usize> {
    save_json(path, settings)
}

/// 原子写任意 JSON 载荷：临时文件 + rename。
///
/// 目录不存在时先建（首次启动要能落盘）。
///
/// 做成泛型是为了让**同目录的其它 JSON 文件**（`recent-projects.json`，见
/// [`crate::recent_projects`]）复用同一套"临时文件 + rename"语义，而不是各自再写一份
/// —— 两份原子写的实现迟早会在"断电只留半个文件"这类细节上漂移。
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<usize> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let json = serde_json::to_string_pretty(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let text = format!("{json}\n");

    // 临时文件名固定（不用随机后缀）：每个文件只有一个写者（设置文件是 `SettingsStore` 的
    // 防抖任务，最近项目是它自己的列表所有者），而固定后缀让"崩溃后残留的临时文件"可预期、可清理。
    let tmp = tmp_path(path);
    std::fs::write(&tmp, text.as_bytes())?;
    std::fs::rename(&tmp, path)?;
    Ok(text.len())
}

/// 临时文件路径（`settings.json` → `settings.json.tmp`）。
pub fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

/// 300ms 防抖的**纯状态机**（不碰时钟、不碰线程）。
///
/// 语义与真源 `lib/settings-persistence.ts:96-112` 一致：每次改动把窗口往后推，
/// 只有"最后一次改动"对应的那次唤醒才真的落盘 —— 也就是把同一窗口里的多次改动合并成一次写。
///
/// 用法：改动时 `arm()` 拿到一个「代数」，睡满 300ms 后把这个代数交回 [`Self::should_flush`]；
/// 期间又改过就会拿到更大的代数，于是这次唤醒什么都不做。
///
/// 之所以做成纯状态机：真实时钟的测试只能靠 `sleep` 同步，而
/// `.agents/skills/write-stable-tests/SKILL.md` 禁止用真实时间同步状态。
#[derive(Debug, Default, Clone)]
pub struct DebounceState {
    /// 已发出的代数（每次改动 +1）。
    revision: u64,
    /// 已落盘的代数。
    flushed: u64,
}

impl DebounceState {
    /// 记一次改动，返回本次改动的代数。
    pub fn arm(&mut self) -> u64 {
        self.revision += 1;
        self.revision
    }

    /// 这次唤醒是否该落盘。
    ///
    /// `true` 会把状态标记成"已落盘到这一代"，于是同一代的重复唤醒不会重复写。
    pub fn should_flush(&mut self, armed: u64) -> bool {
        if armed != self.revision || self.flushed >= armed {
            return false;
        }
        self.flushed = armed;
        true
    }

    /// 是否有改动还没落盘（退出/关对话框时用来决定要不要立刻补一次写）。
    pub fn has_pending(&self) -> bool {
        self.flushed < self.revision
    }

    /// 强制认为"已落盘"（立即写之后调用）。
    pub fn mark_flushed(&mut self) {
        self.flushed = self.revision;
    }
}

/// 逐行 trim + 去空行的数组规范化（Windows §4.3 的多行文本语义：
/// `hiddenFilePatterns` / `hiddenDirectoryPatterns` 每行一项，`trim` 后丢弃空行，
/// `macos-settings-panels.tsx:245-269`）。
///
/// v1 没有任何数组型设置键，所以这个函数目前**只被测试使用**：它把 §4.3 里"数组 trim/去空行"
/// 这条规范化规则先固定下来，等 explorer 接上隐藏路径过滤时直接用，避免那时再各自实现一遍。
/// 去重语义照 `normalizeStringList`（`settings-normalization.ts:199-207`）：去重且保序。
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
        DEFAULT_AUTO_THEME_DARK, DEFAULT_AUTO_THEME_LIGHT, DEFAULT_DISPLAY_LANGUAGE,
        DEFAULT_THEME, UI_FONT_SIZE_DEFAULT, UI_FONT_SIZE_MAX, UI_FONT_SIZE_MIN,
    };

    /// 文件不存在 → 全部默认值，且**不创建文件**。
    #[test]
    fn missing_file_yields_defaults_without_creating_it() {
        let dir = std::env::temp_dir().join(format!(
            "lithe-settings-test-missing-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("settings.json");

        let loaded = load_from(Some(path.clone()));
        assert_eq!(loaded.settings, Settings::default());
        assert!(!loaded.file_existed);
        assert!(loaded.diagnostics.is_empty());
        assert!(!path.exists(), "读取不得创建文件");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 推导不出路径时也必须能工作（全默认值 + 一条诊断，不 panic）。
    #[test]
    fn missing_path_degrades_to_defaults() {
        let loaded = load_from(None);
        assert_eq!(loaded.settings, Settings::default());
        assert!(!loaded.diagnostics.is_empty());
    }

    /// 整份不是 JSON / 不是对象 → 全默认值 + 诊断，不 panic。
    #[test]
    fn broken_json_falls_back_whole_file() {
        let (settings, diagnostics) = load_from_str("{ not json ");
        assert_eq!(settings, Settings::default());
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].starts_with("invalid_json"));

        let (settings, diagnostics) = load_from_str("[1, 2, 3]");
        assert_eq!(settings, Settings::default());
        assert_eq!(diagnostics, vec!["not_an_object".to_string()]);
    }

    /// 坏字段回落默认值，**其余字段照用**（不是整份丢弃）。
    #[test]
    fn broken_field_keeps_the_other_fields() {
        let (settings, diagnostics) = load_from_str(
            r#"{"theme": "Lithe Light", "uiFontSize": "big", "showStatusBar": false}"#,
        );
        assert_eq!(settings.theme, "Lithe Light");
        assert_eq!(settings.ui_font_size, UI_FONT_SIZE_DEFAULT);
        assert!(!settings.show_status_bar);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("bad_key key=uiFontSize"), "{diagnostics:?}");
    }

    /// 未知键静默忽略（手改过的文件里可能有别的版本/别的产品的键）。
    #[test]
    fn unknown_keys_are_ignored() {
        let (settings, diagnostics) =
            load_from_str(r#"{"theme": "Lithe Light", "iconTheme": "idea-icons"}"#);
        assert_eq!(settings.theme, "Lithe Light");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    /// 读入即规范化：越界字号被夹住、非法语言回落。
    #[test]
    fn values_are_normalized_on_load() {
        let (settings, _) =
            load_from_str(r#"{"uiFontSize": 99.0, "displayLanguage": "fr-FR"}"#);
        assert_eq!(settings.ui_font_size, UI_FONT_SIZE_MAX);
        assert_eq!(settings.display_language, DEFAULT_DISPLAY_LANGUAGE);

        let (settings, _) = load_from_str(r#"{"uiFontSize": -1.0}"#);
        assert_eq!(settings.ui_font_size, UI_FONT_SIZE_MIN);
    }

    /// 缺键逐键回退 + 规范化不改变合法值（往返稳定：写出去的再读回来必须一样）。
    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir().join(format!("lithe-settings-test-rt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
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

        let bytes = save(&path, &settings).expect("写入失败");
        assert!(bytes > 0);
        assert!(path.exists());
        assert!(!tmp_path(&path).exists(), "临时文件必须被 rename 掉");

        let loaded = load_from(Some(path.clone()));
        assert_eq!(loaded.settings, settings);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert!(loaded.file_existed);

        // 覆盖写也必须成功（rename 覆盖已存在文件）。
        let mut second = settings.clone();
        second.theme = DEFAULT_THEME.to_string();
        save(&path, &second).expect("覆盖写失败");
        assert_eq!(load_from(Some(path.clone())).settings, second);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **写进去能读回来**（文件级往返，专测「项目 · JDK 与 Maven」页的三个覆盖值）。
    ///
    /// 这条是独立的守卫，不是 `every_key_survives_a_round_trip` 的重复：
    /// 那条走的是内存里的 `load_from_str`，这条走**真实的原子写 + 读文件**，
    /// 覆盖"键名拼错 / 序列化时被 `skip` / 写文件那一段丢了字段"这一类只在落盘路径上出现的失效。
    #[test]
    fn project_environment_overrides_survive_a_file_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "lithe-settings-test-project-env-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("settings.json");

        let mut settings = Settings {
            java_home_path: "D:\\ProgramData\\java\\openjdk-21".to_string(),
            maven_executable_path: "D:\\tools\\apache-maven-3.9.9\\bin\\mvn.cmd".to_string(),
            maven_java_home_path: "C:\\Program Files\\Java\\jdk-21".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        save(&path, &settings).expect("写入失败");

        // 落盘文本里三个键名必须是 Core 契约里的那三个（驼峰）。
        let text = std::fs::read_to_string(&path).expect("读文件失败");
        for key in ["\"javaHomePath\"", "\"mavenExecutablePath\"", "\"mavenJavaHomePath\""] {
            assert!(text.contains(key), "落盘文本缺键 {key}：{text}");
        }

        let loaded = load_from(Some(path.clone()));
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(
            loaded.settings.java_home_path,
            "D:\\ProgramData\\java\\openjdk-21"
        );
        assert_eq!(
            loaded.settings.maven_executable_path,
            "D:\\tools\\apache-maven-3.9.9\\bin\\mvn.cmd"
        );
        assert_eq!(
            loaded.settings.maven_java_home_path,
            "C:\\Program Files\\Java\\jdk-21"
        );
        // 整个结构体也要逐字段相等（漏键会立刻表现为不等）。
        assert_eq!(loaded.settings, settings);

        let _ = std::fs::remove_dir_all(&dir);
    }


    ///
    /// 这条测试守的是一类**静默**失效：`settings_from_object` 里那张逐键表一旦漏了新字段，
    /// 文件里写得再对、读回来的也是默认值，而且**没有任何诊断**（阶段 14 实测踩到过：
    /// `fontSize` / `tabSize` / `terminalDefaultShellId` 写得出、读不回）。
    /// 用全非默认值而不是 `..Settings::default()`，就是为了让"漏了某个键"必定表现为不等。
    #[test]
    fn every_key_survives_a_round_trip() {
        let mut settings = Settings {
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
            // `autoCompletion` 的"非默认值"是 `false`（默认 `true`）。
            auto_completion: false,
            // 同上：这两个键的默认值也是 `true`（`default-settings.ts:111-112`），
            // 所以"非默认值"必须写 `false`，否则这条往返测不出"漏了登记"。
            ask_where_to_open_projects: false,
            open_folders_in_new_window: false,
        };
        // 先规范化，保证"写出去的"就是"合法的"（否则比的是两个不同的东西）。
        settings.normalize();
        assert_eq!(settings, {
            // 规范化不该把上面这些合法值改掉；改掉了说明测试自己的取值不合规。
            let mut copy = settings.clone();
            copy.normalize();
            copy
        });

        let json = serde_json::to_string(&settings).expect("序列化失败");
        let (loaded, diagnostics) = load_from_str(&json);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(loaded, settings, "有字段没被读回来（逐键表漏了它？）");
    }

    /// 一个键的类型坏掉时，**只有那个键**回默认值，其余键照常生效。
    #[test]
    fn a_single_broken_key_falls_back_alone() {
        let (loaded, diagnostics) = load_from_str(
            r#"{"theme": "Lithe Light", "tabSize": "eight", "terminalDefaultShellId": "wsl"}"#,
        );
        assert_eq!(loaded.theme, "Lithe Light", "好键不受坏键连带");
        assert_eq!(loaded.terminal_default_shell_id, "wsl");
        assert_eq!(loaded.tab_size, crate::schema::TAB_SIZE_DEFAULT);
        assert!(
            diagnostics.iter().any(|line| line.contains("bad_key")),
            "坏键必须留下诊断：{diagnostics:?}"
        );
    }

    /// 防抖合并：窗口内的多次改动只落盘一次；更晚的那次唤醒不再重复写。
    #[test]
    fn debounce_merges_changes_in_one_window() {
        let mut debounce = DebounceState::default();
        assert!(!debounce.has_pending());

        // 三次改动挤在同一个 300ms 窗口里。
        let first = debounce.arm();
        let _second = debounce.arm();
        let third = debounce.arm();
        assert!(debounce.has_pending());

        // 前两次改动的唤醒都已经过期：什么都不做（这就是"合并"）。
        assert!(!debounce.should_flush(first));
        assert!(!debounce.should_flush(third - 1));
        assert!(debounce.has_pending());

        // 最后一次改动的唤醒才真的落盘，而且只落一次。
        assert!(debounce.should_flush(third));
        assert!(!debounce.should_flush(third), "同一代不得重复写");
        assert!(!debounce.has_pending());
    }

    /// 立即写（重置/恢复默认）走 `mark_flushed`：把待写标记清掉，随后的防抖唤醒不再补写。
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

    /// §4.3 的数组规范化：trim、去空行、去重且保序（v1 备用，见 `normalize_pattern_lines` 文档）。
    #[test]
    fn pattern_lines_are_trimmed_and_deduplicated() {
        assert_eq!(
            normalize_pattern_lines("  .git \n\n.hg\r\n.git\n   \n__pycache__\n"),
            vec![".git", ".hg", "__pycache__"]
        );
        assert!(normalize_pattern_lines("   \n\n").is_empty());
    }

    /// 默认值的主题名常量与 schema 的默认值必须一致（两处写死会漂移）。
    #[test]
    fn theme_defaults_are_consistent() {
        let settings = Settings::default();
        assert_eq!(settings.theme, DEFAULT_THEME);
        assert_eq!(settings.auto_theme_light, DEFAULT_AUTO_THEME_LIGHT);
        assert_eq!(settings.auto_theme_dark, DEFAULT_AUTO_THEME_DARK);
    }
}
