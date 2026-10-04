//! Git **写操作闭环**的数据层：取回 / 拉取 / 推送 / 检出 + 三条前置检查闸门 + 冲突标记。
//!
//! 规格真源（macOS 前端，逐条 `文件:行`）：
//!
//! | 事实 | 真源 |
//! | --- | --- |
//! | 取回 / 更新项目 / 推送的入口行 | `macos/Sources/Lithe/Views/Git/BranchSwitcherPopover.swift:111-138`（`"Fetch"` / `"Update Project…"` / `"Push…"`） |
//! | 取回对话框（本切片**不做**） | `macos/Sources/Lithe/Views/Git/GitFetchDialog.swift:1-89` |
//! | 检出前查「会不会丢改动」 | `macos/Sources/LitheGitModule/Application/GitFeatureModel.swift:3240-3252`（`checkoutBlockingPaths` → 非空 ⇒ `pendingCheckoutConflict`，**不问就不往下走**） |
//! | 检出确认框的两个出路 | 同上 `:3255-3267`（`.smart` ⇒ `autoStash: true`；`.force` ⇒ `force: true`） |
//! | 拉取前先取回、再看能否快进 | 同上 `:3114-3160`：先 `fetch`（`:3122`，**先取回才不会拿过期 ref 判偏离**），再 `pullPreflight`（`:3129`）；`upstream == nil` ⇒ 停（`:3130-3135`）；已最新 ⇒ 停（`:3136-3141`）；`diverged` ⇒ 问策略（`:3144-3153`）；否则 `--ff-only`。 |
//! | 集成（merge/rebase）前查阻塞 | 同上 `:2885-2905`（`integrationPreflight` → `!isClear` ⇒ `pendingIntegrationConflict`） |
//! | 冲突标记闸门 | 同上 `:147`（`conflictMarkerPaths`）；`git.verifyConflictMarkersFailed` 的原句是「Unable to verify conflict markers. **Retry before committing.**」 |
//!
//! # Core 契约（只读引用，本任务**不改 Core**）
//!
//! | 用途 | 命令 | 请求 | 响应 |
//! | --- | --- | --- | --- |
//! | 取回 | `git.write` | `{ root, operation: "fetch" }`（`rust/lithe-core/src/git/mod.rs:1175-1189`） | `GitCommandResponse` |
//! | 拉取 | `git.write` | `{ root, operation: "pull", mode }`，`mode ∈ {ffOnly, merge, rebase}`（`git/mod.rs:1193-1232`） | 同上 |
//! | 推送 | `git.write` | `{ root, operation: "push", reference }`（`git/mod.rs:1255-1273`） | 同上 |
//! | 检出 | `git.write` | `{ root, operation: "checkout", reference, referenceKind, force, autoStash }`（`git/mod.rs:1274`、`checkout_request_reference` `:3461-3471`） | 同上 |
//! | 检出前置检查 | `git.checkoutPreflight` | `{ root, reference }`（`git/mod.rs:710-716,2174`） | `{ blockingPaths[] }`（`protocol/contracts.rs:775-779`） |
//! | 拉取前置检查 | `git.pullPreflight` | `{ root }`（`git/mod.rs:741-743,2823`） | `{ upstream, ahead, behind, diverged, hasLocalChanges }`（`contracts.rs:807-815`） |
//! | 集成前置检查 | `git.integrationPreflight` | `{ root, reference, operation }`（`git/mod.rs:728-736,2680`） | `{ blockingPaths[], blocksEntirely }`（`contracts.rs:794-799`） |
//! | 冲突标记 | `git.conflictMarkers` | `{ root }`（`git/mod.rs:721-723,2304`） | `{ paths[] }`（`contracts.rs:784-786`） |
//!
//! ## 写字段的合法性（`git/mod.rs:899-919`，逐条对齐）
//!
//! Core 在 `validate_root` **之前**就拒掉三类越界字段组合，本模块把它们前移成**纯函数自查**
//! （[`validate_fields`]），非法组合根本不出门：
//!
//! 1. `fetchOptions` 只对 `fetch` 合法（`:899-904`）；
//! 2. `checkSubmodules` 只对 `push` 合法（`:905-910`）；
//! 3. `expectedCommitState` / `gitlinkUpdates` 只对 `commit` 或 `push` 合法（`:911-919`）。
//!
//! # 本模块的边界
//!
//! 不渲染任何东西（渲染在 `changes_view.rs` / workbench 的 `branch_panel.rs`），
//! 也不另起第二条写网关：所有写都经 [`crate::changes::write`]（`changes.rs:509-564`，
//! 内部就是 `execute_core("git.write", ..)`，`changes.rs:517`）。

use gpui_kit::SharedString;
use serde_json::Value;

use crate::changes::{self, WriteOutcome};
use crate::model::execute_core;

// ---------------------------------------------------------------------------
// 写操作
// ---------------------------------------------------------------------------

/// 本切片经 `git.write` 构造的 operation（`git/mod.rs` 的 `write` 分支）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteOperation {
    /// `git/mod.rs:1175`。
    Fetch,
    /// `git/mod.rs:1193`。
    Pull,
    /// `git/mod.rs:1255`。
    Push,
    /// `git/mod.rs:1274`。
    Checkout,
    /// `git/mod.rs:1275`。
    CheckoutRevision,
}

impl WriteOperation {
    /// Core 认的 operation 名（逐字，大小写敏感）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fetch => "fetch",
            Self::Pull => "pull",
            Self::Push => "push",
            Self::Checkout => "checkout",
            Self::CheckoutRevision => "checkoutRevision",
        }
    }
}

/// 拉取策略（Core `git/mod.rs:1194-1204` 的 `mode`）。
///
/// ⚠️ 缺省（`None`）在 Core 里等价于 `ffOnly`，但**本侧永远显式传**：`ffOnly` 是唯一
/// "绝不凭空造 merge 提交"的那一档（Core 的注释 `:1190-1192` 就是这么写的），
/// 显式传能让"用户没选策略时跑的是哪一条"在请求里看得见。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullStrategy {
    FfOnly,
    Merge,
    Rebase,
}

