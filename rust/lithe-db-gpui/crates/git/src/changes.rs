//! 左栏「源代码管理」视图的**数据层**：工作区变更快照 + 操作状态 + 文件级写操作。
//!
//! 规格真源：`lithe-db-gpui/research/windows/08-source-control.md`（643 行，逐条带 `文件:行号`），
//! 对应 Windows 前端 `windows/tauri/src/features/git/components/{git-view.tsx,
//! status/git-status-panel.tsx, status/git-status-file-item.tsx, git-commit-panel.tsx,
//! git-operation-banner.tsx}`。
//!
//! 本模块不渲染任何东西：视图层是 `changes_view.rs`，它只从这里取数据与纯函数。
//!
//! # 数据来源（逐条对真源）
//!
//! | 用途 | 命令 | payload | 响应 |
//! | --- | --- | --- | --- |
//! | 仓库根 / 分支 / ahead-behind / 变更列表 | `git.status` | `{ root }`（`git/mod.rs:6536`） | `{ repositoryRoot, branch, ahead, behind, changes[] }`（`protocol/contracts.rs:532-538`） |
//! | 进行中的 merge / rebase / cherry-pick / revert | `git.operationState` | `{ root }`（`rust-core-api.md:840`） | `{ kind, reference, step, total, conflictedPaths }`（`contracts.rs:795-802`） |
//! | 暂存 / 取消暂存 | `git.write` | `{ root, operation: "stage" \| "unstage", paths }`（`git/mod.rs:876-903`） | `GitCommandResponse` |
//! | 暂存全部 | `git.write` | `{ root, operation: "stageAll" }`（`git/mod.rs:943`） | 同上 |
//! | 提交 | `git.write` | `{ root, operation: "commit", message, paths }`（`git/mod.rs:944-955`） | 同上 |
//! | 丢弃改动（工作树还原） | `git.write` | `{ root, operation: "discard", paths }`（`git/mod.rs:907-941`） | 同上 |
//! | 继续 / 中止 / 跳过 | `git.write` | `{ root, operation: "operationContinue" \| "operationAbort" \| "operationSkip" }`（`git/mod.rs` 的 `write` 分支） | 同上 |
//! | 「这是不是一个有提交的仓库」 | `git.references` | `{ root }`（`git/history.rs:49-51`） | `{ references[], … }`（`contracts.rs:629-635`） |
//!
//! # 三条硬约束（都在真源里核对过，违反会静默做错事）
//!
//! 1. **`git.status.repositoryRoot` 可能是相对工作区根的路径**（`git/mod.rs:6574` 的
//!    `relative_or_absolute`）：直接当 `root` 用会按**进程 CWD** 解析，必须先拼成绝对路径
//!    （[`resolve_repository_root`](crate::model::resolve_repository_root)）。
//! 2. **「非仓库」没有专用错误码**：`git.status` 返回 `ok:true` + `repositoryRoot: null`
//!    （`git/mod.rs:6535-6544`），其余 `git.*` 返回 `ok:false` + `process_failed`。
//!    所以**先探 `git.status`**，不是仓库就不白跑后面几条。
//! 3. ⚠️ **写操作失败时 Core 的信封仍然是 `ok:true`**：`execute_git` 即使 Git 退出码非 0 也
//!    返回 `Ok(GitCommandResponse)`（`git/mod.rs:1326-1395`，只有少数分支显式改成 `Err`），
//!    失败信息在 `exitCode` / `stderr` / `operationError` 里。真源的写操作失败**常常是完全静默的**
//!    （Windows 侧只 `console.error`），本侧**不照抄**：每个写操作都必须把非零退出码翻成
//!    `Err(用户可见的一句话)`，由视图画成错误条 / 提交面板红块。见 [`WriteOutcome`]。
//!
//! # 与 Windows 真源的刻意偏差（都写在这里，不藏在代码里）
//!
//! 1. **提交不带 `paths`**（= 提交索引里已暂存的内容，`git commit -m`）：真源每个文件行有一个
//!    「提交范围」复选框（`git-status-file-item.tsx:156-165`），提交时把勾选的路径作为 `paths`
//!    传给 `git.write`。第一版**不做复选框**（研究 §6.4 的第一版范围也没要求），所以没有
//!    "选中集合"这个维度，提交恒走索引。代价：用户必须**先暂存**再提交（这正是 IDE 的常规流程，
//!    也是提交按钮的禁用条件之一）。
//! 2. **「无提交」状态用 `git.references` 判**：真源用 `git.repositorySetup` 的 `hasCommits`
//!    （`git-repository-empty-state.tsx:50`、`rust-core-api.md:169`）。本模块不新增 Core 命令，
//!    改判据为「`git.status` 无变更 **且** `git.references` 一条引用都没有」—— 一个提交都没有的
//!    仓库不可能有任何引用（HEAD 未诞生），语义等价且只在本就空列表时多跑一条本地读。
//! 3. **丢弃只做 `git.write discard`（工作树还原）**：Core **没有**"删除工作区文件"命令
//!    （研究 §4.2 第三条），真源的右键「删除」走平台文件系统删除
//!    （`git-status-panel.tsx:557-602`），本侧没有这条通路，所以**不画**该动作。

