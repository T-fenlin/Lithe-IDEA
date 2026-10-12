# 颜思齐 — 设计系统、配色主题、图标组件与视觉规范

GPUI 统一前端相对 Windows 旧端与 macOS 端的**视觉层**缺口、优化项与排期。
基线 `3085f5d1`，分支 `feat/menu-bar-opaque`。**本次只做分析，未改任何代码、未建分支、未提交。**

---

## 结论

**一、视觉层的真实缺口不是「组件缺」，而是「规格分辨率不足」加「素材来源不可复现」。**

矩阵里 macOS 视觉条目**是 6 条不是 5 条**（任务书漏了 `project-sidebar-header-icons` 与
`project-sidebar-tree-disclosure-icons` 是两条）。逐条实测：**4 条有对应实现、2 条部分**，
另有 2 条含视觉成分的条目在 GPUI 侧**检索命中 0**（`editor-wallpaper-overview-ruler`、
`terminal-native-palette`）。所以「GPUI 有对应物」这一层基本成立——
真正的差距在**取值**：圆角、字号、内边距、滚动条厚度、提示气泡配色**五项系统性偏离
IDEA 真源**，且偏离方向一致（lithe-ui 把 macOS 按角色分级的档位压平成单档）。

**二、最硬的一条新事实：图标包的「生成 + 校验」链路已失效。**

`rust/lithe-gpui/assets/icons/idea/SOURCES.md:3` 声明该目录由
`tools/sync-macos-idea-icons.mjs --apply` 生成、「**不要手改**」。实测跑只读 `--check`：
**失败 129 条，exit code 1**，且工具算出的「保留 63 / 删除 123」个 `idea.rs` 常量
与现状（186 组常量、122 个 `legacy/` 文件）相差极远。
**声明的真源同步机制对当前树不成立**——这意味着图标目录已进入「无人能安全重跑」的状态，
任何图标增删都在裸改。

**三、图标层有两个独立问题，都不是「缺图标」。**

- **两套命名空间被类型刻意隔开**：`lithe-ui` 的 **306 个 Lucide 图标在 GPUI 产品界面里
  一个都没用**（`IconName::` 全仓 7 处命中，全是注释或断言），应用图标只取
  `assets/icons/idea/**`。所以**讨论「IDE 图标够不够」时 lithe-ui 完全不相干**。
- **7 张图在浅色主题下偏暗**：`debugger/debug.svg`、`debugger/stop.svg`、
  `expui/general/chevronRight.svg`、`expui/general/search.svg`、`fileTypes/powershell.svg`、
  `fileTypes/text.svg`、`fileTypes/toml.svg` 的**浅色档色值与 macOS 同名文件的深色档完全一致**，
  且深浅两档画同一张深色美术。经渲染路径核算，走 `idea_icon_image`（→ `gpui::img`，保留烘焙色）
  的 **71 处可见**，走 `idea_icon_mono`/`idea_icon_svg_px`（→ `svg()` 光栅化为 alpha mask
  单色填充）的 **52 处不可见**。

**四、主题架构是对的，但「默认观感不是 IDEA 的」。**

宿主主题真源唯一（`rust/lithe-gpui/themes/jetbrains.json` → `palette_for` → 组件库全局），
**宿主零硬编码色值**（`rgb(0x`/`rgba(0x`/`#rrggbb` 在 `rust/lithe-gpui` 非测试代码命中 **0**）。
但 `lithe-ui` 的默认调色板是 **Ely 的暖中性色**（`bg: 0xfcfaf7`、`fg: 0x181613`），
`palette_for` **没给的槽保留库默认值**（`theme.rs:343` 明文）。
实测未覆盖的槽包括 **tooltip 两个槽**与整个 `ansi` 数组——
这正是**浅色主题下 tooltip 呈近黑底浅字**（IDEA 是白底黑字）的直接原因。

**五、排期结论：视觉层应在「构建链路」之后、「信息架构」之前。**

视觉层的工作量集中在**改 token 取值与补素材**，不依赖架构决策，
但**验收依赖构建链路**——台账 §3 已证 GPUI 在 18 个工作流里检索 `lithe-gpui` 命中 0，
没有 CI 就没有视觉回归的自动化判据。所以视觉层的**第一批可交付**必须是
「能跑起来的 `S1_THEME`/`S1_ASSETS` 打点 + 画廊截图」，而不是先改样式。

---

## 证据与定位

### 1. 矩阵 macOS 视觉条目逐条判定（6 条 + 4 条附）

