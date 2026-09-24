# Lithe 插件方案（GPUI 重写）

> 本文只服务 `gpui/` 新宿主的插件机制（路线图阶段 4 的"插件"工作项）。
> 上游决策见
> [`.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md`](../.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md)
> 的「插件方案（推荐：三层插件面）」（229–300 行）。本文把它写成可实现、可验收的方案，**不另起一套**。
> Zed 侧事实核对日期：2026-09-24（URL 见第 8 节）。

## 1. 先说结论

Lithe 插件最终采用**三层插件面**：**声明式贡献**（零代码，宿主校验并渲染）＋**WASM 纯逻辑**（可选，只算、不碰系统）＋**能力白名单加宿主代执行**（需要操作系统能力的部分由宿主执行）。插件**只贡献模块、能力与语言支持，不贡献界面**；只有"宿主无法代执行、且插件必须自己持有长生命周期状态"的插件（例如要长期持有 `gopls` 会话的 GoSupport）才新增第三种 `entrypoint.kind`，走独立进程。

- 证据：路线图 Note 229–272 行给出同一条结论；`rust/lithe-core/src/plugins/mod.rs:350`（`valid_entrypoint` 今天只接受 `builtIn` 与 `nativeBundle`）。

## 2. 三层插件面

| 层 | 能做什么 | 不能做什么 | 由谁执行 | 失败怎么表现 |
| --- | --- | --- | --- | --- |
| **第一层：声明式贡献** | 语言元数据（后缀、文件名、项目标记、注释符号）、主题、片段、语法查询、语义 token 规则 | 不执行任何代码；不改宿主行为 | 宿主（Core 校验＋合并，UI 渲染） | 加载期校验失败 → 该包标记失败并上报插件管理，**不阻塞必装模块启动** |
| **第二层：WASM 纯逻辑**（可选） | 算出该执行什么（`command`/`args`/`env`）、解析后端输出、把后端 token 映射成宿主样式 | 不能直接开进程、读写文件、访问网络；**读不到环境变量**，`cfg` 指令也不生效；不能加界面 | WASM 运行时执行，宿主逐项提供 API | 调用返回稳定错误码；宿主把该能力标为"不可用"，**不允许静默回退到内置 provider** |
| **第三层：能力白名单＋宿主代执行** | 声明"需要什么能力"，由宿主按白名单代为执行（进程、下载、凭据/网络） | 不能发起任意系统调用；不能超出用户已授权的范围 | 宿主代执行，插件只拿结果 | 未授权/越界 → 返回错误，**不允许静默降级** |
| **例外：独立进程**（仅第三层内） | 插件必须自己持有长生命周期状态时（语言服务器会话、数据库连接），按新 kind 起独立二进制 | 不用于纯计算、高频调用；粒度太粗的场景不用它 | 插件进程自己；协议复用宿主命令信封 | 进程退出/超时按模块 `failed` 处理，已持有资源计入租约 |

- 证据：路线图 240–272 行（三层内容、进程定位、失败必须显式）；`shared/contracts/application-boundary.md:51-64`（禁用模块不持有资源、失败可选包上报且不阻塞必装模块）、`88-95`（插件语言不得回退到内置进程 provider）；`shared/contracts/application-boundary.md:60-61`（模块状态集合）。
- 仓库已有独立进程先例：`rust/Cargo.toml:2` 的 `lithe-git-host`、`lithe-db-sidecar`、`lithe-db-mcp`。

## 3. 对标 Zed：借什么、为什么不整包照搬

