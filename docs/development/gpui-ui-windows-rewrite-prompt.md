# 提示词：用 GPUI Kit 重写 Lithe 界面（规格照 Windows 前端）

> 这是一段**可直接复制给新对话里的 AI 编码代理的完整提示词**。从「## 你的任务」开始整段带走。
> 本文件由上一轮对话整理，包含了当时已经查清的事实、已经踩过的坑和已经拍板的决策 —— **不要跳过第 3 节**。

---

## 你的任务（一句话）

在 `gpui/` 工程里**清空重写** Lithe 的 GPUI Kit 界面：**规格来源是 Windows 前端**（`windows/tauri/src/`，Tauri v2 + React + Tailwind + shadcn），
**不是 macOS**；实现框架是 **gpui-kit 0.6.6**。

## 0. 先读这些（顺序不要换）

1. 仓库根 `AGENTS.md` → 规则真源 `.agents/skills/develop-lithe/SKILL.md`，**动手前必须读**。
2. 会写笔记读 `.agents/skills/agent-notes/SKILL.md`；会写测试读 `.agents/skills/write-stable-tests/SKILL.md`。
3. **`gpui/UI-MAP.md` §1**（硬规则 + gpui-kit 实现规则 + 版本陷阱）—— 这是上一轮用真机踩出来的，**先读完再写代码**。
4. **Windows 界面调研（6 份，规格真源）**：`gpui/research/windows/01..06-*.md`
   （外壳 / 编辑区+侧栏 / Git+底部窗 / 主题+组件 / 终端+运行调试 / 全局面板）。每份都带 `windows/tauri/src/...:行号` 证据。
5. `gpui/research/windows/` 之外，造型与文案的**一手真源**永远是仓库里的 Windows 前端源码本身；调研文档只是索引。
6. `gpui/UI-MAP-macos.md`：macOS 界面调研，**只作行为/功能对照**（查"这个交互原本怎么工作"），**不要当尺寸/布局规格**。

## 1. 规格来源（Windows 前端）—— 关键路径速查

| 要什么 | 去哪里看 |
| --- | --- |
| 外壳/三栏/可调分栏 | `windows/tauri/src/features/layout/components/main-layout.tsx`、`resizable-pane.tsx` |
| 标题栏与窗口三键 | `features/layout/components/title-bar*`、`window-controls.tsx` |
| 活动栏（左右两条） | `features/layout/components/sidebar/*` 与 layout 下的 rail 组件 |
| 状态栏 | `features/layout/components/footer/*` |
| 侧栏与项目树 | `features/layout/components/sidebar/main-sidebar.tsx` + 项目树组件 |
| 编辑区（标签栏/pane/空态） | `features/panes/*`、`features/tabs/*`（含 `utils/path-shortener.ts`）、`features/editor/*` |
| Git 工具窗 | `features/git/components/log/*`（`git-log-tool-window.tsx`、`git-reference-tree.tsx`、`git-commit-table.tsx`、`git-graph-row.tsx`…） |
| 变更/终端/调试/搜索/AI 面板 | `features/git/components/*`、`features/terminal*`、`features/run*`/`debug*`、`features/search*`、`features/ai*` |
| 颜色与尺寸 | `windows/tauri/src/styles/theme.css`（CSS 变量）、`extensions/themes/builtin/lithe.json`（39 色） |
| 中文文案（逐字采用） | `windows/tauri/src/i18n/locale.ts` |
| 主题 token 映射 | `gpui/research/windows/04-theme-and-components.md` |
| 运行中的真机（并排对照） | 本机已安装的 `D:\Programs\Lithe\lithe-windows.exe`；仓库截图 `docs/assets/screenshots/windows-*.png` |

**换算约定**：Tailwind 是 CSS-first（没有 `tailwind.config.*`），**1 单位 = 4px**；`--radius: 8px`（`rounded-sm/md/xl` = 4.8/6.4/11.2px）；
标题栏 40px、状态栏 24px、活动栏折叠 38px、chrome 控件 24px、工作区间隔 4px（`theme.css:118-134`）。

## 2. 已拍板的决策（不要再问，直接按此执行）

