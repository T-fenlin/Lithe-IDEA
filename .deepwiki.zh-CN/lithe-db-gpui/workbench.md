# Workbench

`rust/lithe-db-gpui/crates/workbench/` 是 shell：15 个模块，`workspace.rs` 单文件约 287 kB。它的公开表面非常小——只有 `ShellWorkspace` 是 public，另一个 `right_tool_window::RightToolWindowView` 仅通过模块路径导出，因为 `app/src/main.rs:48-50` 需要它来命名。区域模块只暴露 render 函数（`lib.rs:104-108`）。

## Layout

规范的 tree 在 `workspace.rs:1-18`：

```
v_flex
├─ title bar row 1                30    menu bar
├─ title bar row 2                38    project dropdown / branch / run / window controls
├─ project tab strip              32
├─ workspace h_flex flex-1 gap 4
│  ├─ left activity rail        38+4
│  ├─ left side pane           320    (runtime-draggable; SIDEBAR_WIDTH_SPEC)
│  ├─ central column            flex-1 = v_flex[ editor island, bottom pane 240 ]
│  ├─ right tool window         400    (collapsible)
│  └─ right activity rail      38+4
└─ status bar                     24
```

实现位于 `ShellWorkspace::render`（`workspace.rs:3834-3981`）。区域所有者：

| 区域 | 渲染函数 | 位置 |
| --- | --- | --- |
| Title bar（两行） | `title_bar::title_bar` | `:3869-3875` |
| Menu bar（两种样式） | `menu_bar::render_bar` | `:3550` |
| Project dropdown / branch trigger | `render_project_menu` / `branch_trigger` | `:3554-3565` |
| Project tab strip | `project_tabs::project_tabs` | `:3881-3890` |
| Workspace config 错误 banner | `render_workspace_config_error` | `:3321`, `:3896` |
| 左/右 rail | `activity_bar(ActivitySide::.., ..)` | `:3747-3768` |
| 左 pane + drag handle | `side_pane` / `drag_handle` | `:3999`, `:4084` |
| Editor island / bottom pane | `editor_island` / `bottom_pane` | `:4175`, `:4197` |
| Right tool window | `right_tool_window` | `:3781-3788` |
| Status bar | `status_bar::status_bar` | `:3971-3977` |
| Dialog / sheet / notification layers | gpui-kit `Root::render_*_layer` | `:3530-3534`, attached `:3979-3981` |

`Root` 的三层 overlay **必须**挂到最外层 view，否则对话框、sheet 和通知会静默不显示。它们在最前计算，因为它们借用 `cx`，之后所有内容都需要 `cx`。

## 模块地图

| 文件 | 职责 |
| --- | --- |
| `workspace.rs` | `ShellWorkspace`（结构体 `:787-1061`, `new` `:1071-1600`, `Render` `:3528-3985`），`ShellStartup`，活动项索引，metric 常量，lazy-scan state，以及约 25 个单测 |
| `title_bar.rs` | 两行 title bar，拖拽区和手绘 56×38 窗口控制 |
| `project_tabs.rs` | 无状态 project tab strip；`ProjectTab` 是纯值对象，仅持有 `name` |
| `activity_bar.rs` | 无状态左右 icon rail；`ActivityItem`、`ActivityIcon`、`ActivitySide` |
| `status_bar.rs` | 无状态 status bar；`StatusEntry` |
| `right_tool_window.rs` | `RightToolWindowView` enum、`resolve_click`、`right_tool_window`、`diagnose` |
| `menu_bar.rs` | 9 个顶层菜单 / 89 项，两种视觉形式，`MenuAction`、`MenuRequest` queue，以及 thread-local handles |
| `command_palette.rs` | `Ctrl+Shift+P` overlay，基于 `Dialog` + `Command` |
| `project_menu.rs` | 标题栏项目下拉（3 段，2 个分隔线） |
| `branch_panel.rs` | 分支触发器 + 弹窗（只读 v1） |
| `session.rs` | `.lithe/session.local.json` 的时间层：`SessionTracker`、`SessionPlan`，300 ms debounce |
| `notifications.rs` | `NotificationPanel` entity：搜索、严重程度过滤、可展开行、clear-all |
| `maven.rs` / `spring.rs` | 右侧窗口**数据层**（`maven.scan` / `spring.index` -> view models） |
| `spring_paths.rs` | `spring.index` 的有界输入列表（`MAX_SPRING_PATHS = 5_000`, `MAX_SCANNED_ENTRIES = 200_000`） |

## 子面板如何托管

四个面板都是长生命周期 `Entity`，作为 `ShellWorkspace` 字段存在，并按此顺序在 `new()` 中构造（`:1130-1226`）——顺序本身很关键：

