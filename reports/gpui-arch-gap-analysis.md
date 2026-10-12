# GPUI 前端架构层缺口分析（闻本立 · 编辑器架构师）

**维度**：内核架构、文本模型、渲染与 LSP/DAP 集成设计
**基线**：提交 `3085f5d1`，分支 `feat/menu-bar-opaque`，Windows 主机
**取证**：三个可重跑脚本（`reports/gpui-arch-s1-probe.ps1`、`-s2-`、`-s3-`），**均已实跑**
**范围声明**：只做架构分析与设计，不改代码、不建分支、不提交、不启动 Lithe 进程。所有运行验证**未执行**，逐条标注。

---

## 结论

### 一句话

GPUI 前端在架构层的问题**主要不是「缺实现」，而是「实现散落在三个地方、只有一部分被接上」**：Core 已归一化好的能力（LSP 22 个语义操作、DAP 18 个命令、Git 26 种写操作）、组件库已交付的 widget（编辑器 20+ 个），宿主都用得很少。真正的「Core 缺命令」只有 **2 项**（`signatureHelp`、`rangeFormatting`）。

### 1. 模块划分与依赖方向（现状）

依赖只能向下，实测自 `Cargo.toml`（脚本核对）：

```
app (1275 行)  ──→ workbench (24971) ──→ editor (5846) ──→ java (4017)
                                        └→ git (6101)   ──→ shared (6428) ──→ lithe-core
                     settings (14082) ──→ run (1984)
                     explorer (1185) / terminal (1434) / notify (877) / extensions (298)
                     全部 ──→ lithe-ui（平级独立 workspace，path 依赖）
```

| 模块 | 职责 | 依赖 | 现状 |
|---|---|---|---|
| `workbench` | 工作台外壳：菜单、标签、面板、搜索、调试、会话 | editor/git/java/settings/terminal/… | 24971 行，最大；菜单 42 条 `NotWired` |
| `editor` | 编辑器宿主：9 个模块（见 §3） | java/shared/lithe-ui | `go_back`/`go_forward` 死代码 |
| `java` | **唯一的语言服务**（`providerId` 恒为 `"java"`，`session.rs:74`） | shared | 只实现 5 个语义方法 |
| `git` | 变更/日志/远程/身份 | shared | `IntegrationOperation` 无 UI 消费者 |
| `settings` | 设置、主题、键位、最近项目 | run/shared | 562 KB |
| `lithe-ui` | 组件库（平级 workspace） | gpui-pre | 编辑器 widget 大半零引用 |

**结构性问题**：`java` crate 只依赖 `shared`（不碰 gpui），这是好设计；但**它同时是唯一语言服务入口**，导致「非 Java 文件没有任何语义能力」成为架构约束而非功能缺失。

### 2. LSP 请求面：Core 22 个操作 vs GPUI 5 个

`LspSemanticOperation`（`engine.rs:276-324`）有 22 个变体，`semantic_method`（`:4222-4246`）映射齐全。GPUI 侧 `crates/java/src/service.rs` 只发出 **5 个**：

| operation | 行 | | Core 有、GPUI 未用 |
|---|---|---|---|
| `definition` | :146 | | `References`（引用） |
| `hover` | :199 | | `Implementation`（转到实现） |
| `completion` | :253 | | `TypeDefinition`（转到类型定义） |
| `codeActions` | :318 | | `Rename`（重命名） |
| `virtualDocument` | :671 | | `Formatting`（格式化文档） |
| | | | `Declaration`、`InlayHints`、`FoldingRanges`、`SemanticTokens`、`CodeLens`、`ResolveCompletion`、`ResolveCodeAction`、`ExecuteCommand`、`JavaEntrypoints`、`JavaTestItems`、`JavaMainMethods`、`JavaSuperImplementation` |

**7 个 `MISSING_LSP_REQUESTS` 入口的最终判定**（详见第 1 步报告）：

