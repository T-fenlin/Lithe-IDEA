# Feature Panes: Explorer, Git, Terminal

三大侧边 / 底部面板由 shell 托管。三者都遵守同样的结构：一个数据侧模块负责所有 Core 调用和指标常量，一个视图侧模块负责渲染；三者都经过 `shared::core_client` 调用 Core，并且都是由 `ShellWorkspace` 持有的 gpui `Entity`。

---

## Explorer —— 项目树

最小的一个。`model.rs`（数据） + `explorer_view.rs`（视图）。

**唯一一个 Core 命令**：`workspace.snapshot { root }`（`model.rs:241`），在 `cx.background_spawn` 内发起，然后用 `this.update` 应用。快照是一个扁平的工作区相对路径列表；`PathTree::insert`（`:66-91`）把它聚合成多级树，然后 `build_tree_items`（`:129-157`）再展开成 gpui 的 `TreeItem`，交给 `uniform_list`。`RENDER_LIMIT = 5_000`（`:35`）。

状态：`LoadState { Loading, Ready, Failed(String) }`，其中字符串就是**Core 错误码原文**（`explorer_view.rs:130-141`），另外还有 `expanded: BTreeSet<SharedString>` 表示 `dir:<relpath>` 的 id、`active` 和一对 `WeakEntity` / callback，用于打开文件。

三个不太明显的规则：

- **空目录占位符是载荷逻辑。** `TreeItem::is_folder()` 在 gpui 中等价于 `!children.is_empty()`；如果目录为空，就会被渲染成“文件”行，并且点击时会把目录路径当作 `on_open` 输入。`empty_placeholder`（`model.rs:113-119`）通过一个 disabled row 补上占位（`:28-30`）。
- **行元素 id 基于 `TreeItem.id`，而不是行号**，因为 `Tree` 是 `uniform_list`，索引键在展开 / 折叠 / 过滤后会泄漏 hover/selection 状态（`explorer_view.rs:711-713`，测试 `:756-775`）。
- **`rebuild` 会清空选择**，因为 `TreeState::set_items` 会清空（`:336-337`）。

**这里没有文件系统 watcher。** `Explorer::refresh` 是 shell 唤起的入口，但右键 refresh action 没有实现，因此唯一调用点是在 `ShellWorkspace::new` 构造后立即触发。`git.status` 装饰和 auto-reveal 也在 crate 的 “not implemented” register 中（`lib.rs:86-106`）。

**没有陈旧保护。** `load`（`:289-332`）没有 `request-serial` 或 `generation` 检查，因此两个重叠请求可能乱序返回。这在 `git` crate 的 `request_serial` 模式下属于真实缺口。

---

## Git —— 两个独立功能

`lithe-gpui-git` 承载 **两个不相关功能 + 两个数据边界**：

| 文件 | 层 |
| --- | --- |
| `model.rs` | 底部面板的非渲染逻辑：Core 调用、分页 cursor 生命周期、行数据，以及约 **40 个 metric 常量**（`:78-233`）作为布局单一事实来源 |
| `log_view.rs` | `BottomPane` —— 提交日志 + console shell（`:105-150` struct，27 个字段） |
| `changes.rs` | 左侧栏数据层：`ChangeKind`、`ChangesSnapshot`、`load`、`write`、`stage` / `unstage` / `commit` / `discard` |
| `changes_view.rs` | `ChangesView` —— 源代码控制视图 |
| `branch_info.rs` | 标题栏分支弹窗的只读数据边界；`BranchSnapshot::load` 按设计是同步的 |
| `identity.rs` | Git 身份数据层，负责 settings 页的 `GitIdentityHost` hook |

### 两个独立状态机

`BottomPane.load_state` 是一个 **五值枚举**：`Loading, Ready, Stale, Failed, NoRepository`（`model.rs:242-261`）。`Stale` 表示“已有数据，但最近一次刷新失败”，这是两态视图无法表达的差异。

它的 `cursor: Option<HistoryCursor>` **持有一个真实的 `git log` 子进程**，因此 cursor 生命周期是整个 crate 最重要的不变量：它必须在恰好三个路径中被返回——首次 load（拿走并在后台任务运行前关闭，`:779-781`），失败的 “load more”（Core 已停止 session，`:336-340`），以及 `impl Drop for BottomPane`（`:2034-2042`）。漏掉任意一个都会泄漏进程。