1. `Entity<notify::Store>` —— 最先创建，因为 editor 会拿它的 `WeakEntity`。
2. `Entity<EditorPane>` —— 然后立刻调用 `pane.prepare_java(root)`。
3. `Entity<Explorer>` —— 用 `on_open: Box<dyn Fn(PathBuf, &mut Window, &mut App)>` 闭包 editor 的 `downgrade()`。**这就是 shell 把 explorer 和 editor 解耦的方式**（`:1144-1158`）。
4. `Entity<ChangesView>` —— 构造时不拉取，首次激活时才读取。
5. `Entity<TerminalPane>` —— 懒加载，首次可见才建立会话。
6. `Entity<BottomPane>` —— Git log。

随后在 `:1172-1224` 把 callback 注入 editor（`TabMenuHostActions { reveal, open_terminal }`，都是 `Arc<dyn Fn(..)>`）。`reveal` 会调用 `explorer.exe /select,<path>` -> `open -R` -> `xdg-open`，采用**运行时探测**而非 `#[cfg]`，因此 editor 和 explorer 都不需要平台层。顺序约束写在 `:1175-1178`：这个必须在 terminal 存在之后运行，因为 `open_terminal` 闭包引用 `terminal.downgrade()`。

## 面板状态机

### Bottom pane —— 可见性与 kind 分离

`BottomPaneKind { Terminal, Git, Run, Diagnostics }`（`:520-529`）使用**手写的稳定 string ids**而不是 `serde` derive，因此重命名 Rust variant 不会悄悄污染磁盘值（`:532-545`）。`bottom_kind_from_id` 对未知 id 返回 `None`，而不是默认到 Terminal（`:562-574`）。

可见性是独立的 flag，因此折叠后仍保留上一次 tab（`:510-512`）。还有一个细节：Git 工具窗口自己的 header 有 Hide 按钮，会调用 `BottomPane::set_visible(false)`，所以 shell 需要 OR `!bottom_git.visible()` 到 `Git` 判断；不这样会出现空框架（`:3799-3802`）。`TerminalPaneEvent::LastTabClosed` 会自动折叠底部 pane（`:1398-1407`）。

### 左侧 side pane —— 两种状态，IDEA 语义

`left_sidebar_visible`（整个 rail + pane，`Ctrl+B`）和 `left_pane_visible`（仅 pane，可通过点击已高亮顶部分组切换）是独立的（`workspace.rs:820-834`, `:1835-1846`, `:1926-1936`）。内容是两个 view 共用一个 slot 的 switch；两者都无条件构造并保留着状态，因此切换不会丢失 scroll 和展开态（`:3772-3780`）。

### 右侧 tool window —— 两种独立状态

`right_view` 和 `right_visible`，对齐源产品的 `isRightSidebarVisible` + `activeRightSidebarView`。转换是纯函数 `resolve_click(clicked, current, visible) -> (view, visible)`（`:219`），因此可做单测。

### Activity bar 选择契约

`activity_bar` 接收 `is_active: impl Fn(usize) -> bool`——这是**按项谓词**，而不是单一 `Option<usize>`，因为左 rail 的顶部和底部分组可能同时高亮，而右 rail 是第三套独立集合（`activity_bar.rs:61-67`）。谓词会先 snapshot 成 `Vec<bool>`，再传给 `'static` 闭包（`:3738-3746`）。

## 状态与数据流

这个 crate 没有每个 feature 自己的 `*Store`。模式是：

- 状态位于视图自己的 `Entity<T>` 中。
- 跨功能读取通过 `Global` 持有的 handle（settings）或 `WeakEntity` 注入（notify store）。
- `cx.observe` / `cx.subscribe_in` 结果必须**存进 struct 字段**，因为 GPUI 的 `Subscription` 是 RAII——drop 即取消订阅（`:917-930, 1058-1059`）。
- `Task`s 也必须保留（`session_save_task: Option<Task<()>>`，`:1050-1055`）。
- 区域 render 函数都**无状态**：它们接收数据和回调。

Settings -> 子 pane 值转发（`:1328-1392`）是典型例子：子 crate 不知道 settings crate，所以 shell 读取 store 并推进值——`EditorPane::set_tab_size` / `set_auto_completion`、`TerminalPane::set_default_shell` / `set_font_size`、`ChangesView::set_confirm_before_discard`，以及 `register_java_toolchain`。`S1_SETTINGS wiring=workbench …` 行（`:1342-1349`）是这段读回证据。

跨 crate 枚举通过本地 trait `IntoGitIdentity` 映射，因为 `impl From<External> for External` 会违反 orphan rule（`:284-323`）。

