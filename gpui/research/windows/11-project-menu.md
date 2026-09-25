# 11 · 标题栏项目下拉（Windows 真源 → gpui 落地研究）

研究范围：Windows 前端 `windows/tauri/src/`（**只读真源**）里「标题栏项目名下拉面板」的完整规格，
以及它在 `gpui/` 侧（`gpui/crates/workbench/`）的落地代价。

- 分支：`feat/gpui-shell-rewrite`
- 本文**不改任何代码**，所有 gpui 侧结论都给 `文件:行号` 证据。
- 已有实现（本文不重复）：标题栏本体在 `gpui/crates/workbench/src/title_bar.rs`（模块头已登记
  Windows 的三键/度量），项目标签条在 `gpui/crates/workbench/src/project_tabs.rs`。
- 组件文档只引 `gpui/docs/gpui-kit/0.6.6/zh-CN/component/`（`menu.md` / `popover.md` / `dropdown_button.md`）。
- ⚠️ **gpui-component 源码不在本仓库内**，在 cargo registry：
  `CMP = D:/ProgramData/rust/cargo/registry/src/rsproxy.cn-e3de039b2554c837/gpui-component-0.6.6`。
  本仓库既有笔记引用它时写作 `gpui-component-0.6.6/src/...`（例：`gpui/research/windows/09-tab-context-menu.md:459`），
  下文沿用这个写法。同理 `gpre = gpui-pre-0.3.6`、`gbase = gpui-base-0.6.6`。

---

## 0. 一句话结论

| 问题 | 结论 |
| --- | --- |
| 面板分几段 | **3 段 + 2 条分隔线**：①3 个动作行 ②分组「打开的项目」 ③分组「最近项目」 |
| 项数 | 动作 3 项；「打开的项目」= 当前打开的标签数 N；「最近项目」= 去重后 ≤ **12** 项（空则显示一行占位文案） |
| 触发器 | `Button ghost/xs`，图标是 **`/logo.png` 应用 logo（20×20）**，不是项目徽标；`max-w-56`(224)、文字 `truncate`、右侧 chevron 14×14，展开时旋转 180° |
| 项目图标 | 两条分支：有 `customIcon` 就渲染该图片；否则用**首字母徽标** —— 取名字前 2 个词的首字符（大写，兜底 `LI`），配色 = `hash(name) % 5` 命中 5 色板（`title-project-menu-model.ts:6-31`） |
| 路径省略 | 面板内每行是 `truncate`（**尾部省略号**），**不做中间省略**、无 `title` 悬停提示；面板 `w-96`(384) 且 `max-w-[calc(100vw-1rem)]` |
| 「打开的项目」存哪 | `useWorkspaceTabsStore.projectTabs` → **localStorage**，键按 webview 窗口 label 分开（`workspace-tabs.store.ts:51,186`），`version 1` |
| 「最近项目」存哪 | `useRecentFoldersStore.recentFolders` → **localStorage**，键 `lithe-code-recent-folders`、`version 2`（`recent-folders.store.ts:191-194`）；**不经过 Rust Core** |
| 三个动作依赖 | 新建项目 / 克隆仓库 → 打开 `ProjectPicker`（`ui-state.store` 的模态，**不是** Core 命令）；打开… → **平台目录选择器** `openFolder()`（`file-system.store.ts:811`）。**三者都不置灰**，只有「打开的项目」行会在切换中禁用 |
| 「克隆」有没有 git 前置条件 | **没有**。`new-project-content.tsx` 只校验「目标目录不存在」+「仓库 URL 非空」（`:172-176,246-248`），真正的 clone 走 Tauri `git_clone` → Core `git.write`（`src-tauri/src/platform.rs:563-579`） |
| 当前项目判定 | `projectTabs.find(p => p.isActive)`（`title-project-menu.tsx:112`），`isActive` 由 store 的 `addProjectTab`/`setActiveProjectTab` 维护；与 `project.store` 的 `activeProjectId` 通过 `setActiveProjectId(workspaceId)` 同步（`file-system.store.ts:661,2984`） |
| gpui 第一版可做 | **面板外观 + 当前项目行 + 空态**（`dropdown_menu_with_anchor(Anchor::TopLeft, ..)` + `PopupMenu` + `PopupMenuItem::element` 自绘行）；文案键 **零新增**（`lithe.zh-CN.yml:3538-3579` 已在） |
| gpui 第一版做不了 | 最近项目的**来源**、切换项目、打开文件夹对话框、新建/克隆 —— 四项都需要新状态或新平台能力（§5.2） |
| 没能确认的点 | 见 §7（共 7 条），其中第 1 条（触发器图标到底是 logo 还是项目徽标）与 §1.2 的截图描述有出入，需要维护者确认 |

---

## 1. 真源：结构、逐项规格

### 1.1 位置与调用链

| 角色 | 位置 |
| --- | --- |
| 面板本体 | `windows/tauri/src/features/window/components/title-bar/title-project-menu.tsx`（全文 239 行） |
| 数据折算（纯函数，有单测） | `windows/tauri/src/features/window/utils/title-project-menu-model.ts:1-51` |
| 单测 | `windows/tauri/src/features/window/utils/title-project-menu-model.test.ts:24-81` |
| 谁渲染它 | `windows/tauri/src/features/window/components/title-bar/title-bar.tsx:235-240`（`projectControls`）、`:340-343`（Windows 分支的左组） |
| 所属容器 | `ChromeGroup gap="tight"` = **2px**（`windows/tauri/src/ui/chrome.tsx:39`、`styles/theme.css:129`） |
| 面板底层的 UI 原语 | `windows/tauri/src/ui/dropdown.tsx:1`（`import { Menu as DropdownMenuPrimitive } from "@base-ui/react/menu"`），包装见同文件 `:713-938` |
| 触发器按钮样式 | `windows/tauri/src/ui/button.tsx:8-36` |
| 三个动作落到哪 | `windows/tauri/src/features/window/components/project-picker.tsx`（模态）+ `project-picker-mode.ts:1-12` |

**它不是** `window-menu-bar.tsx` 那一套（那是顶部菜单栏，`title-bar.tsx:231`），两者只是并排。

### 1.2 触发器（宽度 / 截断 / 图标 / 箭头）

`title-project-menu.tsx:139-164`：

| 元素 | 源码行 | 规格 |
| --- | --- | --- |
| `DropdownMenuTrigger` 的 `render` | `:140-149` | 把 `Button` 当触发器本体（base-ui 的 `render` 合并 props） |
| `variant="ghost"` | `:144` | `bg-transparent text-subtle-foreground hover:bg-accent hover:text-foreground`（`ui/button.tsx:16-17`） |
| `size="xs"` | `:145` | `h-6`（**24**）`gap-1 px-1.5`（`ui/button.tsx:23`） |
| `className="max-w-56 justify-start gap-1.5 px-2"` | `:146` | `max-w` **224**、`gap` **6**、`px` **8**（后写覆盖 `size` 的 `gap-1`/`px-1.5`，`cn` = `twMerge`，`utils/cn.ts:1-6`） |
| `aria-label` | `:147` | `t("titleProject.trigger", { project })` = 「项目：{project}」（`i18n/locale.ts:6191`） |
| 图标容器 | `:151-156` | `grid size-5 shrink-0 place-items-center overflow-hidden rounded-md` = **20×20**、圆角 `8×0.8`=6.4、裁切 |
| 图标本体 | `:155` | `<img src="/logo.png" className="size-5 scale-[1.19] object-contain" />` —— **应用 logo**（`windows/tauri/public/logo.png` 存在），放大 1.19 后仍被 20×20 容器裁掉溢出 |
| 文字 | `:157` | `<span className="min-w-0 truncate">` —— **单行尾部省略**，无 `title` |
| 箭头 | `:158-163` | `ChevronDownIcon className="size-3.5 shrink-0 text-subtle-foreground transition-transform"` = **14×14**，`isOpen && "rotate-180"` |
| 字号 | — | `buttonVariants` 基类带 `ui-text-sm`（`ui/button.tsx:9`）→ `--ui-text-sm` = `--app-ui-font-size` = **13px**（`styles/theme.css:112,116`） |
| 悬停/焦点 | — | `hover:bg-accent hover:text-foreground`（ghost）、`focus-visible:ring-2 focus-visible:ring-primary/20`、`active:scale-(--app-press-scale)`（值为 1，`theme.css:139`，实测无缩放） |

> ⚠️ **与截图描述的出入**：维护者描述触发器带「项目图标」，但真源 `:155` 写死的是 `/logo.png`（Lithe logo），
> 项目徽标只出现在**面板内每一行**（`:99`）。见 §7 第 1 条。

### 1.3 面板容器

`title-project-menu.tsx:166-171`（配合 `ui/dropdown.tsx:731-761`）：

