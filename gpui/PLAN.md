# GPUI 重写：执行计划与界面规格

> ## 🔁 界面规格来源变更（2026-09-25，维护者决定）
>
> - **实现框架不变：继续用 GPUI Kit**（本目录 `gpui/`，`gpui-kit = "=0.6.6"`）。
> - **界面规格来源改为 Windows 前端**：`windows/tauri/src/features/*`（Tauri v2 + React + Tailwind + shadcn）。
>   **不再以 macOS SwiftUI 作为视觉/布局规格**；macOS 那份调研已拆到 **`gpui/docs/archive/ui-map-macos.md`**
>   （原始调研材料已在 2026-09-25 清理），降级为**行为/功能对照**（它功能最全，用来查"这个交互原本怎么工作"），
>   **尺寸、结构、观感一律看 Windows**。
> - **规格文档布局**：`UI-MAP.md` = §1 硬规则 + gpui-kit 实现规则（仍然有效）；`UI-MAP-WINDOWS.md` = 逐区域对应表（待产出，
>   汇总自 `gpui/research/windows/01..06-*.md`）；`docs/archive/ui-map-macos.md` = macOS 对照。
> - 执行提示词：`docs/development/gpui-ui-windows-rewrite-prompt.md`；决策记录见
>   `.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md` 顶部的状态变更。
> - 因此 `UI-MAP.md` 需要**重新以 Windows 为源做一份对应表**（提示词里列为第 1 步交付物）。

本文件是这条线的**唯一工作计划**，跨会话记忆就靠它。

- `README.md` 讲**怎么跑**；
- `.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md` 讲**为什么这样做**；
- **本文件讲"下一步做什么、做成什么样、怎么算做完"**。

---

## 0. 最初的目标（防止跑偏，每次开工先读这一段）

1. Lithe 现在是 **macOS 一套界面（SwiftUI/AppKit）+ Windows 一套界面（React/Tauri）**。目标是用 **GPUI Kit 写成一套 Rust 界面**，覆盖 macOS / Windows / Linux。
2. 技术栈锁定 **gpui-kit**（`longbridge/gpui-kit`，当前 `0.6.6`）。**只以它为准**：不使用其他 GPUI 技能、示例或类比写法。
3. **界面整体参考 macOS 端**（2026-09-24 维护者改定）：权威界面规格是 `macos/Sources/Lithe`（SwiftUI/AppKit）的**源码**，不是截图、也不是 Windows 端。做法分两步：
   - 先把 macOS 界面逐区域逆向成 **`gpui/UI-MAP.md`**（每个区域的组成 + 每个元素对应 gpui-kit 的哪个组件 + 哪些地方用 gpui-kit 的能力优化）；区域调研的原始材料在 `.artifacts/ui-map/`；
   - 再**照这份文档一比一复刻**，实现只允许用 gpui-kit 0.6.6 源码里**真实存在**的组件与基元。
   Windows 端（`windows/tauri/src/features/*`）与产品截图降级为**旁证**：用来交叉验证同一功能在另一端的取舍，不再是规格来源。
   注意分寸：复刻的是**区域组成、层级、尺寸、状态、交互**，不是逐行翻译 SwiftUI；macOS 里做得不好的地方（信息密度、缺状态、交互绕）允许用 gpui-kit 做得更好，但要在文档里写明理由。
