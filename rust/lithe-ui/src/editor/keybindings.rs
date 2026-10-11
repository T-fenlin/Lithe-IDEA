use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, Keystroke, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    data_display::{Badge, Tone},
    forms::HotkeyInput,
    i18n::text,
    navigation::fuzzy,
    primitives::{Icon, IconName, Tooltip},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::KbdCombo,
};

/// Where a binding comes from.
///
/// Three tiers, not two: a host that layers a preset above the built-in defaults must be able to
/// tell them apart. Collapsing `Preset` into `User` would mark rows the user never touched as
/// "changed by you", which is wrong in a way the reader cannot correct — `Tone::Info` reads as an
/// edit the user made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeySource {
    Default,
    Preset,
    User,
}

/// A command and the keys that run it, in gpui key syntax, when a context holds.
///
/// `bindable` is false for a command the host has no dispatchable action for: the row is still
/// listed so the reader can see the command exists, but it is dimmed and cannot be recorded into.
#[derive(Clone, Debug, PartialEq)]
pub struct Keybinding {
    pub command: SharedString,
    pub title: SharedString,
    pub keys: Option<SharedString>,
    pub when: Option<SharedString>,
    pub source: KeySource,
    pub bindable: bool,
}

/// Commands sharing keys in the same context: for each binding, the others its keys also run.
pub fn conflicts(bindings: &[Keybinding], binding: &Keybinding) -> Vec<SharedString> {
    let Some(keys) = &binding.keys else {
        return Vec::new();
    };
    bindings
        .iter()
        .filter(|other| {
            other.command != binding.command
                && other.keys.as_ref() == Some(keys)
                && other.when == binding.when
        })
        .map(|other| other.title.clone())
        .collect()
}

type OnCommand = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnBind = Rc<dyn Fn(&SharedString, Option<Keystroke>, &mut Window, &mut App)>;

/// Every command with its keys, context and source; a query narrows them, keys that clash are marked, and a double press records new keys.
#[derive(IntoElement)]
pub struct KeybindingsEditor {
    id: ElementId,
    bindings: Vec<Keybinding>,
    query: SharedString,
    recording: Option<SharedString>,
    on_record: Option<OnCommand>,
    on_bind: Option<OnBind>,
}

impl KeybindingsEditor {
    pub fn new(id: impl Into<ElementId>, bindings: impl IntoIterator<Item = Keybinding>) -> Self {
        Self {
            id: id.into(),
            bindings: bindings.into_iter().collect(),
            query: SharedString::default(),
            recording: None,
            on_record: None,
            on_bind: None,
        }
    }

    /// Narrows to commands whose title, command or keys fit.
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    /// The command whose keys are being recorded.
    pub fn recording(mut self, command: Option<SharedString>) -> Self {
        self.recording = command;
        self
    }

    /// Gets the command a double press chose to record.
    pub fn on_record(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_record = Some(Rc::new(handler));
        self
    }

