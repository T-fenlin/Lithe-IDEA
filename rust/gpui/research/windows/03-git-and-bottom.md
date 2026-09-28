# 03 · Windows 前端界面规格：Git 工具窗与底部区域

> 用途：供团队用 **Rust + gpui-kit 0.6.6** 重写桌面界面时"照着写代码"。
> 唯一真源：`windows/tauri/src/`（Tauri v2 + React + TypeScript + Tailwind v4 + shadcn/Base UI）。
> 旧 macOS 规格（`.artifacts/ui-map/*`）已作废，本文不引用。
> 证据格式：`相对路径:行号`。查不到的一律写"未找到"，不做推测。
> 本文为只读调研产物，未修改任何既有文件。

---

## 0. 换算基准（读度量表前必读）

| 基准 | 值 | 证据 |
| --- | --- | --- |
| 根字号 | `html { font-size: calc(16px * var(--app-ui-scale)) }`，`--app-ui-scale: 1` → **1rem = 16px** | `windows/tauri/src/styles/theme.css:113`, `windows/tauri/src/styles/theme.css:220` |
| Tailwind 间距 1 单位 | 0.25rem = **4px** | Tailwind v4 默认，配合上一条 |
| `ui-text-sm` / `ui-text-base` | **13px**（`--app-ui-font-size`） | `windows/tauri/src/styles/theme.css:112`, `windows/tauri/src/styles/utilities.css:30-32` |
| `ui-text-chrome` | **13px**（舒适密度 14px） | `windows/tauri/src/styles/theme.css:115` |
| `ui-text-caption` | **12px**（舒适密度 13px） | `windows/tauri/src/styles/theme.css:114` |
| `leading-row` | `line-height: 1.35` | `windows/tauri/src/styles/theme.css:4` |
| 圆角 `rounded-sm` | `--radius-sm = calc(var(--radius)*0.6)` = **4.8px** | `windows/tauri/src/styles/theme.css:6`, `:134` |
| 圆角 `rounded-md` | `--radius-md = calc(var(--radius)*0.8)` = **6.4px** | `windows/tauri/src/styles/theme.css:7`, `:134` |
| 圆角 `rounded-lg` | `--radius-lg = var(--radius)` = **8px** | `windows/tauri/src/styles/theme.css:8`, `:134` |
| 圆角 `rounded-xl` | `--radius-xl = calc(var(--radius)*1.4)` = **11.2px** | `windows/tauri/src/styles/theme.css:9`, `:134` |
| `rounded-(--lithe-chrome-radius)` | **4px** | `windows/tauri/src/styles/theme.css:133` |
| `--lithe-workbench-gap` | **4px** | `windows/tauri/src/styles/theme.css:125` |
| `--lithe-chrome-control-height` | **24px**（1.5rem；舒适密度 **28px**） | `windows/tauri/src/styles/theme.css:126`, `:194` |
| `--lithe-pane-header-height` | **36px**（2.25rem） | `windows/tauri/src/styles/theme.css:120` |
| `--lithe-tab-height` | **28px**（1.75rem） | `windows/tauri/src/styles/theme.css:122` |
| `--lithe-sidebar-header-height` | **32px**（2rem；舒适密度 36px） | `windows/tauri/src/styles/theme.css:124`, `:193` |
| `--lithe-footer-height` | **24px**（1.5rem；舒适密度 32px） | `windows/tauri/src/styles/theme.css:119`, `:191` |
| 动效时长/曲线 | `--app-duration-fast: 150ms`、`--app-ease-smooth: cubic-bezier(0.22,1,0.36,1)`、`--app-press-scale: 1` | `windows/tauri/src/styles/theme.css:135-139` |

密度切换：`:root[data-window-chrome-density="comfortable"]`，见 `windows/tauri/src/styles/theme.css:188-200`。**下面所有度量按默认（compact）密度**。

### 颜色 token 真值

默认亮/暗主题来自 `windows/tauri/src/extensions/themes/builtin/lithe.json`（由 `windows/tauri/src/extensions/themes/default-theme.ts:1,36-49` 读取并加 `--` 前缀注入 CSS 变量）。

| token | light | dark |
| --- | --- | --- |
| `--background` | `#ffffff` | `#1e1f22` |
| `--surface` | `#f7f8fa` | `#2b2d30` |
| `--foreground` | `#1f2328` | `#dfe1e5` |
| `--muted-foreground` | `#4f5965` | `#b4b8bf` |
| `--subtle-foreground` | `#68717d` | `#8b929e` |
| `--border` | `#dfe1e5` | `#43454a` |
| `--accent` | `#edf3ff` | `#393b40` |
| `--selected` | `#d4e2ff` | `#2e436e` |
| `--selection` | `rgba(53,116,240,0.2)` | `#214283` |
| `--primary` | `#3574f0` | `#3574f0` |
| `--destructive` | `#cf3f4f` | `#db5c5c` |
| `--success` | `#27864f` | `#57965c` |
| `--warning` | `#a86400` | `#d6ae58` |
| `--info` | `#3574f0` | `#548af7` |
| `--git-modified` | `#a86400` | `#d9a441` |
| `--git-modified-staged` | `#bd7411` | `#e5b75e` |
| `--git-added` | `#27864f` | `#4cc38a` |
| `--git-deleted` | `#cf3f4f` | `#f16d75` |
| `--git-untracked` | `#0877c1` | `#58a6e7` |
| `--git-renamed` | `#7656a8` | `#c8a2f4` |

`--border-strong = color-mix(in srgb, var(--border) 72%, var(--foreground) 28%)`，见 `windows/tauri/src/styles/theme.css:140`。

---

## 1. 区域清单

### 1.1 壳层与底部窗的挂载位置（关键结论）

```
MainLayout                                   windows/tauri/src/features/layout/components/main-layout.tsx:81
└─ div.lithe-layout-shell  flex size-full flex-col overflow-hidden bg-surface        :280
   ├─ TitleBarWithSettings                                                            :292
   ├─ ProjectTabBar (hideWhenSingle)                                                  :293
   └─ div.lithe-workbench-glass  flex flex-1 flex-col                                :297
      ├─ div.flex flex-1 flex-row overflow-hidden pr-(--lithe-workbench-gap)          :298-301
      │  ├─ SidebarActivityRail                                                       :302
      │  ├─ ResizablePane(position="left", widthKey="sidebarWidth")  → MainSidebar     :303-310
      │  ├─ div.flex min-h-0 min-w-0 flex-1 flex-col          ← "中央列"              :312
      │  │  ├─ div.lithe-glass-island (编辑器) → WorkbenchErrorBoundary               :313-317
      │  │  └─ [terminalWidthMode==="editor"] <BottomPane/>                           :318-322
      │  ├─ ResizablePane(position="right", widthKey="rightToolWindowWidth")          :325-341
      │  └─ PluginActivityRail                                                        :342
      └─ [terminalWidthMode==="full"] div.px-(--lithe-workbench-gap) > <BottomPane/>  :345-351
   └─ Footer (showStatusBar)                                                          :354
```

**底部窗是"横向占满"还是别的布局？——两种模式，默认不是整窗占满：**

- 默认 `terminalWidthMode === "editor"`：`BottomPane` 挂在**中央列**（编辑器列）内，横向宽度 = 窗口宽度 − 左活动栏 − 左侧栏 − 右侧工具窗 − 插件栏，**不覆盖两侧栏**。默认值证据：`windows/tauri/src/features/terminal/stores/terminal.store.ts:29`（`widthMode: "editor"`）。
- `terminalWidthMode === "full"`：`BottomPane` 挂在 `lithe-workbench-glass` 下单列，**横向跨越整个工作台**（含两侧栏下方），左右各留 `px-(--lithe-workbench-gap)` = 4px。证据：`windows/tauri/src/features/layout/components/main-layout.tsx:345-351`。

### 1.2 BottomPane 组件树

```
BottomPane (default export)                 windows/tauri/src/features/layout/components/bottom-pane/bottom-pane.tsx:28
├─ div[ref=paneFrameRef] flex shrink-0 flex-col   height: calc(Npx + var(--lithe-workbench-gap))  :322-334
│  ├─ resizeGutter   role="separator" aria-orientation="horizontal"  高 4px           :222-241
│  └─ paneContent
│     └─ div[data-bottom-pane-drop-target].lithe-glass-island
│         relative flex min-h-0 flex-col overflow-hidden rounded-xl
│         border-border/70 border-t border-l bg-background                            :244-254
│        └─ div.h-full.overflow-hidden                                                :255
│           ├─ TerminalContainer   （始终挂载，非激活时 "hidden"）                      :257-264
│           ├─ DebuggerView                                                            :266-272
│           ├─ RunPane                                                                 :274-278
│           ├─ MavenRunPane                                                            :280-284
│           ├─ DiagnosticsBuffer  （showCloseButton，onClose=隐藏底部窗）              :286-295
│           ├─ BottomBufferPane   （bottomPaneActiveTab==="buffers" 且有 buffer）       :297-301
│           └─ GitLogToolWindow   （bottomPaneActiveTab==="gitLog"）                    :303-307
```

**底部窗没有自己的标签条（tab strip）组件。** 标签由外部触发：
- 左侧活动栏 `SidebarPaneSelector` 的回调：`onGitLogClick / onTerminalClick / onDiagnosticsClick / onRunClick / onMavenClick`，见 `windows/tauri/src/features/layout/components/sidebar/main-sidebar.tsx:642-652`。
- 命令面板 `windows/tauri/src/features/command-palette/constants/view-actions.tsx:110-157`。
- 状态栏 / 标题栏按钮（见 1.6）。
- `BottomPane` 内部用 `useUIState.bottomPaneActiveTab` 单值切换，见 `windows/tauri/src/features/layout/components/bottom-pane/bottom-pane.tsx:30-31`。
- 关闭逻辑：`gitLog` 标签由 `GitLogTitleBar` 的"最小化"按钮 `setIsBottomPaneVisible(false)` 触发，见 `windows/tauri/src/features/git/components/log/git-log-title-bar.tsx:62-71` 与 `windows/tauri/src/features/git/components/log/git-log-tool-window.tsx:579`。

工具窗类型（`bottomPaneActiveTab` 取值）来自 `windows/tauri/src/features/window/stores/ui-state.store.ts`（值集：`terminal | debugger | run | maven | diagnostics | buffers | gitLog | references`，从 `bottom-pane.tsx:63-106,257-307` 与 `command-palette/constants/view-actions.tsx:110-157` 反推）。`"references"` 一旦成为激活标签会被立即关闭底部窗（`bottom-pane.tsx:62-66`）。

### 1.3 Git Log 工具窗组件树

```
GitLogToolWindow                            windows/tauri/src/features/git/components/log/git-log-tool-window.tsx:78
└─ div.flex.h-full.min-h-0.flex-col.overflow-hidden.bg-background.text-foreground    :564-567
   ├─ {historyDialog}                                                                :568
   ├─ GitFetchDialog（条件）                                                          :569
   ├─ GitLogTitleBar                                                                 :570-580
   │   └─ git-log-title-bar.tsx:10
   ├─ div.flex.shrink-0.gap-4.border-b.px-3.py-1.text-xs
   │   └─ div[role="tablist"].flex.gap-4                                            :582-587
   │      ├─ button[role=tab] "日志"      (git.console.log)                           :584
   │      └─ button[role=tab] "控制台"    (git.console.title)                          :585
   ├─ [panel==="console"] GitExecutionConsole                                        :588
   └─ [panel==="log"]
      ├─ 刷新失败横幅 div.h-7.flex.shrink-0.items-center.gap-2
      │   border-destructive/30.border-b.bg-destructive/10.px-2.ui-text-sm             :589-600
      ├─ 无仓库占位 / 加载中占位                                                       :602-610
      ├─ GitRepositoryEmptyState（加载失败且无提交）                                    :611-612
      └─ ResizablePanelGroup orientation="horizontal" className="min-h-0 flex-1"       :614-621
         ├─ ResizablePanel id="references" defaultSize="19" minSize={140}              :622
         │   └─ GitReferenceTree  (git-reference-tree.tsx:707)
         ├─ ResizableHandle                                                            :647
         ├─ ResizablePanel id="commits" defaultSize="57" minSize={320}                 :648
         │   └─ GitCommitTable  (git-commit-table.tsx:45)
         │      ├─ 工具行 h-8（筛选输入 / 字段下拉 / 装饰开关 / 计数）                     :200-253
         │      ├─ 表头行 h-6（提交 / 作者 / 日期）                                       :255-259
         │      └─ 虚拟滚动容器                                                          :261-266
         │         └─ div.relative.min-w-130 (高度 = virtualizer.getTotalSize())        :276
         │            └─ ContextMenu × N
         │               └─ ContextMenuTrigger[role=button][data-git-commit-index]
         │                  └─ GitGraphRow  (git-graph-row.tsx:26)
         │                     ├─ <svg> 泳道图 + 节点                                          :33-79
         │                     └─ div.flex.min-w-0.flex-1（标签徽章 + 提交说明）               :81-101
         │                  + 作者 span.w-28、日期 span.w-32                                  :321-326
         │            + 加载更多行 h-9（hasMore 时）                                      :428-443
         ├─ ResizableHandle                                                            :683
         └─ ResizablePanel id="inspector" defaultSize="24" minSize={220}                :684
            └─ GitCommitInspector  (git-commit-inspector.tsx:21)
               └─ ResizablePanelGroup orientation="vertical"                            :97-103
                  ├─ ResizablePanel id="files" defaultSize="62" minSize={90}            :104
                  │   ├─ 表头行 h-8（"提交文件" + 计数 + 打开差异按钮）                     :106-130
                  │   └─ GitCommitFileTree (git-commit-file-tree.tsx:138)                :148-159
                  │      → SidebarTree / SidebarTreeRow（复用侧栏树行）                     :177-201, 57-84, 107-134
                  ├─ ResizableHandle                                                    :163
                  └─ ResizablePanel id="details" defaultSize="38" minSize={80}          :164
                     └─ div.h-full.overflow-auto.border-border.bg-background.p-3         :165
   ├─ GitRemoteManager（模态）                                                          :709-714
   └─ Dropdown（空白区右键菜单，单项 `ui.noActionsHere` 且 disabled）                    :715-727
```

标签切换是**本地 `useState<"log" | "console">`**，不是全局状态：`windows/tauri/src/features/git/components/log/git-log-tool-window.tsx:80`。`fetchReferences()` 会主动把 `panel` 切到 `"console"`：同文件 `:452`。

### 1.4 控制台（Console）子组件树

```
GitExecutionConsole                         windows/tauri/src/features/git/components/log/git-execution-console.tsx:15
└─ div.flex.min-h-0.flex-1.font-mono.text-xs（onKeyDown 捕获 Ctrl/Cmd+F）              :68-70
   ├─ div.flex.w-8.shrink-0.flex-col.items-center.gap-1.border-r.py-1
   │   （子按钮统一 size-6 rounded hover:bg-accent，图标 size-3.5）                     :71
   │   ├─ 查找 (git.console.find)                                                      :72
   │   ├─ 自动换行 (git.console.wrap)                                                  :73
   │   ├─ 滚动到底部 (git.console.scrollToEnd)                                          :74
   │   ├─ 取消运行中的 Git (git.console.cancel) → invoke("core_cancel")                 :75-78
   │   ├─ 清空 (git.console.clear)                                                     :79
   │   └─ 复制输出 (git.console.copy)                                                  :80-90
   ├─ div.flex.min-h-0.min-w-0.flex-1.flex-col                                         :92
   │   ├─ 查找条（条件）div.flex.items-center.gap-2.border-b.px-3.py-1                  :93-101
   │   ├─ 压缩不可用提示                                                                :102
   │   └─ div[ref=scroll].min-h-0.flex-1.overflow-auto.p-3                            :103-111
   │      ├─ 历史截断提示 / 空态提示                                                     :104-105
   │      ├─ div（wrapsLines ? whitespace-pre-wrap break-words : w-max min-w-full whitespace-pre） :106
   │      │   └─ GitConsoleEntry × N  (git-console-entry.tsx:9)                        :107-108
   │      └─ div[ref=bottom]（自动滚动锚点）                                             :110
```

