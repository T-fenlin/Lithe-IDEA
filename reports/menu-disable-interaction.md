# S5 菜单项动态置灰 —— 交互口径复核

> 作者：陆井然（界面设计）｜角色：**评审**｜分支 `feat/gpui-editor-basics`
>
> 上源：`reports/menu-disable-arch.md`（闻本立）、`reports/menu-disable-visual.md`（颜思齐）、主理人六条裁决。
>
> 本文只写复核结论，**未修改任何 Rust 代码**，未启动应用、未跑 `cargo build`、未截图。
>
> 所有结论给 `文件:行号`。凡未在代码里核实的，一律标「未核实」。

---

## 0. 复核范围与当前工作树状态

### 0.1 我读了什么

| 对象 | 用途 |
| --- | --- |
| `rust/lithe-gpui/crates/workbench/src/menu_bar.rs`（全文关键段） | `MenuAction` 枚举、`MENUS`、`is_enabled`、`build_menu`、`icon_element`、`menu_key` |
| `rust/lithe-gpui/crates/editor/src/editor_view.rs` | 上下文取值器与**每个动作的执行侧真实前置条件** |
| `rust/lithe-gpui/crates/editor/src/navigation.rs` | `JumpHistory::can_go_back/forward` |
| `rust/lithe-gpui/crates/editor/src/buffer.rs` | `writable` 的四种成因 |
| `rust/lithe-gpui/crates/workbench/src/workspace.rs` | `apply_menu_action` / `run_command_id` / `toggle_breakpoint_at_caret` |
| `rust/lithe-ui/src/menus/model.rs`、`menu.rs`、`draw.rs`、`typography/inline.rs` | 禁用行的绘制、hover、光标、键盘跳转、键帽取色 |

### 0.2 一个必须先说的前提：工作树是**中间态**

`git status` 显示 `editor_view.rs`、`navigation.rs`、`menu_bar.rs`、`title_bar.rs`、`workspace.rs` 均已 `M`（未提交）。也就是说 **rust-core-dev 已经按架构稿开工了**。本文的所有行号是**我读到的那一刻的行号**，与两份上源文档里的行号已经对不上（架构稿写完时 `MENUS` 在 `:764`，现在是 `menu_bar.rs:862`）。

已落地（截至我读到时）：

| 项 | 位置 | 状态 |
| --- | --- | --- |
| `EditorContext` 七标量 | `menu_bar.rs:333-348` | ✅ 已落地，与终稿一致 |
| `MenuAction::is_enabled` | `menu_bar.rs:534-587` | ✅ 已落地，**有 2 处与架构稿不一致**（§2） |
| 编辑器侧取值器 | `editor_view.rs:1013/1018/1026/1033/1038` | ✅ `tab_count` / `active_tab_index` / `has_active_disk_file` / `can_go_back` / `can_go_forward` **已全部就位**（比架构稿估计的还省：`can_go_*` 是现成的，只需删 `JumpHistory::can_go_back/forward` 上那两处 `#[allow(dead_code)]`，那一步也已经做了，注释也改成了「有了真调用点就不能再靠 allow 包着了」） |
| `build_menu` 传参 + `disabled` | `menu_bar.rs:2007-2040` | ✅ 已落地 |
| `icon_element(icon, disabled)` | `menu_bar.rs:2132-2149` | ✅ 已落地 |
| 外壳 `menu_context()` + `render_bar` 调用点 | `workspace.rs:3479` 仍是 `render_bar(&self.menu_bar, window, cx)`，**未传 `ctx`** | ⚠️ 中间态，当前不编译 |
| 格式 | `menu_bar.rs:2116` 出现 `menu =         menu.item(` 多空格 | ⚠️ `cargo fmt` 会收，顺手 |

> 因为落在中间态，本文对「该灰没灰 / 不该灰灰了」的判断**直接对着已落地的 `is_enabled` 写**，实现同学照 §2 改即可，不需要再回读架构稿。

---

## 1. 逐菜单启用 / 禁用清单

### 1.1 口径总表（70 行 `Action` = 32 行会灰 + 38 行永不灰）

主口径与 `menu_bar.rs:534-587` 的 `is_enabled` 一致；「执行侧证据」列是我逐个去 `editor_view.rs` / `workspace.rs` 核对的**真实前置条件**——这一列是本次复核的主要工作量，也是挑刺的依据。

| 组 | 判定式 | 动作（枚举变体，与 `menu_bar.rs:351-429` 逐字对齐） | 数量 | 执行侧证据 |
| --- | --- | --- | --- | --- |
| **A** | `has_active_buffer` | `CloseTab` `Undo` `Redo` `Cut` `Copy` `Paste` `SelectAll` `Find` `FindAndReplace` `ToggleComment` `DuplicateLine` `DeleteLine` `MoveLineUp` `MoveLineDown` `GoToLine` `QuickFix` `ShowHover` `GoToDefinition` `SaveAs` | **19** | `close_active`（`editor_view.rs:2001-2006`）、`open_find`（`:3004-3008`）、`open_go_to_line`（`:2382-2386`）、`show_hover_at_caret`（`:2533-2537`）、`show_code_actions`（`:2867-2871`）、`navigate_to_definition`（`:1363-1367`）、`save_as`（`:1093-1097`）**全部**以 `let Some(index) = self.active else { … return }` 开头 |
| **B** | `has_active_buffer && active_writable` | `Save` | **1** | `save_active`（`:1069-1088`）：先 `self.active?`，再 `buffer.writable == false` 时**静默 return**（无任何诊断） |
| **C** | `has_active_disk_file`（架构稿）<br>`has_active_disk_file && active_writable`（**当前实现**） | `RevertFile` | **1** | `revert_active`（`:1279-1294`）：只判 `self.active` 与 `is_disk()`，**不看 `writable`** → 见 §2.2 |
| **D** | `has_active_disk_file` | `ToggleBreakpoint` | **1** | `toggle_breakpoint_at_caret`（`workspace.rs:1539-1550`）取 `pane.active_disk_file()`，无则 `skipped/no-file` |
| **E** | `tab_count > 0` | `SaveAll` `CloseAllTabs` `CloseSavedTabs` | **3** | `close_all_tabs` / `close_saved_tabs`（`editor_view.rs:2039-2045`）走 `CloseScope::All / Saved`，**不需要活动缓冲**；`save_all`（`:1208`）同样扫全量 |
| **F** | `tab_count > 1` | `CloseOtherTabs` `NextTab` `PreviousTab` | **3** | `next_tab` / `previous_tab`（`:1735-1760`）是 `(current ± 1) % len`，单标签只能切回自己；`close_other_tabs` 走 `close_scope_at_active`（`:2014-2029`） |
| **G** | 按下标 | `CloseTabsToLeft`（`ix > 0`）<br>`CloseTabsToRight`（`ix + 1 < tab_count`） | **2** | 同 `close_scope_at_active` |
| **H** | `can_go_back` / `can_go_forward` | `GoBack` `GoForward` | **2** | `go_back` / `go_forward`（`:1657-1676`）**额外**需要 `current_entry(cx)`，即 `self.active` → 见 §2.1 |
| **永不灰** | `_ => true` | 其余 38 行（见 §1.3） | **38** | 均为新建 / 打开 / 界面开关 / 全局搜索 / 窗口 / 帮助 |
| **NotWired** | 不调用 `disabled()` | 23 条未接线行 | 0 灰 | `menu_bar.rs:2041-2054` 分支未调 `disabled()`，✅ 守住了裁决 |

**会灰合计 32 行 / 70 行 `Action`；`MENUS` 共 118 行（94 行非分隔 + 24 条分隔符）。**

### 1.2 分菜单逐行结论

图例：🔘 会灰 / ✅ 永不灰 / ⬜ 未接线行（`fg_subtle`，保持可点）

#### File（`menu_bar.rs:862`，24 行：16 动作 + 4 未接线 + 4 分隔）

