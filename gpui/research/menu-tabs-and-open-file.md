# 菜单栏 / 标签页「胶囊」关闭 / 打开其他文件 —— 只读侦察报告

**任务**：维护者要动三件事（① 把截图里那些菜单项补齐/摆位；② 文件胶囊点不掉；③ 打开其他文件没实现）。
本文件只登记**事实**：每条都给 `文件:行号`。**推断**单独标注。

**方法（硬边界）**：只读 + `rg`/读文件/`git log`。**没有跑 `cargo`、没有启动 Lithe、没有改任何产品代码**。
所以下面凡"实测"二字，指的是**读源码/读上游源码得出的判定**，不是实机点击结果；真正的实机验证办法写在 §5。

**分支**：`feat/gpui-shell-rewrite`，HEAD = `3096bd18`。
**本轮未取得**维护者的 9 张截图（仓库里最近新增的图是 `.artifacts/p20/*`，2026-09-26，与菜单无关），
所以"截图里有什么"只能以维护者消息里的文字清单为真源；该清单与真源 89 条**逐条对齐**（见 §2 说明）。

---

## 1. 三件事：现状 → 缺口 → 最小改动落点

### A. 菜单栏

| | 内容 |
| --- | --- |
| **现状** | 9 个顶级菜单**齐全**且顺序照真源（`gpui/crates/workbench/src/menu_bar.rs:305-378` 的 `MENUS`，顺序断言在 `:1112-1125`）。但每个菜单里**只列真能执行的项**：全文只有 **13 条 `MenuAction`**（`:184-211`）。菜单项**不显示任何快捷键**（见 §1.A4）。 |
| **缺口** | 与真源 89 条相比：**11 条已实现、0 条空壳、78 条缺项**（逐条见 §2）。其中 21 条是"底层能力已经在、只缺菜单项/动作"（含**刚实现的快速修复 `Ctrl+.`**）。 |
| **最小改动落点** | ① 菜单数据与动作枚举：`gpui/crates/workbench/src/menu_bar.rs` 的 `MENUS`(`:305-378`)、`MenuAction`(`:184-211`)、`label_key`(`:219-239`)、`icon`(`:253-269`)、`command_id`(`:275-291`)；② 执行分支：`gpui/crates/workbench/src/workspace.rs` 的 `apply_menu_action`(`:1136-1208`)、`run_command_id`(`:1217-1314`)；③ 新动作 id：`gpui/crates/workbench/src/command_palette.rs` 的 `CommandId`(`:187-210`) + `id()`(`:214-228`)。 |

**注意两条本侧口径（不是遗漏）**：`menu_bar.rs:46-56` 明确写了"只列真能执行的项、不摆一片灰"，
并用单测 `every_listed_action_is_executable`(`:1146-1188`) 钉死"表里不许有死项"；
`menu_bar.rs:54-56` 也写明这与真源（89 条全画、只按 4 处后端能力灰化）**有意偏离**。
所以"补齐菜单项"这件事，要么同时补实现，要么按维护者新口径改成"可以摆位但先不实现"。

#### A4. 菜单项的快捷键显示是从哪来的

- **写死还是 keymap？→ 都不是：现在一条都不显示。**
  菜单项的构造在 `menu_bar.rs:1047-1054`：`PopupMenuItem::new(action.label()).icon(...).on_click(...)`
  —— **没有 `.action(..)`**，所以没有可解析的 `Action`。
- 上游的渲染点：`gpui-component-0.6.6/src/menu/popup_menu.rs:1119-1144` 的 `render_key_binding(action, …)`
  （`Kbd::binding_for_action_in` / `Kbd::global_binding_for_action`），调用点在 `:1301`。
  **它只从 item 的 `action` 字段解析 keymap 绑定**；item 没有 action → 返回空 → 面板右侧那一列空白
  （真源是有这一列的：`ui/menubar.tsx:175-176`，见 `gpui/research/windows/10-menu-bar.md:270`）。
- 需要的那颗"解析用焦点句柄"**已经接好了**：`menu_bar.rs:1028` 的 `.action_context(action_context.clone())`
  （用途见 `popup_menu.rs:291,410`）。
  → **最小改动**：给 `build_popup`(`menu_bar.rs:1014-1068`) 的 item 传 `.action(..)`（需要把 `MenuAction` 变成 gpui `Action`），
  或退一步用 `.suffix(Kbd)` 写死展示值（真源文档 `10-menu-bar.md:116-259` 的"快捷键"列可直接抄）。

**上游 `gpui-base-0.6.6` / `gpui-component-0.6.6` 已绑的键（`gpui-base-0.6.6/src/input/base/state.rs`，Windows 分支 `#[cfg(not(target_os = "macos"))]`）**：

| 键 | action | 行号 |
| --- | --- | --- |
| `ctrl-a` | SelectAll | `:246` |
| `ctrl-c` | Copy | `:250` |
| `ctrl-x` | Cut | `:254` |
| `ctrl-v` | Paste | `:258` |
| `ctrl-z` | Undo | `:288` |
| `ctrl-y` | Redo（真源是 `ctrl-shift-z`） | `:290` |
| **`ctrl-.`** | **ToggleCodeActions（= 快速修复）** | **`:294`** |
| `ctrl-f` | Search | `:298` |
| `ctrl-h` | Replace（真源是 `ctrl+alt+f`） | `:302` |

其余上游绑定（都不含菜单项要的那些键）：`popup_menu.rs:23-28`（Enter/Esc/↑↓←→）、
`tab_bar.rs` 无按键、`tab.rs` 无按键、`gpui-component-0.6.6/src/root.rs:25-30`（Tab/Shift-Tab/Copy）、
`inspector.rs:34,36`。`ctrl-o` / `ctrl-j` 在上游**生产代码里零命中**（`ctrl-o` 唯一一处是**测试**：
`gpui-component-0.6.6/src/command/state.rs:1184`）。`ctrl-w` 上游也只有测试命中（`gpui-pre-0.3.6/src/keymap.rs:524,540`）。

**本仓库自己的绑定（全量 7 条，grep `KeyBinding::new` 的结论）**：

| 键 | action | 落点 |
| --- | --- | --- |
| `ctrl-s` | `SaveBuffer` | `gpui/crates/editor/src/lib.rs:225`（处理在 `gpui/crates/workbench/src/workspace.rs:1922`） |
| `f12` | `NavigateToDefinition` | `gpui/crates/editor/src/lib.rs:226` |
| `ctrl-w` | `CloseActiveTab`（**关闭当前标签，已有能力**） | `gpui/crates/editor/src/lib.rs:229`（处理在 `editor_view.rs:2446-2448`） |
| `ctrl-shift-p` | `OpenCommandPalette` | `gpui/crates/workbench/src/command_palette.rs:282` |
| `ctrl-m` | `ToggleMenuBar`（切换菜单栏形态） | `gpui/crates/workbench/src/command_palette.rs:283` |
| `ctrl-enter` | `CommitChanges` | `gpui/crates/git/src/lib.rs:220` |
| `ctrl-,` | `OpenSettings` | `gpui/crates/settings/src/dialog.rs:93` |

