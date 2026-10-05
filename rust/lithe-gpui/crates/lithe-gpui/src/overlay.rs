//! 窗口级 overlay 层（第 9 章 Overlays）+ 菜单（第 8 章 Menus）。
//!
//! 经 gpui-base 的 `RootPlugin` 机制挂载：`init` 注册插件，之后每个经
//! `open_window` 创建的窗口都会在应用内容之上渲染本层的菜单与通知。
//! 调用方通过 [`open_menu`] / [`push_notification`] 驱动，状态由插件持有。
//!
//! v1 范围：上下文菜单（点击外部/点击菜单项关闭）、通知卡片（手动关闭）。
//! Dialog/Sheet/Tooltip 的挂载、菜单键盘导航随本章第二期接入。

use std::rc::Rc;

use gpui_kit::TestSupportExt as _;
use gpui_kit::base::{Root, RootPlugin};
use gpui_kit::prelude::*;
use gpui_kit::{
    App, Context, Entity, IntoElement, Pixels, Point, Render, SharedString, Window, div, rems,
};

use crate::theme::ThemeColors;
use crate::{ActiveTheme, Severity};

/// 菜单项动作。
pub type MenuHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// 菜单条目。
#[derive(Clone)]
pub enum MenuEntry {
    Item(MenuItem),
    Separator,
}

/// 菜单项。`on_click` 报告意图；danger 用于破坏性动作（红色文字）。
#[derive(Clone)]
pub struct MenuItem {
    pub label: SharedString,
    pub on_click: Option<MenuHandler>,
    pub disabled: bool,
    pub danger: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
            disabled: false,
            danger: false,
        }
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }
}

/// 菜单内容。打开后快照固定；要改菜单请重新 open。
#[derive(Clone, Default)]
pub struct Menu {
    pub items: Vec<MenuEntry>,
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.items.push(MenuEntry::Item(item));
        self
    }

    pub fn separator(mut self) -> Self {
        self.items.push(MenuEntry::Separator);
        self
    }
}

/// 通知卡片。
#[derive(Clone)]
pub struct Notification {
    pub title: SharedString,
    pub message: Option<SharedString>,
    pub severity: Severity,
}

impl Notification {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            message: None,
            severity: Severity::Info,
        }
    }

    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn success(mut self) -> Self {
        self.severity = Severity::Success;
        self
    }

    pub fn warning(mut self) -> Self {
        self.severity = Severity::Warning;
        self
    }

    pub fn danger(mut self) -> Self {
        self.severity = Severity::Danger;
        self
    }
}

/// 窗口级 overlay 插件状态。经 [`Root::register_plugin`] 注册，
/// 调用方经 [`open_menu`] / [`push_notification`] 驱动。
pub struct OverlayState {
    menu: Option<(Point<Pixels>, Menu)>,
    toasts: Vec<Notification>,
}

impl OverlayState {
    fn close_menu(&mut self, cx: &mut Context<Self>) {
        if self.menu.take().is_some() {
            cx.notify();
        }
    }

    fn close_toast(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix < self.toasts.len() {
            self.toasts.remove(ix);
            cx.notify();
        }
    }
}

impl RootPlugin for OverlayState {}

impl Render for OverlayState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let menu = self.menu.clone();
        let toasts = self.toasts.clone();
        div()
            .absolute()
            .inset_0()
            .when_some(menu, |el, (position, menu)| {
                el.child(menu_layer(position, menu, colors, cx.entity()))
            })
            .when(!toasts.is_empty(), |el| {
                el.child(notification_layer(toasts, colors, cx.entity()))
            })
    }
}

fn menu_layer(
    position: Point<Pixels>,
    menu: Menu,
    colors: ThemeColors,
    state: Entity<OverlayState>,
) -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        // 点击外部关闭：铺一层透明捕获面，菜单渲染在其上。
        .child({
            let state = state.clone();
            div()
                .id("menu-capture")
                .test_support()
                .absolute()
                .inset_0()
                .on_mouse_down(gpui_kit::MouseButton::Left, move |_, _, cx| {
                    state.update(cx, |state, cx| state.close_menu(cx));
                })
        })
        .child(
            div()
                .absolute()
                .left(position.x)
                .top(position.y)
                .min_w(rems(10.0))
                .max_h_full()
                .py(rems(0.25))
                .rounded(rems(0.5))
                .border_1()
                .border_color(colors.border)
                .bg(colors.popover)
                .flex()
                .flex_col()
                .children(menu.items.into_iter().enumerate().map(move |(ix, entry)| {
                    let colors = colors;
                    match entry {
                        MenuEntry::Separator => div()
                            .id(("menu-separator", ix))
                            .test_support()
                            .mx(rems(0.5))
                            .my(rems(0.125))
                            .h(rems(0.0625))
                            .bg(colors.border)
                            .into_any_element(),
                        MenuEntry::Item(item) => {
                            menu_item(ix, item, colors, state.clone()).into_any_element()
                        }
                    }
                })),
        )
}

