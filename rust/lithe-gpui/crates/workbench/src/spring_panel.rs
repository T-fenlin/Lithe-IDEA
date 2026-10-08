use gpui_kit::base::h_flex;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Icon, IconName};
use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, SharedString,
    Styled as _, Subscription, Window, div, rems,
};

use lithe_gpui_shared::tr;

pub struct SpringPanel {
    input: Entity<InputState>,
    query: SharedString,
    _input_subscription: Subscription,
}

impl SpringPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder(tr("lithe.spring.gpui.filterPlaceholder"))
        });
        let subscription = cx.subscribe_in(
            &input,
            window,
            |this: &mut Self, state: &Entity<InputState>, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.query = state.read(cx).value();
                    cx.notify();
                }
            },
        );
        Self {
            input,
            query: SharedString::default(),
            _input_subscription: subscription,
        }
    }

    pub fn query(&self) -> &SharedString {
        &self.query
    }

    pub fn filter_row(&self, cx: &App) -> impl IntoElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .gap(rems(7. / 16.))
            .px(rems(10. / 16.))
            .h(rems(34. / 16.))
            .items_center()
            .child(Icon::new(IconName::Search).text_color(cx.theme().muted_foreground))
            .child(div().flex_1().min_w_0().child(Input::new(&self.input)))
    }
}
