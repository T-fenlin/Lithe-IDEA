# 图标资源清单：我们画什么 vs. Windows 真源是什么

本文回答一个问题：**gpui 前端现在画图标的地方，各自用的字形在 Windows 真源里对应什么？能不能 1:1 还原？**

配套文档：

- `gpui/assets/README.md` 第 7 节 —— 本次把图标资源搬进 `gpui/assets/ui-icons/**` 与
  `gpui/assets/icon-themes/**` 的来源、文件数、字节数与 sha256 复核方法。
- `gpui/research/app-icon-and-assets.md` —— gpui 侧 `AssetSource` 包装层的设计依据。

基线：分支 `feat/gpui-shell-rewrite`，HEAD `a3ed52e7`。真源只读，`windows/**`、`macos/**` 未被修改。

---

## 0. 结论摘要

1. **我们当前用了 27 个 Lucide 字形**（在本文第 3 节列出的 8 个绘制点里，去重后；全 `gpui/crates/**`
   代码范围是 51 个，另有 2 个只出现在文档注释里）。
2. 这 27 个里有 **20 个**能在本次刚搬进 `gpui/assets/` 的两个新目录里找到真源图标文件：
   **14 个**在 `gpui/assets/ui-icons/idea/**`（JetBrains `expui` SVG），**6 个**在
   `gpui/assets/icon-themes/**`（文件类型图标包，但那是"按文件名查表"的另一套体系，不是同一字形）。
3. **4 个**的真源本来就是 Lucide 字形（`lucide-react` npm 包，仓库里没有文件）——gpui 已经用的就是
   同一个字形，**已经是 1:1**。
4. **2 个**的真源**只有内联 SVG 的 React 组件，没有 SVG 文件**，因此**不可能**靠搬资源 1:1 还原：
   `MavenIcon`（`features/maven/components/maven-icon.tsx`）与 `RunIcon`
   （`features/run/components/run-icon.tsx`）。**1 个**（`PanelBottom`）在真源里根本没有对应图标。
5. `gpui/assets/ui-icons/**` 与 `gpui/assets/icon-themes/**` 目前**没有任何 Rust 代码引用**
   （在 `gpui/**` 里检索 `ui-icons`、`icon-themes`、`assets/ui` 零命中）。和 `gpui/assets/README.md`
   第 1 节的那 8 个位图一样，本次只负责"在删 `windows/` 之前把资源保住"，接线是后续任务。

---

## 1. 本次搬入的资源

| 目标 | 内容 | 文件数 / 字节 |
| --- | --- | --- |
| `gpui/assets/ui-icons/**` | Windows `ui/icons/**` 原样（157 个 IntelliJ `expui` SVG + 1 个生成物 `idea-assets.generated.ts`） | 158 / 150 309 |
| `gpui/assets/icon-themes/lithe/**` | Lithe 自研文件类型图标包（含 `icons/light/` 浅色变体） | 459 / 703 513 |
| `gpui/assets/icon-themes/symbols/**` | Symbols 文件类型图标包 | 325 / 4 511 394 |
| `gpui/assets/icon-themes/pierre/**` | Pierre 文件类型图标包 | 149 / 122 415 |
| `gpui/assets/icon-themes/idea/**` | IntelliJ 文件类型/目录图标包 | 104 / 144 097 |

四套主题包合计 1 037 个文件 / 1 027 个 SVG / 5 481 419 字节；加 `ui-icons` 后共
**1 195 个文件 / 1 184 个 SVG / 5 631 728 字节**，全部逐文件 SHA256 比对通过（0 缺失 / 0 不同 / 0 多余）。

`ui-icons/idea-assets.generated.ts` 是理解第 2 节的关键：它有 **95 个显示名 → 188 条 `?url` import**，
而 `ui-icons/idea/**` 的 **157 个 SVG 全部被它引用**（无孤儿）。这份 TS 在 gpui 侧不会被编译
（`gpui/` 下没有 `package.json` / `tsconfig.json`），只作只读映射参考。

---

## 2. Windows 真源的图标体系：三层 + 一层

真源只有一个出口：`windows/tauri/src/ui/icons.tsx`。每个导出都是
`createIconComponent(<组件>, "<显示名>")`（`:148-221`），决议顺序是：

**第 1 层 —— `ideaIconAssets[显示名]` 命中（`:173`）。**
`ideaIconAssets` 来自 `./icons/idea-assets.generated`，键是**显示名**（`"FilesIcon"` 这种）。
命中就渲染一段固定配色的结构（`:176-207`）：

