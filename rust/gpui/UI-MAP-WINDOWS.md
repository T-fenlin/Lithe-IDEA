# Windows 界面 → gpui-kit 0.6.6 逐区域对应表（UI-MAP-WINDOWS）

> **本文件的位置**：`gpui/UI-MAP.md` §2 指定的"逐区域对应表"，由 `gpui/research/windows/01..06-*.md` 六份调研汇总而成。
> 结构照 `gpui/UI-MAP.md` §2 的风格：每个区域 = **组成与层级 / 度量 / 元素 → gpui-kit 对应 / 中文文案**。

---

## 0. 文件头：规格来源与约定

| 项 | 内容 |
| --- | --- |
| **规格来源** | **Windows 前端源码** `windows/tauri/src/`（Tauri v2 + React + TypeScript + Tailwind v4 CSS-first + shadcn 结构 + Base UI 原语）。**不是截图，不是 macOS** |
| **实现框架** | **gpui-kit 0.6.6**，源码根 `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-{kit,component,base}-0.6.6\src\`。**只以已发布源码为准**（在线文档描述的是未发布的 0.7.0） |
| **`docs/archive/ui-map-macos.md`** | **只作行为/功能对照**（查"这个交互原本怎么工作"），**不作尺寸来源** |
| **硬规则** | 沿用 `gpui/UI-MAP.md` §1（单位、token、行高、浮层挂层、DPI 口径…），本文不重复，只在相关处标注"见 §1.x" |
| **调研文档** | `gpui/research/windows/01-shell.md`、`02-editor-sidebar.md`、`03-git-and-bottom.md`、`04-theme-and-components.md`、`05-terminal-run-debug.md`、`06-database-ai-search.md` |

### 0.1 换算约定（逐条有据）

| 约定 | 内容 | 证据 | 状态 |
| --- | --- | --- | --- |
| **1 Tailwind 单位** | `0.25rem` = **4px** | `windows/tauri/src/styles/theme.css:220`（`html { font-size: calc(16px * var(--app-ui-scale)) }`）、`:113`（`--app-ui-scale: 1`） | ✅已核对 |
| **根字号** | 16px（`--app-ui-scale: 1`） | 同上 | ✅已核对 |
| **`--radius` 基准** | `8px` | `styles/theme.css:134` | ✅已核对 |
| **派生圆角** | `--radius-sm` = `×0.6` = 4.8px；`--radius-md` = `×0.8` = 6.4px；`--radius-lg` = `×1.0` = 8px；`--radius-xl` = `×1.4` = 11.2px；`--radius-2xl` = `×1.8` = 14.4px；`--radius-3xl` = `×2.2` = 17.6px；`--radius-4xl` = `×2.6` = 20.8px | `styles/theme.css:6-12`、`:134` | ✅已核对（行号） |
| **密度模式** | 默认档 `focused`；`:root[data-window-chrome-density="comfortable"]` 覆盖一批尺寸 | `styles/theme.css:188-200`；`features/settings/lib/ui-preferences.ts:9`；`features/settings/config/default-settings.ts:97`（默认 `"focused"`） | ✅已核对 |
| **主题注入** | `lithe.json` 的 `colors.<key>` → CSS 变量 `--<key>`，由 `themeRegistry.applyTheme()` 逐条 `root.style.setProperty()` 写入 `document.documentElement` | `extensions/themes/theme-registry.ts:92-99`；`extensions/themes/theme-file.ts:238-242` | 引用 |
| **无 Tailwind 配置文件** | CSS-first：令牌写在 `@theme inline { … }`；`windows/tauri/tailwind.config.*` **不存在** | `styles/theme.css:1-101` | 引用 |
| **默认主题** | `lithe-dark` | `extensions/themes/builtin/lithe.json:74`；`features/settings/config/default-settings.ts:99` | 引用 |
| **gpui-kit 的层级** | `gpui_kit::*` = GPUI 本体；`gpui_kit::base` = `gpui-base`；`gpui_kit::component` = `gpui-component`；`gpui_kit::assets` = `gpui-kit-assets` | `gpui-kit-0.6.6/src/lib.rs:95,106,143,145` | 引用（`UI-MAP.md` §1.1.1） |
| **重要单位提醒** | `--lithe-*` 面板令牌在 run / debugger / diagnostics 三个面板里**几乎不用**（全树只有 `run-pane.tsx:195` 一处）；调试面板头栏写死 `h-10`（40px），运行面板头栏 36px —— 两个面板头栏高度**不一致** | `research/windows/05` §3 前言、`run-pane.tsx:195`、`debugger-view.tsx:362` | 引用 |
| **Radix / cmdk / vaul 均为 0** | 通用组件层是 **Base UI 原语 + cva 变体**，不是 Radix；组件目录是 `windows/tauri/src/ui/`（62 个 `.tsx`），**`windows/tauri/src/components/` 不存在** | `windows/tauri/src/ui/button.tsx:2`、`ui/accordion.tsx:1`；`research/windows/04` §0.4、§5.1 | 引用 |

---

## 1. 数值速查表（必须逐值搬的尺寸/度量）

> 全部按**默认（focused）密度**。`comfortable` 档只覆盖少数几个值（见 §1.1 末表）。
> 证据一律指向 `windows/tauri/src/` 下的真实文件。**`✅已核对` = 本次实际打开该文件读到了该行**；无标记 = 直接引自调研文档。

### 1.1 外壳（chrome）

| 度量名 | 数值 | gpui-kit 写法 | 证据 | 状态 |
| --- | --- | --- | --- | --- |
| 标题栏高 | **40** | `TitleBar::h(px(40.))`（组件默认 34，见 §1.8） | `styles/theme.css:118`（`2.5rem`） | ✅已核对 |
| 标题栏左右内边距 | **8** | `.px(px(8.))` | `styles/theme.css:132`；`ui/chrome.tsx:11` | ✅已核对 |
| 窗口三键（最小化/最大化/关闭） | **56×40**（直角） | 自绘 `Button` + `window_bottom`/原生命中区 | `features/window/components/title-bar/window-controls.tsx:59,71,83-86` | 引用 |
| 三键图标 | **14×14** | `Icon::size_3p5()` | `ui/button.tsx:9`（`[&_svg:not([class*='size-'])]:size-3.5`） | 引用 |
| 窗口三键组 gap（Windows） | **0** | — | `window-controls.tsx:52-54` | 引用 |
| 窗口控件可见性 | Windows 恒为真 | — | `title-bar.tsx:78`、`use-native-window-chrome.ts:6-12` | 引用 |
| 自绘缩放热区 | 边 **5** / 角 **10** | `WindowBorder::resize_hit_size(px(5.))` | `features/window/components/window-resize-border.tsx:23-26` | 引用 |
| 窗口默认尺寸 | **1200×800** | 本项目口径另见 `UI-MAP.md` §1.3（对齐 Dodona） | `windows/tauri/src-tauri/tauri.windows.conf.json:8-9` | 引用 |
| 窗口最小尺寸 | **720×480** | `window_min_size` | `tauri.windows.conf.json:10-11` | 引用 |
| **项目标签条高** | **32**（`h-8`） | `h_flex().h(px(32.))` | `features/window/components/project-tab-bar.tsx:41` | **✅已核对** |
| 项目标签条左右内边距 | **6**（`px-1.5`） | `.px(px(6.))` | `project-tab-bar.tsx:41` | ✅已核对 |
| 项目标签条内层 gap | **4** | `.gap(px(4.))` | `project-tab-bar.tsx:46` | ✅已核对 |
| 项目标签高 | **28**（`h-7`） | `.h(px(28.))` | `project-tab-bar.tsx:64` | ✅已核对 |
| 项目标签宽区间 | min **144**（`min-w-36`）/ max **240**（`max-w-60`） | `.min_w(px(144.)).max_w(px(240.))` | `project-tab-bar.tsx:64` | ✅已核对 |
| 项目标签左右 padding | 左 **10**（`pl-2.5`）/ 右 **32**（`pr-8`，给关闭按钮） | `.pl(px(10.)).pr(px(32.))` | `project-tab-bar.tsx:64` | ✅已核对 |
| 项目标签 gap | **6**（`gap-1.5`） | `.gap(px(6.))` | `project-tab-bar.tsx:64` | ✅已核对 |
| 项目标签圆角 | **4.8**（`rounded-sm`） | `LitheRadii.sm` | `project-tab-bar.tsx:64`；`theme.css:6` | ✅已核对 |
| 项目标签文件夹图标 | **14×14** | `Icon::size_3p5()` | `project-tab-bar.tsx:70-75` | ✅已核对 |
| 项目标签选中下划线 | 高 **2**、左右内缩 **6**（`inset-x-1.5`）、`rounded-t-sm` | 自绘 `div` | `project-tab-bar.tsx:79-82` | ✅已核对 |
| 项目标签关闭按钮 | **24×24**（`icon-xs`） | `Button::with_size(px(24.))` | `project-tab-bar.tsx:93-104`；`ui/button.tsx:27` | ✅已核对 |
| **左侧活动栏宽（折叠态）** | **38** | `const COLLAPSED_ACTIVITY_RAIL_WIDTH = 38` | `features/layout/components/sidebar/main-sidebar.tsx:97` | **✅已核对** |
| 左侧活动栏宽（展开态） | 默认 **160**、min **140**、max **320** | `.w(px(..))` + clamp | `main-sidebar.tsx:98-100` | ✅已核对 |
| 左侧活动栏水平内衬 | **8** | `.px(px(8.))` | `main-sidebar.tsx:101`；`main-sidebar.tsx:610-611` | ✅已核对 |
| 左侧活动栏 rail 容器宽 | `calc(railWidth + var(--lithe-workbench-gap))` → 折叠 **38+4=42** | — | `main-sidebar.tsx:588-590,669-672` | 引用 |
| 左侧活动栏图标 | **16×16**（`size-4`） | `Icon::size_4()` | `features/layout/components/sidebar/sidebar-pane-selector.tsx:107` | 引用 |
| 活动栏条目（垂直）覆盖后 | min-h **24**（`min-h-6 py-1`） | `.min_h(px(24.)).py(px(4.))` | `sidebar-pane-selector.tsx:332`；`main-sidebar.tsx:332` | 引用 |
| 活动栏条目基类 | min-h **28**、gap **6**、px **8**、py **4**、圆角 **4** | — | `ui/sidebar.tsx:240`；`theme.css:122,131,133` | 引用 |
| 活动栏上下组 gap / 下组 pt | **4** / **4** | — | `sidebar-pane-selector.tsx:358-366` | 引用 |
| 活动栏拖拽热区 | 宽 **4**、线宽 **1**、hover `primary/8` | — | `main-sidebar.tsx:702-705` | 引用 |
| 项目圆点热区 / 圆点 | **16×16** / **6×6** | — | `features/layout/components/sidebar/sidebar-projects.tsx:69,92-95` | 引用 |
| 项目圆点条定位 | right **4**、bottom **6**、px **8** | — | `sidebar-projects.tsx:57` | 引用 |
| **右侧活动栏宽** | **38**（`w-9.5`） | `.w(px(38.))` | `features/layout/components/plugin-activity-rail.tsx:34` | 引用 |
| 右侧活动栏按钮 | **28×28**（`icon-sm`）、圆角 **4.8** | `Button::with_size(px(28.))` | `plugin-activity-rail.tsx:39,45,58,62` | 引用 |
| 右侧活动栏按钮图标 | **18×18**（`size-4.5`） | — | `plugin-activity-rail.tsx:48,64` | 引用 |
| 未读徽标 | min-w **12**、字号 **9**、行高 **12**、全圆角；`>9` 显示 `9+` | `Badge::count(n).max(9)` | `features/notifications/components/notifications-trigger.tsx:45-46` | 引用 |
| **状态栏高** | **24** | `StatusBar::h(px(24.))`（组件无高度常量，见 §1.8） | `styles/theme.css:119`（`1.5rem`）；`ui/chrome.tsx:13` | ✅已核对 |
| 状态栏组间 gap / 左右内边距 | **8** / **8** | — | `features/layout/components/footer/footer.tsx:45-49`；`ui/chrome.tsx:6` | 引用 |
| 状态栏项槽最小高 | **24** | `.min_h(px(24.))` | `footer/footer.tsx:53,63,72` | 引用 |
| 状态栏 chip | 高 **24**、max-w **200**、gap **4**、圆角 **6.4**、px **6** | `Button::ghost().with_size(px(24.))` | `footer/footer-status-chip.tsx:4-5` | 引用 |
| 光标位置 chip | 高 **20**（`h-5`）、全圆角、px **6**、编辑态宽 **56** | — | `features/editor/components/toolbar/editor-status-actions.tsx:8-9,79` | 引用 |
| 面包屑分隔符图标 | **14×14**、左右 **2** | `Icon::size_3p5()` | `ui/breadcrumb.tsx:75`；`features/editor/components/toolbar/path-breadcrumb.tsx:45` | 引用 |
| 准备状态展开卡片 | 宽 **320**、p **12**、下边距 **8**、圆角 **4.8** | — | `features/run/components/project-preparation-status.tsx:30-31,36-38,53` | 引用 |
| 内存轮询周期 | **10s**（页面隐藏时跳过） | — | `footer/footer-editor-status.tsx:24,51-53` | 引用 |
| **chrome 控件高** | **24**（`--lithe-chrome-control-height`） | `LitheMetrics.chrome_control_height` | `styles/theme.css:126` | ✅已核对 |
| chrome 命中目标 | **28**（**全仓库无引用点**） | — | `styles/theme.css:127` | ✅已核对 |
| chrome 行高 | **16** | — | `styles/theme.css:128` | ✅已核对 |
| chrome gap 三档 | tight **2** / default **4** / loose **6** | `SpacingTokens` 无 6 档 → 自建 | `styles/theme.css:129-131` | ✅已核对 |
| **工作区间隔** | **4**（`--lithe-workbench-gap`） | `const WORKBENCH_GAP: Pixels = px(4.)` | `styles/theme.css:125` | ✅已核对 |
| 面板岛圆角 / 边框 | 圆角 **11.2**、1px 左（编辑器）/ 左右（侧栏）/ 上+左（底部） | `rounded(px(11.2)).border_1()` | `features/layout/components/main-layout.tsx:313`；`resizable-pane.tsx:203-206`；`bottom-pane.tsx:247` | 引用 |
| 玻璃岛拖放遮罩 | 内框 `rounded-xl border-2 border-primary border-dashed`、`px-8 py-6`（32/24） | — | `main-layout.tsx:283-288` | 引用 |

**`comfortable` 密度覆盖（`styles/theme.css:188-200`，✅已核对）**：`--ui-text-caption` 12→**13**；`--ui-text-chrome` 13→**14**；`--lithe-footer-height` 24→**32**；`--lithe-tab-height` 28→**32**；`--lithe-sidebar-header-height` 32→**36**；`--lithe-chrome-control-height` 24→**28**；`--lithe-chrome-hit-target` 28→**32**；`--lithe-chrome-gap-tight` 2→**4**；`--lithe-chrome-gap` 4→**6**；`--lithe-chrome-gap-loose` 6→**8**；`--lithe-chrome-padding-inline` 8→**10**。
**未被 comfortable 覆盖**：`--lithe-title-bar-height`、`--lithe-pane-header-height`、`--lithe-tab-bar-height`、`--lithe-tab-max-width`、`--lithe-workbench-gap`、`--lithe-chrome-line-height`、`--lithe-chrome-radius`、`--radius`。

### 1.2 Dock / 拖拽面板

| 度量名 | 数值 | gpui-kit 写法 | 证据 | 状态 |
| --- | --- | --- | --- | --- |
| **Dock 栏宽（侧栏默认）** | **320** | `ResizablePanel::size(px(320.))` | `features/settings/config/default-settings.ts:138` | **✅已核对** |
| **Dock 栏最小宽** | **140**（`MIN_SIDEBAR_WIDTH`） | `.size_range(px(140.)..)` | `features/layout/components/resizable-pane.tsx:18` | 引用 |
| 右侧工具窗默认宽 | **400** | — | `default-settings.ts:139` | ✅已核对 |
| AI 聊天最小宽 | **300**（视口 <1100px 时 **220**） | `.size_range(px(300.)..)` | `resizable-pane.tsx:19-20,53-58` | 引用 |
| 全局绝对下限 | **50**（`MIN_RESPONSIVE_PANE_WIDTH`） | — | `features/layout/utils/resizable-pane-layout.ts:1` | 引用 |
| 主内容保底 | **360**（`MIN_MAIN_CONTENT_WIDTH`） | — | `resizable-pane-layout.ts:2` | 引用 |
| 最大宽公式 | `max(50, 视口宽 − reserved − 360)` | 需自行 clamp（`ResizablePanel::size` 只吃 `Pixels`） | `resizable-pane-layout.ts:4-9` | 引用 |
| 左/右面板外边距 | **4** | `.mr(px(4.))` / `.ml(px(4.))` | `resizable-pane.tsx:188-189` | 引用 |
| 面板拖拽热区 | 宽 **4**、线 **1**、hover `primary/8`、动效 **150ms** | `ResizablePanelGroup::with_handle_appearance` | `resizable-pane.tsx:155-176` | 引用 |
| 活动栏展开态设置默认值 | `activityRailWidth: 180`、`activityRailExpanded: false` | — | `default-settings.ts:131-132` | **✅已核对** |

> ⚠️ **`--lithe-workbench-gap` 是唯一"不随字号缩放"的尺寸**（源码里它是字面量 `4px`，不是 rem）。`LitheMetrics.workbench_gap` 不要乘 `scale`（见 `research/windows/04` §3.4(1)）。

### 1.3 侧栏与项目树

| 度量名 | 数值 | gpui-kit 写法 | 证据 | 状态 |
| --- | --- | --- | --- | --- |
| 侧栏头部高 | **32**（`--lithe-sidebar-header-height`） | `.h(px(32.))` | `styles/theme.css:124`；`ui/sidebar.tsx:93` | ✅已核对 |
| 文件树头部额外内边距 | `px-2` = **8**、下边框 1px | — | `features/file-explorer/components/file-explorer-tree.tsx:1310` | 引用 |
| **侧栏行高** | `max(24, uiFontSize × 1.35 + 6)`；13px → **24**（15→26.25，18→30.3） | `.h(px(24.))`（**必须固定**，见 §1.8 `uniform_list` 陷阱） | `features/file-explorer/lib/file-tree-row.ts:1-14`；`file-tree-row.test.ts:6-12` | 引用 |
| 树虚拟滚动留白 / overscan | 上下 **4** / **8 行** | — | `features/file-explorer/lib/file-tree-viewport.ts:1-2,16` | 引用 |
| 树行水平内缩 | **6**（`--file-tree-row-inline-inset`） | — | `features/file-explorer/styles/file-explorer-tree.css:6,36` | 引用 |
| 树行 padding | gap **6** → CSS 覆写 **4**；px **6**；py **4** → 覆写 **2** | 自绘 | `features/sidebar/components/sidebar-tree.tsx:241`；`file-explorer-tree.css:56-57` | 引用 |
| 树行圆角 / 字号 | **4** / **13** | — | `file-explorer-tree.css:7,48,100`；`theme.css:116` | 引用 |
| **树缩进** | `paddingLeft = 10 + depth × indentSize`，`FILE_TREE_BASE_INDENT = 10` | `div().pl(px(10. + 16. * depth))` | `features/file-explorer/components/file-explorer-tree-item.tsx:116,156`；`sidebar-tree.tsx:246`；`file-tree-row.ts:1` | 引用 |
| 缩进步长可选值 / 默认 | 12 / 16 / 20 / 24，默认 **16** | `LitheMetrics.file_tree_indent` | `file-explorer-tree.tsx:1482-1493`；`default-settings.ts:182` | **✅已核对（默认 16）** |
| `SidebarTreeRow` 组件默认缩进 | 基 **10** / 步长 **14** | — | `sidebar-tree.tsx:7-8` | 引用 |
| 折叠三角容器 / 三角 | **16×16** / **12×12**（`size-3` bold） | — | `sidebar-tree.tsx:301,314-316`；`file-explorer-tree.css:115-124` | 引用 |
| 文件图标 | **16×16**（`--file-tree-icon-size`） | — | `file-explorer-tree.css:5,126-131` | 引用 |
| 缩进参考线 | 容器宽 **7**、线宽 **1** 位于 `left:3px`、色 `mix(subtle-foreground 20%)`；默认 `opacity:0`，hover/focus → `0.9` | 自绘 1px 竖线 | `file-explorer-tree.css:9,177-199`；`sidebar-tree.css:6-7,44-59` | 引用 |
| 选中底色（树未聚焦 / 聚焦） | `--border` / `--selected` | `theme.border` / `LithePalette.selected` | `file-explorer-tree.css:8,69-78` | 引用 |
| 行悬停底色 | `--accent` / `mix(accent 68%, transparent)` | `list_hover` | `file-explorer-tree.css:4,93-95`；`sidebar-tree.css:5,20-22` | 引用 |
| 搜索命中高亮 mark | 圆角 **6.4**、底色 `mix(primary 30%)`、`padding: 0 1px` | 自绘 | `file-explorer-tree.css:163-168` | 引用 |
| 忽略文件 / 剪切态 | `opacity-50` / `italic opacity-40` | — | `file-explorer-tree-item.tsx:228-229` | 引用 |
| 拖拽阈值 / 自动展开延时 | **5px** / **550ms** | — | `file-explorer-tree.tsx:1064`；`use-file-explorer-drag-drop.ts:68` | 引用 |
| 拖拽幽灵 | `z-9999`、opacity `.95`、p **6/12**、border **2** `--primary`、radius **8**、`--shadow-popover`，跟随指针 +10/−10 | 自绘 | `use-file-explorer-drag-drop.ts:77-90,106-107` | 引用 |
| 右键菜单面板 | min-w **144**、圆角 **6.4**、p **4**、13px、`ring-1 --border/70`、`z-10070` | `PopupMenu::min_w(px(144.))` | `ui/context-menu.tsx:45` | 引用 |
| 右键菜单项 / 分隔线 | gap **8**、px **8** py **4**、圆角 **2.4**、图标 **16**；分隔线 1px、`my-1`=4 | `.separator()` | `ui/context-menu.tsx:89,196` | 引用 |
| 加载浮层胶囊 | p **12**；胶囊 px **12** py **6**、`rounded-full`、`bg-surface/92`、`shadow-popover`、`backdrop-blur-sm` | — | `file-explorer-pane.tsx:55-56` | 引用 |
| 内联重命名行 | gap **6** px **6** py **4**、13px、行高 1.35；输入框 `xs`、`px-0`、13px | `Input` + `InputState::select_all` | `file-explorer-tree-item.tsx:153`；`ui/input.tsx:67-87,193-205` | 引用 |

### 1.4 编辑区（标签栏 / 面包屑 / 编辑器 / 空态）

| 度量名 | 数值 | gpui-kit 写法 | 证据 | 状态 |
| --- | --- | --- | --- | --- |
| **标签条高** | **36**（`--lithe-tab-bar-height` = `--lithe-pane-header-height`） | `.h(px(36.))` | `styles/theme.css:120-121`；`ui/tab-bar.tsx:248` | ✅已核对 |
| 标签条左右内边距 / 段间距 / 下边框 | **8** / **4** / 1px `--border` | — | `ui/tab-bar.tsx:248` | ✅已核对 |
| 标签条底色 | `--tab-bar-bg` = `--background`（**与编辑区同底，靠 1px 下划线分隔，不用色带**） | 把 `theme.tab_bar` 设成 `background` | `ui/tab-bar.tsx:248`；`styles/theme.css:155` | ✅已核对 |
| **标签高** | **28**（`--lithe-tab-height`） | `.h(px(28.))` | `ui/tab-bar.tsx:260`；`styles/theme.css:122` | ✅已核对 |
| **标签宽上下限** | min **80**（`min-w-20`）/ max **200**（`--lithe-tab-max-width` = `12.5rem`） | `.min_w(px(80.)).max_w(px(200.))` | `ui/tab-bar.tsx:260`；`styles/theme.css:123` | ✅已核对 |
| 标签左右 padding | 左 **8**（`pl-2`）/ 右 **24**（`pr-6`，给关闭按钮） | `.pl(px(8.)).pr(px(24.))` | `ui/tab-bar.tsx:260` | ✅已核对 |
| 标签圆角 / 内 gap / 字号 | **4** / **6**（`--lithe-chrome-gap-loose`）/ **13** | `LitheRadii.chrome` / 自建 | `ui/tab-bar.tsx:170,176,236,260`；`theme.css:115,131,133` | ✅已核对 |
| 标签图标槽 | **12×12**（`grid size-3`） | — | `features/tabs/components/tab-bar-item.tsx:183` | 引用 |
| 活动标签指示条 | 高 **3**、左右内缩 **6**（`inset-x-1.5`）、`rounded-t-sm`、`--primary`、贴标签底边 | **自绘**（gpui `TabVariant::Underline` 只有 2px） | `ui/tab-bar.tsx:209` | **✅已核对** |
| 悬停底色 | `--accent/70`（活动标签悬停 `--accent/35`） | `ThemeColor::accent` + `opacity` | `ui/tab-bar.tsx:170,209,214` | ✅已核对 |
| 焦点环 | **2px** `--primary/25`，offset **1px**，offset-color `--tab-bar` | gpui `FOCUS_RING_WIDTH = px(3.)` / `0.5` → **对不上** | `ui/tab-bar.tsx:170`；`gpui-component/src/styled.rs:11-12` | ✅已核对（两处） |
| 过渡 | **150ms** `--app-ease-smooth` | `MotionTokens` 需覆写 | `ui/tab-bar.tsx:170`；`theme.css:135,137` | ✅已核对 |
| 脏标记圆点 | **8×8**、`rounded-full`、`--primary` | 自绘（gpui-kit **无** dirty/pin） | `tab-bar-item.tsx:284-291` | 引用 |
| 关闭/固定按钮 | **24×24**，绝对定位 `right:4px` + 垂直居中 | `Tab::suffix(Button)` | `tab-bar-item.tsx:164-166`；`ui/button.tsx:27` | 引用 |
| 关闭按钮可见性三档 | `always` / `active`（默认，仅活动标签）/ `hover`；固定标签恒显 | `shouldShowTabCloseButton` | `features/settings/lib/ui-preferences.ts:13-18`；`default-settings.ts:96` | 引用 |
| 拖动插入指示条 | 宽 **2**（`w-0.5`）、上下内缩 **4**、`--primary`、1s 脉冲 | 自绘 | `tab-bar-item.tsx:135`；`styles/utilities.css:68-82` | 引用 |
| 拖拽激活距离 / 重排动画 / 拖拽中透明度 | **5px** / **180ms** / **0.4** | 自研 sortable | `ui/tab-bar.tsx:32-36,74-78,184` | 引用 |
| 标签栏左右按钮容器 | 高 **32**（`h-8`）、左组 `gap-0.5`、右组 `gap-1 pl-0.5` | — | `tabs/components/tab-bar.tsx:633,763` | 引用 |
| 拖出标签栏判定容差 | 水平 **±24** / 垂直 **±64** | — | `tabs/components/tab-bar.tsx:458-459` | 引用 |
| 空态容器 | `size-full` 居中、`bg-background`、`px-24 py-32` | — | `panes/components/empty-editor-state.tsx:15` | 引用 |
| 空态内容块 | `max-w-md` = **448**、gap **12** | `.max_w(px(448.)).gap(px(12.))` | `empty-editor-state.tsx:16` | 引用 |
| 空态图标盒 / 主图标 / 角标放大镜 | **48×48** / **40×40**（`stroke-[1.15]`）/ **20×20** | — | `empty-editor-state.tsx:17-22` | 引用 |
| **面包屑栏高** | **28**（`h-7 min-h-7`） | `.h(px(28.))` | `features/editor/components/toolbar/breadcrumb.tsx:220` | 引用 |
| 面包屑栏边框 / 底色 / padding | 下 1px `--border/50` / `--background` / **8** | — | `toolbar/breadcrumb.tsx:220` | 引用 |
| 面包屑分段按钮 | 高 **24**、gap **4**、px **6** | `Button::with_size(px(24.))` | `toolbar/path-breadcrumb.tsx:54`；`ui/button.tsx:23` | 引用 |
| 面包屑右侧竖分隔线 | 宽 **1**、高 **14** | `Separator::vertical()` | `toolbar/breadcrumb.tsx:254` | 引用 |
| 面包屑目录下拉 | min-w **200**、max-h **300** | `PopupMenu::min_w(px(200.)).max_h(px(300.))` | `features/editor/config/constants.ts:55-57` | 引用 |
| **编辑器宿主 padding** | 上 **8** / 左 **16** / 下 **8** / 右 **16** | — | `editor/config/constants.ts:20-23` | 引用 |
| **编辑器行高公式** | `ceil(fontSize × 1.4)`；默认 14px → **20** | `.line_height(px(20.))` | `editor/utils/lines.ts:8-15`；`editor/config/constants.ts:4-5`；`features/settings/config/typography-defaults.ts:13` | 引用 |
| 编辑器字号 | 默认 **14**（`DEFAULT_CODE_FONT_SIZE`） | `theme.mono_font_size` | `typography-defaults.ts:13` | 引用 |
| 行号槽 | min-w **40**、字符宽 **8**、右边距 **8**、固定 4 位、左侧预留 8 | `EditorState` 内建行号（`LayoutMode::CodeEditor { line_number: true }`） | `editor/config/constants.ts:28-33` | 引用 |
| 编辑器滚动条 | **隐藏**（`.editor-container` `scrollbar-width:none`） | ⚠️ 但本项目为了可滚已改成常显（见 `UI-MAP.md` §1.3） | `editor/components/stylesheet.tsx:6-13` | 引用 |
| 编辑器 z-index 分层 | DECORATION 10 / SELECTION 20 / CURSOR 25 / GIT_BLAME 30 / OVERLAY 40 / DROPDOWN 100 / INLINE_TOOLBAR 200 / TOOLTIP 250 / CONTEXT_MENU 300 | gpui 用挂层顺序表达 | `editor/config/constants.ts:36-49` | 引用 |
| 查找栏 | padding 左右 **6**；圆角 `0 0 8 8`；底色 `mix(surface 94%, background)`；13px；`--shadow-popover` | `EditorState` 自带查找（含 `2/3` 计数） | `editor/styles/monaco-editor.css:11-24` | 引用 |
| 搜索命中高亮 / 当前命中 | 底色 `mix(primary 26%)` / 1px 描边 `mix(primary 72%, white 8%)` | — | `monaco-editor.css:59-65` | 引用 |
| 大文件提示条 | `border-b`、`bg-muted/40`、`px-3 py-1.5`（12/6）、12px | — | `editor/components/code-editor.tsx:665` | 引用 |
| 外部冲突横幅 | min-h **40**、gap **8**、px **12**、12px、图标 **16**、按钮高 **24** | `Alert` + `Button` | `editor/components/external-conflict-banner.tsx:39-59` | 引用 |
| 多文件头（multibuffer） | 行高 **28**、切换按钮 **28×28**、chevron **14**、文件名 `max-w-[45%]`、gap **6** | — | `editor/components/multibuffer/multibuffer-file-header.tsx:63-120` | 引用 |
| **窗格激活描边 / 拖拽悬停描边** | `ring-1 primary/30` / `ring-2 primary` | `ThemeColor::drag_border` | `panes/components/pane-container.tsx:1077-1078` | 引用 |
| 文件拖入高亮层 | `absolute inset-0 z-40 bg-primary/10` | `ThemeColor::drop_target` | `pane-container.tsx:1087` | 引用 |
| **分屏分隔条** | 厚 **4**；1px 视觉线透明 → hover/拖拽 `--primary` | `ResizablePanelGroup` 自带 handle | `panes/components/pane-resize-handle.tsx:147-148,161-163`；`theme.css:125` | 引用 |
| 窗格最小比例 / 默认分割 | `MIN_PANE_SIZE = 10`（且 ≥ pairTotal×10%）/ `[50, 50]` | `size_range` + 自行按比例换算成 px | `panes/constants/pane.ts:1,3` | 引用 |
| 拖放区域命中样式 | `inset-1`=**4**、`rounded-lg`=**8**、`border-2 --primary`、`bg-primary/14`、150ms | `SplitDropOverlay`（**gpui-kit 没有**） | `panes/components/split-drop-overlay.tsx:13-19,67-73` | 引用 |
| 轮播卡片宽 | 默认 **640**、min **320**、max = 视口宽 − **160** | — | `pane-container.tsx:134-136,494-500` | 引用 |
| 轮播卡片外观 / 间距 | `rounded-2xl`=**14.4**、活动 `border-primary/50`、拖拽中 `opacity-70`；gap/padding **16** | — | `pane-container.tsx:1106,1121-1128` | 引用 |
| 同窗格保活编辑器上限 / 工作区缓存上限 | **8** / **3** | — | `pane-container.tsx:137`；`panes/components/split-view-root.tsx:44,72` | 引用 |

### 1.5 底部窗与 Git 工具窗

| 度量名 | 数值 | gpui-kit 写法 | 证据 | 状态 |
| --- | --- | --- | --- | --- |
| **底部窗默认高度** | **320**（`useState(320)`）；外框 `calc(N + var(--gap))` = **324** | `resizable_panel().size(px(320.))` | `features/layout/components/bottom-pane/bottom-pane.tsx:47`；`:322-334` | **✅已核对（:47）** |
| 底部窗高度上下限 | min **200**、max **视口 × 0.8** | `size_range(px(200.)..)` + `on_resize` 自行 clamp（gpui-kit 无百分比上限） | `bottom-pane.tsx:122` | 引用 |
| 顶部拖拽条 | 厚 **4**；内部指示线 **1px**，hover/拖拽 `bg-primary`；hover 背景 `primary/8` | 组件自带 handle 外观近似 | `bottom-pane.tsx:226,234-239` | 引用 |
| 内容圆角 / 边框 / 底色 | **11.2** / 上+左 1px `--border/70` / `--background` | — | `bottom-pane.tsx:247` | 引用 |
| 全屏态 | `rounded-none border-0 shadow-none ring-0` | `DockArea::set_zoomed_in` | `bottom-pane.tsx:249` | 引用 |
| 拖拽悬停环 | `ring-2 ring-primary ring-inset` | — | `bottom-pane.tsx:248` | 引用 |
| **宽度模式（关键）** | 默认 `widthMode: "editor"` → 底部窗挂在**中央列**内（不覆盖两侧栏）；`"full"` → 跨整个工作台，左右各留 **4** | 默认放 center 的 `v_split`；full 放工作台单列 | `features/terminal/stores/terminal.store.ts:29`；`main-layout.tsx:318-322,345-351` | 引用 |
| 工具窗页签类型 | `terminal \| debugger \| diagnostics \| references \| buffers \| run \| maven \| gitLog`，默认 `"terminal"` | — | `features/window/stores/ui-state/types/ui-state.types.ts:17-26`；`ui-state/panel-slice.ts:31` | 引用 |
| **底部工具窗没有自己的标签条** | 页签选择器是左活动栏里的竖向图标列表 | — | `main-sidebar.tsx:632-656` | 引用 |
| Git Log 标题栏 | 高 **32**、内边距 **8**、间距 **8**、13px；图标 **14** | `h_flex().h(px(32.)).gap(px(8.)).px(px(8.))` | `features/git/components/log/git-log-title-bar.tsx:28-30` | 引用 |
| 「引用名」胶囊按钮 | 高 **24**、max-w **240**、圆角 **6.4**、px **8** | `Button::outline().with_size(px(24.)).max_w(px(240.))` | `git-log-title-bar.tsx:32-39` | 引用 |
| Git Log 图标按钮 ×3 | **24×24**，图标 **14** | `Button::ghost().with_size(px(24.))` | `git-log-title-bar.tsx:40-71`；`ui/button.tsx:9,27` | 引用 |
| Log/Console 标签行 | `gap-4 px-3 py-1 text-xs` → 行高 **16** + 上下 **4** = **24**；字号 **12**（**不是 13**）；间距 **16**；左右 **12** | ⚠️ 源码裸 `text-xs`，与标题栏/筛选行不一致；`role="tab"` **无任何选中视觉** | `git-log-tool-window.tsx:582` | 引用 |
| 刷新失败横幅 | 高 **28**、px **8**、13px | `Alert` | `git-log-tool-window.tsx:590` | 引用 |
| **提交表行高** | **30**（`const ROW_HEIGHT = 30`） | `DataTable::with_size(Size::Small)`（=30）或 `Size::Size(px(30.))` | `features/git/components/log/git-commit-table.tsx:43,116` | **✅已核对** |
| 提交表 overscan | **14** 行 | `VirtualList` 等价物需自调 | `git-commit-table.tsx:117` | **✅已核对** |
| 提交表工具行 | 高 **32**、gap **8**、px **8** | `h_flex().h(px(32.))` | `git-commit-table.tsx:200` | **✅已核对** |
| 提交表筛选输入框 | 高 **24**、min-w **144**、max-w **288**、gap **6**、圆角 **6.4**、px **8** | `Input` + `Size::Size(px(24.))` | `git-commit-table.tsx:201` | **✅已核对** |
| 提交表字段下拉 | 高 **24**、圆角 **6.4**、px **6** | `Select` | `git-commit-table.tsx:232` | **✅已核对** |
| 提交表装饰开关 | **24×24**，图标 **14** | `Toggle` 或 `Button::toggled(bool)` | `git-commit-table.tsx:239-249` | 引用 |
| 提交表表头行 | 高 **24**、px **8** | — | `git-commit-table.tsx:255` | **✅已核对** |
| **提交表列宽策略** | 「提交」`min-w-0 flex-1`（弹性，**未预留泳道图宽度**）；「作者」`w-28` = **112**；「日期」`w-32` = **128** 右对齐 | `Column::new(k,n).width(px(112.))` / `.max_width` + 剩余给提交列 | `git-commit-table.tsx:256-258,321-326` | **✅已核对** |
| 提交表内容最小宽 | **520**（`min-w-130`） | — | `git-commit-table.tsx:276` | **✅已核对** |
| 提交行容器 | 下边框 **1** `--border/50`、px **4**；hover/focus `bg-accent/70`；选中 `bg-primary/22` → hover `/28` | `TableDelegate::render_tr` 自绘 | `git-commit-table.tsx:301-303` | 引用 |
| 日期单元格字号 | **11**、等宽、右对齐 | — | `git-commit-table.tsx:324` | **✅已核对** |
| 加载更多行 | 高 **36**（`h-9`）、min-w **520**、按钮高 **24** | `Button` + `ListDelegate::has_more` | `git-commit-table.tsx:428-441` | **✅已核对** |
| **泳道图常量** | `ROW_HEIGHT 30`、`LANE_GAP 13`、`GRAPH_PADDING 8`；svg 宽 `max(30, laneCount×13 + 16)`；线宽 **1.6**；节点 `r=4.3` 描边 **2** | **gpui-kit 没有** → `canvas()` 自绘 | `features/git/components/log/git-graph-row.tsx:4-6,27,50,69-78` | 引用 |
| 泳道配色（6 色循环） | `#55d68b` `#65a9ff` `#d77eea` `#f3aa59` `#e76c72` `#56c7cf` | 自建调色板 | `git-graph-row.tsx:7,9-11` | 引用 |
| 标签徽章 | max-w **112**、字号 **10**、px **6** py **2**、圆角 **6.4** | `Tag` + 自定义色 | `git-graph-row.tsx:87-88` | 引用 |
| 标签配色 4 档 | head `sky` / remote `indigo` / tag `amber` / branch `emerald`（各带 border/bg/text 三值，明暗两套） | 自建 | `git-graph-row.tsx:16,18,20,22` | 引用 |
| **引用树三栏默认布局** | references **19%** / commits **57%** / inspector **24%** | 首帧按容器宽换算成 px | `features/git/stores/git-log-preferences.store.ts:43-47` | 引用 |
| 引用树三栏最小宽 | **140** / **320** / **220** | `.size_range(px(140.)..)` 等 | `git-log-tool-window.tsx:622,648,684` | 引用 |
| Inspector 竖向默认 / 最小高 | files **62%** / details **38%**；min **90** / **80** | `v_resizable` + `size_range` | `git-log-preferences.store.ts:49-52`；`git-commit-inspector.tsx:104,164` | 引用 |
| 引用树左侧工具栏 | 宽 **36**、上下 p **4**、右边框 1px；按钮 **32×32**、图标 **16**、圆角 **4.8** | `v_flex().w(px(36.))` + `Button::with_size(px(32.))` | `features/git/components/log/git-reference-tree.tsx:146,357,371` | 引用 |
| 引用树列表头 / 滚动区 | 高 **32** px **8** / p **6** | — | `git-reference-tree.tsx:840,847` | 引用 |
| HEAD 行 | 高 **28**、下边距 **4**、圆角 **6.4**、px **8**、gap **8**；分支名 max-w **96** | — | `git-reference-tree.tsx:853,861` | 引用 |
| 分区头 / 引用行 | 高 **24**、gap **6**、圆角 **6.4**、px **6** / 高 **24**、圆角 **4.8**、hover `bg-accent/80` | — | `git-reference-tree.tsx:599,876` | 引用 |
| **引用行缩进** | `paddingLeft = 10 + depth × 14` | `.pl(px(10. + 14. * depth))` | `git-reference-tree.tsx:594,603` | 引用 |
| disclosure / 占位 / 引用图标 | **14×14** | `Icon::size_3p5()` | `git-reference-tree.tsx:611,618,630` | 引用 |
| 「当前」徽章 | 字号 **10**、圆角 **6.4**、px **4** | — | `git-reference-tree.tsx:652` | 引用 |
| 引用树右键菜单最小宽 | **288**（`min-w-72`） | `PopupMenu::min_w(px(288.))` | `git-reference-tree.tsx:480,495` | 引用 |
| Inspector 文件区表头 / 详情面板 | 高 **32** px **8** gap **8** / p **12**、项间距 **8** | — | `git-commit-inspector.tsx:106,165-167` | 引用 |
| Inspector 短哈希/作者/邮箱/日期 | 字号 **11** 等宽 | — | `git-commit-inspector.tsx:174,178` | 引用 |
| Inspector 完整哈希 | 字号 **10**、`break-all` 等宽 | — | `git-commit-inspector.tsx:182` | 引用 |
| **控制台左侧按钮栏** | 宽 **32**、gap **4**、上下 p **4**、右边框 1px；按钮 **24×24**、圆角 **6.4**、图标 **14** | `v_flex().w(px(32.))` | `features/git/components/log/git-execution-console.tsx:71-90` | 引用 |
| 控制台根字号 | **12**、等宽 | `theme.mono_font_family` + `text_size(px(12.))` | `git-execution-console.tsx:68` | 引用 |
| 控制台输出区 / 查找条 | p **12** / 上下 **4** 左右 **12** gap **8** | — | `git-execution-console.tsx:93,103` | 引用 |
| 控制台底部跟随阈值 | `scrollHeight − scrollTop − clientHeight < 32` | `AutoScroll` / `message_scroller` | `git-execution-console.tsx:103` | 引用 |
| 控制台条目 | 下边距 **4**、行高 **20** | — | `git-console-entry.tsx:50` | 引用 |
| **变更列表工具行** | 高 **36**（`--lithe-pane-header-height`）、px **12**、下边框 1px | `SidebarToolbar` 等价物 | `ui/sidebar.tsx:57`；`features/git/components/status/git-status-panel.tsx:1107` | 引用 |
| 变更列表「查看差异」按钮组 / caret | 高 **24** / 图标 **12** | — | `git-status-panel.tsx:1109-1134`；`ui/button.tsx:23,27` | 引用 |
| 变更列表差异来源下拉 | min-w **150** | — | `git-status-panel.tsx:1141` | 引用 |
| 变更列表滚动区内边距 | px **8** py **8** | — | `git-status-panel.tsx:1183-1188` | 引用 |
| 变更列表分类头（surface 变体） | 高 **32**、圆角 **8**、px **10**、`bg-accent/80` | — | `ui/sidebar.tsx:327-330`；`git-status-panel.tsx:765-776` | 引用 |
| 变更列表文件夹行尾计数 | 字号 **10** | — | `git-status-panel.tsx:925-931` | 引用 |
| 暂存/取消暂存按钮 | **20×20**，默认 `opacity-0`，行 hover 才显 | ⚠️ gpui-kit **无 group-hover 等价物**，要自管 hover 状态 | `features/git/components/status/git-status-file-item.tsx:62-68` | 引用 |
| pending 旋转指示 | **12** | `Spinner::with_size(Size::XSmall)` 或自绘 | `git-status-file-item.tsx:78-82` | 引用 |
| 提交面板底栏 | 下/左右外边距 **8**、圆角 **11.2**；编辑器外壳圆角 **11.2** | `SidebarFooter` 等价物 | `ui/sidebar.tsx:75,115-117` | 引用 |
| 提交说明输入 | min-h **64**、max-h **128**、`rows={2}`、px **12**、上 **12** 下 **8** | `Textarea` + `.auto_grow(2, N)`（**必须显式给行数**） | `features/git/components/git-commit-panel.tsx:283-288` | 引用 |
| 提交面板选中文件计数行 | 上 p **6**、gap **8/4** | — | `git-commit-panel.tsx:293-295` | 引用 |
| ahead/behind 按钮 | 高 **24**；ahead `text-git-added`、behind `text-git-deleted` | — | `git-commit-panel.tsx:306-333` | 引用 |
| 提交面板错误块 | 外边距 **8**、圆角 **6.4**、px **8** py **4** | `Alert` | `git-commit-panel.tsx:266-269` | 引用 |
| ahead/behind 计数（共用） | 字号 **10**、gap **4**、行高 1；上限 **99** → `99+` | — | `features/git/components/git-tracking-counts.tsx:3,9-14,36-53` | 引用 |
| 侧栏面板标题栏 | 高 **36**、px **12**，标题 `pl-2` + **8**、`ui-text-lg`（**该类不存在，是空操作**） | — | `ui/sidebar.tsx:38,43` | 引用 |
| 侧栏标题栏动作区 | max-w **50%**、gap **4** | — | `ui/sidebar.tsx:47` | 引用 |
| Diff hunk 头 | `grid-cols-[2.75rem_minmax(0,1fr)]` → 左 gutter **44**；高 = 编辑器行高公式 | **gpui-kit 没有 diff** | `features/git/components/diff/git-diff-hunk-header.tsx:39-49,91-161` | 引用 |
| Diff 头部状态徽章 | `rounded-full px-1.5 py-0.5`、13px、`capitalize` | `Tag` | `git-diff-header.tsx:72-79` | 引用 |

