use gpui::{AnyElement, App, ElementId, IntoElement, ParentElement, Styled, div};
use lithe_gpui::{
    buttons::{Button, ButtonGroup, ButtonVariant},
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
                            .label(crate::i18n::gallery("New file"))
                            .primary()
                            .icon(IconName::Plus),
                    )
                    .child(
                        Button::new("share")
                            .label(crate::i18n::gallery("Share"))
                            .icon(IconName::Share2),
                    )
                    .child(
                        Button::new("sort")
                            .label(crate::i18n::gallery("Sort"))
                            .variant(ButtonVariant::Ghost)
                            .trailing_icon(IconName::ChevronDown),
                    )
                    .child(
                        Button::new("delete")
                            .label(crate::i18n::gallery("Delete"))
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
                                Button::new(ElementId::Name(format!("{name}-a").into()))
                                    .icon(IconName::Bold)
                                    .size(size),
                            )
                            .child(
                                Button::new(ElementId::Name(format!("{name}-b").into()))
                                    .icon(IconName::Italic)
                                    .size(size),
                            )
                            .child(
                                Button::new(ElementId::Name(format!("{name}-c").into()))
                                    .icon(IconName::Underline)
                                    .size(size)
                                    .variant(ButtonVariant::Subtle),
                            )
                            .child(
                                Button::new(ElementId::Name(format!("{name}-d").into()))
                                    .icon(IconName::Link)
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
                    .child(
                        Button::new("hint-save")
                            .label(crate::i18n::gallery("Save"))
                            .shortcut("cmd-s"),
                    )
                    .child(
                        Button::new("hint-find")
                            .label(crate::i18n::gallery("Find"))
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
                            .button(Button::new("group-day").label(crate::i18n::gallery("Day")))
                            .button(Button::new("group-week").label(crate::i18n::gallery("Week")))
                            .button(
                                Button::new("group-month").label(crate::i18n::gallery("Month")),
                            ),
                    )
                    .child(
                        ButtonGroup::new()
                            .button(
                                Button::new("group-prev")
                                    .label(crate::i18n::gallery("Previous"))
                                    .variant(ButtonVariant::Outline)
                                    .icon(IconName::ChevronLeft),
                            )
                            .button(
                                Button::new("group-next")
                                    .label(crate::i18n::gallery("Next"))
                                    .variant(ButtonVariant::Outline)
                                    .trailing_icon(IconName::ChevronRight),
                            ),
                    ),
            ),
        )
        .into_any_element()
}
