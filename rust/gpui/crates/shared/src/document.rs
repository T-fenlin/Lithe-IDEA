//! 版本化 JSON 文档的通用原语：解析、未知键保留、原子写。
//!
//! ## 为什么在 `shared` 而不是某个 feature 里
//!
//! 这套东西原本只长在 `lithe-gpui-settings` 的 `persistence.rs` 里，但它跟"设置"这个语义
//! 没有关系：它描述的是**任何一份可以被手改的版本化 JSON 文档**该怎么读写。现在有两个真实
//! 使用方（设置文档、`.lithe/project.json`），所以按《编码指南》的"有名字 + 两个以上使用方"
//! 提到共享层，而不是让第二份文档再抄一遍"临时文件 + rename"和"未知键保留"。
//!
//! ## 两条语义（两条都是"文件是给人改的"逼出来的）
//!
//! - **顶层 `version` 不属于任何 typed 结构体**：写回时由本层注入，读入时先于 typed 解析读取。
//!   这样每个文档各自声明它支持的版本，而结构体只描述它真正拥有的字段。
//! - **未知键原样保留（含嵌套对象内部的键）**：读入时留下整份原始对象，
//!   写回时先合并再落盘（[`merge_document`]）。否则用户手写的字段会在下一次落盘时被吃掉，
//!   而那是不可逆的数据丢失。
//!
//! ## 键表由 schema 派生
//!
//! [`known_keys`] 直接取 `T::default()` 的序列化结果，**没有第二份键名清单**。坏键回落路径
//! 遍历它，所以新加的字段自动进入回落路径。历史教训见 `lithe-gpui-settings` 的 `PLAN.md` §14.2：
//! 手写逐键表漏登记就表现为"设置写得出、读不回"，而且**一点诊断都没有**。
//!
//! ⚠️ **派生机制的两条硬约束**（都是"逐键回落"能不能成立的前提）：
//!
//! 1. **文档类型必须在容器级加 `#[serde(default)]`**。缺某个键时 serde 才能回落到该字段的
//!    默认值；否则"另一个键坏掉"会让逐键路径上拿到的 sanitized 对象仍然缺键，整份解析再失败
//!    一次，最后退化成**全默认值** —— 回落粒度从"一个键"变成"整份"。
//! 2. **字段不能加 `skip_serializing_if`**。一旦默认值不序列化它，它就不会进 [`known_keys`]，
//!    于是那个字段会被当成未知键读不回来 —— 正好是派生机制要消灭的那类静默失效。
//!    代价是默认值文档里会出现 `null` 字段，可以接受。

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

/// 文档版本键名（每个文档都一样，所以放在本层）。
pub const DOCUMENT_VERSION_KEY: &str = "version";

/// 一次解析的结果（纯数据，不碰文件系统）。
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed<T> {
    /// 解析出来的值。**任何失败路径都返回可用的值**（最差就是 `T::default()`）。
    pub value: T,
    /// 文件里的原始对象；写回时用它保留未知键。`None` = 没有可保留的文档
    /// （文件不存在、不是 JSON、或不是对象）。
    pub previous: Option<Value>,
    /// 文件声明的版本高于调用方支持的版本：调用方**不得覆盖**这个文件。
    pub read_only: bool,
    /// 逐键诊断（坏键、整份解析失败、版本过新等）。调用方负责打印。
    pub diagnostics: Vec<String>,
}

/// 文档里声明的版本；缺失或不是非负整数时返回 `None`。
pub fn declared_version(object: &Map<String, Value>) -> Option<u64> {
    object.get(DOCUMENT_VERSION_KEY)?.as_u64()
}

/// 由**文档类型自己**派生的已知键集：`T::default()` 的序列化结果。
///
/// 这是本模块唯一的键名来源。不要再加第二份手写清单——见模块文档。
pub fn known_keys<T: Default + Serialize>() -> BTreeSet<String> {
    match serde_json::to_value(T::default()) {
        Ok(Value::Object(object)) => object.keys().cloned().collect(),
        // 纯标量结构序列化不会失败；真失败时键集为空，坏键回落路径会退化成
        // "全部用默认值"（仍然不 panic）。
        _ => BTreeSet::new(),
    }
}

