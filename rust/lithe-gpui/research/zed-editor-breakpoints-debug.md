# 调研：Zed 的编辑器 / 断点 / 调试是怎么做的，Lithe 的 gpui 宿主该怎么做

状态：调研中（Lithe 侧盘点是实测结论；Zed 侧三路并行调研回报后补齐）

调研对象：
- Zed 源码快照 `D:\developmentProjects\rust\zed`（**非 git 仓库**，仅作只读参考）
- Lithe 宿主 `rust/lithe-gpui/`、内核 `rust/lithe-core/src/debug/`

---

## 一、Lithe 现状盘点（实测，含 `文件:行号`）

### 1.1 内核已经拥有 DAP 的"协议 + 状态机"，宿主只负责字节传输与进程生命周期

- `rust/lithe-core/src/debug/mod.rs:1` —— **"Transport-neutral Debug Adapter Protocol state and normalized debugger models."**
  模块划分：`protocol.rs`（DAP 分帧与 JSON 助手）、`engine.rs`（状态机，3886 行）、`types.rs`（707 行）、
  `breakpoint_relocation.rs`（322 行）、`java_test.rs`（264 行）。
- `rust/lithe-core/src/debug/engine.rs:1` —— **"Stateful DAP reducer whose byte transport and process lifecycle remain platform owned."**
  即：**分帧、请求匹配、事件归约、断点同步都在 Core；spawn adapter、读写管道、进程存活归宿主。**
- `engine.rs` 对外是一组 `debug.*` 命令（`rust/lithe-core/src/debug/engine.rs:80` 起）：
  `create_session`（返回**要发出的 `initialize` 帧**）、`launch`、`execute`、`inspect`、`receive`（喂入对端字节/消息）、
  `set_breakpoints` / `set_exception_breakpoints` / `set_function_breakpoints` / `set_data_breakpoints`、
  `data_breakpoint_info`、`set_variable`、`cancel_operation`、`run_in_terminal_response`、`disconnect`、`destroy_session`；
  每条都返回 `DebugSessionUpdate`（`outbound_frames` + `events`）。
- DAP 分帧已实现且有边界：`Content-Length` 解析、`MAX_HEADER_BYTES = 64 KiB`、`MAX_MESSAGE_BYTES = 64 MiB`
  （`rust/lithe-core/src/debug/protocol.rs:6-8`）。
- Core 里注册的 `debug.*` 命令共 18 条（`debug.createSession` … `debug.steppingFilters`）。

### 1.2 宿主已经有的（不要重造）

| 已有 | 位置 | 说明 |
| --- | --- | --- |
| 断点数据模型与持久化形状 | `rust/lithe-gpui/crates/workbench/src/debug.rs:30`（`DebugBreakpoint`）、`:86`（`DebugBreakpointStore`） | 按文件分组、行号排序、`enabled`、整表静音（`muted`）、生成 `debug.setBreakpoints` 载荷（`:197`）、断点迁移载荷（`:248`） |
| 调试面板骨架（UI） | `debug.rs:2442`（`impl Render for DebugPane`） | 头部 + 工具条 + 「调试 / 控制台」两个页签 + 空态；已有 `DebugPaneEvent::BreakpointsChanged` |
| 编辑器断点 gutter | `rust/lithe-gpui/crates/editor/src/editor_view.rs:43`（`BREAKPOINT_GUTTER_WIDTH_SPEC = 14.`）、`:2140`（`breakpoint_gutter`）、`:548`（`set_gutter_breakpoints`）、`:81`（`DebugGutterHost`） | 编辑器正文**左侧**一条 14pt 自绘列；工作区把断点推给它（`workspace.rs:1247` `push_gutter_breakpoints`） |
| 断点交互入口 | `workspace.rs:719/725/731`（toggle / set_enabled / remove） | 编辑器 gutter 的点击已经接到 `DebugPane` 的 store |
| JDTLS 侧 java-debug 位置已知 | `crates/java/src/jdtls.rs:9`（`com.microsoft.java.debug.plugin-`）、`:120`、`crates/java/src/session.rs:79`（`javaDebugBundlePath`） | 只是把 bundle 路径告诉了 JDTLS；**还没有真的起 adapter** |

### 1.3 宿主**还没有**的（这就是"该怎么弄"要补的部分）

1. **DAP 传输层**：全仓没有任何地方 spawn debug adapter、也没有读写 DAP 字节的管道
   （grep `debug.createSession` / `debug.receive` / `debug.execute` 在 `rust/lithe-gpui/crates/**` **零命中**）。
   现在宿主只用了 3 条命令：`debug.setBreakpoints`、`debug.relocateBreakpoints`、`debug.steppingFilters`
   （`debug.rs:22-25`）。
