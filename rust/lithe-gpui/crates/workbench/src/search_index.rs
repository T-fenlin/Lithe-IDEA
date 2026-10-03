//! 搜索索引在 gpui 宿主侧的**三条接线**：`workspace.searchIndex.warm` / `.update` / `.invalidate`。
//!
//! ## 为什么需要这一层（U18）
//!
//! Core 的索引本体早就写完了（`rust/lithe-core/src/project/search_index.rs`：`get_or_build` /
//! `update_paths` / `invalidate_root`），但 gpui 宿主对 `searchIndex.*` 的引用曾经是 **0**
//! （`git grep searchIndex rust/lithe-gpui` 只命中 UI-MAP 的说明）。后果不是"少一个功能"，
//! 而是**第一次搜索会在调用线程上同步建全量索引** —— 索引的构建成本必须提前到"项目打开"，
//! 而不是让用户的第一下按键去付。
//!
//! 本模块只做**接线**：时机、请求形状、代次、诊断。索引怎么建、缓存键怎么算、增量怎么应用
//! 全部归 Core（"确定性逻辑属于 Core"这条判据没有变）。
//!
//! ## 三条命令 × 触发时机（唯一真源在这张表）
//!
//! | 命令 | 触发点 | 依据 |
//! | --- | --- | --- |
//! | `warm` | `workspace.rs:1719`（`ShellWorkspace::new`） | 那是**所有"打开"路径的公共收口**（argv 启动 + `replace_root`，与 `prepare_workspace_config` 同一处论证）。每个外壳恰好排一次，不需要额外的"已排过"状态。 |
//! | `update` | `editor/src/editor_view.rs:1885`（`write_buffer` 写盘成功之后，经 `BufferWriteHost` 宿主钩子）→ `workspace.rs:1999`（`note_buffer_written`） | 编辑器的 `write_buffer` 是**唯一**的"文件落盘"收口（`Ctrl+S` / 自动保存 / 关闭前的确认保存都汇到它）。 |
//! | `invalidate` | `workspace.rs:3370`（`rebuild_project_window` 的 `replace_root` 之前） | 换根是全仓**唯一**会离开一个工作区根的路径。 |
//!
//! ## `update` 为什么只有"保存"一个驱动（H12）
//!
//! Core 的 `update_paths` 注释写着它服务 watcher（"when a watcher reports file-only changes"），
//! 而 **gpui 宿主今天没有文件系统 watcher**（`H12` 登记在 `BLOCKERS.md`），本任务也**不新建**
//! watcher。所以 `update` 只能由**已有的写盘路径**驱动：编辑器保存。传的是**这一条**变更路径
//! （不是"每次保存就 invalidate 全量"—— 那会把 warm 的收益抵消掉）。
//!
//! ## 没接上的（如实登记，不硬造）
//!
//! - **项目关闭**在现有 gpui 里**没有挂点**：没有"关项目"这条生命周期，窗口关闭＝进程退出
//!   （索引缓存在进程内存里，跟着一起没）。所以 `invalidate` 只落在换根上。
//! - **Git 的写盘路径**（`git discard/checkout` 改文件）没有接：那是 `crates/git` 的写回路径，
//!   要接得在那边再加一个宿主钩子。本任务范围只到编辑器的保存路径。
//! - **界面仍缺**（`U17`）：本模块**没有**任何渲染，也不改左活动栏"搜索"项。
//!
//! ## 请求形状
//!
//! `SearchIndexRequest { root }`（warm / invalidate）与 `SearchIndexUpdateRequest { root, paths }`
//! （update），字段名 camelCase、由 `#[serde(rename_all = "camelCase")]` 定形
//! （`rust/lithe-core/src/project/files.rs:76-98`）。`paths` 是**工作区相对路径**、`/` 分隔
//! （`workspace_relative` 的口径），不在根之下的文件根本不进这个数组。
//!
//! `hiddenDirectoryNames` / `hiddenFilePatterns` **故意不传**：Core 会自己并进
//! `BUILT_IN_HIDDEN_DIRECTORIES` / `BUILT_IN_HIDDEN_FILE_PATTERNS`（`files.rs:530-539`），
//! 而宿主侧今天没有任何一条 Core 调用传过它们（`explorer` 的 `workspace.snapshot` 也只传
//! `root`）。少传这两个字段 = 与其它调用**同一个缓存键**（`search_index.rs:72-78` 的键含
//! 可见性规则），不会让同一次搜索再建第二份索引。
//!
//! ## 代次（stale 结果保护）与它今天的**真实边界**
//!
//! 每条 `warm` / `update` 都带着发出时的代次进后台，回来时经 `SearchIndexWiring::accepts`
//! 判定"还算不算当前根的索引状态"；不成立就只留一行 `stale=1`，**不采纳**。
//!
//! ⚠️ 诚实边界：换根走的是 `window.replace_root` —— 它**重建整个 `ShellWorkspace`**，
//! 所以旧根的 warm 任务今天本来就更不回新外壳（`WeakEntity::update` 拿不到实体）。
//! 也就是说"旧根结果不能被当成新根的"这条**今天首先由架构保证**，代次闸门是把那条结构性
//! 保证变成**显式、可单测的判据**（见本文件末尾的单测），并在 U17 把索引状态显示出来时
//! 成为同一个采纳点。不要把它读成"今天已经有过一次真实串台"。

