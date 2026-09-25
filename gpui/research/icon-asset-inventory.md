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

---

## 7. 接线（已完成）：`gpui/assets/**` 现在真的能画出来了

第 0 节第 5 点的"资源已就位、缺接线"在本节闭环。基线：分支 `feat/gpui-shell-rewrite`，
本次改动之前的 HEAD `d13b254a`。第 1–6 节描述的资源事实**未变**，本节只新增"怎么引用"。

### 7.1 项目应该怎么引用这些图标（四步）

| 步 | 做什么 | 住在哪 |
| --- | --- | --- |
| 1. 注册 | `application().with_assets(LitheAssets)` —— 内嵌 `gpui/assets/**`，未命中回落 `AllAssets` | `gpui/crates/app/src/assets.rs`（在 `main.rs:261-280` 注册，**只此一处**） |
| 2. 清单 | `idea::ALL`（79 个图标）/ `idea::ALIASES`（旧前端别名）—— 名字 → 明/暗资源路径 | `gpui/crates/shared/src/icons/idea.rs`（**生成物**） |
| 3. helper | `idea_icon_svg(&icon, cx)` 按当前主题明暗挑路径，并设好 16×16 尺寸与前景色 | `gpui/crates/shared/src/icons/mod.rs` |
| 4. 调用点 | `.child(idea_icon_svg(&idea::GEAR_ICON, cx))` | 任何 feature crate（首个真实调用点：`crates/settings/src/dialog.rs` 的设置对话框头部） |

最小代码片段（四步合起来看）：

```rust
// ── 1. 注册（App Shell，做一次）────────────────────────────────────────────
// crates/app/src/main.rs
gpui_kit::application()
    .with_assets(assets::LitheAssets)   // 原来是 gpui_kit::assets::AllAssets
    .run(move |cx| { /* ... */ });

// ── 2 + 3. 清单与 helper（crates/shared/src/icons/）────────────────────────
// idea.rs（生成物）里每个图标是一个常量：
pub const GEAR_ICON: IdeaIcon = IdeaIcon {
    light: "ui-icons/idea/expui/general/settings.svg",
    dark:  "ui-icons/idea/expui/general/settings_dark.svg",
    has_dark: true,
};

// mod.rs 里的 helper：挑明暗 + 设尺寸与前景色（调用点什么都不用管）
pub fn idea_icon_svg(icon: &idea::IdeaIcon, cx: &App) -> Svg {
    let theme = Theme::global(cx);
    let path: SharedString = icon.path(theme.is_dark()).into();
    svg().path(path).flex_shrink_0().text_color(theme.foreground).size_4()
}

// ── 4. 调用点 ─────────────────────────────────────────────────────────────
// crates/settings/src/dialog.rs 的 header()
use lithe_gpui_shared::icons::{idea, idea_icon_svg};

h_flex()
    .text_color(cx.theme().foreground)
    .child(idea_icon_svg(&idea::GEAR_ICON, cx))   // 原来是 Icon::new(IconName::Settings).size_4()
    .child(div().child(tr("lithe.workbench.settings")))
```

四条"不要踩"的硬事实（都有源码出处，理由即取舍）：

1. **`with_assets` 只能注册一次**：签名 `impl AssetSource`，第二次调用是**覆盖**不是叠加
   （`gpui-pre-0.3.6/src/app.rs:198-206`）。所以 `AllAssets` 与我们的资源必须由包装层组合。
2. **`AllAssets::load` 未命中返回 `Err` 而不是 `Ok(None)`**
   （`gpui-kit-assets-0.6.6/src/native_assets.rs:27-29`）。回落链里**不能用 `?` 透传**，
   否则"查一个不存在的资源"会变成硬错误；正确做法是把 `Err` 折叠成 `Ok(None)`。
   （`list` 的未命中约定相反，是 `Ok(vec![])`，可以透传。）
3. **`svg()` 从 `gpui_kit` 根就能拿到**，不需要直接依赖 `gpui-pre`：
   `gpui-kit-0.6.6/src/lib.rs:95` 是 `pub use ::gpui::*`，而 `gpui-pre-0.3.6/src/gpui.rs:103`
   是 `pub use elements::*`，`svg()` / `Svg` 就在 `elements/svg.rs:27,40`。
   `gpui/` 的 feature crate 只依赖 `gpui-kit`，本条让它们不必新增依赖。
