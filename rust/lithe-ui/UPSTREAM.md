# 上游来源与改动记录

本 crate 起步于 **Ely GPUI Components** 的源码拷贝。决策见
`.agents/notes/implemented/architecture/2026-10-05-lithe-gpui-vendored-from-ely.md`。

> **维护政策（2026-10-05 定）：本 crate 不与上游对接、不跟踪上游。**
> 这是一次性 fork，此后按 Lithe 自己的需要维护；上游只作为**组件参考**。
> 因此本文档的作用是**来源署名与改动留痕**，不再是"同步流程"。
> 上游仅 5 天历史、每天约 20 次提交，跟踪它没有收益。
>
> **2026-10-08 补正**：本文档此前的"改动记录"只记了 2 条重要改动，而实际有 **34 个提交**
> 改过本 crate。判断"这个副本被改成了什么样"**必须以下面的完整提交清单（或 `git log`）为准**，
> 不能只看重要改动表 —— 这正是复用时最容易踩的坑（统一 GPUI 前端选型时的审计发现）。

## 来源

| 项 | 值 |
| --- | --- |
| 项目 | Ely GPUI Components |
| 仓库 | https://github.com/ZacharyZhang-NY/Ely-GPUI-Components |
| 提交 | `b4716bb09bae4ee9c26afc7197d0f8f6c08f92c2`（2026-10-05） |
| 许可 | MIT（Copyright (c) 2026 Ely GPUI Component contributors） |
| 许可原文 | 同目录 `LICENSE-ELY` |

MIT 允许衍生与再分发，**前提是保留版权声明与许可文本**。所以：

- `LICENSE-ELY` 必须随本 crate 一起保留，不得删除。
- 源码中的 `Ely` 字样（`src/**` 的文档注释 30 处）是**来源署名**，不是待改的旧名，
  不要为了"统一命名"清掉它们。要改的是**标识符与用户可见文案**（见下）。

## 上游原始形态

- 单 crate `ely-gpui-component`，edition 2024，MSRV 1.95。
- 依赖 `gpui` / `gpui_platform` 于 zed 上游 git rev
  `1a28cff4b409169bac058bca40dfbfeb7621d19b`。
- 不用 `gpui-base` / `gpui-component`：直接构建在 gpui 框架本身之上。

## 本副本的改动清单

移植时**只做必要改动**，其余保持与上游逐文件一致，以便对照与同步。

### 1. 包与依赖（`Cargo.toml`）

- 包名改为 `lithe-gpui`（2026-10-08 组件库独立时再改名为 `lithe-ui`，见改动记录）。
- **gpui 依赖改为包重命名**，指到 Lithe 钉住的版本线：

  ```toml
  gpui = { package = "gpui-pre", version = "=0.3.8", default-features = false, features = ["stacker"] }
  gpui_platform = { package = "gpui-pre-platform", version = "=0.3.8", features = ["font-kit"] }
  ```

  这一步让源码里 800+ 处 `use gpui::…` / `gpui_platform::…` **一行都不用改**。
  版本线 2026-10-08 由 `=0.3.7` 升到 `=0.3.8`（对应 zed rev
  `279fe070bb389b79652e52065b2f001edcc0b11b`），并已实测通过
  `cargo check --lib` / `--all-targets` / `--examples`（0 error）。
- `[profile.web]` 移除：member crate 不生效（只在 workspace 根有效）。
- 未改动其余依赖。

### 2. 标识符与文案（源码批量替换）

| 旧 | 新 | 说明 |
| --- | --- | --- |
| `ely_gpui_component` | `lithe_ui` | crate 引用（217 处） |
| `ely-gpui-component` | `lithe-gpui` | 同上（连字符形式） |
| `"ely: …"` | `lithe-gpui: …` | panic 文案 |
| `actions!(ely, …)` | `actions!(lithe, …)` | 动作命名空间 |
| `"ely-<id>"` | `"lithe-<id>"` | 元素 ID / 临时文件名 |
| `ely_live_` / `ely_icon` / `ely_terminal` / `ely-theme-` | `lithe_*` | 内部标识 |
| `elyPicked` / `elyRoute` | `lithePicked` / `litheRoute` | macOS 原生桥标识 |
| `ELY_GALLERY_ASSETS` | `LITHE_GALLERY_ASSETS` | 画廊构建期环境变量 |

