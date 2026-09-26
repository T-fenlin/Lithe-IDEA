# Agent 笔记：工作区与全局配置的分层与文件形状

状态：提议中

## 先说结论

Lithe 对标轻量级 IDEA，所以持久化数据照 IDEA 的两级模型组织：**IDE 级配置目录**（跟人走，跨项目）与**项目级 `.lithe/`**（跟项目走）。在这两级之内再分四层：全局层、**可共享的项目层**（进 Git 的文件）、**项目本机层**（跟"这个人 + 这个项目"走，不进 Git）、以及**派生物层**（缓存与索引，可随时删除）。项目本机层文件一律用 `*.local.json` 后缀，这样"这个文件该不该提交"看文件名就知道，不必记住目录位置的隐含含义——这是把 IDEA 的 `workspace.xml`（个人文件）概念推广到多份本机文档。`.lithe/` **默认不共享**：Lithe 往本机排除文件（`.git/info/exclude`）里写一个 `.lithe/` 条目，`git status` 里看不到它，只有用户显式执行"共享此项目的配置"才会移除该条目并提交。gpui 是目前唯一需要落地的实现端，它今天只有一份扁平全局设置、对 `.lithe/` 只有一处读、零写入，因此本提案要同时定下文件形状和一条"Core 只做纯文档变换、宿主负责原子落盘"的写通道。

## 问题

### 1. gpui 只有一份全局设置，没有项目层

`gpui/crates/settings/src/schema.rs` 里的 `Settings` 是一个**扁平标量结构**，19 个键**全部是全局作用域**，没有任何按项目或按工作区生效的键。这不是设计偏好，而是实现现状的登记——同一文件的注释写着：gpui 侧没有项目级存储，`.lithe/run/local.json` 的读写归 Core 的 `runConfig.*`，而这条通路还没有接上。

后果直接可见：**项目 JDK、Maven 可执行文件、Maven JDK、`settings.xml`、本地仓库这五个值，本来每个项目都可能不同，却被放在同一个全局文件里。** 设置页的文案因此必须专门标注"这一页的作用范围是全局，不是当前项目"，而且用户切换项目时要么忍受错误的 JDK，要么每次改一遍设置。

对照 IDEA：IDEA 把"项目用哪个 JDK"这件事拆成两半——项目文件 `.idea/misc.xml` 里存的是 JDK 的**名字**（`project-jdk-name`），实际安装路径存在 IDE 级配置目录的 `options/jdk.table.xml`（本机表）里。于是项目文件可以安全地提交和共享，而每台机器各自解析到自己的真实路径。Lithe 需要的是同一套拆分，gpui 现在一样都没有。

### 2. gpui 对 `.lithe/` 只有读，没有写

gpui 整个 crate 的非测试磁盘写入只有四处：全局 `settings.json`、`recent-projects.json`、JDT 缓存目录里的 `.lithe-last-used` 标记、以及用户自己保存的文件。对项目目录的**唯一**触及是读 `.lithe/toolchains/jdtls` 当发现候选根；研究文档因此写下了"gpui 侧没有写入通路（所以不能落这里）"的结论。

而项目级配置恰恰是 IDA 式 IDE 的核心：运行配置、模块结构、VCS 映射、源码根标记都要落到项目文件里。gpui 侧对 `.lithe/run/*.json` 只有"读"的一半（能解析、能分层），写的那一半不存在，所以运行配置页只能稳定失败。

### 3. 配置文件缺少能安全手改的性质

`.lithe/` 下的文件是给人改的（`run/local.json`、`maven/config.json`、`lsp/language-providers.json` 都是手写格式），但 gpui 现在的做法有四个会伤害手改者的地方：

- 全局 `settings.json` **没有 `version` 字段**，未知键**静默忽略**（`unknown_keys_are_ignored` 是一条正式测试）。用户手写的字段会在下一次落盘时被吃掉。
- **不监听外部改动**：`settings.json` 每个进程只读一次，改了不重启就不生效。
- **只在"完成"按钮里 flush**，关闭按钮 `×`、`Esc` 以及进程退出都会丢弃 300ms 防抖窗口内的改动。
- **逐键容错表是手写的**（`gpui/crates/settings/src/persistence.rs` 里那张表），历史上出过一次"设置写得出、读不回"的事故：新增键忘记登记，就静默不落盘。设置结构一旦允许数组与嵌套，这张手写表会更容易漏。

### 4. 待建能力没有落点

gpui 今天跨会话只持久化三样东西：全局 `settings.json`、`recent-projects.json`、JDT 索引缓存。会话恢复、面板尺寸、窗口几何、快捷键覆盖、用户自定义主题安装、断点、本地历史、日志落盘、最近文件、搜索与符号索引**全都没有存储**。

这不是"还没写"那么简单：没有落点规则，每个子系统实现者都会自己决定存哪，最后就会长成一份谁都说不清的全家桶。对照 IDEA，这些能力各自有明确归属：快捷键在配置目录的 `keymaps/`、配色在 `colors/`、JDK 表在 `options/jdk.table.xml`、主题与插件在配置目录、索引在缓存目录的 `index/`、本地历史在系统目录的 `LocalHistory/`、界面状态在项目里个人的 `workspace.xml`。

## 提案

本提案定义**目标形状**。gpui 是首个落地端；其余端按分阶段迁移跟进，它们的现有存储不构成本提案的设计输入。

### 一、四层模型

| 层 | 存在哪 | 进 Git | 换台机器还成立 | 删了会丢东西吗 | 对应 IDEA 的什么 |
| --- | --- | --- | --- | --- | --- |
| 全局 | 平台配置目录 | 否 | 否 | 是 | 配置目录的 `options/`、`keymaps/`、`colors/`、`plugins/` |
| 项目可共享 | `.lithe/` 下不带 `.local` 的文件 | 是 | 是 | 是 | 项目 `.idea/` 的 `misc.xml`、`modules.xml`、`vcs.xml`、`runConfigurations/` |
| 项目本机 | `.lithe/*.local.json` | 否 | 否 | 是 | 项目 `.idea/workspace.xml`、`shelf/` |
| 派生物 | 平台缓存目录 | 否 | 否 | **否** | 系统目录的 `caches/`、`index/` |

**硬规则：每个键只能属于一层，并在共享契约里声明。** 任何"两层都能写同一个键"的设计直接否决——一个文件有两个 writer，迟早互相覆盖对方的新内容。

### 二、全局层

