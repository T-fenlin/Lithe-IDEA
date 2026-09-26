//! 设置文件的**外部改动**监听：手改了文件不需要重启就生效。
//!
//! ## 为什么监听目录而不是文件
//!
//! 我们自己的写入是"临时文件 + rename"。rename 会把原文件**换掉**，于是文件级监听在很多
//! 平台上会跟着被换掉的那一份一起失效 —— 自己写完第一次之后就再也听不到了。所以这里监听
//! **父目录**（非递归）再按文件名过滤。`notify` 与 `async-channel` 都已在 workspace 的依赖
//! 树里（分别由 `gpui-component` / `gpui-pre` 引入），这里只是升成直接依赖。
//!
//! ## 为什么不会自激
//!
//! 我们自己的写入同样会触发事件。重载（[`SettingsStore::reload_from_disk`]）在"文件内容与
//! 内存一致"时直接返回，所以 写 → 事件 → 重载 只走一轮就停。
//!
//! ## 外部改动胜出
//!
//! 如果用户刚改了设置（还在 300ms 防抖窗口里），而文件同时被外部改了，**以文件为准**：
//! 内存换成文件内容，并放弃那次还没落盘的界面改动。理由：外部改动是人手改文件或另一个
//! 进程写的，那是一次明确的意图；两边都保留会让状态说不清。放弃待写不会丢数据 ——
//! 文件里的内容就是刚生效的那一份。
//!
//! ## 文件被删掉时不动内存
//!
//! 删掉 `settings.json` 不等于"用户想恢复默认"：更常见的是清理、迁移或临时移走。这时
//! 保留内存里的设置并留一条诊断，而不是把界面打回默认值。

use std::path::{Path, PathBuf};

use gpui_kit::Context;

use crate::store::SettingsStore;

/// 一次文件系统事件是否动到了我们关心的那个文件。
///
/// 只比**文件名**：监听的目录就是设置文件所在的目录（非递归），同一目录里不会有两个同名
/// 文件；这样写还能免疫 Windows 上 `\\?\` 前缀与父目录大小写差异 —— 那些差异出现在父目录
/// 那一段，而不是文件名。
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

/// 文件名比较：Windows 忽略大小写（`Settings.json` 与 `settings.json` 是同一个文件），
/// 其余平台区分大小写。
fn names_match(candidate: &str, watched: &str) -> bool {
    if cfg!(windows) {
        candidate.eq_ignore_ascii_case(watched)
    } else {
        candidate == watched
    }
}

/// 给 `SettingsStore` 装一个外部改动监听。
///
/// 监听器被移进 spawned 任务里，所以它的生命周期和任务一样长 —— 任务不结束就不会释放。
pub fn watch_settings_file(cx: &mut Context<SettingsStore>, path: PathBuf) {
    let Some(parent) = path.parent().map(Path::to_path_buf) else {
        eprintln!("S1_SETTINGS watch_skipped reason=no_parent");
        return;
    };

    // 目录不存在时监听一定失败。这里建目录是有意的例外：**读设置文件**仍然不创建文件
    // （首次启动照旧全默认值），但配置目录是应用自己的地盘，先建出来才能保证
    // "第一次外部改动就被听到"。
    if let Err(error) = std::fs::create_dir_all(&parent) {
        eprintln!(
            "S1_SETTINGS watch_failed dir={} error={error}",
            parent.display()
        );
        return;
    }

    let (sender, receiver) = async_channel::bounded::<notify::Event>(16);
    let mut watcher = match notify::recommended_watcher(
        move |result: notify::Result<notify::Event>| {
            let Ok(event) = result else {
                return;
            };
            // 只看"内容可能变了"的三类事件；`Access` 之类不处理。
            if !matches!(
                event.kind,
                notify::EventKind::Create(_)
                    | notify::EventKind::Modify(_)
                    | notify::EventKind::Remove(_)
            ) {
                return;
            }
            // 通道满说明已经有一批待处理事件。丢掉新事件是安全的：重载是幂等的，
            // 而"丢了最后一个事件"最坏也只是少一次重载，下一次改动还会再触发。
            let _ = sender.try_send(event);
        },
    ) {
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
            // 实体可能已经销毁：`update` 返回 `Err` 时静默忽略，不 panic。
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

    /// 事件按**文件名**匹配：自己的原子写产生的 `.tmp` 事件不算，兄弟文件不算。
    #[test]
    fn events_are_matched_by_file_name() {
        let watched = watched_path();

        assert!(event_touches(&[watched.clone()], &watched));

        // 临时文件（"临时文件 + rename"里的那一半）不算：真正要重载的是目标文件那一条。
        assert!(!event_touches(
            &[PathBuf::from(
                "C:\\Users\\u\\AppData\\Roaming\\Lithe\\settings.json.tmp"
            )],
            &watched
        ));

        // 同目录的兄弟文件不算。
        assert!(!event_touches(
            &[PathBuf::from(
                "C:\\Users\\u\\AppData\\Roaming\\Lithe\\recent-projects.json"
            )],
            &watched
        ));

        // 没有事件、或事件里没有文件名那一段，都不算。
        assert!(!event_touches(&[], &watched));
        assert!(!event_touches(&[PathBuf::from("C:\\")], &watched));

        // 一批事件里只要有一条命中就算命中（真实的一次写入常常产生多条）。
        assert!(event_touches(
            &[
                PathBuf::from("C:\\Users\\u\\AppData\\Roaming\\Lithe\\settings.json.tmp"),
                watched.clone(),
            ],
            &watched
        ));

        // 父目录不同但文件名相同：仍然算命中 —— 这正是"只比文件名"的取舍，
        // 因为我们监听的目录只有一个（见模块文档）。
        assert!(event_touches(
            &[PathBuf::from("D:\\tmp\\settings.json")],
            &watched
        ));
    }

    /// 大小写敏感度跟平台一致（Windows 忽略大小写，其余平台区分）。
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
