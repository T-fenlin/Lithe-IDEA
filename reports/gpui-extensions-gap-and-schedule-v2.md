# 柯展之 — GPUI 插件扩展层：缺口复核、纠错与排期（v2）

**负责维度**：只覆盖插件与扩展层——扩展目录与数据模型、扩展管理页、内置扩展打包、插件清单 / IDE 能力 API 契约。
不涉及编辑器内核、渲染、LSP/DAP 语义、CI/CD，也不涉及扩展页的视觉规格与组件选型。

**基线**：`3085f5d1`，分支 `feat/menu-bar-opaque`（与主理人文档一致，本轮未变）。
**工作树**：`M` 4 个 `workspace_config/*.rs` + `lithe-ui/src/primitives/icon.rs`；`??` 为文档与 `reports/`。均不触及 `crates/extensions`、`crates/settings`、`crates/workbench`，故本轮 extensions 侧结论不受工作树影响。

**本文件与本维度既有三份产出的关系**：
`reports/gpui-extensions-evidence-catalog-contract.md`（A1–A5、B1–B6）、
`reports/gpui-extensions-evidence-page-bundled.md`（C1–C6、D1–D7、E）、
`reports/gpui-extensions-verdicts.md`（5 条裁决）。
本文件是**在新一轮独立复核基础上的增量**：逐条重验 + 新增 4 条此前两份都未覆盖的发现（N1–N4）。

---

## 结论

### 一句话

GPUI 扩展层的问题不是「功能少」，而是**三层同时断链**：**取数通道**断（目录命令不存在）、**身份体系**断（两套 id 零交集）、**版本轴**断（宿主 `0.0.1` 对不上插件要求的 `0.3.0`）。扩展页当前是一个**必然失败、无法重试、且卸载按钮永不可达**的页面。

### 对本维度四个分工问题的直接回答

**问题 1：`crates/extensions` 相对 `windows/tauri` 与 `Plugins/` 的差距。**

差距不在行数比例，而在**能力性质**：

| 维度 | GPUI | Windows 旧端 | 倍数 / 性质 |
|---|---|---|---|
| 扩展 crate 代码 | `crates/extensions/` **3 个 `.rs`**（`lib.rs` 8 行 / `model.rs` 284 / `core_bridge.rs` 37）；公共 API **只有 `load_catalog` 一个函数** | `windows/tauri/src/extensions/` **17 个子目录** | 量级差 ~100× |
| 生命周期动作 | **0 条**（装卸按钮不可达，无启停） | **5 条独立函数**：`installExtensionLifecycle:153` / `uninstallExtensionLifecycle:302` / `enableExtensionLifecycle:357` / `disableExtensionLifecycle:443` / `updateExtensionLifecycle:530` | 5 vs 0 |
| 数据模型 | `ExtensionSnapshot` 只读快照，无状态字段 | `types/extension-manifest.ts` 含 `installed` / `state`(`not-installed|installing|installed`) / `isEnabled` / `installation` | 只读 vs 全生命周期 |
| 发行物 | 无 `Plugins/` 对应物 | `Plugins/win/`（TS Worker 包，`lithe.php-1.0.0.lithe-extension`） | 无 vs 有 |

**关键判断**：`crates/extensions` 是**读取器**，不是**管理器**。它的公共面 `pub use core_bridge::load_catalog;`（`lib.rs:4`）只做一件事——把目录 JSON 解析成展示快照。所有写操作（安装/卸载/启停）都不在该 crate，也不在任何 crate。

**问题 2：`mcp`/`host-api` 检索命中 0 而矩阵两端 implemented —— 是不是缺口？契约在哪一层？**

**是缺口，且是「宿主侧消费者未接线」，不是「契约要扩」。** 契约分居两层，位置明确：

