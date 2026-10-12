# 柯展之 — GPUI 扩展目录、扩展管理页、内置扩展打包与插件契约

**负责维度**：只覆盖插件与扩展层——扩展目录与数据模型、扩展管理页、内置扩展打包、插件清单/IDE 能力 API 契约。不涉及编辑器内核、渲染、LSP/DAP 语义、CI/CD。

**盘点口径**：GPUI 端 = `rust/lithe-gpui`；对照端 = `windows/tauri`（Windows 发行版）与 `macos/`（macOS 发行版）。基线提交 `3085f5d1`，分支 `feat/menu-bar-opaque`。

**配套证据文件**：
- `reports/gpui-extensions-evidence-catalog-contract.md`（A1-A5、B1-B6，11 条）
- `reports/gpui-extensions-evidence-page-bundled.md`（C1-C6、D1-D7、E，14 条）
- `reports/gpui-extensions-verdicts.md`（5 条判断的逐条裁决）

---

## 结论

### 一句话

GPUI 的扩展层不是「功能少」，而是**取数通道断在分支谱系上、生命周期只有装卸没有启停、内置扩展表与目录契约互不相通**。扩展页当前是一个**必然加载失败、且无法重试**的页面。

### 1. 缺口（用户可见）

| # | 缺口 | 性质 | GPUI 现状 | Windows | macOS | 证据 |
|---|---|---|---|---|---|---|
| G1 | 扩展目录完全取不到 | **接口缺失**（非未实现） | 扩展页恒显「加载失败」 | 正常 | 正常 | A1/A2 |
| G2 | 无安装扩展入口 | 完全未实现 | 无 | install/uninstall/enable/disable/update 五条 lifecycle | 下载安装 + 磁盘导入 | C1、`registry/extension-store-lifecycle.ts:153,302,357,443,530` |
| G3 | 无启用/禁用 | 完全未实现 | 无 | 有 | `Button(isEnabled ? "Disable" : "Enable")` | C2、`PluginManagementView.swift:217` |
| G4 | 卸载/重装入口不可达 | **判定恒假的接线缺陷** | `removable` 恒 `false` | 有 | 有 | C3 |
| G5 | IDE 能力 API / MCP 连接面全缺 | 宿主侧未接线 | 检索命中 0 | 404 行 `ide-capabilities.ts` + 设置页 | `IDECapabilities.swift` + `McpSettingsView.swift` | B1/B3 |
| G6 | 无 UI 扩展 SDK | **刻意的范围边界** | 无 | 12 文件（Worker 运行时/视图注入） | 无 | D7 |
| G7 | 内置资源少 | 规模差 | 5 主题 / 1 图标包 / 2 字体包 | 9 主题 / 6 图标包 | — | D2 |
| G8 | 无 `Plugins/` 插件包目录 | 完全未实现 | 无 | `Plugins/win/`（TS Worker 包） | `Plugins/mac/`（Swift 包 + 签名） | D6 |
| G9 | 扩展页失败后无法重试 | 接线缺陷 | 无重试按钮（`retryLoading` 键已备好但零调用） | — | — | C5 |
| G10 | 扩展页页头/空态图标留空 | 接线缺陷 | 两处空槽 | — | — | C4 |

### 2. 优化项（不是缺功能，但拖后腿）

| # | 优化项 | 现状证据 | 影响 |
|---|---|---|---|
| O1 | Core `plugins` 模块成孤儿（`pub mod plugins;` 存在、385 行、无生产调用链） | A3 | 死代码 + 误导 |
| O2 | schemaVersion 三方不一致（Core 只收 1；GPUI 测试样本写 2；真实清单 1） | A4 | 契约漂移，校验被绕过 |
| O3 | 两个扩展 id 空间零交集 | A5 | 目录条目点卸载必然静默失败 |
| O4 | `uninstall()`/`reinstall()` 是生产死代码 | C3/D4 | 有实现有测试但用户摸不到 |
| O5 | 内置扩展是硬编码常量表，与目录契约无代码关联 | D5 | 双份真源 |
| O6 | 两端插件包 manifest schema 不兼容 | B6 | 跨端插件包格式未统一 |
| O7 | `extensions_page.rs:263-267`/`:537-539` 注释称 `PLUGIN_ICON` 未登记，实际已登记且在活动栏使用 | C4 | 过时注释致功能缺失 |
| O8 | 矩阵无 gpui 列，登记方式未决 | B5 | 回填返工风险 |
| O9 | 引用的设计笔记不存在：`rust/lithe-gpui/assets/README.md:453,473` 指向 `.agents/notes/implemented/feature/2026-09-29-plugin-catalog-v2-and-extensions-page.md`，该文件不存在 | 未找到该文件 | 决策溯源断链 |

