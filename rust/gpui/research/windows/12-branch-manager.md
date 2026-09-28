# 12 · 标题栏「分支弹窗」（`GitBranchManager`）Windows 真源 → gpui 接线研究

研究范围：Windows 前端 `windows/tauri/src/`（**只读真源**）里**点标题栏分支名弹出的那个面板**
（组件名 `GitBranchManager`）的完整规格，以及它在 `gpui/` 侧的可做性与接线方式。

- 分支：`feat/gpui-shell-rewrite`。本文**不改任何代码**、不跑 cargo、不启动应用。
- 证据格式：`文件:行号`（相对仓库根）。查不到的一律写「未找到」，不做推测。
- **不重复**：底部工具窗（`03-git-and-bottom.md`）与左栏源代码管理视图（`08-source-control.md`）
  已写完的组件树、度量、i18n 表，本文只在必要处回指。本文是 `08-source-control.md:233-244`
  §2.2⑦「分支与同步状态其实在标题栏」那一句的**展开与落地判定**。
- gpui 侧源码根：`D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\`
  （`gpui-kit-0.6.6/`、`gpui-component-0.6.6/`、`gpui-base-0.6.6/`、`gpui-pre-0.3.6/`）。

---

## 0. 一句话结论

| 问题 | 结论 |
| --- | --- |
| 面板是什么 | **不是菜单，是与命令面板同源的模态浮层**：`ui/command.tsx` 的 `Command`（Base UI `Dialog` + 遮罩 + `pt-16` 居中偏上 + `max-h 512`），内容由 `GitCommandSurface` 组合 —— 搜索行 / 页签行 / 列表 / 底栏四段 |
| 挂在哪 | **标题栏**，不是状态栏：`useFooterGitBranchItem()`（`footer-git-branch-item.tsx:11`）被 `title-bar.tsx:66,238` 消费；`Footer` 只渲染 `filePathItem`（`footer.tsx:24-26`），`FOOTER_LEADING_ITEM_IDS` 里的 `"branch"`（`item-order.ts:20`）是**残留项** |
| 三个页签 | 固定顺序 `仓库` / `分支` / `工作树`（`git-branch-manager.tsx:608-630`），**默认「分支」**（`:158,528`）；三者共用同一个搜索框与同一个列表容器，只是行数据与点击行为不同 |
| 分支列表数据 | 前端调 `git_branches`（`git-branches-api.ts:54`）→ Rust 翻译成 Core 的 **`git.history`**（`platform.rs:219`）→ 适配器只取 `kind === "local"` 的 `shortName`（`core-result-adapter.ts:247-254`）。**只有名字**，没有 ahead/behind、没有最后提交、没有远端分支 |
| 搜索 | **纯前端过滤**（`:65-76` + `utils/search-match.ts`），不发命令；计数徽章显示的是**过滤前的总数**（`:679-681`） |
| 写操作 | 新建 `git.write{createBranch}`（不检出）、切换 `git.checkoutPreflight` + `git.write{checkout}`、删除 `git_delete_branch`→`git.write{deleteBranch}`、合并/变基 `git.write{merge|rebase}`；**刷新只重读分支名**，不刷新 `git.status` |
| 危险操作确认 | 删除 / 合并 / 变基各一次 `showConfirmDialog`；**没有「强制切换」**；脏工作区先被 `checkoutPreflight` 拦住，再给「贮藏更改」按钮重试 |
| gpui 现有能力 | 5 条 `git.*` 已通（`gpui/crates/git/src/lib.rs:93-98`）：`git.status` / `git.references` / `git.historyPage` / `git.historyCursorClose` / `git.commitFiles`。**「只读分支列表 + 当前标记 + 搜索 + 刷新」这一档零新命令** |
| 缺的命令 | 切换（`git.checkoutPreflight` + `git.write{checkout}`）、新建（`createBranch`）、删除（`deleteBranch`）、合并/变基（`merge`/`rebase`）、贮藏（`stashPush`）、工作树（`git.worktrees` + `createWorktree`/`removeWorktree`）、仓库列表（`workspace.repositories`） |
| 组件选型 | `PopupMenu` **不能**做这个面板（无输入框、任一点击即 dismiss）；用 `Dialog` + `Command`（仓库里已有先例 `gpui/crates/workbench/src/command_palette.rs`）。「多行项 + 右侧标记」用 `CommandItem::child` **能画**，但页签的**位置**（搜索框下方、列表上方）内置 API 没有槽位 |
| 没能确认 | 8 条，见 §7.2（其中第 1 条最关键：截图里的 `▾` 在真源里**不属于**分支项） |

---

## 1. 真源：结构、度量、文案

### 1.1 挂载位置（先纠正一个容易踩的坑）

```
TitleBar                                    windows/tauri/src/features/window/components/title-bar/title-bar.tsx:54
└─ ChromeGroup gap="tight"（gap = 2px）     :236
   ├─ TitleProjectMenu（项目下拉）            :237 → title-project-menu.tsx:109
   └─ {branchItem?.content} ← 本面板的触发器   :238
```

| 事实 | 证据 |
| --- | --- |
| 标题栏消费 `useFooterGitBranchItem()` | `title-bar.tsx:8,66,238` |
| 该 hook 是**唯一**实例化 `GitBranchManager` 的地方 | `footer-git-branch-item.tsx:33-66`；全仓 grep `GitBranchManager` 只有 `footer-git-branch-item.tsx:2,33` 与定义文件自身 |
| `Footer` **不渲染** branch 项（`footerLeadingItemsSource` 只有 `[filePathItem]`） | `features/layout/components/footer/footer.tsx:24-29,57-67` |
| `FOOTER_LEADING_ITEM_IDS` 仍写着 `"branch"`，但没人消费 → 残留 | `features/layout/config/item-order.ts:20` |
| hook 返回 `null` 的条件：**没有仓库路径，或没有分支名** | `footer-git-branch-item.tsx:20-27` |
| 标题栏本身：高 40、`gap` 4、`px` 8、13px、`text-subtle-foreground` | `ui/chrome.tsx:6,11`；`styles/theme.css:118,130,132` |
| 触发器所在组间距 `gap="tight"` = **2px** | `title-bar.tsx:236`；`ui/chrome.tsx:39`；`styles/theme.css:129` |
| 从命令面板也能打开（`paletteTarget` 为真时监听 `lithe:open-branch-manager`） | `footer-git-branch-item.tsx:38`；`git-branch-manager.tsx:151,258-268`；派发方 `git-view.tsx:69,345-350` |

### 1.2 触发器（标题栏那一项）

`GitBranchManager` 的返回值是 `<><Button …/><GitCommandSurface …/></>`（`:632-863`），
即「一个按钮 + 一个受控浮层」。

| 元素 | class / 值 | 换算 | 证据 |
| --- | --- | --- | --- |
| 按钮 | `Button variant="ghost" size="xs"`（`h-6 gap-1 px-1.5`）+ `px-2` 覆盖 | 高 **24**、gap **4**、px **8** | `git-branch-manager.tsx:634-646`；`ui/button.tsx:23` |
| 尺寸分支 | `size={triggerSurface === "footer" ? "xs" : "default"}`，调用方**恒传 `"footer"`** → 永远走 `xs` | `default` 分支是死代码 | `:639`；`footer-git-branch-item.tsx:39` |
| 外观 | `inline-flex max-w-full shrink overflow-hidden text-subtle-foreground hover:bg-accent/80`；footer 形态再加 `font-medium`；展开时 `bg-accent/80` | — | `:640-644` |
| 分支图标 | `<GitBranchIcon className="shrink-0" />`，无尺寸类 → 命中 Button 基线的 `[&_svg:not([class*='size-'])]:size-3.5` | **14** | `:647`；`ui/button.tsx:9` |
| 分支名 | `<span className="min-w-0 truncate font-normal" style={{maxWidth: `${min(max(len+1,6),40)}ch`}}>` | 上限 **40ch**（按字符数夹取） | `:189-190,648-653` |
| ahead/behind | `<GitTrackingCounts showCounts={false} …>` → **只有箭头 `↙`/`↗`，不画数字** | 字号 **10**、gap **4**、两个都为 0 时整个返回 `null` | `:654-660`；`git-tracking-counts.tsx:33,36-53` |
| 无障碍名 | `aria-label={t("git.searchBranchesAria")}` = **搜索分支** | — | `:645`；`i18n/locale.ts:6917` |
| **`▾` 箭头** | **触发器里没有 caret**。触发器的子节点只有三样：分支图标、分支名、ahead/behind | — | `:646-661` |
| 截图里的 `▾` 来自哪里 | 来自**左邻的项目下拉**：`TitleProjectMenu` 的 `<ChevronDownIcon className="size-3.5 …" />`（展开时 `rotate-180`），它在 `title-bar.tsx:237`，分支项在 `:238`，两者相距 2px | **14** | `title-project-menu.tsx:140-164`；`title-bar.tsx:236-238` |

> ⚠️ **维护者截图里的「分支名 + `▾`」是两个相邻控件被看成一个**：`项目名 ▾` 是项目菜单，
> `⎇ 分支名` 才是分支项。详见 §7.2 第 1 条；项目菜单那一侧（含 `▾` 的度量）见
> `gpui/research/windows/11-project-menu.md` §1.2「箭头」与 §6 触发器。

### 1.3 面板骨架（四段）

```
GitCommandSurface                            windows/tauri/src/features/git/components/git-command-surface.tsx:18
└─ Command（Base UI Dialog + 遮罩）           ui/command.tsx:121
   ├─ CommandHeader                          :198
   │  ├─ CommandInput ← 搜索框                 :360
   │  ├─ CommandHeaderBadge ← 「{count} 个分支」 :89
   │  └─ CommandHeaderAction(X) ← 关闭按钮      :76 / :213-217
   ├─ headerAddon = CommandTabs（三个页签）      git-command-surface.tsx:57；git-branch-manager.tsx:691
   ├─ CommandList（ScrollArea，content p-2）    ui/command.tsx:236
   └─ CommandFooter（sticky 底栏）               :340
