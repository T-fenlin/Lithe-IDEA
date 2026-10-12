# GPUI 扩展层取证 · 二：扩展管理页与内置扩展打包

基线 `3085f5d1`，分支 `feat/menu-bar-opaque`。承接 `reports/gpui-extensions-evidence-catalog-contract.md`。

---

## C. 扩展管理页

### C1【事实】页面现有动作只有两个：切图标集、卸载/重装

`rust/lithe-gpui/crates/workbench/src/extensions_page.rs` 全部交互动作：

| 动作 | 位置 | 落点 |
|---|---|---|
| 搜索 | `:171-180` 订阅 `InputEvent::Changed` | 仅改 `self.query`（内存） |
| 分类筛选 | `:328-339` `ToggleButton::on_toggle` | 仅改 `self.category`（内存） |
| 行内展开/收起 | `:524` `on_click` → `:232 toggle()` | 仅改 `self.expanded`（内存） |
| 使用图标集 | `:424-427` → `:214 use_icons()` → `store.set_icons()` | 持久化 |
| 卸载/重装 | `:446-451` → `:222 set_uninstalled()` → `store.set_extension_uninstalled()` | 持久化 |

全文件检索 `enable|disable|install|激活|启用|禁用`：**没有任何启用/禁用/安装动作**。命中的只是
`:422 .disabled(is_current)`（按钮禁用态）、`:405-435` 的 `uninstalled` 局部变量与 `install` 子串所在的 `uninstalled`。

⇒ 证实判断「管理页动作缺失」。

### C2【事实】四个状态覆盖：有 3 个，缺「已装/启用/禁用」

`enum CatalogStatus`（`:37-42`）只有 `Loading` / `Loaded(Vec<Card>)` / `Failed(SharedString)`，
在 `:529-535` 的 `empty_state` 里分别映射：

| 状态 | 文案 | 行号 |
|---|---|---|
| 加载中 | `lithe.extensions.loadingExtension` | `:530` |
| 失败 | `lithe.extensions.loadFailed` + 错误原文 | `:531-533` |
| 已加载但空 | `lithe.extensions.noneFound` | `:534` |
| 未安装（徽标） | `lithe.extensions.uninstalled` | `:401` |
| **启用 / 禁用** | **无** | — |

`:386-403` 的两个徽标（`builtIn` / `uninstalled`）是**状态展示**，不是可切换状态。
两端对照：macOS `PluginManagementView.swift:114` 有 `"Installed (\(enabledPluginCount) of \(installedPlugins.count) enabled)"`，
`:217` 有 `Button(isEnabled ? "Disable" : "Enable")` —— 即有启用/禁用；GPUI 没有。

### C3【事实】卸载/重装入口在 UI 上永不可达

`extensions_page.rs:111-112`：

```rust
built_in: entry.entrypoint_kind == "builtIn",
removable: entry.entrypoint_kind == "bundled",
```

而 `entrypoint_kind` 直接来自目录响应的 `entrypoint.kind`（`extensions/src/model.rs:129`）。该字段的合法取值由 Core 契约限定：

- `rust/lithe-core/src/plugins/mod.rs:349-377` `valid_entrypoint()`：`match entrypoint.kind.as_str()` 只有 `"builtIn"` 与 `"nativeBundle"` 两个分支，`_ => false`。
- 原 v2 目录 fixture 的 5 个条目**全部** `"kind": "builtIn"`（`git show 302e5d04:shared/fixtures/plugins/official-v2.json`，逐条已核）。
- 全仓检索 `"kind"\s*:\s*"bundled"`：只命中 `shared/fixtures/settings/workbench-background-v1.json`（工作台背景图的无关字段）与其 macOS 测试 `macos/Tests/LitheTests/AppSettingsTests.swift:220`。**没有任何插件清单产出该值。**

⇒ `removable` 恒为 `false` ⇒ `:431 .when(card.removable, ...)` 分支永不渲染 ⇒ 「卸载/重装」按钮永不出现。
`store.set_extension_uninstalled()`（`settings/src/store.rs:451`）与 `settings/src/extensions.rs` 的
`uninstall()`(:361) / `reinstall()`(:382) 全部是**死代码**（仅被单元测试直接调用）。

⇒ 证实判断「卸载入口不可达」。

