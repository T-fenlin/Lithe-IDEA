use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::persistence;

pub const MAX_RECENT_PROJECTS: usize = 12;

pub const RECENT_PROJECTS_KEY: &str = "recentProjects";

pub const RECENT_PROJECTS_DIAGNOSTIC_TAG: &str = "S1_SETTINGS_RECENT";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    #[serde(default)]
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub last_opened_at: u64,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub missing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_in_new_window: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentProjects {
    #[serde(
        default,
        rename = "recentProjects",
        deserialize_with = "deserialize_entries"
    )]
    entries: Vec<RecentProject>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchRootPick {
    pub root: Option<String>,
    pub missing: Vec<String>,
}

impl RecentProjects {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select_launch_root(&self, mut is_dir: impl FnMut(&str) -> bool) -> LaunchRootPick {
        let mut pick = LaunchRootPick::default();
        for entry in &self.entries {
            if is_dir(&entry.path) {
                if pick.root.is_none() {
                    pick.root = Some(entry.path.clone());
                }
            } else {
                pick.missing.push(entry.path.clone());
            }
        }
        pick
    }

    pub fn entries(&self) -> &[RecentProject] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn find(&self, path: &str) -> Option<&RecentProject> {
        self.entries.iter().find(|entry| entry.path == path)
    }

    pub fn record_open(
        &mut self,
        path: &str,
        opened_at_ms: u64,
        open_in_new_window: Option<bool>,
    ) -> bool {
        if path.trim().is_empty() {
            return false;
        }

        let existing = self.entries.iter().find(|entry| entry.path == path);
        let next = RecentProject {
            name: existing
                .map(|entry| entry.name.clone())
                .unwrap_or_else(|| folder_name(path)),
            path: path.to_string(),
            last_opened_at: opened_at_ms,
            pinned: existing.map(|entry| entry.pinned).unwrap_or(false),
            missing: false,
            open_in_new_window: open_in_new_window
                .or_else(|| existing.and_then(|entry| entry.open_in_new_window)),
        };

        self.entries.retain(|entry| entry.path != path);
        self.entries.insert(0, next);
        self.entries = normalize(std::mem::take(&mut self.entries));
        true
    }

    pub fn set_pinned(&mut self, path: &str, pinned: bool) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.path == path) else {
            return false;
        };
        entry.pinned = pinned;
        self.entries = normalize(std::mem::take(&mut self.entries));
        true
    }

    pub fn toggle_pinned(&mut self, path: &str) -> bool {
        let Some(entry) = self.entries.iter().find(|entry| entry.path == path) else {
            return false;
        };
        let pinned = !entry.pinned;
        self.set_pinned(path, pinned)
    }

    pub fn set_missing(&mut self, path: &str, missing: bool) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.path == path) else {
            return false;
        };
        entry.missing = missing;
        true
    }

    pub fn remove(&mut self, path: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.path != path);
        self.entries.len() != before
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

pub fn folder_name(path: &str) -> String {
    let separator = if path.contains('\\') { '\\' } else { '/' };
    path.split(separator)
        .filter(|segment| !segment.is_empty())
        .next_back()
        .unwrap_or(path)
        .to_string()
}

pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedRecentProjects {
    pub projects: RecentProjects,
    pub path: Option<PathBuf>,
    pub file_existed: bool,
    pub diagnostics: Vec<String>,
}

impl LoadedRecentProjects {
    pub fn diagnostic_line(&self) -> String {
        let path_text = self
            .path
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "(none)".to_string());
        let pinned = self
            .projects
            .entries()
            .iter()
            .filter(|entry| entry.pinned)
            .count();
        format!(
            "{RECENT_PROJECTS_DIAGNOSTIC_TAG} loaded path={path_text} count={} pinned={pinned} fileExisted={} diagnostics={}",
            self.projects.len(),
            self.file_existed,
            self.diagnostics.len()
        )
    }

    pub fn report(&self) {
        println!("{}", self.diagnostic_line());
        for diagnostic in &self.diagnostics {
            eprintln!("{RECENT_PROJECTS_DIAGNOSTIC_TAG} {diagnostic}");
        }
    }
}

pub fn load() -> LoadedRecentProjects {
    load_from(crate::paths::recent_projects_file_path())
}