| 行 | 动作 | 结论 | 口径 |
| --- | --- | --- | --- |
| 1 | `NewTab` | ✅ | 建 scratch，不依赖缓冲（`workspace.rs:1941-1945`） |
| 2 | `NewWindow` | ✅ | |
| 3 | `NewFile` | ✅ | **有一条隐藏前置**：`prompt_new_file`（`workspace.rs:2114-2126`）在无工作区根时只弹通知不建文件 → 见 §2.4 |
| 4 | `OpenFolder` | ✅ | |
| 5 | `OpenFile` | ✅ | |
| — | ⬜ Close Folder | 未接线 | |
| 6 | `Save` | 🔘 **B** | |
| 7 | `SaveAs` | 🔘 **A** | |
| 8 | `SaveAll` | 🔘 **E** | |
| 9 | `RevertFile` | 🔘 **C** | 当前实现多带了 `active_writable` |
| — | ⬜ Show Local History | 未接线 | **夹在两段灰之间**，见 §3.5 |
| 10 | `CloseTab` | 🔘 **A** | |
| — | ⬜ Close Window | 未接线 | |
| 11 | `CloseOtherTabs` | 🔘 **F** | |
| 12 | `CloseAllTabs` | 🔘 **E** | |
| 13 | `CloseSavedTabs` | 🔘 **E** | 1 个脏标签时点击无可见效果 → §2.5 |
| 14 | `CloseTabsToLeft` | 🔘 **G** | |
| 15 | `CloseTabsToRight` | 🔘 **G** | |
| 16 | `ReopenClosedTab` | ✅ | **本批最高价值的漏网之鱼**，见 §2.3 |
| — | ⬜ Quit | 未接线 | |

#### Edit（`menu_bar.rs:910`，23 行：16 动作 + 3 未接线 + 4 分隔）

| 行 | 动作 | 结论 | 备注 |
| --- | --- | --- | --- |
| 1-2 | `Undo` `Redo` | 🔘 **A** | |
| 3-6 | `Cut` `Copy` `Paste` `SelectAll` | 🔘 **A** | 无选区/空剪贴板仍亮（裁决 #6 已定，本批不做） |
| 7-8 | `Find` `FindAndReplace` | 🔘 **A** | **两行都带键位胶囊**（`menu_bar.rs:678-679`） |
| 9-10 | `ToggleComment` `QuickFix` | 🔘 **A** | `QuickFix` 另有一个 100% 失效维度 → §2.6 |
| — | ⬜ Trigger Parameter Hints | 未接线 | |
| 11 | `ShowHover` | 🔘 **A** | |
| 12-15 | `DuplicateLine` `DeleteLine` `MoveLineUp` `MoveLineDown` | 🔘 **A** | 四行**都带键位胶囊** |
| — | ⬜ Format Document / Format Selection | 未接线 | |
| 16 | `CommandPalette` | ✅ | **Edit 在无缓冲时唯一的可用动作** |

#### View（25 行：16 动作 + 2 未接线 + 6 分隔 + 1 主题）

**16 个动作全部永不灰**：`ToggleActivitySidebar` `ToggleSecondarySidebar` `ToggleTerminal` `SearchEverywhere` `ShowDiagnostics` `ShowFileExplorer` `ShowSourceControl` `ShowRunAndDebug` `ToggleMinimap` `ToggleWordWrap` `ToggleLineNumbers` `ToggleRenderWhitespace` `ZoomIn` `ZoomOut` `ResetZoom` `ToggleStatusBar`。

配比确认：`ToggleMinimap` / `ToggleLineNumbers` / `ToggleRenderWhitespace` / `ToggleWordWrap` 改的是 settings store 档位（`workspace.rs:2474-2496`、`:1994-2003`），`ZoomIn/Out/ResetZoom` 改的是 `font_size`（`:2498-2513`），**都是全局设置，与缓冲无关** —— 永不灰正确。

`MenuItem::Theme`（`menu_bar.rs:2067`）沿用「无主题时整行不出现」，不置灰 ✅。

#### Go（15 行：8 动作 + 4 未接线 + 3 分隔）

| 行 | 动作 | 结论 |
| --- | --- | --- |
| 1-2 | `QuickOpen` `ReplaceInFiles` | ✅ |
| 3 | `GoToLine` | 🔘 **A**（带胶囊 `KEY_GO_TO_LINE`） |
| 4-5 | `GoBack` `GoForward` | 🔘 **H**（带胶囊 `KEY_GO_BACK` / `KEY_GO_FORWARD`）→ **当前判定式漏项，见 §2.1** |
| 6 | `GoToDefinition` | 🔘 **A** |
| — | ⬜ Go to Implementation / Type Definition / References / Rename Symbol | 未接线 |
| 7-8 | `NextTab` `PreviousTab` | 🔘 **F**（带胶囊 `ctrl-tab` / `ctrl-shift-tab`） |

#### Terminal（4 行）／Run（3 行）／Tools（9 行）／Window（6 行）／Help（9 行）

| 菜单 | 会灰 | 永不灰 |
| --- | --- | --- |
| Terminal | — | `NewTerminalTab` ✅、`CloseTerminalTab` ✅（**无终端标签时是 no-op + `no_tab` 诊断**，`workspace.rs:1890-1894`；本批无终端标签数 API，维持常亮，见 §2.7） |
| Run | `ToggleBreakpoint` 🔘 **D** | `StartDebugging` ✅、`StopDebugging` ✅（**无会话时两者都是 no-op**，见 §3 IDEA 对照第 9 条） |
| Tools | — | `Preferences` `ShowKeymapSettings` `OpenAppearanceSettings` `ToggleMaven` 全 ✅ |
| Window | — | `Minimize` `Maximize` `ToggleMenuBar` `ToggleFullscreen` 全 ✅ |
| Help | — | `ShowKeymapSettings` ✅（其余 6 条未接线） |

### 1.3 永不置灰的 38 行（要守住，别被后来的 PR 顺手灰掉）

`NewTab` `NewWindow` `NewFile` `OpenFolder` `OpenFile` `ReopenClosedTab`（File 6）·
`CommandPalette`（Edit 1）·
`ToggleActivitySidebar` `ToggleSecondarySidebar` `ToggleTerminal` `SearchEverywhere` `ShowDiagnostics` `ShowFileExplorer` `ShowSourceControl` `ShowRunAndDebug` `ToggleMinimap` `ToggleWordWrap` `ToggleLineNumbers` `ToggleRenderWhitespace` `ZoomIn` `ZoomOut` `ResetZoom` `ToggleStatusBar`（View 16）·
`QuickOpen` `ReplaceInFiles`（Go 2）·
`NewTerminalTab` `CloseTerminalTab`（Terminal 2）·
`StartDebugging` `StopDebugging`（Run 2）·
`Preferences` `ShowKeymapSettings` `OpenAppearanceSettings` `ToggleMaven`（Tools 4）·
`Minimize` `Maximize` `ToggleMenuBar` `ToggleFullscreen`（Window 4）·
`ShowKeymapSettings`（Help 1，与 Tools 共用同一变体）

---

## 2. 挑刺：该灰没灰 / 不该灰灰了

复核结论：**1 处该灰没灰（可稳定复现）、1 处不该灰却灰了、1 处口径写进了错的事实、5 处建议纳入本批、1 处建议登记后续单**。

### 2.1 🔴 该灰没灰：`GoBack` / `GoForward` 漏了 `has_active_buffer`

- **现状**（`menu_bar.rs:581-582`）：

  ```rust
  Self::GoBack => ctx.can_go_back,
  Self::GoForward => ctx.can_go_forward,
  ```

- **执行侧真实前置**（`editor_view.rs:1657-1664`）：

  ```rust
  pub fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
      let Some(present) = self.current_entry(cx) else { return; };   // ← current_entry 取 self.active?
      let Some(entry) = self.history.go_back(present.clone()) else { return; };
      …
  ```

  `current_entry`（`:1698-1706`）第一行就是 `let index = self.active?;`。**没有活动标签页时，`go_back` 一定静默 return。**