路径按平台推导（Windows `%APPDATA%\Lithe\`、macOS `~/Library/Application Support/Lithe/`、Linux `$XDG_CONFIG_HOME/lithe/`）：

```text
settings.json          界面、编辑器、终端、语言、打开方式
keybindings.json       快捷键覆盖
recent-projects.json   最近项目（gpui 已在使用这个形状）
themes/                内置只读主题 + 用户主题；主题用 id 标识
state/                 机器写、**可丢弃**：窗口几何、模块开关、插件安装记录
data/                  机器写、**不可丢弃**：本地历史、Git shelf，按项目 id 分桶
```

`state/` 与 `data/` 必须分开：窗口几何删掉只是重算一次，本地历史删掉就是用户丢工作。把两者放进同一个"可以放心清空"的目录里，迟早会发生一次"清理缓存顺手删掉历史"。

缓存目录（Windows `%LOCALAPPDATA%\Lithe\cache`、macOS `~/Library/Caches/Lithe`）只放真正可以随时删除的派生物：

```text
language-servers/jdtls/<sha256(归一化工作区路径 + JDT 指纹)>/
```

`settings.json` **允许数组与嵌套对象**。理由是现有键集里本来就有数组（隐藏路径模式）和嵌套对象（功能开关组），强行拉平会变成"每新增一个数组就多开一个文件"。保留的原则是"逐键容错 + 未知键原样回写"，这两条在嵌套结构下同样成立。

主题标识统一用 **id**（`lithe-dark`）而不是显示名：机器可读、主题改名不影响用户引用。主题目录必须从**构建树**迁到这里——gpui 现在从 `<CARGO_MANIFEST_DIR>/../../themes` 读主题，那个路径在打包后的安装包里不存在，等于主题功能在发布版直接失效。

字体相关新增**三个**键：`fontFamily`（界面）、`monoFontFamily`（代码与终端）、`terminalFontSize`。

两个字族**有现成落点**：`gpui-base` 的主题有 typography `sans` 与 `mono` 两个 token
（`gpui-base-0.6.6/src/theme_tokens.rs:170-181`），主题 JSON 里它们也是可选字段，编辑器经
`gpui-component-0.6.6/src/input/editor.rs:139-142` 的 `.font_family(cx.theme().mono_font_family)` 取字体。
**空串 = 不覆盖**（与 `javaHomePath` 同一条约定），这样主题文件里指定的字体族不会被空设置抹掉。

⚠️ **`terminalFontSize` 没有现成落点，需要自己造一个缝**（实施时勘察，修正了先前"与写
`mono_font_size` 是同一条路"的说法）：终端正文**不用** `Theme::mono_font_size` —— 它用 typography 的
`sm` token（`.text_sm()` = 14px，`gpui/crates/terminal/src/terminal_view.rs`）。所以既不能写
`mono_font_size`（那会连带把编辑器一起改掉），也没有"终端字号"这个 token 可写。落地方式是终端视图
自己持有一个覆盖值（`TerminalPane::set_font_size`，`None` = 用默认档），由外壳按既有的"值型设置经
外壳转发"路子喂进去 —— 终端是按行渲染而不是字符网格，改字号没有列宽重算的问题。

⚠️ **两个字族键必须校验"这个字族装没装"**：GPUI 在字族找不到时会于**首次布局那一行 panic**
（`gpui-component-0.6.6/src/theme/mono_font.rs:1-13` 正是为这件事存在的），而这两个键是用户填的字符串
—— 一次手写错就能让应用再也起不来。所以写入前对照系统已装字族校验，认不出的**不写进主题**并留诊断；
设置页的下拉也只列已装字族（外加一个「默认（不覆盖）」项），让用户根本选不出会崩的值。

**不做** `lineHeight`、`iconTheme` 与 `ligatures`。前两者是独立工程（行高逐控件硬编码、图标主题是编译期常量）；
`ligatures` 是**侦察后砍掉**的：`FontFeatures::disable_ligatures()` 在上游存在
（`gpui-pre-0.3.6/src/text_system/font_features.rs`），但那条样式链只设字体族与字号，**没有任何地方读
FontFeatures**（`gpui-base-0.6.6/src/input/editor/display_map/text_wrapper.rs` 里的 `features:` 全在
`#[cfg(test)]` 里），加这个键就是"存了没用"。上游哪天给编辑器留出 font features 的缝，再把它加回来。

### 三、工作区层

```text
.lithe/
├── .gitignore                   只有本机成员与构建产物
├── project.json                 共享  version / id / defaultRunConfiguration / directoryMarks
├── settings.json                共享  工作区对全局键的覆盖（含外观）
├── git.json                     共享  vcs / roots / submodules
├── run/
│   ├── configurations.json      共享  团队运行配置
│   └── generated.json           共享  识别产物
├── toolchains/
│   └── requirements.json        共享  这个项目需要什么工具链（**用名字，不用路径**）
├── maven/
│   └── config.json              共享  selectedProfiles / customProfiles / skipTests
├── lsp/
│   └── language-providers.json  共享  手写 provider 覆盖（Lithe 只读）
├── settings.local.json          本机  个人的工作区覆盖
├── run.local.json               本机  运行配置覆盖 + toolchain 的**实际路径**
├── maven.local.json             本机  settingsPath / localRepositoryPath
├── toolchains.local.json        本机  工具链名字 → 本机实际可执行文件
└── session.local.json           本机  会话状态，可丢弃
```

`.lithe/.gitignore` 因此只需要三行：`*.local.json`、`run/classes/`、`**/*.tmp`。新增任何一个本机文档都不用再修改忽略表。它在"还没人共享这个项目"的阶段是惰性的（见第七节：那时 `.lithe/` 被本机排除整体挡住），但要生效的时刻正是共享那一刻，所以必须始终维护。

关键是**共享文件里不出现绝对路径**，这一点与 IDEA 的做法一致：`.lithe/toolchains/requirements.json` 说"这个项目需要 Java 21"，`.lithe/toolchains.local.json` 和 `run.local.json` 才说"这台机器上 Java 21 在 `D:\...`"。`directoryMarks`（`plain | sources | resources | excluded | module | package`）放进共享的 `project.json`：它是团队共享的项目结构事实，对应 IDEA 里被提交的模块与源码根配置。

`git.json` 只放团队共享的**仓库结构事实**：`vcs`（`git`）、`roots`（哪些子目录属于哪个仓库/子模块）、`submodules`。Git 可执行文件路径、是否用凭据助手、fetch 策略留在全局层——它们是"这台机器怎么装 Git"，不是"这个项目是什么"。fetch 配置现在有三种形状（一个对象、三个散键、以及 gpui 完全没有），统一时只保留一种。

