# GPUI 架构缺口 · 第 3 步：DAP / Git 命令差集 + 证据台账计数复核

**取证脚本**：`reports/gpui-arch-s3-probe.ps1`（只读检索，已实跑）
**运行方式**：仓库根目录 `pwsh -NoProfile -File reports/gpui-arch-s3-probe.ps1`
**基线**：提交 `3085f5d1`，分支 `feat/menu-bar-opaque`
**运行状态**：**已实跑**（2026-10-12，Windows PowerShell 7）。

> **脚本修订记录**：首次运行时脚本未排除 `rust/lithe-gpui/target/`，把构建期生成的 14 个 `.rs` 也扫了进去，导致 `viewer` 关键词虚报命中 1（实际来自 `mime_guess` 的生成文件）。已在三个脚本里统一加 `Where-Object { $_.FullName -notmatch '\\target\\' }`，本文所有数字是**修订后**重跑的读数（纳入扫描：lithe-gpui 源码 123 个 `.rs`，lithe-core 源码 140 个 `.rs`）。

---

## 结论

### 1. DAP：Core 有 18 个 debug 命令，GPUI 只用 9 个

脚本 §A1 实测：`rust/lithe-core/src/runtime/dispatcher.rs:673-980` 有 **18 个** `CoreCommand::Debug*` 分支。§A2 逐命令比对 GPUI 引用数：

| GPUI 引用 0 的命令（**8 个**） | Core dispatcher 行号 |
|---|---|
| `debug.cancelOperation` | :878 |
| `debug.dataBreakpointInfo` | :823 |
| `debug.javaTestLaunch` | :706 |
| `debug.runInTerminalResponse` | :941 |
| `debug.setDataBreakpoints` | :842 |
| `debug.setExceptionBreakpoints` | :781 |
| `debug.setFunctionBreakpoints` | :802 |
| `debug.setVariable` | :860 |

（§A2 输出里引用数为 0 的正是上表这 8 个；其余 10 个命令各有 1～2 处引用。）

⇒ 这 8 项是**「Core 已实现、GPUI 零引用」**。它们对应调试体验的实质能力：**改变量值**（`setVariable`）、**异常断点**（`setExceptionBreakpoints`）、**函数断点**（`setFunctionBreakpoints`）、**数据断点**（`setDataBreakpoints` + `dataBreakpointInfo`）、**取消操作**（`cancelOperation`）、**runInTerminal 应答**（`runInTerminalResponse`）。

**注意 `debug.cancelOperation` 的含义**：GPUI 侧不引用它，意味着**没有取消在途调试操作的路径**。这与 `lsp.cancelOperation`（同样 0 引用）是同一个架构缺口——**取消语义在 UI 层缺失**。

### 2. Git：`git.write` 是复用型多路器，不能只看命令名

脚本 §B1 显示 44 个 `git.*` 命令里 24 个 GPUI 引用为 0。**但这个数字会误导**，因为 GPUI 主要走 `git.write` 这个多路器（引用 8 次），命令名藏在 `operation` 字段里。

实测 GPUI 通过 `git.write` 实际发出的 operation（`crates/git/src/changes.rs` + `remote.rs`）：

| 来源 | operation |
|---|---|
| `changes.rs:456` `stage()` | `stage` |
| `changes.rs:460` `unstage()` | `unstage` |
| `changes.rs:464` `stage_all()` | `stageAll` |
| `changes.rs:467` `commit()` | `commit` |
| `changes.rs:476` `discard()` | `discard` |
| `remote.rs` | `fetch` / `pull` / `checkout` |

而 Core 的 `git::write`（`rust/lithe-core/src/git/mod.rs:930` 起）**接受 26 种 operation**：

```
stage, unstage, discard, discardAll, stageAll, commit, ignore, exclude,
cherryPick, revert, reset, createBranch, publishBranch, renameBranch,
setUpstream, unsetUpstream, deleteBranch, merge, rebase, checkoutAndRebase,
createWorktree, pruneWorktrees, repairWorktrees, updateBranch, fetch, pull
```

⇒ **Core 侧 26 种写操作，GPUI 只用了 8 种**，未用的 18 种里包含 **merge / rebase / cherryPick / revert / reset / createBranch / deleteBranch / renameBranch / discardAll / createWorktree** —— 这些正是「Git 主动操作」缺口。

### 3. `IntegrationOperation` / `integration_gate` 确实无 UI 消费者