| 入口 | Core | 结论 |
|---|---|---|
| 格式化文档 / 重命名 / 引用 / 转到实现 / 转到类型定义 | **有** | UI 缺接线（5 项） |
| **格式化选区** | **无** | `validate_lsp_method` 不放行 `rangeFormatting`，且 `formatting` 参数构造（`client.rs:723-732`）**不读 `request.range`**（对照 `inlayHint:735`、`codeAction:744` 都读） |
| **参数提示** | **无** | 全仓 `signatureHelp` 命中 0 |

**必须记住的隐蔽点**：`SemanticRequest` **有** `range` 字段（`engine.rs:343`）。只补 method 白名单而不改参数构造，症状是「格式化选区 = 格式化文档」，**不报错**。

### 3. 编辑器内核 9 个模块

| 模块 | 大小 | 职责 | 缺口 |
|---|---|---|---|
| `buffer.rs` | 11.9 KB | 读取、语言解析、只读、2 MiB 上限 | — |
| `completion.rs` | 17.0 KB | 补全取数 + 接受编辑 | 已接线 |
| `hover.rs` | 7.2 KB | 悬停取数 | 已接线 |
| `navigation.rs` | 18.9 KB | 跳转 + **编辑器列 ↔ LSP UTF-16 列唯一换算点** | 只接 `definition` |
| `diagnostics.rs` | 9.6 KB | 诊断坐标换算 | 已接线（带 generation 防过期，`editor_view.rs:447`） |
| `code_actions.rs` | 11.8 KB | 代码动作 | 已接线（自组菜单，未用库的 `CodeActionMenu`） |
| `problems.rs` | 23.0 KB | 问题面板（**自建**） | 库有 `ProblemsPanel`，宿主另写一份 |
| `go_to_line.rs` | 10.3 KB | 行跳转 | 已接线 |
| `editor_view.rs` | 104.1 KB | 标签、跳转历史、断点槽、会话恢复 | `go_back`/`go_forward` 死代码（:1082、:1093） |

**9 个模块里没有**：大纲/符号树、面包屑、编辑器分组（分屏）、查找替换面板、inlay hint、折叠、code lens、语义高亮。实测关键词命中：`breadcrumb` 0、`EditorGroup`/`PaneGroup` 0、`inlay`/`Inlay` 0、`foldingRange` 0、`documentSymbol` 0；`outline` 的 10 处**全是** `lithe.outline.openFile` 命令 id 或 `ButtonVariant::Outline`。

### 4. 文本模型与高亮：两处 O(n) 每次编辑

这是本次最实的**性能架构债**，两处独立：

**(a) 行表整篇重建** —— `lithe-ui/src/editor/buffer.rs:265-278`：

```rust
pub(crate) fn replace(&mut self, range: Range<usize>, text: &str) {
    assert!(...);                      // 整字符边界校验
    self.text.replace_range(range, text);
    self.starts = starts(&self.text);  // ← 每次编辑重建整张行表
    self.version += 1;
}
```

`starts()`（`:116-120`）遍历全文 `match_indices('\n')`。⇒ **每次击键 O(n)**。

**(b) 语法树整篇重解析** —— `lithe-ui/src/language/mod.rs:259-289`：

```rust
pub fn reparse(&mut self, text: &str) {
    let Some(tree) = self.parser.parse(text, None) else { ... };
    //                                    ^^^^ 第二个参数是 old_tree，传的是 None
    ...
    self.tree = Some(tree);   // 存了，但下次仍然传 None
}
```

结构体**持有** `tree: Option<Tree>`（`:219`），但 `parse` 的旧树参数恒为 `None`。⇒ **tree-sitter 的增量解析能力被完全放弃**，每次版本变化整篇重解析。

**放大因素**：`Buffer::highlight_spans`（`:187`）由 `syntax.rs:147` 按**可见行**调用，而 `Highlight::spans`（`:51-70`）只在 `parsed != Some(version)` 时才重解析 —— 所以重解析是**每个版本一次、懒触发**，不是每行一次。这个设计是对的（避免了每行重解析），但**一旦触发就是整篇**，长文件下拖累明显。

