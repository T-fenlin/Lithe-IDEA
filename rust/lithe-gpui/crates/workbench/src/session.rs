use std::path::{Path, PathBuf};

use lithe_gpui_editor::EditorPane;
use lithe_gpui_settings::DebounceState;
use lithe_gpui_shared::workspace_config::DIAGNOSTIC_PREFIX;
use lithe_gpui_shared::workspace_config::session::{
    self, SessionFileOutcome, SessionState, SessionUiState,
};

use gpui_kit::{App, Entity, Window};

pub const SESSION_SAVE_DEBOUNCE_MS: u64 = 300;

#[derive(Debug, Clone, PartialEq)]
pub enum SessionPlan {
    Unchanged,
    Debounced(u64),
    ReadOnly,
}

type PlanKey = (Vec<PathBuf>, Option<PathBuf>, SessionUiState);

fn plan_key_changed(previous: Option<&PlanKey>, current: &PlanKey) -> bool {
    previous != Some(current)
}

pub struct SessionTracker {
    root: PathBuf,
    current: SessionState,
    last_saved: Option<SessionState>,
    debounce: DebounceState,
    read_only: bool,
    last_seen: Option<PlanKey>,
    pub(crate) load_diagnostics: Vec<String>,
    diagnostics_reported: bool,
}

impl SessionTracker {
    pub fn new(root: &Path) -> Self {
        let loaded = session::load_session(root);
        Self {
            root: root.to_path_buf(),
            current: SessionState::default(),
            last_saved: loaded.file_existed.then_some(loaded.session.clone()),
            debounce: DebounceState::default(),
            read_only: loaded.read_only,
            last_seen: None,
            load_diagnostics: loaded.diagnostics,
            diagnostics_reported: false,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn read_only(&self) -> bool {
        self.read_only
    }

    pub fn current(&self) -> &SessionState {
        &self.current
    }

    pub fn plan(
        &mut self,
        editor: &Entity<EditorPane>,
        ui: SessionUiState,
        cx: &App,
    ) -> SessionPlan {
        if !self.diagnostics_reported {
            self.diagnostics_reported = true;
            for diagnostic in &self.load_diagnostics {
                eprintln!("{DIAGNOSTIC_PREFIX} session_diagnostic {diagnostic}");
            }
        }
        let (open, active) = {
            let pane = editor.read(cx);
            (pane.open_disk_files(), pane.active_disk_file())
        };
        let key: PlanKey = (open.clone(), active.clone(), ui.clone());
        if !plan_key_changed(self.last_seen.as_ref(), &key) {
            return SessionPlan::Unchanged;
        }
        self.last_seen = Some(key);
        self.current = session::session_from_paths(&self.root, &open, active.as_deref(), ui);
        if self.last_saved.as_ref() == Some(&self.current) {
            return SessionPlan::Unchanged;
        }
        if self.read_only {
            return SessionPlan::ReadOnly;
        }
        SessionPlan::Debounced(self.debounce.arm())
    }

    pub fn flush_if_current(&mut self, armed: u64) -> bool {
        if !self.debounce.should_flush(armed) {
            return false;
        }
        self.write()
    }

    pub fn flush(&mut self, force: bool) -> bool {
        self.debounce.mark_flushed();
        if !force && self.last_saved.as_ref() == Some(&self.current) {
            return false;
        }
        self.write()
    }

    fn write(&mut self) -> bool {
        if self.read_only {
            eprintln!(
                "{DIAGNOSTIC_PREFIX} session_save_skipped root={} reason=document_version_newer",
                self.root.display()
            );
            return false;
        }
        match session::save_session(&self.root, &self.current) {
            Ok(saved) => {
                self.last_saved = Some(self.current.clone());
                println!(
                    "{DIAGNOSTIC_PREFIX} session_saved root={} files={} active={:?} bytes={} exclusion={:?}",
                    self.root.display(),
                    self.current.open_files.len(),
                    self.current.active_file,
                    saved.bytes,
                    saved.exclusion
                );
                true
            }
            Err(error) => {
                eprintln!(
                    "{DIAGNOSTIC_PREFIX} session_save_failed root={} error={error}",
                    self.root.display()
                );
                false
            }
        }
    }
}

pub fn restore_into(
    root: &Path,
    editor: &Entity<EditorPane>,
    window: &mut Window,
    cx: &mut App,
) -> SessionOutcome {
    let loaded = session::load_session(root);
    for diagnostic in &loaded.diagnostics {
        eprintln!("{DIAGNOSTIC_PREFIX} session_diagnostic {diagnostic}");
    }
    let outcomes = session::resolve_session_files(root, &loaded.session);
    let paths: Vec<PathBuf> = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            SessionFileOutcome::Restored(path) => Some(path.clone()),
            SessionFileOutcome::Skipped { .. } => None,
        })
        .collect();
    let skipped = outcomes.len() - paths.len();

