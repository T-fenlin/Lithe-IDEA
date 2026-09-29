# Agent 笔记：运行面板与调试按钮（谁拼命令行、谁起进程）

状态：已实现

## 先说结论

底部工具窗的「运行」页与标题栏的 `▶` / `🐞` 现在**真的会起进程**：选一条配置 → Core 给出
平台无关的启动计划 → 宿主把它落成一条真命令行 → 起子进程 → 输出与控制台、退出码都在面板里。

三件事以后要记住：

1. **Core 只返回计划，宿主拼命令行**。计划里的工具链是**引用**（`{ toolchain: "project-jdk" }`），
   参数数组里的 classpath 是**结构化数组** —— 因为路径分隔符是平台相关的，Core 明说它不拼。
2. **宿主必须把 `runConfig.generate` 返回的文档写进 `.lithe/run/generated.json`**。
   `runConfig.createLaunchPlan` 内部会自己再 resolve 一遍，而 resolve 的第一步就是读那份文件；
   不写它，界面上列表是好的、一点运行就报 `workspace_not_found`（本批实测踩到）。
3. **`🐞` 目前只是 JWDP**：JVM 以 `-agentlib:jdwp=…,suspend=y` 起来等外部调试器接入，
   **没有** DAP 会话、没有断点 / 单步 / 变量。别把"调试按钮能用了"读成"调试器做了"。

## 问题

gpui 前端此前**没有运行宿主**：`BottomPaneKind::Run` 是 `placeholder("… 工具窗（未实现）")`，
运行菜单三条全是占位项，标题栏那三个控件一个都不响应，设置「运行配置」页也如实写着
"点击启动尚未接入"。而 Core 那一侧的 `runConfig.*`（识别、分层合并、启动计划）与
`debug.*`（DAP 一整族）都是齐的 —— 缺的是**把计划变成进程**的那一层。

这一层不能塞进 Core：它要平台路径分隔符、要"这台机器上的 JDK / Maven 在哪"、要起进程。
也不能塞进设置 crate：那是界面层。

## 决策

### 1. 新 crate `lithe-gpui-run`，模型从设置搬过来（一份模型两处用）

运行配置的模型与取数（`load` / `parse_generate` / `RunConfigurationView`）原本长在
`settings/src/run.rs` 里。运行工具窗要用同一份清单，所以整体搬进 `lithe-gpui-run`，
设置页改成重导出（`settings -> run -> shared`；`run` 不认识设置）。

好处是**一处解析、一处兜底口径**：不会出现"设置页说 3 条、运行面板说 2 条"。
代价是设置 crate 多了一条依赖边 —— 值这个价。

### 2. 计划 → 命令行：四件事都由宿主补（`plan.rs`，纯函数 + 单测）

| Core 给 | 宿主补 |
| --- | --- |
| `executable: { toolchain: "project-jdk" }` / `{ command: "npm" }` | 工具链引用 → 真实可执行文件（JDK 取 `<home>/bin/java`，`tool: "javac"` 取兄弟程序；`{command}` 走 `PATH`） |
| `classpath[]` / `modulepath[]` | 用**平台分隔符**（Windows `;`、其它 `:`）拼起来，前置 `-cp` / `--module-path` —— 全仓库只有这一处判断分隔符 |
| `environment`（工具链引用）/ `env`（字面量） | 解析成真值（`property: "home"` → 工具链根）；不认识的 `property` **报错**而不是忽略 |
| `preLaunchSteps[]`（独立 Java 的 `javac`） | 按序跑完再跑主进程；非零退出中止整次运行并把 stderr 原文带出来 |

`-m module/name.Type` 是 **Core** 推的（`java` CLI 的模块启动参数），宿主只搬运 —— 别在
宿主里再判一次"是不是模块类"。

### 3. 「工具链在哪」走钩子 + 懒探测

`run` 只依赖 `shared`，拿不到 JDK / Maven 的位置；那是平台适配器的事实。所以按仓库既有的
"低层 crate 定义钩子、外壳填实现"口径（与 `TabMenuActions` / `GitIdentityHost` 同形）：
外壳在 `ShellWorkspace::new` 里登记 [`ToolchainResolver`]，实现复用**设置页同一个**
`settings::project::discover` —— 于是"运行用哪个 JDK"与设置页显示的那个值必然一致。