`GitConsoleEntry`（`windows/tauri/src/features/git/components/log/git-console-entry.tsx:9`）：
- 每条记录是 `ContextMenu > ContextMenuTrigger.mb-1.leading-5`（`:49-50`）。
- 命令行前缀：`<时间戳>: [<仓库标签>] git ` 走 `text-info`（`:51-52`）。
- 参数片段/折叠按钮：`inline bg-success/10 text-foreground`（`:57`, `:64`）。
- stdout 行走 `text-foreground`，stderr 行走 `text-destructive`，命中搜索的行加 `bg-warning/20`（`:43`）。
- 右键菜单四项 + 复制：`git.console.copyPath / copyCommand / copyOutput / details / copy`（`:96-107`）。

### 1.5 变更列表（左侧"源代码管理"）组件树

变更列表**不在底部窗里**，它是左侧栏的一个视图。

```
GitView                                      windows/tauri/src/features/git/components/git-view.tsx:82
└─ SidebarPanel.font-sans.ui-text-sm.select-none                                     :706
   ├─ SidebarTitleBar title={workbench.sourceControl} → 刷新按钮 + 更多按钮               :707-710
   ├─ [showLoadError] 错误条 div.flex.items-center.justify-between.gap-2.p-3.ui-text-sm.text-destructive  :678-685
   ├─ SidebarTabBar items=[changes, history]                                          :712
   │  └─ div.flex.min-h-0.flex-1.flex-col.overflow-hidden.isolate                      :713
   │     ├─ SidebarTabPanels                                                          :714-766
   │     │  ├─ tab "changes" (workbench.changes)                                      :717-753
   │     │  │  ├─ GitOperationBanner                                                  :721
   │     │  │  └─ GitStatusPanel  (status/git-status-panel.tsx, default export :1387)
   │     │  └─ tab "history" (git.history) → GitCommitHistory                          :754-764
   │     └─ SidebarFooter  (mx-2 mb-2 rounded-xl border border-border/60 bg-background) :768-788
   │        └─ GitCommitPanel  (git-commit-panel.tsx)                                  :769-787
   ├─ GitActionsMenu                                                                  :793
   ├─ GitCommandSurface × 3（提交差异选择 / 分支比较选择 / 贮藏列表）                       :794-982
   ├─ GitRemoteManager                                                                :984-989
   └─ GitTagManager                                                                   :991-997
```

```
GitStatusPanel                              windows/tauri/src/features/git/components/status/git-status-panel.tsx:1086
└─ div.flex.h-full.min-h-0.flex-col.select-none                                     :1087-1104
   ├─ [hasFiles] SidebarToolbar  h-(--lithe-pane-header-height)=36px                  :1107
   │  ├─ 左：ButtonGroup["查看差异"(git.viewDiff) | caret] + Dropdown（差异来源）        :1108-1142
   │  └─ 右：SidebarHeaderIconButton × 3（贮藏全部未暂存 / 暂存全部 / 取消暂存全部）        :1144-1181
   ├─ ScrollArea(className="min-h-0 flex-1", contentClassName="px-2 py-2")            :1183-1188
   │  └─ SidebarTree  role="tree"  --file-tree-row-height = presentation.rowHeight     :1189-1199
   │     └─ 虚拟行 div.absolute.inset-x-0.top-0（height=virtualRow.size, translateY）    :1200-1216
   │        ├─ 分类头 SidebarSectionHeader variant="surface"（tracked / untracked）       :765-776, :858-864
   │        ├─ GitFileItem  (status/git-status-file-item.tsx:92)                       :866-895
   │        └─ 文件夹行 SidebarTreeRow + Checkbox                                        :906-970
   └─ [无文件] Empty tone="success" → git.workingTreeClean                              :1221-1226
```

`GitFileItem`（`windows/tauri/src/features/git/components/status/git-status-file-item.tsx:92-183`）：
- `SidebarTreeRow`，`containerClassName="group/git-status-row"`，`className="h-full overflow-clip py-0.5"`，`style={{height: rowHeight}}`（`:126-128`）。
- 文件名按状态着色（`getWorkingTreeStatusColorClassName`，`:132`）。
- `action` 槽：`GitFileStageAction`（`variant="ghost" size="icon-xs"`，`className="size-5 opacity-0 group-hover/git-status-row:opacity-100 group-focus-within/git-status-row:opacity-100"`，`:62-68`）+ `Checkbox`（`:156-165`）。
- 可拖拽：`draggable` + `writeSidebarResourceDragData({type:"git-file-diff",...})`（`:168-179`）。

### 1.6 状态栏（Footer）与标题栏里的 Git 元素

```
Footer                                      windows/tauri/src/features/layout/components/footer/footer.tsx:16
└─ ChromeBar region="footer" className="lithe-footer-bar relative z-20 justify-between gap-2 bg-surface"
   高 h-(--lithe-footer-height) = 24px                                                :44-49
   ├─ ChromeGroup(grow)：ProjectPreparationStatus.compact + 文件路径项 + 其它 leading 项  :50-68
   └─ ChromeGroup(align="end")：trailing 项（编辑器状态 + gitChanges 计数 chip）          :70-76
```

- `gitChanges` chip：`FooterStatusChip`，`h-(--lithe-chrome-control-height)`=24px、`max-w-50`=200px、`rounded-md`、`px-1.5`、`ui-text-chrome`；点击 `openSidebarView("git")`；文案 `footer.noChanges` / `footer.change` / `footer.changes`。证据：`windows/tauri/src/features/layout/components/footer/footer-editor-status.tsx:122-141`、`windows/tauri/src/features/layout/components/footer/footer-status-chip.tsx:4-5`。
- **Git 分支项不在状态栏，在标题栏**：`useFooterGitBranchItem()` 由标题栏消费（`windows/tauri/src/features/window/components/title-bar/title-bar.tsx:66`, `:238`），内部渲染 `GitBranchManager triggerSurface="footer"`（`windows/tauri/src/features/layout/components/footer/footer-git-branch-item.tsx:29-68`）。
- `ChromeBar` 高度系由 `chrome.tsx` 定义的 region class 决定：`windows/tauri/src/ui/chrome.tsx:13`。

### 1.7 Diff 区域（变更列表"打开差异"的落点）

`DiffViewer` 是编辑器缓冲内容（不在底部窗内）：
- `windows/tauri/src/features/git/components/diff/git-diff-viewer.tsx:16-82`：多文件 → `GitDiffEditorStack`（`:28`）；加载中 `Empty`+`Spinner`（`:31-39`）；错误 `Empty tone="error" role="alert"`（`:41-47`）；图片 → `ImageDiffViewer`（`:60`）；二进制 → `BinaryDiffViewer`（`:63-69`）；文本 → `GitDiffEditorSurface`（`:72-80`）。
- 头部 `windows/tauri/src/features/git/components/diff/git-diff-header.tsx:87-180`：`sticky top-0 z-10 border-border border-b` + `Breadcrumb`；状态徽章 `rounded-full px-1.5 py-0.5 font-medium ui-text-sm capitalize leading-none`（`:72-79`）；`+N / -N` 统计（`:54-55`）；右侧按钮组含 `git.expandAll / git.collapseAll / git.diff.showWhitespace / git.unifiedView / git.splitView / git.close`（`:107-176`）。
- Hunk 头 `windows/tauri/src/features/git/components/diff/git-diff-hunk-header.tsx:91-161`：`grid grid-cols-[2.75rem_minmax(0,1fr)]`（左 gutter **44px**），高度 = `lineHeight = calculateLineHeight(editorFontSize*zoom, editorLineHeight)`（`:39-49`），暂存/取消暂存按钮 `rounded-md px-1 py-0 opacity-0 group-hover:opacity-100`（`:143-158`）。

---

## 2. 度量表

> 未特别注明者，度量取自 Tailwind class 并按 §0 基准换算为 px。

### 2.1 底部窗（BottomPane）

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 默认高度 | `useState(320)` | **320** | `features/layout/components/bottom-pane/bottom-pane.tsx:47` |
| 拖拽上限/下限 | `Math.min(Math.max(startHeight + deltaY, 200), window.innerHeight * 0.8)` | 最小 **200**，最大 **80% 窗高** | `bottom-pane.tsx:122` |
| 外框高度 | `calc(${height}px + var(--lithe-workbench-gap))` | 内容高 + **4** | `bottom-pane.tsx:327` |
| 顶部拖拽条 | `h-(--lithe-workbench-gap)` | **4** | `bottom-pane.tsx:226` |
| 拖拽条指示线 | `h-px w-full`，hover/拖拽 `bg-primary` | **1** | `bottom-pane.tsx:234-239` |
| 拖拽条 hover 背景 | `hover:bg-primary/8` | — | `bottom-pane.tsx:227` |
| 内容圆角 | `rounded-xl` = `--radius-xl` | **11.2** | `bottom-pane.tsx:247` |
| 内容边框 | `border-border/70 border-t border-l` | **1**（左、上） | `bottom-pane.tsx:247` |
| 内容背景 | `bg-background` | — | `bottom-pane.tsx:247` |
| 全屏时 | `size-full rounded-none border-0 shadow-none ring-0` | 0 圆角/边框 | `bottom-pane.tsx:249` |
| 拖拽悬停环 | `ring-2 ring-primary ring-inset` | **2** | `bottom-pane.tsx:248` |
| 拖拽时遮罩 | `fixed inset-0 z-40 cursor-ns-resize` | — | `bottom-pane.tsx:332` |
| 左右侧栏最小宽 | `MIN_SIDEBAR_WIDTH = 140` | **140** | `features/layout/components/resizable-pane.tsx:18` |
| AI 聊天最小宽 | `300`（视口 <1100px 时 `220`） | **300 / 220** | `resizable-pane.tsx:19-20,53-58` |
| 侧栏分隔条宽 | `w-(--lithe-workbench-gap)` | **4** | `resizable-pane.tsx:161` |
| 侧栏玻璃岛圆角 | `rounded-xl border-border border-x` | **11.2** | `resizable-pane.tsx:203-205` |

### 2.2 Git Log 标题栏与标签行

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 标题栏容器 | `shrink-0 border-border border-b bg-surface font-sans ui-text-sm` | 字号 **13** | `features/git/components/log/git-log-title-bar.tsx:28` |
| 标题栏行高 | `flex h-8 items-center gap-2 px-2` | 高 **32**；内边距 **8**；间距 **8** | `git-log-title-bar.tsx:29` |
| 分支图标 | `size-3.5` | **14** | `git-log-title-bar.tsx:30` |
| 标题文本 | `font-medium`，文案 `workbench.gitLog` | — | `git-log-title-bar.tsx:31` |
| 引用名按钮 | `h-6 max-w-60 truncate rounded border border-border-strong/60 bg-background px-2` | 高 **24**，最大宽 **240**，圆角 **6.4**，内边距 **8** | `git-log-title-bar.tsx:32-39` |
| 图标按钮 ×3 | `Button variant="ghost" size="icon-xs"` = `size-6 p-0`，图标默认 `size-3.5` | **24×24**，图标 **14** | `git-log-title-bar.tsx:40-71`, `ui/button.tsx:27`, `ui/button.tsx:9` |
| 只读标签 | `<span className="ml-auto text-subtle-foreground">{footer.readOnly}</span>` | — | `git-log-title-bar.tsx:61` |
| 标签行容器 | `flex shrink-0 gap-4 border-b px-3 py-1 text-xs` | 行高 **16**，上下内边距 **4** → 总 **24**；字号 **12**；间距 **16**；左右内边距 **12** | `git-log-tool-window.tsx:582` |
| 标签按钮 | `role="tab"`，无独立样式（继承 `text-xs`），选中态仅由 `aria-selected` 表达 | — | `git-log-tool-window.tsx:583-586` |
| 刷新失败横幅 | `flex h-7 shrink-0 items-center gap-2 border-destructive/30 border-b bg-destructive/10 px-2 ui-text-sm` | 高 **28**，内边距 **8**，字号 **13** | `git-log-tool-window.tsx:590` |

> 注意：**标签行用裸 `text-xs`（12px）而非 `ui-text-sm`（13px）**，与标题栏、筛选行不一致；`role="tab"` 无任何视觉选中样式（`git-log-tool-window.tsx:582-586`）。

### 2.3 提交表格（GitCommitTable）

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| **行高** | `const ROW_HEIGHT = 30`（`estimateSize: () => ROW_HEIGHT`） | **30** | `features/git/components/log/git-commit-table.tsx:43,116` |
| overscan | `overscan: 14` | 14 行 | `git-commit-table.tsx:117` |
| 根容器 | `flex h-full min-h-0 flex-col bg-background font-sans ui-text-sm select-none` | 字号 **13** | `git-commit-table.tsx:199` |
| 工具行 | `flex h-8 shrink-0 items-center gap-2 border-border border-b bg-surface px-2` | 高 **32**；间距 **8**；内边距 **8** | `git-commit-table.tsx:200` |
| 筛选输入框 | `flex h-6 min-w-36 max-w-72 flex-1 items-center gap-1.5 rounded border border-border bg-background px-2` | 高 **24**；最小宽 **144**；最大宽 **288**；间距 **6**；圆角 **6.4**；内边距 **8** | `git-commit-table.tsx:201` |
| 搜索图标 | `size-3.5 shrink-0 text-subtle-foreground` | **14** | `git-commit-table.tsx:202` |
| 清除按钮 | `XIcon className="size-3"` | **12** | `git-commit-table.tsx:225` |
| 字段下拉 `<select>` | `h-6 rounded border border-border bg-background px-1.5 text-subtle-foreground` | 高 **24**；圆角 **6.4**；内边距 **6** | `git-commit-table.tsx:232` |
| 装饰开关 | `Button variant="ghost" size="icon-xs"` | **24×24**，图标 **14**（Eye/EyeSlash 未指定尺寸 → `size-3.5`） | `git-commit-table.tsx:239-249` |
| 计数 | `shrink-0 text-subtle-foreground tabular-nums`，内容 `{visible}/{total}` | — | `git-commit-table.tsx:250-252` |
| 表头行 | `flex h-6 shrink-0 items-center border-border border-b bg-surface/70 px-2 text-subtle-foreground` | 高 **24**；内边距 **8** | `git-commit-table.tsx:255` |
| 表头「提交」列 | `min-w-0 flex-1`（`git.log.commit`） | 弹性 | `git-commit-table.tsx:256` |
| 表头「作者」列 | `w-28 shrink-0`（`git.log.author`） | **112** | `git-commit-table.tsx:257` |
| 表头「日期」列 | `w-32 shrink-0 text-right`（`git.log.date`） | **128**，右对齐 | `git-commit-table.tsx:258` |
| 滚动容器 | `min-h-0 flex-1 overflow-auto [overflow-anchor:none]` + `data-scroll-container` | — | `git-commit-table.tsx:261-264` |
| 内容最小宽 | `relative min-w-130` | **520** | `git-commit-table.tsx:276` |
| 行容器 | `absolute inset-x-0 flex items-center border-border/50 border-b px-1 text-left outline-none` | 下边框 **1**；内边距 **4** | `git-commit-table.tsx:301-302` |
| 行 hover / 键盘焦点 | `hover:bg-accent/70 focus-visible:bg-accent/70` | — | `git-commit-table.tsx:302` |
| 行选中 | `isSelected && "bg-primary/22 hover:bg-primary/28"` | 透明度 22% → 28% | `git-commit-table.tsx:303` |
| 行定位 | `style={{ height: virtualRow.size, transform: translateY(virtualRow.start) }}` | — | `git-commit-table.tsx:305-308` |
| 作者单元格 | `w-28 shrink-0 overflow-clip px-2 text-ellipsis whitespace-nowrap text-subtle-foreground` | 宽 **112**，内边距 **8** | `git-commit-table.tsx:321-323` |
| 日期单元格 | `w-32 shrink-0 overflow-clip text-ellipsis whitespace-nowrap text-right font-mono text-[11px] text-subtle-foreground` | 宽 **128**，字号 **11**，等宽字体 | `git-commit-table.tsx:324-326` |
| 加载更多行 | `flex h-9 min-w-130 items-center justify-center border-border border-t`，按钮 `size="xs"` | 高 **36**；最小宽 **520**；按钮高 **24** | `git-commit-table.tsx:428-441`, `ui/button.tsx:23` |

