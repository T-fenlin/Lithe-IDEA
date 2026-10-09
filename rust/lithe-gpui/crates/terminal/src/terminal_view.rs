use std::cell::RefCell;
use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, ClickEvent, Context, Entity, Focusable as _, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, WeakEntity, Window, div, rems,
};
use gpui::{Role, img, relative};
use lithe_gpui_shared::design::TAB_MAX_WIDTH;
use lithe_gpui_shared::icons::{idea, idea_icon_image, idea_icon_mono};
use lithe_ui::buttons::{Button, ButtonVariant};
use lithe_ui::feedback::EmptyState;
use lithe_ui::forms::{Input, InputState};
use lithe_ui::layout::{h_stack, v_stack};
use lithe_ui::lists::{MessageScroller, MessageScrollerState};
use lithe_ui::menus::{Menu, MenuItem, OverflowMenu};
use lithe_ui::navigation::{Tab, TabBar};
use lithe_ui::theme::{ActiveTheme as _, Size, TextSize};

use crate::constants::new_terminal_label;
use crate::constants::{
    CAPABILITY_NOTICE, TERMINAL_LINE_HEIGHT, choose_profile, clear_terminal, error_fallback,
    error_title, exit_code, failed, no_terminals, retry, running, scroll_to_end, succeeded,
    terminals_aria,
};
use crate::profile::TerminalProfile;
use crate::session::{Session, SessionState};

pub(crate) struct TabSession {
    pub(crate) id: u64,
    pub(crate) run: u64,
    pub(crate) title: SharedString,
    pub(crate) profile: TerminalProfile,
    pub(crate) session: Session,
    pub(crate) scroller: Entity<MessageScrollerState>,
    pub(crate) rows: Rc<RefCell<Vec<SharedString>>>,
}

pub struct TerminalPane {
    pub(crate) tabs: Vec<TabSession>,
    pub(crate) active: usize,
    pub(crate) next_id: u64,
    pub(crate) next_run: u64,
    pub(crate) profiles: Vec<TerminalProfile>,
    pub(crate) active_profile: usize,
    pub(crate) default_shell_id: String,
    pub(crate) font_size: Option<f32>,
    pub(crate) input: Entity<InputState>,
    pub(crate) _input_events: Subscription,
}

impl TerminalPane {
    pub(crate) fn render_tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let shell: WeakEntity<Self> = cx.entity().downgrade();
        // 一次取出**拥有所有权的**那几个值：下面 `self.profile_menu_button(cx)` 还要 `&mut cx`，
        // 不能留一个借 `cx` 的 `&Theme` 横跨那段（`Theme` 自己不是 `Clone`）。
        let colors = cx.theme().colors.clone();
        let small_text = cx.theme().text_size(TextSize::Sm);
        let rem = cx.theme().font_size_px();
        let is_dark = cx.theme().is_dark();

