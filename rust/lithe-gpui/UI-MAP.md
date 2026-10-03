# macOS 界面 → gpui-kit 复刻对应表（UI-MAP）

> ## 🔁 规格来源（2026-10-02 定版：**macOS 前端**）
>
> **界面规格来源 = macOS 前端**（`macos/Sources/Lithe/Views/`，SwiftUI/AppKit，158 个 Swift 文件、约 6.0 万行）：
> 尺寸、布局、层级、观感一律照 macOS。**实现框架不变，仍是 gpui-kit。**
>
> 这份结论曾经来回过一次，记在这里以免再翻：
> - 2026-09-24：规格来源 = macOS（取代"照搬 Windows"）；
> - 2026-09-25：改为 Windows 前端，macOS 降级为行为对照；
> - **2026-10-02：改回 macOS** —— 因为 `windows/tauri/` 那份 React/Tauri 前端**已从仓库删除**，
>   规格指向不存在的源码就一个尺寸都核对不了。当时计划的 `UI-MAP-WINDOWS.md` 与
>   `research/windows/01..06-*.md` 从未落地，不必再找。
>
> 决策记录：`.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md`。

这份文档是 GPUI 外壳的**界面规格**：左边是 macOS 端（`macos/Sources/Lithe`，SwiftUI/AppKit）的**真实界面**，右边是用 **gpui-kit** 复刻时"用哪个组件、怎么用、差在哪"。

工作方式：

- **界面规格取自 macOS 前端源码**（`macos/Sources/Lithe/Views/`）；
- **实现只能用 gpui-kit 源码里真实存在的组件与基元**（判断依据是已发布版本的源码 + 提交的 `Cargo.lock`，不是在线文档）；
- 做得不好的地方**允许**用 gpui-kit 做得更好，但必须在"优化"一栏写清理由。

同目录的现实配套文件只有这些：

| 文件 | 作用 |
| --- | --- |
| 本文 `UI-MAP.md` | **§1 硬规则 + gpui-kit 实现规则** + **§2 逐区域对应（规格 = macOS）** + §3 验收清单 |
| `README.md` | 怎么跑起来、已验证的接口事实、硬约束 |
| `BLOCKERS.md` | 当前卡住的事 |
| `HANDOFF.md` | 交接现状 |
| `crates/` | gpui 宿主的实际实现（12 个 crate） |
| `tools/` | `check-ui-px.mjs`（硬编码 px 守卫）、`generate-idea-icons.mjs`（图标生成） |
| `.agents/notes/proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md` | 为什么这样做的决策记录 |

> 清理说明（2026-09-25 原始记录，2026-10-02 更新）：早期调试期的截图证据（`.artifacts/p1/*`）与 macOS 原始区域调研
> （`.artifacts/ui-map/01..08`）已删除 —— 前者里有**坏截图工具产出的误导性画面**（例如 `startup-max.png`
> 把"内容只占 78%"拍成"已铺满"）。本文的结论都以**文字 + 源码行号**写清，需要复现时按文中命令重跑即可。
> **注意**：文中提到的 `lithe-gpui/capture-screenshot.ps1`、`lithe-gpui/PLAN.md`、`lithe-gpui/research/`、
> `docs/archive/ui-map-macos.md` 这些路径**在当前工作区都不存在**；下面的正文已把这类引用改写为可执行的替代做法。
> `lithe-gpui/docs/**`（上游文档镜像）同样不存在。

---

## 0. 这份文档怎么用

| 你要做的事 | 读哪一节 |
| --- | --- |
| **动手写界面之前**——先读上游规范 | **`docs/gpui kit/0.7.0/docs/coding-guides.md`**（607 行）与 **`design-guides.md`**（515 行）。两者都在本仓库；编码指南开头就要求"请先阅读设计指南"。§1.4 是它们的摘录 |
| 写 gpui 界面代码前，避开已知的 API / 布局坑 | **§1** 硬规则与 gpui-kit 实现规则 |
| 新做一个区域：先看 macOS 长什么样、再决定用哪个组件 | **§2** 逐区域对应表（17 个区域，规格 = macOS） |
| 界面做完自检 | **§3** 验收清单 |
| 核对某个上游 API 到底在不在、0.7 该怎么写 | **§4**（已按 0.7.0 registry 源码逐条核对） |
| 想知道某区域是"缺 Core"还是"缺界面" | `docs/tasks/gpui-core-first.md` |

三条口径先说清，后面所有内容都按它读：

1. **规格真源 = macOS 前端**：`macos/Sources/Lithe/Views/`（17 个区域、158 个 Swift 文件、约 6.0 万行）。
2. **实现框架 = gpui-kit**，且只能用**已发布源码里真实存在**的组件与基元；判断依据是 registry 源码 + 提交的 `Cargo.lock`，不是在线文档。
3. **本文的 gpui 侧落点写到"文件/模块"级**，不写到行号 —— 行号会被重构冲掉，本文此前就因大量失效行号而不可用。要核对具体实现，用文中给的路径自己打开。

> **与两份官方指南的关系**：本文回答"**照 macOS 复刻时每个区域该长什么样、用哪个组件**"；
> 官方指南回答"**gpui-kit 应用应该怎么写**"。两者冲突时以官方指南为准（它是框架规范，本文只是本项目的对照表）。

---

## 1. 硬规则（每条都核对过源码或踩过坑）

### 1.1 通用

1. **度量一律用 gpui-kit 的 rem-based 尺度助手，不要直接写 `px(...)`。** 这是 gpui-kit 的**明文规范**，两份指南各说了一遍：

   > 「application layout 使用 GPUI rem-based helper（`p_2()`、`gap_3()`、`w_64()`、`text_sm()`），而不是直接 `px(...)`」
   > —— `docs/gpui kit/0.7.0/docs/coding-guides.md`（主题与样式）

   > 「原则上 application layout **不得**直接调用 `px(...)`。使用 GPUI rem-based scale helper 或 semantic component Size。
   > 只有值代表 physical/raster boundary 时才使用 fixed px：one-device-pixel hairline、platform window inset、
   > bitmap dimension、minimum hit-test tolerance，或必须匹配 external surface 的 geometry。**这些必须是经过审查和记录的例外。**」
   > —— `docs/gpui kit/0.7.0/docs/design-guides.md`（缩放、基础字号与 `rem`）

   同处还写明：「Application UI 中每个直接 `px(...)` 和 raw color constructor **都应视为 review finding**。只有 documented
   physical/platform boundary、measured runtime geometry、raster/data color 或 theme/token definition 本身可以例外。
   **方便或"匹配截图"都不是有效理由。**」（coding-guides）

   **换算规则**：主题的 rem 基准是 `Theme.font.size`（`Root` 会调 `window.set_rem_size(cx.theme().font_size)`），默认 **16px**。
   命名尺度用 **`_N` = N × 0.25rem**（`style.md` 原文：`_1` 为 `0.25rem`、`_2` 为 `0.5rem`、`_3` 为 `0.75rem`、`_4` 为 `1rem`），
   所以在 16px 基准下 **`_N` = N × 4px**：`h_9()` = 36px、`px_3()` = 12px、`gap_2()` = 8px、`size_4()` = 16px。
   带 `p` 的是半点：`1p5` = 1.5 × 4 = 6px。

   **`style.md` 给出的完整后缀清单**（比本项目实际用到的多得多，别以为没有）：
   `0` · `0p5` · `1` · `1p5` · `2` · `2p5` · `3` · `3p5` · **`4` 到 `12` 的每个整数** · `16` · `20` · `24` · `32` · `40` · `48` · `56` · `64` · `72` · `80` · `96` · `112` · `128`；
   另有 `_px`、`_full`、`_1_2` 等分数后缀，以及 `_neg_` 形式（如 `mt_neg_2`）。
   接受 `auto` 的方法族才有 `_auto`。**边框宽度**：`border`/`border_t`/`border_b`/`border_l`/`border_r`/`border_x`/`border_y` 有 `0`–`12` 与 `16/20/24/32`。
   **圆角**：`rounded*` 有 `none`/`xs`/`sm`/`md`/`lg`/`xl`/`2xl`/`3xl`/`full`。
   （按 `style.md` 的提醒："命名尺度和方法范围以 GPUI 的实现为准，不要假定每个 Tailwind class 都有对应方法"。）

   本项目实际在用的助手（0.7 下核对过、可直接照用）：

   | 类别 | 可用名字 |
   | --- | --- |
   | 高 / 宽 | `h_0p5 h_1 h_5 h_6 h_7 h_8 h_9 h_11`、`w_5 w_6 w_8 w_9 w_20 w_32` |
   | 内边距 | `p_0 p_0p5 p_1 p_1p5 p_2 p_3 p_6`、`px_0 px_1 px_1p5 px_2 px_3 px_4 px_6`、`py_0 py_1 py_1p5 py_2 py_2p5 py_3 py_6 py_8` |
   | 外边距 | `mr_0p5 mt_1 mt_2 mb_1 mb_2 mb_5` |
   | 间隔 | `gap_0 gap_0p5 gap_1 gap_1p5 gap_2 gap_2p5 gap_3 gap_4` |
   | 尺寸 | `size_3 size_3p5 size_4 size_6`（`Sizable`，**需 `use ... as _`**） |

   ⚠️ **更正一处早先的说法**：本文档与相关笔记里出现过 `ui_ml` / `ui_mt` / `ui_control_h` 这类"项目自定义单位助手"，
   **仓库里并不存在**（`ui_*` 只是几个测试函数名）。本项目用的就是 gpui 的 `rems(...)` / `px(...)`，外加每个 crate 各一份的 `rem_px`。

   为什么必须是 rem 而不是 px —— 按这个顺序选写法（三种项目里都有先例）：

   1. **档位内 → 用尺度助手**：`h_9()`（36px）、`px_2()`（8px）、`gap_1()`（4px）、`size_4()`（16px）。首选，可读性最好。
   2. **档位外的值 → 用带类型的 setter**。`style.md` 给的规范写法是 `.gap(rems(0.625))`，另外两种等价形式是 `.w(px(240.))` 与 `.w(relative(0.5))`：
      - **`rems(P / 16.)`**：`P` 是规格像素值，基准 16px 时与 `px(P)` **逐像素相等**，但会随主题字号缩放。项目里已这么写，例如 `div().w(rems(left_sidebar_width / 16.))`，并在注释里写明了等价关系（`crates/workbench/src/workspace.rs` 的度量段落：`rems(400. / 16.)` ≡ `px(400.)`）。
      - `relative(0.5)`：可用相对尺寸的一半（**不是 0.5 像素**，`geometry.md` 专门点了这个常见错误）。
      - `px(...)`：**只在指南允许的例外里用**（见下表）。
   3. **只有必须交出 `Pixels` 的槽 → 用 `rem_px(spec_px)`**：例如 `event.delta.pixel_delta(line_height)` 这类事件增量，它吃 `Pixels`、交不了 `AbsoluteLength`。**不要自己写 `/ 4.`**（那是错的：助手后缀 `N` = `N × 0.25rem`，而规格值 `P` 是像素，只有 `/ 16.` 才对）。

   指南允许 fixed `px` 的**全部**例外（`design-guides` 列举，且要求"经过审查和记录"）：
   one-device-pixel hairline · platform window inset · bitmap dimension · minimum hit-test tolerance · 必须匹配 external surface 的 geometry ·
   measured runtime geometry · raster/data color · theme/token definition 本身。
   **"方便"或"匹配截图"都不是有效理由。**

   **为什么这条值得较真**（guides 的论证，不只是"规范如此"）：`Root::render` 每帧调 `window.set_rem_size(cx.theme().font_size)`，
   所以 **theme base font 同时是应用的设计尺度基准**——type、spacing、control、icon 共享同一条 zoom 轴。
   写死 px 会让"改界面字号"只放大文字、不放大间距与控件，于是 hierarchy 崩掉。设计指南还要求：
   缓存过 resolved layout 的东西（换行行高、文本 shaping、virtual-list 测量、popup 几何、由文字推导的图标尺寸）
   **必须把 `window.rem_size()` 纳入失效键**，否则 zoom 后测到的还是旧尺寸。

   另外：`window.scale_factor()` 只与物理像素有关，**不参与**这套换算。

   ✅ **现状（2026-10-02 复测，本条已达标）**：`D16` / `D17` 已完成 —— `rem_px` 换算助手已收敛成 `lithe-gpui-shared` 的**唯一**实现（`crates/shared/src/lib.rs`，其余 crate 用 `pub(crate) use` 转发），生产代码里的直接 `px()` 由守卫实测 **exit 0 / 违规 0 处**（11 处命中全部落在 7 条平台 / token 例外白名单里）。
   ⚠️ 顺带更正本文早先的口径：**"生产代码约 57 处 `px()`"是错的** —— 那是把注释 / 字符串里的 `px(` 与 `#[cfg(test)]` 断言一起数进去的结果；真正的 UI 代码调用只有 13 处。经过与更正见 `docs/tasks/gpui-core-first.md` §4.3 / §7。
2. **颜色与字号一律走主题 token**（`cx.theme()` / `cx.theme().tokens`），不写裸色值；映射见 §1.2。
3. **文案取 macOS 的本地化原文**（`macos/Resources/zh-Hans.lproj`），不要自己编中文。
4. **图标**：应用注册的是**全量** `gpui_kit::assets::AllAssets`（0.7 仍在：`gpui-kit-assets-0.7.0/src/lib.rs`、`native_assets.rs`），嵌入 `assets/icons/` 下的 **1830 个 Lucide 字形**—— 所以**没有"缺字形"这回事**，也不需要近似替代。
   仍然要记住的两点：
   ① 有**两个** `IconName`。`gpui_kit::component::IconName` 是**为源码兼容保留的组件子集**（0.7 的源码注释写得很直白："Default component icon names, retained for source compatibility. For the complete Lucide catalog, use `gpui_kit_assets::IconName`"），完整目录归 `gpui_kit_assets::IconName`（1830 个）。二者之间有**显式 `From` 转换**（`impl From<component::IconName> for gpui_kit_assets::IconName`），所以换类型即可、不用改写法。
   ② `Icon::new` / `Button::icon` 收 `impl Into<Icon>`，且 `impl<T: IconNamed> From<T> for Icon` 对两者都成立。变体名 = svg 文件名的 PascalCase（`trash-2.svg` 不存在，所以没有 `Trash2`，用 `Trash`）。
   ⚠️ 组件子集清单来自 `gpui-kit-assets-0.7.0/default-icons.txt`（0.7 是 **104 行**，0.6.6 时期是 101，见 §4.1）。
5. **不写平台业务分支**：不给 macOS/Windows 写 `#[cfg]` 业务代码。平台差异（macOS 红绿灯留白 76pt、Windows 最小化/最大化/关闭按钮位置）由 gpui 平台层与 `TitleBar` 自身处理，界面结构三端统一。
6. **一个区域一个模块**：外壳按区域拆分，便于并行与替换。<br>⚠️ 本文早先举的 `lithe-gpui/shell/src/bin/shell_probe/` **已不存在**（旧 P1 冒烟宿主的 bin 已删除，职责被 `crates/app` 覆盖）。现在的实际结构是 `crates/{app,workbench,editor,explorer,extensions,git,java,notify,run,settings,shared,terminal}` —— 见 §2.1 总表的"gpui 落点"一列。
7. **缺 API 先读源码**：读本机 cargo registry 里已解析的 `gpui-kit-*` / `gpui-component-*` / `gpui-base-*` 源码目录（版本以 `rust/Cargo.lock` 为准），不要凭在线文档猜。

### 1.1.1 版本陷阱（⚠️ 本节写于 0.6.6 时期，宿主现已升到 0.7）

**先读这条再往下用**：本节的源码路径与行号来自 **gpui-kit 0.6.6**，而本分支各 crate 现在声明的是
`gpui-kit = "0.7"`（`rust/lithe-gpui/crates/*/Cargo.toml`），`rust/Cargo.lock` 也已锁定 0.7。
所以下面的**结论大多仍然成立**（它们描述的是"某个 API 在不在"），但**行号与文件名必须按 0.7 重新核对**。

→ **完整表格已移到 §4**，避免这里的失效行号被当成当前依据。本节只留最有杀伤力的一条：

**0.7 的 `Root` 模型变了**：`Root` 从 `gpui-component` 搬到了 **`gpui-base`**，现在是"窗口内容 + 浮层宿主"；
对话框 / 抽屉 / 通知由 `WindowState`（一个 `RootPlugin`）**随窗口自动挂载**，
0.6.6 的 `Root::render_dialog_layer / render_sheet_layer / render_notification_layer` 三个手动挂层 API **已删除**。
所以本文 §1.3 里"浮层必须先挂层"那一大段属于 0.6.6 的做法，**在 0.7 上不要照做** ——
完整的三处结构性变化（含 `gpui_kit::open_window` 在 0.7 已存在）见 **§4.1**，要改写法看 **§4.3 的 B 组**。

### 1.2 主题 token 映射（macOS → gpui-kit）

> ⚠️ **本条已核实（2026-10-02）**：`rust/lithe-gpui/crates/settings/src/theme.rs` 与 `themes/*.json` 里
> **没有**自建的颜色表（全仓 grep `LitheColors` 无实现、生产代码无 raw color），全部走 `cx.theme()`。
> 所以上表里那些"macOS 有、gpui-kit 没有"的 token 类别，**当前是"没有对应"而不是"自建了对应"** ——
> 要做就是**往产品 theme / semantic tokens 层加 role**（§1.4 A 组），不要在调用点拼颜色。

macOS 的 token 系统是"2 个家族 × 明暗惰性解析"（`macos/Sources/Lithe/Theme/LitheTheme.swift`），gpui-kit 是"主题配置 × `ThemeMode`"。落地时先注册 3 组 `ThemeConfig` + `ThemeMode::Dark` 起步，**不要**按 token 同名直搬，有两个语义陷阱：

| macOS token | 语义 | gpui-kit 对应 | 陷阱 |
| --- | --- | --- | --- |
| `accent` | 强调色 `#3574F0`（= primary） | `theme.primary` | gpui 的 `accent` 是**悬停底色**，直搬会把强调色变成灰底 |
| `selection` | 选中底，**不透明** | `theme.selection`（需覆盖为不透明） | gpui 默认 30% 透明 |
| `subtleSelection` | 悬停/非活动选中底 | `theme.accent` / `list_hover` | 与 `selection` 是两回事 |
| `window` / `editor` | 窗口底 / 编辑器底（浅色不同） | `theme.background` / 另立一个语义 role（**不要**自建 palette，见 §1.4 A 组） | gpui 默认 `background == popover == white` |
| `toolHeader`、`tertiaryText`、`diff*`、`badge`、`contextMenu`、`toolWindow*` 等 19 项 | — | 在**产品 theme / `Theme::semantic_tokens()`** 层补语义 role（同上） | 取值走 `gpui::rgb(0x…) → Hsla`，不要把 sRGB 浮点当 HSL |
| 动效时长（0.10–0.22s + 1 spring） | — | `Theme::global_mut(cx).motion` | macOS 侧没有 token，是就地写的魔法值 |
| `Metrics`（15 个刻度，`LitheTheme.swift:404-423`） | 尺寸 | **Rust `const`** | 不要做运行时 token |

macOS 侧**不存在**的 token 类别（照搬会落空，直接用 gpui-kit 的）：间距阶梯、字号×字重刻度（macOS 只有 ui 14 / small 12 两点，gpui 有 6 档 `TypographyTokens`）、非编辑器行高、阴影三档、`info` 色、语义色前景配对、禁用态（macOS 两个值 0.45/0.55 并存）、密度档位、滚动条颜色、z-index。

### 1.3 gpui-kit 实现规则（都是本项目实测/源码核对的结论）

**布局与行高**

- `base::{TreeState, TreeItem, TreeEntry}` 的表体是 `uniform_list`，**行高取第 0 行内容的测量高度**；`.h()` 压不住内容，行内纵向 padding 会让内容溢出到相邻行。要固定行高就把行内容做进该高度里（`ListItem` 默认 `py_1`/`px_3`，紧凑行要 `py_0()`）。<br>⚠️ 本条 0.7 **未逐行复核**（源码仍在 `gpui-pre-0.3.7/src/elements/uniform_list.rs`，但没定位到具体测量函数）。
- **`ListItem` 的 children 是竖排**：它内部装 children 的是一个普通块级 `div`（`list/list_item.rs:215-221`），多个 child 会各占一行（箭头、图标、文字各一行）。必须自己套一层 `h_flex()`。
- `DataTable` 行高是**表级**值（`Size::Medium ⇒ 32`，`sizing.rs:57-65`），但表体行高同样受首行内容影响；想让行高正好 22pt（macOS 提交行），必须让单元格自然高度也是 22。
- `TableDelegate::render_td` 的返回值会被组件再包一层（固定列宽 + `overflow_hidden` + `whitespace_nowrap`，`table/state.rs:1323-1329`），单元格不用自己加 `whitespace_nowrap`。
- `Column` 只有 `width/min_width/max_width`，**没有 flex**；列宽按给定值比例铺满表宽，所以要按 macOS 的"剩余宽度给提交列"折算成具体像素。
- `Panel::title_bar()` / `Panel::inner_padding()` 返回 `false`，Dock 就不画自己的标题栏/内边距，面板可以自绘头部（`dock/panel.rs:131-148`、`dock/tab_panel.rs:703-715`）。
- `DockArea` 的 **Bottom dock 只横跨中心列**（`dock_area.rs:1415-1431`）。macOS 的底部工具窗横跨"左栏 + 编辑区"，所以本项目把底部放在 center 的 `v_split` 里（而不是用 `DockPlacement::Bottom`）。
- ⚠️ **上一行那条老结论是错的，已作废**：`viewport_size()` **不是**物理像素，它和 `px()` 是同一套 DPI 无关单位，
  **不要**再 `÷ scale_factor()`（那会让内容只画到窗口的 78%）。正确写法见下面「根视图尺寸」一节。

**组件高度与外观**

- `TitleBar` 默认 34px、`StatusBar` 高度自定；两个都实现 `Styled`，macOS 的 40 / 24 用 `.h(rems(40. / 16.))` / `.h(rems(24. / 16.))` 覆盖（**不要写 `px(40.)`**，见 §1.1 第 1 条）。

**窗口口径：与同目录的参考应用 Dodona 一致（可见区 94%、居中、普通窗口）**

`D:\developmentProjects\rust\Dodona` 是同一台机器上跑通的 GPUI Kit 应用（维护者确认界面正常），**窗口与布局口径以它为准**：

- 窗口：`visible_bounds() * 0.94`，夹在最小尺寸 `1024×680` 与可见区之间，居中，`WindowBounds::Windowed`
  （`Dodona/crates/dodona/src/main.rs:91-108`）。实测两个进程的窗口矩形**完全相同**（都是 1458×819 @ (39,22)）。
- **不要**照抄 macOS 的 1440×900：那是 macOS 的窗口习惯（交通灯、不最大化），搬到 Windows 会得到比屏幕小的窗口、四周露出桌面。
- **不要**用 `WindowBounds::Maximized` 或 `zoom_window()` 去最大化：Dodona 的口径是"94% 的普通窗口"；而且在早期版本上实测过 `Maximized(bounds)` 只把 bounds 当还原尺寸、窗口不会真的最大化（`IsZoomed()` 仍为 false，报 `linker_messages` 那个版本）。⚠️ 这条**没有在 0.7 上重测**，要用请先在目标平台验一次。

**根视图尺寸：用 `.size_full()`；`viewport_size()` 不要除以 `scale_factor()`**

| 写法 | 结果 |
| --- | --- |
| `.size_full()`（= 父容器 100%，**与 Dodona 一致**，`Dodona/crates/dodona/src/app/mod.rs:153-157`） | ✅ 内容铺满窗口 |
| `.w(viewport_size().width / scale_factor()).h(...)`（旧写法） | ❌ 内容只画到窗口的 **78%**，右侧与底部大片空白 |

**为什么旧写法是错的（这是本项目的实际教训）**：`px()` 用的就是那套 DPI 无关单位，`viewport_size()` 已经是它（本机 1440×864），**再除一次 `scale_factor()` 就小了 1.25 倍**。曾经的错误结论"`size_full()` 会大 1.25 倍"来自一个**坏掉的截图工具**：DPI 不感知的进程调 `PrintWindow` 只会拿到虚拟化尺寸（1458×819），恰好把右侧和底部的空白裁掉，于是内容看起来"铺满了"，而 78% 的真相被掩盖。修好截图工具（先 `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)`，抓到真实 1823×1024）后一眼就能看到空白。

**截图工具要求**：抓包必须用 **DPI 感知**的进程（先声明 per-monitor-v2 DPI 感知），否则在 125% 缩放的机器上 `GetWindowRect`/`PrintWindow` 都返回虚拟化尺寸、**证据会缺一大截**。需要抓整个虚拟屏幕来核对窗口占屏幕的比例与位置时，同样要在感知模式下取全屏矩形。⚠️ 本文早先引用的 `lithe-gpui/capture-screenshot.ps1` **当前工作区里不存在**，需要复现时自己写一个（或从 git 历史的对应提交里取回）。

**单位对账**：真屏 1920×1080 物理 + 125% 缩放；窗口 1823×1024 物理；`viewport_size()` 报 1444×812；`scale_factor()` 报 1.25。**gpui 的布局单位是 `viewport_size()` 那一套（DPI 无关），直接用它或 `.size_full()` 即可，不要再乘/除缩放比。**


- **多行输入 / 代码编辑器必须显式给高度**：多行走 `.h_auto()`（高度按内容算），而内容高度来自 `LayoutMode::CodeEditor { rows }` 的 **`rows` 默认只有 2**。不写的话编辑区只有两行高、下面整片空白。正确写法 `Editor::new(&state).h(relative(1.))`，等价于组件自带的 `Input::full_height()`。
- **滚动条要显式改成常显**：gpui-kit 默认是"滚动时出现、停下淡出"，IDE 观感要常显细条 + 可拖。用 `Theme::set_scrollbar_mode(ScrollbarMode::Always, cx)`（0.7 仍在：`gpui-component-0.7.0/src/theme/mod.rs`），否则"内容多了看不出来能滚、也抓不到滑块"。
- ⚠️ **代码编辑器不吃滚轮，宿主必须自己接**：编辑器元素的样式只有 `position: absolute` + `100%`，**没有 `overflow: scroll`**；而 gpui 只把滚轮交给"命中元素样式里带 `Overflow::Scroll`"的那个。做法：在包裹层 `on_scroll_wheel`，把 `event.delta.pixel_delta(line_height)` 交给 `InputBaseState::set_scroll_offset`（0.7 仍在：`gpui-base-0.7.0/src/input/base/state.rs`，它自己 clamp、下一帧生效）。本项目已接，落点是 **`crates/editor/src/editor_view.rs` 里 `EditorPane::render_body` 的编辑器包裹层（`on_scroll_wheel` 接线处）**（本文早先引用的 `panels.rs::render_editor` 与 `.artifacts/p1/scrollbars.png` **都已不存在**）。
- 顺带一条：Dock 的每个面板外面那层 `tab-content` 是 `overflow_y_scroll`，所以面板内容比面板高时，滚的是**整块面板内容**（不是面板里某个列表）——列表要自己滚就得自己接滚轮/自己挂滚动条。
- 深色主题要显式 `Theme::change(ThemeMode::Dark, None, cx)`（`gpui_kit::init` 默认浅色），macOS 真机默认深色。
- `Tab::icon()` 会**丢掉 label**，图标+文字要走 `Tab::child(...)`；`TabBar::max_width()` 是标签文字能截断的前提。

**浮层（对话框 / 抽屉 / 通知）—— ⚠️ 本节整段是 0.6.6 的事实与排查过程，0.7 上不适用**

下面记录的是 **gpui-kit 0.6.6** 时期的现象：三层浮层要业务视图自己挂，没挂就**静默不显示**、也不报错。
**0.7 已改机制**：`Root` 搬到 `gpui-base`，对话框 / 抽屉 / 通知由 `WindowState`（一个 `RootPlugin`）
**随窗口自动挂载**，三个手动挂层 API **已删除**。所以：

- **不要照抄下面的挂层写法**（那三行在 0.7 里已经不存在）；
- 要加自己的窗口级浮层，改为实现 `RootPlugin` + `Root::register_plugin`；
- 完整对照见 §4.1（M2/M3）与 §4.3 的 B 组。

保留下面这段是为了留住"为什么会静默失败"这个结论——**它仍能解释一类症状**：浮层没有宿主时不会报错。

- （0.6.6）`Root::render` 只内联了 tooltip、原生菜单兜底、触摸选择三层，Dialog / Sheet / Notification 三层要业务视图自己挂。全仓 grep 那三个 `render_*_layer` 无匹配，源码注释直接写着：没挂层的对话框"looks exactly like one that does not open"（`dialog/dialog.rs`）。
- **0.7 的做法**：`gpui-component-0.7.0/src/root.rs:434` 的 `impl gpui_base::RootPlugin for WindowState`（`prepare:435` / `style:440` / `decorate:450`）内部有 `notification_layer:129` / `sheet_layer:156` / `dialog_layer:181`，由 `WindowStateLayers`（`:479`）在应用内容之后自动挂载；`gpui_kit::init`（`gpui-kit-0.7.0/src/lib.rs:170`）已调 `gpui_component::init` + `gpui_base::init`。
- ⚠️ **仍然要守的一条**：`open_dialog` 不能在窗口首帧之前调用。0.7 的落点是 `WindowState::open_dialog`（`gpui-component-0.7.0/src/root.rs:235`）——它同样要求窗口根已经是 `Root`（`Root::new` 见 `gpui-base-0.7.0/src/root.rs:147`），而子视图的首次 render 发生在 `Root::new` 内部。所以浮层只能由**用户输入或任务**触发，不要在构造期预开（本项目用 `window.on_next_frame` 处理 `--open-settings` 这种机器验证场景）。
- 命令面板（macOS 没有这个功能，属"用 gpui-kit 优化"的补项）走 **`window.open_dialog` + `Command.bordered(false)`**：Dialog 水平居中（`dialog/dialog.rs:534`）、垂直默认 `viewport/10`（`:529`），Esc 关闭、点外关闭、焦点自动归还全部自带；`Command` 本身只是普通流式 `v_flex`（`command/state.rs:819-837`），**不能直接当浮层**，必须放进 Dialog 这类容器；确认项用 `on_confirm(|IndexPath{section,row,column}|, window, cx)`（`command.rs:157`）。
- 键盘触发要挂在**有焦点**的根元素上：gpui 的键事件沿"焦点节点的祖先链"派发，焦点为空时只派发到窗口根节点，所以根 `div()` 要 `.track_focus(&handle)` 且开局 `window.focus(&handle, cx)`。<br>⚠️ 本条 0.7 **未逐行复核**（`gpui-pre-0.3.7/src/window.rs` 里的派发实现本轮未定位到）。<br>⚠️ **下面是 0.6.6 的记载，与本项目现状不符**：那段说"双击 Shift"要用 `on_modifiers_changed`（Windows 只发 `ModifiersChanged`、不构造 KeyDown/KeyUp）。**但本项目全仓没有 `on_modifiers_changed` / `double_tap` / `shift_shift` 实现，也没有单击 Shift 的搜索入口** —— 全局搜索入口不存在（见 §2.2 ⑬）。这段是**别的实现路线**留下的记录，不要当成"我们已经这么做了"。
- 其余路径的结论：`open_sheet` 是右侧 350px 抽屉（位置不对）；`Popover` 必须有 trigger 且锚在 trigger 上、不能居中；`PopupMenu/ContextMenu` 只挂右键菜单、项里放不了输入框；`Notification` 是角落 Toast 且无 Esc 绑定；自绘 `absolute` + `deferred()` 要自己写遮罩/焦点陷阱/Esc/点外关/焦点归还，等于重造 Dialog。
- 细节与全部证据：`.artifacts/p1/palette-autotest.png`（面板弹出、分组、首项高亮、背景遮罩都在）。⚠️ 早先提到的 `lithe-gpui/research/gpui-kit-overlay-howto.md` **当前工作区里不存在**，浮层 API 请直接读 registry 里已解析版本的源码。

**能力缺口（需要自研或引第三方 crate）**

- **终端**：gpui-kit 没有终端模拟器 / PTY / ANSI 解析（`gpui-component` 搜 `terminal|alacritty|portable_pty|conpty` 零命中）。
  ⚠️ **本条曾经写成"路线是 PTY + 自写 VT Element"，那是计划不是实现**：当前实现是 `std::process::Command`
  + 三管道 + **把控制序列丢掉**的最简清洗（无 PTY、无 resize、无 Ctrl+C）。能力边界与跨平台硬伤见 §2.2 ⑨。
  要做真终端，PTY 属**平台能力**（不进 Core），VT 渲染要么自写 Element、要么引第三方 crate。
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

### 1.4 gpui-kit 官方指南里**必须遵守**的条文

来源：`docs/gpui kit/0.7.0/docs/` 下的官方文档镜像（41 份，其中 `coding-guides.md` 607 行、`design-guides.md` 515 行是主指南）。
**改界面前应先读它们**（编码指南开头就要求"请先阅读设计指南"）。
下面只摘与本项目界面复刻直接相关、且容易违反的条文；**带 ⚑ 的是本项目现状**。共 27 组（A–AA）。

**怎么找**（按你正在做的事定位到组）：

| 我在做… | 看 |
| --- | --- |
| 写尺寸、间距、颜色、圆角、字体档位 | **A** 样式与颜色 |
| 排版式（flex / 对齐 / 滚动 / 间距 / 密度） | **B** 布局与对齐 |
| 写启动顺序、`Root` 所有权、订阅与 Task 生命周期 | **C** 初始化与生命周期 · **G** 窗口 · **O** 异步任务 |
| 加第二个窗口、跨窗口共享状态 | **G** 窗口 · **X** 多窗口 |
| 写界面文案、按钮、无障碍 | **D** 文案与无障碍 · **V** 无障碍 |
| 滚动区域、坐标换算、量 `bounds` | **E** 滚动几何与坐标 |
| 装字体、切语言、管图标资源、选 `img` 还是 `svg` | **F** 字体图标与 i18n · **H** 图标与资源 · **Y** 图片 |
| 设计状态放哪、谁 `notify`、要不要缓存 | **I** 状态渲染与通知 · **M** 视图缓存 |
| 接 `window` / `cx`、跨 `await` | **J** Context 与异步 |
| 写可复用组件（`RenderOnce` vs `Render`） | **K** Render 与 RenderOnce |
| 做列表、找 element id、写焦点 | **L** 稳定标识 · **M** 焦点 |
| 加命令、快捷键、菜单、Global 状态 | **N** 命令快捷键与全局状态 |
| 自绘（diff / minimap / 终端） | **P** 自定义 Element · **Q** 绘制原语与层级选择 |
| 接鼠标滚轮/键盘事件、排查"点了没反应" | **R** 事件与订阅 |
| 画文字、摆光标、算宽度（含 UTF-8 字节坑） | **S** 文字系统 |
| 做动效 | **T** 动效 |
| 查帧率/内存读数怎么做、性能怎么量 | **U** 帧率与性能观测 |
| 给交互补自动化测试 | **W** 测试层级与 UI 集成测试 |
| 系统通知、原生扩展、打包、签名、自动更新 | **Z** 平台集成 |
| 查"这套文档整体在哪、每份读到什么程度" | **AA** 文档索引 |

