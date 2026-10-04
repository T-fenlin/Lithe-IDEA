# Agent 笔记：编辑器断点与 Java 调试的分层与落地顺序

状态：提议中

## 先说结论

调研完同目录的 Zed（`D:\developmentProjects\rust\zed`，非 git 仓库）后，结论是 **Lithe 不需要、也不能照搬 Zed**：
Zed 的 `editor`/`dap`/`debugger_ui`/`project` 全是 **GPL-3.0-or-later**（只有 `gpui`/`gpui_platform` 是 Apache-2.0），
而 Lithe 是 Apache-2.0（仓库根 `LICENSE`）⇒ 只能借鉴架构与调用顺序，不能复制代码。

真正要做的是**接上 Lithe 自己已经写好的那半截**：Core 已经有 DAP reducer
（`rust/lithe-core/src/debug/engine.rs:1`：**"Stateful DAP reducer whose byte transport and process lifecycle
remain platform owned"**，18 条 `debug.*` 命令、分帧、断点迁移、初始化顺序都在里面），
而宿主只做了断点 store、一个 `DebugPane` 骨架与左侧 14pt 断点列 —— **传输层与会话生命周期是空的**
（`debug.createSession`/`receive`/`execute` 在 `rust/lithe-gpui/crates/**` 零命中）。

完整调研与证据（三路只读调研 + 我自己对 Lithe 的实测）见
`rust/lithe-gpui/research/zed-editor-breakpoints-debug.md`。

## 问题

Lithe 的调试能力现在是**半截**的：Core 已经把 DAP 的协议与状态机做完（18 条 `debug.*` 命令、
分帧、断点迁移、初始化顺序），宿主侧也有断点 store 与一个 `DebugPane` 骨架，
但**没有任何地方真正起过 debug adapter、也没有搬过 DAP 字节** ——
全仓 `debug.createSession`/`debug.receive`/`debug.execute` 零调用点，所以断点永远停在"本地状态"，
用户看不到线程、调用栈与变量。另外三处缺口：断点**不落盘**（重启即丢）、
断点列只能画在正文**左侧**（与真源"行号列右侧"不符，已登记 `BLOCKERS.md` B18）、
以及 Java 侧缺一条 `vscode.java.startDebugSession`（拿不到调试端口）。

## 提案

1. **传输与进程归宿主**（不是归 Core）：Core 的契约就是 `ReceiveRequest{data_base64}` 进、
   `DebugSessionUpdate{outbound_frames(base64), events}` 出（`lithe-core/src/debug/types.rs:438/492`），
   所以宿主新增 `DebugTransport`：spawn/连 adapter、双向搬字节、把事件交给 UI。
   形状照抄同仓两个先例，不发明新写法：
   - 进程端口形状 ← LSP 的 `rust/lithe-core/src/lsp/interface/process.rs:35-68`
     （`LspProcessHandle`/`LspProcessStreams`/`LspProcessLauncher` + 可脚本化替身 `scripted.rs:283`）；
   - 阻塞 IO 形状 ← 终端的 `rust/lithe-gpui/crates/terminal/src/session.rs:71/78/81/154/182/502`
     （reader/writer 线程 + mpsc + `cx.spawn` 泵回 UI 线程）。
2. **Java 走 JDTLS 的 `vscode.java.startDebugSession` + loopback TCP**（Zed 没有 Java 适配器，没有可抄的拼装）：
   Core 的 JDT 命令层已有 `buildWorkspace`/`resolveClasspath`/`resolveMainClass`（`lsp/interface/engine.rs:573/7829`），
   **缺这一条 `startDebugSession`**，这是第 1 步的第一个具体改动。
3. **UI 真源分层**：`DebugWorkspaceState`（= Zed 的 `RunningState`）是唯一订阅 Core 事件、
   持"选中线程/栈帧、变量树展开态、面板显隐、活动会话"的实体；面板订阅它，不各自订阅 Core。
   分层理由与 Zed 一致（UI 不反向拥有协议状态）。
4. **面板用 gpui-kit `DockArea` + 一面板一 `Panel`**，**不要**照搬 Zed"外层一个 dock 面板、内部自造 `PaneGroup`"
   那层；布局持久化 key 用稳定字符串、按 adapter 分 key；能力门控（无 `modules`/`loadedSources` 就不显示）。
5. **断点**：数据模型已经齐（`workbench/src/debug.rs:30` 已有 `enabled/verified/condition/hit_condition/log_message`），
   保持"写 store → 反向订阅"的方向（不塞进编辑器事件枚举）；补三件缺的：**落盘**（现在没有）、
   四态图标与右键菜单、以及 gutter 位置（真源是行号列右侧，现在只能放正文左侧，B18）。