### 四、覆盖顺序

```text
内置默认值 < 全局 settings.json < 工作区 settings.json < 工作区 settings.local.json
```

工具链的五个值单独一条顺序，对应 IDEA"项目存名字、本机存路径"的拆分：

```text
项目本机（run.local.json 与 maven.local.json）> 全局默认 > 自动发现
```

保留"全局默认"这一层是有意的：多数开发者在这台机器上主用一个 JDK，新建项目不该要求重新手填一遍。

**外观类键允许被工作区覆盖**（本次决策，与"外观只跟人走"的直觉相反）。IDEA 的先例支持这个选择：它的代码风格与配色都可以选"Project"级方案并随项目提交。随之而来的硬要求：

- 工作区对全局键的覆盖单独放 `.lithe/settings.json`，不与个人覆盖混在同一个文件里。
- 进入一个覆盖了外观的工作区时，必须给出可见提示，说明当前外观来自工作区。
- 必须提供"改回我的全局外观"一键操作。
- 覆盖值沿用已有的钳制范围（`uiFontSize` 10–24、`fontSize` 10–22 等），不允许让界面变得不可用。

### 五、项目身份

`.lithe/project.json` 的 `id`（首次生成时写入的 UUID v4）是**真源**，随项目提交进 Git。理由：目录改名、仓库搬家、换台机器 clone 之后，本地历史、断点、会话仍然认得这是同一个项目。

没有 `id` 的老项目回落到 Core 已经实现好的路径身份：`normalized_workspace_identity` + SHA-256（`rust/lithe-core/src/lsp/languages/jdt.rs`，输出 64 位小写 hex，Windows 盘符与 UNC 路径折叠大小写、POSIX 不折叠）。二者是**主/备关系**，不是二选一。

统一用这个 id 的地方：本地历史分桶、Git shelf 分桶、会话分桶，以及所有"每个项目在全局目录里占一个坑"的文档。

**索引缓存例外，继续用内容寻址**：目录名保持 `sha256(归一化路径 + JDT 指纹)`。那个键的职责是内容寻址——同一个路径换了 `pom.xml` 就必须换目录，否则会复用一份陈旧的项目模型；把项目 id 塞进去会让一个键同时承担"我是谁"和"我是什么内容"两种语义。索引是可丢弃的派生物，搬家后重建是可以接受的代价。

### 六、版本、手改与写入生命周期

- 每个配置文件顶层写 `version`。
- 读入时**保留未知键**并在写回时原样保留（round-trip）；遇到比当前支持的更高版本，转为只读并提示，不静默降级、不丢字段。
- 读取**不得创建文件**（保留 gpui 现在的性质：文件不存在就用默认值，且不落盘）。
- Core 只做纯文档变换：校验、合并、按版本迁移，**不碰文件系统**（沿用现有 `runConfig.*` 契约"这些命令从不写文件"）。
- 宿主负责：原子写（写 `<name>.json.tmp` 再 rename）、300ms 防抖、**进程退出与关闭窗口前 flush**、**监听外部改动并重新加载**。
- 尺寸类值只在拖动结束时落盘，不在拖动过程中持续写入。
- 键表不再是手写的：读写由 schema 派生，避免"新增键忘记登记就不落盘"再发生一次。

### 七、共享层文件与 Git 的关系

**默认不共享，而且让它在磁盘上成立。** Lithe 在第一次往 `.lithe/` 写文件时，把这个目录加进**本机排除文件**（`.git/info/exclude`），于是 `git status` 里根本看不到 `.lithe/`。它不碰仓库顶层的 `.gitignore`——那个文件是团队的、会被提交，往里面塞个人的忽略规则是错的。

> **这里偏离了一条既有约束，需要明确记录。** 仓库现在的口径是"Git 写操作只在用户显式动作时发生"（Core 的 `GitIgnoreTarget` 有 `Repository` 与 `LocalExclude` 两个目标，现有 UI 只在用户点"忽略"时才用它）。本提案让 Lithe **自动**写一次本机排除，属于对该约束的**定向例外**：理由是"默认不共享"如果不落到磁盘，就只是产品口径，用户每次打开陌生仓库都会看到一堆未跟踪的 `.lithe/` 条目，而 `git add .` 是最常见的操作之一。例外的边界限定为三点：只写 `LocalExclude` 目标、只写 `.lithe/` 一个条目、只在条目缺失时追加（不重写文件、不动别人的规则）。

**共享是一个显式动作，并且必须是一步就能完成的。** 被排除的文件用 `git add` 提交需要 `-f`，让用户自己发现这一点是设计缺陷。所以：当用户显式要求共享这个项目的配置时，Lithe 先**移除自己写的那一行**，再提交选中的可共享文件。用户不需要知道 `-f` 存在。

层的划分在这条默认之下仍然必须存在，因为它回答的是**共享那一刻**的问题：

- 带 `.local.json` 后缀的文件，以及派生物（`run/classes/`、`*.tmp`），**绝不能**进版本控制。`.lithe/.gitignore` 就是这条规则的机器可读形式。
- 其余文件**可以**共享，但它们的内容必须满足"换一台机器仍然成立"。绝对路径因此一律不许出现在这些文件里——这是"可共享"对"内容"的约束，与当前是否真的共享无关。

两套忽略机制不是重复，而是**分时间生效**：`.git/info/exclude` 里的 `.lithe/` 是"没人共享时"的整体屏蔽；`.lithe/.gitignore` 是"有人共享之后"防止本机成员被误提交的第二道闸。`.lithe/` 被整体排除期间，`.lithe/.gitignore` 是惰性的（Git 不会进入被排除的目录）——它要生效的时刻，正是那一行被移除的时刻。

"同事的主题覆盖我"这件事因此有完整的条件链：

```text
工作区把外观写进可共享的 .lithe/settings.json
  → 有人显式执行"共享此项目的配置"（默认不会发生）
  → Lithe 移除本机排除行 → 提交
  → 同事拉到
  → 同事那边生效
```

**没有人执行这一步，这个需求就不成立**，那份覆盖只对写它的那台机器可见。产品文案不该暗示相反的意思。

生效语义定为：**只看文件在不在磁盘上，不看它是否被 Git 跟踪。** 但界面必须把来源画清楚，而且**默认保持静默**——在没有人共享 `.lithe/` 的项目里持续显示"尚未与团队共享"，等于把不共享当成异常，与本节原则相反：