因为这个 crate 没有 `TestAppContext` / `#[gpui::test]`，所有能做 pure 的决策都被做成纯函数并单测（`workspace.rs:4625-5100`）：`resolve_project_open_destination`、`left_activity_index`、`visible_commands`、`command_action`、`is_activity_active`、`RightScanState::request`、`right_scan_should_notify`、`clamped_drag_width`、`bottom_kind_from_id`、`java_toolchain_override_from`、`maven_settings_override_from`、`terminal_font_size_override`。

## 多项目与项目切换

**同一时刻只允许一个项目。** `projects: Vec<ProjectTab>` 由 `vec![ProjectTab::new(name)]` 构造（`:1476`）——始终正好一项——并且当 `projects.len() <= 1` 时整条 strip 被隐藏（`:3881-3890`）。`ProjectTab` 只持有 `name`，真正的记录路径是单一 `ShellWorkspace.root`（`:795`）。关闭 tab 被明确未实现（`:3615-3616`）。

因此“在此窗口中打开”本质上就是**重建整个 shell**，而不是重置小部件（`:3010-3083`）。顺序：先 `flush_session(true, cx)`，再 `window.replace_root(cx, ..)` 传入全新 `ShellWorkspace` + `Root`，等成功后才写入 recents 和首选项键。发布链是级联 drop，没有显式 shutdown；唯一锚点日志是 `impl Drop for ShellWorkspace { eprintln!("S1_WORKSPACE_DROP root=…") }`（`:3458-3462`）。

两个关键证据解释为什么必须 full rebuild：`EditorPane::prepare_java` 是幂等 early-return，所以编辑器面板永远无法切换 JDTLS 工作区；而 root 被 6 个 entity 和 1 个跨 crate hook 绑定，不存在 `set_root` 入口。

**多窗口未实现也不伪造。** `OpenDestination::NewWindow` 会直接走 `report_new_window_not_wired`（`:2920-2942`），打印 `S1_OPEN_PROJECT destination=new-window state=not_wired missing=window_handle_routing`。其原理写在 `:2931-2936`：`menu_bar`、`project_menu`、`command_palette` 和 Git-identity host 都是**进程级 thread_local 单例**；第二个 shell 会覆盖第一个的 handles。构造 `ShellWorkspace` 的只有两处：`app/src/main.rs:1030` 和 `workspace.rs:3042`（` :4425-4428` 说明）。

## 编辑器标签页与 session 持久化

这一切由 `EditorPane` 完成（见 [Editor](editor.zh-CN.md)）。时间层由 shell 负责：`SessionTracker`（`session.rs:77-100`）带 `SESSION_SAVE_DEBOUNCE_MS = 300`，并提供三条 flush 保证——内容变更后 300 ms debounce，项目切换前同步 flush，应用退出时同步 flush（`:13-19`）。它复用了 `lithe_db_gpui_settings::DebounceState`，这是纯状态机，因此测试不用 sleep。保留 `Task` 槽位 + generation counter 可以保证“很多点击，只写一次”（`:1764-1777`）。

`restore_session` 在 `new()` 末尾执行，位于 `--right-view` / `--left-view` 之后，因此显式 CLI 意图会覆盖恢复的 session（`:1580-1584`）。

## 工作区身份

`prepare_workspace_config`（`:4455-4510`）在后台执行：
`workspace_config::resolve_project_id` 创建或读取 `.lithe/project.json`（稳定 UUID v4）和两个本地 toolchain 文件。它还把 overlay 的“是否被 Git 跟踪”标志推到 settings store，以便 UI 区分“团队设置”和“这个项目的设置”。

失败时它设置 `workspace_config_error` -> 一个**持久的、可手动关闭**的红色 banner（`:3321-3354`），而不是 toast：双击用户一般看不到 stderr，而隐藏的失败就是静默失败（`:4448-4454`）。

## 值得注意的工程约束

**Edition-2024 RPIT hazard**（`lib.rs:110-121`）：edition 2024 的 return-position `impl Trait` 会把作用域内所有生命周期都 capture，因此 `&mut Window` / `&mut App` 参数会被埋进返回的 `impl IntoElement`，产生 E0499/E0502（首构建出现 12 处）。规则很明确：每个区域 render 函数都必须接收 `&Window` / `&App`。

**复用 vs 重建。** gpui-kit 组件在适当处被复用（`Dialog` + `Command` 同时用于 command palette 和 branch popup；`PopupMenu` 用于菜单项），而 hardcode 不兼容几何的地方才手写 chrome：`AppMenuBar` 硬编码 24 px 条目，Windows 上 `NativeMenu` 从不调用 `SetMenu`（`menu_bar.rs:31-44`）；`component::TitleBar` 无条件绘制自己的 34 px 窗口控制（`title_bar.rs:56-68`）；组件的 status bar 有两段缩放区域（`status_bar.rs:22-36`）；`ListDelegate` 要求统一行高，所以 notification
