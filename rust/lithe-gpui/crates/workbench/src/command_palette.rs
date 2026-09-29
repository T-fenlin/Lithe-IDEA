//! 命令面板浮层（阶段 6 第二半的骨架）。
//!
//! 真源：`windows/tauri/src/features/command-palette/components/command-palette.tsx`。
//! 它是一层**浮层**：搜索输入 + 动作列表（分类 / 标签 / 可选描述），
//! `↑`/`↓` 移动、`Enter` 执行、`Esc` 关闭、空结果给一句空态。
//!
//! ## 快捷键（**取自真源，不是惯例**）
//!
//! 真源命令 `workbench.commandPalette` 的默认绑定是 `cmd+shift+p`
//! （`features/keymaps/defaults/default-keymaps.ts:492-496`），而真机默认预设是
//! `keybindingPreset: "none"`（`features/settings/config/default-settings.ts:144`），
//! 所以生效的就是 `defaultKeymaps` 里那一条。Windows 上绑定在解析时把 `cmd` 归一化成
//! `ctrl`（`utils/platform.ts:47-54` 的 `normalizeKey`，由
//! `features/keymaps/utils/parser.ts:76` 调用），因此**真机 Windows 上按的是 `Ctrl+Shift+P`**。
//! 其它预设还给了别名（JetBrains / Xcode 的 `cmd+shift+a`、Emacs 的 `alt+x`，
//! `keybinding-presets.ts:94,119,145`），本侧不做预设体系，只接默认那一条。
//!
//! ⚠️ 绑定注册成**全局 action**（[`install_actions`]），理由与
//! `lithe_gpui_settings::install_actions` 完全相同：gpui 的按键派发在没有焦点元素时只走
//! "窗口根节点"这一条路径，挂在 `ShellWorkspace` 的根元素上会有"启动后没点过任何地方时
//! 按不出来"的死角。
//!
//! ## 容器：`Dialog` + `Command`，两者都不可替换（读源码得出）
//!
//! - **必须有 `Dialog`**：`component::command::{Command, CommandState}` 只是普通的
//!   `v_flex` + 内部 `Input`（0.7 里依旧没有遮罩、没有定位，也不在窗口层叠序里），
//!   文档给的唯一浮层用法就是"放进 `window.open_dialog`"（组件文档说的"命令面板"
//!   指的就是它本身）。本侧照 `lithe_gpui_settings::open_settings_dialog` 的口径组合：
//!   Dialog 负责遮罩 / Escape / 焦点陷阱 / 关闭，`Command` 负责搜索框、过滤、虚拟列表与行高亮。
//! - **不新增挂载点**：dialog 层由窗口根 `Root` 的 plugin 层每帧渲染（gpui-kit 0.7 起
//!   不再由 `ShellWorkspace::render` 手动挂），本模块只调 `window.open_dialog`。
//! - ❌ **不用 `CommandState` 的 `Action` 机制**：`CommandItem::action` 要求
//!   `Box<dyn Action>`（`command/item.rs:15,64`），实现它要么引 `anyhow` + `serde`
//!   （`Action::build` 收 `serde_json::Value` 并返回 `anyhow::Result`，
//!   `gpui-pre-0.3.6/src/action.rs:135`），要么引 `#[derive(Action)]` 需要的 `serde` /
//!   `schemars`；本轮不新增依赖，所以用 `on_confirm` 的 `IndexPath` 直接查动作表
//!   （未分组条目 `section = 0`、`row` = 动作表行号，`command/state.rs:359-368`）。
//!   代价见「与真源的差异」第 4 条。
//!
//! ⚠️ 浮层只能在**事件回调或任务**里打开；`render` 阶段调 `window.open_dialog` 会 panic
//! （`docs/gpui-kit/0.6.6/zh-CN/shell/overlays.md:200-227`）。本模块唯一的打开入口是按键回调。
//!
//! ## 动作从哪来：`ShellWorkspace` 持有
//!
//! 面板能执行的动作**全部落在 `ShellWorkspace` 的状态上**（终端 / 右工具窗可见性）或
//! `SettingsStore` 上（主题 / 状态栏），所以动作表由 [`ShellWorkspace::command_actions`]
//! 生成、由 [`ShellWorkspace::run_command`] 执行；本模块只负责画与选择。
//! 浮层的 builder 与 `Command` 的回调都要求 `'static`，不能捕获工作台的引用，所以用
//! [`set_shell`] / [`shell`] 这一对线程局部句柄（gpui 的窗口与视图都在主线程）。
//!
//! ## 度量（真源 `windows/tauri/src/ui/command.tsx`）
//!
//! | 规格 | 值 | 出处 | 本侧写法 |
//! | --- | --- | --- | --- |
//! | 面板宽 | `w-[min(44rem,calc(100vw-2rem))]` = 704 | `ui/command.tsx:29` | `rems(704. / 16.)`（档位外，见下） |
//! | 面板最大高 | `max-h-[min(68vh,32rem)]` = 512 | 同上 | `rems(512. / 16.)` → `Dialog` 的 `.max_h()` |
//! | 距窗口顶 | `pt-16` = 16 | `ui/command.tsx:143`（`items-start justify-center pt-16`） | `crate::rem_px(window.rem_size(), 16.)`（`Dialog::margin_top` 是固有方法，只吃 `Pixels`） |
//! | 行样式 | 最小 32、圆角 8、`px-2.5 py-2`、标签 + 描述两层 | `ui/command.tsx:41,528-537` | `CommandItem::child` 自绘（见 `ShellWorkspace::command_actions`） |
//! | 搜索行 / 空态 / 行高亮 | 组件自己 | `command/state.rs:841-858,773-788` | 交给 `Command`（成熟实现，不重做） |
//!
//! 704 / 512 都不在 gpui 的固定档位上（`gpui-pre-macros-0.3.6/src/styles.rs:926-1158`），
//! 按《编码指南》写成 helper 底层的 `rems(P / 16.)`（**不是** `/ 4.`：rem base = 16px）。
//! `Dialog::width` / `margin_top` 是**固有方法**、只收 `Pixels`（`dialog/dialog.rs:395,413`；
//! 它们遮蔽了收 `impl Into<AbsoluteLength>` 的 `Styled` 同名方法），而 gpui 没有
//! `impl From<Rems> for Pixels`，所以照 `lithe_gpui_settings::dialog` 的做法走
//! [`crate::rem_px`]（`AbsoluteLength::to_pixels`，`gpui-pre-0.3.6/src/geometry.rs:3361`），
//! 基准取**当帧的** `window.rem_size()` —— 写死 `px(16.)` 就是假 rem。
//!
//! ## 与真源的差异（有意，逐条）
//!
//! 1. **动作集是子集**：真源有 200+ 条动作（markdown / git / github / database / pane /
//!    vim …），其中大量能力在本侧还不存在，放进来就是"点了没反应"，所以只放**每一条都
//!    真的改到状态**的动作（清单见 [`ShellWorkspace::command_actions`]）。
//! 2. **没有二级视图**（真源的 `color-theme` / `icon-theme` / `local-history` / `outline`
//!    以及扩展注册的视图，`command-palette.tsx:398-438`）：本侧把"切配色主题"做成一级动作，
//!    少了"列全部主题"那一层；图标主题在本侧没有对应子系统，不做。
//! 3. **没有「最近使用」置顶**（真源 `persistentCommands` 的 `lastEnteredActions`，
//!    `command-palette.tsx:345-356`）：那要一份跨会话的动作历史，属后续。
//! 4. **没有快捷键提示列**（真源每行尾部渲染 `Keybinding`，`:461-476`）：`Command` 的提示位
//!    来自被绑定 `Action` 的解析结果（`command/state.rs:722-729`），而本侧走 `on_confirm`
//!    直接派发自己的动作表，没有 `Action` 可解析 —— 代价见上面「不用 Action 机制」。
//! 5. **`Esc` 先清空查询、再关面板**：这是 `Command` 自己的语义
//!    （`command/state.rs:567-582`；`docs/gpui-kit/0.6.6/zh-CN/shell/overlays.md:192` 保证
//!    Escape 只关最上层浮层），与真源一致，不是差异 —— 写在这里免得被当成 bug：
//!    "按一次 Esc 没关掉"是因为搜索框里还有字。