| 覆盖来自哪里 | 界面应当怎么说 |
| --- | --- |
| `.lithe/settings.local.json` | 当前外观来自**你的个人覆盖**（默认形态，不需要额外提示） |
| `.lithe/settings.json`，该文件未被跟踪 | 当前外观来自**本项目的设置**；只有当这个仓库已经在共享 `.lithe/` 时，才补一句"这个文件尚未提交" |
| `.lithe/settings.json`，该文件已被跟踪 | 当前外观来自**团队设置** |

不采用"必须被跟踪才生效"的理由见备选方案七。

**边界情况**：项目不在 Git 仓库里时什么都不做；仓库是 worktree、submodule 或 bare 时，`.git` 可能是文件而不是目录，排除文件的位置必须通过 Git 自己解析（`git rev-parse --git-path info/exclude`）而不是硬编码 `<root>/.git/info/exclude`。

### 八、尚未建成的能力应当落在哪一层

本提案只定分层与文件形状，不设计这些子系统。但每个待建能力必须先认领一层。

| 待建能力 | gpui 现状 | 应落层 | 目标位置 |
| --- | --- | --- | --- |
| 会话恢复（打开的文件、光标、滚动、展开节点） | 无 | 项目本机 | `.lithe/session.local.json` |
| 面板与窗格尺寸 | 无 | 项目本机 | `.lithe/session.local.json` |
| 断点与监视表达式 | 无 | 项目本机 | `.lithe/session.local.json`（可丢弃） |
| 窗口几何 | 每次启动重算 | 全局 | `state/` |
| 快捷键覆盖 | 无 | 全局 | `keybindings.json` |
| 用户自定义主题 | 从构建树读，发布版失效 | 全局 | `themes/` |
| 最近打开的**项目** | 有 | 全局 | `recent-projects.json` |
| 最近打开的**文件** | 无 | 全局 | `state/` |
| 插件安装记录、模块开关 | 无 | 全局 | `state/` |
| 日志落盘 | 无（只有 stdout 的 `S1_*` 行） | 全局 | 沿用平台日志目录，不放进配置目录 |
| 本地历史 | 无 | 全局 | `data/local-history/<项目 id>/` |
| Git shelf | 无 | 全局 | `data/shelves/<项目 id>/` |
| 搜索 / 文件 / 符号索引 | 无 | 派生物 | 缓存目录下按项目 id 分桶，删了要能重建 |
| 编译产物 | 无（运行还没接） | 派生物 | `.lithe/run/classes/`，且必须留在 `.lithe/.gitignore` 里 |

判定顺序只有三步，实现者按它走就不会放错：**换台机器还成立吗**（成立 → 项目可共享层）→ **换个人还成立吗**（成立 → 全局层）→ **删了会丢用户的东西吗**（不丢 → 派生物层；丢 → 项目本机层或全局 `data/`）。

### 与 IDEA 的差异（明确取舍）

| 方面 | IDEA | 本提案 | 为什么不同 |
| --- | --- | --- | --- |
| 文件格式 | XML（`misc.xml`、`workspace.xml` 等） | JSON | 与现有 Core 契约一致，人是主要读者但机器要能逐键容错 |
| 个人文件的识别方式 | 固定文件名 `workspace.xml` | `*.local.json` 后缀 | 本机文档有多份，一个后缀规则比记住若干个固定名字更省心 |
| 个人文件的粒度 | 一个 `workspace.xml` 装下大部分界面状态 | 按子系统分成多份 `*.local.json` | 一个文件一个 writer；单文件会被多个子系统互相覆盖 |
| 目录角色 | `.idea/modules.xml` + `*.iml` | `project.json` 的 `directoryMarks` | 单模块优先的轻量版，先不做多模块文件拓扑 |
| 工具链引用 | `misc.xml` 存名字、`jdk.table.xml` 存路径 | `toolchains/requirements.json` 存名字、`toolchains.local.json` 与 `run.local.json` 存路径 | 同一思路，键名沿用 Core 契约 |
| 索引位置 | 系统目录 `index/`、`caches/` | 缓存目录 `language-servers/jdtls/<key>/` | 同思路 |
| 默认是否共享 `.idea` | 不代替用户提交，未跟踪文件由用户在提交对话框处理 | 自动写本机排除，显式动作才共享 | 本提案多走一步：让"默认不共享"在磁盘上成立，见第七节 |

### 正确做法

- 把"这个项目需要 Java 21"写进 `.lithe/toolchains/requirements.json`（可共享层），把"这台机器上它在哪"写进 `.lithe/run.local.json`（本机层）。
- 团队要统一主题时，用"共享此项目的配置"这一个动作完成：把外观写进 `.lithe/settings.json`、移除本机排除行、提交。
- 新增一个持久化字段时，先在共享契约里声明它属于哪一层，再写代码。
- 新增本机文档时直接命名为 `<name>.local.json`，不需要改 `.lithe/.gitignore`。

### 不要这样做

- 不要把绝对路径写进任何可共享文件。换台机器就不成立，而这类文件按定义是要被别人拉走的。
- 不要把"共享"做成默认行为：不要自动 `git add` `.lithe/`，不要弹出"建议加入版本控制"，不要替用户决定提交哪些文件。
- 不要把 `.lithe/` 写进仓库顶层的 `.gitignore`。那个文件是团队的、会被提交，个人忽略规则只能进 `.git/info/exclude`。
- 不要重写整个排除文件或删除别人的规则，只追加自己那一行，且仅在缺失时追加。
- 不要让第二个子系统去写同一个文件。一个文件只有一个 writer。
- 不要把可丢弃的东西和不可丢弃的东西放进同一个目录（`state/` 与 `data/` 必须分开）。

## 实施进度

增量 1（**配置文档底座**）已在 gpui 落地并验证，落在 `gpui/crates/settings/src/` 的
`persistence.rs`、`store.rs`、`watch.rs`。

增量 2/3 的**项目侧骨架**也已落地（`gpui/crates/shared/src/` 的 `document.rs` 与
`workspace_config/`）：`.lithe/` 路径真源、`project.json` 的身份、**"默认不共享"守卫**、
可调用的共享动作。尚未落地的是**各成员文件自己的读写方**——`.lithe/settings.json` 覆盖层
（增量 4/5）、会话状态（增量 8）等仍然没有 writer。

### 打开即建（产品决策）

**打开任何一个目录就建立它的 `.lithe/project.json`，无条件。** 这是明确的产品决策，**不是**
"按需生成（识别出项目类型 / 用户点运行 / 显式保存设置）"：用户在两者之间选了前者。