| 维度 | 值 | 出处 |
| --- | --- | --- |
| 宽度 | `w-96` = **384** | `:170` |
| 宽度上限 | `max-w-[calc(100vw-1rem)]` = 视口宽 − 16 | `:170` |
| 高度上限 | `max-h-[min(32.5rem,calc(100vh-3rem))]` = min(**520**, 视口高 − 48) | `:170` |
| 内边距 | `p-1.5` = **6**（覆盖基类 `p-1`=4） | `:170`；基类 `ui/dropdown.tsx:753` |
| 圆角 | `rounded-md` = `calc(var(--radius) × 0.8)` = 8 × 0.8 = **6.4** | `:170`；`styles/theme.css:7,134` |
| 底色 | `bg-surface`（基类） | `ui/dropdown.tsx:753` |
| 边框/阴影 | `ring-1 ring-border/70` + `shadow-(--shadow-popover)` | `ui/dropdown.tsx:753`；`theme.css:160-162` |
| 滚动 | `overflow-y-auto` | `:170` |
| 定位 | `align="start"`、`side="bottom"`、`sideOffset` 默认 **4**、`collisionPadding` **8** | `:168-169`；`ui/dropdown.tsx:735-737` |
| 层号 | `z-10070` | `ui/dropdown.tsx:748,753` |
| 最小宽度 | `min-w-44` = 176（基类，被 `w-96` 覆盖） | `ui/dropdown.tsx:753` |
| 滚轮 | `bindScrollContainerWheel(menuNode)` —— **WebView2 专用兜底**：把 wheel 增量直接写进容器的 `scrollTop`（`ui/scroll-container-wheel.ts:82-92`，注释理由 `:4-5`） | `:128-131` |

### 1.4 三段逐条规格（顺序 / 中文文案 / i18n 键 / 图标 / 禁用）

i18n 键值真源：`windows/tauri/src/i18n/locale.ts`，中文 `:6171-6191`、英文 `:1757-1778`。

**段 1 —— 动作（3 项，行高 32）**

| # | 源码行 | 中文文案 | i18n 键 | 图标 | 动作 | 禁用 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `:172-178` | 新建项目… | `titleProject.newProject`（`locale.ts:6171`） | `PlusIcon` | `closeAndRun(() => onOpenProjectPicker("new-project"))` | 恒可点 |
| 2 | `:179-185` | 打开… | `titleProject.open`（`:6174`） | `FolderOpenIcon` | `closeAndRun(() => void handleOpenFolder())` | 恒可点 |
| 3 | `:186-192` | 克隆仓库… | `titleProject.cloneRepository`（`:6186`） | `GitBranchIcon` | `closeAndRun(() => onOpenProjectPicker("clone-repository"))` | 恒可点 |

这三行的 class 一律 `min-h-8 justify-start gap-2 rounded-md px-2`（`:174,181,188`）→ 行高 **32**、
内距 **8**、行内 gap **8**、圆角 6.4。图标尺寸由 `DropdownMenuItem` 基类
`[&_svg:not([class*='size-'])]:size-4` 决定 = **16×16**（`ui/dropdown.tsx:782`；`icons` 默认 `size="1em"`，
`ui/icons.tsx:154`，所以实际尺寸只由这条 CSS 定）。

**分隔线 1**：`:194`（`DropdownMenuSeparator`，`ui/dropdown.tsx:863-871`：`-mx-1 my-1 h-px bg-border`，
左右各外扩 4 超越 `p-1.5` 的 6 —— 即离面板边 2px）。

**段 2 —— 分组「打开的项目」**（`:195-214`）

| 元素 | 源码行 | 规格 |
| --- | --- | --- |
| 分组标题 | `:196-198` | `DropdownMenuLabel className="px-2 pt-1.5 pb-1 font-normal ui-text-xs"`；文案 `titleProject.openProjects` = **「打开的项目」**（`locale.ts:6187`） |
| 行 | `:199-213` → `ProjectMenuRow` `:72-107` | 见 §1.5 |
| 当前项 trailing | `:211` | `project.isActive ? <CheckIcon className="size-4 text-primary" /> : null` → **16×16、`--primary` 色勾**（不是 `DropdownMenuCheckboxItem`） |
| 禁用 | `:206` | `disabled={isSwitchingProject}` —— **只在这一组**，切换进行中整组置灰 |
| 点击当前项 | `:207-210` | `if (project.isActive) return;` → **no-op，面板不关** |
| 点击其它项 | `:209` | `closeAndRun(() => void switchToProject(project.id))` |
| key | `:201` | `project.id`（= `createProjectTabId(path)`） |

**分隔线 2**：`:216`。

**段 3 —— 分组「最近项目」**（`:217-235`）

| 元素 | 源码行 | 规格 |
| --- | --- | --- |
| 分组标题 | `:218-220` | 同段 2 的样式；文案 `titleProject.recentProjects` = **「最近项目」**（`locale.ts:6189`） |
| 空态 | `:221-225` | `projects.recentProjects.length === 0` 时一行 `div.px-2.py-3.text-subtle-foreground.ui-text-xs`，文案 `titleProject.noRecentProjects` = **「没有最近项目」**（`locale.ts:6190`） |
| 行 | `:226-234` | `ProjectMenuRow`，`name={folder.name}`、`path={folder.path}`、`iconPath={folder.customIcon}`；**无 `active`、无 `trailing`、无 `disabled`** |
| 点击 | `:232` | `closeAndRun(() => void openRecentFolder(folder.path))` |
| key | `:228` | `folder.path` |

**合计**：`3 + 2（分组标题）+ N（打开的项目）+ M（最近项目，≤12）` 行；`N === 0` 时段 2 **不会**
出现空态占位（`:199` 直接 map 空数组，只有段 3 有空态）。

### 1.5 行内度量（`ProjectMenuRow`，两段共用）

`title-project-menu.tsx:72-107`：

| 元素 | 源码行 | 规格 |
| --- | --- | --- |
| 行容器 | `:90-97` | `DropdownMenuItem` + `min-h-11 w-full items-center gap-2.5 rounded-md px-2 py-1.5` → min-h **44**、gap **10**、px **8**、py **6**、圆角 6.4 |
| 当前项底色 | `:96` | `active && "bg-selected text-foreground"` |
| `aria-current` | `:91` | `getTitleProjectMenuItemAriaCurrent(active)` → 只为当前项输出 `"true"`（`title-project-menu-model.ts:33-35`） |
| 徽标 | `:99` | `<ProjectBadge name iconPath className="size-7" />` → **28×28** |
| 文字块 | `:100-103` | `span.min-w-0.flex-1.text-left`，内含两行 |
| 第 1 行（名） | `:101` | `block truncate font-medium text-foreground ui-text-sm` —— **13px 中等字重** |
| 第 2 行（路径） | `:102` | `block truncate text-subtle-foreground ui-text-xs` |
| trailing | `:104` | 由调用方给（只有当前项给勾） |
| 悬停/高亮 | — | 来自 `DropdownMenuItem` 基类 `focus:bg-accent focus:text-foreground`（`ui/dropdown.tsx:782`） |
| 行高换算 | — | min-h 44 = `py-1.5`×2(12) + 徽标 28 → **内容撑高，不是固定高** |

> ⚠️ **`ui-text-xs` 是个未定义的类**。`windows/tauri/src/styles/utilities.css:30-44` 只定义了
> `.ui-text-sm` / `.ui-text-caption` / `.ui-text-chrome` / `.ui-text-base`；全仓库 `.css` 里没有
> `.ui-text-xs`（grep 无命中），Tailwind v4 也不会把它当内置工具类（`styles.css:10` 只 `@import "tailwindcss"`）。
> 所以**路径行 / 分组标题 / 空态文案在真机上实际都继承父级字号** ——
> 父级是 `DropdownMenuContent` 的 `ui-text-sm`（`ui/dropdown.tsx:753`）+ `DropdownMenuItem` 的
> `ui-text-sm`（`:35`），即 **13px，不是 12px**。
> 落地时要在「照意图做 12px」和「照实测做 13px」之间拍板，见 §7 第 2 条。

---

## 2. 项目图标（彩色圆角方块）怎么来

### 2.1 两条分支

`title-project-menu.tsx:38-70`（`ProjectBadge`）：

| 条件 | 行为 | 源码行 |
| --- | --- | --- |
| `iconPath` 有值 | `<img src={convertFileSrc(iconPath)} className="shrink-0 rounded-md object-contain size-7" />` —— 真实图片 | `:47-55` |
| 无 `iconPath` | 首字母徽标 `<span className="grid shrink-0 place-items-center rounded-md font-bold text-[10px] text-white {tone} size-7">` | `:57-69` |

`iconPath` 来源是 `ProjectTab.customIcon`（`workspace-tabs.store.ts:27`，由 `setProjectIcon` `:154-161` 写）
或 `RecentFolder.customIcon`（`recent-folders.types.ts:7`）。它是**项目里扫出来的图片文件绝对路径**：
`project-icons.ts:65-108` 从项目根向下最多 4 层扫描 `.{ico,png,svg}` 且文件名匹配
`app-icon|apple-touch-icon|favicon|icon|logo`（`:29-32`），跳过 `.git`/`node_modules`/`dist`/`target` 等
（`:11-23`），按打分排序（`:34-63`：`src-tauri/icons/` +120、`public/` +90、`icon.png` +70、
`favicon.ico` +65、`logo.*` +55、`mask*` −25、`docs|test(s)` −60…）。
用户也可以在 `project-icon-picker.tsx` 里手动选（`:39-52`）。

### 2.2 首字母徽标算法

`title-project-menu-model.ts:14-31`：

