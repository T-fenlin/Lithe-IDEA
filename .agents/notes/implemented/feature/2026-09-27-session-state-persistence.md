# Agent 笔记：会话状态持久化与恢复（`.lithe/session.local.json`）

状态：已实现

## 先说结论

关掉项目再打开时，编辑器原来打开的文件、当前文件、左侧栏可见性、左右工具窗的当前视图与可见性
都回来了。这些状态存在 **`.lithe/session.local.json`**（项目本机层：不进 Git、可随时删除），
文档 `version = 1`，路径一律是**工作区相对路径**。

开发者需要记住四条：**只存今天真的有宿主的字段**（光标、滚动、展开的树节点、面板尺寸、断点在 gpui
今天都没有可读的值，所以一个都没建模）；**恢复阶段跳过坏路径而不是报错**（文件被删、改名、跑到根外
都只是"少开一个标签"）；**保存前先过本机排除守卫**（所以它天然不出现在 `git status` 里）；
**`replace_root` 之前必须 flush**（换项目会把旧外壳连同内存里的会话一起丢掉）。

## 问题

1. **跨会话什么都不记得**：gpui 今天只持久化三样东西（全局 `settings.json`、
   `recent-projects.json`、JDT 索引缓存）。关掉再打开同一个项目，打开的文件、当前文件、
   侧栏与面板状态全部归零，每次都要重新点一遍。
2. **会话状态没有落点**：设计 Note 第八节把"会话恢复"整块指给了 `.lithe/session.local.json`，
   但那只是目标形状——里面还列着光标与滚动、展开节点、面板尺寸、断点这些项。
3. **状态来源分散在两个实体上**：打开的文件与当前文件在 `EditorPane`，侧栏与左右面板在
   `ShellWorkspace`。谁负责收集、谁负责落盘、什么时候落盘，都没有既定答案。
4. **会话文件是"会被删、会被移、会被切项目"的东西**：文件被删是很常见的（用户清理仓库），
   切项目会重建整个外壳，退出又可能落在防抖窗口里——每一条都能变成丢数据或崩溃。

## 决策

### 一、只存今天真的有宿主的字段（其余明确不建模）

| 字段 | 今天的持有者 |
| --- | --- |
| `openFiles` | `EditorPane::buffers`（顺序 = 标签栏顺序） |
| `activeFile` | `EditorPane::active` 的**下标** |
| `leftSidebarVisible` | `ShellWorkspace::left_sidebar_visible` |
| `topActivityView` / `bottomKind` / `bottomVisible` / `rightView` / `rightVisible` | `ShellWorkspace` 的同名字段 |

**今天没有宿主、因此一个字段都没建的**（不是"以后再说"，是今天真的没有值可读）：

- **光标与滚动位置**：只在 `EditorState` 内部，编辑器没有读出口；
- **展开的树节点**：`rust/lithe-db-gpui/crates/explorer` 不记录展开态；
- **面板 / 分栏尺寸**：gpui 侧没有可拖动分隔条那套状态（外壳里的宽度全是常量）；
- **断点与监视表达式**：gpui 侧没有调试器。

给开发者的话：**不要为"将来会有人读"的字段先建契约**。写进去、没人读、还要为它写测试，
就是一份假契约；等宿主出现时再加字段，代价只是一次小迁移。逐条理由写在
`rust/lithe-db-gpui/crates/shared/src/workspace_config/session.rs` 的模块文档里。

### 二、路径存工作区相对路径，根之外的文件根本不写进文档

`openFiles` 里是 `alpha.txt`、`src/main.java` 这样的**工作区相对路径 + `/` 分隔**。三条规则：

- 不在工作区根之下（`../` 逃逸、`C:/windows/win.ini` 这类盘符路径、空串）→ **不写进文档**
  （`is_plain_relative` 的三类拒绝，测试 `plain_relative_paths_reject_escapes`）；
- 磁盘上不存在的文件 → 也不写（写的是"现在能存什么"，测试 `missing_files_are_not_recorded`）；
- 回到绝对路径时按平台分隔符还原（测试 `resolving_a_nested_relative_path_uses_the_platform_separator`）。

**为什么本机层也不许出现机器路径**：这份会话在同一条路径被移动、挂载点变化之后仍然要能用；
而且绝对路径会把用户目录泄露进日志与文档。

### 三、`MAX_OPEN_FILES = 40`，活动文件无条件保留，下标在截断之后算

截断规则（顺序不能改）：

1. 按标签顺序留下**前 `MAX_OPEN_FILES - 1` 个**（= 前 39 个）；
2. 当前活动文件若不在那一段里，**追加到最后**；
3. **在截断之后的列表上**再算 `activeFile` 的下标。