6. **命令单一入口**：统一 `Action` + `register_action!`，Toolbar 只 dispatch
   （Zed 是"按钮直调 + Action 渲染器"双通道，别学）。

## 落地顺序

1. `DebugTransport` + 会话生命周期（含 Core 补 `vscode.java.startDebugSession`）；
2. 事件进现有 `DebugPane`（线程/栈/变量懒加载/控制台拉取）；
3. 面板化（DockArea + 持久化 + 能力门控）；
4. gutter 与真源对齐（覆盖层 + 四态 + 右键菜单）。

每步的验收判据见研报 §3.4；实测缺口清单见 §3.3。

## 进展（截至 2026-10-04，第 1 步做了约一半）

已落地（都在 `rust/lithe-gpui/crates/workbench/src/debug_transport.rs`，共 19 条单测）：

| 能力 | 说明 |
| --- | --- |
| `DebugTransport` | 驱动 Core 的 `debug.createSession/launch/execute/receive/disconnect/destroySession`；分帧仍归 Core（`lithe-core/src/debug/protocol.rs`），宿主只搬 base64 字节 |
| `StdioDebugAdapter` | spawn + 读线程 + mpsc、stderr 尾巴、`write_frame`、幂等 `kill`、kill-on-drop、有界 `recv_bounded`/`wait_exit` |
| `TcpDebugAdapter` | `connect_timeout` + 读写分离 + 幂等 `kill`（Java 路径用） |
| `poll_bounded` | 有界读一轮：先等第一块（≤timeout），再排空已到的；终止后的迟到输入被丢弃并计入 `dropped_after_termination` |
| `DebugSessionDriver` | 专用线程跑 `poll_bounded` → 事件经 mpsc 给 UI；收尾统一做 disconnect → destroySession → 杀 adapter；`stop(timeout)` 有界 |
| Java 端口 | **不需要改 Core**：`lsp.request` 的 `ExecuteCommand` 操作接受调用方 `command` 载荷，宿主用它发 `vscode.java.startDebugSession` 并解析端口 |

提交：`2446f78d` → `a1f23351`（更正）→ `eb989a3d` → `bf53a4bb` → `994cac2b` → `2adc7d3d` → `ec71c3c2`。

**还没做**：把驱动器接进工作台（发起入口 + `DebugWorkspaceState`/`DebugPane` 消费事件）、
逐请求超时与取消（Core 已有 `operationID` + `debug.cancelOperation`）、真机 Java 端到端。
`java_debug_session_port` 本身需要活着的 JDTLS 会话，单测不覆盖（契约由载荷用例钉住），留给真机那一步。

**追记（第 15–20 轮，均已落地并有离线用例）**：

| 能力 | 说明 |
| --- | --- |
| `poll_until` | 有界轮询并**全量返回**本轮事件 + 报告谓词是否命中（命令通道要"转发全部事件、只等回执"，不能像 `wait_for_event` 那样吞掉非匹配事件） |
| **驱动命令通道** | `DebugDriverCommand::Execute` + 待决 `operationId` + deadline：期间事件全量转发给 UI，命中回执成功、超时 `cancel(operationId)` 并回执失败；句柄暴露 `execute_command` |
| `DebugPane` 接线 | 持 `Option<DebugSessionHandle>`；`attach_session` / `drain_session`（变化或退出返回真）/ `stop_session`；`Render` 开头 `if drain_session() { cx.notify() }`（靠变化标志避免自旋）；退出时打 `session_exited reason=…` 并清句柄 |
| 控制台接快照 | `copy_new_output(snapshot, seen, target)` 增量搬运（游标记账、快照丢旧行时不重放），面板**已有**的控制台与错误位直接显示调试输出与操作失败 |
| 会话入口 | `start_tcp_session` / `start_java_session`：拿端口 → 连 loopback TCP → `debug.createSession`；UI 流程 = `start_java_session` → `DebugSessionHandle::start` → `attach_session` |
| `debug.launch` 路径 | `launch_with_configuration`；**Core 的 `DebugLaunchConfiguration` 实测形状**是 `{ name, request: "launch"/"attach", arguments: {…适配器参数…}, steppingFilters? }`，适配器参数走 `arguments` 原样透传（第一版按 `{type,name,mainClass}` 传会被拒为 `invalid request`） |

