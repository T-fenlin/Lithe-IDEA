# 打开项目 / 多窗口 —— 只读侦察报告

**任务**：补「打开项目/文件」流程（系统文件夹对话框 →「你想在哪里打开项目"X"？」带 `□ 不再询问` + 取消/新窗口/此窗口）、
「最近项目」列表、「打开文件」（新开标签页）、「克隆仓库」。

**本文件只登记事实**：每条都给 `文件:行号`。**推断**单独标注 `【推断】`。

**方法（硬边界）**：只读 + `rg` / 读文件 / `git log`。**没有跑 `cargo`、没有启动 Lithe、没有改任何产品代码**。
所以本文件里的"实测"一律指**读源码得出的判定**，不是实机点击结果。

**分支**：`feat/gpui-shell-rewrite`，HEAD = `1c8ce0d0`。
**上游源码位置**（不在仓库里，在 cargo registry 缓存中）：
`D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\`（下称 `$REG`），
本报告里的 `gpui-pre-0.3.6/...` 等路径都相对它。

**与前一份侦察报告的关系**：`gpui/research/menu-tabs-and-open-file.md`（HEAD `1c8ce0d0` 提交的那一份）已覆盖
「打开文件」的缺口定位与菜单/标签页/快捷键全量。本文件**只补它没有确认的四件事**：
① 多窗口能不能做；② `prompt_for_paths` 的精确签名 + macOS 侧是否存在；③ 换根要重建什么；④ 最近项目落盘 + 「不再询问」偏好。
**不重复**它已钉死的事实，需要时直接引用（写 `见前报告 §4.2`）。

---

## 1. 结论速览

| 问题 | 结论（一句话） | 最强证据 |
| --- | --- | --- |
| **`新窗口` 今天能不能做成真的** | **gpui 层能，本仓库今天不能** —— gpui 的 `App::open_window` 是 `&mut self` 上可反复调用的普通方法（不是"启动一次"），但本仓库现在**只开了一个窗口**，且 5 个 `thread_local` 全局单例把「菜单栏 / 项目下拉 / 命令面板 / Git 身份宿主」钉成"一个进程一份"，第二个 `ShellWorkspace` 会**覆盖**第一个的句柄 → 第一个窗口的菜单会去改第二个窗口 | `App::open_window`：`gpui-pre-0.3.6/src/app.rs:1347-1380`；唯一调用点：`gpui/crates/app/src/main.rs:623`；单例：`command_palette.rs:115,141`、`menu_bar.rs:766,774`、`project_menu.rs:813`、`settings/src/identity.rs:311-312` |
| **换根的最小路径** | **重建整个 `ShellWorkspace`（新 `Entity`），不要逐个 reset** —— 6 个实体 + 4 个 `thread_local` + 1 个 JDTLS 会话都在**构造期**绑定根，且**没有任何 `set_root` / `reset`**；好消息是 gpui 有 `Window::replace_root`，「此窗口」不需要重开窗口 | `Window::replace_root`：`gpui-pre-0.3.6/src/window.rs:2242-2254`；根绑定构造点：`workspace.rs:688,697-703,711-715,729-732,773,783-785,959,984`；`prepare_java` 幂等但不是"可重指"：`editor_view.rs:435-438` |
| **最近项目落盘在哪** | **落 gpui 自己的全局设置文件**（`%APPDATA%\Lithe\settings.json`），**不要落 `.lithe/`** —— gpui 侧对 `.lithe/` **一个字节都没写过**（只读 `.lithe/toolchains/jdtls` 当 JDK 探测候选）；`.lithe/` 的写入者全在 Windows Tauri 侧与 Rust Core | 设置文件路径：`gpui/crates/settings/src/paths.rs:45-69`；gpui 读 `.lithe` 的唯一一处：`gpui/crates/java/src/jdtls.rs:402-407`；`.lithe` 写入者：`windows/tauri/src-tauri/src/run.rs:252`、`maven.rs:101` |
| **macOS 有没有对话框** | **有，而且是原生 `NSOpenPanel`** —— 三个桌面平台**都有**真实现（Windows `IFileOpenDialog` / macOS `NSOpenPanel` / Linux `ashpd`），所以"只有 Windows 能用"是**错的** | macOS：`gpui-pre-macos-0.3.6/src/platform.rs` 的 `fn prompt_for_paths`（`NSOpenPanel::openPanel` + `setCanChooseDirectories/Files` + `setAllowsMultipleSelection`，见 §2.2）；Windows：`$REG/gpui-pre-windows-0.3.6/src/platform.rs:673-684` |

**一句话**：**先做「打开文件」，再做「最近项目」，再做「此窗口」换根（`replace_root` 重建 `ShellWorkspace`），最后才做「新窗口」（要先拆 5 个全局单例）。**

---

## 2. 逐条事实表

### 2.1 gpui 侧支持多窗口吗？

#### 2.1.1 上游能力：**支持，且是"可反复调用"的普通方法**

| 事实 | 位置 |
| --- | --- |
| `pub fn open_window<V: 'static + Render>(&mut self, options: WindowOptions, build_root_view: impl FnOnce(&mut Window, &mut App) -> Entity<V>) -> anyhow::Result<WindowHandle<V>>` —— `App` 上的方法，**每次调用插入一个新的 `WindowId`**（`cx.windows.insert(None)`），返回新 `WindowHandle<V>` | `$REG/gpui-pre-0.3.6/src/app.rs:1347-1380`（插入在 `:1353`，句柄登记在 `:1370`） |
| 窗口表是 `SlotMap<WindowId, Option<Box<Window>>>` + `FxHashMap<WindowId, AnyWindowHandle>` —— **本来就是多份**，不是"单窗口的容器" | `$REG/gpui-pre-0.3.6/src/app.rs:758-759` |
| 公开的多窗口查询 API：`App::windows() -> Vec<AnyWindowHandle>`、`App::window_stack() -> Option<Vec<AnyWindowHandle>>`（前到后）、`App::active_window() -> Option<AnyWindowHandle>` | `$REG/gpui-pre-0.3.6/src/app.rs:1323-1328 / 1335-1337 / 1340-1342` |
| 从 `Context<T>` 里能直接调 `open_window`：`Context` 对 `App` 有 `Deref` **和 `DerefMut`** | `$REG/gpui-pre-0.3.6/src/app/context.rs:26-38` |
| 从异步任务里也能开窗口：`AsyncWindowContext::open_window(&self, options, build_root_view) -> Result<WindowHandle<V>>`（内部 `lock.open_window(..)`；`quitting` 时报错） | `$REG/gpui-pre-0.3.6/src/app/async_context.rs:193-207` |
| 已有"把实体在窗口之间搬"的官方示例 —— 说明上游把多窗口当一等场景 | `$REG/gpui-pre-0.3.6/examples/move_entity_between_windows.rs`（`Cargo.toml` 的 `[[example]]` 里注册） |
| 关窗：`Window::remove_window(&mut self)`（置 `removed = true`）；`Window::window_handle()` | `$REG/gpui-pre-0.3.6/src/window.rs:2280-2282 / 2267-2269` |
| **macOS 也有**多窗口（与 Windows 同构：`Platform::open_window(&self, handle, params)`；`window_stack` 返回 `MacWindow::ordered_windows()`） | macOS 平台 `fn open_window` / `fn window_stack`（`gpui-pre-macos-0.3.6/src/platform.rs`）；Windows 平台同名方法在 `$REG/gpui-pre-windows-0.3.6/src/platform.rs` |
| `WindowKind` 是**窗口种类**（`AnchoredPopup` 等），**不是**"项目窗口 vs 设置窗口"那种业务分类；macOS 明确拒绝 `WindowKind::AnchoredPopup`（未实现原生 popup），回落到窗口内浮层 | macOS `open_window` 里的 `if let WindowKind::AnchoredPopup(_) = options.kind { return Err(PopupNotSupportedError.into()) }` |

#### 2.1.2 本仓库现状：**实际只开了 1 个窗口**

