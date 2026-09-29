//! 标题栏「分支弹窗」的**只读数据层**（`git.status` + `git.references`）。
//!
//! 规格真源：`lithe-gpui/research/windows/12-branch-manager.md`（下称「研究」，561 行，逐条带
//! `文件:行号`）对应的 Windows 前端
//! `windows/tauri/src/features/git/components/git-branch-manager.tsx`（1050 行）。
//!
//! # 为什么这一段放在 `lithe-gpui-git` 而不是工作台
//!
//! 工作台（`lithe-gpui-workbench`）要画的是**标题栏那一项与浮层面板**，属于窗口 chrome；
//! 而"分支名 / 当前分支 / ahead-behind / 本地分支列表"是 Git Feature 的事实。研究 §6.2
//! 把这两件事分开写：面板结构与组件选型归 workbench，数据只经 Core 命令来。本模块就是那条
//! 数据边界：它只暴露一个纯数据快照 [`BranchSnapshot`] 与一个加载函数 [`BranchSnapshot::load`]，
//! 不依赖任何界面零件（不 import `component::command`），工作台因此不需要知道
//! `git.status` 的 payload 形状或 `git.references` 的字段名。
//!
//! # 数据来源（逐条对真源）
//!
//! | 用途 | 命令 | 真源 |
//! | --- | --- | --- |
//! | 仓库根 + 当前分支 + ahead/behind | `git.status`（`{ root }`） | `git/mod.rs:94-96`；响应 `protocol/contracts.rs:532-538`；非仓库是 `ok:true` + `repositoryRoot:null`（`git/mod.rs:6535-6544`） |
//! | 本地分支列表 | `git.references`（`{ root }`）→ 过滤 `kind == "local"` | `git/history.rs:49-51`；条目 `contracts.rs:583-597` |
//!
//! ⚠️ **与 Windows 真源的唯一数据差异（有意）**：Windows 的分支列表走 `git_branches`，
//! 而宿主把它翻译成 Core 的 **`git.history`**（`windows/tauri/src-tauri/src/platform.rs:219`），
//! 适配器只取 `kind === "local"` 的 `shortName`（`platform/core-result-adapter.ts:247-254`）——
//! 只拿到名字，还顺带拉一页提交。研究 §3.1 的脚注把 `git.references` 列为**更优**的数据源
//! （一条命令就带 `isCurrent` / `upstreamShortName` / `ahead` / `behind`），本模块按它取，
//! 于是当前分支的判定来自 Core 的 `isCurrent`，而不是前端拿 `status.branch` 与名字比对。
//! 这条偏离登记在研究 §7.2 第 4 条，本侧按研究给的更优解落地。
//!
//! # 非仓库 / 仓库在子目录
//!
//! 工作区根**不一定是仓库根**，所以本模块走与 `model::load_first` 完全相同的三步
//! （`model.rs:750-767`）：`git.status` → `resolve_repository_root` → 仍找不到时
//! `discover_repository_root`（`workspace.snapshot` 一级目录 + `git.status` 探测，上限 6）。
//! 这样"工作区里放了一个仓库"与"工作区本身就是仓库"给出同一份 [`BranchSnapshot`]。
//! 三条命令都失败时快照是**空**的（`repository_root = None`、`branch = None`、`branches = []`），
//! 失败的命令名进 [`BranchSnapshot::failures`] —— 调用方据此决定"标题栏那一项不渲染"。

use std::path::Path;

use gpui_kit::SharedString;

use crate::model::{
    RefKind, Reference, discover_repository_root, execute_core, parse_references,
    resolve_repository_root, repository_root_of,
};

