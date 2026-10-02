# Agent 笔记：扩展目录 schema v2 与中央扩展页

状态：已实现

## 先说结论

扩展（plugin）系统落地了第一块真实地基：Core 的插件清单升级到 **schema v2**（新增
主题 / 图标主题 / 字体 / 调试适配器的贡献项结构，分类从贡献项**推导**），新增
`plugins.listCatalog` 命令把内置目录交给宿主；gpui 宿主新增 `lithe-gpui-extensions`
crate 和**中央扩展页**（搜索 + 分类筛选 + 卡片展开详情）。右活动栏拼图项的行为从
"打开右栏空态"改为"开/关中央扩展页"，右工具窗只剩通知与 Maven。v1 是**只读**目录：
没有安装、启停、下载，也没有 wasm 加载。

日常开发的影响：往目录里加一个内置扩展 = 改 `shared/fixtures/plugins/official-v2.json`
（一条清单）；升级宿主版本时必须同步 fixture 的 `hostVersion`（有测试钉住）；分类
筛选、术语（用户说"扩展"、代码写 `plugin`）都已在 CONTEXT.md 登记。

## 问题

Core 里一直有一个 `plugins` 模块（清单解析 + 兼容性校验），但它是"零消费者"状态：
官方目录 fixture 是空的、没有任何 Core 命令引用它、UI 只有一个右侧栏空态视图
（"未找到扩展。"）。同时它有三个互相纠缠的缺口：

1. 清单 schema v1 只建模了语言支持，表达不了"主题 / 图标主题 / 字体 / 调试适配器"
   这些用户点名的扩展分类；
2. fixture 的 `hostVersion` 是 `0.3.0`——那是已删除的 Tauri 产品的版本，与现在的
   gpui 宿主（`0.0.1`）对不上，校验注定失败；
3. 右侧栏的扩展入口是一个记录在案的临时偏离：真机（已删除的 Windows/Tauri 产品）
   里它是 `openExtensionsBuffer`——扩展占**中央区**，不是右栏视图。

## 决策

### 1. 契约升级到 schema v2：贡献项全建模，分类靠推导（对齐 Zed）

`PluginPackageManifest` 新增 `themes` / `iconThemes` / `fonts` / `debugAdapters`
四个贡献项数组（原有 `languageSupports` 不动）。**清单里没有 categories 字段**：
分类（`debug` / `fonts` / `icon-themes` / `languages` / `lsp` / `themes`）由 Core
从非空贡献项推导，随 `plugins.listCatalog` 响应一起给出。这是照着 Zed 的
`ExtensionManifest::provides()`（`crates/extension/src/extension_manifest.rs`）做的
——Zed 源码里 `extension.toml` 同样不存 categories，分类是计算出来的。好处是
"分类"和"贡献项"是同一份数据的两种视图，永远不会互相矛盾。

校验规则同步收紧：贡献项数组严格按主键排序（防重复）、路径必须包内相对
（无前导 `/`、无反斜杠、无 `..`）、categories 与贡献项的一致性因此**不需要**再校验。

### 2. 官方目录 fixture = 产品数据，由 Core 内嵌并经 `plugins.listCatalog` 交付

`official-v1.json` 更名为 `official-v2.json`，填入 **5 个真实内置条目**覆盖 6 个
分类：内置主题包（settings 的 7 组主题文件）、JetBrains Mono 字体包（assets 的
4 个字重文件）、IDEA 图标主题（shared 里唯一真正接线的 `icon-themes/idea`）、
Java 语言支持（java crate 的 JDTLS 服务，分类 `languages`+`lsp`）、Java 调试支持
（Core DAP 引擎 + JDT LS 调试 bundle 发现）。条目如实描述宿主现有能力，不编造
"假扩展"。字体那条要说明：字体文件目前在 assets 里**尚未接线**（`assets.rs` 有意
排除），条目声明的是"随产品捆绑"这个事实，不是"运行时已可加载"。

