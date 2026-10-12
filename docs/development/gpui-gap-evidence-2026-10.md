# GPUI 三端缺口证据台账（2026-10-12）

本文件是 [GPUI 双端缺口与排期](gpui-platform-gap-and-schedule-2026-10.md) 的引用底稿。
每条证据都可在当前工作区复现；基线提交 `3085f5d1`，分支 `feat/menu-bar-opaque`。

盘点口径：**GPUI 端 = `rust/lithe-gpui`；对照端 = `windows/tauri`（Windows 发行版）与
`macos/`（macOS 发行版）**。三端各有独立的前端实现，`rust/lithe-core` 是共享的命令层。

---

## 1. 跨平台功能矩阵的状态分布

源数据 `shared/platform-feature-matrix/features/*.json`，共 **149 条**能力。
字段只有 `macos` 与 `windows` 两列，**没有 gpui 列**（`metadata.json` 的 `statusDefinitions`
只定义两套状态，`schemaVersion: 4`）。

### 1.1 实现状态分布

| 平台 | implemented | partial | missing | platform-specific | 合计 |
|---|---|---|---|---|---|
| macOS | 128 | 5 | 7 | 9 | 149 |
| Windows | 112 | 12 | 15 | 10 | 149 |

### 1.2 验证状态分布

| 平台 | verified | pending | not-applicable | 合计 |
|---|---|---|---|---|
| macOS | 3 | 130 | 16 | 149 |
| Windows | 4 | 124 | 21 | 149 |

### 1.3 按 area 交叉（Windows 列）

| area | 条数 | implemented | partial | missing | platform-specific |
|---|---|---|---|---|---|
| AI | 28 | 8 | 9 | 11 | 0 |
| 编辑器 | 21 | 18 | 0 | 1 | 2 |
| 工作台 | 20 | 16 | 0 | 0 | 4 |
| 版本控制 | 16 | 15 | 0 | 0 | 1 |
| Java | 15 | 15 | 0 | 0 | 0 |
| 工作区 | 9 | 8 | 0 | 1 | 0 |
| 运行与调试 | 7 | 5 | 2 | 0 | 0 |
| 可观测性 | 4 | 4 | 0 | 0 | 0 |
| 搜索 | 4 | 4 | 0 | 0 | 0 |
| 数据库 | 4 | 4 | 0 | 0 | 0 |
| 界面 | 3 | 0 | 0 | 0 | 3 |
| 语言支持 | 2 | 1 | 1 | 0 | 0 |
| 协作 | 2 | 2 | 0 | 0 | 0 |
| 设置 | 2 | 2 | 0 | 0 | 0 |
| Run/Debug | 2 | 2 | 0 | 0 | 0 |
| 发行与更新 | 2 | 2 | 0 | 0 | 0 |
| 社区 | 2 | 0 | 0 | 2 | 0 |
| Windows 专属 | 2 | 2 | 0 | 0 | 0 |
| 项目 / 启动与窗口 / 扩展 / 插件与扩展 | 各 1 | 各 1 | 0 | 0 | 0 |

### 1.4 Windows 列 missing 的 15 条（含 macOS 侧对照）

| id | area | macOS | Windows | macOS 证据（节选） |
|---|---|---|---|---|
| agent-approval-mode-presentation | AI | implemented | missing | `macos/Sources/Lithe/Views/Agent/AgentSessionSelectors.swift` |
| agent-connection-model-switching | AI | implemented | missing | `macos/Sources/LitheAgentConversationModule/Application/AgentConnectionModel.swift` |
| agent-conversation-history | AI | implemented | missing | `macos/Sources/Lithe/Views/Agent/AgentHistoryView.swift` |
| agent-entry-discoverability | AI | implemented | missing | `macos/Sources/Lithe/Application/Features/AgentConversationEntryPolicy.swift` |
| agent-mode-sync | AI | implemented | missing | `shared/fixtures/agent/acp-events-v1.json` |
| agent-plan-display | AI | implemented | missing | `macos/Sources/Lithe/Views/Agent/AgentSessionGuidanceViews.swift` |
| agent-reasoning-display | AI | implemented | missing | `macos/Sources/Lithe/Views/Agent/AgentTranscriptView.swift` |
| agent-slash-commands | AI | implemented | missing | `macos/Sources/Lithe/Views/Agent/AgentCommandCompletion.swift` |
| agent-thinking-level-labels | AI | implemented | missing | `macos/Sources/Lithe/Views/Agent/AgentSessionSelectors.swift` |
| agent-tool-activity-timeline | AI | implemented | missing | `macos/Sources/Lithe/Views/Agent/AgentToolGroupView.swift` |
| agent-toolbar-brand-marks | AI | implemented | missing | `macos/Sources/Lithe/Views/Agent/AgentPanelStyle.swift` |
| community-auth | 社区 | missing | missing | `shared/fixtures/community`（notes：LINUX DO 插件已退役） |
| community-feed | 社区 | missing | missing | `macos/Sources/Lithe/Platform/MacOS/Plugins/MacPluginPackageStore.swift` |
| markdown-preview-outline | 编辑器 | implemented | missing | `macos/Sources/Lithe/Resources/MarkdownPreview/preview.js` |
| workspace-tree-batch-files | 工作区 | implemented | missing | `macos/Sources/Lithe/Views/Workspace/ProjectTreeSelection.swift` |

