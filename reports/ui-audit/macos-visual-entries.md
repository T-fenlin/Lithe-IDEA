# 矩阵 macOS 视觉条目 × GPUI 侧对应物判定

基线 `3085f5d1`，分支 `feat/menu-bar-opaque`。检索范围 `rust/lithe-ui/src`（858 个 `.rs`）与
`rust/lithe-gpui`（123 个 `.rs`）。判定只回答「有无对应物」，不判视觉是否等价。

## 0. 先纠一个口径：视觉条目是 6 条，不是 5 条

任务书列的 5 条是 `macos-shared-product-dropdowns`、`macos-shared-scrollbar-style`、
`macos-shared-hover-tooltip-style`、`macos-bundled-default-ui-font`、`project-sidebar-*`。

实际把 `shared/platform-feature-matrix/features/*.json` 全量扫一遍（读每个 JSON 的
`macos.implementationStatus`），`macos=implemented` 且 `windows=platform-specific` 的条目共
**9 条**，其中属视觉层的 **6 条**：

| # | id | area | group |
|---|---|---|---|
| 1 | `macos-shared-product-dropdowns` | 界面 | 全局共享控件 |
| 2 | `macos-shared-hover-tooltip-style` | 界面 | 全局共享控件 |
| 3 | `macos-bundled-default-ui-font` | 界面 | 全局字体 |
| 4 | `macos-shared-scrollbar-style` | 工作台 | 界面样式 |
| 5 | `project-sidebar-header-icons` | 工作台 | 项目侧边栏 |
| 6 | `project-sidebar-tree-disclosure-icons` | 工作台 | 项目侧边栏 |

任务书的 `project-sidebar-*` 是 **2 条**（header-icons、tree-disclosure-icons），不是 1 条。
另 4 条 `macos=implemented`/`windows=platform-specific` 属行为或含视觉成分：
`editor-macos-ime-presentation`、`editor-wallpaper-overview-ruler`、
`git-commit-sidebar-toolbar-icons`、`terminal-native-palette`。

> `platform-specific` 语义见 `shared/platform-feature-matrix/metadata.json`：该能力刻意只属一端。
> 所以「GPUI 没有」在这 6 条上**不自动等于缺口**——缺口只在「IDEA 真源有、GPUI 该有却没有」时成立。

## 1. 逐条判定

### 1.1 `macos-shared-product-dropdowns` — 有对应实现，但与 IDEA 真源冲突

- macOS 证据（JSON `macos.evidence`）：`LitheDropdown.swift`、`LitheContextMenu.swift`、
  `LitheSettingsControls.swift`、`LitheTheme.swift` 等 26 条。
- GPUI 侧对应物：`rust/lithe-ui/src/menus/menu.rs`（484 行）、`src/menus/draw.rs`、
  `src/overlays/popover.rs`、`src/forms/select.rs`、`src/forms/combobox.rs`、`src/lists/select.rs`。
  判定：**有对应实现**。
- **冲突点**：该条目 capability 明文要求「直接打开且无系统弹跳动画」。而 lithe-ui 的菜单
  **开时有动画**：`menus/menu.rs:361` 定义 `Animation::new(motion::duration(motion::FAST, cx))`，
  `:451` 面板入场 `root.opacity(t)`，`:467-470` 子菜单入场再叠 `ml(motion::NUDGE * (t - 1.0))`
  位移。`motion::FAST = 120ms`（`motion/curve.rs:7`），`motion::NUDGE = px(4.0)`（`curve.rs:13`）。
  `motion::duration`（`curve.rs:19-23`）只在 `theme.reduced_motion` 时降为 1ms——
  **默认路径就是有动画的**，与「无弹跳」要求方向相反。

### 1.2 `macos-shared-hover-tooltip-style` — 有对应实现，但取色真源不同

- macOS 证据：`LitheTheme.swift`、`WorkbenchHoverTooltip.swift`、`WorkbenchHoverTooltipTests.swift`。
- GPUI 侧对应物：`rust/lithe-ui/src/primitives/tooltip.rs`（226 行，`Tooltip` / `TooltipTrigger`
  / `HelpTooltip`），宿主 20+ 处消费（`editor_view.rs:2570`、`explorer_view.rs:459/470/477/484`、
  `git/changes_view.rs:620/681/1020`、`git/log_view.rs:411/421/441/450/631`、
  `run/pane.rs:512/521/530/539`、`settings/dialog.rs:919`、`terminal/terminal_view.rs:107/406/418/458`）。
  判定：**有对应实现，且被宿主广泛使用**。