### C4【事实】图标槽留空的注释已过时，`PLUGIN_ICON` 实际已登记

`extensions_page.rs:263-267`（页头）与 `:537-539`（空态）两处都留空槽，注释写：

> 本仓图标集里**有** `expui/nodes/plugin.svg` 这张图，但 `shared::icons::idea` 还没为它登记常量 ⇒ 本轮按口径**留空**（同尺寸空槽）

实际已登记：

- `rust/lithe-gpui/crates/shared/src/icons/idea.rs:1032-1036` `pub const PLUGIN_ICON`，`light: "icons/idea/expui/nodes/plugin.svg"`，`dark: .../plugin_dark.svg`。
- `rust/lithe-gpui/assets/icons/idea/expui/nodes/plugin.svg` 文件存在（`Test-Path` = True）。
- 且**已在生产使用**：`workbench/src/workspace.rs:3254` 活动栏 `ActivityItem::idea(&idea::PLUGIN_ICON, tr("lithe.extensions.title"))`。

⇒ 注释所述前提不成立，扩展页留空是**遗留缺陷**而非口径要求。修它只需改两处 `.child(...)`。

### C5【事实】页面的数据源是异步加载，但没有任何重试入口

`extensions_page.rs:185-201` 的 `cx.spawn` 只跑一次，结果写进 `self.status` 后 `detach()`。
`:531-533` 失败态只显示错误原文，**无重试按钮**。而 i18n 里已备好 `lithe.extensions.retryLoading`
（`rust/lithe-gpui/crates/shared/locales/lithe.zh-CN.yml`，`extensions.*` 键表内）——键存在但无调用点。

结合 A1（目录必然加载失败），用户看到的是**一个永远失败、且无法重试**的页面。

### C6【事实】i18n 键完备，不是缺口

逐键核对 `extensions_page.rs` 使用的 16 个键（`lithe.extensions.icons` / `.category.fonts` / `.category.debug` /
`.category.lsp` / `.builtIn` / `.uninstalled` / `.reinstall` / `.uninstall` / `.loadingExtension` / `.loadFailed` /
`.noneFound` / `.byPublisher` / `.contributions` / `.languageUninstallHint` / `.fontRestartHint` / `.contributionLine`）
在 `crates/shared/locales/lithe.zh-CN.yml` 中**全部存在**。且 `extensions_page.rs:631-636` 有测试
`every_filter_label_resolves` 钉住筛选标签。

⇒ 文案层无缺口；缺口在动作与数据层。

---

## D. 内置扩展打包

### D1【事实】`BUNDLED_EXTENSIONS` 8 条，按类型分布

`rust/lithe-gpui/crates/settings/src/extensions.rs:22-122`，`BundledExtension {` 计数 = 8：

| 类型 | 条数 | id |
|---|---|---|
| `Theme` | 5 | `lithe.gruvboxTheme`、`lithe.litheTheme`、`lithe.nordTheme`、`lithe.oneTheme`、`lithe.vscodeTheme` |
| `Font` | 2 | `lithe.ibmPlexMonoFont`、`lithe.sourceCodeProFont` |
| `Languages` | 1 | `lithe.projectLanguages`（7 种语言：yaml/toml/sql/markdown/bash/kotlin/xml，`:67`） |
| `Icons` | **0** | — |

`Icons` 为 0 是**有意**的，测试 `icons_packages_have_a_real_manifest`（`:500-532`）用
`assert_eq!(checked, 0, "当前没有可卸载的捆绑图标集 …")` 钉死，注释说明图标真源已改为
macOS 图标文件夹副本、只保留内置 `idea` 包。

### D2【事实】内置资源与 Windows 端的规模差

| 资源 | GPUI | Windows 端 | 证据 |
|---|---|---|---|
| 主题 JSON | **5** | **9**（ayu/catppuccin/dracula/github/nord/one/solarized/tokyo-night/vitesse） | `rust/lithe-gpui/extensions/themes/` vs `windows/tauri/src/extensions/bundled/themes/` |
| 图标主题包 | **1**（`idea`） | **6**（idea/lithe/material/minimal/pierre/symbols） | `rust/lithe-gpui/assets/icons/` vs `windows/tauri/src/extensions/bundled/icon-themes/` |
| 字体 | 2 包（IBM Plex Mono、Source Code Pro），走 `extensions/fonts/` 落盘 | — | `settings/src/extensions.rs:80-121` |