        if self.tabs.is_empty() {
            return h_stack()
                .w_full()
                .flex_shrink_0()
                .h_9()
                .gap_1p5()
                .px_2()
                .bg(colors.tab_bar)
                .border_b_1()
                .border_color(colors.border)
                // 图标口径：应用界面的图标只取本仓自有图标集，我们**没有**终端图标 ⇒ 留空
                // （占住同一个图标位，不画图形），不拿组件库的 `IconName` 顶替。
                .child(div().flex_none().size_6())
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(small_text)
                        .text_color(colors.fg_muted)
                        .child(no_terminals()),
                )
                .child(self.new_tab_button(shell, cx))
                .into_any_element();
        }

        let close_icon_path = SharedString::from(idea::X_ICON.path(is_dark));

        let tabs: Vec<Tab> = self
            .tabs
            .iter()
            .map(|tab| {
                let id = tab.id;
                let name = tab.title.clone();
                let close_shell = shell.clone();
                let close_label = lithe_gpui_shared::tr_args(
                    "lithe.terminal.tabClose",
                    &[("name", name.as_ref())],
                );
                let close = Button::new(SharedString::from(format!("terminal-tab-close-{id}")))
                    .variant(ButtonVariant::Ghost)
                    .icon_element(img(close_icon_path.clone()).flex_shrink_0().size_4())
                    .size(Size::XSmall)
                    .tooltip(close_label.clone())
                    .on_click(
                        move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                            cx.stop_propagation();
                            let _ = close_shell.update(cx, |this, cx| this.close_tab(id, cx));
                        },
                    );

                Tab::new(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(small_text)
                        .child(name.clone()),
                )
                .aria_label(name)
                .suffix(close)
            })
            .collect();

        let switch_shell = shell.clone();
        let bar = TabBar::new("terminal-tab-bar")
            .selected_index(self.active)
            .max_width(rem_px(rem, TAB_MAX_WIDTH))
            .children(tabs)
            .suffix(
                h_stack()
                    .flex_shrink_0()
                    .items_center()
                    .gap_0p5()
                    .child(self.clear_button(shell.clone(), cx))
                    .child(self.new_tab_button(shell.clone(), cx))
                    .child(self.profile_menu_button(cx)),
            )
            .on_click(move |index: usize, window: &mut Window, cx: &mut App| {
                let _ = switch_shell.update(cx, |this, cx| {
                    this.active = index;
                    let input = this.input.clone();
                    let handle = input.read(cx).focus_handle(cx);
                    window.focus(&handle, cx);
                    cx.notify();
                });
            });

        v_stack()
            .w_full()
            .flex_shrink_0()
            .h_9()
            .bg(colors.tab_bar)
            .border_b_1()
            .border_color(colors.border)
            .child(bar)
            .into_any_element()
    }

    pub(crate) fn render_status_line(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(tab) = self.tabs.get(self.active) else {
            return div().into_any_element();
        };

        let (label, color) = match &tab.session.state {
            SessionState::Running => {
                let label = match &tab.session.io_error {
                    Some(error) => SharedString::from(format!("{} · {error}", running())),
                    None => running(),
                };
                (label, cx.theme().colors.success)
            }
            SessionState::Exited(0) => (succeeded(), cx.theme().colors.success),
            SessionState::Exited(code) => (
                SharedString::from(format!("{} · {} {code}", failed(), exit_code())),
                cx.theme().colors.danger,
            ),
            SessionState::Failed(message) => (
                SharedString::from(format!("{}：{message}", error_title())),
                cx.theme().colors.danger,
            ),
        };

        let id = tab.id;
        let retryable = !tab.session.is_running();

        let mut line = h_stack()
            .w_full()
            .flex_shrink_0()
            .h_6()
            .gap_1p5()
            .px_3()
            .bg(cx.theme().colors.sunken)
            .border_b_1()
            .border_color(cx.theme().colors.border)
            .child(
                div()
                    .max_w_80()
                    .truncate()
                    .text_xs()
                    .text_color(color)
                    .child(label),
            );

        if retryable {
            // 库 `Button` 没有 gpui 的 `Styled`，所以高度与圆角归它自己（`Size::XSmall` = 20px，
            // 状态行 24px 内够放）。宿主原来的 `h(20px) + rounded(4px)` 在库侧没有落点，
            // 这一处几何差异登记在汇报的缺口清单里。
            line = line.child(
                Button::new(SharedString::from(format!("terminal-retry-{id}")))
                    .variant(ButtonVariant::Ghost)
                    .icon_element(idea_icon_image(&idea::PLAY_ICON, cx, rems(1.)))
                    .label(retry())
                    .size(Size::XSmall)
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
                .text_color(cx.theme().colors.fg_muted)
                .child(SharedString::from(CAPABILITY_NOTICE)),
        )
        .into_any_element()
    }

    pub(crate) fn render_output(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(tab) = self.tabs.get(self.active) else {
            return self.render_empty(cx);
        };
        if let SessionState::Failed(message) = &tab.session.state {
            return self.render_failed(message.clone(), cx);
        }

        let rows = tab.rows.clone();
        let mono = cx.theme().mono_family.clone();
        let foreground = cx.theme().colors.fg;
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
                    .text_size(rems(12. / 16.))
                    .when_some(row_font_size, |this, size| this.text_size(rems(size / 16.)))
                    .line_height(relative(TERMINAL_LINE_HEIGHT))
                    .text_color(cx.theme().colors.fg)
                    .child(text)
                    .into_any_element()
            },
        )
        .scrollbar(true)
        .jump_button(true)
        .with_jump_button_label(scroll_to_end());

        let partial = tab.session.text.current_text();

        v_stack()
            .flex_1()
            .min_h_0()
            .w_full()
            .bg(cx.theme().colors.bg)
            .child(
                div()
                    .id("terminal-output-viewport")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .role(Role::Log)
                    .aria_label(terminals_aria())
                    .child(div().size_full().child(scroller)),
            )
            .when_some(partial, |this, line| {
                this.child(
                    h_stack()
                        .w_full()
                        .flex_shrink_0()
                        .pl_4()
                        .pr_4()
                        .font_family(mono.clone())
                        .text_size(rems(12. / 16.))
                        .when_some(self.font_size, |this, size| {
                            this.text_size(rems(size / 16.))
                        })
                        .line_height(relative(TERMINAL_LINE_HEIGHT))
                        .text_color(foreground)
                        .child(line),
                )
            })
            .into_any_element()
    }

    pub(crate) fn render_empty(&self, cx: &mut Context<Self>) -> AnyElement {
        v_stack()
            .size_full()
            .child(
                EmptyState::slots("terminal-empty")
                    // 图标口径：本仓自有图标集里**没有**终端图标 ⇒ 头槽位留空（不画图形），
                    // 不拿组件库的 `IconName` 顶替。
                    .media_element(div().flex_none().size_8())
                    .title(no_terminals())
                    .content(
                        Button::new("terminal-empty-new")
                            .icon_element(idea_icon_image(&idea::PLUS_ICON, cx, rems(1.)))
                            .label(new_terminal_label())
                            .size(Size::Small)
                            .on_click(cx.listener(
                                |this: &mut Self,
                                 _event: &ClickEvent,
                                 window: &mut Window,
                                 cx: &mut Context<Self>| {
                                    this.new_tab(window, cx);
                                },
                            )),
                    ),
            )
            .into_any_element()
    }

    pub(crate) fn render_failed(&self, message: String, cx: &mut Context<Self>) -> AnyElement {
        let id = self
            .tabs
            .get(self.active)
            .map(|tab| tab.id)
            .unwrap_or_default();
        v_stack()
            .size_full()
            .child(
                EmptyState::slots("terminal-failed")
                    // 图标口径：本仓自有图标集里**没有**终端图标 ⇒ 头槽位留空（不画图形）。
                    .media_element(div().flex_none().size_8())
                    .title(error_title())
                    .body(SharedString::from(message))
                    .content(
                        Button::new("terminal-failed-retry")
                            .icon_element(idea_icon_image(&idea::PLAY_ICON, cx, rems(1.)))
                            .label(retry())
                            .size(Size::Small)
                            .on_click(cx.listener(
                                move |this: &mut Self,
                                      _event: &ClickEvent,
                                      _window: &mut Window,
                                      cx: &mut Context<Self>| {
                                    this.retry_tab(id, cx);
                                },
                            )),
                    ),
            )
            .into_any_element()
    }

    pub(crate) fn render_input_row(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tab = self.tabs.get(self.active)?;
        if !tab.session.is_running() {
            return None;
        }

        // 库的 `Input` 是 `RenderOnce`（没有 gpui 的 `Styled`），高度与字号档位归它自己：
        // `Size::Medium` = 28px、`text_size(Size::Medium)` = 13px，正对宿主原来
        // `gpui::Styled::h_7(Input::new(..))`（28px）+ `.text_xs()`。
        // 等宽字体这里没有落点（库在渲染函数里自己写死 `font_family`），
        // 输入行因此用界面字体而不是终端等宽字体——这一处差异登记在汇报里。
        let input = Input::new(&self.input).size(Size::Medium);

        Some(
            h_stack()
                .w_full()
                .flex_shrink_0()
                .gap_1p5()
                .px_3()
                .py_1p5()
                .bg(cx.theme().colors.bg)
                .border_t_1()
                .border_color(cx.theme().colors.border)
                .child(input)
                .into_any_element(),
        )
    }

    pub(crate) fn new_tab_button(&self, shell: WeakEntity<Self>, cx: &App) -> Button {
        // 库 `Button` 没有 gpui 的 `Styled`：`px_0 / size_6 / rounded` 在库侧没有落点，
        // 几何（图标按钮的方形尺寸）归库自己，这一处差异登记在汇报的缺口清单里。
        Button::new("terminal-new-tab")
            .variant(ButtonVariant::Ghost)
            .icon_element(idea_icon_image(&idea::PLUS_ICON, cx, rems(1.)))
            .tooltip(new_terminal_label())
            .on_click(
                move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
                    let _ = shell.update(cx, |this, cx| this.new_tab(window, cx));
                },
            )
    }

    pub(crate) fn clear_button(&self, shell: WeakEntity<Self>, cx: &App) -> Button {
        Button::new("terminal-clear-output")
            .variant(ButtonVariant::Ghost)
            .icon_element(idea_icon_image(&idea::TRASH_ICON, cx, rems(1.)))
            .tooltip(clear_terminal())
            .disabled(self.tabs.is_empty())
            .on_click(
                move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                    let _ = shell.update(cx, |this, cx| this.clear_active_output(cx));
                },
            )
    }

    pub(crate) fn profile_menu_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let shell: WeakEntity<Self> = cx.entity().downgrade();
        let profiles: Vec<SharedString> = self
            .profiles
            .iter()
            .map(|profile| profile.name().clone())
            .collect();
        let active = self.active_profile;
        let is_dark = cx.theme().is_dark();

        let mut menu = Menu::new();
        if profiles.is_empty() {
            menu = menu.item(MenuItem::new(error_fallback()).disabled(true));
        } else {
            for (index, name) in profiles.iter().enumerate() {
                let shell = shell.clone();
                menu = menu.item(MenuItem::radio(name.clone(), index == active).on_click(
                    move |_window, cx| {
                        let _ = shell.update(cx, |this, cx| {
                            this.active_profile = index;
                            cx.notify();
                        });
                    },
                ));
            }
        }

        // `OverflowMenu` 的几何归库自己（它没有 gpui 的 `Styled`），所以这里不给尺寸：
        // 宿主原来那套 `px_0 / size_6 / rounded` 在库侧没有落点，登记在汇报里。
        OverflowMenu::new("terminal-profile-menu", menu)
            .icon_element(idea_icon_mono(&idea::CARET_DOWN_ICON, is_dark))
            .tooltip(choose_profile())
            .into_any_element()
    }
}