use std::cell::RefCell;

use gpui_kit::assets::IconName;
use gpui_kit::base::v_flex;
use gpui_kit::component::command::{Command, CommandItem, CommandState};
use gpui_kit::component::{ActiveTheme as _, WindowExt as _};
use gpui_kit::{
    App, AppContext as _, Context, DefiniteLength, Entity, IntoElement, KeyBinding,
    ParentElement as _, Render, SharedString, Styled as _, WeakEntity, Window, div, rems,
};

use lithe_gpui_shared::tr;

use crate::workspace::ShellWorkspace;

// ---------------------------------------------------------------------------
// 全局句柄：打开的浮层要够得着工作台
// ---------------------------------------------------------------------------

thread_local! {
    /// 当前窗口的工作台句柄。
    ///
    /// 为什么需要它：`window.open_dialog` 的 builder 与 `Command` 的 `on_confirm` 都要求
    /// `'static`，不能捕获 `ShellWorkspace` 的引用；而动作要改的正是工作台的状态
    /// （终端 / 右工具窗可见性）。`WeakEntity` 保证窗口关掉之后这里不会吊住整个视图。
    ///
    /// 为什么不是 `Global`：那是 `App` 级存储，而"当前窗口的这个视图"在一个进程里
    /// 可以有多份，放进去会互相覆盖。
    static SHELL: RefCell<Option<WeakEntity<ShellWorkspace>>> = const { RefCell::new(None) };
}

