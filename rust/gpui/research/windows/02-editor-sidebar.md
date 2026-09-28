# 02 · 编辑区与侧栏 · Windows 前端逆向规格

> 范围：`windows/tauri/src/features/panes/`、`features/tabs/`、`features/editor/`（宿主、面包屑、查找栏、状态）、`features/layout/components/sidebar/`、`features/file-explorer/`、`features/sidebar/`、`features/global-search/`（仅侧栏/面板部分）、`windows/tauri/src/ui/`（被引用的基础控件）、`windows/tauri/src/styles/`、`windows/tauri/src/i18n/locale.ts`。
> 目标：作为 Rust + gpui-kit 0.6.6 重写「编辑区 + 侧栏」的规格真源，可直接照着写代码。
> 旧 macOS 规格（`.artifacts/ui-map/04-editor.md` 等）已作废，本文只以 Windows 前端为准。
> 证据约定：全部写 `相对路径:行号`（相对仓库根）。查不到的写「未找到」，不推测。
> 本文未修改任何已有文件；未运行 bun/npm/cargo；未触碰 `macos/`。

---

## 0. 先决结论（先看这 5 条，否则会从错误前提开工）

| 事实 | 证据 |
|---|---|
| 编辑区 = **React 画 chrome + Monaco 画文本**。标签栏、面包屑、空态、冲突横幅、状态栏全部是 React DOM；行号/折叠/minimap/查找替换/诊断波浪线/补全悬停由 Monaco 自渲染。 | `windows/tauri/src/features/panes/components/pane-container.tsx:1094-1102`、`windows/tauri/src/features/editor/components/code-editor.tsx:647-773`、`windows/tauri/src/ui/tab-bar.tsx:169-297` |
| 「标签」不是文件，而是 **buffer**（内容类型有 24 种：editor/terminal/diff/image/pdf/database/webViewer/pr/issue…）。标签栏按 buffer 渲染，显示名由路径缩短算法算出。 | `windows/tauri/src/features/panes/types/pane-content.types.ts:33-57`、`windows/tauri/src/features/tabs/components/tab-bar.tsx:271-303`、`windows/tauri/src/features/tabs/utils/path-shortener.ts:31-113` |
| 标签/窗格**状态不在 Core**：buffer 列表、激活项、pin、preview、窗格树全在渲染进程 zustand，工作区会话持久化到前端 storage（key `lithe-tab-sessions`）。Core 只提供文件读写、工作区快照/搜索、LSP。 | `windows/tauri/src/features/editor/stores/buffer.store.ts:620-700`、`windows/tauri/src/features/window/stores/session.store.ts:256-266`、`windows/tauri/src/features/panes/stores/pane.store.ts:244-600` |
| **不存在 `features/diff/` 目录**。编辑区里的 diff 是 `features/git/components/diff/*`，经懒加载 `DiffViewer` 挂进 pane 的 `case "diff"`。 | `windows/tauri/src/features/panes/components/pane-container.tsx:77,951-952` |
| **不存在 `SearchSidebar` / `search-sidebar` 组件**。侧栏的「搜索」就是同一个 `GlobalSearchBuffer` 传 `compact`。 | `windows/tauri/src/features/layout/components/sidebar/main-sidebar.tsx:797-804`、`windows/tauri/src/features/global-search/components/global-search-buffer.tsx:51-54` |

---

## 1. 区域清单（组件层级）

### 1.0 编辑区在工作台中的位置

```
main-layout.tsx:295-355                       windows/tauri/src/features/layout/components/main-layout.tsx
├─ SidebarActivityRail expanded={false}       :302 → sidebar/main-sidebar.tsx:120
├─ ResizablePane(position="left")             :303-310 → layout/components/resizable-pane.tsx:32
│   └─ MainSidebar                            sidebar/main-sidebar.tsx:767
├─ <div .lithe-glass-island rounded-xl border-l bg-background>   :312-313  ← 编辑区「玻璃岛」
│   └─ WorkbenchErrorBoundary > CachedWorkspaceSplitViews        :314-315
│       └─ SplitViewRoot（每个 workspace 一份，最多缓存 3 个）     panes/components/split-view-root.tsx:12,46,98-118
│           └─ PaneNodeRenderer               panes/components/pane-node-renderer.tsx:54
│               ├─ PaneContainer（group 叶子） panes/components/pane-container.tsx:319
│               │   ├─ TabBar                 tabs/components/tab-bar.tsx:61
│               │   ├─ EmptyEditorState（无 activeBuffer 时）      pane-container.tsx:1100 → panes/components/empty-editor-state.tsx:10
│               │   ├─ CodeEditor（默认分支）  pane-container.tsx:1050-1058
│               │   ├─ 其它 buffer 渲染器（terminal/webViewer/diff/pr/issue/image/pdf/database/…）  pane-container.tsx:924-1059
│               │   ├─ SplitDropOverlay        pane-container.tsx:1089-1093
│               │   └─ 拖入高亮层              pane-container.tsx:1086-1088
│               └─ PaneResizeHandle（同级）    pane-node-renderer.tsx:115-124 → panes/components/pane-resize-handle.tsx:13
├─ BottomPane（底部面板，pane id = "bottom-pane"）  :318-322 / :345-351 → layout/components/bottom-pane/bottom-pane.tsx:28
└─ Footer（状态栏）                            :354 → layout/components/footer/footer.tsx:16
```

### 1.1 TabBar 自身层级

```
TabBar                                        windows/tauri/src/features/tabs/components/tab-bar.tsx:61
├─ TabDndContext                              ui/tab-bar.tsx:30（PointerSensor distance=5）
│   └─ TabBarSurface（role="tablist"，bg-background）              tab-bar.tsx:626-632 / ui/tab-bar.tsx:284
│       ├─ div.flex.h-8（左侧导航组 gap-0.5）                       tab-bar.tsx:633
│       │   ├─ Button(ArrowLeft 后退, size=icon-xs)                 tab-bar.tsx:634-646
│       │   └─ Button(ArrowRight 前进, size=icon-xs)                tab-bar.tsx:647-659
│       ├─ SortableContext > div（横向滚动容器 gap-0.5）             tab-bar.tsx:662-663
│       │   └─ SortableTab（每 buffer 一个，horizontalListSortingStrategy）  tab-bar.tsx:665 / ui/tab-bar.tsx:61
│       │       └─ ContextMenu > ContextMenuTrigger.contents > TabBarItem      tab-bar.tsx:675-694
│       │           └─ TabBarItem             tabs/components/tab-bar-item.tsx:55
│       │               ├─ 拖放指示条 div.drop-indicator（可选）    tab-bar-item.tsx:134-136
│       │               └─ TabBarTab（role="tab"）                ui/tab-bar.tsx:269
│       │                   └─ Tab（variant=connected, action=关闭/固定按钮）  ui/tab-bar.tsx:220
│       │                       ├─ 图标槽 div.grid.size-3           tab-bar-item.tsx:182-254
│       │                       ├─ 标签文字 span / InlineRenameInput  tab-bar-item.tsx:255-283
│       │                       └─ 脏标记 div.size-2.rounded-full    tab-bar-item.tsx:284-291
│       │       └─ TabContextMenu（右键菜单，同一 ContextMenu 内）   tab-bar.tsx:695-755 → tabs/components/tab-context-menu.tsx:50
│       └─ div.flex.h-8（右侧操作组 gap-1 pl-0.5）                   tab-bar.tsx:763
│           ├─ MarkdownModePicker（仅 editor buffer）               tab-bar.tsx:764-766
│           ├─ Button(Plus 新建标签, hover 才显示)                   tab-bar.tsx:767-784
│           ├─ Button(PanelLeftClose 关闭拆分, 仅分屏时)             tab-bar.tsx:785-797
│           └─ Button(Maximize2/Minimize2 全屏, hover 才显示)        tab-bar.tsx:798-815
├─ div.sr-only role="status"（无障碍播报）                           tab-bar.tsx:821-823
└─ useTabWheelScroll（横向滚轮）                                      tabs/hooks/use-tab-wheel-scroll.ts:3
```

### 1.2 编辑器宿主层级

```
CodeEditor                                    windows/tauri/src/features/editor/components/code-editor.tsx:647-773
├─ EditorStylesheet（隐藏编辑器滚动条、面包屑禁选中）  code-editor.tsx:649 → components/stylesheet.tsx:1-51
├─ div.absolute.inset-0.flex.flex-col.overflow-hidden  code-editor.tsx:650
│   ├─ Breadcrumb（showToolbar 时）            :652-660 → components/toolbar/breadcrumb.tsx:48
│   │   ├─ div.h-7（面包屑栏）                 toolbar/breadcrumb.tsx:220
│   │   │   ├─ FilePathBreadcrumb              toolbar/breadcrumb.tsx:225-229 → toolbar/file-path-breadcrumb.tsx:31
│   │   │   │   ├─ PathBreadcrumb → ui/breadcrumb.tsx 各件   toolbar/path-breadcrumb.tsx:23
│   │   │   │   └─ Dropdown（目录下拉 / 返回上级）             toolbar/file-path-breadcrumb.tsx:205-274
│   │   │   ├─ SymbolBreadcrumb（LSP 符号路径）                toolbar/symbol-breadcrumb.tsx:30
│   │   │   ├─ BreadcrumbActionButton(MoreHorizontal + 操作菜单)  toolbar/breadcrumb.tsx:195-214
│   │   │   └─ rightContent / 扩展工具条                        toolbar/breadcrumb.tsx:243-259
│   ├─ ExternalConflictBanner（磁盘冲突）      :662 → components/external-conflict-banner.tsx:13
│   ├─ 大文件提示条（可点「仍然启用」）         :664-675
│   └─ div.editor-container（relative min-h-0 flex-1）  :677-687
│       ├─ CodeLensOverlay / SignatureHelpTooltip / RenameInput   :688-728
│       └─ div.absolute.inset-0.bg-background（主编辑面）:731
│           ├─ Spinner 恢复中 / 恢复失败重试卡片        :732-747
│           ├─ MarkdownSplitEditor | MarkdownPreview | HtmlPreview | CsvPreview | NotebookEditor  :748-757
│           └─ SvgEditor > MonacoEditor        :758-765
├─ ScrollDebugOverlay（调试）                  :771
└─ MarkdownSplitEditor（左 Monaco 右预览，复用 PaneResizeHandle）  :787-895
```

### 1.3 底部面板与状态栏

```
BottomPane                                    windows/tauri/src/features/layout/components/bottom-pane/bottom-pane.tsx:28
├─ div（resize 手柄 h-(--lithe-workbench-gap)）:222-241
└─ div.lithe-glass-island（rounded-xl border-t border-l bg-background）  :243-254
    ├─ TerminalContainer（常驻，用 hidden 切换）  :257-264
    ├─ DebuggerView / RunPane / MavenRunPane / DiagnosticsBuffer / BottomBufferPane / GitLogToolWindow  :266-307
    └─ BottomBufferPane（含 TabBar，paneId="bottom-pane"）  layout/components/bottom-pane/bottom-buffer-pane.tsx

Footer                                        windows/tauri/src/features/layout/components/footer/footer.tsx:16
├─ ProjectPreparationStatus compact           footer.tsx:51
├─ useFooterFilePathItem（文件路径面包屑）      footer.tsx:24,52-56 → footer/footer-file-path-item.tsx
├─ useFooterEditorStatusItems                 footer.tsx:25 → footer/footer-editor-status.tsx:26
│   ├─ CursorPositionChip（可点击编辑跳转行:列）  footer-editor-status.tsx:70-76 → editor/components/toolbar/editor-status-actions.tsx:11
│   ├─ encoding / indent / readOnly 标签        footer-editor-status.tsx:77-106
│   ├─ memory 轮询项（10s）                    footer-editor-status.tsx:24,45-67,107-121
│   └─ gitChanges chip                        footer-editor-status.tsx:122-141
└─ FooterStatusChip / FooterStatusLabel        footer/footer-status-chip.tsx
```

### 1.4 侧栏层级

```
SidebarActivityRail（活动栏 rail）              windows/tauri/src/features/layout/components/sidebar/main-sidebar.tsx:120
└─ SidebarPaneSelector orientation="vertical"  :632 → sidebar/sidebar-pane-selector.tsx:81
    └─ nav[aria-label=workbench.activityViews]  sidebar-pane-selector.tsx:358
        ├─ 上组 div（flex-1 overflow-y-auto gap-1）:359 → SidebarListItem  :323 → ui/sidebar.tsx:217
        └─ 下组 div（shrink-0 gap-1 pt-1）       :363
MainSidebar                                    main-sidebar.tsx:767
└─ div[data-external-file-drop-scope="sidebar"]  :814
    └─ SidebarPanel                             :815 → ui/sidebar.tsx:11
        └─ 三选一（coreFeatures + activeSidebarView 决定）  :778-816
            ├─ GitView（git）                   :784-789 → git/components/git-view
            ├─ FileExplorerPane（files）         :794-796 → file-explorer/components/file-explorer-pane.tsx:9
            │   ├─ FileExplorerTree             file-explorer-pane.tsx:35 → file-explorer/components/file-explorer-tree.tsx:1108
            │   │   ├─ div.file-explorer-shell（拖拽/右键事件总入口）  file-explorer-tree.tsx:1108
            │   │   ├─ SidebarHeader.file-explorer-header              file-explorer-tree.tsx:1309-1518
            │   │   │   ├─ h2.file-explorer-header-title（「项目」）     :1314
            │   │   │   ├─ SidebarSearchPopover（树内搜索）             :1317-1343
            │   │   │   ├─ 清空按钮（有查询时）                         :1344-1356
            │   │   │   └─ 偏好 DropdownMenu（可见性/外观/缩进）        :1357-1517
            │   │   ├─ FileExplorerViewport role="tree"                file-explorer-tree.tsx:1519-1523 → file-explorer-viewport.tsx:169
            │   │   │   └─ div.file-tree-container → .file-tree-virtual-canvas → .file-tree-virtual-row  viewport.tsx:170-198
            │   │   │       └─ FileExplorerTreeItem                     file-explorer-tree.tsx:1579 → file-explorer-tree-item.tsx:88
            │   │   │           ├─ 编辑态 div.file-tree-edit-row + InlineRenameInput  :148-200
            │   │   │           └─ SidebarTreeRow                       :203 → features/sidebar/components/sidebar-tree.tsx:183
            │   │   │               ├─ SidebarTreeGuides（缩进参考线）    sidebar-tree.tsx:18
            │   │   │               ├─ SidebarTreeDisclosure（三角）      sidebar-tree.tsx:290
            │   │   │               ├─ ThemedFileIcon（文件图标）         file-explorer-tree-item.tsx:239
            │   │   │               ├─ span.file-tree-node-label + mark.file-tree-search-highlight  :244-252
            │   │   │               └─ action（悬停动作区）               sidebar-tree.tsx:275-277
            │   │   ├─ contextMenuElement                               file-explorer-tree.tsx:1612
            │   │   └─ 3 个 Dialog（覆盖确认 / 已存在 / 删除确认）        :1613,1632,1667
            │   └─ 加载浮层（Spinner 胶囊）      file-explorer-pane.tsx:54-60
            └─ GlobalSearchBuffer compact（search）  :797-804 → global-search/components/global-search-buffer.tsx:519
                ├─ GlobalSearchToolbar            global-search-buffer.tsx:520 → global-search-toolbar.tsx:46
                ├─ GlobalSearchResults            global-search-buffer.tsx:550 → global-search-results.tsx:35
                │   ├─ FileNavigatorSidebar（**仅 !compact**）  global-search-results.tsx:62 → file-explorer/components/file-navigator-sidebar.tsx:253
                │   └─ SearchExcerptResults       global-search-results.tsx:86
                └─ GlobalSearchState（空/忙/错误）  global-search-buffer.tsx:573-583 → global-search-state.tsx:46

历史类分区（Rail 展开时用，非 files 面板）:
sidebar-history.tsx  SidebarAgentHistory:232 / SidebarPinnedItems:342 / SidebarTerminalHistory:494 / SidebarWorktreeHistory:617
sidebar-projects.tsx SidebarProjectDots:38 → 底部圆点条 :57
```

### 1.5 被复用的基础控件（重写时优先对齐这些）

