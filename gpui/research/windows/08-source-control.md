# 08 · Windows 前端界面规格：源代码管理视图（左侧栏「更改」）

> 用途：供团队用 **Rust + gpui-kit 0.6.6** 重写桌面界面时"照着写代码"。
> 唯一真源：`windows/tauri/src/`（Tauri v2 + React + TypeScript + Tailwind v4 + shadcn/Base UI）。
> 证据格式：`相对路径:行号`。查不到的一律写"未找到"，不做推测。
> 本文为只读调研产物，未修改任何既有文件。

**本文聚焦"左侧栏那一栏"（源代码管理视图）**，即 `SidebarView === "git"` 的那个面板。
底部工具窗「提交记录」（`GitLogToolWindow`）的规格已在 `gpui/research/windows/03-git-and-bottom.md` 写完，
**本文不重复**其组件树、泳道图、引用树、控制台、Inspector 度量；只说明两者的**分工关系**（§1.3）。
度量表同样不重抄：见 `03-git-and-bottom.md` §2.7（变更列表度量）与 `gpui/UI-MAP-WINDOWS.md` §1.5。
全 Shell / 活动栏 / 状态栏度量见 `01-shell.md` 与 `04-theme-and-components.md`。

---

## 1. 入口与容器

### 1.1 入口：左活动栏的 `git` 项

| 项 | 值 | 证据 |
| --- | --- | --- |
| 活动栏项 id 全集 | `["files","git","search","maven","run","terminal","diagnostics","gitLog","settings"]` | `windows/tauri/src/features/layout/config/item-order.ts:1-11` |
| **打开源代码管理视图的 id** | **`"git"`**（不是 `gitLog`） | `sidebar-pane-selector.tsx:151-167` |
| 图标 | `GitBranchIcon`（`gpui` 侧已映射 `IconName::GitBranch`） | `sidebar-pane-selector.tsx:156`；`gpui/crates/workbench/src/activity_bar.rs:39` |
| 文案 | `workbench.changes` = **更改** | `sidebar-pane-selector.tsx:155`；`i18n/locale.ts:4606` |
| tooltip 快捷键标注 | `Mod+Shift+G` | `sidebar-pane-selector.tsx:160-164` |
| 显示条件 | `coreFeatures.git` | `sidebar-pane-selector.tsx:151`；`features/settings/config/features.ts:19`（默认 `coreFeatures.git: true`，`default-settings.ts:160`） |
| 点击回调 | `onViewChange("git")` | `sidebar-pane-selector.tsx:158` |
| 选中态判定 | `isPrimarySidebarItemActive && isGitViewActive`（`isPrimarySidebarItemActive = isSidebarVisible && !isSearchActive`） | `sidebar-pane-selector.tsx:109,157` |
| 顶部组 / 底部组归属 | `git` 在**顶部组**；`gitLog` 在**底部组**（`SIDEBAR_BOTTOM_ACTIVITY_ITEM_IDS`） | `item-order.ts:12-19`；`sidebar-pane-selector.tsx:311-319`（gpui 侧已按此分组，见 `gpui/crates/workbench/src/activity_bar.rs:136-138`） |

**`"git"` 与 `"gitLog"` 是两个不同的活动栏项、两个不同的目标区域**（`item-order.ts:3,9`）。
点 `git` 打开**左侧栏**；点 `gitLog` 打开**底部工具窗**（`main-sidebar.tsx:642-643` → `view-command-actions.ts:57-65`）。

### 1.2 容器：左栏面板宿主

```
MainSidebar                                    windows/tauri/src/features/layout/components/sidebar/main-sidebar.tsx:767
└─ SidebarPanel（左栏内容面板，flex-1 overflow-hidden）
   └─ <activePane.content>                     main-sidebar.tsx:806-817
      └─ activePaneId = isGitViewActive ? "git" : activeSidebarView     main-sidebar.tsx:777
         └─ GitView                            main-sidebar.tsx:784-788
```

- `GitView` 是**左侧栏的一个 pane 内容**，由 `MainSidebar` 按 `activePaneId` 单选渲染（`main-sidebar.tsx:778-816`）。
- 左栏外壳是 `ResizablePane position="left"`（宽度、最小宽 140、拖拽热区见 `01-shell.md` §2.8 与 `main-layout.tsx:303-310`）。
- **没有** `sidebar-pane-selector.tsx` 的独立容器组件：该文件只画**活动栏的图标项**（`SidebarPaneSelector`，`:81`），
  不承载视图内容；内容由 `MainSidebar` 的 `paneEntries` 表选择（`main-sidebar.tsx:778-816`）。
- 视图根是 `SidebarPanel`：`flex h-full min-h-0 min-w-0 w-full flex-col bg-background`（`ui/sidebar.tsx:18`），
  这里追加 `font-sans ui-text-sm select-none`（`git-view.tsx:706`）。

### 1.3 与底部 Git 工具窗的关系（谁是补充）

**结论：左栏是"工作区现状 + 提交入口"，底部窗是"历史阅读器 + 命令输出台"。两者是互补，不是二选一。**

| 维度 | 左栏「更改」视图（本文） | 底部窗「提交记录」（`03-git-and-bottom.md`） |
| --- | --- | --- |
| 入口项 | 活动栏 `git`（顶部组） | 活动栏 `gitLog`（底部组） |
| 宿主 | `ResizablePane position="left"` 内的 `SidebarPanel` | `BottomPane`（默认挂在中央编辑器列内） |
| 根组件 | `GitView`（`git-view.tsx:82`） | `GitLogToolWindow`（`log/git-log-tool-window.tsx:78`） |
| 核心内容 | 工作区变更列表 + 提交信息输入 + 提交按钮（§2） | 引用树 / 提交表（含泳道图）/ Inspector / 控制台 |
| 自带页签 | `changes`（更改）与 `history`（历史记录）两个**左栏内的**子页签（`git-view.tsx:712-766`） | `log`（日志）与 `console`（控制台）两个子页签（`03-git-and-bottom.md` §3.1） |
| 共享的数据源 | 同一个 `useGitStore`（`gitStatus` / `commits` / `branches` / `stashes` / `operationState`） | 同上 |
| 交叉入口 | 「更多」菜单可打开分支管理 / 远程 / 标签 / 贮藏；「查看差异」来源含"提交"（走 `GitCommandSurface` 选提交） | 引用树右键可 checkout / merge 等；标题栏可刷新 |

补充关系里最直接的两处：

1. 左栏内嵌的 `history` 子页签（`git-view.tsx:754-764`）是**极简提交列表**（`GitCommitHistory`，
   头像 + 消息 + 作者 + 相对时间 + 短哈希，`git-commit-history.tsx:96-146`），
   与底部窗的**多栏提交表 + 泳道图 + Inspector** 是两个不同深度的实现，功能重叠但不等价。
2. 底部窗的**控制台**是 Git 执行输出（`git-execution-console.tsx`），左栏没有控制台；
   左栏的**暂存 / 提交 / 贮藏 / 丢弃**是写操作入口，底部窗没有这些按钮（只有右键菜单里的 checkout/merge 系）。

### 1.4 打开路径汇总（除活动栏图标外）

| 路径 | 证据 |
| --- | --- |
| 快捷键 `Mod+Shift+G`（命令 `workbench.showSourceControl`，keybinding 写作 `cmd+shift+g`） | `features/keymaps/commands/command-registry.ts:671-677`；动作体 `view-command-actions.ts:121-129`（可见且已是 git → 收起；否则 `setActiveView("git")` + `setIsSidebarVisible(true)`） |
| 状态栏 `gitChanges` chip（旧位置在 Footer 尾组） | `features/layout/components/footer/footer-editor-status.tsx:122-141`（`onClick={() => openSidebarView("git")}`，`:127`）；`item-order.ts:21-28` |
| 命令面板「转到源代码管理」等 | `features/command-palette/constants/navigation-actions.tsx:56`；`constants/git-actions.tsx:58,71` |
| 空态里的「打开更改面板」按钮 | `git-repository-empty-state.tsx:36-43,124-126` |
| 折叠/展开语义 | `resolveSidebarPaneClick`：不可见→显示；已是该视图→**折叠**；其它→切换（`layout/utils/sidebar-pane-utils.ts:47-71`）；活动栏点击处理 `features/layout/hooks/use-sidebar-pane-controller.ts:26-48` |

---

## 2. 视图结构（自上而下逐段）

`GitView` 有 **5 种顶层返回分支**（空态/加载/失败各一套，正常一套）。下面先给正常态结构，再给异常态（§5）。

### 2.1 正常态组件树

```
GitView                                          windows/tauri/src/features/git/components/git-view.tsx:82
└─ SidebarPanel  className="font-sans ui-text-sm select-none"                        :706
   ├─ ① SidebarTitleBar  title="源代码管理"                                          :707-710
   │  ├─ 刷新按钮（SidebarHeaderIconButton，icon-xs 24×24）                            :480-493
   │  └─ 「更多」按钮（SidebarHeaderIconButton，icon-xs）                              :459-478
   ├─ ② [showLoadError] 错误条                                                        :677-685, :711
   ├─ ③ SidebarTabBar  items=[{changes}, {history}]                                  :712
   │  └─ div.flex.min-h-0.flex-1.flex-col.overflow-hidden.isolate                    :713
   │     ├─ SidebarTabPanels                                                         :714-766
   │     │  ├─ tab "changes"（workbench.changes=更改）                                :717-753
   │     │  │  ├─ ④ GitOperationBanner（仅当有进行中的 merge/rebase/cherry-pick/revert）:721
   │     │  │  └─ ⑤ GitStatusPanel  (status/git-status-panel.tsx)                    :722-750
   │     │  └─ tab "history"（git.history=历史记录）                                  :754-764
   │     │     └─ GitCommitHistory                                                    :757-762
   │     └─ ⑦ SidebarFooter                                                          :768
   │        └─ GitCommitPanel  (git-commit-panel.tsx)                                :769-787
   ├─ ⑧ GitActionsMenu（浮层，「更多」按钮的下拉）                                      :793
   ├─ ⑨ GitCommandSurface × 3（提交差异选择 / 分支比较选择 / 贮藏列表）                   :794-982
   ├─ ⑩ GitRemoteManager（模态）                                                      :984-989
   └─ ⑪ GitTagManager（模态）                                                         :991-997
```

### 2.2 逐段说明 + 中文文案与 i18n 键