补充事实：`rust/lithe-gpui/assets/fonts/` 下有 **JetBrains Mono** 4 字重 + OFL.txt，
但它在 `crates/app/src/assets.rs:14` 被 `#[exclude = "fonts/**"]` 排除、不进嵌入资源；
BUNDLED_EXTENSIONS 里的字体是 IBM Plex Mono 与 Source Code Pro。
而原 v2 目录 fixture 声明的字体是 **JetBrains Mono 4 字重**（`fonts/JetBrainsMono-*.ttf`）。
⇒ 目录声明、嵌入资产、可卸载字体包三者的字体**互不相同**。

### D3【事实】`mirror`（装载面）只有主题有，字体/图标/语言没有

`extensions.rs:147-160`：

- `store_dir`：Theme→`theme_extensions_dir()`，Font→`font_extensions_dir()`，Icons/Languages→`None`
- `mirror_dir`：Theme→`themes_dir()`，Font/Icons/Languages→**`None`**

`sync_bundled_extensions`（`:203-217`）对 Icons/Languages 走 `store_dir` 返回 `None` 的分支，
打 `S1_EXTENSION sync_skipped reason=no_dirs kind=...` 并 `continue`。⇒ **语言包与图标包不落盘**，
它们「随宿主编译」（测试 `registry_is_well_formed` `:440-447` 断言其 `files` 必须为空）。
这是一致的设计，不是缺陷。

### D4【事实】内置扩展的卸载会删「管理面」与「装载面」两份文件

`remove_files`（`:300-326`）对每个文件删 `store_dir` 与 `mirror_dir` 两处，
并尝试删空父目录。测试 `sync_installs_uninstalls_and_restores_both_surfaces`（`:596-630`）
断言 `report.removed.len() == 2`（两处各一）。`S1_EXTENSION uninstalled` 打点在 `:375-378`。

⇒ 这条链路本身**有实现且有测试**，只是 UI 入口不可达（C3）。

### D5【事实】内置扩展打包**没有**走目录契约

`BUNDLED_EXTENSIONS` 是 Rust 常量表（`&'static [BundledExtension]`），字段只有
`id/kind/files/icons_package/languages`；而目录契约的 `PluginPackageManifest`
（`rust/lithe-core/src/plugins/mod.rs:112-133`）有 `displayName/version/apiVersion/hostCompatibility/vendor/entrypoint/moduleIDs/languageSupports`。

两者**没有任何代码关联**：`settings` crate 不依赖 `lithe-gpui-extensions`，也不调 `core_json`。
（`rust/lithe-gpui/crates/settings/Cargo.toml` 无 `lithe-gpui-extensions`；`extensions.rs` 全文无 `core_json`。）

⇒ 「内置扩展」在 GPUI 侧是**硬编码表**，不是目录的投影。这正是 A5 两个 id 空间零交集的成因。

### D6【事实】两端内置扩展的组织方式不同

- macOS：`Plugins/mac/Official/{GoSupport,PhpSupport}/` 各自是**真实插件包**（`plugin.json` + `Info.plist` + Swift 模块 + 测试），有独立签名与分发流程（矩阵 `php-optional-plugin` macos.evidence 含 `PluginPackageSignature.swift`、`official-plugin-distribution.mjs`）。
- Windows：`Plugins/win/Official/{PhpSupport,V0Support}/`（TS Worker 包）+ `windows/tauri/src/extensions/` 1179 文件子系统。
- GPUI：**无 `Plugins/` 对应物**，只有 `settings/src/extensions.rs` 常量表 + `rust/lithe-gpui/extensions/` 资源目录（5 主题 + 6 字体文件）。

### D7【事实·实测】GPUI 侧完全没有 UI 扩展 SDK

检索 `(?i)ui-extension|ui_extension|worker-runtime|worker_runtime|extension-view-renderer|extensionView|generative-ui|generative_ui`：

在 `rust/lithe-gpui` 命中 8 处，**全部**是 crate 名 `lithe-gpui-extensions` 的子串
（`Cargo.lock:4028,4134`、`workbench/src/extensions_page.rs:62,186,640,645`、`workbench/Cargo.toml:17`、`extensions/Cargo.toml:2`）。

