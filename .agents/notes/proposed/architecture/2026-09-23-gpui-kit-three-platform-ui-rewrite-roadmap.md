# Agent 笔记：用 GPUI Kit 统一三端界面的重写路线图

状态：提议中

> **状态变更（2026-09-25，维护者决定）**：
> **实现框架不变：界面继续用 GPUI Kit 重写**（`rust/gpui/`）。变的是**界面规格来源** ——
> 从 macOS SwiftUI 改回 **Windows 前端**（`windows/tauri/src/features/*`，Tauri v2 + React + Tailwind + shadcn）。
>
> 直接影响：
> - **尺寸、布局、观感一律以 Windows 前端为准**；macOS 那份调研（`rust/gpui/UI-MAP.md` 与 `.artifacts/ui-map/01..09`）降级为
>   **行为/功能对照**（它功能最全，用来查"这个交互原本怎么工作"），不再当视觉规格；
> - `rust/gpui/UI-MAP.md` 需要**重新以 Windows 为源做一份逐区域对应表**；执行提示词见
>   `docs/development/gpui-ui-windows-prompt.md`；
> - 仍然成立：已发布的 `gpui-kit 0.6.x` 与技能文档（描述 0.7.0）的差异、"跨端契约不动、宿主只做平台能力"这条边界、
>   以及 `UI-MAP.md` §1.3 里那些用真机踩出来的 gpui-kit 实现规则（行高、编辑器高度、滚动条、浮层挂层、DPI 与截图口径）。

## 先说结论

Lithe 现在有两套界面实现：macOS 是 SwiftUI/AppKit，Windows 是 React/Tauri，同一件事要做两遍。**长期目标**是用 GPUI Kit（Rust 原生 UI 框架）重写界面层，让 macOS、Windows、Linux 共用一套 Rust 代码，最终删除 SwiftUI 视图层和 React 实现。

**短期目标不是"开始写界面"，而是先证明两件事**：GPUI 宿主能直接驱动现有 Rust Core，以及 GPUI Kit 的编辑器能不能替代 Monaco。这两件事在头 10 周内用两个阶段验证完，任何一件不成立就应当停手，而不是继续投入。

关键前提是业务逻辑不需要重写：Rust Core 已经是语言中立的 JSON 命令信封（128 个命令 + 事件回调），所以真正的工作量是界面层，而不是整个应用。插件方向本文给出明确推荐：插件面按能力分成三层（声明式贡献、WASM 纯逻辑、能力白名单加宿主代执行），进程协议参考 DBX（`t8y2/dbx`）的 `backend.invoke` 模型与它的插件工具链，界面规则参考 Zed 的扩展模型（**扩展不能加界面**）；无论哪一层，插件只贡献模块与能力，不贡献界面。

## 问题

### 现状：同一件事做两遍

仓库里两个产品各自实现了一遍界面，且不共享界面代码：

| 区域 | 文件数 | 行数 | 说明 |
| --- | --- | --- | --- |
| `macos/` Swift | 742 | 184,036 | SwiftUI/AppKit 界面、平台适配、服务编排 |
| `windows/tauri/src` React/TS | 1,572 | 241,831 | Windows 工作台、功能状态与表现层 |
| 界面层合计 | 2,314 | 约 425,900 | 换框架后必须重写的部分 |
| `rust/lithe-core/src` | 122 | 76,922 | 确定性共享逻辑，**可复用** |
| `windows/tauri/src-tauri` | 21 | 12,706 | Tauri 组合根与 Windows 平台适配，部分可复用 |
| `windows/tauri/crates` | 14 | 3,103 | 平台 crate，可复用 |
| `frontend/` | 32 | 3,794 | 共享 Monaco 编辑器包等 |

macOS 是当前参考产品，Windows 是独立实现（见
[仓库所有权与共享边界](../../implemented/architecture/2026-09-13-repository-ownership-and-sharing-boundaries.md)）。
一个新功能要在两端分别实现、分别测、分别修；这类重复本身就是长期成本。

### 为什么现在有可能做成

Rust Core 的边界已经是语言中立的，这一点决定了重写的性质：

- 共享契约定义了统一的 JSON 命令信封（`id`、`operationId`、`timeoutMilliseconds`、`command`、`payload`），成功返回 `ok: true` 与 `data`，失败返回稳定错误码（见
  [Rust Core API 契约](../../../../shared/contracts/rust-core-api.md)）。
- C ABI 里有 `lithe_core_execute_json_with_events`，宿主可以直接拿到带事件回调的命令执行能力；Rust 宿主还能直接调用 `lithe_core::execute_json` 与 `lithe_core::cancel_operation`。
- 契约 Commands 表共列出 128 条命令，命名空间分布为 `git` 37、`debug` 18、`lsp` 18、`java` 11、`community` 7、`runConfig` 7、`maven` 6、`history` 6、`workspace` 5 等。
- 插件清单（manifest）的解析、兼容校验与目录合并**已经在 Rust Core 里**（`rust/lithe-core/src/plugins/mod.rs`，367 行），不是待办。

也就是说：GPUI 宿主是**第三个宿主**，而不是第三个产品。它要重新实现的是"怎么把命令结果画出来、怎么把用户操作变成命令、怎么管理窗口与生命周期"，而不是重新实现 Git、Java、LSP、Maven 的语义。

### 已经解决过的部分（不要重开）

- **编辑器曾经共享，但那条路线整体退役了**：旧前端时期 macOS 与 Windows 都用 Monaco 0.55.1、共用 `frontend/editor/`（`@lithe/editor`）。它投入过真实成本，也验证过「WebView 承载 Monaco」可行（那篇实验笔记已随旧前端归档，可在 git tag `legacy-frontends-final` 追溯）。旧前端与 `frontend/editor/` 一并删除后，gpui 侧编辑器是**原生 Rust 实现**（`rust/gpui/crates/editor/`，语法高亮走 `tree-sitter-java`），不再有 Monaco。
- **Windows 已经重写过一次界面**：此前的 Qt/C++ 实现已经退役，然后是 Tauri，最后是 gpui。历史上做过两次全量前端替换，说明这类替换在本仓库可行但昂贵，不是第一次。

### 本次要解决的新问题

