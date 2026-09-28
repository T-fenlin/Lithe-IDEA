# Lithe Windows 前端 → Rust + gpui-kit 0.6.6 界面规格：外壳类区域

> 本文由只读逆向调研生成，界面规格唯一来源为 Windows 前端 `windows/tauri/src/`（Tauri v2 + React + TypeScript + Tailwind + shadcn/Base UI）。
> 调研范围：应用外壳与窗口、标题栏、项目标签条、活动栏、状态栏、主题与尺寸令牌。
> 未读取、未参考 `macos/`；未修改任何已有文件。
> 每条结论给出 `相对路径:行号` 证据；无法确证处明确写“未找到”，不做推测。

---

## 0. 换算与来源约定

| 约定 | 内容 | 证据 |
| --- | --- | --- |
| 1 Tailwind 单位 | = 0.25rem = **4px** | 见下两行换算链 |
| 根字号 | `html { font-size: calc(16px * var(--app-ui-scale)) }` | `windows/tauri/src/styles/theme.css:220` |
| UI 缩放 | `--app-ui-scale: 1`（默认），运行期按字号设置重算 | `windows/tauri/src/styles/theme.css:113`、`windows/tauri/src/features/settings/lib/appearance-bootstrap.ts:155` |
| Tailwind 配置 | **CSS-first，无 `tailwind.config.*`**；令牌在 `@theme inline` 中声明 | `windows/tauri/src/styles/theme.css:1-101`；`windows/tauri/tailwind.config.*` 未找到（glob 无结果） |
| 字号工具类 | `.ui-text-caption / .ui-text-chrome / .ui-text-sm / .ui-text-base` 由 CSS 定义，不是 Tailwind 内置 | `windows/tauri/src/styles/utilities.css:30-44` |
| 颜色来源 | `lithe.json` 的 `colors.<key>` → CSS 变量 `--<key>`，写入 `document.documentElement` 内联样式 | `windows/tauri/src/extensions/themes/theme-file.ts:146`、`windows/tauri/src/extensions/themes/theme-registry.ts:92-99` |
| 主题切换标记 | `data-theme="<id>"` / `data-theme-type="light|dark"` | `windows/tauri/src/extensions/themes/theme-registry.ts:98-99` |
| 密度模式 | `:root[data-window-chrome-density="comfortable"]` 覆盖一批尺寸变量 | `windows/tauri/src/styles/theme.css:188-200`、`windows/tauri/src/features/settings/lib/ui-preferences.ts:9` |
| 状态栏隐藏 | `html[data-status-bar="hidden"] { --lithe-footer-height: 0rem }` | `windows/tauri/src/styles/utilities.css:106-108` |

---

## 1. 区域清单

### 1.1 React 组件层级（缩进即包含关系）

```
App                                   windows/tauri/src/App.tsx:56
└─ InitialWindowShell（首帧占位外壳）  windows/tauri/src/App.tsx:39-54
   ├─ 顶部拖拽条 h-10 (40px)          windows/tauri/src/App.tsx:50
   └─ 主区 h-[calc(100dvh-2.5rem)]    windows/tauri/src/App.tsx:51
└─ LocaleProvider → WorkbenchApp      windows/tauri/src/App.tsx:94-95
   └─ WorkbenchApp                    windows/tauri/src/workbench-app.tsx:31
      ├─ WindowResizeBorder（8 个不可见缩放热区）  windows/tauri/src/features/window/components/window-resize-border.tsx:209-231
      └─ div.window-container          windows/tauri/src/workbench-app.tsx:84
         └─ MainLayout                windows/tauri/src/features/layout/components/main-layout.tsx:81
            ├─ 拖放遮罩（isDraggingOver）           main-layout.tsx:282-290
            ├─ TitleBarWithSettings                 main-layout.tsx:292
            │  └─ TitleBar                          windows/tauri/src/features/window/components/title-bar/title-bar.tsx:54
            │     ├─ WindowMenuBar（Windows 用自绘菜单栏；compactMenuBar 时为浮动菜单）  window-menu-bar.tsx:36 / title-bar.tsx:205-233
            │     ├─ TitleProjectMenu（项目下拉）    title-bar.tsx:237 → title-project-menu.tsx:109
            │     ├─ Git 分支项（来自 footer 项模型）title-bar.tsx:238 → footer-git-branch-item.tsx:11
            │     ├─ 快速打开按钮（workbench.search）title-bar.tsx:242-254
            │     ├─ TitleBarUpdateControl → AppUpdateControl（仅 Windows）  title-bar.tsx:50-52、347
            │     ├─ WindowControls（最小化/最大化/关闭）  title-bar.tsx:296-302、349-355 → window-controls.tsx:21
            │     └─ SettingsDialog + ProjectPicker(portal)  title-bar.tsx:389-400
            ├─ ProjectTabBar（Windows 项目标签条）   main-layout.tsx:293 → windows/tauri/src/features/window/components/project-tab-bar.tsx:16
            ├─ 工作区（rootFolderPath && !showWelcomeControl 时）  main-layout.tsx:295-355
            │  ├─ div.lithe-workbench-glass         main-layout.tsx:297
            │  │  ├─ SidebarActivityRail expanded={false}    main-layout.tsx:302 → main-sidebar.tsx:120
            │  │  │  ├─ SidebarPaneSelector orientation="vertical"  main-sidebar.tsx:632-656 → sidebar-pane-selector.tsx:81
            │  │  │  ├─ SidebarProjectDots（项目圆点，可滑动切换）  main-sidebar.tsx:689-696 → sidebar-projects.tsx:38
            │  │  │  ├─ 拖拽改宽热区（仅 expanded 时）  main-sidebar.tsx:697-707
            │  │  │  └─ 右键 ContextMenu（操作 / 在活动侧栏中显示）  main-sidebar.tsx:710-762
            │  │  ├─ ResizablePane position="left"  main-layout.tsx:303-310 → resizable-pane.tsx:32
            │  │  │  └─ MainSidebar → SidebarPanel  main-sidebar.tsx:767-819
            │  │  ├─ 中间列（flex-1）                main-layout.tsx:312
            │  │  │  ├─ div.lithe-glass-island（rounded-xl 编辑器岛）  main-layout.tsx:313
            │  │  │  │  └─ WorkbenchErrorBoundary → CachedWorkspaceSplitViews  main-layout.tsx:314-316
            │  │  │  └─ BottomPane（terminalWidthMode==="editor"）  main-layout.tsx:318-322
            │  │  ├─ ResizablePane position="right"  main-layout.tsx:325-341
            │  │  │  ├─ NotificationsToolWindow     main-layout.tsx:332-335
            │  │  │  └─ MavenPane（isMavenSelected）main-layout.tsx:336-340
            │  │  └─ PluginActivityRail（右侧图标竖条）  main-layout.tsx:342 → plugin-activity-rail.tsx:11
            │  │     ├─ 扩展按钮（PuzzlePieceIcon）  plugin-activity-rail.tsx:36-49
            │  │     ├─ NotificationsTrigger（含未读徽标）  plugin-activity-rail.tsx:50 → notifications-trigger.tsx:15
            │  │     └─ Maven 按钮（条件渲染）      plugin-activity-rail.tsx:51-66
            │  └─ BottomPane（terminalWidthMode==="full"，全宽）  main-layout.tsx:345-351
            ├─ Footer（showStatusBar 为真时）        main-layout.tsx:354 → footer/footer.tsx:16
            │  ├─ ProjectPreparationStatus compact    footer.tsx:51 → project-preparation-status.tsx:9
            │  ├─ 文件路径面包屑 FilePathBreadcrumb → PathBreadcrumb   footer.tsx:52-56 / footer-file-path-item.tsx:30-37
            │  ├─ useFooterEditorStatusItems 的项    footer.tsx:37-42 / footer-editor-status.tsx:26
            │  │  ├─ CursorPositionChip（cursor）    footer-editor-status.tsx:70-75
            │  │  ├─ encoding / indent / readOnly    footer-editor-status.tsx:77-106
            │  │  ├─ memory（ApplicationMemoryPoller）footer-editor-status.tsx:107-121
            │  │  └─ gitChanges（FooterStatusChip）  footer-editor-status.tsx:122-141
            │  └─ 项内使用 FooterStatusChip / FooterStatusLabel / FooterTabControl  footer-status-chip.tsx:7-36、footer-tab-control.tsx:35
            ├─ WelcomeScreen（无项目或 showWelcomeControl 时替代整个工作区）  main-layout.tsx:356-358 → welcome-screen.tsx
            ├─ PendingBufferCloseDialog              main-layout.tsx:360
            └─ 全局浮层（QuickOpen / CommandPalette / ProjectNameMenu / ConnectionDialog / … / TerminalHost）  main-layout.tsx:363-378
```

### 1.2 关键外壳组件与文件

| 组件 | 文件 | 角色 |
| --- | --- | --- |
| `MainLayout` | `windows/tauri/src/features/layout/components/main-layout.tsx:81` | 外壳总装 |
| `TitleBar` / `TitleBarWithSettings` | `windows/tauri/src/features/window/components/title-bar/title-bar.tsx:54` / `:363` | 标题栏（含窗口控件/项目切换/分支/更新） |
| `WindowMenuBar` | `windows/tauri/src/features/window/components/window-menu-bar.tsx:36` | Windows 自绘菜单栏 |
| `TitleProjectMenu` | `windows/tauri/src/features/window/components/title-bar/title-project-menu.tsx:109` | 标题栏项目下拉 |
| `WindowControls` | `windows/tauri/src/features/window/components/title-bar/window-controls.tsx:21` | 三键窗口控件 |
| `ProjectTabBar` | `windows/tauri/src/features/window/components/project-tab-bar.tsx:16` | 项目标签条（Windows 有） |
| `SidebarActivityRail` / `MainSidebar` | `windows/tauri/src/features/layout/components/sidebar/main-sidebar.tsx:120` / `:767` | 左侧活动栏（图标竖条）+ 侧栏内容 |
| `SidebarPaneSelector` | `windows/tauri/src/features/layout/components/sidebar/sidebar-pane-selector.tsx:81` | 活动栏图标项（vertical 形态） |
| `PluginActivityRail` | `windows/tauri/src/features/layout/components/plugin-activity-rail.tsx:11` | 右侧图标竖条 |
| `ResizablePane` | `windows/tauri/src/features/layout/components/resizable-pane.tsx:32` | 左右可拖拽面板 |
| `Footer` | `windows/tauri/src/features/layout/components/footer/footer.tsx:16` | 状态栏容器 |
| `ChromeBar` / `ChromeGroup` / `ChromeLabel` / `ChromeSeparator` | `windows/tauri/src/ui/chrome.tsx:75` / `:92` / `:108` / `:122` | 标题栏/状态栏共用 chrome 原语 |
| `InitialWindowShell` | `windows/tauri/src/App.tsx:39` | 首帧占位外壳（避免白屏） |