> 文案逐字取自 `windows/tauri/src/i18n/locale.ts` 的 **zh-CN 段**（该段从 `locale.ts:4468` 起），
> 行号写在每条后面。`{...}` 是运行期插值。

**① 标题栏（`SidebarTitleBar`）**

| 元素 | 文案 / 键 | 证据 |
| --- | --- | --- |
| 标题 | **源代码管理** — `workbench.sourceControl` | `git-view.tsx:707`；`locale.ts:5909` |
| 刷新按钮 tooltip | **刷新状态** — `git.refreshStatus`（`aria-label` 用 `git.refreshAria`） | `git-view.tsx:481-492`；`locale.ts:8636` |
| 刷新按钮禁用 | `disabled = isLoadingGitData \|\| isRefreshing`；加载中换成 `Spinner` | `git-view.tsx:483-491` |
| 「更多」按钮 tooltip | **Git 操作**（键 `git.actions`，本轮未在 locale 中核到 → **未确认**，见 §7） | `git-view.tsx:474` |
| 标题栏度量 | 高 36（`--lithe-pane-header-height`）、px 12、动作区 max-w 50%、gap 4 | `ui/sidebar.tsx:38,43,47`；`UI-MAP-WINDOWS.md` §1.5 末两行 |

**② 加载失败错误条**（条件渲染：`hasLoadError || (activeTab === "history" && hasHistoryLoadError)`，`git-view.tsx:677`）

| 元素 | 文案 / 键 | 证据 |
| --- | --- | --- |
| 文本 | `hasLoadError` → **无法加载 Git 数据，请重试刷新仓库。** — `git.statusLoadFailed`<br>`hasHistoryLoadError` → **无法加载 Git 历史。** — `git.historyLoadFailed` | `git-view.tsx:680`；`locale.ts:7011` / `locale.ts:8536` |
| 重试按钮 | **重试** — `git.log.retry`；`disabled={isRefreshing}` | `git-view.tsx:681-683`；`locale.ts:7197` |
| 外观 | `role="alert"`、`p-3 ui-text-sm text-destructive` | `git-view.tsx:679` |

**③ 子页签条（`SidebarTabBar`）**

| 页签 | 文案 / 键 | 证据 |
| --- | --- | --- |
| `changes` | **更改** — `workbench.changes` | `git-view.tsx:621-624`；`locale.ts:4606` |
| `history` | **历史记录** — `git.history` | `git-view.tsx:625-628`；`locale.ts:6814` |
| 顺序 | 按设置 `gitSidebarTabOrder`（默认 `["changes","history"]`）过滤并强制 `changes` 在前 | `git-view.tsx:612-633`；`default-settings.ts:210` |
| 初始页签 | 固定 `"changes"`；若 `rememberLastGitPanelMode` 为真则恢复到 `gitLastPanelMode`（默认 `"changes"`），切换时回写 | `git-view.tsx:141,333-343`；`default-settings.ts:203,209` |
| 内容切换 | `SidebarTabPanels`；`filter` 掉 `gitTabs` 里没有的项 | `git-view.tsx:714-766` |

**④ 操作横幅（`GitOperationBanner`，整段只在有进行中操作时出现）**

| 元素 | 文案 / 键 | 证据 |
| --- | --- | --- |
| 标题（按 kind） | **合并进行中** `git.mergeInProgress` / **变基进行中** `git.rebaseInProgress` / **拣选进行中** `git.cherryPickInProgress` / **还原进行中** `git.revertInProgress` | `git-operation-banner.tsx:57-62`；`locale.ts:7034-7037` |
| 引用短哈希 | `— <reference 前 7 位>` | `git-operation-banner.tsx:79-83` |
| 进度 | **第 {step} / {total} 步。** — `git.operationStep` | `git-operation-banner.tsx:89-92`；`locale.ts:7042` |
| 冲突提示 | **解决 {count} 个冲突文件，暂存后继续。** — `git.resolveConflicts` | `git-operation-banner.tsx:97-100`；`locale.ts:7043` |
| 无冲突提示 | **所有冲突已解决。继续完成，或中止以撤销。** — `git.conflictsResolved` | `git-operation-banner.tsx:103`；`locale.ts:7044` |
| 变基会话按钮（仅 rebase） | **交互式变基会话…** — `git.rebasePlan.session` | `git-operation-banner.tsx:108`；`locale.ts:6843` |
| 继续按钮（按 kind） | **继续合并 / 继续变基 / 继续拣选 / 继续还原** — `git.continueMerge / continueRebase / continueCherryPick / continueRevert`；`disabled={isResolving \|\| hasConflicts}` | `git-operation-banner.tsx:63-68,109-117`；`locale.ts:7038-7041` |
| 跳过按钮（仅 rebase） | **跳过提交** — `git.skipCommit` | `git-operation-banner.tsx:118-128`；`locale.ts:7045` |
| 中止按钮 | **中止** — `git.abort`（`text-git-deleted`） | `git-operation-banner.tsx:129-137`；`locale.ts:7046` |
| 失败 toast | **Git 操作失败** — `git.operationFailed` | `git-operation-banner.tsx:43`；`locale.ts:7033` |
| 外观 | `flex flex-col gap-2 border-b border-border bg-raised px-3 py-2.5`、`role="status"`、图标 `WarningIcon size-3.5 text-git-modified` | `git-operation-banner.tsx:71-84` |
| 设计意图（源码注释原文） | 刻意**不做成对话框**：用户编辑冲突文件期间这些控件必须一直可达（对齐 macOS 侧栏横幅） | `git-operation-banner.tsx:21-25` |

**⑤ 变更列表（`GitStatusPanel`）——本视图的主体**

```
GitStatusPanel                                            status/git-status-panel.tsx:1086
├─ [hasFiles] SidebarToolbar  h=36px                       :1105-1182
│  ├─ 左：ButtonGroup["查看差异" git.viewDiff | caret 下拉]   :1108-1142
│  │  └─ Dropdown（差异来源，min-w 150）                     :1135-1142
│  └─ 右：SidebarHeaderIconButton × 3                        :1144-1181
│     ├─ 贮藏全部未暂存（仅 unstagedFiles>0）git.stashAllUnstaged :1145-1156
│     ├─ 暂存所有更改（仅 unstagedFiles>0）git.stageAllChanges   :1157-1168
│     └─ 取消暂存所有更改（仅 stagedFiles>0）git.unstageAllChanges :1169-1180
├─ ScrollArea（contentClassName="px-2 py-2"）               :1183-1188
│  └─ SidebarTree role="tree"（虚拟滚动，overscan 12）        :1189-1217
│     ├─ 分类头 SidebarSectionHeader variant="surface"        :765-776, :858-864
│     │  ├─ "tracked"   → 已跟踪 git.tracked                 :861
│     │  └─ "untracked" → 未跟踪 git.untracked               :861
│     ├─ 文件行 GitFileItem                                 :866-895
│     └─ 文件夹行 SidebarTreeRow + Checkbox（仅 folder view 开）:906-970
│     └─ 行间 spacer（分区间 8、分类头下 2）                    :139-141, :374-385
└─ [无文件] Empty tone="success" → 工作区干净 git.workingTreeClean :1220-1226
```

| 元素 | 文案 / 键 | 证据 |
| --- | --- | --- |
| 「查看差异」按钮 | **查看差异** — `git.viewDiff`；`disabled={!onViewDiff \|\| isLoading}` | `git-status-panel.tsx:1110-1119`；`locale.ts:6816` |
| caret 下拉 aria | **选择差异来源** — `git.chooseDiffSource` | `git-status-panel.tsx:1128`；`locale.ts:7153` |
| 差异来源项 | **未暂存** `git.unstaged`? / **已暂存** `git.staged` / **提交** `git.commit` / **分支** `git.branch` / **贮藏** `git.stash`；两项之间一条分隔线 | `git-status-panel.tsx:1031-1079`；`locale.ts:8643,8644,6832,8645,8646` |
| 差异来源禁用 | `unstaged` 需 `hasUnstagedDiffableFiles`；`staged` 需 `hasStagedDiffableFiles`；`commit/branch/stash` 需对应回调存在；全部叠加 `isLoading` | `git-status-panel.tsx:1036,1042,1049,1056,1063` |
| 贮藏全部未暂存 | **贮藏全部未暂存** — `git.stashAllUnstaged` | `git-status-panel.tsx:1150`；`locale.ts:8648` |
| 暂存所有更改 | **暂存所有更改** — `git.stageAllChanges` | `git-status-panel.tsx:1162`；`locale.ts:7154` |
| 取消暂存所有更改 | **取消暂存所有更改** — `git.unstageAllChanges` | `git-status-panel.tsx:1174`；`locale.ts:7155` |
| 分类头 | **已跟踪** `git.tracked` / **未跟踪** `git.untracked`（带 `Badge variant="muted" size="compact"` 计数） | `git-status-panel.tsx:858-864`；`locale.ts:6817,6818`；`ui/sidebar.tsx:327-348` |
| 树容器 aria-label | `"已跟踪文件 / 未跟踪文件"` — `git.trackedFiles` / `git.untrackedFiles` 拼接 | `git-status-panel.tsx:1190`；`locale.ts:7156,7157` |
| 文件夹行尾计数 | **{count} 个文件** — `git.diffFileCount` | `git-status-panel.tsx:925-931`；`locale.ts:7130` |
| 空态 | **工作区干净** — `git.workingTreeClean`（`Empty tone="success"` + `Check` 图标） | `git-status-panel.tsx:1220-1226`；`locale.ts:7158` |
| 分组维度 | 两个 section：`tracked` / `untracked`；每 section 内先按 `GIT_STATUS_ORDER` 分组；`gitChangesFolderView`（默认 true）时改为目录树（`buildGitFolderTree` + `compactPathTreeBranch`） | `git-status-panel.tsx:106,334-420`；`default-settings.ts:193`；`utils/git-status-model.ts` |
| 行高 | `max(24, uiFontSize × 1.35 + 6)`；默认 13px → **24**；分类头 **32**；分区之间 8、分类头下 2 | `features/file-explorer/lib/file-tree-row.ts:1-14`；`git-status-panel.tsx:139-141,427-429` |
| 未跟踪文件开关 | 设置 `showUntrackedFiles`（默认 true）；关掉后由 `buildVisibleGitFiles` 过滤 | `git-view.tsx:133,177`；`default-settings.ts:196` |
| 列表是否置顶已暂存 | 设置 `showStagedFirst`（默认 true） | `default-settings.ts:197`（消费点在 `utils/git-status-model.ts`） |