- **差异点**：条目 verification 写明 macOS 取值「深色背景/边框 `#33353B`、文字 `#D1D3D9`；
  浅色背景 `#FFFFFF`、边框 `#D1D3D9`、文字 `#000000`；共享 1pt 内边框、**4pt 圆角**、13pt 字号、
  左右 12pt/上 8pt/下 9pt 内边距」。GPUI 侧读的是组件库 token：
  `tooltip.rs:24-31` 用 `theme.tooltip_max_width()`、`px_2()`、`py_1()`、
  `theme.radius(Radius::Md)`、`colors.tooltip_bg`、`colors.tooltip_fg`、`text_size(TextSize::Sm)`。
  - `Radius::Md` = 6.0px（`theme/tokens.rs:487`）→ **6pt，不是 IDEA 的 4pt**。
  - `TextSize::Sm` = 12.0px（`tokens.rs:473`）→ **12pt，不是 IDEA 的 13pt**。
  - `px_2`/`py_1` 是 GPUI 固定档位（8px / 4px），与 IDEA 的 12/8/9 不等。
  - 且**没有内边框**：`surface()` 只设 `bg` + `shadow`，无 `border_1()`。
  - 取色走 `Palette::tooltip_bg`（`theme/palette.rs:254` 浅色 `0x181613`、`:334` 深色 `0x373533`），
    与 IDEA 的 `#FFFFFF`/`#33353B` 不同。
  判定：**有对应实现，但四组取值（圆角/字号/内边距/颜色）都与 IDEA 真源不一致**。

### 1.3 `macos-bundled-default-ui-font` — 有对应实现，字体族一致、字重档位差一档

- macOS 证据：`LitheTheme.swift`、`MacBundledFontRegistry.swift`、`macos/Resources/Fonts`、
  `BundledUIFontTests.swift`。verification 写明「安装包 Fonts 目录包含 **Inter 4.1 的 18 个静态 OTF**
  及许可、**JetBrains Mono 2.304 的 16 个静态字型**及许可」。
- GPUI 侧对应物：
  - `rust/lithe-ui/src/theme/mod.rs:119-121`：`font_family: "Inter"`、`mono_family: "JetBrains Mono"`、
    `font_size: 16.0`、`mono_font_size: 13.0`。
  - 资源：`rust/lithe-ui/assets/fonts/` **8 个字体文件 + 3 个许可**（`Inter-Italic/Medium/Regular/SemiBold.ttf`、
    `JetBrainsMono-Medium/Regular.ttf`、`IBMPlexSans-Regular.ttf` + 3 个 `*-LICENSE.txt`）。
  - 宿主另有一份：`rust/lithe-gpui/crates/settings/src/fonts.rs:11-28` 内嵌
    `JetBrainsMono-Bold/BoldItalic/Italic/Regular.ttf` 4 个，`BUILTIN_FONT_FAMILY = "JetBrains Mono"`；
    `rust/lithe-gpui/assets/fonts/` 5 个文件。
  - 实测 `macos/Resources/Fonts`：**Inter 19 个 `.otf`（含 Black/ExtraBold/ExtraLight/Thin 及各自 Italic）
    + JetBrains Mono 17 个 `.ttf` + JetBrainsMonoNerdFontMono 4 个**。
  判定：**字体族选择一致（Inter / JetBrains Mono），是三条里对得最齐的一条**。
- **缺口**：字重档位。macOS 交付 19 个 Inter 静态字型；GPUI 侧 `lithe-ui` 只有 4 个
  （Regular/Medium/SemiBold/Italic），**没有 Bold**。`BundledUIFontTests` 要求
  「Regular、Medium、SemiBold、Bold 和 Black 请求分别匹配真实 Inter 字型」——GPUI 侧
  Bold/Black 无真实字型可匹配。这是一处**可清点的资源缺口**。

### 1.4 `macos-shared-scrollbar-style` — 有对应实现，取值与 IDEA 真源不同

- macOS 证据：`LitheScrollViewChrome.swift`、`DiffScrollSynchronization.swift`、
  `LitheScrollBarStyleTests.swift`、note `2026-10-02-macos-shared-scrollbar.md`。
- GPUI 侧对应物：`rust/lithe-ui/src/layout/scroll.rs`（311 行，`ScrollArea` + `ThumbDrag`）
  与 `src/layout/scroll_aids.rs`；token 在 `theme/tokens.rs:166-172`
  （`scrollbar_thickness()` = 6.0px、`scrollbar_min_thumb()` = 24.0px）。
  判定：**有对应实现**。
