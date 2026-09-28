# macOS 界面调研（**只作行为/功能对照**，不是视觉规格）

> 本文件原先是 `UI-MAP.md` 的第 2 章"逐区域对应"，以 **macOS SwiftUI/AppKit** 为界面规格写的。
> **2026-09-25 起界面规格改为 Windows 前端**（`windows/tauri/src/features/*`），所以这一章降级为
> **行为/功能对照**：它功能最全，用来查"这个交互原本怎么工作、有哪些状态"，**不要拿它的尺寸/布局当规格**。
>
> - 视觉/布局规格：`UI-MAP-WINDOWS.md`（由 `gpui/research/windows/01..06-*.md` 汇总）
> - 仍然有效的硬规则与 gpui-kit 实现规则：`UI-MAP.md` §1
> - 原始调研（逐元素证据）：已在 2026-09-25 清理；本章即其汇总

---
## 2. 逐区域对应（macOS → gpui-kit）

### 2.1 应用外壳（macOS 出处：`macos/Sources/Lithe/Views/App/`、`Application/`、`Theme/`）

**组成**
- `LitheApp`（`LitheApp.swift:309`）声明 **3 个 Scene**（不是单窗口应用）：
  - `WindowGroup(LitheWindowID.welcome)` → `RootView(scope: .primary)`（`:463-464`）
    - `ProjectSessionContent × N`（`ZStack`，所有会话同时驻留视图树）→ `StandaloneEditorView` / `WelcomeView` / `WorkbenchView`（`RootView.swift:70-76,168-172`）
    - `ActiveSessionChrome`（承载 sheet / confirmationDialog）、`ProjectWindowSceneBridge`、`.sheet(OpenProjectLocationDialog)`（`RootView.swift:78-96`）
  - `WindowGroup(LitheWindowID.project, for: UUID.self)` → `ProjectWindowMissingSessionView` 或 `RootView(scope: .dedicated)`（`LitheApp.swift:652-659,703`）
  - `Window(LitheWindowID.settings)` → `SettingsWindow` → `SettingsView`（`LitheApp.swift:679-746`）
- `WorkbenchView`（`WorkbenchView.swift:187`）`VStack(spacing: 0)`：
  - `topBar`（`:669`）→ `projectTabBar`（仅同窗口 >1 项目，`:554`）→ `HStack`：`activityBar`（左 38，`:1170`）+ `workspaceArea`（右侧留 40，`:1510`）→ `statusBar`（高 24，`:1650`）
  - 覆盖层（不占布局）：`rightHoverRegion`（右侧 40 活动栏 + 380 悬停侧栏，`:1349`）、通知 HUD（`:466`）、`SearchEverywhereView`（`:506`）、`ProjectReplaceOverlay`（`:538`）、项目/分支切换 popover（`:446-465`）
- 状态与命令类型：`ProjectSessionManager`（`sessions` / `activeSessionIDs` / `focusedScope`，纯内存）、`ProjectWindowScope = .primary / .dedicated(UUID)`、`LitheWindowLayout = .welcome / .workspace / .standalone`、`LitheCommandCatalog`（41 条命令）、`KeyboardShortcutFeatureModel` + `MacShortcutDetector`（双通道）、`LitheContextMenu`（自绘 `NSPanel`）、`WorkbenchNotificationFeatureModel`、`RecentProjectsStore`、`LitheTheme`（`Palette` / `Metrics`）。

**度量**（macOS 数值，可原样搬成 `px()`）
| 元素 | macOS 值 | 出处 |
| --- | --- | --- |
| 顶栏 `topBar` | 高 **40**，左内边距 **76**（红绿灯）、右 10，`HStack(spacing: 9)` | `WorkbenchView.swift:756-758`；`LitheTheme.swift:416` |
| 项目标签栏 | 高 **38**（`tabHeight` 34 + 4）；标签高 30，最小宽 **180**，水平 padding 6、标签间距 6 | `WorkbenchView.swift:556-563,589,639` |
| 左活动栏 / 右插件活动栏 | 宽 **38** / **40**；按钮 30×30、间距 4、边距 4；底部工具视口高 292 | `WorkbenchView.swift:6-18,1341` |
| 状态栏 | 高 **24**，水平 padding 9，`spacing: 10`，`smallFont`(12) | `WorkbenchView.swift:1651-1664`；`LitheTheme.swift:418` |
| `LitheTheme.Metrics` 全套 | rowHeight 24、treeRowHeight 27、tabHeight 34、toolWindowHeaderHeight 30、statusBarHeight 24、cornerRadius 5、popupCornerRadius 10、contextMenuCornerRadius 9、controlCornerRadius 6 | `LitheTheme.swift:404-423` |
| 字体 | uiFont 14 / smallFont 12 / codeFont JetBrainsMono-Regular 13 | `LitheTheme.swift:376-378` |
| 主分栏约束 | 侧栏默认 **320**、min **220**、max `min(520, 可用宽-400)`；编辑区 min 400；上窗格 min 220、下方 min 260；上窗格默认 `max(255, H*0.40)` | `WorkbenchView.swift:195,1969-1990` |
| 右栏 | 悬停侧栏默认 **380**；右侧 dock 默认 360 / min 300 / max 520；保留工作区最小宽 640 | `WorkbenchView.swift:196`；`WorkbenchLayoutStore.swift:4-6`；`WorkbenchRightToolSplitView.swift:5-16` |
| 面板圆角 / 分隔条 | 圆角 10；分隔条可见厚 5、命中厚 10、线宽 1 | `WorkbenchView.swift:25`；`SplitHandleView.swift:13-14` |
| 窗口尺寸 | welcome 900×620（min 820×560）；project 1440×900（min 980×640）；standalone 1200×760（min 760×480）；settings 1040×720（内容最小 820×620） | `RootView.swift:333-351`；`LitheApp.swift:688`；`SettingsView.swift:60` |
| 屏幕留白 / standalone 自适应 | `screenMargin = 12`；宽 `clamp(W*0.65, 760, 1200)`、高 `clamp(H*0.72, 480, 820)` | `RootView.swift:338,356-382` |
| 欢迎页 | 左栏固定 240；右侧工具条高 66；项目行高 52（首字母方块 34×34 圆角 8） | `WelcomeView.swift:77,119-120,262-263` |
| 设置窗口 | 左导航固定宽 244；分类行高 27；内容 padding 水平 28 / 垂直 22；底栏高 52 | `SettingsView.swift:132,284,305,1425` |
| 通知 HUD | 宽 280…360、圆角 7、内边距 12/6/10；最多 3 条、停留 4s、历史 100；位置 trailing 52 / bottom 38 | `WorkbenchView.swift:495-503`；`WorkbenchNotificationFeatureModel.swift:5-7` |
| 通知中心 | 340×360，头部高 38（标题 13pt、"Clear All" 11.5pt） | `WorkbenchView.swift:1828-1849,1900` |
| 上下文菜单 | 根宽 230…360、子菜单 220；行高 26、分隔 11、垂直 padding 12；项 12pt / 快捷键 11pt；宽度 = 最宽项 + 67 | `LitheContextMenu.swift:4-14,417-444` |
| popover 共用外壳 | 箭头 22×12；leadingOverlap 10；viewportMargin 8 | `WorkbenchView.swift:27-32` |
| 顶栏两个 popover / 对话框 | 项目切换 390×max520；分支切换 375（搜索条 56、动作行 30、分支行 28）；Open Project 470；新建分支 420；检出修订 450 | `ProjectSwitcherPopover.swift:3-6`；`BranchSwitcherPopover.swift:6-11,747,799`；`OpenProjectLocationDialog.swift:68` |
| 按钮 / 输入框 | 图标按钮 28×28 圆角 5；主/次按钮高 30、水平 padding 18；搜索框高 28；`lithePopupChrome` 圆角 10 | `LitheTheme.swift:479-497,528-573,619-641,674-685` |

**元素 → gpui-kit 对应**
| macOS 元素 | gpui-kit 实现 | 判定 |
| --- | --- | --- |
| 3 类窗口（welcome / project / settings） | `gpui_kit::WindowOptions` + `cx.open_window`，根视图 `component::Root` | 可一比一 |
| 隐藏系统标题栏 / 透明标题栏 | `component::TitleBar`（`window_options()` / `title_bar_options()`） | 可一比一 |
| 顶栏内容（项目/分支切换、运行控件） | `component::TitleBar` + 自定义子元素 | 可一比一 |
| 顶栏高 40、左留白 76 | `component::TitleBar`（gpui-kit 常量是 34 / macOS 左留白 80，需覆盖） | 需自行组合 |
| 双击标题栏缩放 | `component::TitleBar` + `gpui_kit::Window::zoom_window` / `is_maximized` | 可一比一 |
| 窗口圆角 / 边框 / 阴影 | `component::WindowBorder` / `component::window_border()` | 可一比一 |
| 多显示器 `visibleFrame` 夹取 + 居中 | 无对应物（`WindowOptions` 只有 `window_min_size`） | gpui-kit 没有 |
| 左侧活动栏 38 / 右侧插件栏 40 | `component::sidebar::Sidebar` + `component::sidebar::SidebarMenu` / `component::sidebar::SidebarMenuItem` | 可一比一 |
| 右侧 docked 工具面板 | `component::dock::Dock` + `component::dock::tab_panel::TabPanel` | 可一比一 |
| 项目标签栏 | `component::tab::TabBar`（`max_width` / `track_scroll`）+ `component::tab::Tab` | 可一比一 |
| 状态栏 / 面包屑 | `component::StatusBar`（`left` / `right`）；`component::breadcrumb::{Breadcrumb, BreadcrumbItem}` | 可一比一 |
| 三向可拖分栏（左右 + 上下嵌套） | `component::h_resizable` / `component::v_resizable` / `component::ResizablePanelGroup` / `component::resizable_panel` | 可一比一 |
| 分栏圆角 10 + 4 个 notch 拼角 | `base::resizable::ResizablePanel`（style）；notch 是 Lithe 私有绕行做法 | 需自行组合 |
| sheet / alert / 确认对话框 | `component::Sheet` + `component::Root::open_sheet_at`；`component::dialog::AlertDialog` / `component::dialog::Dialog` | 可一比一 |
| 顶栏 popover（22×12 箭头、贴边翻转） | `component::Popover`（`anchor` / `trigger` / `content`）；箭头需自绘 | 可一比一 |
| 右键上下文菜单 | `component::menu::ContextMenu` / `component::menu::ContextMenuExt` | 可一比一 |
| 应用菜单栏 / 菜单项 / 快捷键提示 | `component::menu::AppMenuBar`、`component::menu::menu_item`、`component::Kbd` | 可一比一 |
| 通知 HUD / 通知中心 | `component::Notification` + `component::NotificationList` + `component::Root::push_notification`；中心列表用 `component::List` 组合 | 可一比一 |
| 空态 / 加载态 / tooltip / 徽标 | `component::Empty`（+`EmptyHeader`/`EmptyMedia`/`EmptyTitle`/`EmptyDescription`）、`component::Spinner` / `component::Skeleton` / `component::progress::Progress`、`component::Tooltip`、`component::Badge` | 可一比一 |
| 主题 3 套配色 × 明暗 | `component::theme::Theme` + `component::theme::ThemeColor` + `component::ActiveTheme` + `component::theme::ThemeMode` | 可一比一 |
| 设置窗口（11 分类 + 搜索 + 底栏） | `component::setting::Settings` + `component::setting::SettingPage` + `component::setting::SettingGroup` + `component::input::Input` | 可一比一 |
| 命令面板（macOS 端没有，属新增能力） | `component::command::Command` + `component::command::CommandState` | 可一比一 |
| `openWindow(id:)` 三 Scene 语义 / 设置窗口 floating / 失效窗口 stderr 兜底 | 无对应物（需自建窗口注册表） | gpui-kit 没有 |

**复刻要点**
- 先定常量：顶栏 40（左 76 / 右 10）、状态栏 24、活动栏 38 / 40、侧栏 220…520（默认 320）、右栏 380、分栏圆角 10、`Metrics` 全套（`LitheTheme.swift:404-423`）。
- 必须保留的语义：`primary` 是共享 scope 而非被强制的单例窗口；重复打开同一项目按 `standardizedFileURL` 跨 scope 查重 → 激活既有标签并把其窗口前置（`ProjectSessionManager.swift:207-215,268-288`）；`Cmd+W` 先关工作台项再关窗（`RootView.swift:501-573`）；退出/关窗三选 `Save All` / `Don't Save` / `Cancel`（`LitheApp.swift:290-305`）。
- 非活动会话用 `ZStack` + `.opacity(0)` + `allowsHitTesting(false)`，全部会话同时驻留视图树（`RootView.swift:179-182`）——复刻时要显式决定是否保留这份成本。
- 命令目录 **39 条**（以源码为准，见下条注）要实现启动校验（id 唯一、单命令绑定唯一、全局绑定不冲突，`LitheCommandCatalog.swift:89-105`；含 `replacing: .newItem` 的 Open Project、`.saveItem` 后的 Save/Close Project/Close File、`Navigate` 与 `History` 两个 `CommandMenu`）；macOS 的两条执行通道（AppKit 菜单走 `primaryKeyPress`、全局 `NSEvent` 走全部绑定含 ⇧⇧）必须合并成一条。
  > 注：`.artifacts/ui-map/01-app-shell.md` §3.4 记的是 41 条，实现时按源码逐条点算为 **39 条**（38 条 `command(..)` + 显式构造的 search-everywhere）。**以源码为准**，读该文档时留意这处偏差。
- 壁纸模式下逐元素区分「结构背景」（`titlebar` / `editor` / `toolHeader` 退化为透明）与「控件背景」（输入框、选中、菜单、模态保留自身底色，`WorkbenchSurfaceBackground.swift:15-21`）。
- 六态要齐：禁用（活动栏 Pull Requests、路径不存在的欢迎页项目行、通知中心 Clear All）、悬停（`litheRowHover` + `lithePointer`）、加载（`ProgressView` + "Starting module..."）、空（无最近项目 / 无通知 / 设置搜索无结果）、错误（`NSAlert(error:).runModal()`、`ProjectWindowMissingSessionView`）、未保存确认（`Save All` / `Don't Save` / `Cancel`）。

**macOS 做得不好 / 用 gpui-kit 优化**
- **没有命令面板**（最值得补）：39 条命令里 **13 条完全没有默认键**（`.window` 7 条、`.history` 2 条、`.project` 2 条、`.run` 2 条），唯一入口是隐形手势「双击 ⇧」或 ⇧⌘O，且与输入法切 Shift 冲突。用 `component::command::Command`（`searchable` / `on_query` / `on_confirm` / `empty` / `max_h`）+ `component::command::CommandState` 开箱即用，并顺手补上 13 条默认键。
  > 本项目已落地：双击 ⇧（`on_modifiers_changed` + 350ms，见 §1.3 浮层一节）或项目标签条右端的搜索按钮唤起；本轮只有 2 条命令真生效（`大纲` / `项目统计`，切换右侧 dock），7 条 toggle-* 渲染为禁用（无终端组件、0.6.6 无运行时面板可见性 API），其余 24 条只在注释里登记待接线 —— 不放假按钮。

- 顶栏信息密度：9 类元素平铺，项目名 13pt semibold 与分支名 12.5pt medium 只差 0.5pt 无主次；两个 `chevron.down` 一个 9pt semibold、一个 8pt bold；双击缩放挂在背景手势上会与按钮点击竞争。用 `component::TitleBar` 容器 + `component::button::DropdownButton` 统一两个下拉（自带 chevron）+ `component::button::button_group::ButtonGroup` 归并运行组。
- 三个窗口职责交叉：`New Window` 与「回到欢迎页」是同一个动作；`welcome` 是可多实例 `WindowGroup` 但 `.primary` 逻辑假定唯一，`focusedScope` 是全局单值（`ProjectSessionManager.swift:34`）；设置窗口 `level = .floating` 且禁用最小化（`LitheApp.swift:837,855`），会永久压在其它窗口之上。用自建 `WindowRegistry { id → WindowHandle }` + 固定三个窗口类型。
- 活动栏两条：38 / 40 差 2pt 视觉不齐、图标 18pt 与 16pt 混用、未读红点靠 `.offset(-2, 3)` 手搓（壁纸模式下 `titlebar` 描边透明会失效）、292pt 工具视口滚动条隐藏（看不出还有更多）、恒不可用的 Pull Requests 仍占槽位。用 `component::sidebar::SidebarMenu` 的 `suffix` 放 `component::Badge`（`dot` / `count` / `max`），左右栏统一宽度常量。
- 状态栏 24pt 塞 11 项；`ViewThatFits` 只有「全要 / 砍三项」二元降级会跳变；`MemoryUsageStatusView` 与 `FrameRateStatusView` 是诊断信息却常驻；`checkmark.circle.fill` 恒亮 `success` 是误导。用 `component::StatusBar` 的 `left` / `right`，内存/帧率移入 `component::Popover`，对勾改语义色（无改动时隐藏）。
- 项目标签栏只在 >1 项目时出现：单项目没有任何「当前项目」锚点，打开第 2 个项目时整个工作台下移 38pt，溢出无指示器。用 `component::tab::TabBar` 常驻显示（`underline` / `max_width` / `track_scroll` 直接可用）。
- 通知两套并行实现且样式不一致（HUD `notificationBackground` 圆角 7 vs 中心 `raised` 无圆角）；HUD 所有通知同色无类型区分；通知中心一打开就 `markAllNotificationsRead()`，未读红点形同虚设。用 `component::Notification` 的类型化构造器（info/success/warning/error）+ `component::Root::push_notification`，未读改为逐条显式标记。
- 设置窗口：4 个 case 落 `EmptyView()` 的双重分派（`SettingsView.swift:225-266`）、左导航固定 244 不可拖、只有 Git 页 `maxWidth: 760` 与别人不同、`Cancel` 与 `OK` 行为完全相同（都只 `closeSettings()`，因为设置项是 `didSet` 即时写盘）。用 `component::setting::Settings`（`sidebar_width` 可拖）+ `component::setting::SettingPage::resettable` 统一「恢复默认」。
- 浮层 4 种圆角 + 5 套手写 modifier；右键菜单是自绘 `NSPanel`，快捷键**只作文本渲染**（不注册等键，按下去不触发）。用 `component::menu::ContextMenu` + `component::menu::menu_item` + `component::Kbd::binding_for_action`（真正绑定 action）；圆角统一走 `Theme::radius_tokens()`。
- 分栏 min/max 硬编码在 `body` 局部变量；右工具栏用 `committedWidth` 返回 `nil` 表示「临时 fit 值」的隐式约定没有任何类型保障（`WorkbenchRightToolSplitView.swift:26-33`）。用 `component::h_resizable` / `component::v_resizable` + `size_range`，min/max 成为 panel 属性。

**未查清**
- 跨启动恢复窗口：全仓无 `setFrameAutosaveName` / `frameAutosaveName` / `restorationClass` / `encodeRestorableState` / `applicationShouldSaveApplicationState`（**未找到**）；`sessions` / `sessionScopes` / `activeSessionIDs` / `focusedScope` 全是内存态，唯一启动带项目的路径是 CLI `--open-project <dir>`（`LitheApp.swift:692-700`）。确实持久化的只有最近项目（key `"lithe.recent-projects"`，上限 20）、`settings.projectOpenBehavior`、每个 workspace 的工作台布局与编辑器会话快照。
- `beginSheetModal` / `presentError` / `fullScreenCover` / `inspector` 在 `macos/Sources/Lithe/` 内均无出现（**未找到**）。
- 外壳几乎不碰 Rust Core：Core 命令表里**没有** window / recent-project / settings / theme / update / notification / command-palette 命名空间（`rust/lithe-core/src/protocol/command.rs:312-445`）；对应表中「最近项目、设置持久化、主题、Updates、打开位置询问、Toast、窗口/项目会话、工作台布局、活动栏模块贡献」的 Core 命令均**未找到**，需在 Rust 侧另建本地状态机。
- `"history.rename"` / `"history.delete"` 在 Core 契约里存在（`command.rs:335-336`），但 macOS 端**未找到**调用点。
- gpui-kit 侧四项无对应物（见对应表末行）：多显示器 `visibleFrame` 夹取、`openWindow(id:)` 的 Scene 语义、`window.level = .floating` + 禁用最小化、失效窗口 UUID 打印到 stderr 的兜底。

### 2.2 工作台骨架布局（macOS 出处：`macos/Sources/Lithe/Views/Workbench/`、`Views/Components/LitheSplitPaneView.swift`、`Services/Workbench/WorkbenchLayoutStore.swift`）

**组成**
- `WorkbenchView`（`WorkbenchView.swift:187`）`VStack(spacing: 0)`：`topBar` → `projectTabBar`（>1 项目才有）→ `HStack`：`activityBar`(38) + `workspaceArea`（`.padding(.trailing, 40)`）→ `statusBar`(24)；背景/浮层不占布局。
- `workspaceArea`（`:1510`）二选一：
  - `isDockedSidebarVisible` → `WorkbenchRightToolSplitView`（`LitheSplitPaneView(axis: .horizontal, placement: .trailing)`）：flexible = `workspaceContent`，sized = 右侧停靠工具窗（目前只有 Maven）。
  - 否则 → `workspaceContent` → `WorkbenchWorkspaceSplitView`（`:1922`）：
    - `isBottomToolVisible`（`activeToolWindow != nil`，`:1646`）→ `LitheSplitPaneView(axis: .vertical, placement: .leading)`：上 = `LitheSplitPaneView(axis: .horizontal, placement: .leading)`（左 `activeSidebar`（project/changes/pullRequests/search/database），右 `editor`），下 = `bottomTool`（references / spring / 模块 UI）；
    - 否则只有 `topContent`（`:2053`）。
- 右侧悬浮区 `rightHoverRegion`（`:1349`）：`HStack(spacing: 0)` = 悬停侧栏（宽 380，`workbenchPaneChrome`）+ `pluginActivityBar`（40，铃铛 / 插件 / `rightSidebarContributions`）。
- 骨架类型：`LitheToolWindowHeader`、`ActivityBarMetrics` / `WorkbenchLayoutMetrics` / `WorkbenchWorkspaceMetrics` / `WorkbenchPopoverLayoutMetrics`、`LitheSplitPaneView` + `SplitHandleView` + `LitheDragUpdateScheduler`、`WorkbenchLayoutStore`、`WorkbenchFeatureModel.ToolWindow`（`gitLog` / `terminal` / `references` / `problems` / `maven` / `mavenOutput` / `spring` / `run` / `tests` / `debug`）。

