# Int UI Kit / Int UI Icons 设计基线

本文是 JetBrains 官方 Figma Community 库的总览参考，用来在设计评审、UI 讨论和问题单里精确引用某个组件、design token 或图标名称。

- 本文只记录**已经从源文件读取到**的内容。没有读取的项统一列在文末「未覆盖范围」，不要在别处凭印象补全。
- 具体像素值、颜色和变体数量会随上游库更新而变化；发生分歧时以 Figma 源文件为准，并回来更新本文。
- 本文不是实现指南，不规定 Lithe 侧如何使用这些 token。

## 1. 源文件

| 文件 | fileKey | 页面（nodeId） | 内容 |
| --- | --- | --- | --- |
| Int UI Kit (Community) | `FEAVhuAJ1Pa7R2HfsllRHY` | `Getting started` `6222:73687`、`Components` `6204:73431` | 颜色、字体、间距，以及全套 IDE 组件 |
| Int UI Icons (Community) | `TbXho57a8mA80b5vE0oTxk` | `Icons` `955:20814`、`Modifiers` `7758:48731` | 产品图标，以及叠加在图标上的修饰标记 |

- Int UI Kit：<https://www.figma.com/design/FEAVhuAJ1Pa7R2HfsllRHY/Int-UI-Kit--Community-?node-id=6222-73687>
- Int UI Icons：<https://www.figma.com/design/TbXho57a8mA80b5vE0oTxk/Int-UI-Icons--Community-?node-id=955-20814>

两个文件都是 JetBrains 发布到 Figma Community 的副本。要在一个新文件里使用其中的组件，必须先按 §2.1 的三步把它变成自己的本地库。

## 2. Int UI Kit

### 2.1 Getting started 页

该页由三个 section 组成。

**How to use Figma library**（`8151:76094`）——三步接入流程：

1. 从 Figma Community 取得库文件副本。
2. 把副本 publish 成自己的本地库。
3. 在新文件或已有文件中启用该库。

页面同时给出三条使用说明：

- 从左侧面板的 Assets 标签把组件拖到画布，或直接从库文件复制粘贴。
- 改状态时选中实例，用右侧属性面板切换。组件有哪些属性，看该组件所在页；页面上以 Tree node 为例。
- 复杂组件（例如 popup）需要先 detach 实例再编辑。

该 section 还附了三个外链入口：IntelliJ Platform UI Guidelines、JetBrains Mono font、Report an issue。

**Design basics**（`8063:76782`）——四张 tiles，分别概括 Colors、Typography、Spacing、Icons。

**Components**（`8028:90897`）——30 张预览 tile，作为完整组件库的入口索引：

Alert、Banner、Button、Checkbox、Combobox、Context menu、Dialog、Dropdown、Group header、Input field、Link、Loader、Notification、Popup、Progress bar、Radio button、Segmented control、Search、Scrollbar、Shortcut and code sample、Tab、Table、Toggle、Toolbar、Tooltip、Tooltip Editor、Tooltip Got It、Tree、Tool window、Main window。

### 2.2 Design token 摘要

#### 调色板

Colors tile 标注共 **15 个色系**。token 命名规律是 `<主题>/<色系>/<色系> <阶数>`，其中第 4 阶是该色系的主色，命名里带 `(Primary)`。

Light 主题下 6 个强调主色（tile 上直接展示的六色）：

| Token | 色值 |
| --- | --- |
| `Light/Blue/Blue 4 (Primary)` | `#3574F0` |
| `Light/Green/Green 4 (Primary)` | `#208A3C` |
| `Light/Red/Red 4 (Primary)` | `#DB3B4B` |
| `Light/Orange/Orange 4 (Primary)` | `#E56D17` |
| `Light/Yellow/Yellow 4 (Primary)` | `#FFAF0F` |
| `Light/Purple/Purple 4 (Primary)` | `#834DF0` |

Light 主题下已验证的灰阶与白色：

| Token | 色值 | 用途 |
| --- | --- | --- |
| `Light/Gray/Gray 1 (Primary)` | `#000000` | 主文本 |
| `Light/Gray/Gray 7` | `#818594` | 次级文本（tile 标题） |
| `Light/Gray/Gray 11` | `#DFE1E5` | 分隔线、边框 |
| `Light/Gray/Gray 14 (White)` | `#FFFFFF` | 背景 |

Dark 主题下已验证的灰阶：

| Token | 色值 | 用途 |
| --- | --- | --- |
| `Dark/Gray/Gray 1 (Primary)` | `#1E1F22` | 窗口背景 |
| `Dark/Gray/Gray 2` | `#2B2D30` | 面板、内容块背景 |
| `Dark/Gray/Gray 7` | `#6F737A` | 次级文本 |
| `Dark/Gray/Gray 12` | `#DFE1E5` | 主文本 |