```ts
const words = name.split(/[^\p{L}\p{N}]+/u).filter(Boolean);      // :15  按「非字母数字」切词
const initials = words.slice(0, 2)                                 // :16  只取前 2 个词
  .map((word) => Array.from(word)[0] ?? "")                        // :19  每词取第 1 个「码点」
  .join("").toUpperCase() || "LI";                                 // :20-21 大写；空则兜底 "LI"
const hash = Array.from(name).reduce(                              // :22-25
  (value, character) => (value * 31 + (character.codePointAt(0) ?? 0)) & 0x7fffffff, 0);
return { initials, tone: PROJECT_BADGE_TONES[hash % PROJECT_BADGE_TONES.length] ?? ... };  // :27-30
```

要点（逐条可核对）：

1. **是首字母，不是首字符**：多词名给 **2 个字母**，单词名给 1 个。
2. **按 Unicode 码点取**（`Array.from`），所以 emoji / 中文都安全；`"文档项目"` → `"文"`（单测 `:80`）。
3. **哈希是 Java 式 31 进制**，种子 0，每步 `& 0x7fffffff` 截成 31 位正整数；**同一个名字永远同一色**。
4. **配色只由 `hash % 5` 决定**，与「项目类型 / 语言 / git 状态」**无关**，也没有图标主题机制
   （主题只影响 `--primary` 等 UI 色，不影响这 5 个常量）。
5. 用 node 复算验证（对照截图，2/3 命中）：
   `jzwg` → `J` + `bg-sky-600`(蓝) ✅；`qzjk` → `Q` + `bg-violet-600`(紫) ✅；
   `Lithe-IDEA` → `LI` + `bg-emerald-600`(绿)。
   截图里的 `htodsdatams-…` 是绿色 `HO`：**首字母 `HO` 说明名字是「前两个词首字母」**，绿色 = 下标 1；
   具体那个目录名没有在仓库里出现，无法逐字复核（见 §7 第 3 条）。

### 2.3 色板

`title-project-menu-model.ts:6-12`（顺序即下标）：

| 下标 | 类名 | Tailwind v4 默认色值 | 截图对应 |
| --- | --- | --- | --- |
| 0 | `bg-sky-600` | `#0284c7` | `jzwg` = 蓝 ✅（已复算） |
| 1 | `bg-emerald-600` | `#059669` | `htodsdatams-…` = 绿（截图，见 §2.2-5） |
| 2 | `bg-orange-600` | `#ea580c` | — |
| 3 | `bg-violet-600` | `#7c3aed` | `qzjk` = 紫 ✅（已复算） |
| 4 | `bg-rose-600` | `#e11d48` | — |

文字固定 `text-white`、`font-bold`、`text-[10px]`（`:62`），容器圆角 `rounded-md` = 6.4（`:62`）。

### 2.4 路径省略规则

- 面板内**每一行**的路径是 `block truncate`（`:102`）：`overflow:hidden` + `white-space:nowrap` +
  `text-overflow:ellipsis` → **尾部省略**，**不做中间省略**，也不保留尾部目录名。
- 两行都在 `span.min-w-0.flex-1`（`:100`）里 —— `min-w-0` 是 flex 子项能被压缩的前提；
  右侧 trailing 勾（`:104`）是 `shrink-0`，所以可用宽度 = 384 − 12(panel padding) − 16(px) − 28(徽标)
  − 10(gap) − 16(勾) ≈ **302px**。
- **没有 `title` 属性**，所以悬停也看不到完整路径。
- 触发器上的项目名同样是 `min-w-0 truncate`（`:157`），上限由 `max-w-56`=224 兜住（`:146`）。

---

## 3. 数据来源

### 3.1 「打开的项目」= `projectTabs`

| 项 | 事实 | 出处 |
| --- | --- | --- |
| 存储 | `useWorkspaceTabsStore`，`projectTabs: ProjectTab[]` | `windows/tauri/src/features/window/stores/workspace-tabs.store.ts:33-35` |
| 字段 | `id` / `name` / `path` / `isActive` / `lastOpened` / `customIcon?` / `displayAlias?` / `theme?` | `:21-31` |
| 持久化 | zustand `persist`，键 `workspaceTabsStorageKey`，**按 webview 窗口 label 分开**（`"main"` / 多窗口各自一份） | `:51`、`:186`；键生成 `utils/workspace-tabs-storage.ts` |
| 后端 | `createSafeJSONStorage` → **localStorage**（带 JSON 解析兜底） | `:187`；`utils/zustand-storage.ts` |
| 版本 | `version: 1` | `:194` |
| 写入时机 | `addProjectTab(path, name, theme)`：先按路径去重（`areProjectTabPathsEqual`），命中就把**已有标签置为 active** 并刷新 name/path；否则**先把所有标签 `isActive=false`**，再 push 一个 `isActive=true`、`lastOpened=Date.now()` 的新标签 | `:77-111` |
| 切换 | `setActiveProjectTab(id)`：只改 `isActive`，并给命中的标签刷新 `lastOpened` | `:119-128` |
| 排序 | **数组顺序 = 标签条顺序 = 插入顺序**（新项目追加在尾部）；`reorderProjectTabs` 只由标签条拖拽调用（`utils/project-tab-order.ts`），标题栏面板**不排序、不过滤** | `:130-134`；`title-project-menu-model.ts:43` |
| 上限 | **没有上限**（打开多少就多少） | — |
| 关闭 | `removeProjectTab(id)` 只做数组过滤（`state.projectTabs = removeProjectTabItems(...)`）——**它不决定"关了当前项之后谁接管"**；那套逻辑在 `switchToNextAvailableProjectAfterClose`（`file-system/controllers/workspace-project-tabs.ts:13-44`：逐个尝试切换，全失败才清空 + `resetWorkspace`） | `:113-117`、`:115` |
| 显示名 | `getProjectDisplayLabel(project)` = `name` + 可选 `" (alias)"`；别名由 `setProjectDisplayAlias` 维护，用于区分同名文件夹 | `title-project-menu.tsx:119,202`；`utils/project-display-label.ts:5-16` |

`id` 是路径哈希：`createProjectTabId(path)` = djb2 变体 `hash = hash*33 ^ charCode`，base36，前缀
`project-`（`utils/project-tab-path.ts:26-35`）。路径比较规则：去掉尾部斜杠，`X:/` 或 `//` 开头时
**大小写不敏感**（`:3-24`）。

### 3.2 「最近项目」= `recentFolders`

| 项 | 事实 | 出处 |
| --- | --- | --- |
| 存储 | `useRecentFoldersStore`，`recentFolders: RecentFolder[]` | `recent-folders.store.ts:34-36` |
| 字段 | `name` / `path` / `lastOpened`(字符串) / `lastOpenedAt?`(数值) / `activeProjectTabId?` / `customIcon?` / `missing?` / `openInNewWindow?` / `pinned?` / `importSourceId?` / `importSourceName?` | `types/recent-folders.types.ts:1-13` |
| 持久化 | zustand `persist` + `immer`，键 **`lithe-code-recent-folders`**、`version: 2`、`createSafeJSONStorage`（localStorage）、`partialize` 只存 `recentFolders` | `:191-199` |
| 迁移 | v2 `migrate` 把旧的 `lastOpened` 字符串解析成 `lastOpenedAt` 数值 | `:200-221` |
| 上限 | `MAX_RECENT_PROJECTS = 12`（**非遗漏项**：pinned 项不计入这 12 个名额） | `utils/recent-folders.ts:3,36-42` |
| 排序 | `pinned` 置顶；其余按 `lastOpenedAt` 降序 | `:26-34` |
| 去重 | `upsertRecentFolder` 先删同 `path` 的旧项，再把新项放在数组头，最后 `limitRecentFolders` | `:64-89` |
| 写入时机 1 | 打开项目成功后：`initializeLocalWorkspace` 里 `addToRecents(path, {activeProjectTabId, customIcon, missing:false})`（`prewarm` 时跳过） | `file-system.store.ts:661-667` |
| 写入时机 2 | 从「最近项目」里重开（新窗口）后 `addToRecents(..., {openInNewWindow:true})` | `recent-folders.store.ts:134-137` |
| 写入时机 3 | 同上但本窗口打开 | `:143-146` |
| 写入时机 4 | WSL 项目 / 追加工作区文件夹 | `file-system.store.ts:1513-1515`、`:1283` |
| 标记失效 | `updateRecentFolder(path, {missing:true})` 在 `getSymlinkInfo` 报错或不是目录时 | `recent-folders.store.ts:107-120` |
| 清理 | `removeMissingFromRecents` 过滤 `missing`；`clearRecents` 清空；两者都不在标题栏面板里可达 | `:161-171` |

**面板拿到的是过滤+截断后的结果**（`title-project-menu-model.ts:37-51`）：

```ts
openProjects: projectTabs,
recentProjects: recentFolders
  .filter(r => !projectTabs.some(p => areProjectTabPathsEqual(p.path, r.path)))   // :45-48
  .slice(0, maxRecentProjects)                                                    // :49（默认 12）
```

也就是：**已打开的项目从「最近项目」里剔除**（路径比较大小写规则同 §3.1），再截 12 条。
单测覆盖了「d:/code/Lithe/」与「D:\code\Lithe」被判为同一个（`title-project-menu-model.test.ts:25-38`）。

