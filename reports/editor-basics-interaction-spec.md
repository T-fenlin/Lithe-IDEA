# 编辑器基本功交互与键位定稿（B3 查找替换 / B2 文件生命周期 / B1 文件增删改名）

- 状态：交互设计稿（纯设计，未改任何源码）
- 作者：陆井然（界面设计师）
- 分支：`feat/gpui-editor-basics`
- 依据：
  - 架构与接口：`reports/editor-basics-remaining-design.md`（闻本立，§8 主理人裁决 12 项为**结论**，本稿只细化与复核冲突，不改结论）
  - 强制 Skill：`.agents/skills/develop-lithe/SKILL.md`（架构边界、硬编码禁令、验证流程）
  - 交互性能边界：`.agents/notes/implemented/architecture/2026-09-13-resizable-ui-performance-boundaries.md`
  - 前端分层与平台目标：`.agents/notes/proposed/architecture/2026-10-08-unified-gpui-frontend-selection-and-layer-boundaries.md`
    （**关键前提**：gpui 前端的目标平台是 macOS ∪ Windows ∪ Linux；界面规格真源是 IDEA Community；验收取 Windows ∪ macOS 并集）
- 本稿全部结论均经**读码核对**，核对到行号；凡未经读码确认的 IDEA 行为，一律标注「待核实」。

术语：**组件库** = `rust/lithe-ui`；**宿主** = `rust/lithe-gpui/crates/*`。

---

## 0. 复核结论摘要

### 0.1 键位定稿（一句话）

`Ctrl+F` 查找、`Ctrl+R` 替换、`F3` 下一处、`Shift+F3` 上一处、`Esc` 关闭查找条；
**字面量一律写 `secondary-`**（`secondary-f` / `secondary-r` / `f3` / `shift-f3`），
**不写 `ctrl-f`**。理由见 §1.3。

### 0.2 冲突核查结论（一句话）

**五个目标键在现有绑定里全部无冲突**，不需要改任何既有绑定；但发现 4 项本批范围之外的问题，
登记在 §0.3，交由主理人决定是否另开单。

### 0.3 本批发现的新问题（不在裁决范围内，不夹带修复）

| 编号 | 问题 | 位置（读码核对） | 影响 | 建议 |
| --- | --- | --- | --- | --- |
| F1 | **同一个键在同一个 context 里绑了两个动作**：`secondary-d` 同时绑 `SelectNext`（多光标选下一个）与 `DuplicateLine`（复制行） | `rust/lithe-ui/src/editor/keys.rs:103` 与 `:111`（同一 `Some("ElyEditor")` context） | 按 `secondary-d` 到底触发哪个取决于 gpui 的解析顺序，其中一个**永远按不出**；菜单上 `DuplicateLine` 又声明胶囊 `secondary-d`（`menu_bar.rs:535`），于是菜单显示一颗可能是死键的键 | 另开单：二者必须拆键。IDEA 口径「Duplicate Line or Selection」默认 `Ctrl+D`，`Select Next Occurrence` 是 `Alt+J`（Windows）/`Ctrl+G`（mac）。待核实 IDEA Community 源后再定，**本批不动** |
| F2 | **宿主的 `install_actions` 用 `ctrl-` 字面量，而设置页键位表把它报成 `secondary-`**：设置页显示 ⌘S，真实生效的是 Ctrl+S（macOS） | 绑定：`crates/editor/src/lib.rs:31/33/37`、`quick_open.rs:36`、`command_palette.rs:87/88`、`settings/dialog.rs:52`、`workspace.rs:2929`、`menu_bar.rs:209-215/282/283`；报成 `secondary-`：`settings/src/keymap.rs:352-407`（含注释「`ctrl-` 已按本模块的约定改写为 `secondary-`」） | 在 macOS 上「设置页显示的键位」与「真能按出来的键位」系统性不一致；且 IDEA-mac 默认（⌘S/⌘P/⌘⇧P/⌘G…）整体失效 | 另开单收敛：把宿主 `install_actions` 的 `ctrl-` 字面量改成 `secondary-`（这正是 `DEFAULT_BINDINGS` 已经假设的形态，改完只会**消除**macOS 上的错位）。**本批只让新键写对**，见 §1.3 |
| F3 | **菜单栏没有 disabled 通道，但组件库有**：`workbench::menu_bar::MenuItem` 只有 `Action / NotWired / Separator / Theme`（`menu_bar.rs:180-190`）；而组件库的 `lithe_ui::menus::model::MenuItem::disabled(bool)` **已存在**（`menus/model.rs:107`） | 见左 | 裁决 #7「菜单置灰列为后续」的成本比预想低：后续只需给 workbench 的 `MenuItem::Action` 加一个 disabled 标志，视觉能力组件库已有 | 记录，本批仍按裁决 #7 走 no-op；但要补一个「无活动缓冲」的用户可见提示，见 §3.1 |
| F4 | **`NotWired → Action` 会丢掉现有的「未接入」提示**：`MenuItem::NotWired` 点击会走 `report_menu_not_wired` → `notify_not_wired`（用户可见通知）；改成 `Action` + no-op 后，点 Save As（无缓冲）将**完全静默** | `workspace.rs:2663-2671` / `:2883`；`menu_bar.rs:1869-1890` | 从「告诉你还没做」退化成「点了没反应」，是负向体验 | 见 §3.1 的建议（诊断 + 一条信息级通知） |
| F5 | **组件库已有「校验失败就拦下并显示原因」的现成原语**：`PromptDialog::check(impl Fn(&str) -> Result<(), SharedString>)` —— 每次渲染都跑校验、提交按钮 `disabled(!ready)`、Enter/提交被 `held back`、**原因显示在字段下方** | `lithe-ui/src/overlays/dialogs.rs:200-345`（`check` 在 `:246`，文案注释在 `:201`，拦下逻辑在 `:283-289`） | 我原以为「宿主校验 + 行内报错」需要新造机制；其实**库内已有同形原语** | ① B1 的 New File / New Folder 直接用它（零新增 API，文案键已齐，见 §4.3）；② §4.3 的内联重命名按同形补一个小口子（Q1 的批准理由因此更强） |
| F6 | **组件库已有危险操作确认框**：`ConfirmDialog` + `.destructive()`（确认按钮自动用 `ButtonVariant::Danger`） | `lithe-ui/src/overlays/dialogs.rs:113-193`（`destructive()` 在 `:151`，`Danger` 在 `:166-170`） | 删除确认**不必**手搓 `Dialog` + 自建按钮；且它没有输入框 ⇒ `Dialog::on_enter` 不生效 ⇒ **Enter 天然不是「删除」**，我的安全要求结构性成立 | §4.4 改为直接复用它 |

### 0.4 需要主理人/团队裁决的 3 个新点

| 编号 | 事项 | 为什么需要拍板 |
| --- | --- | --- |
| Q1 | **重命名校验失败的内联表现**：本稿推荐「行内字段保持打开 + 行内 danger 文案 + `Input::invalid` 红框」，这需要给 `ProjectTree` 加**一个校验回调**（形态照抄库内已有的 `PromptDialog::check`，见 F5）。保底方案是「字段重开 + 通知 toast」，不需要新增库 API | 库公开面变化需要授权（裁决 #10 只授权了「复用库内 `Editing` 原语」）。**新证据**：`PromptDialog::check` 证明这个形态是库内既有约定，不是新发明 |
| Q2 | **行内重命名初值只选主名、不选扩展名**：`Editing::begin` 现在全选（`forms/inline.rs:34-35`），会导致「打字吃掉 `.java`」。本稿推荐初值选中**去掉最后一个扩展名**的部分，需要给 `Editing::begin` 加一个范围参数（小改动，但同样超出授权清单）。**替代**：若改用库内已有的 `PromptDialog` 做重命名，`PromptDialog` 也是全选（`dialogs.rs:276-283`），不能省掉这一步 | 同 Q1 |
| Q3 | **B1 的「新建」用哪套 UI**：本稿推荐树内 `New File`/`New Folder` 用库内 `PromptDialog` + `check`（零新增 API、文案键已齐、校验免费），菜单 `New File` 仍用架构师定的 `prompt_for_new_path`（平台路径选择器，能选目录）。两者 UI 不同（IDEA 树内是行内命名），需主理人认可这个取舍 | 影响 §3.3 与 §4.3 的落点 |

---

## 1. 键位定稿与冲突复核（硬要求）

### 1.1 既有绑定全量清单（读码实测，用于冲突判定）

**A. 宿主 `cx.bind_keys`（13 处，全窗绑定 `None` context，除注明外）**

| 键 | 动作 / 命令 | context | 位置 |
| --- | --- | --- | --- |
| `ctrl-s` | `SaveBuffer` | None | `crates/editor/src/lib.rs:31` |
| `ctrl-w` | `CloseActiveTab` | None | `…:33` |
| `ctrl-.` | `ShowCodeActions` | None | `…:37`（macOS 另有 `cmd-.`，`…:40`） |
| `f12` | `NavigateToDefinition` | None | `…:32` |
| `cmd-a` / `ctrl-a` | `SelectAllInTree` | `EXPLORER_KEY_CONTEXT` | `crates/explorer/src/lib.rs:12-13` |
| `cmd-c` / `ctrl-c` | `CopySelection` | `EXPLORER_KEY_CONTEXT` | `…:14-15` |
| `ctrl-enter` | `CommitChanges` | None | `crates/git/src/lib.rs:31` |
| `ctrl-,` | `OpenSettings` | None | `crates/settings/src/dialog.rs:52` |
| `ctrl-shift-p` | `OpenCommandPalette` | None | `workbench/src/command_palette.rs:87` |
| `ctrl-m` | `ToggleMenuBar` | None | `…:88` |
| `ctrl-p` | `OpenQuickOpen` | None | `workbench/src/quick_open.rs:36` |
| `ctrl-shift-r` | `OpenReplaceInFiles`（`REPLACE_KEY`） | None | `workbench/src/replace_in_files.rs:39,42` |
| `ctrl-o` | `OpenProjectFolder` | None | `workbench/src/workspace.rs:2929` |
| `ctrl-shift-t` | `ReopenTab` | None | `menu_bar.rs:209`（经 `install_menu_key`） |
| `ctrl-b` | `ToggleSidebar` | None | `menu_bar.rs:210` |
| `ctrl-j` | `ToggleTerminalPane` | None | `menu_bar.rs:211` |
| `ctrl-=` / `ctrl--` / `ctrl-0` | 缩放三键 | None | `menu_bar.rs:212-214` |
| `ctrl-g` | `OpenGoToLine` | None | `menu_bar.rs:215` |
| `secondary-alt-left` / `secondary-alt-right` | `GoBack` / `GoForward` | None | `menu_bar.rs:217-218` |
| `ctrl-tab` / `ctrl-shift-tab` | `NextTab` / `PreviousTab` | None | `menu_bar.rs:282-288` |
| `tab` / `shift-tab` | `NextScope` / `PreviousScope` | `SCOPE_KEY_CONTEXT`（Search Everywhere） | `workbench/src/search_everywhere.rs:61-62` |

**B. 组件库 `lithe-ui` 绑定**

| 键 | 动作 | context | 位置 |
| --- | --- | --- | --- |
| `escape` | `SingleCursor` | `ElyEditor` | `editor/keys.rs:106` |
| `ctrl-space` | `Complete` | `ElyEditor` | `editor/keys.rs:105` |
| `secondary-z` / `secondary-shift-z` / `secondary-a/c/x/v/d/y` / `secondary-/` … | 撤销/重做/剪贴板/行编辑 | `ElyEditor` | `editor/keys.rs:98-121` |
| `secondary-d` | **`SelectNext` 与 `DuplicateLine` 双绑**（见 F1） | `ElyEditor` | `editor/keys.rs:103`、`:111` |
| 文本键（`backspace`/`left`/`enter`/`secondary-a`…） | `ElyInput` 动作 | `ElyInput` | `forms/text/actions.rs:58-107` |
| `secondary-b/i/k` | 富文本 | `ElyRich` | `documents/rich.rs:28-30` |

**C. 关键字面量（`ElyInput` 全部绑定里**没有** `escape`**）**

- `forms/text/actions.rs:58-107` 逐条读过：无 `escape`、无 `f3`、无 `ctrl-f`、无 `ctrl-r`。
  ⇒ **文本输入框不吃 Escape**，Escape 会继续向外冒泡（这是 §1.4 兜底方案成立的结构性依据）。
- 单行输入框未提交时：`enter` → `Enter` 动作（`…:87`），`secondary-enter` → `Submit`（`…:89`）。
  提交语义走 `InputEvent::Submit`，宿主的 `open_go_to_line` 就是这么接的（`editor_view.rs:1854-1866`）。

**D. 全局 Escape 拦截器**

- `MenuBar::new`：`cx.intercept_keystrokes(...)`，判据 `is_escape` = `key == "escape" && number_of_modifiers() == 0`
  （`menu_bar.rs:43-45`、`:1168-1178`）。**关键行为**：只有 `bar.close(...)` 真的关掉了东西才 `stop_propagation()`
  （`…:1172-1177`）⇒ **菜单没开着时它不吞 Escape**。
