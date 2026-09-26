//! 设置文件的读、写与规范化落点（**不依赖 GPUI**）。
//!
//! ## 读取语义
//!
//! | 情况 | 行为 |
//! | --- | --- |
//! | 文件不存在 | 全部默认值，**不创建文件**（改动才写） |
//! | 文件不是合法 JSON / 不是对象 | 全部默认值 + 一条诊断，**不 panic** |
//! | 某个键类型坏了 | **该键**回落默认值 + 一条诊断，其余键照用 |
//! | 未知键 | 忽略（不参与设置），但**写回时原样保留** |
//! | 缺某个键 | 该键回落默认值 |
//! | `version` 高于本程序支持 | 照常读取，但**转只读**：不覆盖用户的文件 |
//!
//! 诊断统一走 `S1_SETTINGS ...` 前缀打 stderr —— 与 `gpui/crates/app/src/main.rs:74-78` 的
//! `S1_THEME` 同一风格：可 grep、可在自动化验证里断言。
//!
//! ## 未知键为什么要保留
//!
//! 设置文件是给人改的：用户可能写注释性字段、可能装了另一个版本的 Lithe、也可能把同一份
//! 文件喂给别的工具。早期实现"未知键静默忽略"，代价是**用户手写的字段会在下一次落盘时
//! 被吃掉**——那是不可逆的数据丢失。现在读入时保留整份原始对象（[`Parsed::previous`]），
//! 写回时先合并再落盘（[`merge_document`]），未知键（含嵌套对象里的）逐字保留。
//!
//! ## 键表由 schema 派生
//!
//! [`known_keys`] 直接取 `Settings::default()` 的序列化结果，**不存在第二份键名清单**。
//! 历史上这里有一张手写的逐键表，新增键忘记登记就表现为"设置写得出、读不回"，而且
//! **一点诊断都没有**（阶段 14 踩过：`fontSize` / `tabSize` / `terminalDefaultShellId`）。
//! 现在的坏键回落路径是"逐个已知键单独试解析"，所以新字段天然被覆盖。
//!
//! ## 写入语义：原子写 + 300ms 防抖
//!
//! - **原子写**：先写 `settings.json.tmp`，再 `rename` 覆盖目标
//!   （`std::fs::rename` 在 Windows 上走 `MoveFileEx(MOVEFILE_REPLACE_EXISTING)`，可以覆盖已存在文件）。
//!   这样断电/崩溃只会留下一个 `.tmp`，不会留下半份 JSON。
//! - **300ms 防抖**：同一窗口内的多次改动合并成一次写。防抖的判定被抽成 [`DebounceState`]
//!   这个**纯状态机**，因此可以确定性单测（不用真实时钟）。

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::schema::Settings;

/// 防抖窗口（毫秒）。
pub const SAVE_DEBOUNCE_MS: u64 = 300;

/// 文档版本键名。
pub const DOCUMENT_VERSION_KEY: &str = "version";

/// 设置文档当前的版本。
///
/// 改动**含义**（不是改动字段）时才 +1：新增可选字段不需要升版本，因为缺键本来就会回落默认值。
pub const DOCUMENT_VERSION: u32 = 1;

/// 一次解析的结果（纯数据，不碰文件系统）。
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    /// 规范化之后的设置。**任何失败路径都返回可用的设置**（最差就是全默认值）。
    pub settings: Settings,
    /// 文件里的原始对象；写回时用它保留未知键。`None` = 没有可保留的文档
    /// （文件不存在、不是 JSON、或不是对象）。
    pub previous: Option<Value>,
    /// 文件声明的版本高于本程序支持：调用方**不得覆盖**这个文件。
    pub read_only: bool,
    /// 逐键诊断（坏键、整份解析失败、版本过新等）。调用方负责打印。
    pub diagnostics: Vec<String>,
}

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
    /// 见 [`Parsed::previous`]。
    pub previous: Option<Value>,
    /// 见 [`Parsed::read_only`]。
    pub read_only: bool,
}