探测**懒做**：`discover` 要跑 `java -version` / `mvn -version`，所以只在真的点 `▶` / `🐞` 时算，
不跟着 `register_java_toolchain` 一起在启动路径上付这笔钱。

### 4. 落盘 `.lithe/run/generated.json` 是宿主的事

`runConfig.generate` 的响应里带着完整的 `generated` 文档，契约写的是"由平台适配器决定要不要
原子落盘"。**必须落**，因为 `createLaunchPlan` → `resolve` → 读 `.lithe/run/generated.json`。
写法走共享原语（`shared::document::save_document`：原子写 + 保留未知键），路径走共享的路径真源
（`WorkspaceConfigPaths::run_generated()`），不在这里拼字符串。

只写 generated，**不写** `toolchainRequirements`：那是工具链诊断的旁路文档，写它会让 resolve
开始按需求清单校验（而候选清单是空的），反而可能把配置标成"缺工具链"。

### 5. 进程宿主归 feature crate（照终端的形状）

`std::process::Command` + 管道 + 每条管道一条读线程 + `mpsc` + 前台任务
`background_spawn(blocking_recv)` 的泵 —— 与 `lithe-gpui/crates/terminal/src/session.rs` 同形
（那套已经被真机跑过）。两条差异是语义的：**没有 stdin**（运行配置不是交互式 shell）、
输出是"一整份文本 + 有界行数"（最后 5000 行，超了在面板与日志里都如实标注）。

生命周期硬要求与终端一致：`kill()` 之后**必须** `wait()`，否则留僵尸进程。

### 6. `🐞` = JWDP，状态按**计划**判定而不是按意图

`debugPort` 交给 Core，Core 对 JVM 配置前置 `-agentlib:jdwp=…,suspend=y`。状态里那一档
`WaitingForDebugger` 的判据是**计划里真的有 `-agentlib:jdwp`**，不是"用户按了 🐞"：
一条 `npm` 脚本按 🐞 起来就是普通进程，那时状态如实显示「运行中」并给一句"这条配置的启动
计划里没有调试参数"。按意图写状态就是假状态。

## 考虑过的备选方案

- **在 Core 里把计划拼成命令行**：被否。classpath 分隔符、可执行文件位置都是平台事实，
  进 Core 就等于把一个平台分支塞进"确定性命令面"，而且会让 `runConfig.createLaunchPlan`
  偷偷开始起进程（它今天是纯函数）。
- **运行面板自己找 JDK / Maven**：被否。那会和设置页的发现逻辑形成第二份真源
  （页面上显示 JDK 21、真正跑的却是另一个），而"发现"本来就有 `settings::project::discover`。
- **生成的文档留在内存里、`createLaunchPlan` 时再传进去**：被否。`resolve` 只认磁盘上的
  `.lithe/run/generated.json`（它同时服务"另一个进程读这份配置"的场景），Core 没有"直接传文档"
  的入口；照它的契约落盘才是唯一一条路。
- **把运行输出接进终端面板复用**：被否。终端要按 ANSI 解析成网格、要输入；运行要的是纯文本
  与退出码，合并会让两条链互相牵扯（`terminal` 的 `TerminalText` 对运行输出是多余的复杂度）。
- **一次跑多条配置 + 输出页签切换**：本批没做（真机可以）。一次一条的实现更短、状态更少，
  而且"同时跑三条 `spring-boot:run`"在当前产品阶段不是高频路径。

## 后果

收益：

- 点 `▶` 真的起进程：`S1_RUN_START … command=…` 一行能看出**到底跑了什么**，
  `S1_RUN_EXIT … code=0 lines=5` 一行能看出结果，不必靠截图。
- 工具链来源与设置页同源：设置里换了 JDK，运行也跟着换（懒探测，每次启动前重算）。
- 启动计划里的 `preLaunchSteps` 一并实现 —— 独立 Java（`java.main` / `java.current-file`）
  的"先 `javac` 再 `java`"那一档因此可用，而不是报一个我们没实现的错。