→ **`Ctrl+O`、`Ctrl+J`、`Ctrl+B`、`Ctrl+E`、`Ctrl+G`、`Ctrl+P` 等在本仓库都没有绑定**，所以"按了没反应"是必然的。

#### A5. 真源对照

- **真源单一文件**：`windows/tauri/src/features/window/components/window-menu-bar.tsx`（579 行，`@base-ui/react` 纯自绘；
  `src-tauri` 全部 `.rs` 里 `menu` 零命中 —— 没有原生分支，见 `menu_bar.rs:7-11`）。
  9 个顶级项的行号：`:133,200,277,365,411,427,440,462,504`（`menu_bar.rs:9` 已登记）。
- **"菜单项 → 命令"清单已经有一份可以直接照抄**：`gpui/research/windows/10-menu-bar.md`
  §2.1–§2.9（`:114-259`）= 89 条 + 21 条分隔线的逐条表（中文文案 / i18n 键 / 快捷键 / 分隔线 / 禁用条件 / 证据行号），
  §2.11（`:279-302`）是计数与 `✅/🟡/❌` 分档。**本报告 §2 就是在它之上补"本侧现在到底有没有"这一列。**
- 命令实现与默认键位另有两份真源可查：`windows/tauri/src/features/keymaps/commands/command-registry.ts`（1156 行）、
  `windows/tauri/src/features/keymaps/defaults/default-keymaps.ts`（525 行）。
- 项目下拉的真源：`windows/tauri/src/features/window/components/title-bar/title-project-menu.tsx`（239 行），
  规格逆向已在 `gpui/research/windows/11-project-menu.md`（`project_menu.rs:3` 引用）。

### B. 标签页（文件胶囊）关闭不掉

先分清**两个不同的"标签条"**（维护者的"文件胶囊"只可能是第一个）：

| | 编辑器文件标签（= 文件胶囊，**推断是它**） | 项目标签条 |
| --- | --- | --- |
| 画在哪 | `gpui/crates/editor/src/editor_view.rs:1846-1873` `render_tab_bar` → `:2126-2209` `render_tab` | `gpui/crates/workbench/src/project_tabs.rs:233-392` `project_tabs` |
| 数据 | `EditorPane::buffers: Vec<Buffer>`（`:273`） | `ShellWorkspace::projects`（`workspace.rs:484`） |
| × 关闭 | `:2220-2233` `close_button` → `:1442-1465` `request_close` → `:1413-1432` `close` + `cx.notify()`（`:1454`） | `project_tabs.rs:386-388` → `workspace.rs:1700-1712` |
| 关键事实 | **关闭链是通的**（不是空壳） | **`on_close` 根本不去掉标签**：只把 `active_project` 清成 `len-2`（`workspace.rs:1700-1712`，注释自认"本轮不做"）；而且 `projects.len()==1` 时**整条不渲染**（`workspace.rs:1952`），`projects` 全程只在 `:988` 构造一次、**没有任何 push 第二项的代码** → 正常单项目下**看不到这条**，所以它不是根因 |

| | 内容 |
| --- | --- |
| **现状** | × 只在**活动标签**上渲染（`editor_view.rs:2169` `let close = is_active.then(..)`）；点它 → `request_close`（`:1442`）→ 干净就 `close()`+`cx.notify()`（`:1452-1455`），脏就弹「未保存的更改」对话框（`:1464`，标题键 `:1710`）。右键菜单另有一套 9 项（`:1901-2037`），含「关闭/关闭其他/关闭右侧/全部关闭」。 |
| **缺口 / 根因** | 见 §3（4 条可疑点 + 判定）。一句话：**不是"没接回调"，也不是"回调打到错的 target"**；最可能是"**非活动胶囊上压根没有 ×**"（`:2169` + 悬停档未做 `:2164`）以及"**菜单里没有关闭入口**"（`menu_bar.rs:306-310` 文件菜单只有「保存」）。 |
| **最小改动落点** | ① `editor_view.rs:2169`（× 可见性：至少补 `hover` 档，或按 `tabCloseButtonVisibility` 设置）；② `editor_view.rs:2220-2233`（`close_button` 补 `cx.stop_propagation()` 并订正 `:2216` 的错误注释）；③ 菜单入口：`menu_bar.rs` 文件菜单 + 新 `CommandId::CloseTab/CloseOtherTabs/CloseTabsToRight/CloseTabsToLeft/CloseAllTabs`，执行分支复用 `editor_view.rs:1473` `close_active` / `:1623` `close_scope`。 |

### C. "打开其他文件"没实现

| | 内容 |
| --- | --- |
| **现状** | ① 项目下拉「打开…」是**真禁用态**（`project_menu.rs:399-425` 的 `action_row` + `.disabled(true)`；渲染在 `:611-613`；原因串 `no_folder_dialog` 在 `:367`；打开面板时打 `S1_PROJECT_MENU action=open state=disabled precondition=no_folder_dialog`，`:698-706`）。② 文件菜单里**没有**「打开文件夹」这一项（`menu_bar.rs:306-310` 文件菜单 = `[Save]`）。③ `Ctrl+O` **没有任何绑定**（见 §1.A4 的绑定全量表）。④ **但"选中路径 → 编辑区新开一个 tab"这条链是通的**：`workspace.rs:691-695` 的 `on_open` 闭包 → `editor_view.rs:648-778` `EditorPane::open`（去重 → 读盘 → 建 `EditorState` → `push Buffer` → `active` → `sync_cursor` → `cx.notify` → Java 诊断）。入口是左侧文件树（`Explorer::new(explorer_root, on_open, …)`，`workspace.rs:698-703`）。 |
| **缺口** | **只缺"拿到任意绝对路径"这一步**：一个 OS 文件/文件夹选择对话框调用点，外加把选中的 `PathBuf` 喂给已有的 `on_open`/`EditorPane::open`。 |
| **依赖** | **不需要任何新 crate**（原判断有误，见 §4.3）：`gpui` 自己就有 `prompt_for_paths`，Windows 上走 Win32 `IFileOpenDialog`。 |
| **最小改动落点** | `gpui/crates/workbench/src/workspace.rs`（新 `CommandId::OpenFile`/`OpenFolder` 分支：`cx.prompt_for_paths(..)` → `cx.spawn(..)` await → `self.editor.update(\|pane,cx\| pane.open(&path, window, cx))`）；`gpui/crates/workbench/src/command_palette.rs` 加 id；`gpui/crates/workbench/src/menu_bar.rs` 文件菜单挂「打开文件/打开文件夹」项；`project_menu.rs:399-425` 解除「打开…」的禁用。**"打开文件夹"另需项目生命周期**（换根要重扫树/Git/LSP），属更大件，建议先只做"打开文件"。 |

---

## 2. A 的逐项表：89 条 → 本侧现状三分类

**列含义**
- **文案键/行号** = `gpui/crates/shared/locales/lithe.zh-CN.yml` 里**实际存在**的键（已逐条 grep 确认；`lithe.` 前缀由命名空间给出，`_version: 2` 在 `:9`）。键在 `:6588-6785` 这一段，每条占 2 行（键在 N、`zh-CN:` 在 N+1）。
- **本侧现状** = 三分类之一：
  - **✅ 已实现** = `MENUS` 里有这一项，且执行分支真的改状态（无一条只是打日志）。
  - **🟡 缺项·能力已在** = 菜单里没有这一项，但底层能力/动作已经在仓库里（补菜单 + 补动作即可）。
  - **❌ 缺项·无能力** = 菜单里没有，仓库里也没有可用底层实现。
