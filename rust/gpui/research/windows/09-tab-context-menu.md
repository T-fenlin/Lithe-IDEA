# 09 · 标签右键菜单（Windows 真源 → gpui 接线研究）

研究范围：Windows 前端 `windows/tauri/src/`（只读真源）里**标签右键菜单**的完整规格，以及它在
`gpui/` 侧（`gpui/crates/editor/src/editor_view.rs`）的可做性与接线方式。

- 分支：`feat/gpui-shell-rewrite`
- 本文**不改任何代码**，所有 gpui 侧结论都给 `文件:行号` 证据。
- 已有实现（不重复）：`gpui/research/windows/02-editor-sidebar.md:422` 已登记过菜单项顺序，
  `gpui/UI-MAP-WINDOWS.md:852-857` 已登记过对应中文键。本文是那两条的**展开与可行性判定**。

---

## 0. 一句话结论

| 问题 | 结论 |
| --- | --- |
| 菜单在哪 | `windows/tauri/src/features/tabs/components/tab-context-menu.tsx`（整体 221 行，菜单数组 `:70-202`），由 `tab-bar.tsx:695-755` 逐标签实例化 |
| 真机可见项数 | **13 项 + 4 条分隔线**（文件标签）。第 1 项 `pin` 是**条件文案**（固定/取消固定） |
| 我们现阶段能做 | **10 项**（固定标签页 / 复制路径 / 复制相对路径 / 在资源管理器中显示 / 重新加载 / 关闭 / 关闭其他 / 关闭右侧 / 全部关闭 + `Ctrl+W` 已有） |
| 依赖我们没有的能力 | **3 项**：向右拆分 / 向下拆分 / 锁定编辑器组，**根因都指向同一个缺口 —— gpui 外壳没有 pane 树（分屏）** |
| 与未保存确认的关系 | 真机是「**逐个发现、批量确认**」：先扫出**第一个**脏标签弹一次确认，确认后**其余全部无条件关闭**（不再逐个问）。我们阶段的 `request_close` 是**逐个**确认，直接复用会让用户被弹 N 次 |
| 文案键 | 15 个键（含条件项的 2 个同胞键）**真源里全部存在**，且 `gpui/crates/shared/locales/lithe.zh-CN.yml` 已经导出，**零新增 i18n 键** |
| 没能确认的点 | 见 §6（共 6 条） |

---

## 1. 真源：菜单在哪、逐项规格

### 1.1 定义位置

| 角色 | 位置 |
| --- | --- |
| 菜单本体（数组 + 渲染） | `windows/tauri/src/features/tabs/components/tab-context-menu.tsx:50-219` |
| 菜单项数组 | 同文件 `:70-202` |
| 渲染（分隔线 / 图标 / 标签 / 快捷键） | 同文件 `:204-218` |
| 谁调用它（逐标签实例化） | `windows/tauri/src/features/tabs/components/tab-bar.tsx:675-756`（`ContextMenu > ContextMenuTrigger > TabBarItem` + `<TabContextMenu ... />`，`:695`） |
| 触发元素（右键目标） | `windows/tauri/src/features/tabs/components/tab-bar-item.tsx:147` 的 `onContextMenu`，落到 `ui/tab-bar.tsx:269` 的 `TabBarTab` |
| 上级结构（已经在 `02-editor-sidebar.md:67` 登记） | `pane-container.tsx:319` → `TabBar`（`tabs/components/tab-bar.tsx:61`） |

**不是** `features/panes/components/` 里的文件 —— 那目录下的 `pane-container.tsx` / `pane-chrome.tsx` /
`pane-node-renderer.tsx` 只提供 `paneId` / `isPaneLocked` 等上下文（`tab-bar.tsx:132,134,697,707`），
菜单本体在 `features/tabs/components/`。

### 1.2 逐项规格（顺序 / 中文文案 / i18n 键 / 分隔线 / 出现与禁用条件）

真机 zh-CN 文案的键值都在 `windows/tauri/src/i18n/locale.ts`（下称 `locale.ts`），
行号见 §4。下表「条件」列区分**整项不出现**（数组里被 spread 掉）与**出现但禁用**。

| # | id（`:行`） | 中文文案 | i18n 键 | 图标（lucide） | 出现条件 | 禁用条件 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `pin`（`:71-76`） | 固定标签页 / **取消固定标签页** | `tabs.pin` / `tabs.unpin`（按 `buffer.isPinned` 二选一，`:73`） | `Pin` / **`PinOff`**（`:74`） | 恒出现 | ❌ 真机**永不置灰**（数组里没有 `disabled`；渲染只透传 `item.disabled`，`:210`） |
| 2 | `rename-terminal`（`:77-86`） | 重命名 | `files.rename` | `PencilSimpleLine` | **仅 `buffer.type === "terminal"`**（`:77`）。截图那 13 项是文件标签，故无此项 | ❌ 不置灰，但 `onRename?.()` 是可选调用（`:83`） |
| — | `sep-1`（`:87`） | — | — | — | **恒出现** | — |
| 3 | `split-right`（`:88-97`） | 向右拆分 | `tabs.splitRight` | `Columns2` | **`paneId && onSplitRight`**（`:88`）。`onSplitRight` 只有 `paneId` 存在时才传（`tab-bar.tsx:741-747`） | ❌ 不置灰 |
| 4 | `split-down`（`:98-107`） | 向下拆分 | `tabs.splitDown` | `Rows2` | **`paneId && onSplitDown`**（`:98`） | ❌ 不置灰 |
| — | `sep-2`（`:108-110`） | — | — | — | **`paneId && (onSplitRight \|\| onSplitDown)`** —— 即"出现了任一拆分项"才画 | — |
| 5 | `toggle-editor-group-lock`（`:111-121`） | 锁定编辑器组 / **解锁编辑器组** | `tabs.lockEditorGroup` / `tabs.unlockEditorGroup`（按 `isPaneLocked` 二选一，`:115`） | `Lock` / **`LockOpen`**（`:116`） | **`onTogglePaneLocked` 存在**（`:111`）。调用方条件是 `paneId && !disablePaneActions && !isBottomPane`（`tab-bar.tsx:708-712`） | ❌ 不置灰 |
| — | `sep-lock`（`:119`） | — | — | — | **紧跟锁定项、与它成对**（`onTogglePaneLocked` 存在才出现） | — |
| 6 | `copy-path`（`:122-138`） | 复制路径 | `files.copyPath` | `Copy` | 恒出现 | ❌ 不置灰（虚拟路径也照拷原字符串，`:128`） |
| 7 | `copy-relative-path`（`:139-144`） | 复制相对路径 | `files.copyRelativePath` | `Copy` | 恒出现 | ❌ 不置灰；无工作区根时**拷全路径**（`tab-bar.tsx:359-363`） |
| 8 | `reveal`（`:145-150`） | 在资源管理器中显示 | `files.reveal` | `FolderOpen` | 恒出现 | ❌ 不置灰 |
| 9 | `terminal`（`:151-168`） | 在终端中打开 | `files.openInTerminal` | `Terminal` | **`!isVirtualContent(buffer) && !buffer.path.includes("://")`**（`:151`） | ❌ 不置灰 |
| 10 | `reload`（`:169-178`） | 重新加载 | `tabs.reload` | `RotateCcw` | **`buffer.path !== "extensions://marketplace"`**（`:169`） | ❌ 不置灰 |
| — | `sep-3`（`:179`） | — | — | — | **恒出现** | — |
| 11 | `close`（`:180-186`） | 关闭 | `tabs.close` | `X` | 恒出现 | ❌ 不置灰。**右侧带快捷键提示** `Ctrl+W` / `Cmd+W`（`closeKeys`，`:69`；`IS_MAC` 来自 `@/utils/platform`，`:29`） |
| 12 | `close-others`（`:187-191`） | 关闭其他 | `tabs.closeOthers` | 无 | 恒出现 | ❌ 不置灰 |
| 13 | `close-right`（`:192-196`） | 关闭右侧 | `tabs.closeToRight` | 无 | 恒出现 | ❌ 不置灰 |
| 14 | `close-all`（`:197-201`） | 全部关闭 | `tabs.closeAll` | 无 | 恒出现 | ❌ 不置灰 |