**已确认的死代码（定义但无任何引用）**：`useFooterDebuggerItem`（`windows/tauri/src/features/layout/components/footer/footer-debugger-item.tsx:29`）与 `FooterControlBadge`（`windows/tauri/src/features/layout/components/footer/footer-tab-control.tsx:23`）——全仓库 grep 仅命中定义处，`Footer`（`footer.tsx:24-42`）只装配 `filePath` 与 editor status 项。重写时**不必**实现调试器页脚控件。

---

## 2. 度量表

### 2.1 全局尺寸令牌（真源，全部来自 `theme.css`）

| 令牌 | 默认值 | px | comfortable 密度 | 证据 |
| --- | --- | --- | --- | --- |
| `--lithe-title-bar-height` | `2.5rem` | **40** | 不变 | `theme.css:118` |
| `--lithe-footer-height` | `1.5rem` | **24** | `2rem` = 32 | `theme.css:119`、`:191` |
| `--lithe-pane-header-height` | `2.25rem` | 36 | 不变 | `theme.css:120` |
| `--lithe-tab-bar-height` | `= pane-header` | 36 | 不变 | `theme.css:121` |
| `--lithe-tab-height` | `1.75rem` | **28** | `2rem` = 32 | `theme.css:122`、`:192` |
| `--lithe-tab-max-width` | `12.5rem` | 200 | 不变 | `theme.css:123` |
| `--lithe-sidebar-header-height` | `2rem` | 32 | `2.25rem` = 36 | `theme.css:124`、`:193` |
| `--lithe-workbench-gap` | `4px` | **4** | 不变 | `theme.css:125` |
| `--lithe-chrome-control-height` | `1.5rem` | **24** | `1.75rem` = 28 | `theme.css:126`、`:194` |
| `--lithe-chrome-hit-target` | `1.75rem` | 28 | `2rem` = 32 | `theme.css:127`、`:195` |
| `--lithe-chrome-line-height` | `1rem` | 16 | 不变 | `theme.css:128` |
| `--lithe-chrome-gap-tight` | `2px` | **2** | `4px` | `theme.css:129`、`:196` |
| `--lithe-chrome-gap` | `4px` | **4** | `6px` | `theme.css:130`、`:197` |
| `--lithe-chrome-gap-loose` | `6px` | **6** | `8px` | `theme.css:131`、`:198` |
| `--lithe-chrome-padding-inline` | `8px` | **8** | `10px` | `theme.css:132`、`:199` |
| `--lithe-chrome-radius` | `4px` | **4** | 不变 | `theme.css:133` |
| `--radius`（圆角基准） | `8px` | **8** | 不变 | `theme.css:134` |

| 字号令牌 | 默认 | comfortable | 证据 |
| --- | --- | --- | --- |
| `--app-ui-font-size`（=`--ui-text-sm`=`--ui-text-base`） | 13px | 13px（由设置改写） | `theme.css:112`、`:116-117` |
| `--ui-text-chrome` | 13px | 14px | `theme.css:115`、`:190` |
| `--ui-text-caption` | 12px | 13px | `theme.css:114`、`:189` |
| `--app-font-family` | `"Microsoft YaHei UI","Segoe UI",…` | — | `theme.css:106-108` |
| `--editor-font-family` | `"Geist Mono", ui-monospace,…` | — | `theme.css:109-111` |

| 派生圆角 | 计算式 | 实际 px | 证据 |
| --- | --- | --- | --- |
| `rounded-sm` | `calc(--radius * 0.6)` | **4.8** | `theme.css:6` |
| `rounded-md` | `calc(--radius * 0.8)` | **6.4** | `theme.css:7` |
| `rounded-lg` | `= --radius` | 8 | `theme.css:8` |
| `rounded-xl` | `calc(--radius * 1.4)` | **11.2** | `theme.css:9` |

| 动效令牌 | 值 | 证据 |
| --- | --- | --- |
| `--app-duration-fast` | 150ms | `theme.css:135` |
| `--app-duration-normal` | 200ms | `theme.css:136` |
| `--app-ease-smooth` | `cubic-bezier(0.22, 1, 0.36, 1)` | `theme.css:137` |
| `--app-ease-in-out` | `cubic-bezier(0.66, 0, 0.34, 1)` | `theme.css:138` |
| `--app-press-scale` | 1（即关闭按压缩放） | `theme.css:139` |
| 减少动效 | `html[data-reduce-motion="true"]` 把过渡压到 1ms | `utilities.css:110-129` |

### 2.2 外壳根与背景

| 元素 | Tailwind / 变量 | px / 值 | 证据 |
| --- | --- | --- | --- |
| 首帧外壳根 | `h-dvh w-dvw overflow-hidden bg-surface` | 全窗口 | `App.tsx:49` |
| 首帧拖拽条 | `h-10 w-full bg-surface/70` | **40 高** | `App.tsx:50` |
| 首帧主区 | `h-[calc(100dvh-2.5rem)] bg-background` | 窗口高 − 40 | `App.tsx:51` |
| 窗口容器 | `.window-container flex size-full flex-col overflow-hidden bg-background` | — | `workbench-app.tsx:84` |
| 外壳根 | `lithe-layout-shell flex size-full flex-col overflow-hidden bg-surface` | — | `main-layout.tsx:280` |
| 外壳背景（透明模式） | `--lithe-glass-shell-bg`：深色 `color-mix(surface 12%, transparent)`；浅色 `surface 42%` | 覆盖 `bg-surface` | `window-transparency.css:6`、`:24`、`:44-48` |
| 工作区容器 | `lithe-workbench-glass … flex-1 flex-col` | — | `main-layout.tsx:297` |
| 工作区行 | `flex flex-1 flex-row overflow-hidden pr-(--lithe-workbench-gap)` | 右侧留 **4px** 间隙 | `main-layout.tsx:299` |
| 编辑器岛 | `lithe-glass-island … rounded-xl border-border border-l bg-background` | 圆角 **11.2**、1px 左边框 | `main-layout.tsx:313` |
| 面板岛（可拖拽面板内容） | `lithe-glass-island … rounded-xl border-border border-x` | 圆角 11.2、1px 左右边框 | `resizable-pane.tsx:203-206` |
| 面板岛（right + outerEdge=false） | 追加 `rounded-r-none border-r-0` | 右侧不圆角、无右边框 | `resizable-pane.tsx:205` |
| 拖放遮罩 | `absolute inset-0 z-50 bg-background/90 backdrop-blur-sm`；内框 `rounded-xl border-2 border-primary border-dashed bg-surface px-8 py-6` | 内边距 32/24 | `main-layout.tsx:283-288` |

### 2.3 标题栏（`TitleBar`，Windows 分支）

| 元素 | Tailwind / 变量 | px / 值 | 证据 |
| --- | --- | --- | --- |
| 标题栏容器 | `lithe-title-bar font-sans ui-text-chrome relative z-50 flex h-(--lithe-title-bar-height) items-center justify-between gap-(--lithe-chrome-gap) bg-surface px-(--lithe-chrome-padding-inline) text-muted-foreground` | 高 **40**、gap 4、左右内边距 8、字号 13、文字 `--muted-foreground` | `title-bar.tsx:337` |
| 标题栏背景（透明模式覆盖） | `.lithe-title-bar { background: var(--lithe-glass-chrome-overlay-bg) !important; border-color: var(--lithe-glass-chrome-border) }` | 深色 `surface 6%`；浅色 `background 10%` | `window-transparency.css:64-72`、`:13`、`:31` |
| ChromeBar 基线（region=title） | `h-(--lithe-title-bar-height) gap-(--lithe-chrome-gap) bg-transparent px-(--lithe-chrome-padding-inline)`，且默认 `text-subtle-foreground` | 同上 | `ui/chrome.tsx:6`、`:10-11` |
| 极简标题栏（showMinimal） | `lithe-title-bar relative z-50 justify-between select-none` | — | `title-bar.tsx:292` |
| 左侧组 | `ChromeGroup grow min-w-0` → 内层 `pointer-events-auto min-w-0` | — | `title-bar.tsx:339-344` |
| 菜单栏根（非 compact） | `flex h-6 items-center gap-0.5 rounded-full border border-border/70 bg-background/65 px-0.5 py-0.5` | 高 **24**、gap 2、px 2、py 2、全圆角、1px 边框 | `ui/menubar.tsx:36` |
| 菜单栏项（MenubarTrigger） | `ui-text-sm flex h-5 … rounded-md px-1.5 text-subtle-foreground hover:bg-accent/50 hover:text-foreground` | 高 **20**、px 6、圆角 6.4、字号 13 | `ui/menubar.tsx:86` |
| 菜单栏（compact 浮动） | `absolute top-full left-0 mt-1 w-max` + `h-auto w-max flex-nowrap rounded-2xl border border-border bg-background/95 px-1 py-1 shadow-(--shadow-popover) backdrop-blur-sm` | 圆角 `--radius-2xl`=**14.4**、内边距 4 | `window-menu-bar.tsx:539`、`:549`；`theme.css:10` |
| 菜单栏（compact 内嵌） | `absolute inset-0` + `h-full rounded-none border-none bg-transparent px-2 py-0` | — | `window-menu-bar.tsx:540`、`:552` |
| 菜单面板 | `z-10031 w-max min-w-60 max-w-[min(480px,calc(100vw-16px))] rounded-xl border border-border bg-surface/95 p-1 shadow-(--shadow-popover) backdrop-blur-sm` | min-w **240**、p 4、圆角 11.2 | `ui/menubar.tsx:126` |
| 菜单条目 | `min-h-7 … gap-6 rounded-lg px-2.5 py-1.5` | min-h **28**、px 10、py 6、圆角 8 | `ui/menubar.tsx:147` |
| 菜单快捷键文本 | `font-mono ml-auto shrink-0 text-subtle-foreground/75 ui-text-sm` | 字号 13 | `ui/menubar.tsx:176` |
| 项目切换触发器 | `Button variant=ghost size=xs` + `max-w-56 justify-start gap-1.5 px-2` | 高 **24**、最大宽 **224**、gap 6、px 8 | `title-project-menu.tsx:146` |
| 触发器中 Logo 容器 | `grid size-5 shrink-0 place-items-center overflow-hidden rounded-md`；`img.size-5 scale-[1.19]` | **20×20**，圆角 6.4 | `title-project-menu.tsx:153-155` |
| 触发器名义标签 | `min-w-0 truncate`，文案取 `getProjectDisplayLabel(activeProject)` 或 `t("projectOpen.title")` | — | `title-project-menu.tsx:157`、`:118-120` |
| 触发器折叠箭头 | `size-3.5 shrink-0 text-subtle-foreground transition-transform`，打开时 `rotate-180` | **14×14** | `title-project-menu.tsx:158-163` |
| 项目下拉面板 | `max-h-[min(32.5rem,calc(100vh-3rem))] w-96 max-w-[calc(100vw-1rem)] overflow-y-auto rounded-md p-1.5` | 最大高 **520**、宽 **384**、p 6、圆角 6.4 | `title-project-menu.tsx:170` |
| 下拉动作条目 | `min-h-8 justify-start gap-2 rounded-md px-2` | min-h **32**、gap 8、px 8 | `title-project-menu.tsx:174`、`:181`、`:188` |
| 项目行 | `min-h-11 w-full items-center gap-2.5 rounded-md px-2 py-1.5`，选中 `bg-selected text-foreground` | min-h **44**、gap 10 | `title-project-menu.tsx:95-96` |
| 项目行徽标 | 图标：`shrink-0 rounded-md object-contain size-7`；无图标：`grid … rounded-md font-bold text-[10px] text-white size-7` | **28×28**、字号 **10**、圆角 6.4 | `title-project-menu.tsx:52`、`:62-64` |
| 项目行主/次文本 | 名称 `font-medium text-foreground ui-text-sm`；路径 `text-subtle-foreground ui-text-xs` | 13 / 12 | `title-project-menu.tsx:101-102` |
| 分组标题 | `px-2 pt-1.5 pb-1 font-normal ui-text-xs` | pt 6 / pb 4 | `title-project-menu.tsx:196`、`:218` |
| 快速打开按钮 | `Button variant=ghost size=icon-xs tooltip=t("workbench.search")` | **24×24** | `title-bar.tsx:242-254` |
| 更新控件（有更新时） | 外层 `ml-3 flex items-center`；`ButtonGroup variant=accent`；主按钮 `size=xs`；箭头 `size=icon-xs` | 左距 **12**、主按钮高 24、箭头 24 | `app-update-control.tsx:126`、`:135-137`、`:144-147`、`:172-175` |
| 更新控件（空闲态，仅欢迎页用） | `Button variant=ghost size=xs className="… font-medium text-subtle-foreground"` | 高 24 | `app-update-control.tsx:85-93` |
| 窗口控件组 | `ChromeGroup gap={IS_WINDOWS ? "none" : "tight"}`，Windows 追加 `h-(--lithe-title-bar-height)` | Windows gap **0**、高 40 | `window-controls.tsx:52-54` |
| 最小化 / 最大化按钮 | `variant=ghost size=icon-xs` + `pointer-events-auto h-full w-14 min-w-14 rounded-none` | **56×40**、直角 | `window-controls.tsx:59`、`:71` |
| 关闭按钮 | `variant=danger size=icon-xs` + `group hover:text-white h-full w-14 min-w-14 rounded-none hover:bg-destructive` | **56×40**；悬停背景 `--destructive`、图标变白 | `window-controls.tsx:83-86` |
| 窗口控件图标 | Button 基类 `[&_svg:not([class*='size-'])]:size-3.5` | **14×14** | `ui/button.tsx:9` |
| 标题栏右键菜单 | `ContextMenuContent`（默认度量），条目含新建窗口/添加项目/打开文件夹/在新窗口打开文件夹/关闭所有项目 | — | `title-bar.tsx:175-203` |