**度量**（macOS 数值，可原样搬成 `px()`）
| 元素 | macOS 值 | 出处 |
| --- | --- | --- |
| 标题栏 / 项目标签条 / 状态栏高 | 40 / 38（34+4）/ 24 | `WorkbenchView.swift:758,589,1663`；`LitheTheme.swift:415-418` |
| 左右活动栏 | 38 / 40；按钮 30×30、间距 4、边距 4、底部工具区高 292 | `WorkbenchView.swift:7-18` |
| 左侧栏宽 | 初始 **320**；min **220**、max `max(220, min(520, 可用宽-400))` | `WorkbenchView.swift:195,1969-1979` |
| 右侧悬浮侧栏 / 右侧停靠宽 | **380** / 默认 360、min 300、max `min(520, 可用宽-640-5)` | `WorkbenchView.swift:196`；`WorkbenchLayoutStore.swift:4-6`；`WorkbenchRightToolSplitView.swift:5-16` |
| 上（编辑区）高 | `topPaneHeight ?? max(255, geo.height * 0.40)`；min **220**、max `max(220, geo.height-5-260)` | `WorkbenchView.swift:1981-1990` |
| 编辑区宽 / 下（工具窗）高 | min 400 / min 260（Git 窗格） | `WorkbenchView.swift:1970,1981-1982` |
| 分隔条 | 可见厚 5 / 命中厚 10、线宽 1；命中区向两侧溢出 | `SplitHandleView.swift:13-14,53-60` |
| 面板圆角 | 10（用 4 个固定角缺口 `WorkbenchPaneCornerNotch` 拼出，共 165 行几何代码） | `WorkbenchView.swift:24,2104-2268` |
| 通用工具窗头 `LitheToolWindowHeader` | 高 30；图标 13（无资源时 SF 12pt medium）；标题 12.5pt semibold；leading 12 / trailing 7 | `LitheToolWindowHeader.swift:31-66` |
| 各面板头部高 | 项目侧栏 39（水平 12）/ 搜索侧栏 44（水平 13）/ Changes 页签头 40 / Changes 提交工具条 37 / Maven 导航条 36 / 终端工具条 30（标签 26） | `ProjectSidebarView.swift:203-204`；`SearchSidebarView.swift:26-27`；`ChangesSidebarView.swift:146-172,491`；`MavenView.swift:68-130`；`TerminalView.swift:26-131` |
| 编辑器标签条 | `tabHeight` = **34** | `EditorAreaView.swift:176,686`；`LitheTheme.swift:415` |
| 图标按钮 / 行悬停 | `LitheIconButtonStyle` 28×28 圆角 5；`litheRowHover` 圆角 5 | `LitheTheme.swift:460-497` |
| 内存浮层 | 宽 280、圆角 8、指标行 min 高 27 | `WorkbenchStatusViews.swift:119-135` |
| 通知中心 / 通知卡 | 340×360、头 38、背景 `raised` / 宽 280…360、圆角 7 | `WorkbenchView.swift:1824-1909,495-498` |
| 项目窗口 | 默认 1440×900、最小 980×640 | `RootView.swift:334,351` |

**元素 → gpui-kit 对应**
| macOS 元素 | gpui-kit 实现 | 判定 |
| --- | --- | --- |
| 整体骨架（活动栏 + 侧栏 + 编辑区 + 底部/右侧工具窗） | `base::dock::DockArea` + `base::dock::DockLayout` + `base::dock::DockPlacement` + `base::dock::Panel` + `base::dock::BasePanel` | 需自行组合 |
| 底部工具窗 | `base::dock::DockPlacement::Bottom`（只横跨 center 列，挂载方式见 `base::dock::dock_area` 的 `dock_frame`） | 需自行组合 |
| 左侧栏 / 右侧停靠工具窗容器 | `base::resizable::h_resizable` + `base::resizable::resizable_panel` + `base::resizable::ResizablePanelGroup`；右侧也可用 `base::dock::DockPlacement::Right` | 可一比一 |
| 左侧栏宽度拖动 | `base::resizable::ResizablePanel::size_range`（`220..520`）+ `base::resizable::ResizableState::resize_panel` | 可一比一 |
| 侧栏容器（Project / Changes / …） | `component::sidebar::Sidebar`（面向图标 + 文字导航项，不是任意面板容器） | 需自行组合 |
| 右侧悬浮侧栏（悬停打开 + 60ms 延迟关闭） | `component::popover::Popover` / `base::hover_card::HoverCard` + 自管状态机 | 需自行组合 |
| 活动栏按钮（图标 + 选中/悬停） | `component::button::Button` / `component::button::Toggle` + `base::StyledExt::h_flex` / `v_flex` | 需自行组合 |
| 面板头部（标题 + 工具按钮 + 省略号 + 折叠） | `component::dock::Panel`（`title` / `title_suffix` / `toolbar_buttons` / `dropdown_menu` / `zoom_control` / `title_bar`） | 可一比一 |
| 面板折叠 / 关闭 / 缩放 | `component::dock::ToggleZoom` / `component::dock::ClosePanel` + `base::dock::Panel` 的 `closable` / `visible` / `zoomable` | 可一比一 |
| 工具窗头部右键菜单 | `component::menu::PopupMenu`（dock 面板经 `dropdown_menu` 注入） | 可一比一 |
| 状态栏 / 面包屑 | `component::status_bar::StatusBar`（三段式，中间留空）；`component::breadcrumb::Breadcrumb` | 可一比一 |
| 编辑器标签条 | `component::tab::TabBar` + `component::tab::Tab` + `component::tab::TabVariant`（`Underline`） | 可一比一 |
| 标签重排拖拽 | `base::dock::tab_group`（`TabGroupContext` 的 `drag_panel` / `drop_panel` / `drop_item`）是「面板跨组」语义，编辑器标签需自建 `on_drag` / `drag_over` | 需自行组合 |
| 分隔条（命中区 / 光标 / 拖动 / `Resized` 事件） | `base::resizable::ResizeHandle`（`with_appearance` / `ResizeHandleRenderer`）；dock 内用 `DockAreaRenderer::render_split_handle` | 可一比一 |
| dock 尺寸持久化 / 恢复 | `base::dock::DockAreaState` + `base::dock::DockState` + `base::dock::PanelState` + `base::dock::PanelRegistry` + `base::dock::register_panel` | 可一比一 |
| 工作区壁纸 / 半透明表面 | 无直接等价（主题 token + `Styled::bg` 自建） | 需自行组合 |
| 通知中心 / 通知栈 / 加载占位 | `component::notification::Notification`；`component::spinner::Spinner` + `component::skeleton::Skeleton` | 可一比一 |
| 快捷键目录（用户可改键 + 冲突检测） | gpui-kit 只有 `gpui_kit::KeyBinding` + `actions!` / `#[derive(Action)]`，没有命令目录组件 | 需自行组合 |

**复刻要点**
- **底部工具窗的位置差异必须先决策**：Lithe 现状是底部横跨「左侧栏 + 编辑区」（`WorkbenchView.swift:2027-2051`），gpui-kit 的 Bottom dock 只横跨 center 列（`dock_area.rs:1415-1431`）。要复刻 Lithe 现状就把底部放进 center 的 `v_split`，不能用 `DockPlacement::Bottom` 直接表达；要复刻 IntelliJ「底部整窗横跨」则 gpui-kit 的 Bottom dock 做不到。
- 工具窗可见性有两套并行模型：`activeToolWindow` **单一互斥**（`setVisibility` 置真即替换，`WorkbenchFeatureModel.swift:56-68`），而 `maven` 走独立的 `isMavenDockVisible`（`:52-61`），所以 Maven 停靠右侧时底部工具窗可以同时存在。
- 面板位置是**编译期固定**的（`bottomTool` 的分支是硬编码 `if/else`，`WorkbenchView.swift:1570-1583`）；唯一可变的是三处 `LitheSplitPaneView` 尺寸（左侧栏宽 / 上下分割高 / 右侧停靠宽）。
- 拖动期间有合并调度并关闭隐式动画（`SplitHandleView.swift:73-82`），且只在拖动**结束**回调一次 `onCommit`（`LitheSplitPaneView.swift:108-128`）；`@State liveSidebarWidth` / `liveTopPaneHeight` 与外部 prop 做不等值守卫防回声写入（`WorkbenchView.swift:2072-2079`）。
- 行悬停统一 `litheRowHover(cornerRadius:activeBackground:)`；面板表面统一 `litheWorkbenchSurface(fallback)`（有壁纸即透明）；分隔条拖动态前景 `primaryText`、悬停 `secondaryText`、空闲 `divider`（`SplitHandleView.swift:134-135`）。
- 尺寸契约分散在 3 个文件（`WorkbenchView.swift` / `WorkbenchLayoutStore.swift` / `WorkbenchRightToolSplitView.swift`），复刻时集中为常量或 panel 属性。

**macOS 做得不好 / 用 gpui-kit 优化**
- 工具窗位置编译期固定、没有任何停靠能力（硬编码 `if/else`）。`base::dock::DockArea` 的 `move_panel` / `split_at` 原生支持跨组与停靠到某侧，复刻应直接采用而不是照抄这套固定分支。
- `activeToolWindow` 单一互斥导致无法同时看 Terminal + Problems，而 Maven 又单走一套可见性 → 两套并行状态机。用 `base::dock::Panel::visible` + dock tab group：同组多面板天然表达「同一槽位只能显示一个」。
- 默认分割比例是像素与比例混用：`topPaneHeight ?? max(255, geo.height * 0.40)` 只在「未拖动过」时跟随窗口，一旦持久化就永久固定像素值。改为保存比例（`base::dock::PanelInfo::Stack { sizes }`）更稳。
- 面板圆角用 4 个角缺口 hack（165 行几何）绕开「SwiftUI `clipShape`/`drawingGroup` 破坏 AppKit 承载视图」的限制。GPUI 没有这个约束，直接换成 `rounded` + `overflow_hidden`。
- 分隔条是自研 AppKit 交互层（`SplitHandleInteractionView`，手工 `NSTrackingArea` + `cursorUpdate` + 屏幕坐标差值 + 拖动合并）。`base::resizable::ResizeHandle` 已内置命中区、光标、拖动与事件，整套自研代码可删。
- `rightHoverRegion` 用 60ms 延迟 + 两个 `@State` 位判断关闭，逻辑分散难测 → 做成显式状态机。
- 活动栏底部固定 292pt「工具视口」：条目多时内部滚动（且 `showsIndicators: false`，用户看不到还有更多），条目少时贴底，条目数变化会让图标位置跳动。
- 工具窗头部三处各写一遍（`LitheToolWindowHeader` / `TerminalView.terminalToolbar` / `MavenView.navigationToolbar`，30 与 36 两种高度、字号与内边距有出入）→ 统一走 `component::dock::Panel` 的 `title` / `toolbar_buttons`。
- 状态栏 `ViewThatFits` 二元降级让整组信息跳变；7 条 `toggle-*` 工具窗快捷键默认没有绑定，IntelliJ 的 ⌘1 / ⌥F12 肌肉记忆需用户手配。

**未查清**
- 工作台是否订阅 Rust Core 事件流：**未找到**骨架层订阅点，骨架只消费功能模型状态（事件方向未逐条核对）。
- `ProjectPreparationSnapshot` 对应的 Core 命令名：**未找到**（契约类型来自 `LitheCoreContracts`，未在 `rust/lithe-core` 定位到产出它的命令）。
- 右侧 dock「hover 侧边栏」宽度 380 是否有持久化：**未找到**（`WorkbenchLayout` 只存 `sidebarWidth` / `topPaneHeight` / `mavenPaneWidth`）。
- 编辑器标签「拖拽到别的容器 / 拆分为新组」：**未找到**（现状只有横向重排与终端跨容器移动）。

### 2.3 资源管理器 / 项目树（macOS 出处：`macos/Sources/Lithe/Views/Workspace/`、`Views/Search/`）

**组成**
- `WorkbenchView` → `activityBar`（选中 `SidebarDestination` 决定侧栏内容，`AppModelSupportTypes.swift:90-115`）→ `WorkbenchWorkspaceSplitView`（水平可拖：侧栏 | 编辑器）→ `sidebar = activeSidebar(projectTreeRowHeight:)`（`WorkbenchView.swift:1552,1588`）：
  - `.project` → `ProjectSidebarView`（980 行）：
    - `sidebarHeader`（"Project" + scope 按钮 + Reveal/Refresh，`:169-205`）
    - 加载态 `ProgressView` + "Reading project…"（`:29-36`）→ `GeometryReader` > `ScrollViewReader` > `ScrollView([.vertical, .horizontal])`（`:38-70`）> 单一 `VStack` > `ProjectFileTreeContent`（`:315-354`）
    - `FileNodeRow(depth:)`（`:356`）→ `directoryRow`（递归 `FileNodeRow depth+1`，`:414-460`）/ `fileRow`（`:462-523`）
    - 错误态（`:109-127`）、空态（`:128-133`）、`.sheet ProjectItemNameDialog`（重命名，`:135-141`）、`.overlay ProjectItemNameDialogPresenter`（新建，无边框 `NSPanel`，`:142-146`）、`.confirmationDialog` "Move … to Trash?"（`:147-166`）
  - `.search` → `SearchSidebarView`：标题行（`:17-27`）→ 搜索输入行（放大镜 + TextField + 清除 + Replace + options 菜单，`:29-64`）→ `fileMaskField`（`:151-179`）→ 结果列表（`:87-139`）
- 同属该区域：状态栏面包屑（点击跳回项目树，`WorkbenchView.swift:1650-1653,1667-1746`）、`ProjectSwitcherPopover`（`:769-819`；`ProjectSwitcherPopover.swift:8-207`）、`SearchEverywhereView`（ZStack 顶层，`SearchEverywhereView.swift:96-563`）、`OpenProjectLocationDialog` / `CloneRepositoryView`（`RootView.swift:88-96,208-212`）。
- 树域模型：`FileNode { url, isDirectory, children, collapsedAncestorPaths, isInsideSourceRoot }`（`id = url.path`，`name` 为压缩后的点分名）、`WorkspaceNode`、`GitTreeStatusProjection`、`WorkspaceDirectoryMark`（6 个 case）。

**度量**（macOS 数值，可原样搬成 `px()`）
| 元素 | macOS 值 | 出处 |
| --- | --- | --- |
| 项目树行高 | **默认 24**，用户可调 `20...32`（步进 1） | `AppSettings.swift:71-72,160,316`；`SettingsView.swift:583-592`；`WorkbenchView.swift:1593` |
| 通用行高 / 通用树行高 | 24 / 27（27 只给设置页用，项目树不用） | `LitheTheme.swift:405-406` |
| 行间距 / 内容内边距 | 1 / 垂直 4、水平 12 | `LitheTheme.swift:409-411`；`ProjectSidebarView.swift:45,64,374` |
| 选中圆角 / 树图标 / 树字号 | 4 / 16 / 13.5 | `LitheTheme.swift:412-414` |
| 缩进步长 / 基线 | **14pt/层**，基线 8（`depth * 14 + 8`） | `ProjectSidebarView.swift:375,441,487` |
| 展开箭头 / 行内间距 / 行右内边距 | 箭头占位宽 10、字号 8 bold（文件行用 `Color.clear.frame(width: 10)` 对齐）；`HStack(spacing: 6)`；右 8 | `ProjectSidebarView.swift:426-430,469-470,442,488` |
| 行宽 | `max(可用宽 - 24, depth*14 + 8 + 180)` | `ProjectSidebarView.swift:372-377` |
| 侧栏标题行高 | 项目 **39**（水平 padding 12）/ 搜索 **44**（水平 padding 13） | `ProjectSidebarView.swift:203-204`；`SearchSidebarView.swift:26-27` |
| 搜索输入框 / file mask | **32** 圆角 6 / **28** 圆角 6；外间距水平 10、下 6（mask 下 10） | `SearchSidebarView.swift:56-68,172-178` |
| 侧栏宽度 | 默认 320、min 220、max `min(520, 可用宽-400)` | `WorkbenchView.swift:195,1969-1974` |
| Git 状态徽标 | 9pt bold monospaced（冲突 `!`、未跟踪 `A`，否则 workTree 状态字符） | `ProjectSidebarView.swift:480-485`；`GitModels.swift:764-769` |
| 上下文菜单 | 根宽 230…360、子菜单 220、最大 360；行高 26、分隔 11、垂直 padding 12；项 12pt / 快捷键 11pt；圆角 9 | `LitheContextMenu.swift:4-14`；`LitheTheme.swift:421` |
| 弹层共用外壳 | `lithePopupChrome` 圆角 10（`panelBorder` 1pt + `popupShadow` radius 30 / y 14） | `LitheTheme.swift:420,674-685` |
| Search Everywhere | 宽 **860**、最大高 **560**、顶部偏移 84；搜索框高 44 字号 15；标签行 38（选中下划线 2pt）；结果行 24（水平 padding 10） | `SearchEverywhereView.swift:192-195,227-232,264,282,421-423,487-489` |
| 项目切换弹层 | 宽 390、最大高 520、内边距 8；动作行高 30（水平 10 圆角 5）；项目行 minHeight 46、徽标 30×30 圆角 7 | `ProjectSwitcherPopover.swift:3-6,65-68,97-101,161-163,186` |
| 对话框 | Open Project 宽 470（body padding 20、按钮区 14）；Clone Repository 560×360（header 44、field 高 30）；新建/重命名 NSPanel 340×78（TextField 高 30）；重命名 sheet 宽 430 | `OpenProjectLocationDialog.swift:36,64-68`；`CloneRepositoryView.swift:75,108,160`；`ProjectSidebarView.swift:760,860,867,881-980` |

**元素 → gpui-kit 对应**
| macOS 元素 | gpui-kit 实现 | 判定 |
| --- | --- | --- |
| 项目树整体（虚拟化 + 展开折叠 + 选中 + 右键命中） | `base::TreeState` + `base::TreeItem` + `base::TreeEntry` + `base::TreeEntryState` + `base::TreeEvent`；组件层 `component::tree::Tree` / `component::tree::tree` | 可一比一 |
| 树行内容（图标 + 名字 + Git 徽标 + 缩进） | `component::tree::Tree::item` + `component::list::ListItem` | 需自行组合 |
| 展开箭头 | gpui-kit 无内置树箭头元素（`Tree::render` 不画箭头），由 `render_item` 自绘 | gpui-kit 没有 |
| 文件类型图标 | `gpui_kit::assets` 无 IntelliJ 图标集，需自行打包 SVG；`component::Icon` 只是文本/符号图标抽象 | gpui-kit 没有 |
| Git 状态徽标 | 无专用组件，自绘 `div().text_xs()` 等 | 需自行组合 |
| 行悬停 / 选中底色 | `component::list::ListItem` 的 hover / 选中态 + `component::theme::ThemeColor`（`list_hover` / `list_active` / `selection`） | 可一比一 |
| 行的纵向内边距 | `component::list::ListItem` 固定 `.py_1().px_3()`，与 24pt 固定行高冲突（紧凑行要 `py_0()`） | 需自行组合 |
| 树右键菜单 + 子菜单 + 勾选 | `component::tree::Tree::context_menu` + `component::menu::PopupMenu` / `component::menu::PopupMenuItem`（`submenu` / `checked` / `disabled` / `min_w` / `max_w` / `max_h`） | 可一比一 |
| 「Move to Trash」危险项 | `component::menu::PopupMenuItem` 没有 destructive 角色，用主题 `danger_foreground` / `danger_hover` 自绘 | 需自行组合 |
| 搜索输入框（放大镜 + 清除 + 后缀菜单） | `component::input::Input` + `component::input::InputState`（`prefix` / `suffix` / `cleanable`） | 可一比一 |
| 全局搜索（输入即搜 + 结果列表 + 键盘上下选） | `component::searchable_list::SearchableListState` + `component::searchable_list::SearchableListDelegate`（或 `component::list::List` 组合） | 可一比一 |
| 侧栏容器（标题 + 内容 + 折叠） | `component::sidebar::Sidebar` + `component::sidebar::SidebarHeader` + `component::sidebar::SidebarCollapsible`（树行不适合用 `SidebarMenuItem`） | 需自行组合 |
| 侧栏 ↔ 编辑器可拖分隔条 | `base::resizable::ResizablePanel` + `base::resizable::ResizeHandle` | 可一比一 |
| 项目切换弹层（箭头 + 分区标题） | `component::Popover` + `component::list::List`；箭头形状需自绘 | 需自行组合 |
| 状态栏面包屑（可点 token + chevron 分隔） | `base::Link` 或自绘 `div().hover(...)` | 需自行组合 |
| 新建 / 重命名 / 删除确认对话框 | `component::dialog::Dialog` / `component::dialog::AlertDialog` + `component::input::Input` | 可一比一 |
| 空态 / 加载态 / 错误态 | `component::list::ListDelegate` 的 `render_empty` / `render_loading` / `initial` + `component::progress::Progress` | 可一比一 |
| 活动文件高亮同步（若目标折叠会**自动展开祖先**） | `base::TreeState::set_selected_item` / `base::TreeState::reveal_item` | 可一比一 |
| 树键盘导航（↑ ↓ ← → Enter） | `base::tree::init` 绑定的 `SelectUp/Down/Left/Right` + `on_action_*` | 可一比一 |
| 树行禁用样式 | `base::TreeItem::disabled` + `component::list::ListItem::disabled`（落到 `muted_foreground`） | 可一比一 |
| 树的多选 / 树内拖拽移动文件 / 树内过滤排序 UI | 无对应；需 GPUI `on_drag` / `on_drop` 自管状态（主题有 `drag_border` / `drop_target`） | gpui-kit 没有 |

**复刻要点**
- **`base::TreeState` 的行高是硬约束**：内部用 `uniform_list`，只测第 0 行后按固定行高铺满（`gpui-pre-0.3.6/src/elements/uniform_list.rs:359-371`）。因此：所有行自然高度必须相等；行内纵向 padding 会让相邻行重叠（`component::list::ListItem` 默认 `.py_1()` 会叠加项目树自己的垂直内边距 4）；行高只能是**一个**固定值（不能按目录/文件区分）；「行间距 1」要么算进行高（24→25），要么无效（`uniform_list` 不为 margin/gap 留位置）。确实需要不等高行时必须改用 `base::VirtualList`（逐行给高度）并自己实现展开/选中。
- 展开状态建议收进 `TreeItem`（`TreeEvent::Expanded/Collapsed`），而不是复刻 `@State expandedDirectoryPaths`；首次进入只展开根。
- 图标 + 名称 + 缩进 + Git 徽标用 `ListItem` 的 `suffix`（放徽标）+ 自绘缩进（depth 由树给出）；`TreeEntry` 提供 depth。
- 交互语义：点击文件行**直接打开**（无「单击选中、双击打开」两段式）；可执行二进制单击不打开、双击交给系统关联（`ProjectSidebarView.swift:6-15,463-467,508-514`）；右键菜单打开时该行保持高亮（`contextMenuPath`）。
- 排序与隐藏规则在 Rust 侧：目录优先 + 文件名小写升序；内置隐藏目录 14 个（`.git`、`node_modules`、`target`、`build` 等）+ `.lithe/run/local.json`；符号链接扫描时一律跳过；界面层没有过滤/排序控件。
- 增量更新链路要照搬：FSEvents（0.25s）→ `DirectoryChangeBatch` → **350ms 去抖** → 全树重扫 或 `updateSearchIndex` / `reloadProjectServices` / `requestGitRefreshNow`；Git 命令期间 `beginGitOperationFreeze()` 暂停观察、结束后 flush；Git 刷新单独 350ms 去抖。