```jsx
<svg viewBox="0 0 16 16">
  <image className="lithe-idea-icon-light" href={asset.light} width={16} height={16} />
  <image className="lithe-idea-icon-dark"  href={asset.dark}  width={16} height={16} />
</svg>
```

两个 `<image>` 同时在 DOM 里，由 CSS 按当前主题显隐，所以切主题不用重渲染。
**这一层的真源物是文件**：`windows/tauri/src/ui/icons/idea/**` 的 JetBrains `expui` SVG，
现在在 `gpui/assets/ui-icons/idea/**`。

**第 2 层 —— 未命中就渲染第一个参数那个组件。**
绝大多数是 `Nucleo.IconXxxOutline18`，而 `Nucleo` 是 `new Proxy(lucideIcons, …)`（`:108-118`）：
读取属性时把 `IconXxxOutline18` 去掉 `Icon` 前缀和 `Outline18` 后缀得到 `Xxx`，依次查

```js
lucideIcons[Xxx] ?? legacyIconCompatibility[Xxx] ?? CircleHelp
```

`lucideIcons` 就是 `lucide-react`（`windows/tauri/package.json` 里 `^0.468.0`）的整包导出，
`legacyIconCompatibility` 是 `icons.tsx:16-106` 里手写的 **89** 条 Nucleo 名 → Lucide 名映射。
**这一层的真源物是 npm 包里的字形，仓库内没有文件**；而且 `windows/node_modules/` 在工作区里
不存在（`Test-Path` 为 false），所以连"从 node_modules 复制一份"都做不到。

⚠️ 注意第 2 层的解析用的是**第一个参数的 Nucleo 名**，不是显示名。例如
`export const TerminalWindowIcon = createIconComponent(Nucleo.IconSquareTerminalOutline18, "TerminalWindowIcon")`
解析出来是 Lucide `SquareTerminal`，而不是一个叫 `TerminalWindow` 的字形。
本文第 3 节的所有"Lucide 字形"列都是按**第一个参数**推出的。

**第 3 层 —— 少数导出完全不走上面两条路，是手写的内联 `<svg><path>` 组件。**
全 `windows/tauri/src` 范围扫 `<path` 只命中 7 个文件：

| 文件 | `<path>` 数 | 相关导出 |
| --- | --- | --- |
| `features/run/components/run-icon.tsx` | 4 | `RunIcon`、`JavaCupIcon` |
| `features/maven/components/maven-icon.tsx` | 1 | `MavenIcon`（头部注明 JetBrains 字形，Apache-2.0） |
| `extensions/icon-themes/components/themed-file-icon.tsx` | 2 | 符号链接角标（内联在 `:100-115`） |
| `extensions/v0/components/v0-icon.tsx` | 2 | v0 品牌字形 |
| `features/ai/components/icons/provider-icons.tsx` | 15 | AI 供应商 logo |
| `features/git/components/log/git-reference-toolbar-icons.tsx` | 8 | Git 引用工具条 |
| `features/git/components/log/git-graph-row.tsx` | 1 | 提交图节点 |

**额外一层 —— 文件类型/目录图标不走 `@/ui/icons`。**
真源用 `ThemedFileIcon`（`extensions/icon-themes/components/themed-file-icon.tsx`），
它读当前图标主题（默认 `iconTheme: "idea-icons"`，`features/settings/config/default-settings.ts:100`），
由 `icon-theme-registry` 按主题的 `extension.json` 把文件名/后缀/目录名映射到主题目录里的 SVG
（可内联 `svg` 字符串、可 `url`、也可 `component`）。**这一层的真源物就是本次搬进来的
`gpui/assets/icon-themes/**`**，缺的只是查找层。

顺带说明 `bundled-icon-theme-assets.ts` 的 glob 是 `{idea,material,pierre,symbols}` ——
**`lithe` 不在这张表里**（它的 SVG 由 `extensions/ui/components/extensions-sidebar.tsx:126-129`
单独 glob `icon-themes/lithe/icons/files/*.svg`）；`minimal/` 全仓无引用。这两点连同
`material` 没有 SVG 文件的事实，都记在 `gpui/assets/README.md` 第 7.3 节。

---

## 3. 逐个绘制点对照

表格列的含义：

- **真源形态**：`expui SVG`（第 1 层文件）/ `lucide-react`（第 2 层，无文件）/ `内联组件`（第 3 层，无文件）/
  `主题 SVG`（`ThemedFileIcon` 那一层）/ `无`（真源没有这个图标）。