/// 读设置文件（带路径推导），**不创建文件、不 panic**。
pub fn load() -> Loaded {
    let path = crate::paths::settings_file_path();
    load_from(path)
}

/// 从指定路径读设置文件。`path = None` 表示只使用默认值。
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

    match std::fs::read_to_string(&path) {
        Ok(text) => {
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
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // 首次启动：全部默认值，且**不写文件**（改动才写）。
            Loaded {
                settings: normalized_default(),
                path: Some(path),
                file_existed: false,
                diagnostics: Vec::new(),
                previous: None,
                read_only: false,
            }
        }
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

/// 解析设置文件文本：逐键容错 + 保留原始文档 + 规范化。**永不失败**。
pub fn parse(text: &str) -> Parsed {
    let mut diagnostics = Vec::new();

    let parsed: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("invalid_json error={error}"));
            return Parsed {
                settings: normalized_default(),
                previous: None,
                read_only: false,
                diagnostics,
            };
        }
    };

    let Value::Object(object) = parsed else {
        diagnostics.push("not_an_object".to_string());
        return Parsed {
            settings: normalized_default(),
            previous: None,
            read_only: false,
            diagnostics,
        };
    };

    // 版本过新时照常读取能读懂的部分，但把文件标成只读：覆盖它等于把用户在新版本里
    // 设置的东西（以及我们不认识的键）无声地降级掉。
    let declared = declared_version(&object);
    let read_only = declared.is_some_and(|version| version > u64::from(DOCUMENT_VERSION));
    if let Some(version) = declared.filter(|_| read_only) {
        diagnostics.push(format!(
            "document_version_newer declared={version} supported={DOCUMENT_VERSION}"
        ));
    }

    let mut settings = settings_from_object(&object, &mut diagnostics);
    settings.normalize();
    Parsed {
        settings,
        previous: Some(Value::Object(object)),
        read_only,
        diagnostics,
    }
}

/// 文档里声明的版本；缺失或不是非负整数时返回 `None`。
pub fn declared_version(object: &Map<String, Value>) -> Option<u64> {
    object.get(DOCUMENT_VERSION_KEY)?.as_u64()
}

/// 由 schema 派生的已知键集：`Settings::default()` 的序列化结果。
///
/// 这是本模块**唯一**的键名来源。不要再加第二份手写清单——见模块文档。
pub fn known_keys() -> &'static BTreeSet<String> {
    static KEYS: OnceLock<BTreeSet<String>> = OnceLock::new();
    KEYS.get_or_init(|| match serde_json::to_value(Settings::default()) {
        Ok(Value::Object(object)) => object.keys().cloned().collect(),
        // `Settings` 是纯标量结构，序列化不会失败；真失败时键集为空，
        // 坏键回落路径会退化成"全部用默认值"（仍然不 panic）。
        _ => BTreeSet::new(),
    })
}

fn normalized_default() -> Settings {
    let mut settings = Settings::default();
    settings.normalize();
    settings
}

/// 逐键取值：键不存在 → 默认值；键的类型坏了 → **该键**默认值 + 诊断。
///
/// 快路径是整体 `serde` 解析一次（任何一个键的类型坏掉都会失败），失败后退到
/// "逐个已知键单独试解析"：能解析的留下，不能的留诊断并回落默认值。
///
/// ⚠️ 逐键那一步遍历的是 [`known_keys`]（由 schema 派生），**不是**手写清单——
/// 所以新加的字段不需要在这里登记第二遍。
fn settings_from_object(object: &Map<String, Value>, diagnostics: &mut Vec<String>) -> Settings {
    if let Ok(parsed) = serde_json::from_value::<Settings>(Value::Object(object.clone())) {
        return parsed;
    }

    let known = known_keys();
    let mut sanitized = Map::new();
    for (key, value) in object {
        if !known.contains(key) {
            continue;
        }
        let mut probe = Map::new();
        probe.insert(key.clone(), value.clone());
        match serde_json::from_value::<Settings>(Value::Object(probe)) {
            Ok(_) => {
                sanitized.insert(key.clone(), value.clone());
            }
            Err(error) => diagnostics.push(format!("bad_key key={key} error={error}")),
        }
    }

    match serde_json::from_value::<Settings>(Value::Object(sanitized)) {
        Ok(parsed) => parsed,
        Err(error) => {
            diagnostics.push(format!("settings_fallback error={error}"));
            Settings::default()
        }
    }
}

