# Language Service: Session & Events

`rust/gpui/crates/java/` 的运行时半边。发现（discovery）、JDK 解析和工作区索引缓存在[父页面](../java.md)；这一页涵盖 session 信封、事件泵，以及语义请求路径。

状态概览：

```
Session                    session.rs:101-107
  id, events: Arc<SessionEvents>, pump: EventPump

SessionEvents              events.rs:139-161
  pending: Mutex<HashMap<operationId, Sender<Result<Value,String>>>>  # in-flight waiters
  lifecycle: Mutex<Lifecycle> + Condvar                              # queued Vec<Value> + terminal
  diagnostics: Mutex<HashMap<uri, Vec<JavaDiagnostic>>>              # latest snapshot only
```

## `lsp.startServer` payload

`Session::start`（`session.rs:127-214`）构造的是完全结构化的直接启动——没有 wrapper 参数。信封超时为 30 s（`:181-185`），只覆盖“启动进程并写入一帧”的时间，因此 UI 后台任务不会因为 `initialize` 截止时间而挂住。所有路径都会经过 `jdtls::normalize_path`（`:882-890`），它会把 `\\?\UNC\server\share` 转成 `//server/share`，去掉 verbatim 的 `\\?\`，并把 `\` 转成 `/`——这是必须的，因为 JVM 会拒绝 verbatim 路径。

pump 会在 `startServer` 成功后立即启动（`:210-211`）：握手阶段的 `stateChanged` / `log` / `serverInfoChanged` 事件已经排队，如果延后启动就可能错过 ready signal。pump 启动失败会导致整个 start 失败。

## Event pump

```
loop {
  if stopping -> break
  match wait_events(session_id, PUMP_SLICE = 1s) {
    Ok(batch)  -> dispatch each; if terminal_reason().is_some() -> break
    Err(error) -> break   // ANY failure means the session is unusable
  }
}
events.mark_terminal("事件泵已退出（{reason}）")
```

`PUMP_SLICE` 并不是轮询周期：pump 会阻塞在 Core 的 condvar 上，并在活动到来时立刻返回。它只约束空闲 session 向 Core 询问的频率，以及 `stop()` join 的最长时间（`events.rs:51-56`）。

| 事件 | 处理 |
| --- | --- |
| `requestCompleted` | 按 `operationId` 移除 waiter 并 `send` 结果；未知 id 会递增 `unclaimed` 并记录 `S1_JAVA_PUMP unclaimed` |
| `diagnostics` | 每个 `uri` 仅保留最新快照，并输出 `S1_JAVA_DIAGNOSTICS uri=… version=… count=… dropped=…` |
| `log` | 输出 `S1_JAVA_LOG level=… message=…`，连续重复会被抑制 |
| `stateChanged` / `serverInfoChanged` | 推入一个**队列**并 `Condvar::notify_all`；terminal state 也会 `mark_terminal` |
| 其他 | 计数，并且只有第一次看到该类型时打印 `S1_JAVA_PUMP ignored type=…` |

**单消费者原则。** `lsp.waitEvents` 具有 drain 语义，因此第二个消费者会把另一个消费者的事件“偷走”。所以 `wait_events` 被设计成一个**自由函数**，接收的是 `session_id`，而不是 `&Session`，从而避免任何人随手拿到第二个读句柄（`session.rs:442-446`）。

`Lifecycle::queued` 是一个**队列，不是单元格**（`:168-169`）：如果它是“最新状态”的 cell，那么一个在调用方开始等待前到达的 `stateChanged: ready` 可能会被覆盖，从而让 `wait_ready` 卡到绝对超时。

锁顺序是 `pending → lifecycle`，并且 `mark_terminal` 会故意**先释放 `lifecycle` 再拿 `pending`**，这样两者永远不会嵌套（`:383-394`）。

`EventPump` 是一个普通的 `std::thread`，而非 gpui `Task`——因为 `Task` 一旦被 drop 就会取消，而 pump 必须在效果完成前继续存活，并且该 crate 必须保持 UI 无关（`events.rs:499-505`）。

## `wait_ready`

它循环调用 `events.wait_lifecycle(min(remaining, 5s))`，并且**不自己读取事件**（`:222-288`）。当返回空切片时，它会检查 `terminal_reason()`，这样在会话已死时能马上报错，而不是等到 600 s 的绝对超时。`stateChanged: failed` 会打印**完整事件**——这是唯一可用的崩溃取证信息。

## `Session::request`——发送前必须先注册

```
operation_id = "lithe-gpui-java-{n}"
body += { sessionId, operationId, operation }
let waiter = events.register(&operation_id)?;      // MUST precede the send
if let Err(e) = core_json("lsp.request", body) { events.unregister(..); return Err(e) }
match waiter.recv_timeout(deadline - now) { .. }
events.unregister(&operation_id);
```

> Core 会主动终止那些文档已经过期的 in-flight 请求（`lsp/interface/engine.rs:4289-4346`），而响应窗口可能非常短，所以晚注册会让结果被看作 unclaimed 并被丢弃（`:345-346`）。`register` 还会拒绝重复的 `operationId`，如果 session 已经处于 terminal 状态，就会快速失败。

## Shutdown —— 三个强制步骤

`Session::shutdown(self)` 是消费型方法（`:382-421`）：

1. `lsp.stopServer`，超时 20 s。
2. **有界等待 terminal**——`events.wait_terminal(now + 5s)`。pump 仍然存活，它负责观察 `stateChanged: stopped`。如果它永远不进入 terminal：会打印 `S1_JAVA_SESSION destroy_skipped reason=not-terminal`，因为 JVM 可能已经孤儿化，日志会显式提醒。
3. `lsp.destroyServer`，随后 `pump.stop()`。

> 这两步都必须存在，而且中间的等待是**强制的**。`stopServer` 只会**发起**一次有界的优雅关闭，并**保留 session 记录**；`destroyServer` 会拒绝非 terminal session（“A running language-server session cannot be destroyed.”）。不等待就直接调用它们会出现 `invalid_request`，并留下 `<cacheDirectory>/jdtls/<key>` 中**损坏的索引**和一个**孤儿 JVM**（`:374-381`）。

`impl Drop for Session` 和 `impl Drop for EventPump` 是兜底逻辑，用于处理“没有显式调用 `shutdown()` 的路径”（例如失败的 `wait_ready`）——没有它们，线程会永远卡在 `waitEvents` 上。

## `JavaLanguageService` orchestration

`start_locked`（`service.rs:661-746`），在 state lock 已持有时执行：

```
1. jdtls::resolve(&workspace_root)                            -> S1_JAVA_JDTLS
2. jdtls::resolve_runtime(installation_root(&executable))     -> S1_JAVA_TOOLCHAIN
3. workspace::plan(&root, &installation.version)              -> S1_JAVA_CACHE directory= key= reuse=
4. workspace::cleanup_expired(&cache, &key)                   -> S1_JAVA_CACHE_RECLAIM
5. workspace::maven_context(&root)                            -> S1_JAVA_MAVEN
6. directory_uri(&root)                                       -> S1_JAVA_START
7. Session::start(&spec)                                      -> S1_JAVA_SESSION started
8. session.wait_ready(now + 600s)                             -> S1_JAVA_READY elapsedMs= logEvents=
9. cache (installation, runtime) into self.installation
```

`is_java_workspace`（`:754-792`）会再次调用 `workspace.snapshot`，而不是复用项目树的私有状态，所以策略输入来自 Core，而不是 `Explorer` 自己的状态。代价是项目打开时多扫描一次（测量 < 1 s）。它**不参与** `definition` / `completion` / `code_actions` / `sync_document` —— 合约允许宿主在用户打开 `.java` 文件时按需启动（`:234-236`）。

## Semantic requests

| 操作 | 超时 | 说明 |
| --- | --- | --- |
| `definition` | 60 s | `JavaTarget::{File, Virtual}`；`Virtual` 会带一个 `jdt://` uri + display path + text |
| `completion` | 15 s | `textEdit` **必须保留**——没有它，`Sys` + `System` 会变成 `SysSystem`（`:117-121`） |
| `codeActions` | 15 s | `context.diagnostics` 是**强制字段**——JDT 会根据请求中的诊断反向查找问题位置，`getProblemId` 会把 `code` 解析成整数（`:381-393`） |
| `virtualDocument` | — | 用于 `jdt://` target 的反编译 source |