| 层 | 位置 | 状态 |
|---|---|---|
| **契约与校验**（工具名、参数、权限分级、分页） | `rust/lithe-ide-host/src/catalog.rs:30-51`，**13 个 `lithe_*` 工具**；权限 `Permissions { configure, execute }`（`:10-15`） | **已在当前基线**，无需新建 |
| **Core 命令通道** | `ideHost.control` —— `protocol/command.rs:342` → `runtime/dispatcher.rs:110-118` → `lithe_ide_host::execute(action, args)` | **已通**（与 `plugins.listCatalog` 的缺失形成对照） |
| **宿主侧消费者** | Windows：`windows/tauri/src/features/host-api/ide-capabilities.ts`（404 行，`invoke("ideHost.control", …)`）；macOS：`IDECapabilities.swift` + `IdeCapabilityRegistry.swift` | 两端**有**，GPUI **无** |

**契约的边界是刻意划定的**：`shared/contracts/ide-api/v1.md:118-119` 原文——
「This adds a capability surface; it does not introduce a new plugin marketplace, sandbox or UI extension SDK.」
⇒ 缺的是消费者，不是契约。**本维度不得以「要接 MCP」为由扩契约。**

**问题 3：删除旧两端后，插件契约由谁承载？**

**承载者应分三份，只有一份已就位：**

| 契约内容 | 当前承载者 | 删旧端后是否幸存 |
|---|---|---|
| 清单 schema / 兼容性校验 / 目录合并语义 | `rust/lithe-core/src/plugins/mod.rs`（385 行，`PLUGIN_MANIFEST_SCHEMA_VERSION=1`、`PLUGIN_API_VERSION=1`、`validate_plugin_catalog_json`） | ✅ **幸存**，但**成孤儿**（见 N2） |
| IDE 能力 API（13 工具 / 权限分级） | `rust/lithe-ide-host/src/catalog.rs` + `shared/contracts/ide-api/v1.md` | ✅ **幸存且已被 Core 接线** |
| **本地化文案键** | `windows/tauri` 与 `macos` 的各自 locale | ❌ **随旧端删除**——这是最容易被忽略的迁移风险（见 R5） |
| 插件包本体 | `Plugins/mac/`（Swift 包）、`Plugins/win/`（TS Worker 包） | ⚠️ **需决策**：GPUI 无第三份对应物 |

**明确建议**：`shared/` + `rust/lithe-core/plugins/` + `rust/lithe-ide-host/` 构成契约承载层，删旧端不影响；
但 **`Plugins/mac|win` 的产物格式必须先定 GPUI 的装载边界**，否则「删掉旧两端」= 同时删掉两端唯一的插件包实现。

**问题 4：主理人把 #100「插件能力 API」判为「部分」、#101「扩展管理页」判为「部分」——这个判法。**

**#100 判轻了，应拆成三条**（`ide-api-plugins` / `ide-mcp-connection` 是**完全缺失**，不是部分）；
**#101 判法正确**（扩展页确有 652 行实现，但只有 2 个持久化动作）。

### 缺口清单（本维度，替代主理人 §1.8 的 #98–#102）

| # | 缺口 | 性质 | GPUI 现状 | 证据 |
|---|---|---|---|---|
| E1 | 扩展目录取数通道不存在 | **接口缺失**（非未实现） | 扩展页恒显失败态 | 见 N1（实测） |
| E2 | 宿主版本轴错配 | **接口缺失（新发现）** | 即使移植命令也会被 `IncompatibleHost` 拒 | 见 N3 |
| E3 | 无安装扩展入口 | 完全未实现 | 无 | Windows 5 条 lifecycle vs GPUI 0 |
| E4 | 无启用/禁用 | 完全未实现 | 无 | `CatalogStatus` 只有 Loading/Loaded/Failed；macOS 有 `Enable/Disable` |
| E5 | 卸载/重装入口永不可达 | **判定恒假的接线缺陷** | `removable` 恒 `false` | `extensions_page.rs:112` + Core `valid_entrypoint` |
| E6 | IDE 能力 API 无消费者 | 宿主侧未接线 | 检索命中 0 | 见「问题 2」 |
| E7 | MCP 连接面全缺 | 宿主侧未接线 | 检索命中 0，**文案键也没有** | 台账 §2.3 |
| E8 | 内置扩展表与目录契约无关联 | 双份真源 | `settings` 不依赖 `extensions` | `settings/Cargo.toml` 无该依赖 |
| E9 | 无 `Plugins/` 插件包对应物 | 完全未实现 | 无 | `Plugins/mac|win` 存在，GPUI 无 |
| E10 | 扩展页失败后无法重试 | 接线缺陷 | `retryLoading` 键已备好零调用 | `extensions_page.rs:185-201` |
| E11 | 扩展页页头/空态图标留空 | 注释过时 | 两处空槽 | `PLUGIN_ICON` 已登记且在活动栏使用 |