### 1.5 Windows 列 partial 的 12 条

`agent-acp-conversation`、`agent-claude-api-key-authentication`、`agent-codex-subscription`、
`agent-context-usage`、`agent-file-references`、`agent-management`、`agent-provider-configuration`、
`agent-subscription-quota`、`agent-turn-statistics`（以上 AI）、`debug-breakpoints`、`debug-state`
（运行与调试）、`php-run-test`（语言支持）。

### 1.6 macOS 列 missing/partial 而 Windows 列 implemented 的 12 条

| id | area | macOS | Windows |
|---|---|---|---|
| ai-chat-history / ai-chat-providers / ai-chat-session | AI | missing | implemented |
| remote-workspace | 工作区 | missing | implemented |
| vim-mode | 编辑器 | missing | implemented |
| docker-compose | 运行与调试 | partial | implemented |
| outline-symbols | 编辑器 | partial | implemented |
| updates-failure-rollback | 发行与更新 | partial | implemented |
| workbench-islands-appearance | 工作台 | partial | implemented |
| workbench-project-color-gradient | 工作台 | partial | implemented |
| community-auth / community-feed | 社区 | missing | missing |

### 1.7 矩阵高估的机制证据

`windows/tauri/src/config/backend-capabilities.ts`：

```ts
export const BACKEND_UNAVAILABLE_TOOLTIP = "待开发";
export const backendCapabilities = {
  agent: false, database: false, debugger: true, docker: false,
  extensions: true, git: true, github: false, remote: false,
  run: true, runActions: false, terminal: true, wsl: false,
} as const;
```

被硬编码为 `false` 的 7 类能力，在矩阵 Windows 列却记为 `implemented`：

| 开关 | 矩阵对应条目 | Windows 列 | 矩阵给的 evidence |
|---|---|---|---|
| `github: false` | github-prs / github-reviews-comments | implemented | `windows/tauri/src/features/github` |
| `database: false` | database-connections / -crud-export / -query-history / -schema | implemented | `windows/tauri/src/features/database` |
| `docker: false` | docker-compose | implemented | `windows/tauri/src/features/docker` |
| `remote: false` | remote-workspace | implemented | `windows/tauri/src/features/remote` |
| `wsl: false` | wsl-paths / wsl-boundaries | implemented | `windows/tauri/src/features/wsl` |
| `agent: false` | agent-*（20 条） | partial/missing | — |
| `runActions: false` | （run-actions，无独立矩阵条目） | — | — |

调用点（10 个文件）：`features/bootstrap/bootstrap-async.ts`、
`features/window/components/window-menu-bar.tsx`、`features/window/components/title-bar/title-bar.tsx`、
`features/window/components/project-picker.tsx`、`features/layout/components/sidebar/sidebar-pane-selector.tsx`、
`features/layout/components/bottom-pane/bottom-pane.tsx`、
`features/layout/components/footer/footer-debugger-item.tsx`、
`features/run-actions/components/run-actions-button.tsx`、`features/run/components/run-pane.tsx`、
`features/github/components/github-notifications-menu.tsx`。