/// 一条本地分支。
///
/// 真源的分支行（`git-branch-manager.tsx:866-954`）只画三样：前导图标（当前分支是绿勾、
/// 其它是分支图标）、分支名、右侧「当前」徽章。名字之外它**不需要**任何字段，因为列表数据
/// 只来自 `shortName`；本结构体多带一个 [`BranchInfo::upstream`]，是"一条命令就拿到了"的
/// 附带信息，v1 界面**不画**它（真源的标题栏不画数字、行内也不画计数），
/// 留在这里是为了让下一版（研究 §7.1 的 B/C 档）不必再改数据层。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchInfo {
    /// 本地分支名（Core 的 `shortName`，如 `main` / `feature/login`）。
    pub name: SharedString,
    /// Core 的 `isCurrent`（`contracts.rs:589`）。
    pub is_current: bool,
    /// 上游分支名（Core 的 `upstreamShortName`，`contracts.rs:588`）；没有上游是 `None`。
    pub upstream: Option<SharedString>,
}

/// ahead / behind 一对计数（Core `git.status` 的 `ahead` / `behind`，`contracts.rs:535-536`）。
///
/// 真源标题栏那一项**不画数字**：`GitTrackingCounts showCounts={false}` 只输出 `↙` / `↗`
/// 两个箭头，且两个都为 0 时整个返回 `null`（`git-branch-manager.tsx:654-660`；
/// `git-tracking-counts.tsx:33,36-53`）。所以本结构体只携带"有没有、谁大"，
/// 箭头字形由 workbench 侧决定。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackingCounts {
    pub ahead: usize,
    pub behind: usize,
}

impl TrackingCounts {
    /// 两个都为 0 → 真源不画任何箭头（`git-tracking-counts.tsx:33` 的早退）。
    pub fn is_empty(self) -> bool {
        self.ahead == 0 && self.behind == 0
    }
}

/// 标题栏「分支弹窗」要显示的全部只读数据。
#[derive(Clone, Debug, Default)]
pub struct BranchSnapshot {
    /// 解析成**绝对路径**的仓库根；`None` = 这个工作区不是 Git 仓库（也不含仓库子目录）。
    pub repository_root: Option<SharedString>,
    /// 当前分支名（Core `git.status.branch`）；分离 HEAD 时 Core 给英文常量 `detached`
    /// （`rust/lithe-core/src/git/mod.rs:6549-6553`，研究 §5 第 3 行已登记这条待拍板）。
    pub branch: Option<SharedString>,
    /// 当前分支的 ahead / behind（Core `git.status`）。
    pub tracking: TrackingCounts,
    /// **本地**分支，当前分支置顶、其余按名字排序（真源排序见 `git-branch-manager.tsx:66-70`）。
    pub branches: Vec<BranchInfo>,
    /// 失败的命令名（空 = 全部成功）。**不静默**：调用方拿它打诊断。
    pub failures: Vec<String>,
}

impl BranchSnapshot {
    /// 当前分支在 [`Self::branches`] 里的下标（`None` = 不在列表里，例如列表没取到）。
    pub fn current_index(&self) -> Option<usize> {
        self.branches.iter().position(|branch| branch.is_current)
    }