| # | 决策 |
| --- | --- |
| 落点 | **在原 `gpui/` 工程里清空重写**：保留 Cargo workspace 与 bin 名 `shell-probe`；旧文件删除（要回看就查 git 历史） |
| 精度 | **混合**：布局与关键尺寸**逐值搬**（Tailwind→px、CSS 变量→`px()`）；组件内部（按钮/输入框高度）用 gpui-kit 的 `Size` 档位，不硬压数值 |
| 第一版范围 | 外壳 + **最小真实数据**（`workspace.snapshot`、`git.status`、`git.historyPage`）**+ 终端外壳**（见下）；拖拽分栏/右键菜单/命令面板等交互按区域顺次做 |
| 终端 | **直接调本地 `cmd` 与 `powershell`**（Rust 宿主 spawn 进程），**第一版只做外壳**：工具窗页签（新建/关闭/标题）+ 流式显示 stdout/stderr + 一个发送命令的输入行；**不做完整 VT 模拟**（不做 ANSI 全量渲染/光标定位/选区）。要预留"自定义 shell"（配置 shell 路径/参数）的位置 |
| 调试 | 同终端口径：**先做界面骨架**（工具窗/工具栏/空态），真实 DAP 接线排后 |
| 数据库 | **不做**（本轮范围外） |
| 数据来源 | 宿主**直连 `lithe-core`**（现有路径依赖已验证）；功能面照 **Core 的数据语义**（命令名/字段/事件）实现，**不要**照抄 `windows/tauri/src-tauri` 的 Tauri command 封装 |
| 窗口三键 | **自绘**：56×40 直角 + `WindowControlArea`（gpui-kit 的 `TitleBar` 高度硬编码 34px、控件宽 34px 不可定制，用 `.h(px(40.))` 覆盖高度后自绘三键） |
| 玻璃层 | Windows 端是 `color-mix` 半透明 + **`backdrop-filter: none`**（没有模糊）→ gpui 用**带 alpha 的颜色**直接画即可，**可一比一** |
| 不做 | `useFooterDebuggerItem` / `FooterControlBadge`（死代码）、`toggle_menu_bar` / `uses_native_window_chrome`（Rust 侧无实现） |

## 3. 上一轮踩过的坑（违反必踩，全部有实测证据）

1. **根视图用 `.size_full()`**；`viewport_size()` **不要**除 `scale_factor()`（那会让内容只画到窗口 **78%**）。
2. **窗口口径**与同目录参考应用 Dodona 一致：`visible_bounds() * 0.94`、夹 `1024×680`、居中、`WindowBounds::Windowed`
   （`Dodona/crates/dodona/src/main.rs:91-108`）；`WindowBounds::Maximized` 在 0.6.6 上不会真的最大化。
3. **截图必须 DPI 感知**：用 `pwsh -File gpui/capture-screenshot.ps1 -ProcessName shell-probe [-WholeScreen]`
   （脚本已 `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)`）。DPI 不感知的抓法只会拿到虚拟化尺寸（1458×819 vs 真实 1823×1024），
   **证据会缺一大截**，上一轮因此把"内容只占 78%"误判成"已铺满"。
4. **行高**：树的 `uniform_list` 用**第 0 行内容的测量高度**当所有行行高；`ListItem` 默认 `py_1()` 会溢出 → 紧凑行 `py_0()`，
   且**一行里的多个元素必须自己套 `h_flex()`**（`ListItem` 的 children 是竖排的）。
5. **多行输入/编辑器必须显式给高度**：`Editor::new(&state).h(relative(1.))`；编辑器**不吃滚轮**，要在包裹层 `on_scroll_wheel` → `set_scroll_offset`。
6. **滚动条常显**：`Theme::set_scrollbar_mode(ScrollbarMode::Always, cx)`。
7. **浮层**：`Root` 不自带 Dialog/Sheet/Notification 三层，必须业务视图自己 `.children(Root::render_*_layer(window, cx))`；
   `open_dialog`/`Root::read` 在窗口根还不是 `Root` 时（首帧之前）会 **panic**。
8. **图标**：应用注册 `gpui_kit::assets::Assets`，只含 `gpui-kit-assets-0.6.6/default-icons.txt` 的 101 个字形；缺字形用最接近的代替并登记。
9. **只用 0.6.6 真实 API**（在线文档是未发布的 0.7.0）：`Button::{ghost,selected,disabled,xsmall}`、`Icon::small()/size_3()/size_4()`
   存在但**在 trait 上，必须 `use ... as _`**。
