# 07 · 设置界面规格（Windows 前端 → Rust + gpui-kit 0.6.6）

> **规格来源**：`windows/tauri/src/`（Tauri v2 + React 19 + TS + Tailwind v4 + Base UI 原语）。**不引用 `macos/` 任何内容**。
> **证据约定**：每条结论标注 `相对路径:行号`（相对仓库根 `D:\developmentProjects\rust\Lithe-IDEA`）。查不到写「未找到」，不猜。
> **本次为只读调查**，未修改任何产品代码；唯一产物是本文件。
> **与既有调研的分工**：`01-shell.md`…`06-database-ai-search.md` 与 `gpui/UI-MAP-WINDOWS.md` 已写清外壳/编辑区/Git/主题 token/终端/数据库/AI 聊天；本文**只聚焦设置界面本身**，token 明细沿用 `04-theme-and-components.md`，不重复。

---

## 0. 结论速览（给实施者）

1. **设置是「模态对话框」而不是「页面」**：820×620 固定尺寸 + 遮罩 + 左侧 190px 分类栏 + 右侧滚动内容区，底部「恢复默认设置 / 完成」（`features/settings/components/settings-dialog.tsx:88-157`）。没有 Sheet、没有独立窗口。
2. **⚠️ 「页签清单」与任务书假设不一致，必须以真实代码为准**：任务书点名的 9 个 `tabs/*-settings.tsx`（general / appearance / editor / keyboard / terminal / file-tree / git / ai / advanced）中，**只有 `git-settings.tsx` 与 `ai-settings.tsx` 被真实对话框使用**；对话框的分类表是 `MacSettingsCategory`，**12 个分类**（`features/settings/components/settings-dialog.tsx:35-48`），与 `types/settings.types.ts:26-35` 的 `SettingsSection`（9 值，含 `file-explorer`）**不是同一套**。
3. **`tabs/` 目录里的 7 个「经典页签」组件是死代码**：`general-settings.tsx`、`appearance-settings.tsx`、`editor-settings.tsx`、`keyboard-settings.tsx`、`terminal-settings.tsx`、`file-tree-settings.tsx`、`advanced-settings.tsx` 均只导出、**全仓库无引用**（`grep` 命中只有自身定义行）；对话框只从 `tabs/` 引了 **2 个**：`ai-settings.tsx`、`git-settings.tsx`（`components/macos-settings-panels.tsx:2,18`）。它们**不是** Windows 的真源，**不要照它们实现**（见 §3，我仍逐条登记，作为「这些键确实在 TS 里被渲染过」的反向证据）。
4. **对话框里没有搜索框**。搜索基础设施（`stores/settings.store.ts:152-201` 的 `setSearchQuery`/`runSearch`、`config/search-index.ts` 的 **132 条**索引、`lib/settings-search.ts` 的打分函数）在 Windows 版本里**已实现但未接线到设置对话框**；真正用搜索的是命令面板（`features/command-palette/constants/settings-actions.tsx:140-151`）。
5. **持久化是 Tauri Store 插件写 `settings.json`，不是 localStorage**（`lib/settings-persistence.ts:50-52`）。localStorage 只用于「外观启动缓存」一个键 `lithe.bootstrap.appearance.v1`（`lib/appearance-bootstrap.ts:17,114,126`）。
6. **默认值合并是「逐键回退 + 仅对对象做一层浅合并」**，不是深合并（`lib/settings-persistence.ts:23-35`）。写入**每次改动都写但带 300ms 防抖**（`lib/settings-persistence.ts:99-112`），重置/导入/初始化是立即写。
7. **schema 版本与迁移只有一处**：键 `hiddenPatternDefaultsVersion`（当前 `1`），修的是「旧的空数组默认值」→ 结论是**没有通用迁移框架**，其余兼容全靠 `normalizeSettings` 的逐字段规范化（`lib/settings-migrations.ts:6-47`、`lib/settings-normalization.ts:440-578`）。
8. **主题系统两侧不同构**：Windows 用 `colors.<39 键>` + `syntax.<18 键>` + `id`；gpui 侧 `themes/lithe-*.json` 用 `themeSet.themes[].colors.<点分键>` + `name` + `mode`，**键名与容器结构都不同**，需要一层转换（见 §5.4）。
9. **gpui 侧第一版建议范围**：只做 **「常规」+「外观」**（第 1 阶段），并且**主题切换要连设置列表一起做**，否则无法验证；键盘/AI/高级/日志/更新**先不做**。逐项「能否立刻生效」见 §7.3。
10. **gpui-kit 组件结论**：**不要**用 `window.open_dialog` 承载设置页（不能从 `render` 打开、宽度要覆写、内部滚动+左侧栏组合更复杂）；用 **`Root::render_sheet_layer` + `window.open_sheet`** 或**在 `ShellWorkspace` 内自绘一个全窗格覆盖层**。`component::setting::{Settings, SettingPage, SettingGroup, SettingItem, SettingField}` **已经内置了「按标题/描述/关键词搜索过滤 + 分页 + 可重置」**，是本界面最贴的组件。硬约束：**任何浮层都必须在 `Root::new` 之后、且只能从事件回调或任务中打开**（`gpui/crates/app/src/main.rs:14-16`、`docs/gpui-kit/0.6.6/zh-CN/shell/overlays.md:203-221`）。详见 §9。

---

## 1. 对话框骨架

### 1.1 总体结构

| 层 | 内容 | 证据 |
| --- | --- | --- |
| 宿主 | `TitleBar` 内渲染 `<SettingsDialog isOpen onClose>`，由 `isSettingsDialogVisible` 控制 | `features/window/components/title-bar/title-bar.tsx:367-391` |
| 状态源 | zustand `ui-state` 的 modal slice：`isSettingsDialogVisible` / `settingsInitialTab` / `settingsTabRequest` | `features/window/stores/ui-state/modal-slice.ts:12,17,23,273-284` |
| 打开时副作用 | 打开设置会**关掉**快速打开、命令面板、全局搜索、分支管理、项目选择器、数据库连接（`openSettingsDialog` 一次性置 false） | `modal-slice.ts:273-284` |
| 打开时初始页签 | `settingsInitialTab` → `categoryFromRequestedTab()` 映射；`settingsTabRequest` 是「每次请求都 +1」的计数器，用于「同一个 tab 再点一次也要跳过去」 | `settings-dialog.tsx:50-82`；`modal-slice.ts:18-23,270-271` |
| 关闭 | `if (!isOpen) return null`；关闭按钮 / Esc / 点遮罩 → `onClose()` → `setIsSettingsDialogVisible(false)` | `settings-dialog.tsx:84,90,111`；`title-bar.tsx:391` |

**初始页签映射表**（`settings-dialog.tsx:50-70`，`default` 分支落到 `general`）：

| 传入 `SettingsTab` | 实际打开的分类 |
| --- | --- |
| `run` | `run` |
| `project` | `project` |
| `git` | `git` |
| `editor` / `keyboard` / `terminal` / `ai` / `ai-commit` / `logs` | 同名分类 |
| `language` | **`lsp`** |
| 其它（含 `general` / `appearance` / `advanced` / `file-explorer` / 空） | **`general`** |

> ⚠️ 因此 `openSettingsDialog("appearance")`（`features/command-palette/components/theme-selector.tsx:253`）实际打开的是**「常规」**页签，不是外观页签；`openSettingsDialog("language")`（`features/run/components/project-preparation-status.tsx:69`）打开的是 **LSP**。

### 1.2 全部打开入口（grep `openSettings` / `openSettingsDialog` / `isSettingsDialogVisible`）

| 入口 | 调用 | 证据 |
| --- | --- | --- |
| 快捷键 `Ctrl+,`（键面写 `cmd+,`） | 命令 `workbench.openSettings` → `setIsSettingsDialogVisible(true)` | `features/keymaps/defaults/default-keymaps.ts:513`；`features/keymaps/commands/command-registry.ts:1073-1082` |
| 键位→打开「快捷键」页签 | `openSettingsDialog("keyboard")` | `features/keymaps/commands/view-command-actions.ts:215` |
| 应用菜单事件 | `openSettingsDialog("general")` | `features/window/hooks/use-menu-events-wrapper.ts:54,346` |
| 深度链接 | `openSettingsFromDeepLink(action.tab, …)` → `openSettingsDialog(tab)` | `features/window/hooks/use-deep-link.ts:51,172-177` |
| 标题栏设置按钮 | `openSettingsDialog()` | `features/layout/components/sidebar/main-sidebar.tsx:127,644` |
| 欢迎页 | `openSettingsDialog("general")` | `features/layout/components/welcome-screen.tsx:49,99` |
| 命令面板（总入口 + 每个页签 + 每条设置记录） | `setIsSettingsDialogVisible(true)` / `openSettingsDialog(tab)` | `features/command-palette/constants/settings-actions.tsx:127-164`；`features/command-palette/components/command-palette.tsx:66-286` |
| 主题选择器 | `openSettingsDialog("appearance")`（→ 实际落到「常规」） | `command-palette/components/theme-selector.tsx:253` |
| AI 聊天偏好菜单 | `openSettingsDialog("ai")` | `features/ai/components/input/chat-preferences-menu.tsx:217,355` |
| Git 提交面板（未配置 AI 提交） | `openSettingsDialog("ai-commit")` | `features/git/components/git-commit-panel.tsx:73,121,342` |
| Git 空状态 / Git Log 工具窗 | `openSettingsDialog("git")` | `features/git/components/git-repository-empty-state.tsx:35,133`；`features/git/components/log/git-log-tool-window.tsx:85,578` |
| 运行面板 / 运行配置编辑器 / 运行状态条 | `openSettingsDialog("run")`、`("project")`、`("language")`、`("logs")` | `features/run/components/run-pane.tsx:73-77,278,299`；`run-configuration-editor.tsx:221-222`；`project-preparation-status.tsx:13,69,72` |
| 运行服务（Java main 启动失败） | `openSettings({ tab: "run" })` 回调 | `features/run/services/java-main-launch.ts:110,115` |
| Maven 面板 | `openSettingsDialog("project")` | `features/maven/components/maven-pane.tsx:684` |

**i18n**：命令标题 `keybindings.commands.workbench.openSettings.title` = 「打开设置」（`windows/tauri/src/i18n/locale.ts:8389`）；对话框标题键 `workbench.settings` = 「设置」（`locale.ts:5912` 附近；`gpui/crates/shared/locales/lithe.zh-CN.yml` 已生成 `lithe.workbench.settings`）。

### 1.3 度量与外观

| 项 | 值 | 证据 |
| --- | --- | --- |
| 遮罩 | `bg-black/55`，`fixed inset-0 z-9998`（AppDialog 默认 `bg-black/20`，这里被覆写） | `settings-dialog.tsx:117`；`ui/dialog.tsx:245` |
| 弹层尺寸 | `h-[620px] w-[820px]`，`max-h-[calc(100vh-32px)]`，`max-w-[calc(100vw-32px)]` | `settings-dialog.tsx:118-119` |
| 弹层定位 | 固定居中：`top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2`，`z-9999` | `ui/dialog.tsx:58` |
| 圆角 / 边框 / 阴影 | `rounded-xl`(11.2px) / `border border-border` / `shadow-(--shadow-dialog)` | `ui/dialog.tsx:59` |
| 头部 | 高 `h-11`(44px)、`border-b`、`bg-surface`、`px-3 py-0`；左侧齿轮图标 + 标题（`ui-text-base font-medium`）+ 右侧 24×24 关闭按钮 | `settings-dialog.tsx:120`；`ui/dialog.tsx:261-283` |
| 内容区 | `flex h-full p-0`（去掉 AppDialog 默认 `p-4`，因为要自己摆左右两栏） | `settings-dialog.tsx:121`；`ui/dialog.tsx:285` |
| 左侧分类栏 | `w-47.5` = **190px**、`shrink-0`、`flex-col gap-0.5`、`border-r`、`bg-surface`、`p-2`；`aria-label = settings.mac.categories`（「设置分类」） | `settings-dialog.tsx:125-128`；i18n 见 `lithe.zh-CN.yml` 的 `settings.mac.categories` |
| 分类按钮 | 高 `h-8`(32px)、`rounded-sm`(4.8px)、`px-2.5`(10px)、`gap-2.5`(10px)、`ui-text-sm`；图标 `size-4`(16px) | `settings-dialog.tsx:137-144` |
| 选中态 | `bg-primary/65 text-white font-medium`；未选中 `text-foreground hover:bg-accent`；`aria-current="page"` | `settings-dialog.tsx:138-142` |
| 分组 | **无分组**（12 个分类平铺，仅靠顺序） | `settings-dialog.tsx:35-48,129-148` |
| 右侧内容区 | `flex-1 overflow-y-auto bg-background p-6`(24px)；页标题 `h2` = `text-xl font-semibold` + `mb-5`(20px)，文案 = 当前分类标签 | `settings-dialog.tsx:151-153` |
| 底部 | `flex w-full items-center justify-between`：左「恢复默认设置」(ghost) + 右「完成」(accent)；AppDialog 的 footer 容器是 `justify-end gap-2 px-4 py-3` | `settings-dialog.tsx:93-115`；`ui/dialog.tsx:289-291` |
| 关闭行为 | Esc → Base UI `onOpenChange(reason==="escape-key")`，先过 `resolveEscapeGuard`（`data-prevent-dialog-escape` 的输入框会吃掉 Esc）；点遮罩关闭；右上关闭按钮 | `ui/dialog.tsx:209-233,276-281`；`utils/keyboard/escape-guard`；`components/typed-confirm-action.tsx:49,57-62` |
| 确认弹窗 | 「恢复默认设置」先弹 `showConfirmDialog(settings.mac.restoreDefaultsConfirm, {title: settings.mac.restoreDefaults})`；文案 = 「⚠️确认恢复所有配置吗？」 | `settings-dialog.tsx:99-107`；i18n `settings.mac.restoreDefaultsConfirm` |

### 1.4 搜索框：**对话框里没有**

