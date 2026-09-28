# Agent 笔记：配置文档语义与工作区配置分层（第一批落地）

状态：已实现

## 先说结论

Lithe 的持久化配置现在分四层：**全局**（跟人走）、**项目可共享**（`.lithe/` 里不带 `.local` 的文件）、
**项目本机**（`.lithe/*.local.json`）、**派生物**（缓存与索引，可删）。设置文件与 `.lithe/project.json`
共用同一套**文档语义**：顶层 `version`、未知键原样保留、键表由 schema 派生、原子写、外部改动监听、
退出前补写。打开任何一个目录时 Lithe 会建立它的 `.lithe/project.json`（稳定 UUID 身份），且**在写之前**
先把 `.lithe/` 加进本机排除文件，所以它默认不与团队共享、也不出现在 `git status` 里。

开发者需要记住三件事：**加一个新的配置键不需要改任何回落路径**（键表由 schema 派生）；**手改配置文件不会
被吃掉**（未知键 round-trip）；**打开项目会产生一次磁盘写入**，其中包含对 `.git/info/exclude` 的一次写入
（这是对"Git 写操作只在用户显式动作时发生"的定向例外，理由见下）。

本文记录的是**已经落地并验证**的部分：文档语义、四层与文件形状、项目身份、默认不共享、打开即建、
主题目录与主题标识、**工具链五值落到项目本机层**。仍在推进的部分（外观的工作区覆盖层、会话状态）
不在本文范围。

配套阅读：本机排除写入的缺陷与"写完必须回读"的约束见
`.agents/notes/implemented/bug-fix/2026-09-27-exclude-write-must-be-read-back.md`。

## 问题

1. **配置只有一层**：gpui 的 `Settings` 是 19 个键全为全局作用域的扁平结构，没有项目层。项目 JDK、
   Maven 可执行文件、`settings.xml`、本地仓库这些**按项目变化**的值只能放全局，设置页文案还必须专门
   标注"作用范围是全局"。
2. **配置文件不能安全手改**：没有 `version`；未知键静默忽略（用户手写的字段会在下一次落盘时被删掉，
   不可逆）；不监听外部改动（改了不重启不生效）；退出时会丢掉防抖窗口里的最后一次改动。
3. **键表有第二份真源**：坏键回落路径是一张手写清单，新增键忘记登记就表现为"设置写得出、读不回"，
   而且**没有任何诊断**（阶段 14 实测踩到 `fontSize` / `tabSize` / `terminalDefaultShellId`）。
4. **`project.json` 无法容纳身份**：契约 schema 是 `additionalProperties: false`，没有 `id` 字段。
5. **主题目录写在构建树里**：`themes_dir()` 是 `CARGO_MANIFEST_DIR/../../themes`，打包成安装包后路径不存在，
   主题功能在发布版整体失效且不报错。主题标识还有两套（显示名与 id）。

## 决策

### 一、文档语义（设置文档与 `project.json` 共用一份实现）

通用原语在 `rust/gpui/crates/shared/src/document.rs`，两个使用方：设置文档
（`rust/gpui/crates/settings/src/persistence.rs` 委托它）与 `.lithe/project.json`。

| 语义 | 行为 |
| --- | --- |
| 顶层 `version` | 写回时写当前版本；文件声明的版本**更高**时转**只读**（内存里照常生效，但绝不覆盖用户的文件），并留 `document_version_newer` 诊断 |
| 未知键 | **原样保留**，含嵌套对象内部的键（写回前先合并上一次文件的原始对象） |
| 键表 | `known_keys::<T>()` 取 `T::default()` 的序列化结果；坏键回落路径遍历它 |
| 坏键 | 只回落**该键**并留 `bad_key` 诊断，其余键照用 |
| 读取 | **不创建文件**（文件不存在 = 全默认值） |
| 写入 | 原子写（`<name>.json.tmp` + rename） |

**给开发者的动作**：新增一个配置键时只改 `schema.rs`（字段 + `Default` + 序列化键名测试）与消费方；
**不要**去改 `persistence.rs`——那里已经没有第二份键名清单了。守卫测试
`any_single_broken_key_leaves_every_other_key_intact` 会逐个已知键喂非法值，任何没进入回落路径的新字段
都会立刻表现为不等。

### 二、四层与文件形状