**GitGraphRow（行内泳道图）**（`features/git/components/log/git-graph-row.tsx`）：

| 常量/元素 | 值 | px | 证据 |
| --- | --- | --- | --- |
| `ROW_HEIGHT` | `30` | **30** | `:4` |
| `LANE_GAP` | `13` | **13** | `:5` |
| `GRAPH_PADDING` | `8` | **8** | `:6` |
| 泳道配色（6 色循环） | `#55d68b` `#65a9ff` `#d77eea` `#f3aa59` `#e76c72` `#56c7cf` | — | `:7`, `:9-11` |
| svg 宽度 | `Math.max(30, row.laneCount * 13 + 8 * 2)` | 最小 **30** | `:27` |
| 边线/竖线 | `strokeWidth={1.6}`，缺失父提交 `strokeDasharray="3 2"` `opacity=0.7` | **1.6** | `:50`, `:63-65` |
| 节点圆 | `r={4.3}`，`fill="var(--background)"`，`strokeWidth={2}` | 半径 **4.3**，描边 **2** | `:69-78` |
| 标签徽章 | `max-w-28 shrink-0 rounded border px-1.5 py-0.5 font-medium text-[10px] leading-none` | 最大宽 **112**；字号 **10**；内边距 x **6** y **2**；圆角 **6.4** | `:87-88` |
| 标签配色 head | `border-sky-700/35 bg-sky-500/18 text-sky-800 dark:border-sky-400/45 dark:bg-sky-500/20 dark:text-sky-200` | — | `:16` |
| 标签配色 remote | `border-indigo-700/35 bg-indigo-500/18 text-indigo-800 dark:...` | — | `:18` |
| 标签配色 tag | `border-amber-700/40 bg-amber-500/20 text-amber-900 dark:...` | — | `:20` |
| 标签配色 branch（default） | `border-emerald-700/35 bg-emerald-500/18 text-emerald-800 dark:...` | — | `:22` |
| 提交说明 | `min-w-0 flex-1 overflow-clip text-ellipsis whitespace-nowrap text-foreground`，`title={commit.message}` | 弹性 | `:95-100` |

> 表头「提交」只占 `flex-1`，**未预留泳道图宽度**，因此表头文字与提交说明的左边缘不对齐（表头从头起算，说明在 `<svg>` 之后）。证据：`git-commit-table.tsx:256` vs `:276,320`。

### 2.4 引用树（GitReferenceTree）

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 根容器 | `flex h-full min-h-0 bg-surface/45 font-sans ui-text-sm select-none` | 字号 **13** | `features/git/components/log/git-reference-tree.tsx:804` |
| 左侧工具栏 | `flex h-full min-h-0 w-9 shrink-0 flex-col items-center overflow-hidden border-border border-r bg-surface/60 py-1` | 宽 **36**；上下内边距 **4**；右边框 **1** | `git-reference-tree.tsx:357`, `:371` |
| 工具栏按钮 | `flex size-8 shrink-0 items-center justify-center rounded-sm ... [&_svg]:size-4` | **32×32**；图标 **16**；圆角 **4.8** | `git-reference-tree.tsx:146` |
| 按钮 active | `bg-accent/70 text-amber-400` | — | `git-reference-tree.tsx:147` |
| 按钮 disabled | `disabled:opacity-30` | — | `git-reference-tree.tsx:146` |
| 分组分隔线 | `my-1 h-px w-5 shrink-0 bg-border` | 高 **1**，宽 **20** | `git-reference-tree.tsx:360` |
| 溢出「更多」按钮 | `flex size-8 ... rounded-sm`，图标 `CaretRightIcon` | **32×32** | `git-reference-tree.tsx:383` |
| 溢出 HoverCard | `side="right" align="end" sideOffset={2} className="flex w-auto items-center gap-0.5 rounded-md p-0.5"` | — | `git-reference-tree.tsx:389-393` |
| 工具栏自适应 | `TOOLBAR_VERTICAL_PADDING=8` / `TOOLBAR_ACTION_SIZE=32` / `TOOLBAR_SECTION_SEPARATOR_SIZE=9`；全展开高 = `8 + N*32 + 9` | — | `features/git/utils/git-reference-toolbar-layout.ts:1-3,13-17` |
| 右侧列 | `flex min-w-0 flex-1 flex-col` | — | `git-reference-tree.tsx:839` |
| 列表头 | `flex h-8 shrink-0 items-center border-border border-b px-2 text-subtle-foreground` | 高 **32**；内边距 **8** | `git-reference-tree.tsx:840` |
| 滚动区 | `min-h-0 flex-1 overflow-auto p-1.5` + `data-scroll-container` | 内边距 **6** | `git-reference-tree.tsx:847` |
| HEAD 行 | `mb-1 flex h-7 w-full items-center gap-2 rounded px-2 text-left font-medium` | 高 **28**；下边距 **4**；圆角 **6.4**；内边距 **8**；间距 **8** | `git-reference-tree.tsx:853` |
| HEAD 行箭头 | `<span className="text-primary">→</span>` | — | `git-reference-tree.tsx:858` |
| HEAD 分支名 | `ml-auto max-w-24 truncate text-subtle-foreground` | 最大宽 **96** | `git-reference-tree.tsx:861` |
| 分区容器 | `<div className="mb-1">` | 下边距 **4** | `git-reference-tree.tsx:871` |
| 分区头 | `flex h-6 w-full items-center gap-1.5 rounded px-1.5 text-left font-medium` | 高 **24**；间距 **6**；圆角 **6.4**；内边距 **6** | `git-reference-tree.tsx:876` |
| 分区 caret | `CaretRightIcon/CaretDownIcon className="size-3"` | **12** | `git-reference-tree.tsx:879-881` |
| 分区计数 | `ml-auto text-subtle-foreground tabular-nums` | — | `git-reference-tree.tsx:884` |
| **引用行** | `flex h-6 w-full min-w-0 items-center gap-1.5 rounded px-1.5 text-left hover:bg-accent/80` | 高 **24**；间距 **6**；圆角 **4.8** | `git-reference-tree.tsx:599` |
| 引用行缩进 | `style={{ paddingLeft: 10 + depth * 14 }}` | 基准 **10**，每级 **14** | `git-reference-tree.tsx:594`, `:603` |
| 引用行选中 | `bg-accent text-accent-foreground` | — | `git-reference-tree.tsx:600` |
| 引用行当前分支 | `font-semibold text-amber-300` | — | `git-reference-tree.tsx:601` |
| disclosure 按钮 | `flex size-3.5 shrink-0 items-center justify-center` | **14×14** | `git-reference-tree.tsx:611` |
| 占位（叶子） | `<span className="size-3.5 shrink-0" />` | **14** | `git-reference-tree.tsx:618` |
| 引用图标 | 当前 `CheckIcon size-3.5 text-amber-400`；标记 `StarIcon size-3.5 fill-amber-400 text-amber-400`；tag `TagIcon size-3.5 text-amber-400`；remote `NetworkIcon size-3.5 text-subtle-foreground`；其它 `GitBranchIcon size-3.5 text-subtle-foreground`；分组 `FolderIcon size-3.5 text-subtle-foreground` | **14** | `git-reference-tree.tsx:74-78`, `:630` |
| 「当前」徽章 | `shrink-0 rounded bg-amber-400/12 px-1 text-[10px] font-medium text-amber-300` | 字号 **10**；圆角 **6.4**；内边距 **4** | `git-reference-tree.tsx:652` |
| 空分区占位 | `h-6 pl-8 leading-6 text-subtle-foreground` | 高 **24**；左内边距 **32** | `git-reference-tree.tsx:919` |
| 上下文菜单最小宽 | `ContextMenuContent className="min-w-72"`（子菜单同） | **288** | `git-reference-tree.tsx:480`, `:495` |

### 2.5 Inspector（GitCommitInspector）

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 根容器 | `h-full min-h-0 bg-surface/35 font-sans ui-text-sm select-none` | 字号 **13** | `features/git/components/log/git-commit-inspector.tsx:96` |
| 垂直分割默认 | `files: 62` / `details: 38` | **62% / 38%** | `features/git/stores/git-log-preferences.store.ts:49-52` |
| 面板最小高 | `files` `minSize={90}`；`details` `minSize={80}` | **90 / 80** | `git-commit-inspector.tsx:104`, `:164` |
| 文件区表头 | `flex h-8 shrink-0 items-center gap-2 border-border border-b bg-surface px-2 text-subtle-foreground` | 高 **32**；间距 **8**；内边距 **8** | `git-commit-inspector.tsx:106` |
| 打开差异按钮 | `Button variant="ghost" size="icon-xs"` | **24×24** | `git-commit-inspector.tsx:113-128` |
| 空/加载/失败提示 | `flex min-h-0 flex-1 items-center justify-center text-subtle-foreground`（失败用 `text-destructive px-4 text-center`） | — | `git-commit-inspector.tsx:132-146` |
| 详情面板 | `div.h-full.overflow-auto.border-border.bg-background.p-3` + `space-y-2` | 内边距 **12**；项间距 **8** | `git-commit-inspector.tsx:165-167` |
| 提交说明 | `font-medium text-foreground` | — | `git-commit-inspector.tsx:168` |
| 描述 | `whitespace-pre-wrap text-subtle-foreground` | — | `git-commit-inspector.tsx:170` |
| 短哈希/作者/邮箱 | `font-mono text-[11px] text-subtle-foreground` | 字号 **11** | `git-commit-inspector.tsx:174` |
| 日期 | `font-mono text-[11px]` | **11** | `git-commit-inspector.tsx:178` |
| 装饰 | `text-primary` | — | `git-commit-inspector.tsx:180` |
| 完整哈希 | `break-all font-mono text-[10px] text-subtle-foreground` | 字号 **10** | `git-commit-inspector.tsx:182` |

**提交文件树（GitCommitFileTree）**

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 滚动容器 | `file-tree-container min-h-0 flex-1 overflow-auto p-1.5` | 内边距 **6** | `features/git/components/log/git-commit-file-tree.tsx:181` |
| 行高变量 | `--file-tree-row-height: ${presentation.rowHeight}px` | 见下 | `git-commit-file-tree.tsx:184` |
| 行高计算 | `max(24, uiFontSize * 1.35 + 6)`，默认字号 13 → `max(24, 23.55)` | **24** | `features/file-explorer/lib/file-tree-row.ts:1-13` |
| 行基准缩进 | `FILE_TREE_BASE_INDENT = 10` | **10** | `features/file-explorer/lib/file-tree-row.ts:1` |
| 行缩进步长 | `presentation.indentSize`（设置项 `fileTreeIndentSize`） | 默认见设置，未在本文核定 | `features/file-explorer/hooks/use-file-tree-presentation.ts:17` |
| 目录行尾计数 | `pr-1 text-subtle-foreground tabular-nums` | — | `git-commit-file-tree.tsx:77` |
| 文件状态文本 | `pr-1 font-mono text-[10px]` + 状态色 | 字号 **10** | `git-commit-file-tree.tsx:127` |
| 叶子行 | `className="h-full py-0.5"`，`style={{height: presentation.rowHeight}}` | — | `git-commit-file-tree.tsx:132-133` |

**SidebarTreeRow（被文件树/变更列表共用的行原语）**（`windows/tauri/src/features/sidebar/components/sidebar-tree.tsx:183-281`）

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 行按钮 | `flex w-full min-w-0 flex-1 ... rounded-(--lithe-chrome-radius) border border-transparent ... gap-1.5 px-1.5 py-1 leading-row` | 圆角 **4**；内边距 x **6** y **4**；间距 **6**；行高系数 **1.35** | `sidebar-tree.tsx:241` |
| 行 hover | `hover:bg-accent` | — | `sidebar-tree.tsx:241` |
| 行选中 | `active && "bg-selected"` | — | `sidebar-tree.tsx:242` |
| 行左内边距 | `paddingLeft: baseIndent + depth * indentSize` | 10 + depth×N | `sidebar-tree.tsx:246` |
| 行默认 `role` | `role="treeitem"`，`aria-level={depth+1}`，`aria-selected={active}`，默认 `tabIndex=-1` | — | `sidebar-tree.tsx:231-239` |
| disclosure | `mr-0.5 flex size-4 shrink-0 items-center justify-center rounded`，caret `size-3` | **16×16**，图标 **12** | `sidebar-tree.tsx:301`, `:314-317` |
| action 槽 | `relative z-3 flex shrink-0 items-center px-2` | 内边距 **8** | `sidebar-tree.tsx:276` |
| 行背景（CSS） | `--file-tree-hover-bg: color-mix(in srgb, var(--accent) 68%, transparent)`；`[data-active="true"]` 用 `var(--selected)` | — | `windows/tauri/src/features/sidebar/styles/sidebar-tree.css:5`, `:33-35` |
| 缩进参考线 | 宽 `7px`，`left: calc(base + level*size + 7px)`，竖线 `left:3px; width:1px`，色 `color-mix(in srgb, var(--subtle-foreground) 18%, transparent)` | — | `sidebar-tree.tsx:38`；`sidebar-tree.css:7`, `:44-59` |
| 默认缩进参数 | `SIDEBAR_TREE_BASE_INDENT=10`，`SIDEBAR_TREE_INDENT_SIZE=14` | **10 / 14** | `sidebar-tree.tsx:7-8` |

### 2.6 控制台（GitExecutionConsole）

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 根容器 | `flex min-h-0 flex-1 font-mono text-xs` | 字号 **12**，等宽 | `features/git/components/log/git-execution-console.tsx:68` |
| 左侧按钮栏 | `flex w-8 shrink-0 flex-col items-center gap-1 border-r py-1` | 宽 **32**；间距 **4**；上下内边距 **4**；右边框 **1** | `git-execution-console.tsx:71` |
| 栏内按钮 | `[&>button]:flex [&>button]:size-6 [&>button]:items-center [&>button]:justify-center [&>button]:rounded [&>button:hover]:bg-accent [&>button:disabled]:opacity-40` | **24×24**；圆角 **6.4** | `git-execution-console.tsx:71` |
| 栏内图标 | `className="size-3.5"` | **14** | `git-execution-console.tsx:72-90` |
| 查找条 | `flex items-center gap-2 border-b px-3 py-1` | 上下内边距 **4**；左右 **12**；间距 **8** | `git-execution-console.tsx:93` |
| 查找输入 | `min-w-0 flex-1 bg-transparent`，`maxLength={256}`，`autoFocus` | — | `git-execution-console.tsx:94-96` |
| 输出区 | `min-h-0 flex-1 overflow-auto p-3` | 内边距 **12** | `git-execution-console.tsx:103` |
| 换行关闭 | `w-max min-w-full whitespace-pre` | — | `git-execution-console.tsx:106` |
| 换行开启 | `whitespace-pre-wrap break-words` | — | `git-execution-console.tsx:106` |
| 底部跟随阈值 | `scrollHeight - scrollTop - clientHeight < 32` | **32** | `git-execution-console.tsx:103` |
| 条目间距 | `mb-1 leading-5` | 下边距 **4**；行高 **20** | `git-console-entry.tsx:50` |
| 命令行 | `text-info` | — | `git-console-entry.tsx:51` |
| 可折叠参数按钮 | `inline cursor-pointer border-0 bg-success/10 p-0 font-[inherit] text-foreground focus-visible:outline focus-visible:outline-1` | — | `git-console-entry.tsx:57` |
| 选项按钮（未压缩时） | `<button ...>{expandedConfiguration ? configuration : "-c …"}</button>` | — | `git-console-entry.tsx:64-66` |
| 折叠提示 | `text-subtle-foreground`，前缀 `▾ ` / `▸ ` | — | `git-console-entry.tsx:71-73` |
| stderr 行 | `text-destructive` | — | `git-console-entry.tsx:43` |
| stdout 行 | `text-foreground`（空行用 `\u00a0`） | — | `git-console-entry.tsx:43` |
| 搜索命中高亮 | `bg-warning/20` | — | `git-console-entry.tsx:43`, `:52`, `:55`, `:77`, `:80` |
| 进度行 | `text-subtle-foreground` | — | `git-console-entry.tsx:77` |
| 截断/通知 | `text-subtle-foreground`（通知带 `role="status"`） | — | `git-console-entry.tsx:78-79` |
| 错误行 / 未确认 / 非零退出 | `text-destructive` | — | `git-console-entry.tsx:80-82` |
| 详情展开块 | `text-subtle-foreground`，行内容 `状态 · N ms · 退出码 N`，等宽字体继承 12px | — | `git-console-entry.tsx:83-93` |

