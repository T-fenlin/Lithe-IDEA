# `lithe-gpui/themes` —— Lithe 的**内置** gpui-kit 主题文件

⚠️ **本目录不再是运行时的监视目录**（2026-09-26 改）。运行时的主题目录是**用户配置目录**下的
`themes/`：Windows `%APPDATA%\Lithe\themes\`、macOS `~/Library/Application Support/Lithe/themes/`、
Linux `$XDG_CONFIG_HOME/lithe/themes/`（解析在 `crates/settings/src/paths.rs::themes_dir`）。

为什么必须改：旧实现把 `concat!(env!("CARGO_MANIFEST_DIR"), "/../../themes")` 当运行目录，
那是**构建树路径**，打包成安装包后不存在 —— 表现是主题功能整体失效、而且不报错。

现在的形状是**播种（seed）**：

```text
启动 → crates/settings/src/theme.rs 的 BUNDLED_THEMES（include_str! 编译进二进制的本目录内容）
     → 写到 <配置目录>/themes/（已存在的文件一个字节都不动）
     → ThemeRegistry::watch_dir(<配置目录>/themes/) 装载 + 监听
```

为什么是播种而不是"内置 + 用户两个目录都装载"：上游只支持**一个**目录 ——
`ThemeRegistry::watch_dir` 里 `self.themes_dir = themes_dir` 是覆盖写，`reload()` 只读它
（`gpui-component-0.6.6/src/theme/registry.rs:102,241-262`）。调两次 `watch_dir` 只有最后一次生效。

**代价（明确登记）**：用户目录里那 7 份是**首次启动那一次的副本**，程序升级后不会自动更新
（"已存在就不动"是保护用户修改的必然结果）。想拿到新版的某一份，删掉
`<配置目录>/themes/<文件>` 再重启即可重新播种。

**本目录的双重身份**：

| 用途 | 谁用 |
| --- | --- |
| `include_str!` 的播种源 | `crates/settings/src/theme.rs::BUNDLED_THEMES` |
| 测试与开发时的格式校验对象 | `every_theme_file_is_a_loadable_theme_set`（跑 `cargo test -p lithe-gpui-settings theme::`） |

所以往本目录丢文件**仍然**要同时改一行 Rust：在 `BUNDLED_THEMES` 里登记，否则运行期看不到它
（`every_theme_file_is_embedded_and_loadable` 会拿本目录的文件清单与那张表比对，漏登记就是测试失败）。

其余加载事实（只扫顶层、`themes[].name` 去重、`mode` 决定明暗、13 个无 fallback 的 key、
颜色值的合法写法、两个会被强制改写的 alpha）见下文各节 —— 那些**没有变**，变的是"目录在哪"
与"文件怎么到用户目录"。

| 文件 | 主题 `name`（`id`，`mode`） | colors 条数 | `highlight`（编辑器语法色） |
| --- | --- | --- | --- |
| `lithe-dark.json` | `Lithe Dark`（`lithe-dark`，dark） | 60 | 无（§6.1） |
| `lithe-light.json` | `Lithe Light`（`lithe-light`，light） | 60 | 无（§6.1） |
| `gruvbox.json` | `Gruvbox Light`（`gruvbox-light`，light）/ `Gruvbox Dark`（`gruvbox-dark`，dark） | 各 60 | 有（§7） |
| `jetbrains.json` | `IntelliJ Light`（`intellij-light`，light）/ `Darcula`（`darcula`，dark） | 各 60 | 有（§7） |
| `nord.json` | `Nord Light`（`nord-light`，light）/ `Nord Dark`（`nord-dark`，dark） | 各 60 | 有（§7） |
| `one.json` | `One Light`（`one-light`，light）/ `One Dark`（`one-dark`，dark） | 各 60 | 有（§7） |
| `vscode.json` | `VS Code Light+`（`vs-code-light`，light）/ `VS Code Dark+`（`vs-code-dark`，dark） | 各 60 | 有（§7） |

本目录共 **12 条主题**（注册表里另有内置的 `Default Light` / `Default Dark`，共 14 条）。
`themes[].name` 必须**全局唯一**：重名条目被注册表直接跳过（`registry.rs:270-273`），表现是"少一个主题"。
`themes[].id` 也必须唯一（同一份索引里 id 或名字重复时**先出现的赢**）。

`Lithe Dark` / `Lithe Light` 的 `id`（`lithe-dark` / `lithe-light`）与 Windows 侧默认主题的 id 一致
（`windows/tauri/src/features/settings/config/default-settings.ts:99`，见 `lithe-gpui/UI-MAP-WINDOWS.md:29`），
所以同一份设置值在两端含义相同。第二批 5 个文件与 `windows/tauri/src/extensions/themes/builtin/`
里的同名族是**同一批设计值**，但格式不同（那张表是 Lithe schema，这张是 gpui-kit schema），
映射与新增的 `highlight` 段见 §7。

---

## 0. 主题标识：`themes[].id` 与 `themes[].name`

| | 用途 | 谁读 | 能不能改 |
| --- | --- | --- | --- |
| `themes[].name` | 注册表的索引键、界面显示名 | `ThemeRegistry`（上游）、菜单栏、下拉的 label | 改了要连带想清楚（见下） |
| `themes[].id` | **设置文件里持久化的值**（`lithe-dark`） | 我们自己的 `crate::schema::ThemeIndex` | 改了等于断掉所有用户的引用 |

- `id` 是**我们加进主题文件的字段**，上游 `ThemeConfig` 里没有它、也不认识它 ——
  `ThemeSet` / `ThemeConfig` 没有 `deny_unknown_fields`，所以它被静默忽略，不影响装载。
- 没写 `id` 的主题（用户手写的文件、gpui-kit 内置的 `Default Light` / `Default Dark`）按
  **显示名 slug 化**兜底：`"VS Code Light+"` → `vs-code-light`。
- 因此改 `name` 时**保留 `id` 不变**，用户的引用就不会断；这正是持久化 id 而不是名字的理由。
- 老设置文件里存的是显示名（升级前的行为）：读入时由 `Settings::normalize_with_themes`
  （及 `store::apply_theme_for`）归一成 id，**用户不需要做迁移**。


---

## 1. 主题文件的确切 schema

### 1.1 文件根是 `ThemeSet`，**不是** `ThemeConfig`（最容易踩的坑）

一个 JSON 文件 = 一个 `ThemeSet`（`src/theme/schema.rs:22-34`，`#[serde(default)]`，无 `deny_unknown_fields`）：

```text
ThemeSet { name: SharedString, author: Option<SharedString>, url: Option<SharedString>, themes: Vec<ThemeConfig> }
```

加载路径只有一条：`serde_json::from_str::<ThemeSet>(&file_content)`（`registry.rs:248`，另见 `registry.rs:152` 的 `load_themes_from_str`）。
解析失败的文件被**整份忽略**并只打日志（`registry.rs:252-258`），不会部分生效。

⚠️ 官方文档 `https://gpui-kit.com/zh-CN/component/theme.md` 里出现的

```json
{ "colors": { "button.primary.background": "#4F46E5" } }
```

只是 `colors` 字段的**片段示意**，不是完整文件。把它当成整份文件去写，`ThemeSet.themes` 会是空数组，
文件语法合法但**一个主题都载不进来**（未知字段被静默忽略）。完整文件必须带 `themes` 数组。