其他在源文件中确认过的色值：`Light/Teal/Teal 4 (Primary)` = `#039BA1`（间距标注色）、`Light/Teal/Teal 9` = `#F2FCFC`（选中行底色）、`Light (Current)/Green/Green 6 (Primary, Stroke)` = `#5FB865`。

#### 字体

字体族为 **Inter**。Design basics 的 Typography tile 定义四个文本样式，行高统一 16：

| Tile 上的名称 | 字号 | 字重 | 行高 | 已确认的 style 名 |
| --- | --- | --- | --- | --- |
| Default | 13 | Medium (500) | 16 | `UI/Default` |
| Default semibold | 13 | Semi Bold (600) | 16 | `UI/Default semibold` |
| Medium | 12 | Medium (500) | 16 | 未确认 |
| Medium semibold | 12 | Semi Bold (600) | 16 | `UI/Medium semibold` |

12px Medium 一行的文字样式名没有从源文件绑定关系里读到，使用前请在 Figma 里按行确认。

图标内部需要字母造型（如类、接口图标）时，规范要求使用 **JetBrains Sans**，而不是 Inter。

#### 间距

Spacing tile 以复选框列表为例展示基准节奏：

- 列表项垂直内边距 **8px**。
- 复选框与文本之间的水平间距 **6px**。
- 相邻控件之间的垂直间距 **8px**。

即 8px 为基准步长，紧凑处退到 6px。

### 2.3 组件清单（Components 页）

Components 页共 **32 个 section、132 个组件 frame**。下表按 section 列出全部组件及其 nodeId，方便精确跳转。