核对到的 Zed 事实（出处见第 8 节）：扩展是含 `extension.toml` 的 Git 仓库；功能清单是语言、调试器、主题、图标主题、片段、MCP 服务器（另有 Agent 服务器）——**清单里没有界面这一类**；代码用 Rust 编译到 `wasm32-wasip2`，实现 `zed_extension_api` 的 `Extension` trait；能力只有 `process:exec`、`download_file`、`npm:install` 三种，用户用 `granted_extension_capabilities` 收窄或清空；语言服务器的 `language_server_command` 只返回 `command`/`args`/`env`，进程由宿主拉起；沙箱代价是 `cfg` 不生效、`std::env::var` 拿不到预期结果，必须改用 `current_platform()` 与 `Worktree`；发布走 Zed 扩展仓库，自 2025-10-01 起扩展仓库必须带指定开源许可之一，否则 CI 拒绝。

| 借 | 理由 | 不整包照搬 |
| --- | --- | --- |
| 清单 + 分层思路 | manifest 校验已在本仓库（`rust/lithe-core/src/plugins/mod.rs`），只差第三层的授权 | Zed 扩展定位是"给编辑器补语言支持"，Lithe 插件是 `dev.lithe.*` 完整功能模块，体量与生命周期不同（路线图 344 行） |
| "扩展不能加界面"这条边界 | 能力与功能清单里都没有界面；Lithe 的界面一致性由 `gpui-kit-design-guides` 保证 | 不引入 Zed 的注册表/许可流程原样：Lithe 今天只有 `sameTeamAsHost` 签名一种信任来源（`mod.rs:268`），注册表与许可门槛是待决问题 |
| LSP 由宿主拉起、扩展只给 `command/args/env` | 与 GoSupport 拉起 `gopls` 是同一件事，"要系统能力的部分不进沙箱"是通行做法 | 不把 WASM 当唯一插件形态：沙箱收益会被宿主自己开放的系统能力函数拆掉，且多一层长期 ABI（路线图 342 行） |
| 能力白名单 + 用户可裁剪 | 用户能收窄甚至清空授权，被裁剪后调用返回错误 | 不照搬 `npm:install`：Lithe 的插件依赖不是 npm 生态，需要替换成等价的"下载/工具链"能力类型（第 5 节） |

## 4. 从现有 manifest 迁移

**今天的事实**（都要保留识别，不能让已装插件静默消失）：

- `entrypoint.kind` 只有两个取值，非这两个值一律判为非法：`mod.rs:350-377`。
- 两个官方包都是 `nativeBundle`：`Plugins/mac/Official/GoSupport/plugin.json:17`、`Plugins/mac/Official/LinuxDoSupport/plugin.json:17`。
- `hostCompatibility` 是 `0.3.0` ~ `0.4.0`（两文件 7–10 行），`schemaVersion`/`apiVersion` 都是 1；Core 常量同为 1：`mod.rs:7`、`mod.rs:9`。
- `vendor.signatureRequirement` 目前只接受 `sameTeamAsHost`：`mod.rs:268`。
- Core 侧的 catalog fixture 是**空的**：`shared/fixtures/plugins/official-v1.json:5` 为 `"plugins": []`；带内容的样例是 `shared/fixtures/plugins/language-support-v1.json`（同样 `nativeBundle`，20–25 行）。

**迁移路径**：

| 现有形态 | 新形态 | 理由 |
| --- | --- | --- |
| `nativeBundle`（macOS 签名 Bundle，Swift/ObjC 桥） | 按插件职责分流：纯逻辑→第二层；要系统能力→第三层；要长生命周期状态→独立进程 | 路线图 339 行：为两个不贡献界面的插件永久保留一块 Swift，代价与收益不成比例 |
| `GoSupport`（语言服务器 + 执行/测试模块） | 独立进程（长期持有 `gopls` 会话） | 路线图 262 行把它点名为独立进程的典型场景 |
| `LinuxDoSupport`（社区模块） | 能力白名单 + 宿主代执行 | 它只贡献一个 `application` scope 模块与一个 `toolWindow` 贡献（`plugin.json:22-47`），没有自己的长生命周期进程 |
| `builtIn` | 保持不变 | `mod.rs:351-359`；路线图验收要求新 kind 加入后 `builtIn` 行为不变 |

