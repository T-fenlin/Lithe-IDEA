use std::path::{Path, PathBuf};

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, IntoElement, ParentElement as _, Styled as _, div,
};
use lithe_gpui_shared::{CoreError, core_json, tr};
use serde_json::{Value, json};

use crate::workspace::ShellWorkspace;

const TAG: &str = "S1_SEARCH_INDEX";

const WARM_COMMAND: &str = "workspace.searchIndex.warm";
const UPDATE_COMMAND: &str = "workspace.searchIndex.update";
const INVALIDATE_COMMAND: &str = "workspace.searchIndex.invalidate";

const WARM_OP: &str = "warm";

const PENDING_NOTICE_KEY: &str = "lithe.search.gpui.indexWarming";

const FAILED_NOTICE_KEY: &str = "lithe.search.gpui.indexWarmFailed";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SearchIndexReadiness {
    #[default]
    Pending,
    Ready,
    Failed,
}

impl SearchIndexReadiness {
    pub(crate) fn notice_key(self) -> Option<&'static str> {
        match self {
            Self::Pending => Some(PENDING_NOTICE_KEY),
            Self::Ready => None,
            Self::Failed => Some(FAILED_NOTICE_KEY),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }
}

pub(crate) fn readiness_notice(readiness: SearchIndexReadiness, cx: &App) -> Option<AnyElement> {
    let key = readiness.notice_key()?;
    Some(
        div()
            .w_full()
            .flex_shrink_0()
            .px_3()
            .py_2()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(tr(key))
            .into_any_element(),
    )
}

fn root_payload(root: &str) -> Value {
    json!({ "root": root })
}

fn update_payload(root: &str, paths: &[String]) -> Value {
    json!({ "root": root, "paths": paths })
}

fn index_path(root: &Path, written: &Path) -> Option<String> {
    lithe_gpui_shared::workspace_config::workspace_relative(root, written)
}

pub(crate) struct SearchIndexWiring {
    root: PathBuf,
    generation: u64,
    readiness: SearchIndexReadiness,
}

impl SearchIndexWiring {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            generation: 0,
            readiness: SearchIndexReadiness::default(),
        }
    }

    pub(crate) fn readiness(&self) -> SearchIndexReadiness {
        self.readiness
    }

    fn generation(&self) -> u64 {
        self.generation
    }

    fn next_generation(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }

    fn accepts(&self, issued: u64) -> bool {
        self.generation == issued
    }

    pub(crate) fn schedule_warm(&self, cx: &mut Context<ShellWorkspace>) {
        let root_text = self.root.to_string_lossy().to_string();
        let issued = self.generation();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { core_json(WARM_COMMAND, root_payload(&root_text)) })
                .await;
            let _ = this.update(cx, |shell, cx| {
                if shell.search_index.adopt(WARM_OP, issued, result) {
                    shell.broadcast_search_index_readiness(cx);
                }
            });
        })
        .detach();
    }

    pub(crate) fn schedule_update(&self, written: &Path, cx: &mut Context<ShellWorkspace>) {
        let Some(relative) = index_path(&self.root, written) else {
            eprintln!(
                "{TAG} update skipped=outside_root root={} path={}",
                self.root.display(),
                written.display()
            );
            return;
        };
        let root_text = self.root.to_string_lossy().to_string();
        let issued = self.generation();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    core_json(UPDATE_COMMAND, update_payload(&root_text, &[relative]))
                })
                .await;
            let _ = this.update(cx, |shell, _cx| {
                shell.search_index.adopt("update", issued, result);
            });
        })
        .detach();
    }

    pub(crate) fn schedule_invalidate(&mut self, cx: &mut Context<ShellWorkspace>) {
        let issued = self.next_generation();
        let root_text = self.root.to_string_lossy().to_string();
        let payload = root_payload(&root_text);
        eprintln!(
            "{TAG} invalidate root={} generation={issued}",
            self.root.display()
        );
        cx.spawn(async move |_this, cx| {
            let result = cx
                .background_spawn(async move { core_json(INVALIDATE_COMMAND, payload) })
                .await;
            match result {
                Ok(_) => eprintln!("{TAG} invalidate root={root_text} status=ok"),
                Err(error) => {
                    eprintln!("{TAG} invalidate root={root_text} status=failed error={error}")
                }
            }
        })
        .detach();
    }

    fn adopt(&mut self, op: &str, issued: u64, result: Result<Option<Value>, CoreError>) -> bool {
        if !self.accepts(issued) {
            eprintln!("{TAG} {op} stale=1 root={}", self.root.display());
            return false;
        }
        eprintln!("{}", result_line(op, &self.root, &result));
        if op != WARM_OP {
            return false;
        }
        let next = if result.is_ok() {
            SearchIndexReadiness::Ready
        } else {
            SearchIndexReadiness::Failed
        };
        let changed = self.readiness != next;
        self.readiness = next;
        if changed {
            eprintln!(
                "{TAG} readiness={} root={}",
                next.as_str(),
                self.root.display()
            );
        }
        changed
    }
}