| Section | nodeId | 组件 |
| --- | --- | --- |
| Buttons | `6285:70852` | Checkbox `3590:81635`、Link `46:11074`、Radio Button `3894:82032`、Toggle `5168:84066`、Slim Button `6517:82733`、Help `7070:146182`、Button `146:52009`、\_Button / Split `7223:45238`、Segmented Control `7281:46902`、\_Segmented Button `7281:46889`、\_Checkbox `7936:67406` |
| Hint | `8006:72903` | Hint `8006:72912` |
| Tree | `6285:70853` | Tree / Node `3873:80233`、Tree `6285:70854`、\_Tree / Selection `6336:75927` |
| Toolbar | `6285:72286` | Toolbar `1146:54334`、Toolbar / Icon `5701:76416`、Toolbar / Separator `5701:76320`、Toolbar / Button `5701:76161`、Toolbar / Dropdown `9393:66721`、\_Inline / State `6555:83459`、\_Toolbar / State `6418:73915`、Toolbar / Search `9578:67127` |
| Notification | `6296:70979` | Notification `3595:83696`、\_Notification / State `6289:70789`、\_Notification / Type `6334:72067`、Collapsible `7880:81066`、Notification stack `461:98590` |
| Alert | `6334:73518` | Alert `3345:82978`、Alert / Type `6334:73527` |
| Tooltip | `6350:72030` | Tooltip / Got It `56:8741`、Tooltip / Tooltip `5388:81364`、Tooltip / Help `7191:45452`、Tooltip / Editor `71:9760`、\_Corner `8097:78250`、Tooltip / Validation `10134:67704` |
| Input | `6353:72296` | Search `7070:146048`、\_Text `7141:46623`、\_Close `7141:46785`、Input Field `7399:46601`、Dropdown `7944:74565`、Combobox `7944:74746`、Labelled Input `7399:47653`、\_Input / Type `7399:46697` |
| Tabs | `6353:72297` | Tab `5664:79158`、Tabs `5664:81901`、\_Tab / Background `6353:72345` |
| Banner | `6353:75026` | Banner `3346:81594` |
| Popup | `6435:72849` | Popup / Cell / Regular `3815:85955`、Popup / Cell / Selection `6437:72852`、Popup / Cell / Advanced `6501:80644`、Popup / Cell / Search `6504:83612`、\_Inline action / right side `4442:82590`、\_Inline action / button `4353:82987`、Popup / Line with actions `4450:82431` |
| Shortcut & Code Sample | `6551:84219` | Code Sample `3711:79004`、Shortcut `6551:84195` |
| Details | `6553:82751` | Scrollbar `6222:73552`、Loader `5767:82632` |
| Inline Search | `6565:87258` | Inline Search `6565:86496` |
| Tabs / Editor | `7054:86374` | Editor / Tab `7050:85603`、Editor / Tab / Close `7054:85952`、Editor / Tabs `7054:86162` |
| Status Bar | `7059:86519` | Status Bar / Widget `544:50314`、Status Bar / Breadcrumb `544:50321`、Status bar `544:47863` |
| Editor Code | `7059:95496` | Editor Code / Gutter `4413:84474`、Editor Code / Snippet `7059:95513`、Editor Code / Inspection Widget `7059:95520`、Editor Code `7059:95559` |
| Main toolbar | `7059:96675` | Main Toolbar `4358:90449`、Main Toolbar / MacOS Buttons `217:51269`、Main Toolbar / Windows Buttons `335:37366`、Main Toolbar / Win Button `1531:58996`、Main Toolbar / Dropdown `217:55078`、Main Toolbar / Project Dropdown `11351:34406`、Main Toolbar / Icon `217:55666`、\_Split Button Dropdown `4970:88924`、Main Toolbar / Split Button `4970:88978`、Project Icon `11348:22441`、\_Main Toolbar / Gradient `11351:33865` |
| Stripes | `7059:97083` | Stripes / Button `217:58591`、Vertical Stripe `269:25319` |
| Tool Window | `7070:86382` | Header `605:50202`、Tool window `217:51356`、Tool window / Project `605:45150`、Tool window / Terminal `605:63976` |
| Main Window | `7070:138478` | Main Window `7070:140132` |
| Dialog | `7070:146076` | Dialog / Header `7070:146218`、Dialog / Footer `7070:146234`、Dialog `7070:146271`、Dialog / Group `7072:91227`、\_State `8006:79771` |
| Table | `7278:47259` | \_Table / Header `7278:46382`、\_Table / Cell `7278:46388`、Table `7278:47114`、\_Table / Toolbar `8259:115125` |
| Popup / Other | `7884:50972` | Popup / Context Menu `7040:84852`、Popup / Run Options `6444:74430` |
| Popup / Main Toolbar | `8043:83916` | Popup / Projects `6445:76524`、Popup / Branches `6451:81293`、Popup / Run Widget / Expanded `6451:84177`、Popup / Run Widget / Collapsed `6451:85666`、Popup / Run Widget / More Actions `6451:86604`、Popup / Run Widget / Stop `6451:88828` |
| Popup / Action Popups | `8043:84054` | Popup / Generate `6444:75279`、Popup / Run `6441:73645`、Popup / VCS Operations `6441:72876`、Popup / Run Anything `6515:83461`、Popup / Breakpoint `6444:75980` |
| Popup / Editor Popups | `8043:85254` | Popup / Completion `6515:85341`、Popup / Quick Fixes `6515:90136`、Popup / Problems `6551:83679`、Popup / Documentation `6515:89894`、Popup / Structure `6515:83331` |
| Popup / Navigation Popups | `8043:85257` | Popup / Search Everywhere `6515:81937`、Popup / Recent Files `6515:81194`、Popup / Tool Windows / More `6451:83424`、Popup / Find in Files `6515:93124`、Popup / Show Usages `6515:80621` |
| Cursors | `8303:75289` | Cursors `8303:75291`（Arrow、Pointing hand、Open hand、Grabbing Hand、Resize、Text） |
| Progress Bar | `9838:68797` | Progress Bar `7059:86470`、\_Indeterminate `9811:67134`、\_Progress `9838:67943` |
| Run Widget | `11351:33914` | Run Widget / Configurations `6701:85332`、Run Widget / Run Button `6701:85384`、Run Widget / Simple Button `6701:85766`、Run Widget / Debug Button `6701:85626`、Run Widget / Stop Button `6701:85528`、Run Widget `6702:85839` |
| Dialog / Examples | `11484:23324` | Dialog / Settings `7072:91657`、Dialog / Welcome Screen `11490:43848` |

以 `_` 开头的组件是内部构件的公有化身（例如 `_Checkbox`、`_Table / Cell`），组合完整结构时直接引用它们；`_Toolbar / State`、`_Notification / State` 这类是状态矩阵。

### 2.4 变体（variant）命名约定

组件属性名和取值都直接出现在变体名里，可以用它反推组件支持哪些状态：