| 控件 | 文件 | 说明 |
|---|---|---|
| `Tab` / `TabBarTab` / `TabBarSurface` / `SortableTab` / `TabDndContext` | `windows/tauri/src/ui/tab-bar.tsx:220,269,284,61,30` | 标签栏全部样式与拖拽 |
| `Button` | `windows/tauri/src/ui/button.tsx:51` | `size`: default h-8 / xs h-6 / sm h-7 / lg h-9 / icon size-8 / icon-xs size-6 / icon-sm size-7 |
| `InlineRenameInput` | `windows/tauri/src/ui/input.tsx:193` | 标签/文件树内联重命名 |
| `Breadcrumb*` | `windows/tauri/src/ui/breadcrumb.tsx:8-98` | 面包屑 |
| `ContextMenu*` | `windows/tauri/src/ui/context-menu.tsx:6-230` | 右键菜单 |
| `Empty*` | `windows/tauri/src/ui/empty.tsx:8-130` | 空状态 |
| `Sidebar*`（Panel/Header/Toolbar/ListItem/ListEditor/SectionHeader/SectionLabel/TabBar） | `windows/tauri/src/ui/sidebar.tsx:11-456` | 侧栏骨架 |
| `ChromeBar` / `ChromeGroup` / `ChromeLabel` / `ChromeSeparator` | `windows/tauri/src/ui/chrome.tsx:75,92,108,122` | 顶栏/底栏区域 |
| `ScrollArea` | `windows/tauri/src/ui/scroll-area.tsx` | 滚动容器 |
| `Spinner` | `windows/tauri/src/ui/spinner.tsx` | 加载指示 |
| `Dropdown` / `DropdownMenu` | `windows/tauri/src/ui/dropdown.tsx` | 面包屑下拉 / 头部菜单 |

---

## 2. 度量表

### 2.0 全局 token 真源

全部 token 集中在 `windows/tauri/src/styles/theme.css`（默认密度；`[data-window-chrome-density="comfortable"]` 在 `:188-200` 覆盖部分值）。
rem 基数：`html { font-size: calc(16px * var(--app-ui-scale)) }`（`theme.css:220`），`--app-ui-scale: 1`（`theme.css:113`）→ **1 Tailwind 单位 = 4px**。

| token | 值（默认） | comfortable | 来源 |
|---|---|---|---|
| `--app-ui-font-size` / `--ui-text-sm` / `--ui-text-base` | 13px | — | `theme.css:112,116,117` |
| `--ui-text-chrome` | 13px | 14px | `theme.css:115,190` |
| `--ui-text-caption` | 12px | 13px | `theme.css:114,189` |
| `--leading-row` | 1.35 | — | `theme.css:4` |
| `--lithe-title-bar-height` | 2.5rem = 40px | — | `theme.css:118` |
| `--lithe-footer-height` | 1.5rem = 24px | 2rem = 32px | `theme.css:119,191` |
| `--lithe-pane-header-height` | 2.25rem = 36px | — | `theme.css:120` |
| `--lithe-tab-bar-height` | = pane-header-height = 36px | 36px | `theme.css:121` |
| `--lithe-tab-height` | 1.75rem = 28px | 2rem = 32px | `theme.css:122,192` |
| `--lithe-tab-max-width` | 12.5rem = 200px | — | `theme.css:123` |
| `--lithe-sidebar-header-height` | 2rem = 32px | 2.25rem = 36px | `theme.css:124,193` |
| `--lithe-workbench-gap` | 4px | — | `theme.css:125` |
| `--lithe-chrome-control-height` | 1.5rem = 24px | 1.75rem = 28px | `theme.css:126,194` |
| `--lithe-chrome-hit-target` | 1.75rem = 28px | 2rem = 32px | `theme.css:127,195` |
| `--lithe-chrome-line-height` | 1rem = 16px | — | `theme.css:128` |
| `--lithe-chrome-gap-tight / gap / gap-loose` | 2 / 4 / 6px | 4 / 6 / 8px | `theme.css:129-131,196-198` |
| `--lithe-chrome-padding-inline` | 8px | 10px | `theme.css:132,199` |
| `--lithe-chrome-radius` | 4px | — | `theme.css:133` |
| `--radius` | 8px（→ `--radius-md`=6.4px、`--radius-lg`=8px、`--radius-xl`=11.2px、`--radius-2xl`=14.4px） | — | `theme.css:134,6-12` |
| `--app-duration-fast / normal` | 150ms / 200ms | — | `theme.css:135,136` |
| `--app-ease-smooth` | `cubic-bezier(0.22,1,0.36,1)` | — | `theme.css:137` |
| `--app-press-scale` | `1`（**没有按压缩放**） | — | `theme.css:139` |
| `--app-scrollbar-size / thin` | 11px / 9px，圆角 9999px，轨道透明 | — | `styles/scrollbars.css:2-5` |
| `--shadow-popover` | 三层阴影 + 0.5px 描边 | — | `theme.css:160-162` |

> ⚠️ **不存在** `--ui-text-lg` / `--ui-text-xs` 变量与类（`ui-text-*` 只有 4 个定义：`styles/utilities.css:30-44`）。`windows/tauri/src/ui/sidebar.tsx:43` 的 `ui-text-lg` 与 `agent-session-sidebar-item.tsx:170` 的 `ui-text-xs` 是**空操作 class**，字号靠继承。重写时不要照抄。

颜色 token（两个来源，必须一起看）：
1. CSS 变量名 → Tailwind 颜色类的映射：`styles/theme.css:14-49`（如 `--color-background: var(--background)`、`--color-subtle-foreground`、`--color-selected`、`--color-tab-bar: var(--tab-bar-bg)`）。
2. 变量实际取值来自主题定义：`windows/tauri/src/extensions/themes/builtin/lithe.json`（默认 `lithe-dark`，`default-settings.ts:99`）。
   - Lithe Light：background `#ffffff`、surface `#f7f8fa`、foreground `#1f2328`、subtle-foreground `#68717d`、border `#dfe1e5`、accent `#edf3ff`、selected `#d4e2ff`、primary `#3574f0`（`lithe.json:13-19` 对应的 `colors` 块）
   - Lithe Dark：background `#1e1f22`、surface `#2b2d30`、foreground `#dfe1e5`、subtle-foreground `#8b929e`、border `#43454a`、accent `#393b40`、selected `#2e436e`、primary `#3574f0`
   - `--tab-bar-bg: var(--background)`、`--tab-active-bg: var(--background)`、`--tab-hover-bg: color-mix(accent 72%, transparent)`（`theme.css:153-157`）→ **标签栏与编辑区同底色，靠 1px 下划线分隔，不用色带**。

### 2.1 编辑区 · 标签栏

| 元素 | 值 | 来源 |
|---|---|---|
| 标签栏高度 | 36px（`h-(--lithe-tab-bar-height)`） | `windows/tauri/src/ui/tab-bar.tsx:248`、`styles/theme.css:121` |
| 标签栏左右内边距 | 8px | `ui/tab-bar.tsx:248` |
| 标签栏段间距 | 4px（`gap-(--lithe-chrome-gap)`） | `ui/tab-bar.tsx:248` |
| 标签栏下边框 | 1px `--border` | `ui/tab-bar.tsx:248` |
| 标签栏底色 | `--tab-bar-bg` = `--background` | `ui/tab-bar.tsx:248`、`theme.css:155` |
| 标签高度 | 28px | `ui/tab-bar.tsx:260`、`theme.css:122` |
| 标签最小宽 / 最大宽 | 80px（`min-w-20`）/ 200px | `ui/tab-bar.tsx:260`、`theme.css:123` |
| 标签左右 padding | 左 8px（`pl-2`）右 24px（`pr-6`，给关闭按钮留位） | `ui/tab-bar.tsx:260` |
| 标签圆角 | 4px | `ui/tab-bar.tsx:176`、`theme.css:133` |
| 标签内 gap | 6px（`gap-(--lithe-chrome-gap-loose)`） | `ui/tab-bar.tsx:236` |
| 标签字号 | 13px（`ui-text-chrome`） | `ui/tab-bar.tsx:170`、`theme.css:115` |
| 图标槽 | 12×12px（`grid size-3`） | `tabs/components/tab-bar-item.tsx:183` |
| 图标颜色 | `--subtle-foreground` | `tab-bar-item.tsx:185,188,191,201…` |
| 活动标签文字 / 非活动 | `--foreground` / `--subtle-foreground` | `tab-bar-item.tsx:276` |
| 预览态标签文字 | 斜体（`italic`） | `tab-bar-item.tsx:277` |
| 活动标签指示条 | 高 3px、左右内缩 6px、`before:rounded-t-sm`、颜色 `--primary`、贴标签底边 | `ui/tab-bar.tsx:209` |
| 悬停底色 | `--accent/70`（活动标签悬停 `--accent/35`） | `ui/tab-bar.tsx:170,209` |
| 焦点环 | 2px `--primary/25`，offset 1px，offset-color `--tab-bar` | `ui/tab-bar.tsx:170` |
| 过渡 | 150ms `--app-ease-smooth` | `ui/tab-bar.tsx:170`、`theme.css:135,137` |
| 脏标记圆点 | 8×8px、`rounded-full`、`--primary` | `tab-bar-item.tsx:284-291` |
| 关闭/固定按钮 | 24×24px（Button `icon-xs`）、绝对定位 `right:4px` + 垂直居中 | `tab-bar-item.tsx:164-166`、`ui/button.tsx:27` |
| 关闭按钮可见性 | 固定标签恒显；`always` 全显；`active`（默认）仅活动标签显；否则仅悬停显 | `features/settings/lib/ui-preferences.ts:13-18`、`features/settings/config/default-settings.ts:96`、`tab-bar-item.tsx:164-167` |
| 固定标签图标 | Pin 图标 `fill-current` + `--primary` | `tab-bar-item.tsx:173-174` |
| 拖动插入指示条 | 宽 2px（`w-0.5`）、上下内缩 4px、`--primary`、1s 脉冲动画 | `tab-bar-item.tsx:135`、`styles/utilities.css:68-82` |
| 拖拽激活距离 | 5px | `ui/tab-bar.tsx:32-36` |
| 拖拽重排动画 | 180ms | `ui/tab-bar.tsx:74-78` |
| 拖拽中标签透明度 | 0.4 | `ui/tab-bar.tsx:184` |
| 标签栏左右按钮容器 | 高 32px（`h-8`），左组 `gap-0.5`，右组 `gap-1 pl-0.5` | `tab-bar.tsx:633,763` |
| 拖出标签栏判定容差 | 水平 ±24px、垂直 ±64px | `tab-bar.tsx:458-459` |
| 标签自动滚动进视野 | `scrollIntoView({behavior:"smooth", inline:"center"})` | `tab-bar.tsx:330-334` |
| 横向滚轮 | 非 passive wheel 监听，`deltaX || deltaY`，ctrl/meta 不拦截 | `tabs/hooks/use-tab-wheel-scroll.ts:8-25` |

### 2.2 编辑区 · 空态 / 面包屑 / 编辑器 / 状态栏

| 元素 | 值 | 来源 |
|---|---|---|
| 空态容器 | `size-full`、居中、`bg-background`、`px-24px py-32px` | `panes/components/empty-editor-state.tsx:15` |
| 空态内容块 | `max-w-md`=448px、`gap-3`=12px、居中 | `empty-editor-state.tsx:16` |
| 空态图标盒 | 48×48px（`size-12`） | `empty-editor-state.tsx:17` |
| 空态主图标 | 40×40px（`size-10`）、`stroke-[1.15]` | `empty-editor-state.tsx:18` |
| 空态角标放大镜 | 20×20px（`size-5`）、右下角绝对定位、`stroke-[1.4]` | `empty-editor-state.tsx:19-22` |
| 空态标题 | 13px / `font-medium` / `--foreground` | `empty-editor-state.tsx:24`、`theme.css:117` |
| 空态描述 | 13px / `--subtle-foreground` | `empty-editor-state.tsx:27`、`theme.css:116` |
| 空态右键菜单 | 仅一项禁用项「此处无任何内容」 | `empty-editor-state.tsx:32-34` |
| 面包屑栏高度 | 28px（`h-7 min-h-7`） | `editor/components/toolbar/breadcrumb.tsx:220` |
| 面包屑栏边框 / 底色 / padding | 下边框 1px `--border/50`、`--background`、`px-2`=8px | `toolbar/breadcrumb.tsx:220` |
| 面包屑栏左右两组 gap | 8px（`gap-2` / `gap-1`） | `toolbar/breadcrumb.tsx:221,243` |
| 面包屑分段按钮 | 高 24px（Button `xs`）、`gap-1`=4px、`px-1.5`=6px | `toolbar/path-breadcrumb.tsx:54`、`ui/button.tsx:23` |
| 面包屑分段文字颜色 | 末段 `--foreground` + medium；其余 `--subtle-foreground` | `toolbar/path-breadcrumb.tsx:60-62` |
| 面包屑分隔符 | 14px chevron（`[&>svg]:size-3.5`）、左右 `mx-0.5`=2px | `ui/breadcrumb.tsx:75`、`path-breadcrumb.tsx:45` |
| 面包屑文件图标 | 14×14px（`size-3.5`） | `toolbar/file-path-breadcrumb.tsx:199` |
| 面包屑操作按钮 | 24×24px、`rounded`、`--subtle-foreground` | `toolbar/breadcrumb.tsx:37-45` |
| 面包屑右侧竖向分隔线 | 宽 1px、高 14px（`h-3.5 w-px`）、`--border/70` | `toolbar/breadcrumb.tsx:254` |
| 面包屑目录下拉 | 最小宽 200px、最大高 300px、`z-index: 100` | `editor/config/constants.ts:55-57`、`file-path-breadcrumb.tsx:210-215` |
| 面包屑下拉项 | Button `xs`（24px）、`gap-2` | `file-path-breadcrumb.tsx:219-271` |
| 编辑器宿主 padding | 上 8 / 左 16 / 下 8 / 右 16 px | `editor/config/constants.ts:20-23` |
| 行高公式 | `ceil(fontSize × 1.4)`；默认字号 14px → **20px** | `editor/utils/lines.ts:8-15`、`editor/config/constants.ts:4-5`、`features/settings/config/typography-defaults.ts:13`、`features/settings/config/default-settings.ts:54` |
| 等宽字体 | `Geist Mono`（`--editor-font-family`，回退 Consolas 等） | `theme.css:109-111`、`typography-defaults.ts:2` |
| 字号 | 默认 14px（`DEFAULT_CODE_FONT_SIZE`），缩放 = fontSize × zoomLevel | `typography-defaults.ts:13`、`editor/engines/monaco/use-monaco-editor-settings.ts:8,40` |
| 行号槽 | 最小宽 40px、字符宽 8px、右边距 8px、固定 4 位、左侧预留 8px git 指示 | `editor/config/constants.ts:28-33` |
| 编辑器滚动条 | 隐藏（`.editor-container` scrollbar-width:none） | `editor/components/stylesheet.tsx:6-13` |
| z-index 分层 | DECORATION 10 / SELECTION 20 / CURSOR 25 / GIT_BLAME 30 / OVERLAY 40 / DROPDOWN 100 / INLINE_TOOLBAR 200 / TOOLTIP 250 / CONTEXT_MENU 300 | `editor/config/constants.ts:36-49` |
| 查找栏（Monaco find widget 重绘） | padding 左右 6px；圆角 `0 0 8px 8px`；底色 `mix(surface 94%, background)`；字号 13px；阴影 `--shadow-popover`；字体 = UI 字体 | `editor/styles/monaco-editor.css:11-24` |
| 查找栏输入框 | 圆角 6.4px、底色 `--background` | `monaco-editor.css:26-31` |
| 查找栏按钮 hover | 底色 `--accent`、文字 `--foreground` | `monaco-editor.css:38-42` |
| 查找栏聚焦 | `outline: 1px solid --primary; outline-offset: -1px` | `monaco-editor.css:44-48` |
| 搜索命中高亮 | 底色 `mix(primary 26%, transparent)`；当前命中 1px 描边 `mix(primary 72%, white 8%)` | `monaco-editor.css:59-65` |
| 粘性滚动条阴影 | `--shadow-editor-sticky` | `monaco-editor.css:50-57` |
| 大文件提示条 | `border-b`、`bg-muted/40`、`px-3 py-1.5`（12/6px）、`text-xs`=12px | `editor/components/code-editor.tsx:665` |
| 外部冲突横幅 | `min-h-10`=40px、`gap-2`=8px、`px-3`=12px、`text-xs`=12px、图标 16px、按钮高 24px（`xs`） | `editor/components/external-conflict-banner.tsx:39-59` |
| 恢复失败卡片 | `max-w-md`=448px、`gap-3`、`p-6`=24px、标题 medium、错误文字 14px | `code-editor.tsx:737-747` |
| 多文件头（multibuffer） | 行高 28px（`h-7`）、切换按钮 28×28px、chevron 14px、文件名 `max-w-[45%]`、`gap-1.5`=6px | `editor/components/multibuffer/multibuffer-file-header.tsx:63-120` |
| 状态栏高度 | 24px（`--lithe-footer-height`）；底色 `--surface`；`px-8px`；组间 `gap-2`=8px | `ui/chrome.tsx:13`、`footer/footer.tsx:47`、`theme.css:119` |
| 状态栏项容器 | 最小高 24px（`--lithe-chrome-control-height`） | `footer/footer.tsx:53,63,72` |
| 状态栏 chip | 高 24px、圆角 6.4px（`rounded-md`）、`px-1.5`=6px、`ui-text-chrome`=13px、`max-w-50`=200px | `footer/footer-status-chip.tsx:5` |
| 光标位置 chip | 高 20px（`h-5`）、`rounded-full`、`px-1.5`、`ui-text-sm`；编辑态宽 56px（`w-14`） | `editor/components/toolbar/editor-status-actions.tsx:8-9,79` |
| 状态栏项内容 | 光标 `行:列`（+1 基准）、编码 `UTF-8`（`TEXT_FILE_ENCODING`）、缩进 `{n} 个空格`、只读锁图标、内存 `总计 X · Lithe Y`、git 更改数 | `footer-editor-status.tsx:29,81,89,96-103,112-117,122-141` |