于是文档里的条数恒 ≤ 40，而且**当前文件任何情况下都不会被丢掉**——"只开了一个第 40 号标签"
的会话恢复之后，当前文件必须还在。下标如果在截断**之前**算，被截掉的那些项会把下标顶到列表之外。
守卫测试是 `the_open_file_list_is_bounded_and_keeps_the_active_file`（三种情形：活动文件在窗口内 /
在窗口外 / 同一路径重复出现）。`MAX_OPEN_FILES = 40` 这一档只对应"一次会话里的标签数"这一量级，
目的是让文档**有界**（长会话不该让它无限增长），不是省字节。
外壳侧另有一条测试（`the_bound_comes_from_the_shared_document_layer`）钉住"上界只有一份定义"。

### 四、坏路径跳过而不崩：判定放在恢复阶段

读取（`load_session`）只把文档解析成 `SessionState`，**不查磁盘**。文件被删 / 改名 / 被移到根外
是恢复阶段才知道的事，所以由 `resolve_session_files` 逐条判定并给出结论：

| 情况 | 行为 | 诊断 token |
| --- | --- | --- |
| 在根之下、磁盘上存在 | 还原成绝对路径，**保持文档里的顺序** | — |
| 在根之下、磁盘上不存在 | 跳过这一条，其余照常恢复 | `reason=missing` |
| `../` 逃逸、盘符路径、空串 | 跳过 | `reason=outside_root` |

**跳过而不是报错**：用户删掉一个文件之后，界面上少开一个标签是合理结果；弹一个错误对话框才是
不合理的结果。恢复失败的界面本身也不残缺（其余文件照常打开）。

### 五、坏 JSON 退化、版本过新拒绝覆盖

| 情况 | 行为 |
| --- | --- |
| 文件不存在 | 全默认值，**不创建文件**（"读取不得创建文件"是设计 Note 的硬要求） |
| 坏 JSON / 不是对象 | 退化成"没有会话" + 一条 `session_diagnostic invalid_json …`，不 panic |
| 某一个键类型坏了 | 那一个键回落默认值，其余照用（`shared::document` 的逐键路径） |
| 版本比本程序新 | **能读**（恢复不因为版本新就丢掉整个会话），但**拒绝覆盖**：`save_session` 返回 `SessionError::ReadOnly`，日志 `session_save_skipped reason=document_version_newer` |
| 未知键 | 写回时逐字保留（靠 `document::previous_object` 从磁盘现读上一版文档） |

版本判据放在**写之前**：先读一次上一版文档，同一次读结果同时用于"判只读"与"保留未知键"。
守卫测试 `a_newer_document_version_is_refused_instead_of_downgraded` 连"被拒绝的写入不得改动文件、
也不该动排除文件"一起断言。

### 六、保存前先过本机排除守卫（与 `project.json` 同一条顺序保证）

`save_session` 内部先调 `sharing::ensure_project_dir_excluded`，再原子写文件，最后维护
`.lithe/.gitignore`（第二道闸）。**顺序由被调函数保证，调用方不要去拼**
`ensure_project_dir_excluded` + 写文件——那样会重实现一遍顺序保证，而漏掉的那次在
`git add .` 下就是"个人会话被提交进团队仓库"。

会话文件因此天然不进 `git status`（端到端里 `git status --porcelain` 为空）。非 Git 项目是正常情况：
守卫返回 `NotARepository`，会话照写。

### 七、写入时机：两处硬保证 + 一段防抖

| 时机 | 走哪条 | 为什么 |
| --- | --- | --- |
| 会话内容变化（开 / 关标签、切标签、切视图、收起侧栏） | 300ms 防抖 + **代数** | 连点只写一次；与设置文档同一个 300ms |
| **切换项目** | `replace_root` **之前** `flush_session(true, ..)`（同步、强制） | `replace_root` 会丢掉旧 `Root` → 旧 `ShellWorkspace`，连带丢掉它的 `SessionTracker`（内存里那份会话） |
| **应用退出 / 关窗** | `on_app_quit` 钩子 → `flush_session(true, ..)` | 防抖窗口里那一次改动不能丢；与设置文档的 `install_quit_flush` 同一口径 |

- **`replace_root` 之前先 flush 是硬顺序**，而且必须在调用 `replace_root` **之前**（不能在它的闭包
  之后）：闭包一执行，`self` 指向的外壳已经被换掉了。这是"换项目丢改动"唯一的堵点。
- **防抖用代数而不是计时器**：防抖状态机是 `lithe-db-gpui-settings` 已有的 `DebounceState`（纯值、
  可确定性单测），计时器由外壳在 `Context<Self>` 上排；过期代数醒来什么都不做
  （`only_the_last_debounce_generation_writes`）。换掉旧任务同时取消旧唤醒——gpui 的 `Task`
  一 drop 就取消。