- `Theme=Light` / `Theme=Dark`，另有 `Theme=Light with Light Header`（浅色内容配浅色标题栏）。
- `State=Active` / `Hovered` / `Pressed` / `Focused` / `Disabled` / `Invalid`。
- `Type=`：按组件而异，例如 Button 用 `Primary` / `Secondary`，Link 用 `Default` / `Dropdown` / `External`，Checkbox 用 `Unchecked` / `Checked` / `Indeterminate`，Progress Bar 用 `Single Line` / `Multiline`，Tool window Header 用 `Tabs` / `Label` / `Label + Separator`。
- 组合维度：Tree / Node 用 `Level=1..7` + `State=Collapsed|Expanded`；Radio Button 用 `Selected=On|Off`；Toolbar / Icon 用 `State=Default|Selected|Inactive` + `Hovered=On|Off` + `Badge=On|Off`；Dropdown 用 `Hovered`、`Pressed`、`Selected`；Run Widget 用 `Hover`、`Running`、`Rerun`、`Restart`、`Multiple`；Progress Bar 用 `Progress=Regular|Indeterminate`、`Animation=1|2`；Vertical Stripe 用 `Side=Left|Right`。

## 3. Int UI Icons

### 3.1 Icons 页分区

Icons 页（`955:20814`）按产品与功能域分成 **11 个 section，共 2252 个图标组**。

| Section | nodeId | 图标组数 | 典型图标名 |
| --- | --- | --- | --- |
| Plugins | `5726:49148` | 752 | `abstractBean`、`factoryMethodBean`、`infrastructureBean`、`requestMapping`、`springBoot` |
| General | `5726:49149` | 329 | `toolWindow*`、`feedback`、`graphQLToolWindow` |
| Run, Debug, Terminal, Build | `5726:49150` | 138 | `breakpoints / breakpoint`、`run / *`、`build / *` |
| Editor | `5726:49151` | 73 | `gutter/run`、`gutter/javadocRead`、`dataSchema` |
| Files & Nodes | `5726:49152` | 201 | `config`、`include`、`template`、`webResources`、`localScope` |
| VCS | `5727:49147` | 60 | `applyNotConflicts`、`compare3*`、`compare4*`、`arrowLeftRight` |
| Database | `5726:49153` | 203 | 58 个 `providers/*` 数据源图标，如 `providers/oracle`、`providers/postgresql`、`providers/mysql`、`providers/h2` |
| Learn | `5726:49154` | 63 | `checkmark`、`resetLesson`、`ide-features-trainer / like` |
| Guidelines | `5727:49148` | 4 | `guides`、`guides@20x20`、`dropdown`、`dropdownGutter` |
| IDEs | `8376:15602` | 428 | `css selector / id`、`css selector / xpath`、各产品图标 |
| Misc | `10406:10834` | 1 | `IntelliJRepository` |

命名使用空格和斜杠分组（如 `css selector / tagWithClasses`、`breakpoints / breakpointMuted`），在 Figma 之外引用时按原样保留。

### 3.2 尺寸、变体与命名

图标有三档尺寸，通过名字后缀区分：

| 尺寸 | 命名 | 使用场景 |
| --- | --- | --- |
| 20 × 20 | `name@20x20` / `name@20x20_dark` | 主工具栏、工具窗口 |
| 16 × 16 | `name` / `name_dark` | 绝大多数 UI 元素的默认尺寸（工具栏、树、菜单等） |
| 14 × 14 | `name@14x14` / `name@14x14_dark` | Gutter、状态栏 |

无后缀的是默认 16 × 16。每个图标在 Figma 里是一组变体，变体名就是 `Theme = Light` / `Theme = Dark`。

源文件当前统计：图标组（定义 frame）共 **2252 个**，其中 2100 个是默认 16 × 16、121 个带 `@14x14`、31 个带 `@20x20`；按尺寸后缀归并后是 **2023 个不同的图标名**。Light 变体 2565 个、Dark 变体 2564 个（同一图标在不同画板重复定义会让变体数高于图标组数）。

Guidelines 里还给出了检索语法示例：在 Figma 资源搜索里按 `尺寸缩写 + 颜色 + 风格` 逐段输入，例如 `li gree gen` 命中 light / green / general，`da bl sol` 命中 dark / blue / solid，`li ye out` 命中 light / yellow / outline。

### 3.3 绘制规范

Guidelines（`1023:22495`）里的通用要求：

- 优先使用轮廓（outline）图标，风格扁平。
- 图标细节保持在能传达含义的最低限度。
- 最小步进 0.5px。
- 内圆角：大对象 2px，小对象 1px。
- 检查图标不能有多余锚点。
- 图标内需要字母造型时使用 JetBrains Sans。
- 勾选「Clip content」以确认图标尺寸正确。

分尺寸的笔触与圆角要求：

| 尺寸 | 笔触 | 圆角 | 其他 |
| --- | --- | --- | --- |
| 20 × 20 | 1.5px | 2px | 较大的形态使用内描边；用圆头端点；保持连续笔触，除非断开能提升辨识度 |
| 16 × 16 | 1px | 2px | 较大的形态使用内描边；同一图标内有多个对象时，用笔触断口分隔 |
| 14 × 14 | 1px | 1px | — |