2. **会话生命周期**：谁在"启动调试"时 `createSession`、谁把 `initialize`/`launch` 帧写给 adapter、
   谁把 adapter 的字节喂给 `debug.receive`、谁在 `terminated` 后 `destroySession` 并把 UI 复位。
3. **实时数据流**：`DebugPane` 目前只有静态骨架，没有"线程 / 调用栈 / 变量 / 监视 / 控制台"的真实数据源
   （`DebugSessionUpdate.events` 没人消费）。
4. **断点列位置与真源不符**（已登记 `BLOCKERS.md` **B18**）：真源 `macos/.../CodeEditorView.swift:410-468`
   的顺序是「行号列 → 断点列 → 代码」，而 gpui-component 的 `Editor` **没有公开的 gutter 缝**
   （0.7 里 `line_number` 只是主题 token `editor.line_number`，见 `gpui-component-0.7.0/src/registry.rs:446`；
   `gutter` 命中全是 plot 的坐标轴），所以 Lithe 只能在正文**左边**另起一条列。

---

## 二、Zed 侧结论

> 证据路径以 `D:\developmentProjects\rust\zed\` 为根，下面简写 `zed/…`。三路只读调研：
> ①编辑器 gutter/断点渲染、②DAP 客户端与断点存储、③调试 UI 与工作区集成。
> **②③ 已回报，① 待回报。**

### 2.1 Zed 的调试 UI 与工作区集成（调研③，已回报）

**结构**（`zed/crates/debugger_ui/src/`）：入口 `debugger_ui.rs`（actions! 定义 + `init()`）、
唯一 dock 面板 `debugger_panel.rs`（会话集合、多会话树、顶部工具条）、`session.rs`（`DebugSession` =
`Item` + `FollowableItem`，持 `running_state`）、`session/running.rs`（`RunningState` = **单会话 UI 真源**，
内嵌自建 `PaneGroup` + 8 个子面板）、各面板 `session/running/{stack_frame_list,variable_list,breakpoint_list,
console,module_list,loaded_source_list,memory_view}.rs`、`persistence.rs`（按 adapter 持久化布局）、
`dropdown_menus.rs`（会话/线程选择器）、`attach_modal.rs`/`new_process_modal.rs`。

**真源分层**（`dap::client` 进程/协议 → `project::debugger::Session` 项目层 → `RunningState` UI 层 → 面板）：
UI **从不反向拥有**调试状态，只读 session + 订阅事件（`running.rs:889-936` 订阅 `SessionEvent`，
`:940-948` 订阅 `SessionStateEvent::{Shutdown,Restart,SpawnChildSession}` → `cx.notify()`）。
变量树懒加载：UI 侧维护展开态 `entry_states` + 调 `session.variables()`（`variable_list.rs:333-381`），
缓存留在 Session。选中态（线程 `running.rs:70/1733`、栈帧 `stack_frame_list.rs:78/404`）在 UI。
多会话真源是 `DebugPanel.sessions_with_children` + `active_session`（`debugger_panel.rs:62-69`）。

**面板/布局**：外层 **一个** `Panel`（`debugger_panel.rs:1513-1600`；position 来自 `DebuggerSettings.dock`，
默认 Bottom；`pane()` 返回 None —— 面板不是 Pane），内部**自己复刻了一套工作台**：
`RunningState.panes: PaneGroup` + `default_pane_layout` 建 3 个 pane（左 Frames+Breakpoints / 中 Console+Variables /
右 Terminal，`running.rs:1925-2026`），每个子面板包成 `SubView`（实现 `Item`，支持拖拽/拆分）。
显隐唯一入口是 `pane_items_status`/`add_pane_item`/`remove_pane_item`/`ensure_pane_item`（`running.rs:1472-1528`），
右键菜单调它们（`debugger_panel.rs:545-594`）；能力探测会剔除不支持的子面板（`running.rs:917-928`）。
**自动切到调试视图**：`SessionEvent::Stopped` → `workspace.open_panel::<DebugPanel>()` + `activate_session_by_id`
（`running.rs:891-912`）；新会话 `register_session(..., focus=true)` → 聚焦面板（`debugger_panel.rs:353-372`）。

**工具条与命令**：Zed 是**双通道**——按钮直接调 `RunningState` 的方法
（`debugger_panel.rs:741-746` continue / `:716` pause / `:884-926` stop / `:938` detach），
键位走 Action 渲染器（`debugger_ui.rs:146-271`），两者最终都落到 `project::debugger::Session`。

**控制台/REPL**：不是终端，而是只读多行 `Editor` + 单行输入 Editor（`console.rs:62-103`），
输出用**拉模式** `Session::output(token)` + `has_new_output`（`console.rs:407-444`，`session.rs:1369/1373`）；
REPL 求值 `Console::evaluate` → `Session::evaluate(..., Repl, …)`，结果由 Session 自己推回输出缓冲
（`session.rs:2805-2860`）——**没有第二条结果通道**。真终端由 `SessionEvent::RunInTerminal` → `SpawnInTerminal`
另起（`running.rs:929-931/1282-1361`）。

**生命周期**：启动 `debugger::Start` → 模态 → `DebugPanel::start_session`（`debugger_panel.rs:184-310`）；
工作区侧还有 `Workspace::start_debug_session` → `DebuggerProvider::start_session` 回调路径
（`workspace/src/tasks.rs:201-220`、`workspace/src/workspace.rs:337-362`）。
**清理是惰性的**：`Terminated` 只改项目层状态（`session.rs:1546`），UI 用 `is_terminated()` 画删除线/禁用，
等到**下一个** session 注册时才 `retain_sessions` 丢弃（`debugger_panel.rs:1451-1459`）；显式关闭走
`close_session` → `session.shutdown()`；全局退出走 `ShutdownDebugAdapters` → `dap_store.shutdown_sessions()`
（`debugger_ui.rs:137-145`）。子会话：`startDebugging` 反请求由 `DebuggerProvider` 处理（`debugger_panel.rs:438-483`）。

**许可证**：`debugger_ui`/`dap`/`debugger_tools` 的 `Cargo.toml` 均声明 `license = "GPL-3.0-or-later"`
（各 `Cargo.toml:6`），目录内 `LICENSE-GPL` 在快照里是 0 字节空文件（全仓一致，未确证文本）。
⇒ **不能把 Zed 的这三份代码移植/逐行改写进 Lithe**，只能借鉴架构与调用顺序，实现自写；
协议类型可用 Apache/MIT 的 `dap`/`dap-types` crate 替代。

### 2.2 Zed 的编辑器 gutter / 断点渲染（调研①，已回报）

**gutter 不是组件、也不是 block，而是自定义 `Element` 的 prepaint 产物**：
`EditorElement`（`zed/crates/editor/src/element.rs:8408` `impl Element … PrepaintState = EditorLayout`）在 prepaint 里
手算出一整块 `EditorLayout`，其中 gutter 是 `Gutter<'a>` 定位器（`element.rs:6970`：行高 / 可见 `Range<DisplayRow>` /
滚动位置 / origin / hitbox），每个"格子"由 `layout_item`（`element.rs:7000`，先判 `range.contains`）产出、
用 `prepaint_as_root(origin + point(x, y))` 绝对定位（`element.rs:7028`），`paint` 阶段按
crease → expand → bookmark → breakpoint → test indicator 顺序重放（`element.rs:5542`）。
**gpui 本身对 gutter/断点零知识**（`crates/gpui/src` grep 零命中）；block（`display_map/block_map.rs:272`）是另一条路，
断点**不走** block。