**可测量的目标**：改为「行表增量维护 + `parse(text, self.tree.as_ref())`」后，单次击键成本应从 O(文件长度) 降到 O(编辑行 + 重解析子树)。测量方法：在 1 万行 Java 文件上连续输入，取 `EditorEvent::Changed` 到下一帧的时间。

### 5. DAP 与 Git

**DAP**：Core 有 18 个 `debug.*` 分支（`dispatcher.rs:673-980`），GPUI 引用为 0 的 **8 个**：`setVariable`、`setExceptionBreakpoints`、`setFunctionBreakpoints`、`setDataBreakpoints`、`dataBreakpointInfo`、`cancelOperation`、`runInTerminalResponse`、`javaTestLaunch`。

**`cancelOperation` 的架构含义**：DAP 与 LSP 的取消命令**都是 0 引用**（`debug.cancelOperation`、`lsp.cancelOperation`）。⇒ **「取消在途操作」这条语义在 UI 层整体缺失**。这不是两个功能，是**一个架构缺口**，应合并设计：统一的中止入口 + 服务端状态一致性保证。

**Git**：`git.write` 是复用型多路器。Core 接受 **26 种** operation（`rust/lithe-core/src/git/mod.rs:930` 起），GPUI 只发出 **8 种**（`stage`/`unstage`/`stageAll`/`commit`/`discard`/`fetch`/`pull`/`checkout`）。未用的 18 种含 **merge / rebase / cherryPick / revert / reset / createBranch / deleteBranch / renameBranch / discardAll / createWorktree**。

`IntegrationOperation` + `integration_gate` 实测 22 处命中**全在 `crates/git/src/` 内部**（含 `lib.rs` re-export 与 `remote.rs` 单测），`workbench`/`editor`/`*_view.rs` 一处都没有。⇒ **实现与单测齐备，UI 消费者为零**。

---

## 证据与定位

### 可重跑的取证脚本

| 脚本 | 覆盖 | 状态 |
|---|---|---|
| `reports/gpui-arch-s1-probe.ps1` | Core LSP 方法面四处交叉 + 关键词命中 | 已实跑 |
| `reports/gpui-arch-s2-probe.ps1` | lithe-ui widget 导出 vs 宿主引用差集 + 15 个 MISSING_* 归因 | 已实跑 |
| `reports/gpui-arch-s3-probe.ps1` | DAP/Git 命令差集 + 台账复核 | 已实跑 |

**脚本缺陷与修订（诚实记录）**：三个脚本首版都**没有排除 `rust/lithe-gpui/target/`**，把构建期生成的 14 个 `.rs` 扫了进去，导致 `viewer` 关键词虚报命中 1（实际来自 `mime_guess` 的 `mime_types_generated.rs`）。已统一加 `Where-Object { $_.FullName -notmatch '\\target\\' }` 并**重跑**，本文与三份分报告的数字都是修订后读数（纳入扫描：lithe-gpui 源码 123 个 `.rs`，lithe-core 140 个）。

### 关键行号索引

