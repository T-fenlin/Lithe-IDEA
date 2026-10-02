//! 运行 feature：**运行配置 → 启动计划 → 真进程 → 输出**，以及「运行」工具窗。
//!
//! ## 职责边界（谁做什么，为什么这样分）
//!
//! | 层 | 做什么 | 不做什么 |
//! | --- | --- | --- |
//! | Rust Core（`runConfig.*`） | 从项目文件识别配置、合并分层、把一条配置投影成**平台无关的启动计划**（工具链引用 + 参数数组 + 工作目录 + 结构化环境 + classpath/modulepath） | 不拼 shell 命令、不返回可执行文件路径、不起进程（契约 `rust-core-api.md:1677-1690`） |
//! | 本 crate | **把计划落成一条真命令行**（解析工具链引用、按平台分隔符合并 classpath、先跑 `preLaunchSteps`）、起子进程、收输出、停进程、把配置与输出画成面板 | 不解析项目文件、不决定"该跑哪条配置"（那是 Core 的判据） |
//! | 外壳（`workbench`） | 登记"工具链在哪"（JDK / Maven 路径来自设置与探测）、把面板放进底部工具窗、标题栏与菜单的入口 | — |
//!
//! 「工具链在哪」是本 crate **拿不到**的信息：JDK 与 Maven 的发现属于平台适配器
//! （`develop-lithe`「Reuse mature developer tooling」与 `application-boundary.md`），
//! 而本 crate 只依赖 `shared`。所以走仓库既有的"低层 crate 定义钩子、外壳填实现"口径
//! （与 `TabMenuHostActions` / `GitIdentityHost` 同一形状）：见 [`ToolchainResolver`]。
//!
//! ## 为什么 `runConfig.generate` 而不是 `resolve`
//!
//! `resolve` 才是真源的两步里的第二步（真源是 generate → resolve），但它**必须先有**
//! `.lithe/run/generated.json`：`resolve` 第一步就是读那个文件，读不到直接
//! `WorkspaceNotFound`（`rust/lithe-core/src/execution/configuration.rs`），而本侧
//! **没有任何东西写过** `.lithe/run/*.json`（Core 明确"返回文档、由平台适配器决定要不要
//! 落盘"）。所以列表用 `generate`（自己走文件树探测，一次调用就够），
//! 启动计划用 `createLaunchPlan`（它内部会自己 resolve 一遍）。
//! 代价如实登记：`generate` 的 `source` 是清单文件而不是"哪一层"，见 `model::split_source`。
//!
//! ## 与真机的差异（本批未做，逐条列出而不是假装做了）
//!
//! 1. **没有配置编辑器**：真源点一条配置进 `RunConfigurationEditor`（改参数 / 环境变量 /
//!    JVM 参数）。本侧的列表与标题栏下拉都只做**选择**，不写 `.lithe/run/local.json`
//!    （Core 的 `runConfig.updateOptions` / `createUserConfiguration` 因此还没有消费方）；
//! 2. **没有断点与 DAP**：`🐞` 走的是 Core 的 `debugPort` 计划（JVM 以
//!    `-agentlib:jdwp=…,suspend=y` 起、等调试器接入），**没有** Java Debug Server 的
//!    DAP 会话 —— 断点、单步、变量、调用栈都还没有落点（下一步的入口是
//!    `vscode.java.startDebugSession` 拿适配器端口 + `debug.*` 一族命令）；
//! 3. **没有多配置并行**：一次只跑一条（真机可以同时跑多条并切换输出页签）。

mod model;
mod pane;
mod plan;
mod session;

pub use model::{
    MAX_PATHS, RUN_DIAGNOSTIC_TAG, RunConfigurationView, RunExecution, RunLayer, RunProjectView,
    load, parse_generate, parse_snapshot, provider_title,
};
pub use pane::{RunPane, RunPaneEvent};
pub use plan::{
    CommandSpec, PlanError, TOOLCHAIN_JAVA, TOOLCHAIN_MAVEN, ToolchainPath, ToolchainResolver,
    launch_command,
};
pub use session::{PreparedStart, RunSession, RunState};