**断点 = gutter 一格里的 `IconButton`**：`IconButton::new(("breakpoint_indicator", row), icon).on_click(…)`
（`editor.rs:4739`）→ `edit_breakpoint_at_anchor`（`editor.rs:6690`，平台修饰键或已禁用 → `InvertState`）
→ `BreakpointStore::toggle_breakpoint`（`project/src/debugger/breakpoint_store.rs:402`）→ 广播
`BreakpointStoreEvent`（`:915`）→ session 下发 DAP `set_breakpoints`（`session.rs:844`）。
**断点事件不进 `EditorEvent`**（`editor.rs:12534` 全枚举核对），UI 是"写 store + 反向订阅"。
外观由两个来源合成：`Breakpoint{message,condition,hit_condition,state}` + `BreakpointSessionState{verified}`
→ 四态图标（`editor.rs:4701`）与"未验证"灰化 + Warning tooltip（`:4730/:4742`）；行号染色条件更严
（`element.rs:8999`：启用**且** verified 才染 `debugger_accent`）。命中当前行**不走 gutter**，走通用行高亮：
`go_to_active_debug_line`（`editor.rs:6729`）→ `go_to_line::<ActiveDebugLine>` → `highlight_rows::<T>`（色 `editor_debugger_active_line_background`）。