* `settings-dialog.tsx` 全文没有输入框、没有 `setSearchQuery`，**132 条**设置搜索索引（`features/settings/config/search-index.ts`）在设置界面里没有任何渲染出口。
* 搜到的能力落在**命令面板**：`getMatchingSettingsRecords(query)` 生成 `Settings: <label>` 动作，执行时 `setSettingsSearchQuery(record.label)` + `openSettingsDialog(record.tab)`（`features/command-palette/constants/settings-actions.tsx:140-151`）——搜索词被写进 store，但没有界面消费它。
* 搜索语义（如果 gpui 侧要还原）：字段权重 label 16 / section 8 / tab 标签 7 / keywords 6 / description 3；所有词必须命中（AND）；先 `NFKD` 去音标、小写、非字母数字转空格，再比「精确/前缀/包含」与「紧凑串包含」两轮（`lib/settings-search.ts:39-101`）；页签别名表 `SETTINGS_SEARCH_TAB_LABELS`（`:18-33`）里为「运行配置/项目环境」补了中文关键词。
* `SettingRow` / `Section` 已经为「搜索高亮」预留了数据属性：`data-setting-row-key` / `data-settings-section-key`（= 归一化后的标签）、`data-settings-search-active` / `data-settings-section-active`（高亮态样式）（`components/settings-section.tsx:55-59,193,197`）。

### 1.5 行与分组的 UI 契约（gpui 侧要还原的行为）

`components/settings-section.tsx`：

| 元素 | 契约 | 证据 |
| --- | --- | --- |
| `SettingsView` | `layout="stack"`（默认，`space-y-4`）/ `"fill"`（`flex h-full min-h-0 flex-col`） | `:26-38` |
| `Section`（分组） | 标题 `h4`（`ui-text-base`）+ 可选 description（`text-subtle-foreground`）；子项 `space-y-2`；**第一个 Section 的标题默认隐藏**（`first:[&>.settings-section-header]:hidden`） | `:51-70` |
| `SettingRow` | 左「标签 + 可选 labelAccessory + 可选重置按钮」，右「控件」；整行可点即激活主控件（点击 → 聚焦/点击/选中主控件；toggle-group 会轮换到下一项；combobox / `aria-expanded` 会展开） | `:84-244`，核心逻辑 `:134-186` |
| 重置按钮 | 标签右侧 20×20 ghost 图标按钮（`ArrowCounterClockwise`），`canReset=false` 时 `invisible`；`aria-label = resetLabel ?? "Reset <label>"` | `:211-226` |
| 行内 a11y | 自动给主控件补 `aria-labelledby`（标签）与 `aria-describedby`（描述） | `:121-132` |
| 响应式 | 宽度 <640px（或容器查询 `@max-[640px]/settings`）时改为纵向堆叠、控件占满宽 | `:197,239` |
| 控件宽度档 | `compact w-28` / `default w-36` / `wide w-44` / `xwide w-56` / `number w-28` / `numberCompact w-24` / `text w-48` / `textWide w-56` | `:40-49` |

> **注意有两个并存的「分组」外观**：`settings-section.tsx` 的 `Section`（大标题 + 无边框）用于 `git-settings.tsx` / 死代码页签；`macos-settings-panels.tsx:40-49` 与 `log-settings-panel.tsx:27-36` 各自**再定义了一份** `SettingsGroup`（带 `border border-border bg-surface/35` 外框 + `bg-surface` 分组头），用于「常规/编辑器/…」与「日志」。**两条样式不统一，实现时择一（建议用带框的 `SettingsGroup`）**。`SettingsRow` 同样有两份（`settings-section.tsx:84` 与 `macos-settings-panels.tsx:51` / `log-settings-panel.tsx:38`）。

---

## 2. 页签（分类）清单

**按真实渲染顺序**（`settings-dialog.tsx:35-48`）：

| # | id (`MacSettingsCategory`) | 中文名 | 英文名（i18n 键） | 图标 | 组件文件 | 面板来源 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `general` | 常规 | General（`settings.tabs.general`） | `GearSixIcon` | `macos-settings-panels.tsx:75-273` 的 `GeneralPanel` | 内联（**没有** `tabs/general-settings.tsx`） |
| 2 | `project` | 项目 · JDK 与 Maven | Project · JDK & Maven（`settings.project.title`） | `FolderIcon` | `components/project-environment-settings.tsx` | 项目环境 |
| 3 | `run` | 运行配置 | Run configurations（`settings.run.title`） | `GearIcon` | `components/run-configuration-settings.tsx` | 运行配置（非 Settings 键） |
| 4 | `editor` | 编辑器 | Editor（`settings.tabs.editor`） | `CodeBlockIcon` | `macos-settings-panels.tsx:275-328` 的 `EditorPanel` | 内联（**没有** `tabs/editor-settings.tsx`） |
| 5 | `keyboard` | 快捷键 | Keybindings（`settings.tabs.keyboard`） | `KeyboardIcon` | `macos-settings-panels.tsx:330-370` 的 `KeyboardPanel` | 内联 |
| 6 | `terminal` | 终端 | Terminal（`settings.tabs.terminal`） | `TerminalWindowIcon` | `macos-settings-panels.tsx:372-396` 的 `TerminalPanel` | 内联（**没有** `tabs/terminal-settings.tsx`） |
| 7 | `lsp` | LSP | LSP（`settings.tabs.lsp`） | `DatabaseIcon` | `macos-settings-panels.tsx:398-438` 的 `LspPanel` | 内联 |
| 8 | `ai` | AI 聊天与编辑 | —— | `MagicWandIcon` | `components/tabs/ai-settings.tsx`（`AISettings`） | 复用 `tabs/` |
| 9 | `ai-commit` | AI 与提交 | AI & Commit（`settings.tabs.aiCommit`） | `MagicWandIcon` | `components/ai-commit-settings-panel.tsx` | 复用 `tabs/` + ai-commit |
| 10 | `git` | Git | Git（`settings.tabs.git`） | `CodeBlockIcon` | `components/tabs/git-settings.tsx`（`GitSettings`） | 复用 `tabs/` |
| 11 | `logs` | 日志 | Logs（`settings.tabs.logs`） | `FileTextIcon` | `components/log-settings-panel.tsx` | 日志 |
| 12 | `updates` | 更新 | Updates（`settings.tabs.updates`） | `ArrowClockwiseIcon` | `macos-settings-panels.tsx:440-483` 的 `UpdatesPanel` | 内联 |

分发点：`macos-settings-panels.tsx:485-518` 的 `MacSettingsPanel({category, onClose})`。

**i18n 说明**：
* `settings.tabs.ai` 的中文是「AI 聊天与编辑」（`gpui/crates/shared/locales/lithe.zh-CN.yml:11`），但**英文目录 `catalogs["en-US"]` 里没有这个键**（我逐行提取 `windows/tauri/src/i18n/locale.ts` 的 en 块未命中），所以 AI 页签在英文界面会回显键名——这是 Windows 的真实缺陷，gpui 侧需要自己补英文。
* 键名与中文一一对应的完整抽取见 §3 各表；`settings.tabs.{general,appearance,editor,files,git,terminal,keyboard,advanced,lsp,aiCommit,logs,updates}` 在 zh/en 两套都存在，其中 `appearance` / `files` / `advanced` **当前对话框并不使用**（属于死代码页签或仅命令面板引用）。

---

## 3. 逐页签规格

> **列约定**：
> `设置键` = `settings` 存储里的**完整点分路径**（顶层键直接写键名，例如 `autoSave`）；
> `控件` ∈ 开关 / 下拉 / 数字 / 文本 / 路径 / 多行文本 / 按钮；
> `默认` 取自 `config/default-settings.ts`；
> `约束` = 显示/可用条件；`hint` = 界面上的描述文案（中文）。
> 证据列给出**渲染行号**（`文件:行`）。中文文案的完整 i18n 值可在 `gpui/crates/shared/locales/lithe.zh-CN.yml`（`lithe.` 前缀）直接被 gpui 侧复用。

### 3.1 `general` —「常规」（8 项）

证据：`components/macos-settings-panels.tsx:75-273`。

分组（`SettingsGroup` 标题）：**外观** → **语言** → **项目** → **文件** → **Git** → **隐藏路径**。

| 设置键 | 标签（中） | 控件 | 默认 | 候选值 / 说明 | 约束 | hint | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `theme` | 配色主题 | 下拉（`<select>`，宽 `w-40`） | `lithe-dark` | 已注册主题；选中「跟随系统」时改的是 `autoTheme*` 而不是本键 | `syncSystemTheme === true` 时该项**不显示**，改为显示「首选浅色/深色主题」 | — | `:138-150`（选项 `:93-113`）、`:115-123` |
| `syncSystemTheme` | 外观模式 | 下拉 | `false` | `system` / `light` / `dark` | 选 `light`/`dark` 会**同时**写 `theme = lithe-light` / `lithe-dark` | 「选择配色主题，并设置是否跟随系统外观。」 | `:151-172` |
| `displayLanguage` | 语言 | 下拉 | `zh-CN` | `en-US`「英语」/ `zh-CN`「简体中文」 | — | 「界面语言会立即生效。默认语言为英文。」（**文案与默认值矛盾**） | `:175-191` |
| `askWhereToOpenProjects` + `openFoldersInNewWindow` | 项目打开方式 | 下拉（映射两个键） | `true` + `true` | `ask`「每次询问」/ `this-window`「此窗口」/ `new-window`「新窗口」 | 读合成值 `getProjectOpenPreference()`；写时分发 `getProjectOpenPreferencePatch()`（`ask` 只写 `askWhereToOpenProjects=true`） | 「选择打开其他项目时是每次询问、保留在此窗口，还是创建新窗口。」 | `:193-216`；`lib/project-open-preference.ts:8-29` |
| `autoSave` | 自动保存更改的文件 | 开关（`size="sm"`） | `true` | — | — | — | `:218-226` |
| （**未接线**）保存本地更改的方式 | 保存本地更改的方式 | 下拉 | 本地 state 初值 `"ask"` | `ask`「每次询问」/ `shelf`「暂存架」/ `stash`「Git 贮藏」 | **只写组件本地 `useState`，不落任何 settings 键**——切页签即丢 | 「选择执行 Git 操作前保护本地更改的方式。」 | `:81,228-243` |
| `hiddenDirectoryPatterns` | 隐藏路径 · 目录 | 多行文本（**草稿态**，按「应用」提交） | `.git`、`.hg`、`.idea`、`.mypy_cache`、`.pytest_cache`、`.ruff_cache`、`.svn`、`CVS`、`__pycache__`、`_svn` | 每行一项，`trim` 后丢弃空行 | 需点「应用」才写 | 「每行一项。目录名称会隐藏匹配的文件夹；文件条目支持 * 和 ?。」 | `:82-85,125-133,245-269`；默认值 `config/default-settings.ts:33-44` |
| `hiddenFilePatterns` | 隐藏路径 · 文件模式 | 多行文本（草稿态 + 「应用」） | `*.pyc`、`*.pyo`、`*.rbc`、`*.yarb`、`*~`、`.DS_Store`、`vssver.scc`、`vssver2.scc` | 同上 | 同上 | 同上 | `:85,245-269`；默认值 `default-settings.ts:22-31` |

### 3.2 `project` —「项目 · JDK 与 Maven」（非 Settings，6 个字段 + 2 个 Maven 字段）

证据：`components/project-environment-settings.tsx`。**没有 settings 键**：它读写的是**项目级文件**（`loadProjectEnvironment` / `saveProjectEnvironmentSettings`，`services/project-environment.ts`）。

| 字段 | 标签（中） | 控件 | 空值语义 | 候选 | 证据 |
| --- | --- | --- | --- | --- | --- |
| `toolchain.javaHomePath` | `run.jdkHome` | 文本 + `<datalist>` 自动补全 + 「浏览目录」按钮 + 「清除」 | 空 = 自动（`run.toolchainAuto`） | `environment.discovered.java` | `:143-159,220-286` |
| `toolchain.mavenExecutablePath` | `run.mavenExecutable` | 同上 | 空 = 自动（`settings.project.mavenAutomatic`） | `environment.discovered.maven` | `:160-176` |
| `toolchain.mavenJavaHomePath` | `run.mavenJdkHome` | 同上 | 空 = 使用项目 JDK（`settings.project.useProjectJdk`） | `environment.discovered.java` | `:177-193` |
| `maven.settingsPath` | `settings.xml` | 文本 + 「浏览文件」 | 空 = `maven.automatic` | — | `:287-314` |
| `maven.localRepositoryPath` | `maven.localRepository` | 文本 + 「浏览目录」 | 同上 | — | `:287-314` |

* 无项目时整页替换为一句提示：`settings.project.openProject`（`:30,50`）。
* 页首显示工作区根（等宽）+ 作用域说明 `settings.project.scope`（`:211-212`）。
* 每行下方显示「当前生效值」`EffectiveToolchain`（`:276`）与「检测到的安装」`settings.project.detected`（`:277-284`）。
* 底部按钮：「重新加载并检测」+ 「保存」；保存成功显示 `settings.project.saved`（`:321-336`）。
* 保存路径与 Run 联动，失败时提示 `settings.project.savedReloadFailed` + `settings.project.reloadFailed`（`:99-127`）。

### 3.3 `run` —「运行配置」（非 Settings）

证据：`components/run-configuration-settings.tsx`。

* 无项目：`settings.project.openProject`（`:50`）。
* 未加载：`settings.project.loading`（`:51`）。
* 正在编辑某条配置：渲染 `RunConfigurationEditor`（`features/run/components/run-configuration-editor.tsx`）+ 配置名标题（`:55-77`）。
* 列表态：说明 `settings.run.description` → 「重新识别」按钮（`run.identifyAgain`）→ 每条配置一个全宽 ghost 按钮（`:79-111`）。
* 错误/提示：`state.invalidMessage`、`state.saveError` 用 `role="alert"`；`generated:<n>` 用 `run.generatedEntries`（`:82-96`）。

### 3.4 `editor` —「编辑器」（4 项）

证据：`components/macos-settings-panels.tsx:275-328`。

分组：**显示** → **编辑器标签页** → **缩进**。