```

| 事实 | 证据 |
| --- | --- |
| 面板是**模态 Dialog**：`DialogPrimitive.Root/Portal/Popup`、遮罩 `fixed inset-0 z-10060 flex items-start justify-center pt-16`、点遮罩关闭、`initialFocus` 指向 `[data-command-input]` | `ui/command.tsx:138-181`（尤其 `:143-149,154`） |
| 打开时用 `requestAnimationFrame` 聚焦并**全选**搜索框内容 | `git-command-surface.tsx:33-42` |
| 有阻塞性模态（快捷打开/命令面板/全局搜索/设置/项目选择器/数据库连接）打开时，本面板自动关闭 | `git-branch-manager.tsx:178-186,276-279` |
| 关闭时搜索词被清空 | `:270-274` |
| 无障碍标题：`GitCommandSurface` **没有**传 `title` → 落到 `t("commandPalette.title")` = **命令面板** | `ui/command.tsx:174-176`；`git-command-surface.tsx:45-46` |
| 动画：`initial {opacity:0, scale:0.98, y:-8, blur(2px)}` → `{1, 1, 0, 0}`；`prefersReducedMotion` 时瞬时 | `ui/command.tsx:156-169` |

### 1.4 搜索行

| 元素 | 文案 / 键 | 证据 |
| --- | --- | --- |
| placeholder（分支页签） | **搜索分支...** — `git.searchBranches` | `git-branch-manager.tsx:670-672`；`locale.ts:6907` |
| placeholder（工作树页签） | **搜索工作树...** — `git.searchWorktrees` | `:673-674`；`locale.ts:6908` |
| placeholder（仓库页签） | **筛选仓库...** — `git.filterRepositories` | `:675`；`locale.ts:6909` |
| 计数徽章（分支） | **{count} 个分支** — `git.branchCount`（单数）/ `git.branchesCount`（复数）；**中文两键同值**，所以「1 个分支」就是它的单数形态 | `:678-681`；`locale.ts:6910,6911` |
| 计数徽章（工作树） | **{count} 个工作树** — `git.worktreeCount` / `git.worktreesCount` | `:682-685`；`locale.ts:6912,6913` |
| 计数徽章（仓库） | **{count} 个仓库** — `git.repositoryCount` / `git.repositoriesCount` | `:686-689`；`locale.ts:6914,6915` |
| **计数取的是未过滤总数**（`branches.length` / `worktrees.length` / `availableRepoPaths.length`），不是 `filtered*` | — | `:679,683,687` |
| 关闭按钮 | `aria-label` **关闭命令面板** — `commandPalette.close`（**不是**「关闭」） | `ui/command.tsx:214`；`locale.ts:7938` |
| 搜索**过滤什么** | 分支：只看**分支名**一个字段（`matchesSearchQuery(normalizedQuery, [branch])`） | `:65-76` |
| | 工作树：**目录名 + 完整路径 + 分支名 + head 短哈希（7 位）** 四个字段 | `:95-113` |
| | 仓库：**目录名 + 完整路径** 两个字段 | `:123-140` |
| 匹配算法 | NFKD 归一 → 去音标 → 小写 → 非 `[a-z0-9]` 变空格；同时再匹配**去空格的紧凑串**（所以 `featrecent` 能命中 `feature/recent`） | `utils/search-match.ts:1-35` |
| 排序 | 当前项置顶，其余 `localeCompare`；工作树另按「目录名」比较、仓库按「目录名」比较 | `:66-70,96-100,128-132` |

### 1.5 页签行

| 元素 | 文案 / 键 | 证据 |
| --- | --- | --- |
| 页签顺序 | `repositories` → `branches` → `worktrees`（数组字面量顺序） | `git-branch-manager.tsx:608-630` |
| `仓库` | `git.repositories` + `FolderOpenIcon` | `:609-615`；`locale.ts:6836` |
| `分支` | `git.branches` + `GitBranchIcon` | `:616-622`；`locale.ts:6837` |
| `工作树` | `git.worktrees` + `NodesIcon` | `:623-629`；`locale.ts:6838` |
| 图标尺寸 | `size-3.5` = **14** | `:63` |
| 页签容器 aria | **Git 选择分区** — `git.selectorSections` | `:691`；`locale.ts:6916` |
| 默认页签 | `useState<GitBranchManagerTab>("branches")`，每次打开**重置为 `branches`** | `:158,526-531` |
| 从命令面板打开可指定页签 | `detail.tab`（`"branches" \| "worktrees" \| "repositories"`） | `:258-264`；`git-view.tsx:345-350`；`command-palette/constants/git-actions.tsx:56-67,103-109` |
| 切页签后**焦点回到搜索框** | `handleTabChange` → `focusCommandInput()`（`requestAnimationFrame`） | `:514-524` |
| 页签是**胶囊按钮**（`rounded-full px-3`，`size="md"` → `min-h-8`），不是下划线 | — | `ui/command.tsx:499-507`；`ui/tabs.tsx:61` |
| 选中态 | `data-active:bg-accent/80 data-active:text-foreground`；悬停 `hover:bg-accent/50` | `ui/tabs.tsx:55` |

### 1.6 列表与三种行

外壳（三个页签共用）：

| 事实 | 证据 |
| --- | --- |
| 列表是 `ScrollArea`，内容内边距 `p-2` = **8** | `ui/command.tsx:243-248` |
| 每个页签的行都包在 `<div className="space-y-1">` 里（行间再加 4px） | `git-branch-manager.tsx:700,739,773` |
| 行原语 `CommandItemRow`：图标槽 + 内容槽 + accessory + action 四段 | `ui/command.tsx:610-651` |
| 所有行都显式传 `as="div"` → 渲染成 `role="button" tabIndex=0` 的 **div**（不是 `<button>`），带 Enter/Space 键盘处理 | `:702,714,741,753,777,890,977,1022`；`ui/command.tsx:424-452` |
| 行基础样式：`min-h-8 gap-2.5 rounded-lg px-2.5 py-2 leading-row 13px`；选中 `bg-selected text-foreground`，否则 `hover:bg-accent` | `ui/command.tsx:37-41,444-448` |
| 三种行都追加 `min-h-9` = **36**（覆盖基础的 32） | `git-branch-manager.tsx:904,991,1042` |
| 行图标槽 `CommandItemIcon`：`inline-flex size-5`；`iconVariant` 默认 `"plain"` → **无边框无底色** | `ui/command.tsx:547-560,613` |
| 行尾 accessory 槽：`flex shrink-0 items-center gap-1.5` | `ui/command.tsx:574-578` |
| 空态：`p-3 text-center leading-row text-subtle-foreground 13px` | `ui/command.tsx:724-730` |
| 键盘导航：`↑`/`↓` 移动高亮、`Enter` 确认；`resetKey = "${activeTab}:${branchQuery}"` → 换页签或改搜索词时高亮回到第 0 项；鼠标悬停也会改高亮 | `git-branch-manager.tsx:594-602`；`ui/command.tsx:672-710` |

**① 分支行 `BranchRow`（`:866-954`）**

| 元素 | 值 | 证据 |
| --- | --- | --- |
| 前导图标（当前分支） | `CheckIcon size-3.5 text-success` | `:892-893` |
| 前导图标（其它分支） | `GitBranchIcon size-3.5 text-subtle-foreground` | `:894-896` |
| 标题 | 分支名（`truncate`） | `:898` |
| 文字色 | 当前 `text-foreground`；其它 `text-subtle-foreground hover:text-foreground` | `:903-906` |
| **右侧「当前」标记** | `CommandItemBadge variant="success"` → Badge `rounded-full px-1.5 py-0.5 bg-success/10 text-success` + 覆盖 `h-auto max-w-32 shrink-0 gap-1 truncate`；文案 `git.current` = **当前** | `:907-909`；`ui/command.tsx:564-570`；`ui/badge.tsx:13,19`；`locale.ts:6948` |
| 右侧「…」按钮 | 仅**非当前**分支有；`Button ghost icon-xs` = 24×24，图标 `DotsThreeIcon`（默认 14），`aria-label` = `git.branchActions` = **{branch} 的分支操作** | `:910-932`；`ui/button.tsx:27,9`；`locale.ts:6952` |
| 「…」下拉（`min-w-64` = 256） | ① **合并到当前分支** `git.mergeIntoCurrent`；② **将当前分支变基到此分支** `git.rebaseOntoThis`；分隔线；③ **删除分支** `git.deleteBranch`（`text-git-deleted`） | `:933-947`；`locale.ts:6953,6954,6955` |
| 当前分支**没有**「…」入口 → 当前分支不可能被删/被合并 | `:910-911` | — |
| 列表为空的两句话 | 有搜索词 → **没有匹配的分支** `git.noMatchingBranches`；否则 **未找到分支** `git.noBranchesFound` | `:694-698`；`locale.ts:6920,6934` |
| 搜索词可作新分支名时，列表**顶部**多一行 | `CommandItemRow` + `Plus`，标题 `git.createNewBranch` = **创建新分支 “{name}”**，`min-h-9` | `:699-712`；`locale.ts:6935` |
| 「可作新分支名」的判定 | 搜索词 trim 后非空、不等于当前分支名、与现有分支**大小写不敏感**地不重名 | `:78-86` |
| 点击行（非当前）= 切换分支；点击顶部那行 = 新建分支 | `:713-726` → `handleBranchChange` / `handleCreateBranch` | — |

**② 仓库行 `RepositoryRow`（`:956-1005`）**

| 元素 | 值 | 证据 |
| --- | --- | --- |
| 前导图标 | 当前仓库 `CheckIcon text-success`；其它 `FolderOpenIcon text-subtle-foreground` | `:979-985` |
| 标题 | 仓库**目录名**（`getFolderName`） | `:986` |
| 描述（同一行，inline） | **相对工作区根的路径**（`getRelativePath`；等于 `"."` 时退回绝对路径） | `:974,987` |
| 右侧徽章 | 当前 → `当前`（success）；已手工添加 → `git.added` = **已添加**（default） | `:995-1002`；`locale.ts:6949` |
| 空态 | 正在发现仓库 → **正在检测仓库...** `git.detectingRepositories`；否则 **没有匹配的仓库** / **未找到仓库** | `:764-771`；`locale.ts:6940,6941,6942` |
| 数据源 | `useRepositoryStore` 的 `availableRepoPaths` / `manualRepoPaths` / `workspaceRootPath` / `isDiscovering` | `:167-171` |

**③ 工作树行 `WorktreeRow`（`:1007-1050`）**

| 元素 | 值 | 证据 |
| --- | --- | --- |
| 前导图标 | 当前工作树 `CheckIcon text-success`；其它 `NodesIcon text-subtle-foreground` | `:1024-1030` |
| 标题 | 工作树**目录名** | `:1031` |
| 描述（同一行） | `GitBranchIcon` + 分支名 | `:1032-1037` |
| 分支名的回退文案 | `worktree.branch` 为空时：`is_detached` → **分离的 HEAD** `git.detachedHead`；否则 **无分支** `git.noBranch` | `:88-93`；`locale.ts:6950,6951` |
| 右侧徽章 | 当前工作树（`worktree.path === repoPath`）→ **当前** | `:1045-1047` |
| 过滤掉的工作树 | `is_bare`、`is_prunable` 或有 `prunable_reason` 的**不列出** | `:96`；`utils/git-worktree-open.ts:12-14` |
| 排序 | 当前工作树置顶，其余按目录名 | `:96-100` |
| 空态 | 加载中 → **正在加载工作树...** `git.loadingWorktrees`；否则 **没有匹配的工作树** / **未找到工作树** | `:729-737`；`locale.ts:6936,6937,6938` |
| 搜索词可作新工作树路径时，顶部多一行 | `Plus` + `git.createWorktree` = **创建工作树 “{path}”** | `:738-751`；`locale.ts:6939` |
| 判定 | 搜索词 trim 后非空、且不存在同路径工作树 | `:115-121` |

### 1.7 底栏（两个按钮 / 三个按钮）

| 页签 | 左 | 右 | 额外 | 证据 |
| --- | --- | --- | --- | --- |
| 分支 | **新建分支** `git.newBranch`（`Plus`） | **刷新** `git.refresh`（`RefreshCw`） | — | `:790-809`；`locale.ts:6943,6944` |
| 工作树 | **管理工作树…** `git.worktreeDialog.manage`（`Plus`） | **刷新** | — | `:810-829`；`locale.ts:6881` |
| 仓库 | **添加** `git.add` / 选择中 **正在添加...** `git.adding`（`Plus`） | **刷新** | 有手工添加的仓库时多一个 **清除已添加** `git.clearAdded`；`selectionError` 以 `text-destructive/90` 内联显示 | `:830-859`；`locale.ts:6945,6946,6947` |

- 「新建分支」的**禁用条件是 `!createBranchName`**（`:795`）→ 按钮恒在，但**只有搜索框里是一个合法新分支名时才可点**；
  点它执行的是 `handleCreateBranch(createBranchName)`，与列表顶部那一行是同一个动作。
- 底栏按钮外观：`Button variant="default"`（`bg-accent text-foreground hover:bg-selected`）+ `ui-text-base gap-1.5 [&_svg]:size-4`，
  `size` 默认 → `h-8 px-3` = **32 高、px 12、图标 16**。证据：`ui/command.tsx:714-720`；`ui/button.tsx:13,22`。

### 1.8 度量表（面板整体）

> 换算基准见 `03-git-and-bottom.md` §0：1rem = 16px、Tailwind 1 单位 = 4px、`ui-text-base`/`ui-text-sm` = 13px、
> `rounded-lg` = 8、`rounded-xl` = 11.2、`rounded-md` = 6.4。

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 遮罩层 | `fixed inset-0 z-10060 flex items-start justify-center pt-16` | 顶距 **64**；层级 10060 | `ui/command.tsx:143` |
| 面板宽 | `w-[min(44rem,calc(100vw-2rem))]` | ≤ **704**（窗宽 −32 时更窄） | `ui/command.tsx:29` |
| 面板最大高 | `max-h-[min(68vh,32rem)]` | ≤ **512** | 同上 |
| 面板圆角 / 边框 / 底色 / 阴影 | `rounded-xl border border-border bg-background shadow-(--shadow-dialog)` | 11.2 / 1 | 同上 |
| 搜索行 | `flex items-center gap-2 px-4 py-3` + `border-b` | gap **8**、px **16**、py **12** | `ui/command.tsx:52,210` |
| 搜索框 | `h-7 min-w-0 flex-1 leading-[1.4]` 13px | 高 **28** | `ui/command.tsx:55` |
| 计数徽章 | `h-auto min-h-7 max-w-40 px-2 leading-row 13px bg-surface/70` + Badge `rounded-full` | ≥ **28**、≤ **160**、px **8** | `ui/command.tsx:92`；`ui/badge.tsx:6,19` |
| 关闭按钮 | Button ghost `sm`（`h-7 px-2.5`）+ `rounded-md px-2` + `[&_svg]:size-4` | 高 **28**、图标 **16**、圆角 **6.4** | `ui/command.tsx:76-83`；`ui/button.tsx:24` |
| 页签行 | `shrink-0 gap-0 bg-background px-2 pt-2` | px **8**、pt **8**、页签间 **4** | `ui/command.tsx:495`；`ui/tabs.tsx:30` |
| 页签（胶囊） | `min-h-8 px-3 rounded-full` + `gap-(--lithe-chrome-gap-loose)` | ≥ **32**、px **12**、图标与文字间 **6** | `ui/tabs.tsx:55,61`；`ui/command.tsx:503`；`styles/theme.css:131` |
| 列表内容内边距 | `p-2` | **8** | `ui/command.tsx:246` |
| 行高 / 行间距 | `min-h-9`；行自身 `mb-1` + 外层 `space-y-1` | ≥ **36**；行间 **8** | `git-branch-manager.tsx:904,700`；`ui/command.tsx:41` |
| 行圆角 / 内边距 / 内部间距 | `rounded-lg px-2.5 py-2 gap-2.5` | 8 / 10 / 8 / 10 | `ui/command.tsx:41` |
| 行图标槽 | `CommandItemIcon` = `inline-flex size-5`（plain） | **20×20**，内含 14 图标 | `ui/command.tsx:547-560`；`git-branch-manager.tsx:63` |
| 行尾徽章槽 | `flex shrink-0 items-center gap-1.5`；Badge `px-1.5 py-0.5` | 槽内 gap **6** | `ui/command.tsx:574-578,564-570`；`ui/badge.tsx:19` |
| 「…」按钮 | `Button ghost icon-xs` | **24×24** | `git-branch-manager.tsx:922-928`；`ui/button.tsx:27` |
| 「…」下拉最小宽 | `min-w-64` | **256** | `git-branch-manager.tsx:933` |
| 空态 | `p-3 text-center` | p **12** | `ui/command.tsx:727` |
| 底栏 | `sticky bottom-0 border-t px-3 py-3` + 内层 `flex flex-wrap gap-2` | px **12**、py **12**、gap **8**、上边框 **1** | `ui/command.tsx:343,345` |
| 底栏按钮 | Button `default` size（`h-8 px-3`）+ `gap-1.5 [&_svg]:size-4` | 高 **32**、px **12**、图标 **16** | `ui/button.tsx:22`；`ui/command.tsx:717` |
| 触发器 | Button ghost `xs`（`h-6 gap-1`）+ `px-2` | 高 **24**、px **8**、gap **4** | `git-branch-manager.tsx:634-646`；`ui/button.tsx:23` |

### 1.9 文案与 i18n 键（zh-CN 段，`windows/tauri/src/i18n/locale.ts`）

| 位置 | 键 | 中文（逐字） | 行号 |
| --- | --- | --- | --- |
| 触发器 aria | `git.searchBranchesAria` | 搜索分支 | 6917 |
| 搜索框 | `git.searchBranches` | 搜索分支... | 6907 |
| | `git.searchWorktrees` | 搜索工作树... | 6908 |
| | `git.filterRepositories` | 筛选仓库... | 6909 |
| 计数徽章 | `git.branchCount` / `git.branchesCount` | {count} 个分支（两键同值） | 6910 / 6911 |
| | `git.worktreeCount` / `git.worktreesCount` | {count} 个工作树 | 6912 / 6913 |
| | `git.repositoryCount` / `git.repositoriesCount` | {count} 个仓库 | 6914 / 6915 |
| 页签容器 aria | `git.selectorSections` | Git 选择分区 | 6916 |
| 页签 | `git.repositories` / `git.branches` / `git.worktrees` | 仓库 / 分支 / 工作树 | 6836 / 6837 / 6838 |
| 关闭按钮 aria | `commandPalette.close` | 关闭命令面板 | 7938 |
| 当前标记 | `git.current` | 当前 | 6948 |
| 已添加标记 | `git.added` | 已添加 | 6949 |
| 新分支行 / 底栏 | `git.createNewBranch` | 创建新分支 “{name}” | 6935 |
| | `git.newBranch` | 新建分支 | 6943 |
| | `git.refresh` | 刷新 | 6944 |
| | `git.add` / `git.adding` / `git.clearAdded` | 添加 / 正在添加... / 清除已添加 | 6945 / 6946 / 6947 |
| 行内「…」 | `git.branchActions` | {branch} 的分支操作 | 6952 |
| | `git.mergeIntoCurrent` | 合并到当前分支 | 6953 |
| | `git.rebaseOntoThis` | 将当前分支变基到此分支 | 6954 |
| | `git.deleteBranch` | 删除分支 | 6955 |
| 空态 | `git.noMatchingBranches` / `git.noBranchesFound` | 没有匹配的分支 / 未找到分支 | 6920 / 6934 |
| | `git.loadingWorktrees` / `git.noMatchingWorktrees` / `git.noWorktreesFound` | 正在加载工作树... / 没有匹配的工作树 / 未找到工作树 | 6936 / 6937 / 6938 |
| | `git.detectingRepositories` / `git.noMatchingRepositories` / `git.noRepositoriesFound` | 正在检测仓库... / 没有匹配的仓库 / 未找到仓库 | 6940 / 6941 / 6942 |
| 工作树行 | `git.detachedHead` / `git.noBranch` | 分离的 HEAD / 无分支 | 6950 / 6951 |
| 工作树行 / 底栏 | `git.createWorktree` | 创建工作树 “{path}” | 6939 |
| | `git.worktreeDialog.manage` | 管理工作树… | 6881 |
| 触发器父级（chrome 项 label） | `footer.gitBranch` | Git 分支 | 7318 |
| ahead/behind 的 tooltip | `git.aheadOfRemote` / `git.behindRemote` | 领先远端分支 {count} 个提交 / 落后远端分支 {count} 个提交 | 6918 / 6919 |
| 切换相关 toast | `git.stashChanges` | 贮藏更改 | 6958 |
| | `git.switchingTo` | 正在切换到 {branch} | 6967 |
| | `git.stashAndSwitchSuccess` | 已贮藏更改并成功切换分支 | 6959 |
| | `git.stashAndSwitchFailed` | 贮藏后切换分支失败 | 6960 |
| | `git.stashFailed` | 贮藏更改失败 | 6961 |
| 合并/变基 | `git.mergeSuccess` | 已成功合并 {branch}。 | 6968 |
| | `git.conflictedFile` / `git.conflictedFiles` | 已停止，有 {count} 个冲突文件。请在更改列表中解决后继续。 | 6969 / 6970 |
| | `git.stoppedBeforeCompletion` | {branch} 在完成前已停止。请在更改列表中继续、跳过或中止。 | 6971 |
| | `git.uncommittedWouldOverwrite` | 未提交的更改会被覆盖：{listed}{more}。请先贮藏或提交。 | 6972 |
| | `git.moreFiles` | （另有 {count} 个） | 6973 |
| 确认框 | `git.deleteBranchConfirm` | 确定要删除分支 “{branch}” 吗？ | 6957 |
| | `git.delete` | 删除 | 6956 |
| | `git.merge` / `git.rebase` | 合并 / 变基 | 6974 / 6975 |
| | `git.mergeBranchTitle` / `git.rebaseBranchTitle` | 合并分支 / 变基分支 | 6976 / 6977 |
| | `git.mergeConfirm` | 将分支 “{branch}” 合并到 “{current}”？冲突可能需要手动解决。 | 6978 |
| | `git.rebaseConfirm` | 将 “{current}” 变基到 “{branch}”？冲突可能需要手动解决。 | 6979 |
| 确认框默认按钮 | `ui.cancel` / `ui.confirm` | 取消 / 确认 | 4609 / 4611 |
| 仓库页签内联错误 | `git.selectedFolderNotRepo` | 所选文件夹不在 Git 仓库内。 | 6982 |
| | `git.failedToSelectRepository` | 选择仓库失败。 | 6983 |
| 合并/变基失败兜底 | `git.mergeFailed` / `git.rebaseFailed` | 合并失败。 / 变基失败。 | 6980 / 6981 |
| 命令面板入口（**英文硬编码，未走 i18n**） | — | `Git: Open Branch Manager` / `Git: Manage Worktrees` 等 | `command-palette/constants/git-actions.tsx:88,104,112` |

---

## 2. 三个页签的完整语义

三个页签**共用**搜索框、搜索词（`branchQuery` 一个状态）、页签下方的列表容器与键盘高亮；
切页签**不清空**搜索词，只把焦点交回搜索框（`:518-524`），高亮因 `resetKey` 变化而回到第 0 项（`:600`）。

### 2.1 `仓库`（`repositories`）

| 项 | 内容 |
| --- | --- |
| 列什么 | 工作区里**被发现到的、以及用户手工添加过的所有 Git 仓库**（`availableRepoPaths`），每项一行：目录名 + 相对工作区根的路径；当前仓库带「当前」徽章，手工添加的带「已添加」徽章（`:774-786,986-1002`） |
| 点击后发生什么 | **只切「当前仓库」，不切分支、不检出**：`selectRepository(path)` → 关面板 → 清空搜索词 → `onRepositoryChange(path)`（`:474-480`）；标题栏实例的 `onRepositoryChange` 会 `getGitStatus(repoPath)` 并写回 `workspaceGitStatus`（`footer-git-branch-item.tsx:57-65`） |
| 顶栏「添加」 | 打开系统目录选择器 → `resolveRepositoryPath`（不是仓库就提示 `git.selectedFolderNotRepo`）→ `setManualRepository`（`:482-506`） |
| 顶栏「清除已添加」 | `clearManualRepository()`，随后回调刷新当前仓库（`:508-512`） |
| 顶栏「刷新」 | `refreshWorkspaceRepositories()` 重新扫工作区（`:840-847`） |
| 危险操作 | **无**（不删仓库、不改仓库） |
| 是否可以从这里删掉某个仓库 | **不能**——「清除已添加」清的是"手工添加"这一整批标记 |

### 2.2 `分支`（`branches`）—— 默认页签

| 项 | 内容 |
| --- | --- |
| 列什么 | **本地分支名**（纯字符串列表），当前分支恒在第一位；顶部可能多一行「创建新分支 “x”」（`:65-76,699-712,713-727`） |
| 单击非当前分支 | **切换分支**（`handleBranchChange`）→ 见 §4.1 |
| 单击当前分支 | `handleBranchChange` 首行就 `return`（`branchName === currentBranch`），**什么都不发生**（`:282`） |
| 「…」→ 合并到当前分支 | 确认框 → `git.write{merge}`（`:399-440`） |
| 「…」→ 将当前分支变基到此分支 | 确认框 → `git.write{rebase}`（同上） |
| 「…」→ 删除分支 | 确认框 → `git_delete_branch` → Core `git.write{deleteBranch}`（`:345-363`；命令映射见 §4.3） |
| 顶部「创建新分支 “x”」或底栏「新建分支」 | `git.write{createBranch}`，**创建后不检出**（`:442-456`；见 §4.2） |
| 危险操作确认 | 删除、合并、变基各一次 `showConfirmDialog`（`ui/dialog.tsx:409-424`）：<br>· 删除：标题 `git.deleteBranch`（**删除分支**），正文 `git.deleteBranchConfirm`（**确定要删除分支 “{branch}” 吗？**），确认按钮 `git.delete`（**删除**），取消 `ui.cancel`（**取消**）<br>· 合并：标题 `git.mergeBranchTitle`（**合并分支**），正文 `git.mergeConfirm`，确认按钮 `git.merge`（**合并**）<br>· 变基：标题 `git.rebaseBranchTitle`（**变基分支**），正文 `git.rebaseConfirm`，确认按钮 `git.rebase`（**变基**） |
| **有没有「强制切换」** | **没有**。整个面板没有任何 `force`/`-f`/`checkout -f` 入口（面板源码里 grep `force` 无匹配） |
| **有没有「工作区脏就不许切分支」这类守卫** | **有，但不是硬禁止**：先跑 `git.checkoutPreflight`，若有 `blockingPaths` 就**不检出**、弹一条警告 toast，并给「贮藏更改」按钮让用户贮藏后自动重试（§4.1） |
| 有进行中的 merge/rebase 时 | 本面板**不感知** `git.operationState`（面板源码未 import），只在 `git.write{merge|rebase}` 返回里根据结果提示（`:365-397`）。真正的中止/继续入口在左栏的操作横幅（见 `08-source-control.md` §2.2④） |

### 2.3 `工作树`（`worktrees`）

| 项 | 内容 |
| --- | --- |
| 列什么 | `git.worktrees`（Core）里**可打开的**工作树（过滤掉 bare / prunable）：目录名 + 分支名；当前工作树置顶并带「当前」徽章（`:95-113,1007-1050`） |
| 单击某个工作树 | **把当前窗口的工作区切到那个工作树目录**（不是切分支）：<br>`handleWorktreeChange(path)` → 关面板 → `onWorktreeChange(path)`（`:458-466`）→ 标题栏实例调 `openGitWorktreeWorkspace(path)`（`footer-git-branch-item.tsx:47-56`）→ `useFileSystemStore.handleOpenFolderByPath(path)` + `useRepositoryStore.selectRepository(path)`（`utils/git-worktree-open.ts:16-36`），随后重新读该路径的 `git.status` |
| 单击**当前**工作树 | `worktreePath === repoPath` → 只关面板（`:459-462`） |
| 顶部「创建工作树 “path”」 | `showGitWorktreeDialog(repoPath, {destination})`，即打开**独立的工作树创建对话框**（`:468-472`），不是内联表单 |
| 底栏「管理工作树…」 | 同一个对话框，`destination` 为空（`:812-819`） |
| 删除 / 移除工作树 | **不在本面板**：在 `git-worktree-dialog.tsx:207`（`removeWorktree`），文案 `git.worktreeDialog.remove` = **移除工作树**，确认 `git.worktreeDialog.removeConfirm`（`locale.ts:6885,6886`） |
| 危险操作确认 | 只有对话框里的「移除工作树」确认；本面板内没有破坏性操作 |
| 打不开的工作树 | bare / prunable 的**直接不出现**，用户看不到也点不到（`utils/git-worktree-open.ts:12-14`） |

### 2.4 三个页签的对比小结

| 维度 | 仓库 | 分支 | 工作树 |
| --- | --- | --- | --- |
| 数据命令 | `useRepositoryStore`（工作区发现，非 git 命名空间） | `git_branches` → `git.history` | `git.worktrees` |
| 点击后果 | 换「当前仓库」（读状态） | **切分支**（改 HEAD） | **换工作区目录** |
| 有写操作 | 添加/清除仓库（本地标记） | 新建 / 切换 / 删除 / 合并 / 变基 | 创建 / 移除（在对话框里） |
| 确认框 | 无 | 3 处 | 1 处（在对话框里） |
| 搜索字段 | 目录名 + 路径 | 分支名 | 目录名 + 路径 + 分支名 + head 短哈希 |

---

## 3. 数据来源与刷新

### 3.1 分支列表的命令链

| 层 | 事实 | 证据 |
| --- | --- | --- |
| 前端包装 | `getBranches(repoPath)` → `tauriInvoke<string[]>("git_branches", { repoPath })`，返回类型就是**字符串数组** | `features/git/api/git-branches-api.ts:45-63`（尤其 `:54-55`） |
| Rust 翻译 | `"git_log" \| "git_branches" => "git.history"` | `windows/tauri/src-tauri/src/platform.rs:219` |
| 请求字段改名 | `repoPath` → `root`；`operationId` 被剥掉 | `platform.rs:205,207-209` |
| Core 命令 | **`git.history`**（兼容命令，一次返回 `references` + 第一页提交）；契约见 `shared/contracts/rust-core-api.md:191`、`:876-880` | `rust-core-api.md:191,876-880` |
| 返回值适配 | `case "git_branches": data.references.filter(kind === "local").map(shortName)` | `platform/core-result-adapter.ts:247-254` |
| 因此拿到的字段 | **只有本地分支名（`shortName`）**。没有：远端分支、tag、`isCurrent`、`ahead`/`behind`、`upstreamShortName`、最后提交信息 | 上两行 + `git-branches-api.ts:14-18` 的 `CheckoutResult` 里也没有 |
| 当前分支从哪来 | 不从分支列表来：由**父级传入 `currentBranch` prop**（来自 `git.status.branch`） | `git-branch-manager.tsx:49,143,188,282,717`；`footer-git-branch-item.tsx:25,34` |
| 排序 / 当前标记 | 全部**前端**做：`a === currentBranch` 置顶 + `localeCompare` | `git-branch-manager.tsx:65-76` |

> **可用的更优数据源**：Core 的 `git.references`（`rust-core-api.md:192,900-908`）一条命令就带
> `kind` / `isCurrent` / `upstreamShortName` / `ahead` / `behind` / `peelsToCommit`，
> 而且 Windows 自己也已经在用（`api/git-commits-api.ts:139-158` 的 `getGitReferences`）。
> `git_branches` 走 `git.history` 是**浪费**（顺带拉一页提交）。

### 3.2 其它数据

| 用途 | 命令 | 证据 |
| --- | --- | --- |
| 当前分支 + ahead/behind（触发器 + 计数徽章不显示数字） | `git.status`（`git_status`）→ `{ branch, ahead, behind, changes[] }` | `api/git-status-api.ts:34-79`；`core-result-adapter.ts:143-162`；`rust-core-api.md:172,284-287` |
| 工作树列表 | `git.worktrees`（`git_get_worktrees`；注意它走的是 `git.command` + `worktree list --porcelain`，**不是** Core 的 `git.worktrees`） | `api/git-worktrees-api.ts:25-43`；`platform.rs:522-527`；`rust-core-api.md:174,303-310` |
| 仓库列表 | `useRepositoryStore`（工作区发现），非 git 命名空间 | `git-branch-manager.tsx:167-171` |

### 3.3 刷新时机（**面板不订阅任何变更事件**）

| 时机 | 触发 | 证据 |
| --- | --- | --- |
| 打开面板 | `handleOpenDropdown()` → `Promise.all([loadBranches(), loadWorktrees()])`；同时 `setActiveTab("branches")` | `git-branch-manager.tsx:526-531` |
| 打开状态变化（含从命令面板打开） | `useEffect([repoPath, isDropdownOpen])` 再跑一次 `loadBranches()` + `loadWorktrees()` | `:251-256` |
| 仓库路径变化 | 清空 `branches`/`worktrees` 并让在途请求作废（`*LoadRequestIdRef`） | `:243-249,215-220,229-235` |
| 手动 | 底栏「刷新」（分支页签 → 只 `loadBranches()`；工作树页签 → 只 `loadWorktrees()`；仓库页签 → `refreshWorkspaceRepositories()`） | `:800-807,820-827,840-847` |
| 删除分支成功后 | `await loadBranches()` | `:357-359` |
| **没有**的时机 | ① **不监听** `lithe:git-changed`（该文件未 import `subscribeToGitChanges`）→ 外部 `git checkout` 后**已打开的面板不会刷新**；② **不监听** `git-metadata-changed` watcher（该 watcher 只刷 `useGitStore`，见 `08-source-control.md` §4.3②）；③ 「刷新」按钮**不会**重读 `git.status`，所以 aheah/behind 与触发器上的分支名靠各自的 `onBranchChange` 回调更新 | 全文件 grep 无 `git-changed` / `git-metadata`；`:800-807` 只调 `loadBranches` |

### 3.4 搜索是前端还是命令

**纯前端**：`getFilteredBranches(branches, activeBranch, branchQuery)`（`:65-76,191-194`）在 `useMemo` 里跑，
`matchesSearchQuery` 是本地字符串函数（`utils/search-match.ts:26-35`）。全程**不产生任何命令调用**。
计数徽章显示的也是**未过滤**的 `branches.length`（`:679`）。

---

## 4. 写操作与失败显示

### 4.1 切换分支（`handleBranchChange`，`:281-341`）

```
① checkoutBranch(repoPath, name)
   └─ localBranchReference(name) = "refs/heads/<name>"           api/git-branches-api.ts:42-43,65-69
   └─ invoke("git.checkoutPreflight", { repoPath, reference })   :79-82  → Core git.checkoutPreflight
      · 返回 { blocked, blockingPaths[] }，翻译层传的是 Core 的 { blockingPaths }（platform.rs:368-371）
      · 契约：shared/contracts/rust-core-api.md:200,814-818
   ├─ blockingPaths.length > 0 →
   │    return { success:false, hasChanges:true, message }（**不检出**）  :83-89
   │    message 是**英文硬编码**："Local changes would be overwritten by switching branches: …（+N more）"  :35-40
   └─ 否则 invoke("git.write", { repoPath, operation:"checkout", reference, referenceKind:"local" })  :91-96
        · Core 契约：rust-core-api.md:177,543-554,610-611
        · 成功后 emitGitChanged({ scopes:["working-tree","history","refs"] })  :97-101