- 同类先例（可复用）：`lithe_ui::overlays::spotlight.rs:145-163` 用「窗口句柄 + 裸 Escape」的窗口级拦截，
  非模态浮层也这么干。

### 1.2 目标键逐一冲突核查

| 目标键 | 现状 | 结论 |
| --- | --- | --- |
| `Ctrl+F` 查找 | 全仓无 `ctrl-f` / `secondary-f` 绑定 | **无冲突** |
| `Ctrl+R` 替换 | 全仓无 `ctrl-r` / `secondary-r` 绑定 | **无冲突**。与 `ctrl-shift-r`（ReplaceInFiles）不冲突（多一个 Shift）；采用 `secondary-r` 后正好与 `secondary-shift-r` 构成 IDEA 的 Replace / Replace in Path 配对 |
| `F3` 下一处 | 全仓只有 `f12`（`editor/src/lib.rs:32`） | **无冲突** |
| `Shift+F3` 上一处 | 同上 | **无冲突** |
| `Esc` 关闭查找条 | `ElyEditor.escape→SingleCursor`（context 限定）；`MenuBar` 全局拦截（仅在菜单开着时吞）；`Dialog` 自己处理 Escape（`overlays/dialog.rs:311-314`）；`InlineEdit` 自己处理 Escape（`forms/inline.rs:132-140`）；`ElyInput` 不处理 | **无冲突**，但优先级必须按 §1.4 固定成契约 |
| `Ctrl+H` | 全仓无绑定，但与 `ctrl-shift-h`（无）、`ctrl-h` 冲突与否都无所谓 | 裁决 #1 已定为 `Ctrl+R`，**H 不用**，记录以免后人误加 |

> 交叉检查：`ctrl-f` 与 `ctrl-shift-f`（无）、`f3` 与 `f12`、`shift-f3` 与 `shift-f12`（无）均不冲突。
> 与组件库 `secondary-f/`、`secondary-r` 也无重叠（`secondary-/` 是注释开关，`secondary-r` 空）。

### 1.3 字面量口径定稿：写 `secondary-`，不写 `ctrl-`

**定稿**

```rust
pub const KEY_FIND: &str = "secondary-f";        // 查找
pub const KEY_FIND_REPLACE: &str = "secondary-r"; // 替换
pub const KEY_FIND_NEXT: &str = "f3";            // 下一处
pub const KEY_FIND_PREVIOUS: &str = "shift-f3";  // 上一处
// Esc 不在这里：它带 context，见 §1.4
```

**理由（四条，逐条有据）**

1. **只有 `secondary-` 能给出 IDEA 在 macOS 上的默认键。**
   gpui 里 `secondary` 是平台归一化开关（macOS → Command，其他 → Control），`ctrl` 是字面 Control
   （`settings/src/keymap.rs:20-26` 模块文档）。IDEA-mac 的 Find/Replace 就是 ⌘F/⌘R；平台目标含 macOS
   （`2026-10-08-unified-gpui-frontend-selection…`：一套 Rust 界面覆盖 macOS/Win/Linux），
   验收又取 Windows ∪ macOS 并集。⇒ `ctrl-f` 在 macOS 上**直接偏离 IDEA 真源**。
2. **`secondary-` 是本仓已声明的约定，`ctrl-` 是尚未收敛的滞后。**
   `keymap.rs:20-26` 把「写 `secondary-`，不写 `cmd-`」列为**硬约束**；`DEFAULT_BINDINGS`
   （`keymap.rs:352-407`）把宿主那批 `ctrl-*` **全部改写为 `secondary-*`** 并注明「Windows 上两者等价」。
   即：仓库的既定意图就是 `secondary-`（详见 F2）。
3. **胶囊显示不会说谎。** 菜单键位胶囊走 `action.menu_key()` 的同一个字面量
   （`menu_bar.rs:1862-1864`），而 `KbdCombo` 会按平台解析字面量渲染
   （`lithe-ui/src/typography/inline.rs`：`keystroke(source, platform)` + `keystroke_labels`）。
   ⇒ 写 `secondary-f`，菜单上 mac 显示 ⌘F、Win/Linux 显示 Ctrl+F，与实际绑定的**同一个字面量**一致，
   物理上不可能「显示的键按不出」。写 `ctrl-f` 则三端都显示 Ctrl+F（mac 上错）。
4. **`ctrl-f` 在 macOS 另有语义（Emacs 前移一字符）。** 用裸 Ctrl+F 做「查找」与 macOS 文本惯例相撞，
   是 IDEA-mac 与 macOS HIG 都不认的组合。

**代价（如实记录，不掩盖）**

- 与 menu_bar.rs 现有 13 处 `ctrl-*` 常量**暂时不一致**（F2）。macOS 上会短暂出现「查找是 ⌘F、跳转到行是 Ctrl+G」
  这种内部不齐。收口方向是**把既有 `ctrl-*` 改成 `secondary-*`**（F2 另开单），不是把新键改回 `ctrl-`。
- 架构师 §附 建议的 `KEY_FIND = "ctrl-f"` 与此冲突：本稿按「IDEA 真源 + 仓库硬约束」优先，采纳 `secondary-f`。
  这只改字面量，不改裁决 #1 的**键位结论**（仍是 F / R / F3 / Shift+F3，三端统一）。

### 1.4 查找条的 key context 与 Escape 优先级

**定稿**

- 查找条容器挂 `.key_context(FIND_BAR_KEY_CONTEXT)`，常量
  `pub const FIND_BAR_KEY_CONTEXT: &str = "EditorFindBar";` **定义在 `crates/editor`**（渲染方持有 context 名），
  workbench 绑键时引用它——与 `crates/explorer/src/lib.rs` 的 `EXPLORER_KEY_CONTEXT` 同一做法。
- 绑定 `escape → CloseFind`，context = `Some("EditorFindBar")`，**handler 挂在查找条容器上**
  （`cx.listener(|pane, _: &CloseFind, window, cx| pane.close_find(window, cx))`，同
  `explorer_view.rs:588` 的 `.on_action(cx.listener(Self::select_all_in_tree))`）。
- `F3` / `Shift+F3` 用**全窗绑定（`None` context）**：因为 IDEA 里焦点在查找框里时 F3 也要跳下一处，
  只绑 `ElyEditor` 会漏掉这种情况。

**优先级契约（从上到下，先命中先处理）**

```
1. MenuBar 全局 Escape 拦截器     —— 仅当有菜单开着时命中并 stop_propagation
2. 模态对话框 Escape（Dialog / GoToLine / 未保存确认） —— 仅当有活动对话框
3. EditorFindBar.escape → CloseFind —— 仅当焦点在查找条内
4. ElyEditor.escape → SingleCursor  —— 仅当焦点在编辑器内
5. 无处理
```

**为什么「查找条压过 `ElyEditor.escape`」是结构保证、不靠 context 深度比拼**

查找条是**正文容器里的兄弟节点**（架构师 §2.2 定的挂点：`div#lithe-editor-host` 内的绝对定位浮层），
不是 `CodeEditor` 的后代。gpui 的 key context 只取**焦点路径上的祖先链**，所以：

- 焦点在查找框里 → context 链是 `[…, "EditorFindBar", "ElyInput"]`，**不含 `ElyEditor`**；
- 焦点在编辑器里 → 链含 `ElyEditor`，**不含 `EditorFindBar`**。

两者上下文**根本不同时存在**，因此不存在「谁先谁后」的歧义。架构师 §2.5 担心的
「查找条输入框持有焦点时其 context 优先级更高」自动成立，无需额外兜底。

**「输入框不吃 Escape」的兜底方案（三条，按推荐顺序）**

1. **主路径（定为正式方案）**：`CloseFind` 是我们**自己声明的 action 类型**。gpui 的按键派发按
   context 解析出 action 后，只有**为该 action 类型注册过 handler 的元素**才处理它；`ElyInput` 声明的是
   `Backspace/Left/Enter/Submit…`（`forms/text/actions.rs:14-47`），**没有 `CloseFind`**，
   于是事件继续冒泡到查找条容器的 `on_action`。
   ⇒ 只要不给输入框加 `CloseFind` 的 handler，Escape 必然到查找条。这是**结构性**兜底，不是权宜。
2. **应急（仅在真机探针发现 Escape 泄漏时才加）**：在**包裹两个 `Input` 的 div**（不是整个条）上挂
   `.on_key_down`，判 `keystroke.key == "escape"` → `close_find` + `cx.stop_propagation()`。
   只挂输入框外层，避免与第 1 条的 handler 重复触发（双关）。
3. **禁止做法**：给查找条挂**窗口级 `intercept_keystrokes`**（`spotlight.rs` 那种）。
   它会连「焦点在编辑器时」的 Escape 一起吞掉，破坏契约第 4 层（编辑器要保留 `SingleCursor`）。

**明确的行为边界（写成契约，避免实现时来回改）**

- 焦点在查找条内按 Esc → 关闭查找条，焦点回编辑器（§2.6）。**不开菜单时不会被 MenuBar 抢**。
- 有菜单开着时按 Esc → **先关菜单**（MenuBar 拦截器 `stop_propagation`），查找条保持打开。正确（菜单是最上层临时层）。
- 有模态对话框时按 Esc → 先关对话框，查找条保持打开。正确。
- **焦点在编辑器里、查找条开着**时按 Esc → 走 `ElyEditor.escape`（收起多光标 / 关补全 / 清 chat 标记），
  **查找条不被关**。这是**有意的偏离**「Esc 一定关查找条」，理由：Escape 在编辑器里已经承担
  多光标与各种弹出层的语义（`editor/keys.rs:314-323`），把它再兼作「关查找条」会与这些语义互相踩。
  用户的退出路径有两条且都显式：点 ✕ 按钮、或再按一次 `Ctrl+F` 回焦查找框后按 Esc。
- `Ctrl+F` 在查找条已开时：**重新聚焦查询框并全选其文本**（IDEA 口径），**不收起已展开的替换行**（不破坏用户布局）。

### 1.5 打开时的选区预填

- **单光标 + 非空 + 不含换行**的选区 → 用选区文本预填查询框（IDEA 口径：选一个词按 Ctrl+F 直接搜它）。
- **多光标（多个 range）** → **不预填**（取哪个是任意的，猜错比空着更差），查询框置空。
- **选区跨行（含 `\n`）** → **不预填**。理由：查找框是单行输入，塞进去只能被截断或变形，属于「静默错」，不如空着。
- 预填后**不自动全选**查询框文本？—— **要全选**。理由：预填只是「帮你省一次输入」，全选后直接打字即覆盖，
  与 §2.2「打开即全选」统一。
- 打开时若查询非空 → 立刻定位到**第一处「起点 ≥ 当前光标偏移」的命中**（找不到则回绕到第 0 处）并选中它。
  这是「find next」的自然起点，也避免「打开后跳回文件开头」的突兀。

### 1.6 键位登记落点与需同步的守卫测试

| 键 | 登记方式 | 落点 | 备注 |
| --- | --- | --- | --- |
| `secondary-f` | `install_menu_key(cx, KEY_FIND, OpenFind, MenuAction::Find)` | `menu_bar.rs::install_key_actions` | 与 `KEY_GO_TO_LINE`/`OpenGoToLine` 一字不差的范式 |
| `secondary-r` | `install_menu_key(cx, KEY_FIND_REPLACE, OpenFindReplace, MenuAction::FindAndReplace)` | 同上 | |
| `f3` | `cx.bind_keys([KeyBinding::new(KEY_FIND_NEXT, FindNext, None)])` + `cx.on_action(...)` → 外壳 | 同上 | **不建 `MenuAction`**：`MenuAction` 若不在任何菜单里，会挂 `menu_bar_non_command_contains` 与
「登记在 `NON_COMMAND` 里但菜单没有它」两条守卫（`menu_bar.rs:2549-2553`） |
| `shift-f3` | 同上（`FindPrevious`） | 同上 | |
| `escape` | `cx.bind_keys([KeyBinding::new("escape", CloseFind, Some(FIND_BAR_KEY_CONTEXT))])` | 同上（引用 `crates/editor` 的常量） | `install_menu_key` 硬编码 `None` context，**用不了**，必须单独绑 |

**必须同步修改的守卫测试（架构师 §7 已列，这里补精确条目）**

1. `menu_bar.rs::b2_actions_are_wired_and_only_bound_ones_carry_a_key`（`:2215-2375`）
   - `(MenuAction::Find, "lithe.menu.find", false)` → `true`（`:2228`）
   - `(MenuAction::FindAndReplace, "lithe.menu.findAndReplace", false)` → `true`（`:2229-2233`）
   - `bound` 清单（`:2353-2374`）按 `MENUS` 顺序插入 `KEY_FIND`、`KEY_FIND_REPLACE`
     —— 位置由 Edit 菜单的项序决定（`Find` 在 `FindAndReplace` 之前，二者在 `SelectAll` 之后，见 `menu_bar.rs:778-779`）。
     **`F3` / `Shift+F3` 不进这份清单**：它们不出现在任何菜单行上，`menu_key()` 保持 `None`，没有胶囊。
   - `MenuAction::Find.keybind_action()` 由 `None` → `Some(Box::new(OpenFind))`（`menu_bar.rs:589`）
