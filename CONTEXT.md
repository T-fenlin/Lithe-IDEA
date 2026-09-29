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

### 主题与装载

**内置主题（Built-in theme）**:
编译进宿主、**不可卸载**的主题组（v1 只有 IDEA 一组：IntelliJ Light + Darcula，也是默认主题）。
清单入口点是 `builtIn`。

**捆绑主题扩展（Bundled theme extension）**:
随宿主分发、**用户可以卸载**的主题组（Gruvbox / Lithe / Nord / One / VS Code）。
清单入口点是 `bundled`；卸载是宿主侧的文件生命周期，不是目录事实。

**装载面与扩展面（load surface / extension surface）**:
主题只有一个装载目录（`<配置目录>/themes/`，注册表只能监视一个目录），而扩展包住在
`<配置目录>/extensions/themes/`。扩展面是**管理面**（装/卸都发生在这里），
装载面是**镜像**（启动时与扩展面对齐）。所以改一个捆绑主题要改扩展面那份。
_Avoid_: 把两个目录说成"双目录加载"（做不到，见 `settings/src/theme.rs` 模块文档）