### 2.4 项目标签条（`ProjectTabBar`）

| 元素 | Tailwind / 变量 | px / 值 | 证据 |
| --- | --- | --- | --- |
| 容器 | `flex h-8 shrink-0 items-center overflow-x-auto border-border border-b bg-surface px-1.5` | 高 **32**、px 6、1px 下边框、横向滚动 | `project-tab-bar.tsx:41` |
| 内层 | `flex min-w-max items-center gap-1` | gap **4** | `project-tab-bar.tsx:46` |
| 标签 | `relative flex h-7 min-w-36 max-w-60 items-center gap-1.5 rounded-sm border py-0 pr-8 pl-2.5 text-left ui-text-chrome` | 高 **28**、min-w **144**、max-w **240**、gap 6、pr **32**、pl **10**、圆角 **4.8** | `project-tab-bar.tsx:64` |
| 标签·选中 | `border-transparent bg-accent text-foreground` | 背景 `--accent` | `project-tab-bar.tsx:66` |
| 标签·未选中 | `border-transparent text-subtle-foreground hover:bg-accent/70 hover:text-foreground` | — | `project-tab-bar.tsx:67` |
| 标签·聚焦 | `focus-visible:ring-2 focus-visible:ring-primary/30` | 2px 环 | `project-tab-bar.tsx:64` |
| 标签·禁用 | `disabled:cursor-not-allowed` | — | `project-tab-bar.tsx:64` |
| 文件夹图标 | `size-3.5 shrink-0`；选中 `text-primary`，未选中 `text-subtle-foreground` | **14×14** | `project-tab-bar.tsx:70-75` |
| 选中下划线 | `absolute inset-x-1.5 bottom-0 h-0.5 rounded-t-sm bg-primary` | 高 **2**、左右内缩 6、圆角 4.8/上 | `project-tab-bar.tsx:79-82` |
| 关闭按钮容器 | `absolute inset-y-0 right-1 z-10 flex items-center transition-opacity`；非选中态 `opacity-0`，`group-hover`/`group-focus-within` 时 `opacity-100` | right 4 | `project-tab-bar.tsx:86-91` |
| 关闭按钮 | `Button size=icon-xs variant=ghost` + tooltip `titleProject.closeProject` | **24×24** | `project-tab-bar.tsx:93-104` |
| 显示条件 | `shouldShowProjectTabBar(count, hideWhenSingle)`：`count > 0 && (!hideWhenSingle \|\| count > 1)`；`MainLayout` 传 `hideWhenSingle` | 单项目时隐藏 | `project-tab-bar-model.ts:12-13`、`main-layout.tsx:293` |

### 2.5 活动栏（左侧图标竖条 = `SidebarActivityRail`）

| 元素 | Tailwind / 变量 | px / 值 | 证据 |
| --- | --- | --- | --- |
| 折叠宽常量 | `COLLAPSED_ACTIVITY_RAIL_WIDTH = 38` | **38** | `main-sidebar.tsx:97` |
| 展开宽区间 | 默认 160、min 140、max 320 | 140–320 | `main-sidebar.tsx:98-100` |
| 水平内衬常量 | `ACTIVITY_RAIL_HORIZONTAL_GUTTER = 8` | **8** | `main-sidebar.tsx:101` |
| rail 容器 | `lithe-sidebar-rail relative flex h-full shrink-0 overflow-hidden`，宽度 `calc(<railPanelWidth>px + var(--lithe-workbench-gap))` | 折叠 **38 + 4 = 42** | `main-sidebar.tsx:669-672`、`:588-590` |
| rail 内容层 | `absolute inset-y-0 left-0 shrink-0 will-change-transform` | — | `main-sidebar.tsx:674-684` |
| 项目面板 | `absolute inset-y-0 left-0 flex w-full flex-col items-start gap-1 overflow-hidden pt-1.5`；`pb-7`（展开+轮播+项目圆点）或 `pb-1.5` | pt **6**、gap **4**、pb **28** 或 **6** | `main-sidebar.tsx:603-606` |
| 项目面板内衬 | 内联 `paddingLeft/Right: 8` | 左右各 **8** | `main-sidebar.tsx:610-611` |
| 图标项（垂直） | `SidebarListItem` + `className="ui-text-sm min-h-6 py-1"`；组件基线 `min-h-(--lithe-tab-height) … rounded-(--lithe-chrome-radius) px-2 py-1` | 覆盖后 min-h **24**、py 4；基线 px 8、圆角 **4** | `main-sidebar.tsx:332`、`ui/sidebar.tsx:240` |
| 图标项·iconOnly | `iconOnly && "justify-center gap-0 px-0"` | px **0** | `ui/sidebar.tsx:245` |
| 图标项·选中（iconOnly） | `active && iconOnly && "bg-accent text-foreground"` | 背景 `--accent` | `ui/sidebar.tsx:244` |
| 图标项·未选中 | `text-subtle-foreground`；`hover:bg-accent/70 hover:text-foreground` | — | `ui/sidebar.tsx:240-241` |
| 图标项·禁用 | `disabled:opacity-50 disabled:cursor-not-allowed` | — | `ui/sidebar.tsx:242` |
| 图标尺寸（垂直） | `iconClassName = compact \|\| isVertical ? "size-4" : undefined` | **16×16**（compact 时 14） | `sidebar-pane-selector.tsx:107` |
| 垂直导航容器 | `flex h-full w-full flex-col`；上组 `flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto`；下组 `flex shrink-0 flex-col gap-1 pt-1` | gap **4**、下组 pt **4** | `sidebar-pane-selector.tsx:358-366` |
| rail 拖拽热区（仅展开） | `group absolute top-0 right-0 z-20 flex h-full w-(--lithe-workbench-gap) cursor-col-resize items-center justify-center hover:bg-primary/8`；内部 `h-full w-px bg-transparent group-hover:bg-primary` | 宽 **4**、线宽 **1**、悬停 `primary/8` | `main-sidebar.tsx:702-705` |
| rail 拖拽遮罩（拖拽中） | `fixed inset-0 z-40 cursor-col-resize` | — | `main-sidebar.tsx:708` |
| 右键菜单面板 | `min-w-56`（子菜单同） | min-w **224** | `main-sidebar.tsx:710`、`:728` |
| 项目圆点容器 | `absolute right-(--lithe-workbench-gap) bottom-1.5 left-0 z-20 flex items-center justify-center overflow-x-auto px-2` | right **4**、bottom **6**、px 8 | `sidebar-projects.tsx:57` |
| 项目圆点热区 | `size-4 shrink-0 rounded-full` | **16×16** | `sidebar-projects.tsx:69` |
| 项目圆点本体 | `size-1.5 rounded-full bg-foreground`；选中 `scale-100 opacity-100`，未选中 `scale-75 opacity-25`，hover `scale-100 opacity-50` | **6×6** | `sidebar-projects.tsx:92-95` |
| 侧栏内容面板 | `SidebarPanel`：`flex h-full min-h-0 min-w-0 w-full flex-col bg-background` | — | `ui/sidebar.tsx:18` |

### 2.6 活动栏（右侧图标竖条 = `PluginActivityRail`）

