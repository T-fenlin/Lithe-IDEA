# 编辑器基本功剩余设计（B3 查找替换 / B2 文件生命周期 / B1 文件增删改名）

- 状态：设计稿（纯设计，未改任何源码）
- 作者：闻本立（编辑器架构师）
- 分支：`feat/gpui-editor-basics`
- 相关既有改动：B4/B5/B6/B10（行编辑、显示开关、后退前进、切标签）已在本分支未提交改动中，本设计与之同文件、同范式，注意不要互相踩。
- 构建约束（本设计不跑构建，仅记录）：
  `cd /d/developmentProjects/rust/Lithe-IDEA/rust/lithe-gpui && CARGO_TARGET_DIR=D:/developmentProjects/rust/tmpbuild RUST_MIN_STACK=67108864 CARGO_INCREMENTAL=0 cargo check -p lithe-gpui-app`

术语：**组件库** = `rust/lithe-ui`（GPUI 组件 + i18n）；**宿主** = `rust/lithe-gpui/crates/*`（app/workbench/editor/explorer/…）。

---

## 0. 现状盘点（先确认已有 / 缺失，全部经读码核对）

| 能力 | 现状 | 结论 |
| --- | --- | --- |
| `find_all(text, query, options)` | `rust/lithe-ui/src/editor/find.rs:24` | 已有，纯函数，返回 `Result<Vec<Range<usize>>, String>` |
| `FindOptions {case, word, regex}` | 同上 `:17` | 已有 |
| `FindWidget`（查找/替换条组件） | 同上 `:60`，`#[derive(IntoElement)]` | 已有，但**产品侧零接线** |
| FindWidget 的可见文案 | `find.rs` 内**硬编码英文**（"No results"/"Match case"/"Replace"/…） | **缺失 i18n**（见 §2.6） |
| `CodeEditor`「匹配高亮」专用 API | 无 | 但通用 `set_backgrounds(Vec<(Range,Hsla)>)` 可用（gallery `editor/search.rs` 已证） |
| `CodeEditor::select(ranges)` / `edit(edits)` / `text()` | `state.rs:662 / 832 / 415` | 已有，够 find 定位与替换 |
| `EditorPane` 查找状态/方法 | 无（`editor_view.rs` 无 `find` 字段） | **缺失** |
| 菜单 `MenuAction::Find` / `FindAndReplace` | `menu_bar.rs:318/319`，`keybind_action()==None`、`command_id()==None`、menu_key None | **未接线**（落进 `dispatch_editor_action` → "no_action"） |
| Ctrl+F/Ctrl+H/F3 键位 | `lithe_ui/editor/keys.rs` 与 workbench `install_key_actions` 均无 | **缺失** |
| `EditorPane::save_active` | `editor_view.rs:876`（仅写入已存在、writable 缓冲） | 已有 |
| `write_buffer`（私有写盘） | `editor_view.rs:1382`，`std::fs::write` + 清 dirty + `buffer_write_host` 回调 | 已有，可复用 |
| `reload_buffer` / `request_reload`（私有） | `editor_view.rs:1625 / 1610` | 已有，`revertFile` 直接复用 |
| `open_unsaved_dialog` / `PendingConfirm` | `editor_view.rs:1698 / 155` | 已有脏状态确认机制 |
| 「无路径的新建缓冲」概念 | 无：`Buffer.path: PathBuf`，`open()` 一定读盘 | **缺失**（B2 的核心结构改动） |
| `cx.prompt_for_paths` / `cx.prompt_for_new_path` | `workspace.rs:2118`（打开）/ `settings/dialog.rs:3100`（导出/另存先例） | 已有，另存为直接复用 |
| Core 文件命令 | `file.read` / `file.write` 有；**无 create/delete/rename**（`lithe-core/src/protocol/command.rs`） | 缺失（见 §4.5 决策） |
| explorer 右键菜单 | `explorer_view.rs` → `lithe_ui::lists::project::menu::row_menu_entries` → `ProjectAction`（Open/RevealInFileManager/CopyPath/CopyRelativePath/Refresh） | 无 rename/delete |
| `ProjectTree`（宿主实际用的树） | `lithe_ui/src/lists/project/mod.rs`，右键经 `open_menu` + `ContextMenu::manual` | 好挂点，但**无行内编辑** |
| 通用 `Tree`（组件库底层树） | `lithe_ui/src/lists/tree/mod.rs`：**已有 F2 行内重命名 `on_rename(key,new_name)`** 与拖拽 `on_move` | 可复用原语 |
| 行内/浮层编辑原语 | `lithe_ui::forms::Editing` / `InlineEdit`（`forms/inline.rs`）；`lithe_ui::editor::RenameInput`（`editor/refactor.rs:118`，`on_rename/on_cancel`） | 可复用 |
| 级联同步入口（buffer/JumpHistory/断点/会话） | 均按 path 为键，**无 rename/delete 重映射入口** | **缺失** |

---

## 1. 架构总览（改动落点一览）

| 模块 | 层 | B3 | B2 | B1 | 公开面变化 |
| --- | --- | --- | --- | --- | --- |
| `lithe-ui/src/editor/find.rs` | 组件库 | 文案 i18n（或 labels） | | | **公开 API 变更** |
| `lithe-ui/locales/ui.yml` | 组件库 | `editor.find.*` 键 | | `lists.project.*` 键 | 新增键 |
| `lithe-ui/src/editor/state.rs` | 组件库 | 可选 `set_find_matches`（见 §2.6） | | | 可选公开 API |
| `lithe-ui/src/lists/project/menu.rs` | 组件库 | | | `ProjectAction` 增 Rename/Delete(+New)、`ProjectLabels` 扩项 | **公开 API 变更** |
| `lithe-ui/src/lists/project/mod.rs` | 组件库 | | | `ProjectTree` 接线行内重命名 / delete 回调 | **公开 API 变更** |
| `lithe-gpui/crates/editor/src/editor_view.rs` | 宿主 | 查找状态/渲染/方法 | new/saveAs/saveAll/revert | remap/remove 端口 | `EditorPane` 公开方法 |
| `lithe-gpui/crates/editor/src/buffer.rs` | 宿主 | | 未命名缓冲键 | rename 时改 name/lang | `Buffer` 内部结构 |
| `lithe-gpui/crates/editor/src/navigation.rs` | 宿主 | | | `JumpHistory::remap/remove_path` | 内部 |
| `lithe-gpui/crates/explorer/src/explorer_view.rs` | 宿主 | | | Rename/Delete/New 宿主端口 | `Explorer` 公开端口 |
| `lithe-gpui/crates/workbench/src/menu_bar.rs` | 宿主 | Find 接线 + 键位 | 5 条菜单接线 | | 菜单表 + guard 测试 |
| `lithe-gpui/crates/workbench/src/workspace.rs` | 宿主 | Find 分支 | 5 条分支 | remap/delete 接 explorer | 无公开面 |
| `rust/lithe-core` | Core | — | 无（初版 `std::fs`） | 无 | **不改契约** |