代码操作处理中的不明显要求：

- 编辑区间按**逆位置**排序，以保证应用时不会让后面的 range 发生偏移。
- 只过滤到当前缓冲区能落地的 action：其他文件的 edit 和任何带 `command` 的内容都会被丢弃，并记录到 `S1_JAVA_CODE_ACTION … other_file_edits=… commands=… malformed=…`。
- `diagnostics_for` 在空 selection 时使用光标的**行号**：当 range 是 zero-width 时，`Ctrl+.` 在缩进位置上会找不到任何结果，菜单就会退化为只显示 source actions（`:1049-1075`）。
- Windows 下 `/C:/…` 的规范化：Core 的 `file_path_from_uri` 在 `file:///C:/x` 上会保留一个前导 `/`，所以只在后面匹配 `X:` 驱动器模式时才去掉一个前导 `/`（`:1027-1039`）。
- Completion snippets 由 **Core** 处理（`lsp.plainSnippet`），不能在宿主本地转换；如果转换失败，返回原始值而不是丢掉候选项（`:1178-1202`）。
- `file_uri` 是**有意公开**的——编辑器的 quick-fix provider 必须使用同一套 URI 映射（`:912`）。

## Staleness and failure handling

三种机制，全部都在这个 crate 里：

1. **`is_superseded`**（`:938-940`）匹配错误前缀 `staleDocumentVersion@` 和 `requestCancelled@`。两者都是**正常的快速打字现象**，会被转换成 `Ok(empty)`，而不是 error；否则编辑器会把 cancelled completion 当成“server unavailable”，导致 JDTLS 与 builtin 候选项来回抖动。错误字符串格式 `"{code}@{stage}：{message}"` 是一个**兼容性表面**，仅由 `events::completion_outcome` 生成（`:456-459`）；改动其中一边而不改另一边会破坏两者，并且测试会锁死两边。
2. **Terminal 传播**（`events.rs:383-403`）：一旦 `mark_terminal` 运行，`register` 会立刻失败，并且**所有** in-flight waiter 都会收到 `Err(terminal_reason)`，而不是等自己的 15 s / 60 s 超时。
3. **Diagnostic snapshot settling**（`service.rs:524-542`）：`code_actions` 会在 `sync_document` 之前拍一份诊断快照；如果 `changed == true`，它会等待最多 2 × 250 ms，直到出现一个**不同的**快照（退出条件是 “changed”，而不是 “non-empty”），因为修复所有错误后结果也可能为空。这个等待是有界的；如果新错误与旧错误字节相同，等待直接超时，手里这份快照仍然是正确的。