用一套 Rust 界面代码同时覆盖 macOS、Windows、Linux，取消"macOS 一个界面、Windows 一个界面"的重复。这不只是换框架：它同时改动了编辑器技术栈、插件加载方式和平台适配边界，因此需要一份显式路线图和几个明确的决策门，而不是"边做边看"。

## 提案

### 长期目标

一年内得到一个**单套 Rust 界面代码覆盖 macOS、Windows、Linux** 的 Lithe：

1. 界面层全部由 GPUI Kit 构成，`macos/Sources/Lithe/Views` 与 `windows/tauri/src` 被替换后删除。
2. Rust Core 继续作为唯一业务真源，命令与事件契约不变；GPUI 宿主只是第三个消费者。
3. 原生插件改为跨平台形态，Windows 与 Linux 也具备插件能力。
4. 平台专属能力（进程、PTY、凭据、更新、对话框、文件监听）保留各自实现，但通过统一端口 trait 收敛，界面代码不出现平台分支。
5. 发布链路在实验验证通过、且两端功能对等之后才讨论；在此之前不动签名、更新通道与安装器。

### 短期目标（滚动更新）

P1 已完成（结果见阶段 1）。此后的短期目标按"**先界面、后地基、再编辑器**"的顺序推进，每条都要有可复验的产出：

| # | 目标 | 要回答的问题 | 完成标准 | 状态 |
| --- | --- | --- | --- | --- |
| S1 | **外壳布局基线** | 根视图、侧栏、编辑区、状态栏的布局约束怎么写才不塌？ | 一个窗口里同时出现标题栏、文件列表、标签页、编辑器、底部面板与状态栏 | **已完成（2026-09-23）** |
| S2 | 组件覆盖验证补全 | `List`、`Tree`、`Command`、`DataTable` 能不能用？ | 每个组件能跑通且渲染正确；关键缺口 ≤ 3 个 | **进行中**（`List` 已完成） |
| S3 | P2 宿主骨架 | 命令、事件、模块生命周期能不能对齐契约？ | 9 状态状态机有测试；禁用模块不持有资源；四个校验脚本仍通过 | 未开始 |
| S4 | P3 工作台外壳 | 能不能接真实数据做出可用外壳？ | 打开工作区、切换与拖动面板、命令面板、项目内搜索都可用 | 未开始 |
| S5 | 决策门 B（编辑器） | GPUI Kit 的编辑器能否替代 Monaco？ | 对等清单逐项有实测结论 | 未开始 |

**S1 结论：工作区用 `DockArea`，不是手搓 flex。** 六个区域（标题栏、左 `Project`、中编辑器、底部 `Terminal`、右 `Outline`、状态栏）同时在真实窗口中渲染，数据来自 `workspace.snapshot`（4,851 个文件）。实现见 `rust/gpui/shell/src/bin/shell-probe.rs`，证据 `.artifacts/p1/s1-final.png`。

**S1 期间确认的两条硬约束（都是踩过坑才拿到的）**：

1. **`window.viewport_size()` 返回物理像素，而 `px()` 是逻辑像素**。本机 `scale_factor = 1.25`：视口返回 1444x812，逻辑实为 1155x650。直接用视口值给根视图设高度会多出 1/scale 的高度，**底部状态栏被切出窗口** —— 这是本阶段所有"状态栏消失"现象的共同真因。正确写法：`viewport.height / window.scale_factor()`。
2. **窗口尺寸必须从显示器算出来**：不设 `window_bounds` 时 GPUI 默认 1536x1095 逻辑像素，在 125% 缩放的显示器上桌面只有 1536x864 逻辑像素，窗口比屏幕高，底部同样落在屏幕外。

**遗留（不影响 S1 判定）**：左栏文件列表尚未在面板内正确裁切与滚动（内容画出面板边界），应改用 gpui-kit 为长列表准备的 `List` / `v_virtual_list` 组件，而不是自己拼 `overflow_y_scroll` 的 div。归入 S2。

**界面参考（2026-09-24 改定，取代此前"照搬 Windows"的决定）**：不做独立设计，**以 macOS 端源码为界面规格**，用 gpui-kit 一比一复刻。

- 原因：macOS 端是功能最全、更新最快的实现（`macos/Sources/Lithe` 的 `Views/` 约 5.4 万行，覆盖编辑区、Git、数据库、运行调试、语言智能等），Windows 端是它的滞后镜像；拿最新最全的一端当规格，才不会把已经改掉的设计抄进来。
- **两步走**：先把 macOS 界面逐区域逆向成 `rust/gpui/UI-MAP.md`（区域组成 + 度量 + 每个元素 → gpui-kit 组件 + 缺口 + 优化点），再照它实现。区域调研原文在 `.artifacts/ui-map/0X-*.md`（不入库）。
- **实现只能用 gpui-kit 0.6.6 源码里真实存在的组件**（判断依据是已发布源码，不是在线文档——在线文档描述的是未发布的 0.7.0）。macOS 做得不好的地方允许用 gpui-kit 做得更好，但必须在 `UI-MAP.md` 的"优化"一栏写明理由。
- **Windows 端与产品截图降为旁证**，只用来交叉验证同一功能在另一端的取舍。此前的 Windows 结构分析（`main-layout.tsx` / `footer/` / `main-sidebar.tsx`）保留在下一段，作为旁证材料，不再是规格来源。

已确认的 macOS 外壳结构（`11` 号子代理调研 `macos/Sources/Lithe/Views/Workbench/WorkbenchView.swift` 等，细节见 `rust/gpui/UI-MAP.md` §2）：

```text
VStack
├── topBar（40pt，左侧 76pt 留给交通灯）
├── projectTabBar（38pt，可隐藏）
├── HStack
│   ├── activityBar（38 宽）
│   └── workspaceArea（尾部内缩 40pt）
│       └── 纵向 split
│           ├── 上：横向 split（左栏 320 | 编辑区，编辑区最小 400）
│           └── 下：工具窗（互斥单例，最小 260）
└── statusBar（24pt）
右侧悬浮栏与 40pt 右活动栏是 overlay，不占布局宽度
```