代价与例外：

- **打开项目就会写 `.lithe/run/generated.json`**（面板第一次识别配置时）。那是生成物，
  内容是 Core 的确定性 output；用户手改它会在下一次识别时被覆盖。
- 一次只跑一条配置，且**没有**配置编辑器：改参数 / 环境变量 / JVM 参数要手改
  `.lithe/run/*.json`（Core 的 `runConfig.updateOptions` / `createUserConfiguration`
  因此仍无消费方）。
- `🐞` 没有断点 / 单步 / 变量 / 调用栈，也**不接** Java Debug Server 的 DAP：
  这一档要 `vscode.java.startDebugSession` 拿适配器端口 + `debug.createSession` /
  `launch` / `receive` 一族（Core 侧已齐），是独立的一批。
- 输出只留最后 5000 行：跑很久的服务日志会被截断（面板上有告示行）。

## 验证

- 单测：`cargo test -p lithe-gpui-run --lib`（16 条，含 5 条计划解析：工具链引用 → `bin/java`、
  `javac` 兄弟程序、classpath 平台分隔符、`{command}` 裸程序名、缺工具链与不支持的
  `environment.property` 的可执行报错）。另有 `-p lithe-gpui-workbench --lib`（83，含菜单
  项接线守卫）、`-p lithe-gpui-settings --lib`（169，模型搬家后不回归）、
  `-p lithe-gpui-editor --lib`（53）、`-p lithe-gpui-shared --lib`（64，i18n 接线）。
- **启动态探针**（真实进程，`--run-probe` / `--run-probe-debug`，走的是与点 `▶` / `🐞`
  完全相同的 `RunPane::launch`）：探针夹具是一个最小 Cargo 工程，日志原文：
  `S1_RUN_GENERATED path=…\.lithe\run\generated.json bytes=792 configs=1` →
  `S1_RUN_CONFIGS configs=1 selected=0` →
  `S1_RUN_START id=cargo.binary:… pid=24892 jdwp=none command=cargo run --bin run-probe-fixture` →
  `S1_RUN_EXIT id=cargo.binary:… code=0 lines=5 truncated=false`。
  调试那一路：`S1_RUN_PLAN … debug_port=40166` + `S1_RUN_START … jdwp=none`（cargo 配置没有
  JVM 调试参数，状态因此停在「运行中」并显示"没有调试参数"那句 —— 这正是设计要的诚实行为）。
  过程记录在 `.artifacts/probe/NOTES.md`。
- **未验证**（如实登记）：① JVM 配置（`java.main` / `spring-boot.maven`）的完整启动 ——
  要在真机上有 JDK + JDT 索引 + 一个能跑的主类，本批只覆盖到"工具链引用解析成
  `<home>/bin/java`"这一层（单测）；② `🐞` 的 JWDP 真机接入（用一个外部 JDWP 客户端连上
  `127.0.0.1:<port>`）；③ 面板上的点击（注入不可靠，因此用启动态探针代替）。
  接手者按 `S1_RUN_*` 那几行复核即可。

## 适用范围

- `rust/lithe-gpui/crates/run/`（模型 / 计划 / 会话 / 面板四个模块）
- `rust/lithe-gpui/crates/workbench/src/workspace.rs`（底部工具窗与工具链钩子登记）、
  `title_bar.rs`（`运行配置 ▾` / `▶` / `🐞`）、`menu_bar.rs`（运行菜单的开始 / 停止调试）
- `rust/lithe-gpui/crates/settings/src/run.rs`（模型重导出）、
  `rust/lithe-gpui/crates/app/src/main.rs`（`--run-probe` / `--run-probe-debug` 验证开关）
- 契约：`shared/contracts/rust-core-api.md` 的 `runConfig.*` 一段（Core 只返回计划、
  工具链引用由宿主解析）
- 相关笔记：
  `.agents/notes/implemented/architecture/2026-09-13-language-tooling-and-lsp-runtime-ownership.md`（LSP / 进程归属的同一套口径）、
  `.agents/notes/implemented/architecture/2026-09-18-java-project-build-and-launch-boundary.md`
