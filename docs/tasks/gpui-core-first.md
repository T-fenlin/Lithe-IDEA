# gpui 宿主任务表（Core-first）

本文是**可逐项执行的任务清单**。判据来自维护者 2026-10-02 的定调：

> gpui 只是一个前端界面。加它是因为"macOS + Windows + 三端代码维护太大"，
> 所以**确定性逻辑一律住在 `rust/lithe-core/`**，三端只做"把命令结果画出来"。

由此得出一条硬判据，下面每条任务都用它筛过：

- **确定性**（同样的输入必然同样的输出、与操作系统和界面无关）→ **必须进 Core**；
- **平台能力**（进程、PTY、socket、文件系统监听、凭据、对话框、DNS/HTTP 传输、窗口）→ **留在宿主**，
  但要用端口 trait 收敛，界面代码不出现平台分支。

> 本文与 Agent Note 的分工：**这里写"怎么改、改成什么样、怎么验"**；
> 决策理由写 `.agents/notes/`。界面逐区域对照见 `rust/lithe-gpui/UI-MAP.md`。

---

## 范围约束（2026-10-02 维护者二次定调，**优先于下文所有阶段**）

> **不要修改 `rust/lithe-core/`。代码改动只落在 `rust/lithe-gpui/`。**

理由是**同步方向**：`rust/lithe-core/` 是从 `main` 同步进来的（见 `74a55eb0`），
在本分支改它，下次同步会被覆盖或产生冲突；**Core 的缺口应由 main 侧修**。

所以下面的阶段要这样读：

| 阶段 / 条目 | 谁能改 | 本分支怎么做 |
| --- | --- | --- |
| 一：C1–C4（`git.write` 两个 operation、`plugins.listCatalog`、内嵌 v2 fixture） | **main** | **不要动**。改为在**前端如实降级**：把"Core 没这条命令"呈现成明确状态，而不是静默失败或假成功 |
| 二：C10–C18（把 git/语言等确定性算法搬进 Core） | **main** | **不要动**。算法暂时留在 gpui 里，别为此改 Core |
| 三：C30–C32、C34–C36（数据库、SQL、文件 CRUD、历史、AI 规划的新契约） | **main** | **不要动**。依赖它们的界面（U21/U22）先不做，或只做到"Core 数据已具备"为止 |
| 一：C5–C7 · 二：C19–C21 · 三：C33 / C37 · 四：H/U/D · 五：V | 本分支 | **按原样执行**（落点都在 `rust/lithe-gpui/`、`scripts/`、`shared/fixtures/`） |

⚠️ **代价要认**：C1–C4 不动，意味着**扩展页恒显"加载失败"、`.lithe/` 共享守卫静默失效**这两处用户可见的坏状态，
在本分支**不会变好**，只能等 main 侧修。前端能做的是让它**可见、可解释**，不是让它"好"。

Core 侧缺口**只上报，不在此分支修**。上报材料 = §0 的三处回归 + 两处契约超前。

---

## 0. 现状底账（先说清哪里漏了）

三处**功能性回归**，都是 `74a55eb0`（从 main 同步 `macos/` + `rust/lithe-core`）造成的：
那次只同步了这两个目录，**没有同步 `shared/`，也没有同步 `rust/lithe-gpui/`**，
于是 Core 退回了旧版，而消费方留在新版。**判定：这是 Core 缺，不是前端缺。**

| 回归 | Core 现状 | 消费方 | 运行期后果 |
| --- | --- | --- | --- |
| `git.write` 的 `excludePatterns` / `unexcludePatterns` | 0 命中（旧版在 `git/mod.rs` 有，走 `mutate_literal_git_ignore_patterns`） | `rust/lithe-gpui/crates/shared/src/workspace_config/sharing.rs:210`、`:361` 仍在发 | `invalid_request: "Unsupported Git write operation"`，`.lithe/` 排除守卫与"取消共享"静默失效 |
| `plugins.listCatalog` | 0 命中；`plugins/mod.rs` 是 **schema v1**，无 `description`/`themes`/`icons`/`fonts`/`debugAdapters`/`categories` | `crates/extensions/src/core_bridge.rs:13` 仍在调；两条测试 `.expect(...)` | 中央扩展页恒显"加载失败"，列表恒空 |
| v2 目录 fixture 未被内嵌 | `official-v2.json` 只出现在文档与注释里，**无 `include_str!`** | 扩展页与契约都假设它已内嵌 | 同上 |

另外两处**契约/文档超前于代码**（不是回归，是从来没做）：

- `shared/contracts/rust-core-api.md:389-390` 列了 `lsp.clearDiagnostics` / `lsp.snapshot`，Core 里 0 实现；
- `shared/contracts/plugin-catalog.md` 整篇描述 schema v2 与 `plugins.listCatalog`，Core 是 v1。

---

## 1. 阶段一：Core 被同步丢掉的回归（⛔ **本分支不改 Core**）

> ⛔ 下表**落点全在 `rust/lithe-core/`，本分支不要执行**。由 main 侧修；本分支只做"前端如实降级"。
> 理由见顶部「范围约束」。

这一阶段在 main 侧做完后，扩展页由失败态转为可用，`.lithe/` 守卫恢复；**不新增架构，只还原**。
本分支能做的就是让这两处失败**可见、可解释**：不要静默失败，也不要假装成功。

| ID | 任务 | 现状 | 要做什么 | 落点 | 验收 |
| --- | --- | --- | --- | --- | --- |
| **C1** | 补回 `git.write` 的 `excludePatterns` | Core 无此 operation，落到 `git/mod.rs:1365-1369` 的 `_ =>` 报 `invalid_request` | 从 `git 74a55eb0~1:rust/lithe-core/src/git/mod.rs` 取回 `"excludePatterns"` / `"unexcludePatterns"` 两个 arm 与 `mutate_literal_git_ignore_patterns`（保留调用方原文、幂等、可精确删除），**不要**用 `append_git_ignore_patterns` 代替 | `rust/lithe-core/src/git/mod.rs` | ① 旧版 `tests/git.rs` 的 8 条断言（`:854,885,896,905,954,973,1018,1023`）恢复并通过；② `cargo test -p lithe-core` |
| **C2** | 补回 `git.write` 的 `unexcludePatterns` | 同上 | 同上（与 C1 同一提交完成） | 同上 | 同上 |
| **C3** | 补回 `plugins.listCatalog` | 0 命中；`plugins/mod.rs:7` 是 `SCHEMA_VERSION = 1` | 恢复 schema v2：贡献项字段 + `categories` 推导（规范顺序见 `plugin-catalog.md:72-86`）+ `hostVersion` 校验 + `invalid_request`/`parse_failed` 失败码（`:88-98`） | `rust/lithe-core/src/plugins/mod.rs`、`protocol/command.rs`、`runtime/dispatcher.rs` | `shared/fixtures/plugins/official-v2.json` 能通过校验；扩展 crate 两条测试转绿 |
| **C4** | 内嵌 v2 目录 fixture | 无 `include_str!` | 在 Core 内嵌 `shared/fixtures/plugins/official-v2.json` 与 `language-support-v2.json` | `rust/lithe-core/src/plugins/mod.rs` | `plugins.listCatalog` 返回非空且 14 个 id 与注册表一一对应 |
| **C5** | 修契约文档超期 | `rust-core-api.md:389-390` 列了不存在的命令 | 二选一：把 `lsp.clearDiagnostics`/`lsp.snapshot` 补进 Core，或从契约删除并加守卫 | `shared/contracts/rust-core-api.md` | 新增守卫：**契约里声明的每条命令都必须能 `CoreCommand::parse` 成功**（见 C6） |
| **C6** | 加"契约命令存在性"守卫脚本 | 无；`HANDOFF.md:21-25` 记录的验证只到 `cargo build` + `cargo test -p lithe-core`，**没人跑扩展 crate 的测试**，所以这次事故没被拦住 | 写一个脚本：解析 `rust-core-api.md` 的命令表，逐条断言 `CoreCommand::parse` 通过 | `scripts/verify-core-command-surface.sh`（或 .mjs） | 故意删一条命令实现，脚本必须失败 |
| **C7** | 核对 v2 目录的宿主版本 | `HOST_VERSION = "0.0.1"`（`crates/shared/src/lib.rs:62`）与 fixture `hostVersion` 一致 | 确认恢复 C3 后不会被版本拒；若不一致则对齐 | `crates/shared/src/lib.rs`、fixture | 扩展页显示真实目录 |

---

## 2. 阶段二：把**住在 gpui 里的确定性算法**搬进 Core（⛔ **本分支不改 Core**）

> ⛔ 落点在 `rust/lithe-core/`，**本分支不要执行**。算法暂时留在 gpui 里，别为此改 Core。
> 理由见顶部「范围约束」。

判据同上。这些代码现在放在前端，意味着 macOS/Windows 端要各写一遍——正是加 gpui 要消除的成本。
搬完之后 gpui 只保留"把 Core 返回的结构画出来"。