**macOS 做得不好 / 用 gpui-kit 优化**
- 无虚拟化：整棵树是 eager `VStack`（注释自认「eager stack 必须测出完整高度」，`ProjectSidebarView.swift:41-46`），每行还挂 `.task(id:)` 异步解析图标。用 `base::TreeState` 的 `uniform_list` 直接换虚拟化。
- 悬停态与活动文件态不可区分：两者都用 `subtleSelection` 灰底（`ProjectSidebarView.swift:446-451,492-499`），没有 IDEA 的蓝底 + 加粗文件名。`component::list::ListItem` 已区分 `list_hover` 与 `list_active` / `accent`，活动项应改成强调底。
- 完全没有键盘导航、多选、拖拽（`ProjectSidebarView.swift` 零 key 处理、零 `onDrag/onDrop`），命令目录也没有「新建文件 / 新建目录 / 重命名 / 删除 / 复制路径 / 在项目树中定位」这些命令。`base::TreeState` 自带方向键与 Enter，是最低成本的一比一补偿。
- 单击即打开、没有单选态，配合上一条导致无法用键盘浏览项目。
- 列表内没有过滤/搜索入口（IDEA 项目树自带常驻过滤框）→ `component::searchable_list::SearchableListState` 或 `component::input::Input` + 过滤闭包。
- 文件操作进行中无任何 UI 反馈、菜单项不禁用（`isPerformingProjectItemOperation` 从未绑到视图层）→ `base::TreeItem::disabled` + `component::list::ListItem::disabled`。
- 危险操作无视觉区分：`LitheContextMenuItem.Role.destructive` 在渲染层完全未被消费 → 用 `danger_foreground` / `danger_hover` 自绘（这一点 macOS 与 gpui-kit 都需要补）。
- 新建与重命名是两个完全不同的对话框（340×78 无边框 `NSPanel` vs 宽 430 的 `.sheet`），同一语义两套交互 → 统一成一个 `component::dialog::Dialog` + `component::input::Input`。
- 同一条侧栏 header 高度不一致（项目 39 / 搜索 44）；两条搜索去抖 250ms / 180ms 都是硬编码 → 提到统一常量；搜索侧栏结果行不可键盘导航（只有 Search Everywhere 有 `NSEvent` monitor），用 `component::list::ListState` 补选中与 `confirm`。
- 排序/过滤规则只在 Rust 侧且界面无开关，要「按类型/按修改时间排序」属 Core 契约变更。

**未查清**
- 树的多选 UI：**未找到**（`ProjectSidebarView.swift` 内无 selection set、无 modifier key 分支）。
- 树的拖拽移动文件：**未找到**（`Views/Workspace/` 无 `onDrag/onDrop`）。
- 树内过滤/排序控件：**未找到**（`Views/Workspace/` 无相关控件，排序仅在 Rust）。
- 空目录占位文案：**未找到**（`children == nil` 或 `[]` 时展开后什么都不渲染）。
- 文件「预览模式」（单击预览、双击固定）：**未找到**；单击即 `openFile` 进编辑区。
- 「新建文件 / 新建目录 / 重命名 / 删除」的菜单栏项或快捷键：**未找到**（`LitheCommandCatalog.swift:13-66` 无对应 id，只能从右键菜单触发）。
- `ProjectGitStatusSnapshot.relativePath(for:root:)`（`ProjectSidebarView.swift:239-244`）已定义但本文件内未见调用点，判断为死代码，未进一步全局确认。
- `WorkspaceDirectoryMark.package` 与 `LitheIcons.kind(for:mark)` 的完整图标资源清单：只确认了映射函数与资产路径表，未逐一核对 SVG 是否存在。
- ~~gpui-kit 是否有官方 breadcrumb~~ **已核实：有**，路径是 `component::breadcrumb::{Breadcrumb, BreadcrumbItem}`（模块里还有 `BreadcrumbSeparator`；`gpui-component-0.6.6/src/breadcrumb.rs:13,20,141`，crate 根只 `pub mod breadcrumb`、没有再导出，所以必须写全模块路径）。同一轮检索**确实没有**的是：Git 状态徽标组件、IntelliJ 图标集（`gpui_kit::assets` 只有默认的 101 个字形）。

### 2.4 编辑区（macOS 出处：`macos/Sources/Lithe/Views/Editor/`、`Platform/MacOS/MonacoWorkbenchEditor.swift`、`macos/EditorFrontend/`）

**组成**
- **先决事实**：编辑内容宿主是 `WKWebView` + Monaco（本地 scheme `lithe-editor://app/index.html`，`MonacoWorkbenchEditor.swift:279-296`；前端实现在 `frontend/editor/src/workbench.ts`）。SwiftUI 只负责标签栏、查找栏、软换行开关、状态栏面包屑、空态、外部冲突横幅、分屏标题条；行号、当前行高亮、minimap、滚动条、折叠、诊断波浪线、补全/悬停/CodeLens、gutter 装饰全部由 Monaco 渲染。`Views/Editor/CodeEditorView.swift`（5497 行自绘 `NSTextView` + `LineNumberGutterView` + `CodeVisionOverlayController`）**没有任何实例化点**，是死代码。
- `EditorAreaView`（`EditorAreaView.swift:44`）`ZStack(alignment: .top)`：
  - 按上下文分流（`DatabaseWorkspaceView` / `BranchComparisonView` / `GitCommitDiffReviewView` / `DiffReviewView`）或 `VStack`：`emptyState`（无标签，`:1298-1311`）/ `editorWorkspace`（`:1086`）
  - `LanguageImplementationChooserView` overlay（顶部内缩 48/24）
- `editorWorkspace`：`editorTabs`（34）→ 分屏分支（标题条 30 + `MonacoWorkbenchEditor(primary: secondary:)`）或非分屏分支（`externalConflictBanner` 42 + `activeEditor`）。
- `editorTabItems` 三类标签：`.document` / `.terminal` / `.media`（`EditorTabItem`）；单标签 = 图标 13 + 标题 + 脏点 + 关闭按钮 20×20 + 拖放插入指示条。
- `activeEditor`：`TerminalSurfaceView` / `MediaViewerView` / 按扩展名分流（`.svg` / `.md` / `.html` 三种预览，其他 `editorWithFindBar` → `codeEditor` + `FindBarOverlay` + `EditorSoftWrapToggle`）。
- 覆盖 chrome：`FindBarOverlay`、`EditorSoftWrapToggle`、`GoToLineDialogPresenter`；问题面板在**底部工具窗口**（`JavaProblemsView`），不在编辑区内。

**度量**（macOS 数值，可原样搬成 `px()`）
| 元素 | macOS 值 | 出处 |
| --- | --- | --- |
| 标签栏高 | `Metrics.tabHeight` = **34** | `EditorAreaView.swift:176,228,417`；`LitheTheme.swift:415` |
| 单行 / 多行布局 | `ScrollView(.horizontal, showsIndicators: false)` + `HStack(spacing: 0)`；`EditorTabFlowLayout(horizontalSpacing: 4, verticalSpacing: 2)`，外 padding 水平 2 / 垂直 2 | `EditorAreaView.swift:222-228,243-247` |
| 多行最小标签宽 | `EditorTabFlowLayout.minimumItemWidth = 154` | `EditorTabFlowLayout.swift:204` |
| 标签内容 | 左边距 11、图标 13、`HStack(spacing: 7)`、标题 12.5pt（多行 `truncationMode(.middle)` + `maxWidth 240`） | `EditorAreaView.swift:679-685,725-735` |
| 脏标记 | 圆点 6×6、`primaryText` | `EditorAreaView.swift:1614-1623` |
| 关闭按钮 | `xmark` 9pt semibold、`frame(20×20)`、右侧 padding 4、仅活动或悬停可见 | `EditorAreaView.swift:632-646` |
| 活动标签 / 拖放态 | 底部 2pt `accent` 下划线；拖放目标 `accent.opacity(0.13)`；插入指示条 `Capsule` 宽 3（上下 padding 5）；拖动中 `opacity 0.92` / `scaleEffect 0.99` | `EditorAreaView.swift:648-671,737-743,392-396` |
| 拖拽阈值 / 死区 | 单行 `DragGesture(minimumDistance: 8)`；`hoverDeadZoneRatio = 0.10` | `EditorAreaView.swift:812-816`；`EditorTabFlowLayout.swift:84-97` |
| 预览模式切换器 | 整体 104×26、`padding(2)`、圆角 5；内部 3 个 29×20 按钮、图标 11pt、`spacing 1`；选中 `selection.opacity(0.82)` | `EditorAreaView.swift:1014-1062` |
| 分屏标题条 | 高 **30**、水平 padding 10、文件名 11pt + "Close split" | `EditorAreaView.swift:1094-1100` |
| 外部冲突横幅 | 高 **42**、`orange.opacity(0.10)`、底部 1pt `warning.opacity(0.35)`、文本 11.5pt medium、右侧两个 small 按钮 | `EditorAreaView.swift:136-163` |
| 空态 | 图标 44pt ultraLight + 标题 15pt medium + 副标题 14pt，`VStack(spacing: 12)` | `EditorAreaView.swift:1298-1311` |
| 软换行开关 | 右上 `padding(.top, 8)` / `trailing 10`；图标 12pt + 叠角 `arrow.turn.down.left` 6.5pt | `EditorSoftWrapToggle.swift:14-37` |
| 查找栏 | 容器 `spacing 6` + `padding(8)` + 圆角 6；行高 28；输入框 `maxWidth 360` 圆角 3；选项按钮 22×22 圆角 3；计数 11pt monospacedDigit `minWidth 60`；按钮 28×24（bordered 时水平 padding 12） | `FindBarView.swift:28-48,205-222,163-179,93-98,225-259` |
| 跳转行对话框 | `NSPanel` **340×96**、label 13pt、字段高 24、按钮 78×30 | `GoToLineDialog.swift:55-123` |
| 编辑器字体 / 行高 | `codeFont` = JetBrainsMono-Regular **13**（设置范围 10…22）；`editorLineHeightMultiple = 1.2`、`editorBaselineLift = 1.5` | `LitheTheme.swift:378-380`；`SettingsView.swift:573-581` |
| 面包屑（在状态栏） | 状态栏高 24、`smallFont`(12)、水平 padding 9；项 `spacing 4`、图标 12、项间距 6、分隔符 `chevron.right` 7pt | `WorkbenchView.swift:1650-1665,1667-1746` |
| Monaco 布局关键项 | `automaticLayout` / `minimap.enabled` / `glyphMargin` / `scrollBeyondLastLine: false` / `fixedOverflowWidgets` / `semanticHighlighting`；构造字号 13 | `workbench.ts:1126-1129`；`MonacoWorkbenchEditor.swift:738-747` |
| Monaco 行号 / blame / 诊断 | `lineNumbersMinChars: 26`（blame 模式替换行号渲染器）；severity 映射 `[1:8, 2:4, 3:2, 4:1]` | `workbench.ts:437-444,1009-1013` |
| git 行改动色条 | 宽 3px + margin-left 2px（`lithe-git-marker`） | `workbench.ts:739-745`、CSS `:1165-1168` |

**元素 → gpui-kit 对应**
| macOS 元素 | gpui-kit 实现 | 判定 |
| --- | --- | --- |
| Monaco 编辑器本体（行号、当前行、缩进线、折叠、选择、光标） | `component::input::Editor` + `component::input::EditorState`（= `base::input::EditorState`） | 可一比一 |
| 行号列 / 当前行高亮 / 缩进参考线 | `base::input::InputEditorStyle`（`editor_gutter_background` / `editor_active_line`） | 可一比一 |
| 语法高亮（tree-sitter） | `base::input::InputHighlighter` + `base::input::HighlightStyleResolver` + `component::highlighter::*` | 可一比一 |
| 诊断波浪线 / 诊断气泡 / 诊断色 | `base::input::Diagnostic` + `base::input::DiagnosticSet` + `base::input::DiagnosticSeverity`；`component::input::popovers::DiagnosticPopover`；`base::input::InputEditorStyle::diagnostics` | 可一比一 |
| 装饰（行背景 / 文本高亮） | `base::input::TextDecoration` + `base::input::TextDecorationCollection` | 可一比一 |
| 软换行 / 只读 / 粘贴钩子 / 右键菜单 | `component::input::EditorState`（`soft_wrap` / `line_number` / `folding`）；`component::input::Editor`（`readonly` / `on_paste` / `context_menu`） | 可一比一 |
| 补全 / 悬停 / 跳转定义 / code action | `base::input::editor::lsp::{CompletionProvider, HoverProvider, DefinitionProvider, CodeActionProvider}` + `base::input::editor::lsp::Lsp`；UI 用 `component::input::popovers::completion_menu` / `hover_popover` / `code_action_menu` | 需自行组合 |
| 语义高亮 | `base::input::editor::lsp::semantic_tokens::DocumentRangeSemanticTokensProvider` | 需自行组合 |
| minimap / overview ruler / sticky scroll | 三个 crate 中 grep 零命中 | gpui-kit 没有 |
| Code Vision（code lens） | 无 `CodeLensProvider` 概念，只能用 `TextDecoration` 自绘 inline 文本 | gpui-kit 没有 |
| 内联 blame / gutter 装饰（git 色条、断点、执行行、运行图标） | 只有 `FoldIconRenderer` 是可注入的 gutter 渲染钩子；其余需自绘左侧 overlay | 需自行组合 |
| 标签栏（34pt、可选中、可滚动） | `component::tab::TabBar` + `component::tab::Tab` + `component::tab::TabVariant::Underline`（Underline 高 36，与 34 差 2pt） | 可一比一 |
| 标签图标 / 脏点 / 关闭按钮 | `component::tab::Tab::icon`；脏点与关闭按钮用 `Tab::suffix` + `component::Badge::dot` / `component::button::ButtonIcon` | 需自行组合 |
| 单行溢出（全部标签下拉） | `component::tab::TabBar`（内层 `overflow_x_scroll`；`menu(true)` 给出溢出下拉，含勾选当前、可滚动、右上锚点） | 可一比一 |
| 多行换行排布（`EditorTabFlowLayout`） | 无对应（`TabBar` 固定单行滚动），需自写 GPUI `Layout` 或改用溢出菜单 | gpui-kit 没有 |
| 标签拖拽排序 / 中键关闭 | `TabBar` 无拖拽（需自建 `on_drag` / `drag_over`）；中键用 `on_mouse_down(MouseButton::Middle)` | 需自行组合 |
| 标签右键菜单（14 项 + 子菜单） | `component::menu::ContextMenuExt` + `component::menu::PopupMenu`（快捷键提示自动解析真实键位） | 可一比一 |
| 查找/替换（28pt 浮动条、`n/m` 计数、Cc/W/.* ） | `base::input::EditorState`（`open_search` / `set_search_query` / `next_search_match` / `replace_all_search_matches` / `search_session`）+ `base::input::editor::search::SearchMatcher::label()`；面板 `SearchPanel` 是 `pub(super)` 不可直接用 | 需自行组合 |
| 跳转行 / 关闭脏文档确认 | `component::dialog::Dialog` / `component::dialog::AlertDialog` + `component::input::Input` | 可一比一 |
| 面包屑 / 状态栏 / 空态 / 加载态 / 冲突横幅 | `component::breadcrumb::{Breadcrumb, BreadcrumbItem}`；`component::StatusBar`；`component::Empty`（+`EmptyHeader`/`EmptyMedia`/`EmptyTitle`/`EmptyDescription`/`EmptyContent`）；`component::Spinner` / `component::Skeleton`；`component::alert::Alert`（无 action 槽，需右侧自放按钮） | 可一比一 |
| 滚动条（IDEA 风格 overlay） | `component::scroll::Scrollbar`（`mode` / `styles`）+ `base::ScrollableElement` | 可一比一 |
| 分屏（一主一副）/ 拖到另一侧 | `component::h_resizable` / `component::v_resizable` + `base::resizable::ResizablePanel::size_range`；迁移用 `base::dock::DockArea::move_panel` / `split_at` | 可一比一 |
| 编辑器字号 / 字体 / 行高 | `component::theme::Theme`（`mono_font_family` / `mono_font_size`）+ `.text_size()` / `.font_family()`（组件默认行高 1.5，Lithe 用 1.2） | 可一比一 |
| LSP 客户端 / 传输、Java tree-sitter grammar | 只提供 provider trait，JSON-RPC 与进程管理自备；内置 grammar 有 rust/go/python/js/ts/json/html/css/markdown/lua/kotlin/php/zig，**无 Java** | gpui-kit 没有 |

**复刻要点**
- **边界结论：编辑内容不必再做 WebView**。`base::input::EditorState`（`= InputBaseState<EditorMode>`）本身就是代码编辑器引擎：行号、折叠、缩进参考线、tree-sitter 高亮、诊断波浪线、装饰、多光标、撤销栈、内置查找替换（含 `"2/3"` 计数）、LSP provider 钩子；官方注释给的上限是「适合简单编辑/展示，不是全功能编辑器，可处理约 5 万行」。超过 5 万行、或要 minimap / overview ruler / Monaco 级语言服务，就只能继续保留 WebView 宿主。
- 标签栏与内容的分工逐层照搬：标签顺序、图标、脏点、关闭、悬停、拖拽排序、右键、中键、右侧预览切换器在宿主；文本渲染与折叠、诊断、gutter 在编辑引擎。
- 数据链路要保留：`file.read` / `file.write`（workspace 相对路径，拒绝绝对路径、`..` 与二进制）；`document.lifecycle`（`clean` / `dirty` / `saving` / `conflict` → `writeToDisk` / `reloadFromDisk` / `showConflict`）；保存前的编辑器冻结屏障（比对 `revision` / `text` / `barrierDrained`，失败则保存中止）；关闭前的 `holdForClose` / `releaseClose`；诊断由 `lsp.*` 汇聚到 `EditorDiagnosticsStore`。
- 装饰数据来源全在 Core：`git.blame` / `git.status` / `git.diff` / `git.apply`、`debug.*`、`java.codeVision`、`java.navigationMarkers` / `java.resolveNavigation` / `java.runMarkers`、`java.structure`、`editor.lineEdit` / `editor.lineCommentToken`、`history.*`。
- 预览标签模型（`previewDocuments`：单击预览、下一步被替换）必须在标签上有视觉语义。
- 编辑器 chrome 度量要显式 token 化：Monaco 侧 `renderLineHighlight` / `scrollbar.*` / `tabSize` / `lineHeight` / `guides` / `overviewRulerLanes` / `stickyScroll` 都没显式设置，只由 `fontSize` / `fontFamily` / `wrap` / `minimap` / `theme` 五个运行期字段驱动，绑死在 Monaco 0.55.1 默认值上，升级即漂移。

**macOS 做得不好 / 用 gpui-kit 优化**
- 5497 行原生编辑器是死代码，还导致「大文件禁用软换行」永不生效（`updateSoftWrapAvailability` 只在这份死代码里被调用，`isSoftWrapAvailable` 恒 true，按钮永不置灰）。复刻直接删掉这条分支，把大文件策略落到编辑引擎配置里。
- 预览标签无视觉语义（模型有 `previewDocuments`，标签栏无斜体/图标/颜色差异），用户无法判断标签会不会被顶掉 → 用斜体标签名（IDEA 做法）。
- 标签操作没有快捷键：命令目录里没有 `close-tab` / `next-tab` / `previous-tab` / `split-right`，`Cmd+W` 完全没绑定（`close-project` 用的是 ⇧⌘W），关标签只能点 × 或中键 → 用 `gpui_kit::KeyBinding` + `actions!` 补齐并进入快捷键设置页。
- 单行标签栏只有横向滚动、没有溢出菜单 → `component::tab::TabBar::menu(true)` 直接提供「全部标签」下拉。
- 标签悬停无任何视觉反馈（`inactiveTabBackground = Color.clear`，悬停只让关闭按钮出现）→ 用 `cx.theme().secondary_hover` 补一层悬停底色。
- 脏标记与关闭按钮是「并存」而非「互换」，多占约 24pt 横向空间；脏点颜色主工作台用 `primaryText`、独立文件窗口用 `accent`。改成同一位置互换 + 统一到一个 token。
- 分屏只有单个 `splitDocumentID`（主 + 1 个附加文档），不能拖回主区，标题条自绘，HTML 分支甚至是裸 `HStack` 各占一半 → `component::h_resizable` / `component::v_resizable` + `size_range` 给真正的 N 路分屏，或 `base::dock::DockArea` 的 `split_at` / `move_panel`。
- 面包屑被塞进 24pt 状态栏、只有路径没有符号层级、点击只做 `revealInProjectTree` → 用 `component::breadcrumb::Breadcrumb` 放到编辑区顶部，符号层级用 `component::menu::PopupMenu` 补。
- 跳转行是阻塞式 `NSApp.runModal` → 换 `component::dialog::Dialog`（自带遮罩 + 焦点陷阱）或做成编辑器内联输入。
- 只读提示过弱（编辑区无横幅，只有状态栏一把锁 + tooltip）→ 用 `component::alert::Alert` 的 `banner()` 做可见提示。
- 大文件/二进制只在打开失败后弹通知，工作台内没有「编辑器内空态 + 重试」的落点 → `component::empty::Empty` + `component::alert::Alert`。
- 查找栏缺关键能力（没有「全项目」切换、没有查询历史下拉、正则非法只改放大镜颜色、Cc/W/.* 是自绘 22×22 文本按钮）→ 引擎内建 `SearchMatcher::label()` 计数与大小写按钮，`Cmd+F` / `Cmd+Shift+F` 键位可直接沿用。
- 空态文案未本地化（`Text("Select a file to review")` 未包 `LocalizedStringKey`，英文硬编码）。

**未查清**
- 标签栏左侧按钮：**未找到**（左侧直接是标签流，无前进/后退按钮；前进/后退只有菜单命令 `Cmd+[` / `Cmd+]`）。
- 切换标签 / 关闭标签 / 下一个标签 / 分屏的快捷键：**未找到**（菜单栏只有 Close Project / Close File）。
- Monaco 内的面包屑：**未找到**（grep `breadcrumb` over `frontend/` 零命中；只有 SwiftUI 版本且在状态栏）。
- 工作台内打开文件的加载指示：**未找到**（只有独立文件窗口有 `ProgressView("Opening file…")`）。
- 预览态标签视觉区分：**未找到任何视觉区分**（`editorTabTitle` 只渲染 `displayName`）。
- gpui-kit 缺失清单（minimap / overview ruler / sticky scroll / per-tab 关闭与脏点 / 标签拖拽排序 / 换行标签布局 / code lens / gutter 装饰渲染器 / LSP 客户端与传输 / Java grammar）已逐条列在对应表，均需自研或换方案，材料未给出替代实现细节。

### 2.5 Git（macOS 出处：`macos/Sources/Lithe/Views/Git/`、`macos/Sources/LitheGitModule`）

**组成**
- 4 个互不嵌套的宿主：① 底部工具窗「Git」（`workbenchFeature.isVisible(.gitLog)`）；② 左侧边栏 `.changes`；③ 编辑区（三选一顶替编辑器）；④ 顶栏分支切换 overlay。
- 底部工具窗 `GitLogView`（`GitLogView.swift:13`，2 671 行单文件）：`toolWindowHeader` 高 32（`:380-461`）→ `primaryContent` 按 `selectedGitToolTab` 分派 `logTabContent`（`:297-324`）/ `GitWorktreesView`（`:290-291`）/ `GitConsoleView`（`:292-293`）。
  - Log 页签：`primaryActionBar` 高 38（`:526-594`）→ `GitLogThreePaneLayout`（`:326-335`）＝ 引用栏（`:638-703`）＋ 提交栏（`:963-1092`）＋ 详情栏（`:1094-1116`，内含提交文件树与提交详情）。
  - 提交表＋提交图：`GitHistorySelectionGraphView`（`:1037-1043`）→ `GitGraphView`（`GitGraphView.swift:61`）；生产路径是 AppKit 自绘 `GitGraphCommitRowsNSView`（`:448-679`）＋ 泳道 `GitGraphNSView`（`:830-965`）。
