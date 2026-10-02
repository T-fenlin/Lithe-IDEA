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
| **U17** | 项目级搜索四件套（Find in Files / Search Everywhere / Quick Open / Replace in Files） | 196 + 563 + 313 | **Core 全有** + `project/search_index.rs` 内存索引；gpui **零界面**，左栏"搜索"只切高亮 |
| **U18** | 搜索索引 warm/update/invalidate 的调用时机 | — | **Core 全有**；gpui 零引用 → 接 U17 前必须先接这个，否则首次搜索会同步建全量索引 |
| **U19** | 调试 UI（调用栈/变量树/监视/求值/断点/单步/步进过滤器） | `Views/Debug/` 2693 | **Core 18 条命令全有**、`debug/engine.rs` 3886 行；gpui **调用点 0** |
| **U20** | 编辑器补缺：外部修改冲突横幅、编码探测与重开、分屏、标签拖拽、Markdown/HTML/SVG/媒体预览、转到行列、软换行开关、Code Vision、inline blame、inlay hints、悬停文档、自动保存设置化 | 分散 | 数据侧部分在 Core（`git.blame`、`git.diff`、LSP `hover`/`semanticTokens`）；上游 gpui-base 已有 hover/semantic 的缝，**宿主未装 provider** |
| **U21** | 数据库界面 | `Views/Database/` 5255 | 依赖 C30–C32 |
| **U22** | AI 提交信息/PR 描述界面 + provider 设置页 | macOS 侧 2111(GitHub) + AI 相关 | 依赖 C36/C37；Core 已有 `plan_commit`/`decode_message`/`parse_codex`/`parse_claude` |
| **U23** | Agent 对话面板 | `Views/Agent/` 3204 + `LitheAgentConversationModule` | Core 有 `lithe_agent_*` C ABI + 4 条管理命令，但 **gpui 无 Rust-native 调用面**（`mod agent` 私有、gpui 不依赖 `lithe-agent-host`）→ 先定异步边界（ACP 是长连接事件流，而 `CoreClient` 是同步信封） |
| **U24** | 资源管理器补缺：空目录显示、watcher 联动、右键菜单/多选/拖拽、Reveal Active File、Git 状态装饰、手动刷新入口 | `ProjectSidebarView.swift` 1029 | 空目录需 `data.root`（**Core 已有**，gpui 只用了 `data.files`）；`git.status` 已有 |
| **U25** | 外壳度量对齐 macOS：标题栏 68→40、项目标签条 32/28→38/30 + 3 个信息元素 + 横向滚动、底部/右侧工具窗可拖 + 宽度持久化、通知 toast 位置与"最多 3 条/hover 暂停"、状态栏 4 项占位 | `LitheTheme.swift:477-480` 等 | 不需要 Core |

### 4.3 技术债（小改动、高收益，可插队）

