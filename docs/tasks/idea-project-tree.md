# IDEA 式项目树（整形片）

> 本文件是**设计稿 + 任务清单**。判据（2026-10-07 维护者定调）：只做 **Project 模式**的
> **整形**（包链压缩 + 源码根/包标记 + git 装饰与组件库一致），**本片不换树控件**；
> 控件从 gpui-kit 的 `Tree` 换成 `lithe_gpui::lists::Tree` 是**下一片**。
>
> 决策理由写 `.agents/notes/`（实施时与代码同提交）；本文写"改成什么样、怎么验"。
> 界面逐区域对照见 `rust/lithe-gpui/UI-MAP.md`。

## 0. 范围

- ⛔ **不改 `rust/lithe-core/`**（本分支约束，见 `docs/tasks/gpui-core-first.md` 顶部）。
- ✅ 落点全在 `rust/lithe-gpui/`：`crates/explorer`（整形 + git 装饰）、`crates/lithe-gpui`（git 装饰的单一真相源）。
- 只做 **Project 模式**（目录树形态）。

## 1. 真源规则（照抄 macOS，不用 Maven）

规格真源：`macos/Sources/Lithe/Theme/LitheIcons.swift`、
`macos/Sources/Lithe/Core/Rust/RustCoreBridge.swift`、
`macos/Sources/LitheCoreContracts/Workspace/WorkspaceModels.swift`。

| 规则 | 真源 | 内容 |
| --- | --- | --- |
| 源码根判定 | `LitheIcons.isSourceRootDirectory`（`:225-232`），名字集合 `:150-151` | 目录名 ∈ `{java, kotlin, scala, groovy}` ∪ `{resources, res, webapp, static}`，**且父目录名 ∈ {main, test}**。纯路径布局 ⇒ **不依赖 Maven**，非 Maven/Gradle 项目同样生效 |
| 包名合法 | `LitheIcons.isValidPackageName`（`:256-262`） | 首字符是字母或 `_`，其余是字母/数字/`_`（所以 `META-INF` 是普通文件夹，不是包） |
| 包链压缩 | `RustCoreBridge.swift` 的 `makeFileNode(at:insideSourceRoot:)`（`:4079-4117`） | **只在源码根之下**压缩；`唯一子节点是目录`时一路压到分叉或含文件的目录；**源码根本身不参与压缩**；被压掉的祖先路径记入 `collapsedAncestorPaths` |
| 源码根内的传播 | 同上 | `inside = 父inside \|\| 自己是源码根`；源码根本身 `isInsideSourceRoot = false`（真源如此） |
| 行标签 | `WorkspaceModels.swift` 的 `FileNode.name`（`:29-34`） | 有压缩祖先时 = `(各祖先末段… + 本节点末段).join(".")`，例如 `com.alibaba.nacos.ai` |
| 目录图标 | `LitheIcons.directoryKind`（`:234-253`） | ① 名字在 excluded 集合 ⇒ 排除图标；② `java/kotlin/…` 且父是 `main`/`test` ⇒ **sourceRoot 图标**；③ `resources/…` 且父是 `main`/`test` ⇒ 资源根图标；④ `isInsideSourceRoot && 合法包名` ⇒ **package 图标**；⑤ 其余普通文件夹 |
| 展开状态 | `macos/Sources/Lithe/Views/Workspace/ProjectSidebarView.swift`（`:536-540`） | 展开/折叠必须**连同 `collapsedAncestorPaths` 一起**处理，否则父目录开关与显示的行对不上 |

图标资产在仓库里已有：`rust/lithe-gpui/assets/icons/idea/nodes/{sourceRoot,package}.svg`（+ `_light`）。

## 2. 落点与架构

**放 `crates/explorer`，新增纯函数模块 `src/project_tree.rs`。**

- 整形输入（`Snapshot` 的 `TreeNode`、`GitTreeStatus`）本来就住在 `explorer`；放别处要反向依赖或重新序列化。
- 不新建 crate。

模块边界（每个单位只做一件事、可单独测）：

