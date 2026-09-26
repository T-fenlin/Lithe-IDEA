# GPUI 重写：会话交接提示词（HANDOFF）

> 用法：新会话开场把本文件路径给代理即可，例如
> 「读 `gpui/HANDOFF.md` 与 `gpui/PLAN.md`，按里面的队列继续做源代码管理之后的部分；
> 先跑一遍当前状态核查（`git log`/`git status`/构建），再动手。」
> 本文件是**操作手册**（怎么做、坑在哪、状态在哪）；设计取舍的长期真源是 `PLAN.md` 与
> `research/**`，不要重复记。

---

## 0. 一句话现状（2026-09-25 会话结束时）

> ## 🔴 优先级变更（维护者 2026-09-25 明确）
>
> **Java 生态是第一优先级**：编辑器**智能提示（代码补全）** + **代码跳转** →
> **Maven / Spring / Spring Boot**；**用户体验高于一切**。
> **Git 相关的先不做**（设置「Git」页的提交身份已经够用，别再往里加东西）；
> 设置剩余页里与 Java 无关的项（项目/运行配置页、日志、更新）**降到 Java 线之后**。
> 调研产物：`gpui/research/editor-lsp-completion.md`（补全怎么接）与
> `gpui/research/java-spring-maven-inventory.md`（上游载荷里到底有什么、Maven/Spring 怎么走）。

### 🔵 Java 线进度（2026-09-26 会话，11 批已提交，工作区干净）

| 方向 | 状态 | 证据 / 落点 |
| --- | --- | --- |
| **智能提示（补全）** | ✅ **端到端可用** | `c97af956` + `ffa095f2`：补全菜单**一行没写**（上游 `gpui-base` 自带），只写了一个 `CompletionProvider` 适配器（`crates/editor/src/completion.rs`）；JDTLS 优先、`lsp.builtinCompletions` 兜底；snippet 用 Core 的 `lsp.plainSnippet` 还原；**交互级已验**：菜单真的弹出（`S1_JAVA_COMPLETION source=jdtls items=10`）+ 键盘接受真插入。见 `PLAN.md` §16 |
| **代码跳转** | ✅ 已有 | 更早的批次：`F12` / `Ctrl+单击` / `←→` 历史 / `jdt://` 虚拟源码 |
| **Maven** | ✅ 项目模型 + 面板 | `dcb6b248`：`mavenContext` 送进 `lsp.startServer`（生成源根 / profile / settings 生效；Core 校验通过）。`6a2b653d` + `6bbae83a` + `76f25427`：右侧 Maven 面板显示 reactor 头 + 模块树 + profile + **源码根**（懒扫 + 缓存，`S1_MAVEN scan=ok`）。见 `PLAN.md` §16.6/§16.7 |
| **Spring** | ✅ 数据层 + 面板 | `c896cb84`：`crates/workbench/src/spring.rs` 把 Core 的 `spring.index`（端点 / 属性 / bean / 注入 / 诊断）解析成可渲染视图 + `S1_SPRING` + 4 条单测。`a8be039e`：右活动栏**第 4 项**「Spring」面板（端点列表 + 计数；空态 `lithe.spring.notDetected`），懒扫 + 缓存与 Maven 同口径。见 `PLAN.md` §16.8。**两条未取证**：面板内容截图（本机右栏点击双触发，需启动态探针或视图菜单项）、`endpoints` 非零（夹具缺 Spring 依赖） |
| 诊断波浪线 | ⛔ 未开工 | **卡在事件泵重构**：`Session::request`（`java/src/session.rs:302-359`）自己消费 `waitEvents` 并丢弃非本 `operationId` 的事件，再加订阅会互相偷事件；必须先改成"单一事件泵 + 按 operationId 分派"，而**补全与跳转都压在这层**（要一整轮，中途状态是坏的，别在预算不足时开工） |
| 自动补 import / `Ctrl+Space` | ⛔ 未做 | 上游 `insert_completion` **忽略 `additionalTextEdits`**（JDT 的自动 import 正靠它；Core 有 `lsp.applyTextEdits` 可自落）；上游**没有任何补全快捷键**，`Ctrl+Space` 要自定义 action |

**2026-09-26 第二轮（四路并行，全部已提交）**：`e358606f` **右栏启动态探针 `--right-view <id>`**
（`from_id` 双向一致、未知 id 只打 stderr 不改状态、两处重复懒扫合并成
`scan_right_view_if_needed`；实机 13 条判定全 OK）· `9bdb0fc7` **修 `spring.index` 少传 `paths`**
（我上一批的真 bug：Core 端点识别是**文本/正则**、与 classpath 无关；修后
`endpoints` **0 → 2**，`beans=1 values=2 paths=3`，opt-in 测试把旧 payload 钉成 0 回归）·
`929e923d` **LSP 会话改单一事件泵 + 按 operationId 分派**（`java/src/events.rs`；诊断存储 +
只读 `JavaLanguageService::diagnostics(&Path)`；真机回归 `items=10 / unclaimed=0`）·
`.artifacts/p15/spring-fixture`（Boot 3.4.5，依赖全复用缓存）。