**E5 的双向确认（补强）**：
- 接受侧：`rust/lithe-core/src/plugins/mod.rs:349-377` `valid_entrypoint()` 只认 `"builtIn"` / `"nativeBundle"`，`_ => false`。
- 产出侧：全仓 `*.json` 检索 `"bundled"` —— 命中的全是 `windows/tauri/src/extensions/bundled/themes/*/extension.json:10` 与图标主题的 `"bundled": true`（旧端字段）与工作台背景 fixture，**无一是插件 `entrypoint.kind`**。
⇒ 消费者在（`extensions_page.rs:112`），生产者在（Core 契约）明确拒绝。**判定恒假。**

### 与主理人文档的差异汇总

| 主理人文档 | 本轮结论 | 差异性质 |
|---|---|---|
| #98 MCP「缺失」，判据「检索命中 0（文案键也没有）」 | **成立**，补强：契约在 `lithe-ide-host`，Core 通道已通 | 补强 |
| #99 Host API「缺失」 | **成立**，性质是「宿主侧未接线」 | 补强 |
| #100 插件能力 API「**部分**」 | **判轻**：应拆为 `ide-api-plugins` / `ide-mcp-connection`，二者 GPUI 侧**完全缺失** | **纠错** |
| #101 扩展管理页「部分」 | **成立**，补：652 行但只有 2 个持久化动作，且装卸按钮不可达 | 补强 |
| #102 社区「缺失」 | **成立** | 一致 |
| §1.8 未提「宿主版本轴」 | **漏项**：`HOST_VERSION=0.0.1` vs 插件要求 `0.3.0` | **遗漏** |
| §2.10 矩阵缺 gpui 字段 | **成立**，且本维度影响的正是 4 条插件相关条目 | 补强 |

---

## 补充与纠错（逐条）

### N1【事实·实测·本轮新跑】`plugins.listCatalog` 在当前基线不存在——运行时确认

**独立复核过程**（不依赖既有报告）：

1. 命令表逐条清点：`rust/lithe-core/src/protocol/command.rs:339` 起 `CoreCommand::parse`，`=> Some(Self::` 共 **148** 条（`Select-String` 逐行计数 = 148，总匹配数亦 = 148，二者一致）。
   **口径更正记录**：本轮首次检索用正则 `=> Some\(Self::|pub fn parse`，得 149 —— 多出的 1 条是 `pub fn parse` 那一行本身，属**我的正则口径错误**，已复算纠正为 148。此为纠错，非分歧。
   逐命名空间查看 `:341-488` 全表，**无任何 `plugins.*`**。在 `command.rs` 内单独检索 `plugin` ⇒ **命中 0**（独立复验）。
2. 失败路径：`runtime/dispatcher.rs:100-106` —— 解析不到即 `ErrorCode::NotSupported` + `"Unsupported core command"`。
3. **实测**（本轮实际执行）：

```
rust\lithe-gpui\target\debug\deps\lithe_gpui_extensions-abe6f30395dbd9a2.exe --nocapture
→ test result: FAILED. 4 passed; 2 failed; exit=101

core_bridge::tests::load_catalog_matches_the_bundled_fixture_host_version
  panicked at crates\extensions\src\core_bridge.rs:20:46:
  bundled catalog should load: "not_supported: Unsupported core command"
core_bridge::tests::bundled_catalog_covers_every_filter_category
  panicked at crates\extensions\src\core_bridge.rs:26:46:
  bundled catalog should load: "not_supported: Unsupported core command"
```

**口径声明**：执行的是**既有测试二进制**（`target/debug/deps/` 下已存在），不是新跑 `cargo test`——因此不声称「重新构建并验证」。等价性依据：工作树改动不含 `crates/extensions` / `crates/settings` / `crates/workbench`（`git status --porcelain` 已核），且报错文案与 `dispatcher.rs:103` 逐字一致。**构建耗时未在本轮产生**。