/// 登记工作台句柄（[`ShellWorkspace::new`] 调用一次）。
pub(crate) fn set_shell(shell: WeakEntity<ShellWorkspace>) {
    SHELL.with(|slot| *slot.borrow_mut() = Some(shell));
}

/// 取当前窗口的工作台句柄。
fn shell() -> Option<WeakEntity<ShellWorkspace>> {
    SHELL.with(|slot| slot.borrow().clone())
}

thread_local! {
    /// 外壳根元素的焦点句柄。
    ///
    /// **为什么需要它**：gpui 的按键派发路径由"当前焦点节点"决定 ——
    /// `dispatch_key_event` 拿 `focus_node_id_in_rendered_frame(self.focus)`
    /// （`gpui-pre-0.3.6/src/window.rs:5815-5816`），`focus` 为 `None` 时路径为空，
    /// keymap 绑定一条都匹配不到（同文件 `:5880` 的 `dispatch_key`）。
    /// 真机上这个焦点来自"用户点过界面里的某个元素"，**启动后什么都不点就没有焦点**，
    /// 于是 `Ctrl+Shift+P` / `Ctrl+,` 这类全局快捷键一个都不响 —— 本轮在
    /// `--palette-keys` 的诊断路径上实测到了这个死角（`focused=false`）。
    ///
    /// 所以外壳根元素挂一个 focus handle（`.track_focus()`）当**兜底锚点**：
    /// 只要没有任何后代元素持有焦点，键盘事件就能落在外壳上，全局 action 照常派发。
    static SHELL_FOCUS: RefCell<Option<gpui_kit::FocusHandle>> = const { RefCell::new(None) };
}

/// 登记外壳根元素的焦点句柄（[`ShellWorkspace::new`] 调用一次）。
pub(crate) fn set_shell_focus(focus: gpui_kit::FocusHandle) {
    SHELL_FOCUS.with(|slot| *slot.borrow_mut() = Some(focus));
}

/// 取外壳根元素的焦点句柄（无人值守启动时用它建立焦点，见 [`SHELL_FOCUS`]）。
pub fn shell_focus() -> Option<gpui_kit::FocusHandle> {
    SHELL_FOCUS.with(|slot| slot.borrow().clone())
}