/// 生成要落盘的文档：typed 设置 + **保留**上一次文件里的未知键 + 写入 `version`。
///
/// 两件事必须一起做，缺一不可：
///
/// - `typed` 覆盖它自己声明的键（所以设置改动一定生效）；
/// - 上一次文件里出现、而 `typed` 不认识的键逐字保留（含嵌套对象内部的键），
///   否则用户手写的字段会被无声吃掉。
///
/// `previous` 传 `None`（首次写、或上次文件不可解析）时就是一份干净的新文档。
pub fn merge_document(
    previous: Option<&Value>,
    settings: &Settings,
) -> Result<Value, serde_json::Error> {
    let typed = serde_json::to_value(settings)?;
    let mut merged = preserve_unknown(previous, &typed);
    if let Value::Object(object) = &mut merged {
        object.insert(
            DOCUMENT_VERSION_KEY.to_string(),
            Value::from(DOCUMENT_VERSION),
        );
    }
    Ok(merged)
}

/// 把 `previous` 里 `next` 不认识的键补回 `next`。
///
/// 递归规则（只对"两边都是对象"的键下钻，其余情况 typed 一方胜出）：
///
/// ```text
/// next 有该键，previous 没有        → typed 胜出
/// next 没有该键，previous 有        → 保留 previous 的值（这就是未知键）
/// 两边都有且都是对象                → 递归合并（保留嵌套未知键）
/// 两边都有但至少一边不是对象        → typed 胜出
/// ```
fn preserve_unknown(previous: Option<&Value>, next: &Value) -> Value {
    let (Some(Value::Object(previous)), Value::Object(next)) = (previous, next) else {
        return next.clone();
    };

    let mut merged = next.clone();
    for (key, previous_value) in previous {
        let unknown = !merged.contains_key(key);
        if unknown {
            merged.insert(key.clone(), previous_value.clone());
            continue;
        }
        let nested = merged
            .get(key)
            .is_some_and(|next_value| next_value.is_object() && previous_value.is_object());
        if nested {
            let next_value = merged.get(key).cloned().unwrap_or(Value::Null);
            merged.insert(
                key.clone(),
                preserve_unknown(Some(previous_value), &next_value),
            );
        }
    }
    Value::Object(merged)
}