**仍未做**（下一轮起手）：① UI 调用点 —— 工具栏三个按钮接 `execute_command`、"开始调试"菜单项接
`start_java_session`；② 线程/调用栈/变量三块的渲染；③ 真机端到端（本机环境不具备，见下节）。

### "开始调试"入口的现状与接线点（2026-10-04 实测）

- **工具栏已经接上会话**：`DebugPane::press_toolbar_action` 的兜底分支在**有会话**时把
  `DebugControl::command()`（本来就是 Core/DAP 的 camelCase 名：`continue`/`pause`/`next`/`stepIn`/
  `stepOut`/`restart`/`terminate`）交给 `DebugSessionHandle::request_command`（**非阻塞**，UI 线程不等回执）；
  没会话时仍打 `execute control=… status=skipped reason=no-session`。
- **"开始调试"还没接**：`DebugPane::start_session`（`debug.rs:1631`）现在只做两件事 ——
  有 blocker 就打 `start status=blocked reason=…` 并 `cx.notify()`，没有 blocker 就打
  `start status=ready_to_dispatch`（**什么都不启动**）。
- 面板**已经有一套现成的 blocker 模型**可复用：`DebugStartBlocker::{NoTransport, NoSession,
  NoTransportAndNoSession}`，由 `start_blocker(self.transport(), self.session())` 组合得出
  （`debug.rs:1380`），并带三条 i18n 文案 `lithe.debugger.gpui.startBlocked{NoTransport,NoSession,Both}`
  —— 也就是说"起不来时告诉用户为什么"这条早有设计，接线时不要新造一套。
- **接线需要两个面板现在没有的输入**（这是下一轮要确认的第一件事，别猜）：
  ① **当前 Java LSP 会话 id**（`java_debug_session_port` 要它去向 JDTLS 要调试端口）——
     它在 `crates/java` / workbench 层，需要确认由谁传给面板（`attach_session` 之前）；
  ② **`DebugLaunchConfiguration`**（形状已实测：`{ name, request, arguments }`）——
     应由 Core 的 `runConfig.*` 生成而不是宿主手拼，需确认用哪条命令（`runConfig.createLaunchPlan` 起步）。
  拿到这两个输入后，入口就是三步：`start_java_session(...)` → `DebugSessionHandle::start(transport, 20ms)`
  → `DebugPane::attach_session(handle)`，再把 `start_session` 的 `None` 分支接到这三步上。

**输入 ① 的追查结果（2026-10-04 第二轮实测）**：

- Java 的 LSP 会话是 **`crates/java` 自己起的**：`crates/java/src/session.rs:109` 发 `lsp.startServer`，
  从响应里取 `sessionId`（`:116-120`，取不到就报"lsp.startServer 的响应缺少 sessionId"），
  之后所有请求（含 `lsp.request`，`:247`）都带 `self.id`（`:211`）。**也就是说调试端口该由 java crate 去要**，
  它本来就知道该用哪个会话。
- 对外入口是 **`lithe_gpui_java::JavaLanguageService`**（`service.rs:93` `new(workspace_root)` 起实例；
  编辑器侧的补全/悬浮/跳转/诊断都在用它）。但它**没有暴露 LSP 会话 id**（`service.rs` 的 pub 方法里没有
  `session_id()` 之类），所以接线第一步要么加一个 `JavaLanguageService::lsp_session_id()` 访问器，
  要么更干净 —— 直接在 java crate 上加 `start_debug_session()`：由它 `lsp.request` + `executeCommand`
  发 `vscode.java.startDebugSession` 拿端口，宿主再 `start_tcp_session`。**两种做法都要先确认谁持有
  `JavaLanguageService` 实例**（app/workbench 谁创建、是否全局共享），这一步下一轮用一次全仓检索确认。
- 结论：目标里"补 Core 的 JDT 命令 `vscode.java.startDebugSession`"在 Core 侧**不需要改动**
  （`lsp.request` 的 `ExecuteCommand` 已支持调用方载荷，宿主侧 `java_start_debug_session_payload` 已就位）；
  真正待补的是**宿主 java crate 的取端口入口**与它的持有者。

至此目标点名的 6 条 `debug.*`（`createSession` / `launch` / `receive` / `execute` / `disconnect` /
`destroySession`）**全部有调用路径**，宿主离线用例共 35 条（transport 25 + 会话层 10）。

### 真机端到端的前置（2026-10-04 实测，**本机不具备**）

- `third_party/jdtls/` 里**只有 `manifest.json`**，JDTLS 载荷不在位；manifest 指明需要
  JDTLS **1.61.0**（tarball 与 SHA256 都在 manifest 里）与 lombok 1.18.46。
