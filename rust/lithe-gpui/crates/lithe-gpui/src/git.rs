//! Git 组件（第 19 章）：Diff 视图与提交列表。组件管呈现与交互，
//! 数据由调用方（git feature crate / Core）提供——Ely 的 `host` 模式。

use gpui_kit::TestSupportExt as _;
use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{
    App, ElementId, IntoElement, RenderOnce, SharedString, Styled, Window, div, px, rems,
};

use crate::ActiveTheme;

/// 差异行的种类。数据模型是纯值，调用方从 Core 的 diff 结果映射。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffLineKind {
    /// hunk 头（@@ ... @@）。
    HunkHeader,
    Context,
    Added,
    Removed,
}

/// 一行差异。`old_no`/`new_no` 为 1-based 行号，`None` 表示该侧不存在。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_no: Option<usize>,
    pub new_no: Option<usize>,
    pub content: SharedString,
}

impl DiffLine {
    pub fn new(
        kind: DiffLineKind,
        old_no: Option<usize>,
        new_no: Option<usize>,
        content: impl Into<SharedString>,
    ) -> Self {
        Self {
            kind,
            old_no,
            new_no,
            content: content.into(),
        }
    }
}

/// 统计条数据：+/− 行数。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
}

/// Diff 视图。v1 提供统一视图；分栏（split）随表格章一起做。
#[derive(IntoElement)]
pub struct DiffView {
    id: ElementId,
    lines: Vec<DiffLine>,
    stats: Option<DiffStats>,
    style: gpui_kit::StyleRefinement,
}

impl DiffView {
    pub fn new(id: impl Into<ElementId>, lines: Vec<DiffLine>) -> Self {
        Self {
            id: id.into(),
            lines,
            stats: None,
            style: Default::default(),
        }
    }

    pub fn stats(mut self, stats: DiffStats) -> Self {
        self.stats = Some(stats);
        self
    }
}

impl Styled for DiffView {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for DiffView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let mono = theme.mono_font_family.clone();
        let text_xs = rems(0.75);
        div()
            .id(self.id)
            .test_support()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.border)
            .overflow_hidden()
            .flex()
            .flex_col()
            .refine_style(&self.style)
            .when_some(self.stats, |el, stats| {
                el.child(
                    div()
                        .flex()
                        .gap(rems(0.75))
                        .px(rems(0.5))
                        .py(rems(0.25))
                        .bg(theme.sunken)
                        .border_b_1()
                        .border_color(theme.border)
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(
                            div()
                                .text_color(theme.success)
                                .child(format!("+{}", stats.additions)),
                        )
                        .child(
                            div()
                                .text_color(theme.danger)
                                .child(format!("\u{2212}{}", stats.deletions)),
                        ),
                )
            })
            .children(self.lines.into_iter().enumerate().map(|(ix, line)| {
                let (bg, fg, prefix) = match line.kind {
                    DiffLineKind::HunkHeader => (theme.sunken, theme.muted_foreground, None),
                    DiffLineKind::Context => (theme.transparent, theme.foreground, None),
                    DiffLineKind::Added => (theme.success_subtle, theme.foreground, Some("+")),
                    DiffLineKind::Removed => {
                        (theme.danger_subtle, theme.foreground, Some("\u{2212}"))
                    }
                };
                div()
                    .id(("line", ix))
                    .test_support()
                    .flex()
                    .items_stretch()
                    .font_family(mono.clone())
                    .text_size(text_xs)
                    .text_color(fg)
                    .bg(bg)
                    .when(line.kind == DiffLineKind::HunkHeader, |el| {
                        el.font_weight(gpui_kit::FontWeight::MEDIUM)
                    })
                    .child(
                        // 行号槽：等宽对齐，固定宽；两侧行号都展示（unified 双列）。
                        div()
                            .flex_shrink_0()
                            .flex()
                            .w(rems(4.5))
                            .px(rems(0.25))
                            .text_color(theme.subtle_foreground)
                            .justify_between()
                            .child(div().child(render_no(line.old_no)))
                            .child(div().child(render_no(line.new_no))),
                    )
                    .children(prefix.map(|p| div().w(px(10.0)).child(p)))
                    .child(div().flex_1().child(line.content))
            }))
    }
}

fn render_no(no: Option<usize>) -> SharedString {
    match no {
        Some(no) => SharedString::from(no.to_string()),
        None => SharedString::from(""),
    }
}

/// 提交摘要（列表行数据）。`hash` 是完整或缩写哈希，展示为等宽。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitSummary {
    pub hash: SharedString,
    pub message: SharedString,
    pub author: SharedString,
    pub relative_time: SharedString,
}

impl CommitSummary {
    pub fn new(
        hash: impl Into<SharedString>,
        message: impl Into<SharedString>,
        author: impl Into<SharedString>,
        relative_time: impl Into<SharedString>,
    ) -> Self {
        Self {
            hash: hash.into(),
            message: message.into(),
            author: author.into(),
            relative_time: relative_time.into(),
        }
    }
}

/// 提交列表。`on_select` 报告被点行的下标，选中值由调用方持有。
#[derive(IntoElement)]
pub struct CommitList {
    id: ElementId,
    commits: Vec<CommitSummary>,
    selected_index: Option<usize>,
    on_select: Option<std::rc::Rc<dyn Fn(usize, &mut Window, &mut App)>>,
    style: gpui_kit::StyleRefinement,
}

impl CommitList {
    pub fn new(id: impl Into<ElementId>, commits: Vec<CommitSummary>) -> Self {
        Self {
            id: id.into(),
            commits,
            selected_index: None,
            on_select: None,
            style: Default::default(),
        }
    }

    pub fn selected_index(mut self, index: Option<usize>) -> Self {
        self.selected_index = index;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(std::rc::Rc::new(handler));
        self
    }
}

impl Styled for CommitList {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CommitList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let selected_bg = theme.accent;
        let hover_bg = theme.hover;
        div()
            .id(self.id)
            .test_support()
            .flex()
            .flex_col()
            .refine_style(&self.style)
            .children(self.commits.into_iter().enumerate().map(|(ix, commit)| {
                let selected = self.selected_index == Some(ix);
                let on_select = self.on_select.clone();
                div()
                    .id(("commit", ix))
                    .test_support()
                    .px(rems(0.75))
                    .py(rems(0.5))
                    .flex()
                    .flex_col()
                    .gap(rems(0.125))
                    .when(selected, |el| el.bg(selected_bg))
                    .when(!selected, |el| el.hover(move |style| style.bg(hover_bg)))
                    .cursor_pointer()
                    .on_click(move |_, window, cx| {
                        if let Some(handler) = on_select.as_ref() {
                            handler(ix, window, cx);
                        }
                    })
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.foreground)
                            .child(commit.message),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(rems(0.5))
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .font_family(theme.mono_font_family.clone())
                            .child(commit.hash)
                            .child(
                                div()
                                    .font_family(theme.font_family.clone())
                                    .child(format!("{} · {}", commit.author, commit.relative_time)),
                            ),
                    )
            }))
    }
}
