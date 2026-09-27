//! 终端面板的**表现层**：页签条、状态行、输出区、空态 / 失败态、命令输入行。
//!
//! 从 `shell_probe/terminal.rs` 的「组件」一节起原样拆出（逐字搬迁，只调整可见性）。
//! 本文件只负责画；进程生命周期与 ANSI 清洗分别在 `session.rs` 与 `ansi.rs`。
//!
//! 对外公开的是 `TerminalPane`（`new` / `ensure_session` / `new_tab` / `replace_profiles` /
//! `add_profile`，以及 `profiles`）与事件 `TerminalPaneEvent`，字段全部私有。
//!
//! 规格出处（每个数值/文案都能查到来源）：
//!
//! - 页签条高 **36**、单页签 min-w **80** / max-w **200**、页签文字 **13px**、关闭按钮 **24×24**：
//!   `gpui/UI-MAP-WINDOWS.md` §1.6（引自 `windows/tauri/src/ui/tab-bar.tsx:244-267`、
//!   `features/terminal/components/terminal-tab-bar-item.tsx:102-107`）。
//! - 终端内容左内边距 **16**（`pl-4`，`features/terminal/components/terminal.tsx:870`）：同上 §1.6。
//! - 终端字号 **14** / 行高 **1**：`features/settings/config/default-settings.ts:76-77`
//!   （`terminalFontSize: DEFAULT_CODE_FONT_SIZE`，值为 14 见 `config/typography-defaults.ts:13`）。
//! - 输出保留上限 **10000 行**：真机默认 `terminalScrollback: 10000`
//!   （`features/settings/config/default-settings.ts:79`）。
//! - 状态文案「运行中 / 成功 / 失败」：`i18n/locale.ts:5930-5932`（`run.running/succeeded/failed`）；
//!   「退出码」`i18n/locale.ts:4568`（`git.console.exit`）；「终端错误 / 无法初始化终端 / 重试」：
//!   `i18n/locale.ts:7549-7551`；「没有终端 / 新建终端 / 关闭 {name} / 清除终端 / 选择终端配置文件」：
//!   `i18n/locale.ts:7544,7531,7519,7510,7543`；输入行占位「输入命令...」：
//!   `i18n/locale.ts:7905`（`commandPalette.placeholder`，「滚动到底部」`:4494`）。
//! - 页面/文案不含裸色值：颜色一律 `cx.theme()`（`gpui/UI-MAP.md` §1.1 第 2 条）。
//!
//! ## ⚠️ 渲染辅助函数一律收 `&self` + `&mut Context<Self>`，返回**具体类型**
//!
//! edition 2024 下 `-> impl IntoElement` 会把作用域里的生命周期捕进 opaque 类型，同一个表达式
//! 连续调用两次就报 E0499（仓库已踩过）。所以本文件的 `render_*` 返回 `AnyElement` / `Button`。

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyTitle};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::message_scroller::{MessageScroller, MessageScrollerState};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, Sizable as _, Size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AbsoluteLength, AnyElement, App, ClickEvent, Context, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Pixels, Render, Role, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled as _, Subscription, WeakEntity,
    Window, div, px, relative, rems,
};

use crate::constants::new_terminal_label;
use crate::constants::{
    CAPABILITY_NOTICE, CHROME_RADIUS, STATUS_LINE_HEIGHT, TAB_MAX_WIDTH, TERMINAL_LINE_HEIGHT,
    TERMINAL_PADDING_INLINE, choose_profile, clear_terminal, error_fallback, error_title,
    exit_code, failed, no_terminals, retry, running, scroll_to_end, succeeded, terminals_aria,
};
use crate::profile::TerminalProfile;
use crate::session::{Session, SessionState};

/// 一个页签：标题 + 会话 + 输出区状态。
pub(crate) struct TabSession {
    /// 稳定 id（页签数组下标会随关闭变动，泵任务按 id 找页签）。
    pub(crate) id: u64,
    /// 本次运行的**代次**：重试会 +1，旧泵的残留事件靠它丢弃。
    ///
    /// 真机用同一套路做过期保护：`run.rs` 用 `execution_id` 校验"旧适配器不得回收新 Run"
    /// （`windows/tauri/src-tauri/src/run.rs:2016-2045`，前端 `run.store.ts:1039-1067`）。
    pub(crate) run: u64,
    pub(crate) title: SharedString,
    /// 起这个页签时用的配置文件（重试时复用）。
    pub(crate) profile: TerminalProfile,
    pub(crate) session: Session,
    /// 输出区的贴底跟随状态（每个页签一份：`MessageScrollerState` 自带 `FollowMode::Tail`，
    /// `gpui-component-0.6.6/src/message_scroller.rs:38,65`）。
    pub(crate) scroller: Entity<MessageScrollerState>,
    /// 已完成行（输出区显示的内容）。`RefCell` 是为了让行渲染闭包按需读，
    /// 避免每个 chunk 都克隆整个 Vec。
    pub(crate) rows: Rc<RefCell<Vec<SharedString>>>,
}