### 2.3 编辑区 · 窗格 / 分屏 / 底部面板

| 元素 | 值 | 来源 |
|---|---|---|
| 窗格容器 | `flex size-full flex-col overflow-hidden` + `bg-background` | `panes/components/pane-container.tsx:1076` |
| 活动窗格描边 | `ring-1 ring-primary/30` | `pane-container.tsx:1077` |
| 拖拽悬停描边 | `ring-2 ring-primary` | `pane-container.tsx:1078` |
| 文件拖入高亮层 | `absolute inset-0 z-40 bg-primary/10` | `pane-container.tsx:1087` |
| 分屏分隔条尺寸 | 厚 4px（`--lithe-workbench-gap`）；横分 `w-4px h-full`，纵分 `h-4px w-full` | `panes/components/pane-resize-handle.tsx:147-148`、`theme.css:125` |
| 分隔条视觉线 | 1px，透明；悬停/拖拽时 `--primary` | `pane-resize-handle.tsx:161-163` |
| 分隔条拖拽遮罩 | `fixed inset-0 z-50` + col/row-resize 光标 | `pane-resize-handle.tsx:165-171` |
| 窗格最小比例 | `MIN_PANE_SIZE = 10`（且不低于 pairTotal×10%） | `panes/constants/pane.ts:1`、`pane-resize-handle.tsx:90` |
| 默认分割比例 | `[50, 50]` | `panes/constants/pane.ts:3` |
| 拖放区域命中样式 | `inset-1`=4px、`rounded-lg`=8px、`border-2 --primary`、`bg-primary/14`、150ms 过渡 | `panes/components/split-drop-overlay.tsx:13-19,67-73` |
| 双击分隔条 | 重置为均分 | `pane-resize-handle.tsx:150`、`pane-node-renderer.tsx:71-74` |
| 轮播卡片宽度 | 默认 640px、最小 320px、最大 = 视口宽 − 160px | `pane-container.tsx:134-136,494-500` |
| 轮播卡片外观 | `rounded-2xl`=14.4px、`border`、活动 `border-primary/50`、拖拽中 `opacity-70` | `pane-container.tsx:1121-1128` |
| 轮播容器间距 | `gap-4 px-4 py-4`（16px） | `pane-container.tsx:1106` |
| 轮播卡片缩放手柄 | 宽 8px（`w-2`）、右贴边、`cursor-col-resize`、hover `bg-primary/20` | `pane-container.tsx:1213` |
| 同窗格保留挂载的编辑器 buffer 上限 | 8 | `pane-container.tsx:137` |
| 底部面板默认/最小高度 | 320px / 200px，最大 视口×80% | `layout/components/bottom-pane/bottom-pane.tsx:47,122` |
| 底部面板外观 | `rounded-xl`、`border-t border-l`、`bg-background`；全屏时 `rounded-none border-0` | `bottom-pane.tsx:247-250` |
| 底部面板 resize 手柄 | 厚 4px、`cursor-ns-resize`、内部 1px 线 hover `--primary` | `bottom-pane.tsx:226-239` |
| 工作区缓存上限 | 同时保留 3 个 workspace 的编辑区 | `panes/components/split-view-root.tsx:44,72` |

### 2.4 侧栏

| 元素 | 值 | 来源 |
|---|---|---|
| 侧栏默认宽 | 320px | `features/settings/config/default-settings.ts:138` |
| 侧栏最小宽 | 140px | `layout/components/resizable-pane.tsx:18,57` |
| 侧栏最大宽 | `视口宽 − reservedWidth − 360px`（且 ≥ 50px） | `layout/utils/resizable-pane-layout.ts:1-9` |
| 侧栏绝对下限 | `MIN_RESPONSIVE_PANE_WIDTH = 50` | `resizable-pane-layout.ts:1` |
| 侧栏与编辑区间距 | 4px（`mr-(--lithe-workbench-gap)`） | `resizable-pane.tsx:188-189`、`theme.css:125` |
| 侧栏内岛外观 | `rounded-xl`=11.2px、`border-x` 1px、`bg-background` | `resizable-pane.tsx:203-205` |
| 侧栏 resize 手柄 | 宽 4px、`cursor-col-resize`、hover `bg-primary/8`、内部 1px 线 hover `--primary` | `resizable-pane.tsx:160-177` |
| 活动栏收起宽 | 38px（`COLLAPSED_ACTIVITY_RAIL_WIDTH`） | `layout/components/sidebar/main-sidebar.tsx:97` |
| 活动栏展开宽 | 默认 160px、min 140px、max 320px；设置默认 `activityRailWidth: 180` | `main-sidebar.tsx:98-100`、`default-settings.ts:132` |
| 活动栏水平留白 | 8px（`ACTIVITY_RAIL_HORIZONTAL_GUTTER`） | `main-sidebar.tsx:101` |
| 活动栏分隔手柄 | 宽 4px、内部 1px 线 hover `--primary`、拖拽遮罩 `fixed inset-0 z-40`、`cursor: col-resize` + 禁用选中 | `main-sidebar.tsx:702-708,311-312` |
| 活动栏条目（Rail 覆盖后） | 最小高 24px（`min-h-6 py-1 ui-text-sm`） | `sidebar/sidebar-pane-selector.tsx:332`、`ui/sidebar.tsx:240` |
| 活动栏条目基类 | `min-h-(--lithe-tab-height)`=28px、`gap-6px`、`px-2 py-1`、`rounded-4px`、13px | `ui/sidebar.tsx:240` |
| 活动栏条目状态 | 悬停 `bg-accent/70`+`text-foreground`+`ring-2 primary/20`；选中 `bg-selected`+`text-foreground`；禁用 `opacity-50` | `ui/sidebar.tsx:241-244` |
| 活动栏图标 | 16px（`size-4`） | `sidebar-pane-selector.tsx:107` |
| 活动栏上下组 | `gap-1`=4px；下组额外 `pt-1`=4px | `sidebar-pane-selector.tsx:359,363` |
| 项目圆点条 | 右 4px、下 6px、`px-2` | `sidebar/sidebar-projects.tsx:57` |
| 项目圆点按钮 / 圆点 | 16px 按钮（`rounded-full`）/ 6px 圆点（`size-1.5`） | `sidebar-projects.tsx:69,89` |
| 项目圆点状态 | 活动 scale-100 opacity-100；非活动 scale-75 opacity-25；悬停 scale-100 opacity-50；150ms | `sidebar-projects.tsx:92-95` |
| 侧栏头部（通用） | 高 32px（`--lithe-sidebar-header-height`）、`sticky top-0 z-20`、`gap-4px`、`py-1`=4px | `ui/sidebar.tsx:93`、`theme.css:124` |
| 文件树头部 | 额外 `px-2`=8px、下边框 1px `mix(border 72%)`、`bg-background` | `file-explorer-tree.tsx:1310`、`file-explorer/styles/file-explorer-tree.css:145-149` |
| 文件树头部标题 | 13px（`--ui-text-chrome`）、`font-weight: 600`、`line-height: 16px` | `file-explorer-tree.css:151-156` |
| 头部图标按钮 | 24×24px（Button ghost `icon-xs`、`rounded-md`） | `ui/sidebar.tsx:132-138`、`ui/button.tsx:27` |
| 树行高 | `max(24, uiFontSize × 1.35 + 6)`；`uiFontSize=13` → **24px**（15→26.25，18→30.3） | `file-explorer/lib/file-tree-row.ts:1-14`、`file-explorer/lib/file-tree-row.test.ts:6-12`、`file-explorer/hooks/use-file-tree-presentation.ts:27` |
| 树虚拟滚动 | 上下留白 4px（`FILE_TREE_VIEWPORT_PADDING`）、overscan 8 行；总高 = 行数×行高 + 8 | `file-explorer/lib/file-tree-viewport.ts:1-2,16` |
| 树行水平内缩 | `padding-inline: 6px`（`--file-tree-row-inline-inset`） | `file-explorer-tree.css:6,36` |
| 树行按钮 | `gap-1.5`=6px、`px-1.5`=6px、`py-1`=4px、`leading-row`=1.35；CSS 覆写 `column-gap: 4px !important`、`padding-block: 2px !important` | `features/sidebar/components/sidebar-tree.tsx:241`、`file-explorer-tree.css:56-57` |
| 树行圆角 | 4px（`--file-tree-row-radius`） | `file-explorer-tree.css:7,48` |
| 树行字号 | 13px（`--ui-text-sm`） | `file-explorer-tree.css:100`、`theme.css:116` |
| 树缩进步长 | `paddingLeft = 10 + depth × indentSize`，`FILE_TREE_BASE_INDENT = 10` | `file-explorer-tree-item.tsx:116,156`、`sidebar-tree.tsx:246`、`file-tree-row.ts:1` |
| 缩进步长可选值 / 默认 | 12 / 16 / 20 / 24，默认 **16px**（`fileTreeIndentSize`） | `file-explorer-tree.tsx:1482-1493`、`default-settings.ts:182` |
| `SidebarTreeRow` 组件默认缩进步长 | 14px（`SIDEBAR_TREE_INDENT_SIZE`），基缩进 10px | `sidebar-tree.tsx:7-8` |
| 折叠三角容器 / 三角 | 16×16px（`size-4`、`mr-0.5`）/ 12×12px chevron（`size-3` bold） | `sidebar-tree.tsx:301,314-316`、`file-explorer-tree.css:115-124` |
| 文件图标 | 16×16px（`--file-tree-icon-size: 16px`） | `file-explorer-tree.css:5,126-131` |
| 缩进参考线 | 容器宽 7px、线宽 1px 位于 `left:3px`、`translateX(-3px)`、颜色 `mix(subtle-foreground 20%, transparent)`；默认 `opacity:0`，悬停或 focus 时 `0.9`；端点内缩 4px | `file-explorer-tree.css:9,177-199`、`sidebar-tree.css:6-7,44-59`、`file-explorer-tree-item.tsx:137-139` |
| 选中底色 | 树未聚焦：`--border`（`--file-tree-selected-idle-bg`）；树聚焦（`data-tree-focused="true"`）：`--selected` | `file-explorer-tree.css:8,69-78`、`file-explorer-tree.tsx:1114` |
| 行悬停底色 | `--accent`（`.file-tree-container`）/ `mix(accent 68%, transparent)`（`[data-sidebar-tree-row]`） | `file-explorer-tree.css:4,93-95`、`sidebar-tree.css:5,20-22` |
| 行焦点环 | `inset 0 0 0 1px mix(primary 52%, border)` | `file-explorer-tree.css:80-83` |
| 根行标签 | `font-weight: 600` | `file-explorer-tree.css:133-135` |
| 搜索命中高亮 mark | 圆角 6.4px、底色 `mix(primary 30%)`、`padding: 0 1px` | `file-explorer-tree.css:163-168` |
| 忽略文件 / 剪切态 | `opacity-50` / `italic opacity-40` | `file-explorer-tree-item.tsx:228-229` |
| 拖放目标态 | `border-2! border-dashed! border-primary! bg-primary!` | `file-explorer-tree-item.tsx:226`、`file-explorer-tree.tsx:1111-1112` |
| 拖拽幽灵 | `z-9999 opacity .95 padding 6px 12px border 2px --primary radius 8px --shadow-popover`，跟随指针 +10/−10px | `file-explorer/hooks/use-file-explorer-drag-drop.ts:77-90,106-107` |
| 拖拽阈值 / 自动展开延时 | 5px / 550ms | `file-explorer-tree.tsx:1064`、`use-file-explorer-drag-drop.ts:68` |
| 内联重命名行 | `gap-1.5 px-1.5 py-1 ui-text-sm leading-row`；输入框 `xs`、`inline`、`px-0`、`font-sans 13px` | `file-explorer-tree-item.tsx:153`、`ui/input.tsx:67-87,193-205` |
| 加载浮层 | 顶部居中 `p-3`=12px；胶囊 `px-3 py-1.5`、`rounded-full`、`bg-surface/92`、`shadow-popover`、`backdrop-blur-sm` | `file-explorer-pane.tsx:55-56` |
| 右键菜单面板 | `min-w-36`=144px、圆角 6.4px、`p-1`=4px、13px、`--shadow-popover`、`ring-1 --border/70`、`z-10070` | `ui/context-menu.tsx:45` |
| 右键菜单项 / 分隔线 | 项：`gap-2`=8px、`px-2 py-1`、圆角 2.4px、图标 16px；分隔线 1px、`my-1`=4px | `ui/context-menu.tsx:89,196` |
| 搜索工具栏 | `py-2`=8px、compact `px-2`=8px、下边框 1px、`bg-surface/55` | `global-search-toolbar.tsx:93` |
| 搜索输入框 | 高 28px（`h-7`）、`rounded-lg`=8px、`px-2`=8px、`gap-2`、图标 16px | `global-search-toolbar.tsx:101-102` |
| 搜索结果区 | `relative min-h-0 flex-1 bg-background`；滚动 `px-2 pb-2` | `global-search-buffer.tsx:548`、`global-search-results.tsx:78-83` |
| 文件导航器（**仅非 compact 搜索结果里出现**） | 默认宽 224px、min 176px、max 420px 且 ≤ 父宽 50% | `file-explorer/lib/file-navigator-layout.ts:1-5` |
| 导航器分隔条 / 键盘步长 | 热区 8px、hover `bg-primary/20`；左右方向键步长 16px | `file-navigator-sidebar.tsx:464-465`、`:45` |
| 历史分区容器 | `mt-3`=12px；分区头高 28px（`h-(--lithe-tab-height)`）/ `px-2` / `gap-1`；三角 12px，折叠时 `-rotate-90` | `sidebar-history.tsx:287,415,553,669`、`ui/sidebar.tsx:327-341` |
| Agent 历史行 | 最小高 24px、`rounded-md`=6.4px、`py-1 pl-2 pr-12`、`gap-2`、13px | `agent-session-sidebar-item.tsx:69,81` |
| 空状态（通用 `Empty`） | `gap-2`=8px、`rounded-lg`=8px、`p-3`=12px、`border-dashed` | `ui/empty.tsx:17-22` |

---

## 3. 状态与交互

### 3.1 标签栏

