# GPUI 设计系统实物盘点（lithe-ui 组件库 + lithe-gpui 宿主）

基线 `3085f5d1`，分支 `feat/menu-bar-opaque`。所有数字都是本次实测得到，
每个数字附得出它的命令或文件。

## 1. 两套图标命名空间（本次最重要的结构性发现）

GPUI 侧**不是一个图标集，是两个，且被刻意隔开**：

| | 组件库 `rust/lithe-ui/assets/icons/` | 应用 `rust/lithe-gpui/assets/icons/idea/` |
|---|---|---|
| 图标语言 | **Lucide** | **IDEA Community** |
| 文件数 | **307 个文件（306 个 `.svg` + LICENSE）** | **297 个 `.svg`** |
| 深浅变体 | **0 个**（无 `*_dark` 文件） | **105 个 `*_dark.svg`** |
| 类型入口 | `IconName`（`primitives/icon.rs`，601 行） | `AppIcon` + `idea::IdeaIcon`（`shared/src/icons/`） |
| 命名空间类型 | `lithe_ui::primitives::IconName` | `lithe_gpui_shared::icons::AppIcon` |
| 声明条数 | `IconName::ALL` | **186 组 light/dark 常量**（`idea.rs`，1141 行） |

**隔开是写进类型的，不是约定**（`rust/lithe-gpui/crates/shared/src/icons/mod.rs:8-33`）：

> 「**口径**（维护者，2026-10-08）……应用图标一律取自**本仓库自己的**图标集
> `rust/lithe-gpui/assets/icons/idea/**`（302 个文件）；我们自己没有那张图就**留空、不画**，
> 不许拿组件库（`lithe-ui`）的图标顶替。」
> 「**这个类型把口径变成类型事实**：它只装资源路径 `&'static str`，装不下
> `lithe_ui::primitives::IconName` 这类组件库图标名 —— 所以「用库图标」不再可表达。」

**实测校验**：全 `rust/lithe-gpui` 检索 `IconName::` → **7 处命中，全部是注释或断言，
无一处把库图标画进产品界面**：

| 位置 | 性质 |
|---|---|
| `app/src/assets.rs:249` | `for icon in IconName::ALL`（资源清单自检） |
| `app/src/main.rs:643` | 注释：解释为何不能调 `lithe_ui::init` |
| `explorer/src/model.rs:275` | 注释：说明**不再**用 `IconName::File` |
| `shared/src/icons/file_icon.rs:677` | 回归测试注释：`.json` 曾错映射成 `IconName::Braces` |
| `workbench/src/extensions_page.rs:265` | 注释：不拿 `IconName::Puzzle` 顶替 |
| `workbench/src/extensions_page.rs:538` | 同上，留空 |
| `workbench/src/right_tool_window.rs:71` | 注释：不拿 `IconName::Leaf` 顶替 |

**结论**：lithe-ui 的 306 个 Lucide 图标在 GPUI 产品界面里**一个都没用**，
只服务组件库自身的内部装饰（关闭 X、勾选、子菜单箭头、严重度图标）。
`app/src/assets.rs:5-10` 对此有明文：组件库组件**自身**的内部装饰图标由
`lithe_ui::Assets` 提供，应用图标一律取 `assets/icons/idea/**`。

**这直接推翻「图标字体已够用」判断的一部分**：判断的推翻条件是「IDE 关键视觉位置
所需图标在 lithe-ui 的 `IconName`/`assets/icons` 中检索不到，而 macOS 端 IDEAIcons 有对应资产」。
实测该条件成立——`locate` 在 GPUI 的 297 个 IDEA 图标里命中 0，macOS 有；且 lithe-ui 的
306 个 Lucide 图标按类型口径**根本不允许用于应用界面**。所以「lithe-ui 图标够用」
不成立：应用图标够不够，只能看 `assets/icons/idea/**`，与 lithe-ui 无关。

## 2. IDEA 图标清单完整性（可证伪的一步）

把 `rust/lithe-gpui/crates/shared/src/icons/idea.rs` 里所有 `"icons/idea/..."` 字面量
抽出，逐个 `Test-Path`：

- **引用总数 372 条，缺失 0 条**。
- 即：**已声明的图标全部有实物**，不存在「登记了但文件不在」的悬空引用。

### 2.1 ⚠️ 更正：`legacy/` 不是「另一代图标语言」

