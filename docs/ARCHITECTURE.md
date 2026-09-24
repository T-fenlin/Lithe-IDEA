# Lithe 架构总览

本文是 Lithe 全景架构说明。README 的「Architecture Overview」是它的摘要版，两者描述同一套分层，冲突时以本文与代码为准。

> 本文基于仓库 `main` 分支 2026-09-23 的代码状态编写。架构决策的**理由**不在本文，而在 `.agents/notes/`（中文 Agent Note）；本文只描述**结构**。

## 先说结论

1. **两个产品，一个 Core。** macOS（SwiftUI/AppKit）是当前参考产品，Windows（React/Tauri 2）是独立实现。两者不共享任何界面代码，只通过 Rust `lithe-core` 共享确定性业务逻辑。
2. **Core 不是 RPC 框架，而是命令信封。** 两端都把请求编码成同一种 JSON 信封（`command` + `payload`），Core 返回 `ok`/`error`，长任务通过事件回调流式返回。契约里有 **128 条命令**、11 个稳定错误码、7 类事件。
3. **两种接入方式是有意为之。** macOS 走**稳定 C ABI**（弱链接薄桥），Windows **直接链接 crate**。这不是历史包袱，而是契约里明确写下的设计（`shared/contracts/rust-core-api.md:3-5`）。
4. **依赖方向单向：** 表现层 → 应用层 → 服务 → 端口（协议）← 平台适配器实现。Core 与应用层不允许出现 SwiftUI/AppKit/Tauri/Win32。
5. **边界靠脚本强制，不靠自觉。** `scripts/` 下有 19 个 `verify-*` 脚本，其中多个只能跑在 macOS 上。改架构前先看第七节。

## 一、顶层结构与规模

| 目录 | 职责 | 文件数 | 源码行数 |
| --- | --- | --- | --- |
| `macos/` | macOS 产品（**参考产品**）：SwiftUI/AppKit 应用 + 15 个 SwiftPM target | 1010 | 190,932 |
| `windows/` | Windows 产品：React/Tauri 2 应用 + Rust host（独立实现） | 3183 | 274,188 |
| `rust/` | 共享 Rust Core workspace（4 个 crate） | 141 | 84,934 |
| `shared/` | 跨平台契约与 fixture，**不编译** | 125 | 13,219 |
| `frontend/editor/` | 共享 Monaco 表现层、分词与编辑器模型生命周期，无平台 API | 37 | 5,699 |
| `Plugins/mac/` | macOS 插件包（官方 2 个） | 16 | 1,017 |
| `Plugins/win/` | Windows 插件包（**只有 README**） | — | — |
| `scripts/` | 构建、打包、验证、发布脚本 | 90 | 10,118 |
| `infra/` | 仓库级开发/验证基础设施（数据库校验容器） | 4 | 152 |
| `third_party/` | 上游依赖清单（`jdk`/`jdtls`/`dbx` 的 `manifest.json`） | 4 | 85 |
| `docs/` | 设计语言、发布说明、性能基线、架构文档 | 83 | 2,182 |
| `Casks/` | Homebrew Cask | 1 | 27 |

「macOS 是参考产品」的依据：`AGENTS.md` 的明文规定，以及 README 徽章只列 macOS 13+ 与 Windows x64。

## 二、Rust Core：唯一共享业务真源

### 2.1 三种接入方式

| 宿主 | 接入方式 | 入口 |
| --- | --- | --- |
| macOS | **稳定 C ABI**（弱链接） | `macos/Sources/LitheRustCore/bridge.c` → `lithe_bridge_*` → `lithe_core_execute_json`；Swift 侧统一封装在 `macos/Sources/Lithe/Core/Rust/RustCoreBridge.swift` |
| Windows（Tauri host） | **直接链接 Rust crate** | `windows/tauri/src-tauri/src/platform.rs` 调 `lithe_core::execute_json` / `execute_json_with_events` |
| 未来 Rust 宿主（如 GPUI） | 直接链接 crate | 同上，另有 `lithe_core::execution::plan_launch_command` 等原生 API |