| 设置键 | 标签（中） | 控件 | 默认 | 候选 | hint | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| `fontSize` | 字体大小 | 数字（`min=10 max=22`，宽 `w-20` 右对齐） | `14` | — | — | `:283-292` |
| `codeLens` | 显示用法与 Git 作者 | 开关 | `true` | — | — | `:293-299` |
| `horizontalTabScroll` | 缓冲区轮播（`settings.editor.bufferCarousel`） | 开关 | `true` | — | 「在主视图中将打开的缓冲区显示为可横向滚动的轮播」 | `:301-312` |
| `tabSize` | 制表符宽度 | 下拉 | `2` | `2` / `4` / `8`（显示为「N 个空格」） | — | `:313-325` |

### 3.5 `keyboard` —「快捷键」（4 项 + 只读提示）

证据：`components/macos-settings-panels.tsx:330-370`。

分组：**快捷键方案** → **键盘快捷键**。

| 设置键 | 标签（中） | 控件 | 默认 | 候选 | hint | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| `keybindingPreset` | 预设 | 下拉（`w-44`） | `none` | `none`「Lithe」/ `vscode`「Visual Studio Code」/ `jetbrains`「JetBrains」/ `xcode`「Xcode」 | — | `:337-355` |
| （静态）搜索快捷键 | 搜索快捷键 | **无 `value`/`onChange` 的纯装饰输入框** | — | — | 「选择快捷键预设，然后使用命令面板查看和运行可用命令。」 | `:356-366` |

> 注意与 `tabs/keyboard-settings.tsx`（死代码）的差异：后者是**完整**的键位编辑器（表格 + 6 档筛选 + 导入/导出/重置），见 §6.3。

### 3.6 `terminal` —「终端」（1 项）

证据：`components/macos-settings-panels.tsx:372-396`。分组：**Shell**。

| 设置键 | 标签（中） | 控件 | 默认 | 候选 | hint | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| `terminalDefaultShellId` | 默认 Shell | 下拉（`w-44`） | `""` | `""`「系统默认」/ `powershell`「PowerShell」/ `cmd`「命令提示符」/ `wsl`「WSL」 | 「用于新的终端会话。」 | `:379-393` |

> 注意：`terminalDefaultProfileId` **在真实对话框里不可设置**（死代码页签里有，见 §6.3）。其余十来个终端键（字形/光标/交互）同样只在死代码页签里。

### 3.7 `lsp` —「LSP」（3 项）

证据：`components/macos-settings-panels.tsx:398-438`。分组：**语言服务** → **已检测语言服务器**。

| 设置键 | 标签（中） | 控件 | 默认 | hint | 证据 |
| --- | --- | --- | --- | --- | --- |
| `autoCompletion` | 自动补全 | 开关 | `true` | 「显示活动语言服务器提供的补全建议。」 | `:406-415` |
| `parameterHints` | 参数提示 | 开关 | `true` | — | `:416-422` |
| `semanticTokens` | 语义高亮 | 开关 | `true` | — | `:423-429` |
| （静态）已检测语言服务器 | 已检测语言服务器 | **纯文本**，无列表 | — | 「语言服务器由已安装的语言扩展检测，并在打开受支持文件时启动。」 | `:431-435` |

### 3.8 `ai` —「AI 聊天与编辑」（`AISettings`）

证据：`components/tabs/ai-settings.tsx`（1024 行）。此页是**动态 + 命令式**的复合面板，逐项列全如下。

**用到的 settings 键**（`components/tabs/ai-settings.tsx:71-86`）：

| 设置键 | 默认（`config/default-settings.ts`） | 语义 |
| --- | --- | --- |
| `aiProviderId` | `anthropic`（`DEFAULT_AI_PROVIDER_ID`，`default-settings.ts:15`） | Agent 提供商 |
| `aiModelId` | `claude-sonnet-4-6`（`:16`） | Agent 模型 |
| `aiCustomBaseUrl` | `""` | 自定义提供商 base URL |
| `aiCustomModelId` | `""` | 自定义提供商模型名 |
| `ollamaBaseUrl` | `http://localhost:11434`（`:129`） | Ollama 端点 |
| `aiCompletion` | `true`（`:120`） | AI 自动补全总开关 |
| `aiAutocompleteProvider` | `openrouter`（`:121`） | 自动补全提供商（`openrouter` / `custom`） |
| `aiAutocompleteModelId` | `mistralai/devstral-small`（`:122`） | 自动补全模型 |
| `aiAutocompleteCustomBaseUrl` | `""`（`:123`） | 自定义自动补全 base URL |
| `aiAutocompleteCustomModelId` | `""`（`:124`） | 自定义自动补全模型 |

**8 个 Section，按真实渲染顺序**（`components/tabs/ai-settings.tsx`）：

| # | Section 标题 | i18n | 行数 | 条件 | 证据 |
| --- | --- | --- | --- | --- | --- |
| 1 | Codex 集成（内层 Section 标题被 `first:` 规则隐藏） | `codexSettings.title` | — | — | `:439`；`components/settings-section.tsx:55` |
| 2 | `Lithe Agent` | **硬编码字符串（无 i18n 键）** | 4 类：提供商 / 模型 / 「API 密钥」按钮 / `providerSettingsActions` 动态行 | — | `:440` |
| 3 | 自定义提供商 | `aiSettings.customProvider` | 2：Base URL / API Key | 仅当选中自定义提供商 | `:538` |
| 4 | `Ollama` | **硬编码字符串（无 i18n 键）** | 5：模式 / 端点 / API 密钥 + 2 个条件行（云端密钥行、连接失败行） | 仅当 `aiProviderId === "ollama"` 一类条件 | `:607` |
| 5 | 认证 | `aiSettings.authentication` | N 行（值带 Badge「即将推出」`aiSettings.comingSoon`） | 条件 | `:749` |
| 6 | ACP 会话 | `aiSettings.acpSession` | N 行（仅渲染 `kind.type === "select"` 的会话选项） | 条件 | `:763` |
| 7 | 自动补全 | `aiSettings.autocomplete` | 6：总开关 + 4 个条件行（提供商 / 模型 / 自定义 base URL / 自定义密钥）+ 条件错误行 | 总开关为「自动补全」本身 | `:796` |
| 8 | Agent 历史 | `aiSettings.agentHistory` | 1：「清空 Agent 历史」`TypedConfirmAction`（确认词 `"yes"`） | — | `:1001` |

另有一个**浮层**（不是 Section）：`ProviderApiKeyCommand`（`ProviderApiKeyCommand` 挂载于 `:743`）。

**API 密钥不进 settings**：走 `features/ai/services/ai-token-service.ts` 的 `getProviderApiToken`(`:20`) / `storeProviderApiToken`(`:33`) / `removeProviderApiToken`(`:46`)，最终落到 Tauri 的 `get_secure_secret` / `store_secure_secret` / `remove_secure_secret`，**key 前缀 `ai-provider-token/`**（`ai-token-service.ts:9`）；三个具体 key 是 `ollama`、`custom`（自定义聊天）、`autocomplete-custom`（自定义补全）（`features/ai/lib/custom-provider-config.ts:4-5`）。Ollama 密钥再由 `setOllamaApiKey` 推给 provider 单例（`lib/settings-effects.ts:161-168`）。

**未在 AI 页出现的键**：`aiDefaultSessionMode` 在 `windows/tauri` 下**没有任何读取点**；`aiSkills` / `v0DesignSystems` / `activeV0DesignSystemId` 在本页无直接 UI（v0 仅以 provider 扩展 action 形式出现，`features/ai/extensions/v0/v0-extension.tsx:69-78`）。

**非持久化的组件本地 state**（`:90-120` 区段，共 15 个 `useState` + 4 个 `useRef`）：`sessionConfigOptions`、`isClearingChats`、`autocompleteModels`、`isLoadingAutocompleteModels`、`autocompleteModelError`、`customAutocompleteModelInput`、`customAutocompleteBaseUrlInput`、`customAutocompleteApiKeyInput`、`hasCustomAutocompleteApiKey`、`isSavingCustomAutocompleteApiKey`、`customChatBaseUrlInput`、`customChatApiKeyInput`、`hasCustomChatApiKey`、`isSavingCustomChatApiKey`、`isApiKeyManagerOpen`、`ollamaUrl`、`ollamaStatus`、`ollamaDebounceRef`、`ollamaDraftDirtyRef`、`ollamaValidationIdRef`、`lastSelfHostedOllamaUrlRef`。

**本页引入的 3 个外部分组组件**：`ProviderApiKeyCommand`（`features/ai/components/provider-api-key-command.tsx`）、`ModelSelector`、`ProviderSelector`（`:18-20`）、`CodexSettings`（`features/ai/integrations/codex/codex-settings.tsx`，`:60`）；这些**不在设置目录里**，实现 gpui 版时需要单独规格（见 `06-database-ai-search.md` §AI）。

**i18n 前缀不是 `settings.ai.*`**：本页用 `aiSettings.*`（**79 个键**，en/zh 齐全），例如 `aiSettings.provider`「提供商」/`aiSettings.model`「模型」/`aiSettings.customModelDescription`「发送到自定义端点的模型名称」/`aiSettings.ollamaModeDescription`「在本地运行 Ollama 或使用 Ollama Cloud」/`aiSettings.clearAgentHistoryDescription`「永久删除所有 Agent 历史」/`aiSettings.savedKeyDescription`「已安全存储。留空将保留现有密钥。」（`windows/tauri/src/i18n/locale.ts:374` 起；抽取结果见 `_tmp-en-ai.txt` 前的推导）。

**持久值的规范化**（导入/初始化时强制）：`lib/settings-normalization.ts:381-438` —— provider 不存在则回落 `anthropic`；模型有 `AI_MODEL_MIGRATIONS` 重映射表（`:27-97`）；自定义提供商的模型名强制等于 `aiCustomModelId`；模型不在 provider 列表则取列表首项；`aiAutocompleteProvider` 只允许 `custom|openrouter`；`aiSkills` 最多 200 条、逐字段截断（`:311-379`）；`v0DesignSystems` 由扩展侧规范化；`activeV0DesignSystemId` 不存在于列表则清空。

### 3.9 `ai-commit` —「AI 与提交」（`AiCommitSettingsPanel`）

证据：`components/ai-commit-settings-panel.tsx`；键全部在 `aiCommit` 子对象下，类型与默认值见 `features/git/types/ai-commit.ts`（`DEFAULT_COMMIT_AI`，`:66-77`）。

| 设置键 | 默认 | 候选 | 证据（类型/默认） |
| --- | --- | --- | --- |
| `aiCommit.enabled` | `true` | 开关 | `ai-commit.ts:67` |
| `aiCommit.activeProviderId` | `null` | provider 列表（`providers[]`） | `:69` |
| `aiCommit.providers[]` | `[]`（最多 30 条） | 每条含 `id/name/endpoint/model/apiProtocol/authentication/source/requiresApiKey/allowsInsecureHttp/chatTokenLimitField` | `:22-33,106-136` |
| `aiCommit.language` | `english` | `english` / `simplifiedChinese` | `:70` |
| `aiCommit.format` | `conventional` | `conventional` / `concise` / `imperative` / `descriptive` / `releaseNote` / `custom` | `:1-8,71` |
| `aiCommit.customInstructions` | `""`（截断 4000） | 文本框 | `:72,145` |
| `aiCommit.includeBody` | `false` | 开关 | `:73` |
| `aiCommit.subjectMaximumLength` | `72` | 数字（20–200） | `:74,148` |
| `aiCommit.maximumDiffCharacters` | `32_000` | 数字（8 000–120 000） | `:75,149` |
| `aiCommit.reasoningEffort` | `default` | `default`/`none`/`minimal`/`low`/`medium`/`high`/`xhigh`/`max` | `:10-19,76` |
| `apiProtocol` 候选 | `responses` | `responses` / `chatCompletions` / `anthropicMessages` | `:9` |
| `chatTokenLimitField` 候选 | `max_completion_tokens` | `max_completion_tokens` / `max_tokens` | `:20` |

> ⚠️ 本页的**逐行 UI 结构**（分组顺序、每个控件的显示条件）**已补齐**，见下表。

**条件控件（逐条）**：

| 控件 / 字段 | 显示或可用条件 | 证据 |
| --- | --- | --- |
| `chatTokenLimitField` | 仅当 `apiProtocol === "chatCompletions"` 时出现 | `components/ai-commit-settings-panel.tsx:254` |
| `allowsInsecureHttp` | 仅当 `endpoint` 以 `http:` 开头时出现 | `:340` |
| 托管密钥提示 / 非托管密钥输入区 | 视 provider 是否 `managed` 切换 | `:302-330` |
| 自定义提示词输入 vs 格式示例 | 视 `format === "custom"` 切换 | `:447-466` |
| `reasoningEffort` | `apiProtocol === "anthropicMessages"` 时 **disabled** | `:412` |

**凭据**：不在 settings 里。走 `commitKey` → `invoke("ai_commit_key")`（`features/git/services/ai-commit-service.ts:21`），最终安全存储键为 `lithe.ai.commit.{id}`（`windows/tauri/src-tauri/src/ai_commit.rs:103-111,138-166`）。

> 协议 / 认证 / 语言等下拉的候选值在组件里是**硬编码字面量**（没有 i18n 键）；`aiCommit.enabled === false` 的消费逻辑**不在本文件内**。

**gpui 第一版不做此页**（§7.4）。

### 3.10 `git` —「Git」（11 项 + 2 个子面板）

证据：`components/tabs/git-settings.tsx`。分组顺序：`GitExecutionSettings`（无分组头）→ `<details>`「Fetch 与提交偏好」（折叠，默认收起）→ **集成** → **Git 视图** → **编辑器**。

