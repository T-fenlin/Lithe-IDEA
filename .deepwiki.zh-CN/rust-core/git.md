# Core: Git

`rust/lithe-core/src/git/` 是 Core 中最大的子系统：`mod.rs` 单独约 6900 行，再加上 20 个聚焦模块。它是**命令门面**；其中几乎所有真正有意思的设计决策都围绕“reviewed mutation”展开。

## 模块地图

| 文件 | 职责 |
| --- | --- |
| `mod.rs` | 命令门面。负责 `git.*` handler、preflight、status、diff、push preview/write、status format strings，以及从 `:6672` 开始的大约 1100 行内联测试 |
| `configuration.rs` | allowlist 的 `git config` 检查、出处、范围受限编辑 |
| `execution_policy.rs` | 请求级 Git preferences + 确定性的临时配置 |
| `execution_events.rs` | Git 事件流（`requestStarted` … `finished`）和 observer scoping |
| `fetch.rs` / `fetch_execution.rs` | 一个共享的 Fetch 命令，同时供 preview 和 execution 使用；每个 remote 的结果保留部分成功与精确 changed refs |
| `graph.rs` | 无渲染器的图投影：整数 lane + typed routes，**不包含渲染路径** |
| `history.rs` | 有界 reference snapshot + 增量历史页 |
| `mutations.rs` | 抽离到 facade 外的共享 mutation helper |
| `patch_exchange.rs` | 无损 UTF-8 patch 导出 / 预览 / 应用和前向检查 |
| `progress.rs` | 与控制台渲染和 pipe framing 无关的确定性进度 |
| `rebase_session.rs` | 受 review 的交互式 rebase，基于 Git 原生可重启 sequencer |
| `rewrite.rs` | 受 review 的历史变更，带 immutable snapshot 和 durable recovery ref |
| `setup.rs` | `git init` 和范围受限的 commit identity 配置 |
| `console/{mod,command,output,types}.rs` | IDEA 风格 console folding 和 retained command text 搜索 |

## Reviewed mutation：preview → write

这是主线模式。一次 mutating operation 分成两步：第一步产出一个*审查过的预期值*，第二步要求现实状态不能偏离这一预期。

```
git.pushPreview   -> GitPushPreviewResponse { localHead, remote, remoteBranch,
                     remoteTrackingOid, tags, commits, hasMore }
git.write { operation: "push", expected: { localHead, remote, ... } }
        ^ 如果本地 HEAD 或 branch.<n>.pushRemote 自预览之后变动：
          报错 stale_preview / message
          "Git push preview is stale; refresh and try again."
          并且**完全不执行 push**
```

同样形态也存在于 rebase（`git/rebase_session.rs:753`，`"The rebase preview is stale; refresh and review it again"`）和 history rewrite（`git/rewrite.rs:608,724`，代码 `stale_preview`，位置 `:402`）。关键不变量是：**preview 和 write 必须对目标和安全选项保持一致**，并通过真实 `git` 二进制测试锁定在 `rust/lithe-core/tests/git_push.rs:156-260`。

其他受保护的变更：

- **History rewrite** 拒绝远端可达提交和 dirty tree（`src/tests/git.rs:1171,1204`）；undo 会保留 staged / unstaged / untracked 内容，并保留 recovery ref（`src/tests/git_history_rewrite.rs:100`）。
- **Patch exchange** 在**不写入**的情况下拒绝 stale preview 和 forward conflicts（`src/tests/git_patch_exchange.rs:172`），同时拒绝不安全路径和 lossy text（`:204`）。
- **Selected commit** 对大 path 集合时把路径写到 **stdin**，而不是命令行，且在失败时回滚而不溢出（`src/tests/git.rs:483,2500`）；失败 hook 会恢复 index，同时保留真实 index 变更（`:558,623`）。

## 命令表面

大约 40 个 `git.*` 命令。gpui 宿主真正发起的（`gpui/crates/git/src/{model,changes,branch_info,identity}.rs`，以及 `gpui/crates/java`）：

`git.status`、`git.write`、`git.references`、`git.historyPage`、`git.historyCursorClose`、`git.commitFiles`、`git.operationState`、`git.repositorySetup`、`git.configureIdentity`、`git.watchContext`、`git.authRespond`。