### 3.3 三个动作各自调用什么、有没有前置条件

| 动作 | 调用链 | 前置条件 / 置灰 |
| --- | --- | --- |
| **新建项目…** | `onOpenProjectPicker("new-project")` → `ui-state.store` 的 `setIsProjectPickerVisible(true, mode)`（`title-bar.tsx:372-377`）→ 门户渲染 `<ProjectPicker>`（`:393-400`）→ `getProjectPickerInitialState("new-project")` 让面板直接落在「选源」步（`project-picker-mode.ts:3-11`、`project-picker.tsx:141,437-446`）→ 用户选定源后在 `new-project-content.tsx:239-277` 真正创建：`exists(destinationPath)` 为真则报错、否则 `createNewDirectory(...)`（或 clone）→ `handleOpenFolderByPath(destinationPath)` → 关模态 → nextjs/vite 源再开一个终端跑脚手架命令（`:266-271`） | **无置灰**。「新建项目」项本身不校验任何能力 |
| **打开…** | `handleOpenFolder()`（`file-system.store.ts:810-...`）→ 平台目录选择器 **`openFolder()`**（`:811`，`features/file-system/controllers/platform`）→ 用户取消则 `false` → 否则 `chooseProjectOpenDestination({projectName, hasOpenWorkspace})` → 可能弹「在哪打开」（`project-open-destination.ts:54-72`，选项 = 新窗口 / 此窗口 + 「不再询问」）→ `executeProjectOpenDecision`：新窗口走 `createAppWindow`（`file-system.store.ts:829-835`），此窗口走 `openWorkspaceRuntime`（`:837-849`） | **无置灰**；「无工作区」时直接走 `this-window`，不弹窗（`project-open-destination.ts:86-88`） |
| **克隆仓库…** | `onOpenProjectPicker("clone-repository")` → `ProjectPicker` 以 `initialSource="clone"` 落在详情步（`project-picker-mode.ts:10`、`project-picker.tsx:437-446`）→ 校验 `projectName` 合法 + `locationPath` 非空 + **`repositoryUrl` 非空**（`new-project-content.tsx:172-176`）→ 目标目录已存在则报错（`:246-248`）→ `invoke("git_clone", {repositoryUrl, destinationPath})`（`:250-254`）→ Tauri 侧翻译成 Core **`git.write`**（`windows/tauri/src-tauri/src/platform.rs:563-579`）→ `handleOpenFolderByPath(destinationPath)` | **不看「有没有装 git」**。`backend-capabilities.ts:9` 的 `git: true` 是**编译期常量**，标题栏面板**没有**用它；只有 `ProjectPicker` 里的「添加远程」按钮会因 `remote: false` 置灰（`project-picker.tsx:661`）。git 缺失只会在 Core 的 `git.write` 阶段报错 |

**唯一的禁用态**：`isSwitchingProject`（`title-project-menu.tsx:117,206`）为真时「打开的项目」整组置灰。
它由 `setIsSwitchingProject` 维护（`file-system.store.ts:2930-2934`），只在
`switchToProject` 里"目标工作区尚未 ready"时置真（`:2948-2950`），初始化完成即复位（`:2978`）。

### 3.4 当前项目的判定与切换时的状态变化

| 环节 | 事实 | 出处 |
| --- | --- | --- |
| 面板里的当前项目 | `projectTabs.find(p => p.isActive)`；`projectLabel` = `getProjectDisplayLabel(activeProject)`，**没有当前项目时退回 `t("projectOpen.title")`** | `title-project-menu.tsx:112,118-120` |
| `isActive` 谁维护 | `addProjectTab`（新项目置 active，其它全清）/ `setActiveProjectTab`（互斥置位 + 刷 `lastOpened`） | `workspace-tabs.store.ts:97-99,106,119-128` |
| 与 `activeProjectId` 的关系 | 另一条线：`project.store` 的 `activeProjectId` 由 `setActiveProjectId(workspaceId)` 写，发生在工作区初始化/恢复成功之后 | `file-system.store.ts:661`、`:2984` |
| 与 `projectTabs` 的关系 | **一一对应但不是同一份状态**：`activeProjectId` = "哪个工作区运行时是活的"，`ProjectTab.isActive` = "标签条上哪个高亮"。前者在 `resume`（`:2984`）里写，后者在 `activateDescriptor`（`workspace-lifecycle.ts:35`）里写 —— 两者都由同一次 `openWorkspaceRuntime`/`switchWorkspaceRuntime` 驱动，但**面板只读 `isActive`** | `workspace-lifecycle.ts:35,186` |
| 多窗口 | `projectTabs` 按 webview 窗口 label 分开存（`workspace-tabs.store.ts:51`），所以每个窗口的"当前项目"是独立的；`createAppWindow` 会另开一份 | `:50-65` |
| 切换项目做什么 | `switchToProject(id)`（`file-system.store.ts:2936-3004+`）：查标签 → 未 ready 时置 `isSwitchingProject` → `switchWorkspaceRuntime(id, {persistCurrent, onActivate, initialize, resume})` | `:2936-3003` |
| 具体变化 | ① 持久化当前工作区会话（`persistCurrent`）；② `onActivate` 切换**应用主题**到目标项目的 `theme`（没有就写入当前主题）`file-system.store.ts:2952-2965`；③ 目标已 ready → `resume`：`setActiveProjectId` + `resumeWorkspaceSession` + 后台重启工作区服务（`:2981-3002`）；④ 未 ready → `initialize`：`handleOpenFolderByPath` / `handleOpenRemoteProject` / `handleOpenWslProject`（`:2966-2980`）；⑤ 桌面窗口**不新建**，标签条仍是同一条（`activeProject` → 标签高亮变化） | 同左 |

---

## 4. 交互细节

### 4.1 键盘

真源**没有一行键盘代码** —— 键鼠行为全部来自 `@base-ui/react` 的 `Menu` 原语
（`ui/dropdown.tsx:1,713-761`：`DropdownMenu`=`Menu.Root`、`DropdownMenuTrigger`=`Menu.Trigger`、
`DropdownMenuItem`=`Menu.Item`、`DropdownMenuGroup`=`Menu.Group`、`DropdownMenuLabel`=`Menu.GroupLabel`），
包装层只提供 class，不接管键盘。

⚠️ **本仓库 `windows/tauri/node_modules` 未安装**（`package.json` 声明 `"@base-ui/react": "^1.6.0"`），
所以**没有在本地逐行核对** Base UI 的键盘实现 —— 下面按该原语的公开契约（`Menu` 是无障碍菜单，
roving focus）列出，并标为「未在仓库内核对」：

| 键 | 期望行为 |
| --- | --- |
| `↑` / `↓` | 在**可点项**之间循环移动（分隔线与分组标题跳过） |
| `Home` / `End` | 跳首 / 跳尾 |
| `Enter` / `Space` | 激活当前高亮项并关闭 |
| `Esc` | 关面板并把焦点还给触发器 |
| `Tab` | 关面板 |
| 字符键 | typeahead 定位（Base UI Menu 默认有） |

**我们能在仓库内核对的替代证据**：`ui/dropdown.tsx:590-629` 是**该仓库自研 `Dropdown`**（另一条路径，
不是标题栏用的这一条）的键盘实现，它显式实现了 `ArrowDown` / `ArrowUp` / `Home` / `End` / `Enter`
与 `:553-560` 的 `Esc`（capture 阶段关闭）。标题栏用的是 `DropdownMenu*` 那一支，**没有**这段代码，
行为依赖原语。

**面板内没有"默认高亮第一项"**：`:139-164` 没有给 `open` 之外的初始状态；也没有 `typeahead` 相关的
自定义。落地时 gpui 侧的键盘行为是**现成的**（见 §5.3）。

### 4.2 点击分组标题是否折叠

**不会**。「打开的项目」/「最近项目」是 `DropdownMenuLabel` → `Menu.GroupLabel`
（`ui/dropdown.tsx:848-861`），是**非交互标签**，没有 `onClick`、没有折叠状态，
面板里也没有任何 collapsed/expanded 的 state（`title-project-menu.tsx` 只有 `isOpen` 与 `menuNode`
两个 state，`:121-122`）。

### 4.3 悬停态

| 对象 | 悬停表现 | 出处 |
| --- | --- | --- |
| 触发器 | `hover:bg-accent hover:text-foreground`（ghost） | `ui/button.tsx:16` |
| 动作行 / 项目行 | `focus:bg-accent focus:text-foreground` —— 高亮由原语把焦点（`data-highlighted`）移上去触发，不是纯 `:hover` | `ui/dropdown.tsx:782`、`:46-49` |
| 当前项目行 | 常态就是 `bg-selected text-foreground`（`:96`），悬停时叠 `bg-accent` | `:96`、`ui/dropdown.tsx:782` |
| 禁用行 | `data-disabled:pointer-events-none data-disabled:opacity-50` | `ui/dropdown.tsx:782` |
| 面板进出场 | `data-open:animate-in fade-in-0 zoom-in-95` + `duration-100` + `slide-in-from-top-2` | `ui/dropdown.tsx:753` |