use std::path::Path;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{App, Hsla, SharedString};

use crate::model::{
    discover_repository_root, execute_core, parse_references, repository_root_of,
    resolve_repository_root,
};

/// 变更状态档，逐字对应 Windows 的 `GitFile["status"]`
/// （`features/git/types/git.types.ts`，归一逻辑在 `platform/core-result-adapter.ts:15-26`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    /// 未跟踪（`??`）：**单独一个分类**，不属于「已跟踪」。
    Untracked,
}

impl ChangeKind {
    /// 把 Core 的 `status`（两字符 porcelain 码）归一成状态档。
    ///
    /// 逐字照 `core-result-adapter.ts:15-26` 的 `normalizeStatus`（包括分支顺序：
    /// 先看 `untracked`，再看索引位 `A`，再看任一位的 `D`，再看 `R`/`C`，最后工作树位 `A`）。
    pub fn parse(status: &str, untracked: bool) -> Self {
        if untracked || status.contains('?') {
            return Self::Untracked;
        }
        let mut chars = status.chars();
        let index = chars.next().unwrap_or(' ');
        let worktree = chars.next().unwrap_or(' ');
        if index == 'A' {
            return Self::Added;
        }
        if worktree == 'D' || index == 'D' {
            return Self::Deleted;
        }
        if [index, worktree].iter().any(|code| *code == 'R' || *code == 'C') {
            return Self::Renamed;
        }
        if worktree == 'A' {
            return Self::Added;
        }
        Self::Modified
    }

    /// 「已跟踪」分类内的分组顺序（`git-status-model.ts:10-16` 的 `GIT_STATUS_ORDER`）。
    pub fn group_order(self) -> usize {
        match self {
            Self::Added => 0,
            Self::Modified => 1,
            Self::Deleted => 2,
            Self::Renamed => 3,
            Self::Untracked => 4,
        }
    }

    /// 行内文件名的颜色。
    ///
    /// ⚠️ **映射到主题 token**（本仓库要求「颜色一律 `cx.theme()`」）：真源是
    /// `getWorkingTreeStatusColorClassName`（`utils/git-file-status-visuals.ts:3-16`）的
    /// `--git-added` / `--info` / `--subtle-foreground` / `--git-renamed` / `--git-deleted`
    /// 五个裸 token —— 它们在 `windows/tauri/src` 里**只有引用、没有定义**（全量 grep 只找到
    /// `styles/theme.css:49-56` 的 `--color-*: var(--git-*)` 转发），运行期由主题注入，
    /// 所以这里按语义取最近的一档：added→`green_light`、modified→`blue_light`、
    /// deleted→`muted_foreground`、renamed→`magenta_light`、untracked→`danger`
    /// （IntelliJ 的未跟踪是红色，与 `log_view` 的偏差 3 同一条口径）。
    pub fn color(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        match self {
            Self::Added => theme.green_light,
            Self::Modified => theme.blue_light,
            Self::Deleted => theme.muted_foreground,
            Self::Renamed => theme.magenta_light,
            Self::Untracked => theme.danger,
        }
    }

    /// 是否归入「未跟踪」分类（`git-status-model.ts:143-151`）。
    pub fn is_untracked(self) -> bool {
        self == Self::Untracked
    }
}

