//! 本地历史（Local History）：每次保存前把**盘上那份旧内容**存一份快照。
//!
//! # 生命周期与清理策略（改之前先读这一段）
//!
//! - **位置**：由宿主注入一个缓存根目录（`<根>/local-history`，见
//!   `ShellWorkspace` 传给 `EditorPane::set_local_history_root` 的那个路径）。绝不写应用安装
//!   目录或 bundle —— 那会让增量更新失去基线（见 AGENTS.md「已发布程序包的运行时只读边界」）。
//! - **布局**：每个源文件一个目录 `<根>/<净化后的路径>-<FNV-1a 64 十六进制>/`。净化的那半截
//!   只为可读，真正保证唯一的是 64 位哈希（同一路径稳定、不同路径几乎不会撞）。
//! - **快照**：每个快照一个文件 `<unix 毫秒>.snap`，内容是保存前盘上的**原始字节**，不做编码
//!   转换：历史的意义就是能原样还原，转一次编码就可能不再是当初那份。
//!   `<目录>/source.txt` 记源文件的绝对路径（只给列表显示用）。
//! - **保留**：每个源文件最多 [`MAX_SNAPSHOTS_PER_FILE`] 份，超出删最旧；比
//!   [`MAX_AGE`] 更旧的也删。两个上限都在**每次写快照时**判一次，没有后台清理任务。
//! - **失败**：写历史失败只留诊断，不阻断保存 —— 历史是附加品，不能挡住用户存盘。

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 每个源文件最多留多少份快照。
pub const MAX_SNAPSHOTS_PER_FILE: usize = 20;

/// 快照的最长保留时间：更旧的条目在下次写快照时清掉。
pub const MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

const SNAPSHOT_EXTENSION: &str = "snap";
const SOURCE_FILE_NAME: &str = "source.txt";

/// 一份历史快照。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// 快照文件本身。
    pub file: PathBuf,
    /// 存下来的那一刻（unix 毫秒，取自快照文件名）。
    pub taken_at_ms: u64,
    /// 快照字节数。
    pub bytes: u64,
}

impl Snapshot {
    /// 列表里显示的那一行：文件名主干 + 字节数。时间就在主干里（unix 毫秒），可排序。
    pub fn label(&self) -> String {
        let stem = self
            .file
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();
        format!("{stem} · {} B", self.bytes)
    }
}

/// 历史仓库的根（`<缓存根>/local-history`）。
pub fn history_root(cache_root: &Path) -> PathBuf {
    cache_root.join("local-history")
}

/// 源文件对应的历史目录。
pub fn file_directory(cache_root: &Path, source: &Path) -> PathBuf {
    let key = source.to_string_lossy().replace('\\', "/");
    let hash = fnv1a64(key.as_bytes());
    let readable: String = key
        .chars()
        .rev()
        .take(48)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    history_root(cache_root).join(format!("{hash:016x}-{readable}"))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// 存一份快照。返回写下的那份；失败时返回 `None`（调用方只记诊断）。
pub fn record(cache_root: &Path, source: &Path, bytes: &[u8]) -> Option<Snapshot> {
    let dir = file_directory(cache_root, source);
    if let Err(error) = std::fs::create_dir_all(&dir) {
        eprintln!(
            "S1_LOCAL_HISTORY record=failed dir={} error={error}",
            dir.display()
        );
        return None;
    }
    let source_path = dir.join(SOURCE_FILE_NAME);
    if !source_path.exists() {
        let _ = std::fs::write(&source_path, source.to_string_lossy().as_bytes());
    }

    let taken_at_ms = now_ms();
    let file = dir.join(format!("{taken_at_ms}.{SNAPSHOT_EXTENSION}"));
    if let Err(error) = std::fs::write(&file, bytes) {
        eprintln!(
            "S1_LOCAL_HISTORY record=failed file={} error={error}",
            file.display()
        );
        return None;
    }

    prune(&dir);
    Some(Snapshot {
        file,
        taken_at_ms,
        bytes: bytes.len() as u64,
    })
}

/// 这个源文件的历史，新的在前。
pub fn list(cache_root: &Path, source: &Path) -> Vec<Snapshot> {
    let dir = file_directory(cache_root, source);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut snapshots: Vec<Snapshot> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some(SNAPSHOT_EXTENSION) {
                return None;
            }
            let taken_at_ms = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.parse::<u64>().ok())?;
            let bytes = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
            Some(Snapshot {
                file: path,
                taken_at_ms,
                bytes,
            })
        })
        .collect();
    snapshots.sort_by(|left, right| right.taken_at_ms.cmp(&left.taken_at_ms));
    snapshots
}