### 1.6 终端

| 度量名 | 数值 | gpui-kit 写法 | 证据 | 状态 |
| --- | --- | --- | --- | --- |
| 终端页签条（水平） | 高 **36**、gap **4**、左右 **8**、下边框 1px | `TabBar` + `.h(px(36.))` | `ui/tab-bar.tsx:244-255` | 引用 |
| 终端页签条（垂直） | 上下 p **4**；宽度 `tabSidebarWidth`（默认 **180**，范围 **80–400**） | **gpui-kit 的 TabBar 无 vertical** → 自绘 | `ui/tab-bar.tsx:249`；`features/terminal/stores/terminal.store.ts:31,65` | 引用 |
| 终端单个页签（水平 / 垂直） | 高 **28**、min-w **80**、max-w **200**、左 **8** 右 **24** / min-h **28**、圆角 **6** | — | `ui/tab-bar.tsx:257-267` | 引用 |
| 终端页签容器 | 水平 gap **2**；垂直 gap **2**、px **6** py **4** | — | `features/terminal/components/terminal-tab-bar.tsx:841-847` | 引用 |
| 终端工具按钮组 | 水平高 **32**；垂直 px **6** py **4** | — | `terminal-tab-bar.tsx:461-467` | 引用 |
| 终端空态条 | min-h **32**、px **8** py **6** | `Empty` | `terminal-tab-bar.tsx:710-714` | 引用 |
| 垂直页签侧栏拖宽热区 | 宽 **4** | — | `terminal-tab-bar.tsx:909-913` | 引用 |
| 页签关闭/固定按钮 | **24×24**，`right:4px` 垂直居中；活动或固定时 `opacity-100`，否则行 hover | ⚠️ gpui-kit 无 group-hover | `terminal-tab-bar-item.tsx:102-107`；`ui/button.tsx:27` | 引用 |
| 终端内容左内边距 | **16**（`pl-4`） | `.pl(px(16.))` | `features/terminal/components/terminal.tsx:870` | 引用 |
| 非活动页签不透明度 | **0.6** | — | `terminal.tsx:882` | 引用 |
| xterm 宿主底色 | `var(--background)` | — | `features/terminal/styles/terminal.css:2-9` | 引用 |
| xterm 滚动条 | 宽 **11**、thumb min-height **36**、圆角 `--app-scrollbar-radius` | `ScrollbarTheme` 需覆写（gpui 默认 6/8px） | `terminal.css:41-64`；`styles/scrollbars.css:2` | 引用 |
| 终端搜索浮层 | `absolute top-2 right-2 z-30`（另 `.terminal-search{z-index:100}`）；top/right **8** | `Popover` | `terminal-search.tsx:84`；`terminal.css:72-74` | 引用 |
| 终端分屏 | 左右 `h-full w-1/2 border-r`；上下 `h-1/2 w-full border-b` | `h_resizable`/`v_resizable` | `terminal-container.tsx:645-646` | 引用 |
| 终端会话区外框 | 圆角 **8**、`border-border/60` | — | `terminal-container.tsx:707-714` | 引用 |
| **终端字号 / 行高 / 字距** | 字号 = `round(terminalFontSize × zoom × 10)/10`（缩放边界 **8–32**，重置 **14**）；`lineHeight`、`letterSpacing` 来自设置 | 自建 VT 渲染 | `terminal.tsx:267-284,704-722` | 引用 |
| 终端高/低水位背压 | **500 000** / **100 000** 字节 | 自建 PTY 层 | `features/terminal/utils/terminal-protocol.ts:13-14` | 引用 |
| 终端工具栏右键菜单最小宽 | **180**；配置文件菜单宽 **220**、内容 max-h **288** | — | `terminal-tab-bar.tsx:203,1031,1037` | 引用 |
| 终端菜单分组标题 | 13px、上下 **4**、左右 **10** | — | `terminal-tab-bar.tsx:204,209,216` | 引用 |

### 1.7 运行与调试

| 度量名 | 数值 | gpui-kit 写法 | 证据 | 状态 |
| --- | --- | --- | --- | --- |
| **运行面板头栏** | 高 **36**、gap **8**、px **12**、下边框 1px | `.h(px(36.))` | `features/run/components/run-pane.tsx:195` | 引用 |
| 运行面板头部图标 / 标题 | **16** / 13px medium | `Icon::size_4()` | `run-pane.tsx:196-197` | 引用 |
| 运行状态文字 | 13px；成功 `text-success` / 失败 `text-destructive` | — | `run-pane.tsx:202-206` | 引用 |
| Java 发现提示条 / 诊断告警横幅 | py **6** px **12** / py **8** px **12**，告警图标 **14** | `Alert` | `run-pane.tsx:49,262-263` | 引用 |
| 运行配置列表标题 / 滚动区 | py **8** px **12** / `pb-2` (8) | — | `run-pane.tsx:325,328` | 引用 |
| 分组标题 | py **4** px **8**、13px medium | — | `run-pane.tsx:494` | 引用 |
| **运行配置行** | py **4** px **6**、gap **4**、圆角 **6**、13px；约 **24–26** 高；选中 `bg-selected`，否则 `hover:bg-accent` | `ListItem` + 自绘 | `run-pane.tsx:500-503` | 引用 |
| 折叠组按钮 | 上 **8**、py **4**、px **8** | — | `run-pane.tsx:341,361` | 引用 |
| 配置详情头 / 详情键值网格 | py **8** px **12** / 首列 **104**、行间距 **2** | `DescriptionList` | `run-pane.tsx:394,397` | 引用 |
| 输出滚动区 / 输出标题 | py **8** px **12** / 下 **4** | — | `run-pane.tsx:411`；`run-output-text.tsx:35` | 引用 |
| **运行输出正文** | **12px** 等宽、`whitespace-pre-wrap`、`select-text` | 等宽 + 虚拟列表自绘 | `run-output-text.tsx:39,48` | 引用 |
| 时间戳颜色 | `text-subtle-foreground/70` | — | `features/run/utils/run-output-style.ts:42` | 引用 |
| 严重级别颜色 | error `--destructive` / warning `--warning` / info `--info` / debug `--subtle-foreground` | — | `run-output-style.ts:35-40` | 引用 |
| stdin 行 / 输入框 | py **6** px **12** gap **6** / 高 **28**、圆角 **6**、px **8**、12px 等宽 | `Input` + `Size::Size(px(28.))` | `run-pane.tsx:454,462` | 引用 |
| 缺配置空态 | gap **12**、px **24**、图标 **32** | `Empty` | `run-pane.tsx:306-307` | 引用 |
| 运行操作面板（命令面板式） | 宽 **360**、圆角 **12**；头 py **10/8** px **12**；搜索框高 **32**；滚动体 max-h **420 / 60vh** | `Popover` + `Input` + `List` | `features/run-actions/components/run-actions-button.tsx:288-332` | 引用 |
| 运行操作行 | min-h **44**、py **4** px **6**；图标容器 **24×24** 圆角 **6** | `ListItem` | `run-actions/components/run-action-row.tsx:35-44` | 引用 |
| 运行配置编辑器 | 段间距 **16 / 24 / 8**；网格首列 **120**、行间距 **4**；页脚上 **12** gap **8** | `Form` + `Field` | `features/run/components/run-configuration-editor.tsx:211-213,248,428` | 引用 |
| 配置列表分隔柄 | 内芯 **1px**，hover `primary/8`；拖动遮罩 `fixed inset-0 z-40` | — | `run/components/run-configuration-list-split.tsx:152-173` | 引用 |
| **调试面板头栏** | 高 **40**（`h-10`，**与运行面板的 36 不一致**）、gap **8**、px **12** | `.h(px(40.))` | `features/debugger/components/debugger-view.tsx:362` | 引用 |
| 调试头栏图标 / 标题 | **16** / 13px medium | — | `debugger-view.tsx:363,365` | 引用 |
| 调试按钮组分隔 | 左 **8**；组内竖分隔高 **16**、左右 **4** | `Separator::vertical()` | `debugger-view.tsx:367,398` | 引用 |
| 调试主体网格 | `minmax(260px,320px) minmax(0,1fr)` → 左栏 **260–320** | `h_resizable` + `size_range(px(260.)..px(320.))` | `debugger-view.tsx:442` | 引用 |
| 调试左栏 | p **12**、块间距 **12**；命令回显框 min-h **32**、圆角 **8**、py **6** px **8** | — | `debugger-view.tsx:443-444,472` | 引用 |
| 调试主/单步按钮 | 主按钮网格 gap **6**，图标按钮 **32×32**；单步按钮高 **24** | `Button` + `ButtonGroup` | `debugger-view.tsx:478-537` | 引用 |
| 调试会话摘要条 / 项目路径条 | py **8** px **12** | — | `debugger-view.tsx:548,564` | 引用 |
| 调试右侧页签条 / 页签按钮 | 高 **36**、gap **4**、px **8** / 高 **32**、下边框 **2**、px **8** | `TabBar` + `TabVariant::Underline`（默认 36） | `debugger-view.tsx:575-599` | 引用 |
| 调试面板网格 | 两列、gap **8**、p **8** | — | `debugger-view.tsx:601` | 引用 |
| 调试面板卡片 / 面板头 | 圆角 **12**、1px 边框 / 高 **32**、gap **4**、px **6** | — | `debugger-panels.tsx:43-45,68` | 引用 |
| 面板标题文字 / 计数徽标 | 13px、**uppercase** / 高 **20**、px **6** py **2**、全圆角 | `Badge::new().count(n)` | `debugger-panels.tsx:78,80`；`ui/badge.tsx:6,19` | 引用 |
| 调用栈行 / 线程行 / 断点行 | py **6** px **12** gap **8**；选中 `bg-selected/70`；栈图标 **13** | `ListItem` | `debugger-panels.tsx:151-157,193-201,229` | 引用 |
| 断点启用点 | **12×12**、圆、启用 `border-destructive bg-destructive`，禁用透明 | 自绘 | `debugger-panels.tsx:238-243` | 引用 |
| **变量行缩进** | 一级 **18**、每层 **+12**；网格 `0.42fr / 0.58fr`、gap **8** | `.pl(px(18. + 12. * depth))` | `debugger-variables-panel.tsx:83-84,95` | 引用 |
| 监视面板 / 监视条目 | p **8**、块间距 **6** / py **6** px **8**、圆角 **8** | — | `debugger-watch-panel.tsx:90,135` | 引用 |
| 调试控制台单行 | py **4** px **12**、13px 等宽；stderr `--destructive`，stdout `--subtle-foreground` | 自绘 | `debugger-view.tsx:671-672` | 引用 |
| 调试控制台保留上限 | 只渲染最近 **80** 条 | — | `debugger-view.tsx:195` | 引用 |

### 1.8 搜索 / AI 面板 / 全局面板

| 度量名 | 数值 | gpui-kit 写法 | 证据 | 状态 |
| --- | --- | --- | --- | --- |
| **全局搜索工具条** | `border-b bg-surface/55 py-2` + `px-2`(compact)/`px-3` → 上下 **8**、左右 **8/12** | — | `features/global-search/components/global-search-toolbar.tsx:93` | 引用 |
| **搜索输入框** | 高 **28**、圆角 **8**、gap **8**、px **8**、底色 `bg-background/65` | `Input` + `.with_size(px(28.))` | `global-search-toolbar.tsx:101` | **✅已核对** |
| 搜索框图标 | **16×16** | `Icon::size_4()` | `global-search-toolbar.tsx:102` | **✅已核对** |
| 结果/警告 Badge | max-w **224** / **256** | `Badge` + `.max_w(..)` | `global-search-toolbar.tsx:150,159` | 引用 |
| 替换框 / 前置图标块 | 高 **32**、圆角 **8** / **32×32** | — | `ui/search.tsx:271,260` | 引用 |
| 包含/排除输入 | 高 **28**、圆角 **6.4**、px **8** | — | `global-search-toolbar.tsx:190,199` | 引用 |
| 结果区布局 | 左导航 `my-2 ml-2`；右 ScrollArea `px-2 pb-2`、`orientation="both"` | **gpui-kit 无横向滚动条等价物** | `global-search-results.tsx:60,73,76-79` | 引用 |
| **文件导航栏宽** | 默认 **224**、min **176**、max **420**、≤ 父宽 **50%**；键盘步长 **16**；拖拽热区 `-right-1 w-2` = **8** | `.size_range(px(176.)..px(420.))` | `features/file-explorer/lib/file-navigator-layout.ts:1-5,28`；`components/file-navigator-sidebar.tsx:45,465` | 引用 |
| 摘录容器 / 列表 gap | 圆角 **11.2** / gap **8** | — | `components/search-excerpt-results.tsx:109,140,207` | 引用 |
| 摘录最小高 | **104** | 自绘（`VirtualList` 变高） | `components/search-excerpt-code.tsx:154,164` | 引用 |
| 摘录字号 / gutter | 编辑器字号 × zoom；行号 gutter 宽 `calculateTotalGutterWidth(...)`、右对齐 tabular-nums | — | `search-excerpt-code.tsx:59-61,143-150` | 引用 |
| 命中高亮 | 普通 `bg-warning/20`；当前 `bg-warning/40 ring-1 ring-inset ring-warning/60`；行 hover `bg-accent/25` | 自绘 span 背景 | `search-excerpt-code.tsx:80-81,111,117` | 引用 |
| 上下文行数 | 默认 **2**、展开 **7** | — | `components/global-search-buffer.tsx:24-25` | 引用 |
| 搜索分页/渲染上限/防抖 | `PAGE_SIZE 140`、`INITIAL_RENDER_LIMIT 40`、`RENDER_INCREMENT 40`、`DEBOUNCE 200ms` | `ListDelegate::has_more` + `load_more_threshold` | `constants/limits.ts:6-11` | 引用 |
| 索引轮询 / provider 缓存 TTL | **150ms** / **2000ms** | — | `hooks/use-content-search.ts:25-26` | 引用 |
| 懒加载哨兵 / 高亮预取 | `rootMargin 640px` / `240px` | `VirtualList` 可见区间回调 | `global-search-buffer.tsx:497-516`；`search-excerpt-results.tsx:32-33,80-91` | 引用 |
| **AI 面板内容最大宽** | **896**（`max-w-4xl`） | `.max_w(px(896.)).mx_auto()` | `features/ai/components/agent-tab.tsx:29` | 引用 |
| AI 初始态容器 / 内容宽 | px **32** py **40** / max-w **720** | — | `components/chat/ai-chat.tsx:1045-1046` | 引用 |
| AI header 行 | min-h **28**、gap **6**、px **6** py **4** | pane chrome 等价物 | `features/panes/components/pane-chrome.tsx:6`；`chat/chat-header.tsx:146` | 引用 |
| AI header 图标 chip / 会话标题 | 高 **20**、图标容器 **24** / 圆角 **6.4**、px **8** py **4** | — | `pane-chrome.tsx:11`；`chat-header.tsx:72,150` | 引用 |
| AI header 动作按钮 | **24×24** | `Button::with_size(px(24.))` | `chat-header.tsx:168`；`ui/button.tsx:27` | 引用 |
| 消息项 padding | 普通 px **16** py **8**；工具-only px **16** pt **8** pb **4** | — | `components/chat/chat-messages.tsx:137-140` | 引用 |
| 空态列表 | `justify-end px-4 pb-2 pt-4` + max-w **384** | — | `chat-messages.tsx:100-101` | 引用 |
| **用户气泡最大宽** | **80%**；BubbleContent 圆角 **11.2**、px **12** py **8** | `Bubble` + `.max_w(relative(0.8))` | `ui/bubble.tsx:16,29,65` | 引用 |
| 消息编辑态 textarea | `max-w-[80%]`、min-h **64**、`resize-y` | `Textarea` | `chat/chat-message.tsx:135,151` | 引用 |
| 消息操作图标 | **14×14** | — | `ui/message.tsx:133` | 引用 |
| Markdown 代码块 / 表格 / 引用 / h1 | p **8** 圆角 **4.8** / my **8** / `border-l-2 pl-3` / `mt-3 mb-1.5` 13px semibold | `TextView::markdown` | `components/messages/markdown-renderer.tsx:158-159,190,348,512-513,824` | 引用 |
| 计划卡 / 计划步骤 | 圆角 **14.4**、头 gap **6** px **12** py **8**、步骤区 p **12** 间距 **6** / 容器圆角 **11.2**、执行按钮 px **10** py **8** | — | `messages/plan-block-display.tsx:44-45,55`；`plan-step-display.tsx:36,44,60` | 引用 |
| 活动行 / 展开详情 | min-h **16**、gap **8**；`mt-1 max-h-64 pl-6` = 上 **4** / **256** / 左 **24** | — | `ui/marker.tsx:7,60`；`chat/chat-activity-line.tsx:65` | 引用 |
| 加载指示 | MarkerIcon **20**、`ThinkingOrb size=20`、文字用 `ui-text-shimmer` | `ShimmerText` | `chat/chat-loading-indicator.tsx:26-29`；`styles/utilities.css:84-104` | 引用 |
| 权限条 | 外层 px **12** pt **8**；条高 **36**、圆角 **8**、gap **8** | `Alert` + `Button` | `chat/acp-permission-prompt.tsx:85-86` | 引用 |
| 后续操作按钮 | 高 **24**、图标 **14** | — | `chat/chat-follow-up-actions.tsx:69-77` | 引用 |
| **composer 初始态外框 / 默认态外框** | 圆角 **14.4** `bg-surface/55` / 外边距 **8**、圆角 **11.2**、`pb-1`=4 | — | `input/chat-composer.tsx:29`；`ui/sidebar.tsx:75` | 引用 |
| composer 可编辑区 | max-h **140**、min-h **64**、px **12** pt **12** pb **8**、`line-height: 1.4` | `Textarea` + `.auto_grow(2, N)` | `input/chat-composer.tsx:77-78,85` | 引用 |
| composer 初始态可编辑区 | max-h **192**、min-h **112**、p **16**、`ui-text-base` | — | `input/chat-input-bar.tsx:1025` | 引用 |
| 发送/停止按钮 | **28×28**（`icon-sm`） | `Button::with_size(px(28.))` | `chat-input-bar.tsx:1162`；`ui/button.tsx:28` | 引用 |
| 上下文 chips | gap **6**、圆角 **6.4**、px **6** py **4**，media 宽 **28** | `Tag` + 自绘 | `ui/attachment.tsx:15,56,169` | 引用 |
| 粘贴图片缩略图 | 宽 **96–120** | — | `ui/attachment.tsx:19` | 引用 |
| `@`/`/` token | min-h **24**、max-w **180**、px **6** py **2**、全圆角、13px | — | `chat-input-bar.tsx:858,899` | 引用 |
| `@` 提及 / `/` 命令面板 | maxHeight **240**；宽 `min(360, max(220, 输入宽−24))` / `min(320, max(180, 输入宽−24))` | `Popover`（**宽度绑定锚点需自己量**） | `input/mentions/file-mention-dropdown.tsx:97`；`slash-command-dropdown.tsx:89`；`chat-input-bar.tsx:361,371` | 引用 |
| 上下文面板 / 技能面板 | maxHeight **320**（列表 max-h **264**）/ 320（编辑器态 **440**） | — | `selectors/context-selector.tsx:101,260`；`skills/skills-command.tsx:565` | 引用 |
| AI 偏好菜单 | 主 min-w **240**；agent **224**；provider **192**；model max-h **320** min-w **256** | `PopupMenu` | `input/chat-preferences-menu.tsx:63,179,197,323` | 引用 |
| ProviderApiKeyCommand | **430 × 560**、左列 **200** | `Dialog` + 自绘 grid | `provider-api-key-command.tsx:55,179` | 引用 |
| Agent 侧栏行 | HoverCard 宽 **288**；行 min-h **24**、px **8** pr **48**；pin/archive **20**、图标 **12** | — | `agent-session-sidebar-item.tsx:81,100,109,133` | 引用 |
| **命令面板 / 快速打开浮层** | max-h ≤ **512**（`min(68vh,32rem)`）、宽 ≤ **704**（`min(44rem, 100vw−2rem)`）、圆角 **11.2** | `Dialog` + `Command::bordered(false)` | `ui/command.tsx:29` | 引用 |
| 浮层定位 | 外层 `fixed inset-0 z-10060 flex items-start justify-center pt-16` → 顶距 **64** | ⚠️ gpui `Dialog` 垂直默认 `viewport/10`、水平居中 | `ui/command.tsx:143`；`gpui-component/src/dialog/dialog.rs:529,534` | 引用 |
| 浮层 header / 输入框 / 徽标 | px **16** py **12**；输入框高 **28**、`ui-text-base`；徽标 min-h **28**、max-w **160** | — | `ui/command.tsx:52,55,92` | 引用 |
| 列表项（default / compact） | min-h **32**、gap **10**、圆角 **8**、px **10** py **8** / min-h **28**、圆角 **6.4**、px **8** py **4** | `CommandItem` | `ui/command.tsx:41-42` | 引用 |
| 项图标 / 项徽标 / 空态 | **20** / max-w **128** / p **12** | — | `ui/command.tsx:554-555,567,727` | 引用 |
| 快速打开上限 | 去抖 **100ms**；`MAX_RESULTS 20`、`MAX_OPEN_BUFFERS_SHOWN 20`、`MAX_RECENT_FILES_NO_QUERY 10`、`MAX_OTHER_FILES_SHOWN 20` | `CommandState` 自行过滤 | `features/quick-open/constants/limits.ts:1-8` | 引用 |

### 1.9 圆角档位 / 字体尺寸档位 / 图标尺寸档位

**圆角档位**（`styles/theme.css:6-12,133-134`，✅已核对行号）

| 档 | 表达式 | 实际 px | 用途（实测） |
| --- | --- | --- | --- |
| `--lithe-chrome-radius` | 字面量 | **4** | 活动栏条目、标签、侧栏树行、页签基类 |
| `--radius-sm` | `--radius × 0.6` | **4.8** | 项目标签、右侧活动栏按钮、Markdown 代码块、引用树行 |
| `--radius-md` | `--radius × 0.8` | **6.4** | 状态栏 chip、提交/引用行、右键菜单面板、工具栏按钮、输入框 |
| `--radius-lg` | `--radius` | **8** | 侧栏分类头、终端会话区、查找栏、调试左栏回显框 |
| `--radius-xl` | `--radius × 1.4` | **11.2** | 编辑器岛、面板岛、底部窗、提交面板、诊断分组卡片 |
| `--radius-2xl` | `--radius × 1.8` | **14.4** | 计划卡、composer 初始态、菜单栏 compact 浮动层、轮播卡片 |
| `--radius-3xl` | `--radius × 2.2` | **17.6** | （仓库内未见使用点） |
| `--radius-4xl` | `--radius × 2.6` | **20.8** | （仓库内未见使用点） |
| `--app-scrollbar-radius` | 字面量 | **999** | 滚动条 thumb |

> ⚠️ `research/windows/04` §1.4 末尾那句"卡片/对话框/浮层用 `rounded-xl`(=12px，**Tailwind 默认**)"是**描述性残留**：本仓库把 Tailwind 整条圆角阶梯改写成了 `calc(var(--radius) × k)`，`rounded-xl` 实际是 **11.2px**。以 `theme.css:6-12` 为准。

**字体尺寸档位**

| 档 / 类 | 变量 | focused | comfortable | 证据 | 状态 |
| --- | --- | --- | --- | --- | --- |
| `.ui-text-caption` | `--ui-text-caption` | **12** | **13** | `styles/utilities.css:34-36`；`theme.css:114,189` | ✅已核对（theme.css 行号） |
| `.ui-text-chrome` | `--ui-text-chrome` | **13** | **14** | `styles/utilities.css:38-40`；`theme.css:115,190` | ✅已核对 |
| `.ui-text-sm` | `--ui-text-sm` = `--app-ui-font-size` | **13** | **13** | `styles/utilities.css:30-32`；`theme.css:116,112` | ✅已核对 |
| `.ui-text-base` | `--ui-text-base` = `--app-ui-font-size` | **13** | **13** | `styles/utilities.css:42-44`；`theme.css:117,112` | ✅已核对 |
| `.ui-text-lg` / `.ui-text-xs` | — | **未找到定义**（`utilities.css:30-44` 只有上面 4 个）→ **空操作 class**，字号靠继承 | — | `research/windows/02` §2.0 警告、§7.5 | 引用 |

> ⚠️ **`ui-text-sm`(13px) ≠ Tailwind `text-sm`(14px)**。两套并存、互不换算（`research/windows/04` §1.6）。重写时必须选定一套：建议**只搬 `ui-text-*` 的 px 字面量**（见 §5.2 的 `LitheMetrics` 建议），不要借 gpui 的 `text_sm()/text_base()`。
> **未找到** CSS 定义的 `--ui-text-lg` / `--ui-text-xs`；使用它们的位置（如 `ui/sidebar.tsx:43`、`agent-session-sidebar-item.tsx:170`）实际字号由继承决定。

| 组件专用字号 | 值 | 证据 |
| --- | --- | --- |
| 编辑器默认字号 / 行高 | **14** / `ceil(14 × 1.4)` = **20** | `editor/config/constants.ts:4-5`；`editor/utils/lines.ts:8-15` |
| 运行输出正文 | **12**（等宽） | `run-output-text.tsx:39` |
| Git 控制台 | **12**（等宽） | `git-execution-console.tsx:68` |
| Git 提交表日期单元格 | **11**（等宽） | `git-commit-table.tsx:324` |
| Git 提交表作者单元格 | **13**（继承） | `git-commit-table.tsx:321` |
| Git Log 标签行 / Log-Console 页签 | **12**（裸 `text-xs`，**与标题栏 13px 不一致**） | `git-log-tool-window.tsx:582` |
| Inspector 短哈希/作者/邮箱/日期 | **11** | `git-commit-inspector.tsx:174,178` |
| Inspector 完整哈希 / 标签徽章 / 未读徽标 / 「当前」徽章 / 文件夹尾计数 / ahead-behind | **10** | `git-commit-inspector.tsx:182`；`git-graph-row.tsx:88`；`notifications-trigger.tsx:45`；`git-reference-tree.tsx:652`；`git-status-panel.tsx:930`；`git-tracking-counts.tsx:36` |
| 诊断严重度图标 | **11**（尺寸，不是字号） | `diagnostics-pane.tsx:1126` |

**常用控件高度档（`ui/button.tsx:22-28`，与 `--lithe-chrome-control-height` 并存）**