/// 一行工作区变更（Core `GitChange`，`protocol/contracts.rs:518-527`）。
#[derive(Clone, Debug)]
pub struct ChangeRow {
    /// **仓库相对**路径，`/` 分隔（Core 约定）。
    pub path: SharedString,
    /// rename / copy 的源路径（`originalPath`）。
    pub original_path: Option<SharedString>,
    /// Core 的原始两字符码，保留给诊断（`" M"` / `"M "` / `"??"` / `"R "`…）。
    pub raw_status: SharedString,
    pub kind: ChangeKind,
    /// 索引里有改动（Core 的 `staged`，即 `X` 非空且非 `?`）。
    pub staged: bool,
    /// 工作树里有改动（Core 的 `worktree`）。
    pub worktree: bool,
}

/// 进行中的顺序操作（Core `git.operationState.kind`，`contracts.rs:797`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationKind {
    Merge,
    Rebase,
    CherryPick,
    Revert,
}

impl OperationKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "merge" => Some(Self::Merge),
            "rebase" => Some(Self::Rebase),
            "cherryPick" => Some(Self::CherryPick),
            "revert" => Some(Self::Revert),
            _ => None,
        }
    }

    /// 横幅标题的文案 key，逐字取自 `git-operation-banner.tsx:57-62`。
    pub fn title_key(self) -> &'static str {
        match self {
            Self::Merge => "lithe.git.mergeInProgress",
            Self::Rebase => "lithe.git.rebaseInProgress",
            Self::CherryPick => "lithe.git.cherryPickInProgress",
            Self::Revert => "lithe.git.revertInProgress",
        }
    }

    /// 「继续」按钮的文案 key（`git-operation-banner.tsx:63-68`）。
    pub fn continue_key(self) -> &'static str {
        match self {
            Self::Merge => "lithe.git.continueMerge",
            Self::Rebase => "lithe.git.continueRebase",
            Self::CherryPick => "lithe.git.continueCherryPick",
            Self::Revert => "lithe.git.continueRevert",
        }
    }

    /// 只有 rebase 能「跳过提交」（`git-operation-banner.tsx:118-128`；
    /// Core 侧同样只支持 rebase，`rust-core-api.md:807-812`）。
    pub fn can_skip(self) -> bool {
        self == Self::Rebase
    }

    /// 诊断用的稳定名（= Core 的 `kind` 原文）。
    pub fn id(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Rebase => "rebase",
            Self::CherryPick => "cherryPick",
            Self::Revert => "revert",
        }
    }
}

/// 一次进行中的 merge / rebase / cherry-pick / revert（`contracts.rs:795-802`）。
#[derive(Clone, Debug)]
pub struct OperationState {
    pub kind: OperationKind,
    /// 被操作的引用短哈希（真源横幅画前 7 位，`git-operation-banner.tsx:79-83`）。
    pub reference: Option<SharedString>,
    /// 只有 rebase 有进度（`contracts.rs:799-800`）。
    pub step: Option<usize>,
    pub total: Option<usize>,
    /// 冲突路径（已排序去重，`contracts.rs:801`）。
    pub conflicted_paths: Vec<SharedString>,
}

impl OperationState {
    /// 有冲突时**不能**继续（Core 会拒绝：`rust-core-api.md:807-812`；真源同样禁用按钮，
    /// `git-operation-banner.tsx:109-117`）。
    pub fn has_conflicts(&self) -> bool {
        !self.conflicted_paths.is_empty()
    }
}

/// 左栏「源代码管理」的一次读取结果。
///
/// 必须在后台线程上构造再送回前台，所以只装 `String` / `SharedString` 这些 `Send` 数据。
#[derive(Clone, Debug, Default)]
pub struct ChangesSnapshot {
    /// 解析成**绝对路径**的仓库根；`None` = 这个工作区不是 Git 仓库（也不含仓库子目录）。
    pub repository_root: Option<SharedString>,
    pub branch: Option<SharedString>,
    pub ahead: usize,
    pub behind: usize,
    /// 已合并同路径记录、已排序的变更行。
    pub changes: Vec<ChangeRow>,
    pub operation: Option<OperationState>,
    /// 仓库**一个引用都没有**（= 还没有任何提交）。只在变更列表为空时才去读
    /// （偏差 2）。`None` = 没读过（变更非空，或不是仓库）。
    pub has_commits: Option<bool>,
    /// 失败的命令名（空 = 全部成功）。**不静默**：调用方拿它画错误条并打诊断。
    pub failures: Vec<String>,
}