**⑥ 提交信息输入区（`SidebarFooter` > `GitCommitPanel`）**

```
SidebarFooter（mx-2 mb-2 rounded-xl border border-border/60 bg-background）   ui/sidebar.tsx:75
└─ GitCommitPanel                                                          git-commit-panel.tsx:57
   ├─ [error] 错误块（mx-2 mt-2 rounded-md border-destructive/30 …）         :264-274
   ├─ SidebarComposerBody → Textarea（rows=2, min-h 64, max-h 128, 自动增高）:276-291
   └─ 底部行（flex flex-wrap items-center gap-x-2 gap-y-1 px-1 pt-1.5）      :293-419
      ├─ 左：选中文件计数 + ahead/behind 按钮                                 :294-336
      └─ 右：AI 设置 / AI 生成 / [提交 | caret]                              :338-418
```

| 元素 | 文案 / 键 | 证据 |
| --- | --- | --- |
| 输入框 placeholder | **提交说明...** — `git.commitMessagePlaceholder` | `git-commit-panel.tsx:281`；`locale.ts:6819` |
| 输入框禁用 | `disabled={isCommitting}` | `git-commit-panel.tsx:289` |
| 自动增高 | `min-h 64 / max-h 128`，按 `scrollHeight` 夹取；超出则 `overflow-y: auto` | `git-commit-panel.tsx:54-55,102-114` |
| 选中文件计数 | **已选择 {count} 个文件** — `git.filesSelected`（单/复数键 `git.fileSelected` 中文同值） | `git-commit-panel.tsx:296-300`；`locale.ts:6827,6828` |
| 未选文件 | **没有选择文件** — `git.noFilesSelected` | `git-commit-panel.tsx:300`；`locale.ts:6829` |
| ahead 按钮 | 图标 `ArrowUp` + 计数；tooltip 英文硬编码 `Push N commit(s)`；`text-git-added`；仅 `ahead>0` 时显示 | `git-commit-panel.tsx:305-318` |
| behind 按钮 | 图标 `ArrowDown` + 计数；tooltip 英文硬编码 `Pull N commit(s)`；`text-git-deleted`；仅 `behind>0` 时显示 | `git-commit-panel.tsx:320-333` |
| AI 设置按钮 | tooltip **AI 提交设置**（键 `aiCommit.settings`，未逐字核） | `git-commit-panel.tsx:339-347` |
| AI 生成按钮 | tooltip **使用 AI 生成提交说明** — `git.generateCommitMessageWithAI`；文案 `AI` | `git-commit-panel.tsx:361-372`；`locale.ts:6820` |
| AI 生成中按钮 | **取消** — `aiCommit.cancel` | `git-commit-panel.tsx:348-359` |
| 提交按钮 | **提交** — `git.commit`；提交中 → **正在提交...** — `git.committing` | `git-commit-panel.tsx:388`；`locale.ts:6832,6835` |
| 提交 caret | tooltip **选择提交操作** — `git.chooseCommitAction` | `git-commit-panel.tsx:402`；`locale.ts:6834` |
| 提交下拉唯一项 | **提交并推送** — `git.commitAndPush`（`ArrowUp` 图标） | `git-commit-panel.tsx:248-259`；`locale.ts:6833` |

**⑦ 分支与同步状态在哪儿？**

**左栏里没有分支管理 UI，也没有专门的"同步状态"条。** 这是与直觉最容易不符的一点：

| 事实 | 证据 |
| --- | --- |
| 分支（+ ahead/behind）是**标题栏**的一项，不在左栏：`useFooterGitBranchItem()` 由 `title-bar.tsx` 消费 | `features/layout/components/footer/footer-git-branch-item.tsx:11-68`；`features/window/components/title-bar/title-bar.tsx:66,238`；`01-shell.md` §2.7 末 |
| 该触发器内渲染完整的分支面板 `GitBranchManager`（`triggerSurface="footer"`） | `footer-git-branch-item.tsx:32-67` |
| 左栏能触达分支管理的路径：①「更多」菜单「管理分支」→ 自定义事件 `lithe:open-branch-manager`；②「查看差异」下拉里的"分支"（只做分支比较，不切换） | `git-view.tsx:69,345-350,552-553`；`git-actions-menu.tsx:214-225` |
| 左栏里 **ahead / behind 的唯一展示**是提交面板底部那两个计数按钮 | `git-commit-panel.tsx:305-333` |
| `history` 子页签顶部另有一行文字提示（**{count} 个本地提交未推送** / **{count} 个远程提交未拉取** — `git.localCommitsNotPushed` / `git.remoteCommitsNotPulled`） | `git-commit-history.tsx:442-457` |
| 底部窗标题栏有独立的「引用名」胶囊（可切分支） | `03-git-and-bottom.md` §2.2；`log/git-log-title-bar.tsx:32-39` |

**⑧ 「更多」菜单（`GitActionsMenu`）全项**（有仓库时；`git-actions-menu.tsx:204-333`）

| 项 | 键 / 中文 | 行号 | 禁用 |
| --- | --- | --- | --- |
| 选择仓库 | `git.selectRepository` 选择仓库 / `git.selecting` 正在选择... | `:208` | `isSelectingRepository` |
| 管理分支 | `git.manageBranches` 管理分支 | `:216` | — |
| 显示分支差异 | `git.showBranchDiff` 显示分支差异 | `:222` | — |
| 推送更改 | `git.pushChanges` 推送更改 | `:229` | `isLoading \|\| isPulling` |
| 拉取更改 | `git.pullChanges` 拉取更改 | `:237` | `isLoading \|\| isPullLocked` |
| 获取 | `git.fetch` 获取（toast 用 `git.fetchingChanges` 正在获取更改... / `git.changesFetched` 已成功获取。 / `git.fetchFailed` 获取更改失败。） | `:244`, `:135-139` | `isLoading \|\| isPulling` |
| 管理远程 | `git.manageRemotes` 管理远程 | `:252` | — |
| 管理标签 | `git.manageTags` 管理标签 | `:258` | — |
| 查看贮藏 | `git.viewStashes` 查看贮藏 | `:264` | — |
| 刷新状态 | `git.refreshStatus` 刷新状态 | `:271` | `isRefreshing` |
| 管理工作树… | `git.worktreeDialog.manage` 管理工作树… | `:279` | `isLoading` |
| 交互式变基会话… | `git.rebasePlan.session` 交互式变基会话… | `:286` | `isLoading` |
| 创建补丁… | `git.patch.create` 创建补丁… | `:293` | `isLoading` |
| 应用补丁… | `git.patch.apply` 应用补丁… | `:300` | `isLoading` |
| 丢弃全部更改（destructive 色） | `git.discardAllChanges` 丢弃全部更改；确认框 `git.discardChangesConfirm` 丢弃所有未暂存的更改吗？此操作无法撤销。 | `:308`, `:146-153` | `isLoading` |
| 「更多」菜单（无仓库时） | 初始化仓库 `git.initializeRepository` 初始化仓库 / `git.initializing` 正在初始化... + 刷新状态 | `:315-333` | `isLoading \|\| isInitializingRepository` |

**⑨ 三个命令面板（`GitCommandSurface`）**：提交差异选择（placeholder `git.searchCommits` 搜索提交；meta `git.commitCount`/`git.commitsCount`；
空态 `git.noMatchingCommits` / `git.noCommits`）、分支比较选择（`git.compareBranchPlaceholder`、`git.branchCount`/`git.branchesCount`、`git.compareWithBranch`、`git.noMatchingBranches`/`git.noOtherBranches`）、
贮藏列表（`git.searchStashes`、`git.stashCount`/`git.stashesCount`、`git.noMatchingStashes`/`git.noStashes`，每行三个动作 `git.applyStash` 应用贮藏 / `git.popStash` 弹出贮藏 / `git.dropStash` 删除贮藏）。
证据：`git-view.tsx:794-982`。这些键的 zh 值见 `locale.ts`（`git.applyStash:7030`、`git.popStash:7031`、`git.dropStash:7032`）。

---

## 3. 交互清单（逐条：动作 / 快捷键 / 禁用条件）

### 3.1 视图与页签级

| # | 动作 | 快捷键 | 禁用条件 | 证据 |
| --- | --- | --- | --- | --- |
| 1 | 打开/折叠源代码管理视图 | `Mod+Shift+G` | `coreFeatures.git` 为假时活动栏不显示该项 | `command-registry.ts:671-677`；`view-command-actions.ts:121-129`；`sidebar-pane-selector.tsx:151` |
| 2 | 切换到 `history` 子页签 | — | 无 | `git-view.tsx:712,754-764` |
| 3 | 切回 `changes` 子页签 | — | 无 | 同上 |
| 4 | 刷新（标题栏按钮） | — | `isLoadingGitData \|\| isRefreshing` | `git-view.tsx:483` |
| 5 | 刷新（错误条里的"重试"） | — | `isRefreshing` | `git-view.tsx:681` |
| 6 | 刷新（「更多」菜单"刷新状态"） | — | `isRefreshing` | `git-actions-menu.tsx:273` |
| 7 | 打开「更多」菜单 | — | 无（无仓库时菜单内容不同） | `git-view.tsx:459-478` |

### 3.2 变更列表（`GitStatusPanel`）