`plugins.listCatalog` 的 payload 是 `{"hostVersion": "..."}`；Core 用
`include_str!` 把 fixture 作为**产品数据**嵌进生产代码，走完整校验后返回规范化
目录。宿主版本常量 `lithe_gpui_shared::HOST_VERSION`（= crate 版本 `0.0.1`）与
fixture 的 `hostVersion` 必须一致，否则 Core 以 `parse_failed` 拒绝——这是"目录
就是随这个宿主构建分发的"这一语义的机械保证。

### 3. 宿主：新 crate `lithe-gpui-extensions` + 中央扩展页

- `lithe-gpui-extensions`（依赖方向 `extensions -> shared`）：目录快照模型 +
  Core 桥 + 搜索/分类过滤纯函数。不渲染、不读时钟，与 `notify` 同一套约束。
- 视图在 `workbench/src/extensions_page.rs`：顶部标题 + 可用数、搜索框、七个分类
  筛选 chips（全部/主题/图标主题/字体/调试/LSP/语言）、卡片列表（名称 + 版本 +
  分类 + "内置"徽标），点击行内展开详情（描述、作者、贡献项）。
- **中央页槽位**：`ShellWorkspace` 新增 `extensions_page` 实体（懒创建、常驻）与
  `extensions_page_open` 开关；中央列在"扩展页 ↔ 编辑器岛"之间二选一渲染——与
  左栏"项目树 ↔ 更改"同一套既有模式，**没有**动 editor crate 的 buffer 模型。
- 行为：拼图项再点一次关闭；从资源管理器或命令面板**打开文件时页面让位回编辑器**；
  会话文档**不**新增字段（遵循"只存今天真的有宿主的字段"）；`--right-view
  extensions` 探针改为打开中央页（`ShellStartup` 独立开关）。
- 右工具窗枚举删除 `Extensions` 变体；`from_rail_index` 改为 1→通知、2→Maven，
  下标 0 由外壳在进入右栏状态机之前拦截。2026-09-27"右栏三项"的组合决策不变，
  变的只是拼图项的行为。

## 后续修订（2026-09-29，同一分支的第二批）

维护者追加的口径：**主题改成 6 组（每组浅色 + 深色一对）、默认 IDEA 主题、除内置主题外
其余都可卸载**。落地成：

1. **主题重组**：仓库里 `lithe-gpui/themes/` 只留 `jetbrains.json`（IDEA 组 = IntelliJ Light
   + Darcula，唯一 `builtIn`、默认主题）；Gruvbox / Nord / One / VS Code 四份搬进
   `lithe-gpui/extensions/themes/`，两份 Lithe 单变体文件（`lithe-dark.json` /
   `lithe-light.json`）合并成一份 `lithe.json`（浅 + 深）。
   ⚠️ 维护者说的是"7 组"，但仓库现状只有 6 个主题族；第 7 组需要从 `themes/legacy-builtin/`
   提拔一个（dracula / ayu / catppuccin …）或新增，**待维护者点名**（`legacy-builtin/` 的
   10 份是按旧 schema 归档的，不能直接启用）。
2. **契约加 `bundled` 入口点**：`builtIn` = 宿主核心、不可卸载；`bundled` = 随宿主分发、
   用户可卸载（形状同 builtIn，只认 `targetName`）。目录 fixture 因此是 10 条：
   IDEA 主题（`themes`，builtIn）+ 5 个主题组（`themes`，bundled）+ 图标主题 + 字体 +
   Java 语言支持 + Java 调试（均 builtIn）。
3. **卸载是宿主侧生命周期，不进契约**：宿主把包文件放在
   `<配置目录>/extensions/themes/`，卸载 = 删文件 + 在设置键 `uninstalledExtensions`
   记一笔（**防止下次启动同步又装回来**，同步是"缺就补"的种子语义）；重装 = 从内嵌内容
   写回 + 从清单移除。设置键是本侧新增（真源没有卸载概念）。
4. **两个目录的分工（必须有，因为注册表只能监视一个目录）**：扩展目录是**管理面**，
   `themes/` 是**装载面**（镜像，以管理面为准）。启动顺序：播种内置 → 迁移老版本播种在
   `themes/` 里的捆绑文件（搬进扩展目录；Lithe 拆分文件删除）→ 按卸载清单同步两个目录
   → `watch_dir(themes/)`。