**A. 样式与颜色**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| application layout 用 rem-based helper 或 semantic component Size，**不得**直接 `px(...)`；fixed px 只允许 physical/raster 边界，且必须记录 | design §缩放 | ✅ 守卫 `node lithe-gpui/tools/check-ui-px.mjs` 实测 **exit 0 / 违规 0 处**：11 处命中全部落在白名单里（7 条：4 条 OS 窗口平台边界 + 3 条 `Theme.font_size`/`mono_font_size` token 定义本身） |
| application code **不得**引入 raw hex / `rgb` / `rgba` / `hsla`；缺语义 role 时**在产品 theme/token 层定义**，不在 call site 嵌 palette 值 | coding §主题与样式、design §颜色与主题 | ✅ **没有** raw color 字面量，也**没有**自建颜色表；全部走 `cx.theme()`（实测复核过） |
| 新 presentation 优先用 `Theme::semantic_tokens()`（通用 color/radius/spacing/typography/shadow）；legacy token 只作兼容 | coding §主题与样式 | 未核 —— 若有自建 token 层，宜迁到 semantic tokens |
| `Theme::update(cx, …)` 才会把 `colors` / `tokens` / base 三份重新对齐；直接 `Theme::global_mut(cx)` 只改你碰的那个字段（会出现"文字新、背景旧"） | coding §主题与样式 | 改主题时注意 |
| 圆角用有限主题档位；嵌套表面走 `inner ≈ max(0, outer − inset)`；**不要假定父级圆角会自动裁剪后代** | design §圆角 | 未核 |
| 密度用组件 `Size` API（`xsmall`/`small`/`medium`/`large`），让字体、图标、padding、hit target **一起**变；"只改外框高度"是不完整的 | design §密度层级 | ✅ **⑥ Git 实测收口（D18 按区域做，其余区域未做）**。`crates/git/` 复核 `h_5()`–`h_12()` 手写定高 **25 处**：(a) 组件被手写定高硬撑 **6 处已改** / (b) 纯布局元素 **18 处不改** / (c) 已是档位或组件 Size **0 处** / (d) **盘点式内待裁决 1 处 + 盘点式外 1 处**。**改动**（`Button` 的 `.h_6().px_1p5()` → `.small().compact()`，锚点一律按**按钮 ElementId + 文件名**记）：`changes_view.rs` 的 `changes-retry` / `changes-operation-continue` / `changes-operation-skip` / `changes-operation-abort` / `changes-empty-action`、`log_view.rs` 的 `bottom-git-filter-field`（该处改前是 `.with_size(Size::Size(24px)).h_6().px_1p5()`——档位算出的外框内衬又被调用点压掉）。外框 24px、内衬 6px **逐像素不变**，字号 `text_base`→`text_sm`、gap `2`→`1` 随档位落下；依据：`Button` 的字体/图标/gap 吃 `self.size`，调用点 `.h_N()`/`.px_N()` 只能压外框内衬（`gpui-component-0.7.0/src/button/button.rs:611-614,646-674,721,737-748`）。**不改 18 处**是工具条/表头/分类头/行高（`changes_view.rs:811,1123,1181,1264`、`log_view.rs:594,617,672,779,829,1090,1267,1377,1421,1465,1596,1641,1756,1995`）：`h_flex()`/`div()` 撑出的布局带，`Size` API 不适用；⑥ 也**没有**虚拟列表行高契约（提交表未虚拟化，全 crate 无 `uniform_list`，故不涉及"只量第 0 行"前提）。**待裁决（盘点式内 1 处）**：`changes_view.rs` 的 `changes-commit` 提交按钮 `.h_7()`（28px；`Button` 档位只有 24/32，没有 28 ⇒ 改档必动可见外观）；另 `log_view.rs` 的 `LOAD_MORE_BUTTON_HEIGHT` 三处调用 `with_size(Size::Size(24px))`（**盘点式外那 1 处**：非手写定高、不在盘点式内）只给 `px(4.8)` 内衬、**不设外框高度**，实际高度由内容决定（`button.rs:657-658`）。⚠️ **未做真机视觉验证**，只有结构性核验。**⑨ 终端实测收口（D18 按区域做）**。`crates/terminal/` 复核 `h_5()`–`h_12()` 出现 **9 处**，其中 **4 处是注释/文档**（`constants.rs:143,146,148,180` 的度量说明表，非调用点），真实代码 **5 处**：(a) 组件被手写定高硬撑 **2 处已改** / (b) 纯布局元素 **3 处不改** / (c) 已是档位或组件 Size **0 处** / (d) 待裁决 **0 处**。**改动**（`Button` 的 `.h_6().px_2()` → `.small()`）：`terminal_view.rs:465`（空态「新建终端」）、`terminal_view.rs:512`（失败态「重试」）。外框 24px、内衬 8px **逐像素不变**，字号 `text_base`→`text_sm`、gap `2`→`1` 随档位落下；⚠️ 图标由调用点按 rem 递给 `child`（`idea_icon_image` 的 `side` 参数），**不走 `Button::icon_size` 故不随档位变**——这是本区域与 ⑥ 的差异点。**不改 3 处**是 `h_flex()`/`v_flex()` 撑出的带子（`terminal_view.rs:136` 空态页签条、`:252` 页签条外壳、`:295` 状态/边界提示行），`Size` API 不适用。⚠️ **终端字符网格是几何契约、不是密度旋钮**：`TERMINAL_LINE_HEIGHT`（`constants.rs:186`，倍数 1）与 `output_row_style()` 的上下 0 内边距决定等宽字符的行高与单元格几何，**未纳入密度盘点、也不得按 `Size` 档位改**；本轮改动只落在页签条 / 空态 / 失败态的**外壳控件**上。**同 crate 范围外的手写组件尺寸（非 `h_N()` 式，一并登记）**：`.px_0().size_6()` 方形图标按钮 4 处（`terminal_view.rs:190/574/598/629`）已是 rem 档位、24×24 恰等于 `Size::Small` 图标按钮尺寸（`button.rs:651-653`）⇒ (c) 不改；`.h(rems((24−4)/16.))`（`:320` 状态行「重试」，20px）与 `Styled::h_7`（`:537` 输入行 `Input`，28px）都落在档位外（`Button` 只有 20/24/32，`Input` 只有 20/24/32/44）⇒ **(d) 待裁决 2 处**，两处规格值都写在 `constants.rs:141-158`，改档必动可见外观。⚠️ **本区域同样未做真机视觉验证**。**③ 项目树实测收口（D18 按区域做）**。`crates/explorer/`（`explorer_view.rs` / `lib.rs` / `model.rs`）复核 `h_5()`–`h_12()` 出现 **7 处**，其中 **4 处是注释/文档**（`explorer_view.rs:76,84,86,102` 的度量表与历史说明，非调用点），真实调用点 **3 处**：(a) 组件被手写定高硬撑 **1 处已改** / (b) 纯布局元素 + 树行高契约 **2 处不改** / (c) 已是档位或组件 `Size` **0 处** / (d) 待裁决 **0 处**（以上是本轮**改前**盘点；改后本 crate 的文本计数为 9（2），差的 2 处是新增注释文本，见本行末尾复核栏）。**改动**（`Button` 的 `.h_6().px_1p5()` → `.small().compact()`）：`explorer_view.rs:674`（空态动作按钮 `explorer-empty-action`，即「打开文件夹」/「重试」）。外框 24px、内衬 6px **逐像素不变**（`Small` = `h_6().px_2()`、`compact` 再压成 `px_1p5()`，`gpui-component-0.7.0/src/button/button.rs:660-663`），字号 `text_base`(16px)→`text_sm`(14px) 随档位落下（`gpui-component-0.7.0/src/sizing.rs:327-333`），正好与同一空态里 `EmptyDescription` 的 `.text_sm()` 对齐；依据同 ⑥：`Button::with_size` 只写 `self.size`（`button.rs:559-562`）、字体与 gap 取 `button_text_size(self.size)` 与档位 gap（`button.rs:737-742`），而调用点的 `.h_N()`/`.px_N()` 经 `refine_style(&instance_style)`（`button.rs:721`）**只**压外框与内衬。⚠️ **explorer 是树，但树的行高契约不在本轮改动范围**：`explorer_view.rs:586` 的 `ListItem.h_6()` 与 `:546` 行内容 `h_flex().h_6()` 判 (b) —— 上游 `uniform_list` **只量第 0 行**再按固定行高铺满，行内容必须正好 24px，属**行为契约、不是密度旋钮**，**不得按 `Size` 档位改**（该 24 同时是 `file-tree-row.ts:1-14` 的 `max(24, uiFontSize×1.35+6)` 设计常量）；`ListItem` 承载的 `Size` API 无从接管这条契约。⚠️ **本区域未做真机视觉验证**，只有结构性核验。**④ 编辑区实测收口（D18 按区域做）**。`crates/editor/`（`src/**/*.rs`）复核 `h_5()`–`h_12()` 出现 **3 处**，其中 **2 处是注释**（`editor_view.rs:121` 度量表、`:2470` 标签栏 36px 的理由说明），真实调用点 **1 处**：(a) **0 处** / (b) **1 处不改** / (c) **0 处** / (d) **0 处**。**唯一调用点判 (b) 而非 (d)**：`editor_view.rs:2478` 的 `.h_9()` 挂在 `TabBar` 上确实是"组件被手写定高"，但**本轮仍不改**，三条证据 —— ① `TabBar` 自身**没有**高度（`gpui-component-0.7.0/src/tab/tab_bar.rs:495-499` 是 `group.relative().flex().items_center()`，无 `h_*`；`:504-514` 那条 `border-b` 用 `absolute().size_full()` 占位，不是高度源），高度由标签的 `variant.height(size)` 撑出（`tab/tab.rs:24-43`），所以**没有标签时会被压成 0** —— `.h_9()` 是"没有标签也照常占 36px"这条布局要求的实现，判 (b) 同 ⑥/⑨ 的"布局元素 `Size` API 不适用"；② 组件自带的档位高度是 26/30/36/44（`tab/tab.rs:24-43`，Underline 变体同表），**`.h_9()` 的 36 恰好等于默认 `Size::Medium`** ⇒ 当前实现已经是"组件中档高度"，"改档必动可见外观"这条 (d) 前提不成立；③ **`Size` 只影响标签内部几何、改不了标签栏总高**：切 `Small` 后标签 30px、内容 22px（`:52-55`）而栏内**没有其他撑高元素** ⇒ 标签栏在**有**标签时也从 36 掉到 30，`Large` 则升到 44，两种都是可见外观改动，而 macOS 侧无对应件可核（macOS 标签栏是 `LitheTheme.Metrics.tabHeight` = **34**，`macos/Sources/Lithe/Theme/LitheTheme.swift:477`，与 Windows 规格的 36 本就不等）。⚠️ **本区域红线：编辑器的行高 / 字符单元格几何是行为契约、不是密度旋钮** —— `FALLBACK_LINE_HEIGHT`（`editor_view.rs:160`）、`EditorState::line_height()`、`text_size` 与光标/命中几何（`editor_view.rs:2896-2931`）、`relative(...)` 的 line-height 全部判 **(b) 不改**，不纳入密度盘点、**不得**按 `Size` 档位改，**这不算缺口**。反面孔径：标签关闭按钮 `editor_view.rs:2852` 的 `.small()` 已经是"用档位对齐尺寸值"的正确写法（真机 `icon-xs` = 24×24 恰等于 `Size::Small`，`:2838-2841`）⇒ (c) 不改。⚠️ **本区域未做真机视觉验证**，只有结构性核验。**② 设置实测收口（D18 按区域做）**。`crates/settings/`（`src/**/*.rs`）复核 `h_5()`–`h_12()` 出现 **2 处**，其中 **1 处是注释**（`dialog.rs:215` 的 `h_11` 说明），真实调用点 **1 处**：(a) **0 处** / (b) **1 处不改** / (c) **0 处** / (d) **0 处**。**`dialog.rs:1285` 的 `.h_11()` 判 (b)**：它挂在设置对话框头部的 `h_flex()`（`:1283`）上，是**纯布局带**、不是任何组件实例，`Size` API 不适用。⚠️ 它**恰好**等于 `Size::Large` 的 `input_h` 44px（`gpui-component-0.7.0/src/sizing.rs:269-277`）—— 但那是 **Input** 的档位，不是 `Button` 的（`Button` 只有 20/24/32，`sizing.rs:327-333` + `button/button.rs:646-674`），所以"正好落在某个档位上"不构成改档理由，本轮没改。**settings 的组件档位本来就落得干净**：`Button` / `Switch` 的 `.small()` 调用点约 **20 处**（`dialog.rs:1311/1546/1701/2248/2305/2380/2389/2883/2893/3034/3044/3098/3106/3178/3403/3722` 等）、`row.rs` 的行最小高已经用 `.min_h_8()`（32px，`row.rs:135`；注意它**不**被本栏 `\bh_5\(\)`–`\bh_12\(\)` 正则计入，是"最小高"不是定高）⇒ **没有"组件被手写定高硬撑"的实例，(a) = 0**。⚠️ **本区域红线：设置页的行高 / 表单控件很多是规格值** —— 本轮**没有**换任何一行/控件的档；凡换档会动可见外观而 macOS 侧没有对应件可核的（对话框头部 44px 即其一：macOS 侧**没有**逐像素对应的设置对话框头部），**一律 (d) 待裁决、不擅自改**（本区域实际落到 (d) 的是 **0 处** —— 归 (b) 的那 1 处是纯布局带，连"换档"这个选择都不存在）。⚠️ **本区域未做真机视觉验证**，只有结构性核验。**① 工作台外壳（`workbench`）实测收口（D18 最后一个区域）**。`crates/workbench/`（`src/**/*.rs`）复核 `h_5()`–`h_12()` 命中 **33 行**（其中 **22 行是注释/文档**，含 5 行双命中）⇒ 真实调用点 **11 处**：(a) 组件被手写定高硬撑 **0 处** / (b) **11 处不改**（纯布局带 10 + 无 `Size` 手段的规格带 1）/ (c) **0 处** / (d) **0 处** ⇒ **零代码改动**（盘点式**内**为 0；式**外**另有 5 处一并登记，见本段末）。**逐处看"挂在什么上"（11 处全部打开过）**：**纯布局带 10 处** —— `extensions_page.rs:370`(32 头带)、`menu_bar.rs:2409`(24 常驻菜单栏带)、`:2462`(24 浮动胶囊带)、`:2539`(20 顶级项 `div()`，字号已显式 `.text_sm()`)、`replace_in_files.rs:626`(40 头带，内衬 `Input` 本身已走 `.small()`)、`right_tool_window.rs:628`(32 工具窗头带)、`search_everywhere.rs:497`(40 scope 行，真源 `SearchEverywhereView.swift:254`)、`status_bar.rs:308`(24 chip 带，真源 `LitheTheme.swift:480` 的 `statusBarHeight`)、`quick_open.rs:512` 与 `search_everywhere.rs:700`（`CommandItem::child` 里的 24 结果行，真源 `SearchEverywhereView.swift:423`）—— 接收者全是 `h_flex()`/`div()`，`Size` API 不适用。⚠️ **这两个结果行点明"虚拟列表行高契约"**：`Command` 把自定义行的高度当**测量输入**（`command/state.rs:641-680` 逐行 `layout_as_root` → `row_sizes` 交给虚拟列表；行外壳 `item_row` 另有 `py_1p5()` 包住自定义内容，`state.rs:705-720`）。**但要说准确**：它**不是** `uniform_list` 那种"只量第 0 行"——`Command` 量的是**每一行**；这两处判 (b) 的决定性理由仍是接收者 `h_flex()` + 24 是 macOS 规格行高。**规格带 1 处（字面像 (a)、但无 `Size` 手段）** —— `status_bar.rs:238` 的 `.h_6()` 挂在 **`StatusBar` 组件**上，而该组件根本没有 `Size` API：`gpui-component-0.7.0/src/status_bar.rs:32-37` 只有 `style`/`left`/`right`/`children`、**仅 `impl Styled`**（全 crate 21 处 `impl Sizable` 里没有 `StatusBar`）⇒ 密度条文给的手段在此不存在；24 又正是真源 `statusBarHeight`，故判 (b) 而非 (a)。⚠️ `menu_bar.rs` 三处的规格出处（`:133,136` 表里的 `ui/menubar.tsx:35-38,85-88`）是**已作废**的 `windows/tauri` 前端 ⇒ 无有效真源可核，这加强了"不改"的判定（属 D15 的注释清理范畴，不在本轮）。**盘点式外一并登记 5 处（非 `h_N()` 式，不计入本栏计数）**：`activity_bar.rs:449-450` 的 `.w(width).h(height)` 挂在 **`Button`** 上（左 26×24 / 右 28×28）⇒ **(d) 待裁决** —— 24 在档位内、但同盒的 26 不在，右轨 28 更是无档（`Button` 图标档只有 20/24/32，`button.rs:651-653`）；`height` 是两轨共用的运行期 `Length`，改档要按 `side` 拆链（动布局结构）；且 `Size` 会经 `Button::icon_size`（`button.rs:611-614,744-749`）改 **Lucide** 图标的实际边长 —— 当前默认 `Medium` 落 `size_8()` = 32，相对该 26/28 项盒已经偏大（`ActivityIcon::apply` 的 expui 分支走 `child` + 固定 `rems(1.25)`，不受档位影响，`activity_bar.rs:172-175`）⇒ 改档必动可见外观，本轮不做；`branch_panel.rs:369` 的 `div().h(rems(28. / 16.))` 包住 `Input.h_full()` ⇒ **(d) 待裁决**（`Input` 档只有 20/24/32/44，28 在档外；规格出处 `ui/command.tsx:55` 同样是已作废前端）；`project_tabs.rs:420` 的 `.with_size(Size::Size(28))` ⇒ **(c) 已是组件 `Size` API**；`right_tool_window.rs:658` 的 `Button.size_6()` ⇒ **(c)** 24×24 恰等于 `Size::Small` 图标按钮（`button.rs:652`）；`branch_panel.rs:490` 的 `h_flex().size_5()`（前导槽）⇒ **(b)** 布局槽位。其余 `.h(rems(P / 16.))` 形式的带子（`title_bar.rs:490`、`project_tabs.rs:245,292`、`search_everywhere.rs:534`、`replace_in_files.rs:832,909`、`project_menu.rs:538,1172`、`branch_panel.rs:439,1029`、`workspace.rs:5238`）本来就是本栏口径要求的"档位外写 `rems(P/16.)`"正确写法，接收者也都是 `div()`/`h_flex()`（自绘，`Size` 不适用）⇒ 不属手写定高、不改。⚠️ **本区域未做真机视觉验证**，只有结构性核验。**全仓复核计数（口径：`crates/*/src/**/*.rs`，不含 `tests/`；`\bh_5\(\)`–`\bh_12\(\)` 的命中**行数**（同一行出现两次记 1 行：全仓仅 6 行双命中 —— `git/src/model.rs:46`、`workbench/src/project_menu.rs:215`、`project_tabs.rs:120,125`、`title_bar.rs:129,489`，故"匹配总数"比它多 6：`git` 37 / `workbench` 38。上一版写"正则出现次数"但记的是行数，此处把口径写明） —— `\b` 词边界把 `min_h_8()` 这类"最小高"排除在外，它不是手写定高；括号内 = 排除**整行**注释与文档后的真实调用点；D18-④/② 那轮在 `2ad2eae0` 工作区实测、D18-① 本轮在 `143e370f` 工作区复核，`git status` 均确认改动全在 `rust/lithe-gpui/`）**：`workbench` **33（11，D18 已收口）** / `git` **36（19）** / `terminal` 11（3）/ `explorer` 9（2）/ `editor` 3（1）/ `settings` 2（1）处；`app`、`extensions`、`java`、`notify`、`run`、`shared` 均 **0 处**。D18-④/② 那轮**实测更正**本栏上一版两处：① `git` 37（19）→ **36（19）**（真实调用点 19 不变 ⇒ 差的是注释/文档文本行）；② `workbench` 32（8）→ **33（11）**（真实调用点 +3，是 `workbench` 在 `0dd2ac84` 之后继续落地的界面改动所致，**本轮未碰 `workbench`**）。`terminal` / `explorer` / `editor` / `settings` 四处与上一版一致；D18-① 本轮复核，这六个数与 `143e370f` 工作区**逐位吻合**（唯一变量是并发写者之后再落地界面时 `workbench` 的 33 会漂，届时整仓复核请读括号内那个数）。✅ **D18 已全区域收口**（六个区域：**① 工作台外壳**、**② 设置**、**③ 项目树**、**④ 编辑区**、**⑥ Git**、**⑨ 终端** —— 逐个按区域做完，不再有"其余区域未做"）。**仍未做的（如实登记，均不是某个区域自己能在本轮收掉的）**：① **⑥ Git** 的提交按钮 `.h_7()`（本轮实测 `changes_view.rs:1577`（本栏上文原记 `:1575`，已漂 2 行；两处现已统一按按钮名 `changes-commit` 记，不认行号；28px；`Button` 档只有 20/24/32，且 macOS `CommitAreaView.swift:96` 无显式高度可核）—— 待裁决；② **⑨ 终端** 2 处档位外手写尺寸（`.h(rems((24−4) / 16.))` 状态行「重试」20px、`Input` 的 `Styled::h_7` 28px）—— 待裁决；③ **"lucide 图标不随档位变"这一类既有缺口**：档位只驱动 `Button::icon_size`（`button.rs:611-614,744-749`），而本项目大量图标是**调用点自己按 rem 递给 `child`**（`idea_icon_image(.., rems(P/16.))`、`FileIcon::render(rems(P/16.))`、`svg().path(..)` + `.h(rems(14. / 16.))`）⇒ 换档**不会**让它们一起变，与 `design-guides.md:208` 的"图标一起变"仍有距离；这是**独立改动**（要动共享图标层，不在 D18 任一区域内）；④ 编辑器行高/字符单元格几何、终端字符网格、`uniform_list` 行高—— **是行为契约，不是缺口**（见本栏 ③④⑨ 段）。更早那版记的 `workbench` 12 / `terminal` 5 / `explorer` 3 / `editor` 1 / `settings` 1 是旧口径，已漂。 |

**B. 布局与对齐**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| `h_flex()` 会把 child 在**交叉轴居中**，`v_flex()` 才是 `stretch`；row 里装 column 必须显式 `items_stretch()`（或给 column `h_full()`），否则 header 会被挤出并裁掉 | coding §布局、测量与滚动 | 已踩过同类坑，按此检查 |
| 布局与 overflow owner 必须唯一；flex 里可收缩 child 用 `min_w_0()` / `min_h_0()`；`Scrollable` 挂在真正滚动的区域上，内容内边距放在 scroll owner **内部** | coding §布局、测量与滚动、design §可调整的桌面窗口 | 滚动条若悬在内容与面板边界中间，说明 owner 错了 |
| **精确对齐是质量不变量**：两条本该相等的边或间距，差 1 个渲染像素就是缺陷；相等关系应来自**同一个** rem helper / spacing token / 共享 inset，**不能用 raw pixel offset 修补** | design §对齐细节 | ✅ **`D23` 实测（本轮）**：查了 5 类"该相等"关系 —— ①**行与表头 inset**：`git/changes_view.rs:1186`（分类头 `px_2`）vs `:1269`（变更行 `px_1p5`）**不是应相等关系** —— macOS 真源本来就不共用起始线（`ChangesSidebarView.swift:717` 头 `padding(.horizontal, 7)` vs `:812-813` 行 `leading = 30 + leadingInset` / `trailing 6`）；②**图标槽**：`explorer/src/explorer_view.rs:504-521` 目录行与文件行共用同一个 `size_4().mr_0p5()` 空槽（指南"图标放入固定槽位"已满足）；③**圆角**：见下"收敛 3 处"；④**hairline**：`crates/**` 共 **38 处** `border_b_1()` / `border_t_1()` / `border_r_1()`，全部走 gpui 的 1 设备像素 helper（`gpui-pre-macros-0.3.7/src/styles.rs:1338` 的 `px(1.)` + `gpui-pre-0.3.7/src/window.rs:3142` 的 `snap_border_widths`）⇒ **是 `coding-guides.md:288`「one-device-pixel hairline」合法例外**，不是 offset 补丁；可改区域内**未发现**"相邻两块各画一次同一条分隔线"；⑤**重复 spacing**：对 13 个 crate 机器扫描全部 `const NAME: f32`，同一 crate 内同名两处**只有** `ICON_BUTTON_RADIUS`、跨 crate 同名同值**只有** 3 对，已全部收敛。**改 3 处、收敛到 2 个 owner**（详见 `git/src/model.rs:67-86` 与 `crates/shared/src/design.rs`）：(a) `git` 的 `--radius × k` 阶梯只留 `RADIUS_BASE`/`SM`/`MD`/`XL` 一份定义（`model.rs:80-86`），`git` 内 7 个具名圆角常量改为派生 —— 改前它们各自硬写 `6.4`/`4.8`，其中 `ICON_BUTTON_RADIUS` 在 `changes_view.rs:116` 与 `model.rs:82` **同名同值两份**；(b) `ROW_RADIUS`(4) 与 `ROW_LINE_HEIGHT`(1.35) 由 `explorer`+`git` 各写一份 → `shared/src/design.rs:23,32`；(c) `TAB_MAX_WIDTH`(200) 由 `editor`+`terminal` 各写一份 → 同上 `:42`。**可改区域的 raw pixel offset 补丁 = 0 处**（`px(1.)` / `px(-1.)` / `+ 1.` / `- 1.` / `rems(1. / 16.)` 在 `crates/{editor,explorer,git,terminal,settings,shared}` 内除注释外无命中）。**待裁决 2 处（未改）**：(a) `git/log_view.rs:838-840` 表头「提交」**不预留**泳道槽，而行有 `graph_cell` 的 `min_w(rems(GRAPH_MIN_WIDTH / 16.))` —— macOS `GitGraphView.swift:707` 明确预留 `Color.clear.frame(width: graphWidth)`，但 macOS **根本没有这个表头** ⇒ 表头的规格值无从确证，且补槽会动可见布局；(b) 同一个 `rounded-md` 有**两个 owner 且值不同**：`settings/src/row.rs:78`、`settings/src/dialog.rs:3520` 走 `cx.theme().radius`（`gpui-component-0.7.0/src/theme/schema.rs:66-68` 默认 **6px**，且是固定 `Pixels`、**不随 rem 缩放**），而 `git`/`explorer` 走 **6.4** 并用 `rems(P / 16.)` 随字号缩放 —— macOS `LitheTheme.swift:484` 的 `controlCornerRadius = 6` 与 Lithe 阶梯的 `6.4` **规格本身冲突**，不能猜哪个是规格值。**workbench 的 3 条欠账已由 `U25` 收口（2026-10-03）**：(1) 负外边距 `-WORKBENCH_GAP_SPEC` 补丁**已删**（`workspace.rs` 的注释保留了这段历史：`WORKBENCH_GAP_SPEC` / 负外边距 2026-09-30 删除），改由 **hairline 的共同 owner** 表达（组内 `gap_0`），不再是负边距补丁 —— 这正是本条文点名的 raw offset 形态，现已消除；(2) `workbench` 6 个文件各自的 `6.4` 常量（`branch_panel.rs` / `menu_bar.rs` / `project_menu.rs` / `right_tool_window.rs` / `status_bar.rs` / `title_bar.rs`）**全部收敛到** `lithe_gpui_shared::design::CONTROL_RADIUS`（`crates/shared/src/design.rs:50`）；(3) `right_tool_window.rs` 对 `ROW_RADIUS` 的行号引用**已改符号式**。⚠️ **未做真机视觉验证**：相等性依据是**结构性派生**（同一个 `const` / token），不是实测 resolved bounds；`coding-guides.md:400` 要求的 geometry assertion / visual regression 覆盖本轮未加（常量断言等于重抄规格值，真正的 resolved-bounds 断言需要 GPUI 测试窗口）。**2026-10-03 追加（`B14`）**：活动栏 rail 宽也按这条收敛 —— 宿主左右两条 rail（`crates/workbench/src/activity_bar.rs` 的 `COLLAPSED_WIDTH`，经 `rail_width_spec`）与 `settings` 的通知右内衬（`NOTIFICATION_MARGIN_RIGHT_SPEC`）共用 `crates/shared/src/design.rs` 的 `ACTIVITY_RAIL_WIDTH_SPEC`(40)，两侧都不再各写一份数字，也**没有**用 raw pixel offset 补差；可见改动、真源补正与残留见 §2.2 ①。 |
| 选中态由**条目自身表面**表达（填充 / 更强前景色 / 更重字重），**不要用左侧竖条或单侧边框**当选中标记 | design §交互状态 | ✅ 活动栏用 `Button::selected` 出底色（`activity_bar.rs`），未用单侧竖条 |
| 相邻区域不能各画一次同一条 hairline（分隔线归边界所有者） | design §对齐细节 | 未核 |

**C. 初始化、状态与生命周期**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| **两个 `open_window` 语义不同，别混用**：`cx.open_window(...)`（gpui 的 `App::open_window`）要求闭包返回**窗口根视图**，所以那里**必须**自己 `Root::new`；`gpui_kit::open_window(...)` 是包装版，内部已 `Root::new`，闭包**只返回应用内容、不能再包一层 `Root`** | coding §初始化与 Root 所有权 + `gpui-kit-0.7.0/src/lib.rs:153` | ✅ 本项目用 `cx.open_window`，并在闭包里 `Root::new`（`crates/app/src/main.rs:1070` 与 `:1239`），**这是正确的一侧**；两者效果等价（都是"Root 内包 ShellWorkspace"）。指南给的 `gpui_kit::open_window` 更省事：它直接返回 `(AnyWindowHandle, Entity<V>)`，不必自己捞实体 |
| `render` 中**不能**无条件 mutate / notify / 起 task；不要每帧重建 Entity / Subscription / FocusHandle | coding §性能规则 | 已修过同类问题，继续守 |
| 昂贵计算进 background executor，回前台更新；结果必须绑 revision/ID 并**拒绝 stale** | coding §异步任务与副作用 | 已有多处代次守卫 |
| 缓存 resolved layout 的东西要把 **`window.rem_size()`** 纳入失效键（换行行高、text shaping、virtual-list 测量、popup 几何、由文字推导的图标尺寸） | coding §基础字号控制应用缩放、§列表表格与大数据集 | ✅ **`D19` 实测（本条原为"需逐个核对"）：本项目没有"缺 `rem_size` 失效键"的缓存缺陷**。逐处核对：真正的跨帧缓存只有 3 处触发器盒子 `Bounds<Pixels>`（`crates/workbench/src/project_menu.rs:992`、`crates/workbench/src/menu_bar.rs:1716`、`:1718`），三处 `on_prepaint` 都**无条件逐帧覆写**（`project_menu.rs:1295`、`menu_bar.rs:2440`/`:2546`）⇒ 改字号经 `refresh_windows()` 出新帧就重测；`uniform_list` 每次 `request_layout` 都重新 `measure_item`（`gpui-pre-0.3.7/src/elements/uniform_list.rs:283`）；`Command` 的测量键显式含 `rem_size` / `line_height` / `content_width` / `text_shape`（`gpui-component-0.7.0/src/command/state.rs:98-103`）；text shaping 走上游、`LineWrapper` 由 `(font, font_size)` 构造（同 crate `text_system.rs:545`）；popup / dialog 几何全部每帧现算。**唯一真缺陷**：`lsp.completion_menu.max_width` 被上游**每帧**读（`gpui-component-0.7.0/src/input/popovers/completion_menu.rs:383`）却只在 `open()` / `prepare_java()` **一次性**写入 ⇒ 改界面字号后**已打开**的补全弹层宽度仍是旧 rem 基准（重开文件才对）。**本次已修**：`EditorPane` 观察全局 `Theme`（`crates/editor/src/editor_view.rs` 的 `_theme_subscription` + `sync_completion_menu_width`），字号一变就按 `cx.theme().font_size` 经 `crates/editor/src/completion.rs` 的 `set_menu_width` 重写该槽（只动装了 provider 的编辑器，**没有**退回写死 `px`）。另：`DisplayMap` 构造时传入的 `window.rem_size()`（`gpui-base-0.7.0/src/input/base/state.rs:706`）只是初值 —— 每帧 prepaint 都按当帧 `text_size = font_size.to_pixels(window.rem_size())` 重调 `set_font`（同 crate `base/element.rs:2425`、`:2434`），`TextWrapper::set_font` 值一变就 `update_all` 重排（`display_map/text_wrapper.rs:261-269`）⇒ **无陈旧软换行边界** |
| 长集合必须 virtualize，只 render 可见 range；row identity 与可见位置分开；**width/typography/rem/row 内容变化要失效 item 测量** | coding §列表、表格与大数据集 | 提交表当前未虚拟化（见 §2.2 ⑥） |
| 先绑键再建菜单栏：`cx.set_menus` 在调用那刻读取 keymap，之后注册的 binding 不会出现在菜单里 | coding §事件、Action 与焦点 | ✅ 本项目**不适用**：全仓零处 `cx.set_menus` 调用（菜单栏自绘 + `PopupMenu` 在渲染期反查 keymap），本条文的 native-menu 前提不存在。实测结论与六处 `bind_keys` 的调用链见 §1.4 N（`D20`） |
| 一个逻辑命令只建模一次：工具栏按钮、菜单项、右键菜单、快捷键 dispatch **同一个 Action**，不复制五份 mutation | coding §事件、Action 与焦点 | 与 §2.2 ⑥ 的"引用行右键 17 项"实现方式相关 |
| `Link` **只用于**交给浏览器/邮件客户端的外部目标；应用内导航与命令用 `Button`/`Action` | coding §事件、Action 与焦点、design §按钮与链接 | ✅ **零使用点，无违规可修**（`D22` 实测）：`crates/*/src/**` 全部 **99 个 `.rs`**（当时实测；现 **103** —— 本会话新增 4 个搜索模块）中 `Link`、`.href(`、`open_with`、`cx.open_url(` **均 0 处**。唯一 `Link` 字样是**图标名**（`shared/src/icons/idea.rs:216-219` 的 `externalLink.svg`，与元素无关）加两处**注释**（`editor/src/diagnostics.rs:222`、`notify/src/model.rs:56`）—— 既没有"应用内导航误用 `Link`"，也就没有外部目标接线点（全仓 `open_url` 调用点同样 0，仅本条与 §1.4 J 组提及）。⇒ 不修、无需替换为 `Button`/`Action`；"点开外部链接"若要落地属 U 系列新能力，不在本条 |

**D. 文案与无障碍**