`--selected` 与 `--accent` 的具体色值来自当前主题预设（`@theme inline` 只做映射：
`styles/theme.css:21-23`；预设示例 `extensions/bundled/themes/vercel/manifest.ts:24,30`）。

### 4.4 点击之后的顺序

`closeAndRun`（`:133-136`）= **先 `setIsOpen(false)`，再执行动作**。逐项：

| 点击 | 面板 | 之后 |
| --- | --- | --- |
| 当前项目行 | **不关**（`:208` 直接 return） | 无 |
| 其它已打开项目行 | 关 | `switchToProject(id)` → §3.4 |
| 最近项目行 | 关 | `openRecentFolder(path)`：`getSymlinkInfo` 校验 → 不存在/非目录则标记 `missing` + toast，**不切换**（`recent-folders.store.ts:107-120`）→ 否则 `chooseProjectOpenDestination` 可能弹「在哪打开」→ 新窗口 `createAppWindow` / 本窗口 `handleOpenFolderByPath`；**取消弹窗则什么都不做**（`project-open-destination.ts:102-104`） |
| 打开… | 关 | 系统目录对话框；取消 → `false`，无副作用（`file-system.store.ts:811-812`） |
| 新建项目… / 克隆仓库… | 关 | 打开 `ProjectPicker` 模态 |
| 点面板外 / `Esc` | 关 | 无（`ui/dropdown.tsx` 由原语处理；自研 `Dropdown` 的对应实现在 `:546-560`） |

打开项目**本窗口**时：`openWorkspaceRuntime` 先 `addProjectTab`（去重；命中已有标签则直接置 active）
再 `activateDescriptor`（`workspace-lifecycle.ts:130-137`），失败时回滚删除新标签并恢复上一个工作区
（`:152-168`）。所以标签条会多一个标签（或高亮已有标签），**不会新开窗口**。

---

## 5. gpui 侧落地评估

### 5.1 现状（逐项给行号）

| 事实 | 出处 |
| --- | --- |
| `ShellWorkspace` 只有 `projects: Vec<ProjectTab>` 与 `active_project: Option<usize>` | `gpui/crates/workbench/src/workspace.rs:345-349` |
| 构造期写死单项目：`projects: vec![ProjectTab::new(project_name.clone())]`、`active_project: Some(0)` | `:503-506` |
| `ProjectTab` **只有 `name: SharedString` 一个字段**，`#[non_exhaustive]` | `gpui/crates/workbench/src/project_tabs.rs:203-208` |
| `title_bar(project_name: &str, window: &Window, cx: &App)` —— **无状态、无 `Entity`、无下拉** | `gpui/crates/workbench/src/title_bar.rs:76-97` |
| 项目名画在 `drag_region` 里，该元素挂了 `WindowControlArea::Drag` | `:108-136`，`:115` |
| 渲染顺序：标题栏 → 标签条（`projects.len() > 1` 才画）→ 主体 | `workspace.rs:987`、`:993-1002` |
| 标签条的切换/关闭回调只改 `active_project`；关闭只"取消选中" | `:804-825` |
| 项目根来自命令行参数 `Lithe <workspace-root>`，**没有目录选择器** | `gpui/crates/app/src/main.rs:28,127-174,361` |
| gpui 侧**没有任何**文件对话框依赖（`rfd` / `tinyfiledialogs` / `native-dialog` 全无） | `gpui/Cargo.toml:1-37` + 各 crate 的 `Cargo.toml`（grep 无命中） |
| gpui 侧 i18n 已有全部所需键 | `gpui/crates/shared/locales/lithe.zh-CN.yml:3538-3579`（`titleProject.newProject` / `open` / `cloneRepository` / `openProjects` / `recentProjects` / `noRecentProjects` / `trigger` 全在） |
| `tr` / `tr_args` 接口 | `gpui/crates/shared/src/i18n.rs:89,102-105`；再导出 `gpui/crates/shared/src/lib.rs:43` |
| 已有「`dropdown_menu_with_anchor` + `PopupMenuItem` + `scrollable`」的先例可抄 | `gpui/crates/settings/src/dialog.rs:663-698`（尤其 `:682-696`）、`gpui/crates/terminal/src/terminal_view.rs:574-590`、`gpui/crates/git/src/log_view.rs:668-680`、`gpui/crates/editor/src/editor_view.rs:1340` |

### 5.2 逐条可做性

**A. 第一版立刻能做（纯外观 + 我们已有的那一个项目）**

| 项 | 做法 | 依据 |
| --- | --- | --- |
| 触发器 | `Button(ghost/xs).max_w(px(224.)).gap_1p5().px_2()` + 20×20 logo 载体 + `Icon::new(IconName::ChevronDown).size_3p5()`，展开时旋转 180° | 度量口径同 `title_bar.rs`（`h_10/px_2/gap_1` 已在用），旋转用 `Styled::rotate` |
| 上面三项动作行 | `PopupMenuItem::element(...)` 自绘 `h(px(32.))` + `IconName::Plus` / `FolderOpen` / `GitBranch` | 图标名对应 lucide `plus` / `folder-open` / `git-branch`，全量 `IconName` 枚举已注册（`project_tabs.rs:73-80` 的同一套机制） |
| 「打开的项目」分组 + 当前项 | `menu.label(tr("lithe.titleProject.openProjects"))` + 一行自绘（徽标 + 名 + 路径 + 右侧勾） | `menu.md:145-153`（label）、`:171-199`（多行自绘）、`:113-129`（勾选） |
| 徽标算法 | 把 `getTitleProjectBadge` 逐行搬到 Rust：`split` 非字母数字 + 前 2 词首码点 + 大写 + 兜底 `LI` + `hash*31 & 0x7fffffff` + `% 5` 取 5 色常量 | 真源 `title-project-menu-model.ts:6-31`（可直接做单测对齐 `title-project-menu-model.test.ts:75-81`） |
| 面板尺寸 | `min_w(px(384.)).max_w(px(384.)).scrollable(true).max_h(px(520.))` | `PopupMenu` builder：`popup_menu.rs:429-450`；`menu.md:277-281`、`:255-272` |
| 空态 | 「最近项目」下自绘一行 `tr("lithe.titleProject.noRecentProjects")` | 真源 `:221-225` |
| 分隔线 | `menu.separator()` | `popup_menu.rs:680`；`menu.md:131-141` |

**B. 需要新状态 / 新能力（第一版必须缩范围）**

| 缺口 | 缺什么 | 为什么不能"顺手做" |
| --- | --- | --- |
| 多项目列表 | `ShellWorkspace.projects` 现在恒为 1 项（`workspace.rs:503`），`ProjectTab` 没有 `path` / `icon` / `is_active`（`project_tabs.rs:203-208`） | 需要把 `ProjectTab` 扩成"项目条目"（`name` + `path` + `custom_icon` + `active`），或另建一个只给面板用的视图模型 |
| 最近项目 | **gpui 侧完全没有持久化存储**（无 `recentFolders` 等价物，也没有 zustand/localStorage 的对应层；`gpui/crates/settings/src/store.rs:108` 的 `SettingsStore` 只管设置） | 需要新落点：写 JSON 到平台配置目录 + 上限 12 + pinned/排序/去重 + `missing` 标记 |
| 切换项目 | `active_project` 只改下标（`workspace.rs:804-825`），没有"重新加载工作区"的运行时 | 需要 workspace 生命周期（重新扫树、重起 watcher / LSP / git），属架构级改动 |
| 打开… | 没有目录选择器，也没有 `rfd` 类依赖（见 §5.1 第 8 行） | 需要新平台依赖或经 Core/宿主提供 |
| 新建项目 | 没有脚手架能力（Windows 侧是 `createNewDirectory` + 终端跑命令，`new-project-content.tsx:239-277`） | 同上；且 Windows 侧本身就是"起终端跑 `npm create`"，不是 Core 能力 |
| 克隆仓库 | 没有 `git.write` clone 的调用点；gpui 侧 `CoreClient` 有没有暴露该命令**未核对** | 需要 Core 命令 + 目标目录选择 + 进度 UI |
| 拖拽区冲突 | 见 §5.5 —— 这是**接线前必须处理**的硬约束，不是可选优化 | — |

### 5.3 组件选型：用哪个？

**推荐：`Button::dropdown_menu_with_anchor(Anchor::TopLeft, ...)`**
（也就是真源同一支：`DropdownMenu` → `Popover(appearance=false)` → `PopupMenu`）

> ⚠️ 锚点别猜：官方文档明确写「`Anchor::TopLeft` 会让 Popover 出现在触发器**正下方**，并与其**左对齐**」
> （`gpui/docs/gpui-kit/0.6.6/zh-CN/component/popover.md:40-49`）—— 正是 Windows 的
> `align="start" side="bottom"`（`title-project-menu.tsx:168-169`）。而且 `dropdown_menu()` 的默认锚点
> 就是 `TopLeft`（`CMP/src/menu/dropdown_menu.rs:14-19`），所以**不传锚点也对**；只有想要"右对齐"时才用
> `Anchor::TopRight`（仓库现有唯一用例：`gpui/crates/settings/src/dialog.rs:682`）。
> （`Anchor` 在该实现里的坐标换算见 `gbase/src/popup.rs:61-80` 与 `gpre/src/geometry.rs:840-...`，
> 与文档描述不是直读关系 —— 以文档 + 真机验收为准。）