| # | 矩阵 id | GPUI 判定 | 关键证据（路径:行号 / 取值） |
|---|---|---|---|
| 1 | `macos-shared-product-dropdowns` | 有对应实现 | `lithe-ui/src/menus/menu.rs:361` 开动画 `FAST=120ms`；`:451` `opacity(t)`；`:467-470` 子菜单 `ml(NUDGE=4px*(t-1))`；`motion/curve.rs:7,13`。真源要求「无系统弹跳动画」→ **方向相反** |
| 2 | `macos-shared-hover-tooltip-style` | 有对应实现 | `lithe-ui/src/primitives/tooltip.rs:24-31`；圆角 `Radius::Md`=**6**（`tokens.rs:487`）vs IDEA **4**；字号 `TextSize::Sm`=**12**（`:473`）vs **13**；`px_2/py_1`=8/4 vs **12/8/9**；**无内边框**；`tooltip_bg` 浅色 `0x181613`（`palette.rs:254`）vs IDEA `#FFFFFF` |
| 3 | `macos-bundled-default-ui-font` | 有对应实现 | `theme/mod.rs:119-121` `font_family:"Inter"`/`mono_family:"JetBrains Mono"` **族一致**；`lithe-ui/assets/fonts/` **8 字体+3 许可**（Inter 仅 4 档，**无 Bold/Black**）vs macOS **19 个 Inter `.otf`** |
| 4 | `macos-shared-scrollbar-style` | 有对应实现 | `lithe-ui/src/layout/scroll.rs:210,216` rail=`thickness*2`=**12px** vs note `:28` IDEA **14pt**；`:172-173` thumb=`thickness`=**6px** vs note `:32` IDEA **7→10/9→12pt**；`:157-162` **无 1pt 内边框**（note `:22` 要求）；`:156` 取色 `fg.opacity(0.22/0.42)` vs note `:33` `#FFFFFF26/#FFFFFF4D`；**Diff 标记未找到实现入口** |
| 5 | `project-sidebar-header-icons` | **部分** | `locate*` 全目录**命中 0**（macOS 有明暗各一）；`LOCATE`/`ScrollFromSource`/`selectIn` 在 `rust/lithe-gpui` 只命中 `theme.rs:305` `locate_theme`、`debug.rs:248` `relocate_breakpoints`（与图标无关）⇒ **功能入口不存在**；`refresh` 仅 `actions/refresh.svg` 单张，`idea.rs:1056-1060` `has_dark:false` |
| 6 | `project-sidebar-tree-disclosure-icons` | **部分** | 7 张图浅色档携带深色美术（见下表）；行距 `lists/project/row.rs:128` `control_height(Size::Small)`=**24**（`tokens.rs:98`，Standard delta=0）**对上** capability 的 24pt；缩进三值并存：`row.rs:29-30` `8+14`、`theme.tree_indent()`=**16**（`tokens.rs:341`）、macOS `LitheTheme.swift:403` `19` |
| 附 | `git-commit-sidebar-toolbar-icons` | 有对应实现 | `crates/git/src/changes_view.rs:891` `render_toolbar`；`:646` `idea_icon_image(&REFRESH_ICON)`、`:926/:937` PLUS/MINUS |
| 附 | `editor-wallpaper-overview-ruler` | **未找到实现入口** | 全 `rust/lithe-gpui` 检索 `wallpaper\|overview_ruler\|overviewRuler\|Wallpaper` **命中 0** |
| 附 | `terminal-native-palette` | **未找到实现入口** | `crates/terminal/` 检索 `ansi\|palette\|Ansi` 仅命中 `ansi.rs` 与 `session.rs:13`；**无 Terminal.app 描述文件读取** |
| 附 | `editor-macos-ime-presentation` | 有对应实现（在 lithe-ui 层） | `lithe-ui/src/editor/ime.rs:66,155`、`forms/text/ime.rs:53`、`terminal/input.rs:261`、测试 `editor/tests.rs:248,265`、`forms/tests/mod.rs:135,152,340,417`；`tokens.rs:194` `underline_thickness()` 注明组合输入下划线，12 处消费 |

> 口径更正：任务书列的 5 条中，`project-sidebar-*` 实为 **2 条**。
> `platform-specific` 语义见 `shared/platform-feature-matrix/metadata.json`——
> 该能力刻意只属一端，**「GPUI 没有」不自动等于缺口**。

### 2. 两套图标命名空间（严格隔开，写进类型）