/// 解析文档文本：逐键容错 + 保留原始文档 + 版本判定。**永不失败**。
///
/// `supported_version` 是调用方支持的版本；文件声明更高的版本时结果标成只读
/// （照常读取能读懂的部分，但调用方不得覆盖）。
pub fn parse<T>(text: &str, supported_version: u32) -> Parsed<T>
where
    T: Default + DeserializeOwned + Serialize,
{
    let mut diagnostics = Vec::new();

    let parsed: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("invalid_json error={error}"));
            return Parsed {
                value: T::default(),
                previous: None,
                read_only: false,
                diagnostics,
            };
        }
    };

    let Value::Object(object) = parsed else {
        diagnostics.push("not_an_object".to_string());
        return Parsed {
            value: T::default(),
            previous: None,
            read_only: false,
            diagnostics,
        };
    };

    // 版本过新时照常读取能读懂的部分，但把文件标成只读：覆盖它等于把用户在新版本里
    // 设置的东西（以及我们不认识的键）无声地降级掉。
    let declared = declared_version(&object);
    let read_only = declared.is_some_and(|version| version > u64::from(supported_version));
    if let Some(version) = declared.filter(|_| read_only) {
        diagnostics.push(format!(
            "document_version_newer declared={version} supported={supported_version}"
        ));
    }

    let value = value_from_object::<T>(&object, &mut diagnostics);
    Parsed {
        value,
        previous: Some(Value::Object(object)),
        read_only,
        diagnostics,
    }
}

/// 逐键取值：键不存在 → 默认值；键的类型坏了 → **该键**默认值 + 诊断。
///
/// 快路径是整体 `serde` 解析一次（任何一个键的类型坏掉都会失败），失败后退到
/// "逐个已知键单独试解析"：能解析的留下，不能的留诊断并回落默认值。
///
/// ⚠️ 逐键那一步遍历的是 [`known_keys`]（由 schema 派生），**不是**手写清单——
/// 所以新加的字段不需要在这里登记第二遍。
fn value_from_object<T>(object: &Map<String, Value>, diagnostics: &mut Vec<String>) -> T
where
    T: Default + DeserializeOwned + Serialize,
{
    if let Ok(parsed) = serde_json::from_value::<T>(Value::Object(object.clone())) {
        return parsed;
    }

    let known = known_keys::<T>();
    let mut sanitized = Map::new();
    for (key, value) in object {
        if !known.contains(key) {
            continue;
        }
        let mut probe = Map::new();
        probe.insert(key.clone(), value.clone());
        match serde_json::from_value::<T>(Value::Object(probe)) {
            Ok(_) => {
                sanitized.insert(key.clone(), value.clone());
            }
            Err(error) => diagnostics.push(format!("bad_key key={key} error={error}")),
        }
    }

    match serde_json::from_value::<T>(Value::Object(sanitized)) {
        Ok(parsed) => parsed,
        Err(error) => {
            // 中性名字：本模块服务所有文档类型，日志里出现 "settings" 会把排查引偏。
            diagnostics.push(format!("document_fallback error={error}"));
            T::default()
        }
    }
}

