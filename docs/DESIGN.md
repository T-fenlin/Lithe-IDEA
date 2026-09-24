# Lithe 设计语言

本文定义 Lithe 的设计语言：它的取向、token 体系、两端的实际差异，以及新界面必须遵守的规范。

> **这不是数值真源。** 颜色与尺寸的权威定义在代码里，本文只记录语言与规则，并指向真源。改数值请改代码与共享 fixture，然后让校验脚本通过。
>
> 目标界面的规范性真源是 `gpui-kit-design-guides`（GPUI Kit 应用必读的 Design Guides）。本文不替代它，两者冲突时以 Design Guides 为准。

## 先说结论

1. Lithe 的视觉取向是 **IntelliJ IDEA New UI**：紧凑行高、连续的项目树行、克制的蓝作为唯一强调色。这一取向在 macOS 侧有直接注释依据（`macos/Sources/Lithe/Theme/LitheTheme.swift:407-408`）。
2. 设计语言现在有**三个真源，但只有一个被强制**：语法高亮有共享 fixture 和校验脚本强制两端一致；外观 token（表面、边框、文字、交互、语义色）两端各写一套，**命名、取值、表示法都不同**。
3. 因此新界面最大的设计风险不是"没有规范"，而是**把两套不同语义的 token 按同名对照**。最典型的陷阱是 `accent`：macOS 的 `accent` 是主强调色（≈`#3574F0`），而 Windows 的 `accent` 是悬停底色。
4. 统一设计语言的正确做法不是再写一份约定，而是**复用语法高亮已经跑通的模式**：共享 fixture 作唯一真源，校验脚本强制，平台允许"子集 + 声明式回退"。

## 一、设计取向

- **参考系**：IntelliJ IDEA New UI。项目树使用连续行（行距 1pt）、4/12pt 内边距、8pt 选中弧（即 4pt 圆角），依据是 `LitheTheme.swift:407-408` 的注释。
- **桌面优先**：原生窗控（macOS 交通灯、Windows 自绘标题栏）、密集信息、持久导航、可调整面板。视觉基线截图见 `docs/visual-qa/`，其中 `01-java-editor-project-tree.png` 呈现了典型工作台：标题栏 + 左侧活动栏图标轨道 + 停靠面板 + 多标签编辑器 + 底部状态栏。
- **强调色克制**：只用一个蓝（`#3574F0`）承担强调、选中、焦点、链接之外的多数语义；成功/警告/错误是独立的语义色，不参与强调。
- **主题生态两端不对等**：
  - macOS 有 **3 套应用配色主题**：`lithe`、`codex`、`linear`（`macos/Sources/Lithe/Models/Settings/AppSettings.swift:609-612`）。
  - Windows 有 **12 个内置主题家族**（`windows/tauri/src/extensions/themes/builtin/`：ayu、catppuccin、christmas、contrast-themes、dracula、github、lithe、nord、one、solarized、tokyo-night、vitesse），并支持导入自定义主题文件。

## 二、真源与校验

| 子系统 | 真源 | 是否被强制 | 说明 |
| --- | --- | --- | --- |
| 语法高亮 | `shared/fixtures/editor-themes/lithe-v1.json` + `shared/contracts/editor-syntax-theme-v1.schema.json` | **是**，`scripts/verify-shared-contracts.sh` 校验两端资源 | 唯一做到契约化的子系统 |
| macOS 外观 token | `macos/Sources/Lithe/Theme/LitheTheme.swift` | 否 | 自有 `Palette`，无跨端契约 |
| Windows 外观 token | `windows/tauri/src/extensions/themes/builtin/lithe.json` + `windows/tauri/src/styles/theme.css` | 否 | 自有 39 键 + CSS 变量，无跨端契约 |

**结论**：外观 token 的跨端一致性目前完全靠人读代码维持。新界面要统一设计语言，第一步就是把外观 token 也纳入"共享 fixture + 校验脚本"的模式。

## 三、Token 分类

### 3.1 macOS：`Palette` 38 字段

定义在 `LitheTheme.swift:44-82`，按用途分为五组：