| | 组件库 `rust/lithe-ui/assets/icons/` | 应用 `rust/lithe-gpui/assets/icons/idea/` |
|---|---|---|
| 图标语言 | **Lucide** | **IDEA Community（expui）** |
| 文件数 | **307**（306 `.svg` + LICENSE） | **301**（297 `.svg` + `extension.json` + `LICENSE-APACHE-2.0.txt` + `NOTICE.txt` + `SOURCES.md`） |
| 深浅变体 | **0** 个 `*_dark` | **105** 个 `*_dark.svg` |
| 类型入口 | `IconName`（`primitives/icon.rs`，601 行） | `AppIcon`/`idea::IdeaIcon`（`shared/src/icons/mod.rs:25-71`） |
| 声明条数 | `IconName::ALL` | **186 组** light/dark 常量（`idea.rs`，1141 行；`has_dark:true` **112**、`false` **75**） |

隔开是**类型事实**（`shared/src/icons/mod.rs:8-33` 原文）：
> 「应用图标一律取自**本仓库自己的**图标集……我们自己没有那张图就**留空、不画**，
> 不许拿组件库（`lithe-ui`）的图标顶替。」
> 「**这个类型把类型变成事实**：它只装资源路径 `&'static str`，装不下
> `lithe_ui::primitives::IconName` 这类组件库图标名 —— 所以「用库图标」不再可表达。」

实测：`IconName::` 全 `rust/lithe-gpui` **7 处**，**全部是注释或断言**
（`app/src/assets.rs:249` 清单自检、`app/src/main.rs:643`、`explorer/src/model.rs:275`、
`shared/src/icons/file_icon.rs:677`、`workbench/src/extensions_page.rs:265/538`、
`workbench/src/right_tool_window.rs:71`）。
⇒ **lithe-ui 的 306 个 Lucide 图标在产品界面里一个都没用。**

清单完整性：`idea.rs` 全部 `"icons/idea/..."` 字面量 **372 条，逐个 `Test-Path` 缺失 0**。

### 3. 图标覆盖与「浅色档携带深色美术」

严格同相对路径比对（297 GPUI svg vs macOS）：

| 关系 | 条数 |
|---|---|
| 同路径逐字节相同 | **162** |
| 同路径内容不同 | **13** |
| GPUI 有、macOS 同路径无（即 `legacy/` 那批） | **122** |

13 条不同里 **7 条是极性错误**（浅色档 == macOS 深色档）：

| 文件 | GPUI 浅色 | macOS 浅色 | macOS 深色 |
|---|---|---|---|
| `debugger/debug.svg` | `#253627,#57965C` | `#208A3C,#F2FCF3` | `#253627,#57965C` |
| `debugger/stop.svg` | `#5E3838,#DB5C5C` | `#DB3B4B,#FFF7F7` | `#5E3838,#DB5C5C` |
| `expui/general/chevronRight.svg` | `#B4B8BF` | `#818594` | `#B4B8BF` |
| `expui/general/search.svg` | `#CED0D6` | `#6C707E` | `#CED0D6` |
| `fileTypes/powershell.svg` | `#43454A,#CED0D6` | `#6C707E,#EBECF0` | `#43454A,#CED0D6` |
| `fileTypes/text.svg` | `#CED0D6` | `#6C707E` | `#CED0D6` |
| `fileTypes/toml.svg` | `#25324D,#548AF7` | `#3574F0,#F5F8FE` | `#25324D,#548AF7` |

这 7 条 `_dark` 兄弟与 macOS `_dark` **逐字节相同**（已验 4 条 True）。
正确素材在本仓可得：`windows/tauri/src/ui/icons/idea/expui/general/search.svg` = `#6C707E`、
`chevronRight.svg` = `#818594`（均为正确浅色值）。

**渲染路径决定可见性**（实测两条语义不同的路）：

| 路径 | 实现 | 颜色语义 | 调用点 |
|---|---|---|---|
| `idea_icon_image` → `theme_file_icon` → `gpui::img(ImageSource::Resource)` | `shared/src/icons/file_icon.rs:245-261` | **保留 SVG 烘焙色** | **71** |
| `idea_icon_mono` / `idea_icon_svg_px` → `lithe_ui::Icon` → `svg()` | `shared/src/icons/mod.rs:159-186` | **丢弃颜色，按主题前景重着色** | **37 / 15** |