**用户可见后果**：扩展页（`EXTENSIONS_RAIL_IX = 0`，`workspace.rs:283`）恒显 `lithe.extensions.loadFailed` + 错误原文，**一条扩展都列不出**。

### N2【事实·本轮独立复核】Core `plugins` 模块是孤儿（既有结论成立）

- `rust/lithe-core/src/lib.rs` 有 `pub mod plugins;`。
- `plugins/mod.rs` 385 行完整：`PLUGIN_MANIFEST_SCHEMA_VERSION = 1`（:7）、`PLUGIN_API_VERSION = 1`（:9）、`validate_plugin_catalog_json`（:209）。
- 全 `rust/lithe-core/src` 检索 `plugins.listCatalog|official-v1|PluginCatalog` ⇒ 只命中 `tests/plugins.rs:4`（读 fixture）与 `plugins/mod.rs` 自身。
⇒ **无生产调用链**，仅测试直接调用。

### N3【新发现·两份既有报告与主理人文档均未覆盖】宿主版本轴三方错配

| 位置 | 版本值 | 证据 |
|---|---|---|
| GPUI 宿主（请求方） | **`0.0.1`** | `crates/shared/src/lib.rs:24` `pub const HOST_VERSION: &str = env!("CARGO_PKG_VERSION");`；`crates/shared/Cargo.toml:3` = `0.0.1`；`crates/app/Cargo.toml:3` = `0.0.1` |
| Core 目录 fixture | **`0.3.0`** | `shared/fixtures/plugins/official-v1.json`（全文 6 行，`plugins: []`）的 `hostVersion` |
| 真实插件包要求 | **`minimum 0.3.0` / `maximumExclusive 0.4.0`** | `Plugins/mac/Official/PhpSupport/plugin.json:8-9` |
| `lithe-core` 自身版本 | `0.1.0` | `rust/lithe-core/Cargo.toml:3` |

而 Core 的校验**严格要求相等**：`plugins/mod.rs:227-232`
```rust
let catalog_host = parse_version("catalog", &catalog.host_version)?;
if catalog_host != host_version {
    return Err(PluginValidationError::IncompatibleHost { plugin: "catalog".into() });
}
```

⇒ **即使把 `plugins.listCatalog` 原样移植到当前基线**，GPUI 传 `0.0.1` 也会得到 `IncompatibleHost`。
**这是「移植命令即可」这一结论的推翻条件之一**——移植必须同时决定版本轴对齐（改 `HOST_VERSION`、改 fixture，或让校验放宽）。**属架构师裁决范围，我不单方面改。**

### N4【新发现】目录响应的 `categories` 是**派生字段**，不在清单 schema 里——移植有第二层工作量

- GPUI 消费侧：`crates/extensions/src/model.rs:64` `categories: Vec<String>`，**无 `#[serde(default)]`**，即**必填**。
- Core 清单结构 `PluginPackageManifest`（`plugins/mod.rs:112-133`）**没有 `categories` 字段**，也没有 `themes`/`icons`/`fonts`/`debugAdapters`。
- 历史提交 `302e5d04` 显示：这些是 **`PluginCatalogEntry` 在 manifest 之外额外派生的**——
  `pub struct PluginCatalogEntry { #[serde(flatten)] pub manifest: PluginPackageManifest, pub categories: Vec<&'static str> }`，
  且 `PluginPackageManifest` 上另有 `themes` / `icon_themes` / `fonts` / `debug_adapters` 贡献字段与 `fn categories()` 派生函数。

⇒ 当前基线的 `plugins/mod.rs` **缺少**这些贡献字段与派生逻辑。移植 = 「加命令」**+** 「补 manifest 贡献字段 + 派生 `categories` + 重建 fixture」，
工作量显著高于「加一条命令」。

**移植范围实测**（`git show 302e5d04 --stat`，29 文件 / +2637 行）：