**兼容策略（硬要求）**：

1. 过渡期内 Core 继续识别 `nativeBundle`；GPUI 宿主**显式拒绝并给出可操作提示**，不是静默失效（路线图 272、369 行）。
2. 声明式贡献在 Core 侧必须先校验再合并（`mod.rs:209-290` 已有排序、重复、宿主区间、语言模块归属校验），失败走 `PluginValidationError`。
3. 已装插件不能被"消失"：`application-boundary.md:62-64` 要求失败的可选包上报插件管理；`65-73` 要求成功加载的模块 ID 在进程生命周期内持久标记，非干净退出后下次启动先隔离——新宿主扫描 catalog 时必须沿用这套语义。
4. 新增 `entrypoint.kind` 要同步三处：Core（`mod.rs:350` 的 match）、Swift（`macos/Sources/LitheModuleAPI/Plugins/PluginTypes.swift:120-122` 的 `PluginEntrypointKind`）、后续 Windows/GPUI 侧。字段形状属于兼容面。

**schema 版本号处理**：

- 只增加一个新的 `kind` 取值**不需要**升 `schemaVersion`：`kind` 是字符串，合法性在 `valid_entrypoint`（`mod.rs:350`）判定，未知取值只让该包校验失败。
- 只有**新增/修改必填字段形状**才升 `PLUGIN_MANIFEST_SCHEMA_VERSION`；Core 对未知 `schemaVersion`/`apiVersion` 会以 `UnsupportedSchema`/`UnsupportedApi` 整体拒绝（`mod.rs:215-226`），因此升版必须同时给旧版迁移路径。

**仓库事实与上游 Note 的一处不一致（按文件为准）**：路线图 235 行称两个官方插件的 `contributions` 都是空数组，但 `Plugins/mac/Official/LinuxDoSupport/plugin.json:33-45` 实际声明了一个 `kind: "toolWindow"` 贡献，且带 `rendererID: "community.linux-do.browser"`（浏览器渲染器）。GoSupport 的两个模块确实是空数组（`GoSupport/plugin.json:39`、`:55`）。所以"插件不贡献界面"相对今天是**收紧**，不是维持现状；且 Core 的 `PluginPackageManifest`（`mod.rs:109-133`）根本没有 `contributions` 字段，该字段目前只被 Swift 侧消费。

## 5. 能力白名单

Zed 的三种能力及其粒度（`granted_extension_capabilities`）：`process:exec` 用 `command` + `args` 约束、`download_file` 用 `host` + `path` 约束、`npm:install` 用 `package` 约束；把列表清空后"很多扩展会变成不可用"，这是被接受的结果而不是回退到宽松授权。

Lithe 现有缺口：manifest 有 `providedCapabilities`（模块对外提供什么，见 `GoSupport/plugin.json:35-38`）与 `signatureRequirement`（谁签的），但**没有"用户可裁剪、按调用点校验"的授权层**；今天是"签名可信即可按宿主权限行事"（路线图 258、370 行）。Core 的 catalog 结构里也没有任何能力字段（`mod.rs:109-133`）。

建议的能力类型（**本节为实现建议，尚未经维护者确认**）：

| 能力 kind | 用途 | 授权粒度 | 裁剪方式 |
| --- | --- | --- | --- |
| `process:exec` | 拉起语言服务器、构建、调试器适配器 | 命令 + 参数白名单（对标 Zed） | 收窄命令或参数；清空则插件进程能力全部不可用 |
| `download_file` | 下载语言服务器/工具链产物 | host + path 白名单（对标 Zed） | 收窄到单一 host 或仓库路径 |
| `credential:read` | 读取宿主已存的凭据（**Lithe 专有，需确认**） | 凭据名称/域白名单，插件拿不到明文以外的范围 | 按凭据逐条授权/撤销；未授权即报错 |
| `network:http`（备选） | 插件自身发起网络请求（**Lithe 专有，需确认**） | host 白名单 + 方法 | 收窄 host 或整体关闭 |

