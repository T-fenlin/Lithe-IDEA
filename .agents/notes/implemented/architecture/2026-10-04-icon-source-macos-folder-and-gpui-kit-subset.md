# Agent 笔记：图标真源改为 macOS 图标文件夹的副本，并去掉 gpui-kit 的整份图标目录

状态：已实现

## 先说结论

宿主的图标来源从"从已删除的 Windows 前端搬来的副本 + gpui-kit 全量 Lucide"改成两条明确的来源：

1. **主来源 = `macos/Resources/IDEAIcons/**` 的副本**（界面真源），逐字节复制进
   `rust/lithe-gpui/assets/icons/idea/`，gpui 引用的就是这份副本；macOS 侧原件不动。
   与旧包取**并集**：macOS 侧没有、但界面/清单仍在用的 122 个旧图标留在 `legacy/` 下
   （`assets/icons/idea/SOURCES.md` 有台账），其余 57 个删掉；旧包本身的路径口径（`icons/files/**`）废止。
2. **gpui-kit 的整份图标目录不再进二进制**：`AllAssets`（1830 个 Lucide 字形 / 731 KiB）从
   `LitheAssets` 里移除，改为 `gpui_kit::assets::icon_assets!(GpuiKitFallbackIcons, [...])`
   只登记**实际会被请求的 106 个名字**（宿主代码 73 + gpui-kit 组件内部 51，合计约 46 KiB）。

两条都要记住：**改图标改的是 `macos/Resources/IDEAIcons`，改完跑同步工具**；
**新增 `IconName::X` 之后必须登记**，否则界面上**静默画空白、没有任何编译错误**。

## 问题

改造前宿主有两套来源，且都与"macOS 是界面规格真源"这条仓库规则不一致：

- `assets/icons/idea/**`（253 个 SVG + `extension.json`）是从**已删除的 Windows 前端**
  （`windows/tauri/src/ui/icons/**`）搬来的 IntelliJ expui 副本；macOS 侧另有一份
  `macos/Resources/IDEAIcons`（176 个 SVG）。两份按内容比对只有 **58 个文件相同**，
  macOS 独有 118 个、Windows 那份独有 196 个（`ui/` 那批通用字形与部分文件类型）。
- 界面图标（工具栏/按钮/面板/对话框）走 gpui-kit 的 `IconName` → `AllAssets`（1830 个），
  宿主自己引用 73 个名字、gpui-kit 组件内部还引用 51 个。
- 另有 `assets/icons/{lithe,symbols,material,pierre}` 四个文件图标主题文件夹（都是从 Windows 前端
  搬来的副本，合计约 4.97 MiB），其中 pierre 还被登记成"可卸载/可切换"的捆绑扩展。

## 决策

### 一、图标真源 = macOS 图标文件夹的副本 + 并集补充，全部由工具生成

`rust/lithe-gpui/tools/sync-macos-idea-icons.mjs`（`--dry-run` / `--apply` / `--check`）是唯一入口：

- 把 `macos/Resources/IDEAIcons/**`（含 Apache-2.0 归属的两个附件）逐字节复制进
  `assets/icons/idea/`，保留 macOS 原目录结构（`fileTypes/ expui/ general/ nodes/ vcs/ debugger/
  terminal/ toolwindows/ testState/ gutter/ maven/ actions/`）。
- 旧包的映射规则：把 legacy 路径按"归一化主干"匹配 macOS 文件；主题极性按 IntelliJ 约定换算
  （legacy `X.svg`＝浅色 → macOS `X_light.svg`；legacy `X_dark.svg` → macOS `X.svg`）。
  语义不同名时用 `tools/macos-idea-icon-map.json` 的 `overrides` 逐条改写。
- **并集**：只保留"仍在用"的旧图标 —— `extension.json` 的 `fileExtensions`/`folders`/默认值点名的定义，
  或宿主代码 `idea::<常量>` 引用到的常量所指向的文件；其余删除。
- 同时删除 `assets/icons/` 下 `lithe/ symbols/ material/ pierre/` 四个图标主题文件夹。
- 重写 `extension.json`（只留被引用的定义、路径改指新布局、id 与 `fileExtensions` 不变）、
  重新生成 `crates/shared/src/icons/idea.rs`（**常量名保持不变**，只换路径）、写出 `SOURCES.md` 台账。

### 二、gpui-kit 只登记"会被请求的名字"

- `crates/app/src/assets.rs`：`AllAssets` 换成 `gpui_kit::assets::icon_assets!(GpuiKitFallbackIcons, [...])`；
  `LitheAssets::load` 的顺序变成"内嵌资源 → 回落子集 → None"。
- 名单由 `rust/lithe-gpui/tools/sync-gpui-kit-icons.mjs --apply` 生成：
  扫 `crates/**` 的 `IconName::X` ∪ 工具里登记的"gpui-kit 组件内部用到的 51 个名字"。
  组件内部那份清单**必须随 gpui-kit 升级重新实测**（工具注释里给了 `rg` 命令）。
- 两道判据：Rust 侧 `gpui_kit_fallback_icons_are_the_used_subset_and_loadable` 逐个 `load` 出非空字节并钉住数量；
  工具 `--check` 保证"宿主用到的名字都登记了、且没有多余登记"。

### 正确做法

- 页面要新图标：先看 `macos/Resources/IDEAIcons` 里有没有，有就改 macOS 那份 + 跑同步工具；
  文件类型图标则同时看 `extension.json` 的 `fileExtensions` 映射。
- 需要 gpui-kit 的某个字形：正常写 `IconName::X`，然后跑 `sync-gpui-kit-icons.mjs --apply`。

### 不要这样做

