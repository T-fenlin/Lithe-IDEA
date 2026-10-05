//! 画廊各章 story。受控组件（表单/标签页/分页/提交列表）的状态都在
//! `GalleryView` 上，经 WeakEntity 在回调里回写。

use gpui_kit::prelude::*;
use gpui_kit::{AnyElement, App, WeakEntity, div, px, rems};

use lithe_gpui::{
    ActiveTheme as _, Alert, Avatar, Badge, BadgeVariant, Breadcrumb, Button, ButtonVariant, Card,
    CardTitle, Checkbox, Code, CodeBlock, CommitList, CommitSummary, DiffLine, DiffLineKind,
    DiffStats, DiffView, Empty, FrameList, Heading, Icon, IconButton, Input, Kbd, Label, Link,
    Menu, MenuItem, Notification, Pagination, Progress, Radio, SectionHeader, Separator, Severity,
    Sidebar, Sizable as _, Spinner, StackFrame, StatusBar, Switch, Tab, Tabs, Tag, Text, TitleBar,
    Variable, VariableList, open_menu, push_notification,
};

use super::GalleryView;

fn el(content: impl IntoElement) -> AnyElement {
    content.into_any_element()
}

fn row(content: impl IntoElement) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_3()
        .child(content)
        .into_any_element()
}

/// 章节清单（与切换侧栏一致）。下标即 `GalleryView::chapter`。
pub const CHAPTERS: [&str; 13] = [
    "1 · Primitives",
    "2 · Typography",
    "3 · Layout",
    "4 · Window & Shell",
    "5 · Buttons & Actions",
    "6 · Forms",
    "7 · Navigation",
    "8 · Menus",
    "9 · Overlays",
    "10 · Feedback",
    "17 · Editor",
    "19 · Git",
    "20 · Debug",
];

/// 渲染单个章节（切换器选中哪章就渲染哪章）。
pub fn chapter(
    ix: usize,
    view: &mut GalleryView,
    state: &WeakEntity<GalleryView>,
    cx: &App,
) -> AnyElement {
    let title = CHAPTERS.get(ix).copied().unwrap_or(CHAPTERS[0]);
    let sections = match ix {
        0 => ch01_primitives(cx),
        1 => ch02_typography(),
        2 => ch03_layout(),
        3 => ch04_shell(cx),
        4 => ch05_buttons(),
        5 => ch06_forms(view, state, cx),
        6 => ch07_navigation(view, state, cx),
        7 => ch08_menus(),
        8 => ch09_overlays(),
        9 => ch10_feedback(),
        10 => ch17_editor(),
        11 => ch19_git(state),
        _ => ch20_debug(),
    };
    let primary = cx.theme().primary;
    let muted = cx.theme().muted_foreground;
    div()
        .flex()
        .flex_col()
        .gap_5()
        .w_full()
        .child(
            div()
                .text_lg()
                .font_weight(gpui_kit::FontWeight::MEDIUM)
                .text_color(primary)
                .child(title),
        )
        .children(sections.into_iter().map(|(name, content)| {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .w_full()
                .child(div().text_sm().text_color(muted).child(name))
                .child(content)
        }))
        .into_any_element()
}