| 行为 | 细节 | 来源 |
|---|---|---|
| 单击标签 | `handleTabSelect` → `handleTabClick` → `setActiveBuffer`；同时 `updateActivePath(buffer.path)` 同步侧栏高亮 | `tabs/components/tab-bar.tsx:402-419`、`editor/stores/buffer.store.ts:1691-1693` |
| 双击标签 | **仅**把预览标签转正（`convertPreviewToDefinite`），不做最大化/关闭 | `tab-bar.tsx:340-351`、`buffer.store.ts:1731-1740` |
| 中键单击 | 关闭该标签（`e.button === 1`） | `tabs/components/tab-bar-item.tsx:122-130` |
| 关闭按钮单击 | 未固定 → 关闭；已固定 → 取消固定 | `tab-bar-item.tsx:156-163` |
| 关闭按钮可见性 | 三档：`always` / `active`（默认，仅活动标签）/ `hover`（仅悬停）+ 固定标签恒显 | `features/settings/lib/ui-preferences.ts:13-18`、`default-settings.ts:96` |
| 预览态 | 标签文字斜体；单击文件树（非预览外）会替换同窗格上一个预览标签；pin 或编辑内容会退出预览态 | `tab-bar-item.tsx:277`、`buffer.store.ts:645-657,1706-1708,1570-1571` |
| 脏标记 | 仅 `type==="editor" && isDirty` 显示 8px `--primary` 圆点，title/aria = 「未保存的更改」 | `tab-bar-item.tsx:284-291` |
| 固定标签 | 排序上置（pinned 在 unpinned 前）；`isPinned` 时关闭按钮变为 Pin（点击取消固定） | `tab-bar.tsx:242-257`、`tab-bar-item.tsx:158-177` |
| 拖拽重排 | 同标签栏内水平重排（`restrictToHorizontalAxis` + `horizontalListSortingStrategy`）；拖出标签栏 ±24/±64px 落入其它窗格或新建拆分 | `tab-bar.tsx:619-625,514-556` |
| 拖拽点击抑制 | 拖拽开始后抑制下一次 click，避免拖完误激活 | `ui/tab-bar.tsx:110-152`、`tab-bar.tsx:147,672` |
| 横向滚动 | 设置项 `horizontalTabScroll`（默认 true）开启滚轮横向滚动；活动标签越界时 `scrollIntoView` 居中 | `tab-bar.tsx:106,240,317-338`、`default-settings.ts:179` |
| 最大标签数 | `maxOpenTabs` 默认 100；超限自动关闭最旧的非固定、非活动标签 | `tab-bar.tsx:305-315`、`default-settings.ts:178` |
| 键盘 | 左右方向键切换（focus 跟随）、`Enter`/`Space` 激活、`Delete`/`Backspace` 关闭（固定标签则播报拒绝） | `tab-bar.tsx:562-615`、`features/layout/utils/chrome-keyboard.ts` |
| 无障碍 | `role="tablist"`/`role="tab"`、`aria-selected`、`aria-label` 拼「名称+（未保存）+（已固定）+（预览）」、`role="status" aria-live="polite"` 播报 | `tab-bar.tsx:630-631,821-823`、`tab-bar-item.tsx:137-141` |
| 终端标签重命名 | 仅 terminal buffer：右键「重命名」→ 内联输入（rAF 后聚焦全选），提交写回 terminal session `customName` | `tab-bar.tsx:378-433`、`tab-context-menu.tsx:77-86` |
| 右键菜单项（顺序） | 固定/取消固定 → （终端）重命名 → — → 向右拆分 → 向下拆分 → — → 锁定/解锁编辑器组 → — → 复制路径 → 复制相对路径 → 在资源管理器中显示 → 在终端中打开 → 重新加载 → — → 关闭 → 关闭其他 → 关闭右侧 → 全部关闭 | `tabs/components/tab-context-menu.tsx:70-202` |
| 重载标签 | 关闭后 100ms 重新打开（保留内容），`extensions://marketplace` 排除 | `tab-bar.tsx:715-739` |
| 拖到其它区域 | 拖到底部面板会 `setBottomPaneActiveTab("buffers")` + 显示底部面板 | `tab-bar.tsx:541-544` |

### 3.2 编辑区 / 窗格

| 行为 | 细节 | 来源 |
|---|---|---|
| 空态 | 无 `activeBuffer` 且非轮播模式时渲染 `EmptyEditorState`（图标 + 标题 + 描述 + 右键「此处无任何内容」） | `pane-container.tsx:1100`、`empty-editor-state.tsx:13-35` |
| 加载态 | 恢复中显示 `Spinner`（带「正在加载」文字）；恢复失败显示错误文案 + 「重试」按钮 | `code-editor.tsx:732-747` |
| 外部冲突 | `documentLifecycle.status === "conflict"` 显示警告横幅，两个按钮：保留编辑器内容 / 加载磁盘版本（文件被删时禁用后者并改文案） | `external-conflict-banner.tsx:16-61` |
| 大文件降级 | 超阈值时关闭语言智能/Git Blame/CodeLens/语义高亮，横幅提供「仍然启用」 | `code-editor.tsx:664-675` |
| 编辑器只读 | footer 显示锁图标 + 「只读」/「可写」 | `footer-editor-status.tsx:93-106` |
| 跳转行:列 | 点状态栏光标 chip 变内联输入（宽 56px，预填 `行:列`），`Enter` 提交派发 `menu-go-to-line`，`Escape` 取消，失焦提交 | `editor-status-actions.tsx:41-83` |
| 窗格激活 | 点击窗格（排除 button/input/textarea/role=button/menu 目标）或 mousedown 捕获激活，并同步全局 activeBuffer | `pane-container.tsx:404-424,376-383` |
| 分隔条拖拽 | mousedown 后 rAF 节流写 flexGrow 预览，mouseup 一次性提交（性能边界）；双击重置均分 | `pane-resize-handle.tsx:56-140,150` |
| 分屏拖放 | 拖拽标签/终端到窗格四边或中心 → `left/right/top/bottom/center` 五区；中心=并入，其余=新建拆分 | `split-drop-overlay.tsx:13-19`、`pane-container.tsx:688-779` |
| 窗格全屏 | 标签栏右侧 Maximize/Minimize 按钮切换；全屏时该窗格覆盖整个编辑区 | `tab-bar.tsx:798-815`、`split-view-root.tsx:35-39` |
| 窗格锁定 | 右键菜单项切换 `locked`，锁定后不允许关闭标签（`isPaneLocked` 传入菜单） | `tab-bar.tsx:132,235-238`、`pane.store.ts:458-464` |
| 底部面板 | 拖拽标签到面板、resize 手柄调高、全屏切换；内容类型：terminal/debugger/run/maven/diagnostics/buffers/gitLog；终端常驻（hidden 切换）保 PTY | `bottom-pane.tsx:243-310,156-220,222-241` |
| 编辑区缓存 | 最多缓存 3 个 workspace 的编辑区；非活动 workspace `invisible pointer-events-none` + `aria-hidden` | `split-view-root.tsx:44,63-74,105-113` |
| 编辑器滚动条 | 隐藏 | `stylesheet.tsx:6-13` |
| 查找/替换 | 复用 Monaco 内置 find widget，`workbench.showFind` / `workbench.showFindReplace` 命令触发 | `editor/components/monaco-editor.tsx:2411`、`features/keymaps/commands/command-registry.ts:631,638` |
| 面包屑末段单击 | 支持 LSP 的文件 → 打开「大纲」命令面板；否则导航到该路径 | `file-path-breadcrumb.tsx:131-149` |
| 面包屑中间段单击 | 打开目录下拉（可上下钻取，带「返回」项）；点空白/再次点击关闭 | `file-path-breadcrumb.tsx:151-178,205-274` |
| 面包屑下拉定位 | `x = 按钮 left`，`y = 按钮 bottom + 2px`（`menuSide="top"` 时为 top） | `file-path-breadcrumb.tsx:166-173` |
| 面包屑可见性 | 受设置 `coreFeatures.breadcrumbs`（默认 true）与 `breadcrumbShowSymbols`（默认 true）控制 | `toolbar/breadcrumb.tsx:75`、`default-settings.ts:170,73` |
| 面包屑操作菜单 | MoreHorizontal → 预览（可预览时）/ AI 行内编辑 / 在文件中查找 | `toolbar/breadcrumb.tsx:170-214` |

### 3.3 侧栏

| 行为 | 细节 | 来源 |
|---|---|---|
| 选中态来源 | `highlightedPath = hasTreeFocus ? (focusedPath ?? activePath) : activePath`；`data-tree-focused` 驱动两套底色 | `file-explorer-tree.tsx:523-524,1590,1114` |
| 展开/折叠 | 目录单击 `onFileSelect(path, true)` → `toggleFolder`；状态在 `useFileTreeStore.expandedPaths`；根目录只自动展开一次 | `file-explorer-tree.tsx:976-982`、`file-explorer-tree.store.ts:8-9,50-59`、`file-explorer-tree.tsx:235-238` |
| 紧凑文件夹链 | 单子目录链合并显示，用 `.` 连接包名 | `file-explorer/lib/visible-file-tree-rows.ts:168-184,194` |
| 单击 / 双击文件 | 单击 `onFileSelect(t.path, false)`（**不是预览态**：`handleFileSelect` 的 `isPreview` 形参默认 `false`）；双击 `onFileOpen` → `handleFileOpen` 注释明写 "definite mode (not preview)" 并显式传 `false` | `file-explorer-tree.tsx:1007-1010`、`file-system/stores/file-system.store.ts:1533-1539`、`:1965-1967` |
| **脏标记** | **侧栏未找到**。唯一脏标记在标签栏（`tabs/components/tab-bar-item.tsx:284`） | — |
| **预览斜体** | **侧栏未找到**；`features/file-explorer` 唯一的 italic 是剪切态 `isCut && "italic opacity-40"`（`file-explorer-tree-item.tsx:229`）。原因：文件树从不以 preview 打开 | — |
| 内联新建 | 插入 `{name:"", path:`${parent}/`, isEditing:true}` 并强制展开父目录 | `file-explorer-tree.tsx:626-667` |
| 内联重命名 | F2 或右键「重命名」；输入框 rAF 后聚焦全选，`Enter` 提交 / `Escape` 取消 / 失焦提交 / 空值视为取消 | `file-explorer-tree.tsx:1288-1293`、`use-file-explorer-context-menu.tsx:468-471`、`ui/input.tsx:209-249` |
| 同名校验 | 提交后查重，冲突弹 Dialog（文件已存在 / 文件夹已存在） | `file-explorer-tree.tsx:669-723` |
| 拖拽 | 阈值 5px 起拖，跟随幽灵，悬停目录 550ms 自动展开，落盘 `moveFile` → `invoke("move_file")`；失败 toast；拖到编辑器派发 `file-tree-drop-on-pane`；拖到 AI 面板派发 sidebar resource drop | `file-explorer-tree.tsx:1064`、`use-file-explorer-drag-drop.ts:68,77-90,216-226,253,258,202-207`、`file-system/controllers/platform.ts:215` |
| 右键菜单（目录/文件） | 在 `use-file-explorer-context-menu.tsx:192-527` 组装；**「在文件夹中查找」的 onClick 是空实现** | `use-file-explorer-context-menu.tsx:266-271` |
| 活动栏右键 | 「操作」标题 + 打开项目 / 搜索 + 分隔 + 「在活动侧栏中显示」子菜单（9 项可见性 checkbox、项目圆点）+ 条件「Show All」（**硬编码英文**） | `main-sidebar.tsx:710-762` |
| 空白区单击 / 双击 | 清焦点 / 无操作；头部区域阻断事件冒泡 | `file-explorer-tree.tsx:986-992`、`file-explorer-tree.tsx:1311-1312` |
| 空 / 加载 | 未打开文件夹 → EmptyState + 「打开文件夹」按钮；树内搜索无命中 / 文件夹为空有专属文案；加载中顶部 Spinner 胶囊；Rail 切项目显示 `Opening {project}`（**硬编码英文**） | `file-explorer-tree.tsx:1532-1547`、`file-explorer-pane.tsx:54-60`、`main-sidebar.tsx:624-628` |
| 树键盘（容器级） | `Mod+F` 搜索、`/` 搜索、`Mod+C/X/V` 复制剪切粘贴、`Escape` 清焦点/关搜索、`↑↓`、`Home/End`、`→` 展开或下钻、`←` 折叠或上跳、`Enter` 打开、`F2` 重命名 | `file-explorer-tree.tsx:1124-1295` |
| 树键盘（通用 `SidebarTree`） | `↑↓ / Home / End / → / ←` 同语义，用于非文件树侧栏 | `features/sidebar/components/sidebar-tree.tsx:79-145` |
| 活动栏键盘 | 无（分隔条无键盘支持，**未找到**）；条目本身是 button，可 Tab/Enter | `main-sidebar.tsx:697-707` |
| 搜索框键盘 | `Escape` 关闭并保留查询？→ 关闭搜索；`Enter` 下一个命中、`Shift+Enter` 上一个 | `file-explorer-tree.tsx:1329-1342` |
| 树内搜索实现 | debounce 80ms → fff 搜索（limit 500）→ 过滤；搜索中保留上次结果 | `file-explorer-tree.tsx:148,187,434-461,472-488` |
| 搜索侧栏 compact 差异 | **不渲染文件导航器**；工具栏换行；Badge 顺序调整 | `global-search-results.tsx:61-62`、`global-search-toolbar.tsx:94`、`:150,159` |
| 项目圆点交互 | 单击切换项目（仅非当前）；`Enter`/`Space` 同；右键菜单（在新窗口打开 / 选择图标 / 设置别名 / 移除 / 切换到 X）；`titleProject.showSwitchProject` 时显示「切换到项目」菜单 | `sidebar-projects.tsx:80-99`、`main-sidebar.tsx:723-762` |
| 面板切换 | 「文件/搜索/更改」由 `coreFeatures` + `activeSidebarView` 决定；`isGitViewActive` 优先于 `activeSidebarView` | `main-sidebar.tsx:776-812` |
| 快捷键（tooltip 声明） | Files `Mod+Shift+E`、Search `Mod+Shift+F`、Changes `Mod+Shift+G`、GitLog `Alt+9`、Terminal `Mod+J`、Diagnostics `Mod+Shift+J`、Run `Shift+F10` | `sidebar-pane-selector.tsx:130,145,162,179,199,216,233` |

---

## 4. 数据与命令

### 4.1 数据来源（前端 store 为主）

| 数据 | 来源 | 证据 |
|---|---|---|
| 标签列表 / 激活 / pin / preview / 脏标记 | `useBufferStore`（zustand，全局单一 store） | `windows/tauri/src/features/editor/stores/buffer.store.ts:620-700,1691-1740` |
| 标签显示名（路径缩短） | `calculateDisplayNames(buffers, rootFolderPath)` | `windows/tauri/src/features/tabs/utils/path-shortener.ts:31-113`、`tabs/components/tab-bar.tsx:271-273` |
| 终端标签名 | terminal session 的 `customName`/`title`/`initialCommand`/`currentDirectory` 逐级回退 | `tab-bar.tsx:275-303,151-179` |
| 单例工具标签名（搜索/诊断/引用/扩展） | `SINGLETON_TOOL_BUFFER_METADATA` → i18n key | `panes/constants/tool-buffers.ts:9-41` |
| 窗格树（split/group/sizes/locked/preview） | `usePaneStore`（root + bottomRoot） | `panes/stores/pane.store.ts:43-76,244-600`、`panes/types/pane.types.ts:1-23` |
| 当前文件路径 / 侧栏高亮路径 | `useFileSystemStore.rootFolderPath`、`useSidebarStore.activePath` | `panes/components/pane-container.tsx:324`、`tabs/components/tab-bar.tsx:108-109`、`layout/stores/sidebar.store.ts` |
| 文件树数据 | `useFileSystemStore.files / rootFolderPath / isFileTreeLoading / isSwitchingProject` | `file-explorer/components/file-explorer-pane.tsx:24-27` |
| 树展开状态 | `useFileTreeStore.expandedPaths` | `file-explorer/stores/file-explorer-tree.store.ts:8-9,50-59` |
| 树 git 装饰 | `useGitStore.workspaceGitStatus / currentWorkspaceRepoPath` | `file-explorer-tree.tsx:196-197,362-399` |
| 编辑器设置（字号/行高/换行/行号） | `useEditorSettingsStore` | `editor/stores/settings.store.ts:17-53`、`editor/engines/monaco/use-monaco-editor-settings.ts:8-48` |
| 光标位置 / 滚动 / 视图状态 | `useEditorStateStore` | `editor/stores/state.store.ts`、`editor-status-actions.tsx:13-14` |
| 工作区会话（标签恢复） | zustand persist，key `lithe-tab-sessions`，version 1，storage = `createSafeJSONStorage` | `window/stores/session.store.ts:256-266`、`editor/stores/buffer-session-persistence.ts:58-90` |
| Rail 可见项 / 顺序 | `settings.coreFeatures` + `sidebarActivityItemsOrder` + `hiddenSidebarActivityItems` | `main-sidebar.tsx:164,216`、`layout/config/item-order.ts:41-51`、`default-settings.ts:107-108` |
| 项目圆点 | `useWorkspaceTabsStore.projectTabs` | `main-sidebar.tsx:165-183`、`sidebar-projects.tsx:50` |
| 侧栏宽度 | `settings.sidebarWidth`（320） | `resizable-pane.tsx:42-44`、`default-settings.ts:138` |
| 全局搜索结果 | `useContentSearch`（fff/Core 路径 + 前端 provider 路径） | `global-search/hooks/use-content-search.ts:8-17,119-172` |

