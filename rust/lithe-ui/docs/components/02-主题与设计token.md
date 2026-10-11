# 02 · 主题与设计 token

覆盖模块：`src/theme/`（`mod.rs`、`palette.rs`、`tokens.rs`、`syntax.rs`、`platform.rs`、各 `*_sizes.rs`）

## 本域速查表

| 名称 | 类型 | 一句话 |
| --- | --- | --- |
| `Theme` | 全局 | 每 app 一份的设计 token：`colors` / `density` / `radius_scale` / `font_scale` / `font_size` / `font_family` / `mono_font_size` / `mono_family` / `reduced_motion` / `platform` |
| `ActiveTheme` | trait | `cx.theme()` 的来源 |
| `Palette` | 数据 | 语义色板（`Palette::light(hc)` / `Palette::dark(hc)`） |
| `Syntax` / `SyntaxTheme` / `syntax_themes` | 数据 / 函数 | 代码配色（关键字、字符串、注释…）与内置配色表 |
| `Mode` / `Platform` | 枚举 | 明暗 / 平台约定（窗口按钮、标题栏布局、**键帽写法**） |
| `Density` / `Size` / `TextSize` / `Radius` / `IconSize` / `AvatarSize` / `ContainerSize` / `Elevation` | 枚举 | 尺寸与层级档位 |
| `Sizable` | trait | 组件尺寸档位：`xsmall()` / `small()` / `medium()` / `large()` |
| `Mix` | trait | 色板插值（主题切换的淡入淡出就靠它） |
| `HUE_NAMES` | 常量 | 图表八个色名（Blue、Teal、Ochre、Rose、Violet、Green、Rust、Cyan） |
| `*Sizes`（Calendar/Canvas/Chart/Dashboard/File/Generative/Interaction/Map/Media/Messaging/Misc/Onboarding/Project/Tooling） | 数据 | 各域自己那一章的度量 |

---

## 怎么读主题

```rust
let theme = cx.theme();                 // &Theme
let colors = &theme.colors;             // &Palette
let h = theme.control_height(Size::Small);
let r = theme.radius(Radius::Md);
let t = theme.text_size(TextSize::Sm);
let shadow = theme.elevation(Elevation::Floating);
let dark = theme.is_dark();
```

### 常用取值（`src/theme/tokens.rs`）

| 方法 | 取值（100% 缩放下的设计 px） |
| --- | --- |
| `control_height(Size)` | XSmall 20 / Small 24 / Medium 28 / Large 32，密度 Compact −4、Comfortable +4 |
| `control_padding(Size)` | 6 / 8 / 12 / 16（密度 ±2） |
| `text_size(TextSize)` | Xs 11 / Sm 12 / Base 13 / Md 14 / Lg 16 / Xl 20 / Xxl 24 / Display 32 |
| `radius(Radius)` | Sm 4 / Md 6 / Lg 8 / Xl 12，再乘 `radius_scale` |
| `icon_size(IconSize)` | Xs 12 / Sm 14 / Md 16 / Lg 20 / Xl 24 / Xxl 32 |
| `avatar_size` | Xs 20 / Sm 24 / Md 32 / Lg 40 / Xl 56 |
| `container_width` | Sm 640 / Md 960 / Lg 1200 |
| `titlebar_height` / `sidebar_width` / `menu_width` / `label_width` | 38 / 240（折叠 56）/ 200 / 160 |
| `tab_width` / `rail_width` / `dialog_width` / `toast_width` | 96–200 / 48 / 440 / 360 |
| `elevation(Elevation)` | Raised / Floating / Modal 三层阴影，深浅各一套透明度 |

尺寸一律返回 `Rems`：`px_to_rems(v) = rems(v / 16.0)`，真正的缩放由 `Theme::apply_rem(window)` 写窗口的 rem 大小完成（`tokens.rs:88`、`mod.rs:233`）。**不要在组件里自己乘缩放系数。**

### 语义色板（`src/theme/palette.rs:125-173`）

```
bg, surface, tab_bar, title_bar, status_bar, sunken, overlay,
hover, active, chrome_hover, chrome_active, border, border_strong,
fg, fg_muted, fg_subtle, fg_disabled,
accent, accent_hover, on_accent, primary, on_primary, on_media,
focus, link, selection,
success, warning, danger, info, success_subtle, warning_subtle, danger_subtle, info_subtle,
backdrop, media_backdrop, shimmer, glass, shadow,
tooltip_bg, tooltip_fg, paper, ink
```

默认值：