fn ch01_primitives(cx: &App) -> Vec<(&'static str, AnyElement)> {
    let theme = cx.theme();
    vec![
        (
            "Icon",
            row(div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .child(Icon::new("icon-check", "icons/check.svg"))
                .child(Icon::new("icon-x-small", "icons/x.svg").xsmall())
                .child(Icon::new("icon-plus-large", "icons/plus.svg").large())
                .child(
                    Icon::new("icon-chevron-primary", "icons/chevron-down.svg")
                        .color(theme.primary),
                )
                .child(Icon::new("icon-check-danger", "icons/check.svg").color(theme.danger))
                .child(Icon::new("icon-plus-custom", "icons/plus.svg").with_size(px(28.)))),
        ),
        (
            "Separator",
            el(div()
                .flex()
                .flex_col()
                .gap_2()
                .w_full()
                .child(Separator::new("separator-h"))
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .h_8()
                        .child(Separator::new("separator-v").vertical()),
                )),
        ),
        (
            "Avatar",
            row(div()
                .flex()
                .items_center()
                .gap_3()
                .child(Avatar::new("avatar-xsmall", "ZK").xsmall())
                .child(Avatar::new("avatar-small", "LT").small())
                .child(Avatar::new("avatar-medium", "LI").medium())
                .child(Avatar::new("avatar-large", "UI").large())),
        ),
        (
            "Badge",
            row(div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(Badge::new("badge-neutral", "Neutral"))
                .child(Badge::new("badge-primary", "Primary").variant(BadgeVariant::Primary))
                .child(Badge::new("badge-success", "Success").variant(BadgeVariant::Success))
                .child(Badge::new("badge-warning", "Warning").variant(BadgeVariant::Warning))
                .child(Badge::new("badge-danger", "Danger").variant(BadgeVariant::Danger))
                .child(Badge::new("badge-info", "Info").variant(BadgeVariant::Info))
                .child(Badge::new("badge-small", "Small").small())
                .child(Badge::new("badge-large", "Large").large())),
        ),
        ("Tag", row(Tag::new("tag-plain", "Plain"))),
        (
            "Label / Kbd",
            row(div()
                .flex()
                .items_center()
                .gap_4()
                .child(Label::new("label-name", "Project name"))
                .child(Label::new("label-required", "Java home").required(true))
                .child(Kbd::new("kbd-cmd", "⌘K"))
                .child(Kbd::new("kbd-combo", "Ctrl+Shift+P"))),
        ),
        (
            "Spinner / Progress",
            el(div()
                .flex()
                .flex_col()
                .gap_2()
                .w_full()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(Spinner::new("spinner-medium"))
                        .child(Spinner::new("spinner-large").large()),
                )
                .child(Progress::new("progress-25").value(0.25))
                .child(Progress::new("progress-60").value(0.6))),
        ),
        (
            "Severity",
            row(div().flex().items_center().gap_3().children(
                [
                    (Severity::Neutral, "Neutral"),
                    (Severity::Info, "Info"),
                    (Severity::Success, "Success"),
                    (Severity::Warning, "Warning"),
                    (Severity::Danger, "Danger"),
                ]
                .into_iter()
                .enumerate()
                .map(|(ix, (severity, name))| {
                    div()
                        .id(("severity", ix))
                        .px_2()
                        .h_6()
                        .flex()
                        .items_center()
                        .rounded_full()
                        .bg(cx.theme().severity_subtle(severity))
                        .text_color(cx.theme().severity(severity))
                        .text_xs()
                        .child(name)
                }),
            )),
        ),
    ]
}

fn ch02_typography() -> Vec<(&'static str, AnyElement)> {
    vec![
        (
            "Heading",
            el(div().flex().flex_col().gap_2().w_full().children([
                Heading::new("heading-display", "Display").display(),
                Heading::new("heading-h1", "Heading 1"),
                Heading::new("heading-h2", "Heading 2").h2(),
                Heading::new("heading-h3", "Heading 3").h3(),
                Heading::new("heading-h4", "Heading 4").h4(),
            ])),
        ),
        (
            "Text / Code",
            row(div()
                .flex()
                .items_center()
                .gap_3()
                .child(Text::new("text-default", "Default body text"))
                .child(Text::new("text-muted", "Muted").muted())
                .child(Text::new("text-subtle", "Subtle").subtle().small())
                .child(Code::new("code-inline", "mvn package"))),
        ),
    ]
}