| # | 动作 | 触发 | 快捷键 | 禁用条件 | 证据 |
| --- | --- | --- | --- | --- | --- |
| 8 | 暂存/取消暂存单个文件 | 行内 `GitFileStageAction`（行 hover 才显形，`size-5`，pending 时换成旋转指示） | — | `disabled`（`isLoading`）或该文件 `stagePending` | `status/git-status-file-item.tsx:52-90,145-155` |
| 9 | 暂存/取消暂存单个文件夹 | 文件夹行右侧 `Checkbox` 之外的 stage 按钮不存在；文件夹用复选框批量选择提交范围 | — | 见 #15 | `git-status-panel.tsx:906-957` |
| 10 | 勾选/取消勾选文件的"提交范围" | 行内 `Checkbox` | — | `disabled={isLoading}` | `git-status-file-item.tsx:156-165` |
| 11 | 暂存所有更改 | 工具行右侧 `Plus` | — | `isLoading` | `git-status-panel.tsx:1157-1168` |
| 12 | 取消暂存所有更改 | 工具行右侧 `Minus` | — | `isLoading`；仅 `stagedFiles>0` 时出现 | `git-status-panel.tsx:1169-1180` |
| 13 | 贮藏全部未暂存 | 工具行右侧 `Archive`；弹 `StashMessageModal` | — | `isLoading \|\| stagePendingPaths.size>0`；仅 `unstagedFiles>0` 时出现；多个仓库时 toast `git.selectSingleRepositoryForStash` 并中止 | `git-status-panel.tsx:702-715,1145-1156` |
| 14 | 查看差异（全部） | 「查看差异」主按钮 | — | `!onViewDiff \|\| isLoading` | `git-status-panel.tsx:1110-1119` |
| 15 | 查看差异（按来源） | caret → 未暂存 / 已暂存 / 提交… / 分支… / 贮藏… | — | 逐项见 §2「差异来源禁用」 | `git-status-panel.tsx:1031-1079` |
| 16 | 打开文件差异（单击文件行） | 行单击 | — | — | `git-status-panel.tsx:873-878` → `git-view.tsx:702`（`openDiffOnClick` 默认 true 时开差异，否则打开原文件） |
| 17 | 打开文件夹差异（单击文件夹行） | 文件夹行单击 | — | — | `git-status-panel.tsx:915-920` |
| 18 | 折叠/展开文件夹 | 行 disclosure 按钮 或 双击 | — | 仅 `gitChangesFolderView` 开时存在文件夹行 | `git-status-panel.tsx:914,921` |
| 19 | 折叠/展开分类（已跟踪/未跟踪） | 分类头 | — | 两类均为 0 时该分类头不渲染 | `git-status-panel.tsx:755-776,374` |
| 20 | 多选行（用于右键菜单作用域） | `Ctrl`/`Cmd` + 单击 | — | — | `git-status-panel.tsx:732-736` |
| 21 | 打开右键菜单 | 行右键 / 空白区右键（空白区 → 单项 `ui.noActionsHere`=**此处无任何内容** 且 disabled） | — | — | `git-status-panel.tsx:738-742,1089,1229-1241`；`locale.ts:4630` |
| 22 | 键盘导航 | 树内 `ArrowDown/ArrowUp/Home/End`；`ArrowRight`/`ArrowLeft` 折叠展开文件夹并跨层移动 | `ArrowUp/Down/Home/End/ArrowLeft/ArrowRight` | 目标行必须是 `folder` 或 `file` 行 | `git-status-panel.tsx:797-854` |
| 23 | 删除选中文件 | `Delete` 键（焦点在 `role="treeitem"` 内） | `Delete` | `isLoading \|\| stagePendingPaths.size>0 \|\| 无选中` | `git-status-panel.tsx:1090-1103` |
| 24 | 拖拽文件行到编辑器等 | 行拖拽（仅 `repoPath` 存在时 `draggable`） | — | 负载见下 | `git-status-file-item.tsx:168-179`（`type:"git-file-diff"`，带 `repoPath/filePath/staged/status`）；文件夹 `git-status-panel.tsx:958-967`（`type:"file"`） |

**右键菜单全项**（`git-status-panel.tsx:1242-1367`）

| 项 | 键 / 中文 | 出现条件 | 禁用条件 | 行号 |
| --- | --- | --- | --- | --- |
| 暂存（按目标命名） | `git.stageFile` 暂存文件 / `git.stageFileNamed` 暂存 {name} / `git.stageFolder` 暂存文件夹 {name} | 选中里含未暂存文件 | `isLoading` | `:1243-1253`, `:1006-1018` |
| 取消暂存 | `git.unstageFile` 取消暂存文件 / `git.unstageFileNamed` 取消暂存 {name} / `git.unstageFolder` 取消暂存文件夹 {name} | 选中里含已暂存文件 | `isLoading` | `:1255-1266` |
| 创建补丁… | `git.patch.create` 创建补丁…（多仓库时显示 `git.patch.singleRepository`=创建补丁需要选择同一仓库中的文件） | 恒有 | `isLoading \|\| 选中仓库数 ≠ 1` | `:1267-1276`；`locale.ts:8549,8561` |
| 提交 | `git.commit` 提交 | 恒有 | `无选中 \|\| isLoading \|\| stagePendingPaths.size>0` | `:1277-1283` |
| 回滚 | `git.rollback` 回滚（确认 `git.rollbackPathsConfirm`=确定回滚选中的 {count} 个路径吗？此操作无法撤销。，仅当设置 `confirmBeforeDiscard` 为真，默认 true） | 恒有 | `无已跟踪文件 \|\| isLoading \|\| stagePendingPaths.size>0` | `:1284-1290`, `:523-555`；`locale.ts:7160,7161`；`default-settings.ts:194` |
| 显示差异 | `git.showDiff` 显示差异 | 恒有 | `无文件路径 \|\| isLoading` | `:1291-1297`；`locale.ts:7162` |
| 跳转到源 | `git.jumpToSource` 跳转到源 | 恒有 | `非单选 \|\| !onOpenPath` | `:1298-1316`；`locale.ts:7163` |
| 删除 | `git.delete` 删除（destructive 色；确认 `git.deleteFileConfirm`=删除"{name}"？ / `git.deleteFilesConfirm`=删除 {count} 个文件？） | 恒有 | `无可删路径 \|\| isLoading \|\| stagePendingPaths.size>0` | `:1317-1324`, `:557-602`；`locale.ts:6956,7164,7165` |
| 添加到 VCS | `git.addToVcs` 添加到 VCS | 含未跟踪文件 | `isLoading` | `:1325-1333`；`locale.ts:7168` |
| 添加到 .gitignore | `git.addToGitignore` 添加到 .gitignore / 多选时 `git.addSelectionToGitignore`=将选中的 {count} 个项目添加到 .gitignore（确认框 `git.addPathsToIgnoreConfirm`，目标名 `git.gitignoreFile`=`.gitignore`） | 含未跟踪文件 | `isLoading \|\| stagePendingPaths.size>0` | `:1334-1345`, `:613-666`；`locale.ts:7169,7171,7173,7175` |
| 添加到 .git/info/exclude | `git.addToLocalExclude` / `git.addSelectionToLocalExclude`（同上，目标 `git.localExcludeFile`） | 含未跟踪文件 | 同上 | `:1346-1357`；`locale.ts:7170,7172,7174` |
| 贮藏 | `git.stash` 贮藏（弹 `git.stashSelected`=搁置选中的更改；默认消息 `git.stashMessageDefaultSelection`） | 恒有 | `无选中 \|\| isLoading \|\| stagePendingPaths.size>0`；多仓库时 toast 中止 | `:1360-1366`, `:668-688`；`locale.ts:8646,7177` |

> **"丢弃"** 在文件级没有独立按钮：语义由右键 **回滚**（`git.rollback`，已跟踪文件）与仓库级 **丢弃全部更改**（`git.discardAllChanges`，在「更多」菜单）承担。

### 3.3 提交信息与提交（`GitCommitPanel`）

| # | 动作 | 快捷键 | 禁用条件 | 证据 |
| --- | --- | --- | --- | --- |
| 25 | 输入提交说明 | — | `disabled={isCommitting}` | `git-commit-panel.tsx:289` |
| 26 | 提交 | — | `isCommitDisabled = 未选文件 \|\| 说明为空 \|\| 有 operationState \|\| isCommitting \|\| isGenerating` | `git-commit-panel.tsx:236-241,374-389` |
| 27 | 提交（键盘） | **`Ctrl`/`Cmd` + `Enter`**（在输入框内） | 同 #26 | `git-commit-panel.tsx:229-234` |
| 28 | 提交并推送 | 提交右侧 caret → 唯一项 | `isCommitDisabled \|\| isRemoteActionLoading \|\| isPulling` | `git-commit-panel.tsx:248-259,391-408` |
| 29 | 推送（ahead 计数按钮） | — | `!repoPath \|\| isRemoteActionLoading \|\| isPulling`；仅 `ahead>0` 时出现 | `git-commit-panel.tsx:305-318` |
| 30 | 拉取（behind 计数按钮） | — | `!repoPath \|\| isRemoteActionLoading \|\| isPullLocked`；仅 `behind>0` 时出现 | `git-commit-panel.tsx:320-333` |
| 31 | AI 生成提交说明 | — | `isGenerateDisabled = 未选文件 \|\| isGenerating \|\| isCommitting \|\| !aiSettings.enabled`；未配置 provider 时设错并打开设置 | `git-commit-panel.tsx:116-145,242-243,364-365` |
| 32 | 取消 AI 生成 | — | 仅 `isGenerating` 时该按钮存在 | `git-commit-panel.tsx:348-359` |
| 33 | 打开 AI 提交设置 | — | 无 | `git-commit-panel.tsx:339-347` |

**提交前置校验（会写进面板内错误块的真实红线）**：

| 校验 | 文案 / 键 | 证据 |
| --- | --- | --- |
| 未选文件 | **请选择要提交的文件。** — `git.selectFilesToCommit` | `git-commit-panel.tsx:148-151`；`locale.ts:6830` |
| 跨仓库 | **请选择当前仓库中的文件进行提交。** — `git.selectSingleRepositoryForCommit` | `git-commit-panel.tsx:156-159`；`locale.ts:6831` |
| 有冲突未解决 | **请先解决冲突：{paths}** — `git.resolveConflictsFirst` | `git-commit-panel.tsx:163-168`；`locale.ts:6997` |
| 有进行中操作 | **请先完成或中止当前 Git 操作，再提交文件。** — `git.finishOperationBeforeCommit` | `git-commit-panel.tsx:169-172`；`locale.ts:6993` |
| 提交成功但有警告 | `git_index_reconcile_failed` → **提交已成功，但暂存区未能刷新。** — `git.commitIndexReconcileFailed`；其它警告原文展示 | `git-commit-panel.tsx:185-191`；`locale.ts:6994` |
| 推送失败 | **推送更改失败。** — `git.pushFailed` | `git-commit-panel.tsx:196-198`；`locale.ts:7021` |
| 未知错误 | `ai.unknownError` | `git-commit-panel.tsx:209` |

### 3.4 操作横幅（`GitOperationBanner`）