**合计**：文件标签恒见 **13 项 + 4 分隔线**（`sep-1` / `sep-2` / `sep-3`，其中 `sep-2` 依赖 `paneId`）；
终端标签多出 `rename-terminal`；锁定项在底部工具窗 / `disablePaneActions` 下整项消失。
编号 1-14 里"文件标签可见"的是 1、3、4、5、6、7、8、9、10、11、12、13、14 —— **正好 13 项**。

> ⚠️ **重要发现（与截图的一处潜在冲突）**：菜单里**没有任何一项被置灰**。
> 真机唯一的"禁用"是 `empty-editor-state.tsx:33` 的空态菜单（`ui.noActionsHere` 禁用项），
> 与标签菜单无关。所以"置灰"这个手段**在真源里没有先例**，§2 的替代方案讨论要按这条来。

### 1.3 三组状态来源

| 状态 | 真源 | 说明 |
| --- | --- | --- |
| `buffer.isPinned` | `buffer.store.ts:1704`、`pane-content.types.ts:75` | 标签级 pin，会**排到标签条最前**（`tab-bar.tsx:242-257` 的 `sortedBuffers`） |
| `pane.locked` | `pane.types.ts:9`、`pane.store.ts:458-464` → `pane-tree.ts:599-608` | **编辑器组**级锁。语义见 §2.3 |
| `paneId` / `isBottomPane` | `tab-bar.tsx:56-59,134` | 有 `paneId` 才给拆分与锁定项；底部窗（`BOTTOM_PANE_ID`）不给 |

---

## 2. 逐项语义与实现代价

### 2.1 可做性总表

「gpui 侧能不能做」的判据：**当前 `EditorPane` / `gpui/crates/editor` 的公开面** +
gpui-kit 0.6.6 的现成能力。`EditorPane` 的公开面很小（`lib.rs:26-32`：
`EditorPane` 的 `new` / `open` / `cursor_position` / `save_active` + `CursorPosition` + `install_actions`）。

