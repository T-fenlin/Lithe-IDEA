//! One project row: its caret, its icon, its name, and its git letter.

use std::{
    collections::{BTreeMap, HashSet},
    ops::Range,
    rc::Rc,
};

use gpui::{
    AnyElement, App, ElementId, Entity, FontWeight, Hsla, ImageSource, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Rems, Resource, SharedString,
    StatefulInteractiveElement, Styled, Window, div, img, prelude::FluentBuilder as _, relative,
    rems, svg,
};

use super::{
    IconFor, ProjectAsset, ProjectIcon, ProjectIconRequest, ProjectTree,
    node::{Arena, ProjectRowKind, Row},
    select::ProjectSelection,
};
use crate::{
    data_display::Tone,
    lists::GitStatus,
    theme::{ActiveTheme, IconSize, Palette, Radius, Size, TextSize},
};

/// The row indentation the IDE's project tree draws with: 8 plus 14 a level
/// (`ProjectSidebarView.swift:558`), which the library's own tree indent does not share.
const INDENT_BASE: f32 = 8.;
const INDENT_STEP: f32 = 14.;
/// The row's own line height, as the sidebar has always drawn it.
const LINE_HEIGHT: f32 = 1.35;
/// The git letter's size, a touch under the row's text.
const LETTER_SIZE: f32 = 9.;
/// The caret's slot, so icons line up whether or not a row can open.
const CARET_SLOT: f32 = 16.;

/// What drawing rows needs, shared by every range the list asks for.
pub(super) struct RowData {
    /// The tree element's own id, which every row id hangs off.
    pub id: ElementId,
    pub arena: Rc<Arena>,
    pub rows: Rc<Vec<Row>>,
    pub expanded: Rc<HashSet<SharedString>>,
    /// Git state by file path.
    pub status: Rc<BTreeMap<SharedString, GitStatus>>,
    /// The most urgent state under each folder, and under the workspace root (`""`).
    pub groups: Rc<BTreeMap<SharedString, GitStatus>>,
    pub selection: Rc<ProjectSelection>,
    pub active: Option<SharedString>,
    /// The shut and open caret assets the owner supplied.
    pub caret: Option<(ProjectAsset, ProjectAsset)>,
    /// The note an open, empty folder shows.
    pub empty_folder: SharedString,
    pub icon_for: Option<IconFor>,
    pub tree: Entity<ProjectTree>,
}

pub(super) fn draw(
    data: &RowData,
    range: Range<usize>,
    window: &mut Window,
    cx: &mut App,
) -> Vec<AnyElement> {
    range.map(|ix| row(data, ix, window, cx)).collect()
}

/// The color a git state inks its row with: the palette's own color, not a badge's fill-and-text pair.
pub(super) fn ink(tone: Tone, colors: &Palette) -> Hsla {
    match tone {
        Tone::Neutral => colors.fg_muted,
        Tone::Accent => colors.accent,
        Tone::Info => colors.info,
        Tone::Success => colors.success,
        Tone::Warning => colors.warning,
        Tone::Danger => colors.danger,
    }
}

/// A drawing the app supplies by asset path: the image itself when the app really has it, else the
/// same path as a glyph tinted with `color`. That is how the sidebar has always drawn its carets,
/// and it keeps a row drawable where no asset source is installed at all.
fn app_icon(path: &SharedString, side: Rems, color: Hsla, cx: &App) -> AnyElement {
    if matches!(cx.asset_source().load(path), Ok(Some(_))) {
        return img(ImageSource::Resource(Resource::Embedded(path.clone())))
            .flex_shrink_0()
            .w(side)
            .h(side)
            .into_any_element();
    }
    svg()
        .path(path.clone())
        .flex_shrink_0()
        .w(side)
        .h(side)
        .text_color(color)
        .into_any_element()
}

/// A glyph the tree tints: the Lucide fallback the owner asked for.
fn glyph(path: &SharedString, side: Rems, color: Hsla) -> AnyElement {
    svg()
        .path(path.clone())
        .flex_shrink_0()
        .w(side)
        .h(side)
        .text_color(color)
        .into_any_element()
}