落地形式是 `ShellWorkspace::new` 里的一次后台解析（`resolve_project_id`）。选这里是因为它是
**所有"打开"路径的公共收口**：`ShellWorkspace::new` 全仓库只有两个调用点 —— `main.rs` 的启动
（argv 给的根）与 `rebuild_project_window` 的 `replace_root`（选择器、项目下拉里的最近项目、
探针三条入口都经 `request_open_project` → `execute_project_open` 汇到这里）。接一处就够了，
不需要给每个入口各接一次。

**"先确保不共享、再写清单"这条顺序由 `save_project_manifest` 内部保证**，调用方不需要记得先调守卫
（自己拼 `ensure_project_dir_excluded` + 写文件会重实现一遍顺序保证，那是禁止的）。

连带后果（都是明确接受的，不是疏漏）：

- **非 Git 目录也会多出一个 `.lithe/`**：守卫返回 `NotARepository` 后清单照建。
- 打开一个目录会**产生磁盘写入**，其中包含对 `.git/info/exclude` 的一次写入——这正是第七节里
  那条"偏离既有约束的定向例外"。
- **失败绝不让打开项目失败**：`.lithe` 建不出来、排除写不了、Core 报错，都只留
  `S1_WORKSPACE_CONFIG` 诊断；窗口照常打开、项目照常可用。
- 解析走**后台执行器**（`background_spawn`），因为这条链会调 Core（`git.write` 内部要
  `git rev-parse`，是子进程），不允许落在 UI 线程上。

**一个待补的实现缺口（不是这条决策的后果，但被它放大）**：`.lithe/.gitignore` **至今没有 writer**。
刚打开的仓库里这不是问题 —— 整个 `.lithe/` 被本机排除挡着。但一旦有人执行共享、移除那一行，
本机层文件（`*.local.json`）就会重新出现在 `git status` 里，因为第二道闸还不存在。设计上明确要求
"Lithe 必须**始终**维护 `.lithe/.gitignore`"（第七节），所以这是要补的实现，应当与增量 4/5
（工作区覆盖层落地、第一次真的写出 `*.local.json`）一起做。

**唯一覆盖不到的路径**：`OpenDestination::NewWindow`。它今天什么都不打开（多窗口整批暂缓，
`execute_project_open` 里只报一句"尚未接入"），所以没有漏；将来多窗口落地时它会新建窗口、
同样走 `ShellWorkspace::new`，那时自动被覆盖。

### 排除条目用未锚定的 `.lithe/`（已决定的取舍）

写进本机排除文件的是**未锚定**的 `.lithe/`（`workspace_config_exclude_pattern()`）。Git 里不带
前导 `/` 的模式**在任何深度都匹配**，所以代价是具体的：把 `<repo>/subdir` 当项目打开时，那一行
同时也忽略了 `<repo>/.lithe/` 以及仓库里任何嵌套的 `.lithe/`；依次打开同一个仓库的多个子目录时，
彼此的工作区配置会被互相忽略。**共享**其中某一个时（`unexcludePatterns` 按同一 literal 精确删行）
会把其他工作区的排除行一起去掉 —— 这是这个取舍唯一真正会咬人的地方。

**为什么不改成锚定的写法**：唯一正确的锚定形态是"仓库根 + 工作区相对路径"（根 == 仓库根时
`/.lithe/`，根是子目录时 `/subdir/.lithe/`），而要知道仓库根就得先问一次 Git（多一次 Core 往返）。
**只把模式锚定到排除文件所在目录（`/.lithe/`）是错的**：那对"把子目录当项目打开"的情形会去忽略
`<repo>/.lithe/` 而不是 `<repo>/sub/.lithe/`，等于在真正需要它的地方失效。

**什么时候该改**：当一个仓库里同时存在多个 Lithe 工作区、且其中至少一个需要共享配置时。
在那之前保持现状：它隐藏的都是 `.lithe/` 目录，而"默认不共享"本就是这些目录的默认语义。

| 已完成 | 证据 |
| --- | --- |
| 顶层 `version` + 更高版本转只读（只读期间不覆盖用户文件，内存照常生效） | `persistence::DOCUMENT_VERSION`、`parse()`；测试 `document_version_is_written_and_normalized`、`newer_document_version_is_read_only` |
| 未知键（含嵌套对象内部）写回原样保留 | `merge_document()` / `preserve_unknown()`；测试 `unknown_keys_are_preserved_across_a_write`、`nested_unknown_keys_are_preserved`、`non_object_previous_does_not_break_the_merge` |
| 键表由 schema 派生，**手写逐键表已删除** | `known_keys()`；守卫测试 `any_single_broken_key_leaves_every_other_key_intact`（逐个已知键喂非法值，要求其余键完好且该键必须被判成坏键） |
| 外部改动监听并热重载（监听父目录，避开 rename 让文件级监听失效） | `src/watch.rs`；端到端实测：外部改文件后 `S1_SETTINGS reloaded … ui_font_size=15.5`（13 → 15.5，未重启） |
| 进程退出前 flush（补上 300ms 防抖窗口里那最后一次改动） | `SettingsStore::install_quit_flush`（gpui `on_app_quit`） |
| 通用文档原语抽到 `shared::document`，设置文档改为委托它（两个使用方） | 设置文档 + `.lithe/project.json`；`document.rs` 9 条测试用**与设置无关**的类型证明通用性 |
| `.lithe/` 路径真源（此前整个仓库没有具名常量） | `WorkspaceConfigPaths`（`workspace_config::paths`）；`members_live_under_the_config_directory` 等 6 条测试 |
| `project.json` 的稳定身份（UUID v4，回落 Core 路径身份） | `resolve_project_id`；测试 `a_new_workspace_gets_a_generated_uuid_identity`、`newer_manifest_version_falls_back_to_the_path_identity` |
| 写 `.lithe/` 之前**先确保不共享**（写 `.git/info/exclude` 的 `.lithe/`，位置由 Core 解析） | `ensure_project_dir_excluded`；**真实临时仓库**端到端：`git status` 无 `.lithe`、排除文件恰好一行、幂等、别人规则逐字保留 |
| 显式共享：移除排除行 + 只暂存可共享成员 | `share_project_config`、`is_shareable_member`；端到端断言 `*.local.json` 与 `run/classes/**` 未进暂存区 |
| **接进产品：打开即建**（`ShellWorkspace::new` 的后台解析，失败不影响打开） | `workbench::workspace::prepare_workspace_config`；端到端两组（Git 仓库 / 非 Git 目录）见下 |
| **工具链五个值的项目本机层**（`.lithe/run.local.json` 的 `toolchain` + `.lithe/maven.local.json`，schema `maven-local-v1`） | `shared::workspace_config::toolchain`；读写往返、清除、`configurations` 保留、旧路径双读、更高版本拒写等单测 |
| **`项目本机 > 全局默认 > 自动发现`** | `ToolchainPaths::resolve`（**唯一实现**）+ `settings::project::resolve_overrides`（只搬运字段）；端到端 A/B 见下 |
| **`.lithe/.gitignore` 的 writer**（第二道闸，此前一直没有写入方） | `sharing::ensure_local_ignore_file`；只追加缺失规则，规则齐全时**不写文件** |
| 设置页如实标注：有工作区写项目本机层、没有才写全局；每一格标出覆盖值来源 | `dialog::project_save` / `project_load` / `override_origin_key`；文案键 `projectScopeProject` / `overrideFromProject` / `overrideFromGlobal` |
| 语言服务登记改用**生效值**（此前只读全局） | `workbench::register_java_toolchain(root, cx)` + `S1_SETTINGS wiring=java_toolchain … project=/global=/unset=` |
| **字体三键**：两个字族写主题 token（带已装校验）+ 终端字号经外壳转发给终端视图 | `schema.rs` 的 `normalize_terminal_font_size`、`theme::apply_font_families` / `usable_family`、`TerminalPane::set_font_size`、`workbench::workspace::terminal_font_size_override`；测试见下 |
| **守卫与共享动作回读校验**（写了 ≠ 写进去了；不再谎报 `excluded`） | `sharing::pattern_is_present`（纯函数，直接直测）+ `SharingError::NotPersisted`；正常路径的回读闭环测试 `ensure_excluded_verifies_its_own_write_by_reading_back` |
| **工作区配置失败对用户可见**（常驻红条，可手动关） | `ShellWorkspace::workspace_config_error` + `render_workspace_config_error`；文案键 `gpui.workspaceConfigFailed`（`GPUI_ONLY_KEYS`，locale 生成） |