- **差异点**（对照 note `2026-10-02-macos-shared-scrollbar.md`，依据 IDEA Community
  `c7f91397daa3a961b4e78bc634fe467a0a7d9ade`）：
  - note `:28`：「rail 为 **14pt** 加 2pt gap」。GPUI `scroll.rs:210-217` 的 track 宽是
    `thickness * 2.0` = **12px**，配 `pl(thickness * 0.5)` = 3px。**12 vs 14**。
  - note `:32`：「overlay thumb **7→10pt**（浅色）/ **9→12pt**（深色）胶囊」。
    GPUI thumb 用 `w(thickness)` = **6px**（`scroll.rs:172-173`）。**6 vs 7~12**。
  - note `:22`：「**1pt 内侧边框**、Mac 灰色透明度及透明轨道」。GPUI `scroll.rs:157-162`
    thumb 只有 `rounded_full().bg(idle)` 与 `hover` 变 `busy`，**无边框**；
    取色 `colors.fg.opacity(0.22)` / `0.42`（`scroll.rs:156`），
    而 note `:33` 明确 `IslandSchemeDark.xml` 覆盖为 `#FFFFFF26`/`#FFFFFF4D` 且
    「不能套用普通滚动区的灰色」。
  - note `:29-31`：Diff 用 **14pt 宽标记**、右侧 x=4/左侧 x=0、至少 2pt。
    GPUI 侧**未找到 Diff 专用滚动条实现**（`crates/git` 下无 `scroll` 相关自绘）。
  判定：**有对应实现，但厚度、胶囊宽、边框、Diff 标记四处都与 IDEA 真源不一致**。

### 1.5 `project-sidebar-header-icons` — 部分：`locate` 完全没有，`refresh` 无色变体

- macOS 证据：`ProjectSidebarView.swift` + `macos/Resources/IDEAIcons/expui/general/`
  的 `chevronDown.svg`、`locate.svg`、`refresh.svg`。
- GPUI 侧：宿主图标真源是 `rust/lithe-gpui/assets/icons/idea/**`（**297 个 `.svg`**
  + `extension.json` + `LICENSE-APACHE-2.0.txt` + `NOTICE.txt` + `SOURCES.md` = 301 个文件），
  经 `crates/shared/src/icons/idea.rs`（1141 行，**186 组 light/dark 常量**）暴露。
  实测能力点名的三张：
  - `locate.svg` → 全 `assets/icons/idea` 下 `locate*` **命中 0**；macOS 有明暗各一。
    且全 `rust/lithe-gpui` 检索 `LOCATE`/`ScrollFromSource`/`scroll_from_source`/`SelectIn`
    → **只命中 `settings/src/theme.rs` 的 `locate_theme` 与 `workbench/src/debug.rs` 的
    `relocate_breakpoints`，与图标无关 ⇒ 该功能入口不存在**。
  - `chevronDown.svg` → GPUI 有 `legacy/ui/chevronDown.svg`，与 macOS
    `expui/general/chevronDown.svg` **SHA-256 逐字节相同**（见 §1.6 的更正），可用。
  - `refresh.svg` → GPUI 只有 `actions/refresh.svg` 单张，**无 `_dark` 兄弟**；
    `idea.rs:1056-1060` 的 `REFRESH_ICON` 是 `has_dark: false`。
  判定：**部分**。清单完整性可证：把 `idea.rs` 里所有 `"icons/idea/..."` 字面量抽出来逐个
  `Test-Path`，**引用 372 条，缺失 0 条**——**已声明的图标全部有实物**，
  但能力点名的 3 张里 `locate` 完全缺、`refresh` 缺色变体。

### 1.6 `project-sidebar-tree-disclosure-icons` — 部分：行距可对上，缩进三值并存

- macOS 证据：`ProjectSidebarView.swift`、`LitheTheme.swift`、
  `expui/general/chevronRight.svg`、`expui/general/chevronDown.svg`。capability 要求
  「项目树展开与折叠箭头使用 IDEA Community 明暗 SVG，目录行保持默认箭头光标和 **24pt 连续行距**」。
- GPUI 侧对应物：`rust/lithe-ui/src/lists/project/row.rs`（项目树行）+
  `rust/lithe-ui/src/lists/tree/mod.rs`；箭头图标在 `idea.rs:291-292`（`CARET_RIGHT_ICON`，
  expui/general，有 dark）与 `:279-280`（`CARET_DOWN_ICON`，`legacy/ui/`）。
  判定：**部分**。