### 4.2 Tauri command（编辑器 / 侧栏用到）

| command 字符串 | 前端调用点 | Rust 定义 | 注册 |
|---|---|---|---|
| `read_local_file` | `features/file-system/controllers/platform.ts:48` | `windows/tauri/src-tauri/src/host.rs:637`（`#[tauri::command]` 于 `:636`） | `windows/tauri/src-tauri/src/main.rs:161` |
| `read_local_file_bounded` | `features/file-system/controllers/bounded-file-read.ts:18` | `host.rs:668`(`:667`) | `main.rs:162` |
| `write_file` | `features/file-system/controllers/platform.ts`（写盘路径） | `host.rs` | `main.rs:164` |
| `move_file` | `controllers/platform.ts:215`（树拖拽 `use-file-explorer-drag-drop.ts:253`） | `host.rs:728`(`:727`) | `main.rs:166` |
| `rename_file` | `controllers/platform.ts:241` | `host.rs:733`(`:732`) | `main.rs:167` |
| `get_symlink_info` | `controllers/platform.ts:264` | `host.rs:738`(`:737`) | `main.rs:168` |
| `read_document_file` | `windows/tauri/src/platform/document-files.ts:8` | `windows/tauri/src-tauri/src/document.rs:53` | `main.rs:113` |
| `save_document_file` | `platform/document-files.ts:12` | `document.rs:61` | `main.rs:114` |
| `set_document_watches` | `features/editor/services/document-watch-controller.ts:143` | `document.rs` | `main.rs:115` |
| `open_file_external` | `editor/components/toolbar/breadcrumb.tsx:109`、`html/html-preview.tsx:48` | `host.rs` | `main.rs:169` |
| `clipboard_set` / `clipboard_get` / `clipboard_paste` / `clipboard_clear` | `file-explorer/stores/file-explorer-clipboard.store.ts:36,48,44,53` | `host.rs:120,147,171,158`（attr 行见各 `+1`） | `main.rs:171,172,173,174` |
| `wsl_read_file` / `wsl_write_file` / `wsl_read_directory` / `wsl_create_directory` / `wsl_delete_path` / `wsl_rename_path` / `wsl_get_symlink_info` | `controllers/platform.ts:41,66,166,90,107,207,258`；`file-system.store.ts:269` | `src-tauri` 内 WSL 模块 | `main.rs`（WSL 命令组） |
| `ssh_read_file` / `ssh_read_directory` / `ssh_rename_path` / … | `file-system.store.ts:1877,1386,2371`；`workspace-session-restore.ts:52` | `src-tauri` 内 SSH 模块 | `main.rs` |
| `revealItemInDir`（非 command） | `file-system.store.ts:2698,2702` | — | `@tauri-apps/plugin-opener`（导入于 `file-system.store.ts:5`） |
| `readDir`（非 command） | `controllers/platform.ts:177` | — | `@tauri-apps/plugin-fs`（导入于 `platform.ts:14`） |
| `handleRevealInFolder` 实现位置 | `file-system/stores/file-system.store.ts:2689` | — | — |

> 说明：**本地目录列举不走自定义 command**，直接用 `@tauri-apps/plugin-fs` 的 `readDir`（`platform.ts:177`）；只有 WSL/SSH 路径才走 command（`platform.ts:166`、`file-system.store.ts:1386`）。

### 4.3 Rust Core 命令（经 `executeCore` 桥，不是 `#[tauri::command]`）

| Core command | 前端调用点 | 契约 |
|---|---|---|
| `workspace.search` | `windows/tauri/src/features/file-search/lib/file-search-api.ts:93-107` | `shared/contracts/rust-core-api.md:53,95,921` |
| `workspace.snapshot` | `file-search-api.ts:201-205` | `shared/contracts/rust-core-api.md:93` |
| `git.worktrees` | `windows/tauri/src/features/git/api/git-worktrees-api.ts:28` | `shared/contracts/rust-core-api.md` |

Core 桥接入口：`windows/tauri/src/core/lithe-core-client.ts`（`executeCore`），Tauri 侧 `core::core_execute` / `core::core_cancel` 注册于 `windows/tauri/src-tauri/src/main.rs:116-117`。

> **编辑区/侧栏的标签与窗格数据不来自 Core**（见 §0 第 3 条）。重写时不要为标签/窗格造 Core 命令。

---

## 5. 文案（`windows/tauri/src/i18n/locale.ts`，中文照抄）

### 5.1 标签栏

| key | 中文 | 行号 |
|---|---|---|
| `tabs.openFiles` | 打开的文件 | `locale.ts:7849` |
| `tabs.goBack` / `tabs.goBackShort` | 后退到上一个位置 / 后退 | `:7850,7854` |
| `tabs.goForward` / `tabs.goForwardShort` | 前进到下一个位置 / 前进 | `:7851,7855` |
| `tabs.newTab` | 新建标签页 | `:7852` |
| `tabs.toggleEditorFullScreen` | 切换编辑器全屏 | `:7853` |
| `tabs.exitFullScreen` / `tabs.fullScreenEditor` | 退出全屏 / 编辑器全屏 | `:7858,7859` |
| `tabs.closeSplit` / `tabs.closeSplitPane` | 关闭拆分 / 关闭拆分窗格 | `:7856,7857` |
| `tabs.pin` / `tabs.unpin` | 固定标签页 / 取消固定标签页 | `:7838,7839` |
| `tabs.close` | 关闭 | `:7845` |
| `tabs.closeOthers` | 关闭其他 | `:7846` |
| `tabs.closeToRight` | 关闭右侧 | `:7847` |
| `tabs.closeAll` | 全部关闭 | `:7848` |
| `tabs.splitRight` / `tabs.splitDown` | 向右拆分 / 向下拆分 | `:7840,7841` |
| `tabs.lockEditorGroup` / `tabs.unlockEditorGroup` | 锁定编辑器组 / 解锁编辑器组 | `:7842,7843` |
| `tabs.reload` | 重新加载 | `:7844` |
| `tabs.rename` | 重命名 {name} | `:7863` |
| `tabs.unsavedChanges` | 未保存的更改 | `:7864` |
| `tabs.ariaUnsavedSuffix` / `ariaPinnedSuffix` / `ariaPreviewSuffix` | （未保存）/（已固定）/（预览） | `:7860,7861,7862` |
| `tabs.announcementUnsaved` | ，未保存 | `:7865` |
| `tabs.switchedTo` | 已切换到 {name}{unsaved} | `:7866` |
| `tabs.activated` | 已激活 {name}{unsaved} | `:7867` |
| `tabs.closed` | 已关闭 {name} | `:7868` |
| `tabs.cannotClosePinned` | 无法关闭已固定标签页 {name} | `:7869` |
| `terminal.tabNamePlaceholder` | 终端名称 | `:7517` |

### 5.2 编辑区

| key | 中文 | 行号 |
|---|---|---|
| `workbench.emptyEditorTitle` | 选择文件以查看 | `locale.ts:6150` |
| `workbench.emptyEditorDescription` | 外部工具产生的更改会自动显示。 | `:6151` |
| `ui.noActionsHere` | 此处无任何内容 | `:4630` |
| `ui.loading` / `ui.retry` | 正在加载 / 重试 | `:4647,4628` |
| `editor.restoreLoadFailed` | 无法加载此文件。 | `:4641` |
| `editor.externalConflict` | 该文件已在 Lithe 外部更改，同时编辑器中还有未保存的修改。 | `:4639` |
| `editor.externalDeleted` | 该文件已被外部删除，编辑器内容已保留。 | `:4643` |
| `editor.recreateFile` / `editor.keepEditorVersion` / `editor.loadDiskVersion` | 允许重新创建文件 / 保留编辑器内容 / 加载磁盘版本 | `:4644,4645,4646` |
| `editor.largeFileServicesDisabled` | 此大文件已关闭语言智能、Git Blame、代码镜头和语义高亮。 | `:8408` |
| `editor.enableLargeFileServices` | 仍然启用 | `:8409` |
| `editor.actions` | 编辑器操作 | `:8404` |
| `editor.preview` | 预览 | `:8133` |
| `editor.aiInlineEdit` | AI 行内编辑 | `:8398` |
| `editor.findInFile` | 在文件中查找 | `:8400` |
| `editor.openHtmlInBrowser` / `editor.openHtmlInBrowserFailed` | 在浏览器中打开 HTML / 无法在浏览器中打开 HTML | `:8134,8135` |
| `editor.unknownFile` | 未知 | `:8403` |
| `editor.goToLineColumn` | 转到行和列 | `:8401` |
| `editor.symbolPath` | 符号路径 | `:8402` |
| `panes.resizePanes` | 调整窗格大小 | `:4699` |
| `panes.resizeBufferCarouselCards` | 调整缓冲区轮播卡片大小 | `:4681` |
| `panes.noPreviewAvailable` | 没有可用预览 | `:4685` |
| `panes.terminalSession` | 终端会话 | `:4672` |
| `panes.webView` | Web 视图 | `:4673` |
| `panes.diffPreview` | 差异预览 | `:4677` |
| `panes.imagePreview` / `panes.pdfPreview` / `panes.binaryFilePreview` | 图片预览 / PDF 预览 / 二进制文件预览 | `:4678,4679,4680` |
| `panes.databaseViewer` | {type} 查看器 | `:4682` |
| `panes.externalEditorSession` | 外部编辑器会话 | `:4683` |
| `panes.missingDatabaseConnection` | 缺少数据库连接 | `:4698` |
| `panes.webViewerDisabled` / `panes.webViewerDisabledDescription` | Web 查看器已禁用 / 在「设置 > 功能」中启用后，可以在嵌入式编辑器标签页中打开 URL。 | `:4695,4696` |
| `panes.pullRequest` / `pullRequestNumber` / `filesCount` / `commitsCount` / `commentsCount` | 拉取请求 / 拉取请求 #{number} / {count} 个文件 / {count} 个提交 / {count} 条评论 | `:4686,4674,4687,4688,4689` |
| `panes.description` / `files` / `comments` | 描述 / 文件 / 评论 | `:4690,4691,4692` |
| `panes.pullRequestPreviewFallback` | 激活此卡片可查看完整的拉取请求描述、变更文件、评论、审查状态和检出操作。 | `:4693-4694` |
| `panes.issueNumber` / `panes.workflowRunNumber` | 议题 #{number} / 工作流运行 #{number} | `:4675,4676` |
| `footer.statusBar` | 状态栏 | `:7316` |
| `footer.cursor` | 跳转到行和列 | `:7305` |
| `footer.encoding` | 文件编码 | `:7306` |
| `footer.indent` | 缩进 | `:7307` |
| `footer.spaces` | {count} 个空格 | `:7308` |
| `footer.readOnly` / `footer.writable` | 只读 / 可写 | `:7309,7310` |
| `footer.memory` / `footer.memoryUsage` | 内存 / 总计 {total} · Lithe {used} | `:7311,7312` |
| `footer.changes` / `footer.change` / `footer.noChanges` | {count} 个更改 / {count} 个更改 / 没有更改 | `:7313,7314,7315` |
| `footer.filePath` | 文件路径 | `:7317` |
| `layout.resizePanes`… 见 panes 行 | | |
| `layout.resizeSidebar` / `layout.resizeBottomPane` / `layout.resizeAiChat` | 调整侧边栏大小 / 调整底部面板大小 / 调整 AI 聊天大小 | `:7887,7888,7886` |
| `layout.sidebarSections` | 侧边栏分区 | `:7889` |
| `keybindings.commands.workbench.showFind.title` / `showFindReplace.title` | 查找 / 查找和替换 | `:8334,8335` |

### 5.3 侧栏 · 活动栏与布局

| key | 中文 | 行号 |
|---|---|---|
| `workbench.project` | 项目 | `locale.ts:4605` |
| `workbench.changes` | 更改 | `:4606` |
| `workbench.search` | 搜索 | `:4607` |
| `workbench.diagnostics` | 诊断 | `:5916` |
| `workbench.terminal` | 终端 | `:5915` |
| `workbench.gitLog` | 提交记录 | `:5910` |
| `workbench.settings` | 设置 | `:5912` |
| `workbench.run` | 运行 | `:5913` |
| `workbench.activityViews` | 活动视图 | `:7890` |
| `layout.actions` | 操作 | `:4700` |
| `layout.resizeActivityRail` | 调整活动栏大小 | `:4701` |
| `layout.showInActivitySidebar` | 在活动侧栏中显示 | `:4702` |
| `layout.projectDots` | 项目圆点 | `:4703` |
| `run.title` / `maven.title` | 运行 / Maven | `:5927,6043` |

### 5.4 侧栏 · 文件树

| key | 中文 | 行号 |
|---|---|---|
| `fileExplorer.preferences` | 文件资源管理器偏好设置 | `locale.ts:7445` |
| `fileExplorer.searchFiles` | 搜索文件 | `:7446` |
| `fileExplorer.visibility` | 可见性 | `:7448` |
| `fileExplorer.gitignoredFiles` | 被 Git 忽略的文件 | `:7449` |
| `fileExplorer.gitStatusDecorations` | Git 状态标记 | `:7450` |
| `fileExplorer.fileIcons` | 文件图标 | `:7451` |
| `fileExplorer.indentGuides` | 缩进参考线 | `:7452` |
| `fileExplorer.indentation` / `indentationCompact` / `indentationDefault` / `indentationSpacious` / `indentationWide` | 缩进 / 紧凑 / 默认 / 宽松 / 宽 | `:7453-7457` |
| `fileExplorer.noFolderOpen` | 未打开文件夹 | `:7458` |
| `fileExplorer.searchingFiles` | 正在搜索文件 | `:7459` |
| `fileExplorer.noMatchingFiles` | 没有匹配的文件 | `:7460` |
| `fileExplorer.folderIsEmpty` | 文件夹为空 | `:7461` |
| `fileExplorer.moveFailed` | 移动失败 | `:7462` |
| `fileExplorer.opening` | 正在打开... | `:7473` |
| `fileExplorer.folderAlreadyExists` / `folderAlreadyExistsMessage` | 文件夹已存在 / 已存在同名文件夹。 | `:7468,7469` |
| `fileExplorer.fileAlreadyExists` / `fileAlreadyExistsMessage` | 文件已存在 / 已存在同名文件。 | `:7470,7471` |
| `fileExplorer.deleteFolder` / `deleteFile` / `deleting` | 删除文件夹 / 删除文件 / 正在删除... | `:7476,7477,7478` |
| `fileExplorer.deleteFolderMessage` | 确定要删除文件夹"{name}"及其所有内容吗？此操作无法撤销。 | `:7479` |
| `fileExplorer.deleteFileMessage` | 确定要删除文件"{name}"吗？此操作无法撤销。 | `:7480` |
| `files.openInTerminal` | 在终端中打开 | `:7426` |
| `files.rename` | 重命名 | `:7399` |
| `files.reveal` | 在资源管理器中显示 | `:7400` |
| `files.copyPath` / `files.copyRelativePath` | 复制路径 / 复制相对路径 | `:7388,7392` |
| `files.newFile` / `files.newFolder` | 新建文件 / 新建文件夹 | `:7403,7404` |
| `files.fileNamePlaceholder` / `files.folderNamePlaceholder` | 文件名 / 文件夹名 | `:7410,7411` |
| `files.renameItem` | 重命名 {name} | `:7412` |
| `files.symlinkTo` | 符号链接到：{target} | `:7413` |
| `files.findInFolder` | 在文件夹中查找（**当前为空实现**） | `:7430` |
| `files.refresh` | 刷新 | `:7418` |
| `files.collapseAll` | 全部折叠 | `:7423` |
| `settings.files.hiddenFiles` | 隐藏文件 | `:6510` |
| `settings.tabs.appearance` | 外观 | `:6230` |
| `quickOpen.loadingFiles` | 正在加载文件 | `:7835` |
| `welcome.openProject` | 打开项目 | `:6154` |