use std::path::{Path, PathBuf};

use gpui_kit::{AppContext as _, Context};
use lithe_gpui_shared::{CoreError, core_json};
use serde_json::{Value, json};

use crate::workspace::ShellWorkspace;

/// 诊断前缀（`eprintln!`，stderr）。与 `S1_*` 一族同口径：stdout 重定向到文件是块缓冲的，
/// 进程还在跑时可能一行都没落盘，所以宿主诊断一律走 stderr。
const TAG: &str = "S1_SEARCH_INDEX";

/// `workspace.searchIndex.warm`（`protocol/command.rs:357`）。
const WARM_COMMAND: &str = "workspace.searchIndex.warm";
/// `workspace.searchIndex.update`（`protocol/command.rs:358`）。
const UPDATE_COMMAND: &str = "workspace.searchIndex.update";
/// `workspace.searchIndex.invalidate`（`protocol/command.rs:359`）。
const INVALIDATE_COMMAND: &str = "workspace.searchIndex.invalidate";

/// `warm` / `invalidate` 的 payload（= `SearchIndexRequest`）。
fn root_payload(root: &str) -> Value {
    json!({ "root": root })
}

/// `update` 的 payload（= `SearchIndexUpdateRequest`）。
fn update_payload(root: &str, paths: &[String]) -> Value {
    json!({ "root": root, "paths": paths })
}

/// 把"刚写回磁盘的绝对路径"翻成 Core 的 `paths` 口径（工作区相对、`/` 分隔）。
///
/// 不在这个根之下时返回 `None`，调用方**不发**这次 update：那一类文件本来就不在索引里
/// （Core 的 `normalized_update_path` 也会把它 `continue` 掉，`search_index.rs:331-340`），
/// 发过去只会白跑一次 Core 信封。
fn index_path(root: &Path, written: &Path) -> Option<String> {
    lithe_gpui_shared::workspace_config::workspace_relative(root, written)
}

/// 搜索索引接线：**一个工作区根 + 一个代次**。
///
/// 它是 `ShellWorkspace` 的一个字段，生命周期与外壳相同（换根＝换外壳＝换一份接线）。
pub(crate) struct SearchIndexWiring {
    /// 所有命令都发往这个根；诊断行也打它（**不用** `self.root` 之外的任何来源）。
    root: PathBuf,
    /// 代次：`invalidate`（换根）时 +1，之后回来的旧代次结果一律不采纳。
    generation: u64,
}