| ID | 任务 | 现状落点（gpui） | 要做什么 | 验收 |
| --- | --- | --- | --- | --- |
| **C10** | Git 提交泳道几何算法进 Core | `crates/git/src/model.rs:917`（`layout_graph`） | 输入"父哈希 + 提交序"，输出"每条提交的泳道号与连线"，作为纯函数进 Core；gpui 只画 | Core 侧单测覆盖分叉/合并/多根；gpui 泳道图渲染不变 |
| **C11** | Git 变更列表解析进 Core | `crates/git/src/changes.rs:392`（`parse_changes`） | `git.status` 原始响应 → 结构化变更条目（分类、状态码、显示名） | 同样的输入产生同样的分类与排序 |
| **C12** | 提交文件树聚合进 Core | `crates/git/src/model.rs:1146`（`build_commit_files`） | 平面 name-status 列表 → 多层目录树 + 递归计数 | Core 单测覆盖深层嵌套与重命名 |
| **C13** | 引用树构建进 Core | `crates/git/src/model.rs:1013`（`build_reference_rows`） | `git.references` → 本地/远端/标签三分区 + `/` 目录分组 + 折叠结构 | 分组顺序与 gpui 现状逐项一致 |
| **C14** | 进行中操作判定进 Core | `crates/git/src/changes.rs:453`（`parse_operation`） | `git.operationState` → 操作种类与可执行动作（continue/skip/abort） | 四种操作 + 未知 kind 各有断言 |
| **C15** | ahead/behind 取数与上限进 Core | `crates/git/src/branch_info.rs:320`（`tracking_count`） | 跟踪计数语义（含 99 上限、双 0 不显示） | 边界用例有断言 |
| **C16** | Git 路径显示名缩短进 Core | `crates/git/src/model.rs`（`shorten_path` 一族） | 绝对/相对路径 → 稳定显示名 | 同 C11 |
| **C17** | 语言 ↔ 扩展名映射进 Core | `crates/editor/src/buffer.rs:68`（`language_for_file`） | 现在是 gpui 的硬编码表；macOS 侧另有一份（`SyntaxHighlightingConfiguration.swift`）。收敛成 Core 一张表 + 两端消费 | 两侧对同一文件名给出同一语言 id；**这是"三端各写一遍"的典型样本** |
| **C18** | 同名标签显示名进 Core | `crates/editor/src/buffer.rs:226`（`display_names`） | 打开文件集合 → 每个标签的显示名（`名 · 目录段`） | 同名/多层同名用例有断言 |
| **C19** | 终端 shell 探测与 profile 发现移出 gpui | `crates/terminal/src/profile.rs:110,171-182` —— **硬编码 `["powershell","cmd"]`**，在 macOS 上直接导致终端打不开 | 探测逻辑按平台能力分离：**候选清单与选择规则进 Core**，`$SHELL`/`/etc/shells`/`where.exe` 的**执行**留在宿主端口 | ① 至少修正"macOS 上 profiles 为空"；② Core 侧单测：给定候选与探测结果 → 选中的 profile |
| **C20** | `runConfig.resolve` 三层合并接线 | gpui 只用 `generate`（`crates/run/src/model.rs:25-35`），`Project`/`Local` 层恒不出现 | 用 Core 已有的 `runConfig.resolve` 做 generated/project/local 三层合并 | 三层同时存在时合并结果有断言 |
| **C21** | JDTLS 载荷发现与工作区指纹核对归位 | `crates/java/src/jdtls.rs`（45 处）、`crates/java/src/workspace.rs`（24 处） | 先**判定**：哪些是"扫目录找 jdt.ls"这种平台能力（留宿主），哪些是"载荷版本兼容/工作区 key 派生"的确定性规则（进 Core）。macOS 侧对应实现是 `MacServiceContainer` 与 `java.jdtWorkspaceKey` | 判定结论写进本表并落一条 Note；能进 Core 的部分有单测 |

---

## 3. 阶段三：Core 也缺的能力（新增契约）（⛔ **本分支不改 Core**）

> ⛔ C30–C32、C34–C36 落点在 `rust/lithe-core/`，**本分支不要执行**。
> 依赖它们的界面（U21 数据库、U22 AI）先不做，或只做到"Core 数据已具备"为止。
> C33 / C37 落点在 gpui，正常执行。理由见顶部「范围约束」。

这些是"连 Core 都没有"的，前端做不了或做了就是第二个真相源。

| ID | 任务 | 现状 | 要做什么 | 验收 |
| --- | --- | --- | --- | --- |
| **C30** | 数据库命令面 | Core `database.*` **0 条**；但 `rust/lithe-db-sidecar`（4045 行）已存在、协议与 macOS 同构、CI 有 lane，**只是没有任何 gpui crate 依赖它** | 定义 Core 的命令面（`testConnection`/`listDatabases`/`listTables`/`describeTable`/`query`/`pageTable`/`execute`/`applyChanges`/`schemaChange`/`explain`/`transaction`/导入导出/备份恢复），sidecar 进程生命周期作为**平台的进程端口**注入 | ⚠️ **前置任务**：先做 sidecar 与 macOS `DatabaseSidecarService.swift:25-41` 的**逐字段 diff**（`ssl`/`caCertificatePath`/`ssh*`/`proxyURL`/`readOnly`/`productionProtection`/`allowWrite`/`confirmed`），不 diff 不要开工 |
| **C31** | SQL 词法/切分/风险分级/格式化进 Core | 现在在 macOS 的 `DatabaseSQLSupport.swift:143-192`（确定性逻辑，却只在一端） | 搬进 Core：语句切分、危险写操作判定（无 WHERE 的 UPDATE/DELETE、DROP、TRUNCATE）、格式化 | Core 单测覆盖方言差异与边界 |
| **C32** | schema diff → 迁移 SQL 生成进 Core | 现在在 `DatabaseSchemaDiff.swift:45-72,205-215` | 进 Core：顺序排名、方言引用符、迁移语句生成 | 双向往返用例 |
| **C33** | 插件安装/卸载/下载命令面 | `plugin-catalog.md:102` 明说"v2 没有远程 registry、没有安装面"；macOS 有自己的 `MacPluginPackageDownloader`/`Store`/`Manager` | 把"包下载与版本存储"的**规则**进 Core（版本兼容区间、安装状态机），**网络与文件写入**留宿主端口 | 契约 + fixture + 状态机单测 |
| **C34** | 文件 CRUD 端口 | Core 只有 `file.read`/`file.write`；重命名/删除/移动/复制在 macOS 是宿主端口（`WorkspaceFeatureModel.swift:536-554`） | 按同构补 Core 命令或明确端口契约，让两端共用一套语义（含名字合法性校验、冲突提示规则） | 契约 + 两端消费点 |
| **C35** | 本地历史的**记录触发点**契约 | Core 有 `history.record/entries/content/relocate/rename/delete`；gpui **零调用点**，且没有触发契约 | 明确"什么时机必须记录"（保存、外部变更、批量替换），写成契约让三端一致 | 契约 + Core 单测 |
| **C36** | AI：PR 描述生成的规划函数 | Core 只有 commit 的 `plan_commit`；macOS 有 `makePullRequestPrompts`（`CommitMessageGenerationService.swift:237-289`） | 把 PR 描述的 prompt 规划搬进 Core（与 `plan_commit` 同构）；**HTTP 与凭证不进 Core** | 与 macOS 语义一致的 fixture |
| **C37** | AI：契约文档的宿主集成路径 | `shared/contracts/ai-commit.md:60-76` 整节是「Windows adapter surface」，教集成方走已删除的 `windows/tauri` 的 4 条 `platform_invoke` | 改写为"宿主直接链接 `lithe_core::ai`"（`rust-core-api.md:202-205` 已说明按设计不加 JSON 命令） | 文档与 `generation.rs` 逐条对得上 |

---

## 4. 阶段四：宿主与界面（属于前端，按容量排）

**这些不进 Core**（平台能力），但要在宿主侧补端口与界面。macOS 侧规模用于估工作量。

### 4.1 平台端口（Core 明确不管，宿主必须自己写）

| ID | 任务 | 端 | 说明 |
| --- | --- | --- | --- |
| **H10** | 调试适配器 transport | 各端 | socket / 子进程 / 适配器进程生命周期。Core `debug/engine.rs:1` 明确"byte transport and process lifecycle remain platform owned"；macOS 有 5 个实现（`Mac{Java,Dlv,Node,Process,Server}DebugAdapterTransport.swift`），gpui **一个都没有** |
| **H11** | Java Debug Server 启动与端口协商 | 各端 | `vscode.java.startDebugSession` 不在 Core 命令面里（`run/src/lib.rs:34` 把它列为下一步入口）；gpui 只发现了载荷（`java/src/jdtls.rs:30`），没启动它 |
| **H12** | 文件系统 watcher | 各端 | macOS 用 FSEvents（`MacDirectoryWatcher.swift:17-86`）；gpui **无 watcher**，树与「更改」列表会过期。Core 已备好 `searchIndex.update` 供 watcher 驱动 |
| **H13** | 终端 PTY | 各端 | gpui `rust/Cargo.lock` 里 `portable-pty`/`alacritty_terminal`/`vte`/`vt100`/`conpty` **零命中**，现在是 `std::process::Command` + 三管道 + ANSI **丢弃**（`ansi.rs:155-175`）→ 无 Ctrl+C、无 resize、无 TUI |
| **H14** | 窗口与多窗口 | 各端 | gpui 的 `OpenDestination::NewWindow` 是死路（只打 `report_new_window_not_wired`）；macOS 有 `LitheWindowCoordinator`（`RootView.swift:402-677`） |

### 4.2 界面（gpui 侧，Core 数据已具备）