thread_local! {
    /// 面板的搜索框还没拿到焦点。
    ///
    /// **为什么必须有这一步**：`Command` 的默认焦点是它自己的搜索输入框
    /// （`CommandState::focus`，`command/state.rs:280-287`），而 `window.open_dialog` 把焦点
    /// 给了 **Dialog 自己**（`root.rs:309-310`）。不显式要一次的话：
    ///
    /// - `↑`/`↓`/`Enter` 落不到 `CommandState` 的 `key_context` 上（面板看得见、键盘没反应）；
    /// - `WM_CHAR` 会被丢掉 —— Windows 的字符输入走**已安装的输入处理器**
    ///   （`gpui-pre-windows-0.3.6/src/events.rs:477-484` 的 `with_input_handler`），
    ///   而没有焦点就没有输入处理器。
    ///
    /// 置位在 [`open_command_palette`]（那里刚把面板建出来），消费在
    /// [`CommandPalette::render`] —— 只有到渲染那一刻，`Command` 的输入框才真的在元素树上。
    static PENDING_FOCUS: RefCell<bool> = const { RefCell::new(false) };
}

// ---------------------------------------------------------------------------
// 动作清单
// ---------------------------------------------------------------------------

/// 命令面板里可执行的动作。
///
/// 每个变体对应一段**真的会改状态**的逻辑（[`crate::workspace::ShellWorkspace::run_command`]）；
/// [`CommandId::id`] 是诊断行 `S1_COMMAND_RUN id=…` 的取值，同时进搜索关键词。
///
/// 与真源逐条对应关系写在 [`crate::workspace::command_action`] 的文档上。
///
/// ⚠️ 这里有一批**不是命令面板的**变体：[`CommandId::ToggleMenuBar`] 只由菜单栏的
/// `Ctrl+M` 与 `run_menu_action` 触发；B1 新增的 [`CommandId::OpenFile`] 与
/// [`CommandId::CloseTab`]…[`CommandId::ReopenClosedTab`] 八条只由**主菜单「文件」**触发
/// （命令面板里不列它们，见 [`crate::workspace`] 的 `COMMAND_ORDER`：面板只放
/// "一屏能扫完的高频动作"）。之所以挂在同一张表上，是因为它们都要一个**稳定 id**
/// 与一个 `Action` —— 那正是本枚举已有的两件事。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandId {
    /// 打开设置对话框（常规页）。
    OpenSettings,
    /// 打开设置对话框（外观页）。
    OpenAppearanceSettings,
    /// 把配色主题切到浅色。
    SwitchToLightTheme,
    /// 把配色主题切到深色。
    SwitchToDarkTheme,
    /// 显示 / 隐藏终端。
    ToggleTerminal,
    /// 显示 / 隐藏右侧的 Maven 工具窗。
    ToggleMaven,
    /// 显示 / 隐藏状态栏。
    ToggleStatusBar,
    /// 切换菜单栏的两种形态（常驻 / 左上角图标下拉）。**不在命令面板里**。
    ToggleMenuBar,
    /// 保存当前文件（`Ctrl+S`；菜单「文件 → 保存」也走它）。
    SaveBuffer,
    /// 打开命令面板（菜单「编辑 → 命令面板」也走它）。
    OpenCommandPalette,
    /// 转到定义（`F12`；菜单「转到 → 转到定义」也走它）。
    NavigateToDefinition,
    /// 打开文件…（菜单「文件 → 打开文件」，**不绑键位**：`Ctrl+O` 留给「打开文件夹」，Q18）。
    OpenFile,
    /// 关闭标签页（菜单「文件 → 关闭标签页」；`Ctrl+W` 是同一个能力的快捷键入口）。
    CloseTab,
    /// 关闭其他标签页。
    CloseOtherTabs,
    /// 关闭所有标签页。
    CloseAllTabs,
    /// 关闭已保存标签页。
    CloseSavedTabs,
    /// 关闭左侧标签页。
    CloseTabsToLeft,
    /// 关闭右侧标签页。
    CloseTabsToRight,
    /// 重新打开已关闭标签页。
    ReopenClosedTab,
}

