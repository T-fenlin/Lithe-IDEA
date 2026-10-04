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
    AnyElement, App, ClickEvent, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled as _, Subscription, WeakEntity, Window, div, img, relative, rems,
};
use lithe_gpui_shared::design::TAB_MAX_WIDTH;
use lithe_gpui_shared::icons::{idea, idea_icon_image};

use crate::constants::new_terminal_label;
use crate::constants::{
    CAPABILITY_NOTICE, CHROME_RADIUS, STATUS_LINE_HEIGHT, TERMINAL_LINE_HEIGHT,
    TERMINAL_PADDING_INLINE, choose_profile, clear_terminal, error_fallback, error_title,
    exit_code, failed, no_terminals, retry, running, scroll_to_end, succeeded, terminals_aria,
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

        let rem = cx.theme().font_size;
        let close_icon_path = SharedString::from(idea::X_ICON.path(cx.theme().is_dark()));

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
                    .ghost()
                    .child(img(close_icon_path.clone()).flex_shrink_0().size_4())
                    .px_0()
                    .tab_stop(false)
                    .size_6()
                    .rounded(rem_px(rem, CHROME_RADIUS))
                    .tooltip(close_label.clone())
                    .accessibility_label(close_label)
                    .on_click(
                        move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                            cx.stop_propagation();
                            let _ = close_shell.update(cx, |this, cx| this.close_tab(id, cx));
                        },
                    );

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
                    .child(idea_icon_image(&idea::PLAY_ICON, cx, rems(1.)))
                    .child(retry())
                    .accessibility_label(retry())
                    .tab_stop(false)
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
                    .when_some(row_font_size, |this, size| this.text_size(rems(size / 16.)))
                    .line_height(relative(TERMINAL_LINE_HEIGHT))
                    .text_color(cx.theme().foreground)
                    .child(text)
            },
        )
        .scrollbar(true)
        .jump_button(true)
        .with_jump_button_label(scroll_to_end())
        .with_row_style(output_row_style());

        let partial = tab.session.text.current_text();

        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .bg(cx.theme().background)
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
                    h_flex()
                        .w_full()
                        .flex_shrink_0()
                        .pl_4()
                        .pr_4()
                        .font_family(mono.clone())
                        .text_sm()
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
        v_flex()
            .size_full()
            .child(
                Empty::new()
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
                                .child(idea_icon_image(&idea::PLUS_ICON, cx, rems(1.)))
                                .child(new_terminal_label())
                                .accessibility_label(new_terminal_label())
                                .small()
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

    pub(crate) fn render_failed(&self, message: String, cx: &mut Context<Self>) -> AnyElement {
        let id = self
            .tabs
            .get(self.active)
            .map(|tab| tab.id)
            .unwrap_or_default();
        v_flex().size_full().child(
            Empty::new()
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
                        .child(idea_icon_image(&idea::PLAY_ICON, cx, rems(1.)))
                        .child(retry())
                        .accessibility_label(retry())
                        .small()
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

    pub(crate) fn render_input_row(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tab = self.tabs.get(self.active)?;
        if !tab.session.is_running() {
            return None;
        }

        let input = gpui_kit::Styled::h_7(Input::new(&self.input))
            .w_full()
            .min_w_0()
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

    pub(crate) fn new_tab_button(&self, shell: WeakEntity<Self>, cx: &App) -> Button {
        Button::new("terminal-new-tab")
            .ghost()
            .child(idea_icon_image(&idea::PLUS_ICON, cx, rems(1.)))
            .px_0()
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

    pub(crate) fn clear_button(&self, shell: WeakEntity<Self>, cx: &App) -> Button {
        Button::new("terminal-clear-output")
            .ghost()
            .child(idea_icon_image(&idea::TRASH_ICON, cx, rems(1.)))
            .px_0()
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
            .child(idea_icon_image(&idea::CARET_DOWN_ICON, cx, rems(1.)))
            .px_0()
            .tab_stop(false)
            .size_6()
            .rounded(rem_px(cx.theme().font_size, CHROME_RADIUS))
            .tooltip(choose_profile())
            .accessibility_label(choose_profile())
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

fn output_row_style() -> StyleRefinement {
    let mut style = StyleRefinement::default();
    let inline = rems(TERMINAL_PADDING_INLINE / 16.);
    let zero = rems(0.);
    style.padding.left = Some(inline.into());
    style.padding.right = Some(inline.into());
    style.padding.top = Some(zero.into());
    style.padding.bottom = Some(zero.into());
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{AbsoluteLength, px};

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