| 设置键 | 标签（中） | 控件 | 默认 | 约束 / 联动 | hint | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| `gitExecutable` | Git 可执行文件 | 文本 + 「保存」+「用 PATH 里的 Git」 | `""` | `git-execution-settings.tsx` | — | `git-execution-settings.tsx:64-68` |
| `gitUseCredentialHelper` | 使用凭据助手 | **`<input type=checkbox>`**（不是 Switch） | `true` | — | 「Git 可以在 Lithe 中请求凭据。Lithe 不保存密码；凭据存储由你的 Git 助手管理。」 | `:69-70` |
| `gitFetchPrune` | 修剪远程已删除引用 | 开关 | `true` | **关闭时若 `gitFetchTags === "prune"` 会把 tags 改回 `inherit`** | 「用于每个项目中的普通 Fetch。」（`git.fetch.scope`） | `git-settings.tsx:48-53` |
| `gitFetchSubmodules` | 拉取子模块 | 下拉 | `inherit` | `inherit`「使用 Git 配置」/ `no`「不拉取子模块」/ `onDemand`「按需拉取」/ `yes`「拉取所有子模块」 | — | `:54-57` |
| `gitFetchTags` | 拉取标签 | 下拉 | `inherit` | `inherit`/`all`「拉取所有标签」/`none`「不拉取标签」/`prune`「同步标签并删除远端已不存在的本地标签」；**选 `prune` 会把 `gitFetchPrune` 置 true** | 「凭据沿用现有 Git 助手与 SSH 配置。」（`git.fetch.credentials`） | `:58-64` |
| `coreFeatures.git` | Git 集成 | 开关 | `true` | 写的是 `coreFeatures` 对象（整体替换） | 「启用 Git 仓库的源代码管理功能」 | `:68-76`；`handleGitFeatureToggle` `:35-40` |
| `autoRefreshGitStatus` | 自动刷新 Git 状态 | 开关 | `true` | — | 「相关文件或 Git 事件发生变化后自动刷新 Git 视图」 | `:78-91` |
| `confirmBeforeDiscard` | 丢弃前确认 | 开关 | `true` | — | 「丢弃文件或仓库更改前显示确认提示」 | `:93-106` |
| `gitChangesFolderView` | 基于文件夹的更改 | 开关 | `true` | — | 「以类似文件视图的文件夹树形式显示 Git 更改」 | `:110-123` |
| `showUntrackedFiles` | 显示未跟踪文件 | 开关 | `true` | — | 「在 Git 状态面板中显示未跟踪文件」 | `:125-138` |
| `showStagedFirst` | 优先显示已暂存项 | 开关 | `true` | — | 「在 Git 面板中将已暂存更改显示在未暂存更改之前」 | `:140-151` |
| `openDiffOnClick` | 单击打开差异 | 开关 | `true` | — | 「单击已更改文件时打开差异，而不是直接打开文件」 | `:153-164` |
| `compactGitStatusBadges` | 紧凑 Git 状态标记 | 开关 | `false` | — | 「在 Git 面板中使用更紧凑的差异统计和暂存标签布局」 | `:166-179` |
| `collapseEmptyGitSections` | 折叠空分区 | 开关 | `false` | — | 「没有项目时隐藏已暂存更改等空 Git 分区」 | `:181-196` |
| `rememberLastGitPanelMode` | 记住上次 Git 面板模式 | 开关 | `false` | — | 「重新打开 Git 视图时恢复上次打开的底部 Git 面板分区」 | `:198-213` |
| `gitDefaultDiffView` | 默认差异视图 | 下拉（可搜索） | `unified` | `unified`「统一视图」/ `split`「拆分视图」 | 「选择 Git 差异的默认布局」 | `:215-236` |
| `enableInlineGitBlame` | 启用行内 Blame | 开关 | `true` | — | 「在编辑器中显示当前行的 Git Blame 元数据」 | `:239-254` |

**子面板 1：`GitExecutionSettings`**（`components/git-execution-settings.tsx`，分组头 `git.execution.title` =「Git 执行」）
* 展开式「高级配置与来源」（`git.execution.advanced`，`<details>`）。
* 作用域下拉：`local`「当前仓库」/ `global`「全局 Git 配置」（`:75-78`）。
* 后端快照 `git.executionInspect`：显示版本、可执行文件、临时配置、生效 Fetch 选项及来源、配置项列表（含「被后面的值覆盖」标记）（`:79-95`）。
* 键/值下拉 + 「保存」+「清除覆盖」：写 `git.executionConfigure`（`:84-90`）。
* 「重新加载 Git 配置」（`:98`）。
* 这些是 **Git 配置文件**读写，**不是 settings 键**。

**子面板 2：`GitIdentitySettings`**（`components/git-identity-settings.tsx`，分组头 `git.setup.identity` =「提交身份」）
* 作用域下拉：`local`「当前仓库」/ `global`「全局 Git 配置」（`:90-101`）。
* 两个字段：`name`「提交者姓名」/ `email`「提交者邮箱」，各自「保存」+「清除覆盖」，逐字段保存、逐字段提示「当前生效值 / 尚未配置有效值」（`:113-165`）。
* 非仓库 + `local` 时禁用两个输入并提示「请先在 Git 面板初始化项目，再保存当前仓库的身份配置。」（`:110-112,129`）。
* 写 `configureGitIdentity` / 读 `getGitRepositorySetup`（`features/git/api/git-setup-api.ts`），**不是 settings 键**。
* i18n 前缀是 `git.setup.*`（34 键，en/zh 齐全）。

### 3.11 `logs` —「日志」

证据：`components/log-settings-panel.tsx`。四组：**日志位置** / **诊断** / **保留策略** / **诊断包**。

| 项 | 标签（中） | 控件 | 数据来源 | 证据 |
| --- | --- | --- | --- | --- |
| 当前日志位置 | 当前日志位置 | 只读等宽路径 + 3 个按钮：「打开当前日志」「打开目录」「复制路径」 | `LogSettingsSnapshot.effective_path`；描述位显示 fallback 原因或「本次会话实际写入日志的目录。」 | `:320-356` |
| 默认日志位置 | 默认日志位置 | 只读等宽路径 | `default_path` | `:357-364` |
| 自定义日志位置 | 自定义日志位置 | 只读路径 + 「选择…」+「恢复默认」 | `configured_path`；空则显示「正在使用默认位置」 | `:365-387` |
| 诊断日志 | 本次会话启用诊断日志 | 开关 | `diagnostic_enabled`；切换同时调 `setFrontendDiagnosticEnabled()` | `:390-402` |
| 保留策略 | （说明文本，无控件） | — | 「单个日志达到 10 MB 后轮转，每天保留最新五个常规日志，并在 Lithe 启动时删除超过 30 天的日志。」 | `:404-407` |
| 清除日志 | 清除当前目录日志 | 按钮 + 确认弹窗 | `clearLitheLogs()`；完成后提示「已删除 {count} 个日志文件，释放 {size}。」 | `:408-415,190-216` |
| 导出诊断包 | 导出诊断包… | 按钮 + 预览确认弹窗 + 保存对话框 | `previewDiagnosticBundle()` / `exportDiagnosticBundle(path)`；预览列出文件名与大小 | `:418-427,255-312` |

* 全部经 `features/logging/log-api.ts`（**不是 settings 键**）；日志目录/诊断开关是**应用级文件与后端状态**。
* 换目录后若旧目录仍有日志，弹确认「清理旧日志？」（`settings.logs.previousDirectoryTitle` / `previousDirectoryPrompt`）（`:116-136`）。

### 3.12 `updates` —「更新」

证据：`components/macos-settings-panels.tsx:440-483`。分组：**软件更新**。

| 项 | 标签 | 控件 | 行为 | 证据 |
| --- | --- | --- | --- | --- |
| 版本 | `Lithe` | 按钮「检查更新」/「正在检查…」 | 用 `useUpdater(false)` + `getVersion()`；`checkForUpdates({ignoreSuppression:true})` | `:455-470` |
| 状态行 | — | `role="status"` 文本 | 四态：失败 / 有新版 `版本 {version} 可用。` / 已是最新 / 「Lithe 可以检查新的预览版和稳定版。」 | `:471-479` |

无 settings 键。`UpdaterStore`/`update-preferences.ts`/`use-updater.ts` 承担更新偏好（不在设置界面暴露）。

---

## 4. 持久化

### 4.1 存在哪里

| 存储 | 键 / 文件 | 内容 | 证据 |
| --- | --- | --- | --- |
| **Tauri Store（主）** | `settings.json`（插件 `@tauri-apps/plugin-store`，`{autoSave:true}`） | **每个 settings 顶层键一条记录**（不是包成一个对象） | `lib/settings-persistence.ts:50-52,84-94` |
| Tauri Store（主题） | `custom-themes.json`，键 `"themes"` | 自定义主题数组（`Theme[]`） | `extensions/themes/custom-theme-store.ts:5-6,30-42` |
| localStorage | `lithe.bootstrap.appearance.v1` | 外观启动缓存（见 §4.5） | `lib/appearance-bootstrap.ts:17,114,126` |
| Git 配置 | `git config`（local/global） | `GitExecutionSettings` / `GitIdentitySettings` 的读写在 Git 自身配置里 | `components/git-execution-settings.tsx:48,59` |
| 密钥 | 安全存储（`ai-token-service`） | API 密钥/令牌，**不写 settings.json** | `lib/settings-effects.ts:161-168`；`components/tabs/ai-settings.tsx:55-59` |
| 项目级 | 项目内文件（`loadProjectEnvironment`/`saveProjectEnvironmentSettings`） | JDK / Maven 路径 | `services/project-environment.ts` |
| 日志 | 后端 `log-api` | 日志目录、诊断开关 | `components/log-settings-panel.tsx:3-14` |

> `UI-MAP-WINDOWS.md:1820` 曾把 Windows 的持久化写成「localStorage + zustand persist」——**对 settings 而言不准确**：settings 走 Tauri Store。localStorage 只承载启动缓存与其它 feature 各自的前缀键（`lithe-tab-sessions`、`git-log-preferences` 等）。

### 4.2 版本与迁移

* **唯一的显式版本键**：`hiddenPatternDefaultsVersion`（当前 `1`，`lib/settings-migrations.ts:6-7`）。
* 迁移逻辑：若旧记录里 `hiddenFilePatterns` 与 `hiddenDirectoryPatterns` **同时为空数组**，则回填新的默认模式，并写入版本键（`:16-47`）。若版本已 `>= 1` 则直接返回（`:23-25`）。
* **没有通用迁移框架**：其余跨版本兼容全部靠 `normalizeSettings`（见 4.3）。
* 初始化时的默认值补齐与迁移在**同一个函数**里：`initializeStoreDefaults()` 先跑迁移，再逐键补默认（`lib/settings-persistence.ts:18-43`）。
* 导入文件的格式版本是另一个常量：`SETTINGS_EXPORT_VERSION = 1` + `format = "lithe.settings"`（`lib/settings-import-export.ts:8-9`）。

### 4.3 规范化（`lib/settings-normalization.ts`，716 行）

`normalizeSettings(settings)` 在**初始化**与**导入**时全量跑一次；`normalizeSettingValue(key, value)` 在**每次 `updateSetting`** 时按 key 跑一次（`stores/settings.store.ts:117-150`）。做的事（逐条带证据）：

| # | 规范化 | 证据 |
| --- | --- | --- |
| 1 | provider 不存在 → `anthropic`；模型按 `AI_MODEL_MIGRATIONS`（约 80 条）重映射；不在列表 → 首项；自定义提供商 → `aiCustomModelId` | `:381-409`（表 `:27-97`） |
| 2 | `aiAutocompleteModelId` 按表重映射并可回落默认；`aiAutocompleteProvider` 只允许 `custom/openrouter` | `:411-420`（表 `:102-104`） |
| 3 | `aiSkills` 校验（id/title/content/createdAt/updatedAt）、id 去重、上限 200、逐字段截断（title 120 / description 240 / content 100 000 / tags 12×40…） | `:311-379` |
| 4 | `v0DesignSystems` 规范化；`activeV0DesignSystemId` 不在列表则清空 | `:422-435` |
| 5 | `aiCommit` → `normalizeCommitAI()`（白名单 + 范围钳制 + provider 去重上限 30） | `:442`；`git/types/ai-commit.ts:95-150` |
| 6 | `gitExecutable` 强制 string；`gitUseCredentialHelper !== false`；`gitFetchX` 白名单；`tags==="prune"` → `prune=true` | `:443-448` |
| 7 | `coreFeatures` 与默认值做**一层**合并，并删除废弃键 `litheEditorEngine` / `energyEdge` | `:452-457` |
| 8 | `gitLastPanelMode` 非法 → `changes`；`gitSidebarTabOrder` 过滤非法值，空则回默认 | `:449-470` |
| 9 | 旧默认 UI 字体 `"Geist Sans"` + 字号 `15` 的组合整体迁移到新默认 | `:472-478`（常量 `:99-100`） |
| 10 | `uiFontSize` 归一到 10–24、0.5 步长 | `:480`；`lib/ui-font-size.ts:10-19` |
| 11 | 三个字体族做「首个可用族」解析（`normalizeConfiguredFontFamily`） | `:481-492` |
| 12 | `terminalLineHeight` 旧默认 `1.2` → `1` | `:493-495` |
| 13 | `editorLineHeight` 0.1 步长 + 夹在 1–2 | `:496-498`（`normalizeEditorLineHeight` `:173-180`） |
| 14 | `renderWhitespace` / `editorCursorStyle` / `editorCursorBlinking` / `terminalCursorInactiveStyle` / `tabCloseButtonVisibility` / `windowChromeDensity` / `fileTreeSortOrder` 全部走**白名单集合**，非法 → 默认 | `:499-519`（集合定义 `:116-153`） |
| 15 | `activityRailWidth` 140–320、`sidebarWidth`/`rightToolWindowWidth` 140–600（取整） | `:520-537`（`normalizeBoundedWidth` `:191-197`） |
| 16 | `externalEditor` 白名单；`custom` 但 `customEditorCommand` 为空 → `none` | `:538-541`（`normalizeExternalEditor` `:284-297`） |
| 17 | 删除废弃键 `editorEngine`、`fileTreeDensity`、`jdtlsJavaHomePath` | `:542,546,550` |
| 18 | `fileTreeIndentSize` 取整 + 8–32 | `:543-545`（`normalizeFileTreeIndentSize` `:182-189`） |
| 19 | `lastSettingsTab`：`"features"` → `"advanced"`，其它非法 → `"general"` | `:547-549`（`normalizeSettingsSection` `:299-309`） |
| 20 | `keybindingPreset` 不在 8 个预设内 → `"none"` | `:552-554` |
| 21 | `iconTheme` 旧 id 重映射：`lithe-icons*` → `idea-icons`；`colorful-material`/`seti` → `symbols` | `:556`（`normalizeIconTheme` `:209-226`） |
| 22 | 四个「项目顺序」数组做规范化（去重、剔除未知项、按默认顺序补全） | `:558-575` |
| 23 | 两个字符串数组去重去空（`hiddenSidebarActivityItems`、`collapsedActivityRailSections`） | `:562-567`（`normalizeStringList` `:199-207`） |

