//! Spring 面板的**筛选状态宿主**。
//!
//! ## 为什么单独一个实体
//!
//! `right_tool_window(..)` 是**无状态函数**（`ShellWorkspace::render` 直接调用），而真源的
//! Spring 面板要一个筛选框（`SpringEndpointsView.swift:15-22`）：输入框需要一个 `InputState`
//! 与查询串的持有者。照 [`crate::notifications::NotificationPanel`] 的先例做成实体，由
//! `ShellWorkspace` 持有 + `observe`（输入一变就重渲染），**面板本身不画内容** ——
//! 端点数据在 [`crate::spring`]，渲染在 `right_tool_window::spring_content`。
//!
//! 这样"状态"与"渲染"各归其位：实体只回答"当前查询串是什么"与"筛选行长什么样"。

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
    /// 当前查询串；判据见 [`crate::spring::filter_endpoints`]。
    query: SharedString,
    /// 盯住输入框：面板自己 `notify()`，持有它的 `ShellWorkspace` 因 `observe` 而重渲染。
    _input_subscription: Subscription,
}

impl SpringPanel {
    /// 建面板。查询串初值为空 —— 空查询按真源判据返回全部端点，所以首帧就是"没有筛选"。
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

    /// 当前查询串（渲染层交给 [`crate::spring::filter_endpoints`]）。
    pub fn query(&self) -> &SharedString {
        &self.query
    }

    /// 真源的筛选行（`SpringEndpointsView.swift:15-23`）：放大镜 + 输入框，
    /// `HStack(spacing: 7)`、内衬左右 10、高 34。
    ///
    /// ⚠️ 真源这一行的底是 `litheWorkbenchSurface(LitheTheme.editor)`，而本侧**没有**
    /// `editor` 这条语义 role（与 `toolHeader` 同一类缺口），所以这里不设底色、由面板表面透出；
    /// 缺口登记进 UI-MAP 校正专章。
    pub fn filter_row(&self, cx: &App) -> impl IntoElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .gap(rems(7. / 16.))
            .px(rems(10. / 16.))
            .h(rems(34. / 16.))
            .items_center()
            // 真源是 `Image(systemName: "magnifyingglass")` + `secondaryText`。
            .child(Icon::new(IconName::Search).text_color(cx.theme().muted_foreground))
            .child(div().flex_1().min_w_0().child(Input::new(&self.input)))
    }
}
