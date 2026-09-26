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
| 6 | 右侧工具窗（✅ **第一半完成**，见 §11）+ 浮层（命令面板 `Command` + `window.open_dialog`，⏳ 下一轮） | ⏳ |
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

---

## 7. 本轮（2026-09-25 晚场）：按《编码指南》重新分层 + rustfmt + 度量迁 rem

### 7.1 代码按"业务能力"重新分层（`gpui/crates/*`）

依据：`gpui/docs/gpui-kit/0.6.6/zh-CN/docs/coding-guides.md`（维护者指定的 0.6.6 在线文档，
已镜像到 `gpui/docs/gpui-kit/0.6.6/`）。指南的硬要求是"**按业务能力组织 crate、依赖只向下、
不要全局 `views/`·`models/`·`modals/` 目录**"，所以把原来的**单个 bin crate + 扁平目录**拆成 7 个 crate：

```text
app → workbench → {editor, explorer, git, terminal} → shared
```

| crate | 从哪来 | 公开边界 |
| --- | --- | --- |
| `shared` | 新增（把散在 explorer / bottom_panel 里各写一遍的 Core 信封层收敛）+ i18n 包装 + `locales/` | `CoreClient` `CoreError` `CoreRequest` `DEFAULT_TIMEOUT_MILLIS` `core_json` `tr` `tr_args` |
| `terminal` | 原 `terminal.rs` 1753 行 → `profile` / `ansi` / `session` / `terminal_view` / `constants` | `TerminalPane` `TerminalProfile` |
| `git` | 原 `bottom_panel.rs` 3240 行 → `model` / `log_view` | `BottomPane` |
| `editor` | 原 `editor.rs` 833 行 → `buffer` / `editor_view` | `EditorPane` |
| `explorer` | 原 `explorer.rs` 1192 行 → `model` / `explorer_view` | `Explorer` |
| `workbench` | 原 `workspace.rs` + `shell/` 四区域 | `ShellWorkspace` + 四个区域模块 |
| `app` | 原 `shell/src/bin/shell_probe/mod.rs` | bin `shell-probe`（无 lib） |

顺带：**删除 `shell/src/main.rs`**（旧的 P1 冒烟宿主 `lithe-gpui-shell`）—— 职责已被 `app` 覆盖，
指南也要求"App Shell 只组合窗口与 Feature、一个应用一个 shell"。旧 `gpui/shell/` 整体消失。

**公开 API 按指南收敛**（跨 crate 后原来的 `pub` 就是真 API）：`ActivityItem` / `StatusEntry` /
`ProjectTab` / `TerminalProfile` 加 `#[non_exhaustive]`、字段改私有、补构造函数
（`ActivityItem::new(..).bottom(true)`、`StatusEntry::new(t).with_icon(i)`、`ProjectTab::new(n)`、
`TerminalProfile::new(p).with_args(..)`）；`Buffer` 改 `pub(crate)` + `Buffer::new(..)`。
feature 的 `lib.rs` 只做明确 `pub use`，`model` / `ansi` / `session` / `*_view` 不成为 import 路径。

**验证"这是纯结构重构"**：同一渲染器复截 + 逐区域像素比对（基准 `.artifacts/p1/stage-refactor.png`
vs `.artifacts/p1/stage-theme2.png`）：标题栏 / 标签条 / 右活动栏 **0% 差异**，
其余差异全部是**数据** —— 项目树内容（本轮删/移了文档、搬了源文件）与状态栏分支名
（`main` → `feat/gpui-shell-rewrite`）。另：`cargo check --bin shell-probe` = 0 error / 0 warning，
`cargo test --workspace` = shared 3 + terminal 5 全通过。

### 7.2 rustfmt（活跃工具链装不上，改用 1.95.0 自带的那份）

指南要求"使用 `rustfmt` 并满足 workspace Clippy"。实际遇到的阻碍与结论：

1. `rustup component add rustfmt` 失败在**打开下载文件**：`D:\ProgramData\rust\rustup\{downloads,tmp,update-hashes}`
   **存在但不可写**（已实测写入失败）。
2. 放宽文件权限后暴露真因：rustup 走**清华 TUNA 镜像**，而它对该组件 **404**
   （`mirrors.tuna.tsinghua.edu.cn/rustup/dist/2026-07-09/rustfmt-1.97.0-…tar.xz`）。
3. 指定 `RUSTUP_DIST_SERVER=https://static.rust-lang.org`（PowerShell 与 `cmd` 都试）**仍走 TUNA**；
   `settings.toml`（`RUSTUP_HOME` 与 `~/.rustup`）里都没有 `[dist-server]`，机器/用户级环境变量也没有
   —— 这个镜像应该是预置在 rustup 里的，找不到可改入口。
4. **结论：不装**。`D:\ProgramData\rust\rustup\toolchains\1.95.0-x86_64-pc-windows-msvc\bin\rustfmt.exe`
   是现成可用的 **rustfmt 1.9.0**，直接调它格式化；`cargo fmt` 用不了（`cargo-fmt` 只属于活跃的 1.97）。
   本文写作时 `rustfmt --edition 2024 --check` 已通过。

### 7.3 度量迁移到 rem helper（指南的"每个裸 `px(...)` 都是 review finding"）

**rem base = 16px**，三条证据：
1. `gpui-component-0.6.6/src/theme/mod.rs:665` 默认 `font_size: px(16.)`，而我们的主题当时没有覆盖它；
2. `gpui-component-0.6.6/src/root.rs:582` 是 `window.set_rem_size(cx.theme().font_size)`
   —— rem base 就是主题字号；
3. Windows 前端（`windows/tauri/src/styles/theme.css`）**没有**给 `:root`/`html` 设 `font-size`
   → 浏览器默认 16px → 它的 Tailwind `2.5rem` 就是 40px，与调研文档的换算一致。

因为本项目所有度量都是从 Windows 的 Tailwind class **逐值搬来的**，映射规则就是**同名数字**：
`h-10`(40px) → `h_10()`、`h-8`(32px) → `h_8()`、`gap-1`(4px) → `gap_1()`、`px-2`(8px) → `px_2()`、
`max-w-50`(200px) → `max_w_50()`。**换算后渲染值必须逐像素等价**（用同一渲染器复截比对验证）。

**三类必须保留 `px(...)` 的例外**（每处都在代码里写了理由）：
1. **字号**：Lithe 的 UI 基准是 **13px**（`theme.css:112,115`），而 gpui 只有 `text_xs()`=12 与
   `text_sm()`=14 —— 改成 14 会改变设计，不是本轮该做的视觉改动。能精确对上的（12/14/16/18）换掉。
2. **圆角阶梯**：Windows 的圆角是 `--radius: 8px` 派生的 `calc(× 0.6/0.8/1/1.4)`
   = **4.8 / 6.4 / 8 / 11.2**（`theme.css:6-12`），**不在 4px 网格上**。
   ⚠️ 而且**不能**改用 `cx.theme().radius`：`ThemeConfig.radius` 是 **`usize`**
   （`theme/schema.rs:67-68`），装不下 6.4/4.8；把主题值设成 8 会让**所有 gpui-kit 组件**的圆角
   从 6 变成 8（那是 shadcn 默认，比 Lithe 的 `rounded-md` = 6.4 更不准）。所以这四级圆角作为
   **应用层具名常量**留在 `workbench`，并登记为对指南的有意偏离。
3. **非长度值**：`line_height(1.35)`、opacity、alpha 等系数保持原样。

### 7.4 主题文件新增三个字段（`themes/lithe-{dark,light}.json`）

| 字段 | 值 | 理由 |
| --- | --- | --- |
| `font.size` | `16.0` | **把 rem base 显式钉住**。默认恰好也是 16，所以不影响现状；但 rem helper 的正确性依赖它，不能靠"上游默认没变" |
| `radius` | `6` | 显式钉成 gpui-kit 的默认值（`mod.rs:665`）。Lithe 的 `rounded-md` 是 6.4，而该字段是 `usize` 只能取整 —— **6 比 8 更接近**，所以不取 `--radius` 的 8 |
| `radius.lg` | `8` | Lithe 的 `--radius-lg` = `--radius` = 8，与 gpui-kit 默认一致 |

### 7.5 本轮提交（`gpui/`）

`docs(gpui): 文档分层与过期文档清理` → `refactor(gpui)` ×7（shared / terminal / git / editor /
explorer / workbench / app，按依赖顺序）→ 随后：度量 rem 迁移、README/PLAN 同步。

⚠️ 这 7 个提交按**路径**切分、按**依赖**排序，但**只有最后一个 tip 是绿的** —— 中间状态缺 `app`
这个 bin，无法单独编译。评审请以 tip 为准。

## 8. 阶段 8：设置界面（2026-09-25，本轮）

规格：`research/windows/07-settings-ui.md`（逐节引用，尤其是 §0 结论速览、§1.3 度量、§1.5 行/分组契约、
§2 页签清单、§4 持久化、§7.3「哪些键能立刻生效」、§9 gpui-kit 组件选型）。
组件与设计口径：`docs/gpui-kit/0.6.6/zh-CN/**`（`component/{dialog,settings,switch,select,number-input,button,icon,theme}.md`、
`shell/overlays.md`、`docs/{coding-guides,design-guides}.md`）。

### 8.1 新 crate：`gpui/crates/settings`（package `lithe-gpui-settings`）

依赖只向下：`gpui-kit` + `lithe-gpui-shared` + `serde`/`serde_json`（后两者 gpui-kit 不重导出）。
**不依赖** `workbench` / `app`。文件分工与启动顺序写在 `crates/settings/src/lib.rs` 的模块文档里：

| 文件 | 职责 |
| --- | --- |
| `schema.rs` | `Settings` + 逐键默认值 + 纯规范化（无 GPUI） |
| `paths.rs` | 设置文件路径（唯一出现 `#[cfg(target_os)]` 的地方）+ `LITHE_GPUI_SETTINGS_FILE` |
| `persistence.rs` | 容错读、原子写、300ms 防抖状态机（无 GPUI） |
| `theme.rs` | 主题目录装载/监听、「按名字应用主题」、UI 字号 → rem 基准 |
| `store.rs` | `SettingsStore`（Entity + Global 句柄）：改设置 → 立即生效 → 防抖落盘 |
| `row.rs` | `SettingsGroup` / `SettingsRow` / 控件宽度档 |
| `dialog.rs` | 820×620 模态对话框：头部 / 分类栏 / 内容页 / 底部 + 确认子对话框 |

`main.rs` 原来的 `apply_lithe_theme` 被拆成 `theme::watch_lithe_themes`（装载 + 监听 + 重载后复原）
与 `theme::apply_theme_by_name`（真正应用一个主题）：**启动与设置里切主题现在是同一条路**。

### 8.2 v1 只做两页（其余 10 个分类为什么不做）

Windows 真实对话框的分类表是 **12 项**（`settings-dialog.tsx:35-48`）。v1 只渲染前两项：

| 分类 | 页面里的项 | 能立刻生效吗 | 怎么验证 |
| --- | --- | --- | --- |
| **常规** | `displayLanguage`（语言下拉：英语 / 简体中文，顺序照抄 Windows 的 `DISPLAY_LANGUAGES`） | ✅ **选完自动重启生效**：写设置（立即落盘）→ 用**相同参数**重新拉起自己 → 退出当前进程（`restart.rs`）。不做运行中热切：`set_locale` 只在启动早期调一次，活动栏/状态栏文案是构造期 `tr()` 的，热切只会"一半新一半旧" | stdout `S1_SETTINGS restarting pid=… args=…` + 新进程 `S1_SETTINGS loaded … locale=en-US`；截图里界面变英文 |
| **外观** | `theme`（配色主题下拉，候选 = `ThemeRegistry` 已加载的主题） | ✅ 立即 | `.artifacts/p1/theme-from-settings.png` + stdout `S1_THEME applied=… dark=false` |
| | `uiFontSize`（数字输入，10–24 / 0.5 步长） | ✅ 立即（写 `Theme.font_size`，`Root::render` 每帧 `window.set_rem_size`） | stdout `S1_THEME font_size=…`；另有 rem 基准不变量兜底 |
| | `showStatusBar`（开关） | ✅ 立即（`ShellWorkspace::render` 条件渲染 + 订阅 `SettingsStore` 重绘） | 关掉后状态栏整条消失 |
| | 外观模式（`syncSystemTheme` + `autoThemeLight/Dark`） | ✅ 立即 | `Window::appearance()` 存在（`gpui-pre-0.3.6/src/window.rs:2765`），并注册了 `observe_window_appearance`，运行中系统明暗切换也会跟随 |

**其余 10 个分类不做，理由只有一个：对应子系统在 gpui 侧不存在，做了就是空壳**（逐项依据 `07-settings-ui.md` §7.1–7.2）：

| 分类 | 前置条件（阶段 B/C） |
| --- | --- |
| `editor`（4 项） | `EditorPane` 接上设置（字号 / 行高 / 缩进 / 换行 / 行号）——**阶段 B**，只差接线 |
| `file-tree`（13 项，死代码页签） | explorer 接上排序 / 缩进 / 图标 / 过滤——**阶段 B**，子系统已有 |
| `terminal`（1 项） | shell **发现**（现在是硬编码常量）+ 终端重建路径——**阶段 B** |
| `project` / `run` | JDK/Maven 发现（Maven 工具窗现在**在右栏**，见 §11；但项目探测与 `maven.*` 数据源还没有）——**阶段 B** |
| `keyboard` / `lsp` / `ai` / `ai-commit` / `logs` / `updates` | 子系统**完全不存在**（键位系统 / 语言服务 / AI / 日志 / 更新器）——**阶段 C** |
| `git`（11 项） | 11 项里 9 项作用于**不存在的 Git 变更面板**；先做变更面板再回来做设置——**阶段 C** |
| `advanced`（`coreFeatures.*`） | gpui 侧没有任何读取方——**阶段 C** |