| 事实 | 位置 |
|---|---|
| `LspSemanticOperation` 22 个变体 | `rust/lithe-core/src/lsp/interface/engine.rs:276-324` |
| `semantic_method` 映射 | 同上 `:4222-4246` |
| `validate_lsp_method` 白名单（18 个 method） | `rust/lithe-core/src/lsp/interface/client.rs:673-699` |
| `feature_request_params`（formatting 不带 range） | 同上 `:701-`，formatting 在 `:723-732` |
| `SemanticRequest` 有 range 字段 | `engine.rs:343` |
| GPUI 只发 5 个 operation | `rust/lithe-gpui/crates/java/src/service.rs:146/199/253/318/671` |
| `providerId` 恒为 java | `rust/lithe-gpui/crates/java/src/session.rs:74` |
| `Buffer::replace` 重建整张行表 | `rust/lithe-ui/src/editor/buffer.rs:265-278`（`starts()` 在 `:116-120`） |
| `Highlighter::reparse` 传 `None` 旧树 | `rust/lithe-ui/src/language/mod.rs:259-289`（`tree` 字段 `:219`） |
| 组件库编辑器 widget 导出 | `rust/lithe-ui/src/editor/mod.rs:36-70` |
| 宿主零引用的 widget 清单 | 第 2 步报告 §B |
| `go_back`/`go_forward` 死代码 | `rust/lithe-gpui/crates/editor/src/editor_view.rs:1082`、`:1093` |
| 15 个 `MISSING_*` 分组 | `rust/lithe-gpui/crates/workbench/src/menu_bar.rs:79-170`，`MISSING_GROUPS` 在 `:154` |
| `MENUS` 数组边界 | 同上 `:595-935` |
| Core DAP 18 个分支 | `rust/lithe-core/src/runtime/dispatcher.rs:673-980` |
| Core `git.write` 26 种 operation | `rust/lithe-core/src/git/mod.rs:930` 起 |
| `IntegrationOperation` / `integration_gate` | `rust/lithe-gpui/crates/git/src/remote.rs:45`、`:515` |
| Git 身份配置页**已接线** | `rust/lithe-gpui/crates/workbench/src/workspace.rs:883`（所在 `pub fn new` 起 `:587`） |

### 台账复核：2 处不一致

| 台账 | 实测 | 判定 |
|---|---|---|
| §2 `MenuItem::Action` = 52 | **51**（数组内 595–935） | 差 1，口径需澄清 |
| §2.2 Git 身份配置页「`set_git_identity_host` 生产代码零调用」 | **`workspace.rs:883` 在生产代码里调用** | **不成立**，该页已接线 |
| §2 `NotWired` 数组内 42 / 外加 2 / 合计 44 | 42 / 2 / 44 | 一致 |
| §2.2 `go_back`/`go_forward` 标 `dead_code` | 仍标 | 一致 |
| §2.3 `viewer` 命中 0 | 0 | 一致 |

**第 2 条会改变排期**：照抄台账会重复排一次已完成的活。

---

## 风险与条件

| 风险 | 影响 | 触发条件 | 缓解 | 验证方式 |
|---|---|---|---|---|
| **只补 method 不补参数构造** | 「格式化选区」静默退化成「格式化文档」 | 排期只改 `validate_lsp_method` | 补 `rangeFormatting` 时必须同改 `feature_request_params` | `lsp/tests.rs` 加断言「请求体含 range」 |
| **取消语义整体缺失** | 卡住的语义/调试请求只能等超时 | 用户想中止 | DAP+LSP 合并为一项设计 | `cancelOperation` 有调用点且服务端可恢复 |
| **文本模型两处 O(n)** | 长文件下每次击键都重算全文 | 打开 >5000 行文件连续输入 | 行表增量 + 传入 `self.tree.as_ref()` | 1 万行文件实测击键到下一帧的时延 |
| **双实现漂移** | `ProblemsPanel` vs `ProblemsPane`、`CodeActionMenu` vs 自组 `Menu`，改一处不生效 | 任一侧改视觉或行为 | 先定真源，删另一份 | 组件库测试 + 宿主引用点收敛 |
| **`workspace/symbol` 半通** | 协商出能力却发不出请求，按「Core 已有」排期会落空 | 有人读 `feature_name_for_method` 就下判断 | 统一口径：**`validate_lsp_method` 放行才算有** | 白名单含该 method |
| **`git.write` 掩盖缺口** | 按命令名引用数评估会虚报 24 个 Git 缺口 | 直接用 §B1 的 0 值 | 先看是否走多路器，再看 operation 覆盖面 | operation 白名单对照 |
| **台账过期条目** | 重复排期 | 照抄台账 | 订正 §2.2 与 §2 计数 | 与代码行号对照 |
| **参数提示/选区格式化是三端都缺** | 当「平替缺口」排期会错估工作量与验收基准 | 未核对两端 | Windows `lsp-core-adapter.ts:1252-1259` 把 `lsp_get_signature_help`、`lsp_format_range` 列为显式不可用；macOS 检索命中 0 | 两端各自检索 |