| 文件 | 变化 | 当前基线状态 |
|---|---|---|
| `rust/lithe-core/src/protocol/command.rs` | +12 | 无 `plugins.*` |
| `rust/lithe-core/src/runtime/dispatcher.rs` | +16 | 无 `PluginListCatalog` 分支 |
| `rust/lithe-core/src/plugins/mod.rs` | +328 | 385 行但无贡献字段/派生 |
| `rust/lithe-core/src/tests/plugins.rs` | +240 | 存在 |
| `shared/contracts/plugin-catalog.md` | +83 | **不存在**（`Test-Path` = False） |
| `shared/fixtures/plugins/official-v2.json` | +195 | **不存在**（仅存于 `302e5d04`） |
| `shared/fixtures/plugins/official-v1.json` | **删除**（-6） | **存在**，6 行空数组 |
| `crates/workbench/src/workspace.rs` | 458 行改动 | — |
| `crates/workbench/src/right_tool_window.rs` | 161 行改动 | — |
| `language-support-v1.json → -v2.json` | 重命名 +10 | 现名 **`language-support-v1.json`** |

⇒ **v1 与 v2 是互斥的两套**：基线保有 v1（空数组、`hostVersion 0.3.0`），历史 v2（5 条目、`hostVersion 0.0.1`）。
移植必须**先决策 v1/v2 取舍**，且 `scripts/verify-shared-contracts.sh:137` 硬断言 `schemaVersion == 1` ——
**v2 fixture 无法通过该门禁**（我未修改脚本，仅报告冲突）。

### N5【纠正我自己的假警报】i18n 键名**没有**不匹配问题

本轮我先发现「页面调 `tr("lithe.extensions.title")` 而 yml 里写着 `extensions.title:`」，一度怀疑键名错。
**读 yml 结构后排除**：`lithe.en.yml:11` 是顶层 `lithe:`，`:6274` 的 `  extensions.title:` 为**两空格缩进**，
故完整路径正是 `lithe.extensions.title`；`i18n.rs:43-999` 的测试 `every_wired_key_resolves_in_both_locales` 亦逐键断言其可解析。
⇒ **i18n 层无缺口，键名正确。** 记录此条以防后人重踩。

---

## 证据与定位

### 本轮实际执行过的验证

| 验证 | 命令 | 结果 |
|---|---|---|
| 扩展 crate 测试（既有二进制） | `rust\lithe-gpui\target\debug\deps\lithe_gpui_extensions-abe6f30395dbd9a2.exe --nocapture` | `4 passed; 2 failed`，exit=101，失败原文 `not_supported: Unsupported core command` |
| 命令表清点 | `Select-String '=> Some\(Self::' command.rs` | **148** 条，无 `plugins.*`；`plugin` 在该文件命中 0 |
| 谱系核验 | `git merge-base --is-ancestor 302e5d04 HEAD` | `exit=1`（非祖先）；`git branch -a --contains 302e5d04` → `LitheUi` / `chore/sync-macos-and-lithe-core-from-main` / `feat/lithe-gpui-components` |

### 静态检索与文件定位

