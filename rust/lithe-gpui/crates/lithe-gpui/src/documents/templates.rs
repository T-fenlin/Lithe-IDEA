use std::rc::Rc;

use gpui::{
    App, AppContext as _, Context, ElementId, Entity, FontWeight, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    Window, div, prelude::*,
};

use super::render::MarkdownRenderer;
use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Choice, ChoiceChips, Input, Pick, TextInput},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// Every category at once.
const ALL: &str = "All";

/// A page to start from: its name, icon, category, a line about it, and its text as markdown.
#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub name: SharedString,
    pub icon: SharedString,
    pub category: SharedString,
    pub description: SharedString,
    pub markdown: SharedString,
}

/// What the picker holds: its search, the category, and the template in view.
struct Picking {
    search: Entity<TextInput>,
    category: SharedString,
    highlighted: usize,
}

/// The templates `query` and `category` keep, by index, in order.
pub(crate) fn kept(templates: &[Template], query: &str, category: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    templates
        .iter()
        .enumerate()
        .filter(|(_, template)| category == ALL || template.category == category)
        .filter(|(_, template)| {
            query.is_empty()
                || template.name.to_lowercase().contains(&query)
                || template.description.to_lowercase().contains(&query)
        })
        .map(|(ix, _)| ix)
        .collect()
}

/// Pages to start from: found by name, kept by category, the one in view shown as it will read; Use takes it.
#[derive(IntoElement)]
pub struct TemplatePicker {
    id: ElementId,
    templates: Rc<Vec<Template>>,
    on_use: Option<Pick>,
}

impl TemplatePicker {
    pub fn new(id: impl Into<ElementId>, templates: impl Into<Rc<Vec<Template>>>) -> Self {
        let templates = templates.into();
        assert!(!templates.is_empty(), "a template picker has templates");
        Self {
            id: id.into(),
            templates,
            on_use: None,
        }
    }

    /// Gets the index of the template taken.
    pub fn on_use(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_use = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TemplatePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let picking = window.use_keyed_state(
            (self.id.clone(), "picking"),
            cx,
            |window, cx: &mut Context<Picking>| Picking {
                search: cx.new(|cx| TextInput::new(window, cx).placeholder("Find a template")),
                category: ALL.into(),
                highlighted: 0,
            },
        );
        let (search, category, highlighted) = {
            let picking = picking.read(cx);
            (
                picking.search.clone(),
                picking.category.clone(),
                picking.highlighted,
            )
        };
        let query = search.read(cx).text().to_string();
        let shown = kept(&self.templates, &query, &category);
        let current = shown
            .iter()
            .copied()
            .find(|ix| *ix == highlighted)
            .or(shown.first().copied());
        let mut categories: Vec<SharedString> = vec![ALL.into()];
        for template in self.templates.iter() {
            if !categories.contains(&template.category) {
                categories.push(template.category.clone());
            }
        }
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let chips = {
            let picking = picking.clone();
            ChoiceChips::new(
                (self.id.clone(), "categories"),
                categories
                    .iter()
                    .map(|name| Choice::new(name.clone(), name.clone())),
            )
            .selected([category.clone()])
            .on_change(move |values, _, cx| {
                let chosen = values.first().cloned().unwrap_or_else(|| ALL.into());
                log::info!("template picker: {chosen}");
                picking.update(cx, |picking, cx| {
                    picking.category = chosen;
                    cx.notify();
                });
            })
        };
        let rows = shown.iter().map(|&ix| {
            let template = &self.templates[ix];
            let lit = current == Some(ix);
            let pick = picking.clone();
            div()
                .id((self.id.clone(), format!("template-{ix}")))
                .flex()
                .gap_2()
                .p_2()
                .rounded(theme.radius(Radius::Md))
                .cursor_pointer()
                .when(lit, |row| row.bg(colors.active))
                .when(!lit, |row| row.hover(|row| row.bg(colors.hover)))
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .on_click(move |_, _, cx| {
                    pick.update(cx, |picking, cx| {
                        picking.highlighted = ix;
                        cx.notify();
                    })
                })
                .child(template.icon.clone())
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .child(template.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(colors.fg_muted)
                                .child(template.description.clone()),
                        ),
                )
        });
        let preview = match current {
            Some(ix) => {
                let take = self.on_use.clone();
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_4()
                            .py_2()
                            .border_b_1()
                            .border_color(colors.border)
                            .child(
                                div()
                                    .text_color(colors.fg_muted)
                                    .child(self.templates[ix].name.clone()),
                            )
                            .child(
                                Button::new((self.id.clone(), "use"), "Use template")
                                    .variant(ButtonVariant::Primary)
                                    .size(ControlSize::Sm)
                                    .disabled(take.is_none())
                                    .when_some(take, |button, take| {
                                        button.on_click(move |_, window, cx| {
                                            log::info!("template picker: use {ix}");
                                            take(ix, window, cx)
                                        })
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .id((self.id.clone(), "preview"))
                            .flex_1()
                            .overflow_y_scroll()
                            .p_4()
                            .child(MarkdownRenderer::new(
                                (self.id.clone(), format!("preview-{ix}")),
                                self.templates[ix].markdown.clone(),
                            )),
                    )
                    .into_any_element()
            }
            None => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(colors.fg_subtle)
                .child("No template matches")
                .into_any_element(),
        };
        div()
            .flex()
            .size_full()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .overflow_hidden()
            .child(
                div()
                    .flex_none()
                    .w(theme.sidebar_width(false))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .border_r_1()
                    .border_color(colors.border)
                    .child(
                        Input::new(&search).size(ControlSize::Sm).prefix(
                            Icon::new(IconName::Search)
                                .size(IconSize::Sm)
                                .color(colors.fg_subtle),
                        ),
                    )
                    .child(chips)
                    .child(
                        div()
                            .id((self.id.clone(), "list"))
                            .flex_1()
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .children(rows),
                    ),
            )
            .child(preview)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(name: &str, category: &str, description: &str) -> Template {
        Template {
            name: name.to_string().into(),
            icon: "📄".into(),
            category: category.to_string().into(),
            description: description.to_string().into(),
            markdown: "".into(),
        }
    }

    #[test]
    fn templates_keep_by_category_and_query() {
        let all = [
            template("Meeting notes", "Team", "Agenda and actions"),
            template("Roadmap", "Product", "Quarters ahead"),
            template("Retro", "Team", "What went well"),
        ];
        assert_eq!(kept(&all, "", ALL), [0, 1, 2]);
        assert_eq!(kept(&all, "", "Team"), [0, 2]);
        assert_eq!(kept(&all, "went", ALL), [2], "a description counts");
        assert!(kept(&all, "zzz", ALL).is_empty());
    }
}