C ABI 共 **7 个导出函数**（`rust/lithe-core/include/lithe_core.h:10-16`），crate 同时产出 `rlib` + `staticlib` + `cdylib`：

```c
const char *lithe_core_version(void);
char *lithe_core_execute_json(const char *request);
char *lithe_core_execute_json_with_events(const char *request, void (*callback)(const char *, void *), void *context);
int32_t lithe_core_git_askpass(const char *prompt);
char *lithe_core_lsp_provider_catalog_json(const char *workspace_root);
int32_t lithe_core_cancel(const char *operation_id);
void lithe_core_free_string(char *value);
```

### 2.2 命令信封

**请求**：

```json
{ "id": "…", "operationId": "…", "timeoutMilliseconds": 30000, "command": "workspace.search", "payload": {} }
```

- `operationId` 可省略，默认取 `id`；`timeoutMilliseconds` 为正数时启动协作式 deadline。
- 取消与超时在命令边界、遍历点与 Git 等待点检查，返回 `cancelled` / `timed_out`。
- Git 相关请求另有 `gitExecution` 字段（`platform.rs`、`RustCoreBridge.swift`）。

**响应**：成功 `{ id, ok: true, data }`；失败

```json
{ "id": "…", "ok": false, "error": { "code": "invalid_request", "message": "…", "details": {} } }
```

**11 个稳定错误码**（`rust/lithe-core/src/protocol/error.rs:11-34`）：`invalid_request`、`workspace_not_found`、`permission_denied`、`not_supported`、`runtime_missing`、`process_start_failed`、`process_failed`、`parse_failed`、`cancelled`、`timed_out`、`unknown`。

**7 类事件**（`execute_json_with_events` 回调）：`requestStarted`、`started`、`output`、`finished`、`requestFinished`、`authentication`、`remoteResult`，每个事件携带 `operationId` 与 `type`。

### 2.3 命令面：128 条命令

契约 Commands 表共 128 条，命名空间分布：

| 命名空间 | 条数 | 命名空间 | 条数 |
| --- | --- | --- | --- |
| `git` | 37 | `history` | 6 |
| `debug` | 18 | `workspace` | 5 |
| `lsp` | 18 | `github` | 3 |
| `java` | 11 | `file` / `editor` / `diagnostics` | 各 2 |
| `community` | 7 | `core` / `document` / `mybatis` / `spring` | 各 1 |
| `runConfig` | 7 | | |
| `maven` | 6 | **合计** | **128** |

### 2.4 Core 内部分层

`rust/lithe-core/src/lib.rs` 声明 15 个模块，每个模块以 `mod.rs` 的 `//!` 首行声明职责：

| 模块 | 职责 | 规模 |
| --- | --- | --- |
| `protocol` | 稳定 command / response / error / event / cancellation 契约 | 1,730 行 |
| `runtime` | JSON 命令派发与导出的 C ABI | 2,455 行 |
| `project` | 项目文件、搜索、本地历史、文档渲染 | 5,363 行 |
| `execution` | Run 配置生成解析与项目 detector | 5,063 行 |
| `languages` | 与 LSP 传输无关的语言级项目检查 | 3,742 行 |
| `git` | 确定性 Git 检查与变更 | 13,710 行 |
| `lsp` | 语言工具，facade 保持 Core 命令 API 稳定 | 20,904 行 |
| `debug` | 传输无关的 DAP 状态与调试模型规范化 | 5,098 行 |
| `ai` | AI commit 配置解析与 HTTP 请求规划（I/O 与密钥归宿主） | — |
| `community` | 共享社区集成与跨平台协议边界 | — |
| `diagnostics` | 诊断包脱敏与 manifest 塑形（纯确定性，无文件/进程） | — |
| `editor` | Core 拥有的编辑器文本变换命令 | — |
| `github` | GitHub 请求规划与响应规范化（网络与凭据归平台） | — |
| `plugins` | 插件 manifest 解析、兼容性检查、目录合并 | 385 行 |
| `tests` | Core 集成测试 | 14,248 行 |

