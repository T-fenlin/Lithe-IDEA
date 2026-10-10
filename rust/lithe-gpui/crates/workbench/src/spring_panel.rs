use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, SharedString,
    Styled as _, Subscription, Window, div, rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_mono};
use lithe_gpui_shared::tr;
use lithe_ui::forms::{Input, InputEvent, InputState};
use lithe_ui::layout::h_stack;
use lithe_ui::theme::ActiveTheme as _;

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
                if matches!(event, InputEvent::Changed) {
                    this.query = state.read(cx).text().into();
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
        h_stack()
            .w_full()
            .flex_shrink_0()
            .gap(rems(7. / 16.))
            .px(rems(10. / 16.))
            .h(rems(34. / 16.))
            .items_center()
            // 放大镜取**本仓**图标集（`icons/idea/**`），不借组件库的 `IconName`。
            .child(
                idea_icon_mono(&idea::MAGNIFYING_GLASS_ICON, cx.theme().is_dark())
                    .text_color(cx.theme().colors.fg_muted),
            )
            .child(div().flex_1().min_w_0().child(Input::new(&self.input)))
    }
}