- java-debug 也不在位：需要 `vscjava.vscode-java-debug` **0.59.0** 的 VSIX（open-vsx）解出的
  `com.microsoft.java.debug.plugin-0.53.2`，放进 jdtls 根下的 `java-debug/`
  —— `crates/java/src/jdtls.rs:120` 就是按 `java-debug/` 目录 + `com.microsoft.java.debug.plugin-`
  前缀找的（找不到就报"选中的 JDTLS 安装缺少 … java-debug …"）。
- 本机 **PATH 上的 `java` 是 1.8**（`JAVA_HOME=D:\ProgramData\java\jdk1.8.0_221`），
  但机器上**其实装了更高版本**：`D:\ProgramData\java\openjdk-21`、`openjdk-17`、`open-25`
  （2026-10-04 复核更正；此前只跑 `java -version` 就下"只有 JDK 8"的结论是**错的**）。
  宿主本来就留了覆盖口：`crates/java/src/jdtls.rs:13` 的 `LITHE_JDTLS_JAVA` —— 指到 openjdk-21 的
  `bin\java.exe` 即可满足 `minimumJavaVersion: 21`。
- 剩下的真缺口只有**第三方载荷**：`third_party/jdtls/` 只有 manifest，JDTLS 1.61.0 与
  java-debug 0.59.0（plugin 0.53.2）的包都在网络（URL 与 SHA256 都在 manifest 里），
  仓库里**没有本地物化脚本**（`scripts/verify-download-cache.mjs` 只做 CI 缓存校验）。
  ⇒ 真机 E2E 需要：① 按 manifest 下载并解包 JDTLS + java-debug（约几十 MB）；
  ② 设 `LITHE_JDTLS_JAVA` 指向 openjdk-21；③ 一个真实 Java 工程。
- 结论：**真机「启动→断点命中→单步→变量→停止」在本机无法验证**；等价的代码侧验证是
  scripted adapter 的离线用例（transport 22 条 + 会话层 8 条）。要跑真机，先把上面三件事补齐。

### 这一段实测出来的四条教训（写进这里，避免后人重踩）

1. **DAP 的 `stopped` 只会在 `configurationDone`/`launch` 之后到达**。脚本化 adapter 只应答
   `initialize` 时，等 15 s 也不会来事件；用例要按真实顺序脚本化。
2. **trait 给了默认实现，转发型测试替身最容易漏转发**：`SharedAdapter` 漏了 `try_recv`（默认返回 `None`），
   于是 `poll_bounded` 永远"什么都看不见"，两条用例静默失败。替身的每个方法都要转发。
3. **测试替身不许对中毒锁 `.expect()`**：断言在持有 `MutexGuard` 时失败 → guard 在 unwinding 中 drop →
   锁中毒 → 之后（如 `Drop` 里）再 `.expect()` → **panic 中再 panic → 进程 abort（0xC0000409）**，
   一次普通断言失败会被放大成整条测试进程崩掉。替身一律用 `unwrap_or_else(|p| p.into_inner())`。
4. **`poll_bounded` 这类循环要有"第一块等多久 / 之后还等不等"的明确语义**，否则要么空转要么每次
   白白等满超时；终止之后的输入要**丢弃并计数**（`dropped_after_termination`）而不是继续喂给已结束的会话。

### 已知的下一个缺口：UI 动作到 transport 没有通路（2026-10-04 盘点）

`DebugSessionDriver::spawn` 把 `DebugTransport` **move 进驱动线程**，线程只做 `poll_bounded` → 事件出队。
于是 `DebugSessionHandle` 能读状态（`snapshot()` / `is_running()` / `last_exit()`）却不能发指令 ——
工具栏的**继续 / 单步 / 暂停 / 断开现在没有可调的路径**。这不是环境问题，是接口缺口，接线前必须先补。

打算这么做（下一轮实现）：

1. 驱动器新增**命令通道**：`spawn` 时建一对 `mpsc`，线程每轮除 `poll_bounded` 外再 `try_recv` 命令；
   命令形如 `Execute { command, thread_id, timeout, reply }` / `Stop { reply }`。
2. 宿主侧 API：`DebugSessionHandle::{continue_program, step_over, step_in, step_out, pause, stop}`，
   内部发命令并在**有界时间**内等 `reply`（超时返回错误，不阻塞 UI 线程）。
