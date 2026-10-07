# `lithe-gpui/` — GPUI Kit 宿主

> **先读 [`UI-MAP.md`](./UI-MAP.md)**：界面规格（**macOS 前端**逐区域 → gpui-kit 组件对应表，
> 规格真源是 `macos/Sources/Lithe/Views/`）+ §1.4 的 27 组官方指南条文与逐条现状核查。
> 本文件只讲**怎么跑**、**已验证的接口事实**、**硬约束**。
> 交接状态见 [`HANDOFF.md`](./HANDOFF.md)；已知卡点见 [`BLOCKERS.md`](./BLOCKERS.md)；
> 任务清单见 [`../../docs/tasks/gpui-core-first.md`](../../docs/tasks/gpui-core-first.md)。

宿主与 `lithe-core` 同在 `rust/` 这一个 Cargo workspace 里（成员见 `rust/Cargo.toml` 的 `members`）。
决策依据：`.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md`。

**版本**：`gpui-kit` / `gpui-kit-assets` `0.7.0`、GPUI 快照 `gpui-pre` `0.3.7`（真源是 `rust/Cargo.lock`）。
本仓库内有一份官方文档镜像：**`docs/gpui kit/0.7.0/`**（184 份 md：`docs/` 41 · `component/` 78 · `base/` 49 · `shell/` 15）。
查 API 先查镜像，再查 registry 里已解析的源码。

## 跑起来

```powershell
cargo build --manifest-path rust/Cargo.toml -p lithe-gpui-app --bin Lithe
.\rust\target\debug\Lithe.exe D:\developmentProjects\rust\Lithe-IDEA   # 参数 = 工作区根（可省略，见下）
```

省略位置参数时打开「最近项目」里最近且仍存在的那个；列表空或全失效则回落到当前工作目录。

诊断/验证用开关（都只作用于这一次启动，**不写回设置文件**）：
`--theme <名>`、`--locale <码>`、`--open-settings`、`--appearance-revert <键名>`、`--right-view <id>`、`--menu-probe`。

- 窗口尺寸从主显示器可见区域算（`startup_window_bounds`），不写死机器路径；
- 界面数据来自 Core（`core.ping` / `workspace.snapshot`），跑在后台任务里，窗口期间保持可交互；
- 诊断一律走 **stderr**（`eprintln!`）：stdout 重定向到文件时是块缓冲，进程还在跑时可能一行都没有。

## 设置（`crates/settings`）

| 平台 | 路径 |
| --- | --- |
| Windows | `%APPDATA%\Lithe\settings.json` |
| macOS | `~/Library/Application Support/Lithe/settings.json` |
| Linux | `$XDG_CONFIG_HOME/lithe/settings.json`（回落 `~/.config/lithe/settings.json`） |

- `LITHE_GPUI_SETTINGS_FILE`（完整文件路径）优先于一切平台推导，测试与机器验证用它把设置文件指到工作区外，不污染真实用户目录。
- 文件不存在时**不创建**（改动才写）；解析失败/单键坏了只回落默认值并打 `S1_SETTINGS …`；写入是原子写 + 300ms 防抖。
- 入口：左侧活动栏「设置」项或 `Ctrl+,`。
- 已实现 **8 页**：常规 / 外观 / 编辑器 / 终端 / 项目 · JDK 与 Maven / LSP / 运行配置 / Git；
  其余分类是**明确空态**（只有页标题 + 「此分类尚未接入」+ 前置条件，一个控件都没有），由测试钉住。

## 目录结构

```text
app        → workbench, settings, shared
workbench  → editor, explorer, extensions, git, java, notify, run, settings, shared, terminal, lithe-gpui
editor     → java, notify, shared
settings   → run, shared
explorer   → lithe-gpui, shared
extensions · git · java · notify · run · terminal → shared
shared     → （无下游兄弟）
lithe-gpui → （仓库内无下游兄弟：上游组件库的 fork，见 crates/lithe-gpui/UPSTREAM.md）
```

> `lithe-gpui` 是仓库内的组件库（Ely GPUI Components 的 vendored fork）。外壳的三块工具窗
> （`layout::Dock`）与左侧项目树（`lists::ProjectTree`）都用它；它不依赖任何宿主 crate，
> 宿主之间的依赖方向因此不变。