| 条文 | 出处 |
| --- | --- |
| 入口/对象用名词、命令用动词、状态用形容词；避免"管理/模块/页面/功能/操作/系统"这类包装词 | design §界面用词 |
| 按钮一两个词且说明**结果**；不提交并离开的操作用"取消"；`OK` 只用于纯信息确认；打开对话框/需要后续输入的命令加 `…` | design §按钮与确认对话框、§大小写标点 |
| 英文用 sentence case；单省略号字符 `…` 而不是三个句点；中文用全角标点；短控件文字不加句末句号 | design §大小写、标点与符号 |
| 每个领域对象/命令/状态一个固定术语；翻译 key 描述稳定意图（`dialog.delete_project.title`），不照抄源语言句子 | coding §文档与界面文案 |
| 仅图标控件**始终**需要无障碍名称；tooltip 只用于陌生符号、模糊范围或未展示的快捷键 | design §字体与图标、§反馈与浮层 |
| 完成前过一遍无障碍检查表与设计评审清单 | design §无障碍检查表、§设计评审清单 |

> ⚠️ **对 §1.2 的修订**：本文档早先写"macOS 有而 gpui-kit 没有的 token 类别 → **自建一层颜色表**"。
> 按指南，正确做法是**在产品 theme / semantic tokens 层补语义 role**，而不是在 call site 自建 palette。
> 好消息是**本项目实际也没自建**（实测无 `LitheColors`、无 raw color），所以这一条只是文档说法要改。

**E. 滚动、几何与坐标（`style.md` / `geometry.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| 每个滚动区域**只有一个 owner**；`.overflow_y_scroll()` 要有状态元素（先 `.id(...)`）；要可见滚动条用 `.overflow_y_scrollbar()`（来自 `ScrollableElement` 扩展，**不是** `Styled` 方法） | style §选择裁剪、滚动或定位 | — |
| 滚动条要贴面板边缘，**内容 padding 放在滚动区域内部**；"滚动条悬在内容与边界中间"通常说明 scroll owner 错了或 padding 放错了层 | style §排查布局结果、design §对齐细节 | 已登记为检查项（D23 同类） |
| 通用 `Styled` **没有** `z_index(...)`；叠放由**绘制顺序**决定。要覆盖而不占位用 `.relative()` + `.absolute().top_0().right_0()` | style §选择裁剪、滚动或定位 | 与 §4.3 的 z-index 结论一致（0.6.6 时也对） |
| `relative(0.5)` 是**父尺寸的一半**，不是 0.5 像素 —— `geometry.md` 把它列为常见错误 | geometry §布局前的 Length | — |
| `auto()` 是"交给布局决定"，**不等于零** | geometry §布局前的 Length | — |
| `Pixels` 是**逻辑 UI 像素、不等于显示器物理像素**；`Pixels::scale(f)` → `ScaledPixels`，`DevicePixels` 是整数设备像素（`px(12.).scale(2.)` 的类型仍不同于 `DevicePixels(24)`） | geometry §为什么不直接用整数 | — |
| `Bounds::contains()` **含上边与左边、不含下边与右边**（相邻矩形不会同时认领边界点）；`intersect()` 只算矩形，**不裁剪**绘制与输入 | geometry §Point、Size 与 Bounds / §滚动、裁剪与计算 | 自绘 element 时注意 |
| 自定义元素：`prepaint` 拿到的 `bounds` 已是**窗口坐标**，指针事件 `position` 也是窗口坐标；只有算子元素内部位置时才转局部坐标 | geometry §明确坐标原点 | 与 §2.2 ⑤ 的 diff/minimap 自绘相关 |
| `Length` 家族：`AbsoluteLength`（`Pixels`/`Rems`）→ `DefiniteLength`（或父比例）→ `Length`（或 `Auto`）；样式方法收哪种看签名 | geometry §布局前的 Length | — |
| `Edges<T>` 与 gpui-kit 的 `base::Edges<T>` 是**不同类型**（后者可序列化、可生成 JSON schema）；按目标 API 的签名选 | geometry §Edges、Side 与 Placement | — |
| `Placement` 是 trigger 的**一侧**（`Top`/`Right`/`Bottom`/`Left`），不是四边留白；`Positioner` 会翻转并 clamp 到 viewport | geometry §Edges、Side 与 Placement | 与 §2.2 ⑤ 的浮层定位相关 |

**F. 字体、图标与 i18n（`fonts.md` / `i18n.md` / `assets.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| 默认 UI 字体 `.SystemUIFont` **16px**；默认等宽 macOS `Menlo` / Windows `Consolas` / Linux `DejaVu Sans Mono`，**13px** | fonts §默认字体 | 本项目默认等宽是内置的 JetBrains Mono |
| `theme.font_size` **同时是应用缩放控制**：`Root` 会调 `window.set_rem_size(cx.theme().font_size)`，所以改基础字号会改变基于 rem 的几何 | fonts §通过 Theme 修改字体 | 与 §1.4 A 组、§2.2 ⑰ 一致 |
| **打包字体必须在首帧之前注册**（`add_fonts` 要早于开窗或任何会测量文字的对象）；字体族全加载不上时**排版会 panic** | fonts §打包自定义字体 | ✅ 本项目 `register_fonts`（`crates/app/src/main.rs:1028`）**早于** `open_window`（`:1070`），且注释写明了三条顺序硬约束 |
| `add_fonts` 只增不删；它会失效字体解析与**行布局缓存**，但已显示的窗口还要 `cx.refresh_windows()` 才会重排 | fonts §更换字体会改变布局 | 本项目注释已记录"运行期摘不掉" |
| 写 family 名要用**字体文件内部**的名字（可能与文件名不同） | fonts §打包自定义字体 | — |
| 自定义文本宽度要用**塑形后的行**测量，不能按字符数估算 | fonts §更换字体会改变布局、i18n §不同文字系统 | 与 §2.2 ⑰ 的"缓存要含 rem_size"同类要求 |
| i18n：应用 locale 放 `locales/ui.yml`，覆盖组件文案要放在 **`gpui_component` namespace** 下；`extend!` 必须在 `gpui_kit::init` **之前**、且一个进程只调一次 | i18n §创建 locale 文件 / §注册扩展 | ⚠️ 本项目**有意不用 `extend!`**（理由写在 `crates/app/src/main.rs:992`：那个宏要求把 locale 放在特定位置），并已在 `init` 前 `set_locale`（`:997`） |
| `set_locale` 是 GPUI 变更跟踪**之外**的全局状态，单独改不触发重绘；要 `cx.notify()`（view 内）或 `cx.refresh_windows()`（`&mut App`），缓存的菜单/标签还要显式重建 | i18n §切换语言 | ⚠️ 本项目把界面语言定为**重启后生效**（`crates/settings/src/store.rs:22` 等处的成文决策），所以不走热切 |
| 不要把分别翻译的片段拼成句子；整句放同一个 key | i18n §在应用中格式化数据、coding §文档与界面文案 | 与 §2.2 ① 的"状态栏假值"同类检查 |
| 这个 locale API **不提供 RTL 镜像**（选阿拉伯语/希伯来语只换文案，不镜像布局） | i18n §不同文字系统 | — |

**G. 窗口与生命周期（`window.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| `gpui_kit::open_window` 返回 `(AnyWindowHandle, Entity<V>)`：**窗口选择用 handle、状态更新用 Entity**；实际 GPUI root 是 `gpui_kit::base::Root`，**不是**你的内容 View | window §打开并持有窗口 | 本项目用 `cx.open_window` 自己包 `Root`，效果等价但拿不到那个返回对（见 §1.4 C 组） |
| 窗口关闭后，基于 handle 的更新**可能失败**，必须处理；`handle.update(...).is_err()` 是判据 | window §打开并持有窗口 | — |
| **几何原点不同**：`window.bounds()` 是**全局**显示坐标（可能跨多显示器），`viewport_size()` 是**窗口局部**逻辑像素；指针事件与 `visual_viewport_bounds()` 用**局部**坐标 —— **不要把指针位置与保存的全局窗口原点比较** | window §几何信息与缩放 | — |
| `scale_factor()` 只用于"真正需要设备像素"的原生集成边界；**GPUI 布局尺寸不要乘它** | window §几何信息与缩放 | 与 §1.1 第 1 条、§1.3 根视图尺寸一致 |
| `window.defer` / `cx.defer_in` 是"**当前 effect cycle 结束后**"；`on_next_frame` 是"**下一次平台帧 tick**" —— **两者不是同一个时机**（`on_next_frame` 用于推进动画这类明确属于下一帧的事） | window §在当前更新结束后执行 | 本项目 10 处 `defer` / `on_next_frame`（`main.rs` 为主） |
| 不要在某 Entity 的 callback 里再通过 handle `update` **同一个** Entity（会重复更新正在更新中的 Entity）；这也是 `defer`/`defer_in` 的主要用途（关浮层后调焦点、继续改同一棵 UI 树） | window §在当前更新结束后执行 | 与 §2.2 ⑤ 浮层焦点归还相关 |
| **帧 tick 不等于重绘**：`on_next_frame` 会产生帧需求但**不把窗口标记为 dirty**；改了 Entity 要 `cx.notify()`，改了窗口级状态要 `window.refresh()` | window §重绘与帧生命周期 | — |
| `window.request_animation_frame()` 在锁定的 `gpui-pre 0.3.7` 里会**立即调用 `current_view()`**，因此**只在 GPUI 有当前 View 的渲染路径中**使用；路径外改用 `on_next_frame` + 显式 notify/refresh | window §重绘与帧生命周期 | 本项目未用（用 `on_next_frame`） |
| 在 **render 中无条件**调 `cx.notify()` / `window.refresh()` / `request_animation_frame()`，即使 UI 空闲也会持续耗资源 | window §重绘与帧生命周期、coding §性能规则 | 与 D19 一类的每帧开销检查同源 |
| `window.remove_window()` 是**程序主动移除**；平台关闭请求的拦截用 `window.on_window_should_close(cx, callback)`（返回 `false` 可取消，**未保存确认由应用负责**）；窗口已关闭用 `cx.on_window_closed(callback)`，其 callback 里**访问不到**已关闭的 `Window` | window §Window 生命周期 | 与多窗口 `U10`、未保存确认（§2.2 ④）直接相关 |
| 订阅（`Subscription`）与 `Task` 要**存在 View 上**：只放局部变量会立刻 drop 并取消订阅；放更长命的 global 则可能在 View 消失后仍持有 callback 与资源 → 泄漏 | window §订阅中访问 Window、§需要 Window 的异步任务 | — |
| Task 谱系：需要 Entity+Window 用 `cx.spawn_in`；只需 Window 用 `window.spawn`；都不要用 `cx.spawn`；CPU 密集用 `cx.background_spawn`。`Task` 被 drop 即取消，所以"生命周期跟随 View"就存进 View，只有确实要独立继续才 `.detach()` | window §需要 Window 的异步任务 | 与 `D10`（运行启动阻塞 UI 线程）同类要求 |

**H. 图标与资源（`assets.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| **嵌入量对照**（原生、release、去符号）：默认组件图标 **101 个 = 44.28 KiB（基线）**；+2 个 = 45.04 KiB；+10 个 = 48.09 KiB；**`AllAssets` 1830 个 = 731.45 KiB，相对基线 +1.02 MiB** | assets §NOTE | ✅ **已评估并明确否决收窄（`D25`，2026-10-02）**。口径更正：0.7 的默认子集实测是 **104 个 / 46 349 B**（指南表里的 101 / 44.28 KiB 是 0.6 旧数未同步），`AllAssets` 实测 **1830 个 / 749 008 B = 731.45 KiB**（与指南逐位吻合）。本项目自嵌 **411 个文件 / 3 325 504 字节**（行尾无关口径，idea 254 + pierre 149 + app-icon 7 + logo 1）。**保留兜底的理由**：界面实际引用 **70 个图标名**，其中 **26 个在组件子集里、44 个只有完整 Lucide 目录才有**（`default-icons.txt` 里没有）—— 换句话说这 1.02 MiB **现在是承重的**，不是纯浪费；且全部 70 个名**静态可枚举**（无运行期拼出来的图标名），收窄在技术上可行，但那属于**改内嵌资源**的独立改动，本次按维护者决定**不做** |
| 指南给的更省做法：用 **`icon_assets!(ExtraIcons, [Accessibility, AlarmClock]);`** 按需选目录图标，再组合成一个 `AssetSource` | assets §用 `icon_assets!` 选择更多目录图标 | ⚠️ 本项目**未用**该宏，且**已否决**收窄方案（`D25`，2026-10-02 维护者定）：要走这条路，就必须把那 44 个"只有 Lucide 侧才有"的变体名逐个写进宏里 —— 而**漏登记任何一个都不会有任何编译或测试失败，只会静默画空白**。真要收窄，先补一条"界面用到的 70 个名逐个 `LitheAssets.load(name.path())` 必须 `Some`"的回归判据，把漏登记变成可执行失败，再动 `assets.rs` |
| `with_assets` **只能注册一个资源源**（重复调用是覆盖）；**必须在打开窗口前注册** | assets §从默认资源源开始、§自定义资源 | ✅ 本项目 `application().with_assets(assets::LitheAssets)`，`main.rs:951` |
| **组合资源源必须先查自定义源、再回退内置**：原生平台内置 `Assets`/`AllAssets` 对**未知非空路径返回错误**，会让后续回退失效（`load` 要先 `ExtraIcons` 后 `ComponentAssets`） | assets §用 `icon_assets!` 选择更多目录图标、§自定义资源 | ✅ 本项目正是这个顺序（`assets.rs`：先查自己的 rust-embed 表，再 `AllAssets`），并把 `AllAssets` 的 `Err` **折叠成 `Ok(None)`** |
| `list(prefix)` 要**合并 + 排序 + 去重**两个来源，使列出的键与 `load` 能提供的键一致；空路径返回 `Ok(None)` | assets §用 `icon_assets!` 选择更多目录图标 | ✅ 本项目 `list` 已合并+排序+去重（`assets.rs`） |
| `load` 与 `list` 走**不同代码路径**（`get()` vs `iter()`），"能 list 出来"不等于"能 load 到" | assets §打包与 WebAssembly | ✅ 本项目回归测试正是先断言路径在**内嵌表**里、再断言 `load` 拿到非空字节（`the_readme_is_not_embedded` / `probe_paths_are_loadable_from_the_embedded_table`） |
| **单色 SVG** 用 `Icon::path` / `svg().path(..)`（当作可着色的透明蒙版）；**照片、位图、多色 SVG** 用 `img()`（`object_fit` 只作用于 `img`）。`img()` 按**字节**识别格式，改扩展名没用 | assets §使用资源路径 | 与 §1.1 第 4 条、§2.2 ⑯ 的图标接线一致 |
| 内置 `AssetSource` 遇到**缺失路径**会返回错误、**空路径**返回 `Ok(None)` | assets §排查资源缺失 | ✅ 本项目的折叠正是为了这个 |

> **`assets.md` 里一句最该记住的话**：**"依赖图标 crate 不等于嵌入全部图标"** ——
> 只补依赖或只用共享 `IconName` 不会引用全部 SVG；**只有显式注册 `AllAssets` 才会**。
> 所以本项目那 1.02 MiB 是**我们的选择**，不是框架强加的成本。

**I. 状态、渲染与通知（`entity.md` / `render.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| clone `Entity<T>` 只复制**句柄**（同步更新强引用计数），不复制 `T`；但**每个保留的副本都会延长生命周期**，所以"只有所有权或回调确实需要时才 clone，避免高频循环里无故反复 clone" | entity §创建 Entity | — |
| **重入会 panic**：同一个 Entity 正在 `update` 或 `render` 时，**不要**再对它 `read` / `update`。要拿结果就让 `update` 闭包**返回**它（先结束借用），或用回调已给的 `&mut T`。被 deferred 的 callback 也守这条 | entity §更新状态 | 与 §2.2 ⑤ 浮层焦点归还、`window.defer` 相关 |
| `cx.observe(&e, …)` 与 `cx.subscribe(&e, …)` 是**两种独立信号**：`notify()` **不发** Event，`emit()` **不会**通知渲染者。两者都需要时分别触发 | entity §观察变化与订阅 Event | — |
| `Subscription` 必须存进**发起订阅的 Entity**（`_subscription` / `_subscriptions`）；放局部变量是"**过早取消**"（不是泄漏），`Subscription::detach()` 才是泄漏方向（callback 与捕获资源会累积） | entity §观察变化与订阅 Event | — |
| 反向引用用 **`WeakEntity`**，否则父子互相强持有会成环、两者都不释放：`Parent ──strong──→ Child ──weak──→ Parent` | entity §使用 WeakEntity | 本项目 `WeakEntity` 用得很广 |
| `cx.processor` **会捕获当前 View 的强句柄**（与 `cx.listener` 不同），所以不要把它的产物存回同一个 View | entity §使用 WeakEntity | — |
| **不能假设每帧恰好调用一次 `render`**，也不能假设父 View 每次渲染都会调用所有子 View 的 `render`：`render` 可能发生在布局或 prepaint 过程中，**有效子树可能被直接复用** | render §View 持有状态、render §返回元素树 | 与"render 里别做重活"同源 |
| **不要在 `render` / `prepaint` / `paint` / canvas callback 里无条件** `cx.notify()` / `window.refresh()` / 起 task / 注册订阅 / 发 Event / 改应用状态 —— 每帧重建都会重复执行，甚至让窗口持续 invalidate | render §Render 构建值、window §重绘与帧生命周期 | ✅ 本项目 render 里**没有**无条件 notify（核过：命中的两处都在关闭按钮的 click listener 里） |
| 给 View 加缓存用 `child.clone().cached(style)` / `AnyView::cached(style)`：父级必须保留**同一个**子 Entity，且 `style` 要给出确定外层尺寸（布局阶段可能跳过内容 render） | entity §Entity 身份与 View 缓存 | 未核 |
| `Entity<T: Render>` 的 `EntityId` 让 View 跨帧有稳定身份；**保留 Entity 与缓存其渲染子树是两回事** | entity §Entity 身份与 View 缓存 | — |
| 状态归属：文档/工作区这类**功能特有**状态由该功能自己的 Entity 持有；只有**范围确实覆盖整个应用**的才用 `Global`（一个类型只有一个存储位置），且 Global 变化要 View **显式观察**才会更新输出 | entity §在功能边界间传递句柄 | 与 `develop-lithe` 的 ownership 表一致 |
| 组件状态要在 `render` **之外**创建一次并存进字段（如 `query: Entity<InputState>`）；在 `render` 里造 `InputState` 会让草稿、选区、焦点行为每次重绘都重置 | entity §在 `render` 外保留组件状态 | — |

**J. Context 与异步（`context.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| `Context<T>` **解引用为 `App`**，所以拿到 `cx: &mut Context<T>` 不必再传 `&mut App`；但 `Window` 必须单独传（同一 Entity 可能显示在多个窗口，纯数据更新又可能不属于任何窗口） | context §开头 | — |
| **异步 context 是句柄，不是引用**：`AsyncApp` / `AsyncWindowContext` 用于跨 `await` 后重新进入；**不要把 `&mut App` / `&mut Window` / `&mut Context<T>` 存进 View 或 task**（只在当前调用期间有效） | context §开头、§创建读取更新 | — |
| **不要让 `read` 的引用跨越 `await`**：先 clone 出任务需要的少量数据 | context §创建读取更新 | 与 §2.2 ⑩ 的运行/终端任务相关 |
| **重入 panic**（与 entity.md 同一条，但这里给了做法）：在自己的 `cx.listener` 里 `this` 已是当前 View，不要再通过句柄 `update` 同一个 Entity；必须等当前 callback 结束就**推迟执行** | context §创建读取更新 | ✅ 本项目用 `cx.defer_in` / `window.defer` 处理这类时序 |
| **`cx.defer_in` 的闭包已经收到 `this: &mut Self`**，所以**不要**在里面再对同一个 Entity 调 `update`；用途是"UI 树变化后恢复 Focus"这类必须等更新结束的操作 | context §在当前更新结束后继续 | ✅ 本项目的浮层焦点归还、`--open-settings` 都是这个模式 |
| 延迟工作是**尽力而为**：窗口关闭或 View 释放后可能不执行，**不要当成持久任务队列** | context §在当前更新结束后继续 | — |
| Task 谱系（与 `window.md` 一致）：`cx.spawn`（仅 `AsyncApp`）／`spawn_in`（要 Window）／`cx.background_spawn`（CPU 密集，**不能直接改 GPUI 状态**，先回前台）。**Task 被 drop 即取消**；"属于 View 的工作存 View 上"；替换已保存的刷新/debounce 任务会取消旧任务 | context §异步任务、§Task 生命周期 | 与 `D10` 同源 |
| 观察**应用级**值要用 `cx.observe_global(...)`，并保存它返回的 `Subscription` | context §observe 与 subscribe | 本项目主题/设置监听正是这个模式 |
| callback 参数约定：运行时参数放**最后**且保持 `window, cx` 顺序（前面可以有 `action` / `event`）；**只处理数据的方法就别收 `window`** | context §`window, cx` 与只有 `cx` | — |
| 用 `cx.open_url(...)` 交给平台默认浏览器（`Link` 设了 `href` 也是走它）；要在窗口内嵌浏览器见 `webview` | context §在默认浏览器中打开 URL | — |

**K. `Render` vs `RenderOnce`（`render-once.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| 区别是**所有权**：`RenderOnce::render(self, …)` **消费组件值**（父级每次 render 造一份新的轻量描述）；`Render::render(&mut self, …)` **借用保留在 Entity 里的 View** | render-once §开头 | — |
| `#[derive(IntoElement)]` **必须与 `impl RenderOnce` 同时写**，否则 `.child(...)` 编译不过；而 derive **不会**转发 `Styled` / `ParentElement` / 焦点 / 无障碍 —— 要直接对自定义类型调 `.px_2()` 就得自己实现那些 trait | render-once §从构造到调用、§Builder 风格的组件 | 本项目自研组件时注意 |
| **`RenderOnce::render` 拿到的是 `&mut App`，不是 `&mut Context<Self>`**：所以它**不能**用 `cx.listener`、**不能**持有自己的 `Subscription` 或 `Task`、**不能** `cx.notify()` 让自己重渲 —— 要么由父级传 handler、要么派发 Action、要么更新真正持有状态的 Entity | render-once §状态应放在组件值之外 | 与 `D21`（一个命令只建模一次）同源 |
| 组件一旦开始积累可变状态、`Subscription` 或后台 `Task`，就应把生命周期搬进 `Entity` 并实现 `Render` | render-once §选择合适的层级 | — |
| 别把 `&mut Window` / `&mut App` 保留到本次调用之后；handler 必须 `'static`，所以 slot / handler 里要放**持有数据**的 `SharedString`、`AnyElement` 或 clone 后的 `Entity` handle | render-once §持有的值与 render 生命周期 | — |
| 常见组合：`Render` View 持有状态 → 创建 `RenderOnce` 组件描述可复用 UI → 组件返回内置 Element。**只有内置 Element API 表达不了所需渲染行为时才自己实现 `Element`** | render-once §选择合适的层级 | 与 §2.2 ⑤ 的自绘 diff/minimap、⑨ 的 VT 渲染相关 |
| 重复行的稳定 ID 要来自**领域数据**（如 `("open-message", message.id)`），**显示文字不能作为身份** | render-once §从构造到调用、§持有值 | 与 `element_id.md` 同一条，§2.2 ④⑥ 已按此做 |

**L. 稳定标识（`element_id.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| 唯一性规则：**同一个最近的带 key 祖先之下，各条带 key 的后代分支必须 ID 不同**；**没有 key 的容器不开辟新命名空间**（所以 `div().child(div().child(div().id("item")))` 两次会撞） | element_id §GlobalElementId 与祖先路径 | — |
| row 先设 key、子控件再设 `"edit"`/`"delete"` 这类局部 key；**行 key 不会自动区分同一行内的兄弟控件**（同一行两个 `Button::new("archive")` 仍冲突） | element_id §变化列表中的稳定 key | — |
| `.id(index)` 把状态绑到**位置**：插入一行后，原来第一行的**焦点、滚动、动画**可能被另一条消息复用 —— 只有"位置本身就是身份且不会移动"时索引才合适 | element_id §变化列表中的稳定 key | ✅ 本项目提交行/变更行/标签都用哈希或相对路径 |
| ⚠️ **本项目直接相关的官方实例**：Dock 的左/右/下三个 `resize_handle` 可能落在**同一个带 key 祖先**下，若都叫 `"resize-handle"` 就会拿到同一 `GlobalElementId` —— "按住一个 handle 可能触发另一个的拖动"；官方实现按方位取 `"resize-handle-left"` / `-right` / `-bottom` | element_id §对其他行为的影响 | ✅ **已按此实现**：本项目三块分栏（左栏 / 右工具窗 / 底部工具窗）的手柄在 `workbench/workspace.rs`，id 互不相同（`lithe-left-pane-resize` / `lithe-right-pane-resize` / `lithe-bottom-pane-resize`），并有单测钉住"三个 `ElementId` 互不相同"（同 §2.2 ①） |
| `window.use_keyed_state(key, cx, init)` 返回 `Entity<S>`，连续帧同 key 才保留；**GPUI 会观察它，状态变化时通知当前 View**。`window.use_state(cx, init)` 用**调用位置**当局部 key，只要求完整路径唯一 | element_id §ID 如何保留状态 | 与 §1.3 的"分栏手柄用窗口级鼠标跟踪"实现相关 |
| **加 `.id(...)` 不会让父级跳过 `render`** —— ID 是身份，**不是 render 缓存**；要跳过得用 `ViewCache` | element_id §对其他行为的影响 | — |
| 改变元素 ID 或带 key 祖先的 ID → 新路径 → **重置该元素状态**；元素被移除 → 状态生命周期结束，重建时重新 `init`。**要在条目消失期间保留的东西必须放 Entity / 模型** | element_id §身份变化也会改变状态 | — |
| 无障碍：自定义元素**需要 ID + role** 才能成为节点；稳定 ID 只保证重绘后身份不变，**不自动提供 role / 标签 / 键盘行为 / 可聚焦** | element_id §对其他行为的影响 | 与 §1.4 D 组"仅图标控件必须有无障碍名称"配套 |
| UI 集成测试里 ID 是**查询选择器**，不是第二套身份：`window.find("archive")` 要求唯一，重复控件要用 `window.within(("message", 42_u64)).find("archive")` 限定作用域；`find` 报"不唯一"**不一定**是状态冲突 | element_id §排查错误的 ID 路径 | 与 `test.md` 的 UI 测试相关 |
| `window.with_element_state(global_id, …)` / `window.with_global_id(key, …)` 是**给元素作者**在绘制期用的；普通 View 优先用 Entity 状态或组件 API | element_id §ID 如何保留状态 | 自绘 element 时用 |

**M. 视图缓存与焦点（`view-cache.md` / `focus.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| 真正的缓存 API 是 **`Entity::cached(style)`** / **`AnyView::cached(style)`**（当前没有需要构造的公开 `ViewCache` 类型）；命中时**不调用子 View 的 `render`**，而是重放上一帧的 prepaint/paint 记录（含命中区域、事件节点、焦点状态、监听器） | view-cache §开头、§GPUI 何时复用 | 未核 |
| `style` 是缓存 View 的**外层布局约定** —— 盒子必须有**确定尺寸**（GPUI 无法向尚未 render 的子树问固有尺寸）；要让内容决定尺寸就**别用缓存** | view-cache §缓存由 Entity 支撑的子树 | — |
| 命中条件：Entity 干净 **且** 缓存 View 的**边界、内容裁剪区域、继承的文本样式**与记录帧一致；后三者任一变化都未命中。**缓存属于窗口的帧，不属于 `Entity<T>`**（同一 Entity 在两个窗口各有一份） | view-cache §GPUI 何时复用 | — |
| 缓存**不解决外部依赖**：子 View 读 Global 就得观察该 Global 并 `notify` 自己；读另一个 Entity 就要观察它。**"重新 render 父级"不会刷新仍干净的缓存子级** | view-cache §GPUI 何时复用 | — |
| `RenderOnce` 值与任意 `ViewElement` **不能**用这个公开缓存 API（没有通知机制去失效冻结的子树） | view-cache §缓存由 Entity 支撑的子树 | — |
| 四种机制别混：`Entity::cached` 省 render+layout+paint；`use_keyed_state` 省重建后的**少量状态计算**（**不会**跳过 render）；模型持有的缓存管派生值与绘图资源；`VirtualList` 只**创建可见范围**的行 | view-cache §开头（对照表） | 与 §2.2 ⑥ 提交表未虚拟化相关 |
| 选择最小有效边界：**先用普通 Entity 与声明式 render**，profiler 证明某稳定面板被旁边 UI 变化反复构建时才加 `cached`；成本随集合长度增长才上虚拟列表。**不要用大缓存去掩盖本该移出 `render` 的计算** | view-cache §选择最小的有效边界 | — |
| **焦点四个操作不能混**：`cx.focus_handle()` 只创建目标（**默认 `tab_stop` 为 `false`**）；`.track_focus(&h)` 只登记到已渲染元素（**不移动焦点、不自动进 Tab 顺序**）；`h.focus(window, cx)` 只移动焦点；`.tab_stop(true)` 只让它可被 Tab 访问 | focus §区分四个操作 | — |
| **句柄要归长期 owner（通常是 `Entity<T>`）所有**，每帧 `render` 重新附上**同一个**句柄；每次渲染新建句柄会让焦点身份与 Tab 行为在用户操作时变化。无状态组件可用 `window.use_keyed_state(...)` 保留句柄（Kit 的 `Button` 就这么做） | focus §区分四个操作 | ✅ 本项目 `focus_handle()` 只在 owner 构造期创建一次（`workspace.rs:1568`） |
| **Tab 配置要设在句柄上**：元素自带的 `.tab_index(...)` / `.tab_stop(...)` **不会**修改传给 `.track_focus` 的句柄 | focus §鼠标、Tab 和键盘命令 | 本项目 `tab_stop` 11 处 |
| `.focus_trap(id, &container_handle)` **只做 Tab 约束**，它**不会**打开模态、自动进入第一个控件、阻止鼠标聚焦外部、也不在关闭后恢复焦点。完整模态流程要自己做四步（保存 `window.focused(cx)` → 明确选初始目标 → 关闭时仅在原目标仍是有效已渲染目标时恢复 → 验证双向 Tab/Esc/点外/禁用项/嵌套/原目标消失） | focus §在弹层中约束并恢复 Focus | ✅ **本项目 `focus_trap` 零处，但这是对的**：所有真模态都走**上游 `window.open_dialog`**（设置对话框 `settings/src/dialog.rs:159,3651`、分支面板 `git/src/changes_view.rs:620`、编辑器关闭确认 `editor/src/editor_view.rs:2181`、命令面板），而 **`Dialog` / `Sheet` 自带焦点陷阱**（`focus.md` 与 `gpui-kit` skill 的组件目录都写明）。**⛔ 我此前把这里标成"⚠️ 未验证"是错的**——它不是缺口。仍不在此契约内的只有**非模态**浮层（资源管理器搜索 Popover、编辑器代码动作菜单），它们本来就不该有模态焦点行为 |
| `is_focused`（精确）vs `contains_focused`（含已登记子元素，用于"面板里输入框聚焦时面板仍显得活动"）；包含关系来自**最近一次渲染的 dispatch tree**，仅有 Entity 所有权关系建立不了它 | focus §精确 Focus 与区域内 Focus | 本项目用 `contains_focused` 2 处、`is_focused` 0 处 |
| 焦点订阅（`on_focus` / `on_blur` / `on_focus_in` / `on_focus_out`）在**构造 owner 时注册一次**并把 `Subscription` 存 owner；**每次 `render` 注册会累积重复回调** | focus §订阅焦点变化 | — |
| **键盘命令只在焦点路径上可用**：handler 的 `key_context` / `on_action` 必须在当前聚焦元素的 dispatch path 上，兄弟元素上的 handler 不在路径里 | focus §鼠标、Tab 和键盘命令 | 与 §1.3 键事件派发那条一致 |