- 左侧边栏 `.changes`：`ChangesSidebarView.swift:5`，页签行 40（Commit | Shelf，`:146-172`）→ 提交工具条 37（`:417-492`）→ `CommitAreaView`（`CommitAreaView.swift:4-118`，默认＝最小高 124）＋ 变更列表（`ChangesSidebarView.swift:494-546`）。
- 编辑区（**硬优先级三选一，不是页签**，`Views/Editor/EditorAreaView.swift:63-100`）：`branchComparison != nil` → `BranchComparisonView.swift:4`；`selectedGitCommitDiffContext != nil` → `GitCommitDiffReviewView.swift:7`；`selectedChange != nil` → `DiffReviewView.swift:4`；否则才回真正的编辑器。
- 浮层：`GitFetchDialog`（sheet，`:144-148`）、`GitBranchNameDialog`（`:157-172`）、`GitTagNameDialog`（`:268-272`）、rebase / patch 呈现（`:273-275`）、4 个 `confirmationDialog`（`:173-267`）；顶栏 `BranchSwitcherPopover` 宽 375（`Views/Workbench/WorkbenchView.swift:828-907`）。

**度量**（macOS 数值，可原样搬成 `px()`）
| 元素 | macOS 值 | 出处 |
| --- | --- | --- |
| 工具窗标题栏 | 高 32；左右内边距 12 / 7；HStack 间距 4；底 1px `divider` | `GitLogView.swift:456,454-455,381,457-460` |
| 页签行（Log / Worktrees / Console） | 按钮高 27、圆角 5、选中描边 `inputFocusBorder.opacity(0.72)` 1px；左右内边距 9 / 9（选中 Console 为 4）；关闭按钮 20×27（xmark 8.5 semibold） | `GitLogView.swift:492,514-518,490-491,502-505` |
| 工具条（主操作条 / 三栏工具行 / 详情工具行） | 高 38（统一 `GitVisual.toolbarHeight`）；主操作条左右内边距 10、间距 7、内 Divider 高 18 | `GitLogView.swift:589,588,527,556-557,72` |
| 工具窗背景 / 底线 | `LitheTheme.sidebar` + 1px `divider` | `GitLogView.swift:93,457-460` |
| 字体真源 `GitVisual`（唯一） | title 13.5 semibold / toolbar 12.5 / section 13 medium / body 13 / bodyMedium 13 medium / meta 12 / monoMeta 12 monospaced | `GitLogView.swift:65-79` |
| `GitVisual` 尺寸常量 | `rowHeight` **38**（定义但全文件无引用）、`treeRowHeight` **28**、提交文件加载防抖 120 ms | `GitLogView.swift:73,74,76,1247` |
| 提交表字体（独立于 `GitVisual`） | body 12.5 / meta 11.5 / monoMeta 11.5 monospaced；引用徽标文字 11、图标 12×12 | `GitGraphView.swift:657-660,764,761-762` |
| 提交行高 | **22** | `GitGraphGeometry.swift:8` |
| 提交信息列 | 起点 `textStart`，宽 = 容器宽 − textStart − 230；左对齐、`byTruncatingTail` | `GitGraphView.swift:559-564,631` |
| 作者列 | 起点 `maxX-222`、**宽 104**、**左对齐**、`secondaryText` | `GitGraphView.swift:565-570` |
| 日期列 | 起点 `maxX-118`、**宽 110**、**右对齐**、mono 11.5 | `GitGraphView.swift:571-577,635-640` |
| 行尾内边距 / 行分隔 / 选中 / 悬停 | 尾 8；每行底 1px `divider`；选中 `accent.withAlphaComponent(0.16)`；悬停 `toolHeader.withAlphaComponent(0.55)` | `GitGraphView.swift:737,581-582,552-554,555-558` |
| SwiftUI 死代码版列度量（与上表不一致） | 作者 104、日期 **118**、徽标为 subject 后独立 HStack | `GitGraphView.swift:700-747` |
| 引用徽标（AppKit 自绘） | 高 16、圆角 4、宽 `min(130, max(28, 文本宽+14))`、水平间距 5、右端 `maxX-230` 从右向左堆叠、文字左内边距 7 | `GitGraphView.swift:612-626` |
| 徽标强调色 | head `accent` / branch `success` / remote `rgb(0.55,0.70,0.96)` 硬编码 / tag `warning` | `GitGraphView.swift:775-782` |
| 提交图几何 `GitGraphGeometry` | rowHeight **22**、laneSpacing 16、leftPadding 8、lineWidth 1.5、nodeDiameter 8、graphTextGap 2；宽 `max(1,laneCount,min(6,recommended))*16+2`；箭头热区宽 12、高 `rowHeight/2` | `GitGraphGeometry.swift:8-13,15-27,39-45` |
| 提交图颜色 | 移植 IntelliJ `DefaultColorGenerator`：hue 由 colorID 生成、saturation 0.6、brightness dark 0.6 / light 0.7，colorID 0 → `labelColor`；merge 前景 dark `rgb(111,115,122)` / light `rgb(129,133,148)` | `GitGraphColor.swift:8-27` |
| 三栏分栏 | 引用栏默认 **220** / 最小 **180**；提交栏（中间弹性）最小 **340**；详情栏默认 **350** / 最小 **250**；分隔条可见厚 **5**（命中 10、线宽 1） | `GitLogView.swift:2886-2896`；`Views/Workbench/SplitHandleView.swift:13-14` |
| 详情栏纵向再分 | 文件栏默认 `容器高−5−156` / 最小 90；提交详情最小 110 | `GitLogView.swift:1094-1116` |
| 变更侧栏纵向分 | 工具条 37、列表最小 120、提交区默认＝最小 **124** | `ChangesSidebarView.swift:175-203` |
| 引用树行 | 行高 28、圆角 4、缩进 `depth*16`、图标槽 16；分组行高 28（chevron 8 bold 占 10 宽、图标 14） | `GitLogView.swift:2691-2729,716-724` |
| 提交文件树 | 行高 **28**、垂直 inset 5；folder 缩进 `8+depth*16`、文件名单 `56+max(depth-1,0)*16` | `GitCommitFileTreeView.swift:120-121,338,441` |
| 变更列表 | 行高 24；状态框 17×17 圆角 3、字 9 bold；勾选框 frame 28×24；分组头 24；左缩进 30 / 右 6 | `ChangesSidebarView.swift:6,652-657,639-643,599-601,680-681` |
| 搜索与过滤控件 | 日志搜索框 236×29 圆角 5；过滤器标签高 22；清除按钮 14×22；路径过滤 popover 宽 300 / padding 12 | `GitLogView.swift:984-991,1596,1600-1611,1574-1575` |
| 提交详情 / 提交消息 | 详情 padding 11（subject 13.5 semibold 2 行内、作者 meta、日期 monoMeta）；消息编辑器最小高 50、`editorFont(size: 13)`、内边距 8×7 | `GitLogView.swift:1190-1211`；`CommitAreaView.swift:55`；`LitheTheme.swift:437-439` |
| 控制台 | 工具列宽 28、顶部内边距 6、按钮间距 3；正文等宽 13、水平内边距 12 / 垂直 4、单行最小高 20；follow 阈值 距底 <32 | `GitConsoleView.swift:60-83,128,211`；`GitConsoleEntryView.swift:81,205` |
| Worktree 页签 | 左列表默认 360、快速信息栏 282、出现阈值 1080 | `GitWorktreesView.swift:15-25,120-137` |
| 对话框宽度 | Branch 420 / Tag 420 / CheckoutConflict 460 / IntegrationConflict 460 / Push 720×430 / PullStrategy 560（minH 248）/ Fetch 580 | `GitLogView.swift:2055,2155,2613,2306,2536,2399-2400`；`GitFetchDialog.swift:66` |
| 空态 / 加载更多 | 空态图标 27~30 light、间距 9~12、padding 20~24；"Load more commits" 行高 32、字 11.5 medium、`accent` | `ChangesSidebarView.swift:497-505`；`GitLogView.swift:1135-1165,1045-1063` |
| 底部工具窗最小高 | workbench 侧栏最小 220、git 最小 260 | `WorkbenchView.swift:1969-1982` |

**元素 → gpui-kit 对应**
| macOS 元素 | gpui-kit 实现 | 判定 |
| --- | --- | --- |
| 底部工具窗 + Log / Worktrees / Console 三页签 | `gpui_kit::component::tab::{TabBar, Tab, TabVariant}`（`Tab` 高 32，需 `.h(px(27.))` 覆盖） | 需自行组合 |
| 三页签内容 `switch` 分派 | `gpui_kit::component::tab::TabBar` + 自有 enum | 可一比一 |
| 编辑区「Git 视图顶替编辑器」的硬优先级 | 无对应物，按 `EditorAreaView.swift:63-100` 顺序自写 switch | 需自行组合 |
| 三栏 / 两栏 5px 可拖分隔 | `gpui_kit::component::resizable::{h_resizable, v_resizable, resizable_panel, ResizablePanelGroup}`（`PANEL_MIN_SIZE = px(100.)`，不挡 180/340/250） | 可一比一 |
| 引用栏 35% / 详情栏 50% 动态上限 | `ResizableState::resize_panel` 只维护像素尺寸 | 需自行组合 |
| 提交表（行高 22、无表头、作者 104 左、日期 110 右） | `gpui_kit::component::table::{DataTable, TableState, TableDelegate, Column}`；行高是**表级单值**（`Size::Medium ⇒ 32`，`sizing.rs:57-65`），要 22 得传 `Size::Size(px(22.))`；还需 `.bordered(false)`、`render_header` 返回空 `div()`、行分隔线自画 | 需自行组合 |
| 提交行右键菜单 | `gpui_kit::component::menu::{PopupMenu, context_menu}` + `TableDelegate::context_menu` | 可一比一 |
| 提交行 Cmd / Shift 多选 | `gpui_kit::component::table::TableState`（只有单行 / 单列 / 单格选中，**无多选集合**） | 需自行组合 |
| 提交表虚拟滚动 | `gpui_kit::component::table::DataTable`；或 `gpui_kit::base::virtual_list::{v_virtual_list, VirtualList, VirtualListScrollHandle}` | 可一比一 |
| 提交图（泳道 / 节点 / 箭头 / 虚线 lane） | 无 | gpui-kit 没有 |
| 提交图箭头 12px 热区 | 绝对定位透明 `div()` + `on_mouse_down` | 需自行组合 |
| 引用徽标（圆角矩形 + 图标 + 11pt 文字） | `gpui_kit::component::tag::Tag`（`custom` / `outline` / `rounded`；tag 形图标需自绘） | 需自行组合 |
| 引用树 / 提交文件树（行高 28、两套缩进） | `gpui_kit::base::{TreeState, TreeItem}` + `gpui_kit::component::tree::tree` | 需自行组合 |
| 引用分组折叠 | `gpui_kit::component::collapsible::Collapsible` 或 `gpui_kit::base::accordion::Accordion` | 可一比一 |
| 变更列表（行内勾选框 + 状态方块） | `gpui_kit::component::list::{ListState, ListDelegate, ListItem}`（`selected_index` 是单值 → 多选自研） | 可一比一 |
| 日志搜索框 / 单行输入 | `gpui_kit::component::input::{Input, InputState}` | 可一比一 |
| 提交消息编辑器 | `gpui_kit::component::input::Textarea` + `gpui_kit::base::input::TextareaState` | 可一比一 |
| 过滤器（Date 预设 / Branch flyout） | `gpui_kit::component::dropdown_menu::{DropdownMenu, DropdownMenuItem}` + `gpui_kit::component::menu::PopupMenu` + `gpui_kit::component::searchable_list` | 需自行组合 |
| 对话框 / 危险确认 / popover | `gpui_kit::component::dialog::{Dialog, DialogButtonProps}`、`gpui_kit::component::dialog::alert_dialog`、`gpui_kit::component::popover::{Popover, PopoverState}` | 可一比一 |
| 横幅（rebase session / operationState / stash 冲突 / 删除可恢复） | `gpui_kit::component::alert::Alert` | 可一比一 |
| 提交图 lane 颜色 | `gpui_kit::component::theme::ActiveTheme`（`GitGraphColor` 只产 `NSColor`，改产 `Hsla`） | 需自行组合 |
| 引用种类图标 / Lithe 资源图标 | `gpui_kit::component::icon::{Icon, IconName}` + `gpui_kit::assets` | 可一比一 |
| 空态 | `gpui_kit::component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription, EmptyContent}` | 可一比一 |
| 命令控制台（等宽多行 + 搜索 + 折叠片段） | `gpui_kit::component::input::Textarea`（只读）+ `gpui_kit::component::input::{Input, InputState}` 搜索；折叠与匹配定位自研 | 需自行组合 |
| diff 视图（并排 + gutter + 连接带 + 折叠带 + minimap） | 无 | gpui-kit 没有 |
| 通知 HUD / 加载 / 工具提示 / 计数 | `gpui_kit::component::notification` + `gpui_kit::base::toast` + `gpui_kit::component::spinner::Spinner` + `gpui_kit::component::progress::{Progress, ProgressCircle}` + `gpui_kit::component::tooltip::Tooltip` + `gpui_kit::component::badge::Badge` | 可一比一 |

**复刻要点**
- 先定常量：页签行 27、工具条 38、标题栏 32、提交行 22、引用行 / 文件树行 28、变更行 24、分隔条 5、三栏 220 / 弹性 / 350（`GitLogView.swift:492,589,456,2886-2896`；`GitGraphGeometry.swift:8`；`SplitHandleView.swift:13`）。
- **行高硬约束**：`DataTable` 行高是表级单值（`Size::Medium ⇒ 32`），要正好 22 必须 `Size::Size(px(22.))` 且让单元格自然高度也是 22（见 §1.3）；`GitVisual.rowHeight = 38` 是死常量，实际提交行是 22（`GitLogView.swift:73`；`GitGraphGeometry.swift:8`）。
- **多选必须自研**：`TableState` / `ListState` 只有单选（`table/state.rs:458-562`；`list/list.rs:184-194`）；macOS 的 `selectedHashes` + Cmd / Shift 区间（`GitGraphView.swift:67`；`GitLogView.swift:1303-1313`）要在 delegate 里维护集合并自绘选中底（`accent` 16% / `toolHeader` 55%，`GitGraphView.swift:552-558`）。
- **提交图必须自研** `Element` / `canvas`：几何可直移 `GitGraphGeometry.swift:8-27`，绘制算法在 `GitGraphView.swift:871-917`，颜色 `GitGraphColor.swift:8-27` 是纯数学无依赖。
- 编辑区互斥**顺序不能改**，否则 diff 会盖住 `DatabaseWorkspaceView`（`EditorAreaView.swift:63-100`）。
- 表格去装饰：`DataTable` 默认 `bordered(true)` + 圆角边框要关掉，表头由 `render_header` 返回空 `div()` 去掉，行分隔线自己画（`table/data_table.rs:146-172`；`table/delegate.rs:39-45`；`GitGraphView.swift:581-582`）。
- 三栏最小宽 180 / 340 / 250 要在 panel 上显式设置（`base::resizable` 的 `PANEL_MIN_SIZE = px(100.)` 不会挡住）；35% / 50% 动态上限要自己在 layout 回调里夹（`resizable/mod.rs:14,69-110`）。
- 引用分组折叠可考虑 `Collapsible` 而非 `TreeState`（macOS 是 3 个固定分组 + 组内折叠集合，`GitLogView.swift:23-27`），但组内引用行仍要自己渲染。
- 必须保留的行为：失败自动跳 Console（`GitLogView.swift:140-143`）、提交文件加载防抖 120 ms（`:76,1247`）、日志过滤防抖 180 ms（`:122-131`）、选中行自动滚动（`:1080-1083`）、`↑/↓` 到头不循环（`:1746-1761`）、切仓库清空过滤与滚动（`:132-139`）。
- 引用栏右键菜单有两份逐行重复实现，改动要同时改 `GitLogView.swift:847-960` 与 `:2733-2818`。

**macOS 做得不好 / 用 gpui-kit 优化**
- **提交表两套并存实现、度量不一致**（AppKit 日期 110 与 SwiftUI 日期 118；`GitGraphView.swift:561-577` vs `:726-735`，生产走前者 `GitLogView.swift:1037`）→ 用 `gpui_kit::component::table::{DataTable, TableDelegate, Column}` 的声明式列宽（`Column::{width,min_width,max_width,text_right}`）替掉 `maxX-222` 魔法偏移，并删掉 230 行自绘代码。
- **行高密度 5 套**（22 / 24 / 28 / 37 / 38）→ 收敛成一套密度 token；`DataTable` 的表级行高天然强制统一。
- **「死按钮」**：`gitToolbarIcon(systemImage:help:)` 是没有 action 的 View 却当按钮用（`GitLogView.swift:1721-1727`；`:1003`、`:1121-1123`）→ 一律 `gpui_kit::component::button::{Button, ButtonIcon, DropdownButton}`，每个图标按钮必须传 `on_click`，disabled / hover / tooltip 由组件白送。
- **2 671 行单文件含全部对话框**（`GitLogView.swift:2002,2098,2195,2247,2334,2426,2556,2648`）→ 拆模块；对话框统一 `gpui_kit::component::dialog::{Dialog, DialogButtonProps}`，危险确认统一 `gpui_kit::component::dialog::alert_dialog`（替掉 4 个 `confirmationDialog`，`:173-267`）。
- **右键菜单写了两份**（`:847-960` 与 `:2733-2818`）→ `gpui_kit::component::menu::{PopupMenu, context_menu}` 的 builder 只写一份。
- **失败态只走右下通知，历史加载失败连通知都没有**（`AppModel+GitModule.swift:18`；`WorkbenchView.swift:466-505`；`GitFeatureModel.swift:1761-1762`）→ 用 `gpui_kit::component::alert::Alert` 补内联失败态 + Retry，通知仍走 `gpui_kit::component::notification`。
- **空态与计数手搓**（`GitRepositoryEmptyView.swift:12-53` 三个 if；「N files」手写文本）→ `gpui_kit::component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription, EmptyContent}` + `gpui_kit::component::badge::Badge`。
- **提交消息编辑器靠全局 `NSEvent` monitor 实现点外失焦**（`CommitMessageEditor.swift:90-98,109-114`，deinit 手动摘除）→ `gpui_kit::component::input::Textarea` + `gpui_kit::base::input::TextareaState` 自带 undo / IME / 多行，无全局监听。
- **栏宽无双击重置、无夹取反馈**（`LitheSplitPaneView.swift:38-130`）→ `ResizablePanelGroup::reset_panel`（`resizable/mod.rs:233`）白送双击重置。
- **过滤器 855 行 + 靠 `NSPopover.animates = false` 关动画**（`GitLogFilterPopover.swift:740-908`；`GitLogView.swift:1782-1846`）→ `gpui_kit::component::dropdown_menu::{DropdownMenu, DropdownMenuItem}` + `gpui_kit::component::searchable_list`（自带搜索与空态），动画改由 `gpui_kit::component::theme::motion` 关。

**未查清**
- `toggle-git-log` 的默认键位字符串：`Models/Keymap/LitheCommandCatalog.swift:59` 只声明 id / title / description / scope，**未找到**默认键。
- Windows 侧（`windows/`）的 Git 界面未做对齐核实（调研约束禁止读取）。
- `GitGraphScrollView`（AppKit 版）是否真的完全未被使用：`grep "GitGraphScrollView("` 只命中自身定义（`GitGraphView.swift:140`），未用编译器验证（调研禁止构建），不排除经 `NSViewRepresentable` 泛型 / 反射间接使用。
- `GitCommitFileTreeNSView` 的 `contentWidth` 与水平滚动策略：只读到 `:448` 的 `contentWidth` 计算，`:200-240`（hover tracking 部分）未逐行读取。
- `GitLogFilterPopover.swift` 855 行中的双列 flyout 布局度量（`groupRow` / `flyoutColumn`）未逐行读，只有行号级定位（`:740-908`）。
- `GitWorktreesView.swift` 的 Overview / Changes / Commit History / Settings 四个分段完整度量未逐段读；只确认枚举（`:6-13`）与若干固定高（32/52/34/74/40）。
- `git.consolePresentation` 的 `GitConsolePresentation` 投影数据结构细节（折叠 kind 全集）只从 `GitConsoleEntryView.swift:123-133` 的 `outputLabel` 反推 8 种 kind，未读 `LitheGitModule/Models/GitConsolePresentation.swift`（108 行）。
- 变更列表父路径「→」重命名显示规则（`ChangesSidebarView.swift:851-865`）在 moved + 跨仓库场景的完整语义未验证。

### 2.6 终端 / 运行 / 调试 / 问题 / 输出（macOS 出处：`macos/Sources/Lithe/Views/Terminal/`、`Views/Run/`、`Views/Debug/`、`Views/Language/`、`Views/Workbench/`）