impl PullStrategy {
    pub fn mode(self) -> &'static str {
        match self {
            Self::FfOnly => "ffOnly",
            Self::Merge => "merge",
            Self::Rebase => "rebase",
        }
    }
}

/// 集成操作（Core `git/mod.rs:2689-2702` 的 `operation`）。#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationOperation {
    Merge,
    Rebase,
    CherryPick,
    Revert,
}

impl IntegrationOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Rebase => "rebase",
            Self::CherryPick => "cherryPick",
            Self::Revert => "revert",
        }
    }
}

/// 旧式检出用的引用类别（Core `git/mod.rs:341-346` 的 `reference_kind`）。
///
/// 刻意不复用 `model::RefKind`：那个类型服务于**读**（`git.references` 的 `kind`），
/// 没有 `Debug`、也不该为了写请求去改它；这里只需要 Core 认的三个字符串。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceKind {
    Local,
    Remote,
    Tag,
}

impl ReferenceKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
            Self::Tag => "tag",
        }
    }
}

/// `git.write` 请求里除 `root` / `operation` 之外的字段。
///
/// 为什么做成一个结构体而不是每个调用点拼 JSON：Core 对其中三个字段有 **operation 约束**
/// （见模块头），只有一个集中点才能在出门前自查（[`validate_fields`]），也才能被单测钉住。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WriteFields {
    pub paths: Vec<String>,
    /// 旧式引用名（`git/mod.rs` 的 `reference`）。
    pub reference: Option<String>,
    /// 检出专用：`local` / `remote` / `tag`（`git/mod.rs:341-346` 的 `reference_kind`）。
    pub reference_kind: Option<ReferenceKind>,
    pub revision: Option<String>,
    /// 策略（拉取的 `mode`、reset 的 `mode`）。
    pub mode: Option<String>,
    pub force: bool,
    pub auto_stash: bool,
    /// 仅 `fetch` 合法（`git/mod.rs:899-904`）。
    pub fetch_options: Option<Value>,
    /// 仅 `push` 合法（`git/mod.rs:905-910`）。
    pub check_submodules: bool,
    /// 仅 `commit` / `push` 合法（`git/mod.rs:911-919`）。
    pub expected_commit_state: Option<Value>,
    /// 仅 `commit` 合法（`git/mod.rs:911-919` 与 `expected_commit_state` 同一条）。
    pub gitlink_updates: Vec<Value>,
}

/// 越界的「字段 × operation」组合。文案**逐字**对齐 Core 的 `invalid_request` 消息
/// （`git/mod.rs:902,908,917`），这样自查拦下的原因与 Core 拦下的原因读起来是同一条。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteFieldError {
    FetchOptionsRequireFetch,
    SubmoduleChecksRequirePush,
    CommitStateRequiresCommitOrPush,
}

impl WriteFieldError {
    pub fn message(self) -> &'static str {
        match self {
            Self::FetchOptionsRequireFetch => "Fetch options require a fetch operation",
            Self::SubmoduleChecksRequirePush => {
                "Submodule publication checks require a push operation"
            }
            Self::CommitStateRequiresCommitOrPush => {
                "Commit state preconditions require a commit or push operation"
            }
        }
    }
}

/// 出门前的字段合法性自查（纯函数，逐条对齐 `git/mod.rs:899-919`）。
///
/// 第 3 条逐字照抄 Core 的表达式：
/// `operation != "commit" && (!gitlinkUpdates.is_empty() || (expectedCommitState.is_some() && operation != "push"))`。
/// 本切片的 operation 集里**没有** `commit`，所以前半句恒真 —— 结果是
/// **`gitlinkUpdates` 在本模块里永远被拒**（Core 也只允许 `commit` 带它），
/// 而 `expectedCommitState` 只允许 `push`。这是等价缩减，不是放宽。
pub fn validate_fields(
    operation: WriteOperation,
    fields: &WriteFields,
) -> Result<(), WriteFieldError> {
    if fields.fetch_options.is_some() && operation != WriteOperation::Fetch {
        return Err(WriteFieldError::FetchOptionsRequireFetch);
    }
    if fields.check_submodules && operation != WriteOperation::Push {
        return Err(WriteFieldError::SubmoduleChecksRequirePush);
    }
    if !fields.gitlink_updates.is_empty()
        || (fields.expected_commit_state.is_some() && operation != WriteOperation::Push)
    {
        return Err(WriteFieldError::CommitStateRequiresCommitOrPush);
    }
    Ok(())
}

impl WriteFields {
    /// 把非缺省字段摊成 `git.write` 的 payload（`camelCase`，与 Core 的 serde 改名一致）。
    ///
    /// 只写"确实设了值"的键：布尔为 `false`、列表为空、`Option` 为 `None` 一律**不出现在
    /// 请求里**，与 macOS 侧 `Encodable` 的可选字段行为一致，也让"这条请求到底带了什么"
    /// 在日志里一眼可读。
    pub fn to_payload(&self) -> Value {
        let mut payload = serde_json::Map::new();
        if !self.paths.is_empty() {
            payload.insert("paths".into(), serde_json::json!(self.paths));
        }
        if let Some(reference) = &self.reference {
            payload.insert("reference".into(), serde_json::json!(reference));
        }
        if let Some(kind) = self.reference_kind {
            payload.insert("referenceKind".into(), serde_json::json!(kind.id()));
        }
        if let Some(revision) = &self.revision {
            payload.insert("revision".into(), serde_json::json!(revision));
        }
        if let Some(mode) = &self.mode {
            payload.insert("mode".into(), serde_json::json!(mode));
        }
        if self.force {
            payload.insert("force".into(), serde_json::json!(true));
        }
        if self.auto_stash {
            payload.insert("autoStash".into(), serde_json::json!(true));
        }
        if let Some(options) = &self.fetch_options {
            payload.insert("fetchOptions".into(), options.clone());
        }
        if self.check_submodules {
            payload.insert("checkSubmodules".into(), serde_json::json!(true));
        }
        if let Some(state) = &self.expected_commit_state {
            payload.insert("expectedCommitState".into(), state.clone());
        }
        if !self.gitlink_updates.is_empty() {
            payload.insert(
                "gitlinkUpdates".into(),
                serde_json::json!(self.gitlink_updates),
            );
        }
        Value::Object(payload)
    }
}

