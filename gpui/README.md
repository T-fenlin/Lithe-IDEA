# `gpui/` — GPUI Kit 宿主

> **先读 [`UI-MAP.md`](./UI-MAP.md)**（界面规格：macOS 界面逐区域 → gpui-kit 组件的对应表）
> 与 [`PLAN.md`](./PLAN.md)（执行计划：下一步做什么、怎么算做完、当前差距）。
> 本文件只讲**怎么跑**、**已经验证过的接口事实**和**硬约束**。

这个目录是「用 GPUI Kit 统一三端界面」方向的落地起点。它**独立于现有产品**：不修改 `macos/`、`windows/`、`rust/`、`shared/`、`Plugins/` 的任何现有文件。

决策依据见
[`.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md`](../.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md)。

## 跑起来

```powershell
cd gpui
cargo build --bin shell-probe
.\target\debug\shell-probe.exe D:\developmentProjects\rust\Lithe-IDEA     # 参数 = 工作区根
```

- 窗口尺寸从主显示器的可见区域算（`startup_window_bounds`），**不写死机器路径**；
- 界面数据来自 Rust Core（`core.ping` / `workspace.snapshot`），跑在后台线程，窗口期间保持可交互；
- 截图证据（只抓目标窗口，不抓整个桌面）：

  ```powershell
  pwsh -File gpui\capture-screenshot.ps1 -OutputPath .artifacts\p1\shot.png -ProcessName shell-probe
  ```

## 目录结构

| 文件 | 职责 |
| --- | --- |
| `shell/src/bin/shell-probe.rs` | crate 根：`mod shell_probe; pub use shell_probe::main;` |
| `shell_probe/mod.rs` | 入口：参数、首窗口尺寸、启动顺序、主题（深色） |
| `shell_probe/workspace.rs` | 工作台骨架：标题栏 + 项目标签条 + 活动栏 + Dock 组装 + 状态栏 |
| `shell_probe/panels.rs` | `ShellPanel`（文件树 / 编辑区 / 大纲 / 统计 / 命令面板）与 `PanelKind` |
| `shell_probe/bottom_panel.rs` | 底部 Git 工具窗（`提交记录`），自带头部与页签行 |
| `shell_probe/files.rs` | 文件域：`workspace.snapshot` 调用 + 两层树构建 |
| `shell_probe/stats.rs` | 右侧统计表（`StatRow` / `StatsDelegate`） |

## 已经验证过的接口事实（0.6.6）

判断依据一律是**已发布源码**：`D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-{kit,component,base}-0.6.6\src\`。

### 启动顺序（0.6.6 只有这一种写法）

```rust
gpui_kit::application().with_assets(gpui_kit::assets::Assets).run(move |cx| {
    gpui_kit::init(cx);
    Theme::change(ThemeMode::Dark, None, cx);          // 真机默认深色；init 默认浅色
    cx.spawn(async move |cx| {
        cx.open_window(WindowOptions { ..TitleBar::window_options() }, move |window, cx| {
            let view = cx.new(|cx| AppView::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))    // Root 必须在最外层
        }).expect("failed to open window");
    }).detach();
});
```

`gpui_kit::open_window`（技能文档里的 0.7.0 写法）**不存在**；`Registry` 里没有。`Application::with_assets` 存在（`gpui-pre-0.3.6/src/app.rs:199`）。

### 尺寸

**根视图用 `.size_full()`，不要自己换算缩放比**：

```rust
// ✅ 正确：根视图 100% 父容器，resize 自动跟随（与参考应用 Dodona 一致）
div().size_full() /* ... */

// ❌ 错误（本项目踩过）：viewport_size() 已经是 DPI 无关单位，再除 scale_factor()
//    会让内容只画到窗口的 78%，右侧/底部留白
// let v = window.viewport_size();
// let logical = v.height / window.scale_factor();
```

单位对账：真屏 1920×1080 物理 + 125% 缩放时，窗口是 1823×1024 物理，而 `viewport_size()` 报 1444×812、`scale_factor()` 报 1.25 —— **布局单位就用 `viewport_size()` 那一套，不乘不除**。

截图必须用 **DPI 感知**的 `capture-screenshot.ps1`（脚本里已 `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)`，并支持 `-WholeScreen`）；DPI 不感知的抓法只会拿到虚拟化尺寸（1458×819），**证据会缺一大截**，曾因此把"内容只占 78%"误判成"已经铺满"。

不清算会让根视图高 1/scale，**底部状态栏被切出窗口**（本项目反复踩过）。

### Dock / 面板

```rust
let (area, skin) = DockSkin::dock_area("lithe-workbench", Some(1), window, cx);
let layout = DockLayout::h_split()
    .child(DockLayout::tabs().panel_view(panel_handle(panel), cx), Some(px(320.)))
    .child(DockLayout::v_split().child(a, None).child(b, Some(px(260.))), None);