- **1:1 可行性**：
  - `expui SVG` → **可以**：文件已在 `gpui/assets/ui-icons/`，装好 `AssetSource` 后按显示名索引、
    按主题挑 light/dark 即可原样渲染。
  - `lucide-react` → **已经是 1:1**：真源本来就没有文件，Lucide 字形是它的唯一形态；gpui 的
    `gpui_kit::assets::IconName` 就是按同一套 Lucide 目录（1830 个 SVG）生成的枚举，
    变体名 = svg 文件名的 PascalCase，所以两边是同一个字形。
  - `内联组件` → **不可以**：没有 SVG 文件，只能移植 path 数据（见第 4 节）。
  - `主题 SVG` → **可以**：文件已在 `gpui/assets/icon-themes/`，但必须先实现
    `extension.json` → SVG 的查找层，且它给的是"按类型区分的美术"而不是同一个字形。
  - `无` → 真源没有对应图标，谈不到还原。

### 3.1 `gpui/crates/workbench/src/activity_bar.rs` —— 左右活动栏

`activity_bar.rs` 本身**不含任何 `IconName::` 字面量**（实测 0 处），它是无状态渲染器；
图标项由 `workbench/src/workspace.rs` 的 `activity_items()`（左栏）与 `right_activity_items()`（右栏）提供。

⚠️ **数量差异（照抄任务描述会写错）**：Windows 是左栏 9 项
（`features/layout/components/sidebar/main-sidebar.tsx:191-215` 的 `SIDEBAR_ACTIVITY_ITEM_IDS`）
+ 右栏 3 项（`features/layout/components/plugin-activity-rail.tsx:31-67`）。
gpui 侧 `activity_items()` 只有 **8 项** —— 因为左栏的 Maven 与右栏的 Maven 是同一个
`toggleMavenPane`，gpui 有意删掉左栏那一项、只保留右栏入口（理由写在 `workspace.rs:1130-1142`）。
所以下表左栏是 8 行 + 「Maven（已删）」1 行说明。

| Windows 用途 | Windows 组件（出处） | gpui `IconName` | Lucide 字形 | 真源形态 | 1:1 可行性 |
| --- | --- | --- | --- | --- | --- |
| 项目 | `FilesIcon`（`main-sidebar.tsx:193`） | `FolderOpen` | `folder-open` | `expui SVG` → `ui-icons/idea/expui/general/listFiles.svg`(+`_dark`) | 可以（⚠️ 美术不同：真源是 JetBrains 的"文件列表"，不是打开的文件夹，属**语义近似**而非同一笔） |
| 更改 | `GitBranchIcon`（`:194`） | `GitBranch` | `git-branch` | `expui SVG` → `ui-icons/idea/vcs/branch.svg` | 可以 |
| 搜索 | `MagnifyingGlassIcon`（`:195`） | `Search` | `search` | `expui SVG` → `ui-icons/idea/expui/general/search.svg` | 可以 |
| 运行 | `RunIcon`（`:204`） | `Play` | `play` | `内联组件`（`features/run/components/run-icon.tsx`，1 条 stroke path，viewBox `0 0 16 16`） | **不可以**（另注：`PlayIcon` 在真源里存在且解析到 `ui-icons/idea/expui/run/run.svg`，但活动栏用的不是它） |
| 终端 | `TerminalWindowIcon`（`:207`） | `SquareTerminal` | `square-terminal` | `lucide-react`（`Nucleo.IconSquareTerminalOutline18`） | 已经是 1:1 |
| 诊断 | `WarningIcon`（`:211`） | `TriangleAlert` | `triangle-alert` | `expui SVG` → `ui-icons/idea/expui/general/warningDialog.svg` | 可以 |
| 提交记录 | `GitGraphIcon`（`:213`） | `GitGraph` | `git-graph` | `lucide-react`（`Nucleo.IconGitGraphOutline18`，未登记在 `legacyIconCompatibility`） | 已经是 1:1 |
| 设置 | `GearIcon`（`:214`） | `Settings` | `settings` | `expui SVG` → `ui-icons/idea/expui/general/settings.svg` | 可以 |
| Maven（**gpui 已从本栏删除**） | `MavenIcon`（`:201`） | —— | —— | `内联组件`（`features/maven/components/maven-icon.tsx`） | 见右栏 Maven 行 |
| 扩展（右栏） | `PuzzlePieceIcon`（`plugin-activity-rail.tsx:48`） | `Puzzle` | `puzzle` | `lucide-react`（`Nucleo.IconPuzzlePieceOutline18` → `legacyIconCompatibility.PuzzlePiece = lucideIcons.Puzzle`） | 已经是 1:1 |
| 通知（右栏） | `BellIcon`（`features/notifications/components/notifications-trigger.tsx:43`） | `Bell` | `bell` | `expui SVG` → `ui-icons/idea/expui/toolwindows/notifications.svg` | 可以 |
| Maven（右栏） | `MavenIcon`（`plugin-activity-rail.tsx:64`） | `Package` | `package` | `内联组件`（`features/maven/components/maven-icon.tsx`，1 条 fill path，viewBox `0 0 16 16`） | **不可以**；gpui 用 `Package` 是"Lucide 没有 Maven 字形"的替代取舍（`activity_bar.rs:66` 有记录） |

