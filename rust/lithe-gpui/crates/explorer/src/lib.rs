//! 侧栏 · 项目树（资源管理器）。
//!
//! 这一块对应 Windows 前端的 `FileExplorerPane` → `FileExplorerTree`：**只有树体**（虚拟滚动、
//! 任意深度、目录展开/折叠、点击文件打开）。规格一律取
//! `windows/tauri/src/features/file-explorer/` 的源码，每个数值都带 `文件:行号`。
//!
//! ⚠️ **2026-09-27 按维护者要求删除**：真机 `FileExplorerPane` 的第一层 `SidebarHeader`
//! 「项目」头部行（高 32、标题 + 搜索按钮 + 清空按钮 + 偏好按钮）与它下面的树内搜索行（高 28，
//! 原实现是一整行内联输入框）都已删除，所以本模块渲染出来的顶部就是文件树本身；搜索入口改由
//! 左活动栏的「搜索」项承担（`lithe-gpui/crates/workbench/src/activity_bar.rs`）。这两条与 Windows
//! 真机的差异原先登记在下面的未实现清单第 10 条（偏好下拉菜单）与第 11 条（搜索浮层），随实现
//! 一并删除（原第 12 条顺延为第 10 条）。图标、颜色、主题一律没动。
//!
//! 两层结构：
//!
//! 1. **树体**：gpui 的 `Tree`（表体是 `uniform_list`）—— 行高固定 24、缩进 `8 + depth × 14`
//!    （真源 `ProjectSidebarView.swift:490,536`）、目录优先 + 名字小写升序，工作区根是一行
//!    600 粗体的根行（真机默认显示根目录）；文件行右侧按 `git.status` 画状态字母；
//! 2. **状态**：加载中顶部胶囊（真源 `ProjectSidebarView.swift:237-241`）、空态 / 失败态用
//!    `Empty`。
//!
//! 数据来自 Rust Core 的 `workspace.snapshot`（`rust/lithe-core/src/project/files.rs:36-45,160-167`）
//! 与 `git.status`（`rust/lithe-core/src/git/mod.rs:6565`），**在同一个后台线程**取，回前台再建树
//! （见 `Explorer::load`）。建树用的是 `data.root`（含空目录），不是扁平的 `data.files`。
//!
//! 已知取舍（各自的理由写在使用处）：
//! - 树内搜索的**状态与查询通路保留、暂时没有界面**：`Explorer` 的 `search` / `search_open` /
//!   `query` 字段与 `toggle_search` / `clear_search` / `set_query` 都留着，其中暂时不可达的那几项
//!   标了 `#[allow(dead_code)]` 并写明为什么留着，等左活动栏「搜索」项与树内 `Mod+F` / `/` 键位接上；
//! - 根行没有可见子项时（空工作区，或过滤后没有命中）挂一条禁用占位行「文件夹为空」，因为
//!   `TreeItem::is_folder()` 是"有没有子节点"推导出来的（`gpui-base-0.7.0/src/tree.rs:132-134`），
//!   没有子节点就会被当成文件行。**U24 起这条分支也真的覆盖磁盘上的空目录**：树改由
//!   `workspace.snapshot` 的 `data.root`（带 `isDirectory` 与 `children`）聚出，空目录就是
//!   `children: Some([])` 的目录节点（`rust/lithe-core/src/project/files.rs:633-638`），于是
//!   `dir:<路径>` 第一次有了"零个可见子项"的真实输入；旧的扁平 `data.files` 只收非目录项
//!   （同文件 `:582-583`），聚不出任何空目录；
//! - 未能实现的项逐条登记在文件末尾。
//!
//! ## 拆分来源与文件分工
//!
//! 本 crate 从 `lithe-gpui/shell/src/bin/shell_probe/explorer.rs`（1192 行，单个 bin crate 里的
//! 扁平模块）拆出，按《编码指南》「大型应用按业务能力组织 crate」独立成 Feature crate：
//!
//! - `model.rs`：数据层 —— `workspace.snapshot` 拉取（`load_snapshot`，解析 `data.root` 树）
//!   + `build_tree_items` / `to_items` 铺行 + `git.status` 装饰（`load_git_status` /
//!   `GitTreeStatus`）+ `RowKind` / `icon_for_file`；只用 `SharedString` / `TreeItem` /
//!   `IconName` 这些数据结构，**不依赖任何 gpui UI 组件**；
//! - `explorer_view.rs`：视图层 —— `Explorer` 实体（字段、`new` / `refresh` / `reveal` / `load` /
//!   `rebuild` / `run_row_action` / 查询与展开状态）、`LoadState`、右键菜单条目表
//!   （`row_menu_entries`）、`impl Render for Explorer` 与各渲染辅助。
//!
//! ## `load_snapshot` 已收敛到 `shared::core_client`
//!
//! 原先在旧文件里本地拼的 `{id, operationId, timeoutMilliseconds, command, payload}` 信封、
//! `ok` 判定与 `/error/code` 提取，全部改为调用 [`lithe_gpui_shared::core_json`]
//! （`lithe-gpui/crates/shared/src/core_client.rs`）：共享层自带自增 `operationId` 与 120 s 默认超时
//! （`CoreRequest::DEFAULT_TIMEOUT_MILLIS`），所以旧文件里固定的 `shell-explorer-snapshot`
//! 操作 id 一并删除（已 grep 确认没有别处引用）。失败仍是 `Result<_, String>`，
//! 字符串仍是 Core 的错误码原文。
//!
//! ## 公开边界
//!
//! 公开 API 只有 [`Explorer`]（`new` / `refresh` / `reveal` / `set_reveal_in_file_manager_host`
//! + `Render`）与宿主钩子的形状 [`RevealInFileManagerHost`]，字段全部私有，因此不需要
//! `#[non_exhaustive]`。`LoadState`、建树逻辑、文案与度量常量都留在 crate 内部；
//! `model` / `explorer_view` 两个模块本身也保持私有，不成为 import 路径
//! （《编码指南》「内部重组时保持 public module path」的反面用法：新建 crate 直接发布 `pub use`）。
//!
//! ## 规格说明（逐字来自旧 `explorer.rs`）
//!
//! ---
//!
//! 图标真源：全量 Lucide 目录（`gpui_kit::assets::IconName`，1830 个变体）
//!
//! | 用途 | Windows 真机的字形 | 本项目用的字形 | 说明 |
//! | --- | --- | --- | --- |
//! | 目录（折叠） | 主题图标集 folder | `FolderClosed`（`folder-closed.svg`） | 同语义 |
//! | 目录（展开） | 主题图标集 folder-open | `FolderOpen`（`folder-open.svg`） | 同语义 |
//! | 展开/折叠箭头 | `chevron-down` / `chevron-right` | `ChevronDown` / `ChevronRight` | 同语义（12px） |
//! | 源码文件 | Seti 等按语言区分的图标 | `FileCode`（`file-code.svg`） | 真实字形 |
//! | JSON / JSONC | Seti 的 json 图标 | `FileBraces`（`file-braces.svg`） | **目录里没有 `file-json.svg`**，取语义最近的 `file-braces` |
//! | Markdown / 文本 | Seti 的 markdown 图标 | `FileText`（`file-text.svg`） | 真实字形 |
//! | 配置（toml/yaml/ini/conf） | Seti 的配置文件图标 | `Settings`（`settings.svg`） | 真实字形 |
//! | 依赖锁 / 包文件 | Seti 的 lock 图标 | `Package`（`package.svg`） | 真实字形 |
//! | 脚本（sh/ps1/bat） | 终端图标 | `Terminal`（`terminal.svg`） | 真实字形 |
//! | 图片/图标文件 | 主题图标集 image | `Image`（`image.svg`） | 真实字形 |
//! | 其它文件 | 主题图标集 file | `File`（`file.svg`） | 同语义 |
//! | 加载转圈 | 自绘 CSS 圆环 | `Spinner` 默认的 `Loader`（`loader.svg`） | 组件自带 |
//!
//! 头部行那三个按钮（搜索 → `Search`（`search.svg`）、清空搜索 → `X`（`x.svg`）、偏好 →
//! `Settings`（`settings.svg`，真机上「偏好」用的是自有字形的 `Preferences` 齿轮））随头部行在
//! **2026-09-27 按维护者要求**删除，本 crate 已不再渲染它们，所以对照表里不再有这三行。
//!
//! 未实现清单（本轮**不做**的，逐条写明卡在哪）：
//!
//! 1. ~~**Git 状态装饰**~~ ✅ **已完成（U24，2026-10-04）**：`model.rs` 的 `load_git_status`
//!    在同一个后台任务里再调一路 `git.status`，把 `{路径 → 状态字母 + 色调}` 与"目录按最紧急
//!    后代取档"投影到行上（`GitTreeStatus`）。字母与色调逐条照 macOS 的
//!    `GitChange.displayStatus` / `GitChangeKind` / `gitStatusColor`
//!    （`macos/Sources/LitheGitModule/Models/GitModels.swift:853-874` 与
//!    `ProjectSidebarView.swift:747-758`），颜色一律从 `cx.theme()` 取（`GitTone::color`）。
//!    仓库根不等于工作区根时按 `repositoryRoot` 拼前缀（真源靠绝对路径键回避同一问题）；
//!    工作区之外 / 不是仓库 ⇒ 不装饰，只留 `S1_EXPLORER_GIT` 诊断。
//! 2. **右键菜单（部分完成）**：已画 5 条**全部有真实落点**的动作 —— 打开
//!    （`on_open`）、在资源管理器中显示（宿主钩子
//!    [`Explorer::set_reveal_in_file_manager_host`](crate::RevealInFileManagerHost)）、
//!    复制路径、复制相对路径、刷新（真的重取快照）。**不画**的是没有落点的那批：
//!    新建/重命名/删除/复制副本/本地历史/Show Git Diff/目录标记 —— 前五类需要 Core 的文件写命令
//!    （**C34**：重命名/删除/移动在 macOS 是宿主端口，gpui 侧不许自己实现文件系统语义），
//!    Show Git Diff 属 **U15**，本地历史属 **C35**，目录标记在 gpui 侧没有状态与消费者。
//!    条目表是纯数据（`row_menu_entries`），落点是穷尽 `match`（`run_row_action`），
//!    所以"画了不接"在编译期就不可能。
//! 3. **拖拽移动 / 拖到编辑区**：需要拖拽 payload + 落盘命令，同第 2 条（Core 没有文件写命令）。
//! 4. **内联重命名 / 内联新建输入框**：依赖第 2 条的命令通路。
//! 5. **缩进参考线**：真机是每行一套绝对定位的 1px 竖线；gpui 的 `Tree` 行没有任何"层级线"支持，
//!    要在行里画 `w(px(1.))` 的竖线；而且真机规定它默认不可见，缺它不影响常驻观感，本轮不做。
//! 6. **紧凑文件夹链**（把单子目录链合并成 `a.b.c`）：纯数据层算法不难，但会改变行的
//!    path↔id 映射（一行覆盖多级目录），要和展开状态一起改，本轮不做。
//! 7. ~~**根行自动定位**~~ ✅ **已完成（U24，2026-10-04）**：[`Explorer::reveal`] 把绝对路径
//!    折算成工作区相对路径、把祖先目录记进展开集合、重建树之后 `reveal_item` + `set_selected_index`
//!    （复用既有的树选中/滚动口径，`gpui-base-0.7.0/src/tree.rs:237-248,275-285`）。
//!    入口是编辑区标签右键菜单的「在项目树中显示」（真源
//!    `macos/Sources/Lithe/Views/Editor/EditorAreaView.swift:1206-1213`，由外壳登记
//!    `TabMenuHostActions::reveal_in_tree`）。真源还有**另外两处**入口没有接：
//!    侧栏头部那颗 `scope` 按钮（`ProjectSidebarView.swift:218-228`，gpui 侧侧栏头部行已在
//!    2026-09-27 按维护者要求删除）与面包屑点击（`WorkbenchView.swift:1679`）。
//!    **另有一处差异如实登记**：路径不在工作区里（或不在快照里）时，真源弹一条可见通知
//!    （`AppModel+FeatureState.swift:63` 的 `showNotification("This file is not in the current
//!    workspace")`），本侧只打 `S1_EXPLORER reveal ... found=false` / `reveal=outside-root`
//!    诊断 —— 要弹通知得再新增一条 gpui 自有文案键（真源那句硬编码在 Swift 里、catalog 没有），
//!    本轮没做。
//! 8. **两档选中底色 / 胶囊阴影**：真机的选中底色分"树未聚焦 / 聚焦"两档，gpui 的 `Tree`
//!    只给一档 `list_active`；加载胶囊的 `shadow-popover` 与 `backdrop-blur-sm` 也没有对应 token。
//! 9. **树容器级键盘**（`Mod+F` / `/` 开搜索、`Mod+C/X/V`、`F2`、`Home/End`）：`Tree` 内建
//!    `↑↓←→` 与 `Enter`，其余要在外层 `track_focus` + 自己绑 action，本轮只保证内建那几条可用。
//! 10. **空态的「打开文件夹」按钮**：渲染成禁用态（原因见 `Explorer::empty_no_rows`）。
//! 11. **多选（Shift / Ctrl 语义）**：`TreeState` 只有**单**选中（`selected_ix: Option<usize>`，
//!    `gpui-base-0.7.0/src/tree.rs:195`），要做多选得先在探索器里另建一份选择集合、
//!    并让每一行自己画选中态（不能再依赖 `ListItem::selected`），本轮不做。
//! 12. **watcher 联动**：`H12` —— gpui 侧没有任何文件系统 watcher（macOS 用 FSEvents）。
//!    这**不是**本 crate 能补的缺口：Core 已经把 `searchIndex.update` 备好给 watcher 驱动，
//!    而 watcher 属平台端口。本轮**不**用定时轮询顶替（那会变成"每 N 秒扫全盘"），
//!    所以树与「更改」列表在外部改动后只能靠手动刷新（右键菜单那条）与重新打开项目更新。
//!    项目**打开**时会取一次快照（`ShellWorkspace::new` 的公共收口）。

mod explorer_view;
mod model;

pub use explorer_view::{Explorer, RevealInFileManagerHost};