| 组 | 字段 |
| --- | --- |
| 表面与层次 | `window`、`titlebar`、`toolHeader`、`toolHeaderInactive`、`sidebar`、`editor`、`raised`、`notification`、`popupBackground`、`activeTabBackground` |
| 边框与分隔 | `divider`、`panelBorder`、`inputBorder`、`inputFocusBorder`、`guide`、`activeGuide` |
| 文字 | `primaryText`、`secondaryText`、`tertiaryText`、`toolWindowText`、`toolWindowSelectedText` |
| 交互状态 | `selection`、`subtleSelection`、`hoverBackground`、`pressedBackground`、`tabUnderline` |
| 语义色 | `accent`、`runAction`、`success`、`warning`、`error`、`skill`、`link`、`badgeBackground`、`diffInformationBackground`、`diffInformationText`、`inputBackground`、`popupShadow` |

要点：

- **表示法是 0–1 的 sRGB 浮点**，例如 `window` 明色 `(0.933, 0.945, 0.961, 1)`、暗色 `(0.157, 0.161, 0.173, 1)`（`LitheTheme.swift:185`）。
- `selection` 与 `accent` 的**明色值相同**（`(0.208, 0.455, 0.941, 1)`，即 `#3574F0`），暗色 accent 更亮（`(0.31, 0.58, 0.98, 1)`，`:198, :214`）。
- 暗色下的悬停/按下/徽标/次级文字使用**白色透明叠加**（`(1,1,1,0.055)` / `0.095` / `0.10` / `0.50`），而不是独立灰阶（`:195-196, :208, :210`）。
- 只有 `Accent/Codex/Linear` 三套中的 Lithe 硬编码了全部 38 个值（`:184-223`），另两套只固定 7 个基色、其余由公式派生（`:130-174`）。
- **明暗外观不由主题运行时决定**：`AppColorTheme` 只管配色家族，明暗由每个视图用 `NSAppearance.bestMatch([.aqua, .darkAqua])` 解析（`:368-374`）。写文档或写代码时不要把两者混为一谈。
- 对外只暴露 13 个 `ResolvedColorToken`（`:229-243`），其余是内部实现。

### 3.2 Windows：39 个颜色键

真源是 `windows/tauri/src/extensions/themes/builtin/lithe.json`，按用途分为五组：

| 组 | 键 |
| --- | --- |
| 基础层 | `background`、`surface`、`foreground`、`muted-foreground`、`subtle-foreground`、`border` |
| 交互与强调 | `accent`、`selected`、`selection`、`primary` |
| 光标 | `cursor`、`cursor-vim-normal`、`cursor-vim-insert` |
| 语义色 | `destructive`、`success`、`warning`、`info` |
| 领域色 | `git-modified`、`git-modified-staged`、`git-added`、`git-deleted`、`git-untracked`、`git-renamed`、`terminal-*`（16 个） |

要点：

- **表示法是 hex 与 rgba 混用**：`background` 明 `#ffffff` / 暗 `#1e1f22`，`selection` 明 `rgba(53, 116, 240, 0.2)` / 暗 `#214283`（`lithe.json:12, :20, :79, :87`）。
- 主题文件格式见 `theme-schema.ts`：一个文件可含多个主题，每个主题有 `id`、`name`、`appearance`（`dark`/`light`）与 `colors`；**必填颜色键 9 个**（`background`、`surface`、`foreground`、`muted-foreground`、`subtle-foreground`、`border`、`accent`、`selected`、`primary`）。`version` 是可选**字符串**，与契约里的数字版本不是同一套版本化。
- 尺寸不在这份 JSON 里，而在 `windows/tauri/src/styles/theme.css` 的 CSS 变量中（见第五节）。

### 3.3 表示法差异（统一时必须先解决）

