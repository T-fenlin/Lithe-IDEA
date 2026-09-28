# Lithe Windows 前端 → Rust + gpui-kit 0.6.6 界面规格：终端 / 运行 / 调试 / 问题 / 输出

> 本文由只读逆向调研生成。界面规格唯一来源为 **Windows 前端** `windows/tauri/src/`（Tauri v2 + React 19 + TypeScript + Tailwind v4（CSS-first）+ shadcn/Base UI）。
> 调研范围：终端（terminal）、运行（run / run-actions）、调试（debugger / DAP）、问题（diagnostics）、输出面板（run output / adapter console）。
> 未读取、未参考 `macos/`；未修改任何已有文件；未运行 bun/npm/cargo 构建或测试；未启停任何进程。
> 每条结论给出 `相对路径:行号` 证据；无法确证处明确写「未找到」，不做推测。

---

## 0. 换算与来源约定

| 约定 | 内容 | 证据 |
| --- | --- | --- |
| 1 Tailwind 单位 | = 0.25rem = **4px**（根字号 16px 时） | `windows/tauri/src/styles/theme.css:220`（`html { font-size: calc(16px * var(--app-ui-scale)) }`） |
| 字号工具类 | `.ui-text-chrome` = 13px、`.ui-text-sm` = `--app-ui-font-size` = 13px、`.ui-text-caption` = 12px | `windows/tauri/src/styles/theme.css:112-117`、`windows/tauri/src/styles/utilities.css:30-44` |
| 舒适密度覆盖 | `:root[data-window-chrome-density="comfortable"]` 把 `--ui-text-chrome` 提到 14px、`--lithe-tab-height` 提到 2rem 等 | `windows/tauri/src/styles/theme.css:188-200` |
| 面板尺寸令牌 | `--lithe-pane-header-height: 2.25rem(36px)`、`--lithe-tab-bar-height = var(--lithe-pane-header-height)`、`--lithe-tab-height: 1.75rem(28px)`、`--lithe-tab-max-width: 12.5rem(200px)`、`--lithe-workbench-gap: 4px`、`--lithe-chrome-control-height: 1.5rem(24px)`、`--lithe-chrome-gap: 4px`、`--lithe-chrome-gap-loose: 6px`、`--lithe-chrome-padding-inline: 8px`、`--lithe-chrome-radius: 4px`、`--lithe-footer-height: 1.5rem(24px)` | `windows/tauri/src/styles/theme.css:118-133` |
| 颜色来源 | 主题文件 `colors.<key>` → CSS 变量 `--<key>`；终端 ANSI 16 色为 `--terminal-black … --terminal-bright-white` | `windows/tauri/src/styles/theme.css:100`（`--color-terminal-bright-white: var(--terminal-bright-white)`）等；`windows/tauri/src/features/run/utils/run-output-style.ts:44-65` |
| 滚动条尺寸 | `--app-scrollbar-size: 11px` | `windows/tauri/src/styles/scrollbars.css:2` |

---

## 1. 区域清单

### 1.1 底部工具窗（BottomPane）容器层级

```
BottomPane                                windows/tauri/src/features/layout/components/bottom-pane/bottom-pane.tsx:28
├─ resizeGutter（拖动改高热区，高 4px）    bottom-pane.tsx:222-241
├─ paneContent
│  └─ div.lithe-glass-island              bottom-pane.tsx:244-254
│     └─ div.h-full.overflow-hidden       bottom-pane.tsx:255
│        ├─ TerminalContainer（始终挂载，仅切换 block/hidden） bottom-pane.tsx:257-264
│        │  └─ terminal-container.tsx:36
│        ├─ DebuggerView（仅 activeTab==="debugger"）  bottom-pane.tsx:266-272
│        ├─ RunPane（activeTab==="run"）              bottom-pane.tsx:274-278
│        ├─ MavenRunPane（activeTab==="maven"）        bottom-pane.tsx:280-284
│        ├─ DiagnosticsBuffer（activeTab==="diagnostics"） bottom-pane.tsx:286-295
│        ├─ BottomBufferPane（activeTab==="buffers"）  bottom-pane.tsx:297-301
│        └─ GitLogToolWindow（activeTab==="gitLog"）    bottom-pane.tsx:303-307
└─ isFullScreen → WorkbenchFullscreenSurface 包裹       bottom-pane.tsx:312-316
```

工具窗页签的类型联合：`BottomPaneTab = "terminal" | "debugger" | "diagnostics" | "references" | "buffers" | "run" | "maven" | "gitLog"`（`windows/tauri/src/features/window/stores/ui-state/types/ui-state.types.ts:17-26`），默认值 `"terminal"`（`windows/tauri/src/features/window/stores/ui-state/panel-slice.ts:31`）。

**重要：底部工具窗本身没有横向页签行。** 页签选择器是左侧活动栏里的**竖向图标列表**（`SidebarPaneSelector orientation="vertical"`，`windows/tauri/src/features/layout/components/sidebar/main-sidebar.tsx:632-656`），每个图标项通过 `toggleTerminalPane / toggleDiagnosticsPane / toggleRunPane...` 控制 `isBottomPaneVisible` + `bottomPaneActiveTab`（`main-sidebar.tsx:643-652`）。

### 1.2 终端区域层级

```
TerminalContainer                          windows/tauri/src/features/terminal/components/terminal-container.tsx:36
├─ div.terminal-container                  terminal-container.tsx:697-701
│  └─ div（isVertical ? flex-row : flex-col）  terminal-container.tsx:702
│     ├─ TerminalTabBar orientation={tabLayout}（vertical 且 position==="left" 或 horizontal） terminal-container.tsx:703-705
│     ├─ div.flex-1（会话区）               terminal-container.tsx:707-717
│     │  └─ terminalSessions               terminal-container.tsx:624-692
│     │     └─ TerminalSession（每个页签一个；split 时左右/上下各一） terminal-container.tsx:650-659 / 673-682
│     │        └─ terminal-session.tsx:19
│     │           └─ TerminalErrorBoundary → TerminalSlot   terminal-session.tsx:82-94
│     │              └─ terminal-slot.tsx:12（渲染 data-terminal-slot 空 div 作为 portal 目标）
│     └─ TerminalTabBar（vertical 且 position==="right"） terminal-container.tsx:719-721
TerminalHost（挂到 App 根，portal 所有 xterm 实例）  windows/tauri/src/features/terminal/components/terminal-host.tsx:14
└─ XtermPortal × N                          terminal-host.tsx:64
   └─ createPortal(<XtermTerminal/>) → wrapper div reparent 到 slot  terminal-host.tsx:84-108、133-147
XtermTerminal                               windows/tauri/src/features/terminal/components/terminal.tsx:75
├─ TerminalSearch（绝对定位右上）            terminal.tsx:861-869 → terminal-search.tsx:22
├─ ContextMenu（复制/粘贴右键菜单）          terminal.tsx:871-916
└─ div.xterm-container（xterm.js 挂载点）    terminal.tsx:877-888
```

### 1.3 运行区域层级

```
RunPane                                    windows/tauri/src/features/run/components/run-pane.tsx:68
├─ ProjectPreparationStatus                run-pane.tsx:193
├─ JavaDiscoveryNotice（条件渲染）          run-pane.tsx:42-66 / 194
├─ 头部工具行（h-(--lithe-pane-header-height)=36px） run-pane.tsx:195-259
│  ├─ RunIcon + "运行 {projectName}"         run-pane.tsx:196-199
│  ├─ Spinner（isLoading 时）                run-pane.tsx:200
│  ├─ 运行状态文字（run.running / succeeded / failed） run-pane.tsx:201-207
│  ├─ 运行/停止按钮（PlayIcon/StopIcon）      run-pane.tsx:208-212
│  ├─ RunServicesMenu                       run-pane.tsx:213-220 → run-services-menu.tsx
│  ├─ 重新扫描按钮                           run-pane.tsx:221-231
│  ├─ 滚动到底开关                           run-pane.tsx:232-243
│  ├─ 清除输出按钮                           run-pane.tsx:244-248
│  └─ 最小化按钮                             run-pane.tsx:249-258
├─ 阻断/过期诊断横幅（warning 色）           run-pane.tsx:261-289
├─ JavaLaunchDecisionBanner（条件）          run-pane.tsx:291-303 → java-launch-decision.tsx
└─ RunConfigurationListSplit（status==="ready" 时） run-pane.tsx:322-431
   ├─ list：分组列表（服务/基础设施/应用/任务） run-pane.tsx:323-391、472-534
   └─ content：配置详情 + RunOutputText + RunStdinInput + 生成提示 run-pane.tsx:392-430
```

运行操作面板（命令面板式）：

```
RunActionsButton                            windows/tauri/src/features/run-actions/components/run-actions-button.tsx
└─ RunActionDialog → RunActionRow           run-actions/components/run-action-dialog.tsx、run-action-row.tsx
```

### 1.4 调试区域层级

```
DebuggerView                                windows/tauri/src/features/debugger/components/debugger-view.tsx
├─ 头部工具行（h-10 = 40px）                debugger-view.tsx:362-440
│  ├─ Bug 图标 + "运行和调试"               debugger-view.tsx:363-366
│  ├─ 开始 / 继续或暂停 / 停止 按钮组        debugger-view.tsx:367-397
│  ├─ 分隔符                                debugger-view.tsx:398
│  ├─ 单步跳过 / 单步进入 / 单步跳出         debugger-view.tsx:399-428
│  ├─ DebugStatusBadge（状态徽标）          debugger-view.tsx:430
│  └─ 切换当前行断点按钮                    debugger-view.tsx:431-439
└─ grid（左 260~320px 配置栏 + 右内容）      debugger-view.tsx:442
   ├─ aside：配置选择 Select + 命令输入      debugger-view.tsx:443-459
   └─ 右侧：页签（线程和变量 / 控制台）      debugger-view.tsx:575-600
      └─ grid grid-cols-2 gap-2 p-2         debugger-view.tsx:601
         ├─ DebugSection 调用栈              debugger-view.tsx:602-608
         ├─ DebugSection 线程                debugger-view.tsx:610-624
         ├─ DebugSection 变量                debugger-view.tsx:626-634
         ├─ DebugSection 监视                debugger-view.tsx:636-643
         ├─ DebugSection 控制台（col-span-2）debugger-view.tsx:645-680
         └─ DebugSection 断点                debugger-view.tsx:682-714
```

### 1.5 问题（diagnostics）区域层级

```
DiagnosticsBuffer                           windows/tauri/src/features/diagnostics/components/diagnostics-buffer.tsx:14
└─ DiagnosticsPane isEmbedded={true}        diagnostics-buffer.tsx:55-64
   └─ diagnostics-pane.tsx:212
      ├─ paneHeaderClassName()（min-h-7 = 28px）diagnostics-pane.tsx:892-898 → pane-chrome.tsx:6
      │  ├─ 问题计数文字（problemSummary）   diagnostics-pane.tsx:900
      │  ├─ 文件导航开关 / 搜索 / 筛选(带 Badge) / 全屏 / 关闭  diagnostics-pane.tsx:902-980
      │  └─ SearchPopover（绝对定位）        diagnostics-pane.tsx:982-1028
      └─ div.flex.flex-1.overflow-hidden     diagnostics-pane.tsx:1032
         ├─ FileNavigatorSidebar（左）        diagnostics-pane.tsx:1034-1046
         └─ ScrollArea contentClassName="px-1.5 py-1.5"  diagnostics-pane.tsx:1049
            └─ space-y-1.5 → section（分组卡片）diagnostics-pane.tsx:1062-1178
               ├─ 分组头 Button（h-auto px-2 py-1）diagnostics-pane.tsx:1073-1096
               └─ 行 div（px-2 py-1.5）        diagnostics-pane.tsx:1122-1172
```

非嵌入模式（`isEmbedded=false`）时外层是 `div.flex.h-44.flex-col.border-t`（**h-44 = 176px**，`diagnostics-pane.tsx:1212`）。

### 1.6 输出（output）的两个不同实现

| 输出面 | 实现 | 证据 |
| --- | --- | --- |
| **运行进程输出** | `RunOutputText`：自研 ANSI SGR 解析 → `<span>` 序列，普通 DOM，无 xterm | `windows/tauri/src/features/run/components/run-output-text.tsx:33-53`、`windows/tauri/src/features/run/utils/run-output-style.ts:92-199` |
| **调试适配器输出** | 纯文本行列表（`whitespace-pre-wrap font-mono ui-text-sm`，stderr 用 `text-destructive`） | `windows/tauri/src/features/debugger/components/debugger-view.tsx:666-678` |
| **终端输出** | xterm.js Canvas/WebGL 渲染 | `windows/tauri/src/features/terminal/components/terminal.tsx:267-289` |

---

## 2. 进程 / 通道结构（终端进程在哪、数据怎么流）

### 2.1 结论速查

| 问题 | 答案 | 证据 |
| --- | --- | --- |
| 终端进程谁 spawn？ | **Rust 侧（Tauri host）**，用 `portable-pty` 0.9 打开 PTY 并 `spawn_command` | `windows/tauri/crates/terminal/Cargo.toml:11`（`portable-pty = "0.9"`）、`windows/tauri/crates/terminal/src/connection.rs:36-49` |
| Windows 上的 PTY 后端 | portable-pty → **ConPTY**（前端另有 `windowsPty.backend: "conpty"` 提示给 xterm.js） | `windows/tauri/src/features/terminal/utils/terminal-options.ts:44-47` |
| 前端有终端模拟器吗？ | 有：**xterm.js 6**（只做渲染/输入编码，不 spawn 进程） | `windows/tauri/package.json`（`"@xterm/xterm": "^6.0.0"`）、`windows/tauri/src/features/terminal/components/terminal.tsx:3、267` |
| 数据通道（输出） | **Tauri v2 `ipc::Channel<TerminalEvent>`**（`create_terminal` 的 `onEvent` 参数），不是 `emit`/`listen` | `windows/tauri/src-tauri/src/terminal.rs:9、108、116`；前端 `windows/tauri/src/features/terminal/utils/terminal-protocol.ts:28` |
| 数据通道（输入/尺寸/暂停/关闭） | 普通 Tauri command：`terminal_write` / `terminal_resize` / `terminal_set_paused` / `close_terminal` | `windows/tauri/src-tauri/src/terminal.rs:131-174` |
| 输出字节如何进 xterm | Channel 回调 → `Uint8Array` → `terminal.write(bytes)`（**二进制直传，不经 base64**） | `windows/tauri/src/features/terminal/hooks/use-terminal-connection.ts:158-187` |
| 运行进程在哪 spawn？ | Rust 侧 `std::process::Command`（`run.rs`），输出经 **Tauri 事件 `run-output` / `run-exit`**（`app.emit_to`）回流 | `windows/tauri/src-tauri/src/run.rs:1959-1963、2008-2012`；前端 `windows/tauri/src/features/run/hooks/use-run-process-events.ts:21-33` |
| 调试适配器在哪 spawn？ | Rust 侧 `std::process::Command`（`debug.rs`），stdin/stdout 走 DAP `Content-Length` 分帧；事件 `debugger_message` / `debugger_output` / `debugger_session_ended` | `windows/tauri/src-tauri/src/debug.rs:1-13、189-200、948-991`；前端 `windows/tauri/src/features/debugger/services/debug-adapter-service.ts:147-157` |

### 2.2 终端数据流（逐跳）

```
xterm.js onData/onBinary  terminal.tsx:290-321 / use-terminal-connection.ts:145-146
  → useTerminalWriteBuffer 批量合并  use-terminal-write-buffer.ts
  → invoke("terminal_write", { id, input: { kind:"text"|"binary", data } })
      src-tauri/src/terminal.rs:131-140
  → TerminalManager::write_to_terminal → TerminalConnection::write → PTY writer
      crates/terminal/src/manager.rs:52-59 / connection.rs:539-548

PTY master reader 线程（64KB 缓冲，支持 pause/resume 背压）
  crates/terminal/src/connection.rs:453-512
  → TerminalEventHandler = Arc<dyn Fn(&str, TerminalEvent) -> bool>
      crates/terminal/src/protocol.rs:70
  → Tauri 侧闭包 → on_event.send(event)  src-tauri/src/terminal.rs:116
  → Channel<TerminalEvent> 传到前端  windows/tauri/src/features/terminal/utils/terminal-protocol.ts:28-45
  → subscribeToTerminalEvents → terminal.write(bytes)  use-terminal-connection.ts:158-187
```

`TerminalEvent` 线格式（`serde(tag = "event", rename_all = "camelCase", rename_all_fields = "camelCase")`，`crates/terminal/src/protocol.rs:50-68`）：

| 事件 | 字段 | 前端处理 | 证据 |
| --- | --- | --- | --- |
| `output` | `data: number[]`（byte 数组） | 累积高水位 → 写 xterm；OCR 流解析标题/目录 | `use-terminal-connection.ts:159-189` |
| `error` | `message: string` | 终端内写红色 `Error: …` | `use-terminal-connection.ts:191-194` |
| `exit` | `exitCode?: number`、`signal?: string` | 缓存，等 `closed` 后判定 | `use-terminal-connection.ts:196-199` |
| `closed` | — | `close_terminal` + 释放 channel；exitCode===0 时自动关页签，否则打印黄色提示 | `use-terminal-connection.ts:201-220` |

`TerminalSize` / `TerminalInput` 也是 camelCase（`protocol.rs:4-48`），并有单测锁定线格式（`protocol.rs:110-156`）。

### 2.3 会话归属与清理

- `FrontendTerminalSessions`（Rust 侧 `HashMap<window_label, session>`）保证同一 WebView 窗口换前端会话时，旧 PTY 被关闭（`windows/tauri/src-tauri/src/terminal.rs:22-98`）。
- 前端在启动时调用一次 `begin_frontend_terminal_session`（`windows/tauri/src/features/terminal/utils/frontend-terminal-session.ts:22-27`）。
- **PTY 进程的生命周期归 buffer store**：xterm UI 卸载不杀进程（注释见 `windows/tauri/src/features/terminal/components/terminal.tsx:587-590`）；真正关页签时才 `close_terminal`（`windows/tauri/src/features/terminal/components/terminal-container.tsx:71-91`）。
- `TerminalHost` 用 portal + `appendChild` 在 pane 之间搬运同一个 xterm DOM，避免重挂载丢 scrollback（`windows/tauri/src/features/terminal/components/terminal-host.tsx:9-13、84-108`）；无 slot 时停放到屏幕外 `[data-terminal-park]`（`terminal-host.tsx:92-107`）。
- 窗口销毁时 `close_all` 杀全部 PTY（`crates/terminal/src/manager.rs:99-113`）。

### 2.4 运行进程流

```
RunPane → run.store.actions.runConfiguration
  → Core: runConfig.createLaunchPlan（lithe-core，见 §4）
  → invoke("run_resolve_launch") → ResolvedLaunch（可执行文件/工作目录/环境）
      src-tauri/src/run.rs:328-420
  → invoke("run_execute_prelaunch")（可选：javac 预编译）
      src-tauri/src/run.rs:559-585
  → invoke("run_start_process", { windowLabel, sessionId, executionId, executable, arguments, workingDirectory, environment })
      src-tauri/src/run.rs:587-655
  → 后端 stdout/stderr 读取线程（按 Windows ANSI code page 解码，处理半个多字节尾部）
      src-tauri/src/run.rs:1864-1903 / 1740-1820
  → 输出分发线程：100ms 或 1MiB 批量，emit_to(window, "run-output", { sessionId, chunk })
      src-tauri/src/run.rs:25-27、1932-1965
  → 退出线程：等 stdout/stderr/dispatcher 全部 join 后 emit_to("run-exit", { sessionId, exitCode })
      src-tauri/src/run.rs:1968-2013
  → 前端 listen("run-output"/"run-exit") → run.store.appendOutput / finishProcess
      windows/tauri/src/features/run/hooks/use-run-process-events.ts:18-34
```