`normalizeSettingValue`（逐键版）覆盖：`aiCommit`、`gitFetchSubmodules`、`gitFetchTags`、`uiFontSize`、三个字体族、`terminalLineHeight`、`editorLineHeight`、`renderWhitespace`、`editorCursorStyle`、`editorCursorBlinking`、`terminalCursorInactiveStyle`、`tabCloseButtonVisibility`、`windowChromeDensity`、`fileTreeSortOrder`、`activityRailWidth`、`sidebarWidth`/`rightToolWindowWidth`、`hiddenSidebarActivityItems`/`collapsedActivityRailSections`、`fileTreeIndentSize`、`lastSettingsTab`、`iconTheme`、`keybindingPreset`、`aiSkills`、`v0DesignSystems`、`activeV0DesignSystemId`、`aiCustomBaseUrl`、`ollamaBaseUrl`、`aiCustomModelId`、`aiAutocompleteProvider`、`aiAutocompleteCustomBaseUrl`、`aiAutocompleteCustomModelId`（`:580-715`）。

### 4.4 默认值合并：逐键回退 + 对象一层浅合并

`initializeStoreDefaults()`（`lib/settings-persistence.ts:18-43`）：

1. 读全部 store 记录 → 跑 `hiddenPatternDefaults` 迁移。
2. **遍历 `defaultSettings` 的每个顶层键**：
   * `undefined`/`null` → 写默认值；
   * 默认值是**对象**（只有 `coreFeatures` 与 `aiCommit` 这种）→ `{...defaultValue, ...currentValue}`（**一层**浅合并，嵌套对象不会递归）；
   * 其它 → 保留当前值。
3. 只有真正变化的键才 `store.set` + `store.save`（`isEqual` 去抖）。

`loadSettingsFromStore()`（`:67-82`）再从「默认快照」出发，**逐键覆盖**已存在的记录，缺键保留默认。`getDefaultSettingsSnapshot()`（`config/default-settings.ts:218-233`）额外对 9 个数组/对象做深拷贝，并再跑一次 `normalizeUiFontSize`——**这是唯一一处「深拷贝」**，不是深合并。

### 4.5 写入时机

| 场景 | 时机 | 证据 |
| --- | --- | --- |
| `updateSetting(key, value)` | **300 ms 防抖**批量写；同一窗口内多个键合并成一次 `saveSettingsToStore(patch)` | `stores/settings.store.ts:149`；`lib/settings-persistence.ts:96-112` |
| `updateSetting` 的连带写 | `syncSystemTheme=true` 时同时写 `autoThemeLight`/`autoThemeDark`；`aiProviderId`/`aiModelId` 等触发 `getAIModelSelectionPatch`，联动键一起进同一个 patch | `stores/settings.store.ts:117-150`；`lib/ai-model-selection.ts`；`lib/theme-resolution.ts:50-62` |
| `toggleAIChatVisible` | 120 ms 冷却 + 防抖写 | `stores/settings.store.ts:30,99-115` |
| 初始化 | **立即**写（当检测到主题或规范化改变了内容时） | `lib/settings-bootstrap.ts:42-57` |
| `resetToDefaults` | **立即** `await saveSettingsToStore` | `stores/settings.store.ts:88-97` |
| 导入 JSON | **立即**写 | `stores/settings.store.ts:61-80` |
| `theme` 变更 | 额外把主题写到当前项目标签（`setProjectTheme`） | `stores/settings.store.ts:140-147` |
| 启动 | `initializeSettingsState()`：load → 补默认（必要时落盘）→ `normalizeSettings` → `applySettingsSideEffects` → 应用 → 必要时落盘 | `lib/settings-bootstrap.ts:26-62` |

**副作用（`lib/settings-effects.ts`）**——这是「改设置能不能立刻看到」的关键，逐条列清：

| 键 | 副作用 | 立即生效？ | 证据 |
| --- | --- | --- | --- |
| `theme` | `applyTheme(resolveEffectiveTheme(...))` —— 走 `themeRegistry.applyTheme()`；注册表未就绪时排队等注册 | ✅ 是 | `:190-192,71-119` |
| `syncSystemTheme` / `autoThemeLight` / `autoThemeDark` | 重新解析生效主题 + 挂/摘 `matchMedia("(prefers-color-scheme: dark)")` 监听 | ✅ 是 | `:194-203,50-69` |
| `windowTransparency` | 写 `data-window-transparency`，并在应用主题时 `invoke("set_native_window_appearance", {themeType, transparencyEnabled})` | ✅ 是（原生窗口） | `:30-38,121-130,217-219` |
| `reduceMotion` / `showStatusBar` / `windowChromeDensity` | 写根元素 `data-reduce-motion` / `data-status-bar` / `data-window-chrome-density` | ✅ 是 | `:40-48,221-223`；`lib/ui-preferences.ts:5-11` |
| `fontFamily` / `uiFontFamily` / `uiFontSize` | 写外观启动缓存（localStorage）——**CSS 变量在下次启动/重新 bootstrap 时生效** | ⚠️ 缓存立即，视觉生效依赖调用方 | `:213-215,132-134`；`lib/appearance-bootstrap.ts:178-197` |
| `ollamaBaseUrl` | `setOllamaBaseUrl()` 推给 provider 注册表 | ✅ 是 | `:205-207,136-146` |
| `aiCustomBaseUrl` | `setCustomProviderBaseUrl()` | ✅ 是 | `:209-211,148-154` |
| （启动时）`syncOllamaApiKey()` | 从安全存储读 Ollama 令牌推给 provider | ✅ 是 | `:161-168,182` |
| 其它所有键 | **没有副作用**，只有对应 feature 自己订阅 store 后生效 | 视 feature | — |

### 4.6 导入 / 导出 / 重置

| 功能 | 入口 | 行为 | 证据 |
| --- | --- | --- | --- |
| **导出** | `tabs/advanced-settings.tsx`（死代码页签）「导出设置」；也有 `settings.keyboard.export` 文案复用 | `save({defaultPath:"lithe-settings.json"})` → `createSettingsExportPayload()` → `writeTextFile(JSON.stringify(payload,null,2))` | `tabs/advanced-settings.tsx:53-84`；`lib/settings-import-export.ts:54-61` |
| **导入（全量）** | 死代码页签「导入设置」 | `createElement("input", file)` → `file.text()` → `updateSettingsFromJSON(text)` | `tabs/advanced-settings.tsx:86-116` |
| **导入（从其它编辑器）** | 常规页签「导入设置」→ `IdeSettingsImportDialog` | 第三方 IDE 设置迁移，**不是** Lithe settings JSON | `macos-settings-panels.tsx:272-292`；`features/file-system/components/ide-settings-import-dialog.tsx` |
| **导入解析** | `parseSettingsImportJson` | JSON.parse → 若 `{format:"lithe.settings", version:1}` 则取 `.settings` → **只保留 `defaultSettings` 里存在的键**（白名单）→ 与默认快照合并 → `normalizeSettings` | `lib/settings-import-export.ts:26-75` |
| **重置** | 对话框底部「恢复默认设置」+ 死代码页签「重置设置」 | `resetToDefaults()`（立即落盘）+ `applySettingsSideEffects` | `settings-dialog.tsx:99-107`；`stores/settings.store.ts:88-97`；`tabs/advanced-settings.tsx:30-33,167-172` |
| **导出/导入格式** | `{format:"lithe.settings", version:1, exportedAt, settings}` | — | `lib/settings-import-export.ts:11-16` |

**导入/导出的实现位置值得注意**：全量导入导出**只存在于死代码页签** `tabs/advanced-settings.tsx`。真实对话框**没有**导出/导入入口（12 个分类里没有 `advanced`）。命令面板也没有。→ gpui 侧若要做「导入/导出设置」，需要**新增**入口，不能从现有 Windows 界面「抄位置」。

### 4.7 键盘快捷键的持久化是**独立**的

键位覆盖**不在 `settings.json`**：走 `features/keymaps/stores/keymaps.store.ts`（`userKeybindings` / `resetToDefaults` / `addKeybinding`），导入导出格式 `createKeybindingsExportPayload` / `parseKeybindingsImportJson`，导出文件名 `keybindings.json`（`tabs/keyboard-settings.tsx:77-78,153-233`）。`settings` 里只有 `keybindingPreset`、`vimMode`、`vimRelativeLineNumbers` 三个键。

---

## 5. 主题系统（`appearance` 页签）

### 5.1 ⚠️ 重大事实：`appearance` 页签**不在真实对话框里**

`components/tabs/appearance-settings.tsx`（587 行，导出 `AppearanceSettings`）**全仓库无引用**（`grep` 只命中自身 `:38`）。真实对话框的「外观」相关项散在**「常规」**页签（配色主题 / 外观模式 / 语言）与**无处可去**。

**但它是 gpui 侧最该照的参考**：它是唯一把主题、图标主题、自定义主题、UI 字体排印、界面开关、布局尺寸全列出的组件。**下面 §5.2 按它逐项登记（并明确标注「未接线」）**。

### 5.2 `appearance` 页签（死代码，587 行）逐项

分组：**主题** → **字体排印** → **界面** → **布局**。

**主题组**（`:196-333`）：

| 设置键 | 标签（中） | 控件 | 默认 | 候选 / 条件 | 证据 |
| --- | --- | --- | --- | --- | --- |
| `syncSystemTheme` | 与操作系统同步 | 开关 | `false` | — | `:197-208` |
| `theme` | 颜色主题 | 下拉（可搜索，宽 `w-44`） | `lithe-dark` | 已注册主题（`normalizedThemeOptions`，含未在列表里的当前值兜底） | `:210-228`（选项 `:74-94`） |
| `autoThemeLight` | 首选浅色主题 | 下拉（可搜索） | `lithe-light` | **只列 `isDark === false` 的主题** | `:230-248`（过滤 `:96-103`） |
| `autoThemeDark` | 首选深色主题 | 下拉（可搜索） | `lithe-dark` | **只列 `isDark === true` 的主题** | `:250-266`（过滤 `:105-112`） |
| `iconTheme` | 图标主题 | 下拉（可搜索） | `idea-icons` | 已注册图标主题 | `:270-286`（选项 `:114-134`） |
| （自定义主题列表） | 每个自定义主题一行：名称 + 描述 `` `${theme.category} custom theme · ${theme.id}` `` + 删除按钮 | 按钮（`variant="danger"`，`icon-xs`） | — | 仅 `getThemeSource(id).kind === "custom"` 的主题 | `:316-332`（筛选 `:68-72`）；删除 `:168-188` |
| （自定义主题操作） | 「创建」+「导入」两个按钮；描述里带外链「格式指南」`${docsUrl}/themes` | 按钮 | — | — | `:288-314`；URL `:67` |
| 条件显示 | `syncSystemTheme` 为 true 时**隐藏**「颜色主题」，改为显示「首选浅色/深色主题」 | — | — | — | `:210,230` |

**字体排印组**（`:335-369`）：

| 设置键 | 标签（中） | 控件 | 默认 | 范围 | 证据 |
| --- | --- | --- | --- | --- | --- |
| `uiFontFamily` | 界面字体 | 字体选择器（`FontSelector monospaceOnly={false}`） | `Microsoft YaHei UI`（`config/typography-defaults.ts:1`） | 捆绑字体 + 系统字体 + 已校验自定义字体 | `:336-348`；`components/font-selector.tsx:40-172` |
| `uiFontSize` | 界面字体大小 | 数字（0.5 步长） | `13`（`typography-defaults.ts:14`） | `10`–`24`，步长 `0.5`（`lib/ui-font-size.ts:3-6`） | `:350-368` |

**界面组**（`:371-436`）：

| 设置键 | 标签（中） | 控件 | 默认 | 候选 | 证据 |
| --- | --- | --- | --- | --- | --- |
| `reduceMotion` | 减少动态效果 | 开关 | `false` | — | `:372-383` |
| `showStatusBar` | 显示状态栏 | 开关 | `true` | — | `:385-396` |
| `showTabIcons` | 显示标签页图标 | 开关 | `true` | — | `:398-409` |
| `tabCloseButtonVisibility` | 标签页关闭按钮 | 下拉 | `active` | `active`「活动和悬停时」/ `hover`「仅悬停时」/ `always`「始终」 | `:411-435` |

**布局组**（`:438-575`）：

| 设置键 | 标签（中） | 控件 | 默认 | 候选 / 范围 / 条件 | 证据 |
| --- | --- | --- | --- | --- | --- |
| `windowChromeDensity` | 窗口框架密度 | 下拉 | `focused` | `focused`「紧凑」/ `comfortable`「舒适」 | `:439-458` |
| `activityRailExpanded` | 展开活动栏 | 开关 | `false` | — | `:460-473` |
| `activityRailWidth` | 活动栏宽度 | 数字 | `180` | `140`–`320`，步长 `10`；**`activityRailExpanded === false` 时禁用** | `:475-492` |
| `sidebarWidth` | 侧边栏宽度 | 数字 | `320` | `140`–`600`，步长 `10` | `:494-510` |
| `nativeMenuBar` | 原生菜单栏 | 开关 | `false` | **仅当平台既不是 mac / win / linux 时显示**（`!IS_MAC && !IS_WINDOWS && !IS_LINUX`，在 Windows 上**永不显示**）；打开时额外 `invoke("toggle_menu_bar")` | `:512-528` |
| `compactMenuBar` | 紧凑菜单栏 | 开关 | `true` | **非 mac 显示**；`nativeMenuBar === true` 时禁用 | `:530-544` |
| `windowTransparency` | 窗口透明度 | 开关 | `false` | — | `:546-559` |
| `openFoldersInNewWindow` | 在新窗口中打开项目 | 开关 | `true` | — | `:561-574` |
| 条件显示 | `CustomThemeCreatorDialog` 以 `isThemeCreatorOpen` 挂在页尾 | — | — | — | `:577-584` |