`docs/development/gpui-parity-plan-2026-10.md:5-10` 给出的折算：
**平替基准的 92 项这个数要重算**（2026-10-12 独立评估者复核后更正）：矩阵 Windows 列
`implemented` 是 112 项，但落在 `backend-capabilities.ts` 7 个 `false` 开关组内的
**只有 10 项**且这 10 项都记成 `implemented`（`database-connections`/`-crud-export`/
`-query-history`/`-schema`、`docker-compose`、`github-prs`/`-reviews-comments`、
`remote-workspace`、`wsl-paths`/`-boundaries`）；另外 20 条 `agent-*` 在 Windows 列是
**11 missing + 9 partial**，本来就不是 implemented。所以可核实的折算是 **112 − 10 = 102 项**。
「92 项」与「已具备 23 / 降级或未接线 23 / 完全缺失 46」均出自该既有计划，**其算法未在文档中给出**，
本台账不复述。

---

## 2. GPUI 侧未接线入口（NotWired）

全部集中在 `rust/lithe-gpui/crates/workbench/src/menu_bar.rs`。
`MenuItem::NotWired` 在 `MENUS` 数组内出现 **42** 次，另有 `NOT_WIRED_OUTSIDE_MENUS`
（`menu_bar.rs:172`）2 条，合计 **44** 个未接线入口。
可执行入口 `MenuItem::Action` 为 **52** 个。

### 2.1 按能力分组的未接线项

15 个分组定义在 `menu_bar.rs:79-152`，汇总在 `MISSING_GROUPS`（`menu_bar.rs:154`）。
「命中次数」= `MENUS` 数组内使用该分组 + `NOT_WIRED_OUTSIDE_MENUS` 的条目数。

| 分组 | id | 命中 | 定义行 | 代表使用点（NotWired 起始行） |
|---|---|---|---|---|
| MISSING_EDITOR_FEATURES | editor_features | 9 | 84 | 676 切换注释、689 复制行、694 删除行、699/704 上下移动行、742 分屏、747 缩略图、753 行号、758 空白字符 |
| MISSING_EDITOR_ENTRY | editor_entry | 7 | 79 | 620 另存为、625 全部保存、630 还原文件、781 后退、786 前进、814/819 |
| MISSING_LSP_REQUESTS | lsp_requests | 7 | 89 | 682 参数提示、709 格式化文档、714 格式化选区、793/798/803/808 |
| MISSING_HELP_ABOUT | help_about | 5 | 139 | 896/906/911/917/922 |
| MISSING_FILE_LIFECYCLE | file_lifecycle | 2 | 119 | 600 新建标签页、606 新建文件 |
| MISSING_WINDOW_ROUTING | window_routing | 2 | 134 | 642 关闭窗口、654 退出 |
| MISSING_SUBSYSTEM | subsystem | 2 | 94 | 857/863 |
| MISSING_SHORTCUTS_VIEW | shortcuts_view | 2 | 144 | 870/901 |
| MISSING_TERMINAL_SPLIT | terminal_split | 2 | 149 | 831/836 |
| MISSING_PROJECT_LIFECYCLE | project_lifecycle | 1 | 124 | 613 关闭项目 |
| MISSING_LOCAL_HISTORY | local_history | 1 | 129 | 635 显示本地历史 |
| MISSING_GITHUB | github_integration | 1 | 99 | 735 GitHub |
| MISSING_UPDATER | updater | 1 | 104 | 928 |
| MISSING_CLONE_UI | clone_ui | 1 | 109 | 173 克隆仓库（菜单外） |
| MISSING_NEW_PROJECT_SCAFFOLDING | new_project_scaffolding | 1 | 114 | 176 新建项目（菜单外） |

### 2.2 未接线项里「实现已在、只缺调用点」的实例