> 关于任务描述里"活动栏那几个字形在真源里是内联 SVG 的 React 组件"：**只有 `MavenIcon` 与 `RunIcon`
> 成立**。`PuzzlePieceIcon` 不是——它在 `windows/tauri/src/ui/icons.tsx:515-518` 是
> `createIconComponent(Nucleo.IconPuzzlePieceOutline18, "PuzzlePieceIcon")`，既不在
> `ui/icons/idea-assets.generated.ts` 的 95 个键里，也没有内联 path；它经 `Nucleo` 代理解析到
> `legacyIconCompatibility.PuzzlePiece = lucideIcons.Puzzle`，即 **Lucide `puzzle`**。
> 而 gpui 用的 `IconName::Puzzle` 就是同一个 Lucide 字形 —— **这一项已经是 1:1，不是缺口。**

### 3.2 `gpui/crates/workbench/src/status_bar.rs` —— 状态栏

同样地，`status_bar.rs` 自己不写 `IconName::` 字面量，条目在 `workspace.rs:518-520,737-743` 组装。

| 用途 | Windows 组件（出处） | gpui `IconName` | Lucide 字形 | 真源形态 | 1:1 可行性 |
| --- | --- | --- | --- | --- | --- |
| 文件路径项前的文件类型图标 | `ThemedFileIcon`（`features/editor/components/toolbar/file-path-breadcrumb.tsx:196-202`；`status_bar.rs` 的模块注释把它记作 `:199`，指的是里面那行 `fileName`） | `FileText` / `File` | `file-text` / `file` | `主题 SVG` | 可以（需查找层） |
| Git 分支 | `GitBranchIcon`（`features/git/components/git-branch-manager.tsx:647`） | `GitBranch` | `git-branch` | `expui SVG` → `ui-icons/idea/vcs/branch.svg` | 可以 |
| 只读锁 | `LockIcon` / `LockOpenIcon`（`footer-editor-status.tsx:102`） | `Lock`（`LockOpen` 只写在文档里，未接） | `lock` / `lock-open` | `expui SVG` → `ui-icons/idea/expui/general/locked.svg` / `unlocked.svg` | 可以 |
| 内存 | `HardDrivesIcon`（`footer-editor-status.tsx:113`） | `HardDrive` | `hard-drive` | `lucide-react`（`Nucleo.IconHardDriveOutline18`） | 已经是 1:1 |
| 无错误标记 | `CheckCircleIcon`（`footer-editor-status.tsx:135`） | `CircleCheck` | `circle-check` | `expui SVG` → `ui-icons/idea/expui/general/successDialog.svg` | 可以 |
| 面包屑分隔符 | `CaretRightIcon`（`ui/breadcrumb.tsx:4,78`，`as ChevronRight`） | `ChevronRight` | `chevron-right` | `expui SVG` → `ui-icons/idea/expui/general/chevronRight.svg` | 可以 |

### 3.3 `gpui/crates/workbench/src/project_tabs.rs` —— 项目标签

| 用途 | Windows 组件（出处） | gpui `IconName` | Lucide 字形 | 真源形态 | 1:1 可行性 |
| --- | --- | --- | --- | --- | --- |
| 标签左侧文件夹 | `FolderIcon`（`features/window/components/project-tab-bar.tsx:6,70`） | `Folder` | `folder` | `expui SVG` → `ui-icons/idea/expui/nodes/folder.svg` | 可以 |
| 关闭按钮 | `XIcon as X`（同文件 `:6,95`） | `X` | `x` | `expui SVG` → `ui-icons/idea/expui/general/close.svg` | 可以 |