### 假设与限制（可能推翻结论的证据）

1. **「引用数为 0」不等于「用户完全无法触发」**。`git.commit` 命令名引用 0，但 `commit` operation 走 `git.write` 是通的（`changes.rs:467`）。我已按 operation 口径修正 Git 部分，但**其余命令（`git.stashes`/`blame`/`diff` 等）未逐一确认是否存在多路器旁路**；它们不在 26 种 operation 里，故判为真缺口。
2. **未验证 JDTLS 实际声明的能力集**。若服务端未声明 `renameProvider`/`referencesProvider`/`documentFormattingProvider`，接线完成后仍拿不到结果。这是运行验证，**未做**。
3. **未验证组件库 widget 在真实数据下的渲染**。`SignatureHelp`（`hints.rs:246`）、`SymbolOutline`（`symbols.rs:162`）只确认了导出与构造签名。
4. **引用计数是静态全词匹配**，不区分类型引用 / 测试引用 / 注释。`FindWidget` 的 2 处我已逐条看上下文确认为注释；其余高命中项（`Keybinding` 47、`RecentProjects` 57）未逐条核实。
5. **台账只复核了 4 处**，其余条目未逐条核对，可能还有同类过期项。
6. **`lithe-ui` 有 659 项测试**（选型笔记 §3 所述）可作回归网，但我**未运行**，不知道当前是否全绿。

---

## 建议与验收

### 排期（我负责的架构维度）

| 序 | 行动 | 依赖 | 人日 | 完成标准 |
|---|---|---|---|---|
| **0** | 订正台账 §2.2（Git 身份页已接线）与 §2 的 Action 计数 | 无 | **0.5** | 台账与代码一致；顺带复核其余条目 |
| **1** | 补 5 项 LSP 接线（文档格式化、重命名、引用、实现、类型定义） | 无（Core 已就绪） | **1.5** | 5 条 `NotWired` 换 `Action`；`service.rs` 新增 5 个方法；`S1_JAVA_*` 打点；**删掉对应 `MISSING_LSP_REQUESTS` 使用点** |
| **2** | 接编辑器显示开关（缩略图/空白字符/行号）+ 4 个行编辑命令（切换注释/复制行/删除行/移动行） | 无（Core `editor.lineEdit` 与组件库 builder 齐备） | **2.0** | 7 条换 `Action`；`minimap()`/`show_whitespace()`/`set_line_numbers` 有调用点；`editor.lineEdit` 引用数 0→4 |
| **3** | 接 `FindWidget`（Ctrl+F / Ctrl+H） | 无 | **1.0** | 两条 Action 的 handler 不再返回 None；删掉 `menu_bar.rs:2175` 注释 |
| **4** | 接 `go_back`/`go_forward` | 无（实现已在） | **0.5** | 去掉两处 `#[allow(dead_code)]`；有 action 定义与键位 |
| **5** | **文本模型增量化**（行表 + tree-sitter 旧树复用） | 无 | **3.0** | 1 万行文件击键时延降到与文件长度无关；`lithe-ui` 现有测试全绿 |
| **6** | 取消语义架构设计（DAP + LSP 统一） | 需确认 Core 取消后一致性语义（受冻结口径约束） | **1.0**（设计） | 设计文档 + Core 缺口上报 |
| **7** | DAP 两项接线（`setVariable` + 异常断点） | 无 | **1.0** | 调试面板可改变量；两个命令引用 0→1 |
| **8** | Git 主动操作接线设计（`IntegrationOperation` → UI 契约） | **需界面设计师定进行中横幅/冲突提示的落点** | **1.5** | 能发起 merge 并在冲突时看到横幅 |
| **9** | 分支管理接线（create/delete/rename） | Core 已就绪 | **1.0** | `log_view.rs:1048/1099` 工具栏按钮不再是 `enabled=false` + 空闭包 |
| **10** | 大纲/符号树面板 | **依赖 Core 放行 `documentSymbol` 或复用 `java.structure`** | **1.0** | 出现符号树；`SymbolOutline` 有调用点 |