- **内容没变时一个字节都不写**：`plan` 先比对"上一次成功落盘的内容"（`plan` → `Unchanged` 时
  连计时器都不排）；这条保证让重绘、切到同一个标签都不会变成写盘。
- **不阻塞 UI 线程**：防抖睡在后台执行器上；两处 `flush` 是**离散动作**（换根、退出），
  写一份几百字节的 JSON 再 rename，与设置文档的 `flush_pending` 同一条量级与理由。

### 八、有意留的快判：`plan` 用"打开的文件 + 当前文件"快照，不每次按键都 stat

`SessionTracker::plan` 记住上一次看到的 `(打开的文件, 当前文件)`；这一对没变时直接返回
`Unchanged`。它挡的是**每帧级别的观察**：编辑区的观察回调在光标移动时也会跑（一次按键一次），
而 `session_from_paths` 要对每个打开的文件各做一次磁盘存在性检查。
`plan` 自己也不直接写盘——唯一的落盘点是 `write()`（防抖到期与 `flush` 都汇到那里），
所以 "把编辑区此刻的状态收进文档"那一步（`plan`）必须先跑再 `flush`。

## 考虑过的备选方案

### 不做会话恢复
零风险、零新增文件。不采用的原因：这是维护者确定的"配置分层"最后一项落地内容，
而且"上次打开的文件"是编辑器最基础的体验之一。

### 恢复时对坏路径报错（对话框 / 状态栏提示）
用户明确知道"有一个文件没恢复"。不采用的原因：用户自己删掉一个文件是很常见的事，
把它变成一个需要用户处理的错误是错配严重度；逐条 `session_skipped` 诊断足够排查。
后果是界面上"少开一个标签"是静默的——这是有意接受的代价。

### `activeFile` 存路径而不是下标
读起来更直观、不依赖列表顺序。不采用的原因：那会与 `openFiles` 重复一份数据，
两份一旦不一致就没有仲裁者。下标配一条三步判据（过滤 → 截断 → **再**算下标）反而有确定答案。

### 上界截断时不保留活动文件（直接取前 40 个）
实现最简单。不采用的原因："只开了一个第 40 号标签"的会话恢复之后当前文件会消失，
而当前文件正是会话恢复最该保住的那一项。代价是条数在 39 与 40 之间不固定，可接受。

### 每次按键都 stat / 每次编辑回调都重算文档
能保证文档永远反映"这一瞬间的磁盘真相"。不采用的原因：正文编辑每次按键都会触发观察回调，
逐个打开文件做磁盘检查会把它变成每帧级 I/O。快判的代价是"文件在外部被删、用户又没动过标签
就退出"时文档里可能留着那条路径——恢复阶段照常跳过它，**用户可见行为不受影响**。

### 恢复阶段对坏路径也照开（交给编辑器报错）
少一层过滤。不采用的原因：那会把"文件不存在"变成一个用户可见的打开失败，
而会话恢复的语义是"尽量把界面还原到上次的样子"。

### 会话落盘也走 300ms 防抖的异步写（不做强制 flush）
代码更少。不采用的原因：换项目与退出是**离散动作**，异步写的后果是"那一刻的写入来不及完成"，
等于新增一条丢数据路径。两处 `flush` 保持同步强制。

### 存绝对路径（本机层不跨机器共享，看起来无所谓）
实现最简单（不需要工作区相对化）。不采用的原因：见第二节——同一份会话在根被移动后要能用，
而且绝对路径会泄露用户目录。

## 后果

**收益**：关掉再打开项目，界面回到上次的样子；会话文件不进 `git status`（本机排除守卫在写之前跑）；
坏路径 / 坏 JSON / 版本过新都有明确且不打断用户的处理；上界保证了文档有界而当前文件**永不丢失**；
文档语义（version / 未知键 / 原子写 / 读不建文件）全部复用 `shared::document`，没有第二份实现。

**代价与未做到的部分**（如实登记）：

- **"关窗退出前 flush"没有端到端取证**：无人值守环境只能强杀进程，而强杀**不走** `on_app_quit`。
  替代证据是探针**显式** `flush_session(true, …)` 之后磁盘上确实有文件，且那两处 `flush` 与设置侧
  已验证过的 `flush_pending` 是同一形状。真机上"关窗再打开"这一条没有在本机取证。
- **会话文件没有 watcher**：外部手改 `.lithe/session.local.json` 要重新打开项目才生效
  （与工作区设置文件同一条下限）。
- **面板尺寸 / 断点 / 展开节点今天真的没存**：不是"忘了"，是 gpui 侧没有宿主（见第一节）。
  将来补这些字段时要先让宿主出现，再改文档模型——不要先加字段。
