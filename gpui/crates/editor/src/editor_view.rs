//! 编辑区的**表现层**：标签栏（`← →` 导航组 + 各文件的标签 + 关闭按钮）与正文 / 空状态。
//!
//! 从 `shell_probe/editor.rs` 原样拆出（逐字搬迁，只调整可见性与 import）。
//! 本文件只负责画与交互；读盘、图标与标签显示名在 `buffer.rs`。
//!
//! ## 阶段 9 接上的四件事
//!
//! | 项 | 落点 |
//! | --- | --- |
//! | A 脏标记 | [`EditorPane::on_input_change`]（订阅 `InputEvent::Change`）→ [`crate::buffer::Buffer::is_dirty`] |
//! | A `Ctrl+S` | [`crate::SaveBuffer`] action + [`EditorPane::save_active`] → [`EditorPane::write_buffer`] |
//! | A 防抖自动保存 | [`EditorPane::schedule_auto_save`] / [`EditorPane::run_auto_save`]（默认开启，见 [`AUTO_SAVE_ENABLED`]） |
//! | B 状态栏光标 | [`EditorPane::sync_cursor`]（`CursorPosition` + `S1_EDITOR_CURSOR` 诊断） |
//! | C 查找替换 | [`EditorPane::open`] 里显式写出的 `searchable(true)` —— 面板与 `Ctrl+F` / `Ctrl+H` 都归组件（见该处注释） |
//! | E 关闭确认 | [`EditorPane::request_close`]（`保存` / `放弃修改` / `取消`） |

use std::path::Path;
use std::time::Duration;

use crate::buffer::{Buffer, display_names, icon_for_file, read_body};
use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::input::{Editor, EditorState, InputEvent};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::tab::{Tab, TabBar, TabVariant};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, Sizable as _, WindowExt as _};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollWheelEvent, SharedString, Styled as _, Window, div, point,
    px, relative,
};
use lithe_gpui_shared::{tr, tr_args};

// ---------------------------------------------------------------------------
// 度量：一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// rem base = 主题字号 16px，所以 helper 后缀 `N` = `N × 4px`，与 Windows 规格逐像素相等：
//
// | 规格（Windows 真源） | 值 | 用到的 helper |
// | --- | --- | --- |
// | 标签栏高 `--lithe-tab-bar-height: 2.25rem`（`styles/theme.css:120-121`、`ui/tab-bar.tsx:248`） | 36 | `h_9()` |
// | 标签栏左右内边距 `--lithe-chrome-padding-inline`（`theme.css:132`） | 8 | `px_2()` |
// | 标签栏段间距 `--lithe-chrome-gap`（`theme.css:130`） | 4 | `mr_1()` |
// | 标签 `pl-2` / `pr-6`（`ui/tab-bar.tsx:260`） | 8 / 24 | `pl_2()` / `pr_6()` |
// | 标签图标↔文字间距 `--lithe-chrome-gap-loose`（`theme.css:131`） | 6 | `gap_1p5()` |
// | 标签图标槽 `size-3`（`tab-bar-item.tsx:183`） | 12 | `size_3()` |
// | 导航组 `gap-0.5`（`tab-bar.tsx:633`） | 2 | `gap_0p5()` |
// | 关闭按钮 `absolute right-1`（`tab-bar-item.tsx:164-166`） | 4 | `right_1()` |
// | 脏标记圆点 `size-2`（`tab-bar-item.tsx:284-291`） | 8 | `size_2()` |
// | 空状态 `px-6 py-8` / `gap-3`（`empty-editor-state.tsx:15-16`） | 24 / 32 / 12 | `px_6()` / `py_8()` / `gap_3()` |
// | 空状态内容块 `max-w-md`（`empty-editor-state.tsx:16`） | 448 | `max_w_112()` |
// | 图标块 `size-12` / 主图标 `size-10` / 放大镜 `size-5`（`empty-editor-state.tsx:17-22`） | 48 / 40 / 20 | `size_12()` / `size_10()` / `size_5()` |
//
// 字号：`--ui-text-base` / `--ui-text-sm` 在这里都是 **13px**（`theme.css:116-117`），不在 gpui 的
// 档位（`text_xs()`=12 / `text_sm()`=14）上，按《编码指南》用 **`text_sm()`（14px）**——
// 13 → 14 是经维护者确认的**有意**视觉改动。

/// 单个标签宽度上限 200px：`--lithe-tab-max-width`（12.5rem，`windows/tauri/src/styles/theme.css:123`，
/// 用在 `windows/tauri/src/ui/tab-bar.tsx:260`）。**这是标签文字能截断的前提**。
///
/// ⚠️ **保留 `px(...)`**：200 不在 gpui 的固定 rem 档位上（档位里 48 → 192、56 → 224，
/// `gpui-pre-macros-0.3.6/src/styles.rs:1039-1047`），没有 `max_w_50()`。
const TAB_MAX_WIDTH: f32 = 200.;