- **⚠️ 更正（原稿在此处判断错误，已按实测改写）**：
  原稿据目录名 `legacy/` 推断「GPUI 用 IDEA 经典 UI 图标、macOS 用 expui，是两代图标语言」。
  **这个判断被实测推翻**。`rust/lithe-gpui/assets/icons/idea/SOURCES.md:13-15` 明写：
  > 「放在 `legacy/` 下，保留旧包的相对路径……它们来自 Windows 前端
  > `windows/tauri/src/ui/icons/**`……**与 macOS 那份同属 intellij-community expui**，
  > 只是这一版更全（macOS 侧缺 `ui/` 那批通用字形与部分文件类型）。」

  实测佐证：GPUI `legacy/ui/` 与 macOS `expui/general/` **同名 36 个文件全部逐字节相同**；
  `legacy/ui/chevronDown.svg` 与 `expui/general/chevronDown.svg` 哈希一致。
  **所以 `legacy/` 只是「来源不同、放在 legacy/ 目录下」的 expui 美术，
  不是另一代图标语言。** 两边的图形语言是同一套。
- **真正的问题在别处：声明的生成器 `--check` 失败**。
  `SOURCES.md:3` 声明本目录由
  `node rust/lithe-gpui/tools/sync-macos-idea-icons.mjs --apply` 生成、「**不要手改**」。
  实测跑只读的 `--check`：
  ```
  macOS 图标文件夹：304 个 SVG + 2 个附件 → 逐字节复制进 gpui
  旧包（Windows 派生）：0 个 SVG
  idea.rs 常量：保留 63、删除 123
  清单定义：删除未被引用的 2 条
  --check 失败（129 条），例如 macOS 文件 dvcs/branchLabel.svg 没被复制进来……
  [exit code: 1]
  ```
  **129 条不一致**，且工具认为应「保留 63、删除 123」个 `idea.rs` 常量——
  与现状（186 组常量、122 个 legacy 文件）差得极远。
  **结论：`SOURCES.md` 描述的「生成 + 校验」链路对当前树不成立**，
  目录实际是手工/历史状态，不是该工具可复现的产物。这是一处**可复现的工具链缺陷**。
- **逐字节一致性实测**（严格按同相对路径比）：
  | 关系 | 条数 |
  |---|---|
  | 同相对路径且逐字节相同 | **162** |
  | 同相对路径但内容不同 | **13** |
  | GPUI 有、macOS 同路径没有（即 `legacy/` 那批） | **122** |
  13 条不同里，**7 条是「浅色档携带了 macOS 深色档的颜色」**：
  | 文件 | GPUI 浅色 | macOS 浅色 | macOS 深色 |
  |---|---|---|---|
  | `debugger/debug.svg` | `#253627,#57965C` | `#208A3C,#F2FCF3` | `#253627,#57965C` |
  | `debugger/stop.svg` | `#5E3838,#DB5C5C` | `#DB3B4B,#FFF7F7` | `#5E3838,#DB5C5C` |
  | `expui/general/chevronRight.svg` | `#B4B8BF` | `#818594` | `#B4B8BF` |
  | `expui/general/search.svg` | `#CED0D6` | `#6C707E` | `#CED0D6` |
  | `fileTypes/powershell.svg` | `#43454A,#CED0D6` | `#6C707E,#EBECF0` | `#43454A,#CED0D6` |
  | `fileTypes/text.svg` | `#CED0D6` | `#6C707E` | `#CED0D6` |
  | `fileTypes/toml.svg` | `#25324D,#548AF7` | `#3574F0,#F5F8FE` | `#25324D,#548AF7` |
  且这 7 条的 `_dark` 兄弟与 macOS 的 `_dark` **逐字节相同**（已验 4 条：debug/stop/
  chevronRight/search = True；3 条 `fileTypes/*` 无 `_dark` 兄弟）。
  **即：这 7 张图在 GPUI 里深浅两档画的是同一张深色美术，浅色主题下偏暗。**
  （另 6 条属配色微差，非极性错误。）