依赖方向不变：`host → lithe-ui`（单向）。Core 契约本批不动。

---

## 2. B3 查找 / 替换

### 2.1 状态归属

`EditorPane` 新增一个 `Option<FindBar>`（关闭即 `None`）。查找条**不放在 `CodeEditor` 内部**：`CodeEditor` 是通用文本编辑器，不认识查找条；这与既有的「补全列表 / hover / 代码动作菜单都由 `EditorPane`（宿主）持有」一致（见 `editor_view.rs` 的 `Completion`、`hover_offset`、`code_actions`）。

```rust
/// 正在显示的查找条（编辑器内 Ctrl+F 的那一条）。与 `Completion` 同一套「宿主持有」的范式。
struct FindBar {
    /// 归属的缓冲区编辑器；切标签、关标签、换缓冲时据此收起。
    editor: EntityId,
    /// 查询框与替换框的 InputState（宿主持有，与 chat_field 同一做法）。
    query: Entity<lithe_ui::forms::InputState>,
    replace: Entity<lithe_ui::forms::InputState>,
    options: FindOptions,
    /// 最近一次 find_all 的结果；编辑后重算。
    matches: Vec<Range<usize>>,
    /// 正则编译失败时留一条消息，交给 FindWidget::error。
    error: Option<SharedString>,
    /// 当前匹配下标；空结果时为 None。
    current: Option<usize>,
    /// 替换行是否展开。
    showing_replace: bool,
    /// 查询/替换框的 InputEvent 订阅（Changed/Submit）。
    _events: Vec<Subscription>,
}

pub struct EditorPane {
    // ...现有字段...
    find: Option<FindBar>,
}
```

不变量：
- `find.is_some()` 时 `find.editor` 必等于当时 active buffer 的 `editor.entity_id()`（切标签即 `close_find`）。
- `matches` 只由「当前 `editor` 的正文 + `query` + `options`」推导，不缓存跨缓冲结果。
- `current` 落点 = `(0..matches.len())`，空结果 `None`。

### 2.2 渲染挂载点与 z 序

**选点：正文容器内的绝对定位浮层，与 `completion_popup` 同一父容器、同一 z 层**（`render_body` 里 `.relative()` 的那个 `div#lithe-editor-host`），不用 `window.open_dialog`。

理由：
- 查找条**非模态**：打开时编辑器仍可滚动、仍能看见代码；`open_dialog`（`GoToLinePrompt` 用的那条）是模态层，会挡编辑器、抢焦点语义，适合「转到行」不适合常驻查找条。
- gallery `editor/search.rs` 的落法就是 `framed(...).relative().child(editor).child(div().absolute().top_1().right_4().child(widget))`，产品照抄即可：`.absolute().top_2().right_2()`。
- 与补全列表同一挂载点也统一了「出界裁剪」行为（都被正文容器 `overflow_hidden` 裁掉），不会「库能出界、宿主不能」。

> 与 team-lead 提示的 `GoToLinePrompt` 关系说明：`GoToLinePrompt` 实际走的是 `window.open_dialog("lithe-go-to-line", …)`（**模态对话框**）；查找条更接近 `completion_popup`（**正文容器内的绝对定位浮层**）。因此落点选后者；两者**相同**的地方在「谁持有 `Entity<InputState>`、谁订阅 `InputEvent::Submit/Changed`」——都由 `EditorPane` 持有并订阅（`chat_field` 是组件库里的先例，GoToLine 的 `input` 是宿主里的先例）。

z 序：编辑器 → 正文容器 canvas（量原点）→ `completion_popup` → **find 条** → `code_action_menu`。find 条排在补全之上（查找条与补全不会同时活跃，但保证查找条不被裁在补全下面）。

### 2.3 匹配收集与高亮的数据流

```
触发点：query 框 Changed / replace 框 Changed / options 变 / EditorEvent::Changed（正文变）
  │
  ├─ let text = editor.read(cx).text()
  ├─ match find_all(&text, query.read(cx).text(), options)
  │     Ok(ranges)  => 建高亮 Vec<(Range, Hsla)>：
  │                     current 那一处 colors.warning.opacity(0.45)
  │                     其余        colors.warning.opacity(0.2)
  │                   editor.update(|e,cx| e.set_backgrounds(marks, cx))
  │                   bar.matches = ranges; bar.error = None
  │                   bar.current = 夹进 [0, len)（空则 None）
  │     Err(message) => editor.update(|e,cx| e.set_backgrounds(Vec::new(), cx))
  │                   bar.error = Some(message 末行); bar.matches.clear(); bar.current = None
  │
  └─ 渲染 FindWidget::new(id, &query, current, matches.len())
         .options(options)
         .when(showing_replace, |w| w.replace(&replace))
         .on_options / .on_step / .on_toggle_replace / .on_replace / .on_close
         （error 时 .error(message)）
```

定位与滚动：`.on_step(forward)` → `current = (current ± 1 + len) % len`；`editor.select([matches[current].clone()])`——`CodeEditor::select` 自带 `scroll_to_item(primary_row, Center)`（`state.rs:687`），所以「上/下一处」会自动滚进视野并选中。

