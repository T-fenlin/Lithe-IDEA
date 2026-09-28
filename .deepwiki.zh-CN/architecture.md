# 架构

Lithe 被拆分为两部分：一个是确定性的命令表面（`rust/lithe-core`），另一个是单一的 GPUI Kit 宿主（`rust/gpui/crates/*`）。设计里最关键的事实，就是把这两者严格分离。

## Crate 图

```
                    ┌──────────────────────────────┐
                    │  lithe-gpui-app  (bin Lithe) │  组合根
                    │  CLI、窗口边界、启动流程     │
                    └───────────┬──────────────────┘
                                │ app → workbench, app → settings
             ┌──────────────────┼───────────────────────┐
             ▼                  ▼                       ▼
   ┌──────────────────┐  ┌──────────────┐      ┌──────────────┐
   │ lithe-gpui-      │  │ lithe-gpui-  │      │ lithe-gpui-  │
   │ workbench        │  │ settings     │      │ (java, notify │
   │ shell layout     │  │ model+theme  │      │  via shell)  │
   └────────┬─────────┘  └──────┬───────┘      └──────┬───────┘
            │                   │                     │
   ┌────────┴─────────┐         │                     │
   ▼        ▼   ▼     ▼         │                     │
 editor  explorer git terminal │                     │
   │        │     │     │       │                     │
   └────────┴─────┴─────┴───────┘                     │
                        │                             │
                        ▼                             ▼
              ┌─────────────────────────────────────────────┐
              │ lithe-gpui-shared                           │
              │ core_client · icons · i18n · workspace_config│
              └──────────────────────┬──────────────────────┘
                                     │ Rust crate，直接链接
                                     ▼
                        ┌────────────────────────┐
                        │ lithe-core             │  无 GPUI，无平台层
                        │ protocol · project ·   │
                        │ lsp · git · execution  │
                        │ debug · ai · github    │
                        └───────────┬────────────┘
                                    ▼
                        ┌────────────────────────┐
                        │ lithe-git-host         │  进程、管道、askpass
                        └────────────────────────┘
```

已声明的方向是：`app → workbench → {editor, explorer, git, terminal} → shared`，以及 `app → settings → shared`（`rust/gpui/README.md:70-74`）。这已通过 `rust/Cargo.lock:4427-4531` 验证：整个图是 DAG，没有依赖向上回流。

**超出图示之外的边**，都是有意而且在清单中写明的：

| 边 | 允许理由 |
| --- | --- |
| `workbench → java` | 设置页的 `javaHomePath` 必须传递到语言服务，而 `java` 又不能依赖 `settings`。唯一调用点：`register_java_toolchain`（`workbench/src/workspace.rs:4582`） |
| `workbench → notify` | 通知中心的状态层属于 shell |
| `editor → notify` | `editor` 位于 `workbench` 下方，因此它只能使用非常窄的注入 `WeakEntity`（`editor/src/editor_view.rs:349`） |
| `editor → java` | 编辑器负责接线 JDTLS，所有协议代码仍留在 `java` 内 |
| `shared → lithe-core` | 这是进程内 Rust 链接，不是 C ABI（`shared/Cargo.toml:11-12`） |

注意：`lithe-gpui-java` 故意**没有** `gpui-kit` 依赖（`java/Cargo.toml:7-15`），它是纯数据 / 请求构造层，这也是它能够持有阻塞式 `std::thread` 事件泵的关键。

## 三条边界规则

**1. Core 是确定性的且无 UI。** `rust/lithe-core/` 中不包含 GPUI，也没有平台实现。它本质上是命令表面：请求 JSON 进来，返回 JSON 结果。那些必须保持纯净的模块会专门写在模块文档中，例如 `editor/line_edit.rs:6-7` 里写着 “The transforms are pure”，`diagnostics/mod.rs:6-8` 写着 “No filesystem or process access happens here”，`lsp/languages/jdt.rs:3-6` 写着 “The adapter is deliberately pure”。

**2. 功能模块绝不能直接和 Core 或其他功能模块相互交谈。** 每个 gpui feature 都拥有一个 `Entity` 驱动的视图，只能通过 `shared::core_client` 访问 Core，并且只能通过 shell 注入回调来与同级模块协作。具体来说，`Explorer` 接收一个 `on_open: Box<dyn Fn(PathBuf, &mut Window, &mut App)>`，而 shell 把 editor 闭包进去（`workbench/src/workspace.rs:1144-1158`）——因此 `explorer` 永远不会直接引用 `editor`。

**3. 上游拥有语言事实；Lithe 负责编排。** Java 符号、源码根、类路径、入口点和测试发现都来自 JDT LS，通过 Core 的 LSP 运行时返回；Core 只负责校验和标准化。验证器要求：`node scripts/verify-java-semantic-ownership.mjs` 若出现六个已废弃的本地入口 / 测试扫描标识符就会失败。

## Core 调用模式

`lithe_core::execute_json` 是**同步**的。宿主必须遵循三步，且文档在 `shared/src/core_client.rs:27-28` 中明确要求：

```rust
// 1. 先翻转状态，让 UI 可以显示 loading
// 2. cx.spawn -> cx.background_spawn -> 阻塞调用 Core
// 3. this.update(cx, ..) 将结果写回 UI 线程
cx.spawn(async move |this, cx| {
    let result = cx.background_spawn(async move { load_snapshot(&root) }).await;
    let _ = this.update(cx, |this, cx| { /* write + cx.notify() */ });
}).detach();
```

典型实例：`explorer/src/explorer_view.rs:289-332`。所有地方都遵循两个推论：

