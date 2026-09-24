# 04 · 主题与通用组件规格（Windows 前端 → Rust + gpui-kit 0.6.6）

> **规格来源**：`windows/tauri/src/`（Tauri v2 + React 19 + TypeScript + Tailwind v4 + shadcn 结构 + **Base UI** 原语）。
> 旧的 macOS 规格**已作废**，本文不引用 `macos/` 任何内容。
> **阅读前提（路径修正）**：任务书写的 `windows/tauri/src/components/ui/` **未找到**；通用组件层实际位于 **`windows/tauri/src/ui/`**（62 个 `.tsx`）。`windows/tauri/src/components/` 目录**不存在**。
> **证据约定**：每条结论标注 `相对路径:行号`。查不到写「未找到」，不猜。
> 本次为**只读调查**，未修改任何已有文件；本文与 `04-components-raw.md` 是仅有的两个新增产物。

---

## 0. 结论速览（给实施者）

1. **1 单位 = 4px 的间距阶梯在 gpui 里同样存在，而且更彻底**：gpui 的 `p_1()/h_8()/gap_2()` 等全部是 **rem 派生**（`rems(n) = n × window.rem_size()`），而 `gpui_component::Root` 把 `rem_size` 设为 `theme.font_size`（`gpui-component-0.6.6/src/root.rs:582`）。gpui 的 `box_style_suffixes()` 数值与 Tailwind 完全同构（`1 → 0.25rem`，`8 → 2rem`，见 `gpui-pre-macros-0.3.6/src/styles.rs:926-1092`）。
   → **推论：Rust 侧应把 `Theme::font_size` 设成「Windows 侧的根字号」= `16px × (uiFontSize / 13)`，而不是 `uiFontSize` 本身。** 只有这样 `h-8`/`p-1` 的缩放行为才与 Windows 一致。
   ⚠️ **取证提示**：`h_8()` 的 px 换算定义在 **`gpui-pre-macros-0.3.6/src/styles.rs`**（crate 名是 `gpui-pre-macros`，**不是** `gpui-macros`，后者源码不在 registry 目录内）。同行的独立调查曾在 `gpui-macros` 下寻找并得出「未找到」，属于找错 crate；本表数值以 `gpui-pre-macros-0.3.6/src/styles.rs:926-1092` 为准。
2. **字号阶梯也同构**：gpui 的 `text_xs/sm/base/lg/xl/2xl/3xl` = `rems(0.75/0.875/1/1.125/1.25/1.5/1.875)`（`gpui-pre-0.3.6/src/styled.rs:546-588`），与 Tailwind 的 `text-xs…text-3xl` 数值完全相同。
3. **圆角阶梯不同构，必须自建**：Lithe 把 Tailwind 圆角阶梯整体改写成 `calc(var(--radius) * k)`（`windows/tauri/src/styles/theme.css:6-12`，`--radius: 8px`），而 gpui 的 `rounded_sm…3xl` 是固定的 rem 阶梯（`gpui-pre-macros-0.3.6/src/styles.rs:1228-1276`），`RadiusTokens` 默认值也是另一套（`gpui-base-0.6.6/src/theme_tokens.rs:118-128`）。
4. **组件层是「Base UI 原语 + cva 变体」，不是 Radix**：36/62 文件用 `@base-ui/react`，`@radix-ui/*`、`cmdk`、`vaul` 均为 0（`windows/tauri/src/ui/button.tsx:2`、`windows/tauri/src/ui/accordion.tsx:1`）。Rust 侧不迁移封装，**只迁移 token + 变体矩阵**。
5. **gpui-kit 缺 4 大块 token**：Lithe chrome 尺寸体系（`--lithe-*`）、第三级文字色 `subtle-foreground`、行选中色 `selected`、以及 git/terminal/cursor/markdown 四组专用色。都需要自建结构体（见 §3.4）。
6. **两处「容易错杀」的地方已核实**：
   - `punctuation` 与 `operator` 在 gpui 的 `syntax.*` 里**是存在的**（41 键里含它们）→ 真正无对应的只有 `boolean` / `null` / `jsx` / `jsx-attribute`（见 §3.4(4)）。
   - `p_1()/h_8()` 的 px 换算定义在 **`gpui-pre-macros-0.3.6/src/styles.rs:926-1092`**（crate 名是 `gpui-pre-macros`，不是 `gpui-macros`）。
7. **gpui 写死的像素常量清单**见 §2.17 —— 其中 `Dialog` 宽 448、`Popover` offset 6 / margin 8、`Notification` 宽 382、`SidebarMenuItem` 高 28、`Separator` 线宽 1、`RADIUS_FULL` 9999 **天然与 Lithe 对齐**；`TITLE_BAR_HEIGHT = 34`（Lithe 40）是**第一个必须显式覆盖**的常量。

---

## 1. Token 总表

### 1.0 变量的两级来源（先看这张图）

| 层 | 内容 | 定义点 | 是否随主题变 |
| --- | --- | --- | --- |
| **A 主题色** | 39 个颜色键 + 18 个 syntax 键 | `windows/tauri/src/extensions/themes/builtin/lithe.json` | ✅ 运行时写进 `document.documentElement.style` |
| **B 结构常量** | 尺寸/圆角/动效/阴影/字体族/密度 | `windows/tauri/src/styles/theme.css` `:root` | ❌ 只有 `data-window-chrome-density` 切换时变 |
| **C 派生色** | `--border-strong`、`--popover`、`--symbol-*` … | `windows/tauri/src/styles/theme.css:140-176` | ✅ 因为它们引用 A 层变量 |
| **D 滚动条** | 11 个 `--app-scrollbar-*` | `windows/tauri/src/styles/scrollbars.css:1-18` | 明/暗两套 |
| **E 玻璃/chrome** | 13 个 `--lithe-glass-*` / `--lithe-chrome-icon-*` / `--lithe-chrome-control-*` | `windows/tauri/src/styles/window-transparency.css:5-36` | 明/暗两套 |
| **F Tailwind 注册** | `--color-*` / `--font-*` / `--radius-*` / `--leading-row` | `windows/tauri/src/styles/theme.css:1-101` (`@theme inline`) | 否，只做别名 |

CSS 唯一入口：`windows/tauri/src/styles.css:1-20`（导入 tailwindcss → `styles/theme.css` → base → scrollbars → syntax-tokens → rendered-code-tokens → window-transparency → utilities → tw-animate-css）。

主题写入机制：`themeRegistry.applyTheme()` 把 `theme.cssVariables` 与 `theme.syntaxTokens` 合并后逐条 `root.style.setProperty(key, value)`，并设置 `data-theme` / `data-theme-type`（`windows/tauri/src/extensions/themes/theme-registry.ts:79-99`）；切换前会 `removeProperty` 上一套多余键（`:86-90`）。
键名规范化：`lithe.json` 的 `colors.<key>` → CSS 变量 `--<key>`（`windows/tauri/src/extensions/themes/theme-file.ts:238-242`）；`syntax.<key>` → `--syntax-<key>`（`windows/tauri/src/extensions/themes/syntax-token-colors.ts:141-143`）。

---

### 1.1 颜色 token 总表（39 键，明/暗，来源 `lithe.json`）

Tailwind 工具类前缀统一为 `--color-*`（`windows/tauri/src/styles/theme.css:14-100`），因此下文「Tailwind 类」即 `bg-<name>` / `text-<name>` / `border-<name>`。

| # | 键 / CSS 变量 | Tailwind 类 | Lithe Light | Lithe Dark | 明 行 | 暗 行 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `--background` | `bg-background` | `#ffffff` | `#1e1f22` | `lithe.json:13` | `lithe.json:79` |
| 2 | `--surface` | `bg-surface` | `#f7f8fa` | `#2b2d30` | `:14` | `:80` |
| 3 | `--foreground` | `text-foreground` | `#1f2328` | `#dfe1e5` | `:15` | `:81` |
| 4 | `--muted-foreground` | `text-muted-foreground` | `#4f5965` | `#b4b8bf` | `:16` | `:82` |
| 5 | `--subtle-foreground` | `text-subtle-foreground` | `#68717d` | `#8b929e` | `:17` | `:83` |
| 6 | `--border` | `border-border` | `#dfe1e5` | `#43454a` | `:18` | `:84` |
| 7 | `--accent` | `bg-accent` | `#edf3ff` | `#393b40` | `:19` | `:85` |
| 8 | `--selected` | `bg-selected` | `#d4e2ff` | `#2e436e` | `:20` | `:86` |
| 9 | `--selection` | `selection:bg-selection` | `rgba(53, 116, 240, 0.2)` | `#214283` | `:21` | `:87` |
| 10 | `--primary` | `bg-primary` / `text-primary` | `#3574f0` | `#3574f0` | `:22` | `:88` |
| 11 | `--cursor` | 未注册到 `@theme` | `#1f2328` | `#ced0d6` | `:23` | `:89` |
| 12 | `--cursor-vim-normal` | `--color-cursor-vim-normal` | `rgba(53, 116, 240, 0.62)` | `rgba(53, 116, 240, 0.68)` | `:24` | `:90` |
| 13 | `--cursor-vim-insert` | `--color-cursor-vim-insert` | `#3574f0` | `#3574f0` | `:25` | `:91` |
| 14 | `--destructive` | `text-destructive` / `bg-destructive` | `#cf3f4f` | `#db5c5c` | `:26` | `:92` |
| 15 | `--success` | `text-success` | `#27864f` | `#57965c` | `:27` | `:93` |
| 16 | `--warning` | `text-warning` | `#a86400` | `#d6ae58` | `:28` | `:94` |
| 17 | `--info` | `--color-info` | `#3574f0` | `#548af7` | `:29` | `:95` |
| 18 | `--git-modified` | `--color-git-modified` | `#a86400` | `#d9a441` | `:30` | `:96` |
| 19 | `--git-modified-staged` | `--color-git-modified-staged` | `#bd7411` | `#e5b75e` | `:31` | `:97` |
| 20 | `--git-added` | `--color-git-added` | `#27864f` | `#4cc38a` | `:32` | `:98` |
| 21 | `--git-deleted` | `--color-git-deleted` | `#cf3f4f` | `#f16d75` | `:33` | `:99` |
| 22 | `--git-untracked` | `--color-git-untracked` | `#0877c1` | `#58a6e7` | `:34` | `:100` |
| 23 | `--git-renamed` | `--color-git-renamed` | `#7656a8` | `#c8a2f4` | `:35` | `:101` |
| 24 | `--terminal-black` | `--color-terminal-black` | `#1f2328` | `#0f1012` | `:36` | `:102` |
| 25 | `--terminal-red` | `--color-terminal-red` | `#cf3f4f` | `#f16d75` | `:37` | `:103` |
| 26 | `--terminal-green` | `--color-terminal-green` | `#27864f` | `#4cc38a` | `:38` | `:104` |
| 27 | `--terminal-yellow` | `--color-terminal-yellow` | `#a86400` | `#d9a441` | `:39` | `:105` |
| 28 | `--terminal-blue` | `--color-terminal-blue` | `#0877c1` | `#58a6e7` | `:40` | `:106` |
| 29 | `--terminal-magenta` | `--color-terminal-magenta` | `#8a4fb0` | `#c8a2f4` | `:41` | `:107` |
| 30 | `--terminal-cyan` | `--color-terminal-cyan` | `#147d83` | `#61c0bf` | `:42` | `:108` |
| 31 | `--terminal-white` | `--color-terminal-white` | `#68717d` | `#c4c9d1` | `:43` | `:109` |
| 32 | `--terminal-bright-black` | `--color-terminal-bright-black` | `#7a8491` | `#757d89` | `:44` | `:110` |
| 33 | `--terminal-bright-red` | `--color-terminal-bright-red` | `#e05260` | `#ff858d` | `:45` | `:111` |
| 34 | `--terminal-bright-green` | `--color-terminal-bright-green` | `#369d62` | `#68d5a0` | `:46` | `:112` |
| 35 | `--terminal-bright-yellow` | `--color-terminal-bright-yellow` | `#bf7a16` | `#edbb5c` | `:47` | `:113` |
| 36 | `--terminal-bright-blue` | `--color-terminal-bright-blue` | `#1684cb` | `#75b9f0` | `:48` | `:114` |
| 37 | `--terminal-bright-magenta` | `--color-terminal-bright-magenta` | `#a267c4` | `#dab9ff` | `:49` | `:115` |
| 38 | `--terminal-bright-cyan` | `--color-terminal-bright-cyan` | `#238f95` | `#7bd3d2` | `:50` | `:116` |
| 39 | `--terminal-bright-white` | `--color-terminal-bright-white` | `#1f2328` | `#ffffff` | `:51` | `:117` |

**必填键只有 9 个**（校验器强制）：`background`、`surface`、`foreground`、`muted-foreground`、`subtle-foreground`、`border`、`accent`、`selected`、`primary`（`windows/tauri/src/extensions/themes/theme-file.ts:5-15`、`:173-177`）。
**旧格式键重映射**（自定义主题兼容）：`primary-bg→background`、`secondary-bg→surface`、`text→foreground`、`text-light→muted-foreground`、`text-lighter→subtle-foreground`、`hover→accent`、`selection-bg→selection`、`accent→primary`、`error→destructive`（`theme-file.ts:17-27`）。Rust 侧若支持导入用户主题，必须复刻这张表。

---

### 1.2 派生颜色 token（C 层，随主题、随明暗）

来源 `windows/tauri/src/styles/theme.css:140-176`（`:root`）与 `:204-215`（dark 阴影）。

| CSS 变量 | 取值 | 行 | 说明 |
| --- | --- | --- | --- |
| `--border-strong` | `color-mix(in srgb, var(--border) 72%, var(--foreground) 28%)` | `theme.css:140` | 焦点边框，被 input/textarea/combobox 复用 |
| `--popover` | `var(--surface)` | `:141` | shadcn 别名 |
| `--popover-foreground` | `var(--foreground)` | `:142` | |
| `--accent-foreground` | `var(--foreground)` | `:143` | |
| `--muted` | `var(--surface)` | `:144` | |
| `--input` | `var(--border)` | `:145` | |
| `--ring` | `var(--border-strong)` | `:146` | |
| `--card` / `--card-foreground` | `var(--surface)` / `var(--foreground)` | `:147-148` | |
| `--primary-foreground` | `var(--background)` | `:149` | **注意：不是白色** |
| `--secondary-foreground` | `var(--foreground)` | `:150` | |
| `--success` | `var(--primary)` | `:151` | **`--success` 被别名成主色**（与 lithe.json 的 `success` 冲突，见 §5 未查清） |
| `--warning` | `var(--muted-foreground)` | `:152` | **同上，`--warning` 被别名成次级文字色** |
| `--tab-bar-bg` / `--tab-active-bg` | `var(--background)` | `:155-156` | IntelliJ 风格：tab 条与编辑器同底，靠 1px 下边线分隔 |
| `--tab-hover-bg` | `color-mix(in srgb, var(--accent) 72%, transparent)` | `:157` | |
| `--symbol-function` | `var(--syntax-function)` | `:170` | 符号图标 7 键，见下表 |
| `--symbol-type` | `var(--syntax-type, var(--syntax-tag))` | `:171` | |
| `--symbol-interface` | `var(--syntax-regex, var(--syntax-operator, var(--syntax-attribute)))` | `:172` | |
| `--symbol-enum` | `var(--syntax-number)` | `:173` | |
| `--symbol-variable` | `var(--syntax-property, var(--syntax-variable))` | `:174` | |
| `--symbol-property` | `var(--syntax-string)` | `:175` | |
| `--symbol-type-parameter` | `var(--syntax-operator, var(--syntax-attribute))` | `:176` | |

`--color-symbol-*` 已注册为 Tailwind 类（`theme.css:77-83`）。

---

### 1.3 尺寸 / 结构 token 表（B 层，两档密度）

默认档 = 「focused」，`data-window-chrome-density="comfortable"` 覆盖为另一套（`windows/tauri/src/styles/theme.css:104-200`；属性由 `windows/tauri/src/features/settings/lib/ui-preferences.ts:5-11` 下发，合法值只有 `focused | comfortable`，`windows/tauri/src/features/settings/lib/settings-normalization.ts:149-152`）。
**rem 基准 = 16px**，因为 `html { font-size: calc(16px * var(--app-ui-scale)) }`（`theme.css:218-221`）。

| Token | focused（默认） | comfortable | 像素（focused） | 行 |
| --- | --- | --- | --- | --- |
| `--app-ui-font-size` | `13px` | — | 13 | `theme.css:112` |
| `--app-ui-scale` | `1` | — | — | `:113` |
| `--ui-text-caption` | `12px` | `13px` | 12 / 13 | `:114` / `:189` |
| `--ui-text-chrome` | `13px` | `14px` | 13 / 14 | `:115` / `:190` |
| `--ui-text-sm` | `var(--app-ui-font-size)` | — | 13 | `:116` |
| `--ui-text-base` | `var(--app-ui-font-size)` | — | 13 | `:117` |
| `--lithe-title-bar-height` | `2.5rem` | — | 40（macOS 覆盖 2.25rem=36） | `:118` / `:180` |
| `--lithe-footer-height` | `1.5rem` | `2rem` | 24 / 32（status bar 隐藏时 0） | `:119` / `:191`；`utilities.css:106-108` |
| `--lithe-pane-header-height` | `2.25rem` | — | 36 | `:120` |
| `--lithe-tab-bar-height` | `var(--lithe-pane-header-height)` | — | 36 | `:121` |
| `--lithe-tab-height` | `1.75rem` | `2rem` | 28 / 32 | `:122` / `:192` |
| `--lithe-tab-max-width` | `12.5rem` | — | 200 | `:123` |
| `--lithe-sidebar-header-height` | `2rem` | `2.25rem` | 32 / 36 | `:124` / `:193` |
| `--lithe-workbench-gap` | `4px` | — | 4 | `:125` |
| `--lithe-chrome-control-height` | `1.5rem` | `1.75rem` | 24 / 28 | `:126` / `:194` |
| `--lithe-chrome-hit-target` | `1.75rem` | `2rem` | 28 / 32 | `:127` / `:195` |
| `--lithe-chrome-line-height` | `1rem` | — | 16 | `:128` |
| `--lithe-chrome-gap-tight` | `2px` | `4px` | 2 / 4 | `:129` / `:196` |
| `--lithe-chrome-gap` | `4px` | `6px` | 4 / 6 | `:130` / `:197` |
| `--lithe-chrome-gap-loose` | `6px` | `8px` | 6 / 8 | `:131` / `:198` |
| `--lithe-chrome-padding-inline` | `8px` | `10px` | 8 / 10 | `:132` / `:199` |
| `--lithe-chrome-radius` | `4px` | — | 4 | `:133` |
| `--leading-row` | `1.35` | — | — | `:4` |

**未找到**：`--lithe-chrome-hit-target` 在全仓库除 `theme.css:127` / `:195` 外**没有任何引用点**（grep 结果只命中定义处）。

`--app-ui-font-size` / `--app-ui-scale` 的来源公式：
`normalizeUiFontSize` 把字号吸附到 0.5 步长并 clamp 到 `[10, 24]`（`windows/tauri/src/features/settings/lib/ui-font-size.ts:3-16`），默认 13（`windows/tauri/src/features/settings/config/typography-defaults.ts:14`），
`getUiFontScale(v) = v / 13`（`ui-font-size.ts:30-33`），
写入点 `windows/tauri/src/features/settings/lib/appearance-bootstrap.ts:153-155` 与 `windows/tauri/src/features/settings/components/font-style-injector.tsx:40-42`。

**文件树行的独立密度公式**（不走 CSS 变量，走内联 px）：
`getFileTreeRowHeight(uiFontSize) = max(24, uiFontSize × 1.35 + 6)`（`windows/tauri/src/features/file-explorer/lib/file-tree-row.ts:1-14`），实测 `13 → 24`、`15 → 26.25`、`18 → 30.3`（`windows/tauri/src/features/file-explorer/lib/file-tree-row.test.ts:6-12`）；以 `--file-tree-row-height: ${rowHeight}px` 注入（`windows/tauri/src/features/file-explorer/components/file-explorer-viewport.tsx:176`）。

---

### 1.4 圆角阶梯（B 层）

`windows/tauri/src/styles/theme.css:6-12`，基数为 `--radius: 8px`（`:134`）：

