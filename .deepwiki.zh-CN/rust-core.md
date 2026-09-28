# Rust Core

`rust/lithe-core/` 是确定性的命令表面。对外有一个统一入口：

```rust
// rust/lithe-core/src/lib.rs:25-27
pub fn execute_json(request: &str) -> String { runtime::execute_json(request) }
```

它大约有 82k 行代码，覆盖 125 个文件，约 930 个公开符号。gpui 宿主直接链接这个 crate，并在后台线程中同步调用它。

## 模块地图

| 模块 | 职责 | 可见性 |
| --- | --- | --- |
| `protocol/` | 命令名、响应模型、错误码、事件、取消 | 全局 re-export |
| `project/` | 工作区遍历、搜索 + 索引、本地历史、文档生命周期、Markdown、Maven | crate-private |
| `lsp/` | 通用 LSP 客户端 / 引擎、JDT LS 策略、进程内语言能力 | crate-private |
| `languages/` | Java / Spring / MyBatis 源码检查，与 LSP 传输无关 | crate-private |
| `runtime/` | JSON 分发和（已退休的）C ABI | crate-private |
| `execution/` | 运行配置生成、12 个项目探测器、启动命令长度处理 | crate-private |
| `debug/` | 与传输无关的 DAP 状态机、Java 测试启动、断点重定位 | crate-private |
| `git/` | 大约 6900 行 Git 检查 + 修改、控制台折叠、事件流 | crate-private |
| `ai/` | Provider 配置解析、提交消息请求规划 | `pub mod ai` |
| `github/` | REST 路径、payload、响应计划、错误翻译 | crate-private |
| `plugins/` | 清单解析、兼容性、目录合并 | `pub mod plugins` |
| `community/` | Discourse API-key 权限校验 | crate-private |
| `diagnostics/` | 包清理 + 清单整理（纯函数，无 I/O） | crate-private |
| `editor/` | 基于 UTF-16 code unit 的逐行文本变换 | crate-private |

有关详细内容，请参阅：[Core: Git](rust-core/git.zh-CN.md)、[Core: Language Tooling](rust-core/language-tooling.zh-CN.md)、[Core: Execution & Support](rust-core/execution.zh-CN.md)。

## 信封结构

请求 — `protocol/command.rs:9-27`，字段全部是 `camelCase`：

| 字段 | 含义 |
| --- | --- |
| `id` | 关联 id，复制到响应中 |
| `operationId` | 协作式取消和过期结果处理；默认等于 `id` |
| `timeoutMilliseconds` | 支持超时的操作截止时间 |
| `gitExecution` | 宿主持有的 Git 配置，仅作用于这个请求 |
| `command` | 稳定命令名，由 `CoreCommand::parse` 解析 |
| `payload` | 缺失时会按 JSON `null` 反序列化 |

响应 — `protocol/contracts.rs:10-21`：

```rust
pub struct CoreResponse {
    pub id: Option<String>,
    pub ok: bool,                        // 判别式：data XOR error
    pub data: Option<ResponseData>,      // 为 None 时省略
    pub error: Option<CoreError>,        // 为 None 时省略
}
```

`CoreResponse::failure` 会强制将 `data: None`（`:48-55`）——失败绝不携带部分成功的数据。

`CoreCommand`（`command.rs:34-305`）**没有 `Serialize` 派生**。它只是保存命令名到域名的映射，而 `CoreCommand::parse`（`:310-448`）是唯一定义映射的位置。它的文档很明确：它解析的是“兼容命名”，并且“不接受别名或大小写变体，否则不同宿主可能产生不一致行为”。新增一个命令意味着要新增 variant，并且要补上一条 `match` arm；这两部分都由 `command.rs:451-542` 覆盖。

## 分发流水线

`runtime/dispatcher.rs` 是一个 2300 行的 `match`。顺序很重要：

1. `:54-63`：解析 JSON → `invalid_request`，并把 serde 错误放进 `details`。
2. `:66`：`operationId` 默认等于 `id`。
3. `:67-70`：为当前线程安装一个 `cancellation::Scope`。
4. `:71-85`：注册 Git 执行事件、提前取消检查、Git 执行作用域。
5. `:79-85`：`CoreCommand::parse`，未知命令 -> `not_supported`，并把命令名写入 `details`。
6. `:87-2232`：每个 arm 先 `from_value::<XRequest>`，再调用域函数，最后返回 `success` / `failure`。
7. `:2233-2240`：**最终门禁**——如果领域函数返回后才发现该操作已被取消或超时，那么成功结果会被降级为 `cancelled` / `timed_out`。

因为闭合点唯一，所以任何域都不能自行发明一套代码形态。

## 取消、超时、过期结果

`protocol/cancellation.rs` 是按线程作用域的全局注册表：

```rust
State { cancelled: Arc<AtomicBool>, deadline: Option<Instant> }   // :12-15
type Registrations = HashMap<String, Vec<Arc<AtomicBool>>>;      // :19  （这是 vector，不是单 token）
thread_local! static CURRENT: RefCell<Option<State>>             // :22-24
```