| 结论 | 定位 |
|---|---|
| 扩展 crate 公共面只有 `load_catalog` | `crates/extensions/src/lib.rs:4`；`core_bridge.rs:4-12` |
| 快照模型（只读，无状态字段） | `crates/extensions/src/model.rs:7-33`；`CATEGORIES` 6 项 `:4` |
| 目录调用点 | `extensions/src/core_bridge.rs:5-12`；`workbench/src/extensions_page.rs:186` |
| 管理页动作仅 2 个持久化 | `extensions_page.rs:214`（切图标集）、`:222`（卸载/重装）、`:232`、`:328-339`、`:524` |
| `removable` 恒假 | `extensions_page.rs:112` + Core `plugins/mod.rs:349-377` |
| 状态机只有 3 态 | `extensions_page.rs:37-42`、`:529-535` |
| 无重试入口 | `extensions_page.rs:185-201`、`:531-533` |
| `PLUGIN_ICON` 已登记 | `crates/shared/src/icons/idea.rs:1032-1034`；生产使用 `workbench/src/workspace.rs:3254` |
| 内置扩展表 8 条 | `settings/src/extensions.rs:22-122`；`bundled_extension()` `:124-126`；`uninstall` `:361` / `reinstall` `:382` |
| `settings` 不依赖 `extensions` | `crates/settings/Cargo.toml` 无 `lithe-gpui-extensions`；`extensions.rs` 无 `core_json` |
| 卸载写入校验 | `settings/src/store.rs:451-481`（`:457` 判空 → `S1_SETTINGS extension_unknown` → 静默 return） |
| 能力 API 契约 | `rust/lithe-ide-host/src/catalog.rs:30-51`（13 工具）、`:10-15`（权限） |
| 能力 API 通道已通 | `protocol/command.rs:342` → `dispatcher.rs:110-118` |
| 契约边界 | `shared/contracts/ide-api/v1.md:118-119` |
| GPUI 无 MCP/能力 API 消费者 | 检索 `(?i)mcp\|host_api\|ideHost\|ide-capabilit` 命中 **6** 处，全为 `explorer_view.rs:82,141,156,157,488` 与 `workspace.rs:666` 的 `hide_host` 无关变量 |
| 矩阵条目 | `shared/platform-feature-matrix/features/{ide-api-plugins,ide-mcp-connection,plugin-extension-management,php-optional-plugin}.json`；`metadata.json:1-2`（`schemaVersion 4`，仅两列） |
| Windows lifecycle 5 条 | `windows/tauri/src/extensions/registry/extension-store-lifecycle.ts:153,302,357,443,530` |
| Windows 扩展子系统规模 | 17 子目录：`bundled/` 6 044 712 B、`registry/` 107 145 B、`ui/` 154 100 B、`tooling/` 76 842 B、`themes/` 76 535 B、`icon-themes/` 36 642 B、`languages/` 21 619 B、`types/` 17 278 B、`loader/` 13 040 B、`viewers/` 12 902 B、`run/` 12 437 B、`installer/` 10 728 B、`packages/` 9 161 B、`runtime/` 8 753 B、`hooks/` 5 319 B、`marketplace/` 5 006 B、`catalog/` 3 806 B |
| 两端 manifest 不兼容 | `Plugins/mac/Official/PhpSupport/plugin.json`（`vendor`/`entrypoint`/`languageSupports`/`fileExtensions` 无点）vs `Plugins/win/Official/PhpSupport/plugin.json`（`publisher`/`main`/`languages[].extensions` 带点 9 项 vs 2 项） |

### 未执行 / 未覆盖

- **未启动 Lithe 应用做界面实机验证**：扩展页的失败态、空态、卸载按钮不出现，**均为静态推理 + 单测证据，未截图确认**。
- **未跑** `scripts/verify-shared-contracts.sh`、`scripts/generate-platform-feature-matrix.mjs`、`scripts/verify-platform-feature-matrix.sh`（Windows 主机无 `/usr/bin/ruby`，脚本为 bash + ruby 依赖）。
- **未跑** `cargo test -p lithe-gpui-extensions`（新构建）——只执行了既有测试二进制，见 N1 口径声明。
- **未验证** macOS 侧 `IDECapabilities.swift` / `Plugins/mac` 的实际可用性（无 macOS 环境）。
- **未评估** Windows 旧端 `extensions/` 1179 文件中有多少是 GPUI 真正需要迁移的（属工作量的第二阶段，需产品决策先定）。

---

## 风险与条件

| # | 风险 | 触发条件 | 影响 |
|---|---|---|---|
| R1 | 按「移植一条命令」估工 | 直接采信既有报告 P0-a 的 1.5 人日 | **低估**：还需补 manifest 贡献字段 + `categories` 派生 + contracts 文档 + v1/v2 取舍（N4） |
| R2 | 移植后仍被拒 | 只移命令、不动版本轴 | 得到 `IncompatibleHost`（N3），白做一轮 |
| R3 | 两个 id 空间不对齐就先修通道 | 只打通目录 | 目录能列条目，点卸载走 `store.rs:457` 判空 → `S1_SETTINGS extension_unknown` → **静默返回**，用户以为坏了 |
| R4 | 契约越界 | 改 `entrypoint.kind` 合法取值、或统一两端 manifest schema | 触及 `rust/lithe-core` 清单语义，**属架构师裁决范围** |
| R5 | **删旧端时丢掉唯一的插件包实现** | 直接删 `Plugins/mac\|win` 而无 GPUI 替代 | 两端唯一的真实插件包（PHP/Go）消失；且本地化文案键随旧端 locale 一同消失 |
| R6 | 矩阵登记方式未定就回填 | `metadata.json` 仅两列 | 4 条插件条目回填返工 |
| R7 | 旧端 `false` 开关误导缺口判断 | 参照 `backend-capabilities.ts` | 已注意：`extensions: true`（`backend-capabilities.ts:8`），故扩展类**不适用**主理人的「旧端自己也是待开发」排除线 |