2. `keymap_engine.rs::menu_bar_non_command_contains`（`:493-527`）：`Find`/`FindAndReplace` 的处理口径要与
   `menu_bar.rs:2440` 的 `NON_COMMAND: [MenuAction; 44]` **两边同时**决定（它们有本地分支就**不**给
   `CommandId`，与 `Preferences`/`GoToLine` 同规则）。
3. **不新增 `CommandId`**（与 GoToLine 同批口径），因此：
   - `settings/src/keymap.rs::DEFAULT_BINDINGS` **不加**这四条；
   - `the_default_table_is_the_keys_that_are_actually_bound_today`（`keymap.rs:593`）**不会失败**，
     但它的注释「改了任何一处 `bind_keys` 就要同步改这里」需要补一句**例外**：没有 `CommandId` 的键
     （Find/FindAndReplace/F3/Shift+F3/Escape/GoToLine/Zoom/NextTab/GoBack…）不进表。
     否则下一位评审会以为表过期了。
   - **已知副作用（登记）**：这四条键**不会出现在设置页的 Keyboard 列表**里（因为那张表按 `CommandId` 索引）。
     与 GoToLine/NextTab/GoBack 现状一致；后续补 `CommandId` 是单独一件事。
4. Esc/F3/Shift+F3 是 `None` context + 非托管 action：`keymap_engine` 的快照会把它们留在
   `baseline_other` 并原样重放（`keymap_engine.rs:40-56, 110-140`），改别的命令键位不会丢它们。**无需改引擎**。

---

## 2. 查找条交互状态机

### 2.1 状态与不变量

沿用架构师 §2.1 的 `FindBar` 结构，补充**状态与不变量**：

| 不变量 | 内容 |
| --- | --- |
| I1 | `find.is_some()` ⇒ `find.editor == 当时 active buffer 的 editor.entity_id()`（换缓冲即 `close_find`） |
| I2 | `matches` 只由「当前 `editor` 的正文 + `query` + `options`」推导，**不跨缓冲缓存** |
| I3 | `matches.is_empty()` ⇒ `current == None`；否则 `current ∈ [0, len)` |
| I4 | `error.is_some()` ⇒ `matches` 为空且编辑器 `backgrounds` 为空 |
| I5 | `find.is_none()` ⇒ 归属编辑器的 `backgrounds` **已被清空**（不能让旧缓冲留着残留高亮） |
| I6 | 关闭后焦点**回到编辑器**（不是回到 shell、不是留在已销毁的输入框） |

**`error` / `matches` / `current` 的合法组合**

| 查询 | 结果 | `matches` | `current` | `error` | 计数区显示 |
| --- | --- | --- | --- | --- | --- |
| 空 | — | `[]` | `None` | `None` | **空白**（不是 "No results"，见 §2.8 缺口 G3） |
| 非空 | 命中 N>0 | N 个 range | `Some(i)` | `None` | `i+1 of N` |
| 非空 | 命中 0 | `[]` | `None` | `None` | `No results`（danger 色） |
| 非空 | 正则编译失败 | `[]` | `None` | `Some(msg)` | `msg`（danger 色） |

### 2.2 打开（find / find+replace 两种入口）

| 步骤 | find（`secondary-f`） | find+replace（`secondary-r`） |
| --- | --- | --- |
| 前置守卫 | 无活动缓冲 → no-op + 诊断 `reason=no-active-buffer`（对齐 `open_go_to_line` 的 `editor_view.rs:1832-1836`） | 同左 |
| 前置守卫 | `window.has_active_dialog(cx)` → no-op + 诊断 `reason=active-dialog`（对齐 `editor_view.rs:1837-1840`）。**新增**：防止在设置对话框开着时从背后弹出查找条抢焦点 | 同左 |
| 已开且同一缓冲 | 重新聚焦查询框 + 查询框全选；`showing_replace` **不变** | 展开替换行（`showing_replace = true`）+ 聚焦查询框 |
| 初始 `showing_replace` | `false` | `true` |
| 预填 | 按 §1.5 | 按 §1.5（查询框预填；替换框**保持上次内容**，同会话内不主动清空） |
| 首次重算 | 立即重算（不走 debounce） | 立即 |
| 首次定位 | 命中非空 → 选中「起点 ≥ 光标偏移」的第一处，`current` 指向它 | 同左 |
| 焦点落点 | **查询框**（替换框不抢焦点——IDEA 是把焦点给搜索框，这点容易做错） | **查询框** |
| 高亮 | `set_backgrounds`：`current` 处 `warning.opacity(0.45)`，其余 `warning.opacity(0.2)`（与 gallery `editor/search.rs` 一字不差） | 同左 |
| 诊断 | `S1_EDITOR_FIND open=true mode=find query=<chars> matches=<n> current=<i>` | `mode=replace` |

### 2.3 输入与重算（含 debounce 与错误的呈现时机）

**触发重算的事件**

| 事件 | 是否 debounce | 说明 |
| --- | --- | --- |
| 查询框 `Changed`（逐键） | **是，约 120 ms** | 照 `AUTO_SAVE_DEBOUNCE` 范式；2 MiB 上限下单次扫描可接受，但逐键全量扫没必要 |
| 替换框 `Changed` | 否（**不重算**） | 替换文本不影响命中，只影响按钮可用性 |
| 选项（case/word/regex）变化 | 否 | 立即，用户期待立刻看到结果变化 |
| `EditorEvent::Changed`（正文变） | **是，约 120 ms**（同一 debounce） | 编辑时重算，避免打字期间每键一次重算 |
| 步进 / 替换后 | 否 | 立即 |
| 打开 / 重新聚焦 | 否 | 立即 |

**debounce 的实现要求**：必须带「代次」失效（照 `completion_generation: Arc<AtomicU64>` 的做法，
`editor_view.rs:219`），任务落地前比对代次，晚到的结果丢弃。否则快速输入会出现「结果回跳」。

**正则错误的呈现时机**

- 用户输入半成品正则（如 `(`）时，`find_all` 返回 `Err(message)` → 计数区**立刻**显示 `message` 的最后一行（danger 色），
  `matches` 清空、高亮清空、上/下一处按钮禁用。
- 因为查询逐键重算（debounce 后），错误会随输入「实时」出现/消失——这正是 IDEA 的表现，不需要额外确认。
- 空查询**不是**错误：不能显示 `No results`（见 §2.8 G3）。

**选项变更后的 `current` 处理（定契约）**：`current = min(old_current, len-1)`，`len == 0` 则 `None`。
不做「按偏移就近匹配」的复杂逻辑（可预期性优先）。同时按新的 `current` 重新选中并滚进视野。

### 2.4 步进与环绕

| 场景 | 行为 |
| --- | --- |
| `F3`（下一处） | `current = (current + 1) rem len` → `editor.select([matches[current]])` |
| `Shift+F3`（上一处） | `current = (current + len - 1) rem len` |
| 环绕 | **两端都环绕**（最后一处按下一处回到第 0 处；第 0 处按上一处到最后一处）。诊断带 `wrapped=true|false` |
| 查询框内按 Enter | 等同「下一处」：订阅查询框 `InputEvent::Submit` → `find_step(true)`（IDEA 口径）。注意 `ElyInput` 的 `enter` 在单行框里产生 `Submit`（`forms/text/actions.rs:87-89`），`open_go_to_line` 就是这么接的 |
| 0 命中 / `error` | 按钮已禁用；若仍被键盘触发 → no-op + 诊断 |
| 查找条**未开**时按 F3 | **no-op + 诊断 `step=false reason=closed`**（v1 口径）。「关闭后 F3 重开并复用上次查询」列为后续（需要会话级 `last_find`，而裁决 #4 只允许会话内保持选项，不持久化） |
| 有活动对话框时按 F3 | no-op + 诊断（与 `open_find` 的守卫一致） |
| 滚进视野 | 复用 `CodeEditor::select` 自带的 `scroll_to_item(primary_row, Center)`（`editor/state.rs:687` 附近），不自己算滚动 |
| 多光标 | `select` 会把选区集合换成单个 range ⇒ **步进会收起多光标**。这是有意的（定位必须唯一），写进契约 |

**焦点**：步进**不改焦点**。焦点在查询框里就留在查询框里（IDEA 一致）。
⇒ 曾担心**编辑器未聚焦时的选区配色**看不清（会让人以为「没跳过去」）——**已实测闭环**：
未聚焦选区+当前匹配的**合成对比度** 深 **3.37:1** / 浅 **2.09:1**，足够；且一次 `F3` 另有
「滚到视口中心」「计数变化」两个独立信号。**结论：够用、不改库**（详见 §6-B5）。

### 2.5 替换单条 / 全部

**落 API（沿用架构师 §2.4）**

- 单条：`editor.update(|e, cx| e.edit([(bar.matches[cur].clone(), replace_text)], cx))`
- 全部：`editor.update(|e, cx| e.edit(bar.matches.iter().map(...).collect::<Vec<_>>(), cx))`
  一次 `edit` 批次 = **一个撤销步**（`editor/state.rs:832-846` 的 `edit` → `apply`），只读文件天然拒绝。

**替换后（定契约）**

| 项 | 单条 | 全部 |
| --- | --- | --- |
| `matches` | 重算（走 debounce 或立即；建议立即，因为替换是显式动作） | 重算 |
| 新 `current` | **第一处起点 ≥ 被替换区间的 end 的命中**；没有则回绕 0；`len==0` 则 `None`。这条规则防「替换文本含查询词」时的原地打转 | `len>0 ? Some(0) : None` |
| 焦点 | **不动**（用户点按钮前焦点在哪就留在哪） | 同左 |
| 选中 | 选中新的 `current` 并滚进视野 | 同左（无命中则不选） |
| 只读缓冲 | `edit` 拒绝 → 文本不变。必须检测「替换前后 `text()` 相同」并给出**诊断 + 错误通知**（走 `record_notification`，见 §3.7），不能静默 | 同左 |
| 已脏 | `on_text_changed` 置脏 + `revision++` + 诊断/自动保存 + `clear_hover`；随后**必须重跑查找**（架构师 §2.4） | 同左 |
| 高亮 | 重算后**重新 `set_backgrounds`**。`set_diagnostics` 清的是诊断不是 backgrounds（`editor_view.rs:1349`），两者独立 | 同左 |

**按钮可用性（沿用 FindWidget 现状）**：`found = total > 0`，`Replace` / `Replace all` 均 `.disabled(!found)`（`editor/find.rs:308,318`）。
⇒ 0 命中时不可能触发替换，无需额外守卫。

### 2.6 关闭与焦点回归

| 触发 | 行为 |
| --- | --- |
| ✕ 按钮 | `close_find` |
| `Esc`（焦点在查找条内） | `close_find`（§1.4） |
| 宿主判定需收起（§2.7） | `close_find`（静默） |

`close_find` 的**必须动作**（顺序敏感）：

1. 若 `find.editor` 对应缓冲仍存在 → `editor.update(|e, cx| e.set_backgrounds(Vec::new(), cx))`（清高亮，满足 I5）。
   **这一步不能省**：`backgrounds` 是编辑器自己的状态，关条不清就会在该缓冲留下永久残留高亮。
2. `self.find = None`。
3. **焦点回到该缓冲的编辑器**：`window.focus(editor.read(cx).focus_handle(cx))`。
   若该缓冲已被关掉（`forget_paths` 场景）→ 跳过，由外层负责把焦点给新的 active。
4. **不动编辑器选区**：保留最后一处命中的选中（IDEA 行为，用户关掉查找条后可以继续编辑选中的那处）。
5. 诊断：`S1_EDITOR_FIND open=false reason=escape|close|switch|buffer-gone idle=true|false`。

**注意**：`GoToLine` 是模态 `window.open_dialog`（`editor_view.rs:1879`），与查找条**不同层**；
查找条是非模态浮层，**不阻塞编辑器滚动与可见性**（架构师 §2.2 的取舍）。二者同时存在时，
Esc 先关对话框（§1.4 契约第 2 层）。

### 2.7 自动收起（切标签 / 关标签 / 换缓冲 / 重载）

| 触发 | 是否收起 | 顺序要求 |
| --- | --- | --- |
| `activate(other)`（切标签 / Ctrl+Tab / 点标签） | **收起** | **必须先 `close_find`（在第 2.6 步 1 清掉旧编辑器的 backgrounds）再切 active**。顺序反了会用「新 active 的 id」去比对 `find.editor`，导致旧缓冲高亮残留 |
| `close(index)`（关掉归属标签） | **收起** | 关缓冲**之前**先 `close_find` |
| `CloseScope`（关全部/关其它/关左侧…）批量关闭 | **收起**（只要目标集合包含 `find.editor`） | 同上；`apply_close_scope` 逆序关，需先判包含关系 |
| `remap_paths`（改名/另存为）**改变路径但不换编辑器实体** | **不收起** | 路径变化不改正文，`matches` 仍有效。若实现顺手 `set_language`/`set_text`，则按 `EditorEvent::Changed` 走一次重算 |
| `forget_paths` / `remap_paths` **关闭了归属缓冲** | **收起** | 同 `close` |
| `reload_buffer`（Revert File） | **不收起，但要重算 + 重新选中** | `set_text` 只在文本真的变了才发 `Changed`（`editor/state.rs:455-463` → `commit` 的 `before.text != …` 判据）。文本没变时 `Changed` 不发，但 `set_text` 会把 `selections` 重置为 `caret(0)`（`…:462`）⇒ **旧的选中丢失**。所以重载后若 `find` 开着，必须重算 `matches` 并重新选中 `current` |
| 会话恢复 `set_text` | 不涉及（此时没有查找条） | — |