5. **默认主题**：`theme` → `darcula`、`autoThemeLight` → `intellij-light`、
   `autoThemeDark` → `darcula`。理由：默认值必须指向一个**卸不掉**的主题。
   真源 `default-settings.ts:99-103` 的 `lithe-dark` / `lithe-light` 是重组前的品牌默认，
   这是有意的偏离。
6. **扩展页的卸载/重装**：`bundled` 卡片带一个 outline 按钮（卸载 ⇄ 重新安装）与
   「已卸载」徽标；`builtIn` 卡片只有「内置」徽标。按钮 `stop_propagation`，否则点它
   会连带展开整行。卸载的是当前主题时，既有的"主题缺失 → 回落 `DEFAULT_THEME`"规则
   自动兜底（设置文件里的选择不抹掉，重装后回来）。
7. **主题索引的取舍**：捆绑扩展**不进**编译期索引（装没装看装载面的镜像），
   所以卸载后下拉里立刻不再出现它们的主题；内置那份仍做编译期兜底。

### 第三批：字体照同一套形式（同日，维护者追加）

维护者口径："字体也弄成扩展的形式，默认当前 IDEA 这个字体，其他字体弄成扩展的形式，
跟主题一样，字体的弄一两个就够了"。落地成：

1. **内置字体 = IDEA 的默认代码字体 JetBrains Mono**（`assets/fonts/` 四个字重，
   `include_bytes!` 进二进制，entrypoint `builtIn`）。`mono_font_family` 的默认值改成
   `DEFAULT_MONO_FONT_FAMILY = "JetBrains Mono"` —— 默认值指向的字族**必须真实可用**，
   否则会落到"字族不存在"的 panic 路径（`theme::apply_font_families` 的文档）。
   界面字体仍默认空串（系统 UI 字体）：等宽字体不适合界面正文，IDEA 自己也这么分。
2. **两个捆绑字体扩展**（可卸载）：Source Code Pro（Adobe）与 IBM Plex Mono（IBM），
   都是 OFL-1.1、各带常规 + 粗体与**随包许可文件**。落在
   `lithe-gpui/extensions/fonts/<包>/`，运行期写到 `<配置目录>/extensions/fonts/<包>/`。
3. **统一了装/卸机制**（新增 `settings/src/extensions.rs`）：把原来主题专用的
   同步/卸载/重装抽成"捆绑扩展"的通用实现（注册表
   `BUNDLED_EXTENSIONS`，`BundledKind::{Theme, Font}`），主题与字体共用一套；
   主题特有的事（内置播种、老文件迁移、装载面镜像）留在 `theme.rs`。
4. **字体为什么不需要装载面镜像**：主题走 gpui-kit 的 `ThemeRegistry::watch_dir`
   （只认一个目录），字体走 `TextSystem::add_fonts`（我们自己喂字节）——
   所以字体的落点就是它唯一的位置。新增 `settings/src/fonts.rs`：
   `register_fonts` 在启动时注册内置字节 + 从管理面目录读已安装的捆绑字体。
5. **启动顺序（硬要求，已写进 `main.rs` 注释与 `lib.rs` 启动顺序表）**：
   `init_store` → **`prepare_bundled_extensions`**（迁移 + 同步写出主题/字体包）
   → **`register_fonts`**（读管理面目录）→ `watch_lithe_themes`（装载 + 应用主题）。
   ②必须早于③（同步没跑就一个字族都读不到）；③必须早于第一次主题应用
   （`installed_font_names` 的已装字族表是**进程内缓存一次**的，晚了会把内置的
   JetBrains Mono 判成"没装"而拒绝写入 —— 实机日志 `S1_THEME font_family
   key=monoFontFamily value=JetBrains Mono` 证明这条顺序成立）。
6. **生效时机差异**（写进契约、CONTEXT.md 与扩展页）：主题卸载立即生效（热重载），
   字体要**下次启动**（`add_fonts` 只增不删，运行期摘不掉已注册字族）。
   扩展页在可卸载字体卡片的详情里加了一行 `lithe.extensions.fontRestartHint`。