`appearance` 页里没做的项：`iconTheme`（gpui 只有一套图标资产，没有图标主题系统）、
`windowTransparency` / `nativeMenuBar` / `compactMenuBar`（无对应原生能力）、
`uiFontFamily`（要先解决"枚举系统字体 + 写 `Theme.font_family` + `sync_base`"）、
`reduceMotion`（`App::reduce_motion` 是应用级开关，改它要评估全部动效）、
布局尺寸类（`activityRailWidth` / `sidebarWidth` 等，属于"可调整面板"，要先把拖拽接上才不是空壳）。
`askWhereToOpenProjects` / `hiddenFilePatterns` 等**没有读方**，按 §7.3-C 也不进 v1。

### 8.3 组件选型：**不用** `component::setting::Settings`，自绘行

`07-settings-ui.md` §9.3 建议优先用 `setting::{Settings, SettingPage, …}`，读完 0.6.6 实现后**判定换不出目标布局**，三条硬证据（写进了 `dialog.rs` 的模块文档）：

1. **它自带搜索框且无法关闭**（`setting/settings.rs:148-153` 永远塞一个 `Input`），
   而 Windows 真源**没有搜索框**（§1.4）；
2. **它的左栏是 `h_resizable` 可拖拽分栏、默认 250（160–360）**（同文件 `:416-422`），
   Windows 是**固定 190、无把手**；
3. **页面外壳不同**：它页头 `p_4 + border_b`（`setting/page.rs:180-186`），
   Windows 是 `p-6` + 无下边框 + `text-xl font-semibold`。

所以复用它的**行内控件**（`Switch` / `NumberInput` / `Button`，以及它自己的下拉实现方式
`Button + dropdown_menu_with_anchor + PopupMenuItem`，见 `setting/fields/dropdown.rs:60-87`）与
`Dialog` 容器，自绘左栏与行。代价：没有"白送"的搜索过滤（真源也没有）。

### 8.4 持久化语义（照 §4）

| 语义 | 实现 | 真源 |
| --- | --- | --- |
| 路径 | Windows `%APPDATA%\Lithe\settings.json`；macOS `~/Library/Application Support/Lithe/`；Linux `$XDG_CONFIG_HOME/lithe/` | 任务书；gpui 侧**没有**数据目录 helper（五个 crate `grep` 零命中），所以平台分支集中在 `paths.rs` |
| 覆盖 | `LITHE_GPUI_SETTINGS_FILE`（完整文件路径）优先于一切推导 | 供测试与两轮机器验证 |
| 读取 | 文件不存在 → 全默认值、**不创建文件**；不是 JSON / 不是对象 → 全默认值 + 一条诊断；**某个键坏了只回落该键**；未知键不参与设置、但**写回时原样保留** | `lib/settings-persistence.ts:18-43,73-79` + 见 §14.2 |
| 版本 | 顶层 `version`（当前 `DOCUMENT_VERSION = 1`）；写回时写当前版本；文件声明的版本**更高**时转**只读**（内存里照常生效，但不覆盖用户的文件） | 本仓库新增（手改安全） |
| 写入 | **原子写**（`settings.json.tmp` + rename）；普通改动 **300ms 防抖**；「恢复默认设置」**立即写**；关对话框补一次 flush；**进程退出前**用 `on_app_quit` 再补一次 | `lib/settings-persistence.ts:96-112`、`stores/settings.store.ts:88-97` |
| 外部改动 | 监听**父目录**（非递归）并按文件名过滤，手改文件不必重启；内容与内存一致时不动（挡住自己写入触发的自激）；**外部改动胜出**，文件被删则保留内存设置 | 本仓库新增（见 `src/watch.rs`） |
| 去重 | 值没变就不应用、不落盘（也挡住"打开对话框时数字框发一次值不变的 Change 就写盘"的噪音） | `lib/settings-persistence.ts:34` 的 `!isEqual` |
| 规范化 | `uiFontSize` 10–24 / 0.5 步长；`displayLanguage` 白名单；空主题名回落默认；**主题名必须在注册表里**（在主题装载回调里校验，不写回文件） | `lib/ui-font-size.ts:10-19`、`settings-normalization.ts:480,499-519` |
| 键表 | 由 schema 派生（`known_keys()` 取 `Settings::default()` 的序列化结果），**没有第二份手写清单** | 见 §14.2 |

⚠️ **监听目录而不是监听文件**：我们自己的写入是"临时文件 + rename"，文件级监听会跟着被 rename
换掉的那一份一起失效（自己写完第一次就再也听不到）。所以监听父目录、按文件名过滤；
代理目录不存在时会先建目录（读设置文件本身仍然不创建文件）。

§4.3 里的数组类条目（隐藏路径 trim / 去空行、四个"项目顺序"数组）**v1 不适用**（没有数组键），
规则先落在 `persistence::normalize_pattern_lines` 并带测试，等 explorer 接过滤时直接用。

### 8.5 接线

- **启动**：读设置文件 → 语言（`--locale` 优先）→ `set_locale` → `gpui_kit::init` → `init_store` →
  `watch_lithe_themes`（主题在这里应用）→ `install_actions`（`Ctrl+,`）→ 开窗 → `attach_window`
  （系统外观监听）。
- `--theme` / `--locale` 从"设置界面做好之前的临时开关"改成**显式覆盖（验证/诊断用、不写回设置文件）**
  —— `.artifacts/` 的 4 配置视觉验证依赖它们。
- **入口**：① 左侧活动栏第 9 项「设置」（`SETTINGS_ACTIVITY_IX = 8`）→ 打开对话框（**不改**活动栏选中态，
  因为真机是模态对话框而不是侧栏视图）；② `Ctrl+,` 走 `actions!` + **`App::on_action` 全局处理器**
  （挂在 `ShellWorkspace` 根元素上会有"没点过任何地方时不响应"的死角：无焦点时按键只派发到窗口根节点，
  `gpui-pre-0.3.6/src/window.rs:5815`）。
- **诊断开关**：`--open-settings`（首帧之后用 `window.on_next_frame` 打开，非 render 阶段）。

### 8.6 与 Windows / 设计指南的有意偏离（都在代码注释里写了理由）

| 偏离 | 理由 |
| --- | --- |
| 确认对话框文案不照抄 Windows 的「⚠️确认恢复所有配置吗？」 | `docs/design-guides.md:434` 明确点名这种"您确定要……吗"是反例；改成"标题=决策（恢复默认设置？）+ 正文=后果（所有设置都会回到默认值。）+ 按钮=结果词（恢复默认设置）"。原句与出处写在 `dialog.rs` 的注释里 |
| 「恢复默认设置」按钮加省略号（`…`） | 会先打开确认对话框（`design-guides.md:444`） |
| 左栏选中态用 `primary/65` + `primary_foreground` + **字重**（不是 Windows 的 `text-white`） | 不许写裸色值；`theme.primary_foreground` 才是组件体系用的前景 token。另加字重作为**颜色之外**的选中信号（无障碍检查表） |
| 「配色主题」在跟随系统时显示**当前生效**的主题 | Windows 在该模式下显示的是 `settings.theme`（并非生效值，`macos-settings-panels.tsx:141` vs `:122`）；显示生效值更不容易骗人 |
| 语言下拉的两个选项名走新键（`settings.gpui.language*`） | Windows 把 `English` / `简体中文` **硬编码在 TSX 里**、不在 catalog；界面上不许出现字面量 |
| 遮罩比 Windows 浅：gpui-kit `Dialog` 默认 **20% 黑**，Windows 覆写成 `bg-black/55` | `Dialog` 只暴露 `overlay(bool)`、没有遮罩色入口。实测压暗系数 k≈0.798（= 20%，与组件默认一致）。为不改组件、也不自绘遮罩（那会丢掉焦点陷阱与 Esc 语义）而保留默认 |
| 「显示语言」选完**自动重启**，而不是"只提示、让用户自己重启" | 热切做不到干净（见 §8.2 常规行）；只提示又会让界面长期停在半生效状态。实现与代价（当前进程未保存的编辑器内容与终端会话会结束）写在 `crates/settings/src/restart.rs` 的模块文档里；编辑器接上保存后要重新评估是否加确认 |

### 8.7 验证（本轮实际跑过）

1. `cargo build --bin Lithe`：0 error；warning 只有既有的两条 `linker_messages`。
2. `cargo test -p lithe-gpui-settings` = 30 通过；`cargo test -p lithe-gpui-shared` = 4 通过。
3. **设置文件 → 主题**：临时文件写 `{"theme":"Lithe Light"}` → stdout
   `S1_SETTINGS loaded … theme=Lithe Light` + `S1_THEME applied=Lithe Light dark=false`，
   截图 `.artifacts/p1/theme-from-settings.png` 工作区底色 `#FFFFFF`。
4. **对话框**：`--open-settings` → stdout `S1_SETTINGS dialog_opened`、stderr 空、进程正常退出；
   截图 `.artifacts/p1/settings-dialog.png` 量得对话框 **1023×773 物理像素**（缩放 1.247 →
   820×620 逻辑，宽高比 1.3234 vs 规格 1.3226），左栏右边界落在离左沿 **189** 逻辑像素处（规格 190）。

### 8.8 本轮未能确认 / 还欠的

1. `Ctrl+,` 与活动栏入口**没有被机器验证过**：都需要真实输入事件（本轮只验了 `--open-settings` 这条
   同为"事件/任务里打开浮层"的路径）。顺带记录一个环境现象：物理鼠标停在工作区某处时会有一次
   杂散点击落到窗口上（曾误触活动栏「设置」并打开对话框），排查方法见本轮记录——改变光标位置后现象消失，
   与代码无关。
2. `syncSystemTheme` 的**运行中跟随**只做了实现（`observe_window_appearance`），没有在真机上切换系统主题验证。
3. 行的"点整行即激活主控件"只对**开关行**做了；下拉行与数字行没做（`Button::dropdown_menu` 没有
   以编程方式展开的入口），这是有意取舍，不是遗漏。
4. `cargo fmt` 跑不了（`rustfmt` 组件在本机镜像 404，见 §7.2）：本轮代码按现有格式手写，
   未做机器格式化。
5. 主题热重载后 `uiFontSize` 的 rem 基准靠"不变量观察者"兜底（`ThemeRegistry` 的全局观察者会在
   重载时把 `Theme.font_size` 写回主题文件的 16），这条路径没有单独截图验证。

---

## 9. 阶段 9：编辑器完善（A+B+C+E，维护者定稿）

**先读这三条前提事实，避免重复判断**：
- `crates/editor` 用的**已经是**真 `gpui_kit::component::input::Editor`（可编辑 + 语法高亮 + 行号/折叠）；
  它模块文档里"本轮明确没做"的三件事（脏标记恒为干净、`← →` 恒禁用、状态栏项写死）就是本阶段要补的。
- Editor 组件**自带查找替换**（`open_search` / `replace_all_search_matches`，
  `gpui/docs/gpui-kit/0.6.6/zh-CN/component/editor.md:128-204`）与 `InputEvent::Change/Focus/Blur`
  （同文件 `:215-226`）—— 脏标记、保存、查找替换全是**接线**，不是自研。
- Windows 的 `autoSave` 默认 **true**，语义是"编辑时**防抖**自动保存"
  （`windows/tauri/src/features/editor/stores/editor-app.store.ts:503-570` 的 `autoSaveTasks` +
  stale-content 守卫）；脏标记仍然存在，手动保存是独立路径。

| 项 | 内容 |
| --- | --- |
| A | 脏标记（订阅 `InputEvent::Change`）+ `Ctrl+S` 保存 + 照 Windows 的**防抖自动保存**（含 stale-content 守卫） |
| B | 状态栏 `1:1` / `UTF-8` 从写死改为真实值（光标行列随光标更新） |
| C | `Ctrl+F` 查找替换（组件自带；开启 + 把 `Search` action 接到当前编辑器） |
| E | 关闭未保存文件时**确认**（维护者后来明确加回） |

**不做**：D（`← →` 文件跳转历史）挪到阶段 10（跳转必须有回退）；大文件降级与"外部修改冲突横幅"后置。

## 10. 阶段 10：Java 代码跳转（两批，维护者定稿）

**Core 里已经有整套能力，不要自己造索引**（`develop-lithe` 的第一条硬规则就是复用成熟上游能力）：
- 轻量（**不启 JDTLS**）：`lsp.builtinNavigation`（当前文件内定义/引用）、`java.sourceDefinition`。
- 完整：`lsp.startServer`（**Rust 拥有进程/会话**）、`lsp.syncDocument`、`lsp.request`
  （`textDocument/definition` 等语义请求 + operationId）、`lsp.pollEvents`、`java.workspacePolicy`、
  `java.navigationMarkers`、`java.resolveNavigation`，以及 JDT 工作区索引缓存三件套
  `lsp.jdtWorkspaceKey` / `java.jdtWorkspaceFingerprint` / `java.jdtCacheRetention`
  （命令面见 `shared/contracts/rust-core-api.md:131-159`，LSP 章节见该文件 §LSP）。