**N. 命令、快捷键与全局状态（`action.md` / `keybinding.md` / `global.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| **一个逻辑命令只建模一次**：`actions!(ns, [Cmd])` 生成稳定名称的类型；Toolbar 按钮、菜单项、右键菜单项、命令面板与 key binding 都应 dispatch **同一个 Action**，状态修改只写在 owner 的**一个** handler 里 | action §一条命令多个入口、coding §事件 Action 与焦点 | `actions!` 6 处、`#[derive(Action)]` 1 处、`on_action` 15 处集中在 6 个 owner（`editor_view` / `command_palette` / `workspace` / `changes_view` / `settings::dialog` / `menu_bar`）——**结构是对的**；`D21` 是"新增菜单时别各写一份 mutation"，不是返工项 |
| 派发两阶段：**Capture**（全局 capture → 根到目标）与 **Bubble**（目标到根 → 全局 `cx.on_action`）。**离目标最近的 handler 先跑，且默认停止传播**；不处理时必须显式 `cx.propagate()`（**单纯 `return` 不会传下去**） | action §Handler 顺序与传播 | 本项目 `cx.propagate()` **0 处**、`capture_action` 0 处、`stop_propagation` 27 处——但 27 处都是**有意的单点拦截**（点 × 不激活标签、面板内点击不上冒、Esc 拦截器），不是 Action 冒泡链。**当前不需要 `propagate`**；一旦做"子级不处理就交给父级"的嵌套命令，必须补 |
| 三种派发**目标不同**：`window.dispatch_action` → 调用时本窗口的当前 Focus（**延后执行**）；`focus_handle.dispatch_action` → 当前帧渲染该 handle 的区域（**明确指定目标**）；`cx.dispatch_action` → 活跃窗口，无活跃窗口时给全局 handler | action §Handler 顺序与传播（对照表） | 本项目 `dispatch_action` 10 处 |
| **直接派发不计算 Key Context predicate** —— 只有**按键**经 keymap 选 Action 时才匹配 context | action §Handler 顺序与传播 | — |
| 并列区域（如 Sidebar → Chat）要通过**共同 owner** 协调：handler 挂在共同祖先上，再用 Entity API 更新兄弟；只把 handler 挂在一个兄弟上，另一个兄弟派发的 Action **到不了** | action §通过共同 owner 协调并列区域 | — |
| `KeyBinding::new(keys, action, context)`：跨平台主修饰键用 **`secondary`**（macOS=Command，其他=Control）；`cmd`/`super`/`win` 都指平台修饰键（**Windows 上是 Win 键，不是 Ctrl**）。**按键串或 context 无法解析会 panic** —— 静态定义要可检查，**用户输入必须先用可返回错误的 `KeyBinding::load(...)`** | keybinding §按键字符串的写法、§用户 Keymap | 本项目键位全是硬编码 `KeyBinding::new`；**尚无用户 keymap 文件**（快捷键设置页就是 `U10`/§2.2 ① 的缺口），所以 panic 风险目前不暴露 |
| **优先级**：先按 context 在 Focus 路径上的**深度**排（内层优先），同深度按**后注册优先**；⚠️ **`None` context 被视为最深**，所以它**不是**低优先级 fallback —— 想让 workspace 快捷键让位给内层控件，就必须显式写 workspace context | keybinding §理解优先级 | 本项目多处用 `None`（`editor/src/lib.rs`、`git/src/lib.rs`、`settings/dialog.rs`、`workspace.rs`）→ **加内层快捷键时要检查是否被 `None` 抢先** |
| **先 `cx.bind_keys(...)` 再 `cx.set_menus(...)`**：native menu 在创建时读取当前 keymap 并固化显示；之后改 binding 必须**再调一次 `cx.set_menus`** 重建菜单 | keybinding §让菜单和快捷键显示保持同步、coding §事件 Action 与焦点 | ✅ **不修**（`D20` 收口）：该条文的 **native-menu 前提在本项目不存在** —— 全仓**零处 `cx.set_menus`** 调用（只有注释里被否掉的 `AppMenuBar` / `NativeMenu`，理由见 `menu_bar.rs:31-44`：Windows 无 `SetMenu`，做不了常驻栏）。菜单栏自绘、候选面板走 `PopupMenu`，快捷键胶囊在**渲染期**从 keymap 反查（`gpui-component-0.7.0/src/menu/popup_menu.rs:1301 → 1119-1144`，`Kbd::binding_for_action_in`），调用那刻不固化 ⇒ "先 bind_keys 再 set_menus"的顺序约束**不适用**，走的正是 `keybinding.md:175` 给"窗口内菜单"的那一支。六处 `bind_keys` 全在窗口与菜单首帧之前，一律按**符号**记（原行号串实测**多不指向所声称的内容**，已改；只有 `app/src/main.rs:1030/1051/1059/1070`、`settings/dialog.rs:101`、`git/lib.rs:222` 这几处实测对得上）：`app/src/main.rs` 的 `lithe_gpui_settings::install_actions`（登记 `ctrl-,`）→ `lithe_gpui_workbench::install_open_project_action` → `lithe_gpui_workbench::menu_bar::install_key_actions`（经 `install_menu_key` 6 条）→ `cx.open_window` 内的 `ShellWorkspace::new`：`lithe_gpui_editor::install_actions`、`lithe_gpui_git::install_actions`（登记 `ctrl-enter`）、`crate::command_palette::install_actions`；`MenuBar::new` 与首帧 `MenuBar::render_bar` 都在其后，下拉面板由 `MenuBar::build_popup` 每次展开现建。「之后改 binding 需再调 `set_menus`」在本项目**当前不暴露**：无运行期改 keybinding 路径（`clear_key_bindings()` 0 处、无用户 keymap 文件），没有需要重建的缓存 |
| `cx.clear_key_bindings()` 会清掉**包括组件默认**的整个 keymap，重载时要按序重新注册全部；**不要硬编码 `⌘A`/`Ctrl+A`** 显示，应从 Action 反查（`window.highest_precedence_binding_for_action_in(&action, &focus_handle)` / `Kbd::binding_for_action_in`） | keybinding §用户 Keymap、§从 Action 反查快捷键 | 与 §1.3 的 `Kbd::binding_for_action` 那条一致 |
| 反查基于**上一帧已渲染**的内容（当前帧首次绘制的 focus handle 可能查不到）；`window.is_action_available_in(&action, &focus_handle)` 才能判断路径上**有没有** handler | keybinding §从 Action 反查快捷键 | — |
| `Global` 是**标记 trait**，每个 `App` 按类型只存一份；`set_global` **替换**而不是合并字段。适用于"多个功能/窗口共享的设置或服务"，**不适合**放窗口独立的 Focus/选择状态或庞大的业务数据集合 | global §开头、§选择状态的所有者 | 与 §1.4 I 组的状态归属一致 |
| `cx.global::<T>()` 在未安装时会 **panic**（可选值用 `try_global`）；`global_mut` / `update_global` / `default_global` / `remove_global` **即使没真正改动也会安排一次通知** —— 想省成本要在调用**之前**用 `global::<T>()` 比较 | global §读取与修改全局值 | — |
| 在 `render` 中读取 Global **不会**自动建立依赖：要 `cx.observe_global::<T>(...)` 并把 `Subscription` 存进 View；否则**只重绘父级不会刷新缓存子级** | global §动手构建双窗口示例、view-cache §GPUI 何时复用 | 本项目的主题/设置监听正是这个模式 |
| ⚠️ **通过 Global 内部的 `Arc` / 锁 / 原子变量改数据，不会触发 Global 观察者** —— GPUI 只感知通过 Global API 的修改 | global §常见错误 | — |
| 不要在 `render` 里修改 Global | global §常见错误 | 与 §1.4 I 组"render 里别做副作用"一致 |

**O. 异步任务（`task.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| **Task 的核心性质是所有权**：handle 被 drop → 未完成的工作取消。只有**保存、等待或显式 `.detach()`** 才会继续跑。⚠️ **单独一句 `cx.spawn(...);` 会在语句结束就 drop handle，任务甚至可能没执行就被取消** | task §开头、§保存等待或 detach | — |
| 四种启动方式决定**跑在哪**：`Context::spawn`（前台 + `WeakEntity`）/ `spawn_in`（前台 + Window）/ `App::spawn`（前台，无 Entity）/ `background_spawn`（后台执行器，**不提供 GPUI context，不能直接改 Entity 状态**） | task §开头（对照表） | 与 §1.4 J 组一致 |
| **前台任务不要求 `Send` 但仍要 `'static`**：把自有输入**移入** future，不要借用 `self`。`background_spawn` 要求 future 与输出都 `Send + 'static` | task §任务如何在线程之间切换 | — |
| ⚠️ **关键的一条（本项目直接相关）**：前台任务在 `await` 未就绪时确实会让出 UI 线程，**但"首次返回 `Pending` 之前的同步代码仍在 UI 线程执行"**；一次 poll 里的大段 CPU 仍会占住它。指南给的做法：**拆成有界步骤、放到后台、或改成真正的异步等待**；并明确禁止 `while !ready {}` 这类忙等 | task §将耗时工作移出 UI 线程、§处理完成失败和取消、§检查与排查任务 | 🔴 这正是 `D10` 的规范依据：`RunSession::start` 在 UI 线程闭包内**同步**跑完所有 `preLaunchSteps`（含 `javac`，上限 300s）——按指南应把前置步骤放后台任务 + 有界等待 |
| 启动任务的位置：**具名方法、事件处理器或生命周期钩子**；**不要在 `render` 中无条件启动**（每次 render 都会再启一份）。启动前先取出输入值，避免 `self`/`cx` 的借用跨过 `await` | task §从 owner 启动任务 | 与 §1.4 I 组"render 里别做副作用"一致 |
| **把 `Task` 存进 `Option<Task<_>>` 并整体替换** = 取消上一次（适合搜索 / 刷新 / debounce / 流）；连续 spawn 并 detach 会让多个独立任务同时在跑（**`.detach()` 本身不等于泄漏**，但持续工作应交 owner 保存） | task §保存等待或 detach、§从 owner 启动任务 | 本项目 `session.rs` 等有 `Option<Task<…>>` 模式 |
| **所有权的代价**：owner 保存 Task，而 Task 的 future 又**强持有同一个 Entity** → 保留环。想"owner 释放即取消"就用 `Context<T>::spawn` 已给的 `WeakEntity`，或捕获 `cx.weak_entity()` | task §保存等待或 detach | 与 §1.4 I 组"反向引用用 WeakEntity"一致 |
| 取消是**协作式**：drop handle 只阻止后续 poll，**不能撤回已发生的外部副作用，也不能中断正在执行的阻塞函数**。迟到结果要靠**请求 ID / 修订号**在应用前拒绝 | task §处理完成失败和取消 | 本项目的代次守卫（`request_serial`、`revision`）正是这个模式 |
| 用户可见操作要保存明确的 **idle / loading / loaded / failed** 状态：启动前置 loading 并 notify；`await` 后**先拒绝过期 ID** 再分别处理 `Ok`/`Err`；清除 loading 后在**这次完整更新里 notify 一次**。**只在日志里记错误、或在 `.detach()` 后丢弃错误，会让用户永远停在 loading** | task §处理完成失败和取消 | 与 §2.2 ⑩ 的运行/终端状态设计相关 |
| `.detach()` 与 `Subscription::detach()` **不同**：前者让工作独立跑完；后者让 callback 持续订阅直到源 Entity 释放 | task §保存等待或 detach | — |

**P. 自定义 `Element` 与绘制（`element.md`；自绘前必读）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| 三阶段职责，判据是一句话：**"这项计算会改变 Element 请求的尺寸吗？"** → 会，放 `request_layout`（或其测量闭包，用 `window.request_measured_layout`）；只依赖最终 bounds → `prepaint`；只改像素或本帧 handler → `paint` | element §如何判断代码放在哪个阶段 | 本项目自绘很少（任务表里 §2.2 ⑤ 的 diff/minimap、⑨ 的 VT 渲染要做） |
| ⚠️ **`request_layout` 里不要 shaping 文字或三角化路径**（除非固有尺寸测量确实需要）；`prepaint` 算一次几何并用 `PrepaintState` 交给 `paint`，**不要在 `paint` 重算** | element §性能取决于各阶段的职责 | — |
| **三类结构完全不同，别互相期待**：布局树（Taffy，`request_layout` 提交）· 派发树（`prepaint` 建 dispatch node，`paint` 激活）· Scene（绘制命令）。**`paint_path` 只改 Scene，不建 hitbox 也不建派发节点**；反过来 mask 类 Element 可以建输入区域却什么都不画 | element §布局树派发树与 Scene 是不同的结构 | — |
| **`HitboxBehavior`**：`Normal` 参与命中但不遮挡后方；`BlockMouse` 遮挡后方鼠标与滚动；`BlockMouseExceptScroll` 保留后方滚动。**画形状不会自动建 hitbox，插 hitbox 也不会自动画形状** | element §从一个完整的底层 Element 入手 | — |
| `window.on_mouse_event` 注册的监听器进**窗口列表**（不是挂在当前 dispatch node）：**捕获阶段按注册顺序、冒泡阶段反向**；handler 要自己检查 hitbox | element §让输入区域可见并响应按下事件 | — |
| `canvas(prepaint, paint)` **本身就是一个 `Element`**：它的 prepaint 回调返回的临时值只在当前帧存在（不是 HTML canvas，也不是跨帧绘图表面）。它**没有子元素、hitbox、Focus 跟踪或稳定 ID** —— 这些职责要协同时就该实现 `Element`，或在 canvas 外组合标准交互元素 | element §canvas 用两个回调进入绘制阶段 | §2.2 ⑤⑦ 的画线/画路径类需求可用它，但别指望它带交互 |
| 组合子元素时必须**三个阶段都做到**：把子布局 ID 传给 `window.request_layout(style, [child_layout], cx)`，`prepaint` 调 `child.prepaint(...)`，`paint` 调 `child.paint(...)`；**位置由 Taffy 算，不要在 `paint` 里手动偏移** | element §在自定义 Element 中组合一个子元素 | — |
| `InteractiveElement`（提供 hitbox / 鼠标 / Focus / Key Context / Action）与 `StatefulInteractiveElement`（标记 trait + 依赖 ID 的 role、无障碍名、tooltip、click）是**两套能力**；**自定义 `Element` 不会自动获得任何一个**。要让自定义组件暴露同一套 fluent 方法，应把交互状态**委托**给内部基础元素（Kit 的 `Radio` 就是这么做的） | element §Identity 与交互 | 与 `D21`/无障碍相关 |
| 只有这些情况才实现 `Element`：需要文字 shaping + 画选区光标的编辑器/输入框；自定义几何的图表或 canvas；自定义布局算法；需要直接控制布局/hitbox/绘制的高性能原语。**"有自定义绘图"不等于必须实现 `Element`**（Plot 的曲线由上层 canvas 画，自己不必是 Element） | element §什么时候实现 Element | 与 §1.4 L 组"RenderOnce 优先"一致 |
| 昂贵绘制前要**裁剪与剔除**：TextView / Input 都按最终 bounds 算可见内容，而不是盲画整份文档 | element §性能取决于各阶段的职责 | 与 §2.2 ⑥ 提交表未虚拟化同类要求 |

**Q. 绘制原语与"自绘该做到哪一层"（`paint.md` + 前述各篇汇总）**

**先选层级**（四档，递进，别越级）：

| 需求 | 用什么 | 依据 |
| --- | --- | --- |
| **一处装饰**、只需在已有 Entity 上画点东西 | `canvas(prepaint, paint)` | paint §GPUI Kit 的三种状态归属 |
| 每帧重建的值需要**跨帧缓存**几何 | 稳定 `ElementId` + `window.use_keyed_state`（Plot 的 `PathCaches` 模式） | paint §Plot |
| **文字 shaping / 布局 / 命中 / 输入 / 绘制必须共享同一份快照** | 实现完整 `Element` | paint §Input、element §什么时候实现 Element |
| 只是**组合已有 UI** | `RenderOnce` / `Render` + 现有 Element | render-once §选择合适的层级 |

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| 矩形与边框用 **`window.paint_quad`**；自由曲线与不规则形状用 **`window.paint_path`**；普通图文优先用现有 Element | paint §开头 | — |
| **`canvas` 的两个回调都是 `FnOnce`**：prepaint 回调返回的自有值交给 paint；**不要把 `Window` / `App` 引用留到下一帧**；绘图数据要跨 render 就放 Entity 或 keyed window state | paint §从一个三角形开始 | — |
| **坐标交接是这类代码最容易错的地方**：`bounds` 是**窗口坐标**，canvas 的 origin 通常不是 `(0,0)`；**输入时减去 origin 存局部坐标，绘制时加上当前 origin** —— 窗口改动或面板移动后笔迹才不会偏移 | paint §让绘制响应点击、§4. 用窗口坐标构建 Path | 与 §2.2 ⑤ minimap / ⑨ VT 渲染相关 |
| `on_prepaint` 里保存 bounds **不要顺手 `cx.notify()`** —— 它只记录后续输入要用的几何，不该每帧再请求一次渲染 | paint §1. 为 canvas 分配空间并接收输入 | 与 §1.4 I 组"render 里别无条件 notify"同源 |
| `PathBuilder::fill()` 用于封闭区域、`PathBuilder::stroke(width)` 用于线条；`build()` **返回 `Result`**（几何输入不保证总能成功，要处理而不是 panic）；虚线用 `.dash_array(&[px(4.), px(2.)])`；`translate`/`scale`/`rotate` 可在三角化前变换 | paint §构建 Path | — |
| ⚠️ **从 SVG 迁移的两个坑**：① `PathBuilder` **没有 `H`/`V` 缩写，也不能直接解析 SVG 的 `d` 字符串**（要自己写解析器逐段喂）；② `curve_to` / `cubic_bezier_to` **先传终点、再传控制点**（与 SVG 的书写顺序相反）。另：`arc_to` 的 `x_rotation` 参数类型是 `Pixels`，但数值按角度解释 | paint §从 SVG Path 迁移 | §2.2 ⑤（diff 连接带、minimap）会用到 |
| 已有 SVG 文件且不需要 `Path<Pixels>` 时，直接 `svg().path("icons/x.svg")` 渲染资源，别手工转路径 | paint §从 SVG Path 迁移 | 与 §1.4 H 组的 `img` vs `svg` 一致 |
| **几何没变就不要重复三角化**：点和尺寸没变时复用已构建的 `Path`（可以换颜色画）；但**若路径存的是窗口绝对坐标，位置一变就必须更新缓存** | paint §哪个阶段完成什么、§Plot | §2.2 ⑤ 的 diff 滚动场景正是此坑 |
| `window.with_content_mask` 限制绘制区域；**绘制 / hitbox / 无障碍三者互相独立**（画出图形不会自动可点击，也不会自动被辅助技术读到） | paint §哪个阶段完成什么 | 与 §1.4 Q 组一致 |
| 简单矩形优先 `paint_quad` 或带样式的 `div()`，不要为它写自定义 Element | paint §哪个阶段完成什么 | — |

**R. 事件与订阅（`event.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| **Action 向内传意图、Event 向外报事实**：Action 经 Focus / Key Context / KeyBinding / Dispatch Path 路由；Event **不经过**这些，只是 Entity 之间的类型化通知 | event §Action 进入 Event 返回 | — |
| **`emit` 不会请求重绘**：`cx.emit(...)` 把事件放进 effect 队列，当前更新先结束，订阅回调**不是**一次直接函数调用；**发事件的 Entity 自己显示了变化，还要在它里面 `cx.notify()`** | event §从零完成一次 Event 传递 | — |
| 命名要能区分：Event 用**事实**（`MessageSent` / `Saved` / `Dismissed`），Action 用**命令**（`SendMessage` / `Save`） | event §从零完成一次 Event 传递 | 本项目 `actions!` 命名符合 |
| 订阅端：`cx.subscribe(&entity, …)` 指定具体 Entity，回调里拿到事件；订阅只对**传给它的那个 Entity** 生效，**列表里新增 object 不会自动进入订阅**（要按 ID 分派） | event §让 owner 按 ID 分派事件 | — |
| ⚠️ **`Subscription` 的默认行为是"源 Entity 释放就结束"** —— 这不是泄漏，而是自动清理（与 `cx.observe` 同源）。要让它比源活得更久才用 `.detach()`，并要自己保证回调捕获的资源仍然有效 | event §让 owner 按 ID 分派事件、§常见错误 | 与 §1.4 I 组一致 |
| `on_focus` / `on_blur` / `on_focus_in` / `on_focus_out` 是 **Entity 生命周期级的两个方向**：前两个只跟随**该 Entity 内部**注册在任意元素上的句柄，后两个再看**其它 Entity** 的 handle（**没有对应祖先/后代关系的 Entity 之间可以嵌套**）。真正用于 Focus 路径路由的是 `track_focus`，生命周期监听不能替代"可聚焦" | event §焦点事件跨越 Entity 边界 | 与 §1.4 N 组一致 |
| **输入事件（鼠标/键盘）走另一套**：capture（根→目标）后 bubble（目标→祖先），**元素不能预先声明捕获**，要靠 `capture_action` / `on_key_down` 在祖先元素上拦截 | event §捕获与冒泡 | — |
| `cx.stop_propagation()` 停**后续 listener**（bubble 不再交给后方 surface、键盘不再交给祖先；在 capture 阶段调用还会阻止剩余 capture 与 bubble）；`window.prevent_default()` 只标记**默认行为**（如 mouse down 时父元素获得 Focus）。**两者互不替代**，GPUI 每次派发开始时都会重置这两个标志 | event §捕获与冒泡、§阻止输入默认行为 | 本项目 `stop_propagation` 27 处（都是有意的单点拦截）；`prevent_default` 与 `focus_trap` 一起用于自建浮层 |
| ⚠️ **Action 与输入事件的默认策略相反**：Action handler **默认停止向上冒泡**，要上层继续尝试才 `cx.propagate()`；输入事件则是监听器都要自己判断 | event §阻止输入默认行为 | 与 §1.4 O 组一致 |
| **handler 没生效的排查顺序**：事件阶段 → Focus path → content mask → hitbox 行为 → z 顺序。**注册成功也可能收不到**（被前方元素遮挡，或事件走了另一条 Focus path） | event §排查事件监听器 | 与 §1.4 Q 组的 hitbox 三种行为配套 |

**S. 文字系统与 `text!`（`text-system.md` / `shared-string.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| **普通文案就用文本 child 或 `text!(...)`**；`text!(...)` 用**宏调用的源码位置**生成 ID，GPUI 可据此向无障碍树提供标签 —— 而普通 `.child("…")` **没有文本节点 ID**。同一位置生成多个标签要显式写 `text!(id = "activity-label", value)`，**同时显示的元素各自需要不同 ID** | text-system §优先使用文本元素 | 与 §1.4 D 组"仅图标控件必须有无障碍名称"配套 |
| ⚠️ **`TextRun::len` 以 UTF-8 字节计数**（不是字符数！），所有 run 合起来必须覆盖要绘制的文本；**只在合法 UTF-8 边界拆 run** | text-system §塑形单行文本 | 编辑器/诊断/补全里拼 `TextRun` 时注意 |
| ⚠️ **`shape_line` 只处理一行，不要传含 `\n` 的文本**；多行/软换行用 `shape_text(...) -> Result<SmallVec<[WrappedLine; 1]>>`。`line_clamp` 限制软换行边界，**但每个显式 `\n` 分隔的源行仍各返回一个 `WrappedLine`**，不保证总数 ≤ N | text-system §塑形单行文本、§多行换行 | — |
| ⚠️ **索引全程是"原始文本中的 UTF-8 字节位置"，不是 Unicode 字符数、也不是视觉列数**（`x_for_index` / `index_for_x` / `closest_index_for_x`）；处理指针要**先减去行原点**再查询。**测量、光标定位与绘制必须用同一塑形结果 + 同一字号/行高/对齐** | text-system §塑形单行文本、§排查文本不一致 | 与 §2.2 ④ 的"UTF-16→字符列换算"是**两套不同坐标**，别混 |
| ⚠️ **`width()` 是塑形后的 advance（笔位），不等于有色像素的外接矩形** —— 字形可能伸出 advance；`font_size` **也不单独决定行高**，垂直定位要结合该行的 ascent/descent 与选定行高。要裁剪框或命中区就按实际放置的行算 | text-system §塑形单行文本、§排查文本不一致 | — |
| **不要用字符数乘平均宽度估算宽度**：连字、字距、字体回退、emoji 都会改变字形数量与 advance | text-system §开头 | — |
| 阶段对应：固有文本尺寸/换行**决定申请尺寸** → 在**测量布局回调**里塑形；只是**在已定尺寸内定位** → `prepaint`；绘制 → `paint`（复用前面的塑形结果） | text-system §对齐 GPUI 渲染阶段 | 与 §1.4 Q 组的"会改变尺寸吗"同一判据 |
| **绘制文字不会建立 hitbox、键盘焦点或无障碍名称** —— 可交互的自定义文本元素要自己实现这些契约 | text-system §对齐 GPUI 渲染阶段 | — |
| **字体解析**：`resolve_font` 在字体族加载不上时走回退栈，**全失败会 panic**；`font_size` 之外，请求的 family 可能落到系统另一种可用字体。**打包字体要早于首帧**，之后再 `add_fonts` 会失效已缓存的字体解析与行布局，且已开始的布局可能仍用旧字体集合 → 要 `cx.refresh_windows()` 并重新测量 | text-system §字体解析与度量、§排查文本不一致 | ✅ 与 §1.4 F 组一致（本项目 `register_fonts` 早于开窗） |
| 缓存键**包含文本、已解析的字体 run、字号与宽度约束**：改文本/字体/features/字号/强制宽度/换行宽度可能要新布局；**只改颜色或背景属于装饰 run，与字形位置分开，未必需要重新塑形**。另有 `prewarm_fonts` 可在后台 executor 预热平台字体缓存 | text-system §缓存与性能边界 | — |
| 大型虚拟化编辑器/文档**只塑形可见内容**；`try_layout_line_by_hash` 可在不构造连续字符串的前提下探缓存，但调用方**必须保证相同 hash 对应完全相同文本**，并传 UTF-8 字节长度 | text-system §缓存与性能边界 | §2.2 ⑥ 提交表/④ 编辑器相关 |
| 自定义文本必须用**塑形后的行**测量（`ShapedLine::paint(origin, line_height, align, align_width, …)`；`WrappedLine::paint` 的对齐宽度参数是 `Bounds<Pixels>`，`ShapedLine::paint` 用的是 `Option<Pixels>`）；**run 有背景要单独调 `paint_background(...)` 并各自处理 `Result`** | text-system §对齐 GPUI 渲染阶段 | — |
| 要渲染带样式 run、选择与滚动的 Markdown / HTML，用上游 **`TextView`**（`TextView::markdown("article", src)`），不要自己写一套 | text-system §优先使用文本元素 | 与 §2.2 ④ 的 Markdown 预览缺口相关（上游已有能力可复用） |

**T. 动效（`animation.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| **三层按"变化中的值由谁持有"选**：GPUI `with_animation`（元素入场、循环提示、固定步骤；进度存在包装元素的 `ElementId` 下）· Base `motion`（目标会变、退出、关键帧、测量式展开；按稳定 key 存通道，活动中请帧）· Component `motion_tokens()`（跟随主题的样式化控件） | animation §开头 | 本项目动效很少 |
| **语义状态归应用所有**，动画只负责呈现：对话框开没开、选中哪个标签、滑块指向哪，都是应用状态；**无论动画两端还是关掉动效，结果都必须清楚** | animation §开头、§多窗口前的自问清单 | — |
| ⚠️ **先判断"新事件是否可能在动画结束前改变目标"**：会 → 用目标驱动的 Base `transition` / `spring`；不会（已挂载元素播一段固定过程）→ 用 `with_animation`。**连续点击时两者中断行为不同** | animation §从选中状态开始做动画、§中断时会怎样 | — |
| ⚠️ **ID 规则**：`with_animation` 的 ID 标识**包装元素**；**同 ID 再次 render 会继续已有播放，`Animation::new(...)` 每次重建不会重播**；要重新入场就把**应用维护的 generation 放进 ID**（如 `("saved-notice", generation)`）。Base 每个独立变化的值都要**自己的稳定通道 ID**，按业务对象+属性命名空间（`(element_id, "opacity")` / `(element_id, "height")`）；**共 ID 会覆盖 retained state，每帧换 ID 会失去连续性**；可重排列表用业务 ID 不用行号 | animation §GPUI 元素动画、§GPUI Base Motion | 与 §1.4 M 组"稳定 ID"同源 |
| ⚠️ **`with_animation` 不是"追逐可变目标"的 transition**：同 ID 下改回调捕获的终点，**原播放时钟仍继续**，且**不会以当前画面值作为新起点** —— 需要连续点击平滑反向就用 spring / Base transition | animation §中断时会怎样 | — |
| 弹簧采样要点：**挂载期间每次 render 都要继续采样该通道**（否则保留值不收敛）；`Spring` 的 `epsilon` **使用目标值自身的单位**（像素偏移通常要比归一化透明度更粗的容差）；指针直接拖动期间用 `Spring::with_travel(false)` 跟随，松开后恢复 travel | animation §每次 render 都采样弹簧、§GPUI Base Motion | — |
| **退出动画要用 `Presence`**：先采样再决定是否渲染（`sample.should_render()`），**不能写成 `if self.present`** —— 那会在第一次更新就卸载元素，退出动画无从绘制；退出中重新打开会**从当前采样值反向** | animation §退出完成后再卸载元素 | — |
| 其他 Base 原语：`Keyframes` / `Timing` / `animate_keyframes`（数值停靠点）· `Sequence`（连续独立步骤，**不会自动反向**）· `Stagger`（列表错峰，**不持有列表与 ID**）· `MotionReveal`（测量 child 后按进度裁剪高度，**本身不采样不推进**） | animation §GPUI Base Motion | — |
| ⚠️ **必须尊重减弱动态效果**：GPUI 侧一次性动画显示终点、循环动画显示起点，都不再请帧；Base 侧立即到达目标。`cx.reduce_motion()` 读取、`cx.set_reduce_motion(...)` 设置；`gpui_kit::init` 已初始化系统偏好处理（读 macOS/Windows，Linux portal 跟随），要重新读取用 `gpui_kit::base::apply_system_reduce_motion(cx)` | animation §减弱动态效果与帧请求 | — |
| **无限循环的活动状态必须有可理解的静态表示**（"停住的旋转图标不足以说明正在加载"）；自定义元素自己持有时钟就要检查 `reduce_motion`，且**不要只在 `render` 里无条件 `notify`/`refresh`/`request_animation_frame`** | animation §减弱动态效果与帧请求 | 与 §1.4 C/I 组一致 |
| 过渡中要**协调键盘焦点、命中区域与语义状态**；**绘制位置改变本身不会向辅助技术播报新状态** | animation §减弱动态效果与帧请求 | — |
| **主题化动效读 token**：`cx.theme().motion_tokens()` 给 `duration_instant/fast/normal/slow`、`easing_enter/exit/move`、`spring_control/move`、`distance_short/medium`——产品集中调策略，而不是散落魔法值 | animation §GPUI Component | 与 §1.2/§2.2 ⑰ 的 token 缺口同源，本项目目前没有自建动效 token |

**U. 帧率与性能观测（`fps.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| 上游有**独立 crate `gpui-fps`**，在窗口上叠一个 HUD：主读数 + 滚动帧耗时曲线 + 本进程 CPU/GPU/内存。父元素必须 `relative()`（HUD 绝对定位）；**每个窗口最多渲染一次** `fps_monitor(window, cx)`（重复调用会复用同一 monitor，HUD 画两遍） | fps §开头、§Immediate Retained 与 Hybrid | 💡 §2.2 ① 状态栏缺的"帧率"项**可以直接接它**，不必自研 |
| **`MAX FPS` 与 `FPS` 是两个不同问题**：前者是 `1 / FRAME` 平均（CPU 侧 draw 吞吐估算，能拿到刷新率就以其为上限），后者由近期 present 时间戳推算。**任一数字都不能单独证明"空闲时持续重绘"或"持续性能"** | fps §主读数 | — |
| ⚠️ **`FRAME` 只取 `draw_end - draw_start`** —— 它不含请求等待、平台提交、GPU 与合成器完成时间。要拿别的口径得自己埋点（`dirty_to_draw_duration()`、`PresentTiming`、`FrameTimingCollector`） | fps §从状态更新到画面显示 | 与 §1.3 的 DPI/截图口径一样属"测量口径"问题 |
| ⚠️ **闲置窗口也可能一直产生活动**：HUD 自身时钟、输入设备、平台合成器或应用代码都会；**"屏幕 120 Hz"不等于"应用每秒重建 120 次元素树"**（`120 Hz` ≈ 8.33 ms 帧预算，只是预算不是承诺） | fps §120 Hz 是帧预算不是刷新承诺 | — |
| 要声称"持续 120 FPS"，必须 release 构建、按**整个窗口**、带真实数据/滚动/动画测，并报预热后的帧耗时分布与 present 间隔；**`平均值很低也可能被 P95 慢帧打断**产生可见卡顿 | fps §120 Hz 是帧预算不是刷新承诺 | — |
| HUD 自己的开销：**每 500 ms 一帧**，且它**先告知采样器把这次绘制排除**在读数外（除非应用自己也要了这一帧）；**隐藏时零开销**（连续两个 tick 没被渲染就停时钟、停资源探针、放掉 trace）；**最初几帧不计入统计**（着色器/字形图集/图标都是冷的） | fps §HUD 自己的开销、§最初的几帧不计入统计 | 💡 本项目状态栏内存/帧率若自研，应照抄这个"排除自身帧"的模式，否则读数会被自己污染 |
| 各行口径：`INTERVAL`（present 间隔）· `FRAME`（`Window::draw` 耗时）· `P95` · `DROP`（draw 耗时**严格大于**预算的比例）· `INV`（合并进同一帧的失效次数，**明显大于 1 说明被要求重绘的频率超过它能画的**）· `GPU`/`CPU`/`MEM`（**各平台不是同一种计数器**：MEM 在 macOS 是 physical footprint、Windows 是 private usage、Linux 是常驻匿名内存） | fps §各行含义 | ⚠️ 与 §2.2 ① 的"内存格是硬编码假值"直接相关：真值口径要按平台选，别只报 RSS |
| `FRAME`/`P95`/`DROP`/`INV` 默认用**最近 120 个 draw 样本**（不是固定时长），`capacity()` 可调；`frame_budget()` 可钉自己的预算 | fps §各行含义 | — |

