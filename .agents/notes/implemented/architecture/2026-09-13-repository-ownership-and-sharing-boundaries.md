# Agent 笔记：仓库所有权与共享边界

状态：已实现

## 先说结论

Lithe 现在只有一个宿主（`rust/gpui/` 里的 GPUI Kit 应用）和一份确定性业务逻辑（`rust/lithe-core`）。目录不只是组织方式，它就是架构所有权信号：写代码前先确认这件事归哪个 crate。判定顺序是**先问它是否确定性、是否被两个以上功能需要**——是就放 Core，否则留在拥有它的功能 crate。

## 问题

旧仓库有两套彼此独立的平台产品（macOS SwiftUI/AppKit 与 Windows React/Tauri），只有少量契约相连。维护两个产品时，平台代码会互相泄漏，同一行为会被实现两次，界面层也容易绕过应用层直接调底层能力。

那两套前端已删除，只留下 `rust/gpui/`。所有权问题没有消失，只是换了一种形态：现在的风险变成"功能 crate 之间互相伸手"，以及"确定性逻辑被塞进界面 crate，导致无法测试"。

## 决策

仓库采用以下所有权边界：

| 路径 | 职责 |
| --- | --- |
| `rust/lithe-core/` | 确定性命令、模型、校验和 JSON 命令信封 |
| `rust/lithe-git-host/` | 原生 Git 子进程、管道、临时输入和有界清理 |
| `rust/lithe-db-mcp/`、`rust/lithe-db-sidecar/` | 数据库辅助进程与 MCP 服务 |
| `rust/gpui/crates/app/` | 组合根：命令行参数、窗口尺寸、启动顺序、设置加载与主题应用 |
| `rust/gpui/crates/workbench/` | 工作台外壳：标题栏、项目标签、活动栏、状态栏、中央列 |
| `rust/gpui/crates/editor/` | 编辑器表现、缓冲区、跳转、诊断、补全 |
| `rust/gpui/crates/explorer/` | 项目树 |
| `rust/gpui/crates/git/` | Git 功能状态与视图 |
| `rust/gpui/crates/terminal/` | 终端会话、ANSI 解析与渲染 |
| `rust/gpui/crates/java/` | JDTLS 会话、workspace 指纹、Maven 上下文 |
| `rust/gpui/crates/settings/` | 设置模型、持久化、主题与设置对话框 |
| `rust/gpui/crates/notify/` | 通知中心的 store 与模型 |
| `rust/gpui/crates/shared/` | 跨 crate 原语：图标、i18n、Core 客户端、workspace 配置 |
| `shared/` | 契约和夹具，不放编译实现 |
| `infra/` | 仓库级开发和验证基础设施 |
| `third_party/` | 固定版本的上游清单和必要的局部源码补丁 |

gpui 各 crate 的依赖方向固定，只向下：

```text
app ──> workbench ──> {editor, explorer, git, terminal} ──> shared
app ──> settings ──> shared
```

`workbench` 不得依赖 `settings`；`shared` 不依赖任何功能 crate；任何 crate 都
不得依赖 `app`。越界依赖属于架构问题，不是风格问题。

Rust Core 内部同样分层：

```text
rust/lithe-core/src/
├── protocol/    # 命令名、线协议、响应、错误、事件和取消
├── runtime/     # JSON 分发器
├── project/     # 文件/搜索、本地历史、Markdown、Maven 项目检查
├── execution/   # 运行配置、启动/工具链模型和项目探测器
├── languages/   # Java 等语言相关的源码检查
├── git/         # Git 校验、解析、状态和变更
├── lsp/         # 通用 LSP、轻量回退和提供者适配器
└── tests/       # 按相同领域组织的命令级测试
```

依赖方向是 `protocol <- 领域包 <- runtime`。领域模块可以使用协议契约，但不得
依赖运行时分发器。`execution/types.rs` 是配置和探测器共用的类型层。各包的
`mod.rs` 是兼容性门面；新的实现逻辑必须放入明确归属的子模块。

Core 与功能 crate 的职责划分：

| `rust/lithe-core` | 功能 crate |
| --- | --- |
| 工作区遍历和搜索规则 | 根目录选择和目录监听 |
| UTF-8 文件命令校验和结果 | 原生文件 API、权限和持久化路径 |
| Git 模型、校验、解析和变更 | 可执行环境和凭据 |
| 历史元数据和快照规则 | 历史存储位置和文件移动 |
| 语言提供者目录、轻量能力、完整 LSP 运行时、Maven 和 Java 源码解析 | 语言服务器/JDK/Maven 探测，Maven/Debug 子进程 |
| 错误码、取消、截止时间和 JSON 信封 | PTY、信号、句柄和全部界面 |