### 2.8 命中计数与「无结果 / 正则错误」的呈现

**保留 FindWidget 现有三态**（`editor/find.rs:173-179`）：`i of N` / `N found` / `No results`（danger）。
补充一条**必须补的第四态**：

- **G3（缺口）**：空查询现在会落进 `Ok((_, 0))` 分支 → 显示 **`No results`（红字）**。
  刚 `Ctrl+F` 打开、什么都还没输入时显示「无结果」是错的（应为什么都不显示）。
  这不是文案问题，是状态缺一态。**建议改法（最小）**：`FindWidget::render` 里读一次
  `self.find.read(cx).text()`，`is_empty()` 时计数区**不渲染文本**（保留占位宽度以免布局跳动）。
  这样宿主不用多传参数，也不需要新增 `idle()` 构造器。
  → 与裁决 #3 的「FindWidget 文案 i18n 化」是同一批 lib 改动，建议合并做。
- 计数文案的三种形态（已 i18n 后）：`{n} found`（有命中但未选中）、`{i} of {n}`（选中中）、
  `No results`（0 命中）。
- **错误态（采纳 ui-designer §2.5，撤回我原先的写法）**：正则编译失败时，计数格只显示**短文案**
  `editor.find.invalidPattern`（danger 色），**不要**把 `find_all` 返回的原始 `regex` 报错塞进计数格
  ——那是英文、可能多行，会撑破 80px 的计数格（`editor/find.rs:173-179` 现在的实现是把
  `matches` 的 `Err(message)` 直接当计数文案，正是在犯这个错）。原始报错改挂 tooltip。
  这条同时修正了 `FindWidget` 的现有行为，属于裁决 #3 那批 lib 改动的一部分。
- **计数格的宽度预算（本批新增的硬约束，来自 ui-designer 只读实测）**：计数格宽固定 **80px**
  （`label_width() * 0.5`，`label_width()`=160px）、格内字 **11px**（`TextSize::Xs`）
  ⇒ 预算约 **13–14 个拉丁字符 / 7 个 CJK 字**。**这是「状态词」的预算，不是句子的预算**：
  该格与 `No results` / `5 found` / `2 of 5` 同宽同族，语义上就该只放**状态 token**。
  因此 `editor.find.invalidPattern` 的**各语言取值已定稿**（team-lead 确认，压进上述预算）：
  en/en-GB `Invalid regex` / de-DE `Regex falsch` / fr-FR `Regex invalide` / ja-JP `正規表現エラー` /
  he-IL `ביטוי שגוי` / zh-CN `正则表达式无效`（不变）；原始 `regex` 报错仍只挂 tooltip。
- **截断是安全网、不是方案**：`.truncate()` 只保证**不撑布局、不换行**；完整信息由 `Tooltip` 兜住。
  **不要**依赖截断来容纳长句——长句从一开始就不该进这一格（这是 IA 口径，不是排版口径）。
- **原始诊断只在 tooltip / 通知里**：`regex::Error` 原文（恒英文、可能多行）**永不**进计数格；
  与 G3（空查询不显示文本）一起，构成「**计数格 = 状态词**」的一致口径。

**G4（登记，非本批）**：`set_backgrounds` 是通用通道，当前只有 gallery 查找页在用（架构师 §2.6-2）。
一旦出现第二个用途（括号匹配 / 当前词高亮）就会互相覆盖。收敛点 `set_find_matches` 按裁决 #2 本批不加——
但**请在 `set_backgrounds` 的 Rustdoc 里加一句「当前被查找条占用，第二个使用者出现时必须收敛成专用 API」**，
免得后来者直接抢用。

---

## 3. 文件生命周期交互

### 3.1 菜单归属与可用性（裁决 #7）

**归属**：File 菜单。当前项序（`menu_bar.rs:705-763`，读码实测）：

```
New Tab(未接)  New Window  New File(未接)  Open Folder  Open File  Close Folder(未接)
───  Save   Save As(未接)   Save All(未接)   Revert File(未接)   Show Local History(未接)
───  Close Tab  Close Window(未接)
───  Close Others / All / Saved / Left / Right  Reopen Closed Tab  Quit(未接)
```

**观察（不改，只登记）**：`New Window` 夹在 `New Tab` 与 `New File` 之间，而 IDEA 把 `New *` 成组。
重排会**直接改动 `b2_actions_are_wired_and_only_bound_ones_carry_a_key` 的 `bound` 清单断言**
（`menu_bar.rs:2353-2374`，顺序敏感）。建议**本批不动项序**，重排另开单。

**可用性（裁决 #7：无活动缓冲 = no-op + 诊断）**

| 动作 | 无活动缓冲 | 有缓冲但无脏 | 说明 |
| --- | --- | --- | --- |
| `newTab` | **照常执行**（不需要活动缓冲） | 同 | |
| `newFile` | 无工作区根 → **通知 + 诊断**（见 §3.3），不算 no-op | 同 | 用 `lithe.files.openFolderFirst` |
| `saveAs` | no-op + 诊断 `reason=no-active-buffer` | 正常（未命名缓冲 → 走另存为；磁盘缓冲 → 写新路径） | no-op 语义 |
| `saveAll` | **no-op + 诊断**（`saved=0 reason=no-buffers`） | **成功 no-op**（`saved=0 skippedDirty=0`，**不报警告**） | 「没有脏缓冲」是成功，不是错误——这条区分要写进实现 |
| `revertFile` | no-op + 诊断 `reason=no-active-buffer` | 干净 → 直接重载；脏 → 走确认（§3.6） | |

**与「灰置」的差异，以及后续（F3 的展开）**

- 现状：`workbench::menu_bar::MenuItem` 只有 `Action / NotWired / Separator / Theme`（`menu_bar.rs:180-190`），
  **`Action` 没有 disabled 通道**。唯一「看起来不可用」的形态是 `NotWired`（带「未接入」徽标 + 通知），
  但它的语义是「还没实现」，不是「当前不可用」，不能借用。
- 所以按裁决 #7：菜单项保持正常显示、可点，点了 no-op。
- **F4 的后果必须正视**：`NotWired → Action` 之后，用户点 Save As（无缓冲）会**完全静默**，
  比现在「弹一条未接入通知」更差。
- **建议（在裁决 #7 的框内，仍算 no-op 语义，不改键位/不改菜单结构）**：
  1. 必须的：`eprintln!("S1_EDITOR_SAVE_AS skipped=no-active-buffer")` 类诊断；
  2. 建议补：一条**信息级**通知（`Severity::Info`），文案键**新增** `lithe.files.noActiveEditor`
     （en: `No editor is open` / zh-CN: `没有打开的编辑器`）。
     理由：不引入 disabled 通道、不改菜单结构，只把「静默死点」变成「一次性说明」，成本极低。
     是否采纳请主理人拍板；不采纳则退化为纯诊断日志（裁决 #7 的原义）。
- 后续（另开单）：给 workbench 的 `MenuItem::Action` 加 disabled 标志。**组件库的
  `MenuItem::disabled(bool)` 已存在**（`menus/model.rs:107`），视觉能力不用重画。

### 3.2 `newTab` 用户旅程（未命名 scratch，裁决 #5）

| 步骤 | 行为 |
| --- | --- |
| 1 | File → New Tab。**无默认键**（IDEA 的 New Scratch File 是 Ctrl+Alt+Shift+Insert；本批不绑，避免与 Insert 系列冲突，登记为后续） |
| 2 | 分配 `BufferKey::Untitled(n)`，`n` 为**每窗格自增、会话内不复用**的序号（不复用是刻意的：复用会让「关闭后再新建」拿到一个旧会话恢复过的键，行为难解释） |
| 3 | 标题 = `tr_args("lithe.files.untitledNumber", [("number", n)])` → 中文「未命名-1」、英文 `Untitled-1`（**键已存在**，无需新增） |
| 4 | 正文空、`is_dirty = false`、`writable = true`、语言 = Plain text（`resolved_language("未命名-1") == None`） |
| 5 | 成为 active；编辑器获得焦点；光标 (0,0)；**不显示未保存圆点**（新建即干净） |
| 6 | 再 New Tab → `未命名-2`；两个未命名缓冲**可并存**（这正是 `BufferKey` 方案 A 的目的） |
| 7 | 未命名缓冲**不进会话**、**不进 `closed` 列表**（ReopenClosedTab 无法有意义地恢复一个没有路径的缓冲） |
| 8 | 在未命名缓冲上按 `Ctrl+S`（`SaveBuffer` → `save_active`）：**必须转为另存为**，不是静默失败。这是「未命名缓冲」最容易漏的一条 |
| 9 | 另存为成功后：键从 `Untitled(n)` 换成 `Disk(path)`，标题/图标/语言按新文件名重算，脏位清除，标签位置不变，`schedule_session_save` |

### 3.3 `newFile` 用户旅程（项目内真实文件，裁决 #5 + #11）

| 步骤 | 行为 |
| --- | --- |
| 1 | 守卫：无工作区根 → **通知** `lithe.files.openFolderFirst`（「请先打开一个文件夹」，**键已存在**）+ 诊断，流程结束 |
| 2 | 平台对话框 `cx.prompt_for_new_path(&workspace_root, Some(default_name))`（先例：`settings/dialog.rs:3100` 的导出） |
| 3 | 默认名：建议 `NewFile`（无扩展名 → Plain text）。**不要写死 `NewFile.java`**：本仓没有「默认 Java 文件」这条产品决策，写死会把每个项目都拽进 Java 语境。取文件名的规则留给用户输入（对话框自带） |
| 3b | **树内入口不走平台路径选择器**：用库内 `PromptDialog`（`overlays/dialogs.rs:200-345`）——标题 `lithe.files.nameNewFile`（已有：`Name new file` / `命名新文件`）、字段 `label` 用 `lithe.files.enterNewFileName`（已有：`Enter the name for the new file:` / `输入新文件名称：`）、提交按钮 `lithe.files.newFile`（已有）、`check` 挂宿主校验（F5）。**零新增库 API，校验与错误显示都是现成的** |
| 4 | 落盘：**必须用 `OpenOptions::new().write(true).create_new(true)`**，不能用 `File::create`（后者会静默截断已存在文件！这是数据丢失级风险） |
| 5 | 已存在 → `AlreadyExists` → 通知（**新增键** `lithe.files.alreadyExists`：en `"{name}" already exists` / zh-CN `“{name}” 已存在`）。**不要复用 `lithe.files.javaClassAlreadyExists`**——那条文案带 Java 命名，用在通用新建上会误导（登记为命名问题）。若走树内 `PromptDialog`，这一条由 `check` 在**提交前**就拦住（错误显示在字段下方），不会走到落盘 |
| 6 | 其它错误（权限/路径） → 通知 `lithe.files.createFileFailedMessage`（`创建文件失败：{error}`，**键已存在**）+ 诊断 |
| 7 | 成功 → `open(&path)`；语言由 `language_for_file(name)` 推断（`editor/buffer.rs:27`） |
| 8 | 与 B1 树内「新建文件」共用**同一落盘入口** `create_file(dir_or_none, name)`：菜单路径把对话框返回的完整路径拆成 (parent, file_name) 再调它；树内路径由行内命名框提交后调它。UI 不同、落盘一条（裁决 #11） |
| 9 | 与 B2 `newTab` 的区别必须体现在用户旅程上：`newTab` 一步到位（立即出现一个干净标签，无对话框）；`newFile` 必有一次命名（因为要有真实路径） |

### 3.4 `saveAs` 流程