| 元素 | Tailwind / 变量 | px / 值 | 证据 |
| --- | --- | --- | --- |
| 容器 | `lithe-plugin-activity-rail flex w-9.5 shrink-0 flex-col items-center rounded-r-xl border-border border-r bg-surface pt-1` | 宽 **38**、右上/右下圆角 **11.2**、1px 右边框、pt **4** | `plugin-activity-rail.tsx:34` |
| 扩展 / Maven 按钮 | `Button variant=ghost size=icon-sm className="rounded-sm"` | **28×28**、圆角 **4.8** | `plugin-activity-rail.tsx:39`、`:45`、`:58`、`:62` |
| 按钮图标 | `PuzzlePieceIcon / MavenIcon className="size-4.5"` | **18×18** | `plugin-activity-rail.tsx:48`、`:64` |
| 通知按钮 | `Button variant=ghost size=icon-sm active={isActive} className="relative rounded-sm"`，`BellIcon size-4.5` | **28×28** / 图标 18 | `notifications-trigger.tsx:30-43` |
| 未读徽标 | `absolute top-0 right-0 flex min-w-3 translate-x-0.5 -translate-y-0.5 items-center justify-center rounded-full bg-primary px-0.5 font-sans text-[9px] leading-3 text-primary-foreground tabular-nums` | min-w **12**、字号 **9**、行高 12、全圆角 | `notifications-trigger.tsx:45` |
| 未读上限显示 | `unreadCount > 9 ? "9+" : unreadCount` | — | `notifications-trigger.tsx:46` |

### 2.7 状态栏（`Footer`）

| 元素 | Tailwind / 变量 | px / 值 | 证据 |
| --- | --- | --- | --- |
| 容器 | `ChromeBar region="footer"` + `lithe-footer-bar relative z-20 justify-between gap-2 bg-surface` | 高 `--lithe-footer-height` = **24**、gap **8**、左右内边距 8 | `footer.tsx:45-49`、`ui/chrome.tsx:12-13` |
| 容器文字/字号 | `font-sans ui-text-chrome … text-subtle-foreground` | 13px | `ui/chrome.tsx:6` |
| 背景（透明模式覆盖） | `.lithe-footer-bar { background: var(--lithe-glass-chrome-overlay-bg) !important; border-color: var(--lithe-glass-chrome-border) }` | 同标题栏 | `window-transparency.css:64-72` |
| 隐藏态 | `html[data-status-bar="hidden"] { --lithe-footer-height: 0rem }` + React 层 `showStatusBar ? <Footer/> : null` | 高 **0** | `utilities.css:106-108`、`main-layout.tsx:354` |
| 左组 | `ChromeGroup gap="tight" grow className="min-w-0"` | gap **2** | `footer.tsx:50` |
| 路径槽 | `flex min-h-(--lithe-chrome-control-height) min-w-0 flex-1 items-center overflow-hidden` | min-h **24** | `footer.tsx:53` |
| 其它前导项槽 | `flex min-h-(--lithe-chrome-control-height) shrink-0 items-center` | min-h **24** | `footer.tsx:63` |
| 尾组 | `ChromeGroup gap="tight" align="end" className="shrink-0"`；项槽 `flex min-h-(--lithe-chrome-control-height) items-center` | gap **2**、min-h **24** | `footer.tsx:70-72` |
| 状态 chip（可点击） | `font-sans inline-flex h-(--lithe-chrome-control-height) max-w-50 shrink-0 items-center gap-1 rounded-md border-0 px-1.5 ui-text-chrome leading-none text-subtle-foreground hover:bg-accent hover:text-foreground` | 高 **24**、max-w **200**、gap 4、圆角 **6.4**、px 6 | `footer-status-chip.tsx:4-5` |
| 状态标签（不可点击） | 同上但 `hover:bg-transparent hover:text-subtle-foreground` | 高 24 | `footer-status-chip.tsx:26-29` |
| 光标位置 chip | `font-sans inline-flex h-5 items-center self-center rounded-full border-0 px-1.5 ui-text-sm leading-none text-subtle-foreground hover:bg-accent hover:text-foreground` | 高 **20**、全圆角、px 6、字号 13 | `editor-status-actions.tsx:9` |
| 文件路径面包屑列表 | `flex flex-wrap items-center gap-1.5 wrap-break-word text-subtle-foreground ui-text-sm`，此处 `flex-nowrap gap-0` | gap **0**、字号 13 | `ui/breadcrumb.tsx:18-20`、`path-breadcrumb.tsx:39` |
| 面包屑分段（可交互） | `Button variant=ghost size=xs` + `min-w-0 items-center gap-1 whitespace-nowrap`；末段 `font-medium text-foreground`，其余 `text-subtle-foreground` | 高 24、gap 4 | `path-breadcrumb.tsx:50-63` |
| 面包屑分段（纯展示） | `inline-flex items-center gap-1 truncate px-1.5` / `truncate px-1.5 text-subtle-foreground` | px 6 | `path-breadcrumb.tsx:73`、`:83` |
| 面包屑分隔符 | `text-subtle-foreground/70 [&>svg]:size-3.5` + `mx-0.5 shrink-0` | 图标 **14×14**、mx 2 | `ui/breadcrumb.tsx:75`、`path-breadcrumb.tsx:45` |
| 文件类型图标（末段前） | `size-3.5 shrink-0 text-subtle-foreground` | **14×14** | `file-path-breadcrumb.tsx:199` |
| 准备状态（compact） | `<details className="relative shrink-0 ui-text-sm">`；失败 `text-destructive`，其余 `text-subtle-foreground`；展开面板 `absolute bottom-full left-0 z-50 mb-2 w-80 rounded border border-border bg-background p-3 shadow-lg` | 面板宽 **320**、p **12**、mb **8**、圆角 `--radius-sm` 4.8 | `project-preparation-status.tsx:30-31`、`:36-38`、`:53` |
| Git 更改 chip | `FooterStatusChip`，0 更改时显示 `CheckCircleIcon className="text-success"`，否则显示计数文案 | 同 chip 度量 | `footer-editor-status.tsx:126-139` |
| 内存标签 | `FooterStatusLabel className="max-w-none whitespace-nowrap"` + `HardDrivesIcon` | 覆盖 max-w | `footer-editor-status.tsx:112` |
| 只读锁标签 | `FooterStatusLabel className="px-1"` | px **4** | `footer-editor-status.tsx:99` |
| 页脚项顺序 | 前导 `FOOTER_LEADING_ITEM_IDS = ["filePath","branch"]`（+`"debugger"`）；尾随 `FOOTER_TRAILING_ITEM_IDS = ["cursor","encoding","indent","readOnly","memory","gitChanges"]` | — | `config/item-order.ts:20-32` |
| 内存轮询周期 | `MEMORY_POLL_INTERVAL_MS = 10_000`，页面隐藏时跳过 | 10s | `footer-editor-status.tsx:24`、`:51-53` |

### 2.8 `ResizablePane`（左右面板）度量

| 元素 | Tailwind / 变量 | px / 值 | 证据 |
| --- | --- | --- | --- |
| 最小宽度 | `MIN_SIDEBAR_WIDTH = 140`；AI 聊天 300 / 窄窗 220 | 140 | `resizable-pane.tsx:18-20` |
| 全局最小 | `MIN_RESPONSIVE_PANE_WIDTH = 50` | 50 | `utils/resizable-pane-layout.ts:1` |
| 主内容保底 | `MIN_MAIN_CONTENT_WIDTH = 360` | 360 | `utils/resizable-pane-layout.ts:2` |
| 最大宽 | `max(50, viewportWidth − reservedWidth − 360)` | 动态 | `utils/resizable-pane-layout.ts:4-9` |
| 面板外框 | `lithe-resizable-pane relative flex h-full min-w-0 shrink-0 overflow-visible bg-transparent` | — | `resizable-pane.tsx:186` |
| 左面板外边距 | `mr-(--lithe-workbench-gap)` | **4** | `resizable-pane.tsx:188` |
| 右面板外边距 | `ml-(--lithe-workbench-gap)` | **4** | `resizable-pane.tsx:189` |
| 内容层 | `flex min-h-0 shrink-0 flex-col overflow-hidden py-0`，宽度 `hidden ? 0 : width` px | — | `resizable-pane.tsx:198-199` |
| 拖拽热区 | `group absolute top-0 z-30 flex h-full w-(--lithe-workbench-gap) cursor-col-resize items-center justify-center transition-colors duration-(--app-duration-fast) ease-(--app-ease-smooth) hover:bg-primary/8` | 宽 **4**、动效 **150ms** | `resizable-pane.tsx:160-163` |
| 拖拽视觉线 | `h-full w-px bg-transparent … group-hover:bg-primary`，拖拽中 `bg-primary` | 宽 **1**、颜色 `--primary` | `resizable-pane.tsx:173-176` |
| 热区偏移 | 左：`right: calc(var(--lithe-workbench-gap) * -1)`；右：`left: calc(...)` | 外移 **4** | `resizable-pane.tsx:155-159` |
| 拖拽遮罩 | `fixed inset-0 z-40 cursor-col-resize` | — | `resizable-pane.tsx:195` |

### 2.9 颜色键（`lithe.json` → CSS 变量，外壳实际用到）

| 语义 | CSS 变量 | Lithe Light | Lithe Dark | 证据 |
| --- | --- | --- | --- | --- |
| 编辑器区/通用底 | `--background` | `#ffffff` | `#1e1f22` | `lithe.json:12`、`:79` |
| 外壳/标签条/状态栏底 | `--surface` | `#f7f8fa` | `#2b2d30` | `lithe.json:13`、`:80` |
| 主前景 | `--foreground` | `#1f2328` | `#dfe1e5` | `lithe.json:14`、`:81` |
| 标题栏文字 | `--muted-foreground` | `#4f5965` | `#b4b8bf` | `lithe.json:15`、`:82` |
| chrome 默认文字 | `--subtle-foreground` | `#68717d` | `#8b929e` | `lithe.json:16`、`:83` |
| 边框 | `--border` | `#dfe1e5` | `#43454a` | `lithe.json:17`、`:84` |
| 悬停/选中底 | `--accent` | `#edf3ff` | `#393b40` | `lithe.json:18`、`:85` |
| 强选中底 | `--selected` | `#d4e2ff` | `#2e436e` | `lithe.json:19`、`:86` |
| 强调色（活动指示/焦点环） | `--primary` | `#3574f0` | `#3574f0` | `lithe.json:21`、`:88` |
| 危险（关闭按钮悬停） | `--destructive` | `#cf3f4f` | `#db5c5c` | `lithe.json:25`、`:92` |
| 成功（无更改勾） | `--success` | `#27864f` | `#57965c` | `lithe.json:26`、`:93` |
| 文本选中 | `--selection` | `rgba(53,116,240,0.2)` | `#214283` | `lithe.json:20`、`:87` |

---

## 3. 状态与交互

### 3.1 悬停 / 选中 / 禁用 / 折叠 / 隐藏

