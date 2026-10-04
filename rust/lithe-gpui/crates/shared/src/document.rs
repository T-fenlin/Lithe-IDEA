use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

pub const DOCUMENT_VERSION_KEY: &str = "version";

#[derive(Debug, Clone, PartialEq)]
pub struct Parsed<T> {
    pub value: T,
    pub previous: Option<Value>,
    pub read_only: bool,
    pub diagnostics: Vec<String>,
}

pub fn declared_version(object: &Map<String, Value>) -> Option<u64> {
    object.get(DOCUMENT_VERSION_KEY)?.as_u64()
}

pub fn known_keys<T: Default + Serialize>() -> BTreeSet<String> {
    match serde_json::to_value(T::default()) {
        Ok(Value::Object(object)) => object.keys().cloned().collect(),
        _ => BTreeSet::new(),
    }
}

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
            diagnostics.push(format!("document_fallback error={error}"));
            T::default()
        }
    }
}

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

pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<usize> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let json = serde_json::to_string_pretty(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let text = format!("{json}\n");

    let tmp = tmp_path(path);
    std::fs::write(&tmp, text.as_bytes())?;
    std::fs::rename(&tmp, path)?;
    Ok(text.len())
}

pub fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

pub fn read_document_text(path: &Path) -> io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

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

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    struct Fixture {
        name: Option<String>,
        count: u32,
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lithe-document-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

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

    #[test]
    fn known_keys_come_from_the_document_type() {
        let keys = known_keys::<Fixture>();
        assert_eq!(
            keys.into_iter().collect::<Vec<_>>(),
            vec!["count".to_string(), "name".to_string()]
        );
    }

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

    #[test]
    fn non_object_previous_does_not_break_the_merge() {
        let next = serde_json::json!({ "a": 1 });
        assert_eq!(preserve_unknown(Some(&Value::from(7)), &next), next);
        assert_eq!(preserve_unknown(None, &next), next);
    }

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

    #[test]
    fn reading_a_missing_document_does_not_create_it() {
        let dir = temp_dir("missing");
        let path = dir.join("document.json");
        assert_eq!(read_document_text(&path).expect("读失败"), None);
        assert!(!path.exists(), "读取不得创建文件");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