**V. 无障碍（`accessibility.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| ⚠️ **只有元素同时有 `ElementId` + 非空 role，GPUI 才把它纳入无障碍树**；**单独的 `.id(...)` 不是 role** —— 没有 role 的 `div` 只是布局容器；**只有 role 也不会让它自动获得焦点或响应操作**。`Role::GenericContainer` 会**被过滤掉** | accessibility §身份与 role | 与 §1.4 D 组"仅图标控件必须有无障碍名称"配套 |
| 一个有用的控件要有四件：**role**（是什么）· **名称**（用户辨认）· **状态**（值/勾选/选中/展开）· **操作**（辅助技术可请求的行为）。**稳定 ID 不是用户听到的名称** | accessibility §开头 | — |
| ⚠️ **相邻的可见文字不会自动成为输入框的名称**（"仅凭相邻位置不会自动建立无障碍关联"）：要用 `Input::aria_label(...)` 命名真正的输入框。**placeholder 会消失，不能很好替代持续可见的标签** | accessibility §1. 给接收输入的控件命名 | — |
| ⚠️ **纯图标按钮要显式命名**（`Button::accessibility_label(...)`）；**tooltip 是另一种提示，不能代替名称**；**不要假设自定义容器里的任意文字会自动成为它的无障碍名称** | accessibility §从语义控件开始 | 与 §1.4 D 组一致 |
| ⚠️ **`.on_click(...)` 已经声明无障碍 Click 激活并路由给点击 handler** —— **再给同一节点注册第二个 `AccessibleAction::Click` 会让命令执行两次** | accessibility §无障碍操作与键盘输入、§从零构建可交互的自定义控件 | — |
| 当前 GPUI `div` API **没有**通用的 `.aria_disabled(...)`、也**没有** `aria-labelledby` / `aria-describedby` / `aria-controls` —— 别按 `aria_` 前缀推断存在。`.aria_keyshortcuts(...)` **只负责向辅助技术说明**，实际按键仍须 `bind_keys` 注册 | accessibility §名称状态与关系、§检查失败时从哪里入手 | — |
| ⚠️ **`readonly(true)` 不能阻止无障碍写入路径**：当前 Input 只要未禁用就注册 `SetValue`（含只读模式，走程序化替换）。只读数据若必须受保护，要自己核实 | accessibility §测试无障碍契约 | — |
| 测试口径：`window.find(id)` 要在 **`window.render_frame(cx)` 之后**查询**已绘制**的元素；`ElementSnapshot` 只描述**一帧**，每次交互后必须重新查询；**`None` 表示属性不可用，不是 `false`**（`.disabled()` 只有在节点真的暴露 disabled 标志时才返回 `Some(true)` —— 测禁用行为要**尝试激活并确认结果未变**） | accessibility §测试无障碍契约 | 与 `test.md` 相关 |
| `.test_support()` 只在启用 `test-support` feature 时让测试工具找到**自定义 `div`**；**原始的 `Element` 实现找不到** —— 那种情况要用 `a11y_role()` / `write_a11y_info()`（只有 ID + role 同时存在时 GPUI 才会调它） | accessibility §从零构建可交互的自定义控件、§自定义 Element | 与 §1.4 Q 组相关 |
| ⚠️ **无头 snapshot 只能证明 GPUI 树的内容**：它**不检查**平台适配器树、屏幕阅读器实际播报、像素与焦点环可见性。**`Role::Status` 不保证变化被播报** —— 要到目标平台用辅助技术实测 | accessibility §测试无障碍契约、§一轮可执行的验收 | 与 §3 验收第 10 条（"在真实窗口验证"）一致 |
| 无障碍树**只在辅助技术启用时才构建**；"没有客户端连接时看不到树"不能用来判定方法失效 | accessibility §自定义 Element | — |
| `accessibility_id(...)` 是**另一个**供平台自动化用的作者标识（Windows → UIA `AutomationId`，macOS → `AXIdentifier`；Linux AT-SPI 取决于 adapter），**不能代替 `.id(...)` 也不能代替名称** | accessibility §身份与 role | — |
| `.track_focus(...)` 建立焦点路径**并**声明无障碍 Focus 操作；只给 role 或 `.focusable()` **不会**带来可用的键盘命令 | accessibility §名称状态与关系 | 与 §1.4 N 组一致 |
| Table 即使有可见 caption，也要在 **Table 根节点**设 `.accessibility_label(...)`（caption 容器不会自动命名） | accessibility §名称状态与关系 | §2.2 ⑥⑦ 的表格相关 |

**W. 测试层级与 UI 集成测试（`test.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| **四层测试**按要验证的行为选：纯数据转换/校验/状态机用普通 `#[test]`；Entity/Action/订阅/异步 Task 用 `#[gpui_kit::test]` + `TestAppContext`（按需建窗口）；**UI 集成测试**在无头窗口渲染真实视图、派发真实事件、检查控件状态与布局；像素检查用**独立离屏渲染器** | test §开头 | ⚠️ **本项目 gpui 侧只有第一层**：`#[cfg(test)]` 模块 73 处、`mod tests` 71 处，但 **`gpui_kit::test` 0 处、`test_support` 0 处、`#[gpui_kit::test]` 0 处**，且注释多处写明"本 crate 拿不到 `TestAppContext` / `#[gpui::test]`，起不了窗口" |
| ⚠️ **`use gpui_kit::*;` 会把 GPUI 的 `test` 宏也引进来，可能遮蔽 Rust 原生的 `#[test]`** —— 测试模块要**显式导入**用到的类型 | test §开头 | — |
| 可用操作（`TestWindowExt`）：`window.find/try_find` · `click` · `click_at(id, offset)` · `right_click` · `double_click` · `hover` · `scroll` · `drag_to(from, to)` · `drag(from, to)` · `press("backspace")` · `input("text")`；**作用域**支持 `window.within("toolbar").click("save")` 并可嵌套（父级**不必**被观察 —— 它的 ID 已在被观察子元素的路径里） | test §操作与断言 | 💡 这正好能覆盖 §2.2 ① 的"**底部工具窗拖拽热区是死区**"（`drag_to`）、⑬ 的浮层焦点、⑤ 的自绘命中 |
| ⚠️ **`ElementSnapshot` 只描述某一次完成的绘制**：交互后**必须重新查询**；`focused()` / `disabled()` / `checked()` 等返回 `Option<bool>`，**`None` 表示"取不到"，不是 `false`** | test §操作与断言 | — |
| ⚠️ **不要用 `assert!(button.disabled().is_none())` 证明"按钮可用"** —— 要**执行操作并断言它应该产生的结果**（如 `window.click("save"); assert_eq!(find("status").label(), Some("Saved: Ada"))`）。`disabled()` 只在原生节点暴露标志时才 `Some(true)` | test §选择稳定的测试目标 | — |
| ⚠️ **`.test_support()` 与 `.track_focus()` 的顺序**：先 `.test_support()` 再 `.track_focus(&handle)`，让包装器观察到实际焦点绑定；顺序错了 `focused()` 会**报错并给修复提示**（Debug 输出 `focused: <binding missed>`）。**隐式 `.focusable()` 句柄读不出来**，要用显式句柄 | test §选择稳定的测试目标 | 与 §1.4 N 组的焦点四操作配套 |
| ⚠️ **作用域内的键盘操作不会移动 Focus**：必须先有一个**已观察的焦点绑定**在该作用域里，否则派发前就报错；`input` 会在**每个字符前**检查，所以 handler 把焦点移出作用域后，剩余文字不会打到别的控件 | test §操作与断言 | — |
| **查询前要完成一帧**：首次查询、外部直接改状态/焦点、调整尺寸之后都调 `window.render_frame(cx)`；用 `TestAppContext::update_window`（**带类型的 `WindowHandle::update` 已经借用了根 entity，不能在同一个回调里重绘**） | test §查询前完成一帧 | — |
| **异步/延迟**要用 async `#[gpui_kit::test]` + `cx.wait_for(handle, Duration, 条件).await`（每 10ms 刷新并检查、超时报错列出注册路径）；**它是有界条件等待，不模拟 OS 事件循环或网络**。GPUI 的 `dispatch_action` **会排队** —— 读它写的值之前要先 `cx.run_until_parked()` 或离开 `update_window` | test §查询前完成一帧、§覆盖范围与失败排查 | 与 `write-stable-tests` skill 的"有界等待"要求一致 |
| ⚠️ **测试里要同时断言"界面状态"和"业务结果"**：只断言 label 会让"只改 label 的错误"通过；反之只断言模型也不够 | test §一个完整测试 | — |
| **Dialog / Sheet / Notification 的测试要求窗口根是 `Root`**；`Root` 会把这三类浮层渲染在应用内容之上（**含缓存视图**），**无需手动挂载** | test §覆盖范围与失败排查 | 与 §4.1 M3 一致 |
| **像素级验证另有一层**：`HeadlessAppContext::with_platform` + `Window::render_to_image` / `capture_screenshot`；⚠️ **当前锁定版本的平台 crate 只在 macOS 提供 Metal 离屏渲染器**，该 target 设了 `test = false` 要显式 `--test rendering`，且 `harness = false`（AppKit 必须在主线程初始化，普通 Rust 测试即使 `--test-threads=1` 也在工作线程） | test §独立验证绘制结果 | — |
| 上游已有的可参考回归契约（**都驱动真实输入**）：`input.rs` / `input_focus.rs`（编辑、剪贴板、选区、历史、Unicode、多行视口、查找替换、补全确认）· `search.rs`（Command 禁用项跳过、循环导航、中文关键词、空结果、两阶段 Escape）· `disclosure.rs`（Accordion/Stepper/Slider）· `collections.rs`（Tree/DataTable 键盘与滚轮虚拟滚动）· `date_picker.rs` · `overlays.rs`（**Dialog 校验→作用域 Input→保存→Notification、悬停显关闭按钮、自动关闭计时、Escape 与焦点恢复、表面边界**）· `menu.rs` · `dock.rs`（**Tab 选择与重排、跨分组拖放、放大和恢复分割**） | test §覆盖范围与失败排查 | 💡 我们缺的那几块（浮层焦点、菜单、Dock 拖拽、虚拟滚动）**上游都已有测试写法可抄** |
| 可见性判断**结合几何、视口与内容裁剪、目标计算样式，但不判断像素遮挡**；覆盖层仍会拦截点击 | test §覆盖范围与失败排查 | — |

**X. `SharedString` 与多窗口（`shared-string.md` / `multi-window.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| **UI 文本默认用 `SharedString`**（自有 + 不可变，跨 render / 组件边界 / task 闭包持有）；**临时读取借用 `&str`**。组件边界收 `impl Into<SharedString>` | shared-string §为何 UI 文本优先用它而不是 String | — |
| ⚠️ **"容易 clone" ≠ "零成本"**：clone 内联文本会复制少量字节、clone 堆上长文本才共享字节并增加引用计数；**`format!(...).into()` 每次仍构造新文本**。它**不是 `Cow` 的别名**，也没有写时复制 | shared-string §微小成本会累积、§与 Cow 的关系 | — |
| 在闭包里要长期用一份文本：**在构建闭包前 clone 一次**存进局部变量，**不要在每次 render 里把 `as_str()` 重新转成新值** | shared-string §避免退化点 | — |
| 多窗口：**每个窗口各有独立的 `Window` / 焦点 / 输入派发 / 几何 / `Root`**；应用数据可以共享。**`init(cx)` 只调一次**；每次 `open_window` 都会为内容 view 新建一层**独立** `Root`，`builder` **返回内容 Entity、不要再包 `Root`** | multi-window §基于同一份 model 打开两个窗口 | 与 §1.4 C/G 组一致；本项目现在只有一个窗口（`U10` 要做多窗口） |
| 状态划分判据：**共享的那份在循环外 `cx.new` 一次**，**每窗口各自的 View 在循环内创建**。clone `Entity<T>` handle 不复制 `T`；**两个 observer 观察同一 model，但各自归属一个窗口的内容 view**（所以每个窗口都要存自己的 `Subscription`，否则另一个窗口不更新） | multi-window §基于同一份 model 打开两个窗口 | — |
| ⚠️ **窗口相关操作要保存并使用该窗口的 `AnyWindowHandle`，不要依赖 `cx.windows()` 顺序**；关闭后 `handle.update(...)` 可能失败，要处理 | multi-window §排查示例问题 | 与 §1.4 G 组一致 |
| "关闭一个窗口就连带退出整个应用"要**检查应用的"最后一个窗口退出策略"与 `on_window_closed`** —— 这不是自动行为 | multi-window §排查示例问题 | `U10` 会直接碰到 |

**Y. 图片：`img()` vs `svg()`（`image.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| ⚠️ **`img("...")` 的参数不是文件路径**：字符串**能解析成 URL 就当 URL**（走 `HttpClient`），否则当作**已注册 `AssetSource` 的键做精确查找** —— `img("images/a.png")` **不会**从工作目录读文件。**不存在也没有 fallback，就是空白** | image §图片来源、§常见问题 | ✅ 本项目正是 `img("icons/idea/...")` 走资源键；`assets.rs` 那条历史回归（旧探针路径 `icons/settings.svg` 打不到 `MISSING`）就是这个语义造成的静默空白 |
| 磁盘文件要传 `&Path` / `PathBuf`（这样既不会被当 URL，也不会被当资源键） | image §图片来源 | — |
| ⚠️ **原生平台上默认 `HttpClient` 让所有请求失败** —— 要 `cx.set_http_client(...)` / `Application::with_http_client(...)` 装了才能加载 URL 图片 | image §图片来源、§常见问题 | 本项目只画内嵌资源，不受影响 |
| `with_loading` 的替换**只在"开始 200ms 后仍未完成"时出现**（GPUI 在**元素状态**里记时间，所以**只有带 `.id(...)` 的图片才会显示占位**）；`with_fallback` 覆盖"资源键/文件不存在、网络错误、非 2xx、解码失败"；**两者都不重试**，要重试得换来源或先 `ImageSource::remove_asset(cx)` 清掉**被缓存的失败结果** | image §加载解码与失败 | 本项目 `with_loading` / `with_fallback` **均未使用**；对纯内嵌资源可以接受，但**索引/路径漂移就是静默空白**，见 `D27` |
| 格式按**字节内容**判断、不看扩展名；非已知位图格式的字节**按 SVG 解析并按固有尺寸 2× 光栅化**（所以 `img()` 画 SVG 正常缩放清晰、放很大才糊） | image §加载解码与失败 | 与本项目"彩色 expui 用 `img()`"一致 |
| ⚠️ **GIF / WebP 动图只有带 `.id(...)` 才播放**（当前帧存在元素状态里）；窗口活动时播，系统要求减少动效时停 | image §加载解码与失败 | — |
| 某维度为 `auto` 时**解码后**才补尺寸：另一维是绝对长度就按比例算，否则用图片自身尺寸；**解码前无从测量，布局会跳** → 用明确尺寸，或宽度 + `.aspect_ratio(...)` 预留位置 | image §尺寸与适配 | 本项目图标都给了 `side` 尺寸（`idea_icon_image(icon, cx, side)`），所以不受影响 |
| `ObjectFit` 默认 **`Contain`**（完整显示、可能留白）；`Cover` 填满但裁边；`Fill` 拉伸变形；`ScaleDown` 同 `Contain` 但不放大；`None` 按原尺寸居中。`.rounded(...)` 让**绘制出的图片本身**带圆角；`.grayscale(true)` 灰度 | image §尺寸与适配 | — |
| ⚠️ **`svg()` 永远是单色**：它只光栅化 SVG 的**透明度通道**再用一种颜色填充，**SVG 自带的颜色被丢弃**；而且**必须在 `svg()` 元素本身设 `.text_color(...)`**（**从父元素继承不算**，没设就什么都不画）；**`svg()` 没有固有尺寸，不设尺寸会缩成零**；`.with_transformation(...)` 只影响绘制，**布局与点击区域不变** | image §svg() | ✅ 本项目 `img()` 24/35 处的分工正是为了**保留 expui 彩色**（`file_icon.rs` 的文档已写明"为什么是 `img()` 而不是 `svg()`"），只有本来就是单色的 Lucide 走 `svg()` |
| 选型：**照片 / 头像 / Logo / 插画 / 多色 SVG → `img()`**；**跟随主题或状态变色的图标与符号 → `svg()`**。`.text_color(...)` 改不了 `img()` 的颜色，`svg()` 也保不住多色 Logo | image §选择 img 还是 svg | 同上 |
| 解码后的图片有缓存：默认按 `Resource` 键**全 App 共用**（一个条目供所有窗口/视图，直到 `ImageSource::remove_asset(cx)`；**失败结果也被缓存**）；`image_cache(...)` / `retain_all("preview")` 可限定作用域；自定义 `ImageCache` 时淘汰要 `cx.drop_image(image, Some(window))` 释放 GPU 纹理，**容量必须大于同屏图片数**否则每帧重载 | image §解码后图片的缓存 | 本项目未自定义缓存 |
| GPUI 的内存图片缓存**不管 HTTP 语义**：进程退出即失、**不理会 `Cache-Control` / `ETag` / `Last-Modified`**。要跨视图跨启动复用响应，就得在 **HTTP 层**装一个遵守缓存规则的 `HttpClient`（只缓存无请求体的 GET、按共享缓存处理 `no-store`/`private`/带 `Authorization` 的响应、用 `If-None-Match`/`If-Modified-Since` 重新验证、按调用方的 `RedirectPolicy` 分开缓存、限制内存与磁盘、**先授权再请求**）；`http-cache-reqwest` 是现成实现 | image §远程图片的 HTTP 缓存 | 本项目暂无远程图片需求；一旦加（如插画/头像）就要按此做 |

**Z. 平台集成：通知、原生扩展、打包与自动更新（`system-notification.md` / `native-extension.md` / `packaging.md` / `installation.md` / `auto-update.md`）**

| 条文 | 出处 | ⚑ 本项目现状 |
| --- | --- | --- |
| 通知选型：**用户正在看结果** → 应用内 toast（`window.push_notification(Notification::info(..), cx)`）；**用户可能已切走** → `.system()`；两者都要 → `.in_app_and_system()`。**重要状态必须留在应用内**（系统可能禁用/抑制/延迟通知），**不能只靠一条短暂系统通知承载错误** | system-notification §开头 | 本项目未用系统通知 |
| ⚠️ **启动时要用 `cx.set_app_identity("com.example.app", "App")` 设稳定标识**（**未打包的 Windows 应用需要**；Linux 用显示名；**GPUI 测试平台也要求**），且要与安装包的 bundle identifier 一致。`gpui_kit::init(cx)` 之后、开窗之前 | system-notification §初始化并设置应用标识、packaging §先构建并列出运行时文件 | ⚠️ 本项目**没调 `set_app_identity`**。当前不发系统通知所以不暴露；一旦要做通知/文件关联/URL scheme 就得先补（且要与打包标识配套） |
| ⚠️ **`gpui_kit::init(cx)` 会安装 GPUI Kit 自己的系统通知响应 handler，而 `App::on_system_notification_response` 全局只有一个 —— 应用再注册会替换它**，替换后组件通知的点击就收不到；GPUI Kit **没有**公开接口把底层 handler 串到组件 handler 上。所以必须**二选一**决定响应所有权 | system-notification §保持单一响应 handler | — |
| 平台投递条件不同：**macOS 必须从真实 `.app` 运行**（`cargo run` 不投递）；Windows 要早设标识；**Linux 需要 D-Bus + XDG 通知守护进程，且当前适配器既不按 tag 替换也不支持撤回** | system-notification §初始化并设置应用标识 | — |
| 组件路径（`window.push_notification(...)` + `WindowExt`）：把 ID 映射成带命名空间的系统 tag、**记住发送窗口**，点击后撤回通知 → 激活应用 → 激活原窗口 → 关掉同一 toast → 调 `on_click`。⚠️ **应用重启或路由记录被清理后，点击只激活应用、不再调 `on_click`**；回调要求原窗口仍存活 | system-notification §使用 GPUI Kit 的窗口集成 | — |
| ⚠️ **`.system()` 不创建应用内 toast（所以不调 `on_close`）**；`.in_app_and_system()` 两者都建，但 **toast 自动超时不会撤回系统副本**；显式 `window.remove_notification::<T>(cx)` / `clear_notifications(cx)` 才会请求撤回该窗口拥有的系统通知 | system-notification §使用 GPUI Kit 的窗口集成 | — |
| 原生扩展两条路径：**`NativeMenu`**（弹出菜单需要超出小窗口边界或遵循系统外观时；Linux 是 GPUI 绘制的 `PopupMenu` 回退、受窗口边界裁剪）与 **`gpui-wry` 嵌入式 WebView**（实验性，**仅 macOS/Windows**，Linux 无受支持路径） | native-extension §开头、§平台表 | 与 §2.2 的菜单栏实现相关 |
| ⚠️ **两个 `window_handle` 名字相近、用途不同**：`Window::window_handle(window)` → GPUI 的 `AnyWindowHandle`（供之后更新**同一个** GPUI 窗口）；`HasWindowHandle::window_handle(window)` → `raw-window-handle` 的原生 handle（AppKit `ns_view` / Win32 `hwnd`）。**原生 handle 只是借用，窗口销毁后不能继续缓存或使用** | native-extension §2. 取得正确的 window handle | — |
| ⚠️ **WebView 是原生子视图，不是 GPUI Element**：它位于 GPUI surface **上方**，会遮住同矩形内的 **popover / dialog / menu / tooltip**，**GPUI overlay 无法可靠显示在它上面**。要 overlay 就把 WebView 放单独窗口，或让 overlay 不跨过它的 bounds | webview §当前限制、native-extension §合成边界 | ⚠️ 与 §2.2 ④ 相关：**macOS 侧编辑器是 WKWebView**，若 gpui 侧也走 WebView 渲染编辑器，**所有浮层（补全、悬停、代码动作菜单）都会被它盖住** —— 这是选型时必须先解决的硬约束 |
| ⚠️ **ZIP ≠ 安装程序**：ZIP 不创建开始菜单项、卸载记录或升级规则；Windows 面向用户用 Inno Setup，Linux 用 DEB/RPM。**`cargo build --release` 不会自动创建应用包** | packaging §开头、§Windows | 与 `release-lithe`（发布链路待建）相关 |
| 打包前要固定：**应用标识**（bundle id / Inno `AppId` / 包名，升级与并存靠它）、**资源**（`include_bytes!` 的不用复制；用 `Path` 读的必须随包提供）、**运行时依赖**（开发机的库 ≠ 用户机的库；Linux 的 `-dev` 包是构建依赖，不能列为用户依赖）、**平台功能**（通知/文件关联/URL scheme/WebView 可能另需身份与权限声明）。先放进**独立暂存目录**再打包，避免把 `target/release` 中间文件与调试符号发出去 | packaging §先构建并列出运行时文件 | 本项目资源全走 `include_bytes!` / rust-embed，所以这一条已经简单 |
| macOS：`.app` 结构 + `Info.plist`（`CFBundleExecutable` 必须与 `Contents/MacOS` 下的文件名一致）；对外分发要 **Developer ID 签名 + 公证**（先签嵌套代码再签外层 `.app`），DMG 也要签名并 `stapler staple`；**ad-hoc 签名不能替代**。最后在**干净机器**上挂载、拖入 Applications、首次启动过 Gatekeeper | packaging §macOS | 待建 |
| Windows：发布前**先签 `.exe`，再签最终安装器**；⚠️ **Inno Setup 会把卸载器嵌入安装 `.exe`，所以事后只签安装器 ≠ 签了卸载器**，要按 Inno 的签名卸载器流程配。Linux：用户可写的 tarball 走便携路径，DEB/RPM 管理的文件交给包管理器（**不要直接写 `/usr`**） | packaging §Windows、auto-update §按平台完成安装 | 待建 |
| ⚠️ **GPUI Kit 不提供自动更新服务**，更新策略属于应用。**更新路径取决于实际安装来源，不能只按操作系统猜**（便携版/`.app`/Inno/DEB 四种各不同）；**`self_update` 不要在启动路径调用** | auto-update §开头、§简单路径 | 待建 |
| 更新 UI 与任务分工：**更新状态留在 `Entity<T>`，网络与磁盘工作放后台任务**，回来再 `notify()`；**不要在 `render` 里查网络/下载/改安装目录**。状态机 `Idle/Checking → Current/Available → Downloading/Verifying/Installing → ReadyToRestart → Failed`；启动时后台查一次 + 保留手动入口；**检查失败不应阻止应用启动**；**下载进度不必逐字节 `notify`**；一次只装一个目标版本，**过期任务不得回写** | auto-update §让 UI 与更新任务各司其职 | 与 §1.4 P 组的"代次守卫"同源 |
| ⚠️ **信任链的三层要分清**：同发布位置的 `SHA256SUMS` 只能发现**下载损坏**（发布位置被控制时攻击者可同时替换摘要）；**发布者身份**要靠签名/签名元数据/平台代码签名；macOS 还要核对 bundle 签名与身份。**验证失败必须停在替换之前** | auto-update §先定义发布契约 | — |
| ⚠️ **`update()` 报告已装新版时，磁盘文件已更新但当前进程仍在跑旧代码** —— 要等应用保存状态、关闭窗口与后台资源后再请求重启，**不要在安装回调里立刻杀进程**。`self_update` 不会替应用提权，也不提供失败恢复 | auto-update §简单路径 | — |
| 发布顺序：**先上传并验证各平台产物与校验资料，最后才让更新元数据指向它们**；不要宣称某版本可用却没有对应架构的可下载文件。最小验证矩阵要覆盖"无更新/有更新/同版本/降级/换渠道/错架构/缺产物/坏摘要/错签名/离线超时/磁盘不足/目录不可写/下载中关窗/安装中退出/重启后确认版本" | auto-update §先定义发布契约、§失败恢复与发布验证 | — |
| **开发模式性能**：Debug 下 GPUI/组件/布局/文字 crate 基本没优化，`cargo run` 会明显慢。指南给 `[profile.dev.package]` 对 `gpui-pre` / `gpui-component` / `gpui-kit` / `gpui-kit-assets` / `gpui-pre-macros` / `gpui-pre-platform` / `rustybuzz` / `taffy` / `ttf-parser` 设 `opt-level = 3`（**不会加快编译**，改善的是运行）。**本地 Rust 要求 1.92+** | installation §提升开发模式运行性能、§Rust 和 Cargo | ✅ 本项目 `rust/Cargo.toml` **已经配了** `[profile.dev.package.*]` 覆盖，注释还写明"是必需的，不是优化"（因为 workspace 统一 feature） |
| 任务页提到的另外两个层面的目标平台：**移动端**（iOS 示例、嵌入 UIKit 视图、平台判断）与 **WebAssembly**（`wasm32-unknown-unknown`，字体/CJK 处理见 `fonts.md` 的 Canvas 回退）。本项目当前都不在范围内 | mobile、webassembly | 忽略 |

**AA. `docs/` 41 份文件索引（已逐份读过）**

| 文件 | 主题 | 用在哪 | 读况 |
| --- | --- | --- | --- |
| `index.md` | 五层架构（`gpui` / `gpui-kit` / `gpui-base` / `gpui-component` / `gpui-shell`）、**成熟度标注制度**、最小示例 | 定位与术语 | ✅ 全文 |
| `getting-started.md` | 初始化三步、`Root` 包裹、Entity/Render 心智模型 | 启动顺序 | ✅ 全文 |
| `coding-guides.md` | 架构、Root 所有权、组成单元、状态、渲染、主题、事件焦点、异步、布局、列表、命名、测试、性能、检查表 | §1.4 的主要来源 | ✅ 全文 |
| `design-guides.md` | 层级、颜色、圆角/间距/密度、空间语法、对齐、缩放与 rem、表面、交互状态、反馈浮层、动效、数据密集、用词、无障碍、评审清单 | §1.4/§2.2 的主要来源 | ✅ 全文 |
| `style.md` | `Styled` 全量方法、**rem 尺度与 `_N` 后缀清单**、滚动/裁剪/定位、FluentBuilder、trait 边界 | **单位口径的权威出处**（§1.1 第 1 条） | ✅ 全文 |
| `geometry.md` | `Point`/`Size`/`Bounds`/`Edges`/`Placement`、坐标原点、`Length` 家族、`Hsla`/`Rgba` | §1.4 E | ✅ 全文 |
| `fonts.md` | 默认字体、系统字体、`Theme` 改字体、**打包字体注册时机**、换字体改变布局 | §1.4 F | ✅ 全文 |
| `i18n.md` | `rust-i18n` 接入、`gpui_component` namespace、`extend!` 顺序、切换语言、RTL 限制 | §1.4 F | ✅ 全文 |
| `window.md` | 窗口创建、几何与缩放、Focus/Action 派发、`defer` vs `on_next_frame`、帧生命周期、Task 谱系、关闭拦截 | §1.4 G | ✅ 全文 |
| `assets.md` | `icon_assets!` 按需选图标、**嵌入量对照表**、自定义 `AssetSource`、`img` vs `svg` | §1.4 H（→`D25`） | ✅ 全文 |
| `entity.md` | `Entity<T>`、`WeakEntity`、observe vs subscribe、重入 panic、订阅生命周期、View 缓存边界 | §1.4 I | ✅ 全文 |
| `render.md` | `render` 的重建语义、状态与元素树分离、常见问题表 | §1.4 I | ✅ 全文 |
| `context.md` | `App`/`Window`/`Context`/`AsyncApp` 作用域、`defer_in`、跨 `await`、callback 参数约定 | §1.4 J | ✅ 全文 |
| `render-once.md` | `RenderOnce` vs `Render`、`#[derive(IntoElement)]`、组件分层选择 | §1.4 K | ✅ 全文 |
| `element_id.md` | `GlobalElementId` 路径、唯一性规则、稳定 key、Dock resize handle 实例 | §1.4 L | ✅ 全文 |
| `view-cache.md` | `Entity::cached(style)`、命中条件、四种机制区别、验证方法 | §1.4 M | ✅ 全文 |
| `focus.md` | 焦点四操作、Tab 顺序、`focus_trap`、`is_focused` vs `contains_focused`、焦点订阅 | §1.4 M | ✅ 全文 |
| `action.md` | `actions!`、Capture/Bubble 两阶段、三种派发目标、共享 owner、传播 | §1.4 N（→`D21`） | ✅ 全文 |
| `keybinding.md` | 按键串写法、`secondary`、context predicate、优先级与 `None` 的陷阱、菜单同步 | §1.4 N（→`D20`） | ✅ 全文 |
| `global.md` | `Global` 选型、读写 API、通知语义、常见错误 | §1.4 N | ✅ 全文 |
| `task.md` | Task 所有权、四种 spawn、**一次 poll 阻塞 UI 线程**、取消与过期拒绝 | §1.4 O（→`D10`） | ✅ 全文 |
| `element.md` | 三阶段职责判据、布局/派发/Scene 三结构、`HitboxBehavior`、`canvas` 边界 | §1.4 P | ✅ 全文 |
| `paint.md` | `paint_quad`/`paint_path`、PathBuilder、SVG 迁移、几何缓存、内容裁剪 | §1.4 Q | ✅ 全文 |
| `event.md` | Action vs Event、`emit` 不重绘、订阅默认结束、输入捕获冒泡、两个阻止 API | §1.4 R | ✅ 全文 |
| `text-system.md` | 塑形与测量、`TextRun::len` 是**字节**、索引是字节偏移、`width()` ≠ 墨迹、缓存边界 | §1.4 S | ✅ 全文 |
| `animation.md` | 三层动效、稳定 ID 规则、`Presence`、减弱动态效果 | §1.4 T | ✅ 全文 |
| `fps.md` | `gpui-fps` HUD、`FRAME` 口径、闲置不等零、**隐藏时零开销** | §1.4 U（→`D10`/状态栏帧率） | ✅ 全文 |
| `accessibility.md` | **ID + role 才进无障碍树**、命名与操作、只读仍可写、测试口径与局限 | §1.4 V | ✅ 全文 |
| `test.md` | 四层测试、`TestWindowExt` 操作集、`ElementSnapshot`、`wait_for`、上游回归契约 | §1.4 W（→`D26`） | ✅ 全文 |
| `shared-string.md` | `SharedString` 选型、clone 成本、与 `Cow` 的区别 | §1.4 X | ✅ 全文 |
| `multi-window.md` | 每窗口独立 `Root`、共享 model 划分、`AnyWindowHandle` | §1.4 X | ✅ 全文 |
| `image.md` | `img()` 图像源、`ObjectFit`、`svg()` 单色、两级缓存、HTTP 缓存 | §1.4 Y | ✅ 全文 |
| `system-notification.md` | toast vs 系统通知、`set_app_identity`、单一响应 handler、平台投递条件 | §1.4 Z | ✅ 全文 |
| `auto-update.md` | 发布契约、更新状态机、信任链、`self_update` 边界 | §1.4 Z | ✅ 全文 |
| `webview.md` | `gpui-wry` 实验性、**overlay 层级限制**、Focus 与生命周期 | §1.4 Z | ✅ 全文 |
| `native-extension.md` | `NativeMenu` vs `PopupMenu`、两个 `window_handle`、原生对象生命周期 | §1.4 Z | ✅ 全文 |
| `installation.md` | 平台依赖、Rust 1.92+、`[profile.dev.package]` 优化、`gpui-pre` 命名由来 | §1.4 Z | ✅ 全文 |
| `packaging.md` | 四类分发格式、macOS 签名公证与 DMG、Windows Inno/签名卸载器、暂存目录 | §1.4 Z | 🔶 前半（macOS/Windows）；Linux 与验证矩阵未读 |
| `mobile.md` | iOS 示例、UIKit 嵌入、平台判断、移动界面设计 | 不在范围内 | 🔶 仅目录 |
| `webassembly.md` | `wasm32-unknown-unknown`、Web 入口、字体与 CJK 回退 | 不在范围内 | 🔶 仅目录 |
| `comparison.md` | 与同类框架的选型速览 | 参考 | 🔶 仅目录 |

> **整棵文档树**（都在本仓库，是 gpui-kit 0.7.0 的官方文档镜像）：
> `docs/` **41** 份（上表）· `component/` **78** 份（组件）· `base/` **49** 份（无样式行为与基础设施）· `shell/` **15** 份（JS 扩展宿主）
> —— 合计 **183** 份 md。本项目不用 `gpui-shell`（不需要 JS 扩展），那一层可以整体忽略。
> `component/` 与 `base/` 两层尚未开始读，按需查阅即可。
>
> **两条伴随 Skill**（本机已装，内容与上面的指南同源，但多了两份速查）：
> `gpui-kit-design-guides`（设计指南 + 非协商项）· `gpui-kit`（编码指南 + **75 个组件的导入路径总表**）。
> 后者在挑组件时比翻 `component/` 目录快；它还明确 **`Dialog` 与 `Sheet` 自带焦点陷阱**（见 §1.4 M）。
>
> ⚠️ **这两个 skill 的指南副本原本落后**，2026-10-02 已按官网 0.7 英文版刷新：
> `coding-guides` 缺了整个 0.7 首节 `Make the common path inexpensive`（21 节 → **22 节**，与仓库中文版一致）；
> `design-guides` 缺了「选中态不得用左侧竖条/单侧边框」等 0.7 条文。另修掉 skill 里 **4 处 `Task::then(...)`** 样例
> —— `gpui-pre 0.3.7` 的 `Task` **没有 `then`**（只有 `ready`/`is_ready`/`detach`/`fallible`/`downcast`），那些样例编译不过；
> 并补上了 0.7 的 `RootPlugin` / `register_plugin` 机制与版本作用域说明。

---

## 2. 逐区域对应（规格来源：macOS 前端）

### 2.0 怎么读 §2

- **规格 = macOS 前端**（`macos/Sources/Lithe/Views/`）：下表的"组成 / 关键类型 / 度量"都出自那里的真实 Swift 源码。
- **同步状态**只有四档，且必须诚实标注：

| 状态 | 含义 |
| --- | --- |
| **✅ 已接通** | gpui 有实现，**并且真的发了 Core 命令 / 消费真实数据** |
| **🟡 部分** | gpui 有实现但缺块，或只有界面没接数据 |
| **⚪ 缺界面** | Core（或 Core 之外的引擎）已经具备，缺的只是 gpui 界面/接线 |
| **❌ 缺 Core** | 连 Core 都没有，先补 Core 再谈界面 |

- **"组件替换"一列标注 `[建议]` 的是选型建议**（"用 gpui-kit 这几个造"，不是"某人已经这么做了"）。
- 需要"谁缺 Core、谁缺界面"的可执行清单，看 `docs/tasks/gpui-core-first.md`；本文只做**界面规格对照**。

### 2.1 总表