| # | 菜单项 | 真机实际做什么（`文件:行号`） | 依赖状态 | gpui 侧 | 代价 / 缺口 |
| --- | --- | --- | --- | --- | --- |
| 1 | 固定标签页 | `handleTabPin`：`buffer.isPinned = !isPinned`，pin 时顺手清 `isPreview`，再同步到所有 pane，最后写工作区会话（`buffer.store.ts:1699-1714`） | 标签条排序 + 关闭按钮形态 | **能做** | `Buffer` 加 `is_pinned`（`buffer.rs:265-311`）；`display_names` 前先按 pin 稳定分组（对应 `tab-bar.tsx:242-257`）；关闭按钮位置换成 Pin 图标（对应 `tab-bar-item.tsx:158-178`）；`tabs.ariaPinnedSuffix` 进 aria（对应 `:140`）。**无新依赖** |
| 3 | 向右拆分 | `splitEditorGroup(paneId, "horizontal", bufferId)`（`tab-bar.tsx:741-747`）→ `createPaneBeside`（`pane-split-actions.ts:4-15`）→ `paneStore.splitPane`（`pane.store.ts:244-267`），新分组**继承被点的 buffer**（`pane-command-actions.ts:10-22,62-72`） | **pane 树** | **❌ 做不了** | 见 §2.2 |
| 4 | 向下拆分 | 同上，`direction = "vertical"`（`tab-bar.tsx:748-754`） | **pane 树** | **❌ 做不了** | 见 §2.2 |
| 5 | 锁定编辑器组 | `handleTogglePaneLocked` → `setPaneLocked(paneId, !isPaneLocked)`（`tab-bar.tsx:235-238`）→ `pane.store.ts:458-464` → `pane-tree.ts:599-608` | **pane 树 + 打开新 buffer 的路由** | **❌ 做不了** | 见 §2.3 |
| 6 | 复制路径 | `writeClipboardText(buffer.path)`（`tab-bar.tsx:353-355`） | 无 | **能做** | gpui 有 `App::write_to_clipboard(ClipboardItem)`（`gpui-pre-0.3.6/src/app.rs:1546`）。`EditorPane` 要在菜单回调里拿到 `App`（`PopupMenuItem::on_click` 给的是 `&mut App`，`popup_menu.rs:196-210`）——即可 |
| 7 | 复制相对路径 | `getRelativePath(path, rootFolderPath)`，无根则拷全路径（`tab-bar.tsx:357-368`） | **工作区根** | **能做** | `EditorPane` 目前**没有**工作区根字段（只有 `java: Option<Arc<JavaLanguageService>>`，`editor_view.rs:184`）→ 要加一个 `workspace_root: Option<PathBuf>`，由 `ShellWorkspace` 在调 `prepare_java` 的同一处转发（`editor_view.rs:216-231`）。相对化算法可直接照搬 `windows/tauri/src/utils/path-helpers.ts:99-117`（含 `\\`→`/` 归一与 `//?/` 前缀处理，`:6-18`） |
| 8 | 在资源管理器中显示 | Windows 侧走 `revealItemInDir(path)`（Tauri 插件，`file-system.store.ts:2689-2703`）；远端路径先拦（`fileSystem.revealLocalOnly`） | 平台层 | **能做（但要自己起进程）** | gpui **没有** `revealItemInDir` 等价物：全量 grep 无 `explorer.exe` / `Command` 的 reveal 调用（`gpui/crates/` 里只有 `terminal/src/profile.rs:103`、`session.rs:100`、`java/src/jdtls.rs:567`、`workbench/src/restart.rs:48`、`command_palette.rs:379` 五处 `Command::new`，都不是 reveal）。做法：`Command::new("explorer").arg(format!("/select,{}", path))`。**这条与 `explorer/src/lib.rs:86-87` 已经登记过的"「在资源管理器中显示」要起 explorer.exe，属于平台层，不该由这个 UI 模块做"是同一条约束** —— 若做，落点应在 workbench（外壳），由 `EditorPane` 发一个事件/回调出去 |
| 9 | 在终端中打开 | 取 `getDirName(buffer.path)`，以该目录起一个**终端 buffer**（`tab-bar.tsx:157-165`） | 终端 + 工作目录 | **能做，但要新 API** | `TerminalProfile::with_working_directory`（`terminal/src/profile.rs:70-71`）与 `Session::start` 的 `command.current_dir`（`session.rs:106-108`）**都已存在**；缺的是 `TerminalPane` 的公开入口只给 `ensure_session` / `new_tab`（`terminal/src/terminal_view.rs:6-7`、`session.rs:329,345`），**都不带目录参数**。要加一个 `TerminalPane::new_tab_in(dir, ..)` 之类的入口；且 `EditorPane` 不持有终端（`workbench/src/workspace.rs:389` 才有）→ 同样要经外壳转发 |
| 10 | 重新加载 | 关掉再重开：`closeBuffer(id)` → `setTimeout(100ms)` → `openBuffer(path, ...)`（`tab-bar.tsx:715-739`） | 无 | **能做** | 我们已有 `read_body`（`buffer.rs:90`）与 `open`（`editor_view.rs:243`）。语义要定：真机是**丢状态重读**（撤销栈、光标、滚动全丢）。建议等价实现 = `read_body` 重读 → `editor.set_value`（同时重置 `is_dirty = false`、`revision += 1`、清 `auto_save_task`）。⚠️ **脏标签直接重载会丢修改**：真机的 `closeBuffer` 走的是 `closeBuffer` 而非 force（`tab-bar.tsx:718`），所以**脏标签会先弹确认**（`buffer.store.ts:1348-1365`）→ 我们的实现也必须先走 `request_close` 那一档（见 §3） |
| 11 | 关闭 `Ctrl+W` | 上下文菜单里的 `close` → `onCloseTab(bufferId)`（`tab-context-menu.tsx:185`）→ `closeTab` → `handleTabClose` → `store.closeBuffer`（`tab-bar.tsx:370-376`、`buffer.store.ts:1695-1697`） | 无 | **✅ 已能做** | 等价于我们已有的 `request_close`（`editor_view.rs:928`）。**`Ctrl+W` 就是"关闭当前"**：`file.close` 的 `execute: closeActiveTab`，取的是 `bufferStore.activeBufferId`（`file-command-actions.ts:75-86`；命令注册 `command-registry.ts:180-185`，默认键 `cmd+w` → `default-keymaps.ts:48`；终端聚焦时改为 `terminal.close`，`use-keymaps.ts:178`）。**注意**：`closeActiveTab` 在**没有任何 buffer 时关窗口**（`file-command-actions.ts:85`），我们侧不做这件事（`request_close` 在无 buffer 时直接返回） |
| 12 | 关闭其他 | `handleCloseOtherTabs(keepId)`：`buffers.filter(b => b.id !== keepId && !b.isPinned)`（`buffer.store.ts:1742-1759`） | 无 | **能做** | 语义见 §2.4 |
| 13 | 关闭右侧 | `handleCloseTabsToRight(id)`：`buffers.slice(index + 1).filter(b => !b.isPinned)`（`buffer.store.ts:1810-1830`） | 数组顺序 | **能做** | 语义见 §2.4。⚠️ 真机用的是 **`state.buffers` 的原始下标**（不是 pin 排序后的可视顺序） |
| 14 | 全部关闭 | `handleCloseAllTabs()`：`buffers.filter(b => !b.isPinned)`（`buffer.store.ts:1761-1777`） | 无 | **能做** | 语义见 §2.4 |

**汇总：13 项里 10 项可做**（1 / 6 / 7 / 8 / 9 / 10 / 11 / 12 / 13 / 14），
**3 项依赖分屏**（3 / 4 / 5）。第 8、9 两项"能做但落点在外壳"，第 7 项要补一个工作区根字段。

### 2.2 向右拆分 / 向下拆分：真机怎么实现、我们缺什么

**真机是"编辑器组（pane 树）"分屏，不是标签内的分栏。**

1. 每个编辑区是一个 **pane 树**：`PaneNode = group | split`，`root` / `bottomRoot` 两棵树
   （`panes/types/pane.types.ts`，`pane.store.ts:244-267`，`pane-tree.ts`）。
2. 拆分 = `splitPane(paneId, direction, bufferId, "after")`：
   把目标 `group` 一分为二，**新组继承被右键的那个 buffer**（`pane-split-actions.ts:4-15`、
   `pane-command-actions.ts:62-72`），然后 `state.activePaneId = 新组`（`pane.store.ts:258-263`）。
3. 方向字面量与文件名**相反**（容易接错）：
   `"horizontal"` = 「向右拆分」（`tab-bar.tsx:744`），`"vertical"` = 「向下拆分」（`:751`）。
