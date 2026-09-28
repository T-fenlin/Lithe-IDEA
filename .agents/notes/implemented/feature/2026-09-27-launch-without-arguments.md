# Agent 笔记：双击 `Lithe.exe` 也能开出窗口（无位置参数启动）

状态：已实现

## 先说结论

在资源管理器里双击 `rust/lithe-db-gpui/target/release/Lithe.exe` 现在一定会开出一个窗口：
**先打开「最近项目」里最近且仍然存在的那个项目**；最近项目列表为空、或者里面每一条都已失效时，
**回落到当前工作目录**（双击时就是 exe 所在目录），窗口照样开出来，用户随即能用项目菜单换根。

开发者要记住三件事：**`Lithe <root>` 的既有行为一字不改**（显式给了路径就用它，连存在性都不检查）；
**"没有最近项目"不是错误路径，不许退出、不许只弹一个提示框**；**选根逻辑是纯函数，放在
`lithe-db-gpui-settings` 里单测，App Shell 只负责"读文件 + 调它 + 建窗口"**。

## 问题

用户双击 `Lithe.exe`、点掉 SmartScreen 的「仍要运行」之后**毫无反应**。父代理实测：

```text
原地无参数启动 →  HasExited = True    ExitCode = 2
stderr         →  一整段 USAGE 用法说明
带路径启动     →  HasExited = False   MainWindowHandle = 2034844（窗口正常）
```

根因是一条**必填参数 + 没有控制台**的组合：

1. `rust/lithe-db-gpui/crates/app/src/main.rs` 里位置参数是必填的：
   `root: root.ok_or_else(|| USAGE.to_string())?`（当时 `Options::root` 是 `PathBuf`）；
2. 解析失败由 `main` 统一处理：`eprintln!("{message}")` 之后 `std::process::exit(2)`；
3. 而 release 是 **GUI 子系统**（`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`），
   **没有控制台** —— 用法说明写进 stderr 之后没有任何接收者，Explorer 双击时也没有谁去重定向它。

于是"打印用法 + 退出码 2"这套本来正确的命令行行为，在双击场景里退化成**静默什么都不发生**，
与仓库那条"失败一定要看得见"的口径（`changes_view` 那一套）正好相反。这也是这个坑值得写下来的
原因：**GUI 子系统的启动路径上，任何"打印一行就退出"都是静默失败**。

## 决策

### 一、无位置参数时打开「最近的项目」，数据来自项目菜单那份 `recent-projects.json`

选择逻辑抽成**纯函数**，放在数据层（`rust/lithe-db-gpui/crates/settings/src/recent_projects.rs`）：

```rust
// 文件系统探测由调用方注入，所以这条逻辑可以在不构造 GPUI、不真的建/删目录的情况下单测。
pub fn select_launch_root(&self, is_dir: impl FnMut(&str) -> bool) -> LaunchRootPick
```

- 顺序 = **界面顺序**（`pinned` 优先，再 `lastOpenedAt` 降序），取**第一条真的存在**的条目。
  有置顶项且它还在时优先打开置顶的那个 —— 用户明确钉住过的项目，比"最近"更能代表他想打开的。
- 扫描**不提前退出**：选中之后剩下的条目照样探测一遍，把失效的路径交回调用方，
  由 App Shell 逐条 `set_missing(.., true)` 并落盘 —— 与项目菜单里点一条失效入口的行为完全一致。
- 结果是纯数据 `LaunchRootPick { root, missing }`：**选根逻辑自己不写文件、不取时钟**，
  与模块既有的"数据层不取时钟"同一条分工。

App Shell（`rust/lithe-db-gpui/crates/app/src/main.rs`）只做三件事：读文件 → 调上面这个函数 → 建窗口。
优先级本身写成一个只有三行的纯函数，只有一个落点：

```rust
fn choose_launch_root(explicit, recent_root, fallback) -> PathBuf   // 显式 > 最近且存在 > 兜底
```

### 二、没有最近项目 / 全部失效时，回落到**当前工作目录**（"直接进"）