**未在 `appearance` 页签出现、但属于外观体系的键**：`showActivityRail*`（4 个）、`showGitHub*`（3 个）、`rightToolWindowWidth`、`sidebarActivityItemsOrder`、`hiddenSidebarActivityItems`、`footerLeading/TrailingItemsOrder`、`collapsedActivityRailSections`、`askWhereToOpenProjects`、`maxOpenTabs`、`horizontalTabScroll`、`lastSettingsTab`、`extensionsActiveTab`、`activityRailWidth`… 它们由布局/扩展界面直接改，**没有设置 UI**。

### 5.3 自定义主题在 Windows 侧怎么做的

**主题文件格式**（`extensions/themes/theme-file.ts` + `theme-schema.ts`）：

```jsonc
{
  "name": "Lithe",                       // ThemeSet 名
  "author": "…",                          // 可选
  "themes": [                             // 1..N 个变体
    {
      "id": "lithe-dark",                 // ^[a-z0-9][a-z0-9._-]*$，必需、全文件唯一
      "name": "Lithe Dark",               // 非空字符串
      "appearance": "dark",               // "light" | "dark"
      "colors": { "background": "#1e1f22", "…": "…" },   // 39 键
      "syntax": { "keyword": "…", "…": "…" }             // 18 键（可选）
    }
  ]
}
```

证据：`theme-file.ts:37`（id 正则）、`:154-180`（必填字段校验）、`:191-233`（JSON/结构校验 + 重复 id）、`:278-300`（`toThemeDefinition`）。内置示例：`extensions/themes/builtin/lithe.json`（`id` 在 `:7` = `lithe-light`、`:74` = `lithe-dark`）。

**导入流程**（`components/tabs/appearance-settings.tsx:148-166` → `utils/theme-upload.ts`）：

1. 只能 `.json`，**≤ 2 MiB**（`theme-upload.ts:14,101-116`）。
2. `parseThemeFileJson()` 结构校验；再用 `CSS.supports("color", value)` 逐个校验 `colors.*` / `syntax.*` 的颜色串，不支持则报错（`:58-64`）。
3. **id 冲突检查**：与已注册主题同名且不是自定义来源 → 报错（`:65-78`）。
4. `installCustomThemes()` 写入 `custom-themes.json`（`:80`；`custom-theme-store.ts:44-48`）。
5. 逐个 `themeRegistry.registerTheme(def, {extensionId:"custom-theme.<id>", kind:"custom"})`；若导入的正是当前主题则重新 `applyTheme`（`:84-89`）。
6. 成功后 toast + `selectImportedTheme(firstTheme.id)` —— `syncSystemTheme` 开着就写 `autoThemeDark`/`autoThemeLight`，否则写 `theme`（`:136-146,163`）。

**创建流程**（`components/custom-theme-creator-dialog.tsx`）：
* 弹「创建主题」对话框（`size="lg"`，标题 `customTheme.createTitle`）：**名称** / **ID**（默认 `my-lithe-theme`，名称未手改时按 `themeIdFromName()` 自动生成 slug，正则 `[^a-z0-9]→-`）/ **基础主题下拉**（全部已注册主题）/ **主题 JSON 文本域**（`min-h-72`、等宽，默认由 `formatThemeFile(createThemeFileFromBase({id,name,baseTheme}))` 生成）。
* 三个动作：「取消」/「保存 JSON」（`save({defaultPath:"<id>.json"})` + `writeTextFile`）/「安装主题」（`installThemeJson(json)`）。
* 校验失败时用 `FieldError` 列出最多 8 条问题（`:78-87,215-227`）。
* 默认名 `customTheme.defaultName` =「My Lithe Theme」（`locale.ts` 的 `customTheme.*`）。

**删除自定义主题**（`appearance-settings.tsx:168-188`）：先把 `theme`/`autoThemeLight`/`autoThemeDark` 中指向它的值回落默认，再 `removeCustomTheme(id)`（写 `custom-themes.json`）+ `themeRegistry.unregisterTheme(id)`。非自定义来源会抛「只能移除导入的自定义主题」（`theme-upload.ts:125-133`）。

### 5.4 与 `gpui/themes/lithe-*.json` 的对应关系

**gpui 侧现状**：`gpui/themes/lithe-dark.json` / `lithe-light.json` 是 **gpui-kit `ThemeSet` 格式**：

```jsonc
{
  "$schema": "https://github.com/longbridge/gpui-kit/raw/refs/heads/main/.theme-schema.json",
  "name": "Lithe", "author": "Lithe Team",
  "themes": [
    { "name": "Lithe Dark", "mode": "dark", "font.size": 16.0, "radius": 6, "radius.lg": 8,
      "colors": { "background": "#1e1f22", "primary.background": "#3574f0", … } }
  ]
}
```

证据：`gpui/themes/lithe-dark.json:1-77`（77 行）；加载与 `mode`/`apply_config` 顺序见 `gpui/crates/app/src/main.rs:39-87`。

**逐维对照**：

| 维度 | Windows（Lithe 格式） | gpui（ThemeSet 格式） | 需要做什么 |
| --- | --- | --- | --- |
| 文件容器 | `{name, author?, themes:[…]}` | `{name, author?, themes:[…]}` | ✅ 同构 |
| 单个主题的**标识** | `themes[].id`（`lithe-dark`） | `themes[].name`（`Lithe Dark`）——registry 按 **name** 索引，且 name 全局唯一 | ⚠️ **必须映射**：Windows 的 `id` ↔ gpui 的 `name`；setting `theme` 存的是 Windows id，gpui 侧要用一张 id→name 表（或反过来改 setting 语义） |
| 明/暗 | `themes[].appearance: "light"｜"dark"` | `themes[].mode: "light"｜"dark"` | ⚠️ 字段名不同 |
| 颜色键 | `colors.<简单键>`（`background`、`primary`）**39 键**；`syntax.<键>` **18 键** | `colors.<点分键>`（`background`、`primary.background`、`tab.active.background`、`list.hover.background`…） | ⚠️ **键名与粒度都不同**，需要一张显式映射表（`04-theme-and-components.md` §3.1 的 39 键 → `ThemeColor` 对应表就是这张表的起点；该文档已判定 26 键 gpui 完全不认识） |
| 圆角 | 由 CSS 结构常量 `--radius: 8px` 派生五档（`styles/theme.css:6-12`） | `themes[].radius` / `radius.lg`，**已经在 lithe-*.json 里写死 6 / 8** | ⚠️ 侧不同源（一个是 CSS 派生、一个是主题文件字段） |
| 字号 | 结构常量：`uiFontSize`（默认 13）→ `--app-ui-scale` → `html{font-size: 16px*scale}`（`styles/theme.css:220`、`--app-ui-scale` 由 `appearance-bootstrap.ts:155` 写） | `themes[].font.size`（当前 **16.0**）+ `window.set_rem_size(cx.theme().font_size)` | ⚠️ 换算口径已在 `04-theme-and-components.md` §0.1 定论：**`font_size = 16px × (uiFontSize / 13)`** |
| 字体族 | 不进主题文件：`uiFontFamily` / `fontFamily` 由 settings + localStorage 启动缓存写入 CSS 变量 | `Theme` 里的 `font_family` / `mono_font_family` | ⚠️ 需要把 settings 值映射到 gpui `Theme` 字段 |
| 背景/透明 | `windowTransparency` → `data-window-transparency` + `set_native_window_appearance` | gpui 无对应 | ⚠️ §7.2 判定为「需自研/跳过」 |
| 主题目录监听 | `themeRegistry` + 扩展加载（内置 12 个 JSON 在 `extensions/themes/builtin/`） | `ThemeRegistry::watch_dir(gpui/themes)`，**解析失败整份静默忽略** | ⚠️ 调试困难，见 `main.rs:45-46` |

---

## 6. 其它与设置相关的横向事实

### 6.1 「设置页签」在命令面板里的中文名

`features/command-palette/constants/settings-actions.tsx` 用 `settingsTabCommands`（含 `label`）生成 `Preferences: Open <label> Settings`；`lib/settings-search.ts:18-33` 的 `SETTINGS_SEARCH_TAB_LABELS` 为每个 `SettingsTab` 提供英文 + 中文关键词（例：`run: "Run configurations 运行配置 服务 启动参数 环境变量"`、`project: "Project JDK Maven 项目环境"`、`"ai-commit": "AI & Commit"`）。

### 6.2 设置里**没有**的东西（避免实现时幻想）

| 期望 | 真实情况 | 证据 |
| --- | --- | --- |
| 设置搜索框 | 无 UI；只有索引与打分函数 | `settings-dialog.tsx` 全文 |
| 设置导出/导入入口 | 只在死代码页签 `tabs/advanced-settings.tsx` | §4.6 |
| 「高级」分类 | 对话框 12 个分类里没有 `advanced`；`coreFeatures` 开关只在死代码页签 | `settings-dialog.tsx:35-48`；`tabs/advanced-settings.tsx:15-21` |
| 「外观」分类 | 对话框里没有；入口 `openSettingsDialog("appearance")` 实际落到「常规」 | §1.1 |
| 「文件」分类 | 对话框里没有；只能在「常规→隐藏路径」改两个模式数组 | `settings-dialog.tsx:35-48` |
| Vim 模式开关 | 真实对话框没有；只在死代码 `tabs/keyboard-settings.tsx:362-373` | 同左 |
| 键位编辑/导入/导出 | 真实对话框只有预设下拉 + 一个装饰搜索框 | §3.5 |
| 更新通道 / 自动更新偏好 | 设置里只有「检查更新」按钮 | `macos-settings-panels.tsx:440-483` |
| 设置窗口（独立窗口） | 无，是模态对话框 | §1.1 |
| 恢复默认的「逐项」粒度 | 只有全局「恢复默认设置」；行级重置按钮只在 `tabs/` 的组件里（`SettingRow onReset`） | `settings-dialog.tsx:95-110`；`settings-section.tsx:211-226` |

### 6.3 死代码页签的登记（**不要照抄**，但要知道它们渲染过哪些键）

下表只记「键 → 控件 → 默认」与文件，不复述完整 UI；完整细节需要时再看源码。这 7 个文件是**当前 Windows 真源之外**的代码，`grep` 证明无引用。

| 文件 | 导出 | 行数 | 覆盖的 settings 键 |
| --- | --- | --- | --- |
| `tabs/general-settings.tsx` | `GeneralSettings` | 377 | `displayLanguage`（唯一 settings 键）；其余是版本检查、CLI 安装/卸载/复制、`IdeSettingsImportDialog`、报告问题命令面板（`REPORT_BUG_CHANNELS` = GitHub） |
| `tabs/editor-settings.tsx` | `EditorSettings` | 570 | **30 项**：`fontFamily` `fontSize` `editorFontLigatures` `editorItalicComments` `editorLineHeight` `tabSize` `wordWrap` `lineNumbers` `renderWhitespace` `renderIndentGuides` `highlightOccurrences` `vimRelativeLineNumbers`(条件 `lineNumbers`) `showMinimap` `editorStickyScroll` `editorBracketPairColorization` `editorSmoothScrolling` `editorScrollBeyondLastLine` `editorCursorStyle` `editorCursorBlinking` `maxOpenTabs` `horizontalTabScroll` `autoSave` `defaultLanguage` `autoDetectLanguage` `formatOnSave` `lintOnSave` `autoCompletion` `parameterHints` `inlayHints` `codeLens` `semanticTokens` `breadcrumbShowSymbols`（唯一分组 `settings.editor.section`） |
| `tabs/appearance-settings.tsx` | `AppearanceSettings` | 587 | §5.2 全表 |
| `tabs/keyboard-settings.tsx` | `KeyboardSettings` | 435 | `vimMode`、`keybindingPreset`；另有完整的键位表格编辑器（搜索 + 6 档筛选 all/user/default/preset/preset-changes/extension + 导入/导出/重置），走 `keymaps.store` 而非 settings |
| `tabs/terminal-settings.tsx` | `TerminalSettings` | 578 | **18 项** + 配置文件 CRUD：`terminalDefaultShellId` `terminalDefaultProfileId` `terminalFontFamily` `terminalFontSize` `terminalLineHeight` `terminalLetterSpacing` `terminalScrollback` `terminalAltClickMovesCursor` `terminalMacOptionIsMeta` `terminalRightClickSelectsWord` `terminalCursorStyle` `terminalCursorBlink` `terminalCursorWidth` `terminalCursorInactiveStyle`；配置文件（`useTerminalProfilesStore`）每项含 名称 / Shell / 启动目录 / 启动命令（多行） |
| `tabs/file-tree-settings.tsx` | `FileTreeSettings` | 331 | **13 项**：`fileTreeSortOrder` `fileTreeIndentSize`(8–32) `showFileIconsInFileTree` `showIndentGuidesInFileTree` `compactFoldersInFileTree` `hideRootFolderInFileTree` `showHiddenFilesInFileTree` `showGitignoredFilesInFileTree`(**取反显示**：`checked={!…}`) `showGitStatusInFileTree` `autoRevealActiveFileInFileTree` `confirmBeforeFileDelete` `hiddenFilePatterns` `hiddenDirectoryPatterns`（后两个用逗号分隔、`onBlur`/Enter 提交，与「常规」页签的换行分隔**不一致**） |
| `tabs/advanced-settings.tsx` | `AdvancedSettings` | 176 | `coreFeatures.*`（过滤掉 `git` 与 `UNSUPPORTED_FEATURE_IDS = {github, remote, debugger, aiChat, webViewer}`，所以实际可切 7 个：`terminal` `search` `diagnostics` `outline` `breadcrumbs` `persistentCommands` + `docker`）；另有导出/导入/重置（§4.6） |

> 需要「设置项 → 中文标签」的完整对照时，**直接用 `gpui/crates/shared/locales/lithe.zh-CN.yml`**（`lithe.settings.*`），它由 `gpui/tools/extract-locale.mjs` 从 `windows/tauri/src/i18n/locale.ts` 生成，键与中文逐字一致。设置相关键共 **549 个 `settings.*`** + `aiSettings.*` 79 + `git.setup.*` 34 + `git.execution.*` 40 + `git.fetch.*` 18 + `customTheme.*` 18 + `fontSelector.*` 5。