4. **`svg().path(..)` 走 alpha mask，SVG 里的 `fill` 会被丢掉**：
   `Window::paint_svg` 把它渲成 `MonochromeSprite`（`gpui-pre-0.3.6/src/window.rs:4858-4866`），
   `render_alpha_mask` 只取 `pixel.alpha()`（`svg_renderer.rs:231-249`），
   所以**实际颜色来自元素的 `text_color`**。这既是"helper 自己设 `text_color`、不让调用点猜"的理由，
   也是"为什么仍要按主题换路径"的理由 —— 78 对明暗变体里有 **12 对几何形状也不同**（实测），
   纯换色在 gpui 这条路径上无效。

### 7.1.1 裸 `svg().path(..)` 直接放进 `h_flex` 是**白板**（实测踩过，两次都是白板）

这是本次实现里最费时间的一个坑，单独记下来。`gpui::svg().path(..)` 本身是能画的，
但**必须同时满足两个条件**，缺任何一个都是"什么都不显示"，而且**不报错、不 panic**：

| 缺什么 | 后果 | 证据 |
| --- | --- | --- |
| `text_color` | **整段不画**：`Svg::paint` 的三条绘制分支全被 `style.text.color` 的 `Option` 挡住，为 `None` 时直接跳过 | `gpui-pre-0.3.6/src/elements/svg.rs:149-186` |
| 显式尺寸 + `flex_shrink_0` | **塌成 0 宽**：`Svg` 无固有尺寸，flex 子项默认 `flex-shrink: 1` + `basis: auto` 把它压到 0 | `svg.rs:84-98`（`request_layout` 只转交 style）；`gpui-component-0.6.6/src/icon.rs:178` 的 `.flex_shrink_0()` 就是同一个坑的官方解法 |

实测过程（这台机器，125% DPI，对话框头部 16×16 图标位置的"亮像素"计数）：

| 写法 | 该位置前景像素 |
| --- | --- |
| 改动前的 `Icon::new(IconName::Settings)`（Lucide，基准） | 142 |
| `svg().path(..).size_4()`（只给尺寸，父容器已有 `text_color`） | **0** |
| `div().w(16).h(16).child(svg().path(..))` / 加 `text_color` / 加红底 | **0 / 0 / 只有红底** |
| `svg().path(..).flex_shrink_0().text_color(..).size_4()`（现在的 helper） | **118** |

结论：**helper 必须把这三件事一起做掉**，所以 `idea_icon_svg` 返回已经设好
`flex_shrink_0()` + `text_color(theme.foreground)` + `size_4()` 的 `Svg`，
调用点**不要**再自己 `.size_4()`（重复设不会坏，但会让"谁负责尺寸"变得含糊）。
同理，返回类型必须是具体的 `Svg` 而不是 `impl IntoElement` —— 后者上 `.size_4()`
会报 `E0599: no method named size_4 found for opaque type impl IntoElement`（实测）。

### 7.2 清单怎么生成、light/dark 怎么配对

生成器：`gpui/tools/generate-idea-icons.mjs`（Node，与 `extract-locale.mjs` 同一风格）。
产物：`gpui/crates/shared/src/icons/idea.rs`，文件头写明 **AUTO-GENERATED，勿手改** 与再生成命令。

```powershell
node gpui/tools/generate-idea-icons.mjs          # 重新生成
node gpui/tools/generate-idea-icons.mjs --check  # 只校验产物与文件系统一致（退出码 0）
```

三个口径：

- **真源是文件系统**，不是那份 TS：脚本递归扫 `gpui/assets/ui-icons/idea/**/*.svg`，**每张 SVG
  都必须出现在产物里**（有"无孤儿"与"无悬空引用"两条校验）。
  `windows/tauri/scripts/idea-icon-mappings.json` **只读**、且**可选**（缺失只降级为"由文件名推导
  常量名"），用来取旧前端的显示名。
- **配对规则**：`foo.svg` 的深色变体是同目录的 `foo_dark.svg`（IntelliJ expui 的约定，
  与旧前端 `generate-idea-icons.ts:95` 同一条）。`*_dark.svg` **不是独立图标**，只是变体文件。