**2026-09-26 第三批：诊断波浪线已接通（提交见 `git log`）** —— 上游**有**宿主塞入口
（`EditorState::diagnostics_mut() -> Option<&mut DiagnosticSet>`，`gpui-base-0.6.6/src/input/base/state.rs:840-847`），
所以是**接线不是自绘**：新 `editor/src/diagnostics.rs`（换算 `navigation::editor_position`、退避重取、
陈旧三层校验）+ `editor_view.rs` 四个触发点（`open` / `prepare_java` / `on_input_change` 防抖 400ms / `reload_buffer`）
+ `java/src/service.rs` 新增 `pub fn sync_document(&self, path, text)`（诊断是服务端**推送**的，
不同步正文不会重算）。实机三条红波浪线 + 悬停浮层 + 改错/撤销的 3→2→3 全程有日志与像素证据。
⚠️ 三条硬约束：`Diagnostic*` 只在 `gpui_base::input` 重导（用 `gpui_kit::base::input::…`）；
只在 `CodeEditor` 模式；`highlight_lines` 在 highlighter 为 `None` 时**提前返回**
（在算诊断样式之前）⇒ **没上色就没波浪线**。

**已全部落地（2026-09-26 这一天，全部已提交）**：
- **编辑器**：补全（JDTLS 优先 + Core 轻量兜底）· 诊断波浪线（复用上游 `DiagnosticSet`）·
  **快速修复 `Ctrl+.`**（含"自动补 import"的真实路径，源码前后对照 + `javac` exit=0）· 跳转（F12 / Ctrl+单击 / 历史 / `jdt://`）。
- **会话层**：单一事件泵 + 按 operationId 分派（诊断/请求不再互偷，`unclaimed=0`）。
- **Maven**：`mavenContext` 进 `lsp.startServer`（生成源根 / profile / settings 生效）· 右栏面板（模块树 / profile / 源码根）。
- **Spring**：`spring.index` 数据层 + 右栏第 4 项面板（**修掉了"没传 `paths` 导致 `endpoints` 恒为 0"**，0 → 2）。
- **设置**：`项目 · JDK 与 Maven` 真值页（探测 + 来源 + 覆盖 + 刷新，含 ≥21 闸门与"尚未生效"标注）·
  `LSP` 页（**一个真开关**：`autoCompletion` 门控补全触发，provider 保留）·
  `运行配置` 页（`runConfig.generate` 真数据，**没有运行按钮**，如实标注执行未接入）·
  对话框正文可滚动（`.h_full()`）· Git 身份页。
- **JDK 覆盖值真正生效**：判据顺序 = 覆盖值 → `LITHE_JDTLS_JAVA` → 捆绑 → JDTLS 自带 jre → `JAVA_HOME` →
  PATH → 常见安装根；实测 21.0.8 / 25.0.3 按设置生效，无效覆盖降级并打原因。
- **卫生**：右栏懒扫移出 MouseUp 派发栈（`cx.spawn`）· `S1_RIGHT_PANEL` 加 `seq/t_ms/坐标`（走 stderr）。

**下一批的候选（按我的建议排序，都需要维护者点头）**：
① **`autoCompletion` 的热生效未验** —— 页面上那个开关点下去走同一条链（会当场打
`S1_EDITOR_AUTO_COMPLETION`），但**没人实机点过它**；要么验一次，要么在页面上写清"需重开 Java 文件"。
② `mavenExecutablePath` / `mavenJavaHomePath` **仍无消费方**（页面已标注"尚未生效"）——
要真生效，得先有执行 Maven 的通路（`runConfig.createLaunchPlan` 是现成入口）。
③ **运行配置页的执行**：gpui 没有 run crate、Run 工具窗是占位、Run 菜单是空的 ⇒ 要画运行按钮必须先补宿主。
④ 让外壳登记一个**中性的工作区根钩子**（现在「运行配置」页借道 `GitIdentityHost::workspace_root`，
只改 `identity::host_workspace_root()` 一个函数体即可）。
⑤ 放开 `java` crate 一个约 10 行的只读访问器，把已运行 JDTLS 会话的 installation 信息搬上 LSP 页
（现在只在 `S1_JAVA_JDTLS` 日志里）。

