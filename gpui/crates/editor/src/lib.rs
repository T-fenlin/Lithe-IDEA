//! 编辑区 Feature：**标签栏 + 正文 + 空状态**。
//!
//! ## 职责
//!
//! 对应 Windows 前端的 pane / tab-bar 结构：上面一条常驻标签栏，下面是正文区；
//! 没有活动 buffer 时标签栏仍在，只有正文位置换成空状态。一个打开的 buffer 对应
//! 一个独立的 `EditorState`，打开 / 切换 / 关闭都由 [`EditorPane`] 持有。
//!
//! ## 拆分来源
//!
//! 从 `gpui/shell/src/bin/shell_probe/editor.rs`（833 行，单个 bin crate 里的扁平模块）
//! 逐字拆出，只调整可见性与 import：**不改**布局、尺寸、颜色、文案、交互与数据接线。
//! 下一节是该文件原来的文件头，整体原样搬进本模块文档。
//!
//! ## 文件分工
//!
//! - `buffer.rs`：`Buffer`（打开的文件 + 它的正文状态）与读盘（`read_body` / `notice`）、
//!   文件类型图标（`icon_for_file`）、标签显示名（`display_names` 等 path-shortener 的同名区分）；
//! - `editor_view.rs`：`EditorPane` 的结构体与字段、`new` / `open` / `close`、
//!   全部 `render_*` 与 `impl Render for EditorPane`，以及界面用的度量常量与文案；
//! - `navigation.rs`：代码跳转的**数据侧** —— Core 轻量导航（`lsp.builtinNavigation`）的
//!   请求/响应与列口径换算、`← →` 的跳转历史 [`navigation::JumpHistory`]；
//! - `diagnostics.rs`：JDTLS 诊断的**数据侧** —— 同步正文 + 有界重取 + 列口径换算
//!   （`DiagnosticSet` 要的是**字符列**，Core 给的是 UTF-16 码元列）；
//!   触发时机与陈旧保护写在那个模块的文档里；
//! - `code_actions.rs`：JDTLS 快速修复的**数据侧 + 落地** —— `CodeActionProvider` 适配器
//!   （菜单 / 键位 / 浮层全归上游），选中时用上游的 `apply_lsp_edits` 把编辑写进 buffer
//!   （上游**完全不处理** `edit`，这条与列口径一起写在那里的模块文档里）。