/// 终端面板（底部工具窗的"终端"页）。
///
/// 对外只暴露 [`TerminalPane::new`]（主代理按这个签名接线）与几个公开方法；字段全私有。
pub struct TerminalPane {
    pub(crate) tabs: Vec<TabSession>,
    /// 当前活动页签的下标（空表时无意义）。
    pub(crate) active: usize,
    pub(crate) next_id: u64,
    /// 下一个运行代次（见 [`TabSession::run`]）。
    pub(crate) next_run: u64,
    /// 可选配置文件（[`profiles`] 或 [`TerminalPane::replace_profiles`] 灌进来的）。
    pub(crate) profiles: Vec<TerminalProfile>,
    /// 新建页签用哪个配置文件。
    pub(crate) active_profile: usize,
    /// 上一次登记进来的「默认 Shell」设置值（Windows 键 `terminalDefaultShellId`）。
    ///
    /// 存它是为了**幂等**：外壳在*每一次*设置变化时都会转发一遍（`ShellWorkspace` 的
    /// 设置订阅），而用户在页签条 ⌄ 菜单里手动选过的配置文件不该被"隔壁开关动了"重置回默认。
    /// 所以只有这个值**真的变了**才重建 `profiles` 并改 `active_profile`。
    pub(crate) default_shell_id: String,
    /// 终端正文字号的覆盖值（`None` = 用默认档）。
    ///
    /// 默认档是 typography 的 `sm` token（`.text_sm()` = 14px，见 `constants.rs` 的说明）。
    /// **不能**改主题的 `mono_font_size` 来实现它：那个 token 是编辑器正文在用的，
    /// 改它会连带把编辑器一起改掉。所以这一个键的落点只能是本视图自己。
    pub(crate) font_size: Option<f32>,
    /// 底部命令输入行。
    pub(crate) input: Entity<InputState>,
    /// 输入行的事件订阅（`PressEnter` → 发送一行）。订阅器一 drop 就失效，所以要存住。
    pub(crate) _input_events: Subscription,
}

impl TerminalPane {
    //
    // ⚠️ 只读的渲染辅助函数一律返回**具体类型**（`AnyElement` / `Button`），并且只借
    // `&self` + `&mut Context<Self>`：edition 2024 下 `-> impl IntoElement` 会把作用域里的
    // 生命周期捕进 opaque 类型，同一个表达式连续调用两次就报 E0499（仓库已踩过）。

    /// 页签条（高 36）。
    ///
    /// 用 gpui-kit 的 `TabBar` + `Tab`（`gpui/UI-MAP-WINDOWS.md` §2⑤「终端页签行」判定为可一比一）：
    /// `TabVariant::Underline` = 活动页签下方一条主色指示条（真机是同语义的 3px 强调条，
    /// `ui/tab-bar.tsx:206-210`）。关闭按钮真机的 `Tab` 没有内建（`UI-MAP-WINDOWS.md` §2⑤），
    /// 所以按建议用 `Tab::suffix(Button)` 自己塞一个。
    ///
    /// 没有页签时退化成真机的"空态条"：终端图标 + 「没有终端」+ `+`
    /// （`terminal-tab-bar.tsx:708-755`）。
    pub(crate) fn render_tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let shell: WeakEntity<Self> = cx.entity().downgrade();

