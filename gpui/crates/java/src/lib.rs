//! Java 语言服务的**宿主侧**：JDT LS 载荷发现 + Core LSP 会话 + JDT 工作区索引缓存。
//!
//! 阶段 10 第二批（`gpui/PLAN.md` §10）。第一批的跳转走 Core 的**无进程**轻量导航
//! （`lsp.builtinNavigation`），只在当前文件内、精度依赖文本启发式；本 crate 把同一条
//! 跳转链路的**结果来源**换成真实 JDT LS 的语义结果，于是跨文件 / 跨模块 / JDK 库源码
//! （`jdt://`）都能跳。
//!
//! ## 边界：这个 crate **没有**一行 LSP 协议、也没有一行 Java 语法
//!
//! 契约（`shared/contracts/rust-core-api.md:1162-1220`）把 LSP 运行时的所有权写得很死：
//! 「Rust owns the returned session's child process, stdin/stdout/stderr, framing buffer,
//! JSON-RPC request IDs, document versions, pending deadlines, capabilities, diagnostics,
//! and graceful/forced termination」。所以这里只做四件事：
//!
//! 1. [`jdtls`]：把 JDT LS 安装**发现**出来（可执行文件 / Equinox 启动 JAR / 配置目录 /
//!    Lombok agent / Java Debug 与 Java Test 扩展包 / 版本），再找出跑它的 JDK（≥ 21）。
//!    用户显式选的 JDK（设置页 `javaHomePath`）经 [`toolchain`] 这个注入点进来，
//!    是判据链的**第一条**；值用不了就降级回自动发现并打 `S1_JAVA_TOOLCHAIN`；
//! 2. [`workspace`]：算 JDT 工作区**索引缓存的键与目录**
//!    （`java.jdtWorkspaceFingerprint` → `lsp.jdtWorkspaceKey` → `cacheDirectory/jdtls/<key>`），
//!    并按 `java.jdtCacheRetention` 回收过期目录；
//! 3. [`session`]：Core LSP 会话的**同步信封**（`lsp.startServer` / `lsp.waitEvents` /
//!    `lsp.syncDocument` / `lsp.request`（含 `virtualDocument` 操作）/ `lsp.stopServer` +
//!    `lsp.destroyServer`）；
//! 4. [`service`]：把上面三样组成一个可被编辑器调用的门面（[`JavaLanguageService`]），
//!    并打 `S1_JAVA_*` 诊断行。
//!
//! [`events`] 是第 3 项的一部分：**事件泵**（`lsp.waitEvents` 的唯一消费者）+ 分派表 +
//! 诊断存储。泵是一个 `std::thread`，与 gpui 无关 —— 本 crate 刻意不依赖 UI 框架，
//! 这样它既能被编辑器调用，也能被没有窗口的测试直接跑（见 `service.rs` 的真实 JDTLS 用例）。
//!
//! ## 移植来源
//!
//! 发现与缓存这两块**逐条**移植 Windows host 的现成实现，不重新设计（`develop-lithe`：
//! 「复用成熟上游能力」）：`windows/tauri/src-tauri/src/lsp.rs`（jdtls 解析、内嵌 JDK、
//! 版本探测）与 `windows/tauri/src-tauri/src/lsp/jdt_workspace.rs`（缓存键、过期回收）。
//! 唯一的结构性差异是"Tauri `AppHandle` 的资源目录"换成 gpui 侧的"当前 exe 向上走"，
//! 以及缓存根目录的平台推导（gpui 没有 `app_cache_dir`）。
//!
//! ## 为什么是独立 crate
//!
//! 它不是编辑器的表现层：JDTLS 载荷发现、缓存目录回收、Core 会话生命周期都是**产品
//! 编排**，与 `editor` 的标签 / 光标 / 渲染无关。`editor` 只依赖它拿到"跳到哪里"。

mod events;
mod jdtls;
mod service;
mod session;
mod toolchain;
mod workspace;

pub use events::{JavaDiagnostic, JavaDiagnosticRange};
pub use toolchain::{JavaToolchainOverride, set_java_toolchain_override};
pub use service::{
    JavaCodeAction, JavaCompletionItem, JavaLanguageService, JavaPosition, JavaTarget, JavaTextEdit,
    file_uri,
};