> ✅ **2026-09-26 结案（维护者确认，务必先读）**：之前三次被记成"环境杂散点击 / 外部注入脚本"的
> 输入，**是维护者本人手动点击与打字**。所以：
>
> - **不存在"环境缺陷"，也不存在"并发注入会话在偷打窗口"**。源码级诊断那一步的结论
>   （`ClickEvent::Mouse` 只在 MouseUp 的 bubble 阶段产生一次、我们侧没有记两遍、DPI 无证据参与
>   ⇒ **不是 gpui 派发层的"一发二"缺陷**）**依然成立**，但它的原因归属要从"环境"改成
>   **"维护者在另一处真实操作"**；
> - 因此**不要**再据此绕开 GUI 验证、**不要**去"修"一个不存在的派发缺陷、也**不要**给面板加防抖；
> - 仍然值得保留的操作纪律（与缺陷无关，只是卫生）：同一时刻只跑一个注入会话；
>   优先用不依赖点击的确定性路径（启动态探针 `--right-view <id>`、菜单探针、资源管理器搜索框过滤）；
>   注入前后用 `SetCursorPos` 把真光标移离窗口（`.artifacts/right-panel/NOTES.md:30` 说
>   `-Mode move` 会挪真光标是**错的**：它只发 `WM_MOUSEMOVE`）。
>
> ⚠️ **实测教训（真实、且与本条无关）**：另一条按进程名找窗口的注入脚本
> （`.artifacts/**/inject.ps1` 的 `FindProcessWindow("Lithe")`）**确实**会把输入打到"当时在跑的那个
> Lithe 窗口"上。这不是本机的常态，但**同时跑两个验证会话时是真的**，所以上述纪律照旧。
其余按价值排序：① **右栏懒扫跑在 MouseUp 派发栈里**（`workspace.rs` 的 `scan_right_view_if_needed`
在点击回调内同步跑，Spring 会读 JAR）—— 真问题（性能），应延后出派发栈；
⚠️ 注意：**"右栏点击双触发是产品缺陷"这个说法已被源码级诊断推翻**，别再按它推理，
证据见 `gpui/research/click-double-trigger-dpi.md`（要点：`ClickEvent::Mouse` 只在 MouseUp 产生一次、
多一个 MouseUp 产生 0 次额外 click、我们侧没有记两遍、DPI 无证据参与；最可能是环境杂散点击，
其次是无障碍 `Action::Click` 自造 down+up）。面板类验证的正确姿势是**注入前后用 `SetCursorPos`
把真光标移离右栏**（`.artifacts/right-panel/NOTES.md:30` 那句是错的），**不要**改 toggle 语义或加防抖；
② 自动补 import（上游忽略 `additionalTextEdits`，Core 有 `lsp.applyTextEdits`）+ `Ctrl+Space`
（上游无补全键位）；③ 依赖 JAR 的 `spring-configuration-metadata.json`（gpui 不解析 `~/.m2` 传
`metadataRepositories`，所以 `properties` 目前只有内置 + 工作区元数据）；④ `projectPreparation`
事件里带 lifecycle / maven profile / building —— 「构建进度」的挂点，分派表加一个分支即可。

**本会话踩到、下个会话必须知道的坑**：
1. **强杀 Lithe 不会走 `lsp.stopServer`** ⇒ JDTLS 的 `java.exe` 会留在后台，
   得单独 `Stop-Process`（验证脚本的"测试后清进程"那一节要照做）。
2. `--menu-probe` 的动作 id 是 **`lithe.workbench.maven`**（不是 `lithe.menu.toggleMaven`）。
3. Maven 夹具现在是**标准布局** `src/main/java/demo/`：一旦带上 `mavenContext`，
   JDT 就按 Maven 源模型解析，放在非标准目录（如 `src/demo/`）的 `.java` **不会被解析**。
4. 实现类子代理在本会话不稳定（一个静默死掉、一个与主代理撞车）；调研类子代理可用。

- 分支 `feat/gpui-shell-rewrite`，**工作区干净**（最近两批：`dba009cc` 设置「Git」页、
  `53c77651` 交接优先级）。
- **Java 线第一批已完成并提交**（阶段 16，见 `PLAN.md` §16 与 `.artifacts/p14/NOTES.md`）：
  **编辑器智能提示（补全）落地** —— 补全菜单一行没写（上游 `gpui-base` 自带），只写了一个
  `CompletionProvider` 适配器：`crates/java` 加 `completion()`（JDTLS `textDocument/completion` +
  Core 的 `lsp.plainSnippet` 还原 snippet + stale/cancelled 翻成"空结果"），
  `crates/editor/src/completion.rs`（新）负责 JDTLS 优先 / 轻量兜底、`textEdit` 透传、
  120ms 防抖 + 代次闸门，`editor_view.rs` 在 `open()` 与 `prepare_java()` 两处装。
  **真实 JDTLS 端到端已验**：`S1_JAVA_COMPLETION … items=10 ms=152`，每条候选都有 `text_edit`、
  `insert_text` 里没有 `$`。**未验**：菜单在界面上真的弹出来（下一步第一件事）。
- **两份新调研已进仓库**（必读）：`gpui/research/editor-lsp-completion.md`、
  `gpui/research/java-spring-maven-inventory.md`（载荷里有 **m2e**、**没有 Spring**；
  Core `lsp.startServer` **已接受 `mavenContext` 而 gpui 没传**；Spring 走 Core 的 `spring.index`）。
- 最近四批（都经主代理复核 + 交互级验证后提交）：`ebdf4885` 源代码管理 →
  `46d62f41` 设置「编辑器」「终端」页（顺手修掉 `persistence.rs`「写得出、读不回」的真 bug，
  见 `PLAN.md` §14.2，**以后加设置键必读**）→ `b740bdb4` 设置左栏 11 项 + 7 个明确空态 →
  `dba009cc` 设置「Git」页（提交身份走 Core `git.repositorySetup` / `git.configureIdentity`，
  用**宿主钩子**保住依赖方向；外加 `confirmBeforeDiscard` 真开关）。