        if self.tabs.is_empty() {
            return h_flex()
                .w_full()
                .flex_shrink_0()
                .h_9()
                .gap_1p5()
                .px_2()
                .bg(cx.theme().tab_bar)
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    Icon::new(IconName::SquareTerminal)
                        .size_4()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(no_terminals()),
                )
                .child(self.new_tab_button(shell, cx))
                .into_any_element();
        }

        // `Button::rounded` 只吃 `ButtonRounded`，而 `ButtonRounded` 只实现了 `From<Pixels>`
        // （`gpui-component-0.6.6/src/button/button.rs:29-33`，没有 `From<Rems>`），所以圆角
        // 必须**先按当前 rem 基准求值成像素**再交出去（见 [`rem_px`]）。这里的 `rem` 就是
        // 主题字号：`Root::render` 每帧把 `cx.theme().font_size` 写进 `window.set_rem_size`
        // （`gpui-component-0.6.6/src/root.rs:582`），所以它就是本帧的 rem 基准。先取出来
        // 是因为下面的 `map` 闭包若再借一次 `cx`，会和后面的 `self.profile_menu_button(cx)` 打架。
        let rem = cx.theme().font_size;

        let tabs: Vec<Tab> = self
            .tabs
            .iter()
            .map(|tab| {
                let id = tab.id;
                let name = tab.title.clone();
                let close_shell = shell.clone();
                // 「关闭 {name}」：`terminal.tabClose`，`locale.ts:7519`。
                let close_label = lithe_gpui_shared::tr_args(
                    "lithe.terminal.tabClose",
                    &[("name", name.as_ref())],
                );
                let close = Button::new(SharedString::from(format!("terminal-tab-close-{id}")))
                    .ghost()
                    .icon(IconName::Close)
                    .tab_stop(false)
                    .size_6()
                    .rounded(rem_px(rem, CHROME_RADIUS))
                    .tooltip(close_label.clone())
                    .accessibility_label(close_label)
                    .on_click(
                        move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                            // 关闭按钮嵌在 `Tab::suffix` 里，`Button` 的点击默认会继续冒泡到
                            // 页签的 `on_click`（那会去切页签），所以这里显式掐断
                            // （`App::stop_propagation`，`gpui-pre-0.3.6/src/app.rs:2383`）。
                            cx.stop_propagation();
                            let _ = close_shell.update(cx, |this, cx| this.close_tab(id, cx));
                        },
                    );

                // 用 `child` 而不是 `label`：`Tab` 会在自己的根上写 `text_sm()`
                // （14px，`gpui-component-0.6.6/src/tab/tab.rs:803-807`），而用户样式在它之前
                // 写入、会被覆盖；把文字放进自己的 div 才能拿到规格值
                // （`ui/tab-bar.tsx:170` 的 `ui-text-chrome` = 13px → `text_sm()`，见 `constants.rs`）。
                Tab::new()
                    .aria_label(name.clone())
                    .min_w_20()
                    .child(div().flex_1().min_w_0().truncate().text_sm().child(name))
                    .suffix(close)
            })
            .collect();

        let switch_shell = shell.clone();
        let bar = TabBar::new("terminal-tab-bar")
            .underline()
            // 真机单页签高 28（`--lithe-tab-height: 1.75rem`，`styles/theme.css:122`；
            // 用法见 `ui/tab-bar.tsx:257-267`）；gpui 的 `Tab` 高度由 `TabVariant::height(size)`
            // 决定，且 `Tab::render` 会在用户样式之后重写 `.h(...)`
            // （`gpui-component-0.6.6/src/tab/tab.rs:24-43,801`），改不动。
            // 表里能选的是 Small+Underline=**30** 与 Medium+Underline=36，取更接近的 30。
            .with_size(Size::Small)
            .selected_index(self.active)
            .max_width(rem_px(rem, TAB_MAX_WIDTH))
            .children(tabs)
            .suffix(
                h_flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_0p5()
                    .child(self.clear_button(shell.clone(), cx))
                    .child(self.new_tab_button(shell.clone(), cx))
                    .child(self.profile_menu_button(cx)),
            )
            .on_click(move |index: &usize, window: &mut Window, cx: &mut App| {
                let index = *index;
                let _ = switch_shell.update(cx, |this, cx| {
                    this.active = index;
                    // 切页签后把焦点还给输入行（真机切页签也会重新聚焦 xterm，
                    // `terminal.tsx:662-687` 的"验证式重试"就是在做这件事）。
                    let input = this.input.clone();
                    input.update(cx, |state, cx| state.focus(window, cx));
                    cx.notify();
                });
            });

        v_flex()
            .w_full()
            .flex_shrink_0()
            .h_9()
            .bg(cx.theme().tab_bar)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(bar)
            .into_any_element()
    }

    /// 状态 + 能力边界一行（高 24）。
    ///
    /// 左边是会话状态：运行中 / 成功·退出码 N / 失败·退出码 N / 终端错误（起不来），
    /// 起不来或已退出时右侧多一个「重试」。右边常驻 [`CAPABILITY_NOTICE`]。
    pub(crate) fn render_status_line(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(tab) = self.tabs.get(self.active) else {
            return div().into_any_element();
        };

        let (label, color) = match &tab.session.state {
            SessionState::Running => {
                // 读线程报过 IO 错误就显示出来（正常管道不会走到这里）。
                let label = match &tab.session.io_error {
                    Some(error) => SharedString::from(format!("{} · {error}", running())),
                    None => running(),
                };
                (label, cx.theme().success)
            }
            SessionState::Exited(0) => (succeeded(), cx.theme().success),
            SessionState::Exited(code) => (
                SharedString::from(format!("{} · {} {code}", failed(), exit_code())),
                cx.theme().danger,
            ),
            SessionState::Failed(message) => (
                SharedString::from(format!("{}：{message}", error_title())),
                cx.theme().danger,
            ),
        };

        let id = tab.id;
        let retryable = !tab.session.is_running();

        let mut line = h_flex()
            .w_full()
            .flex_shrink_0()
            .h_6()
            .gap_1p5()
            .px_3()
            .bg(cx.theme().muted)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .max_w_80()
                    .truncate()
                    .text_xs()
                    .text_color(color)
                    .child(label),
            );

        if retryable {
            line = line.child(
                Button::new(SharedString::from(format!("terminal-retry-{id}")))
                    .ghost()
                    .icon(IconName::Play)
                    .label(retry())
                    .tab_stop(false)
                    // 24 − 4 = 20（让出状态行的上下内边距）：运行时算术，没有档位 helper 可套，
                    // 所以写 helper 底层的 `rems(P / 16.)`（rem base = 16px，见 `constants.rs`）。
                    .h(rems((STATUS_LINE_HEIGHT - 4.) / 16.))
                    .rounded(rem_px(cx.theme().font_size, CHROME_RADIUS))
                    .on_click(cx.listener(
                        move |this: &mut Self,
                              _event: &ClickEvent,
                              _window: &mut Window,
                              cx: &mut Context<Self>| {
                            this.retry_tab(id, cx);
                        },
                    )),
            );
        }

        line.child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(SharedString::from(CAPABILITY_NOTICE)),
        )
        .into_any_element()
    }

    /// 输出区：贴底跟随的虚拟列表 + 底部"未完成行"。
    ///
    /// `MessageScroller`（`gpui-component-0.6.6/src/message_scroller.rs:185,284`）自带
    /// `FollowMode::Tail`、滚动条与"跳回最新"按钮（`jump_button`），正是
    /// `gpui/UI-MAP.md` §1.3 表格里推荐的"贴底跟随的输出区"。
    ///
    /// ⚠️ 它的行容器默认是**聊天记录**的排版（每行间 `pb_8()` = 32px、左右 `px_3()`，
    /// `message_scroller.rs:349-355`），终端要贴行显示，所以用 `with_row_style` 覆写内边距
    /// （`refine_style` 在那些内建 padding **之后**执行，能盖掉它们）。
    pub(crate) fn render_output(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(tab) = self.tabs.get(self.active) else {
            return self.render_empty(cx);
        };
        if let SessionState::Failed(message) = &tab.session.state {
            return self.render_failed(message.clone(), cx);
        }

        let rows = tab.rows.clone();
        let mono = cx.theme().mono_font_family.clone();
        let foreground = cx.theme().foreground;
        // 行渲染闭包是 `move` 的，会把字体名与字号搬走；下面"未完成行"还要用，所以先各留一份。
        let row_mono = mono.clone();
        let row_font_size = self.font_size;

        let scroller = MessageScroller::new(
            SharedString::from(format!("terminal-output-{}", tab.id)),
            tab.scroller.clone(),
            move |index, _window, cx| {
                let text = rows.borrow().get(index).cloned().unwrap_or_default();
                div()
                    .w_full()
                    .min_w_0()
                    .font_family(row_mono.clone())
                    .text_sm()
                    // 设置里的终端字号覆盖默认档（`.text_sm()` 的 14px）；不覆盖时保持原样。
                    .when_some(row_font_size, |this, size| this.text_size(px(size)))
                    .line_height(relative(TERMINAL_LINE_HEIGHT))
                    .text_color(cx.theme().foreground)
                    .child(text)
            },
        )
        .scrollbar(true)
        .jump_button(true)
        .with_jump_button_label(scroll_to_end())
        .with_row_style(output_row_style());

        // 未完成行贴在列表**下面**：进度条（`\r` 反复覆盖同一行）与提示符都停在底部，
        // 与真机的观感一致，也省掉了"每来一块就替换列表最后一行"的整表重排。
        let partial = tab.session.text.current_text();

        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .bg(cx.theme().background)
            .child(
                // 输出区的无障碍标签：`terminal.terminals`「终端」（`locale.ts:7558`）。
                // `MessageScroller` 自己挂 `Role::Log` 但不设标签
                // （`gpui-component-0.6.6/src/message_scroller.rs:364-368`），所以在包裹层补一个。
                // ⚠️ `.role()` / `.aria_label()` 在 `StatefulInteractiveElement` 上，而它只对
                // `Stateful<E>` 实现（`gpui-pre-0.3.6/src/elements/div.rs:1300-1326,4074`），
                // 所以必须先 `.id(...)`。
                div()
                    .id("terminal-output-viewport")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .role(Role::Log)
                    .aria_label(terminals_aria())
                    // `MessageScroller` 的根是 `size_full()`，所以外面这层必须给出确定高度。
                    .child(div().size_full().child(scroller)),
            )
            .when_some(partial, |this, line| {
                this.child(
                    h_flex()
                        .w_full()
                        .flex_shrink_0()
                        .pl_4()
                        .pr_4()
                        .font_family(mono.clone())
                        .text_sm()
                        .when_some(self.font_size, |this, size| this.text_size(px(size)))
                        .line_height(relative(TERMINAL_LINE_HEIGHT))
                        .text_color(foreground)
                        .child(line),
                )
            })
            .into_any_element()
    }

    /// 空态（没有终端页签）：真机是「没有终端」+ `+`（`terminal-tab-bar.tsx:708-755`）。
    pub(crate) fn render_empty(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .size_full()
            .child(
                Empty::new()
                    // `Empty` 硬编码了 `.border_dashed()`（`gpui-component-0.6.6/src/empty.rs:74-75`），
                    // 真机界面没有这圈虚线；改边框色为透明关掉（与编辑器空态同一招，2026-09-27 统一）。
                    .border_color(cx.theme().transparent)
                    .header(
                        EmptyHeader::new()
                            .media(
                                gpui_kit::component::empty::EmptyMedia::new()
                                    .child(Icon::new(IconName::SquareTerminal).size_8()),
                            )
                            .title(EmptyTitle::new().text_sm().child(no_terminals())),
                    )
                    .content(
                        EmptyContent::new().child(
                            Button::new("terminal-empty-new")
                                .icon(IconName::Plus)
                                .label(new_terminal_label())
                                .h_6()
                                .px_2()
                                .rounded(rem_px(cx.theme().font_size, CHROME_RADIUS))
                                .on_click(cx.listener(
                                    |this: &mut Self,
                                     _event: &ClickEvent,
                                     window: &mut Window,
                                     cx: &mut Context<Self>| {
                                        this.new_tab(window, cx);
                                    },
                                )),
                        ),
                    ),
            )
            .into_any_element()
    }

    /// 失败态（子进程起不来）：终端错误 + 系统错误原文 + 「重试」。
    pub(crate) fn render_failed(&self, message: String, cx: &mut Context<Self>) -> AnyElement {
        let id = self
            .tabs
            .get(self.active)
            .map(|tab| tab.id)
            .unwrap_or_default();
        v_flex().size_full().child(
            Empty::new()
                // `Empty` 硬编码了 `.border_dashed()`（`gpui-component-0.6.6/src/empty.rs:74-75`），
                // 真机界面没有这圈虚线；改边框色为透明关掉（与编辑器空态同一招，2026-09-27 统一）。
                .border_color(cx.theme().transparent)
                .header(
                    EmptyHeader::new()
                        .media(
                            gpui_kit::component::empty::EmptyMedia::new()
                                .child(Icon::new(IconName::SquareTerminal).size_8()),
                        )
                        .title(EmptyTitle::new().text_sm().child(error_title()))
                        .description(
                            EmptyDescription::new()
                                .text_xs()
                                .child(SharedString::from(message)),
                        ),
                )
                .content(EmptyContent::new().child(
                    Button::new("terminal-failed-retry")
                        .icon(IconName::Play)
                        .label(retry())
                        .h_6()
                        .px_2()
                        .rounded(rem_px(cx.theme().font_size, CHROME_RADIUS))
                        .on_click(cx.listener(
                            move |this: &mut Self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                this.retry_tab(id, cx);
                            },
                        )),
                )),
        )
        .into_any_element()
    }

    /// 底部命令输入行（真机没有这一行，是本外壳的替代：见文件头「与 Windows 的差异」）。
    pub(crate) fn render_input_row(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tab = self.tabs.get(self.active)?;
        if !tab.session.is_running() {
            return None;
        }

        // ⚠️ 必须走 `Styled::h` 的全限定写法：`Input` 有**同名固有方法**
        // `Input::h(impl Into<DefiniteLength>)`，它只写 `self.height`，而那个字段只在多行输入里
        // 生效（`gpui-component-0.6.6/src/input/input.rs:256-260`）；单行输入的实际高度来自
        // `input_h(size)` = 32px。全限定调用写的是样式表，而 `refine_style` 在 `input_h` 之后
        // 执行（`input.rs:703,719`），能覆写成真机 stdin 输入框的 28px。
        let input = gpui_kit::Styled::h_7(Input::new(&self.input))
            .w_full()
            .min_w_0()
            // `Input` 没有固有 `rounded`，这里走的是 `Styled::rounded`（吃 `AbsoluteLength`），
            // 所以能直接把规格值写成 rem：布局期按窗口 rem 基准求值，`rems(CHROME_RADIUS / 16.)`
            // 在 16px 基准下与原来的 `px(CHROME_RADIUS)` 逐像素相等。上面那 7 个 `Button` 不行
            // —— `Button::rounded` 只吃 `ButtonRounded`（只实现了 `From<Pixels>`），走 [`rem_px`]。
            .rounded(rems(CHROME_RADIUS / 16.))
            .font_family(cx.theme().mono_font_family.clone())
            .text_xs();

        Some(
            h_flex()
                .w_full()
                .flex_shrink_0()
                .gap_1p5()
                .px_3()
                .py_1p5()
                .bg(cx.theme().background)
                .border_t_1()
                .border_color(cx.theme().border)
                .child(input)
                .into_any_element(),
        )
    }

    /// `+` 新建终端（真机 `terminal-tab-bar.tsx:724-735`，图标 `Plus`、提示 `terminal.newTerminal`）。
    ///
    /// 收 `cx` 只为取 rem 基准（`cx.theme().font_size`）：`Button::rounded` 只吃 `Pixels`，
    /// 见 [`rem_px`]。
    pub(crate) fn new_tab_button(&self, shell: WeakEntity<Self>, cx: &App) -> Button {
        Button::new("terminal-new-tab")
            .ghost()
            .icon(IconName::Plus)
            .tab_stop(false)
            .size_6()
            .rounded(rem_px(cx.theme().font_size, CHROME_RADIUS))
            .tooltip(new_terminal_label())
            .accessibility_label(new_terminal_label())
            .on_click(
                move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
                    let _ = shell.update(cx, |this, cx| this.new_tab(window, cx));
                },
            )
    }

    /// 清除输出（真机 `terminal.contextClear`，`terminal-tab-bar.tsx:959-964`）。
    ///
    /// ⚠️ 图标用 `Trash` 而不是真机的 `Trash2`：本仓库注册的 `gpui_kit::assets::AllAssets`
    /// 里**没有 `trash-2.svg`**（全量目录只有 `trash.svg` / `trash-off.svg`），
    /// `IconName` 是按 svg 文件名生成的（`gpui-kit-assets-0.6.6/build.rs:20-51`），
    /// 所以 `IconName::Trash2` 这个变体根本不存在。
    pub(crate) fn clear_button(&self, shell: WeakEntity<Self>, cx: &App) -> Button {
        Button::new("terminal-clear-output")
            .ghost()
            .icon(IconName::Trash)
            .tab_stop(false)
            .size_6()
            .rounded(rem_px(cx.theme().font_size, CHROME_RADIUS))
            .tooltip(clear_terminal())
            .accessibility_label(clear_terminal())
            .disabled(self.tabs.is_empty())
            .on_click(
                move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                    let _ = shell.update(cx, |this, cx| this.clear_active_output(cx));
                },
            )
    }

    /// 配置文件下拉（真机 `terminal-tab-bar.tsx:442-451,1031-1051`）。
    ///
    /// 这就是 [`TerminalProfile`] 文档里说的"设置界面接线点"在界面上的落点：菜单里列出
    /// [`TerminalPane::profiles`] 的内容，选中即改"新建页签用哪个 shell"。
    pub(crate) fn profile_menu_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let shell: WeakEntity<Self> = cx.entity().downgrade();
        let profiles: Vec<SharedString> = self
            .profiles
            .iter()
            .map(|profile| profile.name().clone())
            .collect();
        let active = self.active_profile;

        Button::new("terminal-profile-menu")
            .ghost()
            .icon(IconName::ChevronDown)
            .tab_stop(false)
            .size_6()
            .rounded(rem_px(cx.theme().font_size, CHROME_RADIUS))
            .tooltip(choose_profile())
            .accessibility_label(choose_profile())
            // `DropdownMenu`（`gpui-component-0.6.6/src/menu/dropdown_menu.rs:14-19`）；
            // 菜单项用 `PopupMenuItem::new(label).checked(..).on_click(..)`
            // （`menu/popup_menu.rs:71,180,196`）。
            .dropdown_menu(move |mut menu, _window, _cx| {
                menu = menu.scrollable(true);
                if profiles.is_empty() {
                    menu = menu.item(PopupMenuItem::new(error_fallback()).disabled(true));
                    return menu;
                }
                for (index, name) in profiles.iter().enumerate() {
                    let shell = shell.clone();
                    menu = menu.item(
                        PopupMenuItem::new(name.clone())
                            .checked(index == active)
                            .on_click(move |_event, _window, cx| {
                                let _ = shell.update(cx, |this, cx| {
                                    this.active_profile = index;
                                    cx.notify();
                                });
                            }),
                    );
                }
                menu
            })
            .into_any_element()
    }
}

