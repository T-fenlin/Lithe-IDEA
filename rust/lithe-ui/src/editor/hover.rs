use std::{ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, HighlightStyle, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, StyledText, Window, div,
};

use super::{state::CodeEditor, syntax};
use crate::{
    primitives::{Icon, Severity},
    theme::{ActiveTheme, Elevation, IconSize, Palette, Radius, TextSize},
};

type OnPress = Rc<dyn Fn(&mut Window, &mut App)>;

/// What a hover over a symbol reads: its signature in code, a problem when there is one, its documentation, and links.
///
/// The library only **draws** it: the owner fetches the content and pushes it with [`CodeEditor::present_hover`].
/// `Clone` is there so the editor can redraw the same content every frame (link handlers are shared through
/// `Rc`: a copy is a handle, not state).
#[derive(IntoElement, Clone)]
pub struct HoverInfo {
    id: ElementId,
    signature: SharedString,
    problem: Option<(Severity, SharedString)>,
    docs: Vec<SharedString>,
    links: Vec<(SharedString, OnPress)>,
}

impl HoverInfo {
    pub fn new(id: impl Into<ElementId>, signature: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            signature: signature.into(),
            problem: None,
            docs: Vec::new(),
            links: Vec::new(),
        }
    }

    pub fn problem(mut self, severity: Severity, message: impl Into<SharedString>) -> Self {
        self.problem = Some((severity, message.into()));
        self
    }

    /// A paragraph of documentation.
    pub fn docs(mut self, paragraph: impl Into<SharedString>) -> Self {
        self.docs.push(paragraph.into());
        self
    }

    pub fn link(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.links.push((label.into(), Rc::new(handler)));
        self
    }
}

/// Where the editor draws hover content.
impl CodeEditor {
    /// The hover content the owner pushed, drawn under the character it hangs from; nothing is drawn while the
    /// anchor is not in the viewport, or the frame has not been laid out.
    ///
    /// Draw only, never fetch: the content arrives through [`CodeEditor::present_hover`], and the anchor is the
    /// symbol the last [`super::EditorEvent::HoverRequested`] named. The rule matches
    /// [`CodeEditor::bounds_for_offset`] exactly, so an owner placing its own popup lands in the same place;
    /// taking the content down is the owner's business, the library never withdraws it on its own.
    pub(crate) fn hover_panel(&self, cx: &App) -> Option<AnyElement> {
        let hover = self.hover.as_ref()?;
        let bounds = self.bounds_for_offset(hover.offset, cx)?;
        let body = self.metrics.body;
        Some(
            div()
                .absolute()
                .left(bounds.origin.x - body.x)
                .top(bounds.origin.y - body.y + bounds.size.height)
                .child(hover.info.clone())
                .into_any_element(),
        )
    }
}

/// Text with its `code` spans unquoted, and where they lie; an unpaired backtick stays as typed.
pub(crate) fn code_spans(text: &str) -> (String, Vec<Range<usize>>) {
    if text.matches('`').count() % 2 == 1 {
        return (text.to_string(), Vec::new());
    }
    let (mut plain, mut spans) = (String::new(), Vec::new());
    for (ix, part) in text.split('`').enumerate() {
        let start = plain.len();
        plain.push_str(part);
        if ix % 2 == 1 {
            spans.push(start..plain.len());
        }
    }
    (plain, spans)
}

fn prose(text: &str, colors: &Palette) -> StyledText {
    let (plain, spans) = code_spans(text);
    let code = HighlightStyle {
        color: Some(colors.fg),
        background_color: Some(colors.sunken),
        ..Default::default()
    };
    StyledText::new(plain).with_highlights(spans.into_iter().map(|span| (span, code)))
}

impl RenderOnce for HoverInfo {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let styles = syntax::colors(&self.signature, cx);
        let rule = || div().h_0().border_t_1().border_color(colors.border);
        let has_links = !self.links.is_empty();
        let links = self
            .links
            .into_iter()
            .enumerate()
            .map(|(ix, (label, handler))| {
                div()
                    .id((self.id.clone(), format!("link-{ix}")))
                    .text_color(colors.link)
                    .cursor_pointer()
                    .hover(|link| link.underline())
                    .on_click(move |_, window, cx| handler(window, cx))
                    .child(label)
            });
        div()
            .max_w(theme.label_width() * 5.0)
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.overlay)
            .shadow(theme.elevation(Elevation::Floating))
            .text_size(theme.text_size(TextSize::Sm))
            .children(self.problem.map(|(severity, message)| {
                div()
                    .flex()
                    .items_start()
                    .gap_1p5()
                    .child(
                        Icon::new(severity.icon())
                            .size(IconSize::Sm)
                            .color(severity.color(&colors)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(colors.fg)
                            .child(prose(&message, &colors)),
                    )
            }))
            .child(
                div()
                    .font_family(theme.mono_family.clone())
                    .text_color(colors.syntax.variable)
                    .child(StyledText::new(self.signature.clone()).with_highlights(styles)),
            )
            .children((!self.docs.is_empty()).then(rule))
            .children(self.docs.into_iter().map(|paragraph| {
                div()
                    .text_color(colors.fg_muted)
                    .child(prose(&paragraph, &colors))
            }))
            .children(has_links.then(rule))
            .child(div().flex().gap_3().children(links))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backticks_mark_code_unless_unpaired() {
        let plain = "The color for `name`, lifted by `lift`.";
        assert_eq!(
            code_spans(plain),
            (
                "The color for name, lifted by lift.".to_string(),
                vec![14..18, 30..34]
            )
        );
        assert_eq!(code_spans("a `b"), ("a `b".to_string(), Vec::new()));
    }
}
