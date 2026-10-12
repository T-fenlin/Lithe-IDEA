# GPUI 扩展层取证 · 三：5 条候选判断的裁决

基线 `3085f5d1`，分支 `feat/menu-bar-opaque`。本文件回到立题时写下的**推翻条件**，逐条说明实际读数为何未构成推翻。

---

## 1. 目录桥接失效 — **支持**

**主张**：GPUI 扩展页的目录数据源 `plugins.listCatalog` 在当前基线不可用，`load_catalog()` 运行时必然失败。

**推翻条件**：`cargo test -p lithe-gpui-extensions` 的两个 `core_bridge` 测试通过，或 Core 命令表中能检索到 `plugins.listCatalog`。

**对照推翻条件读出了什么**：两个分支都没出现。

- 测试分支：**未通过**。实测命令
  `CARGO_TARGET_DIR=<workspace>/rust/lithe-gpui/target RUST_MIN_STACK=67108864 CARGO_PROFILE_DEV_DEBUG=0 cargo test -p lithe-gpui-extensions -j 2 -- --nocapture`
  输出 `test result: FAILED. 4 passed; 2 failed`，两个失败项是
  `core_bridge::tests::load_catalog_matches_the_bundled_fixture_host_version` 与
  `core_bridge::tests::bundled_catalog_covers_every_filter_category`，均报
  `bundled catalog should load: "not_supported: Unsupported core command"`
  （`crates/extensions/src/core_bridge.rs:20:46` 与 `:26:46`）。
- 检索分支：**未命中**。`rust/lithe-core/src/protocol/command.rs:339` 起的 `CoreCommand::parse`
  共 148 条 `=> Some(Self::` 条目，按命名空间清点为
  `agent 5 / core 1 / debug 18 / diagnostics 2 / document 2 / editor 2 / execution 1 / file 2 / git 44 / github 3 / history 6 / ideHost 1 / java 11 / lsp 18 / markdown 1 / maven 6 / mybatis 1 / runConfig 8 / spring 1 / workspace 5`，
  **无任何 `plugins.*`**。未知命令的失败路径在 `rust/lithe-core/src/runtime/dispatcher.rs:100-106`
  （`ErrorCode::NotSupported` + `"Unsupported core command"`），与测试报错原文逐字一致。

**证据定位**：`reports/gpui-extensions-evidence-catalog-contract.md` 的 A1、A2、A3；
命令原文与测试输出原文照录于 A1。

**裁决**：**支持**。

---

## 2. 能力API缺失 — **支持**

**主张**：GPUI 侧完全没有 IDE 能力 API / MCP 连接面（`ideHost.control` 与 ide-api v1 的 13 个工具无任何宿主侧消费者）。

**推翻条件**：在 `rust/lithe-gpui` 检索 `mcp` / `host-api` / `ideHost` / `ide-capabilit` 命中非零**且为真实实现**。

**对照推翻条件读出了什么**：命中非零，但**逐处查看后全部不是真实实现**，故推翻条件未满足。

- 检索 `(?i)mcp|host[-_]?api|ideHost|ide_host|ide-capabilit|ide_capabilit` 在 `rust/lithe-gpui` 命中 **6** 处：
  `explorer/src/explorer_view.rs:82,141,156,157,488` 与 `workbench/src/workspace.rs:666`。
  逐处查看，全部是名为 `hide_host` 的无关局部变量（面板隐藏回调），与 IDE 能力 API 无任何关系。
- `rust/lithe-gpui/crates/*/Cargo.toml` 无任何 `ide-host` 直接依赖。
  `lithe-ide-host` 只经 `lithe-core` 传递出现在 `rust/lithe-gpui/Cargo.lock:4147`。

**注**：主张在裁决过程中由「14 个工具」更正为「**13** 个工具」——`rust/lithe-ide-host/src/catalog.rs:30-51`
的 `tools()` 里 `("lithe_*"` 条目逐名点算为 13，与 `shared/contracts/ide-api/v1.md:51-65` 能力表行数一致。
这是主张文本的**数字更正**，不影响结论方向（缺的是消费者，不是工具数量）。

**证据定位**：`reports/gpui-extensions-evidence-catalog-contract.md` 的 B1、B2、B3、B4。

**裁决**：**支持**。

---

## 3. 管理页动作缺失 — **支持**

**主张**：GPUI 扩展管理页缺少安装/启用/禁用生命周期动作，只有「卸载/重装」和「使用图标集」。

**推翻条件**：`extensions_page.rs` 中存在 enable/disable/install 的实际动作与状态持久化。

**对照推翻条件读出了什么**：未出现。

- 逐动作枚举 `rust/lithe-gpui/crates/workbench/src/extensions_page.rs` 的全部交互，只有 5 项：
  搜索（`:171-180`，仅改内存 `query`）、分类筛选（`:328-339`，仅改内存 `category`）、
  行内展开（`:524` → `:232`，仅改内存 `expanded`）、使用图标集（`:424-427` → `:214` → `store.set_icons()`）、
  卸载/重装（`:446-451` → `:222` → `store.set_extension_uninstalled()`）。
  **只有后两项持久化**。
- 全文件检索 `enable|disable|install|激活|启用|禁用`：命中项为
  `:422 .disabled(is_current)`（按钮禁用态）、`:405-435` 的 `uninstalled` 局部变量、
  以及 `uninstalled` 里的 `install` 子串。**无任何启用/禁用/安装动作**。
- 状态维度同样缺失：`enum CatalogStatus`（`:37-42`）只有 `Loading`/`Loaded`/`Failed`，
  两端对照 macOS 有 `Button(isEnabled ? "Disable" : "Enable")`（`macos/Sources/Lithe/Views/App/PluginManagementView.swift:217`）
  与 `"Installed (\(enabledPluginCount) of \(installedPlugins.count) enabled)"`（`:114`）。