3. 语义要点：现在 `DebugTransport::execute_and_wait` 会把**不匹配的事件吞掉**（只返回匹配的那一条），
   而 UI 需要看到全部事件 ⇒ 驱动线程里要改成"**全量转发事件** + 单独记录待决操作"：
   发完请求后把 `operationId` 记成 pending，收到 `stopped`/`continued`/`terminated`/`operationFailed`
   时给 reply 回成功，到 deadline 则 `cancel_operation` 并回失败。
   这需要把 `DebugTransport` 里生成 `operationId` 的那步暴露出来（现在 `take_operation_id` 是私有的）。
4. 验收：用 `RespondingAdapter` 跑离线用例 —— `handle.step_over()` 之后必须收到一次响应回执、
   并且 UI 侧仍能看到中途的全部事件；静默 adapter 时该调用必须**有界超时**且触发 `cancel_operation`。

## 考虑过的备选方案

- **直接依赖 Zed 的 `dap` crate**：不行 —— GPL-3.0-or-later，与 Apache-2.0 分发不兼容。
  要现成类型只能用 MIT/Apache 的 `dap-types`（Zed 自己固定的那个 fork rev 许可未确证）。
- **把传输放进 Core**（像 LSP 那样由 Core 起进程）：与现有 DAP 契约冲突（契约明说进程/字节归平台侧），
  也会把平台 spawn 塞进 Core；如果将来要，应走"Core 定 trait + 宿主注入 launcher"的 LSP 式改造，属独立决定。
- **照抄 Zed 的 `DisplayMap`/`BlockMap` 地基来做 gutter**：那套"多 buffer + 折叠 + inlay 融成显示空间"的
  172 KB 地基只为"行号 + 行高"服务，gpui-component 的 `Editor` 里没有对应物，引入即重写编辑器。
- **在编辑器事件枚举里加 `ToggleBreakpoint`**：与 Zed 的做法相反，会让编辑器 crate 反向依赖调试语义。

## 后果

- 收益：Java 调试能从"零"到"能停、能单步、能看变量"，且不新增一套与 Core 重复的协议层。
- 代价/风险：`vscode.java.startDebugSession` 与 java-debug 的实际行为要真机验证；
  gutter 覆盖层与 gpui-component `Editor` 的行高/滚动口径必须一致，否则标记会漂。
- 需要重新评估的触发条件：gpui-kit 若提供 gutter API，第 4 步可改为走上游能力。

## 验收标准
1. **传输层**：scripted adapter 单测（不依赖真进程）覆盖 `initialize → launch → configurationDone → stopped`、
   一个**有界超时**用例、一个**陈旧回包被丢弃**用例；真机对 java-debug 起会话成功。
2. **事件到 UI**：断点命中后能看到线程 / 调用栈 / 变量；变量树按需懒加载；控制台输出走拉模式且有界。
3. **面板**：`DockArea` 下各面板可拆可拖，布局按 adapter 跨重启保持；不支持的能力不显示空面板。
4. **断点**：启用/禁用/条件/日志/命中次数五类状态可编辑、可落盘、重启后仍在；重复下发是幂等的全量替换。
5. **gutter**：断点标记出现在行号列**右侧**（与 `macos/.../CodeEditorView.swift:410-468` 一致），
   同一行只画一个标记，滚动/缩放/换主题不漂。
6. **守卫**：一条脚本列出"Core 有 `debug.*` 命令、宿主没有调用点"的清单（当前 18 条里只用 3 条），
   该清单为空或只剩明确不做的条目。
7. **端到端**：Java 项目上跑通"启动调试 → 断点命中 → 单步 → 查看变量 → 停止"，无残留 adapter 进程。

### 真机环境现状与真机验证进展（2026-10-04 实测）

**载荷已按 manifest 的 SHA256 全部就位**（不进仓库，`.gitignore` 已忽略 `/third_party/jdtls/*`，保留 manifest）：
JDTLS 1.61.0（48.7 MB，`338e7e73…`）、java-debug 插件 0.53.2（`69128beb…`）、
lombok 1.18.46（`01f7b1a0…`）、java-test 0.46.0 VSIX（`56c1e14d…`，含 runner `8ea737ab…`），
铺好 `java-debug/`、`lombok/lombok.jar`、`java-test/extensions.txt` + `extensions/*.jar`。
机器上 JDK：`openjdk-21`（`21.0.8` 实测可跑，配 `LITHE_JDTLS_JAVA` 用）、`open-25`（配 `LITHE_JDTLS_SMOKE_PROJECT_JDK`）。