4. 拆分后标签条是**每个 group 一条**（`pane-node-renderer.tsx:54` → `pane-container.tsx:319` 的 `TabBar`），
   拖放还有五区遮罩（`split-drop-overlay.tsx`、`pane-drop-zones.ts`）。
5. 旁支能力一整套：`closePane`（关组合并回退组，`pane.store.ts:269-306`）、
   `togglePaneFullscreen`、`moveActiveEditorToAdjacentGroup`、`resetEditorGroupSizes`
   （`pane-command-actions.ts:74-160`）、`mostRecentActivePaneIds` 的焦点历史。

**我们缺的是整个 pane 层**：

- `EditorPane` 的字段里**没有**任何 pane 概念：只有 `buffers: Vec<Buffer>` 与 `active: Option<usize>`
  （`editor_view.rs:161-187`）——**扁平一层，没有分组**。
- 渲染是 `v_flex().child(tab_bar).child(body)`（`editor_view.rs:1423-1438`），单窗格。
- 外壳的布局是固定的三栏 + 底部窗（`workbench/src/workspace.rs:943-1014`），
  编辑岛里只挂一个 `explorer` / 一个 `editor` 实体（同文件 `:944`、`:1014`），**没有树形组合**。
- `gpui/UI-MAP-WINDOWS.md:831` 已给出的候选路（`component::dock::{DockArea, ...}` 的
  `DockArea::split_at`）**与真机的 pane 树不是一回事**：dock 是"面板停靠"，改造成
  "编辑器组"要重写 tab 归属、焦点历史、拖放五区。

**不建议做假控件的理由**（以及反过来的"为什么不做"，逐条）：

1. **没有落点的菜单项就是谎言**：真机上这两项的**唯一**作用就是改变布局（`pane-command-actions.ts:62-72`
   只做 `createPaneBeside`）。假实现只有两种：什么都不做（用户点两次后不再信这个菜单），
   或者变成"新开一个主窗格"（真机不存在这种形态，等于引入一个规格里没有的行为）。
2. **真源里"置灰"没有先例**（§1.2 的 ⚠️）：整个标签菜单没有一项是禁用的。真机的做法是
   **整项不出现**（`tab-context-menu.tsx:88,98,111` 三处 spread 条件），而且这个条件在我们这里是
   **恒假**（没有 `paneId`，也就永远不会传 `onSplitRight` / `onSplitDown` / `onTogglePaneLocked`）。
   所以**最忠实的落地是"不放这两项/这一项"，而不是放一个灰的**。
3. **保留"灰项"的代价反而更高**：它需要给 `EditorPane` 引入一个恒为 `false` 的
   `pane_locked` 状态、一个恒为 `None` 的 `pane_id`，以及与之配套的菜单分支 ——
   这就是《编码指南》说的投机性抽象，且会让后来真的做分屏的人以为已经有一半了。

**建议（按优先级）**：

| 方案 | 建议 |
| --- | --- |
| A. **本轮不放这 3 项** | ✅ **推荐**。与真机的"条件不满足就整项不出现"完全同构（`tab-context-menu.tsx:88-121`），改分屏时把 `pane_id` 从 `None` 换成真值，三项自然出现 |
| B. 放灰项（`.disabled(true)`，能力见 `menu.md:100-111`） | ❌ 不推荐。真源无先例 + 需要引入恒假状态 |
| C. 放项并留 `TODO` 注释 | ❌ 不推荐。菜单是用户可见面，"点了没反应"比"没有这一项"更糟 |
| D. 真的做分屏 | 另开一轮：pane 树 + `splitPane`/`closePane` + 每组一条标签条 + 拖放五区。这是**独立 feature**，不是本菜单的附属工作 |

### 2.3 锁定编辑器组：真机语义（这条最容易误解）

`pane.locked` **不是**"这个组不能被关标签"，而是**"新打开的 buffer 不许进这个组"**：

- 写入口：`resolveWritablePaneForBuffer`（`panes/utils/pane-routing.ts:60-82`）。
  当前活动组若 `!locked`，或该 buffer 已经在组里，就用它（`:69-71`）；
  否则在**同一 scope** 的组里按 `mostRecentActivePaneIds` 找一个**未锁定**的组（`:75-81`），
  一个都没有就返回 `null`。
- 所以锁定一个**唯一的**编辑组 = 之后新开文件**无处可去**（`null`）—— 真机在这个状态下
  打开文件的行为我们**没有验证**（§6 第 4 条）。
- 组级 `pinnedBufferIds` 是**另一件事**（`pane-tree.ts:564-597`），别和 `locked` 混。

**我们缺什么**：① pane 概念本身（§2.2）；② 打开文件的路由点。
我们的 `EditorPane::open` 是"永远进自己这一个列表"（`editor_view.rs:243-307`），
没有第二个组可以路由过去。所以这一项即使做成"能切换一个 bool"，也**没有任何可观察后果** ——
这比拆分两项更彻底地"没有落点"。

### 2.4 三个"关闭"的精确定义 + 边界行为

真源实现（`buffer.store.ts:1742-1830`）与边界：

| 动作 | 关闭集合 | 只有 1 个标签（且就是被点的那个） | 只剩脏标签 | 全是固定标签 | 1 个标签 + 其他情形 |
| --- | --- | --- | --- | --- | --- |
| **关闭其他** `handleCloseOtherTabs(keepId)` | `buffers.filter(b => b.id !== keepId && !b.isPinned)`（`:1744`） | 集合为空 → **无操作、无确认、菜单不关也不报错**（`forEach` 空转，`:1758`） | 只剩它一个（其余已关）→ 同上，**空转** | 别的都被 pin 过滤掉 → **空转** | — |
| **关闭右侧** `handleCloseTabsToRight(id)` | `buffers.slice(index+1).filter(b => !b.isPinned)`（`:1815`）；`index === -1` 直接 `return`（`:1813`） | 被点的是最后一个 → `slice` 为空 → **空转** | 右侧的脏标签 → 进入 `pendingClose{type:"to-right", anchorBufferId}`（`:1818-1827`） | 右侧全 pin → **空转** | — |
| **全部关闭** `handleCloseAllTabs()` | `buffers.filter(b => !b.isPinned)`（`:1763`） | 关掉它 → 编辑区进入空状态 | 进入 `pendingClose{type:"all"}`（`:1766-1774`） | **空转**（固定标签永远关不掉） | — |
| **关闭当前**（`close` / `Ctrl+W`） | 只关 `activeBufferId`（`file-command-actions.ts:75-86` → `buffer.store.ts:1348-1365`） | 关掉它 → 空状态 | 进入 `pendingClose{type:"single"}` | 真机**不拦**：`closeBuffer` 只看 `isEditorContent(buffer) && buffer.isDirty`（`:1354`），**pin 不参与**。唯一的 pin 保护在键盘 `Delete`/`Backspace` 上（`tab-bar.tsx:588-596` 打 `tabs.cannotClosePinned`），以及关闭按钮的**语义替换**（pin 时按钮变成"取消固定"，`:158-168`） | — |