### 3.4 `gpui/crates/workbench/src/right_tool_window.rs` —— 右侧工具窗

`right_tool_window.rs:130-134` 的 `icon()` 与右活动栏逐项同字形（模块注释自己写明了这一点）。

| 用途 | Windows 组件 | gpui `IconName` | Lucide 字形 | 真源形态 | 1:1 可行性 |
| --- | --- | --- | --- | --- | --- |
| 扩展面板标题 | `PuzzlePieceIcon` | `Puzzle` | `puzzle` | `lucide-react` | 已经是 1:1 |
| 通知面板标题 | `BellIcon` | `Bell` | `bell` | `expui SVG` → `ui-icons/idea/expui/toolwindows/notifications.svg` | 可以 |
| Maven 面板标题 | `MavenIcon` | `Package` | `package` | `内联组件` | **不可以** |
| 关闭面板按钮（`:261`） | `XIcon as X`（`ui/dialog.tsx:23,136`） | `X` | `x` | `expui SVG` → `ui-icons/idea/expui/general/close.svg` | 可以 |

### 3.5 `gpui/crates/workbench/src/command_palette.rs` —— 命令面板

**`command_palette.rs` 里没有任何图标字面量**（实测 0 处）。命令面板的行图标由
`workspace.rs::command_action`（`:235-242`）随动作表一起给出，所以真源对照要去
`features/command-palette/constants/{settings-actions,view-actions}.tsx` 看。

| 动作 | 真源 | gpui `IconName` | Lucide 字形 | 真源形态 | 1:1 可行性 |
| --- | --- | --- | --- | --- | --- |
| 打开设置 / 打开外观设置 | `settings-actions.tsx:155-163` | `Settings` | `settings` | `expui SVG` → `ui-icons/idea/expui/general/settings.svg` | 可以 |
| 切换浅色 / 深色主题 | `settings-actions.tsx` 的 `PaletteIcon` | `Palette` | `palette` | `expui SVG` → `ui-icons/idea/expui/toolwindows/palette.svg` | 可以 |
| 开关终端 | `view-actions.tsx:125-145` | `SquareTerminal` | `square-terminal` | `lucide-react`（`TerminalWindowIcon`） | 已经是 1:1 |
| 开关 Maven | 左活动栏 maven 项 → `maven-tool-window-actions.ts:58` | `Package` | `package` | `内联组件` | **不可以** |
| 开关状态栏 | **真源没有这个动作**（它是设置项 `settings.appearance.showStatusBar`） | `PanelBottom` | `panel-bottom` | `无` | 谈不到（gpui 新增动作，真源无图标可对） |

### 3.6 `gpui/crates/explorer/src/` —— 文件浏览器

绘制点：`model.rs:275-304` 的 `icon_for_file`、`explorer_view.rs:412-607` 的工具栏/空态/树节点。
真源对应物是 `ThemedFileIcon` + 图标主题包（`features/file-explorer/components/file-explorer-tree-item.tsx:160,233`
与 `features/sidebar/components/sidebar-tree.tsx:290-323` 的 disclosure）。

| 用途 | gpui `IconName` | Lucide 字形 | 真源对应 | 真源形态 | 1:1 可行性 |
| --- | --- | --- | --- | --- | --- |
| 图片类文件 | `Image` | `image` | 主题包按后缀查表（`ImageIcon` 在 `expui` 里也有 `fileTypes/image.svg`） | `主题 SVG` | 可以（需查找层） |
| JSON / JSONC | `FileBraces` | `file-braces` | 主题包按后缀查表 | `主题 SVG` | 可以（需查找层） |
| Markdown / 文本 | `FileText` | `file-text` | 主题包按后缀查表 | `主题 SVG` | 可以（需查找层） |
| 配置（toml/yaml/ini/…） | `Settings` | `settings` | 主题包按后缀查表（真源不会用齿轮画配置文件） | `主题 SVG` | 可以（需查找层，且**美术语义不同**） |
| 依赖锁 / 包文件 | `Package` | `package` | 主题包按后缀查表 | `主题 SVG` | 可以（需查找层） |
| 脚本（sh/ps1/bat/…） | `Terminal` | `terminal` | 主题包按后缀查表 | `主题 SVG` | 可以（需查找层） |
| 源码（rs/ts/java/…） | `FileCode` | `file-code` | 主题包按后缀查表（真源按语言给不同彩色字形） | `主题 SVG` | 可以（需查找层；真源**分语言**，我们只有一个通用字形） |
| 其余 / 无后缀 | `File` | `file` | 主题包按后缀查表 | `主题 SVG` | 可以（需查找层） |
| 目录（展开 / 折叠） | `FolderOpen` / `FolderClosed` | `folder-open` / `folder-closed` | 主题包按 `folderNames` / 默认目录字形 | `主题 SVG` | 可以（需查找层） |
| 树的展开箭头 | `ChevronDown` / `ChevronRight` | `chevron-down` / `chevron-right` | `ChevronDownIcon` / `ChevronRightIcon`（`sidebar-tree.tsx:314,316`，`weight="bold"`，函数在 `:290-318`） | `expui SVG` → `ui-icons/idea/expui/general/chevronDown.svg` / `chevronRight.svg` | 可以 |
| 搜索图标 | `Search` | `search` | 工具栏自有图标 | `expui SVG` → `ui-icons/idea/expui/general/search.svg` | 可以 |
| 清空搜索 | `X` | `x` | `XIcon` | `expui SVG` → `ui-icons/idea/expui/general/close.svg` | 可以 |
| 筛选/设置入口 | `Settings` | `settings` | `GearIcon` | `expui SVG` → `ui-icons/idea/expui/general/settings.svg` | 可以 |