依据：gpui-pre 0.3.8 `Window::paint_svg`（`window.rs:4898-4906`）把 SVG 光栅化为
**alpha mask** 再填单色 `MonochromeSprite`；`svg_renderer.rs:246-250` 明写
「Convert the pixmap's pixels into an alpha mask」只取 `p.alpha()`。
⇒ **71 处会真画出偏暗的浅色图标**（含 `git/log_view.rs:1721` `LogIcon::Idea`→`element`→
`idea_icon_image`、`git/changes_view.rs:646`、`workbench/src/menu_bar.rs` `MenuIcon::Idea` 路径）。

### 4. 生成器校验失败（工具链缺陷）

```
$ node rust/lithe-gpui/tools/sync-macos-idea-icons.mjs --check
macOS 图标文件夹：304 个 SVG + 2 个附件 → 逐字节复制进 gpui
旧包（Windows 派生）：0 个 SVG
  · 保留：0（其中 0 个换成 macOS 美术、0 个继续用旧美术）
  · 删除（清单与代码都没引用）：0
idea.rs 常量：保留 63、删除 123
清单定义：删除未被 fileExtensions/filenames/folders/expandedFolders/默认值引用的 2 条
--check 失败（129 条），例如 macOS 文件 dvcs/branchLabel.svg 没被复制进来；……
[exit code: 1]
```

`SOURCES.md:3` 声明「本目录由该工具 `--apply` 生成，**不要手改**」——
**该声明对当前树不成立**。工具算出的「保留 63 / 删除 123」与现状（186 组常量、122 legacy 文件）
相差极远，且 129 条 `dvcs/`、`expui/general/` 等文件「没被复制进来」。

### 5. 设计系统规模（可清点数字）

| 项 | 数值 | 来源 |
|---|---|---|
| `lithe-ui/src/theme/` 文件数 | **20**（含 `tests.rs`） | `Get-ChildItem` |
| `theme/` 下 `pub fn` | **110**（其中 `tokens.rs` **72**） | `Select-String 'pub fn '` |
| `Palette::NAMES` | **43**（断言钉死） | `theme/tests.rs:60` |
| `Syntax::NAMES` | **13** | `theme/tests.rs:75` |
| 数组槽 | `chart:[Hsla;8]`、`ansi:[Hsla;16]` | `palette.rs:170-171` |
| `platform.rs` | **20 行**，`Platform` 仅 3 变体 + `current()`，**无按平台尺寸表** | `theme/platform.rs` |
| 尺寸缩放唯一入口 | `Theme::apply_rem`（`mod.rs:233-235`）写窗口 rem | 与选型笔记 §6 一致 |
| 组件库文案 | `locales/ui.yml` **62 行 / 9109 字节 / 顶层键 6**，**7 个语言标签** | `ui.yml` |
| 宿主文案 | `lithe.en.yml` **8977 行 / 282209 字节**；`lithe.zh-CN.yml` 293125 字节，**2 语** | `crates/shared/locales/` |
| 画廊页面 | **44 个页面目录** | `examples/gallery/pages/` |
| 宿主硬编码色值 | **0** | `rgb(0x`/`rgba(0x`/`#rrggbb` |
| `lithe-ui` 非测试硬编码色值 | **23**，其中 **20 在测试**、**3 是设计真源**（`theme/syntax.rs:67-83`） | 同左 |

### 6. 宿主主题接线（真源唯一，但默认残留）

| 环节 | 位置 |
|---|---|
| 主题全局初始化 | `crates/app/src/main.rs:638` `Theme::init(cx)` |
| 主题 JSON 内嵌 | `crates/settings/src/theme.rs:15-16` `include_str!("../../../themes/jetbrains.json")` |
| IDEA 主题文件 | `rust/lithe-gpui/themes/jetbrains.json`（259 行，`IntelliJ Light` + `Darcula`） |
| 主题→调色板 | `theme.rs:348` `fn palette_for(record) -> Palette` |
| 落进组件库全局 | `theme.rs:650` `ComponentTheme::apply(...)` |
| 字号/字体注入 | `theme.rs:669`、`:678`、`:695-702` |
| 设置变更重放 / 热重载 | `crates/settings/src/store.rs:1024`；`theme.rs:607-621` |

`theme.rs:343` 明文：「**没给的栏保留组件库自己的默认值**」。
`lithe-ui` 默认调色板是 **Ely 的暖中性**（`bg 0xfcfaf7`、`surface 0xffffff`、`fg 0x181613`、
`accent 0x181613`，`palette.rs:211-257`），**不是 IDEA 的**。
`palette_for` 的映射清单（`theme.rs:361-493`）**未包含 tooltip 键**，
故 `tooltip_bg`/`tooltip_fg` 残留默认 → 浅色主题下近黑底浅字。