impl Render for TerminalPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 每个 `render_*` 都返回具体类型（`AnyElement` / `Option<AnyElement>`），
        // 所以这里可以顺序调用它们，再在最后一条表达式里 `cx.theme()`。
        let tab_bar = self.render_tab_bar(cx);
        let status = self.render_status_line(cx);
        let output = self.render_output(cx);
        let input_row = self.render_input_row(cx);

        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(tab_bar)
            .child(status)
            .child(output)
            .children(input_row)
    }
}

impl Drop for TerminalPane {
    /// 面板销毁时把**所有**子进程 kill + wait，绝不留下僵尸进程（真机窗口销毁也杀全部 PTY，
    /// `windows/tauri/crates/terminal/src/manager.rs:99-113`）。
    fn drop(&mut self) {
        for tab in &mut self.tabs {
            tab.session.shutdown();
        }
        if !self.tabs.is_empty() {
            println!("S1_TERMINAL_DROP tabs={}", self.tabs.len());
        }
    }
}

/// 把一个**规格像素值**按**当前 rem 基准**求值：`rems(P / 16.)` 再 `to_pixels(rem)`。
///
/// 与 `settings/src/dialog.rs:189-199` 的 `rem_px` 同形 —— 那个形态是仓库里**唯一**正确的换算：
/// `/ 4.` 是错的（gpui 的档位 helper 后缀 `N` = `N × 0.25rem`，而这里的 `P` 是**像素**，
/// 1rem = 16px，见 `constants.rs` 的「度量」一节），写成 `to_pixels(px(16.))` 也是错的
/// （那是**假 rem**：写死 16 基准、不随字号缩放）。
///
/// **为什么圆角要绕这一圈**：`Button::rounded` 收 `impl Into<ButtonRounded>`，而 `ButtonRounded`
/// 只实现了 `From<Pixels>`（`gpui-component-0.6.6/src/button/button.rs:29-33`），**没有**
/// `From<Rems>` —— 实测在 `Button` 上写 `.rounded(rems(..))` 报
/// `E0277: the trait bound ButtonRounded: From<Rems> is not satisfied`（`Button::rounded` 是
/// 固有方法，它优先于 `Styled::rounded`，所以吃不到 `AbsoluteLength`）。`TabBar::max_width`
/// 同理（`impl Into<Pixels>`，`tab_bar.rs:118`）。能吃 `AbsoluteLength` / `Length` 的调用点
/// （`Input` 的 `Styled::rounded`、状态行按钮的 `Styled::h`）直接写 `rems(P / 16.)`，
/// 由布局期按窗口 rem 基准求值，不必经过这里。
///
/// `rem` 从 `cx.theme().font_size` 取：`Root::render` 每帧把它写进 `window.set_rem_size`
/// （`gpui-component-0.6.6/src/root.rs:582`），所以它就是本帧的 rem 基准。
fn rem_px(rem: Pixels, spec_px: f32) -> Pixels {
    AbsoluteLength::from(rems(spec_px / 16.)).to_pixels(rem)
}

