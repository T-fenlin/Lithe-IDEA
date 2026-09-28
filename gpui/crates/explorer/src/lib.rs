//! 侧栏 · 项目树（资源管理器）。
//!
//! 这一块对应 Windows 前端的 `FileExplorerPane` → `FileExplorerTree`：**只有树体**（虚拟滚动、
//! 任意深度、目录展开/折叠、点击文件打开）。规格一律取
//! `windows/tauri/src/features/file-explorer/` 的源码，每个数值都带 `文件:行号`。
//!
//! ⚠️ **2026-09-27 按维护者要求删除**：真机 `FileExplorerPane` 的第一层 `SidebarHeader`
//! 「项目」头部行（高 32、标题 + 搜索按钮 + 清空按钮 + 偏好按钮）与它下面的树内搜索行（高 28，
//! 原实现是一整行内联输入框）都已删除，所以本模块渲染出来的顶部就是文件树本身；搜索入口改由
//! 左活动栏的「搜索」项承担（`gpui/crates/workbench/src/activity_bar.rs`）。这两条与 Windows
//! 真机的差异原先登记在下面的未实现清单第 10 条（偏好下拉菜单）与第 11 条（搜索浮层），随实现
//! 一并删除（原第 12 条顺延为第 10 条）。图标、颜色、主题一律没动。
//!
//! 两层结构：
//!
//! 1. **树体**：gpui 的 `Tree`（表体是 `uniform_list`）—— 行高固定 24、缩进 `10 + depth × 16`、
//!    目录优先 + 名字小写升序，工作区根是一行 600 粗体的根行（真机默认显示根目录）；
//! 2. **状态**：加载中顶部胶囊（`file-explorer-pane.tsx:54-60`）、空态 / 失败态用
//!    `Empty`（`ui/empty.tsx:111-130` 的 `EmptyState`）。
//!
//! 数据来自 Rust Core 的 `workspace.snapshot`（`rust/lithe-core/src/project/files.rs:36-45,160-167`），
//! **在后台线程**取，回前台再建树（见 [`Explorer::load`]）。
//!
//! 已知取舍（各自的理由写在使用处）：
//! - 树内搜索的**状态与查询通路保留、暂时没有界面**：`Explorer` 的 `search` / `search_open` /
//!   `query` 字段与 `toggle_search` / `clear_search` / `set_query` 都留着，其中暂时不可达的那几项
//!   标了 `#[allow(dead_code)]` 并写明为什么留着，等左活动栏「搜索」项与树内 `Mod+F` / `/` 键位接上；
//! - 空目录挂一条禁用占位行「文件夹为空」，因为 `TreeItem::is_folder()` 是"有没有子节点"
//!   推导出来的（`gpui-base-0.6.6/src/tree.rs:131-134`），没有子节点就会被当成文件行；
//! - 未能实现的项逐条登记在文件末尾。
//!
//! ## 拆分来源与文件分工
//!
//! 本 crate 从 `gpui/shell/src/bin/shell_probe/explorer.rs`（1192 行，单个 bin crate 里的
//! 扁平模块）拆出，按《编码指南》「大型应用按业务能力组织 crate」独立成 Feature crate：
//!
//! - `model.rs`：数据层 —— `workspace.snapshot` 拉取（`load_snapshot`）+ 扁平路径聚成多层树
//!   （`PathTree` / `build_tree_items` / `to_items`）+ `RowKind` / `icon_for_file`；
//!   只用 `SharedString` / `TreeItem` / `IconName` 这些数据结构，**不依赖任何 gpui UI 组件**；
//! - `explorer_view.rs`：视图层 —— `Explorer` 实体（字段、`new` / `refresh` / `load` /
//!   `rebuild` / 查询与展开状态）、`LoadState`、`impl Render for Explorer` 与各渲染辅助。
//!
//! ## `load_snapshot` 已收敛到 `shared::core_client`
//!
//! 原先在旧文件里本地拼的 `{id, operationId, timeoutMilliseconds, command, payload}` 信封、
//! `ok` 判定与 `/error/code` 提取，全部改为调用 [`lithe_gpui_shared::core_json`]
//! （`gpui/crates/shared/src/core_client.rs`）：共享层自带自增 `operationId` 与 120 s 默认超时
//! （`CoreRequest::DEFAULT_TIMEOUT_MILLIS`），所以旧文件里固定的 `shell-explorer-snapshot`
//! 操作 id 一并删除（已 grep 确认没有别处引用）。失败仍是 `Result<Vec<String>, String>`，
//! 字符串仍是 Core 的错误码原文。
//!
//! ## 公开边界
//!
//! 只有 [`Explorer`] 是公开 API（`new` / `refresh` + `Render`），字段全部私有，因此不需要
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
//! 1. **Git 状态装饰**（文件行状态字母 + 目录/文件名染色）：数据来自 Core 的 `git.status`，
//!    本模块只调 `workspace.snapshot`。这不是 API 缺口（上一轮的 `panels.rs::load_git_marks`
//!    已经跑通过，契约见 `shared/contracts/rust-core-api.md`），缺的是这里还没接第二路后台调用。
//! 2. **右键菜单**（打开 / 新建文件 / 新建文件夹 / 重命名 / 删除 / 复制路径 / 在资源管理器中显示 /
//!    刷新 / 全部折叠）：`Tree::context_menu` 现成可用（`gpui-component-0.6.6/src/tree.rs:54-62`），
//!    卡住的是**动作落点** —— 新建/重命名/删除在 Windows 走 `#[tauri::command]`
//!    （`move_file` / `rename_file` 等，见 `gpui/research/windows/02-editor-sidebar.md` §4.2），
//!    Core 契约里没有文件写命令；「在资源管理器中显示」要起 `explorer.exe`，属于平台层，
//!    不该由这个 UI 模块做。所以「刷新」以公开方法 [`Explorer::refresh`] 的形式提供。
//! 3. **拖拽移动 / 拖到编辑区**：需要拖拽 payload + 落盘命令，同第 2 条。
//! 4. **内联重命名 / 内联新建输入框**：依赖第 2 条的命令通路。
//! 5. **缩进参考线**：真机是每行一套绝对定位的 1px 竖线（`file-explorer-tree.css:170-200`），
//!    gpui 的 `Tree` 行没有任何"层级线"支持，要自己在行里画 `w(px(1.))` 的竖线；
//!    而且真机规定它默认 `opacity: 0`、只有悬停/聚焦时才到 0.9（同文件 `:177-190`），
//!    缺它不影响常驻观感，本轮不做。
//! 6. **紧凑文件夹链**（`compactFoldersInFileTree: true` 默认开启，
//!    `visible-file-tree-rows.ts:168-184` 把单子目录链合并成 `a.b.c`）：纯数据层算法不难，
//!    但会改变行的 path↔id 映射（一行覆盖多级目录），要和展开状态一起改，本轮不做。
//! 7. **根行自动定位**（`autoRevealActiveFileInFileTree: true`）：`TreeState::reveal_item`
//!    现成可用（`gpui-base-0.6.6/src/tree.rs:268-278`），缺的是"当前活动文件"这个输入 ——
//!    外壳还没把编辑区状态回传给侧栏（本模块只记自己点开过的那一个）。
//! 8. **两档选中底色 / 胶囊阴影**：真机的选中底色分"树未聚焦 `--border` / 聚焦
//!    `--selected`"（`file-explorer-tree.css:69-78`），gpui 的 `Tree` 只给一档 `list_active`；
//!    加载胶囊的 `shadow-popover` 与 `backdrop-blur-sm` 也没有对应 token。
//!    （原先这里还登记了"头部下边框 72% 透明度没有对应 token"一条；头部行已在 **2026-09-27 按
//!    维护者要求**删除，那一条随之不再存在。）
//! 9. **树容器级键盘**（`Mod+F` / `/` 开搜索、`Mod+C/X/V`、`F2`、`Home/End`）：`Tree` 内建
//!    `↑↓←→` 与 `Enter`（`gpui-base-0.6.6/src/tree.rs:19-27,350-411`），其余要在外层
//!    `track_focus` + 自己绑 action，本轮只保证内建那几条可用。**2026-09-27 起**「开搜索」那两条
//!    键位暂时没有落点可绑：树内搜索行已删除，等左活动栏的「搜索」项接上后再绑。
//! 10. **空态的「打开文件夹」按钮**：渲染成禁用态（原因见 [`Explorer::empty_no_rows`]）。
//!
//! 原第 10 条「偏好下拉菜单」与原第 11 条「搜索浮层」（分别对应已被删除的
//! `Explorer::render_header` 与 `Explorer::render_search_row`）随那两行在 **2026-09-27 按维护者
//! 要求**删除；原第 12 条顺延为上面的第 10 条。

mod explorer_view;
mod model;

pub use explorer_view::Explorer;