停止：`invoke("run_stop_process", { windowLabel, sessionId, executionId })`（`windows/tauri/src-tauri/src/run.rs:684-693`）→ `stop_session`（`:2030-2045`）：在**同一把锁内**用 `take_owned_session`（`:2016-2028`）校验并移除会话，再执行 `taskkill /F /T /PID <pid>`（`:2040-2043`）。
stdin：`invoke("run_write_stdin")`（`run.rs:694-716`）。

**没有 operationID 概念**：`grep operation|Operation` 在 `run.rs` 全文件零命中；用 `execution_id` 做归属校验（`:2021-2026` 注释明确「旧适配器不得回收新 Run」）。前端侧 `run.store.ts` 维护 `executions: Map<sessionId, executionId>`（`run.store.ts:375`）并在 `stop()` 里校验（`:1039-1067`），`isCurrent()`（`:688`）。

其它运行期常量与行为：
- 输出保留上限 `MAXIMUM_OUTPUT_CHARACTERS = 500_000`，超出时按换行裁剪并用 `advancePastIncompleteControl` 跳过被切断的 ANSI 序列（`run.store.ts:71、274-276`；`run/utils/output-timestamper.ts:222-248`）。
- 输出自动加时间戳 `HH:mm:ss.SSS `（`output-timestamper.ts:3-9`），跨 chunk 保留可能被截断的 ANSI/时间戳前缀（`:157-200`），`\r` 覆盖行按「较长的部分胜出」合并（`:128-135`），已带时间戳的行不重复加（`LEADING_TIME_PATTERN`，`:1、:11-24`）。
- Windows 命令行过长时用 `%TEMP%\lithe-run\launch-{pid}-{counter}.argfile` 规避，按当前 ANSI 代码页编码并做往返校验（拒绝有损映射），`Drop` 时删除（`windows/tauri/src-tauri/src/run/launch_arguments.rs:9-19、59-66、74-103、129-189`）。
- `.cmd` / `.bat` 走 `cmd.exe /D /S /C`（`run.rs:1652-1671`），参数由 `quote_windows_arg` 转义（`:1683-1713`）；`CREATE_NO_WINDOW = 0x0800_0000`（`:24、:1715-1722`）。

### 2.5 调试（DAP）流

- 传输层：**子进程 stdin/stdout** 或 **loopback TCP**（`windows/tauri/src-tauri/src/debug.rs:1-6、189-200、445-471`）。
- 分帧与状态机在 `lithe-core` 的 `debug.*` 命令里；Windows host 只负责进程/套接字生命周期（`debug.rs:1-13`）。host **不解析 `Content-Length` 帧**，只做 base64 解码 + `write_all`（`debug.rs:921-946`），单帧上限 `MAX_FRAME_BYTES = 64 * 1024 * 1024`（`debug.rs:33、936`）。
- 请求：`invoke("debug_send_request", { sessionId, command, arguments, operationId })` → `translate_request`（`debug.rs:687-779`）映射到 Core 命令 → 返回 base64 `outboundFrames` → 写适配器 stdin。`operationId` 缺省为 `debug-op-{counter}`（`debug.rs:545-550`）。
- 接收：适配器 stdout 分片（4096 字节）→ base64 → Core `debug.receive` → `{ events, outboundFrames }`（`debug.rs:994-1086、1025-1048`）。
- 事件投影：Core 归一化事件 → `debugger_message`；适配器 stderr 原始字节 → `debugger_output`；进程退出 → `debugger_session_ended`（`debug.rs:11-13、948-991、1129-1132`）。退出原因取值：`exited`（含 exitCode，`debug.rs:1157-1164`）、`stopped`（`:661-664`）、`failed`（`:1205-1208`）。
- 启动门（startup gate）：前端调用 `debug_session_ready` 之前，最多缓存 `MAX_BUFFERED_STARTUP_EVENTS = 128` 条启动事件，防止 React 侧漏事件（`debug.rs:34、103-108、492-518、961-991`）。
- 工作区清理：`debug_stop_workspace_sessions` 只关掉属于该 workspace 的适配器（路径归一化比较，`debug.rs:594-609、859-886`）；应用退出时 `shutdown()` 清空全部（`debug.rs:670-685`，由 `main.rs:196-197` 调用）。
- 过期保护：`is_current_session`（`debug.rs:1184-1190`）与按 pid 匹配的 `remove_adapter_session`（`:1174-1182`）；kill 用 `taskkill /F /T /PID`（`:1228-1236`）。

### 2.6 `fake-dap-adapter` 是什么

**测试专用，不进产品。** `windows/tauri/src-tauri/Cargo.toml:12-15`：

```toml
[[bin]]
name = "fake-dap-adapter"
path = "src/bin/fake_dap_adapter.rs"
required-features = ["test-support"]
```

源码首行注释即定位：`//! Deterministic DAP adapter fixture used by Windows process lifecycle tests.`（`windows/tauri/src-tauri/src/bin/fake_dap_adapter.rs:1`）。它实现最小 DAP 握手（`initialize` / `initialized` / `launch` / `stopped` / `terminated`），按命令行第一参数切换故障模式：`normal`、`eof-live`、`reject-initialize`、`reject-launch`、`fragmented`（分片写帧，验证半帧解析）、`hold`、`startup-stopped`（`fake_dap_adapter.rs:45-127`）。它用来验证 Rust 侧 DAP 分帧、启动门、失败路径，不参与发行包。

---

## 3. 度量表

> 换算前提：1 Tailwind 单位 = 4px；`--app-ui-scale: 1`（`windows/tauri/src/styles/theme.css:113`）。
> `--lithe-*` 变量在 run / debugger / diagnostics 三个面板的组件树里**几乎不用**：全树只有 `run-pane.tsx:195` 一处 `h-(--lithe-pane-header-height)`；调试面板头栏直接写死 `h-10`（40px，`debugger-view.tsx:362`），而运行面板头栏用 36px 变量 —— 两个面板头栏高度**不一致**，重写时需注意。

### 3.1 底部工具窗容器

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 工具窗外框高（默认） | `height: calc(320px + var(--lithe-workbench-gap))`，state 初值 `useState(320)` | 320 + 4 = **324px** | `bottom-pane.tsx:47、326-328` |
| 工具窗最小/最大高 | `Math.min(Math.max(startHeight + deltaY, 200), window.innerHeight * 0.8)` | 最小 **200px** | `bottom-pane.tsx:122` |
| 拖动改高热区 | `h-(--lithe-workbench-gap)` = 4px | **4px** | `bottom-pane.tsx:226` |
| 外框圆角/边框 | `rounded-xl border-border/70 border-t border-l` | 圆角 12px | `bottom-pane.tsx:247` |
| 内容包裹 | `h-full overflow-hidden` | — | `bottom-pane.tsx:255` |
| 全屏时 | `size-full rounded-none border-0 shadow-none ring-0` | — | `bottom-pane.tsx:249` |
| 内拖放高亮 | `ring-2 ring-primary ring-inset` | 2px | `bottom-pane.tsx:248` |

### 3.2 终端容器与 xterm

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 终端根容器 | `terminal-container flex h-full flex-col overflow-hidden` | — | `terminal-container.tsx:699` |
| 会话区外框 | `bg-background`；vertical 时 `rounded-tl-lg / rounded-tr-lg border-border/60 border-t border-l|r` | 圆角 8px | `terminal-container.tsx:707-714` |
| xterm 宿主 | `.xterm-container { position:relative; overflow:hidden; display:flex; flex-direction:column; background: var(--background) }` | — | `windows/tauri/src/features/terminal/styles/terminal.css:2-9` |
| xterm 本体 | `flex:1; width:100%; height:100%; font-variant-ligatures:none; font-feature-settings:"liga" 0,"calt" 0` | — | `terminal.css:12-24` |
| **终端内容左内边距** | `pl-4` | **16px** | `terminal.tsx:870` |
| 非活动页签不透明度 | `opacity-60` | — | `terminal.tsx:882` |
| xterm 滚动条 | `width/height: var(--app-scrollbar-size) !important` = 11px；`thumb min-height:36px`；`border-radius: var(--app-scrollbar-radius)` | **11px / 36px** | `terminal.css:41-64`、`scrollbars.css:2` |
| 终端搜索浮层 | `absolute top-2 right-2 z-30`（`.terminal-search { z-index:100 }` 另有一层） | top/right **8px** | `terminal-search.tsx:84`、`terminal.css:72-74` |
| 分屏左右 | `h-full w-1/2 border-border border-r` | 各 50% | `terminal-container.tsx:646` |
| 分屏上下 | `h-1/2 w-full border-border border-b` | 各 50% | `terminal-container.tsx:645` |

xterm 运行时选项（决定内容区观感）：`fontFamily`（来自设置经 `resolveTerminalFont`）、`fontSize = round(terminalFontSize * zoomLevel * 10)/10`、`lineHeight`、`letterSpacing = terminalLetterSpacing * zoomLevel`、`cursorBlink/cursorStyle/cursorWidth/cursorInactiveStyle`、`scrollback`、`convertEol: false`、`allowProposedApi: true`，以及兼容性选项 `customGlyphs:true, reflowCursorLine:false, rescaleOverlappingGlyphs:true, scrollOnUserInput:true, smoothScrollDuration:0`（`terminal.tsx:267-284`、`terminal-options.ts:25-32`）。字号缩放边界 8–32，重置为 14（`terminal.tsx:706、717`）。

### 3.3 终端页签行（TerminalTabBar）

| 元素 | class | 计算值 | 证据 |
| --- | --- | --- | --- |
| 页签条（水平） | `TabBarSurface` horizontal: `h-(--lithe-tab-bar-height) min-h-(--lithe-tab-bar-height) shrink-0 items-center gap-(--lithe-chrome-gap) border-b border-border bg-tab-bar px-(--lithe-chrome-padding-inline)` | 高 **36px**、gap 4px、左右内边距 8px、底边 1px | `windows/tauri/src/ui/tab-bar.tsx:244-255` |
| 页签条（垂直） | `h-full min-h-0 flex-col bg-tab-bar py-(--lithe-chrome-gap)`；宽度取 `tabSidebarWidth`（默认 180，范围 80–400） | 上下内边距 4px | `tab-bar.tsx:249`、`windows/tauri/src/features/terminal/stores/terminal.store.ts:31、65` |
| 单个终端页签（水平） | `TabBarTab` horizontal: `h-(--lithe-tab-height) min-w-20 max-w-(--lithe-tab-max-width) w-fit pl-2 pr-6` | 高 **28px**、最小宽 80px、最大宽 **200px**、左内边距 8px、右内边距 24px（给关闭按钮） | `tab-bar.tsx:257-267` |
| 单个终端页签（垂直） | `min-h-(--lithe-tab-height) w-full max-w-none justify-start rounded-md pl-2 pr-6` | 最小高 28px、圆角 6px | `tab-bar.tsx:261` |
| `Tab` 基类 | `min-h-(--lithe-chrome-control-height) gap-(--lithe-chrome-gap-loose) rounded-(--lithe-chrome-radius) px-2 transition-[...] duration-(--app-duration-fast)` | 最小高 **24px**、gap 6px、圆角 4px、左右 8px、过渡 **150ms** | `tab-bar.tsx:170`、`theme.css:135` |
| 页签文字 | `ui-text-chrome`（13px；comfortable 下 14px）；活动 `text-foreground`，非活动 `text-subtle-foreground` | **13px** | `tab-bar.tsx:170`、`theme.css:115、190`、`terminal-tab-bar-item.tsx:145-150` |
| 活动页签（connected variant） | 透明底 + `before:absolute before:inset-x-1.5 before:bottom-0 before:h-[3px] before:rounded-t-sm before:bg-primary` | 底部强调条 **3px**、左右内缩 6px | `tab-bar.tsx:206-210` |
| 页签容器 | 水平 `flex items-center gap-0.5 overflow-x-auto`；垂直 `flex flex-col gap-0.5 overflow-y-auto px-1.5 py-1` | gap **2px**、垂直内边距 4px/6px | `terminal-tab-bar.tsx:841-847` |
| 已固定页签区 | 水平 `flex items-center gap-0.5 pr-0.5`；垂直 `flex flex-col gap-0.5 pb-0.5` | — | `terminal-tab-bar.tsx:793-798` |
| 工具按钮组 | 水平 `h-8 pl-1`（**32px**）；垂直 `px-1.5 py-1`（6px/4px） | 32px | `terminal-tab-bar.tsx:461-467` |
| 空态条 | `flex min-h-8 items-center justify-between border-border border-b bg-surface px-2 py-1.5` | 最小高 **32px**、内边距 8px/6px | `terminal-tab-bar.tsx:710-714` |
| 垂直页签侧栏拖宽热区 | `absolute top-0 z-10 h-full w-1 cursor-col-resize hover:bg-primary/40 active:bg-primary/60` | 宽 **4px** | `terminal-tab-bar.tsx:909-913` |
| 关闭/固定按钮 | `-translate-y-1/2 absolute top-1/2 right-1`，`size="icon-xs"`；活动或已固定时 `opacity-100`，否则 `group-hover/tab:opacity-100` | right **4px**、按钮 **24×24px** | `terminal-tab-bar-item.tsx:102-107`、`ui/button.tsx:27` |

图标按钮 `size="icon-xs"` = `size-6 p-0` = **24×24px**（`windows/tauri/src/ui/button.tsx:27`，详见 §3.9）。

### 3.4 工具栏 / 右键菜单（终端）

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 工具栏右键菜单最小宽 | `min-w-45` | **180px** | `terminal-tab-bar.tsx:203` |
| 菜单分组标题 | `font-sans ui-text-sm px-2.5 py-1 text-subtle-foreground` | 字号 13px、上下 4px、左右 10px | `terminal-tab-bar.tsx:204、209、216` |
| 菜单分隔线 | `my-0.5 border-border/70 border-t` | 上下 2px | `terminal-tab-bar.tsx:208、215、224` |
| 配置文件菜单宽 | `w-55`；内容 `max-h-72 overflow-y-auto` | **220px** / **288px** | `terminal-tab-bar.tsx:1031、1037` |
| 页签右键菜单项 | 由 `Dropdown` 渲染，快捷键标签 `Keybinding keys={["F2"]}` / `[modKey, "W"]` | — | `terminal-tab-context-menu.tsx:75、89、110` |
| 文档级上下文菜单快捷键提示 | `<ContextMenuShortcut>Ctrl+Shift+C</ContextMenuShortcut>` / `Ctrl+Shift+V` | — | `terminal.tsx:901、913` |

### 3.5 运行面板

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 头部工具行 | `h-(--lithe-pane-header-height) shrink-0 items-center gap-2 border-border/70 border-b px-3` | 高 **36px**、gap 8px、左右 12px | `run-pane.tsx:195` |
| 头部图标 | `size-4` | 16px | `run-pane.tsx:196` |
| 头部标题 | `font-medium ui-text-sm` | 13px | `run-pane.tsx:197` |
| 运行状态文字 | `text-success ui-text-sm` / `text-destructive ui-text-sm` | 13px | `run-pane.tsx:202-206` |
| Java 发现提示条 | `border-border/70 border-b px-3 py-1.5 ui-text-sm` | 上下 6px、左右 12px | `run-pane.tsx:49` |
| 诊断告警横幅 | `border-warning/30 border-b bg-warning/10 px-3 py-2` | 上下 8px、左右 12px；告警图标 `size-3.5` = 14px | `run-pane.tsx:262-263` |
| 配置列表标题 | `px-3 py-2 font-medium text-subtle-foreground ui-text-sm` | 上下 8px、左右 12px | `run-pane.tsx:325` |
| 列表滚动区 | `min-h-0 flex-1 overflow-y-auto pb-2` | 底部 8px | `run-pane.tsx:328` |
| 分组标题 | `px-2 py-1 font-medium text-subtle-foreground ui-text-sm` | 上下 4px、左右 8px | `run-pane.tsx:494` |
| **运行配置行** | `group flex items-center gap-1 rounded-md px-1.5 py-1 ui-text-sm`；选中 `bg-selected text-foreground`，否则 `hover:bg-accent` | 上下 **4px**、左右 6px、gap 4px、圆角 6px、字号 13px；行高约 **24–26px** | `run-pane.tsx:500-503` |
| 折叠组按钮 | `mt-2 flex w-full items-center justify-between px-2 py-1 font-medium text-subtle-foreground ui-text-sm` | 上 8px、上下 4px、左右 8px | `run-pane.tsx:341、361` |
| 配置详情头 | `border-border/70 border-b px-3 py-2` | 上下 8px、左右 12px | `run-pane.tsx:394` |
| 详情键值网格 | `grid grid-cols-[6.5rem_1fr] gap-y-0.5 ui-text-sm` | 首列 **104px**、行间距 2px | `run-pane.tsx:397` |
| 输出滚动区 | `min-h-0 flex-1 overflow-auto px-3 py-2` | 上下 8px、左右 12px | `run-pane.tsx:411` |
| 输出标题 | `mb-1 font-medium text-subtle-foreground ui-text-sm` | 下 4px、13px | `run-output-text.tsx:35` |
| **运行输出正文** | `whitespace-pre-wrap font-mono text-[12px] text-foreground select-text` | 字号 **12px**、等宽 | `run-output-text.tsx:39、48` |
| 时间戳颜色 | `text-subtle-foreground/70` | — | `run-output-style.ts:42` |
| 严重级别颜色 | error `text-destructive`、warning `text-warning`、info `text-info`、debug `text-subtle-foreground` | — | `run-output-style.ts:35-40` |
| stdin 行 | `flex items-center gap-1.5 border-border/70 border-t px-3 py-1.5` | 上下 6px、左右 12px、gap 6px | `run-pane.tsx:454` |
| stdin 输入框 | `h-7 min-w-0 flex-1 rounded-md border border-input bg-transparent px-2 font-mono text-[12px]` | 高 **28px**、圆角 6px、左右 8px、12px 等宽 | `run-pane.tsx:462` |
| 生成提示条 | `border-border/70 border-t px-3 py-1.5 text-subtle-foreground ui-text-sm` | 上下 6px、左右 12px | `run-pane.tsx:425` |
| 缺配置空态 | `flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center`；图标 `size-8` | gap 12px、左右 24px、图标 **32px** | `run-pane.tsx:306-307` |