area.update(cx, |area, cx| area.set_center(layout, window, cx));
```

- 面板 = `impl BasePanel`（`panel_name`）+ `impl Panel`（`title`）+ `Focusable` + `EventEmitter<PanelEvent>` + `Render`；
- `Panel::title_bar()` / `inner_padding()` 返回 `false` → Dock 不再画自己的标题栏/内边距，面板可以自绘头部；
- DockArea 的 `DockPlacement::Bottom` **只横跨中心列**（`gpui-base-0.6.6/src/dock/dock_area.rs:1415-1431`），要"底窗横跨左栏 + 编辑区"就把底部放进 center 的 `v_split`（本项目采用）。

### 组件要点（踩过的坑）

| 事实 | 出处 |
| --- | --- |
| `ListItem` 的 children **是竖排**（内部是普通块级 `div`），多个子元素必须自己套 `h_flex()` | `gpui-component-0.6.6/src/list/list_item.rs:215-221` |
| `uniform_list` 的行高 = **第 0 行内容的测量高度**，`.h()` 压不住内容，行内纵向 padding 会溢出到相邻行 | `gpui-pre-0.3.6/src/elements/uniform_list.rs:359-371,508-509` |
| `ListItem` 默认 `py_1()/px_3()`；紧凑行要 `py_0()` | `list/list_item.rs:186-187` |
| `DataTable` 行高是表级值（默认 `Size::Medium ⇒ 32`），但表体行高同样受首行内容影响 | `sizing.rs:57-65` |
| `render_td` 的返回值会被再包一层（固定列宽 + `overflow_hidden` + `whitespace_nowrap`） | `table/state.rs:1323-1329` |
| `Column` 只有 `width/min_width/max_width`，**没有 flex**；列宽按给定值比例铺满表宽 | `table/column.rs:151-215` |
| `TitleBar` / `StatusBar` 都实现 `Styled`，macOS 的 40 / 24 用 `.h(px(..))` 覆盖（默认 34 / 自定） | `title_bar.rs:15,335,343` |
| `Tab::icon()` 会丢掉 label，图标 + 文字要走 `Tab::child(..)`；`TabBar::max_width()` 是文字能截断的前提 | `tab/tab.rs:719-744` |
| `Button::ghost()/disabled()/xsmall()` 存在，但分别在 `ButtonVariants`/`Disableable`/`Sizable` 三个 trait 上，**必须 `use ... as _`** | `button/button.rs:44-96,503-537` |
| `IconName` 是 `Clone` 不是 `Copy` | `component/icon.rs:17-20` |
| 图标只能取默认图标集（`gpui-kit-assets-0.6.6/default-icons.txt` 的 101 个字形），否则运行时画空白 | 同左 |

### 编辑区不需要 WebView

macOS 端编辑内容是 WKWebView 里的 Monaco（`Platform/MacOS/MonacoWorkbenchEditor.swift`），而 gpui-kit 自带代码编辑器引擎：`component::input::EditorState`（`= gpui-base/src/input/editor/mod.rs:11`）已有行号列、折叠、缩进线、tree-sitter 高亮、诊断波浪线、装饰、多光标、撤销栈、内置查找替换（含 `2/3` 计数），`InputEditorStyle::{editor_active_line, editor_gutter_background}` 可配。缺 minimap、overview ruler、sticky scroll、gutter 装饰渲染器、code lens、LSP 客户端。

### gpui-kit 没有的能力（要自研）

终端模拟器 / PTY / ANSI 解析（`gpui-component` 内零命中）、提交图泳道、并排 diff、表格与列表的**多选**、面包屑之外的大多数 IDE 专属装饰。

## 约束

- **只以 `gpui-kit` 为准**。`gpui-elements`、`gpui-macros`（第三方）明确禁用：它们教的是裸 GPUI 写法，会跳过 `gpui-kit` 的 `Root` 挂载，浮层与弹窗失效。
- **依赖精确钉版本**（`gpui-kit = "=0.6.6"`）并提交 `Cargo.lock`。上游窗口创建 API 近期变更过，宽松版本范围会让构建结果漂移。
- **不写平台专属分支**：目标是一套代码三端用，`#[cfg]` 不进业务逻辑。
- **架构观察**：编辑器内容宿主、终端、Core 状态机都在演化，任何"临时实现"都要在注释里写清归属与后续接线点。

## 验证状态

| 平台 | 状态 |
| --- | --- |
| Windows | 可用：构建通过、真实 Core 数据、界面按 `UI-MAP.md` 逐区域对齐中；证据在 `.artifacts/p1/`。 |
| macOS / Linux | **未验证**：本方向当前只在 Windows 上验证过；三端构建与 CI 属路线图 P6。**不要声称 macOS 已验证。** |

`.artifacts/` 已被仓库根 `.gitignore` 忽略；构建日志、截图、Core 原始响应都写在那里。