- **空壳（有 CommandId 但分支是空/todo）：0 条** —— `workspace.rs:1107-1208,1210-1314` 逐分支复核无空实现；`menu_bar.rs:1146-1188` 的单测也禁止死项。
  （`apply_menu_action` 里那 7 条 `unreachable` 分支 `:1197-1205` 是 `if let` 之后的**不可达兜底**，不是空壳。）
- **⚪ 本侧新增** = 这一项真源 89 条里没有，是本侧为补齐能力加的（3 条）。

### 2.1 文件 File（截图 19 条 / 真源 19 条）

| # | 菜单项 | 文案键:行 | 本侧 CommandId / 执行点 | 现状 |
| --- | --- | --- | --- | --- |
| 1 | 新建标签页 | `menu.newTab`:6606 | — | ❌（`editor_view.rs:1844` 自认无 `+` 按钮） |
| 2 | 新建窗口 | `menu.newWindow`:6608 | — | ❌ |
| 3 | 新建文件 | `menu.newFile`:6610 | — | ❌ |
| 4 | 打开文件夹 | `menu.openFolder`:6612 | — | 🟡（对话框通路存在，见 §4；缺项目生命周期） |
| 5 | 关闭文件夹 | `menu.closeFolder`:6614 | — | ❌ |
| 6 | **保存** | `menu.save`:6616 | `CommandId::SaveBuffer`（`command_palette.rs:205`）→ `workspace.rs:1219-1226` → `EditorPane::save_active`（`editor_view.rs:822`） | ✅（`menu_bar.rs:309`） |
| 7 | 另存为... | `menu.saveAs`:6618 | — | ❌ |
| 8 | 全部保存 | `menu.saveAll`:6624 | —（只在未保存对话框里当"批量"按钮文案用，`editor_view.rs:1695`） | ❌ |
| 9 | 还原文件 | `menu.revertFile`:6626 | —（`request_reload` 能力在：`editor_view.rs:1562`） | 🟡 |
| 10 | 显示本地历史 | `menu.showLocalHistory`:6628 | — | ❌ |
| 11 | 关闭标签页 | `menu.closeTab`:6630 | —（能力在：`CloseActiveTab`+`ctrl-w`，`editor/lib.rs:229`；`request_close` `editor_view.rs:1442`） | 🟡 |
| 12 | 关闭窗口 | `menu.closeWindow`:6632 | — | ❌ |
| 13 | 关闭所有标签页 | `menu.closeAllTabs`:6634 | —（`CloseScope::All` 在：`editor_view.rs:222,1623`） | 🟡 |
| 14 | 关闭其他标签页 | `menu.closeOtherTabs`:6636 | —（`CloseScope::Others`：同上） | 🟡 |
| 15 | 关闭已保存标签页 | `menu.closeSavedTabs`:6638 | — | ❌ |
| 16 | 关闭左侧标签页 | `menu.closeTabsToLeft`:6640 | —（**没有** `ToLeft` 分支，`CloseScope` 只有 Others/ToRight/All，`editor_view.rs:216-223`） | ❌ |
| 17 | 关闭右侧标签页 | `menu.closeTabsToRight`:6642 | —（`CloseScope::ToRight` 在：`editor_view.rs:220`） | 🟡 |
| 18 | 重新打开已关闭标签页 | `menu.reopenClosedTab`:6644 | — | ❌ |
| 19 | 退出 | `menu.quit`:6646 | — | ❌ |

小计：**✅ 1 / 🟡 6 / ❌ 12**。

### 2.2 编辑 Edit（19 条）

| # | 菜单项 | 文案键:行 | 本侧 CommandId / 执行点 | 现状 |
| --- | --- | --- | --- | --- |
| 1–2 | 撤销 / 重做 | `menu.undo`:6648 / `menu.redo`:6650 | 上游 `Input` 上下文已绑 `ctrl-z` / `ctrl-y`（`gpui-base …/state.rs:288,290`） | 🟡 |
| 3–5 | 剪切 / 复制 / 粘贴 | `menu.cut`:6652 / `menu.copy`:6654 / `menu.paste`:6656 | 上游 `:254 / :250 / :258` | 🟡 |
| 6 | 全选 | `menu.selectAll`:6658 | 上游 `:246` | 🟡 |
| 7 | 查找 | `menu.find`:6660 | 上游 `ctrl-f` `:298`（组件自带面板；`editor_view.rs:705` 显式 `searchable(true)`） | 🟡 |
| 8 | 查找并替换 | `menu.findAndReplace`:6662 | 上游 `ctrl-h` `:302`（真源是 `mod+alt+f`） | 🟡 |
| 9 | 切换注释 | `menu.toggleComment`:6664 | — | ❌ |
| 10 | **快速修复...** | `menu.quickFix`:6666 | **能力已实现**：`ctrl-.` → `ToggleCodeActions`（`gpui-base …/state.rs:294`）+ provider `gpui/crates/editor/src/code_actions.rs`（接线 `editor_view.rs:720`）；**菜单里没有这一项** | 🟡 |
| 11 | 触发参数提示 | `menu.triggerParameterHints`:6668 | — | ❌ |
| 12 | 显示悬停信息 | `menu.showHover`:6670 | — | ❌ |
| 13–16 | 复制行/删除行/上移行/下移行 | `menu.duplicateLine`:6672 / `deleteLine`:6674 / `moveLineUp`:6676 / `moveLineDown`:6678 | — | ❌ |
| 17–18 | 格式化文档/格式化所选内容 | `menu.formatDocument`:6680 / `formatSelection`:6682 | — | ❌ |
| 19 | **命令面板** | `menu.commandPalette`:6684 | `CommandId::OpenCommandPalette`（`command_palette.rs:207`）→ `workspace.rs:1227-1232`；键位 `command_palette.rs:282` | ✅（`menu_bar.rs:314`） |

小计：**✅ 1 / 🟡 9 / ❌ 9**。
⚠️ **本任务最容易踩的一条**：`menu.quickFix`(6666) 的能力**已经落地**（`git log`：`a9f46b35 feat(gpui): JDT 快速修复（Ctrl+.）接进编辑器`），
所以截图里那一项"已经有一部分是实现过的" —— 但指的是**编辑器内**，**菜单里仍然没有**。

### 2.3 视图 View（18 条）