| Token | 表达式 | 实际值（--radius=8px） |
| --- | --- | --- |
| `--radius-sm` | `calc(var(--radius) * 0.6)` | **4.8px** |
| `--radius-md` | `calc(var(--radius) * 0.8)` | **6.4px** |
| `--radius-lg` | `var(--radius)` | **8px** |
| `--radius-xl` | `calc(var(--radius) * 1.4)` | **11.2px** |
| `--radius-2xl` | `calc(var(--radius) * 1.8)` | **14.4px** |
| `--radius-3xl` | `calc(var(--radius) * 2.2)` | **17.6px** |
| `--radius-4xl` | `calc(var(--radius) * 2.6)` | **20.8px** |
| `--lithe-chrome-radius` | 字面量 | **4px** |
| `--app-scrollbar-radius` | 字面量 | `999px` |

其它散落的圆角字面量：卡片/对话框/浮层用 `rounded-xl`(=12px，Tailwind 默认)、菜单/列表项用 `rounded-md`(=6px) / `rounded-lg`(=8px) / `rounded-sm`(=4px)；badge 为 `rounded-full`（`windows/tauri/src/ui/badge.tsx:6`）。

---

### 1.5 间距阶梯（Tailwind v4，CSS-first）

**未找到 `tailwind.config.js|ts|mjs`**；仓库使用 CSS-first：`@theme inline { … }`（`windows/tauri/src/styles/theme.css:1`）。
`@theme` 块**没有**定义 `--spacing` / `--text-*` / `--radius-*` 的原始阶梯（只定义了 `--radius-*` 的 *派生* 表达式 `:6-12`），因此间距与字号来自 Tailwind v4 内置默认值。
本机 `windows/tauri/node_modules` **未安装**（`Test-Path` = `False`），故无法给出 `node_modules/tailwindcss/theme.css` 的行号 —— **这一条标注为「未在仓库内查证」**，数值遵循任务给定的「1 单位 = 4px」。

仓库内可验证的机制证据：`windows/tauri/src/ui/calendar.tsx:27` 使用 `[--cell-size:--spacing(7)]`，即 Tailwind v4 的 `--spacing()` 函数，`7 × 4px = 28px`。

| 单位 | 类名示例 | px | 单位 | 类名示例 | px |
| --- | --- | --- | --- | --- | --- |
| 0.5 | `p-0.5` `gap-0.5` | 2 | 8 | `h-8` `p-8` | 32 |
| 1 | `p-1` `gap-1` | 4 | 9 | `h-9` | 36 |
| 1.5 | `p-1.5` `gap-1.5` | 6 | 10 | `h-10` `min-w-10` | 40 |
| 2 | `p-2` `gap-2` | 8 | 12 | `h-12` | 48 |
| 2.5 | `p-2.5` | 10 | 14 | `h-14` | 56 |
| 3 | `p-3` `min-h-3` | 12 | 16 | `w-16` | 64 |
| 3.5 | `h-3.5` `size-3.5` | 14 | 20 | `min-w-20` | 80 |
| 4 | `p-4` `size-4` | 16 | 24 | `w-24` | 96 |
| 5 | `h-5` `size-5` | 20 | 40 | `max-w-40` | 160 |
| 6 | `h-6` `size-6` | 24 | 50 | `max-w-50` | 200 |
| 7 | `h-7` `size-7` | 28 | 60 | `min-w-60` | 240 |

**实测高频尺寸**（这些是 Rust 侧必须对齐的「控制高度阶梯」）：

| 语义 | class | px | 证据 |
| --- | --- | --- | --- |
| 输入类控件 xs | `h-6` | 24 | `windows/tauri/src/utils/control-variants.ts:24` |
| 输入类控件 sm（默认） | `h-7` | 28 | `control-variants.ts:25` |
| 输入类控件 md | `h-8` | 32 | `control-variants.ts:26` |
| Button xs / icon-xs | `h-6` / `size-6` | 24 | `windows/tauri/src/ui/button.tsx:23,27` |
| Button sm / icon-sm | `h-7` / `size-7` | 28 | `button.tsx:24,28` |
| Button default / icon | `h-8` / `size-8` | 32 | `button.tsx:22,26` |
| Button lg | `h-9` | 36 | `button.tsx:25` |
| Badge 高 | `h-6` | 24 | `windows/tauri/src/ui/badge.tsx:6` |
| TableHead 高 | `h-8` | 32 | `windows/tauri/src/ui/table.tsx:65` |
| Kbd 高 | `h-5` | 20 | `windows/tauri/src/ui/kbd.tsx:9` |
| Checkbox / Radio | `size-4` | 16 | `windows/tauri/src/ui/checkbox.tsx:10`、`radio-group.tsx:23` |
| Switch sm / md | `h-3.5 w-7` / `h-5 w-9` | 14×28 / 20×36 | `windows/tauri/src/ui/switch.tsx:24-30` |
| Spinner compact / 默认 | `size-3` / `size-4` | 12 / 16 | `windows/tauri/src/ui/spinner.tsx:27` |
| Toggle xs/sm/md | `min-h-6/7/8` | 24/28/32 | `windows/tauri/src/ui/toggle.tsx:15-17` |
| Progress sm / md | `h-1` / `h-1.5` | 4 / 6 | `windows/tauri/src/ui/progress.tsx:5-18` |
| 文件树行 | 公式 | 24 | `file-tree-row.ts:7-14` |
| 工作台分隔条宽/高 | `--lithe-workbench-gap` | 4 | `theme.css:125` |
| 菜单分隔线 | `h-px` | 1 | `windows/tauri/src/ui/context-menu.tsx:196` |
| 编辑器标签强调线 | `h-[3px]` | 3 | `windows/tauri/src/ui/tab-bar.tsx:204-209` |

---

### 1.6 字号阶梯

两套并存、互不换算（这是重写时最容易搞错的地方）：

**(a) Tailwind `text-*`（rem 派生，随根字号缩放）**

| class | rem | px @ root=16 | 仓库/上游证据 |
| --- | --- | --- | --- |
| `text-xs` | 0.75rem | 12 | `windows/tauri/src/ui/dropdown-menu.tsx:68,234` 使用 |
| `text-sm` | 0.875rem | 14 | 全仓库大量使用 |
| `text-base` | 1rem | 16 | `windows/tauri/src/ui/sheet.tsx:101` 使用 |
| `text-lg` | 1.125rem | 18 | 未在 `src/ui` 中直接命中 |
| `text-xl` | 1.25rem | 20 | 未在 `src/ui` 中直接命中 |
| `text-[0.8rem]` | 0.8rem | 12.8 | `windows/tauri/src/ui/calendar.tsx:76` |

**未找到**：仓库未覆盖 Tailwind 字号阶梯，`node_modules` 未安装 → Tailwind v4 默认数值（0.75/0.875/1/1.125/1.25/1.5/1.875）为「未在仓库内查证」。

**(b) Lithe `ui-text-*`（px 字面量，线性随 `--app-ui-font-size` 缩放）**

| class | 变量 | focused | comfortable | 定义行 |
| --- | --- | --- | --- | --- |
| `.ui-text-caption` | `--ui-text-caption` | 12px | 13px | `utilities.css:34-36` |
| `.ui-text-chrome` | `--ui-text-chrome` | 13px | 14px | `utilities.css:38-40` |
| `.ui-text-sm` | `--ui-text-sm` = `--app-ui-font-size` | 13px | 13px | `utilities.css:30-32` |
| `.ui-text-base` | `--ui-text-base` = `--app-ui-font-size` | 13px | 13px | `utilities.css:42-44` |

即：**`ui-text-sm`（13px）≠ `text-sm`（14px）**。重写时必须决定并统一 —— 见 §3.3。

**(c) 字体族（B 层，不随主题变）**

| 变量 | 值 | 行 |
| --- | --- | --- |
| `--app-font-family` | `"Microsoft YaHei UI", "Segoe UI", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, Roboto, "Helvetica Neue", Arial, sans-serif` | `theme.css:106-108` |
| `--editor-font-family` | `"Geist Mono", ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace` | `theme.css:109-111` |
| `--font-sans` | `var(--app-font-family)` | `theme.css:2` |
| `--font-mono` | `var(--editor-font-family)` | `theme.css:3` |
| `--leading-row` | `1.35` | `theme.css:4` |

字体文件由 `@fontsource/geist-sans`（400/500/600/700）与 `@fontsource/geist-mono`（400/500/600/700）本地导入（`windows/tauri/src/styles.css:1-8`；依赖声明 `windows/tauri/package.json:35-36`）。

---

### 1.7 阴影（B 层，明/暗两套）

| Token | Light | Dark | 行 |
| --- | --- | --- | --- |
| `--shadow-card` | `0 1px 2px rgb(0 0 0/.05), 0 2px 4px rgb(0 0 0/.02), 0 0 0 .5px rgb(0 0 0/.08)` | `0 1px 2px rgb(0 0 0/.24), 0 2px 4px rgb(0 0 0/.18), 0 0 0 .5px rgb(255 255 255/.1)` | `theme.css:158-159` / `:204-205` |
| `--shadow-popover` | `0 8px 16px /.04, 0 16px 32px /.03, 0 2px 4px /.04, 0 0 0 .5px /.1` | `0 8px 16px /.26, 0 16px 32px /.22, 0 2px 4px /.22, 0 0 0 .5px rgb(255 255 255/.1)` | `:160-162` / `:206-208` |
| `--shadow-dialog` | `0 8px 16px /.04, 0 24px 56px /.08, 0 2px 6px /.04, 0 0 0 .5px /.12` | `0 8px 16px /.3, 0 24px 56px /.36, 0 2px 6px /.24, 0 0 0 .5px rgb(255 255 255/.12)` | `:163-165` / `:209-211` |
| `--shadow-drag` | `0 8px 18px /.12, 0 18px 44px /.12, 0 0 0 .5px /.12` | `0 8px 18px /.36, 0 18px 44px /.34, 0 0 0 .5px rgb(255 255 255/.12)` | `:166-167` / `:212-214` |
| `--shadow-editor-sticky` | `0 8px 16px -14px rgb(0 0 0/.28)` | `0 10px 20px -16px rgb(0 0 0/.72)` | `:168` / `:215` |

**重写风险**：每条阴影的最后一层是 `0 0 0 0.5px <color>` —— 是一个 **0.5px 的 hairline 环**。gpui 的 `BoxShadow { spread_radius, blur_radius, inset }`（`gpui-base-0.6.6/src/theme_tokens.rs:225-233`）**能**表达 `spread_radius: px(0.5)`，但 0.5px 在非 100% DPI 下的栅格化会与 CSS 有差异；`ShadowTokens::elevations()` 只提供 3 档（`theme_tokens.rs:197-205`），不足以覆盖 Lithe 的 5 档。

---

### 1.8 动效（B 层）

| Token | 值 | 行 |
| --- | --- | --- |
| `--app-duration-fast` | `150ms` | `theme.css:135` |
| `--app-duration-normal` | `200ms` | `theme.css:136` |
| `--app-ease-smooth` | `cubic-bezier(0.22, 1, 0.36, 1)` | `theme.css:137` |
| `--app-ease-in-out` | `cubic-bezier(0.66, 0, 0.34, 1)` | `theme.css:138` |
| `--app-press-scale` | `1`（即**默认不缩放**，是按压缩放开关） | `theme.css:139` |

其它字面量动效时长（不在 token 体系内，Rust 侧需单独登记）：

| 场景 | 值 | 证据 |
| --- | --- | --- |
| 拖拽排序位移 | `duration: 180, easing: var(--app-ease-smooth)` | `windows/tauri/src/ui/tab-bar.tsx:71-79` |
| 折叠面板开合 | `animate-accordion-down/up` | `windows/tauri/src/ui/accordion.tsx:55` |
| Drop 指示器脉冲 | `dropIndicatorPulse 1s var(--app-ease-in-out) infinite` | `utilities.css:68-82` |
| 文本 shimmer | `textShimmer 2s linear infinite` | `utilities.css:84-104` |
| Drawer 抽屉 | `duration-450 ease-[cubic-bezier(0.22,1,0.36,1)]` | `windows/tauri/src/ui/drawer.tsx:110` |
| Toast 内容 | `duration-250 ease-[cubic-bezier(0.22,1,0.36,1)]` | `windows/tauri/src/ui/toast.tsx:73` |
| Sheet | `duration-200 ease-in-out` | `windows/tauri/src/ui/sheet.tsx:57` |
| Tooltip 延迟 | `delay={150} timeout={100} closeDelay={0}` | `windows/tauri/src/ui/tooltip.tsx:22` |
| Popover/Select/Combobox/Menubar 进出场 | `duration-(--app-duration-fast) ease-(--app-ease-smooth)` + `blur(2px)` + `scale(0.98)` | `select.tsx:69-70`、`combobox.tsx:240-249`、`popover.tsx:139`、`menubar.tsx:126-127` |

**减弱动效**：`html[data-reduce-motion="true"]` 把所有 `transition-duration/animation-duration` 压到 `1ms`、`animation-iteration-count: 1`（`utilities.css:110-119`），并对 `.drop-indicator` / `.ui-text-shimmer` 单独关动画（`:121-129`）；同时有一份 `@media (prefers-reduced-motion: reduce)` 镜像（`:131-152`）。`data-reduce-motion` 值为 `"true" | "system"`（`ui-preferences.ts:7`）。
React 侧还有 `useReducedMotionConfig()` 降级（`windows/tauri/src/ui/popover.tsx:74,78,86-89`）。

---

### 1.9 滚动条 token（D 层）

`windows/tauri/src/styles/scrollbars.css:1-18`：

| Token | Light | Dark | 行 |
| --- | --- | --- | --- |
| `--app-scrollbar-size` | `11px` | `11px` | `scrollbars.css:2` |
| `--app-scrollbar-thin-size` | `9px` | `9px` | `:3` |
| `--app-scrollbar-radius` | `999px` | `999px` | `:4` |
| `--app-scrollbar-track` | `transparent` | `transparent` | `:5` |
| `--app-scrollbar-thumb` | `rgba(120,120,120,0.42)` | `rgba(166,166,166,0.38)` | `:6` / `:14` |
| `--app-scrollbar-thumb-subtle` | `rgba(120,120,120,0.28)` | `rgba(166,166,166,0.24)` | `:7` / `:15` |
| `--app-scrollbar-thumb-hover` | `rgba(105,105,105,0.62)` | `rgba(188,188,188,0.58)` | `:8` / `:16` |
| `--app-scrollbar-thumb-active` | `rgba(85,85,85,0.72)` | `rgba(210,210,210,0.68)` | `:9` / `:17` |
| `--app-scrollbar-thumb-border` | `3px solid transparent`（等价 padding） | 同 | `:10` |

结构类名：`.custom-scrollbar` / `.custom-scrollbar-thin` / `.custom-scrollbar-auto` / `.scrollbar-hidden` / `.editor-scrollable` / `.tab-scroll-container`（`scrollbars.css:99-278`）。Thumb 最小长度 `36px`（`:66-67`）。`.editor-scrollable` 特意**不**用 `scroll-behavior: smooth`（`:241`）。
组件层滚动条（Base UI ScrollArea）：宽/高 `w-2.5`/`h-2.5`(10px)、thumb `w-1.5`/`h-1.5`(6px)、hover 显现（`windows/tauri/src/ui/scroll-area.tsx:125-134`）。

---

### 1.10 玻璃 / chrome 色 token（E 层）

仅在 `html:is(.platform-macos, .platform-windows):not([data-window-transparency="disabled"])` 下生效（`windows/tauri/src/styles/window-transparency.css:5-36`）：

| Token | 通用（dark） | light 覆盖 | 行 |
| --- | --- | --- | --- |
| `--lithe-glass-shell-bg` | `color-mix(in srgb, var(--surface) 12%, transparent)` | `… var(--surface) 42% …` | `:6` / `:24` |
| `--lithe-glass-control-bg` | `… var(--surface) 20% …` | `… var(--background) 42% …` | `:7` / `:25` |
| `--lithe-glass-control-active-bg` | `… var(--accent) 70% …` | `… var(--background) 78% …` | `:8` / `:26` |
| `--lithe-glass-control-border` | `… var(--border) 26% …` | `… var(--border) 42% …` | `:9` / `:27` |
| `--lithe-glass-popover-bg` | `… var(--background) 62% …` | `… var(--background) 78% …` | `:10` / `:28` |
| `--lithe-glass-popover-border` | `… var(--border) 50% …` | `… var(--border) 55% …` | `:11` / `:29` |
| `--lithe-glass-chrome-border` | `transparent` | `… var(--border) 45% …` | `:12` / `:30` |
| `--lithe-glass-chrome-overlay-bg` | `… var(--surface) 6% …` | `… var(--background) 10% …` | `:13` / `:31` |
| `--lithe-chrome-icon-color` | `… var(--muted-foreground) 78% …` | `… var(--foreground) 66% …` | `:14` / `:32` |
| `--lithe-chrome-icon-hover-color` | `var(--foreground)` | `color-mix(var(--foreground) 92%, var(--primary) 8%)` | `:15` / `:33` |
| `--lithe-chrome-icon-active-color` | `var(--foreground)` | `color-mix(var(--foreground) 88%, var(--primary) 12%)` | `:16` / `:34` |
| `--lithe-chrome-control-hover-bg` | `… var(--accent) 58% …` | `… var(--background) 62% …` | `:17` / `:35` |
| `--lithe-chrome-control-active-bg` | `… var(--accent) 72% …` | 未覆盖 | `:18` |

**重要**：`backdrop-filter` 被显式关闭（`-webkit-backdrop-filter: none; backdrop-filter: none;`，`window-transparency.css:46-47,70-71,79-80,95-96`）—— 窗口透明由 **Tauri 原生层**负责，CSS 只做半透明合成。Rust/gpui 侧没有等价的「原生窗后透」API，**这块必须重新设计或降级**（见 §3.4）。组件层仍有 `backdrop-blur-sm` 类（如 `windows/tauri/src/ui/popover.tsx:139`），但那是在透明窗口之上的二次模糊。

---

### 1.11 语法高亮 token（A 层 syntax，18 键 + 9 个无主键）

**18 个有主题键的 `--syntax-*`**（`lithe.json` 的 `syntax` 段）：

| 键 | CSS 变量 | Light | Dark | 明 行 | 暗 行 |
| --- | --- | --- | --- | --- | --- |
| `comment` | `--syntax-comment` | `#68717d` | `#7a7e85` | `lithe.json:53` | `:120` |
| `keyword` | `--syntax-keyword` | `#b83280` | `#cf8e6d` | `:54` | `:121` |
| `string` | `--syntax-string` | `#287d3c` | `#6aab73` | `:55` | `:122` |
| `number` | `--syntax-number` | `#a15c00` | `#2aacb8` | `:56` | `:123` |
| `function` | `--syntax-function` | `#14777d` | `#56a8f5` | `:57` | `:124` |
| `variable` | `--syntax-variable` | `#7656a8` | `#bcbec4` | `:58` | `:125` |
| `tag` | `--syntax-tag` | `#14777d` | `#d5b778` | `:59` | `:126` |
| `attribute` | `--syntax-attribute` | `#a15c00` | `#b3ae60` | `:60` | `:127` |
| `punctuation` | `--syntax-punctuation` | `#59636f` | `#bcbec4` | `:61` | `:128` |
| `constant` | `--syntax-constant` | `#a15c00` | `#c77dbb` | `:62` | `:129` |
| `property` | `--syntax-property` | `#075e9e` | `#c77dbb` | `:63` | `:130` |
| `type` | `--syntax-type` | `#4169a8` | `#bcbec4` | `:64` | `:131` |
| `operator` | `--syntax-operator` | `#b83280` | `#bcbec4` | `:65` | `:132` |
| `boolean` | `--syntax-boolean` | `#b83280` | `#cf8e6d` | `:66` | `:133` |
| `null` | `--syntax-null` | `#7656a8` | `#cf8e6d` | `:67` | `:134` |
| `regex` | `--syntax-regex` | `#287d3c` | `#6aab73` | `:68` | `:135` |
| `jsx` | `--syntax-jsx` | `#14777d` | `#d5b778` | `:69` | `:136` |
| `jsx-attribute` | `--syntax-jsx-attribute` | `#a15c00` | `#b3ae60` | `:70` | `:137` |

**9 个 markdown 专用 `--syntax-markdown-*` 在主题文件中未找到任何定义**（对 `windows/tauri/src/extensions/themes/` 全目录 grep `markdown-` / `markdown_` 无命中），它们只有硬编码 fallback：`markdown-heading` / `bold` / `italic` / `strikethrough` / `link` / `link-text` / `code` / `list` / `quote` 的 fallback 是 One-Dark 系配色（`windows/tauri/src/styles/syntax-tokens.css:110-154`）。**Rust 侧必须自己决定这 9 个键是否入主题。**