| 语义 | class | px | 证据 |
| --- | --- | --- | --- |
| Button `default` / `icon` | `h-8` / `size-8` | **32** | `ui/button.tsx:22,26` |
| Button `xs` / `icon-xs` | `h-6` / `size-6` | **24** | `ui/button.tsx:23,27` |
| Button `sm` / `icon-sm` | `h-7` / `size-7` | **28** | `ui/button.tsx:24,28` |
| Button `lg` | `h-9` | **36** | `ui/button.tsx:25` |
| Button 默认图标 | `size-3.5` | **14** | `ui/button.tsx:9` |
| Badge | `h-6` | **24** | `ui/badge.tsx:6` |
| Kbd | `h-5` | **20** | `ui/kbd.tsx:9` |
| Checkbox / Radio | `size-4` | **16** | `ui/checkbox.tsx:10`；`ui/radio-group.tsx:23` |
| Switch sm / md | `h-3.5 w-7` / `h-5 w-9` | **14×28 / 20×36** | `ui/switch.tsx:24-30` |
| Spinner compact / 默认 | `size-3` / `size-4` | **12 / 16** | `ui/spinner.tsx:27` |
| Toggle xs/sm/md | `min-h-6/7/8` | **24/28/32** | `ui/toggle.tsx:15-17` |
| Progress sm / md | `h-1` / `h-1.5` | **4 / 6** | `ui/progress.tsx:5-18` |
| TableHead 高 | `h-8` | **32** | `ui/table.tsx:65` |
| 菜单分隔线 | `h-px` | **1** | `ui/context-menu.tsx:196` |
| 编辑器标签强调线 | `h-[3px]` | **3** | `ui/tab-bar.tsx:204-209` |

**滚动条档位**

| 项 | 值 | 证据 |
| --- | --- | --- |
| `--app-scrollbar-size` / `--app-scrollbar-thin-size` | **11** / **9** | `styles/scrollbars.css:2-3` |
| `--app-scrollbar-radius` | **999** | `styles/scrollbars.css:4` |
| thumb 最小长度 | **36** | `styles/scrollbars.css:66-67` |
| 轨道 | `transparent` | `styles/scrollbars.css:5` |
| ScrollArea 组件层（Base UI） | 宽/高 **10**（`w-2.5`）、thumb **6**（`w-1.5`）、hover 才显现 | `ui/scroll-area.tsx:125-134` |

> ⚠️ 两套滚动条并存（11px 常显 vs 10px hover 显现），且 **gpui-kit 的默认行为两者都不是**（rest 6px / hover 6px / active 8px / inset 4px，idle 2s / enter 300ms / exit 500ms / expand 300ms，`gpui-component/src/theme/mod.rs:64-86`）。本项目已用 `Theme::set_scrollbar_mode(ScrollbarMode::Always, cx)` 改成常显（见 §5.2）。

---

## 2. 逐区域对应表

> 每节四块：**A 组成与层级** / **B 度量**（↗ §1 的行号） / **C 元素 → gpui-kit 对应**（含坑） / **D 中文文案**（逐字取自 `windows/tauri/src/i18n/locale.ts`，给行号）。
> 判定档：**可一比一** = gpui-kit 有语义足够的直接对应；**需自行组合** = 有积木但要自己拼；**没有** = 必须自研或降级。

### ① 外壳（标题栏 + 窗口三键 + 项目标签条 + 左右活动栏 + 状态栏）

#### A. 组成与层级

```
MainLayout                                features/layout/components/main-layout.tsx:81
├─ 拖放遮罩（isDraggingOver）              :282-290
├─ TitleBarWithSettings                    :292
│  └─ TitleBar                             features/window/components/title-bar/title-bar.tsx:54
│     ├─ WindowMenuBar（Windows 自绘菜单栏；compactMenuBar 时浮动）  window-menu-bar.tsx:36 / title-bar.tsx:205-233
│     ├─ TitleProjectMenu（项目下拉）       title-bar.tsx:237 → title-project-menu.tsx:109
│     ├─ Git 分支项                        title-bar.tsx:238 → footer-git-branch-item.tsx:11
│     ├─ 快速打开按钮                       title-bar.tsx:242-254
│     ├─ TitleBarUpdateControl → AppUpdateControl（仅 Windows）  title-bar.tsx:50-52,347
│     ├─ WindowControls（最小化/最大化/关闭） title-bar.tsx:296-302,349-355 → window-controls.tsx:21
│     └─ SettingsDialog + ProjectPicker(portal)  title-bar.tsx:389-400
├─ ProjectTabBar（Windows 有）              :293 → project-tab-bar.tsx:16
├─ 工作区（rootFolderPath && !showWelcomeControl）  :295-355
│  └─ div.lithe-workbench-glass            :297
│     ├─ SidebarActivityRail expanded={false}   :302 → sidebar/main-sidebar.tsx:120
│     ├─ ResizablePane(position="left")    :303-310 → resizable-pane.tsx:32 → MainSidebar
│     ├─ 中间列（flex-1）                   :312
│     │  ├─ div.lithe-glass-island（编辑器岛）  :313
│     │  └─ BottomPane（terminalWidthMode==="editor"）  :318-322
│     ├─ ResizablePane(position="right")   :325-341
│     └─ PluginActivityRail（右侧图标竖条）  :342 → plugin-activity-rail.tsx:11
├─ Footer（showStatusBar 为真时）           :354 → footer/footer.tsx:16
├─ WelcomeScreen                           :356-358
└─ 全局浮层（QuickOpen / CommandPalette / … / TerminalHost）  :363-378
```

首帧占位外壳：`App.tsx:39-54`（`InitialWindowShell`：拖拽条 `h-10`=40 + 主区 `h-[calc(100dvh-2.5rem)]`）。gpui 首帧无 WebView 白屏问题，**不需要等价物**。

**已确认的死代码（重写时不必实现）**：`useFooterDebuggerItem`（`footer/footer-debugger-item.tsx:29`）、`FooterControlBadge`（`footer/footer-tab-control.tsx:23`）。

#### B. 度量

标题栏 40 / 三键 56×40 / 项目标签条 32 / 项目标签 28·144–240 / 左活动栏 38（展开 160、140–320）/ 右活动栏 38 / 状态栏 24 / chrome 控件高 24 / 工作区间隔 4 → **全部见 §1.1**。

#### C. 元素 → gpui-kit 对应

| Windows 元素 | gpui-kit 0.6.6 实现 | 判定 | 写法要点 / 坑 |
| --- | --- | --- | --- |
| 窗口 + 标题栏宿主 | `gpui_component::title_bar::TitleBar`（`title_bar.rs:42`）+ `TitleBar::window_options()`（`:81`）+ `gpui_component::root::Root`（`root.rs:37`） | **需自行组合** | ⚠️ **`TITLE_BAR_HEIGHT = px(34.)`**（`title_bar.rs:15`，✅已核对）必须用 `.h(px(40.))` 覆盖；底色用 `.bg(cx.theme()…)` 或 `StyleRefinement` 覆盖（Lithe 是 `--surface`）。**这条是必须覆盖的第一项** |
| 窗口三键 | `TitleBar` 内置的私有 `ControlIcon`（`title_bar.rs:110-245`） | **gpui-kit 没有可定制尺寸的公开 API** | 控件宽固定 `w(TITLE_BAR_HEIGHT)`=34（`title_bar.rs:211`），Lithe 要 **56×40 直角 + 关闭悬停整块 `--destructive`**。只能自绘按钮 + `gpui::WindowControlArea::Min/Max/Close`（`title_bar.rs:155-161`）挂在原生命中区上 |
| 标题栏菜单栏 | `gpui_component::menu::app_menu_bar::AppMenuBar`（`app_menu_bar.rs:26`）；原生走 `native_menu::windows` | **需自行组合** | Windows **不用系统菜单栏**（`title-bar.tsx:79`：`shouldUseNativeMenuBar = !isWindows && !isLinux && nativeMenuBar`）。Lithe 是自绘 24px 胶囊 + 20px 项 → 用 `menu::popup_menu::PopupMenu`（`popup_menu.rs:281`）自搭。`default-settings.ts:105` 默认 `compactMenuBar: true` |
| 标题栏项目下拉 | `menu::dropdown_menu::DropdownMenu`（`dropdown_menu.rs:12`）+ `PopupMenuItem`（`popup_menu.rs:71`） | **需自行组合** | gpui-kit 无"项目行（图标 + 名称 + 路径 + 勾）"预置行；用 `menu_element`（`popup_menu.rs:587`）自绘。面板 max-h **520**、宽 **384**、p **6**、圆角 **6.4** |
| 项目徽标（首字母色块 / 图标） | `gpui_component::avatar::Avatar`（`avatar/avatar.rs:17`） | **需自行组合** | Lithe 是 **28×28 圆角方块**（`title-project-menu.tsx:52,62-64`），Avatar 是圆形 + 角标语义 → 建议自绘 |
| **项目标签条** | `gpui_component::tab::{TabBar, Tab}` + `TabVariant::Underline`（`tab/tab.rs:14`） | **需自行组合** | gpui-kit `Tab` **无关闭按钮 / 无下划线指示器 / 无"单项目隐藏"**；关闭按钮走 `Tab::suffix`（`tab.rs:541`）。条高 36 与 Lithe 的 32 不符 → `.h(px(32.))`；标签高 28、宽 144–240、圆角 4.8 也要覆盖（gpui `Tab::height(Size)` 表：`tab.rs:24-43`，XSmall 20 / Small 24 / Medium 32 / Large 36） |
| 左侧活动栏（38px 图标竖条） | `gpui_component::sidebar::Sidebar`（`sidebar/mod.rs:238`）+ `sidebar::menu::SidebarMenuItem`（`sidebar/menu.rs:113`） | **需自行组合** | ⚠️ `Sidebar` 的 `DEFAULT_WIDTH = px(255.)` / `COLLAPSED_WIDTH = px(48.)`（`sidebar/mod.rs:27-28`），**都不是 38** → 要么 `.w(px(38.))` 硬覆盖，要么用 `button::Button` + `tooltip::Tooltip` 自建竖条。**gpui-kit 没有独立的"活动栏 / Activity Bar"组件** |
| 活动栏选中态 | `button::Toggle`（`button/toggle.rs:14`）或 `Button::toggled(bool)` | **需自行组合** | Lithe 选中底色是自定义 `--selected`（`ui/sidebar.tsx:243-244`），gpui-kit 无同名 token → 走自建 `LithePalette.selected`（§5.3） |
| 右侧插件竖条 | 同活动栏方案 | **需自行组合** | `plugin-activity-rail.tsx:34`：宽 38、**右上/右下圆角 11.2 + 1px 右边框** |
| 未读徽标 | `gpui_component::badge::Badge`（`badge.rs:31`）`.count(n)` / `.max(9)` | **可一比一** | `notifications-trigger.tsx:46` 的 `unreadCount > 9 ? "9+"` 即 `Badge::max(9)`。gpui 的 `count == 0` 自动隐藏（`badge.rs:104`） |
| 状态栏 | `gpui_component::status_bar::StatusBar`（`status_bar.rs:41`）`.left(..)`（`:51`）`.right(..)`（`:57`） | **可一比一** | ⚠️ `StatusBar` **无常量高度**，默认 `.py_1().px_2().text_xs().border_t_1()`（`status_bar.rs:86-95`）实际约 28px 且有上边框 → 必须 `.h(px(24.))` + 去边框。左右分组语义完全对应 |
| 状态栏数据 chip | `button::Button`（`ghost()` + `with_size(px(24.))` + `.tooltip(..)`）+ `separator::Separator::vertical()`（`separator.rs:9`） | **可一比一** | `gpui-kit 无 chip 级原语`；`Button` 是最接近的 |
| 状态栏文件路径面包屑 | `gpui_component::breadcrumb::{Breadcrumb, BreadcrumbItem}`（`breadcrumb.rs:13,20`） | **可一比一**（下拉需自行组合） | ⚠️ **`BreadcrumbItem` 不实现 `ParentElement`、没有内建下拉**（`breadcrumb.rs:20-27`）→ 分段点击弹目录树要换 `button::DropdownButton`（`button/dropdown_button.rs:43`） |
| 面板分隔条（左右可拖拽面板） | `gpui_kit::base::resizable::{h_resizable, ResizablePanelGroup, ResizablePanel}`（`resizable/mod.rs:17,22`；`panel.rs:31`） | **可一比一** | ✅ 已核对：`ResizablePanel::size(impl Into<Pixels>)`（`panel.rs:275`）、`.size_range(impl Into<Range<Pixels>>)`（`:283`）、`.visible(bool)`（`:269`）；`ResizablePanelGroup::{new, with_state, axis, size, on_resize, with_handle_appearance}`（`:43,67,73,101,111,59`）；`ResizableState::reset_panel(ix, cx)`（`resizable/mod.rs:233`）可复刻"双击重置"。⚠️ **`size()` 只吃 `Pixels`，没有百分比** → Windows 的 `19%/57%/24%` 要在首帧按容器宽换算 |
| 活动栏项目轮播（横滑切项目） | 无对应（`gpui_component::carousel` 面向内容轮播） | **没有** | 需用 `gpui_base::motion` 自实现（阈值：位移上限 `railWidth × 0.96`、超 `×0.82` 立即提交、否则 40ms 后按 **42px** 判定，`main-sidebar.tsx:530-586`） |
| 右键菜单 | `menu::context_menu::{ContextMenu, ContextMenuExt}`（`context_menu.rs:13,42`） | **可一比一** | ⚠️ **元素必须有稳定 `id`**，否则 fallback `ElementId::CodeLocation`，重渲染丢开关状态（`context_menu.rs:26-35`）。可勾选项 `PopupMenuItem::checked`（`popup_menu.rs:180`） |
| 工具提示 | `tooltip::Tooltip`（`tooltip.rs:34`）`.action(&dyn Action, Option<&str>)`（`:69`） | **需自行组合** | ⚠️ **没有公开的通用 `.tooltip()` 扩展**（`ManagedTooltipExt` 与 `Root::tooltip_overlay` 都是 `pub(crate)`）→ 通用元素只能手搓 `Tooltip::new` + `on_hover`；带 `.tooltip()` 的组件只有 Button / Toggle / Checkbox / Radio / Switch / Clipboard / InputGroup |
| 窗口边框 / 缩放热区 | `gpui_component::window_border::WindowBorder`（`window_border.rs:25`）`.resize_hit_size`（`:63`）+ `WindowOptions.window_min_size` | **可一比一** | 5px 边 / 10px 角 → `resize_hit_size(px(5.))`；最小 720×480 → `window_min_size` |
| 全局浮层宿主 | `gpui_component::root::Root`（`root.rs:37`）+ `window_ext::WindowExt`（`window_ext.rs:109`） | **可一比一** | ⚠️ **必须先挂层**：`Root::render_dialog_layer / render_sheet_layer / render_notification_layer`，否则对话框静默不显示。⚠️ **`open_dialog` 不能在窗口首帧之前调用，会 panic**。详见 `UI-MAP.md` §1.3 |
| 首帧占位外壳 | `gpui_component::skeleton` | 不需要 | gpui 首帧无白屏 |

**尺寸体系决策（必读）**：`Theme` 只有一个 `font_size`，它同时决定 `window.set_rem_size()`（`gpui-component/src/root.rs:582`）与 `TypographyTokens.md.size`。所以 `Theme::font_size` 应设为 **`px(16.0 × uiFontSize / 13.0)`**，与 Windows 的根字号语义对齐；而 Lithe 专用的 `ui-text-sm/base/chrome/caption` **必须用显式 px**，不能借 `text_sm()/text_base()`（见 `research/windows/04` §3.3）。

**Button / Input 的 `Size` 阶梯与 Lithe 不一致**（`research/windows/04` §3.3）：gpui 的 `XSmall/Small/Medium/Large` = 按钮 **20/24/32/32**、输入 **20/24/32/44**；Lithe 是 **24/28/32/36**。→ **不要用 `Size` 枚举表达 Lithe 的尺寸档**，改用自建 `LitheControlSize { Xs, Sm, Md, Lg }` 直接给 `Pixels`。

#### D. 中文文案（逐字取自 `windows/tauri/src/i18n/locale.ts`）

语言目录：`DISPLAY_LANGUAGES = ["en-US", "zh-CN"]`（`locale.ts:2`）；中文目录自 `locale.ts:4468` 起。

| 键 | 中文原文 | `locale.ts:行号` |
| --- | --- | --- |
| `titleProject.trigger` | 项目：{project} | 6191 |
| `titleProject.newProject` | 新建项目… | 6171 |
| `titleProject.newWindow` | 新建窗口 | 6172 |
| `titleProject.addProject` | 添加项目 | 6173 |
| `titleProject.open` | 打开… | 6174 |
| `titleProject.openFolder` | 打开文件夹 | 6175 |
| `titleProject.openFolderInNewWindow` | 在新窗口中打开文件夹 | 6176 |
| `titleProject.openInNewWindow` | 在新窗口中打开 | 6177 |
| `titleProject.closeAllProjects` | 关闭所有项目 | 6178 |
| `titleProject.selectIcon` | 选择图标 | 6179 |
| `titleProject.setDisplayAlias` | 设置显示别名 | 6180 |
| `titleProject.displayAliasPrompt` | 输入显示别名，将显示在文件夹名之后，用于区分同名项目。 | 6181 |
| `titleProject.removeProject` | 移除项目 | 6182 |
| `titleProject.currentProject` | 当前项目 {project} | 6183 |
| `titleProject.switchToProject` | 切换到 {project} | 6184 |
| `titleProject.switchToProjectMenu` | 切换到项目 | 6185 |
| `titleProject.cloneRepository` | 克隆仓库… | 6186 |
| `titleProject.openProjects` | 打开的项目 | 6187 |
| `titleProject.closeProject` | 关闭项目 {name} | 6188 |
| `titleProject.recentProjects` | 最近项目 | 6189 |
| `titleProject.noRecentProjects` | 没有最近项目 | 6190 |
| `projectOpen.title` | 打开项目 | 6165 |
| `workbench.currentFile` | 当前文件 | 6148 |
| `workbench.moreProjectActions` | 更多项目操作 | 6149 |
| `window.menu` | 菜单 | 7903 |
| `window.minimize` / `window.maximize` / `window.restore` / `window.close` | 最小化 / 最大化 / 还原 / 关闭 | 7899-7902 |
| `update.available` / `update.installing` / `update.failed` / `update.options` | 有可用更新 / 正在安装 / 更新失败 / 更新选项 | 6444 / 6442 / 6443 / 6449 |

**菜单栏（`menu.*`，全部中文键见 `locale.ts:7724-7822`）**

| 键 | 中文 | 键 | 中文 |
| --- | --- | --- | --- |
| `menu.file` | 文件 | `menu.edit` | 编辑 |
| `menu.view` | 视图 | `menu.go` | 转到 |
| `menu.terminal` | 终端 | `menu.run` | 运行 |
| `menu.tools` | 工具 | `menu.window` | 窗口 |
| `menu.help` | 帮助 | `menu.toggleActivitySidebar` | 切换活动侧栏 |
| `menu.toggleSecondarySidebar` | 切换辅助侧栏 | `menu.toggleTerminal` | 切换终端 |
| `menu.toggleMenuBar` | 切换菜单栏 | `menu.toggleFullscreen` | 切换全屏 |
| `menu.minimize` | 最小化 | `menu.maximize` | 最大化 |
| `menu.preferences` | 首选项 | `menu.keyboardShortcuts` | 键盘快捷键 |
| `menu.webInspector` | Web 检查器 | `menu.theme` | 主题 |

**活动栏**

| 键 | 中文原文 | `locale.ts:行号` | 状态 |
| --- | --- | --- | --- |
| `workbench.project` | 项目 | 4605 | ✅已核对 |
| `workbench.changes` | 更改 | 4606 | ✅已核对 |
| `workbench.search` | 搜索 | 4607 | ✅已核对 |
| `workbench.gitLog` | 提交记录 | 5910 | 引用 |
| `workbench.sourceControl` | 源代码管理 | 5909 | 引用 |
| `workbench.settings` | 设置 | 5912 | 引用 |
| `workbench.run` | 运行 | 5913 | 引用 |
| `workbench.maven` | Maven | 5914 | 引用 |
| `workbench.terminal` | 终端 | 5915 | 引用 |
| `workbench.diagnostics` | 诊断 | 5916 | 引用 |
| `workbench.activityViews` | 活动视图 | 7890 | 引用 |
| `layout.actions` | 操作 | 4700 | 引用 |
| `layout.showInActivitySidebar` | 在活动侧栏中显示 | 4702 | 引用 |
| `layout.projectDots` | 项目圆点 | 4703 | 引用 |
| `layout.resizeActivityRail` | 调整活动栏大小 | 4701 | 引用 |
| `layout.resizeSidebar` | 调整侧边栏大小 | 7887 | 引用 |
| `layout.resizeAiChat` | 调整 AI 聊天大小 | 7886 | 引用 |
| `files.copyPath` | 复制路径 | 7388 | 引用 |
| `files.reveal` | 在资源管理器中显示 | 7400 | 引用 |
| `extensions.title` | 扩展 | 7566 | 引用 |
| `notifications.title` | 通知 | 7322 | 引用 |
| `run.title` | 运行 | 5927 | 引用 |
| `maven.title` | Maven | 6043 | 引用 |
| `welcome.openProject` | 打开项目 | 6154 | 引用 |
| Maven 项标签（拼接） | `` `${t("run.title")} - ${t("maven.title")}` `` = 「运行 - Maven」 | `main-sidebar.tsx:200` | 引用 |

**状态栏**

| 键 | 中文原文 | `locale.ts:行号` | 状态 |
| --- | --- | --- | --- |
| `footer.statusBar` | 状态栏 | 7316 | ✅已核对 |
| `footer.filePath` | 文件路径 | 7317 | ✅已核对 |
| `footer.gitBranch` | Git 分支 | 7318 | ✅已核对 |
| `footer.cursor` | 跳转到行和列 | 7305 | ✅已核对 |
| `footer.encoding` | 文件编码 | 7306 | ✅已核对 |
| `footer.indent` | 缩进 | 7307 | ✅已核对 |
| `footer.spaces` | {count} 个空格 | 7308 | ✅已核对 |
| `footer.readOnly` / `footer.writable` | 只读 / 可写 | 7309 / 7310 | ✅已核对 |
| `footer.memory` | 内存 | 7311 | ✅已核对 |
| `footer.memoryUsage` | 总计 {total} · Lithe {used} | 7312 | ✅已核对 |
| `footer.changes` / `footer.change` | {count} 个更改（单复数同文案） | 7313 / 7314 | ✅已核对 |
| `footer.noChanges` | 没有更改 | 7315 | ✅已核对 |
| `footer.moveLeft` / `footer.moveRight` / `footer.resetOrder` | 左移 / 右移 / 重置页脚顺序（**死代码**） | 7319 / 7320 / 7321 | ✅已核对 |
| `preparation.starting` | 正在启动 Java 服务 | 5917 | 引用 |
| `preparation.importing` | 正在导入 Java 项目与依赖 | 5918 | 引用 |
| `preparation.configuring` | 正在同步 Java 项目配置 | 5919 | 引用 |
| `preparation.building` | 正在构建 Java 项目 | 5920 | 引用 |
| `preparation.ready` | Java 项目模型已就绪 | 5921 | 引用 |
| `preparation.stopped` | Java 项目准备已停止 | 5922 | 引用 |
| `preparation.failed` | Java 项目准备失败，请查看详情 | 5923 | 引用 |
| `preparation.explanation` | Java 工作区准备完成后才能启动。普通后台索引不阻塞运行，编译在运行前执行。 | 5924 | 引用 |
| `preparation.settings` / `preparation.logs` | 语言服务设置与重试 / 查看日志 | 5925 / 5926 | 引用 |
| 无更改图标 | 仅图标（`CheckCircleIcon`），**无文案** | `footer-editor-status.tsx:135` | 引用 |
| `ui.open` / `ui.cancel` / `ui.save` / `ui.delete` | 打开 / 取消 / 保存 / 删除 | 4616 / 4609 / 4617 / 4618 | ✅已核对 |

**外壳内未本地化的硬编码英文（重写时需注意，不要照抄成中文）**：`Drop folder to open project, or file to open buffer`（`main-layout.tsx:286`）、`Show All`（`main-sidebar.tsx:756`）、`Opening {project}`（`main-sidebar.tsx:625`）、`You're on the latest version` / `Failed to check for updates`（`use-menu-events-wrapper.ts:340,342`）。

---

### ② 侧栏与项目树

#### A. 组成与层级

```
SidebarActivityRail（活动栏 rail）          sidebar/main-sidebar.tsx:120
└─ SidebarPaneSelector orientation="vertical"  :632 → sidebar-pane-selector.tsx:81
    └─ nav[aria-label=workbench.activityViews]  sidebar-pane-selector.tsx:358
        ├─ 上组 div（flex-1 overflow-y-auto gap-1）  :359 → SidebarListItem :323 → ui/sidebar.tsx:217
        └─ 下组 div（shrink-0 gap-1 pt-1）        :363
MainSidebar                                  main-sidebar.tsx:767
└─ div[data-external-file-drop-scope="sidebar"]  :814
    └─ SidebarPanel                            :815 → ui/sidebar.tsx:11
        └─ 三选一（coreFeatures + activeSidebarView）  :778-816
            ├─ GitView（git）                    :784-789 → git/components/git-view.tsx:82
            ├─ FileExplorerPane（files）          :794-796 → file-explorer/components/file-explorer-pane.tsx:9
            │   └─ FileExplorerTree               file-explorer-tree.tsx:1108
            │       ├─ SidebarHeader.file-explorer-header  :1309-1518
            │       │   ├─ h2.file-explorer-header-title（「项目」）  :1314
            │       │   ├─ SidebarSearchPopover      :1317-1343
            │       │   ├─ 清空按钮（有查询时）        :1344-1356
            │       │   └─ 偏好 DropdownMenu          :1357-1517
            │       ├─ FileExplorerViewport role="tree"  :1519-1523 → file-explorer-viewport.tsx:169
            │       │   └─ FileExplorerTreeItem       file-explorer-tree.tsx:1579 → file-explorer-tree-item.tsx:88
            │       │       └─ SidebarTreeRow          features/sidebar/components/sidebar-tree.tsx:183
            │       │           ├─ SidebarTreeGuides（缩进参考线）  sidebar-tree.tsx:18
            │       │           ├─ SidebarTreeDisclosure（三角）    sidebar-tree.tsx:290
            │       │           ├─ ThemedFileIcon（文件图标）        file-explorer-tree-item.tsx:239
            │       │           └─ span + mark.file-tree-search-highlight  :244-252
            │       ├─ contextMenuElement             :1612
            │       └─ 3 个 Dialog                   :1613,1632,1667
            └─ GlobalSearchBuffer compact（search）   :797-804 → global-search-buffer.tsx:519（详见 ⑦）
```

**先决结论（否则会从错误前提开工）**：

| 事实 | 证据 |
| --- | --- |
| **不存在 `SearchSidebar` / `search-sidebar` 组件**；侧栏的「搜索」就是同一个 `GlobalSearchBuffer` 传 `compact` | `main-sidebar.tsx:797-804`；`global-search-buffer.tsx:51-54` |
| 「标签」不是文件，而是 **buffer**（内容类型 24 种） | `features/panes/types/pane-content.types.ts:33-57`；`features/tabs/components/tab-bar.tsx:271-303`；`features/tabs/utils/path-shortener.ts:31-113` |
| 标签/窗格状态**不在 Core**：buffer 列表 / 激活 / pin / preview / 窗格树全在渲染进程 zustand，会话持久化到 `lithe-tab-sessions` | `features/editor/stores/buffer.store.ts:620-700`；`window/stores/session.store.ts:256-266`；`panes/stores/pane.store.ts:244-600` |
| 侧栏单击/双击文件**都不是 preview 态**（`handleFileSelect` 的 `isPreview` 默认 `false`，双击注释明写 "definite mode (not preview)"） | `file-explorer-tree.tsx:1007-1010`；`file-system/stores/file-system.store.ts:1533-1539,1965-1967` |

#### B. 度量

侧栏头部 32 / 行高 24（公式）/ 缩进 `10 + depth × 16`（默认步长 16）/ 参考线 7 宽·1 线·`left:3` / 折叠三角 16·12 / 文件图标 16 / 内联重命名 px 6 py 4 → **全部见 §1.3**。

#### C. 元素 → gpui-kit 对应

| Windows 元素 | gpui-kit 0.6.6 实现 | 判定 | 写法要点 / 坑 |
| --- | --- | --- | --- |
| 侧栏容器（可折叠 + 头部/底部 + 列表） | `component::sidebar::{Sidebar, SidebarCollapsible, SidebarMenuItem, SidebarMenu, SidebarGroup, SidebarHeader, SidebarFooter}`（`sidebar/mod.rs:238,264,270,276,282`；`sidebar/menu.rs:113`） | 可折叠侧栏**可一比一**；**独立图标 rail 没有** | ⚠️ **`Sidebar` 没有 `impl Sizable`**（`sidebar/mod.rs` 无 `Sizable`）；宽度靠 `DEFAULT_WIDTH 255` / `COLLAPSED_WIDTH 48` 与 `.w(px(..))` |
| **项目文件树** | `gpui_kit::base::tree::{Tree, TreeState, TreeItem, TreeEntry, TreeEntryState, TreeEvent}`（`gpui-base/src/tree.rs:468,184,99,50,164,93`） | **需自行组合（推荐这条）** | 层级 / 展开折叠 / 选中 / 键盘 / 虚拟滚动（底层 `uniform_list`，`:423`）全内建；但**缩进、图标、展开箭头、缩进参考线全部要自己在 `render_item` 里画**（渲染循环 `:427-441` 只暴露 `entry.depth()`，不加任何 padding）。键盘内建 `up/down/left/right`（`:20-27`），`key_context() == "Tree"`（`:30`）。`.reveal_item(&SharedString, ScrollStrategy, cx)`（`:268`）对应"自动定位活动文件" |
| 同一棵树的 component 版 | `component::tree::Tree::new(&Entity<TreeState>, R: Fn(usize, &TreeEntry, bool, ..) -> ListItem)`（`component/tree.rs:41`） | **不推荐** | ⚠️ 强制返回 `ListItem`，而 `ListItem` 固定 `py_1()`（`list/list_item.rs:186`）→ **拿不到精确 24px 行高**。折叠菜单 `Tree::context_menu(F)`（`:55`） |
| 平铺列表（搜索结果 / 运行配置） | `component::list::{List, ListState, ListDelegate, ListItem, SeparatorItem, IndexPath}`（`list/list.rs:70,721`；`list/list_item.rs:26`；`list/delegate.rs:10`） | **可一比一** | 行高必须一致（见下）；分页用 `has_more` / `load_more_threshold`（`delegate.rs:148,159`） |
| 内联重命名输入框 | `component::input::{Input, InputState, InputEvent}`（`input/input.rs:180`；`input/base/state.rs:8958`） | **可一比一** | `InputEvent::PressEnter { secondary, shift }` / `Blur` 驱动提交（`state.rs:122-127`）；`.focus(&self, window, cx)`（`:1255`）+ `.select_all(window, cx)`（`:2835`）复刻"rAF 后全选" |
| 空状态（图标+标题+描述） | `component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle, EmptyDescription, EmptyContent}`（`empty.rs:23,89,167,157,231,279,328`） | **可一比一** | ⚠️ 渲染顺序契约 `media → title → description`（`empty.rs:138-152`）；**根默认有虚线边框**（`:65-78`），Lithe 没有 → `.border_0()`；**无 action/按钮槽** → 按钮放 `EmptyContent` 或直接 `.child(Button..)` |
| 右键菜单 | `menu::{ContextMenuExt, ContextMenu, PopupMenu, PopupMenuItem}` | **可一比一** | 见 ① |
| 加载浮层（Spinner 胶囊） | `component::spinner::Spinner`（`spinner.rs:10`） | **可一比一** | 胶囊容器自绘 |
| 复选框（变更列表用） | `component::checkbox::Checkbox`（`checkbox.rs:17`） | **可一比一** | ⚠️ **无"仅悬停显示"语义** → 用 `on_hover` 控 wrapper 的 `opacity` |

**三条必须核实的 gpui-kit 结论（照抄 `UI-MAP.md` §1.3 / `research/windows/02` §6.3）**

1. **`uniform_list` 只测量第 0 行**：`item_to_measure_index: 0`（`gpui-pre-0.3.6/src/elements/uniform_list.rs:41-43`），`measure_item` 只渲染 1 个 item（`:658-680`），该高度用于全部布局（`:359,371,397,427-428,473-476,506,520`）。→ **行高必须 uniform，且 `.h()` 压不住内容**（行内纵向 padding 会让内容溢出到相邻行）。
   Windows 的树行高是固定 24（`file-tree-row.ts:7-14`），**正好满足**；且 Windows 允许 `fileTreeIndentSize` 变缩进而**不变行高**，安全。
2. **`ListItem` 的 children 是竖排**：根是 `h_flex()`（`list/list_item.rs:44-48`），children 被塞进**普通块级 `div()`**（`:215-232`），所以"折叠箭头 + 图标 + 名字"会各占一行。→ **必须自己套一层 `h_flex()`**。
   ⚠️ 且 `ListItem` 固定 `py_1()`（`:186`），**不可能靠 `ListItem` 复刻 24px 行高** → 用 `base::tree::Tree` + 自绘 `AnyElement` 行。
3. **变高行要用 `VirtualList`**：`gpui-base/src/virtual_list.rs:10` 明确 "Unlike the `uniform_list`, the each item can have different size"（`v_virtual_list` `:139`）；`component::virtual_list` 也重新导出了 `VirtualList/h_virtual_list/v_virtual_list`（`gpui-component/src/lib.rs:122`）。
   ⚠️ `List` 要求所有 item 同高，**section header/footer 也算 item，所以虚拟列表不能加 `gap_y`**（`list/list.rs:564-566`；`list/delegate.rs:41`）。

#### D. 中文文案

**文件树**（全部取自 `locale.ts`）

| 键 | 中文原文 | `locale.ts:行号` |
| --- | --- | --- |
| `fileExplorer.preferences` | 文件资源管理器偏好设置 | 7445 |
| `fileExplorer.searchFiles` | 搜索文件 | 7446 |
| `fileExplorer.visibility` | 可见性 | 7448 |
| `fileExplorer.gitignoredFiles` | 被 Git 忽略的文件 | 7449 |
| `fileExplorer.gitStatusDecorations` | Git 状态标记 | 7450 |
| `fileExplorer.fileIcons` | 文件图标 | 7451 |
| `fileExplorer.indentGuides` | 缩进参考线 | 7452 |
| `fileExplorer.indentation` / `indentationCompact` / `indentationDefault` / `indentationSpacious` / `indentationWide` | 缩进 / 紧凑 / 默认 / 宽松 / 宽 | 7453-7457 |
| `fileExplorer.noFolderOpen` | 未打开文件夹 | 7458 |
| `fileExplorer.searchingFiles` | 正在搜索文件 | 7459 |
| `fileExplorer.noMatchingFiles` | 没有匹配的文件 | 7460 |
| `fileExplorer.folderIsEmpty` | 文件夹为空 | 7461 |
| `fileExplorer.moveFailed` | 移动失败 | 7462 |
| `fileExplorer.opening` | 正在打开... | 7473 |
| `fileExplorer.folderAlreadyExists` / `folderAlreadyExistsMessage` | 文件夹已存在 / 已存在同名文件夹。 | 7468 / 7469 |
| `fileExplorer.fileAlreadyExists` / `fileAlreadyExistsMessage` | 文件已存在 / 已存在同名文件。 | 7470 / 7471 |
| `fileExplorer.deleteFolder` / `deleteFile` / `deleting` | 删除文件夹 / 删除文件 / 正在删除... | 7476 / 7477 / 7478 |
| `fileExplorer.deleteFolderMessage` | 确定要删除文件夹"{name}"及其所有内容吗？此操作无法撤销。 | 7479 |
| `fileExplorer.deleteFileMessage` | 确定要删除文件"{name}"吗？此操作无法撤销。 | 7480 |
| `files.openInTerminal` | 在终端中打开 | 7426 |
| `files.rename` | 重命名 | 7399 |
| `files.reveal` | 在资源管理器中显示 | 7400 |
| `files.copyPath` / `files.copyRelativePath` | 复制路径 / 复制相对路径 | 7388 / 7392 |
| `files.newFile` / `files.newFolder` | 新建文件 / 新建文件夹 | 7403 / 7404 |
| `files.fileNamePlaceholder` / `files.folderNamePlaceholder` | 文件名 / 文件夹名 | 7410 / 7411 |
| `files.renameItem` | 重命名 {name} | 7412 |
| `files.symlinkTo` | 符号链接到：{target} | 7413 |
| `files.findInFolder` | 在文件夹中查找（**当前为空实现**） | 7430 |
| `files.refresh` | 刷新 | 7418 |
| `files.collapseAll` | 全部折叠 | 7423 |
| `settings.files.hiddenFiles` | 隐藏文件 | 6510 |
| `quickOpen.loadingFiles` | 正在加载文件 | 7835 |
| `welcome.openProject` | 打开项目 | 6154 |