| # | 菜单项 | 文案键:行 | 本侧 CommandId / 执行点 | 现状 |
| --- | --- | --- | --- | --- |
| 1 | 切换活动侧栏 | `menu.toggleActivitySidebar`:6686 | —（左栏常驻、无收起态） | 🟡 |
| 2 | 切换辅助侧栏 | `menu.toggleSecondarySidebar`:6688 | — | 🟡 |
| 3 | **切换终端** | `menu.toggleTerminal`:6690 | `CommandId::ToggleTerminal`（`command_palette.rs:197`）→ `workspace.rs:1241-1260` | ✅（`menu_bar.rs:320`；**无 `ctrl-j` 绑定**） |
| 4 | 全局搜索 | `menu.globalSearch`:6692 | — | ❌ |
| 5 | 诊断 | `menu.diagnostics`:6694 | —（`BottomPaneKind::Diagnostics` 是占位） | 🟡 |
| 6 | 文件资源管理器 | `menu.fileExplorer`:6696 | —（`Explorer` 已实现，缺"打开该视图"动作） | 🟡 |
| 7 | 源代码管理 | `menu.sourceControl`:6698 | —（`ChangesView` 已实现，缺动作） | 🟡 |
| 8 | GitHub | `menu.github`:6700 | — | ❌ |
| 9 | 运行和调试 | `menu.runAndDebug`:6702 | —（`BottomPaneKind::Run` 占位） | 🟡 |
| 10 | 拆分编辑器 | `menu.splitEditor`:6704 | —（`EditorPane` 是扁平 `buffers`，无 pane 树，`editor_view.rs:1491`） | ❌ |
| 11–14 | 缩略图/自动换行/行号/空白字符 | `toggleMinimap`:6706 / `toggleWordWrap`:6708 / `toggleLineNumbers`:6710 / `toggleRenderWhitespace`:6712 | — | ❌ |
| 15–17 | 放大/缩小/重置缩放 | `zoomIn`:6714 / `zoomOut`:6716 / `resetZoom`:6718 | — | ❌ |
| 18 | **主题**（二级子菜单） | `menu.theme`:6720 | `MenuItem::Theme`（`menu_bar.rs:324`）→ `apply_theme_choice`（`:715-724`），列表现取 `lithe_gpui_settings::theme::theme_names`（`:1035`） | ✅（真源同样无勾选态） |
| ⚪ | 显示状态栏（**真源视图菜单没有**） | `lithe.settings.appearance.showStatusBar`:4456 | `CommandId::ToggleStatusBar`（`command_palette.rs:201`）→ `workspace.rs:1281-1291` | ⚪ 本侧新增（`menu_bar.rs:321`，理由 `:225-229`） |

小计：**✅ 2 / 🟡 6 / ❌ 10**（+1 本侧新增）。

### 2.4 转到 Go（11 条）

| # | 菜单项 | 文案键:行 | 本侧 CommandId / 执行点 | 现状 |
| --- | --- | --- | --- | --- |
| 1 | 快速打开 | `menu.quickOpen`:6722 | — | ❌ |
| 2 | 转到行 | `menu.goToLine`:6724 | —（`status_bar.rs:67` 已登记未实现） | ❌ |
| 3 | 后退 | `menu.goBack`:6726 | —（能力在：`JumpHistory`+`go_back` `editor_view.rs:1154`，标签栏有按钮） | 🟡 |
| 4 | 前进 | `menu.goForward`:6728 | —（同上 `:1165`） | 🟡 |
| 5 | **转到定义** | `menu.goToDefinition`:6730 | `CommandId::NavigateToDefinition`（`command_palette.rs:209`）→ `workspace.rs:1233-1240`；键 `f12`（`editor/lib.rs:226`） | ✅（`menu_bar.rs:330`） |
| 6–8 | 转到实现/类型定义/引用 | `goToImplementation`:6732 / `goToTypeDefinition`:6734 / `goToReferences`:6736 | — | ❌ |
| 9 | 重命名符号 | `menu.renameSymbol`:6738 | — | ❌ |
| 10–11 | 下一个/上一个标签页 | `menu.nextTab`:6740 / `previousTab`:6742 | —（标签栏已实现，缺动作） | 🟡 |

小计：**✅ 1 / 🟡 4 / ❌ 6**。

### 2.5 终端 Terminal（4 条）

| # | 菜单项 | 文案键:行 | 本侧 CommandId / 执行点 | 现状 |
| --- | --- | --- | --- | --- |
| 1 | **新建终端** | `menu.newTerminal`:6744 | `MenuAction::NewTerminalTab`（**不走 CommandId**）→ `workspace.rs:1148-1159` → `TerminalPane::new_tab` | ✅（`menu_bar.rs:336`） |
| 2 | 向右拆分终端 | `menu.splitTerminalRight`:6746 | — | ❌ |
| 3 | 向下拆分终端 | `menu.splitTerminalDown`:6748 | — | ❌ |
| 4 | **关闭终端** | `menu.closeTerminal`:6750 | `MenuAction::CloseTerminalTab` → `workspace.rs:1160-1166` → `TerminalPane::close_active_tab` | ✅（`menu_bar.rs:337`） |

小计：**✅ 2 / 🟡 0 / ❌ 2**。真源的"顶级项按 `terminal` 能力灰化"本侧不做（`menu_bar.rs:54-56` 口径）。

### 2.6 运行 Run（3 条）

| # | 菜单项 | 文案键:行 | 本侧 | 现状 |
| --- | --- | --- | --- | --- |
| 1–3 | 开始调试/停止调试/切换断点 | `startDebugging`:6752 / `stopDebugging`:6754 / `toggleBreakpoint`:6756 | 无 | ❌ ×3 |

小计：**✅ 0 / 🟡 0 / ❌ 3**。⚠️ 本侧 `MENUS` 里"运行"是**空菜单**（`menu_bar.rs:340-344`），
`menu_bar.rs:294-298` 说明"空菜单照画，点开是一张空面板，而不是一片灰"。

### 2.7 工具 Tools（4 条）

| # | 菜单项 | 文案键:行 | 本侧 CommandId / 执行点 | 现状 |
| --- | --- | --- | --- | --- |
| 1 | 数据库 | `menu.databases`:6758 | —（真源默认也是灰的：`backend-capabilities.ts:5`） | ❌ |
| 2 | Web 检查器 | `menu.webInspector`:6760 | — | ❌ |
| 3 | **首选项** | `menu.preferences`:6762 | `MenuAction::Preferences`（不走 CommandId）→ `workspace.rs:1167-1173` → `open_settings_dialog_at(.., General)`；键 `ctrl-,`（`settings/dialog.rs:93`） | ✅（`menu_bar.rs:349`） |
| 4 | 键盘快捷键 | `menu.keyboardShortcuts`:6764 | — | ❌ |
| ⚪ | 外观设置（**真源工具菜单没有**） | `lithe.settings.tabs.appearance`:3656 | `MenuAction::OpenAppearanceSettings` → `CommandId::OpenAppearanceSettings`（`command_palette.rs:191`）→ `workspace.rs:1300-1307` | ⚪ 本侧新增（`menu_bar.rs:354`） |
| ⚪ | Maven（**真源放在右侧栏**） | `lithe.workbench.maven`:3032 | `MenuAction::ToggleMaven` → `CommandId::ToggleMaven` → `workspace.rs:1261-1280` | ⚪ 本侧新增（`menu_bar.rs:361`，理由 `:356-360`） |

小计：**✅ 1 / 🟡 0 / ❌ 3**（+2 本侧新增）。

### 2.8 窗口 Window（4 条）