| 维度 | macOS | Windows |
| --- | --- | --- |
| 颜色表示 | 0–1 sRGB 浮点 | hex / rgba 字符串 |
| 配色家族 | 3 套（lithe / codex / linear） | 12 个家族 + 自定义导入 |
| 明暗切换 | 逐视图 `NSAppearance` 解析 | 主题数组里的 `appearance` 字段 + `data-theme-type` |
| 尺寸定义 | Swift 常量（`Metrics`、`Commit`、各 `*LayoutMetrics`） | CSS 变量（`:root`）与 rem 阶梯 |

## 四、两端分歧（新 UI 的最大设计风险）

### 4.1 语义错位：三个必须记住的陷阱

1. **`accent` 不是同一个东西。** macOS 的 `accent` 是主强调色（`#3574F0`）；Windows 的 `accent` 是悬停底色（明 `#edf3ff` / 暗 `#393b40`）。macOS 的 `accent` 对应的是 Windows 的 **`primary`**。
2. **`selection` 的形态不同。** macOS 是不透明蓝（明暗都是 `#3574F0`）；Windows 明色是 20% 透明蓝 `rgba(53,116,240,0.2)`、暗色是深蓝 `#214283`。直接互抄会出现"选中块变成实心蓝"或"选中看不见"。
3. **`success` / `warning` / `error` 三对值全不同**，不能跨端复用一个色表：

| 语义 | macOS 明 / 暗 | Windows 明 / 暗 |
| --- | --- | --- |
| 成功 | `(0.105,0.545,0.235)` / `(0.28,0.72,0.39)` | `#27864f` / `#57965c` |
| 警告 | `(0.690,0.410,0.035)` / `(0.91,0.63,0.20)` | `#a86400` / `#d6ae58` |
| 错误 | `(0.780,0.175,0.175)` / `(0.92,0.33,0.33)` | `#cf3f4f` / `#db5c5c` |

### 4.2 只在单端存在的 token

- **仅 macOS**：窗口层（`window`、`titlebar`、`toolHeader*`、`raised`、`notification`）、侧栏与编辑器面（`sidebar`、`editor`）、鼠标悬停态（`hoverBackground`、`pressedBackground`、`subtleSelection`）、标签页（`activeTabBackground`、`tabUnderline`）、弹出层（`popupBackground`、`popupShadow`）、输入层（`inputBackground`、`inputBorder`、`inputFocusBorder`）、diff 信息色、`skill`、`link`、`guide`、`activeGuide`。
- **仅 Windows**：光标三色（`cursor`、`cursor-vim-*`）、`info`、`git-*`（6 个）、`terminal-*`（16 个）、以及 CSS 层的 `--border-strong`、`--popover`、`--muted`、`--ring`、标签栏底色（`tab-bar-bg` / `tab-active-bg` / `tab-hover-bg`）、5 组阴影、`lithe-glass-*` / `lithe-chrome-*`。

### 4.3 异名同义（**未找到桥接代码，属推断**）

`divider` ↔ `border`、`panelBorder` ↔ `--border-strong`、`inputBorder`/`inputFocusBorder` ↔ `border` + `--ring`、`popupBackground` ↔ `surface`、`hoverBackground`/`pressedBackground` ↔ `accent`/`selected`、`activeTabBackground` ↔ `tab-active-bg`。

这几组语义大概率相同，但仓库里**没有映射代码**，因此在新界面里必须显式定义映射，不能假设它们等价。

## 五、排版、间距与密度

### 5.1 macOS

| 类别 | 值（`LitheTheme.swift`） |
| --- | --- |
| 字体 | UI 14 / 小字 12（`:376-377`）、代码 `JetBrainsMono-Regular` 13（`:378`） |
| 行高 | 编辑器 `lineHeightMultiple` 1.2、`baselineLift` 1.5（`:379-380`） |
| 行与列表 | `rowHeight` 24、`treeRowHeight` 27、项目树行距 1、树图标 16、树字号 13.5（`:405-414`） |
| 项目树内边距 | 垂直 4 / 水平 12、选中圆角 4（`:409-412`） |
| 框架高度 | 标签 34、工具栏 40、工具窗头 30、状态栏 24（`:415-418`） |
| 圆角 | 通用 5、控件 6、上下文菜单 9、弹窗 10（`:419-422`） |
| 提交面板 | 工具栏 37、面板内边距 10、名单字号 12、消息字号 13、动作图标 14、紧凑按钮 24（`:427-442`） |
| 其他 | 活动栏 38 宽 / 图标 30×30、右活动栏 40、面板圆角 10、分隔条厚 5、差异视图多组 24/27/34/47 常量 |