10. **不要**把 `uniform_list` 直接塞进 `h_resizable` 的栏里（实测会拿到 0 高度视口、整块不渲染）。
11. **同一个问题最多试 5 次**（`cargo check` 每次算一次），超了把现象写进注释并在文档登记。

## 4. 硬性约束

- 允许改：`gpui/**`；写文档/记决策时改 `docs/**`、`.agents/notes/**`。
- **不要改** `macos/`、`windows/`、`rust/`、`shared/`、`Plugins/`（现有产品与跨端契约，只读参考）。
- 不改 Core 的命令名/字段/事件（契约真源 `shared/contracts/`，要改先汇报）。
- **不写平台业务分支**：`#[cfg(target_os)]` 不进业务代码（平台差异放平台层）。
- 颜色一律 `cx.theme()`；中文文案与 `i18n/locale.ts` **逐字一致**。
- 一个区域一个模块（`gpui/shell/src/bin/shell_probe/` 下每区域一个文件）。
- 验证：`cd gpui && cargo check --bin shell-probe` **0 error 0 warning**；每阶段留证据（DPI 感知截图 + 日志）到 `.artifacts/`。
- 收尾跑 `node scripts/verify-agent-notes.mjs`。
- 开发流程遵循 `AGENTS.md`：分支从最新 `preview` 开、PR 用 `gh`、描述逐条写清改动。

## 5. 分阶段（每阶段独立验收，做完停下汇报）

- **阶段 0**：清空 `gpui/shell/src/bin/shell_probe/` 下的旧实现（保留 `Cargo.toml` 与 bin 名），确认 `cargo check` 仍可跑（先搭最小骨架）。
- **阶段 1**：外壳 —— 标题栏（40px，自绘三键）+ 项目标签条 + 左右活动栏（38px）+ 状态栏（24px）+ Dock 三栏与间隔（4px）。
- **阶段 2**：侧栏（项目树：行高/缩进/图标/选中/悬停/右键菜单）+ 全局面包屑。
- **阶段 3**：编辑区（内部标签栏、`← →`、文件类型图标、脏标记、空状态、真实文件内容）。
- **阶段 4**：底部 Git 工具窗（标题栏 + 页签 + 三栏 + 提交表 + 引用树 + 筛选条），接真实 `git.*` 数据。
- **阶段 5**：终端外壳（页签 + 输出流 + 发送命令行；Rust 侧 spawn `cmd`/`powershell`）。
- **阶段 6**：右侧工具窗 + 浮层（命令面板用 `Command` + `window.open_dialog`；注意先挂 `Root` 三层）。
- **阶段 7**：调试界面骨架 + 逐区域与 Windows 真机并排复核。

## 6. 验收标准（逐条勾）

- [ ] 与 Windows 前端**并排对照**：外壳/侧栏/编辑区/底部 Git 窗/状态栏的**尺寸与结构一致**（逐项对照表 + 截图）；
- [ ] 中文文案与 `i18n/locale.ts` 逐字一致；
- [ ] 真实数据接线：`workspace.snapshot`（项目树）、`git.status`/`git.historyPage`/`git.commitFiles`（Git 窗）；
- [ ] 终端能起 `cmd` 与 `powershell`，能发命令、能看到输出流；
- [ ] 没有裸色值、没有默认图标集之外的字形、没有平台业务分支；
- [ ] `cargo check --bin shell-probe` 0 error 0 warning；
- [ ] 证据（DPI 感知窗口帧 + 整屏 + 与真机并排）落到 `.artifacts/`，`gpui/PLAN.md` 当前状态同步；
- [ ] `node scripts/verify-agent-notes.mjs` 通过。

## 7. 起步三步

1. 读 `gpui/UI-MAP.md` §1 与 `gpui/research/windows/01..06-*.md`，把"外壳"这一块的**数值清单**抄成一页（标题栏/活动栏/状态栏/分栏/圆角/颜色）。
2. 阶段 0 + 阶段 1：清空旧实现 → 搭出外壳骨架 → `cargo check` → **DPI 感知截图** → 与 `D:\Programs\Lithe\lithe-windows.exe` 并排比对，差异写进文档。
3. 每个阶段重复第 2 步的"改 → 构建 → 截图 → 与真机对照 → 记录差异"循环。

---

### 报告要求

- 每阶段**停下来汇报**：改了什么、证据在哪、差异清单、下一阶段的输入；
- 需要改契约/Core 的发现，**先汇报再动手**；
- 不要复述本提示词，只写结论、证据与下一步。