| # | 菜单项 | 文案键:行 | 本侧 CommandId / 执行点 | 现状 |
| --- | --- | --- | --- | --- |
| 1 | **最小化** | `menu.minimize`:6766 | `MenuAction::Minimize` → `workspace.rs:1174-1180`（`window.minimize_window()`） | ✅（`menu_bar.rs:368`） |
| 2 | **最大化** | `menu.maximize`:6768 | `MenuAction::Maximize` → `:1181-1189`（`window.zoom_window()`） | ✅（`:369`） |
| 3 | 切换菜单栏 | `menu.toggleMenuBar`:6770 | **能力在但不在菜单里**：`CommandId::ToggleMenuBar`（`command_palette.rs:203`）→ `workspace.rs:1311` → `menu_bar::toggle_menu_bar`（`menu_bar.rs:808-814`）；键 `Ctrl+M`（`command_palette.rs:283`） | 🟡 |
| 4 | **切换全屏** | `menu.toggleFullscreen`:6772 | `MenuAction::ToggleFullscreen` → `:1190-1194` | ✅（`:370`） |

小计：**✅ 3 / 🟡 1 / ❌ 0**。

### 2.9 帮助 Help（7 条）

| # | 菜单项 | 文案键:行 | 本侧 | 现状 |
| --- | --- | --- | --- | --- |
| 1–7 | 文档/键盘快捷键/新增功能/更新日志/报告 Bug/请求新功能/检查更新 | `documentation`:6774 / `keyboardShortcuts`:6764（**与工具菜单同一个键**）/ `whatsNew`:6776 / `changelog`:6778 / `reportBug`:6780 / `requestFeature`:6782 / `checkForUpdates`:6784 | 无 | ❌ ×7 |

小计：**✅ 0 / 🟡 0 / ❌ 7**。本侧"帮助"是**空菜单**（`menu_bar.rs:373-377`）。

### 2.10 汇总

| 分类 | 条数 | 明细（按菜单） |
| --- | --- | --- |
| **✅ 已实现**（菜单里有 + 真改状态） | **11** | 文件 1 / 编辑 1 / 视图 2 / 转到 1 / 终端 2 / 运行 0 / 工具 1 / 窗口 3 / 帮助 0 |
| **空壳**（有 CommandId 无实现 / 打日志） | **0** | — |
| **🟡 缺项·能力已在** | **21** | 文件 6 / 编辑 9 / 视图 6 / 转到 4 / 终端 0 / 运行 0 / 工具 0 / 窗口 1 / 帮助 0 |
| **❌ 缺项·无能力** | **57** | 文件 12 / 编辑 9 / 视图 10 / 转到 6 / 终端 2 / 运行 3 / 工具 3 / 窗口 0 / 帮助 7 |
| 合计 | **89** | 与真源 89 条一一对应 |
| **⚪ 本侧新增（真源 89 条里没有）** | **3** | 视图→显示状态栏、工具→外观设置、工具→Maven |

**本侧当前实际画出来的可执行项 = 13 条**（`menu_bar.rs:381-387` 的 `action_count()`；= 上面 11 条 ✅
+ 「外观设置」+「Maven」两条本侧新增里也计入 `MENUS` 的两条；另 1 条新增「显示状态栏」已在 ✅ 计数内）。
13 这个数由构造期诊断 `S1_MENU_BAR mode=… items=9 actions=13` 打出（`menu_bar.rs:415-422`）。

---

## 3. B 的根因

### 3.1 先把"事实"钉死

1. **× 只画在活动标签上**：`editor_view.rs:2160-2178`，判据是 `let close = is_active.then(|| …)`（`:2169`）。
   真源的是三档可见性（`always` / `active`（默认）/ `hover`，`windows/tauri/src/features/settings/lib/ui-preferences.ts:13-19`，
   见 `gpui/research/windows/02-editor-sidebar.md:256,411`）；本侧**只做了 `active` 一档**，
   `hover` 档**自认未做**（`editor_view.rs:2164`，「`TabBar` 不暴露每个标签的悬停状态」，`gpui/crates/editor/src/lib.rs:166-167` 也登记了）。
   → **非活动胶囊上没有 ×，永远点不掉**（只能先点它选中，× 才出现）。
2. **关闭链本身是通的、也是真的**：`close_button`（`:2220-2233`）→ `request_close`（`:1442-1465`）
   → 干净：`close()`（`:1413-1432`，`buffers.remove(index)` + 活动下标顺位）+ `sync_cursor` + `cx.notify()`（`:1453-1454`）；
   脏：`open_unsaved_dialog`（`:1464`，三个按钮 保存/放弃修改/取消，`:1717-1765`）。
   `close()` 是纯状态改动，**没有**打日志的占位分支。
3. **`CommandId` 里没有关闭类变体**：`command_palette.rs:187-210` 全量 11 个变体（OpenSettings / OpenAppearanceSettings /
   SwitchToLightTheme / SwitchToDarkTheme / ToggleTerminal / ToggleMaven / ToggleStatusBar / ToggleMenuBar /
   SaveBuffer / OpenCommandPalette / NavigateToDefinition）。所以问题里假设的"`CommandId::CloseTab` / `CloseOtherTabs` 分支"
   **在仓库里不存在** —— 关闭能力全在 `editor` crate 内部（`editor_view.rs:1442/1473/1623/1665`），
   没有任何 `CommandId` 或菜单入口通到它。**菜单里现在没有任何一条"关闭标签页"**（`menu_bar.rs:306-310` 文件菜单只有「保存」）。
4. **`Button` 不会 `stop_propagation`**（见 §3.2 第 2 条）—— 与代码里的注释相反。

### 3.2 可疑点（逐条给行号）