### 7. 三方取值对照（macOS owner × IDEA 真源 × lithe-ui）

| 角色 | macOS owner | IDEA 真源（经 note/矩阵转述） | lithe-ui | 判定 |
|---|---|---|---|---|
| 项目树选中圆角 | **4**（`LitheTheme.swift:729`） | 4 | **4**（`Radius::Sm`，`row.rs:216`） | 一致 |
| 控件/按钮/输入框圆角 | **6**（`:739`） | — | **6**（`Radius::Md`，`button.rs:424`、`forms/input.rs:133`） | 一致 |
| 工具栏圆角 | **6**（`:462`） | — | 6（复用 `Md`） | 一致 |
| 悬浮提示圆角 | **4**（`:518`） | **4** | **6** | 不一致 |
| 右键菜单圆角 | **8**（`:738`） | — | **6** | 不一致 |
| 弹窗圆角 | **10**（`:737`） | — | **6** | 不一致 |
| 通用圆角 | **5**（`:736`） | — | **无对应档** | 缺档 |
| 树行高 | 24（`:720`）／项目树 27（`:723`） | **24** | **24**（`Size::Small`） | 一致（Standard 档） |
| 树图标 | **16**（`:730`） | — | **16**（`IconSize::Md`） | 一致 |
| 树字号 | **13**（`:731`） | — | **13**（`TextSize::Base`） | 一致 |
| 树缩进 | **19**（`:403`） | — | 库 **16**（`tree_indent()`）／项目树 **8+14**（`row.rs:29-30`） | **三值并存** |
| 滚动条 rail | **14pt** | 14pt（note `:28`） | **12px** | 不一致 |
| 滚动条 thumb | **7→10 / 9→12pt** | 同左（note `:32`） | **6px** | 不一致 |
| 滚动条内边框 | **1pt** | 1pt（note `:22`） | **无** | 不一致 |
| 提示圆角/字号/内边距 | **4 / 13 / 12-8-9** | 同左（矩阵 JSON） | **6 / 12 / 8-4** | 不一致 |
| 提示底色（浅） | `#FFFFFF` | `#FFFFFF` | `0x181613`（近黑） | **方向不同** |
| 菜单打开动画 | **无**（`animationBehavior = .none`） | 无 | **120ms + 4px** | **方向相反** |

**关键判断**：macOS owner 的 4/5/6/8/10 **不是互相矛盾的两套体系**，而是**同一体系按角色分级**；
lithe-ui 的 `Radius` 只有 4 档且把菜单/弹窗/提示全压到 `Md`=6，是**分辨率不足**。
`Radius::Xl`=12 在 macOS owner 里**无对应角色**（最大 10）。
**唯一够格称「结构性冲突」的是图标族**——而它发生在 GPUI vs macOS/IDEA 之间，
**不是** macOS vs IDEA 之间（两边都是 expui，见 §3 更正）。

> **本结论最脆的假设**：IDEA 真源**只能经二手转述读**。
> 仓库内 `Get-ChildItem -Recurse -Filter 'jetbrains.json'` 只找到 **1 个**文件
> （`rust/lithe-gpui/themes/jetbrains.json`，主题配色，非控件几何）。
> IDEA Community 源码**未检出**。note 里出现**三个不同 revision**：
> `fb72b4df43ab`、`c7f91397daa3a961b4e78bc634fe467a0a7d9ade`、`fb72b4df43aba102479eb0502d20b03586b9c5b8`。

---

## 风险与条件

**R1｜「IDEA 真源」口径未统一（高）**
三个不同 revision 在 note 里并存，且仓库无 IDEA 源码检出。
触发条件：若日后检出 IDEA 并逐项核对，发现 macOS owner 某取值与 IDEA 不符，
则「macOS owner 忠于 IDEA」这一前提被推翻，§7 全部对照需重做。
**当前所有「IDEA 真源」一列都是二手转述值。**

**R2｜图标目录已无法安全重跑（高）**
`sync-macos-idea-icons.mjs --check` 失败 129 条。触发条件：任何人按 `SOURCES.md`
指示跑 `--apply`，会**删除 123 个 `idea.rs` 常量**、重写整个图标目录与清单——
在链路修复前这是破坏性操作。**建议在修好前把 `--apply` 加显式警告或锁。**