7. **实机验证发现并修掉一个真实缺陷**：老文件迁移会无条件删除 `themes/` 里与扩展目录
   同名的文件，而同步阶段刚写出的**镜像**正是同名的 —— 于是每次启动都"删镜像 → 再镜像"
   （日志 `prepared created=3 removed=8`）。修法：迁移时跳过"今天仍由注册表管理的包名"，
   并补了回归测试 `migration_leaves_the_load_surface_mirror_alone`；修复后稳态为
   `created=0 kept=7 removed=0`。

### 第四批：图标（分类改名 + 彩色 + 可切换的图标集）

维护者口径："扩展里面的图标主题修改为图标，图标从单色修改为有颜色的，看下 gpui kit 0.7
里面的 svg 有颜色的是怎么弄的，除当前内置图标，再弄一份可卸载的图标扩展"。

1. **彩色怎么来（源码结论）**：gpui 有两条互斥的渲染管线 —— `svg()` 走
   `SvgRenderer::render_alpha_mask`（渲成 **alpha 遮罩**，用单一 `text_color` 上色，
   `text_color` 为 None 时**根本不画**），`img()` 走图片链路
   （`image::guess_format` 认不出 SVG → `SvgRenderer::render_single_frame` →
   `rasterize_tree` 出**彩色 RGBA**）。所以"彩色"是**换管线**，不是参数开关：
   `theme_file_icon` 从 `svg()` 改成 `img(Resource::Embedded(path))`。
   数据侧佐证：搬进来的图标包本来就是彩色的（idea 引用的 103 个 SVG 里 79 个带
   `fill="#…"`，pierre 完整档 57/57）。
   顺带**订正一条过期结论**：旧注释把"img 时灵时不灵"归因于 `Img::request_layout` 需要
   `GlobalElementId`；复核 0.3.7 源码，那个判断只门控**多帧动画的续帧**（img.rs:382），
   图像在 paint 阶段取（:480），与全局 id 无关。
   已知代价：内嵌资源这条路固定按 `scale_factor = 1.0` 栅格化（上游 TODO），
   HiDPI 下比矢量遮罩略软 —— 取舍是"原色优先"。
2. **分类改名**：扩展页筛选条与卡片上的「图标主题」改叫「图标」（新增 gpui-only 键
   `extensions.icons`）；卡片不再显示原始分类 id，改走本地化标签。
3. **可卸载 + 可切换的图标集（pierre）**：
   - 数据来源与主题/字体不同：图标集**随宿主编译**（`assets/icon-themes/<包>/`，
     pierre 从 assets.rs 的 `#[exclude]` 里放出来），所以它**没有落盘包文件**；
     卸载的语义因此是"不可选 + 正在使用时回落内置包"，而不是删文件（契约里写明了这条差别）。
   - 新增设置键 `iconTheme`（默认 `idea` = 包名；真机存清单 id，本侧存目录名，
     于是"设置值 → 资源路径"是一次字符串拼接，不需要 id 映射表）。
   - `shared::icons::file_icon::set_active_icon_theme(package, asset_source)`：
     从**资源源**读 `icon-themes/<包>/extension.json` —— 内置包与捆绑包走同一条路径，
     没有"两套加载方式"。设置存储（依赖方向 settings → shared）在应用主题时推一次，
     卸载兜底也在那里（与配色主题"当前主题被卸载 → 回落默认"同一条口径）。
   - 扩展页给图标集卡片加「使用 / 当前」（复用真源既有键），切换**立即**生效。

### 第五批：图标相关命名统一成 `icons` 一个词

维护者口径："就算是用户看不到也统一一下，不然修改代码都不好改"。

此前同一件事有四个拼写：分类 id `icon-themes`、清单字段 `iconThemes`、设置键
`iconTheme`、资产目录 `icon-themes/`（界面标签已经叫「图标」）。检索与改动时要在
四个词之间反复确认，所以一次统一成 **`icons`**：