**交互**：左键点 gutter 空白 = 选行（`element/mouse.rs:610`，模拟三击），**没有** toggle 断点；
右键走 `gutter_hitbox` 手工 hit test（`mouse.rs:763`）后开一个菜单（`editor.rs:4427`：Log / Condition /
Hit-Condition / Disable / Enable / Unset / Run to Cursor / Blame / Bookmark…）；悬停时有个"虚影加断点"按钮 +
200ms 防抖 timer（`mouse.rs:178-239`，存 handle 可取消）。

**共用机制与性能**：三类标记都是 `HashMap/Set<DisplayRow, _>` 稀疏 map（只对存在的行建元素，`element.rs:2614/2687/9649`），
**同一列互斥**（runnable > breakpoint > bookmark，`element.rs:9661` 三次 `retain`）；`EditorElement::id()` 返回 `None`
（`element.rs:8412`）⇒ `EditorLayout` 每帧全量重算；行号是唯一每行都 `insert_hitbox` 的部分（`element.rs:2923`）；
列宽用固定 `ch_width * N` + `min_line_number_digits` 防抖（`editor.rs:12342`）。
值得独立重写的抽象：`highlight_rows::<T>` / `go_to_line::<T>` / `highlight_gutter::<T>` / `insert_gutter_highlight::<T>`
（`editor.rs:9247/9357/9554/9573`，`navigation.rs:1681`；业务方 `agent_ui:1319`、`git_ui:311`、`repl:545` 都在用）。

**别照搬**：`DisplayMap`/`DisplaySnapshot`/`DisplayRow`/`DisplayPoint`/`InlayMap`/`BlockMap`/`Anchor`/`MultiBuffer`
这套"多 buffer + 折叠 + inlay + block 融成一个显示空间再做双向换算"的地基（`display_map.rs` 单文件 172 KB）——
`active_breakpoints`（`editor.rs:4373-4422`）一半篇幅都在做 anchor↔display point 换算，而 gutter/断点只需要"行号 + 行高"。
Zed 自己的教训也值得引以为戒：状态双源（未验证断点仍画点却不染行号）、高亮系统两套并存
（`active_rows` 硬编码路 vs `highlighted_rows` 泛型路，`element.rs:10200`）、互斥判定散在 3 处。

**许可证**：`crates/editor`、`project`、`dap`、`debugger_ui`、`ui`、`workspace` 等全部
**GPL-3.0-or-later**（Cargo.toml `license` 字段；`crates/*/LICENSE-GPL` 在快照里是 0 字节空文件，
真文本在仓库根），只有 `crates/gpui` / `gpui_platform` 是 **Apache-2.0**。
⇒ 可以读它理解架构、独立重写，但**不能逐行誊抄/贴改**，`ui::IconButton`、`IconName::DebugBreakpoint` 等
图标与组件也需自备。

### 2.3 Zed 的 DAP 客户端与断点持久化（调研②，已回报）

**`crates/dap` 结构**：`transport.rs`（分帧 + Stdio/Tcp 两种传输）、`client.rs`（会话 + 请求/响应匹配）、
`adapters.rs`（`DebugAdapter` trait、`DebugAdapterBinary`、下载器）、`registry.rs`（`DapRegistry`/`DapLocator`）、
`debugger_settings.rs`、`proto_conversions.rs`；消息类型**不在 Zed 内**，来自外部 `dap-types`（根 `Cargo.toml:608`
固定某个 fork rev）。

- **传输二选一**：`transport.rs:92-100`，有 `connection` 就 TCP（先起进程再连端口，`Child::spawn` :526），
  否则 stdio 管道（:675），stderr 转 adapter 日志（:677）。帧就是 `Content-Length: N\r\n\r\n`。
- **请求匹配**：每请求递增 `next_sequence_id`（`client.rs:169`，AtomicU64）→ 插 `PendingRequests`（seq→oneshot，:112）
  → `callback_rx.await`（:129）；回包按 `request_seq` 取 waiter，取不到就当作**反向响应**交给 message handler（`transport.rs:350-359`）。
- **事件分发**：唯一句柄 `DapMessageHandler`（`client.rs:39`）→ 上层接 mpsc（`session.rs:204/920`）→ UI `cx.subscribe`（`running.rs:891`）。
- **超时与取消（Zed 的缺口）**：只有 **TCP 建连超时**（`transport.rs:605`，默认 2000ms，`debugger_settings.rs:18`），
  **没有逐请求超时、没有心跳**；DAP 的 `cancel` **全仓未使用**，取消只有 `kill()` + `shutdown()`
  （`client.rs:173`）。断连时 `PendingRequests::flush` 把在途请求统一置错（:114）。
