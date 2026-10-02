# `lithe-gpui/` — GPUI Kit 宿主

> **先读 [`UI-MAP.md`](./UI-MAP.md)**（界面规格：**macOS 前端**逐区域 → gpui-kit 组件的对应表，
> 规格真源是 `macos/Sources/Lithe/Views/`）。
> 本文件只讲**怎么跑**、**已经验证过的接口事实**和**硬约束**。
> 交接现状见 [`HANDOFF.md`](./HANDOFF.md)，当前阻塞见 [`BLOCKERS.md`](./BLOCKERS.md)。

这个目录是「用 GPUI Kit 统一三端界面」方向的落地实现。宿主与 `lithe-core` 同在
`rust/` 这一个 Cargo workspace 里（`rust/Cargo.toml` 的 `members` 列出 `lithe-gpui/crates/*`）。

决策依据见
[`.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md`](../.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md)。

> ⚠️ **版本口径**：下面「已经验证过的接口事实」一节写于 **gpui-kit 0.6.6** 时期，而各 crate 现在声明的是
> `gpui-kit = "0.7"`。带 `0.6.6` 的源码路径与行号只能当"当时的依据"读，**结论要按 registry 里已解析的
> 0.7 源码重新核对**，尤其是 `Root` 与浮层挂载相关的部分（0.7 由 `Root` 的 plugin 层统一渲染浮层）。

## 跑起来

```powershell
cargo build --manifest-path rust/Cargo.toml -p lithe-gpui-app --bin Lithe
# 参数 = 工作区根
.\rust\target\debug\Lithe.exe D:\developmentProjects\rust\Lithe-IDEA
```

`--theme` / `--locale` 是**显式覆盖**（只作用于这一次启动、**不写回设置文件**，供 `.artifacts/` 的
4 配置视觉验证与诊断用）；`--open-settings` 启动后自动打开设置对话框（验证/诊断用）。
主题与界面语言的常规来源是**设置文件**（见下）：

```powershell
.\rust\target\debug\Lithe.exe D:\developmentProjects\rust\Lithe-IDEA --theme "Lithe Light"
.\rust\target\debug\Lithe.exe D:\developmentProjects\rust\Lithe-IDEA --locale en
.\rust\target\debug\Lithe.exe D:\developmentProjects\rust\Lithe-IDEA --open-settings
```

- 窗口尺寸从主显示器的可见区域算（`startup_window_bounds`），**不写死机器路径**；
- 界面数据来自 Rust Core（`core.ping` / `workspace.snapshot`），跑在后台线程，窗口期间保持可交互；
- 截图证据必须用 **DPI 感知**的抓取方式（不感知只会拿到虚拟化尺寸，证据会缺一大截）。
  ⚠️ 早先引用的 `lithe-gpui/capture-screenshot.ps1` **当前工作区里不存在**，需要时自己写一个：

  ```powershell
  # 自备脚本，进程内先声明 per-monitor-v2 DPI 感知，再抓目标窗口
  ```

## 设置（`crates/settings`）

设置文件位置（`paths.rs`；gpui / gpui-kit 都**没有**数据目录 helper，所以平台分支集中在那个模块）：

| 平台 | 路径 |
| --- | --- |
| Windows | `%APPDATA%\Lithe\settings.json` |
| macOS | `~/Library/Application Support/Lithe/settings.json` |
| Linux | `$XDG_CONFIG_HOME/lithe/settings.json`（回落 `~/.config/lithe/settings.json`） |

- **`LITHE_GPUI_SETTINGS_FILE`**（完整文件路径）优先于一切平台推导 —— 测试与机器验证用它把设置文件
  指到临时文件，不污染真实用户目录：

  ```powershell
  $env:LITHE_GPUI_SETTINGS_FILE = "$env:TEMP\lithe.json"
  Set-Content $env:LITHE_GPUI_SETTINGS_FILE '{"theme": "Lithe Light"}'
  .\target\debug\Lithe.exe D:\developmentProjects\rust\Lithe-IDEA
  # stdout: S1_THEME applied=Lithe Light dark=false  ← 设置文件 → 主题这条链生效的证据
  ```

- 文件不存在时**不创建**（改动才写）；解析失败/某个键坏了只回落默认值并打 `S1_SETTINGS …` 诊断；
  写入是**原子写 + 300ms 防抖**（重置/恢复默认立即写）。