| ID | 任务 | 落点 | 为什么优先 |
| --- | --- | --- | --- |
| ~~**D10**~~ ✅ | 运行启动别阻塞 UI 线程 | **已完成**：`RunSession::start` 拆成 `prepare`（取计划 + 跑 `preLaunchSteps`，**阻塞**，放进 `background_spawn`）+ `start`（只做 `spawn` + 建读线程，毫秒级，可在 UI 线程）；`ToolchainResolver` 因此补上 `Send + Sync`（外壳实现是纯查表，本来就满足） | `preLaunchSteps` 同步跑，javac 期间**整窗卡死最长 300s** |
| ~~**D11**~~ ✅ | 状态栏别每帧读 `.git/HEAD` | **已完成**：删掉 `workbench::read_branch`，改用 `ChangesView::branch()` —— Git 视图已持有的 `git.status` 快照，**每帧只做内存读取、没有 I/O**；worktree / 分离 HEAD / 仓库根≠工作区根这三处读错交给 Core 判定 | 每帧一次文件 I/O，且 worktree/分离 HEAD/仓库根≠工作区根**全读错**；应改走 `git.status` |
| ~~**D12**~~ ✅ | 修 `searchable` 反向注释 | **已完成，但本行原判断是反的**：0.7.0 的 `EditorState::new` **确实**把 `searchable` 设成 `true`（`gpui-base-0.7.0/src/input/base/state.rs:10085` → `:10088`；`:709` 的 `false` 属于 `InputBaseState::default`，是给单行 `Input` 用的），所以**原注释是对的**。本轮真正修的是：陈旧路径 `0.6.6` → 0.7.0 实测位置，并补上"删掉这一句会**静默**失去 Ctrl+F/H"的后果 | 后人"清理冗余调用"会**静默丢掉 Ctrl+F/H**（这一点原来就成立，只是理由写错了版本） |
| ~~**D13**~~ ✅ | 状态栏只读/缩进格接真值 | **已完成**：新增 `EditorPane::tab_size()` 与 `EditorPane::active_buffer_is_readonly()`；`footer_right` 的缩进格与只读格改读它们，只读图标在 `LOCK_ICON` / `LOCK_OPEN_ICON` 之间切换 | `is_readonly()` 上游已有（`gpui-base-0.7.0/src/input/base/state.rs:509`），`tab_size` 宿主自己有（`editor_view.rs:397`） |
| **D14** | 文档/注释里的死引用 | `UI-MAP.md:141`（`panels.rs` 不存在）、`explorer/src/lib.rs:28-29`（空目录分支不可达）、`model.rs:214`（行号漂移） | 一行级，收益是后续核对不再踩空 |
| **D15** | 清掉 gpui 里的 `windows/tauri` 规格残留 | 约 178 处注释（workbench 49、git crate 头部整份、explorer、terminal、editor） | 规格已定 macOS；不清掉，阶段二/四每条都要重新论证一次 |
| ~~**D16**~~ ✅ | **把重复的 `rem_px` 换算收敛成一份** | **已完成**：唯一实现 `crates/shared/src/lib.rs` 的 `rem_px`（+ 随基准缩放的 5 条断言单测）；`editor` / `git` / `settings` / `terminal` / `workbench` 五处改为 `pub(crate) use lithe_gpui_shared::rem_px;` 转发（共 6 份定义、3 份重复单测 → 1 + 1） | 同一套换算原先写 5 遍，任何一处改口径（16 还是别的基准）都会分叉；`shared` 是跨 crate 原语该在的地方（`icons` / `i18n` / `core_client` 已在那里） |
| ~~**D17**~~ ✅ | **清掉生产代码里直接写的 `px()`，改用 rem-based 助手** | **已完成**：终态 2 处违规（`terminal_view.rs` 的 `row_font_size` / `self.font_size`）改为 `rems(size / 16.)`（设置值是规格 px，`Styled::text_size` 收 `AbsoluteLength`）；守卫 `node lithe-gpui/tools/check-ui-px.mjs` 实测 **exit 0 / 违规 0**。║ **口径更正**：本表原写"生产代码约 57 处"，实测**不是** 57 —— 那是把注释/字符串里的 `px(` 与测试断言一起数进去的结果。守卫的实测分解（`--list`）：`px(` 子串 214 处（改动前）= 注释/字符串 66 + 形近调用 82 + `#[cfg(test)]` 内 53 + **真正的 UI 代码调用 13**（其中白名单 11、违规 2） | 写死 px 会让"用户改界面字号"时不缩放。守卫只认 `coding-guides.md:288` 的四类例外，白名单 7 条（4 条 OS 窗口平台边界 + 3 条 `Theme.font_size`/`mono_font_size` token 定义本身） |

### 4.4 官方指南衍生的债务（来源：`docs/gpui kit/0.7.0/docs/{coding,design}-guides.md`）

> 这两份是 gpui-kit 的**明文规范**（§1.4 是摘录）。下面每条的"现状"都是实测，不是推测。