### 2.7 变更列表（GitStatusPanel）

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 工具行 | `SidebarToolbar` = `h-(--lithe-pane-header-height)` + `border-border/70 border-b bg-background px-3` | 高 **36**；内边距 **12**；下边框 **1** | `windows/tauri/src/ui/sidebar.tsx:57`；`features/git/components/status/git-status-panel.tsx:1107` |
| 「查看差异」按钮组 | `Button size="xs"`（`h-6 gap-1 px-1.5`）+ `Button size="icon-xs"`（`size-6`） | 高 **24** | `git-status-panel.tsx:1109-1134`, `ui/button.tsx:23,27` |
| caret 图标 | `CaretDown className="size-3"` | **12** | `git-status-panel.tsx:1132` |
| 差异来源下拉 | `className="min-w-37.5"` | **150** | `git-status-panel.tsx:1141` |
| 工具行右侧图标按钮 | `SidebarHeaderIconButton` = ghost + `icon-xs` + `rounded-md`；禁用 `disabled:opacity-50` | **24×24**；圆角 **6.4** | `git-status-panel.tsx:1146-1180`；`ui/sidebar.tsx:132-139` |
| 滚动区 | `ScrollArea className="min-h-0 flex-1" contentClassName="px-2 py-2"`，`reserveScrollbarGutter` | 内边距 x **8** y **8** | `git-status-panel.tsx:1183-1188` |
| 树容器行高变量 | `--file-tree-row-height: ${presentation.rowHeight}px`，`height: statusVirtualizer.getTotalSize()` | 行高默认 **24** | `git-status-panel.tsx:1193-1198` |
| 虚拟行 | `absolute inset-x-0 top-0`，`height: virtualRow.size`，`translateY(start)` | — | `git-status-panel.tsx:1204-1211` |
| **分类头（surface 变体）** | `SidebarSectionHeader variant="surface"`：`h-8 rounded-lg bg-accent/80 px-2.5 hover:bg-accent` + `ui-text-chrome` | 高 **32**；圆角 **8**；内边距 **10** | `ui/sidebar.tsx:327-330`；`git-status-panel.tsx:765-776` |
| 分类头计数 Badge | `<Badge variant="muted" size="compact">` | — | `ui/sidebar.tsx:344-348` |
| 文件夹行尾计数 | `shrink-0 text-[10px] leading-none text-subtle-foreground`，文案 `git.diffFileCount` | 字号 **10** | `git-status-panel.tsx:925-931` |
| 暂存/取消暂存按钮 | `Button variant="ghost" size="icon-xs" className="size-5 opacity-0 group-hover/git-status-row:opacity-100 ..."` | **20×20**，默认透明 | `features/git/components/status/git-status-file-item.tsx:62-68` |
| pending 旋转指示 | `size-3 animate-spin rounded-full border-2 border-current border-r-transparent` | **12** | `git-status-file-item.tsx:78-82` |
| 文件行 | `h-full overflow-clip py-0.5` + `style={{height: rowHeight}}` | — | `git-status-file-item.tsx:127-128` |
| 文件状态色 | modified → `text-git-modified`；untracked → `text-git-untracked`；D → `text-subtle-foreground`；其余见文件 | — | `features/git/utils/git-file-status-visuals.ts:10`, `:21` |
| 空态 | `Empty tone="success"` + `EmptyMedia variant="icon"` + `EmptyTitle` → `git.workingTreeClean` | — | `git-status-panel.tsx:1221-1226` |
| 上下文菜单最小宽 | 未在 `Dropdown.items` 路径指定（`Dropdown` 默认宽度，未找到显式值） | 未找到 | `git-status-panel.tsx:1229-1240` |

**提交面板（SidebarFooter 内）**

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 底栏容器 | `SidebarFooter`：`ui-text-chrome relative z-20 isolate mx-2 mb-2 shrink-0 rounded-xl border border-border/60 bg-background p-0 pb-1` | 下/左右外边距 **8**；圆角 **11.2** | `ui/sidebar.tsx:75` |
| 编辑器外壳 | `SidebarComposerBody variant="surface"`：`relative z-10 rounded-xl border border-border/60 bg-background` | 圆角 **11.2** | `ui/sidebar.tsx:115-117` |
| 提交说明输入 | `max-h-32 min-h-16 w-full resize-none ... font-sans ui-text-sm px-3 pt-3 pb-2`，`rows={2}` | 最小高 **64**；最大高 **128**；内边距 x **12**、上 **12** 下 **8** | `features/git/components/git-commit-panel.tsx:283-288` |
| 选中文件计数行 | `flex flex-wrap items-center gap-x-2 gap-y-1 px-1 pt-1.5` + `ui-text-sm text-subtle-foreground` | 上内边距 **6**；行/列间距 **8/4** | `git-commit-panel.tsx:293-295` |
| ahead/behind 按钮 | `Button variant="ghost" size="xs"`，ahead `text-git-added`，behind `text-git-deleted` | 高 **24** | `git-commit-panel.tsx:306-333` |
| 错误块 | `mx-2 mt-2 flex items-center gap-2 rounded-md border border-destructive/30 bg-destructive/20 px-2 py-1 ui-text-sm text-destructive` | 外边距 **8**；圆角 **6.4**；内边距 **8/4** | `git-commit-panel.tsx:266-269` |

### 2.8 引用/分支 ahead-behind 计数（共用组件）

`windows/tauri/src/features/git/components/git-tracking-counts.tsx:36-53`

| 元素 | class / 值 | px |
| --- | --- | --- |
| 容器 | `inline-flex shrink-0 items-center gap-1 whitespace-nowrap text-[10px] leading-none tabular-nums` | 字号 **10**；间距 **4**；行高 **1** |
| behind | `text-info`，前缀 `↙` | — |
| ahead | `text-git-added`，前缀 `↗` | — |
| 最大值 | `MAX_VISIBLE_TRACKING_COUNT = 99` → 显示 `99+` | `:3`, `:9-14` |
| 隐藏 | ahead 与 behind 均为 0 时返回 `null` | `:33` |

### 2.9 变更/源控侧栏标题与标签

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 侧栏面板 | `flex h-full min-h-0 min-w-0 w-full flex-col bg-background` | — | `ui/sidebar.tsx:18` |
| 标题栏 | `flex h-(--lithe-pane-header-height) ... px-3`，标题 `ui-text-lg font-medium truncate pl-2` | 高 **36**；左内边距 **12**（标题再 +**8**） | `ui/sidebar.tsx:38,43` |
| 标题栏动作区 | `flex max-w-[50%] shrink-0 items-center gap-1` | 最大宽 50%；间距 **4** | `ui/sidebar.tsx:47` |

---

## 3. 状态与交互

### 3.1 Git Log 面板切换
- `panel: "log" | "console"` 为**局部** `useState`，切换发生在 `windows/tauri/src/features/git/components/log/git-log-tool-window.tsx:80`，两个按钮 `:584-585`。
- 标签行只有 `aria-selected`，**无选中视觉样式**（`:582-587`）。
- `fetchReferences()` 主动 `setPanel("console")`（`:452`）。

### 3.2 提交表格选择（多选）
- 选择状态：`selectedCommitHashes: Set<string>` + `selectionAnchorRef`（`git-log-tool-window.tsx:99`, `:104`）。
- 单击：`onSelect(commit, visibleCommitHashes, {additive: ctrlKey||metaKey, range: shiftKey})`（`git-commit-table.tsx:309-314`）。
- 选择算法在 `updateGitHistorySelection`（`features/git/utils/git-history-selection.ts`，由 `git-log-tool-window.tsx:42-46` 导入）；`selectedCommits` 按历史顺序归一（`git-log-tool-window.tsx:119-122`）。
- 右键：先 `onContextSelect` 补齐上下文选择（`git-commit-table.tsx:316`；`git-log-tool-window.tsx:203-208`）。
- 选中高亮：`bg-primary/22` + `hover:bg-primary/28`（`git-commit-table.tsx:303`）。
- 单选/多选的分支：`contextSelection.length > 1` 时右键菜单只保留「压缩提交 / 创建补丁」（`:329-351`），否则展示完整菜单（`:352-421`）。
- 「压缩」要求连续选择：`isContiguousGitHistorySelection`（`:285-288`），否则 `disabled` + `title=git.historyReview.contiguousRequired`（`:332-335`）。
- 键盘：`ArrowDown/ArrowUp`（带 ctrl/meta 加选、shift 连选）、`Home/End`、`Enter` 打开差异（`:164-196`）；`tabIndex` 仅选中行（或首行）为 0（`:293-298`）。
- 自动滚动到选中项：`virtualizer.scrollToIndex(selectedIndex, {align:"auto"})`（`:145-149`）；选中后 `requestAnimationFrame` 内聚焦该行（`:157-161`）。
- 滚动容器绑定自定义滚轮：`bindScrollContainerWheel(element)`（`:120-124`）。

### 3.3 提交表格悬停与焦点
- 行 hover / `focus-visible`：`bg-accent/70`（`git-commit-table.tsx:302`）。
- 行 `title` 提示：`git.log.openDiffHint`（`git-commit-table.tsx:318`）。
- 双击打开提交差异：`onDoubleClick={() => onOpenDiff(row.commit)}`（`:315`）。
- 空白区右键：`handleEmptyContextMenu` 打开只有一项 `ui.noActionsHere`（disabled）的 `Dropdown`；若点在 `[data-slot="context-menu-trigger"]` 上则忽略（`git-log-tool-window.tsx:557-561`, `:715-727`）。

### 3.4 筛选与分页加载
- 筛选：`filterQuery` + `filterScope: "text" | "author" | "branch"`（`features/git/stores/git-log-preferences.store.ts:8`, `:15-16`），过滤函数 `matchesGitLogCommit`（`git-commit-table.tsx:105-108`）。
- 占位文案随 scope 变化：`git.log.filterPlaceholder` + `git.log.filterText|filterAuthor|filterBranch`（`git-commit-table.tsx:207-215`）。
- 段落 `<select>` 选项同上（`:229-238`）。
- 计数 `{visibleRows.length}/{commits.length}`（`:250-252`）。
- 无匹配 / 无提交占位：`git.log.noMatch` / `git.log.noCommits`（`:266-273`）。
- 分页：`IntersectionObserver` 监听底部哨兵，`rootMargin: "0px 0px 240px 0px"`，触发 `onLoadMore`（`:126-143`）；同时提供显式「加载更多提交」按钮（`h-9` 行，`:428-443`）。
- 滚动内容强制 `min-w-130`（520px），保证横向不塌缩（`:276`, `:431`）。

### 3.5 装饰开关
- `showDecorations` 布尔，默认 `true`（`git-log-preferences.store.ts:63`）。
- 开关按钮 `aria-pressed={showDecorations}`，图标随状态在 `Eye` / `EyeSlash` 间切换，tooltip 在 `git.log.showDecorations` / `git.log.hideDecorations` 间切换（`git-commit-table.tsx:239-249`）。
- 关闭后不渲染 `row.labels` 徽章（`git-graph-row.tsx:82-94`）。

### 3.6 引用树交互
- 选择单引用：点击行按钮 `onSelect(node.reference)`；分组行点击切换折叠（`git-reference-tree.tsx:620-628`）。
- 折叠状态持久化：`collapsedReferenceSections: GitReferenceKind[]` 与 `collapsedReferenceGroups: string[]`（`git-log-preferences.store.ts:21-22`, `:67-68`）。
- 展开/折叠全部：工具栏 footer 区两个按钮 → `setReferenceExpansion([], [])` / `setReferenceExpansion(全部 kind, allReferenceGroupIds)`（`git-reference-tree.tsx:831-837`）。
- 右键：行右键会**先选中**该引用（`:604-606`，`onContextMenu` → `onSelect`），再展示 `ReferenceActionMenu`（`:667-681`）。
- 右键菜单**按引用类型分组**，组间 `ContextMenuSeparator`：当前分支 4 组、远程分支 6 组、本地分支 6 组；见 `git-reference-tree.tsx:429-452`。每组只渲染该类型允许的 action（`getGitReferenceActions`，`:427`, `:482`）。
- 「跟踪的分支」为子菜单（`ContextMenuSub`），列出全部远程分支 + 「停止跟踪分支」，自身被禁用以示当前 upstream（`:488-529`）。
- 危险项（`deleteLocal` / `deleteRemote`）用 `variant="destructive"`（`:531`, `:545`）。
- 禁用规则：`isMutating`、pull 锁定、`checkoutAndUpdate` 无 upstream、`update` 条件不成立（`:532-540`）。
- 工具栏动作可用性来自 `getGitReferenceToolbarState(selectedReference, currentReference, isMutating)`（`:206`）。
- 工具栏溢出：`ResizeObserver` 监听高度 → `getVisibleGitReferenceToolbarActionCount(height, actions.length)`，溢出动作进右侧 HoverCard（`:314-331`, `:333-335`, `:368-406`）。
- 「我的分支」过滤开关 `showMyBranchesOnly`（默认 `false`，`git-log-preferences.store.ts:64`），过滤函数 `filterGitLogReferences`（`:756-762`）。
- 「标记」按仓库持久化：`markedReferenceFullNamesByRepository[normalizeRepositoryPath(repoPath)]`（`git-log-preferences.store.ts:23`, `:87-103`）。
- 标记图标：当前 → `CheckIcon text-amber-400`；已标记 → `StarIcon fill-amber-400 text-amber-400`（`git-reference-tree.tsx:74-75`）。
- 「导航到所选分支 HEAD」：清空筛选、选中 `history.commits[0]`、设置锚点、`setSelectedCommit(head)`（`git-log-tool-window.tsx:149-157`）；可用条件 `loadState==="ready" && commits.length>0 && selectedReference!==null && kind!=="tag"`（`:639-644`）。
- 远程分区头右键：只有「管理远程...」（`git-reference-tree.tsx:888-895`）。
- 空白/无 action 引用的右键菜单只有一项 disabled 的 `ui.noActionsHere`（`:677-681`）。

### 3.7 Git Log 面板宽度拖拽（两处 Resizable）
- 三栏：`references(19%) | commits(57%) | inspector(24%)`，默认来自 `git-log-preferences.store.ts:43-47`；`defaultLayout={mainPanelLayout}`，仅当 `meta.isUserInteraction` 时写回持久化（`git-log-tool-window.tsx:614-621`）。
- 各栏 `minSize`：`references 140`、`commits 320`、`inspector 220`（`:622`, `:648`, `:684`）。
- Inspector 内部竖向：`files 62% | details 38%`（`git-log-preferences.store.ts:49-52`），`minSize` 分别 90 / 80（`git-commit-inspector.tsx:104`, `:164`）。
- 持久化 key：localStorage `"git-log-preferences"`（`git-log-preferences.store.ts:132`）。