| 候选 | 评价 | 证据 |
| --- | --- | --- |
| `DropdownMenu` / `dropdown_menu_with_anchor` | ✅ **选它**。一个调用就拿到：锚定定位、点击外部关闭、`Esc` 关闭、`↑↓/←→/Enter/Space/Tab` 键盘导航、focus 归还、分隔线/分组标题/勾选、`min_w/max_w/max_h`、`scrollable` | `menu.md:78-87`（锚点）、`:416-422`（键盘表）；`CMP/src/menu/dropdown_menu.rs:14-31`（唯一文档化入口 + 锚点）；键盘动作绑定 `CMP/src/menu/popup_menu.rs:21-30`，处理点 `:1469-1475` |
| `ContextMenu` | ❌ 锚点是**私有常量 `Anchor::TopLeft`**，没有公开 setter；它也要求宿主元素带稳定 `id` 并且是**右键**触发 | `CMP/src/menu/context_menu.rs:42-58,168,206`；右键判定 `:296-299` |
| `Popover`（`menu.md` 之外的另一条） | ⚠️ 备选。好处是 `.appearance(false)` 后**内容完全自绘**（可精确做 6px padding / 6.4 圆角）；代价是**键盘与 Esc 要自己写**，而且 `Popover::appearance` 的文档注释说"点外不关闭"与代码不一致（代码里 `overlay_closable` 与 `appearance` 是**两个独立开关**，`Popover` 默认 `true`） | `menu.md:139-144`（`appearance(false)` 自定义外观）、`popover.md:73-85`；`CMP/src/popover.rs:240-249`（注释）vs `:305`、`:220-223`（代码）；`gbase/src/popover.rs:328-336`（`on_mouse_down_out` 只看 `overlay_closable`） |
| `gpui_kit::component::menu::PopupMenu` 直接用 | ❌ `PopupMenu::new` 是 `pub(crate)`，外部只能经 `PopupMenu::build(window, cx, f)` 或 `dropdown_menu` 回调拿到实例 | `CMP/src/menu/popup_menu.rs:334,358-364` |

**已知的尺寸不能 1:1 的部分**（都能接受，但要写进交接）：

| 维度 | Windows 真源 | gpui-kit 0.6.6 | 差 |
| --- | --- | --- | --- |
| 面板内边距 | 6（`:170`） | **硬编码 4**（`.p_1()`，`CMP/src/menu/popup_menu.rs:1483`） | −2，无 builder |
| 面板圆角 | 6.4（`theme.css:7,134`） | `theme.radius` = **6**（`gpui/themes/lithe-dark.json:10`；`CMP/src/styled.rs:193-201`） | −0.4，只能改全局主题半径，**不要改** |
| 行间距 | 0（`DropdownMenuItem` 之间无 margin） | `gap_y_0p5()` = **2**（`popup_menu.rs:1484`） | +2，无 builder |
| 行高 | 动作 32 / 项目 44 | `Item` 固定 **26**；`ElementItem` **`min_h` 26**（可被内容撑高） | 用 `element` 自绘 `.h(px(32.))` / `.min_h(px(44.))` 补齐 |
| 行内边距 | `px-2` = 8 | `.px(px(8.))` = **8** | 0 ✅ |
| 面板最大高 | min(520, 100vh−48) | `max_h` 只在 `scrollable(true)` 时生效；默认 min(窗口高/2, 450) | 显式 `.scrollable(true).max_h(px(520.))` |
| 触发器 | `h-6`(24) + `gap-1.5`(6) + `px-2`(8) + `max-w-56`(224) | 均可直接写 | ✅ |

### 5.4 「分组 + 多行项」用现成 `PopupMenuItem` 能不能画？

**结论：能画，但必须走 `ElementItem` 自绘；`Item` 的现成字段不够。**

`PopupMenuItem` 是 enum，完整字段见 `CMP/src/menu/popup_menu.rs:33-65`：

| 需求 | 现成支持 | 结论 |
| --- | --- | --- |
| 分组标题 | ✅ `PopupMenu::label(..)`（`:492-495`）→ `PopupMenuItem::Label`（`:119-121`），渲染在 `:1255-1262` | 能用；但它**复用 item 骨架**（`py_1 px_2 text_base`，`CMP/src/menu/menu_item.rs:104-106`），只是 `disabled(true)` → 前景变 `muted_foreground`。**没有独立的"小号灰色分组标题"样式**，`PopupMenuGroup` 类型在 0.6.6 **不存在**（全 crate 无此标识符）。要 12px 灰字只能自绘 |
| 主标题 + 副标题（路径）两行 | ❌ **没有 title/subtitle 字段**。`Item{label}`、`ElementItem{render}` | 必须用 `PopupMenuItem::element(builder)`（`:85-98`）或 `PopupMenu::menu_element*`（`:587,596,610,624`），builder 里自己 `v_flex()` 两行。这正是官方文档给的"两行菜单项"做法（`menu.md:171-199` 的副标题、`:394-407` 的账号行） |
| 彩色圆角方块徽标 | ❌ 走不了 `icon` 槽：`icon: Option<Icon>`（`:40`）只接受 **`Icon` 字形**，且渲染时被硬编码 `.xsmall()`（`:1153-1173` 的 `:1172`），左侧槽还会被 `has_left_icon` 统一占位（`:250-261`） | **自绘**：在 `element` 里 `div().size(px(28.)).rounded(px(6.4)).bg(tone).child(initials)` |
| 右侧勾（当前项） | ✅ `PopupMenuItem::checked(true)` + `PopupMenu::check_side(Side::Right)`（`:180-191`、`:453-456`），渲染 `IconName::Check` 于右侧（`:1208-1212`） | 能用；但勾是 `Icon::xsmall()`，Windows 是 `size-4`(16) —— 用自绘更稳 |
| 当前项**常态高亮** | ❌ 没有 API。`MenuItemElement::selected` 是 `pub(crate)`，只由 `selected_index`（键盘/悬停）驱动（`CMP/src/menu/menu_item.rs:41-44`、`popup_menu.rs:1214,1233`） | **自绘**：在 `element` 里画一层 `bg(cx.theme().accent)` 的圆角底 |
| 行高 | ⚠️ `Item` 固定 `.h(26.)`（`:1309`）；`ElementItem` 用 `.min_h(26.)`（`:1278`），可被内容撑到 44 | 用 `element` 自绘即可 |
| 禁用 | ✅ `.disabled(true)`（`:161-175`），禁用后文字走 `muted_foreground`（`menu_item.rs:131-133`）且不挂 `on_click`（`:1269-1274`） | 能用（用于 `is_switching_project` 的整组置灰） |
| 点击回调 | ✅ `.on_click(Fn(&ClickEvent, &mut Window, &mut App))`（`:196-210`）。`ElementItem` 的点击路径 `:1269-1273` → `on_click(ix)` → `confirm` → 先跑 handler **再 `dismiss`** | 能用；⚠️ **顺序与真源相反**（真源 `closeAndRun` 是先关再执行，`:133-136`）。好消息：`dismiss` 在 handler 已把焦点移走时**不会抢回焦点**（`:1062-1072`），所以"动作里开模态"是安全的 |

**自绘行的形状**（示意，非代码改动）：

```rust
// 一行「徽标 + 名 + 路径 + 勾」，几何照 Windows §1.5
// 关键点：MenuItemElement 自带 .px(8)（popup_menu.rs:1216,1230），
//         想让它像真源那样"底色铺满行宽"，用负外边距抵消：.mx_neg_2() + .px_2()
menu.item(
    PopupMenuItem::element(move |_window, cx| {
        h_flex()
            .min_h(px(44.))
            .gap_2p5()                        // Windows gap-2.5 = 10
            .mx_neg_2()                       // 抵消父级 .px(8)
            .px_2()                           // 再自己给 8
            .rounded(px(6.4))                 // Windows rounded-md
            .when(is_active, |d| d.bg(cx.theme().accent))   // ← 当前项常态高亮（自绘）
            .child(badge(initials, tone))     // 28×28、rounded(6.4)、bg(tone)、10px 白字
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(div().min_w_0().truncate().text_sm().text_color(cx.theme().foreground).child(name))
                    .child(div().min_w_0().truncate().text_xs().text_color(cx.theme().muted_foreground).child(path)),
            )
            .when(is_active, |d| d.child(Icon::new(IconName::Check).size_4().text_color(cx.theme().primary)))
    })
    .on_click(move |_, window, cx| { /* 切项目 */ }),
)
```

（`IconName::Check`、`IconName::Plus`、`IconName::FolderOpen`、`IconName::GitBranch`、
`IconName::ChevronDown` 都在全量枚举 `gpui_kit::assets::IconName` 里 —— 注册机制同 `project_tabs.rs:73-80`，
对应 svg `check.svg` / `plus.svg` / `folder-open.svg` / `git-branch.svg` / `chevron-down.svg`
**都已在 `gpui-kit-assets-0.6.6/assets/icons/` 核实存在**。
`Icon::size_4()` 走 `Styled`；gpui-kit 自己的 `Icon::xsmall()` 是 12px，要 16px 就显式 `.size_4()`，
`CMP/src/icon.rs:181-187`。）