    /// 读一次标题栏面板需要的全部数据（**同步**，两条 Core 命令）。
    ///
    /// 同步是刻意的：调用点只有"启动期一次 + 打开面板 + 点刷新"，与 `model::load_first`
    /// 一样是"点下去就该看到结果"的交互，且两条命令都很快（`git.status` 与
    /// `git.references` 都是本地读）。真源同样是点击时 `Promise.all([loadBranches()])`
    /// 直接读（`git-branch-manager.tsx:526-531`），没有订阅任何变更事件（研究 §3.3）。
    pub fn load(root: &Path) -> Self {
        let mut snapshot = Self::default();

        // ① `git.status`：仓库根 + 当前分支 + ahead/behind。
        let root_text = root.to_string_lossy().to_string();
        match read_status_detailed(&root_text) {
            Ok((repository_root, branch, tracking)) => {
                snapshot.repository_root =
                    repository_root.map(|found| resolve_repository_root(root, &found).into());
                snapshot.branch = branch;
                snapshot.tracking = tracking;
            }
            Err(code) => snapshot.failures.push(format!("git.status（{code}）")),
        }

        // ② 工作区根不是仓库时，用一级目录再找一次（与 `model::load_first` 同一兜底）。
        if snapshot.repository_root.is_none() {
            snapshot.repository_root = discover_repository_root(root).map(SharedString::from);
        }

        let Some(repository_root) = snapshot.repository_root.clone() else {
            // 不是仓库：**不再白跑** `git.references`（`model.rs` 的约束 2 同一条）。
            return snapshot;
        };

        // ③ `git.references`：本地分支列表。
        match execute_core(
            "git.references",
            serde_json::json!({ "root": repository_root.as_ref() }),
        ) {
            Ok(data) => {
                let references = parse_references(&data);
                // 当前分支的判定优先用 Core 的 `isCurrent`；它一条都没标时回落到名字比对
                // （真源就是这个回落：`a === currentBranch`，`git-branch-manager.tsx:66`）。
                let has_current = references.iter().any(|reference| reference.is_current);
                snapshot.branches =
                    local_branches(&references, &snapshot.branch, has_current);
            }
            Err(code) => snapshot.failures.push(format!("git.references（{code}）")),
        }

        snapshot
    }
}

/// `git.status` 的完整读法：`(仓库根, 分支, ahead/behind)`。
///
/// `model::read_status` 只取前两样（`model.rs:493-500`，底部提交记录用不到 ahead/behind），
/// 标题栏要用，所以这里读同一份响应里多出来的两个字段。**不是第二条命令**：
/// `execute_core("git.status", ..)` 只调一次。
fn read_status_detailed(
    root: &str,
) -> Result<(Option<String>, Option<SharedString>, TrackingCounts), String> {
    let data = execute_core("git.status", serde_json::json!({ "root": root }))?;
    let branch = data
        .get("branch")
        .and_then(serde_json::Value::as_str)
        .map(SharedString::from);
    let tracking = TrackingCounts {
        ahead: data
            .get("ahead")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize,
        behind: data
            .get("behind")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize,
    };
    Ok((
        repository_root_of(&data),
        branch,
        tracking,
    ))
}

/// 从 `git.references` 的全部引用里挑出**本地分支**，当前分支置顶、其余按名字排序。
///
/// 真源的前端排序（`git-branch-manager.tsx:66-70`）：
/// `a === currentBranch` 置顶，其余 `localeCompare`。这里用 `sort` 的稳定键
/// `(is_current ? 0 : 1, name)`，与 `localeCompare` 在"按码点比较"上一致（分支名是 ASCII
/// 路径片段，差异只在大小写，本侧不做本地化排序 —— 研究 §1.4 的排序一栏）。
///
/// 纯函数：不碰 Core、不碰界面，单测直接覆盖（见文件末尾）。
fn local_branches(
    references: &[Reference],
    status_branch: &Option<SharedString>,
    has_current_flag: bool,
) -> Vec<BranchInfo> {
    let mut branches: Vec<BranchInfo> = references
        .iter()
        .filter(|reference| reference.kind == RefKind::Local)
        .map(|reference| BranchInfo {
            name: reference.short_name.clone(),
            is_current: if has_current_flag {
                reference.is_current
            } else {
                // Core 没标 `isCurrent` 时的回落：与 `git.status` 的分支名逐字相等
                // （真源同样是名字比对，`git-branch-manager.tsx:66,282`）。
                status_branch
                    .as_ref()
                    .is_some_and(|branch| branch == &reference.short_name)
            },
            upstream: reference.upstream_short_name.clone(),
        })
        .collect();

    branches.sort_by(|left, right| {
        right
            .is_current
            .cmp(&left.is_current)
            .then_with(|| left.name.cmp(&right.name))
    });
    branches
}

#[cfg(test)]
mod tests {
    use super::{BranchInfo, BranchSnapshot, RefKind, TrackingCounts, local_branches};
    use crate::model::Reference;
    use gpui_kit::SharedString;

