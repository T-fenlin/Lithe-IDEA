# Agent 笔记：外壳的三个工具窗改用组件库自己的 Dock

状态：已实现

## 先说结论

外壳（`rust/lithe-gpui/crates/workbench` 的 `ShellWorkspace`）里左、右、下三块
工具窗的**框架、分隔条、拖动和尺寸钳制**，从 gpui-kit 的 `resize_handle`
换成组件库自己的 `lithe_gpui::layout::Dock`。另外，组件库有**自己的主题全局**，
和宿主用的 gpui-kit 主题不是同一份数据，所以外壳启动时要装一次组件库主题、
并把宿主的颜色映射过去。

开发者要注意三点：**别再往外壳里写拖动/钳制逻辑**，尺寸与分隔条归 Dock；
工具窗的"显示什么、开不开"仍归外壳（`panel_content` / `sync_dock`）；
改工具窗的上下限，改的是外壳喂给 Dock 的 `SideLimits`，不是 Dock 里的常数。

## 问题

三块工具窗原先各自手搓：一块 `div` 装内容，挂在它下面的 `resize_handle`
（gpui-kit）负责拖，外壳自己算钳制、自己维护拖动会话（`begin/drag/end_pane_resize`
+ `PaneResizeDrag` + `drag_tracker` 用 `canvas` 监听鼠标移动）、自己落盘。

这套东西有几个问题：

- **重复一套已经有的机制**。组件库的 Dock 已经有分隔条、拖动、每边上下限、
  同侧两组、最大化、浮动、布局存取；外壳又写了一份，两边迟早行为不一致。
- **钳制散落在外壳**。最小值、最大值、"给中央列留多少"三组数写在外壳的三个
  常量函数里，和真正画分隔条的组件不在一起。
- **主题是两套**。组件库读 `lithe_gpui::theme::Theme`，宿主读 gpui-kit 的
  `Theme`。不改的话，工具窗的颜色会和外壳其余部分对不上——而且不装组件库
  主题全局，组件一渲染就 panic。

## 决策

**工具窗的"框"归 Dock，"内容与开关"归外壳。**

- 外壳建一个 `Entity<Dock>`（`dock_with_panels`），把三块窗格注册成三个面板
  （`left` / `right` / `bottom`），都用 `DockPanel::plain`——外壳的活动栏和
  面板自己的标题已经说明了它们是什么，所以 Dock **不画标签头**，分隔条就是
  唯一的把手。
- 面板内容由一个回调产出（`ShellWorkspace::panel_content`）：外壳仍然决定
  左边显示文件树/搜索/改动、底部显示终端/Git/问题/运行/调试。Dock 只负责
  把这棵元素放到正确的位置上。
- 可见性由外壳的开关推给 Dock：`sync_dock` 每帧跑一次，把
  `left_sidebar_visible && left_pane_visible`、`right_visible`、
  `bottom_visible && 非自隐藏` 三个结论写进 Dock 的 show/hide。写进去之前先
  比一次，值没变就什么都不做，所以不会每帧重绘。**这样任何改动开关的路径都
  不用记得通知 Dock**——包括菜单、活动栏、会话恢复。
- 尺寸是**单向**的：人拖分隔条 → Dock 发 `DockEvent::Resized` → 外壳记下这个
  像素宽并走去抖的会话保存；恢复会话时反过来，外壳把存量值交给 Dock 夹取，
  再读回夹取后的值。两边不会来回覆盖。
- 上下限写成 Dock 的 `SideLimits { min, max, reserve }`：`reserve` 是"这一边
  不许吃掉、要留给中央列的空间"，于是 macOS 真源里那三条"窗口宽 − 某值"
  的公式变成了同一个机制（见 `UI-MAP.md` 的三块窗格一节）。

**组件库主题由宿主显式装上并跟随。**

- `lithe_gpui::theme::Theme::init(cx)` 装全局，`Theme::apply(mode, palette, cx)`
  一次性设定模式与调色板。