**关键取舍**：这里复用现成的通用通道 `set_backgrounds`，而不是新造高亮 API。依据是 gallery 的查找页已经在这么用，且 `select` 已解决定位。代价见 §2.6 的第 2 条（通道被多用途争用）。

### 2.4 replace 单条 / 全部如何落到编辑 API

- 单条：`editor.update(|e,cx| e.edit([(bar.matches[cur].clone(), replace_text)], cx))`
- 全部：`editor.update(|e,cx| e.edit(bar.matches.iter().map(|r| (r.clone(), replace_text.clone())).collect::<Vec<_>>(), cx))`

`CodeEditor::edit` 的语义正好是「一批范围一次替换、算一个撤销步、只读拒绝」（`state.rs:832`），批量替换是一步撤销、只读文件天然拒绝，无需额外处理。

替换后链路：`edit` → `apply_edits` → `EditorEvent::Changed` → `EditorPane::on_text_changed`（置脏、`revision++`、诊断/自动保存、`clear_hover`）→ 我们在这里**追加一次重跑查找**（重建 `matches` 与高亮）。注意：`on_text_changed` 末尾是 `state.set_diagnostics(Vec::new())`，不要把它和 `set_backgrounds` 搞混——`backgrounds` 是独立字段，不会被诊断清空。

### 2.5 菜单与键位接线点

照本分支刚立的 **U20/GoToLine 范式**（`menu_bar.rs` 的 `install_menu_key` + `apply_menu_action` 直调宿主方法），不新增 `CommandId`（与 GoToLine 同批口径，见 §2.7 决策）。

| 动作 | 登记方 | 键位（建议） | 落点 |
| --- | --- | --- | --- |
| 查找 | workbench `actions!` 新增 `OpenFind` | `ctrl-f` | `apply_menu_action` → `pane.open_find(false, window, cx)` |
| 查找并替换 | workbench `actions!` 新增 `OpenFindReplace` | `ctrl-h`（见决策） | `apply_menu_action` → `pane.open_find(true, window, cx)` |
| 下一处 | `actions!` 新增 `FindNext` | `f3` | `pane.find_step(true, window, cx)` |
| 上一处 | `actions!` 新增 `FindPrevious` | `shift-f3` | `pane.find_step(false, window, cx)` |
| 关闭 | `actions!` 新增 `CloseFind` | `escape`（查找条 key context 内） | `pane.close_find(window, cx)` |

- `MenuAction::Find.keybind_action()` 由 `None` 改为 `Some(Box::new(OpenFind))`，`menu_key()` 由 `None` 改为 `Some(KEY_FIND)`；`FindAndReplace` 同理。
- `apply_menu_action` 现有分支里 `Find|FindAndReplace` 目前与 Undo 混在 `dispatch_editor_action`（`workspace.rs:1878-1892`）——**要拆出来**，改为直调 `editor.update(|pane| pane.open_find(...))`（与 `QuickFix`/`GoToLine` 同排）。
- **Esc / F3 / Shift+F3 的 key context**：查找条容器挂 `key_context("EditorFindBar")`，三个键绑在该 context 上。`escape` 在 `ElyEditor` 上下文被 `SingleCursor` 占（`keys.rs:314`），菜单栏还有全局 escape 拦截器（`menu_bar.rs:1168`）：查找条输入框持有焦点时，其所在 context 优先级更高，需在实现时**验证输入框自身不吃 Escape**，必要时在查找条容器用 `on_key_down` 兜一层（`event.keystroke.key == "escape"` → `cx.stop_propagation()` + `close_find`）。
- 打开时的预填：若编辑器有非空选区，用选区文本预填查询框（IDEA 口径，选一个词按 Ctrl+F 直接搜该词）。

### 2.6 组件库需要补的 API（明确结论）

1. **必须补：FindWidget 的可见文案要 i18n 化。**
   现状 `find.rs` 把 "No results"、"{n} found"、"Match case"、"Whole word"、"Regular expression"、"Toggle replace"、"Previous match"、"Next match"、"Close"、"Replace"、"Replace all" 全写死为英文，**没有**走组件库自己的 i18n 缝（`lithe_ui::i18n::text(cx, key, args)`，见 `lithe-ui/src/i18n/mod.rs`；组件库已用 `rust_i18n::i18n!("locales")` 且 app 启动时调 `lithe_ui::i18n::set_locale`）。直接接入产品会让中文界面出现英文。
   - 方案 A（推荐，与库内 `editor/keybindings.rs` 用 `i18n::text` 的做法一致）：FindWidget 内部改走 `lithe_ui::i18n::text`，在 `rust/lithe-ui/locales/ui.yml` 新增 `editor.find.*`（`noResults`/`found`/`matchCase`/`wholeWord`/`regularExpression`/`toggleReplace`/`previousMatch`/`nextMatch`/`close`/`replace`/`replaceAll`），各叶列多语言。
   - 方案 B（与 `ProjectLabels` 一致的「owner 供文案」）：新增
     ```rust
     pub struct FindLabels { pub no_results: SharedString, pub found.., pub match_case: SharedString,
                              pub whole_word: SharedString, pub regular_expression: SharedString,
                              pub toggle_replace: SharedString, pub previous: SharedString,
                              pub next: SharedString, pub close: SharedString,
                              pub replace: SharedString, pub replace_all: SharedString }
     impl FindWidget { pub fn labels(mut self, labels: FindLabels) -> Self { … } }
     ```
   - 决策见 §6.3。

2. **可选补：`CodeEditor::set_find_matches`。**
   `set_backgrounds` 是**通用**通道，今天只有 gallery 的查找页在用；一旦产品里有第二处用途（括号匹配高亮、当前词出现处高亮）就会互相覆盖。若要把它收成「查找条专属」，建议加语义方法（**签名如下，是否加见 §6.2**）：
   ```rust
   /// 标注查找匹配：`matches` 是全部匹配，`current` 是当前那一处（更醒目）。
   /// 库只负责按统一样式画「当前 vs 其余」，供查找条使用；空 `matches` 等于清除。
   pub fn set_find_matches(&mut self, matches: &[Range<usize>], current: Option<usize>, cx: &mut Context<Self>);
   /// 清除查找高亮。
   pub fn clear_find_matches(&mut self, cx: &mut Context<Self>);
   ```
   初版可以先用 `set_backgrounds`（改动最小、gallery 已验证），把 `set_find_matches` 作为「第二个用途出现时」的收敛点。