### 3.8 控制台交互
- 查找：`Ctrl/Cmd+F` 打开查找条（`git-execution-console.tsx:69`）；`Enter` 下一个、`Shift+Enter` 上一个、`Escape` 关闭并清空（`:96`）；命中计数与上限提示 `git.console.searchLimit`（`:97`）。
- 搜索高亮：命中的文本片段包 `bg-warning/20`；折叠片段若包含命中会自动展开（`git-console-entry.tsx:21-24`）。
- 自动跟随输出：`followsOutput.current`，记录变化时 `scrollIntoView({block:"end"})`（`git-execution-console.tsx:52`）；用户滚动后被清掉（`:103`），点「滚动到底部」恢复（`:74`）。
- 取消：对 `active` 集合逐个 `invoke("core_cancel", {operationId})`（`:75-78`）。
- 清空：`clearGitConsole()`（`:79`）。
- 复制全部：`redactConsoleText(...)` 后写剪贴板（`:80-90`）。
- 条目右键：复制仓库路径 / 复制完整命令 / 复制完整输出 / 命令详情 / 复制输出（`git-console-entry.tsx:96-107`）。
- 折叠：参数片段与输出片段分别用 `bg-success/10` 按钮与 `▸/▾` 文本按钮展开（`:56-61`, `:71-74`）。
- 表达层由 Core 命令 `git.consolePresentation` 计算（`:31-36`），失败时显示 `git.console.presentationUnavailable`（`:102`）。

### 3.9 变更列表交互
- 选中：`selectedEntryIds: Set<string>`，单击 `event.ctrlKey || event.metaKey` 决定是否加选（`git-status-panel.tsx:732-736`）。
- 右键：`resolveGitStatusContextSelection` 补齐选择后开 `Dropdown`（`:738-742`）。
- 复选框：按文件（`git-status-file-item.tsx:156-165`）与按文件夹（全后代路径，`git-status-panel.tsx:944-957`）切换 `commitSelectedPaths`。
- 文件夹折叠：key 为 `${section}:${folderPath}`（`:744-753`）；分区折叠 key 为 `StatusSection`（`:755-763`）。两者都持久化到 `sourceControlSessions[repoPath]`（`git-view.tsx:147-154`, `:727-733`）。
- 键盘树导航：`ArrowDown/Up`、`Home/End`、`ArrowRight`（展开或下探子行）、`ArrowLeft`（折叠或回父行）（`git-status-panel.tsx:797-854`）。
- `Delete` 键：在 `[role="treeitem"]` 上且非加载/非暂存中且有选择时删除（`:1090-1103`）。
- 悬停显示暂存按钮：`group-hover/git-status-row:opacity-100`（`git-status-file-item.tsx:68`）。
- 拖拽：文件行与文件夹行都调用 `writeSidebarResourceDragData`（`:169-179`；`git-status-panel.tsx:959-967`）。
- 差异来源下拉：`all`（按钮直点）/ `unstaged` / `staged` / 分隔线 / `commit` / `branch` / `stash`（子选择），见 `git-status-panel.tsx:1031-1079`、`:1114`。
- 「贮藏全部未暂存」与「暂存全部/取消暂存全部」按钮按存在性条件渲染（`:1145-1180`）。
- 多仓贮藏被拒绝并 toast `git.selectSingleRepositoryForStash`（`:675-678`）。
- 上下文菜单项随状态变化：暂存/取消暂存、创建补丁（多仓时 `git.patch.singleRepository` 且禁用）、提交、回滚、显示差异、跳转到源、删除（`text-destructive`）、添加到 VCS、添加到 .gitignore / .git/info/exclude、贮藏（`:1242-1367`）。

### 3.10 底部窗交互
- 高度拖拽：`mousedown` → 记录 `startY/startHeight`，`mousemove` 用 `requestAnimationFrame` 写 `frameEl.style.height`，`mouseup` 提交到 state；期间 `document.body.style.cursor = "ns-resize"`、`userSelect = "none"`，并显示 `fixed inset-0 z-40 cursor-ns-resize` 遮罩（`bottom-pane.tsx:109-154`, `:332`）。
- 标签拖放：接受 `application/tab-data` 或内部 tab 拖拽数据；终端面板拖入会 `openTerminalBuffer` + `terminal-detach-to-buffer` 事件；编辑器缓冲拖入会 `moveBufferToPane(..., BOTTOM_PANE_ID)`；结束后切到 `buffers` 标签并显示底部窗（`:156-220`）。
- 悬停高亮：内部拖拽悬停到本面板时加 `ring-2 ring-primary ring-inset`（`:50`, `:55-59`, `:248`）。
- 侧栏分隔条：`role="separator" aria-orientation="vertical"`，独立 overlay 滚轮绑定 `bindOverlayWheelToScrollContainer`（`features/layout/components/resizable-pane.tsx:93-100`, `:161-170`），`aria-valuenow/min/max` 用当前/最小/最大宽（`:167-169`）。
- 底部窗分隔条：`role="separator" aria-orientation="horizontal" aria-label={layout.resizeBottomPane}`（`bottom-pane.tsx:230-232`）——注意横向分隔条**没有** `aria-valuenow/min/max`。

### 3.11 状态栏 Git 计数
- `countUniqueGitChanges(gitFiles)`（`features/layout/utils/footer-status.ts`），`0` 时显示 `CheckCircleIcon text-success`，否则显示 `footer.change/changes` 文案（`footer-editor-status.tsx:122-141`）。

---

## 4. 数据与命令

### 4.1 命令清单（Core 名，已在 `rust/lithe-core/src/protocol/command.rs` 核对）

以下全部在 `rust/lithe-core/src/protocol/command.rs:401-440` 有 `parse` 分支，并在 `shared/contracts/rust-core-api.md:169-205` 有契约条目：

| Core 命令 | 契约说明（`rust-core-api.md` 行） | 本区域用途 |
| --- | --- | --- |
| `git.status` | `:172` | 变更列表数据（[`GitStatus`]） |
| `git.watchContext` | `:173` | 原生 watcher 元数据根 |
| `git.worktrees` | `:174` | 工作树列表 / 管理 |
| `git.pullRequestContext` | `:175` | PR 分支默认与脏状态 |
| `git.command` | `:176` | 参数式 Git 操作 |
| `git.write` | `:177`, 操作枚举 `:543-554` | 所有写操作（stage/commit/branch/checkout/remote/stash/…） |
| `git.fetchPlan` | `:178` | Fetch 对话框校验 |
| `git.consolePresentation` | `:179` | 控制台折叠/通知/搜索区间 |
| `git.remoteUrl` | `:180` | 远程 URL 静默读取 |
| `git.executionInspect` | `:181` | 可执行文件能力与配置来源 |
| `git.executionConfigure` | `:182` | 保存/清除配置项 |
| `git.authRespond` | `:183` | 认证应答 |
| `git.historyRewritePreview` | `:184` | 撤销/改消息/压缩/删除预览 |
| `git.rebasePreview` | `:185` | 交互式变基范围 |
| `git.repositorySetup` | `:169` | 仓库/unborn/身份 |
| `git.initialize` | `:170` | `git init` |
| `git.configureIdentity` | `:171` | 身份覆写 |
| `git.rebaseStart` | `:186` | 启动变基 |
| `git.rebaseSession` | `:187` | 变基会话状态 |
| `git.rebaseControl` | `:188` | continue/skip/abort |
| `git.patchExport` | 未在 `:169-205` 表格中单列（`git.patchExport` 见 `rust-core-api.md` 的 patch 章节；本表 `:169-205` 未包含）→ **表格缺项，命令名在 `protocol/command.rs:421`** | 导出补丁 |
| `git.patchPreview` | 同上（`protocol/command.rs:422`） | 补丁预览 |
| `git.patchApply` | 同上（`protocol/command.rs:423`） | 应用补丁 |
| `git.diff` | `:189` | 结构化 diff（`reference` / `pathspecs` / `untracked` 等，`:847`） |
| `git.apply` | `:190` | stage/unstage/discard/shelf 恢复 |
| `git.history` | `:191` | 旧版合并快照（`:876-879`） |
| `git.references` | `:192` | 引用 + ahead/behind + 身份（`:879-900`） |
| `git.historyPage` | `:193` | 一页提交 + 不透明 cursor（`:880-895`） |
| `git.historyCursorClose` | `:194` | 释放 cursor（`:895`） |
| `git.pushPreview` | `:195` | 推送目标与待推提交 |
| `git.commit` | `:196` | 单提交（`:917`） |
| `git.commitFiles` | `:197` | 提交改动文件 |
| `git.comparison` | `:198` | 引用 ↔ 工作树文件 |
| `git.stashes` | `:199` | 贮藏列表 |
| `git.checkoutPreflight` | `:200` | 阻塞切换的本地路径 |
| `git.pullPreflight` | `:201` | upstream / ahead-behind / 脏状态 |
| `git.integrationPreflight` | `:202` | 阻塞 merge/rebase/cherry-pick/revert |
| `git.conflictMarkers` | `:203` | 仍含冲突标记的已暂存文件 |
| `git.operationState` | `:204` | 中断的 merge/rebase/cherry-pick/revert |
| `git.blame` | `:205` | 行级 blame |

### 4.2 Tauri 包装层（前端 → Core）

**唯一入口**：`windows/tauri/src/platform/tauri-core.ts:93` 的 `invoke<T>()`。
- `:18-91` 列出**原生命令白名单**（`nativeCommands`），不在白名单且不是 `git_*`/`git.*` 的一律走 `platform_invoke`。
- `:109-124` 对 `git_` / `git.` 前缀（除 `git.consolePresentation`）注入 `operationId`、挂 `Channel<GitExecutionEvent>`、附加 `gitExecutionPreferences()` 与 `source`，并调用 `tauriInvoke("platform_invoke", {command, args, gitEvents, gitExecution})`，返回前经 `adaptCoreResult` 适配。
- `:125-127` 其它命令走 `platform_invoke`（无 git 通道）。
- 能力门禁：`capabilityForCommand`（`:134-192`）。

**Tauri host 侧分发器**：`windows/tauri/src-tauri/src/platform.rs`
- `platform_invoke` 定义在 `:9`，注册在 `windows/tauri/src-tauri/src/main.rs:128`。
- `translate(command, args)`（`:203-...`）做 snake_case 兼容名 → `git.*` Core 名 + payload 字段改写；`repoPath → root` 由 `move_field`（`:205`），`operationId` 从严格 Git payload 中移除（`:207-209`）。
- 兼容名 → Core 名映射（节选，全部见 `platform.rs:211-584`）：
  - `git_status → git.status`（`:212`）
  - `git_log`/`git_branches → git.history`（`:219`）
  - `git_references → git.references`（`:220`）
  - `git_history_page → git.historyPage`（`:221`）
  - `git_history_cursor_close → git.historyCursorClose`（`:222`）
  - `git_get_stashes → git.stashes`（`:223`）
  - `git_commit_diff → git.diff`（`commitHash→commit`，`pathspecs=["."]`，`:224-228`）
  - `git_add → git.write{operation:"stage"}`（`:229-233`）
  - `git_add_all → git.write{operation:"stageAll"}`（`:234-237`）
  - `git_reset → git.write{operation:"unstage"}`（`:238-242`）
  - `git_discard_file_changes → git.write{operation:"discard"}`（`:243-247`）
  - `git_discard_all_changes → git.write{operation:"discardAll"}`（`:248-251`）
  - `git_commit → git.write{operation:"commit"}`（`:252-255`）
  - `git_diff_file`/`git_status_diff_stats → git.diff`（`filePaths→pathspecs`，`:256-268`）
  - `git_ref_diff / git_working_tree_ref_diff / git_reference_worktree_diff / git_stash_diff → git.diff`（`:269-318`）
  - `git_create_branch`（`:319`）、`git_delete_branch`（`:339`）、`git_checkout`/`git_checkout_and_rebase`（`:345`）、`git_checkout_preflight`（`:368`）、`git_merge`/`git_rebase`（`:372`）、`git_integration_preflight`（`:384`）、`git_operation_state`（`:388`）、`git_operation_continue|abort|skip`（`:389`）、`git_conflict_markers`（`:398`）、`git_create_stash`（`:399`）、`git_apply_stash|pop_stash|drop_stash`（`:404`）、`git_discover_repo`（`:418`）、`git_discover_workspace_repos`（`:423`）、`git_fetch|pull|push`（`:427`）、`git_get_remotes`（`:434`）、`git_add_remote`（`:438`）、`git_remove_remote`（`:447`）、`git_get_tags`（`:452`）、`git_create_tag`（`:464`）、`git_delete_tag`（`:488`）、`git_push_tag`（`:493`）、`git_delete_remote_tag`（`:502`）、`git_checkout_tag`（`:517`）、`git_get_worktrees`（`:522`）、`git_add_worktree`（`:529`）、`git_remove_worktree`（`:556`）、`git_init → git.initialize`（`:562`）、`git_clone`（`:563`）、`git_reset_all`（`:580`）、`git_stage_hunk|git_unstage_hunk`（`:584`）。
- **薄前端包装文件**（`windows/tauri/src/features/git/api/`）直接调用它：
  - `git-commits-api.ts`（`git_history_page` `:174`、`git_history_cursor_close` `:199`、`git.commitFiles` `:219`、`git.write` `:52`）
  - `git-branches-api.ts`（`git.checkoutPreflight` `:79`、`git.write` `:91,131,155,175,198,213,227`）
  - `git-status-api.ts`（`git.write` `:209,330,352`）
  - `git-remotes-api.ts`（`git.write` `:94`、`git.pullPreflight` `:125`）
  - `git-worktrees-api.ts`（`git.worktrees` `:28`、`git.write` `:76,111,138`）
  - `git-push-api.ts`（`git.pushPreview` `:60`、`git.write` `:76`）
  - `git-rebase-api.ts`（`git.rebasePreview` `:16`、`git.rebaseSession` `:26`、`git.rebaseStart` `:54`、`git.rebaseControl` `:70`）
  - `git-patch-api.ts`（`git.patchExport` `:21`、`git.patchPreview` `:38`、`git.patchApply` `:54`）
  - `git-history-rewrite-api.ts`（`git.historyRewritePreview` `:17`、`git.write` `:33`）
  - `git-setup-api.ts`（`git.repositorySetup` `:19`、`git.initialize` `:23`、`git.configureIdentity` `:38`）

### 4.3 关键字段（TS 类型 ↔ 渲染位置）

```ts
GitCommit          features/git/types/git.types.ts:25-35
  hash, shortHash, parentHashes[], message, description?, author, email?, date, decorations
GitReference       features/git/types/git.types.ts:39-48
  fullName, shortName, kind("local"|"remote"|"tag"), peelsToCommit, isCurrent, upstreamShortName?, ahead?, behind?
GitHistoryPage     features/git/types/git.types.ts:90-94   { commits, nextCursor?, hasMore }
GitCommitFile      features/git/types/git.types.ts:96-99   { status, path }
GitStatus          features/git/types/git.types.ts:18-23   { branch, ahead, behind, files }
GitFile            features/git/types/git.types.ts:1-16
  path, originalPath?, repositoryPath?, repositoryRelativePath?, repositoryOriginalRelativePath?,
  status("modified"|"added"|"deleted"|"untracked"|"renamed"), staged, rawStatus?, worktree?
```

字段到 UI：`features/git/components/log/git-commit-table.tsx:321-326`（author/date）、`git-graph-row.tsx:92-100`（labels + message）、`git-reference-tree.tsx:640-656`（name + upstream + 当前徽章）、`git-commit-inspector.tsx:166-185`（详情）、`git-status-file-item.tsx:132`（文件名 + 状态色）。

### 4.4 布局持久化键

| 键 | 载体 | 默认值 | 证据 |
| --- | --- | --- | --- |
| `git-log-preferences` | localStorage（`createSafeJSONStorage`） | — | `features/git/stores/git-log-preferences.store.ts:132` |
| `mainPanelLayout` | 同上 | `{references:19, commits:57, inspector:24}` | `:19`, `:43-47` |
| `inspectorPanelLayout` | 同上 | `{files:62, details:38}` | `:20`, `:49-52` |
| `filterQuery` / `filterScope` | 同上 | `""` / `"text"` | `:61-62` |
| `showDecorations` | 同上 | `true` | `:63` |
| `showMyBranchesOnly` | 同上 | `false` | `:64` |
| `collapsedReferenceSections` / `collapsedReferenceGroups` | 同上 | `[]` / `[]` | `:67-68` |
| `markedReferenceFullNamesByRepository` | 同上 | `{}` | `:69` |
| `sourceControlSessions[repoPath]` | `git.store`（提交信息 / 勾选路径 / 折叠状态） | — | `features/git/components/git-view.tsx:102-104`, `:147-154`, `:768-788` |
| `sidebarWidth` / `rightToolWindowWidth` / `aiChatWidth` | 设置存储 | — | `features/layout/components/resizable-pane.tsx:16,42` |