| 事实 | 位置 |
| --- | --- |
| **全仓库 `cx.open_window` 只有 1 个调用点**（`rg 'open_window'` 在 `gpui/**` 的 `.rs` 里只命中 3 处：2 处是文档注释、1 处是真调用） | `gpui/crates/app/src/main.rs:14`（注释）、`:474`（注释）、**`:623`（唯一调用）** |
| 它从 `cx.spawn(async move \|cx\| { … cx.open_window(window_options, \|window, cx\| { … }) })` 里调用，`window_options` 在闭包内构造 | `gpui/crates/app/src/main.rs:616-623` |
| 窗口根**不是** `ShellWorkspace`，而是 gpui-kit 的 `Root`（负责对话框 / 浮层 / 通知）；`ShellWorkspace` 是 `Root` 的**子实体**（`cx.new` 出来再交给 `Root::new`） | `gpui/crates/app/src/main.rs:624-633`（建 workspace）、**`:709`（`Root::new(workspace, window, cx)`）**；理由注释在 `:708` 与模块头 `:14` |
| **没有任何"新窗口打开项目"的通路**：`WindowKind` / `window_handles` / `App::windows()` / `window_stack()` / `active_window()` 在本仓库的命中如下 | `rg` 结论：`window_handles` **零命中**；`WindowKind` **零命中**；`App::windows()` **零命中**；`window_stack()` **零命中**；`active_window()` **2 处**，两处都是"取当前窗口开浮层"，**不是**多窗口管理 —— `gpui/crates/settings/src/dialog.rs:95`、`gpui/crates/workbench/src/command_palette.rs:286` |
| 窗口关掉之后的收尾：本仓库**没有** `remove_window` 调用（`rg` 零命中） | — |

#### 2.1.3 真正的阻塞点：**"一个进程一份"的 5 个 `thread_local` 全局单例**

这 5 个都是 `thread_local!`（gpui 主线程单线程），`ShellWorkspace::new` 每次都会**无条件覆盖**：

| 单例 | 位置 | 存什么 | 第二个 workspace 建起来之后会怎样 |
| --- | --- | --- | --- |
| `SHELL` | `gpui/crates/workbench/src/command_palette.rs:115`（登记 `:119-121`，读 `:124-126`） | `WeakEntity<ShellWorkspace>` | 命令面板的执行分支（终端/右栏/状态栏/主题）会改到**第二个**窗口 |
| `SHELL_FOCUS` | `gpui/crates/workbench/src/command_palette.rs:141`（登记 `:145-147`） | `FocusHandle` | 同上；另外 `main.rs:720` 的 `--palette-keys` 也读它 |
| `MENU_BAR` | `gpui/crates/workbench/src/menu_bar.rs:766-767`（登记 `:784-786`） | `WeakEntity<MenuBar>` | 菜单栏**每个窗口一个**（注释 `:771` 自认），但读的时候只拿得到最后一个 → 第一个窗口的菜单栏变成"不被绘制/不被更新" |
| `MENU_BAR_SHELL` | `gpui/crates/workbench/src/menu_bar.rs:774-775`（登记 `:779-781`） | `WeakEntity<ShellWorkspace>` | 菜单动作会打到第二个外壳 |
| `PROJECT_MENU` | `gpui/crates/workbench/src/project_menu.rs:813`（登记 `:817-819`） | `WeakEntity<ProjectMenu>` | 项目下拉同上 |
| 外加 `GIT_IDENTITY_HOST` | `gpui/crates/settings/src/identity.rs:311-312`（`thread_local` + `Rc<GitIdentityHost>`；登记 `:319`） | `GitIdentityHost`，**里面烘焙了 `workspace_root`**（`workspace.rs:783-785`） | 设置页的「Git 提交身份」会去读**最后一个**打开的项目 |

> 代码自己的注释已经承认这件事：`command_palette.rs:113-114`——「为什么不是 `Global`：那是 `App` 级存储，
> 而"当前窗口的这个视图"在一个进程里**可以有多份**，放进去会互相覆盖」。也就是说：**上游 API 允许多份，
> 本仓库自己把它们缩成了一份**。

**【推断】`新窗口` 的工程量**：不是"加一个按钮"，而是"把这 5 个 `thread_local` 换成窗口级存储"
（gpui 的对应物是 `App::Global` 的 `HashMap<WindowId, …>`，或把句柄挂在 `Root`/`Window` 上）。
`ShellWorkspace` 本身**可以**被建出第二份（`new()` 是纯函数式的，见 §2.3），所以缺的**只是"句柄路由"这一层**。

**直接回答"`新窗口` 这个按钮今天能不能做成真的"**：
**不能**（今天点下去会做出一个"看起来能开、但两个窗口的菜单/下拉/命令面板互相串台"的假多窗口）。
**GPUI 不缺能力，缺的是本仓库的窗口级句柄路由。** 最小可信做法见 §3.5。

---

### 2.2 `prompt_for_paths` 的完整用法

#### 2.2.1 精确签名（**不是回调，是 `oneshot::Receiver`**）

| 项 | 事实 | 位置 |
| --- | --- | --- |
| **`App::prompt_for_paths`** | `pub fn prompt_for_paths(&self, options: PathPromptOptions) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>>` | `$REG/gpui-pre-0.3.6/src/app.rs:1687-1692` |
| **返回是 `Task` 还是回调** | **都不是**：返回 `futures::channel::oneshot::Receiver<Result<Option<Vec<PathBuf>>>>`。取消 → `Ok(None)`；选了 → `Ok(Some(paths))`；Linux 打不开选择器 → `Err` | 同上；文档注释 `:1682-1686` 明写「When one or more paths are selected, they'll be relayed **asynchronously via the returned oneshot channel**. If cancelled, a `None` will be relayed instead.」 |
| **`PathPromptOptions` 四个字段** | `pub files: bool` / `pub directories: bool` / `pub multiple: bool` / `pub prompt: Option<SharedString>`（`#[derive(Clone, Debug)]`，**没有 `Default`** → 必须四个都写） | `$REG/gpui-pre-0.3.6/src/platform.rs:2483-2494`（字段在 `:2487,2489,2491,2493`） |
| **`directory: true` 的正确写法** | 没有 `directory` 这个字段名，是 **`directories: true`**；选文件夹 = `PathPromptOptions { files: false, directories: true, multiple: false, prompt: None }` | 同上；Windows 侧靠 `FOS_PICKFOLDERS` 实现（`$REG/gpui-pre-windows-0.3.6/src/platform.rs:1374-1400` 的 `file_open_dialog`，见前报告 §4.2） |
| **文件与文件夹不能混选** | `Platform::can_select_mixed_files_and_dirs()`：Windows 返回 **`false`**（注释：「`FOS_PICKFOLDERS` 在"只文件"与"只文件夹"之间切换」） | `$REG/gpui-pre-windows-0.3.6/src/platform.rs:702-705`；macOS 同名方法在 `gpui-pre-macos-0.3.6/src/platform.rs` 的 `fn can_select_mixed_files_and_dirs` |
| 姊妹 API：**另存为**用 `prompt_for_new_path` | `pub fn prompt_for_new_path(&self, directory: &Path, suggested_name: Option<&str>) -> oneshot::Receiver<Result<Option<PathBuf>>>` | `$REG/gpui-pre-0.3.6/src/app.rs:1700-1706` |
| 类型从哪来 | `platform.rs` 的类型经 `pub use platform::*;` 出现在 `gpui` crate 根 → 本仓库写 `gpui_kit::PathPromptOptions`（`gpui-kit-0.6.6/src/lib.rs` 是 `pub use ::gpui::*;`） | 见前报告 §4.2 |

#### 2.2.2 仓库里**有没有已有调用例**：**没有**

`rg 'prompt_for_paths|prompt_for_new_path|PathPromptOptions'` 扫全仓库（不限 `*.rs`）共 **13 处命中，全部在
`gpui/research/menu-tabs-and-open-file.md` 这一份文档里**（`:113,114,374,375,377,379,382,385,392,413,416,450,465`）。
**产品代码 `.rs` 里零命中** —— 也就是说：**这条 API 本仓库一次都没调过**，没有可照抄的调用例（这正是前报告 §4.4 说的"只缺这一步"）。

#### 2.2.3 ⚠️ macOS 侧实现**存在**（"只有 Windows 能用"是错的）