**组成**
- 底部工具窗是 `WorkbenchWorkspaceSplitView` 垂直 split 的**下半**（`WorkbenchView.swift:1570-1584`），内容由 `WorkbenchModuleUIRegistry.selectedToolContent` 按 contributions 顺序取**第一个** `isSelected` 渲染器，兜底 "Starting module…"（`WorkbenchModuleUIRegistry.swift:139-156,160-169`）；切换入口是左侧活动栏下半区图标（`WorkbenchView.swift:1225-1259`、`activityToolButton`:1473-1508），且为**互斥单例** `activeToolWindow: ToolWindow?`（`WorkbenchFeatureModel.swift:36`；`WorkbenchView.swift:1227-1239`）。
- 工具窗枚举 `gitLog`/`terminal`/`references`/`problems`/`maven`/`mavenOutput`/`spring`/`run`/`tests`/`debug`；`maven` 是例外，为**独立右侧 dock**，与底部互斥组并存（`WorkbenchFeatureModel.swift:10-21,37,52-68`）。
- 各工具窗根视图：`TerminalView`（`terminalToolbar`:26-132 + `terminalCanvas`:223-243）、`RunView`（工具窗头部 + `ProjectPreparationStatusView` + 冲突/notice banner + `RunServicesSplitView`）、`MavenBuildOutputView`（`LitheToolWindowHeader` + issueList + `OutputTextView`）、`MavenView`（头部 + 导航工具条 + 项目树）、`GenericDebugView`（header + debugToolbar + contentTabs + inspector/debugConsole）、Problems = `JavaProblemsView`、`LanguageTestsView`、`SpringEndpointsView`（`WorkbenchModuleUIComposition.swift:26-202`；`WorkbenchView.swift:1574-1575`）。
- 终端工具窗三层：`TerminalView` → `terminalToolbar`（标题、终端页签行、`TerminalStatusView`、`plus`/shell 菜单/ellipsis 菜单/`minus`）+ `terminalCanvas`（有会话 `TerminalSurfaceView.padding(8)`，无会话 34pt 图标）（`TerminalView.swift:4,26-131,223-243`）。
- 终端内容区：`TerminalSurfaceView` → `TerminalNativeSurface`（`NSViewRepresentable`）→ `TerminalSurfaceHostView` → `LitheTerminalView : LocalProcessTerminalView`（SwiftTerm exact 1.15.0）（`TerminalSurfaceView.swift:5,12-17,70-143`；`MacTerminalTransport.swift:9`；`Package.swift:33`）。
- 编辑器侧的终端复用同一 `TerminalSurfaceView`（`EditorAreaView.swift:1207-1214`）；终端可在工具窗与编辑区互迁（`moveTerminalToEditor`/`moveTerminalToTool`，`AppModel+Terminal.swift:255-295`）。
- 四套彼此独立的页签行：终端工具窗页签行（`TerminalView.swift:35-50`）、编辑器页签行里的终端页签（`EditorAreaView.swift:475-558`）、Debug 内容页签 Threads & Variables / Console（`GenericDebugView.swift:96-133,2029-2041`）、Run 内容页签 Console / Configuration details（`RunView.swift:736-778`）。
- 头部三套实现：`LitheToolWindowHeader`（Run/Maven/Maven Build/Problems/Tests/Spring 共用）vs Terminal 手写 vs Debug 手写（`LitheToolWindowHeader.swift:31-73`；`TerminalView.swift:27-131`；`GenericDebugView.swift:135-161`）。
- 浮层：输出区 `Copy`/`Jump to latest` 绝对定位浮动胶囊（`OutputTextView.swift:37-46`）、终端启动失败 overlay（`TerminalSurfaceView.swift:23-35`）、右键菜单（自绘 `NSPanel`，`LitheContextMenu.swift:5-13`）、关闭运行中终端的 `confirmationDialog`（`RootView.swift:229-242`）、智能步入空状态（`GenericDebugView.swift:569-576`）。
- 输出控制台：Run / Maven / Debug / Tests 共用 `OutputTextView`，各持内存字符串缓冲与清空按钮，**无通用 output channel 抽象**（`WorkbenchFeatureModel.swift:10-21`；`OutputTextView.swift:11`）；终端无字符串缓冲，走 PTY 原生渲染（`TerminalSession.swift:134`）。

**度量**（macOS 数值，可原样搬成 `px()`）
| 元素 | macOS 值 | 出处 |
| --- | --- | --- |
| 工具窗头部 `LitheToolWindowHeader` | `HStack(spacing: 8)`；leading 12 / trailing 7；高 `Metrics.toolWindowHeaderHeight`(30)；底 `toolHeader`；图标 `LitheIDEAIcon(13)` 或 SF Symbol 12pt；标题 12.5pt semibold；副标题 11.5pt medium；`Spacer(minLength: 12)`；动作槽 `minus` | `LitheToolWindowHeader.swift:31-66`；`LitheTheme.swift:417` |
| 头部例外（Terminal / Debug 手写） | Terminal `HStack(spacing: 8)`，leading 12 / trailing 7，高 30，底部 1pt `divider`；Debug `HStack(spacing: 10)`，高 34 | `TerminalView.swift:27,125-131`；`GenericDebugView.swift:136,157-160` |
| 度量漂移（勿照抄局部常量） | Debug 头部 34 / 工具条 36 / 图标 18；终端页签高 26；Debug 会话页签 25、内容页签 25（条 32）；Run 内容标题 34、页签行 30、页签按钮 30；Maven 工具条 36 | `DebugToolbarPresentation.swift:38-40`；`TerminalView.swift:150`；`GenericDebugView.swift:110,131,203`；`RunView.swift:735,746,770`；`MavenView.swift:128` |
| 终端页签行容器 | `ScrollView(.horizontal, showsIndicators: false)` + `HStack(spacing: 3)` + `.padding(.horizontal, 2)`；溢出只有横向滚动，无溢出菜单 | `TerminalView.swift:35-43` |
| 终端页签 | 高 26；宽由内容决定（无 min/max）；leading 9 / trailing 5；图标 `terminal` 10pt；标题 11.5pt（active semibold / 否则 medium）；关闭 `xmark` 9pt semibold 22×22 常驻 | `TerminalView.swift:137-183` |
| 终端页签活动态 / 拖拽预览 | 活动态：底 `subtleSelection` + `inputFocusBorder` 1pt 描边圆角 5，页签行无 hover 态；拖拽预览：图标 11pt `accent`、文本 12pt medium、padding.h 10、高 34、底 `activeTabBackground`、圆角 5、阴影 black 0.42/r10/y6；插入按半宽判定 | `TerminalView.swift:184-189,137-183,245-260,352-363`；`TerminalTabDragPayload.swift:5-26` |
| 编辑器终端页签（对照） | `HStack(spacing: 7)`、leading 11、高 34、图标 11pt；活动 = `activeTabBackground` + 底部 2pt `accent`；关闭 20×20，仅 active/hover 可见 | `EditorAreaView.swift:480-492,519-546` |
| Debug / Run 内容页签 | Debug：条高 32、`padding.horizontal 8`、底 `toolHeader`，页签 11.5pt medium、padding.h 11、高 25、圆角 5，选中 = `selection` 底 + `accent.opacity(0.65)` 1pt 描边；Run：条高 30、padding.h 12、`spacing 12`、底部 1pt 分隔，页签 11pt，选中 semibold + `accent` 文字 + 底部 2pt 下划线 | `GenericDebugView.swift:97,103-132`；`RunView.swift:736-747,765-778` |
| 终端内容区 chrome / 启动失败 overlay | 内边距 8（编辑器侧同值）；底 `editor`（工作台有背景图时透明）；空会话仅 34pt `terminal` 图标 ultraLight `tertiaryText`，**无文案**；启动失败 = "Unable to start terminal" headline + 错误文本 + 引导句，`padding(20)` 居中 `secondaryText` | `TerminalView.swift:229,230-233`；`TerminalSurfaceView.swift:19-22,23-35`；`EditorAreaView.swift:1213` |
| 终端渲染参数 | 字体 MesloLGS/JetBrainsMono/Hack/FiraCode/IosevkaTerm Nerd Font，size 12.5（回退系统等宽 12.5）；scrollback 2000 行；前景/caret/选中为字面 sRGB；`TERM=xterm-256color` | `MacTerminalTransport.swift:153-161,279,284-301`；`TerminalSession.swift:160-162` |
| `TerminalStatusView`（仅活动会话） | `TimelineView(.periodic, by: 1)`；运行点 `Circle` 6×6（running 绿 / 否则 `secondaryText`）；`Exit N`（0 绿 / 非 0 orange）；10.5pt medium、`frame(maxWidth: 240, .trailing)`、`lineLimit(1)` | `TerminalView.swift:268-305` |
| `litheIconButton()`（头部动作公共样式） | `frame(28, 28)`、圆角 `Metrics.cornerRadius`(5)、前景 `toolWindowText`、按下 `pressedBackground` / 悬停 `hoverBackground` | `LitheTheme.swift:479-497` |
| 输出区浮动胶囊 Copy / Jump to latest | 11pt medium、padding.h 10 / v 5、`Capsule`、底 `raised.opacity(0.9/0.92)`、`panelBorder` 1pt；`topTrailing+8` / `bottomTrailing+10`；禁用 `opacity(0.4)`；仅 `!isAtBottom` 显示；贴底阈值 80 | `OutputTextView.swift:21,37-46,307-346` |
| 输出文本区 | `NSTextView` `textContainerInset(12,12)`、只读可选非富文本；`Menlo` 11.5（粗体 `Menlo-Bold`）；链接 `accent` + 单下划线 | `OutputTextView.swift:61,486-500,713-719` |
| Problems 行 / 空态 | 行 padding.h 10 / v 6、`LazyVStack(spacing: 2)`、图标宽 16、标题 11.5pt、source/reason 10.5pt、message 11pt `lineLimit 3`；空态图标 28pt light、标题 13.5pt semibold、`VStack(spacing: 9)` | `JavaProblemsView.swift:111-152,154-180` |
| Maven Build issues 列表 | `frame(maxHeight: 132)` 不可折叠、`LazyVStack(spacing: 2)` + v5、行 padding.h 10 / v 6、图标宽 15、标题 11.5pt、消息 11pt `lineLimit 2`、底 `sidebar` | `MavenBuildOutputView.swift:86-121` |
| Run 行度量 | 会话行 `minHeight 34`、padding.h 8、圆角 4、图标 16、状态点 6×6 + 1pt `sidebar` 描边、标题 12pt、副标题 10.5pt、pin 20×24；scope 行高 30；分组头高 28；状态胶囊高 22（10.5pt semibold、`color.opacity(0.10)`）；详情行 label 宽 118 右对齐 | `RunView.swift:507-530,571-600,888-905,924-945,995-1092` |
| Debug 行度量 | 区段头高 27（标题 10.5pt semibold、计数 9.5pt monospaced）；栈帧行 `minHeight 27`；控制台输入行高 34；状态胶囊高 22；工具栏按钮图标 18 / `frame(32,30)` / 禁用 `opacity(0.36)`；分隔 `1×18`；inspector 左 min240·ideal320、右 min300·ideal480 | `GenericDebugView.swift:338-361,460-465,467-504,595-604,624,696,1212-1258,1352-1365` |
| 右键菜单（自绘 `LitheContextMenu`） | 根宽 230…360、子菜单 220；项 12pt / 快捷键 11pt；行高 26、分隔 11、列垂直 padding 12、子菜单间距 1；行 `HStack(spacing: 9)` + 图标 16×16；`chevron.right` 9pt | `LitheContextMenu.swift:5-13,175-176,239-269` |
| 关闭确认对话框 | "Close Running Terminal?" / "Close Terminal"（destructive）/ "Cancel" / "Closing this terminal will stop its shell and any running command." | `RootView.swift:229-242` |

**元素 → gpui-kit 对应**
| macOS 元素 | gpui-kit 实现 | 判定 |
| --- | --- | --- |
| 底部工具窗容器（互斥单例 + 高度可拖） | `gpui_kit::base::dock::DockArea` + `DockPlacement::Bottom`（`gpui-base-0.6.6/src/dock/state.rs:161-185`）、`DockArea::set_dock`（`dock_area.rs:246`） | 可一比一 |
| 工具窗面板契约（标题、工具条按钮、ellipsis、缩放、内边距） | `gpui_kit::component::dock::Panel`：`title`/`title_suffix`/`toolbar_buttons`/`dropdown_menu`/`zoom_control`/`inner_padding`/`title_bar`（`gpui-component-0.6.6/src/dock/panel.rs:71-149`） | 可一比一 |
| 活动栏图标切换工具窗 | `gpui_kit::component::button::Button`（icon + tooltip）、`gpui_kit::component::button::Toggle` | 可一比一 |
| 工具窗头部（标题 + 副标题 + 动作 + minimize，含 Terminal/Debug 手写头部） | 无专用组件；`gpui_kit::base::v_flex`/`h_flex` + `gpui_kit::component::label::Label` + `Button::ghost` | 需自行组合 |
| 终端页签行 | `gpui_kit::component::tab::TabBar`（`underline`/`max_width`/`track_scroll`/`menu`/`prefix`/`suffix`）+ `gpui_kit::component::tab::Tab`（`tab/tab_bar.rs:40-171`、`tab/tab.rs:397`） | 可一比一 |
| 终端页签关闭按钮 | `Tab::suffix(IconButton)`（`tab/tab.rs:541`）；tab 模块无 `closable`/`on_close` | 需自行组合 |
| 终端页签拖拽重排（UTType `com.lithe.terminal-tab` 载荷） | `gpui_kit::base::dock::TabGroup`：`drag_panel`/`drop_panel`/`drop_item`/`close_panel`（`gpui-base-0.6.6/src/dock/tab_group.rs:146,246,855-876`）；tab 模块自身不支持拖拽 | 需自行组合 |
| 工具条（Debug 9 动作 + 分隔） | `gpui_kit::component::button::ButtonGroup::layout(Axis)` 承载 `Button` + `gpui_kit::component::separator::Separator`，`Toggle` 用于 muteBreakpoints；无独立 `Toolbar` 组件 | 需自行组合 |
| 下拉 / shell 选择 / ellipsis 菜单 | `gpui_kit::component::menu::DropdownMenu`、`PopupMenu`/`PopupMenuItem` | 可一比一 |
| 右键菜单（页签 / 终端内容 / 栈帧 / 线程） | `gpui_kit::component::menu::ContextMenuExt::context_menu` + `PopupMenuItem::{separator,disabled,checked,submenu,icon}` | 可一比一 |
| 右键菜单固定度量（230/220/360/26/12/11） | `PopupMenu::{min_w,max_w,max_h}` + `gpui_kit::base::SpacingTokens`；无这套常量 | 需自行组合 |
| 计数徽标 / 运行指示点（6×6） | `gpui_kit::component::badge::Badge::{dot,count,max,icon,color}` | 可一比一 |
| 状态胶囊（Running/Finished/Failed/Not run） | `gpui_kit::component::tag::Tag` 或 `Badge` + `gpui_kit::component::theme::ThemeColor::{success,warning,danger,secondary}` | 可一比一 |
| 头部/工具条/内容分隔线 | `gpui_kit::component::separator::Separator` | 可一比一 |
| 输出文本区（等宽、只读、可选、复制） | `gpui_kit::component::input::Editor`（`readonly(true)`）+ `gpui_kit::component::input::EditorState`；纯只读文本可用 `gpui_kit::base::SelectableText`；**没有 ANSI→样式解析器** | 需自行组合 |
| 输出的贴底跟随 + Jump to latest | `gpui_kit::component::message_scroller::{MessageScroller, MessageScrollerState}`：`is_following_tail`/`is_scrolled_up`/`jump_button`/`with_jump_button_label`/`with_bottom_fade` | 可一比一 |
| 输出 / 问题列表滚动 | `gpui_kit::component::list::{List, ListState, ListDelegate, ListItem}` + `gpui_kit::component::VirtualList`/`v_virtual_list` | 可一比一 |
| Run 配置 / Problems 表格式展示（可选） | `gpui_kit::component::table::{DataTable, TableState, TableDelegate, Column}` | 可一比一 |
| Maven 依赖·生命周期树 / Debug 线程栈 | `gpui_kit::component::tree::{Tree, tree}` + `gpui_kit::base::{TreeState, TreeItem, TreeEntry, TreeEvent}` | 可一比一 |
| Run 三栏 / Debug inspector 分栏 | `gpui_kit::component::resizable::{ResizablePanelGroup, ResizablePanel, ResizableState, h_resizable, v_resizable, resizable_panel}`（`gpui-component-0.6.6/src/lib.rs:68-73`） | 可一比一 |
| 拖拽分隔条外观 | `gpui_kit::base::ResizeHandleRenderer` / `ResizablePanelGroup::with_handle_appearance`；component 侧 `resize_handle()` 是 `pub(crate)` | 需自行组合 |
| 不确定进度 / 空状态 / 通知 / 确认对话框 / tooltip | `gpui_kit::component::progress::Progress::loading(true)` 或 `spinner::Spinner`；`empty::{Empty,EmptyHeader,EmptyMedia,EmptyTitle,EmptyDescription,EmptyContent}`；`notification::Notification` + `Root::push_notification`；`dialog::{Dialog, AlertDialog}` / `sheet::Sheet`；`tooltip::Tooltip` | 可一比一 |
| 图标（IDEA SVG / SF Symbol） | `gpui_kit::component::icon::{Icon, IconName, IconNameExt}` + `gpui_kit::assets::{Assets, AllAssets, icon_assets!}`（内置 Lucide；IntelliJ SVG 需自备资源包） | 可一比一 |
| 主题色 / 字号 / 等宽字体 | `gpui_kit::component::{Theme, ThemeColor, ActiveTheme}` + `gpui_kit::base::{SemanticThemeTokens, ColorTokens, RadiusTokens, SpacingTokens, TypographyTokens}` | 可一比一 |
| 工具窗 toggle 快捷键绑定 | `gpui::KeyBinding::new` + `App::bind_keys` + `gpui_kit::actions!` + `Context::on_action`/`Window::on_action` | 可一比一 |
| 终端内容区（VT 解析、行网格、光标、回看、鼠标模式） | **没有任何对应类型** | gpui-kit 没有 |

**复刻要点**
- **终端零跨端契约**：`shared/contracts/rust-core-api.md:81-210` 的命令表里**无任何 `terminal.*`**；`:315-316` 与 `shared/contracts/application-boundary.md:35` 明确把 PTY/ConPTY、shell 发现、原生偏好、环境划归平台适配器；Run/Debug 的进程启动、输出、退出码同样由平台 `StreamingProcess` 承担（`application-boundary.md:34,151-156`）。所以 PTY 与 shell 发现放 Rust 平台层，Core 不参与。
- **gpui-kit 没有终端模拟器 / PTY / ANSI 解析**：`gpui-component-0.6.6` 全量搜 `\bTerminal\b|terminal|alacritty|portable_pty|conpty` 零命中，`Cargo.toml:268-556` 依赖无终端/PTY/VTE crate，`gpui-kit-0.6.6/src` 只有 `lib.rs`+`test.rs`，唯一 "terminal" 是图标 `square-terminal.svg`（`gpui-kit-assets-0.6.6/default-icons.txt:87`）；对照 macOS 由 SwiftTerm 提供（`Package.swift:33`；`MacTerminalTransport.swift:9`）。
- **VT 渲染必须自写 GPUI `Element`**（`portable-pty` + `vte`/`alacritty_terminal`）：需要 SGR 解析与调色、行列网格重绘、光标与焦点闪烁、2000 行回看、OSC 标题/目录/链接、PTY 生命周期（`MacTerminalTransport.swift:57-63,80-127,279,327-396,400-414`；macOS 自制 `ANSIOutputRenderer` 见 `OutputTextView.swift:675-879`），可参考 `gpui-base-0.6.6/src/input/base/element.rs:1021` 的行号布局写法。
- **UI 照搬 macOS 的字段契约**：`TerminalTransport`（`isRunning`/`processID`/`shellName`/`nativeView`/`onTermination`/`onOutput`/`onTitle`/`onDirectoryUpdate`/`onLink`，`TerminalTransport.swift:39-61`）与 `TerminalSession`（`isRunning`/`isReady`/`isManagedProcess`/`launchError`/`shellName`/`processTitle`/`currentDirectory`/`lastExitCode`/`startedAt`/`endedAt`，`TerminalSession.swift:7-21`）；可移植语义走 `shared/contracts/terminal-profiles.md:3-33` 与 `shared/fixtures/terminal/shell-selection-v1.json`。
- **chrome 与模拟器本体要分层**：工具窗头部、页签行、状态条、空态是可移植的 gpui 层；只有内容区 `TerminalSurfaceView`（`editor` 底 + 8pt padding + 原生宿主 + 启动失败 overlay + 右键菜单）属平台层（`TerminalSurfaceView.swift:5-60`）。切页签必须「持久原生视图 + 宿主重挂载」，否则 TUI 屏幕重置（`TerminalSurfaceView.swift:110-143`；`MacTerminalTransport.swift:224-227`）。
- **必须原样保留 Debug 的 `runInTerminal` 链路**：Core 产出 `runInTerminalRequested` + `requestId`，平台用 PTY 启动后回 `debug.runInTerminalResponse`（`rust-core-api.md:993-1003`；`AppModel+Terminal.swift:96-144`），否则调试目标 stdin/stdout 会断；`runInTerminal` 拒绝 external terminal 与 shell 解释参数（`AppModel+Terminal.swift:372-397`）。
- 不要照抄局部常量：同一产品内头部 30/34、工具条 40/36、页签 34/30/26/25/32 并存（见度量表），复刻按 `Metrics` 统一。
- 状态与文案：终端头部状态条只覆盖**活动**会话；`displayTitle`/`displayDirectory`/`elapsedDescription`（`mm:ss` 或 `h:mm:ss`）在 `TerminalSession.swift:51-61`；关闭运行中终端走确认对话框（`RootView.swift:229-242`）；活动页签被关后优先取后一个否则取前一个（`TerminalFeatureModel.swift:68-77`）；空状态文案见 `TerminalView.swift:230-233`、`MavenBuildOutputView.swift:79`、`RunView.swift:44,414,547,755`、`JavaProblemsView.swift:154-180`、`GenericDebugView.swift:2577`。
- 快捷键：16 条相关命令里 7 条工具窗 toggle **全无默认键**（`LitheCommandCatalog.swift:56-62`），Run/Debug/断点/步入有键（`:21-30`）；只有 Debug 工具栏 tooltip 拼 `"\(title) (\(shortcut))"`（`GenericDebugView.swift:430-450`）。

