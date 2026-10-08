use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, AnyElement, App, Div, ElementId, FontWeight, IntoElement,
    ParentElement, RenderOnce, SharedString, Styled, Window, div, prelude::*,
};
use smallvec::SmallVec;

use crate::{
    buttons::Button,
    forms::Run,
    motion,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::tabular,
};

/// The words and buttons under a state's head.
pub(super) struct Words {
    /// 只有说明、没有标题的形态是 `None`：标题行不画，它那份字号与间距也不再占位。
    title: Option<SharedString>,
    pub(super) body: Option<SharedString>,
    detail: Option<SharedString>,
    pub(super) actions: SmallVec<[AnyElement; 2]>,
}

impl Words {
    pub(super) fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: Some(title.into()),
            body: None,
            detail: None,
            actions: SmallVec::new(),
        }
    }

    /// 一个字都没有：标题、说明与动作都由调用方逐个补上。
    pub(super) fn blank() -> Self {
        Self {
            title: None,
            body: None,
            detail: None,
            actions: SmallVec::new(),
        }
    }
}

/// `block`, centered and no wider than prose; a flex row sets its width before its text wraps.
fn measure(block: Div, cx: &App) -> Div {
    div()
        .w_full()
        .flex()
        .justify_center()
        .child(block.flex_1().min_w_0().max_w(cx.theme().prose_width()))
}

/// A head, a title, a line, a detail, actions and a tail, centered; it rises in once.
///
/// 每个槽位都是可选的：一个什么都没给的形态是一块空盒子，而不是坏掉的状态。
pub(super) fn state_view(
    id: ElementId,
    head: Option<AnyElement>,
    words: Words,
    tail: Option<AnyElement>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    let Words {
        title,
        body,
        detail,
        actions,
    } = words;
    let titled = title.is_some();
    // 一个字都没有的形态（例如只有内容区）不该留一块空文字占位，否则它还会吃掉一个 gap。
    let words_block = (title.is_some() || body.is_some()).then(|| {
        measure(
            div()
                .text_center()
                .when_some(title, |text, title| {
                    text.child(
                        div()
                            .debug_selector(|| "state-title".into())
                            .text_size(theme.text_size(TextSize::Lg))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.fg)
                            .child(title),
                    )
                })
                .when_some(body, |text, body| {
                    text.child(
                        div()
                            .when(titled, |line| line.mt_1())
                            .text_size(theme.text_size(TextSize::Base))
                            .text_color(colors.fg_muted)
                            .child(body),
                    )
                }),
            cx,
        )
    });

    div()
        .debug_selector(|| "state-root".into())
        .flex()
        .flex_col()
        .items_center()
        .gap_4()
        .px_6()
        .py_8()
        .when_some(head, |state, head| state.child(head))
        .when_some(words_block, |state, words| state.child(words))
        .when_some(detail, |state, detail| {
            state.child(measure(
                div()
                    .px_3()
                    .py_2()
                    .rounded(theme.radius(Radius::Md))
                    .bg(colors.sunken)
                    .font_family(theme.mono_family.clone())
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_muted)
                    .child(detail),
                cx,
            ))
        })
        .when(!actions.is_empty(), |state| {
            state.child(div().flex().gap_2().children(actions))
        })
        .when_some(tail, |state, tail| state.child(tail))
        .with_animation(
            id,
            Animation::new(motion::duration(motion::SLOW, cx)).with_easing(motion::ease_out_cubic),
            |state, t| state.opacity(t).mt(motion::NUDGE * 2.0 * (1.0 - t)),
        )
}

/// An icon on a soft disc, the head of an empty or broken view.
pub(super) fn disc(icon: IconName, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(theme.icon_size(IconSize::Xxl) + theme.icon_size(IconSize::Md))
        .rounded_full()
        .bg(theme.colors.sunken)
        .child(
            Icon::new(icon)
                .size(IconSize::Lg)
                .color(theme.colors.fg_subtle),
        )
        .into_any_element()
}

macro_rules! words_builders {
    () => {
        pub fn body(mut self, text: impl Into<SharedString>) -> Self {
            self.words.body = Some(text.into());
            self
        }

        pub fn action(mut self, action: impl IntoElement) -> Self {
            self.words.actions.push(action.into_any_element());
            self
        }
    };
}
pub(super) use words_builders;

/// 空状态的头槽位：库内图标画在软底圆盘上，或者调用方自绘的元素（不套圆盘）。
enum EmptyHead {
    Disc(IconName),
    Drawn(AnyElement),
}

/// What a view shows with nothing in it: an icon on a soft disc, a title, a line of help and actions. It rises in when it appears.
///
/// 槽位都可选。[`EmptyState::new`] 是常见形态（库内图标 + 标题）；[`EmptyState::slots`]
/// 是组合形态：头、标题、说明、内容区逐个给，没给的不画。上游 `gpui-component` 的
/// `Empty`/`EmptyHeader`/`EmptyMedia`/`EmptyTitle`/`EmptyDescription`/`EmptyContent`
/// 一族对着这张表落：`EmptyMedia` → [`EmptyState::media_element`]、`EmptyTitle` →
/// [`EmptyState::title`]、`EmptyDescription` → [`EmptyState::body`]、
/// `EmptyContent` → [`EmptyState::content`]。
#[derive(IntoElement)]
pub struct EmptyState {
    id: ElementId,
    head: Option<EmptyHead>,
    words: Words,
    content: Option<AnyElement>,
}