- **git 那批的一个已知边界已被主代理补验推翻**：NOTES 里写的"global 作用域下拉点不开"是**测量假象**——
  主代理实测下拉正常弹出、`run=load scope=global` + `run=save scope=global action=clear` 都成立，
  PowerShell 侧 `git config --global --get user.name` 随之消失。
- `gpui/` 已是一个能跑的前端：设置界面、编辑器（可编辑/保存/自动保存/查找替换/语法高亮/右键菜单、
  **F12/Ctrl+单击跳转**）、终端、资源管理器、Git 底部工具窗、左栏源代码管理、右侧工具窗、命令面板、
  主菜单栏、项目下拉、分支弹窗全部落地并逐项验证过。**缺的是 Java 智能提示（补全）与项目模型**。
- 验证债务：`Ctrl+,` / `Ctrl+Shift+P` 的**键盘**入口始终没能机器验证（注入侧问题：
  `SetForegroundWindow` 常被系统拒绝，键会送到别的窗口）。鼠标路径都验过。

## 1. 硬规则（这些是维护者明确要求过的，违反会被打回）

1. **cargo 只跑改动范围**：`cargo build --bin Lithe`；测试用 `cargo test -p <只动过的 crate>`，
   能按测试名过滤就加过滤（例：`cargo test -p lithe-gpui-shared every_wired_key_resolves_in_both_locales`，
   实测 2.6s）。**不要** `-p a -p b` 连带，更不要跑 workspace 全量。
2. **绝不要把 cargo / 任何原生进程的输出接进 PowerShell 管道** —— `cargo build … | Select-String`、
   `cargo test … 2>&1 | Select-Object -First N`、`| Tee-Object` **全算违规**（慢消费者会给仍在写输出的
   原生进程发停止信号，表现为"像卡了十分钟"，且 `$LASTEXITCODE` 不可信）。正确做法固定是**两步**：
   **先重定向到文件，再只对文件过滤**。
   ```powershell
   # ✅ 正确：原生进程只重定向，管道左手边是 cmdlet（Select-String 读文件）
   cd D:\developmentProjects\rust\Lithe-IDEA\gpui
   cargo build --bin Lithe *> ..\.artifacts\pN\build.log
   Write-Output "exit=$LASTEXITCODE"
   Select-String -Path ..\.artifacts\pN\build.log -Pattern "^error|error\[|^warning: unused|Finished" -Context 0,6
   ```
   ```powershell
   # 🚫 禁止：原生进程直接被接进管道（这是本仓库最常见的自伤）
   cargo build --bin Lithe 2>&1 | Select-String "error"
   cargo test -p lithe-gpui-settings | Select-Object -Last 5
   ```
   `Select-String -Path <文件>` **之后**再接 `| Select-Object -First N` 是安全的（左手边不是原生进程）；
   **写提示词给子代理时不要把这个安全用例和禁止用例写在一起** —— 实测会诱导代理把 cargo 也接进管道，
   宁可写"只跑 `Select-String`，不要接 `Select-Object`"。
   汇报里贴**日志文件里的原文 + `exit=`**，不要贴管道截断的残留。
3. **同一时刻只有一个代码写者**（`gpui/target` 是排他锁；验证脚本还会 `Get-Process Lithe | Stop-Process`
   互相杀掉对方的实例）。**文档/审计类任务可以并行**（只读源码 + 各写各的文件）。
4. **只读区**：`windows/`、`macos/`、`rust/`、仓库根的 `shared/`、`extensions/`、`Plugins/`、`.agents/`
   对实现代理是只读。**唯一例外**：为打通语法高亮，维护者已授权改 `rust/lithe-core` 的
   `tree-sitter = "0.25"→"0.26"` 与 `mybatis.rs:220` 的 `index as u32`（见 §5）。要再动这些区必须再问。
5. **子代理不要 commit / 不要 git add**；由主代理复核后提交。**主代理自己提交时也注意**：
   `git commit`（不带路径）会提交**整个索引**——本会话就因此把代理已 `git rm` 的删除、以及已暂存的
   图标资源卷进了别的提交。要么只 `git add` 需要的那几个路径，要么提交前先看 `git status --short`。
6. 文案：界面不许出现中英文字面量（注释除外），走 `lithe_gpui_shared::tr`；新键**同时**进两份 yml +
   `i18n.rs` 的 `WIRED` 清单；yml 由 `gpui/tools/extract-locale.mjs` 生成，Windows 缺的键加它的
   `GPUI_ONLY_KEYS`，**不要手改 yml**。
7. 颜色只走 `cx.theme()`；布局一律 rem 档位 helper（`p_2()`/`gap_3()`/`text_sm()`…），档位外的值用
   `rems(P / 16.)`（**不是 `/4.`**，1rem = 16px）并写注释。
   这条规则有守卫脚本：`node gpui/tools/check-ui-px.mjs`（扫 `gpui/crates/*/src/**/*.rs` 的直接 `px(...)` 调用；白名单外的命中 → 退出码 1。白名单只有 7 行，脚本里每条都注明引用了 `coding-guides.md:288` 的哪一类例外）。