impl ChangesSnapshot {
    /// 已暂存的文件数。
    pub fn staged_count(&self) -> usize {
        self.changes.iter().filter(|row| row.staged).count()
    }

    /// 未暂存的文件数（**含未跟踪**）。
    ///
    /// 判据照真源：工具行「暂存所有更改 / 取消暂存所有更改」的出现条件是
    /// `unstagedFiles.length > 0` / `stagedFiles.length > 0`
    /// （`git-status-panel.tsx:1157,1169`），而 `unstagedFiles` 是**所有** `!staged` 的文件
    /// （`git-status-model.ts:121-127`，未跟踪文件的 `staged` 也是 false）——
    /// 也就是说"只有未跟踪文件"时真机**照样**给「暂存所有更改」。
    pub fn unstaged_count(&self) -> usize {
        self.changes.iter().filter(|row| !row.staged).count()
    }

    /// 已跟踪分类的行（`git-status-model.ts:143-151` 的 `trackedFiles`）。
    pub fn tracked_rows(&self) -> Vec<&ChangeRow> {
        self.changes
            .iter()
            .filter(|row| !row.kind.is_untracked())
            .collect()
    }

    /// 未跟踪分类的行。
    pub fn untracked_rows(&self) -> Vec<&ChangeRow> {
        self.changes
            .iter()
            .filter(|row| row.kind.is_untracked())
            .collect()
    }

    /// 视图空态标签（诊断与渲染共用同一个判据，避免两处漂移）。
    pub fn state_label(&self) -> &'static str {
        if self.repository_root.is_none() {
            "norepo"
        } else if !self.changes.is_empty() {
            "ready"
        } else if self.has_commits == Some(false) {
            "nocommits"
        } else {
            "clean"
        }
    }

    /// 一行可 grep 的诊断（口径同 `S1_EXPLORER` / `S1_BRANCH_PANEL`，走 **stderr**）。
    pub fn diagnose(&self) -> String {
        format!(
            "S1_SOURCE_CONTROL files={} staged={} unstaged={} state={} root={} branch={} op={}",
            self.changes.len(),
            self.staged_count(),
            self.unstaged_count(),
            self.state_label(),
            self.repository_root.as_deref().unwrap_or("-"),
            self.branch.as_deref().unwrap_or("-"),
            self.operation
                .as_ref()
                .map(|operation| operation.kind.id())
                .unwrap_or("none"),
        )
    }
}

/// 一次「按仓库根读取」的结果（`git.status` → 兜底探测 → `git.operationState` → 空列表时 `git.references`）。
///
/// 非仓库时**不再白跑**后面几条（约束 2）。
pub fn load(root: &Path) -> ChangesSnapshot {
    let mut snapshot = ChangesSnapshot::default();
    let root_text = root.to_string_lossy().to_string();

    // ① `git.status`：仓库根 + 分支 + ahead/behind + 变更列表。
    match execute_core("git.status", serde_json::json!({ "root": root_text })) {
        Ok(data) => {
            snapshot.repository_root =
                repository_root_of(&data).map(|found| resolve_repository_root(root, &found).into());
            snapshot.branch = data
                .get("branch")
                .and_then(serde_json::Value::as_str)
                .map(SharedString::from);
            snapshot.ahead = data
                .get("ahead")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as usize;
            snapshot.behind = data
                .get("behind")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as usize;
            snapshot.changes = parse_changes(&data);
        }
        Err(code) => snapshot.failures.push(format!("git.status（{code}）")),
    }

    // ② 工作区根不是仓库时，用一级子目录再找一次（与 `model::load_first` 同一兜底）。
    if snapshot.repository_root.is_none() {
        snapshot.repository_root = discover_repository_root(root).map(SharedString::from);
    }

    let Some(repository_root) = snapshot.repository_root.clone() else {
        return snapshot;
    };

    // ③ `git.operationState`：进行中的操作（提交会被它挡住，所以没它用户会看到"点了没反应"）。
    match execute_core(
        "git.operationState",
        serde_json::json!({ "root": repository_root.as_ref() }),
    ) {
        Ok(data) => snapshot.operation = parse_operation(&data),
        Err(code) => snapshot
            .failures
            .push(format!("git.operationState（{code}）")),
    }

    // ④ 变更列表为空时判「有没有提交」（偏差 2）。变更非空时不需要这个信息，
    //    所以这条读是**有条件**的（不是白跑）。
    if snapshot.changes.is_empty() {
        snapshot.has_commits = match execute_core(
            "git.references",
            serde_json::json!({ "root": repository_root.as_ref() }),
        ) {
            Ok(data) => Some(!parse_references(&data).is_empty()),
            Err(code) => {
                snapshot.failures.push(format!("git.references（{code}）"));
                None
            }
        };
    }

    snapshot
}