**2026-10-08 第二次改名**：组件库独立时 `lithe_gpui` → `lithe_ui`、`lithe-gpui` → `lithe-ui`（200 个文件）。下表记录的是 2026-10-05 移植时的替换，保留原样以便追溯。

**只改标识符，不改任意字符串数据。** 测试夹具里的 URL、示例文本、临时目录名
（如 `ely.dev`、`"Ely, ely, ELY and élan"`）**一律保持上游原样**：它们是被解析/断言的
数据，改了会连带改坏硬编码的 range 断言，却没有命名收益。（初版移植曾一并改掉，
被 Ely 自带测试抓出 3 处 range 偏差后回退——这也是本副本第一次用上游测试
验证自身改动的记录。）

### 3. 对齐 gpui-kit 的 API 重构（2026-10-05 一批）

> 本文档此前把"API 对齐 gpui-kit"记成**未做**，这与 git 历史不符：实际上做过一批试点与铺开，
> 于 2026-10-08 更正。除非要做别的改动，**不要回退这些重构**。

| 改动 | 说明 |
| --- | --- |
| 尺寸档位：`ControlSize` 3 档 → `Size` 5 档 + `Sizable` | 对齐 gpui-kit 的尺寸词汇 |
| `IconButton` 并入 `Button` | 只留一个按钮组件、一套变体 |
| 文本输入状态 `TextInput` → `InputState` | 对齐 gpui-kit 命名 |
| `Button` / `Switch` / `Checkbox` / `Dialog` 构造改 builder | 对齐 `new(id).…(...)` 形态，去掉整串参数的构造 |
| i18n 换成 `rust_i18n`（catalogue 迁到 `locales/ui.yml`） | 与 gpui-kit 及宿主同一套 i18n |

### 4. 组件增删与缺陷修复

- **新增** `lists::project`（项目树：`ProjectTree` / `ProjectNode` / `ProjectAsset` / `ProjectIcon` /
  `ProjectAction` / `ProjectLabels` 等）——上游没有这个组件，是 Lithe 侧新增代码。
  **2026-10-08：本 crate 整体随组件库独立** —— 从 `rust/lithe-gpui/crates/lithe-gpui` 移到 `rust/lithe-ui` 并改名为 `lithe-ui`；项目树仍是本 crate 的组件（未拆出）。
- 把 `lists::files::GitStatus` 补成 `Copied` 七态、`word()` / `letter()` / `tone()` 转 `pub`、
  新增 `dominant()`。
- 修 `Dock` 操作区按钮 id 按边区分（无障碍节点 id 撞车曾导致 panic）。
- 修浮动面板：补"停靠回去"与"关闭"，并修隐藏的浮动面板仍渲染。
- `Dock` 能力先加后撤：先加了一批能力（开合、每边尺寸范围、最大化、面板头操作、视图注册表、
  已关闭面板条、图标栏、同侧两组、可配外观），后又**撤掉没有消费方的能力**、只留外壳真正用的。

### 5. 未改动（有意）

- **30 处文档注释里的 `Ely`**：署名，保留。
- 全部组件逻辑、主题、布局、章节目录结构。

## 尚未完成

- **图标体系与字体**：上游 `assets/`（3.0 MB，318 文件）含 IBM Plex Sans 与它自己的
  图标集。Lithe 的字体政策（CONTEXT.md：内置字体 JetBrains Mono、界面字体用系统 UI 字体）
  与图标体系（既有 Agent Note）尚未合并，当前先原样随 crate 带入。
- **语义差异**：仅修到能编译为止；同名 API 的行为差异需靠测试逐步暴露。

## 与上游的关系

**不跟踪。** 本 crate 是 Lithe 自己的代码，上游只作**组件参考**：需要某处做法时
照抄思路或拿它做对照，不采用 git 依赖、不做定期 diff 同步、不为对齐上游而改动。