| # | 区域 | macOS 出处（文件数 / 行数） | 关键类型 | gpui 落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 工作台外壳 | `Workbench/` 9 / 4345 | `WorkbenchView`、`WorkbenchLayoutMetrics`、`SplitHandleView`、`WorkbenchModuleUIRegistry`、`WorkbenchHoverTooltip*` | `crates/workbench/src/workspace.rs`、`title_bar.rs`、`project_tabs.rs`、`activity_bar.rs`、`status_bar.rs`、`menu_bar.rs`、`project_menu.rs`、`right_tool_window.rs` | 🟡 部分 |
| 2 | 应用壳与设置 | `App/` 16 / 5309 | `RootView`、`LitheWindowCoordinator`、`WelcomeView`、`SettingsView`、`KeyboardShortcutSettingsView`、`PluginManagementView`、`UpdateControl` | `crates/app/src/`、`crates/settings/src/`、`workbench/notifications.rs` | 🟡 部分 |
| 3 | 工作区与项目 | `Workspace/` 6 / 2297 | `ProjectSidebarView`、`ProjectSwitcherPopover`、`ProjectAvatarBadge`、`CloneRepositoryView`、`DependencySidebarView`、`OpenProjectLocationDialog` | `crates/explorer/src/`、`workbench/project_menu.rs` | 🟡 部分 |
| 4 | 编辑区 | `Editor/` 28 / 11908（含 5497 行的**未接线**原生编辑器，见下） | `EditorAreaView`、`DocumentPreviewMode`、`EditorTabFlowPlanner`、`FindBarView`、`GoToLineDialogPresenter` | `crates/editor/src/editor_view.rs`、`buffer.rs`、`navigation.rs` | 🟡 部分 |
| 5 | 差异视图 | `Diff/` 6 / 2343 | `DiffReviewView`、`DiffSplitPaneView`、`DiffPaneView`、`DiffMapView`、`DiffCollapsedBandView`、`DiffLayoutMetrics` | **无** | ⚪ 缺界面 |
| 6 | Git | `Git/` 32 / 11940 | `GitLogView`、`GitGraphView`、`GitReferenceRowsBuilder`、`ChangesSidebarView`、`BranchSwitcherPopover`、`GitCommitFileTree*`、`GitWorktreesView`、`GitConsoleView` | `crates/git/src/`（`log_view` / `model` / `changes` / `changes_view` / `branch_info`）、`workbench/branch_panel.rs` | 🟡 部分 |
| 7 | GitHub | `GitHub/` 1 / 1954 | `GitHubPullRequestsSidebarView`、`GitHubPullRequestDetailView` | 只有菜单占位项（`menu_bar.rs`） | ⚪ 缺界面 |
| 8 | 本地历史 | `History/` 2 / 377 | `LocalHistoryView`、`ProjectLocalHistoryView` | **无**（`history.*` 命令 Core 已有） | ⚪ 缺界面 |
| 9 | 终端 | `Terminal/` 3 / 541 | `TerminalView`、`TerminalSurfaceView`、`TerminalTabDragPayload` | `crates/terminal/src/` | 🟡 部分 |
| 10 | 运行与 Maven/Spring | `Run/` 10 / 2989 | `RunView`、`RunConfigurationEditorView`、`MavenView`、`MavenBuildOutputView`、`SpringEndpointsView`、`RunServicesSplitView` | `crates/run/src/`、`workbench/maven.rs`、`spring.rs`、`spring_paths.rs`、`right_tool_window.rs` | 🟡 部分 |
| 11 | 调试 | `Debug/` 2 / 2663 | `GenericDebugView`、`DebugToolbarPresentation`、`DebugBreakpointManagerView` | **无**（Core 18 条 `debug.*` + `debug/engine.rs` 已就绪） | ⚪ 缺界面 |
| 12 | 语言智能与诊断列表 | `Language/` 6 / 1497 + `Diagnostics/` 1 / 80 | `ProblemsView`、`LanguageReferencesView`、`LSPControlCenterView`、`LanguageServerSetupView`、`LanguageTestsView`、`DiagnosticsExportSheet` | `crates/editor/src/problems.rs`、`diagnostics.rs`、`crates/java/src/` | 🟡 部分 |
| 13 | 搜索 | `Search/` 6 / 1498 | `SearchSidebarView`、`SearchEverywhereView`、`ProjectReplaceView`、`ProjectReplaceFloatingPanel`、`ProjectReplacementSourcePreview` | `crates/workbench/src/search_panel.rs`（**Find in Files 面板**：查询框 / 文件掩码 / 三个选项 / 文件头+命中行 / 点击跳行，U17 第一件）+ `quick_open.rs`（**Quick Open 浮层**：`Ctrl+P` 文件名快速打开，U17 第二件）+ `search_everywhere.rs`（**Search Everywhere 浮层**：All / Classes / Files / Symbols / **Actions** / Text 六个 scope + `/` 命令模式，U17 第三件 + 后续切片补 Classes、Actions 与 `/`，菜单「视图 → 全局搜索」）+ `replace_in_files.rs`（**Replace in Files 浮层的替换预览**：`Ctrl+Shift+R` / 菜单「转到 → 在文件中替换」，按文件的 before/after，U17 第四件）+ `search_model.rs`（纯逻辑层，四件共用）+ `search_index.rs`（**索引侧已接线**，U18：三条 `workspace.searchIndex.*` 的唯一 owner） | 🟡 部分（四件套的 Find in Files / Quick Open / Search Everywhere **六 scope + `/` 命令模式** / Replace in Files **预览** 已做；**替换的应用未做** —— Core 本分支没有替换命令，等 main；相关度重排仍未做（等 Core 给 relevance 字段）；⚠️ **Actions 档是子集**：宿主命令目录 `CommandId::ALL` 只有 **22** 条，真源 `LitheCommandCatalog.swift` 有 **39** 条） |
| 14 | 数据库 | `Database/` 6 / 5255 | `DatabaseSidebarView`、`DatabaseSQLWorkspaceView`、`DatabaseTableView`、`DatabaseSchemaDiffView`、`RedisWorkspaceView`、`NacosWorkspaceView` | 只有菜单占位项 | ❌ 缺 Core（引擎 `rust/lithe-db-sidecar` 已在仓库） |
| 15 | AI 助手 / Agent | `Agent/` 13 / 3204 | `AgentConversationView`、`AgentTranscriptView`、`AgentComposerView`、`AgentHistoryView`、`AgentPanelSettingsView`、`AgentProviderConfigurationView` | **无** | ❌ 缺 Core（ACP 引擎 `rust/lithe-agent-host` 已在仓库） |
| 16 | 可复用组件层 | `Components/` 11 / 1998 | `LitheSplitPaneView`、`LitheToolWindowHeader`、`LitheUnavailableView`、`LitheContextMenu`、`LitheSettings*`、`LitheScrollViewChrome`、`WorkbenchHoverTooltip` | 无对应层 —— gpui **直接用 gpui-kit** | 🟡 部分 |
| 17 | 主题 | `macos/Sources/Lithe/Theme/LitheTheme.swift` | `LitheTheme`、`Metrics`（15 刻度） | `crates/settings/src/theme.rs`、`themes/*.json` | 🟡 部分 |

> 表里的文件数/行数是 `macos/Sources/Lithe/Views/` 下按目录实测的（合计 **158 个 Swift 文件 / 60,198 行**），
> 可以直接用 `Get-ChildItem <目录> -Recurse -Filter *.swift | Get-Content | Measure-Object -Line` 复核。

**统计（17 个区域，逐行核对过）**：✅ 0 · 🟡 部分 10 · ⚪ 缺界面 5 · ❌ 缺 Core 2。
没有"✅ 已接通"的区域不是坏消息——它说明**还没有任何区域算做完**，也提醒这张表不能当"已完成"用。

---

### 2.2 逐区域明细

#### ① 工作台外壳

- **macOS 组成**：`WorkbenchView` 是单文件 2389 行的总装。纵向：topBar（40pt，左侧 83pt 给系统交通灯）→ projectTabBar（`tabHeight + 4` = 38pt，只有一个项目时整条隐藏）→ HStack［activityBar（宽 40 = `:12` 的 `ActivityBarMetrics.width`，由 `:1342`/`:1345` 落 `.frame(width:)`；`:257` 的 `HStack` 首个子视图 ⇒ 左侧）· workspaceArea（尾部内缩 40pt）］→ statusBar（24pt）。中央列是纵向 split：上＝横向 split（左栏 320、可拖区间 30…`min(520, 可用宽 − 400)`；编辑区最小 400），下＝工具窗（互斥单例，最小 260；Maven 右栏默认 360）。右侧悬浮栏与 40pt 右活动栏是 overlay，**不占布局宽度**。
- **关键类型**：`WorkbenchLayoutMetrics`（全部度量常量）、`SplitHandleView`（自绘 NSView 拖拽层，thickness 5 / hit 10 / 可见轨 2）、`WorkbenchRightToolSplitView`、`WorkbenchModuleUIRegistry`（模块贡献注册表）、`WorkbenchHoverTooltip*`、`WorkbenchStatusViews`（`EditorCaretPositionLabel` / `MemoryUsageStatusView` / `FrameRateStatusView`）。
- **gpui 落点**：`crates/workbench/src/workspace.rs`（外壳总装 `ShellWorkspace`）、`title_bar.rs`、`project_tabs.rs`、`activity_bar.rs`、`status_bar.rs`、`menu_bar.rs`、`project_menu.rs`、`right_tool_window.rs`、`session.rs`；`crates/notify/src/`（通知中心）。
- **组件替换**：分区与分割直接用 `DockArea` + `DockLayout`（本项目已用，且把底部放进 center 的 `v_split`，因为 `DockPlacement::Bottom` 只横跨中心列）；可拖分栏用上游 `resize_handle` / `resize_handle_appearance`；下拉与右键菜单用 `PopupMenu`；空态用 `Empty`；角标用 `Badge`；头部图标用 `Toolbar`。
- **缺口与优化**：✅ **标题栏已对齐（`U25`，2026-10-03）**：`title_bar.rs` 的 `TITLE_BAR_HEIGHT_SPEC = 40.` **单行**（原先是 30 + 38 两行 68pt）；⚠️ 仍**自绘窗口三键**（macOS 规格里没有这个界面点）—— 这条差异保留。✅ **项目标签条已对齐**：`project_tabs.rs` 的 `STRIP_HEIGHT_SPEC = 38.` / `TAB_HEIGHT_SPEC = 30.`、`TAB_MIN_WIDTH_SPEC = 180.` 且按可用宽度**等分**（`flex_1()` + `min_w(180)`）、`FOLDER_ICON_SIZE_SPEC = 11.`、`CLOSE_BUTTON_SIZE_SPEC = 28.`、`.overflow_x_scroll()` + 切项目 `scroll_to_item`。⚠️ 仍缺：标签条另 **2 个**信息元素（"当前文档名"后缀、`AgentAttentionIndicator`）—— `ShellWorkspace::projects` 恒为**单根**、这条标签条当前根本不渲染，**没有诚实的数据源**，加字段就是死代码。✅ **活动栏 rail 宽已对齐真源并收敛到共同 owner（`B14`，2026-10-03 维护者裁决选 (a)）**：宿主原先**一条** `COLLAPSED_WIDTH = 38.` 管左右两侧（38 出自**已作废**的 `windows/tauri`），现改为引用 `crates/shared/src/design.rs` 的 `ACTIVITY_RAIL_WIDTH_SPEC = 40`；复核真源时**补正**了 B14 原记的"真源只定义右侧"—— 真源左右**各自**都有一条 40（左 = `WorkbenchView.swift:12` 的 `ActivityBarMetrics.width` → `:1342`/`:1345`，右 = `:7` 的 `rightActivityBarWidth` → `:13`/`:1415`）。⚠️ **这条改动是可见的**：两条 rail 各宽 2px —— 左轨对外盒宽因 `WORKBENCH_GAP` 由 42 → **44**、右轨由 38 → **40**（含外侧 4px 间隔后也是 44）。⚠️ **未经真机视觉验证**。仍未解决（不改，留给维护者）：宿主"一条常量管两侧"的**结构**没动（要拆成左右两个值属布局结构调整）、左轨那 4px 间隔真源里没有（真源 `:257` 是 `HStack(spacing: 0)`，且右 rail 是 `.overlay` 不占布局宽）、项盒 26/28 与 rail 的新算术（44 − 16 = 28）不一致（属 `B12`，见 `BLOCKERS.md` B14 的"仍未解决"）。✅ **三块面板都能拖、长度都持久化**（2026-10-01）：左栏（既有手柄）、右工具窗、底部工具窗各一条上游 `resize_handle`，共用同一套会话开关 / 跟踪 / 钳制。手柄贴边与轴：左栏 `HandleEdge::Trailing` + `Axis::Horizontal`、右栏 `HandleEdge::Leading` + `Axis::Horizontal`（往左拖变宽）、底部窗 `HandleEdge::Leading` + **`Axis::Vertical`**（往上拖变高，`gpui-base-0.7.0/src/resizable/resize_handle.rs:277-290` 支持）；三个 `ElementId` 互不相同（`lithe-left-pane-resize` / `-right-` / `-bottom-`，官方 `element_id` 那条"同 key 祖先下同名会串拖"的坑）。长度落在会话文档 `.lithe/session.local.json` 的 `leftPaneWidth` / `rightPaneWidth` / `bottomPaneHeight`（`crates/shared/src/workspace_config/session.rs`，可选键、旧文件读到 `None` 回落外壳默认值），**落盘只在手势结束那一次**、走既有的 300ms 去抖会话保存 —— 拖动中的每一帧只改内存值 + `notify`。钳制（**2026-10-03 按真源落地**，维护者裁决 `U25` 遗留 4 条全部按真源改）：左栏 **30–`min(520, 分栏容器宽 − 400)`**（真源下界 `WorkbenchLayoutStore.swift:4` 的 `minimumSidebarWidth = 30`、上界 `WorkbenchView.swift:2008-2011` 的 `max(minimumSidebarWidth, min(520, availableTopWidth - minimumEditorWidth))`，其中 `minimumEditorWidth = 400` 是**给编辑区**（分栏里 `flexible` 的那半）留的宽，不是给右栏留的；改前是静网 200–720 ⇒ **可见改动**：能拖的下限与上限都变了）；右栏默认 **360**（真源 `WorkbenchLayoutStore.swift:5` 的 `defaultMavenPaneWidth`；改前的 400 出自**已作废的 windows/tauri** 的 `default-settings.ts:139` ⇒ **可见改动**：右栏首次展开的宽度），区间 300–760 且随窗口宽收敛到"中央列至少 640"（真源 `WorkbenchRightToolSplitView.swift:5,7-16`）；底部窗默认 **260** = 真源最小高 `WorkbenchView.swift:2019` 的 `minimumGitPaneHeight = 260`，区间 260–`可用高−220`（同文件 `:2018` 的 `minimumTopPaneHeight = 220`）—— 默认值与下界**同值**、clamp 在它上面恒等 ⇒ **视觉无变化**（改前那个 240 首帧就被钳到 260，一次都没画出来过）。上界现在相对**分栏容器**算：`render` 里给装 [左侧面板, 中央列] 的那个 `gap_0` 组挂 `.on_prepaint`（`ElementId = lithe-workspace-split-container`，本仓既有写法，同 `project_menu.rs` / `menu_bar.rs` 量触发器盒），实测尺寸存进 `ShellWorkspace::split_container`、下一帧供左栏（用 `width`）与底部窗（用 `height`）算上界 —— 它就是真源 `WorkbenchView.swift:1999` 那个 `GeometryReader` 的对应物，真机可 grep 一行 `S1_SPLIT_CONTAINER w=… h=…` 核对。⚠️ **右栏仍只能用 `window.viewport_size().width`**：真源给它的容器是 `WorkbenchRightToolSplitView` 的 `geometry`（= 窗口减两条活动栏、**包含**右工具窗自己），而本侧两条活动栏是同一行的**兄弟**、不是外框，那个容器在树里**没有对应元素**（拿 `split_container.width` 会重复扣掉右工具窗自己）⇒ 差额 = 两条活动栏 + 4px 间距 = **92**（左 44 + 右 44 + 4；rail 宽对齐真源 40 之后的数，见 `BLOCKERS.md` B14），方向是**略松**、非真源。❌ 仍缺：三块面板的尺寸没有真机视觉验证（拖动观感 / 光标变化 / 手感未截图），且**没有** UI 层的拖动端到端测试（无头窗口里定位手柄命中盒要复制布局算术，属脆测试）。❌ gpui 左右活动栏项与视图集合是**写死的两张表**，macOS 走 `WorkbenchModuleUIRegistry` 的模块贡献注册（terminal/agent/git/language/execution/debug 各自贡献活动栏项与工具窗）。❌ 状态栏 4 项是占位或假值：内存恒 `"总计 0.0 MB · Lithe 0.0 MB"`、编码恒 `UTF-8`、缩进恒 `2`、只读图标恒为锁定；缺帧率、编码菜单、usages 计数、`{n} changes`、内存明细 popover、可点分段的面包屑。⚠️ gpui 通知中心比 macOS 强（搜索 + 四档筛选 + 行内展开 + 相对时间 + `×N` 去重）。**toast**：位置**已对齐**右下（`Anchor::BottomRight`，`crates/workbench/src/workspace.rs:3361` 的 `.placement(Anchor::BottomRight)`；本条原先写的"gpui `BottomLeft`、左右相反"已过期，更正于 2026-10-03 —— 这是**事实更正**，不是本轮改动）。**同时可见条数上限 = 3：已接**（真源 `macos/Sources/Lithe/Application/Features/WorkbenchNotificationFeatureModel.swift:6` 的 `maximumVisibleCount = 3`），落在 `crates/settings/src/theme.rs` 的 `apply_theme_by_name`（`Theme` 的 `notification.max_items`，常量 `NOTIFICATION_MAX_ITEMS`）；**不是因为上游缺入口**，而是上游默认 10（`gpui-component-0.7.0/src/notification.rs:545,565`）且非主题文件字段。溢出**丢最旧、保最新**与真源同（真源 `:38-41` 的 `removeFirst()`）。⚠️ **hover 暂停：上游已内建，无需实现** —— `notification.rs:908` 传 `is_expanded()` → `:766-770` → `gpui-base-0.7.0/src/toast.rs:75-77`（`hovered || focused`，`hovered` 由 `:569-570` 的鼠标事件置位）→ `toast.rs:243-245`（`Present if !paused` 时不扣 `timeout_remaining`）。⚠️ **4s 时长仍改不了**：上游硬编码 5s（`notification.rs:872`）且 `Notification` 与 `NotificationSettings` **都没有** timeout 入口，属"等上游"。⚠️ **与真源的差异**：上游溢出只是**不渲染**（`toast.rs:146-152` 的 `visible(limit)`），条目仍挂在 manager 里、超时计时照走（`notification.rs:862-865` 的 `autohide_ids`），而真源 `removeFirst()` 是**永久移除** —— 本轮不修。**内衬（`margins`）：已对齐**（2026-10-03）：真源是同一条 overlay 的两条 padding —— `.padding(.trailing, WorkbenchLayoutMetrics.rightActivityBarWidth + 12)`（`macos/Sources/Lithe/Views/Workbench/WorkbenchView.swift:553`，其中 `rightActivityBarWidth = 40` 见同文件 `:7` ⇒ **52**）与 `.padding(.bottom, 38)`（同文件 `:554`，那里是**写死的常量**，没有状态栏的 `WelcomeView.swift:61` 也独立写了同一个 38）。落点是 `crates/settings/src/theme.rs` 的 `apply_theme_by_name`（`Theme.notification.margins` 的 `right` / `bottom`，常量 `NOTIFICATION_MARGIN_RIGHT_SPEC` = 52 / `NOTIFICATION_MARGIN_BOTTOM_SPEC` = 38），换算走 `lithe_gpui_shared::rem_px(cx.theme().font_size, 规格像素)` —— 因为 `NotificationSettings.margins` 是 `Edges<Pixels>`（`gpui-component-0.7.0/src/notification.rs:543`，消费点 `:1016-1017` 与 `:1062` 的右下锚点），只吃 `Pixels`，不能直接写规格数字。✅ **右内衬现在与 rail 宽共享同一个 owner**（2026-10-03，`BLOCKERS.md` B14 落地）：rail 宽只写在 `crates/shared/src/design.rs` 的 `ACTIVITY_RAIL_WIDTH_SPEC = 40`（真源 `WorkbenchView.swift:7`（右）/`:12`（左）），`crates/workbench/src/activity_bar.rs` 的 `COLLAPSED_WIDTH` 与 `crates/settings/src/theme.rs` 的 `NOTIFICATION_MARGIN_RIGHT_SPEC = 那个 owner + 12` 都引用它（值仍是 52 ⇒ **通知内衬的可见观感不变**）；`settings` → `shared` 是既有合法方向，而 `settings` 与 `workbench` 之间没有依赖方向，所以 owner 只能落 `shared`。两侧各有一条断言钉住这层关系（`activity_bar.rs` 的单测 / `theme.rs` 的 `notification_right_inset_derives_from_the_activity_rail_owner`，另 `tests/ui.rs` 从生产入口再断言一次）。⚠️ **宽度写死、自适应区间不可表达**：真源 `:546` 是 `.frame(minWidth: 280, maxWidth: 360, …)`（自适应），上游 `NotificationSettings.width` 是**单值** `Pixels`（`:547`，默认 `DEFAULT_NOTIFICATION_WIDTH = px(382.)`，`:33`）⇒ 既写不出区间，也没有理由随手填一个数（本轮**不动** `width`）。⚠️ **堆叠间距不可达**：上游 `expanded_gap` 固定 **14**（`gpui-base-0.7.0/src/toast.rs:51`），真源 `:519` 的 `VStack(alignment: .trailing, spacing: 8)` 是 **8** —— 建栈处 `notification.rs:1048` 没把 gap/motion 传进去，宿主够不到。⚠️ `margins` 是 `Pixels`（不是 rem）⇒ 值在应用主题那一刻定死，之后改「界面字体大小」不会重新换算、要等下次应用主题（默认档 `uiFontSize = 13` ⇒ 基准 16px，两条路等价）。⚠️ 内衬这一条**未经真机视觉验证**（只有结构性核验 + 无头窗口测试）。仍缺：逐条关闭（`Notification::on_close`）。⚠️ 右活动栏前两项顺序与 macOS 相反（macOS：通知 → 插件 → 模块贡献；gpui：扩展 → 铃铛 → Maven）。

#### ② 应用壳与设置

- **macOS 组成**：`RootView` 管窗口布局与生命周期（welcome 800×650、workspace 1440×900 / 最小 980×640），`LitheWindowCoordinator` 管多窗口（每项目一窗、`Cmd+W` 语义、未保存确认、关闭守卫）。`WelcomeView` 是独立的无项目界面。`SettingsView` 1512 行，含 Git 执行 / Git 拉取策略 / Git 身份 / MCP / 项目运行时 / 运行配置 / 快捷键（含录制器）/ 插件管理 / 稳定版回滚 / 更新 等子页。
- **关键类型**：`RootView`、`LitheWindowLayout`、`LitheWindowCoordinator`、`WelcomeView`、`SettingsView`、`SettingsViewState`、`KeyboardShortcutSettingsView`、`KeyboardShortcutRecorderView`、`PluginManagementView`、`UpdateControl`、`StableRollbackControl`、`EffectiveRuntimeLabel`。
- **gpui 落点**：`crates/app/src/main.rs`（组合根；启动顺序与窗口尺寸）、`crates/app/src/assets.rs`（内嵌资源）、`crates/settings/src/`（`schema` / `paths` / `persistence` / `theme` / `store` / `row` / `dialog` / `identity` / `extensions` / `fonts` / `project`）、`workbench/notifications.rs`。
- **组件替换**：[建议] 设置页用 `component::setting::{Settings, SettingPage, SettingGroup}`；快捷键录制用 `kbd` + `KeyBinding` 展示；对话框用 `Root` 的 dialog 层（0.7 由 `Root` 每帧渲染，**不要**再手动挂层）。
- **缺口与优化**：❌ **欢迎页整体缺失**：无参启动会兜底打开当前工作目录（`resolve_launch_root`），外壳无论有没有项目都画完整工作台，而 macOS 的欢迎页（最近项目 / 搜索 / 新建 / 克隆入口）是一个独立界面点。❌ **多窗口缺失**：`OpenDestination::NewWindow` 是死路（只打一行"未接线"日志）。❌ **快捷键设置页 + 录制器缺失**（macOS 侧 544 行），gpui 的键位全是硬编码 `KeyBinding`。❌ **更新与回滚缺失**：菜单里"检查更新"是占位项。❌ 设置页目前只有"常规 / 外观"两页，AI 分类是**有意不做**（有守卫测试钉着）。✅ 设置持久化本身是好的（原子写 + 300ms 防抖、`LITHE_GPUI_SETTINGS_FILE` 供测试指向临时文件、文件不存在不创建）。

#### ③ 工作区与项目

- **macOS 组成**：左侧「项目」树（`ProjectSidebarView`，含 Project/Dependencies 两个页签）、项目下拉（三条动作 / 打开的项目 / 最近项目）、项目徽标、克隆仓库对话框、依赖面板、打开位置对话框（New Window / This Window）。
- **关键类型**：`ProjectSidebarView`、`ProjectFileRowActivation`、`ProjectSwitcherLayoutMetrics`、`ProjectSwitcherPopover`、`ProjectIdentityAppearance`、`ProjectAvatarBadge`、`DependencySidebarView`、`DependencyFolderEntry`、`CloneRepositoryView`、`OpenProjectLocationDialog`。
- **gpui 落点**：`crates/explorer/src/model.rs`（`PathTree`）、`explorer_view.rs`（`Explorer`）、`crates/workbench/src/project_menu.rs`。
- **组件替换**：[建议] 树用 `base::{TreeState, TreeItem}`（表体是 `uniform_list`，虚拟化现成）；下拉用 `PopupMenu`；徽标用 `Badge` + 项目色板常量。
- **缺口与优化**：❌ **空目录不显示**：gpui 只用 `workspace.snapshot` 的 `data.files`（只含文件），而 `data.root` 里是有目录节点的。❌ 无文件系统 watcher，树与「更改」列表都会过期；手动刷新入口也没有（`Explorer::refresh` 已公开，但外壳只在构造后调一次）。❌ 无右键菜单 / 多选 / 拖拽（`Tree::context_menu` 上游已有，未用）。❌ 缺 Reveal Active File、Git 状态装饰、目录标记（Sources Root / Excluded / 包图标 / 压缩中间包）。⚠️ 树内搜索输入框是**不可达的死代码**（入口 2026-09-27 已删），且它源自已作废的 Windows 规格 —— macOS 没有树内过滤。⚠️ 度量按 Windows 值实现（缩进 `10 + depth*16`、字号 14），macOS 是 `8 + depth*14`、字号 13.5。⚠️ 行数上限 `RENDER_LIMIT = 5000` 静默截断，只在日志里留一行。

#### ④ 编辑区

- **macOS 组成**：两层结构 = 常驻标签栏 + 正文（或空状态）；标签可拖拽重排、支持单行/多行两种布局、中键关闭；正文支持分屏（右分屏放第二文档）；查找替换是独立 `FindBarView`；转到行列是独立 NSPanel；还有预览模式切换（editor / split / preview）。
- **关键类型**：`EditorAreaView`、`DocumentPreviewMode`、`EditorTabMiddleClick`、`EditorTabFlowPlanner` / `EditorTabFrameStore`（拖拽几何）、`FindBarView`、`GoToLineDialog` / `GoToLineDialogPresenter`、`HTMLPreviewView` / `SVGPreviewView` / `MarkdownPreviewView` / `MediaViewerView`、`EditorSoftWrapToggle`、`SyntaxHighlightingRegistry` + 9 个 adapter。
- **gpui 落点**：`crates/editor/src/editor_view.rs`（标签栏 + 正文 + 空态 + 右键菜单 + 关闭流程 + 会话恢复）、`buffer.rs`（buffer / 脏标记 / 大文件与二进制保护 / 语言映射 / 显示名）、`navigation.rs`、`diagnostics.rs`、`problems.rs`、`completion.rs`、`code_actions.rs`、`languages.rs`（自注册 XML 语法）。
- **组件替换**：正文用 `component::input::{Editor, EditorState}`（行号列 / 缩进线 / 折叠 / 多光标 / 撤销栈 / tree-sitter 都自带）；标签栏用 `TabBar`；查找替换用**上游内置**（**前提是显式 `.searchable(true)`**，见 §4）；对话框用 `Root` 的 dialog 层。
- **缺口与优化**：❌ **差异视图整体为零**（见 ⑤）。❌ **外部修改冲突横幅缺失**：gpui 没有文件监视，`open()` 只读一次盘，"磁盘被别人改了"完全不可见。❌ **编码探测与"按编码重开/保存"缺失**：非 UTF-8 一律有损 + 只读，状态栏固定显示 `UTF-8`。❌ 缺分屏、标签拖拽重排、单行/多行布局切换、Markdown/HTML/SVG/媒体预览、预览模式切换器、转到行列对话框、软换行开关、中键关闭标签。❌ 缺 Code Vision、inline blame、inlay hints、悬停文档、gutter 运行/断点装饰（**其中悬停与语义高亮上游已有缝，只是宿主没装 provider**）。⚠️ 自动保存是**常量开启 + 150ms 硬编码**，macOS 是设置项（默认 1.5s），手感不同。⚠️ 标签栏高度 36/字号 14/间距 16 是被上游写死的（规格是 28/13/4）。⚠️ 大文件策略不同：gpui >2 MiB **不打开**，macOS 是"照常打开 + 关语言服务 + 横幅"（两边的产品取舍不同，不只是实现差异）。✅ gpui 更强的地方：标签用**稳定 ElementId**（按哈希/路径，不用下标）、诊断有**三段陈旧保护**（代次 / buffer revision / buffer 存活）、服务就绪后会给已打开的 buffer **补装** provider。

#### ⑤ 差异视图

- **macOS 组成**：并排与单栏两种 diff、词级高亮、折叠未变区、diff map（右侧整文件标记条）、连接带、hunk 级 stage/unstage/discard、diff 内搜索、空白模式切换、逐差异导航、版本头。
- **关键类型**：`DiffReviewView`（1458 行，含 `DiffRowView` / `DiffLayoutMetrics` / `DiffConnectorOverlay` / `DiffSyntaxHighlighter`）、`DiffSplitPaneView`、`DiffPaneView`、`DiffMapView`、`DiffCollapsedBandView`、`DiffHorizontalScrollSupport`。
- **gpui 落点**：**无**。`settings/src/dialog.rs` 里已登记"gpui 侧没有差异视图"；`git/src/log_view.rs` 的"打开提交差异"按钮是禁用态。
- **组件替换**：[建议] 骨架用 `v_split` + `resizable`；行渲染沿用编辑器的 `EditorState` 只读实例或自绘 `uniform_list`（行高固定、需要虚拟化）；minimap 用 `canvas` 自绘；折叠带用可点击 `div` + `CollapsedBand` 语义。
- **缺口与优化**：⚪ **这是面上最大的单点空白**，且 **Core 已备好**：`git.diff` 支持 8 个参数（`reference` / `gitReference` / `targetGitReference` / `commit` / `staged` / `untracked` / `contextLines` / `ignoreAllWhitespace` / `worktreeSnapshot`），`git.apply` 支持 4 种模式。macOS 的 `Models/Diff/DiffSplitLayout.swift`（184 行）是**纯几何算法、可直接移植并单测**。它与 ⑥ 的"提交差异审阅"、Git 的"分支比较"共用同一块基建，一次做能收掉 5 个 macOS 视图。

#### ⑥ Git

- **macOS 组成**：提交记录工具窗（引用 19% / 提交 57% / 详情 24% 三栏 + 泳道图 + 分页）、引用栏竖排工具栏、提交筛选 popover、变更侧栏（暂存/取消/丢弃/提交）、进行中操作横幅、分支弹窗（仓库/分支/工作树三页签）、提交差异审阅、分支比较、工作树视图、补丁视图、控制台、认证对话框、拉取策略与推送对话框、检出/集成冲突对话框、历史改写菜单。
- **关键类型**：`GitLogView`（3062 行，含 `GitLogNavigation` / `GitLogCommitSelection` / `GitPushDialog` / `GitPullStrategyDialog` / `GitCheckoutConflictDialog` / `GitIntegrationConflictDialog`）、`GitGraphView` + `GitGraphGeometry` + `GitGraphPresentation`、`GitReferenceRowsBuilder` + `GitReferenceMenuAction`（17 项）、`ChangesSidebarView`、`BranchSwitcherPopover`、`GitCommitFileTree*`、`GitWorktreesView`、`GitConsoleView`、`GitPatchPresentation`、`BranchComparisonView`、`GitHistoryRewriteView`、`GitInteractiveRebase*`。
- **gpui 落点**：`crates/git/src/log_view.rs`（提交三栏 / 泳道 / 引用树 / 筛选 / 详情 / 提交文件树 / 控制台外壳）、`model.rs`（`layout_graph` / `build_commit_files` / `build_reference_rows` / `matches_filter`）、`changes.rs` + `changes_view.rs`（变更列表 / 暂存 / 提交 / 丢弃 / 操作横幅 / 写失败红条）、`branch_info.rs`、`workbench/branch_panel.rs`（分支弹窗外壳）。
- **组件替换**：[建议] 三栏与工具窗用 `DockArea` / `v_split`；提交表与文件树用 `DataTable` / `uniform_list`；右键菜单用 `PopupMenu`；泳道与并排 diff **上游没有，需自研**（几何算法可从 macOS 移植）。
- **缺口与优化**：⚠️ **写与差异这一半几乎为零**：Core 有 39 条 `git.*` 与 `git.write` 的 47 个 operation，gpui 只用了 **7 条读命令 + 8 个 operation**。具体缺：推送/拉取/取回（`push`/`pull`/`fetch`/`updateBranch`/`deleteRemoteBranch` + `pushPreview`/`fetchPlan`/`pullPreflight`）、检出与集成前置检查、冲突标记检测与"解决冲突"入口、引用行右键 17 项、提交行右键（复制哈希 / 新建标签 / cherry-pick / revert / reset）、提交多选（Cmd/Shift 区间）、stash、worktree、tag（含删除后恢复）、历史改写、交互式 rebase、分支比较、补丁导出/预览/应用、Git 控制台真实数据（现在是**禁用空壳**，6 个按钮全 `false`）、认证挑战应答、Git 执行设置、多仓库聚合、工作区多仓库提交。❌ 提交**正文**不画（`Commit` 结构没有 body 字段，Core 的 `git.commit` 是返回的）。⚠️ 提交表**未虚拟化**（全部可见行塞进一个 `v_flex`），每页 100 行且无上限。⚠️ 筛选是**客户端对已加载页**过滤，macOS 的 `GitLogQuery` 支持文本/作者/分支/路径/日期/我 8 类并走 Core 重读。⚠️ 泳道图表达力弱：无箭头、无虚线、跨泳道父边只画竖段，且行高用了 Windows 的 30（macOS `GitGraphGeometry` 是 22 / 泳道间距 16）。⚠️ 三栏用比例 + `min_w` 复现，**栏间没有分隔条**（macOS 也没有，但 gpui-kit 的可调分栏是白送的）。✅ gpui 更强的地方：写操作失败**绝不静默**（把"信封 ok 但退出码非 0"翻成用户可见红条并剪掉 stderr 里的绝对路径）、写操作后必刷新且先归还分页游标、行 ElementId 用哈希而不是下标。

#### ⑦ GitHub

- **macOS 组成**：PR 侧栏（列表 + 筛选）+ PR 详情三页签（Overview / Files / Conversation）+ 评审动作（Comment / Approve / Request changes）+ 合并三策略 + 检出 PR 分支 + 发布分支。
- **关键类型**：`GitHubPullRequestsSidebarView`、`GitHubPullRequestDetailView`、`GitHubFeatureUnavailableView`。
- **gpui 落点**：只有菜单里一条占位项（点开给"尚未接入"提示）。
- **组件替换**：[建议] 列表与详情用 `List` / `v_flex` + Markdown 渲染；评审与合并用 `Dialog`；页签用 `Tabs`。
- **缺口与优化**：⚪ Core 侧 `github.parseRemote` / `github.requestPlan` / `github.normalizeResponse` 三条命令已存在（覆盖 5 类操作），也有 `git.pullRequestContext` 与 `git.write{publishBranch}`；缺的是**网络传输（平台端口）+ 整个界面**。这是工作量最大的独立区域，建议放在 Git 写闭环之后。

#### ⑧ 本地历史

- **macOS 组成**：文件级本地历史视图 + 项目级本地历史视图（按条目浏览与还原）。
- **关键类型**：`LocalHistoryView`、`ProjectLocalHistoryView`。
- **gpui 落点**：**无**；菜单里是缺失项。
- **组件替换**：[建议] 列表 + 预览两栏（`h_split` + `uniform_list`），还原用 `Dialog` 确认。
- **缺口与优化**：⚪ Core 的 `history.record` / `entries` / `content` / `relocate` / `rename` / `delete` **全都有**，gpui 零调用点。⚠️ 额外缺口：**"什么时机必须记录"目前没有契约** —— macOS 挂在保存 / 外部变更事件上，gpui 侧连触发点都没有。同时 gpui 也缺文件系统 watcher（见 ③），两者一起做才成立。

#### ⑨ 终端

