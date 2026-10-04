# Agent 笔记：欢迎窗口自己画标题栏、每次打开工作区都记进最近项目

状态：已实现

## 先说结论

欢迎窗口（`Lithe` 无参启动、没有可用最近项目时开出的那个 800×650 窗口）过去**没有标题栏**：
它既不画最小化 / 最大化 / 关闭三个窗口键，也不提供标题栏拖拽区。原因是 gpui 在 Windows 上
**不创建原生标题栏**，每个窗口都必须自己画，而工作区窗口画了、欢迎窗口漏了 —— 用户只能靠
`Alt+F4` 或任务栏关窗。现在欢迎窗口渲染一份 gpui-kit 的 `TitleBar`，窗口键与拖拽区一起回来了。

第二条：**"打开过项目却没有历史记录"**。宿主过去只在"欢迎页点开项目"和"项目菜单换根"两处写
最近项目，所以最常见的 `Lithe <路径>` 启动**一次都不记录**；而欢迎页列的就是这份数据，于是永远
显示"暂无最近项目"。现在**建立工作区外壳（`ShellWorkspace::new`）时一律记录当前根**，与 macOS
真源 `WorkspaceSessionCoordinator.openWorkspace` 里那句 `recordRecentProject` 对齐。

开发者要记住两件事：**给任何新窗口做壳，必须自己画标题栏**（用 gpui-kit 的 `TitleBar`，
不要在业务代码里写平台分支，见下）；**"打开工作区"这件事的唯一收口是 `ShellWorkspace::new`**，
新增打开路径不要再各写一份记录逻辑。

## 问题

用户给了两张对照截图：新版（0.0.1，gpui 宿主）的欢迎窗口没有那三个窗口键，也没有历史记录；
旧版（0.5.12，已删除的 Tauri 前端）两样都有。逐条查证后是两个独立缺陷。

**缺陷一：欢迎窗口没有窗口壳。** 证据链：

- gpui 的窗口在 Windows 上**根本没有原生标题栏** —— `gpui-pre-windows` 构造窗口样式时只给
  `WS_SYSMENU | WS_THICKFRAME | WS_MAXIMIZEBOX | WS_MINIMIZEBOX`，**不加 `WS_CAPTION`**
  （`gpui-pre-windows-0.3.7/src/window.rs`，`hide_title_bar` 由 `titlebar.appears_transparent` 决定）。
  也就是说"窗口键"必须由应用自己画，这是 gpui 的架构前提，不是可选装饰。
- 工作区窗口画了：`crates/workbench/src/title_bar.rs` 的 `window_controls()` 画 min/max/close，
  并有 `window_control_area(WindowControlArea::Drag)` + `start_window_move()` 的拖拽区。
- 欢迎窗口没画：`crates/app/src/main.rs` 建窗口时用的是同一份 `TitleBar::window_options()`
  （即"透明标题栏、没有原生键"），但窗口内容只有 `WelcomeView`；`welcome.rs` 的 `render` 是
  "侧栏 + 分隔线 + 内容"三件套，**没有标题栏、没有窗口键、没有拖拽区**。

后果不只是"点不了关闭"：窗口同样**不能用鼠标拖动**（gpui 需要应用声明拖拽区），
欢迎窗口在 800×650 且 `window_min_size` 传同值（不可缩放）的前提下，几乎只能靠系统快捷键操作。

**缺陷二：`Lithe <路径>` 启动不写最近项目。** 宿主里 `record_opened_project` 只有两个调用点：
欢迎页 `WorkspaceOpener::open_project`（`welcome.rs`）与项目菜单换根 `rebuild_project_window`
（`workspace.rs`）。启动路径 `resolve_launch_plan` 在"显式给了路径"时直接返回
`LaunchPlan::Workspace(root)`，**不读也不写**最近项目文件；建壳时不记录。所以：