| 交互 | 行为 | 证据 |
| --- | --- | --- |
| 图标按钮（chrome 通用） | `variant=ghost`：默认 `text-subtle-foreground`；`hover:bg-accent hover:text-foreground`；`data-[active=true]:bg-accent data-[active=true]:text-foreground`；`disabled:opacity-50 disabled:pointer-events-none` | `ui/button.tsx:17`、`:9` |
| 危险按钮（关闭） | `variant=danger`：`hover:bg-destructive/10 hover:text-destructive`；Windows 标题栏覆盖为 `hover:bg-destructive hover:text-white` | `ui/button.tsx:19`、`window-controls.tsx:84-86` |
| 透明模式下的 chrome 悬停 | `.lithe-chrome-control:hover { background: var(--lithe-chrome-control-hover-bg); color: var(--lithe-chrome-icon-hover-color) }`，active 用 `[data-active="true"]` | `window-transparency.css:108-123` |
| 活动栏图标选中态 | 主侧栏项 `bg-selected text-foreground`（有标签）/ `bg-accent text-foreground`（iconOnly） | `ui/sidebar.tsx:243-244` |
| 活动栏点击语义 | `resolveSidebarPaneClick`：不可见→显示该视图；已可见且点同一项→**折叠**；点其它项→切视图；`edge` 级视图（outline/databases/notifications）改走右面板 | `utils/sidebar-pane-utils.ts:47-71`、`hooks/use-sidebar-pane-controller.ts:26-48`、`:26-34` |
| 项目圆点 | 未选中 `opacity-25 scale-75`，`group-hover` 变 `opacity-50 scale-100`；切换中 `cursor-default` 且 `aria-disabled` | `sidebar-projects.tsx:92-95`、`:70`、`:78` |
| 项目标签关闭按钮 | 选中：常显 `opacity-100`；未选中：`opacity-0` + `pointer-events-none`，`group-hover` / `group-focus-within` 显示 | `project-tab-bar.tsx:88-91` |
| 标签条禁用 | 切换或关闭进行中 `isProjectActionPending` → `disabled` | `project-tab-bar.tsx:24`、`:57`、`:99` |
| 状态栏隐藏 | 设置 `showStatusBar=false` → React 不渲染 `Footer`，且 `--lithe-footer-height: 0rem` | `main-layout.tsx:94`、`:354`；`utilities.css:106-108` |
| 侧边栏隐藏 | `isSidebarVisible=false` → `ResizablePane hidden`：宽度 0、`pointer-events-none`、`aria-hidden`、不渲染拖拽热区、内容宽 0 | `resizable-pane.tsx:150`、`:186-193`、`:198`、`:151` |
| 右侧工具窗显示 | 仅当 `isRightSidebarVisible && activeRightSidebarView ∈ {notifications, maven}` | `main-layout.tsx:101-105`、`:325-341` |
| 活动栏折叠/展开 | `MainLayout` 固定传 `expanded={false}`（38px 折叠态）；`expanded=true` 时才渲染拖拽热区与项目圆点 | `main-layout.tsx:302`、`main-sidebar.tsx:354`、`:689`、`:697` |
| 更新控件显示 | `getUpdateControlVisibility(rootFolderPath)`：有项目 → 标题栏显示；无项目 → 欢迎页显示 | `utils/update-control-visibility.ts:6-14`、`main-layout.tsx:112-113` |
| 工具提示 | 标题栏按钮 `tooltipSide="bottom"`；活动栏（垂直/左）按 `tooltipSide = isVertical ? "right" : "bottom"`；右侧 rail 用 `tooltipSide="left"` | `title-bar.tsx:247`；`sidebar-pane-selector.tsx:106`；`plugin-activity-rail.tsx:41` |
| 系统菜单栏 | Windows **不使用**系统菜单栏：`shouldUseNativeMenuBar = !isWindows && !isLinux && nativeMenuBar` | `title-bar.tsx:79`、`:205-233` |
| 窗口控件可见性 | `showAppWindowControls = !isMacOS && !usesNativeWindowChrome`；Windows 恒为真 | `title-bar.tsx:78`、`use-native-window-chrome.ts:6-12` |
| 减少动效 | `data-reduce-motion` + `prefers-reduced-motion` 双通道压平过渡/动画 | `utilities.css:110-152`；`app-update-control` 以外的动效见 `workbench-app.tsx:77` |

### 3.2 快捷键（外壳直接相关）

| 快捷键 | 动作 | 证据 |
| --- | --- | --- |
| `Mod+B` | 切换活动侧栏（`menu_toggle_activity_sidebar`） | `window-menu-bar.tsx:280-283` |
| `Mod+E` | 切换辅助/主侧栏（`menu_toggle_sidebar`） | `window-menu-bar.tsx:285-287` |
| `Mod+J` | 切换终端 | `window-menu-bar.tsx:288-290` |
| `Mod+Shift+N` | 新建窗口 | `window-menu-bar.tsx:138-140` |
| `Mod+Shift+W` | 关闭窗口（命令 `workbench.closeWindow`） | `window-menu-bar.tsx:170-175` |
| `Alt+M` | 切换菜单栏（非 Linux） | `window-menu-bar.tsx:485-487` |
| `Alt+F9` / `Alt+F10` / `F11` | 最小化 / 最大化 / 全屏 | `window-menu-bar.tsx:464-501` |
| `Mod+Alt+I` | Web 检查器 | `window-menu-bar.tsx:450-452` |
| 活动栏项 tooltip 内标注 | 文件 `Mod+Shift+E`、搜索 `Mod+Shift+F`、更改 `Mod+Shift+G`、提交记录 `Alt+9`、终端 `Mod+J`、诊断 `Mod+Shift+J`、运行 `Shift+F10` | `sidebar-pane-selector.tsx:131`、`:145`、`:162`、`:179`、`:199`、`:216`、`:234` |
| chrome 键盘导航 | `Home/End/ArrowUp/ArrowDown`（垂直）与 `ArrowLeft/ArrowRight`（水平）索引计算 | `utils/chrome-keyboard.ts:3-21` |
| 菜单命令派发 | 菜单项 emit `menu_execute_command`，payload 为命令 ID（如 `workbench.showDebugger`），由 `use-menu-events` 监听后执行 | `window-menu-bar.tsx:119-124`、`use-menu-events.ts:59-61` |
| 打开快捷列表 | Tauri 事件 `menu_*`（完整 31 个事件名） | `use-menu-events.ts:30-70` |

### 3.3 右键菜单

| 位置 | 内容 | 证据 |
| --- | --- | --- |
| 标题栏空白区 | 新建窗口 / 添加项目 / 打开文件夹 / 在新窗口中打开文件夹 / ─ / 关闭所有项目（有项目时） | `title-bar.tsx:175-203` |
| 标题栏：命中交互元素时 | `e.preventDefault()` 并**不**弹标题栏菜单 | `title-bar.tsx:127-137` |
| 活动栏（整条 rail） | 分组“操作”：打开项目、搜索；─；“在活动侧栏中显示”子菜单（9 个可勾选项 + “项目圆点” + 条件出现的 `Show All`） | `main-sidebar.tsx:710-762` |
| 活动栏子菜单可勾选项来源 | `sidebarActivityVisibilityItemIds(coreFeatures)` 过滤：search/git/gitLog/terminal/diagnostics 受 `coreFeatures` 控制 | `config/item-order.ts:41-51`、`main-sidebar.tsx:188-217` |
| 项目圆点 | 切换到项目（当前项禁用）/ 复制路径 / 在资源管理器中显示（非 remote）/ 在新窗口中打开 / 选择图标（非 remote）/ 设置显示别名 / ─ / 移除项目（destructive） | `sidebar-projects.tsx:99-157` |
| 更新控件 | 查看发布说明 / 稍后提醒 / 跳过此版本 | `app-update-control.tsx:193-200`、`:52-79` |
| 页脚调试控件（死代码） | 显示/隐藏运行和调试、清空断点/监视/控制台、左移/右移/重置顺序 | `footer-debugger-item.tsx:76-147`（**未被 Footer 引用**） |
| 圆点子菜单定位 | `ContextMenuContent side="top" sideOffset={6} align="center"` | `sidebar-projects.tsx:99` |

### 3.4 拖拽调整（`ResizablePane` / 活动栏）

| 项 | 行为 | 证据 |
| --- | --- | --- |
| 左/右面板拖拽 | `mousedown` → `setIsResizing(true)`；`position==="right"` 时 `deltaX = startX − clientX`，否则 `clientX − startX` | `resizable-pane.tsx:102-118` |
| 帧节流 | 每次 `mousemove` 先 `cancelAnimationFrame` 再 `requestAnimationFrame` 写 `style.width` | `resizable-pane.tsx:120-128` |
| 提交 | `mouseup` → `setWidth`、`updateSetting(widthKey, currentWidth)`（持久化） | `resizable-pane.tsx:131-140` |
| 全局光标/选区锁定 | `document.body.style.cursor = "col-resize"`、`userSelect = "none"`，结束后清空 | `resizable-pane.tsx:144-145`、`:138-139` |
| 窗口尺寸变化重算 | 监听 `window.resize`，用设置的**存储值**重新 clamp | `resizable-pane.tsx:82-91` |
| 设置值变化重算 | `useEffect` 依赖 `storedWidth`，每次 clamp | `resizable-pane.tsx:76-80` |
| 滚轮透传 | 热区上滚轮绑定到侧栏滚动容器：`bindOverlayWheelToScrollContainer` | `resizable-pane.tsx:93-100`、`:5-8` |
| 活动栏拖拽（仅 expanded） | 与面板同构；预览用 `previewActivityRailWidth`，提交时 `updateSetting("activityRailWidth", nextWidth)`；宽度公式 `calc(Npx + var(--lithe-workbench-gap))` | `main-sidebar.tsx:276-295`、`:297-352` |
| 活动栏 clamp | `clampActivityRailWidth`：`min(320, max(140, round(width)))` | `main-sidebar.tsx:110-111` |
| 活动栏项目轮播（横向滚轮/滑动） | `handleProjectWheel`：仅取 `|deltaX| > |deltaY|`；位移上限 `railWidth * 0.96`；超过 `railWidth * 0.82` 立即提交切换，否则 40ms 后按 `42px` 阈值判定 | `main-sidebar.tsx:530-586`、`:102-104` |
| 轮播动画 | `PROJECT_SNAP_TRANSITION = { type: "tween", ease: [0.2,0.8,0.2,1] }`；`prefersReducedMotion` 时直接 jump | `main-sidebar.tsx:105-108`、`:426-434` |

### 3.5 窗口尺寸与拖拽策略