共同规则：

- 授权是**三处一致**：manifest 声明 → 用户授权的实际集合 → 调用点校验；任一处不满足即失败。
- 失败走既有错误信封（`shared/contracts/rust-core-api.md:65-77`）：稳定错误码 + 面向用户的 message；错误码属于兼容面，不得随意改名。
- 不做静默降级：没有授权就不能改用内置实现或旧路径（`application-boundary.md:88-95` 同一条原则）。
- `signatureRequirement` 只决定"能不能装"，能力授权决定"装上以后能做什么"，两者不能互相替代（路线图 258 行）。

## 6. 宿主实现要点

**WASM 运行时（推荐，未最终确认）**：目标三元组用 `wasm32-wasip2`（与 Zed 一致）。候选是 wasmtime 与 Extism（路线图 342 行把两者并列为"用 WASM 承担全部能力"的被否方案）；建议选 **wasmtime + WASI Preview 2 组件模型**，因为它是 `wasip2` 目标最直接的宿主，且第二层只需要纯逻辑。**仓库内没有 wasmtime/Extism 的任何既有集成**（`rust/Cargo.toml:2` 的 workspace 成员只有 `lithe-core`、`lithe-git-host`、`lithe-db-sidecar`、`lithe-db-mcp`），所以这是新增依赖面，必须固定版本并提交 `Cargo.lock`（沿用 `gpui/README.md:213` 的钉版本约束）。

**宿主函数面**（第二层最小集，逐项等价于 Zed 的宿主 API）：平台信息（对标 `current_platform()`）、环境变量读取、`PATH` 内查找可执行文件（对标 `Worktree`）、返回 `command/args/env` 给宿主拉起、token→样式映射、日志输出。开出去多少就要维护多少兼容性（路线图 371 行）。

**接模块生命周期（9 状态）**：`disabled`、`inactive`、`activating`、`active`、`idle`、`preparingToSleep`、`sleeping`、`sleepBlocked`、`failed`（`application-boundary.md:60-61`）。插件模块必须满足：

- `disabled`：不实例化、不持有任务/定时器/监听/会话/连接/子进程（`:51-52`）。
- `activating`：只在其能力被请求后实例化（`:53`）；`sleeping` 停掉所有自有资源并释放实例（`:54`）；不可中断的工作持租约阻止睡眠并给出原因（`:55`）。
- `failed`：上报插件管理，不影响必装模块（`:62-64`）；已加载的进程型模块退出必须等操作系统进程真正退出，强制停止失败要作为活跃资源继续可见（`:96-99`）。

**接命令信封**：插件协议复用 `shared/contracts/rust-core-api.md:46-56` 的请求形状（`id`、`operationId`、`timeoutMilliseconds`、`command`、`payload`）与 `:65-77` 的响应/错误形状，不新造一套。必须保留 `operationId`、取消、超时与陈旧结果语义（取消/超时分别返回 `cancelled`/`timed_out`，`:58-63`）。独立进程的 IPC 也用同一信封（路线图 270、280 行）。

**贡献点与界面**：宿主只定义**声明式**界面贡献点（例如工具窗位置、顺序、图标），由宿主用 gpui-kit 组件渲染；插件不得注入 HTML。若将来确有自定义界面需求，仍由宿主定义贡献点（路线图 286–288 行）。三端业务逻辑不写 `#[cfg]` 平台分支（`gpui/README.md:214`）。

**工具链**：交付插件 SDK 与 `create`、`package`、`dev` 三段；`dev` 必须能脱离宿主独立启动示例插件，否则插件生态无法增长（路线图 199、281、372 行）。

## 7. 开放问题（需要维护者决定）