8. **诊断走 stderr（`eprintln!`）**：stdout 重定向到文件时是**块缓冲**，进程还在跑时日志里可能一行
   都没有（同一二进制同一参数，实测出现过 3 行也出现过 0 行）。这也意味着"读日志验证"要读 `.err`。
9. **验证要到交互级**：真实鼠标/键盘注入 + 截图 + 像素/几何测量 + 在 PowerShell 侧独立核对副作用
   （例如 stage 后用 `git diff --cached --name-only` 看索引、提交后用 `git log -1 --oneline`）。
   只看"界面看起来对"不算完成。**某条路 3 次没弄好就停下**，把现象/已试方案/证据写进
   `.artifacts/pN/NOTES.md` 并汇报。
10. 结束时杀掉自己启动的 `Lithe`（含 java 子进程与终端 powershell 子进程）。

## 2. 环境事实（不遵守会得出错误结论）

- **显示器 125% DPI：截图是物理像素，逻辑值 × 1.25 = 截图像素**。窗口截图 1823×1024。
  例：对话框 820×620 逻辑 = 1025×775 物理；左栏 190 逻辑 = 237 物理。
- **工作站会锁屏**。锁屏时 `SetForegroundWindow` 被拒 → 真实注入失效；曾用 `PostMessage`
  （`WM_LBUTTONDOWN/UP`、`WM_CHAR`、`AttachThreadInput`+`SetKeyboardState` 置 `VK_CONTROL`）
  作为退路，**它比真实注入弱一档**（不过输入队列与前台判定）。锁屏时**优先做"启动态旗标"验证**
  （加一个 `--xxx-probe` 开关，与点击回调**同一段代码**，首帧后自动打开，无需注入即可截到新帧）。
- **`PrintWindow` 在无人值守下只返回"上次绘制那一帧"**：窗口不再重绘时，两张截图可能 SHA 完全相同
  却已生效。所以"投递点击触发重绘再截图"**不可靠**，用启动态旗标拿新帧。
- `Lithe.exe` 是控制台子系统程序（debug 保留控制台）→ **窗口枚举必须排除控制台窗口**；
  gpui 的窗口类名是 **`Zed::Window`**。`gpui/capture-screenshot.ps1` 与 `.artifacts/ui-click.ps1`
  都已按 pid 枚举、排除 `ConsoleWindowClass`、取客户区最大的窗口。
- 这台会话里偶发**杂散点击**（光标停着时会落到窗口上）→ 测试前把光标挪离活动栏；
  判定不要只看日志，要配像素证据。
- 工具链没有 rustfmt（`cargo fmt` 跑不了）；可用的那份在
  `D:\ProgramData\rust\rustup\toolchains\1.95.0-x86_64-pc-windows-msvc\bin\rustfmt.exe`。
- `./scripts/verify-rust-core.sh` 是 **zsh/macOS** 脚本，本机跑不了（Swift 桥/C ABI 部分需 macOS 侧）。
- JDTLS 载荷已下载在 `.artifacts/jdtls`（61.7 MB，`bin/jdtls.bat`）；跑它要 **JDK 21+**
  （`D:\ProgramData\java\openjdk-21\bin\java.exe`，PATH 上那个是 1.8 不能用）。

## 3. 验证工具（`.artifacts/` 下，不进仓库）

| 工具 | 用途 / 关键点 |
| --- | --- |
| `gpui/capture-screenshot.ps1` | 按 pid 选窗口（排除控制台），打印 `选中窗口 class=…` 与 `窗口矩形 left/top`；`-WholeScreen` 抓整屏 |
| `.artifacts/ui-click.ps1` | 真实鼠标注入，**客户区坐标**；`-Probe` 只报几何（含 `客户区屏幕左上`，用于把截图坐标换算成客户区坐标）；`-Right` 右键；`-Double`；`-Drag` |
| `.artifacts/ui-type.ps1` | 键盘/文本注入，`-Ctrl` 组合键 |
| `.artifacts/p5 … p10/` | 各阶段的验证脚本 + 截图 + `NOTES.md`（**每批的诊断与踩坑都在这里**，接手前值得翻） |
| `.artifacts/verify-visual.ps1` | 早期 4 配置（深/浅/en/坏主题名）视觉回归；**注意**它的"非灰像素"类判据在正文文字上不可靠（色度伪影） |

坐标换算：**客户区坐标 = 截图坐标 − (客户区屏幕左上 − 窗口矩形原点)**（实测内缩 ≈ (9,0)）。

## 4. 队列（按顺序；验收方式一并给出）