| 项 | 值 | 证据 |
| --- | --- | --- |
| 默认尺寸 | 1200 × 800 | `windows/tauri/src-tauri/tauri.windows.conf.json:8-9` |
| 最小尺寸 | 720 × 480 | `tauri.windows.conf.json:10-11` |
| 装饰 | `decorations: false`、`hiddenTitle: true`、`transparent: false` | `tauri.windows.conf.json:12-14` |
| 可缩放 / 居中 | `resizable: true`、`center: true` | `tauri.windows.conf.json:15-16` |
| 窗口标题 / label | `"Lithe"` / `"main"` | `tauri.windows.conf.json:6-7` |
| 基座配置（prod） | `productName: Lithe`、`identifier: app.lithe.windows`、`version: 0.3.0` | `tauri.conf.json:3-5` |
| 窗口状态标志 | `StateFlags::all()` 去掉 `DECORATIONS` | `src-tauri/src/main.rs:205-209` |
| 标题栏拖拽 | `mousedown` 命中非交互元素（`button, a, input, textarea, select, [role='tab'], [contenteditable='true']` 之外）时 `startDragging()` | `utils/title-bar-drag.ts:8-22`、`title-bar.tsx:139-145` |
| 首帧占位拖拽 | 同样调用 `getCurrentWindow().startDragging()` | `App.tsx:40-46` |
| 自绘缩放热区（Windows 必开） | 8 个 `position: fixed` 热区；边 **5px**、角 **10px**（Linux 为 8/16）；`z-index: 100000` | `window-resize-border.tsx:23-26`、`:173-207`、`:211` |
| 缩放触发 | `window.startResizeDragging(direction)`，方向含 8 向 | `window-resize-border.tsx:114-121`、`:6-14` |
| 缩放期间选区锁 | 加 `data-window-resize-dragging`，并设 4000ms 兜底超时 | `window-resize-border.tsx:25`、`:94-112`；`utilities.css:23-28` |
| 窗口状态同步 | `title-bar.tsx` 订阅 `window.onResized` + `onFocusChanged` 更新 `isMaximized/isFullscreen` | `title-bar.tsx:86-104` |
| 菜单栏窗口置顶 | Windows/Linux 打开菜单时临时 `setAlwaysOnTop(true)`，关闭后还原 | `window-menu-bar.tsx:46`、`:69-103` |
| 关闭流程 | 关闭按钮走 `requestWindowClose()`（带未保存守卫，`WindowCloseGuard`） | `window-controls.tsx:47-49`、`main-layout.tsx:374` |
| 多窗口 | 新建窗口 `createAppWindow()`（Tauri 命令 `create_app_window`） | `title-bar.tsx:177`、`feature/system`→`host::create_app_window`（`src-tauri/src/main.rs:175`） |

---

## 4. 数据与命令

### 4.1 命令调用链（关键区别）

前端 `invoke()` 是**统一分发层**：命中 `nativeCommands` 白名单才走真实 Tauri 命令，否则一律走通用命令 `platform_invoke`（Rust 侧再翻译成 Core 命令）。

| 层 | 证据 |
| --- | --- |
| 前端 `invoke` 实现 | `windows/tauri/src/platform/tauri-core.ts:93-128` |
| 真实原生命令白名单 | `windows/tauri/src/platform/tauri-core.ts:18-91` |
| 非白名单 → `platform_invoke`（Git 额外带 Channel 事件） | `tauri-core.ts:109-127` |
| Rust 侧注册 `platform::platform_invoke` | `windows/tauri/src-tauri/src/main.rs:128` |
| Git 命令名翻译：`git_status` → `git.status` | `windows/tauri/src-tauri/src/platform.rs:212`（测试 `:840-841`） |

### 4.2 外壳数据来源

| 外壳数据 | 前端来源 | 命令 / 契约 | 证据 |
| --- | --- | --- | --- |
| 状态栏内存（总计 / Lithe） | `readApplicationMemoryUsage()` | **原生 Tauri 命令 `get_application_memory_usage`**（返回 `{litheBytes,totalBytes}` camelCase）；已在白名单 | `services/memory-api.ts:11-13`、`tauri-core.ts:41`、`src-tauri/src/memory.rs:10-31`、`main.rs:129` |
| 内存轮询策略 | `ApplicationMemoryPoller`，10s 间隔、页面隐藏跳过、单飞请求 | — | `memory-api.ts:15-44`、`footer-editor-status.tsx:45-67` |
| 状态栏 Git 分支 / ahead / behind | `useFooterGitBranchItem` 读 `gitStatus` / `workspaceGitStatus` | `getGitStatus()` → `invoke("git_status")` → `platform_invoke` → Core `git.status` | `footer-git-branch-item.tsx:11-31`、`api/git-status-api.ts:63-64`、`platform.rs:212`、`shared/contracts/rust-core-api.md:172` |
| 状态栏更改计数 | `countUniqueGitChanges(gitFiles)` | 本地派生 | `footer-editor-status.tsx:42`、`utils/footer-status.ts` |
| Git 变更自动刷新 | `subscribeToGitChanges` + 300ms 防抖 → `refreshWorkspaceGitStatus` | 前端订阅 | `main-layout.tsx:256-277` |
| 状态栏光标位置 | `CursorPositionChip` ← `useEditorStateStore.activeEditorViewKey` | 本地 store | `footer-editor-status.tsx:30`、`:70-75` |
| 状态栏编码 | 常量 `TEXT_FILE_ENCODING` | 本地常量 | `footer-editor-status.tsx:16`、`:81` |
| 状态栏缩进 | `settings.tabSize` | 设置 store | `footer-editor-status.tsx:29`、`:89` |
| 状态栏文件路径 | `useBufferStore` 活动 buffer 的 `path`（`coreFeatures.breadcrumbs` 开启才显示） | 本地 store | `footer-file-path-item.tsx:12-25` |
| 面包屑目录下拉 | `readDirectory()` | WSL 走 `wsl_read_directory`；本地走插件 `readDir`（**不是** Tauri 自定义命令） | `file-path-breadcrumb.tsx:92`、`file-system/controllers/platform.ts:154-177` |
| 项目准备状态 | `useProjectPreparation(rootFolderPath)` | 本地 store（Core 事件驱动） | `project-preparation-status.tsx:11` |
| 项目标签条数据 | `useWorkspaceTabsStore.projectTabs`（持久化） | 本地 store | `project-tab-bar.tsx:19`、`main-layout.tsx:186-254` |
| 最近项目 | `useRecentFoldersStore.recentFolders` | 本地 store | `title-project-menu.tsx:113` |
| 主题色 | `lithe.json` → `theme-registry.applyTheme()` 写入 `documentElement` | 前端扩展 | `theme-registry.ts:71-103` |
| 窗口最小化/最大化/关闭/拖拽/缩放 | Tauri window API（非自定义命令）：`minimize`、`toggleMaximize`、`isMaximized`、`startDragging`、`startResizeDragging` | `@tauri-apps/api/window` | `window-controls.tsx:27-45`、`title-bar.tsx:141`、`window-resize-border.tsx:114-121` |
| 关闭确认 | `requestWindowClose()` → `WindowCloseGuard` | 前端 | `window-controls.tsx:48`、`main-layout.tsx:374` |
| 原生窗口外观 | `set_native_window_appearance`（原生命令，白名单） | `tauri-core.ts:74`、`main.rs:156` |
| 菜单命令执行 | Tauri 事件 `menu_execute_command` → 命令注册表 | `window-menu-bar.tsx:119-124`、`use-menu-events.ts:59-61` |
| 菜单栏开关事件 | `menu_toggle_menu_bar` → `invoke("toggle_menu_bar")` | `use-menu-events.ts:70`、`use-menu-events-wrapper.ts:353` |

### 4.3 命令疑点（重要）

| 命令 | 情况 | 证据 |
| --- | --- | --- |
| `toggle_menu_bar` | 前端有 4 处调用；`windows/tauri/src-tauri/` 内**未找到**同名 `#[tauri::command]`，也不在 `platform.rs` 翻译表与 `nativeCommands` 白名单中——调用会走 `platform_invoke` 并大概率失败（代码已 `catch(console.error)`）。重写时可省略或自行定义 | 调用：`use-menu-events-wrapper.ts:353`、`window-command-actions.ts:55`、`view-actions.tsx:203`、`appearance-settings.tsx:523`；白名单：`tauri-core.ts:18-91`；翻译表：`platform.rs`（grep 无 `menu_bar`） |
| `uses_native_window_chrome` | 前端仅在 Linux 分支调用（`IS_LINUX` 不成立时直接 `setUsesNativeWindowChrome(false)`），Windows 上永不触发；Rust 侧未找到实现，仅测试 mock 有 | `use-native-window-chrome.ts:5-12`、`src/mocks/tauri-api-mock.ts:24` |
| `read_directory` | 前端本地读取走插件而非自定义命令；`platform.rs` 无 `read_dir` 翻译 | `file-system/controllers/platform.ts:173-177`、`platform.rs`（grep 无 `read_dir`） |

---

## 5. 文案（`windows/tauri/src/i18n/locale.ts` 键名与中文原文）

语言目录：`DISPLAY_LANGUAGES = ["en-US", "zh-CN"]`（`locale.ts:2`）；键为扁平点号字符串，中文目录自 `locale.ts:4605` 起。

### 5.1 标题栏 / 项目切换

| 键 | 中文 | 证据 |
| --- | --- | --- |
| `titleProject.trigger` | 项目：{project} | `locale.ts:6191` |
| `titleProject.newProject` | 新建项目… | `locale.ts:6171` |
| `titleProject.open` | 打开… | `locale.ts:6174` |
| `titleProject.cloneRepository` | 克隆仓库… | `locale.ts:6186` |
| `titleProject.openProjects` | 打开的项目 | `locale.ts:6187` |
| `titleProject.recentProjects` | 最近项目 | `locale.ts:6189` |
| `titleProject.noRecentProjects` | 没有最近项目 | `locale.ts:6190` |
| `titleProject.newWindow` | 新建窗口 | `locale.ts:6172` |
| `titleProject.addProject` | 添加项目 | `locale.ts:6173` |
| `titleProject.openFolder` | 打开文件夹 | `locale.ts:6175` |
| `titleProject.openFolderInNewWindow` | 在新窗口中打开文件夹 | `locale.ts:6176` |
| `titleProject.closeAllProjects` | 关闭所有项目 | `locale.ts:6178` |
| `titleProject.openInNewWindow` | 在新窗口中打开 | `locale.ts:6177` |
| `titleProject.selectIcon` | 选择图标 | `locale.ts:6179` |
| `titleProject.setDisplayAlias` | 设置显示别名 | `locale.ts:6180` |
| `titleProject.displayAliasPrompt` | 输入显示别名，将显示在文件夹名之后，用于区分同名项目。 | `locale.ts:6181` |
| `titleProject.removeProject` | 移除项目 | `locale.ts:6182` |
| `titleProject.currentProject` | 当前项目 {project} | `locale.ts:6183` |
| `titleProject.switchToProject` | 切换到 {project} | `locale.ts:6184` |
| `titleProject.switchToProjectMenu` | 切换到项目 | `locale.ts:6185` |
| `titleProject.closeProject` | 关闭项目 {name} | `locale.ts:6188` |
| `projectOpen.title` | 打开项目 | `locale.ts:6165` |
| `workbench.currentFile` | 当前文件 | `locale.ts:6148` |
| `workbench.moreProjectActions` | 更多项目操作 | `locale.ts:6149` |
| `window.menu` | 菜单 | `locale.ts:7903` |
| `window.minimize` / `window.maximize` / `window.restore` / `window.close` | 最小化 / 最大化 / 还原 / 关闭 | `locale.ts:7899-7902` |
| `update.available` / `update.installing` / `update.failed` / `update.options` | 有可用更新 / 正在安装 / 更新失败 / 更新选项 | `locale.ts:6444`、`:6442`、`:6443`、`:6449` |