impl SearchIndexWiring {
    /// 绑定一个工作区根（代次从 0 起）。
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            generation: 0,
        }
    }

    /// 当前代次（排任务时带走的就是它）。
    fn generation(&self) -> u64 {
        self.generation
    }

    /// 换代，返回新一代。
    fn next_generation(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }

    /// 这条完成于 `issued` 代次的结果，还算不算"**当前根**的索引状态"。
    fn accepts(&self, issued: u64) -> bool {
        self.generation == issued
    }

    /// **项目打开**：后台建好（或复用）全量索引。
    ///
    /// 这是本模块存在的理由：不排这一步，用户第一次搜索就要在调用线程上同步建全量索引。
    /// Core 调用是同步阻塞的，所以整段进 `background_spawn`（`shared/src/core_client.rs:27-28`），
    /// **不在 UI 线程上等**。失败只留 `S1_SEARCH_INDEX warm … status=failed`，**不影响开项目**
    /// （降级为"第一次搜索时再建"）。
    pub(crate) fn schedule_warm(&self, cx: &mut Context<ShellWorkspace>) {
        let root_text = self.root.to_string_lossy().to_string();
        let issued = self.generation();
        // 任务 detach 掉：它不碰实体以外的东西，外壳被换根丢掉时 `this.update` 自然失败。
        // 与 `prepare_workspace_config` 同一形状（`workspace.rs:4734`）。
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { core_json(WARM_COMMAND, root_payload(&root_text)) })
                .await;
            let _ = this.update(cx, |shell, _cx| {
                shell.search_index.adopt("warm", issued, result)
            });
        })
        .detach();
    }

    /// **保存 / 写回**：把这一条变更路径增量喂给已有索引。
    ///
    /// 只发 `update`（单文件增量），**不退化成"每次保存就 invalidate 全量"**。
    /// 一次真实写盘只会调到这里一次：`write_buffer` 是唯一收口（手动 `Ctrl+S` / 去抖后的
    /// 自动保存 / 关闭前的确认保存都走它），所以这里不需要再去抖。
    pub(crate) fn schedule_update(&self, written: &Path, cx: &mut Context<ShellWorkspace>) {
        let Some(relative) = index_path(&self.root, written) else {
            // 保存的文件不在这个工作区里（例如 `--session-probe` 开了仓库外的文件）：
            // 索引里没有它，不发调用，但**留下痕迹**（不静默）。
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
                shell.search_index.adopt("update", issued, result)
            });
        })
        .detach();
    }

    /// **换根**：先换代（在飞的旧根结果一律作废），再让 Core 丢掉旧根的缓存。
    ///
    /// 调用点在 `rebuild_project_window` 的 `replace_root` **之前** —— 那时 `self` 还是旧根
    /// 的外壳。Core 调用照旧进后台；结果**不回写实体**（外壳正在被丢掉），只走 stderr。
    pub(crate) fn schedule_invalidate(&mut self, cx: &mut Context<ShellWorkspace>) {
        let issued = self.next_generation();
        let root_text = self.root.to_string_lossy().to_string();
        // payload 在**这里**拼好再搬进后台：`json!` 是一次微不足道的分配，而留在这里就不必
        // 为了打诊断行再克隆一份根的文本（下面 `Ok` / `Err` 两行都要用它）。
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
                // 失败不静默：换根本身照常继续（清缓存失败只意味着旧根索引多留一会儿内存，
                // Core 的缓存键含根路径，不会串到新根上）。
                Err(error) => {
                    eprintln!("{TAG} invalidate root={root_text} status=failed error={error}")
                }
            }
        })
        .detach();
    }

    /// 后台结果回到前台：**先过代次闸门**，再打一行可 grep 的诊断。
    ///
    /// 唯一采纳动作就是这一行诊断 —— U17 把索引状态显示出来时，同一个闸门也是它的采纳点。
    fn adopt(&self, op: &str, issued: u64, result: Result<Option<Value>, CoreError>) {
        if !self.accepts(issued) {
            // 旧代次：这条结果描述的不是当前根的索引，不采纳、也不打"当前根"的结论。
            eprintln!("{TAG} {op} stale=1 root={}", self.root.display());
            return;
        }
        eprintln!("{}", result_line(op, &self.root, &result));
    }
}

/// 一次索引调用**采纳后**的诊断行（决定这行长什么样的唯一一处）。
///
/// 它是"接线真的通到 Core 了"的人工核对锚点（U18 的验证记录 grep 它），所以做成纯函数并用
/// 单测钉住字段名，而不是把格式散在几处 `eprintln!` 里。
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
        // `ok:true` 但没有 `data`：Core 的这三条命令都一定回 `data`（`dispatcher.rs:304-334`），
        // 所以这是契约异常 —— 如实报出来，不编数字。
        Ok(None) => format!("{TAG} {op} root={root} status=missing_data"),
        Err(error) => format!("{TAG} {op} root={root} status=failed error={error}"),
    }
}

