# Editor

`rust/lithe-gpui/crates/editor/` 是 5 个模块，约 5400 行代码，负责标签栏、缓冲区、导航、诊断、补全和代码操作。这里没有 `tests/` 目录——测试都写作内联的 `#[cfg(test)] mod tests`。

| 文件 | 职责 |
| --- | --- |
| `lib.rs` | crate 契约、公开表面、三个 actions，以及一个显式的 out-of-scope 注册 |
| `buffer.rs` | 单个打开缓冲区的数据侧：读取 / 降级、`Buffer` 结构体、文件类型图标、语法语言、标签名去重 |
| `editor_view.rs` | `EditorPane` + 展示层：打开/关闭/保存/自动保存/重载、标签栏、标签上下文菜单、空状态、导航落点、诊断调度、`Render`、`Drop` |
| `navigation.rs` | 以 JDTLS 为优先的目标解析，配合 Core 轻量回退，三种列转换，`JumpHistory` |
| `diagnostics.rs` | 诊断数据侧：同步 + 有界重抓取、整快照写入、严重程度映射 |
| `completion.rs` | Java 补全：JDTLS 优先，回退到 `lsp.builtinCompletions`，并带生成次数保护的陈旧判定、触发策略 |
| `code_actions.rs` | Java 快速修复（`Ctrl+.`），并且**直接应用编辑自身** |

公开表面（`lib.rs`）：`EditorPane`、`CursorPosition`、`SessionRestore`、`TabMenuHostActions`、`install_actions`。这里注册了一个 action：`Ctrl+S`。

## `Buffer`

`buffer.rs:280-343`，只能通过 `Buffer::new` 构造（`:347-369`）。

| 字段 | 作用 |
| --- | --- |
| `path` | 打开时的路径；**去重键**和保存目标 |
| `name`, `icon` | 文件名和标签图标。图标在打开时解析一次，亮/暗主题绑定到打开时的主题（这是有意权衡，会带来可见后果） |
| `editor: Entity<EditorState>` | 每个标签页的文本状态 |
| `is_dirty` | **基于生命周期，而不是内容**：`InputEvent::Change` 置为脏，成功写入后清空。手动撤销编辑依然会保持 dirty，而且不会在每次按键时全量 materialize（`:298-304`） |
| `writable` | 是否该主体忠实表示文件；决定 readonly、dirty 和 save 逻辑 |
| `save_generation` | 防抖自动保存版本 |
| `revision` | 单调文本修订号，**永不重置**；用于导航和诊断保护 |
| `auto_save_task`, `diagnostics_task` | `Option<Task<()>>` —— 替换槽位本身就是 `clearTimeout` |
| `diagnostics_generation` | 最新调度刷新生成值 |
| `_subscriptions: Vec<Subscription>` | 必须持有； drop 时会取消订阅 |

**这个 crate 中没有行索引，也没有撤销栈。** 两者都在上游 `EditorState` 中：`Rope` + `RopeExt`。撤销只通过间接方式被使用：代码操作明确使用 `EditorState::apply_lsp_edits`，这样变更才会进入撤销栈；如果用 `set_value`，事件会被抑制（`code_actions.rs:58-63`）。

## 三种列约定

这是该 crate 中最重要的文档化不变量。三种约定只在 ASCII 下才相等（`navigation.rs:39-49`）：

1. **byte offset** —— `EditorState::cursor()`
2. **character column** —— `Position.character`，用于状态栏和 `DiagnosticSet::push`
3. **UTF-16 code-unit column** —— Core / JDT 的 `utf16Column`

两条函数是单一事实来源：

```rust
core_position(text, offset)   -> (u64 line, usize utf16_column)   // navigation.rs:231-245
editor_position(text, line, utf16_column) -> Position              // navigation.rs:247-258
```

用于导航、诊断（两端）、补全（两处 `text_edit` 端点）和代码操作（应用时）。带有非 BMP 字符的测试如 `🎉` / `🙂` 能证明它们不能互换（`navigation.rs:492-510`, `diagnostics.rs:393-448`, `completion.rs:582-594`, `code_actions.rs:505-537`）。

## Tabs

每个 `Buffer` 一个 tab，并按 `path` 去重：重新打开已打开路径只会激活它，不会重复读取（`editor_view.rs:762-778`）。`jdt://` 缓冲区按虚拟 URI 作为 key，并且是只读的（`open_virtual`，`:1272-1354`）。

| 字段 | 行号 | 角色 |
| --- | --- | --- |
| `buffers: Vec<Buffer>` | `:350-351` | 顺序就是 tab-bar 顺序 |
| `active: Option<usize>` | `:352-353` | `None` -> 空状态 |
| `closed: Vec<PathBuf>` | `:420-430` | LIFO 栈，`MAX_CLOSED_TABS = 20` |
| `hovered_tab: Option<usize>` | `:407-419` | × 只在非活动 tab 上显示。这个状态是手动维护的，因为 gpui 的 `.group_hover` 在测量时不会重绘（注释里有 artifact hashes） |
| `history: JumpHistory` | `:357-358` | `←/→` 定义跳转历史 |