验证结果（增量 4 · 工具链分层 + `.lithe/.gitignore`）：

- `cargo test -p lithe-gpui-shared --lib workspace_config`：**25 通过 / 0 失败**（新增 10 条：
  `resolve` 优先级、空白不覆盖、五项往返、清除、`configurations` 保留、旧路径双读、
  排除条目、`.gitignore` 写入、更高版本）
- `cargo test -p lithe-gpui-settings --lib`：**144 通过 / 0 失败**（新增 2 条：字段映射逐位、
  `resolve_overrides` 优先级）
- `cargo test -p lithe-gpui-workbench --lib`：**68 通过 / 0 失败**（新增 1 条：登记边界上的优先级）
- `cargo check --workspace --all-targets`：**exit=0**
- `verify-test-stability.ps1` 静态 gate：**通过**
- **端到端 A/B（优先级）**：临时 Git 仓库 + 仓库内配置目录；全局写 `javaHomePath=D:\global\jdk-B`，
  项目本机写 `javaHomePath=D:\project\jdk-A`：
  - 项目本机有值时：`S1_SETTINGS wiring=java_toolchain java_home_path=D:\project\jdk-A
    maven_settings_path=D:\global\settings-B.xml project=1 global=1 unset=3`
    —— **项目本机压过全局**，且项目本机没设的 `settings.xml` 回落到全局；
  - 把项目本机的 `javaHomePath` 清空后重启：`java_home_path=D:\global\jdk-B project=0 global=2`
    —— **确实回落到全局默认**。
- **端到端（`.gitignore` + 不共享）**：启动前 `.lithe/` 在 `git status` 里是 `?? .lithe/`；
  启动后 `.lithe/.gitignore` **被建出且正好三行**（`*.local.json` / `run/classes/` / `**/*.tmp`）、
  `.git/info/exclude` 里 `.lithe/` **恰好一行**、`git status --porcelain` **为空**；
  二次启动 `.gitignore` 的**字节与 mtime 都不变**（规则齐全 → 一个字都没写）。
- 跑完确认：无残留 `Lithe` 进程、临时目录已删。

验证结果（增量 7 · 字体三键）：

- `cargo test -p lithe-gpui-settings`：**142 通过 / 0 失败**（改动前 139；新增
  `terminal_font_size_normalizes_to_unset_or_clamped`、`font_family_overrides_are_trimmed`、
  `usable_family_only_accepts_empty_virtual_or_installed`）
- `cargo test -p lithe-gpui-terminal`：8 通过 / 0 失败
- `cargo check -p lithe-gpui-settings -p lithe-gpui-terminal -p lithe-gpui-workbench --all-targets`：exit=0
- `verify-test-stability.ps1` 静态 gate **通过**；`node gpui/tools/extract-locale.mjs --check` **通过**
- ⚠️ **`cargo test --workspace` 没能跑完**：同批另一个增量（工具链分层）的 `workbench` 在制品
  还引用着未导入的符号，7 个编译错误全部落在 `gpui/crates/workbench/src/workspace.rs` 的
  `mod tests`（3785 行以后）。本次在 `workspace.rs` 的改动都在 1160/1190 与应用路径上，
  没有一处落在测试模块；两个受影响 crate 的测试与静态检查都已通过，整套验证留给父代理在
  两个增量都停稳后复跑。
- **两个字族键的"已装校验"没有 GUI 级验证**（本次不启动 Lithe，避免与并发代理的窗口互相干扰）；
  `usable_family` 的四种分支由确定性单测覆盖。

验证结果（增量 1）：

- `cargo test --workspace`（gpui，10 个 test binary，合计 **334 通过 / 0 失败**；settings 由 125 → **127**）
- `cargo check --workspace --all-targets`（gpui）**exit=0**
- `verify-test-stability.ps1` 静态 gate **通过**
- 端到端一次：外部改文件 → `S1_SETTINGS reloaded … ui_font_size=15.5`（13 → 15.5，未重启），
  进程已关闭、无残留

验证结果（增量 2/3）：

- `cargo test --workspace`（gpui）**exit=0**，全部 test binary 0 失败（`shared` 37 条，其中本轮新增 26）
- `verify-test-stability.ps1` 静态 gate **通过**
- 端到端（真实临时 Git 仓库）见上表两行；脚本细节与限制写在 `gpui/PLAN.md` §8.9

验证结果（接进产品 · 打开即建）：