已完成（本会话，全部经主代理复核 + 提交）：
`3afa42c7` 右栏镜像修复 · `9d9bcdaa` 图标资源搬入 · `d8bec5a5` exe 图标 · `cf5d28fb` 调研笔记 ·
`61d2c8a2` 设置界面 · `ad81dd88` 截图工具修复 · `208d7b63` 语言自动重启 · `a09f9646` 计划 ·
`aa74cada` 底部窗默认隐藏/切换判据/懒创建 · `f6728ca7` 编辑器 A+B+C+E · `abdcada0` Java 跳转第一批 ·
`928ca3fc` JDTLS 第二批 · `15d223a8` 右侧工具窗 · `a3ed52e7` 命令面板 · `d13b254a` 图标资源提取 ·
`ecd4855a`/`ae548efc`/`260ae5f6`/`a427de1f`/`a2fabcc2` 五份规格/审计 · `060e75d9` 图标换真源 ·
`e0b323e8` 语法高亮（含 Core 升级）· `3b5d1249` 主菜单栏 · `ea731a69`+`1361d75c` 项目下拉 ·
`4c1e28c2` 分支弹窗 · `be1ef509` 标签右键菜单。

**待做**：

0. **🔴 Java 线（当前第一优先级，维护者 2026-09-25 定）**：编辑器**智能提示（补全）** + **代码跳转**，
   然后 **Maven → Spring / Spring Boot**；**用户体验高于一切**。开工前先读本轮两份调研：
   `gpui/research/editor-lsp-completion.md`（上游编辑器自带哪些 LSP 能力、要补哪几件事、
   Core 命令序列与 `syncDocument` 时机）与 `gpui/research/java-spring-maven-inventory.md`
   （`third_party/jdtls` 载荷里到底有没有 m2e / Spring、Maven 最小可用路径、Spring 三条路的取舍）。
   **硬规则**：上游已有的能力**不许自研**（补全菜单、诊断渲染、语义高亮都在 `gpui-base` 的编辑器里）；
   Java 符号/项目模型/classpath 的事实归 JDT LS，我们只做编排与呈现。
1. ~~**源代码管理**~~ **已完成**（规格 `research/windows/08-source-control.md`）：新增
   `crates/git/src/changes.rs`（数据层：`git.status` + `operationState`，空列表时补 `references` 判有无提交；
   `ChangeKind` 逐字对齐 `core-result-adapter.ts:15-26`；6 个写操作；**把"信封 ok:true 但 Git 退出码非 0"
   翻成用户可见失败**（真源这里是静默的）；11 单测）+ `changes_view.rs`（表现层：标题栏 / 错误条 /
   操作横幅 / 变更列表 / 提交面板 / 五态空态）；左栏槽位改成 `top_activity_view == Some(1)` → 变更列表、
   否则项目树；行组件**自写**（`explorer` 零改动，代价是无虚拟滚动）。
   44 条里做了 15 条 + 提交的 4 条前置校验 + 「丢弃全部更改」（带确认框）；未做的逐条列在两个新文件的
   模块文档与 `.artifacts/p11/NOTES.md` §6。验证用真实注入 + PowerShell 侧独立核对（stage →
   `git diff --cached --name-only`；提交 → `git log -1` 真多一条；失败 → 会挂的 pre-commit 钩子触发红条
   且 `git log` 不变）。`git/src/{lib,model}.rs` 里"调 6 条 `git.*`"的文档已纠正为 5 条。