这就是被选中的兜底：不是退出、也不是只弹一个提示框。理由：

- 资源管理器双击时 **CWD 就是 exe 所在目录**，它一定是个真实存在的目录，所以外壳一定建得出来；
- 用户拿到窗口之后**立刻能用项目菜单 / `Ctrl+O` 打开真正的项目** —— 兜底把用户留在了能自救的位置；
- 从终端启动时这条兜底同样合理（CWD 就是用户当前所在的目录）；
- 万一那个目录**不可写**（例如将来装进 `C:\Program Files\…`，`.lithe/` 建不出来），
  这件事会走已经落地的**常驻红条**路径：失败仍然是可见的，不是静默。

CWD 探测失败（目录被删掉）时退到 **exe 所在目录**；连它都拿不到时才用 `.`。
**这条启动路径上没有任何 `exit`** —— 宁可带着一个可疑的根把窗口开出来、让外壳自己的诊断说话。

### 三、`Lithe <root>` 一字不改，`--help` / 未知参数照旧退 2 并打用法

- 显式路径那一支**根本不读** `recent-projects.json`（不产生新诊断行，也不会去动那份状态文件），
  也不做"路径是否存在"的检查 —— 与改动前逐字一致。
- `--help` / 未知参数仍然返回 `Err(USAGE)`，`main` 照旧打到 stderr 并以退出码 2 退出。
- 用法首行由 `Lithe <workspace-root>` 改成 `Lithe [<workspace-root>]`，并补了两句说明
  （省略时打开最近项目 / 一个都没有时用当前工作目录）。**这段文本仍然只在终端可见**，
  所以它不是本缺陷的修复手段，只是让 `--help` 与真实行为一致。

### 四、启动证据走 stderr：`S1_WORKSPACE_LAUNCH …`

```text
S1_WORKSPACE_LAUNCH source=recent root=C:\...\proj-a candidates=2 missing=1
S1_WORKSPACE_LAUNCH recent_marked_missing=1 saved bytes=374 path=C:\...\recent-projects.json
S1_WORKSPACE_LAUNCH source=current_dir root=C:\...\cwd-fallback candidates=0 missing=0
```

前两个字段说明"开的是哪个根、为什么是它"，`candidates` / `missing` 说明列表当时的样子。
必须走 **stderr**（`eprintln!`）：GUI 子系统没有控制台，验证时要靠
`Start-Process -RedirectStandardError` 才看得到；而 stdout 在被重定向时是**块缓冲**的，
进程还活着（窗口开着）的时候它可能一行都没落盘。验证脚本要 grep 就 grep 这一行。

## 考虑过的备选方案

| 方案 | 为什么没选 |
| --- | --- |
| **弹一个真正的消息框（`MessageBoxW`）然后退出** | 那只是把"静默"换成"点一下确认"：点完还是什么都开不出来，用户仍然到不了任何工作区，与"双击之后窗口一定会开出来"直接冲突。而且 GUI 子系统弹框要加平台调用，`app` crate 虽然是平台宿主层、放得下，但为一个只起提示作用的兜底引新依赖不划算。 |
| **写一份启动日志文件再退出** | 日志没人主动去看就等于静默；用户依然开不出窗口。 |
| **让外壳支持"没有根"（欢迎页 / 空态窗口）** | 这是更贴近"直接进"的形态，但今天 `ShellWorkspace::new` 以及它下面的资源树、变更视图、会话、工作区配置、分支读取全都假设有一个根；维护者已把它归到 B 方案（更大的改动），本批不做。 |
| **回落到"上次会话的根"** | 那是第三份状态，而且它同样可能已经失效 —— 等于把同一个失败模式搬到另一个文件里。 |
| **回落到当前工作目录** | 选中（见决策二）。 |
| **无参数时直接弹"打开文件夹"选择器** | 循环依赖：`open_dialog` 只能在窗口根 `Root` 就位之后调用，而建窗口又需要一个根。要绕开就必须先造一个假根 —— 那还不如直接回落到 CWD。 |