元素 id **基于路径**，而不是索引：`editor-tab:<path>` 和 `editor-tab-close:<path>`（`:2752-2772`），回归测试证明顺序无关（`:2856-2875`）。

脏关闭确认使用一个共享对话框，三种结果：`PendingConfirm::{Single,Batch,Reload}`（`:310-319`）。批量关闭只会询问一次 N 个脏文件（`:1976-2019`），并按**高索引优先**关闭，以避免索引移位导致损坏（`:2026-2035`）。

## 状态

这里没有显式的 loading / failed 状态机；状态编码在每个 buffer 上。

| 状态 | 显示方式 |
| --- | --- |
| loading | `restore_entry` 通过路径重开已关闭目标，并等待 `open`（`:1387-1417`）；`FALLBACK_LINE_HEIGHT` 用于补偿 `line_height()` 在第一次布局前为 `None` 的情况（`:149-162`） |
| ready | 正常 `Editor` 渲染 |
| failed / degraded | `read_body` 用 `// ` 开头的通知体替换，并将它标记为非可写，因此编辑器变成只读，并且**不能覆盖真实文件**（`buffer.rs:93-148`, `editor_view.rs:845-851`）。四类错误：不可读、>2 MiB、二进制（包含 NUL）、非 UTF-8 丢失——全部有测试（`buffer.rs:409-481`） |

导航失败时，光标不移动，打印 `S1_NAV_FAILED reason=…`，并记录一个去重后的通知中心条目。保存失败时，脏状态保持为 true，打印 `S1_EDITOR_SAVE_FAILED` 到 stderr，并附带一个带去重 key 的 Error notification。销毁时，`impl Drop` 会同步调用 `service.shutdown()`——如果使用 detached thread，在 `main` 返回时会被杀掉，从而遗留 JVM（`:2803-2815`）。

## 导航

`F12` -> `NavigateToDefinition`（在 `lib.rs:236-240` 注册，位于 pane root 的 `:2830-2834` 处理，因此 action 会从聚焦编辑器冒泡）。`Ctrl+click` 走同一条路径：在 wrapper div 上监听 `on_mouse_down`，并检查 `Modifiers::secondary()` 和 `input_bounds().contains(position)`，因此点击空白区域不会跳转（`:1226-1270`）。

`resolve_target` 有三条决策规则，写在调用点（`navigation.rs:113-168`）：

1. 不是 `.java` 文件，或者没有 service -> **轻量**（Core `lsp.builtinNavigation`）
2. JDTLS 返回结果 —— 即便是 “no positions” 也仍然是**权威结果，不回退**
3. JDTLS 返回 `Err` -> 回退 + `S1_JAVA_UNAVAILABLE`

`apply_definition` 会做三层陈旧检查（generation、buffer 仍存在、revision 未变）后才落定 `Local` / `File` / `Virtual`，并且只有在成功后才记录历史记录。每次跳转都会记录 `S1_NAV_JUMP … via=builtin|jdtls|jdtls-virtual`。

`JumpHistory` 存的是 `JumpEntry { path, position }`——**路径，不是 buffer 索引**，因为关闭 tab 会移动索引。`MAX_JUMP_ENTRIES = 100`；溢出时最旧项会被丢弃，并且索引递减。当前状态：2026-09-27 后标签栏上的 `←/→` 按钮被移除，因此 `go_back` / `go_forward` 目前没有生产调用点（`#[allow(dead_code)]`）；建议复用方式是 `Alt+←` / `Alt+→`。

## 诊断

诊断来自 JDTLS 的 `publishDiagnostics` 快照，在 `sync_document` 后读取，并通过上游 seam `EditorState::diagnostics_mut()` 写入——这里是接线，不是自定义绘制。

- **写语义：先清空，再整体写入，绝不 merge**（`diagnostics.rs:200-210`）。
- **无缓存 / 无 replay。** 上游在每次文本改动后都会重置 `DiagnosticSet`，因此重放缓存快照会把修正过的代码又加回去（`:52-54`）。
- **节流：** 文本改动 `CHANGE_DEBOUNCE = 400 ms`，打开 / 准备 / 重载时为 `Duration::ZERO`。此值比补全的 120 ms 大很多，因为诊断要贵得多（`:100-105`）。
- **有界重试：** `RETRY_BACKOFF_MS = [250, 500, 1000, 2000, 4000]`，最多 5 次，≤ 7.75 s。停止条件是 **非空且与上一轮轮询相同**；空快照永远不会提前停止（`:125-132, 158-170`）。
- **三层陈旧保护**，每层都有日志：buffer 仍存在、generation 匹配、revision 匹配（`editor_view.rs:693-735`）。
- **已知耦合：** `highlight_lines` 在诊断样式计算前直接返回，所以 “没有语法高亮 => 没有红色波浪线”（`diagnostics.rs:23-27`）。

## 补全