**syntax 值的合法性回落**（必须复刻）：若某个 syntax 值缺失，**或**与 `foreground` 的欧氏 RGB 距离 `< 28`，就替换为按明暗硬编码的 `FALLBACK_SYNTAX_BY_APPEARANCE`（light/dark 各 18 条，`windows/tauri/src/extensions/themes/syntax-token-colors.ts:3-44`、判定 `:99-109`、应用 `:123-128`）。

**两份 token 类映射表**（不同命名体系，别搞混）：
- Tree-sitter 路径：`.token-keyword` / `.token-string` / … （`windows/tauri/src/styles/syntax-tokens.css:5-107`），另有 `.token-text` / `.token-identifier` 用 `var(--foreground)`。
- 已渲染 HTML 路径（highlight.js/Prism 风格）：`.token.comment` / `.token.punctuation` / `.token.property` / `.token.keyword` …（`windows/tauri/src/styles/rendered-code-tokens.css:1-208`）；其中 ERB 相关（`:88-152`）与 `.token.dynamic-tag` / `.attribute-tag` / `.inline-js` / `.static-js` / `.tag-name` / `.dynamic-attr` / `.parameter`（`:154-208`）是**脱离主题的硬编码色**。

---

### 1.12 Tailwind 注册表（F 层，别名清单）

`windows/tauri/src/styles/theme.css:1-101` 的 `@theme inline` 共注册：
- 字体：`--font-sans`、`--font-mono`、`--leading-row`（`:2-4`）
- 圆角：`--radius-{sm,md,lg,xl,2xl,3xl,4xl}`（`:6-12`）
- 基础色：`background, surface, foreground, muted-foreground, subtle-foreground, border, border-strong, accent, selected, selection`（`:14-23`）
- shadcn 兼容色：`popover, popover-foreground, accent-foreground, muted, input, ring, card, card-foreground, primary-foreground, secondary-foreground`（`:25-34`）
- Tab：`tab-bar, tab-active, tab-hover`（`:36-38`）与 `primary`（`:40`）
- 光标：`cursor, cursor-vim-normal, cursor-vim-insert`（`:42-44`）
- 语义：`destructive, success, warning, info`（`:46-49`）
- Git：6 键（`:51-56`）
- Syntax：18 键（`:58-75`）
- Symbol：7 键（`:77-83`）
- Terminal：16 键（`:85-100`）

→ **共 39 主题色 + 15 个别名/派生 + 18 syntax + 7 symbol + 16 terminal = 95 个 `--color-*` 工具类**。

---

## 2. 通用组件清单（`windows/tauri/src/ui/`）

**目录事实**：62 个 `.tsx`（`windows/tauri/src/ui/icons.css:1` 所在目录），所有组件都从 `@/utils/cn` 取 `cn`（唯一例外 `direction.tsx`，纯再导出）。
**复用度实测**（排除 `src/ui/` 自身与 `*.test.*` 后的 `@/ui/<name>` import 计数）：`icons` 195、`button` 157、`dialog` 57、`spinner` 53、`input` 52、`empty` 41、`dropdown` 40、`command` 31、`select` 31、`scroll-area` 25、`badge` 23、`tooltip` 17、`sidebar` 16、`context-menu` 15、`checkbox` 14、`textarea` 12、`switch` 11、`alert` 9、`field` 9、`search` 6。**`toast` / `sonner` / `slider` / `chart` / `calendar` / `sheet` / `separator` / `carousel` / `input-otp` / `radio-group` / `pagination` / `drawer` / `label` / `skeleton` / `accordion` 在 `src/ui` 外 0 引用**（`label`/`skeleton`/`separator` 只被 `src/ui` 内部复用）。

### 2.1 跨组件共享基元（不在 `src/ui`，但决定多数控件尺寸）

`windows/tauri/src/utils/control-variants.ts`（全文件 38 行）：
- `controlSurfaceVariants`：`variant = default | ghost | inline`；`default` = `rounded-lg border border-border bg-surface focus:border-border-strong focus:bg-surface focus:ring-1 focus:ring-border-strong/35`（`:8-12`）
- `controlSizeVariants`：`size = xs | sm | md` → `h-6 ui-text-sm` / `h-7 ui-text-sm` / `h-8 ui-text-base`，默认 `sm`（`:21-32`）
- `controlIconSizes = { xs: 12, sm: 12, md: 14 }`（`:34-38`）

被 `input.tsx`、`number-input.tsx`、`combobox.tsx`、`select.tsx` 复用。

---

### 2.2 按钮 — `button.tsx`

| 项 | 内容 |
| --- | --- |
| 用途 | 全站基础按钮；支持 `render` 多态、内置 tooltip/快捷键 |
| 结构 | `useRender({ defaultTagName: "button" })`（Base UI `use-render`，`button.tsx:2,69-70`）；`data-slot="button"` + `data-variant`/`data-size`/`data-active`（`:74-77`）；有 `tooltip` 时外包 `@/ui/tooltip`（`:88-97`） |
| 变体 `variant` | `default` = `border-0 bg-accent text-foreground hover:bg-selected`；`accent` = `border border-primary/30 bg-primary/12 text-primary hover:bg-primary/20 data-[active=true]:…`；`ghost` = `border-0 bg-transparent text-subtle-foreground hover:bg-accent hover:text-foreground data-[active=true]:bg-accent`；`danger` = `border-0 bg-transparent text-foreground hover:bg-destructive/10 hover:text-destructive`（`:13-19`） |
| 尺寸 `size` | `default h-8 px-3`、`xs h-6 gap-1 px-1.5`、`sm h-7 px-2.5`、`lg h-9 px-4`、`icon size-8 p-0`、`icon-xs size-6 p-0`、`icon-sm size-7 p-0`（`:22-28`） |
| 状态 | 基类含 `active:scale-(--app-press-scale) focus-visible:ring-2 focus-visible:ring-primary/20 disabled:opacity-50`、`[&_svg:not([class*='size-'])]:size-3.5`（`:9`） |
| **gpui-kit 类型** | `gpui_component::button::{Button, ButtonVariant}`（`gpui-component-0.6.6/src/button/button.rs:167`）。`ButtonVariant` = `Default / Primary / Secondary / Danger / Info / Success / Warning / Ghost / Link / Text / Custom(ButtonCustomVariant)`（`button.rs:142-155`）；尺寸用 `Sizable::with_size(Size)`，`Size = XSmall/Small/Medium/Large/Size(Pixels)`（`gpui-component-0.6.6/src/sizing.rs:6-13`） |

**映射注意**：gpui 的 `ButtonVariant::Primary` 是**实心主色**，而 Lithe 的 `variant="default"` 才是「浅底（accent）」，Lithe 的 `accent` 变体是「描边+主色文字」。语义错位，不能按名字对齐（见 §3.3）。

---

### 2.3 输入框 — `input.tsx` / `textarea.tsx` / `number-input.tsx` / `input-group.tsx`

| 组件 | 要点 | 证据 |
| --- | --- | --- |
| `Input`（default） | `size = xs/sm/md`（默认 `sm`），`variant = default/ghost/inline`；高度来自 `controlSizeVariants`；有图标时外层 `relative` + 绝对定位图标（`left-1.5/left-2/left-2.5`）；padding 组合 `xs+left="pl-6 pr-2 py-1"`、`sm+left="pl-7 pr-2 py-1"`、`md+left="pl-9 pr-3 py-1"`；placeholder `text-subtle-foreground` | `windows/tauri/src/ui/input.tsx:13-14,48-56,124-134,146-147,173-175` |
| `InlineRenameInput` | 就地重命名：`size` 默认 `xs`；`appearance = inline/field`、`tone = default/muted`、`width = full/content`；Enter 提交 / Escape 取消 / blur 提交 | `input.tsx:67-87,203,242-252` |
| `Textarea` | `size = sm/md`（默认 `sm`）→ `px-2 py-1 ui-text-sm` / `px-3 py-2 ui-text-base`；`variant = default/ghost`；`resize-y` | `windows/tauri/src/ui/textarea.tsx:7,12,16-24,28` |
| `NumberInput` | `root` → `Decrement` / `Input` / `Increment`，按钮为 `Button size="icon-xs" variant="ghost"`；`padding = xs/sm px-2, md px-3`；文字 `xs/sm ui-text-sm, md ui-text-base`；`min-w-[5ch] text-center tabular-nums` | `windows/tauri/src/ui/number-input.tsx:27-37,55,76-121` |
| `InputGroup` | `min-h-7`；`Addon` 的 `align = inline-start/inline-end/block-start/block-end`；`InputGroupButton size = xs h-5 / sm h-6 / icon-xs size-5 / icon-sm size-6`；内部输入用 `variant="ghost"` | `windows/tauri/src/ui/input-group.tsx:14,22-37,61-73,113-132` |

| **gpui-kit 类型** | `gpui_component::input::{InputState, TextInput}`（模块 `input`；`gpui-component-0.6.6/src/input/mod.rs:45-51` 导出 `group::*`、`input::*`、`Textarea`、`NumberInput`、`AnyInputState`）。高度由 `StyleSized::input_h(size)` 决定：`XSmall h_5(20px) / Small h_6(24px) / Medium h_8(32px) / Large h_11(44px)`（`gpui-component-0.6.6/src/sizing.rs:261-269`）；水平内边距 `input_px`：`Large 12 / Medium 10 / Small 8 / XSmall 4`（`:147-155`）；垂直 `input_py`：`Large 10 / Medium 8 / Small 2 / XSmall 0`（`:158-166`） |

**映射差距**：gpui 的 `Size::Medium` 输入高 **32px**，而 Lithe 默认输入高 `h-7` = **28px**（`control-variants.ts:25`）。gpui 的 `Size::Small` = 24px ≈ Lithe 的 `xs`。→ Lithe 的 `sm` 在 gpui 里没有对应档，需要 `with_size(px(28.))` 或直接给 `Size::Size`。

---

### 2.4 下拉 / 选择 — `select.tsx` / `combobox.tsx` / `native-select.tsx`

| 组件 | 要点 | 证据 |
| --- | --- | --- |
| `Select` | `size = xs/sm/md`（默认 `sm`），`variant = default/ghost`（**默认 `ghost`**）；触发器复用 `buttonVariants`，`getButtonSize` 把 `md→default`、`sm→sm`、`xs→xs`（iconOnly 时 `md→icon`/`sm→icon-sm`/其他 `icon-xs`）；列表项 `min-h-7 rounded-lg px-2 py-1.5`；弹层 `rounded-xl border bg-surface/95 shadow-(--shadow-popover) backdrop-blur-sm`，`Positioner z-10070 sideOffset=6 collisionPadding=8` | `windows/tauri/src/ui/select.tsx:47-48,66-76,96-103,201,219-222,237-255,480,489-490` |
| `Select`（`searchable`） | 走 Combobox 分支（`ComboboxPrimitive.Trigger` + `@/ui/combobox`） | `select.tsx:1,6-13,287,378-474` |
| `Combobox` | 全套 Base UI Combobox 封装（含 chips 多选）；`size = xs/sm/md`（默认 `sm`）、`variant = default/ghost`；padding 变体 12 种（见 `combobox.tsx:29-64`）；Item `min-h-7 rounded-lg px-2 py-1.5`，`data-highlighted:bg-accent`、`data-selected:bg-selected/70`；Content `min-w-60 rounded-xl bg-surface/95 shadow-(--shadow-popover)`，`Positioner z-10040 sideOffset=6 align=start` | `windows/tauri/src/ui/combobox.tsx:10-11,13-105,163,183,216-219,238-260,411-429` |
| `NativeSelect` | 原生 `<select>`；`size = sm/default`；`h-8 rounded-lg border border-input`，sm 时 `h-7` | `windows/tauri/src/ui/native-select.tsx:7,23` |

| **gpui-kit 类型** | `gpui_component::select::{Select, SelectState, SelectEvent, SelectDelegate}`（`SelectDelegate` = `SearchableListDelegate` 别名，`gpui-component-0.6.6/src/select.rs:30,118,133`）；`gpui_component::combobox::{Combobox, ComboboxState, ComboboxEvent}`（`combobox.rs:109,128,749`）。自定义数据源需实现 `SearchableListDelegate`（`gpui-component-0.6.6/src/searchable_list/delegate.rs:54`）与 `SearchableListItem`（`:8`） |

---

### 2.5 标签 / 徽标 — `badge.tsx` / `tag` / `kbd.tsx` / `label.tsx` / `marker.tsx`

| 组件 | 要点 | 证据 |
| --- | --- | --- |
| `Badge` | 纯 `<span>`，无原语库、无 `data-slot`；基类 `inline-flex h-6 items-center rounded-full leading-none`；`variant = default / muted / accent / success / warning / error`；size `default px-2 py-0.5` / `compact px-1.5 py-0.5` | `windows/tauri/src/ui/badge.tsx:5-27,31` |
| `Label` | `<label data-slot="label">`；`ui-text-sm font-medium leading-none text-foreground`；peer-disabled 组合态 | `windows/tauri/src/ui/label.tsx:6-9` |
| `Kbd` | `<kbd data-slot="kbd">`；`h-5 min-w-5 rounded-sm bg-accent px-1 ui-text-sm font-medium text-subtle-foreground`；`KbdGroup gap-1` | `windows/tauri/src/ui/kbd.tsx:6-9,20-21` |
| `Marker` | 时间/事件分隔行：`variant = default / separator / border`，`tone = default / accent / error / success / warning`；`separator` 用 `before:`/`after:` 伪元素画两侧横线 | `windows/tauri/src/ui/marker.tsx:6-29,41-52,79` |

| **gpui-kit 类型** | `gpui_component::badge::Badge`（`badge.rs:31`）——**注意 gpui 的 `Badge` 是「数字角标」语义**（红点计数），Lithe 的 Badge 是「状态胶囊」。状态胶囊应映射到 **`gpui_component::tag::{Tag, TagVariant}`**（`tag.rs:10,125`），`TagVariant = Primary / Secondary(default) / Danger / Success / Warning / Info / Color(ColorName) / Custom{color,foreground,border}`（`tag.rs:10-24`）。`Label` → `gpui_component::label::Label`（`label.rs:53`）；`Kbd` → `gpui_component::kbd::Kbd`（`kbd.rs:11`）；`Marker` → `gpui_component::marker::{Marker, MarkerVariant, MarkerLoadingStyle}`（`marker.rs:14,26,48`），`MarkerVariant = Plain/Separator/Border`（`:14-22`） |

---

### 2.6 页签 / 标签栏 — `tabs.tsx` / `tab-bar.tsx`

| 组件 | 要点 | 证据 |
| --- | --- | --- |
| `Tabs` / `TabsList` / `TabsTrigger` / `TabsContent` | Base UI Tabs；`TabsList variant = default / segmented / line / bare`（默认 `default`，`gap-(--lithe-chrome-gap-tight) rounded-(--lithe-chrome-radius) bg-surface/55 p-0.5`）；`TabsTrigger size = xs / sm / md` → `xs: ui-text-chrome h-(--lithe-chrome-control-height) px-2`、`sm: ui-text-chrome h-(--lithe-tab-height) px-2.5`、`md: min-h-8 px-3 ui-text-base`；激活 `data-active:bg-accent/80 data-active:text-foreground` | `windows/tauri/src/ui/tabs.tsx:5,11,20-36,46,54-67,88,94` |
| `TabBarSurface` | Lithe 专有编辑器标签栏外壳；`orientation = horizontal / vertical`；horizontal = `h-(--lithe-tab-bar-height) border-b bg-tab-bar px-(--lithe-chrome-padding-inline)` | `windows/tauri/src/ui/tab-bar.tsx:244-255,284-297` |
| `TabBarTab` | `h-(--lithe-tab-height) min-w-20 max-w-(--lithe-tab-max-width) pl-2 pr-6`（horizontal） | `tab-bar.tsx:257-267,269-282` |
| `Tab`（sortable） | `tabVariants`：基类 `group/tab ui-text-chrome min-h-(--lithe-chrome-control-height) rounded-(--lithe-chrome-radius) px-2 …`；`variant = default / connected`；`active` 有 `connected+active` 的 **IntelliJ 3px 主色强调线** `before:h-[3px] before:bg-primary`；`dragged.true = opacity-40` | `tab-bar.tsx:169-218,220-242` |
| 拖拽 | `@dnd-kit`：`PointerSensor activationConstraint.distance = 5`；碰撞检测 `pointerWithin → closestCenter`；`MeasuringStrategy.Always`；位移 `duration: 180, easing: var(--app-ease-smooth)`；`useTabDragClickGuard` 用 rAF 抑制拖后误点 | `tab-bar.tsx:12-14,20-23,31-43,71-79,110-152` |

| **gpui-kit 类型** | `gpui_component::tab::{Tab, TabBar, TabVariant}`（`gpui-component-0.6.6/src/tab/mod.rs:4-5`）；`Tab::new()`（`tab/tab.rs:478`）；`TabBar::new(id)`（`tab/tab_bar.rs:59`）、`.with_variant`（`:80`）、`.pill`（`:86`）。`TabVariant = Tab(default) / Outline / Pill / Segmented / Underline`（`tab/tab.rs:14-21`） |

**gpui Tab 高度表（`TabVariant::height(size)`，`tab/tab.rs:24-43`）**：

| Size | Underline | 其它变体（Tab/Outline/Pill/Segmented） |
| --- | --- | --- |
| `XSmall` | `px(26.)` | `px(20.)` |
| `Small` | `px(30.)` | `px(24.)` |
| `Medium` | `px(36.)` | `px(32.)` |
| `Large` | `px(44.)` | `px(36.)` |

内层高度另有一套（`tab/tab.rs:45-69`，Medium 档：Tab 30 / Outline·Pill 26 / Segmented 24 / Underline 26）；水平内边距按 size = 8/10/16/12px，Underline 强制 0（`:72-89`）；指示条 2px（`tab/tab_bar.rs:274`）。
`TabBar` 的默认 gap 按 size：`Small/XSmall 8px`、`Large 16px`、其余 `12px`（`tab/tab_bar.rs:361-365`）；`TabVariant::Segmented` 的 `padding_x` 按 size = 2/3/4px（`:380-384`），`Underline` 的 gap 按 size = 10/12/20/16px（`:395-400`）。

**映射差距**：
- Lithe 需要的是 **IntelliJ 式 connected tab + 底部 3px 强调线**，gpui 的 `Underline` 最接近但线宽/位置不同 → **需要自定义绘制**。
- gpui 无拖拽排序组件 → **标签拖拽必须自研**（见 §4.3）。

---

### 2.7 列表行 — `item.tsx` / `sidebar.tsx` / `dropdown.tsx` 的 item

| 组件 | 要点 | 证据 |
| --- | --- | --- |
| `Item` | 列表条目版式（媒体+内容+操作）；`useRender` + `mergeProps`；`variant = default / outline / muted`，`size = default / sm / xs` → `default: gap-2.5 px-3 py-2.5`、`sm: gap-2 px-2.5 py-2`、`xs: gap-2 px-2 py-1.5`；`rounded-lg border font-sans ui-text-sm` | `windows/tauri/src/ui/item.tsx:33-53,55-72` |
| `ItemMedia` | `variant = default / icon / image`；image 时 `size-10`（sm `size-8`、xs `size-6`）`rounded-md object-cover` | `item.tsx:74-88` |
| `ItemTitle` / `ItemDescription` | title `line-clamp-1 ui-text-sm font-medium leading-snug`；description `line-clamp-2 ui-text-sm text-subtle-foreground` | `item.tsx:119-143` |
| `SidebarListItem`（Lithe 专有） | `min-h-(--lithe-tab-height) gap-(--lithe-chrome-gap-loose) rounded-(--lithe-chrome-radius) px-2 py-1 text-subtle-foreground`；`active && !iconOnly → bg-selected text-foreground`；`active && iconOnly → bg-accent text-foreground`；`iconOnly → justify-center gap-0 px-0`（内容 `w-0 flex-none opacity-0`，`aria-hidden`） | `windows/tauri/src/ui/sidebar.tsx:217,240-268` |
| `SidebarListEditor` | 编辑态：`min-h-(--lithe-tab-height) bg-accent/80 px-2 py-1 text-foreground` | `sidebar.tsx:276,290` |
| `SidebarSectionHeader`（可折叠） | `h-(--lithe-tab-height) gap-1 rounded-(--lithe-chrome-radius) px-2 font-medium text-subtle-foreground`；`variant="surface"` 时 `h-8 rounded-lg bg-accent/80 px-2.5`；尾部 `CaretDown size-3`（折叠 `-rotate-90`）+ 可选 `Badge variant="muted" size="compact"` | `sidebar.tsx:308,327-347` |
| `SidebarSectionLabel`（静态） | `h-(--lithe-chrome-control-height) gap-(--lithe-chrome-gap-loose) px-2 text-subtle-foreground` | `sidebar.tsx:353,367` |
| `dropdownItemVariants` | `density = default / compact` → `default: gap-3 rounded-lg px-2.5 py-1.5`、`compact: gap-2 rounded-md px-2 py-1`；`focused.true = bg-accent` | `windows/tauri/src/ui/dropdown.tsx:34-57` |
| 文件树行（不在 `src/ui`） | `file-tree-row font-sans ui-text-chrome rounded-(--lithe-chrome-radius) gap-1.5 px-1.5 py-1 leading-row hover:bg-accent`；高度由公式给出（§1.3） | `windows/tauri/src/features/sidebar/components/sidebar-tree.tsx:241`、`file-tree-row.ts:7-14` |