**三条关键结论**：

1. **"只有 1 个标签"时，关闭其他 / 关闭右侧 / 全部关闭都不会报错、不弹窗、不置灰** ——
   它们静默什么也不做。我们的落地应保持同样行为（`retain` 出一个空集合 → 直接返回）。
   真机之所以敢这么做，是因为它**不置灰**（§1.2）。
2. **固定标签被三道过滤保护**：`closeOthers`(`:1744`)、`closeRight`(`:1815`)、`closeAll`(`:1763`)
   都带 `!b.isPinned`。**做不到 pin 就别放这三个的"全关"承诺** ——
   但这一条不影响本轮：我们不做 pin 也能实现这三个（只是少了保护层）。
3. **`Ctrl+W` 就是"关闭当前"**（不是"关闭全部"、不是"关闭窗口"）：
   `command-registry.ts:180-185` 的 `file.close` → `closeActiveTab`（`file-command-actions.ts:75`）。
   默认键在 `default-keymaps.ts:48`（`cmd+w`，Windows 上映射为 `Ctrl+W`；
   终端聚焦时 `use-keymaps.ts:178` 改成 `terminal.close`）。
   真机在**没有任何 buffer** 时 `Ctrl+W` 会**关窗口**（`file-command-actions.ts:85`），
   这一点我们**不复刻**（`editor_view.rs:928` 的 `request_close` 在空列表时直接返回）。

**`closeToRight` 的数组顺序陷阱**：真机用 `state.buffers` 的原始下标（`buffer.store.ts:1812`），
而标签条的**可视顺序**是 pin 优先重排过的（`tab-bar.tsx:242-257`）。
两个都是"真机行为"，但**我们只要有一份顺序**（`display_names` 输入的那一份）就不会矛盾。
建议：在 `EditorPane` 内维护**一份**顺序（pin 在前），三个关闭动作都基于它算集合。

---

## 3. 与未保存确认的关系（阶段 9 的 `request_close`）

### 3.1 真机的处理：逐个发现、**批量确认**

真机的 `pendingClose` 是一个**单值**（`buffer.store.ts:121`、`PendingClose` 定义），
流程是：

1. 动作先算出**要关的整个集合**，然后 `find(b => isEditorContent(b) && b.isDirty)` 找
   **第一个**脏的编辑 buffer（`closeOthers :1746`、`closeAll :1765`、`toRight :1817`、`toLeft :1795`）。
2. 找到就**只记这一条** `pendingClose = { bufferId: 那个脏 buffer, type: "others"/"all"/"to-right", keepBufferId / anchorBufferId }`
   然后 `return` —— **集合里其余脏标签一条都不问**（`:1748-1756`、`:1767-1774`、`:1818-1827`）。
3. 弹窗 `PendingBufferCloseDialog`（`window/components/pending-buffer-close-dialog.tsx:7-35`），
   正文文件名取的是 **`pendingClose.bufferId` 那个 buffer**（`:9-12`），不是被右键的那个。
4. 用户点：
   - **不保存** → `confirmCloseWithoutSaving()`：按 `type` **重算一次集合**，然后
     `buffersToClose.forEach(closeBufferForce)` —— **整批无确认关闭**（`buffer.store.ts:2064-2113`）。
   - **保存** → `handleSave(pendingClose.bufferId)`，**只有 `result === "saved"` 才**接着
     `confirmCloseWithoutSaving()`（`pending-buffer-close-dialog.tsx:16-23`）。
     保存失败 → 弹窗不关（和我们的 `write_buffer` 失败不关对话框同构，`editor_view.rs:1008-1010`）。
   - **取消** → `cancelPendingClose()`（`:2115-2119`），集合一个都不关。

### 3.2 哪些必须复用、以什么粒度

| 菜单项 | 必须复用 `request_close`？ | 粒度 |
| --- | --- | --- |
| 关闭（`Ctrl+W`） | ✅ **必须** | **逐个**：当前只有一个目标，`request_close(index)` 就是它（已有） |
| 重新加载 | ✅ **必须**（真机走 `closeBuffer` 而非 force，`tab-bar.tsx:718`） | **逐个** |
| 关闭其他 / 关闭右侧 / 全部关闭 | ✅ **必须复用那一档，但不能逐个调** | **批量**：算集合 → 找第一个脏的 → 弹**一次** → 确认后整批关 |

**为什么不能"逐个调 `request_close`"**：我们现在 `request_close` 是**每调用一次就
`window.open_dialog`**（`editor_view.rs:943`）。3 个脏标签连调 3 次 = 3 个叠起来的对话框
（且 `open_dialog` 属于浮层，同时开多个的最终形态没有验证）。这与真机的"**只问一次**"
（`buffer.store.ts:1746-1755`）是两个交互。

**建议的落点（不改现有语义）**：给 `EditorPane` 加一个**批量版**入口，与 `request_close` 并列，
两者共用同一段"脏 → 弹窗"的代码：

- 新增 `fn request_close_batch(&mut self, targets: Vec<usize>, window, cx)`：
  1. 过滤掉 `!is_dirty`（干净的直接关，与真机 `:1746` 的 `find` 同构）；
  2. 取**第一个**脏的（真机就是 `find`）作为弹窗正文的文件名；
  3. 弹窗按钮：`取消` / `放弃修改` / `保存`，语义与现有完全一致
     （`editor_view.rs:950-1012` 的 `lithe.unsavedChanges.title` + `lithe.editor.gpui.discardChanges`
     + `lithe.ui.save`）；
  4. `保存` 分支只保存**那一个** buffer（真机 `handleSave(pendingClose.bufferId)` 也只保存它，
     `pending-buffer-close-dialog.tsx:19`），成功后**整批关**；
  5. `放弃修改` / 保存成功后：`targets` 里的干净项 + 脏项**一起关**（真机 `:2081-2109`）。