| # | 动作 | 快捷键 | 禁用条件 | 证据 |
| --- | --- | --- | --- | --- |
| 34 | 继续（merge/rebase/cherry-pick/revert） | — | `isResolving \|\| hasConflicts` | `git-operation-banner.tsx:109-117` |
| 35 | 跳过提交（仅 rebase） | — | `isResolving` | `git-operation-banner.tsx:118-128` |
| 36 | 中止 | — | `isResolving` | `git-operation-banner.tsx:129-137` |
| 37 | 打开交互式变基会话（仅 rebase） | — | `isResolving` | `git-operation-banner.tsx:108` |

### 3.5 空态里的动作（无仓库时）

| # | 动作 | 文案 / 键 | 禁用 | 证据 |
| --- | --- | --- | --- | --- |
| 38 | 浏览/选择仓库 | `git.browse` 浏览 / `git.selecting` 正在选择...（目录选择器 `open({directory:true})`，随后 `resolveRepositoryPath`） | `isSelectingRepo` | `git-view.tsx:516-530,263-293` |
| 39 | 初始化 Git 仓库 | `git.initialize` 初始化 / `git.initializing` 正在初始化...；tooltip 有目录时 `git.initializeGitRepository`，无目录时 `git.openFolderBeforeInitializing` | `!repoPath \|\| isInitializingRepo` | `git-view.tsx:495-514` |
| 40 | 打开更改面板（无提交仓库空态） | `git.setup.openChanges` 打开更改面板 | 无 | `git-repository-empty-state.tsx:124-126` |
| 41 | 初始化仓库（无仓库空态） | `git.setup.initialize` 初始化 Git 仓库（确认框 `git.setup.initializeTitle` / `git.setup.initializeDescription`） | `busy \|\| isRepository !== false` | `git-repository-empty-state.tsx:66-92,115-117` |
| 42 | 打开 Git 设置 | `git.setup.openSettings` 打开 Git 设置 | 无 | `git-repository-empty-state.tsx:132-136` |
| 43 | 打开 Git 控制台（可选回调） | `git.console.open` 打开 Git 控制台 | 无 | `git-repository-empty-state.tsx:129-131`；`locale.ts:4495` |
| 44 | 重试 | `git.log.retry` 重试 | 无 | `git-repository-empty-state.tsx:143-154` |

### 3.6 快捷键汇总（本视图相关）

| 快捷键 | 作用 | 证据 |
| --- | --- | --- |
| `Mod+Shift+G`（命令注册写作 `cmd+shift+g`） | 切换源代码管理视图 | `command-registry.ts:672-676` |
| `Ctrl`/`Cmd` + `Enter` | 在提交说明输入框内提交 | `git-commit-panel.tsx:230-233` |
| `Delete` | 删除选中的变更路径（焦点在树行内） | `git-status-panel.tsx:1090-1103` |
| `ArrowUp/Down/Home/End/ArrowLeft/ArrowRight` | 变更树键盘导航 / 折叠展开 | `git-status-panel.tsx:797-854` |
| `Ctrl`/`Cmd` + 单击 | 变更列表多选（右键菜单作用域） | `git-status-panel.tsx:734` |

> 未找证据：变更列表/提交面板的**其它**快捷键（如"暂存"的独立键位）在 `windows/tauri/src` 内未找到。

---

## 4. 数据来源与刷新时机

### 4.1 Windows 侧调用链（统一分发层）

前端 `invoke()` 命中 `nativeCommands` 白名单走真实 Tauri 命令，否则走通用命令 `platform_invoke`，
Rust 侧再把兼容命令名翻译成 Core 命令（`01-shell.md` §4.1）。

| 层 | 证据 |
| --- | --- |
| 前端包装 | `windows/tauri/src/platform/tauri-core.ts:93-128`（`@/platform/tauri-core`，按仓库规则**禁止**直接 import Tauri core API） |
| 原生命令白名单（含 git 元数据监听） | `tauri-core.ts:18-91`（`watch_git_repository` / `unwatch_git_repository` 在 `:80-81`） |
| 兼容名 → Core 名翻译 | `windows/tauri/src-tauri/src/platform.rs:211-596` |
| **Core 返回值 → 前端形状的适配器** | `windows/tauri/src/platform/core-result-adapter.ts:135-162`（`git_status` 把 Core 的 `data.changes[]` 映射成 `{branch, ahead, behind, files[]}`，并过滤 `status === "AD"`，把 porcelain 码归一成 `modified/added/deleted/untracked/renamed`，保留 `rawStatus`/`worktree`） |

### 4.2 本视图用到的命令（按用途）

| 用途 | Windows 包装函数 | invoke 名 | 翻译后的 Core 命令 | 证据 |
| --- | --- | --- | --- | --- |
| 读工作区状态（工作区聚合多仓库） | `getWorkspaceGitStatus(repoPaths, activeRepoPath, source)` | `git_status` | `git.status` | `api/git-status-api.ts:117-162`；`platform.rs:212`；`contracts.rs:532-538` |
| 读单仓库状态 | `getGitStatus(repoPath)` | `git_status` | `git.status` | `api/git-status-api.ts:34-79` |
| 暂存/取消暂存（文件集合） | `setFilesStaged(repoPath, paths, staged)` | `git.write` | `git.write`，`operation: "stage" \| "unstage"` | `api/git-status-api.ts:199-221` |
| 暂存单个文件 / 取消暂存单个文件（兼容路径，仍在使用） | `stageFile` / `unstageFile` | `git_add` / `git_reset` | `git.write`，`operation: "stage"` / `"unstage"`（`paths_from_file`） | `api/git-status-api.ts:164-196`；`platform.rs:229-242` |
| 暂存全部 / 取消暂存全部 | `stageAllFiles` / `unstageAllFiles` | `git_add_all` / `git_reset_all` | `git.write`，`operation: "stageAll"`；**`git_reset_all` 走的是 `git.command`，参数 `["reset","HEAD"]`（不是 `git.write` 的 `unstage`）** | `api/git-status-api.ts:223-253`；`platform.rs:234-237` 与 `:580-583` |
| 回收改动（丢弃） | `rollbackFilesChanges(repoPath, paths)` | `git.write` | `git.write`，`operation: "discardAll"` + `paths` | `api/git-status-api.ts:322-340` |
| 丢弃单个文件 / 全部 | `discardFileChanges` / `discardAllChanges` | `git_discard_file_changes` / `git_discard_all_changes` | `git.write`，`operation: "discard"` / `"discardAll"` | `api/git-status-api.ts:289-320`；`platform.rs:243-251` |
| 暂存/取消暂存 hunk（差异视图用，不在本视图按钮上） | `stageHunk` / `unstageHunk` | `git_stage_hunk` / `git_unstage_hunk` | `git.apply`（`stage`/`unstage`） | `api/git-status-api.ts:255-287`；`platform.rs:584-594` |
| 加入忽略 | `addPathsToGitignore` / `addPathsToLocalGitExclude` | `git.write` | `git.write`，`operation: "ignore"` / `"exclude"` | `api/git-status-api.ts:342-378` |
| 提交 | `commitSelectedChanges(repo, message, paths)` | `git.write` | `git.write`，`operation: "commit"` + `message` + `paths` | `api/git-commits-api.ts:86-99` |
| 提交（兼容路径） | `commitChanges(repo, message)` | `git_commit` | `git.write`，`operation: "commit"` | `api/git-commits-api.ts:70-84`；`platform.rs:252-255` |
| 历史（首屏 / 分页） | `getGitHistory` / `getGitHistoryPage` | `git_log` / `git_history_page` | `git.history` / `git.historyPage` | `api/git-commits-api.ts:101-190`；`platform.rs:219,221` |
| 引用快照 | `getGitReferences` | `git_references` | `git.references` | `api/git-commits-api.ts:139-158`；`platform.rs:220` |
| 分支列表 | `getBranches` | `git_branches` | `git.history` | `api/git-branches-api.ts:54-55`；`platform.rs:219` |
| 切换/新建/删除分支等写操作 | 各函数 | `git.write` / `git_delete_branch` / `git.checkoutPreflight` | `git.write(op)` / `git.checkoutPreflight` | `api/git-branches-api.ts:79-245` |
| 远程列表 / fetch / pull | `getRemotes` / `fetchChanges` / `pullChanges` | `git_get_remotes` / `git_fetch` / `git_pull` | `git.command` / `git.write(op: "fetch"/"pull")` | `api/git-remotes-api.ts:45-150`；`platform.rs:427-437` |
| 推送 | `pushBranch` 等 | `git.pushPreview` + `git.write` | `git.pushPreview` + `git.write(op:"push")` | `api/git-push-api.ts:60-76` |
| 贮藏列表 / 动作 | `getStashes` / `createStash` / `applyStash` / `popStash` / `dropStash` | `git_get_stashes` / `git_create_stash` / `git_apply_stash` / `git_pop_stash` / `git_drop_stash` | `git.stashes` / `git.write(op:"stashPush"/"stashApply"/"stashPop"/"stashDrop")` | `api/git-stash-api.ts:20-92`；`platform.rs:223,399-417` |
| 操作状态（merge/rebase 进行中） | `getOperationState` | `git_operation_state` | `git.operationState` | `api/git-integration-api.ts:57-58`；`platform.rs:388`；`contracts: rust-core-api.md:204` |
| 冲突标记检查 | `checkConflictMarkers` | `git_conflict_markers` | `git.conflictMarkers` | `api/git-integration-api.ts:63`；`platform.rs:398` |
| 继续/中止/跳过 | `continueOperation` / `abortOperation` / `skipOperationStep` | `git_operation_continue` / `_abort` / `_skip` | `git.write(op:"operationContinue"/"operationAbort"/"operationSkip")` | `platform.rs:389-397` |
| 仓库探测 | `resolveRepositoryPath` / `clearRepositoryDiscoveryCache` | `git_discover_repo` | `git.command`，参数 `["rev-parse","--show-toplevel"]` | `api/git-repo-api.ts:109-127`；`platform.rs:418-422` |
| 工作区多仓库发现 | `discoverWorkspaceRepositoriesFromCore` | `git_discover_workspace_repos` | **`workspace.repositories`**（非 git 命名空间） | `api/git-repo-api.ts:254-264`；`platform.rs:423-426` |
| 仓库 setup（是否仓库 / 是否有提交） | `getGitRepositorySetup` / `initializeGitRepository` | — | `git.repositorySetup` / `git.initialize` | `api/git-setup-api.ts`；`rust-core-api.md:169-170` |
| 差异 | `getWorkingTreeDiff` 等 | `git_diff_file` / `git_status_diff_stats` / `git_commit_diff` / `git_ref_diff` / `git_working_tree_ref_diff` / `git_stash_diff` | `git.diff` | `api/git-diff-api.ts:191-523`；`platform.rs:224-318` |