### 5.5 侧栏 · 搜索与文件导航器

| key | 中文 | 行号 |
|---|---|---|
| `search.search` | 搜索 | `locale.ts:5892` |
| `search.searchInFiles` | 在文件中搜索 | `:4608`（`workbench.searchInFiles`） |
| `search.clear` | 清除搜索 | `:5895` |
| `search.matchCase` / `matchWholeWord` / `useRegex` | 区分大小写 / 全字匹配 / 使用正则表达式 | `:5889,5890,5891` |
| `search.showDetails` / `hideDetails` | 显示详细信息 / 隐藏详细信息 | `:5894,5893` |
| `search.filesToInclude` / `filesToExclude` | 要包含的文件 / 要排除的文件 | `:5907,5908` |
| `search.noResults` | 未找到结果 | `:5886` |
| `search.options` | 搜索选项 | `:5906` |
| `search.replacedMatches` / `replaceMatchesFailed` | 已替换 {count} 处匹配 / 替换搜索匹配失败 | `:5904,5905` |
| `fileNavigator.files` / `flatList` / `fileTree` | 文件 / 平铺列表 / 文件树 | `:7485,7488,7489` |
| `fileNavigator.showingCount` | 显示 {visible} / {total} | `:7490` |
| `fileNavigator.noFilesMatch` | 没有匹配的文件 | `:7491` |
| `fileNavigator.searchResultFiles` | 搜索结果文件 | `:7493` |

### 5.6 硬编码英文（重写时应补 i18n 键，或保留原文）

| 文案 | 位置 |
|---|---|
| `Show All` | `layout/components/sidebar/main-sidebar.tsx:756` |
| `Opening {project}` | `main-sidebar.tsx:625` |
| `Indexing N files` / `Indexing files` / `Preparing search` / `Searching a/b files` / `Searching files` / `Loading more results` | `global-search/components/global-search-buffer.tsx:453,457,462,466,469,474` |
| `N results (M total)` | `global-search-buffer.tsx:490` |
| `Load all search results before replacing all` | `global-search-buffer.tsx:540` |
| `Showing x of y results` | `global-search-results.tsx:102` |

---

## 6. gpui-kit 0.6.6 对应建议

### 6.0 路径约定与 crate 映射（先读，否则源码路径全错）

gpui-kit 源码根：`D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\`。下文用 `GPUI/` 代表该前缀（本文唯一一处相对化处理，因为它是 cargo registry，不在仓库内）。

| 简写 | 绝对路径 |
|---|---|
| `GPUI/gpui-kit-0.6.6/...` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-kit-0.6.6\...` |
| `GPUI/gpui-base-0.6.6/...` | `...\gpui-base-0.6.6\...` |
| `GPUI/gpui-component-0.6.6/...` | `...\gpui-component-0.6.6\...` |
| `GPUI/gpui-pre-0.3.6/...` | `...\gpui-pre-0.3.6\...` |

`GPUI/gpui-kit-0.6.6/src/` 下**只有 `lib.rs` 和 `test.rs`**，是纯 facade：
- `pub use ::gpui::*;` → `GPUI/gpui-kit-0.6.6/src/lib.rs:95`（`gpui` = crate `gpui-pre` 的 rename，`GPUI/gpui-kit-0.6.6/Cargo.toml:355-357`：`version = "=0.3.6"` / `package = "gpui-pre"`）
- `pub use ::gpui_base as base;` → `GPUI/gpui-kit-0.6.6/src/lib.rs:106`
- `pub use ::gpui_component as component;` → `GPUI/gpui-kit-0.6.6/src/lib.rs:143`
- `pub use ::gpui_kit_assets as assets;` → `GPUI/gpui-kit-0.6.6/src/lib.rs:145`
- `pub fn init` → `GPUI/gpui-kit-0.6.6/src/lib.rs:156`

因此「`gpui_kit::component::xxx`」实际住在 `gpui-component-0.6.6`，「`gpui_kit::base::xxx`」住在 `gpui-base-0.6.6`，「`gpui_kit::uniform_list`」来自 gpui 本体。

### 6.1 逐元素对应表

| # | 本文元素 | gpui-kit 类型（模块路径::类型名） | 判定 | 说明 |
|---|---|---|---|---|
| 1 | 标签栏（活动/悬停/横向滚动/溢出） | `gpui_kit::component::tab::{TabBar, Tab, TabVariant}`；底座 `gpui_kit::base::Tabs` | **需自行组合** | 标签条、活动态、悬停态、横向滚动、溢出「more」菜单内建；**脏标记圆点、pin、拖拽重排 gpui-kit 没有**，关闭按钮靠 `Tab::suffix` |
| 2 | 垂直/水平可调整分栏 + 分隔条拖拽 | `gpui_kit::base::resizable::{v_resizable, h_resizable, resizable_panel, ResizablePanelGroup, ResizablePanel, ResizableState, ResizablePanelEvent}` | **可一比一** | 拖拽、最小尺寸、双击重置语义需自己接 `on_resize`/`reset_panel`；光标与 handle 配色内建 |
| 3 | 侧栏容器（可折叠 + 头部/底部 + 列表） | `gpui_kit::component::sidebar::{Sidebar, SidebarCollapsible, SidebarItem, SidebarToggleButton, SidebarMenu, SidebarMenuItem, SidebarGroup, SidebarHeader, SidebarFooter}` | 可折叠侧栏 **可一比一**；**独立「图标 rail / 活动栏」gpui-kit 没有** | 折叠态宽 48px（`COLLAPSED_WIDTH`），展开默认 255px；活动栏要么用 `Sidebar::collapsed(true)`，要么自己拼 |
| 4 | 项目文件树（层级/展开/选中/虚拟滚动） | `gpui_kit::component::tree::Tree`（返回 `ListItem`）；`gpui_kit::base::tree::{Tree, TreeState, TreeItem, TreeEntry, TreeEntryState, TreeEvent}` | **需自行组合** | 层级、展开折叠、选中、键盘、虚拟滚动内建；**缩进、图标、展开箭头、缩进参考线全部要自己在 `render_item` 里画** |
| 5 | 平铺列表（搜索结果） | `gpui_kit::component::list::{List, ListState, ListDelegate, ListItem, SeparatorItem, IndexPath}` | **可一比一** | 行高必须一致（见 §6.3.1）；分页用 `has_more` / `load_more_threshold` |
| 5b | 面包屑下拉 | `gpui_kit::component::breadcrumb::{Breadcrumb, BreadcrumbItem}` + `gpui_kit::component::button::DropdownButton` | **需自行组合** | `BreadcrumbItem` **不实现 `ParentElement`，没有内建下拉**，只能 `on_click`；下拉要换成 `DropdownButton` |
| 6 | 编辑器宿主容器 | `gpui_kit::component::dock::{DockArea, DockSkin, Panel, PanelView, TabGroup, InsertTarget}`；文档控件 `gpui_kit::component::input::{Editor, EditorState, EditorMode}` | **需自行组合** | 用 `DockArea::split_at` 做分屏；文档/只读预览用 `Editor`；`Panel` trait 负责标题栏/工具栏/持久化 |
| 6b | 面包屑栏 | `gpui_kit::component::breadcrumb::{Breadcrumb, BreadcrumbItem}` | **需自行组合** | 分隔符硬编码 `IconName::ChevronRight`；**不解析路径、不折叠中间段**，长路径省略要自己算 |
| 6c | 底部状态栏 | `gpui_kit::component::status_bar::StatusBar` | **可一比一** | `left` / `right` / 中间区三分；默认 `py_1 px_2 border_t_1 text_xs` |
| 7 | 内联重命名输入框 | `gpui_kit::component::input::{Input, InputState, InputEvent}` | **可一比一** | `PressEnter` / `Blur` 事件驱动提交，`select_all` + `focus` 复刻「rAF 后全选」 |
| 8 | 右键菜单 / 上下文菜单 | `gpui_kit::component::menu::{ContextMenuExt, ContextMenu, PopupMenu, PopupMenuItem}` | **可一比一** | `.context_menu(...)` 扩展；**元素必须有稳定 `id`** |
| 9 | 空状态（图标+标题+描述） | `gpui_kit::component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle, EmptyDescription, EmptyContent}` | **可一比一** | 渲染顺序契约 `media → title → description`；根默认虚线圈选块，不需要时 `.border_0()` |
| 10 | 工具提示 tooltip | `gpui_kit::component::tooltip::Tooltip` + `gpui_kit::base::{Tooltip, TooltipOverlay, TooltipRequest, TooltipPositioner}` | **需自行组合** | **没有公开的通用 `.tooltip()` 扩展**（`ManagedTooltipExt` 与 `Root::tooltip_overlay` 都是 `pub(crate)`）；只有 Button/Toggle/Checkbox/Radio/Switch/Clipboard/InputGroup 自带 |

### 6.2 关键 API 速查（签名 + 行号）

**标签栏**
- `TabBar::new(id)` `GPUI/gpui-component-0.6.6/src/tab/tab_bar.rs:59`；`.with_variant(TabVariant)` `:80`；`.pill()/.outline()/.segmented()/.underline()` `:86/92/98/104`
- `.menu(bool)` `:110`（溢出下拉）；`.max_width(Pixels)` `:118`（超长省略号）；`.track_scroll(&ScrollHandle)` `:127` ⚠️ **不会自动把选中标签滚进视野**（`：124-126` 文档），要自己 `scroll_to_item`
- `.prefix(..)` `:133`；`.suffix(..)` `:139`；`.child(Tab)` `:151`；`.selected_index(usize)` `:157`；`.on_click(Fn(&usize, &mut Window, &mut App))` `:171`；`impl Styled` `:346`；`impl Sizable` `:352`
- 横向滚动：`h_flex().id("tabs-inner").flex_1().overflow_x_scroll().lock_scroll_axis()` `GPUI/gpui-component-0.6.6/src/tab/tab_bar.rs:536-549`
- `Tab::new()` `GPUI/gpui-component-0.6.6/src/tab/tab.rs:478`；`.label(SharedString)` `:483`；`.icon(Icon)` `:499`；`.prefix(Element)` `:535`；**`.suffix(Element)` `:541`（关闭/固定按钮唯一挂载点）**；`.disabled(bool)` `:547`；`.on_click(..)` `:553`；`ParentElement` `:581`；`Selectable` `:587`；`InteractiveElement` `:598`；`StatefulInteractiveElement` `:604`（`on_drag` 的落点）
- 状态样式：`normal` `tab.rs:128`、`hovered` `:172`、`selected` `:221`、`disabled` `:266`；指示器 spring `tab_bar.rs:185,227-240`
- 拖拽重排的唯一现成参考实现（dock 里）：`GPUI/gpui-component-0.6.6/src/dock/tab_panel.rs:150`（`fn tab_drag`）、`:401`、`:525`、`:581`（`drag_over`）

**分栏 / 分隔条**
- ⚠️ 命名陷阱：**`v_resizable` = `Axis::Vertical` → 容器 `v_flex()`（上下堆叠，分隔条水平）**——`GPUI/gpui-base-0.6.6/src/resizable/panel.rs:142-146`；`h_resizable` 才是左右并排
- `ResizablePanelGroup::new(id)` `panel.rs:43`；`.with_state(&Entity<ResizableState>)` `:67`（不传则内部 `use_keyed_state` `:139-141`）；`.axis(Axis)` `:73`；`.child(ResizablePanel)` `:83`；`.size(Pixels)` `:101`（**仅交叉轴** `:157-162`）；`.on_resize(Fn(&Entity<ResizableState>, &mut Window, &mut App))` `:111`；`.with_handle_appearance(ResizeHandleRenderer)` `:59`；`EventEmitter<ResizablePanelEvent>` `:135`
- `ResizablePanel`：`.size(Into<Pixels>)` `:275`；`.size_range(Into<Range<Pixels>>)` `:283`（对应 `MIN_PANE_SIZE`）；`.visible(bool)` `:269`；`Styled` `:289`；`ParentElement` `:295`。**别自己调** `.flex_basis/.absolute/.overflow_hidden`（`:231-236`）；定宽 panel 用 `.flex_none()`（`:224-229` 官方示例）
- `ResizableState`（`GPUI/gpui-base-0.6.6/src/resizable/mod.rs:33`）：`.sizes() -> &Vec<Pixels>` `:56`；`.container_size()` `:249`；`.resize_panel(ix, size, window, cx)` `:69`；`.insert_panel` `:95`；`.remove_panel` `:221`；**`.reset_panel(ix, cx)` `:233`（用于复刻"双击分隔条重置"）**；`.clear()` `:242`
- 拖拽链路（照抄即可）：handle 自动挂 `.on_drag(DragPanel, ..)` `panel.rs:363-379` → `window.on_mouse_event` `:441-471` → 松开 `done_resizing` + 触发 `on_resize` `:474-487`；尺寸算法 `resizable/mod.rs:278-348`；光标 `resize_handle.rs:165,174,182`
- 分隔条度量/配色：`HANDLE_PADDING = px(4.)` `resize_handle.rs:11`、`HANDLE_SIZE = px(1.)` `:12`、取色 `resize_handle.rs:286-295`（`theme.resizable.handle` → fallback `tokens.colors.border`；激活 fallback `ring`）
- `PANEL_MIN_SIZE = px(100.)` `resizable/mod.rs:14`（Windows 侧 `MIN_PANE_SIZE` 是比例 10，语义不同，需换算）
- 替代路径（更重）：dock `DockArea::split_at` `GPUI/gpui-base-0.6.6/src/dock/dock_area.rs:580`、`InsertTarget::Split` `:442`、`PaneRef::Split` `GPUI/gpui-base-0.6.6/src/dock/layout/node.rs:57-58`

**侧栏**
- `Sidebar::new(id)` `GPUI/gpui-component-0.6.6/src/sidebar/mod.rs:238`；`.side(Side)` `:254`；**`.collapsible(Into<SidebarCollapsible>)` `:264`**（`bool` 也可 `:48-52`）；**`.collapsed(bool)` `:270`**；`.header(Element)` `:276`；`.footer(Element)` `:282`；`.child(E)` `:288`；`.children(..)` `:294`；`Styled` `:373`（用 `.w(px(..))` 定展开宽，内部读 `style.size.width` `:196-202`）
- 宽度常量：`DEFAULT_WIDTH = px(255.)` `mod.rs:27`、**`COLLAPSED_WIDTH = px(48.)` `:28`**；折叠动画 `EffectTransition::width` 200ms `:540-544`；折叠态 `.w(COLLAPSED_WIDTH).gap_2()` `:424-426`
- **`Sidebar` 内部就是虚拟列表**：`ListState::new(content_len, ListAlignment::Top, overdraw)` `:390` + `list(...)` `:447-467` + `.vertical_scrollbar(&list_state)` `:470`，overdraw = 视口高 × 0.3 `:385`
- `SidebarMenuItem::new(label)` `sidebar/menu.rs:113`；`.icon(Icon)` `:133`；**`.active(bool)` `:145`（对应选中态）**；`.collapsed(bool)` `:160`；`.on_click(..)` `:151`；`.suffix(F)` `:199`；`.context_menu(..)` `:225`；折叠态自动用 label 当 tooltip `:220`
- `SidebarToggleButton::new()` `mod.rs:311`；`.side(Side)` `:323`；`.collapsed(bool)` `:329`；`.on_click(..)` `:335`；图标自动切 `PanelLeftOpen/Close` `:349-361`
- 配色 token：`cx.theme().tokens.sidebar` `:413`、`sidebar_foreground` `:414`、`sidebar_border` `:415`

