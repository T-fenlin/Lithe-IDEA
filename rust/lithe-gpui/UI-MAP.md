# Windows 界面 → gpui-kit 复刻对应表（UI-MAP）

> ## 🔁 规格来源变更（2026-09-25，维护者决定）
>
> **界面规格来源改为 Windows 前端**（`windows/tauri/src/features/*`，Tauri v2 + React + Tailwind + shadcn）：
> 尺寸、布局、层级、观感一律照 Windows。**实现框架不变，仍是 gpui-kit。**
>
> 因此本文件分两部分使用：
> - **§1 硬规则与 gpui-kit 实现规则**：**继续有效**（单位、token 映射、行高、编辑器高度、滚动条、浮层挂层、DPI 与截图口径…都是真机踩出来的）；
> - **§2 逐区域对应**：目前是以 **macOS** 为源写的，**降级为"行为/功能对照"**（查"这个交互原本怎么工作"）。
>   视觉与布局要改看 Windows —— 新的对应表落在 **`lithe-gpui/UI-MAP-WINDOWS.md`**（提示词：`docs/development/gpui-ui-windows-rewrite-prompt.md`）。
>
> `lithe-gpui/PLAN.md` 顶部的横幅与 `.agents/notes/.../2026-09-23-...roadmap.md` 的状态变更是同一件事的记录。

这份文档是 GPUI 外壳的**界面规格**：左边是 macOS 端（`macos/Sources/Lithe`，SwiftUI/AppKit）的**真实界面**，右边是用 **gpui-kit 0.6.6** 复刻时"用哪个组件、怎么用、差在哪"。

工作方式（维护者 2026-09-24 定，2026-09-25 修订规格来源）：

- **界面规格取自 Windows 前端源码**（`windows/tauri/src/features/*`）；macOS 端只作行为/功能对照；
- **实现只能用 gpui-kit 0.6.6 源码里真实存在的组件与基元**（判断依据是已发布源码，不是在线文档——在线文档描述的是未发布的 0.7.0）；
- 做得不好的地方**允许**用 gpui-kit 做得更好，但必须在"优化"一栏写清理由。

配套文件：

| 文件 | 作用 |
| --- | --- |
| `PLAN.md` | 执行计划：下一步做什么、怎么算做完、当前差距 |
| 本文 `UI-MAP.md` | **§1 硬规则 + gpui-kit 实现规则**（仍然有效的部分）+ §2 指向 Windows 规格 + §3 验收清单 |
| `research/windows/01..06-*.md` | **Windows 界面调研（规格真源，6 份）**：外壳 / 编辑区+侧栏 / Git+底部窗 / 主题+组件 / 终端+运行调试 / 数据库+AI+搜索 |
| `research/gpui-kit-overlay-howto.md` | gpui-kit 浮层（Dialog/Sheet/Notification）怎么用的完整调研 |
| `docs/archive/ui-map-macos.md` | macOS 界面调研，**只作行为/功能对照**（不是视觉规格） |
| `docs/development/gpui-ui-windows-rewrite-prompt.md` | 交给新对话的**重写提示词**（决策 + 坑清单 + 分阶段 + 验收） |
| `.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md` | 为什么这样做的决策记录 |

> 清理说明（2026-09-25）：早期调试期的截图证据（`.artifacts/p1/*`）与 macOS 原始区域调研（`.artifacts/ui-map/01..08`）
> 已删除 —— 前者里有**坏截图工具产出的误导性画面**（例如 `startup-max.png` 把"内容只占 78%"拍成"已铺满"），
> 后者已被 `docs/archive/ui-map-macos.md` 汇总。本文的结论都以**文字 + 源码行号**写清，需要复现时按文中命令重跑即可。

---

## 1. 硬规则（每条都核对过源码或踩过坑）

### 1.1 通用