**R3｜7 张极性错误的可见面取决于渲染路径（中）**
71 处走保色路径可见、52 处不可见。若后续有人把 `idea_icon_mono` 换成
`idea_icon_image`（或反之），可见面会变化。
**未做渲染实测**，本判断是读源码 + 读 gpui 渲染器源码得出的推断。

**R4｜density 一变树行高即偏离（中）**
capability 要 24pt 连续行距，`control_height(Size::Small)` 在 `Compact` 下变 **20**、
`Comfortable` 下变 **28**（`tokens.rs:102-106`）。
触发条件：宿主启用非 Standard density。**实测宿主未设 density**
（`rust/lithe-gpui` 检索 `density`/`Density` **命中 0**），故当前不触发。

**R5｜菜单动画与「无弹跳」要求方向相反（中）**
若产品在 macOS 上沿用 lithe-ui 默认，会出现 IDEA 明确禁止的入场动画。
**可调和**：`motion::duration`（`curve.rs:19-23`）已读 `reduced_motion`，
只需接一个「IDE 档」。

**R6｜组件库 7 语 vs 宿主 2 语（低）**
`ui.yml` 有 `he-IL` 等 7 语，宿主只有 `en`/`zh-CN`。
触发条件：若产品要出第三种语言，组件库与宿主文案覆盖不一致会露出英文/中文混排。

**R7｜未执行任何构建、测试、渲染（影响结论强度）**
本报告全部是**静态源码、资源清单、逐字节哈希**对照。
「浅色档偏暗」「圆角不一致」等结论**未经渲染实测**。
未跑 `cargo build`、未启动画廊、未在 macOS 实机验证。

---

## 建议与验收

### A. 视觉层行动清单（按优先级）

| # | 行动 | 现状证据 | 目标 | 验收方式 | 人日 | 前置依赖 |
|---|---|---|---|---|---|---|
| **V1** | 修图标生成链路：让 `sync-macos-idea-icons.mjs --check` 通过，或明确废弃该工具并改 `SOURCES.md` | `--check` 失败 129 条，exit 1 | `--check` exit 0，或 `SOURCES.md` 不再声称自动生成 | 跑 `node rust/lithe-gpui/tools/sync-macos-idea-icons.mjs --check`，exit 0 | **2** | 无 |
| **V2** | 修 7 张极性错误图标：从 macOS/Windows 侧取正确浅色档覆盖 | §3 表；正确素材在 `windows/tauri/src/ui/icons/idea/expui/general/` | 7 条同路径逐字节 == macOS 浅色档 | 逐字节哈希比对脚本 + `S1_ASSETS` 打点 + 浅色主题截图 | **1.5** | V1（否则改动会被工具覆盖） |
| **V3** | 补 `locate` 等缺失图标并接「定位当前文件」入口 | `locate*` 命中 0；`LOCATE`/`ScrollFromSource` 命中 0 | 图标入 `assets/icons/idea/expui/general/`，`idea.rs` 加常量，explorer 头部接入 | `every_idea_constant_path_is_embedded` 测试 + `S1_ASSETS probe` 打点 | **3**（含入口接线） | V1；入口接线需与陆井然/闻本立核对 |
| **V4** | 圆角补档：`Radius` 增 `Menu`(8)/`Popup`(10)/`Tooltip`(4)，控件按角色取档 | §7 表；`tokens.rs:484-492` 仅 4 档 | 菜单 8、弹窗 10、提示 4、控件 6 | `theme/tests.rs` 加断言钉住四角色取值 + 画廊 `pages/overlays`/`pages/primitives` 截图 | **2** | 无 |
| **V5** | tooltip 对齐 IDEA：圆角 4、字号 13、内边距 12/8/9、补 1pt 内边框、补 `palette_for` 的 tooltip 键映射 | §1 第 2 条；`tooltip.rs:24-31`；`theme.rs:361-493` 无 tooltip 键 | 浅色白底黑字、深色 `#33353B`/`#D1D3D9` | 浅/深主题截图 + 对比度断言（`devtools/contrast.rs` 已有 WCAG 实现） | **2** | V4 |
| **V6** | 滚动条对齐 IDEA：rail 14、thumb 7→10/9→12、1pt 内边框、专用取色 token；补 Diff 标记 | §1 第 4 条；note `2026-10-02-macos-shared-scrollbar.md:22,28,32,33` | 四处取值对齐 note | `LitheScrollBarStyleTests` 的 Rust 对应测试 + 画廊 `pages/layout` 截图 | **3** | V4（共用圆角档） |
| **V7** | 菜单打开动画接「IDE 档」：macOS 上无入场动画 | §1 第 1 条；`menu.rs:361,451,467-470` | macOS 上菜单直接打开；Windows 保留现状 | 画廊 `pages/menus` 交互脚本 + `S1_*` 打点 | **1** | 无 |
| **V8** | 补 Inter 字重：至少 Bold，使 `BundledUIFontTests` 的四档要求可满足 | `lithe-ui/assets/fonts/` Inter 仅 4 档无 Bold；macOS 有 19 档 | Inter Regular/Medium/SemiBold/Bold 齐全 | `register_fonts` 测试 + 字体清单比对 | **1** | 无 |
| **V9** | 统一树缩进口径：明确「库树缩进 16」与「IDE 项目树缩进 8+14」是两个量，写进规格 | `row.rs:27-30` 自承差异；`tokens.rs:341` vs `LitheTheme.swift:403` | 规格文档写明两值及各自适用面 | 文档评审（无需代码改动） | **0.5** | 无 |
| **V10** | 建视觉回归门禁：画廊截图 + `S1_THEME`/`S1_ASSETS` 打点进 CI | 台账 §3：`lithe-gpui` 在 18 个工作流命中 0 | CI 能对主题/图标改动给出自动化判据 | CI 跑通并产出截图 artifact | **5** | 常通关的构建链路（见「交接」） |