- 从命令行 / 快捷方式带路径启动（开发时的日常方式）永远攒不出历史；
- 下一次无参启动读到的仍是空列表，于是又落到欢迎页 —— 用户看到的就是"打开过却没有历史记录"；
- 磁盘证据也吻合：这台机器上 `%APPDATA%\Lithe` 目录**根本不存在**，说明 gpui 宿主从未落过盘
  （落盘路径 `save_json` 会 `create_dir_all(parent)`，目录不存在 ⇒ 没有一次写入发生过）。
  旧版 0.5.12 的 5 条历史存在 `%APPDATA%\app.lithe.windows\` 的 WebView2 存储里，
  和这份 `recent-projects.json` 不是同一份数据，本批不做迁移。

macOS 真源是"打开工作区就记录"：`WorkspaceSessionCoordinator.openWorkspace(at:)` 在建完工作区后
无条件调 `recordRecentProject(normalizedURL)`。gpui 宿主缺的正是这一步。

## 决策

### 一、窗口壳一律用 gpui-kit 的 `TitleBar`，不手写窗口键、不写平台分支

欢迎窗口的 `render` 改成纵向两段：**顶上一条 `TitleBar`，下面才是"侧栏 + 分隔线 + 内容"**。

- 高度与工作区窗口一致：`welcome.rs` 新增 `TITLE_BAR_HEIGHT_SPEC = 40.`，用
  `.h(rems(TITLE_BAR_HEIGHT_SPEC / 16.))` 落下去（不要写 `px(40.)`，`node lithe-gpui/tools/check-ui-px.mjs` 会拦）；
  底色取 `cx.theme().title_bar`，与工作区标题栏同一个 token。
- 组件选 gpui-kit 的 `TitleBar`（`gpui_kit::component::TitleBar`）而不是把
  `title_bar.rs` 的 `window_controls()` 抄一份：**上游组件自己处理平台差异** —— macOS 上
  `WindowControls` 直接渲染空元素（红绿灯由 AppKit 画，应用再画一份就是两套按钮），
  Windows 上画 min/max/close 并挂 `WindowControlArea`，Linux 上还要看窗口是不是客户端装饰。
  这正是仓库硬规则"平台差异交给 gpui 平台层与 `TitleBar`，界面结构三端统一"的落地方式。
- 拖拽与双击由 `TitleBar` 自己接管；`crates/app/src/main.rs` 已经传了
  `TitleBar::window_options()`（含 `app_owns_titlebar_drag: true`），所以欢迎窗口不需要改窗口选项。

### 正确做法

- 新窗口要窗口键 / 可拖动：`.child(TitleBar::new())`（用 `..TitleBar::window_options()` 建窗口），
  需要改高度就 `.h(rems(规格值 / 16.))`。
- 欢迎窗口这条壳放在 `welcome.rs` 的 `render` 里 —— 它是"欢迎态长什么样"的唯一 owner。

### 不要这样做

- 不要 `#[cfg(target_os = ...)]` 自己画三个按钮：macOS 上会与系统红绿灯**重复**，
  而且违反"不写平台业务分支"。
- 不要只在窗口选项里留 `appears_transparent: true` 却什么都不画 —— 那就是本次这个缺陷
  （Windows 上等于把原生标题栏关掉、替补又没画）。
- 不要把窗口键画在欢迎内容自己的工具栏里：那一条工具栏在内容列（侧栏右侧）内部，
  不是窗口右上角，位置与工作区窗口的窗口键对不上。

### 二、`ShellWorkspace::new` 是"打开工作区"的唯一收口，它负责记录当前根

在 `ShellWorkspace::new` 建好 `Self` 之后立刻调 `workspace.record_opened_project(&root, None, cx)`。

- 覆盖所有打开路径：`Lithe <路径>` 启动、欢迎页点开、项目菜单换根、以后新增的入口 ——
  都是先 `ShellWorkspace::new` 再换根，不需要各自补记录。
- `record_opened_project` 复用既有实现：`RecentProjects::record_open`（去重 + 置顶 + 只保留
  12 条未置顶项）+ `save_recent_projects`（原子写、首次落盘时 `create_dir_all` 建出
  `%APPDATA%\Lithe`）+ 既有诊断行 `S1_SETTINGS_RECENT record=ok/saved`。
- 传 `open_in_new_window: None`：初始打开不知道"以后是否总在新窗口打开"，让既有条目的字段
  保持不变（`record_open` 在 `None` 时保留旧值）。
- 欢迎页点开、项目菜单换根那两处**保留**原有调用：它们带 `Some(false)`，语义是"用户刚决定的
  打开方式"。重复记录同一条只是把它再移到最前，结果不变。

### 不要这样做