**活动栏与布局**：见 ① 的表（`workbench.project` = 项目 / `workbench.changes` = 更改 / `workbench.search` = 搜索 / `layout.actions` = 操作 / `layout.projectDots` = 项目圆点 等）。

**硬编码英文（重写时应补 i18n 键，或保留原文）**：`Show All`（`main-sidebar.tsx:756`）、`Opening {project}`（`main-sidebar.tsx:625`）、`Indexing N files` / `Indexing files` / `Preparing search` / `Searching a/b files` / `Searching files` / `Loading more results`（`global-search-buffer.tsx:453,457,462,466,469,474`）、`N results (M total)`（`:490`）、`Load all search results before replacing all`（`:540`）、`Showing x of y results`（`global-search-results.tsx:102`）。

---

### ③ 编辑区（内部标签栏 / pane / 空态）

#### A. 组成与层级

```
main-layout.tsx:295-355
├─ SidebarActivityRail expanded={false}       :302
├─ ResizablePane(position="left")             :303-310
├─ <div .lithe-glass-island rounded-xl border-l bg-background>   :312-313  ← 编辑区「玻璃岛」
│  └─ WorkbenchErrorBoundary > CachedWorkspaceSplitViews         :314-315
│      └─ SplitViewRoot（每 workspace 一份，最多缓存 3 个）        split-view-root.tsx:12,46,98-118
│          └─ PaneNodeRenderer                pane-node-renderer.tsx:54
│              ├─ PaneContainer（group 叶子）  pane-container.tsx:319
│              │  ├─ TabBar                   tabs/components/tab-bar.tsx:61
│              │  ├─ EmptyEditorState（无 activeBuffer 时）  pane-container.tsx:1100 → empty-editor-state.tsx:10
│              │  ├─ CodeEditor（默认分支）     pane-container.tsx:1050-1058
│              │  ├─ 其它 buffer 渲染器        pane-container.tsx:924-1059
│              │  ├─ SplitDropOverlay          pane-container.tsx:1089-1093
│              │  └─ 拖入高亮层                 pane-container.tsx:1086-1088
│              └─ PaneResizeHandle（同级）      pane-node-renderer.tsx:115-124 → pane-resize-handle.tsx:13
├─ BottomPane（pane id = "bottom-pane"）      :318-322 / :345-351
└─ Footer（状态栏）                            :354
```

TabBar 自身：

```
TabBar                                        tabs/components/tab-bar.tsx:61
├─ TabDndContext                              ui/tab-bar.tsx:30（PointerSensor distance=5）
│  └─ TabBarSurface（role="tablist"，bg-background）  tab-bar.tsx:626-632 / ui/tab-bar.tsx:284
│     ├─ div.flex.h-8（左导航组 gap-0.5）       tab-bar.tsx:633
│     │  ├─ Button(ArrowLeft 后退, icon-xs)     :634-646
│     │  └─ Button(ArrowRight 前进, icon-xs)    :647-659
│     ├─ SortableContext > div（横向滚动容器 gap-0.5）  :662-663
│     │  └─ SortableTab → ContextMenu > TabBarItem  tab-bar.tsx:665,675-694
│     │      └─ TabBarItem                     tabs/components/tab-bar-item.tsx:55
│     │         ├─ 拖放指示条 div.drop-indicator（可选）  :134-136
│     │         └─ TabBarTab（role="tab"）      ui/tab-bar.tsx:269
│     │            └─ Tab（variant=connected, action=关闭/固定按钮）  ui/tab-bar.tsx:220
│     │               ├─ 图标槽 div.grid.size-3  tab-bar-item.tsx:182-254
│     │               ├─ 标签文字 span / InlineRenameInput  :255-283
│     │               └─ 脏标记 div.size-2.rounded-full  :284-291
│     └─ div.flex.h-8（右操作组 gap-1 pl-0.5）  tab-bar.tsx:763
│         ├─ MarkdownModePicker（仅 editor buffer）  :764-766
│         ├─ Button(Plus 新建标签, hover 才显)   :767-784
│         ├─ Button(PanelLeftClose 关闭拆分, 仅分屏时)  :785-797
│         └─ Button(Maximize2/Minimize2 全屏, hover 才显)  :798-815
└─ div.sr-only role="status"（无障碍播报）      :821-823
```

编辑器宿主（`code-editor.tsx:647-773`）：`Breadcrumb`（`h-7`）→ `ExternalConflictBanner` → 大文件提示条 → `div.editor-container`（`CodeLensOverlay` / `SignatureHelpTooltip` / `RenameInput` + 主编辑面）。

**`features/diff/` 目录不存在**；编辑区里的 diff 是 `features/git/components/diff/*`，经懒加载 `DiffViewer` 挂进 pane 的 `case "diff"`（`pane-container.tsx:77,951-952`）。

#### B. 度量

标签条 36 / 标签 28·80–200 / 强调线 3 / 面包屑 28 / 编辑器 padding 8/16/8/16 / 行高 `ceil(14×1.4)` = 20 / 空态 448 / 分隔条 4 / 窗格最小比例 10 → **全部见 §1.4**。

#### C. 元素 → gpui-kit 对应

| Windows 元素 | gpui-kit 0.6.6 实现 | 判定 | 写法要点 / 坑 |
| --- | --- | --- | --- |
| **内部标签栏** | `component::tab::{TabBar, Tab, TabVariant}`（`tab/tab_bar.rs:59,80,104`；`tab/tab.rs:14,478`） | **需自行组合** | ✅ 已核对：`TabBar::{new, with_variant, pill, outline, segmented, underline, menu, max_width, track_scroll, selected_index, on_click, prefix, suffix, child, children, last_empty_space}`（`tab_bar.rs:59,80,86,92,98,104,110,118,127,157,171,133,139,151,145,163`）；`Tab::{new, label, icon, prefix, suffix, disabled, on_click}`（`tab.rs:478,483,499,535,541,547,553`）。⚠️ **`Tab::icon()` 会丢掉 label**，图标+文字必须走 `Tab::child(...)`（`UI-MAP.md` §1.3）。⚠️ **`track_scroll` 不会自动把选中标签滚进视野**（`tab_bar.rs:124-126` 文档）→ 自己 `scroll_to_item`
| 活动标签 3px 强调线 | `TabVariant::Underline` 只有 **2px**（`tab/tab_bar.rs:274`） | **需自行组合（自绘）** | 要精确 3px / 左右内缩 6 / `rounded-t-sm` 必须自绘底部条 |
| 标签脏标记 / Pin | **任何 crate 都没有**（`tab.rs:397-419` 无 dirty/pin 字段；dock `TabGroup` 亦无） | **没有** | 脏点自绘 8px `--primary` 圆；Pin 用关闭按钮位替换 |
| 标签拖拽重排 | `TabBar` 无 `on_drag` | **没有（唯一参考实现）** | 现成逻辑只在 dock：`component/dock/tab_panel.rs:150`（`fn tab_drag`）、`:401,525,581`；或 `base::dock::TabGroup`（`gpui-base/src/dock/tab_group.rs:145`）+ `TabGroupContext::{drag_panel,drop_panel,drop_item}`（`:855,863,876`）。复刻参数：5px 激活距离 / `pointerWithin → closestCenter` / 180ms / rAF 点击抑制 |
| **编辑器宿主容器** | `component::dock::{DockArea, DockSkin, Panel, PanelView, TabGroup, InsertTarget}`；文档控件 `component::input::{Editor, EditorState, EditorMode}`（`input/editor.rs:39`） | **需自行组合** | 用 `DockArea::split_at`（`gpui-base/src/dock/dock_area.rs:580`）做分屏；`.set_zoomed_in/.set_zoomed_out/.is_zoomed`（`:626,635,639`）对应"窗格全屏"；`dock/mod.rs:59` 的 `actions!(dock, [ToggleZoom, ClosePanel])` |
| 编辑器本体 | `component::input::EditorState`（= `gpui-base/src/input/editor/mod.rs:11`；`EditorState::new` 在 `input/base/state.rs:9230`） | **可一比一（且强于 Monaco 方案）** | ✅ 自带行号列 / 折叠 / 缩进线 / tree-sitter 高亮 / 诊断波浪线 / 装饰 / 多光标 / 撤销栈 / 内建查找替换（含 `2/3` 计数）；`InputEditorStyle::{editor_active_line, editor_gutter_background}` 可配；上限约 5 万行。⚠️ **多行输入必须显式给高度**：`.h(relative(1.))`，否则只有 `rows` 默认 2 行高（`input/input.rs:706-709`、`input/base/mode.rs:83-85`）。⚠️ **编辑器不吃滚轮，宿主必须自己接** `on_scroll_wheel` → `InputBaseState::set_scroll_offset`（`input/base/state.rs:2806-2817`）。缺 minimap / overview ruler / sticky scroll / gutter 装饰渲染器 / code lens / LSP 客户端 / Java grammar |
| 底部状态栏 / 面板工具条 | `component::status_bar::StatusBar`；`Panel::{title_bar, inner_padding}` 返回 `false` 时 Dock 不画自己的标题栏（`component/dock/panel.rs:146,137`；`dock/tab_panel.rs:703-715`） | **可一比一** | ✅ 已核对 `Panel::title_bar` 在 `component/dock/panel.rs:146`、`inner_padding` 在 `:137`（**`UI-MAP.md` §1.3 写的 `:131-148` 覆盖了这两行，正确**） |
| 面包屑 | `component::breadcrumb::{Breadcrumb, BreadcrumbItem}`（`breadcrumb.rs:119,31`） | **需自行组合** | 分隔符硬编码 `IconName::ChevronRight` + `size_3p5()`（`:141-147`）；**不解析路径、不折叠中间段**；长路径省略要自己算。下拉见 ① |
| 文件拖入高亮 | `ThemeColor::drop_target` / `drag_border`（`theme/theme_color.rs:161,159`） | **可一比一** | — |
| 分屏分隔条 | `base::resizable` 的 `ResizablePanelGroup` + `with_handle_appearance(ResizeHandleRenderer)`（`resizable/panel.rs:59`） | **可一比一（外观需微调）** | 分隔条度量/配色：`HANDLE_PADDING = px(4.)`（`resizable/resize_handle.rs:11`）、`HANDLE_SIZE = px(1.)`（`:12`）；取色 `:286-295`（`theme.resizable.handle` → fallback `theme.tokens.colors.border`；激活 fallback `ring`）。`PANEL_MIN_SIZE = px(100.)`（`resizable/mod.rs:14`）—— Windows 的 `MIN_PANE_SIZE = 10` 是**比例**，语义不同需换算 |
| 空态 | `component::empty::*` | **可一比一** | 见 ② |
| 加载 / 恢复失败卡片 | `component::spinner::Spinner` + 自绘 | **可一比一** | — |
| 拖放五区（左/右/上/下/中） | 无对应 | **没有** | `SplitDropOverlay` 要自绘（`inset-1` 4px / `rounded-lg` 8 / `border-2 --primary` / `bg-primary/14` / 150ms） |
| 缓冲轮播（横向卡片） | `component::carousel` 面向内容轮播 | **需自行组合** | 卡片默认 640 / min 320 / max 视口−160；保活上限 8 |

#### D. 中文文案

| 键 | 中文原文 | `locale.ts:行号` |
| --- | --- | --- |
| `tabs.openFiles` | 打开的文件 | 7849 |
| `tabs.goBack` / `tabs.goBackShort` | 后退到上一个位置 / 后退 | 7850 / 7854 |
| `tabs.goForward` / `tabs.goForwardShort` | 前进到下一个位置 / 前进 | 7851 / 7855 |
| `tabs.newTab` | 新建标签页 | 7852 |
| `tabs.toggleEditorFullScreen` | 切换编辑器全屏 | 7853 |
| `tabs.exitFullScreen` / `tabs.fullScreenEditor` | 退出全屏 / 编辑器全屏 | 7858 / 7859 |
| `tabs.closeSplit` / `tabs.closeSplitPane` | 关闭拆分 / 关闭拆分窗格 | 7856 / 7857 |
| `tabs.pin` / `tabs.unpin` | 固定标签页 / 取消固定标签页 | 7838 / 7839 |
| `tabs.close` / `tabs.closeOthers` / `tabs.closeToRight` / `tabs.closeAll` | 关闭 / 关闭其他 / 关闭右侧 / 全部关闭 | 7845-7848 |
| `tabs.splitRight` / `tabs.splitDown` | 向右拆分 / 向下拆分 | 7840 / 7841 |
| `tabs.lockEditorGroup` / `tabs.unlockEditorGroup` | 锁定编辑器组 / 解锁编辑器组 | 7842 / 7843 |
| `tabs.reload` | 重新加载 | 7844 |
| `tabs.rename` | 重命名 {name} | 7863 |
| `tabs.unsavedChanges` | 未保存的更改 | 7864 |
| `tabs.ariaUnsavedSuffix` / `ariaPinnedSuffix` / `ariaPreviewSuffix` | （未保存）/（已固定）/（预览） | 7860 / 7861 / 7862 |
| `tabs.announcementUnsaved` | ，未保存 | 7865 |
| `tabs.switchedTo` | 已切换到 {name}{unsaved} | 7866 |
| `tabs.activated` | 已激活 {name}{unsaved} | 7867 |
| `tabs.closed` | 已关闭 {name} | 7868 |
| `tabs.cannotClosePinned` | 无法关闭已固定标签页 {name} | 7869 |
| `workbench.emptyEditorTitle` | 选择文件以查看 | 6150 |
| `workbench.emptyEditorDescription` | 外部工具产生的更改会自动显示。 | 6151 |
| `ui.noActionsHere` | 此处无任何内容 | 4630（**✅已核对**） |
| `ui.loading` / `ui.retry` | 正在加载 / 重试 | 4647 / 4628（**✅已核对 `ui.retry`**） |
| `editor.restoreLoadFailed` | 无法加载此文件。 | 4641 |
| `editor.externalConflict` | 该文件已在 Lithe 外部更改，同时编辑器中还有未保存的修改。 | 4639 |
| `editor.externalDeleted` | 该文件已被外部删除，编辑器内容已保留。 | 4643 |
| `editor.recreateFile` / `editor.keepEditorVersion` / `editor.loadDiskVersion` | 允许重新创建文件 / 保留编辑器内容 / 加载磁盘版本 | 4644 / 4645 / 4646 |
| `editor.largeFileServicesDisabled` | 此大文件已关闭语言智能、Git Blame、代码镜头和语义高亮。 | 8408 |
| `editor.enableLargeFileServices` | 仍然启用 | 8409 |
| `editor.actions` | 编辑器操作 | 8404 |
| `editor.preview` | 预览 | 8133 |
| `editor.aiInlineEdit` | AI 行内编辑 | 8398 |
| `editor.findInFile` | 在文件中查找 | 8400 |
| `editor.goToLineColumn` | 转到行和列 | 8401 |
| `editor.symbolPath` | 符号路径 | 8402 |
| `editor.unknownFile` | 未知 | 8403 |
| `editor.openHtmlInBrowser` / `editor.openHtmlInBrowserFailed` | 在浏览器中打开 HTML / 无法在浏览器中打开 HTML | 8134 / 8135 |
| `panes.resizePanes` | 调整窗格大小 | 4699 |
| `panes.resizeBufferCarouselCards` | 调整缓冲区轮播卡片大小 | 4681 |
| `panes.noPreviewAvailable` | 没有可用预览 | 4685 |
| `panes.terminalSession` / `panes.webView` | 终端会话 / Web 视图 | 4672 / 4673 |
| `panes.diffPreview` / `panes.imagePreview` / `panes.pdfPreview` / `panes.binaryFilePreview` | 差异预览 / 图片预览 / PDF 预览 / 二进制文件预览 | 4677 / 4678 / 4679 / 4680 |
| `panes.databaseViewer` | {type} 查看器 | 4682 |
| `panes.externalEditorSession` | 外部编辑器会话 | 4683 |
| `panes.missingDatabaseConnection` | 缺少数据库连接 | 4698 |
| `panes.webViewerDisabled` / `panes.webViewerDisabledDescription` | Web 查看器已禁用 / 在「设置 > 功能」中启用后，可以在嵌入式编辑器标签页中打开 URL。 | 4695 / 4696 |
| `panes.pullRequest` / `pullRequestNumber` / `filesCount` / `commitsCount` / `commentsCount` | 拉取请求 / 拉取请求 #{number} / {count} 个文件 / {count} 个提交 / {count} 条评论 | 4686 / 4674 / 4687 / 4688 / 4689 |
| `panes.description` / `panes.files` / `panes.comments` | 描述 / 文件 / 评论 | 4690 / 4691 / 4692 |
| `panes.issueNumber` / `panes.workflowRunNumber` | 议题 #{number} / 工作流运行 #{number} | 4675 / 4676 |
| `layout.resizeBottomPane` | 调整底部面板大小 | 7888 |
| `layout.sidebarSections` | 侧边栏分区 | 7889 |
| `keybindings.commands.workbench.showFind.title` / `showFindReplace.title` | 查找 / 查找和替换 | 8334 / 8335 |
| `terminal.tabNamePlaceholder` | 终端名称 | 7517 |

---

### ④ Git 工具窗 + 底部窗

#### A. 组成与层级

**挂载位置（关键结论）**：底部窗**两种模式**，默认不是整窗占满 —— `terminalWidthMode === "editor"`（默认）时挂**中央列**内、横向宽度 = 窗宽 − 左活动栏 − 左侧栏 − 右工具窗 − 插件栏，**不覆盖两侧栏**（`main-layout.tsx:318-322`）；`"full"` 时挂 `lithe-workbench-glass` 下单列，**横向跨越整个工作台**，左右各留 4px（`main-layout.tsx:345-351`）。

```
BottomPane                                   bottom-pane.tsx:28
├─ div[ref=paneFrameRef] flex shrink-0 flex-col   height: calc(Npx + var(--lithe-workbench-gap))  :322-334
│  ├─ resizeGutter   role="separator" aria-orientation="horizontal"  高 4px  :222-241
│  └─ paneContent
│     └─ div[data-bottom-pane-drop-target].lithe-glass-island
│         relative flex min-h-0 flex-col overflow-hidden rounded-xl
│         border-border/70 border-t border-l bg-background                :244-254
│        └─ div.h-full.overflow-hidden                                    :255
│           ├─ TerminalContainer（始终挂载，非激活时 "hidden"）              :257-264
│           ├─ DebuggerView / RunPane / MavenRunPane                       :266-284
│           ├─ DiagnosticsBuffer（showCloseButton，onClose=隐藏底部窗）      :286-295
│           ├─ BottomBufferPane（activeTab==="buffers" 且有 buffer）         :297-301
│           └─ GitLogToolWindow（activeTab==="gitLog"）                      :303-307
└─ isFullScreen → WorkbenchFullscreenSurface 包裹                            :312-316
```

**底部窗没有自己的标签条**：标签由外部触发（活动栏回调 `main-sidebar.tsx:642-652`、命令面板 `view-actions.tsx:110-157`、状态栏/标题栏按钮），内部用 `useUIState.bottomPaneActiveTab` 单值切换（`bottom-pane.tsx:30-31`）。`"references"` 一旦成为激活标签会被**立即关闭**底部窗（`bottom-pane.tsx:62-66`）。

Git Log 工具窗（`git-log-tool-window.tsx:78`）：`GitLogTitleBar`（`:570-580`）→ 标签行（日志 `git.console.log` / 控制台 `git.console.title`，`:582-587`）→ `[panel==="console"] GitExecutionConsole` / `[panel==="log"]` 三栏 `ResizablePanelGroup orientation="horizontal"`（`:614-621`）：`references 19% / commits 57% / inspector 24%`；Inspector 内再 `v_resizable` 分 `files 62% / details 38%`。标签切换是**本地 `useState<"log" | "console">`**（`:80`），`fetchReferences()` 会主动切到 `"console"`（`:452`）。

控制台（`git-execution-console.tsx:15`）：左 32px 按钮栏（查找 / 自动换行 / 滚动到底 / 取消 / 清空 / 复制）+ 查找条 + 可滚动输出（`GitConsoleEntry` × N）。

变更列表（`git-view.tsx:82`）：**不在底部窗里**，是左侧栏的一个视图 —— `SidebarTitleBar` + `SidebarTabBar items=[changes, history]` + `GitStatusPanel` + `SidebarFooter`（内含 `GitCommitPanel`）。

#### B. 度量

底部窗默认 320（含 gap 324）/ min 200 / max 80% 窗高；Git Log 标题栏 32；提交表行高 **30**、作者 **112**、日期 **128**、内容 min-w **520**；三栏 19/57/24，min 140/320/220；引用行 `10 + depth × 14`；控制台左栏 32 → **全部见 §1.5**。

#### C. 元素 → gpui-kit 对应

| Windows 元素 | gpui-kit 0.6.6 实现 | 判定 | 写法要点 / 坑 |
| --- | --- | --- | --- |
| **工具窗容器（底部、可拖高 200–80% 窗高、可全屏）** | `component::dock::DockSkin::dock_area(..)`（`component/dock/mod.rs:132`）+ `base::DockArea::set_dock(DockPlacement::Bottom, ..)`（`dock/dock_area.rs:246`）+ `DockLayout::tabs()` + `DockPlacement::Bottom` + `set_dock_collapsible` / `set_dock_size` / `toggle_dock` / `set_zoomed_in`（`:330,348,301,626`） | **可一比一** | dock **无需 feature 门控**（`component/lib.rs:43` 是 `pub mod dock;`）。⚠️ **gpui-kit 没有"80% 窗高"上限概念**，要在 `ResizablePanelGroup::on_resize`（`resizable/panel.rs:111`）里自行 clamp。⚠️ dock 面板标题条高度硬编码 **`px(30.)`**（`dock/tab_panel.rs:372`），与 Windows 的 28/36 不一致 |
| 若不使用 dock，只做「上编辑区 + 下工具窗」 | `component::{v_resizable, resizable_panel, ResizableState, ResizablePanel, ResizablePanelEvent}` | **可一比一** | 但"拖动条 4px + hover 变主色"要自己实现视觉 |
| 工具窗内面板头 | `Panel::title(..)`（`component/dock/panel.rs:82`）+ `Panel::toolbar_buttons() -> Option<Vec<Button>>`（`:101`）+ `title_style()`（`:87`）/ `title_suffix()`（`:92`）+ `PanelControl`（`:45`） | **可一比一** | 高度 30px 与 Windows 的 28/36 不一致 → 需自定义 skin |
| Git Log 标题栏（32 / 图标 14 / 按钮 24 / 右侧只读文本） | `h_flex().h(px(32.)).gap_2().px_2()` + `component::icon::Icon`（`icon.rs:84`）+ `Button::ghost().with_size(px(24.))` | **可一比一** | — |
| 「引用名」胶囊按钮（24 / max-w 240 / 圆角 6.4） | `Button::outline().with_size(px(24.)).max_w(px(240.))` | **可一比一** | — |
| Log/Console 标签行 | `component::tab::{TabBar, Tab}` | **可一比一（外观会变）** | ⚠️ Windows 现状是**无选中样式**（只有 `aria-selected`）；gpui-kit 的 Tab **一定带样式** → 迁移时必须挑最接近的变体并接受外观变化（或自绘） |
| **提交表** | `component::table::{DataTable, TableState, TableDelegate}`（`table/data_table.rs:101`） | **可一比一** | ✅ 已核对：`DataTable::{new, stripe, bordered, scrollbar_visible}`（`:101,109,115,121`）；`Size::table_row_height()` = XSmall **26** / Small **30** / Medium **32** / Large **40**（`sizing.rs:57-65`，✅已核对）→ **提交表 30px 正好 `Size::Small`**。列：`Column::new(key,name)` + `.width(px(112.))` / `.text_right()` / `.min_width(..)` / `.max_width(..)` / `.resizable(..)`；列宽变更事件 `TableEvent::ColumnWidthsChanged(Vec<Pixels>)` |
| ⚠️ **`Column` 没有 flex** | 只有 `width/min_width/max_width` | — | **列宽按给定值比例铺满表宽** → 必须把 Windows 的"剩余宽度给提交列"折算成具体像素 |
| ⚠️ **`DataTable` 行高是表级值，不能逐行设** | `TableState` 统一读 `self.options.size.table_row_height()` | — | **不能逐行设高**。提交表行高受首行内容影响 → 想让行高正好 30，必须让单元格自然高度也是 30 |
| 泳道图（自绘 SVG：竖线/贝塞尔/圆点） | gpui 原生 `canvas(...)` 或 `svg()` 路径元素 | **没有** | gpui-kit 无 graph/泳道组件。常量：`ROW_HEIGHT 30` / `LANE_GAP 13` / `GRAPH_PADDING 8` / 线宽 1.6 / 节点 `r=4.3` 描边 2 |
| 提交表多选（Ctrl/Shift）+ 选中背景 `primary/22` | 状态自管；行外观由 `TableDelegate::render_tr/render_td` 自绘 | **需自行组合** | ⚠️ `TableState`/`ListState`/`TreeState` **只有单选**。唯一例外：`SearchableListState` 本身支持多选（`searchable_list/state.rs:134-219`） |
| 提交表筛选输入 + 字段下拉 + 计数 | `component::input::{Input, InputState}` + `component::select::{Select, SelectState}` | **可一比一** | — |
| 装饰开关（Eye/EyeSlash，`aria-pressed`） | `component::button::Toggle` 或 `Button::toggled(bool)` | **可一比一** | ⚠️ **默认图标集没有 `eye-off`？** —— 实际**有**（`eye.svg`、`eye-off.svg` 都在 `default-icons.txt`）。见 §4 图标缺口表 |
| **引用树**（分区 → 分组 → 引用，24px 行高，缩进 `10+14·depth`，折叠状态持久化） | `component::tree::Tree` + `base::tree::{TreeState, TreeItem}` | **可一比一** | ⚠️ `TreeState` 单选；行高仍是 uniform 语义 → 需保证所有行同高（引用树正好全部 `h-6`） |
| 引用树左侧竖排 32px 按钮栏（自适应溢出到 HoverCard） | `v_flex().w(px(36.))` + `Button.with_size(px(32.))`；溢出用 `component::hover_card::HoverCard`（`hover_card.rs:47-48`：延迟 600ms/300ms，默认 `Anchor::TopCenter`） | **需自行组合** | ⚠️ **gpui-kit 无 `ResizeObserver` 对应物** → 用 `base::measure` / `on_children_prepainted` 或 `Element::request_layout` 后读 `bounds` |
| 引用树右键菜单（分组 + 子菜单 + 危险项 + disabled） | `PopupMenu` + `PopupMenuItem` | **可一比一（危险色需自绘）** | `PopupMenu::menu_with_icon`（`popup_menu.rs:543`）、`menu_with_check`（`:565`）、`menu_element_with_disabled`（`:596`）、`separator()`（`:680`）、`submenu_with_icon`（`:705`）、`min_w(px(288.))`（`:429`）。**gpui-kit 没有 `variant="destructive"`** |
| 「当前」徽章 / 泳道标签徽章 / ahead-behind | `component::tag::Tag` + `component::badge::Badge` + `component::icon::Icon` | **可一比一** | — |
| Inspector 提交文件树（24px 行、`10+14·depth`、图标、状态色尾标） | 同引用树的 `Tree`；或 `component::list::{List, ListState, ListDelegate, ListItem}` | **可一比一** | 两者都只有单选；"选中一个路径用于预览"正好够用 |
| Inspector 详情区 | `v_flex().gap_2().p_3()` + `component::description_list::DescriptionList` | **可一比一** | — |
| 变更列表分节头（32px、`bg-accent/80`、计数 Badge） | `accordion`（`accordion.rs:18`）或 `collapsible`（`collapsible.rs:11`）+ `Tag`/`Badge` | **可一比一** | — |
| 变更列表文件行（20px 悬停出现的暂存按钮 + Checkbox + 缩进） | `component::checkbox::Checkbox` + 自定义行 | **需自行组合** | ⚠️ **Checkbox 无"仅悬停显示"语义** → 用 `on_hover` 控 wrapper 的 `opacity` |
| **变更列表虚拟滚动（变高行：文件 24 / 分组 32）** | `component::{VirtualList, v_virtual_list, h_virtual_list}`（`component/lib.rs:122`；`base/virtual_list.rs:139`） | **可一比一** | ⚠️ **不要用 `uniform_list`**（分组 32 / 文件 24 不等高）。`gpui_base::VirtualList` 支持不同高度（`:1-12`） |
| 控制台左侧按钮栏 + 查找条 + 等宽输出 | `v_flex/h_flex` + `Button.with_size(px(24.))` + `Input/InputState` + `font_family(mono)`；输出用 `v_virtual_list` | **需自行组合** | ⚠️ 每条记录行数不定 → **不能用 `uniform_list`**。**没有 `LogView` / `Console` / `OutputView` / `LogPanel` / `ConsoleView`** |
| 控制台输出行内的「片段折叠 + 命中高亮 + 右键复制」 | gpui 的 `StyledText` / `TextRun` 分段着色 + `PopupMenu` | **没有** | gpui-kit 无"输出折叠"组件 |
| 控制台「Ctrl+F 查找 / 上下一个 / 计数」 | `component::input::search` + 自定义 Action（`gpui_kit::actions!`） | **需自行组合** | gpui-kit 的 `Input` 自带 search 能力，但**不是**"全量输出查找 + 高亮 + 折叠自动展开" |
| 贴底跟随的输出区 | `component::message_scroller::{MessageScroller, MessageScrollerState}`（`message_scroller.rs`：`is_following_tail` / `jump_button` / `with_bottom_fade`） | **可一比一** | `gpui-kit 明显优于手搓实现`（`UI-MAP.md` §1 末表） |
| Diff hunk 头 / Diff 视图（unified / split / 空白字符 / 逐 hunk 暂存） | `component::highlighter` 可提供语法高亮，行渲染需自建 | **没有** | gpui-kit **没有 diff 组件**；hunk 头左 gutter **44**，高度 = 编辑器行高公式 |
| 布局持久化（`git-log-preferences` / `git-log-preferences` localStorage） | `component::global_state::GlobalState` 或自建 JSON；写点用 `ResizablePanelEvent::Resized` / `TableEvent::ColumnWidthsChanged` | **需自行组合** | 持久化 key 见 `research/windows/03` §4.4 |
| 多仓暂存/贮藏校验与 toast | `component::notification` 或 `base::toast` | **可一比一** | — |
| `aria-*` / `role` 可访问性标注 | gpui 有 `role(..)` / `aria_selected` / `aria_expanded` / `accessibility_label`；**数值型 ARIA（`aria-valuenow/min/max`）未见对应 API** | **需自行组合或放弃** | `base/tree.rs:429-437` 是 `role` 用法示例 |

#### D. 中文文案

**Git Log 标题栏 / 面板**：`workbench.gitLog`=提交记录（5910）、`git.log.all`=全部（7188）、`git.log.logLabel`=日志：{name}（7189）、`git.log.showAll`=显示全部引用（7190）、`git.log.refresh`=刷新 Git 日志（7191）、`git.log.settings`=打开 Git 日志设置（7219）、`git.log.hide`=隐藏提交记录（7192）、`footer.readOnly`=只读（7309）、`git.console.log`=日志（4556）、`git.console.title`=控制台（4555）、`git.log.unableToRefresh`=无法刷新 Git 日志。（7196）、`git.log.retry`=重试（7197）、`git.log.noRepository`=未打开仓库（7198）、`git.log.openWorkspace`=打开一个 Git 工作区以查看提交记录。（7199）、`git.log.loading`=正在加载 Git 日志…（7200）、`ui.noActionsHere`=此处无任何内容（4630）、`git.log.copied`=已复制{label}（7203）、`git.log.copyFailed`=无法复制{label}（7204）、`git.log.commitHash`=提交哈希（7205）、`git.log.commitMessage`=提交说明（7206）。

**提交表格**：`git.log.filter`=筛选 Git 日志（7276）、`git.log.filterPlaceholder`={field} 筛选（7282）、`git.log.filterText`=文本（7279）、`git.log.filterAuthor`=作者（7280）、`git.log.filterBranch`=分支（7281）、`git.log.filterField`=Git 日志筛选字段（7278）、`git.log.clearFilter`=清除 Git 日志筛选（7277）、`git.log.showDecorations`=显示分支和标签（7284）、`git.log.hideDecorations`=隐藏分支和标签（7283）、`git.log.commit`=提交（7285）、`git.log.author`=作者（7286）、`git.log.date`=日期（7287）、`git.log.noMatch`=没有符合筛选条件的提交（7288）、`git.log.noCommits`=此视图中没有提交（7289）、`git.log.openDiffHint`=双击或按 Enter 打开提交差异（7290）、`git.log.openCommitDiff`=打开提交差异（7291）、`git.log.compareWithHead`=与 HEAD 比较（7194）、`git.log.copyCommitHash`=复制提交哈希（7292）、`git.log.copyCommitMessage`=复制提交说明（7293）、`git.log.loadingCommits`=正在加载提交…（7294）、`git.log.loadMore`=加载更多提交（7295）、`git.squashCommits`=压缩提交（8617）、`git.historyReview.contiguousRequired`=压缩需要选中当前分支上的连续提交（8616）、`git.patch.create`=创建补丁…（8549）、`git.patch.twoCommits`=导出补丁需要恰好选中两个提交（8560）、`git.undoCommit`=撤销提交（8588）、`git.historyReview.headRequired`=仅 HEAD 提交可撤销（8615）、`git.rebasePlan.fromHere`=从这里交互式变基…（6842）、`git.editCommitMessage`=编辑提交消息（8544）、`git.deleteCommit`=删除提交（8547）、`git.resetToCommit`=重置到此处（8620）、`git.cherryPickCommit`=Cherry-pick 提交（8625）。

**Inspector**：`git.log.commitFiles`=提交文件（7296）、`git.log.filesCount`={count} 个文件（7297）、`git.log.loadingShort`=加载中…（7303）、`git.log.selectCommit`=选择一个提交（7298）、`git.log.loadingChangedFiles`=正在加载更改的文件…（7299）、`git.log.unableToLoadFiles`=无法加载更改的文件（7300）、`git.log.noChangedFiles`=没有更改的文件（7301）、`git.log.commitDetails`=提交详情（7302）、`git.log.openFileDiff`=双击打开文件差异（7304）。

**引用树**：`git.log.references`=引用（7207）、`git.log.headCurrentBranch`=HEAD（当前分支）（7220）、`git.log.local`=本地（7252）、`git.log.remote`=远程（7253）、`git.log.tags`=标签（7254）、`git.log.none`=无（7255）、`git.log.expand`=展开 {name}（7256）、`git.log.collapse`=折叠 {name}（7257）、`git.current`=当前（6948）、`git.aheadOfRemote`=领先远端分支 {count} 个提交（6918）、`git.behindRemote`=落后远端分支 {count} 个提交（6919）、`git.log.toolbar.newBranch`=新建分支（7208）、`git.log.toolbar.updateSelected`=更新所选（7209）、`git.log.toolbar.deleteBranch`=删除分支（7210）、`git.log.toolbar.compareWithCurrent`=与当前分支比较（7211）、`git.log.toolbar.fetch`=提取（7212）、`git.log.toolbar.mark`=标记（7213）、`git.log.toolbar.unmark`=取消标记（7214）、`git.log.toolbar.showMyBranches`=显示我的分支（7215）、`git.log.toolbar.showAllBranches`=显示全部分支（7216）、`git.log.toolbar.goToHead`=导航到所选分支 HEAD（7217）、`git.log.toolbar.moreActions`=更多操作（7218）、`git.log.newBranchFrom`=从"{branch}"新建分支...（7221）、`git.log.checkoutAndRebaseOnto`=签出并变基到"{branch}"（7224）、`git.log.checkoutAndUpdate`=签出并更新（7227）、`git.log.compareWithCurrent`=与"{branch}"比较（7229）、`git.log.showDiffWithWorkingTree`=显示与工作树的差异（7230）、`git.log.rebaseCurrentOnto`=将"{current}"变基到"{branch}"（7231）、`git.log.mergeIntoCurrent`=将"{branch}"合并到"{current}"中（7232）、`git.log.pullRebaseIntoCurrent`=使用"变基拉入""{branch}"（7233）、`git.log.pullMergeIntoCurrent`=使用"合并拉入""{branch}"（7234）、`git.log.newWorktreeFrom`=从"{branch}"新建工作树...（7237）、`git.log.updateBranch`=更新（7240）、`git.log.trackingBranch`=跟踪的分支（7241）、`git.log.renameBranch`=重命名...（7243）、`git.log.deleteRemoteBranch`=删除远程分支（7245）、`git.log.stopTrackingBranch`=停止跟踪分支（7242）、`git.log.noRemoteBranches`=没有远程分支（7248）、`git.log.manageRemotes`=管理远程...（7247）。