**8 个 facade 受强制约束**（`scripts/verify-rust-core-layout.sh`）：`protocol`、`runtime`、`project`、`execution`、`languages`、`git`、`lsp`、`tests` 必须有 `mod.rs`；`src/` 顶层只允许 `lib.rs`。facade 用 `pub(crate) use X::*` 聚合内部实现，对外只暴露 `lib.rs` 重导出的公共入口。目的是让内部文件可以重组，而命令 API 不漂移。

`runtime` 内部再分为 `dispatcher`（校验并路由版本化命令名）与 `ffi`（C ABI），两者不混。

### 2.5 Core 不拥有什么

明确留在平台侧、Core 不得吸收的东西：网络传输与凭据、进程创建与 PTY、文件监听、UI 渲染、平台对话框、更新通道、语言/构建/调试器的语义真源（Java 符号、模块归属、classpath 来自所选后端，不由 Core 二次实现，见 `scripts/verify-java-semantic-ownership.mjs`）。

## 三、macOS 侧分层（参考产品）

`macos/Sources/Lithe/` 六层，依赖方向为 `Views → Models/Application → Services → Core(Ports) ← Platform/MacOS`：

| 层 | Swift 文件数 | 职责 |
| --- | --- | --- |
| `Views/` | 144 | SwiftUI/AppKit 表现与视图局部渲染；不得调用 Rust C ABI、不得构造平台适配器 |
| `Models/` | 64 | UI 面向的模型与 `AppModel` 聚合 |
| `Application/` | 72 | 功能模型、状态转换、用户动作（含 `Composition/`、`Features/`、`Lifecycle/`、`Workspace/`） |
| `Services/` | 25 | 产品工作流编排；不得直接创建 `Process`/`Pipe`/`FileManager`/watcher 或具体 `Mac*` 适配器 |
| `Core/` | 53 | 平台中立端口（`Ports/`）与类型化 Rust 操作（`Rust/`，含 `RustCoreBridge.swift`） |
| `Platform/MacOS/` | 82 | macOS 适配器与组合（AI / Community / Debug / … / Updates） |

- **组合根是 `MacServiceContainer`**（`macos/Sources/Lithe/Platform/MacOS/MacServiceContainer.swift:30-41`）：视图模型接收容器，而不是自己构造平台适配器；`init` 里构建 `RustCoreBridge`、Git 执行日志、Run 配置存储、模块生命周期协调器等。
- 各功能模块被抽为独立 SwiftPM target（`Package.swift`，15 个 library + `Lithe` 主程序 + 4 个 verifier 可执行），实现只存在于 `Lithe*Module` target 内，跨模块依赖受 `scripts/verify-module-boundaries.sh` 校验。

## 四、Windows 侧分层

`windows/tauri/src/` 顶层划分：

| 目录 | 职责 |
| --- | --- |
| `features/` | React 功能代码（39 个子目录：`git/`、`editor/`、`debugger/`、`terminal/`、`run/`、`workspace/` …） |
| `ui/` | 可复用 UI 组件（button / dialog / tabs / icons …） |
| `platform/` | 前端到 Tauri 与原生命令的适配层（`tauri-core.ts`、`core-result-adapter.ts`、`lsp-core-adapter.ts`、`document-lifecycle.ts` …） |
| `core/` | 信封类型与 `executeCore` / `cancelCoreOperation` |
| `extensions/` | 主题、图标主题等扩展子系统 |
| `styles/` | CSS 变量与设计 token |

**唯一 invoke 边界**：全仓库只有 2 个 TypeScript 文件直接 `import … from "@tauri-apps/api/core"` —— `core/lithe-core-client.ts` 与 `platform/tauri-core.ts`。其余模块必须经 `@/platform/tauri-core`，由 `scripts/verify-windows-boundaries.sh` 与 `.ps1` 强制。

**中央 dispatcher**：`windows/tauri/src-tauri/src/platform.rs`（1379 行）是唯一注册为 `platform_invoke` 的 Tauri 命令：