1. **单位可以原样搬**：macOS 的 1pt = 1 逻辑像素 = gpui 的 `px(1.)`。macOS 写 `40`、`14`、`size-3`，gpui 就写 `px(40.)`、`px(14.)`、`size_3()`。`window.scale_factor()` 只和物理像素有关，不影响这套数值。
2. **颜色与字号一律走主题 token**（`cx.theme()` / `cx.theme().tokens`），不写裸色值；映射见 §1.2。
3. **文案取 macOS 的本地化原文**（`macos/Resources/zh-Hans.lproj`），不要自己编中文。
4. **图标**：⚠️ **本条已过时（2026-09-25 更正）**。应用现在注册的是**全量** `gpui_kit::assets::AllAssets`（`gpui-kit-assets-0.6.6/src/lib.rs:36`、`native_assets.rs:9-11`），嵌入 `assets/icons/` 下的 **1830 个 Lucide 字形**（含 18 个 `git*.svg`）—— 所以**没有"缺字形"这回事**了，也不需要近似替代。
   仍然要记住的两点：① 有**两个** `IconName` —— `gpui_kit::component::IconName` 是只有 **101** 个变体的兼容子集（`gpui-component-0.6.6/src/icon.rs:18`，由 `build.rs` 按 `default-icons.txt` 过滤生成），要用真字形就得用 `gpui_kit::assets::IconName`（1830 个）；② `Icon::new` / `Button::icon` 收 `impl Into<Icon>`，`impl<T: IconNamed> From<T> for Icon`（`gpui-component-0.6.6/src/icon.rs:59`）对两者都成立，所以换类型即可、不用改写法。变体名 = svg 文件名的 PascalCase（`build.rs:20-51`）；`trash-2.svg` 不存在，所以没有 `Trash2`，用 `Trash`。