原稿据目录名 `legacy/` 推断「GPUI 用 IDEA 经典 UI、macOS 用 expui，是两代图标语言」。
**该判断被 `SOURCES.md` 与逐字节实测推翻**。

`rust/lithe-gpui/assets/icons/idea/SOURCES.md:11-15` 明写：

> 「并集补充：macOS 文件夹没有、但界面仍在用的旧图标……放在 `legacy/` 下，保留旧包的
> 相对路径……它们来自 Windows 前端 `windows/tauri/src/ui/icons/**`……
> **与 macOS 那份同属 intellij-community expui**，只是这一版更全
> （macOS 侧缺 `ui/` 那批通用字形与部分文件类型）。」

实测佐证：GPUI `legacy/ui/` 与 macOS `expui/general/` **同名 36 个文件全部逐字节相同**；
`legacy/ui/chevronDown.svg` ≡ macOS `expui/general/chevronDown.svg`（SHA-256 一致）。
**所以 `legacy/` 是「来源不同（Windows 派生）、目录名叫 legacy」的 expui 美术，
两边图形语言是同一套。** 目录名误导了原判断。

### 2.2 覆盖广度（同相对路径严格比对）

| 关系 | 条数 |
|---|---|
| 同相对路径且逐字节相同 | **162** |
| 同相对路径但内容不同 | **13** |
| GPUI 有、macOS 同路径没有（`legacy/` 那批） | **122** |
| GPUI 图标合计 | **297** svg |
| macOS 图标合计 | **304** svg |

macOS 有、GPUI 完全没有的 basename 共 **81 个**，其中与 IDE 界面直接相关的有：
`locate`、`closeSmall`/`closeSmallHovered`、`chevronUp`/`chevronUpLarge`/
`chevronDownLarge`、`open`、`paste`、`previewVertically`、`scrollDown`、`softWrap`、
`vcs`、`reformatCode`、`replace`、`revert`、`sideBySide`、`unified`、`showIgnored`、
`fetch`、`push`、`update`、`task`/`taskGroup`、`mavenProfiles`、`magicResolveToolbar`、
`viewAsImage`、`diff`、`folder_dark`、`info`、`library`/`libraryFolder`、`branchLabel`/
`currentBranchLabel`、`changesPush@12x12`/`changesUpdate@12x12`。

**具体缺口（能力点名的）**：
- `locate.svg` — GPUI 全目录 `locate*` **命中 0**；macOS 有明暗各一。
  且全 `rust/lithe-gpui` 检索 `LOCATE`/`ScrollFromSource`/`selectIn` 只命中
  `settings/src/theme.rs` 的 `locate_theme` 与 `workbench/src/debug.rs` 的
  `relocate_breakpoints`（均与图标无关）⇒ **该功能入口不存在**。
- `chevronDown.svg` — GPUI 有 `legacy/ui/chevronDown.svg`，与 macOS 同名文件逐字节相同，**可用**。
- `refresh.svg` — GPUI 只有 `actions/refresh.svg` 单张，**无 `_dark` 兄弟**；
  `idea.rs:1056-1060` `REFRESH_ICON` 是 `has_dark: false`。

### 2.3 7 张图的浅色档携带了深色美术（可复现缺陷）

13 条同路径不同内容里，**7 条是「浅色档 == macOS 的深色档」**：

| 文件 | GPUI 浅色 | macOS 浅色 | macOS 深色 |
|---|---|---|---|
| `debugger/debug.svg` | `#253627,#57965C` | `#208A3C,#F2FCF3` | `#253627,#57965C` |
| `debugger/stop.svg` | `#5E3838,#DB5C5C` | `#DB3B4B,#FFF7F7` | `#5E3838,#DB5C5C` |
| `expui/general/chevronRight.svg` | `#B4B8BF` | `#818594` | `#B4B8BF` |
| `expui/general/search.svg` | `#CED0D6` | `#6C707E` | `#CED0D6` |
| `fileTypes/powershell.svg` | `#43454A,#CED0D6` | `#6C707E,#EBECF0` | `#43454A,#CED0D6` |
| `fileTypes/text.svg` | `#CED0D6` | `#6C707E` | `#CED0D6` |
| `fileTypes/toml.svg` | `#25324D,#548AF7` | `#3574F0,#F5F8FE` | `#25324D,#548AF7` |