/// 取一个计数：字段缺失时给 `?` 而**不是** `0` —— `0` 会被读成"索引里一个文件都没有"。
fn count(data: &Value, key: &str) -> String {
    data.get(key)
        .and_then(Value::as_u64)
        .map_or_else(|| "?".to_string(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// payload 的字段名是**契约面**（`rename_all = "camelCase"`），拼错 Core 就用
    /// `invalid_request` 拒掉整条命令，且只有跑起来才发现。这里逐字钉住。
    #[test]
    fn payloads_follow_the_core_contract_field_names() {
        assert_eq!(root_payload("D:/proj"), json!({ "root": "D:/proj" }));
        assert_eq!(
            update_payload("D:/proj", &["src/a.java".to_string()]),
            json!({ "root": "D:/proj", "paths": ["src/a.java"] })
        );
        // 可见性数组**不出现**在 payload 里，由 Core 的 `#[serde(default)]` 兜底成 `[]`：
        // 宿主侧少一个会漂移的字段，也保证与 `workspace.snapshot` 同一个缓存键。
        assert!(
            root_payload("r").get("hiddenDirectoryNames").is_none(),
            "宿主不传可见性数组（见模块文档「请求形状」）"
        );
        assert!(update_payload("r", &[]).get("hiddenFilePatterns").is_none());
    }

    /// `paths` 是**工作区相对**、`/` 分隔；根之外的文件不进数组（返回 `None`）。
    /// 路径用 `join` 拼而不是写字面反斜杠，测试在 Windows / Unix 上都成立。
    #[test]
    fn update_paths_are_workspace_relative_and_scoped_to_the_root() {
        let root = Path::new("proj-root");
        let inside = root.join("src").join("a.java");
        assert_eq!(index_path(root, &inside).as_deref(), Some("src/a.java"));

        let outside = Path::new("elsewhere").join("a.java");
        assert_eq!(index_path(root, &outside), None);
    }

    /// 换根之后，旧代次的结果**必须**被丢弃（`accepts` 是它唯一的判据）。
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

    /// 诊断行是人工核对锚点，格式（含操作名与字段名）必须稳定；
    /// "契约异常"与"失败"必须有**不同**的行，不能被读成同一件事。
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

        // 缺字段读成 `?`，不是 0。
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

    /// **真实 Core 往返**：临时夹具上走一遍 `warm` → 新增一个文件 → `update`。
    ///
    /// 为什么必须有这一条：上面几条单测只钉住"宿主拼出来的 JSON 长什么样"，钉不住
    /// "Core 收得下这个请求"。字段名错一位，Core 就用 `invalid_request` 拒掉整条命令，
    /// 而那种失败在界面上只表现为"第一次搜索很慢"（正是本任务要消灭的那个现象）。
    ///
    /// 形状照仓库既有夹具（`run/src/model.rs:758-766`）：`temp_dir` + 进程 id、真文件、
    /// 真 `core_json`，**不依赖网络 / 真实进程 / 时钟**，所以是确定性的。清理是显式的
    /// （开头先清、结尾再清），并且用 `invalidate` 把这一份进程级索引缓存放掉 ——
    /// 同一进程里其它测试用的是别的根（键含根路径），互不影响。
    #[test]
    fn a_real_core_round_trip_warms_then_updates_incrementally() {
        let root = std::env::temp_dir().join(format!("lithe-search-index-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("建目录失败");
        std::fs::write(root.join("src").join("A.java"), "class A {}\n").expect("写文件失败");

        let root_text = root.to_string_lossy().to_string();

        // ① warm：一个文件、不是重建。
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

        // ② 只把**这一条**新路径喂给 update（模拟一次保存）。
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

        // ③ 换根口径的收尾：放掉这份缓存，再删夹具（不留进程级残留）。
        core_json(INVALIDATE_COMMAND, root_payload(&root_text)).expect("invalidate 必须成功");
        let _ = std::fs::remove_dir_all(&root);
    }
}