- **可复现路径**：① 打开 `A.java`；② ⌘点击跳到 `B.java`（`navigate_to_definition` 在 `editor_view.rs:1522` 调 `history.record`）→ 此时 `entries = [A]`、`current_index = None`；③ 关掉所有标签页 → `history` **不会被清空**（`close()` 只动 `buffers` 与 `closed`，`editor_view.rs:1946-1976`）；④ 打开 Go 菜单：`can_go_back()` = `!entries.is_empty() && current_index.is_none_or(..)` = **true** → **Back 是亮的，点了完全没反应**。

- **修法**：`Self::GoBack => ctx.can_go_back && ctx.has_active_buffer`，`GoForward` 同理。
  与 `EditorContext` 的不变量不冲突（`has_active_disk_file ⇒ has_active_buffer` 是另一条）。

- **影响评级**：中。它不会执行出错误行为，但它是本批唯一一处「**亮着且点了必然无反应**」的行，恰好是这套置灰机制最该消灭的东西。

### 2.2 🟠 不该灰却灰了：`RevertFile` 多带了 `active_writable`

- **现状**（`menu_bar.rs:563`）：`Self::RevertFile => ctx.has_active_disk_file && ctx.active_writable,`
  架构稿（`menu-disable-arch.md:161`）写的是 `Self::RevertFile | Self::ToggleBreakpoint => ctx.has_active_disk_file`，**实现多了一个 `active_writable`**。

- **执行侧**（`editor_view.rs:1279-1294`）：`revert_active` 只判 `self.active` 与 `self.buffers[index].is_disk()`，**完全不看 `writable`**；落点是 `request_reload` → `reload_buffer`（`:2160-2176`），重读一遍即可。

- **被误灰的四种文件**（`writable` 为 `false` 的成因，`buffer.rs:36-71`）：

  | 成因 | `writable` | 正文是什么 | Revert 有没有意义 |
  | --- | --- | --- | --- |
  | 二进制（`:321-337`） | `false` | 「这是二进制文件」说明文案 | 有（重读还是那段说明，但行为成立） |
  | **有损 UTF-8 转码**（`:302-318`） | `false` | **文件正文的近似转写** | **有，且是用户真会用的** |
  | 文件缺失（`:340-348`） | `false` | 读取失败说明 | 有（重试读取） |
  | 超大小（`:351-362`） | `false` | 「超过 N MiB」说明 | 有 |

  有损转码那条最要紧：正文就是文件内容，用户改坏了想「还原」，结果菜单是灰的。

- **建议**：回到架构稿口径 `ctx.has_active_disk_file`。
  如果实现同学的意图是「占位说明型缓冲不给 Revert」，那正确的谓词是一个新的 `active_is_placeholder` 概念，而不是 `active_writable` —— 后者把「写不回去」和「内容不是文件正文」两件事混成了一个标志，这次能碰巧对上 3/4，下次就会错。

### 2.3 🟠 该灰没灰（本批最该补的）：`ReopenClosedTab`

- 现状：永不灰（`is_enabled` 的 `_ => true`）。
- 执行侧（`editor_view.rs:2047-2054`）：`self.closed.pop()` 为空时 `eprintln!("…result=empty")` 后 return —— **静默 no-op**。
- **为什么它比 `can_go_back/forward` 更值得进本批**：它带键位胶囊 `secondary-shift-t`（`menu_bar.rs:667`，⌘⇧T / Ctrl+Shift+T）。在**本批的主场景**（工作区窗口零标签页、刚开项目还没开过文件）里，`closed` 必为空 → 这是一条「亮着 + 有快捷键 + 点了必然无反应」的行，而且它就摆在 File 菜单的关闭组里，用户一眼能看到。相比之下 `GoBack/GoForward` 只在「关掉全部标签页但跳转历史非空」这一个窄状态下才有差别。
- 成本：`EditorPane.closed: Vec<PathBuf>` 已在 `editor_view.rs:250`；加一个 `pub fn has_closed_tabs(&self) -> bool { !self.closed.is_empty() }` 是 3 行，与已落地的 `can_go_back()` 同形状。
- **这是我唯一正式建议扩进本批的项**（见 §7 分歧 4）。

### 2.4 🟡 该灰没灰（低优先，可登记）：`NewFile` 无工作区根

`prompt_new_file`（`workspace.rs:2114-2126`）：`self.root.as_os_str().is_empty()` 时弹「先打开文件夹」通知并 return。所以它有一条「无工作区根」的前置，而 `EditorContext` 里没有根。

建议：**本批不补**。理由是它跟编辑缓冲不是一个维度（要动 `EditorContext` 加字段），而且它失败时**有明确的用户可见通知**（不是静默 no-op），收益低于 §2.3。登记为后续单。

### 2.5 🟡 该灰没灰（低优先，可登记）：`CloseSavedTabs` 全脏时

`save_all` / `close_saved_tabs` 在「没有干净缓冲」时是成功的空操作（`save_all` 的注释 `editor_view.rs:1207` 就写了 "With nothing dirty this is a successful no-op that reports nothing"）。要判需要 `any_clean()`，与 `any_dirty()` 同级别。本批不做，登记。

### 2.6 🟡 该灰没灰（低优先，可登记）：`QuickFix` 的非 Java 维度

`show_code_actions`（`editor_view.rs:2867-2885`）有三道闸：无活动缓冲 → **非 `.java`** → 无 Java 服务。后两道都不在七标量里。也就是说「打开一个 `.md`，Edit ▸ Quick Fix 亮着，点了只有一行 stderr」。

这是**失效概率最高**的一行（非 Java 文件上 100% 失效），但判据要新增 `active_is_java` + `java_service_ready` 两个字段。建议与 §2.3 一起进下一个批，本次登记。

### 2.7 🟡 该灰没灰（低优先，可登记）：`CloseTerminalTab`

`workspace.rs:1890-1894`：无终端标签时 `close_active_tab` 返回 `false`，诊断打 `no_tab`。`TerminalPane` 无公开标签数 API（架构稿 §1.2 已确认）。本批维持常亮，登记。

### 2.8 🔴 口径里写进了一个错的事实：键位胶囊「两行」实际是**十二行**

`menu_bar.rs:2026-2029` 的注释写着：

> `// 禁用行**不画**键位胶囊：今天受影响的是 Find / FindAndReplace 两行。`

**这是错的。** 把 32 行会灰的动作与 `menu_key()`（`menu_bar.rs:665-695`）逐条相交，带胶囊且会灰的是 **12 行**：

| # | 动作 | 键位 | `menu_key()` 行号 |
| --- | --- | --- | --- |
| 1 | `Find` | `KEY_FIND` | `:678` |
| 2 | `FindAndReplace` | `KEY_FIND_REPLACE` | `:679` |
| 3 | `GoToLine` | `KEY_GO_TO_LINE` | `:673` |
| 4 | `GoBack` | `KEY_GO_BACK` | `:674` |
| 5 | `GoForward` | `KEY_GO_FORWARD` | `:675` |
| 6 | `ToggleComment` | `secondary-/` | `:684` |
| 7 | `DuplicateLine` | `secondary-shift-d` | `:685` |
| 8 | `DeleteLine` | `secondary-y` | `:686` |
| 9 | `MoveLineUp` | `secondary-alt-shift-up` | `:687` |
| 10 | `MoveLineDown` | `secondary-alt-shift-down` | `:688` |
| 11 | `NextTab` | `ctrl-tab` | `:691` |
| 12 | `PreviousTab` | `ctrl-shift-tab` | `:692` |

（另外 `ReopenClosedTab` / `ToggleActivitySidebar` / `ToggleTerminal` / `ZoomIn` / `ZoomOut` / `ResetZoom` 六行也有胶囊，但**永不置灰**，不受这条口径影响。所以是 12，不是 2，也不是 14。）