- **macOS 组成**：终端工具窗（页签条 + 新建/关闭 + 标题 + 空态）、真 PTY 会话、页签可拖拽（含跨区拖入编辑器）。
- **关键类型**：`TerminalView`、`TerminalSurfaceView`、`TerminalTabDragPayload`。
- **gpui 落点**：`crates/terminal/src/`（`session.rs` / `terminal_view.rs` / `profile.rs` / `ansi.rs` / `constants.rs`）。
- **组件替换**：输出区用 `message_scroller::MessageScroller`（贴底跟随 + 跳回最新 + 从头部 splice 删行）；页签用 `TabBar`。
- **缺口与优化**：📌 **必须纠正一处传闻**：本项目**没有 PTY、也没有 VT 模拟器**。`rust/Cargo.lock` 里 `portable-pty` / `alacritty_terminal` / `termwiz` / `vte` / `vt100` / `conpty` **零命中**；实现是 `std::process::Command` + 三管道，`ansi.rs` 的职责是**把控制序列丢掉**（CSI / OSC 整段丢弃），只把 `\r`、`\b` 落实成字符效果。因此：**无 Ctrl+C**（只能关页签杀进程树）、**无 resize**（没有 cols/rows 模型，也没有 `TIOCSWINSZ` / `SIGWINCH` 通路）、**无 SGR 颜色**、无备用屏幕、无宽字符、无 IME、无鼠标上报；输入只有底部单行输入框。❌ 缺分屏 / 垂直页签 / 全屏 / 页签重命名 / "关闭其他·全部" / 终端内搜索 / 链接点击。🔴 **跨平台硬伤**：shell 探测写死 Windows 两项（`["powershell", "cmd"]`），在 macOS 上 `profiles()` 返回空、页签进失败态；对照 macOS 用 `$SHELL` → `/bin/zsh` 与运行时发现。⚠️ 失败态会回落成 `TerminalProfile::new("cmd")` 只为满足字段非空，是**假事实**。⚠️ 每次输出都新建一个后台任务 + 克隆 `Arc<Mutex<Receiver>>`，用互斥量绕开 `Receiver: !Sync`。

#### ⑩ 运行与 Maven / Spring

- **macOS 组成**：运行配置列表（按 scope 分组 / 固定 / 多选）+ 运行工具窗（可并行多会话 + 输出页签）+ 运行配置编辑器 + Java 启动决策对话框 + Maven 视图（模块树 / profile / 生命周期 / 依赖 / 构建输出 / 测试报告）+ Spring 端点视图（含筛选）+ 运行服务分栏。
- **关键类型**：`RunView`、`RunConfigurationEditorView`、`JavaLaunchDecisionDialog`、`MavenView`、`MavenBuildOutputView`、`SpringEndpointsView`、`RunServicesSplitView`、`RunConfigurationTokenCache`。
- **gpui 落点**：`crates/run/src/`（`model.rs` 配置识别、`plan.rs` 启动计划、`session.rs` 进程与输出、`pane.rs` 面板）、`crates/java/src/`（JDTLS 会话 / 工作区指纹 / Maven 上下文）、`workbench/maven.rs`、`spring.rs`、`spring_paths.rs`、`right_tool_window.rs`。
- **组件替换**：输出区用 `MessageScroller`；依赖树用 `Tree`；测试报告用 `DataTable`；进度用 `Progress` / `Spinner`；配置编辑用 `Dialog` + `form`。
- **缺口与优化**：⚪ **运行配置编辑器缺失**（macOS 511 行）：Core 的 `runConfig.updateOptions` / `saveEditorChanges` / `createUserConfiguration` 都已有，gpui 明确登记"没有配置编辑器，所以还没有消费方"。⚪ **配置分层没接**：gpui 只用 `runConfig.generate`，`Project` / `Local` 层恒不出现，Core 的 `runConfig.resolve` 是现成的。⚪ Maven 只画"项目 + 模块 + profile + 源码根"，缺**生命周期 / 依赖树 / 构建输出 / 测试报告**（`maven.dependencyPlan` / `maven.dependencies` / `maven.testResults` 全在 Core）。⚪ Spring 端点面板**没有筛选输入框**。⚠️ 运行面板**一次只能跑一条**（macOS 可并行多会话 + 输出页签）。🔴 **UI 线程阻塞**：`RunSession::start` 在 UI 线程闭包内同步跑完所有 `preLaunchSteps`（含 `javac`），最长可卡 300s。✅ gpui 更强：输出有显式行数上限 + 界面级 `truncated` 事实、前置步骤有显式超时、面板会回显"这次到底跑了什么命令行"。

#### ⑪ 调试

- **macOS 组成**：通用调试视图（调用栈 / 变量树 / 监视 / 求值控制台 / 线程）+ 调试工具栏（三真按钮，未启动时禁用）+ 断点管理对话框 + 步进过滤器设置 + 多会话选择器。
- **关键类型**：`GenericDebugView`（2564 行）、`DebugToolbarPresentation` + `DebugToolbarActionID`、`DebugBreakpointManagerView` / `BreakpointEditorView`、`EditorInlineDebugValue`（内联值）。
- **gpui 落点**：**无**。运行面板只有 ▶ / 🐞 / ■ / ⌫ 四个按钮；菜单里"切换断点"是占位项。
- **组件替换**：[建议] 调用栈与变量用 `Tree`；执行控制用 `Toolbar` + `Button`；断点表用 `DataTable`；求值输入用 `input::Editor`；内联值用编辑器 decoration。
- **缺口与优化**：⚪ **Core 侧已经完全就绪**：`debug/engine.rs` 是完整的有状态 DAP reducer，**18 条 `debug.*` 命令**全部在 `protocol/command.rs` 注册（`createSession` / `launch` / `javaTestLaunch` / `steppingFilters` / `relocateBreakpoints` / `setBreakpoints` / `setExceptionBreakpoints` / `setFunctionBreakpoints` / `dataBreakpointInfo` / `setDataBreakpoints` / `setVariable` / `cancelOperation` / `execute` / `inspect` / `receive` / `runInTerminalResponse` / `disconnect` / `destroySession`），并有 `debug/java_test.rs`、`debug/breakpoint_relocation.rs`。**而 gpui 的 `debug.*` 调用点是 0。** ⚠️ macOS 自己也**一行 DAP 都没写**：它只做 socket / 子进程 transport，协议归约全在 Core。所以这块是**纯 UI + transport 工程量**。🚧 **前置阻断（不在 Core 范围）**：进程/socket transport 与 `vscode.java.startDebugSession` 的启动、端口协商，Core 明确不管（`debug/engine.rs` 模块文档写明 "byte transport and process lifecycle remain platform owned"）。

#### ⑫ 语言智能与诊断列表

- **macOS 组成**：问题视图（按文件分组 / 严重度筛选 / 计数 / 四种空态 / 点击打开）+ Java 引用视图 + 实现选择器 + LSP 控制中心 + 语言服务器安装向导 + 语言测试视图 + 诊断导出对话框。
- **关键类型**：`ProblemsView`、`LanguageReferencesView`、`LanguageImplementationChooserView`、`LSPControlCenterView`、`LanguageServerSetupView`、`LanguageTestsView`、`DiagnosticsExportSheet`。
- **gpui 落点**：`crates/editor/src/problems.rs`（`ProblemsPane`，底部工具窗「诊断」页）、`diagnostics.rs`（波浪线）、`crates/java/src/`（JDTLS 会话 `session.rs` / 事件泵 `events.rs` / 服务 `service.rs` / `jdtls.rs` 载荷发现 / `workspace.rs` 指纹与缓存键）、`crates/editor/src/completion.rs`、`code_actions.rs`、`navigation.rs`。
- **组件替换**：问题列表用 `List` / `v_flex` + 折叠组头；严重度筛选用 `Select` / 分段控件；诊断导出用 `Dialog`。
- **缺口与优化**：⚪ 缺 **LSP 控制中心、语言服务器安装向导、Java 引用视图（跨文件引用列表）、语言测试视图、诊断包导出界面**。⚪ 4 条 Core 命令**没有消费者**：`java.runMarkers`（编辑器 gutter 运行/测试标记）、`java.codeVision`、`java.structure`、`maven.testResults`。⚠️ 问题面板的严重度筛选是**单选**，macOS 是 `Set` 多选；还缺按严重度分组、排序切换、全文搜索、来源筛选、快速修复菜单、复制、全屏。⚠️ 诊断获取是"有界重取 5 次、总上限 7.75s"，且**干净的 0 条与"还没算完"的 0 条不可区分** —— 这是有意的诚实取舍，但要记成体验债。⚠️ Core 的 `lsp.clearDiagnostics` / `lsp.snapshot` 出现在契约里但**全仓无实现**，属契约超期。

#### ⑬ 搜索

- **macOS 组成**：Find in Files 侧栏（查询框 / 大小写 / 全词 / 正则 / file mask / 按行结果 / 跳转）+ Search Everywhere 浮层（All/Classes/Files/Symbols/Actions/Text 六个 scope、`/` 命令模式、相关度排序、Tab 切 scope）+ Replace in Files（预览 before/after、整批应用、浮动面板、角标手柄）。
- **关键类型**：`SearchSidebarView`、`SearchEverywhereView`（含 `SearchEverywhereScope` / `SearchEverywhereQueryMode` / `SearchEverywhereResultSource`）、`ProjectReplaceView`、`ProjectReplaceFloatingPanel`、`ProjectReplacementSourcePreview`、`ProjectReplaceCornerHandle`。
- **gpui 落点**：**Find in Files 已做**（U17 第一件）：`crates/workbench/src/search_panel.rs`（视图：查询框 / 文件掩码框 / 区分大小写·全字匹配·正则三个 `Checkbox` / 文件头 + 命中行列表 / `idle`·`loading`·`ready`·`failed` 四态 / 点击结果打开并跳到该行）+ `crates/workbench/src/search_model.rs`（**纯逻辑层**：请求 payload、响应解析、结果分组、查询代次，可确定性单测）；左栏「搜索」项现在真的渲染这块面板（`workspace.rs` 的 `left_content` 三选一，与活动栏高亮同源）。**Search Everywhere 已做六个 scope + `/` 命令模式**（U17 第三件 + 后续切片补 `Classes`、`Actions` 与 `/`，见下面的 ✅）；`command_palette.rs` 是 `Ctrl+Shift+P` 的**命令面板**（与双击 Shift 的 Search Everywhere 是两件事，gpui **没有**双击 Shift，所以 Search Everywhere **没有键位**、只由菜单进入）；**Replace in Files 的预览已做**（U17 第四件：`crates/workbench/src/replace_in_files.rs`，`Ctrl+Shift+R` / 菜单「转到 → 在文件中替换」），**应用替换未做**（见下面的 ✅/⚪）。**索引侧已接线**（U18）：`crates/workbench/src/search_index.rs` 是三条 `searchIndex.*` 命令的唯一 owner（时机 / 请求形状 / 代次 / 诊断口径），`crates/editor/src/editor_view.rs` 的 `BufferWriteHost` 是"保存 → 索引"的注入点。
- **组件替换**：[建议] 侧栏用 `v_flex` + `input` + `List`；Search Everywhere 用 `Root` 的 dialog 层 + `Command`（上游 `Command` 自带 searchable / on_query / on_confirm / empty / max_h）+ `Tabs` 做 scope；替换用 `Table` 做前后对照。**实做**：Find in Files 用 `v_flex` + `Input` + `v_flex().overflow_y_scrollbar()`（**不用 `List`**：它要求所有行等高，而文件头与命中行不等高 —— 同 `notifications.rs:16-23` 那条取舍）；选项用 `Checkbox` 而不是真机的下拉菜单（见下）。**Replace in Files 的实做**：不用 `Table`，用 `v_flex().overflow_y_scrollbar()` 画"文件头 + before/after 行"（同样是"行不等高"那条取舍），四颗选项用 `Button::selected` + `disabled`（等价于真机那颗 `ProjectReplaceOptionButton` 的 `isOn` / `isEnabled`，`ProjectReplaceView.swift:359-395`），浮层用 `Dialog`（照 `search_everywhere.rs`）。
- **缺口与优化**：✅ **Find in Files 的界面已落地**（U17 第一件）。控件 ↔ Core 字段一一对应，出处逐条写在 `crates/workbench/src/search_model.rs` 的 `payload` 上：查询框 → `query`、掩码框 → `fileMask`、三个开关 → `caseSensitive` / `wholeWords` / `regularExpression`、`maxResults` 显式传 200（与 Core 的 `default_max_results` 同值）；**故意不传** `hiddenDirectoryNames` / `hiddenFilePatterns`（与 `searchIndex.warm` 共用同一个索引缓存键）与三个 `max*Results`（留空由 Core 回落到 `maxResults`）。`operationId` 按代次生成（`lithe-gpui-search-<n>`）进信封，输入变化即换代并调 `lithe_gpui_shared::cancel_operation`（Core 的协作式取消），旧代次结果**一律丢弃**并留 `S1_SEARCH stale generation=…`；防抖 250ms 与真机同值（真机 `SearchSidebarView.swift:142`）。空结果（`ready` + 0 行）与失败（`failed`）是**两个不同状态**。人工核对锚点（stderr）：`S1_SEARCH issue/complete/stale/failed/cancel/open/option/idle`。
  ✅ **索引侧现状（U18，未改动）**：`searchIndex.warm/update/invalidate` 三条的时机 —— `warm` = 项目打开（`workspace.rs` 的 `ShellWorkspace::new`，后台 `background_spawn`，不阻塞 UI）；`update` = 保存写盘（`editor_view.rs` 的 `BufferWriteHost` → `workspace.rs` 的 `note_buffer_written`，只带那**一条**变更路径，不退化成全量 invalidate）；`invalidate` = 换根（`rebuild_project_window` 的 `replace_root` 之前）。索引本体在 `project/search_index.rs`（内存索引）。人工核对锚点（stderr）：`S1_SEARCH_INDEX <op> root=… files=… symbols=… postings=… rebuilt=…`。⚠️ **`update` 只能由保存驱动**：宿主没有文件系统 watcher（`H12`），Git 的写盘路径（discard / checkout）与"项目结构变化"都没接 —— 外部改动要等下一次保存或重开项目才进索引；gpui 也没有"关闭项目"这条生命周期，所以 `invalidate` 只有换根一个挂点。
  ⚪ **四件套里最后一块"应用"没做，而且在本分支做不了**：Replace in Files 的**应用替换**要等 main —— Core 与替换有关的命令**只有** `workspace.replacePreview`（`rust/lithe-core/src/protocol/command.rs:77-78,362`），**没有** apply / replace 命令，所以宿主没有可调的通路（宿主自己写盘会绕开 Core 的 `VisibilityRules` 与 `parse_file_mask`，是第二个真相源）。界面上的表达是**不画「全部替换」那颗按钮** —— 真源同样没有：`ProjectReplaceView.swift:158-185` 的底栏只有 `Preview` 与计数，"应用"在另一条通路（`ProjectReplacementCoordinator.apply`，`macos/Sources/Lithe/Application/Features/ProjectReplacementCoordinator.swift:60-81`）。**搜索历史也没有持久化**（查询与结果不入 `.lithe/session.local.json`）。
  ✅ **Quick Open 已做（U17 第二件，2026-10-01）**：`Ctrl+P` 打开的**文件快速打开浮层**，落在 `crates/workbench/src/quick_open.rs`（视图：`Dialog` + `Command`，形状照 `command_palette.rs`）+ `crates/workbench/src/search_model.rs`（**同一个纯逻辑层**新增 `EVERYWHERE_COMMAND` / `quick_open_rows` / `QuickOpenState`，与 Find in Files 共用 `payload` / `parse_matches` / `QueryTimeline`）。请求是 `workspace.searchEverywhere`（`files.rs:272`，收的就是 `workspace.search` 那个 `SearchRequest`），宿主只取 **Files scope**（`kind: "file"`，macos 的取法是 `SearchEverywhereView.swift:132-133`）并**保持 Core 的顺序** —— 没有第二套文件名匹配 / 模糊排序（`Command` 的本地过滤显式关掉：`.filterable(false)`，`command/command.rs:117-126`）。键盘：`Ctrl+P` 进（`quick_open.rs` 的 `install_actions`，全局 action，`None` context）、`↑`/`↓` 选择（环上组件行为：到头**环绕**，真源是夹住）、`Enter` 打开选中文件（走既有 `EditorPane::open`，Core 的路径命中 `line` 恒为 `null`）、`Esc` 关（先清空查询、再关，同命令面板）。跨帧状态：代次闸门 `QueryTimeline`（旧结果一律丢，留 `S1_QUICK_OPEN stale`）+ 在飞 `operationId` 交给 `cancel_operation`（换查询 / 关浮层两条路径都取消）；输入防抖 **180ms**（`SearchEverywhereView.swift:212`）、上限 200（`MAX_RESULTS` = 真机 `matchLimit`，`SearchResults.swift:58`）。菜单「转到 → 快速打开」从**占位项转正**并 dispatch **同一个** `Action`（`MenuAction::QuickOpen` → `CommandId::OpenQuickOpen` → `ShellWorkspace::open_quick_open`）。**零新增文案**（复用 `quickOpen.searchFiles` / `quickOpen.noMatch` / `menu.quickOpen`）。人工核对锚点（stderr）：`S1_QUICK_OPEN open/idle/issue/complete/stale/failed/cancel/open_file/close`；⚠️ **未经真机视觉验证**（本轮只跑了确定性单测与守卫脚本，没有截图）。**Replace in Files 的预览已做（U17 第四件，见下）；应用替换仍未做。**
  ✅ **Search Everywhere 已做六个 scope + `/` 命令模式（All / Classes / Files / Symbols / Actions / Text；分三批做成 —— U17 第三件 2026-10-01 先做四档，后续切片补 `Classes`、`Actions` 与 `/`，见本条下面两段）**：菜单「视图 → 全局搜索」打开的浮层，落在 `crates/workbench/src/search_everywhere.rs`（视图：`Dialog` + `Command`，形状照 `quick_open.rs`/`command_palette.rs`；scope 标签行自绘）+ `crates/workbench/src/search_model.rs`（**同一个纯逻辑层**新增 `SearchEverywhereScope` / `everywhere_rows` / `EverywhereState` / `EVERYWHERE_MAX_RESULTS`，三件共用 `parse_matches` / `QueryTimeline`，与 Find in Files 共用 `payload` 的**同一个 `json!`**，见 `payload_with_limit`）。**scope ↔ Core `kind`**：Files = `kind=="file"`、Symbols = `kind=="symbol"`、Text = `kind=="content"`、**All = file + type + symbol 三段拼接（不含 content）** —— 最后一条是真源明写的取舍（正文命中只在 Text 出现，避免与 Find in Files 重叠，`SearchEverywhereView.swift:122-129`）。`type` 行在 All 里出现，但 **Classes 这个 scope 当时不在本切片**（后续切片已补，见本条的下一段）；**Actions 也不是工作区搜索**（那是宿主命令目录）；**`/` 命令模式**同样没做。因此本切片把 `search_model` 的 `MatchKind` 从两个取值泛化到四个（`file`/`content`/`type`/`symbol`，出处 `files.rs:223,254,976-981,993-998`）并**恢复 `Match.symbol_name`** —— 旧版显式丢掉它，符号行只能画出文件名；Find in Files 与 Quick Open 的既有行为与单测全部不变。**`maxResults` 显式抬到 `EVERYWHERE_MAX_RESULTS`（1000）**：Core 把 file+content 与 types+symbols 拼成一个数组再统一 `truncate(maxResults)`（`files.rs:326-332`），而 file+content 自己就能顶满（`files.rs:203,211,236`）⇒ file/content 的上限来自 `maxResults`、符号上限固定 50（`files.rs:295`），所以抬高预算就是唯一的宿主侧绕行（B10；**不是**用宿主截断去"修"它）。键盘：菜单进（**没有键位** —— 真源是双击 Shift，gpui 侧没有这条通路，空闲的 `Ctrl+*` 也没有一条有真源依据，所以不硬造组合键）、`Tab`/`Shift+Tab` **环绕**切 scope（真源 `:526-529,542-547`；靠本浮层自己的 key context，不与命令面板抢键）、`←`/`→` **不绑**（上游单行 `Input` 无条件处理它们做光标移动，`gpui-base-0.7.0/src/input/base/state.rs:4454-4457`，与真源的全局 NSEvent 监听是有意差异）、`↑`/`↓` 选择、`Enter` 打开（有 `line` 的走 `EditorPane::open_at`、`origin_kind="search-everywhere"`；路径命中走 `open`）、`Esc` 关（先清空查询，同命令面板）。跨帧状态：代次闸门 + 在飞 `operationId` 取消（与另两件同源）、防抖 **180ms**（`SearchEverywhereView.swift:212`）。文案新增 8 条 gpui 自有键（四个 scope 名 + 四句空态，`GPUI_ONLY_KEYS` 里逐条写了 macOS 出处），菜单项复用真源既有键 `menu.globalSearch`；同一次改动**删掉**了与事实不符的 `gpui.menuMissing.indexer`（「还没有索引器」—— 索引链 U18 就通了，缺的从来不是索引器），`MISSING_GROUPS` 17 → 16、菜单计数 42/50 → **45/47**（当时口径；第四件「在文件中替换」再 +1 ⇒ 现 **46/47**，合计 94 条目 = 真源 89 + 本侧新增 5。Action/占位，两条真源项在 U17 里转正）。人工核对锚点（stderr）：`S1_SEARCH_EVERYWHERE open/scope/idle/issue/complete/stale/failed/cancel/open_result/close`（`complete` 那行按 scope 报可画行数，`symbols=` 是否为 0 就是 B10 有没有绕开的判据）；确定性单测覆盖四个 scope 的投影、All 不含 content、符号行带 `symbolName`、空结果与失败之分、代次与环绕公式。⚠️ **未经真机视觉验证**（只有结构性核验与纯函数单测）。
  ⚠️ **Search Everywhere 与真源的有意差异（4 条）**：(1) **没有相关度重排** —— 真源 All 会按 `SearchRelevance.score` 重排（`SearchEverywhereView.swift:11-90,158-170`），而 Core 响应没有 relevance 字段（`contracts.rs:94-105`），在宿主里重排等于写第二套算法 ⇒ 本切片**沿用 Core 的固定顺序**（它本身确定），重排等 Core 给字段（main 那一侧）；(2) **`Actions` 档只可能显示子集**：宿主命令目录 `CommandId::ALL` 是 **22** 条，真源 `LitheCommandCatalog.swift:129-182` 是 **39** 条（`LitheActionRegistry` 的动作是 35 条）；`Classes` 后续已补；(3) **`←`/`→` 不切 scope**（上游单行输入占用，见上）；(4) scope 标签行**没有用上游 `TabBar`**：它的回调只给下标、标签元素 id 由上游按下标发（`gpui-component-0.7.0/src/tab/tab_bar.rs:171-179,506-537`），与"`ElementId` 用业务身份"冲突，而本侧只有六个固定标签。另外**`tab` 键与 gpui-base 根节点的 focus traversal 绑定同键**（`gpui-base-0.7.0/src/root.rs:16-17`），本浮层用更深的 key context 覆盖它 —— 浮层内 `Tab` 只切 scope、不跳焦点（与真源一致）。
  ✅ **Search Everywhere 补上 `Classes` scope（U17 后续切片，2026-10-03）**：真源 `allCases` 的顺序是 `All` / `Classes` / `Files` / `Symbols` / `Actions` / `Text`（`SearchEverywhereView.swift:5-14`），`Classes` 映射的是 `classMatches`（同文件 `:130-131`），也就是 Core 的 `kind == "type"`（`rust/lithe-core/src/project/files.rs:312-315` 把类型单独攒进 `types`）—— 与 `Symbols`（`kind == "symbol"`，`files.rs:296-320` 的另一段）是**两种 kind**，不是同一个 kind 的两个视图。改动只在 `crates/workbench/src/search_model.rs`：`SearchEverywhereScope` 增一个 `Classes` 变体、`ALL` 由四项变五项（`Classes` 排**第二**，相对顺序逐格照 `allCases`、只去掉本侧没做的 `Actions`）、投影为 `named_rows(matches, MatchKind::is_type)`（复用既有谓词，**没有第二套匹配 / 排序 / 上限**）、加 `scope.classes` 与 `noMatches.classes` 两个文案键；`search_everywhere.rs` 的标签行本来就遍历 `SearchEverywhereScope::ALL`，所以 Tab / `Shift+Tab` 环绕、scope 计数与诊断行的 `classes=` 自动跟上，**视图代码零改动**。**`All` 的口径没有动**：仍是 `file + type + symbol` 三段拼接、仍**不含 `content`**（真源 `:122-129` 的取舍，既有单测继续钉住）；`type` 行本来就同时出现在 `All` 与 `Classes` 里。文案：新增 2 条 gpui 自有键（`searchEverywhere.scope.classes`「类」/ `searchEverywhere.noMatches.classes`「类中没有匹配项」，macOS 出处 `Localizable.strings:884,1166`，`en.lproj` 无对应条目 ⇒ en 逐字取真源字面量），两份 yml 由 `extract-locale.mjs` 重新生成、`shared/src/i18n.rs` 的 `WIRED` 同步接线（该浮层的 gpui 自有键 8 → 10 条）。单测：新增 `classes_scope_only_projects_type_rows_without_changing_all`，并把 `scopes_cycle_with_wrap_around_in_both_directions`（钉住五项与标签顺序）与 `scopes_project_the_core_kinds_exactly`（补 `Classes` 一段）一起更新。⚠️ **未经真机视觉验证**（本轮只有确定性单测 + `check-ui-px.mjs` / `extract-locale.mjs --check` 两个守卫脚本）。
  ✅ **Search Everywhere 补上 `Actions` scope 与 `/` 命令模式（U17 后续切片，2026-10-03）**：真源 `allCases` 的顺序是 `All` / `Classes` / `Files` / `Symbols` / `Actions` / `Text`（`SearchEverywhereView.swift:5-14`），本切片把 `Actions` 补上 ⇒ `SearchEverywhereScope::ALL` 由五项变**六项**，`Actions` 排**第五**（在 `Symbols` 之后、`Text` 之前），Tab / `Shift+Tab` 环绕公式由 `mod 5` 变 `mod 6`。**行的来源分两种，判据是一个纯函数**：`All`/`Classes`/`Files`/`Symbols`/`Text` 走 Core `workspace.searchEverywhere` 的命中投影（`everywhere_rows`），`Actions` 档与 `/` 命令模式走**宿主命令目录**（`ShellWorkspace::action_catalog` → `search_model::action_rows`），真源出处 `SearchEverywhereView.swift:59-63,138-139`。**`Actions` 档（与 `/` 模式）一个 Core 请求都不发**：判据是 `search_model::{uses_command_catalog, issues_core_request}` 两个纯函数，浮层的 `on_query`（查询变了）、`issue`（防抖到期）、`set_scope`（切档）三条路径**都只看它**；进 `Actions` 时还会撤掉在飞的 Core 请求并取消它，切回 Core 档时按 `EverywhereState::is_ready_for` 判断要不要补一次（同一个查询串且已有 `ready` 结果就直接复用）。**动作不来自第二份目录**：候选就是命令面板那张表（`CommandId::ALL`，22 条，与 `command_action` 同一投影 ⇒ 同一个动作在两个入口里的字面与字形逐字相同），行型直接复用 `CommandAction`；匹配器 `action_matches` 逐条照真源 `LitheAction.matches`（`LitheAction.swift:37-51`：去空白 + 转小写的**子序列**匹配，候选串 = `label + description + 分类名`，空 query 恒真）—— **为什么只能落在宿主**：Core 的匹配只认工作区内容，而命令面板的谓词 `item_matches` 是**私有** `fn`（`gpui-component-0.7.0/src/command/state.rs:332`），宿主拿不到；这不是"第三套匹配器"，三处判据来源不同、取值域不重叠（详见 `search_model.rs` 的模块文档）。`Enter` 走**注入回调**：动作行没有路径，浮层只报 `CommandId::id()`（`on_action`，照 `on_open` 的口径），由外壳落到唯一执行点 `ShellWorkspace::run_command_id`（菜单与命令面板走的也是它），且**先关浮层再放行动作**（选中的动作里有「命令面板 / 全局搜索」这类会再开对话框的）。`/` 命令模式照真源：`query.hasPrefix("/")` → `dropFirst()` 当命令查询串（**`/` 覆盖 scope**），空态用真源那句 "No matching commands"（`:77-78`），查询框为空时的提示换成真源那句 "Type / to see commands"（`:266-270`）；真源在 `/` 模式还会发一次空查询（`:207-211`），本侧**一次都不发**（Core 对空查询直接回空）。**`All` 的口径没动**：仍是 `file + type + symbol` 三段、仍**不含 `content`**（真源 `:122-129`），既有单测继续钉住；`everywhere_rows(Actions, …)` 恒为空（`Actions` 不投影 Core 命中）。文案：新增 4 条 gpui 自有键（`scope.actions`「操作」/ `noMatches.actions`「操作中没有匹配项」/ `noMatchingCommands`「没有匹配的命令」/ `typeSlash`「输入 / 查看命令」，macOS 出处 `SearchEverywhereView.swift:10,48,77-78,87-88,267` → `Localizable.strings:48,886,1139,1169`，`en.lproj` 无对应条目 ⇒ en 逐字取真源字面量），两份 yml 由 `extract-locale.mjs` 重新生成、`shared/src/i18n.rs` 的 `WIRED` 同步接线（该浮层的 gpui 自有键 10 → 14 条）。单测新增 6 条（匹配器五类用例 / 目录投影的顺序与字段 / `/` 模式判定 / **Actions 与 `/` 都不发 Core** / Actions 不投影 Core 命中 / 切回 Core 档的复用判据）+ workspace 侧 1 条（`CommandId::ALL` 覆盖整张命令表）+ **交互层 1 条**（`search_everywhere::tests::the_actions_scope_and_command_mode_never_start_a_core_query`，`#[gpui_kit::test]` 在无头窗口里跑真浮层：Actions 档敲字与 `/` 模式下 `issue` 都不排防抖、不进 `Loading` —— 即"没走到 `CoreClient::execute`"，对照组先在 Core 档证明防抖真的会排），并把 `scopes_cycle_with_wrap_around_in_both_directions`（钉住六项与标签顺序）与 `scopes_project_the_core_kinds_exactly`（补 Actions 恒空）一起更新。诊断：`scope` / `complete` 行带 `core=`，新增 `actions … rows=… core=false` 与 `run_action id=…` 两行（stderr）。⚠️ **`Actions` 档是子集**（22 vs 真源 39）；⚠️ **未经真机视觉验证**（本轮只有确定性单测 + 无头窗口里的交互测试 + `check-ui-px.mjs` / `extract-locale.mjs --check` 两个守卫脚本，**没有截图**）。
  ⚠️ **`Actions` 档与真源的有意差异（4 条）**：(1) **没有 keyEquivalent 列** —— 真源动作行右侧画快捷键（`:480-484`），本侧没有"命令 id → 可显示快捷键文本"的通路（键位是 `cx.bind_keys` 里登记的行为，不可反查），硬编一张表就是第二份键位真源；(2) **图标用动作自己的字形，不是统一的 `bolt.fill`**：真源的 `LitheAction` 不携带图标所以才全员 bolt（`:457-465`），本侧动作来自宿主目录、每条自带与命令面板同一个图标；(3) **分类列是宿主自己的两类**（`Category::{Settings, View}`）而不是真源的五个 `LitheActionGroup`（run / navigation / window / project / history，`LitheAction.swift:3-9`）—— 扩分类要动命令面板的选择面与图标映射，不在本切片；(4) **`Actions` 档空查询时不列动作**：真源 `hasQuery` 为假时整个结果区不画（`:172-174,187`），本侧照办 —— 想"浏览全部命令"就敲 `/`（`/` 后面为空 ⇒ 空 query 恒真 ⇒ 列出全部 22 条）。另外**不做"最近使用置顶"与分组标题**（真源也没有）。
  ✅ **Replace in Files 的「替换预览」已做（U17 第四件，2026-10-01）**：`Ctrl+Shift+R` / 菜单「转到 → 在文件中替换」打开的浮层，落在 `crates/workbench/src/replace_in_files.rs`（视图：`Dialog` 浮层 + 查询框 + 替换文本框 + 文件掩码框 + 四颗选项 + **按文件的 before/after 列表** + 四态 + 底栏「预览」按钮与计数）+ `crates/workbench/src/search_model.rs`（**同一个纯逻辑层**新增 `REPLACE_PREVIEW_COMMAND` / `ReplaceOptions` / `replace_payload` / `ReplacementFile`·`ReplacementLine` / `parse_replace_preview` / `ReplaceRow` / `replace_rows` / `ReplaceState`，与另三件共用 `SearchOptions` 的四个字段、`ParseError`、`QueryTimeline`）。**形状选浮层而不是侧栏**：真源自己就是浮动面板 —— `ProjectReplaceView` **只**由 `ProjectReplaceFloatingPanel` 包着用（`WorkbenchView.swift:589` 写明"Replace in Files 是工作台上的自绘模态层"），而 `SearchSidebarView` 是 Find in Files 的侧栏形态，所以照 `search_everywhere.rs` 的 `Dialog` 写法、**不**挂进左活动栏。**控件 ↔ Core 字段**：查询框 → `query`；替换文本框 → `replacement`；掩码框 → `fileMask`（空串 = 不过滤，所以**不画**真源那颗 "File mask:" 勾选框 —— 它只是把一份非空掩码临时关掉，清空输入框是同一个效果）；区分大小写 / 全字匹配 / 正则 / 保留大小写 → `caseSensitive` / `wholeWords` / `regularExpression` / `preserveCase`（前三条字段与查找面板同源）；**故意不传** `paths` / `textOverrides`（真源送的是编辑器里未保存的缓冲区，`AppModel+SearchModule.swift:53-58`；gpui 没有这条通路 ⇒ 预览的是磁盘内容）、不传可见性数组（同一个索引缓存键）、**没有任何上限可传**（契约里没有 `maxResults` 字段，`files.rs:110-136`）。**before/after 逐字来自 Core 的响应**（`contracts.rs:114-131` 的 `ReplacementMatch.before/after` + 1 基 `line` + `occurrenceCount`），宿主**不做任何 diff / 匹配 / 高亮 / 排序**；`replacementText`（整份替换后正文）**故意不解析** —— 那要真源下半那块 Monaco 预览（`ProjectReplacementSourcePreview.swift:14-45`），gpui 侧没有对应元素。**没有防抖**：真源要**点「预览」**（`ProjectReplaceView.swift:160-165`），输入一变只作废上一次预览（`:57-68`）—— 所以"一次可能扫整个工作区"只在用户明确要求时发生；两个输入框里的回车等于点预览。跨帧状态：代次闸门 `QueryTimeline`（旧结果一律丢，留 `S1_REPLACE_PREVIEW stale`）+ 在飞 `operationId` 交给 `cancel_operation`（换输入 / 改选项 / 关浮层三条路径都取消）；空结果（`ready` + 0 文件）与失败（`failed`，Core 的 `code: message` 或契约异常的稳定 token）是**两个不同状态**。入口：菜单项 dispatch **同一个** `Action`（`MenuAction::ReplaceInFiles` → `CommandId::OpenReplaceInFiles` → `ShellWorkspace::open_replace_in_files`），键位 `Ctrl+Shift+R` **有真源依据**（真源的 Windows 平台表 `LitheCommandCatalog.swift:61` 就是 `Ctrl+Shift+R`；菜单项在 `LitheApp.swift:624-627`），本侧菜单从 **89 + 4** 变成 **89 + 5**（`LOCAL_ONLY_ITEMS` 4 → 5、Action 45 → 46）。**新增 7 条 gpui 自有文案键**（`search.gpui.{replaceInFiles,replaceEmptyDescription,replacePreviewCounts,before,after,preserveCase,preview}`，`GPUI_ONLY_KEYS` 里逐条写了 macOS 出处 —— 其中"在项目中替换"逐字取自 `macos/Resources/zh-Hans.lproj/Localizable.strings:65`、"预览"取自 `:837`），其余全部**复用真源既有键**（`search.replaceWith` / `search.filesToInclude` / `search.matchCase` / `search.matchWholeWord` / `search.useRegex` / `search.failed` / `search.noResultsFor` / `search.gpui.queryPlaceholder` / `fileExplorer.searchingFiles` / `menu.go`）。人工核对锚点（stderr）：`S1_REPLACE_PREVIEW open/close/idle/issue/complete/stale/failed/cancel/option`；⚠️ **未经真机视觉验证**（本轮只跑了确定性单测与三个守卫脚本，没有截图）。
  ⚠️ **Replace in Files 与真源的有意差异（6 条）**：(1) **不画下半块的文件预览编辑器**（Monaco，见上）⇒ before/after 是画在列表行里的；(2) **不做查询高亮**（真源给 `before` 里的命中加黄底，`ProjectReplaceView.swift:121-152`）—— 在宿主里找命中等价于写第二套匹配器，Core 也没给命中区间；(3) **行不可点**：真源点一行只是把下半块编辑器切过去（`:261-266`），本切片没有那块，画点击态就成了"点了没反应"；(4) **`Preserve Case` 在正则打开时置灰**（真源置灰的判据是"勾了区分大小写"，`:119-123`）—— Core 的实际语义是正则那一支根本不读 `preserveCase`（`project/files.rs:890-901` 先 `return`），而大小写敏感时**照样**应用（`:937-941`），所以按 Core 的行为置灰；(5) **不传 `paths` / `textOverrides`**（见上）；(6) **面板不可拖 / 不可缩放**：真源能拖动与四角缩放（`ProjectReplaceFloatingPanel.swift:23-53`），gpui-kit 的 `Dialog` 没有这两个交互，本侧尺寸照真源的 650×614 居中显示。
  ⚠️ **两处有意偏离 macOS**：(1) 三个选项画成纵向 `Checkbox`，而不是真机那颗滑块图标的下拉菜单（侧栏宽度可拖到 200px，三个开关文案一行放不下，且不引入浮层）；(2) 面板没有 macOS 那行 44px 的「Search」标题（与维护者 2026-09-27 删掉资源管理器标题行同一口径；IDEA 的 Find in Files 也没有标题）。macOS 的替换按钮（`arrow.left.arrow.right`）按范围**不画**。
  ✅ **"索引就绪"状态已落地（U18 收尾，2026-10-03）**：`crates/workbench/src/search_index.rs` 的 `SearchIndexWiring` 新增 `SearchIndexReadiness`（`pub(crate) enum`：`Pending` / `Ready` / `Failed`），**唯一 owner 就是那条链**（四个界面都不另建索引状态 —— 第二个真相源）。状态机：`new`（项目打开 / 换根后的新外壳）起于 `Pending`；`warm` 的 `adopt` **过了代次闸门**之后落 `Ready`（`Ok(_)`，含 `ok:true` 但无 `data` 的计数字段契约异常）或 `Failed`（`Err(_)`）；`update` 的采纳与代次不匹配**一律不改**状态；`schedule_invalidate`（换根）也不动它 —— 旧外壳整个被丢掉、新外壳天然从 `Pending` 起。**怎么流到界面**：`warm` 任务回到前台时 `adopt` 返回"就绪状态是否真的变了"，为真才 `ShellWorkspace::broadcast_search_index_readiness`（`workspace.rs`）**推**一次 —— 常驻的左栏面板直推，三个浮层经外壳持有的 `WeakEntity` 句柄推（**只在开着时** `upgrade` 成功，关掉的浮层自动跳过、不需要"关闭时清句柄"的记账）；浮层打开时另外在 `open_quick_open` / `open_search_everywhere` / `open_replace_in_files` 里读一次**当刻值**建实例。**口径**：不轮询、不在 `render` 里 mutate/notify/起 task（界面的 `render` 只读自己那一个字段）、界面只读不判；四个界面共用 `search_index.rs` 的同一处判据 `SearchIndexReadiness::notice_key()` 与同一个元素工厂 `readiness_notice()`，未就绪时在控件区与结果区之间多画一行灰字（**不挡输入、不禁用搜索、不排队、不重试、不做进度百分比、不取消 warm**），`Ready` 后整行消失。**文案**：新增两条 gpui 自有键 `lithe.search.gpui.{indexWarming,indexWarmFailed}`（真源与两份 catalog 都没有"第一次搜索可能慢"这类**可解释性**说法；最接近的既有键 `quickOpen.indexing`「正在索引项目文件」只陈述状态、且属 Quick Open 列文件那条路径，故不借 —— 逐条理由见 `extract-locale.mjs` 的 `GPUI_ONLY_KEYS`）；**失败那句与"还在构建"分开**并明写"搜索本身仍然可用"，**不**说成致命错误。人工核对锚点（stderr）新增一行 `S1_SEARCH_INDEX readiness={pending|ready|failed} root=…`（只在状态**真的变了**时打）。⚠️ **未经真机视觉验证**（本轮只有确定性单测：状态机 5 条 + 判据 2 条 + 既有 5 条全绿，workbench 140 → 147 单测 + 1 集成测试全绿）。
  ⚠️ **这条链上仍然可做的**：就绪状态只如实报"宿主已经把 `warm` 发出去、以及它怎么了"，**没有进度百分比**（Core 不报进度）、**没有取消 warm**，也没有改变"第一次搜索要等索引锁"这件事本身（那属于 Core 的 `get_or_build`，`project/search_index.rs:80-93`）。另外宿主侧仍缺文件系统 watcher（`H12`）：`update` 只由保存驱动，Git 的写盘路径（discard / checkout）没接。
  ⚠️ 取消是**尽力而为 + 有代价**的：warm 之后取消纯收益，但**取消一次正在建的索引**会让 Core 的 `get_or_build` 带着取消标记返回且**不缓存**，下一次查询要重建 —— 项目刚打开就连续输入时可能出现。
  ⚠️ 结果契约只有 `kind/path/line/preview/symbolName`，**没有列号/字节范围**，所以精确高亮与逐命中替换做不了（要么改契约，要么近似）；本面板点击结果只跳到**行首**（`utf16_column` 传 0）。**这一条只约束 `workspace.search` / `workspace.searchEverywhere`**：`workspace.replacePreview` 另有一套契约，它**给**的是整行级的 `before` / `after`（`contracts.rs:114-131`）—— 所以替换预览画得出前后对照（U17 第四件），但**列级**的命中位置它同样不给（真源那块查询高亮本侧就没做）。
  ⚠️ ~~菜单文案说"还没有索引器"与 Core 事实不符。~~ **已修**（U17 第三件）：菜单「视图 → 全局搜索」转正为 `MenuAction::SearchEverywhere` → `CommandId::OpenSearchEverywhere`，那句 `gpui.menuMissing.indexer`（连同它所在的整组 `missing` 声明）已删除 —— 索引链 U18 就通了，缺的从来不是索引器。