/// 生成要落盘的文档：typed 值 + **保留**上一次文件里的未知键 + 写入 `version`。
///
/// 两件事必须一起做，缺一不可：
///
/// - `value` 覆盖它自己声明的键（所以文档改动一定生效）；
/// - 上一次文件里出现、而 `value` 不认识的键逐字保留（含嵌套对象内部的键），
///   否则用户手写的字段会被无声吃掉。
///
/// `previous` 传 `None`（首次写、或上次文件不可解析）时就是一份干净的新文档。
pub fn merge_document<T: Serialize>(
    previous: Option<&Value>,
    value: &T,
    version: u32,
) -> Result<Value, serde_json::Error> {
    let typed = serde_json::to_value(value)?;
    let mut merged = preserve_unknown(previous, &typed);
    if let Value::Object(object) = &mut merged {
        object.insert(DOCUMENT_VERSION_KEY.to_string(), Value::from(version));
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
pub fn preserve_unknown(previous: Option<&Value>, next: &Value) -> Value {
    let (Some(Value::Object(previous)), Value::Object(next)) = (previous, next) else {
        return next.clone();
    };

    let mut merged = next.clone();
    for (key, previous_value) in previous {
        if !merged.contains_key(key) {
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

/// 原子写文档：合并未知键 → 临时文件 → rename。
pub fn save_document<T: Serialize>(
    path: &Path,
    value: &T,
    previous: Option<&Value>,
    version: u32,
) -> io::Result<usize> {
    let document = merge_document(previous, value, version)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    save_json(path, &document)
}

/// 原子写任意 JSON 载荷：临时文件 + rename。
///
/// 目录不存在时先建（首次启动要能落盘）。做成泛型是为了让同目录的其它 JSON 文件
/// （`recent-projects.json`、`.lithe/*.json`）复用同一套"临时文件 + rename"语义 ——
/// 两份原子写的实现迟早会在"断电只留半个文件"这类细节上漂移。
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<usize> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let json = serde_json::to_string_pretty(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let text = format!("{json}\n");

    // 临时文件名固定（不用随机后缀）：每个文件只有一个写者，而固定后缀让"崩溃后残留的
    // 临时文件"可预期、可清理。
    let tmp = tmp_path(path);
    std::fs::write(&tmp, text.as_bytes())?;
    std::fs::rename(&tmp, path)?;
    Ok(text.len())
}

/// 临时文件路径（任意文档：`<name>.json` → `<name>.json.tmp`）。
pub fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

/// 读取文档文本；文件不存在时返回 `None`（**不创建文件**）。
pub fn read_document_text(path: &Path) -> io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// 读上一次落盘的原始对象，用来在写回时保留未知键。
///
/// 文件不存在 / 读不了 / 不是 JSON / 不是对象 → `None`（写回时就是一份干净的新文档）。
/// **不创建文件**，也不产出诊断：调用方通常刚读过它，重复报诊断只会吵。
pub fn previous_object(path: &Path) -> Option<Value> {
    let text = read_document_text(path).ok().flatten()?;
    match serde_json::from_str::<Value>(&text) {
        Ok(value @ Value::Object(_)) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    /// 一个与任何 feature 无关的文档类型：证明这套原语不是给单个结构体写的。
    ///
    /// ⚠️ `#[serde(default)]` 是**必须**的：去掉它，"某个键坏了"就会退化成整份默认值
    /// （见模块文档的第 1 条硬约束）。
    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    struct Fixture {
        name: Option<String>,
        count: u32,
    }

    /// 独立的临时目录，用完删掉。目录名带上调用方标记，避免并行测试互相踩。
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lithe-document-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// 一个"**单个键各自合法、组合起来不合法**"的文档类型。
    ///
    /// 它存在的唯一理由：逐键探测看不到跨字段约束，所以 sanitized 对象仍可能整份解析失败 ——
    /// 那条路就是 `document_fallback` 诊断的来源。没有这个类型，那条防御分支无法被覆盖。
    #[derive(Debug, Clone, Default, PartialEq, Serialize)]
    struct Ordered {
        lower: u32,
        upper: u32,
    }

    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct OrderedRaw {
        lower: Option<u32>,
        upper: Option<u32>,
    }

    impl<'de> Deserialize<'de> for Ordered {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let raw = OrderedRaw::deserialize(deserializer)?;
            // 只在两个字段都出现时校验：单个键各自出现时一律合法（这正是逐键探测的前提）。
            if let (Some(lower), Some(upper)) = (raw.lower, raw.upper) {
                if upper < lower {
                    return Err(serde::de::Error::custom("upper 小于 lower"));
                }
            }
            Ok(Self {
                lower: raw.lower.unwrap_or(0),
                upper: raw.upper.unwrap_or(0),
            })
        }
    }

    /// 键集来自文档类型自己（`T::default()` 的序列化结果）。
    #[test]
    fn known_keys_come_from_the_document_type() {
        let keys = known_keys::<Fixture>();
        assert_eq!(
            keys.into_iter().collect::<Vec<_>>(),
            vec!["count".to_string(), "name".to_string()]
        );
    }

    /// 换一种文档类型，未知键（含嵌套）照样保留，`version` 照样由本层注入。
    #[test]
    fn unknown_keys_survive_a_write_for_any_document() {
        let previous = serde_json::json!({
            "name": "old",
            "stranger": { "deep": [1, 2] },
            "version": 0
        });
        let value = Fixture {
            name: Some("new".to_string()),
            count: 7,
        };
        let merged = merge_document(Some(&previous), &value, 1).expect("合并失败");
        assert_eq!(merged.get("name"), Some(&Value::from("new")));
        assert_eq!(merged.get("count"), Some(&Value::from(7)));
        assert_eq!(
            merged.get("stranger"),
            Some(&serde_json::json!({ "deep": [1, 2] })),
            "未知的嵌套对象必须整体保留"
        );
        assert_eq!(merged.get(DOCUMENT_VERSION_KEY), Some(&Value::from(1)));
    }

    /// 坏键只回落那一个键，其余照用（对任何文档类型都成立）。
    #[test]
    fn each_field_falls_back_alone_for_any_document() {
        let parsed = parse::<Fixture>(r#"{"name": "kept", "count": "three"}"#, 1);
        assert_eq!(parsed.value.name.as_deref(), Some("kept"));
        assert_eq!(parsed.value.count, 0);
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|line| line.contains("bad_key key=count")),
            "{:?}",
            parsed.diagnostics
        );
    }

    /// 更高的文档版本 → 照常读取，但标成只读。
    #[test]
    fn higher_version_is_read_only() {
        let parsed = parse::<Fixture>(r#"{"name": "x", "version": 99}"#, 1);
        assert!(parsed.read_only);
        assert_eq!(parsed.value.name.as_deref(), Some("x"));
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|line| line.contains("document_version_newer"))
        );

        assert!(!parse::<Fixture>(r#"{"version": 1}"#, 1).read_only);
        assert!(!parse::<Fixture>(r#"{"version": "one"}"#, 1).read_only);
    }

    /// 嵌套未知键的递归合并规则。
    #[test]
    fn nested_unknown_keys_are_preserved() {
        let previous = serde_json::json!({
            "outer": { "known": "old", "unknown": 42 },
            "gone": true
        });
        let next = serde_json::json!({ "outer": { "known": "new" } });
        assert_eq!(
            preserve_unknown(Some(&previous), &next),
            serde_json::json!({
                "outer": { "known": "new", "unknown": 42 },
                "gone": true
            })
        );
    }

    /// 非对象 / 缺失的 `previous` 不会破坏合并（typed 直接胜出）。
    #[test]
    fn non_object_previous_does_not_break_the_merge() {
        let next = serde_json::json!({ "a": 1 });
        assert_eq!(preserve_unknown(Some(&Value::from(7)), &next), next);
        assert_eq!(preserve_unknown(None, &next), next);
    }

    /// 跨字段约束失败 → **文档级**诊断（`document_fallback`），而不是逐键诊断。
    ///
    /// 这是"逐键回落"覆盖不到的一类失败：每个键单看都合法，组合起来不合法。此时整份回落
    /// 默认值，诊断必须说清是哪一层失败的（否则排查时会以为某个键坏了）。
    #[test]
    fn aggregate_failures_report_a_document_level_diagnostic() {
        let parsed = parse::<Ordered>(r#"{"lower": 5, "upper": 3}"#, 1);
        assert_eq!(parsed.value, Ordered::default(), "整份回落默认值");
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|line| line.starts_with("document_fallback")),
            "跨字段失败必须留下文档级诊断：{:?}",
            parsed.diagnostics
        );
        assert!(
            !parsed
                .diagnostics
                .iter()
                .any(|line| line.contains("bad_key")),
            "每个键单看都合法，不该报成坏键：{:?}",
            parsed.diagnostics
        );
    }

    /// 原子写：文件级往返、`version` 落盘、临时文件被 rename 掉。
    #[test]
    fn save_document_writes_a_versioned_file() {
        let dir = temp_dir("save");
        let path = dir.join("document.json");

        let value = Fixture {
            name: Some("written".to_string()),
            count: 3,
        };
        let bytes = save_document(&path, &value, None, 1).expect("写入失败");
        assert!(bytes > 0);
        assert!(!tmp_path(&path).exists(), "临时文件必须被 rename 掉");

        let text = read_document_text(&path)
            .expect("读失败")
            .expect("文件应存在");
        assert!(text.contains("\"version\": 1"), "{text}");
        assert!(text.contains("\"name\": \"written\""), "{text}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 读取不存在的文件返回 `None`，且**不创建文件**。
    #[test]
    fn reading_a_missing_document_does_not_create_it() {
        let dir = temp_dir("missing");
        let path = dir.join("document.json");
        assert_eq!(read_document_text(&path).expect("读失败"), None);
        assert!(!path.exists(), "读取不得创建文件");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