impl CommandId {
    /// 诊断行与搜索关键词用的稳定 id（**改了就是改可 grep 的契约**）。
    pub const fn id(self) -> &'static str {
        match self {
            Self::OpenSettings => "open-settings",
            Self::OpenAppearanceSettings => "open-appearance-settings",
            Self::SwitchToLightTheme => "switch-theme-light",
            Self::SwitchToDarkTheme => "switch-theme-dark",
            Self::ToggleTerminal => "toggle-terminal",
            Self::ToggleMaven => "toggle-maven",
            Self::ToggleStatusBar => "toggle-status-bar",
            Self::ToggleMenuBar => "toggle-menu-bar",
            Self::SaveBuffer => "save-buffer",
            Self::OpenCommandPalette => "open-command-palette",
            Self::NavigateToDefinition => "navigate-to-definition",
            // B1 的一批：全部**只由主菜单触发**（`COMMAND_ORDER` 里没有它们，所以命令面板
            // 不列）—— 面板要的是"一屏能扫完的高频动作"，而"关闭左侧/右侧标签页"这类
            // 动作在面板里出现只会挤掉别的。
            Self::OpenFile => "open-file",
            Self::CloseTab => "close-tab",
            Self::CloseOtherTabs => "close-other-tabs",
            Self::CloseAllTabs => "close-all-tabs",
            Self::CloseSavedTabs => "close-saved-tabs",
            Self::CloseTabsToLeft => "close-tabs-to-left",
            Self::CloseTabsToRight => "close-tabs-to-right",
            Self::ReopenClosedTab => "reopen-closed-tab",
        }
    }
}

/// 动作行的分类（真源 `Action.category` 的取值域里本侧用到的两个，
/// 本地化键见 `category_label`）。
#[derive(Clone, Copy)]
pub(crate) enum Category {
    Settings,
    View,
}

impl Category {
    /// 分类的本地化名。真源走 `commandPalette.categories.<Category>`
    /// （`features/command-palette/utils/action-localization.ts:29-30`）。
    pub(crate) fn label(self) -> SharedString {
        match self {
            Self::Settings => tr("lithe.commandPalette.categories.Settings"),
            Self::View => tr("lithe.commandPalette.categories.View"),
        }
    }
}

/// 一条动作的**数据**（不含任何界面元素）。
///
/// 由 [`crate::workspace::command_action`] 这个无副作用的纯函数产出，这样
/// `ShellWorkspace::render` 与 [`open_command_palette`] 能用同一张表做两件事：
/// 前者画它、后者数它（诊断里的 `actions=`）。界面零件留在本模块（浮层的职责）。
pub(crate) struct CommandAction {
    pub(crate) id: CommandId,
    pub(crate) category: Category,
    pub(crate) label: SharedString,
    pub(crate) description: SharedString,
    pub(crate) icon: IconName,
}

impl CommandAction {
    /// 分类的本地化名。真源的动作表把 `category` 送进搜索面
    /// （`command-palette.tsx:339-343` 的 `matchesSearchQuery(query, [label, description, category])`），
    /// 也用它算分组标题；本侧现在的列表是**平铺**的（理由见模块文档的差异清单），
    /// 所以它只用在搜索关键词上，由 [`IntoCommandItem`] 取。
    pub(crate) fn category_label(&self) -> SharedString {
        self.category.label()
    }
}

// ---------------------------------------------------------------------------
// 打开 / 关闭
// ---------------------------------------------------------------------------

/// `Ctrl+Shift+P`（真源 `cmd+shift+p` 在 Windows 上的归一化形式，见模块文档）；
/// `Ctrl+M` 是**菜单栏自己的**「切换菜单栏」—— 真源走设置项 + 菜单项
/// （`menu.toggleMenuBar` = `alt+m`），本侧把形态切换做成一个应用级 action。
pub fn install_actions(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-shift-p", OpenCommandPalette, None),
        KeyBinding::new("ctrl-m", ToggleMenuBar, None),
    ]);
    cx.on_action(|_: &OpenCommandPalette, cx: &mut App| {
        let Some(window_handle) = cx.active_window() else {
            return;
        };
        let _ = window_handle.update(cx, |_, window, cx| open_command_palette(window, cx));
    });
    // 「切换菜单栏」直接落到菜单栏自己身上（它才是形态的所有者），不经过命令面板。
    cx.on_action(|_: &ToggleMenuBar, cx: &mut App| crate::menu_bar::toggle_menu_bar(cx));
}

gpui_kit::actions!(lithe_workbench, [OpenCommandPalette, ToggleMenuBar]);

