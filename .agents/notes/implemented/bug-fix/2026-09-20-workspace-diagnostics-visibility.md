# Agent 笔记：未打开文件的诊断必须可见

状态：已实现

## 先说结论

以前只保留"编辑器里已经打开的文件"的诊断，语言服务报给其他文件的错误一律丢弃。
结果是：Java 项目因为编译错误而无法启动时，Lithe 提示"请修复报告的错误"，但用户在
界面上根本看不到任何错误——错误在没打开的模块里，已经被丢掉了。

现在改为：诊断所属文件只要落在某个已打开工作区（workspace root，即用户打开的
项目根目录）内就保留，工作区之外的（JDK 源码、依赖 jar、反编译的 class）继续
丢弃。为防止超大仓库把状态撑爆，未打开文件的诊断最多跟踪 2000 个文件。

开发者以后要注意：**不要再以"文件有没有打开"作为诊断的过滤条件**。判断依据是
"在不在工作区内"。

> **2026-09-30 在 Rust 前端上的重新落地（同一决策，换了实现位置）**：这条决策原本写在
> 旧 Windows 前端（TypeScript）里。旧前端删除、gpui + Rust Core 成为唯一宿主之后，
> 这条过滤**在 Core 里被写成了"只接受已打开的文档"**（`lsp/interface/client.rs` 的
> `publishDiagnostics` 分支），于是问题以同样的形态回来了：错误在没打开的文件里，
> 界面上一片空白（第一次读那行代码的人多半会以为这是有意的性能取舍，其实它就是
> 本笔记要修掉的缺陷）。现在保留判据回到 Core：**已打开的文档 + 工作区根之内的文件**。
> 下面"决策"一节补了新宿主的落点，行为语义（保留 / 清除 / 忽略、上限、"已打开文件不受限"）
> 与旧前端逐条一致。

## 问题

若依 Plus（RuoYi-Vue-Plus，一个 40 多模块的 Maven 项目）在 Windows 上点运行会
失败。日志里的终点是：

```text
Java project build finished outcome=CompilationErrors
run.launch launchFailed reason="The Java project has compilation errors. Fix the reported errors and try again."
```

这条提示本身是准确的。Java Debug Server 的 `Compile.compile` 会构建工作区里
全部 Java 工程，但**只**用目标工程以及它 classpath 上的依赖工程的 error marker
（Eclipse 对某个文件记录的错误标记）来判定 `WITH_ERROR`。也就是说，确实是
ruoyi-admin 自己或它的某个依赖模块有真实编译错误。

问题出在用户拿不到任何可操作信息。`lsp-client.ts` 收到
`textDocument/publishDiagnostics`（语言服务推送诊断的通知）后，用
`resolvePublishedDiagnosticsFilePath` 把路径匹配到已打开的 buffer；匹配不上就
直接 return，并只写一行 `logger.debug`。

那份复现日志里用户只打开了 `DromaraApplication.java`，而错误在别的模块里。
于是：语言服务把错误发过来了，Lithe 自己扔了，然后又提示用户"去修复那些错误"。

这个过滤是有意写的，`diagnostics-file-path.test.ts` 里还有一条
`rejects closed documents` 的用例保护它。所以这不是疏漏，是一个需要被推翻的
旧决策。

## 决策

`resolvePublishedDiagnosticsFilePath` 增加第四个参数 `workspaceRoots`，解析顺序为：

1. 能匹配到已知 buffer 或已跟踪文档 → 返回那个文件**原本的路径拼写**。这样
   同一个文件来自 LSP 和 linter 的诊断仍然合并在同一条记录下。
2. 否则，路径落在任一 workspace root 内 → 返回归一化后的发布路径。
3. 否则 → 返回 `null`，丢弃。

不传 `workspaceRoots` 时行为与改动前完全一致，所以其他调用方不受影响。

workspace root 取自 `LspClient` 已有的 `activeLanguageServers`（serverKey 形如
`<workspacePath>:<languageId>`），不引入对 project store 的新依赖——诊断本来就
由某个语言服务推送，它的工作区就是最准确的判断依据。

### Rust 宿主上的落点（2026-09-30）

