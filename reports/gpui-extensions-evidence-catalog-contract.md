# GPUI 扩展层取证 · 一：扩展目录与插件契约

基线 `3085f5d1`，分支 `feat/menu-bar-opaque`。本文件只记**可复核的事实**，每条带文件路径+行号或可复跑命令。

---

## A. 扩展目录与数据模型

### A1【事实·实测】`plugins.listCatalog` 在当前基线不存在，扩展页目录必然加载失败

代码侧调用点：

- `rust/lithe-gpui/crates/extensions/src/core_bridge.rs:5-12` —— `load_catalog()` 发 `core_json("plugins.listCatalog", {"hostVersion": HOST_VERSION})`。
- `rust/lithe-gpui/crates/workbench/src/extensions_page.rs:186` —— 页面构造时 `cx.spawn` 里调 `lithe_gpui_extensions::load_catalog()`，失败即进 `CatalogStatus::Failed`。

Core 侧**没有**这条命令：

- `rust/lithe-core/src/protocol/command.rs:339-...` 的 `CoreCommand::parse` 是命令名唯一解析点。全表 `=> Some(Self::` 共 **148** 条，按命名空间统计为：
  `agent 5 / core 1 / debug 18 / diagnostics 2 / document 2 / editor 2 / execution 1 / file 2 / git 44 / github 3 / history 6 / ideHost 1 / java 11 / lsp 18 / markdown 1 / maven 6 / mybatis 1 / runConfig 8 / spring 1 / workspace 5`
  —— **没有任何 `plugins.*` 条目**。
- `rust/lithe-core/src/runtime/dispatcher.rs:100-106`：解析不到就返回 `ErrorCode::NotSupported` + `"Unsupported core command"`。

实测（本次实际执行，非推断）：

```
CARGO_TARGET_DIR=<workspace>/rust/lithe-gpui/target RUST_MIN_STACK=67108864 \
CARGO_PROFILE_DEV_DEBUG=0 cargo test -p lithe-gpui-extensions -j 2 -- --nocapture
```

结果：`4 passed; 2 failed`，两个失败项均报

```
thread 'core_bridge::tests::load_catalog_matches_the_bundled_fixture_host_version' panicked at
crates\extensions\src\core_bridge.rs:20:46:
bundled catalog should load: "not_supported: Unsupported core command"
thread 'core_bridge::tests::bundled_catalog_covers_every_filter_category' panicked at
crates\extensions\src\core_bridge.rs:26:46:
bundled catalog should load: "not_supported: Unsupported core command"
```

**用户可见后果**：中央扩展页（活动栏第 0 项 `EXTENSIONS_RAIL_IX = 0`，`rust/lithe-gpui/crates/workbench/src/workspace.rs:283`）恒久显示 `lithe.extensions.loadFailed` 空态 + 错误原文，**一条扩展都列不出来**。不是「数据为空」，是「数据源不存在」。

### A2【事实】根因：该命令只活在另一条分支谱系上

```
git merge-base --is-ancestor 302e5d04 HEAD   → exit 1（不是祖先）
git merge-base --is-ancestor 74a55eb0 HEAD   → exit 1（不是祖先）
git branch -a --contains 302e5d04
  LitheUi / chore/sync-macos-and-lithe-core-from-main / feat/lithe-gpui-components
  （含 origin/ 同名远端）
```

- `302e5d04`「新增扩展模块：目录 schema v2 + plugins.listCatalog + 中央扩展页」改动了
  `rust/lithe-core/src/protocol/command.rs (+12)`、`runtime/dispatcher.rs (+16)`、`plugins/mod.rs (+328)`、`tests/plugins.rs (+240)` —— 命令是那时加的。
- `74a55eb0`「chore: 从 main 同步 macos 与 rust/lithe-core」也改 `command.rs (+68)`。
- 二者都**不在**当前 `feat/menu-bar-opaque` 谱系上。当前 HEAD 的 `command.rs` 里检索 `plugins` 命中 **0**。

**即：这是分支谱系分叉造成的接口缺口，不是「还没做」。** 修复路径是移植，不是新写。