/// 原子写设置文档：合并未知键 → 临时文件 → rename。
pub fn save_document(
    path: &Path,
    settings: &Settings,
    previous: Option<&Value>,
) -> io::Result<usize> {
    let document = merge_document(previous, settings)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    save_json(path, &document)
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
/// 每次改动把窗口往后推，只有"最后一次改动"对应的那次唤醒才真的落盘
/// —— 也就是把同一窗口里的多次改动合并成一次写。
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

/// 逐行 trim + 去空行的数组规范化（多行文本语义：每行一项，`trim` 后丢弃空行）。
///
/// 目前**只被测试使用**：它把"数组 trim/去空行"这条规范化规则先固定下来，等隐藏路径
/// 过滤接上时直接用，避免那时再各自实现一遍。去重语义是"去重且保序"。
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

    /// 每个字段都不是默认值的设置：用来让"某个字段没被读回来"必定表现为不等。
    ///
    /// 用 `..Settings::default()` 会让漏字段的测试**静默通过**，所以这里逐字段写全。
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
            // `autoCompletion` 的"非默认值"是 `false`（默认 `true`）。
            auto_completion: false,
            // 同上：这两个键的默认值也是 `true`，所以"非默认值"必须写 `false`。
            ask_where_to_open_projects: false,
            open_folders_in_new_window: false,
        }
    }

    /// 独立的临时目录（每个测试一个），用完删掉。
    ///
    /// 目录名带上调用方的标记，避免并行执行的测试互相踩。
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lithe-settings-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// 文件不存在 → 全部默认值，且**不创建文件**。
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

    /// 推导不出路径时也必须能工作（全默认值 + 一条诊断，不 panic）。
    #[test]
    fn missing_path_degrades_to_defaults() {
        let loaded = load_from(None);
        assert_eq!(loaded.settings, Settings::default());
        assert!(!loaded.diagnostics.is_empty());
        assert!(loaded.previous.is_none());
        assert!(!loaded.read_only);
    }

    /// 整份不是 JSON / 不是对象 → 全默认值 + 诊断，不 panic。
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

    /// 坏字段回落默认值，**其余字段照用**（不是整份丢弃）。
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

    /// 未知键不影响设置值，也不产生诊断。
    #[test]
    fn unknown_keys_are_ignored() {
        let parsed = parse(r#"{"theme": "Lithe Light", "iconTheme": "idea-icons"}"#);
        assert_eq!(parsed.settings.theme, "Lithe Light");
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    }

    /// 读入即规范化：越界字号被夹住、非法语言回落。
    #[test]
    fn values_are_normalized_on_load() {
        let parsed = parse(r#"{"uiFontSize": 99.0, "displayLanguage": "fr-FR"}"#);
        assert_eq!(parsed.settings.ui_font_size, UI_FONT_SIZE_MAX);
        assert_eq!(parsed.settings.display_language, DEFAULT_DISPLAY_LANGUAGE);

        let parsed = parse(r#"{"uiFontSize": -1.0}"#);
        assert_eq!(parsed.settings.ui_font_size, UI_FONT_SIZE_MIN);
    }

    /// 写出去再读回来必须一样（含原子写路径与临时文件清理）。
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

        // 覆盖写也必须成功（rename 覆盖已存在文件）。
        let mut second = settings.clone();
        second.theme = DEFAULT_THEME.to_string();
        save_document(&path, &second, loaded.previous.as_ref()).expect("覆盖写失败");
        assert_eq!(load_from(Some(path.clone())).settings, second);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **写进去能读回来**（文件级往返，专测「项目 · JDK 与 Maven」页的覆盖值）。
    ///
    /// 走的是真实的原子写 + 读文件，覆盖"键名拼错 / 序列化时被 `skip` / 写文件那一段丢了字段"
    /// 这类只在落盘路径上出现的失效。
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

        // 落盘文本里五个键名必须是 Core 契约 / 真源里的那几个（驼峰）。
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

    /// 每个键都能写出去再读回来。
    ///
    /// 守的是一类**静默**失效：键名表一旦漏了字段，文件里写得再对、读回来的也是默认值，
    /// 而且没有任何诊断。用全非默认值就是为了让"漏了某个键"必定表现为不等。
    #[test]
    fn every_key_survives_a_round_trip() {
        let settings = all_non_default();
        // 先确认这些取值本身就是规范化的不动点（否则比的是两个不同的东西）。
        let mut copy = settings.clone();
        copy.normalize();
        assert_eq!(copy, settings, "测试自己的取值不合规");

        let json = serde_json::to_string(&settings).expect("序列化失败");
        let parsed = parse(&json);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(parsed.settings, settings, "有字段没被读回来");
    }

    /// 一个键的类型坏掉时，**只有那个键**回默认值，其余键照常生效。
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

    /// **坏键回落路径由 schema 派生**：逐个已知键故意喂一个非法值，其余键必须全部完好。
    ///
    /// 这条测试是"手写逐键表"那类失效的守卫：只要新的 `Settings` 字段没有自动进入回落路径，
    /// 当坏键是别的键时它就**读不回来**，这里立刻不等。
    ///
    /// 非法值用 `null`：`Settings` 今天全是标量字段（`String` / `bool` / 数字），`null` 一律不被接受；
    /// 断言里要求必须出现 `bad_key` 诊断，所以将来若真有字段接受 `null`，这条测试会**明确失败**
    /// 并提示换一个非法值，而不是静默变弱。
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

            // 期望值：同一份 JSON 去掉那个坏键（于是它取默认值），其余键保持非默认值。
            let mut without_key = baseline.clone();
            without_key.remove(key);
            let expected =
                parse(&serde_json::to_string(&Value::Object(without_key)).expect("序列化失败"))
                    .settings;
            assert_eq!(
                parsed.settings, expected,
                "坏键 {key} 连带影响了别的键（回落路径漏了某个字段？）"
            );
        }
    }

    /// 未知键在写回时**必须逐字保留**——这是"用户手改不会被吃掉"的守卫。
    #[test]
    fn unknown_keys_are_preserved_across_a_write() {
        let dir = temp_dir("preserve");
        let path = dir.join("settings.json");

        let original = r#"{
  "theme": "Lithe Light",
  "iconTheme": "idea-icons",
  "thirdParty": { "keepMe": [1, 2, 3], "alsoKeep": "yes" },
  "version": 1
}"#;
        // 直接 `fs::write` 不会建父目录（`save_document` 会），所以这里自己建。
        std::fs::create_dir_all(&dir).expect("建目录失败");
        std::fs::write(&path, original).expect("写初始文件失败");

        let loaded = load_from(Some(path.clone()));
        assert_eq!(loaded.settings.theme, "Lithe Light");
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);

        // 改一个已知键再落盘。
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
            written.get("iconTheme"),
            Some(&Value::from("idea-icons")),
            "未知键被吃掉了：{text}"
        );
        assert_eq!(
            written.get("thirdParty"),
            Some(&serde_json::json!({ "keepMe": [1, 2, 3], "alsoKeep": "yes" })),
            "未知的嵌套对象必须整体保留：{text}"
        );

        // 再读一次：保留的键仍然在，且不影响设置解析。
        let reloaded = load_from(Some(path.clone()));
        assert_eq!(reloaded.settings, settings);
        assert!(
            reloaded.diagnostics.is_empty(),
            "{:?}",
            reloaded.diagnostics
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 嵌套对象内部的未知键也要保留（typed 只覆盖它自己认识的那几个子键）。
    #[test]
    fn nested_unknown_keys_are_preserved() {
        let previous = serde_json::json!({
            "advanced": { "known": "old", "unknown": 42 },
            "gone": true
        });
        let typed = serde_json::json!({
            "advanced": { "known": "new" }
        });
        let merged = preserve_unknown(Some(&previous), &typed);
        assert_eq!(
            merged,
            serde_json::json!({
                "advanced": { "known": "new", "unknown": 42 },
                "gone": true
            })
        );
    }

    /// 非对象的文档没有可保留的键：typed 直接胜出（不 panic、不丢设置）。
    #[test]
    fn non_object_previous_does_not_break_the_merge() {
        let merged = preserve_unknown(Some(&Value::from(7)), &serde_json::json!({ "a": 1 }));
        assert_eq!(merged, serde_json::json!({ "a": 1 }));
        let merged = preserve_unknown(None, &serde_json::json!({ "a": 1 }));
        assert_eq!(merged, serde_json::json!({ "a": 1 }));
    }

    /// 落盘文档必须带 `version`，而且**不继承**文件里那个（可能是旧的 / 被手改的）值。
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

    /// 版本高于本程序支持 → 照常读取，但标成只读并留诊断。
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

        // 相同或更低版本照常可写。
        assert!(!parse(r#"{"version": 1}"#).read_only);
        assert!(!parse(r#"{"version": 0}"#).read_only);
        // 版本字段缺失 = 老文件，可写。
        assert!(!parse(r#"{"theme": "Lithe Dark"}"#).read_only);
        // 版本不是整数：当成没写，不因此转只读（保守但可写）。
        assert!(!parse(r#"{"version": "one"}"#).read_only);
    }

    /// 已知键集由 schema 派生（抽查几个键名 + 规模），不是另一张手写表。
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

    /// 数组规范化：trim、去空行、去重且保序。
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