- **所有边界都要绑定。** `DEFAULT_TIMEOUT_MILLIS = 120_000`（`shared/src/core_client.rs:37`）；Git 明确使用 60 s（`git/src/model.rs:486-491`）；返回结果上限也显式写死（`MAX_FILE_SIZE = 2 MiB`，搜索 ≤ 10 000，符号 ≤ 50 —— `core/src/project/files.rs:33,203,295`）。
- **每个结果都要防止过期。** 宿主侧是单调递增计数器（`git/src/changes_view.rs:328-358`，`editor/src/editor_view.rs:693-735`）；Core 侧则是 `operationID` 所有权（`core/src/project/document_lifecycle.rs:35,128`）以及“stale preview”错误，用于审查式 Git 变更。

## 确定性规则

在 `shared/contracts/application-boundary.md:11-19` 中写明：

- 每个进程 / 语言边界都使用 UTF-8 JSON。
- 工作区路径在所有宿主上都必须是**相对路径且使用 `/` 分隔**（`core/src/project/files.rs:715-722`；`invalid_relative_path` 在 `core/src/protocol/error.rs:80-87` 中会先把 `\` 规范化为 `/`，再拒绝 `..`、`:`、NUL 和绝对路径，因此 Windows 风格路径无法在别处绕过校验）。
- 产品显示行是 1-based；编辑器 / LSP 位置是 0-based 行号 + UTF-16 列号；缺失位置用 `null` 表示。
- **列表必须有确定性排序**，这样 fixture 可以按字节逐一比较。
- 每个异步操作都暴露 `idle` / `loading` / `ready` / `failed`。
- 失败必须带稳定的 `code` + 用户可见 `message`；平台细节放在 `details` 中。

十一种稳定错误码位于 `core/src/protocol/error.rs:11-34`：
`invalid_request`、`workspace_not_found`、`permission_denied`、`not_supported`、`runtime_missing`、`process_start_failed`、`process_failed`、`parse_failed`、`cancelled`、`timed_out`、`unknown`。宿主在 `notify/src/severity.rs:59-75` 中把它们映射到不同严重程度，并刻意把未知错误降级为 `warning`，而不是直接宣称 `error`——因为“不要误报严重程度”。

## 所有权表

| 路径 | 拥有 |
| --- | --- |
| `rust/lithe-core/` | 命令、模型、校验、JSON 信封、确定性排序 |
| `rust/lithe-git-host/` | Git 子进程、管道、AskPass 传输、受限清理 |
| `rust/gpui/crates/app/` | 组合根、CLI、窗口尺寸、启动顺序、资源 |
| `rust/gpui/crates/workbench/` | Shell chrome、面板布局、项目标签页、根切换 |
| `rust/gpui/crates/editor/` | 缓冲区、标签页、导航、诊断、补全、代码操作 |
| `rust/gpui/crates/explorer/` | 项目树 |
| `rust/gpui/crates/git/` | 源代码控制 + 提交日志视图、分支数据、Git 身份 |
| `rust/gpui/crates/terminal/` | 进程会话、ANSI 清理、滚动缓冲 |
| `rust/gpui/crates/java/` | JDT LS 发现、工作区索引缓存、会话信封、事件泵 |
| `rust/gpui/crates/settings/` | 设置模型、持久化、主题、设置对话框 |
| `rust/gpui/crates/notify/` | 通知存储和严重程度映射 |
| `rust/gpui/crates/shared/` | Core client、图标、i18n、`.lithe/` 工作区配置 |
| `shared/` | 契约和 fixture——文档，不参与编译 |
| `infra/` | 数据库校验用 docker compose |
| `third_party/` | 上游 manifests（固定修订 + 校验和），无 vendored 代码 |

中文权威版本在 `.agents/notes/implemented/architecture/2026-09-13-repository-ownership-and-sharing-boundaries.md`。

## 打开项目的数据流

```
main.rs
  parse_options -> resolve_launch_root (recent-projects.json)
  lithe_gpui_settings::load()                       # 纯文件读取
  gpui_kit::init -> Theme::change -> init_store -> watch_lithe_themes
  cx.spawn(open_window) -> ShellWorkspace::new(root, ..)
       ├─ Entity<notify::Store>                     # 先创建：editor 会拿到 WeakEntity
       ├─ Entity<EditorPane> -> prepare_java(root)  # JDTLS 计划，按需启动会话
       ├─ Entity<Explorer>        on_open = |p| editor.open(p)
       ├─ Entity<ChangesView>                            # 构造时不拉取；首次展示时才读取
       ├─ Entity<TerminalPane>                          # 首次可见时才建立会话
       ├─ Entity<BottomPane>   (git log)
       ├─ register_java_toolchain(settings.javaHomePath)
       └─ prepare_workspace_config -> .lithe/project.json (UUID v4)
  Root::new(workspace)                                 # 必须是窗口的第一层
```

`ShellWorkspace::new` 只在两个地方被调用：启动路径里（`app/src/main.rs:1030`）和项目切换时的根替换路径（`workbench/src/workspace.rs:3042`）。

## 参考资料

- `rust/Cargo.toml`, `rust/Cargo.lock`
- `rust/gpui/README.md`
- `rust/gpui/crates/*/Cargo.toml`
- `rust/gpui/crates/shared/src/core_client.rs`
- `rust/gpui/crates/app/src/main.rs`
- `rust/gpui/crates/workbench/src/workspace.rs`
- `rust/lithe-core/src/protocol/error.rs`, `contracts.rs`
- `rust/lithe-core/src/project/files.rs`
- `rust/lithe-core/src/project/document_lifecycle.rs`
- `rust/gpui/crates/notify/src/severity.rs`
- `shared/contracts/application-boundary.md`
- `.agents/notes/implemented/architecture/2026-09-13-repository-ownership-and-sharing-boundaries.md`