**要用到的 import（照抄 `gpui/crates/settings/src/dialog.rs` 的既有写法）**：

```rust
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};   // dialog.rs:48
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _};
use gpui_kit::{Anchor, div, px};                                     // dialog.rs:53
```

**已核实的样式 helper**（都在 gpui 的 rem 档位上，见 `gpui-pre-macros-0.3.6/src/styles.rs:926-...`
的 `box_style_suffixes`，`"2p5"` 在表内）：`gap_2p5()`、`px_2()`、`mx_neg_2()`
（负号由 `styles.rs:629-648` 的 `_neg` 限定符生成，用例 `popup_menu.rs:1251` 的 `.mx_neg_1()`）、
`min_h(px(44.))`、`size_4()`、`text_sm()` / `text_xs()`、`rounded(px(6.4))`。

> ⚠️ **圆角不要用 `rounded_sm()` / `rounded_md()`**：gpui 的圆角梯度是 Tailwind 默认
> （`rounded_sm()` = 4px，`gpre-macros/src/styles.rs:1240-1244`），Lithe 的 `rounded-md` 是 **6.4**
> —— 这条口径 `gpui/crates/workbench/src/project_tabs.rs:55-57` 已经写过，沿用即可。

**如果嫌面板 4px padding / 6px 圆角差得多**：改走
`Popover::new(id).appearance(false).anchor(Anchor::TopLeft).trigger(trigger).content(|_, window, cx| 自绘整块面板)`
（`popover.md:73-85,139-144`；`CMP/src/popover.rs:229-249`），代价是键盘/Esc/开合要自己实现 —— **不建议做第一版**。

### 5.5 ⚠️ 拖拽区冲突：加触发器前必须重构 `drag_region`

这是本功能**最容易踩的坑**，也是 `title_bar.rs` 模块头已经在警告的同一类问题。

- 现在 `drag_region` 是 `h_flex().flex_1()...window_control_area(WindowControlArea::Drag)`
  （`title_bar.rs:108-136`，`:115`）。
- Windows 平台命中测试取 `window.rendered_frame.window_control_hitboxes` 里**第一个**命中的项
  （`gpre/src/window.rs:1951-1957`）；这些 hitbox 在**绘制阶段按绘制顺序 push**
  （`gpre/src/window.rs:5136-5139`，`debug_assert_paint`），祖先先于后代 →
  **祖先的 `Drag` 会盖住子元素的 `Min/Max/Close`**（`title_bar.rs:103-107` 的既有结论）。
- 所以：**把项目下拉触发器放进 `drag_region` 内部 → 点它只会拖动窗口，按钮点不动。**
- 正确形状（与 Windows 的 `ChromeGroup grow min-w-0 { ChromeGroup pointer-events-auto { menuItem, projectControls } }`
  同构，`title-bar.tsx:339-343`）：把触发器作为 `drag_region` 的**兄弟**，两者同放在一个
  `h_flex().flex_1().min_w_0()` 里 —— 触发器先画（左）、拖拽区吃剩余宽度（右）：

```rust
h_flex().flex_1().min_w_0()
    .child(project_menu_trigger(..))   // 可点，不含 WindowControlArea
    .child(drag_region(project_name))  // 只覆盖剩余区域
```

（顺带：gpui 侧 `title_bar()` 是**无状态函数**（`title_bar.rs:76`），要弹面板就得让触发器持有
`Entity<ShellWorkspace>` 或在 `workspace.rs:987` 那一行外包一层 —— 这是签名层面的改动，不是面板内部的事。）

---

## 6. 第一版范围建议

**做（纯外观，一轮可验收）**

1. 标题栏左组重构：触发器（logo 20×20 + 项目名 `truncate` + chevron 14×14，展开旋转 180°）
   **作为 `drag_region` 的兄弟**（§5.5）；触发器尺寸 `h-6 / gap-1.5 / px-2 / max-w-56`。
2. 面板：`Button::dropdown_menu_with_anchor(Anchor::TopLeft, ..)`（= 「正下方 + 左对齐」，§5.3），
   `min_w/max_w = 384`、`scrollable(true).max_h(520)`、`p_1`+`gap_y_0p5`（接受 −2/+2 的差异）。
3. 三条动作行（`新建项目… / 打开… / 克隆仓库…`），`h(px(32.))` + 16px 图标；
   **第一版点击只做"占位"**（`on_click` 里什么都不做，或弹一个 `Notification` 说明"待开发"）
   —— ⚠️ **不要用 `.disabled(true)`**，那会让三行一起变灰成 `muted_foreground`（`popup_menu.rs:131-133`），
   与真源"三者都不置灰"（§3.3）不符；它们各自依赖 §5.2-B 的缺口。
4. 分隔线 + 分组标题「打开的项目」+ **当前项目行**（徽标 / 名 / 路径 / 右侧勾 / `bg(accent)` 常态高亮）。
5. 分隔线 + 分组标题「最近项目」+ **空态「没有最近项目」**。
6. 徽标算法（首字母 + 5 色哈希）落到 Rust 并加**对齐 Windows 单测**的单测
   （`title-project-menu-model.test.ts:75-81` 的三条断言：`Lithe-IDEA-issue-35-ci` → `LI` 且稳定、
   `文档项目` → `文`）。
7. 文案全部走 `tr("lithe.titleProject.*")`（**零新增键**，`lithe.zh-CN.yml:3538-3579`）。

**不做（明确写进交接，别顺手做）**

- 最近项目的**数据来源**（无存储层，见 §5.2-B）→ 第一版「最近项目」恒为空态。
- 切换项目（没有工作区生命周期）、打开文件夹对话框（没有平台对话框依赖）、
  新建项目脚手架、克隆仓库（Core 调用点未确认）。
- `isSwitchingProject` 的整组置灰：第一版没有"切换中"这个状态，先不引入。
- 键盘/`Esc`/点外关闭**不用写**：`dropdown_menu_with_anchor` 现成（§5.3）。

**验收口径建议**：`工作台只有一个项目` 时，面板应是「3 动作 + 打开的项目(1，高亮打勾) + 最近项目(空态)」，
共 5 行 + 2 条分隔线 + 2 个分组标题；截图与 Windows 并排对照宽度 384 / 行高 44 / 徽标 28。

---

## 7. 未确认的点

1. **触发器图标：logo 还是项目徽标？** 真源写死 `/logo.png`（`title-project-menu.tsx:155`，且
   `windows/tauri/public/logo.png` 存在），但维护者描述为「带项目图标与 ▾」。
   两种可能：截图那版尚未合到当前 HEAD（该文件最近一次改动是 `72222d40 feat(windows): 支持项目显示别名…`），
   或者维护者把面板内的徽标记到了触发器上。**这一条会直接决定第一版要不要在触发器里画徽标**，需要拍板。
2. **路径行 / 分组标题 / 空态到底几 px？** `ui-text-xs` 在仓库里**未定义**（`styles/utilities.css:30-44` 只有
   `ui-text-sm/-caption/-chrome/-base`；全仓库 `.css` grep 无 `.ui-text-xs`），所以真机实测是继承 **13px**。
   「照意图做 12px（`text_xs()`）」还是「照实测做 13px」需要拍板（注意 gpui 侧已有先例：
   `--ui-text-chrome` 13 → `text_sm()` 14 是**经确认的有意改动**，见 `title_bar.rs:55-57`）。
3. **截图里 `htodsdatams-…` 的具体目录名没找到**，所以那个绿色 `HO` 只在算法层面被解释（两词 → 两字母，
   下标 1 → emerald），没有逐字复核。`jzwg`→蓝、`qzjk`→紫两条已用 node 复算命中。
4. **Base UI `Menu` 的键盘行为没有在本地逐行核对**：`windows/tauri/node_modules` 未安装，
   `package.json` 只声明 `"@base-ui/react": "^1.6.0"`。§4.1 的表格按该原语的公开契约列出。
   （对照物：同仓库自研 `Dropdown` 的键盘实现在 `ui/dropdown.tsx:590-629`、`:553-560`，
   但标题栏**不走**这一支。）
5. **gpui 侧 `CoreClient` 是否暴露 `git.write` 的 clone**：本轮只确认 Windows 侧翻译链路
   （`src-tauri/src/platform.rs:563-579` → Core `git.write`），**没有核对** `gpui/crates/shared` 的
   `CoreClient` 命令表 —— 这决定「克隆仓库…」在 gpui 侧是"接线"还是"先补 Core 能力"。
6. **`PopupMenu` 面板相对触发器的精确偏移**：`dropdown_menu.rs:22-31` 没传 offset，落在
   `gpui-base` 的 `Popover` 默认值上，没有展开核对（`CMP/src/popover.rs:301-322` 只转发）。
   Windows 侧是 `sideOffset = 4`（`ui/dropdown.tsx:736`），gpui 侧可能是别的数 —— 并排验收时才能看出来。
7. **`PopupMenu::size` 是否真的没有公开 setter**：只在 `popup_menu.rs` 内确认无 `pub fn size`，
   没有穷尽 `Sizable` 等派生路径。它决定 gpui 行的 `min_h` 是 26 还是 20（`:1221-1224`）——
   不过我们走自绘 `element`，影响有限。