/// 跑一次 `git.write`：先自查字段合法性，再交给**唯一**的写网关 [`changes::write`]。
///
/// 自查失败时**不发出请求**，直接把它当成一次用户可见的失败返回 —— 与 Core 拒绝时的
/// 表现一致（都进 `WriteOutcome::failure`，由视图画红条），但少一次无用的进程往返。
pub fn execute(root: &str, operation: WriteOperation, fields: WriteFields) -> WriteOutcome {
    match validate_fields(operation, &fields) {
        Ok(()) => changes::write(operation.as_str(), root, fields.to_payload()),
        Err(error) => WriteOutcome {
            failure: Some(error.message().to_string()),
        },
    }
}

/// 取回：`git.write { operation: "fetch" }`（真源 `BranchSwitcherPopover.swift:112-115`）。
pub fn fetch(root: &str) -> WriteOutcome {
    execute(root, WriteOperation::Fetch, WriteFields::default())
}

/// 拉取：`git.write { operation: "pull", mode }`（真源 `RustGitOperations.swift:311-313`）。
pub fn pull(root: &str, strategy: PullStrategy) -> WriteOutcome {
    execute(
        root,
        WriteOperation::Pull,
        WriteFields {
            mode: Some(strategy.mode().to_string()),
            ..WriteFields::default()
        },
    )
}

/// 推送一个本地分支：`git.write { operation: "push", reference }`
/// （真源 `RustGitOperations.swift:414-416`）。
///
/// `reference` 为 `None` 时 Core 走 Git 自己的推送目标解析（`push.default` / 上游）；
/// 本侧总是带上当前分支名，让"推的是哪一个分支"在请求里明确。
pub fn push(root: &str, reference: Option<&str>) -> WriteOutcome {
    execute(
        root,
        WriteOperation::Push,
        WriteFields {
            reference: reference.map(str::to_string),
            ..WriteFields::default()
        },
    )
}

/// 检出本地分支：`git.write { operation: "checkout", reference, referenceKind: "local", force, autoStash }`。
///
/// `force` / `auto_stash` 就是真源检出确认框的两个出路
/// （`GitFeatureModel.swift:3262-3266`：`.smart` ⇒ `autoStash`，`.force` ⇒ `force`）。
pub fn checkout(root: &str, branch: &str, force: bool, auto_stash: bool) -> WriteOutcome {
    execute(
        root,
        WriteOperation::Checkout,
        WriteFields {
            reference: Some(branch.to_string()),
            reference_kind: Some(ReferenceKind::Local),
            force,
            auto_stash,
            ..WriteFields::default()
        },
    )
}

// ---------------------------------------------------------------------------
// 四态
// ---------------------------------------------------------------------------

/// 读操作的**四态**：`idle / loading / ready / failed`。
///
/// 为什么必须四态而不是 `Option`：硬约束要求"拿不到前置检查结果"与"一切正常"**在类型上
/// 就分得开**。`Option::None` 会把"还没读"「正在读」「读失败了」混成一档，视图就只能猜，
/// 最后不是假装没事就是永远转圈。
#[derive(Clone, Debug, PartialEq)]
pub enum Probe<T> {
    /// 还没有发起读取。
    Idle,
    /// 读取在途。
    Loading,
    /// 拿到了结果（**不代表放行**，判定在 `*_gate` 里）。
    Ready(T),
    /// 读取失败（Core 错误码 / 信封失败原文）。
    Failed(SharedString),
}

impl<T> Probe<T> {
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    /// 只有 `Ready` 才给值。**`Failed` 不给**：调用方据此必须走进"如实呈现失败"那一支。
    pub fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            _ => None,
        }
    }

    /// `Failed` 的原文（没有失败时为 `None`）。
    pub fn failure(&self) -> Option<&SharedString> {
        match self {
            Self::Failed(reason) => Some(reason),
            _ => None,
        }
    }
}

/// 一次前置检查的**分代读取器**：`begin` → 回包 → `apply`。
///
/// 分代是必要的而不是装饰：用户连点两次"更新项目"时，第一次的回包可能晚于第二次到达，
/// 直接落地就会用**过期**的 `pullPreflight` 决定要不要弹策略框。`apply` 用代次把这种
/// 晚到的旧回包整份丢掉（宿主既有口径，见 `changes_view.rs` 的 `request_serial`）。
#[derive(Clone, Debug, PartialEq)]
pub struct Preflight<T> {
    serial: u64,
    state: Probe<T>,
}

impl<T> Default for Preflight<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Preflight<T> {
    pub fn new() -> Self {
        Self {
            serial: 0,
            state: Probe::Idle,
        }
    }

    /// 发起一次读取：分配并返回本次代次，状态进 `Loading`。
    pub fn begin(&mut self) -> u64 {
        self.serial = self.serial.wrapping_add(1);
        self.state = Probe::Loading;
        self.serial
    }

    /// 回包落地。返回 `false` 表示**这次回包被丢弃**（代次不匹配，晚到的旧回包）——
    /// 丢弃时状态**一点不动**，否则旧回包会把新一次的 `Loading` 打回 `Ready`。
    pub fn apply(&mut self, serial: u64, result: Result<T, String>) -> bool {
        if serial != self.serial {
            return false;
        }
        self.state = match result {
            Ok(value) => Probe::Ready(value),
            // 失败**不假装 ready**：进 `Failed` 并带上原文，由闸门判成 `Unavailable`。
            Err(reason) => Probe::Failed(SharedString::from(reason)),
        };
        true
    }