| 项 | 定稿 |
| --- | --- |
| 默认目录 | **磁盘缓冲**：该缓冲所在目录；**未命名缓冲**：工作区根；两者都不可用（无根且无路径）→ 交给平台默认目录 |
| 默认名 | **磁盘缓冲**：当前文件名；**未命名缓冲**：当前标题（`untitledNumber` 的结果，如 `未命名-1`）。**不改扩展名、不臆造 `.txt`**（不猜语言） |
| 选择器 | `cx.prompt_for_new_path(&dir, Some(default_name))` |
| 覆盖冲突 | **交给平台对话框**，不额外二次确认（裁决 #6） |
| 写盘 | 复用 `write_buffer` 的 `std::fs::write` 路径（`editor_view.rs:1396`）；成功后清脏位、回调 `buffer_write_host` |
| 原文件 | **保留**（裁决 #6）。旧文件保持上次保存的内容，只有缓冲改指新路径 |
| 成功后要改的键 | `Buffer.path / name / icon / language`；`last_text` 重键；`gutter_breakpoints` 重键；`closed` 列表内替换；标签 id 随 path 自动换；`schedule_session_save`。**统一走 `remap_paths`，不要在各处散改**（架构师 §4.4 的单一入口） |
| 查找条 | 路径变化**不收起**查找条（正文未变），见 §2.7 |
| **边界（必须定）** | **目标路径已被另一个标签打开** → 不能产生「两个标签同一路径」。v1 定：写盘成功后，**关闭当前缓冲并激活已存在的那个标签**（新写的内容已在磁盘上，关掉当前缓冲不丢数据），诊断 `duplicateTarget=true` |
| 失败 | 沿用 `write_buffer` 的失败链：`eprintln!("S1_EDITOR_SAVE_FAILED …")` + `record_notification(tr("lithe.editor.saveFailed"), Error)`（**键已存在**）。缓冲**不改路径**、脏位保持 |
| 菜单键位 | `secondary-shift-s`（IDEA 是 Ctrl+Shift+S；**注意同样用 `secondary-` 才是 mac 的 ⌘⇧S**）。裁决未涉及，本稿建议如此；若要减少本批键位数量，可先不给键（与 `newTab` 一致），登记为后续 |
| 其它三键 | `saveAll` = `secondary-alt-shift-s`（IDEA Ctrl+Alt+Shift+S，同为 `secondary-`）→ 建议同 `saveAs` 一并「先不给键」；`revertFile` **无默认键**（IDEA 也没有）；`newTab`/`newFile` 无默认键 |

> 说明：`saveAs/saveAll` 的键位裁决未列，本稿给出 IDEA 口径的建议值，但**推荐本批先不绑**
> （理由：本批的三条新键已足够覆盖高频；四个文件生命周期动作都是低频菜单动作，
> 少绑四个键就少四处冲突面与四处守卫改动）。请主理人在实现前确认。

### 3.5 `saveAll`

| 项 | 定稿 |
| --- | --- |
| 遍历范围 | 全部缓冲，按标签顺序 |
| 写入条件 | `is_dirty && writable` → `write_buffer` |
| 只读 + 脏 | 理论不可达（`on_text_changed` 对 `!writable` 直接 return，`editor_view.rs:1317-1319`）→ 真出现就**跳过 + 诊断** `skippedReadOnly=<n>`，不报错 |
| **未命名 + 脏** | **不能静默跳过**。v1：跳过并**通知一次**（`lithe.files.noActiveEditor` 不适用，**新增键** `lithe.editor.saveAllUntitled`：en `Save All left N untitled editor(s) unsaved. Use Save As.` / zh-CN `全部保存跳过了 N 个未命名编辑器，请对它们使用“另存为”。`，带 `{count}`），诊断 `skippedUntitled=<n>` |
| 全都不脏 | **成功 no-op**，`saved=0`，**不弹通知**（「没东西要存」不是问题） |
| 中途失败 | **首个失败即停**（与既有 `save_confirm` 的 Batch 分支一致：`editor_view.rs:1814-1816` 遇 `write_buffer == false` 直接 `return false`），通知 `lithe.editor.saveFailed`，诊断 `saved=<n> failed=<path>`。选「即停」而不是「尽力而为」是为了两条批量保存路径语义一致，重试故事好讲 |
| 诊断 | `S1_EDITOR_SAVE_ALL saved=<n> skippedUntitled=<n> skippedReadOnly=<n> failed=<0|1>` |
| 成功反馈 | 只靠**脏圆点消失**（每个成功标签）+ 诊断；**不弹成功 toast**（IDEA 不弹，见 §3.7） |

### 3.6 `revertFile` 的脏确认流程

| 步骤 | 行为 |
| --- | --- |
| 1 | 无活动缓冲 → no-op + 诊断 `reason=no-active-buffer` |
| 2 | 干净 → 直接复用 `request_reload(index, …)`（`editor_view.rs:1610-1623`）→ `reload_buffer`；诊断沿用 `S1_TAB_MENU run=reload path=…` |
| 3 | 脏 → `request_reload` 内部弹 `open_unsaved_dialog(PendingConfirm::Reload(index), body = tr_args("lithe.editor.gpui.unsavedChangesBody", name))` |
| 4 | 按钮：`Cancel`（`lithe.ui.cancel`）/ `Discard`（`lithe.editor.gpui.discardChanges`）/ `Save`（`lithe.ui.save`）——**沿用现有共享确认框**（`editor_view.rs:1725-1762`） |
| 5 | Cancel → 什么都不变（对话框关闭） |
| 6 | Discard → `reload_buffer`，丢弃编辑 |
| 7 | Save → `write_buffer` 成功后 `reload_buffer`（等于写回再读回，内容不变） |
| 8 | 重载后若查找条开着 → 重算 + 重选（§2.7） |
| **IDEA 差异（登记，建议本批接受）** | IDEA 的 Revert 只给 `Revert / Cancel` **两个**按钮，不给 `Save`（它的心智是「放弃当前编辑」而不是「保存或放弃」）。本稿**建议 v1 复用共享三键框**（少一处 UI 分叉、少一条文案），并把「Save-then-Revert 等价于空操作」这个怪异点写进诊断（`via=save`），后续若要贴 IDEA 再换成专用两键框。**这条偏离请主理人知悉** |
| 诊断 | `S1_EDITOR_REVERT path=<…> dirty=<true|false> confirm=<true|false> via=discard|save` |

### 3.7 保存成功 / 失败的用户反馈通道（按仓库现状）

| 情形 | 通道 | 现状 | 本稿建议 |
| --- | --- | --- | --- |
| 保存成功 | ① 标签脏圆点消失（`editor_view.rs:2529` 的 `dirty_dot`）② `println!("S1_EDITOR_SAVED path=… bytes=…")` ③ `buffer_write_host` 回调（explorer/git 刷新） | 已有 | **保持**。不要加成功 toast：IDEA 保存不弹提示，脏圆点是唯一且够用的反馈；加 toast 会变成噪音 |
| 保存失败 | `eprintln!("S1_EDITOR_SAVE_FAILED …")` + `record_notification(tr(failure_key), Severity::Error)`，键 `lithe.editor.saveFailed` / `lithe.editor.autoSaveFailed`（`editor_view.rs:1414-1429`） | 已有 | **统一沿用**：Save As / Save All / New File 的失败**都必须走 `record_notification`**，不得只打日志 |
| 无活动缓冲 | 目前无 | 缺 | 诊断必须；信息级通知按 §3.1 建议 |
| 只读文件被替换/写入 | 目前 `edit` 静默拒绝 | 缺 | 新增诊断 + 错误通知（走 §2.5） |

**结论**：`lithe_gpui_notify::Store`（`record_notification`）是**唯一的用户可见错误通道**；
诊断日志是**测试与排障通道**，两者都要，不可互相替代（`develop-lithe`「Handle failures explicitly」）。

---

## 4. explorer 右键菜单与重命名 / 删除交互

### 4.1 菜单结构与项序

**现状**：`row_menu_entries(kind)` 返回 5 项（`lithe-ui/src/lists/project/menu.rs:18-36`），
右键经 `ProjectTree::context_key`（`lists/project/mod.rs:408-427`）置 `open_menu`，
在指针处用 `ContextMenu::manual` 打开（`…:658-679`）。菜单项是**纯文本**（`MenuItem::new(label)`，`…:555`），
**不带图标** ⇒ B1 的图标对齐点不在这里（见 §6）。

**建议项序**（在架构师 §4.1 草案上补齐「增」，并说明每一处取舍）

```
File 行：
  Open
  ────────────
  New File                     ← 新增（裁决 #11）
  ────────────
  Rename                       ← 新增
  Delete                       ← 新增
  ────────────
  Reveal in File Manager / Copy Path / Copy Relative Path

Directory 行：
  New File                     ← 新增
  New Folder                   ← 新增
  ────────────
  Rename / Delete              ← 新增（Delete 在 Rename 之上，见下）
  ────────────
  Reveal in File Manager / Copy Path / Copy Relative Path
  ────────────
  Refresh
```

- **Delete 与 Rename 谁在上**：IDEA 项目视图里 `Delete` 在 `Rename` 之上。**待核实** IDEA Community
  `ProjectView` / `DeleteAction` / `RenameAction` 的注册顺序后再定稿（本稿按「Delete 在上」写，
  但请实现前对照真源确认，不要凭印象固化）。
- **「New」用平铺还是子菜单**：组件库 `MenuItem::submenu(label, menu)` **已有**（`menus/model.rs:80`）。
  子菜单更贴 IDEA（IDEA 的 New 是一个长列表）。但 v1 只有 File/Folder 两项，平铺更省一次点击。
  **建议 v1 平铺两项**，等文件模板体系进来再改子菜单。
- **文件行的 New File 落在哪**：选中一个文件时「新建」应落到**它所在的目录**（前端算 parent，交给宿主）。
  IDEA 也是这样（在文件的右键菜单里新建是可能的）。
- **`PROJECT_ACTIONS` 与 `ProjectLabels` 同步扩项**：`PROJECT_ACTIONS: [ProjectAction; 5]`
  （`menu.rs:39-45`）→ 8 项；`ProjectLabels`（`…:63-70`）加 `rename / delete / new_file / new_folder` 四个字段；
  `explorer_view.rs::row_labels()`（`:648-657`）填 `tr("lithe.files.rename")`、`tr("lithe.files.delete")`、
  `tr("lithe.files.newFile")`、`tr("lithe.files.newFolder")` —— **四个键全部已存在**（见 §4.4 的键清单），无需新增。
  `lists/project/tests.rs:223` 的一致性测试需同步（架构师 §7 已列）。
- **禁用态**：`MenuItem::disabled(bool)` 组件库已有（`menus/model.rs:107`）。建议把**根行（workspace 行）**的
  `Delete` / `Rename` 设 disabled（改/删工作区根没有意义），其余行不禁用。

### 4.2 行内重命名的进入 / 提交 / 取消

| 项 | 定稿 |
| --- | --- |
| 进入方式 | **两条都支持**：① 树有焦点时按 **F2**（IDEA 快路径）；② 右键菜单 `Rename`。理由：F2 是最快的键盘路径，菜单是可发现路径；IDEA 两者都有 |
| 未绑 | 裸 **`Delete` 键 v1 不绑**（只有菜单项）。理由：本批删除**无回收站**（裁决 #9：强确认 + 直接删），而树里误触 Delete 的概率不低；一个「删不掉但有确认」的键不如没有。后续接入 trash 端口后再讨论 |
| 也不绑 | `Shift+F6`（IDEA 的「Rename…」）不需要；`Alt+Insert`（IDEA 的新建）本批不绑（新建低频，且要与 Insert 系列一起规划） |
| 触发前的守卫 | 树有焦点 + 光标行非占位行 + 非工作区根 + 已在重命名中（`renaming.is_some()`）→ 忽略重复触发（通用 `Tree` 的做法，`lists/tree/mod.rs:329-331`） |
| 初值 | 该行的显示名（对文件：完整文件名含扩展名；对目录：目录名） |
| 初值选中范围 | **建议只选主名，不选最后一个扩展名**（IDEA 口径：改 `Foo.java` 时打字得到 `Bar.java`，不是 `Bar`）。**这是 Q2**：`Editing::begin` 现在全选（`forms/inline.rs:34-35`），要么给它加范围参数，要么 `ProjectTree` 自己造 field 时手动 `select(0..stem_len)`。若不做，用户每次改文件名都要手动补扩展名 |
| 提交 | `Enter`（`InputEvent::Submit`）→ `Editing::finish(false)` → `on_rename(path, new_name)` |
| 提交 | **失焦也提交**（`InputEvent::Blur`，`forms/inline.rs:44-47`）——与通用树一致，不要在这条路径上做例外 |
| 取消 | `Esc` → `Editing::finish(true)`，且**必须 `stop_propagation`**（`~` `forms/inline.rs:132-140` 的做法），否则会继续冒泡给别人。此处**不会**与 MenuBar 的 Escape 拦截器打架：右键点开菜单选 Rename 时菜单已经关闭，进入编辑态时没有菜单开着 |
| 取消（续） | 编辑态下再按一次 F2 / 切走行 → 同取消语义（由 `Editing::finish` 的既有路径决定，实现时确认） |
| `Tab` | 不特殊处理（`ProjectTree::on_key_down` 没有 `tab` 分支，`lists/project/mod.rs:456-481`；`ElyInput` 也没绑 `tab`），落到默认焦点遍历即可。**不做提交**（避免误提交） |
| 提交后 | 宿主执行 `std::fs::rename`；成功后 ① `remap_paths(old,new)` 级联 ② `Explorer::refresh`（重读 snapshot + git status）③ **`reveal(new_relative)` + 选中新行**（IDEA 改名后焦点留在原地），诊断 `S1_EXPLORER rename from=<…> to=<…> result=applied` |

### 4.3 校验与「校验失败的内联表现」

**校验规则（全部放宿主，因为要读文件系统）**