5. **不写平台业务分支**：不给 macOS/Windows 写 `#[cfg]` 业务代码。平台差异（macOS 红绿灯留白 76pt、Windows 最小化/最大化/关闭按钮位置）由 gpui 平台层与 `TitleBar` 自身处理，界面结构三端统一。
6. **一个区域一个模块**：外壳按区域拆分（`lithe-gpui/shell/src/bin/shell_probe/` 下每区域一个文件），便于并行与替换。
7. **缺 API 先读源码**：`D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-{kit,component,base}-0.6.6\src\`。

### 1.1.1 版本陷阱（0.7.0 文档 vs 0.6.6 实际）

在线文档与技能文档描述的是**未发布的 0.7.0**。下面这份清单是核对已发布源码后的结论，写代码前先看它：

| 结论 | API | 依据 |
| --- | --- | --- |
| 0.6.6 **没有** | `gpui_kit::open_window` / `Window::open_window` | `open_window` 只在 `App`（`gpui-pre-0.3.6/src/app.rs:1347`）与 `AsyncApp` 上 |
| 0.6.6 **没有** | `Task::then`、`Theme::tab_active_bg` | 源码内无此方法 |
| ⚠️ **本条曾经写错，已更正** | `.overflow_y_scrollbar()` **是存在的** | 在 `gpui-component-0.6.6/src/scroll/scrollable.rs:60`（`fn overflow_y_scrollbar(self) -> Scrollable<Self>`）。早先写的"0.6.6 没有它"是错的；只设 `overflow = Scroll`、要自己挂 `ScrollHandle` 的那个是 gpui `InteractiveElement::overflow_y_scroll`（`gpui-pre-0.3.6/src/elements/div.rs:1529`，**必须 `use ... as _`**） |
| 0.6.6 **没有** | z-index（`.z_10()` 之类） | `gpui-pre-0.3.6/src/styled.rs` 内无 `z_*`；叠放由**绘制顺序**决定 |
| **存在，但在 trait 上**，必须 `use ... as _` | `Button::{ghost, selected, disabled, xsmall}` | `ButtonVariants` / `Selectable` / `Disableable` / `Sizable`；`button/button.rs:44-96,539`、`sizing.rs:178-202` |
| **存在，但在 trait 上** | `Icon::small()` / `size_3()` / `size_4()` | `Sizable`；组件自己就在用（`list/list_item.rs:227`） |
| 自查方式 | `rg 'fn <名字>' <crate>/src`；或直接 `cargo check`（报错会说"trait 未在作用域内"） | — |

### 1.2 主题 token 映射（macOS → gpui-kit）

macOS 的 token 系统是"2 个家族 × 明暗惰性解析"（`LitheTheme.swift:44-82,184-223,368-374`），gpui-kit 是"主题配置 × `ThemeMode`"。落地时先注册 3 组 `ThemeConfig` + `ThemeMode::Dark` 起步，**不要**按 token 同名直搬，有两个语义陷阱：

| macOS token | 语义 | gpui-kit 对应 | 陷阱 |
| --- | --- | --- | --- |
| `accent` | 强调色 `#3574F0`（= primary） | `theme.primary` | gpui 的 `accent` 是**悬停底色**，直搬会把强调色变成灰底 |
| `selection` | 选中底，**不透明** | `theme.selection`（需覆盖为不透明） | gpui 默认 30% 透明 |
| `subtleSelection` | 悬停/非活动选中底 | `theme.accent` / `list_hover` | 与 `selection` 是两回事 |
| `window` / `editor` | 窗口底 / 编辑器底（浅色不同） | `theme.background` / 自建 `LitheColors.editor` | gpui 默认 `background == popover == white` |
| `toolHeader`、`tertiaryText`、`diff*`、`badge`、`contextMenu`、`toolWindow*` 等 19 项 | — | **自建 `LitheColors`**（不能加在上游 `ThemeColor` 上） | 取值走 `gpui::rgb(0x…)→Hsla`，不要把 sRGB 浮点当 HSL |
| 动效时长（0.10–0.22s + 1 spring） | — | `Theme::global_mut(cx).motion` | macOS 侧没有 token，是就地写的魔法值 |
| `Metrics`（15 个刻度，`LitheTheme.swift:404-423`） | 尺寸 | **Rust `const`** | 不要做运行时 token |

macOS 侧**不存在**的 token 类别（照搬会落空，直接用 gpui-kit 的）：间距阶梯、字号×字重刻度（macOS 只有 ui 14 / small 12 两点，gpui 有 6 档 `TypographyTokens`）、非编辑器行高、阴影三档、`info` 色、语义色前景配对、禁用态（macOS 两个值 0.45/0.55 并存）、密度档位、滚动条颜色、z-index。

### 1.3 gpui-kit 实现规则（都是本项目实测/源码核对的结论）

**布局与行高**

- `base::{TreeState, TreeItem, TreeEntry}` 的表体是 `uniform_list`，**行高取第 0 行内容的测量高度**（`gpui-pre-0.3.6/src/elements/uniform_list.rs:359-371,508-509`）；`.h()` 压不住内容，行内纵向 padding 会让内容溢出到相邻行。要固定行高就把行内容做进该高度里（`ListItem` 默认 `py_1`/`px_3`，紧凑行要 `py_0()`）。
- **`ListItem` 的 children 是竖排**：它内部装 children 的是一个普通块级 `div`（`list/list_item.rs:215-221`），多个 child 会各占一行（箭头、图标、文字各一行）。必须自己套一层 `h_flex()`。
- `DataTable` 行高是**表级**值（`Size::Medium ⇒ 32`，`sizing.rs:57-65`），但表体行高同样受首行内容影响；想让行高正好 22pt（macOS 提交行），必须让单元格自然高度也是 22。
- `TableDelegate::render_td` 的返回值会被组件再包一层（固定列宽 + `overflow_hidden` + `whitespace_nowrap`，`table/state.rs:1323-1329`），单元格不用自己加 `whitespace_nowrap`。
- `Column` 只有 `width/min_width/max_width`，**没有 flex**；列宽按给定值比例铺满表宽，所以要按 macOS 的"剩余宽度给提交列"折算成具体像素。
- `Panel::title_bar()` / `Panel::inner_padding()` 返回 `false`，Dock 就不画自己的标题栏/内边距，面板可以自绘头部（`dock/panel.rs:131-148`、`dock/tab_panel.rs:703-715`）。
- `DockArea` 的 **Bottom dock 只横跨中心列**（`dock_area.rs:1415-1431`）。macOS 的底部工具窗横跨"左栏 + 编辑区"，所以本项目把底部放在 center 的 `v_split` 里（而不是用 `DockPlacement::Bottom`）。
- ⚠️ **上一行那条老结论是错的，已作废**：`viewport_size()` **不是**物理像素，它和 `px()` 是同一套 DPI 无关单位，
  **不要**再 `÷ scale_factor()`（那会让内容只画到窗口的 78%）。正确写法见下面「根视图尺寸」一节。