- 用户描述的"打开项目生成索引、后续重开不用管"= 就是 `cacheDirectory/jdtls/<workspaceKey>` +
  workspace 指纹 + 过期回收（`rust-core-api.md:1241-1301`），**不是**新造一套索引。

| 批次 | 内容 |
| --- | --- |
| 第一批 | `F12` / `Ctrl+单击` → Core 轻量导航 → **编辑器寻址**（跳到目标文件/行并把光标放过去）+ `← →` 回退（即阶段 9 未做的 D）。不启 JDTLS，**立刻可验证** |
| 第二批 | JDTLS：跨文件 / 跨模块 / JDK 与依赖库源码（`jdt://`）跳转；打开项目时生成索引缓存、重开复用。host 侧要移植 `windows/tauri/src-tauri/src/lsp.rs` 的 jdtls 解析（~1000 行：可执行文件 / 启动资源 / 内嵌 JDK / workspace 指纹 / 缓存目录） |

**触发器只有 `F12` 与 `Ctrl+单击`**（维护者选定：不做 `Ctrl+B`、不做右键菜单）。

### 10.1 第一批已完成（2026-09-25，本轮）

**用的 Core 命令**：`lsp.builtinNavigation`（`shared/contracts/rust-core-api.md:135,1143-1151`；
请求 `{filePath, text, position{line, utf16Column}, method}`，`method = "textDocument/definition"`
—— `rust/lithe-core/src/lsp/lightweight/symbols.rs:23-28`；响应
`{locations:[{filePath, range{start{line,utf16Column},…}, isReadOnly, displayPath}]}`，同文件 `:68-80`）。
**没有**选 `java.sourceDefinition`：它要求宿主先答出"光标下这个标识符叫什么"
（`rust-core-api.md:1723-1725`），那等于在 gpui 侧写一遍"取光标处标识符"，而
`lsp.builtinNavigation` 本身就是按光标位置取标识符的。**gpui 侧一行 Java 语法都没写。**

**落点**：新 crate 内模块 `gpui/crates/editor/src/navigation.rs`（Core 调用 + 列口径换算 +
`JumpHistory`）；接线在 `editor_view.rs`；action/键位在 `lib.rs`。

**三组口径必须显式换算**（`navigation.rs` 的 `core_position` / `editor_position`）：编辑器的
光标是**字节偏移**（`state.cursor()`），`Position.character` 是**字符数**
（`gpui-base-0.6.6/src/input/base/rope_ext.rs:190-198`），Core 的 `utf16Column` 是 **UTF-16 码元**。
只有 ASCII 上三者才恰好相等（单测用 `🎉` 把两个口径分开守住了）。

**`Ctrl+单击` 为什么不需要"点→文本位置"换算**（本批最想记下的一条）：
`InputBaseState::index_for_mouse_position` 是 `pub(crate)`、`resolve_mouse_position` 是
`pub(super)`（`gpui-base-0.6.6/src/input/base/state.rs:2868-2887`），组件**没有**公开的点换算 API。
但组件自己的 `on_mouse_down` 已经把光标移到了点击位置（`state.rs:2311-2315` 的
`move_to_with_affinity`），而我们在编辑器外面包一层 div 接 `on_mouse_down`（冒泡阶段）——
冒泡是 `.rev()` 遍历、**祖先排在目标之后**（`gpui-pre-0.3.6/src/window.rs:5778-5787`），
所以处理器里读 `cursor()` 拿到的就是"点到的那个字符"。组件的 `on_mouse_down` 全程不
`stop_propagation()`（`state.rs:2223-2316`），事件能冒到包装层。另配一道
`input_bounds().contains(event.position)` 判据，避免"点到编辑器之外也跳"。**没有 hack 组件内部**。
（上游还有一条 `EditorState::lsp_mut().definition_provider` 的 `DefinitionProvider` 钩子，
`input/editor/lsp/definitions.rs:13-24`；那是"Ctrl 悬停缓存 + Ctrl 点击"的双步路径，
本批没用它——两条路径同时开会让组件的 `handle_click_hover_definition` 提前 `return`
（`state.rs:2259-2261`），光标就不会被点击移动，我们的读取点会读到旧位置。第二批做
`jdt://` 跨文件跳转时再改用它 + `show_document` 钩子。）

**`← →`**：语义逐条照 `windows/tauri/src/features/editor/stores/jump-list.store.ts`
（新跳转**截断前进分支** `:49-53`、同位置不重复入栈 `:77-92`、`go_back` 在"现在"时先压入
当前位置再退一项 `:136-142`、两个 `can_*` 的判据 `:189-200`）。没有历史时两个按钮仍是
`disabled(true)`（`Enabled` 由 `can_go_back` / `can_go_forward` 算）。

**诊断行**（验证就靠它们，`cargo build` 后跑 `.artifacts/p2/verify-nav.ps1`）：
`S1_NAV_JUMP from=l:c to=l:c kind=definition`、`S1_NAV_BACK` / `S1_NAV_FORWARD from/to`、
`S1_NAV_FAILED reason=<no-buffer|no-target|stale|buffer-closed|content-changed|Core 错误码>`；
跳转失败时**不移动光标**，并按真机给一条提示（`navigation.noTargetFound` + `navigation.definition`，
`windows/tauri/src/i18n/locale.ts:8435,8439`，两条都是真源既有键、已进 WIRED）。

### 10.2 第一批的边界（有意，不是缺陷）

1. **只在当前文件内跳转**：Core 的轻量导航返回的 `filePath` 恒等于请求里的文件
   （`symbols.rs:194`）。跨文件是第二批的事。
2. **"定义"的精度依赖 Core 的轻量启发式**：`looks_like_declaration` 只认紧贴标识符的
   `class`/`func`/`let` 这类关键字（`symbols.rs:336-368`），**不认 Java 的 `类型 名字(`**
   （`static int add(` 的前一个 token 是 `int`）。所以 Java 里通常走的是"找不到声明形态 →
   退回全部出现位置、取第一个"（`symbols.rs:183-185`）—— 只要**声明写在首次使用之前**
   就落在声明上（本轮样本 `Hello.java:3` 声明 / `:12` 使用已验证）。类型感知的精确跳转
   归第二批。
3. **`Ctrl+单击` 的位置判据**用 `input_bounds()`（编辑器整块 bounds，含内边距），不是
   精确到字形；点在编辑器内边距上也会触发（真机的编辑器内边距同样算在内）。
4. **`←` 回到一个已经被关掉的 buffer** 时会重新读盘打开该文件（真机同样如此）；
   未保存的修改在关标签时已经过确认对话框，所以这里不会静默丢数据。

环境结论（本机实测）：TLS 可用（`download.eclipse.org` 200 OK）；有 JDK 21 / 25；但 **jdtls 载荷尚未
下载**（`third_party/jdtls/` 只有 `manifest.json`，`.artifacts/jdtls` 不存在）——第二批开工前先跑
`scripts/prepare-jdtls.ps1`（jdtls 1.61.0 + lombok + java-debug + java-test，带 sha256 校验）。
（2026-09-25 第二批开工时已就绪：`.artifacts/jdtls` 61.7 MB / 114 个 plugin。）

### 10.3 第二批已完成（2026-09-25，本轮）

**落点**：新 crate **`gpui/crates/java`（`lithe-gpui-java`）**，四个模块：
`jdtls.rs`（载荷 + JDK 发现，移植 `windows/tauri/src-tauri/src/lsp.rs`）、
`workspace.rs`（指纹 / 缓存键 / 回收，移植同目录 `lsp/jdt_workspace.rs`）、
`session.rs`（Core LSP 会话的同步信封）、`service.rs`（门面 `JavaLanguageService` + `S1_JAVA_*`）。
编辑器只做接线：`navigation.rs` 的 `resolve_target`、`editor_view.rs` 的 `apply_definition` /
`open_virtual` / `prepare_java`。**gpui 侧没有一行 LSP 协议、没有一行 Java 语法**。

#### `lsp.startServer` 的实际请求字段（全部实测通过，`session.rs::Session::start`）

```json
{ "providerId": "java",
  "executablePath": "<jdtls>/bin/jdtls.bat", "arguments": [],
  "environment": { "JAVA_HOME": "<jdk>" },
  "rootUri": "file:///D:/.../jdt-ws/", "workingDirectory": "D:\\...\\jdt-ws",
  "runtimeExecutablePath": "<jdk>/bin/java.exe",
  "jdtlsLaunchResources": { "launcherJarPath": "...", "configurationDirectory": "...\\config_win",
                            "lombokAgentPath": "...", "javaDebugBundlePath": "...",
                            "javaExtensionBundlePaths": ["..."] },
  "cacheDirectory": "%LOCALAPPDATA%\\Lithe\\cache\\language-servers",
  "workspaceFingerprint": "<java.jdtWorkspaceFingerprint 的不透明结果>",
  "javaRuntimes": [{ "homePath": "<jdk>", "version": "21.0.8" }],
  "initializeTimeoutMilliseconds": 90000,
  "serviceReadyIdleTimeoutMilliseconds": 45000,
  "serviceReadyAbsoluteTimeoutMilliseconds": 600000,
  "requestTimeoutMilliseconds": 60000, "javaBuildTimeoutMilliseconds": 600000,
  "shutdownTimeoutMilliseconds": 2000 }
```
三条实测要点：① 有了 `jdtlsLaunchResources` + `runtimeExecutablePath`，`arguments` 留空
（Core 自己拼 JVM 参数，契约 `:1208-1217`）；② `lsp.startServer` 的响应**同步且很快**
（返回 `state: initializing`，握手在 Core 后台线程里跑，`engine.rs:1198-1202`），
就绪要另外等 `lsp.waitEvents` 的 `stateChanged: ready`；③ 送进去的**路径必须先过
`normalize_path`**（去 `\\?\` verbatim + `\`→`/`），否则 JVM 找不到启动 JAR、进程秒退。

#### 五条链路

| 链路 | Core 命令 | 实测结果 |
| --- | --- | --- |
| 起服务 | `lsp.startServer` + `lsp.waitEvents` | `S1_JAVA_READY elapsedMs=4994`（本项目 2 个 `.java`，从应用启动到就绪 6s） |
| 跨文件 | `lsp.syncDocument` + `lsp.request{operation:"definition"}` | `S1_NAV_JUMP from=6:36 to=4:19 kind=definition via=jdtls file=...Greeter.java` |
| 回退 | 第一批的 `JumpHistory` | `S1_NAV_BACK from=4:19 to=6:36` / `S1_NAV_FORWARD from=6:36 to=4:19` |
| 索引缓存 | `java.jdtWorkspaceFingerprint` + `lsp.jdtWorkspaceKey` + `java.jdtCacheRetention` | 键 `8ec4a126…764b`；两遍启动 `reuse=false` → `reuse=true`；Core 自己的进度里 `cacheDisposition` 从 `new` → `reused` |
| `jdt://` | `lsp.request{operation:"virtualDocument"}` | `System.java` 只读标签，118 124 字节反编译源码，光标 `112:20` |

#### 冲突点：组件 `definition_provider` vs "冒泡读光标"——本轮**继续用后者**

上游那条钩子（`gpui-component-0.6.6/src/editor/lsp/definitions.rs:13-24`）的语义是
**"Ctrl 悬停时把位置缓存起来，Ctrl 点击时消费缓存"**：一旦启用，
`handle_click_hover_definition` 会提前 `return`（`.../input/state.rs:2259-2261`），
**点击不再移动光标**，而第一批的 `Ctrl+单击` 正是靠"组件先移光标、我们冒泡时读 `cursor()`"。
第二批**不需要**换路，理由是三条叠加：

1. 需要的东西已经够了：`state.cursor()` 在冒泡阶段就是"点到的那一个字符"，与 `F12` 完全同路，
   所以语义结果与触发方式无关（悬停只是多余的中间态）；
2. 换路会**新增两条真实交互**：Ctrl+悬停必须停够时间（组件的悬停缓存有延时），
   而且悬停失败时点击会退化成"只移光标不跳"——维护者定稿的触发器只有 `F12` 与 `Ctrl+单击`
   两条，不该再多一条"必须先把鼠标停在符号上"；
3. `definition_provider` 是**同步返回缓存位置**的接口（它自己不请求语言服务），
   而语义结果要等 JDT 的异步应答；用它就得把"缓存并发的异步结果"再加一层状态，
   与第一批已有的 `NavRequest`/`generation`/stale 守卫重复。

代价（有意接受）：`Ctrl+单击` 依赖"组件先移光标"这个行为，所以那条
`input_bounds().contains(..)` 判据仍然必要（点在编辑器内边距上光标不动、不该跳）。

#### 诊断行（第二阶段新增，都可 grep）