| 规则 | 失败文案建议 |
| --- | --- |
| 非空（`trim()` 后非空） | `lithe.files.invalidFileName`（已存在：「文件名无效」） |
| 不含路径分隔符 `\` `/` | `lithe.files.invalidFileNameCharacters`（已存在：「文件名无效：不允许路径穿越和特殊字符」） |
| 无 Windows 保留名（`CON/PRN/AUX/NUL/COM1..9/LPT1..9`）、无尾随点 / 尾随空格 | 同上 |
| 同目录下不重名（按目标平台大小写规则；Windows/macOS 默认大小写不敏感） | **新增键** `lithe.files.nameExists`：en `"{name}" already exists in this folder` / zh-CN `此文件夹中已存在“{name}”` |
| 改名前后同名（大小写敏感平台上的纯大小写改名要允许） | 视为合法，直接提交（不做变更） |

**内联表现（推荐方案，即 Q1）**

> **先看库内已有的同形做法（F5）**：`PromptDialog::check(impl Fn(&str) -> Result<(), SharedString>)`
> （`overlays/dialogs.rs:246`）每次渲染都跑校验、`verdict.is_ok()` 决定提交按钮 `disabled`、
> Enter / 提交被 `held back`（`:283-289`）、**原因显示在字段下方**（文档注释 `:201`）。
> 也就是说「校验返回 `Err(SharedString)` → 不提交 → 就地说明原因」**已经是组件库的既有约定**，
> 不是本批新发明的模式。内联重命名只是把同一形态搬到树行里。

- **字段不关闭**：提交路径**不走** `Editing` 的自动 `Submit→finish`，而是由 `ProjectTree` 自己订阅
  `InputEvent::Submit` → 先跑校验回调 → `Ok` 才 `finish(false)`、`Err(msg)` 则**保持编辑态**。
- **行内提示（配色与布局已按 ui-designer 实测定案，见视觉稿 §10）**：
  - **定稿：红框 + `colors.fg` 文字**。该行的 `Input` 用 `.invalid(true)`（**组件库已支持**，
    `forms/input.rs:60-62, 110-111`）——它只负责把**边框**画成 `colors.danger`，**文字仍用 `colors.fg`**。
  - **为什么不用 danger 红字**：正在重命名的行是**选中行**，行底是 `colors.active`（`row.rs:221`）；
    `colors.danger` 文字压在 `colors.active` 上，浅色只有 **4.44:1**（差 0.06 不到 AA），深色 4.78:1；
    而 `colors.fg` 是 **14.6:1 / 12.6:1**（均远超 AA）⇒ 文字通道留给 `colors.fg`。
  - **红框的达标依据（关键，评审会追问，写清可挡）**：红框要按**两个相邻面**分别核（视觉稿 §10.6）：

    | 相邻面 | 浅 | 深 | 门槛 |
    | --- | --- | --- | --- |
    | 框 vs 输入框自身填充 `colors.surface` | **5.48:1** | **5.93:1** | 内部相邻，本就高 |
    | **框 vs 所在行底 `colors.active`** | **4.44:1** | **4.78:1** | ✅ **≥3:1（WCAG 1.4.11 非文本）** |

    **真正要看的是第二行**（控件与其背景的相邻关系，而非控件自身填色）。而这套方案成立的**原因**就在这里：
    `colors.danger` 对行底 **4.44:1** ——**同一个数值**，挂在**文字**通道上差 0.06 不到 AA 的 4.5:1，
    挂在**非文字**通道上却以 **4.44 ≥ 3** 稳稳过 1.4.11。
    ⇒ **把红色从文字搬到描边，是换了一条门槛更低的通道**，不是「近似达标」，也不是侥幸。
  - **一句话口径**：**红框达标 = 「框 vs 行底 4.44:1 ≥ 非文本门槛 3:1」**。将来复核若算出 4.44，
    不要误判「红框不合格」——**4.5:1 是文字门槛，3:1 才是非文本门槛**。
  - **各归各的通道**：红框是**非文字**视觉指示（受 WCAG 1.4.11 管辖，门槛 3:1），错误说明**文字**
    由 `colors.fg` 承载（受 AA 4.5:1 管辖）——两条需求同时满足，不必二选一。
  - **若主理人坚持红字**：保留红字可接受，但需在验收里**如实记录浅色 4.44:1 差 AA 0.06**（视觉稿 §10.6 已登记）。
    **补充实测（ui-designer 本轮新量）**：只有 `colors.active` 这**一档**不达标，其余行底都过——
    未选中悬停 `colors.hover`(`#f2f0ed`) 上 danger = **4.81:1** ✅、面板白底 danger = **5.48:1** ✅；
    而**主路径（先选中再重命名）恰好落在 `colors.active` 这一档**（4.44:1 ❌）。
    即这不是边角态、是**默认态**——这正是本稿坚持 `colors.fg` 的原因。
  - **禁止**：不要走「重命名时把行底换成 `hover`」这条路——会削弱「我正在编辑哪一行」的信号
    （选中态 `active` 本身就是这一个信号，换成 `hover` 等于把它关掉）。
  - **布局（token 化，与视觉稿 §10 同一口径；已采纳实现里更准的一处）**：外框
    `flex items_center h(control_height(Size::Small))`；**输入框 `flex_1 min_w_0`** +
    `Input::new(&field).size(Size::Small).invalid(true)`；行内提示文案与输入框**同一 flex 行**、
    放在其**右侧**：**`flex_none max_w(relative(0.5)) truncate whitespace_nowrap overflow_hidden pl_1p5 text_size(Xs)`**，
    文案用 `colors.fg`；空间不够时**文案先截断**（`Ellipsis`），完整文案挂 `Tooltip`。
    > 注：**不**用「两边都 `flex_1`」的对半分配——那样文案短时输入框也只能拿一半。
    > 改成「输入框 `flex_1` + 文案 `flex_none max_w(50%)`」后：**文案短则输入框多拿宽度**，
    > 文案长则由 `max_w(50%)` 封顶、行内截断。这是实现比我原稿更准确的一处，已采纳。
  ⚠ **硬约束**：`ProjectTree` 用 `uniform_list`（固定行高，`lists/project/mod.rs:651-657`），
  **不能**在行下面加第二行错误文案——那会破坏行高一致性。这正是它与 `PromptDialog` 唯一必须不同的地方：
  `PromptDialog::check` 的 `Result<(), SharedString>` 画在字段**下方**（`dialogs.rs:246/251`），
  **行内必须横向**，两处不要互相照抄。所以错误画在**同一行内**（右侧剩余空间，行尾本来就有 git 状态字母的位置）。
  - ⚠ **落地方向校验（验收必查；偏差、非设计变更）**：本轮 land 的 `lists/project/row.rs` 把提示文字写成了
    **`colors.danger`**，与本节的 **`colors.fg` 不符**（红框 `.invalid(...)` 是对的）。按上面实测，
    选中行（**主路径**）上 danger 文字浅色只有 **4.44:1**（差 AA 0.06）。**验收时须确认已改回 `colors.fg`**
    （ui-designer 已报 rust-core-dev；视觉稿 §10.6.1 已登记）。
- **文案来源**：`Err(SharedString)` 由宿主给的校验回调生成（用 §上表的键）。
- **新增库 API（Q1，需授权；形态照抄 `PromptDialog::check`）**：
  ```rust
  // lithe-ui: lists/project/mod.rs
  /// 行内重命名提交前的校验：给 (节点路径, 新名字)，Err 时保持编辑态并把消息画在行内。
  pub fn on_rename_check(self, check: impl Fn(&str, &str) -> Result<(), SharedString> + 'static) -> Self;
  ```
- **保底方案（不新增库 API）**：`Err` 时 ① 记一条错误通知（toast，文案同上）② 通过架构师已规划的
  `begin_rename(&mut self, key, …)`（把它从 `pub(crate)` 提到宿主可用）**重开编辑态并保留用户输入**。
  用户看到的是「toast 说了原因 + 字段还开着、名字没丢」。**体验弱于内联，但不改库公开面**。
- **另一条保底（新，零新增 API）**：重命名**也**改用 `PromptDialog` + `check`（库内现成）。
  校验与错误显示完全免费，代价是**偏离 IDEA 的行内口径**（裁决 #10）。本稿不建议，但成本最低，供主理人取舍。

**New File / New Folder 的校验（顺带）**：树内入口用 `PromptDialog` + `check`，把同一套校验函数挂上去
（`check` 的签名正好是 `Fn(&str) -> Result<(), SharedString>`）⇒ 命名非法时**提交按钮置灰 + 字段下方显示原因**，
不需要额外的内联设计。这是 F5 的直接收益。

### 4.4 删除确认对话框

**交互**

| 步 | 行为 |
| --- | --- |
| 1 | 右键 `Delete`（v1 作用于**右键的那一行**；多选删除需要多路径端口，架构师的 `DeleteHost = Fn(&Path, …)` 是单路径 ⇒ **本批单选**，多选删除另开单） |
| 2 | 工作区根行：菜单项 disabled（§4.1），不可达 |
| 3 | 弹确认对话框：**直接复用库内 `ConfirmDialog`**（`lithe-ui/src/overlays/dialogs.rs:113-193`，F6），不手搓 `Dialog` + 自建按钮 |
| 4 | **标题**：复用 `lithe.ui.delete`（「删除」/`Delete`）——**已存在** |
| 5 | **正文**：`ConfirmDialog::new(id, title, message, on_close)` 的 `message`（文件与目录分开取 §下的新键），措辞要**点名对象 + 明说不不可撤销**（危险操作措辞规范） |
| 6 | `Cancel`：`ConfirmDialog` 自带，用组件库自己的 `i18n::text(cx, "dialog.cancel", …)`（`dialogs.rs:175`，组件库 `locales/ui.yml:41` 已有） |
| 7 | 确认按钮：`.confirm(tr("lithe.ui.delete"))` + `.destructive()` → 自动用 `ButtonVariant::Danger`（`dialogs.rs:151, 166-170`） |
| 8 | **Enter 不得是删除**：`Dialog::on_enter` 只在**有 `focus_first` 字段时**才生效（`overlays/dialog.rs:90-92, 196-200`），`ConfirmDialog` 没有输入框 ⇒ **Enter 不带默认动作**，Esc = 关闭（`…:311-314`）。**要求**：不要给 `ConfirmDialog` 加 `on_enter` 指向删除按钮（库内它也没有这个口子，天然安全） |
| 9 | 确认后：`std::fs::remove_file` / `remove_dir_all`（目录递归）。**先写诊断再执行**，便于失败定位 |
| 10 | 成功后：`forget_paths(target)`（关掉该子树上所有缓冲，脏缓冲先走未保存确认）→ `Explorer::refresh` → **选中父行**（被删行消失了，焦点要有去处），诊断 `S1_EXPLORER delete path=<…> removed=<n>` |
| 11 | 失败：通知（**新增键** `lithe.files.deleteFailed`：en `Failed to delete "{name}": {error}` / zh-CN `删除“{name}”失败：{error}`）+ 诊断；**不刷新**（保持用户看到的状态与磁盘一致） |

**新增文案键（建议键名 + 中英字串）**

| 键 | en-US | zh-CN | 用途 |
| --- | --- | --- | --- |
| `lithe.files.deleteFileConfirm` | `Delete the file “{name}”? This cannot be undone.` | `删除文件“{name}”？此操作无法撤销。` | 文件删除正文（`{name}` 走 `tr_args`） |
| `lithe.files.deleteFolderConfirm` | `Delete the folder “{name}” and everything inside it? This cannot be undone.` | `删除文件夹“{name}”及其全部内容？此操作无法撤销。` | 目录删除正文 |
| `lithe.files.deleteFailed` | `Failed to delete “{name}”: {error}` | `删除“{name}”失败：{error}` | 删除失败 |
| `lithe.files.nameExists` | `“{name}” already exists in this folder` | `此文件夹中已存在“{name}”` | 重命名重名 |
| `lithe.files.alreadyExists` | `“{name}” already exists` | `“{name}”已存在` | New File 目标已存在 |
| `lithe.files.noActiveEditor` | `No editor is open` | `没有打开的编辑器` | 无活动缓冲的信息提示（§3.1，可选） |
| `lithe.editor.saveAllUntitled` | `Save All left {count} untitled editor(s) unsaved. Use Save As.` | `全部保存跳过了 {count} 个未命名编辑器，请对它们使用“另存为”。` | §3.5 |

**可复用的既有键（不要重复新增）**：`lithe.ui.delete`、`lithe.ui.rename`、`lithe.ui.cancel`、`lithe.ui.save`、
`lithe.files.rename`、`lithe.files.delete`、`lithe.files.newFile`、`lithe.files.newFolder`、`lithe.files.untitled`、
`lithe.files.untitledNumber`、`lithe.files.fileNamePlaceholder`、`lithe.files.folderNamePlaceholder`、
`lithe.files.renameItem`、`lithe.files.enterNewFileName`、`lithe.files.enterNewFolderName`、
`lithe.files.invalidFileName`、`lithe.files.invalidFileNameCharacters`、`lithe.files.createFileFailedMessage`、
`lithe.files.openFolderFirst`、`lithe.menu.saveAsTitle`、`lithe.menu.saveFileFailed`、
`lithe.unsavedChanges.title`、`lithe.editor.gpui.unsavedChangesBody`、`lithe.editor.gpui.unsavedChangesBatchBody`、
`lithe.editor.gpui.discardChanges`。
（均在 `rust/lithe-gpui/crates/shared/locales/lithe.{en,zh-CN}.yml`；上面标「已存在」的都已逐条核对到行。）