fn result_line(op: &str, root: &Path, result: &Result<Option<Value>, CoreError>) -> String {
    let root = root.display();
    match result {
        Ok(Some(data)) => format!(
            "{TAG} {op} root={root} files={} symbols={} postings={} rebuilt={}",
            count(data, "fileCount"),
            count(data, "symbolCount"),
            count(data, "postingCount"),
            data.get("rebuilt")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        Ok(None) => format!("{TAG} {op} root={root} status=missing_data"),
        Err(error) => format!("{TAG} {op} root={root} status=failed error={error}"),
    }
}

fn count(data: &Value, key: &str) -> String {
    data.get(key)
        .and_then(Value::as_u64)
        .map_or_else(|| "?".to_string(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_follow_the_core_contract_field_names() {
        assert_eq!(root_payload("D:/proj"), json!({ "root": "D:/proj" }));
        assert_eq!(
            update_payload("D:/proj", &["src/a.java".to_string()]),
            json!({ "root": "D:/proj", "paths": ["src/a.java"] })
        );
        assert!(
            root_payload("r").get("hiddenDirectoryNames").is_none(),
            "宿主不传可见性数组（见模块文档「请求形状」）"
        );
        assert!(update_payload("r", &[]).get("hiddenFilePatterns").is_none());
    }

    #[test]
    fn update_paths_are_workspace_relative_and_scoped_to_the_root() {
        let root = Path::new("proj-root");
        let inside = root.join("src").join("a.java");
        assert_eq!(index_path(root, &inside).as_deref(), Some("src/a.java"));

        let outside = Path::new("elsewhere").join("a.java");
        assert_eq!(index_path(root, &outside), None);
    }

    #[test]
    fn a_superseded_generation_is_not_adopted() {
        let mut wiring = SearchIndexWiring::new(PathBuf::from("proj-root"));
        let first = wiring.generation();
        assert!(wiring.accepts(first));

        let second = wiring.next_generation();
        assert_ne!(first, second);
        assert!(
            !wiring.accepts(first),
            "换根前发出的结果不能当成当前根的状态"
        );
        assert!(wiring.accepts(second));
    }

    #[test]
    fn the_diagnostic_line_names_the_operation_and_the_counts() {
        let root = Path::new("proj-root");
        let ok = result_line(
            "warm",
            root,
            &Ok(Some(json!({
                "fileCount": 12,
                "symbolCount": 3,
                "postingCount": 44,
                "rebuilt": false,
            }))),
        );
        assert_eq!(
            ok,
            format!(
                "S1_SEARCH_INDEX warm root={} files=12 symbols=3 postings=44 rebuilt=false",
                root.display()
            )
        );

        let partial = result_line("update", root, &Ok(Some(json!({ "fileCount": 1 }))));
        assert!(partial.contains("files=1 symbols=? postings=?"));

        assert!(result_line("update", root, &Ok(None)).ends_with("status=missing_data"));
        let failed = result_line(
            "invalidate",
            root,
            &Err(CoreError::MissingData {
                command: INVALIDATE_COMMAND.to_string(),
            }),
        );
        assert!(failed.starts_with("S1_SEARCH_INDEX invalidate root="));
        assert!(failed.contains("status=failed"));
    }

    #[test]
    fn warm_success_moves_pending_to_ready_exactly_once() {
        let mut wiring = SearchIndexWiring::new(PathBuf::from("proj-root"));
        assert_eq!(
            wiring.readiness(),
            SearchIndexReadiness::Pending,
            "项目刚打开、warm 结果还没回来：必须是 Pending"
        );

        let issued = wiring.generation();
        let warm = || Ok(Some(json!({ "fileCount": 1, "rebuilt": true })));
        assert!(
            wiring.adopt(WARM_OP, issued, warm()),
            "Pending → Ready 是**变**，调用方据此推一次新值"
        );
        assert_eq!(wiring.readiness(), SearchIndexReadiness::Ready);

        assert!(
            !wiring.adopt(WARM_OP, issued, warm()),
            "已经是 Ready 再采纳一次不算变（不重复重绘）"
        );
        assert_eq!(wiring.readiness(), SearchIndexReadiness::Ready);
    }

    #[test]
    fn warm_failure_moves_pending_to_failed() {
        let mut wiring = SearchIndexWiring::new(PathBuf::from("proj-root"));
        let issued = wiring.generation();
        let failed = Err(CoreError::MissingData {
            command: WARM_COMMAND.to_string(),
        });
        assert!(wiring.adopt(WARM_OP, issued, failed));
        assert_eq!(wiring.readiness(), SearchIndexReadiness::Failed);
    }

    #[test]
    fn missing_data_is_still_reported_as_ready() {
        let mut wiring = SearchIndexWiring::new(PathBuf::from("proj-root"));
        let issued = wiring.generation();
        assert!(wiring.adopt(WARM_OP, issued, Ok(None)));
        assert_eq!(wiring.readiness(), SearchIndexReadiness::Ready);
    }

    #[test]
    fn update_never_changes_readiness() {
        let mut wiring = SearchIndexWiring::new(PathBuf::from("proj-root"));
        let issued = wiring.generation();
        assert!(!wiring.adopt(
            "update",
            issued,
            Ok(Some(json!({ "fileCount": 2, "rebuilt": false })))
        ));
        assert_eq!(
            wiring.readiness(),
            SearchIndexReadiness::Pending,
            "update 不该把 Pending 变成 Ready"
        );
    }

    #[test]
    fn a_superseded_generation_does_not_change_readiness() {
        let mut wiring = SearchIndexWiring::new(PathBuf::from("proj-root"));
        let superseded = wiring.generation();
        let _current = wiring.next_generation();

        assert!(
            !wiring.adopt(
                WARM_OP,
                superseded,
                Ok(Some(json!({ "fileCount": 9, "rebuilt": true })))
            ),
            "旧代次的 warm 结果不算变"
        );
        assert_eq!(
            wiring.readiness(),
            SearchIndexReadiness::Pending,
            "旧根的结果不能把当前根标成就绪"
        );

        let fresh = SearchIndexWiring::new(PathBuf::from("other-root"));
        assert_eq!(fresh.readiness(), SearchIndexReadiness::Pending);
    }

    #[test]
    fn the_notice_is_hidden_when_ready_and_differs_when_failed() {
        assert_eq!(
            SearchIndexReadiness::Pending.notice_key(),
            Some(PENDING_NOTICE_KEY)
        );
        assert_eq!(
            SearchIndexReadiness::Ready.notice_key(),
            None,
            "就绪后那一行要整行消失"
        );
        assert_eq!(
            SearchIndexReadiness::Failed.notice_key(),
            Some(FAILED_NOTICE_KEY)
        );
        assert_ne!(
            PENDING_NOTICE_KEY, FAILED_NOTICE_KEY,
            "「还在构建」与「没建成功」必须是两句不同的话"
        );
        for key in [PENDING_NOTICE_KEY, FAILED_NOTICE_KEY] {
            assert!(key.starts_with("lithe.search.gpui."), "{key}");
        }
    }

    #[test]
    fn the_warm_operation_name_matches_the_wired_command() {
        assert_eq!(WARM_OP, "warm");
        assert!(WARM_COMMAND.ends_with(".warm"), "{WARM_COMMAND}");
    }

    #[test]
    fn a_real_core_round_trip_warms_then_updates_incrementally() {
        let root = std::env::temp_dir().join(format!("lithe-search-index-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("建目录失败");
        std::fs::write(root.join("src").join("A.java"), "class A {}\n").expect("写文件失败");

        let root_text = root.to_string_lossy().to_string();

        let warm = core_json(WARM_COMMAND, root_payload(&root_text))
            .expect("warm 必须成功")
            .expect("warm 一定回 data");
        assert_eq!(
            warm.get("fileCount").and_then(Value::as_u64),
            Some(1),
            "{warm}"
        );
        assert_eq!(
            warm.get("rebuilt").and_then(Value::as_bool),
            Some(false),
            "{warm}"
        );

        std::fs::write(root.join("src").join("B.java"), "class B {}\n").expect("写文件失败");
        let relative = index_path(&root, &root.join("src").join("B.java")).expect("在工作区里");
        let update = core_json(UPDATE_COMMAND, update_payload(&root_text, &[relative]))
            .expect("update 必须成功")
            .expect("update 一定回 data");
        assert_eq!(
            update.get("rebuilt").and_then(Value::as_bool),
            Some(false),
            "已有索引 + 单文件增量 = 不该退化成重建：{update}"
        );
        assert_eq!(
            update.get("fileCount").and_then(Value::as_u64),
            Some(2),
            "{update}"
        );

        core_json(INVALIDATE_COMMAND, root_payload(&root_text)).expect("invalidate 必须成功");
        let _ = std::fs::remove_dir_all(&root);
    }
}