```
S1_JAVA_JDTLS executable=… version=… java=… javaVersion=… launcher=… config=…
S1_JAVA_CACHE directory=… key=… reuse=<true|false> stateDirectory=…
S1_JAVA_CACHE_RECLAIM retentionDays=30 removed=n        / S1_JAVA_CACHE_RECLAIM_FAILED error=…
S1_JAVA_START rootUri=… workingDirectory=…
S1_JAVA_SESSION started sessionId=… processId=…  /  failed stateChanged=<整条事件>  /  stopped sessionId=…
S1_JAVA_LOG level=… message=…            （JDT 的导入 / 进度，连续重复只打一次）
S1_JAVA_READY elapsedMs=… logEvents=… serverInfo=…
S1_JAVA_SYNC uri=… version=… changed=… bytes=…
S1_JAVA_DEFINITION target=file path=… line=… col=…      / targets=0
S1_JAVA_VIRTUAL uri=… displayPath=… bytes=… line=… col=…
S1_JAVA_SKIPPED reason=not-a-java-workspace   /   S1_JAVA_PREPARED   /   S1_JAVA_PREPARE_FAILED reason=…
S1_JAVA_UNAVAILABLE reason=… fallback=builtin
```
`S1_NAV_JUMP` 沿用第一批的格式，只多一个 `via=<builtin|jdtls|jdtls-virtual>` 与 `file=`。

### 10.4 第二批的边界（有意，不是缺陷）

1. **JDTLS 只是增强，不是依赖**：起不来（没装 JDK 21、载荷不全）时 `definition` 返回 `Err`，
   `resolve_target` 记一条 `S1_JAVA_UNAVAILABLE` 后**退回第一批的轻量导航**。
   但 JDTLS **答了**（哪怕是空 `locations`）就是权威答案，**不回退** ——
   回退会把"这里没有定义"变成"启发式猜一个位置"。
2. **失败不自动重试**：启动失败几乎都是环境问题，每次 `F12` 都重试一次 30s+ 的启动会把
   每次跳转都变成一次长时间阻塞。失败只记一次，重启应用等于重试。
3. **运行 JDK 靠发现，不靠写死**：`LITHE_JDTLS_JAVA` → 捆绑位置（`LanguageServers/jdk`、
   上一层 `.artifacts/jdk`）→ JDTLS 自带 `jre` → `JAVA_HOME` → PATH → 常见安装根
   （`~/.jdks`、`Program Files\{Java,Eclipse Adoptium,Microsoft,Amazon Corretto,Zulu,BellSoft}`、
   `%ProgramData%\java`、scoop）。每个候选都要过 `java -version` 的 **21+ 闸门**。
   ⚠️ 本机的 JDK 21 装在 `D:\ProgramData\java\openjdk-21`（`%ProgramData%\java` 这一档），
   而 **PATH 上的 java 是 1.8** —— 所以验证链路显式给 `LITHE_JDTLS_JAVA`。
   真正的产品化应当把"Java 运行时"做成设置项（`java.jdt.ls.java.home` 同义），本轮没做。
4. **`jdt://` 的 buffer 关掉之后不能用 `←` 回去**：虚拟源码正文只来自 Core 的虚拟文档请求，
   没有"重新读盘"这条路。历史项仍在，重开需要"异步重取正文"这一层（本轮未做），
   现在给一条 `S1_NAV_FAILED reason=buffer-closed` 而不是读盘失败的说明 buffer。
   `App.java ↔ Greeter.java` 这类真实文件不受影响。
5. **不做 `java.project.updateSettings` / Maven 剖面 / 构建**：`mavenContext` 没传，
   所以 Maven 工程的源根与剖面不由 Lithe 喂给 JDT（JDT 自己的 m2e 导入仍在跑）。
   代价是 Maven 项目里 `src/main/java` 之外的生成源根可能识别不全；本轮验收样本是
   无构建文件的 invisible project，不受影响。
6. **一个工作区一个会话，退出时才关**：不随窗口/项目切换重启 JDTLS。
   窗口关闭时 `EditorPane::drop` **同步**调 `lsp.stopServer` + （等终态后）`lsp.destroyServer`
   —— 同步是必要的，进程退出不会替我们回收 JDT 的子进程，丢给分离线程会留下孤儿 JVM。
7. **`← →` 的 `→` 分支**仍沿用第一批的语义（新跳转截断前进分支），跨文件与虚拟源码同样适用。

**验证现场**：`.artifacts/p2/verify-jdt.ps1`（两遍：生成 / 复用）、
`.artifacts/p2/NOTES-BATCH2.md`（日志原文 + 截图 + 踩坑）、
`jdt-04-f12.png`（跨文件跳转）/ `jdt-07-virtual.png`（`jdt://` 只读标签）。

## 11. 阶段 6 第一半：右侧工具窗（2026-09-25，本轮）

阶段 6 的另一半（**命令面板浮层**：`Command` + `window.open_dialog`）**不在本轮**，留给下一轮。
本轮只做一件事：把右侧工具窗（截图 x≈1250..1750）从一行占位文本做成**真实区域**，
并把 Maven 从错位的底部工具窗搬回右栏。

### 11.1 状态模型：可见性 + 当前视图，都归 `ShellWorkspace`

| 字段 | 真源 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `right_visible: bool` | `features/window/stores/ui-state/panel-slice.ts:28` 的 `isRightSidebarVisible` | **`false`（隐藏）** | `features/layout/components/main-layout.tsx:101-105,328` 用它决定整块右 `ResizablePane` 的 `hidden` |
| `right_view: RightToolWindowView` | `stores/ui-state/view-slice.ts:12,26` 的 `activeRightSidebarView` | `Maven` | 真机初值是 `"outline"`（`:26`），那是**左栏**视图、右栏三项里没有它；面板默认隐藏，所以这个初值在界面上不可见，只决定"第一次点别的项之前面板里是什么" |

两个字段分开的理由与底部窗完全一致：真机 toggle 收起时**不改** `activeRightSidebarView`
（`features/layout/actions/right-tool-window-actions.ts:26-31`），收起再打开还是原来那个视图。

宽度 `RIGHT_TOOL_WINDOW_WIDTH = 400`（`features/settings/config/default-settings.ts:139`，
取值区间 140–600 见 `features/settings/lib/settings-normalization.ts:114-115,532-537`）。
本轮把上一轮的 `px(400.)` 改成 **`rems(400. / 16.)`**：400 不在 gpui 的固定档位上，
按《编码指南》"档位外的值用 helper 底层的 rem"表达（基准 16px 时逐像素相等）。

### 11.2 右活动栏三项的联动规则

判据与迁移全部照真机（`plugin-activity-rail.tsx:24-26`、`notifications-trigger.tsx:18-21`、
`right-tool-window-actions.ts:11-37`）：

| 当前状态 | 点的项 | 结果 | 右栏高亮 |
| --- | --- | --- | --- |
| 可见且就是这一项 | 同一项 | **收起**（视图保持不变） | 三项全灭 |
| 可见但是别的项 | 另一项 | 切到该视图，**保持可见** | 新项亮、旧项灭 |
| 不可见 | 任意项 | 切到该视图并显示 | 该项亮 |

`is_right_activity_active(index) = right_visible && right_view == 该项` —— **面板收起时三项都不亮**，
与左栏顶部组的"照常亮"互不影响（两边是两套独立状态）。
三条迁移写成纯函数 `right_tool_window::resolve_click`，单测照真源
`right-tool-window-actions.test.ts` 的用例逐条覆盖。

**收起有三条出路**：① 再点右栏同一项；② 面板头部的关闭按钮（真机两个工具窗都自带：
`maven-pane.tsx:606-616`、`notifications-tool-window.tsx:350-360`）；③ 点另一项只换视图、不收起。

### 11.3 右栏视图清单与各自完成度

| 视图 | 本轮做到哪 | 复用的 gpui-kit 能力 | 还欠什么 |
| --- | --- | --- | --- |
| **Maven** | 头部（`package` 字形 + `maven.title`「Maven」+ 关闭按钮）+ 空态 `maven.notDetected`「未检测到 Maven 项目」 | `component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle}`（`docs/gpui-kit/0.6.6/zh-CN/component/empty.md`）、`component::button::Button`（`ghost` + `size_6`）、`component::Icon` | 工具栏 / 模块树 / 生命周期 / 依赖树 / Profiles / 构建输出 —— 要 Maven 项目探测与 `maven.*` 数据源（阶段 B）。空态是**如实**的：真机也只探测到 Maven 项目才渲染右栏那一项（`plugin-activity-rail.tsx:21-23,51`） |
| **通知** | 头部（`bell` + `notifications.title`「通知」+ 关闭）+ 空态 `notifications.empty`「暂无通知。」 | 同上 | 搜索 / 过滤 / 分组 / 详情（`notifications-tool-window.tsx:340-475`）—— 没有通知数据源 |
| **扩展** | 头部（`puzzle` + `extensions.title`「扩展」+ 关闭）+ 空态 `extensions.noneFound`「未找到扩展。」 | 同上 | ⚠️ **本轮有意偏离真源**：真机那个按钮调 `openExtensionsBuffer`，扩展是**编辑器缓冲区**、不是右栏视图（`plugin-activity-rail.tsx:13,46`）。按本轮任务口径统一走右栏；若将来收敛成缓冲区，删 `RightToolWindowView::Extensions` + `right_activity_items()[0]` 即可回到真机的两项 |

`TabBar` **没有垂直方向**（`docs/gpui-kit/0.6.6/zh-CN/component/tabs.md`），所以视图切换不走页签组件 ——
真机的右工具窗本来也没有页签条（`main-layout.tsx:332-340` 是同层叠放 + `hidden`），
本轮沿用"无状态渲染函数按当前视图画内容"的口径（与 `activity_bar` / `status_bar` 一致）。

### 11.4 底部工具窗不再有 Maven

- `BottomPaneKind::Maven` **删掉**；`bottom_pane_for` 变成 3 运行 / 4 终端 / 5 诊断 / 6 提交记录；
- **左活动栏的 Maven 项也删掉**：真机左栏底部组第一项确实是 maven（`item-order.ts:12-19`），
  但它点下去切的也是**右栏**（`maven-tool-window-actions.ts:58` → `applyRightToolWindowIntent`）。
  本侧已经有右栏入口，再留一个就是"两处都能开 Maven"，所以只留右栏一处（有意偏离真机的图标集合）；
- 连带下标下移：底部组从 6 项变 5 项，`SETTINGS_ACTIVITY_IX` 由 8 改为 7；
- 左栏**原** Maven 位置（截图 y=788）现在是**空槽** —— 底部组贴着栏底排，少一项就整体下移
  28 逻辑（= 35 物理），点它没有任何反应（实测：无 `S1_RIGHT_PANEL`、`S1_TERMINAL_TAB` 不变、
  底部窗墨迹 180 → 180、右栏附近与隐藏对照帧逐像素无差异）。

### 11.5 诊断与 i18n

- 新增可 grep 的诊断行 `S1_RIGHT_PANEL view=<extensions|notifications|maven> visible=<true|false>`：
  构造期打一行（"默认隐藏"这件事本身有日志证据）、每次状态迁移打一行；
- 新接线 **5 条键，全部是真源既有键**（所以 `gpui/tools/extract-locale.mjs` 的 `GPUI_ONLY_KEYS`
  没动、两个 yml 无需重新生成）：`lithe.maven.title`、`lithe.maven.notDetected`、
  `lithe.notifications.empty`、`lithe.extensions.noneFound`、
  `lithe.commandPalette.close`（真源两个工具窗的关闭按钮都用它，文案是「关闭命令面板」
  = `locale.ts:7938` —— 真源自身的键复用，本侧照抄、不改写成新键）；
- 5 条都已补进 `crates/shared/src/i18n.rs` 的 `WIRED` 清单（`lithe.maven.title` 另进
  `SAME_IN_BOTH_LOCALES`：中英都是 `Maven`）；界面里没有中英文字面量，颜色全走 `cx.theme()`，
  间距/字号走 rem 档位 helper，圆角与 400 宽度按仓库既有约定处理并逐处写了理由。

### 11.6 验证（本轮实际跑过）

| 检查 | 结果 |
| --- | --- |
| `cargo build --bin Lithe` | 0 error；`lithe-gpui-workbench` 0 warning（顺手删掉了因占位文本移除而变成死字段的 `ShellWorkspace::root`） |
| `cargo test -p lithe-gpui-workbench` | 5 passed（`resolve_click` 四条迁移 + 右栏下标映射 + 视图 id） |
| `cargo test -p lithe-gpui-shared every_wired_key_resolves_in_both_locales` | passed（新增 5 条键在 zh-CN/en 下都命中） |
| `.artifacts/right-panel/verify-right-panel.ps1` | **16/16 判定全过**，见下表 |

端到端证据（**注入方式：PostMessage**；本机仍锁屏，`SetForegroundWindow` 被拒，
强度比真实注入弱一档，理由见 `.artifacts/p2/NOTES.md` §1；截图坐标 = 客户区坐标 + (9,0)）：

| 步骤 | 判定 | 截图 |
| --- | --- | --- |
| 1 默认隐藏 | `S1_RIGHT_PANEL view=maven visible=false`；右栏三项静止态（14.4/9.3/16%） | `rp-00-default.png` |
| 2 点 Maven | `S1_RIGHT_PANEL view=maven visible=true`；只有 Maven 项亮（70.4%） | `rp-01-maven.png` |
| 2b 宽度 | 头部下边框那条线长 **500 物理像素 = 400 逻辑像素**，与真机 `rightToolWindowWidth: 400` 逐像素相等 | `rp-01-maven.png` |
| 3 再点收起 | `S1_RIGHT_PANEL view=maven visible=false`；三项回到静止态；与默认帧在右栏附近**逐像素无差异** | `rp-02-hidden-again.png` |
| 4a 切通知 | `view=notifications visible=true`；高亮从 Maven 移到通知（70.4% / Maven 16%）；面板内容与 Maven 视图帧有 521px 宽的差异 | `rp-03-notifications.png` |
| 4b 切扩展 | `view=extensions visible=true`（**可见时切视图不收起**）；高亮移到扩展（70.4% / 通知 9.3%） | `rp-04-extensions.png` |
| 5 左栏原 Maven 位置 | 新增 `S1_RIGHT_PANEL` 行数 0；`S1_TERMINAL_TAB` 0 → 0；底部窗墨迹 180 → 180；右栏附近与隐藏对照帧无差异 | `rp-05a/05b` |
| 6 面板关闭按钮 | `view=maven visible=false`；三项全灭 | `rp-06/07` |