| 层 | 之前 | 现在 |
| --- | --- | --- |
| 分类 id | `icon-themes` | `icons` |
| 清单字段 | `iconThemes` | `icons` |
| 设置键 | `iconTheme` | `icons` |
| 资产目录 | `assets/icon-themes/` | `assets/icons/` |
| Rust 标识符 | `icon_theme_*` / `IconTheme*` | `icons_*` / `Icons*` |
| 扩展 id | `lithe.ideaIconTheme` / `lithe.pierreIconTheme` | `lithe.ideaIcons` / `lithe.pierreIcons` |

**顺带腾了一次名字**：原来的 `assets/icons/` 装的是**应用图标**（`icon.ico` /
`icon.icns` / 各尺寸 png，给 exe 资源与任务栏用），与"图标集"是两件事，却占着
`icons` 这个最合适的名字。所以把它改成 `assets/app-icon/`（引用点只有 `build.rs`、
`lithe.rc` 与 `assets.rs` 的探针，三处），`icons/` 让给图标集。
`assets.rs` 的内嵌分组断言与字节总数断言同步更新（pierre 149 个文件随之进入二进制，
这是"可切换"的必然代价 —— 内嵌总数 269 → 418）。

### 第六批：语法高亮扩到工程文件，并补上 XML

维护者问："语言扩展主要是语言高亮，gpui-kit 内置了，扩展的形式该怎么弄？Zed 的语言扩展
是怎样的？"—— 下面是结论与落地。