`ChangesView.state` 是 `LoadState { Loading, Ready, Failed(String) }`，仅针对**读取**失败；写失败则走单独的 `error: Option<String>`，渲染成持续显示的红条，成功则用 `notice` 渲染绿条。`record_write`（`:683-709`）是统一尾部逻辑：先诊断、再展示失败，然后**总是** `refresh`。

`git.write` 返回 `ok: true`，即使 Git 退出非零；所以 `WriteOutcome::failure` 由 `operationError` / 非零 `exitCode`，以及首条有意义的 stderr / stdout 输出共同推导，且消息中**去掉了仓库根路径**（`changes.rs:488-589`）。

### 陈旧保护

`request_serial` 是单调递增的 `u64`。`spawn_first_load` / `refresh` 会 bump，它，`apply_first` / `apply` 会丢弃任何 serial 非当前的结果（`log_view.rs:233-251`, `changes_view.rs:327-358`）。

两个已知漏洞：`BottomPane::apply_more`（`:333`）和 `apply_commit_files`（`:398`）**没有 serial**，因此慢速 `historyPage` 追加或此前已选提交的 `commitFiles` 可能出现在新请求之后才落盘。只有首次加载路径被保护。

### 有意偏离库组件

这个 crate 在 `lib.rs` 中写明了 14 个偏离点。其中有几条会产生明显代价：

- **手绘图标按钮**出现在日志面板里，因为 `Button` 的图标渲染大小是 18 px，而目标要求 14 px——代价是**没有 hover tooltip**，只有 `aria_label`（`:148-151`）。`ChangesView` 则走了反方向的取舍：使用真正的 `Button`，因为 stage 按钮的 tooltip 很重要，但代价是行几何不统一。
- **手写 row，而不是 `Tree`**，因为 explorer 的 `Tree` row 没有 action slot，而且列表混合了类别头和可变高 row，而 `uniform_list` 无法处理（`changes_view.rs:8-31`）。代价是没有虚拟滚动，stage 按钮也始终可见，而不是 hover-only。
- **三列拆分使用 `relative()` 分数**，而不是 `ResizablePanel`（`0.19 / 0.57 / 0.24`，最小宽度 140 / 320 / 220）——所以这些列**不可拖拽**（`:132-137`）。
- **简化版 swimlane graph**：每条 lane 1.6 px 竖线 + 节点圆，跨 lane parent 边只是直竖线，没有弧线；缺失的 parent 用 `opacity(0.7)` 替代 `strokeDasharray`（`:138-140`）。
- **console tab 只是 shell**——完整的 chrome 渲染了（32 px toolbar，六个图标按钮，全部 `enabled = false`），但真正的输出需要 Git 执行事件通道，而这超出该 crate 的范围（`log_view.rs:1875-1930`）。

产品级约束：`%D` 装饰不能区分包含 `/` 的本地分支和远程 ref（`:623-624`）；Core 的 commit 格式是 `%s`，因此没有 commit body，Inspector 的 description 部分不会画；`commit` 总是作用在 index 上，因为没有按文件范围选择的复选框，所以计数标签是 `git.filesStaged` 而不是 `filesSelected`。

### 超时

`GIT_TIMEOUT_MILLIS = 60_000`（`model.rs:491`），通过 `with_timeout_millis` 应用（`:507`）。它故意**不遵循**共享的 120 s 默认，因为 Git 操作是交互式的，挂起提示比受限失败更糟。

### 身份

`identity.rs` 实现 settings 页的 `GitIdentityHost` hook。它返回的是 Core JSON **文本**，而不是 `lithe_gpui_git` 类型，精确地把 settings 与 git crate 解耦（`:232-260`）。`IDENTITY_MAX_BYTES = 1024` 按 **bytes** 而非 characters 计算，并且 `redact` 会把仓库根路径从消息中去掉。

---

## Terminal —— 无 PTY 的进程会话

`ansi.rs`（纯状态机） + `profile.rs`（shell 发现） + `session.rs`（进程生命周期） + `terminal_view.rs`（渲染） + `constants.rs`。

### 没有 PTY

`Session::start`（`session.rs:98-148`）使用 `std::process::Command` + 三个管道。Windows 方案本来会使用 `portable-pty` + ConPTY；但这个 crate 没有引入。后果在 crate 的常驻能力说明中写得很明确：没有完整的全屏 TUI（`vim`、`top`、`less`），`Ctrl+C` 不可用（关闭 tab 才能结束正在运行命令），没有 selection / find / links / cursor / mouse，软回车是按 viewport 而不是列格对齐。该说明位于 `constants.rs:66`，它是唯一一个不来自 locale catalogue 的用户可见字符串，故意保留。