| **gpui-kit 类型** | `gpui_component::list::{List, ListState, ListItem, ListDelegate, ListEvent}`（`gpui-component-0.6.6/src/list/list.rs:70,721`、`list/list_item.rs:26`、`list/delegate.rs:10`）。`ListDelegate` 必实现 `items_count(&self, section, cx) -> usize`（`delegate.rs:35`）与 `render_item(...)`（`:42`）；可选 `sections_count`、`render_section_header/footer`、`render_empty`、`render_initial`、`loading`、`load_more` 等（`:27-170`）；`set_selected_index` / `set_right_clicked_index`（`:119,127`）。列表项尺寸 `StyleSized::list_size` → `list_px` `Small px_2 / 其余 px_3`，`list_py` `Large py_2 / Medium py_1 / Small py_0p5`（`sizing.rs:272-292`） |

---

### 2.8 表格 — `table.tsx`

| 项 | 内容 |
| --- | --- |
| 用途 | 原生 HTML 表格样式封装（无虚拟化） |
| 结构 | 8 个原生元素 + `data-slot`：`table` / `table-header` / `table-body` / `table-footer` / `table-row` / `table-head` / `table-cell` / `table-caption`（`windows/tauri/src/ui/table.tsx:7-93`） |
| 尺寸 | 根 `w-full border-collapse ui-text-sm`；Header `sticky top-0 z-10 border-b bg-background`；Row `border-b hover:bg-accent data-[state=selected]:bg-selected`；Head `h-8 px-1.5 text-left font-medium text-subtle-foreground whitespace-nowrap`；Cell `px-1.5 py-1.5`；Footer `border-t bg-surface/55 font-medium` |
| 状态 | `hover:bg-accent`、`data-[state=selected]:bg-selected`（`table.tsx:52,65,77`） |
| **gpui-kit 类型** | **两套，别选错**：<br>① **静态** `gpui_component::table::{Table, TableHeader, TableBody, TableRow, TableHead, TableCell, TableCaption, TableFooter}`（`gpui-component-0.6.6/src/table/table.rs:41,143,215,283,356,428,519,610`），`Table::new()`（`:50`），文档明确「无虚拟滚动、无列管理」（`:16-39`），`MIN_CELL_WIDTH = px(100.)`（`:14`）；<br>② **数据表** `DataTable<D: TableDelegate>`（`table/data_table.rs:91`），`new(&Entity<TableState<D>>)`（`:101`）、`.stripe(bool)`（默认 `false`，`:109`）、`.bordered(bool)`（默认 `true`，`:115`）、`.scrollbar_visible(v,h)`（`:121`）—— **支持虚拟化**：`TableState` 持 `vertical_scroll_handle: UniformListScrollHandle` + `horizontal_scroll_handle: VirtualListScrollHandle`（`table/state.rs:237-238`），行高统一用 `options.size.table_row_height()`（`:1814,1875,1976,2231,2323,2376,2385`） |
| 行高 | `Size::table_row_height()`：`XSmall 26 / Small 30 / Medium 32 / Large 40`（`gpui-component-0.6.6/src/sizing.rs:57-65`）；`Size(px)` 直接用该 px；单元格 padding `table_cell_padding()` = XS(2,4) / S(3,6) / L(8,12) / M(4,8)（`:69-96`） |
| 列定义 | `Column`（`table/column.rs:10`）：`Column::new(key, name)`（`:88`）、`.sortable/.ascending/.descending/.text_center/.text_right/.paddings/.width/.fixed_left/.min_width/.max_width`（`:107-215`）；默认 `width = px(100.)`（`:75`）、`min_width = px(20.)`（`:80`）、`max_width = f32::MAX`（`:81`）；`ColumnSort{Default,Ascending,Descending}`（`:263`）、`ColumnGroup::new(label, span)`（`:59`） |
| `TableState` 默认 | `loop_selection / col_selectable / row_selectable / sortable / col_movable / col_resizable / col_fixed = true`；`cell_selectable = false`；`row_header = true`（`table/state.rs:287-295`）；链式开关 `:315,321,327,333,339,367`；键盘上下文 `"DataTable"`（`data_table.rs:15-30`） |

**映射结论**：Lithe 表头 `h-8`=32px = gpui 静态 `Table` 的 `Size::Medium::table_row_height()`，**这一档最接近可直接用**；但 Lithe 的 `src/ui/table.tsx` 没有虚拟化，而 gpui 的静态 `Table` 也没有 → 静态表格语义一致。**数据密集的表（如 git log / 数据库结果）应改用 `DataTable` + `TableDelegate`**，这比 Windows 侧更强。

---

### 2.9 树 — 未在 `src/ui`，实现在 feature 层

| 项 | 内容 |
| --- | --- |
| 位置 | `windows/tauri/src/features/file-explorer/components/file-explorer-tree.tsx`(-item)、`windows/tauri/src/features/sidebar/components/sidebar-tree.tsx` |
| 行样式 | `[data-sidebar-tree-row]::before` 提供 hover 底（`--file-tree-hover-bg = color-mix(accent 68%, transparent)`）、`[data-active=true] → bg: var(--selected)`、`rounded: var(--lithe-chrome-radius)`（`windows/tauri/src/features/sidebar/styles/sidebar-tree.css:1-35`） |
| 行高 | `getFileTreeRowHeight(uiFontSize)` 公式（§1.3），经 `--file-tree-row-height` 下发 |
| 缩进 / 指示线 | `--file-tree-row-inline-inset: 6px`、`--file-tree-row-radius: 4px`、`--file-tree-disclosure-size: 16px`、`--file-tree-icon-size: 16px`、`--tree-guide-color: color-mix(subtle-foreground 20%, transparent)`、指示线 `width: 7px` + `::before { left:3px; width:1px }`（`windows/tauri/src/features/file-explorer/styles/file-explorer-tree.css:1-18,170-200`） |
| 选中态 | 焦点不在树上时用 `--file-tree-selected-idle-bg: var(--border)`，聚焦时用 `var(--selected)`（`file-explorer-tree.css:8,69-78`） |
| 虚拟化 | 自研：`getFileTreeTotalHeight` / 可视区间计算（`windows/tauri/src/features/file-explorer/lib/file-tree-viewport.ts:13-84`），行绝对定位 |
| 基础缩进 | `FILE_TREE_BASE_INDENT = 10`、`FILE_TREE_MIN_ROW_HEIGHT = 24`（`file-tree-row.ts:1-2`） |
| **gpui-kit 类型** | `gpui_base::tree::{Tree, TreeState, TreeItem, TreeEntry, TreeEntryState, TreeEvent}`（`gpui-base-0.6.6/src/tree.rs:41,50,93,164,184,468`）；`gpui-component-0.6.6/src/tree.rs` 只是 **类型别名 shim**（`gpui-component-0.6.6/src/tree.rs:120` 的测试名 `legacy_tree_types_are_base_types` 即证据）。**渲染 delegate 是闭包而不是 trait**：`Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem`（`gpui-component-0.6.6/src/tree.rs:18-23,41-43`）→ 行内自绘的自由度足够。右键菜单 `Tree::context_menu(f)`（`:55`）。状态操作：`TreeState::new(cx)`/`set_items`/`set_selected_index`/`entry(ix)`/`scroll_to_item`/`reveal_item`/`focus`（`gpui-base-0.6.6/src/tree.rs:197,214,225,252,260,268,280`） |

**映射差距**：gpui 的 `Tree` 提供展开/折叠、选中、滚动与右键菜单，但 **不提供 IDE 式的缩进指导线**（Lithe 的 `file-tree-guide` 是绝对定位 `::before` 竖线，`file-explorer-tree.css:170-200`）。因为 delegate 是闭包，可以在返回的 `ListItem` 里自绘：每层一条 `w(px(1.))` 的竖线，颜色 `subtle_foreground.alpha(0.2)`，横向间距按 `FILE_TREE_BASE_INDENT = 10`（`file-tree-row.ts:1`）与 `--file-tree-row-inline-inset: 6px`（`file-explorer-tree.css:6`）复刻。

---

### 2.10 卡片 — `card.tsx` / `group_box`

| 项 | 内容 |
| --- | --- |
| `cardVariants` | 基类 `flex flex-col overflow-hidden rounded-xl ui-text-sm`；`variant = default(border border-border/70 bg-surface/45) / muted(bg-surface/55) / outline(border bg-transparent) / elevated(bg-surface/65 shadow-(--shadow-card))`；`size = flush(gap-0 py-0 [--card-spacing:0rem]) / sm(gap-3 py-3 [--card-spacing:0.75rem]) / default(gap-4 py-4 [--card-spacing:1rem])`（`windows/tauri/src/ui/card.tsx:5-26`） |
| 子件 | Header `grid auto-rows-min gap-1 px-(--card-spacing) has-data-[slot=card-action]:grid-cols-[1fr_auto]`；Footer `border-t bg-surface/55 px-(--card-spacing) pt-(--card-spacing)`；内容间距全部走 `--card-spacing` 变量（`card.tsx:48-99`） |
| 复用度 | `@/ui/card` 在 `src/ui` 外仅 4 处 import |
| **gpui-kit 类型** | **未找到 `Card`**（对 `gpui-component-0.6.6` 全目录无 `pub struct Card`）。最接近的是 `gpui_component::group_box::{GroupBox, GroupBoxVariant}`，`GroupBoxVariant = Normal(default) / Fill / Outline`（`gpui-component-0.6.6/src/group_box.rs:11-16,62`）。另有 `gpui_component::accordion` 的 `accordion` 色 token（`theme_color.rs:65`） |

---

### 2.11 对话框 — `dialog.tsx` / `alert-dialog.tsx` / `sheet.tsx` / `drawer.tsx`

| 组件 | 要点 | 证据 |
| --- | --- | --- |
| `DialogContent` | `size = sm/md/lg`（默认 `md`）；`w-full max-w-sm/md/lg`；基类 `-translate-x-1/2 -translate-y-1/2 fixed top-1/2 left-1/2 z-9999 max-h-[90vh] rounded-xl border bg-background shadow-(--shadow-dialog)`；进出场 `scale-95 + opacity-0` | `windows/tauri/src/ui/dialog.tsx:43,56-74,112,121-124` |
| `AppDialog`（应用级） | header `px-4 py-3` + 标题 `ui-text-base font-medium` + 关闭按钮 `size-6`；内容 = `ScrollArea className="flex-1"` + `contentClassName="p-4"`；footer `justify-end gap-2 px-4 py-3` | `dialog.tsx:258-290` |
| 命令式服务 | `showAlertDialog` / `showConfirmDialog` / `showChoiceDialog` / `showChoiceDialogWithCheckbox` / `showPromptDialog` + 队列 `DialogServiceProvider` | `dialog.tsx:400,409,426,442,461,480` |
| `AlertDialogContent` | `size = default/sm` → `max-w-sm` / `data-[size=sm]:max-w-xs`；`grid gap-4 rounded-xl bg-background p-4 shadow-(--shadow-dialog)`；媒体块 `size-9 rounded-lg bg-accent` | `windows/tauri/src/ui/alert-dialog.tsx:33-43,83` |
| `SheetContent` | `side = top/right/bottom/left`（默认 `right`）；左右 `w-3/4 sm:max-w-sm`；上下 `h-auto border-t/b`；进出场 `translate-10` | `windows/tauri/src/ui/sheet.tsx:41-57` |
| `DrawerContent` | `swipeDirection = down/up/left/right`（默认 `down`）+ `snapPoints`；`m-(--drawer-inset,0px)`、`rounded-t-xl` + 边框；`duration-450` | `windows/tauri/src/ui/drawer.tsx:27-33,110-118,195-207` |
| Overlay | dialog/alert-dialog：`bg-black/20 z-9998`；sheet：`bg-black/10 z-50 backdrop-blur-xs` | `dialog.tsx:97`、`alert-dialog.tsx:23`、`sheet.tsx:30` |

| **gpui-kit 类型** | `gpui_component::dialog::Dialog`（`gpui-component-0.6.6/src/dialog/dialog.rs:258`）+ `DialogHeader`/`DialogFooter`/`DialogClose`/`DialogAction`（`dialog/header.rs:17`、`footer.rs:23,71,104`）等。`Dialog::new(cx)`（`dialog/dialog.rs:287`），`.title`（`:323`）/`.content`（`:314`）/`.footer`（`:339`）/`.width(impl Into<Pixels>)`（`:413`）/`.margin_top`（`:395`）/`.overlay`（`:425`）/`.overlay_closable`（`:433`）/`.keyboard`（`:439`）/`.close_button`（`:389`）；**默认宽 `px(448.)`**（`:177`）。`gpui_component::sheet::{Sheet, SheetSettings}`（`sheet.rs:27,42`），`Sheet::new(window, cx)`（`:59`）、`.title`（`:77`）/`.footer`（`:83`）/`.size(DefiniteLength)`（`:89`）/`.resizable`（`:95`）/`.overlay`（`:101`）/`.overlay_closable`（`:107`）/`.on_close`（`:113`）；**默认 `Placement::Right` + 350px**（`:62-63`），四位置渲染（`:184-189`），`SheetSettings.margin_top = TITLE_BAR_HEIGHT`（`:27-38`）。打开方式 `gpui_component::window_ext::WindowExt`（`window_ext.rs:12`，`impl WindowExt for Window` `:109`）：`open_dialog`（`:30`）、`open_alert_dialog`（`:51`）、`has_active_dialog`（`:56`）、`close_dialog`（`:59`，只关最后一个）、`close_all_dialogs`（`:62`）、`open_sheet`（`:14`，≡ `open_sheet_at(Placement::Right, …)`）、`open_sheet_at`（`:19`）、`has_active_sheet`（`:24`）、`close_sheet`（`:27`） |

**`Placement` 已确认是四边枚举**：`Placement{Top, Bottom, Left, Right}`（`gpui-base-0.6.6/src/geometry.rs:12-21`，辅助 `is_horizontal` `:36` / `is_vertical` `:44` / `axis` `:52`）→ **可以直接映射 Lithe `sheet.tsx` 的 `side = top/right/bottom/left`**（`windows/tauri/src/ui/sheet.tsx:41,45`）。

**映射差距**：**没有 Drawer**（`gpui-component` 无 drawer 模块）。`AlertDialog` **没有严重度方法**（`dialog/alert_dialog.rs:79-260` 的全部 `pub fn` 中无 `info/warning/success/error`；`window_ext.rs:44-49` 的 `alert.warning()` 文档示例是**过时**的）→ 语义色改用 `alert::Alert` + `AlertVariant`（`alert.rs:16`）。Lithe 的 `drawer.tsx` 使用了 Base UI 的 snapPoints / 嵌套堆叠 / 滑动手势，这些在 gpui 中**全部要自研**。

---

### 2.12 弹出菜单 — `dropdown.tsx` / `dropdown-menu.tsx` / `context-menu.tsx` / `menubar.tsx` / `popover.tsx`

| 组件 | 要点 | 证据 |
| --- | --- | --- |
| `Dropdown`（Lithe 专有，40 处复用） | **不使用 Base UI Positioner**，手写视口碰撞定位 + `FloatingPopoverContent`；`density = default/compact`；`VIEWPORT_PADDING = 8`、`RESIZE_REPOSITION_THRESHOLD = 2`、`maxHeight ≥ 120`；`mousedown` 外部关闭、捕获阶段 Escape 关闭并 `stopImmediatePropagation`；`ResizeObserver` 重定位；键盘 `ArrowDown/Up/Home/End/Enter` | `windows/tauri/src/ui/dropdown.tsx:24-30,34-57,287-288,303-314,402-470,522-626,643` |
| `DropdownMenu*`（15 个） | Base UI `Menu`；Content 默认 `align=end side=bottom sideOffset=4 collisionPadding=8`，`z-10070 min-w-44 rounded-md bg-surface p-1 shadow-(--shadow-popover) ring-1 ring-border/70 duration-100`；Item `gap-2 rounded-sm px-2 py-1.5 focus:bg-accent`，`variant?: default\|destructive` | `windows/tauri/src/ui/dropdown.tsx:713-939` |
| `dropdown-menu.tsx`（15 个） | shadcn 风格声明式菜单；Content `z-50 w-(--anchor-width) min-w-32 rounded-lg bg-popover p-1 shadow-md ring-1 ring-foreground/10`；Item `gap-1.5 rounded-md px-1.5 py-1 ui-text-chrome`；Label `px-1.5 py-1 text-xs`；SubContent `min-w-24` | `windows/tauri/src/ui/dropdown-menu.tsx:22-25,42,68,91,139,242-258` |
| `context-menu.tsx`（15 个） | 右键菜单；Content `z-10070 min-w-36 rounded-md bg-surface p-1 ui-text-chrome`，默认 `align=start alignOffset=4 side=right sideOffset=0`；Item `gap-2 rounded-sm px-2 py-1 focus:bg-accent` | `windows/tauri/src/ui/context-menu.tsx:26-45,89,215-231` |
| `menubar.tsx`（12 个） | 根 `h-6 rounded-full border border-border/70 bg-background/65 px-0.5 py-0.5`；Trigger `h-5 rounded-md px-1.5`+`openOnHover`；Content `z-10031 min-w-60 max-w-[min(480px,…)] rounded-xl bg-surface/95`；Item `min-h-7 gap-6 rounded-lg px-2.5 py-1.5`；Shortcut 用等宽字体 | `windows/tauri/src/ui/menubar.tsx:36,84-88,106-127,147,157,176-179` |
| `PopoverContent` | 默认 `align=center side=bottom sideOffset=6 collisionPadding=8`；`z-10070 w-72 rounded-xl border bg-surface/95 p-2.5 shadow-(--shadow-popover) backdrop-blur-sm` | `windows/tauri/src/ui/popover.tsx:110-114,139` |
| `FloatingPopoverContent` | `createPortal` + `motion/react` `AnimatePresence`；`fixed z-10070 min-w-60 max-w-[min(480px,calc(100vw-16px))] p-1`；`data-prevent-dialog-escape`、`onWheelCapture=containScrollChain` | `popover.tsx:14-16,18-45,61,80-97` |

**层级常量（z-index 阶梯）**：`z-50`（sheet/drawer/dropdown-menu）→ `z-10031`（menubar）→ `z-10040`（combobox）→ `z-10050`（menubar sub）→ `z-10060`（command 遮罩）→ `z-10070`（select/context-menu/popover/hover-card）→ `z-9998/9999`（dialog overlay/content）→ `z-99999`（tooltip）。

| **gpui-kit 类型** | `gpui_component::menu::{DropdownMenu, ContextMenu, ContextMenuExt, ContextMenuState, PopupMenu, PopupMenuItem, AppMenuBar}`（`gpui-component-0.6.6/src/menu/mod.rs:9-12`）；`gpui_component::popover::{Popover, PopoverState}`（`popover.rs:16,106`）；`gpui_base::PopoverState`（同处再导出） |

**gpui 侧调用方式与硬约束**：
- 右键菜单：`element.context_menu(|menu, window, cx| …)`（trait `ContextMenuExt`，`menu/context_menu.rs:13`，方法 `:19-36`）；`ContextMenu::new(id, element)`（`:53`）。
- 下拉菜单：`button.dropdown_menu(f)`（`menu/dropdown_menu.rs:12,14`）、`.dropdown_menu_with_anchor(anchor, f)`（`:22`）、`impl DropdownMenu for Button`（`:34`）。
- ⚠️ **`PopupMenu::new` 是 `pub(crate)`**（`menu/popup_menu.rs:334`）→ 外部**只能**通过上面两个回调拿到菜单对象，不能自己 `new`。构建 API：`.item`（`:728`）/`.separator`（`:680`）/`.label`（`:492`）/`.menu`（`:465`）/`.link`（`:498`）/`.submenu`（`:694`）/`.min_w`（`:429`）/`.max_w`（`:435`）/`.max_h`（`:441`）/`.scrollable`（`:447`）/`.check_side(Side)`（`:453`）；`PopupMenuItem::new/element/submenu/separator/label/link`（`:71,85,102,113,119,214`）。
- `AppMenuBar::new(cx)`（`menu/app_menu_bar.rs:34`）、`.reload`（`:47`）；`NativeMenu`（`native_menu/mod.rs:72`）。
- Popover 默认定位 `placement(Bottom) + align(Start) + offset(px(6.)) + margin(px(8.))`（`popover.rs:33-39`），入场 **150ms**（`:24`），位移 **8px**（`:31`）；`dropdown_popup`（`:82-102`）。
- **弹层统一外观直接用现成的 `ThemeStyled::popover_style(cx)`**（`gpui-component-0.6.6/src/styled.rs:151`，实现 `:193-201` = `bg(popover)` + `text(popover_foreground)` + `popover_shadow` + `rounded(theme.radius)`，**无 border**）；同处还有 `focus_ring_style`（`:142`）、`SURFACE_SHADOW_INK = 0.1`（`:16`）、`POPOVER_RING_INK = 0.1`（`:26`）、`FOCUS_RING_WIDTH = px(3.)`（`:11`）、`FOCUS_RING_OPACITY = 0.5`（`:12`）—— Lithe 的 `focus-visible:ring-2 ring-primary/20` 与之对不上，需要决定接受 3px 还是自绘。