| # | 可疑点 | 行号 | 判断 |
| --- | --- | --- | --- |
| 1 | **× 只在活动标签上渲染**（`is_active.then(..)`），`hover` 档未做 | `editor_view.rs:2169`（+ `:2160-2164` 自认、`editor/src/lib.rs:166-167` 登记） | **最可能**。用户在非活动胶囊上找 × → 什么都没有 → "点不掉" |
| 2 | 点 × 的 click **会继续冒泡**到标签自身：`Button::on_click` 只在 `loading` 分支 `stop_propagation`（`gpui-component-0.6.6/src/button/button.rs:797-805`），而 `editor_view.rs:2216` 的注释断言"gpui-kit 的 `Button` 点击时会 `stop_propagation`（`button/button.rs:797-808`）"——**该断言与上游源码不符**。冒泡链：`Button` → 内容 div → `Tab::on_click`（`gpui-component-0.6.6/src/tab/tab.rs:871-873`）→ `TabBar::on_click`（`tab_bar.rs:457-459`）→ `EditorPane::activate(渲染时捕获的旧 index)`（`editor_view.rs:1254-1262`） | `editor_view.rs:2216`、`button.rs:797-805`、`tab.rs:871-873`、`tab_bar.rs:457-459`、`editor_view.rs:1254-1262` | **是真 bug 但不是"点不掉"的根因**：gpui 的命中最内层优先（`gpui-pre-0.3.6/src/window.rs:1106-1128` 的 `hit_test` 从 `hitboxes.iter().rev()` 起，`:5752-5787` 的 `dispatch_mouse_event` 用 `.rev()` 冒泡），所以 `request_close` 先跑；随后 `activate(旧index)` 又被 `editor_view.rs:1255-1257` 的 `index >= len` 守卫挡住（`close()` 已经把列表缩短）。净效果：**关闭仍然成功**，但"关闭后活动标签被再设一次"是隐患（`close()` 已经算过邻居，`activate` 会覆盖）。**修 × 的时候应顺手补 `cx.stop_propagation()` 并订正注释。** |
| 3 | **布局只差 2px**：× 按钮 `.small()` = **24×24**（`button.rs:621` `Size::Small => this.size_6()`），放在 `TabVariant::Underline` 的 `inner_content` 里，而该层 `inner_height` = **26**（`tab.rs:66`）且显式 `.overflow_hidden()`（`tab.rs:711`），外层标签也 `.overflow_hidden()`（`tab.rs:802`）；hit-test 会把 hitbox 与 `content_mask` 求交（`window.rs:1110`）。关闭容器自己**没有 id**（`editor_view.rs:2170-2178`）。 | `editor_view.rs:2169-2178`、`tab.rs:66,711,802`、`button.rs:621`、`window.rs:1110` | **推断（未实测）**：DPI 缩放/取整若让 24 变成 >26，或 `content.h_full()`（`editor_view.rs:2206`）被父级 margin 挤小，× 的 hitbox 会被裁 → 点了完全没反应。这是唯一能让"点了没反应"成立的**结构性**原因，建议实机用 `S1_TAB_MENU`（见 §5）先排掉 #1 再验它。 |
| 4 | **菜单里没有关闭入口**：文件菜单只有「保存」（`menu_bar.rs:306-310`），`CommandId` 没有 CloseTab 系（`command_palette.rs:187-210`） | 同左 | 事实。维护者从菜单找不到"关闭标签页"是必然的 |
| 5 | 项目标签条的 `on_close` **真的不去掉**（只清 `active_project`） | `workspace.rs:1700-1712`（注释自认） | 事实，但**不可能是本次根因**：`projects` 全程只有 1 项（`workspace.rs:988`，无任何 push），`projects.len() > 1` 才渲染（`workspace.rs:1952`）→ 正常情况这条标签条**根本不出现** |

### 3.3 判定（最可能的那一条）

> **最可能是 #1 + #4 的组合**：**"文件胶囊"的 × 只在活动标签上**（非活动标签没有任何关闭入口，`editor_view.rs:2169`），
> 而**上半部分的菜单里也没有一条"关闭标签页"**（`menu_bar.rs:306-310` + `command_palette.rs:187-210` 无 CloseTab 系）。
> 所以对用户而言"点击这个胶囊后无法去除"成立：他点的那个胶囊（非活动）没有 ×，菜单里也找不到关闭项。
>
> #2（冒泡 + 注释与上游不符）是**真 bug 但当前不会阻止关闭**；#3（24 vs 26 的裁剪）是**未实测的结构性风险**，
> 只有在 #1 排掉后仍然"点了没反应"时才成为根因。

### 3.4 一条命令 / 一次实机点击的验证办法

关闭链已经内置了可 grep 的证据行，所以"点了有没有反应"是**可判定的**：

- 干净标签的关闭会打 `S1_TAB_MENU run=close tab=<文件名> dirty=false`（**stderr**，`editor_view.rs:1448-1452`）。
- 脏标签则弹对话框（标题 `lithe.unsavedChanges.title`:6884 = 「未保存的更改」），**不打上面那行**。
- 右键胶囊会打 `S1_TAB_MENU opened=true tab=<名> items=9 separators=1`（`editor_view.rs:2034`）。

**步骤**（都要求实机，本轮没做）：
1. 启动 gpui 版 Lithe，在左侧树里点开两个文件 → 观察**两个**胶囊：只有活动那个有 ×？（验证 #1）
2. 鼠标悬停在**非活动**胶囊上 → × 会不会出现？（验证 #1 的 `hover` 档确实没做）
3. 点活动胶囊的 ×，同时看 stderr：
   - 出现 `S1_TAB_MENU run=close tab=… dirty=false` 但**标签还在** → 状态改了没重绘（查 `cx.notify()`，`editor_view.rs:1454`）；
   - **没有**这行 → 点击没到达 handler → 回到 #1（那个胶囊本来就没有 ×）或 #3（hitbox 被裁）。
4. 一个更省事的旁证：`Ctrl+W` 应走同一条 `request_close`（`editor/lib.rs:229` → `editor_view.rs:2446-2448` → `:1473`）。
   **若 `Ctrl+W` 能关掉而点 × 不能**，就锁定在 #3（hit-test/几何），并可直接排除状态与重绘问题。
   ⚠️ 前提：焦点必须在编辑区（处理器挂在 `EditorPane` 根上，`editor_view.rs:2438-2448`）。

### 3.5 关闭一个 tab 的"正确路径"现状（可直接复用，不需要新造）

| 环节 | 归谁 | 落点 |
| --- | --- | --- |
| 持有 buffer | `EditorPane`：`buffers: Vec<Buffer>`（`:273`）、`active: Option<usize>`（`:275`）；`Buffer` 在 `gpui/crates/editor/src/buffer.rs` | — |
| 从 pane 里移除 | `EditorPane::close(index)`（`:1413-1432`） | 移 + 活动下标顺位 + 越界守卫 |
| 脏 buffer 要不要确认 | **要**：`request_close`（`:1442-1465`）先看 `buffer.is_dirty`，脏才弹 | 与真源同口径（`buffer.store.ts:1746` 只看 `isDirty`） |
| **可复用的"放弃修改"对话框** | **已经有，就是它**：`open_unsaved_dialog`（`:1684-1767`）+ `PendingConfirm`（`:261-268`）+ `apply_confirm`（`:1771-1788`）/ `save_confirm`（`:1790-1825`） | 批量档只弹一次（`close_scope`，`:1623-1664`） |
| 相关文案键（**已逐条 grep 确认存在**） | `lithe.unsavedChanges.title`:6884（值「未保存的更改」）、`lithe.editor.gpui.unsavedChangesBody`:8714（「对"{name}"的修改尚未保存。」）、`lithe.editor.gpui.discardChanges`:8712（「放弃修改」）、`lithe.editor.gpui.unsavedChangesBatchBody`:8716（「有 {count} 个文件的修改尚未保存。」）、`lithe.unsavedChanges.doNotSave`:6886、`lithe.menu.saveAll`:6624、`lithe.ui.save`:452、`lithe.ui.cancel`:436 | — |
| 这两个键"什么时候加的、在哪用" | `git log -S` 结论：**`editor.gpui.discardChanges` 与 `editor.gpui.unsavedChangesBody` 加于 `f6728ca7`**（阶段 9「E 关闭确认」）；**`editor.gpui.unsavedChangesBatchBody` 加于 `be1ef509`**（阶段 11 标签右键菜单）。前者用于 `editor_view.rs:1461,1571`（关闭 / 重新加载），按钮上用 `:1738`；后者用于 `:1654` | — |

### 3.6 关闭**最后一个** tab 时现在的行为