- 不要把记录塞进 `app/src/main.rs` 的 `resolve_launch_plan`：那只覆盖"带路径启动"这一条，
  欢迎页点开与菜单换根会漏，而且启动根策略是纯函数（有单测钉着），不该产生文件写入。
- 不要在"读最近项目"的地方顺手写：读（`load_recent_projects`）与写（`record_opened_project`）
  分开，否则无参启动的只读分支会变成写盘分支。
- 不要把"打开工作区不记录"当成特性来依赖：`Lithe <路径>` 的**选根行为**（给什么根就用什么根、
  不检查存在性、不读最近项目文件）一字不改，本批只新增"顺手把它记进历史"这一个副作用。

## 考虑过的备选方案

### 备选方案一：欢迎窗口用 `TitleBar::window_options()` 的原生装饰（去掉 `appears_transparent`）

看起来最省事：让 Windows 自己画标题栏。没采用，因为 `WindowOptions.titlebar` 是**全平台**的
一处配置，去掉透明标题栏会连带改变 macOS 侧的红绿灯留白与双击行为，而且要动工作区窗口共用的
窗口选项；更要紧的是 gpui 在 Windows 上本来就不加 `WS_CAPTION`，这条路在事实上并不存在。

### 备选方案二：把欢迎页自己的工具栏（搜索框 / 打开按钮）抬进标题栏

旧版 0.5.12 的标题栏里确实有"打开项目"下拉与搜索框，看起来更"满"。没采用：那是把
`welcome.rs` 的整块布局重排（工具栏现在属于内容列、宽度也不含侧栏），超出本次缺陷范围，
而且 macOS 真源的欢迎页并没有这条标题栏；先把"没有窗口键"这个可用性缺陷修掉，
工具栏是否上提属于另一条产品决定。

### 备选方案三：迁移旧版 0.5.12 的最近项目

用户确实可能期望"新版能看到旧版那 5 条"。没采用：旧版是已删除的 Tauri 前端，它的历史存在
`%APPDATA%\app.lithe.windows\` 的 WebView2 localStorage（leveldb）里，不在 `settings.json`
里；解析另一套存储去喂新文件是一次跨产品数据迁移，风险与收益不对等。本批只保证"以后打开的
项目都会进历史"，迁移留给维护者决定。

### 备选方案四：只在"无参启动 + 最近项目为空"时把当前工作目录记进去

那样欢迎页至少不会空。没采用：它记录的不是"用户打开过的项目"，而是"程序随便开的目录"，
会把 `%APPDATA%\Lithe` 里塞进噪音；而且带路径启动这条主力路径照样没有历史。

## 后果

- 收益：欢迎窗口重新可用（三个窗口键 + 标题栏可拖动 + 双击最大化由上游接管）；带路径启动
  的项目会出现在欢迎页列表与项目菜单的"最近项目"里，下次无参启动直接进最近项目。
- 代价：`Lithe <路径>` 启动现在会**写** `recent-projects.json`（行为变化，已同步更新
  `implemented/feature/2026-09-27-launch-without-arguments.md` 里原来那句"也不会去动那份状态文件"）；
  欢迎窗口比原来多占 40pt 高的标题栏，内容整体下移。
- 代价：建壳路径上多一次小文件同步写（与既有的"打开后立即落盘设置 / 会话"同量级，
  不引入新的线程或阻塞点）。
- 需要重新评估的触发条件：上游 `TitleBar` 改动窗口键的 ElementId；或以后引入真正的不带标题栏
  的欢迎窗口设计（例如 macOS 风格的"内容顶到窗口上沿"），届时要在 `welcome.rs` 一处改壳。

## 验证

本批在 Windows 上实测，取证脚本与产物都在 `.artifacts/`（未纳入版本库）。

**单测与静态检查**（全部通过）：

```bash
cargo fmt --manifest-path rust/Cargo.toml --all -- --check      # 退出码 0
node lithe-gpui/tools/check-ui-px.mjs                          # 违规 0 处（新增的 40pt 用 rems 表达）
node lithe-gpui/tools/extract-locale.mjs --check                # 退出码 0（本批没有新增 tr 文案）
./.agents/skills/write-stable-tests/scripts/verify-test-stability.ps1
node scripts/verify-agent-notes.mjs
node .agents/skills/write-stable-tests/scripts/run-rust-tests-with-timing.mjs \
    --manifest rust/Cargo.toml --package lithe-gpui-workbench \
    --suite-timeout-ms 180000 --report .artifacts/test-stability/gpui-workbench.json