**证据定位**：`reports/gpui-extensions-evidence-page-bundled.md` 的 C1、C2。

**裁决**：**支持**。

---

## 4. 卸载入口不可达 — **支持**

**主张**：内置扩展的卸载/重装入口在 UI 上永不可达：`removable` 判定 `entrypoint_kind=="bundled"`，
而 Core 契约与全部 fixture 只产出 `builtIn`/`nativeBundle`。

**推翻条件**：检索到任一产出 `"kind":"bundled"` 的插件清单，或 Core 侧接受该值。

**对照推翻条件读出了什么**：两个分支都没出现，且从**两个方向**各自独立确认。

- 接受侧（Core 契约）：`rust/lithe-core/src/plugins/mod.rs:349-377` 的 `valid_entrypoint()` 是
  `match entrypoint.kind.as_str()`，只有 `"builtIn"` 与 `"nativeBundle"` 两个分支，`_ => false`。
- 产出侧（全部清单）：全仓检索 `"kind"\s*:\s*"bundled"`，只命中
  `shared/fixtures/settings/workbench-background-v1.json`（工作台背景图的无关字段）与
  `macos/Tests/LitheTests/AppSettingsTests.swift:220`（该字段的 macOS 测试）。
  原 v2 目录 fixture 的 5 个条目**全部**是 `"kind": "builtIn"`（`git show 302e5d04:shared/fixtures/plugins/official-v2.json`，逐条已核）。
- 推论链：`removable` 恒为 `false` ⇒ `extensions_page.rs:431 .when(card.removable, ...)` 永不渲染
  ⇒ 按钮永不出现。进而 `settings/src/store.rs:451 set_extension_uninstalled()` 与
  `settings/src/extensions.rs` 的 `uninstall()`(:361) / `reinstall()`(:382) 成为**生产死代码**（仅单元测试直接调用）。

**证据定位**：`reports/gpui-extensions-evidence-page-bundled.md` 的 C3、D4。

**裁决**：**支持**。

---

## 5. 无UI扩展SDK — **支持**

**主张**：GPUI 侧没有 UI 扩展 SDK（Windows 有 Worker 运行时 / 生成式 UI 渲染器 / 视图注入），
扩展只能贡献主题、字体、图标、语言与调试适配器。

**推翻条件**：在 `rust/lithe-gpui` 检索到 UI 扩展宿主、视图注入或 Worker 运行时实现。

**对照推翻条件读出了什么**：命中 8 处，但**逐处查看全部是 crate 名子串**，无一是实现。

- 检索 `(?i)ui-extension|ui_extension|worker-runtime|worker_runtime|extension-view-renderer|extensionView|generative-ui|generative_ui`，
  在 `rust/lithe-gpui` 命中 8 处：`Cargo.lock:4028,4134`（包名 `lithe-gpui-extensions`）、
  `workbench/src/extensions_page.rs:62,186,640,645`（`lithe_gpui_extensions::` 路径前缀）、
  `workbench/Cargo.toml:17`、`extensions/Cargo.toml:2`（依赖声明）。
  **全部是 crate 名 `lithe-gpui-extensions` 的子串**，与 UI 扩展 SDK 无关。
- 对照端有完整实现：`windows/tauri/src/extensions/ui/services/` 12 个文件，含
  `ui-extension-host.ts`（11 038 B）、`ui-extension-worker-runtime.ts`（8 435 B）、
  `ui-extension-api.ts`（4 093 B）、`generated-ui-extension-installer.ts`（18 625 B），
  以及 `components/` 下的 `extension-view-renderer.tsx` / `generative-ui-renderer.tsx` / `external-extension-view.tsx`。
- **性质说明**：这是**刻意的范围边界**，不是遗漏。`shared/contracts/ide-api/v1.md:118-119` 明示
  「This adds a capability surface; it does not introduce a new plugin marketplace, sandbox or UI extension SDK.」
  且 macOS 端也没有对应物。

**证据定位**：`reports/gpui-extensions-evidence-page-bundled.md` 的 D7、E。

**裁决**：**支持**。

---

## 汇总

| # | 判断短名 | 裁决 | 推翻条件是否出现 | 关键读数 |
|---|---|---|---|---|
| 1 | 目录桥接失效 | **支持** | 否（两个分支都未出现） | 4 passed / 2 failed；148 条命令表无 `plugins.*` |
| 2 | 能力API缺失 | **支持** | 否（6 处命中全为无关局部变量） | 检索命中 0 处真实实现 |
| 3 | 管理页动作缺失 | **支持** | 否 | 5 个动作中仅 2 个持久化；无启用/禁用 |
| 4 | 卸载入口不可达 | **支持** | 否（双向确认） | `valid_entrypoint` 只有 2 个分支；全仓无 `"bundled"` 清单 |
| 5 | 无UI扩展SDK | **支持** | 否（8 处命中全为 crate 名子串） | 对照端 12 文件 vs GPUI 0 |

**5 条全部支持，0 条推翻，0 条不确定。**

### 本次实际执行过的验证（其余为静态检索）

| 验证 | 命令 | 结果 |
|---|---|---|
| 扩展 crate 单元测试 | `cargo test -p lithe-gpui-extensions -j 2 -- --nocapture`（带 `CARGO_TARGET_DIR` / `RUST_MIN_STACK` / `CARGO_PROFILE_DEV_DEBUG=0`） | `4 passed; 2 failed`，失败项报 `not_supported: Unsupported core command` |

**未执行**：未启动 Lithe 应用做界面实机验证（扩展页空态/失败态未截图确认）；
未跑 `scripts/verify-shared-contracts.sh`；未跑功能矩阵校验脚本。这些是**静态与单测证据的边界**。