**控制台**：`git.console.find`=在 Git 控制台中查找（4473）、`git.console.wrap`=自动换行（4493）、`git.console.scrollToEnd`=滚动到底部（4494）、`git.console.cancel`=取消正在运行的操作（4557）、`git.console.clear`=清空（4558）、`git.console.copy`=复制输出（4559）、`git.console.copyPath`=复制仓库路径（4479）、`git.console.copyCommand`=复制完整命令（4480）、`git.console.copyOutput`=复制完整输出（4481）、`git.console.details`=命令详情（4470）、`git.console.previousMatch`=上一个匹配（4474）、`git.console.nextMatch`=下一个匹配（4475）、`git.console.closeSearch`=关闭查找（4476）、`git.console.searchLimit`=前 {count} 个位置（4478）、`git.console.empty`=Git 命令及其输出将显示在这里。（4560）、`git.console.historyTruncated`=为限制内存占用，较早的 Git 执行记录已截断，无法展开。（4471）、`git.console.presentationUnavailable`=暂时无法压缩，正在显示完整保留的输出。（4482）、`git.console.queued`=等待启动（4561）、`git.console.running`=正在运行（4562）、`git.console.completed`=已完成（4566）、`git.console.exit`=退出码（4568）、`git.console.truncated`=为限制内存占用，已省略较早的输出。（4569）、`git.console.unconfirmed`=未收到已完成的命令记录（4567）、`git.console.options`=Git 命令选项（4472）、`git.console.matches`={count} 处匹配（4477）、`git.console.referenceChanges`=其余引用变化：新增 {added}、更新 {updated}、删除 {deleted}（4484）、`git.console.automaticQueries`=自动查询 {count} 次，结果相同（4483）、`git.console.arguments.files`=… 另 {count} 个文件（4485）、`git.console.arguments.references`=… 另 {count} 个引用（4486）、`git.console.fold.lines`=展开中间 {count} 行（4487）、`git.console.fold.repeat`=重复 {count} 次（4488）、`git.console.fold.files`=其余 {count} 个文件（4489）、`git.console.fold.branches`=其余 {count} 个分支（4490）、`git.console.fold.tags`=其余 {count} 个标签（4491）、`git.console.fold.commits`=其余 {count} 条提交（4492）、`git.console.notice.waitingForOutput`=正在执行 Git，等待输出。（4563）、`git.console.notice.completedWithoutOutput`=执行成功，没有输出。（4564）、`git.console.notice.fetchUnchanged`=获取完成，没有引用变化。（4565）。

**变更列表 / 提交面板**：`workbench.sourceControl`=源代码管理（5909）、`workbench.changes`=更改（4606）、`git.history`=历史记录（6814）、`git.viewDiff`=查看差异（6816）、`git.chooseDiffSource`=选择差异来源（7153）、`git.unstaged`=未暂存（8643）、`git.staged`=已暂存（8644）、`git.commit`=提交（6832）、`git.branch`=分支（8645）、`git.stash`=贮藏（8646）、`git.tracked`=已跟踪（6817）、`git.untracked`=未跟踪（6818）、`git.trackedFiles`=已跟踪文件（7156）、`git.untrackedFiles`=未跟踪文件（7157）、`git.workingTreeClean`=工作区干净（7158）、`git.stashAllUnstaged`=贮藏全部未暂存（8648）、`git.stageAllChanges`=暂存所有更改（7154）、`git.unstageAllChanges`=取消暂存所有更改（7155）、`git.rollback`=回滚（7160）、`git.showDiff`=显示差异（7162）、`git.jumpToSource`=跳转到源（7163）、`git.delete`=删除（6956）、`git.addToVcs`=添加到 VCS（7168）、`git.addToGitignore`=添加到 .gitignore（7169）、`git.addToLocalExclude`=添加到 .git/info/exclude（7170）、`git.gitignoreFile`=.gitignore（7173）、`git.localExcludeFile`=.git/info/exclude（7174）、`git.diffFileCount`={count} 个文件（7130）、`git.selectSingleRepositoryForStash`=请选择同一个仓库中的更改进行贮藏。（7152）、`git.stashSelected`=搁置选中的更改（7177）、`git.stashSelectedDefault`=搁置选中的更改（7151）、`git.stashAllUnstagedChanges`=贮藏所有未暂存的更改（7150）、`git.stashMessageDefaultSelection`=说明（默认：搁置选中的更改）（8653）、`git.stashMessageDefaultAll`=说明（默认：贮藏全部未暂存更改）（8651）、`git.noRepositorySelected`=未选择仓库（7010）、`git.loadingGitStatus`=正在加载 Git 状态（7013）、`git.statusLoadFailed`=无法加载 Git 数据，请重试刷新仓库。（7011）、`git.historyLoadFailed`=无法加载 Git 历史。（8536）、`git.actions`=Git 操作（8641）、`git.refresh`=刷新（6944）、`git.refreshAria`=刷新 Git 状态（8642）、`git.commitMessagePlaceholder`=提交说明...（6819）、`git.filesSelected` / `git.fileSelected`=已选择 {count} 个文件（6827 / 6828）、`git.noFilesSelected`=没有选择文件（6829）、`git.commitCount` / `git.commitsCount`={count} 个提交（6930 / 6931）、`git.noCommits`=暂无提交（6932）。

**差异视图**：`git.diff.changedFiles`=已更改文件（7146）、`git.diff.uncommitted`=未提交的更改（8738）、`git.diff.unstagedChanges`=未暂存的更改（8681）、`git.diff.stagedChanges`=已暂存的更改（8682）、`git.diff.showWhitespace`=显示空白字符（8664）、`git.diff.hideWhitespace`=隐藏空白字符（8665）、`git.diff.lineEndingChanges`=换行符不同（8666）、`git.diff.stage`=暂存（8736）、`git.diff.unstage`=取消暂存（8737）、`git.loadingDiff`=正在加载差异（7118）、`git.noDiffData`=没有可用的差异数据（7119）。

> **`git.fetch.options` 与 `git.expandAll` / `git.collapseAll` / `git.checkout` / `git.push` / `git.deleteBranch` / `git.deleteBranchConfirm` / `git.actionCompleted` / `git.actionFailed` / `git.unifiedView` / `git.splitView` 的 zh 行号未核定**（英文键存在，zh 区段未逐行扫描）—— 见 §7 未确认清单。

**底部窗 / 布局**：`layout.resizeBottomPane`=调整底部面板大小（7888）、`layout.resizeActivityRail`=调整活动栏大小（4701）、`layout.resizeSidebar`=调整侧边栏大小（7887）、`layout.resizeAiChat`=调整 AI 聊天大小（7886）、`layout.actions`=操作（4700）。

---

### ⑤ 终端

#### A. 组成与层级

```
TerminalContainer                          features/terminal/components/terminal-container.tsx:36
├─ div.terminal-container                  :697-701
│  └─ div（isVertical ? flex-row : flex-col）  :702
│     ├─ TerminalTabBar orientation={tabLayout}  :703-705
│     ├─ div.flex-1（会话区）                :707-717
│     │  └─ terminalSessions → TerminalSession（split 时左右/上下各一）  :624-692
│     │     └─ TerminalErrorBoundary → TerminalSlot   terminal-session.tsx:82-94
│     └─ TerminalTabBar（vertical 且 position==="right"）  :719-721
TerminalHost（挂到 App 根，portal 所有 xterm 实例）  terminal-host.tsx:14
└─ XtermPortal × N → createPortal(<XtermTerminal/>)  :64,84-108,133-147
XtermTerminal                              terminal.tsx:75
├─ TerminalSearch（绝对定位右上）             :861-869 → terminal-search.tsx:22
├─ ContextMenu（复制/粘贴右键菜单）           :871-916
└─ div.xterm-container（xterm.js 挂载点）     :877-888
```

**进程 / 通道结论**：终端进程由 **Rust 侧（Tauri host）** 用 `portable-pty` 0.9 打开 PTY 并 `spawn_command`（`windows/tauri/crates/terminal/src/connection.rs:36-49`）；Windows 后端是 **ConPTY**（`terminal-options.ts:44-47`）。前端 xterm.js 6 **只做渲染/输入编码，不 spawn 进程**。输出走 **Tauri v2 `ipc::Channel<TerminalEvent>`**（不是 `emit`/`listen`，`src-tauri/src/terminal.rs:108,116`）；输入/尺寸/暂停/关闭走普通 command `terminal_write` / `terminal_resize` / `terminal_set_paused` / `close_terminal`。**`lithe-core` 里没有任何 `terminal.*` 命令**（与 `AGENTS.md`「Windows-only terminal 行为留在 Tauri host」一致）。

#### B. 度量

页签条 36 / 单页签 28·80–200 / 垂直侧栏 180（80–400）/ 内容左 padding 16 / 滚动条 11·thumb≥36 / 分屏各 50% / 背压 500k·100k → **全部见 §1.6**。

#### C. 元素 → gpui-kit 对应

> ⚠️ **§4 缺口清单第 1 条**：**gpui-kit 没有终端模拟器 / PTY / ANSI 解析**。对 `portable_pty|conpty|alacritty|termwiz|crossterm|vte|vt100|ansi_escape|termion|wezterm` 在三个 crate 的全部 `.rs` 与 `Cargo.toml` 里**精确检索 0 命中**。终端内容区**必须自研或引第三方**。下表只覆盖**外壳**。

| Windows 元素 | gpui-kit 0.6.6 实现 | 判定 | 写法要点 / 坑 |
| --- | --- | --- | --- |
| 终端页签行（水平 36px） | `component::tab::{TabBar, Tab}` + `TabVariant::Underline` | **可一比一（尺寸需显式覆盖）** | ✅ 已核对：`TabBar::{new, underline, pill, outline, segmented}`（`tab/tab_bar.rs:59,104,86,92,98`）；`Tab` 四变体高度表在 `tab/tab.rs:24-43`（Medium + Underline = **36**，其余 32）。`.h(px(36.))` 覆盖后 ⚠️ `TabVariant::inner_height`（`tab.rs:45-69`）的固定值可能导致内容被裁（**未实机验证**） |
| 页签活动态底部 3px 强调条 | 无「3px 下划线」参数 | **需自行组合** | `TabVariant::Underline` 是 **2px**；要精确 3px 需自绘 `div` 底部条 |
| 工具栏图标按钮（搜索/新建/下箭头/全屏） | `component::button::Button` + `.icon(..)` + `.ghost()` + `.tooltip(..)` + `.disabled(..)` | **可一比一** | ⚠️ **没有 `IconButton` 类型**：`Button::is_icon_only` 由"只设 icon 不设 label"推断（`button/button.rs:307-310`）。`ButtonIcon` 是 `pub(crate)`（`button/mod.rs:9`） |
| 页签右键菜单 | `menu::ContextMenu` + `ContextMenuExt::context_menu(..)` + `PopupMenu` / `PopupMenuItem` | **可一比一** | — |
| 页签工具栏右键菜单（终端宽度/标签页布局/位置） | `PopupMenu` + `PopupMenuItem::checked(bool)` + `label(..)` + `separator()` | **可一比一** | — |
| 终端配置文件下拉（列表 + 底部「重新检测」按钮） | `PopupMenu::max_h(..).scrollable(true)` + `PopupMenuItem::element(..)` 挂按钮 | **可一比一** | — |
| 页签关闭按钮 | `Tab` **无内建关闭按钮**（`tab/tab.rs` / `tab_bar.rs` 内无 close/closable/on_close） | **需自行组合** | 用 `Tab::suffix(impl IntoElement)` 塞一个 `Button` |
| 页签拖拽重排 | `TabBar`/`Tab` **不支持拖拽** | **需自行组合** | 可拖拽的 tab group 只在 dock：`component::dock::TabGroup` + `TabGroupContext::{drag_panel,drop_panel,drop_item}` + `DragPanel` / `DropIndicator` |
| **垂直页签布局（180px 侧栏）** | `TabBar`/`Tab` **无 orientation/vertical 参数**（源码无 `vertical`/`orientation`） | **没有（需替代方案）** | 考虑 `component::sidebar::Sidebar` + `SidebarMenu` / `SidebarMenuItem` 模拟，或自绘竖列 |
| 空态条（`terminal.noTerminals`） | `component::empty::Empty` + `.title(EmptyTitle)` + `.description(EmptyDescription)` + `EmptyMedia` | **可一比一** | — |
| 页签溢出「更多」菜单 | `TabBar::menu(true)` + `TabBar::max_width(..)` + `.track_scroll(&ScrollHandle)` + `.last_empty_space(..)` | **可一比一** | ⚠️ `track_scroll` **不会自动把选中标签滚进视野** → 自己 `scroll_to_item` |
| 终端搜索浮层（查找/上一个/下一个/大小写/全字/正则/计数） | 无 `SearchInput` 类型；用 `component::input::{Input, InputState}` + `.cleanable(bool)`，或 `ListState::searchable(true)` + `search_placeholder` | **需自行组合** | 选项开关用 `Toggle` / `ToggleGroup` 自组 |
| 终端右键菜单（复制所选内容 / 粘贴） | `ContextMenuExt::context_menu(..)` + `PopupMenuItem::{disabled, on_click}` | **可一比一** | 快捷键提示用 `Kbd::binding_for_action` 或自绘字符串 |
| 终端错误兜底页 | `Empty` + `EmptyTitle`/`EmptyDescription` + `Button`「重试」 | **可一比一** | — |
| **终端内容区（VT / 字符网格 / scrollback / 选择 / 链接）** | **无** | **没有** | 两条路：① 自研 VT 渲染（GPUI `Element` / `canvas` 上画字符网格）；② 内嵌第三方终端窗口。⚠️ **`gpui-pre` 能否嵌入原生窗口/视图未核实** —— 这决定走哪条路。参考实现：`portable-pty` + `vte` / `alacritty_terminal` / `wezterm-term` / `vt100` |

**自研必须覆盖的 xterm.js 能力（按依赖优先级，含证据）**：VT 解析（CSI/SGR/OSC、光标移动、滚动区域、备用屏幕、reflow，`terminal.tsx:267-289`）；真彩 SGR（`connection.rs:208-210`）；Unicode 11 宽字符（`terminal.tsx:358`）；光标样式（`:272-275`）；scrollback + 滚动条（`:279`）；选择（`:745-749,853`）；链接层（`use-terminal-addons.ts:102-165`）；输入编码 `onData`/`onBinary`（`use-terminal-connection.ts:145-146`）；键盘拦截（`terminal.tsx:290-321`）；IME/输入法（`:323-347`）；OSC 标题/目录（`terminal-osc-stream.ts:93-104`）；尺寸同步（`terminal.tsx:632-651`）；背压（`terminal-protocol.ts:13-14`）；ConPTY 兼容标记（`terminal-options.ts:34-48`）；非 0 退出提示（`use-terminal-connection.ts:213-220`）；拖入文件写路径（`terminal.tsx:176-216`）。

#### D. 中文文案（`terminal.*`，zh-CN 区块 `locale.ts:7503-7558`）

| 键 | 中文原文 |
| --- | --- |
| `terminal.searchPlaceholder` | 在终端中查找... |
| `terminal.searchMatchCase` / `searchMatchWholeWord` / `searchUseRegex` | 区分大小写 / 全字匹配 / 使用正则表达式 |
| `terminal.contextPin` / `contextUnpin` | 固定终端 / 取消固定终端 |
| `terminal.contextDuplicate` / `contextClear` / `contextRename` / `contextExport` / `contextClose` | 复制终端 / 清除终端 / 重命名终端 / 导出输出 / 关闭终端 |
| `terminal.contextCloseOthers` / `contextCloseAll` / `contextCloseRight` | 关闭其他终端 / 关闭所有终端 / 关闭右侧终端 |
| `terminal.tabNamePlaceholder` / `tabUnpin` / `tabClose` / `tabAriaPinned` / `tabRenameAria` | 终端名称 / 取消固定终端 / 关闭 {name} / （已固定）/ 重命名 {name} |
| `terminal.toolbarFullWidth` / `toolbarEditorWidth` | 全宽 / 编辑器宽度 |
| `terminal.toolbarHorizontalTabs` / `toolbarVerticalTabs` | 水平标签页 / 垂直标签页 |
| `terminal.toolbarTabsOnLeft` / `toolbarTabsOnRight` | 标签页在左侧 / 标签页在右侧 |
| `terminal.toolbarTerminalWidth` / `toolbarTabLayout` / `toolbarTabPosition` | 终端宽度 / 标签页布局 / 标签页位置 |
| `terminal.newTerminal` / `terminal.search` / `terminal.nextTab` / `terminal.previousTab` | 新建终端 / 搜索 / 下一个标签页 / 上一个标签页 |
| `terminal.exitFullScreen` / `terminal.fullScreen` / `terminal.fullScreenTerminal` | 退出全屏 / 全屏 / 全屏终端 |
| `terminal.findInTerminal` / `terminal.detectShells` / `terminal.detectingShells` | 在终端中查找 / 重新检测已安装的 Shell / 正在检测 Shell… |
| `terminal.detectShellsFailed` | Shell 检测失败，请重试。现有终端不受影响。 |
| `terminal.shellUnavailable` / `terminal.chooseTerminalProfile` / `terminal.noTerminals` / `terminal.terminalTabs` | 暂未检测到 / 选择终端配置文件 / 没有终端 / 终端标签页 |
| `terminal.resizeSidebar` / `terminal.textFiles` / `terminal.allFiles` | 调整终端侧栏大小 / 文本文件 / 所有文件 |
| `terminal.errorTitle` / `terminal.errorFallback` / `terminal.retry` | 终端错误 / 无法初始化终端 / 重试 |
| `terminal.pasteLinesConfirm` | 要将 {count} 行粘贴到终端吗？这可能会执行多条命令。 |
| `terminal.pasteIntoTerminal` / `terminal.copySelection` / `terminal.paste` | 粘贴到终端 / 复制所选内容 / 粘贴 |
| `terminal.openExternalLink` / `terminal.openExternalLinkConfirm` | 打开外部链接 / 要在浏览器中打开此链接吗？\n\n{url} |
| `terminal.terminals` | 终端 |

（`terminal.copySelection` 有单测锁定：`windows/tauri/src/i18n/locale.test.ts:64`。）

---

### ⑥ 运行与调试

#### A. 组成与层级

```
RunPane                                    features/run/components/run-pane.tsx:68
├─ ProjectPreparationStatus                :193
├─ JavaDiscoveryNotice（条件渲染）           :42-66 / :194
├─ 头部工具行（36px）                        :195-259
│  ├─ RunIcon + "运行 {projectName}"          :196-199
│  ├─ Spinner（isLoading 时）                :200
│  ├─ 运行状态文字                           :201-207
│  ├─ 运行/停止按钮（PlayIcon/StopIcon）      :208-212
│  ├─ RunServicesMenu                       :213-220
│  ├─ 重新扫描 / 滚动到底 / 清除输出 / 最小化  :221-258
├─ 阻断/过期诊断横幅（warning 色）            :261-289
├─ JavaLaunchDecisionBanner（条件）           :291-303
└─ RunConfigurationListSplit（status==="ready"）  :322-431

DebuggerView                                features/debugger/components/debugger-view.tsx
├─ 头部工具行（h-10 = 40px）                 :362-440
│  ├─ Bug 图标 + "运行和调试"                 :363-366
│  ├─ 开始 / 继续或暂停 / 停止 按钮组          :367-397
│  ├─ 分隔符 / 单步跳过 / 单步进入 / 单步跳出   :398-428
│  ├─ DebugStatusBadge                       :430
│  └─ 切换当前行断点按钮                       :431-439
└─ grid（左 260~320px 配置栏 + 右内容）        :442
   ├─ aside：配置选择 Select + 命令输入        :443-459
   └─ 右侧：页签（线程和变量 / 控制台）          :575-600
      └─ grid grid-cols-2 gap-2 p-2           :601
         ├─ 调用栈 / 线程 / 变量 / 监视 / 控制台(col-span-2) / 断点  :602-714

DiagnosticsBuffer                           features/diagnostics/components/diagnostics-buffer.tsx:14
└─ DiagnosticsPane isEmbedded={true}        :55-64 → diagnostics-pane.tsx:212
   ├─ paneHeaderClassName()（min-h-7）        :892-898
   ├─ FileNavigatorSidebar（左）              :1034-1046
   └─ ScrollArea contentClassName="px-1.5 py-1.5"  :1049
      └─ space-y-1.5 → section（分组卡片）     :1062-1178
```

输出有**三种不同实现**（不要混为一谈）：运行进程输出 = `RunOutputText`（自研 ANSI SGR → `<span>`，普通 DOM，无 xterm）；调试适配器输出 = 纯文本行列表；终端输出 = xterm.js。

运行进程由 Rust 侧 `std::process::Command` spawn（`run.rs`），输出经 **Tauri 事件 `run-output` / `run-exit`**（`app.emit_to`）回流；**没有 `operationID` 概念**，用 `execution_id` 做归属校验。调试适配器同样在 Rust 侧 spawn，stdin/stdout 走 DAP `Content-Length` 分帧，分帧与状态机在 `lithe-core` 的 `debug.*`。

#### B. 度量

运行头栏 36 / 调试头栏 **40**（不一致）/ 左栏 260–320 / 输出正文 12px 等宽 / 配置行 24–26 / 变量缩进 `18 + 12·depth` / 诊断头 min-h 28 / 问题行 py 6 px 8 / 非嵌入外层 h-44=176 → **全部见 §1.7**。

#### C. 元素 → gpui-kit 对应

| Windows 元素 | gpui-kit 0.6.6 实现 | 判定 | 写法要点 / 坑 |
| --- | --- | --- | --- |
| **工具窗容器** | dock 整块（见 ④） | **可一比一（建议直接采用 dock）** | `DockArea::set_zoomed_in`（`dock/dock_area.rs:626`）对应「全屏」 |
| 运行配置列表（多分组、可折叠、每行图标按钮） | `component::list::ListDelegate` 的 `sections_count` / `items_count` / `render_section_header` / `render_section_footer` + `List`/`ListState` + `ListItem` | **可一比一** | ⚠️ **所有 item 必须同高**（`list/delegate.rs:41`）→ 组头也是 item，需处理"组头折叠状态 + 同高约束" |
| **运行输出（可换行、等宽 12px、SGR 着色、千行级）** | 无现成 console viewer → `base::{VirtualList, v_virtual_list}` + `base::Scrollbar::vertical(&VirtualListScrollHandle)`；需要选择/复制时叠加 `base::text::TextView` | **没有（虚拟列表 + 自研 span 渲染可拼出）** | 等宽取 `cx.theme().mono_font_family`；**ANSI SGR 解析仍需自研** |
| 输出自动滚到底开关 | `VirtualListScrollHandle::scroll_to_bottom()`（`base/virtual_list.rs:123`）+ `base::AutoScroll`；或 `component::message_scroller` | **需自行组合** | `AutoScroll` 在 `gpui-base/src/lib.rs:84` re-export（✅已核对） |
| stdin 输入行 + 发送按钮 | `component::input::{Input, InputState}` + `Button::primary()` | **可一比一** | — |
| 运行/调试头部工具行（Play/Continue/Pause/Stop/Step） | `Button` + `.icon(..)` + `.tooltip(..)` + `.disabled(..)` + `.ghost()`；分组用 `ButtonGroup` | **可一比一** | — |
| 调试状态徽标（空闲/运行中/已暂停/已退出/已停止/失败） | `component::badge::Badge` `.color(..)`；或 `component::tag::Tag` + `TagVariant` | **可一比一** | — |
| 调试面板卡片（圆角 12、边框、可折叠 section、右上角计数徽标） | `div().rounded(cx.theme().radius_lg).border_1().bg(cx.theme().tokens.list)` + `collapsible` / `accordion` + `Badge::new().count(n)` | **可一比一** | — |
| 调试左右分栏（260–320 / 1fr） | `component::{h_resizable, resizable_panel}` + `.size(px(280.))` + `.size_range(px(260.)..px(320.))` | **可一比一** | — |
| 调试页签（32px 高 + 2px 下边框） | `TabBar` + `TabVariant::Underline`（默认 **36px**） | **可一比一（高度需覆盖）** | 要 32px 需 `.h(px(32.))` + 自行处理 `inner_height`（`tab/tab.rs:45-69`） |
| 调试控制台逐行输出（stderr 红 / stdout 灰） | 同"运行输出"，或每行 `v_flex` + `text_color(theme.danger / theme.muted_foreground)` | **需自行组合** | — |
| **问题列表（分组卡片 + 行：严重度图标 11px + 消息 + 位置 chip + 文件路径 + 来源/代码 chip）** | `List` + `ListDelegate` 多 section + `ListItem` + `Badge`/`Tag`；图标用 `component::icon::Icon` + `IconName` | **可一比一** | 严重度色用 `theme.danger/success/warning/info`。⚠️ **Windows 严重度只有三档**（`error \| warning \| info`），LSP 的 `Hint(4)` 与 linter 的 `hint` **都被折叠为 info**（`diagnostics.store.ts:40-42,68`）→ **没有独立的 hint 图标/颜色** |
| 问题筛选下拉菜单 | `PopupMenu` + `PopupMenuItem::{new, label, separator, checked, disabled, on_click}` | **可一比一** | — |
| 问题搜索浮层（Esc 先清空再关闭） | `Input` + `InputState`；Popover 容器 `component::popover::Popover` 或 `base::popup::Popup` | **可一比一** | — |
| 问题/输出面板滚动区 | `component::scroll::ScrollableElement::{overflow_y_scrollbar, overflow_scrollbar}` + `Scrollbar::vertical(..)` | **可一比一** | ⚠️ **`ScrollbarState` 存在但私有**（`base/scrollbar.rs:149`，未 re-export）→ **不要找它**，用 `ScrollableElement` |
| 工具窗全屏 | `DockArea::set_zoomed_in` / `set_zoomed_out` / `is_zoomed` | **可一比一** | — |
| 底部状态栏 | `component::status_bar::StatusBar` `.left(..)` `.right(..)`；token `status_bar` / `status_bar_border` | **可一比一** | — |
| **DAP / PTY / 进程生命周期** | 平台层（Tauri host / 未来的 gpui 平台层） | **不在 UI 范围** | 跨端契约里终端与调试的进程行为**不归 Core** |

#### D. 中文文案

**运行（`run.*`，zh-CN 区块 `locale.ts:5927-6042`）**：`run.title`=运行、`run.run`=运行配置、`run.stop`=停止运行、`run.running`=运行中、`run.succeeded`=成功、`run.failed`=失败、`run.rescan`=重新扫描服务、`run.clearOutput`=清除运行输出、`run.scrollToEnd`=输出始终滚动到最后一行、`run.minimize`=最小化、`run.configurations`=运行配置、`run.resizeConfigurationList`=调整运行配置列表宽度、`run.services`=服务、`run.infrastructure`=Docker 服务、`run.applications`=应用、`run.tasks`=任务、`run.groups`=组、`run.otherConfigurations`=其他运行配置、`run.chooseServices`=选择要运行的服务、`run.runSelectedServices`=运行选中的服务、`run.runAllServices`=运行所有服务、`run.configurationDetails`=配置详情、`run.processOutput`=进程输出、`run.emptyOutput`=运行配置后将在这里显示进程输出。、`run.selectConfiguration`=选择一个运行配置以查看输出。、`run.missingTitle`=未找到项目运行配置、`run.missingMessage`=生成 .lithe/run/generated.json 后即可运行项目。已有的项目和本机覆盖不会被改动。（`locale.ts:5953-5954`）、`run.invalidTitle`=项目运行配置无效、`run.identifyAndGenerate`=识别并生成、`run.identifying`=正在识别项目…、`run.identifyAgain`=重新识别、`run.toolchainNeedsAttention`=项目工具链需要处理、`run.staleConfigurations`=运行配置可能已过期、`run.freshnessCheckFailed`=未能确认运行配置是否为最新、`run.javaBuildFailedTitle`={name} 的 Java 构建未成功完成、`run.javaBuildMarkersMayRemain`=语言服务的构建器在本次会话中发生过失败，因此当前报告的部分错误可能是那次失败留下的。（`locale.ts:5963-5964`）、`run.javaBuildWorkspaceScope`=本次构建未能确认唯一目标项目，因此其他工作区模块的错误可能影响了结果。（`locale.ts:5965-5966`）、`run.javaBuildElapsed`=构建响应耗时：{milliseconds} 毫秒。该数据只用于说明情况，不决定是否允许运行。、`run.javaBuildContinue`=仍然运行、`run.javaBuildAlwaysContinue`=对此工作区始终继续、`run.javaBuildRebuildIndex`=重建 Java 索引、`run.javaBuildOpenLogs`=打开日志、`run.javaBuildAlwaysContinueEnabled`=此工作区遇到 Java 构建失败时会直接继续启动，不再询问。、`run.javaBuildAskAgain`=构建失败时重新询问、`run.editService`=编辑服务、`run.generatedEntries`=已生成 {count} 个可运行项目入口。、`run.runTarget`=运行 '{target}'、`run.modifyRunConfiguration`=修改运行配置…、`run.runMarkerFailed`=无法运行 {target}、`run.javaDiscoveryLoading`=正在等待 Java 语言服务列出可运行的类...、`run.javaDiscoveryStale`=Java 语言服务正在准备项目，暂时显示上次的 Java 入口。、`run.javaDiscoveryFailed`=无法刷新 Java 入口：{message}、`run.editorTitle`=运行配置、`run.saveScope`=保存范围、`run.saveScopeLocal`=此电脑、`run.saveScopeProject`=项目、`run.saveScopeLocalHint`=保存在 .lithe/run/local.json，并已从 Git 中排除。、`run.saveScopeProjectHint`=保存在 .lithe/run/configurations.json，供整个团队使用。本机 JDK 路径不会被共享。（`locale.ts:5987-5988`）、`run.configuration`=配置、`run.type`=类型、`run.effectiveSource`=生效来源、`run.mainClass`=主类、`run.jdkHome`=JDK 主目录、`run.jdkHomeHint`=留空则使用自动检测到的 JDK。、`run.mavenExecutable`=Maven 主目录 / 可执行文件、`run.mavenExecutableHint`=可选择 Maven 主目录；留空则使用项目 Wrapper 或系统 Maven。、`run.nodeExecutable`=Node.js 可执行文件、`run.nodeExecutableHint`=可选择 node.exe；留空则使用自动检测到的 Node.js 运行时。、`run.mavenJdkHome`=Maven JDK 主目录、`run.mavenJdkHomeHint`=留空则与应用使用同一个 JDK。、`run.mavenTests`=Maven 测试、`run.mavenTestsProjectDefault`=使用项目默认值、`run.mavenTestsRun`=运行测试、`run.mavenTestsSkip`=跳过测试、`run.mavenTestsHint`=为当前运行配置覆盖 Maven 工具窗口中的"跳过测试"设置。、`run.toolchainAuto`=自动检测（留空）、`run.toolchainCurrent`=当前路径、`run.runtimeSection`=运行环境（本机）、`run.projectDefaultsSection`=项目默认运行环境、`run.configurationOverridesSection`=当前配置覆盖、`run.toolchainProjectDefault`=使用项目默认值、`run.configurationOverrideHint`=留空则使用项目默认值。、`run.programArguments`=程序参数、`run.vmArguments`=JVM 参数、`run.environment`=环境变量、`run.stdinPlaceholder`=输入程序所需内容…、`run.stdinSend`=发送、`run.workingDirectory`=工作目录、`run.workingDirectoryHint`=留空则使用项目根目录。、`run.command`=命令、`run.done`=完成、`run.cancel`=取消、`run.projectAction`=运行项目操作、`run.deleteAction`=删除运行操作、`run.deleteActionMessage`=要删除运行操作"{name}"吗？、`run.rescanProjectActions`=重新扫描项目操作、`run.scanning`=正在扫描、`run.filterActions`=筛选操作、`run.currentFile`=当前文件、`run.detectedInProject`=项目中检测到、`run.custom`=自定义、`run.scanningProjectActions`=正在扫描项目操作、`run.noMatchingActions`=没有匹配的操作、`run.noRunnableActionsFound`=未找到可运行操作、`run.tryAnotherActionSearch`=请尝试其他名称、命令或来源。、`run.addCustomCommandHint`=添加自定义命令，或打开带有可运行 LSP CodeLens 操作的文件。、`run.newCustomAction`=新建自定义操作、`run.runCell`=运行单元、`run.runChunk`=运行代码块。

**调试（`debugger.*`，zh-CN 区块 `locale.ts:5086-5145`）**：`debugger.runAndDebug`=运行和调试、`debugger.statusIdle`=空闲、`statusRunning`=运行中、`statusPaused`=已暂停、`statusExited`=已退出、`statusStopped`=已停止、`statusFailed`=失败、`toggleCurrentLineBreakpoint`=切换当前行断点、`configuration`=配置、`debugConfiguration`=调试配置、`command`=命令、`commandToRun`=要运行的命令、`noCommandAvailable`=没有可用命令、`start`=启动、`continue`=继续、`pause`=暂停、`continueDebugging`=继续调试、`pauseDebugging`=暂停调试、`stop`=停止、`stepOver`=单步跳过、`stepInto`=单步进入、`stepOut`=单步跳出、`over`=跳过、`into`=进入、`out`=跳出、`paused`=已暂停、`noLaunchJsonFound`=未找到 launch.json、`openProjectToLoadLaunchJson`=打开项目以加载 launch.json、`stack`=调用栈、`threads`=线程、`threadsAndVariables`=线程和变量、`variables`=变量、`watch`=监视、`console`=控制台、`breakpoints`=断点、`showRunAndDebug`=显示运行和调试、`hideRunAndDebug`=隐藏运行和调试、`toggleRunAndDebug`=切换运行和调试、`clearConsole`=清空控制台、`clearBreakpoints`=清空断点、`clearWatchExpressions`=清空监视表达式、`clearDebugConsole`=清空调试控制台、`adapterOutputAppearsHere`=适配器输出会显示在这里。、`stackEmpty`=启动会话后可查看栈帧。、`threadsEmpty`=暂停进程后可查看线程。、`variablesEmpty`=暂停在栈帧上以查看变量值。、`breakpointsEmpty`=点击行号槽或切换当前行断点。、`lineNumber`=第 {line} 行、`disableBreakpoint`=禁用断点、`enableBreakpoint`=启用断点、`removeBreakpoint`=移除断点、`loadingVariable`=正在加载变量、`empty`=空、`addExpression`=添加表达式、`addWatch`=添加监视、`refreshWatches`=刷新监视、`watchEmpty`=添加表达式，以便暂停时查看。、`removeWatch`=移除监视、`evaluating`=正在求值...、`notEvaluated`=未求值。

**问题（`diagnostics.*`，zh-CN 区块 `locale.ts:7342-7383`）**：`diagnostics.problemCount` / `problemCountOne`={count} 个问题、`empty`=未检测到问题、`noMatch`=没有符合当前筛选条件的问题、`resetFilters`=重置筛选、`search`=搜索问题、`filter`=筛选问题、`groupByFile`=分组：文件、`groupBySeverity`=分组：严重程度、`groupByNone`=不分组、`sortBySeverity`=排序：严重程度、`sortByFile`=排序：文件、`sortByPosition`=排序：位置、`severityCount`={label} ({visible}/{total})、`errors`=错误、`warnings`=警告、`info`=信息、`onlyCurrentFile`=仅当前文件、`clearSourceFilter`=清除来源筛选（{source}）、`clearSourceFilterSimple`=清除来源筛选、`resetAllFilters`=重置全部筛选、`hideFiles`=隐藏文件、`showFiles`=显示文件、`closePane`=关闭问题面板、`filesAria`=诊断文件、`all`=全部诊断、`fullWidth`=全宽、`editorWidth`=编辑器宽度、`fullScreen`=全屏、`exitFullScreen`=退出全屏、`goToProblem`=转到问题、`copyMessage`=复制消息、`copyLocation`=复制位置、`copyDetails`=复制完整详情、`filterBySource`=按来源筛选：{source}、`enableWrap`=启用消息换行、`disableWrap`=禁用消息换行、`loadingFixes`=正在加载快速修复...、`noFixes`=没有可用的快速修复、`messageCopied`=已复制诊断消息、`locationCopied`=已复制诊断位置、`detailsCopied`=已复制诊断详情。

