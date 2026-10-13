use std::{ops::Range, rc::Rc};

use gpui::{
    App, Context, ElementId, Entity, FocusHandle, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Subscription,
    Window, div, prelude::*,
};

use super::{Input, InputEvent, InputState};
use crate::{
    primitives::{FocusRing, Icon, IconName, tab_stop},
    theme::{ActiveTheme, IconSize, Radius, Size},
    typography::Ellipsis,
};

pub(crate) type OnCommit = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
/// Says why some text cannot be kept yet; a refusal holds the field open for the owner to explain.
pub(crate) type OnCheck = Rc<dyn Fn(&SharedString) -> Result<(), SharedString>>;

/// A text being edited in place: its field while open, who hears the kept text, and where focus goes back after Enter or Escape. An edit that hands focus back keeps its text on Enter's release, so the release does not open it again.
#[derive(Default)]
pub(crate) struct Editing {
    field: Option<(Entity<InputState>, Subscription)>,
    pub on_commit: Option<OnCommit>,
    back: Option<FocusHandle>,
    /// Asked before Enter or a leaving field keeps the text; a refusal holds the field open.
    check: Option<OnCheck>,
    /// Why the last submit was held back; cleared once the text changes or passes.
    refusal: Option<SharedString>,
}

impl Editing {
    /// Opens a field on `value`, all of it selected and focused; Enter or leaving keeps it.
    pub(crate) fn begin(state: &Entity<Editing>, value: String, window: &mut Window, cx: &mut App) {
        Self::open(state, value, None, None, window, cx);
    }

    /// Opens a field on `value`, focused. `select` is the range left selected, whole value when it
    /// is `None` — a rename selects the main name, not the extension. `check`, when given, is asked
    /// before Enter or a leaving field keeps the text: a refusal holds the field open and is read
    /// back through [`Self::refusal`].
    pub(crate) fn open(
        state: &Entity<Editing>,
        value: String,
        select: Option<Range<usize>>,
        check: Option<OnCheck>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let field = cx.new(|cx| {
            let mut input = InputState::new(window, cx);
            input.set_text(value, cx);
            input
        });
        let all = field.read(cx).text().len();
        let range = match select {
            Some(range) => {
                let end = range.end.min(all);
                range.start.min(end)..end
            }
            None => 0..all,
        };
        field.update(cx, |input, cx| input.select(range, cx));
        window.focus(&field.read(cx).focus().clone(), cx);
        let owner = state.clone();
        let events = window.subscribe(&field, cx, move |_, event, window, cx| match event {
            InputEvent::Submit => owner.update(cx, |editing, cx| {
                if editing.back.is_none() {
                    editing.keep(window, cx);
                }
            }),
            InputEvent::Blur => owner.update(cx, |editing, cx| {
                editing.back = None;
                editing.keep(window, cx);
            }),
            InputEvent::Changed => owner.update(cx, |editing, cx| {
                if editing.refusal.take().is_some() {
                    cx.notify();
                }
            }),
            _ => {}
        });
        state.update(cx, |editing, cx| {
            editing.field = Some((field, events));
            editing.check = check;
            editing.refusal = None;
            cx.notify();
        });
    }

    /// Focus goes back here after Enter or Escape.
    pub(crate) fn returning(&mut self, back: FocusHandle) {
        self.back = Some(back);
    }

    pub(crate) fn give_back(&mut self, window: &mut Window, cx: &mut App) {
        if let Some(back) = self.back.take() {
            log::info!("inline edit: focus handed back");
            window.focus(&back, cx);
        }
    }

    pub(crate) fn field(&self) -> Option<Entity<InputState>> {
        self.field.as_ref().map(|(field, _)| field.clone())
    }

    /// Why the last submit was held back, if any; the owner draws it beside the field.
    pub(crate) fn refusal(&self) -> Option<SharedString> {
        self.refusal.clone()
    }

    /// Keeps the text the field holds, unless the check refuses it — a refusal leaves the field
    /// open and the reason on [`Self::refusal`].
    fn keep(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((field, _)) = self.field.as_ref() else {
            return;
        };
        let text = SharedString::from(field.read(cx).text().to_string());
        if let Some(check) = self.check.clone()
            && let Err(reason) = check(&text)
        {
            self.refusal = Some(reason);
            cx.notify();
            return;
        }
        self.refusal = None;
        self.finish(false, window, cx);
    }

    /// Ends editing; keeps the text unless `cancel`.
    pub(crate) fn finish(&mut self, cancel: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some((field, _)) = self.field.take() else {
            return;
        };
        self.refusal = None;
        if cancel {
            log::info!("inline edit: cancelled");
        } else {
            let text = SharedString::from(field.read(cx).text().to_string());
            log::info!("inline edit: kept");
            if let Some(commit) = self.on_commit.clone() {
                commit(&text, window, cx);
            }
        }
        cx.notify();
    }
}

/// Text that becomes a field when clicked. Enter or leaving keeps; Escape reverts.
#[derive(IntoElement)]
pub struct InlineEdit {
    id: ElementId,
    value: SharedString,
    placeholder: SharedString,
    on_commit: Option<OnCommit>,
}

impl InlineEdit {
    pub fn new(id: impl Into<ElementId>, value: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            placeholder: "Empty".into(),
            on_commit: None,
        }
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn on_commit(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_commit = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for InlineEdit {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Editing::default());
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        state.update(cx, |editing, _| editing.on_commit = self.on_commit.clone());
        if let Some(field) = state.read(cx).field() {
            let (escape, enter) = (state.clone(), state.clone());
            return div()
                .id(self.id)
                .on_key_down(move |event, window, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        escape.update(cx, |editing, cx| {
                            editing.finish(true, window, cx);
                            editing.give_back(window, cx);
                        });
                    }
                })
                .on_key_up(move |event, window, cx| {
                    if event.keystroke.key == "enter" {
                        cx.stop_propagation();
                        enter.update(cx, |editing, cx| {
                            editing.finish(false, window, cx);
                            editing.give_back(window, cx);
                        });
                    }
                })
                .child(Input::new(&field).size(Size::Small))
                .into_any_element();
        }
        let theme = cx.theme();
        let colors = &theme.colors;
        let empty = self.value.is_empty();
        let (value, start, back) = (self.value.clone(), state, focus.clone());
        let group = SharedString::from(format!("inline-edit-{:?}", self.id));
        let label = div()
            .debug_selector(|| format!("inline-edit {}", self.id))
            .flex_1()
            .min_w_0()
            .child(Ellipsis::new(if empty {
                self.placeholder
            } else {
                self.value
            }));
        div()
            .id(self.id)
            .group(group.clone())
            .flex()
            .items_center()
            .gap_2()
            .h(theme.control_height(Size::Small))
            .px(theme.control_padding(Size::Small))
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(gpui::transparent_black())
            .track_focus(&focus)
            .focus_ring(cx)
            .cursor_text()
            .hover(|style| style.bg(colors.hover))
            .text_color(if empty { colors.fg_subtle } else { colors.fg })
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(move |_, window, cx| {
                log::info!("inline edit: editing");
                start.update(cx, |editing, _| editing.returning(back.clone()));
                Editing::begin(&start, value.to_string(), window, cx);
            })
            .child(label)
            .child(
                div()
                    .flex_none()
                    .invisible()
                    .group_hover(group, |style| style.visible())
                    .child(
                        Icon::new(IconName::Pencil)
                            .size(IconSize::Xs)
                            .color(colors.fg_subtle),
                    ),
            )
            .into_any_element()
    }
}