pub fn load_from(path: Option<PathBuf>) -> LoadedRecentProjects {
    let Some(path) = path else {
        return LoadedRecentProjects {
            projects: RecentProjects::new(),
            path: None,
            file_existed: false,
            diagnostics: vec![
                "no_recent_projects_path (推导不出最近项目文件路径，本次会话不落盘)".to_string(),
            ],
        };
    };

    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let (projects, diagnostics) = load_from_str(&text);
            LoadedRecentProjects {
                projects,
                path: Some(path),
                file_existed: true,
                diagnostics,
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => LoadedRecentProjects {
            projects: RecentProjects::new(),
            path: Some(path),
            file_existed: false,
            diagnostics: Vec::new(),
        },
        Err(error) => LoadedRecentProjects {
            projects: RecentProjects::new(),
            path: Some(path),
            file_existed: false,
            diagnostics: vec![format!("read_failed error={error}")],
        },
    }
}

pub fn load_from_str(text: &str) -> (RecentProjects, Vec<String>) {
    let mut diagnostics = Vec::new();

    let parsed: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("invalid_json error={error}"));
            return (RecentProjects::new(), diagnostics);
        }
    };

    let Value::Object(object) = parsed else {
        diagnostics.push("not_an_object".to_string());
        return (RecentProjects::new(), diagnostics);
    };

    let Some(value) = object.get(RECENT_PROJECTS_KEY) else {
        return (RecentProjects::new(), diagnostics);
    };
    let Value::Array(items) = value else {
        diagnostics.push(format!("bad_key key={RECENT_PROJECTS_KEY}"));
        return (RecentProjects::new(), diagnostics);
    };

    let mut entries = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        match serde_json::from_value::<RecentProject>(item.clone()) {
            Ok(entry) if entry.path.trim().is_empty() => {
                diagnostics.push(format!("bad_entry index={index} error=missing_path"));
            }
            Ok(entry) => entries.push(entry),
            Err(error) => diagnostics.push(format!("bad_entry index={index} error={error}")),
        }
    }

    let parsed_count = entries.len();
    let entries = normalize(entries);
    if entries.len() < parsed_count {
        diagnostics.push(format!(
            "normalized removed={}",
            parsed_count - entries.len()
        ));
    }

    (RecentProjects { entries }, diagnostics)
}

pub fn save(path: &Path, projects: &RecentProjects) -> io::Result<usize> {
    persistence::save_json(path, projects)
}

fn deserialize_entries<'de, D>(deserializer: D) -> Result<Vec<RecentProject>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let entries = Vec::<RecentProject>::deserialize(deserializer)?;
    Ok(normalize(entries))
}