| crate | 职责 |
| --- | --- |
| `crates/app/` | **App Shell**：命令行参数、窗口尺寸、启动顺序、设置加载与主题应用。bin 名是 `Lithe` |
| `crates/workbench/` | 工作台外壳：标题栏 / 项目标签条 / 活动栏 / 状态栏 + 中央列组装（`ShellWorkspace`）、命令面板、主菜单栏、扩展页 |
| `crates/editor/` | 编辑区：标签栏 + 正文 + 补全 + 诊断波浪线 + `Ctrl+.` 快速修复（`buffer` / `editor_view` / `completion` / `diagnostics` / `code_actions` / `navigation`） |
| `crates/explorer/` | 左侧栏「项目」：真实 `workspace.snapshot` 的多层树 |
| `crates/git/` | Git 功能：底部工具窗「提交记录」+ 左栏「更改」（变更列表 / 暂存 / 提交）+ 分支弹窗数据 |
| `crates/java/` | JDTLS 会话、工作区指纹、Maven context、单一事件泵 |
| `crates/run/` | 运行配置与运行面板 |
| `crates/notify/` | 通知中心 store 与 model |
| `crates/terminal/` | 终端页签 + 流式输出 + 输入行 + 最小 ANSI 清洗（**无 PTY、无 VT 模拟器**，见 UI-MAP §2.2 ⑨） |
| `crates/extensions/` | 扩展目录状态：Core `plugins.listCatalog` 的响应快照、Core 桥、搜索与分类过滤 |
| `crates/settings/` | 设置模型 / 路径 / 持久化 / 主题应用 / 状态与副作用 / 行与分组 / 对话框 |
| `crates/shared/` | 跨 feature 稳定能力：Core 信封层（`core_client`）、i18n 包装、图标、工作区配置；`locales/` 也在这里 |

根下还有：`assets/`（内嵌图标与品牌资源，经 `crates/app/src/assets.rs` 的 `LitheAssets` 暴露）、
`extensions/`（`themes/` 与 `fonts/` —— 主题与字体是**可卸载的捆绑扩展**）、
`themes/`（遗留目录，仅 `jetbrains.json` + `README.md`）、
`tools/`（`extract-locale.mjs` 生成 i18n / `generate-idea-icons.mjs` 生成图标 / `check-ui-px.mjs` 守硬编码 px）。

## 已经验证过的接口事实

判断依据一律是**已发布源码**（本机 cargo registry 里已解析的版本）。**带行号的条目写于 0.6.6 时期，
行号已失效，结论仍成立**；升级到 0.7 后需按 0.7 源码重新定位。

### 启动顺序

```rust
gpui_kit::application().with_assets(assets::LitheAssets).run(move |cx| {
    gpui_kit::component::set_locale("zh-CN");     // 组件文案用它自带的 zh-CN
    gpui_kit::init(cx);                            // MUST be first
    Theme::change(ThemeMode::Dark, None, cx);      // Lithe 默认深色；init 默认浅色
    lithe_gpui_settings::register_fonts(cx);       // 打包字体必须在首帧之前注册，否则排版 panic
    lithe_gpui_settings::watch_lithe_themes(cx);
    lithe_gpui_settings::install_actions(cx);      // Ctrl+, → 打开设置
    cx.open_window(WindowOptions { ..TitleBar::window_options() }, move |window, cx| {
        let view = cx.new(|cx| ShellWorkspace::new(root, window, cx));
        cx.new(|cx| Root::new(view, window, cx))   // Root 必须是最外层
    })
});
```

- ⚠️ 浮层**只能在事件回调或任务里打开**：`Root::new` 返回之前窗口根还不是 `Root`，`render` 阶段调
  `window.open_dialog` 会 panic。`--open-settings` 用 `window.on_next_frame`（首帧之后）。
- ⚠️ **两个 `open_window` 语义不同**：`cx.open_window` 要求闭包返回**窗口根视图**，所以必须自己包 `Root`；
  `gpui_kit::open_window`（`gpui-kit-0.7.0/src/lib.rs:144`，**0.7 才存在**）内部已包 `Root`，
  闭包只返回应用内容、不能再包一层。本项目用前者。
- `Application::with_assets` 是 `impl AssetSource`，**重复调用是覆盖**（`gpui-pre-0.3.7/src/app.rs:202`）。
- 0.7 的浮层由 window-root `Root` 的 plugin 层每帧渲染；**已移除的 `Root::render_*_layer` 不要再调**。

### 尺寸

**根视图用 `.size_full()`，不要自己换算缩放比。**

`viewport_size()` 已经是 DPI 无关的逻辑单位，再除 `scale_factor()` 会让内容只画到窗口的一部分
（本项目踩过，曾据此把"内容只占 78%"误判成"已经铺满"）。实测对账：真屏 1920×1080 + 125% 缩放时，
窗口 1823×1024 物理，而 `viewport_size()` 报 1444×812、`scale_factor()` 报 1.25 —— **布局单位就用
`viewport_size()` 那一套，不乘不除**。截图必须用 **DPI 感知**的抓取方式，否则只拿到虚拟化尺寸。