> **`problems.*` 与 `output.*` 前缀的 key 不存在**（在 zh-CN 区块全量检索零命中）。问题面板统一用 `diagnostics.*`，运行输出用 `run.*`。

**工具窗 / 页签 / 准备工作区**：`workbench.terminal`=终端（5915）、`workbench.diagnostics`=诊断（5916）、`workbench.run`=运行（5913）、`workbench.maven`=Maven（5914）、`workbench.gitLog`=提交记录（5910）、`workbench.activityViews`=活动视图（7890）、`layout.resizeBottomPane`=调整底部面板大小（7888）、`preparation.*`（5917-5926）、`runActions.editRunAction`=编辑运行操作（6107）、`runActions.newRunAction`=新建运行操作（6108）、`runActions.saveChanges`=保存更改（6109）、`runActions.addAction`=添加操作（6110）、`runActions.savedForPrefix`=此操作将保存到（6111）、`runActions.savedForSuffix`=并在新终端中运行。（6112）、`runActions.name`=名称（6113）、`runActions.namePlaceholder`=启动开发服务器（6114）、`runActions.workingDirectoryDescription`=留空则使用项目根目录，或输入相对路径，例如（6115-6116）、`runActions.run` / `test` / `check` / `build` / `projectScript`=运行 / 测试 / 检查 / 构建 / 项目脚本（6117-6121）、`runActions.editNamedAction`=编辑 {name}（6122）、`runActions.deleteNamedAction`=删除 {name}（6123）、`ui.open` / `ui.cancel` / `ui.save` / `ui.delete`（4616 / 4609 / 4617 / 4618）。

---

### ⑦ 搜索

#### A. 组成与层级

**唯一可达形态：主侧栏视图 `search`**（**不是浮层、不是 pane buffer**）。`isGlobalSearchVisible` / `openGlobalSearchBuffer` **均无调用方**，浮层与 pane buffer 两种形态都是死代码。

```
SidebarPaneSelector（活动栏放大镜，快捷键提示 Mod+Shift+F）  sidebar-pane-selector.tsx:134-150
MainSidebar → pane id "search" → <GlobalSearchBuffer compact />   main-sidebar.tsx:797-804
活动栏右键菜单「搜索」                             main-sidebar.tsx:717-720
命令 workbench.showGlobalSearch（cmd+shift+f）/ workbench.showProjectSearch（cmd+shift+shift+h）  command-registry.ts:644-663
  └─ openGlobalSearchSidebar()                    layout/actions/workbench-tool-window-actions.ts:37-49

GlobalSearchBuffer（compact 时隐藏左导航栏）       global-search-buffer.tsx:54,518-588
├─ GlobalSearchToolbar                            global-search-toolbar.tsx:46-208
│   ├─ SearchReplaceToggle（展开/收起详情）        :95-100
│   ├─ 搜索框：h-7 + MagnifyingGlass + CommandInput + 清除  :101-129
│   ├─ ToggleGroup（区分大小写 / 全字匹配 / 正则，iconOnly segmented）  :130-146
│   └─ Badge（searchWarning 优先，否则 resultLabel）  :147-165
│   └─ 详情区：SearchReplaceRow + 包含/排除文件两列  :167-206
├─ GlobalSearchResults                            global-search-results.tsx:35-107
│   ├─ FileNavigatorSidebar（默认宽 224，可拖 176–420）  file-navigator-sidebar.tsx:269
│   └─ ScrollArea（both 方向）→ SearchExcerptResults  search-excerpt-results.tsx:166-223
└─ GlobalSearchState（空/忙/错误/不可用）           global-search-state.tsx:39-117
```

数据流：`useContentSearch` → `searchFilesContent` → lithe-core `workspace.search`；索引状态 `fffScanStatus` → `workspace.snapshot`；WSL 工作区改走 `searchProviderFilesContent`。

#### B. 度量

工具条 8/8·12 / 搜索框 28·圆角 8 / 导航栏 224（176–420，≤父宽 50%）/ 摘录 min-h 104 / 上下文 2→7 行 / 分页 140·初始 40·增量 40·防抖 200ms → **全部见 §1.8**。

#### C. 元素 → gpui-kit 对应

| Windows 元素 | gpui-kit 0.6.6 实现 | 判定 | 写法要点 / 坑 |
| --- | --- | --- | --- |
| 搜索面板（侧栏形态） | `component::sidebar::{Sidebar, SidebarHeader}` + 自绘主体；或作为 dock 面板 | **可一比一** | — |
| 搜索输入框（28 / 圆角 / 内嵌放大镜 + 清除） | `component::input::{Input, InputState}` + `.prefix(..)` / `.suffix(..)` / `.cleanable(bool)` | **可一比一** | ⚠️ **`SearchInput` 类型不存在** |
| 大小写/全字/正则三连开关（segmented iconOnly） | `component::button::{Toggle, Button}` + `.toggled(bool)`；或 `tab::{TabBar, TabVariant}` | **需自行组合** | **无独立 `Segmented` 类型** |
| 结果计数 / 警告 Badge | `component::badge::Badge`（或 `tag::Tag`） | **可一比一** | — |
| 替换行（输入框 + 替换 + 全部） | `Input` + `Button` + `Tooltip::new(..)` | **可一比一** | — |
| 包含/排除文件两列输入 | `Input` × 2 + 自绘 grid | **可一比一** | — |
| 左侧命中文件导航（224，可拖 176–420，树/平铺切换） | `base::resizable::{h_resizable, resizable_panel, ResizableState}` + `component::tree::Tree` 或 `list::{List, ListState}` + `input::Input` | **需自行组合** | `ResizablePanel::size_range` 可表达 176–420；树/平铺切换要自己换 renderer |
| **结果摘录列表（虚拟化多行代码卡片 + 语法高亮 + 命中高亮）** | `base::{VirtualList, v_virtual_list}` + `component::highlighter::*` + 自绘 span 背景 | **需自行组合** | ⚠️ **`uniform_list` 行高取第 0 行，摘录卡片高度不一 → 必须用 `VirtualList`**。`v_virtual_list` 签名需自备 `Rc<Vec<Size<Pixels>>>` 行高 |
| 摘录行号 gutter（右对齐、等宽数字、按最大行号算宽） | 自绘 + `component::text::TextView`；等宽字体用 `cx.theme().mono_font_family` | **需自行组合** | — |
| 上下文展开/收起（按文件读全文，2→7 行） | `Button` + 自己异步读文件（`cx.spawn`） | **需自行组合** | — |
| 懒加载哨兵（IntersectionObserver 640px / 240px） | `VirtualList` 的可见区间回调或 `ListDelegate::load_more_threshold` | **需自行组合** | **语义等价物存在，机制不同**（gpui-kit 是委托回调，不是观察者） |
| 点击摘录跳转到编辑器行列 | `component::input::Editor` 的 `set_cursor_position` 或自定义 action + `KeyBinding` | **需自行组合** | — |
| 横向滚动（ScrollArea `orientation="both"`） | `Styled::overflow_x_scroll()` / `overflow_scroll()` | **需自行组合** | ⚠️ **0.6.6 没有 `.overflow_y_scrollbar()`**（见 `UI-MAP.md` §1.1.1）；横向滚动条要自己挂 |
| 搜索面板偏好持久化（`localStorage`） | `component::global_state::GlobalState` 或自建 JSON | **需自行组合** | — |

#### D. 中文文案

**全局搜索（`search.*`，zh-CN 区块 `locale.ts:5879-5908`）**

| 键 | 中文原文 | `locale.ts:行号` |
| --- | --- | --- |
| `search.emptyTitle` | 在项目中搜索 | 5879 |
| `search.emptyDescription` | 输入关键词，即可在整个项目中查找匹配的文件和代码行。 | 5880 |
| `search.openProjectTitle` | 打开项目后再搜索 | 5881 |
| `search.openProjectDescription` | 全局搜索需要先打开一个项目文件夹。 | 5882 |
| `search.unsupported` | 当前工作区类型不支持全局搜索。 | 5883 |
| `search.failed` | 搜索失败 | 5884 |
| `search.retry` | 重试 | 5885 |
| `search.noResults` | 未找到结果 | 5886 |
| `search.noResultsFor` | 未找到 "{query}" 的结果 | 5887 |
| `search.noResultsForWithFilters` | 未找到 "{query}" 的结果（已应用当前文件筛选） | 5888 |
| `search.matchCase` / `matchWholeWord` / `useRegex` | 区分大小写 / 全字匹配 / 使用正则表达式 | 5889 / 5890 / 5891 |
| `search.search` | 搜索 | 5892 |
| `search.hideDetails` / `showDetails` | 隐藏详细信息 / 显示详细信息 | 5893 / 5894 |
| `search.clear` | 清除搜索 | 5895 |
| `search.close` | 关闭搜索 | 5896 |
| `search.previousMatch` / `nextMatch` | 上一个匹配项 / 下一个匹配项 | 5897 / 5898 |
| `search.hideReplace` / `showReplace` | 隐藏替换 / 显示替换 | 5899 / 5900 |
| `search.replaceWith` | 替换为... | 5901 |
| `search.replace` | 替换 | 5902 |
| `search.replaceAllShort` | 全部 | 5903 |
| `search.replacedMatches` / `replaceMatchesFailed` | 已替换 {count} 处匹配 / 替换搜索匹配失败 | 5904 / 5905 |
| `search.options` | 搜索选项 | 5906 |
| `search.filesToInclude` / `filesToExclude` | 要包含的文件 / 要排除的文件 | 5907 / 5908 |
| `search.searchInFiles` | 在文件中搜索 | 4608（`workbench.searchInFiles`） |

**文件导航器 / 快速打开 / 命令面板**：`fileNavigator.files`=文件（7485）、`fileNavigator.viewAria`=文件导航器视图（7486）、`fileNavigator.flatList`=平铺列表（7487）、`fileNavigator.fileTree`=文件树（7488）、`fileNavigator.showingCount`=显示 {visible} / {total}（7489）、`fileNavigator.noFilesMatch`=没有匹配的文件（7490）、`fileNavigator.resizeAria`=调整文件导航器大小（7491）、`fileNavigator.searchResultFiles`=搜索结果文件（7492）、`panes.searchResults`=搜索结果（4684）、`workbench.search`=搜索（4607）、`workbench.searchInFiles`=在文件中搜索...（4608）、`quickOpen.searchFiles`=输入以搜索文件...（7823）、`quickOpen.searchSymbols`=输入以筛选符号...（7824）、`quickOpen.searchWorkspaceSymbols`=在项目中搜索符号...（7825）、`quickOpen.filesCount`= {count} 个文件（7826）、`quickOpen.fileCountOne`={count} 个文件（7827）、`quickOpen.symbolsCount`={count} 个符号（7828）、`quickOpen.openFolder`=打开文件夹后再搜索文件（7829）、`quickOpen.noMatch`=未找到匹配的文件（7830）、`quickOpen.searching`=正在搜索...（7831）、`quickOpen.noFilesInProject`=项目中未找到文件（7832）、`quickOpen.noFiles`=没有可用文件（7833）、`quickOpen.indexing`=正在索引项目文件（7834）、`quickOpen.loadingFiles`=正在加载文件（7835）、`quickOpen.loadingSymbols`=正在加载符号...（7836）、`quickOpen.noSymbols`=未找到符号（7837）、`menu.quickOpen`=快速打开（7791）、`keybindings.commands.file.quickOpen.title`=快速打开（8285）、`commandPalette.actions.quick-open.label`=转到：快速打开（8026）、`commandPalette.actions.search-global.label`=搜索：全局搜索（8024）。

**未走 i18n 的搜索可见文案（硬编码英文，file:line）**：`Indexing N files` / `Indexing files` / `Preparing search` / `Searching a/b files` / `Searching N files` / `Searching files` / `Loading more results`（`global-search-buffer.tsx:453-474`）；`N results (M total)`（`:490`）；`Invalid regular expression; showing literal matches`（`stores/global-search.store.ts:58`）；`Load all search results before replacing all`（`:540`）；`Showing N of M results`（`global-search-results.tsx:102`）；`match`/`matches`（`search-excerpt-results.tsx:120`）；`Collapse context`/`Expand context`（`:130-131`）；`Open line N`（`search-excerpt-code.tsx:119`）；`Failed to expand search context`（`global-search-buffer.tsx:306`）；`Search failed: ...`（`hooks/use-content-search.ts:303,351`）；`Search indexing failed: ...`（`:401`）。

---

### ⑧ AI 面板

#### A. 组成与层级

```
App → WorkbenchApp
└─ PaneContainer → case "agent"                pane-container.tsx:948-949
    └─ AgentTab（lazy）                         pane-container.tsx:55-59
        └─ AIChat                              features/ai/components/chat/ai-chat.tsx:53-1157

AIChat（根）
├─ ChatHeader                                 chat-header.tsx:96
│   ├─ PaneChip + ProviderIcon / EditableChatTitle  :150-152,27-79
│   ├─ 按钮：消息搜索 / Agent 历史 / 新建 Agent       :165-201
│   ├─ 消息搜索条（Input h-7 + 计数 + 上/下/关闭）    :205-271
│   └─ ChatHistoryDropdown                    history/chat-history-dropdown.tsx:34
├─ 初始态分支（无消息且无 ACP 事件）             ai-chat.tsx:987-988,1044-1083
│   └─ AgentShortcuts + AIChatInputBar presentation="initial"
├─ 消息态                                      ai-chat.tsx:1085-1107
│   └─ MessageScrollerProvider autoScroll / MessageScroller / Viewport / Button
│       └─ ChatMessages                       chat/chat-messages.tsx:37-185
│           └─ ChatMessage                    chat/chat-message.tsx:76-317
├─ AcpPermissionPrompt（permissionQueue 非空时） ai-chat.tsx:1111-1117
└─ AIChatInputBar（消息态）                     chat-input-bar.tsx:66-1314
    └─ ChatComposer → ChatComposerBody → ChatComposerEditable + ChatComposerToolbar
```

`AgentTab` 只做两件事：标题回写 buffer.name、把内容限制在 `mx-auto size-full max-w-4xl`（896px）。

⚠️ **`ModelSelector` / `ProviderSelector` 不在 composer 内**，只被设置页使用；composer 内的模型/提供商选择由 `ChatPreferencesMenu` 的 `DropdownMenuRadioGroup` 承担。

⚠️ **`AgentLauncher` 组件无挂载点**（`agent-launcher.tsx:124` 全仓库仅此一处），其键位 `cmd+shift+space` 落地后没有任何组件消费该状态。

#### B. 度量

内容最大宽 896 / header min-h 28 / 气泡最大宽 80% / composer 可编辑区 max-h 140·min-h 64 / 初始态 192·112 / 发送按钮 28 / 面板 maxHeight 320（提及 240 / 斜杠 240）→ **全部见 §1.8**。

#### C. 元素 → gpui-kit 对应

| Windows 元素 | gpui-kit 0.6.6 实现 | 判定 | 写法要点 / 坑 |
| --- | --- | --- | --- |
| 面板容器（`max-w-4xl` 居中） | 自绘 `div().max_w(px(896.)).mx_auto()` | **可一比一** | — |
| header（标题可重命名 + 动作 + 可展开搜索行） | `gpui_component` 无 pane-header 专用类型 | **需自行组合** | 用 `sidebar::SidebarHeader` 结构或自绘 |
| 会话标题就地重命名 | `component::input::{Input, InputState}` + `InputEvent::PressEnter` | **可一比一** | — |
| **消息滚动 + 粘底自动跟随** | `base::VirtualList` + `ScrollStrategy` 或 `UniformListScrollHandle::{scroll_to_bottom, is_scrolled_to_end}`；`component::message_scroller` | **需自行组合** | ⚠️ Windows 用的 `autoScroll` / `defaultScrollPosition="last-anchor"` / `useMessageScroller` 来自**外部包** `@shadcn/react/message-scroller`（`ui/message-scroller.tsx:1-6`），**该算法本仓库没有** → 必须自研（判定阈值、粘底恢复、`scrollAnchor`）。gpui-kit 的 `MessageScroller` 提供 `is_following_tail` / `jump_button` / `with_bottom_fade`，**比手搓好** |
| 消息气泡（用户右对齐 secondary / 助手 ghost 无背景） | `component::bubble::{Bubble, BubbleContent, BubbleVariant}` + `component::message::{Message, MessageContent, MessageAlignment}` | **可一比一** | 对齐/背景需自调 |
| 消息 hover 才显形的 copy/edit 动作 | `Button` + `.on_hover` / 自管 opacity；或 `menu::DropdownMenu` | **需自行组合** | — |
| Markdown 渲染（标题/表格/列表/引用/代码块/错误块） | `component::text::{TextView, markdown, html}` | **可一比一** | ⚠️ **错误块 `[ERROR_BLOCK]` 是 Lithe 私有协议** → 需自己在 markdown 前后处理 |
| 代码块 + 复制按钮 | `text::markdown` 的代码渲染 + `button` + `clipboard::Clipboard` | **可一比一** | — |
| 工具调用折叠行（activity line） | `collapsible::Collapsible` 或 `accordion::Accordion`；图标旋转自绘 | **需自行组合** | — |
| 计划卡 + 步骤状态 | `group_box::GroupBox` + `stepper::Stepper`；或自绘 | **需自行组合** | ⚠️ 状态语义不完全一致（Windows 的 `getStepStatus` 实际只返回 `current`（首个）或 `pending`，`plan-step-display.tsx:15-31`） |
| 权限确认条（FIFO 队列 + 允许/始终/拒绝/永不） | `alert::{Alert, AlertVariant}` + `Button` 组；或 `DialogButtonProps` | **可一比一** | — |
| 回到底部按钮 | `Button`（圆角/浮动自绘） | **可一比一** | — |
| **composer（多行可增长输入 + 内联 token）** | `component::input::{Textarea, TextareaState}` + `.auto_grow(min,max)` + `.soft_wrap(bool)`；**行内 token（@mention / /command 胶囊）kit 不支持** | **需自行组合** | token 胶囊要么在 `Editor`（`component::input::{Editor, EditorState}`）里做 decorations，要么自绘富文本层。⚠️ **必须显式给行数**：`.auto_grow(2, N)` 或不给高度会塌成 2 行 |
| 发送 / 停止按钮 + 队列 Badge | `Button` `.loading(bool)` + `Badge` | **可一比一** | — |
| `Enter` 发送 / `Shift+Enter` 换行 | `gpui::KeyBinding::new` + `InputEvent::PressEnter { secondary, shift }` | **可一比一** | — |
| `@` 提及 / `/` 斜杠下拉（锚定在输入框上方、宽随锚点） | `popover::Popover` + `list::{List, ListState}` 或 `searchable_list::{SearchableListState, SearchableVec}` | **需自行组合** | ⚠️ `w-(--anchor-width)` 这种宽度绑定要靠 `Popover` + 自己量锚点宽 |
| 模型 / 提供商 / 上下文选择 | `menu::{DropdownMenu, PopupMenu}` 或 `select::{Select, SelectState}` | **可一比一** | — |
| 聊天历史下拉（搜索 + 置顶/归档/删除） | `PopupMenu` + `input::{Input, InputState}`；或 `list` | **需自行组合** | — |
| 上下文 chips（←→ 移焦点、Del 移除） | 自绘 `div` + `tag::Tag` / `Badge` + `KeyBinding` | **需自行组合** | — |
| 语音输入 / 附件缩略图 | **无对应** | **没有** | 自绘或引第三方 |
| 消息搜索（`Cmd+F` + 上/下/关闭） | `Input` + `Button` + `KeyBinding`；命中高亮自绘 | **需自行组合** | — |
| 技能市场面板（列表 + 编辑器，430×560） | `list` / `searchable_list` + `input::{Editor, EditorState}` + `WindowExt::open_dialog` | **需自行组合** | — |
| 空态「4 个技能快捷键」 | `button::Button`（`.label/.icon`） | **可一比一** | — |
| 流式 shader 文字动效（`ui-text-shimmer`） | `component::shimmer::{ShimmerText, ShimmerStyle}` | **可一比一** | `gpui-kit` 有现成实现 |
| 思维球（`ThinkingOrb`） | **无对应** | **没有** | 自绘或复用 `spinner::Spinner` |

#### D. 中文文案

**AI（`ai.*` / `aiShortcut.*` / `aiHistory.*`，zh-CN 区块 `locale.ts:4807-5561`）**

| 键 | 中文原文 | `locale.ts:行号` |
| --- | --- | --- |
| `ai.suggestions` | 建议 | 4807 |
| `ai.agentInstalled` | {name} 已安装 | 4808 |
| `ai.agentInstallFailed` | 安装 {name} 失败 | 4809 |
| `ai.unknownError` | 未知错误 | 4810 |
| `ai.planSummary` | 计划（{count} 个{label}） | 4811 |
| `ai.step` / `ai.steps` | 步骤 / 步骤 | 4812 / 4813 |
| `ai.executePlan` | 执行计划 | 4814 |
| `aiShortcut.planImplementation` | 规划实现方案 | 4815 |
| `aiShortcut.planImplementationContent` | 查看相关代码，并为此任务提出聚焦的实现计划： | 4816 |
| `aiShortcut.findFixBug` | 查找并修复 Bug | 4817 |
| `aiShortcut.findFixBugContent` | 调查此 Bug，找出根因，实施修复并验证： | 4818 |
| `aiShortcut.writeTests` | 为变更编写测试 | 4819 |
| `aiShortcut.writeTestsContent` | 查看相关行为，并为此变更添加聚焦测试： | 4820 |
| `aiShortcut.reviewChanges` | 审查当前更改 | 4821 |
| `aiShortcut.reviewChangesContent` | 审查当前工作区更改，查找 Bug、回归和缺失测试： | 4822 |
| `aiHistory.searchPlaceholder` | 搜索 Agent 历史... | 5380 |
| `aiHistory.empty` | 暂无 Agent 历史 | 5381 |
| `aiHistory.noMatches` | 没有匹配"{query}"的会话 | 5382 |
| `aiHistory.archived` | 已归档 | 5383 |
| `aiHistory.restoreSession` / `restoreSessionNamed` | 恢复会话 / 恢复 {title} | 5384 / 5385 |
| `aiHistory.deleteSession` / `deleteSessionNamed` | 删除会话 / 删除 {title} | 5386 / 5387 |
| `ai.newSession` | 新会话 | 5388 |
| `ai.renameSession` | 重命名 {title} | 5389 |
| `ai.clickToRenameSession` | 点击重命名会话 | 5390 |
| `ai.searchMessages` | 搜索消息 | 5391 |
| `ai.agentHistory` | Agent 历史 | 5392 |
| `ai.agents` | Agents | 5393 |
| `ai.toggleAgentHistory` | 切换 Agent 历史 | 5394 |
| `ai.newAgent` | 新建 Agent | 5395 |
| `ai.previousMatch` / `previousSearchMatch` | 上一个匹配项 / 上一个搜索匹配项 | 5396 / 5397 |
| `ai.nextMatch` / `nextSearchMatch` | 下一个匹配项 / 下一个搜索匹配项 | 5398 / 5399 |
| `ai.closeSearch` / `closeMessageSearch` | 关闭搜索 / 关闭消息搜索 | 5400 / 5401 |
| `ai.executePlanStepPrompt` | 执行计划的第 {number} 步：{title}\n\n{description} | 5402 |
| `ai.editPrompt` | 编辑提示词 | 5403 |
| `ai.cancel` / `ai.send` | 取消 / 发送 | 5404 / 5405 |
| `ai.copyPrompt` | 复制提示词 | 5406 |
| `ai.thinking` | 正在思考... | 5407 |
| `ai.generatedContentNumber` | AI 生成内容 {number} | 5408 |
| `ai.generatedImageNumber` | 生成的图片 {number} | 5409 |
| `ai.openResource` | 打开 {name} | 5410 |
| `ai.copyResponse` | 复制回复 | 5411 |
| `ai.unpinSession` / `pinSession` | 取消置顶会话 / 置顶会话 | 5412 / 5413 |
| `ai.archiveSession` | 归档会话 | 5414 |
| `ai.agent` / `agentDefault` | Agent / Agent 默认值 | 5415 / 5416 |
| `ai.model` / `ai.project` / `ai.branch` | 模型 / 项目 / 分支 | 5417 / 5418 / 5419 |
| `ai.enterApiKey` | 输入 API key... | 5420 |
| `ai.pleaseEnterApiKey` | 请输入 API key。 | 5421 |
| `ai.invalidApiKey` | API key 无效。 | 5422 |
| `ai.failedValidateApiKey` / `failedRemoveApiKey` | 验证 API key 失败。/ 移除 API key 失败。 | 5423 / 5424 |
| `ai.searchApiKeyProviders` | 搜索 API key 提供商... | 5425 |
| `ai.noProvidersFound` | 未找到提供商 | 5426 |
| `ai.apiKeySaved` | API key 已保存 | 5427 |
| `ai.apiKeyRequired` | 需要 API key | 5428 |
| `ai.apiKeySavedWithPeriod` | API key 已保存。 | 5429 |
| `ai.openDashboard` | 打开控制台 | 5430 |
| `ai.remove` / `ai.validating` / `ai.saveKey` | 移除 / 正在验证 / 保存 key | 5431 / 5432 / 5433 |
| `ai.selectProvider` | 选择提供商 | 5434 |
| `ai.copyCode` / `ai.applyCodeToCurrentBuffer` / `ai.apply` | 复制代码 / 将此代码应用到当前缓冲区 / 应用 | 5435 / 5436 / 5437 |
| `ai.error` | 错误 | 5438 |
| `ai.agentSessionRestarted` / `couldNotRestartAgentSession` | Agent 会话已重启 / 无法重启 Agent 会话 | 5439 / 5440 |
| `ai.agentSetup` / `couldNotOpenAgentTerminal` | Agent 设置 / 无法打开 Agent 终端 | 5441 / 5442 |
| `ai.hideDetails` / `ai.details` | 隐藏详情 / 详情 | 5443 / 5444 |
| `ai.restarting` / `restartAgentSession` | 正在重启... / 重启 Agent 会话 | 5445 / 5446 |
| `ai.opening` / `openAgentTerminal` | 正在打开... / 打开 Agent 终端 | 5447 / 5448 |
| `ai.finishAgentSetupThenRestart` | 完成 Agent 设置后重启会话。 | 5449 |
| `ai.completeLoginThenRestart` | 在 Agent CLI 中完成登录后重启会话。 | 5450 |
| `ai.addContext` / `ai.terminal` | 添加上下文 / 终端 | 5451 / 5452 |
| `ai.databaseContext` | {type} 数据库 | 5453 |
| `ai.pullRequestNumber` / `actionRunNumber` | 拉取请求 #{number} / Actions 运行 #{number} | 5454 / 5455 |
| `ai.searchContext` / `noMatchingContextFound` | 搜索上下文... / 未找到匹配的上下文 | 5456 / 5457 |
| `ai.openTabs` / `ai.added` | 打开的标签页 / 已添加 | 5458 / 5459 |
| `ai.notSynced` / `syncing` / `syncPaused` / `synced` | 未同步 / 正在同步 / 同步已暂停 / 已同步 | 5460-5463 |
| `ai.searchAvailableSkills` / `searchSkills` | 搜索可用技能... / 搜索技能... | 5464 / 5465 |
| `ai.new` / `mySkills` / `browse` | 新建 / 我的技能 / 浏览 | 5466 / 5467 / 5468 |
| `ai.loadingAvailableSkills` | 正在加载可用技能... | 5469 |
| `ai.noPublishedSkillsYet` | 暂无已发布技能 | 5470 |
| `ai.publishedSkillsWillAppear` | Lithe 技能注册表可用后，已发布技能会显示在这里。 | 5471 |
| `ai.noAvailableSkillsMatch` | 没有可用技能匹配"{query}" | 5472 |
| `ai.add` / `noSkillsYet` / `noSkillsMatch` | 添加 / 暂无技能 / 没有技能匹配"{query}" | 5473 / 5474 / 5475 |
| `ai.marketplace` / `localOverride` | 市场 / 本地覆盖 | 5476 / 5477 |
| `ai.editSkill` / `editNamedSkill` | 编辑技能 / 编辑 {title} | 5478 / 5479 |
| `ai.deleteSkill` / `deleteNamedSkill` | 删除技能 / 删除 {title} | 5480 / 5481 |
| `ai.newSkill` / `marketplaceSkill` / `marketplaceSkillWithLocalOverride` | 新建技能 / 市场技能 / 带本地覆盖的市场技能 | 5482 / 5483 / 5484 |
| `ai.title` / `skillTitlePlaceholder` | 标题 / 代码审查清单 | 5485 / 5486 |
| `ai.markdown` / `skillContentPlaceholder` | Markdown / 编写此技能的指令或可复用上下文... | 5487 / 5488 |
| `ai.save` / `ai.skills` | 保存 / 技能 | 5489 / 5490 |
| `ai.open` / `recent` / `files` | 打开 / 最近 / 文件 | 5491 / 5492 / 5493 |
| `ai.searchFiles` / `fileList` / `openLower` | 搜索文件... / 文件列表 / 打开 | 5494 / 5495 / 5496 |
| `ai.installingAgent` / `install` | 正在安装 {name} / 安装 | 5497 / 5498 |
| `ai.mode` / `provider` / `configureCustomModel` | 模式 / 提供商 / 配置自定义模型... | 5499 / 5500 / 5501 |
| `ai.apiKeys` / `aiPreferences` / `settings` | API Keys / AI 偏好设置 / 设置 | 5502 / 5503 / 5504 |
| `ai.ask` / `ai.plan` | 提问 / 计划 | 5505 / 5506 |
| `ai.microphoneAccessFailed` | 麦克风访问失败。请检查系统设置 -> 隐私与安全 -> 麦克风。 | 5511 |
| `ai.voiceNotSupportedWebview` / `voiceStoppedUnexpectedly` / `voiceCouldNotStart` | 此 webview 不支持语音输入。/ 语音输入意外停止。/ 无法启动语音输入。 | 5513 / 5514 / 5515 |
| `ai.noOllamaModelsDetected` / `noModelsFound` / `failedFetchModels` / `loadingModels` / `selectModel` | 未检测到模型。请在 Ollama 中安装模型。/ 未找到模型。/ 获取模型失败 / 正在加载模型... / 选择模型 | 5516-5520 |
| `ai.openDiff` / `openFile` / `openTerminal` | 打开 diff / 打开文件 / 打开终端 | 5521 / 5522 / 5523 |
| `ai.toolCall` / `toolCalls` | 工具调用 / {count} 个工具调用 | 5524 / 5525 |
| `ai.toolStatusFailed` / `Pending` / `Running` / `Completed` | 失败 / 等待中 / 运行中 / 已完成 | 5526-5529 |
| `ai.toolChangedFile` / `toolChangedFiles` | 已更改 {file} / 已更改 {count} 个文件 | 5530 / 5531 |
| `ai.toolTerminalOutput` / `toolTerminals` | 终端输出 / {count} 个终端 | 5532 / 5533 |
| `ai.toolDetailWithSummary` | {status} - {summary} | 5534 |
| `ai.allow` / `always` / `deny` / `never` | 允许 / 始终 / 拒绝 / 永不 | 5535-5538 |
| `ai.allowOnce` / `alwaysAllowRequestType` / `denyOnce` / `alwaysDenyRequestType` | 允许一次 / 始终允许此类请求 / 拒绝一次 / 始终拒绝此类请求 | 5539-5542 |
| `ai.permission` / `messageInput` | 权限 / 消息输入 | 5543 / 5544 |
| `ai.showSlashCommands` | 显示斜杠命令 | 5545 |
| `ai.stopVoiceInput` / `startVoiceInput` | 停止语音输入 / 开始语音输入 | 5548 / 5549 |
| `ai.stopGeneration` / `addToQueue` / `sendMessage` | 停止生成 / 加入队列 / 发送消息 | 5550 / 5551 / 5552 |
| `ai.selectedContext` | 已选上下文 | 5553 |
| `ai.removeContextItemHint` | {name}。按 Delete 可从上下文中移除。 | 5554 |
| `ai.selectAiModel` / `useCustomValue` / `typeModelName` | 选择 AI 模型 / 使用 {value} / 输入模型名称并按 Enter | 5555 / 5556 / 5557 |
| `ai.selectAiProvider` | 选择 AI 提供商 | 5558 |
| `ai.slashCommandSuggestions` / `noMatchingSlashCommands` / `noSlashCommandsAvailable` | 斜杠命令建议 / 没有匹配的斜杠命令 / 暂无可用斜杠命令 | 5559 / 5560 / 5561 |
| `layout.resizeAiChat` | 调整 AI 聊天大小 | 7886 |
| `commandPalette.actions.toggle-ai-chat-view.enableLabel` / `disableLabel` | 视图：显示 AI 聊天 / 视图：隐藏 AI 聊天 | 8107 / 8108 |
| `commandPalette.actions.toggle-ai-chat-feature.enableLabel` / `disableLabel` | 功能：启用 AI 聊天 / 功能：禁用 AI 聊天 | 8092 / 8093 |

**未走 i18n 的 AI 可见文案（硬编码英文，file:line）**：输入框 placeholder `"What do you want to create?"` / `"Ask anything... (@ files, / commands)"` / `"Ask anything... (@ to mention files)"` / `"Configure API key to enable Agent..."`（`chat-input-bar.tsx:960,962-964`）；`Remove ${image.name}`（`:1002`）；`Remove ${item.name} from context`（`:1258`）；`"Unknown"`（`:440`）；提及面板 `ariaLabel="File suggestions"`（`file-mention-dropdown.tsx:96`）、`emptyLabel = "No matching files found"`（`ai-file-selector.tsx:73`）；权限回退选项名 `"Deny"` / `"Allow"`（`acp-permission-prompt.tsx:16-17`）；斜杠项快捷提示 `"Enter"`（`slash-command-dropdown.tsx:119`）；活动行标签 `"Permission requested"` / `"Permission response"` / `"Session title updated"` / `"Agent error"` / `Plan updated (${n} steps)` / `"No plan steps"`（`ai-chat.tsx:778,997,836,872,857,854`）；`"Web Viewer is disabled. Enable it in Settings > Features to open URLs."`（`:485`）；`Opened ${url} in Lithe web viewer.`（`:495`）；`` Opened terminal and ran `${cmd}`. ``（`:504`）；`` Applied `/${cmd}`. ``（`:552`）；`"Session updated."`（`:552`）；`"The selected agent did not return a visible response..."`（`:567`）；`No Response` / `EMPTY_RESPONSE`（`:572-573`）；错误标题 `API Error` / `Rate Limit Exceeded` / `Authentication Error` / `Access Denied` / `Server Error` / `Bad Request` / `Agent Configuration Required` / `Authentication Required` / `Connection Lost`（`:597-671`）；`"Error: Failed to connect to Agent service. Please check your API key and try again."`（`:907`）；默认会话名 `"New Session"`（`chat-actions.ts:57`）；Agent buffer 名 `Agent ${n}`（`buffer.store.ts:744`）；`"Lithe Agent"` / `"Use Lithe Agent settings and provider configuration"`（`use-agent-options.ts:14-18`）；`"ACP-compatible coding agent"`（`:77`）；`"Claude Code"` / `"Open Claude Code in an Lithe terminal"`（`claude-code.ts:6-7`）；`"Antigravity CLI"` / `"Open Antigravity CLI in an Lithe terminal"`（`terminal-agents.ts:12-15`）；Codex 一串（`codex-integration-service.ts:50,170-174,122,190,184`）；`by {author}`（`skills-command.tsx:383`）；`"Scroll to end"` / `"Scroll to start"`（`message-scroller.tsx:109`）。

---

### ⑨ 全局面板（命令面板等）

#### A. 组成与层级