这 7 条的 `_dark` 兄弟与 macOS `_dark` **逐字节相同**（已验 4 条 = True；
3 条 `fileTypes/*` 无 `_dark` 兄弟）。
**即：这 7 张图深浅两档画同一张深色美术，浅色主题下偏暗。**
（另 6 条属配色微差，非极性错误。）

**是否用户可见取决于渲染路径**——GPUI 有两条语义不同的路：

| 路径 | 实现 | 颜色语义 |
|---|---|---|
| `theme_file_icon` → `gpui::img(ImageSource::Resource)` | `shared/src/icons/file_icon.rs:245-261` | **保留 SVG 内烘焙颜色** |
| `idea_icon_mono` / `idea_icon_svg_px` → `lithe_ui::Icon` → `svg()` | `shared/src/icons/mod.rs:159-186` | **丢弃颜色，按主题前景重着色** |

依据：gpui-pre 0.3.8 `Window::paint_svg`（`window.rs:4898-4906`）把 SVG 光栅化成
**alpha mask** 再填单色 `MonochromeSprite`；`svg_renderer.rs:246-250` 明写
「Convert the pixmap's pixels into an alpha mask」只取 `p.alpha()`。
调用点计数：`idea_icon_image`（走 img，**保色**）**71** 处；
`idea_icon_mono`（走 svg，**单色**）**37** 处；`idea_icon_svg_px`（单色）**15** 处。
**⇒ 走 `idea_icon_image` 的 71 处会真画出偏暗的浅色图标；另 52 处不受影响。**

### 2.4 声明的生成器校验失败（工具链缺陷）

`SOURCES.md:3` 声明本目录由
`node rust/lithe-gpui/tools/sync-macos-idea-icons.mjs --apply` 生成、「**不要手改**」。
实测跑只读 `--check`：

```
macOS 图标文件夹：304 个 SVG + 2 个附件 → 逐字节复制进 gpui
旧包（Windows 派生）：0 个 SVG
  · 保留：0（其中 0 个换成 macOS 美术、0 个继续用旧美术）
  · 删除（清单与代码都没引用）：0
idea.rs 常量：保留 63、删除 123
清单定义：删除未被 fileExtensions/filenames/folders/expandedFolders/默认值引用的 2 条
--check 失败（129 条），例如 macOS 文件 dvcs/branchLabel.svg 没被复制进来；……
[exit code: 1]
```

**129 条不一致**，且工具认为应「保留 63、删除 123」个 `idea.rs` 常量——
与现状（186 组常量、122 个 legacy 文件）相差极远。
**结论：`SOURCES.md` 描述的「生成 + 校验」链路对当前树不成立**，
目录实际是历史/手工状态，不是该工具可复现的产物。这是一处**可复现的工具链缺陷**。

## 3. 主题 token 面

### 3.1 文件与规模

`rust/lithe-ui/src/theme/` 共 **20 个 `.rs`**（含 `tests.rs`），按体量：

| 文件 | 字节 | 内容 |
|---|---|---|
| `tokens.rs` | 14266 | 尺寸/圆角/字号/高度核心 token |
| `palette.rs` | 14209 | 颜色槽位与深浅两套取值 |
| `mod.rs` | 9164 | `Theme` 全局、`Mode`、过渡、rem 锚点 |
| `tests.rs` | 6853 | token 与对比度回归 |
| `syntax.rs` | 2415 | 语法高亮主题 |
| `canvas.rs` | 1889 | 画布章节尺寸 |
| 其余 14 个 | 386–1392 | 按域分章的尺寸结构（calendar/chart/dashboard/…） |

- `theme/` 下 `pub fn` 合计 **110 个**；`pub const` 合计 **2 个**。
- `tokens.rs` 单文件 `pub fn` **72 个**（尺寸/圆角/字号入口）。

### 3.2 颜色槽位：43 + 13

- `Palette::NAMES.len() == 43`（`theme/tests.rs:60` 断言钉死）。构成：
  36 个基础槽 + chrome 三槽 `tab_bar`/`title_bar`/`status_bar` + 品牌两槽
  `primary`/`on_primary` + chrome 悬停两档 `chrome_hover`/`chrome_active`
  （加注见 `palette.rs:96-124`）。
- `Syntax::NAMES.len() == 13`（`theme/tests.rs:75`）：keyword/string/number/comment/function/
  type_name/constant/property/tag/attribute/operator/punctuation/variable。
- 另有 `chart: [Hsla; 8]`、`ansi: [Hsla; 16]` 两个数组槽（`palette.rs:170-171`）。

### 3.3 尺寸锚点机制