| 单位 | 职责 | 依赖 |
| --- | --- | --- |
| `project_tree::is_source_root(path) -> bool` | 路径布局判定源码根 | 无 |
| `project_tree::is_valid_package_name(name) -> bool` | Java 包名段判定 | 无 |
| `project_tree::is_excluded_dir(name) -> bool` | 排除目录名单（照真源 `:146`） | 无 |
| `project_tree::shape(root: TreeNode) -> TreeNode` | 产出一棵"压缩 + 标记"的新树 | 上面三个 |
| `project_tree::display_name(node) -> String` | 行标签（点号连接） | 无 |
| `project_tree::icon_kind(node) -> FolderIcon` | 目录图标档位（排除/源码根/资源根/包/普通） | 上面几个 |

`TreeNode`（`crates/explorer/src/model.rs`）加两个字段与一个方法：

- `collapsed: Vec<String>`（默认空）：被压缩掉的祖先目录路径，**工作区相对、与 `path` 同口径**（真源用绝对路径；本仓既有口径是相对 —— `git.status` 装饰、Reveal、右键菜单都按相对路径走）；
- `inside_source_root: bool`（默认 false）；
- `display_name(&self) -> String`。

**目录图标的渲染通路**：现在行里的目录图标是
`FileIcon::lucide(IconName::FolderOpen / FolderClosed).with_theme_folder(...)`（`explorer_view.rs` 的行渲染）。
本片新增 `project_tree::FolderIcon` 档位（`Excluded` / `SourceRoot` / `ResourcesRoot` / `Package` / `Plain`），
行渲染按档位取值：`Plain` **沿用现有 folder 图标不动**，其余走 expui 资产通路
（`lithe_gpui_shared::icons::idea_icon_image`）指向 `assets/icons/idea/nodes/{sourceRoot,resourcesRoot,excludeRoot,package}.svg`。
这 4 组资产常量需要在 `crates/shared/src/icons/idea.rs` 补上（该文件现在只有 `folder` 一组 light/dark）。

## 3. 数据流（本片）

```text
workspace.snapshot ─► Snapshot { root: TreeNode }
                        │
                        ▼  project_tree::shape(root)        ← 新增：压缩 + 标 isInsideSourceRoot
                   TreeNode（children 变少；节点带 collapsed / inside_source_root）
                        │
                        ▼  build_tree_items(...)            ← 既有（行预算/B20 的广度取用不动）
                   Vec<TreeItem> ─► 现在的 gpui-kit 树
```

- 行文本用 `display_name()`；目录图标按第 1 节的 ①–⑤ 选择；
- git 装饰仍来自 `GitTreeStatus`，但词汇与渲染改用组件库（第 4 节）。

## 4. git 装饰：单一真相源落组件库

维护者 2026-10-07 定：**项目树与 `lists::FileTree` 的 git 效果必须一致**，真相源放组件库，宿主消费。

改动：

1. `lithe_gpui::lists::GitStatus`（`crates/lithe-gpui/src/lists/files.rs`）成为唯一词汇，**补 `Copied`**；`letter()` / `tone()` 由 `pub(crate)` 改 **`pub`**。
2. 把宿主现有的**目录汇总**搬进库：`GitStatus::dominant(statuses) -> Option<GitStatus>`，优先级 **Conflicted > Deleted > Added > Renamed > Copied > Modified**。
3. 库的 `tone()` 采用**宿主现有配色**（真源里只有编辑器 gutter 的 `gitModified` 一个蓝，项目树这套是宿主 U24 已评的），逐条为：
   `Modified → Accent`、`Added | Copied → Success`、`Deleted → Danger`、`Conflicted → Warning`、`Renamed → Info`、`Untracked → Success`。
4. 宿主 `crates/explorer/src/model.rs`：删 `GitTone` / `GitDecoration` / `git_tone` / `display_status`（以及 `GitTone::color`、`priority`），改为
   - **解析留宿主**：把 Core `git.status` 的 porcelain 码解析成 `lists::GitStatus`（含冲突 `!`、untracked、`R`、`C`）；
   - **渲染用库**：行字母取 `letter()`、着色取 `tone()`；
   - 目录汇总取 `GitStatus::dominant()`。