### 3.6 调试面板

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 头部工具行 | `flex h-10 shrink-0 items-center gap-2 border-border/70 border-b px-3` | 高 **40px**、gap 8px、左右 12px | `debugger-view.tsx:362` |
| 头部图标 | `size={16}` | 16px | `debugger-view.tsx:363` |
| 头部标题 | `truncate font-medium ui-text-sm` | 13px | `debugger-view.tsx:365` |
| 按钮组左侧分隔 | `border-border/60 border-l pl-2` | 左 8px | `debugger-view.tsx:367` |
| 组内竖向分隔 | `mx-1 h-4 w-px bg-border/60` | 高 16px、左右 4px | `debugger-view.tsx:398` |
| 主体网格 | `grid grid-cols-[minmax(260px,320px)_minmax(0,1fr)]` | 左栏 **260–320px** | `debugger-view.tsx:442` |
| 左栏（配置） | `flex min-h-0 flex-col border-border/70 border-r`；内容 `space-y-3 p-3` | 内边距 12px、块间距 12px | `debugger-view.tsx:443-444` |
| 左栏字段标签 | `font-sans text-subtle-foreground ui-text-sm` | 13px | `debugger-view.tsx:446` |
| 左栏命令回显框 | `font-sans min-h-8 truncate rounded-lg border border-border/60 bg-surface/70 px-2 py-1.5 font-mono ui-text-sm text-subtle-foreground` | 最小高 **32px**、圆角 8px、上下 6px、左右 8px | `debugger-view.tsx:472` |
| 主按钮网格 | `grid grid-cols-[1fr_auto_auto] gap-1.5`；启动 `variant="accent"`，继续/暂停 `variant="default" size="icon"`（32×32），停止 `variant="danger" size="icon"` | gap 6px | `debugger-view.tsx:478、483、488-497、498-507` |
| 单步按钮网格 | `grid grid-cols-3 gap-1.5`；三个 `variant="default" size="xs"` | gap 6px、按钮高 **24px** | `debugger-view.tsx:510-537` |
| 会话摘要条 | `border-border/70 border-t px-3 py-2 ui-text-sm`；描述 `mt-1 line-clamp-2 ui-text-sm text-subtle-foreground` | 上下 8px、左右 12px | `debugger-view.tsx:548、558` |
| 项目路径条 | `mt-auto border-border/70 border-t px-3 py-2 ui-text-sm text-subtle-foreground` | 上下 8px、左右 12px | `debugger-view.tsx:564` |
| 右侧页签条 | `flex h-9 shrink-0 items-end gap-1 border-border/70 border-b px-2` | 高 **36px**、gap 4px、左右 8px | `debugger-view.tsx:575` |
| 右侧页签按钮 | `h-8 border-b-2 px-2 font-medium ui-text-sm`；活动 `border-primary text-foreground`，非活动 `border-transparent text-subtle-foreground` | 高 **32px**、下边框 **2px**、左右 8px | `debugger-view.tsx:577-599` |
| 面板网格 | `grid min-h-0 flex-1 grid-cols-2 gap-2 p-2` | gap 8px、内边距 8px、两列 | `debugger-view.tsx:601` |
| 面板卡片 | `flex min-h-0 flex-col overflow-hidden rounded-xl border border-border/70 bg-surface/30` | 圆角 **12px**、1px 边框 | `debugger-panels.tsx:43-45` |
| 面板头 | `flex h-8 shrink-0 items-center gap-1 border-border/60 border-b px-1.5` | 高 **32px**、gap 4px、左右 6px | `debugger-panels.tsx:68` |
| 面板标题按钮 | `flex min-w-0 flex-1 items-center gap-2 rounded-lg px-1.5 py-1 text-left text-subtle-foreground hover:bg-accent/60` | gap 8px、上下 4px、左右 6px、圆角 8px | `debugger-panels.tsx:71` |
| 面板标题文字 | `min-w-0 flex-1 truncate font-medium ui-text-sm uppercase` | 13px、大写 | `debugger-panels.tsx:78` |
| 计数徽标 | `Badge size="compact" variant="muted" className="h-5 tabular-nums"` | 高 **20px**、`px-1.5 py-0.5`（6px/2px）、圆角 full | `debugger-panels.tsx:80`、`windows/tauri/src/ui/badge.tsx:6、19` |
| 空态 | `Empty min-h-0 flex-none rounded-none px-3 py-6` | 上下 24px、左右 12px | `debugger-panels.tsx:102` |
| 调用栈行 | `font-sans flex w-full items-start gap-2 px-3 py-1.5 text-left ui-text-sm hover:bg-accent/70`；选中 `bg-selected/70` | 上下 **6px**、左右 12px、gap 8px；栈图标 `size={13}` | `debugger-panels.tsx:151-157` |
| 栈帧位置行 | `block truncate ui-text-sm text-subtle-foreground` | 13px | `debugger-panels.tsx:160-164` |
| 线程行 | `font-sans flex w-full items-center gap-2 px-3 py-1.5 text-left ui-text-sm hover:bg-accent/70`；选中 `bg-selected/70`；线程 id `font-mono ui-text-sm text-subtle-foreground` | 上下 6px、左右 12px | `debugger-panels.tsx:193-201` |
| 断点行 | `group font-sans flex items-center gap-2 px-3 py-1.5 ui-text-sm hover:bg-accent/70` | 上下 6px、左右 12px；启用点 `size-3 rounded-full border`（**12×12px**），启用 `border-destructive bg-destructive`，禁用 `border-subtle-foreground bg-transparent` | `debugger-panels.tsx:229、238-243` |
| 变量作用域头 | `flex items-center justify-between px-3 py-1.5 ui-text-sm` | 上下 6px、左右 12px | `debugger-variables-panel.tsx:130` |
| 变量行 | `grid grid-cols-[minmax(0,0.42fr)_minmax(0,0.58fr)] gap-2 py-1 pr-3 ui-text-sm`，缩进 `paddingLeft: 18 + depth * 12` | 上下 4px、右 12px、gap 8px；一级缩进 **18px**，每层 +12px；Caret `size={10}` | `debugger-variables-panel.tsx:83-84、95` |
| 变量值 | `truncate font-mono text-foreground` | — | `debugger-variables-panel.tsx:105` |
| 监视面板根 | `space-y-1.5 p-2` | 内边距 8px、块间距 6px | `debugger-watch-panel.tsx:90` |
| 监视条目 | `group rounded-lg border border-border/60 bg-surface/40 px-2 py-1.5`；表达式 `min-w-0 flex-1 truncate text-left font-mono ui-text-sm text-foreground`；结果 `mt-1 truncate font-mono ui-text-sm text-subtle-foreground` | 上下 6px、左右 8px、圆角 8px | `debugger-watch-panel.tsx:135、140、157` |
| 控制台单行 | `whitespace-pre-wrap wrap-break-word px-3 py-1 font-mono ui-text-sm`；stderr `text-destructive`，stdout `text-subtle-foreground` | 上下 4px、左右 12px、13px 等宽 | `debugger-view.tsx:671-672` |
| 控制台保留上限 | `adapterOutput.filter(...).slice(-80)` | 只渲染最近 **80** 条 | `debugger-view.tsx:195` |
| 状态徽标 | `<DebugStatusBadge status={...}/>`：failed→`error`、exited→`success`、paused→`warning`、running→`accent`、其它→`muted`；`Badge variant size="compact" className="gap-1.5"` | 高 20px、gap 6px | `debugger-view.tsx:430`；`debugger-panels.tsx:68-99` |

### 3.7 问题面板

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 面板头 | `paneHeaderClassName()` = `flex min-h-7 items-center gap-1.5 bg-background px-1.5 py-1` | 最小高 **28px**、gap 6px、上下 4px、左右 6px | `pane-chrome.tsx:6`、`diagnostics-pane.tsx:892-893` |
| 头部内容行 | `relative flex min-h-7 w-full items-center gap-1.5` | 最小高 28px、gap 6px | `diagnostics-pane.tsx:899` |
| 问题计数文字 | `font-sans ui-text-sm` + 严重度色（error `text-destructive` / warning `text-warning` / info `text-info` / 无 `text-subtle-foreground`） | 13px | `diagnostics-pane.tsx:900、754-761` |
| 筛选次数徽标 | `Badge variant="accent" size="compact" className="ui-text-sm -top-1 -right-1 absolute min-w-4"` | 最小宽 **16px**、圆角 full | `diagnostics-pane.tsx:949-955` |
| 内容滚动区内边距 | `contentClassName="px-1.5 py-1.5"` | 6px 四周 | `diagnostics-pane.tsx:1049` |
| 分组卡片间距 | `space-y-1.5` | 块间距 **6px** | `diagnostics-pane.tsx:1062` |
| 分组卡片 | `overflow-hidden rounded-xl border border-border/60 bg-surface/40` | 圆角 **12px** | `diagnostics-pane.tsx:1070` |
| 分组头按钮 | `h-auto w-full justify-start gap-1.5 rounded-none border-border/60 border-b bg-background/70 px-2 py-1 text-left hover:bg-accent` | 左右 8px、上下 4px、gap 6px | `diagnostics-pane.tsx:1077` |
| 分组标题文字 | `font-sans ui-text-sm flex-1 truncate font-medium text-foreground` | 13px | `diagnostics-pane.tsx:1091` |
| 分组计数 chip | `PaneChip`：`inline-flex h-5 items-center rounded-full border border-border/70 bg-background px-1.5 text-subtle-foreground` | 高 **20px**、左右 6px、圆角 full | `pane-chrome.tsx:10-12、1095` |
| 行分隔 | `divide-y divide-border/40` | 1px | `diagnostics-pane.tsx:1100` |
| **问题行** | `group cursor-pointer px-2 py-1.5 transition-colors hover:bg-accent` | 上下 **6px**、左右 **8px**；内容两行时行高约 48–56px | `diagnostics-pane.tsx:1122` |
| 行内首行 | `flex items-center gap-1.5` | gap 6px | `diagnostics-pane.tsx:1124` |
| 严重度图标 | `getSeverityIcon(severity, 11)`，`size=11` | **11px** | `diagnostics-pane.tsx:1126、119-130` |
| 消息正文 | `font-sans ui-text-sm min-w-0 flex-1` + 换行时 `whitespace-pre-wrap wrap-break-word leading-snug`，否则 `truncate` | 13px | `diagnostics-pane.tsx:1130-1138` |
| 位置 chip | `PaneChip`：`h-5 px-1.5 rounded-full` | 高 20px | `diagnostics-pane.tsx:1143-1145` |
| 第二行缩进 | `mt-1 pl-5` | 上 4px、左 **20px** | `diagnostics-pane.tsx:1148` |
| 描述文字 | `ui-text-sm mb-1 text-subtle-foreground/90 leading-snug` | 13px、下 4px | `diagnostics-pane.tsx:1152` |
| 文件路径文字 | `ui-text-sm max-w-105 truncate text-subtle-foreground/75` | 13px、最大宽 **420px** | `diagnostics-pane.tsx:1163` |
| 非嵌入外层 | `flex h-44 flex-col border-border border-t bg-background` | 高 **176px**、上边框 1px | `diagnostics-pane.tsx:1212` |

### 3.8 运行操作面板（命令面板式下拉）与其它

| 元素 | class / 值 | px | 证据 |
| --- | --- | --- | --- |
| 下拉面板 | `w-90 max-w-[calc(100vw-1rem)] overflow-hidden rounded-xl p-0` | 宽 **360px**、最大 `100vw-16px`、圆角 **12px**、无内边距 | `run-actions/components/run-actions-button.tsx:288-295` |
| 面板头 | `border-border/70 border-b bg-surface/35 px-3 pt-2.5 pb-2` | 上 10px、下 8px、左右 12px | `run-actions-button.tsx:296` |
| 面板头标题 | `font-medium text-foreground ui-text-sm`；工作区名 `truncate text-subtle-foreground ui-text-sm` | 13px | `run-actions-button.tsx:299-300` |
| 搜索框 | `className="h-8 bg-background"` | 高 **32px** | `run-actions-button.tsx:316-321` |
| 滚动体 | `max-h-[min(420px,60vh)] overflow-y-auto overscroll-contain` → 内层 `py-1` | 最大高 **420px / 60vh** | `run-actions-button.tsx:331-332` |
| 分组标题 | `px-2.5 pt-2 pb-1 font-medium text-subtle-foreground ui-text-sm`；列表 `space-y-0.5 px-1` | 上 8px、下 4px、左右 10px、行间距 2px | `run-actions-button.tsx:76-77` |
| **操作行** | `Item size="xs"` + `min-h-11 flex-nowrap px-1.5 py-1 hover:bg-accent focus-within:bg-accent` | 最小高 **44px**、上下 4px、左右 6px | `run-actions/components/run-action-row.tsx:35-38` |
| 行内图标容器 | `ItemMedia className="grid size-6 rounded-md bg-surface text-subtle-foreground"` | **24×24px**、圆角 6px | `run-action-row.tsx:44` |
| 行内来源徽标 | `Badge size="compact" variant={lsp ? "accent" : "muted"}` | 高 24px（`h-6`）、`px-1.5 py-0.5` | `run-action-row.tsx:50-52`、`ui/badge.tsx:6、19` |
| 行内描述 | `ItemDescription className="block truncate font-mono"` | 等宽 | `run-action-row.tsx:55` |
| 面板底栏 | `border-border/70 border-t p-1.5`；按钮 `ui-text-sm h-8 w-full justify-start gap-2` | 内边距 6px、按钮高 **32px**、gap 8px | `run-actions-button.tsx:375-380` |
| 空态 | `Empty className="min-h-0 flex-none rounded-none px-6 py-8"` | 上下 32px、左右 24px | `run-actions-button.tsx:350、358` |
| 运行配置编辑器根 | `space-y-4` → `space-y-6` → 段 `space-y-2` | 16px / 24px / 8px | `run/components/run-configuration-editor.tsx:211-213` |
| 编辑器配置网格 | `grid grid-cols-[7.5rem_1fr] gap-y-1 ui-text-sm` | 首列 **120px**、行间距 4px | `run-configuration-editor.tsx:248` |
| 保存范围分段控件 | `flex gap-1 rounded-md bg-surface p-0.5`，两个 `size="xs"` 按钮 | gap 4px、内边距 2px、按钮高 24px | `run-configuration-editor.tsx:264-278` |
| 编辑器页脚 | `flex items-center justify-end gap-2 border-t border-border pt-3` | 上 12px、gap 8px | `run-configuration-editor.tsx:428` |
| 配置列表分隔柄 | 厚度 `RUN_CONFIGURATION_LIST_HANDLE_THICKNESS`；`group absolute top-0 right-0 z-20 flex h-full translate-x-1/2 cursor-col-resize items-center justify-center transition-colors duration-(--app-duration-fast) hover:bg-primary/8`；内芯 `h-full w-px group-hover:bg-primary` | 内芯 **1px** | `run/components/run-configuration-list-split.tsx:152-169` |
| 拖动全屏遮罩 | `fixed inset-0 z-40 cursor-col-resize` | — | `run-configuration-list-split.tsx:173` |
| 状态栏调试项 | `FooterTabControl` = `Button variant="ghost" size="xs"` + `font-sans ui-text-chrome font-normal` | 高 **24px**、字号 13px | `layout/components/footer/footer-tab-control.tsx:17、56-73` |
| 项目准备状态（compact） | `relative shrink-0 ui-text-sm`；展开体 `absolute bottom-full left-0 z-50 mb-2 w-80 rounded border border-border bg-background p-3 shadow-lg` | 展开卡片宽 **320px**、内边距 12px | `run/components/project-preparation-status.tsx:30-32、51-55` |
| 项目准备状态（默认） | `border-border border-b px-3 py-2 ui-text-sm`；展开体 `space-y-2 pt-2` | 上下 8px、左右 12px | `project-preparation-status.tsx:30-32、51-55` |
| 工具链行 | `space-y-0.5`；主行 `break-all ui-text-caption` | 字号 **12px**（comfortable 13px） | `run/components/effective-toolchain.tsx:23-28` |

### 3.9 共享 chrome 度量（本节元素复用）

| 元素 | class | 计算值 | 证据 |
| --- | --- | --- | --- |
| `Button` 基类 | `font-sans ui-text-sm inline-flex shrink-0 items-center justify-center gap-1.5 whitespace-nowrap rounded-md leading-row transition-[...] duration-(--app-duration-fast) ease-(--app-ease-smooth) select-none outline-none active:scale-(--app-press-scale)` | 字号 **13px**、gap **6px**、圆角 **6px**、过渡 150ms、按下缩放 `--app-press-scale`（=1） | `windows/tauri/src/ui/button.tsx:9`、`theme.css:135、139` |
| `Button` 默认图标尺寸 | `[&_svg:not([class*='size-'])]:size-3.5` | **14px**（显式带 `size-*` 类时以该类为准） | `button.tsx:9` |
| `Button` variant | `default`：`bg-accent text-foreground hover:bg-selected`；`accent`：`border border-primary/30 bg-primary/12 text-primary`；`ghost`：`bg-transparent text-subtle-foreground hover:bg-accent`；`danger`：`text-foreground hover:bg-destructive/10 hover:text-destructive` | — | `button.tsx:12-20` |
| **`Button` size 全表** | `default: h-8 px-3`、`xs: h-6 gap-1 px-1.5`、`sm: h-7 px-2.5`、`lg: h-9 px-4`、`icon: size-8 p-0`、**`icon-xs: size-6 p-0`**、`icon-sm: size-7 p-0` | 默认 **32px/12px**、xs **24px/6px**（gap 4px）、sm **28px/10px**、lg **36px/16px**、icon **32×32**、**icon-xs 24×24**、icon-sm **28×28** | `button.tsx:21-29` |
| `Tabs` 列表 | `inline-flex w-fit items-center justify-center text-subtle-foreground`；`default`：`gap-(--lithe-chrome-gap-tight) rounded-(--lithe-chrome-radius) bg-surface/55 p-0.5` | gap 2px、圆角 4px、内边距 2px | `windows/tauri/src/ui/tabs.tsx:25-29` |
| `TabsTrigger` size | `xs: h-(--lithe-chrome-control-height) px-2`；`sm: h-(--lithe-tab-height) px-2.5`；`md: min-h-8 px-3 ui-text-base` | xs **24px/8px**、sm **28px/10px**、md **32px/12px** | `windows/tauri/src/ui/tabs.tsx:59-61` |
| 触发态 | `data-active:bg-accent/80 data-active:text-foreground`；默认 `text-subtle-foreground` | — | `tabs.tsx:55` |
| 活动栏列表项（页签选择器） | `SidebarListItem className="ui-text-sm min-h-6 py-1"` | 最小高 **24px**、上下 4px | `sidebar-pane-selector.tsx:332` |
| 状态栏（Footer） | `h-(--lithe-footer-height)` = 1.5rem | **24px** | `windows/tauri/src/ui/chrome.tsx:12-13`、`theme.css:119` |
| 状态栏项最小高 | `min-h-(--lithe-chrome-control-height)` | **24px** | `footer.tsx:53、63、72` |
| 标题栏 | `h-(--lithe-title-bar-height)` = 2.5rem | **40px** | `chrome.tsx:10-11`、`theme.css:118` |
| 活动栏宽度 | `PluginActivityRail`：`w-9.5` | **38px** | `plugin-activity-rail.tsx:34` |

> 由此反推第 3.3 / 3.5 / 3.7 节里 `size="icon-xs"` 的按钮实际都是 **24×24**，终端页签条（高 36px）里的工具按钮与页签关闭按钮都落在 24×24 上（`terminal-tab-bar.tsx:472、485、502、728、746`、`terminal-tab-bar-item.tsx:92`）。

---

## 4. 状态与交互

### 4.1 工具窗页签