**已用真实 JDTLS 跑通到哪一步**：`cargo test -p lithe-core --lib real_jdtls`（三个 `LITHE_JDTLS_SMOKE_*`
环境变量门控）现在**不再被跳过**，能起来真实 JDTLS 会话并执行我们的 JDT 命令 ——
实测 `session_id="lsp-session-1"`、`workspace/executeCommand` → `vscode.java.buildWorkspace`
返回结构化 `JavaBuildReport`。也就是说 **Core 的 JDT 命令层 + 我们准备的载荷在真机上端到端可用**。

**卡在哪**：该用例在夹具工程编译处失败（`javaBuildCompilationErrors`，`underlying_message: "2"`，
`elapsed_milliseconds: 9` —— 9 毫秒说明是**工程配置级**失败而不是真的在编译）。用例文档注释说夹具由 `scripts/prepare-jdtls.{sh,ps1}` 准备，但**这个脚本此前在仓库里不存在**
（`glob **/prepare-jdtls*` 无命中）；2026-10-04 已补上 `scripts/prepare-jdtls.ps1`
（不下载、只校验并按 manifest 铺 `java-debug/`、`lombok/lombok.jar`、`java-test/extensions.txt`
+ `extensions/*.jar`，然后打印三个 `LITHE_JDTLS_SMOKE_*` 的值）。⇒ 夹具之外的部分现在可复现；
夹具工程本身的编译失败仍未解决。

**根因（2026-10-04 继续定位，已找到）**：夹具的**主源码编译是好的** —— 把
`workspace/src/main/java` 的 8 个文件直接交给 `open-25` 的 `javac --release 25 --enable-preview`
编，**exit 0**。所以那 `2` 个错误来自**测试源**：`src/test/java/demo/{BaseBehavior,FastTest}.java`
都 `import org.junit.jupiter.api.Test`，而夹具 `pom.xml` 要求
`org.junit.jupiter:junit-jupiter:5.14.0`，本机 `~/.m2/repository` 里**只有 5.10.5**，
且 `mvn dependency:get -Dartifact=org.junit.jupiter:junit-jupiter:5.14.0` **拉不到任何东西**
（版本仍只有 5.10.5）⇒ JDTLS 无法为这两个测试类解析 JUnit，于是报 2 个编译错误。

⇒ 这是**夹具依赖可用性**问题（pom 写了一个本机 Maven 源拿不到的 junit 版本），
与我们的 DAP 代码、与已铺好的 JDTLS/java-debug/lombok 载荷无关。
（此前排除的两个假设——Maven 离线缓存、JDTLS 运行在 JDK 21——也都不是原因。）

**已解决（2026-10-04，真机冒烟变绿）**：夹具 pom 补 `maven-compiler-plugin:3.13.0` + `<release>25</release>`
（Maven 3.6.3 默认的 3.1 处理不了 Java 25 紧凑源文件 `Compact.java`）后，
`real_jdtls_discovers_builds_and_launches_java_25_entrypoints` **通过**（80s，`2 passed; 0 failed`）。

跑通需要这一整套环境（本机验证过的配方）：

```
LITHE_JDTLS_SMOKE_ROOT=<repo>/third_party/jdtls      # 载荷按 manifest 的 SHA256 铺好
LITHE_JDTLS_SMOKE_JAVA=<JDK21>/bin/java.exe          # JDT LS 自身跑在 21 上
LITHE_JDTLS_SMOKE_PROJECT_JDK=<JDK25>                # 夹具工程 target 是 release 25
LITHE_JDTLS_SMOKE_MAVEN=<maven>/bin/mvn.cmd
LITHE_JDTLS_SMOKE_JUNIT=5.10.5                       # 5.14.0 在本机 Maven 源拿不到
JAVA_HOME=<JDK25>                                    # 否则 Maven 用 -source 8 编夹具
MAVEN_OPTS=-Dmaven.repo.local=<工作区内的可写目录>     # 关键：沙箱下 Maven 写不进 ~/.m2 与发行版目录
```

最后一条是最隐蔽的：不指到工作区内的仓库时，Maven 会在
`~/.m2`/发行版目录里因为写 `.part.lock` 失败而报 "could not be resolved"（看起来像网络问题，其实是写权限）。

**仍未做**：DAP 那半边的真机端到端（启动→断点命中→单步→变量→停止）—— 它需要 GUI 驱动（点工具栏/打断点），当前只能靠人工或 UI 自动化；DAP 链路本身已有 43 条离线用例覆盖。

## 风险

- **java-debug 的真实行为未验证**：`vscode.java.startDebugSession` 的返回（端口形态）、
  JDTLS 与 java-debug 版本搭配、attach 模式差异，都要真机实测；Zed 没有 Java 适配器可对照。