**合计约 21 人日**（不含 V10 的 5 人日）。

### B. 先后顺序与依赖

```
第 0 批（可与构建链路并行，2 人日）
  V1 图标生成链路修复  ← 阻塞 V2/V3，最高优先
  V9 缩进口径文档（0.5，无依赖）

第 1 批（token 层，4 人日）
  V4 圆角补档  ← 阻塞 V5/V6
  V8 Inter 字重

第 2 批（组件层，6 人日，依赖第 1 批）
  V5 tooltip 对齐
  V6 滚动条对齐
  V2 图标极性修复（依赖 V1）

第 3 批（入口与动画，4 人日）
  V7 菜单动画档
  V3 locate 图标 + 入口（需跨角色核对）

第 4 批（门禁，5 人日）
  V10 视觉回归进 CI  ← 依赖构建链路就绪
```

**关键依赖链**：`V1 → V2/V3`（不改工具就改图标，改动会被覆盖）；
`V4 → V5/V6`（三者共用圆角档）；
**`构建链路 → V10`**——台账 §3 已证 GPUI 无任何 CI 入口，
**在常通关把 Windows 侧 gpui 构建链路建起来之前，V10 做不了**，
届时视觉验收只能靠人工跑画廊截图。

### C. 建议的 token/组件约定（可直接落地）

1. **圆角按角色取档，不按控件取档**。新增 `Radius` 档位对应 macOS owner 的语义：
   `Radius::Sm`=4（选中弧、提示）、`Radius::Md`=6（控件/按钮/输入框/工具栏）、
   `Radius::Menu`=8、`Radius::Popup`=10。**禁止**把菜单/弹窗继续压到 `Md`。
2. **浮层类组件（`menus/`、`overlays/`、`primitives/tooltip.rs`）的圆角从角色档取，
   不从 `Md` 取**。这是 §7 里三条不一致的共同根因。
3. **滚动条补专用 token**，不再从 `colors.fg` 的透明度派生：
   新增 `scrollbar_thumb`/`scrollbar_thumb_hover`/`scrollbar_rail`，
   默认值取 note `:33` 的 `#FFFFFF26`/`#FFFFFF4D`（深色）。
4. **`palette_for` 补 tooltip 键映射**：`tooltip.background` → `tooltip_bg`、
   `tooltip.foreground` → `tooltip_fg`。**未给时才回落到库默认**。
5. **图标极性由测试钉住**：加一条测试断言「每个 `IdeaIcon` 的 `light` 与 `dark`
   在 macOS 侧同路径文件里分别对应浅色档与深色档」——
   当前 7 张图会挂，正好当 V2 的验收判据。
6. **`idea_icon_image` 与 `idea_icon_mono` 的语义差异写进注释**：
   前者保色（走 `gpui::img`）、后者单色（走 `svg()` alpha mask）。
   两者混用会让「图标偏暗」的可见面漂移（R3）。
7. **密度档与视觉常量解耦**：capability 里的 24pt 行距在 `Compact`/`Comfortable`
   下会变 20/28（R4）。若产品要求固定 24，应给项目树专用 token 而非复用 `Size::Small`。

