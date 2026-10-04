use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::document;
use crate::workspace_config::paths::{self, WorkspaceConfigPaths};
use crate::workspace_config::sharing;

pub const SESSION_DOCUMENT_VERSION: u32 = 1;

pub const MAX_OPEN_FILES: usize = 40;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionState {
    pub open_files: Vec<String>,
    pub active_file: Option<usize>,
    pub left_sidebar_visible: Option<bool>,
    pub top_activity_view: Option<usize>,
    pub bottom_kind: Option<String>,
    pub bottom_visible: Option<bool>,
    pub right_view: Option<String>,
    pub right_visible: Option<bool>,
    pub left_pane_width: Option<f32>,
    pub right_pane_width: Option<f32>,
    pub bottom_pane_height: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionLoaded {
    pub session: SessionState,
    pub previous: Option<Value>,
    pub read_only: bool,
    pub file_existed: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSaved {
    pub bytes: usize,
    pub exclusion: sharing::EnsureExcluded,
}

#[derive(Debug)]
pub enum SessionError {
    Exclude(sharing::SharingError),
    Io(io::Error),
    Json(serde_json::Error),
    ReadOnly {
        path: String,
        declared: u64,
        supported: u32,
    },
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exclude(error) => write!(formatter, "确保本机排除失败：{error}"),
            Self::Io(error) => write!(formatter, "会话文件读写失败：{error}"),
            Self::Json(error) => write!(formatter, "会话 JSON 生成失败：{error}"),
            Self::ReadOnly {
                path,
                declared,
                supported,
            } => write!(
                formatter,
                "文档版本 {declared} 高于本程序支持的 {supported}，拒绝覆盖：{path}"
            ),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<io::Error> for SessionError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for SessionError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub fn session_path(root: &Path) -> PathBuf {
    WorkspaceConfigPaths::new(root).session_local()
}

pub fn load_session(root: &Path) -> SessionLoaded {
    let path = session_path(root);
    match document::read_document_text(&path) {
        Ok(Some(text)) => {
            let parsed = document::parse::<SessionState>(&text, SESSION_DOCUMENT_VERSION);
            SessionLoaded {
                session: parsed.value,
                previous: parsed.previous,
                read_only: parsed.read_only,
                file_existed: true,
                diagnostics: parsed.diagnostics,
            }
        }
        Ok(None) => SessionLoaded {
            session: SessionState::default(),
            previous: None,
            read_only: false,
            file_existed: false,
            diagnostics: Vec::new(),
        },
        Err(error) => SessionLoaded {
            session: SessionState::default(),
            previous: None,
            read_only: false,
            file_existed: false,
            diagnostics: vec![format!("read_failed error={error}")],
        },
    }
}

pub fn save_session(root: &Path, session: &SessionState) -> Result<SessionSaved, SessionError> {
    let path = session_path(root);
    let previous = document::previous_object(&path);
    if let Some(Value::Object(object)) = previous.as_ref() {
        let declared = document::declared_version(object);
        let supported = u64::from(SESSION_DOCUMENT_VERSION);
        if declared.is_some_and(|version| version > supported) {
            return Err(SessionError::ReadOnly {
                path: path.display().to_string(),
                declared: declared.unwrap_or_default(),
                supported: SESSION_DOCUMENT_VERSION,
            });
        }
    }

    let exclusion = sharing::ensure_project_dir_excluded(root).map_err(SessionError::Exclude)?;
    let bytes =
        document::save_document(&path, session, previous.as_ref(), SESSION_DOCUMENT_VERSION)?;
    let _ = sharing::ensure_local_ignore_file(root);
    Ok(SessionSaved { bytes, exclusion })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionFileOutcome {
    Restored(PathBuf),
    Skipped {
        relative: String,
        reason: &'static str,
    },
}

pub fn resolve_session_files(root: &Path, session: &SessionState) -> Vec<SessionFileOutcome> {
    session
        .open_files
        .iter()
        .map(|relative| match resolve_one(root, relative) {
            Some(path) => SessionFileOutcome::Restored(path),
            None if is_plain_relative(relative) => SessionFileOutcome::Skipped {
                relative: relative.clone(),
                reason: "missing",
            },
            None => SessionFileOutcome::Skipped {
                relative: relative.clone(),
                reason: "outside_root",
            },
        })
        .collect()
}

fn resolve_one(root: &Path, relative: &str) -> Option<PathBuf> {
    if !is_plain_relative(relative) {
        return None;
    }
    let path = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    path.is_file().then_some(path)
}

fn is_plain_relative(relative: &str) -> bool {
    let normalized = relative.replace('\\', "/");
    if normalized.is_empty() {
        return false;
    }
    if normalized.starts_with('/') {
        return false;
    }
    let bytes = normalized.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        return false;
    }
    !normalized.split('/').any(|segment| segment == "..")
}

pub fn session_from_paths(
    root: &Path,
    open_paths: &[PathBuf],
    active_path: Option<&Path>,
    ui: SessionUiState,
) -> SessionState {
    let mut kept: Vec<(String, PathBuf)> = Vec::new();
    for path in open_paths {
        if !path.is_file() {
            continue;
        }
        let Some(relative) = paths::workspace_relative(root, path) else {
            continue;
        };
        if kept.iter().any(|(_, existing)| existing == path) {
            continue;
        }
        kept.push((relative, path.clone()));
    }

    let active_relative = active_path
        .filter(|active| kept.iter().any(|(_, path)| path == active))
        .and_then(|active| paths::workspace_relative(root, active));

    let mut open_files: Vec<String> = if kept.len() <= MAX_OPEN_FILES {
        kept.iter().map(|(relative, _)| relative.clone()).collect()
    } else {
        let mut bounded: Vec<String> = kept
            .iter()
            .take(MAX_OPEN_FILES - 1)
            .map(|(relative, _)| relative.clone())
            .collect();
        if let Some(active) = active_relative.as_ref() {
            if !bounded.contains(active) {
                bounded.push(active.clone());
            }
        }
        bounded
    };

    let active_file = active_relative
        .and_then(|active| open_files.iter().position(|relative| *relative == active));
    open_files.shrink_to_fit();

    SessionState {
        open_files,
        active_file,
        left_sidebar_visible: ui.left_sidebar_visible,
        top_activity_view: ui.top_activity_view,
        bottom_kind: ui.bottom_kind,
        bottom_visible: ui.bottom_visible,
        right_view: ui.right_view,
        right_visible: ui.right_visible,
        left_pane_width: ui.left_pane_width,
        right_pane_width: ui.right_pane_width,
        bottom_pane_height: ui.bottom_pane_height,
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionUiState {
    pub left_sidebar_visible: Option<bool>,
    pub top_activity_view: Option<usize>,
    pub bottom_kind: Option<String>,
    pub bottom_visible: Option<bool>,
    pub right_view: Option<String>,
    pub right_visible: Option<bool>,
    pub left_pane_width: Option<f32>,
    pub right_pane_width: Option<f32>,
    pub bottom_pane_height: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_config::test_repo::TempRepo;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lithe-session-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        dir
    }

    fn write_files(root: &Path, relatives: &[&str]) -> Vec<PathBuf> {
        relatives
            .iter()
            .map(|relative| {
                let path = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
                std::fs::create_dir_all(path.parent().expect("有父目录")).expect("建父目录");
                std::fs::write(&path, "// fixture\n").expect("写 fixture 文件");
                path
            })
            .collect()
    }

    fn ui_hidden() -> SessionUiState {
        SessionUiState {
            left_sidebar_visible: Some(false),
            top_activity_view: Some(1),
            bottom_kind: Some("terminal".to_string()),
            bottom_visible: Some(false),
            right_view: Some("maven".to_string()),
            right_visible: Some(false),
            left_pane_width: Some(268.5),
            right_pane_width: Some(413.25),
            bottom_pane_height: Some(287.75),
        }
    }

    #[test]
    fn loading_a_missing_session_does_not_create_it() {
        let dir = temp_dir("missing");
        let path = session_path(&dir);

        let loaded = load_session(&dir);
        assert_eq!(loaded.session, SessionState::default());
        assert!(!loaded.file_existed);
        assert!(loaded.diagnostics.is_empty());
        assert!(!path.exists(), "读取不得创建文件");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn broken_json_degrades_to_no_session_with_a_diagnostic() {
        let dir = temp_dir("broken");
        let path = session_path(&dir);
        std::fs::create_dir_all(path.parent().expect("有父目录")).expect("建配置目录");
        std::fs::write(&path, "{ this is not json").expect("写坏文件");

        let loaded = load_session(&dir);
        assert_eq!(loaded.session, SessionState::default());
        assert!(loaded.file_existed, "文件读到了，只是内容坏");
        assert!(
            loaded
                .diagnostics
                .iter()
                .any(|line| line.contains("invalid_json")),
            "{:?}",
            loaded.diagnostics
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_a_session_lands_under_the_excluded_directory() {
        let repo = TempRepo::new("session-save");
        repo.write("alpha.txt", "a\n");
        let session = session_from_paths(
            repo.root(),
            &[repo.root().join("alpha.txt")],
            Some(&repo.root().join("alpha.txt")),
            ui_hidden(),
        );

        let saved = save_session(repo.root(), &session).expect("写会话失败");
        assert!(saved.bytes > 0);
        assert!(!repo.status().contains(".lithe"), "{}", repo.status());

        let text = repo.read(".lithe/session.local.json");
        assert!(text.contains("\"version\": 1"), "{text}");
        assert!(text.contains("\"openFiles\""), "{text}");
        assert!(text.contains("\"alpha.txt\""), "{text}");

        let mut object: Value = serde_json::from_str(&text).expect("应是 JSON");
        object.as_object_mut().expect("应是对象").insert(
            "futureField".to_string(),
            serde_json::json!({ "deep": [1, 2] }),
        );
        std::fs::write(
            crate::workspace_config::session::session_path(repo.root()),
            serde_json::to_string_pretty(&object).expect("序列化"),
        )
        .expect("写回");

        save_session(repo.root(), &session).expect("第二次写会话失败");
        let text = repo.read(".lithe/session.local.json");
        assert!(
            text.contains("futureField"),
            "未知键必须原样保留，否则下一次落盘会吃掉别的生产者写的东西：{text}"
        );
    }

    #[test]
    fn a_newer_document_version_is_refused_instead_of_downgraded() {
        let repo = TempRepo::new("newer");
        repo.write(
            ".lithe/session.local.json",
            r#"{"version":99,"openFiles":["x.txt"]}"#,
        );

        let error =
            save_session(repo.root(), &SessionState::default()).expect_err("更高版本必须被拒绝");
        assert!(matches!(error, SessionError::ReadOnly { .. }), "{error:?}");
        assert_eq!(
            repo.read(".lithe/session.local.json"),
            r#"{"version":99,"openFiles":["x.txt"]}"#,
            "被拒绝的写入不得改动文件"
        );
        assert!(
            !repo.exclude_lines().iter().any(|line| line == ".lithe/"),
            "被拒绝的写入不该动排除文件"
        );
    }

    #[test]
    fn resolving_files_skips_missing_and_outside_paths_in_order() {
        let dir = temp_dir("resolve");
        write_files(&dir, &["alpha.txt", "gamma.txt"]);
        let session = SessionState {
            open_files: vec![
                "alpha.txt".to_string(),
                "deleted.txt".to_string(),
                "gamma.txt".to_string(),
                "../outside.txt".to_string(),
                "C:/windows/win.ini".to_string(),
                "".to_string(),
            ],
            ..SessionState::default()
        };

        let resolved = resolve_session_files(&dir, &session);
        assert_eq!(resolved.len(), 6, "逐条都要给出结论");
        assert_eq!(
            resolved[0],
            SessionFileOutcome::Restored(dir.join("alpha.txt"))
        );
        assert_eq!(
            resolved[1],
            SessionFileOutcome::Skipped {
                relative: "deleted.txt".to_string(),
                reason: "missing",
            }
        );
        assert_eq!(
            resolved[2],
            SessionFileOutcome::Restored(dir.join("gamma.txt"))
        );
        for outcome in &resolved[3..] {
            assert!(
                matches!(
                    outcome,
                    SessionFileOutcome::Skipped {
                        reason: "outside_root",
                        ..
                    }
                ),
                "逃出根 / 绝对路径必须按 outside_root 跳过：{outcome:?}"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn files_outside_the_root_are_not_recorded() {
        let dir = temp_dir("outside");
        let root = dir.join("project");
        std::fs::create_dir_all(&root).expect("建工作区根");
        let inside = write_files(&root, &["inside.txt"]);
        let outside_dir = temp_dir("outside-file");
        let outside = write_files(&outside_dir, &["outside.txt"]);

        let mut paths = inside.clone();
        paths.extend(outside.iter().cloned());
        let session = session_from_paths(&root, &paths, Some(&inside[0]), ui_hidden());

        assert_eq!(session.open_files, vec!["inside.txt".to_string()]);
        assert_eq!(session.active_file, Some(0));

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&outside_dir);
    }

    #[test]
    fn missing_files_are_not_recorded() {
        let dir = temp_dir("missing-file");
        let present = write_files(&dir, &["present.txt"]);
        let gone = dir.join("gone.txt");

        let session = session_from_paths(
            &dir,
            &[present[0].clone(), gone.clone()],
            Some(&gone),
            SessionUiState::default(),
        );
        assert_eq!(session.open_files, vec!["present.txt".to_string()]);
        assert_eq!(
            session.active_file, None,
            "活动文件被过滤掉时不能留下指向别人的下标"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_open_file_list_is_bounded_and_keeps_the_active_file() {
        let dir = temp_dir("bounded");
        let relatives: Vec<String> = (0..(MAX_OPEN_FILES + 5))
            .map(|index| format!("file-{index:03}.txt"))
            .collect();
        let borrowed: Vec<&str> = relatives.iter().map(String::as_str).collect();
        let paths = write_files(&dir, &borrowed);

        let session = session_from_paths(&dir, &paths, Some(&paths[1]), ui_hidden());
        assert_eq!(session.open_files.len(), MAX_OPEN_FILES - 1);
        assert_eq!(session.active_file, Some(1));
        assert_eq!(session.open_files[0], "file-000.txt");
        let expected_tail = format!("file-{:03}.txt", MAX_OPEN_FILES - 2);
        assert_eq!(
            session.open_files.last().map(String::as_str),
            Some(expected_tail.as_str())
        );

        let last = paths.len() - 1;
        let session = session_from_paths(&dir, &paths, Some(&paths[last]), ui_hidden());
        assert_eq!(session.open_files.len(), MAX_OPEN_FILES);
        assert_eq!(
            session.open_files.last().map(String::as_str),
            Some(relatives[last].as_str()),
            "活动文件必须无条件保留"
        );
        assert_eq!(session.active_file, Some(MAX_OPEN_FILES - 1));
        let active_index = session.active_file.expect("有活动文件");
        assert_eq!(
            session.open_files[active_index], relatives[last],
            "下标必须指向活动文件自己"
        );

        let duplicated = vec![paths[0].clone(), paths[0].clone(), paths[1].clone()];
        let session = session_from_paths(&dir, &duplicated, Some(&paths[1]), ui_hidden());
        assert_eq!(
            session.open_files,
            vec!["file-000.txt".to_string(), "file-001.txt".to_string()]
        );
        assert_eq!(session.active_file, Some(1));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_saved_session_round_trips_through_disk() {
        let repo = TempRepo::new("round-trip");
        repo.write("src/main.java", "class Main {}\n");
        repo.write("README.md", "# readme\n");
        let paths = vec![
            repo.root().join("src").join("main.java"),
            repo.root().join("README.md"),
        ];
        let written = session_from_paths(repo.root(), &paths, Some(&paths[1]), ui_hidden());
        save_session(repo.root(), &written).expect("写会话失败");

        let loaded = load_session(repo.root());
        assert!(loaded.file_existed);
        assert!(!loaded.read_only);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(loaded.session, written);
        assert_eq!(
            loaded.session.open_files,
            vec!["src/main.java".to_string(), "README.md".to_string()],
            "路径用工作区相对 + `/` 分隔"
        );
        assert_eq!(loaded.session.active_file, Some(1));
        assert_eq!(loaded.session.left_sidebar_visible, Some(false));
        assert_eq!(loaded.session.top_activity_view, Some(1));
        assert_eq!(loaded.session.bottom_kind.as_deref(), Some("terminal"));
        assert_eq!(loaded.session.left_pane_width, Some(268.5));
        assert_eq!(loaded.session.right_pane_width, Some(413.25));
        assert_eq!(loaded.session.bottom_pane_height, Some(287.75));
        let text = repo.read(".lithe/session.local.json");
        for key in ["leftPaneWidth", "rightPaneWidth", "bottomPaneHeight"] {
            assert!(text.contains(key), "缺少键 {key}：{text}");
        }
        assert!(
            text.contains("\"version\": 1"),
            "版本号不该因为加可选键而变：{text}"
        );
    }

    #[test]
    fn a_legacy_document_without_size_keys_still_loads() {
        let repo = TempRepo::new("legacy-sizes");
        repo.write(
            ".lithe/session.local.json",
            r#"{"version":1,"openFiles":["alpha.txt"],"activeFile":0,
                "leftSidebarVisible":true,"topActivityView":2,"bottomKind":"git",
                "bottomVisible":true,"rightView":"maven","rightVisible":true}"#,
        );

        let loaded = load_session(repo.root());
        assert!(!loaded.read_only, "同版本号的旧文档不是只读");
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(loaded.session.left_pane_width, None, "缺键必须回落成 None");
        assert_eq!(loaded.session.right_pane_width, None);
        assert_eq!(loaded.session.bottom_pane_height, None);
        assert_eq!(
            loaded.session.top_activity_view,
            Some(2),
            "其余字段不受影响"
        );
        assert_eq!(loaded.session.right_view.as_deref(), Some("maven"));

        let path = session_path(repo.root());
        let previous = document::previous_object(&path);
        document::save_document(
            &path,
            &loaded.session,
            previous.as_ref(),
            SESSION_DOCUMENT_VERSION,
        )
        .expect("写回旧文档失败");
        let reloaded = load_session(repo.root());
        assert_eq!(reloaded.session, loaded.session);
        assert!(
            reloaded.diagnostics.is_empty(),
            "{:?}",
            reloaded.diagnostics
        );
        let text = repo.read(".lithe/session.local.json");
        assert!(text.contains("\"version\": 1"), "版本号不该变：{text}");
        assert!(text.contains("\"topActivityView\": 2"), "{text}");
    }

    #[test]
    fn the_three_pane_sizes_round_trip_through_the_document() {
        let repo = TempRepo::new("size-round-trip");
        let session = session_from_paths(repo.root(), &[], None, ui_hidden());
        let path = session_path(repo.root());
        document::save_document(&path, &session, None, SESSION_DOCUMENT_VERSION)
            .expect("写会话文档失败");

        let text = repo.read(".lithe/session.local.json");
        for key in ["leftPaneWidth", "rightPaneWidth", "bottomPaneHeight"] {
            assert!(text.contains(key), "缺少键 {key}：{text}");
        }

        let loaded = load_session(repo.root());
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(loaded.session.left_pane_width, Some(268.5));
        assert_eq!(loaded.session.right_pane_width, Some(413.25));
        assert_eq!(loaded.session.bottom_pane_height, Some(287.75));
        assert_eq!(loaded.session, session, "整份文档逐字段相等");
    }

    #[test]
    fn size_keys_are_carried_verbatim_without_clamping() {
        let mut ui = ui_hidden();
        ui.left_pane_width = Some(-5.0);
        ui.right_pane_width = Some(9_999.0);
        ui.bottom_pane_height = Some(f32::INFINITY);

        let session = session_from_paths(std::env::temp_dir().as_path(), &[], None, ui);
        assert_eq!(session.left_pane_width, Some(-5.0), "越界值原样搬运");
        assert_eq!(session.right_pane_width, Some(9_999.0));
        assert_eq!(session.bottom_pane_height, Some(f32::INFINITY));
    }

    #[test]
    fn a_newer_document_is_still_readable() {
        let dir = temp_dir("read-newer");
        let path = session_path(&dir);
        std::fs::create_dir_all(path.parent().expect("有父目录")).expect("建配置目录");
        std::fs::write(
            &path,
            r#"{"version":99,"openFiles":["kept.txt"],"activeFile":0}"#,
        )
        .expect("写文件");

        let loaded = load_session(&dir);
        assert!(loaded.read_only, "更高版本必须标成只读");
        assert_eq!(loaded.session.open_files, vec!["kept.txt".to_string()]);
        assert!(
            loaded
                .diagnostics
                .iter()
                .any(|line| line.contains("document_version_newer")),
            "{:?}",
            loaded.diagnostics
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plain_relative_paths_reject_escapes() {
        for accepted in ["a.txt", "src/main.java", "deep/nested/x/y.txt"] {
            assert!(is_plain_relative(accepted), "{accepted} 应当被接受");
        }
        for rejected in [
            "",
            "/abs.txt",
            "\\abs.txt",
            "../up.txt",
            "a/../../b.txt",
            "C:/x.txt",
        ] {
            assert!(!is_plain_relative(rejected), "{rejected} 应当被拒绝");
        }
    }

    #[test]
    fn resolving_a_nested_relative_path_uses_the_platform_separator() {
        let dir = temp_dir("separator");
        write_files(&dir, &["src/main/java/App.java"]);
        let session = SessionState {
            open_files: vec!["src/main/java/App.java".to_string()],
            ..SessionState::default()
        };
        let resolved = resolve_session_files(&dir, &session);
        assert_eq!(
            resolved,
            vec![SessionFileOutcome::Restored(
                dir.join("src").join("main").join("java").join("App.java")
            )]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
