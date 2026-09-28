# Core: Execution & Support Subsystems

Core 的较小一半：运行配置、调试、AI、GitHub、插件、社区、诊断，以及编辑器逐行变换。它们都遵循同样的模式——**在 Core 做计划，在宿主做 I/O**。

## `execution/` —— 运行配置和项目检测

`execution/configuration.rs` 持有 schema、分层覆盖和确定性生成逻辑。`execution/types.rs` 持有共享分类类型。两个模块承担特别重的约束：

**`execution/launch_command.rs`** 会把组装好的启动命令压进 OS 限制内（Windows: 32 767 UTF-16 units），并重写成 JDK `@argfile`。说明是“Core 拥有文本，宿主拥有文件”（`:10-14`）。对应决策：
`.agents/notes/implemented/bug-fix/2026-09-20-oversized-java-launch-command.md`。

**`execution/detectors/`** —— 12 个 detector，共用一个有界 walk。`detectors/scan.rs` 持有遍历逻辑，所有 detector 都必须用它；自己递归 walk tree 的 detector 会被视为 bug（`detectors/mod.rs:1-12`）。这些 detector 包括：`npm`、`python`、`go`、`cargo`、`make`、`gradle`、`maven`、`compose`、`procfile`、`shell`。

| Detector | 分类规则 |
| --- | --- |
| `npm` | 使用**lockfile 命名**的包管理器；继承工作区包管理器；`bun` 脚本保留 `bun`，不吞掉 `node` |
| `gradle` | 只读构建脚本，不启动 Gradle，也不求值。支持 Kotlin DSL、从 build root 找 subproject tasks、不同父目录下同名 module、忽略没有 start task 的 build、忽略注释掉的插件 |
| `maven` | 只从声明的 reactor 和应用插件推断 |
| `compose` | 报告为**infrastructure**，不是应用服务 |
| 其他 | 模块根、常规布局、框架声明 |

`src/tests/detectors.rs` 中有两个不变量：**每个**内置 detector 都必须产出一个启动计划（`:289`），并且 detector 不能声称一个已经被 `maven.scan` 产出过的 id，且不能破坏它的配置（`:398,440`）。重复名称会按目录做限定（`:1555`）。

`RunConfiguration` 分层是 `project environment → overrides → generate`，并包含三类缓解机制，全部有对应测试（`src/tests/run_configuration.rs`）：旧生成器 revision 会被失效（`:74`），project-scoped toolchain 路径必须保持相对且在项目内（`:1940`），被记录后已删除的条目会被丢弃（`:2544`）。

## `debug/` —— 传输中立的 DAP

- `engine.rs` —— “有状态的 DAP reducer，字节传输和进程生命周期保留在平台侧。”断点、步进过滤器、scopes 和 variables 的顺序都确定性排列（`:174,208,245,337`）。
- `protocol.rs` —— 有界 `Content-Length` 帧 + JSON helper，不依赖原生传输。
- `types.rs` —— 稳定请求、更新、事件、检查结果。
- `java_test.rs` —— 确定性的 Java 测试启动配置。
- `breakpoint_relocation.rs` —— 在 UTF-16 编辑中跨文本变更移动 source breakpoints。

Fixtures：`shared/fixtures/debug/`（8 个文件）。JSON 边界上的 round-trip 测试覆盖，包括一个必须解码为真实 framed `initialize` 请求的 base64 `outboundFrames` payload（`src/tests/protocol.rs:19-61`）。

## `ai/` —— 提交信息生成

两个纯模块：

- `configuration.rs` —— 解析外部提供的配置文本，“不读取文件或进程环境”。
- `generation.rs` —— 校验提交输入，**预算 diff**，并转换支持的 provider wire protocol。

宿主负责 I/O 和 secrets（`ai/mod.rs:1`）。`ai/tests.rs` 覆盖配置导入、secret 边界和提交 wire protocol。契约：`shared/contracts/ai-commit.md`，带自己的一套 `AI_COMMIT_*` 错误码命名空间。

## `github/` —— 请求规划

它保留 REST 路径、payload、响应字段和错误翻译，让不同产品端完全一致；传输和凭证仍由平台负责（`github/mod.rs:3-5`）。

确定性很明确：`BTreeMap` 作为 query parameters（`:99`，“Deterministically ordered query parameters”），labels 按 name 排序，assignees 按 login 排序（`:193-196`）。

两个安全规则有测试：可能改变已规划路径的远程组件会被拒绝（`src/tests/github.rs:26`），HTTP 失败时会使用稳定错误分类，并且**绝不把响应体泄露到 `details`**（`:213`）。

## `plugins/`、`community/`、`diagnostics/`

- `plugins/` —— 清单解析、兼容性检查、使用 `BTreeMap` / `BTreeSet` 确定性合并。宿主不兼容时会被确定性拒绝（`src/tests/plugins.rs:47`）。
- `community/discourse.rs` —— 每个宿主共享的 API-key 授权。
- `diagnostics/` —— “纯确定性清洗 + 清单整理。没有文件系统或进程访问”（`:6-8`）。`redaction.rs` 处理文本规则，`manifest.rs` 组织 bundle。此设计允许任意宿主导出 support bundle，而不泄露凭证或机器路径。

## `editor/` —— 逐行文本变换

`editor/line_edit.rs` 是纯函数：文本 + selection -> replacement + range + selection，基于 **UTF-16 code units**。`editor/tests.rs` 固定了 IDEA 语义和 PR #642 的回归修正。模块文档注明迁移状态：macOS 已接入这里，且这些 fixture 是 Windows 实现的参考（`editor/mod.rs:1-7`）。

## 宿主侧的 4 状态异步契约

Core 统一在 `application-boundary.md:18` 中说明，并且只在一个地方实现。每个 gpui view 都会按自己的命名镜像这套规则：

| 层 | 枚举 | 位置 |
| --- | --- | --- |
| Core | `ProjectPreparation { status }` | `lsp/languages/project_preparation.rs:8-43` |
| Git log pane | `LoadState { Loading, Ready, Stale, Failed, NoRepository }` —— `Stale` 表示“有数据，但最后一次刷新失败” | `lithe-db-gpui/crates/git/src/model.rs:249-261` |
| Git commit inspector | `FilesState { Idle, Loading, Ready, Failed }` | `lithe-db-gpui/crates/git/src/model.rs:408-417` |
| Explorer | `LoadState { Loading, Ready, Failed(String) }` —— 没有 `idle`，因为空根直接短路到 `Ready` | `lithe-db-gpui/crates/explorer/src/explorer_view.rs:130-141` |
| Source control | `LoadState { Loading, Ready, Failed(String) }` | `lithe-db-gpui/crates/git/src/changes_view.rs:213-219` |
| Settings run page | `RunPageMode { NoProject, Failed, Loading, Ready }` | `lithe-db-gpui/crates/settings/src/dialog.rs:561-570` |

## 参考资料

- `rust/lithe-core/src/execution/`（全部文件）
- `rust/lithe-core/src/debug/`（全部文件）
- `rust/lithe-core/src/{ai,github,plugins,community,diagnostics,editor}/`（全部文件）
- `rust/lithe-core/src/tests/{detectors,run_configuration,github,plugins,ai}.rs`
- `rust/lithe-core/src/lsp/languages/project_preparation.rs`
- `shared/contracts/{application-boundary,ai-commit,github}.md`
- `shared/fixtures/debug/`、`shared/fixtures/ai/`、`shared/fixtures/github/`
- `lithe-db-gpui/crates/{explorer,git}/src/`、`lithe-db-gpui/crates/settings/src/dialog.rs`