| 能力 | 实现位置 | 缺什么 | 证据 |
|---|---|---|---|
| 跳转历史前进/后退 | `crates/editor/src/editor_view.rs:1083` `go_back`、`:1094` `go_forward`；`crates/editor/src/navigation.rs:310/328` 的 `JumpHistory` | 两个函数都标了 `#[allow(dead_code)]`，全仓没有 action 定义与键位绑定；`GoBack`/`GoForward` 只在 `i18n.rs:45-48` 有文案 | `editor_view.rs:1082`、`:1093`；检索 `GoBack\|GoForward` 只命中 i18n 与 menu_bar |
| Spring 面板 | `crates/workbench/src/spring_panel.rs`（54 行）、`spring.rs`（360 行）、`spring_paths.rs`（342 行） | 右栏 rail 无映射，点不出来 | `docs/development/gpui-parity-plan-2026-10.md:134-154` |
| Git 主动 merge/rebase/cherry-pick | `crates/git/src/remote.rs:45` `IntegrationOperation`、`:515` `integration_gate` | **在 `workbench`/`explorer`/`settings`/`editor` 四个 UI crate 里检索 `IntegrationOperation`/`integration_gate` 命中 0**，即无 UI 消费者 | `docs/development/gpui-parity-plan-2026-10.md:280`（该文档说「全仓无 UI 消费者」，本台账收窄为「UI crate 内无消费者」） |
| Git 身份配置页 | `crates/git/src/identity.rs`；host 注入点 `workbench/src/workspace.rs:883` | **已接线**：`set_git_identity_host` 在生产路径被调用（`workspace.rs:883`，该文件测试模块起点 `:3395`），消费者 `settings/src/dialog.rs:264`。仅 `identity.rs:104` 的 `parse_setup` 标 `#[allow(dead_code)]`，且只在 `identity.rs` 自己的测试（`:218/241/267`，`cfg(test)` 起点 `:176`）里被调用 | 已更正：既有计划 `:283` 称「生产代码里从未被调用」，该说法与 `workspace.rs:883` 不符 |

### 2.3 GPUI 侧完全检索不到**实现代码**的能力

**口径**：在 `rust/lithe-gpui/crates/` 下按 `.rs` 实现代码检索（137 个 `.rs` 中 `crates/` 下 123 个）。
**这个口径排除了本地化文件**（`crates/shared/locales/**` 的 `.yml` 与 `.ts`）——文案键不是实现。
第 4 列另给**全文件口径**（含 `.rs`/`.yml`/`.ts`）的结果，两组数不同，不要混用。

| 关键词 | `.rs` 实现代码命中 | 全文件命中（含文案键） | 对照端位置 |
|---|---|---|---|
| `mybatis` / `MyBatis` | 0 | **0**（文案键也没有） | `windows/tauri/src/features/mybatis`（7 文件） |
| `viewer` | 0 | **207**（`locale.ts` 100、`lithe.en.yml` 57、`lithe.zh-CN.yml` 50，如 `panes.databaseViewer`、`viewer.enterUrl`） | `windows/tauri/src/features/viewer`（30 文件） |
| `local-history` | 0 | **4**（`lithe.en.yml:7102`、`lithe.zh-CN.yml:7102`、`locale.ts:3662/7981`） | `windows/tauri/src/features/local-history`（5 文件） |
| `run-actions` / `runActions` | 0 | **68**（`locale.ts` 34、`lithe.zh-CN.yml` 17、`lithe.en.yml` 17） | `windows/tauri/src/features/run-actions`（8 文件） |
| `onboarding` | 0 | **125**（`lithe.en.yml:7218/8532-8590` 整组 `onboarding.*`） | `windows/tauri/src/features/onboarding`（5 文件） |
| `logging` | 0 | **13**（`settings.logs.diagnosticMode` 等） | `windows/tauri/src/features/logging`（6 文件） |
| `host-api` / `hostApi` / `HostApi` | 0 | **0** | `windows/tauri/src/features/host-api`（6 文件） |
| `community` / `Community` | 0 | **0** | 矩阵 community-auth / community-feed（两端均 missing） |
| `mcp` / `MCP` | 0 | **0** | 矩阵 ide-mcp-connection（两端 implemented） |
| `ai_chat` / `AiChat` / `aiChat` | 0 | **8**（`layout.resizeAiChat`、`keybindings.commands.workbench.toggleAIChat.title`） | 矩阵 ai-chat-session / -history / -providers（Windows implemented） |
| `notebook` | 0 | 0 | — |

**读法**：`mybatis`、`host-api`、`community`、`mcp`、`notebook` 五个是**连文案键都没有**的真空白；
`viewer`、`onboarding`、`logging`、`run-actions`、`local-history`、`ai_chat` 六个是**有文案键、
无实现代码**——这类缺口的实际工作量比真空白小（界面文案已定稿），但也不是「已实现未接线」，
因为代码入口不存在。

命中不为 0 但只是文案或图标占位：`github`（8 处，全在 `i18n.rs` 与 `menu_bar.rs` 的未接线项）、
`docker`（6 处，`assets.rs`/`file_icon.rs`/`idea.rs` 的图标资源）、`database`（5 处，同上）、
`wsl`（10 处，`settings/src/dialog.rs`/`persistence.rs`/`schema.rs` 的路径配置项）、
`vim`（1 处，`terminal/src/constants.rs` 的终端模式名）。