Core 侧 `git.write` 的完整 `operation` 取值表（**含本视图未用到的分支/远程/工作树/标签操作**）见 `shared/contracts/rust-core-api.md:543-554`。

**三个"命令不缺、但走的不是 git 命名空间 / 契约里没有"的坑**（读 Windows 实现时容易看错）：

| 坑 | 事实 | 证据 |
| --- | --- | --- |
| `git_reset_all`（取消暂存全部） | 翻译成 **`git.command` + `arguments:["reset","HEAD"]`**，不是 `git.write(op:"unstage")`；gpui 侧应改用 `git.write` 的 `unstage` + 路径列表（语义等价且更安全） | `platform.rs:580-583` |
| `git_discover_workspace_repos` | 翻译成 `workspace.repositories`（`rust/lithe-core` 的 project 命令，**不是** `git.*`） | `platform.rs:423-426` |
| `git.patchPreview` / `git.patchApply` | Windows 包装**在调**（`api/git-patch-api.ts:38,54`），但 `shared/contracts/rust-core-api.md` 里**只文档化了 `git.patchExport`**（`:1800`）；另两个 **未找到** | `api/git-patch-api.ts:12-58`；`rust-core-api.md:1800` |
| **删除工作区文件** | **Core 没有"删除文件"命令**（`git.write` 白名单里没有 `deleteFile`/`remove`）。Windows 的文件删除走**原生 Tauri 命令** `move_file`/`rename_file` 之外的平台删除（`useFileSystemStore.deleteFile`），Git 侧只做后续刷新 | `git-status-panel.tsx:557-602`（调 `deleteFile`）→ `features/file-system/stores/file-system.store.ts`；`rust-core-api.md:543-554`（无删除项） |

### 4.3 刷新时机（三条通路）

| 通路 | 机制 | 证据 |
| --- | --- | --- |
| ① 初始加载 | `useGitDataController` 在工作区 ready 时：`getWorkspaceGitStatus` 先落状态 → 再并发 `getGitHistory` / `getBranches` / `getStashes` / `getOperationState`；带 `requestId` 与 `activeRepoPath` 双重陈旧保护 | `hooks/use-git-data-controller.ts:57-144,263-274` |
| ② **文件监听（外部改动）** | `GitMetadataWatchHost` 调原生命令 `watch_git_repository`，监听 Tauri 事件 **`git-metadata-changed`**；命中后 `emitGitChanged({scopes:["working-tree","history","refs","stashes","repository"], source:"external-git-change"})` | `runtime/git-metadata-watch-host.tsx:19-59`；`platform/tauri-core.ts:80-81` |
| ③ 前端自触发 | 每个写操作完成后 `emitGitChanged(...)`（`stage-file`/`unstage-file`/`stage-files`/`unstage-files`/`stage-all`/`unstage-all`/`discard-all`/`rollback-files`/`commit`/`add-to-gitignore`/`add-to-git-external…`），走 `window` 自定义事件 `lithe:git-changed`（**不是** Tauri 事件） | `events/git-events.ts:3-45`；`api/git-status-api.ts:168-173,185-190,214-218,227-231,243-247,293-297,335-339,357-361` |
| 订阅与防抖 | `subscribeToGitChanges` → **100 ms** 定时器合并 scopes → `refreshGitData(scopes, false, "background")`；并发读由 `createGitRefreshQueue` 按 key 去重合并 | `hooks/use-git-data-controller.ts:283-315`；`services/git-operation-coordinator.ts:14`（暂存写队列）、`:52`（刷新队列） |
| 被动改动的策略开关 | 设置 `autoRefreshGitStatus`（默认 true）；为假时**忽略**被动来源（`save`/`auto-save`/`external-file-change`/`external-git-change`） | `default-settings.ts:195`；`events/git-events.ts:20,43-45`；`use-git-data-controller.ts:290` |
| 重新激活视图时刷新 | `autoRefreshGitStatus && isActive && !wasActive && gitStatus` → 后台整刷一次 | `use-git-data-controller.ts:276-281` |
| 手动刷新 | 标题栏按钮 / 错误条"重试" / 「更多」菜单"刷新状态" → `refresh()`（会清仓库发现缓存，并 `refreshWorkspaceRepositories`） | `git-view.tsx:92-101,480-493`；`use-git-data-controller.ts:245-254` |
| 仅工作区刷新（暂存后） | `refreshWorkingTree()` → `refreshGitData(["working-tree"], true)`；会"吃掉"一次同范围的待处理事件刷新以省一次读 | `use-git-data-controller.ts:230-243`；`git-view.tsx:747` |

### 4.4 状态存放位置（本地 store，不是 Core）

| 状态 | 存放 | 证据 |
| --- | --- | --- |
| `gitStatus` / `commits` / `branches` / `stashes` / `operationState` / `isLoadingGitData` / `isRefreshing` | `useGitStore`（工作区作用域 store） | `stores/git.store.ts`；`git-view.tsx:84-90`；`use-git-data-controller.ts:41-43`；`stores/git.store.ts:271`（`createWorkspaceScopedStore("git", …)`） |
| 活动仓库 / 可用仓库列表 | `useRepositoryStore`（`activeRepoPath` / `availableRepoPaths`） | `use-git-data-controller.ts:37-40` |
| 提交勾选、草稿提交说明、折叠的文件夹/分类 | `useGitStore.sourceControlSessions[repoPath]`（按仓库持久于 store 内） | `git-view.tsx:102-104,143-167,730-733` |
| 选中行 / 差异下拉 / 贮藏弹窗 / 加载态等纯 UI | 组件内 `useState` | `git-status-panel.tsx:234-250`；`git-view.tsx:123-174` |

---

## 5. 空态与失败态

`GitView` 的 5 个返回分支（`git-view.tsx:635-699` + 正常态 `:704`）：

| 条件 | 显示 | 文案 / 键 | 证据 |
| --- | --- | --- | --- |
| `!activeRepoPath && repoPath`（有工作区，但未解析出仓库） | 标题栏 + `GitRepositoryEmptyState context="changes"` | 主文案由 setup 决定（见下） | `git-view.tsx:635-644` |
| `!activeRepoPath && !repoPath`（无工作区） | 标题栏 + `Empty` | 标题 **未选择仓库** — `git.noRepositorySelected`；内容区两个按钮：浏览/初始化 | `git-view.tsx:645-656`；`locale.ts:7010` |
| `isLoadingGitData && !gitStatus` | 标题栏 + `Spinner` | **正在加载 Git 状态** — `git.loadingGitStatus` | `git-view.tsx:663-675`；`locale.ts:7013` |
| `!gitStatus`（加载完但无快照） | 标题栏 + `GitRepositoryEmptyState` | 同第一行 | `git-view.tsx:687-699` |
| 正常 | §2 结构 | — | `git-view.tsx:704-999` |

**`GitRepositoryEmptyState`（`context="changes"`）内部文案**（`git-repository-empty-state.tsx:94-155`）：

| 条件 | 文案 / 键 | 证据 |
| --- | --- | --- |
| `busy` | **正在检查 Git 仓库…** — `git.setup.loading` | `:96-97`；`locale.ts:4589` |
| 不是仓库 | 标题 **此项目尚未初始化 Git 仓库** — `git.setup.notRepository`；说明 **初始化 Git 后即可跟踪更改并创建首次提交。** — `git.setup.initializeHint`；按钮 `git.setup.initialize` | `:102-118`；`locale.ts:4595,4596,4597` |
| 是仓库但无提交 | 标题 **当前分支尚无提交** — `git.setup.noCommits`；分支名；说明 **在"更改"面板暂存文件、填写提交消息，然后创建首次提交。** — `git.setup.firstCommit`；按钮 **打开更改面板** `git.setup.openChanges` | `:102-127`；`locale.ts:4600,4601,4603` |
| 是仓库且有提交、但 `context="changes"` 走到这里（即 status 加载失败） | 标题 **无法加载 Git 数据，请重试刷新仓库。** — `git.statusLoadFailed` | `:102-110`；`locale.ts:7011` |
| 有错误 / 历史错误 | `GitRepositoryErrorNotice`（组件 `git-repository-error-notice.tsx`） | `:140-142` |
| 恒有 | 底部 **重试** — `git.log.retry` | `:143-154`；`locale.ts:7197` |

**命令失败时的表现**：

| 场景 | 表现 | 证据 |
| --- | --- | --- |
| `git.status` 系列失败 | 顶部错误条 + 重试（§2 ②） | `git-view.tsx:677-685` |
| 暂存/取消暂存失败 | 逐仓库 `Promise.allSettled`，失败的仓库 toast **错误：{error}** (`git.operationError`)；UI 用乐观 stage map 回滚；只上报固定分类（`index_lock`/`permission_denied`/`timeout`/`operation_failed`）到前端日志，**不带 Git 原始 stderr**（注释原文：stderr 可能含绝对路径、筛选输出或凭据） | `git-status-panel.tsx:452-505,174-204`；`locale.ts:7001` |
| 提交失败 | 面板内错误块（红底）显示错误原文 | `git-commit-panel.tsx:203-213,264-274` |
| 删除文件部分失败 | toast **有 {count} 个选中文件删除失败。** — `git.deleteFilesFailed`；刷新失败再 toast **文件已删除，但源代码管理刷新失败。** — `git.refreshAfterDeleteFailed` | `git-status-panel.tsx:585-597`；`locale.ts:7166,7167` |
| 回滚失败 | toast `git.operationError` | `git-status-panel.tsx:549-554` |
| 贮藏跨仓库 | toast **请选择同一个仓库中的更改进行贮藏。** — `git.selectSingleRepositoryForStash` | `git-status-panel.tsx:704-707,675-678`；`locale.ts:7152` |
| 操作横幅动作失败 | toast 错误消息 或 **Git 操作失败** — `git.operationFailed` | `git-operation-banner.tsx:39-43`；`locale.ts:7033` |
| 「更多」菜单动作失败 | toast `messages.error`（如 `git.fetchFailed`）或 `${actionName} failed.`；成功后 `onRefresh()` | `git-actions-menu.tsx:76-119` |
| 无仓库时用写操作 | toast **未打开仓库** — `git.noRepositoryOpen`（拉取路径） | `git-view.tsx:113-117`；`locale.ts:6984` |
| 选到的目录不是仓库 | 空态下 `EmptyDescription`（destructive 色）+ `showAlertDialog`，文案 **所选文件夹不是 Git 仓库**（键 `git.selectedFolderNotRepo`，zh 值未核） | `git-view.tsx:276-282,648-652` |