| 交互 | 行为 | 证据 |
| --- | --- | --- |
| 切换页签 | `SidebarPaneSelector` 的竖向项 → `toggleTerminalPane / toggleDiagnosticsPane / toggleRunPane` → 写 `bottomPaneActiveTab` + `isBottomPaneVisible` | `main-sidebar.tsx:643-652`；`windows/tauri/src/features/keymaps/commands/view-command-actions.ts:47-52` |
| 页签持久化 | 每次切换写回工作区会话（`workspaceSessionRepository.saveUi`） | `panel-slice.ts:62-74` |
| 自动折叠 | 终端页签可见但页签数从 >0 → 0 时自动隐藏工具窗（`shouldAutoHideTerminalPane`）；debugger/terminal 能力不可用时也自动隐藏 | `terminal-container.tsx:205-219`、`bottom-pane.tsx:68-106` |
| 首次显示自动建终端 | 工具窗变为可见且 `terminals.length === 0` 时自动新建（`terminal-ensure-session` 同理） | `terminal-container.tsx:592-601、547-554` |
| 全屏 | `onFullScreen` 切换 `isFullScreen`，用 `WorkbenchFullscreenSurface` 包裹 | `bottom-pane.tsx:49、261、312-320` |
| 拖拽改高 | mousedown → document mousemove/mouseup，`requestAnimationFrame` 节流，写 `frameEl.style.height` | `bottom-pane.tsx:109-154` |
| 拖放标签到工具窗 | `onDragOver/onDrop`，payload `application/tab-data` 或 `getInternalTabDragData()`；终端面板来源用 `source === "terminal-panel"` 转成 buffer | `bottom-pane.tsx:156-220` |
| 内部拖放高亮 | `lithe-internal-tab-drag-hover` 事件 → `ring-2 ring-primary ring-inset` | `bottom-pane.tsx:53-60、248` |

### 4.2 终端页签

| 交互 | 行为 | 证据 |
| --- | --- | --- |
| 新建 | `terminal-new` 事件 / `terminal.new` 命令（`cmd+t`）→ `handleNewTerminal` → `createTerminal` | `command-registry.ts:258-267`；`terminal-container.tsx:530-537、176-179` |
| 新建并选 profile | `openProfileMenu`（ChevronDown 按钮，菜单位置 `rect.right - 220, rect.bottom + 8`）→ `onNewTerminalWithProfile(profileId)`；菜单底部有「重新检测已安装的 Shell」 | `terminal-tab-bar.tsx:442-451、557-565、1041-1051` |
| 点击页签 | `setActiveTerminal(id)`；焦点由 `XtermTerminal` 的 `isActive` effect「验证式重试」处理（最多 8 次） | `terminal-container.tsx:221-228`；`terminal.tsx:662-687` |
| 关闭页签 | 关闭按钮 / 中键（`onAuxClick` button===1）/ `Delete`·`Backspace`（非固定）/ `terminal.close`（`cmd+w`）→ `closeTerminal` → `invoke(close_terminal)` + `removeSession` | `terminal-tab-bar.tsx:374-377`；`terminal-tab-bar-item.tsx:50-58`；`terminal-container.tsx:71-91` |
| 关闭后焦点 | 关闭活动页签后焦点移到 `currentIndex` 位置（越界则前一个） | `terminal-container.tsx:230-250` |
| 固定 / 取消固定 | 固定页签排到最前（`sortedTerminals` 排序）且关闭按钮变为 Pin 图标；固定页签不被「关闭其他/全部/右侧」影响 | `terminal-tab-bar.tsx:384-386、453-458`；`terminal-container.tsx:281-312` |
| 重命名 | `F2` 或右键「重命名」→ `InlineRenameInput`；提交后同时更新 terminal 名与对应 buffer 名 | `terminal-tab-bar.tsx:334-339、388-416`；`terminal-container.tsx:262-279` |
| 右键菜单 | 页签右键 → `TerminalTabContextMenu`（固定/复制/清除/重命名/导出/关闭/关闭其他/关闭全部/关闭右侧）；条空白处右键 → `ToolbarContextMenu`（终端宽度、标签页布局、标签页位置、新建/搜索/上一下一个/全屏） | `terminal-tab-bar.tsx:320-327、422-432、945-1026` |
| 键盘 | `F2` 重命名；`Shift+F10` / `ContextMenu` 打开右键菜单并定位到页签下沿；方向键（按 orientation 用 `getChromeNavigationIndex`）移动焦点；`Enter`/`Space` 激活；`Delete`/`Backspace` 关闭非固定页签 | `terminal-tab-bar.tsx:329-378` |
| 拖拽重排 | dnd-kit `SortableTab`，`distance: 5` 激活约束，重排过渡 `180ms` + `--app-ease-smooth`；点击抑制用 `useTabDragClickGuard` | `tab-bar.tsx:30-47、71-79`；`terminal-tab-bar.tsx:671-677` |
| 拖到编辑区/工具窗 | 拖出页签条（水平 slop 24px，垂直 slop 64px）→ `openTerminalBuffer` + `terminal-detach-to-buffer`，工具窗时切到 `buffers` 页签 | `terminal-tab-bar.tsx:586-598、643-670` |
| 滚轮横滚 | `onWheel`：水平累加 `deltaX || deltaY` 到 `scrollLeft`；垂直累加 `deltaY || deltaX` 到 `scrollTop` | `terminal-tab-bar.tsx:849-860` |
| 页面切换/移动后重绘 | `TerminalHost` 换 slot 后派发 `lithe-terminal-refit`，xterm 重新 fit + repaint（模拟 SIGWINCH，让 TUI 重绘） | `terminal-host.tsx:121-129`；`terminal.tsx:619-630` |

### 4.3 终端分屏

| 项 | 行为 | 证据 |
| --- | --- | --- |
| 开启 | `terminal.split`（`cmd+d`，向右）/ `terminal.splitDown`（`cmd+shift+d`，向下）→ `terminal-split` 事件 → `handleSplitView(direction)` | `command-registry.ts:286-303`；`terminal-container.tsx:556-559` |
| 实现 | 不是嵌套分屏：为当前页签新建一个伴生页签（名 `${name} (Split)`），记录 `splitMode / splitWithId / splitDirection` | `terminal-container.tsx:314-351` |
| 布局 | `splitDirection === "down"` → `h-1/2 w-full border-b`；否则 `h-full w-1/2 border-r` | `terminal-container.tsx:640-648、667-671` |
| 切换方向 | 已有 split 且方向不同 → 只改方向；再次按同方向 → 取消 split 并关闭伴生页签 | `terminal-container.tsx:321-330` |
| 页签布置模式 | `tabLayout: "horizontal" \| "vertical"`、`tabSidebarPosition: "left" \| "right"`、`tabSidebarWidth`（默认 180，钳制 80–400）、`widthMode: "full" \| "editor"` | `windows/tauri/src/features/terminal/stores/terminal.store.ts:5-7、29-32、65` |

### 4.4 终端内容交互

| 交互 | 行为 | 证据 |
| --- | --- | --- |
| 查找 | `Ctrl/Cmd+F`（或 `terminal.find` 命令）→ `TerminalSearch` 浮层；支持区分大小写/全字/正则；结果计数 `current/total`；`Enter` 下一个、`Shift+Enter` 上一个、`Esc` 关闭 | `terminal.tsx:754-793`；`terminal-search.tsx:69-100` |
| 复制/粘贴 | 右键菜单「复制所选内容」(`Ctrl+Shift+C`) / 「粘贴」(`Ctrl+Shift+V`)；自定义键盘处理 `attachCustomKeyEventHandler` 把 action 映射为 switchTab/write/copy/paste/passthrough | `terminal.tsx:290-321、890-915`；`windows/tauri/src/features/terminal/utils/terminal-keyboard.ts` |
| 多行粘贴确认 | 行数 > 阈值时弹确认框 `terminal.pasteLinesConfirm` | `terminal.tsx:218-231` |
| 链接 | `WebLinksAddon` + 自定义 `ILinkProvider`（文件链接解析）；链接颜色用 `--primary` 注入 `<style>` | `use-terminal-addons.ts:75-95、102-165` |
| 文件拖入 | `onDragOver/onDrop` + `TERMINAL_FILE_DROP_EVENT` → 把路径写入 PTY | `terminal.tsx:176-216` |
| 字号缩放 | `Ctrl/Cmd +/-` 调 ±2（8–32），`Ctrl/Cmd 0` 重置为 14 | `terminal.tsx:704-722、777-788` |
| 标题/目录跟随 | OSC 0/2 → 标题，OSC 7 → 当前目录（`file://` 解析），驱动页签显示名 | `windows/tauri/src/features/terminal/utils/terminal-osc-stream.ts:93-104`；`terminal-tab-bar.tsx:543-556` |
| 导出输出 | 右键「导出输出」→ `serializeAddon.serialize()` → `save` + `writeTextFile`，默认文件名 `<name>_<YYYY-MM-DD>.txt` | `terminal-tab-bar.tsx:974-1008` |
| 清除 | 右键「清除终端」→ `session.ref.current.clear()` | `terminal-tab-bar.tsx:959-964` |
| 复制（分屏） | 右键「复制终端」→ 用当前目录/shell/profile 新建页签 | `terminal-tab-bar.tsx:965-970` |
| 错误兜底 | `TerminalErrorBoundary` 渲染 `terminal.errorTitle` + 错误信息 + 「重试」 | `windows/tauri/src/features/terminal/components/terminal-error-boundary.tsx:40-51` |

### 4.5 输出背压（终端）

| 项 | 值 | 证据 |
| --- | --- | --- |
| 高水位（暂停后端读取） | `500_000` 字节 | `windows/tauri/src/features/terminal/utils/terminal-protocol.ts:13、111` |
| 低水位（恢复） | `100_000` 字节 | `terminal-protocol.ts:14、112` |
| 触发命令 | `terminal_set_paused { id, paused }` → Rust `TerminalReaderControl` Condvar 阻塞 reader 线程 | `use-terminal-connection.ts:89-103`；`crates/terminal/src/protocol.rs:72-102` |

### 4.6 运行按钮与状态

| 状态 | 表现 | 证据 |
| --- | --- | --- |
| 头部主按钮 | `isSelectedRunning ? StopIcon(text-warning) : PlayIcon(text-success)`；`disabled={isLoading || Boolean(javaLaunchDecision)}` | `run-pane.tsx:208-212` |
| 状态文字 | 运行中 `run.running`(text-success)；退出码 0 `run.succeeded`(text-success)；否则 `run.failed`(text-destructive) | `run-pane.tsx:201-207` |
| 列表行内按钮 | 每行一个「运行/停止」图标按钮：`running ? StopIcon(text-warning) : PlayIcon(text-success)` | `run-pane.tsx:522-529` |
| 重新扫描 | `run.rescan` tooltip；`disabled={!rootFolderPath || isLoading}` | `run-pane.tsx:221-231` |
| 滚动到底 | `active={scrollOutputToEnd}` + `aria-pressed`（持久化到 run-preferences） | `run-pane.tsx:232-243`；`windows/tauri/src/features/run/stores/run-preferences.store.ts` |
| 清除输出 | `actions.clearOutput()` | `run-pane.tsx:244-248` |
| 最小化 | `setIsBottomPaneVisible(false)` | `run-pane.tsx:249-258` |
| 多服务 | `RunServicesMenu`：勾选服务列表、`run.runSelectedServices` / `run.runAllServices` | `run-pane.tsx:171-177、213-220` |
| 失败决策横幅 | Java 构建未成功时 `JavaLaunchDecisionBanner`（仍然运行 / 对此工作区始终继续 / 重建 Java 索引 / 打开日志 / 取消） | `run-pane.tsx:291-302`；键见 §6 |

### 4.7 调试按钮与状态

| 元素 | 状态逻辑 | 证据 |
| --- | --- | --- |
| 开始（Play） | `disabled={!canStartDebugging \|\| isActiveSession}` | `debugger-view.tsx:368-377` |
| 继续/暂停（动态） | `isPaused ? Play + debugger.continue : Pause + debugger.pause`；`disabled={!canSendAdapterThreadRequest}` | `debugger-view.tsx:378-387` |
| 停止（Square） | `disabled={!isActiveSession}` | `debugger-view.tsx:388-397` |
| 单步跳过/进入/跳出 | `disabled={!canStep}` | `debugger-view.tsx:399-428` |
| 状态徽标 | `DebugStatusBadge status={activeSessionDisplayStatus}`；状态映射 idle/running/paused/exited/stopped/failed | `debugger-view.tsx:430`；`debugger-panels.tsx:26-41`；文案见 §6 |
| 断点切换 | `debug.toggleBreakpoint` / 按钮 `disabled={!activeFile}` | `debugger-view.tsx:431-439`；`default-keymaps.ts:383` |
| 控制台清空 | 有输出时才显示 Trash 按钮 | `debugger-view.tsx:651-660` |
| 断点清空 | 有断点时才显示 Trash 按钮 | `debugger-view.tsx:686-696` |
| 变量展开 | 变量面板支持逐级展开/加载（`debugger.loadingVariable`）；监视支持求值/移除/刷新 | `debugger-variables-panel.tsx:107`；`debugger-watch-panel.tsx:101-162` |

### 4.8 问题面板交互

| 交互 | 行为 | 证据 |
| --- | --- | --- |
| 点击问题行 | `handleDiagnosticClick` → `handleFileSelect(path, false, line+1, column+1)` | `diagnostics-buffer.tsx:31-52` |
| 键盘 | 行 `role="button" tabIndex={0}`，`Enter`/`Space` 跳转 | `diagnostics-pane.tsx:1110-1121` |
| 右键菜单 | 快速修复（最多 8 条，懒加载 `lspClient.getCodeActions`）/ 转到问题 / 复制消息·位置·完整详情 / 按来源筛选 / 切换换行 | `diagnostics-pane.tsx:604-722` |
| 筛选菜单 | 分组（文件/严重程度/不分组）、排序（严重程度/文件/位置）、各严重度开关（显示 `visible/total`）、仅当前文件、清除来源筛选、重置全部筛选 | `diagnostics-pane.tsx:763-855` |
| 头部菜单 | 全宽/编辑器宽度、全屏/退出全屏 | `diagnostics-pane.tsx:857-886` |
| 搜索 | `SearchPopover`，`Esc` 先清空再关闭 | `diagnostics-pane.tsx:982-1028` |
| 文件导航栏 | 可开关（`diagnostics.hideFiles`/`showFiles`），带每文件计数与最高严重度着色 | `diagnostics-pane.tsx:903-919、436-481` |
| 分组折叠 | 点击分组头 toggle；选中文件时自动展开该分组 | `diagnostics-pane.tsx:526-547` |
| 偏好持久化 | `localStorage["diagnostics-pane-prefs"]` | `diagnostics-pane.tsx:79、191-206、265-268` |
| 计数徽标 | 头部显示 `N 个问题`（单复数同文案）；筛选按钮上叠加 filter 计数徽标 | `diagnostics-pane.tsx:750-753、948-956` |

### 4.9 状态 store 形状（重写时需要对齐的状态机）

| store | 关键状态 | 持久化 | 证据 |
| --- | --- | --- | --- |
| `run.store.ts` | `root`、`javaDiscovery: "idle"\|"loading"\|"ready"\|"stale"\|"failed"`、`status: "missing"\|"ready"\|"invalid"`（初值 `"missing"`）、`isLoading`、`isGenerating`、`recoveryAction: "none"\|"regenerate"\|"editConfiguration"\|"fixPermissions"\|"upgradeApplication"`（初值 `"regenerate"`）、`recoveryPath`、`invalidMessage`、`diagnostics`、`configurations`、`selectedConfigurationId`、`defaultConfigurationId`、`primaryOutput`、`primaryRunning`、`primaryTitle`、`primaryExitCode`、`sessions: RunSession[]`、`selectedSessionId`、`saveError`、`editingConfigurationId`、`generationNotice`、`javaLaunchDecisions`、`discoveredJava/Maven/Runtimes`、`globalToolchain`、`effectiveRuntimeExecutablePaths` | 输出上限 `MAXIMUM_OUTPUT_CHARACTERS = 500_000`；会话 id 规则：`execution === "service"` 用配置 id，否则 `PRIMARY_SESSION_ID = "primary"` | `run/store/run.store.ts:71、93-154、404-431、673-674`；`run/types/run.types.ts:1-7、210` |
| `run-preferences.store.ts` | `configurationListWidth`、`scrollOutputToEnd`（默认 **true**）、`selectedServiceIDsByWorkspace`、`javaBuildFailurePolicyByWorkspace`（默认 `"ask"`） | localStorage `"lithe-run-preferences"`；工作区键归一化（反斜杠→斜杠、去尾斜杠、小写） | `run/stores/run-preferences.store.ts:10-13、22-23、29、52、66-72` |
| `project-preparation.store.ts` | `phase: "starting"\|"importing"\|"configuring"\|"building"\|"ready"\|"stopped"`、`status: "idle"\|"loading"\|"ready"\|"failed"`、`blocksRun` | 无（内存） | `run/stores/project-preparation.store.ts:4-8、18-58` |
| `run-actions.store.ts` | `runActions: CustomRunAction[]`（工作区专属 + 共享两段） | localStorage `"terminal-custom-actions"`，version **1** | `run-actions/stores/run-actions.store.ts:8、45-54、67-68` |
| `debugger.store.ts` | `breakpoints`、`watchExpressions`、`watchResults`、`workspaceConfigs`、`userConfigs`、`activeConfigId`、`activeSession`（`status: "idle"\|"running"\|"paused"`）、`adapterMessages`、`adapterOutput`、`endedSessions`、`threads`、`stackFrames`、`selectedFrameId`、`scopes`、`variablesByReference`、`stoppedState`、`pendingRequests` | localStorage：`"lithe-debugger-breakpoints"`、`"lithe-debugger-user-configs"`、`"lithe-debugger-watch-expressions"`；上限 `adapterMessages` 500、`adapterOutput` 500、`endedSessions` 100 | `debugger/stores/debugger.store.ts:20-73、84-92、121-127、330-345` |
| 显示态扩展 | `DebugSessionDisplayStatus = status \| "exited" \| "stopped" \| "failed"`（由 `endedReason` 映射） | — | `debugger/components/debugger-panels.tsx:26-41` |
| `diagnostics.store.ts` | `diagnosticsByFile: Map<file, Diagnostic[]>`、`diagnosticsByOwner: Map<file, Map<owner, Diagnostic[]>>` | 无 | `diagnostics/stores/diagnostics.store.ts:13-14` |
| 问题面板偏好 | `groupBy`（默认 `"file"`）、`sortBy`（默认 `"severity"`）、`onlyCurrentFile`（默认 false）、`wrapMessages`（默认 **true**）、`fileNavigatorViewMode`（默认 `"flat"`） | localStorage `"diagnostics-pane-prefs"` | `diagnostics/components/diagnostics-pane.tsx:79-87、265-268` |
| `terminal.store.ts` | `sessions: Map<id, Partial<Terminal>>`、`widthMode: "full"\|"editor"`（默认 `"editor"`）、`tabLayout: "horizontal"\|"vertical"`（默认 `"horizontal"`）、`tabSidebarWidth`（默认 **180**，钳制 **80–400**）、`tabSidebarPosition: "left"\|"right"`（默认 `"left"`） | 无 | `terminal/stores/terminal.store.ts:5-7、29-32、65` |

### 4.10 快捷键汇总（默认键位）

