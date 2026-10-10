mod kinds;
mod launcher;
#[cfg(all(test, feature = "test-support"))]
mod tests;

use std::{ops::Range, rc::Rc};

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, Entity, FontWeight, HighlightStyle,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Point, Role, ScrollHandle,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, StyledText, Window, div,
    prelude::*,
};

use crate::{
    forms::{Down, Enter, InputState, OnValue, Pick, Run, Up},
    motion::{self, Spinner},
    primitives::{Backdrop, FocusScope, Icon, IconName, give_back, take_focus},
    theme::{ActiveTheme, Elevation, IconSize, Radius, Size, TextSize},
};

pub use kinds::{Command, CommandPalette, QuickOpen, QuickSwitcher, SearchPalette};
pub use launcher::QuickLauncher;

/// What a pick does: the row's index under the cursor and the value it carries.
///
/// The index is what a caller that owns the rows needs — it can look the row up
/// again without relying on values being unique.
pub(crate) type OnPick = Rc<dyn Fn(usize, &SharedString, &mut Window, &mut App)>;

/// 只关心值的回调适配成一次拾取（忽略行号）。
pub(crate) fn on_value(run: Option<OnValue>) -> Option<OnPick> {
    run.map(|run| {
        let pick: OnPick = Rc::new(
            move |_: usize, value: &SharedString, window: &mut Window, cx: &mut App| {
                run(value, window, cx)
            },
        );
        pick
    })
}

/// How well a query fits a text, and the byte ranges it covers.
#[derive(Debug, PartialEq)]
pub(crate) struct Fit {
    pub score: i32,
    pub hits: Vec<Range<usize>>,
}

fn fold(c: char) -> char {
    c.to_lowercase()
        .next()
        .expect("a char lowercases to at least one char")
}

/// The query's characters in order within `text`, ignoring case and spaces. Word starts and runs score higher, gaps lower.
pub(crate) fn fuzzy(query: &str, text: &str) -> Option<Fit> {
    let query: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(fold)
        .collect();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut next = 0;
    let end = match query.len() {
        0 => {
            return Some(Fit {
                score: 0,
                hits: Vec::new(),
            });
        }
        len => chars.iter().position(|&(_, c)| {
            next += usize::from(fold(c) == query[next]);
            next == len
        })?,
    };
    // ponytail: the tightest window at the first full match, not the best; score every window if rankings disappoint.
    let mut at = Vec::with_capacity(query.len());
    for ix in (0..=end).rev() {
        if at.len() < query.len() && fold(chars[ix].1) == query[query.len() - 1 - at.len()] {
            at.push(ix);
        }
    }
    at.reverse();
    let starts_word = |ix: usize| {
        ix == 0 || {
            let (before, here) = (chars[ix - 1].1, chars[ix].1);
            !before.is_alphanumeric() || (before.is_lowercase() && here.is_uppercase())
        }
    };
    let mut score = 0;
    let mut hits: Vec<Range<usize>> = Vec::new();
    for (k, &ix) in at.iter().enumerate() {
        let bonus = if starts_word(ix) { 8 } else { 0 };
        score += 16 + if k == 0 { bonus * 2 } else { bonus };
        if k > 0 {
            let gap = (ix - at[k - 1] - 1) as i32;
            score += if gap == 0 { 4 } else { -2 - gap };
        }
        let (from, c) = chars[ix];
        match hits.last_mut() {
            Some(last) if last.end == from => last.end = from + c.len_utf8(),
            _ => hits.push(from..from + c.len_utf8()),
        }
    }
    Some(Fit { score, hits })
}

/// `text` with the letters a query matched drawn in the accent color.
pub(crate) fn marked(text: SharedString, hits: Vec<Range<usize>>, cx: &App) -> AnyElement {
    let mark = HighlightStyle {
        color: Some(cx.theme().colors.accent),
        font_weight: Some(FontWeight::SEMIBOLD),
        ..Default::default()
    };
    StyledText::new(text)
        .with_highlights(hits.into_iter().map(|hit| (hit, mark)))
        .into_any_element()
}

/// One row: the value a pick hands back, then what the row shows.
///
/// The palette kinds below build these from their own inputs. A caller that
/// answers queries itself ([`Palette`]) builds them directly, which is how a
/// row can carry an icon, a two-part label or a trailing note the library
/// knows nothing about.
pub struct PaletteRow {
    /// Handed to `on_pick`.
    pub value: SharedString,
    /// The row's plain name, for assistive technology.
    pub name: SharedString,
    /// A library icon, when the row has one.
    pub icon: Option<IconName>,
    /// 调用方自带的图标，优先于 `icon`（由 `Command` / `Choice` 的 `icon_element` 现取）。
    pub icon_element: Option<AnyElement>,
    /// What the row shows as its name; plain text unless the caller marks it.
    pub label: AnyElement,
    /// A quieter second part, such as a path or a description.
    pub detail: Option<AnyElement>,
    /// A trailing part, such as a key chord or a count.
    pub end: Option<AnyElement>,
}