这个数直接改变 §7 分歧 1 的取舍——**12 行里有 Ctrl+Tab、⌘G、⌘[、⌘] 这些导航主路径的键**，抹掉键位信息的损失是原估计的 6 倍。

---

## 3. 与 IntelliJ IDEA / VS Code 对照

对照口径按「我们的做法 → IDEA 实际行为 → 跟不跟」三列给。IDEA 行为里凡我无法在本仓代码或文档中取证的，标「凭使用经验，未核实」。

| # | 我们 | IDEA | VS Code | 建议 |
| --- | --- | --- | --- | --- |
| 1 | **clean 时 Save 不灰**（只看有无活动缓冲） | **一致**：IDEA 的 Save 在文件未修改时也亮着（凭使用经验，未核实源码） | **一致**：VS Code 的 `File: Save` 在 clean 时同样亮 | **跟，且已定**。可用性影响见 §3.1 |
| 2 | 零标签页时 File 的 Save / SaveAs / SaveAll / Revert / Close 全灰 | 一致 | 一致 | ✅ 已跟 |
| 3 | 零标签页时 Edit 的 Undo…Move Line Down 全灰，`CommandPalette` 亮 | 一致 | 一致 | ✅ 已跟 |
| 4 | **clean 时 Undo / Redo 不灰**（不看撤销栈） | **不一致**：IDEA 的 Undo 标签是动态的（"Undo Typing" / "Undo Rename"），撤销栈空时**置灰**（凭使用经验，未核实） | 一致（VS Code 的 Undo 常亮） | **不跟**。撤销栈在 `CodeEditor` 内，无公开 API（架构稿 §1.2）；且 IDEA 靠动态标签弱化了这个差异 |
| 5 | **Paste 不灰**（不看剪贴板） | **不一致**：IDEA 剪贴板为空时 Paste 灰 | **不一致**：VS Code 的 Paste 常亮 | **不跟**。读剪贴板是同步平台调用，不该放在每帧的菜单构建里。与 VS Code 同口径，可接受 |
| 6 | **Cut / Copy 不灰**（不看选区） | **不一致**：无选区时 IDEA 灰掉 | 一致（常亮） | **不跟**，选区在库内，无公开 API；无选区的 Cut/Copy 是合法 no-op |
| 7 | `GoBack` / `GoForward` 按跳转历史 | 一致 | 一致 | ✅ 已跟（**但补上 §2.1 的 `has_active_buffer` 才是完整跟随**） |
| 8 | **`ReopenClosedTab` 常亮** | **不一致**：IDEA 的 `Reopen Closed Tab` 在无已关闭标签时**灰掉**（凭使用经验，未核实） | **不一致**：VS Code 的 `Reopen Closed Editor` 无记录时灰 | **建议跟** → §2.3 |
| 9 | **`StopDebugging` 常亮** | **不一致**：无调试会话时 IDEA 的 Stop 灰 | — | **登记，不跟**。要读 `DebugPane` 会话状态，属另开一条；且 Stop 的 no-op 无副作用 |
| 10 | `ToggleBreakpoint` 按「是不是磁盘文件」 | **基本一致**：IDEA 的行断点可落在任意文本文件的行上；`jdt://` 这种虚拟源不行（凭使用经验，未核实） | — | ✅ 口径接近，不跟 |
| 11 | `NextTab` / `PreviousTab` 按 `tab_count > 1` | IDEA 无此菜单项（走 Ctrl+Tab / 标签栏） | 一致 | ✅ 合理 |
| 12 | `CloseTabsToLeft/Right` 按下标 | IDEA 无 | 一致（VS Code 的 `Close Tabs to the Right` 在活动为最后一个时灰） | ✅ 已跟 |
| 13 | View / Window / Help 全组永不灰 | 一致 | 一致 | ✅ |
| 14 | **顶层 / 父级菜单项永不灰** | 一致（IDEA 顶层基本从不置灰） | 一致 | ✅ 裁决 #5 正确 |
| 15 | **未接线行永不灰，用 `fg_subtle` 且可点** | IDEA 无对应状态（IDEA 没有「未接入」这种行） | 无 | ✅ 本仓自有约定，守住 |
| 16 | **禁用行不画键位胶囊**（当前实现） | **不一致**：IDEA / VS Code 的禁用菜单项**照常显示快捷键**，只是随行一起变淡 | **不一致**：VS Code 的灰行仍显示 `Ctrl+S` | **建议改回颜思齐的首选** → §7 分歧 1 |
| 17 | 菜单灰了但快捷键仍会执行（裁决 #6） | **一致**：IDEA 的菜单禁用与 action 的 `enabled` 其实是同一套 `AnActionEvent` 判定，理论上不会分叉 —— 我们这里**分叉了**（凭使用经验，未核实 IDEA 是否存在分叉场景） | — | 见 §4.4，可接受但要把代价写进文档 |

### 3.1 「clean 时 Save 不灰」这个取舍的可用性影响（主理人点名要确认）

**结论：这个取舍是对的，可以采纳。** 三条理由：

1. **置灰 Save 会制造一个无法解释的死区。** 用户改了文件想保存，看到 Save 是灰的，第一反应是「程序坏了」而不是「我还没改」。dirty 是**用户自己制造的**状态，用户理应知道；把它做成禁用态，等于用一个系统信号去复述用户已知的事实，收益是负的。
2. **置灰会打断肌肉记忆。** ⌘S 是本产品最高频的键。如果菜单里的 Save 会灰，用户会开始怀疑「这次 ⌘S 到底存没存」，而 ⌘S 的实际行为（`save_active` 在 clean 时会照常写一遍）与「灰」传达的「不可用」是矛盾的。**裁决 #6 已经决定键位路径不禁用**，若菜单再按 dirty 灰，矛盾会更明显。
3. **与 IDEA / VS Code 同口径**，用户的迁移成本为零。

**代价（要写进文档）**：用户失去了「一眼看出这个标签页有没有未保存改动」的菜单通道。真正的补偿信号在别处——标签页的 dirty 标记。本批不涉及，但我登记一条后续项：**确认标签页的 dirty 标记在两套主题下都足够明显**（这是「有没有未保存改动」的主通道，菜单不该是）。未核实当前是否有该标记。

---

## 4. 边界态的可用性判断

### 4.1 多标签 → 只剩一个：哪些行一亮一灰

以「3 个磁盘标签、活动在 ix=2、可写、无跳转历史」为起点：

| 动作 | 3 标签（ix=2） | 2 标签（ix=1） | 1 标签（ix=0） | 0 标签 |
| --- | --- | --- | --- | --- |
| `CloseOtherTabs` | ✅ | ✅ | 🔘 灰 | 🔘 灰 |
| `CloseTabsToLeft` | ✅ | ✅ | 🔘 灰 | 🔘 灰 |
| `CloseTabsToRight` | 🔘 灰 | 🔘 灰 | 🔘 灰 | 🔘 灰 |
| `NextTab` / `PreviousTab` | ✅ | ✅ | 🔘 灰 | 🔘 灰 |
| `CloseTab` | ✅ | ✅ | ✅ | 🔘 灰 |
| `CloseAllTabs` / `CloseSavedTabs` / `SaveAll` | ✅ | ✅ | ✅ | 🔘 灰 |
| `Save` / `SaveAs` / `RevertFile` | ✅ | ✅ | ✅ | 🔘 灰 |
| `Undo`…`MoveLineDown`（15 行） | ✅ | ✅ | ✅ | 🔘 灰 |
| `GoToLine` / `GoToDefinition` | ✅ | ✅ | ✅ | 🔘 灰 |
| `ToggleBreakpoint` | ✅ | ✅ | ✅ | 🔘 灰 |
| `GoBack` / `GoForward` | 🔘（无历史） | 🔘 | 🔘 | 🔘 |