## 后果

- **收益**：双击可用；顺手把已失效的最近条目标成 `missing`（与项目菜单一致）；启动选的哪个根
  有一行可 grep 的诊断，无人值守验证也能取证。
- **代价**：没有项目时用户看到的根是 exe 所在目录（或 CWD），可能一时困惑"为什么这里是我的工作区"；
  缓解手段是那行诊断 + 菜单里随时换根，以及 CWD 不可写时的常驻红条。
- **代价**：`Options::root` 变成 `Option<PathBuf>`，"显式给了路径"与"要走启动策略"这两条路
  从此在类型上就分开了（这是有意的：避免把 `None` 当成一个空的根到处传）。
- **已知没做**：外壳仍然要求一个根；没有欢迎页 / 最近项目落地页；没有把"这是兜底工作区"
  显示在界面上（只有诊断行）。

## 验证

- `cargo test --workspace`：**432 通过 / 0 失败**（本批之前的基线是 423，本批新增 9 条：
  `lithe-db-gpui-settings` 5 条守纯函数，`lithe-db-gpui-app` 4 条守优先级与兜底目录）。
- `./.agents/skills/write-stable-tests/scripts/verify-test-stability.ps1`：通过。
- `node rust/lithe-db-gpui/tools/extract-locale.mjs --check`：通过（本批没有新增界面文案，
  `S1_WORKSPACE_LAUNCH` 是诊断行、不是 `tr` 文案）。
- `node scripts/verify-agent-notes.mjs`：通过。
- GUI 端到端（工作区**外**的 exe 副本 + `LITHE_GPUI_SETTINGS_FILE` 指到工作区外的配置目录，
  三种情形各跑一次）：断言与观察输出记在 `rust/lithe-db-gpui/HANDOFF.md` 的本批小节里。
- **release 二进制复验**（由父代理在本批提交后重跑，取代"release 未复验"这条缺口）：
  `cargo build --release` exit=0（增量 3m42s）；**PE 子系统位 = 2（GUI，无控制台）**；
  用 release 副本按**双击的等价条件（不带任何位置参数）**启动：
  - 手写最近列表两条、其中最新的那条目录已删 → `S1_SETTINGS_RECENT loaded count=2 fileExisted=true`、
    `S1_WORKSPACE_LAUNCH recent_marked_missing=1 saved bytes=386`、
    `S1_WORKSPACE_LAUNCH source=recent root=…\proj-b candidates=2 missing=1`、
    窗口句柄非 0（`HasExited=False`）、`proj-b` 里建出 `.lithe/project.json`、
    磁盘上失效那条被标 `"missing": true`；
  - 最近列表**不存在** → `source=current_dir root=<CWD>`、窗口句柄非 0、
    **CWD 里建出 `.lithe/project.json`** —— 也就是"直接进"这一支在 release 上确实可见地发生了，
    不是静默退出。
  - ⚠️ 一个排查教训：早先一次实验里 `.lithe` 看似没建出，原因是**在窗口出现后立刻杀进程**——
    工作区配置那一步是后台任务。判这条要么等文件出现（有界轮询），要么多等几秒再杀。

## 适用范围

- `rust/lithe-db-gpui/crates/app/src/main.rs` —— 启动根策略（`resolve_launch_root` / `choose_launch_root` /
  `fallback_directory`）、`Options::root` 的类型、用法文本、`S1_WORKSPACE_LAUNCH` 诊断。
- `rust/lithe-db-gpui/crates/settings/src/recent_projects.rs` —— `LaunchRootPick` 与
  `RecentProjects::select_launch_root`；最近项目文件的读写口径不变。
- `rust/lithe-db-gpui/crates/workbench/src/workspace.rs` —— 最近项目的消费方：外壳仍然**要求**一个根，
  项目菜单的换根与 `set_missing` 行为是本次选择逻辑的对齐目标。
- `rust/lithe-db-gpui/crates/shared/src/i18n.rs` —— 模块文档里引用 CLI 用法首行的那句话（已同步）。