- 入口：左侧活动栏第 9 项「设置」或 `Ctrl+,`；`--open-settings` 供机器验证。
- v1 只有「常规」（语言，**重启后生效**）与「外观」（配色主题 / 外观模式 / 界面字号 / 显示状态栏，
  全部立即生效）两页；其余 10 个分类的取舍与前置条件原先记在 `PLAN.md` §8 —— **该文件已不在工作区**，
  需要时从 git 历史取回。

## 目录结构

按 gpui-kit《编码指南》"**按业务能力组织 crate、依赖只向下**"分层
（上游那份 `coding-guides.md` 的本地镜像 `lithe-gpui/docs/gpui-kit/**` **已不在工作区**；
按需查 `https://gpui-kit.com` 或 registry 里的源码）。依赖图：

```text
app → workbench → {editor, explorer, git, terminal} → shared
app → settings → {shared}
workbench → settings
```

| crate | 职责 |
| --- | --- |
| `crates/app/` | **App Shell**：只组合窗口与 Feature —— 命令行参数、窗口尺寸、启动顺序、设置加载与主题应用。bin 名仍是 `Lithe` |
| `crates/workbench/` | 工作台外壳：标题栏 / 项目标签条 / 活动栏 / 状态栏 + 中央列组装（`ShellWorkspace`） |
| `crates/explorer/` | 左侧栏「项目」：真实 `workspace.snapshot` 的多层树（`model.rs` + `explorer_view.rs`） |
| `crates/editor/` | 编辑区：标签栏 + 正文 / 空状态（`buffer.rs` + `editor_view.rs`） |
| `crates/git/` | 底部工具窗「提交记录」：引用 / 提交 / 详情三栏（`model.rs` + `log_view.rs`） |
| `crates/terminal/` | 终端：页签 + 流式输出 + 输入行 + 最小 ANSI 清洗（`profile` / `ansi` / `session` / `terminal_view`） |
| `crates/settings/` | 设置：模型（`schema`）/ 路径（`paths`）/ 持久化（`persistence`）/ 主题应用（`theme`）/ 状态与副作用（`store`）/ 行与分组（`row`）/ 对话框（`dialog`）。**只向下依赖** `gpui-kit` + `shared` + serde |
| `crates/shared/` | 跨 Feature 的稳定能力：Core 信封层（`core_client`）+ i18n 包装（`i18n`），`locales/` 也在这里 |

`lithe-gpui/` 根下还有：`themes/`（运行时按目录加载并监听的 Lithe 主题）、`tools/`
（`extract-locale.mjs` 从**仓库内置**的文案真源 `crates/shared/locales/source/*.ts` 生成 i18n 资源；
`generate-idea-icons.mjs` + `idea-icon-mappings.json` 生成图标；`check-ui-px.mjs` 守硬编码 px）、
`assets/`、`extensions/`。

> 早先这里还列过 `docs/`（上游 0.6.6 文档镜像）、`research/`（Windows 调研原文）与
> `capture-screenshot.ps1`——**这三项目前都不在工作区里**，Windows 调研也已随前端失效。

## 已经验证过的接口事实（写于 0.6.6，宿主现已升到 0.7）

判断依据一律是**已发布源码**，读本机 cargo registry 里已解析的版本（版本号以 `rust/Cargo.lock` 为准；
本节的行号来自 0.6.6，升级到 0.7 后需要重新核对）。

### 启动顺序（0.6.6 的写法，0.7 请核对 `Root` 的新行为）

```rust
gpui_kit::application().with_assets(gpui_kit::assets::AllAssets).run(move |cx| {
    gpui_kit::component::set_locale("zh-CN");   // 组件文案用它自带的 zh-CN
    gpui_kit::init(cx);
    Theme::change(ThemeMode::Dark, None, cx);   // Lithe 默认深色；init 默认浅色
    // 设置：读到的值决定主题与语言；主题目录在这里装载并监听。
    lithe_gpui_settings::init_store(cx, Init { loaded, theme_override });
    lithe_gpui_settings::watch_lithe_themes(cx);        // ThemeRegistry::watch_dir + apply_config
    lithe_gpui_settings::install_actions(cx);           // Ctrl+, → 打开设置对话框
    cx.spawn(async move |cx| {
        cx.open_window(WindowOptions { ..TitleBar::window_options() }, move |window, cx| {
            let view = cx.new(|cx| ShellWorkspace::new(root, window, cx));
            if let Some(store) = lithe_gpui_settings::try_store(cx) {
                store.update(cx, |store, cx| store.attach_window(window, cx));  // 系统外观监听
            }
            cx.new(|cx| Root::new(view, window, cx))    // Root 必须在最外层
        }).expect("failed to open window");
    }).detach();
});
```