**快速打开（Quick Open）** —— 全局浮层，挂在 `MainLayout` 顶层（`main-layout.tsx:363-366`）：

```
QuickOpen                                      features/quick-open/components/quick-open.tsx:16-211
└─ Command（浮层外壳）                          ui/command.tsx:121
    ├─ CommandHeader：CommandInput + 计数徽标
    └─ CommandList（ScrollArea）
        ├─ @ 符号模式 / # 工作区符号模式 → SymbolListItem
        ├─ 无结果 → EmptyState
        └─ 文件模式：三段（打开的标签页 / 最近 / 其他）各为 FileListItem
```

模式判定：`query.startsWith("@")` → 当前文件符号；`query.startsWith("#")` → 工作区符号。

**命令面板** —— 同一个 `ui/command.tsx` 外壳；`commandPalette.title` = 命令面板。

**全局搜索** —— **不是浮层**（见 ⑦）。

#### B. 度量

浮层 max-h ≤ **512**、宽 ≤ **704**、圆角 **11.2**、顶距 **64**、`z-10060`；列表项 default min-h **32** / compact min-h **28** → **全部见 §1.8**。

#### C. 元素 → gpui-kit 对应

| Windows 元素 | gpui-kit 0.6.6 实现 | 判定 | 写法要点 / 坑 |
| --- | --- | --- | --- |
| **快速打开 / 命令面板浮层** | `component::command::{Command, CommandState, CommandGroup, CommandItem, CommandEntry}`（`command/mod.rs:11-13`） | **可一比一**（gpui-kit 的一等公民） | ✅ 已核对：`Command::{new(&Entity<CommandState>), group, searchable, on_query, on_confirm, placeholder, empty, max_h, bordered}`（`command/command.rs:72,100,112,130,157,177,183,195,204`）。⚠️ **`Command` 本身只是普通流式 `v_flex`，不能直接当浮层**，必须放进 Dialog 这类容器。⚠️ 确认项 `on_confirm` 回调签名是 `Fn(IndexPath, ..)` |
| 浮层容器 | `WindowExt::open_dialog`（`window_ext.rs:30`）+ `dialog::Dialog`（`dialog/dialog.rs:258`） | **可一比一** | ⚠️ **必须先挂层**（`Root::render_dialog_layer`），否则**静默不显示**；⚠️ **`open_dialog` 不能在窗口首帧之前调用，会 panic**。Dialog 默认宽 **448**（`dialog/dialog.rs:177`）→ 用 `.width(px(704.))` 覆盖；Dialog 水平居中（`:534`）、垂直默认 `viewport/10`（`:529`）；Esc 关闭 / 点外关闭 / 焦点自动归还**全部自带** |
| 浮层定位与动画（顶距 64、scale/opacity/blur） | `Dialog::margin_top`（`dialog/dialog.rs:395`）+ `Theme::motion` | **需自行组合** | Windows 是 `fixed inset-0 flex items-start justify-center pt-16`；gpui 的 `margin_top` 语义不同 → 需要 `.margin_top(px(64.))` 或自绘 overlay |
| 列表项（图标 + 标题 + 路径 + 徽标） | `CommandItem` + `Icon`；或 `list::ListItem` | **可一比一** | — |
| 分段结果（打开的标签页 / 最近 / 其他） | `CommandGroup`（`.label(..).items(..)`） | **可一比一** | — |
| `@`/`#` 模式切换（同一输入框换数据源与 placeholder） | `CommandState::set_query` + 自己按首字符切 delegate；`Command::placeholder(..)` | **需自行组合** | 是否支持"同一输入框切数据源"**未逐行核对源码** → 见 §7 |
| 搜索结果高亮（fuzzy 匹配字符加粗/变色） | 自绘 span | **需自行组合** | `component::highlighter` 面向代码，不适用 |
| 键盘导航（↑↓ 环绕、Enter、Esc / Cmd+K） | `gpui::KeyBinding::new` + `base::actions::{SelectUp, SelectDown, Confirm{secondary}, Cancel, SelectFirst, SelectLast}` + `.key_context(..)` | **可一比一** | `Command` 自带 query / selected_index。⚠️ Windows 的"`Cmd/Ctrl+K` 关闭"要自己绑 |
| 文件图标（按扩展名着色） | `Icon` + 自己的图标名映射表 | **需自行组合** | ⚠️ **默认图标集只有 101 个字形**（见 §4 缺口清单） |
| 键盘触发（`cmd+p` / `cmd+shift+f` / 双击 ⇧） | `gpui::KeyBinding` / `on_action` | **需自行组合** | ⚠️ **Windows 平台对 ⇧ 这类修饰键只发 `ModifiersChanged`、不构造 KeyDown/KeyUp**（`gpui-pre-windows-0.3.6/src/events.rs:1504-1518`）→「双击 Shift」要用 `on_modifiers_changed` + 时间戳（阈值 350ms）。⚠️ 键事件沿"焦点节点的祖先链"派发 → 根元素要 `.track_focus(&handle)` 且开局 `window.focus(&handle, cx)` |
| 设置窗口 | `component::setting::{Settings, SettingPage, SettingGroup}` | **可一比一** | `gpui-kit 明显优于手搓实现`（`UI-MAP.md` §1 末表） |
| 通知 | `component::notification::{Notification, NotificationType, NotificationDelivery}`（`notification.rs:31,53,107`） | **可一比一** | ⚠️ **位置默认 `Anchor::TopRight`**（`notification.rs:551`），Windows toast 是 **bottom-right** → 必须 `.placement(Anchor::BottomRight)`。⚠️ `NotificationSettings`：margins `px(16.)` 且 top 额外加 `TITLE_BAR_HEIGHT`（`:549-557`）、`max_items = 10`（`:558`）、**width `px(382.)`**（`:26,540`）。`NotificationDelivery::System` **天然覆盖**了 Windows 目前没有的能力（OS 通知中心） |
| **通知中心工具窗（持久列表 + 分组 + 过滤 + 搜索 + 详情）** | `NotificationList`（`notification.rs:693`）只有 `push` / `clear` / `notifications` | **没有** | 需自研 `NotificationCenterPanel`（可复用 `List` + `ListDelegate` + `Dock`） |
| 快捷键提示 | `component::kbd::Kbd::binding_for_action`（`kbd.rs:11`） | **可一比一** | — |
| 徽标 | `component::badge::Badge::{dot, count, max}`（`badge.rs:31,56-76`） | **可一比一** | — |
| 可调分栏 | `component::resizable::{h_resizable, v_resizable}` + `size_range` | **可一比一** | gpui 版还白送**双击重置**（`ResizableState::reset_panel`，`resizable/mod.rs:233`） |
| 通知（应用内）+ Toast | `window.push_notification(note, cx)`（`window_ext.rs:65`） | **可一比一** | 见上 |

#### D. 中文文案

`commandPalette.title`=命令面板（7904）、`commandPalette.placeholder`=输入命令...（7905）、`commandPalette.noCommands`=未找到命令（7906）、`commandPalette.close`=关闭命令面板（7938）、`commandPalette.clearPersistedActions`=清除持久命令（7939）、`commandPalette.actions.quick-open.label`=转到：快速打开（8026）、`commandPalette.actions.search-global.label`=搜索：全局搜索（8024）、`menu.quickOpen`=快速打开（7791）、`keybindings.commands.file.quickOpen.title`=快速打开（8285）、`quickOpen.*`（7823-7837，见 ⑦）、`database.searchDatabases`=搜索数据库（5155）、`commandPalette.categories.Database`=数据库（7946）、`menu.databases`=数据库（7809）、`keybindings.commands.database.connect.title`=显示数据库（8388）、`commandPalette.actions.database-connect.label`=数据库：显示数据库（7974）。

---

## 3. 颜色与 token 映射

> ⚠️ **实现一律走 `cx.theme()`**，**不写裸色值到实现建议里**。本节的十六进制值只用于**核对映射是否落对**。

### 3.1 Windows 语义色 → CSS 变量 → gpui-kit token

**A 层：39 个主题色键**（`extensions/themes/builtin/lithe.json`，✅已核对行号；<kbd>L</kbd> = lithe-light，<kbd>D</kbd> = lithe-dark）

| # | Windows 键 / CSS 变量 | <kbd>L</kbd> | <kbd>D</kbd> | `lithe.json:行`（L / D） | gpui-kit `ThemeColor` 对应 | 判定 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `--background` | `#ffffff` | `#1e1f22` | 13 / 79 | `background`（`theme_color.rs:67`） | **直接对应** |
| 2 | `--surface` | `#f7f8fa` | `#2b2d30` | 14 / 80 | ⚠️ **语义错位**：gpui 的 `surface` = `ThemeColor::popover`（`theme/mod.rs:418`），而 Lithe 的 `--surface` 是**面板底**（比 background 暗一档）→ **需重设** | **需自建 / 重设** |
| 3 | `--foreground` | `#1f2328` | `#dfe1e5` | 15 / 81 | `foreground`（`:163`） | **直接对应** |
| 4 | `--muted-foreground` | `#4f5965` | `#b4b8bf` | 16 / 82 | `muted_foreground`（`:195`） | **直接对应** |
| 5 | `--subtle-foreground` | `#68717d` | `#8b929e` | 17 / 83 | **未找到**（gpui 只有 `muted_foreground` 一档，Lithe 有两档） | **需自建 `LithePalette.subtle_foreground`** |
| 6 | `--border` | `#dfe1e5` | `#43454a` | 18 / 84 | `border`（`:69`） | **直接对应** |
| 7 | `--accent` | `#edf3ff` | `#393b40` | 19 / 85 | ⚠️ **语义陷阱**：gpui 的 `accent` 是**悬停底色**（默认近中性灰），Lithe 的 `--accent` 是淡蓝/暖灰**悬停底色** —— 名字对上但**值必须重设**；且 Lithe 的"强调色"是 `--primary` | **需重设值** |
| 8 | `--selected` | `#d4e2ff` | `#2e436e` | 20 / 86 | 最接近 `list_active`（`:183`）/ `table_active`；**无同名 token** | **需自建 `LithePalette.selected`**（同时建议写入 `Theme::list_active`） |
| 9 | `--selection` | `rgba(53,116,240,0.2)` | `#214283` | 21 / 87 | `selection`（`:227`） | ⚠️ **语义陷阱**：gpui 默认 **30% 透明**（`rgb(0x55a0fc)` α=0.3），而 Windows 的 dark 值是**不透明**的 `#214283` → 必须按主题分别设值 |
| 10 | `--primary` | `#3574f0` | `#3574f0` | 22 / 88 | `primary`（`:201`） | ⚠️ gpui 默认主色是**近黑**（`hsla(0,0,0.09,1)`）→ **必须覆盖** |
| 11 | `--cursor` | `#1f2328` | `#ced0d6` | 23 / 89 | `caret`（`:131`） | **近似对应**（gpui 只有 1 个光标 token，Lithe 有 3 个） |
| 12 | `--cursor-vim-normal` | `rgba(53,116,240,0.62)` | `rgba(53,116,240,0.68)` | 24 / 90 | **未找到** | **需自建** |
| 13 | `--cursor-vim-insert` | `#3574f0` | `#3574f0` | 25 / 91 | **未找到** | **需自建** |
| 14 | `--destructive` | `#cf3f4f` | `#db5c5c` | 26 / 92 | `destructive` / `danger`（`:147`） | **直接对应** |
| 15 | `--success` | `#27864f` | `#57965c` | 27 / 93 | `success`（`:249`） | ⚠️ **但 `theme.css:151` 把 `--success` 别名成 `var(--primary)`**（见 §3.3 陷阱） |
| 16 | `--warning` | `#a86400` | `#d6ae58` | 28 / 94 | `warning`（`:301`） | ⚠️ **`theme.css:152` 把 `--warning` 别名成 `var(--muted-foreground)`** |
| 17 | `--info` | `#3574f0` | `#548af7` | 29 / 95 | `info`（`:165`） | **直接对应** |
| 18-23 | `--git-modified` / `-modified-staged` / `-added` / `-deleted` / `-untracked` / `-renamed` | `#a86400` / `#bd7411` / `#27864f` / `#cf3f4f` / `#0877c1` / `#7656a8` | `#d9a441` / `#e5b75e` / `#4cc38a` / `#f16d75` / `#58a6e7` / `#c8a2f4` | 30-35 / 96-101 | **未找到**（gpui 只在 `highlight` 段有 `conflict` / `created` / `modified`） | **需自建 `GitPalette`** |
| 24-39 | `--terminal-{black,red,green,yellow,blue,magenta,cyan,white,bright-*}`（16 个） | 见 `lithe.json:36-51` | 见 `:102-117` | 36-51 / 102-117 | **未找到**（gpui 有 `chart_1..5` 与 `base.{red,green,blue,yellow,magenta,cyan}[.light]` 可借色板） | **需自建 `[Hsla; 16]`** |

**必填键只有 9 个**（校验器强制）：`background`、`surface`、`foreground`、`muted-foreground`、`subtle-foreground`、`border`、`accent`、`selected`、`primary`（`extensions/themes/theme-file.ts:5-15`，✅已核对）。
**旧格式键重映射**（自定义主题兼容，9 条，`theme-file.ts:17-27`，✅已核对）：`primary-bg→background`、`secondary-bg→surface`、`text→foreground`、`text-light→muted-foreground`、`text-lighter→subtle-foreground`、`hover→accent`、`selection-bg→selection`、`accent→primary`、`error→destructive`。**Rust 侧若支持导入用户主题，必须复刻这张表。**

**C 层：派生色**（`styles/theme.css:140-176`，✅已核对）

| CSS 变量 | 取值 | 行 | gpui-kit 映射 |
| --- | --- | --- | --- |
| `--border-strong` | `color-mix(in srgb, var(--border) 72%, var(--foreground) 28%)` | 140 | `border.mix(foreground, 0.72)`（`component::Colorize::mix`，`theme/color.rs:44-45,209-225`）。⚠️ `mix` 的 `factor` 是**第一个**颜色的权重 |
| `--popover` / `--popover-foreground` | `var(--surface)` / `var(--foreground)` | 141-142 | `popover` / `popover_foreground` |
| `--accent-foreground` | `var(--foreground)` | 143 | `accent_foreground` |
| `--muted` | `var(--surface)` | 144 | `muted` |
| `--input` | `var(--border)` | 145 | `input` |
| `--ring` | `var(--border-strong)` | 146 | `ring` |
| `--card` / `--card-foreground` | `var(--surface)` / `var(--foreground)` | 147-148 | 无 `card` token → 自建或用 `popover` |
| `--primary-foreground` | `var(--background)` | 149 | `primary_foreground`。⚠️ **不是白色** |
| `--secondary-foreground` | `var(--foreground)` | 150 | `secondary_foreground` |
| ⚠️ `--success` | **`var(--primary)`** | 151 | **`--success` 被别名成主色**（与 `lithe.json` 的 `success` 冲突） |
| ⚠️ `--warning` | **`var(--muted-foreground)`** | 152 | **`--warning` 被别名成次级文字色** |
| `--tab-bar-bg` / `--tab-active-bg` | `var(--background)` | 155-156 | 把 `Theme::tab_bar` / `tab_active` 设成 `background` |
| `--tab-hover-bg` | `color-mix(in srgb, var(--accent) 72%, transparent)` | 157 | `accent.opacity(0.72)` |
| `--symbol-function` | `var(--syntax-function)` | 170 | **未找到**（Lithe 为 IDE 符号图标专门造的 7 键） |
| `--symbol-type` / `-interface` / `-enum` / `-variable` / `-property` / `-type-parameter` | 见 `theme.css:171-176` | 171-176 | **未找到** |

**D 层：滚动条**（`styles/scrollbars.css:1-18`）：`--app-scrollbar-size` **11**、`--app-scrollbar-thin-size` **9**、`--app-scrollbar-radius` **999**、track `transparent`、thumb light `rgba(120,120,120,0.42)` / dark `rgba(166,166,166,0.38)`、hover / active 各自一套、`--app-scrollbar-thumb-border: 3px solid transparent`。→ gpui-kit 侧需自定 `ScrollbarStyles` + `Theme::set_scrollbar_mode(ScrollbarMode::Always, cx)`（`theme/mod.rs:250`，✅已核对）。

**E 层：玻璃 / chrome**（`styles/window-transparency.css:5-36`，仅在 `html:is(.platform-macos, .platform-windows):not([data-window-transparency="disabled"])` 下生效）
13 个 `--lithe-glass-*` / `--lithe-chrome-icon-*` / `--lithe-chrome-control-*`，明暗两套。⚠️ **`backdrop-filter` 被显式关闭**（`window-transparency.css:46-47,70-71,79-80,95-96`）—— 窗口透明由 **Tauri 原生层**负责，CSS 只做半透明合成。**gpui 侧没有等价的"原生窗后透"API** → 这块必须重新设计或降级为固定半透明色板（详见 §4 缺口清单）。

**F 层：语法高亮（18 键）**

| Windows `--syntax-*` | gpui `HighlightTheme` 的 `syntax.*` | 备注 |
| --- | --- | --- |
| `comment` | `comment` + `comment.doc` | 同值填两个 |
| `keyword` | `keyword` | |
| `string` | `string` + `string.escape` | 同值填两个 |
| `number` | `number` | |
| `function` | `function` | |
| `variable` | `variable` | |
| `tag` | `tag` + `tag.doctype` | 同值填两个 |
| `attribute` | `attribute` | |
| `punctuation` | **`punctuation`** (+ `.bracket` / `.delimiter` / `.list_marker` / `.special`) | ✅ gpui **有**对应 |
| `constant` | `constant` | |
| `property` | `property` | |
| `type` | `type` | |
| `operator` | **`operator`** | ✅ gpui **有**对应 |
| `boolean` | **未找到** | 建议映射到 `constant`，或走 Lithe 自有高亮器 |
| `null` | **未找到** | 建议映射到 `constant` 或 `keyword` |
| `regex` | `string.regex` | |
| `jsx` | **未找到** | 建议映射到 `tag` |
| `jsx-attribute` | **未找到** | 建议映射到 `attribute` |

→ **真正无对应的只有 4 个**：`boolean`、`null`、`jsx`、`jsx-attribute`。
另外三类在 gpui 中**零对应**：`--symbol-*` 7 键；9 个 `--syntax-markdown-*`（Windows 侧本来就是硬编码 fallback，`styles/syntax-tokens.css:110-154`）；`--syntax-*` 的"与 `foreground` 欧氏 RGB 距离 `< 28` 就回落"规则（`extensions/themes/syntax-token-colors.ts:99-128`，**Rust 侧必须复刻**）。
换向也成立：gpui 有 41 键而 Lithe 只有 18 键，gpui 的 `constructor / embedded / emphasis / emphasis.strong / enum / hint / label / link_text / link_uri / predictive / preproc / primary / text.code.span / text.literal / title / variant / punctuation.bracket / punctuation.delimiter / punctuation.list_marker / punctuation.special / tag.doctype / string.special / string.special.symbol / comment.doc / string.escape` 需要 Lithe 侧给出派生规则。

### 3.2 已记录的语义陷阱（逐条）

1. ⚠️ **gpui 的 `accent` 是悬停底色，不是强调色**。Lithe 的强调色是 `--primary`（`#3574f0`）→ 映射到 `theme.primary`；`theme.accent` 要设成 Lithe 的 `--accent`。**直搬会把强调色变成灰底。**
2. ⚠️ **gpui 的 `selection` 默认 30% 透明**（`rgb(0x55a0fc)` α=0.3），而 **Windows 的 dark `--selection` 是 `#214283`（不透明）** → 必须按主题分别设值。
3. ⚠️ **gpui 默认 `background == popover == white`**（浅色主题），而 Lithe 的 `--surface`（`#f7f8fa`）比 `--background`（`#ffffff`）暗一档 → 必须显式设 `popover = surface`。
4. ⚠️ **`--success` / `--warning` 被 `theme.css:151-152` 别名覆盖**（`--success = var(--primary)`、`--warning = var(--muted-foreground)`），但 `lithe.json` 里 `success`/`warning` 有独立值。主题注入顺序是 JS `setProperty`（inline style）晚于 CSS `:root`，因此**运行时 lithe.json 的值胜出**；但 `badge.tsx:13-14`、`empty.tsx:37`、`marker.tsx`、`alert.tsx:5-21` 都引用了 `success`/`warning` —— 若主题加载失败会落到 `primary`/`muted-foreground`。**这是有意 fallback 还是遗留冲突未查清**（`research/windows/04` §5.2 第 4 条）。
5. ⚠️ **`--primary-foreground = var(--background)`，不是白色**（`theme.css:149`）→ gpui 的 `primary_foreground` 默认是近白（`hsla(0,0,0.98,1)`），**语义不同**。
6. ⚠️ **`--lithe-chrome-hit-target` 全仓库除定义处外没有任何引用点**（`theme.css:127,195`）→ 不必实现。
7. ⚠️ **`Theme::spacing_tokens()` 直接返回 `SpacingTokens::default()`，完全不读主题**（`theme/mod.rs:482-484`）→ 间距阶梯**无法由主题配置**；要按密度改间距只能改 gpui-component 或全部走显式 `px()`。
8. ⚠️ **`RadiusTokens` 的系数与 Lithe 不同构**：gpui 是 `sm = radius/2`、`md = radius`、`lg = radius_lg`、`xl = radius*2`（`theme/mod.rs:471-480`），另有 `radius_2xl = *2.5` / `3xl = *3.` / `4xl = *3.5`（`:457-469`）；Lithe 是 `0.6/0.8/1.0/1.4/1.8/2.2/2.6` → **必须自建 `LitheRadii`**。
9. ⚠️ **`MotionTokens` 默认值与 Lithe 不同**：gpui `duration_fast = 120ms` / `duration_normal = 180ms`（`theme/motion.rs:26-27`），Lithe 是 **150 / 200ms**；缓动也不同（gpui `easing_enter = cubic_bezier(0.16,1,0.3,1)` vs Lithe `cubic-bezier(0.22,1,0.36,1)`）→ 字段全 `pub`，**直接改写**。
10. ⚠️ **`ShadowTokens` 只有 3 档且颜色统一**（`theme_tokens.rs:197-205`），Lithe 有 **5 档**且每档最后一层是 **0.5px 的 hairline 环**（`theme.css:158-168`）→ **必须自建 `LitheShadows`**。`BoxShadow { spread_radius, blur_radius, inset }`（`theme_tokens.rs:225-233`）**能**表达 `spread_radius: px(0.5)`，但 0.5px 在非 100% DPI 下的栅格化会与 CSS 有差异。
11. ⚠️ **焦点环对不上**：gpui `FOCUS_RING_WIDTH = px(3.)` / `FOCUS_RING_OPACITY = 0.5`（`component/styled.rs:11-12`），Lithe 是 `ring-2`(**2px**) / `/20` → 需要决定"接受 3px"还是"自绘"。
12. ⚠️ **滚动条行为不同**：gpui 默认 rest 6px / hover 6px / active 8px / inset 4px，idle 2s / enter 300ms / exit 500ms / expand 300ms（`theme/mod.rs:64-86`）；Lithe 是 **11px 常显 / 9px 细、无 transition**。

### 3.3 gpui-kit 没有的颜色类别 → 需要自建

按「放哪个结构体 / 怎么取取值」给出建议。

**(1) `LithePalette`**（`Global`，`lithe-ui/src/theme/palette.rs`）—— 解析 `lithe.json` 的 `colors` 段：

```rust
pub struct LithePalette {
    pub subtle_foreground: Hsla,          // 17 / 83
    pub selected: Hsla,                   // 20 / 86（同时建议写入 Theme::list_active）
    pub border_strong: Hsla,              // = border.mix(foreground, 0.72)
    pub git: GitPalette,                  // modified / modified_staged / added / deleted / untracked / renamed
    pub terminal: [Hsla; 16],             // 36-51 / 102-117
    pub cursor: Hsla,                     // 23 / 89
    pub cursor_vim_normal: Hsla,          // 24 / 90
    pub cursor_vim_insert: Hsla,          // 25 / 91
    pub syntax_markdown: Option<MarkdownSyntaxPalette>,  // 9 键，Windows 侧本来无主题键
    pub glass: LitheGlass,                // 13 个 --lithe-glass-*/chrome-* 变量
}
```

颜色混合用现成的 `component::Colorize`：`mix` / `mix_oklab` / `lighten` / `darken` / `opacity`（`theme/color.rs:20-61`）。

**(2) `LitheMetrics`**（`Global`，密度与 chrome 尺寸）—— 从静态常量表读（两档密度），乘以根缩放系数 `scale = root_font_size / 16px`：
`title_bar_height 40` / `footer_height 24` / `pane_header_height 36` / `tab_bar_height = pane_header` / `tab_height 28` / `tab_max_width 200` / `sidebar_header_height 32` / **`workbench_gap 4`（不缩放）** / `chrome_control_height 24` / `chrome_hit_target 28` / `chrome_line_height 16` / `chrome_gap_tight 2` / `chrome_gap 4` / **`chrome_gap_loose 6`（`SpacingTokens` 无此档）** / `chrome_padding_inline 8` / `chrome_radius 4`。
`comfortable` 档按 §1.1 末表覆盖。
**必须同时覆盖 gpui 的写死常量**：`TITLE_BAR_HEIGHT 34→40`（`title_bar.rs:15`）；`StatusBar` 无高度常量需显式 `.h()`；`SidebarMenuItem` `.h_7()`(28px) 与 Lithe **恰好一致**；`SidebarGroup` `.h_8()`(32px) 与 Lithe 28px **差 4px**。

**(3) `LitheRadii`**（7 档 + chrome + full）：见 §2 ① 的尺寸体系决策。

**(4) `LitheShadows`**（5 档 + 0.5px hairline）：见 §3.2 第 10 条。

**(5) Lithe 主题文件加载器** —— gpui 自带 `ThemeRegistry`（`theme/registry.rs:151,98,121,126,143,147`），但只认 `ThemeSet`/`ThemeConfig` 结构，键名是 `background` / `primary.background` / `tab.active.background` 这类点分字符串（`theme/schema.rs:252-675`），**与 Lithe 的 39 键 `colors` + 18 键 `syntax` 完全不同**。
⚠️ **`ThemeRegistry` 没有 `load_themes` / `set_theme`**：只有 `load_themes_from_str`（`registry.rs:151`）；`reload_themes`（`:226`）/ `reload`（`:238`）都是私有。
**推荐的换肤流程**：① 解析 Lithe 主题 → 写入 `Theme` 的 legacy 字段（含 `font_size = px(16.0 * scale)`、`radius`、`mono_font_family`、`motion`、`highlight_theme`）；② **必须** `Theme::sync_base(cx)`（`theme/mod.rs:367-372`）（否则滚动条/把手仍用旧值）；③ 若要让设置界面看到，再走 `ThemeRegistry::global_mut(cx).load_themes_from_str(..)`。
**必须自建的部分**：9 个必填键校验；旧键重映射 9 条；syntax 的欧氏距离 `< 28` 回落；`appearance` → 决定写入 `light_theme` 还是 `dark_theme`；39 键里 gpui 完全不认识的 **26 键**（git 6 + terminal 16 + cursor 3 + subtle-foreground 1）。

**(6) 未找到对应、需要产品决策的**：`--symbol-*` 7 键；9 个 markdown syntax 键；玻璃/透明窗口；双色图标（205 个 + IntelliJ light/dark）。

---

## 4. 缺口清单

> 列：**缺口是什么 / 建议怎么补 / 谁来做**（自研 / 用近似 / 登记跳过）。

| # | 缺口 | 建议怎么补 | 谁来做 |
| --- | --- | --- | --- |
| 1 | **终端模拟器 / PTY / ANSI 解析**（gpui-kit 三个 crate 全部 `.rs` 与 `Cargo.toml` 精确检索 `portable_pty\|conpty\|alacritty\|termwiz\|crossterm\|vte\|vt100\|ansi_escape\|termion\|wezterm` → **0 命中**） | PTY + shell 发现放 Rust 平台层（跨端契约明确不归 Core）；UI 照搬 `TerminalTransport`/`TerminalSession` 字段契约。VT 渲染两条路：① 自研 `Element`/`canvas` 字符网格（引 `portable-pty` + `vte` / `alacritty_terminal` / `wezterm-term` / `vt100`）；② 内嵌第三方终端窗口。⚠️ **`gpui-pre` 能否嵌入原生窗口/视图未核实** —— 这决定走哪条路 | **自研**（决策门：先核实能否嵌入原生视图） |
| 2 | **终端字体的字形细节**：`customGlyphs`、`rescaleOverlappingGlyphs`、`reflowCursorLine:false`、Unicode 11 宽度表、双宽字符/组合字符 | 自研 VT 渲染时一并实现；Unicode 宽度表引 `unicode-width` + 自定义补丁 | 自研 |
| 3 | **提交图泳道**（竖线 + 贝塞尔 + 节点圆；`ROW_HEIGHT 30` / `LANE_GAP 13` / `GRAPH_PADDING 8` / 线宽 1.6 / 节点 `r=4.3`） | 几何算法可从 Windows 的 `git-graph-row.tsx` 移植（`svg` 路径 → gpui `canvas` / `Path`） | **自研** |
| 4 | **并排 diff**（unified / split / 空白字符 / 逐 hunk 暂存 / hunk 头 44px gutter） | gpui-kit **完全没有 diff 组件**；行渲染可挂在 `VirtualList` 上，语法高亮借 `component::highlighter` | **自研** |
| 5 | **多选**（`TableState` / `ListState` / `TreeState` 只有单选） | 提交表 / 变更列表 / 诊断的多选在应用侧维护集合，由 delegate 自行渲染选中样式。唯一例外：`SearchableListState` 本身支持多选（`searchable_list/state.rs:134-219`） | **自研**（应用侧） |
| 6 | **横向滚动条** | gpui `Styled` 有 `overflow_x_scroll()`，但**没有 `.overflow_y_scrollbar()`**（0.6.6），且横向滚动条要自己挂 `Scrollbar::horizontal(..)` | **自研/组装** |
| 7 | **拖拽排序（标签 / 列表 / 树 / 文件）** | gpui 只有 dock 拖拽（`base/dock/drag.rs`），**不是通用 sortable**。建议复刻 4 个参数：激活距离 **5px**、碰撞 `pointerWithin → closestCenter`、位移 **180ms**、rAF 点击抑制 | **自研** |
| 8 | **拖拽分栏的手感**（4px 命中区 + 1px 线 + hover 变主色 + 拖动遮罩 + 帧节流） | `ResizablePanelGroup::with_handle_appearance` 可覆盖外观；Windows 的 rAF 节流在 gpui 里由"框架每帧重排"替代（不需要移植）。**双击重置** `ResizableState::reset_panel` 白送 | **用近似**（外观微调） |
| 9 | **`only-hover` 子控件**（暂存按钮 `opacity-0 → group-hover:opacity-100`，多个面板都有） | gpui **没有 group-hover 等价物** → 用 `on_hover` 自管 wrapper 的 `opacity` | **自研**（应用侧小工具） |
| 10 | **`ResizeObserver` 语义**（引用树工具栏自适应溢出、`Dropdown` 重定位） | gpui 换成 `base::measure` / `on_children_prepainted` 或 `Element::request_layout` 后读 `bounds` | **用近似** |
| 11 | **`IntersectionObserver` 语义**（懒加载哨兵 640px / 高亮预取 240px） | 用 `VirtualList` 的可见区间回调 / `ListDelegate::load_more_threshold` 替代（语义等价物存在，机制不同：委托回调 ≠ 观察者） | **用近似** |
| 12 | **拖放五区（左/右/上/下/中）+ 五区高亮** | `DockArea::split_at`（`dock/dock_area.rs:580`）+ `InsertTarget::Split` 只覆盖 dock 的插入；编辑区 pane 的拖放遮罩（`ring-2 --primary` / `bg-primary/14` / 150ms）需自绘 | **自研** |
| 13 | **活动栏项目轮播（横滑切项目）** | gpui-kit 无 carousel-for-sidebar → 用 `base::motion::timing` 自实现（阈值：位移上限 `railWidth × 0.96`、超 `×0.82` 立即提交、否则 40ms 后按 42px 判定） | **自研**（且**Windows 产品里是否可达未确认** —— `MainLayout` 只传 `expanded={false}`） |
| 14 | **通知中心工具窗（持久列表 + 分组 + 过滤 + 搜索 + 详情 + 右键）** | gpui 的 `notification` 只有 toast + `NotificationList`（只有 `push` / `clear` / `notifications`）→ 自研 `NotificationCenterPanel`（`Dock` + `List` + `ListDelegate` + `setting` 组合） | **自研** |
| 15 | **Drawer（snapPoints / 嵌套堆叠 / 滑动手势）** | gpui **没有 drawer 模块** → 基于 `sheet::Sheet` 扩展或自研 | **自研**（`ui/drawer.tsx` 在 `src/ui` 外 **0 引用**，可考虑**登记跳过**） |
| 16 | **`card.tsx`（4 档 variant + `--card-spacing` 变量）** | **未找到 `pub struct Card`**；最接近 `group_box::{GroupBox, GroupBoxVariant}`（`group_box.rs:11-16,62`，3 档） | **用近似** |
| 17 | **`ChromeBar/Group/Label/Separator` chrome 排版原语** | gpui 无 → 薄封装 3 个 `Div` 构造器 + `LitheMetrics` | **自研**（薄） |
| 18 | **`Command` 的命令面板交互（视口定位 / 分组 / 多 Tab / footer 快捷键条）** | gpui `command::Command` 交互模型不同 → 用 `Dialog` + `Input` + `List/Scrollable` + `Tab(bare)` 组装 | **需自行组合** |
| 19 | **`Dropdown`（命令式视口碰撞定位 + `ResizeObserver` 重定位 + 捕获阶段 Escape）** | 用 `popover::Popover`（声明式）替换手写定位；若要复刻手写定位需 `window` 锚点信息 | **用近似** |
| 20 | **IntelliJ 式 connected tab 3px 强调线** | gpui `TabVariant` 没有 → 自绘 `before:` 等价元素 | **自研**（小） |
| 21 | **IDE 式文件树缩进指导线** | gpui `Tree` 无 → 自绘 1px 竖线（`--tree-guide-color` = `mix(subtle-foreground 20%)`） | **自研**（小） |
| 22 | **标签脏标记 / Pin** | 任何 crate 都没有 → 自绘 | **自研**（小） |
| 23 | **标签拖拽重排** | `TabBar` 无 `on_drag`；唯一参考实现在 dock `tab_panel.rs:150-581` | **自研** |
| 24 | **`IconButton` / `CountBadge` / `Modal` / `SearchInput` / `TextInput` / `TextSize` / `Splitter` / `PanelGroup` / `TabPanel` / `TreeDelegate` / `ListDelegate`(gpui 本体)** | **全部不存在**（名称检索 0 命中）。替代：`Size` + `Sizable`；`Badge::count`；`Dialog` + `WindowExt::open_dialog`；`Input` + `InputState`；`text_xs/sm/base/lg`；`h_resizable`/`v_resizable`；`ResizablePanelGroup`；dock `TabGroup`；`Tree::new(&state, render_item)`；`component::list::ListDelegate` | **用近似** |
| 25 | **`ScrollbarState`** | **存在但私有**（`base/scrollbar.rs:149`，未 re-export）→ **不要找它**，用 `ScrollableElement` | **登记跳过** |
| 26 | **`ActiveTheme for Context<T>` / `for Window`** | **只有 `impl ActiveTheme for App`**（`theme/mod.rs:48-53`）→ `cx.theme()` 依赖 `Deref<Target = App>`；若编译失败改用 `Theme::global(cx)` | **用近似** |
| 27 | **公开的通用 `.tooltip()` 扩展** | **没有**（`ManagedTooltipExt` 与 `Root::tooltip_overlay` 都是 `pub(crate)`）→ 通用元素手搓 `Tooltip::new` + `on_hover`；优先给 Button 用自带 `.tooltip()` | **用近似** |
| 28 | **`BreadcrumbItem` 的下拉能力** | **没有**（不实现 `ParentElement`）→ 换 `DropdownButton` | **用近似** |
| 29 | **独立「活动栏 / Activity Bar」组件** | **没有** → 自建竖条（`Button` + `Tooltip`）或硬覆盖 `Sidebar` 宽度 | **自研**（小） |
| 30 | **Tabs 垂直方向 / 内建关闭按钮** | `TabBar`/`Tab` **都没有** → 垂直布局自绘；关闭按钮走 `Tab::suffix` | **自研** |
| 31 | **玻璃 / 透明窗口（`backdrop-filter` + 原生窗后透）** | gpui 无 backdrop-filter、无原生"窗后透" → **降级为固定半透明色板**（直接取 light/dark 覆盖值，不做 `color-mix` 动态混合） | **登记降级** |
| 32 | **双色图标（205 个 + IntelliJ light/dark）** | Windows 靠 CSS `display: none/block` 切换；gpui 需在渲染时按 `Theme::global(cx).is_dark()` 选 `<image>` href。**205 个图标的映射表需要单独出一份文档** | **自研 + 单独立项** |
| 33 | ⚠️ **图标缺失（本次核查出的硬缺口）** | 应用注册的是 `gpui_kit::assets::Assets`，它**只嵌入 `gpui-kit-assets-0.6.6/default-icons.txt` 列出的 101 个字形**（build.rs:73-91 用 `default_paths` 过滤；`IconName` 枚举是完整目录，**用不在默认集里的字形会编译通过但运行时画空白**）。**逐项核对结果**：<br>• **默认集里没有**：`git-branch`、`git-commit`、`git-graph`、`git-merge`、`git-compare`、`tag`、`trash`、`bug`、`database`、`pin`、`wrap-text`(= `text-wrap`)、`list-tree`、`code`、`triangle-alert`（**有**）、`star`（**有**）、`search`（**有**）、`eye`/`eye-off`（**有**）、`fold-vertical`/`unfold-vertical`（**有**）、`copy`（**有**）、`plus`/`minus`（**有**）、`folder`/`folder-open`/`folder-closed`（**有**）、`file`/`file-text`（**有**）、`square-terminal`（**有**）、`panel-bottom`/`panel-left`/`panel-right`（**有**）、`play`/`pause`（**有**）、`network`（**有**）、`hard-drive`（**有**）、`book-open`（**有**）、`rotate-cw`（**有**）、`settings`/`settings-2`（**有**）、`github`（**有**）、`circle-check`/`circle-x`（**有**）、`window-*-*`（**有**）<br>• **缺的用最接近字形代替并登记**（`docs/DESIGN.md` 的图标映射）：`git-branch→Network` 或 `Map`、`git-commit→CircleCheck` 或 `Asterisk`、`trash→Delete`、`bug→Bot`、`database→HardDrive`、`pin→Star`、`tag→BookOpen`、`wrap-text→Replace`<br>• **要拿全量**：注册 `gpui_kit::assets::AllAssets`（`gpui-kit-assets-0.6.6/src/lib.rs:36`，✅已核对 `pub use native_assets::{AllAssets, Assets};`）—— 但会显著增大二进制 | **登记跳过（当前）/ 后续换 `AllAssets`** |
| 34 | **拖拽幽灵（跟随指针 + 阴影）** | 自绘（`z-9999` / opacity `.95` / padding 6/12 / border 2 `--primary` / radius 8 / `--shadow-popover`，跟随 +10/−10） | **自研**（小） |
| 35 | **`color-mix()` 动态混色**（`--border-strong`、行悬停 `mix(accent 68%)`、命中高亮 `mix(primary 30%)`…13+ 处） | 用 `component::Colorize::{mix, mix_oklab, opacity}` 在 Rust 侧算，**不要**试图在 gpui 里做 CSS 变量式惰性解析 | **用近似** |
| 36 | **`--symbol-*` 7 键 / 9 个 markdown syntax 键** | 全无对应 → 建议在 Rust 侧把 markdown 高亮颜色**提升为主题键**（不要沿用硬编码），符号色自己定派生规则 | **自研** |
| 37 | **`DataTable` 无法逐行变高**（表级 `Size`） | 若结果网格必须支持展开行 → 放弃 `DataTable`，改 `uniform_list`（行高取第 0 行）或 `base::VirtualList`（变高） | **设计取舍** |
| 38 | **`uniform_list` 放进 `h_resizable` 中间栏后视口高度为 0**（本项目实测：`visible_range` 为 `0..0`/`0..1`，屏幕上一个像素都不画；`ListSizingBehavior::Auto` 也救不回来） | 改固定行高的 `v_flex()` 直接铺行；数据量大到需要虚拟化时再回来解决。见 `PLAN.md` 本轮结论 | **登记已知坑** |
| 39 | **`ThemeRegistry` 没有 `load_themes` / `set_theme`** | 只有 `load_themes_from_str`；官方换肤路径是自定义 JSON 里置 `"is_default": true`（`theme/schema.rs:40`）后调 `Theme::change`，或直接替换 `Theme::global_mut(cx).light_theme / dark_theme` | **用近似** |
| 40 | **数值型 ARIA（`aria-valuenow/min/max`）** | gpui-kit 有 `role(..)` / `aria_selected` / `aria_expanded` / `accessibility_label`，**数值型未见对应 API** | **登记跳过** |