//! ## 阶段 12 接上的一件事：**JDTLS 诊断波浪线**
//!
//! 上游 0.6.6 的编辑器自带诊断集合与波浪线渲染，而且**公开了宿主塞入口**
//! （`EditorState::diagnostics_mut()`），所以这一项是"接线"而不是"自绘"：
//!
//! 1. [`diagnostics::apply`] 把一份 JDT 快照整份写进某个 `EditorState`（换算复用
//!    [`navigation::editor_position`]）；
//! 2. [`EditorPane::schedule_diagnostics`] 在四个时机排一次后台取回
//!    （打开 / 服务就绪 / 正文变化 / 重新加载），
//!    [`EditorPane::apply_diagnostics`] 做代次 + 修订号 + buffer 还在 三段校验后落地；
//! 3. 诊断行 `S1_EDITOR_DIAGNOSTICS file=… count=… severity_max=… attempts=…`，
//!    让"界面上有没有波浪线"能从日志判定。
//!
//! ## 阶段 12 接上的第二件事：**JDTLS 快速修复（`Ctrl+.`）**
//!
//! 上游 0.6.6 的编辑器自带 code action 菜单、`Ctrl+.` 绑定与右键菜单项，
//! **但完全不处理 `edit`**（`CodeActionMenu` 只读标题、`perform_code_action` 只是转发），
//! 所以这一项是"装 provider + 自己落盘"：
//!
//! 1. `code_actions::JavaCodeActionProvider` 实现上游的 `CodeActionProvider`，
//!    经 [`EditorPane::open`] / [`EditorPane::prepare_java`] 装上（只在 Java buffer、
//!    且只在服务句柄可用时 —— 它没有兜底数据源）；
//! 2. 选中时由 provider 调上游的 `EditorState::apply_lsp_edits` 落地：这样才会标脏、
//!    进撤销栈、触发诊断重取（理由写在 `code_actions` 的模块文档里）；
//! 3. 触发路径：`Ctrl+.`（上游自己的绑定）或编辑器右键菜单的 `Show Code Actions`；
//!    ⚠️ **没有 `Alt+Enter`**（上游 0.6.6 全量 grep 零命中），按"不自造键位"的约定不新增；
//! 4. 诊断行 `S1_EDITOR_CODE_ACTION file=… actions=N kind=…`（取菜单）与
//!    `… result=applied edits=N`（真的落地），让"菜单里有没有修复"能从日志判定。
//!
//! ## 公开边界
//!
//! 对外只有这么几样东西（都由本文件 `pub use` 发布）：[`EditorPane`]（`new` / `open` /
//! `cursor_position` / `save_active` / `close_active` / `set_workspace_root` /
//! `set_tab_menu_host_actions`）、它用来回话给外壳的 [`CursorPosition`] 与
//! [`TabMenuHostActions`]（标签右键菜单里两件"只有外壳做得了"的动作），
//! 以及登记快捷键的 [`install_actions`] 与它绑定的三个 action（[`SaveBuffer`] /
//! [`NavigateToDefinition`] / [`CloseActiveTab`]）；
//! `Buffer` 与三个实现模块都留在 crate 内部（`mod buffer; mod editor_view; mod navigation;`），
//! 不出现在 import 路径里（《编码指南》「内部重组时保持 public module path」的反面用法：
//! 新建 crate 直接发布 `pub use`，不让 `buffer` / `editor_view` 这类实现路径变成契约）。
//!
//! 阶段 10 第二批（JDTLS 语义跳转 + 索引缓存，维护者定稿的两批里的第二批）：见
//! `gpui/PLAN.md` §10.3。**编辑器只做接线**，Java 语言服务本体在新 crate
//! `lithe-gpui-java`（`gpui/crates/java`）：
//!
//! 1. **跨文件 / 跨模块**：`F12` / `Ctrl+单击` 同一入口，结果来源换成
//!    `lsp.request{textDocument/definition}`，目标文件没开着就先打开再定位
//!    （`editor_view.rs` 的 `apply_definition`）；
//! 2. **`jdt://` 库源码**：目标的 URI 不是 `file://` 时用 `lsp.request{virtualDocument}`
//!    取回只读正文，开一个以虚拟 URI 为身份的只读 buffer（`open_virtual`）；
//! 3. **打开项目生成索引**：外壳在 `ShellWorkspace::new` 里转发工作区根 →
//!    `EditorPane::prepare_java` 后台起服务并等就绪，缓存键与目录由 Core 的
//!    `lsp.jdtWorkspaceKey` / `java.jdtWorkspaceFingerprint` 决定；
//! 4. **`← →` 对跨文件目标继续可用**（第一批的历史实现不动）。
//!
//! JDTLS 是**增强**：服务不可用时 `navigation::resolve_target` 会退回第一批的轻量导航。
//!
//! ## 原 `editor.rs` 文件头（逐字保留）
//!
//! 编辑区：**标签栏 + 正文 + 空状态**。
//!
//! 形状照 Windows 前端的 `windows/tauri/src/features/panes/components/pane-container.tsx:1094-1100`：
//! 上面一条标签栏、下面是 `relative min-h-0 flex-1 overflow-hidden` 的正文区；
//! **没有活动 buffer 时标签栏仍在**，只有正文位置换成空状态
//! （`windows/tauri/src/features/panes/components/pane-container.tsx:1100` →
//! `windows/tauri/src/features/panes/components/empty-editor-state.tsx`）。
//!
//! 界面规格来源是 Windows 前端（`gpui/UI-MAP.md` 顶部横幅），
//! 观感用 gpui-kit 0.6.6 真实存在的组件与主题 token 实现，不逐层翻译 Tailwind class：
//!
//! > 引用约定：Windows 侧路径一律写全（相对仓库根）；gpui-kit 侧的行号相对
//! > `gpui-component-0.6.6/src/`（`base::` 的少量引用相对 `gpui-base-0.6.6/src/`），
//! > 两类根目录见 `gpui/UI-MAP.md` §1.1 第 7 条。
//!
//! - 标签条 = `component::tab::TabBar` 的 `TabVariant::Underline`
//!   （Windows 活动标签是"透明底 + 底边主色条"= IntelliJ 风格，
//!   `windows/tauri/src/ui/tab-bar.tsx:204-209`）；
//! - 标签 = `Tab::child(...)`（**不是** `Tab::icon()`，后者会丢掉 label）；
//! - 空状态 = `component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription}`；
//! - 正文 = `component::input::Editor`（`EditorState` 每个标签一份，对应真机"一个 buffer 一个
//!   Monaco model"）。
//!
//! 本轮**明确没做的一件事**（只有外观/位置，行为留待后续接线）：
//!
//! 1. ~~**`← →` 只有外观 + 禁用态**~~：阶段 10 第一批已接上真实跳转历史
//!    （[`navigation::JumpHistory`]），无历史时仍是禁用态。
//!
//! 阶段 9（编辑器完善）已经接上的（原来登记在模块文档里的三件事）：
//!
//! 1. **脏标记是真实值**：订阅 `EditorState` 的 `InputEvent::Change` 置脏、写盘成功清零
//!    （`editor_view.rs` 的 `on_input_change` / `write_buffer`）；
//! 2. **`Ctrl+S` 保存**与**防抖自动保存**（默认开启，150ms，带 stale 守卫）；
//! 3. **状态栏的光标位置**由 [`EditorPane::cursor_position`] 提供（外壳观察本实体重绘），
//!    并留 `S1_EDITOR_CURSOR` 诊断行；
//! 4. **`Ctrl+F` / `Ctrl+H` 查找替换**（组件自带面板，阶段 9 的 C）；
//! 5. **关闭未保存 buffer 时确认**（保存 / 放弃修改 / 取消，阶段 9 的 E）。
//!
//! 阶段 10 第一批（Java 代码跳转的**轻量链路**，维护者定稿的两批里的第一批）：见
//! `gpui/PLAN.md` §10。
//!
//! 1. **`F12`** → [`NavigateToDefinition`] action → [`EditorPane::navigate_to_definition`]；
//! 2. **`Ctrl+单击`** → 编辑器包装层的 `on_mouse_down`（同一落点）；
//! 3. **`← →` 回退**（阶段 9 未做的 D）：[`navigation::JumpHistory`]，标签栏那两个按钮
//!    由"恒禁用"改成按历史算。
//!
//! Core 侧只用了既有的**无进程**轻量导航 `lsp.builtinNavigation`
//! （`shared/contracts/rust-core-api.md:135,1143-1151`），**没有**自己解析 Java：
//! 请求/响应与列口径换算都在 [`navigation`]。本批只在**当前文件内**跳转
//! （Core 的轻量导航本身就只返回当前文件的位置），跨文件 / 依赖库源码属于第二批（JDTLS）。
//!
//! 还有几处**组件写死、公开 API 改不动**的尺寸偏差（细节见交付报告）：
//! `TabVariant::Underline` 的标签高度是 36px（真机标签 28px 居中在 36px 条里，
//! `tab/tab.rs:38-41`）、标签字号默认 14px（真机 `--ui-text-chrome` 是 13px，
//! `tab/tab.rs:803-807` 按 `Size` 档位给字号）、标签之间的间距写死 16px
//! （真机 `--lithe-chrome-gap` 是 4px；`tab/tab_bar.rs:393-403` 只把这个 gap 给
//! 内部的 `tabs-inner`，`TabBar` 没有公开的 gap setter，`.gap()` 设的是外层容器）。
//! 这些值都在组件自己的 `render` 里最后写入，外层 `.h()` / `.text_size()` 覆盖不掉
//! （`Tab::style()` 与组件自己写的字段是同一个 `StyleRefinement`，后写的赢）。
//!
//! ## 阶段 11 接上的一件事：**标签右键菜单**
//!
//! 规格是 `gpui/research/windows/09-tab-context-menu.md`，真源是
//! `windows/tauri/src/features/tabs/components/tab-context-menu.tsx`（13 项 + 4 条分隔线）。
//! 本侧画 **9 项 + 1 条分隔线**（复制路径 / 复制相对路径 / 在资源管理器中显示 /
//! 在终端中打开 / 重新加载 / ── / 关闭 / 关闭其他 / 关闭右侧 / 全部关闭），
//! 缺的 4 项按真源"条件不满足就整项不出现"的口径**整项不画**（不是置灰 ——
//! 真源整份菜单没有任何禁用项）：`pin`（本侧没有 pin 能力）、`rename-terminal`
//! （只对终端标签出现）、`split-right` / `split-down` / `toggle-editor-group-lock`
//! （依赖 pane 树，本侧是扁平 `buffers` + 单窗格）。
//!
//! 落点：菜单挂载在 [`EditorPane::render_tab`] 的内容 div 上（[`EditorPane::with_tab_menu`]），
//! 三个批量关闭与确认对话框在 [`EditorPane::close_scope`] / [`EditorPane::open_unsaved_dialog`]，
//! `Ctrl+W`（[`CloseActiveTab`] → [`EditorPane::close_active`]）是"关闭当前"。
//! 「在资源管理器中显示」与「在终端中打开」要起进程 / 改外壳布局，所以由外壳
//! （`ShellWorkspace`）通过 [`TabMenuHostActions`] 登记两个回调进来。
//!
//! 本轮**范围外**（真机有、这里没有）还有：标签悬停才显示关闭按钮的那一档
//! （`TabBar` 不暴露每个标签的悬停状态）、标签拖拽重排 / 拖出成新窗格、
//! 面包屑栏、分屏与轮播、外部冲突横幅与大文件「仍然启用」降级、
//! 非 UTF-8 文件的编码探测与"按编码保存"。
//!
//! 另外两条与阶段 10 第二批直接相关的边界（细节见 `gpui/PLAN.md` §10.4）：
//! `jdt://` 的 buffer 被关掉之后不能用 `←` 回去（虚拟正文只来自 Core 的虚拟文档请求，
//! 没有"重新读盘"这条路）；Java 运行时的选择靠发现 + `LITHE_JDTLS_JAVA` 覆盖，
//! 还没有做成设置项。