---

### 2.13 Toast / 通知 — `toast.tsx` / `sonner.tsx` + feature 层通知中心

| 项 | 内容 |
| --- | --- |
| 实际在用的实现 | **`sonner`**（外部库），由 `windows/tauri/src/ui/sonner.tsx` 主题化。`src/ui/toast.tsx`（Base UI Toast）在 `src/ui` 外 **0 引用** |
| 位置与行为 | `position="bottom-right"`、`expand`；主题跟随 `data-theme-type`（`MutationObserver`） |
| 尺寸 | toast `rounded-xl border-border bg-background shadow-(--shadow-popover) backdrop-blur-sm`；标题/描述 `ui-text-sm leading-5`；关闭按钮 `size-4.5 opacity-0 group-hover:opacity-100` |
| 图标 | success/info/warning/error = 18px；close = 14px；loading = `ThinkingOrb state="working" size={20}` |
| 业务封装 | `windows/tauri/src/features/layout/contexts/toast-context.tsx`：`showToast / updateToast / dismissToast / hasToast`，类型 `success / warning / error / info`（默认 info），并派发 `toast-dismissed` 自定义事件 |
| 通知中心 | `windows/tauri/src/features/notifications/`：`NotificationRecorder`（用 `useSonner` 录制）、`notifications-tool-window.tsx`（分组、过滤 all/info/success/warning/error、搜索、上下文菜单、详情面板）—— 这是一个**独立于 toast 的持久化通知列表** |
| Base UI Toast（备用） | `createToastManager()`，Viewport `fixed inset-x-4 bottom-4 max-w-sm`；Toast `rounded-2xl border bg-popover shadow-lg`，堆叠变量 `[--gap:.75rem] [--peek:.75rem] [--scale:calc(max(0,1-(index*0.1)))]`；Content `p-4 gap-3` |

| **gpui-kit 类型** | `gpui_component::notification::{Notification, NotificationType, NotificationDelivery, NotificationSettings, NotificationList}`（`gpui-component-0.6.6/src/notification.rs:31,53,107,530,693`）。`Notification::new()`（`:167`）、快捷构造 `info/success/warning/error(msg)`（`:196,203,210,217`）、`.message`（`:190`）/`.title`（`:243`）/`.icon`（`:251`）/`.with_type`（`:257`）/`.placement(Anchor)`（`:266`）/`.delivery`（`:290`）/`.system`（`:299`）/`.in_app_and_system`（`:309`）/`.autohide`（`:314`）/`.on_click`（`:320`）/`.on_close`（`:332`）/`.action`（`:340`）/`.id::<T>`（`:229`）/`.id1::<T>(key)`（`:235`）/`.dismiss`（`:350`）。`NotificationType = Info(default) / Success / Warning / Error`（`:31-37`）；`NotificationDelivery = InApp(default) / System / InAppAndSystem`（`:53-61`）。打开方式：`window.push_notification(note, cx)`（`window_ext.rs:65`）、`remove_notification` / `remove_notification1` / `clear_notifications` / `notifications`（`:69,72,75,78`） |

**关键尺寸/行为差异（必须显式覆盖）**：
- gpui 通知**位置枚举是 `gpui::Anchor`，默认 `Anchor::TopRight`**（`notification.rs:551`），而 Lithe toast 是 **`position="bottom-right"`**（`windows/tauri/src/ui/sonner.tsx:37`）→ 必须 `.placement(Anchor::BottomRight)`。
- `NotificationSettings`（`:530-545`）：margins `px(16.)` 且 top 额外加 `TITLE_BAR_HEIGHT`（`:549-557`）、`max_items = 10`（`:558`）、**width `px(382.)`**（常量 `:26`，字段 `:540`）。Lithe toast 宽度为 `max-w-sm`(384px)（`windows/tauri/src/ui/toast.tsx:33`）→ 几乎一致，可直接用。
- gpui 的 `NotificationDelivery::System` **天然覆盖**了 Lithe 目前没有的能力（OS 通知中心）。
- 反过来 **Lithe 的通知中心工具窗（持久列表 + 分组 + 过滤 + 搜索 + 详情）在 gpui 中未找到**，需自研（`NotificationList`（`:693`）只提供 `push`（`:798`）/`clear`（`:952`）/`notifications`（`:965`））。

---

### 2.14 进度 / 骨架 / 加载 — `progress.tsx` / `skeleton.tsx` / `spinner.tsx`

| 组件 | 要点 | 证据 |
| --- | --- | --- |
| `Progress` | `size = sm(h-1) / md(h-1.5)`，默认 `sm`；track `rounded-full bg-surface`；indicator `bg-primary transition-[width] duration-(--app-duration-normal)`；`ProgressValue` `ml-auto tabular-nums text-subtle-foreground` | `windows/tauri/src/ui/progress.tsx:5-18,32,50-90` |
| `Skeleton` | 单行：`animate-pulse rounded-md bg-muted`（`data-slot="skeleton"`）；**无变体** | `windows/tauri/src/ui/skeleton.tsx:6-13` |
| `Spinner` | `animate-spin rounded-full border-2 border-current border-r-transparent`；`compact ? size-3 : size-4`；`showLabel` 时 `gap-2 ui-text-sm text-subtle-foreground` + `role=status aria-live=polite` | `windows/tauri/src/ui/spinner.tsx:8,22-44` |
| 文本 shimmer（AI 场景） | `.ui-text-shimmer`：渐变背景裁字 + `textShimmer 2s linear infinite` | `windows/tauri/src/styles/utilities.css:84-104` |

| **gpui-kit 类型** | `gpui_component::progress::{Progress, ProgressCircle}`（`gpui-component-0.6.6/src/progress/mod.rs:4-5`）—— **注意：模块内没有 `pub struct Progress` 的顶层定义**，是 re-export 自子模块；`gpui_component::skeleton::Skeleton`（`skeleton.rs:10`）；`gpui_component::spinner::Spinner`（`spinner.rs:10`）；`gpui_component::shimmer::{ShimmerStyle, ShimmerText, ShimmerSpread}`（`shimmer.rs:20,51,139`）—— **shimmer 有现成实现，可直接用** |

---

### 2.15 空状态 — `empty.tsx`

| 项 | 内容 |
| --- | --- |
| 导出 | `Empty` / `EmptyHeader` / `EmptyMedia` / `EmptyTitle` / `EmptyDescription` / `EmptyContent` / `EmptyState`（`windows/tauri/src/ui/empty.tsx:132`） |
| 用途 | 空状态占位（列表/面板无数据、错误、警告、成功） |
| `tone` | `neutral(default) / error / warning / success`，通过 `data-tone` + `group-data-[tone=…]/empty:*` 着色 |
| `EmptyMedia` | `variant = default(bg-transparent) / icon(size-8 rounded-lg bg-accent [&_svg]:size-4)` |
| 尺寸 | 根 `flex-1 flex-col items-center justify-center gap-2 rounded-lg border-dashed p-3 text-center`；Header `max-w-sm gap-2`；Title `ui-text-base font-medium tracking-tight`；Description `ui-text-sm leading-relaxed text-subtle-foreground`；Content `max-w-sm gap-2.5` |
| 便捷组合 | `EmptyState({ message, action })` = `Empty > EmptyDescription (+ EmptyContent > Button size="xs")` |
| 复用度 | **41 处** import（`@/ui/empty`），是全站最高频的「非表单」组件之一 |

| **gpui-kit 类型** | `gpui_component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription, EmptyContent}`（`gpui-component-0.6.6/src/empty.rs:14,89,167,231,279,328`）+ `EmptyMediaVariant = Default / Icon`（`:157-164`） |

**映射差距**：gpui 的 Empty 无 `tone` 概念（Lithe 有 4 档语义色）→ 需要在调用侧套 `AlertVariant` 式的着色或自建。

---

### 2.16 其余高频组件速查

| 组件 | 关键点 | 证据 | gpui-kit 类型 |
| --- | --- | --- | --- |
| `Tooltip` | `delay=150 timeout=100 closeDelay=0`；内容 `ui-text-caption rounded-lg border border-border/70 bg-surface/95 px-2 py-1 shadow-(--shadow-popover) backdrop-blur-sm`；`sideOffset=6 collisionPadding=8`；`z-99999` | `windows/tauri/src/ui/tooltip.tsx:16-22,44-48` | `gpui_component::tooltip::Tooltip`（`tooltip.rs:34`） |
| `Checkbox` | 固定 `size-4 rounded-md border border-border bg-surface`；勾 `size-3.5 strokeWidth=3`；`data-checked:border-primary data-checked:bg-primary data-checked:text-white`；热区 `after:-inset-x-3 after:-inset-y-2` | `windows/tauri/src/ui/checkbox.tsx:8-19` | `gpui_component::checkbox::Checkbox`（`checkbox.rs:17`） |
| `RadioGroup` | `grid gap-2`；item `size-4 rounded-full border border-input`；指示点 `size-2 bg-primary-foreground` | `windows/tauri/src/ui/radio-group.tsx:11-32` | `gpui_component::radio::{Radio, RadioGroup}`（`radio.rs:19,280`） |
| `Switch` | `size = sm(h-3.5 w-7) / md(h-5 w-9)`，默认 `md`；thumb `sm size-2.5 translate-x-3.5` / `md size-4 translate-x-4`；`data-checked:bg-primary` | `windows/tauri/src/ui/switch.tsx:13-40` | `gpui_component::switch::Switch`（`switch.rs:14`） |
| `Slider` | track `h-1 rounded-full bg-muted`；thumb `size-3 rounded-full border border-ring bg-white`，热区 `after:-inset-2`；`thumbAlignment="edge"` | `windows/tauri/src/ui/slider.tsx:13-46` | `gpui_component::slider::Slider`（`slider.rs:86`） |
| `Toggle` | `variant = default/outline`，`size = xs(min-h-6) / sm(min-h-7) / md(min-h-8)`，默认 `sm`；`data-pressed:bg-selected` | `windows/tauri/src/ui/toggle.tsx:6-25` | `gpui_component::button::{Toggle, ToggleVariant}`（`button/toggle.rs:14-18`）`ToggleVariant = Ghost(default) / Outline` |
| `ToggleGroup` | `variant = default(gap-1 p-1) / segmented(gap-0 p-0)`；`wrap` 默认 `true`；item `size = xs/sm/md`，默认 `xs`；`iconOnly` → `aspect-square px-0` | `windows/tauri/src/ui/toggle-group.tsx:36-93` | `gpui_base::toggle_group`（`gpui-base-0.6.6/src/toggle_group.rs`），`gpui-component` 无独立模块 |
| `ButtonGroup` | `orientation = horizontal/vertical`；`variant = default(bg-accent) / accent`；自动去相邻圆角与边框 | `windows/tauri/src/ui/button-group.tsx:6-28,79` | **未找到**直接对应（可用 H 布局 + `ButtonRounded` 逐边控制，`button/button.rs:20`） |
| `Separator` | `bg-border`，horizontal `h-px w-full` / vertical `h-full w-px` | `windows/tauri/src/ui/separator.tsx:4-10` | `gpui_component::separator::{Separator, SeparatorStyle}`（`separator.rs:9,17`），`SeparatorStyle = Solid/Dashed` |
| `ScrollArea` | Root `relative min-h-0 overflow-hidden`；Scrollbar `absolute z-10 opacity-0 group-hover:opacity-100 data-scrolling:opacity-100`，vertical `w-2.5`，thumb `w-1.5`；`reserveScrollbarGutter` 追加 `pr-2.5`/`pb-2.5` | `windows/tauri/src/ui/scroll-area.tsx:10-29,65-134` | `gpui_component::scroll::{Scrollable, ScrollableElement, ScrollableMask}`（`scroll/scrollable.rs:16,67`） |
| `Field` | 表单布局族 10 个导出；`orientation = vertical(default) / horizontal / responsive`；Set/Group `gap-4`；Legend `mb-1.5`，`variant = legend/label`；Error 用 `role=alert` | `windows/tauri/src/ui/field.tsx:51-64,10-43,24-30,141-143,183-193` | `gpui_component::form::{Form, Field, FieldBuilder}`（`form/form.rs:14`、`form/field.rs:33,81`） |
| `Alert` | `tone = default / info / success / warning / error`；`grid gap-0.5 rounded-lg border px-2.5 py-2 ui-text-sm`；`has-[>svg]:grid-cols-[auto_1fr]`；Action `absolute top-1.5 right-1.5` | `windows/tauri/src/ui/alert.tsx:5-21,44-67` | `gpui_component::alert::{Alert, AlertVariant}`（`alert.rs:16,59`），`AlertVariant = Default/Info/Success/Warning/Error`（`:16-24`） |
| `Avatar` | `rounded-full bg-surface`，**尺寸由调用方 `className` 决定**；Fallback `ui-text-sm font-medium text-subtle-foreground`，`delay = 图片存在时 150ms` | `windows/tauri/src/ui/avatar.tsx:19-43` | `gpui_component::avatar::Avatar`（`avatar/avatar.rs:17`）+ `avatar_group`；`gpui_component::rating` 亦存在 |
| `Accordion` | Trigger `py-2.5 text-sm font-medium`，`aria-expanded` 切换上下 chevron；Content `data-open:animate-accordion-down`，内层 `h-(--accordion-panel-height)` | `windows/tauri/src/ui/accordion.tsx:28-60` | `gpui_component::accordion::{Accordion, AccordionItem}`（`accordion.rs:18,160`） |
| `Collapsible` | 最小容器，**不注入任何 className** | `windows/tauri/src/ui/collapsible.tsx:4-15` | `gpui_component::collapsible::Collapsible`（`collapsible.rs:11`） |
| `Breadcrumb` | List `gap-1.5 ui-text-sm text-subtle-foreground`；Page `font-medium text-foreground` + `aria-current=page`；Separator `[&>svg]:size-3.5` | `windows/tauri/src/ui/breadcrumb.tsx:17-94` | `gpui_component::breadcrumb::{Breadcrumb, BreadcrumbItem}`（`breadcrumb.rs:13,20`） |
| `Pagination` | Link 默认 `size="icon"`、`variant={active ? "default" : "ghost"}`；Previous/Next `size="default"` | `windows/tauri/src/ui/pagination.tsx:38-109` | `gpui_component::pagination::Pagination`（`pagination.rs:21`） |
| `Resizable` | `react-resizable-panels`；Handle `w-px bg-border` + `after:w-1` 热区；`withHandle` → `h-6 w-1 rounded-lg bg-border` | `windows/tauri/src/ui/resizable.tsx:1,8-35` | `gpui_component::resizable`（`lib.rs:68`；`gpui-base-0.6.6/src/resizable/{mod,panel,resize_handle}.rs`）；把手颜色由 `Theme::base_theme()` 的 `resizable.handle = border`、`active_handle = drag_border` 决定（`gpui-component-0.6.6/src/theme/mod.rs:342-345`） |
| `Command` | Lithe 自研命令面板（**不用 cmdk**）：`CommandItem density = default(ui-text-base min-h-8 rounded-lg px-2.5 py-2) / compact(ui-text-sm min-h-7 rounded-md px-2 py-1)`；面板 `max-h-[min(68vh,32rem)] w-[min(44rem,calc(100vw-2rem))] rounded-xl bg-background shadow-(--shadow-dialog)`；遮罩 `pt-16 z-10060`；列表用 `ScrollArea`，Tab 用 `Tabs variant="bare"` | `windows/tauri/src/ui/command.tsx:1,3,28-56,143,240-250,394,488-503,732` | **未找到**同构组件；`gpui_component::command::{Command, CommandState, CommandEntry, CommandGroup, CommandItem}`（`command/mod.rs:11-13`）是 gpui 的版本，API 与交互需重新对齐 |
| `Sidebar`（Lithe 专有 14 导出） | 全部尺寸走 `--lithe-*` 变量，**无 `size` prop**；`SidebarPanel` `flex h-full flex-col bg-background`；`SidebarTitleBar` `h-(--lithe-pane-header-height) px-3`，标题 `ui-text-lg font-medium`；`SidebarToolbar` `border-b`；`SidebarHeader` `sticky top-0 z-20 h-(--lithe-sidebar-header-height)`；`SidebarFooter` `mx-2 mb-2 rounded-xl border border-border/60`；`SidebarTabBar` 用 `TabsTrigger size="xs"` | `windows/tauri/src/ui/sidebar.tsx:11-449` | `gpui_component::sidebar::{Sidebar, SidebarMenu, SidebarMenuItem, SidebarToggleButton, SidebarCollapsible}`（`sidebar/mod.rs:20,22,94,211,222,302`），`SidebarCollapsible = Icon(default) / Offcanvas / None`（`:38-45`） |
| `Chrome`（Lithe 专有） | `ChromeBar region = title/footer/tabs/sidebar`（高度分别取 `--lithe-title-bar-height` / `--lithe-footer-height` / `--lithe-tab-bar-height` / `--lithe-sidebar-header-height`）；`emphasis = supporting/neutral/primary`；`separated` → `border-border/55 border-b`；`ChromeGroup gap = none/tight/default/loose`；`ChromeLabel tone = muted/default/strong/accent`；`ChromeSeparator` vertical `mx-0.5 h-3.5 w-px` | `windows/tauri/src/ui/chrome.tsx:5-142` | **未找到**；对应 `gpui_component::{title_bar::TitleBar, status_bar::StatusBar}`（`title_bar.rs:42`、`status_bar.rs:32`） |
| `Icons` | 205 个 `createIconComponent` 导出；优先 IntelliJ 双色 SVG（`<svg viewBox="0 0 16 16">` + 两张 `<image>`），否则回落 lucide；`size` 默认 `"1em"`；`weight = thin/light/regular/bold/fill/duotone` → strokeWidth 1/1.25/2；明暗切换靠 CSS `display` | `windows/tauri/src/ui/icons.tsx:12-14,108-146,154,191-217,223-628`；`windows/tauri/src/ui/icons.css:8-21` | `gpui_component::icon::{Icon, IconName}`（`icon.rs`，`lib.rs:112`）+ `gpui-component-macros::icon_named`（`lib.rs:111`）；**双色 light/dark 资源切换需自建** |
| `Chart` | recharts 换皮（334 行） | `windows/tauri/src/ui/chart.tsx:2,40` | `gpui_component::chart::{AreaChart, BarChart, LineChart, PieChart, RadarChart, CandlestickChart, SankeyChart}`（`chart/mod.rs:9-15`，定义见 `bar_chart.rs:44`/`line_chart.rs:33`/`area_chart.rs:35`/`pie_chart.rs:41`/`radar_chart.rs:77`/`candlestick_chart.rs:31`/`sankey_chart.rs:115`）—— **gpui 侧更完整**；hover dot `px(8.)`、halo 20（`chart/mod.rs:44,47`） |

### 2.17 gpui-kit 里**写死的像素常量**（密度映射必须逐个覆盖）

gpui 组件的「高度」大多不是 px 常量而是 Tailwind 尺度类（`h_5/h_6/h_8/h_11`），**明确写死 px 的只有下面这些**：