- **多会话/子会话**：`create_child_connection`（`client.rs:75`）——父会话是 TCP 时子会话复用同一连接参数、不重复起进程；
  反向 `startDebugging` 由 `session.rs:1136-1171` 解析成 `SessionStateEvent::SpawnChildSession`。
- **初始化顺序**（最值得抄的一条）：`initialize`（`session.rs:1270`，客户端能力硬编码在 `dap_command.rs:1535`）
  → **等 `initialized` 事件**（`session.rs:465`）→ 按文件 `setBreakpoints` → 异常断点 → `configurationDone`
  （仅当能力支持，:521）；`launch`/`attach` 与配置序列用 `try_join` **并行**（:530）。`stopped` 事件
  `handle_stopped_event`（:1463）会停线程、失效 stackTrace 缓存再 emit（:1509）。
- **收尾**：`Terminated` → `shutdown`（:1546）；`Session::shutdown` 优先 `terminate`（能力支持）否则
  `disconnect(terminateDebuggee=true)`，最后 `kill()`（:2293）。

**断点：全局单例 + per-session 状态 + 按文件全量重发**

- 数据结构：`Breakpoint{message,hit_condition,condition,state}`、`BreakpointState::{Enabled,Disabled}`
  （`project/src/debugger/breakpoint_store.rs:935-968`）；内存位置用 `text::Anchor`，只在发给 adapter 时
  `line = row + 1`（:1065）；每个断点带 `StatefulBreakpoint{session_state: HashMap<SessionId, BreakpointSessionState{id, verified}>}`
  （:39-62）——**同一断点在每个 session 各有一套 adapter 分配的 id / verified**。
- **持久化不在 dap/project，而在 workspace 的 SQLite**：`SerializedWorkspace.breakpoints`
  （`workspace/src/persistence/model.rs:147`）、表 `breakpoints(workspace_id, path, breakpoint_location,
  log_message, condition, hit_condition, state)`（`workspace/src/persistence.rs:1543`），
  保存时先 DELETE 该 workspace 全部再逐条 INSERT；**只在 Local 模式恢复**（`breakpoint_store.rs:825`）。
- **同步**：`Session::new` 订阅 `BreakpointStoreEvent::{BreakpointsUpdated,Cleared}`（`session.rs:843`）→
  按**单文件整体重发** `setBreakpoints`（:262）——没有 diff、没有内容 hash 去重，正因为它是幂等全量替换；
  会话启动时 `send_source_breakpoints` 按文件分组、**并发 join**（:344-415）。只发 `enabled` 的断点（禁用 = 不发）。
  回包按索引 zip 写回 `id/verified`（`breakpoint_store.rs:371`）。
- 断点集是**全局单例**（Project 持一个 `BreakpointStore`，`project.rs:221`），不按 session 分；
  会话级开关另有 `ignore_breakpoints`（`session.rs:1983`）。

**`dap_adapters`：Zed 根本没有 Java 适配器**。注册的只有 codelldb / Python(Debugpy) / JavaScript / Go / GDB
（`dap_adapters.rs:29-42`），全仓 grep `java`/`jdtls`/`java-debug` 无命中 ⇒ **"用 java-debug 调 Java" 没有可抄的命令行拼装**。
最接近的参考是 codelldb：同样"GitHub 下 VSIX → 解压 → 起独立 adapter"（`codelldb.rs:340-383` + 通用下载器
`adapters.rs:263-346`，`DownloadedFileType::Vsix`），而 java-debug 的 `com.microsoft.java.debug.plugin`
也以 VSIX 形式发布 —— Lithe 已有它的路径（`crates/java/src/jdtls.rs:9/120`），但**怎么起（JDTLS 的
`vscode.java.startDebugSession` 还是直接起 adapter）要自己定**，Zed 这边没有答案。

**许可证**：`dap`/`dap_adapters`/`project`/`debugger_ui`/`workspace` 全是 **GPL-3.0-or-later**
（Cargo.toml + 目录内 `LICENSE-GPL`；全仓 206 个 GPL / 34 个 Apache，Apache 那批就是 gpui 家族与 util/collections 等）。
⇒ **不能依赖、也不能复制**；唯一可用的现成件是类型定义：crates.io 的 `dap-types 0.0.1` 是 MIT OR Apache-2.0 双许可
（Zed 固定的那个 fork rev 的许可**未确证**，落地前要自行核对）。

---

## 三、结论：Lithe 的 gpui 侧该怎么弄

### 3.1 分层：Core 已经有 reducer，"进程 + 字节 + UI"是宿主的事（既有契约，不要另立）

