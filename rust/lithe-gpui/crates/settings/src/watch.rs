use std::path::{Path, PathBuf};

use gpui::Context;

use crate::store::SettingsStore;

pub fn event_touches(event_paths: &[PathBuf], watched: &Path) -> bool {
    let Some(watched_name) = watched.file_name() else {
        return false;
    };
    let watched_name = watched_name.to_string_lossy();
    event_paths.iter().any(|path| {
        path.file_name()
            .is_some_and(|name| names_match(&name.to_string_lossy(), &watched_name))
    })
}

fn names_match(candidate: &str, watched: &str) -> bool {
    if cfg!(windows) {
        candidate.eq_ignore_ascii_case(watched)
    } else {
        candidate == watched
    }
}

pub fn watch_settings_file(cx: &mut Context<SettingsStore>, path: PathBuf) {
    let Some(parent) = path.parent().map(Path::to_path_buf) else {
        eprintln!("S1_SETTINGS watch_skipped reason=no_parent");
        return;
    };

    if let Err(error) = std::fs::create_dir_all(&parent) {
        eprintln!(
            "S1_SETTINGS watch_failed dir={} error={error}",
            parent.display()
        );
        return;
    }

    let (sender, receiver) = async_channel::bounded::<notify::Event>(16);
    let mut watcher =
        match notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            let Ok(event) = result else {
                return;
            };
            if !matches!(
                event.kind,
                notify::EventKind::Create(_)
                    | notify::EventKind::Modify(_)
                    | notify::EventKind::Remove(_)
            ) {
                return;
            }
            let _ = sender.try_send(event);
        }) {
            Ok(watcher) => watcher,
            Err(error) => {
                eprintln!("S1_SETTINGS watch_failed error={error}");
                return;
            }
        };

    let watched = path.clone();
    let watched_for_log = path.clone();
    cx.spawn(async move |this, cx| {
        use notify::Watcher as _;

        if let Err(error) = watcher.watch(&parent, notify::RecursiveMode::NonRecursive) {
            eprintln!(
                "S1_SETTINGS watch_failed dir={} error={error}",
                parent.display()
            );
            return;
        }
        println!("S1_SETTINGS watching path={}", watched_for_log.display());

        while let Ok(event) = receiver.recv().await {
            if !event_touches(&event.paths, &watched) {
                continue;
            }
            let _ = this.update(cx, |store, cx| store.reload_from_disk(cx));
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watched_path() -> PathBuf {
        PathBuf::from("C:\\Users\\u\\AppData\\Roaming\\Lithe\\settings.json")
    }

    #[test]
    fn events_are_matched_by_file_name() {
        let watched = watched_path();

        assert!(event_touches(&[watched.clone()], &watched));

        assert!(!event_touches(
            &[PathBuf::from(
                "C:\\Users\\u\\AppData\\Roaming\\Lithe\\settings.json.tmp"
            )],
            &watched
        ));

        assert!(!event_touches(
            &[PathBuf::from(
                "C:\\Users\\u\\AppData\\Roaming\\Lithe\\recent-projects.json"
            )],
            &watched
        ));

        assert!(!event_touches(&[], &watched));
        assert!(!event_touches(&[PathBuf::from("C:\\")], &watched));

        assert!(event_touches(
            &[
                PathBuf::from("C:\\Users\\u\\AppData\\Roaming\\Lithe\\settings.json.tmp"),
                watched.clone(),
            ],
            &watched
        ));

        assert!(event_touches(
            &[PathBuf::from("D:\\tmp\\settings.json")],
            &watched
        ));
    }

    #[test]
    fn name_matching_follows_the_platform() {
        assert!(names_match("settings.json", "settings.json"));
        assert_eq!(
            names_match("SETTINGS.JSON", "settings.json"),
            cfg!(windows),
            "Windows 上同名文件不分大小写"
        );
    }
}