    fn reference(name: &str, kind: RefKind, is_current: bool) -> Reference {
        Reference {
            full_name: SharedString::from(format!("refs/heads/{name}")),
            short_name: SharedString::from(name),
            kind,
            is_current,
            upstream_short_name: None,
            ahead: 0,
            behind: 0,
        }
    }

    /// 列表只收**本地**引用：远端分支与 tag 一个都不能出现
    /// （真源适配器只留 `kind === "local"`，`core-result-adapter.ts:247-254`）。
    #[test]
    fn only_local_references_become_branch_rows() {
        let references = vec![
            reference("main", RefKind::Local, true),
            reference("origin/main", RefKind::Remote, false),
            reference("v1.0.0", RefKind::Tag, false),
            reference("feature/login", RefKind::Local, false),
        ];
        let branches = local_branches(&references, &Some(SharedString::from("main")), true);
        let names: Vec<&str> = branches.iter().map(|b| b.name.as_ref()).collect();
        assert_eq!(names, vec!["main", "feature/login"]);
    }

    /// 当前分支置顶，其余按名字升序（真源 `git-branch-manager.tsx:66-70`）。
    #[test]
    fn current_branch_comes_first_then_alphabetical() {
        let references = vec![
            reference("zeta", RefKind::Local, false),
            reference("main", RefKind::Local, true),
            reference("alpha", RefKind::Local, false),
        ];
        let branches = local_branches(&references, &Some(SharedString::from("main")), true);
        let names: Vec<&str> = branches.iter().map(|b| b.name.as_ref()).collect();
        assert_eq!(names, vec!["main", "alpha", "zeta"]);
    }

    /// Core 一条 `isCurrent` 都没标时回落到"与 `git.status.branch` 同名"，且**只标一条**
    /// （两条同名分支不存在，但回落逻辑不能把全部行都标成当前）。
    #[test]
    fn current_flag_falls_back_to_the_status_branch_name() {
        let references = vec![
            reference("main", RefKind::Local, false),
            reference("dev", RefKind::Local, false),
        ];
        let branches = local_branches(&references, &Some(SharedString::from("dev")), false);
        let current: Vec<&str> = branches
            .iter()
            .filter(|branch| branch.is_current)
            .map(|branch| branch.name.as_ref())
            .collect();
        assert_eq!(current, vec!["dev"]);
        assert_eq!(branches[0].name.as_ref(), "dev");
    }

    /// 空引用列表 → 空分支列表（不是仓库 / 读失败时的形态）。
    #[test]
    fn empty_references_give_an_empty_branch_list() {
        assert!(local_branches(&[], &None, false).is_empty());
        let snapshot = BranchSnapshot::default();
        assert_eq!(snapshot.current_index(), None);
        assert!(snapshot.failures.is_empty());
    }

    /// `current_index` 指向置顶的那一行；没有当前分支标记时是 `None`。
    #[test]
    fn current_index_points_at_the_pinned_row() {
        let snapshot = BranchSnapshot {
            branches: vec![
                BranchInfo {
                    name: SharedString::from("main"),
                    is_current: true,
                    upstream: None,
                },
                BranchInfo {
                    name: SharedString::from("dev"),
                    is_current: false,
                    upstream: Some(SharedString::from("origin/dev")),
                },
            ],
            ..BranchSnapshot::default()
        };
        assert_eq!(snapshot.current_index(), Some(0));
    }

    /// `TrackingCounts::is_empty` 就是真源 `GitTrackingCounts` 的早退判据
    /// （`git-tracking-counts.tsx:33`：两个都为 0 时整段返回 `null`）。
    #[test]
    fn tracking_counts_are_empty_only_when_both_are_zero() {
        assert!(TrackingCounts { ahead: 0, behind: 0 }.is_empty());
        assert!(!TrackingCounts { ahead: 2, behind: 0 }.is_empty());
        assert!(!TrackingCounts { ahead: 0, behind: 1 }.is_empty());
    }
}