若将来确实要取上游某一处改动，人工比对后手工合入，并在下面留一行记录。

## 改动记录

只记**重要改动**；完整历史见下一节。

| 日期 | 改了什么 | 为什么 |
| --- | --- | --- |
| 2026-10-05 | 移植：包重命名、标识符改名、数据串保持原样 | 见上文"本副本的改动清单" |
| 2026-10-07 | 新增 `lists::project`（`ProjectTree` / `ProjectNode` / `ProjectAsset` / `ProjectIcon` / `ProjectLabels` / `ProjectAction`，导出在 `src/lists/mod.rs`），并把 `lists::files::GitStatus` 补成 `Copied` 七态、`word()`/`letter()`/`tone()` 转 `pub`、新增 `dominant()` | 项目树要变成组件库自己的控件（`.agents/notes/implemented/architecture/2026-10-07-project-tree-moves-to-own-component-library.md`）；git 字母与色调的单一真源按 `docs/tasks/idea-project-tree.md` §4 落在这里。上游没有这个组件，它是 Lithe 侧新增的代码 |
| 2026-10-08 | 组件库独立：本 crate 从 `rust/lithe-gpui/crates/lithe-gpui` 移到 `rust/lithe-ui`，包名与标识符由 `lithe-gpui`/`lithe_gpui` 改为 `lithe-ui`/`lithe_ui`；gpui 依赖由 `=0.3.7` 升到 `=0.3.8` | 分层：界面层（要重写的宿主）留在 `rust/lithe-gpui`，组件库独立成 `rust/lithe-ui` 并由界面层 path 依赖；项目树仍在本 crate 内。见 `.agents/notes/proposed/architecture/2026-10-08-unified-gpui-frontend-selection-and-layer-boundaries.md` |
| 2026-10-08 | 新增 `src/shell/host/`：窗口根宿主 `WindowRoot` 与窗口扩展 `WindowRootExt`；`feedback::toast` 的 `MOST` 由 4 改为 3 | 门 G1：把上游 `Root` 的统一浮层挂载换成自建（复用 `primitives::raise(..).with_priority(..)`，排序与绘制交 gpui 的 `Window::defer_draw`）；`MOST` 与真源（通知上限 3）对齐 |

## 完整提交清单

口径：本 crate 的路径历史共 **39 个提交**，其中 **5 个属于被本 fork 替换掉的早期手写实现**
（`563780c4` / `f1072e48` / `2fd500f7` / `b55f4006` / `c0aa2aa4`），**Ely fork 自身占 34 个**。
下表从 fork 点 `c6da4f26` 起、按时间倒序列出全部 34 条。