| 项 | gpui 值 | 来源 | Lithe 对应值 | 差距 |
| --- | --- | --- | --- | --- |
| **TitleBar 高**（`pub const`） | `px(34.)` | `gpui-component-0.6.6/src/title_bar.rs:15` | `--lithe-title-bar-height` = **40px**（`theme.css:118`） | **差 6px，必须覆盖** |
| TitleBar 左内边距 | macOS `px(80.)` / 其它 `px(12.)` | `title_bar.rs:17,19` | `--lithe-chrome-padding-inline` = 8px | 差 4px |
| StatusBar 高 | **无常量**，`.py_1().px_2().text_xs().border_t_1()` | `status_bar.rs:86-95` | `--lithe-footer-height` = 24px（`theme.css:119`） | 靠 padding 推导，实际约 8+8+12(行高)=28 左右 → 需 `.h()` 覆盖 |
| Sidebar 展开宽 / 折叠宽 | `px(255.)` / `px(48.)` | `sidebar/mod.rs:27,28` | 由用户拖拽决定（`main-sidebar.tsx` 的 `railPanelWidth`） | **不适用**，需接管宽度 |
| Sidebar 折叠过渡 | `200ms` | `sidebar/mod.rs:29` | `--app-duration-normal` = 200ms | ✅ 一致 |
| SidebarMenuItem 高 | `.h_7()` = 28px | `sidebar/menu.rs:308` | `SidebarListItem` `min-h-(--lithe-tab-height)` = **28px** | ✅ **巧合一致** |
| SidebarGroup 高 | `.h_8()` = 32px | `sidebar/group.rs:70` | `SidebarSectionHeader h-(--lithe-tab-height)` = 28px | 差 4px |
| Dialog 宽 | `px(448.)` | `dialog/dialog.rs:177` | `size=md` → `max-w-md` = 448px | ✅ **完全一致** |
| Sheet 宽 | `350px` + padding `16` | `sheet.rs:63,147` | 垂直时 `w-3/4`、`sm:max-w-sm`(384px) | 差 34px |
| Notification 宽 / margin | `px(382.)` / `px(16.)` | `notification.rs:26,549` | toast `max-w-sm`=384px，`inset-x-4 bottom-4` | ✅ 基本一致 |
| Notification 最大条数 | `10` | `notification.rs:558` | 未找到上限 | — |
| Skeleton 默认尺寸 | `w_full().h_4()` = 16px 高 | `skeleton.rs:40-41` | 由调用方 className 决定 | 需覆盖 |
| Spinner | `Size::Medium` + `0.8s` | `spinner.rs:22,23` | `size-4`(16px) + `animate-spin`；Lithe 无显式时长（Tailwind 默认 1s） | 需覆盖 |
| Popover offset / margin | `px(6.)` / `px(8.)` | `popover.rs:37-38` | `sideOffset=6` / `collisionPadding=8` | ✅ **完全一致** |
| Form label 宽 / 列数 | `140px` / `1` | `form/form.rs:64-68,100` | Lithe `field.tsx` 用 `orientation` 三档，无固定 label 宽 | 不适用 |
| Form 间距 | 6 / 8 / 12px（按 size） | `form/form.rs:123-127` | `gap-4`(16px) / `gap-2`(8px) | 差 |
| Separator 线宽 / 虚线 | `1px` / dash `[px(4.), px(2.)]` | `separator.rs:81-82,95` | `h-px` / `w-px` | ✅ 线宽一致 |
| InputGroup 按钮 | XS `h_6 px_2` / S `h_8 px_2p5` | `input/group.rs:590-592` | `InputGroupButton xs h-5 / sm h-6 / icon-xs size-5` | 差 |
| OtpInput 单元 | `w_6 h_6` / `w_8 h_8` / `w_11 h_11` | `input/otp_input.rs:127-130` | `input-otp` slot 固定 `size-8`(32px) | Medium 档一致 |
| Radio 尺寸 | `size_4`(16px) | `radio.rs:126`（Sizable） | `size-4`(16px) | ✅ 一致 |
| Checkbox | 由 `Size` 决定 | `checkbox.rs:167` | 固定 `size-4`(16px) | 需固定 |
| Slider thumb ring / thumb / 竖排 | `px(3.)` / `px(24.)` / `px(120.)` | `slider.rs:15,329,268` | thumb `size-3`(12px) + `after:-inset-2` | 差 |
| Carousel 轴锁 / 手势分隔 | `px(2.)` / `28ms` | `carousel/state.rs:8,11` | 未找到对应 | — |
| HoverCard 延迟 | `600ms` / `300ms`；默认 `Anchor::TopCenter` | `hover_card.rs:47,48,43` | Lithe 无 hover-card 使用点（0 引用） | 不适用 |
| 滚动条 thumb 宽 / inset | rest 6px、hover 6px、active 8px、inset 4px；idle 2s / enter 300ms / exit 500ms / expand 300ms | `theme/mod.rs:64-86` | 11px 常显 / 9px 细、hover 才显现、无 transition | **行为不同** |
| 焦点环宽 / 不透明度 | `px(3.)` / `0.5` | `styled.rs:11-12` | `ring-2`(2px) / `/20` | 差 |
| `RADIUS_FULL` | `px(9999.)` | `theme/mod.rs:62` | `rounded-full` = 9999px | ✅ 一致 |

**结论**：`Dialog` 宽、`Popover` offset/margin、`Notification` 宽、`SidebarMenuItem` 高、`Separator` 线宽、`RADIUS_FULL` 这 6 项**天然对齐**；**`TitleBar` 34→40px 是必须显式覆盖的第一项**，其余按上表逐个 `.h(px(...))` 或 `.width(px(...))` 覆盖。

---

## 3. 映射表：Windows token → gpui-kit 0.6.6

gpui-kit 0.6.6 的层级：`gpui_kit::*` = GPUI 本体（`pub use ::gpui::*`，`gpui-kit-0.6.6/src/lib.rs:95`）；`gpui_kit::base` = `gpui-base`（`:106`）；`gpui_kit::component` = `gpui-component`（`:143`）；`gpui_kit::assets` = `gpui-kit-assets`（`lib.rs:9-14` 的 feature 表）。**gpui-kit 自身只有 164 行，不定义任何组件** —— 所以「gpui-kit 的组件 API」= `gpui_kit::component::*` = `gpui-component-0.6.6` 的 API。
主题 token 实际定义在 **`gpui-base-0.6.6/src/theme_tokens.rs`**，由 `gpui-component` 重新导出（`gpui-component-0.6.6/src/theme/mod.rs:9-12`）。

### 3.1 颜色映射

**语义 token（`gpui_base::ColorTokens`，共 18 项）** —— `gpui-base-0.6.6/src/theme_tokens.rs:20-45`：

| Windows token | gpui `ColorTokens` 字段 | gpui 默认 Light（l 为 HSL 亮度） | 说明 |
| --- | --- | --- | --- |
| `--background` | `background` | `hsl(0,0,1.0)` 白 | 直接对应 |
| `--foreground` | `foreground` | `hsla(0,0,0.039,1)` | 直接对应 |
| `--surface` | `surface` | `hsl(0,0,1.0)` | ⚠️ gpui 里 `surface` = `ThemeColor::popover`（`theme/mod.rs:418`），而 Lithe 的 `--surface` 是**面板底**（比 background 暗一档）→ **语义不同，需重设** |
| `--foreground` | `surface_foreground` | `hsla(0,0,0.039,1)` | |
| `--primary` | `primary` | `hsla(0,0,0.09,1)` | gpui 默认主色是**近黑**，Lithe 是蓝 `#3574f0` → 必须覆盖 |
| `--background` | `primary_foreground` | `hsla(0,0,0.98,1)` | ⚠️ Lithe 的 `--primary-foreground` = `var(--background)`（`theme.css:149`），不是白 → 语义不同 |
| **未找到** | `secondary` / `secondary_foreground` | `hsla(0,0,0.898)/hsla(0,0,0.09)` | Lithe 无二级底色 token；`--accent`(`#edf3ff`) / `--selected` 更接近 |
| `--surface`（或 `--accent`） | `muted` | `hsla(0,0,0.961,1)` | Lithe 的 `--muted` = `var(--surface)`（`theme.css:144`） |
| `--muted-foreground` | `muted_foreground` | `hsla(0,0,0.451,1)` | 直接对应（Lithe 的 `muted-foreground` = `#4f5965`） |
| `--accent` | `accent` | `hsla(0,0,0.961,1)` | ⚠️ gpui 的 `accent` = hover 底色（近中性灰），Lithe 的 `--accent` = `#edf3ff`（淡蓝）→ **需重设** |
| `--foreground` | `accent_foreground` | `hsla(0,0,0.09,1)` | |
| `--destructive` | `destructive` | `hsla(0,0.842,0.602,1)` | 直接对应 |
| **未找到** | `destructive_foreground` | `hsla(0,0,0.98,1)` | Lithe 无此 token |
| `--border` | `border` | `hsla(0,0,0.898,1)` | 直接对应 |
| `--input`（= `--border`） | `input` | `hsla(0,0,0.898,1)` | 直接对应 |
| `--ring`（= `--border-strong`） | `ring` | `hsla(0,0,0.639,1)` | 直接对应 |
| `--selection` | `selection` | `rgb(0x55a0fc)` α=0.3 | Lithe light = `rgba(53,116,240,0.2)`；dark = `#214283`（**不透明**）→ 需按主题设值 |
| **未找到** | — | — | `--subtle-foreground`（第三级文字色）、`--selected`（行选中底）、`--border-strong`、`--info`、`--cursor*` 全部无对应 |

**Legacy 组件级 token（`gpui_component::ThemeColor`，约 130 字段）** —— `gpui-component-0.6.6/src/theme/theme_color.rs:59-341`。这里能找到 Lithe 缺失者的近似项：

| Windows token | gpui `ThemeColor` 字段 | 备注 |
| --- | --- | --- |
| `--selected` | `list_active` / `table_active` | gpui 用「active」表示选中行；还有 `list_active_border`、`table_active_border` |
| `--accent`（hover 底） | `list_hover` / `table_hover` / `button_hover` / `accent` | |
| `--surface` | `popover` / `list` / `table` / `sidebar` / `group_box` / `accordion` / `tab_bar` | gpui 按组件给了名字，Lithe 只有一个 `--surface` |
| `--subtle-foreground`（近似） | `muted_foreground`（`table_head_foreground`、`table_foot_foreground`、`description_list_label_foreground` 均回落它，`theme/schema.rs:1007,1009,949-952`） | ⚠️ 只有一档，Lithe 有两档（muted / subtle） |
| `--border-strong` | **未找到**；最接近 `ring`（`schema.rs:973` 回落到 `blue`）或 `drag_border`（`schema.rs:953` 回落到 `primary@65%`） | |
| `--info` | `info` + `info_foreground/hover/active` | gpui 有；Lithe 的 `--info` 目前**未被 `@theme` 之外的组件大量使用** |
| `--cursor` | `caret`（`theme_color.rs:131`） | Lithe 有 3 个光标 token（cursor / vim-normal / vim-insert），gpui 只有 1 个 |
| `--git-*`（6 个） | **未找到** | 需要自建；gpui 只在 `highlight` 段有 `conflict/created/modified`（`default-theme.json:119-124`） |
| `--terminal-*`（16 个） | **未找到** | 需要自建；gpui 有 `chart_1..5` 与 `base.{red,green,blue,yellow,magenta,cyan}[.light]`（`theme_color.rs:317-340`）可借用色板 |
| `--syntax-*`（18） | `HighlightTheme`（`highlighter`） | 见 §3.4 |

**Lithe → gpui 的颜色取值建议（`Theme` 是全局 `Global`，字段可直写）**：
```rust
let t = Theme::global_mut(cx);
t.background   = Hsla::parse_hex("#ffffff")?;   // lithe.json:13
t.foreground   = Hsla::parse_hex("#1f2328")?;   // :15
t.primary      = Hsla::parse_hex("#3574f0")?;   // :22
t.border       = Hsla::parse_hex("#dfe1e5")?;   // :18
t.muted_foreground = Hsla::parse_hex("#4f5965")?; // :16
t.popover      = Hsla::parse_hex("#f7f8fa")?;   // --surface
t.accent       = Hsla::parse_hex("#edf3ff")?;   // :19
t.list_active  = Hsla::parse_hex("#d4e2ff")?;   // --selected
t.selection    = rgba(0x3574f0……) α=0.2;        // :21
t.danger       = Hsla::parse_hex("#cf3f4f")?;   // :26
Theme::sync_base(cx);                            // 必须！见 gpui-component-0.6.6/src/theme/mod.rs:367-372
```

### 3.2 尺寸 / 字体 / 圆角 / 间距 / 动效 / 手势映射

| Windows token | gpui-kit 对应 | 差距与做法 |
| --- | --- | --- |
| `--lithe-title-bar-height` 2.5rem(40) | `Theme` 无对应字段；`gpui_component::title_bar::TitleBar`（`title_bar.rs:42`）内部自算 | **自建** `LitheMetrics.title_bar_height: Pixels = px(40.0 * scale)` |
| `--lithe-footer-height` 1.5rem(24) | `gpui_component::status_bar::StatusBar`（`status_bar.rs:32`） | **自建** |
| `--lithe-pane-header-height` 2.25rem(36) | **未找到** | **自建** |
| `--lithe-tab-bar-height` = pane-header | `TabBar` 高度按 `TabVariant` + `Size` 硬编码（`tab/tab.rs:22-30`） | **自建**；或覆写 `TabBar` 的 size |
| `--lithe-tab-height` 1.75rem(28) | 同上 | **自建** |
| `--lithe-tab-max-width` 12.5rem(200) | **未找到** | **自建**（布局约束） |
| `--lithe-sidebar-header-height` 2rem(32) | `sidebar` 无高度常量 | **自建** |
| `--lithe-workbench-gap` 4px | `gpui_base::ResizableTheme { handle, active_handle }` 只管颜色不管宽度（`theme/mod.rs:342-345`）；`resize_handle.rs` 内部定义热区 | **自建**：把手宽度用 `px(4.)`，颜色用 `theme.border` / `theme.drag_border` |
| `--lithe-chrome-control-height` 1.5rem(24) | 无 | **自建** |
| `--lithe-chrome-hit-target` 1.75rem(28) | 无 | **自建**（且 Windows 侧本身无引用点） |
| `--lithe-chrome-line-height` 1rem(16) | 无 | **自建** |
| `--lithe-chrome-gap{,-tight,-loose}` 4/2/6px | `gpui_base::SpacingTokens` 有 7 档（见下） | 用 `SpacingTokens::xxs(2)` / `xs(4)` / `sm(8)`；**loose=6 无对应** → 自建 |
| `--lithe-chrome-padding-inline` 8px | `SpacingTokens::sm = px(8.)` | 可对应 |
| `--lithe-chrome-radius` 4px | `RadiusTokens::sm = px(3.)`（默认） | 需覆写为 `px(4.)` |
| `--radius` 8px / sm 4.8 / md 6.4 / lg 8 / xl 11.2 / 2xl 14.4 / 3xl 17.6 / 4xl 20.8 | `gpui_base::RadiusTokens { none, sm, md, lg, xl, full }`，由 `Theme::radius` / `radius_lg` 派生：`sm = radius/2`、`md = radius`、`lg = radius_lg`、`xl = radius*2`、`full = 9999 或 0`（`theme/mod.rs:471-480`）；另有 `radius_2xl = radius*2.5`、`3xl = *3.`、`4xl = *3.5`（`:457-469`） | **无法表达 Lithe 的 0.6/0.8/1.0/1.4/1.8/2.2/2.6 系数** → **必须自建** `LitheRadii`（7 档 `Pixels`），或接受重投影 |
| `SpacingTokens`（gpui 默认） | `xxs 2 / xs 4 / sm 8 / md 12 / lg 16 / xl 24 / xxl 32`（`gpui-base-0.6.6/src/theme_tokens.rs:142-153`） | 与 Tailwind 的 `0.5/1/2/3/4/6/8` 单位**完全同值**（2/4/8/12/16/24/32px）→ **间距可直接用，这是最省事的一块**。<br>⚠️ 但 `Theme::spacing_tokens()` **直接返回 `SpacingTokens::default()`，完全不读主题**（`gpui-component-0.6.6/src/theme/mod.rs:482-484`）→ **间距阶梯目前无法由主题配置**；若 Lithe 要按密度改间距，只能改 gpui-component 或全部走显式 `px()` |
| `Theme::radius` / `radius_lg` | gpui 默认 `radius = px(6.)`、`radius_lg = px(8.)`（`theme/mod.rs:668-669`） | Lithe `--radius = 8px` → 设 `Theme::radius = px(8.)`（但派生系数仍不匹配，见上） |
| `TITLE_BAR_HEIGHT` | `pub const TITLE_BAR_HEIGHT: Pixels = px(34.)`（`title_bar.rs:15`） | Lithe 40px → **必须覆盖**（`SheetSettings.margin_top` 也吃这个值，`sheet.rs:27-38`） |
| `TypographyTokens`（gpui 默认） | `xs 12/16`、`sm 14/20`、`md 16/24`、`lg 18/28`、`xl 20/28`、`mono_md 13/20`（`theme_tokens.rs:175-188`） | 与 Tailwind `text-xs/sm/base/lg/xl` **完全同值**；但 Lithe 的 `ui-text-sm`(13px)/`ui-text-caption`(12px)/`ui-text-chrome`(13→14px) 是**另一套 px 值** → 见 §3.3 决策 |
| `--app-font-family` | `Theme::font_family`（`theme/mod.rs:126`），默认 `.SystemUIFont` | 直接赋值 `"Microsoft YaHei UI"` 等；注意 gpui 会做 installed-family 回落（`:120-126`） |
| `--editor-font-family` | `Theme::mono_font_family`（`:141`），Windows 默认 `Consolas` | 赋 `"Geist Mono"` 需随 assets 注册字体文件 |
| `--app-duration-fast` 150ms | `MotionTokens::duration_fast` = **120ms**（`theme/motion.rs:26`） | ⚠️ 值不同；`MotionTokens` 字段全 `pub` 且可整体替换（`theme/mod.rs:170`）→ **直接改写** |
| `--app-duration-normal` 200ms | `duration_normal` = **180ms**（`:27`） | 同上 |
| `--app-ease-smooth` `cubic-bezier(0.22,1,0.36,1)` | `easing_enter` = `cubic_bezier(0.16,1,0.3,1)`（`:29`） | 不同曲线 → 覆写 `Easing::cubic_bezier(0.22,1.0,0.36,1.0)` |
| `--app-ease-in-out` `cubic-bezier(0.66,0,0.34,1)` | `easing_move` = `cubic_bezier(0.2,0,0,1)` / `easing_exit` = `cubic_bezier(0.4,0,1,1)`（`:31-33`） | 需覆写 |
| 滚动条 idle/enter/exit/expand | gpui-component 自己定了：**idle 2s、enter 300ms、exit 500ms、expand 300ms**，thumb 宽 6px(hover)/8px(active)、inset 4px（`theme/mod.rs:64-86`） | Lithe 是 **11px 常显/9px 细、hover 才出现、thumb 无 transition** → 行为不同，需要 `Theme::set_scrollbar_mode` + 自定 `ScrollbarStyles` |
| `--shadow-*` 5 档 + hairline | `ShadowTokens { sm, md, lg }` 3 档，由 `ShadowTokens::elevations(transparent.alpha(0.18))` 生成（`theme/mod.rs:495-501`、`theme_tokens.rs:197-205`） | **只有 3 档且颜色统一** → **自建 `LitheShadows`（5 档 `Vec<BoxShadow>`）** |

### 3.3 尺寸体系的关键决策（必读）

gpui 的 `Theme` 只有一个 `font_size`，同时承担两个角色：
1. `window.set_rem_size(cx.theme().font_size)`（`gpui-component-0.6.6/src/root.rs:582`）→ 决定**所有** `rems()` 派生的间距/尺寸/圆角；
2. `TypographyTokens.md.size`（`theme/mod.rs:490`）→ 决定 `text_base()`。

Windows 侧则是：`html { font-size: calc(16px * uiFontSize/13) }`（`theme.css:220`），因此 **Tailwind 的 `h-8` / `p-1` / `gap-2` 也随 `uiFontSize` 缩放**（因为 Tailwind v4 的 spacing 是 `0.25rem` 派生）。

**结论**：`Theme::font_size` 应设为 **`px(16.0 * uiFontSize / 13.0)`**，与 Windows 的根字号语义完全对齐。此后再把 gpui 的 `text_*` 阶梯与 Tailwind 的 `text-*` 阶梯一一对应，而 Lithe 专用的 `ui-text-sm/base/chrome/caption`（13/13/13→14/12→13px）必须用**显式 px** 表达，不能借用 `text_sm()`/`text_base()`。