Core 的 DAP 契约已经把边界划死了：`ReceiveRequest { session_id, data_base64 }` 进去、
`DebugSessionUpdate { outbound_frames（base64）, events }` 出来（`rust/lithe-core/src/debug/types.rs:24/343/438/492`），
`create_session` 返回**要发出去的 `initialize` 帧**（`engine.rs:80`），
`protocol.rs` 负责分帧且有边界（`protocol.rs:6`：64 KiB / 64 MiB）。
⇒ 宿主只做三件事：**spawn adapter、在两个方向搬字节、把事件变成 UI 状态**。

同仓两个先例直接照形状做，不要发明新写法：

| 先例 | 位置 | 借什么 |
| --- | --- | --- |
| LSP 的进程端口 | `rust/lithe-core/src/lsp/interface/process.rs:35-68`（`LspProcessHandle` / `LspProcessStreams` / `LspProcessLauncher`）、scripted 替身 `scripted.rs:283/306` | DAP 的"进程 + 流"端口形状与**可脚本化的测试替身**思路 |
| 终端会话的 IO | `rust/lithe-gpui/crates/terminal/src/session.rs:71/78/81/154/182/502` | 阻塞 IO 放专用线程（reader/writer）+ mpsc + `cx.spawn` 泵回 UI 线程 |

### 3.2 具体落点

1. **`DebugTransport`（宿主新模块，建议 `crates/debug/` 或先落 `workbench/src/debug/transport.rs`）**：
   - `trait DebugAdapterProcess { fn write_frame(&self, &[u8]); fn exit_status(); fn terminate(); }` +
     `trait DebugAdapterLauncher { fn launch(spec) -> streams }`（照 LSP 的 trait 形状），
     真实现 spawn java-debug/JDTLS 调试入口，测试用 **scripted adapter**（照 `scripted.rs`）。
   - 读线程把 adapter 的 stdout 字节 base64 后走 `debug.receive`；写侧把 `outbound_frames` 解码写出。
   - 阻塞 IO 在专用线程，UI 线程只收事件（照 terminal 的形状）。
   - 已知 bundle 路径无需再找：`crates/java/src/jdtls.rs:9/120`（`com.microsoft.java.debug.plugin-*`）、
     `crates/java/src/session.rs:79`（`javaDebugBundlePath`）。
   - **Java 的起法要自己定，Zed 没有答案**（调研②：Zed 的 `dap_adapters` 只有 codelldb/python/js/go/gdb，
     全仓无 Java）。行业标准做法是走 **JDTLS 的 `vscode.java.startDebugSession`**（java-debug 插件随
     JDTLS 起，命令返回一个调试端口，之后 DAP 走 **loopback TCP**）—— 这条正好复用 Lithe 已有的 LSP 通道：
     Core 的 JDT 命令层已经在发 `vscode.java.buildWorkspace` / `resolveClasspath` / `resolveMainClass` /
     `test.findTestTypesAndMethods`（`rust/lithe-core/src/lsp/interface/engine.rs:573/7829/7986`），
     **但 `vscode.java.startDebugSession` 现在没有** ⇒ 这是第 1 步要补的第一个具体东西。
     替代路线是像 codelldb 那样把 VSIX 解出 jar 直接起独立 adapter（`dap_adapters/src/codelldb.rs:340` +
     下载器 `adapters.rs:263`），但那样要自己管 adapter 进程与端口，不如复用 JDTLS。
   - **初始化顺序不用宿主操心**：Core 的 reducer 已经用 `did_receive_initialized` 把关
     （`engine.rs:201/238/278/374`：初始化事件到达前不发断点），并用 `supports_configuration_done` 决定
     是否发 `configurationDone`（`:1066`）。宿主要补的是 Zed 明确缺的那两件：
     **逐请求超时**（Zed 只有 TCP 建连超时，`transport.rs:605`）与**代次/陈旧保护**
     （Core 已有 `operationID` + `debug.cancelOperation`，宿主按会话代次丢弃过期回包）。
2. **`DebugWorkspaceState`（宿主 UI 真源，= Zed 的 `RunningState`）**：
   唯一订阅 `DebugSessionUpdate.events` 的实体，持"选中线程 / 选中栈帧 / 变量树展开态 / 面板显隐 / 活动会话"；
   面板订阅它而不是各自订阅 Core 事件（Zed 是各面板各订阅一次，面板一多订阅面就散）。
   `workbench/src/debug.rs` 现有的 `DebugBreakpointStore` + `DebugPane` 保留，扩成"状态 + 面板"两层。