- **2 → 1 的那一帧，变灰的是 4 行**：`CloseOtherTabs` / `CloseTabsToLeft` / `NextTab` / `PreviousTab`。（`CloseTabsToRight` 在 ix=1 时就已经是灰的。）
- **1 → 0 的那一帧，新增变灰 25 行**：File 7（Save / SaveAs / SaveAll / RevertFile / CloseTab / CloseAllTabs / CloseSavedTabs）+ Edit 15 + Go 2（GoToLine / GoToDefinition）+ Run 1（ToggleBreakpoint）。跳转历史非空时再加 `GoBack` / `GoForward` 两行 —— **而这两行按 §2.1 是错亮的，不该由这一帧来收尾**。
- **可用性判断**：可以接受。变灰的行全部集中在「关闭组」和「编辑组」两组内部，语义上是成块的，用户读到的是「跟当前文件/当前标签页有关的都不可用了」，而不是零散的几十行随机变灰。这也正是 §1 里按 `has_active_buffer` / `tab_count` 分层的价值。

### 4.2 活动缓冲是未命名 scratch（未落盘）时

| 动作 | 结论 | 理由 |
| --- | --- | --- |
| `Save` | **亮** | `save_active`（`editor_view.rs:1081-1086`）在 `!is_disk()` 时**主动转成 Save As**，注释写着 "A scratch buffer has no path: Save must become Save As, never a silent no-op"。灰掉它等于砍掉「把 scratch 存下来」的唯一入口，是本批最严重的可用性错误之一。当前实现（组 B：`has_active_buffer && active_writable`）正确 |
| `SaveAs` | **亮** | 同上，另存为的目录回落到 `workspace_root`（`:1100-1105`） |
| `RevertFile` | **灰** | `revert_active`（`:1284-1287`）对 `!is_disk()` 打 `reason=untitled` 后 return；且 `has_active_disk_file` 为 false。正确 |
| `ToggleBreakpoint` | **灰** | `toggle_breakpoint_at_caret`（`workspace.rs:1542`）取的是 `active_disk_file()`，scratch 没有磁盘行号可落断点。正确 |
| `SaveAll` | **亮**（`tab_count > 0`） | 与执行侧一致：`save_all` 对脏的未命名缓冲计数并**弹一次通知**（`editor_view.rs:1256-1268`），不是静默。正确 |

**这条边界是本批评判得最漂亮的一处**：scratch 上「Save 亮 / Revert 灰」的不对称，恰好精确对应了执行侧的真实能力（`save_active` 会转另存为，`revert_active` 什么也做不了）。建议把这条写进单测（架构稿 A8 已有，保留）。

### 4.3 只读 / 二进制文件时 Save 灰掉，用户会不会莫名？

**先纠正一处前提**：`active_writable` 覆盖的**不是**「操作系统只读权限」，而是 `buffer.rs:36-71` 里 `read_body` 判定的四种情况——**二进制 / 有损 UTF-8 转码 / 文件缺失 / 超过大小上限**。一个 `chmod 444` 的普通 UTF-8 文件，`writable` 仍然是 `true`，Save 仍然亮着，点了会在 `write_buffer` 失败时弹「保存失败」通知。**所以「只读文件导致 Save 灰掉」这个场景在本产品里并不存在**，-task 里的这个提法需要更正。

真会灰的是这四种，逐一判断：

| 场景 | Save 灰掉用户会不会莫名？ | 判断 |
| --- | --- | --- |
| 二进制文件 | **不会**。缓冲里显示的是「这是二进制文件，无法编辑」的说明文案，编辑器本身就是只读的（`build_editor` 里 `if !writable { state.read_only() }`，`editor_view.rs:796-798`）。用户看得见自己改不了，Save 灰掉是**一致**的 | ✅ 合理 |
| 超过大小上限 | **不会**，同上，正文是「超过 N MiB」的说明 | ✅ 合理 |
| 文件缺失 | **可能有一点**。正文是「读取失败」的说明，用户可能想「再存一次试试」。但 `write_buffer` 对一个读不到的路径写回去本来也不合理 | 🟡 可接受 |
| 有损 UTF-8 转码 | **会，有一点**。正文是文件内容的近似转写，看起来像个能编辑的文本文件；Save 灰掉后用户不知道为什么 | 🟡 可接受，但**缺解释** |

**共同的缺口**：四种情况下 Save 都是灰的，但**没有任何一处告诉用户为什么**。缓冲区里的说明文案（二进制/超限/缺失）解释了「为什么不能编辑」，有损转码那种则连说明都没有。

**建议（低成本，本批可做可不做）**：不用加 tooltip（菜单行没有 tooltip 通道），把解释责任交给已有机制——`save_active` 在 `writable == false` 时是**静默 return**（`editor_view.rs:1074-1080` 连 `eprintln!` 都没有，与上面 `:1071` 的 `no-active-buffer` 分支不对称）。建议补一行诊断：

```
S1_EDITOR_SAVE active=false reason=not-writable name={name}
```

这不是视觉改动，是让「灰掉」这件事在日志里留下可追溯的原因，与裁决 #7 的「no-op + 诊断」同口径。同理 `RevertFile` 若按 §2.2 改回口径，也可顺手确认其诊断齐全。

### 4.4 菜单开着的时候状态变了：会不会视觉突变

**会发生，且机制上是必然的**：

1. 面板只在展开时现建一次（`menu_bar.rs:1941-1945`：`if open { build_menu(index, ctx, cx) }`）；外壳已经 observe 了编辑窗格（`workspace.rs:1054-1057`：`cx.observe(&editor, … cx.notify())`）。
2. 于是：编辑窗格一变 → 外壳重画 → `build_menu` 重跑 → `is_enabled` 重算 → 面板当场重建。
3. `ctrl-tab` / `ctrl-shift-tab` / `secondary-s` / `secondary-w` / `KEY_GO_BACK` / `KEY_GO_FORWARD` / `KEY_GO_TO_LINE` / `KEY_FIND` / `KEY_FIND_REPLACE` 都是**全窗绑定**（`menu_bar.rs:258-297`，`install_menu_key` 的 `KeyBinding::new(key, action, None)` 上下文为 `None`）。菜单面板的键盘处理只吃 ↑↓←→ / Enter / Space / Esc，**不消费这些键**。

所以：**Go 菜单开着时按 Ctrl+Tab，Next Tab / Previous Tab / Close Tabs to Left / Close Tabs to Right 会在你眼前一亮一灰。**

| 现象 | 可接受吗 | 说明 |
| --- | --- | --- |
| 灰亮当场翻转 | ✅ 可接受，而且**是期望行为**——这正是「动态置灰」的意义，比「关掉菜单重开才更新」好得多 | 架构稿 §3.2 判断正确 |
| 高亮行刚好落在变灰的那一行上 | ✅ 可接受。`settle()`（`lithe-ui/src/menus/menu.rs:37-40`）会清掉落在禁用行上的层级标记 → 高亮消失；此时按 ↓ 走 `menu.step(None, 1)` 落到第一行可用项。这正是我们想要的「不可用就不该被指着」 | 与架构稿 R6 判断一致 |
| 面板宽度抖动 | 🟠 **需要注意**。若按 §7 分歧 1 的建议把键位胶囊画回来，就不会抖；若维持「不画」，灰行的胶囊会消失，行内容长度变化 → 面板是定宽的（`build_menu` 的 `Menu::new().width(...)`, `menu_bar.rs:2013`），所以**面板宽度不会变**，只是行内容短了一截。可接受，但验证了「不画胶囊」会让行信息量在可用/不可用之间跳变 | 见 §7 分歧 1 第 ② 条 |
| 一帧的陈旧状态 | ✅ 可接受。快照在同一帧的 `render` 里现取，最坏延迟一帧（16ms） | 架构稿 R7 |

**另外一条必须写进文档的代价**（裁决 #6 的连带后果）：由于键位路径不禁用，出现了 **9 行「菜单灰、按键有效」**：

`Save`(⌘S) · `CloseTab`(⌘W) · `GoToLine` · `GoBack` · `GoForward` · `Find`(⌘F) · `FindAndReplace`(⌘R) · `NextTab`(Ctrl+Tab) · `PreviousTab`(Ctrl+Shift+Tab)