- ⚠️ 判断脏的**时机**：真机在**点菜单时**算集合与脏项，在**确认后重算集合**
  （`:2081`、`:2088`、`:2097`、`:2107`）。这中间用户可能又改了别的文件 ——
  重算会把"确认后新变脏的标签"也一并关掉且不再问。**我们要不要复刻这个重算**：
  建议**不复刻**（按点击时定下的下标集合关，越界/不存在就跳过），因为重算会引入
  "关掉一个从没被确认过的脏标签"的窗口。这一点登记为**刻意的行为差异**（§6 第 5 条）。

### 3.3 与 `close` 的现状对齐

现有 `close(index)`（`editor_view.rs:899-918`）已经在处理"被关标签之后的标签左移"：
批量关闭时**必须**先按"要关的下标集合"算好，然后**从大到小**逐个 `close`，
否则下标会错位。这条是纯粹的实现注意点，登记在此以免踩。

---

## 4. 文案落地：i18n 键逐条核对

### 4.1 gpui 侧现状

- 真源是 `windows/tauri/src/i18n/locale.ts`，经 `gpui/tools/extract-locale.mjs`
  生成 `gpui/crates/shared/locales/lithe.{zh-CN,en}.yml`（脚本头部注释 `extract-locale.mjs:1-30`）。
- 生成物里同时有 `zh-CN:` 与 `en:` 两条（例：`lithe.zh-CN.yml:5916-5917`、`:6816-6817`）；
  调用侧统一 `tr("lithe.tabs.pin")` / `tr_args`（`gpui/crates/shared/src/i18n.rs`）。
- 有一条测试 `every_wired_key_resolves_in_both_locales` 守着"接线的键必须两种语言都能解析"
  （`gpui/crates/shared/src/i18n.rs:52-53`）—— 新接的键会被它检查。

### 4.2 逐条核对表

| 键 | zh-CN 文案 | `locale.ts:行号` | gpui yml（`lithe.zh-CN.yml:行`） | 状态 |
| --- | --- | --- | --- | --- |
| `tabs.pin` | 固定标签页 | **7838** | **6816-6817** | ✅ 存在 |
| `tabs.unpin` | 取消固定标签页 | **7839** | **6818-6819** | ✅ 存在 |
| `tabs.splitRight` | 向右拆分 | **7840** | **6820-6821** | ✅ 存在（本轮若不做分屏则不接） |
| `tabs.splitDown` | 向下拆分 | **7841** | **6822-6823** | ✅ 存在（同上） |
| `tabs.lockEditorGroup` | 锁定编辑器组 | **7842** | **6824-6825** | ✅ 存在（同上） |
| `tabs.unlockEditorGroup` | 解锁编辑器组 | **7843** | **6826-6827** | ✅ 存在（同上） |
| `files.copyPath` | 复制路径 | **7388** | **5916-5917** | ✅ 存在 |
| `files.copyRelativePath` | 复制相对路径 | **7392** | **5924-5925** | ✅ 存在 |
| `files.reveal` | 在资源管理器中显示 | **7400** | **5940-5941** | ✅ 存在 |
| `files.openInTerminal` | 在终端中打开 | **7426** | **5992** | ✅ 存在 |
| `tabs.reload` | 重新加载 | **7844** | **6828-6829** | ✅ 存在 |
| `tabs.close` | 关闭 | **7845** | **6830-6831** | ✅ 存在（**已在用**：`editor_view.rs:1245-1246`） |
| `tabs.closeOthers` | 关闭其他 | **7846** | **6832-6833** | ✅ 存在 |
| `tabs.closeToRight` | 关闭右侧 | **7847** | **6834-6835** | ✅ 存在 |
| `tabs.closeAll` | 全部关闭 | **7848** | **6836-6837** | ✅ 存在 |

**缺失的：无。** 15 个键（含 2 个条件同胞键 + 3 个本轮不接的分屏/锁定键）**全部存在于两份 locale**，
所以本菜单**不需要动 `extract-locale.mjs` 的 `GPUI_ONLY_KEYS`**（对照 `i18n.rs:292-303` 的做法）。

### 4.3 可选：两个额外文案（如果落地 §2.1 的第 8、9 项）

| 场景 | 可选键 | 出处 |
| --- | --- | --- |
| 显示失败 / 远端路径 | `fileSystem.revealLocalOnly`（"在文件夹中显示仅适用于本地工作区。"） | `locale.ts:4716`（en `:258`） |
| 终端打开失败 | `terminal.shellUnavailable` 等（已有接线，见 `terminal/src/constants.rs` 一带） | `locale.ts:7549` 一档 |

以上两条**不是必需**（我们的 buffer 全是本地路径），登记备查。

---

## 5. gpui 侧接线要点

### 5.1 `ContextMenuExt` 怎么挂

现有唯一用法是**空态**（`editor_view.rs:1336-1341`）：

```rust
v_flex()
    .flex_1()
    .min_h_0()
    .context_menu(|menu, _window, _cx| {
        menu.item(PopupMenuItem::new(tr("lithe.ui.noActionsHere")).disabled(true))
    })
```

要点（逐条给证据）：

1. **`ContextMenuExt` 是给任意 `InteractiveElement + ParentElement + Styled` 的扩展 trait**
   （`gpui-component-0.6.6/src/menu/context_menu.rs:13,39`），`div` 天然满足。
   已 import 在 `editor_view.rs:52`（`ContextMenuExt as _`）。
2. ⚠️ **元素必须有稳定且唯一的 `id`，否则同一个 `context_menu(...)` 调用点上的所有标签会共用
   一个元素状态**：`context_menu` 的 id 取 `self.interactivity().element_id`，
   `None` 时退回 `ElementId::CodeLocation(caller)` —— **同一个源码位置 = 同一个 id**
   （`context_menu.rs:26-35`）。`Tab::new()` 不带 id，`Tab` 的 `element_id()` 是 `None`
   （`tab/tab.rs:598-604` 只转发 `interactivity()`）→ **不能直接给 `Tab` 挂 `context_menu`**。
   做法：像 `explorer/src/lib.rs` 的经验一样，**给每个标签包一层 `div().id(("editor-tab", index))`
   再 `.context_menu(...)`**（`div` 的 id 是 `StatefulInteractiveElement` 要求的，
   参考 `terminal_view.rs:374-376` 的注释：`.id(..)` 先于 role/aria）。