- **这条缺陷是否用户可见，取决于渲染路径**——实测 GPUI 有两条语义不同的路：
  | 路径 | 实现 | 颜色语义 |
  |---|---|---|
  | `theme_file_icon` → `gpui::img(ImageSource::Resource)` | `shared/src/icons/file_icon.rs:245-261` | **保留 SVG 内烘焙颜色** |
  | `idea_icon_mono` / `idea_icon_svg_px` → `lithe_ui::Icon` → `svg()` | `shared/src/icons/mod.rs:159-186` | **丢弃颜色，按主题前景重着色** |
  依据：gpui-pre 0.3.8 的 `Window::paint_svg`（`window.rs:4898-4906`）把 SVG 光栅化成
  **alpha mask** 再填单色 `MonochromeSprite`；`svg_renderer.rs:246-250` 明写
  「Convert the pixmap's pixels into an alpha mask」只取 `p.alpha()`。
  调用点计数：`idea_icon_image`（走 img，**保色**）**71** 处；
  `idea_icon_mono`（走 svg，**单色**）**37** 处；`idea_icon_svg_px`（单色）**15** 处。
  **所以：走 `idea_icon_image` 的那 71 处会真的画出偏暗的浅色图标；
  走 `idea_icon_mono`/`idea_icon_svg_px` 的 52 处不受影响**（颜色本就被主题覆盖）。
  受影响的具体位置（该图标确走保色路径）：`git/log_view.rs:1721` 的 `LogIcon::Idea`
  → `LogIcon::element` → `idea_icon_image`，含 `REFRESH_ICON`；
  `git/changes_view.rs:646`、`workbench/src/menu_bar.rs` 的 `MenuIcon::Idea` 路径等。
- **行距**：capability 要 24pt 连续行距。GPUI 项目树行高走
  `theme.control_height(Size::Small)`（`lists/project/row.rs:128`）= **24.0px + density delta**
  （`tokens.rs:98`；`Density::Standard` delta=0，`Compact` -4，`Comfortable` +4）。
  Standard 档下 24px 恰好对上；但 density 一变就偏离。macOS 侧是硬常量
  `LitheTheme.swift:723` `treeRowHeight = 27`、`:720` `rowHeight = 24`——
  **注意 macOS 自己也有两个值**（通用行 24、项目树 27），capability 取的是 24。
  缩进：GPUI `lists/project/row.rs:29-30` 用 `INDENT_BASE = 8.`、`INDENT_STEP = 14.`
  并注明「(`ProjectSidebarView.swift:558`)，which the library's own tree indent does not share」；
  库自己的树缩进是 `theme.tree_indent()` = **16.0px**（`tokens.rs:341`）。
  macOS `LitheTheme.swift:403` `indent = 19`。**8+14 / 16 / 19 三个值并存**。

## 2. 附：另四条含视觉成分的条目

### 2.1 `git-commit-sidebar-toolbar-icons`（macOS implemented / Windows platform-specific）

- macOS 证据含 `expui/general/refresh.svg`、`expui/vcs/revert.svg`、`expui/general/download.svg`、
  `expui/general/show.svg`、`expui/general/history.svg`、`expui/diff/magicResolveToolbar.svg`。
- GPUI 侧：`crates/git/src/changes_view.rs:891` 有 `render_toolbar`，用
  `idea_icon_image(&idea::REFRESH_ICON, ...)`（`:646`）、`PLUS_ICON`（`:926`）、
  `MINUS_ICON`（`:937`）等。判定：**有对应实现**（图标族同上，需核对是否 expui）。

### 2.2 `editor-wallpaper-overview-ruler`（macOS implemented / Windows platform-specific）

- macOS 证据：`frontend/editor/src/theme.ts`、`MonacoWorkbenchEditor.swift`。
- GPUI 侧：全 `rust/lithe-gpui` 检索 `wallpaper|overview_ruler|overviewRuler|Wallpaper`
  → **命中 0**。判定：**未找到实现入口（关键词 `wallpaper`/`overview_ruler`/`overviewRuler`/`Wallpaper`，命中 0）**。

### 2.3 `terminal-native-palette`（macOS implemented / Windows platform-specific）

- GPUI 侧：`crates/terminal/src/ansi.rs` 存在（`lib.rs:1` `mod ansi;`），
  但 `terminal/src/` 检索 `ansi|palette|Ansi` 只命中 `ansi.rs` 与 `session.rs:13` 的引用，
  **没有 Terminal.app 描述文件读取**。判定：**未找到实现入口（关键词 `Terminal.app 默认描述文件`/`palette`，命中 0）**。

### 2.4 `editor-macos-ime-presentation`（macOS implemented / Windows platform-specific）

- macOS 证据：`EditorFrontend/ime-input.css`、`main.ts`、`ime-input.integration.ts`。
  capability：「macOS 中文组合输入层遮住原文字，输入和删除候选文字时不透出整行重影」。