| 键位 | 命令 | 上下文 | 证据 |
| --- | --- | --- | --- |
| `cmd+t` | `terminal.new` | `!terminalFocus` 时为 `workbench.newTab`；`terminalFocus` 时为 `terminal.new` | `default-keymaps.ts:11-16、122-127` |
| `cmd+n` | `terminal.new` | `terminalFocus` | `default-keymaps.ts:58-63` |
| `cmd+f` | `terminal.find` | `terminalFocus`（否则 `workbench.showFind`） | `default-keymaps.ts:128-133、350-355` |
| `cmd+w` | `terminal.close` | `terminalFocus`（否则 `file.close`） | `default-keymaps.ts:46-51、134-139` |
| `cmd+d` | `terminal.split`（向右） | `terminalFocus` | `default-keymaps.ts:140-145` |
| `cmd+shift+d` | `terminal.splitDown`（向下） | `terminalFocus` | `default-keymaps.ts:146-151` |
| `cmd+j` | `workbench.toggleTerminal` | 全局 | `default-keymaps.ts:329`；`command-registry.ts:582-588` |
| ``cmd+` `` | `workbench.toggleTerminalAlt` | 全局 | `default-keymaps.ts:331`；`command-registry.ts:589-595` |
| `shift+f10` | `workbench.toggleRun` | 全局 | `default-keymaps.ts:330`；`command-registry.ts:575-581` |
| `cmd+shift+j` | `workbench.toggleDiagnostics` | 全局 | `default-keymaps.ts:332-338`；`command-registry.ts:603-609` |
| `ctrl+tab` / `ctrl+shift+tab` | next/prev 页签（终端聚焦时切终端页签，否则切 buffer） | 全局 | `default-keymaps.ts:417-422`；`command-registry.ts:810-824` |
| `F5` | `debug.start` | 全局 | `default-keymaps.ts:381` |
| `shift+F5` | `debug.stop` | 全局 | `default-keymaps.ts:382` |
| `F9` | `debug.toggleBreakpoint` | 全局 | `default-keymaps.ts:383` |
| `ctrl+shift+f10` | `run.runContextConfiguration` | `editorFocus` | `default-keymaps.ts:384-389` |
| `cmd+shift+d` | `workbench.showDebugger` | 全局（**与 `terminal.splitDown` 复用同一键位**，靠 `when` 区分） | `default-keymaps.ts:376-380`；`command-registry.ts:690-696` |
| `cmd+.` | `editor.quickFix`（使用 `diagnostics/utils/quick-fix.ts` 的 `selectDiagnosticForQuickFix` / `selectPreferredCodeAction`） | — | `default-keymaps.ts:309-314`；`features/keymaps/commands/editor-command-actions.ts:463-477` |
| `alt+9` | `workbench.toggleGitLog` | 全局 | `default-keymaps.ts:497` |
| `F11` | `window.toggleFullscreen` | 全局 | `default-keymaps.ts:514` |

- 键位归一：`cmd` 在 Windows/Linux 规范化为 `ctrl`（`windows/tauri/src/features/keymaps/utils/parser.ts:75`）。
- `F2` / `Shift+F10` / `ContextMenu` / `Enter` / `Space` / `Delete` / `Backspace` 是**终端页签内部**的键盘处理，不走 keymaps 系统（`terminal-tab-bar.tsx:329-378`）。
- 终端内部键盘动作（切页签/写字节/复制/粘贴/透传）由 `attachCustomKeyEventHandler` 处理（`terminal.tsx:290-321`），规则在 `windows/tauri/src/features/terminal/utils/terminal-keyboard.ts`。
- **未找到** F8 / Shift+F8（下一个/上一个问题）等诊断导航默认键位；预设文件 `features/keymaps/defaults/keybinding-presets.ts` 中无 run/debug/terminal/diagnostics 条目。
- 运行/调试面板**没有 restart 与 disconnect 按钮**：`RunPane` 只有运行(停止)按钮（`run-pane.tsx:208-212`），`DebuggerView` 只有启动/暂停继续/停止（`debugger-view.tsx:368-397`）。

工具条 tooltip 中显示的快捷键（`tooltip.shortcut`）：终端 `Mod+J`、诊断 `Mod+Shift+J`、运行 `Shift+F10`、Git Log `Alt+9`、项目 `Mod+Shift+E`、搜索 `Mod+Shift+F`、更改 `Mod+Shift+G`（`sidebar-pane-selector.tsx:130、145、161、180、199、216、233`）。

---

## 5. 数据与命令

### 5.1 终端相关

| 类型 | 名称 | 定义处 | 前端调用处 |
| --- | --- | --- | --- |
| Tauri command | `begin_frontend_terminal_session` | `windows/tauri/src-tauri/src/terminal.rs:85-98`（注册 `main.rs:130`） | `terminal/utils/frontend-terminal-session.ts:24` |
| Tauri command | `warm_terminal_environment` | `src-tauri/src/terminal.rs:100-103`（`main.rs:131`） | 未找到前端调用点（grep `warm_terminal_environment` 在 `windows/tauri/src` 无结果） |
| Tauri command | `create_terminal` | `src-tauri/src/terminal.rs:105-129`（`main.rs:132`） | `terminal/components/terminal.tsx:405-418` |
| Tauri command | `terminal_write` | `src-tauri/src/terminal.rs:131-140`（`main.rs:133`） | `hooks/use-terminal-connection.ts:60`、`terminal-container.tsx:492` |
| Tauri command | `terminal_resize` | `src-tauri/src/terminal.rs:142-151`（`main.rs:134`） | `hooks/use-terminal-connection.ts:114` |
| Tauri command | `terminal_set_paused` | `src-tauri/src/terminal.rs:153-162`（`main.rs:135`） | `hooks/use-terminal-connection.ts:95` |
| Tauri command | `close_terminal` | `src-tauri/src/terminal.rs:164-174`（`main.rs:136`） | `hooks/use-terminal-connection.ts:201`、`terminal-container.tsx:83` |
| Tauri command | `list_shells` | `src-tauri/src/terminal.rs:176-181`（`main.rs:137`） | `terminal/stores/shells.store.ts:48` |
| Tauri channel | `Channel<TerminalEvent>`（`create_terminal.onEvent`） | `src-tauri/src/terminal.rs:108` | `terminal/utils/terminal-protocol.ts:28-45` |
| 远程变体 | `create_remote_terminal` / `remote_terminal_write` / `remote_terminal_resize` / `remote_terminal_set_paused` / `close_remote_terminal` | 未在 `src-tauri/src` 的 `generate_handler!` 中找到（`main.rs:112-191` 无这些名字）；应由 `platform::platform_invoke` 分派 | `terminal.tsx:393`、`use-terminal-connection.ts:60、95、114、201` |
| Tauri 事件（自定义 DOM） | `lithe-terminal-refit`、`terminal-ready`、`terminal-switch-tab`、`terminal-detach-to-buffer`、`terminal-new`、`close-active-terminal`、`terminal-open-search`、`terminal-ensure-session`、`terminal-split`、`terminal-activate-tab`、`create-terminal-with-command` | — | `terminal-host.tsx:126`、`terminal.tsx:439、295`、`terminal-tab-bar.tsx:663`、`terminal-container.tsx:423、467、501、511、526、567-571` |
| 命令 id（命令面板 / 键位） | `terminal.new`、`terminal.close`、`terminal.find`、`terminal.split`、`terminal.splitDown` | `features/keymaps/commands/command-registry.ts:258-304` | 工具栏按钮 `commandId="terminal.new"`（`terminal-tab-bar.tsx:487、729`）、`commandId="terminal.find"`（`:474`）、`commandId="terminal.close"`（`terminal-tab-bar-item.tsx:113`） |

`lithe-core` 中**没有** `terminal.*` 命令（`grep '"terminal\.' rust/lithe-core/src/protocol/command.rs` 无结果），终端完全属于 Windows 平台层（与 `AGENTS.md` 中「Windows-only terminal 行为留在 Tauri host」一致）。

### 5.2 运行相关

| 类型 | 名称 | 后端定义 | 前端调用 |
| --- | --- | --- | --- |
| Tauri command | `run_list_java_sources` | `src-tauri/src/run.rs:213-221` | `features/run/services/java-entrypoint-discovery.ts`（未逐行核对） |
| Tauri command | `run_write_generated` | `run.rs:222-231` | `run/store`（经 `run-configuration` 工具） |
| Tauri command | `run_write_documents` | `run.rs:232-255` | 同上 |
| Tauri command | `run_discover_toolchains` | `run.rs:256-280` | `features/run/stores/run.store.ts` |
| Tauri command | `maven_resolve_installation` | `run.rs:281-286` | `features/maven/` |
| Tauri command | `run_resolve_launch` | `run.rs:328-421` | `run.store` |
| Tauri command | `run_resolve_toolchains` | `run.rs:422-438` | `features/run/hooks/use-resolved-toolchains.ts` |
| Tauri command | `run_execute_prelaunch` | `run.rs:559-586` | `run.store` |
| Tauri command | `run_start_process` | `run.rs:587-656` | `run.store` |
| Tauri command | `run_stop_process` | `run.rs:684-693` | `run.store` |
| Tauri command | `run_write_stdin` | `run.rs:694-716` | `run.store`（`RunStdinInput` → `actions.writeStdin`，`run-pane.tsx:419-422`） |
| Tauri 事件 | `run-output` `{ sessionId, chunk }` | 发出处 `run.rs:1959-1963` | 监听 `features/run/hooks/use-run-process-events.ts:21-26` |
| Tauri 事件 | `run-exit` `{ sessionId, exitCode }` | 发出处 `run.rs:2008-2012` | 监听 `use-run-process-events.ts:28-33` |
| Core 命令 | `runConfig.inspect` / `runConfig.updateOptions` / `runConfig.generate` / `runConfig.resolve` / `runConfig.createLaunchPlan` / `runConfig.saveEditorChanges` | `rust/lithe-core/src/`（经 `core_execute` 分派） | `features/run/api/run-core-api.ts:44、49、69、79、90、118` |

后端常量：输出刷新间隔 **100ms**、高水位 **1 MiB**、队列容量 **64 chunk**、`CREATE_NO_WINDOW = 0x0800_0000`（`run.rs:24-27`）。

### 5.3 调试相关

| 类型 | 名称 | 后端定义 | 前端调用 |
| --- | --- | --- | --- |
| Tauri command | `debug_start_session` | `src-tauri/src/debug.rs:172-178`（`main.rs:120`） | `features/debugger/api/debug-adapter-host-api.ts:13` |
| Tauri command | `debug_connect_session` | `debug.rs:329-444`（`main.rs:121`） | `debug-adapter-host-api.ts:19` |
| Tauri command | `debug_allocate_loopback_port` | `debug.rs:445-455`（`main.rs:122`） | `debug-adapter-host-api.ts:23` |
| Tauri command | `debug_wait_for_port` | `debug.rs:456-471`（`main.rs:123`） | `debug-adapter-host-api.ts:27` |
| Tauri command | `debug_session_ready` | `debug.rs:492-496`（`main.rs:124`） | `debug-adapter-service.ts:48` |
| Tauri command | `debug_send_request` | `debug.rs:524-586`（`main.rs:125`） | `debug-adapter-service.ts:29` |
| Tauri command | `debug_stop_session` | `debug.rs:587-593`（`main.rs:126`） | `debug-adapter-service.ts:44` |
| Tauri command | `debug_stop_workspace_sessions` | `debug.rs:594-609`（`main.rs:127`） | `windows/tauri/src/features/file-system/stores/file-system.store.ts:3071`（**不在 `features/debugger` 内**） |
| Tauri 事件 | `debugger_message` `{ sessionId, message }` | `debug.rs:954-957、988`（另 `:515`） | `debug-adapter-service.ts:148`（**全局 `listen`**） |
| Tauri 事件 | `debugger_output` | `debug.rs:1129-1132`（stderr）、`:270-273`、`:1201-1204` | `debug-adapter-service.ts:151` |
| Tauri 事件 | `debugger_session_ended` | `debug.rs:1157-1164`(`exited`)、`:661-664`(`stopped`)、`:1205-1208`(`failed`)、`:1216-1219` | `debug-adapter-service.ts:154` |
| Core 命令（session） | `debug.createSession`（`debug.rs:238、378`）、`debug.launch`（`:710`）、`debug.execute`（`:748`）、`debug.inspect`（`:744`）、`debug.receive`（`:1027`）、`debug.disconnect`（`:626`）、`debug.destroySession`（`:255、636、1156、1200、1215`）、`debug.setBreakpoints`（`:735`）、`debug.setVariable`（`:758`） | `shared/contracts/rust-core-api.md:113-130、976-1140`；名称解析表 `rust/lithe-core/src/protocol/command.rs:344-361` | 经 `debug.rs` 的 `execute_core_sync`（`debug.rs:887-920`）与 `translate_request`（`:687-779`） |
| 请求翻译表（DAP command → Core 命令） | `launch\|attach`→`debug.launch`；`setBreakpoints`→`debug.setBreakpoints`；`threads\|stackTrace\|scopes\|variables\|evaluate`→`debug.inspect`；`continue\|pause\|next\|stepIn\|stepOut\|stepBack`→`debug.execute`；`setVariable`→`debug.setVariable`；其它 → `"Unsupported debug adapter request: {command}"` | `debug.rs:687-779` | — |
| 命令 id | `debug.start`、`debug.stop`、`debug.toggleBreakpoint`、`workbench.showDebugger` | `features/keymaps/commands/command-registry.ts:698-717` | `debugger-view.tsx:483、503` |

**契约与实现的名称核对结果**：`shared/contracts/rust-core-api.md:113-130` 列出的 18 个 `debug.*` 命令中，`rust/lithe-core/src/protocol/command.rs` 的 `from_name` / 名称表（`:344-361`、`:505-520`）只直接出现 5 个字符串：`debug.launch`（`:345`、`:505`）、`debug.execute`（`:356`、`:516`）、`debug.inspect`（`:357`、`:517`）、`debug.receive`（`:358`、`:518`）、`debug.disconnect`（`:360`、`:520`）。但 **Windows host 实际使用的 `debug.createSession` / `debug.setBreakpoints` / `debug.setVariable` / `debug.destroySession` 在 `debug.rs` 中以字符串字面量传给 Core**（见上表行号），却未在 `command.rs` 的字符串表命中 —— 说明 Core 侧的解析另有路径（可能是 `arguments.command` 子分派或 `execute_json` 的其它入口），**本次未逐行确认**（见 §8）。

### 5.4 问题/诊断相关

| 类型 | 名称 | 定义处 | 前端调用 |
| --- | --- | --- | --- |
| Tauri command | `preview_diagnostic_bundle` / `export_diagnostic_bundle` | `src-tauri/src/diagnostics.rs`（注册 `main.rs:118-119`） | 未在 `features/diagnostics` 找到调用点（该模块是日志/诊断包导出，不是「问题」列表） |
| Core 命令 | `diagnostics.redactText`、`diagnostics.buildManifest` | `rust/lithe-core/src/protocol/command.rs:444-445` | `src-tauri/src/diagnostics.rs` |
| Tauri 事件 | `lsp://diagnostics` | 前端订阅 `windows/tauri/src/features/editor/lsp/lsp-client.ts:565`；**Rust 发射端未定位**（见 §8） | `lsp-client.ts:605-633` → `convertLSPDiagnostic` → `setDiagnostics(path, …, "lsp")` |
| Tauri command | `lint_code` | **未找到 Rust 定义**（`windows/tauri/src-tauri/src/` grep 无 `#[tauri::command] lint_code`，`main.rs:112-191` 未注册，`platform.rs` 的 `translate` 未匹配） | `features/editor/linter/linter-service.ts:66-86` |
| LSP 通道 | `LspClient.getInstance()` → `getCodeActions` / `applyCodeAction` | `features/editor/lsp/lsp-client.ts` | `diagnostics-pane.tsx:224、293-306、551` |
| 数据源 | `useDiagnosticsStore.diagnosticsByFile` | `features/diagnostics/stores/diagnostics.store.ts`：LSP（`convertLSPDiagnostic`，severity 1→error / 2→warning / **3 与 4 都 → info**，`:29-56`）+ 内置 linter（`convertLintDiagnostic`，**hint → info**，`:61-78`） | `diagnostics-buffer.tsx:20-29` |
| store 结构 | `diagnosticsByFile: Map<file, Diagnostic[]>`、`diagnosticsByOwner: Map<file, Map<owner, Diagnostic[]>>`；`setDiagnostics(filePath, diagnostics, owner = "default")`、`clearDiagnosticsForOwner` | `diagnostics.store.ts:13-14、86-105、120-140` | — |
| 保留上限 | `MAX_WORKSPACE_DIAGNOSTIC_FILES = 2000`，保留决策委托 `decideWorkspaceDiagnostics` | `lsp-client.ts:203`、`lsp/diagnostics-retention.ts:32`、`lsp-client.ts:478-509` | — |
| TypeScript/JS 内建诊断 | 被显式关闭以让位 LSP：`setDiagnosticsOptions(lspOwnedDiagnosticsOptions)` | `features/editor/engines/monaco/language-contributions.ts:20、26` | — |
| 跳转事件 | `menu-go-to-line` | 消费端 `features/editor/components/code-editor.tsx:611` | `diagnostics-buffer.tsx:45-49` |

**严重度只有三档**：`Diagnostic["severity"] = "error" | "warning" | "info"`（`features/diagnostics/types/diagnostics.types.ts:1-12`）。LSP 的 `Hint(4)` 与 linter 的 `hint` **都被折叠为 info**，因此**没有独立的 hint 图标/颜色**（`diagnostics.store.ts:40-42、68`）。

**问题计数没有全局状态栏徽标**：`grep 'problems|Problems|errorCount|warningCount'` 在 `windows/tauri/src/**/*.tsx` 只命中 GitHub PR 面板（`features/github/components/pr-files-panel.tsx:37、166-168`、`github-pr-viewer.tsx:218`），与编辑器诊断无关。问题计数只出现在 `DiagnosticsPane` 头部（`diagnostics-pane.tsx:900`）。底部状态栏里的调试项只把计数用于**菜单项禁用**，不显示徽标（`features/layout/components/footer/footer-debugger-item.tsx:39-43、88-108`）。

### 5.5 ⚠️ 命令白名单不一致（会在运行/调试主路径上失败）

`windows/tauri/src/platform/tauri-core.ts:18-91` 维护了一份原生命令白名单 `nativeCommands`；不在名单里的命令会被改写成 `platform_invoke`（`tauri-core.ts:125`）。以下**已注册且被前端真实调用**的命令**不在白名单**，且 `platform.rs` 的 `translate`（`:211-607`）**没有对应分支**，会落到 `:602-605` 返回 `"Windows platform command is not implemented: <name>"`：

| 命令 | 已注册 | Rust 定义 | 前端调用 | 可达路径 |
| --- | --- | --- | --- | --- |
| `run_execute_prelaunch` | `main.rs:188` | `run.rs:559-585` | `features/run/api/run-host-api.ts:107` | **`run.store.ts:904` 的 `plan.preLaunchSteps` 循环（正常运行前置编译会走到）** |
| `debug_connect_session` | `main.rs:121` | `debug.rs:329-442` | `features/debugger/api/debug-adapter-host-api.ts:19` | `maven-module-debug.ts:65-84` → `maven-pane.tsx:280`（Maven 模块调试） |
| `debug_allocate_loopback_port` | `main.rs:122` | `debug.rs:445-453` | `debug-adapter-host-api.ts:23` | 同上 |
| `debug_wait_for_port` | `main.rs:123` | `debug.rs:456-470` | `debug-adapter-host-api.ts:27` | 同上 |
| `maven_resolve_installation` | `main.rs:184` | `run.rs:281-315` | `features/maven/api/maven-host-api.ts:26` | Maven 工具窗 |