| 加载事实 | 证据 |
| --- | --- |
| 只扫描监视目录下扩展名为 `json` 的**普通文件**（非递归收集，但 notify 监听是递归的） | `registry.rs:242-262` |
| 注册表自己有个默认目录 `./themes`（相对进程工作目录）；**我们从不使用它** —— `watch_lithe_themes` 总是显式传 `<配置目录>/themes` | `registry.rs:174`、`paths.rs:themes_dir` |
| 监视目录不存在时注册表会自动创建（我们播种时也会先建） | `registry.rs:187-189` |
| 主题按 `themes[]` 条目的 **`name`** 去重，**同名条目被跳过**（不覆盖） | `registry.rs:154`、`:270-273` |
| `is_default: true` 会写进 `default_themes[theme.mode]` | `registry.rs:275-278` |
| 一个文件可以放多条主题（一条一个明暗） | `registry.rs:250`（`themes.extend(theme_set.themes)`） |

### 1.2 `ThemeConfig` 的确切字段（`schema.rs:36-82`，`#[serde(default)]`）

**没有** `typography` / `appearance` 字段。字体与圆角是「点分名字 → 展开成独立字段」：

| Rust 字段 | JSON key | 类型 | 默认 | `apply_config` 落点 |
| --- | --- | --- | --- | --- |
| `is_default` | `is_default` | bool | `false` | `registry.rs:275-278` |
| `name` | `name` | SharedString | `""` | 注册表查找键（`registry.rs:154`、`:271`） |
| `mode` | `mode` | `ThemeMode` | `light` | 决定写 `dark_theme` 还是 `light_theme`（`schema.rs:1060-1065`） |
| `font_size` | `font.size` | f32? | 16 | `schema.rs:1081-1083` |
| `font_family` | `font.family` | str? | 系统字体 | `schema.rs:1084-1086` |
| `mono_font_family` | `mono_font.family` | str? | 平台相关 | `schema.rs:1087-1089` |
| `mono_font_size` | `mono_font.size` | f32? | 13 | `schema.rs:1090-1092` |
| `radius` | `radius` | usize? | 6 | `schema.rs:1093-1095` |
| `radius_lg` | `radius.lg` | usize? | 8 | `schema.rs:1096-1098` |
| `shadow` | `shadow` | bool? | true | `schema.rs:1099-1101` |
| `colors` | `colors` | `ThemeConfigColors` | 全 `None` | 见 §1.3 |
| `highlight` | `highlight` | `HighlightThemeStyle`? | — | Zed 风格语法高亮，`schema.rs:1066-1073`；本次**未写入**（§6） |

本次两个文件**只写了 `name` / `mode` / `colors`**：Lithe 的字号/字体/圆角不来自主题真源，而来自
`windows/tauri/src/styles/theme.css:106-134`（`--app-ui-font-size` / `--radius` 等），且 Lithe 的圆角阶梯与
gpui 的 `RadiusTokens` 不同构（`lithe-gpui/UI-MAP-WINDOWS.md:1590`），塞进 `radius` 只会得到错误的中间档。
这两项应由 Rust 侧显式赋值或自建 `LitheRadii`。

### 1.3 `colors` 的 key 命名空间：共 **139** 个 key

JSON key 就是 `ThemeConfigColors` 上 `#[serde(rename = "...")]` 的字面量（`schema.rs:252-675`），
**不是** Rust 字段名的下划线形式（例如字段 `button_primary` ↔ key `button.primary.background`）。
命名空间靠点分（`button.*` / `table.*` / `sidebar.*` / `base.*`）。

完整 139 个 key（`registry` 侧校验用；本目录的文件只用到其中 60 个）：

```text
background border foreground overlay ring caret window.border
accent.background accent.foreground input.border selection.background
muted.background muted.foreground
primary.background primary.foreground primary.hover.background primary.active.background
secondary.background secondary.foreground secondary.hover.background secondary.active.background
danger.background danger.foreground danger.hover.background danger.active.background
info.background info.foreground info.hover.background info.active.background
success.background success.foreground success.hover.background success.active.background
warning.background warning.foreground warning.hover.background warning.active.background
button.background button.foreground button.hover.background button.active.background
button.primary.background button.primary.foreground button.primary.hover.background button.primary.active.background
button.secondary.background button.secondary.foreground button.secondary.hover.background button.secondary.active.background
button.danger.background button.danger.foreground button.danger.hover.background button.danger.active.background
button.info.background button.info.foreground button.info.hover.background button.info.active.background
button.success.background button.success.foreground button.success.hover.background button.success.active.background
button.warning.background button.warning.foreground button.warning.hover.background button.warning.active.background
accordion.background group_box.background group_box.foreground group_box.title.foreground
description_list.label.background description_list.label.foreground
drag.border drop_target.background
chart.1 chart.2 chart.3 chart.4 chart.5 chart.bullish chart.bearish
link link.active link.hover
list.background list.active.background list.active.border list.even.background list.head.background list.hover.background
popover.background popover.foreground progress.bar.background
scrollbar.background scrollbar.thumb.background scrollbar.thumb.hover.background
sidebar.background sidebar.foreground sidebar.border sidebar.accent.background sidebar.accent.foreground
sidebar.primary.background sidebar.primary.foreground
skeleton.background slider.background slider.thumb.background switch.background switch.thumb.background
tab.background tab.foreground tab.active.background tab.active.foreground tab_bar.background tab_bar.segmented.background
table.background table.active.background table.active.border table.even.background table.head.background
table.head.foreground table.foot.background table.foot.foreground table.hover.background table.row.border
title_bar.background title_bar.border status_bar.background status_bar.border
base.red base.red.light base.green base.green.light base.blue base.blue.light
base.yellow base.yellow.light base.magenta base.magenta.light base.cyan base.cyan.light
```

**三个静默失效的坑**（key 写错不报错，未知 key 被 serde 忽略——`schema.rs` 通篇没有 `deny_unknown_fields`）：

1. `group_box.title.foreground`（`schema.rs:360-361`）是**死键**：`ThemeColor` 没有同名字段，`apply_config`
   （`schema.rs:677-1056`）从不读它。写了没有任何效果。
2. 上游 `src/theme/default-theme.json` 自身有 6 处**过期 key**，全部静默失效：
   `drag_border`（应为 `drag.border`，`schema.rs:405`）、`link.foreground` / `link.active.foreground` / `link.hover.foreground`
   （应为 `link` / `link.active` / `link.hover`，`schema.rs:429-436`）、`progress_bar.background`（应为 `progress.bar.background`，`schema.rs:480`）、
   `slider.bar.background`（应为 `slider.background`，`schema.rs:534-535`）。对照 `default-theme.json:31,37-39,54,72`。
   → 所以内置默认主题的 `link` / `drag_border` 等实际走的是 fallback（蓝），**不要照抄上游那一行的 key 拼法**。
3. 别把 Lithe 的键名当 gpui 的键名：`background` 是唯一同名的一个；`surface` / `subtle-foreground` / `selected` /
   `muted-foreground`（横线）在 gpui 里分别是 `popover`+`muted` / 无 / `list.active` 系 / `muted.foreground`（点号）。

**颜色值的合法格式**（`src/theme/color.rs:693-757`、`:763-791`）：

- `#RRGGBB` 或 `#RRGGBBAA`（`gpui::Rgba::try_from`）
- Tailwind 颜色名：`white` / `black` / `<name>` / `<name>-<scale>` / `<name>/<opacity>` / `<name>-<scale>/<opacity>`
  （`color.rs:375-431` 的 20 个色名：neutral/gray/red/orange/amber/yellow/lime/green/emerald/teal/cyan/sky/blue/indigo/violet/purple/fuchsia/pink/rose）