- **有意留的快判有代价**：外部删掉一个打开中的文件、用户又没动过标签就退出时，
  文档里可能留着那条路径（恢复阶段跳过它，用户可见行为不受影响）。
- **`--session-probe` / `--session-assert` 是验证入口，不是产品能力**：它们与 `--menu-probe`
  同一条口径（走的是与用户操作相同的那条迁移，被绕开的只有鼠标点击那一段）。

## 验证

- `cargo test --workspace`（gpui）：**423 通过 / 0 失败**（基线 407；本批新增 16：`shared` 12 +
  `workbench` 4）。**父代理独立复跑得到同一数字。**
- `cargo check --workspace --all-targets`：**exit=0**（只剩两条既有 `dead_code` 警告）。
- `./.agents/skills/write-stable-tests/scripts/verify-test-stability.ps1`：通过。
- `node scripts/verify-agent-notes.mjs`：通过（当时 38 篇）。
- `node rust/lithe-db-gpui/tools/extract-locale.mjs --check`：报"产物与真源一致"。
- **规则对应的测试名**（`rust/lithe-db-gpui/crates/shared/src/workspace_config/session.rs` 与
  `rust/lithe-db-gpui/crates/workbench/src/session.rs`）：`loading_a_missing_session_does_not_create_it`、
  `broken_json_degrades_to_no_session_with_a_diagnostic`、
  `saving_a_session_lands_under_the_excluded_directory`、`a_newer_document_version_is_refused_instead_of_downgraded`、
  `resolving_files_skips_missing_and_outside_paths_in_order`、`files_outside_the_root_are_not_recorded`、
  `missing_files_are_not_recorded`、`the_open_file_list_is_bounded_and_keeps_the_active_file`、
  `a_saved_session_round_trips_through_disk`、`a_newer_document_is_still_readable`、
  `plain_relative_paths_reject_escapes`、`resolving_a_nested_relative_path_uses_the_platform_separator`、
  `an_unchanged_document_is_not_written`、`a_newer_document_is_never_written`、
  `only_the_last_debounce_generation_writes`、`the_bound_comes_from_the_shared_document_layer`。
- **GUI 端到端**（实现代理留下脚本、父代理用当前构建的 exe 独立重跑；工作区外 exe 副本 +
  工作区外真 Git 仓库）：
  1. 打开三个文件、第二个设为当前、隐藏侧栏 → 落盘 `openFiles[alpha,beta,gamma]`（**相对路径**）+
     `activeFile=1` + `leftSidebarVisible=false` + `version=1`，且 `git status --porcelain` **为空**；
  2. 重启 → `open=3 active_index=Some(1) active=…\beta.txt left_sidebar_visible=false`；
  3. 删掉 `beta.txt` 后重启 → `open=2 active_index=Some(1) active=…\gamma.txt`，
     `session_skipped path=beta.txt reason=missing`——**跳过被删的那个、无 panic、其余照常恢复**；
     收尾 `SESSION E2E OK`、`residual_lithe_processes=0`。
- **release 打包**（同批）：`cargo build --release` exit=0、11m20s、产物 49 MB；PE 子系统位
  debug=**3（控制台）**、release=**2（GUI/无控制台）**；release 版在**工作区外**的真 Git 项目上
  冒烟通过（`.lithe/project.json`、`.lithe/.gitignore`、排除文件恰好一行、`git status` 为空）。
- **环境坑（实测记录，两条都已写进 `rust/lithe-db-gpui/PLAN.md` §8.13 与 `rust/lithe-db-gpui/HANDOFF.md`）**：无人值守启动里
  `window.on_next_frame` **一次都不跑**（窗口在、`MainWindowHandle` 非零、`Responding=true`），
  所以探针改成窗口建好之后立刻同步执行；另外**不要**在 `application().run(..)` 之后加驻留循环，
  它会把 `cx.spawn` 的建窗口任务整个饿死。

## 适用范围

- `rust/lithe-db-gpui/crates/shared/src/workspace_config/session.rs`
- `rust/lithe-db-gpui/crates/shared/src/workspace_config/paths.rs`
- `rust/lithe-db-gpui/crates/shared/src/workspace_config/sharing.rs`
- `rust/lithe-db-gpui/crates/shared/src/document.rs`
- `rust/lithe-db-gpui/crates/workbench/src/session.rs`
- `rust/lithe-db-gpui/crates/workbench/src/workspace.rs`
- `rust/lithe-db-gpui/crates/editor/src/editor_view.rs`
- `rust/lithe-db-gpui/crates/app/src/main.rs`
- `rust/lithe-db-gpui/crates/settings/src/persistence.rs`
- `rust/lithe-db-gpui/HANDOFF.md`