### 5.2 Windows（全部在 `styles/theme.css`）

| 类别 | 值 |
| --- | --- |
| 字体 | UI `13px`（`:112`），族为 `Microsoft YaHei UI, Segoe UI, …`（`:106-108`）；代码族 `Geist Mono, …`（`:109-111`） |
| 行高 | `--leading-row` 1.35（`:4`） |
| 框架高度 | 标题栏 2.5rem、页脚 1.5rem、面板头 2.25rem、标签栏 = 面板头、标签 1.75rem、侧栏头 2rem（`:118-124`） |
| 间距 | 工作台 4px、chrome 2/4/6px、行内 8px（`:125-132`） |
| 圆角 | `--radius` 8px 并有 ×0.6…×2.6 的阶梯（`:134`、`:6-12`）；chrome 圆角单独 4px（`:133`） |
| 密度 | 默认与 `comfortable` 两档覆盖（`:188-200`） |

### 5.3 已对齐与未对齐

| 项 | macOS | Windows | 结论 |
| --- | --- | --- | --- |
| 标题栏/工具栏高度 | 40 | 2.5rem = 40px | **已对齐** |
| 状态栏/页脚高度 | 24 | 1.5rem = 24px | **已对齐** |
| 基础字号 | 14 | 13px | 差 1px |
| 标签页高度 | 34 | 1.75rem = 28px | 差 6px |
| 通用圆角 | 5（控件 6） | 8（chrome 4） | 基线不同 |
| 行高 | 1.2 | 1.35 | 不同 |
| 密度档位 | 无 | 有 `comfortable` | Windows 多一档能力 |

## 六、语法高亮：唯一已契约化的子系统（统一设计语言的模板）

契约把角色分成两组，共 27 个：

- **20 个 palette 角色**（必填）：attribute、boolean、comment、constant、function、invalid、jsx、jsx-attribute、keyword、null、number、operator、property、punctuation、regex、string、tag、text、type、variable。
- **7 个 fallback 角色**（必填）：annotation、documentationComment、field、functionCall、functionDeclaration、parameter、typeParameter。
- 颜色必须是 `#RRGGBB` 或 `#RRGGBBAA`。

`scripts/verify-shared-contracts.sh` 强制这些不变量：

```zsh
syntax_theme_fixture="shared/fixtures/editor-themes/lithe-v1.json"
macos_syntax_colors="macos/.../SyntaxHighlighting/color-mappings.json"
windows_lithe_theme="windows/tauri/src/extensions/themes/builtin/lithe.json"

windows_syntax_roles = palette_roles - %w[invalid text]   # Windows 必须恰好覆盖 18 个
abort "Windows #{appearance} #{role} differs from shared palette" unless syntax.fetch(role).casecmp?(expected)
abort "macOS syntax roles differ from the shared subset" unless macos_defaults.keys.sort == macos_role_sources.keys.sort
abort "cyclic syntax role fallback for #{role}" if trail.include?(role)
```

即：**Windows 必须是共享 palette 的精确子集（18 个，缺 `invalid` 与 `text`）且取值逐项相等；macOS 允许只覆盖自己的子集（20 个，含 7 个 fallback 角色、不含 `invalid`），但必须自身自洽并声明回退链，回退图不允许成环。**

实测印证：两端语法值确实一致 —— Windows 明色 `comment #68717d`、`keyword #b83280`、`string #287d3c`，暗色 `keyword #cf8e6d`、`string #6aab73`，与 macOS `color-mappings.json` 的对应角色逐项相同。

**为什么这是模板**：它同时给出了三件新界面需要的东西 ——（1）一个语言中立的真源；（2）一个可执行的强制校验；（3）平台被允许"只实现子集"，但必须显式声明回退，而不是各自发明。外观 token 应当照此办理。