1. `translate(command, args)` 把 Windows 兼容命令名译成 Core 命令名并规范化 payload（如 `git_status` → `git.status`、`repoPath` → `root`）。
2. 组装信封，在 `spawn_blocking` 中调用 `lithe_core::execute_json(_with_events)`。
3. `core_response` 把 Core 信封折回前端期望的兼容形状。

**规则：不得恢复"每个 Core 操作一个 Tauri 命令"。** 依据有三处：`AGENTS.md` 明文、`tauri-core.ts` 只经 dispatcher、以及 `verify-windows-boundaries.*` 还会拒绝任何新增的 `.cpp`/`.h`（防止已退役的 Qt/C++ 实现复活）。

## 五、端到端数据流：以「获取 Git 状态」为例

**macOS**

1. 视图层：`macos/Sources/LitheGitModule/Views/…`（Git 视图 import `LitheGitModule`）
2. 应用层：`LitheGitModule/Application/GitFeatureModel.swift`
3. 端口：`LitheGitModule/Services/GitService.swift` 的 `protocol GitOperations` → `snapshot(at:)`
4. 平台无关实现：`macos/Sources/Lithe/Core/Rust/RustGitOperations.swift` → `core.gitStatus(at:)`
5. Core 端口：`RustCoreBridge.swift` 构造 `command: "git.status"` 与 `GitStatusRequest(root:)`
6. 桥：编码请求 → `lithe_bridge_execute_json` → `lithe_core_execute_json`
7. Rust：`runtime/ffi.rs` → `lib.rs` 的 `execute_json` → `runtime/dispatcher.rs` 路由到 `git` 模块
8. 回程：`GitStatusPayload` → `makeSnapshot(at:)` → `GitFeatureModel` 发布状态 → 视图重绘

**Windows**

1. 功能层：`windows/tauri/src/features/git/api/git-status-api.ts` → `tauriInvoke("git_status", { repoPath })`（含 in-flight 去重与 generation 失效）
2. 边界层：`platform/tauri-core.ts` 的 `invoke` → 非白名单命令走 `platform_invoke` → `adaptCoreResult`
3. 原生 dispatcher：`src-tauri/src/platform.rs` 的 `platform_invoke` → `translate` 得到 `git.status` → 组装信封
4. Rust：`lithe_core::execute_json_with_events`（需要事件时）或 `execute_json`
5. 回程：`core_response` 折回兼容形状 → `core-result-adapter.ts` 适配 → 功能 store 更新

两条链路在**第 3–4 步汇合**：命令名、payload 字段、错误码与事件类型完全一致。这是"一套业务逻辑、两套界面"的技术基础。

## 六、模块生命周期与插件

### 6.1 内置模块（10 个）

真源 `shared/fixtures/modules/built-in-v1.json`：

- `dev.lithe.workspace` —— 唯一 `required: true`，`scope: workspace`，`activationPolicy: eager`，`sleepPolicy: never`。
- `dev.lithe.ai-assistance` —— `scope: application`，空闲 300 秒后休眠。
- `dev.lithe.database`、`dev.lithe.debug`、`dev.lithe.execution`、`dev.lithe.git`、`dev.lithe.language-intelligence`、`dev.lithe.local-history`、`dev.lithe.search`、`dev.lithe.terminal` —— 按需激活，空闲 600 秒后休眠。

字段集：`id`、`displayName`、`scope`、`defaultState`、`activationPolicy`、`sleepPolicy`、`dependencies`、`capabilities`、`contributions`、`required`。

### 6.2 状态机（9 态）

`macos/Sources/LitheModuleAPI/Lifecycle/ModuleTypes.swift:113-123`：

```
disabled · inactive · activating · active · idle
preparingToSleep · sleeping · sleepBlocked(reason:) · failed(message:)
```

配套：`ModuleActivity`（活跃租约数、活跃资源数、最近活动时间）、`ModuleSnapshot`（含 `isQuarantined`、`isSuppressedBySafeMode`）。不变量：禁用模块不持有任何任务/定时器/监听器/子进程；非可中断工作持租约阻塞休眠。