2. **设置剩余页**（除 AI）：`research/windows/07-settings-ui.md` §3 有逐页签规格、§7.3/§7.4 有
   "能否立刻生效/建议范围"。原则：**能真生效的先做**（编辑器字号/换行、终端 profile、Git 身份、
   LSP 的 jdtls 路径…），没有数据源的做成**明确空态**并写清前置条件，**不塞假控件**。
   ⚠️ **本项已降级**：维护者 2026-09-25 明确"Git 先不管、Java 优先"，所以本项**只剩 LSP 页的
   jdtls 运行时路径**与 Java 项目页（都与 Java 线相关，随 Java 线一起做）；
   与 Java 无关的空态页（快捷键 / 日志 / 更新）**不再单独推进**。
   **进度（阶段 14，见 `gpui/PLAN.md` §14）**：
   - ✅ **编辑器页**：`fontSize`（→ `Theme.mono_font_size`）+ `tabSize`（→ 每个 `EditorState`）。
     真源 4 项里的 `codeLens` / `horizontalTabScroll` **不画**（gpui 侧没有消费方，理由在
     `settings/src/dialog.rs::editor_page` 的文档里）。
   - ✅ **终端页**：`terminalDefaultShellId`（→ 新建会话真的换 shell；幂等，不覆盖页签条 ⌄ 的手动选择）。
   - ✅ **左栏补齐到 11 项**：实现的 4 页（常规/外观/编辑器/终端）+ **7 个明确空态页**
     （项目 / 运行配置 / 快捷键 / LSP / Git / 日志 / 更新：只有页标题 + 「此分类尚未接入」+ 一句前置条件，
     **一个控件都没有**）。空态页的诚实性由测试钉住（`every_category_is_implemented_or_declares_a_prerequisite`）。
     阶段 15 之后「Git」不再算空态页（见下条）；剩下的空态页是 6 个 + Git 的降级分支。
   - ✅ **Git 身份**（阶段 15，见 `gpui/PLAN.md` §15）：照上面建议的 `TabMenuHostActions` 口径做了宿主钩子
     （`settings::identity::{GitIdentityHost, GitIdentityPage, set_git_identity_host}`，
     `ShellWorkspace::new` 用 `lithe-gpui-git` 的实现登记；**未登记 → 明确空态**，不 panic）。
     做了**提交身份**（作用域 local/global + 姓名/邮箱，各自保存 + 清除覆盖 + 当前生效值；
     `git.repositorySetup` / `git.configureIdentity`）**外加该页唯一一个有消费方的开关**
     `settings.git.confirmDiscard`（→ `ChangesView::set_confirm_before_discard`，丢弃路径的确认框）。
     新增 1 个设置键 `confirmBeforeDiscard`（默认 `true`，三处表都改到位）；**零新增 locale 键**。
     真源另外 9 项（`gitExecutable` / 凭据助手 / 3 个 Fetch 项 / `coreFeatures.git` / `autoRefreshGitStatus` /
     `gitChangesFolderView` / 5 个视图开关 / `gitDefaultDiffView` / `enableInlineGitBlame`）**一项没画**，
     逐条理由在 `settings/src/dialog.rs::git_page`。
     **剩下什么**：① global 作用域的真实写入**没有从 UI 走通**（作用域下拉点了不弹菜单，3 次停手，
     证据在 `.artifacts/p13/NOTES.md` §3.3；本机验证全跑在假 `HOME` 下，真实 `.gitconfig` 未被写）；
     ② `git.initialize`（非仓库页的「初始化 Git 仓库」按钮 + 确认框）没做；
     ③ `confirmBeforeDiscard` 只有"值到消费方"的日志证据，"关掉后不弹框"的交互未跑到。
   - ⏳ **LSP 页**：真源 3 个开关（`autoCompletion` / `parameterHints` / `semanticTokens`）在 gpui 侧
     **没有消费方**（不画）；能真做的是 **jdtls 运行时路径**（现在只有
     `java/src/jdtls.rs:440-460` 的 `LITHE_JDTLS_JAVA` / `JAVA_HOME` / PATH 三级发现）。
     这是"能真生效"的设置页里**最后一块没做的**。
   - ⚠️ **加新设置键时先读 `gpui/PLAN.md` §14.2**：`persistence.rs` 的手写逐键表漏键会
     "写得出、读不回"且无诊断；守卫测试 `every_key_survives_a_round_trip` 会用"所有字段非默认"
     的往返把它照出来。（阶段 15 的 `confirmBeforeDiscard` 就是照这条走的。）
3. **删旧前端之前的前置**（缺一不可）：
   - `gpui` **CI 覆盖**（`.github/**` 目前对 `gpui` **零命中** → 新前端完全没有 CI；建议加
     `paths: gpui/**` 的构建 + 改动范围测试 job）；
   - **字体迁移接线**（`macos/Resources/Fonts/JetBrainsMono-*.ttf` 4 个 + `OFL.txt`；`gpui/` 目前
     0 个字体文件，真机编辑器用 JetBrains Mono）；
   - **生成器解耦**（`extract-locale.mjs` 读 `windows/tauri/src/i18n/{locale,ai-commit}.ts` 是**硬阻塞**；
     `generate-idea-icons.mjs` 读 `windows/tauri/scripts/idea-icon-mappings.json`，缺了会**改产物** →
     两个 TS + 一个 JSON 拷进 `gpui/` 并改常量）；
   - **主题补迁**（legacy 12 文件/35 条主题，gpui 只有 2 条；legacy 独有 36 个语义色 key：18 syntax +
     16 terminal + 6 git…）。顺带解决已知偏差：编辑器底色从 Lithe Dark `#292A2E` 变成内置高亮主题的
     `#0A0A0A`（**不能只补 `editor.background`**：`HighlightThemeStyle.syntax` 是非 Option，
     部分段会让整份主题**静默失效**，要补全 18 色）。
4. **打 tag `legacy-frontends-final` → 按 `research/legacy-frontend-removal-audit.md` §4 的分步流程
   删 `macos/`、`windows/`**。注意该审计的结论：删目录的 PR 会因 CI **无 paths 过滤**而红、
   `classify-ci-changes.sh:394-398` 的 `*)` 回退还会把**全部 lane 点亮**；
   `deploy-agent-notes-board` 硬引用 `macos/Resources/AppIcon.png`；`.agents/notes/**` 有 136 条
   适用范围引用 + 9 条相对链接；约 **764 处死链接**要清。

## 5. 已知偏差 / 未完成项（别当已解决）

1. **`rust/` 的 tree-sitter 升级**：`gpui-kit` 的 `tree-sitter-java` 要求 `^0.26.13`，与 Core 的
   `0.25` 因 `links` 唯一性冲突 → 已按授权改成 `0.26` + `mybatis.rs:220` 的 `index as u32`。
   **macOS 侧未验证**（Swift 桥/C ABI/全量 `verify-rust-core.sh`）；`cargo test -p lithe-core` 本机
   631 passed / 35 failed（全 `git*` 超时，已用改动前的 0.25 在仓外树证明是**既有**问题）。