---

## 6. gpui 侧落地评估

现状（本仓库）：`gpui/crates/git/`（`model.rs` + `log_view.rs`，只做底部提交记录）
与 `gpui/crates/explorer/`（侧栏树）；活动栏项与文案已经就位
（`gpui/crates/workbench/src/activity_bar.rs:39`；`gpui/crates/workbench/src/workspace.rs:1149-1158`
已注册 `IconName::GitBranch` + `tr("lithe.workbench.changes")` 的「更改」项）。

### 6.1 现在就能做（Core 有对应命令 + 契约稳定）

| 能力 | 需要的 Core 命令 | 现状证据 |
| --- | --- | --- |
| 读工作区变更 + 分支 + ahead/behind | `git.status` | 契约 `rust-core-api.md:172`；响应 `contracts.rs:532-538`；**gpui 已实现**（`gpui/crates/git/src/model.rs:493-500` `read_status`） |
| 仓库探测（工作区根不是仓库时逐子目录探） | `git.status` + `workspace.snapshot` | 已有兜底实现 `gpui/crates/git/src/model.rs:502-534` |
| 非仓库 / 未初始化识别 | `git.repositorySetup`（是否仓库、是否有提交、分支） | `rust-core-api.md:169`；Windows 侧同源 `git-repository-empty-state.tsx:50` |
| 初始化仓库 | `git.initialize` | `rust-core-api.md:170` |
| **暂存 / 取消暂存（文件集合）** | `git.write` `operation: stage \| unstage` + `paths` | `rust-core-api.md:543-554`（`stage`/`unstage` 在必填表内） |
| **暂存全部 / 取消暂存全部** | `git.write` `operation: stageAll`；取消暂存全部 = `unstage` + 全部路径（Core 的 `discardAll` 是丢弃，**不可**拿来当取消暂存） | `rust-core-api.md:543-554`；Windows 的 `git_reset_all` 走的是 `git.command ["reset","HEAD"]`（`platform.rs:580-583`），gpui 侧**不要照搬**，改用 `unstage` + 路径列表 |
| **提交（指定路径 + 消息）** | `git.write` `operation: commit` + `message` + `paths` | `rust-core-api.md:543-554`；Windows `api/git-commits-api.ts:94-98` |
| **丢弃 / 回滚单个或选中的改动** | `git.write` `operation: discard` + `paths`；全部丢弃 `discardAll` | `rust-core-api.md:543-554`；`platform.rs:243-251` |
| **加入 .gitignore / .git/info/exclude** | `git.write` `operation: ignore \| exclude` | `rust-core-api.md:543-554` |
| **操作状态横幅（merge/rebase/cherry-pick/revert + 冲突路径）** | `git.operationState` | `rust-core-api.md:204`；响应字段说明 `rust-core-api.md:840-845` |
| **继续 / 中止 / 跳过** | `git.write` `operation: operationContinue \| operationAbort \| operationSkip` | `rust-core-api.md:543-554`；约束（有冲突时拒绝 continue、无状态时 `invalid_request`）`rust-core-api.md:807-812` |
| 冲突标记检查 | `git.conflictMarkers` | `rust-core-api.md:203` |
| **差异（工作树 / 索引 / 引用 / 提交）** | `git.diff` | `rust-core-api.md:189` |
| 提交历史（左栏 `history` 页签 / 提交差异选择器） | `git.references` + `git.historyPage` + `git.commitFiles`(+`git.historyCursorClose`) | 契约 `:191-197`；**gpui 已实现**（`gpui/crates/git/src/model.rs:707-841`） |
| 贮藏列表 / 应用 / 弹出 / 删除 / 新建 | `git.stashes` + `git.write` `stashPush/stashApply/stashPop/stashDrop` | `rust-core-api.md:199,543-554` |
| 分支列表 | `git.references`（本地分支 + 当前分支 + ahead/behind） | `rust-core-api.md:192` |
| 切换分支前预检 | `git.checkoutPreflight` | `rust-core-api.md:200` |
| 拉取前预检（upstream / 分叉 / 本地改动） | `git.pullPreflight` | `rust-core-api.md:201` |
| 合并/变基/拣选/还原前预检 | `git.integrationPreflight` | `rust-core-api.md:202` |

> 注：`git.write` 的**必填 `operation` 白名单**里已经有 `createBranch / publishBranch / renameBranch / setUpstream / unsetUpstream / deleteBranch / updateBranch / merge / rebase / checkout / checkoutAndRebase / fetch / pull / push / clone / createTag / deleteTag / createWorktree / …`
> （`rust-core-api.md:543-550`），所以**分支与同步的写命令并不缺**；缺的是 gpui 侧的 UI 与状态机。

**已有 gpui 资产的确切边界**（评估复用前先看这段）：

| crate | 文件 | 现状 |
| --- | --- | --- |
| `gpui/crates/git` | `Cargo.toml`、`src/lib.rs`(165)、`src/model.rs`(1148)、`src/log_view.rs`(≈2018) | **只做底部提交记录**。实际调用的 `git.*` 命令只有 **5 条**：`git.status`（`model.rs:494,526`）、`git.references`（`:770-773`）、`git.historyPage`（`:712-721`）、`git.historyCursorClose`（`:397`）、`git.commitFiles`（`:816-819`）；另有非 git 的 `workspace.snapshot`（`:509-512`）用于仓库探测。**`log_view.rs` 内没有任何 `git.*` 调用**（渲染层零命令）。文档里写的"6 条 `git.*` 命令"（`lib.rs:9`、`model.rs:1`）与实际 5 条不一致——第 6 条数据调用是 `workspace.snapshot`。信封与超时走 `gpui/crates/shared/src/core_client.rs`（`model.rs:454-460`；超时 60 s 见 `lib.rs:155-157`） |
| `gpui/crates/explorer` | `Cargo.toml`、`src/lib.rs`(114)、`src/model.rs`(305)、`src/explorer_view.rs`(835) | 侧栏树。**只调 `workspace.snapshot`**（`model.rs:236`），**没有任何 `git.*` 调用**；`lib.rs:79-81` 明确把"Git 状态装饰"登记为未实现。树行渲染入口：`explorer_view.rs:562`（`render_tree`）、`:570`（`Tree::new` 的逐行闭包）、`:659`（`ListItem::new`），行内动作槽/复选框**当前未实现**（`:622-641` 的 `h_flex` 行只有 caret + 图标 + 文本） |
| `gpui/crates/workbench` | `activity_bar.rs`、`workspace.rs` | 活动栏「更改」项已注册（`workspace.rs:1151`，`IconName::GitBranch` + `tr("lithe.workbench.changes")`）；`workspace.rs:156-157` 目前把 `BottomPaneKind::Git` 用作**底部** git 窗（提交记录），左栏的「更改」视图尚未接线 |

### 6.2 缺命令 / 需要新加 Core 命令或另走路径

| 缺口 | 说明 | 建议路径 |
| --- | --- | --- |
| **原生 Git 元数据文件监听** | Windows 走 Tauri 原生命令 `watch_git_repository` / `unwatch_git_repository` + Tauri 事件 `git-metadata-changed`（`runtime/git-metadata-watch-host.tsx:25,29,55`），这不是 Core 命令，Core 只提供 `git.watchContext` 给出需要观察的元数据目录（`rust-core-api.md:173`） | 这是**平台适配层**职责：gpui 宿主需自己起文件监听（`gpui/crates/explorer` 已有 watcher 经验），把 `git.watchContext` 的结果作为观察根，事件转成本地"刷新请求" |
| **差异的富渲染（并排 / 语法高亮 / 图片 / 二进制 / hunk 级暂存）** | Windows 用 Monaco + 自绘 diff（`features/git/components/diff/*`，15 个文件） | 契约侧 `git.diff` + `git.apply` 都在（`rust-core-api.md:189-190`）；**缺的是 gpui 侧的 diff 视图子系统**（`gpui/crates/editor` 只有编辑器，没有 diff） |
| **补丁创建 / 预览 / 应用对话框** | Windows 走 `showGitPatchDialog` + `api/git-patch-api.ts` 的 `git.patchExport` / `git.patchPreview` / `git.patchApply`；补丁落盘是平台侧（`windows/tauri/src-tauri/src/host.rs:362`，写 `changes.patch`） | 契约**只文档化了 `git.patchExport`**（`rust-core-api.md:1800`），`patchPreview`/`patchApply` 在 `rust-core-api.md` 中**未找到**（`api/git-patch-api.ts:38,54` 仍在调）。第一版不做；要做得先确认这两个命令的契约状态 |
| **删除工作区文件** | Windows 的文件级"删除"是**原生文件系统删除**（`useFileSystemStore.deleteFile`），Core 无对应命令；Git 侧的 `discard` 只还原内容、不删文件 | `git-status-panel.tsx:557-602`；`rust-core-api.md:543-554`（无 `deleteFile`）。gpui 侧要自己走文件系统删除 + 之后刷新 |
| **推送对话框 / 认证交互** | Windows: `git.pushPreview` + 认证挑战 `git.authRespond` + `GitAuthenticationDialog` | 契约有 `git.pushPreview`（`:195`）与 `git.authRespond`（`:183`）；**缺 gpui 侧凭据 UI 与 `git.executionInspect`/`executionConfigure` 接线** |
| **AI 生成提交说明** | Windows 在 `git-commit-panel.tsx` 里经 `services/ai-commit-service` 直接发 HTTP（`ai_commit.rs:119-225`） | 与 Git 契约无关；gpui 侧要独立评估（不在本文范围） |
| **新增文件监听事件去抖语义** | Windows 的 100 ms 合并 + 作用域合并 + 被动源过滤（`use-git-data-controller.ts:283-315`）是产品语义，Core 不表达 | gpui 侧需在 git crate 内实现等价的刷新协调器（`createGitRefreshQueue` 的语义在 Windows 是前端实现，`services/git-operation-coordinator.ts`） |

### 6.3 缺子系统