3. **面板用 gpui-kit `DockArea` + 一面板一 `Panel`**（Console / Variables / Frames / Breakpoints / Modules…），
   **不要**照搬 Zed"外层一个 dock 面板、内部自建 `PaneGroup`"那层；布局持久化 key 直接用稳定字符串
   （可照搬 `debugger_ui/src/persistence.rs:19-28` 的枚举名），按 adapter 分 key；
   能力门控（无 `modules`/`loadedSources` 请求就不显示该面板）照 `running.rs:917-928` 的 caps 判定。
4. **控制台 = 只读编辑器 + 单行输入，输出走"拉模式"**（Zed `console.rs:407-444` + `Session::output(token)`）：
   Lithe 侧对应 `debug.inspect`/控制台输出缓冲，REPL 求值复用同一条输出通道（不要另开一条）。
   真终端（`RunInTerminal` 反请求）另算：Core 已有 `debug.runInTerminalResponse`，宿主用现有 terminal crate 起终端。
5. **断点**：
   - 数据模型**已经齐全**（更正我原先的判断）：`DebugBreakpoint` 已有
     `line/column/enabled/verified/message/condition/hit_condition/log_message`（`debug.rs:30`），
     Core 的 `SourceBreakpoint` 也一样（`types.rs`）。缺的是**界面**：四态图标（禁用 / 日志点 / 未验证）
     与右键菜单（条件 / 日志 / 命中次数 / 禁用 / 删除）—— 可照 Zed 的判定
     （`editor.rs:4701` 四态、`:4730` 未验证 tooltip），但把判定收敛成**一个**函数。
   - **断点目前不落盘**（实测：宿主 `debug.rs` 里没有任何 save/load，只有单测）。
     Zed 的参考是"全局单例断点集 + 每个 session 各存 adapter 给的 `id/verified`"
     （`breakpoint_store.rs:39-62`）与"按 workspace 存 SQLite"（`workspace/src/persistence.rs:1543`）。
     Lithe 应沿自己的落盘口径（`.lithe/**` 工作区配置）补一次持久化，字段直接用 `DebugBreakpoint` 的形状。
   - 事件方向：照 Zed 的教训，**断点变更不进编辑器事件枚举**，写 store 后反向订阅（Lithe 现在就是这样：
     `DebugPaneEvent::BreakpointsChanged` → `workspace.rs:1247 push_gutter_breakpoints`）✓ 保持。
   - 同步口径也照 Zed：**按文件整体重发 `setBreakpoints`**（幂等全量替换、不做 diff、只发 `enabled` 的），
     回包把 `id/verified` 写回本地（`session.rs:262/344`、`breakpoint_store.rs:371`）——
     Lithe 的 `set_breakpoints_payload`（`debug.rs:197`）已经是这个形状 ✓ 保持。
   - 渲染：现行做法是正文**左侧**一条 14pt 自绘列（`editor_view.rs:2140`），与真源不一致（B18）。
     要按真源（行号列右侧）就得走 Zed 的机制："自定义元素 prepaint + 稀疏 map + 真实交互元素绝对定位 + 每行稳定 ElementId"，
     在 gpui-kit 的 `Editor` 外面做**覆盖层**：Lithe 已经能拿到编辑器行高 / 滚动 / 可见行
     （`editor_view.rs:2145` 用 `visible_row_range`/`line_height`/`scroll_offset`），把标记按
     `y = (row - first_visible_row) * line_height - scroll_y` 定位到行号列右侧即可，点击用普通元素事件。
   - 三条"别照搬"：不要引入 DisplayMap/BlockMap 那套行空间抽象；不要做双源渲染判定（未验证/禁用/条件/日志
     收敛成**一个** `marker_style(bp, session_state)` 函数）；同一行只画一个标记（断点 > 书签 > 运行标记，
     互斥写在一处）。悬停虚影按钮带 200ms 防抖（可取消），列宽用固定 `ch_width * N` 防抖。
6. **命令单一入口**：Zed 是"按钮直调 `RunningState` + Action 渲染器"双通道，Lithe 统一成
   `Action` + `register_action!` + `debug::init(cx)` 注册，Toolbar 只 `dispatch_action`
   （能力门控照 `debugger_ui.rs:162-171` 的 caps 判定）。
7. **生命周期**：`terminated` 不立刻销毁面板（Zed 是惰性清理，`debugger_panel.rs:1451-1459`，否则重启/重连会闪）；
   显式关闭走 `debug.disconnect` + `debug.destroySession`；应用退出时统一收口（Core 已有 `disconnect`/`destroy`）。
   超时/取消用 Core 已有的 `operationID` + `debug.cancelOperation`，不要在宿主自己造一套。