    /// Gets the keys recorded for a command, or none when cleared.
    pub fn on_bind(
        mut self,
        handler: impl Fn(&SharedString, Option<Keystroke>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_bind = Some(Rc::new(handler));
        self
    }
}

/// The four columns every row shares.
fn columns<E: Styled>(row: E) -> E {
    row.flex().items_center().gap_3().px_2()
}

impl RenderOnce for KeybindingsEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let query = self.query.trim().to_string();
        let fits = |binding: &Keybinding| {
            query.is_empty()
                || [
                    Some(&binding.title),
                    Some(&binding.command),
                    binding.keys.as_ref(),
                ]
                .into_iter()
                .flatten()
                .any(|text| fuzzy(&query, text).is_some())
        };
        let widths = [2.25, 1.5, 1.5, 0.75].map(|share| theme.label_width() * share);
        // Source 那一栏的角标走文案表：它们跟别的界面文案一起换语言。
        let (source_default, source_preset, source_user, row_unbindable) = (
            text(cx, "keys.source.default", &[]),
            text(cx, "keys.source.preset", &[]),
            text(cx, "keys.source.user", &[]),
            text(cx, "keys.row.unbindable", &[]),
        );
        // 四列表头同样走文案表：中文 IDE 的快捷键设置页不能留英文表头。
        let (head_command, head_binding, head_when, head_source) = (
            text(cx, "keys.table.command", &[]),
            text(cx, "keys.table.binding", &[]),
            text(cx, "keys.table.when", &[]),
            text(cx, "keys.table.source", &[]),
        );
        let head = columns(div())
            .py_1()
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(colors.fg_subtle)
            .border_b_1()
            .border_color(colors.border)
            .child(div().w(widths[0]).child(head_command))
            .child(div().w(widths[1]).child(head_binding))
            .child(div().w(widths[2]).child(head_when))
            .child(div().w(widths[3]).child(head_source));
        let rows = self
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, binding)| fits(binding))
            .map(|(ix, binding)| {
                let clashes = conflicts(&self.bindings, binding);
                let recording = self.recording.as_ref() == Some(&binding.command);
                let bindable = binding.bindable;
                let keys = if recording {
                    let (bind, command) = (self.on_bind.clone(), binding.command.clone());
                    HotkeyInput::new((self.id.clone(), format!("keys-{ix}")), None)
                        .on_change(move |stroke, window, cx| {
                            log::info!("keybindings: {command} = {stroke:?}");
                            if let Some(bind) = &bind {
                                bind(&command, stroke, window, cx);
                            }
                        })
                        .into_any_element()
                } else {
                    let cell = div().flex().items_center().gap_1p5();
                    let cell = match &binding.keys {
                        Some(keys) => {
                            cell.child(KbdCombo::new(keys))
                                .when(!clashes.is_empty(), |cell| {
                                    let words = format!("Also runs {}", clashes.join(", "));
                                    cell.child(
                                        div()
                                            .id((self.id.clone(), format!("clash-{ix}")))
                                            .tooltip(Tooltip::text(words))
                                            .child(
                                                Icon::new(IconName::TriangleAlert)
                                                    .size(IconSize::Xs)
                                                    .color(colors.warning),
                                            ),
                                    )
                                })
                        }
                        None => cell.text_color(colors.fg_subtle).child("—"),
                    };
                    // 角标挂在按键格而不是空值分支里：不可绑的行**也可能有键位**（预设或默认给
                    // 的），那时同样要说清「看得见但改不了」，否则整行变灰没有任何解释。
                    cell.when(!bindable, |cell| {
                        cell.child(Badge::new(row_unbindable.clone()))
                    })
                    .into_any_element()
                };
                let record = self.on_record.clone();
                let command = binding.command.clone();
                columns(div().id((self.id.clone(), format!("row-{ix}"))))
                    .py_1p5()
                    .rounded(theme.radius(Radius::Sm))
                    // 不可改的行整行压暗，且不参与悬停与双击录制 —— 灰而不解释为什么会让人
                    // 以为是自己点漏了，所以按键那格还要给一枚「未接入」角标（见下）。
                    .when(!bindable, |row| row.opacity(0.55))
                    .when(recording, |row| row.bg(colors.active))
                    .when(!recording && bindable, |row| {
                        row.hover(|row| row.bg(colors.hover))
                    })
                    .when_some(record.filter(|_| bindable), |row, record| {
                        row.on_click(move |event, window, cx| {
                            if event.click_count() == 2 {
                                record(&command, window, cx)
                            }
                        })
                    })
                    .child(
                        div()
                            .w(widths[0])
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(colors.fg)
                                    .child(binding.title.clone()),
                            )
                            .child(
                                div()
                                    .font_family(theme.mono_family.clone())
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.fg_subtle)
                                    .child(binding.command.clone()),
                            ),
                    )
                    .child(div().w(widths[1]).child(keys))
                    .child(
                        div()
                            .w(widths[2])
                            .font_family(theme.mono_family.clone())
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_muted)
                            .children(binding.when.clone()),
                    )
                    .child(div().w(widths[3]).flex().child(match binding.source {
                        KeySource::Default => Badge::new(source_default.clone()),
                        // 预设与默认都不是用户改的，所以共用中性徽章，只靠文案区分；
                        // `Tone::Info` 那档留给 `User`——它才表示「这行是你改过的」。
                        KeySource::Preset => Badge::new(source_preset.clone()),
                        KeySource::User => Badge::new(source_user.clone()).tone(Tone::Info),
                    }))
            });
        div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .child(head)
            .children(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(command: &str, keys: &str, when: Option<&str>) -> Keybinding {
        Keybinding {
            command: command.to_string().into(),
            title: command.to_string().into(),
            keys: Some(keys.to_string().into()),
            when: when.map(|when| when.to_string().into()),
            source: KeySource::Default,
            bindable: true,
        }
    }

    #[test]
    fn keys_clash_only_in_the_same_context() {
        let all = [
            binding("save", "secondary-s", None),
            binding("sort", "secondary-s", None),
            binding("split", "secondary-s", Some("Terminal")),
        ];
        assert_eq!(conflicts(&all, &all[0]), ["sort"]);
        assert!(
            conflicts(&all, &all[2]).is_empty(),
            "another context does not clash"
        );
    }
}