注意：`shared/contracts/editor-syntax-theme-v1.schema.json` 本身只有 `shared/README.md` 引用，**没有代码在运行时读它**；强制来自 fixture 与校验脚本。文档读者不要误以为应用启动时会加载这个 schema。

## 七、交互状态

| 状态 | macOS | Windows |
| --- | --- | --- |
| 悬停 | `hoverBackground`（明 `(0.949,0.953,0.961)`，暗白色 5.5%） | `bg-accent`（明 `#edf3ff` / 暗 `#393b40`） |
| 按下 | `pressedBackground`（明 `(0.882,0.890,0.906)`，暗白色 9.5%）；主按钮额外 opacity 0.78 | `bg-selected` + `active:scale(--app-press-scale)`，而该值当前为 **1**（即不缩放） |
| 选中 | `selection`（实心蓝）/ `subtleSelection` / `activeTabBackground` | `selected` / `selection`（半透明蓝） |
| 焦点 | `inputFocusBorder`（accent 90% 明 / 85% 暗） | `ring-2 ring-primary/20~25` |
| 禁用 | **未定义禁用视觉**，仅不给手型光标，视觉靠系统 `.disabled` | `opacity-50` + `pointer-events-none` |

两个需要修正的地方：

1. **macOS 缺少禁用态的设计**。这是设计语言上的真实缺口 —— 同一个禁用按钮在两端外观会不一致。新界面必须显式定义禁用态（Windows 的 `opacity-50` 可作起点，但应对照对比度要求复核）。
2. **`--app-press-scale` 为 1 意味着按压缩放事实上不存在**。要么删掉这条机制，要么给它一个非 1 的值，不要让一个恒等于 1 的变量留在语言里。

## 八、动效

- **Windows**（`theme.css:135-139`）：`--app-duration-fast` 150ms、`--app-duration-normal` 200ms、`--app-ease-smooth` `cubic-bezier(0.22,1,0.36,1)`、`--app-ease-in-out` `cubic-bezier(0.66,0,0.34,1)`；`reduced-motion` 支持在 `styles/utilities.css:131`。
- **macOS**：标签页 `interactiveSpring(response 0.22, dampingFraction 0.86, blendDuration 0.10)`、格式选择器 `.easeOut(0.18)`、通知 `.easeOut(0.14/0.10)`，另有一组 0.12/0.14/0.18 的短过渡；全部受 `accessibilityReduceMotion` 控制。
- **统一建议**：新界面采用"两档时长 + 两条曲线"的 Windows 模型（150/200ms、ease-smooth/ease-in-out），因为它可枚举、可测试；macOS 的 spring 参数保留给标签页这类需要连续性的动效。

## 九、图标与视觉基线

- 图标默认尺寸 14、应用标志 42（`LitheIcons.swift:569`、`:549`）。
- 视觉基线截图在 `docs/visual-qa/`，共 7 张：`00-welcome-projects`、`01-java-editor-project-tree`、`07-git-history-graph`、`08-git-diff-green-state`、`09-search-everywhere-results`、`13-spring-boot-usages`、`14-spring-boot-run-configuration`。
- **编号不连续**（缺 02–06、10–12），目录内也没有索引文件。新界面若要做视觉回归，需要先补齐编号或建立索引。

## 十、新界面（GPUI Kit）必须遵守的规范

**复刻的界面规格以 macOS 端源码为准**（2026-09-24 定）：逐区域的组成、度量与"元素 → 组件"对应表在 `gpui/UI-MAP.md`，执行计划在 `gpui/PLAN.md`；本节与第十一节的 token 契约是那份文档的前置条件。Windows 端与产品截图此后只作旁证。

以下取自 `gpui-kit-design-guides`，是下限而不是全部；做任何屏幕前应完整阅读该指南。