对比：`tauri-core.ts:33-37` 只登记了 `debug_send_request` / `debug_start_session` / `debug_session_ready` / `debug_stop_session` / `debug_stop_workspace_sessions`；`:65-73` 登记了 9 个 `run_*`（**不含 `run_execute_prelaunch`**）；`:51-52` 只登记 `maven_load_configuration` / `maven_write_configuration`。**未找到**任何断言「`nativeCommands` 覆盖全部已注册命令」的测试（`tauri-core.test.ts:23` 只断言了 `lsp_rebuild_java_index`）—— 这可能是 Windows 端运行/调试的真实缺陷，重写时务必对齐。

### 5.6 DAP 适配器如何被启动（路径 / env / args）

- **没有内置或打包的适配器二进制路径解析**。`debug_start_session` 直接 `Command::new(command)`，参数用 `config.adapterArgs`，`cwd = config.cwd ?? workspacePath`，`env = config.env`（`windows/tauri/src-tauri/src/debug.rs:189-200`；前端 `features/debugger/services/debug-adapter-service.ts:51-70`）。
- `adapterCommand` / `adapterArgs` / `env` 的**唯一来源是工作区 `.vscode/launch.json`**：`debugger-view.tsx:245-246` 读 `joinPath(root, ".vscode", "launch.json")` → `parseDebugLaunchJson`（`utils/debugger-command.ts:168-170`）→ `normalizeLaunchConfigs` 只接受字符串形式的 `adapterCommand`（`:139-140`）与 `adapterArgs`（`:144-146`），来源标记 `source: "workspace"`（`:162`）。自动生成的配置**不设** `adapterCommand`（`createGeneratedDebugConfig`，`:38-64`）。
- 无 launch.json 时回退到「终端命令」：`buildDebugCommand` 生成 `bun/node --inspect-brk`、`python -m pdb`、`cargo run`、`dlv debug`（`debugger-command.ts:66-83`），由 `debugger-view.tsx:283-306` 派发 `create-terminal-with-command`。
- **Java 路径不用进程适配器**：经 JDT LS `java_start_debug_session` 起 Java Debug Server 并返回端口（`maven-module-debug.ts:73-74`）；目标 JVM 端口由 `debug_allocate_loopback_port` 分配（`:65`）并传入启动计划（`:67-71` → `run.store.ts:641-644` → `run-core-api.ts:82-104` 的 `createLaunchPlan(..., debugPort, ...)`）；等端口就绪（`waitForJvmDebugPort`，30s，`:26、:72、:172`）后用 `debug_connect_session` 以 TCP `127.0.0.1:{adapterPort}` 接入（`debug-adapter-service.ts:72-82`）。
- 会话 id 规则：进程适配器 `windows-debug-{pid}-{counter}`，`adapterId` 取命令 file stem（缺省 `"custom"`）（`debug.rs:215-219、1238-1245`）；TCP 适配器 `windows-java-debug-{pid}-{counter}`，`adapterId: "java"`，`pid = 0` 表示外部拥有（`debug.rs:356-360、381、63-64、416`）。
- TCP 连接超时 **5s**（`debug.rs:35、343-348`），`set_nodelay(true)`（`:349`）；等端口上限 **60s**，轮询粒度 `min(remaining, 50ms)`（`debug.rs:36、458-464、487`），且**不消耗调试套接字**（测试断言 `:1405-1417`）。

---

## 6. 文案（`windows/tauri/src/i18n/locale.ts`）

locale 文件结构：`export const DISPLAY_LANGUAGES = ["en-US", "zh-CN"]`（`locale.ts:2`），`const catalogs = { "en-US": { ... }, "zh-CN": { ... } }`（`locale.ts:6-7、4468`），键为**扁平点分字符串**，插值用 `{name}`。

### 6.1 终端（`terminal.*`，zh-CN 区块 `locale.ts:7503-7558`）

| key | 中文原文 |
| --- | --- |
| `terminal.searchPlaceholder` | 在终端中查找... |
| `terminal.searchMatchCase` | 区分大小写 |
| `terminal.searchMatchWholeWord` | 全字匹配 |
| `terminal.searchUseRegex` | 使用正则表达式 |
| `terminal.contextPin` | 固定终端 |
| `terminal.contextUnpin` | 取消固定终端 |
| `terminal.contextDuplicate` | 复制终端 |
| `terminal.contextClear` | 清除终端 |
| `terminal.contextRename` | 重命名终端 |
| `terminal.contextExport` | 导出输出 |
| `terminal.contextClose` | 关闭终端 |
| `terminal.contextCloseOthers` | 关闭其他终端 |
| `terminal.contextCloseAll` | 关闭所有终端 |
| `terminal.contextCloseRight` | 关闭右侧终端 |
| `terminal.tabNamePlaceholder` | 终端名称 |
| `terminal.tabUnpin` | 取消固定终端 |
| `terminal.tabClose` | 关闭 {name} |
| `terminal.tabAriaPinned` | （已固定） |
| `terminal.tabRenameAria` | 重命名 {name} |
| `terminal.toolbarFullWidth` | 全宽 |
| `terminal.toolbarEditorWidth` | 编辑器宽度 |
| `terminal.toolbarHorizontalTabs` | 水平标签页 |
| `terminal.toolbarVerticalTabs` | 垂直标签页 |
| `terminal.toolbarTabsOnLeft` | 标签页在左侧 |
| `terminal.toolbarTabsOnRight` | 标签页在右侧 |
| `terminal.toolbarTerminalWidth` | 终端宽度 |
| `terminal.toolbarTabLayout` | 标签页布局 |
| `terminal.toolbarTabPosition` | 标签页位置 |
| `terminal.newTerminal` | 新建终端 |
| `terminal.search` | 搜索 |
| `terminal.nextTab` | 下一个标签页 |
| `terminal.previousTab` | 上一个标签页 |
| `terminal.exitFullScreen` | 退出全屏 |
| `terminal.fullScreen` | 全屏 |
| `terminal.fullScreenTerminal` | 全屏终端 |
| `terminal.findInTerminal` | 在终端中查找 |
| `terminal.detectShells` | 重新检测已安装的 Shell |
| `terminal.detectingShells` | 正在检测 Shell… |
| `terminal.detectShellsFailed` | Shell 检测失败，请重试。现有终端不受影响。 |
| `terminal.shellUnavailable` | 暂未检测到 |
| `terminal.chooseTerminalProfile` | 选择终端配置文件 |
| `terminal.noTerminals` | 没有终端 |
| `terminal.terminalTabs` | 终端标签页 |
| `terminal.resizeSidebar` | 调整终端侧栏大小 |
| `terminal.textFiles` | 文本文件 |
| `terminal.allFiles` | 所有文件 |
| `terminal.errorTitle` | 终端错误 |
| `terminal.errorFallback` | 无法初始化终端 |
| `terminal.retry` | 重试 |
| `terminal.pasteLinesConfirm` | 要将 {count} 行粘贴到终端吗？这可能会执行多条命令。 |
| `terminal.pasteIntoTerminal` | 粘贴到终端 |
| `terminal.copySelection` | 复制所选内容 |
| `terminal.paste` | 粘贴 |
| `terminal.openExternalLink` | 打开外部链接 |
| `terminal.openExternalLinkConfirm` | 要在浏览器中打开此链接吗？\n\n{url} |
| `terminal.terminals` | 终端 |

（`terminal.copySelection` 有单测锁定：`windows/tauri/src/i18n/locale.test.ts:64`。）

### 6.2 运行（`run.*`，zh-CN 区块 `locale.ts:5927-6042`）

| key | 中文原文 |
| --- | --- |
| `run.title` | 运行 |
| `run.run` | 运行配置 |
| `run.stop` | 停止运行 |
| `run.running` | 运行中 |
| `run.succeeded` | 成功 |
| `run.failed` | 失败 |
| `run.rescan` | 重新扫描服务 |
| `run.clearOutput` | 清除运行输出 |
| `run.scrollToEnd` | 输出始终滚动到最后一行 |
| `run.minimize` | 最小化 |
| `run.configurations` | 运行配置 |
| `run.resizeConfigurationList` | 调整运行配置列表宽度 |
| `run.services` | 服务 |
| `run.infrastructure` | Docker 服务 |
| `run.applications` | 应用 |
| `run.tasks` | 任务 |
| `run.groups` | 组 |
| `run.otherConfigurations` | 其他运行配置 |
| `run.chooseServices` | 选择要运行的服务 |
| `run.runSelectedServices` | 运行选中的服务 |
| `run.runAllServices` | 运行所有服务 |
| `run.configurationDetails` | 配置详情 |
| `run.processOutput` | 进程输出 |
| `run.emptyOutput` | 运行配置后将在这里显示进程输出。 |
| `run.selectConfiguration` | 选择一个运行配置以查看输出。 |
| `run.missingTitle` | 未找到项目运行配置 |
| `run.missingMessage` | （多行，`locale.ts:5953-5954`，需按原文照抄） |
| `run.invalidTitle` | 项目运行配置无效 |
| `run.identifyAndGenerate` | 识别并生成 |
| `run.identifying` | 正在识别项目… |
| `run.identifyAgain` | 重新识别 |
| `run.toolchainNeedsAttention` | 项目工具链需要处理 |
| `run.staleConfigurations` | 运行配置可能已过期 |
| `run.freshnessCheckFailed` | 未能确认运行配置是否为最新 |
| `run.javaBuildFailedTitle` | {name} 的 Java 构建未成功完成 |
| `run.javaBuildMarkersMayRemain` | （多行，`locale.ts:5963-5964`） |
| `run.javaBuildWorkspaceScope` | （多行，`locale.ts:5965-5966`） |
| `run.javaBuildElapsed` | 构建响应耗时：{milliseconds} 毫秒。该数据只用于说明情况，不决定是否允许运行。 |
| `run.javaBuildContinue` | 仍然运行 |
| `run.javaBuildAlwaysContinue` | 对此工作区始终继续 |
| `run.javaBuildRebuildIndex` | 重建 Java 索引 |
| `run.javaBuildOpenLogs` | 打开日志 |
| `run.javaBuildAlwaysContinueEnabled` | 此工作区遇到 Java 构建失败时会直接继续启动，不再询问。 |
| `run.javaBuildAskAgain` | 构建失败时重新询问 |
| `run.editService` | 编辑服务 |
| `run.generatedEntries` | 已生成 {count} 个可运行项目入口。 |
| `run.runTarget` | 运行 '{target}' |
| `run.modifyRunConfiguration` | 修改运行配置… |
| `run.runMarkerFailed` | 无法运行 {target} |
| `run.javaDiscoveryLoading` | 正在等待 Java 语言服务列出可运行的类... |
| `run.javaDiscoveryStale` | Java 语言服务正在准备项目，暂时显示上次的 Java 入口。 |
| `run.javaDiscoveryFailed` | 无法刷新 Java 入口：{message} |
| `run.editorTitle` | 运行配置 |
| `run.saveScope` | 保存范围 |
| `run.saveScopeLocal` | 此电脑 |
| `run.saveScopeProject` | 项目 |
| `run.saveScopeLocalHint` | 保存在 .lithe/run/local.json，并已从 Git 中排除。 |
| `run.saveScopeProjectHint` | （多行，`locale.ts:5987-5988`） |
| `run.configuration` | 配置 |
| `run.type` | 类型 |
| `run.effectiveSource` | 生效来源 |
| `run.mainClass` | 主类 |
| `run.jdkHome` | JDK 主目录 |
| `run.jdkHomeHint` | 留空则使用自动检测到的 JDK。 |
| `run.mavenExecutable` | Maven 主目录 / 可执行文件 |
| `run.mavenExecutableHint` | 可选择 Maven 主目录；留空则使用项目 Wrapper 或系统 Maven。 |
| `run.nodeExecutable` | Node.js 可执行文件 |
| `run.nodeExecutableHint` | 可选择 node.exe；留空则使用自动检测到的 Node.js 运行时。 |
| `run.mavenJdkHome` | Maven JDK 主目录 |
| `run.mavenJdkHomeHint` | 留空则与应用使用同一个 JDK。 |
| `run.mavenTests` | Maven 测试 |
| `run.mavenTestsProjectDefault` | 使用项目默认值 |
| `run.mavenTestsRun` | 运行测试 |
| `run.mavenTestsSkip` | 跳过测试 |
| `run.mavenTestsHint` | 为当前运行配置覆盖 Maven 工具窗口中的“跳过测试”设置。 |
| `run.toolchainAuto` | 自动检测（留空） |
| `run.toolchainCurrent` | 当前路径 |
| `run.runtimeSection` | 运行环境（本机） |
| `run.projectDefaultsSection` | 项目默认运行环境 |
| `run.configurationOverridesSection` | 当前配置覆盖 |
| `run.toolchainProjectDefault` | 使用项目默认值 |
| `run.configurationOverrideHint` | 留空则使用项目默认值。 |
| `run.programArguments` | 程序参数 |
| `run.vmArguments` | JVM 参数 |
| `run.environment` | 环境变量 |
| `run.stdinPlaceholder` | 输入程序所需内容… |
| `run.stdinSend` | 发送 |
| `run.workingDirectory` | 工作目录 |
| `run.workingDirectoryHint` | 留空则使用项目根目录。 |
| `run.command` | 命令 |
| `run.done` | 完成 |
| `run.cancel` | 取消 |
| `run.projectAction` | 运行项目操作 |
| `run.deleteAction` | 删除运行操作 |
| `run.deleteActionMessage` | 要删除运行操作“{name}”吗？ |
| `run.rescanProjectActions` | 重新扫描项目操作 |
| `run.scanning` | 正在扫描 |
| `run.filterActions` | 筛选操作 |
| `run.currentFile` | 当前文件 |
| `run.detectedInProject` | 项目中检测到 |
| `run.custom` | 自定义 |
| `run.scanningProjectActions` | 正在扫描项目操作 |
| `run.noMatchingActions` | 没有匹配的操作 |
| `run.noRunnableActionsFound` | 未找到可运行操作 |
| `run.tryAnotherActionSearch` | 请尝试其他名称、命令或来源。 |
| `run.addCustomCommandHint` | 添加自定义命令，或打开带有可运行 LSP CodeLens 操作的文件。 |
| `run.newCustomAction` | 新建自定义操作 |
| `run.runCell` | 运行单元 |
| `run.runChunk` | 运行代码块 |

### 6.3 调试（`debugger.*`，zh-CN 区块 `locale.ts:5086-5145`）

| key | 中文原文 |
| --- | --- |
| `debugger.runAndDebug` | 运行和调试 |
| `debugger.statusIdle` | 空闲 |
| `debugger.statusRunning` | 运行中 |
| `debugger.statusPaused` | 已暂停 |
| `debugger.statusExited` | 已退出 |
| `debugger.statusStopped` | 已停止 |
| `debugger.statusFailed` | 失败 |
| `debugger.toggleCurrentLineBreakpoint` | 切换当前行断点 |
| `debugger.configuration` | 配置 |
| `debugger.debugConfiguration` | 调试配置 |
| `debugger.command` | 命令 |
| `debugger.commandToRun` | 要运行的命令 |
| `debugger.noCommandAvailable` | 没有可用命令 |
| `debugger.start` | 启动 |
| `debugger.continue` | 继续 |
| `debugger.pause` | 暂停 |
| `debugger.continueDebugging` | 继续调试 |
| `debugger.pauseDebugging` | 暂停调试 |
| `debugger.stop` | 停止 |
| `debugger.stepOver` | 单步跳过 |
| `debugger.stepInto` | 单步进入 |
| `debugger.stepOut` | 单步跳出 |
| `debugger.over` | 跳过 |
| `debugger.into` | 进入 |
| `debugger.out` | 跳出 |
| `debugger.paused` | 已暂停 |
| `debugger.noLaunchJsonFound` | 未找到 launch.json |
| `debugger.openProjectToLoadLaunchJson` | 打开项目以加载 launch.json |
| `debugger.stack` | 调用栈 |
| `debugger.threads` | 线程 |
| `debugger.threadsAndVariables` | 线程和变量 |
| `debugger.variables` | 变量 |
| `debugger.watch` | 监视 |
| `debugger.console` | 控制台 |
| `debugger.breakpoints` | 断点 |
| `debugger.showRunAndDebug` | 显示运行和调试 |
| `debugger.hideRunAndDebug` | 隐藏运行和调试 |
| `debugger.toggleRunAndDebug` | 切换运行和调试 |
| `debugger.clearConsole` | 清空控制台 |
| `debugger.clearBreakpoints` | 清空断点 |
| `debugger.clearWatchExpressions` | 清空监视表达式 |
| `debugger.clearDebugConsole` | 清空调试控制台 |
| `debugger.adapterOutputAppearsHere` | 适配器输出会显示在这里。 |
| `debugger.stackEmpty` | 启动会话后可查看栈帧。 |
| `debugger.threadsEmpty` | 暂停进程后可查看线程。 |
| `debugger.variablesEmpty` | 暂停在栈帧上以查看变量值。 |
| `debugger.breakpointsEmpty` | 点击行号槽或切换当前行断点。 |
| `debugger.lineNumber` | 第 {line} 行 |
| `debugger.disableBreakpoint` | 禁用断点 |
| `debugger.enableBreakpoint` | 启用断点 |
| `debugger.removeBreakpoint` | 移除断点 |
| `debugger.loadingVariable` | 正在加载变量 |
| `debugger.empty` | 空 |
| `debugger.addExpression` | 添加表达式 |
| `debugger.addWatch` | 添加监视 |
| `debugger.refreshWatches` | 刷新监视 |
| `debugger.watchEmpty` | 添加表达式，以便暂停时查看。 |
| `debugger.removeWatch` | 移除监视 |
| `debugger.evaluating` | 正在求值... |
| `debugger.notEvaluated` | 未求值 |

### 6.4 问题（`diagnostics.*`，zh-CN 区块 `locale.ts:7342-7383`）

| key | 中文原文 |
| --- | --- |
| `diagnostics.problemCount` | {count} 个问题 |
| `diagnostics.problemCountOne` | {count} 个问题 |
| `diagnostics.empty` | 未检测到问题 |
| `diagnostics.noMatch` | 没有符合当前筛选条件的问题 |
| `diagnostics.resetFilters` | 重置筛选 |
| `diagnostics.search` | 搜索问题 |
| `diagnostics.filter` | 筛选问题 |
| `diagnostics.groupByFile` | 分组：文件 |
| `diagnostics.groupBySeverity` | 分组：严重程度 |
| `diagnostics.groupByNone` | 不分组 |
| `diagnostics.sortBySeverity` | 排序：严重程度 |
| `diagnostics.sortByFile` | 排序：文件 |
| `diagnostics.sortByPosition` | 排序：位置 |
| `diagnostics.severityCount` | {label} ({visible}/{total}) |
| `diagnostics.errors` | 错误 |
| `diagnostics.warnings` | 警告 |
| `diagnostics.info` | 信息 |
| `diagnostics.onlyCurrentFile` | 仅当前文件 |
| `diagnostics.clearSourceFilter` | 清除来源筛选（{source}） |
| `diagnostics.clearSourceFilterSimple` | 清除来源筛选 |
| `diagnostics.resetAllFilters` | 重置全部筛选 |
| `diagnostics.hideFiles` | 隐藏文件 |
| `diagnostics.showFiles` | 显示文件 |
| `diagnostics.closePane` | 关闭问题面板 |
| `diagnostics.filesAria` | 诊断文件 |
| `diagnostics.all` | 全部诊断 |
| `diagnostics.fullWidth` | 全宽 |
| `diagnostics.editorWidth` | 编辑器宽度 |
| `diagnostics.fullScreen` | 全屏 |
| `diagnostics.exitFullScreen` | 退出全屏 |
| `diagnostics.goToProblem` | 转到问题 |
| `diagnostics.copyMessage` | 复制消息 |
| `diagnostics.copyLocation` | 复制位置 |
| `diagnostics.copyDetails` | 复制完整详情 |
| `diagnostics.filterBySource` | 按来源筛选：{source} |
| `diagnostics.enableWrap` | 启用消息换行 |
| `diagnostics.disableWrap` | 禁用消息换行 |
| `diagnostics.loadingFixes` | 正在加载快速修复... |
| `diagnostics.noFixes` | 没有可用的快速修复 |
| `diagnostics.messageCopied` | 已复制诊断消息 |
| `diagnostics.locationCopied` | 已复制诊断位置 |
| `diagnostics.detailsCopied` | 已复制诊断详情 |