- 不要手工改 `assets/icons/idea/**` 或 `idea.rs`：它们都是生成物，下次 `--apply` 会覆盖。
- 不要为了"少改一行"而保留 `AllAssets`：那就是把 1830 个用不到的字形连带打进二进制（实测 731 KiB）。
- 不要把 gpui-kit 的 `IconName` 换成 `idea::` 彩色图标来"统一风格"：gpui-kit 的字形是**单色 + 跟随
  `text_color` 着色**的，而 `idea::*` 是带固定配色的浅/深两套（`_dark` / `_light`），换过去会让
  主题不再影响这些图标。两者各管一摊：文件/节点/工具窗图标用 `idea::*`，界面字形用 `IconName`。

## 考虑过的备选方案

### 备选方案一：只换美术、不动目录与清单

把 macOS 美术覆盖到旧路径（`icons/files/java.svg` 等）上，`idea.rs`/清单零改动。
没采用：用户要的是"把 macOS 那份复制进 gpui、gpui 引用复制的这份"，
保留旧口径会让 `assets/icons/idea/` 看起来仍是 Windows 那套，来源也说不清。

### 备选方案二：真搬走 macOS 侧原件，只留一份

能少一份副本，但 `macos/Sources/Lithe/Theme/LitheIcons.swift` 按 `IDEAIcons/<目录>` 从 bundle 取图，
两个 Swift 测试与 `shared/platform-feature-matrix.json` 也按路径引用它 —— 真搬走会断掉这些引用，
而 macOS 仍是界面规格真源。所以选择"复制一份、macOS 保留"。

### 备选方案三：严格只用 macOS 侧，缺的走 Lucide 子集 / 干脆不画

macOS 侧只有 61 个在用 `idea::` 常量里的 12 个有同名对应，其余 49 个（`ui/close`、`ui/save`、
`files/xsd`…）会缺图或风格混用。用户裁决：**取并集**，把在用的旧图标留下。

### 备选方案四：保留 pierre 等四个图标主题

它们是 Windows 前端的文件图标主题副本，其中 pierre 还被登记成可卸载扩展。用户裁决：**删除**，
只保留这一套；随之删掉 `lithe.pierreIcons`（bundled 列表 + `shared/fixtures/plugins/official-v2.json`）。

## 后果

- 收益：`AllAssets` 的 731 KiB 从二进制消失（只留 46 KiB 的回落子集）；图标来源单一可讲清
  （macOS 副本 + `SOURCES.md` 台账）；新增 `IconName` 忘登记会被两道判据拦住。
- 代价：`ui/` 那批通用字形与部分文件类型继续来自 Windows 前端那份美术 —— 它们与 macOS 那份同源
  （intellij-community expui）但不是同一版本，视觉上可能有细微差异；要彻底统一得先把这些图标补进
  macOS 图标文件夹。
- 代价：删除四个图标主题文件夹是**能力削减** —— 宿主不再提供 pierre 等可选文件图标主题
  （`assets/icons/` 下只剩 `idea/`），扩展页里 pierre 那一项已随之移除。
- 需要重新评估的触发条件：gpui-kit 升级（组件内部图标名可能变）、或 macOS 侧补齐 `ui/` 字形后
  可以清掉 `legacy/`。

## 验证

```bash
node rust/lithe-gpui/tools/sync-macos-idea-icons.mjs --check    # macOS 副本齐全 + idea.rs 引用都在包里
node rust/lithe-gpui/tools/sync-gpui-kit-icons.mjs --check      # 宿主/组件用到的名字都已登记
cargo test --manifest-path rust/Cargo.toml -p lithe-gpui-app       # 11 条（含资产守卫与回落子集）
cargo test --manifest-path rust/Cargo.toml -p lithe-gpui-settings  # 171 条
cargo test --manifest-path rust/Cargo.toml -p lithe-gpui-workbench # 205 条
cargo test --manifest-path rust/Cargo.toml -p lithe-gpui-editor    # 72 条
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
```

- 实测数字：内嵌资源 411 → **310** 个路径（图标包 302 = macOS 副本 178 + legacy 122 + 清单 + 台账），
  3 325 504 → **3 244 470** 字节；gpui-kit 侧从 1830 个 / 749 008 字节降到 **106 个 / 约 46 KiB**。
- 真机取证（debug 构建 + `--open-settings`，`PrintWindow` 抓窗口，
  `.artifacts/editor-probe/icons2/printwindow.png`）：资源管理器里 11 种文件类型（java/kt/ts/xsd/
  graphql/md/xml/css/py/Dockerfile/未知扩展名）各画出了新美术的图标；设置对话框的 14 个分类图标、
  关闭键、下拉箭头全部可见 —— **没有因为去掉 `AllAssets` 而出现空白**。
- `lithe-gpui-extensions` 仍有 2 条失败，是既有的 B2（Core 没有 `plugins.listCatalog`），与本批无关。

## 适用范围

- `rust/lithe-gpui/tools/sync-macos-idea-icons.mjs`、`tools/sync-gpui-kit-icons.mjs`、
  `tools/macos-idea-icon-map.json`（映射覆写，按需新增）。
- `rust/lithe-gpui/assets/icons/idea/**`（生成物：macOS 副本 + `legacy/**` + `extension.json` + `SOURCES.md`）。
- `rust/lithe-gpui/crates/app/src/assets.rs`（内嵌资源 + gpui-kit 回落子集 + 守卫）。
- `rust/lithe-gpui/crates/shared/src/icons/idea.rs`（生成物）。
- `rust/lithe-gpui/crates/settings/src/extensions.rs`、`shared/fixtures/plugins/official-v2.json`
  （删除 `lithe.pierreIcons`）。
- `rust/lithe-gpui/UI-MAP.md` §1.1 第 4 条、`docs/tasks/gpui-core-first.md` 的 `D25`（均按本决定订正）。