**文件树**
- `TreeItem::new(id, label)` `GPUI/gpui-base-0.6.6/src/tree.rs:99`；`.child(TreeItem)` `:111`；`.children(..)` `:116`；`.expanded(bool)` `:121`；`.disabled(bool)` `:126`；`.is_folder()` `:132`（**= `!children.is_empty()`，不是文件系统概念**）；`.is_expanded()` `:141`；`.ancestors(&SharedString)` `:146`。public 字段：`id` `:42`、`label` `:43`、`children` `:44`
- `TreeEntry`：`.item()` `:61`；**`.depth()` `:66`（缩进的唯一依据）**；`.is_root()` `:71`；`.is_folder()` `:76`；`.is_expanded()` `:81`
- `TreeEntryState`：**`.is_selected()` `:171`**；`.is_right_clicked()` `:176`
- `TreeState::new(cx)` `:197`；`.items(..)` `:209`；`.set_items(.., cx)` `:214`（**会清空选中** `:216-218`）；`.set_selected_index(Option<usize>, cx)` `:225`；**`.set_selected_item(.., cx)` `:230`（自动展开祖先 `:234`）**；`.selected_item()` `:243`；`.entry(ix)` `:252`；`.index_of(&SharedString)` `:264`；**`.reveal_item(&SharedString, ScrollStrategy, cx)` `:268`（对应「自动定位活动文件」）**；`.scroll_handle() -> &UniformListScrollHandle` `:256`；`.scroll_to_item(ix, ScrollStrategy)` `:260`；`.focus(window, cx)` `:280`
- `gpui_kit::base::tree::Tree::new(&Entity<TreeState>)` `:477`；`.item(R: Fn(usize, &TreeEntry, TreeEntryState, &mut Window, &mut App) -> AnyElement)` `:488`；`.list_style(StyleRefinement)` `:497`
- `gpui_kit::component::tree::Tree::new(&Entity<TreeState>, R: Fn(usize, &TreeEntry, bool, ..) -> ListItem)` `GPUI/gpui-component-0.6.6/src/tree.rs:41`；`.context_menu(F: Fn(usize, &TreeEntry, PopupMenu, &mut Window, &mut Context<TreeState>) -> PopupMenu)` `:55`
- 键盘内建：`up/down/left/right` `GPUI/gpui-base-0.6.6/src/tree.rs:20-27`；`key_context() == "Tree"` `:30`；左右折叠/展开 `:365-387`；Enter 在 folder 上切换 `:350-363`
- 剪贴板 / 展开状态：不在 Tree 里，需自己接 action（Windows 侧树键盘行为见 §3.3）

**平铺列表**
- `ListState::new(delegate, window, cx)` `GPUI/gpui-component-0.6.6/src/list/list.rs:94`；`.searchable(bool)` `:125`（内建搜索框 `:673-689`）；`.selectable(bool)` `:136`；`.set_selected_index(Option<IndexPath>, window, cx)` `:184`；`.set_right_clicked_index(..)` `:199`；`.set_query(&str, window, cx)` `:215`；`.scroll_to_item(..)` `:239`；`.scroll_to_selected_item(window, cx)` `:263`；`.scroll_handle() -> &VirtualListScrollHandle` `:259`；`.focus(window, cx)` `:156`
- `ListDelegate`（`GPUI/gpui-component-0.6.6/src/list/delegate.rs`）：`type Item: Selectable + IntoElement` `:11`；`perform_search(&mut self, query, window, cx) -> Task<()>` `:15`；`sections_count(cx) -> usize` `:27`；`items_count(section, cx) -> usize` `:35`；**`render_item(ix: IndexPath, window, cx) -> Option<Self::Item>` `:42`**；`render_section_header` `:52`；`render_empty` `:74`；`render_initial` `:95`；`render_loading` `:110`；`set_selected_index` `:119`；`confirm(secondary, window, cx)` `:139`；`cancel` `:143`；`has_more(cx)` `:148`；`load_more_threshold() -> usize` `:159`（默认 20）
- `List::new(&Entity<ListState<D>>)` `list.rs:732`；`.scrollbar_visible(bool)` `:741`；`.search_placeholder(SharedString)` `:747`；`Sizable` `:766`
- 虚拟滚动原语：`gpui_kit::base::{VirtualList, v_virtual_list, h_virtual_list, VirtualListScrollHandle}` `GPUI/gpui-base-0.6.6/src/lib.rs:200-201`；`VirtualListScrollHandle::new()` `GPUI/gpui-base-0.6.6/src/virtual_list.rs:89`；`.scroll_to_item(ix, ScrollStrategy)` `:107`；`.scroll_to_bottom()` `:123`

**面包屑 / 状态栏 / 宿主**
- `Breadcrumb::new()` `GPUI/gpui-component-0.6.6/src/breadcrumb.rs:119`；`.child(BreadcrumbItem)` `:127`；`.children(..)` `:133`；分隔符硬编码 `IconName::ChevronRight` + `size_3p5()` `:141-147`；样式 `h_flex().gap_1p5().text_sm().text_color(cx.theme().muted_foreground)` `:171-176`
- `BreadcrumbItem::new(label)` `:31`；`.disabled(bool)` `:42`；`.on_click(..)` `:47`；**无 `ParentElement`、无下拉** `:20-27`
- 下拉替身：`DropdownButton::new(id)` `GPUI/gpui-component-0.6.6/src/button/dropdown_button.rs:43`；`.button(Button)` `:76`；`.dropdown_menu(F)` `:82`；`.dropdown_menu_with_anchor(F, Anchor)` `:91`；`.outline()` `:104`；实战 `tab_bar.rs:560-583`
- `StatusBar::new()` `GPUI/gpui-component-0.6.6/src/status_bar.rs:41`；`.left(Element)` `:51`；`.right(Element)` `:57`；`ParentElement`（child 落中间区）`:65`；三段对齐规则 `:98-104`；样式 `:88-95`
- `DockArea::new(id, version: Option<usize>, window, cx)` `GPUI/gpui-base-0.6.6/src/dock/dock_area.rs:126`；`.with_renderer(Rc<dyn DockAreaRenderer>)` `:154`；`.set_center(DockLayout, window, cx)` `:232`；`.add_panel<P: Panel>` `:371`；`.select_panel` `:558`；**`.split_at` `:580`**；`.set_zoomed_in/.set_zoomed_out/.is_zoomed` `:626/635/639`（对应窗格全屏）；`.dump/load` `:794/716`（布局持久化）；`actions!(dock, [ToggleZoom, ClosePanel])` `GPUI/gpui-component-0.6.6/src/dock/mod.rs:59`
- `Panel` trait：`panel_name()` `GPUI/gpui-base-0.6.6/src/dock/panel.rs:23`（持久化标识，"Once chosen, never change it" `:22`）；`visible` `:28`；`closable` `:34`；`zoomable` `:40`；`set_active` `:53`；`set_zoomed` `:61`；`on_added_to(WeakEntity<TabGroup>)` `:68`；`on_removed` `:82`；`dump` `:90`；呈现半 `Panel`/`PanelView`/`TitleStyle`/`PanelHandle` 在 `GPUI/gpui-component-0.6.6/src/dock/panel.rs:71/153/40/215`
- 文档控件 `Editor::new(&Entity<EditorState>)` `GPUI/gpui-component-0.6.6/src/input/editor.rs:39`；`.h(Into<DefiniteLength>)` `:56`；`.bordered(bool)` `:66`；`.readonly(bool)` `:81`；`.context_menu(..)` `:105`；`.on_paste(..)` `:119`；`EditorState::new(window, cx)` `GPUI/gpui-base-0.6.6/src/input/base/state.rs:9230`（默认行号开、tab 2、缩进导线开、多行、搜索开，`：9221-9225`）；语法高亮/LSP/折叠/诊断在 `GPUI/gpui-base-0.6.6/src/input/editor/{highlighting,diagnostics,lsp,display_map/folding}.rs`

**内联输入 / 菜单 / 空态 / tooltip**
- `Input::new(&Entity<InputState>)` `GPUI/gpui-component-0.6.6/src/input/input.rs:180`；`.id(..)` `:174`；`.h(Into<DefiniteLength>)` `:257`；`.appearance(bool)` `:263`；`.bordered(bool)` `:269`；`.suffix(Element)` `:245`；`.disabled(bool)` `:310`；`.context_menu(..)` `:334`
- `InputState::new(window, cx)` `GPUI/gpui-base-0.6.6/src/input/base/state.rs:8958`；`.set_value(..)` `:897`；`.value() -> SharedString` `:1197`；`.set_placeholder(..)` `:850`；**`.focus(&self, window, cx)` `:1255`**；**`.select_all(window, cx)` `:2835`**；`InputEvent` = `Change | PressEnter { secondary, shift } | Focus | Blur` `:122-127`（`PressEnter` emit 点 `:1998`）
- `ContextMenuExt::context_menu` `GPUI/gpui-component-0.6.6/src/menu/context_menu.rs:19-22`；**元素必须有稳定 `id`**，否则 fallback `ElementId::CodeLocation`，重渲染丢开关状态 `:26-35`；触发/关闭恢复焦点 `:296-358,303-311`；`PopupMenu` `GPUI/gpui-component-0.6.6/src/menu/popup_menu.rs:282`，`.item` `:728`、`.separator()` `:680`、`.menu(label, Box<dyn Action>)` `:465`、`.menu_with_check(..)` `:565`、`.submenu(..)` `:694`、`.max_h(..)` `:441`；`PopupMenuItem` `:33`，`.icon` `:126`、`.disabled` `:161`、`.checked` `:180`、`.on_click` `:196`
- `Empty::new()` `GPUI/gpui-component-0.6.6/src/empty.rs:23`；`.header(EmptyHeader)` `:33`；`.content(EmptyContent)` `:39`；顺序契约 `media → title → description` `:138-152`（且"Additional direct children follow the named slots, regardless of builder call order" `:10-12`）；根样式 `:65-78`（**默认虚线边框**）；`EmptyMediaVariant::Icon` → `size_8().rounded(radius.lg).bg(cx.theme().muted)` `:216-223`；`EmptyTitle::new()` `:237`（`text_sm().font_medium()` `:266-273`）；`EmptyDescription::new()` `:286`（`text_sm().line_height(1.625).text_color(cx.theme().muted_foreground)` `:314-322`）；`EmptyContent::new()` `:335`（`max_w(rems(24.)).gap_2p5()` `:362-368`）。⚠️ **无 action/按钮槽**，按钮放 `EmptyContent` 或直接 `.child(Button..)`
- `Tooltip::new(text)` `GPUI/gpui-component-0.6.6/src/tooltip.rs:43`；`Tooltip::element(F)` `:53`；`.action(&dyn Action, Option<&str>)` `:69`（自动显示快捷键，可用于 `tabs.goBackShort` 等 tooltip）；`.build(&mut Window, &mut App) -> AnyView` `:81`；配色 `:114-125`（`tokens.popover` / `popover_foreground` / `border` / `radius`）
- 自带 `.tooltip()` 的组件（可直接用）：`Button` `GPUI/gpui-component-0.6.6/src/button/button.rs:389`（+`.tooltip_placement` `:398`、`.tooltip_with_action` `:404`）、`Toggle` `button/toggle.rs:72`、`Checkbox` `checkbox.rs:66`、`Radio` `radio.rs:64`、`Switch` `switch.rs:101`、`Clipboard` `clipboard.rs:45`、`InputGroup` `input/group.rs:502`

### 6.3 四条必须核实的结论

#### 6.3.1 `uniform_list` 只测量一行 → **行高必须 uniform**

模块文档原文（`GPUI/gpui-pre-0.3.6/src/elements/uniform_list.rs:1-5`）：
> "Rather than use the full taffy layout system, uniform_list simply **measures the first element** and then lays out all remaining elements in a line based on that measurement. … only works for elements with uniform height."

链路证据：
- 工厂默认 `item_to_measure_index: 0` — `uniform_list.rs:41-43`；唯一 setter `with_width_from_item(Option<usize>)`，`None → 0` — `:621-625`
- `measure_item` 只渲染 1 个 item：`let item_ix = cmp::min(self.item_to_measure_index, self.item_count - 1); let mut items = (self.render_items)(item_ix..item_ix + 1, window, cx);` — `:658-680`
- 该高度被用于全部布局：`:359` 测量 → `:371` `let item_height = longest_item_size.height;` → `:397` `content_height = item_height * self.item_count` → `:427-428` 每行 top/bottom → `:473-476` 可见区间反算 `/- item_height` → `:506` 每行拿 `AvailableSpace::Definite(item_height)` → `:520` 传给 scrollbar
- gpui-components 自己依赖该前提：`GPUI/gpui-component-0.6.6/src/list/delegate.rs:41` — `NOTE: Every item should have same height.`（section header `:51`、footer `:63` 同）

**对本项目的直接影响**：
- 侧栏文件树行高 = 固定 24px（`file-explorer/lib/file-tree-row.ts:7-14`）**正好满足**该约束，`gpui_kit::base::tree::Tree` 可行。
- 但 Windows 侧文件树允许 `fileTreeIndentSize` 变缩进而**不变行高**，安全。
- **标签栏不受影响**（不是 uniform_list）；但搜索结果列表若想「首行/分组头更高」必须换 `VirtualList`：`GPUI/gpui-base-0.6.6/src/virtual_list.rs:10` — "Unlike the `uniform_list`, the each item can have different size."，签名 `v_virtual_list` `:139`（逐行给高度，`:133-135` 文档）。
- ⚠️ `ListItem` 固定 `py_1()`（`GPUI/gpui-component-0.6.6/src/list/list_item.rs:186`），**不可能靠 `ListItem` 复刻 24px 行高**（`py_1` = 上下 4px + 内容高）；要精确 24px 需用 `gpui_kit::base::tree::Tree` + 自绘 `AnyElement` 行（`base/tree.rs:488`）。

#### 6.3.2 `ListItem` children 排布 → **横排外壳 + 单个 `div()` 内的竖排内容，无 root/child 概念**

- 根是 `h_flex()`：`ListItem::new` → `base: h_flex().id(id)` — `GPUI/gpui-component-0.6.6/src/list/list_item.rs:44-48`
- `children: SmallVec<[AnyElement; 2]>` — `:40`；`impl ParentElement` 只 push — `:159-163`（**无 depth / 无父子关系**）
- 渲染：根 `h_flex()`（`.items_center().justify_between()`）→ 内层 `h_flex().w_full()` → **`.child(div().w_full().children(self.children))`** — `:182-232`（关键行 `:215-232`）
- `div()` 默认 `Display::Block` → 竖排：`GPUI/gpui-pre-0.3.6/src/elements/div.rs:3407-3408`（`Style::default()` + refine）、`GPUI/gpui-pre-0.3.6/src/style.rs:774`（`display: Display::Block`）、`style.rs:1279-1287`（映射 taffy `Display::Block`）；对照 `h_flex()`/`v_flex()` 才显式设方向 — `GPUI/gpui-base-0.6.6/src/styled.rs:38-48`
- 槽位：`check_icon` 固定 `w_5()` 在右 `:222-231`；`suffix` 最右 `:233`
- 状态：`selected` / `secondary_selected`（右键）/ `confirmed` / `disabled` / `separator` `:64-90`；悬停 `cx.theme().tokens.list_hover` `:209`；选中 `cx.theme().list_active` 或 `cx.theme().accent`（由 `cx.theme().list.active_highlight` 决定）`:234-246`