**是否需要进系统回收站**：裁决 #9 已定 v1 直接删。本稿补充**交互层**的兜底要求：
① 正文必须点名对象；② 必须写明不可撤销；③ 确认按钮用 danger 且**不是**默认键；
④ 失败必须可见（通知）。这四条是「无回收站」前提下不可削减的部分。
`Delete` 端口要留成可替换点（架构师已定），将来接 trash 时只换实现、不动交互。

### 4.5 改名 / 删除后的选择与刷新

| 操作 | 选中 | 滚动 | 刷新 |
| --- | --- | --- | --- |
| Rename | **选中改名后的新行**（不是父行、不是清空选择） | 滚进视野（`reveal` 已经会展开祖先） | `Explorer::refresh`（snapshot + git status），随后 `reveal(new)` |
| Delete | **选中父行**（同级、同目录的第一个兄弟行也可以，但父行更可预期） | 滚到可见 | `Explorer::refresh` |
| 若 affected 缓冲被关 | 编辑区 active 切到相邻标签（由 `close(index)` 的既有语义决定），查找条按 §2.7 收起 | — | `schedule_session_save` |

---

## 5. 验收断言清单（可观测交互结果，供实现阶段写集成测试）

约定：**A** 类 = 直接观察状态/API（最稳）；**V** 类 = 用户可见表现（截图/探针）；
**D** 类 = 诊断行（辅助定位，不作为唯一判据）。

### 5.1 B3 查找 / 替换

| # | 断言 | 类型 |
| --- | --- | --- |
| B3-1 | 有活动缓冲时按 `secondary-f` → `pane.find.is_some()`，`showing_replace == false`，焦点在 `query` 输入框 | A |
| B3-2 | 按 `secondary-r` → `find.is_some()`，`showing_replace == true`，焦点仍在 **query**（不是 replace） | A |
| B3-3 | 打开前编辑器有单行非空选区 → `query.text()` == 选区文本 | A |
| B3-4 | 打开前编辑器有跨行选区 / 多光标 → `query.text()` 为空 | A |
| B3-5 | 无活动缓冲时按 `secondary-f` → `find.is_none()` 且 `D: reason=no-active-buffer` | A+D |
| B3-6 | 有活动对话框时按 `secondary-f` → `find.is_none()` 且 `D: reason=active-dialog` | A+D |
| B3-7 | 查询命中 3 处 → 编辑器 `backgrounds` 恰好 3 条，`current` 处的不透明度 > 其余（`0.45` vs `0.2`） | A |
| B3-8 | 命中 0 处 → `backgrounds` 为空，计数区不为空且为 danger 色（V） | A+V |
| B3-9 | 空查询 → 计数区**无文本**（G3 修好后），且 `backgrounds` 为空 | A+V |
| B3-10 | 输入非法正则 `(` → `error.is_some()`、`backgrounds` 清空、上/下一处按钮禁用（V） | A+V |
| B3-11 | `F3` 三连：`current` 走 `0→1→2→0`（环绕），每次选区 == `matches[current]` 且**滚到视口中心**；视觉基线用**合成对比度** 深 **3.37:1** / 浅 **2.09:1**（**不**要求单靠选区洗色达 3:1，见 §6-B5） | A+V |
| B3-12 | `Shift+F3` 在 `current == 0` → `current == len-1`（反向环绕） | A |
| B3-13 | 查询框内按 Enter → 同「下一处」 | A |
| B3-14 | 查找条未开时按 `F3` → 编辑器文本/选区/背景均不变，`D: step=false reason=closed` | A+D |
| B3-15 | 替换当前处 → `text()` 变化、`is_dirty == true`、**一次撤销**即回到原文（`undo` 一次后 `text()` == 原文本） | A |
| B3-16 | 替换全部（3 处）→ `text()` 变化、`is_dirty == true`、**一次撤销**即回原文（证明是单批次 edit） | A |
| B3-17 | 只读缓冲上替换 → `text()` 不变，且通知 store 里新增一条 error | A |
| B3-18 | 关闭（✕ / Esc / `close_find`）→ `find.is_none()`、`backgrounds` 为空、**焦点 == 该编辑器的 focus handle**、编辑器选区保留 `matches[current]` | A |
| B3-19 | 查找条开着切标签 → `find.is_none()`，且**旧标签的 `backgrounds` 为空**（I5，顺序 bug 的检测点） | A |
| B3-20 | 查找条开着点标签栏切到别的标签 → 同上 | A |
| B3-21 | 关掉归属标签 → `find.is_none()`（且不 panic） | A |
| B3-22 | 多光标状态下按 `F3` → 光标收成 1 个且 == `matches[current]`（写进契约的行为） | A |
| B3-23 | 编辑器有焦点、查找条开着时按 Esc → **查找条仍在**，且多光标被 `SingleCursor` 收起（§1.4 契约） | A |
| B3-24 | 菜单开着 + 查找条开着时按 Esc → 菜单关闭且**查找条仍在**（优先级契约） | A |
| B3-25 | 连续快速输入 20 个字符 → 最终 `matches` 与一次性输入同一查询的结果一致（debounce 代次无泄漏） | A |
| B3-26 | `Revert File`（文本真变了）后查找条仍开 → `matches` 已重算且 `current` 处重新被选中 | A |

### 5.2 B2 文件生命周期

| # | 断言 | 类型 |
| --- | --- | --- |
| B2-1 | `New Tab` → 新缓冲 `key == Untitled(n)`、`name == “未命名-n”`（按当前 locale）、`is_dirty == false`、语言 Plain text、成为 active、编辑器有焦点 | A |
| B2-2 | 连按两次 → 得到 `未命名-1`、`未命名-2` 两个**并存**标签（去重不把它们合并） | A |
| B2-3 | 未命名缓冲上 `Ctrl+S` → **打开另存为**（不静默失败），`D: saveAs reason=untitled` | A+D |
| B2-4 | 未命名缓冲不写入会话：`schedule_session_save` 后的会话内容里只有磁盘路径 | A |
| B2-5 | 关掉未命名缓冲后 `ReopenClosedTab` → 不恢复它（`closed` 不含未命名键） | A |
| B2-6 | `New File` 无工作区根 → 出现 `lithe.files.openFolderFirst` 的通知，无文件被创建 | A+V |
| B2-7 | `New File` 目标已存在 → 已存在文件**内容不变**（`create_new` 生效），出现 `alreadyExists` 通知 | A |
| B2-8 | `New File` 成功 → 磁盘上出现 0 字节文件，且以新标签打开、语言按文件名推断 | A |
| B2-9 | `Save As`（未命名）→ 默认目录 == 工作区根、默认名 == 当前标题 | A |
| B2-10 | `Save As`（磁盘缓冲）→ 默认目录 == 该文件所在目录、默认名 == 原文件名 | A |
| B2-11 | `Save As` 成功后：缓冲 `path/name/icon/language` 全部指向新路径；**原文件仍在磁盘上且内容不变**（裁决 #6） | A |
| B2-12 | `Save As` 后：`gutter_breakpoints`、`last_text` 的键已迁到新路径（旧路径无残留）| A |
| B2-13 | `Save As` 到**已打开**的路径 → 不出现两个同路径标签，`D: duplicateTarget=true`，编辑区 active 指向已存在的那个 | A+D |
| B2-14 | `Save All`：2 个脏磁盘缓冲 → 两个文件都被写、两个脏圆点都消失、`D: saved=2` | A+V |
| B2-15 | `Save All` 全都不脏 → 无通知、`D: saved=0`（成功 no-op，不误报） | A+D |
| B2-16 | `Save All` 有 1 个脏未命名缓冲 → 出现 `saveAllUntitled` 通知，其余脏磁盘缓冲仍被保存 | A+V |
| B2-17 | 无活动缓冲时 `Save As`/`Save All`/`Revert File` → 无 panic、无副作用，`D: reason=no-active-buffer` | A+D |
| B2-18 | `Revert File` 干净缓冲 → 立即重载（`D: run=reload`），无对话框 | A+V+D |
| B2-19 | `Revert File` 脏缓冲 → 出现未保存确认框（标题 `lithe.unsavedChanges.title`）；`Cancel` 后文本与脏位不变 | A+V |
| B2-20 | 同上选 `Discard` → 文本 == 磁盘内容、`is_dirty == false` | A |
| B2-21 | 保存失败（只读目录）→ 通知 store 新增一条 error + `D: S1_EDITOR_SAVE_FAILED`，且**脏位保持**、缓冲路径不变 | A+D |
| B2-22 | 保存成功 → **不新增成功类通知**（只有诊断 + 脏圆点消失） | A+V |
| B2-23 | 菜单守卫：`b2_actions_are_wired_and_only_bound_ones_carry_a_key` 通过；`Find`/`FindAndReplace` 的第三列由 `false` 变 `true`；`bound` 清单含 `secondary-f`、`secondary-r` | A |
| B2-24 | `Find` / `FindAndReplace` 的菜单键位胶囊在 macOS 渲染为 ⌘F / ⌘R，Windows 渲染为 Ctrl+F / Ctrl+R（`KbdCombo` 按平台） | V |
| B2-25 | `F3` / `Shift+F3` **不出现**在任何菜单行的胶囊上（无 `MenuAction`、无菜单项） | A+V |

### 5.3 B1 文件增删改名

| # | 断言 | 类型 |
| --- | --- | --- |
| B1-1 | `row_menu_entries(File)` == `[Open, —, NewFile, —, Rename, Delete, —, Reveal, CopyPath, CopyRelativePath]` | A |
| B1-2 | `row_menu_entries(Directory)` 含 `NewFile`/`NewFolder`（在首段）与 `Rename`/`Delete`/`Refresh` | A |
| B1-3 | `PROJECT_ACTIONS` 覆盖全部 `ProjectAction` 变体，且每个变体都有 `label()` 映射（一致性测试同步） | A |
| B1-4 | 工作区根行：`Rename`/`Delete` 为 disabled（点了无反应），其余可用 | A+V |
| B1-5 | 树有焦点时按 `F2` → 该行进入行内编辑，字段有焦点 | A+V |
| B1-6 | 右键 `Rename` → 同行内编辑 | A+V |
| B1-7 | 编辑态按 `Enter` → 磁盘上文件被改名、`D: rename result=applied`、**选中改名后的新行** | A+D |
| B1-8 | 编辑态按 `Esc` → 文件**未**改名、字段关闭、且没有其他动作被 Esc 触发（`stop_propagation` 生效） | A |
| B1-9 | 编辑态失焦 → 等同 `Enter`（提交） | A |
| B1-10 | 改名成已存在的名字 → **文件未被改名**、编辑态**保持打开**、行内出现重名提示（或保底方案下出现 toast 且字段重开） | A+V |
| B1-11 | 改名成 `a/b`、`a\b`、`..`、`CON`、`name.`（尾随点）→ 全部拒绝，文件不动 | A |
| B1-12 | 文件改名后：相关标签的路径/名字/图标/语言全部更新；`JumpHistory`、`gutter_breakpoints`、`last_text`、`closed` 均指向新路径；会话已重存 | A |
| B1-13 | 目录 `a/b` 改名 → `a/bc` 的缓冲**不受影响**（路径**组件**前缀匹配，不是字符串前缀） | A |
| B1-14 | 鼠标拖拽改名（预留的 `on_move`）不会与行内编辑互相抢 `Editing` 状态 | A |
| B1-15 | 右键 `Delete` → 出现确认框，标题 `lithe.ui.delete`、正文按文件/目录取对应新键且含 `{name}`、按钮为 `Cancel` + danger `Delete` | A+V |
| B1-16 | 确认框里按 `Enter` → **不执行删除**（没有默认动作）；按 `Esc` → 关闭且不删除 | A |
| B1-17 | 确认删除文件 → 磁盘文件消失、该文件标签被关闭、explorer 刷新、**选中父行** | A+V |
| B1-18 | 确认删除目录 → 整棵子树从磁盘消失、子树内所有标签关闭（脏的先走未保存确认） | A |
| B1-19 | 删除失败（权限）→ 新增 `deleteFailed` 通知 + 诊断，**explorer 不刷新** | A+V |
| B1-20 | 树内 `New File` → 行内命名框；提交合法名 → 磁盘出现空文件 + explorer 刷新 + 选中新行 | A+V |
| B1-21 | 树内 `New File` 与菜单 `New File` 走**同一**落盘入口（`create_file`），两条路径产生同样的磁盘结果与诊断前缀 | A |
| B1-22 | 树内 `New Folder` → 创建目录（不是文件） | A |
| B1-23 | 建文件/文件夹名非法 → 拒绝且不落盘；无工作区根时给出 `openFolderFirst`（若树内入口也校验） | A |

---

## 6. 需要 ui-designer 对齐的点（图标与图标位）