### 6.5 工具窗 / 页签相关（`workbench.*`、`layout.*`、`ui.*`）

| key | 中文原文 | 出处 |
| --- | --- | --- |
| `workbench.terminal` | 终端 | `locale.ts:5915` |
| `workbench.diagnostics` | 诊断 | `locale.ts:5916` |
| `workbench.run` | 运行 | `locale.ts:5913` |
| `workbench.maven` | Maven | `locale.ts:5914` |
| `workbench.gitLog` | 提交记录 | `locale.ts:5910` |
| `workbench.activityViews` | 活动视图 | `locale.ts:7890` |
| `layout.resizeBottomPane` | 调整底部面板大小 | `locale.ts:7888`（英文 `Resize bottom pane`，`locale.ts:3569`） |
| `layout.resizeSidebar` | 调整侧边栏大小 | `locale.ts:7887` |
| `preparation.starting` | 正在启动 Java 服务 | `locale.ts:5917` |
| `preparation.importing` | 正在导入 Java 项目与依赖 | `locale.ts:5918` |
| `preparation.configuring` | 正在同步 Java 项目配置 | `locale.ts:5919` |
| `preparation.building` | 正在构建 Java 项目 | `locale.ts:5920` |
| `preparation.ready` | Java 项目模型已就绪 | `locale.ts:5921` |
| `preparation.stopped` | Java 项目准备已停止 | `locale.ts:5922` |
| `preparation.failed` | Java 项目准备失败，请查看详情 | `locale.ts:5923` |
| `preparation.explanation` | Java 工作区准备完成后才能启动。普通后台索引不阻塞运行，编译在运行前执行。 | `locale.ts:5924` |
| `preparation.settings` | 语言服务设置与重试 | `locale.ts:5925` |
| `preparation.logs` | 查看日志 | `locale.ts:5926` |
| `runActions.editRunAction` | 编辑运行操作 | `locale.ts:6107` |
| `runActions.newRunAction` | 新建运行操作 | `locale.ts:6108` |
| `runActions.saveChanges` | 保存更改 | `locale.ts:6109` |
| `runActions.addAction` | 添加操作 | `locale.ts:6110` |
| `runActions.savedForPrefix` | 此操作将保存到 | `locale.ts:6111` |
| `runActions.savedForSuffix` | 并在新终端中运行。 | `locale.ts:6112` |
| `runActions.name` | 名称 | `locale.ts:6113` |
| `runActions.namePlaceholder` | 启动开发服务器 | `locale.ts:6114` |
| `runActions.workingDirectoryDescription` | 留空则使用项目根目录，或输入相对路径，例如 | `locale.ts:6115-6116` |
| `runActions.run` | 运行 | `locale.ts:6117` |
| `runActions.test` | 测试 | `locale.ts:6118` |
| `runActions.check` | 检查 | `locale.ts:6119` |
| `runActions.build` | 构建 | `locale.ts:6120` |
| `runActions.projectScript` | 项目脚本 | `locale.ts:6121` |
| `runActions.editNamedAction` | 编辑 {name} | `locale.ts:6122` |
| `runActions.deleteNamedAction` | 删除 {name} | `locale.ts:6123` |
| `footer.statusBar` | 状态栏 | `locale.ts:7316` |
| `footer.moveLeft` / `footer.moveRight` / `footer.resetOrder` | 左移 / 右移 / 重置页脚顺序 | `locale.ts:7319-7321` |
| `ui.open` / `ui.cancel` / `ui.save` / `ui.delete` | 打开 / 取消 / 保存 / 删除 | `locale.ts:4616、4609、4617、4618` |

**重要：`problems.*` 与 `output.*` 前缀的 key 不存在**（在 zh-CN 区块全量检索零命中）。问题面板统一用 `diagnostics.*`，运行输出用 `run.*`。

多行模板字符串的完整原文（前文只给了键名）：

| key | 中文原文 | 证据 |
| --- | --- | --- |
| `run.missingMessage` | 生成 .lithe/run/generated.json 后即可运行项目。已有的项目和本机覆盖不会被改动。 | `locale.ts:5953-5954` |
| `run.javaBuildMarkersMayRemain` | 语言服务的构建器在本次会话中发生过失败，因此当前报告的部分错误可能是那次失败留下的。 | `locale.ts:5963-5964` |
| `run.javaBuildWorkspaceScope` | 本次构建未能确认唯一目标项目，因此其他工作区模块的错误可能影响了结果。 | `locale.ts:5965-5966` |
| `run.saveScopeProjectHint` | 保存在 .lithe/run/configurations.json，供整个团队使用。本机 JDK 路径不会被共享。 | `locale.ts:5987-5988` |
| `runActions.workingDirectoryDescription` | 留空则使用项目根目录，或输入相对路径，例如 | `locale.ts:6115-6116` |

---

## 7. gpui-kit 0.6.6 对应建议