### A3【事实】Core 里 `plugins` 模块在 HEAD 上成了孤儿

- `rust/lithe-core/src/lib.rs` 有 `pub mod plugins;`。
- `rust/lithe-core/src/plugins/mod.rs`（385 行）完整存在：`PLUGIN_MANIFEST_SCHEMA_VERSION = 1`（:7）、`PLUGIN_API_VERSION = 1`（:9）、`validate_plugin_catalog_json()`（:209）。
- 但 HEAD 的 `command.rs` 无 `plugins.*`，`dispatcher.rs` 无 `PluginsListCatalog` 分支 ⇒ 该模块只被 `rust/lithe-core/src/tests/plugins.rs` 直接调用，**没有任何生产调用链**。

### A4【事实】schema 版本三方不一致

| 位置 | `schemaVersion` | 证据 |
|---|---|---|
| Core 接受值 | **1** | `plugins/mod.rs:7`；`:215` 不等即 `UnsupportedSchema` |
| `shared/fixtures/plugins/official-v1.json` | **1**（`plugins: []` 空数组） | 文件全文 6 行 |
| `shared/fixtures/plugins/language-support-v1.json` | **1** | 该文件 `:2` |
| `Plugins/mac|win` 真实清单 | **1** | `Plugins/mac/Official/PhpSupport/plugin.json:2` |
| GPUI 测试样本（两处） | **2** | `extensions/src/model.rs:200`、`workbench/src/extensions_page.rs:647` |
| 原 v2 设计（另一谱系） | **2** | `git show 302e5d04:shared/fixtures/plugins/official-v2.json` |

`rust/lithe-gpui` 两处测试样本写 `schemaVersion: 2` 且能被 `CatalogSnapshot::from_response` 解析成功——因为 **GPUI 的 `WirePlugin` 是自定义宽松反序列化**，不校验 `schemaVersion`/`apiVersion`/`hostCompatibility`：

- `extensions/src/model.rs:48-52` `CatalogResponse` 只取 `plugins` 字段，其余顶层字段全被忽略。
- `extensions/src/model.rs:56-75` `WirePlugin` 无 `apiVersion`、无 `hostCompatibility`、无 `moduleIDs` 字段。

**后果：Core 的清单校验语义（版本区间、模块归属、入口一致性、排序）在 GPUI 侧被完全绕过。** 这与硬约束「前端不重新实现 Core 的清单校验/合并语义」并不冲突（前端确实没实现），但意味着**前端当前也没有消费 Core 的校验结果**——因为那条通道断了（A1）。

### A5【事实】GPUI 侧存在两个互不相交的扩展 id 空间

| 平面 | id 集合 | 证据 |
|---|---|---|
| catalog 平面（目录 fixture） | `lithe.builtinThemes`、`lithe.bundledFonts`、`lithe.ideaIconTheme`、`lithe.javaDebugAdapter`、`lithe.javaLanguageSupport` | `git show 302e5d04:shared/fixtures/plugins/official-v2.json` |
| settings 平面（`BUNDLED_EXTENSIONS`） | `lithe.gruvboxTheme`、`lithe.litheTheme`、`lithe.nordTheme`、`lithe.oneTheme`、`lithe.projectLanguages`、`lithe.vscodeTheme`、`lithe.ibmPlexMonoFont`、`lithe.sourceCodeProFont` | `rust/lithe-gpui/crates/settings/src/extensions.rs:22-122` |

两组 id **零交集**。而 `rust/lithe-gpui/crates/settings/src/store.rs:457` 的 `set_extension_uninstalled` 用 `crate::extensions::bundled_extension(id)` 校验 id，该函数（`extensions.rs:124-126`）只在 settings 平面的 8 条里查。

**推论（建议，非事实）**：即便 A1 修好、目录能列出 `lithe.builtinThemes`，点它的「卸载」也会走 `store.rs:457` 判空 → 打 `S1_SETTINGS extension_unknown id=...` 并**静默返回**，UI 无反馈。两个平面必须先对齐 id 或建立映射。