---

## 5. 与本项目已确定规则的衔接

> 本节把 `UI-MAP.md` §1 的**仍然有效**的结论按"Windows 规格"重新标注适用性。照抄并标明适用。

### 5.1 仍然有效、且 Windows 规格需要它的硬规则

| 规则 | Windows 规格下是否适用 | 说明 |
| --- | --- | --- |
| 单位可以原样搬（`px(1.)` 直搬） | **适用，但换算链不同**：Windows 是 Tailwind 1 单位 = 4px、`--app-ui-scale: 1` → 根字号 16px | macOS 的 1pt = 1 逻辑像素；Windows 要多一步 `×4` 与 `ui-text-*` 的 px 字面量 |
| 颜色与字号一律走主题 token | **适用** | 见 §3；24 项需要自建 |
| 文案取本地化原文 | **适用，但来源改成 `windows/tauri/src/i18n/locale.ts`** | 不是 `macos/Resources/zh-Hans.lproj` |
| 图标只能在默认图标集里选 | **适用且更严重** | 默认集只有 **101** 个字形（本次核查）；Windows 设计里用到的一批 Git/数据库/调试字形**都不在集内**（§4 #33） |
| 不写平台业务分支 | **适用** | 但 Windows 规格本身有 `IS_WINDOWS` 分支（如三键位置、`shouldUseNativeMenuBar`），实现时要收敛到 gpui 平台层 |
| 一个区域一个模块 | **适用** | 区域划分见 §2 的 ①–⑩ |
| 缺 API 先读源码 | **适用** | 源码根不变 |

### 5.2 仍然有效的 gpui-kit 实现规则（Windows 规格下逐条复核）

| 规则 | 依据 | Windows 规格下的影响 |
| --- | --- | --- |
| `base::{TreeState, TreeItem, TreeEntry}` 的表体是 `uniform_list`，**行高取第 0 行**；`.h()` 压不住内容 | `gpui-pre-0.3.6/src/elements/uniform_list.rs:359-371,508-509` | 侧栏树行高 = 固定 **24**（`file-tree-row.ts:7-14`）**正好满足**；引用树全部 `h-6` 也满足 |
| **`ListItem` 的 children 是竖排** | `list/list_item.rs:215-221` | 每个"一行多元素"的行都要自己套 `h_flex()`；Windows 的树行 / 提交行 / 运行配置行都受影响 |
| `DataTable` 行高是**表级**值 | `sizing.rs:57-65` | 提交表 30px **正好是 `Size::Small`**；诊断行 / 结果网格行高不固定 → 需别的方案 |
| `TableDelegate::render_td` 的返回值会被组件再包一层 | `table/state.rs:1323-1329` | 单元格不用自己加 `whitespace_nowrap` |
| `Column` 只有 `width/min_width/max_width`，**没有 flex** | — | Windows 的"剩余宽度给提交列"必须折算成具体像素 |
| `Panel::title_bar()` / `inner_padding()` 返回 `false` → 不画标题栏/内边距 | `component/dock/panel.rs:146,137`（✅已核对） | 底部窗 / 编辑器面板自绘头部的前提 |
| `DockArea` 的 Bottom dock 只横跨中心列 | `dock_area.rs:1415-1431` | Windows 的"底部窗两种宽度模式"（`editor` / `full`）用"是否放在 center 的 `v_split` 里"表达 |
| ⚠️ 老结论已作废：`viewport_size()` **不是**物理像素，**不要** `÷ scale_factor()` | `UI-MAP.md` §1.3 | 适用 |
| **`TitleBar` 默认 34px、`StatusBar` 高度自定；两个都实现 `Styled`，用 `.h(px(..))` 覆盖** | `title_bar.rs:15`（✅已核对） | **`TitleBar 34→40` 是必须显式覆盖的第一项**；`StatusBar` 需 `.h(px(24.))` |
| 窗口口径与 Dodona 一致（可见区 94%、居中、普通窗口） | `UI-MAP.md` §1.3 | 适用；**不要**照抄 Windows 的 1200×800 |
| 根视图用 `.size_full()`；`viewport_size()` 不要除以 `scale_factor()` | `UI-MAP.md` §1.3 | 适用 |
| **多行输入 / 代码编辑器必须显式给高度**（`rows` 默认 2） | `input/input.rs:706-709`、`input/base/mode.rs:83-85` | Windows 的提交说明输入 `rows={2}`、composer `.auto_grow(2, N)` 也踩同一条 |
| **滚动条要显式改成常显** | `base/scrollbar.rs:48-56`；`Theme::set_scrollbar_mode`（`theme/mod.rs:250`，✅已核对） | Windows 的两套滚动条都是常显（11px / 10px）→ 适用 |
| ⚠️ **代码编辑器不吃滚轮，宿主必须自己接** | `input/base/element.rs:202-213`；`input/base/state.rs:2806-2817` | 适用 |
| Dock 的 `tab-content` 是 `overflow_y_scroll` | `dock/tab_panel.rs:728-734` | 适用 |
| 深色主题要显式 `Theme::change(ThemeMode::Dark, None, cx)` | `theme/mod.rs:261`（✅已核对） | Windows 默认主题是 `lithe-dark` → 一致 |
| **`Tab::icon()` 会丢掉 label**；`TabBar::max_width()` 是截断前提 | `tab/tab.rs:719-744` | 标签"图标 + 文件名"必须走 `Tab::child(...)` |
| **浮层必须先挂层**（`Root::render_dialog_layer / sheet / notification`） | `root.rs:185-295,586-602` | 命令面板 / 快速打开 / 对话框全部依赖 |
| ⚠️ **`open_dialog` 不能在窗口首帧之前调用，会 panic** | `root.rs:176-182`；`window_ext.rs:162` | 同上 |
| 键盘触发要挂在有焦点的根元素上；Windows 对 ⇧ 只发 `ModifiersChanged` | `gpui-pre-0.3.6/src/window.rs:6244-6252`；`gpui-pre-windows-0.3.6/src/events.rs:1504-1518` | 快速打开 `cmd+p` / 命令面板双击 ⇧ 都受影响 |
| 命令面板走 `window.open_dialog` + `Command.bordered(false)`；`Command` 不能直接当浮层 | `dialog/dialog.rs:529,534`；`command/state.rs:819-837` | ✅ 与 Windows 的 `ui/command.tsx` 外壳语义一致（**快开/命令面板是一等公民可一比一**） |
| `open_sheet` 是右侧 350px 抽屉；`Popover` 必须有 trigger | `sheet.rs:62-63` | Windows 的 `SheetContent side` 四向映射见 `research/windows/04` §2.11 |
| **能力缺口（终端 / 多选 / 泳道 / 并排 diff）** | `UI-MAP.md` §1.3 | 与 §4 缺口清单第 1/3/4/5 条一致，**Windows 规格下结论不变** |

### 5.3 `UI-MAP.md` §1 里"gpui-kit 优于 macOS 手搓"的 7 处优化 —— Windows 规格下的新鲜度

| 用途 | 用 gpui-kit 的 | Windows 现状 | 结论 |
| --- | --- | --- | --- |
| 命令面板 / 快速搜索 | `component::command::Command` | **Windows 有**（`ui/command.tsx`，快开 + 命令面板共用外壳） | **仍然推荐**（gpui-kit 是一等公民，可一比一） |
| 设置窗口 | `component::setting::{Settings, SettingPage, SettingGroup}` | Windows 有设置页 | 推荐 |
| 通知 | `component::Notification` | Windows 有 sonner toast + 独立通知中心 | 推荐（且 `NotificationDelivery::System` 是额外能力） |
| 可调分栏 | `component::resizable::{h_resizable, v_resizable}` + `size_range` | Windows 有 `react-resizable-panels` | 推荐（白送双击重置） |
| 贴底跟随的输出区 | `component::message_scroller` | Windows 的 `@shadcn/react/message-scroller` **不在本仓库** | **推荐**（gpui-kit 的 `is_following_tail` / `jump_button` / `with_bottom_fade` 是现成的） |
| 快捷键提示 | `component::kbd::Kbd::binding_for_action` | Windows 有 `ui/kbd.tsx` | 推荐 |
| 徽标 | `component::badge::Badge::{dot, count, max}` | Windows 的 `Badge` 是状态胶囊（应映射到 `tag::Tag`），**数字角标**场景才用 `badge::Badge` | **注意语义分工**：`Badge` = 数字角标（未读 9+）；`Tag` = 状态胶囊 |

---

## 6. 验收清单（按本文档复刻时逐条过）

1. **数值速查表**（§1）里每一行都能在实现里找到对应常量，且**没有裸数字**散落在组件代码里。
2. **每个区域**（§2 ①–⑩）的"组成与层级"与 Windows 源码一致；区域的四块内容（组成/度量/对应/文案）都落到代码。
3. **每个元素**都能在 §2 的对应表里查到写法与坑；标"没有"的走 §4 缺口清单的处置。
4. **文案逐字一致**，且来源是 `windows/tauri/src/i18n/locale.ts`（**不是自己编的中文，也不是 macOS 的**）。
5. **颜色一律走 `cx.theme()`**；§3.2 的 12 条语义陷阱逐条不犯；§3.3 的 5 个自建结构体到位。
6. **图标只用默认集的 101 个字形**（或显式注册 `AllAssets`）；缺字形的地方在 `docs/DESIGN.md` 的图标映射里登记了替代。
7. **状态齐全**：空 / 加载 / 失败 / 禁用 / 选中 / 悬停 / 拖拽，逐一可见。
8. **没有平台业务分支**；Windows 特有的三键位置、菜单栏策略收敛到平台层。
9. **证据**：截图 + 日志落到 `.artifacts/p1/`，并在 `PLAN.md` 的当前状态里更新。

---

## 7. 未确认清单（明确列出，不含猜测）

### 7.1 调研文档自身标注的"未查到"（本次未逐一复核，直接转记）

| # | 未确认项 | 出处 |
| --- | --- | --- |
| 1 | `--ui-text-xs` / `ui-text-lg` 的 CSS 定义位置（`utilities.css:30-44` 只有 4 个 `ui-text-*`）→ 实际渲染字号未查清 | `research/01` §7.2、`research/02` §2.0 警告、§7.5 |
| 2 | `compactMenuBar` 的默认值与取值范围（只确认 `title-bar.tsx:61` 读取与三处渲染分支）→ **本次已补**：`default-settings.ts:105` 默认 `true` | `research/01` §7.3 |
| 3 | `windowChromeDensity` 的可选值与默认档 → **本次已补**：合法值只有 `focused \| comfortable`（`settings-normalization.ts:149-152`），默认 `"focused"`（`default-settings.ts:97`） | `research/01` §7.4 |
| 4 | 活动栏展开态（`expanded=true`）在 Windows 产品中是否可达（`MainLayout` 只传 `false`） | `research/01` §7.5 |
| 5 | 状态栏 `gitChanges` 点击目标在右栏不可用时的行为 | `research/01` §7.6 |
| 6 | `menu_execute_command` 的命令 ID 全集 | `research/01` §7.7 |
| 7 | `toggle_menu_bar` 的真实宿主（Rust 侧未找到实现） | `research/01` §7.8、§4.3 |
| 8 | `SidebarProjectDots` 的 ContextMenu 与主 rail 右键菜单的命中顺序 | `research/01` §7.9 |
| 9 | 窗口三键是否重复（自绘 vs 系统按钮）在 Rust 重写后如何取舍 | `research/01` §7.10 |
| 10 | `.file-tree-empty-state` 的 CSS 规则未找到（类名有使用、全仓 CSS 无定义） | `research/02` §7.1 |
| 11 | 活动栏 Rail 分隔条无键盘支持 | `research/02` §7.2 |
| 12 | `write_file` 的前端调用点未精确定位 | `research/02` §7.4 |
| 13 | `BottomBufferPane` 的完整结构与 TabBar 复用方式未展开 | `research/02` §7.6 |
| 14 | `GitView`（侧栏 git 面板）内部结构未展开 | `research/02` §7.7（本文已在 ④ 补到组件树级） |
| 15 | `file-explorer-tree.tsx`（1712 行）的拖拽自动滚动 / 多选 / 剪贴板粘贴细节只做了检索式确认 | `research/02` §7.8 |
| 16 | **Monaco 内部子 UI（折叠 / minimap / 补全/悬停 widget / peek view）的精确度量未逐条核对** | `research/02` §7.9 |
| 17 | `git.fetch.options` / `git.expandAll` / `git.collapseAll` / `git.checkout` / `git.push` / `git.deleteBranch` / `git.deleteBranchConfirm` / `git.delete` / `git.actionCompleted` / `git.actionFailed` / `git.unifiedView` / `git.splitView` 等键的 **zh 行号**未逐条核定 | `research/03` §7.1-7.2、§5.4/§5.7 |
| 18 | `GitStatusPanel` 右键 `Dropdown` 的宽度/圆角（走 `Dropdown.items` 路径，未指定 className） | `research/03` §7.3 |
| 19 | `--lithe-pane-header-height` / `--lithe-sidebar-header-height` 在舒适密度下的覆盖（**已确认 `theme.css:188-200` 没有覆盖 pane-header**） | `research/03` §7.5 |
| 20 | 终端面板 / 运行面板在底部窗内的**精确头部度量**未逐项核定（`terminal-tab-bar.tsx:465,712-713`；`run-pane.tsx:394-417`）→ **本文 ⑤⑥ 已补主要项** | `research/03` §7.6 |
| 21 | `DiagnosticsPane` 的完整头部/列表度量（仅知 `:899` / `:1077` / `:1212`）→ **本文 ⑥ 已补** | `research/03` §7.7 |
| 22 | `bottomPaneActiveTab` 的完整取值类型定义（本文已从 `ui-state.types.ts:17-26` 补全） | `research/03` §7.8 |
| 23 | **底部窗高度是否持久化**（倾向"不持久化"） | `research/03` §7.9 |
| 24 | `--app-ui-scale` 的运行时来源（`appearance-bootstrap.ts:153-155`） | `research/03` §7.10 |
| 25 | **gpui-kit 是否有"百分比初始面板尺寸"**（`ResizablePanel::size` 只接受 `Pixels`；`resizable/mod.rs:137` 的 `adopt_sizes` 是 `pub(crate)` 不可用） | `research/03` §7.11 |
| 26 | gpui-kit 对"仅悬停显示"子控件的原生支持 | `research/03` §7.12 |
| 27 | `Sidebar` → `SemanticThemeTokens` 的消费情况：**哪些组件已改用 `semantic_tokens`、哪些还在读 legacy `ThemeColor` 字段未查清** | `research/04` §5.3 第 15 条 |
| 28 | `window.rem_size()` 与 `add_fonts` 的交互未核对 | `research/04` §5.3 第 16 条 |
| 29 | **205 个图标的逐项映射未做**（需单独出一份文档） | `research/04` §5.4 |
| 30 | `src/ui/command.tsx` 的 22 个导出到 gpui 的逐项重构方案未细化 | `research/04` §5.4 |
| 31 | **Dock 能否直接承载 Lithe 的可拖拽工作台布局未评估** | `research/04` §5.4 |
| 32 | 数据库 sidecar（`lithe-database` crate）**在本仓库不存在**；协议版本/帧格式/超时只在错误文案里留线索 | `research/06` §7.1、§1.0 |
| 33 | 数据库连接凭据落点 / `list_saved_connections` 等的持久化位置 **未找到** | `research/06` §7.2 |
| 34 | AI 聊天库（SQLite）的表/索引定义 **未找到** | `research/06` §7.3 |
| 35 | ACP 子进程 spawn/stdio 细节 **未找到**（`rust/lithe-core/src` 无 `acp` 命中；`shared/contracts/` 无 AI chat/ACP 契约） | `research/06` §7.4 |
| 36 | `acp-event` / `codex-event` 的生产端 **未找到** | `research/06` §7.5 |
| 37 | **`Esc` 停止生成**：`shortcut={isStreaming ? "escape" : "enter"}` 只用于 tooltip，**未找到对应 keydown 处理** | `research/06` §7.6 |
| 38 | AI 消息搜索的匹配与高亮实现（`message-search.ts` 未逐行核对） | `research/06` §7.7 |
| 39 | 全局搜索「替换」的写入通道（`source-replace.ts` 未逐行核对） | `research/06` §7.8 |
| 40 | 快速打开的工作区符号 LSP 方法名与错误处理 | `research/06` §7.9 |
| 41 | `useSymbolSearch`（`@` 当前文件符号）的数据来源 | `research/06` §7.10 |
| 42 | **`@shadcn/react/message-scroller` 的内部实现**（自动滚动判定与粘底恢复算法不在本仓库） | `research/06` §7.11 |
| 43 | gpui-kit 侧尚未核实的具体点：`empty::Empty` 的组合 API 是否六件套齐全；`bubble` / `message` 的完整 API；`CommandState` 是否支持"同一输入框切数据源"；`Popover` 是否支持 `w-(--anchor-width)` 式宽度绑定 | `research/06` §7.12 |
| 44 | `debug.*` 契约与 `command.rs` 名称表不完全对齐（只有 5 个字符串命中；Windows host 实际用到 `debug.createSession` / `setBreakpoints` / `setVariable` / `destroySession`） | `research/05` §8 第 2 条、§5.3 |
| 45 | 远程终端的 Rust 实现位置未定位；`warm_terminal_environment` / `lint_code` 的前端或后端落点未定位；`lsp://diagnostics` 的 Rust 发射端未定位 | `research/05` §8 第 3-6 条 |
| 46 | 问题面板的**行高未实测**（`diagnostics-pane.tsx:1122` 无 `h-*`/`min-h-*`，实际行高取决于单行/双行与 `wrapMessages`） | `research/05` §8 第 12 条 |
| 47 | `TerminalTabBar` 在 `widthMode` 下的宽度差异（`main-layout.tsx` 的消费点未逐行核对） | `research/05` §8 第 13 条 |
| 48 | gpui-kit 侧的两个坑未实测：`List` 要求所有 item 同高（含 section header/footer，因此虚拟列表**不能加 `gap_y`**）；`Tab`/`TabBar` 的 `.h()` 覆盖后 `inner_height` 的固定值可能导致内容被裁 | `research/05` §8 第 1 条 |
| 49 | `RunOutputText` 的 ANSI 覆盖度未与 xterm 对齐（**不处理光标控制、擦除、备用屏幕**；运行输出**没有换行开关**） | `research/05` §8 第 10 条 |
| 50 | 终端 profile 自定义存储格式（`profiles.store.ts`）未逐行读取 | `research/05` §8 第 11 条 |

### 7.2 本次核查中新发现 / 修正的、仍需后续动作的项

| # | 项 | 说明 |
| --- | --- | --- |
| A | **图标缺口需要单独立项**（§4 #33） | Windows 图标层有 205 个 `export const`（`ui/icons.tsx`）+ IntelliJ 双色资源表（`ui/icons/idea-assets.generated.ts`）；**默认图标集只有 101 个**。需要一份"205 → gpui-kit 字形（或自备 SVG）"的映射文档 |
| B | **`gpui-kit` 的 `Storage` / 设置持久化落点未评估** | Windows 用 `localStorage` + zustand persist（`lithe-tab-sessions`、`git-log-preferences`、`lithe-run-preferences`、`lithe-debugger-*`、`diagnostics-pane-prefs`、`lithe-ai-chat-settings-v7`…）；gpui 侧对应物（`GlobalState` + 自建 JSON）**未评估** |
| C | **`git.log.*` 的 zh 行号批量核定** | 本文 ④ 已抄了有行号的项；`git.expandAll` / `git.collapseAll` / `git.checkout` / `git.push` / `git.deleteBranch` / `git.unifiedView` / `git.splitView` / `git.fetch.options` 等**缺 zh 行号**，需要一次 `locale.ts` 全量扫描 |
| D | **`--lithe-*` 令牌的"是否走 gpui 主题"决策未做** | `research/windows/04` §3.2 的建议是"自建 `LitheMetrics`（`Global`）+ 显式 `px()`"，但 `Theme::font_size` 承担 rem 基准这件事**必须先定**（否则 `h_8()` / `p_1()` 的缩放行为与 Windows 不一致） |
| E | **`DebuggerView` 与 `RunPane` 头栏高度不一致（40 vs 36）** | 这是 Windows 源码的**事实**（`debugger-view.tsx:362` vs `run-pane.tsx:195`）。重写时要决定"照抄不一致"还是"统一"——本文档记录事实，**不做决策** |

---

## 8. 本次核对记录（抽样核对 ≥15 条）

> 规则：**核对过的在文中标 `✅已核对`；发现错的直接改对并标 `🔧已修正(原写 X)`。**

### 8.1 修正清单（2 条）

| # | 项 | 原写 | 实际 | 处置 |
| --- | --- | --- | --- | --- |
| 🔧1 | `research/windows/03-git-and-bottom.md` §6.1 第 37 行表格：声称 `gpui-kit-assets-0.6.6/assets/icons/` 下的图标"**可一比一**"，并列出 `git-branch.svg`、`git-commit-horizontal.svg`、`git-graph.svg`、`tag.svg`、`trash.svg`、`folder.svg`、`network.svg`、`search.svg`、`eye.svg`、`eye-off.svg`、`fold-vertical.svg`、`unfold-vertical.svg`、`text-wrap.svg`、`arrow-down-to-line.svg`、`circle-stop.svg`、`copy.svg`、`panel-bottom.svg`、`columns-2.svg`、`rows-3.svg`、`settings.svg`、`pencil.svg`、`plus.svg`、`minus.svg`、`refresh-cw.svg`、`star.svg`、`git-merge.svg`、`git-compare.svg` 等，只承认缺 `wrap-text.svg` | 那句"**可一比一**"是**错的**：`gpui-kit-assets-0.6.6/assets/icons/` 里有 1830 个 SVG，但应用注册的 `gpui_kit::assets::Assets` **只嵌入 `default-icons.txt` 列出的 101 个字形**（`gpui-kit-assets-0.6.6/build.rs:73-91` 用 `default_paths` 过滤生成 `Assets`）。**这 101 个里没有任何 `git-*` / `tag` / `trash` / `bug` / `database` / `pin` / `terminal` 字形**（只有 `github.svg` 与 `square-terminal.svg`）。用不在默认集里的 `IconName` 变体会**编译通过但运行时画空白** | **已修正**：写入 §4 缺口清单第 33 条（含 101 个完整字形名列表与替代建议），并按 `UI-MAP.md` §1.1.5 的规则要求登记到 `docs/DESIGN.md` 图标映射 |
| 🔧2 | `research/windows/04-theme-and-components.md` §1.4 末尾：「卡片/对话框/浮层用 `rounded-xl`(=12px，**Tailwind 默认**)；菜单/列表项用 `rounded-md`(=6px) / `rounded-lg`(=8px) / `rounded-sm`(=4px)」 | 本仓库把 Tailwind 整条圆角阶梯改写成了 `calc(var(--radius) × k)`（`theme.css:6-12`，`--radius: 8px`）：`rounded-sm` = **4.8**、`rounded-md` = **6.4**、`rounded-lg` = **8**、`rounded-xl` = **11.2**、`rounded-2xl` = **14.4** | `rounded-xl` 是 **11.2px** 不是 12；`rounded-sm` 是 **4.8px** 不是 4；`rounded-md` 是 **6.4px** 不是 6 | **已修正**：在 §1.9「圆角档位」表下加显式警告，表内一律用 `calc(var(--radius) × k)` 的真实值 |

### 8.2 抽样核对通过清单（≥15 条，全部读到了该行）

| # | 核对对象 | 结论 |
| --- | --- | --- |
| ✅1 | `styles/theme.css:118` = `--lithe-title-bar-height: 2.5rem`（→40px） | 一致 |
| ✅2 | `styles/theme.css:119` = `--lithe-footer-height: 1.5rem`（→24px）；`:191` comfortable = `2rem`（→32px） | 一致 |
| ✅3 | `styles/theme.css:120-123` = `2.25rem` / `= pane-header` / `1.75rem` / `12.5rem` | 一致 |
| ✅4 | `styles/theme.css:124-134` = `2rem` / `4px` / `1.5rem` / `1.75rem` / `1rem` / `2px` / `4px` / `6px` / `8px` / `4px` / `8px` | 一致 |
| ✅5 | `styles/theme.css:188-200` comfortable 覆盖块（11 个变量） | 一致 |
| ✅6 | `styles/theme.css:6-12` 圆角派生式 | 一致（并据此修正了 1 条描述，见 🔧2） |
| ✅7 | `features/window/components/project-tab-bar.tsx:41` = `flex h-8 shrink-0 items-center overflow-x-auto border-border border-b bg-surface px-1.5` | 一致（32 / 6） |
| ✅8 | `project-tab-bar.tsx:64` = 含 `h-7 min-w-36 max-w-60 gap-1.5 rounded-sm pr-8 pl-2.5` | 一致（28 / 144 / 240 / 6 / 4.8 / 32 / 10） |
| ✅9 | `main-sidebar.tsx:97` = `export const COLLAPSED_ACTIVITY_RAIL_WIDTH = 38;` | 一致 |
| ✅10 | `main-sidebar.tsx:98-101` = `160` / `140` / `320` / `8` | 一致 |
| ✅11 | `ui/chrome.tsx:6` = `font-sans ui-text-chrome flex shrink-0 items-center text-subtle-foreground`；`:11` 标题栏 region = `h-(--lithe-title-bar-height) … px-(--lithe-chrome-padding-inline)`；`:13` footer region = `h-(--lithe-footer-height) …` | 一致 |
| ✅12 | `ui/tab-bar.tsx:209` = `connected + active` → `before:h-[3px] before:bg-primary` | 一致（3px 强调线） |
| ✅13 | `ui/tab-bar.tsx:244-255` / `:257-267` = TabBarSurface horizontal 36 / TabBarTab 28·80–200 | 一致 |
| ✅14 | `features/git/components/log/git-commit-table.tsx:43` = `const ROW_HEIGHT = 30;`；`:116-117` = `estimateSize: () => ROW_HEIGHT` / `overscan: 14` | 一致 |
| ✅15 | `git-commit-table.tsx:257-258` = `w-28`（作者=112）/ `w-32`（日期=128）；`:321,324` 同值 | 一致 |
| ✅16 | `features/layout/components/bottom-pane/bottom-pane.tsx:47` = `const [height, setHeight] = useState(320);` | 一致 |
| ✅17 | `extensions/themes/builtin/lithe.json:12-27`（light）/ `:79-94`（dark）全部 39+ 键行号 | 一致 |
| ✅18 | `extensions/themes/theme-file.ts:5-15`（9 个必填键）/ `:17-27`（9 条旧键重映射） | 一致 |
| ✅19 | `i18n/locale.ts:4605-4608`（`workbench.project`=项目 / `changes`=更改 / `search`=搜索 / `searchInFiles`=在文件中搜索...）、`:4628`（`ui.retry`=重试）、`:4630`（`ui.noActionsHere`=此处无任何内容） | 一致 |
| ✅20 | `i18n/locale.ts:7305-7321`（`footer.*` 全部 17 键，含 `footer.changes` 与 `footer.change` 同文案） | 一致 |
| ✅21 | `features/settings/config/default-settings.ts:138`（`sidebarWidth: 320`）、`:139`（`rightToolWindowWidth: 400`）、`:131-132`（`activityRailExpanded: false` / `activityRailWidth: 180`）、`:182`（`fileTreeIndentSize: 16`）、`:97`（`windowChromeDensity: "focused"`）、`:105`（`compactMenuBar: true`） | 一致 |
| ✅22 | `features/global-search/components/global-search-toolbar.tsx:101` = 含 `h-7 … rounded-lg … px-2`；`:102` = `size-4` | 一致（28 / 圆角 8 / 8 / 16） |
| ✅23 | `features/settings/lib/ui-preferences.ts:9`（`data-window-chrome-density`） | 一致 |
| ✅24 | **gpui 侧**：`gpui-component-0.6.6/src/title_bar.rs:15` = `pub const TITLE_BAR_HEIGHT: Pixels = px(34.);`；`:42` `pub struct TitleBar`；`:50,59,81` `new` / `title_bar_options` / `window_options` | 一致 |
| ✅25 | **gpui 侧**：`gpui-component-0.6.6/src/sizing.rs` `Size` 枚举（`Size/XSmall/Small/[default]Medium/Large`）与 `table_row_height()` = `26/30/32/40` | 一致（提交表 30 = `Size::Small`） |
| ✅26 | **gpui 侧**：`gpui-component-0.6.6/src/theme/mod.rs:250` `pub fn set_scrollbar_mode`；`:367` `sync_base`；`:399` `semantic_tokens`；`:471` `radius_tokens`；`:482` `spacing_tokens`；`:486` `typography_tokens` | 一致 |
| ✅27 | **gpui 侧**：`gpui-component-0.6.6/src/dock/panel.rs:137` `fn inner_padding`；`:146` `fn title_bar`（`UI-MAP.md` 的 `:131-148` 范围正确） | 一致 |
| ✅28 | **gpui 侧**：`gpui-component-0.6.6/src/command/command.rs:72,100,112,130,157,177,183,195,204`（`new` / `group` / `searchable` / `on_query` / `on_confirm` / `placeholder` / `empty` / `max_h` / `bordered`） | 一致 |
| ✅29 | **gpui 侧**：`gpui-base-0.6.6/src/resizable/mod.rs:14,17,22,27,233`（`PANEL_MIN_SIZE` / `h_resizable` / `v_resizable` / `resizable_panel` / `reset_panel`）；`resizable/panel.rs:275,283`（`size` / `size_range`） | 一致 |
| ✅30 | **gpui 侧**：`gpui-kit-assets-0.6.6/src/lib.rs:8,36`（`pub use icon::{IconName, IconNamed};` / `pub use native_assets::{AllAssets, Assets};`）；`build.rs:73-91`（`default_paths` 过滤生成 `Assets`）；`default-icons.txt` = **101 行** | 一致（并据此产出 🔧1 的修正） |
| ✅31 | **gpui 侧**：`gpui-component-0.6.6/src/sizing.rs:57`（`pub fn table_row_height`）、`:69`（`pub fn table_cell_padding`） | `table_row_height` 一致；⚠️ **`table_cell_padding` 也未在 research 引用的 `:69-96` 之外被核对**；`input_h` / `list_py` / `list_px` **未在 `sizing.rs` 中找到同名 `pub fn`** → research 04 §2.3/§2.7 与 06 §1.0 引用的 `sizing.rs:261-269` / `:272-292` **未能复现，列为待核实**（不影响本文正文，正文只用 `table_row_height` 与 `Size` 枚举） |

### 8.3 本次核对里**未能复现**的引用（需后续单独核实，不在正文中当结论用）

| # | research 的引用 | 本次核对结果 | 影响 |
| --- | --- | --- | --- |
| ⚠️1 | `research/windows/04` §2.3 / §2.7 / §3.3：`gpui-component-0.6.6/src/sizing.rs:261-269`（`input_h` → XSmall 20 / Small 24 / Medium 32 / Large 44）、`:272-292`（`list_px` / `list_py`） | `sizing.rs` 中的 `pub fn` 只有 `table_row_height`（`:57`）与 `table_cell_padding`（`:69`）；**未找到 `pub fn input_h` / `list_px` / `list_py`**（可能在 `Sizable` 的宏展开或别的模块）→ 行号与实际实现位置**未能复现** | 本文正文**没有**用这三项做结论（只用 `Size` 枚举 + `table_row_height`），但 §1.9 的"控件高度档"目前是**用 Windows 侧的 px 值**表述的，不依赖 gpui 侧行号 |
| ⚠️2 | `research/windows/06` §6.0：`gpui-component-0.6.6/src/table/state.rs:36-40` 的私有 `enum SelectionMode` | 未核对 | 结论（`TableState` 只有单选）不受影响，已被 `table/state.rs:240-245,458,463,529,548` 支持 |