3. **菜单构建闭包拿不到"被点的标签"**：签名是
   `Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu`（`context_menu.rs:19-22`），
   **没有鼠标事件、没有位置**。且它是在**下一帧**的 `window.defer` 里跑的（`context_menu.rs:324-333`）。
   → **上下文必须在 `render_tab_bar` 的循环里"当场捕获"**：per-index 闭包捕获 `index`（以及
   该标签的 `path` / `is_dirty` / `is_pinned` 等快照），`move` 进去。
   这与 `render_tab(index, buffer, name, cx)`（`editor_view.rs:1147-1154`）的现有形状一致，
   **不需要引入新的状态字段来表示"右键的是谁"**。
4. **右键语义**：`ContextMenuExt` 只认 `MouseButton::Right` 的按下（`context_menu.rs:296-299`），
   `Tab` 自己的选中逻辑不吃右键，**不需要额外处理**。位置上它会
   `anchored().position(event.position).snap_to_window_with_margin(px(8.))`（`:204-205`）。

### 5.2 菜单项与回调：能拿到哪些上下文

| API | 默认行为 | 我们要的 |
| --- | --- | --- |
| `menu.menu(label, Box<dyn Action>)` | 派发一个 action（`popup_menu.rs:465`） | ❌ 不带"哪个标签"。用它就得为每个标签造一个带 index 的 action 实例，不如 `item` |
| `PopupMenuItem::new(label)` | 纯标签项（`popup_menu.rs:71-81`） | ✅ 主用 |
| `.icon(IconName::X)` | 左侧图标（`:126-140`） | ✅ 图标键可直接用 lucide 名（`IconName` 由 `assets/icons/*.svg` 生成，变体名 = 文件名的 PascalCase，`gpui-kit-assets-0.6.6/build.rs:26-38`）。已确认 `columns-2` / `rows-2` / `lock` / `lock-open` / `pin` / `pin-off` / `copy` / `folder-open` / `rotate-ccw` / `x` / `terminal` 的 svg **都在**目录里 |
| `.disabled(bool)` | 禁用态（`:161-175`） | 见 §2.2 方案 A：**不用** |
| `.on_click(handler)` | `handler: Fn(&ClickEvent, &mut Window, &mut App)`（`:196-210`） | ✅ **主用**。注意：**`&mut Window` 有**（`popup_menu.rs:868,879` 传的是 `window`），所以**可以直接 `window.open_dialog(cx, ..)`**（浮层约束：不能在 `render` 里开，见 `editor_view.rs:926-927` —— 菜单回调不是 render，满足） |
| `menu.separator()` | 分隔线（`:680`；项工厂 `PopupMenuItem::separator()` 在 `:113`） | ✅ |

**回调里怎么回到 `EditorPane`**：`on_click` 的第三参是 `&mut App`（**不是** `Context<EditorPane>`），
所以用 `window.listener_for(&entity, |pane, event, window, cx| ...)`
（`gpui-pre-0.3.6/src/window.rs:6626-6635`）——它内部 `view.downgrade()` 再 `update`，
视图已销毁时静默 `ok()`，正是我们要的。

形状（示意，非代码改动）：

```rust
// render_tab_bar 的循环里，per-index 捕获
let pane = cx.entity();                       // 或 cx.weak_entity()
let path = buffer.path.clone();
let menu_index = index;

div()
    .id(("editor-tab", index))                // ← 必须：稳定唯一 id
    .context_menu(move |menu, window, cx| {   // ← 闭包在下一帧跑，捕获此刻的 index/path
        let pane = pane.clone();
        menu.item(
            PopupMenuItem::new(tr("lithe.tabs.closeOthers"))
                .on_click(window.listener_for(&pane, move |pane, _e, window, cx| {
                    pane.close_others(menu_index, window, cx);
                })),
        )
        // ... 其余项 + menu.separator()
    })
    .child(tab)
```

### 5.3 需要新增的落点（不改现有语义）

| 落点 | 位置建议 | 说明 |
| --- | --- | --- |
| `ContextMenuExt` 的 per-tab 包装 | `editor_view.rs::render_tab` / `render_tab_bar`（`:1032-1059`、`:1147-1227`） | 现有 `render_tab` 返回 `Tab`，包装层要返回能带 id 的元素 |
| `copy_path` / `copy_relative_path` | `EditorPane` 新方法 | `cx.write_to_clipboard`（`gpui-pre-0.3.6/src/app.rs:1546`）；相对化需要 `workspace_root` 字段 |
| `reveal_in_explorer` / `open_in_terminal` | **外壳**（workbench） | 与 `explorer/src/lib.rs:86-87` 已登记的同一条理由：起进程属平台层；`EditorPane` 只发请求（回调/事件），由 `ShellWorkspace` 做 |
| `reload` | `EditorPane` 新方法 | 复用 `read_body`（`buffer.rs:90`）；脏标签先走 §3 的确认档 |
| `close_others` / `close_to_right` / `close_all` | `EditorPane` 新方法 | 与 `close`（`:899`）/ `request_close`（`:928`）并列；批量确认见 §3.2 |
| `pin` | `Buffer` 字段 + `display_names` 前的排序 + 关闭按钮形态 | §2.1 第 1 行 |
| `workspace_root` | `EditorPane` 字段 | 由 `ShellWorkspace` 在 `prepare_java` 同一处转发（`editor_view.rs:216-231`） |

---

## 6. 未确认的点

1. **`splitRight` / `splitDown` 的方向字面量与中文文案的对应关系**：
   已确认 `tab-bar.tsx:744`（`"horizontal"` ← `tabs.splitRight`）与 `:751`（`"vertical"` ← `tabs.splitDown`），
   但 `"horizontal"` 在 `pane-tree.ts::splitPane` 内部的坐标语义（是"左右分"还是"上下分"）
   **没有读到实现体**（只读了 `pane.store.ts:244-267` 的调用层）。本轮不做分屏，影响有限；
   真要做时**必须**先确认这个映射，否则会左右/上下搞反。
