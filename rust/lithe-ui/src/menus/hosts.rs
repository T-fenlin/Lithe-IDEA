use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, Entity, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Pixels, Point, RenderOnce, SharedString, Stateful, Styled, Window,
    div, prelude::*,
};
use smallvec::SmallVec;

use super::{
    menu::{Open, Spot, hang, measure_host},
    model::Menu,
};
use crate::{
    buttons::{Button, ButtonGroup, ButtonVariant},
    forms::InputState,
    primitives::IconName,
    theme::Size,
};

pub(crate) type Click = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

fn state(id: &ElementId, window: &mut Window, cx: &mut App) -> Entity<Open> {
    window.use_keyed_state((id.clone(), "menu"), cx, |_, _| Open::default())
}

/// Toggles the menu under the host; a keyboard press marks its first row.
fn toggle(state: &Entity<Open>, menu: &Menu, cx: &App) -> Click {
    let (state, menu, open) = (state.clone(), menu.clone(), state.read(cx).is_open());
    Rc::new(move |event, window, cx| {
        if open {
            Open::close(&state, window, cx);
        } else {
            Open::show(&state, &menu, Spot::Under, event.is_keyboard(), cx);
        }
    })
}

/// A trigger, the canvas that measures it, and its menu while open.
fn host(
    id: ElementId,
    trigger: impl IntoElement,
    menu: &Menu,
    state: &Entity<Open>,
    field: Option<&Entity<InputState>>,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    div()
        .id(id.clone())
        .relative()
        .flex_none()
        .child(trigger)
        .child(measure_host(state.clone(), true))
        .children(hang(&id, menu, state, field, window, cx))
}

/// A menu under a trigger of the caller's making, which runs `click` when pressed.
pub(crate) fn menu_under(
    id: ElementId,
    menu: &Menu,
    trigger: impl FnOnce(Click) -> AnyElement,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let state = state(&id, window, cx);
    let click = toggle(&state, menu, cx);
    host(id, trigger(click), menu, &state, None, window, cx)
}

/// 调用方自己的触发器，和一个挂在它下面的菜单。
///
/// 与 [`DropdownMenu`] 的分工：那个的触发器是一个按钮（label + 图标），
/// 这个的触发器由调用方整块画——项目徽标 + 名字 + 箭头这种富触发器，库画不出来。
/// `trigger` 收"现在开着没有"和当帧的 [`Window`] / [`App`]（主题色那类），
/// 自己决定箭头朝哪边；库只负责把这个元素挂在菜单上面。
///
/// 面板宽度/高度跟着 [`Menu`]（见 [`Menu::width`] / [`Menu::max_height`]）。
/// 开合默认由库管（点触发器开、点外面或 Esc 关）；一排触发器里只有一个该开着、
/// 还要跟着 hover 换的那种所有者走 [`Self::open`] + [`Self::on_toggle`] 的受控模式。
#[derive(IntoElement)]
pub struct TriggerMenu {
    id: ElementId,
    menu: Menu,
    trigger: Box<dyn FnOnce(bool, &mut Window, &App) -> AnyElement>,
    /// 所有者按编号请求打开（程序化路径：快捷键、探针）。
    manual: Option<u64>,
    /// 受控模式：所有者说开还是关；`None` = 库自己管。
    controlled: Option<bool>,
    /// 库自己改了开合时回报给谁。
    report: Option<Rc<dyn Fn(bool, &mut App)>>,
    /// 触发器的可访问名；不给就用调用方画出来的内容当名字。
    aria_label: Option<SharedString>,
}

impl TriggerMenu {
    pub fn new(
        id: impl Into<ElementId>,
        menu: Menu,
        trigger: impl FnOnce(bool, &mut Window, &App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            menu,
            trigger: Box::new(trigger),
            manual: None,
            controlled: None,
            report: None,
            aria_label: None,
        }
    }

    /// 开合由所有者说了算：`Some(true)` 让库去开，`Some(false)` 让它关。
    ///
    /// 与 [`Self::manual`] 一样是**所有者驱动**的那次，因此不触发 [`Self::on_toggle`] ——
    /// 所有者本来就知道自己按了什么。
    pub fn open(mut self, open: Option<bool>) -> Self {
        self.controlled = open;
        self
    }

    /// 库自己把开合改了时叫一次（点外面、Esc、选中一行）。
    ///
    /// 受控模式的所有者用它把自己的状态对齐：面板关掉这件事库不通知就没别人知道了。
    pub fn on_toggle(mut self, handler: impl Fn(bool, &mut App) + 'static) -> Self {
        self.report = Some(Rc::new(handler));
        self
    }

    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// 由所有者按编号请求打开，不走点击：编号变了就打开一次。
    ///
    /// 与 [`ContextMenu::manual`] 同一口径。展开方向仍与点击一致（挂在触发器下面），
    /// 因为它挂的还是同一块触发器。
    pub fn manual(mut self, request: Option<u64>) -> Self {
        self.manual = request;
        self
    }
}