**Zed 的语言扩展是纯声明式**：\`languages/<语言>/config.toml\`（识别 + 编辑规则）+
\`.scm\` 查询（highlights/brackets/indents/injections/...）+ 清单里的
\`[grammars.*]\`（tree-sitter 仓库 + rev）。高亮语言**不写 Rust**，但语法要在**装扩展时**
编成 \`.wasm\`（wasi-sdk 的 clang），运行时装进 **wasmtime 沙箱**。LSP 是另一段
（\`language_servers\`），只有它需要扩展的 Rust/wasm 部分。

**gpui-kit 给到哪一步**：注册接口是公开的，而且明确为"运行期加载的 WASM 语法"留了缝
（\`register_parser_factory\` 的文档原话），但**没有加载器**（没有 libloading / wasmtime /
dlopen）。语法集是**编译期固定**的 35 个（\`Language\` 枚举按 feature 门控，
\`singleton()\` 用 \`Language::all()\` 一次性建表）。

**我们这次的做法（编译期 + 公开注册缝）**：
1. 开工程文件需要的 feature：yaml / toml / sql / markdown / bash / kotlin；
2. \`buffer.rs::language_for_file\` 补映射（含无扩展名的 \`mvnw\` / \`gradlew\`）；
3. **XML 不在那 35 个里**，所以自己带 \`tree-sitter-xml\`（它同时给出 \`LANGUAGE_XML\`
   与 \`XML_HIGHLIGHT_QUERY\`，不需要自己收 \`.scm\`、也不需要 unsafe），在新增的
   \`editor/src/languages.rs\` 里用 \`LanguageRegistry::register\` 注册 ——
   **这条路径与"运行期加载语法再注册"是同一个 API**，将来换成加载器只改"Language 从哪来"
   这一个表达式。

**为什么这次没做"运行期加载"**：运行期要自己补加载器，只有两条路 —— 原生
\`dlopen\`（引入 \`libloading\` + \`unsafe\`，且**没有沙箱**：语法是 C 代码，等于允许
扩展执行任意本机代码）或 WASM + wasmtime（安全，但要引入 wasmtime 并自建"把 C 编成 wasm"
的打包步骤，等于把 Zed 的 extension_builder 重写一遍）。在"我们还没有第三方语法来源"的
当下，这两条都是先付代价后得收益；而注册缝已经就位，等真有第三方来源时再选。

**已知缺口（登记在此免得反复查）**：\`.properties\` 与 Groovy（\`build.gradle\`）
在两个语法集里都没有，仍是纯文本；\`Dockerfile\` 同理。颜色方面：gpui-kit 只认 **41 个**
高亮色名（封闭结构体 + 点号前缀回退），我们的主题只定义了 **16 个**；好在回退机制让
\`punctuation.delimiter\`→\`punctuation\`、\`string.escape\`→\`string\` 这类都能落到已有色上，
所以非 Java 语言也能看；补齐那 25 个是观感优化项（未做）。

### 第七批：图标集变成**叠加层**（缺的回落内置 `idea`）

维护者的口径是："内置 idea 的图标集，其他图标集做成插件化，切换到下载的那一个就用那个包的，
**缺少图标的直接使用内置的**。" 这条口径与改前的实际行为**不一致**，所以先改查找层。

**改前是什么样**：`file_icon.rs` 只要有一个生效包就**只查它**，查不到会落到代码里硬编码的
`DEFAULT_FILE = "idea-text"` / `DEFAULT_FOLDER = "idea-folder"`（清单自己的 `defaultFile` /
`defaultFolder` 字段根本没被读）。而这个 id 属于**内置包的命名空间**，换到 `pierre` 上必然
再 Miss 一次，最后画的是 **Lucide** 的细线文件图标 —— 于是"换图标集"在未覆盖的后缀上得到的
是第三种图标语言：既不是包里的，也不是内置的。

**这次的做法**（`file_icon::lookup_path`）：

1. **专属条目**：生效包的"映射表 + 定义表"作为一个整体查；没命中就换内置 `idea` 包再来一遍；
2. **默认值**：各自的 `defaultFile` / `defaultFolder` / `defaultFolderOpen`，
   **生效包自己声明的那份优先**；
3. 都落空才返回 `None`，调用方回落 Lucide（这一层没变）。

两条容易踩的语义，写在这里免得以后当成 bug：

- **具体优先于通用**：`idea` 有 `.java` 的专属条目、`pierre` 只有通用默认值时，`.java` 仍显示
  `idea` 的 java 图标 —— "缺的用内置的"就是这个意思。
- **id 是包内局部的**：每一级都在同一个包内完成"映射 → 定义"，**不跨包借 id**。所以两个包可以
  给同一个 id 配不同的图，也可以各用各的命名习惯；清单里图标 id 的 `idea-` 前缀也趁这次去掉
  （它是历史噪声，级联并不依赖 id 对齐）。

**顺带**：`defaultFile` / `defaultFolder` / `defaultFolderOpen` 从"代码硬编码"变成"清单声明"，
清单与代码不再能各说一套；`file_path` / `folder_path` 拆成"只查专属条目"的 `file_id` /
`folder_id` + 默认值取值器，级联本身是纯函数（不依赖 gpui 的 `App`，所以测试能直接构造包）。

**顺带修掉一个静默 bug**：`FileIcons::root()`（`icons/<包>/`）**从来没被拼到定义表的路径上**，
而 `theme_file_icon` → `LitheAssets::load` 要的是**资源根路径**，于是清单里那 53 条定义
**一条都 load 不到** —— 界面上文件类型图标一直是 **Lucide 细线字形**，不是 IDEA 的彩色美术，
而且"截图上看不出区别"（这正是 `every_definition_exists_in_the_assets_tree` 的注释警告过的
失败模式；两条旧测试都自己把根 join 上再比对，所以谁也没发现）。现在拼根发生在**解析清单时**
（`definitions`），那条测试改成直接断言表里的原值 —— 它当场抓到了这个 bug。

**验证**：`cargo test -p lithe-gpui-shared` 新增 `missing_entries_fall_back_to_the_builtin_pack`
—— 合成一个"只登记文件夹图标"的部分包，钉住四种命中：包内专属 / 回落内置的具体图标 /
生效包自己的默认值 / 内置默认值。

## 考虑过的备选方案

- **在清单里声明 `categories` 数组**（最初共识里选的就是它）：看完 Zed 源码后否决
  ——声明式分类需要额外校验"声明的分类与贡献项一致"，而推导式分类让这种漂移
  在结构上不可能发生。
- **独立的轻量 registry 契约**（Zed 的 registry entry 与 extension.toml 是两层）：
  否决。会造出两套目录文档，现有 `validate_plugin_catalog_json` 被旁路，v1 阶段
  目录与清单合一是更小的面。
- **用 moduleID 前缀约定推导分类**：否决。隐式契约，前缀改名即静默破坏。
- **扩展页继续放右侧工具窗**：否决。真机口径是中央区（`openExtensionsBuffer`），
  右栏空态是记录在案的临时偏离；且维护者明确要 Zed 式的独立界面。
- **v1 带安装 / 启停 / 下载**：否决。没有任何可安装的包、没有下载基础设施，
  画按钮就是假控件（`07-settings-ui.md` §7.3-D）。
- **hostVersion 放宽成区间或升级产品版本号**：否决。前者改契约语义，后者是独立
  的发布决策，都不该混在这个功能里。

## 后果

**收益**：Core 的清单校验从"零消费者"变成扩展系统的真实地基；目录、命令、宿主
三层都有确定性测试；分类与贡献项结构上不可能漂移；入口行为与真机口径对齐，
右栏偏差消除；主题首次有了**真实生命周期**（可卸载/重装），而且"卸载 = 删文件 +
记清单"这条路径不会在下一次启动被同步悄悄回滚。

**代价**：

- fixture 的 `hostVersion` 与宿主版本**硬耦合**：升级版本号时必须同步
  `official-v2.json` 和 `HOST_VERSION`（`lithe-gpui-extensions` 的
  `load_catalog_matches_the_bundled_fixture_host_version` 测试会在不同步时先红）；
- `lithe-core` 生产代码第一次 `include_str!` 一个 shared fixture（此前只有测试
  这么做）——这是"fixture 即产品数据"的有意先例，契约文档已写明；
- 旧会话里的 `rightView: "extensions"` 恢复时解不出视图，按"未知 id 保留当前值"
  的既有规则落到默认视图（不会 panic，可接受）；
- 扩展页关闭再打开不丢状态，但它不是编辑器 buffer，不出现在"打开的文件"里，
  也不进会话；
- **装载面是镜像**：直接改 `<配置目录>/themes/<捆绑主题>.json` 会在下次启动被扩展面
  覆盖（与内置主题"绝不覆盖"的规则相反）。改捆绑主题要改扩展目录那份 —— 这条差异
  来自"扩展有卸载/重装生命周期、内容必须跟着管理面走"；
- 卸载主题包会连带让该组的**两条**主题从主题下拉里消失；如果那正是当前主题，
  界面会回落到默认的 Darcula（设置文件里的原选择保留，重装后自动回来）；
- 老用户升级时有一次**文件迁移**（老播种在 `themes/` 的 6 份文件搬到扩展目录）：
  幂等、失败只留诊断；迁移后 `themes/` 里用户自己改过的同名文件会被扩展目录那份
  顶替（迁移的"缺才搬"保证了不会覆盖扩展目录里已有的）；
- **字体扩展多占约 550KB 仓库体积**（两个字体族各两个字重 + 许可原文）；
  字体是二进制资产，加一个字体族就是加一份体积，取舍时要知道这条；
- **字体卸载/重装要重启才生效**（GPUI 的 `add_fonts` 没有反注册入口）：
  文件立刻删/写，但已注册的字族活到进程结束。扩展页写了这句提示；
- 用户的字族覆盖值（设置里的 `monoFontFamily`）如果指向一个**已卸载**的字体扩展，
  会走既有的"字族未安装 → 不写入 + 留诊断"路径（界面回落到主题里的等宽字体），
  不会崩；重装后下次启动恢复。

## 验证


> **历史标注（2026-10-02）**：下面"验证"里以 `windows/tauri/**`、`bun test`、Windows Tauri 宿主
> `cargo test` 开头的命令，**随 Windows 前端删除而全部失效**，仅作历史记录；界面规格真源已改为
> macOS 前端（`macos/Sources/Lithe/Views/`）。需要复跑等价验证时，用 `rust/` workspace 里对应的
> `cargo test -p <crate>`，或本仓库真实的校验脚本（`scripts/verify-rust-core.sh`、`scripts/verify-shared-contracts.sh`）。
- `cargo test --manifest-path rust/Cargo.toml -p lithe-core`：plugins 用例全部通过
  （v2 校验、分类覆盖、v1 拒绝、`plugins.listCatalog` 信封级正反用例）。git/lsp
  引擎在本机有与本次无关的既有失败（干净基线同样失败，已复核）。
- `cargo test --manifest-path rust/Cargo.toml -p lithe-gpui-extensions / -p
  lithe-gpui-shared / -p lithe-gpui-settings / -p lithe-gpui-workbench / -p
  lithe-gpui-app`：全部通过（快照归一、过滤判据、i18n WIRED、右栏下标映射、
  扩展页筛选 id、播种/同步/卸载/重装/迁移的文件语义、字体族名解析、默认主题与默认
  字体的真源、镜像不被迁移误删的回归）。
- **实机启动验证**（隔离的 `LITHE_GPUI_SETTINGS_FILE`，三次启动）：
  `S1_FONTS registered builtin=4 bundled=4`（第一次）、`bundled=2`（卸载清单里有
  一个字体包之后）、`S1_THEME font_family key=monoFontFamily value=JetBrains Mono`
  （默认代码字体真的写进了主题 token，证明注册顺序正确）；卸载清单里的主题组与
  字体包在两个目录面都被清掉，稳态同步为 `created=0 kept=7 removed=0`。
- `node rust/lithe-gpui/tools/extract-locale.mjs` 重新生成两份 yml；新增 4 条
  gpui-only 键（字体 / 调试 / LSP / 加载失败）带理由。
- `scripts/verify-shared-contracts.sh` 的插件段已更新到 v2 并加了贡献项断言；
  ⚠️ 本机（Windows）没有 zsh / ruby，脚本本体跑不了，改用 Node 复刻同一段断言
  验证通过——CI（Linux）仍以脚本为准。
- `./scripts/verify-agent-notes.sh`、`./scripts/verify-rust-core-comments.sh`、
  `cargo fmt --check` 随本批次执行。

## 适用范围

- `rust/lithe-core/src/plugins/`、`rust/lithe-core/src/protocol/command.rs`、
  `rust/lithe-core/src/runtime/dispatcher.rs`、`rust/lithe-core/src/tests/plugins.rs`
- `shared/fixtures/plugins/official-v2.json`、`language-support-v2.json`、
  `shared/contracts/plugin-catalog.md`
- `rust/lithe-gpui/crates/extensions/`、
  `rust/lithe-gpui/crates/workbench/src/extensions_page.rs`、
  `rust/lithe-gpui/crates/workbench/src/workspace.rs`、
  `rust/lithe-gpui/crates/workbench/src/right_tool_window.rs`、
  `rust/lithe-gpui/crates/app/src/main.rs`
- `rust/lithe-gpui/crates/shared/`（`HOST_VERSION`、locales、i18n WIRED）
- `rust/lithe-gpui/themes/`（只剩内置的 `jetbrains.json`）、
  `rust/lithe-gpui/extensions/themes/`（五个捆绑主题组）
- `rust/lithe-gpui/crates/settings/src/theme.rs`（播种/迁移/索引/应用）、
  `settings/src/extensions.rs`（捆绑扩展注册表 + 装/卸/同步，主题与字体共用）、
  `settings/src/fonts.rs`（内置字体注册 + 字族名解析）、
  `settings/src/paths.rs`（扩展目录与两个覆盖开关）、
  `settings/src/schema.rs`（默认主题 + 默认代码字体 + `uninstalledExtensions`）、
  `settings/src/store.rs`（`set_extension_uninstalled`）
- `rust/lithe-gpui/assets/fonts/`（内置 IDEA 字体）、
  `rust/lithe-gpui/extensions/fonts/`（两个捆绑字体族，含 OFL 许可原文）
- `CONTEXT.md`（术语：扩展 / Plugin / 模块 / 目录 / 贡献项 / 分类 / 内置主题 /
  内置字体 / 捆绑扩展 / 装载面与扩展面 / 生效时机）