    let restore = editor.update(cx, |pane, cx| {
        pane.restore_session(&paths, loaded.session.active_file, window, cx)
    });

    for outcome in &outcomes {
        if let SessionFileOutcome::Skipped { relative, reason } = outcome {
            println!(
                "{DIAGNOSTIC_PREFIX} session_skipped root={} path={relative} reason={reason}",
                root.display()
            );
        }
    }
    println!(
        "{DIAGNOSTIC_PREFIX} session_restored root={} opened={} skipped={} active={:?} active_file={:?}",
        root.display(),
        restore.opened.len(),
        skipped,
        restore.active_index,
        restore
            .active_index
            .and_then(|index| restore.opened.get(index))
            .map(|path| path.display().to_string())
    );

    SessionOutcome {
        session: loaded.session,
        outcomes,
        opened: restore.opened.len(),
        skipped,
        active_index: restore.active_index,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionOutcome {
    pub session: SessionState,
    pub outcomes: Vec<SessionFileOutcome>,
    pub opened: usize,
    pub skipped: usize,
    pub active_index: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lithe_gpui_shared::workspace_config::session::MAX_OPEN_FILES;

    #[test]
    fn a_ui_only_change_is_not_swallowed_by_the_plan_fast_path() {
        let key = |width: Option<f32>, visible: bool| -> PlanKey {
            (
                vec![PathBuf::from("a.txt")],
                None,
                SessionUiState {
                    left_sidebar_visible: Some(visible),
                    left_pane_width: width,
                    ..SessionUiState::default()
                },
            )
        };

        assert!(
            plan_key_changed(None, &key(Some(320.), true)),
            "第一次看必然算变了"
        );
        assert!(
            !plan_key_changed(Some(&key(Some(320.), true)), &key(Some(320.), true)),
            "三样完全一样才算没变"
        );
        assert!(
            plan_key_changed(Some(&key(Some(320.), true)), &key(Some(360.), true)),
            "只有面板宽度变了也必须算变（否则拖完的宽度永远写不出去）"
        );
        assert!(
            plan_key_changed(Some(&key(Some(320.), true)), &key(Some(320.), false)),
            "只有可见性变了也必须算变"
        );
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lithe-workbench-session-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        dir
    }

    #[test]
    fn an_unchanged_document_is_not_written() {
        let dir = temp_dir("unchanged");
        let mut tracker = SessionTracker::new(&dir);
        tracker.current = SessionState {
            open_files: vec!["a.txt".to_string()],
            ..SessionState::default()
        };
        tracker.last_saved = Some(tracker.current.clone());

        assert!(!tracker.flush(false), "内容没变时 flush 不该写");
        assert!(!tracker.debounce.has_pending());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_newer_document_is_never_written() {
        let dir = temp_dir("read-only");
        let path = session::session_path(&dir);
        std::fs::create_dir_all(path.parent().expect("有父目录")).expect("建配置目录");
        std::fs::write(&path, r#"{"version":99,"openFiles":["kept.txt"]}"#).expect("写文件");

        let mut tracker = SessionTracker::new(&dir);
        assert!(tracker.read_only());
        assert!(
            tracker
                .load_diagnostics
                .iter()
                .any(|line| line.contains("document_version_newer")),
            "{:?}",
            tracker.load_diagnostics
        );
        assert!(!tracker.flush(true), "只读文档必须拒绝写入");
        assert_eq!(
            std::fs::read_to_string(&path).expect("读文件"),
            r#"{"version":99,"openFiles":["kept.txt"]}"#,
            "被拒绝的写入不得改动文件"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_the_last_debounce_generation_writes() {
        let dir = temp_dir("debounce");
        let mut tracker = SessionTracker::new(&dir);
        tracker.current = SessionState {
            open_files: vec!["a.txt".to_string()],
            ..SessionState::default()
        };

        let first = tracker.debounce.arm();
        let second = tracker.debounce.arm();
        assert!(!tracker.debounce.should_flush(first));
        assert!(tracker.debounce.should_flush(second));
        assert!(!tracker.debounce.should_flush(second));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_bound_comes_from_the_shared_document_layer() {
        assert_eq!(MAX_OPEN_FILES, 40);
    }
}