**架构维度合计 13.0 人日**（逐项相加：0.5+1.5+2.0+1.0+0.5+3.0+1.0+1.0+1.5+1.0 = 13.0；不含 UI 设计与实现工时）。

### 构建批次的约束（必须按此编批）

- 构建一次约 **15 分钟**，每天有效 3～4 次 ⇒ 功能**成批写完再一起验证**，不能一项一构建。
- 构建须绕开工作区：`CARGO_TARGET_DIR=D:/developmentProjects/rust/lithe-gpui-target`。
- 内存配方：`RUST_MIN_STACK=67108864`、`CARGO_PROFILE_DEV_DEBUG=0`、`-j 2`。
- **建议编批**：批次 A = 序 1+2+3+4（全是接线，互不依赖，一次构建全验）；批次 B = 序 5（文本模型，单独构建，因为它动的是所有编辑器行为的公共路径）；批次 C = 序 7+9+10；序 6、8 是设计，不占构建。
- **每批必须同步功能矩阵** `shared/platform-feature-matrix/features/<id>.json`，否则 CI 门禁挂。

### 验收方式

**启动日志的 `S1_*` 打点，不是截图**。每接一根线**必须同步删掉**对应的 `MenuItem::NotWired` 与 `MISSING_*` 使用点——留假入口违反「降级可见」纪律。验证结束后关闭 Lithe 进程并确认无残留。

### 架构口径建议（写给后续实现者）

1. **判定「Core 有命令」的唯一口径是 `validate_lsp_method` 放行**，不是 `feature_name_for_method` 或能力协商表。`workspace/symbol` 就是反例。
2. **`java` crate 保持不依赖 gpui** 是好设计，继续守住；新增语言服务时按同一形状（`shared` + 一个 provider crate）扩展，不要在 `editor` 里写语言分支。
3. **`navigation.rs` 是列口径的唯一换算点**，任何新的语义请求都必须经它换算，不要在别处再写一份 UTF-16 转换。
4. **不要在宿主重写组件库已有的 widget**。`ProblemsPane` 与 `CodeActionMenu` 是既成事实，新增能力前先查 `lithe-ui/src/editor/mod.rs` 的导出清单。

---

## 交接给主理人

### 需与其他职责核对的具体问题

1. **给界面设计师（陆井然）**：
   - 序 8（Git 主动操作）的**进行中横幅与冲突提示落点**由谁定？我这边只出接口契约（`IntegrationOperation` → UI 事件），界面结构不在我维度。
   - 序 2 的 3 个显示开关（缩略图/空白字符/行号）放在哪个菜单、是否要给状态指示？组件库 builder 已就绪，但入口归属是界面问题。
   - `MISSING_LSP_REQUESTS` 清空后，「参数提示」与「格式化选区」这两个**三端都缺**的入口：先不画（避免假入口），还是画成 `NotWired` 带角标？这是界面决策。
2. **给 UI 设计师（颜思齐）**：
   - `ProblemsPanel`（库）与 `ProblemsPane`（宿主自建，`crates/editor/src/problems.rs:172`）**谁是真源**？这直接影响视觉规范落在哪一侧。
   - `CodeActionMenu`（库）与宿主自组 `Menu`（`code_actions.rs:89`）同理。
   - 组件库零引用的装饰类 widget（`InlayHint`/`CodeLens`/`GitMark`/`DiffHunk`/`ReadOnlyBanner`）中，哪些属于视觉规范要求、哪些可以不做？我只能报「零引用」，不能定「该不该有」。