- GPUI 侧：**组合输入层在组件库已实现**，不是缺口：
  - `rust/lithe-ui/src/editor/ime.rs`（`end_composition` :66、`marked_text_range` :155）
  - `rust/lithe-ui/src/forms/text/ime.rs:53` `marked_text_range`
  - `rust/lithe-ui/src/terminal/input.rs:261` `marked_text_range`
  - 测试：`editor/tests.rs:248`「a slow composition undoes in one step」、
    `:265`「a composition cleared to nothing ends」、
    `forms/tests/mod.rs:135/152/340/417`（含「a composition shows the caret as typing does」）
  - token：`theme/tokens.rs:194` `underline_thickness()` 注明「Underline under composing text」，
    `lithe-ui` 内 12 处消费（`editor/line.rs:82`、`forms/text/element.rs:136`、
    `editor/search.rs:137`、`terminal/view.rs:309` 等）。
  - 宿主 `rust/lithe-gpui` 侧 `marked_text`/`preedit`/`underline_thickness` **命中 0**——
    但这是**分层正确的**：组合输入归组件库的编辑器/输入框，宿主不自己实现。
  判定：**有对应实现（在 lithe-ui 层）**。
  注：`rust/lithe-gpui` 检索 `ime` 得 409 处，**全是 `time`/`timeout` 的误命中**，
  不是 IME 证据——这一点原稿判断有误，已更正。

## 3. 判定汇总

| # | 条目 | GPUI 判定 | 主要差异 |
|---|---|---|---|
| 1 | macos-shared-product-dropdowns | 有对应实现 | 菜单开有 120ms 动画 + 4px 位移，真源要求无弹跳 |
| 2 | macos-shared-hover-tooltip-style | 有对应实现 | 圆角 6≠4、字号 12≠13、内边距、无内边框、颜色 |
| 3 | macos-bundled-default-ui-font | 有对应实现 | 字体族一致；Inter 字重 4≠19（缺 Bold/Black） |
| 4 | macos-shared-scrollbar-style | 有对应实现 | rail 12≠14、thumb 6≠7~12、无边框、无 Diff 标记 |
| 5 | project-sidebar-header-icons | 部分 | `locate` 命中 0（功能入口也不存在）；`refresh` 无色变体 |
| 6 | project-sidebar-tree-disclosure-icons | 部分 | 7 张图浅色档携带深色美术；缩进 8+14/16/19 三值 |
| 附 | git-commit-sidebar-toolbar-icons | 有对应实现 | 走 `idea_icon_image` 保色路径，受同上 7 张影响 |
| 附 | editor-wallpaper-overview-ruler | **未找到实现入口（命中 0）** | — |
| 附 | terminal-native-palette | **未找到实现入口（命中 0）** | — |
| 附 | editor-macos-ime-presentation | **有对应实现（在 lithe-ui 层）** | 非缺口 |

**对判断「视觉条目全有对应」的读数**：6 条主条目里 **4 条有对应实现、2 条部分**，
另 2 条含视觉成分的条目**完全命中 0**。按登记时的推翻条件
（「任一条目在 rust/lithe-ui 与 rust/lithe-gpui 全目录检索对应关键词命中 0，
且无等价组件/token 承担该职责」），`editor-wallpaper-overview-ruler` 与
`terminal-native-palette` 命中 0 且无等价物承担 —— **该判断被推翻**。

**本次最硬的一条新事实**：`SOURCES.md:3` 声明图标包由
`tools/sync-macos-idea-icons.mjs --apply` 生成且「不要手改」，
但只读 `--check` **失败 129 条**（exit 1），且工具算出的「保留 63 / 删除 123」
与现状（186 组常量、122 个 legacy 文件）相差极远。**声明的生成链路不可复现。**

## 4. 未执行的检索

- 未做任何视觉/像素级比对（无截图、无渲染）。本文件全部判定是**静态源码、资源清单与
  逐字节哈希对照**；「浅色档偏暗」是读 SVG 内色值 + 读 gpui 渲染器源码得出的推断，
  **不是渲染实测**。
- 未在 macOS 实机或 GPUI 实机上跑验证；`verificationStatus` 侧一律未动。
- 未运行 `cargo build`、未启动画廊，故「有对应实现」只表示源码存在，
  不表示该组件在 GPUI 前端里被接线使用。
- **未跑 `sync-macos-idea-icons.mjs --apply`**（会改工作区），只跑了只读 `--check`。