---

## 3. 三端构建与发布入口对照

| 端 | CI 工作流 | 本地脚本 | 产物 | 检索证据 |
|---|---|---|---|---|
| Windows（旧 Tauri） | `.github/workflows/release-windows.yml`、`release-preview-windows.yml`、`ci-windows.yml` | `scripts/build-windows.ps1`、`scripts/package-windows.ps1` | MSI / NSIS，走 `windows/tauri` | `release-preview-windows.yml:52` 目标 `x86_64-pc-windows-msvc`；缓存键含 `windows/tauri/src-tauri/Cargo.lock` |
| macOS（Swift） | `.github/workflows/release-macos.yml`、`release-preview-macos.yml`、`ci-macos.yml` | `scripts/build-macos.sh`、`scripts/build-rust-core.sh` | `.app` / DMG，走 `macos/` + `lithe-core` | `build-macos.sh:64` 只调 `build-rust-core.sh`，`RUST_TARGET` 为 `aarch64-apple-darwin` / `x86_64-apple-darwin` |
| **GPUI 统一前端** | **无** —— 18 个工作流中检索 `lithe-gpui` 命中 0 | 仅 `scripts/sign-windows.ps1`（默认目录 `lithe-gpui-target/debug`）；构建靠手写 `cargo build -p lithe-gpui-app` | `Lithe.exe`（Windows） | `Get-ChildItem .github/workflows` 逐个检索 `lithe-gpui` 全部 0 命中；`sign-windows.ps1:3` 的默认参数是唯一引用 |

补充事实：

- `rust/lithe-gpui/Cargo.toml` 的 `[profile.*]` 只覆盖 `gpui-pre` 等上游包，无平台条件。
- `crates/app/Cargo.toml` 只有一处平台条件：`[target.'cfg(windows)'.build-dependencies] embed-resource = "3"`，
  用于把 `lithe.rc` 的图标嵌进 `Lithe.exe`。
- `crates/settings/src/paths.rs` 已有 `#[cfg(target_os = "windows")]`（:98）与
  `#[cfg(target_os = "macos")]`（:108）两条分支，说明 macOS 配置目录已在代码层被考虑。
- `crates/settings/src/keymap.rs:62` 明确写着「**不做 macOS 档**：⌘ 在 Windows 上没有意义」，
  键位表目前只有 IDEA 与 Eclipse 两档，且 `ECLIPSE_BINDINGS` 为空（`:44`、`:420`）。
- `rust/lithe-gpui` 的 `#[cfg(target_os)]` 全目录仅 4 处（`java/src/workspace.rs` 2 处、
  `settings/src/paths.rs` 2 处）。

---

## 4. 规模基线

| 端 | 语言 | 文件数 | 备注 |
|---|---|---|---|
| `rust/lithe-gpui` | Rust | `crates/` 下 123 个 `.rs`，13 个宿主 crate | 最大 crate：`workbench`（988 KB）、`settings`（562 KB）、`shared`（252 KB）；README 与 workspace 注释仍写「12 个」，漏了 `languages` |
| `windows/tauri/src/features` | TS/TSX | 1,633 个文件，39 个 feature 目录 | 最大：`git`（326）、`editor`（271）、`ai`（113） |
| `macos/Sources` | Swift | 666 个 `.swift` | 其中 `Views/` 177 个，19 个子目录 |

---

## 5. 与既有排期文档的关系

`docs/development/gpui-parity-plan-2026-10.md`（2026-10-12 写）是本次盘点的直接前身，差异：

| 维度 | 既有文档 | 本台账 |
|---|---|---|
| 对照端 | 只有旧 Windows 端 | Windows 与 macOS 两端 |
| 时间窗 | 10/12 → 10/31 | 见最终排期文档 |
| 基线 | 92 项（写「112 − 20」但 20 的算法未给出） | **102 项**（112 − 10，10 = 落在 7 个 `false` 开关组内且矩阵记 implemented 的条目数；`agent-*` 20 条本就不是 implemented） |
| 矩阵登记 | 记「待定：新增 gpui 平台字段还是记到 windows 端」（`:344-348`） | 作为未决项列出 |

该文档自述的硬约束（构建 15 分钟/次、`CARGO_TARGET_DIR` 需绕开工作区、
内存被 `explorer.exe` 占用会污染 target 缓存、每批功能必须同步矩阵）在最终排期中作为
工作量系数沿用。