---

## 5. 文案（`windows/tauri/src/i18n/locale.ts`，中文原文照抄）

`locale.ts` 是单文件双语（英文在前、中文在后）：`git.console.*` 英文在 `:9-108`、中文在 `:4470-4569`；`git.log.*` 英文在 `:2616-2978`、中文在 `:7188-7304`；`footer.*` 英文在 `:2979-2995`、中文在 `:7305-7321`。

### 5.1 Git Log 标题栏 / 标签行 / 面板

| 键 | 中文原文 | 位置 | 使用点 |
| --- | --- | --- | --- |
| `workbench.gitLog` | 提交记录 | `locale.ts:5910` | `git-log-title-bar.tsx:31` |
| `git.log.all` | 全部 | `:7188` | `git-log-tool-window.tsx:571` |
| `git.log.logLabel` | 日志：{name} | `:7189` | `git-log-title-bar.tsx:38` |
| `git.log.showAll` | 显示全部引用 | `:7190` | `git-log-title-bar.tsx:36` |
| `git.log.refresh` | 刷新 Git 日志 | `:7191` | `git-log-title-bar.tsx:46-47` |
| `git.log.settings` | 打开 Git 日志设置 | `:7219` | `git-log-title-bar.tsx:56-57` |
| `git.log.hide` | 隐藏提交记录 | `:7192` | `git-log-title-bar.tsx:67-68` |
| `footer.readOnly` | 只读 | `:7309` | `git-log-title-bar.tsx:61` |
| `git.console.log` | 日志 | `:4556` | `git-log-tool-window.tsx:584` |
| `git.console.title` | 控制台 | `:4555` | `git-log-tool-window.tsx:585` |
| `git.log.unableToRefresh` | 无法刷新 Git 日志。 | `:7196` | `git-log-tool-window.tsx:591` |
| `git.log.retry` | 重试 | `:7197` | `git-log-tool-window.tsx:597` |
| `git.log.noRepository` | 未打开仓库 | `:7198` | `:604` |
| `git.log.openWorkspace` | 打开一个 Git 工作区以查看提交记录。 | `:7199` | `:605` |
| `git.log.loading` | 正在加载 Git 日志… | `:7200` | `:609` |
| `ui.noActionsHere` | 此处无任何内容 | `:4630` | `:722`；`git-reference-tree.tsx:679` |
| `git.log.copied` | 已复制{label} | `:7203` | `git-log-tool-window.tsx:551` |
| `git.log.copyFailed` | 无法复制{label} | `:7204` | `:554` |
| `git.log.commitHash` | 提交哈希 | `:7205` | `:661` |
| `git.log.commitMessage` | 提交说明 | `:7206` | `:665` |

### 5.2 提交表格

| 键 | 中文原文 | 位置 | 使用点 |
| --- | --- | --- | --- |
| `git.log.filter` | 筛选 Git 日志 | `:7276` | `git-commit-table.tsx:216` |
| `git.log.filterPlaceholder` | {field} 筛选 | `:7282` | `:207` |
| `git.log.filterText` | 文本 | `:7279` | `:212`, `:235` |
| `git.log.filterAuthor` | 作者 | `:7280` | `:211`, `:236` |
| `git.log.filterBranch` | 分支 | `:7281` | `:213`, `:237` |
| `git.log.filterField` | Git 日志筛选字段 | `:7278` | `:233` |
| `git.log.clearFilter` | 清除 Git 日志筛选 | `:7277` | `:223` |
| `git.log.showDecorations` | 显示分支和标签 | `:7284` | `:244-245` |
| `git.log.hideDecorations` | 隐藏分支和标签 | `:7283` | `:244-245` |
| `git.log.commit` | 提交 | `:7285` | `:256` |
| `git.log.author` | 作者 | `:7286` | `:257` |
| `git.log.date` | 日期 | `:7287` | `:258` |
| `git.log.noMatch` | 没有符合筛选条件的提交 | `:7288` | `:271` |
| `git.log.noCommits` | 此视图中没有提交 | `:7289` | `:271` |
| `git.log.openDiffHint` | 双击或按 Enter 打开提交差异 | `:7290` | `:318` |
| `git.log.openCommitDiff` | 打开提交差异 | `:7291` | `:356` |
| `git.log.compareWithHead` | 与 HEAD 比较 | `:7194` | `:361` |
| `git.log.copyCommitHash` | 复制提交哈希 | `:7292` | `:415` |
| `git.log.copyCommitMessage` | 复制提交说明 | `:7293` | `:419` |
| `git.log.loadingCommits` | 正在加载提交… | `:7294` | `:440` |
| `git.log.loadMore` | 加载更多提交 | `:7295` | `:440` |
| `git.squashCommits` | 压缩提交 | `:8617` | `:339` |
| `git.historyReview.contiguousRequired` | 压缩需要选中当前分支上的连续提交 | `:8616` | `:334` |
| `git.patch.create` | 创建补丁… | `:8549` | `:349` |
| `git.patch.twoCommits` | 导出补丁需要恰好选中两个提交 | `:8560` | `:344` |
| `git.undoCommit` | 撤销提交 | `:8588` | `:374` |
| `git.historyReview.headRequired` | 仅 HEAD 提交可撤销 | `:8615` | `:368` |
| `git.rebasePlan.fromHere` | 从这里交互式变基… | `:6842` | `:381` |
| `git.editCommitMessage` | 编辑提交消息 | `:8544` | `:388` |
| `git.deleteCommit` | 删除提交 | `:8547` | `:396` |
| `git.resetToCommit` | 重置到此处 | `:8620` | `:403` |
| `git.cherryPickCommit` | Cherry-pick 提交 | `:8625` | `:410` |

### 5.3 Inspector

| 键 | 中文原文 | 位置 | 使用点 |
| --- | --- | --- | --- |
| `git.log.commitFiles` | 提交文件 | `:7296` | `git-commit-inspector.tsx:107`；`git-commit-file-tree.tsx:180` |
| `git.log.filesCount` | {count} 个文件 | `:7297` | `git-commit-inspector.tsx:111` |
| `git.log.loadingShort` | 加载中… | `:7303` | `:110` |
| `git.log.selectCommit` | 选择一个提交 | `:7298` | `:133` |
| `git.log.loadingChangedFiles` | 正在加载更改的文件… | `:7299` | `:137` |
| `git.log.unableToLoadFiles` | 无法加载更改的文件 | `:7300` | `:141` |
| `git.log.noChangedFiles` | 没有更改的文件 | `:7301` | `:145` |
| `git.log.commitDetails` | 提交详情 | `:7302` | `:188` |
| `git.log.openFileDiff` | 双击打开文件差异 | `:7304` | `git-commit-file-tree.tsx:131` |

### 5.4 引用树

| 键 | 中文原文 | 位置 | 使用点 |
| --- | --- | --- | --- |
| `git.log.references` | 引用 | `:7207` | `git-reference-tree.tsx:841` |
| `git.log.headCurrentBranch` | HEAD（当前分支） | `:7220` | `:859` |
| `git.log.local` | 本地 | `:7252` | `:59` |
| `git.log.remote` | 远程 | `:7253` | `:60` |
| `git.log.tags` | 标签 | `:7254` | `:61` |
| `git.log.none` | 无 | `:7255` | `:920` |
| `git.log.expand` | 展开 {name} | `:7256` | `:613` |
| `git.log.collapse` | 折叠 {name} | `:7257` | `:613` |
| `git.current` | 当前 | `:6948` | `:653` |
| `git.aheadOfRemote` | 领先远端分支 {count} 个提交 | `:6918` | `:647` |
| `git.behindRemote` | 落后远端分支 {count} 个提交 | `:6919` | `:648` |
| `git.log.toolbar.newBranch` | 新建分支 | `:7208` | `:213` |
| `git.log.toolbar.updateSelected` | 更新所选 | `:7209` | `:221` |
| `git.log.toolbar.deleteBranch` | 删除分支 | `:7210` | `:235` |
| `git.log.toolbar.compareWithCurrent` | 与当前分支比较 | `:7211` | `:243` |
| `git.log.toolbar.fetch` | 提取 | `:7212` | `:252` |
| `git.fetch.options` | 未在本区域核定中文（未找到此行号的 zh 条目） | 未找到 | `:260` |
| `git.log.toolbar.mark` | 标记 | `:7213` | `:268` |
| `git.log.toolbar.unmark` | 取消标记 | `:7214` | `:268` |
| `git.log.toolbar.showMyBranches` | 显示我的分支 | `:7215` | `:288` |
| `git.log.toolbar.showAllBranches` | 显示全部分支 | `:7216` | `:287` |
| `git.log.toolbar.goToHead` | 导航到所选分支 HEAD | `:7217` | `:277` |
| `git.log.toolbar.moreActions` | 更多操作 | `:7218` | `:382` |
| `git.expandAll` | 未在本区域核定（未找到 zh 条目行号） | 未找到 | `:298` |
| `git.collapseAll` | 未在本区域核定（未找到 zh 条目行号） | 未找到 | `:306` |
| `git.checkout` | 未在本区域核定（未找到 zh 条目行号） | 未找到 | `:454` |
| `git.log.newBranchFrom` | 从“{branch}”新建分支... | `:7221` | `:455` |
| `git.log.checkoutAndRebaseOnto` | 签出并变基到“{branch}” | `:7224` | `:456` |
| `git.log.checkoutAndUpdate` | 签出并更新 | `:7227` | `:457` |
| `git.log.compareWithCurrent` | 与“{branch}”比较 | `:7229` | `:458` |
| `git.log.showDiffWithWorkingTree` | 显示与工作树的差异 | `:7230` | `:459` |
| `git.log.rebaseCurrentOnto` | 将“{current}”变基到“{branch}” | `:7231` | `:460` |
| `git.log.mergeIntoCurrent` | 将“{branch}”合并到“{current}”中 | `:7232` | `:464` |
| `git.log.pullRebaseIntoCurrent` | 使用“变基拉入”“{branch}” | `:7233` | `:468` |
| `git.log.pullMergeIntoCurrent` | 使用“合并拉入”“{branch}” | `:7234` | `:469` |
| `git.log.newWorktreeFrom` | 从“{branch}”新建工作树... | `:7237` | `:470` |
| `git.log.updateBranch` | 更新 | `:7240` | `:471` |
| `git.push` | 未在本区域核定（未找到 zh 条目行号） | 未找到 | `:472` |
| `git.log.trackingBranch` | 跟踪的分支 | `:7241` | `:473` |
| `git.log.renameBranch` | 重命名... | `:7243` | `:474` |
| `git.deleteBranch` | 未在本区域核定（未找到 zh 条目行号） | 未找到 | `:475` |
| `git.log.deleteRemoteBranch` | 删除远程分支 | `:7245` | `:476` |
| `git.log.stopTrackingBranch` | 停止跟踪分支 | `:7242` | `:506` |
| `git.log.noRemoteBranches` | 没有远程分支 | `:7248` | `:525` |
| `git.log.manageRemotes` | 管理远程... | `:7247` | `:892` |
| `git.deleteBranchConfirm` / `git.delete` / `git.actionCompleted` / `git.actionFailed` | 未在本区域核定（未找到 zh 条目行号） | 未找到 | `git-log-tool-window.tsx:408,409` |

### 5.5 控制台

| 键 | 中文原文 | 位置 | 使用点 |
| --- | --- | --- | --- |
| `git.console.find` | 在 Git 控制台中查找 | `:4473` | `git-execution-console.tsx:72,94` |
| `git.console.wrap` | 自动换行 | `:4493` | `:73` |
| `git.console.scrollToEnd` | 滚动到底部 | `:4494` | `:74` |
| `git.console.cancel` | 取消正在运行的操作 | `:4557` | `:78` |
| `git.console.clear` | 清空 | `:4558` | `:79` |
| `git.console.copy` | 复制输出 | `:4559` | `:90`；`git-console-entry.tsx:107` |
| `git.console.copyPath` | 复制仓库路径 | `:4479` | `git-console-entry.tsx:96` |
| `git.console.copyCommand` | 复制完整命令 | `:4480` | `:97` |
| `git.console.copyOutput` | 复制完整输出 | `:4481` | `:98` |
| `git.console.details` | 命令详情 | `:4470` | `:99` |
| `git.console.previousMatch` | 上一个匹配 | `:4474` | `git-execution-console.tsx:98` |
| `git.console.nextMatch` | 下一个匹配 | `:4475` | `:99` |
| `git.console.closeSearch` | 关闭查找 | `:4476` | `:100` |
| `git.console.searchLimit` | 前 {count} 个位置 | `:4478` | `:97` |
| `git.console.empty` | Git 命令及其输出将显示在这里。 | `:4560` | `:105` |
| `git.console.historyTruncated` | 为限制内存占用，较早的 Git 执行记录已截断，无法展开。 | `:4471` | `:104` |
| `git.console.presentationUnavailable` | 暂时无法压缩，正在显示完整保留的输出。 | `:4482` | `:102` |
| `git.console.queued` | 等待启动 | `:4561` | `git-console-entry.tsx:67,84` |
| `git.console.running` | 正在运行 | `:4562` | 同上 |
| `git.console.completed` | 已完成 | `:4566` | 同上 |
| `git.console.exit` | 退出码 | `:4568` | `:82,84` |
| `git.console.truncated` | 为限制内存占用，已省略较早的输出。 | `:4569` | `:79` |
| `git.console.unconfirmed` | 未收到已完成的命令记录 | `:4567` | `:81` |
| `git.console.options` | Git 命令选项 | `:4472` | `:65` |
| `git.console.matches` | {count} 处匹配 | `:4477` | `:38` |
| `git.console.referenceChanges` | 其余引用变化：新增 {added}、更新 {updated}、删除 {deleted} | `:4484` | `:36` |
| `git.console.automaticQueries` | 自动查询 {count} 次，结果相同 | `:4483` | （`git-console-entry` 折叠标签，由 Core 折叠类型决定） |
| `git.console.arguments.files` | … 另 {count} 个文件 | `:4485` | `:59` |
| `git.console.arguments.references` | … 另 {count} 个引用 | `:4486` | `:59` |
| `git.console.fold.lines` | 展开中间 {count} 行 | `:4487` | `:37` |
| `git.console.fold.repeat` | 重复 {count} 次 | `:4488` | `:37` |
| `git.console.fold.files` | 其余 {count} 个文件 | `:4489` | `:37` |
| `git.console.fold.branches` | 其余 {count} 个分支 | `:4490` | `:37` |
| `git.console.fold.tags` | 其余 {count} 个标签 | `:4491` | `:37` |
| `git.console.fold.commits` | 其余 {count} 条提交 | `:4492` | `:37` |
| `git.console.notice.waitingForOutput` | 正在执行 Git，等待输出。 | `:4563` | `git-console-entry.tsx:78` |
| `git.console.notice.completedWithoutOutput` | 执行成功，没有输出。 | `:4564` | `:78` |
| `git.console.notice.fetchUnchanged` | 获取完成，没有引用变化。 | `:4565` | `:78` |

### 5.6 变更列表 / 提交面板