② UI 反应
   ├─ hasChanges → 警告 toast（`type:"warning"`、`duration: 0`），action = 「贮藏更改」git.stashChanges  :288-327
   │    · 点它：createStash(repoPath, t("git.switchingTo",{branch}), true) → 再 checkoutBranch 一次
   │    · 成功 → 清 blame → success toast「已贮藏更改并成功切换分支」→ 关面板 → onBranchChange()  :296-311
   │    · 失败 → error toast「贮藏后切换分支失败」/「贮藏更改失败」                :312-323
   ├─ success → clearAllBlame() → 关面板 → onBranchChange()                          :328-331
   └─ 其它失败 → error toast，内容 = `result.message`（一般就是 Git 的原文）           :332-337
```

- **守卫是"先探测、后提示"，不是"硬拒绝"**：`checkoutPreflight` 只检查「本地已改 & HEAD 与目标不同」的已跟踪文件
  加上「目标引用会跟踪的未跟踪文件」（`rust-core-api.md:814-818`）。用户点「贮藏更改」就能继续。
- 整个过程中面板**不 disabled**（`isLoading` 期间行与按钮变灰：`:900,925,795`）。

### 4.2 新建分支（`handleCreateBranch`，`:442-456`）

| 项 | 事实 | 证据 |
| --- | --- | --- |
| 命令 | `git.write { operation:"createBranch", name, reference:"refs/heads/<当前分支>" }` | `api/git-branches-api.ts:123-147`（source 由 `from` 推出，`:130`） |
| **是否检出** | **不检出**：没有 `checkout: true`，Core 走 `git branch <name> <ref>`（`rust/lithe-core/src/git/mod.rs:1011-1019`；`checkout: true` 时才 `git switch -c`） | 同上 |
| 参数校验 | 分支名合法性由 Core `validated_branch_name` 校验，非法 → `invalid_request`（`mod.rs:1012`） | — |
| 成功后 | 清空搜索词 → **关面板** → `onBranchChange()` → 触发器重读 `git.status` | `git-branch-manager.tsx:448-452` |
| 事件 | `emitGitChanged({ scopes:["refs"], source:"create-branch" })` | `api/git-branches-api.ts:137-141` |
| **失败显示** | **没有 toast**：`createBranch` 内部 `catch` 只 `console.error` 并返回 `false`，调用方在 `success === false` 时**什么都不做**（不关面板、不提示） | `api/git-branches-api.ts:143-146`；`git-branch-manager.tsx:446-455` |
| 另有一个「创建并检出」的 API | `createAndCheckoutBranch`（`operation:"createBranch"` + `checkout:true`，`git-branches-api.ts:149-167`），**本面板没用** | — |

### 4.3 删除分支（`handleDeleteBranch`，`:345-363`）

| 项 | 事实 | 证据 |
| --- | --- | --- |
| 前置 | `branchName === currentBranch` → 直接 return；且当前分支行**根本没有**「…」按钮 | `:346`；`:910-911` |
| 确认 | `showConfirmDialog(git.deleteBranchConfirm, { title: git.deleteBranch, confirmLabel: git.delete })` | `:348-351`；`ui/dialog.tsx:409-424` |
| 命令 | `deleteBranch(repoPath, name)` → `invoke("git_delete_branch", { repoPath, branchName })` → `git.write { operation:"deleteBranch", reference:"refs/heads/<name>" }` | `api/git-branches-api.ts:242-256`；`platform.rs:339-344` |
| Core 侧安全网（三层，都会失败而不是强删） | ① 当前分支 → `invalid_request`「The current branch cannot be deleted」；② 被任一工作树检出 → 「The branch '…' is checked out in a worktree」；③ **未合并**（无可用 upstream 时对比 HEAD）→ 拒绝 | `rust/lithe-core/src/git/mod.rs:1057-1067`；`:4374-4424`；`rust-core-api.md:797-805` |
| 成功后的记录 | Core 返回 `branchDeletion { name, deletedTarget }`，**供宿主提供"恢复分支"**；本面板**没有**做恢复入口 | `rust-core-api.md:797-805` |
| 成功后 | `await loadBranches()` 重读列表 | `:356-359` |
| **失败显示** | **没有 toast**（同 4.2）：`deleteBranch` 只 `console.error` + 返回 `false`；调用方 `if (success)` 之外没有任何 else | `api/git-branches-api.ts:252-255`；`git-branch-manager.tsx:356-362` |
| 有没有强制删除（`-D`） | **没有**。`deleteBranch` 请求不带 `force`，Core 侧 `deleteBranch` 也不读 `force` | `api/git-branches-api.ts:242-256`；`rust/lithe-core/src/git/mod.rs:1057-1067` |

### 4.4 合并 / 变基（`handleIntegration`，`:399-440`）

| 项 | 事实 | 证据 |
| --- | --- | --- |
| 前置 | 无仓库 / 无 `currentBranch` / 同名分支 → return | `:400` |
| 确认 | `mergeConfirm` 或 `rebaseConfirm`；标题 `mergeBranchTitle` / `rebaseBranchTitle`；确认按钮 `git.merge` / `git.rebase` | `:402-411` |
| 命令 | `mergeBranch` / `rebaseOntoBranch`（`api/git-integration-api.ts`）→ `git_merge` / `git_rebase` → `git.write { operation:"merge" \| "rebase", reference }` | `platform.rs:372-383`；`rust-core-api.md:543-554` |
| 结果提示（按 `IntegrationOutcome.status`） | `clean` → success `git.mergeSuccess`；`conflicts` → 警告 `git.conflictedFile(s)`（6s）；`stopped` → 警告 `git.stoppedBeforeCompletion`（6s）；`blocked` → 警告 `git.uncommittedWouldOverwrite`（列前 3 个 + `git.moreFiles`，6s）；其它 → error `outcome.message` | `:365-397` |
| 抛异常 | error toast，用 `error.message` 或 `git.mergeFailed` / `git.rebaseFailed` | `:427-436` |
| 成功后 | `clean`/`conflicts`/`stopped` 都调 `onBranchChange()` 刷状态 | `:420-426` |
| 前置脏树检查 | 本面板**不调** `git.integrationPreflight`（只有 Core 层在 rebase 系操作里要求干净工作区，`rust-core-api.md:827-832`）；脏树时会由 Git 自己报错 → error toast | — |

### 4.5 「刷新」（底栏）

| 事实 | 证据 |
| --- | --- |
| 不是写操作、不发写命令 | `:800-807`（分支）、`:820-827`（工作树）、`:840-847`（仓库） |
| 分支页签只重读 `getBranches`（= `git.history`），**不重读 `git.status`** | `:212-224,800-807` |
| 失败时：`getBranches` 内部吞掉错误并返回 `[]`（不是仓库除外）→ 列表**静默变空**，UI 显示「未找到分支」 | `api/git-branches-api.ts:57-62`；`git-branch-manager.tsx:696` |

### 4.6 失败的统一显示方式

| 通道 | 说明 | 证据 |
| --- | --- | --- |
| **Toast（sonner）** | 面板内**没有错误条**。切换/合并/变基/贮藏的失败与警告都走 `showToast({ message, type, duration?, action? })` | `features/layout/contexts/toast-context.tsx:5-48`；`git-branch-manager.tsx:289-337,365-397,428-436` |
| 内联文本（仅仓库页签） | `selectionError` → `ui-text-sm min-w-0 flex-1 truncate text-destructive/90`，位置在底栏按钮之后 | `:853-857` |
| **静默（缺陷）** | 新建分支失败、删除分支失败：`console.error` + 返回 `false`，UI 无任何反馈 | `api/git-branches-api.ts:143-146,252-255` |
| 列表加载失败 | `console.error`（非仓库错误被过滤）+ 空列表 | `:221-223,229-240`；`api/git-branches-api.ts:57-62` |

---

## 5. 标题栏那一项的显示规则（逐条确认）

| 场景 | 显示什么 | 证据 |
| --- | --- | --- |
| **没有仓库** | **整项不渲染**（`useFooterGitBranchItem` 返回 `null`）。具体条件：`!footerRepoPath \|\| !footerBranch`；`footerBranch` 来自 `git.status.branch`，非仓库时 Core 给 `null`、适配器给 `""` → falsy | `footer-git-branch-item.tsx:20-27`；`platform/core-result-adapter.ts:145`；`rust/lithe-core/src/git/mod.rs:6537-6543` |
| 有仓库但 HEAD **分离** | 显示**分支名 = 英文常量 `detached`**。Core：`git branch --show-current` 输出为空时 `.or_else(\|\| Some("detached".to_string()))` | `rust/lithe-core/src/git/mod.rs:6549-6553` |
| | ⚠️ i18n 里的 `git.detachedHead` = **分离的 HEAD** 只用于**工作树行**的分支名回退，**不是**标题栏 | `git-branch-manager.tsx:88-93`；`locale.ts:6950` |
| detached 时的 ahead/behind | `git.status` 的 ahead/behind 来自 `@{upstream}...HEAD`，无 upstream → `(0,0)` → `GitTrackingCounts` 返回 `null` → **不画箭头** | `rust/lithe-core/src/git/mod.rs:6582-6599`；`git-tracking-counts.tsx:33` |
| `GitBranchManager` 自身的兜底 | `if (!currentBranch) return null;`——**整个触发器 + 浮层都不渲染**（`currentBranch` 是 prop，空串也算） | `git-branch-manager.tsx:604-606` |
| 分支名过长 | `truncate` + `maxWidth = min(max(len+1, 6), 40)ch`，即最多 40 个字符宽 | `:189-190,648-653` |
| **ahead/behind 数字画在哪** | **标题栏那一项不画数字**：`showCounts={false}` → 只输出 `↙` / `↗` 两个箭头（10px，`--info` 与 `--git-added` 上色），两个都为 0 时整个 `null` | `:654-660`；`git-tracking-counts.tsx:33,36-53` |
| 数字在别的三处 | ① **底部「提交记录」的引用树**：每个有 upstream 的引用行右侧显示 `↙N ↗N`（`showCounts` 默认 true）+「当前」徽章 | `log/git-reference-tree.tsx:641-656`；`git-tracking-counts.tsx:16-23` |
| | ② **左栏提交面板底部**：ahead 按钮（`ArrowUp` + 数字，`text-git-added`，tooltip 英文硬编码 `Push N commit(s)`，点击推送）与 behind 按钮（`ArrowDown` + 数字，`text-git-deleted`，`Pull N commit(s)`，点击拉取） | `git-commit-panel.tsx:305-333` |
| | ③ 左栏 `history` 子页签顶部一行文字：**{count} 个本地提交未推送** / **{count} 个远程提交未拉取** | `git-commit-history.tsx:442-457`；键 `git.localCommitsNotPushed` / `git.remoteCommitsNotPulled` |
| 数字上限 | `MAX_VISIBLE_TRACKING_COUNT = 99` → `99+` | `git-tracking-counts.tsx:3,9-14` |
| 触发器的 tooltip / 无障碍 | 只有 `aria-label`（**搜索分支**）；`GitTrackingCounts` 的两个 span 各自带 `aria-label`/`title`（**领先远端分支 {count} 个提交** / **落后远端分支 {count} 个提交**）——所以数字虽不显示，读屏仍能读到计数 | `:645`；`git-tracking-counts.tsx:43,49`；`locale.ts:6918,6919` |
| 项目菜单的 `▾` 与分支项的关系 | 同一个 `ChromeGroup gap="tight"`（2px）内，项目菜单在左（末子节点是 `ChevronDownIcon size-3.5`，展开时 `rotate-180`），分支项在右 | `title-bar.tsx:236-238`；`title-project-menu.tsx:157-163` |

---

## 6. gpui 侧落地评估

### 6.1 我们已有的能力（逐条）

| 能力 | 现状 | 证据 |
| --- | --- | --- |
| 已通的 Core 命令 | `git.status` / `git.references` / `git.historyPage` / `git.historyCursorClose` / `git.commitFiles`（外加兜底用的 `workspace.snapshot`） | `gpui/crates/git/src/lib.rs:89-98,93`（`workspace.snapshot`） |
| 统一信封 | `model::execute_core(command, payload)` —— 加新命令只是加一个调用点，**基础设施已就绪** | `gpui/crates/git/src/model.rs:454-470` |
| 引用解析（**本面板的关键复用点**） | `parse_references` → `Reference { full_name, short_name, kind, is_current, upstream_short_name, ahead, behind }` | `model.rs:537-569`；结构体 `:295-303` |
| 引用种类 | `RefKind::{Local, Remote, Tag}` + `parse`/`id`/`title`/`sections` | `model.rs:253-291` |
| 当前分支 | `read_status` → `(Option<仓库根>, Option<分支名>)`（**只取 root 与 branch，没取 ahead/behind**） | `model.rs:493-500` |
| 仓库根修正 | `resolve_repository_root`（Core 可能给相对路径） | `model.rs:476-483` |
| 非仓库兜底 | `discover_repository_root`（`workspace.snapshot` 一级目录 + `git.status` 探测，上限 6） | `model.rs:508-534` |
| ahead/behind 格式化 | `tracking_count` + `TRACKING_COUNT_MAX = 99` | `model.rs:1139-1145,163` |
| 面板宿主 | `Root::render_dialog_layer / sheet / notification` **已经挂在** `ShellWorkspace::render` | `gpui/crates/workbench/src/workspace.rs:792-794` |
| 浮层 + 搜索列表的**现成先例** | `command_palette.rs`：`window.open_dialog` + `Command` + `CommandItem::child` 自绘行 + `on_confirm(IndexPath)` + `PENDING_FOCUS` 抢焦点 + `SHELL` 弱句柄 | `gpui/crates/workbench/src/command_palette.rs:87-96,304-322,361-412,423-483` |
| 错误/成功提示 | `window.push_notification(Notification::error/info(...))`；宿主层已挂 | `gpui/crates/editor/src/editor_view.rs:735-736,882-884`；`workspace.rs:794` |
| 行内「…」菜单 | `Button::dropdown_menu(...)` + `PopupMenuItem::new(..).on_click(..)` 已有先例 | `gpui/crates/git/src/log_view.rs:668-689` |
| 标题栏 | **无状态**：`title_bar(project_name, window, cx)` 只画「项目名 + 拖拽区 + 三键」，**没有分支项** | `gpui/crates/workbench/src/title_bar.rs:76-97` |
| 当前分支在 gpui 里的来源 | 状态栏第 2 项来自**直读 `.git/HEAD`** 的临时实现（detached 时返回 `None` → 显示 `—`） | `workspace.rs:500,515-521`；实现 `:1121-1129` |
| 已有分支相关文案 | `lithe.git.current`=当前（`shared/src/i18n.rs:205`）、`lithe.git.log.toolbar.newBranch`=新建分支（`:204`）、`lithe.git.noRepositoryOpen`（`:226`）、`lithe.git.log.refresh`=刷新 Git 日志（`:184`，**语义不同**）、`lithe.git.log.filterBranch`=分支（`:234`，是过滤器字段名） | `gpui/crates/shared/src/i18n.rs:181-264` |

### 6.2 第一版能做什么 / 缺什么命令

**能做（零新 Core 命令）**

| 面板元素 | 数据来源 | 说明 |
| --- | --- | --- |
| 标题栏触发器：分支图标 + 分支名 | `git.status.branch`（已有 `read_status`） | 需要把 `ahead`/`behind` 也读出来（现在 `read_status` 只返回两个字段，`model.rs:493-500`；Core 响应里本来就有，`rust-core-api.md:284-287`） |
| ahead/behind 箭头（不画数字） | 同上 | `tracking_count` 已有上限逻辑；`showCounts=false` 的等价物就是只画 `↙`/`↗` |
| 分支列表 | **`git.references`** → 过滤 `RefKind::Local` | 比 Windows 走的 `git.history` 更准：一条命令就带 `is_current` / `ahead` / `behind` / `upstream_short_name` |
| 当前分支高亮 + 右侧「当前」徽章 | `Reference::is_current` | — |
| 排序（当前置顶 + 字典序） | 纯前端 | 照 `getFilteredBranches` 的前半段（`git-branch-manager.tsx:66-70`） |
| 搜索过滤 | 纯前端 | 照 `matchesSearchQuery`（`utils/search-match.ts:1-35`）：NFKD + 去音标 + 小写 + 非字母数字折叠 + 紧凑串双匹配 |
| 计数徽章「{n} 个分支」 | `references` 过滤后的长度 | 需要新增 i18n 键 |
| 刷新 | 重跑 `git.references` | 语义与 Windows 的「刷新」等价（Windows 那一版只重读分支名） |
| 空态「没有匹配的分支」/「未找到分支」 | — | 需要新增 i18n 键 |

**缺的命令（逐条）**

| 功能 | 缺什么 | Core 契约（已有） | gpui 现状 |
| --- | --- | --- | --- |
| 切换分支 | ① `git.checkoutPreflight`（脏树守卫）② `git.write { operation:"checkout", reference, referenceKind }` | `rust-core-api.md:200,814-818`；`:177,543-554,610-611` | **一条都没调** |
| 新建分支 | `git.write { operation:"createBranch", name, reference }` | `rust-core-api.md:545`；`rust/lithe-core/src/git/mod.rs:1011-1019` | 无 |
| 删除分支 | `git.write { operation:"deleteBranch", reference }`（返回里有 `branchDeletion` 供恢复） | `rust-core-api.md:797-805` | 无 |
| 合并 / 变基 | `git.write { operation:"merge" \| "rebase", reference }`（+ 可选 `git.integrationPreflight`） | `rust-core-api.md:546`；`:827-832` | 无 |
| 切换前贮藏 | `git.write { operation:"stashPush", message, includeUntracked }` | `rust-core-api.md:548` | 无 |
| 工作树页签 | `git.worktrees` + `git.write { operation:"createWorktree" \| "removeWorktree" }` | `rust-core-api.md:174,303-310,546-547,560-580` | 无（`git.worktrees` 这条连调用点都没有） |
| 仓库页签 | `workspace.repositories`（**非 git 命名空间**） | `rust-core-api.md:288-291` | 无（gpui 只有 `workspace.snapshot` 的一级目录探测） |

> 注意：**「缺命令」= 缺 gpui 侧的调用点**，不是缺基础设施。`execute_core` 是通用信封
> （`model.rs:454-470`），`git.write` 的请求体在契约里已经完整定义（`rust-core-api.md:543-554`）。
> 真正的工作量在**结果解释**：`git.write` 返回的是进程结果
> （`arguments/output/stdout/stderr/exitCode/invocations/operationError/stashRestore/warnings`，
> `rust-core-api.md:588-609`），"失败" 不等于 `exitCode != 0` —— 必须同时看 `operationError`
> 与 `warnings`，否则会把「提交成功但暂存区没刷新」这类**部分成功**误报成失败。

### 6.3 组件选型

#### 结论表

| 需求 | `PopupMenu` 能做吗 | 说明 |
| --- | --- | --- |
| 单行项 + 右侧标记 | **能**（但没必要） | `PopupMenuItem::element(Fn(&mut Window,&mut App) -> E)`（`menu/popup_menu.rs:85-98`）可完全自绘；`render_item` 的 ElementItem 分支把内容放进 `h_flex().flex_1().min_h(item_height).items_center().gap_x_1()`（`:1263-1290`），`item_height` 默认 **26**（`:1221-1224`），外层 `px(8) py(0)` + 圆角（`:1226-1232`）。所以「多行项 + 右侧标记」**可以**用一个 `w_full().justify_between()` 的自绘子元素实现 |
| **搜索框** | **不能** | 全文件（1533 行）grep `Input` / `search` **零匹配** → `PopupMenu` 里没有任何输入控件 |
| **页签行 / 底栏** | **不能** | `PopupMenu` 只有 `PopupMenuItem` 的四种形态（Separator / Label / Item / ElementItem / Submenu，`:33-65`），没有 header/footer 槽位；`min_w`/`max_w`/`max_h`/`scrollable`（`:429-450`）都不解决这个问题 |
| **点一下不关菜单** | **不能** | 任一可点项确认后都会 `self.dismiss(&Cancel, …)`（`:852-890` 的 `on_click` → `confirm`）→ 无法做"点页签只切页签" |
| **面板浮层** | **不适合** | 它是 Trigger/ContextMenu 挂出来的**菜单**，没有遮罩、没有焦点陷阱、没有居中入口 |
| 行内「…」下拉 | **正解** | 唯一合适的位置：每个非当前分支行右侧一个 `Button::dropdown_menu`，与 `log_view.rs:668-689` 的字段下拉同一形态 |

#### 推荐：`Dialog` + `Command`（与仓库里的命令面板同一套）

| 结论 | 证据 |
| --- | --- |
| `Command` 必须放进别的容器（首选 `Dialog`）——它自己是普通流式 `v_flex`，没有遮罩/定位/关闭语义 | `gpui-component-0.6.6/src/command/state.rs:819-837`；`command/command.rs:200-207`（`bordered(false)` 的文档就写着"已在 Dialog 框里时关掉"） |
| `Dialog` 负责遮罩 / Esc / 点外关 / 焦点陷阱 / 焦点归还 | `gpui/research/gpui-kit-overlay-howto.md` §2 路径 A + §5；仓库里已落地的用法 `command_palette.rs:308-322` |
| 宿主层已在，不需要新增挂载点 | `gpui/crates/workbench/src/workspace.rs:792-794` |
| 「多行项 + 右侧标记」怎么做 | `CommandItem::child(Fn(&mut Window,&mut App) -> E)`（`command/item.rs:95-104`）替换整行内容；**注意**：一旦用 `child`，`.icon()` 与 `.label()` **不再渲染**（`state.rs:731-742`），`.checked(true)` 的勾也只在**没有** `child` 时才会画（`state.rs:762-769` 只在 `binding` 为 `None` 且 `item.checked` 时 `ml_auto` 加勾）→ **前导勾、标签、右侧「当前」徽章都必须画在 `child` 里面**（`h_flex().w_full().min_w_0().justify_between()`）。仓库里的自绘两行行是现成范例：`command_palette.rs:446-481` |
| 行高必须自己给下限 | `command_palette.rs:451-453` 已记录：不设 `min_h` 时自绘内容会让行高忽高忽低；这里对应 Windows 的 `min-h-9` = **36** |
| 搜索 / 过滤 / 键盘高亮 / 空态 / 虚拟列表 | 组件自带，不用重做（`state.rs:841-858,689-788,903-931`） |

#### 唯一的结构性缺口：**页签的位置**

`Command` 的渲染顺序是 **header → 搜索框 → 列表 → footer**（`command/state.rs:838-935`），
而 Windows 的顺序是 **搜索框（+ 计数徽章 + 关闭按钮）→ 页签 → 列表 → 底部按钮**。
`Command::header`（`command/command.rs:210-219`）画在**搜索框上方**，**没有**"搜索框下方、列表上方"的槽位。
另外内置搜索行的结构是 `div.px_3().border_b_1().child(Input…)`（`state.rs:842-857`），
**没有**放「{n} 个分支」徽章与关闭按钮的位置。

两条落地路线：

| 路线 | 做法 | 代价 |
| --- | --- | --- |
| **A（推荐给第一版）** | 先**不做页签行**。`.searchable(true)` + `.placeholder(搜索分支...)` + `CommandItem::child` 自绘行 + `.footer(刷新)`。搜索框右侧的计数徽章 / 关闭按钮暂缺（关闭用 Esc / 点遮罩 / Dialog 自带关闭按钮） | 与截图有 3 处可见差异（页签、计数徽章、面板内关闭按钮），但数据与交互语义完整 |
| **B（完整复刻顺序）** | 自己持有 `InputState`（或直接自绘输入框）放在 `header()` 里，把「搜索行 + 计数徽章 + 关闭按钮 + 页签行」整段画在 header；`Command` 用 `.searchable(false)`，`.items()` 直接喂**已经过滤好的**行，过滤逻辑用 §6.2 的纯函数自己做 | 多写约 60~100 行；换来与真源一一对应的四段结构。注意 `.searchable(false)` 会让 `CommandState::focus` 去 focus 自己而不是输入框（`state.rs:791-799`），抢焦点那一步要改成 focus 自己的 `InputState` |

> `Command` **没有宽度 API**（只写 `w_full()`，`state.rs:828`），宽度必须由 `Dialog::width(...)` 给：
> Windows 是 `min(44rem, 100vw-2rem)` = **≤704**，`Dialog` 默认 448（`gpui-kit-overlay-howto.md` §2 路径 A），
> 而 gpui 没有 `impl From<Rems> for Pixels`，要照 `command_palette.rs:57-67` 的做法用
> `AbsoluteLength::to_pixels(rem_size)` 把 `rems(44.)` 转成 `Pixels`。列表 `max_h` 默认 300，
> Windows 面板上限是 **512**，需要 `.max_h(...)` 放开（对照 `command_palette.rs:383-386`）。

### 6.4 标题栏接线方式

| 方案 | 说明 | 取舍 |
| --- | --- | --- |
| (a) 改 `title_bar` 签名，加一个「分支项」参数 + 回调 | `title_bar(project_name, branch, tracking, on_open, window, cx)` | 最直接，但 `title_bar.rs` 现在刻意保持**无状态**（模块头 `:71-75`），加回调会破坏这个边界 |
| (b) 复用命令面板那一套**全局句柄** | `ShellWorkspace` 持有分支数据 + `Entity<CommandState>`，用 `command_palette.rs:106-152` 的 `SHELL` / `SHELL_FOCUS` 模式，让分支项在点击回调里直接调工作台的方法 | 不改 `title_bar` 签名；与既有先例一致 |
| (c) 加一条 action | `Ctrl+Shift+B` 之类的全局 action 打开分支面板（真源**没有**这个快捷键，属新增） | 不建议当第一版的**唯一**入口 |

⚠️ 无论哪条路，**分支项必须放在拖拽区之外**：`title_bar.rs:99-136` 的 `drag_region` 挂了
`WindowControlArea::Drag`，注释 `:101-107` 明确写过"若把 Drag 挂在祖先上会盖住子节点"。
分支项应当与 `window_controls` 一样是 `h_flex` 内的**兄弟**节点。

---

## 7. 第一版范围建议与未确认点

### 7.1 第一版范围建议（分四档，逐档可独立验收）

| 档 | 内容 | 需要的 Core 命令 | 依据 |
| --- | --- | --- | --- |
| **A. 只读分支面板**（建议本次做） | ① 标题栏分支项：`git-branch` 图标 + 分支名 + ahead/behind 箭头（不画数字）；没有仓库 / detached 时按 §5 的规则处理<br>② 点击弹出 `Dialog` + `Command`：搜索框（**搜索分支...**）、一行一条本地分支（前导勾 / 分支图标 + 分支名 + 右侧「当前」徽章）、空态两句、底栏「刷新」<br>③ 搜索是前端过滤；计数徽章与内联关闭按钮**先不做**（路线 A） | **零新增**：`git.status`（分支名 + ahead/behind）+ `git.references`（本地分支 + isCurrent） | §6.2；`model.rs:537-569` |
| **B. 切换分支** | 行点击 → `git.checkoutPreflight` → 有阻塞就 `Notification` 提示（先不做"贮藏并重试"，那要 `stashPush`）→ 否则 `git.write{checkout}`；成功后刷 `git.status` + `git.references`；失败用 `Notification::error` 显示 `operationError` / `stderr` 原文 | 新增 `git.checkoutPreflight` + `git.write{checkout}` | `rust-core-api.md:200,610-611,814-818` |
| **C. 写操作 + 危险确认** | 新建分支（照真源**创建后不检出**）、删除分支（先确认，处理 Core 的三层拒绝：「当前分支不能删」「被工作树占用」「未合并」；`branchDeletion` 可用于后续做"恢复分支"）、行内「…」`PopupMenu`、合并/变基（确认框 + 按 `mergeSuccess` / 冲突数提示） | `createBranch` / `deleteBranch` / `merge` / `rebase`（+ 可选 `integrationPreflight`、`stashPush`） | §2.2、§4.3、§4.4 |
| **D. 其余两个页签** | 工作树页签（`git.worktrees` + 打开为工作区 + `createWorktree`/`removeWorktree`）、仓库页签（`workspace.repositories`） | `git.worktrees`、`git.write{createWorktree,removeWorktree}`、`workspace.repositories` | §2.1、§2.3 |

**给 A 档的两条实现注意**

1. **新 i18n 键**：`gpui/crates/shared/src/i18n.rs` 现有表里**没有**「搜索分支...」「{count} 个分支」「没有匹配的分支」「未找到分支」「刷新」。可复用的只有 `lithe.git.current`=当前（`:205`）。A 档至少需要新增 5 个键（键名按现有 `lithe.git.*` 习惯；真源的中文逐字取自 `locale.ts:6907-6944`）。
2. **分支名的宽度夹取**：真源用 `min(max(len+1,6),40)ch`（`git-branch-manager.tsx:189-190`）。gpui 没有 `ch` 单位，第一版建议直接给一个像素上限（如 `max_w`）+ `truncate`，并在文档里登记这个偏离。

### 7.2 未确认的点

| # | 未确认的事 | 已知证据 | 需要谁拍板 |
| --- | --- | --- | --- |
| 1 | **截图里的 `▾` 到底属于谁** | 分支触发器**没有** caret，子节点只有图标 / 分支名 / ahead-behind（`git-branch-manager.tsx:646-661`）；`▾` 是左邻 `TitleProjectMenu` 的 `ChevronDownIcon size-3.5`（`title-project-menu.tsx:157-163`），两者相距 2px（`title-bar.tsx:236-238`） | 维护者：若确要分支项也带 `▾`，那是**新增**（真源没有），需明确登记 |
| 2 | 面板的无障碍标题 | `GitCommandSurface` 不传 `title` → 落到 `t("commandPalette.title")` = **命令面板**（`ui/command.tsx:174-176`） | 是缺陷还是可接受 |
| 3 | detached HEAD 时标题栏显示英文 `detached` | Core 硬编码 `Some("detached".to_string())`（`rust/lithe-core/src/git/mod.rs:6549-6553`）；`git.detachedHead`=分离的 HEAD 只用于工作树行（`git-branch-manager.tsx:88-93`） | 是缺陷还是有意（gpui 侧要不要改文案） |
| 4 | 分支列表该不该继续走 `git.history` | `platform.rs:219` 把 `git_branches` 映射到 `git.history`（顺带拉首页提交）；`git.references` 字段更全（`rust-core-api.md:900-908`） | gpui 用 `git.references` 是否算"与真源不一致" |
| 5 | 新建 / 删除分支失败**完全静默** | `git-branches-api.ts:143-146,252-255` 只 `console.error` + 返回 `false`；调用方在 `false` 时不做任何事（`git-branch-manager.tsx:446-455,356-362`） | 是缺陷还是有意；gpui 侧应不应该显示 `Notification::error` |
| 6 | 「刷新」只重读分支名，不重读 `git.status` | `:800-807`；触发器的分支名/ahead-behind 靠 `onBranchChange` 更新（`footer-git-branch-item.tsx:40-46`） | gpui 第一版是否照此（还是刷新时一并重读 `git.status`，更符合直觉） |
| 7 | 面板**不订阅**任何变更事件 | 全文件无 `lithe:git-changed` / `git-metadata-changed`；只在打开与手动刷新时读（`:212-256`）→ 外部 `git checkout` 后已开面板不更新 | 是缺陷还是有意 |
| 8 | 在 `Command` 自绘行里嵌「…」下拉的**事件冲突** | `Command` 行自身绑定 `on_click`（`command/state.rs:750-760`），`PopupMenu` 也消费点击并 `stop_propagation`（`popup_menu.rs:852-854`）。真源靠 `onPointerDown/onClick` 上的 `stopPropagation` 隔离（`git-branch-manager.tsx:914-918`） | 需实测：gpui 里点「…」会不会同时把整行也确认掉 |

---

## 8. 一页速查（给实现者）

```
标题栏（40 高，px 8，gap 4，组内 gap 2）
└─ [项目菜单 ▾]  [⎇ 分支名 ↙↗]              ← 分支项：h 24、px 8、13px、hover bg-accent/80
                                               分支名 maxWidth ≤40ch、truncate
                                               ahead/behind 只有箭头（10px），都为 0 则无