1. **WASM 运行时与 ABI 冻结时点**：wasmtime 还是 Extism；组件模型版本；ABI 一旦冻结后的兼容承诺多久。
2. **能力类型全集与默认授权**：是否引入 `credential:read`、`network:http`；安装时默认授予什么；是否允许"清空全部授权"（Zed 允许，代价是多数扩展不可用）。
3. **`nativeBundle` 过渡期的长度与终点**：是永久保留识别、还是给一次"迁移向导"，以及 0.5.2 用户在三种选择（显式拒绝 / 提示迁移 / 自动禁用）里取哪种组合。
4. **第三种 `entrypoint.kind` 的命名与进程协议**：`sidecar` 之外是否还有别的形态；传输用 JSONL 还是 framed；心跳与重启策略。
5. **注册表、许可与签名**：是否自建插件注册表、是否像 Zed 那样强制指定开源许可、`sameTeamAsHost` 之外是否接受第三方签名。
6. **两份 manifest 形状是否统一**：包级 `plugin.json` 用 `modules` 数组，而 Core catalog 用 `moduleIDs` 数组（`mod.rs:128`），同一条信息有两种形状。
7. **`contributions` 的归属**：它今天只被 Swift 侧消费、Core 完全不解析；新宿主是否把它纳入 Core 校验（建议纳入，否则贡献点无法在 Core 侧做确定性合并）。
8. **语言支持与内置模块的边界**：`application-boundary.md:77-82` 规定 AI/Terminal/Git/Search/Local History/Debug/Java 是内置模块、不是市场插件；插件化这些能力是否在范围内。

## 8. 核对到的外部来源与仓库文件

外部（均为 2026-09-24 实际抓取）：

- <https://zed.dev/docs/extensions/developing-extensions>：`extension.toml`、功能清单、`wasm32-wasip2`、`zed_extension_api` 与 `Extension` trait、`cfg` 与 `std::env::var` 限制、`current_platform()`/`Worktree`。
- <https://zed.dev/docs/extensions/capabilities>：`granted_extension_capabilities`、`process:exec`/`download_file`/`npm:install` 的粒度与示例、清空授权的后果。
- <https://zed.dev/docs/extensions/languages>：语言元数据 `config.toml`、tree-sitter 查询与语义 token、`language_server_command` 只返回 `command/args/env`。
- <https://zed.dev/docs/extensions/publishing/license-requirements>：2025-10-01 起扩展仓库必须带指定许可之一，否则 CI 失败。
- <https://zed.dev/docs/extensions/publishing/overview>：发布流程（许可 → 提交到 Zed 扩展仓库）。

仓库：

- `rust/lithe-core/src/plugins/mod.rs`（schema/API 常量、entrypoint 校验、语言支持校验、排序与重复约束）
- `rust/lithe-core/src/tests/plugins.rs`、`shared/fixtures/plugins/official-v1.json`、`shared/fixtures/plugins/language-support-v1.json`
- `Plugins/mac/Official/GoSupport/plugin.json`、`Plugins/mac/Official/LinuxDoSupport/plugin.json`
- `shared/contracts/application-boundary.md`（模块生命周期与插件不变量）、`shared/contracts/rust-core-api.md`（命令信封与错误形状）
- `shared/fixtures/modules/built-in-v1.json`（10 个内置模块，含 `dev.lithe.workspace` 必装）
- `macos/Sources/LitheModuleAPI/Plugins/PluginTypes.swift`、`macos/Sources/Lithe/Platform/MacOS/Plugins/MacNativePluginLoader.swift`
- `gpui/README.md`、`gpui/PLAN.md`（步 12 = 插件，Zed 式 WASM）

**未能确认**：`Plugins/win/` 目录存在但没有任何插件包（`Plugins/**/*.json` 只匹配到 mac 两个包），因此 Windows 侧插件现状无从核对；`wasmtime`/`Extism` 在仓库内无任何既有集成或依赖记录（全文检索 md 仅命中路线图的方案讨论）；Zed 文档未给出运行时内部实现细节（是否自研 WASM 宿主），故"Zed 用什么运行时"未确认。