（绑定来源：`editor/src/lib.rs:41/43` 的 `secondary-s` / `secondary-w`、`menu_bar.rs:280-297`。`Undo`/`Redo`/`Cut`/`Copy`/`Paste`/`SelectAll`/行编辑五个绑在 `ElyEditor` 上下文里，无缓冲时焦点不在编辑器上，不会触发 —— 这 11 行天然一致，不受影响。）

**我的判断：可接受，因为「有效」的实际表现是静默 no-op，而不是执行出错误行为。** 逐个确认：

| 键 | 无缓冲 / 单标签时按下会怎样 | 用户看得见吗 |
| --- | --- | --- |
| ⌘S | `save_active` → `S1_EDITOR_SAVE active=false reason=no-active-buffer`（`editor_view.rs:1071`） | 看不见 |
| ⌘W | `close_active` → `self.active?` return（`:2001-2003`） | 看不见 |
| ⌘F | `open_find` → `S1_EDITOR_FIND open=false reason=no-active-buffer`（`:3006`） | 看不见 |
| Ctrl+Tab（1 标签） | `next_tab` → `(0+1)%1 = 0`，切回自己（`:1735-1744`） | 看不见 |
| ⌘[ / ⌘]（空历史） | `go_back` → `history.go_back()` 返回 `None`（`:1639-1641`） | 看不见 |

也就是说「菜单说不可用」与「按键什么也没发生」在**结果上是一致的**，用户不会撞见矛盾。这个代价值得承担（禁用键位要动 `push_run` 与 `keymap_engine`，架构稿 §9 已论证）。**但请把上面这张表写进实现注释**，否则下一个人会以为这是个 bug。

---

## 5. 可观测验收断言（15 条，人能看能点）

**前置**：工作区窗口（**不是**欢迎页 —— 欢迎页无菜单栏），未打开任何标签页。两套主题各过一遍。

| # | 断言 |
| --- | --- |
| **V1** | 零标签页打开 **File**：`Save` `SaveAs` `SaveAll` `RevertFile` `CloseTab` `CloseOtherTabs` `CloseAllTabs` `CloseSavedTabs` `CloseTabsToLeft` `CloseTabsToRight` **十行灰**；`NewTab` `NewWindow` `NewFile` `OpenFolder` `OpenFile` `ReopenClosedTab` **六行亮**。鼠标划过任一灰行：无 hover 底、光标是默认箭头；点击灰行：菜单不关、终端无新增 `S1_MENU_RUN` |
| **V2** | 同状态打开 **Edit**：`Undo` `Redo` `Cut` `Copy` `Paste` `SelectAll` `Find` `FindAndReplace` `ToggleComment` `DuplicateLine` `DeleteLine` `MoveLineUp` `MoveLineDown` `QuickFix` `ShowHover` **十五行灰**；只有 `CommandPalette` 亮 |
| **V3** | 同状态打开 **Go**：`GoToLine` `GoBack` `GoForward` `GoToDefinition` `NextTab` `PreviousTab` **六行灰**；`QuickOpen` `ReplaceInFiles` 亮。同状态打开 **Run**：只有 `ToggleBreakpoint` 灰，`Start / Stop Debugging` 亮 |
| **V4** | 同状态打开 **View / Window / Help / Tools / Terminal**：**一行都不灰**（View 的 `MenuItem::Theme` 在有主题时出现且不灰） |
| **V5** | 只打开一个磁盘文件、**不做任何修改**（clean）：`Save` **亮**（验证 IDEA 口径）、`SaveAs` 亮、`RevertFile` 亮、`CloseTab` 亮、`SaveAll` / `CloseAllTabs` / `CloseSavedTabs` 亮；`CloseOtherTabs` / `NextTab` / `PreviousTab` / `CloseTabsToLeft` / `CloseTabsToRight` **五行灰** |
| **V6** | 只 `File ▸ New Tab`（未命名 scratch）：`Save` **亮**（点击会转成另存为对话框）、`SaveAs` 亮、`CloseTab` 亮；`RevertFile` **灰**、`ToggleBreakpoint` **灰** |
| **V7** | 打开一个二进制文件（如 `.bin`）：`Save` 灰，且缓冲正文是只读的说明文案、光标无法输入。打开一个含非法 UTF-8 字节的 `.txt`：`Save` 灰；**并确认 `RevertFile` 的当前表现与 §2.2 的结论一致**（当前实现是灰，若按建议改回应为亮） |
| **V8** | Java 项目里 ⌘点击进入 `jdt://` 虚拟源：`Save` / `RevertFile` / `ToggleBreakpoint` **三行全灰**；返回普通 `.java` 文件后三行全亮 |
| **V9** | 开 3 个标签，活动在中间（ix=1）：`CloseTabsToLeft` 亮、`CloseTabsToRight` 亮、`CloseOtherTabs` 亮；切到第一个（ix=0）后 `CloseTabsToLeft` 灰、`CloseTabsToRight` 亮；切到最后一个（ix=2）后反过来 |
| **V10** | 开 2 个标签，关掉一个：**同一帧内** `NextTab` / `PreviousTab` / `CloseOtherTabs` / `CloseTabsToLeft` 四行变灰（菜单若正开着，肉眼应看到当场变灰，无需关掉重开） |
| **V11** | 键盘可达性：零标签页打开 **Edit**，按 ↓ 从顶走到底，落点**只有 4 站**（`CommandPalette` + 3 条未接线行），不会停在任一灰行上；回车只在这 4 站之一生效。对 Go / File / Run 重复同样检查，落点数见 §6 |
| **V12** | 键位胶囊：零标签页打开 Edit，`Find` / `FindAndReplace` 两行**看不到** ⌘F / ⌘R 胶囊；打开 Go，`GoToLine` / `GoBack` / `GoForward` / `NextTab` / `PreviousTab` 五行也看不到。**共 12 行受此口径影响**（§2.8），请逐行走一遍确认，不要只验 Edit |
| **V13** | 灰行的图标与文字**同为一档灰**：File ▸ Save 的软盘图标与 "Save" 文字颜色一致，不出现「灰字 + 深色图标」；同时确认 `SaveAs` / `SaveAll`（`MenuIcon::Empty`，`menu_bar.rs:637`）不留空槽导致行跳动 |
| **V14** | 「禁用」与「未接线」在**同一面板相邻**时能一眼分出两档：File 里 `Save` 组之下紧邻的 `Show Local History`、Edit 里 `ToggleComment`/`QuickFix` 之间的 `Trigger Parameter Hints` —— 未接线行明显更清晰且**可点**（点了弹「尚未接入…」），禁用行明显更淡且**不可点** |
| **V15** | compact 模式（汉堡按钮，`menu_bar.rs:1798`）里九个顶层项**全亮**；随便点进一个，内部灰亮与横排模式完全一致。另：守卫数字不变 —— `action_count()=70`、`not_wired_count()=23`、`SOURCE_MENU_ITEMS=89`、`LOCAL_ONLY_ITEMS=5`、`MENUS` 行数 118（含 24 条分隔符，其中 94 条非分隔行） |

**补充（不占编号，建议顺手记）**：⌘S 在零标签页时仍是**静默 no-op** —— 终端出 `S1_EDITOR_SAVE active=false reason=no-active-buffer`，界面无任何弹窗；Ctrl+Tab 在单标签时同样无任何可见反应。

---

## 6. 无障碍 / 键盘：死局排查与落点统计

### 6.1 会不会出现「某菜单内无一可选项」

库的保证：`Menu::step`（`lithe-ui/src/menus/model.rs:192-203`）只停在 `!item.disabled` 的行上，且 `item_at()` 对 `Separator` 返回 `None`（分隔条自动跳过）；`choose()`（`menu.rs:227-229`）对禁用行直接 `return`。`step` 每轮循环 `count` 次，一个可用项都没有时返回 `None`。