⚠️ 一处测量坑（已写进脚本注释）：右活动栏图标在**悬停**与**选中**下都是 `bg-accent` 底色，
点击后指针仍停在按钮上，截出来的高亮分不清 hover 与 selected。脚本于是给
`.artifacts/right-panel/inject.ps1` 加了 `-Mode move`（只投 `WM_MOUSEMOVE` 不按键），
**每次截图前把指针挪到编辑区**，上表的高亮占比都是"指针不在栏上"的干净读数。

## 12. 阶段 6 第二半：命令面板（2026-09-25，本轮）

上一半（右侧工具窗，§11）已完成。本半做的是**命令面板浮层**：
`windows/tauri/src/features/command-palette/` 那一层，本轮落到
**新模块 `gpui/crates/workbench/src/command_palette.rs`**（浮层零件）+ `workspace.rs` 里的
动作表与执行器。

### 12.1 快捷键：取自真源，不是惯例

| 项 | 值 | 出处 |
| --- | --- | --- |
| 命令 id | `workbench.commandPalette` | `features/keymaps/commands/command-registry.ts:611-615` |
| 默认绑定 | `cmd+shift+p` | `features/keymaps/defaults/default-keymaps.ts:492-496` |
| 生效的预设 | `keybindingPreset: "none"` → 用 `defaultKeymaps` | `features/settings/config/default-settings.ts:144` |
| **Windows 上按什么** | **`Ctrl+Shift+P`** | 绑定在解析时把 `cmd` 归一化成 `ctrl`：`utils/platform.ts:47-54` 的 `normalizeKey`，由 `features/keymaps/utils/parser.ts:76` 调用 |
| 其它预设的别名（未做） | JetBrains / Xcode 的 `cmd+shift+a`、Emacs 的 `alt+x` | `keybinding-presets.ts:94,119,145` |

实现是**全局 action** `lithe_workbench::OpenCommandPalette` + `KeyBinding::new("ctrl-shift-p", .., None)`，
登记在 `ShellWorkspace::new`（与 `Ctrl+,` / `Ctrl+S` 同一口径）：gpui 的按键派发在没有焦点元素时
只走"窗口根节点"这一条路径，挂在工作台根元素上会有"启动后没点过任何地方时按不出来"的死角。

### 12.2 容器：`Dialog` + `Command`（两者都不可替换）

- `component::command::{Command, CommandState}` **只是普通 `v_flex` + 内部 `Input`**
  （`gpui-component-0.6.6/src/command/state.rs:819-909`），没有遮罩、没有定位、不在窗口层叠序里；
  官方文档给的唯一浮层用法就是放进 `window.open_dialog`
  （`docs/gpui-kit/0.6.6/zh-CN/component/command.md:107-157`）。它负责**搜索框、过滤、虚拟列表、
  行高亮、`↑`/`↓`/`Enter`/`Esc`** —— 这些是成熟实现，本侧不重做。
- `Dialog` 负责**遮罩 / Escape / 焦点陷阱 / 关闭**，照 `lithe_gpui_settings::open_settings_dialog`
  的口径组合；**没有新增挂载点**（dialog 层已经由 `ShellWorkspace::render` 挂好）。
- ❌ **不用 `CommandState` 的 `Action` 机制**（`CommandItem::action(Box<dyn Action>)`）：实现它要么引
  `anyhow` + `serde`（`Action::build` 收 `serde_json::Value` 并返回 `anyhow::Result`，
  `gpui-pre-0.3.6/src/action.rs:135`），要么引 `#[derive(Action)]` 需要的 `serde` / `schemars`。
  本轮不新增依赖，所以用 `on_confirm` 的 `IndexPath` 直接查动作表
  （未分组条目 `section = 0`、`row` = 动作表行号，`command/state.rs:359-368`）。
  **代价**：没有真源每行尾部的快捷键提示列（提示位来自被绑定 `Action` 的解析结果，
  `command/state.rs:722-729`）。

度量（真源 `windows/tauri/src/ui/command.tsx`）：宽 `44rem` = **704**、最大高 `32rem` = **512**、
距窗口顶 `pt-16` = **16**。704 / 512 不在 gpui 的 rem 档位上，按《编码指南》写 `rems(P / 16.)`
（不是 `/ 4.`），`Dialog::width` / `margin_top` 只收 `Pixels`，所以走
`AbsoluteLength::to_pixels`（与 `settings/src/dialog.rs` 的 `rem_px` 同一换算）。

### 12.3 动作清单（**每一条都真的改状态**，没有"点了没反应"的项）

| id（`S1_COMMAND_RUN id=…`） | 标签（zh-CN） | 真的改了什么 |
| --- | --- | --- |
| `open-settings` | 首选项：打开设置 | 打开设置对话框（**常规**页） |
| `open-appearance-settings` | 首选项：打开设置 | 打开设置对话框且**停在外观页**（`open_settings_dialog_at`，真源 `openSettingsDialog(tab)` 的等价物） |
| `switch-theme-light` / `switch-theme-dark` | 首选项：切换到浅色/深色主题 | `SettingsStore::set_theme_explicit`（**同时关掉「跟随系统」**，照真源 `handleThemeChange`）+ 立即生效 + 防抖落盘 |
| `toggle-terminal` | 视图：显示/隐藏终端 | 底部工具窗可见性 + 首次显示时懒建终端会话 |
| `toggle-maven` | 视图：显示/隐藏 Maven | 右工具窗 `resolve_click(Maven, …)`（与右活动栏点击同一份状态、同一条诊断） |
| `toggle-status-bar` | 视图：显示/隐藏状态栏 | `SettingsStore::set_show_status_bar` |

- **主题那两条互斥**：当前深色只列"切浅色"，反之亦然（真机把两色都列出来是因为它走二级视图
  `color-theme`，本侧没有那一层；同时列两条会出现一条按下去什么都不变）。行序的**唯一真源**是
  `workspace::visible_commands`，`command_actions`（画）与 `run_command`（执行）都必须经过它 ——
  单测钉住了这一点。
- 标签 / 描述**能复用真源既有键就复用**（`commandPalette.actions.open-settings.label`、
  `toggle-terminal.enableLabel/disableLabel`、`color-theme.label`、
  `settings.appearance.showStatusBar(+Description)`、`commandPalette.categories.*`）；真源缺的
  9 条（切主题两对 + Maven 三条 + 状态栏两条）进 `extract-locale.mjs` 的 `GPUI_ONLY_KEYS`（每条写了理由）。
- **没放进来**的：`toggle-sidebar`（本侧左栏是常驻的、没有收起态 → 放了就是"点了没反应"）、
  真源的二级视图、窗口 / 缩放 / pane / git / github / markdown / database 等动作（对应能力在
  gpui 侧还不存在）。

### 12.4 两个实测出来的缺陷（本轮顺带修掉，都不是命令面板独有的）

1. **没有焦点节点时全局快捷键全死**。`dispatch_key_event` 拿
   `focus_node_id_in_rendered_frame(self.focus)`（`window.rs:5815-5816`）定派发路径，`focus` 为
   `None` 时路径为空、keymap 一条都匹配不到。真机上这个焦点来自"用户点过界面里的某个元素"，
   **启动后什么都不点时 `Ctrl+Shift+P` / `Ctrl+,` 一个都不响**。修法：`ShellWorkspace` 根元素挂
   `.track_focus(&self.focus)` 当兜底锚点，并在 `new()` 里主动 `window.focus(..)` 一次。
2. **命令面板打开后搜索框没有焦点**。`window.open_dialog` 把焦点给了 Dialog 自己
   （`root.rs:309-310`），于是 `↑`/`↓`/`Enter` 落不到 `CommandState` 的 key_context 上，而且
   `WM_CHAR` 会被丢掉（Windows 的字符输入走**已安装的输入处理器**，
   `gpui-pre-windows-0.3.6/src/events.rs:477-484`）。修法：打开时置位 `PENDING_FOCUS`，
   `CommandPalette::render` 的第一帧消费它并调 `CommandState::focus`。

### 12.5 诊断与 i18n

- `S1_COMMAND_PALETTE opened=true actions=<n>` / `focus=search`（可 grep）；
- `S1_COMMAND_RUN id=<id> state=<open|applied|visible|hidden|unavailable>`：**每条动作一行**，
  `unavailable` 用在"没有设置状态的宿主"上（不装作做成了）；`toggle-maven` 另有一行
  `S1_RIGHT_PANEL`（与右活动栏点击同一份诊断）；
- 新接线 **22 条键**（13 条真源既有 + 9 条 `GPUI_ONLY_KEYS`），全部进
  `crates/shared/src/i18n.rs` 的 `WIRED` 清单（`lithe.commandPalette.categories.Settings` 另进
  `SAME_IN_BOTH_LOCALES`：中英都是「设置」）；界面上没有中英文字面量，颜色全走 `cx.theme()`。

### 12.6 验证（本轮实际跑过）

| 检查 | 结果 |
| --- | --- |
| `cargo build --bin Lithe` | 0 error（只有既有的 `linker_messages` warning） |
| `cargo test -p lithe-gpui-workbench` | **9 passed**（原 5 + 新增 4：主题互斥 / 其余动作恒在 / 行序是 `COMMAND_ORDER` 的子序列 / 诊断 id 唯一） |
| `cargo test -p lithe-gpui-settings category` | passed（`Category::DEFAULT` + 分类 id 是诊断取值） |
| `cargo test -p lithe-gpui-shared every_wired_key_resolves_in_both_locales` | passed（新增 22 条键在 zh-CN/en 下都命中） |
| `node gpui/tools/extract-locale.mjs --check` | 两个 yml 与真源一致（4334 条 key） |
| `.artifacts/command-palette/verify-command-palette.ps1` | **21/21 判定全过**（见下表） |

端到端证据（**注入方式：PostMessage**；⚠️ 本机仍锁屏，**键盘注入这次不生效**，详见 §12.7）：

| 步骤 | 判定 | 截图 |
| --- | --- | --- |
| 1 基线 | 不带 `--open-palette` 的实例里 `S1_COMMAND_PALETTE` 行数 = 0 | `cp-00-baseline.png` |
| 2 打开 | `S1_COMMAND_PALETTE opened=true actions=6` + `focus=search` | `cp-01-opened.png` |
| 2b 宽 | 880 物理 = **704 逻辑**，与真源 `44rem` **逐像素相等** | 同上 |
| 2c 顶距 | 16.8 逻辑 vs 真源 `pt-16` = 16（本次截图落在动画帧上，见 §12.7） | 同上 |
| 2d 高 | 367.2 逻辑 ≤ 真源上限 512 | 同上 |
| 2e 选中行 | accent 底色只出现在第 1 行（y=100 附近） | 同上 |
| 3 执行 + 二级界面 | 点「首选项：打开设置（外观）」→ `S1_COMMAND_RUN id=open-appearance-settings` + `S1_SETTINGS dialog_opened category=appearance` | `cp-02-open-appearance.png` |
| 4 过滤 | 输入 `maven` → 面板 y 21..479 → 21..139（矮 340 物理），墨迹 2555 → 631；只剩「视图：显示 Maven」 | `cp-03`/`cp-04-filtered-maven.png` |
| 4c 空态 | 输入 `zzzz` → 墨迹 68，面板只剩搜索行 + 「未找到命令」 | `cp-05-empty.png` |
| 5 主题（自证） | `id=switch-theme-light state=applied` + `S1_THEME applied=Lithe Light dark=false` + 设置文件写入 `theme: "Lithe Light"` + 界面底色 `#1E1F22` → `#FFFFFF` | `cp-06-after-light-theme.png` |
| 5e 对称性 | 浅色下再打开，点同一位置执行的是 `switch-theme-dark`，设置文件回到 `Lithe Dark` | `cp-07-reopened-light.png` |
| 6 终端 | `id=toggle-terminal state=visible` + `S1_TERMINAL_TAB` 0 → 1（懒建会话，`profile=powershell`） | `cp-08-terminal-shown.png` |
| 6b Maven | `id=toggle-maven state=visible` + `S1_RIGHT_PANEL view=maven visible=true`（右栏 500 物理 = 400 逻辑） | `cp-09-maven-shown.png` |
| 7 关闭 | 点面板外的遮罩 → 与基线帧在面板区域**逐像素无差异**（差异 0） | `cp-11-closed.png` |

### 12.7 ⚠️ 本轮验证的强度边界（必须与结论一起读）

**键盘输入在锁屏下进不了应用。** 5 组对照实验（`.artifacts/command-palette/NOTES.md` §2）：