3. **无需新 API 的部分**：定位/滚动用 `select`，替换用 `edit`，取正文用 `text()`——全部已有。

### 2.7 性能

`find_all` 每次击键同步跑 `regex`。文件上限 2 MiB（`editor/buffer.rs` 的 `MAX_EDITOR_BYTES`），单次扫描可接受；但连续输入时每键一次全量扫描略浪费。建议（可选）：对 query 变化加 ~120ms debounce（照 `AUTO_SAVE_DEBOUNCE` 范式），或只在 `EditorEvent::Changed` 后与 query 提交后重跑。列为 §6 决策。

---

## 3. B2 文件生命周期

### 3.1 5 项各自的 Core 命令 / EditorPane 方法 / 语义

| 菜单项 | 现有/缺失 Core 命令 | EditorPane 侧方法（新增/复用） | 脏状态与交互 |
| --- | --- | --- | --- |
| `lithe.menu.newTab` | 无（纯内存） | **新增** `new_untitled(&mut self, window, cx)` | 新建即 dirty=false；无路径 |
| `lithe.menu.newFile` | `file.write`（可选）或 `std::fs` | **新增** `create_file(dir_or_none, name, window, cx)` | 对话框选目录+名字 → 建空文件 → `open` |
| `lithe.menu.saveAs` | 无 | **新增** `save_as(&mut self, window, cx)` | `prompt_for_new_path` → 写盘 → 改 path/name/icon/lang → 清 dirty |
| `lithe.menu.saveAll` | 无 | **新增** `save_all(&mut self, window, cx)` | 遍历 dirty+writable 逐个 `write_buffer` |
| `lithe.menu.revertFile` | 无 | **新增** `revert_active(&mut self, window, cx)`（内部复用 `request_reload`） | 脏时弹 `open_unsaved_dialog(PendingConfirm::Reload)`，已有 |

Core 侧结论：`file.read`/`file.write` 存在，**create/delete/rename 没有**。但既有 gpui 写盘走的是 `std::fs`（`write_buffer`），native filesystem 属平台适配器（仓库边界允许）。因此 **B2 初版可零 Core 改动**，用 `std::fs` + `prompt_for_*` 完成；是否需要为「跨平台语义/稳定错误码」补 Core 命令见 §6.8。

### 3.2 「无路径的新建缓冲」是核心结构改动

现状 `Buffer.path: PathBuf`，且 `EditorPane::open`/`index_of`/`activate`/`close` 全部**以 path 相等**做去重与定位。未命名缓冲没有磁盘路径，会破坏这些假设。两种建模：

| 方案 | 做法 | 优点 | 代价 |
| --- | --- | --- | --- |
| A（推荐） 引入稳定键 | 新增 `enum BufferKey { Disk(PathBuf), Untitled(u32) }`；`Buffe` 携带 `key`，`index_of` 等改按 key 比较；`untitled` 自增标号 | 不歧义、可并存多个未命名缓冲、改名/另存为时自然换键 | 改动 `buffer.rs`/`editor_view.rs` 若干比较点 |
| B 加标志位 | `Buffer` 加 `untitled: bool`，path 用哨兵（如空 `PathBuf` 或 `untitled:N` 伪路径） | 改动少 | path 语义被污染，`is_virtual_source_path`/会话/断点等处易出边界 bug |

初版建议 **A**。影响面清单（逐处审）：`active_buffer_path`、`open_disk_files`、`active_disk_file`、`is_virtual_source_path`、`last_text`、`gutter_breakpoints`、`session.rs`（未命名缓冲**不落会话**）、`render_tab` 的 `tab_element_id(path)`（未命名用 key 生成稳定 id）。

### 3.3 newFile vs newTab 的区别与初始语言/路径

- `newTab` = **未命名（scratch）缓冲**：无路径，`name = "Untitled-1"`（或 `Scratch`，见决策）；`resolved_language("Untitled-1") == None` → **Plain text**；保存时走 `save_as`。
- `newFile` = **项目内建一个真实文件**：`prompt_for_new_path(&workspace_root 或当前目录, Some("NewFile.java"))` → 建空文件 → `open(path)`；语言由所选文件名经 `language_for_file` 推断（`editor/buffer.rs:27`）。
- 两者都存在是否还要 IDEA 的「New Scratch File（可选语言）」——见 §6.5。

### 3.4 另存为的现有机制

- 文件选择：`cx.prompt_for_new_path(&directory, Some(default_name))`，先例在 `settings/dialog.rs:3100`（导出）。目录取当前缓冲所在目录；未命名缓冲取工作区根。
- 覆盖确认：`prompt_for_new_path` 由平台对话框决定是否提示覆盖（按平台默认）。若产品要求二次确认，另加 `Dialog`。
- 写盘复用 `write_buffer` 的 `std::fs::write` 路径；成功后**改键**：`Buffer.path/name/icon/language`、`last_text` 键、`gutter_breakpoints` 键、标签 id（自动跟随）、`schedule_session_save`。是否删除旧文件见 §6.6。

### 3.5 菜单接线

把 file 菜单里 5 条 `MenuItem::NotWired` 换成 `MenuItem::Action(MenuAction::{NewFile, NewTab, SaveAs, SaveAll, RevertFile})`，并按 U20 范式补齐 `label_key/icon/keybind_action/command_id` 五张映射 + `apply_menu_action` 分支。文案键 `lithe.menu.newFile/newTab/saveAs/saveAll/revertFile` **已存在于 locale**（现由 NotWired 的 `label_key` 引用），无需新增。

无活动缓冲时：saveAs/saveAll/revertFile 应为 no-op（打印诊断）或置灰（菜单项禁用来源当前只有 `NotWired` 无 disabled 通道，见 §6.7）。