| 子系统 | 说明 | 证据 |
| --- | --- | --- |
| **分支管理面板** | Windows 是 `GitBranchManager`（一个带页签的多功能模态：分支 / 工作树 / 仓库），从标题栏、左栏「更多」、命令面板多处触达 | `features/git/components/git-branch-manager.tsx`；`footer-git-branch-item.tsx:32-67`；`git-actions-menu.tsx:214-225` |
| **远程管理 / 标签管理 / 工作树对话框 / 贮藏模态 / 交互式变基 / 历史重写对话框** | 各自独立模态 | `git-remote-manager.tsx`、`git-tag-manager.tsx`、`git-worktree-dialog.tsx`、`stash/git-stash-modal.tsx`、`git-rebase-dialog.tsx`、`git-history-rewrite-dialog.tsx` |
| **冲突解决体验** | 「变更列表里解决冲突」= 在编辑器改文件 → 暂存 → 横幅"继续"。没有三方合并编辑器 | `git-operation-banner.tsx:21-25,86-104`；`git.rebasePlan.conflicts`（`locale.ts:6851`）明确指向"在更改中解决" |
| **多仓库聚合视图** | 工作区含多个仓库时，`getWorkspaceGitStatus` 把各仓库文件加前缀聚合，并在每行保留 `repositoryPath` / `repositoryRelativePath` | `api/git-status-api.ts:106-162`；`types/git.types.ts:1-16` |
| **拖拽互操作** | 变更行可拖到编辑器/其它面（`writeSidebarResourceDragData`，`type:"git-file-diff"`） | `git-status-file-item.tsx:168-179`；`features/sidebar/utils/sidebar-resource-drag.ts` |
| **设置面（Git 页签）** | 19 个 git 相关设置项（默认值见 §4.3 / `default-settings.ts:190-212`），Windows 有独立设置页 | `default-settings.ts:190-212`；`features/settings/components/tabs/*` |
| **状态栏/标题栏的 Git 分支项** | 分支 + ahead/behind 的常驻显示在**标题栏**，不在左栏 | `footer-git-branch-item.tsx`；`title-bar.tsx:66,238` |

### 6.4 建议的第一版范围

**先做"只读 + 单仓库 + 文件级写操作"，不做分支/远程/冲突/差异富渲染。**

第一版（建议）：

1. **视图接线**：活动栏 `git` 项 → 左栏 pane 切换到「源代码管理」视图（活动栏项已在 `workspace.rs:1151`，只缺 pane 切换与内容）。
2. **顶部标题栏**：标题「源代码管理」+ 刷新按钮 + 「更多」菜单（**只放本版可用的项**：选择仓库、刷新状态、初始化仓库、丢弃全部更改）。
3. **状态区**：`git.status` 读回 → 顶部三件套：分支名（只读文本）、ahead/behind 计数、`git.operationState` 有值时渲染操作横幅（继续/中止/跳过）。
4. **变更列表**：两个分类（已跟踪 32px 分类头 + 未跟踪）+ 文件行（复用 `explorer` 的树行 + `file-tree-row` 行高公式 `max(24, 13×1.35+6)`），行内 stage/unstage 按钮 + 复选框，工具行「查看差异」+「暂存所有更改」+「取消暂存所有更改」。
5. **提交面板**：底栏 `Textarea`（min 64 / max 128 自动增高）+ 计数 + `Ctrl+Enter` + 「提交」（`git.write op:commit`）。暂不做「提交并推送」、暂不做 AI。
6. **写操作**：stage / unstage / stageAll / discard(单文件，带确认) / commit / ignore（右键菜单里放这 6 个）。
7. **右键菜单**：只放 暂存 / 取消暂存 / 提交 / 回滚 / 显示差异 / 跳转到源 / 添加到 VCS / 添加到 .gitignore / 添加到 .git/info/exclude / 删除 / 贮藏 这 11 项中**命令已具备的**子集（去掉补丁类）。
8. **刷新**：手动刷新 + 写操作后本地刷新（先不做文件监听，用"激活视图时刷新 + 手动刷新"兜底，并在文档里写明这是**刻意偏差**）。
9. **空态**：无仓库 / 未初始化 / 无提交 / 加载中 / 加载失败 五态（§5 的文案逐字落地）。

**理由**（逐条对应本仓库已有的约束）：

- **越早锁定"能提交"这条最小闭环越有价值**：Core 的 `git.write(stage/unstage/stageAll/discard/commit)` 已全部齐备
  （`rust-core-api.md:543-554`），不需要新增任何 Core 命令，风险集中在 UI 与刷新协调。
- **分支 / 远程 / 冲突是三个独立子系统**（§6.3），各自需要一个模态 + 一个状态机；与"能提交"耦合度低，
  先做会把第一版摊薄。而且分支切在 Windows 里**主要入口不在左栏**（在标题栏 `GitBranchManager`），
  放第一版会迫使同时改标题栏。
- **差异富渲染（Monaco 级）成本最高**：Windows 侧是 15 个文件的 diff 子系统 + Monaco；
  gpui 侧 `editor` crate 尚无 diff 视图。第一版先用「点击行 → 用最简文本差异视图或直接打开原文件」
  兜底（Windows 本身也有这个开关：`openDiffOnClick`，默认 true，`default-settings.ts:199`）。
- **文件监听可以稍后补**：Windows 的自动刷新依赖平台 watcher（Tauri 原生命令），
  gpui 宿主需要先有 watcher 基础设施；用"激活时刷新 + 手动刷新 + 写后刷新"能保证第一版**正确**（只是不够实时）。
- **`operationState` 横幅要放第一版**：因为**提交会被进行中操作阻塞**（`git-commit-panel.tsx:163-172`
  的两条守卫），没有横幅用户会看到"提交按钮点了没反应"。

---

## 7. 未确认的点

1. **`git.actions` 这个 i18n 键**（`git-view.tsx:474` 用作「更多」按钮 tooltip）在本次核对中没有在 `locale.ts` 里定位到；
   「更多」按钮的 tooltip 中文文案 **未确认**。
2. **若干次级键的 zh 原文未逐字核**：`git.refreshAria`、`aiCommit.settings`、`aiCommit.cancel`、`aiCommit.configure`、
   `aiCommit.replace`、`ai.unknownError`、`git.searchCommits`/`git.commitCount`/`git.commitsCount`/`git.noCommits`/`git.noMatchingCommits`、
   `git.compareBranchPlaceholder`/`git.branchCount`/`git.branchesCount`/`git.compareWithBranch`/`git.noOtherBranches`/`git.noMatchingBranches`、
   `git.searchStashes`/`git.stashCount`/`git.stashesCount`/`git.noStashes`/`git.noMatchingStashes`、
   `git.selectedFolderNotRepo`、`git.failedToSelectRepository`、`git.openFolderBeforeInit`、`git.repositoryInitialized`、`git.initializeGitRepository`、
   `git.openFolderBeforeInitializing`、`git.refresh`、`git.browse`、`git.initialize`、`git.historySearch`、`git.historyFilter`、
   `git.localCommitsNotPushed`、`git.remoteCommitsNotPulled`、`git.historyNoCommits`、`git.historyNoMatch`、`git.historyEnd`、`git.log.loadingCommits`、
   `git.stashMessageDefaultSelection`、`git.stashMessageDefaultAll`。
   已核实的键见 §2/§3/§5 各表；上列键的**中文逐字原文**待补。
3. **`GitRepositoryErrorNotice` 的内部结构与文案**（`git-repository-error-notice.tsx`，仅 `:140-142` 被本视图使用）未读。
4. **`buildVisibleGitFiles` / `buildGitStatusPresentation` / `GIT_STATUS_ORDER` 的确切排序规则**
   （`utils/git-status-model.ts`）未逐行读；`showStagedFirst` 的具体生效位置未确认。
5. **`openDiffOnClick` 为假时单击行的确切行为**（`handleOpenOriginalFile` 的落点）未逐行确认。
6. **`git.write` 的 `commit` 操作在 Core 侧对 `paths` 为空时的语义**（是否等价于 `-a`）未确认。
7. **`GitMetadataWatchHost` 挂载在哪个层级**（是否受 `autoRefreshGitStatus` 影响、是否按工作区隔离）未确认——
   只确认了它自身读 `activeRepoPath ?? projectPath` 并随 `workspaceId` 重建（`git-metadata-watch-host.tsx:15-19,59`）。
8. **`git.viewDiff` 主按钮在 `openDiffOnClick=false` 时的落点**（打开差异视图 vs 多文件差异栈）未确认。
9. **左侧栏「更改」视图是否也在"边缘视图"通路里**：`EDGE_SIDEBAR_VIEWS = {outline, databases, notifications}` 不含 `git`
   （`sidebar-pane-utils.ts:26-30`），所以 `git` 是 `primary` 级；但**右栏**是否也存在一个 git 视图未确认。
10. **`gpui/crates/explorer` 的树行原语是否已支持行内动作槽 + 复选框**：本次只确认了文件结构与入口
    （`explorer/src/model.rs:124,160`；`explorer_view.rs:562,570,659`），
    以及现有行（`explorer_view.rs:622-641`）只有 caret + 图标 + 文本、**没有动作槽/复选框**——
    第一版要加行内 stage 按钮必须先扩这个行组件，具体改法未定。
11. **`git.patchPreview` / `git.patchApply` 的契约状态**：Windows 包装在调（`api/git-patch-api.ts:38,54`），
    但 `shared/contracts/rust-core-api.md` 里只文档化了 `git.patchExport`（`:1800`）；
    这两个命令**是否真的存在于 Core**（还是只存在于文档之外）未确认——只能确认"契约未文档化"。
12. **`git_reset_all` 的路径语义**：Windows 取消暂存全部走 `git.command ["reset","HEAD"]`（`platform.rs:580-583`），
    它是**全量**重置索引；gpui 侧若改用 `git.write(op:"unstage") + paths`，在"部分路径被稀疏检出/忽略"等场景下
    是否与 `reset HEAD` 完全等价未确认。
13. **工作区多仓库时的 `git.status` 聚合方式**在 gpui 侧的做法未定：Windows 在前端逐仓库调 `git_status` 再拼前缀
    （`api/git-status-api.ts:117-162`），而 Core 也有 `workspace.repositories`（`platform.rs:423-426`）。
    第一版建议**先只支持单仓库**（已在 §6.4 写明），多仓库聚合留到后续评估。