### 3. 未覆盖 / 未找到实现入口

- **UI 扩展 SDK**：未找到实现入口（检索关键词 `ui-extension` / `ui-extension-host` / `worker-runtime` / `extension-view-renderer` / `generative-ui`，在 `rust/lithe-gpui` 真实实现命中 0；8 处命中全为 crate 名 `lithe-gpui-extensions` 子串）。
- **MCP / IDE 能力 API**：未找到实现入口（检索关键词 `mcp` / `MCP` / `host-api` / `HostApi` / `ideHost` / `ide-capabilit`，真实实现命中 0；6 处命中全为 `hide_host` 无关局部变量）。
- **扩展市场 / 安装器 / 注册表**：未找到实现入口（`marketplace` / `installer` / `registry` 在 GPUI 侧无对应物；Windows 侧分别为 5 006 B / 10 728 B / 107 145 B）。
- **未执行**：未启动 Lithe 做界面实机验证；未跑 `scripts/verify-shared-contracts.sh`；未跑功能矩阵校验脚本。以上为静态检索与单测的边界。

---

## 证据与定位

### 本次实际执行过的验证（唯一一条运行时证据）

```
CARGO_TARGET_DIR=<workspace>/rust/lithe-gpui/target \
RUST_MIN_STACK=67108864 CARGO_PROFILE_DEV_DEBUG=0 \
cargo test -p lithe-gpui-extensions -j 2 -- --nocapture
```

结果 `test result: FAILED. 4 passed; 2 failed`，两个失败项：

```
core_bridge::tests::load_catalog_matches_the_bundled_fixture_host_version
  panicked at crates\extensions\src\core_bridge.rs:20:46:
  bundled catalog should load: "not_supported: Unsupported core command"
core_bridge::tests::bundled_catalog_covers_every_filter_category
  panicked at crates\extensions\src\core_bridge.rs:26:46:
  bundled catalog should load: "not_supported: Unsupported core command"
```

> 注：`CARGO_TARGET_DIR=D:/developmentProjects/rust/lithe-gpui-target` 因工作区沙盒 ACL 被拒（`failed to open .cargo-build-lock: 拒绝访问 (os error 5)`），改用工作区内 `rust/lithe-gpui/target` 成功。首次构建耗时 **11 分 50 秒**，与既有台账「约 15 分钟」同量级。

### 关键定位

| 结论 | 定位 |
|---|---|
| 调用点 | `rust/lithe-gpui/crates/extensions/src/core_bridge.rs:5-12`；`crates/workbench/src/extensions_page.rs:186` |
| Core 命令表无 `plugins.*` | `rust/lithe-core/src/protocol/command.rs:339-...`（148 条，命名空间清点无 plugins） |
| 未知命令失败路径 | `rust/lithe-core/src/runtime/dispatcher.rs:100-106`（`NotSupported` + `"Unsupported core command"`） |
| 根因（谱系分叉） | `git merge-base --is-ancestor 302e5d04 HEAD` → exit 1；`74a55eb0` → exit 1；二者均在 `LitheUi` / `chore/sync-macos-and-lithe-core-from-main` / `feat/lithe-gpui-components` |
| `removable` 恒假 | `extensions_page.rs:112`；Core `plugins/mod.rs:349-377` `valid_entrypoint`（`_ => false`） |
| 能力 API 宿主 | `rust/lithe-ide-host/src/catalog.rs:30-51`（13 工具）；`command.rs:342` `ideHost.control` → `dispatcher.rs:110-118` |
| 契约边界 | `shared/contracts/ide-api/v1.md:118-119`（不引入新市场/沙箱/UI extension SDK） |
| 矩阵 | `shared/platform-feature-matrix/features/{ide-api-plugins,ide-mcp-connection,plugin-extension-management,php-optional-plugin}.json`；`metadata.json` 仅两列 |

### 矩阵影响清单