**这与 gpui-kit 的一个能力边界冲突，已定取舍**：gpui-kit 的 `DockPlacement::Bottom` 只横跨中心列（`gpui-base-0.6.6/src/dock/dock_area.rs:1415-1431` 把 bottom dock 挂在 center_frame 内），而 macOS 的底部工具窗横跨"左栏 + 编辑区"。因此本项目把底部工具窗放进 center 的 `v_split`（而不是用 Bottom dock）：结构上等价于 macOS，代价是失去 Dock 自带的底部停靠/拖拽能力，需要时再评估。

**旁证（Windows 结构，不再是规格）**：`windows/tauri/src/features/layout/components/main-layout.tsx`（外壳）、`footer/`（状态栏）、`sidebar/main-sidebar.tsx`（侧栏）。旧结论：设计 token 名与 gpui-kit 的 `Theme` 高度对应（`--sidebar`、`--title-bar`、`--status-bar`、`--tab-bar`、`--border`、`--background`、`--primary`）；这条对 macOS 同样成立，但 macOS token 有两个语义陷阱（`accent` = primary、`selection` 不透明），映射表在 `rust/gpui/UI-MAP.md` §1.2。

**已确认的一条硬约束**：不设 `window_bounds` 时 GPUI 默认窗口是 1536x1095 **逻辑**像素；在 125% 缩放的显示器上桌面只有 1536x864 逻辑像素，窗口比屏幕高 231px，**底部（状态栏）整体落在屏幕之外**。窗口尺寸必须从 `primary_display().visible_bounds()` 算出来。

**已实测的组件缺陷（1 个，未达"换框架"阈值）**：`h_resizable` / `resizable_panel` 能渲染出可拖动的分割线，但**不把确定高度传给子元素**，会按内容高度撑开祖先 —— 结果是文件列表铺满整个窗口高度、滚动条不出现、状态栏被顶到窗口外。需要真正"可调整面板 + 拖拽重排"时应评估 `dock`。

### 阶段 0：技能、分支与代码落点基线（已完成，仍可复验）

`gpui-kit` 与 `gpui-kit-design-guides` 两个技能已全局更新（`longbridge/gpui-kit`）。本次更新**不是空跑**：`gpui-kit` 27 个文件中 8 个内容变化。但更新后的技能文档描述的是**尚未发布的 0.7.0 API**，而 crates.io 上最新只有 **0.6.6**。P1 实测因此暴露了文档与已发布版本的错位：

```rust
// 技能文档给的写法（属于 0.7.0，在 0.6.6 上编译失败：crate 里没有 open_window）
gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| { /* … */ })

// 0.6.6 的真实写法（gpui-kit 自身 lib.rs 的文档示例；P1 已实测通过）
gpui_kit::application().run(move |cx| {
    gpui_kit::init(cx);
    cx.spawn(async move |cx| {
        cx.open_window(WindowOptions::default(), move |window, cx| {
            let view = cx.new(|cx| AppView::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx)) // Root 提供对话框、浮层与通知
        })
        .expect("failed to open window");
    })
    .detach();
});
```

同类错位还有三处已实测确认：`Application::with_assets` 在 0.6.6 不存在；滚动只有 `overflow_y_scroll`，没有文档里的 `overflow_y_scrollbar`；`Task` 没有 `then`，文档里的 `.then(cx.spawn(..))` 写法不成立，正确做法是在前台任务里 `await` 后台任务。

记录这条的原因很实际：**照抄技能文档会直接编译失败**。判断依据必须是"已发布版本的源码"，而不是技能文档；升级 gpui-kit 时这几处要重新核对。

技能基线里有三套与 GPUI 相关的技能。维护者于 2026-09-23 决定：**只以 `gpui-kit` 为准，其余 GPUI 技能一律不使用**。

- `gpui-kit`、`gpui-kit-design-guides`（`longbridge/gpui-kit`，框架官方）：**唯一可用**。应用入口、开窗、组件、主题、布局、Element、状态、测试一律以它为准。
- `gpui-elements`、`gpui-macros`（`onehr/gpui-skills`，第三方）：**明令禁用，不得作为编写依据**。它们教的是裸 GPUI（`gpui_platform::application()` 加 `cx.open_window(...)`），会跳过 `gpui-kit` 的 `Root` 挂载，导致浮层与弹窗失效；而 `gpui-kit` 自带的 `references/rust/gpui/` 已覆盖 action、async、context、element（含 advanced、api、patterns、best-practices、examples、id）、entity、event、focus、global、layout、test，没有需要外部补齐的缺口。保留安装仅供对照排查。
- `frontend-design`（`anthropics/skills`）：只当视觉方向的灵感来源。界面规则是否合格，仍然按 `gpui-kit-design-guides` 判定。

正确做法：应用入口、开窗、组件、状态、Element 与测试全部按 `gpui-kit` 写；需要自定义 Element 时读它自带的 `references/rust/gpui/element-*.md`。
错误做法：把 `gpui-elements` 的 `application().run(...)` 或 `gpui-macros` 的宏写法抄进正式代码；也不要因为第三方示例"看起来更短"就绕开 `gpui-kit`。

**分支与落点**：

- 在**当前工作区与当前分支**上开发，不新建分支：`rust/gpui/` 作为仓库根的新目录加入，现有产品目录不做改动。
- 新宿主作为**仓库根的独立 Cargo workspace**（例如 `rust/gpui/`），用 path 依赖引用 `rust/lithe-core`，不并入 `rust/` workspace。原因：`rust/Cargo.toml` 的 release profile 是为 Core 体积优化的（`strip`、`lto`、`codegen-units = 1`、`opt-level = "s"`），而桌面应用的 profile 需求不同；并入同一个 workspace 会让 `rust/Cargo.lock` 被 GPUI 的大依赖树污染，并拖慢现有 Core 构建与校验。
- 本方向落地后需要更新 `develop-lithe` 的所有权表与
  [仓库所有权与共享边界](../../implemented/architecture/2026-09-13-repository-ownership-and-sharing-boundaries.md)，把 `rust/gpui/` 登记为新的界面层；在此之前它只是实验目录。

### 阶段 1：冒烟验证（1–2 周）