- **缺 dark 变体**：`dark` 回落到 `light`，并把 `has_dark` 置 `false`，让调用方与 `--check`
  能区分"真变体"与"兜底"（旧前端 `:131` 也是这么兜的，但类型上看不出来）。
  当前 157 个文件 = **79 个图标** + 78 个深色变体；79 个图标里 **78 个有真 dark 变体**，
  唯一没有的是 `fileTypes/text.svg`。

**与旧前端 TS 的对应关系（重要）**：`idea-assets.generated.ts` 有 **95 个显示名**，
但它们只对应 **79 张 SVG** —— 13 个路径被多个显示名共用
（`CaretDownIcon`/`ChevronDownIcon` 都指 `chevronDown.svg`；`GearIcon`/`GearSixIcon` 都指
`settings.svg`；`PenIcon`/`PencilIcon`/`PencilSimpleIcon`/`PencilSimpleLineIcon` 四个都指 `edit.svg`）。
本模块为每张 SVG 出一个规范常量，其余 **16 个别名**全部保留在 `idea::ALIASES`，
所以 **95 个显示名一个都没丢**。规范名取字典序最小的显示名（稳定、可复现）。

### 7.3 已接的真实图标（本次唯一改动点）

| 项 | 值 |
| --- | --- |
| 位置 | `gpui/crates/settings/src/dialog.rs` 的 `header()` —— 设置对话框头部左侧齿轮 |
| 之前 | `Icon::new(IconName::Settings)`（Lucide `settings.svg`） |
| 之后 | `idea_icon_svg(&idea::GEAR_ICON, cx)` |
| 浅色资源 | `ui-icons/idea/expui/general/settings.svg` |
| 深色资源 | `ui-icons/idea/expui/general/settings_dark.svg` |
| 真源对照 | `settings-dialog.tsx:36` 的 `GearSixIcon` → 同一张 `settings.svg`（第 3.8 节已核） |

实测证据（125% DPI，窗口 1823×1024 物理像素，设置对话框头部齿轮所在的 24×24 物理方块）：

| 指标 | before（Lucide `IconName::Settings`） | after（expui `GEAR_ICON`） |
| --- | --- | --- |
| 方块内前景像素（亮度 ≥ 100） | 154 | 132 |
| 齿轮 20×20 区域前景像素 | 142（包围盒 x 416..433） | 118（包围盒 x 417..432） |
| 两者逐像素差异 | —— | **186**，包围盒 `x 416..433, y 138..157`（**只**在齿轮这一小块） |

差异包围盒就是齿轮本身 —— 证明改动**只**影响这一个图标，其余头部像素（标题 / 关闭按钮）逐字节相同。
叠图（上 = before，下 = after，头部最左 320 物理像素 ×3）：`.artifacts/idea-icons/before-after-header.png`。

选它的理由：它是 `ui-icons/idea/**` 里**已存在 1:1 真源**的一处（第 5 节的 14 个之一）、改动只有
一行、可一行回退，而且它在设置对话框里，`--open-settings` 就能稳定复现（本机工作站锁屏，
只能用鼠标/参数驱动，不能靠按键）。

**回落没坏的证据**：同一帧里活动栏与对话框仍然用 `IconName`（Lucide）画图标 ——
设置对话框自己的关闭按钮 `IconName::Close`、左栏分类的 `IconName::Settings`
（`Category::General` 仍在用 Lucide，**本次有意没改**，正好当对照组）、活动栏的
`FolderOpen` / `GitBranch` / `Search` / `SquareTerminal` / `GitGraph` / `TriangleAlert` 等。
同一张"after"截图上的量化结果：

| 区域（同一帧） | 用的资源源 | 前景像素 | 结论 |
| --- | --- | --- | --- |
| 对话框头部齿轮 | `LitheAssets` 内嵌的 `ui-icons/idea/**` | 118 | 真源图标画出来了 |
| 对话框左栏两个分类图标（含 `IconName::Settings`） | `AllAssets` 回落（Lucide `icons/**`） | 371 | 回落链路通 |
| 左活动栏整列 | `AllAssets` 回落（Lucide `icons/**`） | 994 | 回落链路通 |

启动诊断（可 grep，证明包装层被注册且两侧都能列出资源）：

```
S1_ASSETS embedded=1204 ui_icons=158 fallback_icons=1830
S1_ASSETS probe path=ui-icons/idea/expui/general/settings.svg bytes=2914
S1_ASSETS probe path=ui-icons/idea/expui/general/settings_dark.svg bytes=2914
S1_ASSETS probe path=icons/settings.svg bytes=586
```