- `cargo check --workspace --all-targets`：**exit=0**
- `cargo test --workspace`：**372 通过 / 0 失败**（其中 `workspace_config` 16 条）
- `verify-test-stability.ps1` 静态 gate：**通过**
- **端到端 · Git 仓库**（临时仓库 `git init` + 一次提交，配置目录指到仓库内）：
  首次 `excluded … pattern=.lithe/` → `manifest_saved bytes=67 exclusion=Ensured` →
  `project_id_created id=4be89b14-…` → `shell_identity … source=manifest`；
  `.lithe/project.json` 存在、`version=1`、`id` 是 UUID v4；本机排除文件里 `.lithe/` **恰好一行**；
  `git status --porcelain` **完全为空**（连 `.lithe` 都没有）。二次启动：`id` / 字节数 / mtime
  **三者都不变**、排除仍是一行、日志里**没有** `project_id_created`（证明没有无谓改写）。
- **端到端 · 非 Git 目录**（用 `GIT_CEILING_DIRECTORIES` 挡住向上找到外层仓库）：
  `manifest_saved … exclusion=NotARepository` → `project_id_created` → `.lithe/project.json` 建出来了，
  且**没有**创建任何 `.git` 或排除文件、**没有** `exclude_failed` / `shell_identity_failed`。
- 两组跑完都确认：真实 Lithe 仓库的 `.git/info/exclude` 仍只有 git 默认注释（**零污染**）、
  无残留 `Lithe` 进程、临时目录已删。
- 这一层**没有单测**：`ShellWorkspace::new` 需要真实 `Window`，而仓库既有口径就是"测试不构造它"
  （`gpui/crates/settings/src/identity.rs` 里已登记）。所以证据来自上面两组端到端，
  底下那一层（`shared::workspace_config`）已有 16 条真实 Git 仓库测试。