1. **桌面先于 Web 惯例**：键盘可达、窗口装饰、菜单、密集数据视图、可调整区域、持久导航。
2. **Tokens before values**：代码里不出现裸 hex 或 `rgb(...)`，一律使用语义 token；指南里出现的间距数字是默认刻度，不是要逐字复制的字面值。**这条正好呼应本文第二节的结论：外观 token 必须先契约化，新界面才可能只引用 token。**
3. **`Button` 与 `Link` 分工**：应用内命令一律 `Button`，次要动作用 `ghost` 或 `outline`；`Link` 只用于外部 URL 与邮箱。
4. **状态必须可见**：悬停、焦点、选中、禁用、加载、校验、危险操作各自需要一致且可区分的处理（对照本文第七节：禁用态是当前缺口）。
5. **浮层**：Esc 关闭最上层并把焦点还给触发者。
6. **文案**：说出对象与动词，例如 `Delete "Roadmap"?` 配 `Delete` 按钮，而不是 `Are you sure?` 配 `OK`。

## 十一、迁移映射（现有 → 目标语义 token）

新界面应采用一套语义 token，并显式声明它从哪一端取值。推荐映射：

| 目标 token | macOS 来源 | Windows 来源 | 处置建议 |
| --- | --- | --- | --- |
| `surface.base` | `window` | `background` | 取 Windows `background`（明色纯白更符合 Web 化深色/浅色基线），但需复核与 macOS `window` 的观感差异 |
| `surface.raised` | `raised` | `surface` | 两端值已接近，统一命名即可 |
| `text.primary` | `primaryText` | `foreground` | 值几乎相同，直接统一 |
| `text.secondary` | `secondaryText` | `muted-foreground` | 需重新取值，两端语义粒度不同 |
| `text.tertiary` | `tertiaryText` | `subtle-foreground` | 同上 |
| `border.default` | `divider` | `border` | 统一命名，取值需重新定标 |
| `border.strong` | `panelBorder` | `--border-strong` | Windows 已用 color-mix 派生，可作为统一方案 |
| `accent.primary` | `accent` | `primary` | **务必确认语义再合并**，这是最易出错的一处 |
| `accent.subtle` | `hoverBackground` | `accent` | 命名互换，需一次性改到位 |
| `state.selected` | `selection` | `selected` | 形态不同（实心 vs 半透明），需定一个 |
| `state.danger` | `error` | `destructive` | 取值不同，需定一个 |
| `state.success` / `state.warning` | `success` / `warning` | `success` / `warning` | 取值不同，需定一个 |

未列入的 `git-*`、`terminal-*`、`cursor-*` 建议**保留在 Windows 命名空间内**，因为 macOS 侧当前没有对应概念，强行统一会凭空发明语义。

## 十二、未决问题

1. **外观 token 是否建立共享 fixture？** 本文建议照语法高亮的模式做，但这会新增一个契约与一条校验，需要先确认。
2. **禁用态怎么定义？** macOS 目前没有禁用视觉，需要一次设计决定（含对比度要求）。
3. **密度档位**：Windows 有默认与 `comfortable` 两档，macOS 没有。新界面要几档？
4. **明暗模型**：macOS 的"逐视图 `NSAppearance` 解析"与 Windows 的"主题数组 + `appearance` 字段"是两种模型，新界面应当只保留一种。
5. **视觉基线编号**：`docs/visual-qa/` 的编号缺口是否需要补齐，并建立索引。

## 参考

- `macos/Sources/Lithe/Theme/LitheTheme.swift` —— macOS token、字体、`Metrics`、`Commit`
- `windows/tauri/src/extensions/themes/builtin/lithe.json` —— Windows 颜色真源
- `windows/tauri/src/extensions/themes/theme-schema.ts` —— 主题文件格式
- `windows/tauri/src/styles/theme.css` —— Windows 尺寸、圆角、时长、缓动
- `shared/contracts/editor-syntax-theme-v1.schema.json`、`shared/fixtures/editor-themes/lithe-v1.json` —— 语法高亮契约
- `scripts/verify-shared-contracts.sh` —— 跨端一致性强制入口
- `docs/visual-qa/` —— 视觉基线截图
- `.agents/skills/develop-lithe/SKILL.md` —— 仓库所有权与边界