```rust
// 根字号：对齐 `html { font-size: calc(16px * --app-ui-scale) }`（theme.css:220）
let root = px(16.0 * (ui_font_size / 13.0));
Theme::global_mut(cx).font_size = root;      // == window.rem_size()（root.rs:582）
Theme::sync_base(cx);

// Lithe 专用字号：显式 px（不要用 text_sm/text_base）
const UI_TEXT_CAPTION: f32 = 12.0;  // theme.css:114 / utilities.css:34-36
const UI_TEXT_CHROME:  f32 = 13.0;  // theme.css:115 / utilities.css:38-40
const UI_TEXT_SM:      f32 = 13.0;  // theme.css:116 / utilities.css:30-32
const UI_TEXT_BASE:    f32 = 13.0;  // theme.css:117 / utilities.css:42-44
// 使用：div().text_size(px(UI_TEXT_CHROME * scale))
```

**组件尺寸映射表**（Lithe class → gpui）：

| Lithe | px | gpui 写法 | 说明 |
| --- | --- | --- | --- |
| 输入 xs `h-6` | 24 | `Size::Small`（`input_h` → `h_6`=24px，`sizing.rs:266`） | ✅ 直接对应 |
| 输入 sm（默认）`h-7` | 28 | **无对应** → `Size::Size(px(28.))` 或 `with_size(px(28.))` | gpui 的 Small=24 会偏小 |
| 输入 md `h-8` | 32 | `Size::Medium`（`h_8`=32px，`sizing.rs:265`） | ✅ 直接对应 |
| Button xs/icon-xs `h-6`/`size-6` | 24 | `Size::XSmall`（`button.rs:620,628` → `size_5`/`h_5`=**20px**） | ⚠️ gpui `XSmall` = 20px，Lithe = 24px |
| Button sm/icon-sm `h-7`/`size-7` | 28 | `Size::Small`（`button.rs:621,629-632` → `size_6`/`h_6`=**24px**） | ⚠️ 差 4px |
| Button default/icon `h-8`/`size-8` | 32 | `Size::Medium`（`button.rs:622,633-636` → `size_8`/`h_8`=**32px**） | ✅ 对应 |
| Button lg `h-9` | 36 | `Size::Large`（`button.rs:622,637-640` → `size_8`/`h_8`=**32px**） | ⚠️ gpui Large = 32px，Lithe = 36px |
| 表格行 `h-8`(表头) | 32 | `Size::Medium::table_row_height()` = 32px（`sizing.rs:63`） | ✅ 对应 |
| 文件树行 24px | 24 | 自建（无 List 行高常量） | ListItem 走 `list_py` |

→ **Button / Input 的 `Size` 阶梯与 Lithe 不一致**：gpui 的 `XSmall/Small/Medium/Large` 对应 20/24/32/32（按钮）与 20/24/32/44（输入），Lithe 是 24/28/32/36。**建议：Rust 侧不要用 `Size` 枚举表达 Lithe 的尺寸档，改为自定义 `LitheControlSize { Xs, Sm, Md, Lg }` 直接给 `Pixels`，再在恰好的场景复用 `Size`。**

### 3.4 gpui-kit 没有对应项 —— 需要自建的部分

按「放哪个结构体 / 怎么取取值」给出建议。

#### (1) `LitheMetrics` —— 密度与 chrome 尺寸（**最大缺口**）

放哪：新建 `lithe-ui/src/theme/metrics.rs`，注册为 gpui `Global`，与 `Theme` 并列。
怎么取值：从 `lithe.json` 之外的**静态常量表**读（两档密度），乘以根缩放系数 `scale = root_font_size / 16px`。

```rust
#[derive(Clone, Copy, Debug)]
pub struct LitheMetrics {
    pub scale: f32,                       // = ui_font_size / 13.0
    pub title_bar_height: Pixels,         // compact 2.5rem  → px(40.0)  * scale
    pub footer_height: Pixels,            // compact 1.5rem  → px(24.0)  * scale
    pub pane_header_height: Pixels,       // 2.25rem         → px(36.0)  * scale
    pub tab_bar_height: Pixels,           // = pane_header_height
    pub tab_height: Pixels,               // 1.75rem         → px(28.0)  * scale
    pub tab_max_width: Pixels,            // 12.5rem         → px(200.0) * scale
    pub sidebar_header_height: Pixels,    // 2rem            → px(32.0)  * scale
    pub workbench_gap: Pixels,            // 4px（不缩放）    → px(4.0)
    pub chrome_control_height: Pixels,    // 1.5rem          → px(24.0)  * scale
    pub chrome_hit_target: Pixels,        // 1.75rem         → px(28.0)  * scale
    pub chrome_line_height: Pixels,       // 1rem            → px(16.0)  * scale
    pub chrome_gap_tight: Pixels,         // 2px
    pub chrome_gap: Pixels,               // 4px
    pub chrome_gap_loose: Pixels,         // 6px             ← SpacingTokens 无此档
    pub chrome_padding_inline: Pixels,    // 8px
    pub chrome_radius: Pixels,            // 4px
}
impl Global for LitheMetrics {}
```
`comfortable` 档把 `ui_text_caption` 12→13、`ui_text_chrome` 13→14、`footer_height` 24→32、`tab_height` 28→32、`sidebar_header_height` 32→36、`chrome_control_height` 24→28、`chrome_hit_target` 28→32、`chrome_gap_tight` 2→4、`chrome_gap` 4→6、`chrome_gap_loose` 6→8、`chrome_padding_inline` 8→10（全部来自 `theme.css:188-200`）。

**必须同时覆盖 gpui 的写死常量**（否则 `LitheMetrics` 与组件实际高度不一致）：`TITLE_BAR_HEIGHT = px(34.)` → Lithe 40px（`title_bar.rs:15` vs `theme.css:118`）；`StatusBar` 无高度常量需显式 `.h()`；`SidebarMenu` `.h_7()`(28px) 与 Lithe `--lithe-tab-height`(28px) **恰好一致**，但 `SidebarGroup` `.h_8()`(32px) 与 Lithe 28px 差 4px。完整对照见 §2.17。

#### (2) `LitheRadii` —— 7 档圆角

放哪：同 `metrics.rs` 或 `theme/mod.rs`。gpui 的 `RadiusTokens` 只表达 6 档且系数固定（`sm=radius/2`、`md=radius`、`xl=radius*2`，`theme/mod.rs:471-480`），无法表达 `0.6/0.8/1.0/1.4/1.8/2.2/2.6`（`theme.css:6-12`）。

```rust
pub struct LitheRadii {  // 全部 = base * k * scale
    pub sm: Pixels,   // base * 0.6
    pub md: Pixels,   // base * 0.8
    pub lg: Pixels,   // base * 1.0  (base = 8px)
    pub xl: Pixels,   // base * 1.4
    pub xl2: Pixels,  // base * 1.8
    pub xl3: Pixels,  // base * 2.2
    pub xl4: Pixels,  // base * 2.6
    pub chrome: Pixels, // 4px
    pub full: Pixels,   // 9999
}
```

#### (3) `LithePalette` —— git / terminal / cursor / 第三级文字 / selected

放哪：`lithe-ui/src/theme/palette.rs`，`Global`。
怎么取值：直接解析 `lithe.json` 的 `colors` 段（39 键），因为 gpui 的 `ThemeColor` 覆盖不到这些键。

```rust
pub struct LithePalette {
    pub subtle_foreground: Hsla,     // lithe.json:17 / :83
    pub selected: Hsla,              // :20 / :86（同时建议写入 Theme::list_active）
    pub border_strong: Hsla,         // = mix(border 72%, foreground 28%)；用 Colorize::mix
    pub git: GitPalette,             // modified / modified_staged / added / deleted / untracked / renamed（:30-35 / :96-101）
    pub terminal: [Hsla; 16],        // :36-51 / :102-117
    pub cursor: Hsla,                // :23 / :89
    pub cursor_vim_normal: Hsla,     // :24 / :90
    pub cursor_vim_insert: Hsla,     // :25 / :91
    pub syntax_markdown: Option<MarkdownSyntaxPalette>, // 见 (4)
    pub glass: LitheGlass,           // 13 个 --lithe-glass-*/chrome-* 变量（window-transparency.css:5-36）
}
```
颜色混合用现成的 `gpui_component::Colorize`：`mix`/`mix_oklab`/`lighten`/`darken`/`opacity`（`gpui-component-0.6.6/src/theme/color.rs:20-61`）。
`--border-strong` 的 `color-mix(in srgb, border 72%, foreground 28%)` → `border.mix(foreground, 0.72)`（注意 `mix` 的 `factor` 是**第一个**颜色的权重，`color.rs:44-45,209-225`）。

#### (4) Syntax 高亮映射（含 9 个无主键的 markdown）

gpui 的 `HighlightTheme` / `HighlightThemeStyle` 定义在 **`gpui-component-0.6.6/src/highlighter/registry.rs`**（`HighlightTheme` `:465-470`，`HighlightThemeStyle` `:436-457`，`Deref<Target = SyntaxColors>` `:472-478`）。结构为：

- `editor.*` 7 键：`editor.background`（`:437`）/`editor.foreground`（`:439`）/`editor.active_line.background`（`:441`）/`editor.line_number`（`:443`）/`editor.active_line_number`（`:445`）/`editor.invisible`（`:447`）/`editor.gutter.background`（`:451`，缺省回退 `editor.background`）。
- `#[serde(flatten)] status: StatusColors`（`:453`）—— **15 键**：`error{,.background,.border}`、`warning{,.background,.border}`、`info{,.background,.border}`、`success{,.background,.border}`、`hint{,.background,.border}`（`:315-346`）；缺省回退主题色（red `:351`、yellow `:368`、blue `:385`、green `:402`、cyan `:419`；背景 = `background.blend(color.alpha(0.2))`）。
- `#[serde(rename = "syntax")] syntax: SyntaxColors`（`:455`）—— **完整 41 个 token 键**（常量 `:15-57`、字段 `:113-172`、match `:246-289` 三处一致）：
  `attribute, boolean, comment, comment.doc, constant, constructor, embedded, emphasis, emphasis.strong, enum, function, hint, keyword, label, link_text, link_uri, number, operator, predictive, preproc, primary, property, punctuation, punctuation.bracket, punctuation.delimiter, punctuation.list_marker, punctuation.special, string, string.escape, string.regex, string.special, string.special.symbol, tag, tag.doctype, text.code.span, text.literal, title, type, variable, variable.special, variant`。
  键回退：`keyword.modifier` 这类退到首段前缀（`:292-305`）；`style_for_index`（`:309-311`）；`impl gpui_base::input::HighlightStyleResolver for HighlightTheme`（`:490-494`）。
- `ThemeStyle{color, font_style, font_weight}`（`:223-227`）、`FontStyle{Normal, Italic, Underline}`（serde 小写，`:176-180`）、`FontWeightContent` 100..900（`:194-204`）。
- **从 JSON 构造两条路**：① 主题 JSON 的 `ThemeConfig.highlight: Option<HighlightThemeStyle>`（`theme/schema.rs:78-81`，Zed 兼容），registry 装载时 `HighlightTheme { name: theme.name.to_string(), appearance: theme.mode, style: theme.highlight.unwrap_or_default() }`（`theme/registry.rs:26-30`）；② `HighlightTheme` 自身 `derive(Deserialize)`（`:464`），可直接 `serde_json::from_str::<HighlightTheme>`；内置 `default_dark()/default_light()`（`:481-487`）。
- 无 tree-sitter feature 时 `wasm_stub.rs` 提供同名类型（`:166 SyntaxColors`、`:289 HIGHLIGHT_NAMES`、`:405 HighlightThemeStyle`、`:421 HighlightTheme`）。

**Lithe 18 键 → gpui 41 键的映射（把上一版「无对应」的误判修正）**：

| Windows `--syntax-*` | gpui `syntax.*` | 备注 |
| --- | --- | --- |
| `comment` | `comment` + `comment.doc` | 同值填两个 |
| `keyword` | `keyword` | |
| `string` | `string` + `string.escape` | 同值填两个 |
| `number` | `number` | |
| `function` | `function` | |
| `variable` | `variable` | |
| `tag` | `tag` + `tag.doctype` | 同值填两个 |
| `attribute` | `attribute` | |
| **`punctuation`** | **`punctuation`（+ `punctuation.bracket` / `.delimiter` / `.list_marker` / `.special`）** | ✅ **gpui 有对应**（早前版本误判为缺失，已修正） |
| `constant` | `constant` | |
| `property` | `property` | |
| `type` | `type` | |
| **`operator`** | **`operator`** | ✅ **gpui 有对应**（早前版本误判为缺失，已修正） |
| `boolean` | **未找到** | 建议映射到 `constant`，或走 Lithe 自有高亮器 |
| `null` | **未找到** | 建议映射到 `constant` 或 `keyword` |
| `regex` | `string.regex` | |
| `jsx` | **未找到** | 建议映射到 `tag` |
| `jsx-attribute` | **未找到** | 建议映射到 `attribute` |

→ **真正无对应的只有 4 个**：`boolean`、`null`、`jsx`、`jsx-attribute`（不是 6 个）。
**另外两类在 gpui 中零对应**：
- `--symbol-*` 7 键（`theme.css:170-176`）**全部无对应** —— 这是 Lithe 为 IDE 符号图标专门造的。
- 9 个 `--syntax-markdown-*`（`syntax-tokens.css:110-154`）**全部无对应**，且 Windows 侧本身也没有主题键（§1.11）→ 建议在 Rust 侧把 markdown 高亮颜色**提升为主题键**，不要沿用硬编码。

**换向也成立**：gpui 有 41 键而 Lithe 只有 18 键，因此 gpui 的 `constructor / embedded / emphasis / emphasis.strong / enum / hint / label / link_text / link_uri / predictive / preproc / primary / text.code.span / text.literal / title / variant / punctuation.bracket / punctuation.delimiter / punctuation.list_marker / punctuation.special / tag.doctype / string.special / string.special.symbol / comment.doc / string.escape` 需要 Lithe 侧给出派生规则（例如 `enum → number`、`constructor → function`、`title → tag`、`link_uri → property`、`emphasis → comment 斜体`）。

**应用方式**：`Theme::global_mut(cx).highlight_theme = Arc::new(HighlightTheme { name, appearance, style })`（字段 `pub`，`theme/mod.rs:115`；构造见 `theme/schema.rs:1066-1073`）。

#### (5) Lithe 主题文件加载器

gpui 自带 `ThemeRegistry`：`load_themes_from_str(content)`（`theme/registry.rs:151`，同名不覆盖 `:154-158`）、`watch_dir(themes_dir, cx, on_load)`（`:98`，notify 递归监听 `:186-223`）、`themes()` / `sorted_themes()`（`:121,126`）、`default_light_theme()/default_dark_theme()`（`:143,147`）。**没有 `load_themes(...)` / `set_theme(...)`**（`reload_themes` `:226` 与 `reload` `:238` 都是私有）。
但它只认 `ThemeSet`/`ThemeConfig` 结构（`theme/schema.rs:22-82`），键名是 `background` / `primary.background` / `tab.active.background` 这类点分字符串（`schema.rs:252-675`），**与 Lithe 的 39 键 `colors` + 18 键 `syntax` 完全不同**。

**推荐的换肤流程**（把三份文档的结论合起来）：
```rust
// 1) 解析 Lithe 主题 → 写入 Theme 的 legacy 字段
let mut t = Theme::global_mut(cx);
t.background = lithe.background; t.foreground = lithe.foreground; /* … */
t.radius = px(8.0 * scale); t.radius_lg = px(8.0 * scale);
t.font_family = "Microsoft YaHei UI".into();
t.mono_font_family = "Geist Mono".into();
t.font_size = px(16.0 * scale);            // = window.rem_size()，见 §3.3
t.motion = lithe_motion_tokens();          // MotionTokens 字段全 pub（theme/motion.rs:8-20）
t.highlight_theme = Arc::new(lithe_highlight_theme);
// 2) 重建 Base 层副本（否则滚动条/把手仍用旧值）
Theme::sync_base(cx);                       // theme/mod.rs:367-372
// 3) 若要让 gpui 的 ThemeRegistry/设置界面看到这套主题，再走
ThemeRegistry::global_mut(cx).load_themes_from_str(&lithe_theme_as_gpui_json)?;
```
**必须自建的部分**（gpui 的 schema 装不下）：
- 9 个必填键校验（`theme-file.ts:5-15`）；
- 旧键重映射表（`theme-file.ts:17-27`，9 条）；
- syntax 的 `foreground` 欧氏距离 `< 28` 回落（`syntax-token-colors.ts:99-128`）；
- `appearance: "light" | "dark"` → 决定写入 `light_theme` 还是 `dark_theme`（`theme-registry.ts:98-99`）；
- 39 键里 gpui 完全不认识的 26 键（git 6 + terminal 16 + cursor 3 + subtle-foreground 1）。

#### (6) 玻璃 / 透明窗口

gpui 无 backdrop-filter、无原生「窗后透」。涉及：`--lithe-glass-*` 13 个变量（`window-transparency.css:5-36`）、`.lithe-layout-shell` / `.lithe-title-bar` / `.lithe-footer-bar` / `.lithe-chrome-control` 的透明覆盖规则（`:38-123`）。
**建议**：Rust 侧把「窗口透明 + chrome 半透明」简化为一套固定的 `LitheGlass` 色板（直接取 light/dark 覆盖值），不做 `color-mix` 动态混合；`backdrop-filter` 的行为直接放弃。

#### (7) 图标（205 个 + IntelliJ 双色）

gpui 有 `Icon` / `IconName` 与 `gpui-kit-assets`（`gpui-kit-0.6.6/src/lib.rs:111-114`），但 **light/dark 双色 SVG 切换**在 Windows 侧是靠 CSS `display: none/block`（`windows/tauri/src/ui/icons.css:12-21`）实现的。Rust 侧需在渲染时按 `Theme::global(cx).is_dark()` 选 `<image>` href —— 已确认 gpui 支持 SVG（`gpui-pre-0.3.6/src/svg_renderer.rs`）；**205 个图标的映射表需要单独出一份文档。**

#### (8) 拖拽排序（标签栏 / 树 / 列表）

`@dnd-kit` 在 `tab-bar.tsx` 定义了 5px 激活距离、`pointerWithin → closestCenter` 碰撞、180ms 位移、rAF 点击抑制（`windows/tauri/src/ui/tab-bar.tsx:20-43,71-79,110-152`）。gpui 只有 `gpui_base::dock::drag`（`gpui-base-0.6.6/src/dock/drag.rs`）用于 **dock 面板拖拽**，不是通用 sortable。
→ **标签排序、列表排序、文件拖放必须自研**，建议复刻上述 4 个参数。

#### (9) 通知中心

`windows/tauri/src/features/notifications/` 是独立工具窗（分组/过滤/搜索/详情/上下文菜单）。gpui 的 `notification` 只有 toast + `NotificationList`（`notification.rs:693`）。
→ 需要 `NotificationCenterPanel`（可复用 `List` + `ListDelegate` + `Dock`）。

#### (10) 命令面板

`command.tsx` 是 Lithe 自研（`windows/tauri/src/ui/command.tsx:437`），gpui 的 `command::Command` API 完全不同（`command/mod.rs:11-13`）。→ 需要在 gpui `Dialog + Input + List/Scrollable` 上重建（见 §4.2）。

---

## 4. 复刻优先级

### 4.1 A 类：直接用 gpui-kit 现成组件（只做 token 重着色）