```

- `lithe-gpui-workbench` **204 条全绿**（报告：`.artifacts/test-stability/gpui-workbench.json`）；
  新增 `ui::welcome_window_renders_the_title_bar_above_its_content` 实测 93ms —— 它钉住
  "窗口壳的 id 存在、高 40 规格像素、在窗口最顶上、最近项目列表落在它下面"。
- `lithe-gpui-settings` 171 条、`lithe-gpui-app` 10 条全绿。
- ⚠️ 本机环境约束（与本次改动无关）：`cargo fmt --manifest-path rust/Cargo.toml -- --check`
  因为 `rust/Cargo.toml` 是虚拟 workspace 而报 "Failed to find targets"，要加 `--all`；
  另外会话沙箱下测试进程写不了真实 `%TEMP%`（`C:\Users\admin\AppData\Local\Temp`），
  既有用例 `ui::editor_pane_renders_a_real_file_and_reports_its_indent` 会以
  `PermissionDenied` 失败，把 `TEMP`/`TMP` 指到工作区内（`.artifacts/tmp`）后即通过。

**真机端到端**（debug 构建 + `LITHE_GPUI_SETTINGS_FILE` 指到 `.artifacts/e2e`，脚本
`.artifacts/verify-welcome-e2e.ps1`，20 条断言全过）：

| 场景 | 实测结果 |
| --- | --- |
| `Lithe <proj-a>`（带路径启动） | 诊断 `S1_SETTINGS_RECENT record=ok` + `saved`；`.artifacts/e2e/recent-projects.json` **被创建**（父目录由 `save_json` 的 `create_dir_all` 建出）且恰好 1 条 = `proj-a` |
| `Lithe`（无参，历史里有 `proj-a`） | `S1_WORKSPACE_LAUNCH source=recent root=…\proj-a` —— 历史真的被消费，直接进工作区 |
| `Lithe`（历史清空） | `source=welcome` + `S1_WELCOME view_open`，开出 800×650 欢迎窗口 |
| 欢迎窗口的三个窗口键 | 悬停扫描 `WM_NCHITTEST`：`HTCLOSE@x=1453`、`HTMAXBUTTON@1417`、`HTMINBUTTON@1373`、`HTCAPTION@1333`（窗口右沿 1461，物理像素，125% 缩放）—— 三个键都在、顺序正确，左侧还有拖拽区 |
| 点标题栏关闭键 | 真实鼠标点击后**窗口进程退出**（用户报的"关不掉"当场复现失败） |
| 视觉 | `.artifacts/e2e/welcome.png`（1002×814，可见帧）里能看到 40pt 标题栏与右上角 − □ ✕ |

单测只钉到"欢迎窗口渲染了窗口壳"这一层：gpui-kit 的 `TitleBar` 没有调用
`.test_support()`，它的三个窗口键（ElementId `minimize`/`maximize`/`close`）在测试观察
注册表里查不到，所以"按钮真的可点"由上面那次真机 `WM_NCHITTEST` + 真实点击取证，
不靠单测声称。跑完三场景后已确认无 `Lithe` 进程残留。

## 适用范围

- `rust/lithe-gpui/crates/workbench/src/welcome.rs` —— 欢迎窗口的窗口壳（`welcome_title_bar`
  与 `WELCOME_TITLE_BAR_HEIGHT_SPEC`）与内容布局。
- `rust/lithe-gpui/crates/workbench/src/workspace.rs` —— `ShellWorkspace::new` 里的
  `record_opened_project(&root, None, cx)`。
- `rust/lithe-gpui/crates/workbench/tests/ui.rs` —— 欢迎窗口窗口壳的回归判据。
- `rust/lithe-gpui/crates/app/src/main.rs` —— 只作为"窗口选项来自 `TitleBar::window_options()`"
  的事实依据，本批未改。
- `rust/lithe-gpui/UI-MAP.md` §2.2 ② —— 欢迎页那一行已按代码复核订正。
- `.agents/notes/implemented/feature/2026-09-27-launch-without-arguments.md` —— 显式路径那一支的
  记录口径已更新（新增「后续变更（2026-10-04）」小节）。