| 键 | 中文原文 | 位置 | 使用点 |
| --- | --- | --- | --- |
| `workbench.sourceControl` | 源代码管理 | `:5909` | `git-view.tsx:639,667,691,707` |
| `workbench.changes` | 更改 | `:4606` | `git-view.tsx:623`；`footer-editor-status.tsx:124` |
| `git.history` | 历史记录 | `:6814` | `git-view.tsx:627` |
| `git.viewDiff` | 查看差异 | `:6816` | `git-status-panel.tsx:1118` |
| `git.chooseDiffSource` | 选择差异来源 | `:7153` | `:1128` |
| `git.unstaged` | 未暂存 | `:8643` | `:1035` |
| `git.staged` | 已暂存 | `:8644` | `:1041` |
| `git.commit` | 提交 | `:6832` | `:1048`, `:1279` |
| `git.branch` | 分支 | `:8645` | `:1055` |
| `git.stash` | 贮藏 | `:8646` | `:1062`, `:1362` |
| `git.tracked` | 已跟踪 | `:6817` | `:861` |
| `git.untracked` | 未跟踪 | `:6818` | `:861` |
| `git.trackedFiles` | 已跟踪文件 | `:7156` | `:1190` |
| `git.untrackedFiles` | 未跟踪文件 | `:7157` | `:1190` |
| `git.workingTreeClean` | 工作区干净 | `:7158` | `:1225` |
| `git.stashAllUnstaged` | 贮藏全部未暂存 | `:8648` | `:1150` |
| `git.stageAllChanges` | 暂存所有更改 | `:7154` | `:1162` |
| `git.unstageAllChanges` | 取消暂存所有更改 | `:7155` | `:1174` |
| `git.rollback` | 回滚 | `:7160` | `:1286` |
| `git.showDiff` | 显示差异 | `:7162` | `:1293` |
| `git.jumpToSource` | 跳转到源 | `:7163` | `:1300` |
| `git.delete` | 删除 | `:6956` | `:1319` |
| `git.addToVcs` | 添加到 VCS | `:7168` | `:1329` |
| `git.addToGitignore` | 添加到 .gitignore | `:7169` | `:1341` |
| `git.addToLocalExclude` | 添加到 .git/info/exclude | `:7170` | `:1353` |
| `git.gitignoreFile` | .gitignore | `:7173` | `:624` |
| `git.localExcludeFile` | .git/info/exclude | `:7174` | `:624` |
| `git.diffFileCount` | {count} 个文件 | `:7130` | `:926`；`git-diff-header.tsx:96` |
| `git.selectSingleRepositoryForStash` | 请选择同一个仓库中的更改进行贮藏。 | `:7152` | `:676`, `:705` |
| `git.stashSelected` | 搁置选中的更改 | `:7177` | `:1376` |
| `git.stashAllUnstaged` | 贮藏全部未暂存 | `:8648` | `:1376` |
| `git.stashSelectedDefault` | 搁置选中的更改 | `:7151` | `:723` |
| `git.stashAllUnstagedChanges` | 贮藏所有未暂存的更改 | `:7150` | `:723` |
| `git.stashMessageDefaultSelection` | 说明（默认：搁置选中的更改） | `:8653` | `:1379` |
| `git.stashMessageDefaultAll` | 说明（默认：贮藏全部未暂存更改） | `:8651` | `:1380` |
| `git.noRepositorySelected` | 未选择仓库 | `:7010` | `git-view.tsx:647` |
| `git.loadingGitStatus` | 正在加载 Git 状态 | `:7013` | `:670` |
| `git.statusLoadFailed` | 无法加载 Git 数据，请重试刷新仓库。 | `:7011` | `:680` |
| `git.historyLoadFailed` | 无法加载 Git 历史。 | `:8536` | `:680` |
| `git.actions` | Git 操作 | `:8641` | `git-view.tsx:474` |
| `git.refresh` | 刷新 | `:6944` | `:484` |
| `git.refreshAria` | 刷新 Git 状态 | `:8642` | `:485` |
| `git.commitMessagePlaceholder` | 提交说明... | `:6819` | `git-commit-panel.tsx:281` |
| `git.filesSelected` | 已选择 {count} 个文件 | `:6827` | `git-commit-panel.tsx:297` |
| `git.fileSelected` | 已选择 {count} 个文件 | `:6828` | `:297` |
| `git.noFilesSelected` | 没有选择文件 | `:6829` | `:300` |
| `git.commitCount` / `git.commitsCount` | {count} 个提交 | `:6930` / `:6931` | `git-view.tsx:803` |
| `git.noCommits` | 暂无提交 | `:6932` | `git-view.tsx:810` |

### 5.7 差异视图

| 键 | 中文原文 | 位置 |
| --- | --- | --- |
| `git.diff.changedFiles` | 已更改文件 | `:7146` |
| `git.diff.uncommitted` | 未提交的更改 | `:8738` |
| `git.diff.unstagedChanges` | 未暂存的更改 | `:8681` |
| `git.diff.stagedChanges` | 已暂存的更改 | `:8682` |
| `git.diff.showWhitespace` | 显示空白字符 | `:8664` |
| `git.diff.hideWhitespace` | 隐藏空白字符 | `:8665` |
| `git.diff.lineEndingChanges` | 换行符不同 | `:8666` |
| `git.diff.stage` | 暂存 | `:8736` |
| `git.diff.unstage` | 取消暂存 | `:8737` |
| `git.loadingDiff` | 正在加载差异 | `:7118` |
| `git.noDiffData` | 没有可用的差异数据 | `:7119` |
| `git.unifiedView` / `git.splitView` | 未在本区域核定（未找到 zh 条目行号） | 未找到 |

### 5.8 底部窗 / 布局

| 键 | 中文原文 | 位置 | 使用点 |
| --- | --- | --- | --- |
| `layout.resizeBottomPane` | 调整底部面板大小 | `:7888` | `bottom-pane.tsx:232` |
| `layout.resizeActivityRail` | 调整活动栏大小 | `:4701` | `main-sidebar.tsx:700` |
| `layout.resizeSidebar` | 调整侧边栏大小 | `:7887` | `resizable-pane.tsx:166` |
| `layout.resizeAiChat` | 调整 AI 聊天大小 | `:7886` | `resizable-pane.tsx:166` |
| `layout.actions` | 操作 | `:4700` | `main-sidebar.tsx:712` |
| `workbench.terminal` | 终端 | `:5915` | 活动栏 |
| `workbench.run` | 运行 | `:5913` | 活动栏 |
| `workbench.maven` | Maven | `:5914` | 活动栏 |
| `footer.statusBar` | 状态栏 | `:7316` | `footer.tsx:48` |
| `footer.cursor` | 跳转到行和列 | `:7305` | `footer-editor-status.tsx:73` |
| `footer.encoding` | 文件编码 | `:7306` | `:80` |
| `footer.indent` | 缩进 | `:7307` | `:88` |
| `footer.spaces` | {count} 个空格 | `:7308` | `:89` |
| `footer.writable` | 可写 | `:7310` | `:96,100` |
| `footer.memory` | 内存 | `:7311` | `:110` |
| `footer.memoryUsage` | 总计 {total} · Lithe {used} | `:7312` | `:114` |
| `footer.changes` | {count} 个更改 | `:7313` | `:131,137` |
| `footer.change` | {count} 个更改 | `:7314` | `:131,137` |
| `footer.noChanges` | 没有更改 | `:7315` | `:130` |
| `footer.filePath` | 文件路径 | `:7317` | 文件路径项 |
| `footer.gitBranch` | Git 分支 | `:7318` | `footer-git-branch-item.tsx:31` |

---

## 6. gpui-kit 对应建议（gpui-kit 0.6.6）

> gpui-kit 是门面：`gpui_kit::*` = GPUI；`gpui_kit::base` = `gpui-base`；`gpui_kit::component` = `gpui-component`（`D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-kit-0.6.6\src\lib.rs` 的 `pub use ::gpui_base as base;` / `pub use ::gpui_component as component;`）。
> 下文 `模块路径::类型名` 均以 `gpui_kit::` 为根。

### 6.0 三条已知硬约束（务必在设计阶段消化）

1. **`uniform_list` 只测量第 0 行**
   `gpui::uniform_list(id, item_count, f)`（`gpui-pre-0.3.6/src/elements/uniform_list.rs:22`）内部 `item_to_measure_index: 0`（`:43`），`measure_item` 取 `cmp::min(self.item_to_measure_index, self.item_count - 1)`（`:668`），再用 `item_size.height * item_count` 推总高（`:367`、`:371`）。
   → 若第 0 行因上下文拿到 0 高（典型：把列表塞进未确定尺寸的 `h_resizable` 面板，或第 0 行用 `h_full()` 依赖父高），整表高度会塌成 0。**对策**：给列表一个确定的像素高容器，并让第 0 行用固定 `px(30.)` 之类的绝对高，不要用 `h_full()`/百分比高。`gpui-base` 的 `VirtualList` 同样"逐项测量"但支持不同高度（`gpui-base-0.6.6/src/virtual_list.rs:1-12` 明确说明它是 `uniform_list` 的变体）。
2. **`DataTable` 行高是表级值**
   `size.rs` 的 `Size::table_row_height()`（`gpui-component-0.6.6/src/sizing.rs`）返回 `XSmall=26 / Small=30 / Medium=32 / Large=40`，`TableState` 全部按 `self.options.size.table_row_height()` 计算（`gpui-component-0.6.6/src/table/state.rs:713`、`:1814`、`:1875`、`:1962`、`:2231`、`:2376`）。**不能逐行设高**。本仓库提交表行高正好 **30px** → 用 `Size::Small` 或 `Size::Size(px(30.))` 即可一比一。
3. **`TableState` / `ListState` / `TreeState` 只有单选**
   - `TableState.selected_row: Option<usize>`（`gpui-component-0.6.6/src/table/state.rs:240`、`:458-473`）。
   - `ListState.selected_index: Option<IndexPath>`（`gpui-component-0.6.6/src/list/list.rs:78`、`:194`）。
   - `TreeState.selected_ix: Option<usize>`（`gpui-base-0.6.6/src/tree.rs:188`、`:221-226`）。
   → 提交表的 `Set<string>` 多选、Ctrl 加选、Shift 连选必须在**应用侧**维护，并由 delegate 自行渲染选中样式（`DataTable` 的 `render_tr`／`TableDelegate::render_td` 可以完全接管行外观）。唯一例外是 `SearchableListState`（`gpui-component-0.6.6/src/searchable_list/state.rs:134-219`，`selection()` / `add_selected_index` / `set_selected_indices`）本身支持多选。

### 6.1 逐元素映射

