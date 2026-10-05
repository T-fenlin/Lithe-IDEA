use gpui::{AnyElement, App, ElementId, IntoElement, ParentElement, Styled, div};
use lithe_gpui::{
    buttons::{Button, ButtonGroup, ButtonVariant, IconButton},
    primitives::IconName,
    theme::Size,
};

use crate::ui::{code, row, section, specimen, specimens};

const VARIANTS: [(ButtonVariant, &str); 7] = [
    (ButtonVariant::Primary, "Primary"),
    (ButtonVariant::Secondary, "Secondary"),
    (ButtonVariant::Outline, "Outline"),
    (ButtonVariant::Ghost, "Ghost"),
    (ButtonVariant::Subtle, "Subtle"),
    (ButtonVariant::Danger, "Danger"),
    (ButtonVariant::Link, "Link"),
];

pub fn basics(cx: &App) -> AnyElement {
    let sizes = [
        (Size::Small, "Small"),
        (Size::Medium, "Medium"),
        (Size::Large, "Large"),
    ];

    div()
        .child(
            section("Button", "Primary is ink. Color is saved for meaning.", cx)
                .child(row().children(VARIANTS.map(|(variant, name)| {
                    Button::new(name)
                        .label(name)
                        .variant(variant)
                        .on_click(move |_, _, _| log::info!("gallery: {name} clicked"))
                })))
                .child(row().children(VARIANTS.map(|(variant, name)| {
                    Button::new(ElementId::Name(format!("{name}-off").into()))
                        .label(name)
                        .variant(variant)
                        .disabled(true)
                }))),
        )
        .child(
            section(
                "Sizes",
                "Heights follow density. Tab moves the focus ring.",
                cx,
            )
            .child(row().children(sizes.map(|(size, name)| {
                Button::new(ElementId::Name(format!("size-{name}").into()))
                    .label(name)
                    .primary()
                    .size(size)
            }))),
        )
        .child(
            section(
                "With icons",
                "Leading icon for verbs. Trailing for menus.",
                cx,
            )
            .child(
                row()
                    .child(
                        Button::new("new")
                            .label("New file")
                            .primary()
                            .icon(IconName::Plus),
                    )
                    .child(Button::new("share").label("Share").icon(IconName::Share2))
                    .child(
                        Button::new("sort")
                            .label("Sort")
                            .variant(ButtonVariant::Ghost)
                            .trailing_icon(IconName::ChevronDown),
                    )
                    .child(
                        Button::new("delete")
                            .label("Delete")
                            .variant(ButtonVariant::Danger)
                            .icon(IconName::Trash2),
                    ),
            ),
        )
        .child(
            section("Icon button", "Square. Ghost by default.", cx).child(specimens().children(
                sizes.map(|(size, name)| {
                    specimen(
                        name,
                        row()
                            .gap_1()
                            .child(
                                IconButton::new(
                                    ElementId::Name(format!("{name}-a").into()),
                                    IconName::Bold,
                                )
                                .size(size),
                            )
                            .child(
                                IconButton::new(
                                    ElementId::Name(format!("{name}-b").into()),
                                    IconName::Italic,
                                )
                                .size(size),
                            )
                            .child(
                                IconButton::new(
                                    ElementId::Name(format!("{name}-c").into()),
                                    IconName::Underline,
                                )
                                .size(size)
                                .variant(ButtonVariant::Subtle),
                            )
                            .child(
                                IconButton::new(
                                    ElementId::Name(format!("{name}-d").into()),
                                    IconName::Link,
                                )
                                .size(size)
                                .variant(ButtonVariant::Outline),
                            ),
                        cx,
                    )
                }),
            )),
        )
        .child(
            section(
                "ShortcutHint",
                "The keystroke rides along, quiet, in the platform's spelling.",
                cx,
            )
            .child(
                row()
                    .child(Button::new("hint-save").label("Save").shortcut("cmd-s"))
                    .child(
                        Button::new("hint-find")
                            .label("Find")
                            .variant(ButtonVariant::Ghost)
                            .icon(IconName::Search)
                            .shortcut("cmd-shift-f"),
                    ),
            )
            .child(code("Button::new(id).label(label).shortcut(\"cmd-s\")", cx)),
        )
        .child(
            section("ButtonGroup", "Buttons joined edge to edge.", cx).child(
                row()
                    .child(
                        ButtonGroup::new()
                            .button(Button::new("group-day").label("Day"))
                            .button(Button::new("group-week").label("Week"))
                            .button(Button::new("group-month").label("Month")),
                    )
                    .child(
                        ButtonGroup::new()
                            .button(
                                Button::new("group-prev")
                                    .label("Previous")
                                    .variant(ButtonVariant::Outline)
                                    .icon(IconName::ChevronLeft),
                            )
                            .button(
                                Button::new("group-next")
                                    .label("Next")
                                    .variant(ButtonVariant::Outline)
                                    .trailing_icon(IconName::ChevronRight),
                            ),
                    ),
            ),
        )
        .into_any_element()
}
