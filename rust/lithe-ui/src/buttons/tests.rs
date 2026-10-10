use gpui::{
    Context, FocusHandle, InteractiveElement as _, IntoElement, KeyBinding, KeyUpEvent, Keystroke,
    Modifiers, ParentElement, Render, Styled, TestAppContext, VisualTestContext, Window, div,
    point, px,
};

use super::{Button, ToggleButton, ToggleItem};
use crate::{
    primitives::{FocusNext, FocusScope},
    theme::Theme,
};

/// One button that counts its presses.
struct Bench {
    root: FocusHandle,
    loading: bool,
    presses: usize,
}

impl Render for Bench {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        FocusScope::new(&self.root).root().size_full().child(
            Button::new("save")
                .label("Save")
                .loading(self.loading)
                .on_click(move |_, _, cx| view.update(cx, |bench, _| bench.presses += 1)),
        )
    }
}

fn press(key: &str, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(key).expect("a key"),
    });
    cx.run_until_parked();
}

#[gpui::test]
fn a_loading_button_ignores_presses_and_keeps_its_tab_stop(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        cx.bind_keys([KeyBinding::new("tab", FocusNext, None)]);
    });
    let (view, cx) = cx.add_window_view(|_, cx| Bench {
        root: cx.focus_handle(),
        loading: true,
        presses: 0,
    });
    let root = view.read_with(cx, |bench, _| bench.root.clone());
    cx.update(|window, cx| window.focus(&root, cx));
    press("tab", cx);
    press("enter", cx);
    assert_eq!(view.read_with(cx, |bench, _| bench.presses), 0);
    view.update(cx, |bench, cx| {
        bench.loading = false;
        cx.notify();
    });
    cx.run_until_parked();
    press("enter", cx);
    assert_eq!(
        view.read_with(cx, |bench, _| bench.presses),
        1,
        "focus stayed on it"
    );
}

/// One toggle that counts its presses.
struct Toggles {
    disabled: bool,
    presses: usize,
}

impl Render for Toggles {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().size_full().child(
            ToggleButton::new("bold", ToggleItem::new("bold").label("B"), false)
                .disabled(self.disabled)
                .on_toggle(move |_, _, cx| view.update(cx, |toggles, _| toggles.presses += 1)),
        )
    }
}

#[gpui::test]
fn a_disabled_toggle_ignores_clicks(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (view, cx) = cx.add_window_view(|_, _| Toggles {
        disabled: true,
        presses: 0,
    });
    cx.simulate_click(point(px(4.0), px(4.0)), Modifiers::none());
    assert_eq!(view.read_with(cx, |toggles, _| toggles.presses), 0);
    view.update(cx, |toggles, cx| {
        toggles.disabled = false;
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_click(point(px(4.0), px(4.0)), Modifiers::none());
    assert_eq!(view.read_with(cx, |toggles, _| toggles.presses), 1);
}

/// The rule an icon-only button leans on: with no visible text, `aria_label` is the only name it
/// has, and it must win over an empty label.
#[test]
fn an_icon_only_button_is_named_by_its_aria_label() {
    assert_eq!(
        Button::new("close").accessible_name().as_ref(),
        "",
        "an icon-only button with neither has no name"
    );
    assert_eq!(
        Button::new("close")
            .aria_label("Close")
            .accessible_name()
            .as_ref(),
        "Close"
    );
    assert_eq!(
        Button::new("save").label("Save").accessible_name().as_ref(),
        "Save",
        "a visible label still names it"
    );
    assert_eq!(
        Button::new("save")
            .label("Save")
            .aria_label("Save the buffer")
            .accessible_name()
            .as_ref(),
        "Save the buffer",
        "the explicit name wins"
    );
}

/// One keeper beside one button the host keeps out of the tab order.
struct TabOrder {
    root: FocusHandle,
    reached: usize,
}

impl Render for TabOrder {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        FocusScope::new(&self.root).root().size_full().child(
            div()
                .flex()
                .child(Button::new("skipped").label("Close").tab_stop(false))
                .child(Button::new("reached").label("Save").on_click({
                    let view = view.clone();
                    move |_, _, cx| view.update(cx, |order, _| order.reached += 1)
                })),
        )
    }
}

/// Tab skips a button built with `tab_stop(false)`, so a host's close control is not a stray stop.
#[gpui::test]
fn tab_skips_a_button_taken_out_of_the_order(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        cx.bind_keys([KeyBinding::new("tab", FocusNext, None)]);
    });
    let (view, cx) = cx.add_window_view(|_, cx| TabOrder {
        root: cx.focus_handle(),
        reached: 0,
    });
    let root = view.read_with(cx, |order, _| order.root.clone());
    cx.update(|window, cx| window.focus(&root, cx));
    press("tab", cx);
    press("enter", cx);
    assert_eq!(
        view.read_with(cx, |order, _| order.reached),
        1,
        "Tab landed on the button after the skipped one"
    );
}

/// 一个没有 label、只靠调用方自绘图标命名的开关（宿主活动栏那种图标按钮）。
struct IconToggle;

impl Render for IconToggle {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(ToggleButton::new(
            "terminal",
            ToggleItem::new("terminal").icon_element(|_| {
                div()
                    .size_4()
                    .debug_selector(|| "toggle-icon-element".into())
                    .into_any_element()
            }),
            true,
        ))
    }
}

#[gpui::test]
fn a_toggle_shows_the_icon_element_it_is_given(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| IconToggle);
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("toggle-icon-element").is_some(),
        "宿主自带的图标元素画在开关里，且无 label 也成立"
    );
}