/// 面板宽度 704：`ui/command.tsx:29` 的 `w-[min(44rem,calc(100vw-2rem))]`（44 × 16 = 704）。
const PANEL_WIDTH: f32 = 704.;
/// 面板最大高 512：同上 `max-h-[min(68vh,32rem)]`（32 × 16 = 512）。
const PANEL_MAX_HEIGHT: f32 = 512.;
/// 距窗口顶 16：`ui/command.tsx:143` 的 `pt-16`。
///
/// 16 正好在 gpui 的档位上，但 `Dialog::margin_top` 是**固有方法**、只收 `Pixels`，所以调用点
/// 写 [`crate::rem_px`]（`window.rem_size()` 作基准），不能直接给 `rems`。
const PANEL_TOP_INSET: f32 = 16.;

/// 打开命令面板。**只能从事件回调或任务里调用**（`render` 阶段会 panic，见模块文档）。
///
/// 已经开着别的对话框时不叠第二层 —— 与 `open_settings_dialog` 同一条口径
/// （`settings/src/dialog.rs:91-94`），也免得 `Esc` 的"只关最上层"语义被两个面板搞乱。
pub fn open_command_palette(window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        return;
    }
    let action_count = shell()
        .and_then(|shell| shell.upgrade())
        .map(|shell| shell.read(cx).command_actions(cx).len())
        .unwrap_or(0);
    println!("S1_COMMAND_PALETTE opened=true actions={action_count}");

    let palette = cx.new(|cx| CommandPalette::new(window, cx));
    // 打开后要把焦点交给搜索框（理由见 `PENDING_FOCUS`）：在渲染里做，因为只有那一刻
    // `Command` 的输入框才在元素树上。
    PENDING_FOCUS.with(|slot| *slot.borrow_mut() = true);
    window.open_dialog(cx, move |dialog, window, _cx| {
        // `Dialog::width` / `Dialog::margin_top` 是**固有方法**、只吃 `Pixels`（见模块头「度量」），
        // 所以按**当帧的** rem 基准换算；写死 16 就是假 rem（基准一变就不跟着缩放）。
        let rem = window.rem_size();
        dialog
            // 面板整个由 `Command` 自己画：搜索行、列表、圆角与边框都不要 Dialog 的。
            .close_button(false)
            .overlay(true)
            .overlay_closable(true)
            .keyboard(true)
            .width(crate::rem_px(rem, PANEL_WIDTH))
            .margin_top(crate::rem_px(rem, PANEL_TOP_INSET))
            .p_0()
            .content({
                let palette = palette.clone();
                move |content, _, _| content.p_0().gap_0().child(palette.clone())
            })
    });
}

// ---------------------------------------------------------------------------
// 面板
// ---------------------------------------------------------------------------

/// 命令面板的交互状态。
///
/// 与 `SettingsDialog` 同一口径：浮层的 builder 每帧重建，**状态必须存在 `Entity` 里**，
/// 否则每帧都会被重置（查询、高亮、滚动位置全丢）。这里只持有 `CommandState` ——
/// 动作表在 [`ShellWorkspace`] 上（它才是状态所有者），面板每帧现取。
pub struct CommandPalette {
    command: Entity<CommandState>,
}

impl CommandPalette {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            command: cx.new(|cx| CommandState::new(window, cx)),
        }
    }

    /// `Enter`（或鼠标点击）之后：执行第 `row` 条动作 → 关面板 → 需要时打开二级对话框。
    ///
    /// ⚠️ **先关面板再放行动作**，顺序不能换：动作里的「打开设置」会在同一个回调里开第二个
    /// 对话框，而 `open_settings_dialog` 有"已经开着对话框就不叠第二层"的保护
    /// （`settings/src/dialog.rs:91-94` 的 `has_active_dialog`），留着面板就会静默什么都
    /// 不发生。
    fn confirm(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        window.close_dialog(cx);
        let Some(shell) = shell().and_then(|shell| shell.upgrade()) else {
            // 工作台已经销毁（窗口正在关）：面板已经关了，什么都不用做。
            return;
        };
        shell.update(cx, |shell, cx| shell.run_command(row, window, cx));
    }
}