点击 → 模态浮层（宽 ≤704、高 ≤512、圆角 11.2、距顶 64）
├─ 搜索行（px 16、py 12、gap 8、下边框）
│   ├─ 输入框 h 28、13px，placeholder「搜索分支...」
│   ├─ 徽章「{n} 个分支」（≥28 高、≤160 宽、胶囊、bg-surface/70）   ← n 是总数，不是过滤后
│   └─ 关闭按钮（28 高、图标 16、aria「关闭命令面板」）
├─ 页签行（px 8、pt 8）：[仓库] [分支] [工作树]   胶囊 min-h 32、px 12、图标 14、默认选中「分支」
├─ 列表（内边距 8；行 ≥36 高、圆角 8、px 10、py 8、行间距 8）
│   ├─ 分支行：前导勾(当前,绿)/分支图标 + 分支名 + 「当前」徽章(绿) + 「…」(非当前,24×24)
│   │          文字：当前 text-foreground，其它 text-subtle-foreground
│   ├─ 仓库行：前导勾/FolderOpen + 目录名 + 相对路径同行 + 「当前」/「已添加」徽章
│   └─ 工作树行：前导勾/Nodes + 目录名 + ⎇ 分支名同行 + 「当前」徽章（bare/prunable 不列）
└─ 底栏（px 12、py 12、gap 8、上边框）
    · 分支： [新建分支] [刷新]        ← 新建分支在搜索词不是合法新分支名时禁用
    · 工作树：[管理工作树…] [刷新]
    · 仓库： [添加] [刷新] (清除已添加) (内联错误)
```

**数据一句话**：分支名与 ahead/behind 来自 `git.status`；分支列表来自 `git_branches`（真实落到 Core 的
`git.history`，只取本地分支名）；工作树来自 `git.worktrees`；仓库列表来自 `useRepositoryStore`；
面板自身**不订阅任何变更事件**，只在打开与手动刷新时读；搜索是纯前端过滤。
