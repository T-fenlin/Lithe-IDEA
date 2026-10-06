# Lithe

Lithe 是一个纯 Rust 的 IDE：`lithe-core` 是确定性的命令面，`lithe-gpui` 是唯一的
GPUI 宿主。本文件是领域术语表：只记录"这个词在我们的语境里指什么"，不记录实现细节。

## Language

### 扩展系统

**扩展（Extension）**:
用户可见的、可按分类浏览的能力包；v1 全部是随宿主内置的能力，v2 契约起支持声明式贡献项。
_Avoid_: 插件（用户可见文案一律用"扩展"）、addon、模块

**Plugin**:
"扩展"在代码与协议层的拼写（Core 的 `plugins` 模块、`pluginAPIVersion`、
`plugins.listCatalog` 命令）。与"扩展"是同一概念的两层叫法：用户说"扩展"，代码写 `plugin`。

**模块（Module）**:
扩展包声明的能力所有权单元（`moduleIDs`）。一个模块 ID 全目录只能被一个扩展包拥有；
语言支持的能力插槽（LSP / 执行 / 测试 / 调试）必须指向自己包拥有的模块。

**目录（Catalog）**:
随宿主一起分发的扩展清单文档（官方目录）。校验、兼容性与分类推导的唯一真源是 Core；
宿主只消费校验后的结果。

**贡献项（Contribution）**:
扩展包声明的一类具体能力：主题、图标主题、字体、调试适配器、语言支持。清单只存声明，
分类由贡献项推导。

**分类（Category）**:
扩展在目录里的浏览分类（`debug` / `fonts` / `icon-themes` / `languages` / `lsp` /
`themes`）。分类**永远从非空贡献项推导**，不在清单里声明——这是 Zed 式做法，
保证分类与贡献项不可能互相矛盾。

### 主题与字体（随宿主分发的能力包）

**内置主题（Built-in theme）**:
编译进宿主、**不可卸载**的主题组（v1 只有 IDEA 一组：IntelliJ Light + Darcula，也是默认主题）。
清单入口点是 `builtIn`。

**内置字体（Built-in font）**:
编译进宿主、**不可卸载**的字体族。v1 是 IDEA 的默认代码字体 JetBrains Mono，
也是本产品的默认代码字体（`DEFAULT_MONO_FONT_FAMILY`）。界面字体仍用系统 UI 字体。

**捆绑扩展（Bundled extension）**:
随宿主分发、**用户可以卸载**的能力包。v1 有五种主题组（Gruvbox / Lithe / Nord / One /
VS Code）与两个字体族（Source Code Pro / IBM Plex Mono）。清单入口点是 `bundled`；
卸载是宿主侧的文件生命周期 + 设置键 `uninstalledExtensions`，不是目录事实。

**装载面与扩展面（load surface / extension surface）**:
主题只有一个装载目录（`<配置目录>/themes/`，注册表只能监视一个目录），而扩展包住在
`<配置目录>/extensions/<种类>/`。扩展面是**管理面**（装/卸都发生在这里），
装载面是**镜像**（启动时与扩展面对齐）。所以改一个捆绑主题要改扩展面那份。
字体**没有**装载面（文本系统由我们启动时喂字节）。
_Avoid_: 把两个目录说成"双目录加载"（做不到，见 `settings/src/theme.rs` 模块文档）

**图标集（Icon set）**:
文件与文件夹的图标（一套一包，`assets/icons/<包>/extension.json`）。
**一个词贯穿全部层**：界面标签「图标」、分类 id `icons`、清单字段 `icons`、
设置键 `icons`、资产目录 `assets/icons/`、Rust 标识符 `icons_*`。
_Avoid_: `icon-theme` / `iconThemes` / `iconTheme` 这些旧拼写（2026-09-29 统一掉了，
留着会让人在两个词之间反复确认）；也注意 `assets/app-icon/` 是**应用图标**
（exe 资源那份），与图标集是两件事**彩色**：图标按 SVG 原色渲染
（`img()` 管线），不是单色遮罩。内置 `idea` 包不可卸载，捆绑的 `pierre` 包可卸载；
同一时刻只有一个生效（设置键 `iconTheme`，扩展页的「使用」切换）。

**生效时机（apply timing）**:
主题卸载/重装**立即**生效（注册表监视主题目录，热重载后重新应用当前主题）；
字体**下次启动**生效（GPUI 的 `TextSystem::add_fonts` 只增不删，运行期摘不掉已注册字族）；
图标集**立即**生效（切换只是换一份查表）。
这两条差异写在扩展页的卡片上，不是实现细节。

### 组件库与界面外壳

**组件库（Component library）**:
本仓库自己的 GPUI 控件库（crate `lithe-gpui`），是 Ely GPUI Components 的
vendored fork（把上游源码拷进本仓库、由我们长期维护）。宿主界面控件的归宿。
_Avoid_: 用 "Ely" 指本仓库这一份；把宿主 crate（`lithe-gpui-app` 等）叫"组件库"

**gpui-kit**:
外部发布的 GPUI 组件包，宿主当前大部分界面控件的来源；方向是逐步换成"组件库"。
_Avoid_: 把 gpui-kit 与"组件库"当同一个东西

**工具窗（Tool window）**:
外壳里左 / 右 / 下三块可开合、可调尺寸的停靠面板：左侧项目 / 搜索 / 更改，
右侧工具窗，底部终端 / Git / 问题 / 运行 / 调试。
_Avoid_: "侧栏"（只指左边那一块）

**Dock**:
组件库里承载"工具窗 + 中央列"的容器控件（`lithe_gpui::layout::dock`）。它管分隔条、
拖动、每边尺寸上下限与开合；外壳只决定每块工具窗"显示什么、开不开"。
_Avoid_: 用 "Dock" 指整个工作台或整个中央列

**主题桥（theme bridge）**:
把宿主 gpui-kit 主题的颜色映射进组件库自己的主题全局的那一层。它是过渡措施，
方向是让组件库主题直接成为真源、这层消失。
_Avoid_: 说成"主题同步"（它只单向覆盖宿主确实有的语义色，不是双向同步）