---

## 7. gpui 侧落地评估

### 7.1 gpui 现有子系统盘点（`gpui/crates/`）

| 子系统 | gpui 现状 | 证据 |
| --- | --- | --- |
| 外壳 / 窗口 / 标题栏 / 活动栏 / 状态栏 / 项目标签 | **已有**（`ShellWorkspace`） | `gpui/crates/workbench/src/workspace.rs`、`title_bar.rs`、`activity_bar.rs`、`status_bar.rs`、`project_tabs.rs` |
| 主题加载 + 明暗切换 | **已有**：`ThemeRegistry::watch_dir(gpui/themes)` + `Theme::change(mode)` + `apply_config`；默认 `Lithe Dark` | `gpui/crates/app/src/main.rs:49-87,196-205` |
| 主题**选择 UI** | **没有**（现由 `--theme <name>` 命令行参数顶替，`main.rs:125-136` 明确写「设置界面（阶段 6）做好后应由设置接管，届时删掉它们」） | 同左 |
| 国际化 | **已有**：`rust_i18n` + `lithe_gpui_shared::{tr, tr_args}`，locale 由 `set_locale` 设定，默认 `zh-CN` | `gpui/crates/shared/src/i18n.rs:81-100`；`main.rs:189-197` |
| 语言**切换 UI** | **没有**（`--locale <tag>` 命令行参数顶替） | `main.rs:134-136,196` |
| 字体 / 字号缩放 | **部分**：`window.set_rem_size(cx.theme().font_size)`（`workbench/src/lib.rs:30`）；主题文件里 `font.size` | `gpui/crates/workbench/src/lib.rs:30`；`gpui/themes/lithe-dark.json:9` |
| 编辑器 | **已有** `EditorPane` | `gpui/crates/editor/src/editor_view.rs` |
| 文件树 | **已有** `Explorer` | `gpui/crates/explorer/src/explorer_view.rs`、`model.rs` |
| Git | **部分**：有 Git Log（`git/src/log_view.rs`）；无变更/暂存面板 | `gpui/crates/git/src/` |
| 终端 | **已有** `TerminalPane` + `terminal/src/profile.rs`（152 行）+ `constants.rs` | `gpui/crates/terminal/src/` |
| 键盘快捷键 / 键位表 | **没有**（无 keymap/预设系统） | `grep` 无命中 |
| AI / 提交信息 | **没有** | `grep` 无命中 |
| 设置持久化 | **没有**（无 store / 无读写） | `grep` 无命中 |
| LSP / 语言服务 | **没有**（`BottomPaneKind` 里是 `Pending("诊断")`） | `workbench/src/workspace.rs:100-110` |

### 7.2 逐页签判定

| 页签 | gpui 是否有对应子系统 | 判定 | 依据 |
| --- | --- | --- | --- |
| `general`（常规） | 主题 ✅ / i18n ✅ / 项目打开方式 ❌ / 隐藏路径 ❌ | **可做**（其中「隐藏路径」需先在 explorer 侧落地过滤，否则是「存了没用」） | §3.1；`gpui/crates/explorer/src/model.rs` |
| `project`（项目 JDK/Maven） | ❌ 无 JDK/Maven 发现（`run` 是 `Pending`） | **不做**（依赖不存在的运行子系统） | `workbench/src/workspace.rs:104` |
| `run`（运行配置） | ❌ 同上 | **不做** | 同上 |
| `editor`（编辑器 4 项） | 编辑器 ✅ | **可做**（`fontSize`/`tabSize`/`wordWrap`/`lineNumbers` 需先把 `EditorPane` 接上设置） | `gpui/crates/editor/src/editor_view.rs` |
| `keyboard`（快捷键） | ❌ 无键位系统 | **不做**（键位预设需要整套 keymap 基础设施） | §7.1 |
| `terminal`（终端 1 项） | 终端 ✅ / shell 检测 ❌ | **半可做**：只有 shell 选择有意义，且 shell 列表目前是硬编码常量；改默认 shell **必须重启终端会话** | `gpui/crates/terminal/src/constants.rs`、`profile.rs` |
| `lsp`（LSP 3 项） | ❌ | **不做**（开关无处生效） | §7.1 |
| `ai` | ❌ | **不做** | §7.1 |
| `ai-commit` | ❌ | **不做** | §7.1 |
| `git`（11 项） | 部分（只有 log） | **暂不做**（11 项里 9 项作用于不存在的 Git 变更面板） | `gpui/crates/git/src/` |
| `logs` | ❌ 无日志系统 | **不做** | — |
| `updates` | ❌ 无更新器 | **不做** | — |
| （死代码）`appearance` | 主题 ✅ / 图标主题 ❌ / 自定义主题导入 ❌ / UI 字体 ⚠️ | **做其主题+字体部分**（见 §7.4） | §5.2 |
| （死代码）`file-tree` | 文件树 ✅ | **可做**（13 项里大部分 explorer 已具备行为） | `gpui/crates/explorer/src/` |
| （死代码）`advanced` | `coreFeatures` 无引用 | **不做** | §6.3 |

### 7.3 每个设置项「能否立刻生效」

**A. 立刻生效（改完当帧可见）**

| 设置键 | gpui 侧对应动作 |
| --- | --- |
| `theme` | `Theme::global_mut(cx).apply_config(&cfg)` + `cx.refresh_windows()` / `window.refresh()` |
| `syncSystemTheme` + `autoThemeLight/Dark` | `Theme::change(mode, None, cx)`（**必须先切 mode 再 apply_config**，`main.rs:64-71`） |
| `windowChromeDensity` | `LitheMetrics` 换档 + 刷新；需重算布局的组件要 `cx.notify()` |
| `showStatusBar` | 状态栏 `display` 切换 |
| `reduceMotion` | 动画时长归零 |
| `tabCloseButtonVisibility` / `showTabIcons` | 标签栏重绘（条件渲染） |
| `sidebarWidth` / `activityRailWidth` / `activityRailExpanded` | `ResizablePanel::size()` / 宽度字段 + 重绘 |
| `fontSize` / `editorLineHeight` / `tabSize` / `wordWrap` / `lineNumbers` | `EditorState` 更新（gpui-kit `Editor` 支持） |
| `uiFontSize` | `window.set_rem_size(px(16.0 * uiFontSize / 13.0))` —— 立刻影响所有 rem 尺寸（`04` §0.1 的换算） |
| `uiFontFamily` / `fontFamily` | 写 `Theme.font_family` / `mono_font_family`，**必须再调 `Theme::sync_base(cx)`**，否则滚动条/把手等仍用旧值（`04-theme-and-components.md:1630` 换肤流程第 ② 步） |

**B. 需要重启 / 重建资源才生效**

| 设置键 | 为什么 |
| --- | --- |
| `terminalDefaultShellId` / `terminalDefaultProfileId` | 已存在的终端会话不会换 shell；**必须新开终端**（Windows 侧语义相同：`settings.terminal.defaultShellDescription` =「用于新的终端会话。」） |
| `terminalFont*` / `terminalScrollback` / `terminalCursor*` | 需要重建 VT 渲染器/尺寸计算（新标签页） |
| `displayLanguage` | Windows 侧是「立即生效」（`settings.mac.languageDescription`），**gpui 侧当前做不到**：`set_locale` 在 `gpui_kit::init` 之前调用一次（`main.rs:196-197`），运行中切换语言需要重新布局全部视图；**建议第一版标为「重启后生效」或只做「下次启动生效」** |
| `windowTransparency` | gpui 侧无对应原生能力（§7.2 / `04` §3.4(6)） |

**C. 只存不生效（第一版接受「存了没用」）**

`hiddenFilePatterns` / `hiddenDirectoryPatterns`（explorer 过滤未接）、`autoSave`、`codeLens`、`horizontalTabScroll`、`keybindingPreset`、`vimMode`、`autoCompletion`、`parameterHints`、`semanticTokens`、`git*` 全部、`coreFeatures.*`、`maxOpenTabs`、`defaultLanguage`、`formatOnSave`、`lintOnSave`、`externalEditor`、`ai*` 全部。

**D. gpui 侧根本没有对应物，做不到（应隐藏或标注）**

| 设置键 | 为什么做不到 |
| --- | --- |
| `iconTheme` | gpui 只有一套图标资产（`gpui_kit::assets::AllAssets`，1830 个 Lucide 字形），**没有图标主题系统**（`gpui/crates/app/src/main.rs:180-187`） |
| `nativeMenuBar` | Windows 侧本来就不显示这一项（`appearance-settings.tsx:512` 的 `!IS_MAC && !IS_WINDOWS && !IS_LINUX`） |
| `compactMenuBar` | gpui 无「界面菜单栏 / 原生菜单栏」二选一；`UI-MAP-WINDOWS.md:514` 判定需用 `PopupMenu` 自搭 |
| `codeLens` / `inlayHints` / `parameterHints` / `semanticTokens` / `autoCompletion` / `formatOnSave` / `lintOnSave` | 无 LSP / 格式化 / 静态检查子系统（§7.1），开关无处生效 |
| `externalEditor` / `customEditorCommand` | 无外部编辑器集成 |
| `git*` / `coreFeatures.*` / `ai*` / `aiCommit.*` | 对应子系统不存在（§7.1） |
| `windowTransparency` / `nativeMenuBar` 的 `invoke("toggle_menu_bar")` | gpui 侧无对应原生窗口能力（`04-theme-and-components.md` §3.4(6)） |

### 7.4 建议的第一版范围

**阶段 A（第一版，只做两页，但把「设置能改主题」这条闭环打通）**

1. **`general` —「常规」**：`displayLanguage`（标「重启生效」）、`askWhereToOpenProjects` + `openFoldersInNewWindow`（合成一个三选下拉，照 `lib/project-open-preference.ts`）、`hiddenFilePatterns` / `hiddenDirectoryPatterns`（可以先只存，但要登记「暂不生效」）。**主题不放这里**（见下条）。
2. **新建 `appearance` —「外观」**（照 `tabs/appearance-settings.tsx` 但**只取已生效子集**）：`syncSystemTheme`、`theme`、`autoThemeLight` / `autoThemeDark`（同步开启时显示后两者）、`uiFontFamily`、`uiFontSize`、`reduceMotion`、`showStatusBar`、`showTabIcons`、`tabCloseButtonVisibility`、`windowChromeDensity`、`activityRailExpanded`、`activityRailWidth`、`sidebarWidth`。**主题只在这一页出现一次**——Windows 真源里「常规」与「外观（死代码）」都能改主题，参考实现时不要把这个重复带过来。
3. **持久化**：新建 `lithe-gpui-settings` 的 JSON 文件（放应用数据目录），照 §4 的语义：逐键默认回退 + 对象一层浅合并 + **300ms 防抖写** + 启动时规范化（把 §4.3 里**已经有效的键**的规范化搬过来，尤其 §4.3 表里的第 14/15/18/22 条这批白名单与范围钳制）。
4. **入口**：`Ctrl+,`（Windows 用 `ctrl-,`）+ 标题栏/活动栏按钮，走 `openSettingsDialog(Some(Tab))` 等价的 action。

**阶段 B（第二批，等对应子系统落地后）**

| 页签 | 前置条件 |
| --- | --- |
| `editor` | `EditorPane` 接上设置（字号/行高/缩进/换行/行号） |
| `file-tree`（13 项） | explorer 接上排序/缩进/图标/过滤 |
| `terminal` | shell **发现**（现在硬编码）+ 终端重建路径 |
| `project` | JDK/Maven 发现（`Pending("Maven")` 落地） |

**阶段 C（暂不做，理由）**

| 页签 | 理由 |
| --- | --- |
| `keyboard` / `lsp` / `ai` / `ai-commit` / `logs` / `updates` / `run` | 对应子系统**完全不存在**，UI 做了也只是空壳 |
| `git` | 11 项里 9 项作用于不存在的变更面板；先做 git 变更面板，再回来做设置 |
| `advanced` | `coreFeatures` 在 gpui 侧没有任何读取方 |
| 设置内搜索 | 第一版不需要（Windows 真源也没有）；`gpui-kit` 的 `Settings` 组件**自带**搜索，可以「白送」 |
| 导入/导出/自定义主题 | 需要主题文件格式转换层（§5.4）；建议紧跟主题页之后做 |

---

## 9. gpui-kit 0.6.6 组件选型

> 依据：本地镜像 `gpui/docs/gpui-kit/0.6.6/zh-CN/**`（**0.6.6 默认版**，非在线 `versions/main`）。页码 = `component/<名>.md` / `shell/<名>.md`，小节标题照抄。

### 9.1 硬约束：对话框必须在 `Root::new` 之后、且只能从事件/任务打开

| 约束 | 原文 / 证据 |
| --- | --- |
| `Root` 必须是窗口**第一层**子节点 | `component/root.md:8-10`：「必须把 [Root] 作为窗口中的 **第一层子节点**……如果不把 [Root] 放在窗口的第一层，许多行为都会出现异常或不符合预期。」 |
| 必须在 `Root` 之下渲染浮层 | `component/root.md:61-73`：用 `Root::render_dialog_layer` / `render_sheet_layer` / `render_notification_layer`，且要用 `children`（返回 `None` 时不渲染） |
| **本仓库已确认的 panic 约束** | `gpui/crates/app/src/main.rs:14-16`：「`Root` **必须是窗口的第一层**：它负责对话框、抽屉、通知与焦点归还；`Root::new` 之前不得打开任何浮层（那时窗口根还不是 `Root`，`window.open_dialog` 会 panic）。」同句也在 `gpui/crates/workbench/src/workspace.rs:150` 复述 |
| **只能从事件回调或任务中打开/关闭** | `shell/overlays.md:203-213`：「浮层只能从事件回调或任务中打开与关闭。……`window.open_dialog(content, options) is not allowed during the 'render' phase`」——`render` / `layout` / 无 Host 调用三种情形全部拒绝 |
| 未知选项会被**拒绝**而不是忽略 | `shell/overlays.md:91-98` |
| 浮层需要 `ShellRoot` 作第一层视图 | `shell/overlays.md:214-221`（Rust 侧对应 `Root`） |
| `open_dialog` 返回**栈深度**，不是句柄 | `shell/overlays.md:100`、`:227` |
| Escape 只关最上层；回车不外溢 | `shell/overlays.md:190-198` |
| 本仓库当前浮层挂载点 | `gpui/crates/workbench/src/workspace.rs:236-242`（`ShellWorkspace::render` 里三个 `render_*_layer`；⚠️ 注释说「顺序不可调换：这三个借用 `cx`，必须先把值取出来」） |