- **JDTLS 优先，其次才是 Core 的** `lsp.builtinCompletions`（无进程、当前文件中的标识符，带前缀 `textEdit`）。
- **触发条件：** 上游在每次文本变化时调用 `is_completion_trigger`，所以“打字即弹出补全”无需键绑定；该判定接受标识符字符、`.` 和 `@`。
- **`autoCompletion` 控制的是触发，不是安装。** provider 仍然会一直安装，因此未来的 `Ctrl+Space` 可以直接调用 `completions()`，并且共享同一个 `Rc<AtomicBool>` 扳机，影响所有打开的 buffer（`completion.rs:108-129`）。
- **陈旧保护：** provider 级的 `Arc<AtomicU64>` generation；非当前 generation 返回**空响应**，这样菜单只是简单不更新，而不会展示旧前缀版本的候选项。
- **列正确性：** `text_edit` 端点使用当前 buffer 自身 rope 计算 `editor_position`；`insert_text_format` 强制为 `PLAIN_TEXT`。
- **已知边界：** 上游 `insert_completion` 忽略 `additional_text_edits`，所以 JDT 的“接受时自动补充 import”不会发生（`:338-340`）。

## 代码操作

`Ctrl+.`。provider 只有在存在 service handle 时才安装，因为它没有 fallback。三个不容易注意到的要求：

- 编辑区间保留 **UTF-16** 形式，`CodeAction` 值中只在应用时把它转回当前 rope —— 这一过程推迟到唯一可以正确计算的位置。
- 编辑按**后到先执行**顺序应用，因为 `apply_lsp_edits` 是顺序应用。
- 应用必须发生在返回的 `Task` 内：上游在已经持有该 entity 的可变借用时调用 `perform_code_action`，否则会 panic（`code_actions.rs:19-43`）。

## 语法高亮

双关键开关：扩展名 -> 语言*名称* via `language_for_file`，以及语法可用性 via **Cargo feature**。`gpui-kit` 申明了 `features = ["tree-sitter-java"]`，因此它隐含带上 `tree-sitter` + `tree-sitter-json`，Java 和 JSON 会一起点亮。`tree-sitter-languages`（35 种语法）被刻意关闭，以减小构建和二进制成本（`editor/Cargo.toml:8-19`）。

`language_for_file` 只有两个入口：`.java -> "java"`，`.json` / `.jsonc` -> "json"；这里有 typo 也几乎不可见，因此三种情况都被测试覆盖（`buffer.rs:488-518`）。`S1_EDITOR_LANG` 读取回存储的名称——这证明名称已落地，而不证明文本一定已着色。

## 性能约束

| 常量 | 值 | 位置 |
| --- | --- | --- |
| `AUTO_SAVE_DEBOUNCE` | 150 ms | `editor_view.rs:164-166` |
| `MAX_EDITOR_BYTES` | 2 MiB（直接拒绝，不退化） | `buffer.rs:15-21` |
| `CHANGE_DEBOUNCE` | 400 ms | `diagnostics.rs:100-106` |
| `COMPLETION_DEBOUNCE_MS` | 120 ms | `completion.rs:68-74` |
| `COMPLETION_MENU_WIDTH_PX` | 480（上游 320 会截断 Java 签名） | `completion.rs:55-66` |
| `MAX_CLOSED_TABS` | 20 | `editor_view.rs:440-445` |
| `MAX_JUMP_ENTRIES` | 100 | `navigation.rs:73-75` |
| `TAB_MAX_WIDTH` | 200 spec px | `editor_view.rs:140-147` |

**基于 drop 的取消是防抖原语。** 每个后台任务都放在字段里，替换槽位就等于取消旧任务——`auto_save_task`、`diagnostics_task`、`nav_task`、`java_task`。

值得记住的布局纪律：每个容器都 `min_h_0()` + `overflow_hidden()`；编辑器显式 `h(relative(1.))`，因为多行模式默认是 `.h_auto()` 且只保留 2 行；tab 关闭按钮使用**绝对定位**，以避免切换 tag 时宽度跳动；滚轮事件是手写的，因为上游 editor 样式没有 `overflow: scroll`，GPUI 因此不会把滚轮事件路由到它。

**`rem_px(rem, spec_px) = AbsoluteLength::from(rems(spec_px / 16.)).to_pixels(rem)`** 是仓库里唯一正确的 rem->pixel 转换（`editor_view.rs:103-113`，测试在 `:2889-2905`）。因为 `TabBar::max_width`、`Button::rounded`、`Dialog::width` 和 `Size::with_size` 只接收 `Pixels`；`/ 4.` 是错误的，1 rem = 16 px。

**渲染里仍然有工作待做，且代码会直说：** `render_tab_bar` 每一帧都会重算 `display_names(&self.buffers)`（`:2208`；O(n·depth) 的算法在 `buffer.rs:197-264`）。数十个 tab 时还可以接受，但没有 memoization。

## 参考资料

- `rust/lithe-gpui/crates/editor/` 下全部文件
- `lithe-gpui/crates/java/src/service.rs`
- `lithe-gpui/crates/notify/src/severity.rs`
- `shared/contracts/application-boundary.md`