- **不支持 `rgba(...)` / `hsl(...)` 这类函数写法**（`color.rs:694-697` 只认 `#` 开头，否则按色名解析 → 解析失败）。
  Lithe 的 light `selection` 是 `rgba(53, 116, 240, 0.2)`，所以必须转成等值 8 位 hex `#3574f033`。
- 带背景语义的 token（`background`、`button.background`、`list.active.background`、`selection.background` 等走
  `apply_background_color!` 的那些）额外支持两段渐变：`linear-gradient(<角度|to right 等>, <stop>, <stop>)`
  （`color.rs:807-857`，**只支持 2 个 stop**）。此时顶层 `theme.xxx` 纯色字段取第一个 stop 的颜色
  （`schema.rs:14-19`、`color.rs:793-799`）。

### 1.4 明暗怎么表达

**一个 `ThemeConfig` 一个 `mode`**，同一个主题族的明暗是**两条** `ThemeConfig`（同一文件里两条，或分文件）。

- `mode` 只接受 `"light"` / `"dark"`（`ThemeMode` 是 `#[serde(rename_all = "snake_case")]`，`mod.rs:700-705`；`Light` 是默认值）。
- `apply_config` 按 `mode` 决定写 `theme.dark_theme` 还是 `theme.light_theme`（`schema.rs:1060-1065`），
  并决定缺失 key 的 fallback 基线用 `ThemeColor::dark()` 还是 `light()`（`schema.rs:1075-1079`）。
- **没有** `appearance` 字段，文件名与明暗无关，`_ref` / `$schema` / `panel.background` 之类的未知根/颜色字段一律被忽略。

### 1.5 「哪些 key 必须写」——无 fallback 的 13 个

`apply_config` 的两套宏（`schema.rs:688-738`）：

- 带 fallback 的 key 缺失时，用**本文件已解析出的真源值**推算（例：`input.border` ← `border`，`schema.rs:776`；
  `primary.hover` ← `background.blend(primary@0.9)`，`schema.rs:804-807`）。这类可以不写。
- **不给 fallback 的 key 缺失时会去读编译进二进制的 shadcn 默认主题值**（`schema.rs:685`、`:717-719`），
  会把 shadcn 的中性灰/蓝漏进来。必须写的最小集合（13 个）：

  `background`（`:740`）、`border`（`:774`）、`foreground`（`:775`）、`muted.background`（`:777`）、
  `primary.background`（`:802`）、`secondary.background`（`:819`）、`overlay`（`:1016`）、
  `base.red` / `base.green` / `base.blue` / `base.yellow` / `base.magenta` / `base.cyan`（`:743-772`）。

  ⚠️ `base.red` 这一组特别隐蔽：`danger` 的 fallback 是 `self.red`（`schema.rs:925`）、`success` ← `green`（`:836`）、
  `info` ← `cyan`（`:859`）、`warning` ← `yellow`（`:879`）、`ring` ← `blue`（`:973`）——`base.*` 不写，
  这四个语义色的 fallback 链会整条塌到 shadcn 色板。

### 1.6 一个最小可用主题 JSON

```json
{
  "name": "My Theme Set",
  "themes": [
    {
      "name": "My Dark",
      "mode": "dark",
      "colors": {
        "background": "#1e1f22",
        "foreground": "#dfe1e5",
        "border": "#43454a",
        "muted.background": "#2b2d30",
        "primary.background": "#3574f0",
        "secondary.background": "#2b2d30",
        "overlay": "#00000033",
        "base.red": "#db5c5c",
        "base.green": "#57965c",
        "base.blue": "#3574f0",
        "base.yellow": "#d6ae58",
        "base.magenta": "#c8a2f4",
        "base.cyan": "#61c0bf"
      }
    }
  ]
}
```

（这 13 个就是 §1.5 的无 fallback 集合；其余 key 缺失时会从这 13 个值推导出来。`mode`/`name` 必填，`is_default` 可选。）

### 1.7 加载期会被强制改写的值（写进去也会变）

| 被改写的 key | 强制规则 | 证据 |
| --- | --- | --- |
| `list.active.background`、`table.active.background` | alpha 被硬压到 ≤ **0.2** | `schema.rs:1035-1046` + `:1022-1033` |
| `selection.background` | alpha 被硬压到 ≤ **0.3** | `schema.rs:1047-1052` |

即：这两个 token **无法**表达不透明的实心选中底（Lithe 的 `--selected` / dark `--selection` 都是不透明的）。
`clamp_alpha` 只压 alpha、不改 RGB，所以写 `#2e436e` 与写 `#2e436e33` 渲染结果相同（都变成 33% 透明度）。

---

## 2. 格式参照

- **上游主题文件（网络可取，已取）**：`Ayu`，`https://cdn.jsdelivr.net/gh/longbridge/gpui-kit@main/themes/ayu.json`
  （`web_fetch` 直连 `raw.githubusercontent.com` 被沙盒拦截，改走 jsDelivr 镜像）。本项目的 JSON 结构照它写，它证实了：
  ① 根是 `ThemeSet` 且带 `$schema`（未知根字段无害）；② `themes[]` 里明暗各一条 `ThemeConfig`；
  ③ 上游一个主题**只写需要覆盖的 ~30 个 key**，其余交给 fallback（不需要写满 139 个）；
  ④ 透明度直接写进 8 位 hex（`"list.active.background": "#55B4D422"`）；⑤ `highlight` 段是 Zed theme 兼容格式。
  注：Ayu 里的 `"panel.background"` 并不在 0.6.6 的 139 个 key 里，同样属于被忽略的未知 key。
- **本地格式依据（离线兜底）**：`gpui-component-0.6.6/src/theme/default-theme.json`（`registry.rs:12` 用 `include_str!` 编译进二进制）
  + `src/theme/schema.rs`。

---

## 3. 对照表：Lithe 键 → 值 → gpui-kit token key

真源①：`windows/tauri/src/extensions/themes/builtin/lithe.json`（L = `lithe-light` `:11-71`，D = `lithe-dark` `:78-138`）。
真源②：`windows/tauri/src/styles/theme.css`（别名/派生层）。