新宿主里"谁接受一条诊断"只有一个地方：`lsp/interface/client.rs` 的
`publishDiagnostics` 分支（LSP 状态机归 Rust Core，见
`2026-09-13-language-tooling-and-lsp-runtime-ownership.md`）。判据与上文同源：

- **已打开的文档**：行为不变（版本号必须与文档版本一致，快照整份替换）；
- **未打开但工作区根之内的文件**：`store` / `clear` / `ignore` 三种结果照旧 ——
  非空快照保留并发一条 `diagnostics` 事件（这样前端的两个消费者都收得到：
  编辑器按 URI 找自己的 buffer，问题列表按 URI 分组列出全部）；
  空快照只在**曾经有过诊断**时才清掉，从来没干净文件不产生事件；
  没进过表的空快照什么都不做（否则清理一个干净仓库会为每个文件发一条事件）；
- **工作区之外**：丢弃。包含判据按**路径**比较（`Path::starts_with`，经 Core 既有的
  URI → 路径解码），所以 `/project-b` 不算在 `/project` 之内，UNC 与百分号转义也认得；
  非 `file:` URI（`jdt://` 虚拟源码）永远不算在内；
- **上限**：未打开文件最多 2000 个。**已打开的文档不受上限约束**（编辑器看不到
  眼前文件的错误，比列表被截断更糟）；触顶后已经在跟踪的文件继续更新，只拒绝新文件；
- **关闭文档**：文档本身（含版本号）照旧离开会话，但**工作区内的文件保留快照** ——
  文件还在，它报的问题就还在。旧前端同样如此（store 按文件路径存，不随 buffer 关闭清理），
  只有服务端用空快照才能真正撤回一条诊断。

### 为什么需要"保留 / 清除 / 忽略"三种结果

语言服务用**空诊断数组**来撤回某个文件的错误。如果照单全收，工作区里每个干净
文件都会在状态里占一行。所以未打开文件的处理拆成三种结果，逻辑放在
`diagnostics-retention.ts` 的 `decideWorkspaceDiagnostics` 里：

- `store`：有诊断，保留。
- `clear`：这个文件之前有诊断、现在空了 → 删掉记录，并把名额还回去。
- `ignore`：这个文件本来就没记录、现在也是空的 → 什么都不做。

**已打开的文件永远是 `store`，不受上限约束**，否则编辑器可能显示不出自己的错误。

到达上限后，**已经在跟踪的文件仍然继续更新**，只拒绝新文件。否则已跟踪文件会
卡在一份过期的错误上，比看不到错误更容易误导人。

正确做法：

```ts
// 判断依据是"在不在工作区内"
resolvePublishedDiagnosticsFilePath(published, buffers, openDocs, this.activeWorkspaceRoots());
```

不要这样做：

```ts
// 以"有没有打开"做过滤，未打开模块的错误会消失
if (!isDocumentOpen(published)) return;
```

关闭工作区时，`stop()` 调用 `clearWorkspaceDiagnostics` 清掉该工作区下所有
未打开文件的诊断。否则项目关掉了，它的错误还留在面板上。

## 考虑过的备选方案

- **只在启动失败时抓一次快照**：仅在 `javaBuildCompilationErrors` 发生时收集
  目标工程及其依赖的 error marker，展示在 Run 面板，不动全局诊断管线。改动
  范围最小，也保留了原有过滤。被否：Problems 面板只显示已打开文件，本身就不
  符合用户对 IDE 的预期；而且这条路要为"启动失败"单独造一套诊断收集，和已有的
  `diagnostics.store` 形成第二份真源。
- **完全不限制，收下全部发布的诊断**：被否。JDK 源码、依赖 jar 里的类、反编译
  出来的 class file 都会进面板，噪音压过真正要看的错误。
- **只把被丢弃的诊断补进日志，不改行为**：改动最小，也能让这次排查继续。被否：
  它只服务于这一次排查，用户界面上依然看不到错误，下一个人遇到同样问题还是
  只能看日志。
- **在 `diagnostics.store` 里统一加上限**：被否。store 同时服务 LSP 和 linter，
  在那里加限制会把"已打开文件的诊断"也一起限制掉。上限属于"未打开文件"这个
  新增来源，应该由引入它的边界负责。