4. 插件方向：**走 Zed 那种 WASM 扩展模型**（声明式贡献 + WASM 逻辑 + 宿主代执行系统能力），后面阶段再做。
5. 工程纪律（每一步都适用）：
   - **缺 API 就先去读源码**，不要凭记忆或类比写。源码在
     `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-*\gpui-{kit,component,base}-0.6.6\src\`。
   - **同一个问题最多试 5 次**（编译/实验各算一次）。超了就写进"已知问题"并跳过，不许卡住。
   - 每步留**可复验证据**（截图、日志、原始 JSON）到 `.artifacts/p1/`。
   - **进程约定（维护者 2026-09-23 指定）**：参考真机（已安装的 Lithe）与 GPUI 开发版窗口**都保持运行、不要关**，用后台任务托管：
     - 参考真机：`Start-Process "D:\Programs\Lithe\lithe-windows.exe"`（后台任务常驻）
     - 开发版：`cargo run --bin shell-probe -- <workspace-root>`（后台任务常驻，改完重新构建后再重启）
     截图直接抓这两个窗口即可，不必每次起停。仓库 AGENTS.md 的"测完关进程"规则在这里按维护者要求让位。
   - **改动范围（2026-09-23 澄清）**：
     - **`gpui/` 下的所有文件都可以自由改动、新增、拆分模块** —— 这是本实验工程自己的目录，不存在"只能改某两个文件"的限制。
     - **不要动** `macos/`、`windows/`、`rust/`、`shared/`、`Plugins/`：那是现有产品与跨平台契约，实验必须是非侵入的。
     - `docs/`、`.agents/notes/` 只在任务明确要求写文档或记录决策时改动。
   - **并行子代理的文件所有权**：同一时间**一个文件只归一个子代理**。为了避免大家挤在 `shell-probe.rs` 上，下一步要把它拆成模块（`shell/` 下每个面板/区域一个文件），之后就能并行推进。

---

## 1. 界面长什么样（目标，逐区域对照）

**权威界面规格 = `macos/Sources/Lithe`（SwiftUI/AppKit）的源码**，逐区域整理在 **`gpui/UI-MAP.md`** 里；本节只记录"怎么读 macOS 源码、怎么换算、怎么落到 gpui-kit"。

### 1.1 读 macOS 界面源码的规矩

- 目录分工（`macos/Sources/Lithe`，446 个文件 / 约 7.4 万行，另有各 `Lithe*Module`）：
  | 目录 | 行数 | 内容 |
  | --- | --- | --- |
  | `Views/App` | 4.9k | 应用外壳：窗口、标题栏、菜单、设置窗口、浮层 |
  | `Views/Workbench` | 4.0k | 工作台骨架：活动栏 / 侧栏 / 编辑区 / 工具窗 / 状态栏 |
  | `Views/Editor` | 11.8k | 编辑区：标签栏、面包屑、编辑内容宿主 |
  | `Views/Git` | 11.2k | Git：提交日志工具窗、引用树、提交表、diff |
  | `Views/Database` / `Views/Run` / `Views/Debug` / `Views/Diff` | 5.3k / 2.8k / 2.6k / 2.3k | 数据库、运行、调试、diff |
  | `Views/Components` | 1.7k | 复用组件（按钮、行、表头…） |
  | `Views/Search` / `Views/Language` / `Views/Workspace` / `Views/History` / `Views/GitHub` | 1.5k–2.0k | 搜索、语言智能、项目树、历史、GitHub |
  | `Theme` | 1.6k | 设计 token（颜色、字号、度量） |
- **单位直接换算**：macOS 的 1pt = 1 逻辑像素 = gpui 的 `px(1.)`，所以 macOS 里的 `40`、`12`、`size-3` 可以原样搬过来，不需要乘缩放比（`window.scale_factor()` 只和物理像素有关）。
- **颜色/字号一律走 token**：先去 `macos/Sources/Lithe/Theme/` 找到 token 名，再换算到 `cx.theme()` 里**语义相同**的 token（映射表见 `UI-MAP.md` 的附录）；只有 gpui-kit 没有对应 token 时才自建，并在文档里写明。
- **文字取 macOS 的本地化原文**（`macos/Resources/zh-Hans.lproj`），不要自己编中文。
- 复刻的最小单位是"元素"，每个元素都要在 `UI-MAP.md` 里写清：macOS 出处（`文件:行号`）→ 尺寸/状态 → gpui-kit 用什么实现 → 是否一比一。

### 1.2 Windows 截图（旁证，不再当规格）

`docs/assets/screenshots/windows-*.png` 与已安装的 `D:\Programs\Lithe\lithe-windows.exe`（0.4.10）只用来交叉验证同一功能在另一端的取舍：

```powershell
Start-Process "D:\Programs\Lithe\lithe-windows.exe"
pwsh -File gpui\capture-screenshot.ps1 -OutputPath .artifacts\p1\real-lithe.png -ProcessName lithe-windows
```

### 1.3 区域清单与当前差距（macOS 口径，逐轮更新）

权威版本在 `gpui/UI-MAP.md`；这里只留一张"现在做到哪"的速查表（证据 `.artifacts/p1/rows-fixed.png`）：

| # | 区域（macOS 口径） | 现状 |
| --- | --- | --- |
| 1 | 标题栏（40pt、项目名 + 分支） | ⚠️ 结构与文案有；高度仍是 gpui-kit 默认 34，缺项目图标与下拉菜单 |
| 2 | 项目标签条（项目芯片） | ✅ 观感对齐（真机同款圆角选中底 + `×`） |
| 3 | 活动栏（约 34px、图标顶部对齐） | ⚠️ 结构对齐；图标是默认图标集里的近似字形（无 Git/数据库字形） |
| 4 | 侧栏 / 项目树 | ⚠️ 行结构（折叠箭头 + 类型图标 + 缩进 + 截断）已对齐；缺 git 状态字母、右键菜单、多选、真实递归 |
| 5 | 编辑区（标签栏 + 面包屑 + 正文） | ⚠️ 内部标签栏（`← →` + 类型图标 + 文件名 + `✕`）与真实文件内容/行号已通；缺面包屑、脏标记、多标签、分屏 |
| 6 | 底部工具窗（提交记录） | ⚠️ 已横跨整个工作区、头部/标签/引用树/提交表都在；缺 `提交文件` 第三栏、提交图、真实 `git.*` 数据 |
| 7 | 状态栏 | ❌ 只有左右两段文字，未按 macOS 的 `Footer` 复刻 |
| 8 | 浮层（搜索一切 / 命令面板） | ❌ macOS 只有 `SearchEverywhereView`；gpui-kit 有现成 `Command` 可补 |

配色全部走 gpui-kit 主题 token（`cx.theme()`），不写裸色值；真机默认**深色**，`gpui_kit::init` 之后要显式 `Theme::change(ThemeMode::Dark, None, cx)`。

> ⚠️ 已知坑：gpui-kit 的**在线文档与技能文档描述的是未发布的 0.7.0**，而可用版本是 0.6.6。**判断依据一律是已发布版本的源码。**
> - 0.6.6 里**确实没有**：`gpui_kit::open_window`、`.overflow_y_scrollbar()`、`Task::then`、`Theme::tab_active_bg`。
> - **存在，但定义在 trait 上，必须 `use ... as _` 才会解析**：`Button::{ghost, selected, disabled, xsmall}`（分别在 `ButtonVariants` / `Selectable` / `Disableable` / `Sizable`；`button/button.rs:44-96,539`、`sizing.rs:178-202`）、`Icon::small()`（`Sizable`，组件自己就用：`list/list_item.rs:227`）。
> - 自查方法：`rg 'fn <名字>' <crate>/src` 比翻文档可靠；`cargo check` 的报错会直接点出"trait 未在作用域内"。

---

## 2. 分步计划（每步 = 一个可验收的交付）

| 步 | 做什么 | 验收标准 | 状态 |
| --- | --- | --- | --- |
| **0** | 工具链 + 启动基线 | 窗口能开；`core.ping` 与 `workspace.snapshot` 返回真实数据 | ✅ |
| **1** | 外壳骨架（`DockArea` 作布局基础） | 六个区域同时在窗口里，无溢出 | ✅ |
| **2** | **界面规格文档**：按 macOS 源码逐区域逆向 | 8 个区域调研 + 合并成 `gpui/UI-MAP.md`（区域组成 / 度量 / 元素 → gpui-kit 对应 / 缺口 / 优化） | ⏳ 调研已完成（`.artifacts/ui-map/01..08`），合并中 |
| **3** | 工作台骨架对齐 macOS | 标题栏 40 / 项目标签条 38 / 活动栏 38 / 状态栏 24 / 左栏 320 / 右栏 360 / 底部工具窗 260 且**横跨左栏 + 编辑区**；深色主题 | ✅（`rows-fixed.png`、`metrics-aligned.png`） |
| **4** | 资源管理器（`项目`） | 行结构（折叠箭头 + 类型图标 + 缩进 14 + 行高 24 + 截断）、**点击文件打开到编辑区**、真实 `workspace.snapshot` 数据 | ⚠️ 基本行结构与点击已通；缺 git 状态字母、右键菜单、递归目录、活动文件高亮 |
| **5** | 编辑区 | 内部标签栏（`← →` + 类型图标 + 文件名 + `✕`）+ 真实文件内容 + 行号 + 空状态 | ⚠️ 已通；缺面包屑（macOS 在状态栏里）、脏标记、多标签、分屏、预览态 |
| **6** | 底部工具窗（Git） | 照 macOS `GitLogView`：页签 + 三栏（引用 220 / 提交 弹性 / 提交文件 350）、提交行 22pt、引用树行结构 | ⏳ 横跨全宽已完成；三栏与 22pt 行改造中 |
| **7** | 状态栏 | 照 macOS `statusBar`（24pt）的真实组成与文案，数据来自 Core | ⏳ 进行中 |
| **8** | 浮层（搜索一切；命令面板是**用 gpui-kit 补的优化项**） | 快捷键唤起、Esc 关闭、焦点归还触发者；实现路径见 `gpui/research/gpui-kit-overlay-howto.md` | ⏳ |
| **9** | 宿主骨架（路线图 P2） | 命令统一入口（`operationId`/取消/超时/陈旧结果）+ 事件路由 + 模块生命周期 9 状态；校验脚本仍通过 | ⏳ |
| **10** | 功能面移植（路线图 P5，按批次） | git → 运行/调试/终端 → AI/数据 → Java 语义 → 设置写路径，每批次功能对等清单 | ⏳ |
| **11** | 编辑器对等（决策门 B） | 对等清单逐项实测（大文件、语义 token、undo 分支、保存冲突、diff、LSP 全流程） | ⏳ |
| **12** | 插件（Zed 式 WASM） | 三层插件面 + 能力白名单 + `create`/`package`/`dev` 工具链 | ⏳ |

**顺序**：2 是这一步的规格来源，3→4→5→6→7 是"照规格把界面搭出来"，8→9 是浮层与地基，10 之后是功能量。写任何界面代码之前先读 `UI-MAP.md` 的对应区域一节。

---

## 3. 已确认的组件用法（从 0.6.6 源码取，可复用）

```rust
// 启动（0.6.6 唯一正确顺序）
gpui_kit::application().run(move |cx| {
    gpui_kit::init(cx);
    cx.spawn(async move |cx| {
        cx.open_window(options, move |window, cx| {
            let view = cx.new(|cx| AppView::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        }).expect("failed to open window");
    }).detach();
});

// 窗口尺寸：从显示器可见区域算（可见区 94%、居中、普通窗口 —— 与参考应用 Dodona 同口径）
// 根视图：直接用 `.size_full()`。
// ⚠️ 下面这两行是**错误示范**，保留在此仅作警示（见本轮结论第 8 轮）：
// let v = window.viewport_size();                 // 已经是 DPI 无关单位，不是物理像素
// let logical = v.height / window.scale_factor(); // ❌ 再除一次 → 内容只占窗口 78%

// Dock
let (area, skin) = DockSkin::dock_area("lithe-workbench", Some(1), window, cx);
let layout = DockLayout::h_split()
    .child(DockLayout::tabs().panel_view(panel_handle(panel), cx), Some(px(280.)))
    .child(DockLayout::v_split().child(.., None).child(.., Some(px(180.))), None);
area.update(cx, |area, cx| area.set_center(layout, window, cx));
// 面板：impl BasePanel { fn panel_name } + impl Panel { fn title } + Focusable + EventEmitter<PanelEvent> + Render

// Tree
TreeState::new(cx); state.set_items(Vec<TreeItem>, cx);   // TreeItem { pub id, pub label, pub children }
Tree::new(&state, |ix, entry, selected, _, _| ListItem::new(..).selected(selected))

// DataTable
TableDelegate 必需：columns_count / rows_count / column(-> Column) / render_td
DataTable::new(&state) 实现 Sizable 而非 Styled → 尺寸靠外层 div().size_full()

// Command
CommandState::new(window, cx)
Command::new(&state).searchable(true).group(CommandGroup::new().label(..).items([CommandItem::new().label(..)]))
```

---

## 4. 当前状态与已知问题（每轮更新）

**已完成**：步 0、步 1、步 2（标题栏 + 项目标签条 + 活动栏，Dock 区域正常）。

**已知问题（按优先级）**：

| # | 问题 | 现象 | 下一步 |
| --- | --- | --- | --- |
| 1 | ~~Dock 区域消失~~ | **已修**：包裹层缺 `h_full()` | — |
| 2 | ~~Tree 行重叠~~ | **已修**：`gpui-base/src/tree.rs:423` 用 `uniform_list`（均匀行高），而我的树行没禁用换行 → 长子路径折行导致行高不一致。加 `.whitespace_nowrap().overflow_hidden()` 后干净（`tree-fixed.png`） | — |
| 3 | ~~DataTable 表体为空~~ | **已修**：异步加载把行写进了 `files_panel.stats`，而右栏渲染的是 `stats_panel.stats` —— 两个不同的 `TableState`；被渲染的那个 `rows_count()` 恒为 0，于是走 `TableState::render` 的 `empty_view` 分支（`gpui-component-0.6.6/src/table/state.rs:2406`、`:2435`）。表头一直正常，是因为 `columns_count()` 恒为 3，与行无关。修法：构造后先存 `stats_panel.read(cx).stats.downgrade()`，加载完写回该实体并 `refresh(cx)`。证据：`table-fixed.png` | — |
| 3 | 活动栏图标为占位、垂直居中 | `PanelLeft/Inbox/Search/LayoutDashboard` | 枚举完整图标集后换成 文件/Git/搜索/数据库，并改顶部对齐 |
| 4 | 侧栏顶部工具条未做 | 缺搜索 + 视图设置两个图标 | 步 4 |
| 5 | 编辑器空状态与真实文件内容未做 | 现在显示的是硬编码示例 | 步 5 |
| 6 | 命令面板是标签页而非浮层 | — | 步 8 |

**已澄清的非问题**：树"行重叠"在 Dock 高度修好后不再出现（`step2-dock-back.png` 中树是干净的），此前是布局高度不可靠导致的连带现象。

**本轮结论（DataTable，核对过 gpui-component-0.6.6 源码）**：

- 每个面板各自持有独立的 `TableState`；要改哪个面板的表，就必须更新**该面板实体里**的 `TableState`，更新别的面板不会影响它 —— 这正是表体为空的根因。
- 表体行数在 `TableState::render` 里每次现取 `delegate.rows_count(cx)`（`table/state.rs:2373`），所以不必重建 `TableState`；改完 delegate 调 `TableState::refresh(cx)`（只重算列与表头、**不** notify，`table/state.rs:388`）再 `cx.notify()`。
- `render_td` 的返回值由组件自带的 `render_cell` 包裹（`table/state.rs:1323-1329`：固定列宽 + `overflow_hidden` + `whitespace_nowrap`），因此单元格文本不需要自己加 `whitespace_nowrap`。
- 三列总宽必须 ≤ 面板宽度，否则最右侧的 `占比` 列会被面板右边缘裁掉（已由 120/70/60 改为 96/58/54）。
- 0.6.6 里确实**没有** `TableState::set_rows` / `update_delegate`，更新入口就是 `delegate_mut()` + `refresh()`。

**证据目录**：`.artifacts/p1/`（截图、日志、Core 原始响应）。

**本轮结论（模块拆分，2026-09-24）**：

- 目录结构（`gpui/shell/src/bin/`，Rust 2018+ 规则：`shell-probe.rs` 的子模块放 `shell_probe/`）：

  | 文件 | 行数 | 职责 |
  | --- | --- | --- |
  | `shell-probe.rs` | 12 | crate 根：`mod shell_probe;` + `pub use shell_probe::main;` |
  | `shell_probe/mod.rs` | 90 | `main`、`workspace_root`、`startup_window_bounds`、启动顺序 |
  | `shell_probe/workspace.rs` | 270 | `ShellWorkspace`：标题栏 + 项目标签条 + 活动栏 + Dock 组装 + 状态栏 + `Render` |
  | `shell_probe/panels.rs` | 229 | `ShellPanel`（`BasePanel`/`Panel`/`Focusable`/`EventEmitter<PanelEvent>`/`Render`）与 `PanelKind` |
  | `shell_probe/files.rs` | 120 | `FILE_ROWS`、`build_tree`、`load_files`（Core `workspace.snapshot`） |
  | `shell_probe/stats.rs` | 65 | `StatRow` 与 `StatsDelegate`（`TableDelegate`） |

- **bin 目标的 `main` 必须能从 crate 根解析**。`fn main` 放在 `shell_probe/mod.rs` 时，根文件要写
  `mod shell_probe; pub use shell_probe::main;`；此时 `mod.rs` 里的 `main` **必须是 `pub fn`**，
  否则报 `error[E0603]: function main is private` + `error[E0601]: main function not found in crate`。
- 跨区域访问用 `pub(super)`：`panels` 里 `ShellPanel::{title, tree, stats}` 与 `ShellPanel::{files, editor, placeholder}`、
  `stats` 里 `StatRow` 字段与 `StatsDelegate::rows`、`files` 的 `build_tree`/`load_files`/`FILE_ROWS`、
  `workspace::ShellWorkspace`。`pub(super)` 在 `shell_probe` 内对所有兄弟模块可见，正好够用，不需要 `pub(crate)`。
- 每个模块**自带 import**（原来的单文件共享 import 不再存在）；`AppContext` 这类 trait 要按模块补，
  漏了会报 `method not found ... trait AppContext which provides new is implemented but not in scope`。
- **行为与界面不变**：代码是逐字搬迁，只加可见性与 import。对照 `table-fixed.png`（拆分前）与
  `modular.png`（拆分后）：布局、面板、表格列、状态栏完全一致；只有侧栏计数从 4876 → 4886、`.artifacts` 41 → 46、
  `gpui` 10 → 15，这些差异来自本轮新增的文件本身。
- 编译：失败 2 次（首次 14 个错，第二次 2 个错），第 3 次成功；未超"同一问题 5 次"上限。
  残留 warning 与拆分前相同（`files_panel` 字段未读、`lithe-core` 链接提示）。
- 本机 `rustfmt` 组件未安装（`rustup component add rustfmt`），所以没跑 `cargo fmt`，格式按原文件手工对齐。
- `shell-probe` 开发版已重新构建并常驻运行（PID 17092），日志 `.artifacts/p1/shell-probe-run.log`。

**本轮结论（步 5：编辑区对齐真机，2026-09-24）**：

- 改动只落在 `gpui/shell/src/bin/shell_probe/panels.rs`（未新建文件、未动 `workspace.rs` / `mod.rs` / 其他模块）。
- 编辑区现在的结构：面板内部 `TabBar`（左侧 `← →` + 当前文件标签：类型图标 + 文件名 + `✕`）→ 正文 `Editor`（真实文件内容 + 行号，行号是 `gpui-base` 的 `LayoutMode::CodeEditor { line_number: true }` 默认行为，不需要额外接线）；`document` 为 `None`（`source` 为空）时正文位置换成 `Empty` 空状态。
- **0.6.6 API/图标核对结论（都用已发布源码，不是文档）**：
  - `TabBar`（`gpui-component-0.6.6/src/tab/tab_bar.rs:40-166,359-560`）：`new(id)` / `prefix(impl IntoElement)` / `child(impl Into<Tab>)` / `selected_index` / `max_width(Pixels)` / `with_variant`。默认变体是 `TabVariant::Tab`（`tab/tab.rs:14-21`），bar 底色 = `tokens.tab_bar`，选中标签 = `tokens.tab_active` + `tab_active_foreground`（`tab/tab.rs:223-233`），自带底部 1px 分隔线（`tab_bar.rs:501-515`）。
  - `Tab` 的 `icon` 分支只画图标、**丢掉 label**（`tab/tab.rs:719-744`），所以"图标 + 文件名"要走 `Tab::child(...)`；`max_width` 让标签内容 `flex_auto()`，此时标签内文字才可 `truncate()` 成 `…`。
  - `Empty` 家族（`empty.rs`）：`Empty/EmptyHeader/EmptyMedia(+EmptyMediaVariant::Icon)/EmptyTitle/EmptyDescription`；`Empty` 默认带虚线边框，真机没有，用 `.border_color(cx.theme().transparent)` 关掉。
  - `Button::ghost()` / `Button::disabled()` / `Button::xsmall()` **存在**，但分别是 `ButtonVariants` / `Disableable` / `Sizable` 三个 trait 的方法，**必须 `use ... as _` 才会解析**（`button/button.rs:44-96,503-537`，`sizing.rs:178-202`）；禁用 ghost 按钮 = 透明底 + `muted_foreground.opacity(0.5)` 图标（`button/button.rs:1273-1306`），和真机那对灰箭头一致。确实**没有** `Button::selected()` 的裸方法（同样只在 `Selectable` trait 上）。
  - `IconName` 在 0.6.6 是 `Clone` 但**不是 `Copy`**（`component/icon.rs:17-20`），从 `&self` 里取要 `.clone()`。
  - `truncate()` / `text_ellipsis()` / `whitespace_nowrap()` 都在 GPUI 的 `Styled` 上（`gpui-pre-0.3.6/src/styled.rs:82-141`）。
- **图标集硬约束（踩坑，已同步给并行子代理）**：`mod.rs` 注册的是 `gpui_kit::assets::Assets`，它**只嵌入 `gpui-kit-assets-0.6.6/default-icons.txt` 列出的 ~95 个字形**；完整 Lucide 目录要注册 `AllAssets`。因此 `Icon::new(IconName::X)` 里的 `X` 必须来自
  `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-kit-assets-0.6.6\default-icons.txt`，否则要么编译不过（兼容枚举里没有），要么运行时画空白（用了全集的 `assets::IconName` 但没注册 `AllAssets`）。本步用的是默认集内的 `FileText` / `Close` / `ArrowLeft` / `ArrowRight`。
- **改为以 Windows 源码为规格（维护者 2026-09-24 要求）**：截图只核对观感，结构与文案以 `windows/tauri/src/features/{tabs,panes,editor}` 为准。核对结果：
  - `panes/components/pane-container.tsx:1094-1100`：pane = `<TabBar/>` 在上 + `relative min-h-0 flex-1 overflow-hidden` 正文；没有活动 buffer 时**正文位置**换成 `EmptyEditorState`（标签栏仍在）→ 本步结构一致。
  - `tabs/components/tab-bar.tsx:617-816`：条内三段 = 左侧 `← →`（ghost/icon-xs，`disabled={!canGoBack}`）+ 中间可横向滚动的标签条 + 右侧动作组（Markdown 模式选择器、`+` 新建标签页、拆分关闭、编辑器全屏，后两个/`+` 悬停才显示）。
  - `tabs/components/tab-bar-item.tsx:132-292`：标签 = 类型图标（`text-subtle-foreground`，`size-3` 盒子）+ 文件名（`min-w-0 flex-1 text-ellipsis whitespace-nowrap`，活动 `text-foreground` / 非活动 `text-subtle-foreground`，预览态斜体）+ 未保存圆点（`size-2 bg-primary`）+ 关闭/Pin 按钮（ghost，右上绝对定位，默认悬停才显示）；`title={buffer.path}`。
  - `ui/tab-bar.tsx:244-255` + `styles/theme.css:120-133`：条底 `bg-tab-bar` + `border-b`，条高 `2.25rem = 36px`，标签宽上限 `12.5rem = 200px`、下限 `min-w-20`，条内边距 8px、图标↔文字 6px；活动标签（`connected` 变体，`ui/tab-bar.tsx:204-210`）是**透明底 + 底边 3px 主色条**（IntelliJ 风格）→ 对应 gpui-kit `TabVariant::Underline`（`tab/tab.rs:253-262`：透明底 + 底边 2px `primary`，条高也正好 36px），再用 `.bg(cx.theme().tab_bar)` 补回条底色。
  - `tabs/utils/path-shortener.ts:31-113`：同名文件才追加 `文件名 · 区分目录`，否则就是文件名。
  - `panes/components/empty-editor-state.tsx` + `i18n/locale.ts:6150-6151`：居中一列（48px 图标块 = 40px `FileText` + 右下角 20px 放大镜）+ 标题「选择文件以查看」+ 说明「外部工具产生的更改会自动显示。」→ 文案已逐字采用（原来我自己编的说明句已删）。
- **已做到 / 未做到**：`← →` 只有外观（禁用态 ghost，`aria-label` = `tabs.goBack/goForward` = "后退到上一个位置"/"前进到下一个位置"，`tooltip` = `tabs.goBackShort/goForwardShort`），**没有历史栈**；标签 `✕` 现在**有行为**：等于 Windows 关掉最后一个标签 → 没有活动 buffer → 正文换成空状态（`pane-container.tsx:1100`），清空 `EditorState` 内容。未做（`tab-bar.tsx`/`tab-bar-item.tsx` 里有、本步没有）：右侧 `+`/拆分关闭/编辑器全屏动作组、未保存圆点、固定(Pin)与预览态、标签拖拽重排与拖出成新 pane、右键菜单（固定/重命名/关闭其他/关闭右侧/全部关闭/复制路径/复制相对路径/重新加载/在文件夹中显示/向右拆分/向下拆分）、滚轮横滚、`displayName` 的同名区分、`title=完整路径` 悬停提示、中键关闭。
- **需要主代理接线**（都属于 `workspace.rs` / 后续步骤，未自己改）：
  1. `ShellPanel::editor(window, cx, source)` 只收到**内容**，收不到路径；标签文件名硬编码 `lib.rs`（常量 `EDITOR_FILE_NAME`）。接上真实路径后应由路径推导文件名 + 类型图标，并处理 `path-shortener` 的同名区分规则。
  2. 标签拖拽重排 / 拖出成新 pane / Dock 分组：Windows 用 dnd-kit + pane 树；gpui-kit 里对应能力在 `Dock`（`DockArea` 的标签拖拽），面板内部自己做不了 —— 需要决定"编辑区标签用 Dock 的 tab group 承载，还是面板内自绘 + 自己实现拖拽"。
  3. 右侧动作组（`+` 新建标签页、拆分关闭、编辑器全屏）没有对应能力（没有 new-tab 视图 / pane 拆分 / 面板全屏），是"能力缺失"而不是"少画了按钮"，所以本步没有放空按钮。
  4. Dock 的标签头无法去掉：编辑区面板的 `title` 是 `Editor`，文件名交给内部标签栏。真正做到"只有一条标签栏"需要 Dock/工具栏层面的改动。
  5. 空状态现在**可见**（点标签 ✕ 即可），但重新打开文件需要文件树 → 编辑器的接线（`Tree` 项还没有 `on_click` 打开文件的路径）。
- 编译：`cargo check --bin shell-probe` 共 5 次（用满上限）。第 1 次 1 个错（`IconName` 非 `Copy`，改 `.clone()`）；第 2、3 次只剩并行子代理 `bottom_panel.rs` 的图标名错误；第 4 次 **0 error**；第 5 次（源码对齐改写后）**只有 1 个错且属于 `bottom_panel.rs`**（`Map<Iter<..>>: IntoElement`，对方正在改），`panels.rs` 依旧 0 error / 0 new warning。现存 warning 都不是本步引入：`PanelKind::Terminal` 已不被构造（底部面板换成了 `bottom_panel.rs` 的 `BottomPanel`）与 `workspace.rs` 的 `files_panel` 未读。

**本轮结论（步 6：底部工具窗照 Windows 源码移植，2026-09-24）**：

- 改动范围：新建 `gpui/shell/src/bin/shell_probe/bottom_panel.rs`（底部工具窗面板 `BottomPanel`），
  改 `workspace.rs`（把 Terminal 占位面板换成它）。**没有**动 `mod.rs` / `panels.rs` / `files.rs` / `stats.rs`。
- 新模块的声明写在 `workspace.rs` 里：`#[path = "bottom_panel.rs"] mod bottom_panel;`。之所以不去 `mod.rs` 加一行，
  是因为 `mod.rs` 归别的子代理；合并后把这一行挪进 `mod.rs` 即可（否则同一个文件会被编译成两个模块，能过但很怪）。
- `bottom_panel.rs` 的结构：`BottomTab{Log,Console}`、`CommitRow`、`CommitDelegate`（`TableDelegate`）、
  `placeholder_references`、`placeholder_commits`、`reference_row`、`author_cell` / `date_cell` / `reference_badge` /
  `current_badge`、`BottomPanel::{new, render_header, render_tabs, tab_item, render_log, render_reference_actions,
  render_filter_bar, render_console}`，外加 `BasePanel` / `Panel` / `Focusable` / `EventEmitter<PanelEvent>` / `Render` 五个 impl。
- `workspace.rs` 的 4 处改动：① 顶部加 `#[path] mod bottom_panel;` + `use self::bottom_panel::BottomPanel;`；
  ② `terminal_panel`（`ShellPanel::placeholder(.., PanelKind::Terminal, ..)`）→ `BottomPanel::new(window, cx)`；
  ③ 中列 `v_split` 底部格 `.panel_view(panel_handle(bottom_panel), cx)` 且高度 `px(180.)` → `px(220.)`
  （真机这一格约 220 逻辑像素，属这一格自己的参数，其它区域未动）；④ 一行注释（"终端" → "底部工具窗"）。
- **移植依据（只读 Windows 源码，不是照截图猜）**：

  | Windows 源码 | 对应实现 |
  | --- | --- |
  | `features/git/components/log/git-log-tool-window.tsx:563-708` | 面板三层骨架 + `panel` 状态（`日志`/`控制台`）+ 两栏结构 |
  | `.../git-log-title-bar.tsx:27-73` | `render_header`：图标 + `提交记录` + `日志：{name}` 按钮 + 刷新 / 设置 + 右对齐 `只读` + 隐藏 |
  | `.../git-reference-tree.tsx:354-407,803-899` | `render_reference_actions`（`w-9` 动作条）+ `引用` 标题行（右侧条数）+ `HEAD（当前分支）` 单行 + `本地/远程/标签` 分组 |
  | `.../git-reference-tree.tsx:560-660` | `reference_row`：`h-6` 行、分组右侧条数、分支右侧"当前"徽标、当前分支高亮色 |
  | `.../git-commit-table.tsx:198-259` | `render_filter_bar`（`Q {字段} 筛选` + 字段下拉 + 装饰开关 + `数/总数`）与列宽（`提交` flex-1 / `作者` `w-28` / `日期` `w-32` 右对齐） |
  | `.../git-commit-table.tsx:290-327` | `CommitDelegate::render_td`：装饰徽标 + 说明 / 作者弱化色 / 日期右对齐小字 |
  | `i18n/locale.ts` 的 `git.log.*`、`git.console.*`、`footer.readOnly` | 全部界面文案（含全角冒号 `日志：全部`） |
- **0.6.6 API 核对**（一律以已发布源码为准）：`TreeState::new(cx).items(..)`（`gpui-base/src/tree.rs:197-219`）、
  `Tree::new(&state, \|ix, &entry, selected, &mut Window, &mut App\| -> ListItem)`（`component/tree.rs:18-63`）、
  `TreeItem{pub id, pub label, pub children}` + `.expanded(..)`（`tree.rs:41-126`）、`TreeEntry::item()`（`tree.rs:61`）、
  `ListItem::{selected,on_click,suffix,h,gap_2,px_2}`（`list/list_item.rs:44-110`）、`TableDelegate` 四个必需方法
  （`table/delegate.rs:16-118`）、`Column::{new,width,text_right,resizable,movable}`（`table/column.rs:88-226`）、
  `TableState::{new,delegate,delegate_mut}`（`table/state.rs:267-312`）、`DataTable` 是 `Sizable` 不是 `Styled`、
  表头默认按列名画（`table/delegate.rs:77-86`）、`Input::new(&InputState)`（`input/input.rs:180`）、
  `InputState::new(window,cx).placeholder(..)`（`gpui-base/src/input/base/state.rs:766`）、
  `Button::{compact,icon,outline,label,tooltip,dropdown_caret,on_click}`（`button/button.rs:318-472`）、
  `Panel::{title_bar,inner_padding}`（`dock/panel.rs:131-148`）返回 `false` 时，Dock 在"单面板 + `PanelStyle::Auto`"
  下不画自己的标题栏（`dock/tab_panel.rs:703-715`）—— 这是面板能自绘头部与标签行的关键。
- **图标（只用默认图标集，逐个核对 `gpui-kit-assets-0.6.6/default-icons.txt`）**：`BookOpen`、`RotateCw`、`Settings`、
  `Minus`、`Search`、`Eye`、`EyeOff`、`Plus`、`ArrowDown`、`Delete`、`Replace`、`Ellipsis`、`Check`、`ArrowRight`。
  默认集没有 Git 字形，所以 `BookOpen` 代 Git 日志图标、`ArrowDown` 代"更新所选"、`Replace` 代"与当前分支比较"、
  `RotateCw` 代"提取"、`ArrowRight` 代远端引用图标（每处都在代码注释里写了"默认图标集无对应字形，暂用 X 代替"）。
- **对照 Windows 源码的差异清单**（源码有什么 / 实现了什么 / 缺什么 / 谁来做）：

  | # | 源码 | 现状 | 缺什么 | 归属 |
  | --- | --- | --- | --- | --- |
  | 1 | 三栏：引用 19% / 提交 57% / 提交详情 24%，可拖拽（`git-log-tool-window.tsx:614-706`） | 只做前两栏（引用 200px + 提交） | `提交详情` 第三栏（`GitCommitInspector`：`提交文件` 树 + `提交详情` 卡片） | **主代理决策**：本面板只占中列底部（约 686px），三栏最小宽 140+320+220=680 已贴死；真机底部窗横跨整窗，要改成 `DockPlacement::Bottom` 的真底部 dock，会动到其它区域 |
  | 2 | 提交图（`git-graph-row.tsx`：竖线 + 节点圆点） | 只有引用装饰徽标 | 提交图 gutter（要 `git.log` 提供图数据） | 后端接线后再做 |
  | 3 | 筛选真的过滤（`visibleRows` 按 text/author/branch） | 输入框能输入但不参与过滤；字段按钮无菜单；计数是 `行数/行数` | 过滤与字段切换 | `git.log` 接线时一起做 |
  | 4 | 装饰开关 `showDecorations` | **已实现且可用**：眼睛按钮切换提交行的引用徽标 | — | — |
  | 5 | 引用树：远端按远端名分组、`标记(我的分支)`、ahead/behind 计数、右键菜单 20+ 动作、分区折叠 | 分组只到 `本地/远程/标签`，远端引用平铺 | 远端分组、标记、ahead/behind、右键菜单、分区折叠 | 后端 + 后续步骤 |
  | 6 | 标题栏 `日志：{name}` 点击回到"显示全部引用"，名字来自引用树选中项 | 固定 `日志：全部`，按钮未接行为 | 选中引用 → 重新加载日志 | `git.log` 接线 |
  | 7 | 错误条（`loadState=failed`）、空状态（无仓库/加载中/失败/无匹配/无提交）、`加载更多提交` | 未做（静态数据不会失败） | 加载状态机 + 分页 | `git.*` 接线 |
  | 8 | 控制台 `GitExecutionConsole`（命令记录 + 清空/复制/自动换行/滚动到底/查找） | 只有源码的空状态文案「Git 命令及其输出将显示在这里。」 | 整个控制台 | 后续步骤 |
  | 9 | 两个标签是无样式的 `<button>`，只有 `aria-selected` | 用 `ListItem` 选中态 + 主题色给出可见的当前标签 | —（没用 `TabBar`：它自带底栏底色与指示条，源码这条横带没有） | — |
  | 10 | 底部工具窗横跨整个窗口宽度 | 只占中列底部（沿用现有 `v_split` 接线） | 真底部 dock | **主代理决策**（同 #1） |
  | 11 | `只读` 文案 + `隐藏` 按钮（`git.log.hide`） | 文案与按钮都画了 | `只读` 只是状态文案；`隐藏` 需要 `DockArea` 层面的可见性控制 | **主代理决策** |
- 编译：`cargo check --bin shell-probe` 共 5 次（用满上限）。第 1 次 6 个图标名错误（`GitCommitHorizontal`/`RotateCcw`/
  `Trash`/`GitBranch`/`ListTree` 不在默认图标集）外加并行子代理 `panels.rs` 的 1 个错；第 2、3 次 **0 error**；
  第 4 次（按源码改写后）1 个错：`children()` 收到惰性迭代器时 item 是迭代器本身而不是元素
  （`Option<Map<..>>`），改成先 `collect::<Vec<AnyElement>>()`；第 5 次 **0 error**。
  第 5 次之后只删了一个被编译器判定"从未使用"的常量（`LOG_SCOPE_MAX_WIDTH`）；删声明不可能引入编译错误，
  按 5 次上限没有重跑。
- 现存 warning（都不是本步引入的新 API 问题）：`panels.rs` 的 `PanelKind::Terminal` 变体已不被构造
  （底部面板被本步换成 `BottomPanel`，建议 `panels.rs` 的归属者删掉该变体与它的渲染分支）；
  `workspace.rs` 的 `files_panel` 字段未读（本步之前就未读）。
- 本机没有 `rustfmt` 组件，未跑格式化；新文件按周围文件的手工风格对齐。

---

**本轮结论（方向改定 + 行高/布局两个真 bug，2026-09-24）**：

- **方向改定（维护者要求）**：界面规格从 Windows 端改为 **macOS 端源码**，先产出 `gpui/UI-MAP.md`（区域清单 + 逐元素 → gpui-kit 对应 + 优化点），再按它一比一复刻；实现只允许用 gpui-kit 0.6.6 源码里真实存在的组件。Windows 端与截图降为旁证。§0.3、§1 已按新口径重写。
- **调研方式**：8 个并行子代理，每个负责一个区域，产出 `.artifacts/ui-map/0X-*.md`（中文、逐元素带 `文件:行号`）。区域划分：App 外壳 / 工作台骨架 / 资源管理器 / 编辑区 / Git / 终端-运行-调试 / 搜索-语言-数据库 / 组件与 token。
- **两个真 bug（都已修，证据 `.artifacts/p1/rows-fixed.png`）**：
  1. **`ListItem` 的 children 是竖排**：`ListItem::render` 把 children 放进一个**普通块级 `div().w_full()`**（`gpui-component-0.6.6/src/list/list_item.rs:215-221`），所以"折叠箭头 + 图标 + 名字"会各占一行，行高失控、相邻行互相覆盖。修法：行内容自己套一层 `h_flex()`。
  2. **行高由内容决定，不由 `.h()` 决定**：`uniform_list` 的行高来自**第 0 行内容的测量高度**，`ListItem` 默认 `py_1`（自然高 32）——`.h(px(24.))` 压不住内容，文字会被下一行盖掉。修法：`py_0()` + 把内容做进 24px（图标 16 / `text_sm`），并且**不要**在单元格里放超出表级行高的内容。
  3. 同源问题在 `DataTable` 上复现：`with_size(Size::Size(px(22.)))` 想对齐 macOS 提交行 22pt，结果行高量到 18、内容 30 → 行行重叠（`git-table-22.png`）。已回退为默认表级行高，22pt 留给 Git 面板专步（要么让单元格自然高也是 22，要么自绘行）。
- **布局改成 macOS 口径**：
  - 底部工具窗**横跨整个工作区**（= macOS 的"底窗横跨左栏 + 编辑区"）：`center = v_split[ h_split[项目 320 | 编辑区 | 右栏 360], 提交记录 260 ]`。注意 gpui-kit 的 `DockPlacement::Bottom` 只横跨 center 列（`dock_area.rs:1415-1431`），所以底窗放在 center 的 `v_split` 里而不是用 Bottom dock。
  - 度量对齐 macOS：标题栏 **40**（gpui-kit 默认 34，用 `TitleBar::h()` 覆盖）、项目标签条 **38**、活动栏 **38**、状态栏 **24**、左栏 **320**、右栏 **360**、底部 **260**、侧栏树行高 **24** / 缩进 **14**。
  - 真机默认深色：`Theme::change(ThemeMode::Dark, None, cx)`。
  - 编辑面板 `title_bar() = false` + `inner_padding() = false`：Dock 不再多画一条 `Editor` 标签头，编辑区只剩内部那条文件标签栏。
  - 命令面板实体不再放进 Dock（真机没有这个标签页，它是浮层，留到 S8）。
- **文件树 → 编辑区打通（维护者反馈"点文件不显示"）**：`ShellPanel::files(window, cx, editor.downgrade())` 拿到编辑区弱引用，行 `on_click` 解析节点 id 里的 `file:<相对路径>` → `ShellPanel::open_document()`（按后缀选图标、>2 MiB / 二进制 / 读不到三种情况在正文给说明、同一文件不重复读盘）；`ShellPanel::editor(window, cx, root)` 不再接收内容字符串，启动时用同一条路径打开 `rust/lithe-core/src/lib.rs`。
- **本环境无法合成鼠标事件**（`SetCursorPos` 后光标停在 (0,0)、`GetForegroundWindow()` 返回 0），所以"点击打开"只能由维护者在本机验证；启动路径已验证（标签 `lib.rs` + 真实内容 + 行号）。
- **区域调研的关键发现（会改变后续实现）**：
  - 外壳是 **3 个窗口**（welcome 兼主窗口 / project per-UUID / settings 浮窗），且**九成不碰 Core**：命令表里没有 window/recent/settings/theme/notification/palette 命名空间，Rust 侧要自建整套外壳状态机（`01-app-shell.md`）。
  - **编辑区在 macOS 上是 WKWebView 里的 Monaco**，5497 行的原生 `CodeEditorView.swift` 是死代码；而 gpui-kit 有**真正的代码编辑器引擎**（`EditorState` = `gpui-base/src/input/editor/mod.rs:11`，自带行号/折叠/tree-sitter 高亮/诊断/查找替换），所以 gpui 侧不需要 WebView（`04-editor.md`）。
  - **终端没有跨端契约也不该有**（PTY 归平台层），gpui-kit 零终端能力，必须自研（`06-terminal-run-debug.md`）。
  - **Git**：提交行 22pt、作者 104 左 / 日期 110 右、引用徽标从右往左堆叠；macOS 的 diff 是"顶替编辑器"而不是页签；gpui-kit 缺提交图泳道、并排 diff、多选（`05-git.md`）。
  - **组件与 token**：macOS `accent` = primary 而 gpui 的 `accent` 是悬停底；`selection` 在 macOS 不透明而 gpui 默认 30% 透明；19 项颜色要自建 `LitheColors`；5 处 macOS 手搓实现不值得照搬（`08-components-theme.md`）。
  - 命令面板 / 设置窗口 / 通知 / 可调分栏 / 贴底输出区 5 处，gpui-kit 都有现成且更好的能力（macOS 没有命令面板、`Cancel` 不撤销设置、通知一打开就全标已读）。
- **编译**：`cargo check --bin shell-probe` 多轮 0 warning；`cargo build` 只余 linker 提示。开发版已重启并在跑（日志 `.artifacts/p1/run.log`）。参考真机 `lithe-windows`（PID 25792）保持常驻未动。

---

**本轮结论（界面规格文档 + 骨架/状态栏/Git 工具窗按 macOS 重做，2026-09-24 下半场）**：

- **`gpui/UI-MAP.md` 成型（900 行）**：第 1 章是硬规则（单位换算、token 映射与两个语义陷阱、gpui-kit 实现规则与坑、浮层必须先挂层、版本陷阱清单），第 2 章是 8 个区域的"组成 / 度量 / 元素 → gpui-kit 对应 / 复刻要点 / macOS 做得不好处 / 未查清"，第 3 章是验收清单。区域调研原文在 `.artifacts/ui-map/01..09-*.md`。
- **骨架按 macOS 度量**（`workspace.rs`）：标题栏 40（`.h()` 覆盖组件默认 34）、右内边距 10，**左内边距不写死 76**（那是 macOS 给交通灯留的位置，`TitleBar` 内部已按平台给 80/12，写死会在 Windows 上留白）；项目标签条 38；活动栏 38（5 个图标 + 底部设置齿轮）；状态栏 24 且按 macOS `statusBar` 的组成填了真实条目（`1:1` / `UTF-8` / `4 个空格` / 只读锁 / `Java 项目模型已就绪` / `总计 — · Lithe —` / `0 FPS` / `没有更改`）；标题栏分支名读真实 `.git/HEAD`。
- **Git 工具窗按 macOS `GitLogView` 重做**（`bottom_panel.rs`，32507 → 63742 字节）：标题栏与页签合并成一行（`Git` + `日志：所有引用` + `工作树` + `控制台`），三栏 `h_resizable` 220(min180) / 弹性(min340) / 350(min250)，第三栏再 `v_resizable` 分成"提交文件 + 提交详情"；提交行自绘（作者列 104 左、日期列 110 右、徽标从右往左堆叠）。缺口在文件头登记 11 项（提交图泳道、diff、多选、动态栏宽上限、右键菜单、控制台折叠搜索…）。
- **本轮又踩到三个 gpui 布局地雷（都已写进 `UI-MAP.md` §1.3 与代码注释）**：
  1. `ListItem` 的 children 是竖排（内部是普通块级 `div`）→ 一行里的图标/文字必须自己套 `h_flex()`。
  2. 行高由内容决定，不由 `.h()` 决定：`uniform_list` 取第 0 行的测量高度，`ListItem` 默认 `py_1` 会溢出；`DataTable` 的 `with_size(Size::Size(px(22.)))` 也压不住（实测"行高 18 / 内容 30"互相覆盖，证据 `git-table-22.png`）。
  3. **`uniform_list` 放进 `h_resizable` 的中间栏后视口高度是 0**：诊断输出 `visible_range` 为 `0..0`/`0..1`，屏幕上一个像素都不画（`ListSizingBehavior::Auto` 也救不回来）。改为固定行高的 `v_flex()` 直接铺行后立刻正常（证据 `commit-rows-flex.png`）。接真实 `git.historyPage`、数据量大到需要虚拟化时再回来解决。
- **一个未解决的已知问题（用满 5 次尝试后按规则登记跳过）**：提交行的提交信息文字**顶部被切掉约 1/4 行高**。已排除：行高（`h(22)` → `min_h(22)`）、`line_height(22)`；当前判断是行盒小于字体 ascent+descent，而 `overflow_hidden` 裁掉了字顶。下一步可试"`line_height` 给到比行高更大"或"去掉 `overflow_hidden` 只留 `text_ellipsis`"。作者/日期两列没有 `overflow_hidden`，完整显示。
- **编辑区"只有一行、没铺满"的根因（已修，维护者反馈）**：多行输入在组件里走 `.h_auto()`（高度 = 内容高度），而内容高度来自 `LayoutMode::CodeEditor { rows }`，**`rows` 默认只有 2**（`gpui-component-0.6.6/src/input/input.rs:706-709`、`gpui-base-0.6.6/src/input/base/mode.rs:83-85`）。所以不给高度时编辑区就是两行高、下面整片空白。修法：`Editor::new(&state).h(relative(1.))`（等价于组件自带的 `Input::full_height()`，`input.rs:250-252`）。修后编辑区铺满并显示真实文件内容 + 行号（证据 `editor-filled.png`）。**这条对所有多行输入都成立**，已写进 `UI-MAP.md` §1.3。
- **证据**：`.artifacts/p1/round2-lh.png`（整体）、`zoom-git-round2.png`（工具窗）、`commit-rows-flex.png`、`zoom-commit-final.png`（行裁切问题）、`metrics-aligned.png`（度量对齐后）。
- **下一步建议（按 `UI-MAP.md`）**：① 把 `Root::render_dialog_layer` 等三层挂上（现在对话框静默不显示），再做命令面板浮层（`window.open_dialog` + `Command`，路径与证据在 `gpui/research/gpui-kit-overlay-howto.md`）；② 项目树补 Git 状态字母 + 右键菜单（`git.status` 已有契约）；③ 顶栏/状态栏剩下的待接线项（面包屑、内存/帧率、运行控件）。

---

**本轮结论（右栏/浮层/资源管理器/Git 真实数据，2026-09-24 第四轮）**：

四个并行子代理 + 主代理接线，全部落在 `gpui/`：

- **浮层三层挂上（原来是坏的）**：`workspace.rs` 的 `render` 根部补 `Root::render_dialog_layer / render_sheet_layer / render_notification_layer`（官方形态 `gpui-kit-0.6.6/tests/overlays.rs:20-22`）。三个 `let` 必须写在持有 `cx` 共享借用的 `render_*` 之前（借用顺序）。
- **命令面板做出来了并已实机验证**：双击 ⇧（Windows 对修饰键只发 `ModifiersChanged`，所以用 `on_modifiers_changed` + 350ms 时间戳，不能 `on_key_down`；根 `div` 必须 `track_focus` + 开局 `window.focus`）或项目标签条右端搜索按钮唤起；`window.open_dialog` + `Command::bordered(false)`；Esc / 点外关 / 焦点归还全部由 Dialog 自带。证据 `.artifacts/p1/palette-autotest.png`（主代理临时在第 3 帧自动开了一次，验证后已删除临时代码）：面板居中弹出、分组 `窗口`、9 个条目、首项高亮、背景遮罩都在。真生效 2 条（`大纲` / `项目统计`），7 条 toggle-* 禁用并写明原因，其余 24 条登记待接线。
  - ⚠️ **新踩的坑（已写进 `UI-MAP.md` §1.3）**：`open_dialog` 不能在窗口首帧之前调用 —— 它内部走 `has_active_dialog` → `Root::read`，会 `expect` 窗口根是 `Root`（`root.rs:176-182`、`window_ext.rs:162`）；而子视图首次 render 发生在 `Root::new` 内部，此时窗口根还不是 `Root`，直接 panic（本项目实测两次）。浮层只能由用户输入触发。
- **右侧栏改成 macOS 口径**：`项目统计`/`大纲` 从"常驻 360pt 第三列"改成 `DockPlacement::Right` 的 dock（尺寸 360、可折叠、**默认收起**，新建 dock 默认是 open 所以要显式 `toggle_dock` 收起），行尾加 40pt 右活动栏（`pt(4)`、`gap(4)`、`bg(title_bar)`）两个按钮切换，选中态读真实开合。macOS 的 hover 型（60ms 延迟关）用点击代替，差异写在注释里。**效果**：编辑区铺到窗口右缘，右侧那大片空白没了。
- **资源管理器补全**：Git 状态字母（真实 `git.status`：`{root}` → `repositoryRoot/branch/ahead/behind/changes[]{path,status,staged,worktree,untracked}`，规则照 `displayStatus`：冲突 `!`、未跟踪 `A`、否则 worktree→index；9pt bold 等宽，只在文件行；失败即不显示字母、不弹错）+ 右键菜单（**真生效**：打开 / 复制路径 / 复制相对路径 / 在文件资源管理器中显示；**禁用占位**：新建文件/新建目录/重命名/移到废纸篓，待 Core 文件写命令接线）+ 活动文件改强调底（`.selected()` 会被 `component::tree::Tree` 覆盖，改为行内自绘 `list_active`）。
- **项目树改成真递归**（主代理改 `files.rs`）：原来的"两层"假树换成按 `/` 聚合的**任意深度树**，目录优先 + 名字小写升序，节点 id 仍是 `dir:<相对路径>` / `file:<相对路径>`；渲染上限从 500 提到 5000（本仓库约 4.8 千文件 = 全显示）。
- **底部 Git 工具窗接真实数据**（`bottom_panel.rs`）：接了 `git.references`（引用树 + 当前分支 + ahead/behind）、`git.historyPage`（提交列表，第一页 limit 100，有游标立即 `git.historyCursorClose` 释放 Core 的 git log 流）、`git.status`（分支/仓库根兜底）、`git.commitFiles`（点提交行 → 多层文件树）；`git.*` 失败/非仓库时**保留占位 + 一行提示**（非仓库显示 macOS 原文「当前项目不是 Git 仓库」）。提交信息字顶裁切问题按"去掉单元格 `overflow_hidden`、只留 `text_ellipsis`"修，并加 `min_h(22)` 钉住行盒。`BottomPanel::new` 增加 `root: PathBuf` 参数，由 `workspace.rs` 传入（原来的 `std::env::current_dir()` 兜底已删）。
- **窗口尺寸按 macOS 常量**（`mod.rs`）：默认 1440×900、最小 980×640（`RootView.swift:334,351`），再夹到当前显示器可见区域。
- **证据**：`.artifacts/p1/round3.png`（整体：右栏收起、真递归树、真实提交历史）、`zoom-round3-commits2.png`（提交行）、`palette-autotest.png`（命令面板）。
- **剩下的大件（缺口清单在 `UI-MAP.md` 各区域"未查清/缺口"里）**：提交图泳道、并排 diff、提交行多选与键盘、Worktrees/Console 页签、终端（要自研 PTY + VT 渲染）、面包屑进状态栏、内存/帧率真实数据、文件监听（FSEvents 350ms 去抖）、编辑器 minimap/多标签/分屏。

---

**本轮结论（可滚动性，2026-09-24 第五轮，维护者反馈"文件树/编辑器多了滚不动"）**：

- **滚动条改成常显**：gpui-kit 默认 `ScrollbarMode::Scrolling`（滚动时才出现、停下淡出，`gpui-base-0.6.6/src/scrollbar.rs:48-56`），IDE 观感是常显可拖。`mod.rs` 里加 `Theme::set_scrollbar_mode(ScrollbarMode::Always, cx)`（`theme/mod.rs:250-251`）。改完编辑器右侧、文件树右侧、提交列表右侧都能看到滑块（证据 `.artifacts/p1/scrollbars.png`）。
- **代码编辑器不吃滚轮（真 bug）**：编辑器元素（`gpui-base-0.6.6/src/input/base/element.rs:202-213`）只有 `position: absolute` + `100%`，**没有 `overflow: scroll`**；gpui 只把滚轮交给"命中元素带 `Overflow::Scroll`"的那个（`gpui-pre-0.3.6/src/elements/div.rs:3332-3370`），所以内容多了滚不动。修法：在 `panels.rs::render_editor` 的包裹层 `on_scroll_wheel`，把 `event.delta.pixel_delta(line_height)` 交给 `InputBaseState::set_scroll_offset`（`input/base/state.rs:2806-2817`，自己 clamp、下一帧生效）。
- **文件树/列表的滚轮**走组件自带的机制（`uniform_list` 自己把 `overflow.y = Scroll`，`gpui-pre/src/elements/uniform_list.rs:32`，并用 `last_item_size` 判断是否可滚），只要内容高于视口就生效；这次日志里确认到它的可滚区间（滚动条出现）——**滚轮是否真的滚得动仍要维护者实机确认**（本环境注入不了鼠标事件）。
- 另一条要记住的布局事实：Dock 给每个面板外面套的 `tab-content` 是 `overflow_y_scroll`（`tab_panel.rs:728-734`），所以面板内容比面板高时滚的是**整块面板内容**；面板里某个列表要独立滚动就得自己接滚轮（编辑器就是这么修的）。
- `cargo check --bin shell-probe` 0 error 0 warning；开发版已重启在跑。

---

**本轮结论（窗口策略按平台区分 + 根视图尺寸的写法，2026-09-24 第六轮，维护者反馈"屏幕 1920×1080、窗口没占满"）**：

- **真屏是 1920×1080 + 125% 缩放**。我此前看到的 1444×812 / 1550×878 / 1536×864 都是这台机器上"物理 / 逻辑 / DPI 不感知虚拟化"三种坐标混着报出来的数，**不能拿来判断窗口有没有铺满**。已写进 `UI-MAP.md` §1.3 的"单位对账"。
- **窗口策略改成按平台区分**（依据是实测真机，不是照抄常量）：
  - **Windows：启动即最大化** —— 实测已安装的 `lithe-windows` 主窗口 `IsZoomed()=true`、尺寸 = 工作区（Windows 上 IDE 的普遍做法）。实现是开窗后 `window.zoom_window()`（macOS 分支不调用）。
  - **macOS：普通窗口 1440×900 / 最小 980×640**（`RootView.swift:334,351`，真机带交通灯不最大化）。
  - 之前是把 macOS 的窗口尺寸照搬到所有平台 → Windows 上开出一个比屏幕小的窗口，四周露出桌面，这就是"没占满"的真正原因。
  - ⚠️ `WindowBounds::Maximized(bounds)` 在 0.6.6 的 Windows 实现里**只当还原尺寸用、不会真的最大化**（实测 `IsZoomed()` 仍 false）；`window.zoom_window()` 才有效（`gpui-pre-0.3.6/src/window.rs:2834-2837`、`gpui-pre-windows-0.3.6/src/window.rs:944-951`）。
  - gpui-kit 官方示例是"不给 `window_bounds`"（`WindowOptions::default()`），gpui 会把 `DEFAULT_WINDOW_SIZE = 1536×1095` 夹到显示器可见区域并居中（`platform.rs:427-435`）—— 它只给普通窗口，**不最大化**，所以"最大化"这步必须自己写。
- **根视图尺寸的写法**（⚠️ 这一轮的两条结论**后来被第 8 轮全部推翻**，保留原文以记录误判过程）：
  - ~~`.size_full()` → 内容大 1.25 倍~~ —— 那是**坏掉的截图工具**造成的假象（DPI 不感知 → 抓回 1458×819，恰好裁掉空白）；
  - ~~`.w(viewport.width / scale)` 才对齐~~ —— 恰恰相反，**除 scale_factor 才是错的**；
  - **第 8 轮的正确答案**：根视图用 **`.size_full()`**，`viewport_size()` **不要**除 `scale_factor()`；截图工具必须先声明 per-monitor-v2 DPI 感知。详见 `UI-MAP.md` §1.3 的「根视图尺寸」与「截图工具要求」两节。
- 验证方式说明：我用 Win32 `MoveWindow` / `ShowWindow`（只改窗口几何、不模拟输入）来回改尺寸并抓 `PrintWindow` 帧；每次测完都把窗口还原。

---

**本轮结论（窗口口径改为对齐 Dodona，2026-09-25 第七轮，维护者指出同目录 Dodona 的界面是正常的）**：

- **上一轮"按 Windows 产品最大化"的结论是错的**，本轮推翻。维护者指出：`D:\developmentProjects\rust\Dodona`（同目录的 GPUI Kit 参考应用）界面正常 → **以它为准**。
- **Dodona 的窗口口径**（`Dodona/crates/dodona/src/main.rs:91-108`）：`visible_bounds() * 0.94`、夹在最小 `1024×680` 与可见区之间、居中、`WindowBounds::Windowed`。**不是最大化**，也不做任何 `/ scale_factor` 的手工换算。
- **本项目已按它改回**：删掉第 6 轮加的 `cfg!(target_os)` 平台分支与 `zoom_window()`，`startup_window_bounds` 与 Dodona 逐行同构。**实测两个进程的窗口矩形完全相同**：`shell-probe` 与 `dodona` 都是 1458×819 @ (39,22)；证据 `.artifacts/p1/dodona-aligned.png`（我的）与 `dodona-window.png`（Dodona 的）。
- **但根视图尺寸不能照抄 Dodona**：Dodona 用 `.size_full()`，那是 gpui-kit **0.6.1 / gpui-pre 0.3.4**；本项目钉的是 **0.6.6 / 0.3.6**，在这个版本上 `.size_full()` 会把内容画成 **1.25 倍大**（状态栏被挤出窗口、右栏截断，证据 `startup-max.png`），必须用 `.w(viewport/scale).h(viewport/scale)`。这条版本差异已写进 `UI-MAP.md` §1.3 的对照表。
- **Dodona 作为对照基线**（它已构建：`Dodona\target\debug\dodona.exe`）：以后验证界面口径时可以直接起它做"几何 + 观感"对照，跑完即关。它的界面结构是成熟的 gpui-kit 写法：标题栏 → 工具栏行（新建连接/新建查询/历史/SQL 库/SQL 文件/驱动管理）→ 左侧栏 | 右侧（标签条 + 二级工具栏 + 编辑区）→ 底部结果面板（`结果 / 执行计划` 页签）。
- **教训（写进文档，避免再犯）**：不要在 DPI 缩放的机器上用 Win32 的 `GetWindowRect`/`IsZoomed` 去判断 gpui 窗口的尺寸与状态；判定口径一律是"抓帧对比 + 与参考应用对账"。

---

**本轮结论（真因找到：截图工具 + 根视图除错缩放比，2026-09-25 第八轮）**：

维护者指出"你的截图分辨率不对、少了一大截，实际是 1920×1080"，这一句直接定位了两个我此前都没看见的问题：

1. **截图工具是坏的（DPI 不感知）**：旧 `capture-screenshot.ps1` 在 125% 缩放的机器上，`GetWindowRect`/`PrintWindow` 只拿到**虚拟化尺寸**（1458×819），而窗口真实是 **1823×1024** —— 抓回的位图**恰好把右侧与底部的空白裁掉**，所以内容看起来"铺满了"。这就是我一直复现不出"没占满"的原因。修法：脚本开头先 `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)`，并新增 `-WholeScreen`（抓 1920×1080 整屏，用于核对窗口占屏比例）。改完立刻抓到 1823×1024，**空白一眼可见**。
2. **根视图尺寸算错了**：旧代码写 `.w(viewport_size().width / scale_factor())`，而 `px()` 用的就是 `viewport_size()` 那套 DPI 无关单位，**再除一次 1.25 就把内容缩成窗口的 78%**（右侧 + 底部大片空白）。改成 **`.size_full()`**（与参考应用 Dodona 一致，`Dodona/crates/dodona/src/app/mod.rs:153-157`）后内容铺满。
   - 我此前那条"`size_full()` 会大 1.25 倍"的结论是**错的**，来源正是第 1 条的坏截图工具（内容被裁掉后看起来像溢出）。已在 `UI-MAP.md` §1.3 更正。
3. **窗口口径**：保持与 Dodona 同构（可见区 94%、居中、普通窗口）。整屏证据 `.artifacts/p1/fixed-screen.png`（1920×1080，窗口居中占 94%）与 `.artifacts/p1/fixed-window.png`（1823×1024，内容铺满）。
4. **本轮改动**：`gpui/capture-screenshot.ps1`（DPI 感知 + 整屏模式）、`gpui/shell/src/bin/shell_probe/workspace.rs`（根视图改回 `.size_full()`，诊断输出改为打印 viewport + scale）、`gpui/shell/src/bin/shell_probe/mod.rs`（窗口策略与 Dodona 同构），以及 `UI-MAP.md` §1.3 的更正。

---

## 5. 本轮进行中（2026-09-25：规格来源改回 Windows + 清空重写第 1 轮）

> 这一节是新口径的施工记录，**旧的 §1–§4 是 macOS 口径的历史**，保留只作"这个交互原本怎么工作"的参考。
> **规格真源 = Windows 前端**（`windows/tauri/src/`），执行提示词 = `docs/development/gpui-ui-windows-rewrite-prompt.md`。

### 5.1 新口径的分阶段（取代 §2 的旧表）

| 阶段 | 做什么 | 状态 |
| --- | --- | --- |
| 0 | 清空 `shell_probe/` 旧实现（保留 `Cargo.toml` 与 bin 名），搭最小骨架，`cargo check` 可跑 | ⏳ |
| 1 | 外壳：标题栏（40px、自绘三键 56×40）+ 项目标签条（32px）+ 左右活动栏（38px）+ 状态栏（24px）+ Dock 三栏与 4px 间隔 | ⏳ |
| 2 | 侧栏项目树（行高/缩进/图标/选中/悬停/右键菜单）+ 全局面包屑 | ⏳ |
| 3 | 编辑区（内部标签栏、`← →`、文件类型图标、脏标记、空状态、真实文件内容） | ⏳ |
| 4 | 底部 Git 工具窗（标题栏 + 页签 + 三栏 + 提交表 + 引用树 + 筛选条），接真实 `git.*` | ⏳ |
| 5 | 终端外壳（页签 + 输出流 + 发送命令行；宿主 spawn `cmd`/`powershell`，不做 VT 模拟） | ⏳ |
| 6 | 右侧工具窗 + 浮层（命令面板 `Command` + `window.open_dialog`） | ⏳ |
| 7 | 调试界面骨架 + 逐区域与 Windows 真机并排复核 | ⏳ |

### 5.2 本轮已经落地的工程决定

1. **代码基线**：工作区里 `gpui/` 的 `Cargo.toml`、`Cargo.lock`、`shell/Cargo.toml`、`shell/src/main.rs` 被上一轮"清空"误删（计划只要求清空 `shell_probe/` 下的实现），已从 `HEAD` 恢复；`gpui/docs/archive/ui-map-macos.md` 同样被误删，已恢复（计划里它是配套的行为对照文档）。
2. **模块布局**：外壳按区域拆成 `shell_probe/shell/{mod,title_bar,project_tabs,activity_bar,status_bar}.rs`，每个区域是一个**无状态渲染函数**（`-> impl IntoElement`），状态集中在 `workspace.rs` 的 `ShellWorkspace` —— 这样区域之间不会因为共享 `Entity` 互相打架。
3. **编辑区/底部窗的 Dock 口径**：Windows 默认 `terminalWidthMode === "editor"`，底部窗**嵌在中央编辑器列内**（不是横跨工作台）。gpui-kit 的 `DockPlacement::Bottom` **本来就只横跨 center 列**，所以这一版用真 Bottom dock（旧实现"放进 center 的 `v_split` 以横跨左栏+编辑区"是 macOS 口径，已作废）。
4. **重试上限从 5 次改成 3 次**，超限一律登记到新增的 **`gpui/BLOCKERS.md`**，不再原地打转。
5. **构建串行化**：`cargo check` 独占 `gpui/target` 锁，所以**只有主代理跑构建**，区域子代理只写代码、每个 API 都必须先在 gpui-kit 0.6.6 的 registry 源码里 `rg` 到定义。
6. **新增规格文档**：`gpui/UI-MAP-WINDOWS.md`（逐区域对应表，原先被引用但不存在）与 `gpui/research/gpui-kit-0.6.6-api.md`（0.6.6 真实 API 清单）由并行子代理产出。

### 5.3 维护者拍板（2026-09-25，"按推荐"）

`gpui/GRILL.md` 的 12 条 frontier **全部按推荐执行**：① 代码基线改为"恢复 + 按 Windows 口径削减"（已执行：恢复了被误删的 5 个文件）；② 分支从当前 `main` 开本地分支、不做 `gh` 尝试；③ 冲突时以重写提示词为准；④ `windows/tauri/src/` **只认呈现层**（buffer/store 是前端 zustand 的，不重写）；⑤ 玻璃层本轮不做；⑥ 底部窗用"中央列内的分栏"（**比 DockPlacement::Bottom 更贴 Windows**：`MainLayout` 本身就是 flex + `ResizablePane`，没有 dock 系统）；⑦ "一次"= 一个根因的一次修复，不是一次 `cargo check`；⑧ `0 error` 硬、`0 code warning`（`linker_messages` 见 `BLOCKERS.md` B6）；⑨ 命令白名单 6 条；⑩ 终端 = 输出查看器 + 最小 ANSI 清洗，显式声明能力边界；⑪ 资产源用 `AllAssets`；⑫ api.md 已产出并定为唯一 API 真源。

### 5.4 阶段 1-3 完成情况（验证过的）

| 阶段 | 内容 | 证据 |
| --- | --- | --- |
| 1 | 外壳：标题栏 40（自绘三键 56×40）+ 项目标签条 32 + 左右活动栏 + 状态栏 24 + 三栏与 4px 间隔 | `.artifacts/p1/stage1-window.png`（逐像素：50/1.25=**40**、40/1.25=**32**、31/1.25=**24.8**） |
| 2 | 侧栏项目树：**真递归任意深度 + 真实 `workspace.snapshot`**（`S1_EXPLORER rendered=4865`）、行高 24/缩进 10+16×depth、图标、活动文件高亮、真过滤搜索 | `.artifacts/p1/stage2b-window.png` |
| 3 | 编辑区：内部标签栏（`← →` + 类型图标 + 文件名 + 关闭）+ `Editor`（真实内容 + 行号，`.bordered(false)` + `h(relative(1.))` + 滚轮接线）+ 空状态 + 多标签 + `path-shortener` 同名区分 | 同上（`CLAUDE.md` 标签 + 真实正文；被打开的文件来自维护者点击） |
| 3.5 | 底部窗**嵌在中央列内**（默认 320 + 4px 热区）—— Windows 默认口径 | 同上（`底部 Git 工具窗（阶段 4）` 占位在编辑岛下方、**不横跨工作台**） |

**编译**：`cargo check --bin shell-probe` **0 error / 0 code warning**（`cargo clean -p` 后全量验证）。
**关键接口纪律（本 crate 是 edition 2024）**：只读渲染函数必须收 `&Window` / `&App` —— 收 `&mut` 会让 `-> impl IntoElement` 的可变借用被捕获，同一表达式连续调用两次就报 E0499（本项目第一次构建一次报了 12 个）。详见 `shell/mod.rs` 的模块文档。

### 5.5 还欠的（下一步输入）

1. **阶段 4**（底部 Git 工具窗：6 条 `git.*` + 游标释放）与**阶段 5**（终端：spawn `powershell`/`cmd` + 最小 ANSI 清洗）子代理运行中。
2. **把代替字形换成真字形**：资产源已是 `AllAssets`（1830 个 Lucide 字形，含 18 个 `git*.svg`），但 `activity_bar` / `status_bar` / `explorer` / `editor` 仍在用 101 项的 `component::IconName` 与代替字形。换成 `gpui_kit::assets::IconName`（`GitBranch` / `GitGraph` / `X` / `Lock` / `Image` / `FileCode` …）即可。
3. **侧栏/右栏的拖拽改宽**（`ResizablePane` 的 4px 热区）、**项目标签条的横向滚动**、**编辑区 `TabVariant` 的 28px 标签高**（组件在 render 里最后写死 `.h(36).text_sm().gap(16)`，外层覆盖不掉；要么接受、要么自绘标签条、要么给上游加覆盖点 —— 已登记）。
4. 阶段 6（右侧工具窗 + 命令面板浮层）、阶段 7（调试骨架 + 与真机并排复核）、`verify-agent-notes.mjs`。

### 5.6 与 Windows 真机并排复核（2026-09-25，证据 `.artifacts/p1/ref-windows-window.png`）

真机（`D:\Programs\Lithe\lithe-windows.exe`，维护者当时开着设置对话框、活动栏处于**展开**态）与我的外壳逐项对照：

| # | 项 | 真机 | 我的实现 | 判定 |
| --- | --- | --- | --- | --- |
| 1 | 标题栏左端 | 项目芯片（logo + `jwg`）+ 分支 `main` | 项目名 + 分支 | ✅ 结构一致（我的还缺 logo 与项目下拉触发器） |
| 2 | 标题栏右端 | 搜索 /「有可用更新」/ 最小化 / 最大化 / 关闭 | 只有自绘三键 | ⚠️ 缺搜索按钮与更新控件（`title-bar.tsx:242-254,347`） |
| 3 | 项目标签条 | **不可见**（单项目时 `shouldShowProjectTabBar` 判为隐藏） | **恒显示** | ❌ **要改**：`projects.len() > 1` 才显示 |
| 4 | 状态栏右端 | `总计 295.1 MB · Lithe 10.8 MB`、`6 个更改` | `总计 0.0 MB · Lithe 0.0 MB`、无更改勾 | ⚠️ 文案与顺序一致，数值待接真实内存 / `git.status` |
| 5 | 左活动栏 | 维护者把它**展开**成了「图标 + 标签」竖列表（`activityRailExpanded`）；默认仍是 38px 折叠 | 默认 38px 折叠图标条 | ✅ 默认态一致 |
| 6 | 左侧栏内容 | 此刻是 **Git 源代码管理视图**（更改 / 查看差异 / 历史记录 + 文件树 + 底部「提交说明」框与提交按钮） | **恒是项目树** | ❌ **真缺口**：左侧栏内容要跟着活动栏切换（我目前只切了底部窗） |
| 7 | 编辑区 | 被对话框遮挡，只见空态 | 标签栏 + 正文 / 空态 | — |
| 8 | 右侧工具窗 | 提交文件 `0 个文件` + 日期列表 + `没有更改的文件` + 提交详情（`Merge remote-tracking branch 'main/main'`、作者、日期） | 阶段 4 的第三栏（提交文件 / 提交详情） | ✅ 结构对得上 |
| 9 | 窗口尺寸 | 1938×1098（接近满屏、可最大化） | 1823×1024（Dodona 口径：可见区 94%、居中、普通窗口） | ✅ **按决策 #2 有意不同**，不是缺陷 |

**结论**：#3 与 #6 是这一轮新发现的两个真差异，要修；#2/#4 是"还没接线的元素"，已在缺口清单里。
另外确认一条：真机**没有**把 `toggle_menu_bar` / `uses_native_window_chrome` 画出来，与提示词"不做"的判断一致。

---

## 6. 本轮（2026-09-25 下半场）：按官方文档对齐 + i18n + 主题系统

### 6.1 依赖口径改成官方文档的写法（并纠正一条被传错的结论）

官方文档（默认版与 `versions/main` 都）写 `gpui-kit = "0.6"`。原先的 `=0.6.6` 已改成 `"0.6"`，锁文件仍解析到 **0.6.6**（最新已发布；本机 `cargo` 连不上 registry，只能用它）。同时加了 `rust-i18n = "4.2"`（解析到 4.2.2，本来就在依赖树里）。

**⚠️ 纠正一条长期传错的结论**：`UI-MAP.md` / `PLAN.md` 过去写"在线文档描述的是未发布的 0.7.0、一律不可信"。
核对结果：**默认版文档描述的确实就是已发布的 0.6**（用 `cx.open_window` + `Root::new`，与 0.6.6 一致）；
只有 **`versions/main`** 才描述未发布 API。两处文档都写 `gpui-kit = "0.6"`。
已把 `UI-MAP.md` 里那条硬规则改掉，并在新增的 `gpui/docs/README.md` 里写清两套文档的差别与证据。

**为什么不能真按 `versions/main` 写**：它用的是 `gpui_kit::open_window(...)`（由它自己包 `Root`），
而 0.6.6 里**没有这个函数** —— `gpui-kit-0.6.6/src` grep `pub fn open_window` 零命中，
它自己的文档注释（`lib.rs:37,132`）用的就是 `cx.open_window`；0.6.6 里也没有应用层的 `WindowExt`。
想按 main 写就必须把依赖换成 gpui-kit 的 git main，而**本机 TLS 坏了**
（`schannel: SEC_E_NO_CREDENTIALS`），`D:\ProgramData\rust\cargo\config.toml` 里注释掉的本地代理
`127.0.0.1:31180/31181` **没有在跑**（端口关闭）。→ 结论：规格以**默认版文档 + 本地 0.6.6 源码**为准，
`versions/main` 当**前瞻**读。等上游正式发版后再整体迁移新 API（维护者已确认这个顺序）。

### 6.2 文档本地镜像（因为 shell 没有 TLS，子代理读不到在线文档）

`gpui/docs/gpui-kit/versions-main/` —— **160 个 zh-CN 页面**，用
`obscura.exe`（`C:\Program Files\obscura-x86_64-windows`，headless 浏览器 v0.2.3）
的 `fetch <url> --dump original` 抓raw markdown。页面清单来自 `versions/main/llms.txt`。
坐标、重抓脚本、两个踩过的坑都写在 `gpui/docs/README.md`。
⚠️ **踩坑记录**：`llms.txt` 里的链接**已经带版本前缀**（`/versions/main/zh-CN/...`），
第一版脚本又拼了一次前缀 → 全部 404；而且"失败就 `Remove-Item`"让现象看起来像"下完又删了"。
判据必须用内容（正文以 `---` 开头 + 长度 > 500B），失败要保留现场。

### 6.3 i18n（新增）

| 项 | 位置 / 做法 |
| --- | --- |
| 资源 | `gpui/shell/locales/lithe.{zh-CN,en}.yml`，`_version: 2`，各 **4317 条 `lithe.*` + 2 条 `gpui_component.*`** |
| 生成器 | `gpui/tools/extract-locale.mjs`（读 `windows/tauri/src/i18n/locale.ts`，`--check` 可校验） |
| 初始化 | `shell-probe.rs`（bin crate 根）里 `rust_i18n::i18n!("locales", fallback = "en");` |
| 语言 | `mod.rs` 里 `gpui_kit::component::set_locale("zh-CN")`（gpui-kit 组件自带 zh-CN） |
| 覆盖组件文案 | **本轮不做 `extend!`**，理由见下 |

**两个必须记住的坑**：
1. **`rust_i18n::t!` 只替换 `%{name}`，不认 `{name}`**（`rust-i18n-4.2.2/src/lib.rs:45-91`）。
   locale.ts 用的是 `{count}` 这种写法，数据里按原样保留，所以**调用处必须自己再套一层 `{name}` 替换**，
   否则界面上会直接显示 `{count}`。
2. **`extend!(gpui_component)` 与"只依赖 gpui-kit 一个 crate"的编码规范冲突**：该宏收的是
   **ident**，展开成 `gpui_component::_rust_i18n_extend(..)` 且用 `stringify!` 当 namespace
   （`rust-i18n-4.2.2/src/lib.rs:214-219`），所以必须把 `gpui-component` 加为**直接依赖**。
   本轮以编码规范为准不启用它；`gpui_component.*` 那 2 条覆盖项先留在 YAML 里，等规则放宽再启用。
3. **尚未消费**：界面模块目前仍直接写中文（那是上一轮"逐字采用 locale.ts"的要求）。
   把 4317 条搬进 `t!("lithe.…")` 调用是下一步的机械改造 —— **i18n 现在是"已就绪"而不是"已使用"**。

### 6.4 主题系统（新增，已运行期验证）

| 项 | 内容 |
| --- | --- |
| 数据 | `gpui/themes/lithe-{dark,light}.json`，各 **60 个 colors key**；`gpui/themes/README.md` 有逐值对照表 |
| 接线 | `mod.rs::apply_lithe_theme` → `ThemeRegistry::watch_dir(<CARGO_MANIFEST_DIR>/../themes, cx, on_load)`，回调里 `Theme::global_mut(cx).apply_config(&theme)` |
| 运行期证据 | 启动日志 `S1_THEME applied=Lithe Dark`；截图 `.artifacts/p1/stage-theme.png` 采样点全部变成 Lithe 的 `#1E1F22`（原来是 gpui-kit 默认的 `#0A0A0A`） |
| 额外好处 | `watch_dir` 是**监听目录**，所以改 JSON 不用重编译；主题文件是运行时读盘 |

**⚠️ 三个 schema 坑（都核对过源码）**：
1. 文件根是 **`ThemeSet`**（`{name, author?, url?, themes:[ThemeConfig]}`），**不是 `ThemeConfig`**；
   官方文档里的 `{"colors": {…}}` 只是 colors 片段，当整份文件写会得到空 `themes`、一个主题都载不进
   （`theme/schema.rs:22-34`，加载 `registry.rs:98,152`）。
2. **解析失败的文件被整份静默忽略**（`registry.rs:252-258`）→ 格式写错只表现为"主题没生效"，不报错。
3. **`title_bar` 与 `status_bar` 必须是 `--surface` 而不是 `--background`**：真机标题栏
   （`title-bar.tsx:337`）与页脚（`footer.tsx:47`）都是 `bg-surface`（深色 `#2b2d30`）。
   另注意 `status_bar` 缺省会**回落到 `title_bar`**（`schema.rs:1014`），但两个都显式写了。
   （这条是并排截图时发现标题栏与编辑区同色才查出来的。）

### 6.5 本轮顺带修掉的差异

1. **单项目时项目标签条整条隐藏**（真机 `shouldShowProjectTabBar`）——已改，截图里标签条不再出现。
2. **底部窗可见性改成以 Git 窗格自己的「隐藏」按钮为准**（原先 ShellWorkspace 与 BottomPane 各持一份状态，会出现"外框还在、内容空了"）。
3. **图标全部换成真字形**（全仓不再有 `component::IconName`）：`GitBranch` / `GitGraph` / `Package` / `Lock` / `X` / `Image` / `FileCode` / `FileBraces` …
   （`file-json.svg` 实测不存在，JSON 用 `FileBraces`；`trash-2.svg` 不存在，用 `Trash`。）

### 6.6 已知问题（新登记）

| # | 问题 | 现象 / 根因 | 下一步 |
| --- | --- | --- | --- |
| B7 | **终端里 GBK 中文变 `�`** | PowerShell / cmd 在管道下按系统 ANSI 代码页（CP936）输出，我们按 UTF-8 `from_utf8_lossy` 兜底 → 截图 `stage-theme.png` 里 `版权所有`→`��Ȩ����`。真机 host 做了代码页检测（`windows/tauri/src-tauri/src/run.rs:1864-1903`）。 | 按代码页解码（引 `encoding_rs`）或起进程时强制 UTF-8（`chcp 65001` / `$OutputEncoding`）。**目前是可见缺陷**。 |
| B8 | **项目树渲染上限 5000 被顶到** | 日志 `S1_EXPLORER rendered=5000 limit=5000`（本轮新增了 160 篇文档 + 4317×2 条 locale，仓库文件数过 5000）。 | 提高上限，或改成虚拟列表（`Tree` 本身就是 `uniform_list`，应该能承载）。 |
| B9 | **`UI-MAP-WINDOWS.md` §2④ 与 §1.5 自相矛盾**（提交表行高能否一比一） | 子代理核对时发现，建议按"`DataTable` 行高受首行内容影响"那一版为准。 | 改文档。 |