    pub fn state(&self) -> &Probe<T> {
        &self.state
    }

    /// 回到 `idle`（换仓库 / 关闭面板时用；不清代次，代次只增不减）。
    pub fn reset(&mut self) {
        self.state = Probe::Idle;
    }
}

// ---------------------------------------------------------------------------
// 前置检查的响应
// ---------------------------------------------------------------------------

/// `git.checkoutPreflight` 的响应（`contracts.rs:775-779`）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckoutPreflight {
    /// 工作树里有改动、**且**会在 HEAD 与目标引用之间有差异的路径 —— 检出会丢它们。
    pub blocking_paths: Vec<SharedString>,
}

/// `git.integrationPreflight` 的响应（`contracts.rs:794-799`）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IntegrationPreflight {
    /// merge = 与将要写入的文件重叠的脏路径；rebase = **全部**脏路径。
    pub blocking_paths: Vec<SharedString>,
    /// `true` = 任何未提交改动都拦（rebase 的形态，`git/mod.rs:2734-2744`）。
    pub blocks_entirely: bool,
}

/// `git.pullPreflight` 的响应（`contracts.rs:807-815`）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PullPreflight {
    /// 上游短名；`None` = 这个分支没有上游，**这本身就是停下来的理由**。
    pub upstream: Option<SharedString>,
    pub ahead: usize,
    pub behind: usize,
    /// 两边都有对方没有的提交 ⇒ `--ff-only` 会拒绝，必须先问策略。
    pub diverged: bool,
    pub has_local_changes: bool,
}

/// 解析 `git.checkoutPreflight` 的 `data`（纯函数）。
pub fn parse_checkout_preflight(data: &Value) -> CheckoutPreflight {
    CheckoutPreflight {
        blocking_paths: string_list(data, "blockingPaths"),
    }
}

/// 解析 `git.integrationPreflight` 的 `data`（纯函数）。
pub fn parse_integration_preflight(data: &Value) -> IntegrationPreflight {
    IntegrationPreflight {
        blocking_paths: string_list(data, "blockingPaths"),
        blocks_entirely: data
            .get("blocksEntirely")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

/// 解析 `git.pullPreflight` 的 `data`（纯函数）。
///
/// `upstream` **缺字段与 `null` 同义**（都是"没有上游"）：Core 在两种情况下都可能不写这个键
/// （`git/mod.rs:2840-2846` 的早退与 serde 的 `Option`），所以不能用"键不存在 ⇒ 解析失败"
/// 的严格口径，否则会把正常的"没有上游"报成读取失败。
pub fn parse_pull_preflight(data: &Value) -> PullPreflight {
    PullPreflight {
        upstream: data
            .get("upstream")
            .and_then(Value::as_str)
            .map(SharedString::from),
        ahead: count_of(data, "ahead"),
        behind: count_of(data, "behind"),
        diverged: data
            .get("diverged")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        has_local_changes: data
            .get("hasLocalChanges")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

/// 解析 `git.conflictMarkers` 的 `data`（纯函数）。**已排序去重**，保证投影确定性。
pub fn parse_conflict_markers(data: &Value) -> Vec<SharedString> {
    let mut paths = string_list(data, "paths");
    paths.sort();
    paths.dedup();
    paths
}

fn string_list(data: &Value, key: &str) -> Vec<SharedString> {
    data.get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(SharedString::from)
                .collect()
        })
        .unwrap_or_default()
}

fn count_of(data: &Value, key: &str) -> usize {
    data.get(key)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// 前置检查的读取
// ---------------------------------------------------------------------------

/// `git.checkoutPreflight`（`git/mod.rs:2174-2295`）。
pub fn checkout_preflight(root: &str, reference: &str) -> Result<CheckoutPreflight, String> {
    let data = execute_core(
        "git.checkoutPreflight",
        serde_json::json!({ "root": root, "reference": reference }),
    )?;
    Ok(parse_checkout_preflight(&data))
}

/// `git.pullPreflight`（`git/mod.rs:2823-2894`）。
pub fn pull_preflight(root: &str) -> Result<PullPreflight, String> {
    let data = execute_core("git.pullPreflight", serde_json::json!({ "root": root }))?;
    Ok(parse_pull_preflight(&data))
}

/// `git.integrationPreflight`（`git/mod.rs:2680-2815`）。
pub fn integration_preflight(
    root: &str,
    reference: &str,
    operation: IntegrationOperation,
) -> Result<IntegrationPreflight, String> {
    let data = execute_core(
        "git.integrationPreflight",
        serde_json::json!({
            "root": root,
            "reference": reference,
            "operation": operation.as_str(),
        }),
    )?;
    Ok(parse_integration_preflight(&data))
}

/// `git.conflictMarkers`（`git/mod.rs:2297-2308`）：**暂存区**里仍含冲突标记的文件。
pub fn conflict_markers(root: &str) -> Result<Vec<SharedString>, String> {
    let data = execute_core("git.conflictMarkers", serde_json::json!({ "root": root }))?;
    Ok(parse_conflict_markers(&data))
}

// ---------------------------------------------------------------------------
// 闸门（纯函数）
// ---------------------------------------------------------------------------

/// 检出闸门的判定。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckoutGate {
    /// 干净 ⇒ 直接检出。
    Proceed,
    /// 会丢改动 ⇒ **必须问**（真源 `GitCheckoutConflictDialog`，`GitFeatureModel.swift:3245-3251`）。
    Confirm { blocking_paths: Vec<SharedString> },
    /// 结论还没出来 ⇒ **不许往下走**（用户什么都还没确认）。
    Wait,
    /// 拿不到结论 ⇒ 也不许往下走，但原因必须与"干净"区分开。
    Unavailable { reason: SharedString },
}

/// 检出的闸门判定（纯函数）。
pub fn checkout_gate(probe: &Probe<CheckoutPreflight>) -> CheckoutGate {
    match probe {
        Probe::Idle | Probe::Loading => CheckoutGate::Wait,
        Probe::Failed(reason) => CheckoutGate::Unavailable {
            reason: reason.clone(),
        },
        Probe::Ready(preflight) if preflight.blocking_paths.is_empty() => CheckoutGate::Proceed,
        Probe::Ready(preflight) => CheckoutGate::Confirm {
            blocking_paths: preflight.blocking_paths.clone(),
        },
    }
}

/// 拉取闸门停下时的原因（每一档对应一条**可行动**的既有文案键）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PullStop {
    /// 没有上游（`git.pullResult.noUpstream`）。
    NoUpstream,
    /// 已是最新，没什么可拉的（`git.pullResult.upToDate`）。
    UpToDate,
}

impl PullStop {
    pub fn message_key(self) -> &'static str {
        match self {
            Self::NoUpstream => "lithe.git.pullResult.noUpstream",
            Self::UpToDate => "lithe.git.pullResult.upToDate",
        }
    }
}