---

## 附：证据索引（按文件）

| 文件 | 关键行 |
| --- | --- |
| `windows/tauri/src/features/window/components/title-bar/title-project-menu.tsx` | 徽标 `:38-70`；行 `:72-107`；触发器 `:139-164`；面板容器 `:166-171`；3 动作 `:172-192`；分隔 `:194,216`；打开的项目组 `:195-214`；最近项目组 `:217-235`；当前项 no-op `:207-210`；空态 `:221-225` |
| `windows/tauri/src/features/window/utils/title-project-menu-model.ts` | 色板 `:6-12`；徽标算法 `:14-31`；`aria-current` `:33-35`；过滤+截断 `:37-51` |
| `windows/tauri/src/features/window/utils/title-project-menu-model.test.ts` | 去重 `:25-38`；截断 `:40-51`；保持打开 `:53-68`；`aria-current` `:70-73`；徽标确定性 `:75-81` |
| `windows/tauri/src/features/window/components/title-bar/title-bar.tsx` | `projectControls` `:235-240`；Windows 布局 `:332-360`（左组 `:339-344`）；`ProjectPicker` 门户 `:393-400` |
| `windows/tauri/src/features/window/stores/workspace-tabs.store.ts` | `ProjectTab` `:21-31`；存储键 `:50-51`；`addProjectTab` `:77-111`；`setActiveProjectTab` `:119-128`；`setProjectIcon` `:154-161`；持久化配置 `:185-195` |
| `windows/tauri/src/features/window/utils/project-tab-path.ts` | 归一化 `:3`；大小写规则 `:5-24`；`createProjectTabId` `:26-35` |
| `windows/tauri/src/features/window/utils/project-display-label.ts` | `:5-16` |
| `windows/tauri/src/features/file-system/stores/recent-folders.store.ts` | 状态 `:34-36`；`addToRecents` `:60-64`；`importRecentFolders` `:66-92`；`openRecentFolder` `:94-153`；持久化 `:190-222` |
| `windows/tauri/src/features/file-system/utils/recent-folders.ts` | 上限 `:3`；排序 `:26-34`；`limitRecentFolders` `:36-42`；`upsertRecentFolder` `:64-89` |
| `windows/tauri/src/features/file-system/types/recent-folders.types.ts` | `:1-23` |
| `windows/tauri/src/features/file-system/stores/file-system.store.ts` | 默认 `isSwitchingProject` `:806`；`handleOpenFolder` `:810-...`；`addToRecents`+`setActiveProjectId` `:661-667`；`handleOpenFolderByPath` `:1196-1230`；`setIsSwitchingProject` `:2930-2934`；`switchToProject` `:2936-3004` |
| `windows/tauri/src/features/file-system/controllers/project-open-destination.ts` | `chooseProjectOpenDestination` `:78-110`；`executeProjectOpenDecision` `:112-131`；`hasOpenProjectWorkspace` `:133-135` |
| `windows/tauri/src/features/workspace/services/workspace-lifecycle.ts` | `openWorkspaceRuntime` `:115-169`（`addProjectTab` `:130-136`、回滚 `:152-168`）；`switchWorkspaceRuntime` `:171-189` |
| `windows/tauri/src/features/window/components/project-picker.tsx` | `commandStep` `:87,141,437-446`；`remote` 置灰 `:661` |
| `windows/tauri/src/features/window/components/new-project-content.tsx` | `canCreate` `:172-176`；`createProject` `:239-277`（`git_clone` `:250-254`） |
| `windows/tauri/src/features/window/utils/project-picker-mode.ts` | `:1-12` |
| `windows/tauri/src/features/window/components/project-icon-picker.tsx` | `:39-64` |
| `windows/tauri/src/features/window/utils/project-icons.ts` | 扫描规则 `:29-32`；打分 `:34-63`；主入口 `:65-113` |
| `windows/tauri/src/ui/dropdown.tsx` | 原语 import `:1`；条目变体 `:34-57`；自研 `Dropdown` 键盘/Esc `:553-629`；`DropdownMenu*` 包装 `:713-938`（Content `:731-761`、Item `:767-788`、Label `:848-861`、Separator `:863-871`） |
| `windows/tauri/src/ui/button.tsx` | 变体/尺寸 `:8-36` |
| `windows/tauri/src/ui/chrome.tsx` | `chromeGroupVariants` `:35-59`（`tight` `:39`） |
| `windows/tauri/src/ui/scroll-container-wheel.ts` | 理由 `:4-5`；`bindScrollContainerWheel` `:82-92` |
| `windows/tauri/src/styles/theme.css` | 圆角档 `:6-12`；色映射 `:14-40`；字号令牌 `:112-117`；chrome 度量 `:118-134`；阴影 `:158-162` |
| `windows/tauri/src/styles/utilities.css` | `ui-text-sm/-caption/-chrome/-base` `:30-44`（**无 `ui-text-xs`**） |
| `windows/tauri/src/styles.css` | `@import "tailwindcss"` `:10` |
| `windows/tauri/src/i18n/locale.ts` | 英文 `:1757-1778`；中文 `:6171-6191` |
| `windows/tauri/src/config/backend-capabilities.ts` | `git: true` `:9`；`BACKEND_UNAVAILABLE_TOOLTIP` `:1` |
| `windows/tauri/src-tauri/src/platform.rs` | `git_clone` → `git.write` `:563-579` |
| `gpui/crates/workbench/src/title_bar.rs` | 契约 `:76-97`；`drag_region` `:99-136`；hitbox 优先级的既有结论 `:103-107` |
| `gpui/crates/workbench/src/project_tabs.rs` | `ProjectTab` `:203-220`；`project_tabs` `:234-241`；token 口径 `:248-254`；图标机制说明 `:71-80` |
| `gpui/crates/workbench/src/workspace.rs` | 字段 `:345-349`；单项目初始化 `:503-506`；切换/关闭回调 `:804-825`；渲染 `:987,993-1002` |
| `gpui/crates/shared/src/i18n.rs` | `tr` `:89`；`tr_args` `:102-105` |
| `gpui/crates/shared/locales/lithe.zh-CN.yml` | `:3538-3579` |
| `gpui/crates/settings/src/dialog.rs` | `dropdown_menu_with_anchor` + `PopupMenuItem` 先例 `:663-698` |
| `gpui/crates/app/src/main.rs` | 根目录来自命令行 `:28,127-174,361` |
| `gpui/themes/lithe-dark.json` / `lithe-light.json` | `radius` `:10`；`accent.*` `:18-19`；`muted.foreground` `:21`；`popover.*` `:26-27`；`primary.*` `:31-32` |
| `gpui/docs/gpui-kit/0.6.6/zh-CN/component/menu.md` | 锚点 `:78-87`；勾选 `:113-129`；分组标签 `:145-153`；自定义多行 `:171-199`、`:381-407`；滚动/尺寸 `:255-281`；键盘表 `:416-422`；`item`+`on_click` `:325-344` |
| `gpui/docs/gpui-kit/0.6.6/zh-CN/component/popover.md` | `.content` `:73-85`；`.appearance(false)` `:139-144` |
| `gpui-component-0.6.6/src/menu/popup_menu.rs` | `PopupMenuItem` 字段 `:33-65`；构造/builder `:71-210`；`has_left_icon`/`is_checked` `:250-279`；`PopupMenu` 字段 `:282-331`；`new` 是 `pub(crate)` `:334`；`build` `:358-364`；尺寸 builder `:429-462`；`label` `:492`；`menu_element*` `:587-677`；`clickable`/`on_click`/`confirm` `:845-890`；`dismiss` 焦点保护 `:1055-1082`；`render_icon` `:1153-1173`；`max_width` 默认 `:1176-1178`；`render_item` `:1198-1403`；`Render`（`p_1`/`gap_y_0p5`/`popover_style`）`:1421-1506` |
| `gpui-component-0.6.6/src/menu/menu_item.rs` | `MenuItemElement` `:11-136`（骨架样式 `:96-134`，hover/selected `:115-123`，disabled `:131-133`） |
| `gpui-component-0.6.6/src/menu/dropdown_menu.rs` | trait `:12`；`dropdown_menu` `:14-19`；`dropdown_menu_with_anchor` `:22-31`；`DropdownMenuPopover` `:37-88`；`Popover(appearance=false, overlay_closable=false)` `:121-129` |
| `gpui-component-0.6.6/src/menu/context_menu.rs` | trait `:13-39`；锚点常量 `:58` |
| `gpui-component-0.6.6/src/popover.rs` | builder `:157-258`；`appearance` 文档注释 `:240-249`；内容层 `:274-291`；`overlay_closable` 转发 `:301-322` |
| `gpui-component-0.6.6/src/styled.rs` | `popover_style`（bg/shadow/`rounded(theme.radius)`）`:193-201` |
| `gpui-base-0.6.6/src/popover.rs` | `overlay_closable` 默认 `:193`；`Esc`/`Cancel` `:316-324`；点外关闭 `:328-336` |
| `gpui-pre-0.3.6/src/window.rs` | 命中测试取第一个 `:1951-1957`；`insert_window_control_hitbox` `:5136-5139` |
