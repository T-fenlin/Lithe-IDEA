# GPUI 宿主如何消费组件库主题（接线现状）

基线 `3085f5d1`，分支 `feat/menu-bar-opaque`。判定目标：**组件库主题是否真被宿主用起来、
有没有第二套颜色**。

## 1. 真源声明

`rust/lithe-gpui/crates/settings/src/theme.rs:632-635` 原文：

> 「宿主主题的真源只有一个：`lithe_ui::theme::Theme`。本函数记下活动主题名，
> 颜色与字体都往那一个全局写。」

类型别名在 `theme.rs:8`：
`use lithe_ui::theme::{Mode as ComponentMode, Palette, Theme as ComponentTheme};`
——**宿主的 `ComponentTheme` 就是组件库的 `Theme`，不是自己的类型。**

## 2. 调用链（全部带行号）

| 环节 | 位置 |
|---|---|
| 主题全局初始化 | `crates/app/src/main.rs:638` `Theme::init(cx);` |
| 键位安装（不碰资产） | `crates/app/src/main.rs:649` `lithe_ui::install_key_bindings(cx);` |
| 字体注册 | `crates/app/src/main.rs:664` `lithe_gpui_settings::register_fonts(cx);` |
| 主题文件热重载 | `crates/app/src/main.rs:665` `lithe_gpui_settings::watch_lithe_themes(cx);` |
| 主题 JSON 内嵌 | `crates/settings/src/theme.rs:15-16` `include_str!("../../../themes/jetbrains.json")` |
| 主题→调色板 | `crates/settings/src/theme.rs:348` `fn palette_for(record: &ThemeRecord) -> Palette` |
| 落进组件库全局 | `crates/settings/src/theme.rs:650` `ComponentTheme::apply(record.mode.to_component(), Some(palette_for(&record)), cx);` |
| 字号注入 | `crates/settings/src/theme.rs:669` `ComponentTheme::update(cx, \|theme\| theme.font_size = font_size);` |
| 编辑器字号注入 | `crates/settings/src/theme.rs:678` `theme.mono_font_size = font_size;` |
| 字体族注入 | `crates/settings/src/theme.rs:695-702` 写 `font_family` / `mono_family` |
| 设置变更重放 | `crates/settings/src/store.rs:1024` `theme::apply_theme_by_id(cx, &applied);` |
| 主题目录变更监听 | `crates/settings/src/theme.rs:607-621` watcher → `reapply_theme` |
| 测试期初始化 | `crates/workbench/src/search_everywhere.rs:772`、`workspace.rs:3418/3469` |

## 3. 一个刻意的例外（值得记下）

`crates/app/src/main.rs:643-645` 注释：

> 「这里不能调 `lithe_ui::init`：它开头就 `load(IconName::Check.path())`
> （`icons/check.svg`），而本仓挂的资产源是 `LitheAssets`（`rust/lithe-gpui/assets`），
> 那个路径不在里面，启动即 panic。`install_key_bindings` 只装键位，不碰资产。」

——所以宿主**只调 `lithe_ui::install_key_bindings`，不调 `lithe_ui::init`**。
资产由宿主的 `LitheAssets` 承担，并把 `lithe_ui::Assets` 作为回落
（`crates/app/src/assets.rs:56-74`）。**这条是「两个资产源合并」的关键接缝。**

## 4. 宿主是否有第二套颜色：没有

实测三条检索（`rust/lithe-gpui` 全目录，123 个 `.rs`）：

| 检索式 | 命中 |
|---|---|
| `rgb(0x` / `rgba(0x` / `\b0x[0-9a-fA-F]{6}\b` | **0** |
| `"#rrggbb"` 形式的 hex 字符串字面量 | **0** |

对照 `rust/lithe-ui/src` 非测试代码：`rgb(0x`/`rgba(0x`/`0xRRGGBB` **23 处**，
其中 **20 处在测试文件**（`forms/tests/colors.rs`、`i18n/tests/a11y.rs`、
`settings/vscode.rs` 断言、`devtools/contrast.rs:195`、`tooling/a11y.rs:233`），
**3 处是设计真源本身**（`theme/syntax.rs:67-83` 的两套语法色板）。
19 处 hex 字符串全部在测试或 VS Code 主题解析样例里。

**结论：宿主零硬编码颜色，全部经 `lithe_ui::theme::Theme` 取色。**
判断「单主题源成立」**被支持**。

## 5. 但「默认观感不是 IDEA 的」

`crates/settings/src/theme.rs:343` 明文：

> 「……**没给的栏保留组件库自己的默认值**」

