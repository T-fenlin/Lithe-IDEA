//! 文件域逻辑：把 Rust Core 的工作区快照变成侧栏的项目树。
//!
//! - [`load_files`] 直接调用 Rust Core 的 `workspace.snapshot` 命令，返回
//!   （要显示的行，文件总数，按顶层目录的统计）；
//! - [`build_tree`] 把扁平的相对路径列表聚成**完整的多层目录树**。
//!
//! 树做成任意深度是照 macOS 的做法：真机的项目树是递归渲染的
//! （`macos/Sources/Lithe/Views/Workspace/ProjectSidebarView.swift` 的 `FileNodeRow(depth:)`
//! 递归展开 `children`），目录能一直点开到最里层，而不是只有两层。
//!
//! 节点 id 里带的是**相对工作区根的路径**：`dir:<path>` / `file:<path>`。
//! `panels.rs` 的点击处理靠这个前缀区分"目录行（交给 Tree 展开/折叠）"和
//! "文件行（打开到编辑区）"，改格式要同时改那边。

use std::path::PathBuf;

use gpui_kit::base::TreeItem;
use gpui_kit::SharedString;

use super::stats::StatRow;

/// 项目树最多渲染多少个文件节点。
///
/// 本仓库快照约 4.8 千个文件，这个上限等于"全部显示"；设它是为了防止把
/// 巨型工作区（几十万文件）一次性建成树。超过时只显示前 N 个（Core 返回的顺序
/// 是排序过的，所以截断是可预期的），文件总数与右侧统计仍然按**全量**算。
pub(super) const FILE_ROWS: usize = 5_000;

/// 临时树：先把路径按 `/` 拆成层级，再转成 `TreeItem`。
///
/// 用 `BTreeMap` 只是为了取"目录名去重"，真正的排序在 [`to_items`] 里按
/// macOS 的规则做（目录优先 + 名字小写升序）。
#[derive(Default)]
struct PathTree {
    directories: std::collections::BTreeMap<String, PathTree>,
    files: Vec<String>,
}

impl PathTree {
    /// 把一条相对路径插进树里。
    ///
    /// `path` 用 `/` 分隔（Core 返回的就是这种形式）；末尾的空段（路径以 `/` 结尾）
    /// 会被忽略，避免建出空名字的节点。
    fn insert(&mut self, path: &str) {
        let mut segments = path.split('/').filter(|segment| !segment.is_empty()).peekable();
        let mut node = self;

        while let Some(segment) = segments.next() {
            if segments.peek().is_some() {
                node = node.directories.entry(segment.to_string()).or_default();
            } else {
                node.files.push(segment.to_string());
            }
        }
    }
}

/// 把扁平路径列表聚成多层树。
pub(super) fn build_tree(files: &[String]) -> Vec<TreeItem> {
    let mut tree = PathTree::default();
    for path in files {
        tree.insert(path);
    }

    to_items("", &tree)
}

/// 把 [`PathTree`] 的一层转成 `TreeItem` 列表。
///
/// 排序照 macOS（`ProjectSidebarView` 的目录优先 + 名字升序）：目录在前、文件在后，
/// 两边都按**小写**比较，免得大写开头的名字被排到最前面。
fn to_items(prefix: &str, node: &PathTree) -> Vec<TreeItem> {
    let join = |name: &str| {
        if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}/{name}")
        }
    };

    let mut directories: Vec<&String> = node.directories.keys().collect();
    directories.sort_by_key(|name| name.to_lowercase());

    let mut items: Vec<TreeItem> = directories
        .into_iter()
        .map(|name| {
            let path = join(name);
            TreeItem::new(
                SharedString::from(format!("dir:{path}")),
                // 真机的目录行只有名字（类型靠左边的文件夹图标表示），不带尾斜杠。
                SharedString::from(name.clone()),
            )
            // 默认折叠：真机首屏也是折叠的，点箭头才展开下一层。
            .expanded(false)
            .children(to_items(&path, &node.directories[name]))
        })
        .collect();

    let mut files = node.files.clone();
    files.sort_by_key(|name| name.to_lowercase());
    items.extend(files.into_iter().map(|name| {
        let path = join(&name);
        TreeItem::new(
            SharedString::from(format!("file:{path}")),
            SharedString::from(name),
        )
    }));

    items
}

/// 用真实 Core 调用取工作区快照，返回（要显示的行，文件总数，按顶层目录的统计）。
pub(super) fn load_files(
    root: &PathBuf,
    max_rows: usize,
) -> Result<(Vec<String>, usize, Vec<StatRow>), String> {
    let request = serde_json::json!({
        "id": "shell-snapshot",
        "operationId": "shell-snapshot",
        "timeoutMilliseconds": 120_000,
        "command": "workspace.snapshot",
        "payload": { "root": root.to_str().ok_or("工作区根不是合法 UTF-8 路径")? },
    })
    .to_string();

    let raw = lithe_core::execute_json(&request);
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|error| format!("响应不是合法 JSON：{error}"))?;

    if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        let code = value
            .pointer("/error/code")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        return Err(format!("workspace.snapshot 失败：{code}"));
    }

    let files = value
        .pointer("/data/files")
        .and_then(serde_json::Value::as_array)
        .ok_or("响应缺少 data.files")?;

    let total = files.len();
    let rows: Vec<String> = files
        .iter()
        .filter_map(serde_json::Value::as_str)
        .take(max_rows)
        .map(str::to_string)
        .collect();

    // 统计用**完整**列表算，不是只算显示出来的那 500 条。
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for path in files.iter().filter_map(serde_json::Value::as_str) {
        let top = path.split_once('/').map_or("(根目录)", |(top, _)| top);
        *counts.entry(top).or_default() += 1;
    }

    let mut stats: Vec<StatRow> = counts
        .into_iter()
        .map(|(name, count)| StatRow {
            name: name.to_string(),
            files: count,
            share: count as f32 / total.max(1) as f32,
        })
        .collect();
    stats.sort_by(|left, right| right.files.cmp(&left.files));

    Ok((rows, total, stats))
}