### 3.3 实测缺口清单（就是"该从哪里下手"）

| 缺口 | 证据（实测） | 归属 |
| --- | --- | --- |
| DAP 传输：spawn adapter / 连端口 / 双向字节泵 | `rust/lithe-gpui/crates/**` 里 `debug.createSession`、`debug.receive`、`debug.execute` **零命中**；Core 契约要求宿主喂 `data_base64`、取 `outbound_frames`（`types.rs:438/492`） | 宿主新模块（`DebugTransport`） |
| `vscode.java.startDebugSession`（拿调试端口） | Core 的 JDT 命令层只有 `buildWorkspace`/`resolveClasspath`/`resolveMainClass`/`test.findTestTypesAndMethods`（`engine.rs:573/7829/7986`） | Core `lsp/interface/engine.rs` |
| 事件消费（threads / stackTrace / scopes / variables / output → UI 状态） | `DebugSessionUpdate.events` 目前无人消费；`DebugPane` 只是骨架（`debug.rs:2442`） | 宿主 `DebugWorkspaceState` |
| 断点持久化 | 宿主 `debug.rs` 无任何 save/load（只有单测）；Zed 是 workspace SQLite（`persistence.rs:1543`） | 宿主（沿 `.lithe/**` 口径） |
| 四态图标 + 右键菜单（条件/日志/命中次数/禁用/删除） | 数据字段已齐（`debug.rs:30`），界面判定未做 | 宿主 UI |
| gutter 位置与真源不一致 | `BLOCKERS.md` B18；真源 `CodeEditorView.swift:410-468` 是「行号列→断点列→代码」，而 gpui-component 无 gutter 缝（`registry.rs:446` 只有主题 token） | 宿主覆盖层 |
| 逐请求超时 + 陈旧回包保护 | Zed 只有 TCP 建连超时（`transport.rs:605`）；Core 已有 `operationID` + `debug.cancelOperation` | 宿主 |
| "Core 有、宿主没接"的 `debug.*` 命令守卫 | 18 条命令里宿主只调了 3 条（`debug.rs:22-25`） | 守卫脚本 |

### 3.4 落地顺序（每步都能单独验证）

| 步 | 内容 | 验收 |
| --- | --- | --- |
| 1 | `DebugTransport` + 会话生命周期：给 Core 的 JDT 命令层补 **`vscode.java.startDebugSession`**（拿调试端口）；接通 `debug.createSession/launch/receive/execute/disconnect/destroySession`；scripted adapter 单测 | 能对 java-debug 起会话；`initialize→launch→configurationDone→stopped` 在诊断锚点里可见；一个有界超时 + 代次丢弃回包的用例；scripted 单测不依赖真进程 |
| 2 | 实时数据流进现有 `DebugPane`：消费 `DebugEvent`（threads/stackTrace/scopes/variables/output），栈帧/变量懒加载、控制台拉取 | 断点命中后能看到线程/栈/变量；变量树展开按需拉取 |
| 3 | 面板化：`DockArea` + 面板拆分 + 按 adapter 持久化 + caps 门控 | 布局跨重启保持；无相关能力的 adapter 不显示空面板 |
| 4 | gutter 与真源对齐（收敛 B18）：覆盖层 + 四态标记 + 右键菜单（条件/日志/命中次数） | 断点在行号列右侧；条件/日志点生效；与真源顺序一致 |

### 3.5 一条现在就该补的守卫

Core 有 18 条 `debug.*` 命令，而宿主只调用了 3 条（`debug.setBreakpoints`/`relocateBreakpoints`/`steppingFilters`，
`workbench/src/debug.rs:22-25`）——**没有任何东西会提醒"某条 debug 命令一直没人接"**。
建议加一条守卫（照 `check-ui-px.mjs` 的形态）：列出 Core 的 `debug.*` 命令与宿主调用点，
输出"未接线"清单；这条清单就是上面四步的计划表，也能防止以后接了又断。

### 3.6 许可证红线

Lithe 是 **Apache-2.0**（仓库根 `LICENSE`），而 Zed 除 `crates/gpui` / `gpui_platform`（Apache-2.0）外
（`editor`、`project`、`dap`、`debugger_ui`、`ui`、`workspace`…）都是 **GPL-3.0-or-later**。
⇒ **只借鉴架构与调用顺序，不复制/不逐行改写代码**；图标与 `ui` 组件自备。
若将来想直接用现成 DAP 协议类型，选 Apache/MIT 的 `dap-types` 一类，而不是 Zed 的 `dap` crate。