### 5.2 菜单栏（`menu.*`）

| 键 | 中文 | 键 | 中文 |
| --- | --- | --- | --- |
| `menu.file` | 文件 | `menu.edit` | 编辑 |
| `menu.view` | 视图 | `menu.go` | 转到 |
| `menu.terminal` | 终端 | `menu.run` | 运行 |
| `menu.tools` | 工具 | `menu.window` | 窗口 |
| `menu.help` | 帮助 | `menu.toggleActivitySidebar` | 切换活动侧栏 |
| `menu.toggleSecondarySidebar` | 切换辅助侧栏 | `menu.toggleTerminal` | 切换终端 |
| `menu.toggleMenuBar` | 切换菜单栏 | `menu.toggleFullscreen` | 切换全屏 |
| `menu.minimize` | 最小化 | `menu.maximize` | 最大化 |
| `menu.preferences` | 首选项 | `menu.keyboardShortcuts` | 键盘快捷键 |
| `menu.webInspector` | Web 检查器 | `menu.theme` | 主题 |

（全部 `menu.*` 中文键见 `locale.ts:7724-7822`；菜单项装配见 `window-menu-bar.tsx:131-531`）

### 5.3 活动栏

| 键 | 中文 | 证据 |
| --- | --- | --- |
| `workbench.project` | 项目 | `locale.ts:4605` |
| `workbench.changes` | 更改 | `locale.ts:4606` |
| `workbench.search` | 搜索 | `locale.ts:4607` |
| `workbench.gitLog` | 提交记录 | `locale.ts:5910` |
| `workbench.settings` | 设置 | `locale.ts:5912` |
| `workbench.run` | 运行 | `locale.ts:5913` |
| `workbench.maven` | Maven | `locale.ts:5914` |
| `workbench.terminal` | 终端 | `locale.ts:5915` |
| `workbench.diagnostics` | 诊断 | `locale.ts:5916` |
| `workbench.activityViews` | 活动视图 | `locale.ts:7890` |
| `layout.actions` | 操作 | `locale.ts:4700` |
| `layout.showInActivitySidebar` | 在活动侧栏中显示 | `locale.ts:4702` |
| `layout.projectDots` | 项目圆点 | `locale.ts:4703` |
| `layout.resizeActivityRail` | 调整活动栏大小 | `locale.ts:4701` |
| `layout.resizeSidebar` | 调整侧边栏大小 | `locale.ts:7887` |
| `layout.resizeAiChat` | 调整 AI 聊天大小 | `locale.ts:7886` |
| `files.copyPath` | 复制路径 | `locale.ts:7388` |
| `files.reveal` | 在资源管理器中显示 | `locale.ts:7400` |
| `extensions.title` | 扩展 | `locale.ts:7566` |
| `notifications.title` | 通知 | `locale.ts:7322` |
| `run.title` | 运行 | `locale.ts:5927` |
| `maven.title` | Maven | `locale.ts:6043` |
| `welcome.openProject` | 打开项目 | `locale.ts:6154` |
| Maven 项标签（拼接） | `` `${t("run.title")} - ${t("maven.title")}` `` = “运行 - Maven” | `main-sidebar.tsx:200`、`sidebar-pane-selector.tsx:243` |
| 右键菜单 "Show All" | **未本地化硬编码英文** `Show All` | `main-sidebar.tsx:756` |

### 5.4 状态栏

| 键 | 中文 | 证据 |
| --- | --- | --- |
| `footer.statusBar` | 状态栏 | `locale.ts:7316` |
| `footer.filePath` | 文件路径 | `locale.ts:7317` |
| `footer.gitBranch` | Git 分支 | `locale.ts:7318` |
| `footer.cursor` | 跳转到行和列 | `locale.ts:7305` |
| `footer.encoding` | 文件编码 | `locale.ts:7306` |
| `footer.indent` | 缩进 | `locale.ts:7307` |
| `footer.spaces` | {count} 个空格 | `locale.ts:7308` |
| `footer.readOnly` / `footer.writable` | 只读 / 可写 | `locale.ts:7309-7310` |
| `footer.memory` | 内存 | `locale.ts:7311` |
| `footer.memoryUsage` | 总计 {total} · Lithe {used} | `locale.ts:7312` |
| `footer.change` / `footer.changes` | {count} 个更改（单数/复数同文案） | `locale.ts:7314`、`:7313` |
| `footer.noChanges` | 没有更改 | `locale.ts:7315` |
| `footer.moveLeft` / `footer.moveRight` / `footer.resetOrder` | 左移 / 右移 / 重置页脚顺序（**死代码**） | `locale.ts:7319-7321` |
| `preparation.starting` | 正在启动 Java 服务 | `locale.ts:5917` |
| `preparation.importing` | 正在导入 Java 项目与依赖 | `locale.ts:5918` |
| `preparation.configuring` | 正在同步 Java 项目配置 | `locale.ts:5919` |
| `preparation.building` | 正在构建 Java 项目 | `locale.ts:5920` |
| `preparation.failed` | Java 项目准备失败，请查看详情 | `locale.ts:5923` |
| `preparation.explanation` | Java 工作区准备完成后才能启动。普通后台索引不阻塞运行，编译在运行前执行。 | `locale.ts:5924` |
| `preparation.settings` / `preparation.logs` | 语言服务设置与重试 / 查看日志 | `locale.ts:5925-5926` |
| 无更改图标 | 仅图标（`CheckCircleIcon`），无文案 | `footer-editor-status.tsx:135` |

### 5.5 外壳内未本地化的硬编码英文（重写时需注意）

| 文本 | 位置 |
| --- | --- |
| `Drop folder to open project, or file to open buffer` | `main-layout.tsx:286` |
| `Show All` | `main-sidebar.tsx:756` |
| `Opening {project}`（Spinner label） | `main-sidebar.tsx:625` |
| `You're on the latest version` / `Failed to check for updates` | `use-menu-events-wrapper.ts:340`、`:342` |

---

## 6. gpui-kit 对应建议