### 3.7 `gpui/crates/editor/src/buffer.rs` —— 编辑器缓冲区标题图标

`buffer.rs:33-63` 的 `icon_for_file` 与 3.6 的 `explorer/src/model.rs::icon_for_file` 是**同一份判断、
逐条同值**。真源同样是 `ThemedFileIcon`（`features/tabs/components/tab-bar-item.tsx:246-251`）。

用到的字形与 3.6 完全重合：`Image`、`FileBraces`、`FileText`、`Settings`、`Package`、`Terminal`、
`FileCode`、`File`。**没有新增字形。**

`buffer.rs` 模块注释里有一句"那是 Windows 自己的一套图标资源，gpui-kit 侧无法等价复刻"——
本次搬运之后这句话的前半段不再成立：那套资源现在就在 `gpui/assets/icon-themes/`，
剩下的只是查找层。

### 3.8 `gpui/crates/settings/src/dialog.rs` —— 设置对话框分类图标

真源：`features/settings/components/settings-dialog.tsx:34-47` 的 12 个分类。gpui 侧目前只实现 2 个分类。

| 分类 | Windows 组件（出处） | gpui `IconName` | Lucide 字形 | 真源形态 | 1:1 可行性 |
| --- | --- | --- | --- | --- | --- |
| 常规 | `GearSixIcon`（`:36`） | `Settings` | `settings` | `expui SVG` → `ui-icons/idea/expui/general/settings.svg` | 可以（⚠️ `GearSix` 与 `GearIcon` 在真源里是**两个不同显示名**，但 `idea-assets.generated.ts` 都把 `GearIcon`/`GearSixIcon` 指到同一个 `settings.svg`，所以确实同一笔） |
| 外观 | **真源外观页没有分类图标**（`settings-dialog.tsx:34-47` 里没有 `appearance` 那一行） | `Palette` | `palette` | `expui SVG` → `ui-icons/idea/expui/toolwindows/palette.svg`（`PaletteIcon` 另有使用者，如主题动作） | 可以，但**真源没有这个用法**，属 gpui 侧自选（`dialog.rs:200-201` 有记录） |
| 关闭按钮（`:294`） | `XIcon as X`（`ui/dialog.tsx:23,136`） | `Close` | `x` | `expui SVG` → `ui-icons/idea/expui/general/close.svg` | 可以 |

### 3.9 其它绘制点（不在任务列出的 8 个之内，附此备查）

