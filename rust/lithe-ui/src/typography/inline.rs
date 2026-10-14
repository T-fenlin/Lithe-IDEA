use std::ops::Range;

use gpui::{
    AnyElement, App, FontWeight, HighlightStyle, Hsla, IntoElement, Keystroke, ParentElement,
    RenderOnce, SharedString, Styled, StyledText, Window, div,
};
use regex::RegexBuilder;
use smallvec::SmallVec;

use super::keys::{keystroke, keystroke_labels};
use crate::theme::{ActiveTheme, Platform, Radius, TextSize};

/// Monospace fragment on a quiet fill.
#[derive(IntoElement)]
pub struct Code {
    text: SharedString,
}

impl Code {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl RenderOnce for Code {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .px_1()
            .rounded(theme.radius(Radius::Sm))
            .bg(theme.colors.sunken)
            .border_1()
            .border_color(theme.colors.border)
            .font_family(theme.mono_family.clone())
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(theme.colors.fg)
            .child(self.text)
    }
}

/// 一个键帽。`color` 由调用方给：键帽自己定色，父级的 `text_color` 传不进来。
fn cap(label: String, color: Hsla, cx: &App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    div()
        .min_w(theme.text_size(TextSize::Xs) * 1.8)
        .px_1()
        .flex()
        .justify_center()
        .rounded(theme.radius(Radius::Sm))
        .border_1()
        .border_b_2()
        .border_color(theme.colors.border_strong)
        .bg(theme.colors.surface)
        .text_size(theme.text_size(TextSize::Xs))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color)
        .child(label)
}

/// One key, labeled for the theme's platform. Takes gpui key syntax.
#[derive(IntoElement)]
pub struct Kbd {
    source: SharedString,
}

impl Kbd {
    /// `"secondary-s"` reads ⌘S for the Mac and Ctrl S for the others.
    pub fn new(source: &str) -> Self {
        Self {
            source: SharedString::from(source.to_string()),
        }
    }
}

impl RenderOnce for Kbd {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let platform = theme.platform;
        let color = theme.colors.fg_muted;
        let labels = keystroke_labels(&keystroke(&self.source, platform), platform);
        div()
            .flex()
            .gap_0p5()
            .children(labels.into_iter().map(|label| cap(label, color, cx)))
    }
}

/// A chord sequence such as `"cmd-k cmd-s"`.
#[derive(IntoElement)]
pub struct KbdCombo {
    source: SharedString,
    disabled: bool,
}

impl KbdCombo {
    pub fn new(source: &str) -> Self {
        Self {
            source: SharedString::from(source.to_string()),
            disabled: false,
        }
    }

    /// 让键帽文字跟着所在行一起降一档，留给菜单那种「整行都灰了、只有快捷键胶囊还亮着」
    /// 的地方。
    ///
    /// **默认关闭**：其余调用点（设置页快捷键清单、命令面板、欢迎页、热键录制框、
    /// 编辑器键位表、画廊）照旧取 `fg_muted`，一字不变。只有菜单显式打开这个开关。
    ///
    /// 值得说明为什么非得加这个开关：键帽的底色与文字色都是 `cap()` 自己从主题取的
    /// （`:44` / `:55`），父级 `text_color` 传不进键帽内部，所以在 `menus/draw.rs` 的
    /// 容器上改 `text_color` 是死样式 —— 想让胶囊跟着行走，只能从这里传进来。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for KbdCombo {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let platform = cx.theme().platform;
        let strokes: Vec<Keystroke> = self
            .source
            .split_whitespace()
            .map(|source| keystroke(source, platform))
            .collect();
        // 禁用时键帽文字与「+」都降成 `fg_disabled`，与所在行的文字同档；
        // 键帽的底色（`surface`）与描边（`border_strong`）不动 —— 形状已经很安静，
        // 抢眼的从来只有文字。
        let theme = cx.theme();
        let key_color = if self.disabled {
            theme.colors.fg_disabled
        } else {
            theme.colors.fg_muted
        };
        let subtle = if self.disabled {
            theme.colors.fg_disabled
        } else {
            theme.colors.fg_subtle
        };
        let spelled = platform != Platform::Mac;
        div()
            .flex()
            .items_center()
            .gap_2()
            .children(strokes.iter().map(|stroke| {
                let labels = keystroke_labels(stroke, platform);
                let last = labels.len() - 1;
                div().flex().items_center().gap_0p5().children(
                    labels.into_iter().enumerate().flat_map(|(ix, label)| {
                        let mut parts = vec![cap(label, key_color, cx).into_any_element()];
                        if spelled && ix < last {
                            parts.push(div().text_color(subtle).child("+").into_any_element());
                        }
                        parts
                    }),
                )
            }))
    }
}

/// Quoted passage behind a rule.
#[derive(IntoElement)]
pub struct Blockquote {
    children: SmallVec<[AnyElement; 2]>,
}

impl Blockquote {
    pub fn new() -> Self {
        Self {
            children: SmallVec::new(),
        }
    }
}

impl Default for Blockquote {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Blockquote {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Blockquote {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .pl_4()
            .border_l_2()
            .border_color(theme.colors.border_strong)
            .text_color(theme.colors.fg_muted)
            .text_size(theme.text_size(TextSize::Md))
            .children(self.children)
    }
}

/// Text with marked ranges, as for search matches.
#[derive(IntoElement)]
pub struct Highlight {
    text: SharedString,
    ranges: Vec<Range<usize>>,
}

impl Highlight {
    /// Byte ranges on char boundaries.
    pub fn new(text: impl Into<SharedString>, ranges: Vec<Range<usize>>) -> Self {
        Self {
            text: text.into(),
            ranges,
        }
    }

    /// Marks every case-insensitive occurrence of `query`.
    pub fn matching(text: impl Into<SharedString>, query: &str) -> Self {
        let text = text.into();
        let ranges = if query.is_empty() {
            Vec::new()
        } else {
            RegexBuilder::new(&regex::escape(query))
                .case_insensitive(true)
                .build()
                .expect("an escaped literal is a valid regex")
                .find_iter(&text)
                .map(|found| found.range())
                .collect()
        };
        Self { text, ranges }
    }
}

impl RenderOnce for Highlight {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let mark = HighlightStyle {
            background_color: Some(colors.warning_subtle),
            color: Some(colors.fg),
            ..Default::default()
        };
        StyledText::new(self.text)
            .with_highlights(self.ranges.into_iter().map(|range| (range, mark)))
    }
}

#[cfg(test)]
mod tests {
    use super::{Highlight, Range};

    #[test]
    fn matching_finds_every_case_insensitive_hit() {
        let found = Highlight::matching("Ely, ely, ELY and élan", "ely");
        assert_eq!(found.ranges, [0..3, 5..8, 10..13]);
        let none = Highlight::matching("abc", "");
        assert!(none.ranges.is_empty());
        let literal = Highlight::matching("a.b ab", ".");
        assert_eq!(literal.ranges, vec![Range { start: 1, end: 2 }]);
    }
}