/// 滚轮增量换算用的行高兜底值：真机默认行高就是 **20**
/// （`windows/tauri/src/features/editor/config/constants.ts:5` 的 `DEFAULT_LINE_HEIGHT: 20`；
/// 算法是 `ceil(fontSize × 1.4)`，`windows/tauri/src/features/editor/utils/lines.ts:8-15`）。
/// 编辑器还没完成首次布局时 `line_height()` 是 `None`，用这个值兜底，
/// 否则 `ScrollDelta::Lines` 换算出 0，整段滚不动。
///
/// 它是滚轮增量换算里的 `Pixels`（与 `line_height()` 的返回值同类型做算术：
/// `event.delta.pixel_delta(line_height)`、`if delta == px(0.)`），不是布局样式槽，
/// 套不了 `Styled` 的 `line_height` 系列 helper，所以保留 `px(...)`。
const FALLBACK_LINE_HEIGHT: f32 = 20.;

/// 防抖自动保存的等待窗口 **150ms**：逐值照 Windows 的 `setTimeout(.., 150)`
/// （`windows/tauri/src/features/editor/stores/editor-app.store.ts:567`）。
const AUTO_SAVE_DEBOUNCE: Duration = Duration::from_millis(150);

/// 自动保存总开关，**默认开启**。
///
/// Windows 的真源是设置项 `editor.autoSave`，默认值就是 `true`
/// （`windows/tauri/src/features/settings/config/default-settings.ts:49`）；语义是"编辑时
/// 防抖自动保存"，脏标记与手动 `Ctrl+S` 是**独立**的另一条路径（同文件 `:503-570`）。
///
/// ⚠️ **这里是常量而不是读设置**：阶段 9 的范围由维护者定稿为"只做编辑器侧"，
/// gpui 的设置文件（`gpui/crates/settings/src/schema.rs` 目前只有主题 / 字号 / 状态栏 /
/// 语言）与设置页这一轮不扩。接设置页时把它改成 `Settings` 的对应字段即可，
/// 调用点只有 [`EditorPane::on_input_change`] 一处。
const AUTO_SAVE_ENABLED: bool = true;

/// 状态栏用的光标位置：**1 基**行列。
///
/// 显式用 1 基而不是拿组件的 0 基 `lsp_types::Position` 直接用，是因为"第几行第几列"
/// 的换算与显示口径（`line + 1` / `column + 1`）只该有一处实现：
/// 真机就是 `editor-status-actions.tsx:29` 的 `` `${line + 1}:${column + 1}` ``。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CursorPosition {
    /// 行号，1 基。
    pub line: usize,
    /// 列号，1 基。
    pub column: usize,
}

impl CursorPosition {
    /// 没有活动 buffer 时的位置。
    ///
    /// 真机拿不到任何光标位置时返回 `INITIAL_CURSOR_POSITION = {line: 0, column: 0}`
    /// （`windows/tauri/src/features/editor/utils/editor-view-cursor-position.ts:3`），
    /// 显示出来同样是 `1:1`。
    const INITIAL: Self = Self { line: 1, column: 1 };
}

/// 编辑区视图：标签栏 + 正文（正文没有活动 buffer 时是空状态）。
pub struct EditorPane {
    /// 打开的 buffer，顺序就是标签栏里的顺序。
    buffers: Vec<Buffer>,
    /// 活动 buffer 在 [`Self::buffers`] 里的下标；`None` = 没有活动 buffer → 空状态。
    active: Option<usize>,
    /// 上一次报给外壳的光标位置；用来把"光标真的动了"和"编辑器又重绘了一次"分开，
    /// 否则外壳会跟着每一次编辑器重绘重排状态栏。
    cursor: Option<CursorPosition>,
}