| 矩阵 id | 现状 | 本维度要求的改动 |
|---|---|---|
| `ide-api-plugins` | 两列 implemented / pending | GPUI 侧无实现。**需先定 gpui 列登记方式**，再据实回填 |
| `ide-mcp-connection` | 两列 implemented / pending | 同上 |
| `plugin-extension-management` | 两列 implemented / pending | 同上；GPUI 侧实为 partial（无启停/安装） |
| `php-optional-plugin` | 两列 implemented / pending | GPUI 侧无 `Plugins/` 对应物 |

---

## 风险与条件

### 风险

| # | 风险 | 触发条件 | 影响 |
|---|---|---|---|
| R1 | 直接照抄台账结论会写错 | 台账称目录走 `plugins.listCatalog`，但该命令在基线不存在 | 排期会按「已接线只需优化」低估，实际是「接口缺失需移植」 |
| R2 | 契约决策越界 | 改 `entrypoint.kind` 合法取值、或统一两端 manifest schema | 触及 `rust/lithe-core` 清单语义，属架构师把关范围 |
| R3 | 两个 id 空间不对齐就修 G1 | 只把目录通道打通、不动 id | 目录能列出，但点卸载走 `store.rs:457` 判空 → 打 `S1_SETTINGS extension_unknown` 后**静默返回**，用户以为坏了 |
| R4 | 矩阵登记方式不定就动手 | `gpui-parity-plan-2026-10.md:344-348` 未决 | 阶段回填返工 |
| R5 | 构建环境 | 工作区 `target` 之外的 `CARGO_TARGET_DIR` 被沙盒 ACL 拒 | 排期里「构建必须绕开工作区」这条硬约束在本机**当前不成立**，需重核 |

### 假设

- **H1**：`plugins.listCatalog` 的移植是可行的（源实现完整存在于 `302e5d04`）。**未验证**——未尝试 cherry-pick 或手工移植。
- **H2**：G1 修复后扩展页能正常列出 5 条内置条目。**未验证**——未运行应用。
- **H3**：`removable` 的口径应以 Core 契约为准（即「内置扩展不可卸载」可能是有意设计），而非改 Core 去接受 `bundled`。**需架构师裁决**。

### 可能推翻结论的证据

- 若 `feat/lithe-gpui-components` 谱系即将合入当前分支，则 G1/G3 的「缺口」是**暂时的分支状态**，工作量应大幅下调。
- 若 `entrypoint.kind: "bundled"` 是已规划但未落地的取值，则 G4 是「半成品」而非「判定写错」。
- 若产品决策明确 GPUI 不做扩展安装市场（只做内置扩展管理），则 G2/G6 不构成缺口。

---

## 建议与验收

### 行动项（按优先级，含依赖与人日）

| 优先级 | 行动 | 依赖 | 人日 | 完成标准 |
|---|---|---|---|---|
| **P0-a** | 移植 `plugins.listCatalog` 到当前基线：Core `command.rs` + `dispatcher.rs` 分支 + 重建 v2 fixture + 更新 `verify-shared-contracts.sh` 断言 | 需架构师确认 schema v2 与 v1 关系（R2） | 1.5 | `cargo test -p lithe-gpui-extensions` 全绿（当前 2 红）；`cargo test -p lithe-core` 通过 |
| **P0-b** | 修 `removable` 判定（G4）：或改前端口径、或按契约产出正确 kind | 需架构师裁决 H3 | 0.5 | 扩展页上能出现卸载按钮，或明确记「内置扩展不可卸载」并删死代码 |
| **P0-c** | 对齐两个 id 空间（O3） | 依赖 P0-a | 1.0 | 目录条目 id 能在 `BUNDLED_EXTENSIONS` 命中；无 `S1_SETTINGS extension_unknown` |
| **P1-a** | 补失败态重试 + 图标槽（G9/G10/O7） | 无 | 0.5 | 失败态有重试按钮；页头与空态显示 `PLUGIN_ICON`；`retryLoading` 有调用点 |
| **P1-b** | 补启用/禁用生命周期（G3） | 依赖 P0-c | 2.0 | 设置持久化 `disabled_extensions`；禁用后主题/字体/图标/语言各自生效；重启保持 |
| **P1-c** | IDE 能力 API / MCP 连接面（G5） | 需架构师确认宿主边界 | 5.0-8.0 | GPUI 实现 `ideHost.control` 的 `open/poll/respond/catalog/validate/output` 循环 + MCP 设置页；`ide-api` 13 工具可用 |
| **P2-a** | 补内置资源（G7）：主题 5→9、图标包 1→6 | 无 | 1.0-2.0 | 注册表条目与资源文件一致；`theme_entries_match_the_repository_files` 类测试通过 |
| **P2-b** | 扩展市场 / 安装器（G2/G8） | 需产品决策 | 待定 | — |
| **P2-c** | UI 扩展 SDK（G6） | 建议**不做**（契约边界） | 0 | — |