对照源码根：`D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\`
`gpui-kit-0.6.6` 只是门面，`gpui_kit::component` = `gpui_component`、`gpui_kit::base` = `gpui_base`（`gpui-kit-0.6.6\src\lib.rs:106`、`:143`）。下表用真实模块路径。

| Lithe 元素 | gpui-kit 0.6.6 对应 | 判定 | 依据 / 差距 |
| --- | --- | --- | --- |
| 窗口 + 标题栏宿主 | `gpui_component::title_bar::TitleBar` + `TitleBar::window_options()` / `title_bar_options()`；`gpui_component::root::Root` | **需自行组合** | `title_bar.rs:42`、`:59-91`、`root.rs:37`。gpui 标题栏固定高 `TITLE_BAR_HEIGHT = px(34.)`（`title_bar.rs:15`）且返回 `app_owns_titlebar_drag: true`；Lithe 为 40px、背景是自绘 `--surface`/玻璃层，需 `.h(px(40.))` 覆盖并用 `.refine_style` 覆盖底色 |
| 窗口三键（最小化/最大化/关闭） | `TitleBar` 内置的私有 `ControlIcon`（`title_bar.rs:110-245`） | **gpui-kit 没有可定制尺寸的公开 API** | 控件宽固定 `w(TITLE_BAR_HEIGHT)`=34px（`title_bar.rs:211`），Lithe 为 **56px** 直角；只能通过 `gpui::WindowControlArea::Min/Max/Close`（`title_bar.rs:155-161`）自行实现按钮 + 原生命中区域 |
| 标题栏菜单栏 | `gpui_component::menu::app_menu_bar::AppMenuBar`；原生走 `gpui_component::native_menu::windows` | **需自行组合** | `app_menu_bar.rs:26`、`native_menu\windows.rs`。Lithe 是自绘 Menubar（24px 胶囊 + 20px 项），非系统菜单栏；建议用 `gpui_component::menu::popup_menu::PopupMenu`（`popup_menu.rs:282`，含 `menu_with_check` `:565`）自行搭 |
| 标题栏项目下拉 | `gpui_component::menu::dropdown_menu::DropdownMenu` / `DropdownMenuPopover`；条目用 `PopupMenuItem` | **需自行组合** | `dropdown_menu.rs:12`、`:37`、`popup_menu.rs:71`。gpui-kit 无“项目行（图标+名称+路径+勾）”预置行，需自绘或用 `menu_element`（`popup_menu.rs:587`） |
| 项目徽标（首字母色块） | `gpui_component::avatar::avatar::Avatar` | **需自行组合** | `avatar\avatar.rs`。Lithe 是 28×28 圆角方块 + 首字母（`title-project-menu.tsx:62-68`），Avatar 形状/角标语义不同 |
| 项目标签条 | `gpui_component::tab::tab_bar::TabBar` + `tab::tab::Tab`（`TabVariant::underline`） | **需自行组合** | `tab_bar.rs:40`、`tab.rs:397`、`:14`、`:529`。gpui-kit 的 `Tab` 无“关闭按钮 + 下划线指示器 + 单项目隐藏”组合；需 `Tab::prefix/suffix`（`tab.rs:535`、`:541`）+ 自定义逻辑 |
| 左侧活动栏（38px 图标竖条） | `gpui_component::sidebar::Sidebar`（`side(Side::Left)`、`collapsed`）+ `sidebar::menu::SidebarMenuItem` / `sidebar::header::SidebarHeader` / `sidebar::footer::SidebarFooter` | **需自行组合** | `sidebar\mod.rs:222`、`:254`、`:264`、`:270`、`sidebar\menu.rs:94`、`sidebar\header.rs:10`、`sidebar\footer.rs:10`。gpui-kit Sidebar 面向可折叠抽屉（默认更宽），Lithe 折叠态仅 38px 且只有图标按钮 → 建议 `gpui_component::button::button_icon::ButtonIcon`（`button\button_icon.rs:7`）+ `tooltip::Tooltip`（`tooltip.rs:34`）自建竖条，而非硬套 Sidebar |
| 活动栏项选中态（`bg-selected` / `bg-accent`） | `gpui_component::button::toggle::Toggle`（`.checked()`、`ToggleVariant`）或 `Button::toggled()` | **需自行组合** | `button\toggle.rs:34`、`:92`；`button\button.rs:483`。Lithe 选中底色是自定义 `--selected`，gpui-kit token 无同名（见下） |
| 右侧插件竖条 | 同活动栏方案 | **需自行组合** | `plugin-activity-rail.tsx:34` |
| 未读徽标 | `gpui_component::badge::Badge`（`.count()`、`.max()`、`.dot()`） | **可一比一** | `badge.rs:31`、`:56-76`；Lithe 的 `9+` 上限即 `Badge::max(9)`（`notifications-trigger.tsx:46`） |
| 状态栏 | `gpui_component::status_bar::StatusBar`（`.left()` / `.right()` / `child`） | **可一比一** | `status_bar.rs:32`、`:51-60`。默认 `py_1 px_2 text_xs border_t_1`（约 26px），Lithe 为 24px/13px、无上边框、背景 `--surface` → 用 `.refine_style` 微调；左右分组语义完全对应 |
| 状态栏数据 chip | `gpui_component::button::button::Button`（`ButtonVariant::Ghost`、`.compact()`、`.tooltip()`）+ `gpui_component::separator::Separator::vertical()` | **可一比一** | `button\button.rs:186`、`:142`、`:427`、`:389`；`separator.rs:28` |
| 状态栏文件路径面包屑 | `gpui_component::breadcrumb::Breadcrumb` / `BreadcrumbItem` | **可一比一**（下拉需自行组合） | `breadcrumb.rs:13`、`:20`、`:31`、`:47`。Lithe 的分段点击弹目录树（`file-path-breadcrumb.tsx:205-273`）需配 `DropdownMenu` |
| 状态栏内存/只读图标 | `gpui_component::icon::Icon` + `IconName` | **需自行组合**（图标集需自备） | `icon.rs`。gpui-kit 用 `IconName` 枚举；Lithe 用 Phosphor 图标，需注册自有图标资产（`gpui_kit::assets`） |
| 面板分隔条（左右可拖拽面板） | `gpui_base::resizable`：`h_resizable` / `ResizablePanelGroup` / `ResizablePanel::size_range` / `.visible` / `.on_resize` / `ResizableState` | **可一比一** | `resizable\mod.rs:17`、`:22`、`resizable\panel.rs:31`、`:252`、`:269`、`:275`、`:283`、`:111`。Lithe 的 min 140 / max `viewport−reserved−360` / 隐藏 (`hidden`) / 持久化宽度全部有对应（`resizable-pane.tsx:44`、`:60-62`、`:150`、`:135`）；分隔条外观用 `.with_handle_appearance` / 主题 `ResizableTheme { handle, active_handle }`（`theme.rs:109-111`）对齐“透明→`--primary`” |
| 分隔条拖拽性能 | `ResizableState` 内置拖拽与 `ResizablePanelEvent`；`resizable\resize_handle.rs:16` | **可一比一** | `resizable\mod.rs:33`、`:388`、`resize_handle.rs:53`。Lithe 手写 rAF 节流（`resizable-pane.tsx:120-128`）在 gpui 中由框架每帧重排替代 |
| 活动栏项目轮播（横滑切换项目） | 无对应 | **gpui-kit 没有** | 全仓库无 carousel-for-sidebar；最接近的是 `gpui_component::carousel`（`carousel\carousel.rs`，面向内容轮播）。需用 `gpui_base::motion::timing`/`gpui::Animation` 自实现（`gpui-base-0.6.6\src\motion\timing.rs`） |
| 右键菜单 | `gpui_component::menu::context_menu::ContextMenu` / `ContextMenuExt` | **可一比一** | `menu\context_menu.rs:13`、`:42`。可勾选项用 `PopupMenuItem::checked`（`popup_menu.rs:180`），子菜单 `submenu`（`:694`），分隔 `separator`（`:680`） |
| 工具提示（含快捷键展示） | `gpui_component::tooltip::Tooltip::new/.key_binding` | **可一比一** | `tooltip.rs:34`、`:75`、`:207` |
| 首帧占位外壳 | `gpui_component::skeleton` | **需自行组合** | `skeleton.rs`。gpui 首帧无 WebView 白屏问题，通常不需要等价物 |
| 欢迎页 | `gpui_component::empty::Empty` + `gpui_component::list` | **需自行组合** | `empty.rs`、`list\list.rs` |
| 工作区/编辑器岛容器 | `gpui_component::dock`：`dock_area` / `Panel` / `TabPanel` / `PanelStyle` | **需自行组合** | `dock\mod.rs:132`；`dock\panel.rs:31`、`:71`、`:153`。gpui-kit Dock 自带标签与面板工具条语义，与 Lithe 自定义 split-view（`features/panes/components/split-view-root`）不直接对应；外壳只需把 `gpui_base::resizable` 组合成三栏即可 |
| 全局浮层宿主（命令面板/对话框/通知） | `gpui_component::root::Root`（`open_dialog` / `push_notification` / `open_sheet_at`） | **可一比一** | `root.rs:37`、`:297`、`:425`、`:379` |
| 通知 | `gpui_component::notification::Notification`（`info/success/warning/error`、`placement`、`autohide`） | **可一比一** | `notification.rs:107`、`:167`、`:196-217` |
| 加载指示 | `gpui_component::spinner::Spinner` | **可一比一** | `spinner.rs:10` |
| 窗口边框/缩放热区 | `gpui_component::window_border::WindowBorder`（`resize_hit_size`）+ gpui `WindowOptions.window_min_size` | **可一比一** | `window_border.rs:25`、`:63`、`:88`。Lithe 的 5px 边 / 10px 角（`window-resize-border.tsx:23-24`）可直接映射为 `resize_hit_size(px(5.))`；最小 720×480 用 `window_min_size` |
| 主题令牌体系 | `gpui_component::theme::Theme`（`ColorTokens` / `RadiusTokens` / `SpacingTokens` / `TypographyTokens` / `ShadowTokens`，来源 `gpui_base::theme_tokens::SemanticThemeTokens`） | **需自行组合（需扩展令牌）** | `gpui-base-0.6.6\src\theme_tokens.rs:11-37`、`:109-139`、`:164-172`；颜色语义配置 `gpui_component::theme::schema::SemanticColorConfig`（`theme\schema.rs:106-123`）。**缺口**：Lithe 的 `--subtle-foreground`、`--selected`、`--tab-bar/tab-active/tab-hover`、`--git-*` 在 gpui-kit 中无同名语义 token（有 `accent` 但语义不同：Lithe `accent` 是悬停底色、`selected` 才是选中底色），需要自定义 `Theme` 扩展或把 `selected` 映射到 `accent`+`primary` 组合 |
| 尺寸令牌映射 | `RadiusTokens` / `SpacingTokens` / `TypographyTokens` | **需自行组合（数值不同）** | gpui-kit 半径/间距/字号是成套 token（`theme_tokens.rs:109-172`），Lithe 是 `--lithe-*` 专用变量（`theme.css:118-134`）。建议把关键值（40/24/38/4/1px 分隔线/13px）写入自有 token 层，而不是硬编码到各元素 |
| 动效令牌 | `gpui_component::theme::motion::MotionTokens`（`duration_fast/normal`、`easing_enter/exit/move`） | **可一比一** | `theme\motion.rs:8-19` 对应 Lithe `--app-duration-fast 150ms` / `--app-duration-normal 200ms` / `--app-ease-smooth` |
| 减少动效 | `gpui_base::reduce_motion` | **可一比一** | `gpui-base-0.6.6\src\reduce_motion.rs`；对应 `data-reduce-motion`（`utilities.css:110-152`） |
| 玻璃/透明层（`backdrop-filter: none` + `color-mix` 半透明） | 无对应 | **gpui-kit 没有** | Lithe 的 `.lithe-glass-*` 是 CSS `color-mix` + 透明窗口（`window-transparency.css:5-72`）。gpui 需用 `Hsla` 手动混色 + 窗口 `transparent` 标志；Lithe Windows 窗口配置 `transparent: false`（`tauri.windows.conf.json:14`），说明玻璃层只做半透明叠加，可直接换算为 `Hsla` 混合 |

---

## 7. 未查清

1. **`lithe-workbench-gap` 之外的实际“岛间距”在窄窗下的表现**：`ResizablePane` 宽度 clamp 依赖 `window.innerWidth`（`resizable-pane.tsx:50`），但 gpui 无 `window.innerWidth` 等价物；重写时用什么作为视口宽度基准（逻辑尺寸 / scale factor）未在 Windows 前端中找到依据。
2. **`--ui-text-xs` / `ui-text-xs` 的定义位置**：`title-project-menu.tsx:102`、`:196`、`:218` 使用 `ui-text-xs`，但 `utilities.css:30-44` 只定义了 `ui-text-sm/caption/chrome/base`，未见 `ui-text-xs`；实际渲染字号未查清（可能由 Tailwind 内置或其它 CSS 提供，未定位）。
3. **`compactMenuBar` 默认值与设置项取值范围**：只确认读取 `settings.compactMenuBar`（`title-bar.tsx:61`）与三处渲染分支（`window-menu-bar.tsx:539-552`），未查设置默认值定义文件。
4. **`windowChromeDensity` 的可选值与默认值**：仅确认写入 `data-window-chrome-density`（`ui-preferences.ts:9`）与 comfortable 覆盖块（`theme.css:188-200`），未查枚举定义与默认档。
5. **活动栏展开态（`expanded=true`）在 Windows 产品中是否可达**：`MainLayout` 只传 `false`（`main-layout.tsx:302`），全仓库未见其它 `SidebarActivityRail` 调用点；`expanded` 相关的 140–320 拖拽与项目圆点逻辑可能是为 macOS/未来预留，未确认 Windows 是否有入口切换。
6. **状态栏 `gitChanges` 的点击目标在右栏不可用时的行为**：`openSidebarView("git")` 走 primary 级视图（`footer-editor-status.tsx:127`、`sidebar-pane-utils.ts:26-34`），但 `isRightSidebarVisible` 与左侧栏的联动冲突未在前端代码中查清。
7. **`menu_execute_command` 的命令 ID 全集**：只抽样确认 `workbench.showDebugger`、`workbench.closeWindow`、`editor.*` 等（`window-menu-bar.tsx:135-531`），命令注册表与快捷键映射的完整清单未核对（属 `features/keymaps`，超出本次外壳范围）。
8. **`toggle_menu_bar` 的真实宿主**：Rust 侧未找到实现（见 4.3），是否为已废弃命令或有条件编译分支未查清。
9. **`SidebarProjectDots` 的 `ContextMenu` 定位与主 rail 右键菜单的冲突处理**：`sidebar-projects.tsx:79` 做了 `stopPropagation`，但外层的 rail 级 `ContextMenuTrigger`（`main-sidebar.tsx:667`）在圆点上右键的最终命中顺序未通过运行验证。
10. **窗口控件在 Windows 上是否重复（自绘 3 键 vs 系统按钮）**：配置为 `decorations: false`（`tauri.windows.conf.json:12`）且自绘三键（`window-controls.tsx`），但 gpui 侧 `TitleBar` 会自行绘制系统控制区；两者在 Rust 重写后是否需要关闭其中一个未在本仓库前端代码中给出依据。