**结论**：`ListItem` 只能表达「一行」，**树形缩进/层级必须调用方自己在 children 之前用 padding 表达**（印证 6.3.4）。要把 Windows 侧「左：箭头+图标+名字；右：git 状态+动作」一行做出来，根横排骨架够用；但要在同一行里做「名字 + 灰色目录后缀」（`sidebar-tree.tsx:258-265` 的 `items-baseline gap-1.5`），需要在 children 里再放一个 `h_flex()`——因为默认那个 `div()` 是竖排。

#### 6.3.3 多行输入**不必须**显式给高度（但要给行数）

- 组件层多行走 `h_auto()`，高度可选：`GPUI/gpui-component-0.6.6/src/input/input.rs:706-709`
  ```rust
  .when(presentation.is_multi_line(), |this| {
      this.h_auto().when_some(self.height, |this, height| this.h(height))
  })
  ```
  单行分支才固定高 — `:700-703`
- `Textarea` 只是 `Input` 薄壳 — `GPUI/gpui-component-0.6.6/src/input/textarea.rs:14-24`、`into_input` `:141-159`；`height: Option<DefiniteLength>` 默认 `None` `:17,40`；`.h(..)` `:54`
- 行数控制（`GPUI/gpui-base-0.6.6/src/input/base/state.rs`，`impl InputBaseState<TextareaMode>` `:9158`）：`set_auto_grow(min_rows, max_rows, cx)` `:9167`（doc `:9211` "Grow with the content from min_rows through max_rows"）、builder `auto_grow(min_rows, max_rows)` `:9212`、`rows(usize)` `:9178`（`#[doc(hidden)]`）、`set_rows(&mut self, rows, cx)` `:9195`；doc `:9172-9176`：「default: 2」「only used when multi_line is set to true」
- `TextareaState::new(window, cx)` `:9163`，注释 `:9161-9162`：「Being multi-line is carried by the mode, not by the layout, so the default plain-text layout needs no adjustment here.」
- 内层编辑器总是填满外层框架：`.flex_1().when(self.is_multi_line(), |this| this.h_full())` — `state.rs:4259-4260`；因此只有「外层高度不确定」时才会塌，而 `h_auto()` 正好把外层高度从 rows 推出来，自洽
- `EditorState::new(window, cx)` `:9230`，默认 `rows: 2`（doc `:9221-9225`）
- ⚠️ **源码内一处不一致**：`LayoutMode::default()` = `plain_text()` = `PlainText { rows: 1 }`（`GPUI/gpui-base-0.6.6/src/input/base/mode.rs:57-71`），而 `CodeEditor` 默认 `rows: 2`（`mode.rs:78`），`rows` 的 doc 又说 textarea 默认 2。**依赖「默认 2 行」时必须显式写** `.auto_grow(2, N)` 或 `.set_rows(2, cx)`。
- `LayoutMode` 本身是 `pub(crate)`（`mode.rs:29`），外部只能用上述公开方法。

**对本项目的直接影响**：Windows 侧没有多行文本输入组件在编辑区/侧栏（唯一的多行是 Monaco，非 gpui 控件），所以这条在本范围内不构成约束；但如果要用 `Textarea` 做比如终端输入框/提交信息，**要显式给 `auto_grow(2, N)` 而不是不给高度**。

#### 6.3.4 Tree 的 API 与虚拟滚动 → **有 Tree，虚拟滚动支持（底层就是 `uniform_list`），但缩进/图标/箭头全无**

- **虚拟滚动确认**：`GPUI/gpui-base-0.6.6/src/tree.rs:423` — `uniform_list("entries", self.entries.len(), { cx.processor(move |state, visible_range: Range<usize>, window, cx| { visible_range.map(|ix| { ... }) }) })`；`.track_scroll(&self.scroll_handle)` `:461`（字段 `scroll_handle: UniformListScrollHandle` `:187`，初始化 `:201`）
- **等行高约束继承 6.3.1**：每行是 `div().id(ix).child(render_item(..))` `:432-441`，高度取决于 `render_item`，由调用方保证；`gpui_kit::component::tree::Tree` 强制返回 `ListItem`（`GPUI/gpui-component-0.6.6/src/tree.rs:41-43`），而 `ListItem` 固定 `py_1()`，**恰好满足但拿到的是 `ListItem` 的行高，不是 Windows 的 24px**
- **扁平化 + 展开折叠**：`add_entry` `:311-318`、`rebuild_entries` `:340-348`、`toggle_expand` `:320-338`；`set_selected_item` 会自动展开祖先 `:230-241`、`expand_ancestors` `:291-309`
- **无内建缩进/图标/箭头**：渲染循环 `:427-441` 只暴露 `entry.depth()`、**不加任何 padding**；`TreeItem` 无 icon 字段 `:41-46`。必须自己写，例如 `div().pl(px(BASE + INDENT * entry.depth() as f32))`（Windows 值是 `10 + depth × 16`，见 §2.4）
- 图标可直接用内置：`Icon::new(IconName::File | Folder | FolderOpen | ChevronRight)` — `GPUI/gpui-component-0.6.6/src/icon.rs:105`；资产清单 `GPUI/gpui-kit-assets-0.6.6/default-icons.txt:40,43,42,23`
- **选中/右键/滚动**：`TreeEntryState::is_selected()` `:171`、`is_right_clicked()` `:176`；`set_selected_index` `:225`、`scroll_to_item` `:260`、`reveal_item` `:268`；右键经 `ListItem::secondary_selected`（`GPUI/gpui-component-0.6.6/src/tree.rs:90`）
- **键盘**：`up/down/left/right` 内建 `:20-27`，`key_context() == "Tree"` `:30`，`.track_focus(&focus_handle).on_action(...)` `:521-527`；Windows 侧额外的 `Mod+C/X/V`、`F2`、`Home/End` 需自己加 action

### 6.4 主题 / 颜色 token 获取方式

- 入口 trait：`gpui_kit::component::ActiveTheme` — `GPUI/gpui-component-0.6.6/src/theme/mod.rs:44`，`fn theme(&self) -> &Theme` `:45`
  - ⚠️ **只有 `impl ActiveTheme for App`** — `mod.rs:48-53`；**未找到 `impl ActiveTheme for Context<T>` 或 `for Window`**（全 crate grep 只命中这一处）→ 在 `Render`/`Context<T>` 里写 `cx.theme()` 依赖 `Deref<Target = App>`；若编译失败，改用 `Theme::global(cx)`
- 全局读写：`Theme::global(cx: &App)` `mod.rs:198`；`Theme::global_mut(&mut App)` `:208`；`Theme::change(mode, window, cx)` `:261`
- `Theme` 结构 `mod.rs:107`：**`pub colors: ThemeColor` `:108`**、`pub tokens: ThemeTokens` `:114`（注释 `:110-113`：tokens 是 legacy，新代码建议 `semantic_tokens()`）、`font_family` `:126`、`font_size` `:128`、**`mono_font_family` `:141` / `mono_font_size` `:143`**（编辑器用）、`radius` `:145`、`radius_lg` `:147`、`shadow` `:148`、`focus_ring` `:156`、`scrollbar_mode` `:160`、`list: ListSettings` `:165`、`motion: MotionTokens` `:170`
- **`impl Deref for Theme { type Target = ThemeColor; }`** `mod.rs:179-180` → 所以 `cx.theme().foreground` 是 Deref 到 `ThemeColor`（`GPUI/gpui-component-0.6.6/src/theme/theme_color.rs:59`）
- 字段对照（Windows token → gpui-kit 字段，`theme_color.rs`）：
  - `--background` → `background` `:67`；`--foreground` → `foreground` `:163`；`--surface` → 可用 `popover` `:197` 或 `muted` `:193`；`--subtle-foreground` → **`muted_foreground` `:195`**；`--border` → `border` `:69`；`--accent` → `accent` `:61`；`--selected` → **`list_active` `:183`**；`--primary` → `primary` `:201`；`--selection` → `selection` `:227`；`--ring` → `ring` `:211`；`--input` → `input` `:173`
  - 列表/树：`list` `:181`、`list_active` `:183`、`list_active_border` `:185`、`list_even` `:187`、`list_head` `:189`、**`list_hover` `:191`（对应行悬停 `--accent/70`）**
  - 标签栏：`tab` `:261`、`tab_active` `:263`、`tab_active_foreground` `:265`、`tab_bar` `:267`、`tab_bar_segmented` `:269`、`tab_foreground` `:271`（⚠️ Windows 侧标签栏底色 = `--background`，需把 `tab_bar` 设成 background；活动标签指示条 3px `--primary` 要自绘，见 §2.1）
  - 侧栏：`sidebar` `:229`、`sidebar_accent` `:231`、`sidebar_accent_foreground` `:233`、`sidebar_border` `:235`、`sidebar_foreground` `:237`、`sidebar_primary` `:239`
  - 状态栏：`status_bar` `:297`、`status_bar_border` `:299`
  - 拖拽：`drag_border` `:159`、`drop_target` `:161`（对应 §2.3 的 `ring-2 ring-primary` / `bg-primary/10`）
- 新一套（建议新代码用）：`Theme::semantic_tokens()` `mod.rs:399`、`color_tokens()` `:414`、`motion_tokens()` `:410`、`radius_tokens()` `:471`、`spacing_tokens()` `:482`、`typography_tokens()` `:486`、`shadow_tokens()` `:495`、`radius_full()` `:445`
  - 注意 `empty.rs:65-78` 用的是 `cx.theme().radius_tokens().xl`，说明 radius token 与 Windows 的 `--radius*` 不是同一套刻度，需要做一次映射表（`--radius` 8px / `--radius-md` 6.4px / `--radius-lg` 8px / `--radius-xl` 11.2px ↔ gpui-kit `radius` / `radius_lg` / `radius_tokens().{md,lg,xl}`）
- gpui-base 侧主题（不依赖 component crate 也能用）：`gpui_kit::base::theme::{Theme, ResizableTheme, ScrollbarTheme, ThemeAppearance}` — `GPUI/gpui-base-0.6.6/src/lib.rs:182`；分隔条取色 `theme.resizable.handle` / `active_handle`，fallback `theme.tokens.colors.border` / `ring` — `GPUI/gpui-base-0.6.6/src/resizable/resize_handle.rs:286-295`（`ResizableTheme` 只有 `handle` / `active_handle` 两个字段）
- 组件内通行写法：`cx.theme().border`、`cx.theme().tokens.list_hover`、`cx.theme().muted_foreground` — `GPUI/gpui-component-0.6.6/src/list/list_item.rs:209`、`GPUI/gpui-component-0.6.6/src/status_bar.rs:92-95`

### 6.5 明确不存在（避免误用 gpui-kit 里没有的东西）

| 以为有 | 实际 | 证据 |
|---|---|---|
| `gpui_kit::component::uniform_list` | **不存在**；`uniform_list` 来自 gpui 本体，经 `pub use ::gpui::*` 暴露为 `gpui_kit::uniform_list` | `GPUI/gpui-pre-0.3.6/src/elements/uniform_list.rs:22`、`GPUI/gpui-kit-0.6.6/src/lib.rs:95` |
| 标签脏标记 / pin | 任何 crate 都没有 | `tab.rs:397-419`（无 dirty/pin 字段）、dock `TabGroup` 亦无 |
| 标签拖拽重排 | `TabBar` 无；现成逻辑只在 dock `TabGroup` | `tab_bar.rs` 全文件无 `on_drag`；`GPUI/gpui-component-0.6.6/src/dock/tab_panel.rs:150,401,525,581` |
| Tree 的内建缩进 / 图标 / 展开箭头 | 都没有 | `GPUI/gpui-base-0.6.6/src/tree.rs:427-441`、`:41-46` |
| `BreadcrumbItem` 的下拉能力 | 没有（不实现 `ParentElement`） | `GPUI/gpui-component-0.6.6/src/breadcrumb.rs:20-27` |
| 独立「活动栏 / Activity Bar」组件 | 没有 | `GPUI/gpui-component-0.6.6/src/sidebar/mod.rs` 全部条目中无 activity bar |
| 公开的通用 `.tooltip()` 扩展 | 没有（`ManagedTooltipExt` 与 `Root::tooltip_overlay` 都是 `pub(crate)`） | `GPUI/gpui-component-0.6.6/src/tooltip.rs:229`、`GPUI/gpui-component-0.6.6/src/root.rs:470,44` |
| `impl ActiveTheme for Context<T>` / `for Window` | 未找到（只有 `for App`） | `GPUI/gpui-component-0.6.6/src/theme/mod.rs:48` |
| `Empty` 的 action/按钮槽 | 没有 | `GPUI/gpui-component-0.6.6/src/empty.rs`（用 `EmptyContent` 或 `.child(Button..)`） |

### 6.6 重写落地建议（按风险排序）

1. **先搭骨架**：`h_resizable` 分栏（侧栏 ↔ 编辑区）+ `Sidebar`（可折叠）+ `DockArea`（编辑区内部拆分）。这三块是 6.1 里判定最稳的，且直接对上 §2.4/§2.3 的度量。
2. **标签栏自研**：`TabBar` + `Tab` 能覆盖活动/悬停/横向滚动/溢出菜单；**脏点、pin、拖拽重排要自己写**（拖拽可抄 `dock/tab_panel.rs:150-581`）。注意 `TabBar::track_scroll` 不会自动露出选中标签，必须自己 `scroll_to_item` 复刻 Windows 的 `scrollIntoView` 行为（`tabs/components/tab-bar.tsx:317-338`）。
3. **文件树用 `gpui_kit::base::tree::Tree`（而非 component 版）**：因为 Windows 行高是精确 24px（`lib/file-tree-row.ts:7-14`），而 component 版强绑 `ListItem`（`py_1`）拿不到精确高度。缩进 `10 + depth × 16`、参考线（7px 宽 / left 3px / 1px 线 / 20% subtle）、箭头 16px + 12px chevron、选中双底色（聚焦 `--selected` / 未聚焦 `--border`）全部自绘。
4. **空状态 / 右键菜单 / 内联重命名 / 状态栏**直接一比一，注意 `Empty` 根默认有虚线边框（`.border_0()` 去掉）、`context_menu` 必须有稳定 `id`。
5. **tooltip 最后处理**：通用 tooltip 只能手搓 `gpui_base::Tooltip` + `on_hover`；优先给 Button 用自带 `.tooltip()`。

---

## 7. 未查清

1. **`.file-tree-empty-state` 的 CSS 规则未找到**：类名在 `file-explorer-tree.tsx:1533,1540` 使用，但全仓 CSS 无定义 → 该空态样式实际只来自 `ui/empty.tsx`；需确认是否有遗漏的样式文件。
2. **活动栏 Rail 分隔条无键盘支持**（`main-sidebar.tsx:697-707` 只有 `onMouseDown`，`role="separator"` 却带 `aria-*`），未找到 `onKeyDown`。
3. **`workspace-session-restore.ts:52` 的 `ssh_read_file` 与 `document-watch-controller` 的完整命令组**未逐一核对 `src-tauri` 定义行；本文只核对了编辑/侧栏直接相关的一组。
4. **`write_file` 的前端调用点未精确定位**（`controllers/platform.ts:76-79` 走的是 `@tauri-apps/plugin-fs` 的 `writeTextFile`，而非 `invoke("write_file")`；`host::write_file`（`main.rs:164`）的实际调用方未找到）。
5. **`ui-text-lg` / `ui-text-xs` 无对应 CSS 变量定义**（`styles/utilities.css:30-44` 只有 4 个 `ui-text-*`），属空操作 class；若重写要还原设计意图，需回头问设计/作者。
6. **底部面板 `BottomBufferPane` 的完整结构与 TabBar 复用方式未展开**（`layout/components/bottom-pane/bottom-buffer-pane.tsx` 未逐行核对）。
7. **`GitView`（侧栏 git 面板）内部结构未展开** —— 属其它文（git）范围。
8. **`file-explorer-tree.tsx`（60KB / 1712 行）的拖拽自动滚动、多选、剪贴板粘贴细节只做了检索式确认**，未逐行通读；若重写要做到一比一，需要把它作为单独任务再拆。
9. **Monaco 内部子 UI（折叠、minimap、补全/悬停 widget、peek view）的精确度量未逐条核对**：本文只覆盖了 Lithe 自己重绘的部分（`editor/styles/monaco-editor.css:1-70` 等）。这些在 gpui-kit 重写中需要自绘，度量需另立规格。