⚠️ 浮层**只能在事件回调或任务里打开**：`Root::new` 之前窗口根还不是 `Root`，而 `render` 阶段调
`window.open_dialog` 会 panic（`shell/overlays.md:203-213`）。`--open-settings` 用的是
`window.on_next_frame`（首帧之后）。

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

截图必须用 **DPI 感知**的抓取方式（进程内先声明 per-monitor-v2 DPI 感知）；DPI 不感知的抓法只会拿到虚拟化尺寸（1458×819），**证据会缺一大截**，曾因此把"内容只占 78%"误判成"已经铺满"。⚠️ 早先引用的 `lithe-gpui/capture-screenshot.ps1` **当前工作区里不存在**，需要时自己写一个（或从 git 历史取回）。

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
- **版本按实版声明 + 提交 `Cargo.lock` 保证可复现**。各 crate 现在写的是 `gpui-kit = "0.7"`（不是精确钉版本），
  可复现性由**提交进仓库的 `rust/Cargo.lock`** 保证：只有显式 `cargo update` 才会移动版本。
  上游窗口创建 API 变更过，升级时逐处复核。
- **不写平台专属分支**：目标是一套代码三端用，`#[cfg]` 不进业务逻辑。
- **架构观察**：编辑器内容宿主、终端、Core 状态机都在演化，任何"临时实现"都要在注释里写清归属与后续接线点。

## 验证状态

| 平台 | 状态 |
| --- | --- |
| Windows | 历史上的构建通过、真实 Core 数据、界面按 `UI-MAP.md` 逐区域对齐；证据在 `.artifacts/p1/`（该目录不入库）。 |
| macOS / Linux | **未验证**：本方向只在 Windows 上验证过；三端构建与 CI 属路线图 P6。**不要声称 macOS 已验证。** |

> ⚠️ **当前构建状态（2026-10-02 实测）**：
>
> 1. **已构建通过**：
>
>    ```text
>    cargo build --manifest-path rust/Cargo.toml -p lithe-gpui-app --bin Lithe
>    → Finished `dev` profile [unoptimized + debuginfo] target(s) in 6m 26s
>    → rust/target/debug/Lithe.exe（约 102 MB）
>    ```
>
> 2. 为让"main 的 `lithe-core` + 本分支 gpui 宿主"能同时构建，本次动了四处（**都不改 `lithe-core` 的业务逻辑**）：
>
>    | 调整 | 原因 |
>    | --- | --- |
>    | `rust/Cargo.toml` 的 `members` 补上 `lithe-agent-host`、`lithe-ide-host` | 它们是 `lithe-core` 的 path 依赖，缺了连 manifest 都解析不了 |
>    | `rust/lithe-core/Cargo.toml`：`tree-sitter` `0.25` → `0.26` | gpui-kit 0.7 依赖 `tree-sitter 0.26.13`，两者都声明 `links = "tree-sitter"`，0.25 与 0.26 不能共存 |
>    | `rust/lithe-core/src/languages/mybatis.rs:220`：`node.child(index as u32)` | tree-sitter 0.26 的 `Node::child()` 收 `u32`（0.25 收 `usize`） |
>    | `shared/` 从 `main` 同步（+22 个文件、改 9 个，未删任何分支独有文件） | `lithe-core` 的测试用 `include_str!` 引用 main 上才有的 fixture |
>
> 3. **`lithe-core` 测试：762 条里 761 条通过，1 条失败**。
>    `git::history::tests::date_order_pages_interleave_branches_and_preserve_cursor_policy` 在
>    `history.rs:952` 的 `create_dir_all` 报 `PermissionDenied (os error 5)`。已排除环境能力问题
>    （独立的 Rust 程序能在同一临时目录建目录），判定为该测试的隔离/清理逻辑在本机受限，与本次
>    `tree-sitter` 调整无关（涉及 tree-sitter 的 java / mybatis 测试全部通过）。
>
> 4. ⚠️ **仓库的 `scripts/verify-*.sh` 系列在本机跑不起来**：它们依赖 `rg`、`ruby`、`awk`，
>    而本机三者都缺（`find` 还是 Windows 自带的 `find.exe`）。用 Node 写的
>    `scripts/verify-agent-notes.mjs` 可以正常跑。

`.artifacts/` 已被仓库根 `.gitignore` 忽略；构建日志、截图、Core 原始响应都写在那里。