## 后果

收益：

- 构建失败阻止启动时，用户能在诊断面板里直接看到是哪个模块、哪个文件、哪一行。
  #692 这类问题重跑一次就能自证病因。
- 诊断面板从"当前打开文件的错误列表"变成真正的工作区问题列表。

代价和例外：

- 大型项目首次构建后，诊断状态会比以前大。上限 2000 个未打开文件是拍出来的
  经验值，不是测出来的；若依 Plus 量级（1186 个文件）远达不到。触顶时会报一次
  （旧前端是 `logger.warn`，Rust 宿主是 Core 发的一条 `warn` 级 `log` 事件），
  每次触顶只报一次。
- 上限触顶后新文件的错误会被静默丢弃。这是刻意的取舍：面板在几千条错误时
  本来就已经失去意义。
- 本次改动**没有**触碰启动门禁本身。构建报编译错误时 Lithe 仍然拒绝启动，
  且不提供"仍然运行"的选项——VS Code 在同样情况下会弹窗让用户选择继续。
  是否要加这个放行路径是独立决策，见
  `.agents/notes/implemented/architecture/2026-09-18-java-project-build-and-launch-boundary.md`。

## 验证

- 旧前端（已删除）：`cd windows/tauri && bun test src/features/editor/lsp`
  （`diagnostics-file-path.test.ts` 覆盖工作区内保留、已知 buffer 优先、工作区外
  丢弃、同名前缀目录不算包含、分隔符归一、POSIX 大小写敏感；
  `diagnostics-retention.test.ts` 覆盖三种结果、上限行为和"已打开文件不受限"）。
  完整口径见 git tag `legacy-frontends-final`。
- Rust Core（当前真源）：`cargo test -p lithe-core --lib lsp::`
  —— `client_core_retains_diagnostics_for_unopened_workspace_files`（保留 / 清除 / 忽略）、
  `client_core_drops_diagnostics_outside_the_workspace`（工作区之外、同名前缀目录、
  没有工作区根三种情形）、`client_core_accepts_versioned_diagnostics_for_unopened_workspace_files`、
  `client_core_bounds_unopened_workspace_retention_without_freezing_tracked_files`（上限
  只拒绝新文件）、`client_core_keeps_workspace_diagnostics_when_a_document_closes`；
  引擎层两条：`diagnostics_follow_the_current_document_version_and_clear_on_close`
  与 `closing_a_document_outside_the_workspace_clears_its_diagnostics`。
- 问题列表的数据整理（排序 / 分组 / 显示名 / 严重度分档）：`cargo test -p lithe-gpui-editor --lib`。
- 未在本次执行：真实 JDTLS 端到端（需要 JDK 21 + `.artifacts/jdtls` 载荷 + 一个会报错的
  Java 工程）与界面点击验证。接手者按"打开一个含编译错误的 Maven 工程 → 打开底部「诊断」页"
  复现，日志判据是 `S1_JAVA_PROBLEMS files=… problems=…` 与 `S1_EDITOR_PROBLEMS …`
  两行（前者来自服务查询，后者来自列表重建）。

## 适用范围

- Rust Core：`rust/lithe-core/src/lsp/interface/client.rs`（保留判据与上限）、
  `rust/lithe-core/src/lsp/interface/engine.rs`（关闭文档与上限提示）
- gpui 宿主：`rust/lithe-gpui/crates/java/src/{events.rs,service.rs}`（按 URI 归档 +
  `workspace_diagnostics` 查询）、`rust/lithe-gpui/crates/editor/src/problems.rs`
  （底部工具窗「诊断」页）、`rust/lithe-gpui/crates/workbench/src/workspace.rs`
  （把面板接进底部工具窗的切换处）
- 兼容面：`shared/contracts/rust-core-api.md` 的 `lsp.pollEvents` 一节（诊断接受范围）
- 相关笔记：
  `.agents/notes/implemented/architecture/2026-09-18-java-project-build-and-launch-boundary.md`、
  `.agents/notes/implemented/architecture/2026-09-13-language-tooling-and-lsp-runtime-ownership.md`