2. **`sept-2`（分隔线）在"有 `paneId` 但两个拆分回调都没传"时不会出现** ——
   调用方总是两个一起传（`tab-bar.tsx:741-754`），所以实践中 `sep-2 ⟺ paneId`。
   但 `onSplitRight` / `onSplitDown` 各自独立可缺（`tab-context-menu.tsx:88,98`），
   这个组合在真机有没有真实路径**没找到**（不做分屏时无影响）。
3. **锁定最后一个（唯一的）编辑组之后再打开文件会怎样**：
   `resolveWritablePaneForBuffer` 在有锁且无别组时返回 `null`（`pane-routing.ts:75-81`），
   调用方对 `null` 的处理**没有追**（超出本菜单范围）。做分屏时必须补这一条。
4. **`revealItemInDir` 的确切行为**：真源调的是 Tauri 插件（`file-system.store.ts:2702`），
   我只确认了"我们侧没有等价物、要自己起 `explorer.exe`"。`/select,` 的引号与
   含空格路径的转义**没有实测**（本轮不允许跑应用）。WSL 路径真源还会先
   `wsl_resolve_windows_path`（`:2697-2699`）—— 我们没有 WSL 工作区，按"不支持"处理。
5. **批量关闭的"确认后是否重算集合"**：真机会重算（`buffer.store.ts:2081-2109`），
   §3.2 建议我们**不重算**。这是一个**刻意的行为差异**，需要维护者拍板。
6. **脏项的确认弹窗一次只展示第一个脏标签的文件名**（真机 `pending-buffer-close-dialog.tsx:9-12`
   + `buffer.store.ts:1746` 的 `find`）—— 也就是说用户看到"是否保存对 A 的更改？"，
   确认后**B、C 也会一起被关掉且没被问过**。这是真机行为，但读起来像 bug；
   我们的 `lithe.editor.gpui.unsavedChangesBody`（`i18n.rs:305-308`）只支持单个 `{name}` 占位，
   如果维护者想改成"共 N 个文件有未保存修改"就需要新键（`GPUI_ONLY_KEYS`）。**本条要维护者拍板**。

---

## 附：证据索引（按文件）

| 文件 | 关键行 |
| --- | --- |
| `windows/tauri/src/features/tabs/components/tab-context-menu.tsx` | 菜单数组 `:70-202`；渲染 `:204-218`；`closeKeys` `:69` |
| `windows/tauri/src/features/tabs/components/tab-bar.tsx` | 实例化 `:675-756`；`isPaneLocked` `:132`；`isBottomPane` `:134`；复制路径 `:353-368`；`closeTab` `:370-376`；键盘 `Delete` `:588-596`；pin 排序 `:242-257` |
| `windows/tauri/src/features/editor/stores/buffer.store.ts` | `closeBuffer` `:1348-1365`；`closeBufferForce` `:1367-1378`；`handleTabPin` `:1699-1714`；`handleCloseOtherTabs` `:1742-1759`；`handleCloseAllTabs` `:1761-1777`；`handleCloseTabsToRight` `:1810-1830`；`confirmCloseWithoutSaving` `:2064-2113`；`cancelPendingClose` `:2115-2119` |
| `windows/tauri/src/features/panes/utils/pane-command-actions.ts` | `splitEditorGroup` `:62-72`；`toggleActiveEditorGroupLock` `:42-51` |
| `windows/tauri/src/features/panes/utils/pane-routing.ts` | `resolveWritablePaneForBuffer` `:60-82` |
| `windows/tauri/src/features/panes/stores/pane.store.ts` | `splitPane` `:244-267`；`closePane` `:269-306`；`setPaneLocked` `:458-464` |
| `windows/tauri/src/features/panes/utils/pane-tree.ts` | `setPaneLocked` `:599-608`；`setPaneBufferPinnedEverywhere` `:564-597` |
| `windows/tauri/src/features/keymaps/commands/file-command-actions.ts` | `closeActiveTab` `:75-86`；三个批量关闭 `:92-119` |
| `windows/tauri/src/features/keymaps/commands/command-registry.ts` | `file.close` `:179-185`；`file.closeAll` `:193-198`；`file.closeOthers` `:199-204`；`file.closeTabsToRight` `:217-222` |
| `windows/tauri/src/features/window/components/pending-buffer-close-dialog.tsx` | 全文 `:1-35` |
| `windows/tauri/src/i18n/locale.ts` | 见 §4.2 |
| `gpui/crates/editor/src/editor_view.rs` | 字段 `:161-187`；`open` `:243-307`；`close` `:899-918`；`request_close` `:928-1015`；`render_tab_bar` `:1032-1059`；`render_tab` `:1147-1227`；空态菜单 `:1336-1341` |
| `gpui/crates/editor/src/buffer.rs` | `Buffer` `:265-311`；`read_body` `:90-133` |
| `gpui/crates/editor/src/lib.rs` | 公开边界 `:26-32`；范围外清单（含"标签右键菜单"）`:112-115` |
| `gpui-component-0.6.6/src/menu/context_menu.rs` | trait `:13-39`；闭包签名 `:19-22`；id 生成 `:26-35`；右键判定 `:296-299`；defer 构建 `:324-333` |
| `gpui-component-0.6.6/src/menu/popup_menu.rs` | `PopupMenuItem::new` `:71`；`icon` `:126`；`disabled` `:161`；`on_click` `:196-210`；`separator` `:113,680`；handler 调用点 `:859-879` |
| `gpui-pre-0.3.6/src/window.rs` | `listener_for` `:6626-6635` |
| `gpui-pre-0.3.6/src/app.rs` | `write_to_clipboard` `:1546` |
| `gpui/crates/shared/locales/lithe.zh-CN.yml` | §4.2 |
| `gpui/docs/gpui-kit/0.6.6/zh-CN/component/menu.md` | `context_menu` `:27-38`；禁用 `:100-111`；自定义 `item`+`on_click` `:325-344`；分隔线 `:131-141` |
| `gpui/crates/explorer/src/lib.rs` | 平台层不做 reveal 的理由 `:86-87` |
| `gpui/UI-MAP-WINDOWS.md` | 编辑区结构 `:765-813`；元素对应 `:825-840`；文案 `:844-864`；dock 分屏候选 `:831` |