2. **文件类型图标只部分验证**：`pom.xml` 确实画出了 IDEA 的 Maven 图标；但 `.gitignore`/`README.md`/
   **文件夹图标**未确证（前一个代理卡在这里耗尽预算）。下一步从这个现场查：主题 `extension.json:222`
   的 `filenames` 里明明有 `.gitignore`、`file_icon.rs:130` 也解析了 `filenames`，为什么没生效。
3. **主菜单栏没有键盘路径**："Tab 聚焦 + Enter 打开"**未实现**（只有 `close()` 的纯函数单测）；
   `Alt` 助记键在 gpui 无 mnemonic 概念下做不到 —— 要么补 Tab/Enter，要么明确接受"鼠标优先"。
4. **分支弹窗两处偏差**：页签行在**搜索框上方**（用了 `Command` 的 header 槽位），真源是搜索框**下方**、
   列表上方（需自绘）；搜索行右侧没有真源的关闭按钮，「新建分支」按钮 absent（命令缺失）。
   面板几何 `704/512/64` 与真源逐值相等。
5. **项目下拉**：面板右侧有一条**常显细滚动条**（`scrollable(true)` 的 16 逻辑 px 占位）；
   三条动作行已按 review 改成**禁用态**（不再"点了只打日志还关面板"）。
6. **设置界面**：已做「常规」（语言）+「外观」（主题/外观模式/界面字号/显示状态栏）+
   **「编辑器」**（编辑器字号 / 制表符宽度）+ **「终端」**（默认 Shell）四页，其余分类按 §8.2 的理由没做；
   「显示语言」选完**自动重启**（`restart.rs`），代价是当前进程里未保存的编辑器内容与终端会话会结束
   —— 编辑器接上保存后要重新评估是否加确认。
   已知副作用（**有意**）：真源把 `fontSize` 与 `terminalFontSize` 分成两个键，而 gpui 的主题只有
   一个 `mono_font_size`，"编辑器字号"因此同时作用于终端正文（设置页的描述里写了）。
7. **平台/环境**：`cargo fmt` 不可用；`verify-rust-core.sh` 需 macOS；`Lithe` 的 release 构建已关掉
   控制台窗口（`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`）。

## 6. 文档索引（接手先读这些）

| 文件 | 内容 |
| --- | --- |
| `gpui/PLAN.md` | 执行计划；**§9 阶段 9 编辑器 / §10 Java 导航（两批）/ §11 协作纪律** 是本会话定的 |
| `gpui/research/windows/01…12-*.md` | Windows 真源逐区域规格（01 shell / 02 explorer / 03 git+底部 / 04 主题与组件 / 05 终端运行调试 / 06 数据库 AI 搜索 / **07 设置界面** / **08 源代码管理** / **09 标签右键菜单** / **10 主菜单栏** / **11 项目下拉** / **12 分支弹窗**） |
| `gpui/research/editor-syntax-highlighting.md` | 语法高亮的两处缺口、语言↔扩展名映射、判据与主题坑 |
| `gpui/research/app-icon-and-assets.md` | `AssetSource` 机制、`WindowOptions.icon` 只对 X11 有效、exe 图标必须走资源 ID 1 |
| `gpui/research/icon-asset-inventory.md` | 我们用的 27 个 Lucide 字形 ↔ 真源对应物（哪些能 1:1、哪两个只有内联组件） |
| `gpui/research/legacy-frontend-removal-audit.md` | 删旧前端的分步流程、阻塞项、CI 影响、死链接清单 |
| `gpui/assets/README.md` §7 | 图标资源来源 + 全量复核脚本 |
| `gpui/docs/gpui-kit/0.6.6/zh-CN/**` | gpui-kit **0.6.6** 本地文档镜像（**规格真源是 0.6.6，不是在线 `versions/main`**）；`docs/design-guides.md` 是界面验收的硬要求 |

## 7. 可直接粘贴的开场提示（给自己或新会话）

> 读 `gpui/HANDOFF.md`、`gpui/PLAN.md`（§9/§10/§11）与 `gpui/research/legacy-frontend-removal-audit.md`。
> 先做状态核查：`git log --oneline -6`、`git status --short`、`cargo build --bin Lithe`（**重定向到
> `.artifacts/pN/build.log` 再 grep，不要用管道**）。若工作区有未提交的改动，判断属于队列里哪一项，
> 按 HANDOFF §4 的验收方式复核后提交（**只 `git add` 需要的那几个路径**）。
> 然后按 HANDOFF §4 的队列往下做：设置剩余页（除 AI）→ 删前端前置（gpui CI → 字体迁移 →
> 生成器解耦 → 主题补迁）→ 打 tag 删 `macos/`、`windows/`。
> 纪律照 HANDOFF §1：cargo 只跑改动范围、同一时刻只有一个代码写者（文档类可并行）、
> 子代理不 commit、界面文案走 i18n、颜色走主题、布局走 rem 档位、诊断走 stderr、
> 验证要到交互级（真实注入可用；锁屏时用启动态旗标）且**某条路 3 次没弄好就停下来写文档**。