**逐个顶层菜单统计「零标签页 + 跳转历史为空」这一最坏状态下的键盘落点数**（落点 = 非分隔且未禁用的行 = 亮的 `Action` + 全部未接线行 + 主题行）：

| 顶层菜单 | 非分隔行 | 会灰 | **键盘落点** | 落点构成 |
| --- | --- | --- | --- | --- |
| File | 20 | 10 | **10** | 6 亮动作 + 4 未接线 |
| Edit | 19 | 15 | **4** ⚠️ 最少 | 1（`CommandPalette`）+ 3 未接线 |
| View | 19 | 0 | **19** | 16 亮 + 2 未接线 + 1 主题 |
| Go | 12 | 6 | **6** | 2（`QuickOpen` / `ReplaceInFiles`）+ 4 未接线 |
| Terminal | 4 | 0 | **4** | 2 亮 + 2 未接线 |
| **Run** | 3 | 1 | **2** ⚠️ 次少 | `StartDebugging` / `StopDebugging` |
| Tools | 6 | 0 | **6** | 4 亮 + 2 未接线 |
| Window | 4 | 0 | **4** | 4 亮 |
| Help | 7 | 0 | **7** | 1 亮 + 6 未接线 |
| **合计** | **94** | **32** | **62** | |

**结论：无死局。** 最少的 Run 仍有 2 个落点，Edit 有 4 个。**没有任何一个顶层菜单会落到"一个可选项都没有"。**

### 6.2 但要记住 Edit 是薄的那一个

Edit 的 19 行里 15 行会灰，只剩 4 个落点，其中 3 个还是「点了会弹『尚未接入』」的未接线行。也就是说 **无缓冲时 Edit 菜单实质上只剩 `CommandPalette` 一条有实际功能**。这是可接受的（IDEA / VS Code 同口径，`CommandPalette` 恰恰是无缓冲时最该给用户的出口），但请在视觉走查时确认：**Edit 面板打开时，用户第一眼能看到那 3 条未接线行与 15 条灰行的区别**，而不是看成"一整面板都是灰的"。这与 V14 是同一件事，Edit 是最严的位置。

### 6.3 已知的库侧无障碍缺口（不在本批）

- 菜单行整体没有 aria（`lithe-ui/src/menus/draw.rs` 的 `row()` 无 `.role()` / `.aria_*`），置灰行对读屏**没有「不可用」信号**。颜思齐已在 `menu-disable-visual.md:§3.9` 登记为后续单，**我同意不阻塞本批**：置灰的四个信号里有三个是非颜色的（无 hover / 无 pointer / 键盘跳过 / 点击无反应），符合 WCAG 1.4.1「信息不靠颜色单独传达」。
- 顶层触发器有 `.aria_label(MENUS[index].label())`（`menu_bar.rs:1985`），compact 按钮也有（`menu_bar.rs:1775`），这部分无障碍是有的。

### 6.4 一处与本批相邻的既有缺陷（登记，不在本批改）

Edit 菜单里 11 个动作（`Undo`…`MoveLineDown`）走 `dispatch_editor_action`（`workspace.rs:2088-2105`），它依赖 thread-local `ACTION_TARGET`（`menu_bar.rs:1487-1493`）——而这个槽**只在顶层触发器的 `on_mouse_down` 里更新**（`menu_bar.rs:1803-1808`）。两条后果：

1. 用**键盘**打开 Edit 菜单（不经过鼠标点触发器）时，`ACTION_TARGET` 可能是陈旧值或 `None` → `dispatch_editor_action` 打 `no_target` 后 return → **Edit ▸ Undo 亮着但点了没反应**。
2. 鼠标点击触发器前焦点在终端，则 `ACTION_TARGET` 是终端的 handle → `target.focus()` 后把 Undo 派发进终端 → 同样没反应。

这是**既有缺陷**（置灰之前就存在），本批既没修也没恶化。但它意味着：**置灰上线后，「不灰」会被用户理解为「能点」，而这 11 行还要再加一个"焦点得在编辑器里"的隐性条件**。建议在 `is_enabled` 的文档注释里写一句，并登记一条后续单：给 `EditorContext` 加一个 `editor_focused` 标量（由外壳在 `menu_context()` 里从 `action_target()` 取），只用于这 11 行。**未核实** gpui 的 `window.focused(cx)` 在菜单 Takeover 期间的具体行为，落地前需实测。

---

## 7. 对主理人六条裁决的复核意见

### 裁决 #1（静态表 + `is_enabled` 现算）—— ✅ 同意，已落地

`MENUS` 一行未动（`action_count()=70`、`not_wired_count()=23`、`SOURCE_MENU_ITEMS=89` 三个守卫数字不变）；`is_enabled`（`menu_bar.rs:534`）与 `label_key()` / `icon()` / `menu_key()` / `keybind_action()` / `command_id()` 同处一个 `impl`；`build_menu`（`:2007`）建行时现算。行下标稳定（库的 `Open.levels` 存的是下标），方案 (b) 的下标漂移问题被规避。**无异议。**

### 裁决 #2（场景纠正：工作区窗口零标签页，不是欢迎页）—— ✅ 同意

架构稿 §1.1 的纠正成立。我的 §1、§5、§6 全部按「工作区窗口零标签页 / 刚开项目还没开文件」写。**无异议。**

### 裁决 #3（键位胶囊：禁用行不画，不动 lithe-ui）—— 🔴 **不同意，建议改回颜思齐的首选**

三条理由，逐条给证据：

**① 事实层面：这不是「颜思齐口径」，而是她明确否掉的那个回退方案。**

`menu-disable-visual.md:167` 的定稿口径是：

> **结论（与架构师对齐后定稿，覆盖初稿的「禁用行不画胶囊」口径）：禁用行照旧画键位胶囊，但键帽文字与「+」连字符一起降到 `colors.fg_disabled`**

而「不画胶囊」出现在 `:192`，被她标为**回退方案**，原文：

> **回退方案（若实现期发现开关不好加）：禁用行不设 `keys`** … **但不要把它当成首选**：那会让禁用行与启用行的宽度与信息量不一致。

主理人核验的「`cap()` 在 `inline.rs:55` 硬取 `fg_muted`、父级 `text_color` 传不进键帽」**完全成立**——我逐行复核过：`inline.rs:41-57` 的 `cap()` 在 `:55` 硬写 `.text_color(theme.colors.fg_muted)`，`:52` 硬取 `surface`、`:51` 硬取 `border_strong`；`KbdCombo` 的 `subtle` 在 `:107` 取值、`:120` 使用（`div().text_color(subtle).child("+")`）。**主理人写的 `:107` 与颜思齐写的 `:120` 是同一件事的两行，两者都对，不构成分歧。**

但这段证据**证明的是「改 `draw.rs:96` 一行无效」，而这恰恰是颜思齐提出「改 `typography/inline.rs`」这条首选路径的理由**，不是「不画胶囊」的理由。

**② 代价层面：受影响的是 12 行，不是 2 行。**

见 §2.8 的完整清单。里面包含 `Ctrl+Tab`（Next/Previous Tab）、`⌘G`（Go to Line）、`⌘[` `⌘]`（Back/Forward）—— 这些都是**导航主路径的键**。抹掉键位信息的损失是原估计的 6 倍。而且当前实现把这个错误数字写进了代码注释（`menu_bar.rs:2026-2029`：「今天受影响的是 Find / FindAndReplace 两行」），下一个人会照着错的数估影响面。

**③ 收益层面：不画胶囊会引入新的不一致。**

灰行的行内容短一截、行信息量在「可用 / 不可用」之间跳变；鼠标在菜单里上下移动时，键帽会忽有忽无。这一点**颜思齐自己在 `:192` 已经写明了**（"那会让禁用行与启用行的宽度与信息量不一致"）。

**建议：回到颜思齐的首选** —— `typography/inline.rs` 给 `KbdCombo` 加一个默认关闭的 `.disabled(bool)`，键帽文字（`:55`）与「+」（`:107`/`:120`）取 `colors.fg_disabled`，键帽底与描边不动；`menus/draw.rs:91-99` 传 `item.disabled`。她已核实 `KbdCombo` 全仓 7 处调用，**新增开关默认关闭 = 其余 6 处零变化**。