3. **给核心开发（娄码通）**：
   - 序 5 的文本模型增量化会改动 `lithe-ui` 的公共路径（`Buffer::replace`、`Highlighter::reparse`），**影响组件库全部编辑器测试**。我需要你评估实现风险与测试策略——我只出「改成什么样」，不写实现。
   - `Buffer::replace` 有 `assert!` 做整字符边界校验（`buffer.rs:266-273`）；增量行表维护必须保持这个不变量，否则会 panic。
4. **给扩展工程师（柯展之）**：`ide-mcp-connection` 与 `ide-api-plugins` 在 GPUI 侧关键词命中 0，但**我确认这属于你的维度**（插件契约在哪一层），我不越界判断。只提醒：`crates/extensions` 只有 298 行，是最小的 crate。
5. **给构建发布（常通关）**：我这份排期的构建批次约束（15 分钟/次、`CARGO_TARGET_DIR`、内存配方）沿用既有文档；**若你那边改了构建链路，我这边的批次假设要跟着改**。

### 缺失资料 / 未覆盖内容

- **未找到**：GPUI 侧任何形式的大纲面板、面包屑、编辑器分组（分屏）实现入口（检索 `breadcrumb`/`EditorGroup`/`PaneGroup`/`SymbolOutline`/`GoToSymbol`，命中 0）。
- **未找到**：GPUI 侧任何非 Java 的 LSP 会话启动点（`providerId` 全目录只有一个取值 `"java"`）。
- **未找到**：`signatureHelp`、`rangeFormatting`、`documentSymbol`、`prepareRename` 在 `lithe-core` 的任何实现（各命中 0）。
- **未覆盖**：`crates/terminal`（1434 行）、`crates/explorer`（1185 行）、`crates/notify`（877 行）、`crates/run`（1984 行）的内核结构我未逐模块分析——它们不在「编辑器内核/文本模型/渲染/LSP·DAP」这四项分工内。
- **未覆盖**：渲染层的性能细节（虚拟滚动、脏区重绘）我只分析了高亮路径；`lithe-ui` 的 GPUI Element 生命周期实现未审。
- **未覆盖**：`workspace.rs`（3975 行）与 `menu_bar.rs`（2766 行）只做了定点检索，未通读。

### 需要主理人裁决的两项

1. **台账订正**：§2.2 Git 身份配置页条目与 §2 的 Action 计数（51 vs 52）是否订正？我的证据是 `workspace.rs:883`（生产代码，所在 `pub fn new` 起 `:587`，文件首个 `#[cfg(test)]` 在 `:3395`）。
2. **矩阵如何登记 GPUI 端**：`shared/platform-feature-matrix/features/*.json` 只有 `macos`/`windows` 两列。既有文档 `gpui-parity-plan-2026-10.md:344-348` 把它列为待确认。我的排期里每一批都要求「同步矩阵」，**这个口径不定，批次 1 就无法合规交付**。建议新增 `gpui` 平台字段（而非记到 `windows` 端），因为本报告已证明 GPUI 与旧 Windows 端的能力面差异极大（LSP 5 vs 14 个 operation、Git 8 vs 26 种写操作）。

---

## 未运行验证清单（本报告全部结论的适用边界）

- **未运行任何 `cargo` 命令**：未构建、未跑 `cargo test`（含 `lithe-ui`、`lithe-core`、各宿主 crate）。
- **未启动 Lithe 进程**，无任何 `S1_*` 日志。
- **未验证 JDTLS 实际声明的 LSP 能力集**。
- **未验证组件库 widget 在真实数据下的渲染**。
- **未在真实仓库执行任何 git 写操作**。
- **未验证调试会话运行时行为**（断点命中、变量读取）。
- **未在 macOS / Linux 上验证**：本次为 Windows 主机静态分析，三端对照只依据仓库内代码与矩阵 JSON。
- **未运行** `scripts/verify-*` 系列脚本与 `node scripts/generate-platform-feature-matrix.mjs`。

⇒ 本文所有结论都建立在**静态检索与源码阅读**之上，属于架构层判断；任何涉及「实际能否工作」的结论都需要在目标平台上按 `S1_*` 打点验收。