界面只能依赖功能模型，不得依赖具体适配器。`rust/lithe-core/` 必须保持不依赖
GPUI、任何 UI 框架、`Process`、平台文件 API 或网络客户端。语言工具还遵循协议层
与应用层分离的规则，具体说明见
[语言工具分层与 LSP Runtime 归属](2026-09-13-language-tooling-and-lsp-runtime-ownership.md)。

变更必须保留这些兼容性表面：JSON 命令名、Serde 字段名、错误码、模块 ID、
能力 ID、插件入口名称。物理移动源码时不许重命名它们。

`rust/lithe-core/` 下的第一方生产模块必须以简短的英文 `//!` 模块边界说明开头。
这是 Rust 源码注释规范，不代表工程文档使用英文。导出的 API、共享请求和响应
类型、核心领域类型使用 `///`；不安全入口必须记录指针所有权和 `# Safety`
要求。实现中的注释只解释兼容性、确定性、排序、安全、性能等不明显的约束。

原生 Git 适配器 `rust/lithe-git-host/` 负责子进程、管道、临时输入文件，以及
进程组或 Windows 作业对象的限时清理，**不负责** Git 参数策略或界面模型。
共享事件解码和凭据脱敏留在 `rust/lithe-core/src/git/`。新增 Git 操作应复用此
执行边界，不能在视图或功能 crate 里直接启动进程；否则取消时容易漏掉凭据助手
等子进程。具体取舍见
[Git 执行与项目控制台](2026-09-12-git-execution-and-project-console.md)。

不得提交 `target/`、`.artifacts/`、夹具构建目录和本地 IDE 配置等生成物。
`third_party/` 不是通用的上游代码归档目录；除非 Lithe 实际编译经过记录的局部
补丁，否则应使用不可变清单、构建期校验下载和产物级许可证说明。

## 考虑过的备选方案

### 让所有功能 crate 都能直接调用 `lithe-core`

少写一层间接，调用点更短。但 Core 之后会变成"什么都往里塞"的抽屉：解析、
排序、文件遍历、进程启动混在一起，既无法单独测试，也让 Core 的确定性承诺
失效。因此 Core 只收**跨功能复用且必须确定**的行为，其余留在功能 crate。

### 把工作台和功能合成一个大 crate

省掉 crate 之间的接线。但那样一来，"这个行为归谁"就不再由目录回答，而要靠
读代码；改动的影响面也无法从依赖图上看出来。保持按功能分 crate，是为了让
所有权边界可以被机械检查。

### 保留两个宿主，让 gpui 只做增量替换

能降低一次性风险。但两个宿主意味着两套发布流水线、两套本地化、两套图标
真源，而 gpui 已经接管了主要界面。旧宿主在删除前没有任何 gpui 尚未覆盖的
能力路径，继续保留只是双份维护成本。

### 在 `docs/` 保留一份相同架构文档

这样便于人类查找，但会形成两个潜在事实来源。当前决策和理由统一保存在本
Note 中；正式契约、源码、夹具和验证脚本分别作为各自表面的权威来源。

## 后果

- 仓库对 Rust Core、各 gpui 功能 crate、共享契约、脚本和基础设施建立了明确的
  所有权边界，并且这个边界可以从目录和 Cargo 依赖图直接读出来。
- 新增跨功能行为时，先判断它是否确定性：确定性就放 Core 并补契约与夹具，
  否则留在功能 crate。不要为了"看起来统一"把平台行为下沉。
- 跨越边界的变更需要同时检查 Rust Core、契约和所有消费者。
- Rust Core 承担确定性行为的中心职责，必须保持 JSON、错误码和取消语义兼容。
- 目录布局成为架构的一部分。允许移动代码，但改变所有权或兼容性表面时必须
  形成明确的决策记录。
- 仅涉及当前事实的架构变化可以原地更新本 Note；如果决策本身被完全替代，
  必须创建新 Note 并归档本 Note。

## 验证

- `./scripts/verify-rust-core.sh`
- `./scripts/verify-rust-core-comments.sh`
- `./scripts/verify-shared-contracts.sh`
- `node scripts/verify-java-semantic-ownership.mjs`

仓库布局和所有权规则还需要通过受影响 crate 的构建与测试验证。契约变化必须
同时检查相关夹具和所有消费者。

## 适用范围

- `rust/lithe-core/`
- `rust/lithe-git-host/`
- `rust/gpui/crates/`
- `shared/`
- `scripts/`
- `infra/`
- `third_party/`

> 旧前端（`macos/`、`windows/`、`frontend/editor/`、`Plugins/`）已删除；删除前的
> 布局与决策记录见 git tag `legacy-frontends-final`。