| Lithe 键（行号 L/D） | L 值 | D 值 | gpui-kit token key | 依据 |
| --- | --- | --- | --- | --- |
| `background`(13/79) | `#ffffff` | `#1e1f22` | `background` | 直搬（gpui「Default background color」，`schema.rs:263-265`） |
| `foreground`(15/81) | `#1f2328` | `#dfe1e5` | `foreground` | 直搬（`schema.rs:410-412`） |
| `border`(18/84) | `#dfe1e5` | `#43454a` | `border`、`input.border` | 直搬；`input.border` 依据 `theme.css:145` `--input: var(--border)`（`schema.rs:425-427`） |
| `border`+`foreground` | `#a9acb0` | `#6f7175` | `ring` | 推导：`theme.css:146` `--ring: var(--border-strong)`，`theme.css:140` `--border-strong: color-mix(in srgb, var(--border) 72%, var(--foreground) 28%)` → 本期手算成 8 位 hex（§4） |
| `accent`(19/85) | `#edf3ff` | `#393b40` | `accent.background`、`button.background`、`list.hover.background`、`secondary.hover.background` | 直搬（`schema.rs:254-256`）；`button.background` 依据 `windows/tauri/src/ui/button.tsx:13-19`（`variant=default` = `bg-accent hover:bg-selected`） |
| `foreground` | 同上 | 同上 | `accent.foreground`、`button.foreground`、`popover.foreground`、`secondary.foreground`、`tab.active.foreground`、`sidebar.foreground`、`sidebar.accent.foreground` | `theme.css:143` `--accent-foreground: var(--foreground)`、`:142` `--popover-foreground`、`:150` `--secondary-foreground`；`ui/tab-bar.tsx:170` 激活页签文字；`ui/sidebar.tsx:240-268` 侧栏激活项 `text-foreground` |
| `muted-foreground`(16/82) | `#4f5965` | `#b4b8bf` | `muted.foreground` | 直搬（`schema.rs:458-460`） |
| `surface`(14/80) | `#f7f8fa` | `#2b2d30` | `muted.background`、`popover.background`、`secondary.background` | `theme.css:144` `--muted: var(--surface)`、`:141` `--popover: var(--surface)`；`secondary` 为推导（§4） |
| `selected`(20/86) | `#d4e2ff` | `#2e436e` | `secondary.active.background`、`button.hover.background`、`button.active.background`、`list.active.background`、`sidebar.accent.background` | Lithe「整行选中底」语义：`ui/sidebar.tsx:240-268`（`active → bg-selected`）、`ui/button.tsx:13-19`（`hover:bg-selected`）；gpui 侧 `sidebar_accent` 是侧栏激活底（`gpui-component-0.6.6/src/sidebar/menu.rs:291-304`）。⚠️ alpha 被压到 0.2（§1.7） |
| `selection`(21/87) | `#3574f033` | `#214283` | `selection.background` | 直搬（`schema.rs:506-508`）；L 的 `rgba(53, 116, 240, 0.2)` gpui 解析不了，已转成等值 8 位 hex。⚠️ D 的 alpha 被压到 0.3（§1.7） |
| `primary`(22/88) | `#3574f0` | `#3574f0` | `primary.background`、`sidebar.primary.background`、`base.blue` | 直搬（`schema.rs:467-469`） |
| `background` | `#ffffff` | `#1e1f22` | `primary.foreground`、`sidebar.primary.foreground`、`danger.foreground`、`success.foreground`、`warning.foreground`、`info.foreground` | 推导：`theme.css:149` `--primary-foreground: var(--background)`（**不是白色**）；四条语义色前景沿用同一「填充底文字色」规则（§4） |
| `cursor`(23/89) | `#1f2328` | `#ced0d6` | `caret` | 直搬（gpui「Input caret color」，`schema.rs:362-364`）；gpui 只有这一个光标位 |
| `destructive`(26/92) | `#cf3f4f` | `#db5c5c` | `danger.background`、`base.red` | 直搬（gpui「Danger background color」，`schema.rs:386-388`） |
| `success`(27/93) | `#27864f` | `#57965c` | `success.background`、`base.green` | 直搬（`schema.rs:539-541`）。⚠️ `theme.css:151` 把 `--success` 别名成 `var(--primary)`，但运行时 `lithe.json` 的内联变量优先，故取 `lithe.json` 的值（`lithe-gpui/UI-MAP-WINDOWS.md:1586`） |
| `warning`(28/94) | `#a86400` | `#d6ae58` | `warning.background`、`base.yellow` | 直搬（`schema.rs:617-619`）；同上，`theme.css:152` 的别名在运行时被覆盖 |
| `info`(29/95) | `#3574f0` | `#548af7` | `info.background` | 直搬（`schema.rs:413-415`） |
| `subtle-foreground`(17/83) | `#68717d` | `#8b929e` | `tab.foreground`、`table.head.foreground` | 语义最近的落点：`ui/tab-bar.tsx:170`（未激活页签 `text-subtle-foreground`）、`ui/table.tsx:65`（表头 `text-subtle-foreground`）。gpui **没有**第三级文字色 token，其余位置只能给一档（§5） |
| `background` | 同上 | 同上 | `tab.background`、`tab.active.background`、`tab_bar.background`、`list.background`、`title_bar.background`、`sidebar.background` | `theme.css:155-156`（`--tab-bar-bg` / `--tab-active-bg` = `var(--background)`）；列表/标题栏/侧栏容器在 Lithe 是 `bg-background`（`ui/sidebar.tsx` 的 `SidebarPanel`） |
| `primary`/`destructive`/`success`/`warning` | — | — | `base.{blue,red,green,yellow}` | 推导：Lithe 无 base 调色板，取语义最接近的真源键（§4） |
| `terminal-magenta`(41/107) | `#8a4fb0` | `#c8a2f4` | `base.magenta` | 推导：Lithe 的 magenta 真源（§4） |
| `terminal-cyan`(42/108) | `#147d83` | `#61c0bf` | `base.cyan` | 推导：Lithe 的 cyan 真源（§4） |
| `terminal-bright-red`(45/111) | `#e05260` | `#ff858d` | `base.red.light` | 推导：`terminal-bright-*` 是同色相亮档（§4） |
| `terminal-bright-green`(46/112) | `#369d62` | `#68d5a0` | `base.green.light` | 同上 |
| `terminal-bright-blue`(48/114) | `#1684cb` | `#75b9f0` | `base.blue.light` | 同上 |
| `terminal-bright-yellow`(47/113) | `#bf7a16` | `#edbb5c` | `base.yellow.light` | 同上 |
| `terminal-bright-magenta`(49/115) | `#a267c4` | `#dab9ff` | `base.magenta.light` | 同上 |
| `terminal-bright-cyan`(50/116) | `#238f95` | `#7bd3d2` | `base.cyan.light` | 同上 |
| —（非 lithe.json） | `#00000033` | `#00000033` | `overlay` | 推导：`ui/dialog.tsx:97` 的遮罩是 `bg-black/20`（明暗同一个类名） |

**未写入、交给 gpui 自身 fallback 的 token**（这些 fallback 的根值全部来自本文件已写的真源值，不会漏进 shadcn 色）：

`input.border`←`border`；`primary.hover/active`←`background.blend(primary@…)`/`primary.darken(…)`（`schema.rs:804-811`）；
`danger|success|warning|info` 的 `hover/active`←`background.blend(x@0.9)`/`x.darken(…)`（`schema.rs:838-845,861-865,881-888,928-931`）；
`button.*` 的五组语义变体←对应的 `primary/secondary/danger/info/success/warning`（`schema.rs:812-818,829-835,846-858,866-878,889-901,932-944`）；
`accordion`←`background`；`group_box.background`←`background.blend(secondary@0.4/0.3)`（**这条恰好复刻了 Lithe 卡片 `bg-surface/45`、`ui/card.tsx:5-26`**）；
`list.even`/`list.head`/`table.*`←`list.*`；`scrollbar.*`←`background`/`accent`；`skeleton`/`switch`←`secondary` 系；
`tab_bar.segmented`←`secondary`（=Lithe `bg-surface/55`，`ui/tabs.tsx:20-36`）；`slider.*`/`progress.bar`←`primary`；
`chart.1..5`←`blue` 明度阶梯（Lithe 无图表色真源，见 §5）；`link*`←`primary`；`window.border`/`title_bar.border`/`sidebar.border`←`border`。

---

## 4. 我推导出来的 token（不是真源直搬）与推导依据