脚本 §B2 实测全部 22 处命中，**全在 `crates/git/src/` 内部**：

```
crates/git/src/lib.rs:17,18,20        （re-export）
crates/git/src/remote.rs:45           pub enum IntegrationOperation
crates/git/src/remote.rs:52           impl IntegrationOperation
crates/git/src/remote.rs:318          pub struct IntegrationPreflight
crates/git/src/remote.rs:338,339      parse_integration_preflight
crates/git/src/remote.rs:410,411      integration_preflight()
crates/git/src/remote.rs:515          pub fn integration_gate
crates/git/src/remote.rs:659          （单测断言）
crates/git/src/remote.rs:690          （单测 fixture）
crates/git/src/remote.rs:839-855      （单测 integration_gate_reports_the_shape_of_the_block）
crates/git/src/remote.rs:940,944      （单测）
```

**`crates/workbench`、`crates/editor`、`crates/git/src/*_view.rs` 一处都没有**。⇒ 判定成立：**实现与单测齐备，UI 消费者为零**。

---

## 台账复核（3 处，2 处不一致）

| # | 台账说法 | 我的实测 | 判定 |
|---|---|---|---|
| 1 | §2「可执行入口 `MenuItem::Action` 为 **52** 个」 | **51**（数组内，595–935 行） | **不一致，差 1** |
| 2 | §2「`MenuItem::NotWired` 在 `MENUS` 数组内出现 **42** 次」 | **42** | 一致 |
| 3 | §2「另有 `NOT_WIRED_OUTSIDE_MENUS` 2 条，合计 **44**」 | 声明为 `[(&str, Missing); 2]`（`menu_bar.rs:172`），合计 44 | 一致 |
| 4 | §2.2「Git 身份配置页：`set_git_identity_host` **生产代码零调用**」 | **`crates/workbench/src/workspace.rs:883` 在生产代码里调用** | **不一致** |
| 5 | §2.2「`go_back`/`go_forward` 标 `#[allow(dead_code)]`」 | 仍标（`editor_view.rs:1082`、`:1093`） | 一致 |
| 6 | §2.3「`viewer` 命中 0」 | **0**（排除 `target/` 后） | 一致（我的脚本首跑虚报 1，是脚本缺陷） |

### 关于第 4 条：Git 身份配置页**不是**零调用

台账 §2.2 写「`set_git_identity_host` 生产代码零调用」。实测：

```
crates/settings/src/identity.rs:173   pub fn set_git_identity_host(...)      ← 定义
crates/workbench/src/workspace.rs:883 lithe_gpui_settings::set_git_identity_host(Some(GitIdentityHost { ... }))   ← 生产调用
crates/settings/src/identity.rs:257   set_git_identity_host(None);            ← 测试（文件首个 #[cfg(test)] 在 :251）
crates/settings/src/dialog.rs:3543    crate::identity::set_git_identity_host(None);  ← 测试（首个 #[cfg(test)] 在 :3431）
```

**关键判定依据**：`workspace.rs:883` 所在的函数是 **`pub fn new`（起始于 :587）**，而该文件首个 `#[cfg(test)]` 在 **:3395**（全文 3975 行）。⇒ `:883` **在生产代码里**。

配套证据：设置对话框确实消费它 —— `dialog.rs:264` 的 `host: git_identity_page()`、`:252` 的字段声明、`:801` 的 `host_workspace_root()`。⇒ **Git 身份配置页是接线完成的**。

**这条差异会改变排期**：台账把它列为「未接线」，若照抄会重复排一次已经做完的活。**建议主理人订正台账**。

### 关于第 1 条：Action 计数的口径

台账 52 vs 实测 51。总数 44（42+2）自洽，说明台账没算错总数，差异出在「数组内 Action」的边界。我在第 2 步已记录同一差异，此处复现。**我不改台账，只报实测值。**

---

## 风险与条件

| 风险 | 影响 | 触发条件 | 缓解 |
|---|---|---|---|
| **`git.write` 掩盖缺口** | 按「命令名引用数」评估会得出 24 个 Git 缺口，实际只有 18 种 operation 未用，且命令名引用数与能力缺口不是一对一 | 排期时直接用 §B1 的 0 值当缺口清单 | 判定口径：**先看是否走 `git.write` 多路器，再看 operation 覆盖面**。本文 §2 已给出 26 vs 8 的正确口径 |
| **台账 §2.2 的 Git 身份条目过期** | 重复排期、白做一遍 | 照抄台账排期 | 订正台账；本报告给出反证行号 |
| **取消语义整体缺失** | 调试与语义操作无法中止，长请求只能等超时 | 用户想取消一次卡住的格式化/调试 | `debug.cancelOperation` 与 `lsp.cancelOperation` 都是 0 引用；属同一个架构缺口，应作为一项设计（不是两项功能）排期 |
| **`javaTestLaunch` 归属** | 可能被误当作 DAP 缺口 | 见下 | 它属 Java 测试运行（`debug.javaTestLaunch`），与 `run-java-test` 矩阵条目相关，需与 Java 工具链排期合并评估 |