| ID | 任务 | macOS 规模 | Core 状态 |
| --- | --- | --- | --- |
| **U10** | 欢迎页 / 无项目状态 | `WelcomeView.swift` 363 + `RootView.swift:345-352` | 不需要 Core |
| **U11** | Git 写操作闭环：推送/拉取/取回 + 检出前置检查 + 集成前置检查 + 冲突解决 | 多文件 | **Core 全有**（`push`/`pull`/`fetch`/`checkoutPreflight`/`integrationPreflight`/`conflictMarkers`），gpui 零命中 |
| **U12** | 引用行右键 17 项 + 提交行右键 + 提交多选（Cmd/Shift） | `GitReferenceRows.swift:126-215`、`GitGraphView.swift:9-46` | **Core 全有**（`git.write` 47 个 operation 里 gpui 只用了 8 个） |
| **U13** | stash / worktree / tag（含删除后恢复） | 多文件 | **Core 全有**，gpui 零命中 |
| **U14** | 历史改写 + 交互式 rebase | `GitHistoryRewriteView`、`GitInteractiveRebase*.swift` | **Core 全有**（`historyRewritePreview`、`rebase*` 四条） |
| **U15** | 差异视图族（并排/单栏、词级、hunk/行级暂存、diff map、提交审阅、分支比较） | `Views/Diff/` 2343 + `DiffSplitLayout.swift` 184 | **Core 全有**（`git.diff` 8 参数 + `git.apply` 4 模式）。几何算法可直接移植（纯几何、可单测） |
| **U16** | Git 控制台 + 执行日志 + 认证挑战应答 + 执行设置 | `GitConsoleView.swift` 等 | **Core 全有**（`git.consolePresentation`、`execution_events.rs` 782 行、`git.authRespond`、`git.executionInspect/Configure`）；gpui 控制台是**禁用空壳** |
| **U17** ⏳ | 项目级搜索四件套（Find in Files / Search Everywhere / Quick Open / Replace in Files） | 196 + 563 + 313 | ⏳ **四件里做了一件**。**Find in Files 已落地**：`crates/workbench/src/search_model.rs`（**纯逻辑层**：payload、响应解析、结果分组、查询代次）+ `crates/workbench/src/search_panel.rs`（视图：查询框 / 文件掩码 / 区分大小写·全字匹配·正则三个开关 / 文件头+命中行 / `idle`·`loading`·`ready`·`failed` 四态 / 点结果打开并跳行）；左栏「搜索」项现在真的渲染它（`workspace.rs` 的 `left_content` 三选一，与活动栏高亮**同源**）。控件 ↔ Core 字段逐条对应（`query` / `fileMask` / `caseSensitive` / `wholeWords` / `regularExpression` / `maxResults=200`），**故意不传**可见性数组与三个 `max*Results`（与 `searchIndex.warm` 共用同一个索引缓存键）；`operationId` 按代次生成进信封，输入变化即换代并调 `cancel_operation`，旧代次结果一律丢弃；防抖 250ms 与真机同值。诊断锚点（stderr）：`S1_SEARCH issue/complete/stale/failed/cancel/open/option/idle`。新增 20 条确定性单测（workbench 87 → **107**）。**Quick Open 已落地（第二件，2026-10-03）**：新增 `crates/workbench/src/quick_open.rs`（`Ctrl+P` 浮层，形状照 `command_palette.rs` 的 `Dialog` + gpui-kit `Command`；`Command::filterable(false)` 关掉上游的本地二次过滤，避免宿主再写一套匹配）；请求/解析/代次**直接复用 `search_model`** 的 `payload` / `parse_matches` / `QueryTimeline`，只换命令名常量（`workspace.searchEverywhere` 收的**就是** `workspace.search` 那个 `SearchRequest`），只在 gpui 侧取 `kind=="file"` 的行；取真源 `SearchEverywhereView.swift` 的 Files scope 当界面规格（macOS **没有** Quick Open 专属视图，这一步是判断不是逐刻）；`Ctrl+P` 全仓/全 registry 实测**原本零绑定**（`None` context 那条"最深"的坑已核对）；菜单「转到 → 快速打开」原为 `NotWired{MISSING_INDEXER}` → 接成**同一个** Action（`MenuAction::QuickOpen → CommandId::OpenQuickOpen`，符合"一个逻辑命令只建模一次"）；菜单计数不变量同步 41→42 动作 / 51→50 占位。**零新增文案**（复用 `quickOpen.*` 既有键），`WIRED` 补 3 条断言；新增 7 条确定性单测（workbench 107 → **119**）。⚠️ 已知代价：Core **只有"文件+内容+符号"这一条搜索命令**、没有"只搜文件名"的命令，所以 Quick Open 的开销与 Search Everywhere 同量级 —— 要更省得**等 main** 在 Core 侧加纯路径搜索命令，不在 gpui 里做。**Search Everywhere 已做只读判定（2026-10-03，未写代码）**：六个 scope 里 **All / Files / Symbols / Text 四个能由 Core 的 `kind` 直接投影**（Core 的符号索引本来就有 `type`（类/接口/枚举/record）与 `symbol`（方法）两种 kind，所以 **Classes 与 Symbols 不是同一个 kind 的两个视图** —— macOS 侧 `.type→classMatches`、`.symbol→symbolMatches` 与之一致）；**Actions 那一格不来自 Core**（是宿主命令目录，用 `command_palette.rs` 的 `CommandId` / `COMMAND_ORDER`，合并要写成纯函数、**不该进 Core**）。⚠️ 落地前必须补两处宿主泛化：`search_model::Match` 现在**显式丢掉了 `symbolName`**、`MatchKind` 也只认 `file`/`content` ⇒ `type`/`symbol` 落进 `Other`，符号行画不出符号名。⚠️ **两条 Core 缺口已登记为 `BLOCKERS.md` B10、留给 main**：(a) `search_everywhere` 先填 file+content 再追加符号、最后 `truncate(max_results)`，而 file/content 各自就能顶满 200 ⇒ **宽查询下符号会被整段截掉**，且 types/symbols 共用一个 `symbol_limit`（默认与上限都是 50）、Core 不返回按 kind 的分段 ⇒ Symbols/Classes 会**饿死**；(b) macOS 的 All 排序是**纯确定性算法**（匹配形态 400/300/200/100 + kind bonus 12/8/4/0 − 深度罚，同分按原序），按"确定性逻辑进 Core"的判据**该进 Core**，但 Core 响应没有 relevance 字段、本分支不做重排（沿用 Core 反回的固定顺序，它本身是确定的）。因此第三件的**最小正确切片**＝只做 All / Files / Symbols / Text 四个 scope（显式把 `maxResults` 抬到 ≥2× 上限量级以避开饥饿），**不做** Actions、**不做** `/` 命令模式、**不做**重排。**Search Everywhere 已落地（第三件，2026-10-03）**：新增 `crates/workbench/src/search_everywhere.rs`（四 scope 浮层）—— 请求/解析/代次**零新实现**（直接调 `search_model` 的 `payload_with_limit` / `parse_matches` / `QueryTimeline`）、浮层形状照 `quick_open.rs`（`Dialog` + `Command` + `filterable(false)`）、Files scope **直接复用 `quick_open_rows`**、打开结果复用 `EditorPane::open` / `open_at`（`origin_kind="search-everywhere"`）。落地前置的泛化已做且**没破坏两个既有消费方**：`MatchKind` 两值 → 四值、`Match` 加回 `symbol_name`（`search_panel` 只补 `symbol_name: None`；四个搜索模块共 34 条测试全绿）。**All = file + type + symbol 三段、不含 content**（真源明写"正文命中只在 Text 出现，避免与 Find in Files 重叠"）。`maxResults` 抬到 **1000**（`EVERYWHERE_MAX_RESULTS`）以**缩小** B10 的符号饥饿窗口（门槛从 100 条抬到 900 条 file/content）—— 文档里写明这是**缩小不是关闭**、真修在 Core、宿主不许自己截断。入口**只接菜单**（「视图 → 全局搜索」→ `MenuAction::SearchEverywhere` → `CommandId::OpenSearchEverywhere`）：`Ctrl+P`/`Ctrl+Shift+P` 等十余个组合已占，而真源是**双击 Shift**、本仓已登记 gpui 无此通路 ⇒ **不硬造组合键**（`keybind_action()` 返回 `None`，菜单不画胶囊，有单测钉住）。顺带**删掉与事实不符的「还没有索引器」整个占位组**（`gpui.menuMissing.indexer`；`MISSING_GROUPS` 17→16、菜单计数 42/50 → **45/47**），文案经 `extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 新增 8 条 `searchEverywhere.*`（各带 macOS 出处）、两份 yml 由脚本重新生成。新增 9 条确定性单测（workbench 119 → **128**）。**`←` / `→` 切 scope 有意不绑**：上游单行 `Input` **无条件**绑定并处理 `left/right`，真源靠 NSEvent 全局监听连光标键一起吞，gpui 侧要那样得上应用级 `intercept_keystrokes`（会全局吞键），收益不抵风险。**未做**：**Classes** 独立 scope ✅ **已做（2026-10-03 后续切片）**：`SearchEverywhereScope` 加 `Classes`（位置照真源 `allCases`：All → Classes → Files → Symbols → Text），投影直接复用既有 `MatchKind::is_type` ⇒ 没有第二套匹配；`All` 口径未动（仍 file+type+symbol、不含 content）；新增 2 条文案键；workbench 147 → **148**。✅ `Actions` scope 与 `/` 命令模式**已实现（2026-10-03 后续切片）** ⇒ Search Everywhere **六档齐 + `/` 模式**，workbench 148 → **156**。`Actions` 的匹配器落宿主侧一个纯函数（**依据**：上游 `item_matches` 是私有 `fn`，且它匹配 label/keywords 子串、与真源的 title+subtitle+group 子序列不是同一条规则；Core 的输入里没有命令目录）—— 所以不是"第三套匹配器"；"Actions 档不发 Core"有三层证据（纯函数判据 / `CoreClient::execute` 全文件只有一处且在闸门之后 / 新增无头窗口交互测试带**对照组**）。**与真源的有意差异**：宿主命令目录实测 **22** 条 vs 真源 **39** 条（只可能是子集）、不画 keyEquivalent 列（宿主没有"id → 快捷键文本"通路，硬编就是第二份键位真源）、分类列是宿主自己的两类、图标用动作自带字形而非真源统一的 bolt。仍**未做**：相关度重排（=B10 第二条，等 main）。**只读评估的原文保留**：真源**不合并两个来源**（每个 (mode, scope) 只出一个来源），Actions 需新行型 + Enter 走 `CommandId` dispatch + 复用上游 `Command::item.matches`（否则是第三套匹配器），成本≈3 文件、宿主命令目录 24 条 vs 真源 39 条属子集；`/` 模式在 Actions 落地后≈零额外成本（同一投影 + 纯前缀解析 + 2 条文案 + 一个提前 return，且真源 scope 行不隐藏）、**Actions**（宿主命令目录，另一件事）、**`/` 命令模式**、**相关度重排**（=`BLOCKERS.md` B10 的第二条，等 main 在 Core 给 relevance）、Replace in Files（`workspace.replacePreview`）；**搜索历史也没持久化**。⚠️ 菜单里剩下的「全局搜索」占位项仍显示"还没有索引器"，但**索引链早在 U18 就通了**（缺的是那层浮层）—— 文案与事实不符，已登记在 UI-MAP §2.2 ⑬。**两处有意偏离 macOS**（已写进 UI-MAP §2.2 ⑬）：三个选项画成纵向 `Checkbox` 而不是真机滑块图标的下拉菜单（侧栏可拖到 200px、三个文案一行放不下，且不引入浮层）；不画 macOS 那行 44px 的「Search」标题（与 2026-09-27 删资源管理器标题行同口径）。⚠️ **`U18` 链上仍缺"索引就绪"状态**：项目刚打开、warm 未跑完就搜索时，第一次 `workspace.search` 会在后台等索引锁而变慢（界面不卡）；本面板**不另建索引状态**（第二个真相源），要补该补在 `search_index.rs` 那条链上。⚠️ 结果契约只有 `kind/path/line/preview/symbolName`（无列号），所以点结果只跳到**行首**。⚠️ **未经真机视觉验证**（Find in Files 与 Quick Open 的浮层观感、真按键都只有代码与静态证据） |
| ~~**U18**~~ ✅ | 搜索索引 warm/update/invalidate 的调用时机 | — | **已完成（U17 的界面仍未做）**。新增 `crates/workbench/src/search_index.rs` 作为三条 `workspace.searchIndex.*` 的**唯一 owner**：**warm = 项目打开**（`ShellWorkspace::new` 是所有打开路径的公共收口 ⇒ 结构性保证"每根恰好一次"，走 `cx.background_spawn` 不在 UI 线程建索引）；**update = 保存写盘**（新增 `BufferWriteHost` 宿主钩子，编辑区只报"哪个文件落盘了"，外壳收到后只带**那一条**工作区相对路径 —— 不退化成"每次保存就 invalidate 全量"）；**invalidate = 换根**（`rebuild_project_window` 在 `replace_root` 之前）。带代次闸门（`accepts`）与 stderr 诊断锚点 `S1_SEARCH_INDEX <op> root=… files=… symbols=… postings=… rebuilt=…`。**如实登记的没接上的**：git 的写盘路径（discard/checkout）没接、项目结构变化（新建/删除文件）不触发（gpui **无 watcher**，H12 未做）、**没有"关闭项目"这条生命周期**所以 invalidate 只落换根；**U17 的搜索界面一行未做**。5 条确定性单测（含真夹具上的**真实 Core 往返**：warm(1 文件) → 新增 → update 断言 `rebuilt=false fileCount=2` → invalidate），不依赖网络/真实进程/时钟。⚠️ warm 的**真机端到端效果未经真机验证**（本环境未跑 `-p lithe-gpui-app`）|
| **U19** | 调试 UI（调用栈/变量树/监视/求值/断点/单步/步进过滤器） | `Views/Debug/` 2693 | **Core 18 条命令全有**、`debug/engine.rs` 3886 行；gpui **调用点 0** |
| **U20** | 编辑器补缺：外部修改冲突横幅、编码探测与重开、分屏、标签拖拽、Markdown/HTML/SVG/媒体预览、转到行列、软换行开关、Code Vision、inline blame、inlay hints、悬停文档、自动保存设置化 | 分散 | 数据侧部分在 Core（`git.blame`、`git.diff`、LSP `hover`/`semanticTokens`）；上游 gpui-base 已有 hover/semantic 的缝，**宿主未装 provider** |
| **U21** | 数据库界面 | `Views/Database/` 5255 | 依赖 C30–C32 |
| **U22** | AI 提交信息/PR 描述界面 + provider 设置页 | macOS 侧 2111(GitHub) + AI 相关 | 依赖 C36/C37；Core 已有 `plan_commit`/`decode_message`/`parse_codex`/`parse_claude` |
| **U23** | Agent 对话面板 | `Views/Agent/` 3204 + `LitheAgentConversationModule` | Core 有 `lithe_agent_*` C ABI + 4 条管理命令，但 **gpui 无 Rust-native 调用面**（`mod agent` 私有、gpui 不依赖 `lithe-agent-host`）→ 先定异步边界（ACP 是长连接事件流，而 `CoreClient` 是同步信封） |
| **U24** | 资源管理器补缺：空目录显示、watcher 联动、右键菜单/多选/拖拽、Reveal Active File、Git 状态装饰、手动刷新入口 | `ProjectSidebarView.swift` 1029 | 空目录需 `data.root`（**Core 已有**，gpui 只用了 `data.files`）；`git.status` 已有 |
| **U25** ⏳ | 外壳度量对齐 macOS：标题栏 68→40、项目标签条 32/28→38/30 + 3 个信息元素 + 横向滚动、底部/右侧工具窗可拖 + 宽度持久化、通知 toast 位置与"最多 3 条/hover 暂停"、状态栏 4 项占位 | `LitheTheme.swift:477-480` 等 | ⏳ **部分完成**。**已做**：① **标题栏 68 → 40 单行**（真源 `Metrics.toolbarHeight = 40`，`LitheTheme.swift:478`；原先是 30+38 两行）；② 标题栏内的运行配置槽位 30/min160/max190、运行按钮 **28×28** + 图标 **16**、项目下拉触发器 30 + pl6/pr10、分支项 32 + px9 —— **字体/图标/padding/hit target 一起对齐**（`design-guides.md:208`）；③ **项目标签条 38/30**（真源 `tabHeight(34)+4`，`WorkbenchView.swift:653`）、标签按可用宽度**等分** + `min_w(180)`、图标 11、关闭按钮 28、圆角对齐、**新增横向滚动**（`overflow_x_scroll` + 切项目 `scroll_to_item`，内衬放在滚动区**内部**）；④ 通知 toast 锚点 `BottomLeft → BottomRight`（真源 `WorkbenchView.swift:517` 的 `.bottomTrailing`）；⑤ **收掉 D23 移交的 3 条 workbench 欠账** —— 负外边距 `-WORKBENCH_GAP_SPEC` 补丁 → 改由 **hairline 的唯一所有者**表达（`design-guides.md:188/:196`）、6 处硬写 `6.4` → 收敛到 `shared::design::CONTROL_RADIUS`、`ROW_RADIUS` 的行号引用 → 改符号式。**未做（如实登记）**：① 标签条另 **2 个**信息元素（当前文档名后缀、agent 注意力圆点）—— `ShellWorkspace::projects` 恒为**单根**、这条标签条当前根本不渲染，**没有诚实的数据源，加字段就是死代码**；② **状态栏 4 项** —— 真源已查清（`WorkbenchView.swift:1740-1802`：光标/编码/缩进/只读/准备状态/内存/帧率/git 状态），本轮限定"不接真实数据源"，而"加占位"与"不许假装能用"直接冲突，**选择不动**（不新增假数字、也不删既有格）；③ ~~**底部/右侧工具窗可拖 + 宽度持久化**~~ ✅ **已做（2026-10-03）**：三块分栏都能拖（左栏原本就能；右工具窗补水平手柄、底部工具窗补**垂直**手柄，上游 `HandleEdge`/`Axis` 支持垂直），尺寸落进会话文档的三个**可缺省**字段（`leftPaneWidth` / `rightPaneWidth` / `bottomPaneHeight`，旧文件缺字段即回落默认、**版本号未动**、文档层刻意**不钳制**——钳制是宿主规格）；拖动期间**零落盘**，只在手势结束走既有 `SessionTracker` 300ms 去抖保存；min/max 取真源（右 300–760 + 中间保留 640、底部 min 260 + 上方保留 220）。**顺带修掉一个既有真缺陷**：`SessionTracker` 的快判键原先只有 (打开文件, 当前文件)、**不含界面状态** ⇒ 任何"只改界面状态"的调用都被判 `Unchanged`，拖出来的宽度**永远写不出去**（切换视图 / 收起侧栏同样受影响）。**遗留 4 条待裁决**：(a) 底部默认常量仍写 240 而真源下界 260 ⇒ 首帧实际渲染 260、常量与下界不自洽（把常量改成 260 是**视觉无变化**的，但那条常量带维护者墓碑注释，未擅动）；(b) 左栏仍用静网 200/720，未改成真源 `30 / min(520, avail−400)`、也没做窗口自适应；(c) 右栏默认仍 **400**，而它的出处 `default-settings.ts:139` 属**已作废的 windows/tauri**，真源是 **360**；(d) 上界取的 `window.viewport_size()` 是**窗口内**尺寸、没减活动栏/标题栏/标签条/状态栏，比真源量"分栏容器"略松，属近似。(e) 左活动栏 140–320 拖拽仍不在范围内；④ **toast 最多 3 条 / hover 暂停**（真源已核实 `WorkbenchNotificationFeatureModel.swift:5-7,38-41,53-74,112`，但该语义在 gpui-kit 通知层内部，查 3 次未确证上限定在，按纪律停手）。**任务表数字与真源冲突已更正 3 处**：红绿灯留白真源是 **83**（非 76，`WorkbenchView.swift:30`，注释里 IDEA 用 78，且本侧按"不写平台业务分支"**不预留**）、标签可见信息元素真源是 **4 个**（非 3）、标题栏 40 与真源一致但 **macOS 菜单栏在系统菜单栏上而 Windows 没有等价物** ⇒ 单行 40 意味着菜单与项目/分支/运行控件**同排**。⚠️ **未验证风险（真机复核请优先看）**：窄窗口下这一行里固定宽子项之和（菜单 + 运行槽位 min160 + 两个 28 按钮 + 3×44 窗口键…）可能超过可用宽度，先被挤到边界的是菜单栏/运行槽位 —— 本轮**没有**写宽度分支或收窄常量。⚠️ 全部改动**未经真机视觉验证**；底部工具窗维持 240（真源最小高 260 比它大 20）是**已知差距** |

### 4.3 技术债（小改动、高收益，可插队）

| ID | 任务 | 落点 | 为什么优先 |
| --- | --- | --- | --- |
| ~~**D10**~~ ✅ | 运行启动别阻塞 UI 线程 | **已完成**：`RunSession::start` 拆成 `prepare`（取计划 + 跑 `preLaunchSteps`，**阻塞**，放进 `background_spawn`）+ `start`（只做 `spawn` + 建读线程，毫秒级，可在 UI 线程）；`ToolchainResolver` 因此补上 `Send + Sync`（外壳实现是纯查表，本来就满足） | `preLaunchSteps` 同步跑，javac 期间**整窗卡死最长 300s** |
| ~~**D11**~~ ✅ | 状态栏别每帧读 `.git/HEAD` | **已完成**：删掉 `workbench::read_branch`，改用 `ChangesView::branch()` —— Git 视图已持有的 `git.status` 快照，**每帧只做内存读取、没有 I/O**；worktree / 分离 HEAD / 仓库根≠工作区根这三处读错交给 Core 判定 | 每帧一次文件 I/O，且 worktree/分离 HEAD/仓库根≠工作区根**全读错**；应改走 `git.status` |
| ~~**D12**~~ ✅ | 修 `searchable` 反向注释 | **已完成，但本行原判断是反的**：0.7.0 的 `EditorState::new` **确实**把 `searchable` 设成 `true`（`gpui-base-0.7.0/src/input/base/state.rs:10085` → `:10088`；`:709` 的 `false` 属于 `InputBaseState::default`，是给单行 `Input` 用的），所以**原注释是对的**。本轮真正修的是：陈旧路径 `0.6.6` → 0.7.0 实测位置，并补上"删掉这一句会**静默**失去 Ctrl+F/H"的后果 | 后人"清理冗余调用"会**静默丢掉 Ctrl+F/H**（这一点原来就成立，只是理由写错了版本） |
| ~~**D13**~~ ✅ | 状态栏只读/缩进格接真值 | **已完成**：新增 `EditorPane::tab_size()` 与 `EditorPane::active_buffer_is_readonly()`；`footer_right` 的缩进格与只读格改读它们，只读图标在 `LOCK_ICON` / `LOCK_OPEN_ICON` 之间切换 | `is_readonly()` 上游已有（`gpui-base-0.7.0/src/input/base/state.rs:509`），`tab_size` 宿主自己有（`editor_view.rs:397`） |
| ~~**D14**~~ ✅ | 文档/注释里的死引用 | **已完成，但本行原给的三处坐标全部过期**（`UI-MAP.md:141` 实测在讲"不写平台业务分支"；`explorer/src/lib.rs:28-29` 与 `model.rs:214` 也都不是问题所在）。真正修的是：① `UI-MAP.md:223` 指的落点函数 `render_editor` **不存在** → 改为 `EditorPane::render_body` 的编辑器包裹层（`on_scroll_wheel` 接线处）；② `explorer/src/lib.rs` 的「空目录挂占位行」注释**与代码不符** —— 树只从 `data.files`（扁平**文件**路径）聚出，Core `project/files.rs` 的 `snapshot` 只收非目录项，空目录不会成为树里的行，可达的只有根行那条；③ `git/src/branch_info.rs` 两处 + `editor/src/navigation.rs` 一处 `model.rs:<行号>` **已漂**（`load_first` 750-767→**774**、`read_status` 493-500→**538**、`load_snapshot` 236-242→**236**）→ 一律改成「文件 + 符号名」这种不会被行号漂移打破的写法；④ `editor/src/editor_view.rs` 的 `git HEAD panels.rs::open_document` **补上"该文件已删除"交代**（`buffer.rs` 那处本来就有）。`run/src/model.rs` 两处行号**未漂，保留**。另修 `UI-MAP.md` §1.1 的一处**过期状态**（原写"仍有约 57 处 `px()`、`rem_px` 复制在 5 个 crate、`shared` 没有共享版"——D16/D17 已完成后这三句全错） | 一行级，但收益是后续核对不再踩空；`UI-MAP.md` 已无"未被交代的不存在文件引用" |
| **D15** | 清掉 gpui 里的 `windows/tauri` 规格残留 | 约 178 处注释（workbench 49、git crate 头部整份、explorer、terminal、editor） | 规格已定 macOS；不清掉，阶段二/四每条都要重新论证一次 |
| ~~**D16**~~ ✅ | **把重复的 `rem_px` 换算收敛成一份** | **已完成**：唯一实现 `crates/shared/src/lib.rs` 的 `rem_px`（+ 随基准缩放的 5 条断言单测）；`editor` / `git` / `settings` / `terminal` / `workbench` 五处改为 `pub(crate) use lithe_gpui_shared::rem_px;` 转发（共 6 份定义、3 份重复单测 → 1 + 1） | 同一套换算原先写 5 遍，任何一处改口径（16 还是别的基准）都会分叉；`shared` 是跨 crate 原语该在的地方（`icons` / `i18n` / `core_client` 已在那里） |
| ~~**D17**~~ ✅ | **清掉生产代码里直接写的 `px()`，改用 rem-based 助手** | **已完成**：终态 2 处违规（`terminal_view.rs` 的 `row_font_size` / `self.font_size`）改为 `rems(size / 16.)`（设置值是规格 px，`Styled::text_size` 收 `AbsoluteLength`）；守卫 `node lithe-gpui/tools/check-ui-px.mjs` 实测 **exit 0 / 违规 0**。║ **口径更正**：本表原写"生产代码约 57 处"，实测**不是** 57 —— 那是把注释/字符串里的 `px(` 与测试断言一起数进去的结果。守卫的实测分解（`--list`）：`px(` 子串 214 处（改动前）= 注释/字符串 66 + 形近调用 82 + `#[cfg(test)]` 内 53 + **真正的 UI 代码调用 13**（其中白名单 11、违规 2） | 写死 px 会让"用户改界面字号"时不缩放。守卫只认 `coding-guides.md:288` 的四类例外，白名单 7 条（4 条 OS 窗口平台边界 + 3 条 `Theme.font_size`/`mono_font_size` token 定义本身） |

### 4.4 官方指南衍生的债务（来源：`docs/gpui kit/0.7.0/docs/{coding,design}-guides.md`）

> 这两份是 gpui-kit 的**明文规范**（§1.4 是摘录）。下面每条的"现状"都是实测，不是推测。

| ID | 任务 | 指南条文 | 本项目现状 |
| --- | --- | --- | --- |
| **D18** ⏳ | 大面积手写高度 → 改用组件 `Size` API | 「优先使用组件尺寸（`xsmall`/`small`/`medium`/`large`），**不要单独写高度**」（`design-guides.md:115`）；「使用组件 `Size` API，让字体、图标、padding 和 hit target 一起变化。**只改外框高度通常是不完整的**」（同文件 `:208`） | ⏳ **进行中：本轮做了 ⑥ Git 区域**。盘点 25 处手写定高（`h_5()–h_12()`）：**(a) 组件被手写高度硬撑 6 处已改**（`changes_view.rs:897/1072/1084/1097/1455`、`log_view.rs:730` → `.small().compact()`）、**(b) 纯布局元素 18 处不改**（`h_flex()`/`div()` 撑的工具条、表头、分类头、胶囊、行高；`Size` 对它们不适用）、**(c) 0 处**、**(d) 待裁决 2 项**（① `changes_view.rs:1575` 提交按钮 `.h_7()`=28px 不在 `Button` 档位（24/32）内，且 macOS 侧 `CommitAreaView.swift:96` 走 `LithePrimaryButtonStyle()` 默认值、没给显式高度，无法确证该取哪档；② `LOAD_MORE_BUTTON_HEIGHT` 的三处 `Size::Size(24px)` 在 `Button` 上**只**产生 `px(size×0.2)` 内衬、不设外框高度，实际高度由行高决定，换 `Size::Small` 会同时改字体与内衬）。要害是用**档位**而不是只压高度：改前那种 `.h_6().px_1p5()` 会留下 `Medium` 的 `text_base`(16px) 字号（`Button` 末尾 `refine_style` 只让调用点赢外框与内衬，压不掉字体与 gap），改成 `small`（24px 外框 + `text_sm`）后字体与 gap 随档位落下，**外框 24px 与内衬 6px 与改前逐像素相同**；24px 亦与真源 `controlSize(.small)` / `LitheTheme.Commit.compactButtonHeight` 一致。顺带删掉因此成为死常量的 `ICON_BUTTON_SIZE` 与其**已被证伪**的理由注释。**其余区域**（按本条"不要做成一次全仓替换"）：⑨ **终端已做（2026-10-03）** —— 9 处出现里 4 处是注释、真实调用点 5 处：(a) **2 处改**（空态「新建终端」与失败态「重试」的 `.h_6().px_2()` → `.small()`，外框 24 / 内衬 8 逐像素不变、字号 16→14 与 gap 8→4 随档落下）、(b) **3 处不改**（页签条 / 空态页签条 / 状态行的纯布局带）；⚠️ **终端的字符行高 `TERMINAL_LINE_HEIGHT` 是等宽网格几何契约，单列红线不改**；范围外另有 4 处已是 rem 档位的方形图标按钮 (c) 与 2 处待裁决 (d)（状态行「重试」20px 恰在 `Size::XSmall` 上但换档会同时动 padding 与字号，且 macOS 侧无此按钮；输入行 28px 不在 `Input` 档位 20/24/32/44 上，与 ⑥ Git 的 `changes_view.rs:1575` 同形）。③ **项目树已做（2026-10-03）** —— 7 处出现里 4 处注释、真实 3 处：(a) **1 处改**（空态动作按钮 `.h_6().px_1p5()` → `.small().compact()`）、(b) **2 处不改**（布局行内容与 `ListItem`；⚠️ `uniform_list` **只量第 0 行**再按固定值铺满，24px 是**行高契约**）。④ **编辑区收口（2026-10-03，零代码改动）** —— 文本 3 处 / 真实调用点 1 处：`editor_view.rs:2478` 的 `.h_9()` 挂在 **`TabBar`** 上，而 `TabBar` **自身没有高度**（无标签时会被压成 0），`.h_9()`(36) 是"无标签也占位"这条**布局要求**的实现 ⇒ (b) 不改；⚠️ 编辑器**行高 / 字符单元格几何**是行为契约、红线不改。⑤ **设置收口（2026-10-03，零代码改动）** —— 文本 2 处 / 真实 1 处：`dialog.rs:1285` 的 `.h_11()` 是**纯布局带**（不是组件实例，`Size` 不适用；44 恰等于 `Size::Large` 的 `Input` 档位，但那是 Input 不是 Button）⇒ (b) 不改；dialog.rs 约 20 处 `.small()` 本就落得干净 (c)。**两个区域 (a) 均为 0 处，故没有 Rust 改动**。**① 工作台外壳已收口（2026-10-03，D18 最后一个区域，(a)=0 ⇒ 零代码改动）** —— 33 行命中里 22 行是注释、真实 **11 处**：10 处接收者是 `h_flex()`/`div()`（`Size` API 不适用），第 11 处 `status_bar.rs:238` 的接收者是 **`StatusBar` 组件、而它没有 `Size` API**（结构体只有 `style`/`left`/`right`/`children`，仅 `impl Styled`）⇒ **手段不存在**；`quick_open.rs` / `search_everywhere.rs` 的两处结果行高是**契约**（且机制已更正：`Command` 是**逐行**测量后把 `row_sizes` 交给虚拟列表，不是"只量第 0 行"）。**D18 至此六区域全部收口**。**仍未做**：（无剩余区域）手写定高的**当前实测计数**（口径：`crates/*/src/**`、不含 `tests/`、排除**整行**注释/文档、`\b` 把 `min_h_8()` 这类"最小高"排除；括号内 = 真实调用点）：`workbench` **33（11）** / `git` **36（19）** / `terminal` 11（3）/ `explorer` **9（2）** / `editor` 3（1）/ `settings` 2（1），`shared`/`app`/`extensions`/`java`/`notify`/`run` 为 0 —— 详见 UI-MAP §1.4 A 组 |
| ~~**D19**~~ ✅ | 核对缓存失效键是否含 `window.rem_size()` | 「从 resolved layout 得到的 cache **必须把 `window.rem_size()` 纳入 invalidation key**，**或者依赖随它变化的 revision**」——换行行高、text shaping、virtual-list 测量、popup 几何、由文字推导的图标尺寸 | **已核：项目侧没有"缺 `rem_size` 失效键"的缓存缺陷**。真正的跨帧缓存只有 3 处触发器 `Bounds<Pixels>`（`project_menu.rs:992`、`menu_bar.rs:1716`、`:1718`），三处 `on_prepaint` **无条件逐帧覆写** ⇒ rem 变化经 `refresh_windows()` 出新帧就重测；`uniform_list` 每次 `request_layout` 重测；`Command` 的 `ListMeasurementKey` 显式含 `rem_size`；text shaping 走上游、键含 `font_size`；popup/dialog 几何每帧现算。**唯一真缺陷**（不同类，但同属"改字号后仍用旧值"）：`lsp.completion_menu.max_width` 被上游每帧读（`completion_menu.rs:383`）却只在 `open()` / `prepare_java()` **一次性**写入 ⇒ **已修**（见 §7 的 D19 行）。另闭环了"上游 `DisplayMap` 缩放后是否陈旧"：每帧 prepaint 重调 `set_font` 并按需 `update_all` 重排，**无陈旧软换行边界** |
| ~~**D20**~~ ✅ | 核对"先 `bind_keys` 再 `set_menus`"的顺序 | 「`cx.set_menus` 在调用那刻读取 keymap；之后注册的 binding **不会**出现在菜单项旁，菜单项也不响应该按键」 | **已完成，结论是"不适用"**：`rust/lithe-gpui` 全仓**零处** `cx.set_menus` 调用（`gpui-pre-0.3.7/src/app.rs:2530-2532` 确认该 API 确实在调用那刻把 keymap 交给平台，但本项目没走这条路）。菜单栏**自绘**（`menu_bar.rs:31-44` 已论证 Windows 无 `SetMenu` + `AppMenuBar` 24px/gap 硬编码两条否掉 native 路线），下拉面板用 `PopupMenu`，快捷键胶囊在**渲染期**从 keymap 反查（`gpui-component-0.7.0/src/menu/popup_menu.rs:1301` → `:1119-1144` 的 `Kbd::binding_for_action_in`）⇒ 不在构造时固化，因此后注册的 binding 照样显示。**六处 `bind_keys` 也全在窗口与菜单首帧之前**：`app/src/main.rs:1030`（→`settings/dialog.rs:101`）→ `:1051`（→`workspace.rs:3689`）→ `:1059`（→`menu_bar.rs:620`，6 条）→ `cx.open_window`（`:1070`）内的 `ShellWorkspace::new`（`workspace.rs:1137/1141/1145` → `editor/lib.rs:244`、`git/lib.rs:222`、`command_palette.rs:309`）。「之后改 binding 需再调 `set_menus`」在本项目**当前不暴露**（`clear_key_bindings()` 0 处、无用户 keymap 文件、面板每次展开现建）。**未改任何 `.rs`**，结论已写回 `UI-MAP.md` §1.4 N 与 §1.4 C 组那条重复行 |
| **D21** | 核对是否"一个逻辑命令只建模一次" | 「Toolbar Button、菜单项、ContextMenu item、menu-bar item 与 key binding 应 dispatch **同一个 Action**，不能复制五份 mutation」 | 与 §2.2 ⑥ 的"引用行右键 17 项 / 提交行右键"实现方式直接相关：**新做这些菜单时不要各写一份 mutation** |
| ~~**D22**~~ ✅ | 核对 `Link` 的使用边界 | 「`Link` **只**用于交给浏览器或邮件客户端的外部目标；应用内导航与命令用 `Button`/`Action`」 | **已完成，结论 = 零使用点、无违规可修**。实测：`crates/*/src/**` 全部 **99 个 `.rs`** 里 `Link` / `.href(` / `open_with` / `cx.open_url(` **均 0 处**（唯一的 `Link` 字样是图标名 `externalLink.svg` 与两处注释）。补强判据：`coding-guides.md:347` 明说这是**产品设计约定、不是 `gpui_kit::base::Link` 的能力限制**（base 版要靠 `open_with` 注入导航，`gpui-base-0.7.0/src/link.rs:80` 写明"Base never calls `App::open_url` on its own"）；`design-guides.md:358/362` 给出理由（`Link` 暴露 Link role + 手形 cursor，当应用内按钮用会隐藏 control affordance 并暴露**错误 role**）；且**外部目标用 `Link` 不是必须** —— 指南自己的外部打开示例就是 `Button` + `cx.open_url(...)`（`context.md:43-45`）。⇒ 未改任何 `.rs`；结论已写回 `UI-MAP.md` §1.4（该规则在 UI-MAP 里只有一处状态单元格，无重复行） |
| ~~**D23**~~ ✅ | 精确对齐不变量落地 | 「两条 edge 或两段 spacing 按设计应相等时，**相差一个 rendered pixel 也是缺陷**」（`design-guides.md:186`；`:188` 给出机理：rem zoom 与 display scale 会把 fractional 坐标放大成整设备像素漂移，所以"只在某一档看着齐"等于没对齐）；相等关系必须来自**同一个** rem helper / spacing token / shared inset，**不能用 raw pixel offset 修补** | **已完成（可改区域）**：查 5 类"该相等"关系 + 机器扫描 13 个 crate 的 `const NAME: f32`。**raw pixel offset 补丁在可改区域 = 0 处**（唯一 `+ 1.` 命中是 `editor_view.rs` 的索引区间）；38 处 `border_*_1()` 是 `coding-guides.md:288` 的 **one-device-pixel hairline 合法例外**（`snap_border_widths` 贴设备像素），**不是** offset 补丁 —— 别误报。**收敛 3 处到 2 个 owner**：① `git` 内 7 个圆角常量原先各自硬写 `6.4`/`4.8` 且 `ICON_BUTTON_RADIUS` 在 `model.rs` 与 `changes_view.rs` **同名同值两份** → 只留 `RADIUS_BASE`/`SM`/`MD`/`XL` 一份派生（`git/src/model.rs`）；② `ROW_RADIUS`(4) + `ROW_LINE_HEIGHT`(1.35) 由 `explorer` 与 `git` 各写一份 → 新建 `crates/shared/src/design.rs`；③ `TAB_MAX_WIDTH`(200) 由 `editor` 与 `terminal` 各写一份 → 同上。**待裁决 2 处（未改）**：`git/log_view.rs` 提交表头不预留泳道槽（macOS 根本没这个表头，规格值无从确证）、同一个 `rounded-md` 有两个 owner 且值不同（`settings` 走 `cx.theme().radius`=6px 固定 `Pixels` 不随 rem 缩放 vs `git`/`explorer` 走 6.4 随 rem 缩放 —— macOS 的 `controlCornerRadius = 6` 与 Lithe 阶梯 6.4 **本身冲突**）。**workbench 发现 3 条但未改**（已转 U25）：负外边距 `-WORKBENCH_GAP_SPEC` 消"两条平行线夹一条缝"（按指南该修 hairline 所有者）、6 个文件各写一份 `6.4`、`right_tool_window.rs` 对 `ROW_RADIUS` 的行号引用漂移。⚠️ 相等性依据是**结构性派生**（同一个 const/token），**未经真机视觉验证**；`coding-guides.md:400` 要求的 resolved-bounds 断言本轮未加 |
| **D24** | 界面文案过一遍用词规范 | 「入口/对象用名词、命令用动词、状态用形容词」；按钮说明**结果**；`OK` 只用于纯信息确认；打开对话框的命令加 `…`；英文 sentence case；单省略号 `…` | 文案真源是 `macos/Resources/zh-Hans.lproj`，复刻时顺带按此校对；§2.2 ①的"状态栏 4 项假值"也属这类 |
| ~~**D25**~~ ✅ | 评估是否还需要嵌 `AllAssets`（**约 1.02 MiB**） | `assets.md` 的嵌入量对照：默认组件图标 **101 个 = 44.28 KiB（基线）**，**`AllAssets` 1830 个 = 731.45 KiB，+1.02 MiB**；并明确"**依赖图标 crate 不等于嵌入全部图标**，只有显式注册 `AllAssets` 才会" | **已评估，结论 = 保持现状、明确否决收窄（2026-10-02 维护者定）**。统计结果：界面实际引用 **70 个图标名** → **26 个在组件子集里**（`check`/`folder`/`settings`/`github`…）、**44 个只有完整 Lucide 目录才有**（`app-window`/`git-graph`/`tag`/`sparkles`/`file-code`…，合计 17 271 B）、**0 个**不在图标 crate 内 ⇒ **`AllAssets` 目前是承重的**：删掉兜底会让这 44 个 + 全部 gpui-kit 组件内部图标（Dialog/Sheet/Input 的关闭、Select 的下拉箭头等）静默空白。收窄技术上可行（44 个名 100% 静态可枚举，`IconName::X` 全是枚举字面量、无运行期拼名，唯一的字符串通道 `shared/src/icons/file_icon.rs` 的 `lucide_fallback` 也是静态 match），改法就是指南的 `icon_assets!(ExtraIcons, [44 个])`，原始 SVG 从 749 008 B 降到 63 620 B、**估算省约 0.94 MiB**（按指南唯一实测点 +1.02 MiB 推算，未实测二进制增量）。**但仍决定不做**：它是"改内嵌资源"的独立改动，且真正的风险不是动态名而是**静态名漏登记**——漏一个不会有任何编译/测试失败，只会静默画空白。若要重开，**先补一条"70 个名逐个 `LitheAssets.load(name.path())` 必须 `Some`"的回归判据**，再动 `assets.rs`。口径更正：0.7 的默认子集实测 **104 个 / 46 349 B**（指南表的 101 / 44.28 KiB 是 0.6 旧数）。**未改任何代码** |
| ~~**D26**~~ ✅ | **引入 gpui-kit 的 UI 集成测试层** | **已完成**：落点 `crates/workbench/tests/`（`ui.rs` + `common/mod.rs`，夹具形状照 `gpui-kit-0.7.0/tests/common/mod.rs` 抄）。**不用新建 `ui-tests/` crate**：上游 `test.md §配置测试项目` 里那个独立项目是因为上游要测自己的 Kit 源码检出；本仓库把 `gpui-kit = { version = "0.7", features = ["test-support"] }` 放进 `crates/workbench` 的 `[dev-dependencies]` 即可 —— 同一个 registry 来源与版本，feature 正常合并，**不需要 Cargo patch**（`Cargo.lock` 只多了 test-support 链上的 proptest 等 10 个包）。已验证**不泄漏**：`cargo tree -p lithe-gpui-app -e normal` 里 gpui-kit 的 feature 是 `assets,component,default,tree-sitter-*`，**没有 `test-support`**。首条测试用**真实** `EditorPane`：开真实临时文件 → 渲染一帧 → 断言 `tab_size()` / `active_buffer_is_readonly()` / `active_buffer_name()` / `set_tab_size(8)` 生效 | 见左列；后续 §2.2 那批"只能靠真机手测"的交互缺口（底部工具窗拖拽热区 `drag_to`、浮层焦点约束与归还、菜单键盘路径、Dock Tab 重排、表格/树虚拟滚动）现在有落点了 |

---

## 5. 阶段五：验收与清理

| ID | 任务 | 验收 |
| --- | --- | --- |
| **V10** | 补 `UI-MAP.md` §2 的逐区域对照表 | 现在 §2 是**空章节**（只有说明、无表格），而 §3 验收第 3 条要求"每个元素都能查到 macOS 出处 → gpui 实现"——自相矛盾。补表或改验收条目 |
| **V11** | 更新 `develop-lithe` 的所有权表 | 加一条判据：**确定性逻辑进 Core，平台能力进宿主端口**；并列出"Core 命令面 / 宿主端口"的归属清单 |
| **V12** | 逐区域完成后跑仓库校验 | `cargo test --manifest-path rust/Cargo.toml -p lithe-core`、`node scripts/verify-agent-notes.mjs`、新增的 C6 守卫 |
| **V13** | 修 `verify-*.sh` 在本机的可运行性 | 现在 `scripts/verify-rust-core-comments.sh`、`verify-shared-contracts.sh` 都因缺 `rg`/`ruby`/`awk` 而**跑不起来**（`find` 还是 Windows 自带的）。要么提供 Node 版，要么在文档里写明本机限制 |

---

## 6. 依赖关系与建议顺序（**已按"本分支不改 Core"重排**）

```text
⛔ 以下全部由 main 负责，本分支不执行：
   C1/C2（git 排除双 operation）· C3/C4/C7（目录 v2）· C5/C6（契约守卫）
   C10..C18（算法搬 Core）· C30/C31/C32/C34/C35/C36（新契约）

──── 本分支从这条线开始 ────

D16（rem_px 收敛）─────► 无依赖，可最先做（纯重构，行为不变）
D17（清 px()）─────────► 建议在 D16 之后（先有统一换算，再清 px）
D10..D13 ─────────────► 随时可插队（都是小改动、明确的 bug）
D14/D15 ───────────────► 清死引用与 tauri 残留，成本极低
D20..D24 ─────────────► 指南条文核对；多为读代码就能定论
D26（UI 集成测试层）───► 越早接越省事：后面每个 U 系列都能拿它验收

C19/C20/C21、C33、C37 ─► 落点在 gpui，可做（不再需要先动 Core）

H10/H11（调试 transport）► 依赖"异步边界"决策（U23 同一个问题）
U17/U18（搜索）────────► U18 必须先于 U17
U19（调试 UI）─────────► 依赖 H10/H11
U21（数据库界面）──────► ⛔ 阻塞：等 main 提供 C30 的命令面
U22（AI 界面）─────────► ⛔ 部分阻塞：等 main 提供 C36
U23（Agent）───────────► 依赖"ACP 长连接 vs 同步信封"的架构决策
```

**建议的开工顺序（本分支）**：
`D16 → D17`（单位口径，纯重构、零行为变化）
→ `D10、D11、D12、D13`（清明确的 bug，成本极低）
→ `D26`（接上 UI 集成测试，后面用它验收）
→ `D20、D22、D23`（指南条文核对，多为读代码就能定论）
→ 之后按容量在 U 系列里挑一个区域做透（**做每个区域时顺手带 D18、D24**）。

**与 Core 相关的 C1–C4 / C10–C18 / C30–C36：不排期，只上报。**

> **为什么 D16/D17 仍是第一**：它们是全仓口径类改动，越晚做冲突面越大（每个新写的界面都要跟着改一次）。
> 而且 D16 是纯重构（把 5 份 `rem_px` 合成 1 份），不改行为，风险最低。
>
> **D18 不单独排期**：它天然属于"做某个区域时顺手改"（哪个区域动到定高就在那里用组件 `Size`），
> 单独做成一次全仓替换反而容易只换高度、不换字体图标。

---

## 7. 每做完一条要更新这里

- 状态：`待办` / `进行中` / `已完成` / `已否决（原因）`

**已完成（本分支）**

| ID | 状态 | 验证证据 |
| --- | --- | --- |
| **D10** | ✅ 已完成 | `cargo test -p lithe-gpui-run` 16 条通过；拆点判据：阻塞的只剩 `prepare`，`start` 只 `spawn` |
| **D11** | ✅ 已完成 | `cargo test -p lithe-gpui-workbench` 82 条通过；`read_branch` 已从代码里消失（grep 0 命中） |
| **D12** | ✅ 已完成 | 注释改为 0.7.0 实测行号；**并更正了本表原来写反的判断**（见 §4.3 的 D12 行） |
| **D13** | ✅ 已完成 | `cargo test -p lithe-gpui-editor` 52 条 / `-p lithe-gpui-workbench` 82 条通过；两格改读 `EditorPane` 的真值 |
| **D14** | ✅ 已完成 | `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` **exit 0**（日志 0 字节）；`cargo test -p lithe-gpui-explorer` **1 通过** · `-p lithe-gpui-editor` **52 通过** · `-p lithe-gpui-git` **27 通过**（均 exit 0）；`node lithe-gpui/tools/check-ui-px.mjs`（**在 `rust/` 下跑**）exit 0 / **违规 0 处**。5 文件、11 增 8 删。逐处核实清单见 §4.3 的 D14 行 |
| **D16** | ✅ 已完成 | `cargo test -p lithe-gpui-shared` 的 `tests::rem_px_scales_with_the_runtime_base` 通过；`cargo fmt --all -- --check` exit 0 |
| **D20** | ✅ 已完成 | **结论 = 不适用，未改代码**。证据：全仓 `set_menus` 零命中（仅文档 2 处提到该 API）；菜单栏自绘 + `PopupMenu` 渲染期反查（`popup_menu.rs:1301` → `:1119-1144`）；六处 `bind_keys` 全在窗口建立前。只改 `UI-MAP.md` 两处状态单元格 |
| **D22** | ✅ 已完成 | **结论 = 零使用点，未改代码**。`Link` / `.href(` / `open_with` / `cx.open_url(` 在 `crates/*/src/**`（99 个 `.rs`）全为 0 处；只改 `UI-MAP.md` 一处状态单元格 |
| **D19** | ✅ 已完成 | **结论 = 无缓存失效键缺陷；另修掉一处同源真缺陷**。核对覆盖指南点名的五类（换行行高 / text shaping / virtual-list / popup 几何 / 文字推导尺寸）：唯一跨帧缓存是 3 处 `Bounds<Pixels>` 且每帧覆写；其余走上游且键正确。真缺陷 = `completion.rs` 的菜单宽度一次性 rem 换算 → `EditorPane` 新增 `_theme_subscription`（`observe_global::<Theme>`，与 `settings/src/store.rs:242` 同形状）+ `sync_completion_menu_width`，`completion.rs` 拆出 `menu_width` / `set_menu_width`（只动装了 provider 的编辑器、值未变不写）。验证：`cargo fmt -p lithe-gpui-editor -- --check` exit 0（0 字节）· `cargo test -p lithe-gpui-editor` **52 通过** exit 0 · px 守卫 exit 0 / 违规 0。**未加自动化测试**：`editor` 测试层是纯函数层（无 `[dev-dependencies]`），要起真实窗口就得加 `test-support` 依赖，按任务约束不加。已闭环上游 `DisplayMap` 缩放陈旧问题 = **不存在** |
| **D17** | ✅ 已完成 | `node lithe-gpui/tools/check-ui-px.mjs` **exit 0 / 违规 0**；`--selftest` 仍报出样例里那 1 条真违规、放行白名单 2 处（判据没被改松） |
| **D26** | ✅ 已完成 | `cargo test -p lithe-gpui-workbench --test ui` → **1 passed**；`-p lithe-gpui-workbench` 全量 82 单测 + 1 集成测试全绿；`cargo fmt --all -- --check` exit 0；生产图 `cargo tree -p lithe-gpui-app -e normal` 无 `test-support` |
| **D18** | ✅ 六区域全部收口（只 3 个区域产生了改动） | ①（⑥ Git）`cargo fmt -p lithe-gpui-git -- --check` exit 0 · `cargo test -p lithe-gpui-git` **27 通过** · px 守卫违规 0。25 处盘点 → **6 改** / 18 不改 / 1 待裁决。②（⑨ 终端）`cargo test -p lithe-gpui-terminal` **8 通过** · px 0。9 处出现里 4 处注释、真实 5 处 → **2 改** / 3 不改。⚠️ 终端字符行高 `TERMINAL_LINE_HEIGHT` 是**网格几何契约**、红线不改。③（③ 项目树）`cargo test -p lithe-gpui-explorer` **1 通过** · px 0。7 处出现里 4 处注释、真实 3 处 → **1 改** / 2 不改。⚠️ 两处不改的是 **`uniform_list` 的行高契约**。④+⑤（④ 编辑区 + ② 设置）**两个区域 (a) 均为 0 ⇒ 零代码改动**：`cargo test -p lithe-gpui-editor` **52 通过** · `-p lithe-gpui-settings` **169 通过** · px 0。依据：`.h_9()` 挂在**自身没有高度的 `TabBar`** 上 ⇒ 布局要求；`.h_11()` 是**纯布局带**（44 恰等于 `Size::Large` 的 **Input** 档位，不是 Button）。⑥（① 工作台外壳，2026-10-03，**D18 最后一个区域**）`cargo fmt -p lithe-gpui-workbench -- --check` exit 0（日志 0 字节）· `cargo test -p lithe-gpui-workbench` **140 单测 + 1 UI 集成测试** exit 0 · px 守卫 exit 0 / 违规 0。33 行命中里 22 行是注释 ⇒ 真实 **11 处，(a) 0 / (b) 11 ⇒ 零代码改动**：10 处接收者是 `h_flex()`/`div()`（`Size` 不适用），第 11 处 `status_bar.rs:238` 的接收者是 **`StatusBar` 组件而它没有 `Size` API**（只有 `impl Styled`）⇒ **手段不存在**；其中 `quick_open.rs` / `search_everywhere.rs` 的两处结果行高是**契约**（且 `Command` 是**逐行**测量、不是"只量第 0 行"，机制已更正）。**D18 结论：六区域共盘点 55 处出现 / 真实 24 处 → 9 处改（Git 6 / 终端 2 / 项目树 1）、其余为布局带或行为契约；只有 3 个区域产生代码改动。** ⚠️ **仍未做的**：Git 提交按钮 `.h_7()`=28 不在档位内（真源无显式高度，待裁决；行号已漂到 `changes_view.rs:1577`）、终端 2 处档位外尺寸、以及**"Lucide 图标不随档位变"这类既有缺口**（档位只驱动 `Button::icon_size`，本项目大量图标由调用点按 rem 递给 `child`）。⚠️ **所有区域都未经真机视觉验证**：只依据组件 `Size` 的结构性行为，**不声称外观已核对** |
| **D25** | ✅ 已完成（评估） | **未改代码**。结论 = 保持现状、否决收窄。统计：界面引用 70 个图标名 → 组件子集 26 / **只有 Lucide 侧才有 44** / 不在图标 crate 0；`AllAssets` 因此是承重的。0.7 默认子集实测 104 个 / 46 349 B（指南的 101 / 44.28 KiB 是 0.6 旧数）；`AllAssets` 实测 1830 个 / 749 008 B = 731.45 KiB（与指南逐位吻合）。结论与重开条件已写回 `UI-MAP.md` §1.4 H 组两行 |
| **D23** | ✅ 已完成（可改区域） | `cargo fmt -p {lithe-gpui-shared,git,explorer,editor,terminal} -- --check` **全部 exit 0**（5 个日志均 0 字节）；`cargo test -p lithe-gpui-git` **27** · `-p lithe-gpui-explorer` **1** · `-p lithe-gpui-editor` **52** · `-p lithe-gpui-terminal` **8**，全部 exit 0；px 守卫 exit 0 / 违规 0。⚠️ **`-p lithe-gpui-shared` 不能声称全绿**：既有红 22 条 = B7（6 条 `file_icon`，manifest 三分组解析不出） + B2（16 条 `workspace_config`，Core 缺 `git.write`）；本轮用 `--lib -- --skip workspace_config` 隔离，17 通过 / 6 失败，失败文件不在本轮 diff 内。详见新增的 `BLOCKERS.md` B7/B8 |
| **U18** | ✅ 已完成（含"索引就绪"状态收尾） | ① `cargo fmt -p {lithe-gpui-workbench,lithe-gpui-editor} -- --check` exit 0 · `cargo test -p lithe-gpui-workbench` **87 单测 + 1 UI 集成测试** exit 0 · `-p lithe-gpui-editor` **52** exit 0 · px 守卫违规 0 · `verify-test-stability.ps1` exit 0。新增 `crates/workbench/src/search_index.rs`（三条命令的唯一 owner + 5 条确定性单测，含真实 Core 往返）。②（2026-10-03 **"索引就绪"状态收尾**）`cargo fmt -p {lithe-gpui-workbench,lithe-gpui-shared} -- --check` exit 0 · `cargo test -p lithe-gpui-workbench` **147 单测 + 1 UI 集成测试** exit 0（+7 条：状态机 `Pending→Ready` 只报一次、失败→`Failed`、`Ok(None)` 仍算 `Ready`（显式取舍，诊断行另报 `missing_data`）、`update` **永不**翻状态、代次不匹配不采纳、提示三分支、`WARM_OP` 与命令名同源）· `-p lithe-gpui-shared --lib -- --skip workspace_config` **24** exit 0 · px 守卫 exit 0 / 违规 0 · `extract-locale.mjs --check`「产物与真源一致」。**状态 owner 唯一**（`SearchIndexWiring.readiness`，`Pending`/`Ready`/`Failed`），**只在 `adopt` 说"真的变了"时推一次**（不轮询、不在 `render` 里 mutate），四个界面**共用同一处** `readiness_notice()` 元素工厂，浮层用 `WeakEntity` 登记（自清理、打开时先读当刻值）。新增 2 条 `search.gpui.indexWarming` / `indexWarmFailed` 文案走 `GPUI_ONLY_KEYS`，**理由里明说这是宿主侧新增的可解释性提示**（macOS 三份真源都没有这种说法，没有编假出处）。⚠️ warm 的真机端到端效果与那行提示的排版**未经真机验证** |
| **U25** | ⏳ 部分完成（3 项未做） | ①（首轮）`cargo fmt -p {lithe-gpui-workbench,lithe-gpui-shared} -- --check` exit 0 · `cargo test -p lithe-gpui-workbench` **87 单测 + 1 UI 集成测试** exit 0 · `-p lithe-gpui-shared --lib -- --skip workspace_config` **23 通过** exit 0 · px 守卫 违规 0。②（第 4 项，2026-10-03）`cargo fmt -p {lithe-gpui-workbench,lithe-gpui-shared} -- --check` exit 0 · `cargo test -p lithe-gpui-workbench` **112 单测 + 1 UI 集成测试** exit 0 · `-p lithe-gpui-shared --lib -- --skip workspace_config` **24 通过** exit 0 · px 守卫 exit 0 / 违规 0。已做：标题栏 68→40 单行、标题栏各控件四项一起对齐、标签条 38/30 + 等分 + 横向滚动、toast 锚点右下、D23 移交的 3 条 workbench 欠账、**三块分栏可拖 + 尺寸持久化**（含修掉 `SessionTracker` 快判键不含界面状态、导致 UI-only 变更永不落盘的既有缺陷）。未做：标签条 2 个信息元素（无诚实数据源）、状态栏 4 项（与"不许假装能用"冲突）、toast 上限/hover 暂停。遗留 4 条待裁决（底部常量 240 vs 真源下界 260、左栏静网 200/720、右栏默认 400 vs 真源 360、上界用 viewport 属近似）见 §4.2。⚠️ **未经真机视觉验证**：拖动观感/光标/手感一次没在真窗口跑过；"持久化跨重启生效"只证到**文档层往返 + 代码接线**，没跑端到端（需 `-p lithe-gpui-app`，被约束禁止）—— 详见 §4.2 的 U25 行 |
| **U17** | ✅ 四件套已落地（Replace 的「应用」等 main） | ①（第一件 Find in Files）`cargo test -p lithe-gpui-workbench` **107 单测 + 1 UI 集成测试** exit 0 · `-p lithe-gpui-shared --lib -- --skip workspace_config` **24** exit 0 · `-p lithe-gpui-editor` **52** exit 0 · px 守卫违规 0。②（第二件 Quick Open）`cargo test -p lithe-gpui-workbench` **119 + 1** exit 0 · px 0 · `extract-locale.mjs --check`「产物与真源一致」· `verify-test-stability.ps1` exit 0（新增测试最慢 63ms）。③（第三件 Search Everywhere 四 scope）**128 + 1** exit 0（四个搜索模块共 34 条测试全绿，证 `MatchKind`/`Match` 泛化没破坏既有消费方）· shared **24** · px 0 · locale 一致（yml 由脚本重生成）。④（第四件 Replace in Files **预览**，2026-10-03）`cargo fmt -p {lithe-gpui-workbench,lithe-gpui-shared} -- --check` exit 0 · `cargo test -p lithe-gpui-workbench` **140 单测 + 1 UI 集成测试** exit 0 · shared **24** · px 守卫 exit 0 / 违规 0 · locale「产物与真源一致」。**四件套的界面全部落地**（共新增 5 个搜索模块 + 46 条确定性单测）；入口：Find in Files = 左栏「搜索」、Quick Open = `Ctrl+P`、Search Everywhere = 菜单「视图 → 全局搜索」、Replace = 菜单「转到 → 在文件中替换」+ `Ctrl+Shift+R`（真源平台表 `LitheCommandCatalog.swift:61`）。⚠️ **Replace 只做预览、不做应用**：Core **只有 `workspace.replacePreview` 一条**替换命令、没有应用面（已登记 `BLOCKERS.md` B11）⇒ 界面**不画**假的应用按钮，宿主也不自己写盘（会绕开 Core 的 `VisibilityRules`）。⚠️ **未经真机视觉与键盘验证**：四个界面的观感、真按键、Tab 切 scope 都只有代码 + 静态证据 |

⚠️ **D17 的口径更正**：原表"生产代码约 57 处"来自把注释/字符串里的 `px(` 与 `#[cfg(test)]` 断言一起计数，
实测真正的 UI 代码调用是 **13 处**（白名单 11 + 违规 2），本次只改了那 2 处。见 §4.3 的更正说明。

⚠️ **顺带修复（不在任务表编号内，做 D19/D25 时发现）**：`cargo test -p lithe-gpui-app` **在本轮之前是红的**。
`assets.rs` 的 `embedded_range_stays_within_the_documented_envelope` 钉的内嵌资源字节数 `3_317_489`
自 `b66dbd12`（改写 `icons/idea/extension.json` 为三分组、**只改引用不增文件**，+8 015 字节）之后就过期了，
而那次提交没有重测这个 pin —— 与 `assets.rs` 里上一段"改 assets 就要重测"的教训是同一个坑，又踩了一次。
修复同时纠正了一个更隐蔽的问题：**该断言原先按"检出工作区原始字节"求和，而文本资源行尾随
`core.autocrlf` 等本地设置变化**（本机 `extension.json` 工作区是 CRLF 17 754 / blob 是 LF 17 348，差 406；
`git ls-files --eol` 显示 `assets/` 下只有它 `w/crlf`），仓库又**没有 `.gitattributes`** 固定它，
所以同一个提交在 LF 与 CRLF 检出上会算出不同的数（3 325 504 vs 3 325 910）⇒ **必然择一红**。
现改为**行尾无关**口径：文本资源（显式扩展名白名单 + 无扩展名文本路径白名单，**不猜内容**）只扣 CRLF 里的 `\r`，
二进制资源（PNG/ICO/ICNS/TTF，实测含 10 513 个合法 `0x0D` 字节）原样计长；pin = `3_325_504`。
验证：`cargo test -p lithe-gpui-app` **10 通过** exit 0；`cargo fmt -p lithe-gpui-app -- --check` exit 0。
**残留**：将来新增白名单外扩展名的文本资源（如 `.jsonc`）或无扩展名文本资源，CRLF 检出上仍会漂，需同时加进白名单。

- ⛔ **本分支不改 `rust/lithe-core/`**：需要 Core 契约变更的条目，开工前先确认它已被 main 同步进来；
  否则记成"阻塞：等 main"，不要在本分支实现。
- 涉及架构判定的，落一条 `.agents/notes/`；
- 涉及边界的，同步 `develop-lithe`。