- **目标**：证明"GPUI 宿主能直接驱动 Rust Core"。
- **工作项**：
  1. 在当前分支上新建 `rust/gpui/` workspace，添加 `gpui`、`gpui-kit` 与 `lithe-core` 依赖。
  2. 用 `gpui_kit::open_window` 打开窗口并挂载 `Root`。
  3. 进程内调用 `lithe_core::execute_json` 执行 `core.ping`。
  4. 执行 `workspace.snapshot`，把返回的相对路径列表画成一个可滚动列表。
  5. 按契约核对路径与编号规则：工作区相对路径用 `/` 分隔，界面行号一基，缺失位置为 `null`。
- **产出**：可运行的宿主二进制；两条命令的原始 JSON 响应；一张运行截图或录屏。
- **门槛**：`cargo build` 通过；两条命令返回真实数据，且字段与 Core 源码派生的形状一致；窗口开关无资源泄漏。注意 `shared/fixtures/core/` 下**并没有** snapshot 的 fixture，所以基准是"真实数据 + 契约字段对齐 + 原始响应留档"，而不是与 fixture 比对。
- **回滚点**：任一条不成立，记录失败原因并暂停本方向，不进入阶段 2。
- **P1 结果（2026-09-23，Windows）**：**通过**。构建成功（Rust 1.97.0 stable）；`core.ping` 返回 `protocolVersion=1`、`coreVersion=0.1.0`；`workspace.snapshot` 以本仓库为工作区返回 4,834 个文件、原始响应 1,016,586 字节，路径全为 `/` 分隔的工作区相对路径；两条命令合计 1,419 ms，后台线程执行期间窗口保持可交互。原始响应、窗口截图与构建日志在 `.artifacts/p1/`。**macOS 未验证。**

### 阶段 2：宿主骨架（3–5 周）

- **目标**：把"命令、事件、模块生命周期"三件事做对。这是后面所有功能的地基，做错要推倒重来。
- **工作项**：
  1. 统一命令入口：生成 `operationId`、超时、取消（`lithe_core::cancel_operation`）、陈旧结果丢弃。
  2. 事件回调路由：把 `lithe_core_execute_json_with_events` 的事件投递到 GPUI 实体状态。后台线程不得直接改 UI，必须经 GPUI 的异步任务回到前台。
  3. 模块生命周期状态机：9 个状态 `disabled`、`inactive`、`activating`、`active`、`idle`、`preparingToSleep`、`sleeping`、`sleepBlocked`、`failed`，以及租约与 provider 依赖规则，契约见
     [应用边界契约](../../../../shared/contracts/application-boundary.md)。状态机原本照 Swift 侧那套模块运行时实现（该 Note 已随 macOS 旧前端归档，可在 git tag `legacy-frontends-final` 追溯）；9 个状态名与租约/依赖规则属于跨端契约，不随实现一起消失。。
  4. 内置模块注册表：`workspace`（唯一 required）、`terminal`、`search` 三个先接通，其余 7 个（`ai-assistance`、`database`、`debug`、`execution`、`git`、`language-intelligence`、`local-history`）先登记为 disabled。
  5. 平台端口 trait：进程、PTY、文件监听、凭据、对话框。先各给 macOS 与 Windows 一份最小实现，界面代码只依赖 trait。
- **产出**：宿主骨架 crate、状态机测试、端口 trait 定义与两份平台实现。
- **门槛**：状态机 9 状态有测试覆盖；禁用模块不创建任何任务、定时器、监听器或子进程；休眠释放全部资源；`scripts/verify-rust-core.sh`、`scripts/verify-shared-contracts.sh`、`scripts/verify-service-boundaries.sh`、`scripts/verify-module-boundaries.sh` 仍然通过。
- **回滚点**：如果状态机语义无法在不改契约的前提下实现（例如租约规则需要改 `application-boundary.md`），先停下来讨论契约，不允许宿主私改语义。

### 阶段 3：工作台外壳（4–6 周）

- **目标**：把用户看到的框架画出来，暂不包含编辑器内核。
- **工作项**（括号内为 Windows 侧对应行数，用于排序）：
  1. 窗口与布局：`window`（6,389）、`layout`（5,139）、`panes`（3,926）、`tabs`（1,761）、`sidebar`（614）。
  2. 命令与输入：`command-palette`（3,372）、`keymaps`（5,770）、`notifications`（892）。
  3. 文件域：`file-explorer`（5,350）、`file-system`（7,258）、`viewer`（4,155）、`quick-open`（1,450）、`global-search`（3,465）。
  4. 设置只读展示：`settings`（12,334）先做读取与呈现，写路径留到阶段 5。
- **产出**：可交互的工作台外壳。
- **门槛**：能打开工作区、切换与拖动面板、开命令面板、搜索并打开文件；高频拖动满足
  [可调整界面性能边界](../../implemented/architecture/2026-09-13-resizable-ui-performance-boundaries.md)；
  不出现"界面在但后端连不上"的入口。组件选择与视觉规则一律按 `gpui-kit-design-guides`。
- **回滚点**：如果 gpui-kit 缺少关键组件（例如可拖拽标签页、嵌套 Dock、虚拟化数据表），记录缺口并评估自研成本；缺口超过 3 个就重新评估框架选择。

### 阶段 4：编辑器与插件（6–10 周，不确定性最大）

- **目标**：关闭两个决策门。
- **工作项**：
  1. 编辑器：按决策门 B 的清单逐项实测 GPUI Kit 的 `Editor`。
  2. 插件：按本文"插件方案"落地三层插件面。第一层（声明式贡献）与第三层（能力白名单加进程型模块）在本阶段交付，第二层（WASM 逻辑）视实际需求再启动；同时交付插件 SDK 与 `create`、`package`、`dev` 三段工具链，`dev` 必须能脱离宿主独立启动示例插件，否则插件生态无法增长。
- **产出**：编辑器对等实测报告；插件 ABI 的契约改动与两个官方插件的迁移。
- **门槛**：见决策门 B 与插件方案。
- **回滚点**：编辑器任一项不可替代 → 回退到"GPUI 宿主内嵌 WebView 承载 Monaco"，把结论写进本 Note 的后继决策；插件改动如果不兼容现有 0.5.2 用户 → 保留旧 `nativeBundle` kind 作为过渡。

### 阶段 5：功能面移植（16–24 周，需并行）

- **目标**：把功能面按批次搬完，每批次独立可验收。
- **批次划分**（括号内为 Windows 侧行数）：