| 层 | 位置 | 进 Git | 典型内容 |
| --- | --- | --- | --- |
| 全局 | 平台配置目录（Windows `%APPDATA%\Lithe\`） | 否 | 界面、编辑器、终端、语言、主题文件 |
| 项目可共享 | `.lithe/` 里不带 `.local` 的文件 | 是 | `project.json`、`settings.json`、`git.json`、`run/configurations.json`、`toolchains/requirements.json`、`maven/config.json`、`lsp/language-providers.json`、`.gitignore` |
| 项目本机 | `.lithe/*.local.json` | 否 | `run.local.json`、`maven.local.json`、`toolchains.local.json`、`session.local.json` |
| 派生物 | 平台缓存目录 | 否 | `language-servers/jdtls/<workspaceKey>/`、`run/classes/` |

**"可共享"是两条排除规则、其余默认可共享**：`*.local.json`（本机层）与 `*.tmp` / `run/classes/**`
（派生物）。所以新增**可共享**成员不用改判定代码；新增**本机**成员只要命名 `<name>.local.json`；
只有新增**另一种派生物**才要同改 `is_pruned_directory` 与 `is_shareable_member` 两处（各有测试守着）。

绝对路径**只允许**出现在本机层文件里——可共享文件按定义会被别人拉走。

### 三、项目身份：清单里的 UUID 是真源，路径哈希是回落

`.lithe/project.json` 的 `id`（UUID v4，首次生成时写入）是身份真源；没有它时回落到 Core 的路径身份
（`lsp.jdtWorkspaceKey` **不传指纹** = `normalized_workspace_identity` + SHA-256）。二者是主/备关系。
**没有 `id` 时拒绝写入**（`MissingId`）：typed 的 `id: null` 会在合并时盖掉文件里已有的 id，那是静默
数据丢失；拒绝发生在守卫之前，所以被拒的写入不会去动排除文件。

`ProjectManifest` 只建模 `id`；`defaultRunConfiguration` 等**别的生产者写的字段靠未知键保留逐字带过**——
不为没有消费方的字段建类型，那只会造出假契约。

### 四、默认不共享：让默认落在磁盘上

**默认不共享**，而且让它成为磁盘事实：写 `.lithe/` 之前，把 `.lithe/` 追加进**本机排除文件**
（`.git/info/exclude`）。**不碰仓库顶层的 `.gitignore`**——那个文件是团队的、会被提交。

- 位置由 Core 解析（`git.watchContext` 给出 `gitCommonDirectory`，worktree 感知），**不硬编码**
  `<root>/.git/info/exclude`：仓库可能是 worktree / submodule / bare。
- 用 Core 的 `excludePatterns`（literal、幂等、只追加）而不是 `exclude`：判据只有一条——**能不能用同一个
  literal 把我加的那一行精确删掉**；`exclude` 会 root-anchor 与转义，想删就得复现它的转义逻辑。
- "这个根不是 Git 仓库"用 Core 的稳定信号（错误码 `invalid_request` + 消息 `Not a Git repository`），
  静默跳过，不报错、不创建任何文件。
- 显式共享是它的精确逆操作：先移除那一行，再暂存**可共享成员**（`*.local.json` 与派生物绝不进暂存区）。
  没有可共享内容时**不动**排除行。

**⚠️ 这是一处偏离既有约束的定向例外**：仓库原来的口径是"Git 写操作只在用户显式动作时发生"，而
"打开即建"会**自动**写一次本机排除。边界限定为三点：只写 `LocalExclude` 目标、只写 `.lithe/` 一个条目、
只在缺失时追加。回退方式是让 Lithe 不写任何忽略文件、只保留"用户自己 `git add` 才共享"的产品口径。

**排除模式是未锚定的 `.lithe/`（已决定的取舍）**：Git 里不带前导 `/` 的模式在任何深度都匹配，所以把
`<repo>/subdir` 当项目打开时，那一行也会忽略仓库里其他嵌套的 `.lithe/`；共享其中一个会把其他工作区的
排除行一起去掉。不改成锚定写法的理由是：唯一正确的锚定形态是"仓库根 + 工作区相对路径"，那需要先问一次
Git（多一次 Core 往返）；而**只锚定到排除文件所在目录（`/.lithe/`）是错的**——那对"把子目录当项目打开"
的情形会去忽略 `<repo>/.lithe/` 而不是 `<repo>/sub/.lithe/`，等于在真正需要它的地方失效。**什么时候该改**：
一个仓库里同时存在多个 Lithe 工作区、且其中至少一个需要共享配置时。

### 五、打开即建（产品决策）

**打开任何一个目录就建立它的 `.lithe/project.json`，无条件。** 接线点是 `ShellWorkspace::new`——它是
所有"打开"路径的公共收口（全仓库只有两个调用点：`main.rs` 的启动根与 `rebuild_project_window` 的
`replace_root`；选择器、项目菜单里的最近项目、探针三条入口都汇到后者）。

- **顺序由 `save_project_manifest` 内部保证**（先确保不共享、再写清单）。调用方**不要**自己拼
  `ensure_project_dir_excluded` + 写文件，那会重实现一遍顺序保证。
- **失败绝不让打开项目失败**：只留 `S1_WORKSPACE_CONFIG` 诊断，窗口与项目照常。
- **非 Git 目录是正常情况**：守卫返回 `NotARepository`，清单照建——代价是打开任何目录都会多出一个
  `.lithe/`，这是明确接受的产品选择。
- 阻塞段（会起 `git rev-parse` 子进程）走 `background_spawn`，**不允许落在 UI 线程**。
- **唯一覆盖不到的路径**是 `OpenDestination::NewWindow`：它今天什么都不打开（多窗口整批暂缓），
  将来落地时同样走 `ShellWorkspace::new`，自动被覆盖。

### 六、主题目录与主题标识

运行期只读 `<配置目录>/themes/`；内置主题用 `include_str!` 编译进二进制、启动时**播种**到该目录
（已存在的不覆盖），再 `watch_dir` 它。打包后不再读构建树：`CARGO_MANIFEST_DIR/../../themes` 只剩
`include_str!`、播种源与测试三个用途。

**播种必须原子写**，原因是它与"已存在就不覆盖"这条规则合起来的后果：写到一半崩溃会留下截断的 JSON，
而下次启动看到"已存在"就跳过 → 该内置主题**永久损坏且用户没有线索**。原子写让最坏结果只剩一个
注册表不会读的 `.tmp`。**刻意不做**"已存在文件解析失败就重新播种"——那会把用户**手写坏**的主题悄悄
换成内置版，比留着更糟。

主题标识统一用 **id**（`lithe-dark`）：id 的来源是给主题文件加 `id` 字段（不从 `name` 派生——那等于
"改名就换 id"，正是改用 id 要避免的事）；没写 `id` 的主题按名字 slug 兜底。**老设置文件不迁移也能用**：
`ThemeIndex::canonicalize` 同时接受 id 与显示名（精确优先，最后做一次不区分大小写）。

### 七、工具链五值落到项目本机层

五个"跟着机器走"的值现在有了项目层：`javaHomePath` / `mavenExecutablePath` / `mavenJavaHomePath`
进 `.lithe/run.local.json` 的 `toolchain` 对象，`mavenSettingsPath` / `mavenLocalRepositoryPath` 进
`.lithe/maven.local.json`。优先级是 **项目本机 > 全局 `settings.json`（作为本机默认值）> 自动发现**。

三条给开发者的规则：

1. **优先级的唯一实现是 `ToolchainPaths::resolve(local, global)` 这个纯函数。** 设置页只做字段搬运
   （`resolve_overrides` 返回 `(值, 来源)`），界面按返回的**来源**如实标注。不要在任何地方再写一遍
   "项目优先"的判断——两份实现漂移的表现是"界面显示 A、语言服务用 B"，这是最难查的一类错。
2. **写侧按"有没有工作区"分流**：有工作区写项目本机层，没有才写全局默认；失败不假装成功
   （不置"已保存"）。
3. **旧路径只做文件级回落**：`.lithe/run.local.json` **不存在**时才去读旧路径；它存在就不再读旧的。
   不要做键级合并——同一份配置有两个来源会在用户改值时产生"改了没生效"。

`.lithe/.gitignore` 的 writer 也在这一批落地：三条规则（`*.local.json` / `run/classes/` / `**/*.tmp`）、
**只追加缺失规则、绝不重写、绝不删用户既有行**、幂等（命中时不改文件、不留 `.tmp`）。它要生效的时刻是
"共享那一刻"——在此之前整个 `.lithe/` 被本机排除挡着，它是惰性的。



## 考虑过的备选方案

### 只做产品口径、不写任何忽略文件
`git status` 里 `.lithe/` 以未跟踪状态出现，是否提交交给用户。吸引力是完全不动用户的 Git 配置、
与"Git 写操作只在用户显式动作时发生"零冲突。不采用的原因：`git add .` 是最常见的操作之一，
一次误操作就把个人状态提交进了团队仓库。

### 排除模式按仓库根锚定
语义更精确。不采用的原因见第四节：正确形态需要多一次 Core 往返，而只锚定到排除文件所在目录会让子目录
工作区失去保护。

### 未知键静默忽略（保持原状）
实现零成本。不采用的原因：会在下一次落盘时删掉用户手写的键，属不可逆的数据丢失。

### 保留手写逐键表（加第二份真源）
不采用的原因：阶段 14 已经用它制造过一次"写得出、读不回"且无诊断的静默失效。

### 内置主题用"内置目录 + 用户目录各装载一次"
不采用的原因：上游 `ThemeRegistry` 只支持**一个**目录——`watch_dir` 的 `themes_dir` 是覆盖写、
`reload()` 会 `themes.clear()`，且文件变更触发的重载**不回调 `on_load`**，所以"内存注入 + 目录装载并存"
必然在第一次文件变更后丢掉注入内容。

### 手写坏的文件就重新播种
不采用的原因：会把用户自己的（写坏的）主题悄悄换掉，比留着更糟；用户至少还能看到自己的文件。

### 打开时按需生成（识别出项目类型 / 用户点运行 / 显式保存设置）
不采用的原因：这是产品决策，维护者选择"打开即建、无条件"（原因是非 Git 目录之外的一切都由本机排除
挡住，不打扰版本控制）。

## 后果

**收益**：加新配置键不再需要记得改回落路径；手改配置文件不会被吃掉；外部改动不重启即生效；退出不丢最后一次
改动；主题在打包后仍然可用，且主题改名不断用户引用；`.lithe/` 默认不进 `git status`；`.lithe/` 这个名字
第一次有了具名路径真源（此前整个仓库里是散落的字面量）。

**代价**（都已在上面写明理由）：

- 打开任何目录都会多出一个 `.lithe/`（含非 Git 目录），并且会产生一次磁盘写入，其中包含对
  `.git/info/exclude` 的追加——这是对"Git 写操作只在用户显式动作时发生"的定向例外。
- 排除模式未锚定 → 同一仓库里多个工作区的 `.lithe/` 会互相忽略。
- 用户主题目录里的内置主题副本不随程序升级更新（删掉再重启即重新播种）。

## 验证

- `cargo test --workspace`（gpui）：**393 通过 / 0 失败**（本批从 334 起；settings 125 → 144、
  shared 从 3 到 52、terminal 8、workbench 与其余 191）
- **工具链优先级的端到端**（工作区之外的仓库；全局层设 `javaHomePath=D:\…global-jdk`，
  项目本机层设 `javaHomePath=D:\…project-jdk`）：

  ```text
  S1_SETTINGS wiring=java_toolchain java_home_path=D:\lithe-e2e-project-jdk maven_settings_path=D:\lithe-e2e-global-settings.xml project=3 global=1 unset=1
  ```

  **项目本机值赢了全局值**，只在全局设的 `mavenSettingsPath` 正常兜底，来源计数 `project=3 global=1
  unset=1` 与界面标注同源。同一趟还证明：预设的 `.lithe/run.local.json` 读完后**逐字未变**
  （读侧不写文件、不丢键、不升版本）。
- `cargo check --workspace --all-targets`（gpui）：**exit=0**
- `./.agents/skills/write-stable-tests/scripts/verify-test-stability.ps1`：**通过**
- `./scripts/verify-agent-notes.sh`：**通过**
- **端到端（Git 仓库）**：临时仓库 `git init` + 一次提交 → 启动应用 → 日志
  `excluded … pattern=.lithe/` → `manifest_saved … bytes=67 exclusion=Ensured` →
  `project_id_created` → `shell_identity … source=manifest`；`.lithe/project.json` 存在、`version=1`、
  `id` 是 UUID v4；本机排除文件里 `.lithe/` **恰好一行**；`git status --porcelain` **完全为空**。
  二次启动：`id` 与 mtime **都不变**、不重复创建。
- **端到端（非 Git 目录）**：`GIT_CEILING_DIRECTORIES` 挡住向外层查找后，
  `manifest_saved … exclusion=NotARepository`，清单照建、**没有**创建 `.git` 或排除文件。
- **端到端（主题）**：播种 `created=7 kept=0 failed=0` → `watching dir=<新目录>` →
  `resolved id=nord-light name=Nord Light` → `applied=Nord Light dark=false`；目录里无残留 `.tmp`；
  二次启动 `created=0 kept=7`；设置文件里的显示名**未被强制迁移**。

## 适用范围

- `rust/gpui/crates/settings/`
- `rust/gpui/crates/shared/src/document.rs`
- `rust/gpui/crates/shared/src/workspace_config/`
- `rust/gpui/crates/workbench/src/workspace.rs`
- `rust/gpui/crates/app/src/main.rs`
- `rust/gpui/themes/`
- `shared/contracts/project-manifest-v1.schema.json`
- `rust/gpui/PLAN.md`
- `rust/gpui/HANDOFF.md`