### 6.3 插件面

manifest 由 **Rust Core 解析与校验**（`rust/lithe-core/src/plugins/mod.rs`），关键字段：`schemaVersion`、`id`、`displayName`、`version`（严格三段）、`apiVersion`、`hostCompatibility{minimum, maximumExclusive?}`、`vendor{id, displayName, signatureRequirement}`（当前只接受 `sameTeamAsHost`）、`entrypoint`、`moduleIDs`（必须排序）、`languageSupports[]`。

**两种 entrypoint kind**（`mod.rs:349-377`）：

| kind | 必填 | 必须为空 | 含义 |
| --- | --- | --- | --- |
| `builtIn` | `targetName` | `bundleIdentifier` / `principalClass` / `bundlePath` | 内置插件，随宿主构建 |
| `nativeBundle` | `bundleIdentifier`、`principalClass`、合法的**工作区相对** `bundlePath` | `targetName` | macOS 原生 Bundle 加载 |

校验还包含：catalog 与 plugin API 版本、host 版本精确匹配、plugin/module 标识符严格排序（保证确定性目录合并）、重复 plugin/module 拒绝、空插件拒绝、`languageSupports` 引用的模块必须由本包拥有。官方插件示例见 `Plugins/mac/Official/GoSupport/plugin.json`。

## 七、边界由脚本强制

19 个 `verify-*` 脚本按可运行平台分组：

**跨平台（Node / bash，可在任意平台跑）**

| 脚本 | 校验内容 |
| --- | --- |
| `verify-agent-notes.mjs` | Agent Note 的生命周期、状态行、章节、链接与目录骨架 |
| `verify-editor-boundaries.mjs` | `frontend/editor` 不得 import 平台实现，不得直连 `window.webkit` / `__TAURI__` |
| `verify-java-semantic-ownership.mjs` | 禁止在 Core 或两端界面里二次实现 Java 语义（如 `is_main_method`、`java.runConfigurations`） |
| `verify-download-cache.mjs` | 下载缓存的哈希与命中 |
| `verify-rust-core-comments.sh` | Core 模块文档、导出 Rustdoc、英文注释、unsafe `# Safety` |
| `verify-windows-boundaries.sh` | 禁 `.cpp`/`.h`、禁直连 Tauri core、检查 mocks 未进 vite 配置（供 macOS/Linux 上验证 Windows 边界） |

**仅 macOS**

| 脚本 | 校验内容 |
| --- | --- |
| `verify-core.sh` | 串联 Core verifier + service-boundaries + shared-contracts + java-semantic-ownership |
| `verify-rust-core.sh` | Core 注释检查、`cargo fmt --check`、`lithe-git-host` 与 `lithe-core` 测试、双架构构建 |
| `verify-rust-core-layout.sh` | `src/` 顶层只允许 `lib.rs`；8 个 facade 必须有 `mod.rs`；禁止旧路径导入 |
| `verify-service-boundaries.sh` | Core 层禁用 SwiftUI/AppKit/FileManager/Process；Services 层禁用 `Mac*` 与硬编码路径；View 层不得依赖具体 Service |
| `verify-shared-contracts.sh` | 所有 `shared/fixtures/**/*.json` 与 `contracts/*.schema.json` 可解析，并逐项比对 modules / plugins / github / workbench-background / editor-themes / java-build-report / maven 等 fixture |
| `verify-module-boundaries.sh` | 10 个内置模块 ID 与各 `Lithe*Module` target 实现对应 |
| `verify-macos-app-build-safety.sh` | `WorkbenchView.swift` 不得使用 `drawingGroup()` |
| `verify-macos-package.sh` | JDT LS 与测试插件、双架构 JDK 的打包完整性 |
| `verify-official-plugins.sh` | 按宿主架构构建并校验官方插件包的 manifest 与模块归属 |
| `verify-git-graph.sh` | 造仓后校验 merge 父提交、rebase 分支、remote ref、tag |
| `verify-sparkle-appcast.rb` | Sparkle appcast 的单条更新、版本、URL、长度与 EdDSA 签名 |