| 批次 | 内容 | 主要目录 | 规模 |
| --- | --- | --- | --- |
| A | 版本控制 | `git`（29,787）、`github`（9,628）、`local-history`（879） | 约 4.0 万行 |
| B | 运行/调试/终端 | `run`（8,661）、`run-actions`（1,106）、`debugger`（2,730）、`terminal`（5,806）、`diagnostics`（1,369） | 约 2.0 万行 |
| C | AI 与数据 | `ai`（14,463）、`database`（8,602）、`docker`（3,519） | 约 2.7 万行 |
| D | Java 语义与项目模型 | `maven`（6,962）、`spring`（629）、`mybatis`（389）、`references`（423）、`outline`（1,055） | 约 0.95 万行 |
| E | 设置写入与剩余 | `settings`（12,334）、`remote`（659）、`wsl`（123）、`onboarding`（423） | 约 1.35 万行 |

- **门槛**：每批次的 128 个命令映射逐条有结论（"已对等"或"显式不支持"），不允许"看起来能用但走的是本地假实现"，这条沿用
  已归档的 Windows 产品功能对齐提案的教训（随 `windows/` 删除，可在 git tag `legacy-frontends-final` 追溯）；Java/Maven 相关事实必须来自所选后端，不得在宿主里重建第二真源。
- **并行要求**：批次 A–E 之间只共享宿主骨架与端口 trait，可以 2–3 条线并行。单线串行会超过一年，这一点在排期时必须显式决定。

### 阶段 6：三端构建与发布准备（4–6 周）

- **目标**：三端都能构建出可运行产物。
- **工作项**：macOS 打包、Windows 打包（沿用 `scripts/build-windows.ps1` 的经验）、Linux 打包（全新）、三端 CI lane、依赖与缓存策略（参考
  已归档的 CI 构建缓存与产物策略（随两个旧前端删除，可在 git tag `legacy-frontends-final` 追溯））。
- **门槛**：三端构建通过；不修改现有发布 workflow 与签名配置。
- **注意**：仓库目前**没有任何 Linux 支持**——没有 Linux 目录，CI 只有 `ci-macos.yml` 与 `ci-windows.yml`。第三端是从零开始，不是"顺便多编一个目标"。

### 插件方案（推荐：三层插件面）

**推荐**：不再把插件做成单一形态，而是按"是否需要操作系统能力"把插件面劈成三层：声明式贡献（零代码）、逻辑与适配（WASM，可选）、系统能力（能力白名单加宿主代执行）。无论哪一层，**插件都只贡献模块、能力与语言支持，不贡献界面**。这套分层参考了 Zed 的扩展模型，理由与实测代价见下文。

现状与证据：

- 官方插件只有两个：`Plugins/mac/Official/GoSupport`（Go 语言服务器与执行模块，`contributions` 为空）与 `Plugins/mac/Official/LinuxDoSupport`（社区模块）。
- **更正（2026-09-23，由 `rust/gpui/PLUGINS.md` 的调查发现并已复核）**：`LinuxDoSupport/plugin.json` 的模块**声明了一个界面贡献** —— `kind: "toolWindow"`、`placement: "rightSidebar"`、`rendererID: "community.linux-do.browser"`。也就是说 macOS 侧**今天就存在"插件贡献工具窗面板、由浏览器渲染器渲染"的机制**，本文早先"两个插件都不贡献界面"的说法有误。只有 GoSupport 的 `contributions` 是空数组。
- 另一点：Core 的 `PluginPackageManifest`（`rust/lithe-core/src/plugins/mod.rs`）**没有 `contributions` 字段**，该字段目前只被 Swift 侧消费。因此"插件不贡献界面"相对今天是**收紧**而不是维持现状，且已有插件面板在迁移时必须给出处置。
- manifest 的解析、兼容校验与目录合并**已经在 Rust Core 里**（`rust/lithe-core/src/plugins/mod.rs`），`PluginEntrypoint.kind` 目前只接受 `builtIn` 与 `nativeBundle` 两种取值，判定逻辑集中在文件末尾的一个 `match`。
- 唯一 macOS 专属的部分是 `nativeBundle` 的加载元数据：`bundleIdentifier`、`principalClass`、`bundlePath`，对应 Swift 侧的 `MacNativePluginLoader.swift`、`MacPluginHostContext.swift` 等文件。
- **仓库已经有独立进程模式**：`lithe-git-host`、`lithe-db-sidecar`、`lithe-db-mcp` 都是独立二进制。进程型插件与既有模式一致。

#### 第一层：声明式贡献（零代码，立即可做）

- **内容**：语言元数据（文件后缀、项目标记文件、注释符号）、主题、片段、语法查询、语义 token 规则。
- **执行风险为零**：这一层只是数据，由宿主校验、合并与渲染，插件不执行任何代码。
- **Lithe 现状**：`plugin.json` 的 `languageSupports` 已属这一层，Core 已完成解析、兼容校验与目录合并（`rust/lithe-core/src/plugins/mod.rs`）。
- **Zed 印证**：主题、图标主题、片段、tree-sitter 查询全部是声明式资源；Zed 到今天仍不允许扩展添加界面。

#### 第二层：逻辑与适配（WASM，可选，不阻塞主路径）