而 `palette_for`（`theme.rs:361-493`）的映射清单**覆盖的键**是：
`background`→`bg`+`surface`、`tab_bar.background`、`title_bar.background`、
`status_bar.background`、`muted.background`→`sunken`、`list.hover.background`→`hover`、
`list.active.background`→`active`、`overlay`→`backdrop`、`popover.background`→`overlay`、
`border`→`border`+`border_strong`、`foreground`→`fg`、
`muted.foreground`→`fg_muted`+`fg_subtle`+`fg_disabled`、
`accent.background`→`accent`+`accent_hover`、`accent.foreground`→`on_accent`、
`primary.background`→`primary`、`primary.foreground`→`on_primary`、
`ring`→`focus`+`link`、`sidebar.accent.background`→`selection`、
`success.background`、`warning.background`、`danger.background`、`info.background`，
外加由三键推导的 `chrome_hover`（`:406-414`）与 `chrome_active`（`:419-432`）。

**未被覆盖的槽**（保留 `Palette::light/dark` 的 Ely 默认值）至少包括：
- **`tooltip_bg` / `tooltip_fg`** ← 直接导致浅色主题下 tooltip 呈近黑底浅字
  （`palette.rs:254` 浅色 `0x181613`、`:334` 深色 `0x373533`）
- `ansi: [Hsla; 16]`（终端 16 色）
- `shimmer`、`glass`、`paper`、`ink`、`on_media`、`media_backdrop`
- `*_subtle` 四槽（`success_subtle`/`warning_subtle`/`danger_subtle`/`info_subtle`）
- `chart: [Hsla; 8]`（除非开了 `color_blind_safe`，见 `theme/mod.rs:245-247`）

`lithe-ui` 的默认调色板是**暖中性色**（`palette.rs:211-257`）：
`bg 0xfcfaf7`、`surface 0xffffff`、`fg 0x181613`、`accent 0x181613`、`sunken 0xf6f4f1`
——**这是 Ely 的通用调色板，不是 IDEA 的**。

## 6. 主题真源文件

`rust/lithe-gpui/themes/jetbrains.json`（**259 行**），两条主题：
`IntelliJ Light`（`id: intellij-light`，`mode: light`）与 `Darcula`（`id: darcula`，`mode: dark`）。
每条的键面：`font.size: 16`、`radius: 6`、`radius.lg: 8`，以及
`background`/`foreground`/`border`/`input.border`/`ring`/`accent.*`/`muted.*`/
`secondary.*`/`popover.*`/`selection.background`/`caret`/`overlay`/`primary.*`/
`danger.*`/`success.*`/`warning.*`/`info.*`/`button.*`/`list.*`/`table.head.foreground`/
`tab.*`/`tab_bar.background`/`sidebar.*`/`title_bar.background`/`status_bar.background`/
`base.{red,green,blue,yellow,magenta,cyan}[.light]`，
外加 `highlight.editor.background`/`editor.foreground`/`editor.active_line.background`
与 `highlight.syntax.*`（16 个 token）。

注意：文件里**有 `radius: 6` 与 `radius.lg: 8` 两个键**，但 `palette_for` 是
「颜色 → `Palette`」的映射，**不处理 `radius`**。`Theme::radius()` 读的是
`Theme::radius_scale`（`tokens.rs:491`），而实测 **`rust/lithe-gpui` 全目录检索
`radius_scale`/`density`/`Density` 命中 0** ——即宿主**从未设置** `radius_scale`
与 `density`，两者恒为初始值 `1.0`（`theme/mod.rs:116`）与 `Density::Standard`
（`theme/mod.rs:115`）。**主题文件里的 `radius`/`radius.lg` 目前是死键。**

## 7. 结论

- **判断「单主题源成立」被支持**：宿主 0 硬编码色值，全部经组件库 `Theme` 全局取色，
  连类型都是同一个。
- **但有两处「声明了却没接通」**：
  1. 主题文件的 `radius` / `radius.lg` **无人消费**（宿主不设 `radius_scale`）。
  2. `palette_for` 不覆盖 `tooltip.*`、`ansi`、`chart` 等槽，
     **未给时保留 Ely 暖色默认值**——这是「默认观感不是 IDEA」的机制来源。

## 8. 未执行的检索与测试

- **未运行任何构建或测试**；§2 的调用链是**读源码得出的静态路径**，
  不是运行观测。**未验证运行时 `S1_THEME` 打点实际输出了什么。**
- 未在 macOS 实机验证；未启动画廊。
- `radius_scale`/`density` 「宿主从未设置」是从「检索命中 0」推出的；
  若存在动态设置（如配置文件驱动而字面量不出现），本结论会被推翻。