| token | 浅色（`Palette::light`） | 深色（`Palette::dark`） |
| --- | --- | --- |
| `bg` | `#fcfaf7` | `#131110` |
| `surface` | `#ffffff` | `#1b1917` |
| `tab_bar` | `#ffffff` | `#1b1917` |
| `title_bar` | `#f2f2f2` | `#313335` |
| `status_bar` | `#f2f2f2` | `#313335` |
| `sunken` | `#f6f4f1` | `#0e0d0b` |
| `overlay` | `#ffffff` | `#201f1d` |
| `hover` / `active` | `#f2f0ed` / `#e9e7e5` | `#252422` / `#2c2b29` |
| `chrome_hover` / `chrome_active` | `#e1dfdd` / `#cccac8` | `#413f3d` / `#5c5a58` |
| `border` / `border_strong` | `#e1dfdd` / `#cccac8` | `#2c2b29` / `#413f3d` |
| `fg` / `fg_muted` / `fg_subtle` / `fg_disabled` | `#181613` / `#605d5a` / `#696764` / `#acaaa8` | `#f3f1f0` / `#afadab` / `#94928f` / `#5c5a58` |
| `focus` / `link` | `#3772bb` / `#2863ab` | `#69a1e8` / `#7eb1f3` |

> **chrome 槽位**：`tab_bar` / `title_bar` / `status_bar` 是"窗口外壳"专用的三个槽（`palette.rs:96-124` 有说明）。JetBrains 主题给它们的值与页面底色不同（浅色 `#ffffff` / `#f2f2f2` / `#f2f2f2`，深色 `#2b2b2b` / `#313335` / `#313335`），既不能合并成一个槽，也不能拿 `surface` 顶替。库默认调色板里这两个槽取的就是 JetBrains 的那一档（`#f2f2f2` / `#313335`），与页面底色 `bg` 不同值。

#### 悬停/选中取哪个槽位：看它画在什么底上

> **悬停是「从它所在底色上迈的一档」，不是一个到处通用的绝对色。** 高亮槽位由**它画在什么底上**决定：
>
> - 画在 `list.background`（浅色纯白）上 → `hover` / `active`
> - 画在 chrome 面上（`title_bar` / `status_bar` / `popover` / `overlay` / `sunken` / `sidebar.background`）→ `chrome_hover` / `chrome_active`
>
> 在 JetBrains 浅色下，后面这一整排底色**全是 `#f2f2f2`**，与 `hover` 逐位相同 —— 把 `hover` 画上去等于没画（实测对比度 **1.0000**）。`chrome_active` 必须比 `chrome_hover` 更强、两者不同色（`theme/tests.rs` 的 `the_chrome_hover_and_active_stand_out_on_the_chrome_surfaces` 断言了这一点，下限 1.10:1）。
>
> **否定式提示**：这不是「`hover` 这个色不对」，是「`hover` 的服务对象是白底列表」。**别去改 `hover` 的取值** —— 要换的是**调用点的槽位**。

#### chrome 面上不要放常态填充

> 在一个所有常态色都等于 chrome 底色的主题里（JetBrains 浅色下 `button` / `secondary` / `accent` / `muted` / `popover` / `title_bar` / `status_bar` / `sidebar` 的 `.background` 全是 `#f2f2f2`），**chrome 面上的「常态填充」这个概念本身不成立** —— 要么不铺，要么用状态色（悬停/选中族），**没有第三条路**。
> 所以 chrome 面上的元件只能靠**描边、投影、文字/图标色**区分层次。
>
> 这不是单个主题的个别缺陷，而是这类「扁平浅灰」配色方案的结构性约束：它把 8 个语义不同的 `.background` 键收敛成了同一个色值，于是 chrome 面上任何「用底色区分元件」的设计手段都失效了。
> **设计新组件时先默认透明**，需要轮廓就上描边，不要试图给它一个常态底色。

#### 悬停态与选中态叠在同一个元素上时（gpui builder 顺序）

`.hover()` 的样式在 gpui 里是**最后** refine 上去的。gpui-pre 0.3.8 `src/elements/div.rs` 里三处 `style.refine(..)` 按源码顺序是：

| 行号 | refine 的内容 |
| --- | --- |
| `:3426` | `focus_style`（`.focus(..)`） |
| `:3454` | `group_hover.style`（`.group_hover(..)`） |
| `:3473` | `hover_style`（`.hover(..)`） |

后写的覆盖先写的，所以**鼠标停在已选中/已聚焦的行上时，`.hover()` 那一档会盖掉 `.when(..)` / `.focus(..)` 铺的背景**，该行反而掉回悬停档，看着像「选中丢了」。

**若选中态与悬停态是两行独立代码，务必把 hover 挂在 `!= chosen` 上**：

```rust
.when(ix != chosen, |row| row.hover(|row| row.bg(colors.chrome_hover)))
```

档位判据是「**有几处代码、两态会不会分离**」，不是「它叫什么名字」：