### 假设（未验证）

- **H1**：`plugins.listCatalog` 移植可行。**部分验证**——源实现存在于 `302e5d04`（已读到 `list_catalog` 与 `PluginCatalogEntry` 源码），但**未尝试 cherry-pick**，且 N3/N4 表明需额外工作。
- **H2**：修好 E1 后扩展页能列出条目。**未验证**——未运行应用。
- **H3**：`removable` 的正确口径应是「内置扩展不可卸载」（即 E5 是有意设计，改注释而非改代码），而非改 Core 接受 `bundled`。**需裁决**。
- **H4**：GPUI 的产品定位是「管理内置扩展」而非「插件市场」。**未验证**——若成立则 E3/E9 不构成缺口（契约 `v1.md:118-119` 支持这一读法）。

### 可能推翻结论的证据

- 若 `feat/lithe-gpui-components` 谱系即将合入当前分支 ⇒ E1/E4 是**暂时的分支状态**，工作量大幅下调。
- 若产品明确 GPUI 不做插件市场（只做内置扩展）⇒ E3/E9 从缺口转为**范围外**。
- 若版本轴由发布流程统一（如 GPUI 版本号即将从 `0.0.1` 提到 `0.3.x`）⇒ N3 自然消解，R2 不成立。

---

## 建议与验收

### 行动项（含依赖与人日）

| 优先级 | 行动 | 依赖 | 人日 | 完成标准 |
|---|---|---|---|---|
| **P0-a** | **决策并在架构层落地宿主版本轴**（N3）：统一 `HOST_VERSION` / fixture / 插件包 `hostCompatibility` 三者 | **架构师裁决**（R4） | 1.0 | 三方版本在同一区间内；`IncompatibleHost` 不再出现 |
| **P0-b** | 移植 `plugins.listCatalog`：`command.rs` + `dispatcher.rs` 分支 + `plugins/mod.rs` 贡献字段与 `categories` 派生 + 重建 fixture + 同步 `verify-shared-contracts.sh` | P0-a，架构师确认 v1/v2 取舍 | **3.0**（原估 1.5，按 N4 上调） | `cargo test -p lithe-gpui-extensions` 全绿（当前 2 红） |
| **P0-c** | 修 `removable` 判定（E5）：或改前端口径、或按契约产出正确 kind | 架构师裁决 H3 | 0.5 | 扩展页出现卸载按钮，或明确记「内置不可卸载」并删死代码 |
| **P0-d** | 对齐两个 id 空间（E8/R3） | P0-b | 1.0 | 目录条目 id 能在 `BUNDLED_EXTENSIONS` 命中；无 `S1_SETTINGS extension_unknown` |
| **P1-a** | 补失败态重试 + 图标槽（E10/E11） | 无 | 0.5 | 失败态有重试；页头与空态显示 `PLUGIN_ICON`；`retryLoading` 有调用点 |
| **P1-b** | 补启用/禁用生命周期（E4） | P0-d | 2.0 | 持久化 `disabled_extensions`；禁用后主题/字体/图标/语言各自生效；重启保持 |
| **P1-c** | IDE 能力 API / MCP 消费者（E6/E7） | **架构师确认宿主边界** | 5.0–8.0 | GPUI 实现 `ideHost.control` 的 `open/poll/respond/catalog/validate/output` + MCP 设置页；13 工具可用 |
| **P2-a** | 补内置资源：主题 5→9、图标包 1→6 | 无 | 1.0–2.0 | 注册表条目与资源文件一致；`theme_entries_match_the_repository_files` 类测试通过 |
| **P2-b** | 迁移 `Plugins/` 插件包对应物（E9） | **产品决策**（H4） | 待定 | — |
| **P2-c** | UI 扩展 SDK（`ui-extension-host`/Worker 运行时） | **建议不做**（契约边界） | 0 | — |