**macOS 做得不好 / 用 gpui-kit 优化**
- **没有底部工具窗页签行**：切换靠活动栏图标（`WorkbenchView.swift:1227-1239`）+ 底部互斥单例（`WorkbenchFeatureModel.swift:62-68`），IDEA 的 Terminal/Problems/Output 页签组不存在 → `gpui_kit::base::dock::DockArea` + `DockPlacement::Bottom` 直接得到页签组 + 拖拽 + 缩放 + 布局 `dump`/`load`（`dock_area.rs:716,794`）。
- **三套头部实现且度量不一致**（Terminal `TerminalView.swift:27-131`、Debug `GenericDebugView.swift:135-161`、`LitheToolWindowHeader`；30/34/30）→ Dock `Panel` 已把 `title`/`title_suffix`/`toolbar_buttons`/`dropdown_menu`/`zoom_control` 收敛成统一槽位（`dock/panel.rs:71-149`）。
- **终端无分屏、无重命名**（标题只跟随 OSC 标题）、**无固定/ZOOM** → `TabGroup` 支持组内拆分与拖拽（`tab_group.rs:855-876`）；Dock 内建 `actions!(dock, [ToggleZoom, ClosePanel])` 与 `DockArea::set_zoomed_in`（`dock/mod.rs:59`、`dock_area.rs:626`）。
- **页签溢出只有横向滚动**（`TerminalView.swift:35-43`），截断标题无 tooltip、无溢出菜单 → `TabBar::menu(true)` + `TabBar::max_width`（`tab/tab_bar.rs:110,118`）。
- **页签状态反馈缺失**：运行点只在活动会话的头部状态条（`TerminalView.swift:52-54`），页签行**无 hover 态**（`:137-183`），关闭按钮常驻 22×22（`:172-182`，编辑器页签却是 hover 才显示，`EditorAreaView.swift:531-532`）→ `Badge::dot()` 挂到每个 `Tab::suffix`（`badge.rs:56`）+ 主题 `list_hover`。
- **`TerminalStatusView` 每秒重建整个 HStack**（`TimelineView(.periodic, by: 1)`，`:272`）→ 改为仅在 `elapsedDescription` 变化时更新。
- **裸色绕过主题**：`Color.green`/`Color.orange`（`TerminalView.swift:275,289`）、`Color.orange.opacity(0.10)`（`RunView.swift:380`）、INFO 级别字面 sRGB（`OutputTextView.swift:103`）、终端前景/caret/选中字面 sRGB（`MacTerminalTransport.swift:153-161`）→ 统一用 `ThemeColor` 语义色。
- **Problems 五级严重度里 `hint` 与 `unknown` 同色、且与次要文字同色**（`JavaProblemsView.swift:182-190`）；列表用 `LazyVStack` + 每行一个 `Button`，**无虚拟化**（`:17-24,108`）→ `ThemeColor` 有独立 `info`/`muted_foreground`；`gpui_kit::component::list::{List, ListDelegate}` + `VirtualList` 替代表级重建。
- **Maven Build issues 硬上限 132pt 且不可折叠**（`MavenBuildOutputView.swift:119`），问题多时挤掉输出 → `resizable` + 折叠/`ResizablePanel::visible`。
- **输出区三处低效**：复制时重跑整段 ANSI 解析（`OutputTextView.swift:312`）、增量更新是 `previous != next` + `hasPrefix` 的整串比较（`:362-366`）、贴底是手写 `bottomThreshold = 80`（`:21`）→ `message_scroller` 的 `append`/`splice`/`prepend` + `is_following_tail`/`is_scrolled_up`/`jump_button`/`with_bottom_fade`（`message_scroller.rs:58-65,80-115,221-284`）。
- **Copy / Jump to latest 是两个绝对定位浮动胶囊**，遮挡正文右上/右下角，空输出时 Copy 仍可点（`OutputTextView.swift:37-46`）→ 放进 `Panel::toolbar_buttons`（`dock/panel.rs:101`）。
- **右键菜单项严重不全且三处分叉**：终端页签只有 `Move to Editor`+`Close`（`TerminalView.swift:209-219`），缺 `Split`/`Rename`/`Close Others`/`Close All`/`Copy Path`；原生 surface 又用 AppKit `menu(for:)` 且**未本地化**（`MacTerminalTransport.swift:129-142`）→ 统一 `ContextMenuExt::context_menu` + `PopupMenuItem::*`（分隔/禁用/勾选/子菜单全有），快捷键提示用 `Kbd::binding_for_action`（`kbd.rs:53`）而非手拼字符串。
- **无通用 output channel 抽象**：Run/Maven/Tests/Debug 各写一套缓冲与清空按钮（`WorkbenchFeatureModel.swift:10-21`）→ 自建 `OutputChannel`（追加块/清空/复制/上限/时间戳）；活动栏也没有问题总数徽标 → `Badge::{count,max,color}` 挂活动栏按钮（`WorkbenchView.swift:1283-1289`）。
- **无默认快捷键 + 禁用态不统一**：7 条 toggle 无键（`LitheCommandCatalog.swift:56-62`）→ `gpui_kit::actions!` + `App::bind_keys` 补齐；禁用态只有手写 `opacity(0.36)`（`GenericDebugView.swift:357`）→ `Disableable` + 主题禁用 token。
- **本区域几乎无动效**（唯一 `RunView` pin 切换 `easeInOut 0.18`，`RunView.swift:442`；页签切换与工具窗显隐都是瞬变）→ `theme::motion`/`MotionTokens` 补 150/200ms 过渡（相对 macOS 是能力增强，不是一比一）。

**未查清**
- 终端分屏（split）**未找到**（全仓 `splitTerminal`/`TerminalSplit` 无匹配）。
- 终端重命名**未找到**（无 rename API，标题只跟随进程 OSC 标题；全仓 `renameTerminal`/`Rename Terminal` 无匹配）。
- 底部工具窗统一页签行**未找到**（靠活动栏切换 + 互斥单例）。
- 活动栏上的问题总数徽标**未找到**（`badgeBackground` 仅用于 Database 与 GitHub PR 视图，`DatabaseSidebarView.swift:743`、`GitHubPullRequestsView.swift:540/547/1108`）。
- Problems 行 / Run scope 行的右键菜单**未找到**；Problems 值复制**未找到**；终端页签的复制/关闭其他/关闭全部**未找到**。
- 通用 output channel 抽象**未找到**（工具窗枚举里没有 "Output" 项，`WorkbenchFeatureModel.swift:10-21`）。
- `gpui-pre 0.3.6` 是否能嵌入原生 NSView/HWND 子视图：本次**未核实**（只读 gpui-kit/gpui-component，未读 gpui-pre 平台层）——它决定「嵌现有终端」还是「重写渲染」。
- gpui-kit `Editor` 在每秒数十次全量文本替换下的性能：本次**未核实**（无基准证据）。
- gpui-kit 是否有 ANSI SGR → `Hsla` 映射工具：**未找到**（只有 tree-sitter 语法角色的 `highlighter/registry.rs:443-452`）。
- 变量树的右键菜单项完整清单：只核到栈帧与线程两处（`GenericDebugView.swift:665-684`、`:768-782`），变量行（`:806-930`）未逐项核验。
- Debug 断点管理对话框（`DebugBreakpointManagerView`，`GenericDebugView.swift:1518-2028`）的完整度量：本次**未逐项展开**（属 sheet，不在底部工具窗 chrome 范围）。
- `Views/Run/JavaRunConfigurationEditorView.swift`（22KB）与 `MavenView.swift` 细节：只核到对外接口与工具窗头部/工具条，未逐行展开。

### 2.7 搜索 / 语言智能 / 数据库 / 历史 / 诊断 / GitHub / Diff / 社区（macOS 出处：`macos/Sources/Lithe/Views/` 下的 `Search/`、`Language/`、`Database/`、`History/`、`Diagnostics/`、`GitHub/`、`Diff/`、`Community/` 八个目录；数据库传输层在 `macos/Sources/LitheDatabaseModule/`）

**组成**
- 八个区域**没有一个是独立窗口**，全部寄生在工作台或设置窗口里；三类宿主是共同前置：左侧栏多 destination（`SidebarDestination.search|database|pullRequests`，`WorkbenchView.swift:1619-1641`）、底部工具窗 + 模块 UI registry（`WorkbenchModuleUIComposition.swift:99-229`）、编辑区整块替换路由（`EditorAreaView.swift:64-108`，优先级 数据库 > 分支比较 > 提交 diff > 变更 diff > 编辑器）。
- 搜索：侧栏 `SearchSidebarView`（`SidebarDestination.search`，默认宽 320 可拖）；浮层 `SearchEverywhereView`（工作台 overlay，opacity 过渡 0.12s）；自绘模态 `ProjectReplaceView` 装在 `ProjectReplaceFloatingPanel` 里（刻意不用系统 sheet，`WorkbenchView.swift:537` 注释），内含 `ProjectReplacementSourcePreview` 右侧窗格。
- 语言智能：底部工具窗 `ProblemsView`（**类型定义在 `JavaProblemsView.swift:3`**）、`LanguageTestsView`、`LanguageReferencesView`（**定义在 `JavaReferencesView.swift:3`**）；设置窗口面板 `LSPControlCenterView`（`SettingsView.swift:226`）；编辑区浮层 `LanguageImplementationChooserView`（`EditorAreaView.swift:103-108`）。
- 数据库：侧栏 `DatabaseSidebarView`（文件夹 → 连接 → 库 → 表 → 列/索引/外键树）；编辑区 `DatabaseWorkspaceView`（dashboard · SQL · 表 · 结构 · 历史，按 `kind` 分流 Redis / Nacos / MongoDB 专用工作区）；`DatabaseSchemaDiffView` 与 `DatabaseDiagnosticsView` 承载 schema 迁移审批、恢复点/审计、备份计划。
- 历史：`LocalHistoryView` / `ProjectLocalHistoryView`，都挂在 `ActiveSessionChrome` 上以 **sheet** 呈现（`RootView.swift:221-228`）；左栏条目列表 + 右栏 `DiffPaneView`；模块目录另有一个 `history.local` 工具窗贡献但**无 actionID/rendererID，未接线**（`BuiltInModuleCatalog.swift:127`）。
- 诊断：由 **4 个互不相同的界面族**组成 —— A 导出包 sheet `DiagnosticsExportSheet`（两个入口 `RootView.swift:215-220`、`SettingsView.swift:1281`）、B Problems 工具窗、C 运行配置诊断横幅（`RunView.swift:127-148`）、D 数据库内 `DatabaseDiagnosticsView` 子页签。
- GitHub：单文件 2111 行共 25 个类型；侧栏 `GitHubPullRequestsSidebarView` + 编辑区 `GitHubPullRequestDetailView`（直接替换 `EditorAreaView()`）+ 编辑 sheet `GitHubEditPullRequestView` + `GitHubFeatureUnavailableView`。
- Diff：编辑区 `DiffReviewView`（用**自绘标签头** `diffTab`，**不进 `EditorTabItem` 体系**）、`GitCommitDiffReviewView`、`BranchComparisonView`；共享面板 `DiffPaneView`（历史两处 + 分支比较共三处消费）；`DiffSplitPaneView` 并排几何、`DiffMapView` 右侧 14pt 刻度条、`DiffCollapsedBandView` 折叠条带。
- 社区：`LinuxDoCommunityView` 挂右侧 hover 工具窗（`placement: .rightSidebar`，未指定 `rightSidebarBehavior` → 默认 `.hover`，鼠标离开 60ms 自动隐藏，`WorkbenchView.swift:1398-1415`），内部只包一个只读 WKWebView。
- ⚠️ 两处产品级不可达：GitHub 被 `LitheFeatureAvailability.githubPullRequests = false` 整体关闭（`AppModelSupportTypes.swift:55-58`），活动栏条目 `disabled`、两处实例化退化成 `GitHubFeatureUnavailableView()`；`LinuxDoTopicListView` / `LinuxDoTopicDetailView` / `LinuxDoCommunityFormatting` 在整个 `macos/` 树**只有定义、无任何实例化**，7 条 `community.discourse.*` 命令运行期不可达。

**度量**（macOS 数值，可原样搬成 `px()`）
| 元素 | macOS 值 | 出处 |
| --- | --- | --- |
| 工具窗头（Problems / Tests / References / GitHub 共用的 `LitheToolWindowHeader`） | 高 **30**、leading 12 / trailing 7、图标 13（或 SF 12 medium）、标题 12.5 semibold、副标题 11.5 medium、`HStack(spacing: 8)` | `LitheToolWindowHeader.swift:31-66`；`LitheTheme.swift:417` |
| 通用刻度 | rowHeight 24、treeRowHeight 27、treeIconSize 16、treeFontSize 13.5、tabHeight 34；cornerRadius 5 / controlCornerRadius 6 / contextMenuCornerRadius 9 / popupCornerRadius 10 | `LitheTheme.swift:404-423` |
| Search Everywhere | 面板 860×≤560、top 84；标签行 40（项 38、下划线 2）；搜索框 44 / 字号 15；结果行 24、Action 行 24、more 行 22、占位行 34 | `SearchEverywhereView.swift:192-195,254-283,423,489,342,361` |
| 侧栏搜索 / 替换浮层 | 标题行 44、搜索框 32、file mask 28、结果行 v8/h11；替换面板 650×614（最小 520×400）、拖动条 38、把手 18×18、匹配行 46、footer 52、预览头 32 | `SearchSidebarView.swift:27,56,172,111`；`ProjectReplaceFloatingPanel.swift:16-25,100`；`ProjectReplaceView.swift:287,178,90` |
| 数据库侧栏头部 / 搜索 | 头 42（leading 10 / trailing 5）、图标容器 22×22 圆角 5；搜索框 minHeight 31 / h-padding 9 / 圆角 6；过滤·排序按钮 29×31、中间竖线 1×17 | `DatabaseSidebarView.swift:118-144,165-183` |
| 数据库侧栏行高 | 文件夹 31、连接 30、库节点 ≥30、表行 ≥30、`sidebarLeaf` ≥26、DisclosureGroup 子行 23、空表卡片 padding 10、`Loading table metadata…` 24 | `DatabaseSidebarView.swift:420,522,572,659,789,705,626,683` |
| 数据库侧栏缩进阶梯 | 子文件夹 +14 → 文件夹内连接 21+indent → 连接对象树 +24 → 库对象 +18 → 表行 +14 → 列/索引/外键叶 +36 → 对象分区 DisclosureGroup +10 | `DatabaseSidebarView.swift:466-469,538,588,657,685-688,719` |
| 连接状态点 | idle 6×6 `tertiaryText.opacity(0.42)`；connecting 12×12 `ProgressView(.mini)`；connected 7×7 `success`；failed 12×12 `exclamationmark.triangle.fill` 10 semibold warning | `DatabaseSidebarView.swift:1110-1134` |
| 数据表 / SQL 结果网格 | 行高 29 / 27、表头 32 / 29、行号槽 64 / 44、单元格 padding h 7、行列分隔线各 1px；结果网格列宽硬编码 180 | `DatabaseTableView.swift:596,606,536,605,607-614`；`DatabaseSQLWorkspaceView.swift:768,784,775,776,790` |
| 数据表工具条族 / 表标签条 | 工具栏 44、clause bar 34、action bar 38、错误条 ≥28；表标签条 40、单标签 30、圆角 5；结构·诊断工具栏 36、备份历史头 36、Dashboard 导航头 42（segmented 276×28） | `DatabaseTableView.swift:224,251,322,55,1020,999,1005`；`DatabaseSQLWorkspaceView.swift:825,1059,423,74-78` |
| SQL 编辑器 | minHeight 180 / ideal 240 / max 320；等宽 12.5pt；`textContainerInset` 11×9；初始 frame 900×240；查询标签条 32、标签 31、命令条 37 | `DatabaseSQLWorkspaceView.swift:656,1268-1270,1265,568,541,639` |
| Redis / Nacos 专用工作区 | Redis 头 44、键行 40、键图标 22×22 圆角 5、详情图标 28×28 圆角 7、值编辑器 ≥230、列表栏 250/300/350；Nacos 头 44、列表栏 285/340/390、配置行 42、底部发布条 48 | `DatabaseSpecializedWorkspaceViews.swift:101,163,153-155,196-198,264,23,436,477,469,509` |
| 数据库浮层 | 连接编辑器 500×（sqlite 640 / 数据网格 750 / 其余 710）、Header 62 / Footer 58；文件夹编辑器 390×240；DBX 导入 680×560、Header 58 / 摘要 54 / Footer 54；Schema Diff 工具栏 38、内容标题条 42 | `DatabaseSidebarView.swift:1870,1741,1847,1644,1614,1641,1315,1278,1378,1312`；`DatabaseSchemaDiffView.swift:44,91` |
| 历史 sheet | 最小 980×620（工程版 1120×680）；左栏 290 / 350；header 42、小节头 34、条目行 ≥52 / ≥58、条目一级 12 medium、二级元数据 10.5 | `LocalHistoryView.swift:20,15,72,81,113,115-122`；`ProjectLocalHistoryView.swift:20,15,129` |
| 诊断导出 sheet / Problems 行 | sheet 固定宽 460、padding 20、spacing 16、内容区 minHeight 120、清单字号 12 mono；Problems 行 padding h10/v6、严重度图标槽 16、消息 11 `lineLimit 3`、related 10.5 | `DiagnosticsExportSheet.swift:8,34-35,18,65-74`；`JavaProblemsView.swift:111-145` |
| GitHub 列表 / 详情 | PR 行 padding h8/v8、标题 11.5、元信息 9.5；详情页头 56、段栏 40（tab 30）、账号头 46、搜索框 27（外 padding 10/9）、文件行 38；概览 maxWidth 880、文件·创建页 980（创建页 padding h34/v28）、编辑表单 590；头像 28 / 27 / 30 | `GitHubPullRequestsView.swift:857-858,830,834-841,523,569,255,301,1028,640,1380-1382,2091,224,1066,1617` |
| Diff 行几何 / 标签头 / 工具条 / map | 行高 24、information·hunk 27、折叠带 27、中缝 34、行号列 47（单文件 55）、标记竖条 3、文本 padding 8（单文件 10）、代码 12.5 mono、画布最小宽 980 / 680；标签头 34（accent 下划线 2）、工具条 40、版本头 34；map 宽 14 / 标记最小占比 0.004；横向滚动条 track 3 / thumb 5 / 最小宽 46 | `DiffReviewView.swift:1020-1047,332-333,53-97,224-289`；`DiffCollapsedBandView.swift:23`；`DiffMapView.swift:27,31`；`DiffHorizontalScrollSupport.swift:27,36,44` |
| 社区 | header 30（同工具窗头，未传 `onMinimize`）；失败态 spacing 14 / padding 28；注入 CSS 固定 `#17181c` 深色底、`.cooked` 14px/1.62、`.topic-list` 13px | `LinuxDoCommunityView.swift:34-56,58-80`；`LinuxDoAnonymousWebView.swift:169,177-183` |

**元素 → gpui-kit 对应**
| macOS 元素 | gpui-kit 实现 | 判定 |
| --- | --- | --- |
| 三类宿主容器（侧栏 destination / 底部工具窗 + 模块 registry / 编辑区整块替换） | `gpui_kit::component::dock::{DockSkin, DockArea, Panel, PanelView, register_panel}` + `gpui_kit::component::sidebar::{Sidebar, SidebarMenu, SidebarMenuItem}` + `gpui_kit::component::resizable::{h_resizable, v_resizable}` | 可一比一 |
| 连接树骨架（多级 / 多展开 / 定位到当前项） | `gpui_kit::component::tree::Tree` + `gpui_kit::base::{TreeState, TreeItem, TreeEntry}`（`TreeState::reveal_item` 正好补 macOS 缺失的「定位到当前连接/表」） | 可一比一 |
| 连接树行的自定义渲染（行高 31/30/26/23 混杂、色条 / 品牌图标 / 状态点 / 徽标各一） | `TreeState` 的 `render_item` 只能返回 `gpui_kit::component::list::ListItem`（`selected` / `suffix` / `on_click` / `check_icon`） | 需自行组合 |
| 结果表格（数据表 / SQL 结果网格 / Problems 多列） | `gpui_kit::component::table::DataTable` + `gpui_kit::component::table::TableState` + `gpui_kit::component::table::TableDelegate`（`context_menu` / `loading` / `load_more`）+ `gpui_kit::component::table::Column`（`width`/`min_width`/`max_width`/`resizable`/`fixed_left`/`sortable`）+ `TableEvent::ColumnWidthsChanged` | 可一比一 |
| 行号槽 + 行复选框 + 表头三态全选；单元格就地编辑 | `TableState::row_header(bool)` + `SelectionMode::Row`；`gpui_kit::component::input::Input` + `gpui_kit::base::input::state::AnyInputState` 包进 `render_td` | 需自行组合 |
| 长列表（搜索结果 / Tests / References / Redis 键 / Nacos 配置 / 历史条目 / PR 列表 / diff 文件清单）与 provider·Tables 分区头 | `gpui_kit::component::list::{List, ListState, ListDelegate, ListItem}` + `gpui_kit::component::virtual_list::{VirtualList, v_virtual_list}` + `ListDelegate::render_section_header` + `gpui_kit::component::badge::Badge` | 可一比一 |
| 命令面板式浮层（Search Everywhere 的 scope 标签 + `/` 命令模式 + Actions 分组行 + 空态） | `gpui_kit::component::command::Command` + `gpui_kit::component::command::CommandState` + `CommandItem` / `CommandGroup` | 可一比一 |
| 侧栏搜索 / 过滤异步钩子（现状是纯本地同步 `contains`） | `ListDelegate::perform_search` + `ListState::searchable` + `List::search_placeholder` | 可一比一 |
| 图标按钮 / 工具条 / 下拉 / 分段选择器 / 标签条 / 状态标签 | `gpui_kit::component::button::{Button, button_icon::ButtonIcon, dropdown_button::DropdownButton, button_group::ButtonGroup, toggle::Toggle}` + `gpui_kit::component::tab::{TabBar, Tab}` + `gpui_kit::component::select::Select` + `gpui_kit::component::{badge::Badge, tag::Tag}` | 可一比一 |
| 右键菜单（搜索三处、数据库侧栏 11 处、Changes 侧栏、PR 行） | `gpui_kit::component::menu::ContextMenu` + `ContextMenuExt` + `ContextMenuState` + `gpui_kit::component::menu::popup_menu::{PopupMenu, PopupMenuItem}` | 可一比一 |
| sheet / alert / 确认框（连接编辑器、文件夹编辑器、DBX 导入、还原历史、关闭·重开·合并 PR、丢弃 hunk） | `gpui_kit::component::Sheet` + `gpui_kit::component::dialog::Dialog` + `gpui_kit::component::dialog::AlertDialog` | 可一比一 |
| 空态 / 加载 / 骨架 / 进度 | `gpui_kit::component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription}` + `gpui_kit::component::{spinner::Spinner, skeleton::Skeleton, progress::{Progress, ProgressCircle}}` + `ListDelegate::{render_empty, loading}` + `TableDelegate::render_loading`（自带 Skeleton 表格骨架） | 可一比一 |
| SQL 语法编辑器 / diff 逐 token 着色（替换 200 行手写 `DiffSyntaxHighlighter`） | `gpui_kit::component::input::editor::Editor`（`EditorState::set_highlighter`）+ `gpui_kit::component::highlighter::SyntaxHighlighter`（tree-sitter，增量 parse + 缓存） | 可一比一 |
| 键值详情 / 行内选项按钮 / 并排可调分栏 / 筛选·排序 popover | `gpui_kit::component::description_list::{DescriptionList, DescriptionItem}` + `gpui_kit::component::button::toggle::Toggle` + `gpui_kit::component::resizable::{h_resizable, v_resizable, ResizablePanelGroup, resizable_panel}` + `gpui_kit::component::popover::Popover` | 需自行组合 |
| 并排 diff 双栏 + 中缝过渡带 + 词级差异高亮 + 行号 gutter / 变更竖条 | 无对应物（`table::DataTable` 是「列-记录」模型，diff 左右两侧是**独立行流**） | gpui-kit 没有 |
| diff map 14pt 刻度条 / 相对时间格式化 / 树节点拖放与行内重命名 / PR 评论时间线竖轨 | 无 minimap（`plot::Plot` 是图表 trait，画满高刻度属过度设计）；无相对时间组件（`time` 只有 `calendar` / `date_picker`）；组件层无树拖放与内联编辑（`dock::DragPanelPreview` 只服务面板）；`bubble::Bubble` 无竖轨原子 | gpui-kit 没有 |
| 评论·评审输入与 Markdown / HTML 正文渲染 | `gpui_kit::component::input::textarea::Textarea` + `gpui_kit::component::text::TextView`（`::markdown` / `::html`） | 可一比一 |