**组件高度与外观**

- `TitleBar` 默认 34px、`StatusBar` 高度自定；两个都实现 `Styled`，macOS 的 40 / 24 用 `.h(px(40.))` / `.h(px(24.))` 覆盖。

**窗口口径：与同目录的参考应用 Dodona 一致（可见区 94%、居中、普通窗口）**

`D:\developmentProjects\rust\Dodona` 是同一台机器上跑通的 GPUI Kit 应用（维护者确认界面正常），**窗口与布局口径以它为准**：

- 窗口：`visible_bounds() * 0.94`，夹在最小尺寸 `1024×680` 与可见区之间，居中，`WindowBounds::Windowed`
  （`Dodona/crates/dodona/src/main.rs:91-108`）。实测两个进程的窗口矩形**完全相同**（都是 1458×819 @ (39,22)）。
- **不要**照抄 macOS 的 1440×900：那是 macOS 的窗口习惯（交通灯、不最大化），搬到 Windows 会得到比屏幕小的窗口、四周露出桌面。
- **不要**用 `WindowBounds::Maximized` 或 `zoom_window()` 去最大化：Dodona 的口径是"94% 的普通窗口"；而且 `Maximized(bounds)` 在 0.6.6 的 Windows 实现里只把 bounds 当还原尺寸、窗口不会真的最大化（实测 `IsZoomed()` 仍为 false）。

**根视图尺寸：用 `.size_full()`；`viewport_size()` 不要除以 `scale_factor()`**

| 写法 | 结果 |
| --- | --- |
| `.size_full()`（= 父容器 100%，**与 Dodona 一致**，`Dodona/crates/dodona/src/app/mod.rs:153-157`） | ✅ 内容铺满窗口 |
| `.w(viewport_size().width / scale_factor()).h(...)`（旧写法） | ❌ 内容只画到窗口的 **78%**，右侧与底部大片空白 |

**为什么旧写法是错的（这是本项目的实际教训）**：`px()` 用的就是那套 DPI 无关单位，`viewport_size()` 已经是它（本机 1440×864），**再除一次 `scale_factor()` 就小了 1.25 倍**。曾经的错误结论"`size_full()` 会大 1.25 倍"来自一个**坏掉的截图工具**：DPI 不感知的进程调 `PrintWindow` 只会拿到虚拟化尺寸（1458×819），恰好把右侧和底部的空白裁掉，于是内容看起来"铺满了"，而 78% 的真相被掩盖。修好截图工具（先 `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)`，抓到真实 1823×1024）后一眼就能看到空白。

**截图工具要求（`lithe-gpui/capture-screenshot.ps1`）**：必须先声明 per-monitor-v2 DPI 感知，否则在 125% 缩放的机器上 `GetWindowRect`/`PrintWindow` 都返回虚拟化尺寸、**证据会缺一大截**。脚本现在也支持 `-WholeScreen`（抓整个虚拟屏幕，用来核对窗口占屏幕的比例与位置），默认抓窗口。