5. `crates/lithe-gpui/src/lists/files.rs` 的 `FileTree` 与它的单测/画廊 demo 跟着新 `tone()` 改。

### 可见变更（只有这些，其余逐像素不变）

1. **重命名 / 移动**的行：`magenta_light` → **Info**（库的 `Tone` 没有 magenta 档；要保留就得给库加档，代价大，已接受）。
2. 画廊 `FileTree` 演示：`Modified` 由 Warning → Accent、`Conflicted` 由 Danger → Warning（**只在画廊**，宿主不变）。
3. untracked 仍按宿主现状映射成 `Added`（字母 `A`、绿色）；**不使用**库的 `Untracked`/字母 `U`，避免与冲突的 `!` 混淆。

## 5. 测试

纯函数单测（`crates/explorer/src/project_tree.rs` 的 `#[cfg(test)]`）：

- 压缩：`src/main/java/com/a/b/C.java` ⇒ 出现一行 `com.a.b`；**遇文件停**；**只在源码根下压**（`docs/a/b/c` 不压）；**源码根本身不压**；包名不合法（`META-INF`）不压。
- 源码根判定：`src/main/java` ✓、`src/main/kotlin` ✓、`src/main/resources` ✓、`src/java`（父不是 `main`/`test`）✗、顶层 `resources` ✗。
- `inside_source_root` 传播；`icon_kind` 的 ①–⑤ 各一例。
- `display_name()`：有压缩祖先 ⇒ 点号连接；无 ⇒ 原名。

git 装饰：

- 库内：`dominant()` 的优先级各有断言；`letter()`/`tone()` 表逐条断言。
- 宿主：porcelain 码（` M`、`A `、`D `、`??`、`R `、`C `、冲突 `UU`/`AA`/`DD`）→ `lists::GitStatus` 的映射表。

回归：`cargo test -p lithe-gpui-explorer`、`-p lithe-gpui`（库，含画廊）、`-p lithe-gpui-workbench`；`cargo fmt --check`；`node rust/lithe-gpui/tools/check-ui-px.mjs`。

## 6. 不在本片（已登记，另开切片）

- **控件换 `lithe_gpui::lists::Tree`**（`explorer_view.rs` 的多选/右键菜单/Reveal/⌘C/键盘要重新接线）。
- `Packages` 拍平模式、`Project Files` 文件管理器模式。
- `Compact Middle Packages` **开关**（宿主现在没有"树设置"落点；本片按默认开）。
- `文件修改`标（未保存 / 磁盘变更）—— 需先定数据源（编辑器脏标记 vs mtime）。
- 「项目视角 / git 视角」切换。

## 7. 验收

1. 打开一个标准布局的 Java/Maven 工程，左栏项目树在 `src/main/java` 之下出现**点号连接的包行**（如 `com.example.app`），且只在源码根下压缩。
2. `src/main/java` / `src/main/resources` / 源码根下的包目录各自显示对应图标（sourceRoot / 资源根 / package）。
3. 改动文件所在目录有汇总装饰；**同一状态在两处（项目树 / 画廊 FileTree）字母与颜色一致**。
4. 第 4 节列出的三条可见变更之外，项目树的观感与改动前一致。
5. 上述测试与守卫全绿。

## 8. 参考资料

- `docs/tasks/gpui-core-first.md`（任务表与范围约束）、`rust/lithe-gpui/UI-MAP.md`（区域对照）。
- 真源：`macos/Sources/Lithe/Theme/LitheIcons.swift`、`macos/Sources/Lithe/Core/Rust/RustCoreBridge.swift`、`macos/Sources/LitheCoreContracts/Workspace/WorkspaceModels.swift`。
- Core 契约（本期不消费，仅备查）：`rust/lithe-core/src/protocol/contracts.rs` 的 `MavenSourceRootKind`。