| 站点 | gpui `IconName` | 真源 | 真源形态 |
| --- | --- | --- | --- |
| `workbench/src/title_bar.rs:225-228` 窗口按钮 | `WindowMinimize` / `WindowMaximize` / `WindowRestore` / `WindowClose` | `features/window/components/title-bar/window-controls.tsx:2-7`：`MinusIcon`(→`remove.svg`)、`SquareIcon`(→ Lucide `square`)、`CopyIcon`(→`copy.svg`)、`XIcon`(→`close.svg`) | 混合：2 个 `expui SVG`、1 个 `lucide-react`、1 个 `expui SVG` |
| `git/src/log_view.rs`（23 个字形） | `Copy`、`Eye`、`EyeOff`、`Trash`、`Plus`、`Minus`、`Search`、`Settings`、`Folder`、`GitBranch`、`ChevronDown/Right`、`ArrowDown`、`Check`… | 大部分命中 `expui`（`copy.svg`、`show.svg`、`hide.svg`、`delete.svg`、`add.svg`、`remove.svg`、`search.svg`、`settings.svg`、`nodes/folder.svg`、`vcs/branch.svg`、`chevronDown/Right.svg`） | `expui SVG` |
| `terminal/src/terminal_view.rs`（6 个） | `SquareTerminal`、`Play`、`Plus`、`Trash`、`ChevronDown`、`Close` | `TerminalWindowIcon`、`PlayIcon`(→`run/run.svg`)、`PlusIcon`、`TrashIcon`、`ChevronDownIcon`、`XIcon` | 混合：5 个 `expui SVG`、1 个 `lucide-react` |
| `editor/src/editor_view.rs`（5 个） | `ArrowLeft`、`ArrowRight`、`FileText`、`Search`、`X` | 真源编辑器工具栏 | `expui SVG` |

⚠️ `terminal_view.rs:532,657` 记录了一个反向事实：`IconName::Trash2` **根本不存在**，
因为全量 Lucide 目录里没有 `trash-2.svg`。这是"我们画的字形要在资源里找得到"的既有教训。

---

## 4. 不可能 1:1 还原的地方（只有内联组件，没有 SVG 文件）

这两处是本次结论里唯一的**硬缺口**：

| 组件 | 文件 | 内容 | 许可 |
| --- | --- | --- | --- |
| `MavenIcon` | `windows/tauri/src/features/maven/components/maven-icon.tsx` | 1 条 `<path fill="currentColor">`，`viewBox="0 0 16 16"`，`fill="none"` 根节点 | 文件头注明 "JetBrains Maven tool window glyph, used under the Apache 2.0 license" |
| `RunIcon` | `windows/tauri/src/features/run/components/run-icon.tsx` | 1 条 `<path stroke="currentColor" strokeWidth="1.25" strokeLinejoin="round" strokeLinecap="round">`，`viewBox="0 0 16 16"`（同文件另有 `JavaCupIcon`，4 条 path） | 无单独声明 |

同一类（同样只有内联 path、没有 SVG 文件）但**不在这 8 个绘制点上**的还有第 2 节表里的
`provider-icons.tsx`（15 条）、`git-reference-toolbar-icons.tsx`（8 条）、`v0-icon.tsx`（2 条）、
`git-graph-row.tsx`（1 条）、`themed-file-icon.tsx` 的符号链接角标（2 条）。

还有一类"没有文件也没有组件，只有 npm 包"的：**`lucide-react` 的全部字形**。它不构成缺口
（gpui 自带同一套 Lucide，见第 0 节第 3 点），但如果哪天要"逐字节等于 Windows 那一份"，
就必须把 `lucide-react@0.468` 装出来再取文件，而 `windows/node_modules/` 在本工作区不存在。

### 要 1:1 还原 `MavenIcon` / `RunIcon` 的可行路径与代价（**本次不做**）

**前提**：gpui 现在注册的是 `gpui_kit::assets::AllAssets`，它只嵌 Lucide 一套；
`IconName` 是一个**封闭枚举**（由 `gpui-kit-assets-0.6.6/build.rs:20-51` 按目录生成）。
要画上面这些美术，必须先有自己的 `AssetSource` 包装层（组合 `AllAssets` + `gpui/assets/**`），
设计依据见 `gpui/research/app-icon-and-assets.md`。

在此之上有两条路：

1. **把 path 数据落成 SVG 文件**（推荐，成本低、可复用 `AssetSource`）。
   把 1 条 path 原样写进 `gpui/assets/**`，再扩一个"自定义图标名 → 资源路径"的表。
   代价：新增资源文件；许可与出处必须跟着搬（`maven-icon.tsx` 的 Apache-2.0 声明要写进
   `gpui/assets/README.md` 或伴随的 `NOTICE`）。
2. **把 path 数据当 Rust 常量，用 GPUI 的路径绘制能力画**。
   代价：要自己处理 `stroke` / `fill` / `strokeWidth` / `linecap` / `linejoin` 的差异
   （`RunIcon` 是 stroke 1.25 + `currentColor`，`MavenIcon` 是 fill `currentColor`），
   还要自己跟主题色；收益是不新增文件。当前 gpui 侧没有这样的绘制原语，工作量明显更大。