⚠️ **环境注意（与代码无关，但会误导后来者）**：这台机器的 `%TEMP%` 对 Rust 测试进程返回
`PermissionDenied`，仓库外的 `D:\` 与 `%LOCALAPPDATA%` 同样被拒；于是 `lithe-gpui-editor` 里
5 个使用临时目录的测试会失败（`buffer.rs:390` 的 `create_dir_all`），且 rustdoc 的 doctest 会因为
建不了临时目录而报"compiler unexpectedly panicked"。把 `TEMP`/`TMP` 指到仓库内可写目录后一切正常。
代价是"临时目录"落在 Lithe 仓库自己的工作树里 —— 所以任何直接调排除守卫的测试都必须先在自己的
临时目录里 `git init`，否则会通过 `git rev-parse` 找到**外层**仓库、把 `.lithe/` 写进真实检出的
`.git/info/exclude`。

**这条限制对应用进程同样成立**（不只是测试进程）：把根指到 `%TEMP%` 或仓库外的目录时，
`create_dir_all` 报 `os error 5`，而写既有文件可能"看起来成功却没落盘"。所以接进产品这一步的
端到端必须在 `<repo>/.artifacts/` 下建临时仓库；非 Git 那一组用 `GIT_CEILING_DIRECTORIES`
把向上查找挡住，否则会误判成"在仓库里"并污染外层仓库的排除文件。

**未能执行的验证**：测试计时 harness 没有 gpui scope（`test-stability-windows.ps1` 只支持
`Frontend` / `WindowsRust` / `SharedRust`），所以这次拿不到 gpui 的逐测试计时报告。

连带修正的文档（不改就会留下"照旧结论做"的坑）：`gpui/PLAN.md` §8.4 / §14.2 / §15.3、
`gpui/HANDOFF.md`、`gpui/research/settings-lsp-and-run-pages.md`、
`gpui/research/open-project-and-windows.md`、`gpui/research/editor-lsp-completion.md`、
`gpui/crates/settings/src/recent_projects.rs` 的模块文档。

### 两条硬要求（实测缺陷换来的）

**一、宣称成功之前必须回读。** 实测：把根指向会话工作区之外的仓库时，日志里出现
`S1_WORKSPACE_CONFIG excluded root=… pattern=.lithe/`，而那个仓库的 `.git/info/exclude` 里
**根本没有 `.lithe/` 行**（仍是 git 默认模板注释）—— 写入被环境静默丢弃，`fs` 不报错、Core 也返回成功，
旧实现于是直接宣称成功。这与仓库"不静默丢弃错误"的规则冲突，而且后果很具体：用户以为目录已被排除，
实际它正暴露在 `git status` 里。**所以：写（或删）之后读回 `<gitCommonDirectory>/info/exclude`，
确认那一条的状态确实变了；没变就返回结构化失败（`SharingError::NotPersisted`），绝不打成功那行。**
判据抽成纯函数 `sharing::pattern_is_present`，因为它所对应的真实成因（静默丢写）**没法确定性构造**，
只能把判据本身拿出来直测。

**二、失败必须对用户可见。** 同一个实测里，失败只落在 stderr 上；双击启动的用户看不到它，
只会发现"`.lithe` 目录没出现"。仓库既有口径是"失败一定可见：常驻红条"（`changes_view.rs` 的
`render_write_error`），所以打开即建这条链的失败也回填到外壳的常驻红条上（可手动关、不自作消失）。
这两条不只服务 `.lithe` 身份文件：任何"Lithe 主动写用户目录"的动作都适用同一标准。

## 考虑过的备选方案

### 备选方案一：照 IDEA 只用一个个人文件（`workspace.local.json`）

最贴近 IDEA 的形状：全部个人状态进一个文件，识别方式简单到"认文件名"。不采用的原因：那个文件会被会话恢复、面板尺寸、断点、终端列表等至少五个子系统同时读写，而它们各自的保存时机不同（有的防抖、有的拖动结束才写）。一个文件五个 writer，必然出现"我刚调的断点被面板尺寸的落盘覆盖掉"。IDEA 能用一个 `workspace.xml` 是因为它在一个进程内按组件串行写 XML；gpui 的多个面板是两个独立实体，没有这个前提。

### 备选方案二：项目本机层用目录位置表达归属（`run/local.json`）

零迁移成本，沿用 Core 已经发布的路径。不采用的原因：位置语义要求每个人都记住"`run/` 目录下的 `local.json` 是本机、`configurations.json` 是共享"，而这两种文件会分布在 `run/`、`maven/`、`toolchains/` 三个目录里，同类判断要做三次。后缀语义把判断压成一条规则。代价是一段**双读**过渡期，这个代价是明确接受的。

### 备选方案三：项目身份只用路径哈希，不落盘 UUID

零新增字段、零发明，直接复用 Core 已经实现好的 `normalized_workspace_identity` + SHA-256。不采用的原因：它是**路径派生**的，目录改名或换台机器 clone 之后身份就变了，本地历史、断点、会话会全部对不上——而这恰恰是"换个目录继续干活"最常见的场景。UUID 落盘的唯一代价是多一个提交字段，值得。

### 备选方案四：索引目录改成 `<项目 id>/<指纹>` 两级

好处是目录可读、可整体清理，目录搬家后不重建索引。不采用的原因：见提案第五节——会给 Core 的索引键增加一个输入，并让一个键承担两种语义，换来的只是化妆性的可读性。索引本来就可丢弃。

### 备选方案五：禁止外观被工作区覆盖

与"外观只跟人走"的直觉一致，也不可能出现"打开同事的仓库，字号莫名变成 11"。本次**未采用**：团队统一主题与字号是真实需求，IDEA 也有"Project"级配色方案。风险用"可见提示 + 一键改回 + 沿用钳制范围"来对冲，见提案第四节。

### 备选方案六：全局设置保持扁平标量结构

gpui 现在特意把 `Settings` 做成"扁平标量"，理由是最近项目这类高频写入不该污染用户手改过的设置文件。不采用的原因：那个具体问题已经被独立的 `recent-projects.json` 解决了，而扁平化的代价是每新增一个数组或对象就要多开一个文件。保留"逐键容错 + 未知键 round-trip"这两条真正的原则即可。

### 备选方案七：可共享文件必须被 Git 跟踪才生效

理由是"没被提交的东西就不算团队设置"，语义最干净。不采用的原因：用户改完设置会看到**毫无反应**，而原因（文件未被跟踪）在界面上不可见——这是最难排查的一类失败。改为"文件在就生效 + 把来源画出来"。

### 备选方案八：只做产品口径，不碰文件系统

即"Lithe 不 add、不提示，也不写任何忽略文件"，`.lithe/` 以未跟踪状态出现，是否提交交给用户。吸引力在于它完全不动用户的 Git 配置，与"Git 写操作只在用户显式动作时发生"零冲突。不采用的原因：这等于把"默认不共享"停留在文案上，而 `git add .` 是最常见的操作之一，一次误操作就把个人状态提交进了团队仓库。另有两条同族做法同样不采用：写进仓库顶层的 `.gitignore`（那是团队文件），以及加一个"共享项目配置"开关（同一个目录在两种模式下内容不同，排查问题前得先问开关状态）。

### 备选方案九：写进本机排除，但要求用户自己处理 `-f`

即采纳本机排除，但不提供"共享此项目的配置"动作。不采用的原因：被排除的文件用普通 `git add` 提交会被拒绝，用户要么知道 `-f`、要么手工编辑 `.git/info/exclude`——等于用一个新的知识门槛替换掉原来的"要记得提交"。

## 验收标准

- 三端读到同一份 `.lithe/project.json` 时得到同一个 `id`；把一个已存在 `id` 的项目目录改名后，本地历史与新会话仍然挂在同一个分桶上。
- 手工修改 `.lithe/run.local.json` 后无需重启应用即生效（外部改动被监听到）。
- 在设置里改主题后立刻强杀进程，重启后仍是新主题（退出路径已 flush）。
- 打包后的应用能从全局 `themes/` 目录读到用户安装的主题（而不是只从构建树读）。
- 在一个 Git 仓库里首次生成 `.lithe/` 之后，`git status` 不出现任何 `.lithe/` 条目，且仓库顶层 `.gitignore` 未被修改。
- 本机排除文件里只多出一行 `.lithe/`；用户原有的其他规则逐字不变，重复触发不会产生第二行。
- 执行"共享此项目的配置"之后：本机排除里的那一行消失，选中的可共享成员进入暂存区，全程不需要用户使用 `git add -f`。
- 在没有 `.git` 的目录里打开项目时不创建任何忽略文件，也不报错；在 worktree 或 submodule 里，排除文件写在 Git 自己解析出的位置（`git rev-parse --git-path info/exclude`）。
- 在从未共享过 `.lithe/` 的项目里，打开项目、改外观、写会话之后，界面不出现"尚未与团队共享""建议加入版本控制"之类的任何提示（默认静默）。
- 把 `.lithe/settings.json` 显式提交并重新打开项目后，界面把它标为团队设置；撤销跟踪后该提示随之消失。
- 外观覆盖的三种来源（团队设置 / 本项目的设置 / 个人覆盖）在界面文案上可以区分。
- 在一份手写的 `settings.json` 里加一个未知键，应用读写若干次之后该键仍然存在。
- `./scripts/verify-shared-contracts.sh` 通过，且新增的每个配置文件都有对应的 schema 与 fixture。

## 风险

- **自动写本机排除偏离了"Git 写操作只在用户显式动作时发生"这条既有约束**：它是本提案唯一的定向例外（理由与三条边界见提案第七节）。回退方式：如果这个例外不可接受，退到备选方案八不需要改动其他任何决策——层划分、文件名、身份方案都不依赖它。
- **gpui 写通道是新能力**：它直接推翻 `gpui/research/open-project-and-windows.md` 里"不要落 `.lithe/`"的现行结论。那份研究文档必须与本次改动同步更新，否则后来者会照着旧结论把写通道又拆掉。
- **"默认不共享"容易被误读成"已经共享"**：团队成员各自打开同一个仓库时，`.lithe/` 的内容互不可见，但每个人本机看到的效果都"很正常"。回退方式：文案与验收标准都按"默认静默、只在检测到已在共享时才提示"来设计；不要为了让功能"看起来生效"而改成默认提示或默认提交。
- **外观可被工作区覆盖**会带来"为什么我的字号变了"的困惑。提示与一键改回是**验收标准的一部分**，不是可选项；如果困惑度太高，退回"禁止工作区覆盖"是安全的（删掉 `.lithe/settings.json` 的外观段即可，不涉及数据迁移）。
- **未知键 round-trip 与嵌套结构**：嵌套对象内部的未知键需要在类型化解析之前、在原始 `Value` 树上做保留与回写，否则会在深层丢字段。这是实现上最容易做错的一处，必须有测试覆盖。
- **Core 契约为纯文档变换**：宿主落盘意味着"校验通过但写失败"这条路径必须被显式处理并回报给用户，不能像现在这样只打一行 `S1_*` 诊断。
- **项目身份统一**需要迁移期同时认新 id 和旧的路径哈希，否则用户会看到"历史还在但断点没了"这类半迁移状态。

## 适用范围

- `.agents/notes/`
- `gpui/crates/settings/`
- `gpui/crates/java/src/`
- `gpui/crates/workbench/src/`
- `rust/lithe-core/src/execution/`
- `rust/lithe-core/src/lsp/`
- `rust/lithe-core/src/git/`
- `shared/contracts/`