> 本节把上面每个可复用 UI 元素映射到 gpui-kit 0.6.6 的类型，判定档位为「可一比一 / 需自行组合 / gpui-kit 没有」。
> 源码根（下文所有行号均相对此根）：
> - `RT = D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837`
> - `gpui-kit` = `RT\gpui-kit-0.6.6\src\`（**只有 `lib.rs` 一个门面文件**，见 `gpui-kit-0.6.6\src\lib.rs:1-160`；它 `pub use ::gpui::*`、`pub use ::gpui_base as base`、`#[cfg(feature="component")] pub use ::gpui_component as component`）
> - `gpui-base` = `RT\gpui-base-0.6.6\src\`
> - `gpui-component` = `RT\gpui-component-0.6.6\src\`
> 因此引用路径写作 `gpui_kit::base::…` / `gpui_kit::component::…`（`gpui_kit::*` 直接可用 GPUI 本身）。
> `gpui-component` 的 `[features]` 与 dock 无关（只有 `decimal` / `inspector` / `test-support` / 一批 `tree-sitter-*`），**dock 无需 feature 门控**：`gpui-base-0.6.6\src\lib.rs:24`（`pub mod dock;`）。

### 7.1 终端内容区：**gpui-kit 没有终端模拟器 / PTY / ANSI 解析**

已核实（精确匹配标识符 `portable_pty|conpty|ConPTY|alacritty|termwiz|crossterm|vte|vt100|ansi_escape|termion|wezterm`）：
- `gpui-component-0.6.6\src\`、`gpui-kit-0.6.6\src\`、`gpui-base-0.6.6\src\` 下全部 `.rs` 文件（含子目录）→ **0 命中**。
- 三个 crate 的 `Cargo.toml` 依赖列表 → **0 命中**。
  - `gpui-component-0.6.6\Cargo.toml` `[dependencies.*]`：`anyhow, chrono, enum-iterator, gpui(=gpui-pre 0.3.6), gpui-base, gpui-component-macros, gpui-kit-assets, gpui_macros, instant, itertools, log, lsp-types, markdown, notify, num-traits, once_cell, paste, regex, ropey, rust-i18n, rust_decimal(optional), schemars, serde, serde_json, serde_repr, smallvec, sum-tree, tracing, uuid`（+ 平台/可选 `smol`、`tree-sitter*`、macOS `core-text/objc2*`、Windows `resvg/windows`）。
  - `gpui-base-0.6.6\Cargo.toml`：`aho-corasick, anyhow, chrono, data-url, futures, gpui, gpui_macros, html5ever, instant, lsp-types, markdown, markup5ever_rcdom, regex, ropey, schemars, serde, serde_json, smallvec, sum-tree, tracing, unicode-segmentation, web-time`（+ 平台）。
  - `gpui-kit-0.6.6\Cargo.toml`：`gpui(=gpui-pre 0.3.6), gpui-base, gpui-component(optional), gpui-kit-assets(optional), gpui_platform(desktop), gpui_web(wasm)`。
- 唯一出现的 "terminal" 字样只可能在 `gpui-component::dock` 的文档示例变量名中（**本次对 `gpui-component-0.6.6\src\dock\mod.rs` 直接检索 `terminal` → 0 命中**），不构成任何类型或依赖。

**结论：终端内容区无法用 gpui-kit 现成组件实现；PTY/终端模拟必须自行引入**（如 `portable-pty` + `vte` / `alacritty_terminal` / `wezterm-term` / `vt100` 之一）。

因此终端内容区只有两条路：
1. **自研 VT 渲染**（在 GPUI 的 `Element` / `canvas` 上画字符网格），或
2. **别的替代方案**（内嵌第三方终端窗口 / 复用现有 native 终端）。

### 7.2 自研对照：Windows 端实际用到的 xterm.js 能力清单

**addon（`windows/tauri/package.json` 依赖 + `windows/tauri/src/features/terminal/hooks/use-terminal-addons.ts`）**

| addon | 版本 | 用途 | 必需性证据 |
| --- | --- | --- | --- |
| `@xterm/xterm` | ^6.0.0 | 核心 VT 解析 + 渲染 + 输入编码 | `package.json`；`terminal.tsx:267-289` |
| `@xterm/addon-fit` | ^0.11.0 | 容器尺寸 → 行列数 | `use-terminal-addons.ts:34、45`；`terminal.tsx:170` |
| `@xterm/addon-search` | ^0.16.0 | 查找、结果计数、装饰 | `use-terminal-addons.ts:35、46`；`terminal.tsx:692-702、802-828` |
| `@xterm/addon-serialize` | ^0.14.0 | 导出 scrollback 为文本 | `use-terminal-addons.ts:36、47`；`terminal.tsx:853` |
| `@xterm/addon-unicode11` | ^0.9.0 | Unicode 11 宽度表（CJK/emoji 宽字符） | `use-terminal-addons.ts:37、48`；`terminal.tsx:358` |
| `@xterm/addon-clipboard` | ^0.2.0 | OSC 52 剪贴板（仅写） | `use-terminal-addons.ts:38-43、49` |
| `@xterm/addon-web-links` | ^0.12.0 | URL 识别 + 点击（先确认再 `open(uri)`） | `use-terminal-addons.ts:75-95`；`terminal.tsx:349` |
| `@xterm/addon-webgl` | ^0.19.0 | WebGL 渲染器，含 context loss 回退 | `use-terminal-addons.ts:56-73`；`terminal.tsx:287-289` |
| ~~`@xterm/addon-canvas`~~ | 未在依赖中 | — | `package.json` 未列出 |

**自研必须覆盖的行为（按依赖优先级）**

| # | 行为 | 证据 |
| --- | --- | --- |
| 1 | VT 解析：CSI/SGR/OSC、光标移动、滚动区域、备用屏幕、重绘/reflow | `terminal.tsx:267-289`（xterm 核心职责） |
| 2 | 字符网格渲染 + 真彩 SGR（`COLORTERM=truecolor`，后端设置见 `connection.rs:209`） | `crates/terminal/src/connection.rs:208-210` |
| 3 | Unicode 11 宽字符/组合字符宽度 | `terminal.tsx:358` |
| 4 | 光标样式（block/bar/underline、闪烁、inactive 样式、宽度） | `terminal.tsx:272-275`、设置项 `terminalCursorStyle` 等 |
| 5 | scrollback 缓冲 + 滚动条样式（`--app-scrollbar-*`，thumb 最小 36px） | `terminal.tsx:279`；`terminal.css:41-69` |
| 6 | 选择（选区渲染、选中文本导出、活动选区高亮 `--selected`） | `terminal.tsx:745-749、853` |
| 7 | 链接层（`ILinkProvider`、按行解析文件路径、hover 光标） | `use-terminal-addons.ts:102-165`；`terminal.css:77-89` |
| 8 | 输入编码：`onData`/`onBinary`，含 Alt/Option 作为 Meta、Alt+Click 移动光标、右键选词 | `use-terminal-connection.ts:145-146`；`terminal.tsx:276-282` |
| 9 | 键盘拦截：自定义 key handler 分出「切页签 / 写字节 / 复制 / 粘贴 / 透传」 | `terminal.tsx:290-321`；`windows/tauri/src/features/terminal/utils/terminal-keyboard.ts` |
| 10 | IME/输入法：`beforeinput`（`insertReplacementText` / `insertFromDrop`）与 `paste` 事件旁路 | `terminal.tsx:323-347` |
| 11 | OSC 标题（0/2）与 OSC 7 当前目录解析 | `terminal-osc-stream.ts:93-104` |
| 12 | 尺寸同步（`ResizeObserver` + `visualViewport` + `document.fonts`）→ `terminal_resize` | `terminal.tsx:632-651`；`terminal-protocol.ts:86-96` |
| 13 | 背压：高/低水位暂停后端读取（500 000 / 100 000 字节） | `terminal-protocol.ts:13-14、107-114` |
| 14 | 换行/宽字符下的 reflow 关闭语义 `reflowCursorLine:false`、`rescaleOverlappingGlyphs:true`、`customGlyphs:true` | `terminal-options.ts:25-32` |
| 15 | ConPTY 兼容标记 `windowsPty: { backend: "conpty", buildNumber }` | `terminal-options.ts:34-48` |
| 16 | 进程退出提示：非 0 退出时在终端内打印黄色告警行 | `use-terminal-connection.ts:213-220` |
| 17 | 拖入文件 → 路径文本写入 PTY | `terminal.tsx:176-216` |

### 7.3 终端外壳（可复用部分）

| 元素 | gpui-kit 0.6.6 建议 | 判定 |
| --- | --- | --- |
| 终端页签行（`TerminalTabBar`，水平 36px） | `gpui_kit::component::tab::TabBar`（`gpui-component-0.6.6/src/tab/tab_bar.rs:39`）+ `gpui_kit::component::tab::Tab`（`tab/tab.rs:396`），经 `TabVariant::Underline`（`tab/tab.rs:13`）可拿到 36px 默认高（`tab/tab.rs:24-43`：Medium+Underline = 36，其余变体 32）；高度也可用 `Tab::new(..).h(px(28.))` 覆盖（`Tab: Styled` 在 `tab/tab.rs:606`，`render` 末尾 `refine_style` 在 `tab/tab.rs:518`） | **可一比一**（尺寸需显式覆盖） |
| 页签活动态底部 3px 强调条 | 无「3px 下划线」参数；最接近的是 `TabVariant::Underline` + `TabVariant::selected(cx)`（`tab/tab.rs:221-264`），厚度/颜色由主题 token 决定。要精确 3px 需自绘 `div` 底部条 | **需自行组合** |
| 工具栏图标按钮（搜索/新建/下箭头/全屏） | `gpui_kit::component::button::Button`（`gpui-component-0.6.6/src/button/button.rs:185`）+.icon(..)（`:383`）+ `.ghost()`（`:78`）+.tooltip(..)（`:389`）+.disabled(..)（`:503`）。**没有 `IconButton` 类型**：`Button::is_icon_only` 由「只设 icon 不设 label」推断（`button/button.rs:307-310`） | **可一比一** |
| 页签右键菜单 | `gpui_kit::component::menu::ContextMenu`（`menu/context_menu.rs:41`）+ `ContextMenuExt::context_menu(..)`（`:12-19`）+ `PopupMenu`（`menu/popup_menu.rs:281`）/`PopupMenuItem`（`:32`）；`PopupMenuItem::separator()`（`:113`）+ `.disabled(bool)`（`:161`）+ `.icon(..)`（`:126`） | **可一比一** |
| 页签工具栏右键菜单（终端宽度/标签页布局/位置） | `PopupMenu` + `PopupMenuItem::checked(bool)`（`popup_menu.rs:180`）+ `label(..)`（`:119`）+ `separator()`（`:113`） | **可一比一** |
| 终端配置文件下拉菜单（列表 + 底部「重新检测」按钮） | `PopupMenu`（`popup_menu.rs:281`）`.max_h(..)`（`:441`）`.scrollable(true)`（`:447`）+ `PopupMenuItem::element(..)`（`:85`）挂按钮 | **可一比一** |
| 页签关闭按钮 | **`Tab` 无内建关闭按钮**（`tab/tab.rs`/`tab_bar.rs` 内无 close/closable/on_close）。用 `Tab::suffix(impl IntoElement)`（`tab/tab.rs:541`）塞一个 `Button` | **需自行组合** |
| 页签拖拽重排 | `TabBar`/`Tab` **不支持拖拽**。可拖拽的 tab group 只在 dock：`gpui_kit::component::dock::TabGroup`（`gpui-base-0.6.6/src/dock/tab_group.rs:145`）+ `TabGroupContext::{drag_panel, drop_panel, drop_item}`（`tab_group.rs:855/863/876`）+ `DragPanel`（`dock/drag.rs:34`）/`DropIndicator`（`dock/drag.rs:156`） | **需自行组合**（或改用 dock 的面板体系） |
| 垂直页签布局（180px 侧栏） | `TabBar`/`Tab` **无 orientation/vertical** 参数（源码无 `vertical`/`orientation`）。可考虑用 `gpui_kit::component::sidebar::Sidebar`（`gpui-component-0.6.6/src/sidebar/mod.rs:222`）+ `SidebarMenu`（`sidebar/menu.rs:20`）/`SidebarMenuItem`（`sidebar/menu.rs:94`）模拟，或自绘竖列 | **gpui-kit 没有（需替代方案）** |
| 空态条（`terminal.noTerminals`） | `gpui_kit::component::empty::Empty`（`gpui-component-0.6.6/src/empty.rs:14`）+ `.title(EmptyTitle)`（`:114`）+ `.description(EmptyDescription)`（`:120`）+ `EmptyMedia`（`:167`） | **可一比一** |
| 页签溢出「更多」菜单 | `TabBar::menu(true)`（`tab_bar.rs:110-113`）+ `TabBar::max_width(..)`（`:118`）+ `Track_scroll(&ScrollHandle)`（`:127`）+ `last_empty_space(..)`（`:163`） | **可一比一** |
| 终端搜索浮层（查找/上一个/下一个/大小写/全字/正则/计数） | 无 `SearchInput` 类型；用 `gpui_kit::component::input::Input`（`input/input.rs:110`）+ `InputState` + `.cleanable(bool)`（`input/input.rs:130` 附近）；或 `ListState::searchable(true)`（`list/list.rs:125`）+`search_placeholder`（`list/list.rs:747`）。选项开关（大小写/全字/正则）用 `Toggle`/`ToggleGroup`（`gpui-component-0.6.6/src/button/toggle.rs:34/222`）自组 | **需自行组合** |
| 终端右键菜单（复制所选内容 / 粘贴） | `ContextMenuExt::context_menu(..)`（`menu/context_menu.rs:19`）+ `PopupMenuItem::{disabled, on_click}` | **可一比一** |
| 终端错误兜底页 | `Empty`（`empty.rs:14`）+ `EmptyTitle`/`EmptyDescription` + `Button`「重试」 | **可一比一** |

### 7.4 运行 / 调试 / 问题面板

| 元素 | gpui-kit 0.6.6 建议 | 判定 |
| --- | --- | --- |
| **工具窗容器（底部、可拖动改高 200px–80vh、可全屏）** | 首选整块 dock：`gpui_kit::component::dock::DockSkin::dock_area(..)`（`gpui-component-0.6.6/src/dock/mod.rs:132`）+ `base::DockArea::set_dock(DockPlacement::Bottom, ..)`（`gpui-base-0.6.6/src/dock/dock_area.rs:246`）+ `DockLayout::tabs()`（`dock/layout/builder.rs:51`）+ `DockPlacement::Bottom`（`dock/state.rs:160`）+ `set_dock_collapsible`（`:330`）/`set_dock_size`（`:348`）/`toggle_dock`（`:301`）/`set_zoomed_in`（`:626`，对应「全屏」）。**dock 无 feature 门控**（`gpui-component-0.6.6/src/lib.rs:43` 是 `pub mod dock;`；`Cargo.toml` 的 `[features]` 无 `dock`） | **可一比一**（建议直接采用 dock） |
| 若不使用 dock，仅做「上编辑区 + 下工具窗」两段 | `gpui_kit::component::{v_resizable, resizable_panel, ResizableState, ResizablePanel, ResizablePanelEvent}`（`gpui-base-0.6.6/src/resizable/mod.rs:17/22/27/32`、`panel.rs:16/237`）+ `.size_range(range)`（`panel.rs:283`）+ `.on_resize(..)`（`panel.rs:111`）。`PANEL_MIN_SIZE = px(100)`（`resizable/mod.rs:14`） | **可一比一**（但需自己实现「拖动条 = 4px、hover 变主色」视觉） |
| 工具窗内面板头（28px / 36px 高、标题 + 右侧图标按钮组） | 用 dock 的 `Panel::title(..)`（`gpui-component-0.6.6/src/dock/panel.rs:82`）+ `Panel::toolbar_buttons() -> Option<Vec<Button>>`（`dock/panel.rs:101`）+ `Panel::title_style()`（`:87`）/`title_suffix()`（`:92`）+ `PanelControl`（`:45`）。**注意 dock 面板标题条高度硬编码 `px(30.)`**（`dock/tab_panel.rs:372`） | **可一比一**（高度 30px 与 Windows 端 28/36px 不一致，需自定义 skin） |
| 运行配置列表（多分组、可折叠、每行图标按钮） | 多分组：`gpui_kit::component::list::ListDelegate`（`gpui-component-0.6.6/src/list/delegate.rs:10`）的 `sections_count`（`:27`）/`items_count`（`:35`）/`render_section_header`（`:52`）/`render_section_footer`（`:64`）+ `List`/`ListState`（`list/list.rs:720/70`）+ `ListItem`（`list/list_item.rs:25`）。行高由 item 自身 `.h(px(26.))` 决定（`ListItem` 靠 `Styled`，`list_item.rs:153-157`。**约束：所有 item 必须同高**，见 `list/delegate.rs:41`）。分组头带计数 chip → `Badge`/`Tag` | **可一比一**（需自行处理组头折叠状态与「组头也是 item」的同高约束） |
| 运行输出（可换行、等宽 12px、SGR 着色、千行级） | 无现成 console viewer（`LogView`/`Console`/`OutputView` 均**未找到**）。自建方案：`gpui_base::{VirtualList, v_virtual_list}`（`gpui-base-0.6.6/src/virtual_list.rs:215/139`，需自备 `Rc<Vec<Size<Pixels>>>` 行高）+ `gpui_base::Scrollbar::vertical(&VirtualListScrollHandle)`（`gpui-base-0.6.6/src/scrollbar.rs`，`list/list.rs:603` 是同款用法）；需要文本选择/复制时叠加 `gpui_base::text::TextView`（`gpui-base-0.6.6/src/text/text_view.rs:117`）。等宽字体取 `cx.theme().mono_font_family`（`gpui-component-0.6.6/src/theme/mod.rs:141`）。ANSI SGR 解析仍需自研（见 §7.2） | **gpui-kit 没有现成组件（虚拟列表 + 自研 span 渲染可拼出）** |
| 输出自动滚到底开关 | `VirtualListScrollHandle::scroll_to_bottom()`（`gpui-base-0.6.6/src/virtual_list.rs:123`）+ `gpui_base::AutoScroll`（`gpui-base-0.6.6/src/auto_scroll.rs`）；或 `gpui_kit::component::message_scroller`（`gpui-component-0.6.6/src/message_scroller.rs`，自带 jump-to-bottom） | **需自行组合** |
| stdin 输入行 + 发送按钮 | `gpui_kit::component::input::Input`（`input/input.rs:110`）+ `InputState`；发送按钮 `Button`（`button/button.rs:185`）+ `.primary()` | **可一比一** |
| 运行/调试头部工具行（Play/Continue/Pause/Stop/Step） | `Button` + `.icon(..)`（`button/button.rs:383`）+ `.tooltip(..)`（`:389`）+ `.disabled(..)`（`:503`）+ `.ghost()`（`:78`）；分组可用 `ButtonGroup`（`button/button_group.rs:18`） | **可一比一** |
| 调试状态徽标（空闲/运行中/已暂停/已退出/已停止/失败） | `gpui_kit::component::badge::Badge`（`gpui-component-0.6.6/src/badge.rs:30`）`.color(impl Into<Hsla>)`（`:82`）；或 `gpui_kit::component::tag::Tag`（`tag.rs:124`）+ `TagVariant`（`tag.rs:9`：Primary/Secondary/Danger/Success/Warning/Info/Color/Custom） | **可一比一** |
| 调试面板卡片（圆角 12px 边框、可折叠 section、右上角计数徽标） | 卡片：`div().rounded(cx.theme().radius_lg)`（`theme/mod.rs:147`）+ `border_1()` + `bg(cx.theme().tokens.list)`；折叠：`gpui_kit::component::collapsible`（`gpui-component-0.6.6/src/collapsible.rs`）或 `accordion`（`accordion.rs`）；计数：`Badge::new().count(n)`（`badge.rs:64`，count==0 自动隐藏，`badge.rs:104`） | **可一比一** |
| 调试左右分栏（260–320px / 1fr） | `gpui_kit::component::{h_resizable, resizable_panel}`（`gpui-base-0.6.6/src/resizable/mod.rs:17/27`）+ `.size(px(280.))`（`panel.rs:275`）+ `.size_range(px(260.)..px(320.))`（`panel.rs:283`）；`ResizableState` 持久化 | **可一比一** |
| 调试页签（线程和变量 / 控制台，32px 高 + 2px 下边框） | `TabBar` + `TabVariant::Underline`（`tab/tab.rs:13`，Medium+Underline 默认 **36px**，见 `tab/tab.rs:24-43`）；要 32px 需 `.h(px(32.))` 覆盖 + 自行处理 `inner_height`（`tab/tab.rs:45-69`） | **可一比一**（高度需覆盖） |
| 控制台逐行输出（stderr 红 / stdout 灰） | 同「运行输出」，或每行 `v_flex` + `text_color(cx.theme().danger)` / `text_color(cx.theme().muted_foreground)`（token 见 `theme/theme_color.rs:147/195`） | **需自行组合** |
| 问题列表（分组卡片 + 行：严重度图标 11px + 消息 + 位置 chip + 文件路径 + 来源/代码 chip） | `List` + `ListDelegate` 多 section（组=section，`delegate.rs:27-64`）+ `ListItem`（`list/list_item.rs:25`）+ `Badge`/`Tag` 做 chip。严重度图标用 `gpui_kit::component::icon::Icon`（`icon.rs:84`）+ `IconName`（`icon.rs:18`），颜色用 `cx.theme().danger/success/warning/info`（`theme_color.rs:147/249/301/165`） | **可一比一** |
| 问题筛选下拉菜单（分组/排序三选一 + 三个严重度开关 + 仅当前文件 + 重置） | `PopupMenu`（`popup_menu.rs:281`）+ `PopupMenuItem::{new, label, separator, checked, disabled, on_click}`（`:71/119/113/180/161/196`） | **可一比一** |
| 问题搜索浮层（`SearchPopover`，Esc 先清空再关闭） | `Input` + `InputState`；Popover 容器 `gpui_kit::component::popover::Popover`（`gpui-component-0.6.6/src/popover.rs:105`）或 `gpui_base::popup::Popup`（`gpui-base-0.6.6/src/popup.rs:28`） | **可一比一** |
| 问题/输出面板滚动区 | `gpui_kit::component::scroll::ScrollableElement::{overflow_y_scrollbar, overflow_scrollbar}`（`scroll/scrollable.rs:60/46`）+ `Scrollbar::vertical(..)`（`gpui-base-0.6.6/src/scrollbar.rs:815`）；`ScrollbarState` **不公开**（`scrollbar.rs:149` 私有，未 re-export） | **可一比一**（用 `ScrollableElement`，不要找 `ScrollbarState`） |
| 工具窗全屏 | `DockArea::set_zoomed_in(NodeId, ..)`（`gpui-base-0.6.6/src/dock/dock_area.rs:626`）/`set_zoomed_out`（`:635`）/`is_zoomed`（`:639`） | **可一比一** |
| 底部状态栏（若把项目准备状态/调试状态放状态栏） | `gpui_kit::component::status_bar::StatusBar`（`gpui-component-0.6.6/src/status_bar.rs:31`）`.left(..)`（`:51`）`.right(..)`（`:57`）；默认外观用 token `status_bar` / `status_bar_border`（`theme/theme_color.rs:297/299`） | **可一比一** |

### 7.5 明确「gpui-kit 没有」

| 能力 | 结论 | 依据 |
| --- | --- | --- |
| 终端模拟器 / PTY / VT-ANSI 解析 | **没有**，且无任何相关依赖 | 三 crate 全部 `.rs` + `Cargo.toml` 精确检索 `portable_pty\|conpty\|alacritty\|termwiz\|crossterm\|vte\|vt100\|ansi_escape\|termion\|wezterm` → **0 命中**；三个 `Cargo.toml` 的依赖清单（`gpui-component-0.6.6/Cargo.toml`、`gpui-base-0.6.6/Cargo.toml`、`gpui-kit-0.6.6/Cargo.toml`）中亦无 |
| 字符网格 / GPU 终端渲染器（等价 xterm + WebGL addon） | **没有** | 同上 |
| 终端专用 addon（fit/search/serialize/unicode11/clipboard/weblinks） | **没有** | 同上 |
| `LogView` / `Console` / `OutputView` / `LogPanel` / `ConsoleView`（日志/输出控制台 viewer） | **没有** | 三 crate `.rs` 类型名精确检索 0 命中；需用 `VirtualList`(+`TextView`) 自建 |
| `Toolbar` 独立组件 | **没有** | 只有 dock 的 `PanelControl::Toolbar`（`gpui-component-0.6.6/src/dock/panel.rs:45`）+ `Panel::toolbar_buttons()`（`dock/panel.rs:101`） |
| `Sidebar` 在 dock 内 | **没有**；`Sidebar` 是独立模块 `gpui_component::sidebar`（`gpui-component-0.6.6/src/sidebar/mod.rs:222`） | `gpui-component-0.6.6/src/sidebar/` 与 `dock/` 平级 |
| `Splitter` / `Resizable`（无后缀） | **没有**；用 `ResizablePanelGroup` / `v_resizable` / `h_resizable`（`gpui-base-0.6.6/src/resizable/mod.rs:17/22/31`）；静态分隔线用 `gpui_component::separator::Separator`（`separator.rs:16`） | 名称检索 0 命中 |
| `IconButton` / `ButtonSize` / `ButtonIcon`（公开） | **没有**；尺寸统一用 `Size`（`gpui-component-0.6.6/src/sizing.rs:6`）+ `Sizable`（`sizing.rs:177`）；`ButtonIcon` 是 `pub(crate)`（`button/mod.rs:9`） | 名称检索 0 命中 |
| `CountBadge` / `Indicator` | **没有**；用 `Badge::new().count(n)`（`badge.rs:64`） | 名称检索 0 命中 |
| `Modal` / `SearchInput` / `TextInput` / `TextSize` | **没有**；分别用 `Dialog`+`WindowExt::open_dialog`（`window_ext.rs:30`）、`Input`+`InputState`、`text_xs/sm/base/lg`（gpui `Styled`，`gpui-pre-0.3.6/src/styled.rs:545/552/559/566`） | 名称检索 0 命中 |
| `ScrollbarState` | **存在但私有**（`gpui-base-0.6.6/src/scrollbar.rs:149`，未 re-export） | `gpui-base-0.6.6/src/lib.rs:154-157` 的 re-export 列表不含它 |
| Tabs 垂直方向 / 内建关闭按钮 / tab 拖拽换序 | **`TabBar`/`Tab` 均没有** | 源码无 `vertical`/`orientation`/`close`/`closable`/`on_close`；拖拽只在 dock `TabGroup`（`gpui-base-0.6.6/src/dock/tab_group.rs:855-876`） |

---

## 8. 未查清

1. **gpui-kit 侧的两个坑未实测**：`List` 要求所有 item 同高（section header/footer 也算 item，因此虚拟列表**不能加 `gap_y`**，见 `gpui-component-0.6.6/src/list/list.rs:564-566` 与 `list/delegate.rs:41`）；`Tab`/`TabBar` 的尺寸档与内建 padding/height 强耦合，`.h()` 覆盖后 `TabVariant::inner_height`（`tab/tab.rs:45-69`）的固定值可能导致内容被裁 —— 均未实机验证。
2. **`debug.*` 契约与 `command.rs` 名称表不完全对齐**：`shared/contracts/rust-core-api.md:113-130` 列出的 18 个命令中，只有 `debug.launch / debug.execute / debug.inspect / debug.receive / debug.disconnect` 在 `rust/lithe-core/src/protocol/command.rs:344-361、505-520` 找到字符串。但 Windows host 确实以字符串形式调用了 `debug.createSession`（`debug.rs:238`）、`debug.setBreakpoints`（`:735`）、`debug.setVariable`（`:758`）、`debug.destroySession`（`:255`）等，说明 Core 侧另有分派入口（可能是 `arguments.command` 子分派或 `execute_json` 的另一条路径）—— **未逐行确认**。
3. **远程终端的 Rust 实现位置未定位**：`create_remote_terminal` / `remote_terminal_write` / `remote_terminal_resize` / `remote_terminal_set_paused` / `close_remote_terminal` 不在 `main.rs` 的 `generate_handler!` 列表（`windows/tauri/src-tauri/src/main.rs:112-191`），推测经 `platform::platform_invoke`（`main.rs:128`）分派，但**未读到对应实现**。
4. **`warm_terminal_environment` 前端调用点未找到**：命令已注册（`main.rs:131`），`grep warm_terminal_environment windows/tauri/src` 无结果。
5. **`lint_code` 的后端实现未定位**：前端在 `features/editor/linter/linter-service.ts:66-86` 调用 `invoke("lint_code", …)`，但 `windows/tauri/src-tauri/src/` 下无 `#[tauri::command] lint_code`，`main.rs:112-191` 未注册，`platform.rs` 的 `translate` 也未匹配 —— 该命令是否已被移除/改名**未查清**。
6. **`lsp://diagnostics` 的 Rust 发射端未定位**（前端订阅见 `features/editor/lsp/lsp-client.ts:565`）；本次未展开 `windows/tauri/src-tauri/src/lsp.rs`（1072 行）与 `rust/lithe-core` 的 LSP 事件发射路径。
7. **`ssh_read_file` 的 Windows 侧实现未定位**（`features/run-actions/utils/run-action-discovery.ts:288` 调用）；`tauri-core.ts:138-144` 把 `ssh_` 前缀归到 `remote` 能力，但未追踪其 Rust 命令定义。
8. **§5.5 的白名单不一致是否是真实缺陷未验证**：本次不运行任何构建/测试，也未找到断言「`nativeCommands` 覆盖全部已注册命令」的测试（`windows/tauri/src/platform/tauri-core.test.ts:23` 只断言了 `lsp_rebuild_java_index`）。`run_execute_prelaunch` 在 `run.store.ts:904` 的 `preLaunchSteps` 循环上真实可达，但**是否被其它路径补偿未查清**。
9. **`run-output` / `run-exit` 多窗口下的送达范围未实测**：`run.rs:1959、2008` 用 `app.emit_to(window_label, …)`（窗口级），前端用 `getCurrentWebviewWindow().listen`（`use-run-process-events.ts:19-28`）；而调试事件是**全局** `listen`（`debug-adapter-service.ts:148-156`）。多窗口场景下两者范围不同，静态确认但未运行时验证。
10. **`RunOutputText` 的 ANSI 覆盖度未与 xterm 对齐**：`run-output-style.ts` 只处理 SGR（0/1/22/30-37/40-47/39/49/90-97/100-107/38/48，含 256 色与真彩）与 OSC 跳过，**不处理光标控制、擦除、备用屏幕**；若 maven/Java 输出依赖这些序列，Windows 端当前行为与 xterm 不一致（未实测）。运行输出**没有换行开关**（`run-output-text.tsx:39、48` 固定 `whitespace-pre-wrap`，`run-preferences.store.ts:9-20` 也无 wrap 字段）。
11. **终端 profile 自定义存储格式（`profiles.store.ts`）未逐行读取**：`terminal-profile` 的字段（`startupCommands`、`startupDirectory`、`icon`）只从 `terminal-profiles.ts:19-30、77-80` 间接推断。
12. **问题面板的行高未实测**：§3.7 给出的是 padding（`px-2 py-1.5`）与字号；**行本身没有固定高/最小高**（`diagnostics-pane.tsx:1122` 无 `h-*`/`min-h-*`），实际行高取决于单行/双行与 `wrapMessages`，未渲染测量。
13. **`TerminalTabBar` 在 `widthMode` 下的宽度差异**：`widthMode: "full" | "editor"` 由 `TerminalStore` 持有（`terminal.store.ts:5、29`），但 `main-layout.tsx` 的消费点未逐行核对。
14. **`FileNavigatorSidebar` / `ScrollArea` / `Dropdown` / `Tooltip` / `Empty` 等 UI 原语的内部类未展开**：本次只读了 `ui/button.tsx`、`ui/badge.tsx`、`ui/tabs.tsx`、`ui/chrome.tsx`、`ui/tab-bar.tsx`、`features/panes/components/pane-chrome.tsx`，问题面板左栏（`features/diagnostics/components/diagnostics-pane.tsx:1034` → `features/file-explorer/components/file-navigator-sidebar.tsx`）的高度与内边距未取。
15. **`en-US` 侧 `footer.*` / `ui.*` / `workbench.*` / `maven.*` / `settings.project.*` 的英文原文未逐一提取**：§6 表里中文已全，英文只在 `run.* / runActions.* / debugger.* / diagnostics.* / preparation.*` 上做了对照。