| # | 界面元素（本文位置） | gpui-kit 0.6.6 建议 | 判定 |
| --- | --- | --- | --- |
| 1 | 主壳层纵向三段（TitleBar / 工作台 / Footer） | `gpui_kit::component::title_bar::TitleBar`、`gpui_kit::component::status_bar::StatusBar`（`status_bar.rs:32-57`：`left()` / `right()`），中间用 `v_flex()` | **可一比一** |
| 2 | 底部窗容器（圆角 + 上/左边框 + 可拖高 200–80% 窗高） | `v_flex()` + `rounded(px(11.2))` + `border_t_1()` + `border_l_1()`；高度用 `gpui_base::resizable` 的**纵向** `v_resizable("bottom")` + `resizable_panel().size(px(320.)).size_range(px(200.)..)`。gpui-kit 没有"80% 窗高"上限概念，需在 `ResizablePanelGroup::on_resize`（`gpui-base-0.6.6/src/resizable/panel.rs:111`）里自行 clamp | **需自行组合** |
| 3 | 底部窗拖拽条（4px 命中区 + 1px 线 + hover 变主色） | `gpui_kit::component::resizable::ResizablePanelGroup` 自带的 `resize_handle`（`gpui-base-0.6.6/src/resizable/resize_handle.rs`，`with_handle_appearance(ResizeHandleRenderer)`，见 `panel.rs:59`）。默认外观与"1px 线 + hover 主色"近似但不是像素级一致 | **可一比一（外观需微调）** |
| 4 | 底部窗无标签条、由活动栏/命令面板切换 `bottomPaneActiveTab` | gpui-kit 的**两种**做法：(a) 保持单值切换 + 自绘侧栏按钮（与现状同构）；(b) 用 `gpui_kit::component::dock::{DockArea, Panel, TabPanel}`（`gpui-component-0.6.6/src/dock/`），`gpui_component::dock::Panel` trait 提供 `tab_name()` / `title()` / `tab_icon()` / `toolbar_buttons()`（`dock/panel.rs:76-101`），且基础层的 `Panel` 有 `panel_name()` / `visible()` / `closable()` / `zoomable()`（`gpui-base-0.6.6/src/dock/panel.rs:21-42`）。若要走 IDEA 风格底部工具窗+缩放，dock 是唯一合理选择 | **需自行组合**（a）/ **可一比一**（b，但会改变现有"无标签条"外观） |
| 5 | Git Log 标题栏（h=32，图标 14，按钮 24×24，右侧只读文本） | `h_flex().h(px(32.)).gap_2().px_2()` + `gpui_kit::component::icon::Icon`（`icon.rs:84`，`size_3p5()` 由 `Sizable` 提供）+ `gpui_kit::component::button::Button::new(id).ghost().with_size(px(24.))`（`button/button.rs:233`、`Sizable::with_size`） | **可一比一** |
| 6 | 「引用名」胶囊按钮（h=24 / max-w 240 / 圆角 6.4 / border-strong） | `Button::new(..).outline().with_size(px(24.)).max_w(px(240.))`（`Button::outline` 在 `button/button.rs:318`，`rounded` 在 `:324`） | **可一比一** |
| 7 | Log/Console 标签行（h≈24，12px 字号，`role="tab"`） | `gpui_kit::component::tab::{TabBar, Tab}`（`tab/tab_bar.rs:40-171`，变体 `TabVariant`：`pill/outline/segmented/underline`，`tab/tab.rs:14`）。**注意**：gpui-kit 的 Tab 一定带样式，本仓库现状是"无选中样式"，迁移时需要挑选最接近的变体并接受外观变化 | **可一比一（外观会变）** |
| 8 | 三栏横向分割 `19% / 57% / 24%` + 最小宽 140/320/220 | `gpui_kit::component::h_resizable("git-log")` + `resizable_panel().size(px(..)).size_range(px(140.)..)`。**注意**：`ResizablePanel::size` 只接受 `Pixels`（`resizable/panel.rs:275`），没有百分比；需在首帧按容器宽度把持久化的百分比换算成像素。尺寸回读用 `ResizableState::sizes()`（`resizable/mod.rs:56`），变更事件 `ResizablePanelEvent::Resized`（`panel.rs:17`）或 `ResizablePanelGroup::on_resize`（`panel.rs:111`）用于持久化 | **需自行组合** |
| 9 | Inspector 内部竖向 `62% / 38%` | 同上，`v_resizable("inspector")` | **需自行组合** |
| 10 | 提交表（30px 行高、作者 112px、日期 128px、右侧对齐、可拖列宽） | `gpui_kit::component::table::{DataTable, TableState, TableDelegate}`；列用 `Column::new(key, name).width(px(112.)).resizable(true).min_width(..).max_width(..)`，日期列 `.text_right()`（`table/column.rs:133,151,169,202,215`）。行高 `Size::Small`(=30) 或 `Size::Size(px(30.))`（`sizing.rs`）。列宽变更事件 `TableEvent::ColumnWidthsChanged(Vec<Pixels>)`（`table/state.rs:85`）→ 持久化 | **可一比一**（行高与列宽度量完全对得上） |
| 11 | 提交表"泳道图 + 标签 + 说明 + 作者 + 日期"单行混合布局 | `TableDelegate::render_tr`（`table/delegate.rs:91`）/ `render_td`（`:112`）完全接管行渲染 → 可在第一列里画泳道 `svg`/自绘 element。**注意**：`DataTable::render` 强制 `size_full()` + `bordered` 时 `rounded(theme.radius).border_1()`（`data_table.rs` 的 `RenderOnce`），如果只要行不要外框，用 `.bordered(false)` | **需自行组合** |
| 12 | 泳道图（自绘 SVG：竖线/贝塞尔/圆点） | gpui 原生 `canvas(...)`（`gpui-elements` 技能覆盖的 `Element` 三阶段）或直接 `svg()` 路径元素。gpui-kit **没有** graph/泳道组件 | **gpui-kit 没有** |
| 13 | 提交表多选（Ctrl/Shift）+ 选中背景 `primary/22` | 状态自管（见 6.0-3）；行外观由 `render_tr` 自绘。键盘多选需自行绑 Action（`gpui_kit::actions!` 宏在 `gpui-kit-0.6.6/src/lib.rs`） | **需自行组合** |
| 14 | 提交表筛选输入 + 字段下拉 + 计数 | `gpui_kit::component::input::{Input, InputState}`（`input/input.rs:111-296`，`cleanable()` 提供清除按钮）+ `gpui_kit::component::select::{Select, SelectState}`（`select.rs:133,642`）。计数用 `Label`/`div` | **可一比一** |
| 15 | 装饰开关（Eye/EyeSlash，`aria-pressed`） | `gpui_kit::component::button::Toggle`（`button/toggle.rs`）或 `Button::new(..).toggled(bool)`（`button/button.rs:483`）+ `Icon`；图标名用 `gpui_kit_assets::IconName::Eye` / `EyeOff`（assets 目录存在 `eye.svg`、`eye-off.svg`） | **可一比一** |
| 16 | 引用树（分区 → 分组 → 引用，24px 行高，缩进 10+14·depth，折叠状态持久化） | `gpui_kit::component::tree::{tree, Tree}`（`tree.rs:18,32`）+ `gpui_kit::base::tree::{TreeState, TreeItem}`（`gpui-base-0.6.6/src/tree.rs:41,184`）；`TreeItem::new(id,label).child(..).expanded(..)`（`:99,111,121`），`TreeState::set_selected_item` / `reveal_item`（`:230,268`）。**注意**：`TreeState` 单选；行高由 `TreeState::scroll_handle()` 背后的 `UniformListScrollHandle`（`:256`）决定，仍是 uniform 语义 → 需保证所有行同高（本仓库引用树正好全部 `h-6`） | **可一比一** |
| 17 | 引用树左侧竖排 32px 按钮栏（自适应溢出到 HoverCard） | `v_flex().w(px(36.))` + 多个 `Button.with_size(px(32.))`；溢出用 `gpui_kit::component::hover_card::HoverCard`。**注意**：`ResizeObserver` 语义在 gpui 里换成 `gpui_base::measure` / `on_children_prepainted` 或 `Element` 的 `request_layout` 后读 `bounds`——gpui-kit 无 `ResizeObserver` 对应物 | **需自行组合** |
| 18 | 引用树右键菜单（分组 + 子菜单 + 危险项 + disabled 规则） | `gpui_kit::component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem}`：`ContextMenuExt::context_menu(..)`（`menu/context_menu.rs:13,42`）；`PopupMenu::menu_with_icon`（`:543`）、`menu_with_check`（`:565`）、`menu_element_with_disabled`（`:596`）、`separator()`（`:680`）、`submenu_with_icon`（`:705`）、`min_w(px(288.))`（`:429`）。gpui-kit 没有 `variant="destructive"`，危险项需自绘颜色 | **可一比一（危险色需自绘）** |
| 19 | 引用树「当前」徽章 / 泳道标签徽章 / ahead-behind | `gpui_kit::component::tag::Tag`（`tag.rs:125-209`，`primary/secondary/danger/success/warning/info/custom/outline/rounded/rounded_full`）+ `gpui_kit::component::badge::Badge`（`badge.rs:31-82`，`dot/count/icon/max/color`）+ `gpui_kit::component::icon::Icon` | **可一比一** |
| 20 | Inspector 文件树（24px 行、10+14·depth、图标、状态色尾标） | 同 #16 的 `Tree`；或 `gpui_kit::component::list::{List, ListState, ListDelegate, ListItem}`（`list/list.rs:721,732`；`list/list_item.rs:26-121` 提供 `selected/confirmed/disabled/suffix/on_click`）。**注意**：两者都只有单选；文件树需要"选中一个路径用于预览"正好够用 | **可一比一** |
| 21 | Inspector 详情区（说明/描述/短哈希/日期/装饰/完整哈希） | `v_flex().gap_2().p_3()` + `Label` + `gpui_kit::component::description_list`（`description_list.rs`，键值对布局） | **可一比一** |
| 22 | 变更列表分节头（32px、`bg-accent/80`、计数 Badge） | `gpui_kit::component::accordion` 或 `collapsible`（`gpui-component-0.6.6/src/{accordion.rs,collapsible.rs}`）+ `Tag`/`Badge` 做计数 | **可一比一** |
| 23 | 变更列表文件行（20px 悬停出现的暂存按钮 + Checkbox + 缩进） | `gpui_kit::component::checkbox::Checkbox`（`checkbox.rs:17-114`，`checked/on_change`）+ 自定义行。**注意**：`Checkbox` 无"仅悬停显示"语义，需用 `on_hover` 控制 wrapper 的 `opacity` | **需自行组合** |
| 24 | 变更列表虚拟滚动（变高行：文件 24px、分组 32px 混合） | `gpui_kit::component::{v_virtual_list, VirtualList}`（`virtual_list.rs:1-12` 说明支持不同行高；`gpui-component-0.6.6/src/lib.rs:122` 重新导出 `VirtualList/h_virtual_list/v_virtual_list`）。**不要**用 `uniform_list`（分组 32 / 文件 24 不等高） | **可一比一** |
| 25 | 变更列表上下文菜单（14 项、条件禁用、危险色） | 同 #18 的 `PopupMenu` | **可一比一** |
| 26 | 多仓暂存/贮藏校验与 toast | `gpui_kit::component::notification`（`notification.rs`）或 `gpui_base::toast`（`gpui-base-0.6.6/src/toast.rs`） | **可一比一** |
| 27 | 控制台左侧 32px 按钮栏 + 查找条 + 等宽输出 | `v_flex/h_flex` + `Button.with_size(px(24.))` + `Input/InputState` + `font_family(mono)`；输出区用 `v_virtual_list`（每条记录行数不定，不能用 `uniform_list`） | **需自行组合** |
| 28 | 控制台输出行内的"片段折叠 + 命中高亮 + 右键复制" | gpui 的 `StyledText`/`TextRun` 分段着色 + `PopupMenu`。gpui-kit 没有"输出折叠"组件 | **gpui-kit 没有** |
| 29 | 控制台"Ctrl+F 查找 / 上下一个 / 计数" | `gpui_kit::component::input::search`（`input/search.rs`）+ 自定义 Action 绑定（`gpui_kit::actions!`）。gpui-kit 的 `Input` 自带 `search` 能力但不是"全量输出查找 + 高亮 + 折叠自动展开" | **需自行组合** |
| 30 | Diff hunk 头（`grid-cols-[2.75rem_1fr]`，随编辑器字号缩放） | `h_flex/grid` + 一个固定 `px(44.)` 的 gutter；字号随设置换算（本仓库用 `calculateLineHeight`，见 `features/editor/utils/lines.ts`）。gpui-kit 无 diff 组件 | **gpui-kit 没有** |
| 31 | Diff 视图（unified / split / 空白字符 / 逐 hunk 暂存） | gpui-kit **没有** diff 组件；`gpui_component::highlighter`（`highlighter/`）可提供语法高亮，行渲染需自建（可作为 `VirtualList` 的自定义行） | **gpui-kit 没有** |
| 32 | 状态栏 chip（24px 高、圆角、hover 背景、最大宽 200px） | `StatusBar::new().left(..).right(..)`（`status_bar.rs:32-57`）+ `Button::ghost()`/`Tag`。`StatusBar` 未提供 chip 级原语 | **需自行组合** |
| 33 | 状态栏左侧可排序项（`footerLeadingItemsOrder`/`footerTrailingItemsOrder`） | `orderChromeItems`（`features/layout/utils/chrome-items.ts`）为纯函数，可原样移植；UI 侧用 `h_flex` 按序 `child()` | **可一比一**（逻辑移植） |
| 34 | 无仓库/无匹配/加载中占位 | `gpui_kit::component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription, EmptyContent}`（`empty.rs:14-335`，`EmptyMediaVariant` 在 `:157`）+ `gpui_kit::component::spinner::Spinner`（`spinner.rs:10-47`）+ `gpui_kit::component::skeleton` | **可一比一** |
| 35 | 命令式选择面（提交差异选择 / 分支比较 / 贮藏列表，带搜索和元信息） | `gpui_kit::component::command::{Command, CommandState}`（`command/{command.rs,state.rs,item.rs}`）或 `gpui_kit::component::searchable_list`（`searchable_list/state.rs:17-219`，支持多选） | **可一比一** |
| 36 | 面包屑式 diff 头 | `gpui_kit::component::breadcrumb`（`breadcrumb.rs`） | **可一比一** |
| 37 | 图标（git-branch / git-commit / git-graph / tag / star / refresh / settings / pencil / trash / plus / minus / network / search / eye / eye-off / fold-vertical / unfold-vertical / text-wrap / arrow-down-to-line / circle-stop / copy / panel-bottom） | `gpui_kit::component::icon::Icon` + `gpui_kit_assets::IconName`。`gpui-kit-assets-0.6.6/assets/icons/` 下已确认存在：`git-branch.svg`、`git-branch-plus.svg`、`git-commit-horizontal.svg`、`git-graph.svg`、`git-merge.svg`、`git-compare.svg`、`tag.svg`、`star.svg`、`refresh-cw.svg`、`settings.svg`、`pencil.svg`、`trash.svg`、`plus.svg`、`minus.svg`、`network.svg`、`search.svg`、`eye.svg`、`eye-off.svg`、`fold-vertical.svg`、`unfold-vertical.svg`、`text-wrap.svg`、`arrow-down-to-line.svg`、`circle-stop.svg`、`copy.svg`、`folder.svg`、`panel-bottom.svg`、`columns-2.svg`、`rows-3.svg`。**缺失**：`wrap-text.svg`（Lucide 名为 `text-wrap`） | **可一比一**（个别图标需改名） |
| 38 | 工具提示（按钮 tooltip / 侧向 / 延迟） | `gpui_kit::component::button::Button::tooltip(..)` / `tooltip_placement(..)`（`button/button.rs:389,398`）；独立用 `gpui_kit::component::tooltip::Tooltip`（`tooltip.rs:34-81`） | **可一比一** |
| 39 | 模态（远程管理 / 标签管理 / 贮藏说明 / 补丁 / 变基 / 推送 / 工作树 / 认证） | `gpui_kit::component::dialog::{Dialog, AlertDialog, DialogHeader/Title/Description/Footer/Content}`；侧滑用 `gpui_kit::component::sheet::Sheet`；悬浮面板用 `popover::Popover` | **可一比一** |
| 40 | 输入/文本域（提交说明、各类 prompt） | `gpui_kit::component::input::{Input, InputState, Textarea, NumberInput}`（`input/{input.rs,textarea.rs,number_input.rs}`） | **可一比一** |
| 41 | `bindScrollContainerWheel`（自定义滚轮接管，见 `windows/tauri/src/ui/scroll-container-wheel.ts`） | gpui 原生滚动即 `overflow_y_scroll()` + `ScrollHandle`；`gpui_kit::component::scroll::{Scrollable, Scrollbar}`（`scroll/mod.rs`）。**没有** DOM 那种"把滚轮事件重定向到指定容器"的问题，此机制不需要移植 | **gpui-kit 没有（也不需要）** |
| 42 | 布局持久化（`git-log-preferences` localStorage） | 用 gpui 的 `Global`（`gpui_kit::component::global_state::GlobalState`）或自建 JSON 文件；`ResizablePanelEvent::Resized` / `TableEvent::ColumnWidthsChanged` 作为写点 | **需自行组合** |
| 43 | `aria-*` / `role` 可访问性标注意（`role="treeitem"`, `role="separator"`, `aria-pressed`…） | gpui-kit 有 `role(..)`（`Role`）/ `aria_selected` / `aria_expanded`（见 `gpui-base-0.6.6/src/tree.rs:429-437`）与 `accessibility_label`；`aria-valuenow/min/max` 一类数值型 ARIA 未见对应 API → **需自行组合或放弃** | **需自行组合** |

### 6.2 直接可以"一比一"抄的度量结论（最省事的迁移清单）

- 提交表行高 **30px** = `Size::Small.table_row_height()`（`gpui-component-0.6.6/src/sizing.rs` 的 `test_table_row_height` 断言）→ 无需自定义行高。
- 引用树/提交树/文件树行高 **24px**：`Tree`/`List` 的 uniform 语义要求所有行同高，24px 用 `Size::Size(px(24.))` 即可。
- 三栏/两栏分割的**最小尺寸**（140/320/220 与 90/80）可直接映射到 `ResizablePanel::size_range`。
- 所有"横向占满"的判定：底部窗默认只在中央列内（`main-layout.tsx:318-322`），"full"模式才跨全宽（`:345-351`）；gpui 侧用"是否位于中央列 `v_flex` 内"表达同一开关即可。

---

## 7. 未查清

1. **`git.fetch.options` 的中文原文行号**未定位（英文 `git.fetch.options` 见于 `git-reference-tree.tsx:260` 的 tooltip；zh 条目未在 `locale.ts` 逐行确认）。
2. **`git.expandAll` / `git.collapseAll` / `git.checkout` / `git.push` / `git.deleteBranch` / `git.deleteBranchConfirm` / `git.delete` / `git.actionCompleted` / `git.actionFailed` / `git.fetch` / `git.changesFetched` / `git.fetchFailed` / `git.operationFailed` / `git.stashChanges` / `git.unifiedView` / `git.splitView`** 等键的 **zh 行号**未逐条核定（英文键存在，zh 区段未逐行扫描）。
3. **`GitStatusPanel` 右键 `Dropdown` 的宽度/圆角**：走 `Dropdown.items` 路径，未指定 `className`，需要读 `windows/tauri/src/ui/dropdown.tsx` 的默认样式——本次未读该文件，故标"未找到"。
4. **`useSettingsStore.settings.fileTreeIndentSize` 与 `uiFontSize` 的默认值**：`use-file-tree-presentation.ts:17,20` 读取，但设置默认值定义在 `features/settings/` 的 schema 中，本次未定位。
5. **`--lithe-pane-header-height` 与 `--lithe-sidebar-header-height` 在舒适密度下是否有覆盖**：`theme.css:188-200` 只覆盖了 `--lithe-footer-height` / `--lithe-tab-height` / `--lithe-sidebar-header-height` / `--lithe-chrome-*`，**没有**覆盖 `--lithe-pane-header-height` —— 已确认（非未查清），但底部窗/侧栏标签条高度是否跟随其它密度机制未查。
6. **终端面板（TerminalContainer）与运行面板（RunPane）在底部窗内的精确头部度量**未逐项核定：本文仅记录 `terminal-tab-bar.tsx:465,712-713`（`h-8` / `min-h-8` 标签条、`border-border border-b bg-surface px-2 py-1.5`）与 `run-pane.tsx:394-417`（`border-border/70 border-b px-3 py-2` 详情头、`px-3 py-2` 输出区），完整度量表未做。
7. **`DiagnosticsPane`（问题面板）的完整头部/列表度量**未核定：仅知 `diagnostics-pane.tsx:899`（`min-h-7` 行）、`:1077`（`border-border/60 border-b bg-background/70 px-2 py-1`）、`:1212`（`h-44` 内联变体）。
8. **`bottomPaneActiveTab` 的完整取值类型定义**未定位（`ui-state.store.ts` 中的 union 类型行号未确认），本文的取值集是从 `bottom-pane.tsx` 与 `command-palette/constants/view-actions.tsx` 反推。
9. **底部窗高度是否持久化**：`bottom-pane.tsx:47` 是 `useState(320)`，未见写回 settings/store 的代码；是否在别处持久化未查清（倾向"不持久化"）。
10. **`--app-ui-scale` 的运行时来源**（缩放设置如何写入 CSS 变量）未定位；本文所有 px 换算都假设 `--app-ui-scale: 1`。
11. **gpui-kit 是否有"百分比初始面板尺寸"**：`ResizablePanel::size` 只接受 `Pixels`（`gpui-base-0.6.6/src/resizable/panel.rs:275`），未在 0.6.6 中发现按比例初始化的 API；如存在 dock 的 `adopt_sizes` 路径（`resizable/mod.rs:137`，`pub(crate)`）亦不可用，故一律按"需自行把百分比换算成像素"处理。
12. **gpui-kit 对"仅悬停显示"子控件（暂存按钮 `opacity-0 → group-hover:opacity-100`）的原生支持**：未见 group-hover 等价物，需自管 hover 状态；未在 0.6.6 中找到现成 API。