| gpui token key | L 值 | D 值 | 推导规则 |
| --- | --- | --- | --- |
| `ring` | `#a9acb0` | `#6f7175` | `theme.css:140` 的 `color-mix(in srgb, var(--border) 72%, var(--foreground) 28%)` 逐通道求值（srgb 空间线性插值，四舍五入到 8 位）：L `223×0.72+31×0.28=169.24→169(A9)`，`225×0.72+35×0.28=171.8→172(AC)`，`229×0.72+40×0.28=176.08→176(B0)`；D `67×0.72+223×0.28=110.68→111(6F)`，`69×0.72+225×0.28=112.68→113(71)`，`74×0.72+229×0.28=117.4→117(75)`。再按 `theme.css:146` `--ring: var(--border-strong)` 落到 `ring` |
| `primary.foreground` / `sidebar.primary.foreground` | `#ffffff` | `#1e1f22` | `theme.css:149` `--primary-foreground: var(--background)` → `lithe.json:background`（L `#ffffff` / D `#1e1f22`）。**不是白色**：dark 下 `primary` 上的文字是 `#1e1f22` |
| `danger.foreground` / `success.foreground` / `warning.foreground` / `info.foreground` | `#ffffff` | `#1e1f22` | Lithe 没有 `--destructive-foreground` 等键，只有 `--primary-foreground` 这一条「填充底上的文字色」规则（`theme.css:149`）；四条语义色是同一类「实心填充底」，沿用同一规则。gpui 的内置 fallback 也是 `primary_foreground`（`schema.rs:837,860,880,927`），值一致 |
| `secondary.background` | `#f7f8fa` | `#2b2d30` | **合并**：Lithe 只有 `background`/`surface` 两档中性面，gpui 需要 `background`/`muted`/`secondary` 三档。`theme.css:144` 只给了 `--muted: var(--surface)`，`secondary` 在 Lithe 无对应 → 与 `muted` 合并到 `--surface` |
| `secondary.hover.background` | `#edf3ff` | `#393b40` | Lithe 的「悬停底」规则 = `--accent`（`ui/button.tsx:13-19` `hover:bg-selected`/`bg-accent`、`ui/dropdown.tsx` `focused → bg-accent`） |
| `secondary.active.background` | `#d4e2ff` | `#2e436e` | Lithe 的「选中底」规则 = `--selected` |
| `button.active.background` | `#d4e2ff` | `#2e436e` | Lithe 未定义按下底色（只有 `active:scale-(--app-press-scale)`，`ui/button.tsx:9`）→ 取与 hover 同值。若想更暗，删掉这个 key 让 gpui 用 `input.mix_oklab(transparent, 0.7)`（`schema.rs:798-801`） |
| `base.red`/`base.green`/`base.blue`/`base.yellow` | `#cf3f4f`/`#27864f`/`#3574f0`/`#a86400` | `#db5c5c`/`#57965c`/`#3574f0`/`#d6ae58` | 这 4 个 key **无 fallback**（`schema.rs:743-772`），缺失会漏进 shadcn 色板。Lithe 无 base 调色板 → 取语义最接近的真源：`red←destructive`、`green←success`、`blue←primary`、`yellow←warning` |
| `base.magenta` / `base.cyan` | `#8a4fb0`/`#147d83` | `#c8a2f4`/`#61c0bf` | 同上（无 fallback）。Lithe 的 magenta/cyan 真源在终端色里：`terminal-magenta`、`terminal-cyan`（其余候选 `git-renamed` 与 magenta 同族，取终端色更纯粹） |
| `base.{red,green,blue,yellow,magenta,cyan}.light` | `#e05260`/`#369d62`/`#1684cb`/`#bf7a16`/`#a267c4`/`#238f95` | `#ff858d`/`#68d5a0`/`#75b9f0`/`#edbb5c`/`#dab9ff`/`#7bd3d2` | Lithe 没有 `base.*.light`；`lithe.json` 的 `terminal-bright-*` 就是同一色相的亮档 → 直接取用（比 gpui 自己的 `background.blend(base@0.8)` 更贴 Lithe） |
| `overlay` | `#00000033` | `#00000033` | Lithe 的遮罩是 `bg-black/20`（`ui/dialog.tsx:97`；`ui/alert-dialog.tsx:23` 同值，sheet 为 `bg-black/10`）→ 黑色 20% = `#00000033`。明暗同一类名，故两套同值 |
| `tab.background` / `tab.active.background` / `tab_bar.background` | `#ffffff` | `#1e1f22` | `theme.css:155-156` `--tab-bar-bg` / `--tab-active-bg` = `var(--background)`（IntelliJ 风格：页签条与编辑区同底，靠 1px 下边线分隔） |
| `tab.foreground` / `table.head.foreground` | `#68717d` | `#8b929e` | 取 `subtle-foreground`：`ui/tab-bar.tsx:170`、`ui/table.tsx:65` 明确用 `text-subtle-foreground`。gpui 的 fallback 只有 `muted.foreground` 一档，这里用真源把这两处修回来 |
| `list.background` / `title_bar.background` / `sidebar.background` | `#ffffff` | `#1e1f22` | Lithe 的列表/侧栏/标题栏容器都是 `bg-background`（`ui/sidebar.tsx` 的 `SidebarPanel`）。与 gpui fallback 同值，写出来是为了显式记录语义 |
| `selection.background` (L) | `#3574f033` | — | `lithe.json:21` 是 `rgba(53, 116, 240, 0.2)`，gpui 的颜色解析器只认 hex / Tailwind 色名（`color.rs:694-697`），故转成等值 8 位 hex（0.2×255=51=0x33）。RGB 与 alpha 完全不变 |

---

## 5. 降级 / 合并：Windows 语义色在 gpui-kit 里没有对应 token

按「Windows 键 → 处理 → 原因（证据）」列出。这一节是给 Rust 侧看的缺口清单，主题文件**无法**表达这些。