fn menu_item(
    ix: usize,
    item: MenuItem,
    colors: ThemeColors,
    state: Entity<OverlayState>,
) -> impl IntoElement {
    let fg = if item.danger {
        colors.danger
    } else {
        colors.popover_foreground
    };
    div()
        .id(("menu-item", ix))
        .test_support()
        .mx(rems(0.5))
        .px(rems(0.625))
        .py(rems(0.3125))
        .rounded(rems(0.25))
        .text_sm()
        .text_color(if item.disabled {
            colors.disabled_foreground
        } else {
            fg
        })
        .when(!item.disabled, |el| {
            el.cursor_pointer()
                .hover(move |style| style.bg(colors.hover))
        })
        .child(item.label)
        .when_some(item.on_click.filter(|_| !item.disabled), |el, handler| {
            el.on_click(move |_, window, cx| {
                state.update(cx, |state, cx| state.close_menu(cx));
                handler(window, cx);
            })
        })
}

fn notification_layer(
    toasts: Vec<Notification>,
    colors: ThemeColors,
    state: Entity<OverlayState>,
) -> impl IntoElement {
    div()
        .absolute()
        .bottom_4()
        .right_4()
        .w(rems(22.5))
        .flex()
        .flex_col()
        .gap(rems(0.5))
        .children(
            toasts
                .into_iter()
                .enumerate()
                .map(move |(ix, note)| notification_card(ix, note, colors, state.clone())),
        )
}

fn notification_card(
    ix: usize,
    note: Notification,
    colors: ThemeColors,
    state: Entity<OverlayState>,
) -> impl IntoElement {
    div()
        .id(("toast", ix))
        .test_support()
        .rounded(rems(0.5))
        .border_1()
        .border_color(colors.border)
        .border_l_2()
        .bg(colors.popover)
        .p(rems(0.75))
        .flex()
        .items_start()
        .gap(rems(0.5))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(rems(0.25))
                .flex_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui_kit::FontWeight::MEDIUM)
                        .text_color(colors.severity(note.severity))
                        .child(note.title),
                )
                .children(
                    note.message
                        .map(|m| div().text_sm().text_color(colors.muted_foreground).child(m)),
                ),
        )
        .child(
            div()
                .id(("toast-close", ix))
                .test_support()
                .cursor_pointer()
                .text_sm()
                .text_color(colors.subtle_foreground)
                .hover(move |style| style.text_color(colors.foreground))
                .child("×")
                .on_click(move |_, _, cx| {
                    state.update(cx, |state, cx| state.close_toast(ix, cx));
                }),
        )
}

/// 打开上下文菜单。`position` 为窗口坐标（通常取 `window.mouse_position()`）。
pub fn open_menu(menu: Menu, position: Point<Pixels>, window: &mut Window, cx: &mut App) {
    let root = Root::read(window, cx);
    let Some(state) = root.plugin::<OverlayState>() else {
        panic!("overlay state is missing; call lithe_gpui::init before open_window");
    };
    state.update(cx, |state, cx| {
        state.menu = Some((position, menu));
        cx.notify();
    });
}

/// 关闭当前菜单（若开着）。
pub fn close_menu(window: &mut Window, cx: &mut App) {
    let root = Root::read(window, cx);
    if let Some(state) = root.plugin::<OverlayState>() {
        state.update(cx, |state, cx| state.close_menu(cx));
    }
}

/// 推送通知卡片（右下角）。
pub fn push_notification(note: Notification, window: &mut Window, cx: &mut App) {
    let root = Root::read(window, cx);
    let Some(state) = root.plugin::<OverlayState>() else {
        panic!("overlay state is missing; call lithe_gpui::init before open_window");
    };
    state.update(cx, |state, cx| {
        state.toasts.push(note);
        cx.notify();
    });
}

/// 注册 overlay 插件。由 crate 的 [`crate::init`] 调用；只影响之后创建的窗口。
pub fn init(cx: &mut App) {
    Root::register_plugin::<OverlayState>(cx, |_, _cx| OverlayState {
        menu: None,
        toasts: Vec::new(),
    });
}