/// 拉取闸门的判定。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PullGate {
    /// 可以 `--ff-only`。
    Proceed,
    /// 历史偏离 ⇒ **该问就问**：让用户选 merge / rebase（真源 `pendingPullStrategy`）。
    AskStrategy {
        upstream: SharedString,
        ahead: usize,
        behind: usize,
        has_local_changes: bool,
    },
    /// 没有可拉的 / 拉不了 ⇒ 停，并把原因说清楚。
    Stop(PullStop),
    Wait,
    Unavailable {
        reason: SharedString,
    },
}

/// 拉取的闸门判定（纯函数）。
///
/// ⚠️ **顺序即真源顺序**（`GitFeatureModel.swift:3130-3153`）：先判有没有上游，
/// 再判"已最新"，最后才判偏离 —— 偏离的分支如果同时已经是最新的（不可能），
/// 也应当先给出"已最新"这句更有用的话。
pub fn pull_gate(probe: &Probe<PullPreflight>) -> PullGate {
    match probe {
        Probe::Idle | Probe::Loading => PullGate::Wait,
        Probe::Failed(reason) => PullGate::Unavailable {
            reason: reason.clone(),
        },
        Probe::Ready(preflight) => match &preflight.upstream {
            None => PullGate::Stop(PullStop::NoUpstream),
            // 快进即可满足（没有落后就没有东西可拉）。
            Some(_) if preflight.behind == 0 && !preflight.diverged => {
                PullGate::Stop(PullStop::UpToDate)
            }
            Some(upstream) if preflight.diverged => PullGate::AskStrategy {
                upstream: upstream.clone(),
                ahead: preflight.ahead,
                behind: preflight.behind,
                has_local_changes: preflight.has_local_changes,
            },
            Some(_) => PullGate::Proceed,
        },
    }
}

/// 集成闸门的判定。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegrationGate {
    Proceed,
    /// 有阻塞 ⇒ 停下来问（`blocks_entirely` 决定解释的口气：
    /// `true` = 任何未提交改动都拦，`false` = 只有重叠的文件拦）。
    Confirm {
        blocking_paths: Vec<SharedString>,
        blocks_entirely: bool,
    },
    Wait,
    Unavailable {
        reason: SharedString,
    },
}

/// 集成的闸门判定（纯函数）。
pub fn integration_gate(probe: &Probe<IntegrationPreflight>) -> IntegrationGate {
    match probe {
        Probe::Idle | Probe::Loading => IntegrationGate::Wait,
        Probe::Failed(reason) => IntegrationGate::Unavailable {
            reason: reason.clone(),
        },
        Probe::Ready(preflight) if preflight.blocking_paths.is_empty() => IntegrationGate::Proceed,
        Probe::Ready(preflight) => IntegrationGate::Confirm {
            blocking_paths: preflight.blocking_paths.clone(),
            blocks_entirely: preflight.blocks_entirely,
        },
    }
}

/// 冲突标记闸门的判定。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictGate {
    /// 暂存区里没有残留的冲突标记 ⇒ 可以继续。
    Clear,
    /// 有残留 ⇒ **拦**，并把文件列出来（`git.conflictMarkersRemain`）。
    Present {
        paths: Vec<SharedString>,
    },
    Wait,
    /// 拿不到结论 ⇒ **也拦**，但走另一句文案
    /// （`git.verifyConflictMarkersFailed` = "…Retry before committing."）。
    Unavailable {
        reason: SharedString,
    },
}

/// 冲突标记的闸门判定（纯函数）。
pub fn conflict_gate(probe: &Probe<Vec<SharedString>>) -> ConflictGate {
    match probe {
        Probe::Idle | Probe::Loading => ConflictGate::Wait,
        Probe::Failed(reason) => ConflictGate::Unavailable {
            reason: reason.clone(),
        },
        Probe::Ready(paths) if paths.is_empty() => ConflictGate::Clear,
        Probe::Ready(paths) => ConflictGate::Present {
            paths: paths.clone(),
        },
    }
}