fn row(data: &RowData, ix: usize, _window: &mut Window, cx: &mut App) -> AnyElement {
    let placed = &data.rows[ix];
    let arena = &data.arena;
    let entry = arena.entry(placed.node);
    let kind = placed.kind(arena);
    let key = placed.key(arena);
    let expanded = data.expanded.contains(&key);
    let placeholder = kind == ProjectRowKind::EmptyFolder;
    let selected = data.selection.covers(entry.path.as_ref());
    let active = data.active.as_deref() == Some(entry.path.as_ref());
    let status = match kind {
        ProjectRowKind::File => data.status.get(&entry.path).copied(),
        ProjectRowKind::Directory => data.groups.get(&entry.path).copied(),
        ProjectRowKind::EmptyFolder => None,
    };
    let theme = cx.theme();
    let colors = &theme.colors;
    let row_height = theme.control_height(Size::Small);
    let icon_side = theme.icon_size(IconSize::Md);

    // The caret opens the folder; the indent keeps its slot on every other row.
    let caret_slot = div()
        .flex_none()
        .w(rems(CARET_SLOT / 16.))
        .mr(rems(2. / 16.));
    let caret: AnyElement = match (entry.is_dir && !placeholder, data.caret) {
        (true, Some((shut, open))) => {
            let path: SharedString = match expanded {
                true => open.path(theme.is_dark()),
                false => shut.path(theme.is_dark()),
            }
            .into();
            caret_slot
                .flex()
                .justify_center()
                .child(app_icon(
                    &path,
                    theme.icon_size(IconSize::Xs),
                    colors.fg,
                    cx,
                ))
                .into_any_element()
        }
        _ => caret_slot.into_any_element(),
    };
    let icon: AnyElement = match data.icon_for.as_ref().and_then(|find| {
        find(
            &ProjectIconRequest {
                kind,
                expanded,
                path: entry.path.clone(),
                name: entry.name.clone(),
            },
            cx,
        )
    }) {
        Some(ProjectIcon::Glyph(path)) => glyph(&path, icon_side, colors.fg),
        Some(ProjectIcon::Image(path)) => img(ImageSource::Resource(Resource::Embedded(path)))
            .flex_shrink_0()
            .w(icon_side)
            .h(icon_side)
            .into_any_element(),
        None => div().flex_none().w(icon_side).into_any_element(),
    };
    let text = match placeholder {
        true => data.empty_folder.clone(),
        false => entry.name.clone(),
    };
    let mut label = div().flex_1().min_w_0().truncate().child(text);
    if arena.is_root(placed.node) {
        label = label.font_weight(FontWeight::SEMIBOLD);
    }
    if placeholder {
        label = label.text_color(colors.fg_subtle);
    } else if let Some(status) = status {
        label = label.text_color(ink(status.tone(), colors));
    }
    let letter = status
        .filter(|_| kind == ProjectRowKind::File)
        .map(|status| {
            div()
                .flex_shrink_0()
                .text_size(rems(LETTER_SIZE / 16.))
                .font_weight(FontWeight::BOLD)
                .font_family(theme.mono_family.clone())
                .text_color(ink(status.tone(), colors))
                .child(status.letter())
        });
    let tree = data.tree.clone();
    let caret_tree = data.tree.clone();
    let menu_tree = data.tree.clone();
    let toggled = key.clone();
    let clicked = key.clone();
    let right_clicked = key.clone();
    div()
        .id((data.id.clone(), format!("row-{key}")))
        .w_full()
        .h(row_height)
        .flex()
        .items_center()
        .gap_1()
        .pl(rems(
            (INDENT_BASE + placed.depth as f32 * INDENT_STEP) / 16.,
        ))
        .pr(rems(6. / 16.))
        .rounded(theme.radius(Radius::Sm))
        .text_size(theme.text_size(TextSize::Md))
        .line_height(relative(LINE_HEIGHT))
        .whitespace_nowrap()
        .overflow_hidden()
        .when(selected || active, |row| row.bg(colors.active))
        .when(!(selected || active) && !placeholder, |row| {
            row.hover(|style| style.bg(colors.hover))
        })
        .child(
            div()
                .flex_none()
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    cx.stop_propagation();
                    let _ = caret_tree.update(cx, |tree, cx| tree.toggle_key(&toggled, cx));
                })
                .child(caret),
        )
        .child(icon)
        .child(label)
        .children(letter)
        .on_click(move |event, window, cx| {
            let modifiers = event.modifiers();
            let (extending, toggling) = (modifiers.shift, modifiers.secondary());
            let asked = tree.update(cx, |tree, cx| {
                tree.click_key(&clicked, extending, toggling, window, cx)
            });
            if let Some((action, path)) = asked {
                let handler = tree.read(cx).on_action.clone();
                if let Some(handler) = handler {
                    handler(action, path.as_ref(), window, cx);
                }
            }
        })
        .on_mouse_down(MouseButton::Right, move |event, _, cx| {
            let at = event.position;
            let key = right_clicked.clone();
            let _ = menu_tree.update(cx, |tree, cx| tree.context_key(&key, at, cx));
        })
        .into_any_element()
}