impl RenderOnce for TriggerMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = state(&self.id, window, cx);
        if let Some(number) = self.manual {
            let asked = window.use_keyed_state((self.id.clone(), "asked"), cx, |_, _| None);
            if *asked.read(cx) != Some(number) {
                asked.update(cx, |asked, _| *asked = Some(number));
                log::info!("trigger menu {:?}: asked to open (#{number})", self.id);
                Open::show_silently(&state, &self.menu, Spot::Under, false, cx);
            }
        }
        // 受控模式：把库里的状态对齐到所有者要的那样。这一步是所有者驱动的，不回报。
        if let Some(want) = self.controlled {
            let open = state.read(cx).is_open();
            if want != open {
                if want {
                    Open::show_silently(&state, &self.menu, Spot::Under, false, cx);
                } else {
                    Open::close_silently(&state, window, cx);
                }
            }
        }
        // 回报口径每帧重挂：`Open` 活在窗口的键控状态里，而本元素每帧新建。
        state.update(cx, |open, _| open.report(self.report.clone()));
        let open = state.read(cx).is_open();
        let click = toggle(&state, &self.menu, cx);
        let trigger = (self.trigger)(open, window, cx);
        let aria_label = self.aria_label.clone();
        // 触发器长在可拖拽的窗口壳里（标题栏那类）：按下先吃掉，别让它变成拖窗口。
        // 这层要自己的 id：`on_click` 与 `aria_label` 只在 `Stateful` 上，外层 host 的 id 已经用掉了。
        let trigger = div()
            .id((self.id.clone(), "trigger"))
            .when_some(aria_label, |trigger, label| trigger.aria_label(label))
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .on_click(move |event, window, cx| click(event, window, cx))
            .child(trigger);
        host(self.id, trigger, &self.menu, &state, None, window, cx)
    }
}

/// A button that opens a menu under it.
#[derive(IntoElement)]
pub struct DropdownMenu {
    id: ElementId,
    label: SharedString,
    icon: Option<IconName>,
    variant: ButtonVariant,
    size: Size,
    /// What the trigger says on hover; unset leaves it without one.
    tooltip: Option<SharedString>,
    menu: Menu,
}

impl DropdownMenu {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, menu: Menu) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            variant: ButtonVariant::Secondary,
            size: Size::default(),
            tooltip: None,
            menu,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// The trigger button's size tier; the default is [`Size::Medium`].
    ///
    /// Hosts that put a dropdown in a dense toolbar need this: without it the
    /// trigger would be the only button in the row at the default tier.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// The trigger button's hover text; unset leaves it without one.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

impl RenderOnce for DropdownMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = state(&self.id, window, cx);
        let click = toggle(&state, &self.menu, cx);
        let button = Button::new((self.id.clone(), "button"))
            .label(self.label)
            .variant(self.variant)
            .size(self.size)
            .trailing_icon(IconName::ChevronDown)
            .when_some(self.tooltip, |button, tooltip| button.tooltip(tooltip))
            .when_some(self.icon, |button, icon| button.icon(icon))
            .on_click(move |event, window, cx| click(event, window, cx));
        host(self.id, button, &self.menu, &state, None, window, cx)
    }
}

/// An icon button that opens a menu of what did not fit.
#[derive(IntoElement)]
pub struct OverflowMenu {
    id: ElementId,
    menu: Menu,
    icon: IconName,
    /// 以**元素**给出的图标（宿主自己的图标体系），优先于 [`Self::icon`]。
    icon_element: Option<AnyElement>,
    tooltip: SharedString,
    /// 无障碍名字。图标按钮没有可见文字，缺了它读屏软件就没有名字可念。
    aria_label: Option<SharedString>,
}

impl OverflowMenu {
    pub fn new(id: impl Into<ElementId>, menu: Menu) -> Self {
        Self {
            id: id.into(),
            menu,
            icon: IconName::Ellipsis,
            icon_element: None,
            tooltip: "More".into(),
            aria_label: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = icon;
        self
    }

    /// 图标以**元素**传入——给"图标来自宿主自己的图标体系"的场景用。
    ///
    /// 与 [`crate::buttons::Button::icon_element`] 同一口径：库内 [`IconName`] 走
    /// [`Self::icon`]，宿主（如 Lithe 的 IDEA 单色图标集）的元素走这里，两者都给时以元素为准。
    pub fn icon_element(mut self, icon: impl IntoElement) -> Self {
        self.icon_element = Some(icon.into_any_element());
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = tooltip.into();
        self
    }

    /// 无障碍名字；图标按钮没有可见文字，不给它读屏软件就没有名字可念。
    ///
    /// 与 [`crate::buttons::Button::aria_label`] 同一口径：提示语（tooltip）不是名字。
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }
}

impl RenderOnce for OverflowMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = state(&self.id, window, cx);
        let click = toggle(&state, &self.menu, cx);
        let button = Button::new((self.id.clone(), "button"))
            .variant(ButtonVariant::Ghost)
            .tooltip(self.tooltip)
            .when_some(self.aria_label, |button, label| button.aria_label(label))
            .map(|button| match self.icon_element {
                Some(icon) => button.icon_element(icon),
                None => button.icon(self.icon),
            })
            .on_click(move |event, window, cx| click(event, window, cx));
        host(self.id, button, &self.menu, &state, None, window, cx)
    }
}