| 平台 | 实现 | 证据 |
| --- | --- | --- |
| **Windows** | **真实现**：Win32 `IFileOpenDialog` + `FOS_PICKFOLDERS` / `FOS_ALLOWMULTISELECT` / `FOS_FILEMUSTEXIST`，取消返回 `Ok(None)` | `$REG/gpui-pre-windows-0.3.6/src/platform.rs:673-684`（`prompt_for_paths`）→ `:1374-1400`（`file_open_dialog`） |
| **macOS** | **真实现**：**`NSOpenPanel`** —— `let panel = NSOpenPanel::openPanel(marker); panel.setCanChooseDirectories(options.directories); panel.setCanChooseFiles(options.files); panel.setAllowsMultipleSelection(options.multiple); panel.setCanCreateDirectories(true); panel.setResolvesAliases(false);` 然后 `panel.beginWithCompletionHandler(&handler)`，在 handler 里按 `NSModalResponseOK` 决定 `Ok(Some(paths))` / `Ok(None)`；`options.prompt` 走 `panel.setPrompt(...)` | `gpui-pre-macos-0.3.6/src/platform.rs` 的 `fn prompt_for_paths`（该文件 `use objc2_app_kit::{NSModalResponse, NSModalResponseOK, NSOpenPanel, NSSavePanel, …}`） |
| **Linux** | `ashpd`（XDG portal）；`prompt_for_paths` 文档注释明写「May return an error on Linux if the file picker couldn't be opened」 | `$REG/gpui-pre-0.3.6/src/app.rs:1686`；`gpui/Cargo.lock` 有 `ashpd`（前报告 §4.2 已记） |

**为什么 macOS 的实现"不在仓库里也不在 registry 的已下载目录里"**：平台实现是**独立 crate**，按 target 条件依赖：