/// 解析 `git.status.changes[]`，并合并同路径的索引 / 工作树两条记录。
///
/// 真源对应 `buildVisibleGitFiles` → `coalesceWholePathStatuses`（`git-status-model.ts:54-85,167-181`）：
/// 同一路径出现两次时保留**已暂存**的那一条。Core 侧已经把 `AD` 过滤掉了
/// （`git/mod.rs:6626-6629`），所以剩下的重复只有 `XY` 两种位同时非空的组合。
pub fn parse_changes(data: &serde_json::Value) -> Vec<ChangeRow> {
    let mut rows: Vec<ChangeRow> = Vec::new();

    let Some(changes) = data.get("changes").and_then(serde_json::Value::as_array) else {
        return rows;
    };

    for change in changes {
        let Some(path) = change.get("path").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let raw_status = change
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let untracked = change
            .get("untracked")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let row = ChangeRow {
            path: SharedString::from(path.to_string()),
            original_path: change
                .get("originalPath")
                .and_then(serde_json::Value::as_str)
                .map(SharedString::from),
            raw_status: SharedString::from(raw_status.to_string()),
            kind: ChangeKind::parse(raw_status, untracked),
            staged: change
                .get("staged")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            worktree: change
                .get("worktree")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        };

        match rows.iter_mut().find(|existing| existing.path == row.path) {
            // 已暂存的那条胜出（`mergeWholePathStatus` 的最后一行）。
            Some(existing) => {
                if !existing.staged && row.staged {
                    *existing = row;
                }
            }
            None => rows.push(row),
        }
    }

    // 顺序 = 分类（已跟踪 → 未跟踪）+ 分类内 `GIT_STATUS_ORDER` + 路径。Core 已按 path 排序，
    // 这里重排一次是为了让"分类头 + 组内顺序"和真源一致（`git-status-panel.tsx:334-420`）。
    rows.sort_by(|left, right| {
        left.kind
            .is_untracked()
            .cmp(&right.kind.is_untracked())
            .then_with(|| left.kind.group_order().cmp(&right.kind.group_order()))
            .then_with(|| left.path.cmp(&right.path))
    });
    rows
}