**结论**：设置界面**可以**是 `Dialog`，但打开它的调用必须发生在**点击事件处理器**里（不能放在 `render` 里或 `Root::new` 之前）。当前 `ShellWorkspace` 已经挂了 dialog/sheet/notification 三层，**不需要新增挂载点**。

### 9.2 组件选型表

| 用途 | 推荐组件 | 构造方式（确切签名要点） | 状态放哪 | 文档位置 |
| --- | --- | --- | --- | --- |
| **设置页主体（首选）** | `setting::Settings` + `SettingPage` + `SettingGroup` + `SettingItem` + `SettingField` | `Settings::new("my-settings").pages(vec![SettingPage::new("常规").group(SettingGroup::new().title("外观").items(vec![…]))])` | 值读写走闭包：`SettingField::switch(|cx:&App| …, |val:bool, cx:&mut App| …)`；**状态在调用方的 `Global`/`Entity`**，组件不持有 | `component/settings.md`（全文 568 行）；导入 `use gpui_kit::component::setting::{Settings, SettingPage, SettingGroup, SettingItem, SettingField};`（`:15`） |
| **`SettingField` 种类** | `switch` / `checkbox` / `input` / `dropdown` / `number_input(NumberFieldOptions{min,max,..})` / `render(closure)` / `element(impl SettingFieldElement)` | 例：`SettingField::dropdown(vec![(value.into(), label.into())], getter, setter)`（`:359-372`）；`SettingField::number_input(NumberFieldOptions{min:8.0,max:72.0,..Default::default()}, getter, setter)`（`:381-393`） | 同上 | `component/settings.md:303-451` |
| **分页** | `SettingPage::new("常规").icon(IconName::Settings).groups(...)` / `.group(...)` / `.default_open(true)` / `.resettable(true)` / `.title_suffix(|_,_| …)` | 多页在 `Settings::pages(vec![…])` 里给 | 组件内部管「当前页」，**搜索时会自动选页** | `component/settings.md:108-166`、`:87-92` |
| **搜索** | **`Settings` 自带**（按标题/描述/`keywords(…)` 过滤；`SettingItem::keywords(["MFA","2FA"])`） | 不需要自己写搜索框 | 组件内部 | `component/settings.md:279-301`、`:87-92` |
| **分组外观** | `Settings::with_group_variant(GroupBoxVariant::Outline \| Fill)` | — | — | `component/settings.md:94-106` |
| **尺寸** | 全族实现 `Sizable`：`xsmall()`/`small()`/`medium()`(默认)/`large()`/`with_size(Size)` | — | — | `component/settings.md:462-470` |
| **禁用整行** | `SettingItem::disabled(true)`（内置字段自动禁用；`render` 自定义项要自己读 `options.disabled`） | — | — | `component/settings.md:247-277` |
| **纵向布局项** | `SettingItem::layout(Axis::Vertical)` | 路径/长文本用它 | — | `component/settings.md:222-233` |
| **开关** | `Switch::new("id").checked(bool).on_change(fn(&bool,…))` | **受控**：值由调用方保存并 `cx.notify()` | 调用方 | `component/switch.md:22-47,120-143`；`small()` ≈ 28×16 |
| **下拉** | `Select::new(&state)`，`state = cx.new(|cx| SelectState::new(items, selected, window, cx))`；`SearchableVec` + `.searchable(true)` 支持搜索 | **Stateful**：`Entity<SelectState<_>>` 由调用方持有；事件 `cx.subscribe_in(&state, window, …)` 收 `SelectEvent::Confirm(value)` | 调用方持有 `Entity<SelectState>` | `component/select.md:31-97,210-247` |
| **数字** | `NumberInput::new(&input_state)`；`InputState::new(window,cx).default_value("14").min(8.).max(32.).step(1.)` | **Stateful**（`Entity<InputState>`）；内置递增/递减按钮；失焦收敛到范围 | 调用方 | `component/number-input.md:20-74,210-253` |
| **文本** | `Input::new(&state)`；`InputState::new(window,cx).placeholder(..).default_value(..)` | Stateful | 调用方 | `component/input.md:20-56` |
| **多行/glob 列表** | `Textarea::new(&state)`（`TextareaState`） | Stateful | 调用方 | `component/textarea.md` |
| **按钮** | `Button::new("id").label(..).primary()/.ghost()/.outline().on_click(|_,window,cx| …)` | Stateless | — | `component/button.md` |
| **对话框容器** | `window.open_dialog(cx, |dialog, window, cx| dialog.title(..).child(..).footer(|_,_,_,_| vec![Button…]))`；关闭 `window.close_dialog(cx)` | 内容闭包**每次重绘都会重新执行**，闭包捕获的东西就是 dialog 的状态 | 数据由闭包捕获；**不要闭包捕获 `cx`**（会 stale） | `component/dialog.md:49-118,185-196`；`shell/overlays.md:100-106` |
| **抽屉（备选，见 9.3）** | `window.open_sheet(cx, |sheet,_,_| sheet.title(..).size(px(820.)).child(..).footer(..))`；`open_sheet_at(Placement::Left, …)`；`close_sheet(cx)` | 同时最多 1 个 sheet | — | `component/sheet.md:49-193` |
| **侧栏导航** | `sidebar::{Sidebar, SidebarHeader, SidebarFooter, SidebarGroup, SidebarMenu, SidebarMenuItem}`；`SidebarMenuItem::new("常规").icon(IconName::Gear).active(bool).on_click(..)` | 组件无选中状态托管，`active(bool)` 由调用方给 | 调用方 | `component/sidebar.md:23-51,81-92,149-161` |
| **页签条（备选）** | `tab::{TabBar, Tab}`；`TabBar::new("id").selected_index(usize).on_click(|ix,..|).child(Tab::new().label("常规"))`；`.pill()/.underline()/.outline()/.segmented()/.menu(true)` | 选中态由 `TabBar` 统一管理（`selected_index`） | 调用方持有 index | `component/tabs.md:20-29,216-256` |
| **命令面板式搜索** | `command::{Command, CommandState}` | ⚠️ `Command` 只是普通 `v_flex`，**不能当浮层**，必须放进 Dialog | Stateful | `UI-MAP-WINDOWS.md:1472`（引用）；`component/command.md` |
| **图标** | `Icon::new(IconName::Gear)`；⚠️ 应用注册的是 `gpui_kit::assets::AllAssets`（全量 1830 字形），所以 `git-*` 等可用 | — | — | `gpui/crates/app/src/main.rs:180-187`；`UI-MAP-WINDOWS.md:1480,1675` |
| **主题 token** | `cx.theme().background` / `.foreground` / `.border` / `.primary` / `.muted` / `.radius` | — | — | `component/theme.md:8-17` |

### 9.3 我的选型建议（含理由）

1. **优先 `setting::Settings` 全家桶做「中间内容」**，因为它**已经内置**：分页、分组、行（标题+描述）、字段（switch/checkbox/input/dropdown/number/render/element）、**按标题/描述/关键词搜索过滤**、**整行禁用**、**可重置（`resettable(true)`）**、尺寸族（`component/settings.md:22-30,87-92,247-301,462-470`）。这几乎逐条对应 Windows 的 `Section` + `SettingRow` + 行级重置按钮（`settings-section.tsx:51-244`）。
   * 代价：`SettingField` 的候选类型是**固定枚举**，Windows 里「路径选择器 + 浏览按钮」「字体族选择器（需要枚举系统字体）」「快捷键录制」「自定义主题 JSON 文本域」这四类必须走 `SettingField::render(...)` 或 `SettingField::element(impl SettingFieldElement)`（`component/settings.md:396-451`）。
2. **左侧 12 项分类导航**：若用 `setting::Settings`，页签导航由组件自己画（`SettingPage`），**不需要** `Sidebar`/`TabBar`。若要还原 Windows 的「独立左侧 190px 分类栏 + 右侧滚动区」布局，则用 `Sidebar`（宽 `width(190)`，`SidebarMenuItem::new(label).icon(..).active(..)`，`component/sidebar.md:149-161`）或 `TabBar` 竖排 — ⚠️ **`TabBar` 没有垂直方向**（`UI-MAP-WINDOWS.md:272`：终端垂直页签条「gpui-kit 的 TabBar 无 vertical → 自绘」），所以左栏用 `Sidebar` 或自绘 `v_flex` + `Button`。
3. **外层容器**：
   * **不建议 `Dialog`**：`Dialog` 默认宽 **448**，要 `.width(px(820.))` 覆盖（`UI-MAP-WINDOWS.md:1473`）；垂直默认 `viewport/10`（要 `.margin_top(px(64.))`）；而且 Dialog 的内容是**单一滚动区**，再嵌「固定左栏 + 右栏独立滚动 + 固定 footer」需要小心：`component/dialog.md:120-122` 说明 Dialog **不会超过窗口**，正文在内部滚动——这与 Windows 的「左栏固定、右栏滚动」正好可以组合（把左右栏放进同一个 `.child()`，右栏自己 `overflow_y_scroll`）。
   * **建议 `Sheet`**（`open_sheet` + `.size(px(820.))`）：语义上「设置」是窗口里的一块面板而不是需要回答的模态问题，而且 `Sheet` 的 `footer(..)`/`resizable(true)`/`margin_top(px(..))` 都现成（`component/sheet.md:73-140,175-185`）。缺点是 Sheet **贴边**、同时只允许 1 个，且 Escape 行为受 dialog 栈影响（`shell/overlays.md:196`）。
   * **也可以自绘全窗覆盖层**（`absolute inset_0` + 半透明底 + 居中卡片）：完全可控，代价是丢掉了焦点陷阱与 Escape/遮罩语义，要自己实现（`FocusTrapElement` 可用：`UI-MAP-WINDOWS.md:1480` 提到 `.focus_trap(id, &handle)`）。
   * **最终取舍交给实现者**：如果第一版就要「遮罩 + Esc 关闭 + 焦点归还」全部白送，**用 `Dialog` + `.width(px(820.)).h(px(620.))`**；如果要「非模态、可和主界面共存」，**用 `Sheet`**。
4. **保存时机**：Windows 是「每次改都写 + 300ms 防抖」（§4.5）。`SettingField` 的 setter 闭包天然是「改即回调」，正好对应：setter 里更新全局设置 + 触发副作用 + 打一个 300ms 的防抖定时器（`cx.spawn` + `Timer`，参考 `component/settings.md:544-556` 的写法风格）。
5. **`displayLanguage` 的 UI 必须标注**：gpui 侧 `set_locale` 只在启动早期调用一次（`main.rs:196`），运行中切语言做不到「立即生效」。第一版应把这一项写成**「下次启动生效」**并在描述里说明，或者干脆不做。

---

## 10. 我没能确认的点

| # | 未确认 | 影响 | 建议 |
| --- | --- | --- | --- |
| 1 | ~~`ai-commit-settings-panel.tsx` / `ai-settings.tsx` 的逐行 UI 结构~~ → **已补齐**（§3.8 的 8 个 Section 表、§3.9 的条件控件表），但 `CodexSettings`（`features/ai/integrations/codex/codex-settings.tsx`）、`ProviderApiKeyCommand`、`ModelSelector`、`ProviderSelector` 这 4 个子组件的内部结构我**没有读** | 若要实现 AI 页需要补读 | 第一版不做 AI 页（§7.4） |
| 2 | `general` 页签里 `theme` 下拉在 `syncSystemTheme === true` 时的**真实行为**：`handleThemeChange` 会写 `autoThemeDark/Light`，但该项在同步模式下**不渲染**，所以这段代码是否可达我没验证 | 不影响 gpui 设计（我会把「配色主题」只放在外观页） | 实现时按「同步模式只显示一对首选主题」处理 |
| 3 | `AI 页` 归属：`aiSettings.*` 的 79 个键里，哪些属于上述 4 个子组件**我没有逐个归属**；两个 Section 标题（`Lithe Agent` / `Ollama`）**没有 i18n 键**（硬编码英文），gpui 侧要么沿用它、要么新增键 | 影响 AI 页的完整文案表 | 需要时读那 4 个子组件 |
| 4 | `git` 页的「Fetch 与提交偏好」`<details>` 默认收起，`open` 由本地 state 控制；展开后 `GitIdentitySettings` 才挂载（`git-settings.tsx:45-46,66`） | 只是交互细节 | 已记录 |
| 5 | `updates` 页背后的 `UpdaterStore` / `update-preferences.ts`（3573 B）/ `use-updater.ts` 的完整偏好模型我**没有展开**（设置界面只暴露一个「检查更新」按钮） | 若 gpui 要做更新器才需要 | 第一版不做 |
| 6 | `logs` 页的 `log-api.ts`（后端命令清单）我没读 | 第一版不做日志页 | — |
| 7 | `project` / `run` 两页依赖的 `services/project-environment.ts`、`run.store`、`maven.store` 我没读全 | 第一版不做 | — |
| 8 | gpui 侧 `ThemeRegistry::watch_dir` 是否支持**运行中新增主题文件后自动重载并可选**（Windows 的「导入自定义主题」需要这个） | 决定「自定义主题导入」能否第一版做 | 现在只能确认 `watch_dir` 会重载（`main.rs:56-61` 注释：「`watch_dir` 会先 `reload_themes` 再调这个回调」） |
| 9 | gpui-kit `setting::Settings` 的**页签导航外观**（是否等于 Windows 的左侧 190px 列表）我没有逐行读源码，只读了文档 | 影响「用不用自己画左栏」 | 建议实现前先跑一个最小 `Settings` 组件看外观 |
| 10 | `.theme-schema.json`（gpui 侧）的确切字段清单我没读，只看了 `lithe-dark.json` 的实际字段与 `component/theme.md` | 影响主题转换器 | `04-theme-and-components.md` §3.4 已有部分结论 |