| 情况 | 档位 | 例子 |
| --- | --- | --- |
| 两行独立代码、两态可分离 | **选中强、悬停弱**，给成阶梯 | `editor/hints.rs` 的补全候选行：`.when(ix == chosen)` 与 `.hover(..)` 各一处，本文件无 `on_mouse_move` 把光标拖到悬停行 |
| 一处代码兼两态（`on_mouse_move` 让光标行 ≡ 悬停行） | 分不出阶梯，**只能取最强档** `chrome_active` | `navigation/palette/mod.rs` 的候选行 |

判定底色时要**追到最外层容器**：`render_panel()` 自己没有 `.bg(` 不代表它透明，要看它挂进去的那个卡片/浮层铺了什么。

---

## 宿主怎么接管

```rust
Theme::init(cx);                                     // 启动时装一次全局（src/theme/mod.rs:107）
Theme::apply(Mode::Dark, Some(palette), cx);          // 宿主自带配色：换成宿主的（立即生效）
Theme::set_mode(Mode::Light, cx);                     // 只切明暗（带淡入淡出）
Theme::set_mode_now(Mode::Dark, cx);                  // 立刻切换，不做过渡
Theme::set_high_contrast(true, cx);                   // 高对比
Theme::set_color_blind_safe(true, cx);                // 图表换色盲友好色
Theme::set_palette(Mode::Light, Some(palette), cx);   // 只改某一个模式的色板
Theme::update(cx, |theme| theme.font_size = 16.0);    // 改非颜色 token（会把新 rem 标到所有窗口）
window.set_rem_size(cx.theme().base_rem() * cx.theme().font_scale);  // 或 Theme::apply_rem(window, cx)
```

- `Theme::apply(mode, Some(palette), cx)`：宿主用自己的整套配色（Lithe 的 `lithe-gpui` 就是把 `themes/jetbrains.json` 映成 `Palette` 后调它，见 `rust/lithe-gpui/crates/settings/src/theme.rs`）。
- `None` 表示回到库自己的配色。
- 换模式默认走 `fade_to_target`（8ms 一帧、按 `motion::THEME` 时长插值，靠 `Palette: Mix`）。

## 高对比与色盲

- `high_contrast` 只改四件事：`fg_muted`、`fg_subtle`、`border`、`border_strong` 加深/加强，`glass` 退回 `bg`（`palette.rs:261`、`337`）。
- `color_blind_safe` 只换图表八色（`color_blind_chart`，`palette.rs:175`），`HUE_NAMES` 仍是同样的名字。

## 尺寸与密度

```rust
theme.density = Density::Compact;      // 控件高度 −4、内边距 −2
theme.radius_scale = 1.0;              // 圆角整体缩放
theme.font_scale = 1.0;                // 用户缩放 85%–130%
theme.font_size = 16.0;                // 基准字号（px）= 100% 时的窗口 rem 大小
theme.mono_font_size = 13.0;           // 编辑器字号，与 UI 基准是两个不同的量
```

`Size::Size(px(30.))` 可以传一个明确的高度，此时密度不再缩放它（`tokens.rs:96`）。

## 平台差异

`Platform::current()` 决定：窗口按钮的画法、标题栏布局、**键帽写法**（Mac 显示 ⌘/⇧ 等符号，其它平台拼 `Ctrl+S` 并在键间加 `+`）。`Kbd` / `KbdCombo` 就是读它（`typography/inline.rs:76`、`:108`）。

## 代码配色

```rust
let themes = syntax_themes();                 // 内置配色表
let syntax = &cx.theme().colors.syntax;       // 当前生效的语法色（keyword/string/comment/…）
```

`Syntax` 的字段：`keyword`、`string`、`number`、`comment`、`function`、`type_name`、`constant`、`property`、`tag`、`attribute`、`operator`、`punctuation`、`variable`。

## 各域度量

每个域有自己的一章度量，例如 `ChartSizes`、`FileSizes`、`CalendarSizes`。组件内部读它们，调用方一般不用直接取；要对齐某个域的既有尺寸时再读。

## 组件侧的红线

1. **不写死颜色**：一律 `cx.theme().colors.*`。需要透明用 `gpui::transparent_black()`，不要用 `rgba(0x00000000)` 之类。
2. **不写死尺寸**：用 `control_height` / `text_size` / `radius` / `icon_size`，或各域 `*Sizes`。
3. **不在组件里乘缩放**：缩放只有 `apply_rem` 一个入口。
4. **深浅都要成立**：新增颜色必须同时给 `Palette::light` 与 `Palette::dark` 两套值。
5. **chrome 与页面底色分开**：标题栏/标签栏/状态栏用 `title_bar` / `tab_bar` / `status_bar`，不要用 `bg`。