两条路都**不在本次任务范围内**（本次是纯资源搬运 + 文档，硬约束禁止改任何 Rust 代码）。

### 文件类型图标（第 3.6/3.7 节）要做的事

资源已经就位（`gpui/assets/icon-themes/**` 4 套 + 各自的 `extension.json`），缺的是
`ThemedFileIcon` 的等价物：一个"主题 id → `extension.json` 的 `iconDefinitions` → SVG 字节"
的查找层，加上 `fileNames` / `fileExtensions` / `folderNames` 的匹配顺序与 `icons/light/` 变体选择。
注意 `lithe` 主题的资源在仓库里（`icon-themes/lithe/icons/{files,folders,light/…}`），
但 `material` 主题**没有 SVG 文件**（美术内联在 `extension.json` 里），
`minimal` 主题全仓无引用 —— 详见 `gpui/assets/README.md` 第 7.3 节。

---

## 5. 统计口径与结果

### 口径

- **分母**：第 3.1–3.8 节（任务列出的 8 个绘制点）里，`gpui` 代码**实际引用**的
  `gpui_kit::assets::IconName` 变体，**去重**。只写在 `//!` / `///` 注释里的变体不算
  （按这条规则，`activity_bar.rs`、`status_bar.rs`、`command_palette.rs` 三个文件的字面量计数是 0，
  它们的图标来自 `workspace.rs`）。
- **分子**：在 `gpui/assets/ui-icons/**` 或 `gpui/assets/icon-themes/**` 里能找到
  "近似或等价的真源图标文件"的字形数。**"已经是 1:1 的 Lucide 字形"单独计一栏**，
  因为它们虽然在语义上完全对得上，但真源**没有文件**，不属于"刚搬进来的资源"。

### 结果

| 分类 | 数量 | 占比 | 字形 |
| --- | --- | --- | --- |
| `expui SVG` 文件已在 `gpui/assets/ui-icons/**` | **14** | 51.9% | `FolderOpen`、`GitBranch`、`Search`、`TriangleAlert`、`Settings`、`Bell`、`Lock`、`CircleCheck`、`ChevronRight`、`ChevronDown`、`Folder`、`X`、`Close`、`Palette` |
| 主题 SVG 文件已在 `gpui/assets/icon-themes/**`（需查找层） | **6** | 22.2% | `FileText`、`File`、`FileBraces`、`FileCode`、`Terminal`、`FolderClosed` |
| **小计：真源文件已落在本次搬入的两个目录里** | **20** | **74.1%** | |
| 真源本来就是 Lucide（无文件，**已经 1:1**） | 4 | 14.8% | `SquareTerminal`、`GitGraph`、`Puzzle`、`HardDrive` |
| 真源只有内联 SVG 组件（**不可能 1:1**） | 2 | 7.4% | `Play`（真源是 `RunIcon`）、`Package`（真源是 `MavenIcon`） |
| 真源没有对应图标 | 1 | 3.7% | `PanelBottom`（gpui 新增的"状态栏显隐"动作） |
| **合计** | **27** | 100% | |

### 仓库范围（不只是这 8 个点）

| 项 | 数量 |
| --- | --- |
| `gpui/crates/**` 代码里实际引用的 `IconName` 变体（去重） | **51** |
| 只在注释里出现、代码未接的变体 | 2（`LockOpen`、`Trash2`；后者在 Lucide 目录里**不存在**，`terminal_view.rs:657` 有记录） |
| 全量 Lucide 目录（`gpui-kit-assets-0.6.6/assets/icons/`） | 1830 |
| 本次搬入 `ui-icons/` 的 `expui` SVG | 157（全部被 `idea-assets.generated.ts` 引用，无孤儿） |
| 本次搬入 `icon-themes/` 的 SVG | 1 027 |

---

## 6. 一句话总结

> 我们画的 27 个字形里，**20 个**的真源图标文件现在就躺在 `gpui/assets/ui-icons/**` 与
> `gpui/assets/icon-themes/**`（前者可直接渲染，后者要先写一层 `extension.json` 查找层）；
> **4 个**的真源本来就是 Lucide，已经一模一样；真正**无法靠搬资源还原的只有 2 个**
> —— Maven 与 Run 的 JetBrains 字形，它们在真源里是内联 SVG 组件（`maven-icon.tsx` / `run-icon.tsx`），
> 要 1:1 只能移植 path 数据，而这件事需要一个 `AssetSource` 包装层，本次**不做**。