| # | Windows 键 / 值 | 处理 | 原因与证据 |
| --- | --- | --- | --- |
| 1 | `--subtle-foreground`（L `#68717d` / D `#8b929e`） | **降级**：只在 `tab.foreground`、`table.head.foreground` 两处有据可循的落点用上；其余只能并进 `muted.foreground` | gpui 的 `ThemeConfigColors` 只有 `muted.foreground` 一档文字灰（`schema.rs:458-460`），没有第三级。→ 需要 Rust 侧自建 `LithePalette.subtle_foreground`（`lithe-gpui/UI-MAP-WINDOWS.md:1510`、`:1604`） |
| 2 | `--selected`（L `#d4e2ff` / D `#2e436e`，**不透明**） | **合并**到 `list.active` / `table.active` / `button.hover` / `button.active` / `secondary.active` / `sidebar.accent`；**且 alpha 被强压到 20%** | gpui 无同名 token；`clamp_alpha(..., 0.2)`（`schema.rs:1035-1046`）会把不透明值变成 20% 叠加层 → **Windows 的实心选中底无法复刻**。要实心需自建 token 或在组件里自绘 |
| 3 | `--selection` D `#214283`（**不透明**） | 值直搬，但 alpha 被强压到 30% | `schema.rs:1047-1052`。Lithe 不透明的 dark 选区蓝在此变成 30% 叠加 |
| 4 | `--toolHeader` | **仓库中未找到该变量**（对 `windows/` 全目录 grep `toolHeader` / `tool-header` / `toolbar-foreground` 零命中） | 无需映射；若确有此需求请给出定义点 |
| 5 | `.lithe-*` 尺寸令牌 16 个（`--lithe-title-bar-height` 40 / `--lithe-footer-height` 24 / `--lithe-pane-header-height` 36 / `--lithe-tab-bar-height` / `--lithe-tab-height` 28 / `--lithe-tab-max-width` 200 / `--lithe-sidebar-header-height` 32 / `--lithe-workbench-gap` 4 / `--lithe-chrome-control-height` 24 / `--lithe-chrome-hit-target` 28 / `--lithe-chrome-line-height` 16 / `--lithe-chrome-gap-tight` 2 / `--lithe-chrome-gap` 4 / `--lithe-chrome-gap-loose` 6 / `--lithe-chrome-padding-inline` 8 / `--lithe-chrome-radius` 4） | **无法表达** | `ThemeConfig` 只有 `radius` / `radius.lg` / `font.*` / `shadow`（`schema.rs:46-74`），没有任何尺寸 token → 必须 Rust 侧覆盖（另见 `lithe-gpui/UI-MAP-WINDOWS.md:1619-1622`，含 gpui 写死的 `TITLE_BAR_HEIGHT 34` vs Lithe 40） |
| 6 | `--lithe-glass-*` / `--lithe-chrome-icon-*` / `--lithe-chrome-control-*` 共 13 键（玻璃/半透明） | **降级**：`title_bar.background`（以及未写的 `status_bar.background`，fallback 到 `title_bar`）用实心 `background` | gpui 无「原生窗后透」API，`backdrop-filter` 那套在 Rust 侧不存在（`lithe-gpui/UI-MAP-WINDOWS.md:1552`） |
| 7 | `--tab-hover-bg`（`color-mix(in srgb, var(--accent) 72%, transparent)`） | **丢失** | `ThemeConfigColors` 的 tab 家族只有 `tab` / `tab.active` / `tab_bar` / `tab_bar.segmented`（`schema.rs:557-574`），**没有 `tab.hover`** → 页签 hover 底色只能组件侧自绘或用 `accent` |
| 8 | `--cursor-vim-normal`（α0.62/0.68）、`--cursor-vim-insert` | **丢失**（只保留 `--cursor` → `caret`） | gpui 只有 `caret` 一个光标位（`schema.rs:362-364`） |
| 9 | `--git-modified` / `-modified-staged` / `-added` / `-deleted` / `-untracked` / `-renamed`（6 键） | **丢失** | 139 个颜色 key 里没有任何 git 语义 token；`highlight` 段的 `conflict`/`created`/`modified`/`deleted`/`renamed` 是 `StatusColors`（Zed 的编辑状态色），语义与 Lithe 的 git 状态色不完全等价 |
| 10 | `--terminal-*`（16 键） | **丢失 10 键**；其中 `terminal-magenta` / `terminal-cyan` / `terminal-bright-{red,green,blue,yellow,magenta,cyan}` 被借用为 `base.*` / `base.*.light`（§4） | 139 个颜色 key 里没有终端色；gpui 侧终端的 ANSI 16 色需自建 `[Hsla; 16]`（`lithe-gpui/UI-MAP-WINDOWS.md:1524`、`:1608`） |
| 11 | 18 个 `--syntax-*` | **本次未写入**（走 `highlight` 段，形状完全不同） | 见 §6 |
| 12 | `--symbol-*`（7 键）、9 个 `--syntax-markdown-*` | **丢失** | 两个命名空间在 gpui 里零对应（`lithe-gpui/UI-MAP-WINDOWS.md:1546-1547`、`:1578`） |
| 13 | `--border-strong`（`color-mix(border 72%, foreground 28%)`） | **只映射到 `ring`** | gpui 没有独立 token；近似的 `drag.border` 语义是拖拽指示边框（`schema.rs:404-406`），本文件未使用 → 组件里需要「强边框」时只能用 `ring` 或自建 |
| 14 | `--card` / `--card-foreground`（=`surface` / `foreground`） | **合并**到 `popover`（同值） | gpui 无 `card` token；`theme.css:141-142,147-148` 两者取值完全相同，所以合并**不丢信息**；容器底色则由 `group_box.background` 的 fallback 复刻 Lithe 的 `bg-surface/45` |
| 15 | `--muted`（=`surface`）、`--input`（=`border`）、`--ring`（=`border-strong`）、`--primary-foreground`（=`background`）、`--secondary-foreground`（=`foreground`）、`--accent-foreground`（=`foreground`）、`--popover*`（=`surface`/`foreground`） | 已按各自语义落到不同 gpui token（§3） | 这些都是 `theme.css:140-150` 的 shadcn 别名层，gpui 侧有真实字段，不是降级 |
| 16 | 图表色（Lithe 无真源） | 未写，交给 gpui 的 `chart.1..5` ← `base.blue` 明度阶梯（`schema.rs:918-922`） | Lithe 的 `lithe.json` 里没有图表色键，**不编颜色** |
| 17 | `--success` / `--warning` 在 `theme.css:151-152` 被别名成 `primary` / `muted-foreground` | 采用 `lithe.json` 的值 | 运行时 `setProperty` 写的是 `documentElement` 内联样式，优先级高于 `:root` 规则 → `lithe.json` 胜出（`lithe-gpui/UI-MAP-WINDOWS.md:1586`）。如果哪天主题加载失败，组件才会落到 `primary`/`muted-foreground` |

---

## 6. 未写入的内容与未确认项

### 6.1 `highlight`（语法高亮）：**第一批**的两个 Lithe 文件没写

（下面是第一批不写的三条理由，原文保留——它们对 `lithe-*.json` 仍然成立）

1. 它是独立的 `HighlightThemeStyle`（`src/highlighter/registry.rs:436`）：字段是 `Option<Hsla>` +
   `#[serde(flatten)] StatusColors` + 一个**非 Option** 的 `syntax`，与 `colors` 的形状完全不同；
   `ThemeSet` 里写错形状会让**整份文件**解析失败并被静默丢弃（`registry.rs:252-258`）。
2. Lithe 侧还有一条主题文件表达不了的规则：「syntax 值缺失**或**与 `foreground` 的欧氏 RGB 距离 < 28 就回落到
   按明暗硬编码的 18 色」（`windows/tauri/src/extensions/themes/syntax-token-colors.ts:3-44,99-128`）。
   硬搬 18 色会得到与 Windows 不一致的高亮结果，必须由 Rust 侧实现这条回落。
3. 第一批任务不允许运行 `cargo`（`lithe-gpui/target` 由主代理独占），**没有**用 `serde_json::from_str::<ThemeSet>` 实测过
   `highlight` 的形状 → 宁可不写，避免整份主题静默失效。

**第二批起（§7）**：`gruvbox.json` / `jetbrains.json` / `nord.json` / `one.json` / `vscode.json`
**都写 `highlight` 段**，映射规则与"整段替换"的后果见 §7。`lithe-*.json` 维持不写——第 2 条那条回落规则
没有变，Lithe 的 18 个 `--syntax-*` 值直搬进主题文件反而不等于 Windows 的显示结果。
于是当前 12 条主题里，只有 Lithe 两族的编辑器语法色来自 gpui 内置色板。

格式参照上游 `themes/ayu.json` 的 `highlight` 段（`{"editor.background": "#...", "syntax": {"comment": {"color": "#..."}}}`，
Zed theme 兼容，`registry.rs:459-463` 的文档链接即 Zed 官方说明）。

**形状已实测**（第 3 条的历史限制已解除）：`crates/settings/src/theme.rs` 的
`every_theme_file_is_a_loadable_theme_set` 用真实 `ThemeSet` 反序列化整个目录，形状写错就是测试失败，
不会再出现"整份主题静默消失"。

### 6.2 `is_default` 未设置（默认 `false`）

若要让 Lithe 成为注册表的默认主题（`sorted_themes()` 置顶、`ThemeRegistry::default_light_theme()`/`default_dark_theme()`
指向 Lithe），在对应条目加 `"is_default": true`（`schema.rs:39` → `registry.rs:275-278`）。