**仅 Windows**：`verify-windows-boundaries.ps1`（与 bash 版同规则）。

## 八、构建与 CI

`.github/workflows/` 共 15 个 workflow：

| Workflow | 用途 | Runner |
| --- | --- | --- |
| `ci-macos.yml` | macOS CI：改动分类 → Swift 测试 / Rust Core 测试 / release 构建 → gate | `macos-26`（分类与 gate 在 ubuntu） |
| `ci-windows.yml` | Windows CI：前端 / Rust 测试 / real-jdt → gate | `windows-latest` |
| `ci-database.yml` / `ci-plugins.yml` | 数据库侧车与官方插件测试 | `macos-26` |
| `release-macos.yml` / `release-windows.yml` | 正式发布（含 AtomGit 同步、Cask 更新） | macOS / Windows |
| `release-preview-macos.yml` / `release-preview-windows.yml` | Preview 发布 | macOS / Windows |
| `verify-agent-notes.yml` / `deploy-agent-notes-board.yml` | 校验与部署 Agent Note 决策看板 | `ubuntu-latest` |
| `lithe-issue-claim.yml` / `lithe-issue-priority.yml` / `lithe-pr-review.yml` | issue 认领、标签同步、AI PR review | `ubuntu-latest` |
| `sync-atomgit-release.yml` / `update-repo-charts.yml` | Release 同步与 README 图表 | `ubuntu-latest` |

**关于 Linux 支持的准确表述**：仓库**没有 Linux 产品**。`ubuntu-latest` 只用于改动分类、gate、Node/Ruby 静态校验、发布同步、机器人与看板；两个产品的构建与发布 runner 只有 `macos-*` 与 `windows-latest`。README 徽章也只列 macOS 13+ 与 Windows x64。

**开发基线**：功能开发从上游 `preview` 分支切出（`AGENTS.md` 要求），提交前后按仓库脚本验证（清单见 README 的 Develop Lithe 折叠区）。

## 九、架构约束速查（改代码前先看）

**必须**

- 跨平台确定性行为放 `rust/lithe-core/`；文件系统、进程、终端、运行时、安全、持久化、UI 放平台适配器。
- 新的共享行为先加/改 `shared/` fixture，再让第二个平台依赖它。
- Windows 功能代码经 `@/platform/tauri-core`；兼容命令名经中央 dispatcher 翻译。
- 保持 `operationID`、取消、超时与陈旧结果语义。

**禁止**

- 两端互相 import（Windows 不得依赖 macOS 类型或 Swift 源码）。
- 复活 Qt/C++ 应用层，或为每个 Core 操作新增一个 Tauri 命令。
- 在 Core、Application、Services 层引入 SwiftUI/AppKit/Tauri/WebView2/Win32。
- 在 `frontend/editor` 里 import 平台实现。
- 在 Core 里重建 Java/构建/调试器的语义真源。
- 提交凭据、机器路径、临时目录或工具安装路径到业务逻辑里。

## 参考

- `shared/contracts/rust-core-api.md` —— C ABI、命令信封与 128 条命令
- `shared/contracts/application-boundary.md` —— 功能契约与模块生命周期不变量
- `shared/fixtures/modules/built-in-v1.json` —— 内置模块真源
- `rust/lithe-core/src/` —— Core 模块与 facade
- `rust/lithe-core/include/lithe_core.h` —— C ABI 声明
- `macos/Sources/Lithe/Platform/MacOS/MacServiceContainer.swift` —— macOS 组合根
- `windows/tauri/src-tauri/src/platform.rs` —— Windows 中央 dispatcher
- `scripts/verify-*.sh|.ps1|.mjs` —— 边界强制入口
- `docs/DESIGN.md` —— 设计语言
- `.agents/notes/` —— 架构决策的理由（中文 Agent Note）
- `AGENTS.md` —— 所有权表与验证清单