impl EditorPane {
    /// 建一个没有打开任何文件的编辑区。
    ///
    /// 真机启动时编辑区就是空状态（标签栏在、正文是空状态），
    /// 打开动作由用户触发（`panes/components/pane-container.tsx:1100`）。
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            buffers: Vec::new(),
            active: None,
            cursor: None,
        }
    }

    /// 打开一个文件：读盘、判定类型、更新标签栏与正文。
    ///
    /// **同一路径重复打开只切换活动标签、不再读盘**（沿用上一轮已验证实现的约定，
    /// git HEAD `panels.rs::open_document`）：正文已经在这个 [`Buffer`] 的
    /// `EditorState` 里，重读会白白丢掉撤销栈与光标。
    /// 注意"同路径"是按传入的 `Path` 字面比较；调用方要保证同一个文件每次传同一种写法
    /// （绝对路径就一直绝对路径）。
    ///
    /// 关闭标签会把该 buffer 的 `EditorState`（以及缓存正文）一起丢掉 —— 真机同样会释放
    /// 对应 model，所以"重复打开不读盘"只对**仍然打开着**的标签成立。
    pub fn open(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.to_path_buf();

        if let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) {
            self.activate(index, cx);
            return;
        }

        let name: SharedString = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string())
            .into();
        let body = read_body(&path, &name);
        let writable = body.writable;

        // 一个标签一个 `EditorState`：先建状态再灌正文，然后才入列。
        let editor = cx.new(|cx| {
            // `searchable` 上游默认就是 `true`
            // （`gpui-base-0.6.6/src/input/base/state.rs:9233` 的 `EditorState::new`），
            // 显式写出来是把 C（`Ctrl+F` 查找替换）依赖的前提钉在调用点：
            // **不 searchable 的编辑器不拦截 `Ctrl-F`**，会冒泡到上层
            // （`gpui/docs/gpui-kit/0.6.6/zh-CN/component/editor.md:158-159`）。
            let mut state = EditorState::new(window, cx).searchable(true);
            if !writable {
                // 说明性正文（读不到 / 超大 / 二进制 / 非 UTF-8）不可写：只读能避免
                // "用户以为改了、其实保存的是说明文案"这种更坏的结果。
                state.set_readonly(true, cx);
            }
            state
        });
        editor.update(cx, |state, cx| state.set_value(body.text, window, cx));

        // 订阅必须在 `set_value` **之后**：`set_value` 内部关掉了事件发射
        // （`gpui-base-0.6.6/src/input/base/state.rs:904-907` 的 `emit_events = false`），
        // 但顺序反过来仍然更容易踩坑，所以固定成"先灌正文、再订阅"。
        let subscriptions = vec![
            // A：脏标记 + 防抖自动保存。
            cx.subscribe_in(
                &editor,
                window,
                |pane: &mut Self,
                 editor: &gpui_kit::Entity<EditorState>,
                 event: &InputEvent,
                 window,
                 cx| {
                    if matches!(event, InputEvent::Change) {
                        pane.on_input_change(editor, window, cx);
                    }
                },
            ),
            // B：光标位置。光标移动不发事件（`InputEvent` 只有 Change / PressEnter /
            // Focus / Blur），但会 `cx.notify()`（例如 `select_to`，
            // `gpui-base-0.6.6/src/input/base/state.rs:3039`），所以走观察。
            cx.observe(&editor, |pane: &mut Self, _editor, cx| pane.sync_cursor(cx)),
        ];

        // 实参从左到右求值，图标要在 `name` 被移进构造函数之前算好。
        let icon = icon_for_file(&name);
        self.buffers
            .push(Buffer::new(path, name, icon, editor, writable, subscriptions));
        self.active = Some(self.buffers.len() - 1);
        self.sync_cursor(cx);
        cx.notify();
    }

    /// 当前活动 buffer 的光标位置（**1 基**行列，状态栏的显示口径）。
    ///
    /// 只读：给外壳的状态栏用。外壳通过 `cx.observe(&editor_pane, ..)` 重绘，
    /// 而本视图只在位置**真的变了**的时候 `notify`（见 [`Self::sync_cursor`]）。
    pub fn cursor_position(&self, cx: &App) -> CursorPosition {
        self.active
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| {
                // 组件给的是 0 基 `lsp_types::Position`
                // （`gpui-base-0.6.6/src/input/base/state.rs:1232`）。
                let position = buffer.editor.read(cx).cursor_position();
                CursorPosition {
                    line: position.line as usize + 1,
                    column: position.character as usize + 1,
                }
            })
            .unwrap_or(CursorPosition::INITIAL)
    }

    /// `Ctrl+S`：把**活动** buffer 写回磁盘。
    ///
    /// 由 `ShellWorkspace` 根元素的 `SaveBuffer` action 处理器转发进来（见 `install_actions`）。
    /// 说明性正文（[`Buffer::writable`] = `false`）没有可写的内容，直接忽略。
    pub fn save_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            return;
        };
        if !self
            .buffers
            .get(index)
            .is_some_and(|buffer| buffer.writable)
        {
            return;
        }
        self.write_buffer(index, "lithe.editor.saveFailed", window, cx);
    }

    /// 切到第 `index` 个标签（重复打开同一路径、点标签都走这里）。
    fn activate(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.buffers.len() {
            return;
        }
        self.active = Some(index);
        // 状态栏的光标项要立刻跟着换 buffer，不能等下一次编辑器通知。
        self.sync_cursor(cx);
        cx.notify();
    }

    /// 光标位置变了就通知外壳重绘，并留一行可 grep 的诊断。
    ///
    /// 光标位置是渲染出来的小字，靠截图逐像素核对不可靠，所以状态一变就打
    /// `S1_EDITOR_CURSOR`（与 `S1_EXPLORER` / `S1_SETTINGS` 同一套探针口径）。
    fn sync_cursor(&mut self, cx: &mut Context<Self>) {
        let position = self.cursor_position(cx);
        if self.cursor == Some(position) {
            return;
        }
        self.cursor = Some(position);
        println!(
            "S1_EDITOR_CURSOR line={} col={}",
            position.line, position.column
        );
        cx.notify();
    }

    /// `InputEvent::Change`：置脏并排一次防抖自动保存。
    fn on_input_change(
        &mut self,
        editor: &gpui_kit::Entity<EditorState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self
            .buffers
            .iter()
            .position(|buffer| buffer.editor.entity_id() == editor.entity_id())
        else {
            return;
        };
        // 只读的说明性 buffer 不会发 Change（没有编辑动作能被接受），这里是第二道闸。
        if !self.buffers[index].writable {
            return;
        }

        if !self.buffers[index].is_dirty {
            self.buffers[index].is_dirty = true;
            cx.notify();
        }

        if AUTO_SAVE_ENABLED {
            self.schedule_auto_save(index, window, cx);
        }
    }

    /// 排一次防抖自动保存。
    ///
    /// 两件事都与 Windows 一一对应：**换掉待执行的任务**（`Task` drop 即取消）等价于
    /// `clearTimeout`（`editor-app.store.ts:505-509`）；**版本号 +1** 是 stale-content 守卫
    /// 的等价物（同文件 `:518-529`，理由见 [`Buffer::save_generation`]）。
    fn schedule_auto_save(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let generation = self.buffers[index].save_generation.wrapping_add(1);
        self.buffers[index].save_generation = generation;

        let task = cx.spawn_in(window, async move |pane, cx| {
            cx.background_executor().timer(AUTO_SAVE_DEBOUNCE).await;
            // 编辑区可能已经销毁：`update_in` 返回 `Err` 时静默忽略，不 panic。
            let _ = pane.update_in(cx, |pane, window, cx| {
                pane.run_auto_save(index, generation, window, cx)
            });
        });
        self.buffers[index].auto_save_task = Some(task);
    }

    /// 防抖窗口结束时真的写盘。
    fn run_auto_save(
        &mut self,
        index: usize,
        generation: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        // stale-content 守卫：这一代已经被后来的改动替换（或者已经被手动保存过），
        // 交给新的那一次写，这里什么都不做。
        if buffer.save_generation != generation || !buffer.is_dirty {
            return;
        }
        self.write_buffer(index, "lithe.editor.autoSaveFailed", window, cx);
    }

    /// 把 buffer 的正文写回磁盘；成功返回 `true`。
    ///
    /// 失败**不静默**（《编码指南》「Handle failures explicitly」）：内容留在编辑器里、
    /// `is_dirty` 保持 `true`、打一行 `S1_EDITOR_SAVE_FAILED`，并推一条通知。
    /// 文案用 Windows 的 `editor.saveFailed` / `editor.autoSaveFailed`
    /// （`windows/tauri/src/i18n/locale.ts:4637-4638`；英文侧同键 `:176-177`），
    /// 手动保存与自动保存各用一条 —— 两句话说的是不同的后果（"重试"vs"更改仍在编辑器里"）。
    fn write_buffer(
        &mut self,
        index: usize,
        failure_key: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(buffer) = self.buffers.get(index) else {
            return false;
        };
        let path = buffer.path.clone();
        let name = buffer.name.clone();
        let value = buffer.editor.read(cx).value();

        match std::fs::write(&path, value.as_bytes()) {
            Ok(()) => {
                if let Some(buffer) = self.buffers.get_mut(index) {
                    buffer.is_dirty = false;
                    // 已经在盘上了：待执行的自动保存没有意义，顺手丢掉。
                    buffer.auto_save_task = None;
                }
                println!(
                    "S1_EDITOR_SAVED path={} bytes={}",
                    path.display(),
                    value.len()
                );
                cx.notify();
                true
            }
            Err(error) => {
                eprintln!(
                    "S1_EDITOR_SAVE_FAILED path={} error={error}",
                    path.display()
                );
                window.push_notification(
                    // 保存失败是**错误**而不是普通提示：用 error 档（自带语义色与图标）。
                    Notification::error(tr_args(failure_key, &[("name", name.as_ref())])),
                    cx,
                );
                false
            }
        }
    }

    /// 关闭一个标签。
    ///
    /// 关掉最后一个标签后编辑区回到空状态（`windows/tauri/src/features/panes/components/pane-container.tsx:1100`）；
    /// 关掉活动标签时顺位接上它的邻居（真机就是"关掉后激活相邻标签"）。
    ///
    /// ⚠️ 调用方要在之后调一次 [`Self::sync_cursor`]：`self.cursor` 是"上次报出去的值"，
    /// 不刷新的话关标签后状态栏会停在旧 buffer 的行列上，直到下一次编辑器通知。
    fn close(&mut self, index: usize) {
        if index >= self.buffers.len() {
            return;
        }

        // 被关掉的标签之后的标签整体左移一位，所以旧下标大于 index 的要减一。
        let shift = |active: usize| if active > index { active - 1 } else { active };
        self.buffers.remove(index);

        self.active = if self.buffers.is_empty() {
            None
        } else {
            Some(
                self.active
                    .map(shift)
                    .unwrap_or(index)
                    .min(self.buffers.len() - 1),
            )
        };
    }

    /// E：请求关闭第 `index` 个标签 —— **有未保存修改时先确认**。
    ///
    /// 干净的标签直接关，不打扰用户（真机也只有 `isDirty` 才拦：
    /// `windows/tauri/src/features/editor/stores/buffer.store.ts:1746` 找的就是
    /// `isEditorContent(b) && b.isDirty`）。
    ///
    /// ⚠️ 浮层只能在事件回调或任务里打开；本函数由关闭按钮的 `on_click` 调用，满足约束
    /// （`render` 阶段调 `window.open_dialog` 会 panic）。
    fn request_close(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };

        if !buffer.is_dirty {
            self.close(index);
            self.sync_cursor(cx);
            cx.notify();
            return;
        }

        let name = buffer.name.clone();
        let pane = cx.weak_entity();

        window.open_dialog(cx, move |dialog, _, cx| {
            let discard_pane = pane.clone();
            let save_pane = pane.clone();
            dialog
                // 标题 = 状态/条件，正文 = 作用范围（哪个文件），按钮 = 结果词。
                // 指南 `design-guides.md:427-432`：确认对话框要组成一个紧凑决策；
                // `:434` 明确不用"您确定要……吗"这类套话，所以正文只补"是哪个文件"。
                .title(tr("lithe.unsavedChanges.title"))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr_args(
                            "lithe.editor.gpui.unsavedChangesBody",
                            &[("name", name.as_ref())],
                        )),
                )
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        // 按钮顺序照指南「操作使用"取消"和结果词」；破坏性结果放中间并保持
                        // 次样式，主操作（保存）最右且 primary —— 与设置对话框的
                        // 「恢复默认设置」确认框同一套排法（`settings/src/dialog.rs:586-617`）。
                        .child(
                            Button::new("editor-unsaved-cancel")
                                .label(tr("lithe.ui.cancel"))
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("editor-unsaved-discard")
                                // 真机的字是「不保存」（`unsavedChanges.doNotSave`，
                                // `windows/tauri/src/i18n/locale.ts:7873`），但设计指南
                                // `design-guides.md:421` 对"未保存修改"这一档明确推荐
                                // **「放弃修改」**（"不保存"看不出放弃了什么），所以这里用
                                // gpui 侧自有键，理由登记在 `extract-locale.mjs` 的
                                // `GPUI_ONLY_KEYS`。
                                .label(tr("lithe.editor.gpui.discardChanges"))
                                .on_click(move |_, window, cx| {
                                    let _ = discard_pane.update(cx, |pane, cx| {
                                        pane.close(index);
                                        pane.sync_cursor(cx);
                                        cx.notify();
                                    });
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("editor-unsaved-save")
                                .primary()
                                .label(tr("lithe.ui.save"))
                                .on_click(move |_, window, cx| {
                                    // 写失败**不关**对话框：用户还可以重试或改成"放弃修改"，
                                    // 失败原因由 `write_buffer` 推的通知说明。
                                    let saved = save_pane
                                        .update(cx, |pane, cx| {
                                            pane.write_buffer(
                                                index,
                                                "lithe.editor.saveFailed",
                                                window,
                                                cx,
                                            )
                                        })
                                        .unwrap_or(false);
                                    if saved {
                                        window.close_dialog(cx);
                                    }
                                }),
                        ),
                )
        });
    }

    /// 标签栏：左侧 `← →` 导航组 + 各文件的标签。
    ///
    /// 组件的选型与为什么这么用：
    ///
    /// - `TabVariant::Underline` 是 gpui-kit 里唯一"活动标签 = 透明底 + 底边主色条"的变体
    ///   （`tab/tab.rs:253-262`：`bg` 透明、`border_b` 2px `primary`），与 Windows 的
    ///   IntelliJ 风格一致（`windows/tauri/src/ui/tab-bar.tsx:204-209`：`bg-transparent` +
    ///   `before:h-[3px] before:bg-primary`）；
    /// - Underline 变体的标签条自带 1px 下边框（`tab/tab_bar.rs:501-514`，
    ///   `border_b_1()` + `theme.border`），正好是 Windows 的 `border-b border-border`；
    /// - Underline 变体的条底色是**透明**、外层 padding 是 0
    ///   （`tab/tab_bar.rs:393-403`），所以这里显式补 `.bg(theme.tab_bar)` 与
    ///   `.px_2()`（8px），把 Windows 的 `bg-tab-bar` + `px-(--lithe-chrome-padding-inline)` 找回来；
    /// - `.h_9()`（36px）是**必须**的：标签条自身没有高度，没有标签时会被压成 0；
    ///   真机里没有活动 buffer 时标签栏也照常占着 36px（`windows/tauri/src/features/panes/components/pane-container.tsx:1094-1100`）。
    fn render_tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let names = display_names(&self.buffers);

        let mut bar = TabBar::new("editor-tab-bar")
            .with_variant(TabVariant::Underline)
            .bg(cx.theme().tab_bar)
            .h_9()
            .px_2()
            // `max_width` 让**标签文字**在空间不够时让位（图标与关闭按钮保持原尺寸），
            // 也就是真机那种 `OrderChargeService.j…` 的截断。
            .max_width(TAB_MAX_WIDTH)
            .prefix(Self::render_nav_group())
            .on_click(cx.listener(|pane, index: &usize, _window, cx| {
                // `TabBar::on_click` 给的是被点标签的下标
                // （`tab/tab_bar.rs:168-177`），切换活动 buffer 就是切标签。
                pane.activate(*index, cx);
            }));

        for (index, (buffer, name)) in self.buffers.iter().zip(names).enumerate() {
            bar = bar.child(self.render_tab(index, buffer, name, cx));
        }

        if let Some(active) = self.active {
            bar = bar.selected_index(active);
        }

        bar.into_any_element()
    }

    /// 标签栏左侧的后退/前进按钮组。
    ///
    /// 真机是一个 `h-8` 的行（`windows/tauri/src/features/tabs/components/tab-bar.tsx:633-660`），
    /// 与后面的标签区之间有标签栏自己的 4px gap；`TabBar` 的 `Underline` 变体不给
    /// 外层容器设 gap，所以这里用右外边距补上同样的 4px。
    fn render_nav_group() -> impl IntoElement {
        h_flex()
            .items_center()
            .gap_0p5()
            .mr_1()
            // 文案取中文 i18n 原文：tooltip = `tabs.goBackShort` / `tabs.goForwardShort`
            // （"后退" / "前进"，`windows/tauri/src/i18n/locale.ts:7854-7855`），
            // 无障碍名 = `tabs.goBack` / `tabs.goForward`
            // （"后退到上一个位置" / "前进到下一个位置"，`windows/tauri/src/i18n/locale.ts:7850-7851`）。
            .child(Self::nav_button(
                "editor-nav-back",
                IconName::ArrowLeft,
                tr("lithe.tabs.goBackShort"),
                tr("lithe.tabs.goBack"),
            ))
            .child(Self::nav_button(
                "editor-nav-forward",
                IconName::ArrowRight,
                tr("lithe.tabs.goForwardShort"),
                tr("lithe.tabs.goForward"),
            ))
    }

    /// 单个导航按钮。
    ///
    /// 真机：`variant="ghost" size="icon-xs"`、`disabled={!canGoBack}`、
    /// tooltip `tooltipSide="bottom"`（`windows/tauri/src/features/tabs/components/tab-bar.tsx:634-659`）。
    ///
    /// ⚠️ **尺寸陷阱**：真机的 `icon-xs` 是 **24×24**（`windows/tauri/src/ui/button.tsx:27`
    /// 的 `icon-xs size-6`），而 gpui-kit 的 `Sizable::xsmall()` 给图标按钮是 **20×20**、
    /// `small()` 才是 24×24（`button/button.rs:618-623`）。这里对齐的是**尺寸值**，
    /// 所以用 `.small()`，不要被变体名带偏。
    ///
    /// ⚠️ **本轮只有外观 + 禁用态，没有历史栈**：探针没有 jump list，
    /// 所以两个按钮恒为 `disabled(true)`（禁用态 ghost = 灰图标，`button/button.rs:1273-1306`），
    /// 而不是"点了没反应"的假按钮。接上历史栈后改成 `disabled(!can_go_back)`
    /// 并在 `on_click` 里跳转即可。
    fn nav_button(
        id: &'static str,
        icon: IconName,
        tooltip: SharedString,
        label: SharedString,
    ) -> Button {
        Button::new(id)
            .icon(icon)
            .ghost()
            .small()
            .disabled(true)
            .tab_stop(false)
            .tooltip(tooltip)
            .accessibility_label(label)
    }

    /// 一个标签：文件类型图标 + 显示名（+ 未保存圆点）+ 关闭按钮。
    ///
    /// ⚠️ **图标 + 文件名必须走 `Tab::child(...)`**：`Tab::icon()` 只渲染图标，
    /// label 与 children 在图标分支里被整个丢掉（`tab/tab.rs:719-744`，
    /// 图标分支没有 `.children(self.children)`）。
    ///
    /// 颜色照 Windows（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:276`：活动 `--foreground`、非活动
    /// `--subtle-foreground`；`:250`：图标恒为 `--subtle-foreground`）。
    /// 这里必须**自己给文字设色**：`TabBar` 会把标签的 `text_color` 换成自己的
    /// `tab_foreground`/`tab_active_foreground`（`tab/tab.rs:253-264`），
    /// 而 Windows 的非活动标签要的是更暗的 `subtle-foreground`。
    ///
    /// 关闭按钮用**绝对定位**放在标签里，不走 `Tab::suffix`：
    /// suffix 是 flex 项，会算进标签宽度，切换标签时标签宽度会跳；
    /// 绝对定位后宽度只由 `pr(24px)` 决定，与真机一致（`right-1` + `pr-6`）。
    fn render_tab(
        &self,
        index: usize,
        buffer: &Buffer,
        name: SharedString,
        cx: &mut Context<Self>,
    ) -> Tab {
        let is_active = self.active == Some(index);
        let label_color = if is_active {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };

        // 无障碍名照真机拼「名称 +（未保存）」（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:137-141`、
        // `windows/tauri/src/i18n/locale.ts:7860` 的 `tabs.ariaUnsavedSuffix`）；
        // 固定/预览后缀本轮没有这两种状态。必须在 `name` 被移进正文之前算好。
        let aria_label = if buffer.is_dirty {
            SharedString::from(format!("{name}（未保存）"))
        } else {
            name.clone()
        };

        // 未保存圆点：8×8、`rounded-full`、`primary`（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:284-291`）。
        // `is_dirty` 现在是**真实值**：`InputEvent::Change` 置位（`EditorPane::on_input_change`）、
        // 写盘成功清零（`EditorPane::write_buffer`）。
        let dirty_dot = buffer.is_dirty.then(|| {
            div()
                .flex_shrink_0()
                .size_2()
                .rounded_full()
                .bg(cx.theme().primary)
        });

        // 关闭按钮：只有活动标签显示。真机默认设置是
        // `tabCloseButtonVisibility: "active"`（`windows/tauri/src/features/settings/config/default-settings.ts:96`），
        // 规则见 `windows/tauri/src/features/settings/lib/ui-preferences.ts:13-19`
        // （固定标签恒显 / `always` 全显 / `active` 只有活动标签 / 否则悬停才显）。
        // 悬停那一档没做：`TabBar` 不暴露每个标签的悬停状态，见报告的"未能实现"一节。
        //
        // 绝对定位而不是 `Tab::suffix`：suffix 是 flex 项，会算进标签宽度，
        // 切换标签时标签宽度会跳；绝对定位后宽度只由正文常驻的 `pr(24px)` 决定，
        // 与真机一致（关掉按钮绝对定位在 `right-1` 上，见 `windows/tauri/src/features/tabs/components/tab-bar-item.tsx:164-166`）。
        let close = is_active.then(|| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right_1()
                .flex()
                .items_center()
                .child(Self::close_button(index, cx))
        });

        let content = h_flex()
            .items_center()
            .pl_2()
            .pr_6()
            .gap_1p5()
            // 没有 `min_w_0` 的话 flex 项的最小尺寸会按内容算，文字截断不了。
            .min_w_0()
            .child(
                // 全量目录的 `IconName` 是 `Copy`（`gpui-kit-assets-0.6.6/build.rs:52-53`
                // 的 `#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, IntoElement)]`），
                // 从借用里直接取出即可。
                Icon::new(buffer.icon)
                    .size_3()
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_color(label_color)
                    .child(name),
            )
            .children(dirty_dot)
            .children(close);

        Tab::new().aria_label(aria_label).child(content)
    }

    /// 标签上的关闭按钮。
    ///
    /// 真机：ghost 图标按钮 `icon-xs`（24×24），tooltip `tabs.close`（"关闭"，
    /// `windows/tauri/src/i18n/locale.ts:7845`），点击关闭该标签
    /// （`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:150-180`）。
    /// gpui-kit 的 `Button` 点击时会 `stop_propagation`（`button/button.rs:797-808`），
    /// 所以点关闭不会顺带把标签激活。
    /// 尺寸用 `.small()` 的理由（gpui-kit 的 `small()` = 24×24、`xsmall()` = 20×20）
    /// 见 [`TAB_CLOSE_INSET`] 的注释。
    fn close_button(index: usize, cx: &mut Context<Self>) -> Button {
        Button::new(format!("editor-tab-close-{index}"))
            // Windows 的关闭字形是 lucide `x`，`icons/x.svg` 在全量目录里确有该字形。
            .icon(IconName::X)
            .ghost()
            .small()
            .tab_stop(false)
            .tooltip(tr("lithe.tabs.close"))
            .accessibility_label(tr("lithe.tabs.close"))
            .on_click(cx.listener(move |pane, _event, window, cx| {
                // E：脏标签要先过确认对话框（`request_close`）。
                pane.request_close(index, window, cx);
            }))
    }

    /// 正文区：有活动 buffer 时是代码编辑器，没有时是空状态。
    fn render_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(buffer) = self.active.and_then(|index| self.buffers.get(index)) else {
            return Self::render_empty_state(cx);
        };
        // 渲染闭包要 `'static`，先把活动编辑器的句柄取出来（`&Buffer` 借用到此结束）。
        // 两份：滚轮闭包会把它 move 走，正文还要再渲染一次同一个编辑器。
        let editor = buffer.editor.clone();
        let scroll_editor = editor.clone();

        div()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            // ⚠️ **滚轮必须自己接**：编辑器元素的样式只有 `position: absolute` + `100%`，
            // **没有** `overflow: scroll`（`gpui-base-0.6.6/src/input/base/element.rs:202-213`），
            // 而 gpui 只把滚轮交给"命中元素样式里带 `Overflow::Scroll`"的那个
            // （`gpui-pre-0.3.6/src/elements/div.rs:3332-3370`）。
            // 做法是在包裹层接，把增量交给编辑器自己的滚动偏移
            // （`InputBaseState::{scroll_offset, set_scroll_offset}`，
            // `gpui-base-0.6.6/src/input/base/state.rs:2806-2817`；后者会自己 clamp、
            // 下一帧生效）。
            .on_scroll_wheel(
                cx.listener(move |_pane, event: &ScrollWheelEvent, _window, cx| {
                    let line_height = scroll_editor
                        .read(cx)
                        .line_height()
                        // 兜底行高 20：滚轮增量换算里的 `Pixels`（不是样式槽），见常量注释。
                        .unwrap_or(px(FALLBACK_LINE_HEIGHT));
                    let delta = event.delta.pixel_delta(line_height).y;
                    if delta == px(0.) {
                        return;
                    }
                    let current = scroll_editor.read(cx).scroll_offset();
                    scroll_editor.update(cx, |state, cx| {
                        state.set_scroll_offset(point(current.x, current.y + delta), cx);
                    });
                }),
            )
            // ⚠️ **多行编辑器必须显式给高度**：多行走 `.h_auto()`，高度 = **内容高度**，
            // 而内容高度来自 `LayoutMode::CodeEditor { rows }`，`rows` 默认只有 2
            // （`gpui-component-0.6.6/src/input/input.rs:706-709`、
            // `gpui-base-0.6.6/src/input/base/mode.rs:83-85`）—— 不给高度时编辑区只有两行高、
            // 下面整片空白。`relative(1.)` 让它填满外层这一格
            // （等价于组件自带的 `Input::full_height()`，`input.rs:250-252`）。
            //
            // `.bordered(false)`：真机的编辑面是**无边框**的纯表面
            // （`windows/tauri/src/features/editor/components/code-editor.tsx:650,731`：
            // `absolute inset-0 bg-background`，没有 border），而 `Editor` 默认
            // `bordered: true`（`gpui-component-0.6.6/src/input/editor.rs:46`）会画出
            // 输入框那种 1px `theme.input` 描边（`input/input.rs:713-715`）。
            // 焦点环不用管：`Editor` 已经把 `focus_bordered(false)` 写死了
            // （`input/editor.rs:146`）。
            .child(Editor::new(&editor).h(relative(1.)).bordered(false))
            .into_any_element()
    }

    /// 没有活动 buffer 时的空状态。
    ///
    /// 结构照 `windows/tauri/src/features/panes/components/empty-editor-state.tsx`：
    /// 居中一列 = 图标块（48×48 的 `FileText` 40×40 + 右下角 20×20 放大镜）
    /// + 标题 + 说明（`:15-30`）。文案取**中文 i18n 原文，逐字**：
    ///
    /// - 标题 `workbench.emptyEditorTitle` = "选择文件以查看"（`windows/tauri/src/i18n/locale.ts:6150`）；
    /// - 说明 `workbench.emptyEditorDescription` = "外部工具产生的更改会自动显示。"
    ///   （`windows/tauri/src/i18n/locale.ts:6151`）；
    /// - 右键菜单只有一项禁用项 `ui.noActionsHere` = "此处无任何内容"
    ///   （`windows/tauri/src/i18n/locale.ts:4630`；空态菜单在
    ///   `windows/tauri/src/features/panes/components/empty-editor-state.tsx:32-34`）。
    fn render_empty_state(cx: &App) -> AnyElement {
        // 外层用 `v_flex` 而不是裸 `div`：`Empty` 自己带 `flex_1`，
        // 但"flex 项"只在 flex 容器里才成立 —— 裸 div 里它会塌成内容高度，
        // 垂直居中就没了。
        v_flex()
            .flex_1()
            .min_h_0()
            .context_menu(|menu, _window, _cx| {
                menu.item(PopupMenuItem::new(tr("lithe.ui.noActionsHere")).disabled(true))
            })
            .child(
                Empty::new()
                    // ⚠️ 真机空状态**没有边框**；`Empty` 的 render 里硬编码了
                    // `.border_dashed().border_color(cx.theme().border)`
                    // （`gpui-component-0.6.6/src/empty.rs:74-75`），
                    // 所以把边框色改成透明来关掉那圈虚线。
                    .border_color(cx.theme().transparent)
                    .gap_3()
                    .px_6()
                    .py_8()
                    .header(
                        EmptyHeader::new()
                            .max_w_112()
                            .gap_3()
                            .media(
                                EmptyMedia::new().child(
                                    div()
                                        .relative()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .size_12()
                                        .child(
                                            Icon::new(IconName::FileText)
                                                .size_10()
                                                .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(
                                            Icon::new(IconName::Search)
                                                .absolute()
                                                .right_0()
                                                .bottom_0()
                                                .size_5()
                                                .text_color(cx.theme().muted_foreground),
                                        ),
                                ),
                            )
                            // 字号：Windows 的 `ui-text-base` / `ui-text-sm` 都是 13px
                            // （`windows/tauri/src/styles/theme.css:116-117`），不在 gpui 的档位上，
                            // 按《编码指南》用 `text_sm()`（14px，经维护者确认的有意改动；
                            // 正好等于组件默认值，写出来是为了标明"这就是规格值"）。
                            // 颜色用组件默认：标题 `foreground`、说明 `muted_foreground`（`empty.rs:264-324`）。
                            .title(
                                EmptyTitle::new()
                                    .text_sm()
                                    .child(tr("lithe.workbench.emptyEditorTitle")),
                            )
                            .description(
                                EmptyDescription::new()
                                    .text_sm()
                                    .child(tr("lithe.workbench.emptyEditorDescription")),
                            ),
                    ),
            )
            .into_any_element()
    }
}

impl Render for EditorPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 两层：标签栏在上，正文占剩下的高度。容器要 `min_h_0` + `overflow_hidden`，
        // 否则正文里的编辑器会把整个视图撑出父容器。
        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .child(self.render_tab_bar(cx))
            .child(self.render_body(cx))
    }
}