#### ⑭ 数据库

- **macOS 组成**：连接侧栏（连接列表 + 连接编辑器）+ SQL 工作台（编辑器 + 结果表 + 标签页）+ 表视图 + schema diff 视图 + Redis 专用工作台 + Nacos 专用工作台。
- **关键类型**：`DatabaseSidebarView`（1797 行）、`DatabaseConnectionEditor`、`DatabaseSQLWorkspaceView` / `DatabaseWorkspaceView`、`DatabaseTableView`、`DatabaseSchemaDiffView`、`RedisWorkspaceView`、`NacosWorkspaceView`、`DatabaseTransferDocument`。
- **gpui 落点**：**无**。只有菜单里一条明确标为未接线的灰项（这本身是**正确的诚实做法**，应保留）。
- **组件替换**：[建议] 连接树用 `Tree`，结果集用 `DataTable`（大数据集需要虚拟化），SQL 编辑用 `input::Editor` + SQL 语法（gpui-kit 有 `tree-sitter-sql`），schema diff 用两栏对照 + `resizable`。
- **缺口与优化**：❌ **Core 的 `database.*` 命令是 0 条**（`protocol/command.rs` 里一个都没有）。但要看清一件事：**引擎已经在仓库里** —— `rust/lithe-db-sidecar`（4045 行 + MongoDB 适配 510 + SQL Server 适配 658）是 workspace member，协议与 macOS 期望的**同构**（一次性进程，stdin 收 `Request{id,method,params}`、stdout 回 `Response`），`capabilities` 自报 `protocolVersion: 1` 与**完全相同的 8 种库**（mysql / mariadb / postgresql / sqlite / sqlserver / mongodb / redis / nacos），方法集覆盖 macOS 的全部操作。**只是没有任何 gpui crate 依赖它**（CI 有独立 lane，产物不进应用）。🚧 **前置阻断**：开工前必须先做 sidecar 与 macOS `DatabaseSidecarService` 的**逐字段 diff**（`ssl` / `caCertificatePath` / `serverName` / `ssh*` / `proxyURL` / `readOnly` / `productionProtection` / `allowWrite` / `confirmed`），不 diff 不能接线。❌ 另有一批**确定性逻辑只写在 macOS 一侧**（本应属 Core）：SQL 语句切分与风险分级（无 WHERE 的写操作、DROP/TRUNCATE 确认）、schema diff → 迁移 SQL 生成、DBX 连接导入解析。

#### ⑮ AI 助手 / Agent

> 这是**两件独立的事**，不要混着排期。

**⑮-a AI 提交信息 / PR 描述**

- **macOS**：设置在 Git 分类下有 AI 子页；提交区有"用 AI 生成提交信息"；PR 描述生成。
- **gpui 落点**：**无**。设置页**故意没有 AI 分类**（有守卫测试钉着这一条）；提交区只有一个占位 `TextareaState`，没有生成按钮；无 HTTP 客户端、无凭证存储、无 provider 配置界面。
- **缺口与优化**：Core 有**完整的 typed AI API** 且**按设计不加 JSON 命令**（`rust-core-api.md` 明说 "It does not add a JSON command or change the C ABI"）：`plan_commit` 规划 URL+body、三种线协议编码、输出 token 预算与字段名可切、URL 校验、选项范围校验、diff 按文件数均分预算、**敏感文件拒发**（`.env` / `credentials.json` / `id_rsa` / `.pem` / `.key` 等）、`decode_message` 三信封解析去围栏、`parse_codex` / `parse_claude`。**唯一可调用的命令是 `agent.parseProviderConfiguration`。** ⚠️ 所以 gpui 要做的**不是重写 prompt 与解析**（那部分 Core 已有，且与 macOS 语义一一对应），只补三件事：HTTP 传输（宿主）、凭证存储（宿主）、设置持久化（宿主）。⚠️ 缺 **PR 描述生成的规划函数**（Core 只有 commit 的 `plan_commit`；macOS 有对应的 prompt 构造）。⚠️ `shared/contracts/ai-commit.md` 里「Windows adapter surface」整节教集成方走已删除的 `windows/tauri` 命令，**该节已作废**，需改写为"宿主直接链接 `lithe_core::ai`"。

**⑮-b Agent 对话面板**

- **macOS**：右栏常驻面板（未配置态 / 内嵌设置页切换）+ 面板头（新建会话 / 历史 / 设置）+ 会话页签与标题 + transcript（相邻工具调用合并、搜索、hero、活动汇总、工具证据、Markdown 分段）+ Composer（上下文条与用量、附加文件、拖放、Esc 取消）+ 文件引用列表 + 工具分组视图 + 历史视图（筛选 / 搜索）+ 面板内设置（agent 列表 / preflight 检查 / 一键修复 / ACP 徽标）+ Provider 配置 + 会话选择器（mode / model / ACP configOptions）+ 上下文用量 + 订阅额度 + 回合统计。
- **关键类型**：`AgentConversationView`、`AgentSessionTabStrip`、`AgentTranscriptView`（`AgentTranscriptItem` / `AgentHeroView` / `AgentActivitySummaryBar` / `AgentToolEvidenceView`）、`AgentComposerView`、`AgentFileReferenceList`、`AgentToolGroupView`、`AgentHistoryView`、`AgentPanelSettingsView`、`AgentProviderConfigurationView`、`AgentSessionSelectors`、`AgentContextUsageView`、`AgentSubscriptionQuotaView`、`AgentTurnStatisticsView`。
- **gpui 落点**：**无**（全仓只有两处无关的 "Agent" 字样命中）。
- **组件替换**：[建议] transcript 用 `message_scroller` + `list`（虚拟化）；代码块与 Markdown 用 `text_view` + 渲染；Composer 用 `input` + 附件条；工具分组用可折叠 `v_flex`；长任务进度用 `Progress`。
- **缺口与优化**：❌ **引擎已经在 Rust 里**：`rust/lithe-agent-host` 是完整的 ACP 会话引擎（`AgentHandle` 持有一个 agent 进程 + 一条 ACP 连接），已通过 Core 的 C ABI 暴露（`lithe_agent_open_json` / `lithe_agent_send_json` / `lithe_agent_close`），并有 4 条 JSON 管理命令（`agent.status` / `install` / `installCli` / `uninstall`）与 `agent.parseProviderConfiguration`，契约与事件 fixture 都在。**缺的只有 gpui 界面 + 一个 Rust-native 调用面**：`lithe-core` 里 `mod agent` 与 `mod runtime` 都是**私有**，也没有 `pub use lithe_agent_host::*`；而 `lithe-agent-host` **不是任何 gpui crate 的依赖** —— 所以两条路（加直接依赖 / 调私有 ffi）**都还没铺**。🚧 **前置阻断（架构决策）**：ACP 是**长连接 + 事件流**，而 gpui 的 `CoreClient` 是**同步信封**（调用方负责放进后台任务）。仓库里已有同类前车之鉴（`java/src/events.rs` 的"单一事件泵 + 按 operationId 分派"）。**接 Agent 之前必须先定这个异步边界**，否则会重演同一类 bug。⚠️ 不要照抄 Swift 的 `@Published` + `Combine` + 手写批处理；GPUI 侧应用 `Entity` + `cx.spawn` + 显式状态枚举。

#### ⑯ 可复用组件层

- **macOS 组成与关键类型**：`LitheSplitPaneView` + `LitheSplitPaneGeometry`（分栏与几何）、`SplitHandleView`（拖拽手柄）、`LitheToolWindowHeader`（工具窗头部三段式）、`LitheUnavailableView`（统一空态）、`LitheContextMenu` + `LitheContextMenuPresenter`（上下文菜单）、`LitheSettingsControls`（搜索框 / 下拉 / 分段控件 / 复选框 / 步进器）、`LitheScrollViewChrome` + `LitheScrollWheelRouting`（滚动外观与滚轮路由）、`LitheTextViewportLayout`（文本视口布局）、`WorkbenchHoverTooltip*`（悬停提示）、`FrameCoalescedDragUpdateBuffer` + `LitheDragUpdateScheduler`（**按帧合并的高频拖拽更新**）、`WorkbenchSurfaceBackground`。
- **gpui 落点**：**没有独立组件层** —— 直接用 gpui-kit + 各 crate 内部的小工具。这是**有意的架构选择**，不是遗漏。
- **组件替换对照**：

| macOS 自绘 | gpui-kit 直接给 |
| --- | --- |
| `LitheSplitPaneView` + `SplitHandleView` | `resizable` / `resize_handle` / `resize_handle_appearance` |
| `LitheToolWindowHeader` | `Toolbar` + `TitleBar`（面板自绘头部） |
| `LitheUnavailableView` | `Empty` |
| `LitheContextMenu` | `PopupMenu` / `ContextMenu` |
| `LitheSettingsControls` | `form` + `input` / `select` / `checkbox` / `stepper` / 分段控件 |
| `LitheScrollViewChrome` | `scroll` / `scrollbar`（注意本项目把滚动条改成常显） |
| `WorkbenchHoverTooltip` | `Tooltip` + `tooltip_placement` |
| `LitheDragUpdateScheduler` + `FrameCoalescedDragUpdateBuffer` | **上游没有等价物** —— 本项目改为"paint 期窗口级监听"（见 `workspace.rs` 的分栏手柄实现） |
| `EditorTabFlowPlanner`（标签拖拽几何） | 上游没有；`Tab` / `TabBar` 不支持拖拽重排 |

- **缺口与优化**：⚠️ 两处 macOS 特意做了而 gpui 没有的：① **高频拖拽的按帧合并**（避免拖动时反复整页重绘）；② **标签拖拽重排几何**（含死区比例、跨行、跨容器 drop）。后者是 ⑤/④ 的共同前置。✅ 一处 gpui 更强：上游 `resize_handle` + **窗口级鼠标事件跟踪**替掉了 macOS 的自绘 NSView 拖拽层，还顺手修掉了"拖过一次后任意左键拖动都改宽度"的旧 bug。

#### ⑰ 主题

- **macOS**：`LitheTheme.swift` 一套 token 系统（2 个家族 × 明暗惰性解析）+ 15 个 `Metrics` 尺寸刻度；9 个语法高亮 adapter 各自带颜色配置。
- **gpui 落点**：`crates/settings/src/theme.rs`（主题应用与监听）、`crates/settings/src/fonts.rs`（内置字体）、`themes/*.json`（gpui-kit schema 的 7 份主题）、`crates/shared/src/icons/`（图标按明暗选择）。
- **组件替换**：直接注册 `ThemeConfig` + `ThemeMode`；动效走 `Theme::global_mut(cx).motion`；尺寸用 **Rust `const`**，不做运行时 token。
- **缺口与优化**：⚠️ **token 同名不等于语义相同**（完整陷阱表见 §4.2）：macOS 的 `accent` 是强调色（= gpui 的 `primary`），而 gpui 的 `accent` 是**悬停底色**；macOS 的 `selection` 是**不透明**选中底，gpui 默认 30% 透明。直搬会把强调色变灰底、把实心选中底变成半透明叠加。⚠️ macOS 有 19 个 token 类别在 gpui-kit 里**没有对应**（工具窗头部、tertiary 文本、diff 系列、badge、上下文菜单等）——按指南**不要自建 palette**，正确做法是**在产品 theme / `Theme::semantic_tokens()` 层补语义 role**（见 §1.4 A 组）；实测**本项目既没有自建颜色表、也没有 raw color**，所以这条当前不是债务。⚠️ macOS 侧**不存在**的 token 类别（照搬会落空，直接用 gpui-kit 的）：间距阶梯、字号 × 字重刻度（macOS 只有 14/12 两点，gpui 有 6 档）、非编辑器行高、阴影三档、`info` 色、禁用态（macOS 有 0.45/0.55 两个并存值）、密度档位、滚动条颜色、z-index。

> **间距语义档**（指南原文的 current scale，本项目复刻时优先用这些语义而非裸数值）：
> `xxs` 2px（光学校正）· `xs` 4px（同一控件的部件）· `sm` 8px（紧密相关的控件）·
> `md` 12px（同一内容 group）· `lg` 16px（同一 section 的不同 group）· `xl` 24px（不同 section）·
> `xxl` 32px（主要区域边界）。对应的 gpui 助手就是 `gap_0p5`(2) / `gap_1`(4) / `gap_2`(8) /
> `gap_3`(12) / `gap_4`(16) / `gap_6`(24) / `gap_8`(32)。
> ⚠️ 指南同时提醒：`Theme::apply_semantic_tokens` **不保存** custom spacing scale，需要自定义 scale 的应用
> 必须**自己持有完整 token snapshot** 并一致使用；`Theme::spacing_tokens()` 只投射固定默认档。

---

## 3. 验收清单（复刻完成时逐条过）

1. 每个区域的**组成与层级**与 macOS 一致（逐项对照 §2.2 各区域的"macOS 组成"）。
2. **度量数值取自 macOS**（同 §1.1 第 1 条：档位内用样式助手、档位外用 `rems(P / 16.)`，**不要 `px()` 直搬**），密度档位与行高符合 §2.2 各区域的说明。
3. 界面上出现的每个元素，都能在**本文件 §2.2** 查到"macOS 出处 → gpui-kit 实现"；查不到就是 §2.2 该补，不是"跳过"。
4. 状态齐全：空 / 加载 / 失败 / 禁用 / 选中 / 悬停，逐一可见（macOS 缺的按"优化"补上）。
5. 文案与 macOS 本地化原文一致（中文）——**文案真源是 `macos/Resources/zh-Hans.lproj`**，不是已删除的 Windows locale。
6. 没有裸色值、没有平台业务分支、没有默认图标集之外的字形。
7. **度量用 rem-based 助手**（`h_9()` / `px_2()` / `gap_1()`…），不直接写 `px(...)`；必须交 `Pixels` 的槽走 `rem_px(spec_px)`，不自己写 `/ 4.`（见 §1.1 第 1 条）。
8. **确定性逻辑不在 gpui 里**：凡是"同样输入必然同样输出、与系统无关"的算法，都应住在 `rust/lithe-core/`（判据与任务清单见 `docs/tasks/gpui-core-first.md`）。<br>⚠️ **但本分支不改 `rust/lithe-core/`**（它从 main 同步进来）：这类条目**记成"等 main"并跳过**，**不是**"那就放进 gpui 里" —— 往 gpui 里放会同时违反这条架构判据。范围约束见任务表顶部。
9. **过一遍官方指南的两份检查表**：`coding-guides.md` 的「实现检查表」（state/副作用归属、`RenderOnce` vs `Entity<T>`、稳定 domain ID、token 替代视觉字面量、键盘与 overlay 协同、loading/empty/error/cancellation、长数据虚拟化、public API 依赖方向、测试层级、fmt/clippy/env）与 `design-guides.md` 的「无障碍检查表」「设计评审清单」8 问。§1.4 已把与本项目相关的条目摘出来。
10. **在真实窗口里验证**：指南要求用真实组件、真实文案与代表性内容亲手走一遍，**不能只评审一张理想截图**；截图只用于验证语义树表达不了的视觉事实。
11. 证据：截图 + 日志落到 `.artifacts/p1/`，并在 `HANDOFF.md` 的当前状态里更新（`PLAN.md` 已不存在）。

---

## 4. 附录：gpui-kit 0.7 的核对结果（附 0.6.6 对照）

> **本节的 0.7.0 结论是照着本机 registry 里已解包的源码核出来的**，不是转述。
> 被核对的源码根：`$CARGO_HOME/registry/src/<镜像>/`，
> 目录为 `gpui-base-0.7.0`、`gpui-component-0.7.0`、`gpui-kit-0.7.0`、`gpui-kit-assets-0.7.0`、`gpui-pre-0.3.7`。
> `rust/Cargo.lock` 锁定的版本与之一致（`gpui-kit 0.7.0` / `gpui-component 0.7.0` / `gpui-base 0.7.0` / `gpui-pre 0.3.7`）。
> 下面凡标 **0.6.6** 的行，是历史记录；凡标 **0.7** 的行，是本轮实际核对的结论。

### 4.1 0.6.6 → 0.7 的三处结构性变化（**会改变写法，不只是行号**）

| # | 变化 | 0.6.6 | 0.7（已核实） |
| --- | --- | --- | --- |
| **M1** | **`open_window` 现在存在了，而且有两个** | `gpui_kit` 里**没有** `pub fn open_window`，只能 `cx.open_window(...)` | `gpui-kit-0.7.0/src/lib.rs:144` 有 `pub fn open_window<V: Render>(options, cx, build)`。⚠️ **语义与 `cx.open_window` 不同**：它内部已经 `cx.new(\|cx\| base::Root::new(view, window, cx))`，所以 `build` 闭包**只返回应用内容**，不能再包 `Root`；返回 `(AnyWindowHandle, Entity<V>)` 也是白送的。⚠️ **本项目长期断言"它不存在"（0.6.6 时期为真）**：`crates/app/src/main.rs` 的模块文档与注释仍按"不存在"写，`README.md` 已在本轮更正 |
| **M2** | **`Root` 搬了家，浮层改成插件层** | `Root` 在 `gpui-component`，Dialog / Sheet / Notification **三层要业务视图自己挂**（`render_dialog_layer` / `render_sheet_layer` / `render_notification_layer`） | `Root` 现在在 **`gpui-base-0.7.0/src/root.rs:92`**（"窗口内容 + 浮层宿主，独立于任何组件库"），三个手动挂层 API **已删除**（全 registry 零命中）。新机制是 **`RootPlugin` trait**（`root.rs:46`）：每帧三阶段 `prepare` → `style` → `decorate`，插件用**注册顺序**叠放，后注册的在上 |
| **M3** | **三层浮层改为自动挂载** | 要自己挂，否则对话框"看起来就像没打开的"（静默失败） | `gpui-component-0.7.0/src/root.rs:434` 的 `impl gpui_base::RootPlugin for WindowState`（`prepare:435` / `style:440` / `decorate:450`），内部有 `notification_layer:129`、`sheet_layer:156`、`dialog_layer:181`，由 `WindowStateLayers`（`:479`）在应用内容之后**随窗口自动挂载**。`gpui_kit::init`（`gpui-kit-0.7.0/src/lib.rs:170`）里已经调了 `gpui_component::init` + `gpui_base::init`，**所以应用不再需要手动挂层** |

其余相关 API 也一并核了：

| 事实 | 0.7 核对结果 |
| --- | --- |
| `Root::register_plugin` / `Root::plugin` / `Root::read` / `Root::new` / `Root::view` | 都在 `gpui-base-0.7.0/src/root.rs` 的 `impl Root`（`:101` / `:170` / `:176` / `:147` / `:165`） |
| `Application::with_assets` | 存在（`gpui-pre-0.3.7/src/app.rs:202`），且仍是 `impl AssetSource`，**重复调用是覆盖不是叠加** |
| `Task::then` | **没有**（0.7 仍然没有） |
| `Theme::tab_active_bg` | **没有**（0.7 仍然没有） |
| z-index（`.z_10()` 之类） | **没有**（`gpui-pre-0.3.7/src/styled.rs` 内无 `z_*`）→ 叠放仍由**绘制顺序**决定 |
| `.overflow_y_scrollbar()` | **存在**（`gpui-component-0.7.0/src/scroll/scrollable.rs:60`）。⚠️ 早先写的"0.6.6 没有它"是错的，已更正 |
| `InputBaseState` 的 `searchable` 默认值 | **`false`**（`gpui-base-0.7.0/src/input/base/state.rs:709`）。⚠️ `crates/editor` 的注释写"上游默认 true"是**反的**，那两处显式 `.searchable(true)` 是**必需的**，删掉会静默丢掉 Ctrl+F / Ctrl+H |
| `Theme::set_scrollbar_mode` | 存在（`gpui-component-0.7.0/src/theme/mod.rs:357`） |
| `EditorState::is_readonly` | 存在（`gpui-base-0.7.0/src/input/base/state.rs:509`） |
| `EditorState::diagnostics_mut` | 存在（`gpui-base-0.7.0/src/input/base/state.rs:879`，另见 `mode.rs:335`） |
| `set_scroll_offset` | 存在（`gpui-base-0.7.0/src/input/base/state.rs:2975`） |
| `Button::{ghost, selected, disabled, xsmall}`、`Icon::{small, size_3, size_4}` | 仍**在 trait 上**（`ButtonVariants` / `Selectable` / `Disableable` / `Sizable`），**必须 `use ... as _`** |

> ⚠️ **尚未逐条复核的**：`editor/src/lib.rs` 里提到的 `apply_lsp_edits` 等 0.7 API 面、以及上游 `input` 的
> hover / semantic tokens 具体缝位（本轮搜 `lsp/hover`、`lsp/semantic` 在 `gpui-base-0.7.0` 下无命中，
> 可能已改名或移出）。要用之前请自己读源码确认。

### 4.2 主题 token 映射

macOS 的 token 系统是"2 个家族 × 明暗惰性解析"，gpui-kit 是"主题配置 × `ThemeMode`"。落地时先注册 3 组 `ThemeConfig` + `ThemeMode::Dark` 起步，**不要**按 token 同名直搬：

| macOS token | 语义 | gpui-kit 对应 | 陷阱 |
| --- | --- | --- | --- |
| `accent` | 强调色 `#3574F0`（= primary） | `theme.primary` | gpui 的 `accent` 是**悬停底色**，直搬会把强调色变成灰底 |
| `selection` | 选中底，**不透明** | `theme.selection`（需覆盖为不透明） | gpui 默认 30% 透明 |
| `subtleSelection` | 悬停 / 非活动选中底 | `theme.accent` / `list_hover` | 与 `selection` 是两回事 |
| `window` / `editor` | 窗口底 / 编辑器底（浅色不同） | `theme.background` / 自建一层编辑器底色 | gpui 默认 `background == popover == white` |
| `toolHeader`、`tertiaryText`、`diff*`、`badge`、`contextMenu`、`toolWindow*` 等 19 项 | — | **自建一层颜色表**（不能加在上游 `ThemeColor` 上） | 取值走 `gpui::rgb(0x…) → Hsla`，不要把 sRGB 浮点当 HSL |
| 动效时长（0.10–0.22s + 1 spring） | — | `Theme::global_mut(cx).motion` | macOS 侧没有 token，是就地写的魔法值 |
| `Metrics`（15 个刻度） | 尺寸 | **Rust `const`** | 不要做运行时 token |

macOS 侧**不存在**的 token 类别（照搬会落空，直接用 gpui-kit 的）：间距阶梯、字号 × 字重刻度、非编辑器行高、阴影三档、`info` 色、语义色前景配对、禁用态、密度档位、滚动条颜色、z-index。

> ✅ **已核实（2026-10-02）**：`crates/settings/src/theme.rs` 与 `themes/*.json` 里**没有**自建颜色表 ——
> 全仓 grep `LitheColors` 无实现、生产代码无 raw color，全部走 `cx.theme()`。所以上表那些"macOS 有、gpui-kit 没有"的
> token 类别，**现状是"没有对应"而不是"自建了对应"**（详见 §1.2 的修订说明）。表中"陷阱"三列是踩过的坑，与版本无关。

### 4.3 本项目已实测的 gpui-kit 实现规则（按 0.7 复核后分类）

**A. 仍然成立，且与版本无关（放心用）**

| 规则 | 说明 |
| --- | --- |
| 编辑器必须显式给高度 | `LayoutMode::CodeEditor { rows }` 的 `rows` 默认只有 **2**，不写就只有两行高。正确写法 `Editor::new(&state).h(relative(1.))` |
| 代码编辑器不吃滚轮 | 编辑器元素样式**没有** `overflow: scroll`，而 gpui 只把滚轮交给带 `Overflow::Scroll` 的元素 → 宿主必须在包裹层自己接 `on_scroll_wheel` 并转成 `set_scroll_offset` |
| `ListItem` 的子元素竖排 | 内部是普通块级 `div`，多个子元素各占一行；要横排必须自己套 `h_flex()` |
| `uniform_list` 行高 = 第 0 行测量高度 | `.h()` 压不住内容；行内纵向 padding 会让内容溢出到相邻行 |
| `Column` 没有 flex | 列宽按给定值比例铺满表宽，要"剩余宽度给某列"必须折算成具体像素 |
| 根视图用 `.size_full()` | **不要**拿 `viewport_size()` 再除 `scale_factor()` —— 那会让内容只画到窗口的 78% |
| Dock 的 Bottom 只横跨中心列 | 要"底窗横跨左栏 + 编辑区"就把底部放进 center 的 `v_split` |
| Dock 每个面板外层是 `overflow_y_scroll` | 所以面板内列表要自己滚，就得自己接滚轮 |
| 面板可自绘头部 | `Panel::title_bar()` / `inner_padding()` 返回 `false`，Dock 就不画自己的标题栏/内边距 |
| 命令面板不是浮层本身 | `Command` 是普通流式布局，必须放进 Dialog 这类容器 |
| 键盘触发要挂在有焦点的元素上 | 键事件沿焦点节点的祖先链派发；焦点为空时只派发到窗口根 |
| 单位用 rem-based 助手 | macOS 的 1pt = 1 逻辑像素，**但不要照搬成 `px()`**：能用样式助手就用 `h_9()` / `px_2()` / `gap_1()`（后缀 `N` = N × 0.25rem，rem 基准 = 主题字号 16px ⇒ N × 4px）。只有必须交 `Pixels` 的槽才走项目里的 `rem_px(spec_px)`。详见 §1.1 第 1 条 |
| 不写平台业务分支 | 平台差异（红绿灯留白、窗口按钮位置）由 gpui 平台层与 `TitleBar` 处理，界面结构三端统一 |

**B. 0.7 已改写法（**照 0.6.6 的写法做会失效**）**

| 0.6.6 的写法 | 0.7 要怎么做 |
| --- | --- |
| 在业务视图 render 里挂 `.children(Root::render_dialog_layer(window, cx))` 等三层 | **不要挂**。API 已删除；`WindowState` 作为 `RootPlugin` 由 `Root` 自动渲染（见 §4.1 M3） |
| `Root` 从 `gpui-component` 导入 | 改从 **`gpui-base`** 导入（`pub use root::{Root, RootPlugin}`，`gpui-base-0.7.0/src/lib.rs:159`） |
| 想加自己的窗口级浮层就直接往冲突处塞元素 | 实现 `RootPlugin`（`prepare` / `style` / `decorate`）并用 `Root::register_plugin` 注册，**注册只影响之后创建的新窗口** |
| 滚动条常显 | 仍然要 `Theme::set_scrollbar_mode(ScrollbarMode::Always, cx)`（`:357` 仍在），否则内容多了看不出来能滚 |
| 依赖 `searchable` 的默认值 | 0.7 默认 `false`，**必须显式 `.searchable(true)`** 才有 Ctrl+F / Ctrl+H |
| 断言 `gpui_kit::open_window` 不存在 | 0.7 已有（`gpui-kit-0.7.0/src/lib.rs:144`）；`cx.open_window` 仍可用，但别再把"不存在"写进注释和文档 |

**C. 仍然有效但需要按 0.7 重新定位的**

| 规则 | 0.7 的位置 |
| --- | --- |
| `open_dialog` 不能在窗口首帧之前调用 | `WindowState::open_dialog`（`gpui-component-0.7.0/src/root.rs:235`）；仍要在 `Root` 就位后、由用户输入或任务触发 |
| `Root::new` 必须是窗口第一层 | `gpui-base-0.7.0/src/root.rs:147` |
| 上层 `EditorScrollbar` / 诊断绘制 | 本轮未逐条复核，需要时读 `gpui-base-0.7.0/src/input/` |