以下「建议在 gpui 端暂不复刻」逐条列出（原因 + 出处）：
- 系统文件面板类：`fileExporter` / `fileImporter`（数据库导入导出，`DatabaseSidebarView.swift:272-280`）、`NSSavePanel` + `ditto` 打包（`DiagnosticsExportSheet.swift:84-88`、`MacDittoArchiver.swift:7`）、`NSOpenPanel` 选可执行文件（`AppModel+LanguageServerRuntime.swift:6-17`）→ gpui-component 全量源码无文件对话框/文件系统模块（仅 `dialog` 可做确认框）→ **建议在 gpui 端暂不复刻**，改由宿主注入路径、打包用 `zip`。
- Keychain 凭据存储：`hasSavedPassword`（`DatabaseSidebarView.swift:1866`）、LINUX DO 服务名 `app.lithe.desktop.linux-do`（`MacServiceContainer.swift:122`）→ 平台安全存储差异，Windows 需另设凭据后端 → **建议在 gpui 端暂不复刻**。
- Homebrew 安装器与可执行文件探测校验：`installWithHomebrew` 直接 `brew install`（`LanguageServerToolService.swift:248-254`，超时 600_000ms、校验 5_000ms）→ 平台安装器能力，且 `LanguageServerSetupView` 当前**未挂载** → **建议在 gpui 端暂不复刻**。
- LSP 进程生命周期（`lsp.startServer` / `stopServer` / `destroyServer` / `retryMavenProfiles`）：依赖 Rust Core 会话注册表与 30s 初始化 / 2s 关闭超时（`RustCoreBridge.swift:3634-3636`）→ 是进程与 IPC 能力而非 UI 组件 → **建议在 gpui 端暂不复刻**，gpui 只渲染状态并用 `Panel` 承载。
- 外部浏览器打开（GitHub 链接与 `Open GitHub profile`，`GitHubPullRequestsView.swift:194,242,498,687`）→ `link::Link` 只有 `href`、未承诺等价 API，硬接会引入平台分支 → **建议在 gpui 端暂不复刻**。
- AppKit 光标校正：`ProjectReplaceCornerHandle` 的 `NSTrackingArea` / `resetCursorRects` / `disableCursorRects` / `endTracking`（`ProjectReplaceCornerHandle.swift:52-105`）→ 复杂度全在对抗 AppKit 拖动中重置 cursor rect，gpui 的 cursor 由元素样式声明 → **建议在 gpui 端暂不复刻**（改用元素级 `.cursor(CursorStyle)`）。
- 全局键盘 / 滚轮监视器：`SearchEverywhereView` 的 `NSEvent.addLocalMonitorForEvents(.keyDown)`（`SearchEverywhereView.swift:512-540`）、`ProjectReplaceKeyMonitorView` 的 Cmd 组合白名单（`WorkbenchView.swift:129-155`）、`DiffHorizontalScrollWheelMonitor` 的 `.scrollWheel`（`DiffHorizontalScrollSupport.swift:126-141`）→ gpui 有 `KeyBinding` + scoped `actions!` 与 `on_scroll_wheel` → **建议在 gpui 端暂不复刻**。
- 品牌图标的 AppKit 归一化（`subdirectory: "DatabaseIcons"`、显式 `NSImage.size`、`isTemplate = false`、`.renderingMode(.original)`、SF Symbol 兜底，`DatabaseBrandIcon.swift:46-85`）→ 属 AppKit 菜单桥接读 intrinsic size 的特有绕行 → **建议在 gpui 端暂不复刻**（SVG 加载本身可一比一）。
- `diffTab` 标签头（`DiffReviewView.swift:53-97`）与 diff map 刻度条（`DiffMapView.swift:27`）：无 diff 页签组件、`tab::Tab` 是文档页签语义不匹配；无 minimap 组件，用 `canvas` 直接绘制更省 → **两者建议在 gpui 端暂不复刻**；`DataTable` 表格方案对 diff 也不适用 → **建议暂不复刻表格方案**。
- WKWebView 内嵌浏览器 + Cloudflare cookie 持久 + 注入 CSS + `LinuxDoIdleRetainer` 10 分钟空闲释放（`LinuxDoAnonymousWebView.swift:70,116-128,142-189`）→ gpui-kit 无 web view 组件，且设备校验、`NSWorkspace.open`、`lithe://` scheme 回调均为 macOS 平台能力 → **建议在 gpui 端暂不复刻**。

**复刻要点**
- **架构事实（最高优先级）**：整个**数据库区域不走 Rust Core**，而走**独立 sidecar 子进程的 JSON-RPC** —— 每调用 fork 一次进程，stdin 送 `RequestEnvelope{id, method, params}`，stdout 收 `ResponseEnvelope{id, ok, result, error}` 并校验 `id`（`DatabaseSidecarService.swift:665-690`、`:862-864`）；sidecar 在模块激活时构造（`LitheDatabaseModule/Module/DatabaseModule.swift:42-47`）。方法名共 **40 条**，全量清单见 `DatabaseSidecarService.swift:480-654`。
- **契约缺口（复刻前必须先决策）**：`shared/contracts/` 与 `docs/` 中**没有任何数据库 / SQL / sidecar 方法契约**（grep `pageTable|redisScan|applyChanges|nacosListConfigs|sidecar|listTables` 零命中），协议只在 Swift 侧 `RequestEnvelope`/`ResponseEnvelope` 中自洽。两条路：复用同一个 sidecar（需把 40 条方法表补进 `shared/contracts/`），或把 DBX 能力搬进 Rust Core。
- **依赖顺序**：§9 结论第 1 条 —— 先做三类宿主容器，不要先做这八个区域（§0.1）；§9 另指出最值得一比一复刻的是**数据库结果表格与 Diff 行列表**（gpui-kit 在这里明显强于现状），**语言智能区域契约最完整、移植成本最低**（`lsp.*` 21 条 + `java.*` 13 条已进 `rust-core-api.md:131-159`），**必须补契约才能复刻：数据库区域**，GitHub 与社区可以先不投入。
- 先定常量：工具窗头 30（leading 12 / trailing 7）、数据库侧栏头部 42、侧栏行高 31/30/26/23、数据表行高 29 与结果网格 27、Diff 行高 24 与 information 27、中缝 34、行号列 47（单文件 55）、diff map 14、历史 sheet 980×620。
- 40 条 sidecar 方法的默认参数：`pageTable` limit 200、`query` limit 10 000、`exportCsv`/`exportJson` limit 100 000、`redisScan` count 100、Nacos pageSize 100；默认超时 30s，`importSql*`/`restoreSql*`/`exportSqlToFile` 120s（`DatabaseSidecarService.swift:586-598`）；错误类型 4 种（`executableNotFound`/`processFailed`/`invalidResponse`/`requestFailed`），输出截断到 500 字符（`:443-464`）。
- 数据库五种状态要齐：空（`DatabaseSidebarView.swift:300-329`）、搜索无结果（`:361-370`）、加载（表元数据 `:675-683`）、错误（`DatabaseLocalization.error` 的 4 条精确/前缀改写规则，`DatabaseLocalization.swift:17-50`）、运行中（SQL 标签换 `arrow.triangle.2.circlepath`，`DatabaseSQLWorkspaceView.swift:534`）、截断（`Limited to 10,000 rows`，`:684`）。
- 数据库侧栏展开/折叠是**负向状态集合**：`collapsedFolderIDs`（默认展开，注释说明避免刷新时被重新打开，`:24-27`）、`expandedProfileIDs`（`onChange(selectedProfileID)` 重置为 `[selectedID]`，一次仅一个）、`collapsedDatabaseProfileIDs`、`collapsedObjectKinds`、`expandedTableKey`（同时只允许一个表展开）；搜索时所有文件夹强制展开（`DatabaseSidebarView.swift:386`）。
- 历史区域两视图**不订阅任何 Core 事件**，只调 `history.record/entries/content/relocate` 四条（`RustCoreBridge.swift:2654-2708`），适配层 payload 字段 `id/timestamp/relativePath/reason/contentPath/byteCount`（`RustLocalHistoryOperations.swift:27-29`）。
- GitHub 区域**不 shell out 到 `gh`**（全域 grep 0 命中），全程 HTTP `URLSession`（`MacGitHubHTTPTransport.swift:19-21`），host 只允许 api/web 且由 Rust 计划给出；命令面是 `github.parseRemote` / `requestPlan` / `normalizeResponse`（`RustCoreBridge.swift:4121-4130`，`normalizeResponse` 按 16 个 operation 分派）+ `git.remoteUrl` / `pullRequestContext` / `command` / `write`（`:3067-3108,3232`）；契约 `shared/contracts/github.md` 与代码分派完全一致。
- Diff 区域命令面只有 `git.diff`（`RustCoreBridge.swift:3268`，`contextLines` 默认 80）与 `git.apply`（`:3290`，mode ∈ `stage`/`unstage`/`discard`/`restoreIndex`/`worktree`/…）；语法高亮与词级差异**没有任何 Core 命令**，是 macOS 侧纯 Swift 手写 tokenizer（`DiffReviewView.swift:1388-1592`）。折叠阈值 12、两侧各留 3 行上下文（`DiffCollapse.swift:56,59`）。
- 社区区域真正生效的数据流只有 WKWebView 直接加载 `https://linux.do/latest`（`LinuxDoAnonymousWebView.swift:140`），不经任何 Rust Core 命令；导航策略仅放行 https 且 host 为 `linux.do`，非白名单 `.linkActivated` 交 `NSWorkspace.shared.open`，`/login` `/signup` `/session` `/user-api-key` `/new-topic` 直接 cancel 且无提示（`:251-277,307-314`）。

**macOS 做得不好 / 用 gpui-kit 优化**
- **列宽是最大结构性问题**：数据表列宽按可用宽均分 `max(140, floor((availableWidth - 64) / columns.count))`，不测量内容、不可拖拽、不可持久化（`DatabaseTableView.swift:655-658`，列多时全部压到 140 下限 → 极宽横向滚动）；结果网格硬编码 180（`DatabaseSQLWorkspaceView.swift:776,790`）。用 `table::Column::{width,min_width,max_width,resizable}` + 文本测量 + `TableEvent::ColumnWidthsChanged` 一次性解决。
- **列表普遍未虚拟化**：数据库表格与结果网格是 `ScrollView([.horizontal,.vertical]) + LazyVStack`（横向完全不虚拟化，最多 10 000 行）；搜索侧栏结果、Problems、GitHub 会话时间线（`VStack { ForEach }`，`GitHubPullRequestsView.swift:680-692`）、并排 diff 两侧（`DiffSplitPaneView.swift:161-165`，而单文件模式反而用 `LazyVStack`）同样如此。gpui 用 `DataTable` / `virtual_list` 补齐。
- **每个单元格挂一个 AppKit 右键捕获层**：`overlay { NSViewRepresentable }`（`DatabaseTableView.swift:609` → `LitheContextMenu.swift:558-566`），单元格数 = 行×列，且每格一个 `TextField`（`:604`）。gpui 的 `TableDelegate::context_menu` + `TableEvent::RightClickedCell` 是行/单元格粒度事件，无每格视图开销。
- **静默丢数据与错误的行身份**：数据库翻页/筛选/排序在 `loadPage` 后直接 `discard()`，清空 `drafts`/`insertedRows`/`deletedRows`/`selectedRows`/`pasteAnchor`（`DatabaseTableView.swift:810-815`）；`DatabaseCellDraft(rowIndex:)` 用页内索引而非已暴露的 `primaryKeyColumns`（`:725`）；filter/sort 是视图局部 `@State`，切表即丢（`:17-23`）。
- **数据库侧栏完全绕开主题刻度且无键盘可达性**：`treeRowHeight 27` / `treeIconSize 16` / `treeFontSize 13.5` 零使用，实际行高散落 31/30/26/23/24、字号 15/13/11.5/10.8/10.5/10/9.5；全区域无 `keyboardShortcut`、无命令目录条目。另：列/索引/外键叶子与视图/存储过程/触发器/序列只是裸 `Text`，**无点击、无 hover、无菜单**（`:699-705,761-792`）。
- 数据库侧栏**无法定位到当前连接/表**（`expandedProfileIDs` 单值、`expandedTableKey` 单值，切换表会折叠上一张，`:31,283-285`）；文件夹行永远 `litheRowHover(isActive: false)`（`:427`），数据库节点与分区头同样；`folderRow` 返回 `AnyView` 并递归自调用，`recursiveProfileCount`/`isFolderVisible` 每帧递归整棵树（`:384,469,1176-1194`）。gpui `TreeState::reveal_item` + `scroll_to_item` + 一次性拍平 items 直接解决。
- 搜索：侧栏结果列表未虚拟化（`SearchSidebarView.swift:87-88`）；结果行**无 hover 反馈**（`LitheTheme.hoverBackground` 在搜索六文件内**未找到引用**）；三处右键菜单与两处选项菜单逐字重复；排序只在 All 页生效（`SearchEverywhereView.swift:129` vs `:508-510`）；浮动面板尺寸硬编码 650×614 / 拖动条 `width - 260`，小窗口下四把手重叠（`ProjectReplaceFloatingPanel.swift:16-25,103-104`）。
- 语言智能：`LanguageServerSetupView` 443 行**是死代码**（全树 grep 只命中定义行），`BuiltInModuleCatalog.swift:124` 的 `language.settings` contribution 无 renderer；Tests 工具窗渲染器缺 `ideaAssetPath`（`WorkbenchModuleUIComposition.swift:170` 传 `nil`），活动栏图标退回 SF Symbol；`ProblemsView` 缺加载态与错误态、每次 body 全量排序 + 三遍过滤（`JavaProblemsView.swift:83-105`）；本地化覆盖断裂（一批字符串在 `en.lproj/Localizable.strings` 中 MISSING）；`DiagnosticSeverity` 与 gpui-base 的 4 档不对齐，Lithe 用字符串匹配 `"unused"`/`"never used"` 推断，**gpui 端应只认 LSP `tags`**。
- 历史：两视图 ~90% 重复（只差栏宽 290/350、行高 52/58、`minimumWidth` 860 vs 900），应参数化成同一组件；**无错误态**（Core 失败被吞成空列表，`ProjectHistoryFeatureModel.swift:305-309`）；无键盘导航；选中条目即整块 `ProgressView` 清空已渲染 diff（`LocalHistoryView.swift:154`）；契约里的 `history.rename`/`history.delete` 无 UI。
- GitHub / Diff / 社区：GitHub 单文件 2111 行 25 个类型零拆分、20 个子视图全 `private` 不可单独实例化；Markdown 正文是纯 `Text`（`:614`、`:1088`）却提示 "Markdown is supported"；label 颜色被丢弃（`GitHubContracts.swift:34` vs `:846`）；合并态复用 `LitheTheme.skill`（AI 技能紫）语义不成立。Diff 单文件住 8 个类型，`DiffRowView`（`:778`）与 `DiffConnectorOverlay`（`:1100`）是死代码（可砍 ~370 行），行高常量四处漂移，并排模式每帧重算布局计划（`DiffSplitPaneView.swift:53`），**`DiffMapView.visibleRange` 无调用方传入 → 视口指示器恒不渲染**，错误态缺失（失败也写成 `diffRows = []`）。社区 `plainText` 降级丢链接/代码块/图片，注入 CSS 固定 `#17181c` 绕过主题，60ms 自动隐藏过于激进，写入路径被静默拦截。

**未查清**
- **契约缺口（最重要）**：`shared/contracts/`（含 `rust-core-api.md`）与 `docs/` grep `pageTable|redisScan|applyChanges|nacosListConfigs|sidecar|listTables` **零命中** → 40 条数据库 sidecar 方法无任何跨端契约；`rust-core-api.md` 命令表里也**没有**任何数据库 / SQL 命令（grep `database.`/`sql.`/`db.` 只命中 Git 与 Maven 的无关行）。
- **契约-实现不一致**：`RustCoreBridge.swift:2489/2505/2521` 发出 `workspace.searchIndex.warm` / `.update` / `.invalidate` 三条命令，但 `shared/contracts/` 中**未登记**（grep `searchIndex` 0 命中）；`:3683` 的 `lsp.retryMavenProfiles` 同样**未登记在契约命令表内**。
- 数据库区域**独立窗口：未找到**；模块贡献 `database.workspace` 虽声明 `kind: .toolWindow`，但**没有 `rendererID`/`actionID`，也没有任何渲染方（未找到）**（`BuiltInModuleCatalog.swift:107`）；`EditorTabItem` 只有 `document`/`terminal`/`media`，**无 database 类型**（`EditorTabItem.swift:4-10`）。
- 数据库**全区域无任何 `keyboardShortcut` / `onKeyPress` / `KeyEquivalent`（未找到）**，`LitheCommandCatalog` 中也没有任何数据库命令（grep `Database` 0 命中）；唯一键盘处理是 `SQLTextView.keyDown` 的 `⌘Return`/`⌘Enter`/`⌃Space`（`DatabaseSQLWorkspaceView.swift:1395-1406`）。
- 数据库侧栏**未找到**：列宽拖拽、列宽状态持久化、单元格选择、SQL 结果网格的选择与复制、表格行/单元格 hover 高亮、Esc/取消编辑语义、冻结首列与行号列、内联重命名（改名走 sheet）。
- 数据库**右键菜单未找到的节点**：列 / 索引 / 外键 / 视图 / 存储过程 / 触发器 / 序列 / Redis key / Nacos config / Nacos service（`DatabaseSidebarView.swift:761-792`、`:699-705`）、SQL 编辑器（`SQLTextView` 只重写 `keyDown`，未重写 `menu(for:)`，`:1391-1407`）、SQL 结果网格（`:754-806`）、Schema Diff（只有工具栏 `Copy migration SQL` / `Apply`）全部无右键菜单；Schema Diff **无错误展示（未找到）**。
- 搜索：`.keyboardShortcut` / `.onKeyPress` / `KeyEquivalent` 在六个文件内**未找到**；**Core→Swift 推送事件未找到**（全为 request/response）；`LitheTheme.hoverBackground` 在本区域**未找到引用**。
- 语言智能：**文件名与类型名不一致**（`JavaProblemsView.swift:3` 定义 `ProblemsView`、`JavaReferencesView.swift:3` 定义 `LanguageReferencesView`），故 grep `JavaProblemsView(` / `JavaReferencesView(` **未找到**任何调用点，按文件名找挂载点会得出错误结论；`LanguageServerSetupView` 挂载点**未找到**（疑似死代码或仅预览/测试引用）；`Views/Language/` 内 `contextMenu` / `litheContextMenu` / `.keyboardShortcut` / `.onKeyPress` / `KeyEquivalent` **均 0 命中**；`ProblemsView` **加载态未找到**（无 loading 分支）。
- 历史：`.contextMenu` / `.litheContextMenu` **未找到**；`.keyboardShortcut` **未找到**（`local-history` / `project-local-history` 在命令目录中未分配按键）；Core 事件**未找到**；**无错误态**；`history.rename` / `history.delete`（`rust-core-api.md:105-106`）**无任何 UI 调用方**（grep 无命中）；工程版有 "No textual difference" 分支而**单文件版没有**（不对称）。
- 诊断：`DiagnosticsExportSheet` 右键菜单**未找到**；命令目录中**不存在** diagnostics 相关命令（`toggle-problems` 与导出菜单项均无按键）；markdown / 图片附件 / 外链在诊断区**全部未找到**；`lsp.pollEvents` / `lsp.snapshot` 的事件名与字段细节只在 LSP 契约内，本区**未逐字段展开（精确字段清单未找到）**。
- GitHub：**本文件没有任何 `.contextMenu`（grep 0 命中）**，存在的菜单只有账号头与详情页头两处 `Menu { }`；`GitHubPullRequestBranchDefaults` **不在** `shared/contracts/` 中（定义在 `Core/Ports/GitHubOperations.swift:23`，**契约未找到**）；**`Timeline` 类型不存在**（grep 0 命中，`lib.rs:25-94` 模块清单无 `timeline`）；**无分页**（`listPullRequests` 只带 `state`，无 page/per_page）；`canUseDeviceFlow` 在视图里**无引用**；复用标题栏注入的右键项在本区域**实际为空菜单**（调用未传 `onMinimize`）。
- Diff：**工具窗未找到、独立窗口未找到**；**行级 hover 未找到**；上下文菜单在 `Views/Diff/` 内**未找到**；`.keyboardShortcut` / `.onKeyPress` / `KeyEquivalent` **全部未找到**；**错误态未找到**；**语法高亮 token 契约未找到**（无对应 Core 命令）；**diff 专用组件未找到**（`lib.rs:25-94` 无 diff/compare 模块，全 src grep `(?i)diff` 只命中 tree-sitter-diff 语言注册）。
- Diff 未接线代码：`MonacoDiffEditor`（`Platform/MacOS/MonacoDiffEditor.swift:8`）、`DiffRowView`（`DiffReviewView.swift:778`）、`DiffConnectorOverlay`（`:1100`）**全仓无任何实例化**；⚠️ `DiffMapView.visibleRange`（`DiffMapView.swift:24`）**所有调用点都没传**，视口阴影与 "scroll indicator" 行为**恒不渲染**；并排同步横向滚动在分隔条拖到不等宽后左/右内容宽公式不再相等（**推断**：最长行落在右窗格时右窗格无法滚到末尾，缺验证用例）。
- gpui-kit 侧核对限制：**`gpui-base` 源码在本机 registry 路径下未找到**（glob `gpui-base*/Cargo.toml` 无结果），凡以 `gpui_base::` 为实现的类型只能依据 gpui-component 的再导出声明，**不能核对实现**。
- 社区：`contextMenu` **未找到**（且因 `onMinimize` 为 nil，工具窗头右键项返回空数组 → **完整右键菜单列表为空**）；`.keyboardShortcut` **未找到**（`community.linux-do.toggle` 未注册进命令目录）；Core 事件订阅**未找到**；原生视图**未找到**任何远程图片或附件渲染；`canGoBack` / `canGoForward` 是**死状态**（header 无前进/后退按钮，`.back`/`.forward` 分支全仓无构造点）。
- 死代码与不可达（移植前必须先决断）：`DatabaseSchemaDiffView`（`DatabaseSchemaDiffView.swift:5`）与 `DatabaseDiagnosticsView`（`DatabaseSQLWorkspaceView.swift:1028`）**全仓零实例化**；`LinuxDoTopicListView` / `LinuxDoTopicDetailView` / `LinuxDoCommunityFormatting` 与 7 条 `community.discourse.*` 命令**不可达**（`AppModel.discourseCommunityFeature` 无消费方，`AppModel.swift:166`）；GitHub 被 `githubPullRequests = false` 整体关闭，投入复刻前必须先确认解除条件。

### 2.8 组件库与设计 token（macOS 出处：`macos/Sources/Lithe/Theme/`、`macos/Sources/Lithe/Views/Components/`）

**组成**
- token 层：`enum LitheTheme`（单例命名空间）+ 私有 `struct Palette`（36 字段）（`Theme/LitheTheme.swift:4`、`:44-82`）；尺寸表 `Metrics`（16 常量）+ `Commit`（12 常量）（`:404-423`、`:426-443`）。
  - 配色家族 3 套 `lithe`/`codex`/`linear`（`Models/Settings/AppSettings.swift:609-612`）；**明暗不由家族决定**，每个 token 用 `NSAppearance.bestMatch([.aqua,.darkAqua])` 惰性解析（`LitheTheme.swift:368-374`）。
  - `lithe` 硬编码 36 组 `adaptive(light:dark:)`（`:177-224`）；`codex`/`linear` 只固定 7 个基色，其余由 `mixed()` 公式派生（`:96-175`）；对 AppKit 装饰代码只暴露 13 个 `ResolvedColorToken`（`:229-266`）。