20 × 20 的画板内叠加 14px / 16px / 18px 三档参考线，并标注 Padding 1px。

### 3.4 颜色

Guidelines 明确说明：**图标的正确颜色要到 Int UI Kit community 文件里按 color style 名去查**，例如 `Light (Current)/Green/Green 6 (Primary, Stroke)`。也就是说图标本身不携带颜色语义，颜色由引用它的组件按主题决定。

### 3.5 导出流程

Guidelines 记录了官方导出方式，基于第三方 Figma 插件 **JB Variant Exporter**：

1. 从插件仓库下载并导入开发插件（Figma → Plugins → Development → Import plugin from manifest，选择 `manifest.json`）。
2. 把成对的组件 `iconName` 与 `iconName_dark` 合并成变体：选中组件或整个画板，执行 `Plugins → Development → JB Variant Exporter → Combine as Variants`。没有配对的对象会被忽略。
3. 选中成对的变体，执行 `Plugins → Development → JB Variant Exporter → Export Selected`，选择导出目录，得到 `Exported Variants.zip`。同样只导出成对的变体。

### 3.6 Modifiers 页

Modifiers 页（`7758:48731`）提供 **126 个不同的 `mod / *` 修饰图标**。它们是叠加在基础图标右下角的小标记，用来表达语言、构建工具或状态附加信息，例如 `mod / java`、`mod / kotlin`、`mod / gradle`、`mod / maven`、`mod / rust`、`mod / python`、`mod / debug`、`mod / failed`、`mod / warning`、`mod / vcs`。

该页同时用实例排布展示修饰图标与基础图标的组合方式，例如 `abstractClassKotlin`、`junitTestMark`、`springSecurity` 这类由基础图标加修饰组成的完整图标。

## 4. 如何定位到具体节点

把 §2.3 和 §3.1 表里的 nodeId 换成 URL 里的短横线形式，即可直接打开对应画板：

```text
https://www.figma.com/design/<fileKey>/<fileName>?node-id=<a>-<b>
```

例如 `Tree / Node`（`3873:80233`）：

```text
https://www.figma.com/design/FEAVhuAJ1Pa7R2HfsllRHY/Int-UI-Kit--Community-?node-id=3873-80233
```

在问题单或 PR 里引用具体组件时，建议同时写出组件名和这个链接，避免只写「那个下拉框」这类无法定位的描述。

## 5. 未覆盖范围

以下内容本次**没有**从源文件中提取，需要时按 §4 打开对应节点自行确认：

- 完整的颜色/文字/效果样式清单。本文只收录了读取过程中直接验证过的 token；15 个色系 × 各阶 × Light/Dark 的完整色阶表尚未导出。
- 每个组件的内部结构与精确尺寸（padding、圆角、高度）。本文只给出了变体维度和 §2.2 中验证过的间距节奏。
- 各组件的交互行为与动效参数。
- Int UI Kit 的 `Design basics / Icons` tile 实际引用的是 Int UI Icons 中的哪些图标。
- 两个库的版本号与更新历史。

## 6. 配套原型

[prototype/index.html](prototype/index.html) 是按本文数据还原的界面原型，从索引页进入，也可以直接打开单个页面。无外部依赖、无构建步骤。

| 文件 | 内容 |
| --- | --- |
| `prototype/01-main-window.html` | 主窗口，1536×960 |
| `prototype/02-settings.html` | 设置对话框 891×653 |
| `prototype/03-welcome.html` | 欢迎屏 |
| `prototype/04-search-everywhere.html` | Search Everywhere 650×382 |
| `prototype/05-find-in-files.html` | Find in Files 677×624 |
| `prototype/06-components.html` | 组件画廊 |
| `prototype/assets/` | 共用样式、脚本与字体 |

- 三套主题：Light（深色标题栏 `#27282E`）、Light with Light Header（`#F7F8FA`）、Dark。用底部切换条或 `?theme=` 切换。
- `?bare=1` 会隐藏原型自身的页眉与切换条，画布按 1:1 输出，可直接截图或与设计稿逐像素比对。
- `assets/fonts/` 内含 Inter 与 JetBrains Mono（可变字体，SIL OFL 1.1，许可证原文同目录）。不打包这两款字体会回落到 Segoe UI / Consolas，字形宽度完全不同。

原型是用于对齐视觉与交互密度的一次性产物，不是产品代码，也不规定 Lithe 侧应该如何实现。