**合计（不含 P2-b/c）**：约 **11.5-15.5 人日**。

### 排期约束（已实测，必须按它们算）

1. **构建一次约 12-15 分钟**（本次实测 11m50s），每天有效构建 3～4 次 ⇒ 功能必须成批写完再一起验证。按此，P0-a+P0-b+P0-c 应合并为一批，一次构建验证。
2. **每批必须同步功能矩阵**，否则 CI 门禁挂（`develop-lithe` 的强制前置门）。
3. **验收方式是启动日志的 `S1_*` 打点**，不是截图。本维度相关打点已就位：
   - `S1_EXTENSIONS prepared {created/kept/failed/removed}`（`crates/app/src/main.rs:663`）
   - `S1_EXTENSIONS_PAGE open=...`（`workbench/src/right_tool_window.rs:146-153`）
   - `S1_EXTENSION sync_*` / `uninstalled` / `reinstalled`（`settings/src/extensions.rs:211,228,265,277,334,353,376,408`）
   - `S1_SETTINGS extension_unknown` / `extension_failed`（`settings/src/store.rs:458,468`）
   - `S1_ICONS applied|missing|parse_failed`（`shared/src/icons/file_icon.rs:25,43,50`）
4. 验证后必须关闭 Lithe 进程并确认无残留。

### 依赖关系（谁阻塞谁）

```
P0-a（目录通道）──┬──> P0-c（id 对齐）──> P1-b（启停生命周期）
                  └──> P1-a（重试/图标，可并行）
P0-b（removable）──> 需架构师裁决，与 P0-a 可并行
P1-c（能力 API）──> 独立，但需先定宿主边界
P2-a（资源补齐）──> 独立
```

**构建链路未通前做不了的验收**：所有需要启动应用观察 `S1_*` 打点的验收（扩展页列出条目、卸载生效、启停保持、MCP 连接）。单测层面的验收（`cargo test -p lithe-gpui-extensions`）不受此限。

---

## 交接给主理人

### 需与其他职责核对

1. **给闻本立（架构师）**：`entrypoint.kind` 的合法取值能否扩到 `bundled`？两个 id 空间该对齐哪一侧？schema v2 与 v1 是并存还是替换？（R2/H3，我按硬约束不单方面改契约）
2. **给常通关（构建发布）**：本次实测发现 `CARGO_TARGET_DIR=D:/developmentProjects/rust/lithe-gpui-target` 被沙盒 ACL 拒（`os error 5`），与台账「构建必须绕开工作区」的硬约束**矛盾**。请复核该结论是否已过时。
3. **给娄码通（核心开发）**：`rust/lithe-gpui/target` 与 `lithe-gpui-target` 两份 target 缓存共 17.8 GB，是否应清理其一。
4. **给陆井然（界面设计）**：扩展页缺「已装/启用/禁用」三态展示（C2），与 macOS `PluginManagementView.swift:114` 的 `Installed (N of M enabled)` 口径需对齐。
5. **给颜思齐（UI 设计）**：`PLUGIN_ICON` 已登记且在活动栏使用，扩展页却留空槽（C4）——视觉上应补齐还是保持空槽口径。

### 缺失资料

- `rust/lithe-gpui/assets/README.md:453` 与 `:473` 引用的 `.agents/notes/implemented/feature/2026-09-29-plugin-catalog-v2-and-extensions-page.md` **不存在**（`Test-Path` = False；`.agents/notes/implemented/feature/` 下 23 个文件中无此篇）。目录 schema v2 的设计决策溯源断链。
- `shared/fixtures/plugins/official-v2.json` 在当前基线不存在（只存在于 `302e5d04` 谱系）。

### 未覆盖

- 编辑器内核、渲染、LSP/DAP 语义、CI/CD 流水线（属其他成员）。
- 扩展页的视觉规格与组件选型（属颜思齐/陆井然）。
- 未做界面实机验证与矩阵校验脚本执行。