impl EmptyState {
    pub fn new(id: impl Into<ElementId>, icon: IconName, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            head: Some(EmptyHead::Disc(icon)),
            words: Words::new(title),
            content: None,
        }
    }

    /// 槽位式构造：头、标题、说明、内容区都由调用方给，没给的不画（宿主 `Empty::new()` 的形态）。
    pub fn slots(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            head: None,
            words: Words::blank(),
            content: None,
        }
    }

    /// 标题槽位；`slots` 之后补标题，`new` 已经给过就换成新的。
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.words.title = Some(title.into());
        self
    }

    /// 头槽位换成调用方自绘的元素，不再套库内图标的软底圆盘；`new` 给的 `icon` 就此让位。
    pub fn media_element(mut self, media: impl IntoElement) -> Self {
        self.head = Some(EmptyHead::Drawn(media.into_any_element()));
        self
    }

    /// 说明与动作之下的任意内容区（宿主 `EmptyContent`）：一个按钮、一个输入框、一整块版面都行。
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    words_builders!();
}

impl RenderOnce for EmptyState {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let head = self.head.map(|head| match head {
            EmptyHead::Disc(icon) => disc(icon, cx),
            EmptyHead::Drawn(element) => element,
        });
        state_view(self.id, head, self.words, self.content, cx)
    }
}

enum Head {
    Code(SharedString),
    Icon(IconName),
}

/// A view that could not show what was asked: a status code or an icon, what happened, and a way on.
#[derive(IntoElement)]
pub struct ErrorView {
    id: ElementId,
    head: Head,
    words: Words,
}

impl ErrorView {
    pub fn new(id: impl Into<ElementId>, icon: IconName, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            head: Head::Icon(icon),
            words: Words::new(title),
        }
    }

    /// Headed by a status code, such as 404, instead of an icon.
    pub fn with_code(
        id: impl Into<ElementId>,
        code: impl Into<SharedString>,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            head: Head::Code(code.into()),
            words: Words::new(title),
        }
    }

    pub fn not_found(id: impl Into<ElementId>) -> Self {
        Self::with_code(id, "404", "Page not found")
            .body("The link may be broken, or the page has moved.")
    }

    pub fn no_permission(id: impl Into<ElementId>) -> Self {
        Self::with_code(id, "403", "You don't have access")
            .body("Ask an owner to share it with you.")
    }

    pub fn maintenance(id: impl Into<ElementId>) -> Self {
        Self::new(id, IconName::Wrench, "Down for maintenance").body("We'll be back shortly.")
    }

    pub fn offline(id: impl Into<ElementId>) -> Self {
        Self::new(id, IconName::WifiOff, "You're offline")
            .body("Check your connection. Your work stays on this device.")
    }

    /// The error itself, in a quiet mono box.
    pub fn detail(mut self, text: impl Into<SharedString>) -> Self {
        self.words.detail = Some(text.into());
        self
    }

    words_builders!();
}

impl RenderOnce for ErrorView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let head = match self.head {
            Head::Icon(icon) => disc(icon, cx),
            Head::Code(code) => {
                let theme = cx.theme();
                tabular(div())
                    .text_size(theme.text_size(TextSize::Display))
                    .font_weight(FontWeight::LIGHT)
                    .text_color(theme.colors.fg_subtle)
                    .child(code)
                    .into_any_element()
            }
        };
        state_view(self.id, Some(head), self.words, None, cx)
    }
}

/// Shows its content, or an `ErrorView` of the error it holds, logged once per error. It never catches panics.
#[derive(IntoElement)]
pub struct ErrorBoundary {
    id: ElementId,
    content: anyhow::Result<AnyElement>,
    on_retry: Option<Run>,
}

impl ErrorBoundary {
    pub fn new<E: IntoElement>(id: impl Into<ElementId>, content: anyhow::Result<E>) -> Self {
        Self {
            id: id.into(),
            content: content.map(IntoElement::into_any_element),
            on_retry: None,
        }
    }

    pub fn on_retry(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_retry = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ErrorBoundary {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let error = match self.content {
            Ok(content) => return content,
            Err(error) => error,
        };
        let chain = format!("{error:#}");
        let logged = window.use_keyed_state((self.id.clone(), "logged"), cx, |_, _| None::<String>);
        if logged.read(cx).as_deref() != Some(chain.as_str()) {
            log::error!("error boundary {:?}: {chain}", self.id);
            logged.update(cx, |logged, _| *logged = Some(chain.clone()));
        }
        let retry = (self.id.clone(), "retry");
        ErrorView::new(self.id, IconName::CircleAlert, "Something went wrong")
            .body(error.to_string())
            .detail(chain)
            .when_some(self.on_retry, |view, run| {
                view.action(
                    Button::new(retry)
                        .label("Try again")
                        .on_click(move |_, window, cx| run(window, cx)),
                )
            })
            .into_any_element()
    }
}