///
/// 左右 16 = 真机终端内容左内边距（`pl-4`，`terminal.tsx:870`）；上下 0 = 终端行必须贴行显示。
fn output_row_style() -> StyleRefinement {
    let mut style = StyleRefinement::default();
    // `StyleRefinement` 是裸样式表，没有 `Styled` 的档位 helper 可用，所以直接写单位构造函数：
    // 16 / 16 = `rems(1.)` = 1rem（与 `pl_4()` 同值）、0 → `rems(0.)`。
    let inline = rems(TERMINAL_PADDING_INLINE / 16.);
    let zero = rems(0.);
    style.padding.left = Some(inline.into());
    style.padding.right = Some(inline.into());
    style.padding.top = Some(zero.into());
    style.padding.bottom = Some(zero.into());
    style
}

// ---------------------------------------------------------------------------
// 图标替代表
//
// | 用途 | Windows 真机的字形 | 本项目用的字形 | 说明 |
// | --- | --- | --- | --- |
// | 终端（空态条 / 空态页） | `TerminalIcon` | `SquareTerminal`（`square-terminal.svg`） | 同语义，全量目录里确有 |
// | 新建终端 | `Plus` | `Plus`（`plus.svg`） | 同名 1:1 |
// | 关闭页签 | `X` | `Close`（`close.svg`） | 默认集没有 `x`，`close` 是同一个叉 |
// | 重新拉起 | —（真机是错误兜底的「重试」文字按钮） | `Play`（`play.svg`） | 表达"再跑一次" |
// | 清除输出 | `Trash2` | `Trash`（`trash.svg`） | **全量 Lucide 目录里没有 `trash-2.svg`**，`IconName::Trash2` 不存在 |
// | 配置文件下拉 | `ChevronDown` | `ChevronDown`（`chevron-down.svg`） | 同名 1:1 |
// | 跳回最新（输出区） | —（真机是 xterm 自己的滚动条） | `MessageScroller` 内建的 `ArrowDown` | 组件自带 |
//
// 未实现清单（本轮**不做**的，逐条写明卡在哪）：
//
// 1. **能力边界文案不是逐字取自 locale**：[`CAPABILITY_NOTICE`] 是本模块唯一自写的用户可见文案。
//    卡点：`locale.ts` 的 `terminal.*`（`:7503-7558`）里没有任何描述"没有 VT / Ctrl+C 不可用"的键，
//    而维护者的范围决定要求"界面上写清"。取舍：写一句中文并在这里登记，而不是假装它来自 locale。
//    建议：等 Windows 前端补了这类键（例如 `terminal.capabilityNotice`）再换成同一个键。
// 2. **VT 模拟**（光标定位、备用屏幕、滚动区域、真彩 SGR 着色、选区、查找、链接、IME、鼠标）：
//    全部不做 —— 这是维护者 2026-09-25 明确划出的范围（`gpui/UI-MAP-WINDOWS.md` §4 缺口清单第 1 条
//    也说 gpui-kit 三 crate 里 `portable_pty|conpty|alacritty|termwiz|vte|vt100` 零命中）。
//    本模块只做"丢弃控制序列 + `\r`/`\b` 落实"这一层，够"输出查看器"用（文件头列了参考实现）。
// 3. **ANSI 颜色**：SGR 被整段丢弃，所以 `ls --color` 之类的彩色输出全是单色。要上色得先有
//    "按 run 解析 SGR → 分段 span"的实现（真机是 `run-output-style.ts:92-199` 那套），
//    以及 16 色板（gpui-kit 没有终端色 token，`UI-MAP-WINDOWS.md` §3.1 第 24-39 行说要自建 `[Hsla; 16]`）。
// 4. **stderr 不单独着色**：真机的**运行**输出也是 stdout/stderr 混成一条流
//    （`run.rs:1932-1965` 只发 `run-output {sessionId, chunk}`），只有调试器控制台分色
//    （`debugger-view.tsx:671-672` 的 `text-destructive`）。本模块跟运行输出一致。
// 5. **GBK/CP936 输出**：`cmd.exe` 在 936 代码页下的中文会变成 `�`（[`TerminalText::push_bytes`]
//    有详述）。要修需要读系统 ANSI 代码页并做编码转换（真机 host 就是按 ANSI 代码页解码的，
//    `run.rs:1864-1903`），本仓库的 `gpui/shell` 没有这个依赖（`Cargo.toml` 只有 gpui-kit /
//    lithe-core / serde_json，且本轮**不允许**改 Cargo.toml）。
// 6. **无 PTY**：子进程没有控制台，因此 (a) 全屏程序不可用，(b) `Ctrl+C` 送不到，
//    (c) 行缓冲/回显/宽度协商都由子进程自己决定。要真做终端必须引 `portable-pty`
//    （真机就是它，`windows/tauri/crates/terminal/Cargo.toml:11`）—— 属于下一轮的决策，不在本轮范围。
// 7. **页签拖拽重排 / 固定 / 重命名 / 右键菜单（关闭其他、关闭全部、导出输出）**、
//    **分屏**（真机 `cmd+d` / `cmd+shift+d`）、**垂直页签布局**、**全屏**：`TabBar` 不支持拖拽，
//    垂直布局它也没有（`UI-MAP-WINDOWS.md` §2⑤），分屏要自建布局。本轮只做"新建 / 关闭 / 标题"。
// 8. **终端宽度模式**（`terminalWidthMode: "full" | "editor"`，默认 `"editor"`，
//    `features/terminal/stores/terminal.store.ts:29`）与**页签布局/位置**：属于底部工具窗容器的
//    职责（真机在 `main-layout.tsx:318-322`），不在这个面板模块里。
// 9. **搜索（`terminal.find`）、复制/粘贴、字号缩放、OSC 标题/目录跟随、导出输出**：未做。
//    真机的这些能力分别依赖 xterm 的 selection / addon 与 OSC 流解析
//    （`terminal-osc-stream.ts:93-104`），没有 VT 就没有落点。
// 10. **焦点**：[`TerminalPane::ensure_session`] 在首次显示时把焦点交给输入行，且必须延到帧末
//    （原因见该方法：输入行要等会话起来的那一帧渲染完才上树）。此外没有 ⌘/Ctrl 级快捷键
//    （`terminal.new` = `cmd+t`、`terminal.close` = `cmd+w`，`default-keymaps.ts:122-139`）
//    —— 快捷键要挂在有焦点的根元素上，属于外壳的接线，不在本模块。
// 11. **会话是懒创建的**：构造期不 spawn shell（`terminal_view.rs` 的模块文档与
//    [`TerminalPane::new`] 都写了理由）。所以"页面一打开就有终端在跑"这件事**不该**再出现；
//    若在启动日志里看见 `S1_TERMINAL_TAB` 而没有点过终端按钮，那就是接线退化了。
//
// 实测记录（本文件里"不做本地回显""按 CRLF 断行"两条结论的依据，2026-09-25，Windows 10 19045）：
//
// - `"echo hello-from-pipe`r`ndir /b`r`nexit`r`n" | & cmd.exe /Q`（PowerShell 造管道）：
//   输出是 `Microsoft Windows [版本 …]` 横幅 + `D:\…>hello-from-pipe`（**提示符 + 命令回显**）
//   + 目录列表 + 下一条提示符；命令逐行执行、输出实时流出，`exit` 后进程自行退出。
// - 用 `Start-Job` 起
//   `cmd /c "(echo Write-Output MARKER-1 & ping -n 6 127.0.0.1 >nul & echo Write-Output MARKER-2) | powershell -NoLogo -NoProfile"`
//   并把 stdout 重定向到文件：**t=2s**（第一条命令之后、生产者还没写下第二条时）文件里已经有
//   `PS D:\…> Write-Output MARKER-1` 与 `MARKER-1`；t=8s 才看到 `MARKER-2`。
//   → PowerShell 在管道下是**逐行**读 stdin 并执行（不是"读完整个 stdin 当脚本跑"），
//   而且同样打印 `PS D:\…>` 提示符与命令回显。顺带测了 `powershell -Command -`：也逐行执行，
//   但**不**打印提示符。
// - 因此：本模块不做本地回显（会重复两遍），并且发送时用 `\r\n` 结尾。
// - ⚠️ 上面两条测的是**带 `-NoLogo -NoProfile` 的 powershell**；[`profiles`] 默认**不带参数**
//   （对齐真机的"系统默认"profile，`terminal-profiles.ts:19-23`），差别只是多一条启动横幅。

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::px;

    /// rem 换算：`rems(P / 16.)` 在 16px 基准下必须等于规格像素值。
    ///
    /// 与 `settings/src/row.rs:160-170`、`settings/src/dialog.rs:3298-3306` 同型：`P` 是
    /// **规格像素值**（真源里的 px），1rem = 16px（默认 `uiFontSize` 13 →
    /// `theme_font_size_for(13) = 16.0`，`Root::render` 每帧把它写进 `window.set_rem_size`），
    /// 所以 16px 基准下 `rems(P / 16.)` 与 `px(P)` 逐像素相等，外观不变；基准变了才等比缩放。
    /// 把 `/ 16.` 写成 `/ 4.` 或别的基准，下面两组断言就会红。
    #[test]
    fn rem_conversions_match_the_spec_pixels() {
        // ① 走 [`rem_px`] 的调用点（`Button::rounded`、`TabBar::max_width` 只吃 `Pixels`）。
        assert_eq!(rem_px(px(16.), CHROME_RADIUS), px(CHROME_RADIUS));
        assert_eq!(rem_px(px(16.), TAB_MAX_WIDTH), px(TAB_MAX_WIDTH));
        assert_eq!(
            rem_px(px(16.), STATUS_LINE_HEIGHT - 4.),
            px(STATUS_LINE_HEIGHT - 4.)
        );

        // ② 直接写 `rems(P / 16.)` 的调用点（吃 `AbsoluteLength` / `Length` 的样式：
        //    输入框圆角、状态行按钮高度）。
        let spec_to_px =
            |spec_px: f32| AbsoluteLength::from(rems(spec_px / 16.)).to_pixels(px(16.));
        assert_eq!(spec_to_px(CHROME_RADIUS), px(CHROME_RADIUS));
        assert_eq!(
            spec_to_px(STATUS_LINE_HEIGHT - 4.),
            px(STATUS_LINE_HEIGHT - 4.)
        );

        // ③ 基准字号翻倍（`uiFontSize` 26 → rem 基准 32px）时长度等比例放大：
        //    这就是用 rem 而不是写死 `px(...)` 的意义。
        assert_eq!(rem_px(px(32.), CHROME_RADIUS), px(CHROME_RADIUS * 2.));
        assert_eq!(
            AbsoluteLength::from(rems(TAB_MAX_WIDTH / 16.)).to_pixels(px(32.)),
            px(TAB_MAX_WIDTH * 2.)
        );
    }
}