**单位对账**：真屏 1920×1080 物理 + 125% 缩放；窗口 1823×1024 物理；`viewport_size()` 报 1444×812；`scale_factor()` 报 1.25。**gpui 的布局单位是 `viewport_size()` 那一套（DPI 无关），直接用它或 `.size_full()` 即可，不要再乘/除缩放比。**


- **多行输入 / 代码编辑器必须显式给高度**：多行走 `.h_auto()`（高度按内容算），而内容高度来自 `LayoutMode::CodeEditor { rows }` 的 **`rows` 默认只有 2**（`gpui-component-0.6.6/src/input/input.rs:706-709`、`gpui-base-0.6.6/src/input/base/mode.rs:83-85`）。不写的话编辑区只有两行高、下面整片空白（本项目实测：`look-now.png` 只有一行 → 加高度后 `editor-filled.png` 正常）。正确写法 `Editor::new(&state).h(relative(1.))`，等价于组件自带的 `Input::full_height()`（`input.rs:250-252`）。
- **滚动条要显式改成常显**：gpui-kit 默认 `ScrollbarMode::Scrolling`（滚动时出现、停下淡出，`gpui-base-0.6.6/src/scrollbar.rs:48-56`），IDE 观感是常显细条 + 可拖。用 `Theme::set_scrollbar_mode(ScrollbarMode::Always, cx)`（`gpui-component-0.6.6/src/theme/mod.rs:250-251`），否则"内容多了看不出来能滚、也抓不到滑块"。
- ⚠️ **代码编辑器不吃滚轮，宿主必须自己接**：编辑器元素（`gpui-base-0.6.6/src/input/base/element.rs:202-213`）的样式只有 `position: absolute` + `100%`，**没有 `overflow: scroll`**；而 gpui 只把滚轮交给"命中元素样式里带 `Overflow::Scroll`"的那个（`gpui-pre-0.3.6/src/elements/div.rs:3332-3370`）。做法：在包裹层 `on_scroll_wheel`，把 `event.delta.pixel_delta(line_height)` 交给 `InputBaseState::set_scroll_offset`（`input/base/state.rs:2806-2817`，它自己 clamp、下一帧生效）。本项目已接（`panels.rs::render_editor`），证据 `.artifacts/p1/scrollbars.png`。
- 顺带一条：Dock 的每个面板外面那层 `tab-content` 是 `overflow_y_scroll`（`gpui-component-0.6.6/src/dock/tab_panel.rs:728-734`），所以面板内容比面板高时，滚的是**整块面板内容**（不是面板里某个列表）——列表要自己滚就得自己接滚轮/自己挂滚动条。
- 深色主题要显式 `Theme::change(ThemeMode::Dark, None, cx)`（`gpui_kit::init` 默认浅色），macOS 真机默认深色。
- `Tab::icon()` 会**丢掉 label**，图标+文字要走 `Tab::child(...)`；`TabBar::max_width()` 是标签文字能截断的前提。

**浮层（对话框 / 抽屉 / 通知）：必须先挂层，否则静默不显示**

这是本项目**踩过的坑**（挂层之前 `shell-probe` 弹不出任何对话框，而且不报错）：

- `Root::render` 只内联了 tooltip、原生菜单兜底、触摸选择三层，**Dialog / Sheet / Notification 三层要业务视图自己挂**（`gpui-component-0.6.6/src/root.rs:185-295` 是静态方法，`:586-602` 的 render 里没有它们）。全仓 grep `render_dialog_layer|render_sheet_layer|render_notification_layer` 无匹配，源码注释直接写着：没挂层的对话框"looks exactly like one that does not open"（`dialog/dialog.rs:287-289`）。
- 正确形态（官方测试就是这么写的）：在窗口最外层视图的 render 里补
  `.children(Root::render_dialog_layer(window, cx))`、`.children(Root::render_sheet_layer(window, cx))`、`.children(Root::render_notification_layer(window, cx))`（`gpui-kit-0.6.6/tests/overlays.rs:20-22,84-86`）。本项目已在 `workspace.rs` 的 `render` 里挂好（三个 `let` 必须写在其它持有 `cx` 共享借用的 `render_*` 之前）。