mod buffer;
mod code_actions;
mod completion;
mod diagnostics;
mod editor_view;
mod navigation;

pub use editor_view::{CursorPosition, EditorPane, TabMenuHostActions};

use gpui_kit::{App, KeyBinding};

gpui_kit::actions!(lithe_editor, [SaveBuffer, NavigateToDefinition, CloseActiveTab]);

/// 登记编辑区的应用级快捷键。**每个窗口调用一次**（`ShellWorkspace::new`）。
///
/// 三条：`ctrl-s` → [`SaveBuffer`]、`f12` → [`NavigateToDefinition`]、
/// `ctrl-w` → [`CloseActiveTab`]；命中后都由
/// `ShellWorkspace` / [`EditorPane`] 根元素的处理器接住（见各自的 `on_action`）。
///
/// `ctrl-w` 的语义照真源 `file.close` → `closeActiveTab`
/// （`windows/tauri/src/features/keymaps/commands/file-command-actions.ts:75-86`）：
/// **关闭当前标签**，不是"关闭全部"、也不是"关闭窗口"（真机在没有任何 buffer 时会关窗口，
/// 这一支**不复刻**，见 [`EditorPane::close_active`] 的文档）。
///
/// `KeyBinding::new(.., None)` 的上下文谓词是空，`binding_enabled` 会按
/// `contexts.len()`（**最深**）算深度（`gpui-pre-0.3.6/src/keymap.rs:246-252`），
/// 也就是"任何焦点下都可能命中、且优先级最高"。`ctrl-s` 在整个应用里没有别的绑定
/// （gpui-base / gpui-component 全量 grep 零命中），所以给它 `None` 是安全的。
///
/// ⚠️ `f12` 也**没有**和组件撞车：gpui-base 的 `GoToDefinition` action
/// （`gpui-base-0.6.6/src/input/base/state.rs:117`、处理器在
/// `input/editor/lsp/definitions.rs:109-126`）在本组件里**没有登记任何按键**
/// （`gpui-base` / `gpui-component` 全量 grep `f12` 零命中），而且它是"先 Ctrl+悬停
/// 缓存过位置、再按键"才动的那种；我们的 [`NavigateToDefinition`] 是独立 action，
/// 由 Core 的轻量导航直接给目标，不依赖悬停缓存。
///
/// ⚠️ **这里没有 `Search` / `Replace` 的绑定，这是有意的**：`Ctrl+F` / `Ctrl+H`
/// 由**组件自己**接管 —— `EditorState::new` 默认 `searchable = true`
/// （`gpui-base-0.6.6/src/input/base/state.rs:9233`），而 `gpui_base::input::init`
/// 已经在 `Input` 上下文里绑好 `ctrl-f` → `Search` / `ctrl-h` → `Replace`
/// （同文件 `:296-302`），编辑器元素自己带 `key_context("Input")` 并有对应的
/// `on_action`（`gpui-base-0.6.6/src/input/base/state.rs:4176,4229-4230`）。
///
/// 曾经试过"在编辑区自己的键上下文里再绑一份 `ctrl-f` 作为兜底"，但实测**不可达**：
/// 点标签栏不会让焦点离开编辑器（点完再敲字仍然进入编辑器，`S1_EDITOR_CURSOR` 照常推进），
/// 而聚焦编辑器时 `Input` 上下文更深、组件内置绑定先命中并 `stop_propagation`。
/// 兜底绑定既然永远不会触发，就是死代码，已删除（《编码指南》「避免投机性抽象」）。
pub fn install_actions(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-s", SaveBuffer, None),
        KeyBinding::new("f12", NavigateToDefinition, None),
        // `ctrl-w` 在整个依赖树里没有别的绑定（`gpui-base` / `gpui-component` 全量 grep
        // `ctrl-w` 零命中），所以给它 `None` 也是安全的。
        KeyBinding::new("ctrl-w", CloseActiveTab, None),
    ]);
}