- 组合根（`crates/app`）启动时装一次，并 `observe_global` gpui-kit 的主题：
  宿主换主题 → 重新映射一次颜色（`crates/workbench/src/component_theme.rs`）。
- 映射是**保守**的：只覆盖组件真正读、宿主也确实有的语义色（背景、边框、
  前后景、强调、焦点环、成功/警告/危险/信息等），渐变、图表、ANSI、语法色
  继续用组件库自己的调色板。

## 考虑过的备选方案

### 只换分隔条，保留外壳自己的布局与钳制

把 gpui-kit 的 `resize_handle` 换成一个自绘的手柄，其余照旧。改动小，但等于
**把同一套拖动/钳制再留一份在外壳里**，Dock 的同侧两组、最大化、浮动、布局
存取都用不上，两边仍然会漂移。否掉了。

### 让三块工具窗都变成 Dock 的标签页

底部那排"终端/Git/问题/运行/调试"改为 Dock 的一组标签，由 Dock 管切换。
更接近 IntelliJ，但会牵动外壳的 `bottom_kind`、活动栏高亮、会话字段与相关
测试，属于**行为变化**而不是这次要解决的"机制重复"。留待以后按需再做。

### 让组件库直接读宿主的主题

组件库要 `use gpui_kit` 才能拿到宿主主题，等于让组件库反向依赖宿主的组件栈
（还会固定到某一个主题实现）。反过来由宿主把颜色映射进组件库，组件库保持
对 gpui-pre 的单一依赖。

## 后果

- 收益：外壳少了约 260 行手搓的拖动/钳制代码和三条钳制公式；分隔条、拖动、
  尺寸行为在画廊与真机上是**同一份实现**；Dock 的额外能力（同侧两组、最大化、
  浮动、布局序列化）以后可以直接用。
- 代价：外壳与组件库之间多了一条依赖（`workbench → lithe-gpui`），并且多了
  一层主题映射需要维护——宿主的主题字段一变，`component_theme.rs` 要跟着改。
- 代价：`S1_SPLIT_CONTAINER` 这个诊断锚点没了（分栏容器不再由外壳测量）。
  核对拖动结果仍可 grep `S1_PANE_RESIZE`（现在由 Dock 的事件打印）。
- 需要重新评估的情况：底部要按 IntelliJ 那样做成一排标签、或者外壳需要
  Dock 之外的新尺寸语义时，再回来动这里。

## 验证

- `cargo test --manifest-path rust/Cargo.toml -p lithe-gpui-workbench`：
  238 个单元测试 + 集成测试（含 `tests/ui.rs`）。
- `cargo test --manifest-path rust/Cargo.toml -p lithe-gpui --features test-support`：
  组件库自身 1536 个测试，其中 Dock 的上下限、`reserve`、尺寸读写、事件、
  无标签头面板都有测试。
- 真机启动：`./rust/target/debug/Lithe.exe <项目根>`，确认无 panic，日志里
  `S1_SESSION_SIZES` 的宽度经过 Dock 夹取后回写、`S1_PANE_RESIZE` 在拖动时打印。
- `./scripts/verify-agent-notes.sh`

## 适用范围

- `rust/lithe-gpui/crates/workbench/src/workspace.rs`（`dock_with_panels`、
  `sync_dock`、`panel_content`、`on_dock_event`）
- `rust/lithe-gpui/crates/workbench/src/component_theme.rs`
- `rust/lithe-gpui/crates/app/src/main.rs`（启动时装组件库主题）
- `rust/lithe-gpui/crates/lithe-gpui/src/layout/dock/`（`SideLimits::reserve`、
  `Dock::size/set_size/set_side_limits_with`、`DockEvent`、`DockPanel::plain`）
- `rust/lithe-gpui/UI-MAP.md`（三块工具窗与活动栏一节）