impl PaletteRow {
    /// A row whose label is its plain name.
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        Self {
            value: value.into(),
            name: label.clone(),
            icon: None,
            icon_element: None,
            label: label.into_any_element(),
            detail: None,
            end: None,
        }
    }

    /// A library icon in the leading slot.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A caller-owned icon in the leading slot; wins over [`PaletteRow::icon`].
    pub fn icon_element(mut self, icon: impl IntoElement) -> Self {
        self.icon_element = Some(icon.into_any_element());
        self
    }

    /// The name the row shows, instead of its plain label.
    pub fn label(mut self, label: impl IntoElement) -> Self {
        self.label = label.into_any_element();
        self
    }

    /// The quieter second part.
    pub fn detail(mut self, detail: impl IntoElement) -> Self {
        self.detail = Some(detail.into_any_element());
        self
    }

    /// The trailing part.
    pub fn end(mut self, end: impl IntoElement) -> Self {
        self.end = Some(end.into_any_element());
        self
    }
}

/// Rows under an optional heading.
pub(crate) struct Group {
    pub title: Option<SharedString>,
    pub rows: Vec<PaletteRow>,
}

/// The row under the keyboard, the query it was set for, and the list's scroll.
#[derive(Default)]
struct Cursor {
    at: usize,
    query: Option<String>,
    scroll: ScrollHandle,
}

/// The query field kept for palette `id`.
pub(crate) fn query_field(
    id: &ElementId,
    placeholder: SharedString,
    window: &mut Window,
    cx: &mut App,
) -> Entity<InputState> {
    window.use_keyed_state((id.clone(), "query"), cx, |window, cx| {
        InputState::new(window, cx).placeholder(placeholder)
    })
}

/// A query field over grouped rows. Arrows move, Enter or a click picks, Escape closes.
///
/// There are two ways to feed it:
///
/// - the [`CommandPalette`] / [`QuickOpen`] / [`QuickSwitcher`] / [`SearchPalette`]
///   kinds, which rank and mark rows themselves from a list the caller hands over;
/// - `Palette` built directly, for a caller that answers each query itself — an
///   async search, say. It takes the caller's own query field, so the caller
///   subscribes to that field's change event, computes the rows for the current
///   query, and gets the picked index back. While `loading` is set the empty text
///   stays hidden, because "nothing yet" is not the same as "no match".
pub struct Palette {
    /// 字段对库内开放；库外只能走 `new` 与各构造口，免得把 `Group` 这类内部形状漏出去。
    pub(crate) id: ElementId,
    pub(crate) input: Entity<InputState>,
    pub(crate) groups: Vec<Group>,
    /// The row marked whenever the query changes.
    pub(crate) start: usize,
    /// Said when no row fits and nothing is in flight.
    pub(crate) empty: AnyElement,
    /// An answer is in flight: the leading mark spins and `empty` stays hidden.
    pub(crate) loading: bool,
    /// 调用方对卡片的样式覆盖（`Styled`）；尺寸、内衬这类规格不该在组件里写死。
    pub(crate) style: StyleRefinement,
    /// 查询框右侧的收尾元素（条数、快捷键提示…）。
    pub(crate) field_trailing: Option<AnyElement>,
    /// 列表之下的收尾行（调用方的按钮条）。
    pub(crate) footer: Option<AnyElement>,
    pub(crate) on_pick: Option<OnPick>,
    pub(crate) on_close: Run,
}