- **gutter 覆盖层与编辑器口径不一致会漂**：行高、软换行、折叠、滚动偏移任一处口径不同，标记就会错行；
  需要一条"标记 y 坐标 vs 编辑器行 y 坐标"的判据。
- **逐请求超时缺失会让会话悬挂**（Zed 的既有缺口）：adapter 卡住时必须能取消，
  否则 UI 会永久停在"加载中"。
- **断点落盘与工作区配置的写入时机**：沿用 `.lithe/**` 口径时要防抖，避免每次点击都写盘。
- **范围蔓延**：这一步只做"能调试"，`debugger_ui` 那 8 个面板、REPL、inline values、
  data breakpoints 都要按能力门控分批做，不要一次全铺。

## 验证

- 调研阶段：三路只读调研（编辑器 gutter、DAP 客户端、调试 UI）结论均带 `zed/...:行号` 证据；
  Lithe 现状为实测（`debug.rs`、`engine.rs`、`types.rs` 的命令与字段清单）。
- 实现阶段（待做）：scripted adapter 单测不依赖真进程；一个有界超时 + 陈旧回包丢弃用例；
  真机 Java 端到端"启动 → 断点命中 → 单步 → 查看变量 → 停止"；
  `debug.*` 命令接线守卫（列出 Core 有、宿主未接的命令）。

## 适用范围

- `rust/lithe-gpui/research/zed-editor-breakpoints-debug.md`（调研真源）
- `rust/lithe-core/src/debug/**`（reducer 与契约）、`rust/lithe-core/src/lsp/interface/engine.rs`（JDT 命令层）
- `rust/lithe-gpui/crates/workbench/src/debug.rs`、`crates/editor/src/editor_view.rs`（断点列）、
  `crates/terminal/src/session.rs`（IO 形状先例）、`crates/java/src/{jdtls,session}.rs`（java-debug 路径）

### DAP 真机端到端的程序化通路（2026-10-04 实测，已找到路）

不需要点界面：app 自带 `--run-probe-debug`（`main.rs` 的 `run_probe_debug` → `ShellWorkspace`），
会自己挑第一个生成的运行配置并以 debug 启动。三次实跑各暴露一个可修项：

1. 只传项目目录 ⇒ `S1_JAVA_FAILED 未找到 jdtls`：宿主按 `EXECUTABLE_NAMES`（`jdtls.bat` 等）找，
   把 `<payload>/bin` 加进 `PATH` 即可（载荷里有 `bin/jdtls.bat`）。
2. 把 `.java` 文件当位置参数 ⇒ 宿主把它当成工作区根，`Workspace root must be a directory`（整条链路失败）。
3. 项目目录用**相对路径** ⇒ `无法把工作区路径转成 file URI`（java crate 要绝对路径）。

仍未解决的一步：探针挑中的配置是 **current-file 作用域**，而探针不打开任何缓冲区，
于是 `S1_RUN_PLAN result=failed error=Current File requires a Java source path`。
下一轮做法：用**绝对**项目路径重跑，让 JDTLS 先导入工程、Core 生成 main-class 配置（而不仅是 current-file），
再让探针启动 —— 之后就能在 `S1_DEBUG start/launch/session_state/inspect_requested` 锚点上验证
「启动→断点命中→单步→变量→停止」的真机链路。
（本轮三次运行的 app 进程与 JDTLS 子进程都已清理，`leftover Lithe=0`。）

第 4 次实跑（绝对路径 + PATH 里有 jdtls）的新卡点：
`S1_JAVA_FAILED reason=process_start_failed: Could not create the Java language-server configuration area.`
—— JDT LS 起来了但**建不了它自己的配置区**（要把 `config_win` 拷进 cache/configuration 目录），
与 Maven 那次 `.part.lock` 写失败同源：**沙箱不让子进程写工作区外的目录**。
⇒ 下一步：把 JDT LS 的 configuration/data/cache 指到工作区内（看 `crates/java` 的 `cache_directory` 从哪来、
有没有环境变量口子），或让运行环境放开工作区外的写权限。之后才轮得到 `Current File requires a Java source path`
那一步（探针挑中 current-file 配置而它不打开缓冲区）。

### 第 4 次实跑的路径级根因（2026-10-04，证据完整）

日志给出了确切路径：

```
S1_JAVA_CACHE directory=C:\Users\admin\AppData\Local\Lithe\cache\language-servers ...
S1_SETTINGS watch_failed dir=C:\Users\admin\AppData\Roaming\Lithe error=拒绝访问。 (os error 5)
S1_THEME seeded dir=C:\Users\admin\AppData\Roaming\Lithe\themes ... failed=1
S1_EXTENSION sync_failed dir=C:\Users\admin\AppData\Roaming\Lithe\extensions\... error=拒绝访问。 (os error 5)
```