键位建议：SaveAs `secondary-shift-s`；SaveAll `secondary-alt-shift-s`（IDEA 是 Ctrl+Alt+Shift+S）；Revert 无默认键。三端一致，需拍板。

---

## 4. B1 文件增删改名

### 4.1 explorer 右键菜单结构

组件库 `lists/project/menu.rs` 的 `ProjectAction` 现为 5 项。建议扩为：

```rust
pub enum ProjectAction {
    Open,
    RevealInFileManager,
    CopyPath,
    CopyRelativePath,
    Refresh,
    Rename,   // 新增
    Delete,   // 新增
    // New 是否纳入见 §6.11
}

pub fn row_menu_entries(kind: ProjectRowKind) -> Vec<Option<ProjectAction>> {
    match kind {
        ProjectRowKind::EmptyFolder => Vec::new(),
        ProjectRowKind::File => vec![
            Some(ProjectAction::Open),
            None,
            Some(ProjectAction::Rename),
            Some(ProjectAction::Delete),
            None,
            Some(ProjectAction::RevealInFileManager),
            Some(ProjectAction::CopyPath),
            Some(ProjectAction::CopyRelativePath),
        ],
        ProjectRowKind::Directory => vec![
            Some(ProjectAction::Rename),
            Some(ProjectAction::Delete),
            None,
            Some(ProjectAction::RevealInFileManager),
            Some(ProjectAction::CopyPath),
            Some(ProjectAction::CopyRelativePath),
            None,
            Some(ProjectAction::Refresh),
        ],
    }
}
```

`PROJECT_ACTIONS` 数组与 `ProjectLabels`（新增 `rename`/`delete` 字段）同步扩项；`host.Labels` 由 `explorer_view.rs::row_labels()` 填（`tr("lithe.ui.rename")` / `tr("lithe.ui.delete")`，两键**已存在**）。

触发方式：**沿用现成机制**——`ProjectTree::context_key` 右键选中并置 `open_menu`，`render` 用 `ContextMenu::new(...).manual(Some((number, at)))` 在指针处开菜单（`lists/project/mod.rs:658-679`）。菜单项回调：
- `Delete` → 走 `tree.on_action(ProjectAction::Delete, path)`（现有 `on_action` 通道，宿主处理）。
- `Rename` → 进入行内编辑（见下），提交时走**新回调** `on_rename(path, new_name)`。

### 4.2 重命名交互（行内 vs 对话框）

| 方案 | 做法 | 优点 | 代价 |
| --- | --- | --- | --- |
| A 行内编辑（推荐） | `ProjectTree` 内持有 `Editing`（`window.use_keyed_state((id,"rename"))`，与通用 `Tree` 的做法一字不差，`lists/tree/mod.rs:177-183` + `266-284`）；`Rename` 时 `Editing::begin(label)`；行渲染时该行画输入框；提交/Esc 由 `Editing` 处理 | IDEA 口径；复用库内既有原语 | `ProjectTree` 要接一个 `Editing` 状态与行内渲染（改动比 B 大） |
| B 浮层/对话框 | 复用 `lithe_ui::editor::RenameInput`（浮层，`on_rename/on_cancel`）或 `Dialog` + Input | 改动最小 | 非行内，观感与 IDEA 有差 |