- ⚠️ **`open_dialog` 不能在窗口首帧之前调用，会 panic**：`window.open_dialog` 内部先走 `has_active_dialog` → `Root::read`，它 `expect` 窗口根视图是 `Root`（`root.rs:176-182`、`window_ext.rs:162`）。而子视图的**首次 render 发生在 `Root::new` 内部**，那时窗口根还不是 `Root` —— 在 `new` 里或第 1 帧调用会 panic："The window root view should be of type `ui::Root`"。所以浮层只能由**用户输入**触发（本项目：双击 ⇧ 或项目标签条右端的搜索按钮），不要在构造期预开。
- 命令面板（macOS 没有这个功能，属"用 gpui-kit 优化"的补项）走 **`window.open_dialog` + `Command.bordered(false)`**：Dialog 水平居中（`dialog/dialog.rs:534`）、垂直默认 `viewport/10`（`:529`），Esc 关闭、点外关闭、焦点自动归还全部自带；`Command` 本身只是普通流式 `v_flex`（`command/state.rs:819-837`），**不能直接当浮层**，必须放进 Dialog 这类容器；确认项用 `on_confirm(|IndexPath{section,row,column}|, window, cx)`（`command.rs:157`）。
- 键盘触发要挂在**有焦点**的根元素上：gpui 的键事件沿"焦点节点的祖先链"派发，焦点为空时只派发到窗口根节点（`gpui-pre-0.3.6/src/window.rs:6244-6252`），所以根 `div()` 要 `.track_focus(&handle)` 且开局 `window.focus(&handle, cx)`。另：Windows 平台对 ⇧ 这类修饰键**只发 `ModifiersChanged`、不构造 KeyDown/KeyUp**（`gpui-pre-windows-0.3.6/src/events.rs:1504-1518`），所以"双击 Shift"要用 `on_modifiers_changed` + 时间戳（阈值 350ms，对齐 macOS `MacShortcutDetector.swift:256-317`），不能用 `on_key_down`。
- 其余路径的结论：`open_sheet` 是右侧 350px 抽屉（位置不对）；`Popover` 必须有 trigger 且锚在 trigger 上、不能居中；`PopupMenu/ContextMenu` 只挂右键菜单、项里放不了输入框；`Notification` 是角落 Toast 且无 Esc 绑定；自绘 `absolute` + `deferred()` 要自己写遮罩/焦点陷阱/Esc/点外关/焦点归还，等于重造 Dialog。
- 细节与全部证据：`lithe-gpui/research/gpui-kit-overlay-howto.md`；实机证据：`.artifacts/p1/palette-autotest.png`（面板弹出、分组、首项高亮、背景遮罩都在）。

**能力缺口（需要自研或引第三方 crate）**

- **终端**：gpui-kit 没有终端模拟器 / PTY / ANSI 解析（`gpui-component` 搜 `terminal|alacritty|portable_pty|conpty` 零命中）。路线是 PTY + shell 发现放 Rust 平台层（跨端契约里明确不归 Core），UI 照搬 macOS 的 `TerminalTransport`/`TerminalSession` 字段契约，VT 渲染自写 Element（`portable-pty` + `vte`/`alacritty_terminal`）。⚠️ `gpui-pre` 能否嵌入原生窗口/视图未核实，这决定"嵌现有终端"还是"重写渲染"。
- **编辑器不需要 WebView**：`component::input::EditorState`（`= gpui-base/src/input/editor/mod.rs:11`）自带行号列、折叠、缩进线、tree-sitter 高亮、诊断波浪线、装饰、多光标、撤销栈、内置查找替换（含 `2/3` 计数），`InputEditorStyle::{editor_active_line, editor_gutter_background}` 可配；上限约 5 万行。缺 minimap、overview ruler、sticky scroll、gutter 装饰渲染器（git 色条/断点）、code lens、LSP 客户端、Java grammar。
- **多选**：`TableState`/`ListState` 只有单选；macOS 的 Cmd/Shift 区间选择要自己在 delegate 里维护集合。
- **提交图泳道**与**并排 diff**：gpui-kit 完全没有，需自研（几何算法可从 macOS 移植）。