fn normalize(entries: Vec<RecentProject>) -> Vec<RecentProject> {
    let mut index_of: HashMap<String, usize> = HashMap::with_capacity(entries.len());
    let mut kept: Vec<RecentProject> = Vec::with_capacity(entries.len());
    for mut entry in entries {
        if entry.path.trim().is_empty() {
            continue;
        }
        if entry.name.is_empty() {
            entry.name = folder_name(&entry.path);
        }
        match index_of.get(&entry.path).copied() {
            Some(index) => {
                if entry.last_opened_at > kept[index].last_opened_at {
                    kept[index] = entry;
                }
            }
            None => {
                index_of.insert(entry.path.clone(), kept.len());
                kept.push(entry);
            }
        }
    }

    kept.sort_by(|left, right| {
        right
            .pinned
            .cmp(&left.pinned)
            .then_with(|| right.last_opened_at.cmp(&left.last_opened_at))
    });

    let mut unpinned = 0usize;
    kept.retain(|entry| {
        if entry.pinned {
            return true;
        }
        unpinned += 1;
        unpinned <= MAX_RECENT_PROJECTS
    });

    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "lithe-recent-projects-test-{name}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            Self(dir)
        }

        fn file(&self) -> PathBuf {
            self.0.join("recent-projects.json")
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn recent_projects_survive_a_file_round_trip() {
        let dir = TempDir::new("round-trip");
        let path = dir.file();

        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\alpha", 1_700_000_000_000, Some(false)));
        assert!(projects.record_open(r"D:\proj\beta", 1_700_000_001_000, Some(true)));
        assert!(projects.set_pinned(r"D:\proj\alpha", true));
        assert!(projects.set_missing(r"D:\proj\beta", true));

        let bytes = save(&path, &projects).expect("写入失败");
        assert!(bytes > 0);
        assert!(path.exists());
        assert!(
            !crate::persistence::tmp_path(&path).exists(),
            "临时文件必须被 rename 掉"
        );

        let text = std::fs::read_to_string(&path).expect("读文件失败");
        for key in [
            "\"recentProjects\"",
            "\"path\"",
            "\"name\"",
            "\"lastOpenedAt\"",
            "\"pinned\"",
            "\"missing\"",
            "\"openInNewWindow\"",
        ] {
            assert!(text.contains(key), "落盘文本缺键 {key}：{text}");
        }

        let loaded = load_from(Some(path.clone()));
        assert!(loaded.file_existed);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(loaded.projects, projects, "整个列表必须逐字段相等");
        assert_eq!(loaded.projects.entries()[0].path, r"D:\proj\alpha");
        assert!(loaded.projects.entries()[0].pinned);
        assert!(loaded.projects.entries()[1].missing);
        assert_eq!(loaded.projects.entries()[1].open_in_new_window, Some(true));

        let mut second = projects.clone();
        second.clear();
        save(&path, &second).expect("覆盖写失败");
        let reloaded = load_from(Some(path));
        assert!(
            reloaded.diagnostics.is_empty(),
            "{:?}",
            reloaded.diagnostics
        );
        assert!(reloaded.projects.is_empty(), "清空必须落盘成空数组");
    }

    #[test]
    fn cap_keeps_only_the_twelve_most_recent_unpinned_projects() {
        let mut projects = RecentProjects::new();
        for index in 0..20u64 {
            assert!(projects.record_open(&format!(r"D:\proj\p{index}"), 1_000 + index, None));
        }

        assert_eq!(projects.len(), MAX_RECENT_PROJECTS);
        assert_eq!(MAX_RECENT_PROJECTS, 12);
        assert_eq!(projects.entries()[0].path, r"D:\proj\p19");
        assert_eq!(projects.entries()[11].path, r"D:\proj\p8");
        assert!(projects.find(r"D:\proj\p7").is_none());
        assert!(projects.find(r"D:\proj\p0").is_none());
    }

    #[test]
    fn pinned_projects_do_not_consume_the_cap() {
        let mut projects = RecentProjects::new();
        for index in 0..3u64 {
            let path = format!(r"D:\proj\pinned{index}");
            assert!(projects.record_open(&path, 1_000 + index, None));
            assert!(projects.set_pinned(&path, true));
        }
        for index in 0..15u64 {
            assert!(projects.record_open(&format!(r"D:\proj\loose{index}"), 2_000 + index, None));
        }

        assert_eq!(projects.len(), 3 + MAX_RECENT_PROJECTS);
        for index in 0..3u64 {
            assert!(
                projects.find(&format!(r"D:\proj\pinned{index}")).is_some(),
                "置顶项不得被上限截掉"
            );
        }
        assert!(projects.find(r"D:\proj\loose14").is_some());
        assert!(projects.find(r"D:\proj\loose3").is_some());
        assert!(projects.find(r"D:\proj\loose2").is_none());
    }

    #[test]
    fn order_is_pinned_first_then_most_recent() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\old", 100, None));
        assert!(projects.record_open(r"D:\proj\new", 900, None));
        assert!(projects.record_open(r"D:\proj\mid", 500, None));
        assert!(projects.set_pinned(r"D:\proj\old", true));
        assert!(projects.set_pinned(r"D:\proj\mid", true));

        let paths: Vec<&str> = projects
            .entries()
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(
            paths,
            vec![r"D:\proj\mid", r"D:\proj\old", r"D:\proj\new"],
            "置顶块在前（时间降序），未置顶块在后（时间降序）"
        );
    }

    #[test]
    fn paths_are_deduplicated_exactly() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\Proj", 100, None));
        assert!(projects.record_open(r"d:\proj", 200, None));
        assert!(projects.record_open(r"D:/Proj", 300, None));
        assert_eq!(projects.len(), 3, "精确去重 ⇒ 大小写/分隔符不同是三条");

        assert!(projects.record_open(r"D:\Proj", 400, None));
        assert_eq!(projects.len(), 3, "同一条 path 不得产生第二项");
        let found: Vec<u64> = projects
            .entries()
            .iter()
            .filter(|entry| entry.path == r"D:\Proj")
            .map(|entry| entry.last_opened_at)
            .collect();
        assert_eq!(found, vec![400]);
    }

    #[test]
    fn recording_the_same_path_updates_in_place() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\alpha", 100, Some(true)));
        assert!(projects.set_pinned(r"D:\proj\alpha", true));
        assert!(projects.set_missing(r"D:\proj\alpha", true));
        assert!(projects.record_open(r"D:\proj\beta", 200, None));

        assert!(projects.record_open(r"D:\proj\alpha", 300, Some(false)));
        assert_eq!(projects.len(), 2, "不得产生重复项");
        assert_eq!(
            projects.entries()[0].path,
            r"D:\proj\alpha",
            "刚打开的提到最前"
        );
        let alpha = projects.find(r"D:\proj\alpha").expect("α 必须在列表里");
        assert_eq!(alpha.last_opened_at, 300, "时间必须更新");
        assert!(alpha.pinned, "重开不改变置顶");
        assert!(!alpha.missing, "重开一次就是又能用了");
        assert_eq!(alpha.open_in_new_window, Some(false), "打开方式按本次更新");
        assert_eq!(
            alpha.name,
            folder_name(r"D:\proj\alpha"),
            "名字仍是路径末段"
        );

        assert!(projects.record_open(r"D:\proj\alpha", 400, None));
        assert_eq!(
            projects.find(r"D:\proj\alpha").unwrap().open_in_new_window,
            Some(false)
        );
    }

    #[test]
    fn recording_keeps_the_existing_display_name() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\alpha", 100, None));
        projects.entries[0].name = "自定义别名".to_string();
        assert!(projects.record_open(r"D:\proj\alpha", 200, None));
        assert_eq!(projects.find(r"D:\proj\alpha").unwrap().name, "自定义别名");
    }

    #[test]
    fn empty_paths_are_not_recorded() {
        let mut projects = RecentProjects::new();
        assert!(!projects.record_open("", 100, None));
        assert!(!projects.record_open("   ", 100, None));
        assert!(projects.is_empty());
    }

    #[test]
    fn broken_json_yields_an_empty_list_with_a_diagnostic() {
        let (projects, diagnostics) = load_from_str("{ not json ");
        assert!(projects.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0].starts_with("invalid_json"),
            "{diagnostics:?}"
        );

        let (projects, diagnostics) = load_from_str("[1, 2, 3]");
        assert!(projects.is_empty());
        assert_eq!(diagnostics, vec!["not_an_object".to_string()]);

        let (projects, diagnostics) = load_from_str(r#"{"recentProjects": "nope"}"#);
        assert!(projects.is_empty());
        assert_eq!(diagnostics, vec!["bad_key key=recentProjects".to_string()]);

        let (projects, diagnostics) = load_from_str(r#"{"other": 1}"#);
        assert!(projects.is_empty());
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn a_broken_entry_falls_back_alone() {
        let (projects, diagnostics) = load_from_str(
            r#"{
                "recentProjects": [
                    {"path": "D:\\proj\\good", "name": "good", "lastOpenedAt": 300},
                    {"name": "no-path", "lastOpenedAt": 200},
                    {"path": 7, "name": "wrong-type"},
                    {"path": "D:\\proj\\noname", "lastOpenedAt": 100}
                ]
            }"#,
        );

        let paths: Vec<&str> = projects
            .entries()
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(paths, vec![r"D:\proj\good", r"D:\proj\noname"]);
        assert_eq!(
            projects.find(r"D:\proj\noname").unwrap().name,
            "noname",
            "缺 name 用路径末段补"
        );
        assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
        assert!(
            diagnostics[0].contains("bad_entry index=1"),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics[1].contains("bad_entry index=2"),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn duplicates_in_a_hand_edited_file_collapse_to_the_newest() {
        let (projects, diagnostics) = load_from_str(
            r#"{
                "recentProjects": [
                    {"path": "D:\\proj\\a", "name": "a", "lastOpenedAt": 100},
                    {"path": "D:\\proj\\b", "name": "b", "lastOpenedAt": 200},
                    {"path": "D:\\proj\\a", "name": "a2", "lastOpenedAt": 300}
                ]
            }"#,
        );

        assert_eq!(projects.len(), 2);
        let a = projects.find(r"D:\proj\a").unwrap();
        assert_eq!(a.last_opened_at, 300, "保留的是最新的那条");
        assert_eq!(a.name, "a2");
        assert_eq!(diagnostics, vec!["normalized removed=1".to_string()]);
    }

    #[test]
    fn an_over_limit_file_is_trimmed_on_load() {
        let mut items = Vec::new();
        for index in 0..15u64 {
            items.push(format!(
                r#"{{"path": "D:\\proj\\p{index}", "name": "p{index}", "lastOpenedAt": {}}}"#,
                1_000 + index
            ));
        }
        items.push(
            r#"{"path": "D:\\proj\\pinned", "name": "pinned", "lastOpenedAt": 1, "pinned": true}"#
                .to_string(),
        );
        let text = format!(r#"{{"recentProjects": [{}]}}"#, items.join(","));

        let (projects, diagnostics) = load_from_str(&text);
        assert_eq!(projects.len(), MAX_RECENT_PROJECTS + 1);
        assert!(projects.find(r"D:\proj\pinned").unwrap().pinned);
        assert!(projects.find(r"D:\proj\p14").is_some());
        assert!(projects.find(r"D:\proj\p2").is_none());
        assert_eq!(diagnostics, vec!["normalized removed=3".to_string()]);
    }

    #[test]
    fn empty_and_missing_files_behave_like_a_first_launch() {
        let (projects, diagnostics) = load_from_str("");
        assert!(projects.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0].starts_with("invalid_json"),
            "{diagnostics:?}"
        );

        let dir = TempDir::new("missing-file");
        let path = dir.file();
        assert!(!dir.0.exists());
        let loaded = load_from(Some(path.clone()));
        assert!(loaded.projects.is_empty());
        assert!(!loaded.file_existed);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert!(!path.exists(), "读取不得创建文件");

        std::fs::create_dir_all(&dir.0).expect("建目录失败");
        std::fs::write(&path, b"").expect("写空文件失败");
        let loaded = load_from(Some(path.clone()));
        assert!(loaded.projects.is_empty());
        assert!(loaded.file_existed);
        assert_eq!(loaded.diagnostics.len(), 1);
        assert!(
            loaded.diagnostics[0].starts_with("invalid_json"),
            "{:?}",
            loaded.diagnostics
        );

        let loaded = load_from(None);
        assert!(loaded.projects.is_empty());
        assert!(loaded.path.is_none());
        assert_eq!(loaded.diagnostics.len(), 1);
        assert!(loaded.diagnostics[0].starts_with("no_recent_projects_path"));
    }

    #[test]
    fn maintenance_actions_report_whether_they_matched() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\a", 100, None));
        assert!(projects.record_open(r"D:\proj\b", 200, None));

        assert!(projects.toggle_pinned(r"D:\proj\a"));
        assert!(projects.find(r"D:\proj\a").unwrap().pinned);
        assert!(
            projects.entries()[0].path.ends_with('a'),
            "置顶后回到列表最前"
        );
        assert!(projects.toggle_pinned(r"D:\proj\a"));
        assert!(!projects.find(r"D:\proj\a").unwrap().pinned);

        assert!(projects.set_missing(r"D:\proj\b", true));
        assert!(projects.find(r"D:\proj\b").unwrap().missing);
        assert!(projects.set_missing(r"D:\proj\b", false));
        assert!(!projects.find(r"D:\proj\b").unwrap().missing);

        assert!(!projects.toggle_pinned(r"D:\proj\ghost"));
        assert!(!projects.set_pinned(r"D:\proj\ghost", true));
        assert!(!projects.set_missing(r"D:\proj\ghost", true));

        assert!(projects.remove(r"D:\proj\b"));
        assert!(!projects.remove(r"D:\proj\b"), "第二次移除没有命中");
        assert_eq!(projects.len(), 1);

        projects.clear();
        assert!(projects.is_empty());
        assert!(projects.find(r"D:\proj\a").is_none());
    }

    #[test]
    fn direct_deserialization_is_normalized_too() {
        let text = r#"{"recentProjects": [
            {"path": "D:\\proj\\old", "name": "old", "lastOpenedAt": 1},
            {"path": "D:\\proj\\new", "name": "new", "lastOpenedAt": 9},
            {"path": "D:\\proj\\new", "name": "new2", "lastOpenedAt": 5}
        ]}"#;
        let projects: RecentProjects = serde_json::from_str(text).expect("解析失败");
        let paths: Vec<&str> = projects
            .entries()
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(paths, vec![r"D:\proj\new", r"D:\proj\old"]);
        assert_eq!(projects.find(r"D:\proj\new").unwrap().last_opened_at, 9);
    }

    #[test]
    fn folder_name_is_the_last_segment() {
        assert_eq!(
            folder_name(r"D:\developmentProjects\Lithe-IDEA"),
            "Lithe-IDEA"
        );
        assert_eq!(folder_name("/home/dev/lithe/"), "lithe");
        assert_eq!(folder_name("lithe"), "lithe");
        assert_eq!(folder_name(r"C:\"), "C:");
        assert_eq!(folder_name(""), "");
        assert_eq!(folder_name("///"), "///");
    }

    #[test]
    fn diagnostic_line_reports_count_and_pinned() {
        let dir = TempDir::new("diagnostic-line");
        let path = dir.file();
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\a", 100, None));
        assert!(projects.record_open(r"D:\proj\b", 200, None));
        assert!(projects.set_pinned(r"D:\proj\a", true));
        save(&path, &projects).expect("写入失败");

        let loaded = load_from(Some(path.clone()));
        let line = loaded.diagnostic_line();
        assert!(line.starts_with(RECENT_PROJECTS_DIAGNOSTIC_TAG), "{line}");
        assert!(line.contains("count=2"), "{line}");
        assert!(line.contains("pinned=1"), "{line}");
        assert!(line.contains("fileExisted=true"), "{line}");
        assert!(line.contains("diagnostics=0"), "{line}");
        assert!(
            line.contains(&path.display().to_string()),
            "诊断行必须带路径：{line}"
        );
    }

    #[test]
    fn now_unix_ms_is_milliseconds_since_epoch() {
        let now = now_unix_ms();
        assert!(
            (1_577_836_800_000..4_102_444_800_000).contains(&now),
            "now_unix_ms 必须是毫秒：{now}"
        );
    }

    #[test]
    fn select_launch_root_on_an_empty_list_picks_nothing() {
        let projects = RecentProjects::new();
        let pick = projects.select_launch_root(|_| panic!("空列表不得探测任何路径"));

        assert_eq!(pick.root, None);
        assert!(pick.missing.is_empty(), "{pick:?}");
    }

    #[test]
    fn select_launch_root_reports_every_missing_entry() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\gone-newer", 300, None));
        assert!(projects.record_open(r"D:\proj\gone-older", 100, None));

        let pick = projects.select_launch_root(|_| false);

        assert_eq!(pick.root, None, "一条都不存在 ⇒ 没有根可选（调用方走兜底）");
        assert_eq!(
            pick.missing,
            vec![
                r"D:\proj\gone-newer".to_string(),
                r"D:\proj\gone-older".to_string()
            ],
            "失效条目按列表顺序（时间降序）交回，调用方逐条 set_missing + 落盘"
        );
    }

    #[test]
    fn select_launch_root_picks_the_most_recent_existing_project() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\gone", 300, None));
        assert!(projects.record_open(r"D:\proj\here", 200, None));
        assert!(projects.record_open(r"D:\proj\older-gone", 100, None));

        let pick = projects.select_launch_root(|path| path == r"D:\proj\here");

        assert_eq!(pick.root.as_deref(), Some(r"D:\proj\here"));
        assert_eq!(
            pick.missing,
            vec![
                r"D:\proj\gone".to_string(),
                r"D:\proj\older-gone".to_string()
            ],
            "选中之后排在后面的条目照样被探测（要标记的失效条目与选中项无关）"
        );
    }

    #[test]
    fn select_launch_root_respects_pinned_order() {
        let mut projects = RecentProjects::new();
        assert!(projects.record_open(r"D:\proj\newer", 900, None));
        assert!(projects.record_open(r"D:\proj\pinned-old", 100, None));
        assert!(projects.set_pinned(r"D:\proj\pinned-old", true));

        let pick = projects.select_launch_root(|_| true);

        assert_eq!(
            pick.root.as_deref(),
            Some(r"D:\proj\pinned-old"),
            "置顶项排在界面前面 ⇒ 双击打开的也是它（用户的置顶意图优先于最近）"
        );
        assert!(pick.missing.is_empty(), "{pick:?}");
    }

    #[test]
    fn select_launch_root_finds_the_only_surviving_project_at_the_end() {
        let mut projects = RecentProjects::new();
        for index in 0..3u64 {
            assert!(projects.record_open(&format!(r"D:\proj\gone{index}"), 100 + index, None));
        }
        assert!(projects.record_open(r"D:\proj\survivor", 1, None));

        let pick = projects.select_launch_root(|path| path == r"D:\proj\survivor");

        assert_eq!(pick.root.as_deref(), Some(r"D:\proj\survivor"));
        assert_eq!(pick.missing.len(), 3, "{pick:?}");
    }
}