### D. 与现有约束的兼容性

| 约束 | 是否冲突 | 说明 |
|---|---|---|
| 选型笔记 §5「界面规格真源是 IDEA Community」 | **兼容** | 上述 V4-V8 全部是**向 IDEA 靠拢**，方向一致 |
| 选型笔记 §6「不给不同平台写不同像素常量」 | **兼容** | V4 只加档位，`platform.rs` 仍 20 行无平台分支 |
| `develop-lithe`「只用 token，不硬编码」 | **兼容** | 宿主已 0 硬编码；V5/V6 是补 token 而非加字面量 |
| `develop-lithe`「macOS shared frontend controls」 | **兼容** | 实测**未发现** macOS owner 与 IDEA 真源在同一属性上的数值矛盾 |
| `lithe-ui-gallery`「深浅主题必须同时成立」 | **兼容，且正是 V5 的判据** | tooltip 浅色近黑底是当前唯一的主题方向性问题 |
| `assets.rs`「应用图标只取 `assets/icons/idea/**`，缺则留空」 | **兼容** | V3 是往该目录补素材，不违反 |
| `AGENTS.md` 运行时只读边界 | **兼容** | 全部改动在编译期资源，不涉运行时写入 |

---

## 交接给主理人

**需与其他职责核对的具体问题**

1. **给常通关**：V10 依赖 GPUI 的构建链路。台账 §3 称 18 个工作流检索 `lithe-gpui`
   命中 0——请其独立复核，并给出「Windows 侧 gpui 产物链路何时可用」。
   **在链路可用前，视觉层的验收只能靠人工跑画廊 + 截图**。
2. **给陆井然**：V3 的「定位当前文件」入口涉及 explorer 头部按钮的排布与键位
   （`explorer_view.rs:449-493` 现为 refresh/collapse/options/hide 四钮）。
   图标补上后**入口放哪里、绑什么键**属其职责，我这边只负责图标与视觉。
3. **给闻本立**：V3 的 `ScrollFromSource` 语义需要「当前编辑器标签 → 树节点」的映射，
   这在 `crates/editor` 与 `crates/explorer` 之间。我确认了**功能入口不存在**，
   但**不知道该能力应由哪个 crate 提供**。
4. **给娄码通**：R2（图标工具 `--apply` 是破坏性操作）与 R3（两条渲染路径语义不同）
   都涉及工具链安全。V1 的修法（改工具 vs 改文档）需要其判断
   `sync-macos-idea-icons.mjs` 是否还值得维护。
5. **给柯展之**：`crates/extensions/src/model.rs:215-231` 的扩展清单里有
   `themes/gruvbox.json`、`themes/nord.json` 与 `fonts/JetBrainsMono-*.ttf` 条目。
   **扩展主题/字体包如何与 V4/V5 的 token 体系交互**（扩展能不能覆盖新档位）
   需与其插件契约对齐。

**缺失资料**

- **IDEA Community 源码未检出**，三个 revision 号并存（`fb72b4df43ab`、
  `c7f91397…`、`fb72b4df43aba…`）。这是本报告所有「IDEA 真源」一列的**共同弱点**。
  建议主理人明确：以哪个 revision 为准，是否需要检出。
- `shared/platform-feature-matrix/metadata.json` 的 `statusDefinitions` 只定义
  macOS/Windows 两套状态，**没有 gpui 列**（台账 §1 已列）。
  视觉层的新增/缺失**当前无处登记**——这是 `gpui-parity-plan:344-348` 留下的未决项，
  直接影响 V1-V9 的验收能否被 CI 门禁追踪。

**未覆盖内容（明确说明）**

- **未做任何渲染、截图或像素比对**；未跑 `cargo build`、未启动画廊、未在 macOS 实机验证。
- **未跑 `sync-macos-idea-icons.mjs --apply`**（会改工作区），只跑了只读 `--check`。
- **未展开**：`lithe-ui` 44 个画廊页面的逐页视觉审查；
  `theme/` 下 14 个按域分章尺寸模块（calendar/chart/dashboard/…）的取值合理性；
  `lithe-ui/src/syntax.rs` 两套语法色板与 IDEA 语法配色的逐色对照。
  这三块**有明确入口但本次未做**，如需要可另开一轮。
- **未判定** `Radius::Xl`=12 在 macOS owner 里无对应角色这一事实意味着什么
  （可能是组件库通用能力，也可能是多余档位）——**证据不足，不猜**。