| 通路 | 结果 |
| --- | --- |
| `WM_LBUTTONDOWN/UP`（PostMessage） | **有效** —— 行点击 / 遮罩点击都生效 |
| `WM_CHAR`（PostMessage） | **有效** —— 文本进搜索框（第 4 步靠它） |
| `WM_KEYDOWN`（带 / 不带 scan code）、直接投 `WM_GPUI_KEYDOWN` | **无效** |
| `AttachThreadInput` + `SetKeyboardState` | 修饰键**置位成功**（`GetKeyState` 读到 0x80），按键仍到不了 gpui |
| `--palette-keys`（gpui 自己的 `dispatch_keystroke` + keymap，已显式补焦点锚点） | 派发执行了，**keymap 不命中**（连进程内已登记的 `ctrl-,` 都不响） |

所以本轮：
- **面板打开**走 `--open-palette`（诊断入口，调的就是按键最终落到的那个函数
  `command_palette::open_command_palette`），**不是**真的按了 `Ctrl+Shift+P`；
- **`Enter` 执行**改成**点行** —— gpui 里两者是同一个 `confirm`
  （`command/state.rs:757-759` 的 `on_click` → `confirm(matched_ix, ..)`，与 `:561-565` 的
  `on_action_confirm` 调同一个函数）；
- **`Esc` 关闭**改成**点遮罩** —— `Dialog` 的遮罩关闭与键盘 `Cancel` 走 `Root` 同一个关闭路径
  （`dialog/dialog.rs:584-585,601-607`）。

**这三条替换都不能证明"操作系统把 Ctrl+Shift+P 送进窗口"**。能被证明的是：绑定已登记
（`--palette-keys` 的调试输出能列出 `action: "lithe_workbench::OpenCommandPalette"`、
`key: "p"` + `control/shift`）、面板打开后的几何/过滤/键盘导航/执行/关闭全部按真源语义工作。
下一轮若要补这一条，需要在**没锁屏**的会话里重跑一次真实按键（`keybd_event` + `SetForegroundWindow`）。

另记一处小偏差（本轮**已修**）：行高原本**不均匀** —— `CommandItem::child` 自绘内容的高度决定行高，
主题那两条（没有图标、标签也短）会塌成别的行的一半，列表看起来一高一低（真源固定 `min-h-8`，
`ui/command.tsx:41`）。修法是给自绘内容加 `min_h_8()` + `justify_center()`；修后行距稳定
**50 物理 = 40 逻辑**（首行中心截图 y=90，逐行 +50）。

顶距 16.8 而不是 16 的另一个可能解释：截图可能落在 Dialog 的入场动画帧上
（`dialog/dialog.rs:558-563` 的 `ANIMATION_DURATION` 缓动），偏差 0.8 逻辑 < 1 物理像素量级；
宽 704 是逐像素准的（动画只改 `opacity/scale/y`）。

## 13. 协作纪律（每轮都适用）

- **cargo 只跑改动范围**（维护者多次强调，全量测试会拖很久）：`cargo build --bin Lithe`；
  测试用 `cargo test -p <只动过的那个 crate>`，能按测试名过滤就再加过滤
  （例：`cargo test -p lithe-gpui-shared every_wired_key_resolves_in_both_locales`，实测 2.6s）。
  **不要** `-p a -p b` 连带，更不要跑 workspace 全量。
- **同一时刻只有一个 cargo 写者**（`gpui/target` 是排他锁）：代理之间**严格串行**。阶段 9 与阶段 10
  第一批都动 `crates/editor` 与 `shared/locales`，所以必须一前一后，不并行。
- **验证要到交互级**，不只是静态截图：`.artifacts/ui-click.ps1`（真实鼠标注入，**客户区坐标**，
  `-Probe` 只报几何用于标定）、`gpui/capture-screenshot.ps1`（按 pid 选窗口、排除控制台窗口）。
  本机 **125% DPI，截图是物理像素**（逻辑值 × 1.25 = 截图像素值）；会话里存在**"杂散点击"**
  （光标停着时会落到窗口上），测试前先把光标挪离活动栏 —— 但也要注意**维护者可能同时在手动操作**。
- 界面验收要对照 `docs/gpui-kit/0.6.6/zh-CN/docs/design-guides.md` 的 **Design review checklist**
  与**无障碍检查表**逐条自检。

## 14. 阶段 14：设置剩余页（第一批：编辑器 / 终端，2026-09-25）

规格：`research/windows/07-settings-ui.md` §3.4（编辑器页 4 项）、§3.6（终端页 1 项）、
§7.3（逐项"能否立刻生效"）、§7.4（建议范围）。Windows 渲染点
`features/settings/components/macos-settings-panels.tsx:275-328,372-396`。
HANDOFF §4 的队列里这一项是"设置剩余页（除 AI）"，本批做的是其中**能真生效**的前两项。

### 14.1 落了两个分类、三个键

分类从 2 → **4**（常规 / 外观 / **编辑器** / **终端**），顺序是 Windows 12 个分类表里的相对次序子序列。

| 设置键 | 控件 | 落点（立即生效） | 真源 |
| --- | --- | --- | --- |
| `fontSize`（默认 14，10–22） | 数字输入 | `Theme.mono_font_size` → 编辑器正文；`apply_config` 之后补写（见 `theme.rs`） | `macos-settings-panels.tsx:283-292`、`typography-defaults.ts:13` |
| `tabSize`（默认 2，2/4/8） | 下拉（`N 个空格`） | 每个 `EditorState` 的 `TabSize`，**已打开的 buffer 也重设** | `:313-325`、`default-settings.ts:55` |
| `terminalDefaultShellId`（默认 `""`） | 下拉（系统默认 / PowerShell / 命令提示符 / WSL） | `TerminalPane::set_default_shell` → **新建**会话用哪个 shell | `:379-393` |

**"值"型设置经外壳转发**：`tabSize` 与终端 shell 都是"面板自己不认识设置 crate"的值
（依赖方向是 `workbench` → `editor`/`terminal`，反向会成环），所以由 `ShellWorkspace` 订阅
`SettingsStore` 后读出来喂给两个面板 —— 与「显示状态栏」同一条路子，只是多了一个"值要送出去"的动作。
`TerminalPane::set_default_shell` 做成**幂等**（值没变就直接返回）：不然用户在页签条 ⌄ 菜单里
手动选的配置文件会被"隔壁开关动了一下"重置掉。

### 14.2 设置**写得出、读不回**（`persistence.rs`）——已根治：键表由 schema 派生

历史上 `settings_from_object` 是一张**手写的逐键表**；新键不登记进去，文件里写得再对、读回来也是默认值，
而且**没有任何诊断**（文件本身完全合法）。实测：文件里 `"tabSize": 8`，启动后
`S1_SETTINGS wiring=workbench tab_size=2`。

**现在不存在第二份键名清单。** `persistence::known_keys()` 直接取 `Settings::default()` 的序列化结果，
坏键回落路径遍历它，所以：

- 先整体 `serde_json::from_value::<Settings>`（认**所有**字段）；只有某个键的类型真坏了才退到逐键容错；
- 逐键那一步自动覆盖新字段 —— **加新键不需要再改 `persistence.rs`**；
- 守护测试从 `every_key_survives_a_round_trip` 扩到
  `any_single_broken_key_leaves_every_other_key_intact`：逐个已知键喂 `null`，要求其余键全部完好。
  任何新字段没进入回落路径，这条测试立刻不等（它同时断言"喂的值确实被判成坏键"，
  所以将来若真有字段接受 `null`，测试会明确失败并提示换一个非法值，而不是静默变弱）。

同一次改动还落地了两条**文档层**性质（完整理由在 `persistence.rs` 的模块文档）：

| 性质 | 行为 |
| --- | --- |
| 顶层 `version` | 写回时写当前 `DOCUMENT_VERSION`；文件声明的版本比程序新时**转只读**，只读期间不落盘（内存里照常生效） |
| **未知键原样保留** | 写回前先用上一次文件的原始对象合并（含嵌套对象内部的键）。用户手写的字段、另一个版本写的字段都不会被吃掉 |

这两条一起解决的是"设置文件是**给人改的**"这件事：早先"未知键静默忽略"会在下一次落盘时
删掉用户手写的键，那是不可逆的数据丢失。

### 14.3 不画假控件：编辑器页真源 4 项只做 2 项

`codeLens`（显示用法与 Git 作者）与 `horizontalTabScroll`（缓冲区轮播）在 gpui 侧
**没有消费方**（无 code lens / 无行内 blame / 标签条没有轮播形态），按 §7.3-D 的"应隐藏或标注"
整项不画，理由写在 `dialog.rs::editor_page` 的文档里。

一处**有意的副作用**要记住：真源把 `fontSize` 与 `terminalFontSize` 分成两个键，
而 gpui 的主题只有**一个** `mono_font_size`（编辑器与终端正文共用）。终端页真源只有「默认 Shell」一项，
所以本侧不加 `terminalFontSize` 控件（会是"存了没用"的键），改在编辑器字号那行加一句描述说明作用范围
（新键 `settings.gpui.editorFontSizeDescription`，理由在 `extract-locale.mjs` 的 `GPUI_ONLY_KEYS`）。

### 14.4 验证（详见 `.artifacts/p12/NOTES.md`）

| 检查 | 结果 |
| --- | --- |
| `cargo build --bin Lithe` | 0 error |
| `cargo test -p lithe-gpui-settings` | 35 passed（含两条新的往返/坏键守卫） |
| `cargo test -p lithe-gpui-terminal` | 7 passed |
| `cargo test -p lithe-gpui-workbench` | 28 passed |
| `cargo test -p lithe-gpui-shared every_wired_key_resolves_in_both_locales` | passed（新接线 16 条键） |
| `node gpui/tools/extract-locale.mjs` | 两 yml 各 4336 条 key |
| 交互级（真实鼠标注入） | 4 个分类可点、两个下拉可改、落盘可验；`S1_EDITOR_TAB_SIZE size=4 buffers=1`（已开 buffer 被重设）；字号 20→14 时同一文件同一滚动位置文字行数 21→31 |
| 终端端到端 | `""` → `S1_TERMINAL_TAB profile=powershell`；`"cmd"` → `S1_TERMINAL default_shell=cmd outcome=applied` + `profile=cmd`（页签写 `cmd`、正文是 cmd.exe 横幅） |

### 14.5 其余分类：**明确空态**（不是空壳）

左栏补齐到 **11 项** = Windows 分类表（`settings-dialog.tsx:35-48`）去掉 AI 两个之后的 10 项，
**外加「外观」**（真源对话框里没有这个分类 —— `tabs/appearance-settings.tsx` 是死代码，见 §8.2 与
`07-settings-ui.md` §5.1；本侧必须留着它，因为主题要有落点）。

| 类别 | 分类 | 页面 |
| --- | --- | --- |
| 实现页（4） | 常规 / 外观 / 编辑器 / 终端 | 有控件，全部真的有消费方 |
| **空态页（7）** | 项目 · JDK 与 Maven / 运行配置 / 快捷键 / LSP / Git / 日志 / 更新 | 页标题 + `Empty`（分类图标 + 「此分类尚未接入」+ 一句前置条件），**没有一个控件** |

为什么空态页也要列出来（HANDOFF §4 的口径）：用户点进一个分类期望看到"这里能配什么、为什么现在没有"，
而不是"这个分类不存在"。而**不画假控件**是同一句话的另一半 —— 一个永远不生效的开关比不画更容易骗人
（`07-settings-ui.md` §7.3-D）。

两条结构化守卫（`settings/src/dialog.rs` 的测试）：`categories_match_the_windows_list_without_ai`
钉住左栏顺序就是真源子序列 + `appearance`；`every_category_is_implemented_or_declares_a_prerequisite`
钉住"每个分类要么是实现页、要么登记了前置条件"（漏登记的空态页会在运行期 `expect` 炸）。

前置条件文案是 7 条新的 `settings.gpui.prerequisite*`（+ 1 条标题 + 1 条编辑器字号描述），
理由逐条写在 `extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 里。
`cargo test -p lithe-gpui-settings` = 36 passed；`13-nav-11.png` / `14-empty-project.png` / `15-empty-updates.png` 是实机证据。

### 14.6 未做 / 已知边界

1. **`Ctrl+,` 仍是"没被机器验证"**：本轮 `ui-keys.ps1` 打印出 `SetForegroundWindow` 被拒
   （`目标 hwnd ≠ 注入前的前台 hwnd`），键送到了别的窗口 —— 是注入侧的限制，不是绑定坏了。
   对话框的两条入口 `--open-settings` 与**活动栏齿轮（真实点击）**都验证过。
2. **`outcome=unavailable` 本机跑不到**：`powershell` / `cmd` / `wsl` 三个都在 PATH 上，
   那条分支只有单测守着。
3. 还没做的（按顺序）：~~**Git 身份**~~（阶段 15 已做，见 §15）；**LSP 页的 jdtls 运行时路径**
   （`java/src/jdtls.rs:440-460` 现在是 env/JAVA_HOME/PATH 三级发现）；
   **其余分类的明确空态**（项目 / 运行配置 / 快捷键 / 日志 / 更新）。

---

## 15. 阶段 15：设置「Git」页 = 提交身份 + 一个真有消费方的开关（2026-09-26）

规格：`research/windows/07-settings-ui.md` §3.10（Git 页逐项规格，11 项 + 2 个子面板）、
§7.3（逐项"能否立刻生效"）、§7.3-D（没有消费方的项应隐藏，不画假控件）；
契约 `shared/contracts/git-repository-setup.md` + `rust-core-api.md:169,171`；
渲染真源 `windows/tauri/src/features/settings/components/git-identity-settings.tsx` 与
`components/tabs/git-settings.tsx:93-106`。这是 HANDOFF §4 队列第 2 项里最后一块
"能真生效"的设置页（LSP 页仍缺数据源）。

### 15.1 落地形态：宿主钩子（依赖方向不破）

`lithe-gpui-settings` 只许依赖 `gpui-kit` + `lithe-gpui-shared`，而两条身份命令住在
`lithe-gpui-git`。照仓库里已有的同类口径（`editor/src/editor_view.rs` 的 `TabMenuHostActions`）：

```text
settings::identity::{GitIdentityHost, GitIdentityPage, set_git_identity_host}
        ▲                                    │ 由 ShellWorkspace::new 登记（用 lithe-gpui-git 的实现）
        └── 未登记 → git_page 退回「明确空态」 ┘