impl Palette {
    /// A palette over `input`, the query field the caller owns and watches.
    ///
    /// `on_close` runs on Escape and after a pick: close the host layer there.
    pub fn new(
        id: impl Into<ElementId>,
        input: Entity<InputState>,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            input,
            groups: Vec::new(),
            start: 0,
            empty: SharedString::default().into_any_element(),
            loading: false,
            style: StyleRefinement::default(),
            field_trailing: None,
            footer: None,
            on_pick: None,
            on_close: Rc::new(on_close),
        }
    }

    /// Rows under a heading. A heading with no rows under it is dropped on render.
    pub fn group(
        mut self,
        title: impl Into<SharedString>,
        rows: impl IntoIterator<Item = PaletteRow>,
    ) -> Self {
        self.groups.push(Group {
            title: Some(title.into()),
            rows: rows.into_iter().collect(),
        });
        self
    }

    /// Rows with no heading.
    pub fn rows(mut self, rows: impl IntoIterator<Item = PaletteRow>) -> Self {
        self.groups.push(Group {
            title: None,
            rows: rows.into_iter().collect(),
        });
        self
    }

    /// The row marked whenever the query changes.
    pub fn start(mut self, at: usize) -> Self {
        self.start = at;
        self
    }

    /// Said when no row fits and nothing is in flight.
    ///
    /// Any element, so a failed search can say so in the danger color while a
    /// plain "nothing matched" stays quiet. Plain text keeps the quiet default.
    pub fn empty(mut self, empty: impl IntoElement) -> Self {
        self.empty = empty.into_any_element();
        self
    }

    /// Whether an answer is in flight.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// An element after the query field, inside the same row — a count, a hint.
    pub fn field_trailing(mut self, trailing: impl IntoElement) -> Self {
        self.field_trailing = Some(trailing.into_any_element());
        self
    }

    /// A row below the list, inside the card — the caller's own actions.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// What a pick does: the row's index and the value it carries.
    ///
    /// It runs **before** `on_close`, so the caller can still read the state the
    /// rows were built from — a list cleared on close is the usual trap.
    pub fn on_pick(
        mut self,
        handler: impl Fn(usize, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }

    /// Over a scrim; a click on it closes. Tab stays inside, and focus returns on every close.
    pub fn overlay(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let takeover = take_focus((self.id.clone(), "takeover"), window, cx);
        let scope = takeover.read(cx).focus.clone();
        if scope.is_focused(window) {
            window.focus(&self.input.read(cx).focus().clone(), cx);
        }
        let close = self.on_close.clone();
        let close: Run = Rc::new(move |window, cx| {
            give_back(&takeover, window, cx);
            close(window, cx)
        });
        self.on_close = close.clone();
        Backdrop::new((self.id.clone(), "scrim"))
            .on_dismiss(move |window, cx| close(window, cx))
            .child(FocusScope::new(&scope).trap().child(self.card(window, cx)))
    }

    /// The card alone, as in a window or a dialog of its own.
    pub fn card(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let cursor =
            window.use_keyed_state((self.id.clone(), "cursor"), cx, |_, _| Cursor::default());
        let values: Vec<SharedString> = self
            .groups
            .iter()
            .flat_map(|group| group.rows.iter().map(|row| row.value.clone()))
            .collect();
        let count = values.len();
        let text = self.input.read(cx).text().to_string();
        if cursor.read(cx).query.as_deref() != Some(text.as_str()) {
            let start = self.start.min(count.saturating_sub(1));
            cursor.update(cx, |cursor, _| {
                cursor.at = start;
                cursor.query = Some(text);
                cursor.scroll.set_offset(Point::default());
            });
        }
        let at = cursor.read(cx).at.min(count.saturating_sub(1));
        let scroll = cursor.read(cx).scroll.clone();
        let (mut slots, mut heads) = (Vec::with_capacity(count), Vec::new());
        let mut slot = 0;
        for group in &self.groups {
            if group.title.is_some() {
                slot += 1;
                heads.push(slots.len());
            }
            slots.extend(slot..slot + group.rows.len());
            slot += group.rows.len();
        }
        let slots = Rc::new((slots, heads));
        let go = {
            let (cursor, slots) = (cursor.clone(), slots.clone());
            move |by: isize, cx: &mut App| {
                if count == 0 {
                    return;
                }
                cursor.update(cx, |cursor, cx| {
                    let at = cursor.at.min(count - 1) as isize;
                    cursor.at = (at + by).rem_euclid(count as isize) as usize;
                    let (slots, heads) = &*slots;
                    let head = heads.contains(&cursor.at) && (by < 0 || cursor.at == 0);
                    cursor
                        .scroll
                        .scroll_to_item(slots[cursor.at] - usize::from(head));
                    cx.notify();
                })
            }
        };
        let pick: Pick = {
            let (id, on_pick, close) = (self.id.clone(), self.on_pick, self.on_close.clone());
            Rc::new(move |ix, window, cx| {
                let value = values[ix].clone();
                log::info!("palette {id:?}: picked {value}");
                // 先交回给调用方、再关闭：调用方常在关闭时清空行，倒过来就查不到这一行了。
                if let Some(on_pick) = &on_pick {
                    on_pick(ix, &value, window, cx);
                }
                close(window, cx);
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let mut children: Vec<AnyElement> = Vec::with_capacity(slot);
        let mut ix = 0;
        for group in self.groups {
            if let Some(title) = group.title {
                children.push(
                    div()
                        .px_2()
                        .pt_3()
                        .pb_1()
                        .text_size(theme.text_size(TextSize::Xs))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(colors.fg_subtle)
                        .child(title)
                        .into_any_element(),
                );
            }
            for row in group.rows {
                let (here, pick, hover) = (ix, pick.clone(), cursor.clone());
                children.push(
                    div()
                        .id(("row", ix))
                        .role(Role::ListBoxOption)
                        .aria_selected(ix == at)
                        .aria_label(row.name.clone())
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap_3()
                        .h(theme.control_height(Size::Large))
                        .px_2()
                        .rounded(theme.radius(Radius::Md))
                        .when(ix == at, |row| row.bg(colors.hover))
                        .cursor_pointer()
                        .on_mouse_move(move |_, _, cx| {
                            if hover.read(cx).at != here {
                                hover.update(cx, |cursor, cx| {
                                    cursor.at = here;
                                    cx.notify();
                                });
                            }
                        })
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                            pick(here, window, cx);
                        })
                        .when_some(
                            // 自有图标元素优先，库内 `IconName` 兜底。
                            row.icon_element.or_else(|| {
                                row.icon.map(|name| {
                                    Icon::new(name)
                                        .size(IconSize::Sm)
                                        .color(colors.fg_muted)
                                        .into_any_element()
                                })
                            }),
                            |line, icon| line.child(icon),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_1()
                                .min_w_0()
                                .items_center()
                                .gap_2()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(div().flex_none().child(row.label))
                                .when_some(row.detail, |line, detail| {
                                    line.child(
                                        div()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .text_color(colors.fg_subtle)
                                            .child(detail),
                                    )
                                }),
                        )
                        .when_some(row.end, |line, end| {
                            line.child(div().flex_none().text_color(colors.fg_subtle).child(end))
                        })
                        .into_any_element(),
                );
                ix += 1;
            }
        }
        let (up, down, escape) = (go.clone(), go, self.on_close);
        let (enter, chosen) = (pick, cursor.clone());
        let mut card = div()
            .id(self.id.clone())
            .debug_selector(|| "palette-card".into())
            .flex()
            .flex_col()
            .w(theme.palette_size().width)
            .h(theme.palette_size().height)
            .rounded(theme.radius(Radius::Xl))
            .bg(colors.overlay)
            .border_1()
            .border_color(colors.border)
            .shadow(theme.elevation(Elevation::Modal))
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(colors.fg)
            .capture_action(move |_: &Up, _, cx| {
                cx.stop_propagation();
                up(-1, cx);
            })
            .capture_action(move |_: &Down, _, cx| {
                cx.stop_propagation();
                down(1, cx);
            })
            .capture_action(move |_: &Enter, window, cx| {
                cx.stop_propagation();
                if count > 0 {
                    enter(chosen.read(cx).at.min(count - 1), window, cx);
                }
            })
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    escape(window, cx);
                }
            })
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(colors.border)
                    .text_size(theme.text_size(TextSize::Md))
                    .child(if self.loading {
                        Spinner::new((self.id.clone(), "loading"))
                            .size(IconSize::Sm)
                            .color(colors.fg_subtle)
                            .into_any_element()
                    } else {
                        Icon::new(IconName::Search)
                            .size(IconSize::Sm)
                            .color(colors.fg_subtle)
                            .into_any_element()
                    })
                    .child(div().flex_1().min_w_0().child(self.input.clone()))
                    .when_some(self.field_trailing, |field, trailing| {
                        field.child(div().flex_none().child(trailing))
                    }),
            )
            .child(
                div()
                    .id((self.id.clone(), "list"))
                    .role(Role::ListBox)
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .children(children)
                    // 在途时列表空是"还没到"，不是"没匹配"：这时候别说空话。
                    .when(count == 0 && !self.loading, |list| {
                        list.child(
                            div()
                                .debug_selector(|| "palette-empty".into())
                                .flex()
                                .flex_1()
                                .items_center()
                                .justify_center()
                                .text_color(colors.fg_subtle)
                                .child(self.empty),
                        )
                    }),
            )
            .when_some(self.footer, |card, footer| {
                card.child(
                    div()
                        .flex()
                        .flex_none()
                        .p_2()
                        .border_t_1()
                        .border_color(colors.border)
                        .child(footer),
                )
            });
        card.style().refine(&self.style);
        // Padding grows the box the scrim centers.
        div().child(card).with_animation(
            (self.id, "in"),
            Animation::new(motion::duration(motion::BASE, cx)).with_easing(motion::ease_out_cubic),
            |card, t| card.opacity(t).pt(motion::NUDGE * (1.0 - t)),
        )
    }
}

impl Styled for Palette {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