| ID | 任务 | 指南条文 | 本项目现状 |
| --- | --- | --- | --- |
| **D18** | 大面积手写高度 → 改用组件 `Size` API | 「优先使用组件尺寸（`xsmall`/`small`/`medium`/`large`），**不要单独写高度**」；「只改外框高度通常是不完整的」——字体、图标、padding、hit target 要**一起**变 | gpui 侧大量 `h_8()` / `h_9()` / `h_7()` 手写定高（§2.2 ①⑤⑥ 里的度量偏差多由此而来） |
| **D19** | 核对缓存失效键是否含 `window.rem_size()` | 「从 resolved layout 得到的 cache **必须把 `window.rem_size()` 纳入 invalidation key**」——换行行高、text shaping、virtual-list 测量、popup 几何、由文字推导的图标尺寸 | **未核**。若不满足，改界面字号后测量结果仍是旧的 |
| **D20** | 核对"先 `bind_keys` 再 `set_menus`"的顺序 | 「`cx.set_menus` 在调用那刻读取 keymap；之后注册的 binding **不会**出现在菜单项旁，菜单项也不响应该按键」 | 菜单栏在 `crates/workbench/src/menu_bar.rs`，**顺序未核** |
| **D21** | 核对是否"一个逻辑命令只建模一次" | 「Toolbar Button、菜单项、ContextMenu item、menu-bar item 与 key binding 应 dispatch **同一个 Action**，不能复制五份 mutation」 | 与 §2.2 ⑥ 的"引用行右键 17 项 / 提交行右键"实现方式直接相关：**新做这些菜单时不要各写一份 mutation** |
| **D22** | 核对 `Link` 的使用边界 | 「`Link` **只**用于交给浏览器或邮件客户端的外部目标；应用内导航与命令用 `Button`/`Action`」 | 未核 |
| **D23** | 精确对齐不变量落地 | 「两条 edge 或两段 spacing 按设计应相等时，**相差一个 rendered pixel 也是缺陷**」；相等关系必须来自**同一个** rem helper / spacing token / shared inset，**不能用 raw pixel offset 修补** | 未核；§2.2 多处"度量与 macOS 不一致"应优先修共同 owner，而不是逐个调 offset |
| **D24** | 界面文案过一遍用词规范 | 「入口/对象用名词、命令用动词、状态用形容词」；按钮说明**结果**；`OK` 只用于纯信息确认；打开对话框的命令加 `…`；英文 sentence case；单省略号 `…` | 文案真源是 `macos/Resources/zh-Hans.lproj`，复刻时顺带按此校对；§2.2 ①的"状态栏 4 项假值"也属这类 |
| **D25** | 评估是否还需要嵌 `AllAssets`（**约 1.02 MiB**） | `assets.md` 的嵌入量对照：默认组件图标 **101 个 = 44.28 KiB（基线）**，**`AllAssets` 1830 个 = 731.45 KiB，+1.02 MiB**；并明确"**依赖图标 crate 不等于嵌入全部图标**，只有显式注册 `AllAssets` 才会" | 本项目 `application().with_assets(assets::LitheAssets)` 里把 `AllAssets` 当兜底。我们自己另嵌 411 个文件（3,317,489 B）。若界面实际只用到我们自己那批图标，用指南的 `icon_assets!(ExtraIcons, [...])` 按需选、或直接去掉 `AllAssets` 兜底，能省下这笔体积。**需要先统计"哪些图标名只有 Lucide 侧才有"**，否则会静默画空白 |
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
| **D16** | ✅ 已完成 | `cargo test -p lithe-gpui-shared` 的 `tests::rem_px_scales_with_the_runtime_base` 通过；`cargo fmt --all -- --check` exit 0 |
| **D17** | ✅ 已完成 | `node lithe-gpui/tools/check-ui-px.mjs` **exit 0 / 违规 0**；`--selftest` 仍报出样例里那 1 条真违规、放行白名单 2 处（判据没被改松） |
| **D26** | ✅ 已完成 | `cargo test -p lithe-gpui-workbench --test ui` → **1 passed**；`-p lithe-gpui-workbench` 全量 82 单测 + 1 集成测试全绿；`cargo fmt --all -- --check` exit 0；生产图 `cargo tree -p lithe-gpui-app -e normal` 无 `test-support` |

⚠️ **D17 的口径更正**：原表"生产代码约 57 处"来自把注释/字符串里的 `px(` 与 `#[cfg(test)]` 断言一起计数，
实测真正的 UI 代码调用是 **13 处**（白名单 11 + 违规 2），本次只改了那 2 处。见 §4.3 的更正说明。

- ⛔ **本分支不改 `rust/lithe-core/`**：需要 Core 契约变更的条目，开工前先确认它已被 main 同步进来；
  否则记成"阻塞：等 main"，不要在本分支实现。
- 涉及架构判定的，落一条 `.agents/notes/`；
- 涉及边界的，同步 `develop-lithe`。