/// 读一份快照的原始字节。
pub fn read(snapshot: &Snapshot) -> Option<Vec<u8>> {
    std::fs::read(&snapshot.file).ok()
}

/// 清掉过旧的与超额的两批，只留最新的 [`MAX_SNAPSHOTS_PER_FILE`] 份。
fn prune(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut stamped: Vec<(u64, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some(SNAPSHOT_EXTENSION) {
                return None;
            }
            let taken_at_ms = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.parse::<u64>().ok())?;
            Some((taken_at_ms, path))
        })
        .collect();
    stamped.sort_by(|left, right| right.0.cmp(&left.0));

    let now = now_ms();
    let oldest_allowed = now.saturating_sub(MAX_AGE.as_millis() as u64);
    for (index, (taken_at_ms, path)) in stamped.iter().enumerate() {
        let too_many = index >= MAX_SNAPSHOTS_PER_FILE;
        let too_old = *taken_at_ms < oldest_allowed;
        if too_many || too_old {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(case: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "lithe-local-history-{case}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("建临时目录");
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn source() -> PathBuf {
        PathBuf::from("C:/work/demo/src/Main.java")
    }

    #[test]
    fn snapshots_round_trip_the_original_bytes() {
        let dir = TempDir::new("roundtrip");
        let bytes = b"class Main {}\r\n".to_vec();

        let written = record(&dir.0, &source(), &bytes).expect("应写下快照");
        assert_eq!(written.bytes, bytes.len() as u64);

        let entries = list(&dir.0, &source());
        assert_eq!(entries.len(), 1, "一份快照");
        assert_eq!(entries[0].file, written.file);
        assert_eq!(
            read(&entries[0]).as_deref(),
            Some(bytes.as_slice()),
            "历史必须是原始字节，不做编码转换"
        );
    }

    #[test]
    fn different_sources_do_not_share_a_directory() {
        let dir = TempDir::new("distinct");
        record(&dir.0, Path::new("C:/a/x.java"), b"a").expect("写快照");
        record(&dir.0, Path::new("C:/a/y.java"), b"b").expect("写快照");

        assert_eq!(list(&dir.0, Path::new("C:/a/x.java")).len(), 1);
        assert_eq!(list(&dir.0, Path::new("C:/a/y.java")).len(), 1);
        assert_ne!(
            file_directory(&dir.0, Path::new("C:/a/x.java")),
            file_directory(&dir.0, Path::new("C:/a/y.java"))
        );
    }

    #[test]
    fn the_cap_keeps_only_the_newest_snapshots() {
        let dir = TempDir::new("cap");
        // 快照名就是时间戳。这里用「刚刚过去的一小段秒数」造名字：既在保留期内（否则会被
        // MAX_AGE 那一条按过期删掉），又严格递增，能分辨谁是最新的。
        let target = file_directory(&dir.0, &source());
        std::fs::create_dir_all(&target).expect("建历史目录");
        let newest = now_ms();
        let oldest = newest - (MAX_SNAPSHOTS_PER_FILE as u64 + 4) * 1_000;
        for index in 0..(MAX_SNAPSHOTS_PER_FILE + 5) {
            let stamp = oldest + index as u64 * 1_000;
            let file = target.join(format!("{stamp}.snap"));
            std::fs::write(&file, b"x").expect("写快照");
        }
        prune(&target);

        let remaining = list(&dir.0, &source());
        assert_eq!(remaining.len(), MAX_SNAPSHOTS_PER_FILE, "超额只留上限那么多");
        assert_eq!(
            remaining.first().map(|entry| entry.taken_at_ms),
            Some(newest),
            "留下的必须是最新的那批"
        );
    }

    #[test]
    fn a_source_without_history_has_no_entries() {
        let dir = TempDir::new("empty");
        assert!(list(&dir.0, Path::new("C:/never/saved.java")).is_empty());
    }
}