| Windows 组件 | gpui-kit 类型 | 需覆盖的 token |
| --- | --- | --- |
| `button.tsx` | `button::Button` + `Sizable` | `button*` / `primary*` / `danger*` / `accent` |
| `input.tsx` / `textarea.tsx` | `input::{InputState, TextInput}` / `input::Textarea` | `input` / `background` / `foreground` / `ring` |
| `number-input.tsx` | `input::NumberInput`（`input/mod.rs:48`） | 同上 |
| `checkbox.tsx` / `radio-group.tsx` / `switch.tsx` / `slider.tsx` | `checkbox::Checkbox` / `radio::{Radio, RadioGroup}` / `switch::Switch` / `slider::Slider` | `primary` / `border` / `input` / `muted` |
| `toggle.tsx` / `toggle-group.tsx` | `button::Toggle` + `gpui_base::toggle_group` | `accent` / `selected` |
| `progress.tsx` | `progress::{Progress, ProgressCircle}` | `primary` / `surface` |
| `skeleton.tsx` | `skeleton::Skeleton` | `muted` |
| `spinner.tsx` | `spinner::Spinner` | `foreground` |
| `separator.tsx` | `separator::Separator` | `border` |
| `tooltip.tsx` | `tooltip::Tooltip` | `popover` / `border` |
| `avatar.tsx` | `avatar::Avatar` | `surface` / `muted_foreground` |
| `accordion.tsx` / `collapsible.tsx` | `accordion::{Accordion, AccordionItem}` / `collapsible::Collapsible` | `accordion` / `border` |
| `breadcrumb.tsx` / `pagination.tsx` | `breadcrumb::{Breadcrumb, BreadcrumbItem}` / `pagination::Pagination` | `muted_foreground` / `foreground` |
| `kbd.tsx` | `kbd::Kbd` | `accent` / `subtle_foreground`（需自建第三级色） |
| `label.tsx` | `label::Label` | `foreground` |
| `badge.tsx` | **`tag::Tag`**（不是 `badge::Badge`） | `primary/secondary/danger/success/warning/info` |
| `alert.tsx` | `alert::Alert`（`AlertVariant` 5 档**完全对齐** Lithe 的 tone 5 档） | `info/success/warning/danger` |
| `empty.tsx` | `empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription, EmptyContent}` | 需补 `tone` 概念 |
| `hover-card.tsx` | `hover_card::HoverCard` | `popover` / `border` |
| `table.tsx`（静态展示） | `table::{Table, TableHeader, TableBody, TableRow, TableHead, TableCell, TableCaption, TableFooter}` | `table*` / `list*` |
| `resizable.tsx` | `resizable`（`gpui-base/src/resizable`） | `border` / `drag_border` |
| `scroll-area.tsx` | `scroll::{Scrollable, ScrollableMask}` | scrollbar 4 色 |
| `chart.tsx` | `chart::{AreaChart, BarChart, LineChart, PieChart, RadarChart, CandlestickChart}`（**gpui 侧更强**） | `chart_1..5` / `chart_bullish/bearish` |
| `marker.tsx` | `marker::{Marker, MarkerVariant, MarkerLoadingStyle}`（变体名对齐） | — |
| `shimmer`（`utilities.css:84-104`） | `shimmer::{ShimmerText, ShimmerStyle}` | `subtle_foreground` / `foreground` |
| `dialog.tsx` / `alert-dialog.tsx` | `dialog::Dialog` + `window.open_dialog` / `open_alert_dialog` | `background` / `overlay` / shadow |
| `sheet.tsx` | `sheet::Sheet` + `window.open_sheet[_at]` | `popover` |
| `popover.tsx` | `popover::Popover` | `popover` / `border` |
| `dropdown-menu.tsx` / `context-menu.tsx` | `menu::{DropdownMenu, ContextMenu, ContextMenuExt, PopupMenu}` | `popover` / `accent` |
| `menubar.tsx` | `menu::AppMenuBar` | `popover` / `accent` |
| `sonner.tsx`（toast） | `notification::Notification` + `window.push_notification` | `background` / `border` + shadow |
| `radio`/`rating`/`color_picker`/`stepper`/`setting`/`description_list`/`clipboard`/`link` | 各自模块同名类型（`lib.rs:25-94`） | — |

### 4.2 B 类：需要「现成组件 + 组合」（不能直接替换）

| 目标 | 组合方式 |
| --- | --- |
| `card.tsx` | `group_box::GroupBox`（Normal/Fill/Outline 三档）+ 自建 4 档 variant 与 `--card-spacing` 等价物（`card.tsx:5-26`） |
| `item.tsx` | `list::ListItem` + 自建 media/content/title/description/actions 版式（`item.tsx:33-88`）；`size` 用 `list_size`（`sizing.rs:272-292`）而非 `Size` |
| `sidebar.tsx` 14 组件 | `sidebar::{Sidebar, SidebarMenu, SidebarMenuItem}` + `LitheMetrics`；`SidebarSectionHeader` / `SidebarListItem` / `SidebarListEditor` 需在 `ListDelegate::render_item` 内自绘（`sidebar.tsx:217-347`） |
| `tabs.tsx` 的 `line` / `bare` 变体 | `tab::{TabBar, Tab}`，`TabVariant::Underline` 近似 `line`（`tab/tab.rs:14-20`），`bare` 需自定样式 |
| `table.tsx` 动态数据表 | `table::{Table, TableDelegate}`：实现 `columns_count`/`rows_count`/`column`/`render_th`/`render_tr`/`render_td`（`table/delegate.rs:18-112`）；表头 `h-8` = `Size::Medium::table_row_height()` |
| `field.tsx` 10 组件 | `form::{Form, Field, FieldBuilder}`（`form/form.rs:14`、`form/field.rs:33,81`）+ 自建 `orientation` 三档 |
| `command.tsx` 22 导出 | `dialog::Dialog` 外壳 + `input::{InputState, TextInput}` + `list::{List, ListDelegate}` + `tab::Tab`（`bare`）+ `scroll::Scrollable`；复刻 `density = default/compact`（`command.tsx:41-42`）与键盘导航（`:595-626`） |
| `dropdown.tsx`（命令式定位） | `popover::Popover`（声明式）替换手写视口碰撞；若必须复刻手写定位，需要 `window.rem_size()`/`viewport` 信息 + 自绘锚点跟随 |
| `button-group.tsx` | 用 H/V 布局 + `ButtonRounded`（`button/button.rs:20`）逐边去圆角 |
| `notification` 中心 | `dock` + `list::{List, ListDelegate}` + `setting` 组合 |
| `empty.tsx` 的 `tone` | `empty::Empty` + `AlertVariant` 式着色包装 |
| `badge.tsx` 的 6 档 tone | `tag::Tag`（`TagVariant` 6 档 + `Custom`）—— 档数几乎一致（`tag.rs:10-24`） |

### 4.3 C 类：必须自研

| 项 | 理由 | 建议落点 |
| --- | --- | --- |
| **Lithe 主题加载器** | gpui `ThemeSet` 结构/键名完全不同（`theme/schema.rs:252-675` vs `lithe.json`） | `lithe-ui/src/theme/loader.rs`，复刻 `theme-file.ts:5-27` + `syntax-token-colors.ts:99-128` |
| **LitheMetrics 密度体系** | gpui 无 chrome 尺寸概念（§3.4(1)） | `lithe-ui/src/theme/metrics.rs`（`Global`） |
| **LitheRadii 7 档** | `RadiusTokens` 系数固定（§3.4(2)） | `lithe-ui/src/theme/metrics.rs` |
| **LithePalette（git/terminal/cursor/subtle/selected）** | gpui `ThemeColor` 无这些键（§3.4(3)） | `lithe-ui/src/theme/palette.rs`（`Global`） |
| **syntax 映射（含 9 个 markdown）与 6 个无对应键** | gpui `HighlightTheme` 键集不含 `punctuation/operator/boolean/null/jsx/jsx-attribute`（§3.4(4)） | `lithe-ui/src/syntax/mapping.rs` + 自建 highlighter |
| **LitheShadows 5 档（含 0.5px hairline）** | `ShadowTokens` 只有 3 档（§3.2） | `lithe-ui/src/theme/shadows.rs` |
| **玻璃 / 透明窗口** | gpui 无 backdrop-filter / 窗后透（§3.4(6)） | 降级为固定半透明色板 |
| **双色图标（205 个 + IntelliJ light/dark）** | gpui 无 CSS `display` 切换机制（§3.4(7)） | 渲染时按 `is_dark()` 选 `<image>` |
| **拖拽排序（标签 / 列表 / 文件树）** | gpui 只有 dock 拖拽（§3.4(8)） | `lithe-ui/src/dnd/sortable.rs`，复刻 5px/`closestCenter`/180ms |
| **Drawer（snapPoints / 嵌套堆叠 / 滑动手势）** | gpui 无 drawer 模块（§2.11） | 基于 `sheet::Sheet` 扩展或自研 |
| **通知中心工具窗** | gpui 无（§3.4(9)） | `lithe-ui/src/notifications/center.rs` |
| **命令面板交互（视口定位 / 分组 / 多 Tab / footer 快捷键）** | gpui `command::Command` 交互模型不同（§3.4(10)） | 用 A 类组件组装 |
| **IntelliJ 式 connected tab 3px 强调线** | gpui `TabVariant` 没有对应（§2.6） | 自绘 `before:` 等价元素 |
| **IDE 式文件树缩进指导线** | gpui `Tree` 无（§2.9） | 自绘 1px 竖线（`file-explorer-tree.css:170-200`） |
| **chrome 排版原语（`ChromeBar/Group/Label/Separator`）** | gpui 无（§2.16） | 薄封装 3 个 `Div` 构造器 + `LitheMetrics` |
| **markdown 语法彩色化（9 键）** | Windows 侧本来就是硬编码（§1.11） | 提升为主题键后自研 |

---

## 5. 未查清 / 未找到（明确列出，不含猜测）

### 5.1 路径与文件

1. **`windows/tauri/src/components/ui/` 未找到** —— 组件层实际在 `windows/tauri/src/ui/`；`windows/tauri/src/components/` 目录不存在。
2. **`tailwind.config.js|ts|mjs` 未找到** —— 项目是 CSS-first（`@theme inline`），仅 `windows/tauri/src/styles/theme.css:1`。
3. **`windows/tauri/node_modules` 未安装**（`Test-Path` = `False`）→ 无法给出 Tailwind v4 默认 `--spacing` / `--text-*` 的 `node_modules/tailwindcss/theme.css` 行号。§1.5 / §1.6 中的 Tailwind 默认值属于**未在仓库内查证**，仅依据任务给定的「1 单位 = 4px」与 `calendar.tsx:27` 的 `--spacing(7)` 用法。

### 5.2 冲突与疑似缺陷（需要产品决策）

4. **`--success` / `--warning` 被别名覆盖**：`windows/tauri/src/styles/theme.css:151-152` 把 `--success` 设为 `var(--primary)`、`--warning` 设为 `var(--muted-foreground)`，但 `lithe.json` 里 `success`/`warning` 有独立值（`:27-28` / `:93-94`）。主题注入顺序是 JS `setProperty`（inline style）晚于 CSS `:root`，因此**运行时 lithe.json 的值胜出**；但 `badge.tsx:13-14`、`empty.tsx:37`、`marker.tsx`、`alert.tsx:5-21` 都引用了 `success`/`warning` —— 若主题加载失败会落到 `primary`/`muted-foreground`。「这是有意 fallback 还是遗留冲突」**未查清**。
5. **`DialogProps.headerBorder` / `footerBorder` 声明但未使用**（`windows/tauri/src/ui/dialog.tsx:44-45` vs 解构 `:187-196`）。
6. **`SheetPortal` / `SheetOverlay` 已定义但未导出**（`windows/tauri/src/ui/sheet.tsx:21,25` vs 导出清单 `:117-125`）。
7. **`--lithe-chrome-hit-target` 无任何引用点**（只在 `theme.css:127` / `:195` 定义）。
8. **`popover.tsx:180` 的 import 位于文件末尾**（在所有 `export` 之后），属代码异味。

### 5.3 gpui-kit 侧（第一轮未核对、第二轮已补齐的项 —— 这些**不再是空白**）

9. ✅ **`progress::Progress` 已定位**：`gpui_component::progress::Progress`（`progress/progress.rs:14`，`new(id)` `:26`、`.loading` `:42`、`.color` `:48`、`.value(0..=100)` `:56`）+ `ProgressCircle`（`progress_circle.rs:17`）；Sizable `:74`/`:149`。
10. ✅ **input 主类型与构造已确认**：`Input`（`input/input.rs:111`，`new(&Entity<InputState>)` `:180`）、`Textarea`（`textarea.rs:14`，`new` `:36`）、`Editor`（`editor.rs:18`）、`OtpInput`（`otp_input.rs:22`，`new(&OtpState)` `:32`）、`NumberInput`（`number_input.rs:21`，`new(&InputState)` `:35`）、`InputGroup`（`group.rs:34`）、`AnyInputState{Input,Textarea,Editor,Otp}`（`state.rs:17-26`）。`input_h` 的落地调用点在 `input/input.rs:703`，内边距在 `:700-702`。
11. ✅ **Tree 的自定义渲染能力已确认**：delegate 是**闭包**而不是 trait —— `Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem`（`gpui-component-0.6.6/src/tree.rs:18-23,41-43`）；`TreeItem::new(id, label)`（`gpui-base-0.6.6/src/tree.rs:99`）、`.child/.children/.expanded/.disabled`（`:111,116,121,126`）、`TreeEntry`（`:50`）+ `new(item, depth)`（`:56`）+ `item/depth/is_root/is_folder/is_expanded/is_disabled`（`:61-86`）；`TreeState::new(cx)`（`:197`）、`set_items`（`:214`）、`set_selected_index`（`:225`）、`selected_entry`（`:248`）、`entry(ix)`（`:252`）、`scroll_to_item`（`:260`）、`reveal_item`（`:268`）、`focus`（`:280`）、`TreeEvent`（`:93`）。右键菜单入口 `Tree::context_menu(f)`（`gpui-component-0.6.6/src/tree.rs:55`）。
12. ✅ **`Placement` 已确认是四边枚举**：`Placement{Top, Bottom, Left, Right}`（`gpui-base-0.6.6/src/geometry.rs:12-21`）→ 可直接映射 Lithe `sheet.tsx` 的 `side`。
13. ✅ **`HighlightThemeStyle` 已逐字段核对**：`editor.*` 7 键 + `StatusColors` 15 键 + `syntax.*` **41 键**（`gpui-component-0.6.6/src/highlighter/registry.rs:315-346,436-457,15-57`）。§3.4(4) 已据此重写。
14. ✅ **写死的 px 常量已基本穷举**：见 §2.17 的完整表（`TITLE_BAR_HEIGHT 34`、Tab 四变体高度、Table 行高与单元格 padding、Dialog 448、Sheet 350、Notification 382、Sidebar 255/48、Separator 1、Form label 140、Slider、Carousel 等）。
15. ⚠️ **`gpui-base` 的 `SemanticThemeTokens` 是否被 gpui-component 组件真正消费 —— 仍未逐个确认**。已确认的是：`Theme::spacing_tokens()` 直接返回默认值、**不读主题**（`theme/mod.rs:482-484`）；`RadiusTokens` / `ColorTokens` / `TypographyTokens` / `ShadowTokens` 由 `Theme` 字段派生（`theme/mod.rs:414-501`）。**哪些组件已改用 `semantic_tokens`、哪些还在读 legacy `ThemeColor` 字段，未查清。** 这决定了「改 `Theme` 字段还是改 `semantic_tokens` 才生效」。
16. **`window.rem_size()` 与 `add_fonts` 的交互未核对** —— 设置 `Theme::font_size` 后自定义字体是否需要在 `px` 层面另行缩放，**未查清**。
17. 🆕 **`gpui-base-0.6.6/src/sizing.rs` 不存在**（`Size` 阶梯只在 `gpui-component/src/sizing.rs`）；`Sizable::medium()` **不存在**（只有 `xsmall/small/large`，默认即 Medium，`sizing.rs:178-202`）。
18. 🆕 **`ThemeRegistry` 没有 `load_themes` / `set_theme`** —— 只有 `load_themes_from_str`（`theme/registry.rs:151`）；`reload_themes`（`:226`）/`reload`（`:238`）都是私有。换肤的官方路径是：自定义 JSON 里置 `"is_default": true`（`theme/schema.rs:40`，`reload` 据此写 `default_themes`，`registry.rs:275-278`）后调 `Theme::change`，或直接替换 `Theme::global_mut(cx).light_theme / dark_theme`。
19. 🆕 **`AlertDialog` 没有严重度方法**（`dialog/alert_dialog.rs:79-260` 的 `pub fn` 中无 `info/warning/success/error`）；`window_ext.rs:44-49` 的 `alert.warning()` 文档示例**已过时**。
20. 🆕 **`Sidebar` 没有 `impl Sizable`**（`sidebar/mod.rs`）；宽度靠 `DEFAULT_WIDTH = px(255.)` / `COLLAPSED_WIDTH = px(48.)`（`:27,28`）与 `.side/.collapsible/.collapsed`（`:254,264,270`）控制。
21. 🆕 **`PopupMenu::new` 是 `pub(crate)`**（`menu/popup_menu.rs:334`）→ 外部只能通过 `ContextMenuExt::context_menu` / `DropdownMenu::dropdown_menu` 回调获得菜单对象。

### 5.4 仍未做（工作量项，不是疑点）

- **205 个图标的逐项映射未做**（`windows/tauri/src/ui/icons.tsx` 205 个 `export const`；资源表 `windows/tauri/src/ui/icons/idea-assets.generated.ts`）。需要单独出一份 `05-icons.md`。
- **`src/ui/command.tsx` 的 22 个导出到 gpui 的逐项重构方案未细化**（§4.2 只给了组合思路）。
- **Dock（`gpui_component::dock`）能否直接承载 Lithe 的可拖拽工作台布局未评估**。`gpui-base` 的 `dock` 模块很完整（`DockArea/DockAreaState/DockLayout/DockPlacement/PaneNode/PaneTree/PanelRegistry/TabGroup` 等 re-export 自 `dock/mod.rs:48-55`，外观用 `DockSkin`，`dock/mod.rs:122`），值得在下游「工作台布局」调研里单独评估。

### 5.5 引用文件清单（本次实际读取）

**Windows 侧**：`windows/tauri/src/styles.css`、`styles/{theme.css, base.css, utilities.css, scrollbars.css, syntax-tokens.css, rendered-code-tokens.css, window-transparency.css}`、`extensions/themes/{builtin/lithe.json, theme-file.ts, theme.types.ts, theme-schema.ts, theme-registry.ts, theme-initializer.ts, default-theme.ts, syntax-token-colors.ts}`、`utils/control-variants.ts`、`ui/{button,badge,chrome,item,empty,spinner,sonner,control-variants 相关,tooltip,scroll-area,separator,table,card,dialog,alert-dialog,sheet,drawer,toast,progress,skeleton,tabs,tab-bar,select,combobox,dropdown,dropdown-menu,context-menu,popover,menubar,search,kbd,command,accordion,collapsible,avatar,alert,breadcrumb,pagination,resizable,marker,icons,number-input,input-group,calendar,carousel,hover-card,navigation-menu,aspect-ratio,direction,input-otp,textarea,switch,checkbox,radio-group,slider,toggle,toggle-group,button-group,field,label,input-handler,inspector,sidebar,attachment,bubble,chart,message,message-scroller,thinking-orb}.tsx`（逐文件清单见 `04-components-raw.md`）、`features/file-explorer/{lib/file-tree-row.ts, styles/file-explorer-tree.css, components/file-explorer-viewport.tsx}`、`features/sidebar/{styles/sidebar-tree.css, components/sidebar-tree.tsx}`、`features/settings/lib/{ui-font-size.ts, ui-preferences.ts, settings-normalization.ts, appearance-bootstrap.ts}`、`features/settings/config/typography-defaults.ts`、`features/layout/contexts/toast-context.tsx`、`package.json`。
**gpui 侧**：`gpui-component-0.6.6/src/{theme/{mod,color,theme_color,schema,registry,motion}, sizing.rs, root.rs, window_ext.rs, lib.rs, styled.rs, highlighter/{mod,registry}.rs, title_bar.rs, status_bar.rs, button/{button,toggle,button_icon,button_group,dropdown_button}.rs, tab/{tab,tab_bar}.rs, alert.rs, badge.rs, tag.rs, tooltip.rs, separator.rs, kbd.rs, label.rs, link.rs, empty.rs, marker.rs, shimmer.rs, spinner.rs, skeleton.rs, switch.rs, checkbox.rs, radio.rs, slider.rs, accordion.rs, collapsible.rs, breadcrumb.rs, pagination.rs, rating.rs, hover_card.rs, group_box.rs, description_list.rs, notification.rs, sheet.rs, list/{list,list_item,delegate,loading,separator_item}.rs, table/{table,data_table,state,delegate,column}.rs, select.rs, combobox.rs, searchable_list/delegate.rs, command/{mod,command,state,item}.rs, dialog/{mod,dialog,header,title,description,content,footer,alert_dialog}.rs, progress/{mod,progress,progress_circle}.rs, input/{input,textarea,editor,otp_input,number_input,group,content_type,state}.rs, form/{form,field}.rs, scroll/{mod,scrollable}.rs, avatar/{avatar,avatar_group}.rs, menu/{mod,context_menu,dropdown_menu,popup_menu,app_menu_bar}.rs, native_menu/mod.rs, sidebar/{mod,menu,header,group,footer}.rs, tree.rs, virtual_list.rs, chart/mod.rs, plot/mod.rs, setting/{settings,page,group,item,fields/mod}.rs, carousel/{carousel,state}.rs, message.rs, message_scroller.rs, color_picker.rs, scheduler 无关项, theme/default-theme.json}`、`gpui-base-0.6.6/src/{theme_tokens.rs, tree.rs, geometry.rs, virtual_list.rs, resizable/{mod,panel,resize_handle}.rs, toggle_group.rs, input/*, text/*, lib.rs}`、`gpui-kit-0.6.6/src/lib.rs`、`gpui-pre-0.3.6/src/styled.rs`、`gpui-pre-macros-0.3.6/src/styles.rs`。