### Dock / 面板

```rust
let (area, skin) = DockSkin::dock_area("lithe-workbench", Some(1), window, cx);
let layout = DockLayout::h_split()
    .child(DockLayout::tabs().panel_view(panel_handle(panel), cx), Some(rems(320. / 16.)))
    .child(DockLayout::v_split().child(a, None).child(b, Some(rems(260. / 16.))), None);
area.update(cx, |area, cx| area.set_center(layout, window, cx));
```

- 面板 = `impl BasePanel`（`panel_name`）+ `impl Panel`（`title`）+ `Focusable` + `EventEmitter<PanelEvent>` + `Render`；
- `Panel::title_bar()` / `inner_padding()` 返回 `false` → Dock 不再画自己的标题栏/内边距，面板可自绘头部；
- `DockPlacement::Bottom` **只横跨中心列**；要"底窗横跨左栏 + 编辑区"就把底部放进 center 的 `v_split`（本项目采用）。
- 分栏手柄用上游 `resize_handle`（按方位取 ID，**三个手柄不能共用同一 ID**，否则会互相触发拖动）。

### 组件要点（踩过的坑）

| 事实 | 出处 |
| --- | --- |
| `ListItem` 的 children **是竖排**，多个子元素必须自己套 `h_flex()` | `list/list_item.rs:215-221` |
| `uniform_list` 的行高 = **第 0 行内容的测量高度**，`.h()` 压不住内容，行内纵向 padding 会溢出到相邻行 | `gpui-pre-0.3.6/src/elements/uniform_list.rs:359-371,508-509` |
| `ListItem` 默认 `py_1()/px_3()`；紧凑行要 `py_0()` | `list/list_item.rs:186-187` |
| `DataTable` 行高是表级值（默认 `Size::Medium ⇒ 32`），但表体行高同样受首行内容影响 | `sizing.rs:57-65` |
| `render_td` 的返回值会被再包一层（固定列宽 + `overflow_hidden` + `whitespace_nowrap`） | `table/state.rs:1323-1329` |
| `Column` 只有 `width/min_width/max_width`，**没有 flex**；列宽按给定值比例铺满表宽 | `table/column.rs:151-215` |
| `TitleBar` / `StatusBar` 都实现 `Styled`，macOS 的 40 / 24 用 `.h(rems(40. / 16.))` / `.h(rems(24. / 16.))` 覆盖（默认 34 / 自定）。**不要写 `px(40.)`** —— 档位用 `h_10()` 之类助手，档位外用 `rems(P / 16.)`，见 UI-MAP §1.1 | `title_bar.rs` |
| `Tab::icon()` 会丢掉 label，图标 + 文字要走 `Tab::child(..)`；`TabBar::max_width()` 是文字能截断的前提 | `tab/tab.rs:719-744` |
| `Button::ghost()/disabled()/xsmall()` 存在，但分别在 `ButtonVariants`/`Disableable`/`Sizable` 三个 trait 上，**必须 `use ... as _`** | `button/button.rs:44-96,503-537` |
| `IconName` 是 `Clone` 不是 `Copy` | `component/icon.rs:17-20` |
| ⚠️ `Task` **没有 `then`**：`gpui-pre-scheduler-0.3.7` 的 `Task` 只有 `ready`/`is_ready`/`detach`/`fallible`/`downcast`；链式要包进 `cx.spawn(async move \|this, cx\| { …await… })` | `gpui-pre-scheduler-0.3.7/src/executor.rs:527` |
| `Dialog` 与 `Sheet` **自带焦点陷阱**，所以项目里 `focus_trap` 零处是正确用法，不是缺口 | `UI-MAP.md` §1.4 M |

### 编辑区不需要 WebView

gpui-kit 自带代码编辑器引擎：`component::input::EditorState` 已有行号列、折叠、缩进线、tree-sitter 高亮、
诊断波浪线、装饰、多光标、撤销栈、内置查找替换（含 `2/3` 计数），`InputEditorStyle::{editor_active_line,
editor_gutter_background}` 可配。缺 minimap、overview ruler、sticky scroll、gutter 装饰渲染器、code lens。

⚠️ macOS 侧编辑内容是 WKWebView 里的 Monaco（`macos/Sources/Lithe/Views/Editor/CodeEditorView.swift`
是**零调用点**的原生实现，不要当成 macOS 规格）。**WebView 会盖住同矩形内所有 GPUI 浮层**
（popover / dialog / menu / tooltip）—— 所以 gpui 侧不要把浮层压在 WebView 上。

### gpui-kit 没有的能力（要自研或引第三方）