`theme/mod.rs:229-235` 的 `apply_rem` 是**全套尺寸唯一的缩放入口**：
token 只把设计值换算成 rem（`px_to_rems`，`tokens.rs:88-90`，除数 16），
缩放由写窗口 rem 大小完成。`base_rem()` = `font_size`（`mod.rs:232-234`）。

这与选型笔记 §6（`.agents/notes/proposed/architecture/2026-10-08-...md:236-251`）一致：
「不给不同平台写不同的像素常量」「改**主题基准字号这一处**」。
**且 `theme/platform.rs` 只有 20 行**——`Platform` 枚举只有 `Mac`/`Windows`/`Linux`
三个变体 + `current()`，**没有任何按平台分支的尺寸表**。这正是笔记 §6 要求的形态。

## 4. 字体

| | 组件库 `rust/lithe-ui/assets/fonts/` | 宿主 `rust/lithe-gpui/assets/fonts/` | macOS `macos/Resources/Fonts/` |
|---|---|---|---|
| 文件数 | **8 字体 + 3 许可** | **5** | **40 字体 + 6 许可** |
| Inter | Regular/Medium/SemiBold/Italic（**4 个 `.ttf`**） | — | **19 个 `.otf`**（Thin→Black 全档 + Italic） |
| JetBrains Mono | Medium/Regular（2 个 `.ttf`） | 5 个 | **17 个 `.ttf`** + NerdFontMono 4 个 |
| IBM Plex Sans | Regular（1 个） | — | — |

- 默认族在 `theme/mod.rs:119-121`：`font_family: "Inter"`、`mono_family: "JetBrains Mono"`、
  `font_size: 16.0`、`mono_font_size: 13.0`。
- 宿主另有独立内嵌：`rust/lithe-gpui/crates/settings/src/fonts.rs:11-28`
  （JetBrainsMono Bold/BoldItalic/Italic/Regular），`BUILTIN_FONT_FAMILY = "JetBrains Mono"`（`:28`）。
- **缺口**：`lithe-ui` 的 Inter 只有 4 档，**无 Bold、无 Black**；
  而 macOS 的 `BundledUIFontTests` 要求「Regular、Medium、SemiBold、Bold 和 Black 请求
  分别匹配真实 Inter 字型」。GPUI 侧 Bold/Black 无真实字型可匹配。

## 5. 文案（i18n）

| | 文件 | 体量 |
|---|---|---|
| 组件库 | `rust/lithe-ui/locales/ui.yml` | **62 行 / 9109 字节**，顶层键 **6 个** |
| 宿主 | `rust/lithe-gpui/crates/shared/locales/lithe.en.yml` | **8977 行 / 282209 字节** |
| 宿主 | `rust/lithe-gpui/crates/shared/locales/lithe.zh-CN.yml` | **293125 字节** |

- 组件库文案走 `rust_i18n`，键名表意图不写句子；每叶列 7 个语言标签
  （`en-US`/`en-GB`/`de-DE`/`fr-FR`/`ja-JP`/`he-IL`/`zh-CN`），含 `he-IL` 的 RTL 覆盖。
- 宿主只有 `en` 与 `zh-CN` 两档，体量是组件库的 **145 倍**。
- **口径分裂**：组件库 7 语、宿主 2 语。组件库组件显示 7 语，产品文案只显示 2 语。

## 6. 硬编码色值（对判断「单主题源成立」的读数）

实测两条检索：

| 检索 | 命中 |
|---|---|
| `rust/lithe-gpui` 非 test 代码 `rgb(0x` / `rgba(0x` / `\b0x[0-9a-fA-F]{6}\b` | **0** |
| `rust/lithe-gpui` 非 test 代码 `"#rrggbb"` 字面量 | **0** |
| `rust/lithe-ui/src` 非 test 代码 `rgb(0x` / `rgba(0x` / `\b0x[0-9a-fA-F]{6}\b` | **23** |
| `rust/lithe-ui/src` 非 test 代码 `"#rrggbb"` 字面量 | **19** |

lithe-ui 的 23 处色值字面量里，**20 处落在测试文件**
（`forms/tests/colors.rs`、`i18n/tests/a11y.rs`、`settings/vscode.rs` 的断言、
`devtools/contrast.rs:195`、`tooling/a11y.rs:233` 的对比度样例），
**3 处是设计真源本身**：`theme/syntax.rs:67-83`（语法主题的两套色板）。
`palette.rs` 的 43 槽取值是设计意图，不是「第二套」。
19 处 hex 字符串全部在测试或 VS Code 主题解析样例里。