**合计（不含 P2-b/c）**：约 **14.0–18.5 人日**（较 v1 的 11.5–15.5 上调，主因 N3/N4）。

### 排期约束（本维度相关，已核）

1. **每批必须同步矩阵**：`shared/platform-feature-matrix/features/<id>.json`，CI 门禁比较实际内容（`develop-lithe` 的强制前置门）。
2. **验收用启动日志 `S1_*`**，本维度已就位：`S1_EXTENSION sync_*|uninstalled|reinstalled`（`settings/src/extensions.rs:211,228,265,277,334,353,376,408`）、`S1_SETTINGS extension_unknown|extension_failed`（`store.rs:458,468`）、`S1_EXTENSIONS prepared`（`app/src/main.rs:662`）。
3. **验证后必须关闭 Lithe 进程并确认无残留**（根 `AGENTS.md`）。

### 依赖关系

```
P0-a（版本轴，需架构师）──> P0-b（移植目录通道）──> P0-d（id 对齐）──> P1-b（启停）
                        └─> P0-c（removable，需架构师，可并行）
P1-a（重试/图标）──> 独立，可并行
P1-c（能力 API）──> 独立，需先定宿主边界
P2-a（资源补齐）──> 独立
```

### 矩阵改动清单（本维度）

| 矩阵 id | 现状 | 本维度要求 |
|---|---|---|
| `ide-api-plugins` | mac/windows 均 implemented / pending | **需先定 gpui 列登记方式**，再据实回填「GPUI 缺失」 |
| `ide-mcp-connection` | 同上 | 同上 |
| `plugin-extension-management` | 同上 | GPUI 侧实为 **partial**（无安装/启停，装卸不可达） |
| `php-optional-plugin` | 同上 | GPUI 侧无 `Plugins/` 对应物 |

---

## 交接给主理人

### 需与其他职责核对

1. **给闻本立（架构师）— 三项裁决**（我不单方面改契约）：
   - `entrypoint.kind` 的合法取值能否扩到 `bundled`？（H3 / E5）
   - **宿主版本轴**（N3）如何对齐：改 `HOST_VERSION`、改 fixture，还是放宽校验？
   - v1 与 v2 目录 fixture 是并存还是替换？注意 `verify-shared-contracts.sh:137` 硬断言 `schemaVersion == 1`，**v2 过不了该门禁**。
2. **给常通关（构建发布）**：`Plugins/mac` 有签名与分发流程（`scripts/official-plugin-distribution.mjs`、`PluginPackageSignature.swift`），删旧端后 GPUI 是否需要等价机制？这决定 R5 的迁移成本。
3. **给陆井然（界面设计）**：扩展页缺「已装/启用/禁用」三态（E4），与 macOS `PluginManagementView.swift:114` 的 `Installed (N of M enabled)` 口径需对齐。
4. **给颜思齐（UI 设计）**：`PLUGIN_ICON` 已登记且在活动栏使用，扩展页却留空槽（E11）——视觉上补齐还是保持空槽口径。
5. **给娄码通（核心开发）**：`rust/lithe-gpui/target` 与工作区外 target 两份缓存并存，是否清理其一。

### 缺失资料

- `rust/lithe-gpui/assets/README.md:453,473` 引用的 `.agents/notes/implemented/feature/2026-09-29-plugin-catalog-v2-and-extensions-page.md` **不存在**——目录 schema v2 的设计决策溯源断链。
- `shared/contracts/plugin-catalog.md` 在当前基线**不存在**（仅存于 `302e5d04`）。
- `shared/fixtures/plugins/official-v2.json` 在当前基线**不存在**。

### 未覆盖

- 编辑器内核、渲染、LSP/DAP 语义、CI/CD 流水线（属其他成员）。
- 扩展页视觉规格与组件选型（属颜思齐 / 陆井然）。
- 未做界面实机验证；未跑功能矩阵与共享契约校验脚本（Windows 主机无 `/usr/bin/ruby`）。
- Windows 旧端 `extensions/` 1179 文件的迁移必要性未评估（需 H4 产品决策先行）。