- `gpui-pre-platform-0.3.6/Cargo.toml` 里 `[target.'cfg(target_os = "macos")'.dependencies.gpui_macos]` → `package = "gpui-pre-macos"`（`=0.3.6`）；Windows 分支同时依赖 `gpui_pre`(=`gpui-pre`) 与 `gpui_windows`(=`gpui-pre-windows`)。
- `gpui/Cargo.lock` 有完整条目：`gpui-pre-macos`(`:2672-2718`)、`gpui-pre-apple`(`:2558`)、`gpui-pre-linux`(`:2625`)、`gpui-pre-windows`(`:2935`)。
- 本机是 Windows，所以 `gpui-pre-macos-0.3.6` 的源码**没被下载**（`$REG` 里只有 `gpui-pre-windows-*`）。
  本报告用 docs.rs 的源码页 + cdn.jsdelivr 上的 zed 固定 rev 源码核对：
  crate 元数据里 `zed-rev = bcf6582ce3500df93a8a39366640173e6786cea6`（`$REG/gpui-pre-0.3.6/Cargo.toml:50`）。
  来源：[docs.rs gpui-pre-macos 0.3.6 source](https://docs.rs/crate/gpui-pre-macos/0.3.6/source/src/platform.rs)、
  [cdn.jsdelivr zed@bcf6582 gpui_macos/src/platform.rs](https://cdn.jsdelivr.net/gh/zed-industries/zed@bcf6582ce3500df93a8a39366640173e6786cea6/crates/gpui_macos/src/platform.rs)。

**结论**：`prompt_for_paths` 是**跨三平台**的现有能力；本仓库"gpui 侧没有任何文件对话框依赖"这句
（`project_menu.rs:111` 与模块头 `:108-113`）**字面对但不完整**，它漏掉了 gpui 自带的这条 API。

---

### 2.3 工作区换根（"此窗口"打开另一个项目）要重建哪些东西

#### 2.3.1 `ShellWorkspace` 的字段全表（`gpui/crates/workbench/src/workspace.rs:482-640`），标出与"根"的关系

| # | 字段（结构体字段名） | 行号 | 与"根"的关系 | 构造点 |
| --- | --- | --- | --- | --- |
| 1 | `projects: Vec<ProjectTab>` | `:484` | **间接**：内容由根算出的 `project_name` 决定 | `:988`（**恒 1 项**） |
| 2 | **`root: PathBuf`** | **`:490`** | **根本身** | `:651`（`new` 的参数）、`:989` |
| 3 | `active_project: Option<usize>` | `:492` | 无 | `:992` |
| 4 | `activity_items` | `:497` | 无 | `:993` |
| 5 | `right_activity_items` | `:501` | 无 | `:994` |
| 6 | `top_activity_view` | `:507` | 无 | `:996` |
| 7 | `right_view` | `:516` | 无 | `:999` |
| 8 | `right_visible` | `:523` | 无 | `:1000` |
| 9 | **`maven_project: Option<MavenProjectView>`** | **`:533`** | **根的缓存**（`maven.scan` 的结论，懒扫一次） | `:1001`（`None`）；扫在 `:1521-1535`，**用 `self.root`** |
| 10 | **`spring_index: Option<SpringIndexView>`** | **`:539`** | **根的缓存**（`spring.index` 的结论） | `:1002`（`None`）；扫在 `:1521-1540`，**用 `self.root`** |
| 11 | **`right_scan: RightScanState`** | **`:546`** | **根的"扫过没有"闸门** | `:1003`（`Default`）；类型 `:1588-1639` |
| 12 | **`explorer: Entity<Explorer>`** | **`:553`** | **构造期绑根** | `:697-703`（`Explorer::new(explorer_root, on_open, …)`） |
| 13 | **`changes: Entity<ChangesView>`** | **`:561`** | **构造期绑根** | `:711-715`（`ChangesView::new(changes_root, …)`） |
| 14 | **`editor: Entity<EditorPane>`** | **`:563`** | **两条绑根**：JDTLS 会话 + `workspace_root` | `:684`（建实体）、**`:688`（`pane.prepare_java(root.clone(), cx)`）**、**`:729-732`（`pane.set_workspace_root(root)`）** |
| 15 | `terminal: Entity<TerminalPane>` | `:566` | **无**（只有每个页签自己的 cwd） | `:717` |
| 16 | **`bottom_git: Entity<BottomPane>`** | **`:568`** | **构造期绑根** | `:773`（`BottomPane::new(root.clone(), …)`） |
| 17 | `bottom_kind: BottomPaneKind` | `:570` | 无 | `:1011` |
| 18 | `bottom_visible: bool` | `:575` | 无 | `:1012` |
| 19 | `_settings_subscription` | `:580` | 无（订阅 `SettingsStore`） | `:884-930`、`:1013` |
| 20 | `_terminal_subscription` | `:586` | 无 | `:936-945`、`:1014` |
| 21 | `_editor_subscription` | `:592` | 无 | `:948`、`:1015` |
| 22 | `focus: FocusHandle` | `:601` | 无（但 `window.focus(&focus)` 在 `:1037`） | `:964` |
| 23 | **`menu_bar: Entity<MenuBar>`** | **`:611`** | 无（但句柄进 `MENU_BAR` 全局） | `:969`（`MenuBar::new(mode_for(compact_menu), focus, cx)`） |
| 24 | **`project_menu: Entity<ProjectMenu>`** | **`:617`** | **构造期绑"项目名"** | `:977`（`ProjectMenu::new(project_name.as_ref(), …)`） |
| 25 | `_project_menu_subscription` | `:623` | 无 | `:979`、`:1020` |
| 26 | **`branch_panel: Entity<BranchPanel>`** | **`:629`** | **构造期绑根**（自己读 `git.status` + `git.references`） | `:984`（`BranchPanel::new(root.clone(), window, cx)`） |
| 27 | `_branch_panel_subscription` | `:633` | 无 | `:985`、`:1022` |
| 28 | `branch_panel_probe: bool` | `:639` | 无 | `:1023` |

#### 2.3.2 根在构造期被"烘焙"进哪些地方（换根真正要碰的就是这张表）

| 落点 | 行号 | 烘焙方式 | 有 `set_root` / `reset` 吗 |
| --- | --- | --- | --- |
| Java 语言服务（JDTLS 会话） | `workspace.rs:688` → `editor_view.rs:435-440` | `JavaLanguageService::new(workspace_root)` 存进 `EditorPane.java`（字段 `editor_view.rs:296`） | **没有，而且最硬**：`prepare_java` 开头 `if self.java.is_some() { return; }`（`editor_view.rs:436-438`），注释 `:434` 明写「**幂等**：已经登记过工作区就不再重复起服务」→ **同一个 `EditorPane` 永远换不了根** |
| 编辑器相对路径根 | `workspace.rs:729-732` → `EditorPane::set_workspace_root`（`editor_view.rs:625-627`，字段 `:303`） | `set_workspace_root(root)` | **有 setter**（唯一一个可重指的） |
| 项目树 | `workspace.rs:697-703` | `Explorer::new(root, …)`（字段 `Explorer.root`：`explorer_view.rs:163`） | **没有**。`Explorer::refresh`（`explorer_view.rs:263-265`）只是 `load(cx)`，用的是**已存的 root**；`Explorer::rebuild`（`:323`）是"重建树控件"，不是换根 |
| 源代码管理视图 | `workspace.rs:711-715` | `ChangesView::new(root, …)`（字段 `ChangesView.root`：`changes_view.rs:242`） | **没有**。`ChangesView::refresh`（`changes_view.rs:325`）同上口径 |
| 底部窗 Git 提交记录 | `workspace.rs:773` | `BottomPane::new(root, …)`（字段 `BottomPane.root`：`log_view.rs:57`） | **没有**（只有 `log_view.rs:168` 的 `BottomPane` 系 `refresh`） |
| 标题栏分支弹窗 | `workspace.rs:984` | `BranchPanel::new(root, …)` | **没有** |
| **Git 提交身份宿主（全局）** | `workspace.rs:782-873`（`set_git_identity_host`，`workspace_root: root` 在 `:785`；load/save 各克隆一份根文本 `:792,833`） | `thread_local` 单例，**烘焙到闭包里** | **没有**；只能整体重登记（`set_git_identity_host(Some(..))`，`identity.rs:319`） |
| 状态栏 Git 分支 | `workspace.rs:959`（启动诊断）、`:1371`（每帧 `read_branch(&self.root)`） | 每次现读 `self.root` | **自动跟着走**（它读字段） |
| 右栏 Maven / Spring 扫描 | `workspace.rs:1521-1540`（`let root = self.root.clone();` → `maven::scan(&root)` / `spring::index(&root)`） | 扫一次后写进 `maven_project` / `spring_index`，并由 `right_scan` 保证"每个视图只扫一次"（`RightScanState::request`，`:1605-1639`） | **没有 setter**，但 `maven_project = None` + `spring_index = None` + `right_scan = RightScanState::default()`（`:1001-1003`）就是"重扫"所需的全部状态 |
| 项目下拉里的项目名 | `workspace.rs:950-954`（算 `project_name`）→ `:977`（喂给 `ProjectMenu::new`） | 名字在构造期定死 | **没有** |

#### 2.3.3 判定：**"重建整个 `ShellWorkspace`"，不是"逐个 reset"**

**理由（三条，都基于上面的行号）**：

1. **可复用的 `reset` / `rebuild` 方法不存在**。`rg 'pub fn refresh|pub fn reset|pub fn rebuild|pub fn reload|pub fn set_root'` 在 `gpui/crates/**` 只有 **7 处**命中，
   其中与本任务相关的只有 `Explorer::refresh`(`explorer_view.rs:263`)、`ChangesView::refresh`(`changes_view.rs:325`)、
   `BottomPane` 系的 `refresh`(`log_view.rs:168`) —— **三个都读自己存着的 root，没有一个能重指**；
   `Explorer::rebuild`(`:323`) 是内部树重建。**没有任何 `reset(root)` / `rebuild(root)`。**
2. **两处"根本没法 reset"**：`EditorPane.java`（JDTLS 会话，`editor_view.rs:436-438` 幂等早退）与
   `GIT_IDENTITY_HOST`（`thread_local` + 闭包烘焙根，`identity.rs:311-312,319`）。逐个 reset 这两条**做不到**，
   只能整实体重建。
3. **整实体重建是被上游支持的**：`Window::replace_root<E>(&mut self, cx: &mut App, build_view: impl FnOnce(&mut Window, &mut Context<E>) -> E) -> Entity<E>`
   （`$REG/gpui-pre-0.3.6/src/window.rs:2242-2254`：`cx.new(|cx| build_view(self, cx))` → `self.root = Some(...)` → `self.refresh()`）。
   而且 `ShellWorkspace::new` 会把 5 个 `thread_local` + `GIT_IDENTITY_HOST` **全部重新登记**（§2.1.3 的登记行号），
   所以"新建一份就自动接管"这件事**恰好**对单窗口是对的。

**「此窗口」的最小可行做法【推断·未验证】**：

```
window.replace_root(cx, |window, cx| {
    let workspace = cx.new(|cx| ShellWorkspace::new(new_root, compact_menu, false, window, cx));
    Root::new(workspace, window, cx)          // ⚠️ 窗口根必须是 Root，不是 ShellWorkspace
})
```

⚠️ **两个必须注意的点**：
- 窗口根是 `Root`（`main.rs:708-709`），所以 `replace_root` 的 `E` 要填 `Root`，不是 `ShellWorkspace`。
- **旧 `ShellWorkspace` 会随 `replace_root` 被 drop**。它持有的 `EditorPane`（内含 JDTLS 服务）与 `TerminalPane`（内含终端会话）
  必须自己收尾，否则会留下孤儿 JVM / 终端进程。仓库里**有** `JavaLanguageService::shutdown`（`java/src/service.rs:623`）、
  会话 `shutdown`（`java/src/session.rs:382`、`terminal/src/session.rs:172`），但**没有验证过 `Drop` 是否调用它们**（§4 未确认项 #3）。

---

### 2.4 「最近项目」要落盘在哪

#### 2.4.1 本侧现状：**恒空态，零数据来源**

| 事实 | 位置 |
| --- | --- |
| `const RECENT_PROJECTS: usize = 0;` —— 写成常量，注释自认「v1 的最近项目条数：**恒 0**（没有来源）」 | `gpui/crates/workbench/src/project_menu.rs:662-665` |
| 面板段③ 每次都挂**空态行**（不是按数据渲染） | `build_popup` 的 `:624-627`（`.separator().item(group_label(...)).item(recent_empty_row())`） |
| 空态行本体 | `recent_empty_row()`：`project_menu.rs:556-573`，文案键 `lithe.titleProject.noRecentProjects`（`:568`） |
| 诊断行把 `recent=0` 打成可 grep 契约 | `project_menu.rs:670`（`S1_PROJECT_MENU opened=… recent={RECENT_PROJECTS} …`）与 `:685-687` |
| 段② 「打开的项目」的数据来源（**与最近项目不是一回事**） | `ShellWorkspace::project_entries()`：`workspace.rs:1052-1063`（由 `self.projects` + `self.root` 现算，**每条的 path 都是同一个根**） |

#### 2.4.2 真源：`recent-folders.store.ts` 的存在形式（可照抄的字段全表）

**真源文件**：`windows/tauri/src/features/file-system/stores/recent-folders.store.ts`（227 行）

| 项 | 事实 | 行号 |
| --- | --- | --- |
| **持久化机制** | zustand `persist` + `immer`；`storage: createSafeJSONStorage()`；`partialize: ({ recentFolders }) => ({ recentFolders })` | `:189-199`（`partialize` 在 `:194`） |
| **storage key** | `"lithe-code-recent-folders"` | `:191` |
| **版本** | `version: 2`，**带 `migrate`**：非对象 / 非数组 → `{ recentFolders: [] }`；补齐 `lastOpenedAt`（从 `lastOpened` 解析，解析不出用 `Date.now()`） | `:192`（`version`）、`:200-221`（`migrate`，特别是 `:210-220`） |
| **落盘位置** | `createSafeJSONStorage()` → localStorage（`@/utils/zustand-storage`）。**不是文件**，是前端 localStorage | `:193` |
| **上限** | `MAX_RECENT_PROJECTS = 12`；**pinned 项不计入上限**（`limitRecentFolders`：先按 `sortRecentFolders` 排，再 `pinned` 全留 + `unpinned.slice(0, 12)`） | `utils/recent-folders.ts:3`、`:36-42`（`slice` 在 `:39`，拼接在 `:41`） |
| **排序规则** | 先按 `pinned` 降序（pinned 在前），再按 `lastOpenedAt` 降序 | `utils/recent-folders.ts:26-34`（`:28-30` pinned 优先，`:32` 时间降序） |
| **去重规则** | `upsertRecentFolder`：`folders.find(f => f.path === folderPath)` 命中则更新、并 `filter(f => f.path !== folderPath)` 前置到队首 → **按 `path` 精确去重**（大小写不敏感化没有做） | `utils/recent-folders.ts:64-89`（查找 `:69`，去重 `:87`，`limitRecentFolders` 包在最外 `:85`） |
| 「上次打开」字段 | **有**：`lastOpened: string`（本地化时间字符串，`new Date(ts).toLocaleString()`）+ `lastOpenedAt?: number`（时间戳，排序用真源） | 类型：`types/recent-folders.types.ts:4-5`；格式化 `utils/recent-folders.ts:10-12`；取时间戳（含老数据兜底）`:14-17` |
| **`RecentFolder` 全字段** | `name` / `path` / `lastOpened` / `lastOpenedAt?` / `activeProjectTabId?` / `customIcon?` / `missing?` / `openInNewWindow?` / `pinned?` / `importSourceId?` / `importSourceName?` | `types/recent-folders.types.ts:1-13` |
| **`RecentFolderMetadata`（可更新字段）** | `activeProjectTabId?` / `customIcon?` / `lastOpenedAt?` / `missing?` / `openInNewWindow?` / `importSourceId?` / `importSourceName?` | `types/recent-folders.types.ts:15-23` |
| **动作（8 个）** | `addToRecents` / `importRecentFolders`（从其它 IDE 导入，`uniqueRecentFolderImports` 去重、时间戳按 `importBaseTime - index` 递减）/ `openRecentFolder` / `removeFromRecents` / `removeMissingFromRecents` / `clearRecents` / `togglePinned` / `updateRecentFolder` | `:39-46`（签名）、`:60-187`（实现） |
| **点击最近项目的流程** | ① `getSymlinkInfo` 校验是不是目录 → 不是/失败则 `updateRecentFolder(..,{missing:true})` + toast `fileSystem.recentProjectNotFolder` / `fileSystem.recentProjectUnavailable`；② `chooseProjectOpenDestination({projectName, hasOpenWorkspace})`；③ `executeProjectOpenDecision` → `new-window` 走 `createAppWindow({path,isDirectory:true})`，否则 `handleOpenFolderByPath(path)`；④ 成功后 `addToRecents` | `:94-153`（校验 `:106-120`，决策 `:122-126`，分派 `:128-149`） |

**最小可照抄字段集（若只做"最近项目"列表）**：
`path`（去重键）、`name`（显示名，`getFolderName` = 末段）、`lastOpenedAt`（排序）、
可选 `pinned`（置顶）、`missing`（失效标记）、`openInNewWindow`（上次用的打开方式）。
**「上次打开」就是 `lastOpenedAt`**（真源有，且是排序真源）。

#### 2.4.3 `.lithe/` 目录：**gpui 侧没有写入通路**（所以不能落这里）

| 事实 | 位置 |
| --- | --- |
| gpui 侧对 `.lithe` 的**唯一**触及是"**读** `.lithe/toolchains/jdtls` 当 JDTLS 探测候选根" | `gpui/crates/java/src/jdtls.rs:402-407`（`roots.push(workspace_root.join(".lithe").join("toolchains").join("jdtls"))`） |
| gpui 侧**唯一的 workspace 内写入**是 JDTLS 缓存目录里的 `.lithe-last-used` 标记文件，而那个目录在**应用缓存目录**（Windows `%LOCALAPPDATA%\Lithe\cache\language-servers\jdtls\<key>`），**不在工作区里** | 写：`gpui/crates/java/src/workspace.rs:301-307`；缓存根：`:87`（「Windows \| `%LOCALAPPDATA%\Lithe\cache`」）、`:90-124`；`LAST_USED_MARKER` 常量：`:35` |
| `.lithe/run/*.json` 的读写**归 Core**，gpui 侧**没有任何东西写过** —— 代码里明确登记了这件事 | `gpui/crates/settings/src/run.rs:25-28`（「gpui 侧目前**没有任何东西写过** `.lithe/run/*.json`（全仓库 grep `runConfig` 零命中）」） |
| gpui 设置页也自认**没有项目级存储** | `gpui/crates/settings/src/schema.rs:216-227`（「gpui 侧**没有项目级存储**（`.lithe/run/local.json` 的读写归 Core 的 `runConfig.*`，本侧还没有那条通路）」，`:221`）；对照文案 `gpui/crates/shared/src/i18n.rs:374` |
| `.lithe/` 的真正写入者全在别处 | Windows Tauri：`windows/tauri/src-tauri/src/run.rs:252`（`ensure_lithe_gitignore(root/.lithe/.gitignore)`）、`run.rs:734,920`、`maven.rs:101`（`.lithe/maven/config.json`）；Rust Core：`rust/lithe-core/src/execution/configuration.rs:2568+`（`.lithe/run/*` 的读写）、`rust/lithe-core/src/lsp/languages/catalog.rs:270` |

**结论（回答"最近项目该落到设置文件还是别处"）**：
- **落 gpui 自己的全局设置文件**：`%APPDATA%\Lithe\settings.json`（macOS `~/Library/Application Support/Lithe/settings.json`，Linux `$XDG_CONFIG_HOME/lithe/settings.json`）。
  路径解析：`gpui/crates/settings/src/paths.rs:45-52`（`settings_file_path()`）+ 平台分支 `:61-100`；
  验证覆盖开关：`SETTINGS_FILE_ENV = "LITHE_GPUI_SETTINGS_FILE"`（`:28`）。
- **不要落 `.lithe/`**：gpui 侧**没有**写入通路，也不该由 UI 层去写项目文件（与 `explorer/src/lib.rs:86-87` 登记的"平台层职责"同一约束）。
- 两个落盘层面的**已知代价**：
  1. `Settings` 是**扁平标量结构**（`schema.rs:106-270`：`String` / `bool` / `f64` / `u32`），**没有 `Vec`**。要放"最近项目列表"得新增一个复合字段
     （`Vec<RecentProject>`），并同步 `persistence.rs:144-196` 的**逐键容错表**（那张表是"坏键路径"用的；`:142-143` 说明它必须与字段保持同步，守卫测试是 `every_key_survives_a_round_trip`）。
  2. 设置文件是**用户可编辑的 JSON**，最近项目写进去会与"用户手改设置"混在一份文件里 —— 真源是**另起一个 key**（`lithe-code-recent-folders`），本侧如果想对齐，
     更干净的做法是**另起一个文件**（例如 `%APPDATA%\Lithe\recent-projects.json`），复用 `persistence.rs:236-262` 那套"临时文件 + rename"的原子写。

---

### 2.5 「不再询问」这类偏好现在存哪

#### 2.5.1 本侧：**"询问一次后记住选择"的既有机制 = 有，但只有一条，且不是这一件事**

| 事实 | 位置 |
| --- | --- |
| **唯一的同类机制**：`confirmBeforeDiscard`（丢弃更改前是否先弹确认框） —— 默认 `true`，**立即生效**，落到 `ChangesView::set_confirm_before_discard`，由外壳订阅设置后转发；丢弃路径按它决定弹不弹 | schema：`gpui/crates/settings/src/schema.rs:196-204`（键 `#[serde(rename = "confirmBeforeDiscard")]`，字段 `confirm_before_discard`）；默认值 `:285`；持久化 `persistence.rs:191-196`；转发 `workspace.rs:884-905`（初始）、`:918-922`（变化）；消费 `changes_view.rs:265-272`（字段）、`:309-320`（setter）、`:583,604`（判据）；设置页控件 `settings/src/dialog.rs:1930-1941`；文档表 `dialog.rs:1744,1922` |
| **本侧没有任何 `doNotAsk` / `不再询问` 的存储键** | `rg 'doNotAsk\|不再询问\|confirmBefore'` 的结论：`.rs` 里 `doNotAsk` **零命中**；`不再询问` 只出现在**译文**里（见下） |
| **但文案键已经在了（零新增可用）** —— 与真机截图里那个对话框**逐字对应** | `gpui/crates/shared/locales/lithe.zh-CN.yml:3526-3537`：`projectOpen.title` =「打开项目」(`:3526-3527`)、`projectOpen.where` =「你想在哪里打开项目"{project}"？」(`:3528-3529`)、**`projectOpen.doNotAskAgain` =「不再询问」(`:3530-3531`)**、`projectOpen.cancel` =「取消」(`:3532-3533`)、**`projectOpen.newWindow` =「新窗口」(`:3534-3535`)**、**`projectOpen.thisWindow` =「此窗口」(`:3536-3537`)**；英文同键在 `lithe.en.yml:3526-3537` |
| ⚠️ **这 6 个 `projectOpen.*` 键在 gpui 侧是"孤儿键"**：`rg 'projectOpen'` 在 `gpui/crates/**` 的 `.rs` 里**零命中**（只命中两个 yml） | 建议：实现对话框时直接 `tr("lithe.projectOpen.where")` 等，**零新增文案** |
| 另一处（不相干的）「不再询问」：Java 构建失败横幅 | `lithe.zh-CN.yml:3143`（「此工作区遇到 Java 构建失败时会直接继续启动，不再询问。」） |

#### 2.5.2 真源对应键（照抄对象）

| 真源键 | 默认值 | 真源位置 | 语义 |
| --- | --- | --- | --- |
| **`askWhereToOpenProjects`** | **`true`** | `windows/tauri/src/features/settings/config/default-settings.ts:111` | 「选择打开其他项目时是每次询问、保留在此窗口，还是创建新窗口。」 |
| **`openFoldersInNewWindow`** | **`true`** | `default-settings.ts:112` | 「在新窗口中打开项目」 |
| 两个键的类型定义 | — | `windows/tauri/src/features/settings/types/settings.types.ts:102-103` | — |
| 三态合成（设置页下拉） | — | `windows/tauri/src/features/settings/lib/project-open-preference.ts:8-29`（`ask` → `{askWhereToOpenProjects:true}`；`new-window` → `{askWhereToOpenProjects:false, openFoldersInNewWindow:true}`；`this-window` → 两者 `false`） | 真源设置页把它画成「每次询问 / 此窗口 / 新窗口」三选一 |
| **`□ 不再询问` 勾上后写什么** | — | `windows/tauri/src/features/file-system/controllers/project-open-destination.ts:112-131`（`executeProjectOpenDecision`）：**打开成功之后**才 `updateSetting("openFoldersInNewWindow", destination === "new-window")` + `updateSetting("askWhereToOpenProjects", false)`（`:122-128`）—— **两个键一起写，且是"先打开、后记忆"** | ⚠️ 关键细节 |
| 决策顺序（何时弹对话框） | — | 同文件 `:78-110` `chooseProjectOpenDestination`：① 显式指定目的地 → 直接用；② **没有已打开的工作区 → 直接 `this-window`，不询问**；③ `askWhereToOpenProjects === false` → 按 `openFoldersInNewWindow` 直接定；④ 否则弹对话框 | 真机行为："第一个项目不问，之后才问" |
| 对话框本体 | — | 同文件 `:54-72`：`showChoiceDialogWithCheckbox(t("projectOpen.where", {project}), { title, checkboxLabel: t("projectOpen.doNotAskAgain"), cancelLabel, choices: [newWindow, thisWindow(opts: accent)] })` | — |
| 「有没有已打开的工作区」的判据 | — | 同文件 `:133-135`：`!!rootFolderPath \|\| fileCount > 0 \|\| projectTabCount > 0` | — |
| **gpui 侧这两个键存在吗** | — | **不存在**：`gpui/crates/settings/src/schema.rs` 全量字段（`:106-270`）里没有 `askWhereToOpenProjects` / `openFoldersInNewWindow`；`rg` 在 `gpui/crates/**` 对这两个名字**零命中**（命中全在 `windows/**` 与 `gpui/PLAN.md:738`、`gpui/research/windows/07-settings-ui.md:162`） | **要新增 2 个 `bool` 键**（照 §2.4.3 的落盘路径） |

---

### 2.6 项目标签条（多项目）：与"最近项目"的区别

| 项 | 「打开的项目」 | 「最近项目」 |
| --- | --- | --- |
| **本侧数据** | `ShellWorkspace::projects: Vec<ProjectTab>`（`workspace.rs:484`，**构造期恒 1 项**：`:988` `vec![ProjectTab::new(project_name.clone())]`，全仓库**没有任何 push 第二项的代码**） | **不存在**（`RECENT_PROJECTS = 0`，`project_menu.rs:665`） |
| **本侧渲染** | ① 顶部项目标签条：`project_tabs(&self.projects, self.active_project, …)`（`workspace.rs:1952-1961`，**`projects.len() > 1` 才渲染**，`:1952`）；② 标题栏项目下拉的段②：`project_entries()`（`:1052-1063`） | 段③ 恒空态行（`project_menu.rs:624-627`） |
| **本侧标签条是不是"已打开的项目列表"** | **是**（但退化成 1 项）。`project_tabs.rs` 本身**无状态**（模块文档 `:5-6`：「数据（`tabs`/`active`）与回调全部由调用方传入」），只画 `ProjectTab { name: SharedString }`（`project_tabs.rs:202-219`，字段 `:206`，`#[non_exhaustive]`） | — |
| **本侧是否允许多个项目** | **结构上允许、行为上不允许**：`projects` 是 `Vec`、`active_project: Option<usize>`（`workspace.rs:492`）能表达多项，但没有任何"加入第二个"的入口；`on_close` 也**真的不删**（只把 `active_project` 清成 `len-2`，`workspace.rs:1700-1712`，注释 `:1703` 自认"属项目生命周期，本轮不做"） | — |
| **真源（打开的项目）** | `windows/tauri/src/features/window/stores/workspace-tabs.store.ts`：`projectTabs: ProjectTab[]`，存储键**按窗口**（`getWorkspaceTabsStorageKey(currentWebviewWindow?.label ?? "main")`，`:51` → `persist({ name: workspaceTabsStorageKey, version: 1 })`，`:186,194`）；`ProjectTab` 字段 `id/name/path/isActive/lastOpened/customIcon?/displayAlias?/theme?`（`:22-32`） | `recent-folders.store.ts`（§2.4.2），**全局一份**、跨窗口 |
| **真源一句话区别** | **「打开的项目」= 本窗口此刻打开的标签（每窗口一份，`workspace-tabs.store.ts:51,186`）** | **「最近项目」= 跨会话、跨窗口的历史（全局一份，`recent-folders.store.ts:191`），含 `pinned` / `missing` / `openInNewWindow`** |
| 真源规格逆向（可查） | `gpui/research/windows/11-project-menu.md`（段② 的 `:199` 直接 map 空数组，段③ 才有空态）；`gpui/research/windows/01-shell.md:477`（`titleProject.cloneRepository`） | `gpui/research/windows/10-menu-bar.md:320-328`（真源最近项目上限 12、固定项优先、剔除已打开、按 `lastOpenedAt` 降序） |

---

### 2.7 附带确认的两条（实现顺序要用）

| 事实 | 位置 |
| --- | --- |
| **「克隆仓库」的 Core 通路已经有了**：`git.write` 的 `operation` 取值域**包含 `clone`**，`clone` 用 `remote` 当源、`destination` 当目标路径 | 契约 `shared/contracts/rust-core-api.md:543-554`（`clone` 在 `:548`）、`:610-612`（「`clone` uses `remote` as its source and `destination` as its target path」） |
| gpui 侧调 `git.write` 的入口是**泛型字符串 operation**，加一条 clone 不需要新的信封层 | `gpui/crates/git/src/changes.rs:506-522`（`pub fn write(operation: &str, root: &str, payload: Value) -> WriteOutcome`，请求拼装在 `:507-509`） |
| 真源「克隆仓库…」的入口 | `windows/tauri/src/features/window/components/title-bar/title-project-menu.tsx:191`（`closeAndRun(() => onOpenProjectPicker("clone-repository"))`）；UI 在 `windows/tauri/src/features/window/components/project-picker.tsx:438`、`new-project-content.tsx:117-118`；文案键 `lithe.titleProject.cloneRepository`(`lithe.zh-CN.yml:3568` =「克隆仓库…」) |
| **「打开文件」在真源里不是一个"文件菜单项"**：`file.open`(Cmd+O) 是 **`title: "Open Project"` → 打开项目选择器**；真正的"选一个文件并打开"在真源里只作为 **outline 侧栏的「打开文件」**（`outline.openFile`，用一个 `open({directory:false, multiple:false})` 对话框选文件） | 命令注册：`windows/tauri/src/features/keymaps/commands/command-registry.ts:236-242`（`id: "file.open"`, `title: "Open Project"`, `keybinding: "cmd+o"`）；文件选择器：`windows/tauri/src/features/file-system/controllers/platform.ts:137-148`（`openFile()` → `open({directory:false, multiple:false})`）；唯一调用方：`windows/tauri/src/features/outline/components/outline-sidebar.tsx:30,124`；真源文件菜单 19 项里**没有**「打开文件」（见前报告 §2.1） |
| 编辑器"选中路径 → 新开标签页"的链路**已经通了**（不用动） | `ShellWorkspace::new` 的 `on_open`：`workspace.rs:691-695`；`EditorPane::open`：`editor_view.rs:648`（去重在 `:651-654`，`push Buffer` + `active` 在 `:765-767`） |
| 文件菜单现在**只有「保存」一项** | `menu_bar.rs:305-310`（`TopMenu { id: "file", items: &[MenuItem::Action(MenuAction::Save)] }`，`:309`） |
| 项目下拉三条动作**全禁用**，禁用原因串是 `no_scaffolding` / `no_folder_dialog` / `no_git_clone_call_site` | `project_menu.rs:322-371`（`PanelAction` 枚举 `:322-329`，`precondition()` `:364-370`）；禁用渲染 `:399-425`（`.disabled(true)` 在 `:424`）；装配 `:611-613` |

---

## 3. 实现顺序建议

**总原则**：按"依赖最少 → 依赖最多"排，且**每一步都能单独验收**（不留假按钮）。
维护者要的四件事里，**「打开文件」不依赖任何新东西**，而「新窗口」依赖最多（要动 5 个全局单例）。

### 3.1 第 1 步：打开文件（新开标签页）—— **最小可做版本最干净，建议先做**

| 项 | 内容 |
| --- | --- |
| **最小可做版本** | `Ctrl+O` / 文件菜单新增「打开文件」→ `cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: true, prompt: None })` → `await` receiver → 对每个 `PathBuf` 调 `self.editor.update(cx, \|pane, cx\| pane.open(&path, window, cx))` |
| **依赖** | **零新增依赖**（gpui 自带，§2.2）。只缺"取路径"这一步 |
| **改动面** | `gpui/crates/workbench/src/workspace.rs`（新 `CommandId::OpenFile` 分支，落在 `run_command_id` `:1217` 一族，await 写法照 `:806-828` 的 `async_cx.spawn` + `background_spawn`）、`command_palette.rs`（`CommandId` 枚举 `:187-210` + `id()` `:214-228`）、`menu_bar.rs`（`MENUS` 文件菜单 `:305-310` + `MenuAction` 枚举 `:184-211`）、`editor/src/lib.rs`（`KeyBinding::new`，现有绑定表见前报告 §1.A4） |
| **验收证据** | 打一行与 `S1_*` 同族的诊断（例如 `S1_OPEN_FILE count=… result=opened`），否则无人值守验证没有证据 |
| **风险** | `multiple: true` 与真源（`multiple:false`）不同 → 建议**先按真源 `multiple: false`** 做，多选留到之后 |
| **⚠️ 真源口径** | 真源文件菜单里**没有**这一项（§2.7）。所以"放在文件菜单"是本侧新增；要么按 `outline.openFile` 的口径说清，要么请维护者拍板（见 §4 #5） |

### 3.2 第 2 步：最近项目（列表 + 点击打开）

| 项 | 内容 |
| --- | --- |
| **最小可做版本** | ① 新增持久化（**建议另起 `%APPDATA%\Lithe\recent-projects.json`**，原子写复用 `persistence.rs:236-262`）；字段取 §2.4.2 的 `path` / `name` / `lastOpenedAt`（+ 可选 `pinned` / `missing`）；上限 12；按 `path` 去重、置顶插入、按 `lastOpenedAt` 降序；② `project_menu.rs` 把 `RECENT_PROJECTS` 常量换成真实数据（`:624-627` 的渲染 + `:665/:670/:685-687` 的诊断要一起改，注释 `:663-664` 明写"它变的那一天，面板结构与这条诊断要一起改"）；③ **点击行为先只做"打开文件所在文件夹"或"记下来但暂不换根"** |
| **依赖** | ② 的"点击真的打开项目"依赖第 3 步（换根）；① 不依赖任何东西 |
| **可复用的行元素** | `project_menu.rs:459-520` 的 `project_row`（段② 的当前项目行）—— 最近项目行可以照它的版式（两行：名 + 路径） |
| **必须先定义的语义** | 「点最近项目」如果落在"换根未实现"，就必须像三条动作那样 **`.disabled(true)`**，否则又是假按钮（`project_menu.rs:115-120` 口径） |

### 3.3 第 3 步：打开文件夹（**「此窗口」**）—— 换根

| 项 | 内容 |
| --- | --- |
| **最小可做版本** | ① `prompt_for_paths { files: false, directories: true, multiple: false, prompt: None }` 取根；② `window.replace_root(cx, \|window, cx\| { let ws = cx.new(\|cx\| ShellWorkspace::new(new_root, compact_menu, false, window, cx)); Root::new(ws, window, cx) })`（§2.3.3）；③ **先不做对话框**：因为只有一个窗口时真源也**不询问**（`hasOpenWorkspace === false` → 直接 `this-window`，§2.5.2 决策顺序②）。**"此窗口"是第一个能独立交付的换根形态。** |
| **依赖** | `prompt_for_paths`（第 1 步已接）+ `Window::replace_root`（上游有）+ **`compact_menu` 的取值要能跨帧拿到**（现在是 `main.rs` 的启动参数，`ShellWorkspace::new` 的第二个参数 `:652`）—— 【推断】需要把启动参数提到一个 `App::Global` 或存成字段，否则 `replace_root` 里重建时拿不到 |
| **必须逐个确认的收尾** | 旧 `ShellWorkspace` 被 drop 时：JDTLS 会话（`java/service.rs:623` `shutdown`）、终端会话（`terminal/session.rs:172` `shutdown`）**是否真的被调**（§4 #3）。若不调，换根会留孤儿 JVM/终端 |
| **验收证据** | `S1_*` 一族：换根后 `S1_BRANCH name=…`（`workspace.rs:960`）、`S1_PROJECT_MENU current=…`（`:670`）应变成新项目名 |

### 3.4 第 4 步：`□ 不再询问` + 「新窗口 / 此窗口」对话框

| 项 | 内容 |
| --- | --- |
| **最小可做版本** | ① 新增 **2 个 `bool` 设置键** `askWhereToOpenProjects`（默认 `true`）/ `openFoldersInNewWindow`（默认 `true`）—— 照真源 §2.5.2；② 复刻 `chooseProjectOpenDestination` 的四段决策（`:78-110`）；③ 对话框用 gpui-kit 的 `showChoiceDialogWithCheckbox` 等价物（**⚠️ 本副侦察未核对 gpui-kit 0.6.6 是否有"带 checkbox 的二选一对话框"** —— §4 #4）；④ 「不再询问」的写入**必须在打开成功之后**（真源 `:112-128` 的 `rememberAfterOpen`） |
| **零新增文案** | 6 个 `projectOpen.*` 键**已经存在**（`lithe.zh-CN.yml:3526-3537`），且是幽灵键（§2.5.1） |
| **依赖** | 第 3 步（「此窗口」）+ 第 5 步（「新窗口」）都落地之后才有意义 —— 否则对话框里的"新窗口"按钮是假的 |

### 3.5 第 5 步：`新窗口`（**最后做，且要先重构**）

| 项 | 内容 |
| --- | --- |
| **为什么最后** | §2.1.3：5 个 `thread_local`（+1 个 `GIT_IDENTITY_HOST`）会在第二个 `ShellWorkspace::new` 时被覆盖 → 双窗口串台 |
| **最小可做版本（前置重构）** | 把 §2.1.3 那 6 个 `thread_local` 改成**按窗口寻址**：候选落点是 `Window`/`Root` 上的句柄，或 `App::Global` 里的 `HashMap<WindowId, …>`（`command_palette.rs:113-114` 的注释已经把这条路标出来了）。做完之后 `新窗口` 才是"`cx.open_window(...)` + `Root::new(new ShellWorkspace)`"，与 `main.rs:616-633,709` 同一段代码路径 |
| **风险登记** | ① `--menu-probe` / `--project-menu-probe` / `--palette-keys`（`main.rs:659-746`）都靠这些单例取句柄，重构会同时动到诊断链路；② `set_git_identity_host` 的 `workspace_root` 烘焙（`workspace.rs:783-785`）是**跨 crate 的公共钩子**（`settings/src/identity.rs:319`），改签名会波及 `settings` crate |

### 3.6 「克隆仓库」的落点（可与第 2 步并行评估）

| 项 | 内容 |
| --- | --- |
| **最小可做版本** | 一个"远程 URL + 目标目录"对话框 → `lithe_gpui_git::changes::write("clone", root, json!({"remote": url, "destination": dest}))`（§2.7）→ 成功后按第 3 步换根到 `dest` |
| **依赖** | ① Core 契约**已有** `clone`（`rust-core-api.md:548,610-612`）；② 需要确认 `git.write` 的 `root` 在"目标目录还不存在"时该传什么（**未确认**，§4 #2）；③ 需要"选目标父目录"（= 第 1 步的 `directories: true` 那一档） |
| **⚠️ 前置条件串要同步改** | `project_menu.rs:368` 的 `no_git_clone_call_site`、模块头 `:113`；否则诊断与实现对不上 |

---

## 4. 未确认项 + 下一步确认办法

| # | 未确认的事 | 为什么没确认 | 下一步怎么确认 |
| --- | --- | --- | --- |
| 1 | **gpui-kit 0.6.6 有没有"带 checkbox 的二选一对话框"**（对话框那一颗 `□`） | 本副侦察只核对了 gpui 平台层、没翻 `gpui-component-0.6.6/src/dialog*` | `rg 'checkbox\|Checkbox' $REG/gpui-component-0.6.6/src/dialog` + 读 `show_dialog` 系列 builder；若无，就用 `Dialog` 自绘一行 checkbox（`settings/src/dialog.rs` 已有大量自绘对话框可照抄） |
| 2 | **`git.write` 的 `clone` 在"目标目录不存在"时 `root` 传什么** | 只读到了契约的字段说明，没读 Core 的 `clone` 分支实现 | 读 `rust/lithe-core/src/...` 里 `git.write` 的 `clone` 分支（`rg '"clone"' rust/lithe-core/src`），或看 `shared/fixtures/git/` 有没有 clone 的 fixture |
| 3 | **旧 `ShellWorkspace` 被 drop 时，JDTLS / 终端会话会不会自己关** | `Drop` 实现没查；本轮只确认了 `shutdown` 方法**存在**（`java/service.rs:623`、`java/session.rs:382`、`terminal/session.rs:172`） | `rg 'impl Drop' gpui/crates/java/src gpui/crates/terminal/src`；实机换根后查进程表有没有残留 `java` |
| 4 | **`compact_menu` 等启动参数在 `replace_root` 时怎么拿到** | 它是 `main.rs` 的局部变量，只经由 `ShellWorkspace::new` 的第二个参数进入（`workspace.rs:652`） | 读 `main.rs` 的参数解析段（`:220-260`）确认它是不是 `App::Global`；否则第 3 步要先把它提成 `Global` |
| 5 | **「打开文件」放哪**（真源文件菜单里没有这一项） | 真源只有 `file.open`="Open Project"(Cmd+O) 与 outline 侧栏的 `outline.openFile`（§2.7） | 需维护者拍板：放文件菜单（本侧新增）／放 `Ctrl+O` 指向项目选择器（对齐真源）／两者都要 |
| 6 | **`prompt_for_paths` 在无人值守（`--` 探针）下能不能被脚本驱动** | 它弹的是**系统模态**，本轮没跑 | 若要自动化验收，需另设一条"不经对话框"的探针入口（照 `--open-settings` / `--project-menu-probe` 的既有口径，`main.rs:643-691`） |
| 7 | **最近项目另起文件 vs 塞进 `settings.json`** | **已定**：另起 `recent-projects.json`（`menu-and-open-project-plan.md:35` 的 Q20），理由是最近的机器状态是高频改写，不该混进用户手改的设置文件 | 当时这条依赖"`Settings` 是扁平标量、塞列表要同步 `persistence.rs` 的逐键表"；**那个前提已经不存在**（键表由 schema 派生，见 `persistence.rs` 的模块文档），但结论不变：分文件是为了"高频改写不重写用户文件"，与结构能否容纳列表无关 |
| 8 | **「打开的项目（多项目）」这次要不要一起做** | 维护者这轮只说"打开后项目树变成新项目"，没说要不要真正的多项目标签条 | 若要做，`ProjectTab`（`project_tabs.rs:203-207`）只有 `name`，要先长出 `path`（缺口已登记：`workspace.rs:1043-1045`）；`workspace.rs:1952` 的 `len()>1` 渲染条件与 `on_close`(`:1700-1712`) 都要改 |

---

## 5. 一句话结论（给实现者）

- **多窗口**：gpui **支持**（`App::open_window` 可反复调用，`app.rs:1347`；`windows/window_stack/active_window` 全在，`app.rs:1323-1342`），
  但本仓库**今天只有 1 个调用点**（`main.rs:623`）且被 **5 个 `thread_local` 单例**（`command_palette.rs:115,141`、`menu_bar.rs:766,774`、`project_menu.rs:813`）
  +`GIT_IDENTITY_HOST`（`identity.rs:311-312`）钉成"一份" → **`新窗口` 今天做不真，缺的是窗口级句柄路由，不是 gpui 能力**。
- **对话框**：`App::prompt_for_paths(&self, PathPromptOptions) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>>`（`app.rs:1687-1692`），
  `PathPromptOptions { files, directories, multiple, prompt }`（`platform.rs:2483-2494`）；**三平台都有真实现**（Windows `IFileOpenDialog`、
  macOS `NSOpenPanel`、Linux `ashpd`）→ **"只有 Windows 能用"是错的**；本仓库**一次都没调过**（零调用例）。
- **换根**：**重建整个 `ShellWorkspace`**（`window.replace_root`，`window.rs:2242-2254`），
  因为 6 个实体 + JDTLS 会话 + `GIT_IDENTITY_HOST` 都在构造期绑根且**没有 `set_root`/`reset`**
  （最硬的证据：`editor_view.rs:436-438` 的 `prepare_java` 幂等早退）。
- **最近项目**：落 **`%APPDATA%\Lithe\settings.json`（或另起一份同目录文件）**，**不要落 `.lithe/`**（gpui 侧零写入通路，
  只读 `.lithe/toolchains/jdtls`，`jdtls.rs:402-407`）；字段照抄真源 `recent-folders.store.ts` 的 `path`/`name`/`lastOpenedAt`
  （上限 12、`path` 去重、pinned 优先、`lastOpenedAt` 降序，`utils/recent-folders.ts:3,26-42,64-89`）。
- **「不再询问」**：本侧同类机制只有 `confirmBeforeDiscard` 一条（`schema.rs:196-204`）；
  真源是 **`askWhereToOpenProjects`（默认 true）+ `openFoldersInNewWindow`（默认 true）** 两个键
  （`default-settings.ts:111-112`），勾选后**先打开、后写两个键**（`project-open-destination.ts:112-131`）；
  本侧这 2 个键**都不存在**，但**对话框的 6 个文案键全在**（`lithe.zh-CN.yml:3526-3537`，且是可用未用的幽灵键）。
- **先做哪个**：**「打开文件」**（零新依赖，`workspace.rs` + `command_palette.rs` + `menu_bar.rs` 三处）→ 再做**「最近项目」的持久化** →
  再做**「此窗口」换根**（`replace_root`）→ 最后才碰**「新窗口」**（先拆 5 个全局单例）。