**退一步的底线**：若主理人坚持不动 `lithe-ui`，**至少把 `menu_bar.rs:2026-2029` 注释里的「两行」改成「十二行」**，并补一句「这是回退方案，首选是 `typography/inline.rs` 的 `.disabled(bool)`，见 `reports/menu-disable-visual.md:167-190`」。错的注释比错的实现更贵。

**顺带纠正主理人对颜思齐的一处指责**：主理人说「带键位的菜单行不止 Find 一个，`FindAndReplace` 也有，你按两行走」——方向对，但**量级不对，是十二行**。而且颜思齐 §6 那张表的标题是「S5 八个目标行的对照」，她那句「八个目标动作里只有 `Find` 有 `keys`」在**她自己限定的范围内是准确的**，不算事实错误；真正的错是把这个「八个目标行」的范围当成了「全部会灰的行」。

### 裁决 #4（自画图标按状态选色，不动库）—— ✅ 同意，已落地且正确

`icon_element(icon, disabled)`（`menu_bar.rs:2132-2149`）按状态取 `fg_disabled` / `fg`；**非禁用分支显式给 `fg`**，与 `Icon::render` 的回落值（`primitives/icon.rs:506`）一致，视觉零变化。三处调用都跟上了：`build_menu` 的启用行（`:2024`）、未接线行传 `false`（`:2052`）、主题子菜单传 `false`（`:2119`）。**无异议。**

### 裁决 #5（子菜单父项不置灰）—— ✅ 同意，且比裁决描述得更关键

`settle()`（`lithe-ui/src/menus/menu.rs:37-40`）会清掉落在禁用行上的层级标记；compact 面板九个顶层项**全部**是 `MenuRow::submenu`（`menu_bar.rs:1801`）——父项一灰，汉堡按钮整个失效。`is_enabled` 也确实没有覆盖任何子菜单行。**无异议。**

### 裁决 #6（范围：不碰快捷键 / 命令面板 / dirty / 选区 / 剪贴板 / 撤销栈）—— ✅ 同意，但要把代价写进文档

- **不碰快捷键**：同意。代价是 §4.4 枚举的 9 行「菜单灰、按键有效」；我已逐个确认这 9 行的按键表现都是**静默 no-op**，结果上与「不可用」一致，不会让用户撞见矛盾。**请把 §4.4 那张表写进实现注释。**
- **不碰命令面板**：同意。命令面板的语义是裁剪行（`ActionFlags`），与置灰是两套真源，混起来会打架。
- **不做 dirty / 选区 / 剪贴板 / 撤销栈**：同意。§3 对照表第 1/4/5/6 条给出了各自的取舍依据。

**但我要正式提一条范围异议**：

**分歧 4 —— 建议把 `ReopenClosedTab` 提进本批。**

主理人把 `can_go_back/forward` 从 P1 提到了本批，却把 `ReopenClosedTab` 留在 P1。按我 §2.3 的测算，二者的性价比是反的：

| | `GoBack`/`GoForward` | `ReopenClosedTab` |
| --- | --- | --- |
| 在本批主场景（零标签页、刚开项目）下是否失效 | **否**（历史为空 → `can_go_*` 已经是 false，本来就是灰的） | **是**（`closed` 为空 → 亮着且点了必然无反应） |
| 有键位胶囊 | 有（`⌘[` `⌘]`） | 有（`⌘⇧T`，`menu_bar.rs:667`） |
| 新增访问器成本 | **0**（`editor_view.rs:1033/1038` 已存在） | 3 行 `has_closed_tabs()`（`editor_view.rs:250` 的 `closed` 已就位） |
| 只在什么状态下才显出差别 | 关掉全部标签页但跳转历史非空（窄） | **刚开项目还没关过任何文件**（这正是本批的主场景） |

也就是说：`can_go_back/forward` 的收益主要落在「让 Back/Forward 在**有**历史时亮起来」这个**常态正确性**上，而 `ReopenClosedTab` 的收益落在「消除本批主场景里唯一一条亮着却必然失效的行」上。两者都值得做，但如果本批只能再加一条，我选 `ReopenClosedTab`。

**请主理人重判。** 若坚持不放，我建议**至少**把它作为已知缺口登记进 `menu_bar.rs` 的 `is_enabled` 文档注释（那里已经写了「拿不到低成本判据的（已关闭标签页栈、终端标签页数）也暂时维持常亮」—— 但**已关闭标签页栈的判据成本并不高**，这句注释把它和「终端标签页数，真的没有 API」混为一类了，建议分开写）。

---

## 8. 实现侧待办清单（按优先级，供 rust-core-dev 直接使用）

| # | 项 | 位置 | 级别 |
| --- | --- | --- | --- |
| 1 | `GoBack` / `GoForward` 加 `&& ctx.has_active_buffer` | `menu_bar.rs:581-582` | 🔴 必改 |
| 2 | `RevertFile` 去掉 `&& ctx.active_writable` | `menu_bar.rs:563` | 🟠 建议改 |
| 3 | 键位胶囊：改回「照画 + 键帽降档」，或**至少**把注释的「两行」改成「十二行」 | `menu_bar.rs:2026-2034`（+ `lithe-ui/src/typography/inline.rs`） | 🟠 建议改 |
| 4 | `save_active` 在 `writable == false` 时补一行诊断（与 `:1071` 对称） | `editor_view.rs:1074-1080` | 🟡 低成本 |
| 5 | `ReopenClosedTab` 纳入本批（待主理人裁决） | `editor_view.rs` + `menu_bar.rs:534-587` | 🟠 待裁 |
| 6 | `workspace.rs:3479` 的 `render_bar` 调用点补 `ctx` + 新增 `menu_context()` | `workspace.rs:3479` | 🔴 中间态未完 |
| 7 | `cargo fmt`（`menu_bar.rs:2116` 有多余空格） | — | 🟡 |
| 8 | 注释里把「已关闭标签页栈」与「终端标签页数」分开写（前者低成本，后者真无 API） | `menu_bar.rs:534-587` 的 `is_enabled` 文档 | 🟡 |
| 9 | 登记后续单：`QuickFix` 的 Java 维度、`CloseSavedTabs` 的 `any_clean`、`NewFile` 的无根、`CloseTerminalTab`、Edit 那 11 行的 `editor_focused`、菜单行 aria | — | 后续 |

---

## 9. 未核实清单

- **未跑应用、未截图、未 `cargo build`**（主理人要求）。§5 的 15 条断言是**待执行**的验收条目，不是已通过的结论。
- **IDEA 的实际菜单行为**：§3 对照表里标「凭使用经验，未核实」的 5 条（第 1、4、8、9、10 条）取自使用经验，未在本仓或 IDEA Community 源码中取证。若要落地为硬口径，需按 `develop-lithe` 的要求去查 IDEA Community 的 action 源码并记下路径与版本。
- **Ctrl+Tab 在菜单面板打开期间是否真的会穿透到全局 action**：机制上推断会（`install_menu_key` 的 `KeyBinding` 上下文为 `None`，而 `lithe-ui/src/menus/menu.rs` 的 `keys()` 只吃方向键 / Enter / Space / Esc），但**未实测** gpui 的 Takeover 是否会拦截。V10 / §4.4 需要实测确认。
- **标签页是否有 dirty 标记**：§3.1 结尾提到的补偿信号，「当前是否有该标记」未核实。
- **`window.focused(cx)` 在菜单 Takeover 期间的行为**：§6.4 的后续单方案需要实测。
- **行编辑五个动作的键位是否在 `ElyEditor` 上下文里**：§4.4 里「这 11 行天然一致」的依据来自 `menu_bar.rs:680-689` 的注释与 `keybind_action()` 的返回值，未去 `lithe-ui/src/editor` 逐条核对绑定上下文。