校验（两方案共用，**放宿主**，因为要读文件系统）：
- 非空；不含路径分隔符 `\` `/`；
- 无 Windows 保留名（`CON/PRN/AUX/NUL/COM1..9/LPT1..9`）与尾随点/空格；
- 目标名不与同目录既有条目重名（按目标平台的大小写规则）；
- 默认保留扩展名（改扩展名是否允许见 §6 决策）。
- 失败：行内不提交并内联提示（或对话框禁用 OK）。

### 4.3 删除确认

复用 `Dialog`（`workspace.rs` 里已有未保存确认对话框的先例）。文案：标题复用 `lithe.ui.delete`、动作复用 `lithe.ui.cancel`；**确认正文需新增**（例如 `lithe.explorer.deleteConfirm`，带 `{name}` 参数，走 `tr_args`）。确认后 `std::fs::remove_file` / `remove_dir_all`（目录递归）。

是否进系统回收站：IDEA 走「删除到回收站」。本仓**无跨平台 trash 抽象**，见 §6.9。

### 4.4 级联同步方案（重命名/删除后）

统一由 workbench 在 explorer 宿主回调里调一个**单一入口**处理，避免各处漏改：

```rust
// EditorPane 新增
pub fn remap_paths(&mut self, from: &Path, to: &Path, window: &mut Window, cx: &mut Context<Self>);
pub fn forget_paths(&mut self, target: &Path, window: &mut Window, cx: &mut Context<Self>);
```

`remap_paths`（重命名 old → new；目录则整棵子树）逐项处理：

| 状态 | 处理 | 所需新入口 |
| --- | --- | --- |
| `EditorPane.buffers[i].path` | 组件级前缀替换：`a/b` → `a/bc` 不受影响（按路径**组件**前缀，不按字符串前缀）；同步 `name`/`icon`（`icon_for_file`）/`editor.set_language(resolved_language(new_name))` | 内部 |
| `last_text: BTreeMap<PathBuf,String>` | 重键 | 内部 |
| `gutter_breakpoints: BTreeMap<PathBuf,Vec<..>>` | 重键 | 内部 |
| `closed: Vec<PathBuf>` | 列表内替换 | 内部 |
| `JumpHistory.entries` | 遍历替换 path | **`JumpHistory::remap(from,to)`**（`navigation.rs`，`pub(crate)`） |
| 标签 id | `tab_element_id(path)` 随 path 自动换，无需专门处理 | 无 |
| 会话 `session.rs` | 改完立即 `schedule_session_save`；已落盘的旧路径会在下次启动重开旧路径，故**改名后必须重存会话** | 复用现有 |
| LSP / 调试 | 对 Java 文件需 `lsp.closeDocument(old)` + 重开新 URI；已下发断点需重发 | 见 §5 风险 |

`forget_paths`（删除）：
- 关掉 `target`（或子树）下所有 buffer：先处理脏缓冲（提示保存/放弃，复用 `open_unsaved_dialog` 的批量语义或新增 `PendingConfirm` 变体）；关闭走现有 `close(index)`。
- 清 `last_text`、`gutter_breakpoints`；`JumpHistory` 加 `remove_path(target)` 移除相关条目。
- 树自身：改/删后调 `Explorer::refresh`（重读 `workspace.snapshot` + `git.status`）。

`Explorer` 侧新增宿主端口（照 `set_reveal_in_file_manager_host` 范式）：

```rust
pub type RenameHost = Arc<dyn Fn(&Path, &str /*new name*/, &mut Window, &mut App) + 'static>;
pub type DeleteHost = Arc<dyn Fn(&Path, &mut Window, &mut App) + 'static>;
// Explorer 增字段 + set_rename_host / set_delete_host；run_row_action 增分支。
```

### 4.5 所需 Core 命令

- 刷新：`workspace.snapshot`（已有）。
- 创建/删除/改名：**无 Core 命令**；初版用 `std::fs`（`rename`/`remove_file`/`remove_dir_all`/`create`）。决策见 §6.8。

---

## 5. 模块边界、依赖与风险

### 5.1 跨 crate 契约变化

- `lithe-ui`（组件库）公开面变化：`FindWidget`（文案 i18n / labels）、`ProjectAction`（+Rename/Delete）、`ProjectLabels`（+字段）、`ProjectTree`（行内重命名回调 / delete 回调）、可选 `CodeEditor::set_find_matches`。
- `lithe-gpui`：`EditorPane` 新增公开方法（`remap_paths`/`forget_paths`/`open_find`/…）、`Explorer` 新增公开端口、`menu_bar` 菜单表变化。
- `rust/lithe-core`：**不改**（无新命令、无 C ABI 改动）。

### 5.2 风险清单

| 风险 | 影响 | 缓解 | 验证方式 |
| --- | --- | --- | --- |
| FindWidget 硬编码英文进入产品 | 中（中文界面出英文） | i18n 化 / owner labels（§2.6） | 切换 locale 后文案变化（组件库单测） |
| `set_backgrounds` 被多用途争用 | 中 | 若出现第二用途则加 `set_find_matches` | 查找 vs 括号匹配共存测试 |
| Escape 被 `SingleCursor`/菜单栏拦截器抢占 | 中 | 查找条独立 key context + 兜底 `on_key_down` | 查找条开着按 Esc 只关查找条 |
| 无路径缓冲破坏 path 键假设 | 高 | 引入 `BufferKey`，逐点改比较（§3.2） | 两个未命名缓冲并存/去重/另存为 |
| rename/delete 级联遗漏（会话/断点/LSP） | 高 | 单一 `remap_paths` 入口 + 清单核对 | 改名后跳转历史/断点/会话均指向新路径 |
| 目录改名的前缀匹配与跨平台大小写 | 高 | 路径**组件**前缀匹配（非字符串） | `a/b` 改名不影响 `a/bc` |
| 删除不可逆（无回收站） | 中 | 强确认兜底；回收站见 §6.9 | 手工 |
| Windows 保留名/尾随点/重名校验 | 中 | 校验函数 + 单测 | 单测 |
| find_all 同步 regex 卡顿 | 中 | 2 MiB 上限 + debounce | 基准 |
| 另存为时 `last_text`/断点键未迁 | 中 | 归入 `remap_paths` 统一处理 | 另存为后断点仍在 |

---

## 6. 待裁决清单（需主理人 / 用户拍板）

1. **查找键位**：`Ctrl+F`/`Ctrl+H`（VS Code 口径，team-lead 提供）还是 `Ctrl+F`/`Ctrl+R`（IDEA 默认 Replace）？`F3`/`Shift+F3` 三端是否统一？
2. **高亮通道**：复用 `set_backgrounds`（改动最小）还是新增 `CodeEditor::set_find_matches`（语义清晰、防争用）？
3. **FindWidget 文案**：走组件库 `i18n::text` + `ui.yml`，还是 owner labels（`FindLabels`）？
4. **查找选项持久化**：`case/word/regex`、替换行展开态是否存 settings store 或按会话记住？
5. **newFile / newTab 语义**：newFile 是否 = 项目内建真实文件（对话框选目录+名字）；newTab 是否 = 未命名 scratch；是否还要「New Scratch File（可选语言）」？
6. **另存为是否删除原文件**（通常保留、写新文件）；是否需要覆盖二次确认。
7. **saveAs/saveAll/revertFile 无活动缓冲**时禁用还是 no-op（当前菜单项无 disabled 来源）。
8. **文件写操作走 `std::fs` 还是新增 Core `file.create/delete/rename`**（对齐 `file.read/write` 的稳定契约）？
9. **删除是否进系统回收站**（需跨平台 trash 抽象，初版可「强确认 + 直接删」）。
10. **explorer 重命名**：行内编辑（改 `ProjectTree`，库改动大、IDEA 口径）vs 浮层/对话框（`RenameInput`，改动小）？
11. **「增」是否纳入本批**（右键 New File / New Folder）；若纳入，New File 与 B2 的 newFile 是否同一入口。
12. **目录改名/删除的级联是否包含通知 LSP（closeDocument/reopen）与重发断点**（涉及语言服务生命周期）。

---

## 7. 验证计划（每块可测断言）

### B3 查找/替换
- 单测（`lithe-ui`）：`find_all` 已覆盖（`find.rs` 内已有）；新增 FindWidget labels/i18n（locale 切换后 `i18n::text` 输出随语言变）；可选 `set_find_matches` 的「当前 vs 其余」样式。
- 集成（`crates/editor`）：`EditorPane::new` → `open_find(false)` → 注入 query → 断言 `set_backgrounds` 的 range 数与 `current` 定位；`find_step` 环绕；`replace` 单条/全部后 `text()` 变化、`is_dirty==true`、一次撤销步；`close_find` 后 `backgrounds` 清空、焦点回编辑器。
- 交互：查找条开着按 Esc、F3、Shift+F3 的行为（key context 生效）。

### B2 文件生命周期
- 单测：未命名缓冲键唯一/去重；`save_as` 后 `path/name/language` 正确；`save_all` 只写 dirty 且 writable；`revert` 脏时弹确认。
- 集成：newFile → 编辑 → saveAs → reload 全链；未命名缓冲不落会话。
- 守卫：`menu_bar.rs::b2_actions_are_wired_and_only_bound_ones_carry_a_key` 的条目与 `bound` 清单更新。

### B1 文件增删改名
- 单测（`lithe-ui`）：`row_menu_entries` 新项；`PROJECT_ACTIONS`/`ProjectLabels` 一致性（`lists/project/tests.rs:223` 已有一致性测试，需同步）。
- 单测（宿主）：重命名校验（保留名/分隔符/重名）；`remap_paths` 组件级前缀（`a/b` 不影响 `a/bc`）。
- 集成：改名后 tabs / `JumpHistory` / `gutter_breakpoints` / 会话均指向新路径；删除后相关 tab 关闭、explorer 刷新。

### 守卫与构建
- 必改测试：`menu_bar.rs::b2_actions_are_wired_and_only_bound_ones_carry_a_key`（Find/FindAndReplace 由 `false`→`true`、`bound` 清单加查找键）、`keymap_engine.rs::menu_bar_non_command_contains` 里 `NON_COMMAND` 副本、（若菜单结构变化）`MENUS` 占位/计数断言。
- 构建（本设计阶段不跑）：
  `cd /d/developmentProjects/rust/Lithe-IDEA/rust/lithe-gpui && CARGO_TARGET_DIR=D:/developmentProjects/rust/tmpbuild RUST_MIN_STACK=67108864 CARGO_INCREMENTAL=0 cargo check -p lithe-gpui-app`
- 组件库改动附 `cargo test -p lithe-ui`（find / lists::project 相关）。

---

## 附：接口签名汇总（实现交核心开发工程师时可直接对照）

```rust
// ---- lithe-ui: editor/find.rs（文案 i18n 或 labels，见 §2.6 方案 A/B）----
// 方案 B 时：
// pub struct FindLabels { … }
// impl FindWidget { pub fn labels(mut self, labels: FindLabels) -> Self; }