/// 解析 `git.operationState`；`kind` 为空串 = 没有进行中的操作（`contracts.rs:796-797`）。
pub fn parse_operation(data: &serde_json::Value) -> Option<OperationState> {
    let kind = data
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .and_then(OperationKind::parse)?;

    Some(OperationState {
        kind,
        reference: data
            .get("reference")
            .and_then(serde_json::Value::as_str)
            .map(SharedString::from),
        step: data
            .get("step")
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize),
        total: data
            .get("total")
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize),
        conflicted_paths: data
            .get("conflictedPaths")
            .and_then(serde_json::Value::as_array)
            .map(|paths| {
                paths
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(SharedString::from)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

// ---------------------------------------------------------------------------
// 写操作
// ---------------------------------------------------------------------------

/// 一次写操作的结果。
///
/// ⚠️ **两条失败通路都要看**（硬约束 3）：
/// 1. 信封级失败 → `execute_core` 给 `Err(Core 错误码)`；
/// 2. **信封 `ok:true` 但 Git 退出码非 0** → 从 `exitCode` / `stderr` / `operationError` 里
///    拼一句给人看的话。真源的写操作失败常常只 `console.error`（研究 §5 末表），本侧不照抄。
#[derive(Debug)]
pub struct WriteOutcome {
    /// `None` = 成功；`Some(一句话)` = 用户可见的失败原因。
    pub failure: Option<String>,
}

/// 调一次 `git.write` 并把「Git 非零退出」翻成失败。
///
/// `redact` 是仓库根：Git 的 stderr 里常带绝对路径，真源刻意不把原始 stderr 给用户
/// （`git-status-panel.tsx:174-204` 的注释：stderr 可能含绝对路径、筛选输出或凭据）。
/// 这里保留 Git 自己的**一句话**（它通常就是"nothing to commit"这类可读信息），
/// 但把仓库根的绝对路径剪掉，避免把开发机路径摊到界面上。
pub fn write(operation: &str, root: &str, payload: serde_json::Value) -> WriteOutcome {
    let mut request = serde_json::json!({ "root": root, "operation": operation });
    if let (Some(target), Some(extra)) = (request.as_object_mut(), payload.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }

    let data = match execute_core("git.write", request) {
        Ok(data) => data,
        Err(code) => {
            return WriteOutcome {
                failure: Some(code),
            };
        }
    };

    // ① Core 自己记下的失败（例如 `commit_selected_paths` 里的校验失败）。
    if let Some(error) = data.get("operationError") {
        let message = error
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("operation failed");
        let code = error
            .get("code")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("operation_failed");
        return WriteOutcome {
            failure: Some(format!("{code}: {}", redact(message, root))),
        };
    }

    // ② Git 退出码：`execute_git` 即使非 0 也返回 `Ok`，所以这一条必须自己判。
    let exit_code = data
        .get("exitCode")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    if exit_code == 0 {
        return WriteOutcome { failure: None };
    }

    let stderr = data
        .get("stderr")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let stdout = data
        .get("stdout")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let detail = first_meaningful_line(stderr)
        .or_else(|| first_meaningful_line(stdout))
        .unwrap_or_else(|| format!("git exited with {exit_code}"));
    WriteOutcome {
        failure: Some(redact(&detail, root)),
    }
}

/// 把 Git 输出里第一行有内容的文本取出来（Git 的错误一般在 stderr 首行）。
fn first_meaningful_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

/// 剪掉消息里的仓库根绝对路径（理由见 [`write`]）。
fn redact(message: &str, root: &str) -> String {
    if root.is_empty() {
        return message.to_string();
    }
    // 同时剪掉两种分隔符的写法：Git 在 Windows 上可能回显 `/` 或 `\`。
    let forward = root.replace('\\', "/");
    let backward = root.replace('/', "\\");
    let mut text = message
        .replace(root, "")
        .replace(&forward, "")
        .replace(&backward, "");
    text = text.replace("''", "").trim().to_string();
    if text.is_empty() {
        "git command failed".to_string()
    } else {
        text
    }
}

/// `git.write operation:"stage"`（`git/mod.rs:876-885`）。
pub fn stage(root: &str, paths: &[String]) -> WriteOutcome {
    write("stage", root, serde_json::json!({ "paths": paths }))
}

/// `git.write operation:"unstage"`（`git/mod.rs:886-906`）。
///
/// ⚠️ **不要照抄 Windows 的「取消暂存全部」**：它走 `git.command ["reset","HEAD"]`
/// （`platform.rs:580-583`），研究 §4.2 明确点名这不是 `git.write` 的 `unstage`；
/// 本侧一律用 `unstage` + 路径列表（语义等价且更安全）。
pub fn unstage(root: &str, paths: &[String]) -> WriteOutcome {
    write("unstage", root, serde_json::json!({ "paths": paths }))
}

/// `git.write operation:"stageAll"`（`git/mod.rs:943`，等价 `git add --all`）。
pub fn stage_all(root: &str) -> WriteOutcome {
    write("stageAll", root, serde_json::json!({}))
}

/// `git.write operation:"commit"`（`git/mod.rs:944-955`）。
///
/// `paths` 为空 = 提交索引（`git commit -m`）；非空 = 只提交这些路径。本侧第一版恒传空
/// （偏差 1）。`message` 是**必填**，Core 用 `required_text` 校验
/// （`git/mod.rs:945`），空消息会拿到 `invalid_request`。
pub fn commit(root: &str, message: &str) -> WriteOutcome {
    write(
        "commit",
        root,
        serde_json::json!({ "message": message, "paths": Vec::<String>::new() }),
    )
}

/// `git.write operation:"discard"`（`git/mod.rs:907-941`，工作树还原；未跟踪路径走 `clean -f -d`）。
pub fn discard(root: &str, paths: &[String]) -> WriteOutcome {
    write("discard", root, serde_json::json!({ "paths": paths }))
}

/// `git.write` 的 `operationContinue` / `operationAbort` / `operationSkip`
/// （`rust-core-api.md:807-812`）。
pub fn operation_action(root: &str, operation: &str) -> WriteOutcome {
    write(operation, root, serde_json::json!({}))
}

#[cfg(test)]
mod tests {
    use super::{ChangeKind, ChangeRow, OperationKind, parse_changes, parse_operation};
    use gpui_kit::SharedString;

    fn row(path: &str, status: &str, untracked: bool) -> ChangeRow {
        ChangeRow {
            path: SharedString::from(path),
            original_path: None,
            raw_status: SharedString::from(status),
            kind: ChangeKind::parse(status, untracked),
            staged: !status.starts_with(' ') && !untracked,
            worktree: !status.ends_with(' ') && !untracked,
        }
    }

    /// 归一逻辑逐条对齐 `core-result-adapter.ts:15-26`：分类顺序错一个都会让行染色错。
    #[test]
    fn status_codes_normalize_like_the_windows_adapter() {
        assert_eq!(ChangeKind::parse("??", true), ChangeKind::Untracked);
        assert_eq!(ChangeKind::parse(" M", false), ChangeKind::Modified);
        assert_eq!(ChangeKind::parse("M ", false), ChangeKind::Modified);
        assert_eq!(ChangeKind::parse("MM", false), ChangeKind::Modified);
        assert_eq!(ChangeKind::parse("A ", false), ChangeKind::Added);
        assert_eq!(ChangeKind::parse(" A", false), ChangeKind::Added);
        assert_eq!(ChangeKind::parse(" D", false), ChangeKind::Deleted);
        assert_eq!(ChangeKind::parse("D ", false), ChangeKind::Deleted);
        assert_eq!(ChangeKind::parse("R ", false), ChangeKind::Renamed);
        assert_eq!(ChangeKind::parse("RM", false), ChangeKind::Renamed);
        // 索引位 `A` 比工作树位 `D` 优先（`AD` 会被 Core 提前过滤，但归一本身不能反）。
        assert_eq!(ChangeKind::parse("AD", false), ChangeKind::Added);
    }

    /// 未跟踪文件**不**进「已跟踪」分类（`git-status-model.ts:143-151`）。
    #[test]
    fn untracked_is_its_own_category() {
        assert!(ChangeKind::Untracked.is_untracked());
        for kind in [
            ChangeKind::Added,
            ChangeKind::Modified,
            ChangeKind::Deleted,
            ChangeKind::Renamed,
        ] {
            assert!(!kind.is_untracked());
        }
        // 分类内顺序 = `GIT_STATUS_ORDER`（added / modified / deleted / renamed）。
        assert!(ChangeKind::Added.group_order() < ChangeKind::Modified.group_order());
        assert!(ChangeKind::Modified.group_order() < ChangeKind::Deleted.group_order());
        assert!(ChangeKind::Deleted.group_order() < ChangeKind::Renamed.group_order());
    }

    /// 同路径的索引 / 工作树两条记录合并成一条，且**已暂存的那条胜出**
    /// （`mergeWholePathStatus`，`git-status-model.ts:54-73`）。
    #[test]
    fn same_path_records_coalesce_and_staged_wins() {
        let data = serde_json::json!({
            "changes": [
                { "path": "a.txt", "status": " M", "staged": false, "worktree": true, "untracked": false },
                { "path": "a.txt", "status": "M ", "staged": true, "worktree": false, "untracked": false },
                { "path": "b.txt", "status": "??", "staged": false, "worktree": false, "untracked": true }
            ]
        });
        let rows = parse_changes(&data);
        assert_eq!(rows.len(), 2);
        let a = rows.iter().find(|row| row.path == "a.txt").unwrap();
        assert!(a.staged, "已暂存的那条必须胜出");
        assert_eq!(a.raw_status.as_ref(), "M ");
    }

    /// 排序 = 已跟踪在前 + 分类内 `GIT_STATUS_ORDER` + 路径升序。
    #[test]
    fn rows_sort_by_category_then_status_then_path() {
        let data = serde_json::json!({
            "changes": [
                { "path": "z.txt", "status": "??", "staged": false, "worktree": false, "untracked": true },
                { "path": "m.txt", "status": " M", "staged": false, "worktree": true, "untracked": false },
                { "path": "a.txt", "status": "A ", "staged": true, "worktree": false, "untracked": false },
                { "path": "b.txt", "status": " M", "staged": false, "worktree": true, "untracked": false }
            ]
        });
        let rows = parse_changes(&data);
        let paths: Vec<&str> = rows.iter().map(|row| row.path.as_ref()).collect();
        assert_eq!(paths, vec!["a.txt", "b.txt", "m.txt", "z.txt"]);
    }

    /// rename 的源路径要保留（行内画「旧名 → 新名」需要它）。
    #[test]
    fn rename_keeps_original_path() {
        let data = serde_json::json!({
            "changes": [
                { "path": "new.txt", "originalPath": "old.txt", "status": "R ", "staged": true, "worktree": false, "untracked": false }
            ]
        });
        let rows = parse_changes(&data);
        assert_eq!(rows[0].kind, ChangeKind::Renamed);
        assert_eq!(rows[0].original_path.as_deref(), Some("old.txt"));
    }

    /// `kind` 为空串 = 没有进行中的操作（`contracts.rs:796-797`）；未知 kind 同样不画横幅。
    #[test]
    fn operation_state_is_none_without_an_active_operation() {
        assert!(parse_operation(&serde_json::json!({ "kind": "" })).is_none());
        assert!(parse_operation(&serde_json::json!({ "kind": "bisect" })).is_none());
    }

    /// rebase 的进度与冲突列表要读出来；只有 rebase 能「跳过提交」。
    #[test]
    fn rebase_operation_reads_progress_and_conflicts() {
        let operation = parse_operation(&serde_json::json!({
            "kind": "rebase",
            "reference": "abcdef1234567890",
            "step": 2,
            "total": 5,
            "conflictedPaths": ["src/a.rs"]
        }))
        .expect("rebase 必须被识别");
        assert_eq!(operation.kind, OperationKind::Rebase);
        assert_eq!(operation.step, Some(2));
        assert_eq!(operation.total, Some(5));
        assert!(operation.has_conflicts());
        assert!(operation.kind.can_skip());
        assert!(!OperationKind::Merge.can_skip());
    }

    /// 冲突路径为空 → 可以继续（Core 只在有冲突时拒绝 continue）。
    #[test]
    fn merge_without_conflicts_can_continue() {
        let operation = parse_operation(&serde_json::json!({
            "kind": "merge", "reference": null, "step": null, "total": null, "conflictedPaths": []
        }))
        .expect("merge 必须被识别");
        assert!(!operation.has_conflicts());
        assert_eq!(operation.reference, None);
    }

    /// 空态标签与计数是渲染与诊断共用的判据。
    #[test]
    fn empty_state_labels_follow_the_snapshot() {
        let mut snapshot = super::ChangesSnapshot::default();
        assert_eq!(snapshot.state_label(), "norepo");

        snapshot.repository_root = Some(SharedString::from("/repo"));
        snapshot.has_commits = Some(true);
        assert_eq!(snapshot.state_label(), "clean");

        snapshot.has_commits = Some(false);
        assert_eq!(snapshot.state_label(), "nocommits");

        snapshot.changes = vec![row("a.txt", " M", false), row("b.txt", "??", true)];
        assert_eq!(snapshot.state_label(), "ready");
        assert_eq!(snapshot.tracked_rows().len(), 1);
        assert_eq!(snapshot.untracked_rows().len(), 1);
        // 未跟踪文件**算进**「暂存所有更改」的出现条件（真源 `unstagedFiles` 含未跟踪，
        // 见 [`super::ChangesSnapshot::unstaged_count`]）：一行已跟踪改动 + 一行未跟踪 = 2。
        assert_eq!(snapshot.unstaged_count(), 2);
        assert_eq!(snapshot.staged_count(), 0);
        assert!(snapshot.diagnose().starts_with("S1_SOURCE_CONTROL files=2 staged=0 unstaged=2"));
    }
}