- `close()` 在 `buffers` 变空时把 `active` 置 `None`（`editor_view.rs:1422-1424`）。
- `render_body` 拿到 `None` → 走 `render_empty_state`（`:2236-2238`，空态实现 `:2314-2380`：居中图标 + 「选择文件以查看」+「外部工具产生的更改会自动显示。」，文案键 `lithe.workbench.emptyEditorTitle` 见 `:2308-2310`）。
- 标签条本身**仍然占 36px**（`.h_9()`，`:1844-1845`，理由是 `pane-container.tsx:1094-1100` 同款）。
- 所以答案：**编辑器区变空状态，标签条留着**（不是"留一个空编辑器"）。全仓库**没有任何**"没有 buffer 时关窗口"的分支（真源有，本侧明确不复刻：`editor_view.rs:1470-1472`）。

---

## 4. C 的最小可用路径与依赖

### 4.1 现状接线（三条入口分别接到哪）

| 入口 | 落点 | 结果 |
| --- | --- | --- |
| 项目下拉「打开…」 | `project_menu.rs:322-329` `PanelAction::OpenFolder` → `:399-425` `action_row` → `.disabled(true)`（`:424`）；面板装配 `:611-613` | **真禁用**：不挂点击、不进键盘导航、点了不关面板（理由 `:381-398`）。诊断 `S1_PROJECT_MENU action=open state=disabled precondition=no_folder_dialog`（`:698-706`） |
| 文件菜单「打开文件夹」 | `menu_bar.rs:306-310` | **这一项不存在**（文件菜单只有 `MenuAction::Save`） |
| `Ctrl+O` | 全仓库绑定全量表见 §1.A4 | **没有绑定**，按键什么都不发生 |

### 4.2 有没有 OS 文件/文件夹选择对话框的通路？—— **有，而且是现成的**

- **仓库自己的依赖里没有** `rfd` / `native-dialog` / `tinyfiledialogs`（`grep` 全部 `gpui/**/Cargo.toml` 零命中；
  `gpui/Cargo.lock` 里也没有这三个包）。
- **但 gpui 自己有**：
  - API：`App::prompt_for_paths(&self, options: PathPromptOptions) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>>`
    —— `gpui-pre-0.3.6/src/app.rs:1687-1692`（`PathPromptOptions` 定义在 `gpui-pre-0.3.6/src/platform.rs:2483-2494`：
    `files` / `directories` / `multiple` / `prompt` 四个字段）。
  - 通过 `gpui-pre-0.3.6/src/gpui.rs:145` 的 `pub use platform::*;` 出现在 crate 根 → 本仓库写 `gpui_kit::PathPromptOptions`
    （`gpui-kit-0.6.6/src/lib.rs:95` 是 `pub use ::gpui::*;`）。
  - **Windows 实现是真的**（不是 `unimplemented!`）：`gpui-pre-windows-0.3.6/src/platform.rs:673-684` 的 `prompt_for_paths`
    → `:1374-1400` 的 `file_open_dialog`：`CoCreateInstance(&FileOpenDialog)` + `FOS_PICKFOLDERS`（`directories:true` 时）
    + `FOS_ALLOWMULTISELECT`（`multiple:true` 时） + `FOS_FILEMUSTEXIST`，取消返回 `Ok(None)`。
  - 另有一条 `prompt_for_new_path`（`app.rs:1700-1706`，Windows 实现 `platform.rs:686-700` → `file_save_dialog`）可给「另存为」用。
  - Linux 走 `ashpd`（`gpui/Cargo.lock:274` 有 `ashpd`）；**macOS 实现本轮没核对**（§5 未确认项）。
- **结论**：`project_menu.rs:111` 与模块头 `:108-113` 写的"gpui 侧没有任何文件对话框依赖（无 `rfd` / `tinyfiledialogs` /
  `native-dialog`）"**字面对但不完整** —— 它漏掉了 gpui 自带的 `prompt_for_paths`，于是把"能做到"误判成"做不到"。

### 4.3 Windows 侧真源是怎么做的

- 「打开…」：`windows/tauri/src/features/window/components/title-bar/title-project-menu.tsx:180`
  （`closeAndRun(() => void handleOpenFolder())`）→ `features/file-system/stores/file-system.store.ts:832` 的 `handleOpenFolder`
  → `features/file-system/controllers/platform.ts:126-127` 的 `open({ directory: true })`（`@tauri-apps/plugin-dialog`，`:3` import）。
  即：**真源用的就是宿主提供的原生目录选择器**，与 gpui 的 `prompt_for_paths` 同一层能力。
- 「最近项目」：`useRecentFoldersStore.recentFolders`（zustand `persist`，storage key `lithe-code-recent-folders`，
  `features/file-system/stores/recent-folders.store.ts:191-195`；上限 `MAX_RECENT_PROJECTS = 12`、
  先剔除已打开项目、按 `lastOpenedAt` 降序），见 `gpui/research/windows/10-menu-bar.md:320-328`。
- 关闭标签页的真源路径：`tab-context-menu.tsx:185` → `tab-bar.tsx:370-376` → `buffer.store.ts:1348-1365`（`closeBuffer`），
  脏则进 `pendingClose`（`:2064-2113` 的 `confirmCloseWithoutSaving`），见 `gpui/research/windows/09-tab-context-menu.md:107,446`。

### 4.4 最小可用路径：从"选中路径"到"编辑器里出现一个新 tab"

**已经存在的环节（不用动）**：

| 环节 | 落点 | 状态 |
| --- | --- | --- |
| 谁创建 `EditorState` | `EditorPane::open` 里 `cx.new(\|cx\| EditorState::new(window, cx)…)` | ✅ `editor_view.rs:699-729` |
| 谁往 pane 里加一项 | `self.buffers.push(Buffer::new(..))` + `self.active = Some(len-1)` | ✅ `editor_view.rs:765-767` |
| 谁通知重绘 / 刷光标 / 排诊断 | 同函数尾部 `sync_cursor` + `cx.notify` + `schedule_diagnostics` | ✅ `editor_view.rs:768-777` |
| 同路径去重（只切活动标签） | `open` 开头 `position(\|b\| b.path == path)` | ✅ `editor_view.rs:651-654` |
| 外壳侧的"路径 → 编辑区"回调 | `ShellWorkspace::new` 的 `on_open` | ✅ `workspace.rs:691-695`（已被 `Explorer::new` move 走，`:702`） |

**缺的环节（只有 2 个）**：

1. **取路径**：在 `ShellWorkspace` 里调 `cx.prompt_for_paths(PathPromptOptions { files: true, directories: false,
   multiple: true, prompt: None })`，拿 `oneshot::Receiver`。
   - 资产：**零新增依赖**（§4.2）。
   - ⚠️ `prompt_for_paths` 是 `&App` 上的方法，`Context<ShellWorkspace>` 能调；返回的 receiver 要 await。
2. **await 后落盘到编辑区**：用仓库现成的后台/前台任务写法（`workspace.rs:806-828` 的
   `async_cx.spawn(...)` + `background_spawn(...).await` + `async_cx.update(...)`），
   在 `update` 里对每个路径调 `this.editor.update(cx, |pane, cx| pane.open(&path, window, cx))`。
   - 也可以把 `on_open` 改成 `Rc<dyn Fn>` 让外壳自己留一份（现在是 `Box<dyn Fn>`，`workspace.rs:691`，**不可 Clone**，直接调 `self.editor` 更省事）。
   - 打开成功后应打一行诊断（与 `S1_*` 一族同口径），否则无人值守验证没有证据。