```

三个关键取舍（详见 `settings/src/identity.rs` 的模块文档）：

1. **边界只交换 JSON 文本**，不在钩子签名里出现 `lithe_gpui_git` 的类型 ——
   否则 settings 为了写出那个类型又得反向依赖 git，钩子就成摆设。
2. **钩子放线程局部**（`RefCell<Option<Rc<..>>>`）而不是 `App::set_global`：登记有 `App` 没问题，
   但**读取发生在 `SettingsDialog::render`**，那里只有 `&mut Context<Self>`，gpui 的 `Context`
   不暴露 `try_global`。
3. **`Task` 必须 `detach()`**：gpui 的 `Task` 一 drop 就取消
   （`gpui-pre-scheduler-0.3.6/src/executor.rs:389-391`）。第一版写成 `let _ = cx.spawn_in(..)`
   的实测现象是"`run=load` 有、`result=` 永远没有、界面卡在「正在检查 Git 仓库…」"。
   另外 `Context::spawn` 给的是 `AsyncApp`（拿不到窗口），所以设置侧用 `spawn_in` 起任务、
   把 `AsyncWindowContext` 交给宿主去回前台投递。

### 15.2 界面做了两件事

| 项 | 真源 | 落点 |
| --- | --- | --- |
| 提交身份（作用域 local/global + 姓名/邮箱，各自保存 + 清除覆盖 + 当前生效值） | `git-identity-settings.tsx` | `git.repositorySetup` / `git.configureIdentity`（经宿主钩子） |
| 「丢弃前确认」`settings.git.confirmDiscard`（默认 `true`） | `tabs/git-settings.tsx:93-106` | `ChangesView::set_confirm_before_discard` → 丢弃路径决定要不要先弹确认框 |

**本页零新增 locale 键**：整页文案来自真源既有的 `git.setup.*`（17 条）与
`settings.git.*`（3 条），`extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 一个字没动、
两份 yml 也没变（仍各 4344 条）。

真源那页另外 9 项（`gitExecutable` / 凭据助手 / 三个 Fetch 项 / `coreFeatures.git` /
`autoRefreshGitStatus` / `gitChangesFolderView` / 5 个视图开关 / `gitDefaultDiffView` /
`enableInlineGitBlame`）**一项都没画**，逐条理由写在 `settings/src/dialog.rs::git_page` 的文档
（没有消费方的一律不画，`§7.3-D`）。同处还登记了两个**明确的缺口**：
`git.initialize`（非仓库页的「初始化 Git 仓库」按钮，需要自己的确认框）与
`git.setup.openProject`（gpui 的 `Lithe` 恒带工作区根，"没有项目"这个状态到不了）。

### 15.3 新增一个设置键（走完 §14.2 的硬前提）

| 键 | 默认 | 落点 | 真源 |
| --- | --- | --- | --- |
| `confirmBeforeDiscard` | `true` | 外壳订阅 `SettingsStore` 后转发给 `ChangesView::set_confirm_before_discard`（幂等） | `default-settings.ts:194`、`tabs/git-settings.tsx:93-106` |

两处同时改到位：`schema.rs`（字段 + `Default` + 序列化键名测试）、`store.rs`（
`set_confirm_before_discard`）。**`persistence.rs` 不需要改**（§14.2：键表由 schema 派生）；
`any_single_broken_key_leaves_every_other_key_intact` 与 `every_key_survives_a_round_trip`
会自动用非默认值覆盖它，所以"写得出、读不回"那条老 bug 不会再发生。

### 15.4 验证（详见 `.artifacts/p13/NOTES.md`）

| 检查 | 结果 |
| --- | --- |
| `cargo build --bin Lithe` | exit=0（`Finished dev profile … in 1.58s`） |
| `cargo test -p lithe-gpui-git` | **23 passed**（新增 10 条：请求体 / 响应解析 / 值校验 / 路径剪裁） |
| `cargo test -p lithe-gpui-settings` | **46 passed**（新增 7 条：空态降级 / 线格式 / 按钮判据 / 新键往返） |
| `cargo test -p lithe-gpui-workbench` | **29 passed**（新增 `identity_enums_map_both_ways`） |
| `cargo test -p lithe-gpui-shared every_wired_key_resolves_in_both_locales` | passed（新接线 22 条真源既有键） |
| `node gpui/tools/extract-locale.mjs` | 两 yml 各 4344 条 key，**产物无变化** |
| 交互级（真实鼠标注入 + PowerShell 侧独立核对） | 读身份（`result=ok … effective_name=true`）、local 保存 name/email（`git config --local` 真的变了）、两次清除覆盖（`--local` 真的没了、`--list \| Select-String user\.` 为空）、非仓库 + local 全禁用 + 三行提示、启动态 `confirm_before_discard=false` 同时出现在 `S1_SETTINGS wiring=workbench` 与 `S1_SOURCE_CONTROL` |

**已知边界（照 NOTES §3.3 / §4 的原文）**：global 作用域的**真实写入没有从 UI 走通** ——
作用域下拉点了不弹菜单（网格试了 7 个点、内容区像素直方图完全一致），3 次没弄好就停下；
本机真实全局身份也没有被写（全部验证跑在**假 HOME** `.artifacts/p13/home` 下，
`C:\Users\admin\.gitconfig` 字节未变）。`confirmBeforeDiscard` 只有"值真的到消费方"这一步有日志证据，
"关掉后丢弃不再弹确认框"这条**交互**本轮没跑到（需要在左栏「更改」里点「更多 → 丢弃全部更改」）。

### 15.5 未做项（按顺序）

1. **global 作用域的 UI 路径**：给 Git 页加一个"初始作用域 = global"的启动态旗标
   （HANDOFF §2 的退路），绕开这次没接住的点击，再在假 HOME 下把 global 写入跑通。
2. **`git.initialize`**：非仓库页的「初始化 Git 仓库」按钮 + 确认框（键全在 locale 里：
   `git.setup.initializeTitle` / `initializeDescription`）。
3. **`confirmBeforeDiscard` 的交互证据**：左栏「更改」活动栏项的坐标要先标定。
4. **LSP 页**：jdtls 运行时路径（`java/src/jdtls.rs:440-460` 现在是 env/JAVA_HOME/PATH 三级发现）；
   真源那三个开关（`autoCompletion` / `parameterHints` / `semanticTokens`）在 gpui 侧**没有消费方**，不画。

---

## 16. Java 第一优先级 · 第一批：智能提示（代码补全，2026-09-26）

**维护者口径（2026-09-26）**：Java 生态第一优先级，**用户体验高于一切**；
编辑器智能提示 + 代码跳转 → Maven / Spring / Spring Boot；**Git 相关先不做**。

规格来源（都已进仓库）：

| 文件 | 内容 |
| --- | --- |
| `gpui/research/editor-lsp-completion.md` | 上游编辑器 LSP 能力清单（6 个 provider 槽位）、触发链路、`lsp_types` 形状、Core 命令序列、分步计划 |
| `gpui/research/java-spring-maven-inventory.md` | 载荷 106 个 bundle 逐个枚举（**有 m2e、没有 Spring**）、Windows/macOS 的 Java 实现、Core 的 Java 命令、Maven 最小可用路径、Spring 三条路、**不许自研清单** |

### 16.1 补全菜单**一行没写**

上游 `gpui-base` 0.6.6 的编辑器自带整套补全 UI（弹窗渲染、`↑↓`/`Enter`/`Esc`、鼠标点选、
`filterText` 过滤），挂载点就是 `Lsp.completion_provider`
（`gpui-base-0.6.6/src/input/editor/lsp/mod.rs:39`，经 `EditorState::lsp_mut()` 赋值；
官方接线样例在 `gpui-component-0.6.6/src/inspector.rs:104-107`）。
我们只写一个 `CompletionProvider` 适配器 —— 这正是 `develop-lithe`「复用成熟上游」要求的样子。

| 落点 | 内容 |
| --- | --- |
| `crates/java/src/service.rs` | `completion()`：`ensure_session` → `sync_document` → `lsp.request{operation:"completion"}` → 解析 `{items:[…]}`（`client.rs:871-876,937-965`）；**snippet 用 Core 的 `lsp.plainSnippet` 还原**；`staleDocumentVersion`/`requestCancelled` 翻成 `Ok(空)` |
| `crates/editor/src/completion.rs`（新） | provider：JDTLS 优先、`lsp.builtinCompletions` 兜底；`textEdit` 透传；`sort_text`/`filter_text` 原样过；120ms 防抖 + 代次闸门 |
| `crates/editor/src/editor_view.rs` | `open()` 装 provider（仅 Java buffer）；`prepare_java()` 给**已打开**的 Java buffer 补装 |
| `crates/editor/src/navigation.rs` | `core_position` 提成 `pub(crate)`：补全与跳转共用同一条"字节偏移 → UTF-16 列"换算 |

### 16.2 四条口径（每条都对应一类会毁体验的 bug）

1. **JDT 答了就是权威**：`Ok(items)`（含空列表）不回落；只有 `Err`（不可用/超时）才回落轻量补全
   —— 与 `navigation.rs::resolve_target` 同一条口径。
2. **`staleDocumentVersion` / `requestCancelled` 不是失败**：快速打字时 Core 会因新 `syncDocument`
   自动取消旧补全（`engine.rs:4348-4360`），这是常态；翻成 `Ok(空)` 才不会误触发降级、
   菜单才不会在 jdtls ↔ builtin 两种候选之间乱跳。
3. **`textEdit` 必须带、且列口径要换算**：没有它上游就"在光标处再插一段"⇒ `Sys` + `System` = `SysSystem`；
   而 Core 的 `utf16Column` 是 **UTF-16 码元**，上游 `position_to_offset` 当**字符**解
   （`rope_ext.rs:330-343`）⇒ 入站过 `navigation::editor_position`、出站过 `navigation::core_position`。
   含 emoji 的行上不换算必错（单测 `item_mapping_converts_utf16_columns_to_character_columns` 钉住）。
4. **snippet 必须还原成纯文本**：JDT 大量候选是 `insertTextFormat: 2`（`greet(${1:name})`），
   上游插入路径不认识 snippet 语法（`overlay.rs:166-197`），还原走 Core 的 `lsp.plainSnippet`
   （进程内同步调用，逐条转换代价可忽略），别在 gpui 侧再写一个转换器。

### 16.3 验证

| 检查 | 结果 |
| --- | --- |
| `cargo build --bin Lithe` | exit=0 |
| `cargo test -p lithe-gpui-java` | 14 passed |
| `cargo test -p lithe-gpui-editor` | 24 passed（新增 6 条补全单测） |
| `cargo test -p lithe-gpui-workbench` | 29 passed |
| **真实 JDTLS 端到端**（`LITHE_GPUI_JDTLS_SMOKE=1` + `LITHE_JDTLS_JAVA=<JDK21>`） | `S1_JAVA_COMPLETION … line=5 col=35 items=10 ms=152`；断言：候选含 `greet`、**每条都有 `text_edit`**、`insert_text` 里没有 `$` |

⚠️ **交互级验证已补上（2026-09-26 同一批）**：在 Maven 夹具
（`.artifacts/p14/fixture`：`pom.xml` + `App.java` + `Greeter.java`）上真实注入——
JDTLS 起会话并就绪 → 点开 `App.java` → 在 `greeter` 后输入 `.` →
`S1_JAVA_COMPLETION … line=3 col=16 items=10 ms=64` + `source=jdtls prefix="greeter." items=10 ms=192`
→ **菜单真的渲染**（`12-completion.png`：`args : String[]` / `greeter : Greeter` /
`main(String[] args) : void` / `App` / `class` / `cast` / `null` / `opt` 等真实语义成员）
→ 键盘 `Return` 接受条目后**正文真的变了**（第 4 行 `greeter..args`，光标 18 → 22 = `args` 长度）。
**没复现成功的**：鼠标点选条目（点了之后弹窗关闭、光标跑到别的行）—— 两种可能（点偏 / 截图与点击间隔 2s
弹窗已自关）**没有区分开**，键盘路径是成功的；下一轮用"先截图、立刻点"的口径重测。
另外记两条：**强杀 Lithe 会留下 JDTLS 的 `java.exe`**（走不到 `lsp.stopServer`），需单独清理；
弹窗候选文字偏暗，值得按无障碍检查表量一次对比度。详见 `.artifacts/p14/NOTES.md` §4.3。