impl Render for CommandPalette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 打开后的第一帧把焦点交给搜索框（理由见 `PENDING_FOCUS`）。
        let needs_focus = PENDING_FOCUS.with(|slot| {
            let pending = *slot.borrow();
            *slot.borrow_mut() = false;
            pending
        });
        if needs_focus {
            self.command.update(cx, |command, cx| command.focus(window, cx));
            println!("S1_COMMAND_PALETTE focus=search");
        }

        let actions = shell()
            .and_then(|shell| shell.upgrade())
            .map(|shell| shell.read(cx).command_actions(cx))
            .unwrap_or_default();

        Command::new(&self.command)
            .bordered(false)
            .w_full()
            .placeholder(tr("lithe.commandPalette.placeholder"))
            // 真源列表跟着面板的 512 一起长（面板是 `max-h`，列表填剩余空间）；
            // `Command` 列表上限的默认值是 18.75rem = 300（`command/command.rs:34`），
            // 不放开的话 512 的面板只会用到一半。
            .max_h(DefiniteLength::from(rems(PANEL_MAX_HEIGHT / 16.)))
            .items(
                actions
                    .into_iter()
                    .map(IntoCommandItem::into_command_item),
            )
            .empty(|_, _, cx| {
                // 空态文案走真源（`command-palette.tsx:455` 的 `CommandEmpty`）。
                div()
                    .py_6()
                    .w_full()
                    .text_center()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.commandPalette.noCommands"))
            })
            .on_confirm({
                let palette = cx.entity();
                move |index, window, cx| {
                    // 未分组条目的 `IndexPath` 是 `section = 0` + `row = 动作表行号`
                    // （`command/state.rs:359-368` 的 `IndexPath::new(item_ix).section(0)`）。
                    palette.update(cx, |palette, cx| {
                        palette.confirm(index.row, window, cx);
                    });
                }
            })
    }
}

/// 让 [`CommandAction`] 能被面板直接消费的适配。
///
/// 放在这里而不是 `workspace.rs`：`CommandItem` / `Command` 是**浮层**的零件，
/// 工作台只出数据（文字、图标、朝向），不依赖 `component::command`。
trait IntoCommandItem {
    fn into_command_item(self) -> CommandItem;
}

impl IntoCommandItem for CommandAction {
    fn into_command_item(self) -> CommandItem {
        // 分类名进搜索关键词（真源把 `category` 也算进匹配面，`command-palette.tsx:339-343`）。
        let search_category = self.category_label();
        let CommandAction {
            id,
            label,
            description,
            icon,
            ..
        } = self;
        let search_key: SharedString = id.id().into();
        let item_label = label.clone();
        let item_description = description.clone();
        CommandItem::new()
            // `label` 同时是**搜索文本**：`Command` 的本地过滤只看 label 与 keywords
            // （`command/state.rs:307-313` 的 `item.matches(query)`），所以 label 必须是
            // 那条真实可见的标签；描述与机器 id 靠 keywords 一起进搜索面
            // （真源的 `matchesSearchQuery` 也把 label / description / category 一起匹配，
            // `command-palette.tsx:339-343`）。
            .label(label)
            .keywords([description, search_key, search_category])
            .icon(icon)
            .child(move |_, cx| {
                // 真源每行是「标签 + 次级色描述」的上下两层
                // （`ui/command.tsx:528-537` 的 `CommandItemTitle` + `CommandItemDescription`），
                // 所以用 `CommandItem::child` 自绘 —— 默认行只有一行标签 + 前置图标。
                //
                // ⚠️ `min_h_8()`（= 32 逻辑，真源 `density: default` 的 `min-h-8`，
                // `ui/command.tsx:41`）**不能省**：行高由自绘内容决定，不设下限时短标签
                // （主题那两条没有图标、标签也短）会塌成别的行的一半，列表看起来一高一低。
                //
                // ⚠️ 这个工厂是 `Fn`（可能因测量 / 进视口 / 排版失效而**跑多次**，
                // `command/item.rs:90-104`），所以只能从捕获的 `SharedString` 克隆，
                // 不能把它们 move 出去（`SharedString` 的克隆是原子引用计数，很便宜）。
                v_flex()
                    .w_full()
                    .min_w_0()
                    .min_h_8()
                    .justify_center()
                    .gap_0p5()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(item_label.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(item_description.clone()),
                    )
                    .into_any_element()
            })
    }
}