⇒ **未找到实现入口（检索关键词 `ui-extension` / `worker-runtime` / `extension-view-renderer` / `generative-ui`，真实实现命中 0）。**

对照端有完整实现（`windows/tauri/src/extensions/ui/services/`，12 个文件）：

| Windows 文件 | 作用 |
|---|---|
| `ui-extension-host.ts`（11 038 B） | UI 扩展宿主 |
| `ui-extension-worker-runtime.ts`（8 435 B） | Worker 运行时 |
| `ui-extension-api.ts`（4 093 B） | 注入给扩展的 UI API |
| `generated-ui-extension-installer.ts`（18 625 B） | 生成式 UI 扩展安装 |
| `extension-host-services.ts`（5 153 B）、`extension-permissions.ts` | 宿主服务与权限 |
| `components/extension-view-renderer.tsx`、`generative-ui-renderer.tsx`、`external-extension-view.tsx` | 视图渲染 |

⇒ 证实判断「无UI扩展SDK」。**性质**：这是**刻意的范围边界**，不是遗漏——`shared/contracts/ide-api/v1.md:118-119`
明示「does not introduce a new plugin marketplace, sandbox or UI extension SDK」，且 macOS 端也没有对应物。

---

## E. 与 Windows 端的总体规模差

| 维度 | GPUI | Windows | 倍数 |
|---|---|---|---|
| 扩展子系统代码 | `crates/extensions/` **11 117 B**（4 文件）+ `settings/src/extensions.rs` 25 291 B + `extensions_page.rs` 26 200 B ≈ **62 KB** | `windows/tauri/src/extensions/` **1179 文件**，其中 `bundled/` 6 044 712 B、`registry/` 107 145 B、`ui/` 154 100 B、`tooling/` 76 842 B、`themes/` 76 535 B | ~100×（含资源） |
| 生命周期动作 | 卸载/重装（不可达）、切图标集 | install / uninstall / enable / disable / update 五条独立 lifecycle（`registry/extension-store-lifecycle.ts:153,302,357,443,530`） | — |
| 注册表 | 无 | `registry/extension-registry.ts`（13 252 B）、`extension-store.ts`（21 567 B）、`extension-store-runtime.ts`（20 159 B） | — |
| 市场 | 无 | `marketplace/`（5 006 B） | — |
| 安装器 | 无 | `installer/`（10 728 B）、`packages/local-extension-package.ts`（6 426 B） | — |
| 语言/查看器 | 无 | `languages/`（21 619 B）、`viewers/`（12 902 B） | — |

---

## 本文件结论索引

| # | 结论 | 性质 | 证据 |
|---|---|---|---|
| C1 | 页面只有「切图标集」与「卸载/重装」两个持久化动作 | 事实·实测 | 逐动作表 + 检索 |
| C2 | 状态覆盖 3/5，无启用/禁用 | 事实 | `:37-42`、`:529-535` |
| C3 | 卸载/重装入口永不可达（`removable` 恒 false） | 事实 | `:112` + Core `valid_entrypoint` |
| C4 | 图标槽留空注释已过时，`PLUGIN_ICON` 已登记且在用 | 事实 | `idea.rs:1032` + `workspace.rs:3254` |
| C5 | 无重试入口，但 `retryLoading` 键已备好 | 事实 | `:185-201`、`:531-533` |
| C6 | i18n 键完备 | 事实·实测 | 16 键逐个核对 |
| D1 | `BUNDLED_EXTENSIONS` 8 条（5 主题/2 字体/1 语言/0 图标） | 事实 | `:22-122` |
| D2 | 资源规模 5 主题 vs 9、1 图标包 vs 6；字体三方不一致 | 事实 | 目录计数 |
| D3 | mirror 只有主题有，语言/图标不落盘（有意设计） | 事实 | `:147-160` |
| D4 | 卸载链路有实现有测试，只是 UI 不可达 | 事实 | `:300-326`、`:596-630` |
| D5 | 内置扩展是硬编码表，未走目录契约 | 事实 | `Cargo.toml` 无依赖 |
| D6 | 两端有 `Plugins/` 真实包，GPUI 无 | 事实 | 三端目录 |
| D7 | 无 UI 扩展 SDK（刻意的范围边界） | 事实·实测 | 检索命中 0 + 契约 |
| E | 扩展子系统规模差约 100× | 事实 | 文件计数与字节 |