终端模拟器 / PTY / ANSI 解析·并排 diff·提交图泳道·表格与列表的多选·面包屑之外的大多数 IDE 专属装饰。
（终端当前实现是 `std::process::Command` + 三管道 + 丢控制序列，**不是真终端**，见 UI-MAP §2.2 ⑨。）

## 约束

- ⛔ **不要修改 `rust/lithe-core/`**：它从 `main` 同步进来，本分支改它会被下次同步覆盖或冲突；
  Core 缺口**只上报**。确定性逻辑属于 Core 这条架构判据本身没有变 —— 所以这类活是**跳过**，不是搬进 gpui。
  范围约束与任务归属见 `docs/tasks/gpui-core-first.md` 顶部。
- **只以 `gpui-kit` 为准**。`gpui-elements`、`gpui-macros`（第三方 skill）教的是裸 GPUI 写法，
  会跳过 `gpui-kit` 的 `Root` 挂载，浮层与弹窗失效。
- **版本按实版声明 + 提交 `Cargo.lock` 保证可复现**：各 crate 写 `gpui-kit = "0.7"`（不是精确钉版本），
  可复现性由**提交进仓库的 `rust/Cargo.lock`** 保证。
- **不写平台专属分支**：目标是一套代码三端用，`#[cfg]` 不进业务逻辑。
- **颜色只走 `cx.theme()`**；**布局不直接写 `px(...)`**（守卫脚本 `node lithe-gpui/tools/check-ui-px.mjs`）。
- **架构观察**：编辑器内容宿主、终端、Core 状态机都在演化，任何"临时实现"都要在注释里写清归属与后续接线点。

## 验证状态

| 平台 | 状态 |
| --- | --- |
| Windows | 构建通过、真实 Core 数据、界面按 `UI-MAP.md` 逐区域对齐 |
| macOS / Linux | **未验证**。**不要声称 macOS 已验证。** |

### UI 集成测试层（`D26`）

`crates/workbench/tests/` 下是**能在无头窗口里跑真实交互**的测试层（此前 `#[cfg(test)]`
只有纯函数单测）。由 `gpui-kit` 的 `test-support` feature 提供，按上游
`docs/gpui kit/0.7.0/docs/test.md` 的口径接：

```powershell
cargo test --manifest-path rust/Cargo.toml -p lithe-gpui-workbench --test ui
```

- feature **只在 `[dev-dependencies]` 里打开**（`crates/workbench/Cargo.toml`），
  这样生产制品不会被装上观察包装 —— 上游 `test.md` 的硬要求；
- `tests/common/mod.rs` 是窗口夹具（形状照 `gpui-kit-0.7.0/tests/common/mod.rs` 抄）。
  上游明确说这两个文件**必须成对**：只拷 `ui.rs` 会在 `mod common;` 处失败；
- 断言走组件自己的公开访问器 +「窗口真的渲染出一帧」，**不为了好写而给生产渲染树加
  `.test_support()` / `aria_label`**（`test.md` 明确劝阻"仅为让视觉断言通过而加 aria_label"）。
  要在渲染树里查询控件时才加 `.test_support()`，它不增加布局盒子，且关闭 feature 时是空操作。

**构建（2026-10-02 实测）**：

```text
cargo build --manifest-path rust/Cargo.toml -p lithe-gpui-app --bin Lithe
→ Finished `dev` profile … in 6m 26s → rust/target/debug/Lithe.exe（约 102 MB）
```

为让"main 的 `lithe-core` + 本分支 gpui 宿主"同时构建，当时动了四处（**都不改 Core 业务逻辑**，已在
`29a5a771` 提交）：`rust/Cargo.toml` 的 `members` 补 `lithe-agent-host`/`lithe-ide-host`；
`lithe-core` 的 `tree-sitter` `0.25`→`0.26`（与 gpui-kit 的 `0.26.13` 在 `links = "tree-sitter"` 上不能共存）；
`mybatis.rs` 的 `node.child(index as u32)`；`shared/` 从 main 同步（`lithe-core` 测试用 `include_str!` 引用）。

`lithe-core` 测试 762 条里 **761 通过、1 失效**：
`git::history::tests::date_order_pages_interleave_branches_and_preserve_cursor_policy` 在
`history.rs:952` 的 `create_dir_all` 报 `PermissionDenied (os error 5)`；已排除环境能力问题，
判定为该测试的隔离/清理逻辑在本机受限。

⚠️ **仓库的 `scripts/verify-*.sh` 系列在本机跑不起来**：依赖 `rg`、`ruby`、`awk`，本机三者都缺
（`find` 还是 Windows 自带的 `find.exe`）。Node 写的 `scripts/verify-agent-notes.mjs` 可以跑。