**结论**：**「单主题源成立」被支持**。宿主零硬编码色，全部经
`lithe_ui::theme::Theme` 取色；组件库的色值集中在 `palette.rs` + `syntax.rs` 两个真源文件。

## 7. 宿主如何消费组件库主题（接线现状）

真源声明（`rust/lithe-gpui/crates/settings/src/theme.rs:632-635`）：

> 「宿主主题的真源只有一个：`lithe_ui::theme::Theme`。本函数记下活动主题名，
> 颜色与字体都往那一个全局写。」

调用链（全部带行号）：

| 环节 | 位置 |
|---|---|
| 主题全局初始化 | `crates/app/src/main.rs:638` `Theme::init(cx);` |
| 主题 JSON 内嵌 | `crates/settings/src/theme.rs:15-16` `include_str!("../../../themes/jetbrains.json")` |
| IDEA 主题文件 | `rust/lithe-gpui/themes/jetbrains.json`（259 行，含 `IntelliJ Light` + `Darcula`） |
| 主题 → 调色板 | `crates/settings/src/theme.rs:348` `fn palette_for(record: &ThemeRecord) -> Palette` |
| 落进组件库全局 | `theme.rs:650` `ComponentTheme::apply(record.mode.to_component(), Some(palette_for(&record)), cx)` |
| 字号注入 | `theme.rs:669` `ComponentTheme::update(cx, \|theme\| theme.font_size = font_size)` |
| 编辑器字号注入 | `theme.rs:678` `theme.mono_font_size = font_size` |
| 字体族注入 | `theme.rs:695-702` `ComponentTheme::update(...)` 写 `font_family`/`mono_family` |
| 设置变更重放 | `crates/settings/src/store.rs:1024` `theme::apply_theme_by_id(cx, &applied)` |
| 主题文件热重载 | `theme.rs:607-621` watcher 收到变更即 `reapply_theme` |

**宿主主题真源是 `rust/lithe-gpui/themes/jetbrains.json`，不是 `lithe-ui/src/theme/palette.rs`。**
`palette_for` 把 IDEA 主题键映射到组件库 43 槽；**没给的栏保留组件库默认值**
（`theme.rs:343`「没给的栏保留组件库自己的默认值」）。

**关键后果**：`lithe-ui/src/theme/palette.rs` 里那套**暖中性色**（`bg: 0xfcfaf7`、
`surface: 0xffffff`、`fg: 0x181613`、`accent: 0x181613`——见 `palette.rs:211-257`）
是 **Ely 的通用调色板，不是 IDEA 的**。宿主一旦应用主题，这些默认值就被
IDEA 键覆盖；**未被 `palette_for` 覆盖的槽（如 `sunken` 之外的 `shimmer`/`glass`/`paper`/`ink`
与整个 `ansi` 数组）会残留 Ely 的暖色**。`theme.rs:343` 明说「宁可用组件库的默认值」——
这是有意的取舍，但它意味着**默认观感不是 IDEA 的**。

## 8. 画廊

- `rust/lithe-ui/examples/gallery/pages/` 有 **44 个页面目录**（account/agent/buttons/
  canvas/charts/collab/dashboard/data/debug/devtools/documents/editor/feedback/finance/
  forms/generative/git/interaction/layout/lists/media/menus/motion/navigation/onboarding/
  overlays/primitives/settings/shell/tables/terminal/typography/…）。
- `pages/mod.rs` 的 `ALL` 数组注册；`stories.json` 顶层只有 `pages` 一个键。
- 有 `pages/typography/`（排版）与 `pages/primitives/` 页面，可作为 token 视觉回归的挂点。

## 9. 未执行的检索与测试

- **未运行任何构建、测试或画廊**。本文件全部是静态清点（`Get-ChildItem` / `Select-String` /
  `Test-Path` / `ConvertFrom-Json`）。
- 未做像素级或截图比对；「取值不同」是**读源码常量得出的**，不是渲染测量。
- 未在 macOS 实机运行；macOS 侧数字来自仓库内资源清单，未安装验证。
- 未验证 GPUI 前端运行时实际用的是哪个主题（需跑起来看 `S1_THEME` 打点），
  故 §7 的「宿主真源是 jetbrains.json」是**代码路径推断**，不是运行观测。