三个**硬约束**宿主必须遵守，且都在 `gpui/crates/git/src/lib.rs` 中写明：

1. `git.status.repositoryRoot` 可能返回**工作区相对路径**，必须按宿主 root 重新拼接（`gpui/crates/git/src/model.rs:517-528`）。
2. “Not a repository” 是 `ok: true` 且 `repositoryRoot: null`——**不是错误**（`model.rs:805-812`）。
3. `git.write` 即使 Git 退出非零也返回 `ok: true`，因此必须检查 `operationError` / `exitCode`（`changes.rs:488-561`）。

契约类型位于 `protocol/contracts.rs:518-853`；描述性文档在 `shared/contracts/rust-core-api.md`，另外还有 rebase sessions、patch exchange、repository setup 的专项说明（`git-rebase-session.md`、`git-patch-exchange.md`、`git-repository-setup.md`）。

## 确定性细节

| 关注点 | 执行方式 |
| --- | --- |
| Changed paths 排序 | `git/mod.rs:6655`；契约文档中写“Deterministically ordered changed paths”（`contracts.rs:711`） |
| 历史分页是 cursor-based，而非 offset-based | `contracts.rs:640-649`；`next_offset` 明确声明“Deprecated… only for compatibility”。测试断言页面互不重叠：`src/tests/git.rs:3600` |
| 最近 checkout 列表有界且有序 | 最多五个本地分支，最近 checkout 的优先（`contracts.rs:618-619`） |
| Graph projection 是 renderer-neutral 且可复现 | 整数 lane 坐标，`GraphRoute.id` 由 child hash + parent order + parent hash 生成，row 按提供的 history order 排列；测试 `projection_is_deterministic_for_identical_input`（`git/graph.rs:439`） |
| Windows verbatim path 被擦掉 | `simplified_canonical_path`（`git/mod.rs:3053-3071`）—— `\\?\C:\…` 会在分隔符规范化后变成不可解析的 `//?/…`。测试位于 `:6687` |
| `git.status` 不应刷新 index | `src/tests/git.rs:221` |

## Console folding

`git/console/` 是与传输无关的展示逻辑：保守的 command 压缩，要求行为改变的 flag 仍保留可见（`console/command.rs`）；IDEA 风格的 progress folding，同时普通输出仍保持连续文本（`console/output.rs`）；无损 disclosure ranges（`console/types.rs`）。`console/tests.rs` 被描述为“Cross-platform presentation fixtures protect semantics, lossless ranges and grouping”，但 gpui 底部面板目前只渲染 console shell，所有 6 个 toolbar button 都禁用（`gpui/crates/git/src/log_view.rs:1875-1930`，对应 `gpui/crates/git/src/lib.rs:152-154` 的 deviation 6）。真正的输出需要 Git 执行事件通道，而这超出该 crate 的范围。

## 变更起点

| 任务 | 从哪里开始 |
| --- | --- |
| 添加 `git.*` 命令 | `protocol/command.rs`（variant + `parse` arm） → `runtime/dispatcher.rs` → `git/mod.rs`；先加 fixture |
| 修改 reviewed-mutation guard | `git/{mod,rebase_session,rewrite,patch_exchange}.rs` 和对应 `src/tests/git*.rs` |
| 修改 graph layout | 只改 `git/graph.rs`——它不含 renderer paths |
| 修改 console folding | `git/console/` + `git/console/tests.rs`；宿主 UI 是另一层关注点 |
| 添加事件种类 | `git/execution_events.rs:34-65`；还要看嵌套测试 `:537-599` |

## 参考资料

- `rust/lithe-core/src/git/`（全部文件）
- `rust/lithe-core/src/tests/git.rs`、`git_history_rewrite.rs`、`git_patch_exchange.rs`、`git_fetch.rs`、`git_repository_setup.rs`
- `rust/lithe-core/tests/git_push.rs`、`git_watch_context.rs`
- `rust/lithe-core/src/protocol/contracts.rs`
- `rust/lithe-core/src/protocol/command.rs`
- `rust/lithe-core/src/runtime/dispatcher.rs`
- `rust/gpui/crates/git/src/lib.rs`、`model.rs`、`changes.rs`
- `shared/contracts/{rust-core-api,git-rebase-session,git-patch-exchange,git-repository-setup}.md`