> **对齐状态（已闭环）**：ui-designer 已产出 `reports/editor-basics-visual-spec.md`，
> 其 **§10** 逐条答复了我在本节提出的 5 个问题，其中 **2 条修正了我的前提**（A1 的邻居关系、A2 的相邻性），
> 已在下文改正。本节只记交互层视角的结论；配色与对比度的**实测数据以视觉稿 §10 为准**。

**A. 菜单栏（`idea::` 体系）——本批会新增/变更的行**

| 位置 | 图标 | 现状 | 说明 |
| --- | --- | --- | --- |
| `MenuAction::Find`（Edit 菜单） | `idea::MAGNIFYING_GLASS_ICON` | **已是**（`menu_bar.rs:473`） | 不变 |
| `MenuAction::NewFile`（File 菜单） | `idea::FILE_PLUS_ICON` | 现挂在 `NotWired{newFile}`（`:712-715`） | 转成 `Action` 后沿用 |
| `MenuAction::NewTab`（File 菜单） | `idea::FILE_PLUS_ICON` | 现挂在 `NotWired{newTab}`（`:705-709`） | 与 `NewFile` **不相邻**（中间隔着 `New Window`），见 A2 |
| `MenuAction::RevertFile`（File 菜单） | `idea::REFRESH_ICON` | 现挂在 `NotWired{revertFile}`（`:738`） | ⚠ 语义借用（「刷新」≠「还原文件」），见 A3 |
| `MenuAction::FindAndReplace`（Edit 菜单） | 保持 `MenuIcon::Empty`（`:507`） | — | 见 A1（**结论已定**） |
| `MenuAction::SaveAs` / `SaveAll`（File 菜单） | 保持 `MenuIcon::Empty`（`:495-517` 分组内） | — | 见 A3（**结论已定**） |

**A1（结论：保持 `MenuIcon::Empty`，理由已按实测改正）**

我原先的理由（「空槽混在一堆空槽里不显眼」）**不准确**，ui-designer 实测改正：
`Find` 有放大镜（`:473`）、`FindAndReplace` 空（`:507`），两者在 Edit 菜单里**直接相邻**（`:779-780`，同组无分隔）
⇒ 真实画像是「**一对兄弟动作，一个有图、一个空槽**」的不对称。

结论不变、理由换成三条**硬理由**：
1. 库内规则就是「**我们没画过这张图**」（`menu_bar.rs:371-375` 的注释原文）；
2. `IdeaIcon`（186 条精选子集）里**确实没有** find/replace 图；
3. `develop-lithe` 的「未指派图标保留空槽」。
⚠ **禁止拿别的图顶替**：不能借 `MAGNIFYING_GLASS_ICON`（Replace ≠ Search），
`MAGNIFYING_GLASS_PLUS/MINUS` 已被 `ZoomIn/ZoomOut` 占用（`:481-482`）。

**A1b（建议登记的粒度：不是一条 FindAndReplace 单，而是一条「菜单图标补齐」单）**

同样的不对称在 Edit/File 菜单里是**系统性**的：
`Cut`/`Copy` 有图（`:471-472`）vs `Paste`/`SelectAll` 空（`:505-506`）；
`Save`/`RevertFile` 有图（`:456`/`:738`）vs `SaveAs`/`SaveAll` 空。
⇒ 单条登记会把一个集合问题写成个案。建议登记为一条「**`IdeaIcon` 精选子集缺口 → 菜单图标补齐**」的后续单。

**A2（结论：保持现状 + 登记，**并且不要移动 `New Window`**）**

我原先说 `NewTab` 与 `NewFile`「两行相邻」**是错的**。ui-designer 实测 File 菜单顺序（`:706-716`）：

```
newTab (FILE_PLUS)  →  NewWindow (Empty)  →  newFile (FILE_PLUS)
```

中间隔着 `New Window`，所以我担心的「紧挨着分不清」比预想轻。
**并且**：把 `New Window` 移出 New 组会让两者**真的相邻、把问题加重** ⇒
**明确不要移动**（顺带这也规避了我原先担心的「动 `MENUS` 项序会碰顺序敏感的守卫断言」）。
`FILE_PLUS_ICON`（`legacy/ui/addFile.svg`，深浅齐）对 `NewTab` 在语义上是「新文档」，可接受；
两者都改 `Empty` 属倒退（把可识别性一起丢掉）。IDEA 的真解是 `New` **子菜单** → 独立单。

**A3（结论：另开单，含两条验收约束）**

`SaveAs` / `SaveAll` 本批变真菜单项后仍是空槽。另开单时：
1. ⚠ **不要把 `FLOPPY_DISK_ICON` 复用给 `SaveAs`/`SaveAll`**：三个动作共用一个软盘比空槽**更糟**
   （分不清「保存 / 另存为 / 全部保存」）；要补就补**同族带角标**的变体。
2. 顺手登记：`revertFile` 用 `REFRESH_ICON`（`:738`）是一处**语义借用**（「刷新」≠「还原文件」）。

**B. 与视觉稿一致的地方 + 只有交互层才会产生的确认项**

1. **查找条内部控件**全部走组件库 `IconName`（`editor/find.rs:213-288`）：`ChevronDown/ChevronRight`、
   `CaseSensitive`、`WholeWord`、`Regex`、`ArrowUp/ArrowDown`、`X`。与菜单栏的 `idea::` 是**两套体系**，
   不要互串 —— 与 ui-designer §3.2 的判定一致，无需再对。
2. **`editor.find.*` 文案键**：ui-designer §2 与我 §2.3/§2.8 都要求走组件库 `i18n::text`（裁决 #3 方案 A）。
   我方额外提一条**交互缺口需合并进同一批 lib 改动**：**空查询态**（刚打开、什么都没输入）
   现在会显示红色 `No results`，属于缺一态（G3，§2.8）。
   另：ui-designer §2.5 建议把 `regex` **原始**报错挂 tooltip、计数格只显示短文案 `editor.find.invalidPattern`，
   **我采纳其方案**（比我在 §2.8 写的「显示错误末行」更好：原始 `regex` 报错是英文且可能多行，
   塞进 80px 计数格会撑破布局）。**该处以 ui-designer §2.5 为准，我撤回 §2.8 的措辞。**
   补充：计数格 **80px 宽度预算**与「状态词≠句子」口径见 §2.8；各语言短文案取值**已定稿**
   （en/en-GB `Invalid regex`、de-DE `Regex falsch`、fr-FR `Regex invalide`、ja-JP `正規表現エラー`、
   he-IL `ביטוי שגוי`、zh-CN `正则表达式无效`）；原始 `regex` 报错仍只挂 tooltip。
3. **删除确认对话框的 danger 视觉：已实测通过，可以用。**
   `tone()` 的 Danger 臂 = `bg: colors.danger`、`fg: colors.on_accent`（`buttons/button.rs:143-149`）。
   实测（视觉稿 §10）：标签 浅 `#fcfaf7`/`#ba3e38` = **5.26:1**、深 `#131110`/`#e5756e` = **6.37:1**（✅ AA）；
   hover 在深主题反而升到 **7.29:1**；填充 vs 对话框面板 = **5.48:1 / 5.57:1**（✅ ≥3:1）；
   Cancel 的 Ghost = 18.1:1 / 14.6:1（✅ AAA）。**四状态 × 两主题全过。**
   ⚠ **登记一处潜在耦合**：Danger 的 `fg` 用的是 `colors.on_accent`，**不是** danger 专用前景。
   当前两主题刚好成立，但只要哪个宿主单独改 `accent`/`on_accent`（例如蓝色 accent + 白 `on_accent`），
   Danger 标签就可能掉对比度。属潜在耦合、非当前 bug，建议记进 `tone()` 的待收项。
4. **B1 右键菜单图标**：现状是**纯文本菜单**（`row_menu` 只传 label，`lists/project/mod.rs:555`）；
   组件库 `MenuItem::icon/icon_element` 是**可用**的（ui-designer §1 已列出该 API）。本批**建议不加图标**
   （减少视觉面）。若团队决定加，图标清单按 ui-designer §3.3 走。**这条以 ui-designer 决定为准，我无异议。**
5. **编辑器未聚焦时的选区配色：已实测，结论是「够用，且不要改库」。** 我原先的担心被数据推翻，改正如下：
   - 源码事实：未聚焦选区 = `colors.selection.opacity(0.5)`（`editor/line.rs:191-195`），
     而 `colors.selection` 自身带 alpha（浅 0.20 / 深 0.302，`palette.rs:240/320`）
     ⇒ 有效 alpha 仅 浅 **0.10** / 深 **0.151**；编辑器底 = `colors.surface`（`view.rs:517`）。
   - 实测（vs 编辑器底）：未聚焦选区 **1.14:1 / 1.27:1**（确实弱）；
     当前匹配 `warning@0.45` **1.90:1 / 2.90:1**；**两者叠加 2.09:1 / 3.37:1**。
     选区是行内绝对定位底层 div（`line.rs:185-196`），查找高亮是文字 run 背景（`line.rs:58-71`）画在其上，
     所以「当前匹配」= warning 洗色叠在选区底上，**叠加值才是实际看到的合成**。
   - **不会出现「以为没跳过去」**：一次 `F3` 有 **4 个独立信号** ——
     ① 当前匹配洗色（深 2.90:1）② 叠加合成（深 3.37:1）③ 编辑器把匹配**滚到视口中心**
     ④ 查找条计数从 `3 of 12` 变 `4 of 12`。
   - ⚠ **不要改库**：`selection.opacity(0.5)` 半透明影响**所有**非聚焦场景，为查找条调它属越界。
     若主理人坚持更稳，唯一干净做法是在后续的 `set_find_matches` 里按 `focused` 加强当前匹配一档（裁决 #2 的收敛点）。
   - ⚠ **验收基线按此修正**：用「深 **3.37:1** / 浅 **2.09:1** + **滚动居中**」，
     **不要**要求「单靠选区洗色本身达 3:1」——那会把一个已达标的设计误判为不达标（见 §5.1 B3-11 已改）。
   - 另：未聚焦时**根本不画光标**（`line.rs:256` 的 `.filter(|_| focused && self.caret_on)`），
     当前行底（`line.rs:324`）与「其它出现处」高亮（`view.rs:413`）也一起被门控 ——
     这是组件统一的「编辑器已失活」约定，**不是查找条独有的缺失**，不当作本批缺陷。

---

## 7. 一句话交接

- **键位定稿**：`secondary-f` / `secondary-r` / `f3` / `shift-f3` + `escape`（context `EditorFindBar`）；
  五个目标键**全部无冲突**，不需要改任何既有绑定。
- **字面量口径**：写 `secondary-`（mac 上是 Cmd）；架构师 §附 的 `ctrl-f` 建议应改为 `secondary-f`，
  理由见 §1.3（IDEA-mac 真源 + `keymap.rs` 硬约束 + `DEFAULT_BINDINGS` 已按 `secondary-` 假设）。
- **Escape 优先级**按 §1.4 的四层契约实现；「输入框不吃 Escape」是结构保证（`ElyInput` 无 Escape 绑定 +
  `CloseFind` 是我们自己的 action），不需要窗口级拦截器。
- **两个待授权点**（Q1 行内校验的内联表现、Q2 只选主名不选扩展名）都会新增一点组件库 API；
  **新证据（F5）**：`PromptDialog::check` 已经是库内同形原语，所以 Q1 不是新发明，批准理由充分。
  若 Q1 被否，用 §4.3 的保底方案（重开字段 + toast），或直接用 `PromptDialog` 做重命名（偏离 IDEA 行内口径）。
- **B1 的「新建」建议改用库内 `PromptDialog` + `check`**（Q3）：零新增 API、
  文案键（`files.nameNewFile` / `files.enterNewFileName` / `files.fileNamePlaceholder` / `files.newFile` …）已齐、
  校验与错误显示免费。菜单 `New File` 仍用 `prompt_for_new_path`（能选目录）。两者 UI 不同，需主理人认可。
- **删除确认直接复用 `ConfirmDialog.destructive()`**（F6）：`ButtonVariant::Danger` 与
  「无输入框 ⇒ Enter 不是删除」两条都是现成的，不需要手搓 Dialog。
- **五项新发现**（F1 `secondary-d` 双绑、F2 `ctrl-` 方言与设置页在 macOS 上错位、F3 菜单栏无 disabled 通道
  但组件库有、F4 `NotWired→Action` 会丢「未接入」提示、F5/F6 两个可直接复用的现成原语）建议另开单，本批不夹带。
- **与 ui-designer 的交接：已全部闭环**（视觉稿 §10）。5 条对齐点里，
  ①`FindAndReplace`/`NewTab`/`SaveAs` 图标位 ②未聚焦选区对比度 ③`ConfirmDialog` danger 对比度
  均已答复（其中 ①②**修正了我的前提**，我已改正）；另两条（查找条图标体系、`invalidPattern` 文案）
  以视觉稿 §2.5/§3.2 为准，我已撤回自己的写法。
- **行内重命名配色**（Q1 的内联表现）**已按 ui-designer 实测定案：红框 + `colors.fg` 文字**（§4.3）；
  备选「保留 danger 红字」需在验收里如实记录浅色 4.44:1 差 AA 0.06。