⇒ JDT LS 的 **data/configuration 目录落在 `%LOCALAPPDATA%\Lithe\cache\language-servers`（工作区外）**，
而当前沙箱**拒绝一切对 `%APPDATA%`/`%LOCALAPPDATA%` 的写入**（同一批日志里 themes/settings/extensions 全部
`拒绝访问 os error 5`）⇒ `Could not create the Java language-server configuration area`。
这与 Maven 的 `.part.lock` 写失败、`extension` 同步失败是**同一个根因**：子进程只能写工作区内路径。

⇒ 两条出路，二选一：
① 给宿主加"状态目录可否覆盖"的口子（例如 `LITHE_CONFIG_HOME` / `LITHE_CACHE_HOME`，或便携模式让
   settings/cache/extensions 都落到 `<workspace>/.lithe/...`）—— 这本身也是个真实需求（便携/沙箱/CI 场景）；
② 在沙箱外（或放开 `%APPDATA%`/`%LOCALAPPDATA%` 写权限）跑那一次真机端到端。
在此之前，DAP 真机实跑无法继续（`S1_JAVA_FAILED` 之后整条语言服务链路都不成立）。

### 第 5/6 次实跑的突破与最后一步（2026-10-04）

**沙箱写权限卡点已清除**：宿主本来就有缓存目录覆盖口 `LITHE_GPUI_CACHE_DIR`
（`crates/java/src/workspace.rs:10` 的 `CACHE_DIR_ENV`，`cache_root()` 优先读它）。
把它指到工作区内（`.artifacts/state/cache`）后：

```
S1_JAVA_CACHE directory=...\.artifacts\state\cache\language-servers ...
S1_JAVA_SESSION started sessionId=lsp-session-1 processId=16072      ← 真实 JDT LS 在 app 里起来了
S1_RUN_CONFIGS configs=1 selected=0
S1_RUN_PROBE launching=first-configuration debug=true
S1_RUN_PLAN result=failed error=Current File requires a Java source path
```

⇒ 现在只剩**一步**：探针挑中的唯一配置是 **current-file 作用域**，而它不打开任何文件。
`--session-active <App.java>` 试过：app 立刻 exit 2（该旗标期望的不是文件路径），需要换路子。
可选做法：① 搞清 `--session-active` 的正确取值（`main.rs:219`）让它激活该文件；
② 让 Core 的 `runConfig` 为这个 Maven 工程生成 **main-class 配置**（而不只是 current-file），
   探针自然会挑到它；③ 让探针优先挑非 current-file 配置。

跑通这一条后，`S1_DEBUG start/launch/session_state/inspect_requested` 就是 DAP 真机链路的判据。

### 第 7/8 次实跑：文件能打开并激活，但探针不喂"当前文件"（2026-10-04 收口）

```
S1_JAVA_SESSION started sessionId=lsp-session-1 processId=2704     ← JDT LS 正常
S1_SESSION_PROBE opened=1 active=Some(0)                           ← --session-probe <file> + --session-active 0 生效
S1_RUN_PROBE launching=first-configuration debug=true
S1_RUN_PLAN result=failed error=Current File requires a Java source path   ← 仍失败
```

⇒ 剩下的**不是环境问题，是一处代码缺口**：探针（`--run-probe-debug` → `ShellWorkspace`）发起
`runConfig.createLaunchPlan` 时没有把"当前文件"作为 `currentFile` 传进去（配置 id 又是 `current-file` 作用域），
所以 Core 报 `Current File requires a Java source path`。两条修法：

① **让探针/工作台把活动文件的 workspace 相对路径当 `currentFile` 传**（最小改动，且对真实用户路径也正确 ——
   用户点"运行/调试"时本来就该用当前打开的文件）；
② 让 Core 的 `runConfig` 生成器为 Maven 工程产出 **main-class 配置**（探针自然会挑到它）。

做完任一条，第 7/8 次实跑的日志就会继续往后走：`S1_RUN_PLAN result=ok` → `S1_DEBUG start status=ok`
→ `launch status=sent jdwp=…` → `session_state=…` → `inspect_requested kind=threads|stackTrace|scopes`，
那就是目标最后一项的真机证据。当前环境配方（载荷 + PATH + `LITHE_JDTLS_JAVA` + `LITHE_GPUI_CACHE_DIR`
+ `MAVEN_OPTS` + `--session-probe`/`--session-active`）已经完整，不再有环境未知数。
