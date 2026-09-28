# Core: Language Tooling & Project Preparation

这部分是 Lithe 与 JVM / JDT LS 交互的核心。目标不是“历史包围”，而是把文件系统、依赖与最终的 LSP 调用分开：Core 负责项目准备、workspace 组织和语言服务调用；宿主负责 UI、windowing 和实际 process 行为。

## 模块地图

| 文件 | 职责 |
| --- | --- |
| `languages/mod.rs` | language provider registry，按文件类型分配 runtime / provider / symbols |
| `languages/project_preparation.rs` | “准备工作区”状态机：Idle / Preparing / Ready / Failed |
| `languages/jdtls.rs` | JDT LS process 参数和 LSP 运行对象 |
| `languages/gradle.rs` | Gradle / Kotlin / Maven 触发器 |
| `languages/java.rs` | Java file/URI 语义与 `workspaceRoot` 计算 |
| `languages/inspection.rs` | 语法检查、diagnostic classification、`quick fix` 改写 |
| `languages/hover.rs` | 定义悬停和 symbol 解析，有确定性的 fallback |
| `languages/completion.rs` | completion kind 与 `textEdit` 抽象 |
| `languages/code_actions.rs` | 代码操作 `edit` 的整理，保证 UTF-16 端点闭合 |
| `languages/indexing.rs` | project index / build output / symbol cache |
| `workspace/` | project manifest、workspace snapshot、外部 tooling root |
| `resources/` | language providers, JSON schema, plugin manifest |

## project preparation

`project_preparation.rs` 是唯一带状态的入口：

```
ProjectPreparationStatus {
  Idle,
  Preparing,
  Ready,
  Failed(String)
}
```

这个状态机会在**宿主从窗口打开到编辑器 ready** 之间控制 JDT LS 的启动窗口。关键行为：

- 失败不等于“永远失败”。`Failed` 只是当前 attempt 的终结状态；如果 user 重新打开同一 workspace 或重试更改配置，状态允许再次进入 `Preparing`。
- `Ready` 只在叫 `languageProviders` 的结果里表明**至少存在一个 provider**，而不是“所有语言都好”。这意味着 Java provider 可能 ready，而做法依赖于 fallback 逻辑单独判定是否可用。
- 语义有两个层次：project preparation 是框架状态；installation / runtime detection 由 `lithe-db-gpui/crates/java` 完成，并通过 `settings` 重新注入。

`workspace detection` 只读取**受控目录集合**；未被显式写入的目录不会被认为是有效 worktree root。`workspaceRoot` 规范化规则是：

- 以 `.` 结尾的唯一相对路径被拒绝；
- `..` 是错误；
- Windows dir names 如果具有 `\\?\` verbatim 前缀就被规范化成 `/` / 统一形式；
- 反向路径和 UNC share 需要同一套 `normalize_path` 函数。

这个一致性非常重要，因为 JDT LS 的实际 config 与 gpui 侧缓存根都依赖同一套路径 normalization，见 `lithe-db-gpui/crates/java/workspace.rs` 和 `shared/workspace_config/paths.rs`。

## Java provider

Java 语言服务被显式分成两部分：

1. `JavaLanguageService`（宿主侧，gpui crate）——负责 `lsp.startServer`, `waitEvents`, `shutdown`, 日志洪峰和诊断泵。
2. `lithe-core` 语言 provider（Core）——负责**project selection**、运行时构造、`workspaceRoot` 和 capabilities。

其中 provider 的关键组件：

- `ProviderCapabilities`：定义 provider 需要哪些 `WorkspaceCapability`s；
- `JavaRuntime`：JDK detection result；
- `ProjectModel`：workspace root 的解析结果；
- `SymbolInfo`：对 editor navigation 的统一符号索引。

负载构造时最重要的字段是 `project_id`、`workspace_root`、`java_home`、`jdtls_home`、`cache_key`、`debug_enabled`。这个对象在每次 `openWorkspace` 时被重新生成，避免启动额外串联的会话。`project_id` 必须与 `.lithe/project.json` 的 `id` 对应，否则 same workspace 的多个 project id 会被当成不同缓存 bucket。

## Completion / code action / diagnostics contract

这些调用的安全边界是“UTF-16 columns”必须对齐：

- `completion`：`textEdit` 端点交给宿主 `editor::Position` 转换；
- `code_action`：`edit` payload 必须在执行前转换成当前 rope 的 **index / text edits**；
- `diagnostics`：`line`, `character` 是**字符列**，并在宿主映射回 UTF-16；
- `hover`：symbol 解析的 `range` 必须在当前 revision 之内，否则抛弃。

这套约束在 `lithe-db-gpui/crates/editor/src/navigation.rs` 与 `diagnostics.rs` 中被反复用到；Core 只保证**语义输出**，不保证宿主 UI 直接使用原始 `LSP` 结构。

## Determinism guardrails

| 事项 | 规则 |
| --- | --- |
| `completion` order | `lsp.completion` results 按 `sortText` / `label` / `insertText` 作为稳定 tie-breaker；无排序时保留原顺序 |
| `diagnostics` | 每次 change 只保留最新快照；旧快照永远不 merge |
| `symbols` | path + name + kind + range 进入 `BTreeSet`；有序输出可复现 |
| `project discovery` | `BTreeMap` or sorted `Vec`；directory enumerations 走 sort by path |
| `jdtls detection` | 不对 path 进行 locale-sensitive sorting；否则 Windows 上 verbatim path 可能错误落入 JDK 目录 |

### 这不是“用户可见的 UI 规则”

Language tooling 以 “project detection / java runtime / workspace cache / diagnostic snapshots / completion edits” 为中心；真正的渲染和菜单逻辑都由 gpui 侧处理。Core 只负责**稳定结果**和**协议边界**。

## 参考资料

- `rust/lithe-core/src/languages/`（全部文件）
- `rust/lithe-core/src/workspace/`（全部文件）
- `rust/lithe-core/src/protocol/contracts.rs`
- `rust/lithe-db-gpui/crates/java/src/`（全部文件）
- `rust/lithe-db-gpui/crates/editor/src/navigation.rs`、`diagnostics.rs`、`completion.rs`
- `shared/contracts/application-boundary.md`
- `shared/contracts/language-providers.schema.json`