impl Render for TerminalPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tab_bar = self.render_tab_bar(cx);
        let status = self.render_status_line(cx);
        let output = self.render_output(cx);
        let input_row = self.render_input_row(cx);

        v_stack()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().colors.bg)
            .text_color(cx.theme().colors.fg)
            .child(tab_bar)
            .child(status)
            .child(output)
            .children(input_row)
    }
}

impl Drop for TerminalPane {
    fn drop(&mut self) {
        for tab in &mut self.tabs {
            tab.session.shutdown();
        }
        if !self.tabs.is_empty() {
            println!("S1_TERMINAL_DROP tabs={}", self.tabs.len());
        }
    }
}

pub(crate) use lithe_gpui_shared::rem_px;

#[cfg(test)]
mod tests {
    // `CHROME_RADIUS` 与 `STATUS_LINE_HEIGHT` 现在只剩这里的换算断言在读：宿主那两处
    // `h(...)` / `rounded(...)` 随库 `Button`/`Input` 的几何收口而消失（见上面的差异注释）。
    use super::*;
    use crate::constants::{CHROME_RADIUS, STATUS_LINE_HEIGHT};
    use gpui::{AbsoluteLength, px};

    #[test]
    fn rem_conversions_match_the_spec_pixels() {
        assert_eq!(rem_px(px(16.), CHROME_RADIUS), px(CHROME_RADIUS));
        assert_eq!(rem_px(px(16.), TAB_MAX_WIDTH), px(TAB_MAX_WIDTH));
        assert_eq!(
            rem_px(px(16.), STATUS_LINE_HEIGHT - 4.),
            px(STATUS_LINE_HEIGHT - 4.)
        );

        let spec_to_px =
            |spec_px: f32| AbsoluteLength::from(rems(spec_px / 16.)).to_pixels(px(16.));
        assert_eq!(spec_to_px(CHROME_RADIUS), px(CHROME_RADIUS));
        assert_eq!(
            spec_to_px(STATUS_LINE_HEIGHT - 4.),
            px(STATUS_LINE_HEIGHT - 4.)
        );

        assert_eq!(rem_px(px(32.), CHROME_RADIUS), px(CHROME_RADIUS * 2.));
        assert_eq!(
            AbsoluteLength::from(rems(TAB_MAX_WIDTH / 16.)).to_pixels(px(32.)),
            px(TAB_MAX_WIDTH * 2.)
        );
    }
}