vector 很关键：一个嵌套请求如果复用了同一个 `operationID`，也不会注销掉仍然活着的调用者（`:17-18`）。`Scope::begin` 仅在 `milliseconds > 0` 时安装 deadline（`:53-55`）；`Drop` 期间会恢复上一个状态，并且只移除自己的 token（通过 `Arc::ptr_eq`，`:66-81`）。`check()`（`:96-116`）会返回 `cancelled` 或 `timed_out`，并且在发生超时时还会设置取消标志，从而让下游清理也停止。

检查点显式存在于命令边界和工作区遍历点：`project/files.rs:385,400,574,628` 和 `project/search_index.rs:367`。

**过期结果保护有三层**，必须明确分辨自己处于哪一层：

| 层 | 机制 | 位置 |
| --- | --- | --- |
| Core，确定性 | `DocumentLifecycleState::Saving` 携带 `operation_id`；如果完成者 owner 不匹配，就返回 `IgnoreStaleResult` | `project/document_lifecycle.rs:35,128,253-281` |
| Core，Git | 预览 → 写入。若本地 HEAD 或目标分支变化，就返回固定错误，例如 `"Git push preview is stale; refresh and try again."` | `git/mod.rs:5459`；rebase `git/rebase_session.rs:753`；rewrite `git/rewrite.rs:608,724` |
| 宿主 | 单调递增 `request_serial` / `generation` / `revision`；非当前值会被丢弃 | `git/src/changes_view.rs:328-358`，`editor/src/editor_view.rs:693-735` |

## 事件

有两种机制。`protocol/event.rs:10-24` 定义了 `CoreEvent`，但真正的在线流来自 Git：`git/execution_events.rs:16` 安装了 `EventSink`，每条事件都带有 `operationId`（`:21-25`）。事件种类：`requestStarted`、`requestFinished`、`started`、`output`、`finished`（`:34-65`）。边界：每行 16 KiB，诊断最大 512 KiB，最多 4096 条事件（`:12-14`）。

`ObserverScope`（`:78-88`）会恢复上一层 sink，因此嵌套请求不会继承外层 observer；`deliver`（`:105-110`）则在回调外层包一层 `None`，确保同步的 `git.authRespond` 不会篡改观察对象的身份。这个嵌套场景的测试在 `git/execution_events.rs:537-599`。

## C ABI 已退休

`include/lithe_core.h` 仍然导出 7 个函数，全部在 `runtime/ffi.rs` 中实现：`lithe_core_version`、`lithe_core_git_askpass`、`lithe_core_execute_json`、`lithe_core_execute_json_with_events`、`lithe_core_lsp_provider_catalog_json`、`lithe_core_cancel`、`lithe_core_free_string`。

它们**没有调用方**。`shared/contracts/rust-core-api.md:3-11` 明确说明：C ABI 已退休，gpui 宿主直接链接 Rust crate；头文件和 `ffi.rs` 仅保留为有效导出历史兼容。该 crate 仍然构建成 `["rlib", "staticlib", "cdylib"]`（`Cargo.toml:9`），因此将它视为历史兼容层。

后果是：`lithe_core_git_askpass` 是唯一进入 `lithe_git_host::authentication::helper_main` 的路径，而且 gpui 宿主中没有注册 `LITHE_GIT_ASKPASS_MODE`，因此交互式 Git 身份验证目前还没有接通。见 [Supporting Crates](supporting-crates.zh-CN.md)。

## 测试

测试分两类，覆盖范围不同：

| 类型 | 能触达到 | 数量 / 例子 |
| --- | --- | --- |
| `rust/lithe-core/tests/*.rs` | 仅能调用 `lithe_core::execute_json` —— 真正的黑盒测试 | 2 个文件。`git_push.rs` 真实调用 `git` 二进制，针对临时裸仓库验证 reviewed-push 合约：确保存在 `--force-with-lease=refs/heads/main:<oid>`，不存在裸 `--force`，并在本地 HEAD 或 `branch.*.pushRemote` 改变后拒绝执行并返回稳定消息，没有记录 push。`git_watch_context.rs` 检查 `git.watchContext` 返回原生路径（去掉 `\\?\`，将 `\\?\UNC\` 改写成 `\\`） |
| `rust/lithe-core/src/tests/*.rs` | 能访问 `pub(crate)` 内部 | 15 个模块。`git.rs` 最大（约 4500 行，约 55 个测试）。模块内的 `#[cfg(test)]` 也存在于 `command.rs`、`cancellation.rs`、`dispatcher.rs`、`lsp/`、`git/graph.rs`、`project/document_lifecycle.rs`、`editor/tests.rs`、`ai/tests.rs`、`git/console/tests.rs` |

大约 90 个 JSON fixture 位于 `shared/fixtures/`，用于锁定线下协议；其实只有 `lithe-core` 读取它们（见 [Contracts & Fixtures](contracts.zh-CN.md)）。

## 参考资料

- `rust/lithe-core/src/lib.rs`
- `rust/lithe-core/src/protocol/{command,contracts,error,event,cancellation}.rs`
- `rust/lithe-core/src/runtime/{dispatcher,ffi}.rs`
- `rust/lithe-core/include/lithe_core.h`
- `rust/lithe-core/Cargo.toml`
- `rust/lithe-core/tests/{git_push,git_watch_context}.rs`
- `rust/lithe-core/src/tests/`
- `rust/lithe-core/src/project/document_lifecycle.rs`
- `rust/lithe-core/src/git/execution_events.rs`
- `shared/contracts/rust-core-api.md`