### 假设与限制

1. **「引用数为 0」不等于「用户完全无法触发」**。例如 `git.commit` 命令名引用 0，但 `commit` operation 走 `git.write` 是通的（`changes.rs:467`）。本文已按 operation 口径修正，但**其余命令（如 `git.stashes`、`git.blame`、`git.diff`）我没有逐一确认是否存在多路器旁路**——它们不在 `git.write` 的 26 种 operation 里，因此判定为真缺口。
2. **未验证 Core 的 `git.write` 26 种 operation 在真实仓库上都能成功**。本文只读了 operation 白名单（`mod.rs:930` 起的 match 分支），**没有运行**任何 git 操作。这属于运行验证，本次**未做**。
3. **未运行任何 `cargo` 命令**，未跑 Core 的 git/DAP 测试。
4. **台账复核只覆盖了我检索到的 3 处**（§2 计数、§2.2 身份页、§2.2 dead_code、§2.3 关键词）。台账其余条目未逐条复核。

---

## 建议与验收

| 优先级 | 行动 | 前置 | 完成标准 |
|---|---|---|---|
| P0 | **订正台账 §2.2 的 Git 身份配置页条目**（改为「已接线」并附 `workspace.rs:883`） | 交主理人 | 台账与代码一致，避免重复排期 |
| P0 | 订正台账 §2 的 `MenuItem::Action` 计数（51 或说明口径） | 交主理人 | 台账与实测一致 |
| P0 | 接「改变量值」与「异常断点」 | 无（Core 已就绪） | 调试面板可改变量；`debug.setVariable` / `setExceptionBreakpoints` 引用数从 0 变 1 |
| P1 | 补「取消在途操作」的架构设计（DAP + LSP 一起） | 需定 UI 入口 | `debug.cancelOperation` 与 `lsp.cancelOperation` 有调用点；取消后服务端状态可恢复 |
| P1 | 接 Git 主动操作：merge / rebase / cherry-pick | `IntegrationOperation` 已就绪 | 能发起一次 merge 并在冲突时看到进行中横幅；`integration_gate` 有 UI 消费者 |
| P1 | 接分支管理：createBranch / deleteBranch / renameBranch | Core `git.write` 已就绪 | 日志视图工具栏按钮不再是 `enabled=false` + 空闭包 |
| P2 | 接 stash / tag / diff / blame | 需确认 Core 命令面 | `git.stashes` 等引用数从 0 变 1 |

**验收方式**：`S1_*` 启动日志打点，不是截图。涉及 Git 写入的验收必须在**临时仓库**上做，不能动用户仓库。

**工作量（架构维度，不含 UI 设计与实现工时）**：
- 订正台账两条：**0.5 人日**（含复核其余条目，避免再发现同类过期）
- DAP 两项接线（setVariable + 异常断点）：**1 人日**
- 取消语义架构设计（DAP + LSP 统一）：**1 人日**（设计，实现另计）
- Git 主动操作接线设计（含 `IntegrationOperation` → UI 的接口契约）：**1.5 人日**
- 分支管理三项接线：**1 人日**
- **依赖**：Git 主动操作依赖 `integration_gate` 的状态机在 UI 侧有落点（进行中横幅 + 冲突提示），这是**界面设计问题**，需与界面设计师核对后再定实现；取消语义需要 Core 侧确认取消后的一致性语义，可能触发 Core 改动（受冻结口径约束）。

---

## 未运行验证清单

- 未运行任何 `cargo` 命令（未构建、未跑测试，含 Core 的 git/DAP 测试）。
- 未启动 Lithe 进程，无 `S1_*` 日志。
- 未在真实仓库上执行任何 git 写操作。
- 未验证调试会话的运行时行为（断点命中、变量读取）。
- 台账仅复核 4 处，其余未逐条核对。
- 未在 macOS / Linux 上验证。