⚠️ 副作用：置 `true` 后，`reload()` 会先清空 `themes` 再只填 `default_themes.values()`（`registry.rs:264-268`），
于是**内置的 shadcn `Default Light` / `Default Dark` 会在第一次文件变更后从 `themes()` 里消失**（第二次 reload 不再回填）。
如果设置界面需要保留这两个内置主题，就不要置 `true`，改为在 Rust 侧直接 `apply_config` / `Theme::change`。

### 6.3 其它未确认

- **第一批**的合法性校验方式是 `ConvertFrom-Json` + node `JSON.parse`，并把 `colors` 的 key 与
  `schema.rs:252-675` 里正则抽出的 **139 个** `#[serde(rename = ...)]` 逐一做了子集检查（结果：未知 key 0 个、非 hex 值 0 个、
  重复 key 0 个、无 BOM、LF）；当时**没有**跑 cargo，所以"真实 Rust 反序列化"是缺的。
  **第二批起这条缺口已补**：`crates/settings/src/theme.rs` 的 `every_theme_file_is_a_loadable_theme_set`
  对**整个目录**做真实 `ThemeSet` 反序列化 + 13 个无 fallback 的 key + 每个颜色值过 gpui 解析器 + 主题名唯一，
  跑 `cargo test -p lithe-gpui-settings theme::` 即可（12 条主题全绿）。
- 未确认：Lithe 的 `--tab-hover-bg`、`--lithe-glass-*` 是否值得自建 token；`switch` 未选中态的底色（gpui 用
  `secondary.active`，Lithe 的 `ui/switch.tsx` 未在本次核查范围内确认其 unchecked 底色）→ 未写入 `switch.background`。
- 上游主题目录共 20+ 个，第一批只取了 `ayu.json` 作为格式参照；`raw.githubusercontent.com` 在本沙盒不可直连（解析到非公网 IP），
  走 `cdn.jsdelivr.net` 镜像成功。docs 页 `https://gpui-kit.com/zh-CN/component/theme.md` 可正常抓取。

---

## 7. 第二批主题：Gruvbox / JetBrains / Nord / One / VS Code

### 7.1 来源：设计真源两件套

每个族由**同一批设计值**导出两份文件，本次进仓库的是第二份：

| 文件 | 格式 | 用途 |
| --- | --- | --- |
| `theme-<族>.json` | Lithe 设计语言 schema（`themes[]` 带 `id` / `appearance` / `colors` / **`syntax`**），与 `windows/tauri/src/extensions/themes/builtin/` 里那些 JSON 同构 | 设计真源；Windows 侧读的格式 |
| `gpui-theme-<族>.json` → 本目录 `<族>.json` | gpui-kit `ThemeSet`（`name` / `author` / `themes[]` 带 `mode` / `font.size` / `radius` / `colors` / **`highlight`**） | gpui 侧加载的格式 |

进仓库时**只做了一件事**：把设计真源的 `syntax` 块按下面的表映射成 gpui 的 `highlight` 段写进 `themes[]`。

| 事实 | 证据 |
| --- | --- |
| `colors`（60 个 key）、`themes[].name`、`mode`、`font.size` / `radius` / `radius.lg` **一个字节都没改**，与导出的 `gpui-theme-<族>.json` 逐 key 相等 | 逐族比对：5 个族的 `colors` 键序与取值、`name`、`mode`、`radius*`、`font.size` 差异为 0；**只有 `highlight` 段是本次新增的** |
| 这 5 个族的 `colors` 映射**由设计真源自己给出**，与 §3/§4 的 Lithe 推导**不是同一套**——键名与语义对齐，推导过程不同。已核对的两个例子：① `ring` 十色**全部**等于设计真源的 `subtle-foreground`（Gruvbox Light `#7c6f64`、VS Code Light+ `#8c8c8c`…），而 Lithe 的 `ring` 是 `color-mix(border 72%, foreground 28%)` 算出来的（§4）；② `primary.foreground` 多数族取 `#ffffff`（Darcula、One Light、VS Code Dark+…），而 Lithe 的规则是 `= --background` | 逐值比对两套导出件；§3/§4 的推导只对 `lithe-*.json` 成立，**不要**拿它当这批文件的验收标准 |
| §5 里"gpui 没有对应 token"的那批设计键对这批主题同样成立，但**落点可能不同**：`subtle-foreground` 在这里还被用作 `ring`（Lithe 不用它做 `ring`），`cursor-vim-*`、6 个 `git-*`、16 个 `terminal-*` 里未被借用的 10 个仍然没有落点 | §5；逐键比对导出件 |
| `is_default` 仍**没写**（默认 `false`），默认主题仍是 `lithe-dark`（显示名 `Lithe Dark`） | `crates/settings/src/schema.rs` 的 `DEFAULT_THEME` |

### 7.2 `syntax` → `highlight.syntax` 的映射（16 个 key）

设计真源的 `syntax` 是 18 个 token，gpui 的 `SyntaxColors`（`highlighter/registry.rs:113-172`）是 41 个名字。
下表左列在设计真源里都存在，右列就是写进主题文件的那 16 个 key：

| 设计 `syntax.*` | gpui `highlight.syntax.*` | 说明 |
| --- | --- | --- |
| `attribute` | `attribute` | 直搬 |
| `boolean` | `boolean` | 直搬 |
| `comment` | `comment`、`comment_doc` | 一条设计值覆盖两个 gpui 名字 |
| `constant` | `constant` | 直搬（Java 的 `@constant.builtin` 会回落到它，见下） |
| `function` | `function` | 直搬（`@function.builtin` / `@function.method` 回落到它） |
| `keyword` | `keyword` | 直搬 |
| `number` | `number` | 直搬 |
| `operator` | `operator` | 直搬 |
| `property` | `property` | 直搬 |
| `punctuation` | `punctuation` | `punctuation.bracket` / `.delimiter` / `.special` 回落到它 |
| `regex` | `string.regex` | 直搬（没写 `string.special`，它回落到 `string`） |
| `string` | `string` | 直搬（`string.escape` / `string.special` 回落到它） |
| `tag` | `tag` | 直搬（`tag.doctype` 回落到它） |
| `type` | `type` | 直搬（`@type.builtin` 回落到它） |
| `variable` | `variable` | 直搬（`@variable.builtin` 回落到它） |

**"回落"不是 gpui 的猜测，是 `SyntaxColors::style` 的显式规则**：命中不到名字、且名字里含 `.` 时，
用第一个点号前的前缀再查一次（`highlighter/registry.rs:241-307`）。所以上表里那些带点号的 gpui 名字
不写也能拿到同族颜色，写出来只是重复。

三个**没有落点**的设计 token（写进去也会被忽略，所以没写）：

| 设计 token | 为什么没落点 |
| --- | --- |
| `null` | 41 个 gpui 名字里没有 `null`。Java 的 `null_literal` 被 grammar 标成 `@constant.builtin`（`tree-sitter-java-0.23.5/queries/highlights.scm:82-86`），只能回落到 `constant`——**无法**单独给 `null` 上色 |
| `jsx` | gpui 只有一套 `tag` / `attribute` 名字，没有 JSX 专用名；tsx 的 JSX 元素名也会走 `tag` |
| `jsx-attribute` | 同上，merge 进 `attribute` |