// ---- lithe-ui: editor/state.rs（可选，见 §2.6-2）----
// pub fn set_find_matches(&mut self, matches: &[Range<usize>], current: Option<usize>, cx: &mut Context<Self>);
// pub fn clear_find_matches(&mut self, cx: &mut Context<Self>);

// ---- lithe-ui: lists/project/menu.rs ----
// pub enum ProjectAction { Open, RevealInFileManager, CopyPath, CopyRelativePath, Refresh, Rename, Delete }
// pub struct ProjectLabels { …, pub rename: SharedString, pub delete: SharedString }

// ---- lithe-ui: lists/project/mod.rs ----
// impl ProjectTree {
//     /// 行内重命名提交：节点路径 + 新名字。
//     pub fn on_rename(mut self, handler: impl Fn(&str /*path*/, &str /*new name*/, &mut Window, &mut App) + 'static) -> Self;
//     /// 请求对某行开始行内重命名（右键菜单 Rename 走它）。
//     pub(crate) fn begin_rename(&mut self, key: &SharedString, window: &mut Window, cx: &mut Context<Self>);
// }

// ---- lithe-gpui: editor/editor_view.rs ----
// impl EditorPane {
//     pub fn open_find(&mut self, with_replace: bool, window: &mut Window, cx: &mut Context<Self>);
//     pub fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>);
//     pub fn find_step(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>);
//     pub fn new_untitled(&mut self, window: &mut Window, cx: &mut Context<Self>);
//     pub fn create_file(&mut self, directory: Option<PathBuf>, name: String, window: &mut Window, cx: &mut Context<Self>);
//     pub fn save_as(&mut self, window: &mut Window, cx: &mut Context<Self>);
//     pub fn save_all(&mut self, window: &mut Window, cx: &mut Context<Self>);
//     pub fn revert_active(&mut self, window: &mut Window, cx: &mut Context<Self>);
//     pub fn remap_paths(&mut self, from: &Path, to: &Path, window: &mut Window, cx: &mut Context<Self>);
//     pub fn forget_paths(&mut self, target: &Path, window: &mut Window, cx: &mut Context<Self>);
// }

// ---- lithe-gpui: editor/navigation.rs ----
// impl JumpHistory {
//     pub(crate) fn remap(&mut self, from: &Path, to: &Path);
//     pub(crate) fn remove_path(&mut self, target: &Path);
// }

// ---- lithe-gpui: explorer/explorer_view.rs ----
// pub type RenameHost = Arc<dyn Fn(&Path, &str, &mut Window, &mut App) + 'static>;
// pub type DeleteHost = Arc<dyn Fn(&Path, &mut Window, &mut App) + 'static>;
// impl Explorer { pub fn set_rename_host(&mut self, host: RenameHost); pub fn set_delete_host(&mut self, host: DeleteHost); }