/// 把一串路径列表折成一行给人看的文本（上限 `limit`，超出加省略号）。
///
/// 与 `git.uncommittedWouldOverwrite` 的 `{listed}{more}` 两个占位符对应（`{more}` 由
/// 调用方补），也是本模块唯一的"投影成界面文案"的纯函数，所以单独测。
pub fn join_paths(paths: &[SharedString], limit: usize) -> (String, usize) {
    let listed = paths
        .iter()
        .take(limit)
        .map(SharedString::as_ref)
        .collect::<Vec<_>>()
        .join(", ");
    (listed, paths.len().saturating_sub(limit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn path(value: &str) -> SharedString {
        SharedString::from(value)
    }

    // ---- 写字段合法性（Core `git/mod.rs:899-919` 的镜像）----

    /// 取回选项只对取回合法：`pull` 带上它必须先被自查拦下（Core 的 `invalid_request` 前置）。
    #[test]
    fn fetch_options_are_rejected_outside_fetch() {
        let fields = WriteFields {
            fetch_options: Some(json!({ "remote": "origin" })),
            ..WriteFields::default()
        };
        assert_eq!(
            validate_fields(WriteOperation::Pull, &fields),
            Err(WriteFieldError::FetchOptionsRequireFetch)
        );
        assert_eq!(validate_fields(WriteOperation::Fetch, &fields), Ok(()));
    }

    /// 子模块检查只对推送合法。
    #[test]
    fn submodule_checks_are_rejected_outside_push() {
        let fields = WriteFields {
            check_submodules: true,
            ..WriteFields::default()
        };
        assert_eq!(
            validate_fields(WriteOperation::Fetch, &fields),
            Err(WriteFieldError::SubmoduleChecksRequirePush)
        );
        assert_eq!(validate_fields(WriteOperation::Push, &fields), Ok(()));
    }

    /// 提交状态前置只对推送合法（本切片没有 commit operation）。
    #[test]
    fn commit_state_preconditions_are_rejected_outside_push() {
        let expected = WriteFields {
            expected_commit_state: Some(json!({ "head": "abc" })),
            ..WriteFields::default()
        };
        assert_eq!(
            validate_fields(WriteOperation::Checkout, &expected),
            Err(WriteFieldError::CommitStateRequiresCommitOrPush)
        );
        assert_eq!(validate_fields(WriteOperation::Push, &expected), Ok(()));

        let gitlinks = WriteFields {
            gitlink_updates: vec![json!({ "path": "sub" })],
            ..WriteFields::default()
        };
        // Core 只允许 `commit` 带 `gitlinkUpdates`（`git/mod.rs:911-913`），
        // 而本切片没有 commit operation ⇒ **连 push 也要拒**（同一条表达式）。
        assert_eq!(
            validate_fields(WriteOperation::Push, &gitlinks),
            Err(WriteFieldError::CommitStateRequiresCommitOrPush)
        );
        assert_eq!(
            validate_fields(WriteOperation::Fetch, &gitlinks),
            Err(WriteFieldError::CommitStateRequiresCommitOrPush)
        );
    }

    /// 缺省字段**一个键都不出门**：请求里不应出现 `false` / `[]` / `null` 这类噪声。
    #[test]
    fn default_fields_produce_an_empty_payload() {
        let payload = WriteFields::default().to_payload();
        assert_eq!(payload, json!({}));
    }

    /// 检出的请求形状：`referenceKind` 是 `local`，`force` / `autoStash` 只在为真时出现。
    #[test]
    fn checkout_fields_serialize_with_the_local_kind() {
        let fields = WriteFields {
            reference: Some("main".into()),
            reference_kind: Some(ReferenceKind::Local),
            force: true,
            auto_stash: false,
            ..WriteFields::default()
        };
        assert_eq!(
            fields.to_payload(),
            json!({ "reference": "main", "referenceKind": "local", "force": true })
        );
    }

    /// 拉取策略逐条对 Core 的 `mode` 取值（`git/mod.rs:1194-1204`）。
    #[test]
    fn pull_strategies_use_the_core_modes() {
        assert_eq!(PullStrategy::FfOnly.mode(), "ffOnly");
        assert_eq!(PullStrategy::Merge.mode(), "merge");
        assert_eq!(PullStrategy::Rebase.mode(), "rebase");
    }

    /// operation 名逐字对 Core 的匹配臂。
    #[test]
    fn write_operation_names_match_core() {
        assert_eq!(WriteOperation::Fetch.as_str(), "fetch");
        assert_eq!(WriteOperation::Pull.as_str(), "pull");
        assert_eq!(WriteOperation::Push.as_str(), "push");
        assert_eq!(WriteOperation::Checkout.as_str(), "checkout");
        assert_eq!(
            WriteOperation::CheckoutRevision.as_str(),
            "checkoutRevision"
        );
        assert_eq!(IntegrationOperation::CherryPick.as_str(), "cherryPick");
    }

    // ---- 纯函数自查的失败通路也会被翻成用户可见失败（不给"静默 return"留口子）----

    /// 自查失败时 `execute` 不调 Core，但**必须**返回一句可见的失败原因。
    #[test]
    fn a_rejected_field_combination_becomes_a_visible_failure() {
        let outcome = execute(
            // root 是个不存在的路径：这条测试要证明的是"根本没走到 Core"，
            // 所以若它真的发请求，错误码会是文件系统/仓库类而不是这句自查文案。
            "/lithe-u11-does-not-exist",
            WriteOperation::Fetch,
            WriteFields {
                check_submodules: true,
                ..WriteFields::default()
            },
        );
        assert_eq!(
            outcome.failure.as_deref(),
            Some(WriteFieldError::SubmoduleChecksRequirePush.message())
        );
    }

    // ---- 解析 ----

    /// 缺字段 / 空数组 / 类型不对都退化到"空 + 缺省"，**不 panic**。
    #[test]
    fn parsers_tolerate_missing_and_mistyped_fields() {
        assert_eq!(
            parse_checkout_preflight(&json!({})),
            CheckoutPreflight::default()
        );
        assert_eq!(
            parse_checkout_preflight(&json!({ "blockingPaths": "not-an-array" })),
            CheckoutPreflight::default()
        );
        assert_eq!(
            parse_integration_preflight(&json!({ "blockingPaths": ["a"], "blocksEntirely": 1 })),
            IntegrationPreflight {
                blocking_paths: vec![path("a")],
                blocks_entirely: false,
            }
        );
        assert_eq!(
            parse_pull_preflight(&json!({ "ahead": "two", "behind": null })),
            PullPreflight::default()
        );
        assert!(parse_conflict_markers(&json!({ "paths": null })).is_empty());
    }

    /// 拉取前置检查：`upstream` 缺键与 `null` 同义（都是"没有上游"），不能变成读取失败。
    #[test]
    fn an_absent_upstream_is_a_missing_upstream_not_a_failure() {
        let absent = parse_pull_preflight(&json!({ "ahead": 0, "behind": 0, "diverged": false }));
        let null = parse_pull_preflight(&json!({ "upstream": null }));
        assert_eq!(absent, null);
        assert_eq!(absent.upstream, None);
    }

    /// 冲突路径列表是**确定性**的（排序 + 去重），因为 Core 的 stderr 顺序不可依赖。
    #[test]
    fn conflict_paths_are_sorted_and_deduplicated() {
        let parsed = parse_conflict_markers(&json!({ "paths": ["b.txt", "a.txt", "b.txt"] }));
        assert_eq!(parsed, vec![path("a.txt"), path("b.txt")]);
    }

    // ---- 分代读取器 ----

    /// 晚到的旧回包必须被丢掉，且**不能把新一次的 `Loading` 打回 `Ready`**。
    #[test]
    fn a_stale_reply_is_dropped_without_disturbing_the_new_read() {
        let mut probe: Preflight<PullPreflight> = Preflight::new();
        let stale = probe.begin();
        assert!(probe.state().is_loading());

        let fresh = probe.begin();
        assert!(probe.state().is_loading());

        // 旧回包：丢弃。
        assert!(!probe.apply(stale, Ok(PullPreflight::default())));
        assert!(
            probe.state().is_loading(),
            "旧回包不该把状态改成 Ready：{:?}",
            probe.state()
        );

        // 新回包：落地。
        let value = PullPreflight {
            upstream: Some(path("origin/main")),
            behind: 2,
            ..PullPreflight::default()
        };
        assert!(probe.apply(fresh, Ok(value.clone())));
        assert_eq!(probe.state().ready(), Some(&value));
    }

    /// 失败进 `Failed`，**不假装 ready**；`Ready` 与 `Failed` 在类型上分得开。
    #[test]
    fn a_failed_read_is_not_a_ready_read() {
        let mut probe: Preflight<CheckoutPreflight> = Preflight::new();
        let serial = probe.begin();
        assert!(probe.apply(serial, Err("process_failed".into())));
        assert!(probe.state().is_failed());
        assert_eq!(probe.state().ready(), None);
        assert_eq!(
            probe.state().failure().map(SharedString::as_ref),
            Some("process_failed")
        );
    }

    /// `idle` 与 `loading` 都不是 `ready`，也不是 `failed`。
    #[test]
    fn idle_and_loading_are_their_own_states() {
        let mut probe: Preflight<Vec<SharedString>> = Preflight::new();
        assert!(probe.state().is_idle());
        probe.begin();
        assert!(probe.state().is_loading());
        probe.reset();
        assert!(probe.state().is_idle());
    }

    // ---- 闸门 ----

    /// 检出：干净放行、有阻塞要问、失败要如实拦、没结论要等。
    #[test]
    fn checkout_gate_covers_all_four_outcomes() {
        assert_eq!(checkout_gate(&Probe::Idle), CheckoutGate::Wait);
        assert_eq!(checkout_gate(&Probe::Loading), CheckoutGate::Wait);
        assert_eq!(
            checkout_gate(&Probe::Failed(path("process_failed"))),
            CheckoutGate::Unavailable {
                reason: path("process_failed")
            }
        );
        assert_eq!(
            checkout_gate(&Probe::Ready(CheckoutPreflight::default())),
            CheckoutGate::Proceed
        );
        assert_eq!(
            checkout_gate(&Probe::Ready(CheckoutPreflight {
                blocking_paths: vec![path("a.rs")],
            })),
            CheckoutGate::Confirm {
                blocking_paths: vec![path("a.rs")]
            }
        );
    }

    /// 拉取：没有上游 ⇒ 停；已最新 ⇒ 停；偏离 ⇒ 问策略；落后不偏离 ⇒ 直接快进。
    #[test]
    fn pull_gate_follows_the_source_order() {
        assert_eq!(
            pull_gate(&Probe::Ready(PullPreflight::default())),
            PullGate::Stop(PullStop::NoUpstream)
        );
        assert_eq!(
            pull_gate(&Probe::Ready(PullPreflight {
                upstream: Some(path("origin/main")),
                ..PullPreflight::default()
            })),
            PullGate::Stop(PullStop::UpToDate)
        );
        assert_eq!(
            pull_gate(&Probe::Ready(PullPreflight {
                upstream: Some(path("origin/main")),
                behind: 3,
                ..PullPreflight::default()
            })),
            PullGate::Proceed
        );
        assert_eq!(
            pull_gate(&Probe::Ready(PullPreflight {
                upstream: Some(path("origin/main")),
                ahead: 1,
                behind: 3,
                diverged: true,
                has_local_changes: true,
            })),
            PullGate::AskStrategy {
                upstream: path("origin/main"),
                ahead: 1,
                behind: 3,
                has_local_changes: true,
            }
        );
    }

    /// 拉取闸门拿不到结论时**既不停成 `Stop` 也不放行**。
    #[test]
    fn pull_gate_never_pretends_success_when_the_read_failed() {
        assert_eq!(pull_gate(&Probe::Idle), PullGate::Wait);
        assert_eq!(pull_gate(&Probe::Loading), PullGate::Wait);
        assert_eq!(
            pull_gate(&Probe::Failed(path("timed_out"))),
            PullGate::Unavailable {
                reason: path("timed_out")
            }
        );
    }

    /// 集成：`blocksEntirely` 是给用户看的"为什么全被拦"，不改变"拦不拦"。
    #[test]
    fn integration_gate_reports_the_shape_of_the_block() {
        assert_eq!(
            integration_gate(&Probe::Ready(IntegrationPreflight::default())),
            IntegrationGate::Proceed
        );
        assert_eq!(
            integration_gate(&Probe::Ready(IntegrationPreflight {
                blocking_paths: vec![path("a.rs"), path("b.rs")],
                blocks_entirely: true,
            })),
            IntegrationGate::Confirm {
                blocking_paths: vec![path("a.rs"), path("b.rs")],
                blocks_entirely: true,
            }
        );
        assert_eq!(
            integration_gate(&Probe::Failed(path("process_failed"))),
            IntegrationGate::Unavailable {
                reason: path("process_failed")
            }
        );
    }

    /// 冲突标记：空 ⇒ 放行；非空 ⇒ 拦并列出；失败 ⇒ 也拦（另一句文案）。
    #[test]
    fn conflict_gate_blocks_on_markers_and_on_unknown() {
        assert_eq!(
            conflict_gate(&Probe::Ready(Vec::new())),
            ConflictGate::Clear
        );
        assert_eq!(
            conflict_gate(&Probe::Ready(vec![path("a.rs")])),
            ConflictGate::Present {
                paths: vec![path("a.rs")]
            }
        );
        assert_eq!(conflict_gate(&Probe::Idle), ConflictGate::Wait);
        assert_eq!(
            conflict_gate(&Probe::Failed(path("parse_failed"))),
            ConflictGate::Unavailable {
                reason: path("parse_failed")
            }
        );
    }

    /// `join_paths` 的 `{listed}` / `{more}` 两个投影量（句子的省略号由文案负责）。
    #[test]
    fn join_paths_reports_the_hidden_count() {
        let paths = vec![path("a"), path("b"), path("c")];
        assert_eq!(join_paths(&paths, 2), ("a, b".to_string(), 1));
        assert_eq!(join_paths(&paths, 9), ("a, b, c".to_string(), 0));
        assert_eq!(join_paths(&[], 3), (String::new(), 0));
    }

    // ---- 真实 Core 往返（不依赖网络 / 真实仓库 / 时钟）----

    /// **真实 Core 往返**：把字段合法性自查的结论与 Core 的结论对一遍。
    ///
    /// 为什么必须有这一条：上面几条只钉住"宿主自查会拦什么"，钉不住"Core 认的是不是同一组
    /// 约束"。Core 的这三条检查在 `validate_root` **之前**（`git/mod.rs:899-920`），所以拿一个
    /// 不存在的 root 也能拿到同一句拒绝 —— 这正是本测试不需要真实仓库的原因。
    ///
    /// 断言的是 **Core 的原文**：自查的文案与它逐字相同，所以"自查拦下"与"Core 拦下"
    /// 给用户看到的是同一句话。
    #[test]
    fn the_field_rules_match_core_over_the_real_envelope() {
        let root = "/lithe-u11-field-rules";

        // ① fetchOptions 只对 fetch：Core 在 validate_root 之前就拒。
        let rejected = execute_core(
            "git.write",
            json!({ "root": root, "operation": "pull", "fetchOptions": { "remote": "origin" } }),
        )
        .expect_err("Core 必须拒掉 fetch 之外的 fetchOptions");
        assert_eq!(rejected, "invalid_request");

        // ② checkSubmodules 只对 push。
        let rejected = execute_core(
            "git.write",
            json!({ "root": root, "operation": "fetch", "checkSubmodules": true }),
        )
        .expect_err("Core 必须拒掉 push 之外的 checkSubmodules");
        assert_eq!(rejected, "invalid_request");

        // ③ expectedCommitState 只对 commit / push。
        let rejected = execute_core(
            "git.write",
            json!({ "root": root, "operation": "checkout", "expectedCommitState": { "head": "0" } }),
        )
        .expect_err("Core 必须拒掉 commit/push 之外的 expectedCommitState");
        assert_eq!(rejected, "invalid_request");

        // ④ 合法的取回组合不会被这三条规则拦下：它继续走到 validate_root 并因根不存在而失败。
        //    这一半同样重要 —— 只测"会拒"不测"不误拒"，把约束写成"一律拒"也能过。
        let deeper = execute_core(
            "git.write",
            json!({ "root": root, "operation": "fetch", "fetchOptions": { "remote": "origin" } }),
        )
        .expect_err("这个根不存在，取回不该成功");
        assert_ne!(
            deeper, "invalid_request",
            "合法的取回组合不该被字段约束拦下"
        );
    }

    /// **真实 Core 往返**：前置检查的命令名与 payload 形状真的被 Core 收下。
    ///
    /// 判据是"不是 `invalid_request`"：`invalid_request` 表示命令名或 payload 形状不对
    /// （这正是本测试要防的），而根不是仓库会让 Core 在更深处以 `process_failed` 之类失败。
    #[test]
    fn the_preflight_commands_are_accepted_by_core() {
        let root = "/lithe-u11-preflight-shape";

        let checkout = checkout_preflight(root, "main").expect_err("根不存在，不该成功");
        assert_ne!(
            checkout, "invalid_request",
            "checkoutPreflight 的请求形状被拒了"
        );

        let pull = pull_preflight(root).expect_err("根不存在，不该成功");
        assert_ne!(pull, "invalid_request", "pullPreflight 的请求形状被拒了");

        let integration = integration_preflight(root, "main", IntegrationOperation::Rebase)
            .expect_err("根不存在，不该成功");
        assert_ne!(
            integration, "invalid_request",
            "integrationPreflight 的请求形状被拒了"
        );

        let markers = conflict_markers(root).expect_err("根不存在，不该成功");
        assert_ne!(
            markers, "invalid_request",
            "conflictMarkers 的请求形状被拒了"
        );
    }
}