- 基础组件层（`Theme/` 放样式与图标）：`LitheIcon`/`LitheIDEAIcon`/`LitheSystemIcon`（`Theme/LitheIcons.swift:566`、`:593`、`:639`；`LitheIconKind` 48 case `:7-64`）；`LitheIconButtonStyle`/`LitheTreeRowButtonStyle`/`LithePrimaryButtonStyle`/`LitheSecondaryButtonStyle`（`LitheTheme.swift:479`、`:500`、`:528`、`:549`）；`.litheRowHover`/`.litheSearchField`/`.litheContextMenuSurface`/`.lithePopupChrome`（`:460-476`、`:619-641`、`:659-671`、`:674-685`）；`LitheScrollViewChrome` + `CompactScroller`（`Views/Components/LitheScrollViewChrome.swift:82`、`:258`）。
- 复合组件层（`Views/Components/`，10 个文件）：`LitheContextMenuPresenter` + `LitheContextMenuItem`（`LitheContextMenu.swift:301`、`:16`）、`LitheToolWindowHeader`（`LitheToolWindowHeader.swift:6`）、`LitheSettingsSearchField`/`LitheSettingsSelect`/`LitheSettingsSegmentedControl`/`LitheSettingsCheckbox`（`LitheSettingsControls.swift:13`、`:65`、`:202`、`:254`）、`LitheSplitPaneView` + `LitheSplitPaneGeometry` + `SplitHandleView`（`LitheSplitPaneView.swift:11`、`Views/Workbench/SplitHandleView.swift:10`）、拖动合帧 `LitheDragUpdateScheduler`/`FrameCoalescedDragUpdateBuffer`、`LitheUnavailableView`（`:3`）。
- 编辑区（占布局）：编辑器页签（`Views/Editor/EditorAreaView.swift:219-232`、`:622-720`）与工具窗头；**浮层**（覆盖层）：`.lithePopupChrome`/`.litheContextMenuSurface` 承载的 12 处 `.sheet`，以及自管 `NSPanel` 的右键菜单（`LitheContextMenu.swift:361-366`）。

**度量**（macOS 数值，可原样搬成 `px()`）
| 元素 | macOS 值 | 出处 |
| --- | --- | --- |
| 字号 / 字重 | `uiFont` 14 regular、`smallFont` 12、`codeFont` JetBrainsMono-Regular 13、编辑器字号默认 13；**字重没有 token 表**，`.medium`/`.semibold`/`.bold` 与点数就地写（如 `.system(size: 13, weight: .medium)`） | `LitheTheme.swift:376-378,382-388,535`；`AppSettings.swift:157,313,316` |
| 行高 | `editorLineHeightMultiple` 1.2、`editorBaselineLift` 1.5、`editorParagraphStyle.lineHeightMultiple` 1.2；**非编辑器行高没有定义** | `LitheTheme.swift:379-380,390-394` |
| 圆角 | cornerRadius 5、controlCornerRadius 6、contextMenuCornerRadius 9、popupCornerRadius 10、树选中弧 4；HUD 7 与菜单行 5 是硬编码（未进 `Metrics`） | `LitheTheme.swift:419-422,412`；`WorkbenchView.swift:497`；`LitheContextMenu.swift:276` |
| 间距刻度 | **没有 4/8/12/16 通用阶梯**：`Metrics` 只有行高/图标/容器高/圆角四类，水平垂直间距在视图内硬编码（`HStack(spacing: 8)`、`.padding(.horizontal, 9)`） | `LitheTheme.swift:404-423` |
| `Metrics` 全套 | rowHeight 24、treeRowHeight 27、projectTreeRowSpacing 1、内容内边距 4/12、treeIconSize 16、treeFontSize 13.5、tabHeight 34、toolbarHeight 40、toolWindowHeaderHeight 30、statusBarHeight 24 | `LitheTheme.swift:405-418` |
| `Commit` 全套 | toolbarHeight 37、listMinimumHeight 120、areaMinimumHeight 124、panelPadding 10、toolbarFontSize 12.5、tabItemPadding 7/6、metadataFontSize 12、amendFontSize 12.5、actionIconSize 14、messageFontSize 13、editorInset 8/7、compactButton 高 24/padding 7/字 11 | `LitheTheme.swift:427-442` |
| 阴影三档 | **没有 sm/md/lg**，仅 3 个具名用法 + 1 特例：浮层 `popupShadow, radius 30, y 14`、替换面板 `radius 18, y 8`、拖动/终端标签 `.black 0.42, radius 10, y 6`、插入指示 `accent 0.7, radius 4` | `LitheTheme.swift:207,684`；`ProjectReplaceView.swift:42`；`EditorAreaView.swift:769,741`；`TerminalView.swift:259` |
| 动效时长与曲线 | **没有 token**：6 个时长 + 1 spring 就地写（0.10/0.12/0.14/0.15/0.16/0.18/0.22s），`easeOut` 与 `easeInOut` 混用；`interactiveSpring(response 0.22, damping 0.86, blend 0.10)`；拖动与右键菜单显式 `animation = nil` / `.none` | `EditorAreaView.swift:773-777`；`WorkbenchView.swift:536,666,1392-1405`；`SplitHandleView.swift:76-78`；`LitheContextMenu.swift:370` |
| 图标尺寸 | 三个包装默认 14、树 16、`Commit.actionIconSize` 14、应用标志 42、菜单图标列 16×16；显式 `size:` 实际分布 12/13/14/15/16 | `LitheIcons.swift:569,596,641,549,843`；`LitheTheme.swift:413,436`；`LitheContextMenu.swift:250` |
| 密度档位 | **没有**：`Theme/` grep `density|紧凑|宽松` 0 匹配，`Metrics`/`Commit` 是单档常量表；最接近的是可调标量 `projectTreeRowHeight` 24 与 `editorFontSize` 13 | `LitheTheme.swift:404-443`；`AppSettings.swift:160,316,157,313` |
| 交互状态值 | hover 浅 `(0.949,0.953,0.961)` / 深 `(1,1,1,0.055)`；pressed 浅 `(0.882,0.890,0.906)` / 深 `(1,1,1,0.095)`；`selection` 不透明 `(0.208,0.455,0.941)` 明暗同值；**禁用态 0.45 与 0.55 并存未统一** | `LitheTheme.swift:193-196`；`LitheContextMenu.swift:283`；`LitheSettingsControls.swift:397` |
| 按钮 / 输入框 | 图标按钮 28×28 圆角 5；主按钮高 30、圆角 6、水平 padding 18、字 13 medium、底 `accent` 常态 `opacity 0.92`；次按钮底 `raised` 常态 `opacity 0.72` + 1pt 描边；搜索框高 28、圆角 6、水平 8；设置输入框高 30、字 12.5、内边距 9 | `LitheTheme.swift:479-497,528-573,619-641,380-405` |
| 弹层 / 菜单 | `lithePopupChrome` 圆角 + `popupBackground` + 1pt `panelBorder` + 投影；菜单面圆角 9；菜单根最小宽 230 / 子菜单 220 / 最大 360，行高 26、分隔 11、垂直内边距 12、子菜单间距 1 | `LitheTheme.swift:674-685,659-671`；`LitheContextMenu.swift:5-13` |
| 滚动条 / 分隔条 | 滚动条**无 token**：knob = `secondaryText` + alpha（深 0.62 / 浅 0.36），lithe 深色特例 `(67,67,67)`，圆角 `min(2.5, 尺寸/2)`，可见宽 5；分隔条可见厚 5、命中厚 10、线宽 1 | `LitheScrollViewChrome.swift:279-289,270-276`；`SplitHandleView.swift:13-14,134-135` |

**元素 → gpui-kit 对应**
| macOS 元素 | gpui-kit 实现 | 判定 |
| --- | --- | --- |
| token 落值层（3 家族 × 明暗） | 3 组 `gpui_kit::component::theme::ThemeConfig` + `gpui_kit::component::theme::ThemeColor`（117 字段）+ `gpui_kit::component::theme::ThemeMode` | 需自行组合 |
| `window`/`primaryText`/`secondaryText`/`divider`/`popover`/`link`/`titlebar` | `gpui_kit::component::theme::ThemeColor::{background, foreground, muted_foreground, border, popover, link, title_bar}` | 可一比一 |
| `accent`（唯一强调蓝 `#3574F0`） | `gpui_kit::component::theme::ThemeColor::primary`（**不是** gpui 的 `accent`） | 可一比一 |
| `selection`（不透明选中底） | `gpui_kit::component::theme::ThemeColor::selection`（须覆盖为不透明） | 需自行组合 |
| `subtleSelection` / `hoverBackground` / `pressedBackground` | `ThemeColor::accent` / `ThemeColor::list_hover` / `ThemeColor::table_hover` / `ThemeColor::button_active` | 可一比一 |
| 19 项 gpui 没有的颜色 token | 应用侧 `LitheColors` 结构体 + `cx.global` | gpui-kit 没有 |
| `Metrics` / `Commit` / 图标尺寸 / 间距 | 应用侧 `const LitheMetrics`（gpui 无行高/容器高/图标尺寸 token） | gpui-kit 没有 |
| 字号×字重刻度 / 非编辑器行高 / 阴影三档 / 动效 | `gpui_kit::base::theme_tokens::{TypographyTokens, SpacingTokens, ShadowTokens}` + `gpui_kit::component::theme::MotionTokens`（macOS 侧缺，反向采纳） | 可一比一 |
| `LitheIcon`/`LitheIDEAIcon`/`LitheSystemIcon`（IntelliJ SVG + SF Symbol 映射） | `gpui_kit::component::Icon` + `gpui_kit::assets::Assets`（IntelliJ SVG 需自备资源包） | 需自行组合 |
| 4 个 `Lithe*ButtonStyle` | `gpui_kit::component::button::Button`（`.primary()`/`.outline()`/`ghost()` + `with_size`） | 可一比一 |
| `LitheSearchFieldStyle`/`LitheSettingsSearchField`/`LitheSettingsTextFieldModifier` | `gpui_kit::component::input::Input` + `gpui_kit::component::input::ClearButton` | 可一比一 |
| `LitheSettingsSelect`/`SegmentedControl`/`Checkbox` | `gpui_kit::component::select::Select` + `gpui_kit::component::popover::Popover` + `gpui_kit::base::positioner::Positioner`；`gpui_kit::component::toggle_group::ToggleGroup`；`gpui_kit::component::checkbox::Checkbox` | 可一比一 |
| 计数胶囊（4 处就地写，无通用 `Badge`） | `gpui_kit::component::badge::Badge` / `gpui_kit::component::tag::Tag` | 可一比一 |
| 编辑器页签 + `EditorTabFlowLayout` | `gpui_kit::component::tab::TabBar` + `gpui_kit::component::tab::Tab`（`TabVariant::Underline`，中号高 36 ≈ Lithe 34） | 可一比一 |
| `LitheToolWindowHeader`（高 30，图标 13 + 标题 12.5 semibold + 副标题 11.5） | 无对应组件：用 `gpui_kit::base::h_flex` + `gpui_kit::base::StyledExt` 自绘头部，配 `gpui_kit::component::dock::tab_panel::TabPanel` | 需自行组合 |
| 项目树行 / 设置侧栏项 / `.litheRowHover` | `gpui_kit::component::tree::{Tree, TreeState, TreeItem, TreeEntry}` + `gpui_kit::component::list::{List, ListItem}` + `gpui_kit::base::StateStyle` | 可一比一 |
| `DatabaseTableView` 与 `NSTableView`/`NSOutlineView` 包装 | `gpui_kit::component::table::{Table, TableState, TableDelegate, DataTable, Column}` | 可一比一 |
| `GroupBox` 就地描边 / `GitSettingsRow` / `LitheSettings*` 四控件 | `gpui_kit::component::group_box::GroupBox` + `gpui_kit::component::setting::{Settings, SettingPage, SettingGroup, SettingItem}` | 可一比一 |
| 12 处 `.sheet` 对话框 + `lithePopupChrome` | `gpui_kit::component::dialog::{Dialog, AlertDialog}` + `gpui_kit::component::sheet::Sheet`（含焦点陷阱与 Esc） | 可一比一 |
| `LitheContextMenuPresenter`（约 570 行 `NSPanel` + `NSEvent` 监听） | `gpui_kit::component::menu::{ContextMenu, ContextMenuExt}` + `gpui_kit::base::positioner::Positioner` | 可一比一 |
| 通知 HUD 数组内联渲染（`WorkbenchView.swift:466-505`） | `gpui_kit::component::notification::{Notification, NotificationList}` + `gpui_kit::base::toast` | 可一比一 |
| `GitHubCenteredProgress` / 4 份空状态 | `gpui_kit::component::progress::{Progress, ProgressCircle}` + `gpui_kit::component::empty::{Empty, EmptyHeader, EmptyTitle, EmptyDescription, EmptyContent}` | 可一比一 |
| 骨架屏（macOS 没有） | `gpui_kit::component::skeleton::Skeleton` + `gpui_kit::component::shimmer::ShimmerText` | 可一比一 |
| `SplitHandleView` + `LitheSplitPaneView` | `gpui_kit::base::{h_resizable, v_resizable, resizable_panel, resize_handle}` + `gpui_kit::base::theme::ResizableTheme` | 可一比一 |
| `LitheScrollViewChrome` + `CompactScroller`（11 文件引用） | `gpui_kit::base::theme::ScrollbarTheme` + `gpui_kit::component::theme::ThemeColor::{scrollbar, scrollbar_thumb, scrollbar_thumb_hover}` | 可一比一 |

**复刻要点**
- 语义陷阱一：`accent` 语义互换。macOS `accent` = 主强调蓝 `#3574F0`（`LitheTheme.swift:214`）→ 落 `gpui_kit::component::theme::ThemeColor::primary`；gpui 的 `accent` 是**悬停/弱选中底**（对应 macOS `subtleSelection`，`:194`）。按同名直搬，整个应用的强调色会变成一排灰底（`LitheTheme.swift:214`；`docs/DESIGN.md:13`）。
- 语义陷阱二：`selection` 形态不同。macOS 是**不透明** `#3574F0` 且明暗同值（`:193`），gpui 默认 30% 透明 `rgb(0x55a0fc)`（`theme_tokens.rs:74`）；必须显式覆盖为不透明，否则选中行是半透明色块、白字不可读。
- 语义陷阱三：`editor` 面 ≠ `background`。浅色下 macOS `window=(0.933,0.945,0.961)` 而 `editor=(1,1,1)`（`:185`、`:190`）；gpui 默认 `background == popover == white`（`default-theme.json:16,48`），要 `popover` 落 `editor`、`background` 落 `window`。
- 语义陷阱四：`inactiveTabBackground` 是 `Color.clear`（`:324`），gpui 的 `tab` 默认也是 `#00000000`（`default-theme.json:77`）——这一项天然对齐。
- 语义陷阱五：`secondaryText` 深色是半透明白 `(1,1,1,0.50)`（`:210`），gpui 的 `muted_foreground` 默认是不透明灰（`theme_tokens.rs:90`）；落成不透明灰会失去壁纸透出观感（`WorkbenchSurfaceBackground.swift:11`）。
- **需自建的 19 项颜色**（`LitheColors`，出处均在 `Theme/LitheTheme.swift`，逐项如下）：
  - 背景与表面（8）：`window` 应用窗口底（`:185`）、`tool_header` 工具窗标题栏（`:187`）、`tool_header_inactive` 非激活工具窗标题栏（`:188`）、`editor` 编辑器面（`:190`）、`raised` 抬起面/次级按钮底（`:191`）、`notification` 通知 HUD 底（`:192`）、`context_menu` 右键菜单面（`:306-314`）、`badge_background` 徽标底（`:208`）。
  - 文本（3）：`tool_window_text` 工具窗标题文本（`:212`）、`tool_window_selected_text` 选中/hover 项文字（`:213`）、`tertiary_text` 三级文本/占位符（`:211`，gpui 的 `muted_foreground` 只有一档）。
  - 语义色（2）：`skill` Skill 标记紫（`:219`）、`run_action` 运行按钮绿（`:215`，gpui 无专用色）。
  - 编辑器（4）：`guide` 缩进竖线（`:221`）、`active_guide` 激活缩进竖线（`:222`）、`diff_information_bg` diff 信息块底（`:199`）、`diff_information_text` diff 信息块文本（`:200`）。
  - 浮层与标签（2）：`popup_shadow` 浮层投影色（`:207`）、`tab_underline` 标签下划线（`:198`，gpui 无专用下划线色）。
  - 为什么不能加在上游：`ThemeColor` 是 `Copy + Serialize + Deserialize + JsonSchema` 的**上游 crate 类型**（`theme_color.rs:58`），应用无法加字段，只能声明自己的颜色结构体（`2.3-A`）。
  - 取值方式：走 `gpui::rgb(0x…)` → `Hsla::from(...)`（gpui-base 自己就这么写，`theme_tokens.rs:74`）；**不要**把 sRGB 浮点 `(0.208, 0.455, 0.941)` 当 HSL 直接用。
- 依赖顺序（P0 三件）：按 §2.1 表落 `ThemeColor` 117 字段 → 建 `LitheColors`（19 项）→ `Theme::global_mut(cx).motion` 覆写时长；`LitheMetrics` 用 `const`，**不要**做成运行时 token（`:436`）。
- 字号对齐：`TypographyTokens::md = 14`（对齐 `uiFont`）、`xs = 12`（对齐 `smallFont`）、`mono_md = 13` 且行高按 `1.2em`；字体族必须从 gpui 默认 `Menlo` 改成 `JetBrainsMono-Regular`（`theme/mod.rs:665-667`）。
- 按钮尺寸硬约束：gpui `Button::Medium` 高 32、`Small` 24，与 Lithe 的 30/28/24 不同 —— 用 `Button::new().with_size(px(30.))` 精确对齐，**不要**改 gpui 的 `Size` 表（`button.rs:619-637`）。
- 必须保留的自绘只有 3 处：壁纸底「有壁纸时透明」语义（`WorkbenchSurfaceBackground.swift:11`）、IDEA 风格树选中弧（4pt + 行距 1 + 内边距 4/12）、聚焦 `activeTabBackground` 对 `Color.clear` 的对比。
- 滚动条 knob 色落 3 个 scrollbar token：`secondaryText` + alpha（深 0.62 / 浅 0.36）与 lithe 深色特例 `(67,67,67)`（`LitheScrollViewChrome.swift:279-284`）。

**macOS 做得不好 / 用 gpui-kit 优化**
- `LitheContextMenuPresenter`（约 570 行 `NSPanel` + `NSEvent` 本地/全局监听 + 手写定位翻转与键盘，`LitheContextMenu.swift:301-502`、`:370-385`、`:470-501`）：把平台级菜单当应用级组件实现 → `gpui_kit::component::menu::{ContextMenu, ContextMenuExt}` + `gpui_kit::base::positioner::Positioner`，一行 `context_menu` 即自动处理 Esc/翻转/失焦。
- `LitheScrollViewChrome`（11 文件引用，含 `NSEvent` 滚轮转发与手绘 `CompactScroller`）：用 representable 探针改**外层** AppKit 滚动视图，属平台补丁 → `gpui_kit::base::theme::ScrollbarTheme` + `Theme::scrollbar_mode`（idle 2s / enter 300ms / exit 500ms 与淡入策略已在主题里）。
- `LitheDragUpdateScheduler` + `FrameCoalescedDragUpdateBuffer`（手动合帧 + `animation = nil`）：解决的是 SwiftUI 每次拖动重算 body 的问题 → `gpui_kit::base::{h_resizable, v_resizable, resizable_panel}`；GPUI 是帧驱动立即模式，但须保留「拖动中不触发全窗口重排」的等价纪律。
- `litheRoundedControlBackground` 的「不能 clip」约束（AppKit 承载的 `TextEditor`/`TextField` 被 mask 后变黄色占位符，`LitheTheme.swift:643-648`）：gpui-base 的 input 是自绘的 → 直接用 `rounded`/clip，该约束不继承。
- `LitheSettingsSelect` 的手算宽度（`最宽项文本宽 + shortcut + 67` 再夹取，`LitheSettingsControls.swift:184-199`）：手算 `NSFont` 文本宽并与屏幕宽比较 → `gpui_kit::component::popover::Popover` + `gpui_kit::base::positioner::Positioner`（自动避让与翻转）。
- `EditorTabFlowLayout`（自定义 `Layout` 协议实现，`EditorAreaView.swift:203`）：自写流式布局处理换行标签 → `gpui_kit::component::tab::TabBar` 的滚动/换行 + `gpui_kit::base::dock::tab_group`（含拖拽重排）。
- 空状态四份重复实现（`LitheUnavailableView.swift:3-35`、`GitHubPullRequestsView.swift:1281-1306`、`GitRepositoryEmptyView.swift:4`、`DatabaseTableView.swift:1108`；padding 24/28、图标 28/27 漂移）→ `gpui_kit::component::empty::{Empty, EmptyHeader, EmptyTitle, EmptyDescription, EmptyContent}`。
- 通知数组内联渲染（`WorkbenchView.swift:466-505`，无生命周期/堆叠/自动消失策略）→ `gpui_kit::component::notification::{Notification, NotificationList}` + `gpui_kit::base::toast`。
- `LitheTreeRowButtonStyle` 的空 `ButtonStyle`（`LitheTheme.swift:500-504`，靠清空外观绕过系统 Button）→ `gpui_kit::component::button::Button` 的 `ghost()`；禁用态也统一交给 `gpui_kit::component::Disableable` / `gpui_kit::base::StateStyle`，不再保留 0.45/0.55 两个值。
- `settingsSelection`/`settingsPrimaryAction` 不随明暗变化（`:282-291`，深色下 `(43,66,113)` 选中底对比度可疑）→ 落成明暗两套值，或用 `primary` + `primary_foreground` 组合。
- `accent` 同时承担「强调色 + 链接色」（`link = accent`，`:171`）：语义重叠使链接无法独立调色 → gpui 已有独立 `link`/`link_hover`/`link_active`，分开落值。
- `LitheToolWindowHeader` 与工具窗内两套次级页签各自实现（`EditorAreaView.swift:1611-1631`、`TerminalView.swift:307`，未见共享类型）→ 统一走 `gpui_kit::component::tab::TabBar`。

**未查清**
- `Metrics` 之外的间距是否有隐含刻度：`Views/` 内 `spacing:`/`padding` 未逐项统计，只确认「没有统一间距 token」；严格 1:1 需再做一次全量间距直方图。
- 禁用态的单一目标值：0.45（`LitheContextMenu.swift:283`）与 0.55（`LitheSettingsControls.swift:397`）并存，未找到第三处或设计说明判定哪个正确；`docs/DESIGN.md:185-189` 也把「macOS 缺少禁用态设计」列为未决。
- `LitheSignpost` 的定义位置未定位（被 `SplitHandleView.swift:27`、`:70`、`:87` 使用）；属性能埋点，本次未展开。
- codex/linear 家族在其 UI 中的实际使用比例未统计；若这两套很少使用，复刻可只做 lithe 一套 + 声明式回退。
- `LitheIcons.ideaAssetPath(forSystemImage:)` 的映射表规模：只确认在 `LitheIcons.swift:180-312`（约 130 行），未逐条清点，需要单独一份图标清单任务。
- `gpui-kit-design-guides` 的具体条文未加载全文；§4 建议以 `docs/DESIGN.md:206-213` 的摘要为依据，落地前应完整阅读该指南。
- gpui-base 的 `TextStyleToken` 是否被组件消费未逐个确认：`TypographyTokens` 6 档定义在 `theme_tokens.rs:157-188` 并经 `theme/mod.rs:486-493` 投影，若多数组件仍走 `theme.font_size`，字号 token 落值效果有限。

---