### 16.4 已知边界（别当已解决）

1. **自动补 import 会丢**：上游 `insert_completion` 只应用 `text_edit` / `insert_text`，
   **完全忽略 `additionalTextEdits`**（`overlay.rs:166-197`），而 JDT 的自动 import 正靠后者。
   要补就得自己落（Core 有 `lsp.applyTextEdits`）。
2. **没有 `Ctrl+Space`**：上游**没有任何补全 action/键位**（全 registry grep `ShowCompletions` 零命中），
   只能打字触发。要加就得我们自己定义 action 并在编辑器上绑。
3. **菜单不做前缀过滤**：候选集合完全由服务端按位置给；我们透传服务端的 `sort_text`/`filter_text`。
4. **诊断（波浪线）还没有**：现有 `Session::request` 自己消费 `lsp.waitEvents` 并丢弃非本 `operationId`
   的事件 —— 再加订阅会互相偷事件，必须先改成"单一事件泵 + 按 operationId 分派"。
5. ~~**Maven context 没传**~~ → **本批已做**（见 §16.6）。

### 16.6 Maven 项目模型：把 `mavenContext` 送进 `lsp.startServer`（2026-09-26）

**做了什么**：`crates/java/src/workspace.rs` 新增 `maven_context(root)` —— 调 Core 的 `maven.scan`
拿 `relativePath` / `profiles` / `settingsPath`，构造 Core 的 `MavenLaunchContextRequest` 形状
（`rust/lithe-core/src/project/maven.rs:63-78`：`version` / `reactorPath` / `profiles` /
`settingsPath` / `skipTests`，camelCase），由 `session.rs` 在 `lsp.startServer` 的 payload 里带上
（`None` 就**不带**这个字段：空 reactor 只会让 Core 做无意义的校验）。**没有解析 pom.xml** ——
`maven.scan` 就是上游给的入口。诊断行 `S1_JAVA_MAVEN context reactorPath=… profiles=… settings=…`。

**为什么值钱**：Core 拿到 `mavenContext` 后会 ① 把 reactor 的 main / test / generated 四类源根归一成
`java.project.sourcePaths`（**生成源根生效**：注解处理器产出的代码也能补全/跳转）；
② `ServiceReady` 后按项目下发 `org.eclipse.m2e.core.selectedProfiles`（profile 生效）；
③ 发布 `java.configuration.maven.userSettings`。

**验证**：
- 单测 3 条（`workspace.rs`）：无 scan 结果 → `None`；根 pom → `reactorPath="."`、profiles 取 `id`、
  `settingsPath` 透传、**没有数据源的字段不出现**；子模块 reactor → `reactorPath` 原样。
- 选定式真实 JDTLS（夹具**加了 `pom.xml`**，并改成标准 Maven 布局 `src/main/java/demo/`）：
  `S1_JAVA_MAVEN context reactorPath=. profiles=0 settings=false` → 会话正常起来（Core 校验通过）
  → 跨文件定义仍解析 → `S1_JAVA_COMPLETION … line=5 col=35 items=10 ms=392`。`cargo test -p lithe-gpui-java` = 17 passed。
- **一条意外的强证据**：把源码放在非标准的 `src/demo/` 时，一旦带上 `mavenContext`，`Greeter` 就解析不到了
  —— 说明这个字段**真的改变了 JDT 的项目模型**（源根从"目录里有 .java"变成"Maven 模型"）。
  夹具因此改成标准布局；这条也写进了测试注释。

**边界**：`localRepositoryPath` / `mavenExecutablePath` / `javaHomePath` 一律**不传**
（没有数据源，宁可不传也不猜）—— 那些属于「项目环境设置」，等 Java 项目页做出来再接。

### 16.7 右侧 Maven 工具窗：从"未检测到 Maven 项目"变成真实项目结构（2026-09-26）

**数据层**（`crates/workbench/src/maven.rs`，新）：`maven.scan` → `MavenProjectView`
（reactor 根 + 递归 `MavenModuleView` + `profiles` / `active_profiles`）+ `scan(root)` + `parse()` 纯函数。
**归属**：`maven.scan` 是 **Core 的项目命令**（不是 LSP 会话事实），所以放 workbench、
用 `core_json` 直接问 Core，**不解析 pom.xml**（那是 m2e / `maven.scan` 的活）。

**表现层**（`right_tool_window.rs`）：`right_tool_window(view, maven, …)` 多收一个
`Option<&MavenProjectView>`；Maven 有项目时画「reactor 头（`artifactId` + 版本 + 相对路径）+
模块树（递归、每层 `pl_3()`，缩进跟着 rem 档位走）+ profile 列表（默认激活的用主色 **和** 中粗
两种信号，不只靠颜色）」，没有项目时保持真源的 `maven.notDetected` 空态。
真源那页的工具栏 / 生命周期 / 依赖树 / 构建输出**不画**（要 `mvn` 执行与依赖解析的数据源，
画了就是假控件）。

**懒扫 + 缓存**：`ShellWorkspace` 新增 `maven_project` / `maven_scanned`，**第一次真正显示**
Maven 面板时才扫（右活动栏点击与菜单 `ToggleMaven` 两条路都接），避免启动首帧为不相关的面板付钱。
诊断 `S1_MAVEN scan=ok artifactId=… version=… reactorPath=… modules=… profiles=…`
（面板里每个事实都能在这行核对）。

**验证**：`cargo build --bin Lithe` exit=0；`cargo test -p lithe-gpui-workbench` **33 passed**
（含 4 条 `parse` 单测：null/缺字段 → 不是项目、根 pom 字段落位、递归层级与计数、label 兜底）；
实机（Maven 夹具 + `--menu-probe view lithe.workbench.maven`）：
`S1_MAVEN scan=ok artifactId=lite-fixture version=1.0.0 reactorPath=. modules=1 profiles=0`，
截图 `22-maven-pane.png` 里面板显示 `lite-fixture 1.0.0` + `.`，空态消失。

### 16.8 右侧「Spring」视图（数据层 §16.9 + 面板，2026-09-26）

**数据层**（`crates/workbench/src/spring.rs`）：把 Core 的 `spring.index` 解析成
`SpringIndexView`（端点 `label()` + 各集合计数）+ `S1_SPRING` 诊断 + 4 条单测。
`spring.index` 读工作区元数据**与依赖 JAR 里的 `spring-configuration-metadata.json`**
（契约 `:1737-1758`），所以**一行注解解析都不写**（`verify-java-semantic-ownership.mjs` 把这类 token 列为禁区）。

**面板**：右活动栏新增第 4 项（`RightToolWindowView::Spring`，图标 Lucide `leaf` ——
真源 `spring-icon.tsx` 是内联品牌 path，没有 SVG 文件；与 Maven 用 `package` 同一取舍）；
内容 = 端点列表（`GET/POST /api/users` + 弱化的 `控制器.方法`）+ 一行计数
（properties / values / refs / injections / beans / diagnostics）。空态文案是本侧新增的
`lithe.spring.notDetected`（说「组件与端点」而不是「项目」——不把"没有 Spring 的 Java 项目"
说成"没检测到项目"）。懒扫 + 缓存与 Maven 同口径（`spring.index` 更贵：会读依赖元数据）。

**验证**：`cargo build --bin Lithe` exit=0；`cargo test -p lithe-gpui-workbench` **38 passed**
（新增 rail 映射 3→Spring、id 契约、`spring::parse` 4 条）；WIRED 键测试 passed（新增 2 条键，
`spring.title` 进 `SAME_IN_BOTH_LOCALES`）；`extract-locale.mjs` 两 yml 各 4346 条。
实机：`S1_SPRING index=ok endpoints=0 beans=0 properties=10 values=0 refs=0 injections=0 diagnostics=0`
（**懒扫真的在第一次激活时跑了**，且从本机依赖元数据里读到了 10 条属性），
`S1_RIGHT_PANEL view=spring visible=true`，右栏第 4 个图标在截图里可见。

### 16.9 本轮并行四路的结论（2026-09-26，含两条产品级发现）

**① 右栏启动态探针 `--right-view <id>`**（`app/src/main.rs` + `workbench/{right_tool_window,workspace}.rs`）：
`from_id` 与 `id()` 双向一致、未知 id 打一行可 grep 的 stderr 且**不改启动状态**；
应用走 `window.on_next_frame`（不在 render 阶段改状态）；顺带把"右活动栏点击"与
「视图 → Maven」菜单两处**重复且已经有分歧**的懒扫抽成 `scan_right_view_if_needed`。
实机 13 条判定全 OK：Maven 面板看到 `lite-fixture 1.0.0` + `.`；Spring 面板看到计数行
`properties 10 · values 0 · …`（不是空态）；未知 id 与基线图像素一致；收尾 `Lithe=0 java=0`。

**② "右活动栏点击双触发"——我原来的说法是错的（2026-09-26 经源码级诊断推翻）**：
一次性读到的事实是 `ClickEvent::Mouse` 只在 **MouseUp 的 bubble 阶段产生一次**
（`gpui-base-0.6.6/src/div.rs:3100-3121`；`pending_mouse_down` 是单个 `Option`，首个 MouseUp 就 take，
`:3084-3098` ⇒ **多一个 MouseUp 反而产生 0 次额外 click**，要凑出第二次 click 必须**完整多一对 DOWN+UP**）；
我们侧也没有"记两遍"（`activity_bar.rs:403` 每项一次 `on_click`）；DPI 无证据参与
（坐标只除一次、hitbox 与鼠标位置同为逻辑坐标、右栏节距 40 物理、注入点都在项中心）。
可对齐注入次数的日志全是 **1:1**（8 次投递 → 8 行）。所以：**不是"一次点击被派发两次"的产品缺陷**，
现有证据指向"**确实发生了两次输入**"，第二次来源**已由维护者确认为他本人手动操作**
（2026-09-26 澄清：那次"无人点击却自动开文件、按 F12、往源码敲字"的输入是他自己点的/敲的）。
⇒ **不是 gpui 派发层缺陷，也**不是**环境缺陷**。完整证据见
`gpui/research/click-double-trigger-dpi.md`（其 §5 已按此更正）。
⇒ **不要再把它当"面板类验证拿不到截图的原因"**；正确的做法是**注入前后用 `SetCursorPos` 把真光标
移离右栏**（`.artifacts/right-panel/NOTES.md:30` 说 `-Mode move` 会挪真光标，**那句是错的**：它只发
`WM_MOUSEMOVE`）。同时**不要**改 toggle 语义、**不要**加防抖 —— 真机就是一次 click 一个 toggle。
附带发现（真问题，性能）：`workspace.rs` 的懒扫是在 **MouseUp 派发栈里同步跑**的（Spring 会读 JAR），
应当延后出派发栈。

**③ `spring.index` 的 `endpoints` 恒为 0 是我上一批的调用 bug（不是夹具问题）**：
Core 的 `endpoint_index`（`rust/lithe-core/src/languages/spring.rs:1477-1552`）是**纯文本/正则**索引，
**与 classpath 无关**；而 `paths` 是 `#[serde(default)]`，我调用时**没传** ⇒ 一个 Java 文件都没扫
（`properties=10` 正好是 Core 的 10 条内置属性，就是"没扫工作区"的铁证）。修法（另一路在做）：
有界收集工作区 `.java`（排除 `target/build/out/.git`、有上限、相对路径、确定性排序）后传 `paths`。
**教训**：诊断行里的数字要读完再下结论 —— 我当时把 `endpoints=0` 归因成"夹具缺依赖"，是错的。

**④ 环境事实（下个会话别再踩）**：本机**没有** `~/.m2/settings.xml`，真正的本地 Maven 仓库是
`D:\ProgramData\maven\apache-maven-3.6.3\conf\settings.xml:53` 指到的
**`D:\ProgramData\maven\apache-maven-3.6.3\maven_new`**（`C:\Users\admin\.m2\repository` 里一个 spring 都没有）；
镜像 `alimaven`。跑 `mvn` 时只给该进程设 `JAVA_HOME=D:\ProgramData\java\openjdk-21`
（本机 PATH 上的 `java` 是 1.8）。Spring 夹具（`.artifacts/p15/spring-fixture`，Boot 3.4.5）的项目依赖
**100% 复用缓存**，只有插件走了一次网；`mvn -o … compile` 之后完全离线可复现。

⚠️ **仍未取证**：`endpoints > 0` 的 GUI 端到端（等 `paths` 修完，用探针在
`.artifacts/p15/spring-fixture` 上复验，期望 `endpoints=2 beans=1 values=2`，面板出现
`GET/POST /api/users`）。

### 16.5 下一批（按用户可感知价值）

1. 补上"菜单真的弹出来"的交互级证据（打字注入 + 截图 + 诊断原文）。
2. 自动补 import（`additionalTextEdits`）+ `Ctrl+Space`。
3. ~~**Maven context 送进 `lsp.startServer`**~~ → **本批已做（§16.6）**，Maven 项目的源根 / profile / settings
   现在会同步给 JDT。下一步的自然延伸是**右侧 Maven 工具窗**用同一份 `maven.scan` 结论显示模块与 profile。
4. 会话改事件泵 → 诊断波浪线（同时是 Spring 端点/注入提示的前置）。
5. **Spring**：载荷里没有 Spring 插件（已核实），走 Core 的 `spring.index`
   （读 `~/.m2` 的 spring 元数据；macOS 已用它做补全/hover/端点），不捆新载荷。