/// A button's main action, and related ones in a menu under its arrow.
#[derive(IntoElement)]
pub struct SplitButton {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    menu: Menu,
    on_click: Option<Click>,
}

impl SplitButton {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, menu: Menu) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Secondary,
            menu,
            on_click: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// The main action.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SplitButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = state(&self.id, window, cx);
        let click = toggle(&state, &self.menu, cx);
        let (shut, run) = (state.clone(), self.on_click);
        let main = Button::new((self.id.clone(), "main"))
            .label(self.label)
            .variant(self.variant)
            .on_click(move |event, window, cx| {
                if shut.read(cx).is_open() {
                    Open::close(&shut, window, cx);
                }
                if let Some(run) = &run {
                    run(event, window, cx);
                }
            });
        let more = Button::new((self.id.clone(), "more"))
            .label("")
            .variant(self.variant)
            .icon(IconName::ChevronDown)
            .on_click(move |event, window, cx| click(event, window, cx));
        host(
            self.id,
            ButtonGroup::new().button(main).button(more),
            &self.menu,
            &state,
            None,
            window,
            cx,
        )
    }
}

/// Its children, and a menu that opens at the pointer on a right click.
#[derive(IntoElement)]
pub struct ContextMenu {
    id: ElementId,
    menu: Menu,
    request: Option<Option<(u64, Point<Pixels>)>>,
    children: SmallVec<[AnyElement; 2]>,
}

impl ContextMenu {
    pub fn new(id: impl Into<ElementId>, menu: Menu) -> Self {
        Self {
            id: id.into(),
            menu,
            request: None,
            children: SmallVec::new(),
        }
    }

    /// Opens on the owner's numbered request, not a right click.
    pub fn manual(mut self, request: Option<(u64, Point<Pixels>)>) -> Self {
        self.request = Some(request);
        self
    }
}

impl ParentElement for ContextMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ContextMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = state(&self.id, window, cx);
        if let Some(Some((number, at))) = self.request {
            let asked = window.use_keyed_state((self.id.clone(), "asked"), cx, |_, _| None);
            if *asked.read(cx) != Some(number) {
                asked.update(cx, |asked, _| *asked = Some(number));
                log::info!("context menu {:?}: asked at {at:?}", self.id);
                Open::show(&state, &self.menu, Spot::At(at), false, cx);
            }
        }
        let (open, shut, menu) = (state.clone(), state.clone(), self.menu.clone());
        div()
            .id(self.id.clone())
            .relative()
            .when(self.request.is_none(), |host| {
                host.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                    window.prevent_default();
                    log::info!("context menu: at {:?}", event.position);
                    Open::show(&open, &menu, Spot::At(event.position), false, cx);
                })
            })
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                if shut.read(cx).is_open() {
                    window.prevent_default();
                    Open::close(&shut, window, cx);
                }
            })
            .children(self.children)
            .child(measure_host(state.clone(), false))
            .children(hang(&self.id, &self.menu, &state, None, window, cx))
    }
}

/// A button that opens a menu headed by a filter field. Typing narrows the rows, best first, and marks the letters that matched.
#[derive(IntoElement)]
pub struct SearchableMenu {
    id: ElementId,
    label: SharedString,
    icon: Option<IconName>,
    placeholder: SharedString,
    menu: Menu,
}

impl SearchableMenu {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, menu: Menu) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            placeholder: "Filter".into(),
            menu,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }
}

impl RenderOnce for SearchableMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = state(&self.id, window, cx);
        let field = state.read(cx).is_open().then(|| {
            let placeholder = self.placeholder.clone();
            window.use_keyed_state((self.id.clone(), "filter"), cx, move |window, cx| {
                InputState::new(window, cx).placeholder(placeholder)
            })
        });
        let query = field
            .as_ref()
            .map(|field| field.read(cx).text().trim().to_string());
        let shown = match &query {
            Some(query) => self.menu.filtered(query),
            None => self.menu.clone(),
        };
        let last = window.use_keyed_state((self.id.clone(), "query"), cx, |_, _| None::<String>);
        if *last.read(cx) != query {
            last.update(cx, |last, _| *last = query.clone());
            if query.is_some() {
                log::info!(
                    "searchable menu {:?}: {} rows fit",
                    self.id,
                    shown.entries.len()
                );
                Open::restart(&state, &shown, cx);
            }
        }
        let click = toggle(&state, &shown, cx);
        let button = Button::new((self.id.clone(), "button"))
            .label(self.label)
            .trailing_icon(IconName::ChevronDown)
            .when_some(self.icon, |button, icon| button.icon(icon))
            .on_click(move |event, window, cx| click(event, window, cx));
        host(self.id, button, &shown, &state, field.as_ref(), window, cx)
    }
}