// ---- lithe-gpui: workbench/menu_bar.rs ----
// gpui::actions!(lithe_workbench, [OpenFind, OpenFindReplace, FindNext, FindPrevious, CloseFind]);
// MenuAction 新增：NewFile, NewTab, SaveAs, SaveAll, RevertFile
// pub const KEY_FIND: &str = "ctrl-f";
// pub const KEY_FIND_REPLACE: &str = "ctrl-h"; // 见 §6.1
```

---

## 8. 主理人裁决（2026-10-13，郝链成）

依据产品 IntelliJ 血统 + 仓库既有约定（`ctrl-*` 键位、组件库 i18n 缝、`std::fs` 写盘、LSP 属 C 批未接线），对 §6 十二项裁决如下（**实现以此为准**）：

| # | 项 | 裁决 |
| --- | --- | --- |
| 1 | 查找键位 | `Ctrl+F` 查找 / `Ctrl+R` 替换（IDEA 口径），`F3` / `Shift+F3` 下一处/上一处，三端统一。陆井然复核冲突后定稿。 |
| 2 | 高亮通道 | v1 **复用 `set_backgrounds`**（最小改动、gallery 已验证）；`set_find_matches` 记为「第二用途出现时」的收敛点，本批不加。 |
| 3 | FindWidget 文案 | **方案 A**：走组件库 `i18n::text` + `locales/ui.yml` 新增 `editor.find.*`（符合 develop-lithe 硬编码规则）。 |
| 4 | 查找选项持久化 | v1 **不持久化**，会话内保持；跨会话持久化列为后续。 |
| 5 | newFile/newTab 语义 | `newTab` = 未命名 scratch 缓冲（Plain text，`Untitled-N`）；`newFile` = 项目内真实文件（对话框选目录+名字）。暂不做 New Scratch File。 |
| 6 | 另存为 | **保留原文件**（写新文件、缓冲改指新路径）；覆盖冲突交给平台对话框，不额外二次确认。 |
| 7 | 无活动缓冲 | **no-op + 诊断日志**；菜单置灰列为后续（菜单暂无 disabled 通道）。 |
| 8 | 文件写操作 | v1 走 **`std::fs`**（与既有 `write_buffer` 一致），Core 契约不动。 |
| 9 | 删除 | v1 **强确认对话框 + 直接删除**；回收站留 `Delete` 端口可替换点，trash crate 列为后续。 |
| 10 | explorer 重命名 | **行内编辑（方案 A）**，复用库内 `Editing` + 通用 `Tree::on_rename` 原语。 |
| 11 | 「增」 | **纳入本批**：右键 New File / New Folder；New File 与 B2 `newFile` 共用同一落盘入口。 |
| 12 | 目录改删的 LSP 通知 | v1 只做 remap / 关闭受影响文档；LSP closeDocument/reopen 与断点重发**列为后续**（LSP 属 C 批）。 |

约束：Core 契约不动；`host → lithe-ui` 单向依赖不变。

---

## 9. 后续单（主理人收集，不占本批）

| # | 事项 | 来源 | 说明 |
| --- | --- | --- | --- |
| S1 | 菜单图标补齐（系统性单） | 颜思齐 §10 | `IdeaIcon` 是 186 条精选子集，导致成对动作一有一空：`Cut/Copy` 有图 vs `Paste/SelectAll` 空；`Save/RevertFile` 有图 vs `SaveAs/SaveAll` 空；`Find` 有图 vs `FindAndReplace` 空；`NewTab/NewFile` 同图。补齐需另取上游 SVG 并同步登记 `idea.rs::ALL`。 |
| S2 | `Button::tone()` 的 Danger 臂用 `colors.on_accent` 而非 danger 专用前景 | 颜思齐 §10 | 潜在耦合（当前两主题刚好达标）；建议加 `on_danger` 或让前景从 `danger` 反推。 |
| S3 | `revertFile` 用 `REFRESH_ICON`（"刷新"≠"还原"） | 颜思齐 §10 | 语义借用，属 S1 同一批。 |
| S4 | `CodeEditor::set_find_matches` 收敛点 | 架构师 §2.6-2 | 当出现第二用途（括号匹配/当前词高亮）时把 `set_backgrounds` 收成查找专属 API。 |
| S5 | 菜单动态 `disabled` 通道 | 裁决 #7 后续 | 库内 `MenuItem::disabled(bool)` 已存在（`menus/model.rs:107`），只需给 workbench 的 `MenuItem::Action` 加标志 + 菜单表动态化。用于「无活动缓冲」置灰。 |
| S6 | 系统回收站删除（trash crate） | 裁决 #9 后续 | 本批留 `Delete` 端口可替换点；需跨平台 trash 抽象。 |
| S7 | rename/delete 后通知 LSP（closeDocument/reopen）+ 重发断点 | 裁决 #12 后续 | 属 C 批 LSP 接线。 |
| S8 | `FindWidget` 宽用 `label_width()*2.75` 语义借用 | 颜思齐 §2.2 | 建议加 `Theme::find_bar()`；本批实现 PR 需注明是借用。 |
| S9 | `stories.json` 的 `ReplaceWidget` 是假组件 | 颜思齐 §2.7 | 实为 `FindWidget.replace(&field)` 展开态，建议从 gallery `components` 删掉。 |
| S10 | `find_all` 每击键同步 regex 的 debounce | 架构师 §2.7 | 2 MiB 上限下可接受；连续输入可加 ~120ms debounce。 |

## 10. 实现期需知的视觉验收基线（颜思齐 §10 / 陆井然交互稿）

- **未聚焦选区对比度**：`F3` 步进后编辑器选区处于未聚焦态（`editor/line.rs:191-195` 半透明减半），叠加当前匹配后 浅 **2.09:1** / 深 **3.37:1**。**验收基线取「深 3.37:1 / 浅 2.09:1 + 匹配滚动居中 + 计数文案变化」，不要要求单靠选区洗色本身达 3:1**（那会永远不过）。未聚焦不画光标/当前行底/其它出现处是组件统一的「编辑器已失活」约定，**不要为查找条改库**。
- **删除确认框**：`ConfirmDialog::destructive()` 四状态两主题全达标（标签 浅 5.26:1 / 深 6.37:1），可直接复用，不必自绘。
- **行内重命名错误提示（口径修正）**：选中行底色 `colors.active` 上 `colors.danger` 文字浅色仅 **4.44:1**（差 0.06 不到文字门槛 AA 4.5）。改为 **`Input::invalid(true)` 红框 + 提示文字用 `colors.fg`（14.6:1）**。红框达标依据是**非文本门槛 3:1**：框 vs 所在行底 `colors.active` = 浅 4.44:1 / 深 4.78:1 ≥ 3:1（数值与红字相同，但搬到描边即换到门槛更低的通道，故合规）。
- **File 菜单项序**：`newTab → NewWindow → newFile`，**不要移动 `New Window`**（移了会让 `NewTab/NewFile` 真相邻，加重同图难辨，且会碰顺序敏感守卫）。