**"打开文件夹"为什么不是同一个最小件**：`ShellWorkspace.root` 是 `new()` 的参数（`workspace.rs:490,651`），
换根要连带重建/重扫：`Explorer`(`:698-705`)、`ChangesView`(`:712`)、`BottomPane`(`:773`)、
`editor.prepare_java`(`:688`)、状态栏 Git 分支（`:487-489`）与 JDTLS 会话（`editor_view.rs:435`）。
`workspace.rs:1703-1704` 已经明确登记"关闭项目要销毁项目窗口 / 落盘项目列表，属项目生命周期，本轮不做"。
→ **建议先只做"打开文件（多选）"，"打开文件夹"单独立项。**

### 4.5 「打开的项目 / 最近项目」现在的数据来源

| 段 | 数据来源 | 行号 |
| --- | --- | --- |
| ① 三条动作（新建项目…/打开…/克隆仓库…） | 常量表 `PanelAction::ALL`，三条**全禁用** | `project_menu.rs:322-371`、`:399-425`、`:611-613`；禁用原因串 `:364-370`（`no_scaffolding` / `no_folder_dialog` / `no_git_clone_call_site`） |
| ② 打开的项目 | `ShellWorkspace::project_entries()` —— 由 `self.projects`（**恒 1 项**，`workspace.rs:988`）+ `self.root` 现算；**每条的 path 都是同一个工作区根** | `workspace.rs:1052-1063`；渲染 `project_menu.rs:617-622`；行元素 `:459-520`（当前项 `.disabled(true)`，`:519`） |
| ③ 最近项目 | **恒空态**：`const RECENT_PROJECTS: usize = 0`（`:665`）+ `recent_empty_row()`（`:559-573`，文案键 `lithe.titleProject.noRecentProjects`:3576）。**本侧没有任何最近项目来源 / 持久化层** | `project_menu.rs:659-665`、`:624-627`；诊断 `recent={RECENT_PROJECTS}`（`:670,678-691`） |
| 真源对照 | 「打开的项目」← `useWorkspaceTabsStore.projectTabs`；「最近项目」← `useRecentFoldersStore.recentFolders`（上限 12、固定项优先、剔除已打开、按 `lastOpenedAt` 降序、zustand persist） | `10-menu-bar.md:320-328` |

→ 要让「最近项目」出现行，**要新增一个跨会话的最近列表存储**（本侧设置持久化在 `gpui/crates/settings/src/schema.rs`，
可以加字段；或按真源另起一份）。这是 C 里**唯一需要"新增"的东西**（比对话框重得多）。

---

## 5. 未确认项 + 下一步确认办法

| # | 未确认的事 | 为什么没确认 | 下一步怎么确认 |
| --- | --- | --- | --- |
| 1 | **"文件胶囊"到底指哪个控件** | 截图没进仓库、维护者消息里也没说是哪一张图的哪个像素 | 请维护者在截图上圈一下；或先用 §3.4 的 `S1_TAB_MENU` 证据法，对"编辑器胶囊"和"项目胶囊"各点一次，看哪条诊断不出现 |
| 2 | **B 的 #3（24px 的 × 被 26px 的 `overflow_hidden` 裁掉）** | 纯几何推断，没跑起来量像素 | 启动后截图测 × 的可点区域，或用 `Ctrl+W` vs 点 × 的对照（§3.4 步骤 4）；也可以先用 §1 的 `--` 系列探针确认渲染路径 |
| 3 | **B 的 #2 在真机上是否有可见后果** | 只能读源码推；`activate` 的越界守卫挡住了最坏情形 | 加一行临时诊断（或看现有 `S1_EDITOR_CURSOR` 序列）确认"关闭后活动标签是否被改到别的文件" |
| 4 | **macOS 的 `prompt_for_paths` 实现** | 本轮只核对了 Windows（`gpui-pre-windows-0.3.6/src/platform.rs:673`）与 Linux（`ashpd`，`Cargo.lock:274`） | 查 `gpui-pre-0.3.6` 的 macOS 平台实现（本轮未在注册表里定位到该文件） |
| 5 | **菜单项快捷键该"显示真 keymap"还是"写死展示值"** | 需要维护者口径（真源是每项手写 `shortcut` 字符串，见 `10-menu-bar.md:116-259`；本侧走 keymap 只有 7 条真绑定） | 先做"写死展示值"成本最低（`.suffix(Kbd)`），要真联动再改 `.action(..)` |
| 6 | **「切换菜单栏」要不要进菜单** | 本侧能力在（`Ctrl+M`）但菜单里没有；截图里这一项在 | 维护者拍板；进菜单只是 `menu_bar.rs` 加一条 `MenuAction` |
| 7 | **89 条里"先摆位不实现"具体摆哪些** | 维护者说"没有的可以先弄个位置，不一定先实现"，但本侧现行口径+单测（`menu_bar.rs:1146-1188`、`:46-56`）明确**禁止死项** | 需要一次拍板：要么放宽 `every_listed_action_is_executable`（允许 `.disabled(true)` 的占位），要么维持"补齐=同时补实现" |
| 8 | **「打开…」的禁用态是否要一并解除** | 解除禁用＝承诺"点了有反应"，所以必须先落地对话框通路 | 与 C 的实现同批做，不要先解禁 |

---

## 6. 一句话结论（给实现者）

- **A（菜单栏）**：结构与真源一致、执行点唯一（`menu_bar.rs` 的表 + `workspace.rs::apply_menu_action/run_command_id`），
  现在是 **11 已实现 / 0 空壳 / 78 缺项**；快捷键**一条都没显示**（需要给 item 传 `.action(..)` 或写死 `Kbd`）。
- **B（胶囊关闭）**：关闭链（`editor_view.rs:1442/1413`）是通的，**根因最可能是"× 只画在活动标签上"**（`editor_view.rs:2169`）
  加上**菜单里没有任何关闭入口**（`menu_bar.rs:306-310`）；顺手要修 `editor_view.rs:2216` 那条与上游源码不符的注释
  （`Button` 不会 `stop_propagation`，`button.rs:797-805`）并给 × 补 `cx.stop_propagation()`。
- **C（打开其他文件）**：**不缺依赖** —— gpui 自带 `prompt_for_paths`（`gpui-pre-0.3.6/src/app.rs:1687`，
  Windows 实现 `gpui-pre-windows-0.3.6/src/platform.rs:673,1374-1400`）；缺的只是"取路径 → 喂给已有的
  `EditorPane::open`（`editor_view.rs:648`）"这一步；「打开文件夹」要项目生命周期、「最近项目」要新增持久化。
- **先动哪个文件**：**`gpui/crates/workbench/src/menu_bar.rs`**（A 的菜单数据 + 快捷键显示 + B/C 的菜单入口都在这里），
  紧接着 **`gpui/crates/workbench/src/workspace.rs`**（执行分支）与 **`gpui/crates/editor/src/editor_view.rs`**（B 的真修复点）。