- **内容**：算出该执行什么命令（命令、参数、环境变量）、解析后端输出、把后端返回的 token 映射成宿主样式。
- **为什么这一层适合 WASM**：它是纯逻辑，不直接碰进程、文件与网络，沙箱才有真实意义。
- **代价有实测证据**：Zed 官方文档明确记录，扩展编译到 WASM 后 `cfg` 指令不会生效，`std::env::var` 也拿不到预期结果，必须改用 `zed_extension_api::current_platform()` 和 `Worktree` 的方法去读环境变量、在 PATH 里找二进制。也就是说，为沙箱付出的代价是"重建一套宿主 API 并长期维护"，不是零成本。这条路线本身也存在争议，例如 [Consider to not use Extism](https://github.com/Pumpkin-MC/Pumpkin/issues/110) 记录了反对意见。
- **落地时机**：第一层跑通，并且确实出现"需要一点代码但不需要操作系统能力"的需求时再做。它不阻塞主路径。

#### 第三层：系统能力（能力白名单加宿主代执行）

- **原则**：绝不把任意系统调用交给插件。插件只能声明"需要什么能力"，由宿主按白名单代为执行；没有对应授权时返回错误，不允许静默降级。
- **能力形态**（参考 Zed 的三种能力）：`process:exec` 用命令与参数白名单约束，`download_file` 用 host 与 path 白名单约束，`npm:install` 用包名白名单约束。Zed 允许用户通过 `granted_extension_capabilities` 收窄甚至清空这些授权。
- **Lithe 的现状与缺口**：`plugin.json` 已有 `providedCapabilities`（模块对外提供什么）与 `signatureRequirement`（谁签的），但**缺少"用户可裁剪、按调用点校验的授权"这一层**。今天一个签名可信的插件在能力上仍然没有约束，这是应该补的。

#### 独立进程的定位（比上一版收窄）

独立进程只用于"宿主无法代执行，而且插件必须自己持有长生命周期状态"的场景，例如插件自己的语言服务器会话或数据库连接。`GoSupport` 正属此类：它需要长期持有 `gopls` 进程与执行模块的生命周期。对于只是算一条命令或读一段配置的插件，起一个进程粒度太粗。

实现上仍然在 manifest 里新增第三种 `entrypoint.kind`（例如 `sidecar`），沿用仓库已有的独立进程模式（`lithe-git-host`、`lithe-db-sidecar`、`lithe-db-mcp` 都是独立二进制）。

代价与例外：

- 进程间通信有开销，纯计算型、高频调用的插件不适合这一层。
- 需要 manifest schema 版本号加一，并在过渡期内保留 `nativeBundle` kind，避免已装插件的 0.5.2 用户升级后插件失效。
- 进程协议直接复用现有的 JSON 命令信封与错误码约定，不新造一套。

**过渡策略**：新宿主先支持 `builtIn` 与 `sidecar`；`nativeBundle` 在 GPUI 宿主里显式拒绝并给出可操作提示，而不是静默失效。

#### 参考 DBX 的插件模型（借三件，不借一件）

DBX（`t8y2/dbx`，Apache-2.0）的插件模型是"每个插件自带 Web 前端，加一个可选的 Rust 或 Go 后端进程"，运行时通过 `request("backend.invoke", { method, params, streamId, timeoutMs })` 通信，支持 JSONL 与 framed 两种传输，并提供 `dbx-plugin create`、`package`、`dev` 三段工具链与 Rust、Go 两套 SDK。

值得照搬的三条：

1. **进程型后端加显式请求协议**。DBX 的 `method`、`params`、`streamId`、`timeoutMs` 与 Lithe 的 `command`、`payload`、`operationId`、`timeoutMilliseconds` 几乎同构。这说明"进程加命令信封"是同类产品的通行做法，Lithe 直接复用现有信封即可，不需要为插件另造一套协议。
2. **多语言 SDK 加 `create`、`package`、`dev` 三段工具链**。Lithe 今天的插件作者必须安装完整宿主、构建签名 Bundle 才能调试；`dev` 子命令让插件脱离宿主独立运行，这是真实痛点，值得在阶段 4 一并交付。
3. **把流式与超时当成一等公民**。语言服务器日志、构建输出、调试事件都是长连接场景，Lithe 契约里的 `operationId`、取消与陈旧结果语义正好对应。

不能照搬的一条：

4. **插件自带 Web 前端界面**。这与本方向的根本目的冲突：选择 GPUI Kit 的理由就是摆脱 WebView。如果插件界面走 Web，宿主必须长期保留 WebView 运行时，等于把刚移除的东西装回去。而且 macOS 侧**已经有一个走这条路的插件面板**（`LinuxDoSupport` 的 `toolWindow` + `rendererID: community.linux-do.browser`，见上文更正），所以这不是"解决不存在的问题"，而是**要主动放弃一项现有能力**：必须明确 LinuxDo 面板在新宿主里由谁渲染（宿主原生面板、还是继续保留一个受控 WebView）。同时，一旦允许插件注入 HTML，`gpui-kit-design-guides` 对插件面板就不再有效，界面一致性会被第三方内容打穿。结论是**借协议与工具链，不借界面形态**，并为现有那个面板单独给出迁移决定。

如果将来确实出现"插件需要自定义界面"的需求，应当由宿主定义声明式界面贡献点，用 gpui-kit 组件渲染，而不是让插件自带 HTML。

#### 参考 Zed 扩展模型（外部先例）

Zed 是 GPUI 的创造者，它的扩展模型是本方向最直接的先例。官方文档记录的形态是：扩展是一个含 `extension.toml` 的 Git 仓库，可提供语言、调试器、主题、图标主题、片段、MCP 服务器与 Agent 服务器；代码部分用 Rust 编译到 WASM（`wasm32-wasip2`），通过 `zed_extension_api` 的 `Extension` trait 注册。

三条对本方向有决定意义的结论：

1. **扩展加不了界面**。能力清单里没有界面这一类，主题与图标主题是声明式资源。这为"插件不贡献界面"提供了同一技术栈的权威先例。
2. **语言服务器不由扩展运行**。扩展实现的 `language_server_command` 只返回 `command`、`args`、`env`，真正的语言服务器进程由宿主拉起。这与 `GoSupport` 拉起 `gopls` 是同一件事，说明"要操作系统能力的部分不进沙箱"是通行做法。
3. **沙箱的代价是真实的**：`cfg` 失效、`std::env::var` 不可用，必须改用宿主 API 才能读环境变量与查找 PATH。这与"WASM 只承担纯逻辑层"的判断一致。

需要预期的一点：Zed 社区持续提出要更多扩展能力（例如把扩展脚本化的 Rhai 提案），说明受限扩展模型会长期承受"能不能支持某个能力"的压力。Lithe 的插件面设计应当把"新增一层能力"当作常态，而不是一次定死。

### 决策门 B：编辑器对等（阶段 4 前关闭）

已决定弃用 Monaco，改用 GPUI Kit 的 `Editor`（`input::{Editor, EditorState}`，代码编辑器，依赖 `tree-sitter` feature）。风险在于 Monaco 在本仓库已经不是"一个文本框"，它承载了一整套已经解决过的问题，替换必须逐项证明对等，而不是假设。

需要逐项实测并给出结论的能力清单（每条都要有实测证据，不能只写"支持"）：

- 大文件与虚拟化：20 万行量级的打开、滚动、编辑。
- 语义高亮与 token：两端共享的语义 token 编码是否仍成立，Java 后台上色是否还能工作。
- 编辑语义：UTF-16 范围增量与版本号、undo/redo 分支、grouped undo、外部替换。
- 保存安全：保存冲突、磁盘回读核对、只读策略拒绝写入后不落回旧路径。
- 差异视图：`MonacoDiffEditor.swift` 对应的 diff 能力。
- 多光标、折叠、括号匹配、注释与行编辑命令、智能选区。
- 图片粘贴（`MonacoImagePastePayload.swift` 对应的能力）。
- LSP 全流程：诊断、补全、悬停、跳转、重构。

门槛：以上每条都有实测结论；任一项证明不可替代时，回退到"GPUI 宿主内嵌 WebView 承载 Monaco"，并把该结论写进 Note。

**特别说明**：外部资料声称 GPUI Kit 编辑器"在 20 万行代码下保持稳定、内置 LSP 诊断支持"，本仓库内没有任何验证记录，**不能当作事实使用**。这份清单存在的意义就是把这类说法变成实测数据。

### 时间与人力假设

以上阶段相加为 34–53 周，与"约一年"吻合的**前提是阶段 5 的批次能并行 2–3 条线**。单线串行会明显超过一年。这些是估算区间，不是承诺；每个阶段的门槛才是判断依据。

### 现在就能做的下一步

1. 在当前分支上新建 `rust/gpui/` 目录与 workspace，不修改 `macos/`、`windows/`、`rust/`、`shared/` 的现有文件。
2. 做阶段 1：新 workspace + 最小宿主 + `core.ping` + `workspace.snapshot` 真实数据。
3. 阶段 1 通过后立刻做决策门 B 的实测：编辑器是最大不确定性，越早知道越好。

## 考虑过的备选方案

- **继续维护两套界面实现（维持现状）**：零迁移成本，功能可以立刻继续推进。但每个新功能都要写两遍、测两遍，长期成本随时间线性增长，而且已经出现过一次全量替换（Qt/C++ → Tauri）。之所以提出本方向，正是因为重复本身被确认为长期负担。
- **只把 Windows 换成 GPUI，macOS 保留 SwiftUI**：风险最低，能先验证技术栈，界面代码量减半。但没有达成本次目标——"macOS 一个界面、Windows 一个界面"的问题依然存在，只是把 React 换成了 GPUI，所以不采用。
- **GPUI 宿主内嵌 WebView 承载 Monaco，只换外层 UI**：保住 Monaco 与 `@lithe/editor` 的全部既有投入，编辑器风险降到接近零。代价是"纯 Rust 原生、无 WebView"的核心卖点不成立，内存收益也打折。本轮选择弃用 Monaco，因此该方案作为决策门 B 的回退项保留，而不是首选。
- **另起一个轻量新应用，只做编辑器 + 文件树 + LSP，与 Lithe 共存**：范围小、见效快，也不会碰到插件与更新等历史包袱。但它不会替换现有产品，等于增加第三个产品线，与"统一"目标相反。
- **回到 Qt/C++ 做统一界面**：仓库已经退役过 Qt 实现并明确不再构建它，重新引入等于推翻既有决策，且与 GPUI 的原生渲染目标不符，直接排除。
- **通过共享 Web 前端（让 macOS 也跑 React）统一界面**：两端都用 Web 技术可以最大化复用界面代码。但 macOS 侧会从原生 SwiftUI 变成 WebView 应用，整体体验和性能按现有方向是退步，且仍然依赖 WebView，因此不采用。
- **插件保留 macOS 原生 Bundle（在 GPUI 宿主里内嵌 Swift/ObjC 桥）**：插件 ABI 完全不变，两个官方插件零改动。代价是"一套 Rust 界面"里永久留一块 Swift，而且这块桥只为两个不贡献界面的插件存在，代价与收益不成比例，因此不作为首选，仅在阶段 4 的进程型插件被证明不可行时回退。
- **首版直接砍掉插件**：范围最小，能最快得到统一的界面。但 Go 语言支持是官方能力，砍掉等于功能倒退；而进程型插件的增量成本只是"一个枚举取值 + 进程协议"，没有必要先做减法。
- **直接采用 DBX 的插件格式与工具链（插件自带 Web 前端加 Rust/Go 后端进程）**：可以少设计一套协议，还能直接拿到 `create`、`package`、`dev` 工具链与两套现成 SDK。但它的界面部分是 Web 内容，会把 WebView 永久留在宿主里，与本方向"摆脱 WebView"的目标相反；两套插件生态并存还会让契约、审查与签名模型分裂。因此只借鉴它的协议形态与工具链思路，不引入它的格式与运行时。
- **用 WASM（Extism 或 wasmtime）承担全部插件能力**：隔离最强，能力边界可精确控制，也天然跨平台。但 Lithe 的官方插件要拉起语言服务器、执行外部程序、访问网络与凭据，这些能力必须由宿主以函数形式开放，沙箱收益会被自己拆掉，还多一层需要长期维护的 ABI（Zed 官方记录的 `cfg` 与 `std::env::var` 限制就是这条代价的实证）。因此 WASM 只承担纯逻辑的第二层，不承担需要系统能力的插件。
- **把插件全部做成独立进程（本文上一版的推荐）**：实现最直接，崩溃隔离天然成立，也复用仓库已有的 sidecar 模式。但粒度太粗——只贡献一段语言元数据的插件也要起进程；它也无法表达"用户可按调用点裁剪授权"这层语义。因此改为按能力分层，进程只服务需要长生命周期状态的插件。
- **整包照搬 Zed 的扩展模型**：形态成熟、有注册表与审查流程，而且同属 GPUI 家族。但 Zed 的扩展定位是"给编辑器补语言支持"，而 Lithe 的插件是 `dev.lithe.*` 完整功能模块（AI 助手、数据库、Docker、调试器、本地历史、搜索、终端），体量与生命周期都不同，整包照搬会低估模块侧的需求。因此只借它的分层思路、能力白名单与"扩展不加界面"这条边界。

## 验收标准

- 本 Note 通过 `node scripts/verify-agent-notes.mjs`。
- 阶段 1：宿主 crate 能构建，窗口能打开，`core.ping` 与 `workspace.snapshot` 返回真实数据且与既有 fixture 一致。
- 阶段 2：模块生命周期 9 个状态有测试覆盖；`scripts/verify-rust-core.sh`、`scripts/verify-shared-contracts.sh`、`scripts/verify-service-boundaries.sh`、`scripts/verify-module-boundaries.sh` 仍然通过。
- 阶段 3：工作台外壳可交互；高频拖动符合既有性能边界 Note 的要求；无"界面在但后端连不上"的入口。
- 阶段 4：决策门 B 的对等清单逐项附实测证据；插件的三层各有交付与验收——第一层声明式贡献能被 Core 校验并合并，第三层新增 entrypoint kind 后 `builtIn` 行为不变、`nativeBundle` 在 GPUI 宿主里显式拒绝而非静默失效、两个官方插件有进程型替代实现、能力授权可按调用点校验且用户能收窄，第二层若启动则需给出 WASM 宿主 API 的维护成本评估；插件 SDK 与 `create`、`package`、`dev` 工具链可用，且 `dev` 能脱离宿主启动一个示例插件。
- 阶段 5：128 个命令都有"已对等"或"显式不支持"的明确结论；Java/Maven 语义仍来自所选后端。
- 阶段 6：三端都能构建出可运行产物。
- 全程约束：本方向的改动只新增在 `rust/gpui/` 内，不修改 `macos/`、`windows/`、`rust/`、`shared/` 的现有文件，直到明确决定替换现有产品。

## 风险

- **编辑器是最贵也最不确定的一块**：Monaco 在本仓库承载了增量协议、版本号、undo/redo 分支、保存冲突、差异视图、图片粘贴与共享语义 token 编码，这些都在
  已归档的「macOS Monaco 独立可行性实验」里被逐条解决过（随旧前端归档，可在 git tag `legacy-frontends-final` 追溯）。换成 GPUI Kit 编辑器意味着重新赚回这些保证，任何一项不达标都会让编辑器成为阻塞点。
- **"一套 Rust 界面"不等于"零平台代码"**：平台专属的进程、PTY/ConPTY、凭据存储、更新、对话框、文件监听仍然要各写一份。`windows/tauri/src-tauri` 的 12,706 行里有多少属于这一类，需要在对齐阶段逐项确认，不能假设都白拿。
- **GPUI / GPUI Kit 的成熟度边界**：技能文档记录的平台差异（macOS/Windows/Linux/wasm 的 feature gates）说明跨平台一致性需要实测。Windows 上的输入法、DPI 缩放、无障碍、窗口装饰、拖拽这类细节，历史上是两套实现产生体验差异的主要来源，换框架不会自动解决。
- **技能文档与已发布版本错位**：`gpui-kit` 的技能文档已经描述尚未发布的 0.7.0，而可用版本是 0.6.6；P1 为此付出了一次编译失败（`open_window`、`with_assets`、`overflow_y_scrollbar`、`.then(..)` 四处）。一年周期里 GPUI、GPUI Kit 与 `lithe-core` 契约都会变，因此判断依据必须固定在"已发布版本的源码 + 提交的 `Cargo.lock`"上，升级时逐处复核。
- **42.6 万行界面代码的体量被低估**：这个数字只统计了 Swift 与 TS 源码，不含资源、本地化、测试、构建脚本与平台配置。真实迁移量大于该数字。
- **三端里的第三端是从零开始**：仓库没有 Linux 目录、没有 Linux CI、没有 Linux 打包与签名链路。把 Linux 放进一年范围，等于同时新增一个平台。
- **并行开发期**：`rust/gpui/` 与现有产品在同一分支上共存，需要持续跟进 `lithe-core` 契约的变化，否则新宿主会快速过期。
- **"看起来完成了"的假象**：对齐阶段最容易出现界面入口存在但后端没接的情况。这条已经在
  已归档的 Windows 产品功能对齐提案里被点名为真实风险（随 `windows/` 删除，可在 git tag `legacy-frontends-final` 追溯），本方向必须沿用"没有共享实现就显式失败"的规则。
- **插件 ABI 改动的兼容风险**：`nativeBundle` 是已发布的插件加载方式，改动 manifest schema 会影响已装插件的用户。过渡期必须保留旧 kind 的识别与明确提示，否则用户升级后会看到插件神秘消失。
- **能力授权缺失导致插件权限过大**：今天只要签名可信，插件在能力上就没有进一步约束，用户也无法收窄授权。补上"用户可裁剪、按调用点校验"的授权层之前，任何插件都可以按宿主权限行事。这是 Zed 的 `granted_extension_capabilities` 已经解决、而 Lithe 尚未解决的问题。
- **第二层的宿主 API 会成为长期维护面**：WASM 里的插件读不到环境变量、找不到 PATH，必须由宿主逐项开放。开出去多少 API，就要维护多少兼容性；Zed 的记录说明这不是一次性成本。
- **插件开发体验是生态成败的实际瓶颈**：今天写一个 Lithe 插件需要完整宿主、签名 Bundle 与 macOS 环境，既没有独立的调试链路也没有打包链路。如果阶段 4 只交付加载机制而不交付工具链，插件的数量不会增长，跨平台 ABI 的价值也就无法被验证。
- **排期风险**：阶段 5 的 16–24 周假设 2–3 条并行线；如果实际只有一条线，总周期会超过一年，届时需要重新划定首版范围，而不是压缩验收标准。

## 适用范围

- `rust/lithe-core/`
- `shared/contracts/`
- `shared/fixtures/modules/`
- `.agents/notes/proposed/architecture/`
- `scripts/verify-agent-notes.sh`
- `scripts/verify-rust-core.sh`
- `scripts/verify-shared-contracts.sh`

> 旧前端（`macos/`、`windows/`、`frontend/editor/`、`Plugins/`）已删除，原先列在这里的路径不复存在。
> 正文引用的类名与行号对应 git tag `legacy-frontends-final`（最后一份含旧前端的提交）。