---

## B. 插件清单与能力 API 契约

### B1【事实·实测】GPUI 侧完全没有 IDE 能力 API / MCP 连接面

在 `rust/lithe-gpui` 全目录检索 `(?i)mcp|host[-_]?api|ideHost|ide_host|ide-capabilit|ide_capabilit`：

命中 **6** 处，全部是 `explorer_view.rs` / `workspace.rs` 里名为 `hide_host` 的**无关局部变量**（`explorer/src/explorer_view.rs:82,141,156,157,488`、`workbench/src/workspace.rs:666`）。

⇒ **未找到实现入口（检索关键词 `mcp` / `MCP` / `host-api` / `HostApi` / `ideHost` / `ide-capabilit`，真实实现命中 0）。** 台账 §2.3 该条结论复核成立。

### B2【事实】契约的宿主是 `lithe-ide-host`，不是 `lithe-core` 的新命令

- `shared/contracts/ide-api/v1.md:10-14`：「`rust/lithe-ide-host/src/catalog.rs` owns the tool names, schemas, validation, permission classification and UTF-8 output paging.」
- `rust/lithe-ide-host/src/catalog.rs:30-51` 定义 **13** 个工具（`tools()` 里 `("lithe_*"` 条目计数 = 13，逐名枚举已核）：
  `lithe_project_inspect`、`lithe_environment_inspect`、`lithe_environment_configure`、`lithe_maven_inspect`、`lithe_maven_configure`、`lithe_maven_reload`、`lithe_maven_execute`、`lithe_run_list`、`lithe_run_start`、`lithe_run_restart`、`lithe_operations_list`、`lithe_operation_output`、`lithe_operation_stop`。
  与 `shared/contracts/ide-api/v1.md:51-65` 的能力表行数一致（同为 13）。
- 三档授权在 `catalog.rs:10-15` `Permissions { configure, execute }`，校验在 `:54-137`。
- 唯一入口是 Core 的 `ideHost.control`：`protocol/command.rs:342` → `runtime/dispatcher.rs:110-118` → `lithe_ide_host::execute(action, args)`。
- `ideHost.control` 的 action 白名单在 `shared/contracts/ide-api/v1.md:38`：`open, close, closeOwner, poll, respond, catalog, validate, output`。
- `rust/lithe-ide-host/src/lib.rs:309-391` 是该 `execute` 的实现（`output` / `closeOwner` / `open` / `catalog` / `validate` / `close` / `poll` / `respond`）。

**关键**：`ideHost.control` **是**在当前基线 Core 命令表里的（`command.rs:342`）。也就是说，**能力 API 的通道在 Core 侧是通的，缺的只是 GPUI 宿主侧的消费者**。

### B3【事实】两端都有真实消费者，GPUI 没有

| 端 | 消费者 | 证据 |
|---|---|---|
| Windows | `windows/tauri/src/features/host-api/ide-capabilities.ts`（404 行）`hostControl()` → `invoke("ideHost.control", …)`（:24-25）；`IdeCapabilities` 类（:52-403）实现 14 个工具的宿主语义 | 矩阵 `ide-api-plugins` windows.evidence |
| Windows | `windows/tauri/src/features/host-api/mcp-settings.tsx`、`mcp-lifecycle.ts`、`mcp-connection.ts` | 矩阵 `ide-mcp-connection` windows.evidence |
| macOS | `macos/Sources/LitheModuleAPI/Plugins/IDECapabilities.swift`、`macos/Sources/Lithe/Application/Composition/IdeCapabilityRegistry.swift`、`Views/App/McpSettingsView.swift` | 矩阵 `ide-api-plugins` / `ide-mcp-connection` macos.evidence |
| GPUI | — | B1：命中 0 |

`lithe-ide-host` 在 `rust/lithe-gpui/Cargo.lock:4147` 出现（经 `lithe-core` 传递），但 `rust/lithe-gpui/crates/*/Cargo.toml` 无任何直接 `ide-host` 依赖。

### B4【事实】契约明确划定了「不做什么」，这决定缺口性质