**gpui-kit 明显优于 macOS 手搓实现、建议直接替换的四处**

| 用途 | 用 gpui-kit 的 | 替代 macOS 的 |
| --- | --- | --- |
| 命令面板 / 快速搜索 | `component::command::Command`（searchable/on_query/on_confirm/empty/max_h） | macOS **没有**命令面板（只有 Search Everywhere 的一个模式） |
| 设置窗口 | `component::setting::{Settings, SettingPage, SettingGroup}` | macOS 手写 switch（4 个 case 落 `EmptyView`；`Cancel` 不撤销、`OK`/`Cancel` 行为相同） |
| 通知 | `component::Notification`（info/success/warning/error + autohide/placement/on_click） | macOS 手搓 HUD 与通知中心两套样式；通知中心一打开就全部标已读 |
| 可调分栏 | `component::resizable::{h_resizable, v_resizable}` + `size_range` | macOS 把 min/max 硬编码在 body 局部变量；gpui 版还白送双击重置 |
| 贴底跟随的输出区 | `message_scroller::{MessageScroller, MessageScrollerState}`（`is_following_tail` / `jump_button` / `with_bottom_fade`） | macOS 手写 `bottomThreshold = 80` |
| 快捷键提示 | `component::kbd::Kbd::binding_for_action` | macOS 手写字符串 |
| 徽标 | `component::badge::Badge::{dot, count, max}` | macOS 自绘小圆点 |

---

## 2. 逐区域对应（规格来源：Windows 前端）

> 界面规格已改为 **Windows 前端**（`windows/tauri/src/features/*`，Tauri v2 + React + Tailwind + shadcn）。
> 逐区域对应表在 **`UI-MAP-WINDOWS.md`**，由 `lithe-gpui/research/windows/` 六份调研汇总：
> `01-shell.md`（外壳/标题栏/活动栏/状态栏）、`02-editor-sidebar.md`（编辑区/标签栏/侧栏）、
> `03-git-and-bottom.md`（Git 工具窗/底部窗）、`04-theme-and-components.md`（token + 组件映射）、
> `05-terminal-run-debug.md`（终端/运行/调试）、`06-database-ai-search.md`（数据库/AI/搜索）。
>
> macOS 那份逐区域调研已移到 **`docs/archive/ui-map-macos.md`**，**只作行为/功能对照**，不要当尺寸来源。

---
## 3. 验收清单（复刻完成时逐条过）

1. 每个区域的**组成与层级**与 macOS 一致（区域清单逐项对照各节"组成"）。
2. **度量**用 macOS 的数值（`px()` 直搬），密度档位与行高符合各节"度量"。
3. 界面上出现的每个元素，都能在本文档里查到"macOS 出处 → gpui-kit 实现"。
4. 状态齐全：空 / 加载 / 失败 / 禁用 / 选中 / 悬停，逐一可见（macOS 缺的按"优化"补上）。
5. 文案与 macOS 本地化原文逐字一致（中文）。
6. 没有裸色值、没有平台业务分支、没有默认图标集之外的字形。
7. 证据：截图 + 日志落到 `.artifacts/p1/`，并在 `PLAN.md` 的当前状态里更新。