每个 stream 有一个专门的 reader thread，最后合并进一个 `mpsc::channel<SessionEvent>`（`:210-236`）——到达顺序即展示顺序。`mpsc` 没有 async receiver，而且这个 crate 不依赖 `async-channel`，所以 pump 把阻塞的 `recv()` 包在 `cx.background_spawn` 中；`blocking_recv` 只存在为了确保 `!Send` 的 `MutexGuard` 不会泄漏进 `Send` async block（`:258-264`）。

### ANSI：过滤而非解释

`ansi.rs` 是纯状态机，没有 GPUI，因此可做单测。`EscapeState { None, Esc, Csi, Osc, OscEsc }`（`:24-36`）：

- `\r` 把 `cursor = 0`，随后字符**覆盖**前内容（`:181-188`）
- `\n` 结束当前行，`\b` / `\u{7f}` 为退格
- `Esc` -> `[` 为 `Csi`，`]` 为 `Osc`，其他情况直接丢弃，因此 `ESC ( B` 这类双字节序列会被完全吞掉
- `Csi` 在最终字节 `0x40..=0x7E` 结束；**所有 SGR 都被丢弃——没有任何颜色**
- `Osc` 在 BEL 或 `ESC \` 结束

跨 chunk 的 UTF-8 通过 `valid_up_to()` 处理：不完整尾部（`error_len() == None`）保留到下一段，避免出现伪造的 `U+FFFD`；真实错误会做 lossy replacement 并计数。已知限制：code page 936 的 `cmd.exe` 会把中文输出成 `�`。

### 会话是惰性的

`TerminalPane::new` **不启动任何进程**（`session.rs:275-280`）；宿主必须在第一次可见时调用 `ensure_session`。空 `tabs` 是正确状态，而不是“尚未就绪”。`ensure_session` 通过 `window.defer` 延迟 focus，因为输入行只在当前帧渲染完成后才挂载（`:331-344`）。

输入行只有在 `session.is_running()` 为真时才绘制（`terminal_view.rs:509`），并且**没有本地回显**——`cmd.exe` / `powershell.exe` 会在管道下自己回显提示和命令（`:649`）。

### 生成号保护和清理

`TabSession.run` 是生成计数器。`retry_tab` 会 bump `next_run` 并重设 `tab.run`；旧泵仍持有旧值，会返回 `PumpStep::Stop`（`:738-741`）并在 `settle_exit` 中处理（`:808-810`）。这是与 `execution_id` 相同的技巧。

三条清理路径，都是 `kill()` then `wait()`：`close_tab`、`retry_tab`，以及 `impl Drop for TerminalPane` 会循环处理**所有** tab。`Session::shutdown` 会先关闭写端，确保 shell 正常退出，然后再 kill、再 wait——`wait()` 被明确标注成非可选，以避免僵尸进程（`:169-181`）。

退出检测：两个管道都 EOF 后，`settle_exit` 会重试 `try_wait`，最多 `EXIT_POLL_ATTEMPTS = 10` 次，每次间隔 `EXIT_POLL_INTERVAL = 20 ms`。

滚动缓冲上限 `MAX_LINES = 10_000`，超出时从前端裁剪（`session.rs:784-787`），并且每次 append 时同步更新 `MessageScrollerState` 的 item count。

### 设置接线

`TerminalPane` 自己持有 `font_size: Option<f32>`，而不是直接写 `Theme::mono_font_size`，因为这个 token 属于 editor，改它会连带重设 editor 样式（`terminal_view.rs:101-106`）。`set_default_shell` 和 `set_font_size` **故意做成幂等**——shell 会在每次设置变更时都重新推送值，而 naive 实现会把用户从 tab-bar 菜单里选定的 profile 重置掉（`:468-475`）。

shell profile 在运行时用 `where.exe` 再 `which` 探测，不用 `#[cfg]`（`profile.rs:130-146`），并缓存到 `OnceLock`。`wsl` 只有在设置中显式命名时才可达。

## 参考资料

- `gpui/crates/explorer/src/` 下全部文件
- `gpui/crates/git/src/` 下全部文件
- `gpui/crates/terminal/src/` 下全部文件
- `gpui/crates/shared/src/core_client.rs`
- `gpui/crates/workbench/src/workspace.rs`