fn ch03_layout() -> Vec<(&'static str, AnyElement)> {
    vec![(
        "SectionHeader / Card / CardTitle",
        el(div()
            .flex()
            .flex_col()
            .gap_3()
            .w_full()
            .child(
                SectionHeader::new("sh-1", "Section header")
                    .description("Optional description under the section title."),
            )
            .child(
                Card::new("card-1")
                    .child(CardTitle::new("card-title", "Card title"))
                    .child(Text::new("card-text", "Card body on the surface color.").muted()),
            )),
    )]
}

fn ch04_shell(cx: &App) -> Vec<(&'static str, AnyElement)> {
    vec![(
        "TitleBar / Sidebar / StatusBar（示意框内）",
        el(div()
            .w_full()
            .max_w(rems(42.0))
            .rounded(rems(0.5))
            .border_1()
            .border_color(cx.theme().border)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(TitleBar::new("shell-titlebar").child(Text::new("shell-title", "Lithe").small()))
            .child(
                div()
                    .flex()
                    .items_stretch()
                    .h(rems(8.0))
                    .child(
                        Sidebar::new("shell-sidebar").child(
                            div()
                                .p_3()
                                .text_sm()
                                .text_color(cx.theme().sidebar_foreground)
                                .child("Explorer"),
                        ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .text_color(cx.theme().subtle_foreground)
                            .child("Editor area"),
                    ),
            )
            .child(
                StatusBar::new("shell-status")
                    .child(Text::new("shell-status-1", "main").small())
                    .child(Text::new("shell-status-2", "UTF-8").small()),
            )),
    )]
}

fn ch05_buttons() -> Vec<(&'static str, AnyElement)> {
    vec![
        (
            "Button variants",
            row(div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(Button::new("btn-primary").label("Primary").primary())
                .child(Button::new("btn-secondary").label("Secondary"))
                .child(Button::new("btn-outline").label("Outline").outline())
                .child(Button::new("btn-ghost").label("Ghost").ghost())
                .child(Button::new("btn-danger").label("Delete").danger())
                .child(Button::new("btn-disabled").label("Disabled").disabled(true))
                .child(
                    Button::new("btn-selected")
                        .label("Selected")
                        .ghost()
                        .selected(true),
                )),
        ),
        (
            "Button sizes / IconButton / Link",
            row(div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(Button::new("btn-xsmall").label("XSmall").xsmall())
                .child(Button::new("btn-small").label("Small").small())
                .child(Button::new("btn-large").label("Large").large())
                .child(
                    IconButton::new("iconbtn-plus", "icons/plus.svg")
                        .variant(ButtonVariant::Outline),
                )
                .child(IconButton::new("iconbtn-x", "icons/x.svg").danger())
                .child(Link::new(
                    "link-docs",
                    "GPUI Kit docs",
                    "https://gpui-kit.com",
                ))),
        ),
    ]
}

fn ch06_forms(
    view: &mut GalleryView,
    state: &WeakEntity<GalleryView>,
    _cx: &App,
) -> Vec<(&'static str, AnyElement)> {
    let checked = view.checked;
    let switch_on = view.switch_on;
    let radio_ix = view.radio_ix;
    let input = view.input.clone();
    let checkbox_state = state.clone();
    let switch_state = state.clone();
    let radio_state = state.clone();
    vec![
        (
            "Input（受控 InputState）",
            el(div().w(rems(18.0)).child(Input::new(&input))),
        ),
        (
            "Checkbox / Switch / Radio",
            row(div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_4()
                .child(
                    Checkbox::new("checkbox-demo")
                        .label("Show hidden files")
                        .checked(checked)
                        .on_change(move |checked, _, cx| {
                            let _ = checkbox_state.update(cx, |v, cx| {
                                v.checked = *checked;
                                cx.notify();
                            });
                        }),
                )
                .child(Switch::new("switch-demo").checked(switch_on).on_change(
                    move |checked, _, cx| {
                        let _ = switch_state.update(cx, |v, cx| {
                            v.switch_on = *checked;
                            cx.notify();
                        });
                    },
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .children([0usize, 1, 2].map(|ix| {
                            let radio_state = radio_state.clone();
                            Radio::new(("radio-demo", ix))
                                .label(format!("Option {ix}"))
                                .selected(radio_ix == ix)
                                .on_change(move |_, _, cx| {
                                    let _ = radio_state.update(cx, |v, cx| {
                                        v.radio_ix = ix;
                                        cx.notify();
                                    });
                                })
                        })),
                )),
        ),
    ]
}

fn ch07_navigation(
    view: &mut GalleryView,
    state: &WeakEntity<GalleryView>,
    _cx: &App,
) -> Vec<(&'static str, AnyElement)> {
    let tab_state = state.clone();
    let page_state = state.clone();
    let tab_ix = view.tab_ix;
    let page = view.page;
    vec![(
        "Tabs / Breadcrumb / Pagination（回调回写画廊状态）",
        el(div()
            .flex()
            .flex_col()
            .gap_4()
            .w_full()
            .child(
                Tabs::new("tabs-demo")
                    .selected_index(tab_ix)
                    .child(Tab::new("General"))
                    .child(Tab::new("Editor"))
                    .child(Tab::new("Terminal"))
                    .on_select(move |ix, _, cx| {
                        let _ = tab_state.update(cx, |v, cx| {
                            v.tab_ix = ix;
                            cx.notify();
                        });
                    }),
            )
            .child(
                Breadcrumb::new("breadcrumb-demo")
                    .item("Project", |_, _| {})
                    .item("src", |_, _| {})
                    .current("main.rs"),
            )
            .child(
                Pagination::new("pagination-demo", page, 9).on_change(move |page, _, cx| {
                    let _ = page_state.update(cx, |v, cx| {
                        v.page = page;
                        cx.notify();
                    });
                }),
            )),
    )]
}

fn ch08_menus() -> Vec<(&'static str, AnyElement)> {
    vec![(
        "ContextMenu（点击打开；点击外部或菜单项后关闭）",
        row(Button::new("btn-open-menu")
            .label("Open menu")
            .outline()
            .on_click(|_, window, cx| {
                let menu = Menu::new()
                    .item(MenuItem::new("New File").on_click(|_, _| {}))
                    .item(MenuItem::new("Rename…").on_click(|_, _| {}))
                    .separator()
                    .item(MenuItem::new("Disabled item").disabled(true))
                    .separator()
                    .item(MenuItem::new("Delete").danger(true).on_click(|_, _| {}));
                open_menu(menu, window.mouse_position(), window, cx);
            })),
    )]
}

fn ch09_overlays() -> Vec<(&'static str, AnyElement)> {
    vec![(
        "Notification（右下角，手动关闭）",
        row(div().flex().items_center().gap_2().children([
            Button::new("btn-notify-info")
                .label("Info")
                .outline()
                .on_click(|_, window, cx| {
                    push_notification(
                        Notification::new("Indexing complete").message("1247 files scanned"),
                        window,
                        cx,
                    );
                }),
            Button::new("btn-notify-success")
                .label("Success")
                .outline()
                .on_click(|_, window, cx| {
                    push_notification(Notification::new("Build succeeded").success(), window, cx);
                }),
            Button::new("btn-notify-danger")
                .label("Error")
                .danger()
                .on_click(|_, window, cx| {
                    push_notification(
                        Notification::new("Build failed")
                            .message("1 error, 2 warnings")
                            .danger(),
                        window,
                        cx,
                    );
                }),
        ])),
    )]
}

fn ch10_feedback() -> Vec<(&'static str, AnyElement)> {
    vec![
        (
            "Alert",
            el(div().flex().flex_col().gap_2().w_full().children([
                Alert::new("alert-info", "Heads up")
                    .info()
                    .child(div().child("A new version of the JDK index is available.")),
                Alert::new("alert-warning", "Slow indexing")
                    .warning()
                    .child(div().child("Indexing is taking longer than usual.")),
                Alert::new("alert-danger", "Build failed")
                    .danger()
                    .child(div().child("Compilation error in Main.java:12.")),
            ])),
        ),
        (
            "Empty",
            el(Empty::new("empty-demo", "No results")
                .description("Try a different search term.")
                .child(Button::new("btn-empty-clear").label("Clear filter").ghost())),
        ),
    ]
}

fn ch17_editor() -> Vec<(&'static str, AnyElement)> {
    let code = "public class Main {\n    public static void main(String[] args) {\n        System.out.println(\"Hello, Lithe\");\n    }\n}";
    vec![(
        "CodeBlock（只读；语法高亮待 base tree-sitter 接入）",
        el(div().w(rems(24.0)).child(CodeBlock::new("code-demo", code))),
    )]
}

fn ch19_git(state: &WeakEntity<GalleryView>) -> Vec<(&'static str, AnyElement)> {
    let lines = vec![
        DiffLine::new(
            DiffLineKind::HunkHeader,
            None,
            None,
            "@@ -12,7 +12,9 @@ public class Main {",
        ),
        DiffLine::new(
            DiffLineKind::Context,
            Some(12),
            Some(12),
            "public class Main {",
        ),
        DiffLine::new(
            DiffLineKind::Removed,
            Some(13),
            None,
            "    System.out.println(\"hi\");",
        ),
        DiffLine::new(
            DiffLineKind::Added,
            None,
            Some(13),
            "    if (args.length == 0) {",
        ),
        DiffLine::new(DiffLineKind::Added, None, Some(14), "        return;"),
        DiffLine::new(DiffLineKind::Added, None, Some(15), "    }"),
        DiffLine::new(DiffLineKind::Context, Some(16), Some(16), "}"),
    ];
    let commits = vec![
        CommitSummary::new(
            "f0c20e04",
            "feat(gpui): inspect 链条加有界自旋 pump",
            "Fixture",
            "2 days ago",
        ),
        CommitSummary::new(
            "eaf26bd2",
            "feat(gpui): inspect 自动推进改为串行四步",
            "Fixture",
            "3 days ago",
        ),
        CommitSummary::new(
            "e51a7e43",
            "docs(notes): 最终解法记录",
            "Fixture",
            "3 days ago",
        ),
    ];
    let commit_state = state.clone();
    vec![
        (
            "DiffView（unified）",
            el(div()
                .w_full()
                .child(DiffView::new("diff-demo", lines).stats(DiffStats {
                    additions: 3,
                    deletions: 1,
                }))),
        ),
        (
            "CommitList",
            el(div()
                .w(rems(28.0))
                .child(
                    CommitList::new("commit-list", commits).on_select(move |ix, _, cx| {
                        let _ = commit_state.update(cx, |v, cx| {
                            v.selected_commit = Some(ix);
                            cx.notify();
                        });
                    }),
                )),
        ),
    ]
}

fn ch20_debug() -> Vec<(&'static str, AnyElement)> {
    let frames = vec![
        StackFrame::new("Main.main", "Main.java:15"),
        StackFrame::new("App.launch", "App.java:88"),
        StackFrame::new("Thread.run", "Thread.java:834"),
    ];
    let variables = vec![
        Variable::new("args", "String[1] { \"--verbose\" }").depth(0),
        Variable::new("count", "42").changed(true).depth(1),
        Variable::new("builder", "StringBuilder@1f2a").depth(1),
    ];
    vec![(
        "FrameList / VariableList",
        el(div()
            .flex()
            .gap_6()
            .w_full()
            .child(
                div()
                    .w(rems(18.0))
                    .child(FrameList::new("frames-demo", frames).selected_index(Some(0))),
            )
            .child(
                div()
                    .w(rems(22.0))
                    .child(VariableList::new("variables-demo", variables)),
            )),
    )]
}