`embedded` 来自 `LitheAssets::embedded_count()`（编译期静态表，1 204 个），
`ui_icons` 来自 `embedded_count_under("ui-icons/")`（158 个），
`fallback_icons` 来自 `AllAssets.list("")`（1830 个）。
后三条 `probe` 是**真的走了一遍 `AssetSource::load`**：它们证明"路径键对、能取到字节"。
⚠️ 只打印"能 list 出来"是不够的 —— 实测就出现过"名字对、`load` 也返回 2 914 字节，
但界面是白板"的情况（原因见 7.1.1，是渲染层缺 `text_color`/尺寸，与资源源无关）。
⚠️ 这四条都走 **stderr**（`eprintln!`）：stdout 重定向到文件时是块缓冲，
进程还在跑时日志里可能一行都看不到（本机实测同一二进制同一参数，出现过 3 行也出现过 0 行）。

**二进制体积影响**：`gpui/target/debug/Lithe.exe` 从 81 815 040 字节 → 81 906 176 字节，
**+91 136 字节（+0.09 MiB）**。比 8.2 MiB 的原始资源总和小得多，因为 debug 构建里
`rust-embed` 的默认非压缩分支只把编译期静态表链进来、字节留给 rustc 的 section GC。
（release 构建未实测；若届时体积不可接受，`#[include]` 是唯一需要改的地方。）

### 7.4 明确**还没接**的部分

| 没做的 | 为什么 / 需要什么 |
| --- | --- |
| `gpui/assets/icon-themes/**`（4 套文件类型图标包，1 027 个 SVG） | 它不是"一个名字一张图"，而是"按 `fileNames` / `fileExtensions` / `folderNames` 查各包 `extension.json` 的 `iconDefinitions`，再选 `icons/light/` 变体"的另一套体系。需要：① 一个主题 id → `extension.json` → SVG 字节的**查找层**（`ThemedFileIcon` 的等价物）；② `explorer` / `editor` 的 `icon_for_file` 从"返回 `IconName`"改成"返回主题资源路径"（第 3.6/3.7 节）。注意 `material` 主题**没有 SVG 文件**（美术内联在 `extension.json` 里）、`minimal` 主题全仓无引用。 |
| 第 3 节表里其余"可以"的 `expui` 站点 | 只是**没接线**，不是不能接：`workspace.rs` 的活动栏、`status_bar.rs`、`project_tabs.rs`、`right_tool_window.rs`、`command_palette.rs`、`explorer_view.rs`、`git/log_view.rs`、`terminal_view.rs`、`editor_view.rs` 都还在用 `IconName`。逐个换的成本很低（一行一处），但每换一处都要在**同帧**保留至少一个 `IconName` 图标，才能继续证明回落链路没坏。 |
| `MavenIcon` / `RunIcon`（第 4 节） | 仍然只有内联 SVG path、没有文件。第 4 节给的两条路（落成 SVG 文件 / 当 Rust 常量自己画）都还没做。 |
| `.ico` / 窗口图标接线 | 与本节无关：`AssetSource` **不参与**窗口/exe 图标（见 `app-icon-and-assets.md` §4.1）。`crates/app/lithe.rc` + `build.rs` 那套已在别处完成。 |

### 7.5 本次动过的文件（供 review）

| 文件 | 性质 |
| --- | --- |
| `gpui/tools/generate-idea-icons.mjs` | 新增：生成器 + `--check` |
| `gpui/crates/shared/src/icons/idea.rs` | 新增：**生成物**（79 图标 + 16 别名） |
| `gpui/crates/shared/src/icons/mod.rs` | 新增：模块文档 + `idea_icon_svg` helper |
| `gpui/crates/shared/src/lib.rs` | `pub mod icons;` |
| `gpui/crates/app/src/assets.rs` | 新增：`LitheAssets`（`rust-embed` + 回落） |
| `gpui/crates/app/src/main.rs` | `mod assets;`、换注册、加 `S1_ASSETS` 诊断 |
| `gpui/crates/app/Cargo.toml` | 加 `rust-embed = "8"` |
| `gpui/crates/settings/src/dialog.rs` | 头部齿轮换成真源图标（唯一调用点改动） |
| 本节（`gpui/research/icon-asset-inventory.md`） | 文档 |

`gpui/assets/**` 的**资源内容一个字节都没改**（只读）。