`shared/contracts/ide-api/v1.md:118-119`：

> This adds a capability surface; it does not introduce a new plugin marketplace, sandbox or UI extension SDK.

⇒ 能力 API 缺口的**性质是「宿主侧消费者未接线」**，不是「契约要扩」。这与 A1 的性质不同（A1 是接口缺失）。

### B5【事实】矩阵里本维度的三条条目，两列都写 `implemented`，且证据里没有 GPUI

| 矩阵 id | area | macos | windows | GPUI |
|---|---|---|---|---|
| `ide-api-plugins` | 插件与扩展 | implemented / pending | implemented / pending | 无此列 |
| `ide-mcp-connection` | AI | implemented / pending | implemented / pending | 无此列 |
| `plugin-extension-management` | 扩展 | implemented / pending | implemented / pending | 无此列 |
| `php-optional-plugin` | 语言支持 | implemented / pending | implemented / pending | 无此列 |

`shared/platform-feature-matrix/metadata.json` 只有 `macos` / `windows` 两列，`schemaVersion: 4`，`statusDefinitions` 只定义两套状态。

**未决项（沿用既有文档）**：`docs/development/gpui-parity-plan-2026-10.md:344-348` 把「新前端状态该记到哪」列为待确认——新增 gpui 平台字段，还是暂记到 `windows` 端。这条不解决，阶段回填会返工。

### B6【事实】两个真实插件包的清单 schema 互不兼容

| | macOS | Windows |
|---|---|---|
| 路径 | `Plugins/mac/Official/PhpSupport/plugin.json` | `Plugins/win/Official/PhpSupport/plugin.json` |
| 顶层字段 | `schemaVersion/id/displayName/version/apiVersion/hostCompatibility/vendor/entrypoint/modules/languageSupports` | `id/name/displayName/description/version/publisher/categories/main/languages/lsp/runActions/installation` |
| 入口 | `entrypoint.kind = "nativeBundle"`（:17） | `main: "plugin.js"`（:11） |
| 签名 | `vendor.signatureRequirement = "publisherPackage"`（:14） | 无 |
| 语言贡献 | `languageSupports[].fileExtensions`（无点） | `languages[].extensions`（带点，9 项 vs 2 项） |
| 版本 | `0.3.0` | `1.0.0` |

⇒ 跨端插件包格式**当前不统一**。GPUI 的 `WirePlugin` 只认 macOS 那一套（`vendor.displayName` / `entrypoint.kind` / `languageSupports` / `fileExtensions`），对 Windows 那一套解析会失败（`vendor` 必填，`WireVendor.display_name` 无 `#[serde(default)]`，`model.rs:77-81`）。

---

## 本文件结论索引

| # | 结论 | 性质 | 证据 |
|---|---|---|---|
| A1 | `plugins.listCatalog` 不存在，扩展页目录必然失败 | 事实·实测 | 命令原文 + 测试输出 |
| A2 | 根因是分支谱系分叉，非未实现 | 事实 | `merge-base --is-ancestor` exit 1 |
| A3 | Core `plugins` 模块成孤儿 | 事实 | `lib.rs` vs `command.rs` |
| A4 | schemaVersion 三方不一致，GPUI 宽松解析绕过校验 | 事实 | 四处行号 |
| A5 | 两个 id 空间零交集 ⇒ 卸载必然静默失败 | 事实+推论 | 两组 id 枚举 |
| B1 | GPUI 无 MCP / 能力 API 消费者 | 事实·实测 | 检索命中 0 |
| B2 | 契约宿主是 `lithe-ide-host`，通道在 Core 侧已通 | 事实 | `command.rs:342`；13 个工具 |
| B3 | 两端有消费者、GPUI 无 | 事实 | 矩阵 evidence + 文件 |
| B4 | 契约明示不引入新市场/沙箱/UI SDK | 事实 | `v1.md:118-119` |
| B5 | 矩阵无 gpui 列，登记方式未决 | 事实 | `metadata.json` |
| B6 | 两端插件包 schema 不兼容 | 事实 | 两份 `plugin.json` |