**启动失败后不会自动重试**（`service.rs:19-22`）：启动失败本质上是环境问题，若每次 `F12` 都重试 30 s 以上，会让每次导航都变成长阻塞。先打一条诊断日志，然后让 batch-1 的轻量 navigation 接管。

诊断只有每个 `uri` 的**最新快照**；空数组会同时表示 "server cleared" 和 "never published"（`events.rs:246-250`），这也是为什么编辑器会在有界延迟后重新读取。

## Core commands used

`workspace.snapshot`、`java.workspacePolicy`、`java.jdtWorkspaceFingerprint`、`java.jdtCacheRetention`、`lsp.jdtWorkspaceKey`、`lsp.startServer`、`lsp.waitEvents`、`lsp.syncDocument`、`lsp.request`、`lsp.stopServer`、`lsp.destroyServer`、`lsp.plainSnippet`、`maven.scan`。

## Known gaps

- **`JavaLanguageService` 没有 `Drop`。** `EditorPane::drop` 必须同步调用 `shutdown()`；编译期无法强制保证这一点。
- `installation` 是私有的，没有访问器，因此设置页的“detected language servers”行会显示为“unreadable on this side”（`gpui/crates/settings/src/dialog.rs:823`）。
- 真正的 JDTLS smoke test 受 `LITHE_GPUI_JDTLS_SMOKE` 控制，而且还需要 `LITHE_JDTLS_JAVA` 指向 JDK 21+（`service.rs:1554-1561`）。
- 该服务只支持单 workspaces，不支持 root switch，因为 `EditorPane::prepare_java` 是幂等 early-return。

## Sources

- `rust/gpui/crates/java/src/session.rs`, `events.rs`, `service.rs`
- `rust/gpui/crates/editor/src/{navigation,diagnostics,completion,code_actions}.rs`
- `rust/gpui/crates/editor/src/editor_view.rs`
- `rust/gpui/crates/settings/src/dialog.rs`
- `rust/lithe-core/src/lsp/interface/engine.rs`
- `rust/lithe-core/src/lsp/lightweight/snippets.rs`
- `shared/fixtures/lsp/`
- `shared/contracts/rust-core-api.md`
- `.agents/notes/implemented/architecture/2026-09-13-language-tooling-and-lsp-runtime-ownership.md`
- `.agents/notes/implemented/architecture/2026-09-18-java-project-build-and-launch-boundary.md`