⚠️ **`comment_doc` 是下划线，不是 `comment.doc`**：结构体字段是 `pub comment_doc`（`registry.rs:117`，**没有** `rename`），
所以 JSON key 是 `comment_doc`。上游自己的 `default-theme.json:136`、`:347` 写的是 `"comment.doc"` ——
那是**死键**（和 §1.3 第 2 条列的上游过期 key 同一类问题），照抄会静默失效。

### 7.3 `highlight` 段里额外写的三个 `editor.*` 值

`syntax` 之外只写了三个键，`editor.*` 的其余键（`editor.invisible`、`editor.line_number`、
`editor.active_line_number`、`editor.gutter.background`）**故意不写**：

| 写入的 key | 取值来源 | 依据 |
| --- | --- | --- |
| `editor.background` | 设计 `background` | 编辑器画布的落点是 `Theme::editor_background()`：`highlight` 里没写就回落到 `input_background()`（`theme/mod.rs:389-393`），而那个函数在**深色**下返回 `input.mix_oklab(transparent, 0.3)`（`:379-385`，`input` = `input.border` 键，`schema.rs:776`）——即"背景 + 30% 边框色"，一个不在设计调色板里的混色。写成主题自己的 `background` 才是设计值（VS Code Dark+ 的编辑器底就是 `#1e1e1e` = 它的 `background`），代价是**这批主题的深色编辑器底比 Lithe 两族更"平"**（Lithe 没有 `highlight`，走的就是那个混色） |
| `editor.foreground` | 设计 `foreground` | 与 `InputEditorStyle.foreground`（`cx.theme().foreground`，`gpui-component-0.6.6/src/input/input.rs:498`）同值，不另造颜色 |
| `editor.active_line.background` | 设计 **`accent`**（推导，非直搬） | 设计语言没有"当前行高亮"这个 token。**不写的话当前行高亮会消失**（见 §7.4），所以取语义最近的"悬停/激活底" `accent`；VS Code Dark+ 的真实 `editor.lineHighlightBackground` 恰好就是 `#2a2d2e` = 该主题的 `accent` |

### 7.4 为什么 `highlight` 是**整段替换**，缺的键不会沿用内置值

`apply_config` 在 `highlight` 存在时**直接换掉整个 `HighlightTheme`**（`schema.rs:1066-1073`），不是按字段合并。
于是没写的字段变成 `None`，各自走自己的回落：

| 没写的字段 | 结果 | 回落点 |
| --- | --- | --- |
| `editor.invisible` | 空白符指示色 = `muted.foreground` | `gpui-base-0.6.6/src/input/base/element.rs:1077-1080` |
| `editor.line_number` / `editor.active_line_number` | **无影响**：这两个字段在 `gpui-base` / `gpui-component` 里没有任何消费方，行号色走 `muted.foreground`（`element.rs:2118`） | 全仓 grep 只命中结构体定义 |
| `editor.active_line.background` | 若也不写 → **当前行不再有底色**（内置主题里有）→ 所以 §7.3 补上了它 | `element.rs:2237-2239`、`:2261` |
| status 色（`error` / `warning` / `info` / `success` / `hint` 及其 `.background` / `.border`） | 落到本主题的 `red` / `yellow` / `blue` / `green` / `cyan`——比内置色板更贴主题，所以**不写** | `highlighter/registry.rs:348-434` 的 `unwrap_or(cx.theme().…)` |

（第一批的 `lithe-*.json` 没有 `highlight` 段：`apply_config` 不动 `highlight_theme`，所以 Lithe 两族用的是
**内置同明暗的那一份**——`crates/settings/src/theme.rs` 的 `stamp_builtin_highlight_if_absent` 在 `apply_config`
之前按明暗把它复位，否则"上一个带 `highlight` 的主题"的语法色会留在 Lithe 上（浅色语法画在深底）。
`editor.active_line` / `editor.invisible` 也来自这一份。这不是 Lithe 的设计值，是 §6.1 第 2 条那个待实现的回落。）

### 7.5 怎么校验（改主题文件后必跑）

```powershell
cargo test -p lithe-gpui-settings theme::
```

`every_theme_file_is_a_loadable_theme_set`（`crates/settings/src/theme.rs`）对**整个目录**断言四件事，
每一件都是"不报错但主题消失/漏色"的静默失效：

1. 每份 `*.json` 都能被真实 `ThemeSet` 反序列化（`highlight` 少写非 Option 的 `syntax`、颜色值写成 `rgba(...)`、JSON 语法错，都会在这里失败）；
2. 每条主题都有 §1.5 的 13 个无 fallback 的 key；
3. 每个**写了**的颜色值都能过 gpui 的颜色解析器（`try_parse_color` / `try_parse_background`，`theme/color.rs:693`、`:763`）；
4. `themes[].name` 全局唯一且非空。

覆盖不到的两件事（要实机看）：颜色**搭配**好不好看，以及 `list.active.background` / `selection.background`
的 alpha 被 `clamp_alpha` 压到 ≤ 0.2 / 0.3 之后的实际观感（§1.7）。

---

## 8. 以后加主题就放这个目录

1. 在本目录新建 `*.json`，根必须是 `{"name": ..., "themes": [{"id": ..., "name": ..., "mode": "light|dark", "colors": {...}}]}`。
2. 每条主题都要写 `id`（小写、用 `-` 分隔，例如 `my-theme-dark`）：它是**设置文件里持久化的值**，
   `name` 只是显示名。不写也会工作（按名字 slug 兜底），但那时改 `name` 就会断掉用户的引用（§0）。
3. `themes[].id` 与 `themes[].name` 都必须全局唯一（与内置的 `Default Light` / `Default Dark` 也不能同名/同 id）。
4. 明暗各写一条；一个文件放一条或两条都行（文件名不影响加载）。**不要放进子目录**——`reload` 不递归（`registry.rs:242-262`）。
5. **在 `crates/settings/src/theme.rs` 的 `BUNDLED_THEMES` 里登记这一份**（`include_str!`），
   否则运行期看不到它、打包后更没有；`every_theme_file_is_embedded_and_loadable` 会用目录清单比对这张表，漏登记即测试失败。
6. 至少写 §1.5 的 13 个无 fallback 的 key。
7. key 只能取自 §1.3 的 139 个；**拼错的 key 不会报错，只会静默失效**。
8. 值用 `#RRGGBB` / `#RRGGBBAA` / Tailwind 色名；**不要用 `rgba()`**。
9. 保存为 UTF-8（无 BOM）、LF；不要写注释和尾逗号——解析失败就是整份主题消失。
10. 想让**编辑器语法色**也跟着换，再加一个 `highlight` 段：`{"editor.background": …, "editor.foreground": …, "editor.active_line.background": …, "syntax": {"comment": {"color": …}, …}}`。
    映射照 §7.2 的表，注意 `comment_doc` 的下划线写法与"整段替换"（§7.4）。
11. 保存后 `notify` 会自动重载（`registry.rs:186-223`），不需要重启进程；但**当前激活主题的切换逻辑由 Rust 侧负责**。
12. 跑一遍 `cargo test -p lithe-gpui-settings theme::`（§7.5）。

⚠️ **改已有主题时的两条提醒**（都和"播种只补缺失、不覆盖"有关）：

- 已经跑过一次的用户，其 `<配置目录>/themes/` 里那份**不会自动更新**：要么让用户删掉旧副本重启，
  要么**换一个文件名**（新文件名会被补种，但旧文件仍在用户目录里、表现为多出一条旧主题）。
- **改 `name` 时保留 `id`**：这样用户的设置仍然指向同一个主题；反之（改 `id`）等于让所有用户的
  那份设置失效并回落默认主题。