| 日期 | 提交 | 说明 |
| --- | --- | --- |
| 2026-10-07 | `bd8f6f01` | feat(gpui): 项目树进组件库 + 补回面板标题行；修行序与右键后行消失 |
| 2026-10-07 | `d844f539` | refactor(gpui): 组件库 Dock 撤掉没有消费方的能力，并修回外壳塌陷布局 |
| 2026-10-06 | `6b50c155` | feat(gpui): 外壳三块工具窗改用组件库自己的 Dock，去掉 gpui-kit 的拖动机制 |
| 2026-10-06 | `064e1cf1` | feat(gpui): Dock 面板可配边框粗细 / 圆角 / 背景，分隔条粗细可配 |
| 2026-10-06 | `1079370d` | feat(gpui): 画廊夹具中文化——291 处调用点包 gallery() + 259 条翻译 |
| 2026-10-06 | `53218f3d` | fix(gpui): 修复 misc 段两行以英文开头的裸翻译（ChangeLog/ShareSheet） |
| 2026-10-06 | `465102a7` | feat(gpui): 画廊阅读层中文化第二批至第四批完成（全部 43 页） |
| 2026-10-06 | `f5c0c177` | feat(gpui): 画廊阅读层中文化第一批——primitives/typography/layout/shell/buttons 的段说明（137 条） |
| 2026-10-06 | `3d42440c` | fix(gallery): 页面挂上滚动条——之前能滚但没有条 |
| 2026-10-05 | `ac66c4d1` | refactor(gpui): 画廊文案与库文案分家——库 catalogue 只留组件文案 |
| 2026-10-05 | `a65c745b` | feat(gpui): Dock 可选图标栏——左右边一列图标，点一下开/切该面板 |
| 2026-10-05 | `387fd8fa` | feat(gpui): Dock 同侧两组——左边可上下叠两个工具窗（优化 4，最后一项） |
| 2026-10-05 | `96c00ba4` | feat(gpui): 画廊 Layout 页的 Dock 段中文化（文案进 catalogue，标题走 PanelTitle::Key） |
| 2026-10-05 | `6c416394` | fix(gpui): 浮动面板加上"停靠回去"和"关闭"，并修隐藏的浮动面板仍渲染 |
| 2026-10-05 | `f307a494` | feat(gpui): Dock 可选"已关闭面板"条，补上关闭后的回程 |
| 2026-10-05 | `cdc08303` | fix(gpui): Dock 操作区按钮 id 按边区分，修无障碍节点 id 撞车导致 panic |
| 2026-10-05 | `0c41ee93` | feat(gpui): Dock 面板视图注册表（优化 7） |
| 2026-10-05 | `36cc5329` | feat(gpui): Dock 面板头操作——关闭 / 浮动 / 最大化（优化 6） |
| 2026-10-05 | `f6989345` | feat(gpui): Dock 最大化/还原（优化 5） |
| 2026-10-05 | `092c06bb` | feat(gpui): Dock 每边独立尺寸范围（优化 3） |
| 2026-10-05 | `ffac1ea2` | feat(gpui): Dock 支持关闭/打开面板 + 面板标题走 i18n（优化 1、2） |
| 2026-10-05 | `d9e96f41` | feat(gpui): 补齐 5 种语言文案 + 画廊顶栏本地化与语言切换 |
| 2026-10-05 | `38f212a5` | refactor(gpui): i18n 换成 rust_i18n（对齐 gpui-kit 与宿主），catalogue 迁到 locales/ui.yml |
| 2026-10-05 | `05db6903` | chore(gpui): 画廊元数据同步组件改名（stories.json 里的 IconButton -> Button） |
| 2026-10-05 | `cb5b869e` | refactor(gpui): Dialog 构造对齐 id-only 约定（new(id, title, on_close) -> new(id).title(..).on_close(..)） |
| 2026-10-05 | `761ce5da` | refactor(gpui): Checkbox 构造对齐 gpui-kit（new(id, state) -> new(id).checked(..)） |
| 2026-10-05 | `8337185f` | refactor(gpui): Switch 构造对齐 gpui-kit（new(id, on) -> new(id).checked(bool)） |
| 2026-10-05 | `93bdd0bd` | refactor(gpui): 文本输入状态类型改名 TextInput -> InputState（对齐 gpui-kit） |
| 2026-10-05 | `ebc74a3a` | refactor(gpui): IconButton 并入 Button（对齐 gpui-kit） |
| 2026-10-05 | `1e585ab0` | refactor(gpui): 尺寸对齐 gpui-kit —— ControlSize 3 档扩到 Size 5 档 + Sizable |
| 2026-10-05 | `339380aa` | refactor(gpui): Button 构造对齐 gpui-kit 形态（试点） |
| 2026-10-05 | `78edf163` | docs(gpui): UPSTREAM 改为来源署名与改动留痕，不再是对接上游的同步流程 |
| 2026-10-05 | `82bca6c0` | fix(gpui): 回退测试数据串的重命名，Ely 自带 659 项测试全绿 |
| 2026-10-05 | `c6da4f26` | feat(gpui): lithe-gpui 改为移植 Ely 组件库（vendored fork），替换原手写实现 |

其中**画廊/文档类改动**（`1079370d`、`53218f3d`、`465102a7`、`f5c0c177`、`3d42440c`、`ac66c4d1`、
`96c00ba4`、`d9e96f41`、`05db6903`）只影响演示与文案，不影响产品运行路径；
组件与行为改动集中在上半部分。