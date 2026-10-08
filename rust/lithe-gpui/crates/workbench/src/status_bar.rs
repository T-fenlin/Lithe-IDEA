use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, ClickEvent, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, rems,
};
use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::status_bar::StatusBar;

use lithe_gpui_shared::icons::{FileIcon, file_icon, idea};

const CHIP_MAX_WIDTH_SPEC: f32 = 200.;

const CHIP_RADIUS_SPEC: f32 = lithe_gpui_shared::design::CONTROL_RADIUS;

#[non_exhaustive]
pub struct StatusEntry {
    /// 我们自己的图标集（`assets/icons/idea/**`）里的一条资源路径；
    /// `None` = 我们没有这张图 ⇒ 这个条目不带图标（见 [`entry_icon`]）。
    icon: Option<&'static str>,
    text: SharedString,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl StatusEntry {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            icon: None,
            text: text.into(),
            on_click: None,
        }
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn with_idea_icon(mut self, icon: &'static idea::IdeaIcon, cx: &App) -> Self {
        self.icon = Some(icon.path(gpui_kit::component::Theme::global(cx).is_dark()));
        self
    }

    /// 文件类型图标：清单（我们自己的 `extension.json`）命中就用它那张；没命中就用
    /// [`file_icon::fallback_icon`] 按扩展名兜的那张；两张都没有 ⇒ 不带图标。
    ///
    /// 改口径前这里还会在两者都没命中时退回组件库的 `IconName`（`StatusEntry::lucide`
    /// 那个字段），那是「用库图标顶我们的」，已随本片删除。
    pub fn with_file_icon(mut self, name: &str, cx: &App) -> Self {
        let icon = FileIcon::own(file_icon::fallback_icon(name)).with_theme_icon(name, cx);
        let is_dark = gpui_kit::component::Theme::global(cx).is_dark();
        self.icon = icon
            .themed
            .or(icon.fallback.map(|fallback| fallback.path(is_dark)));
        self
    }
}

pub fn status_bar(
    left: &[StatusEntry],
    right: &[StatusEntry],
    _window: &Window,
    cx: &App,
) -> impl IntoElement + use<> {
    StatusBar::new()
        .h_6()
        .py_0()
        .border_0()
        .px_2()
        .bg(cx.theme().tokens.status_bar)
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(status_row(left, right, cx))
}

fn status_row(left: &[StatusEntry], right: &[StatusEntry], cx: &App) -> impl IntoElement {
    h_flex()
        .w_full()
        .items_center()
        .justify_between()
        .gap_2()
        .child(entry_group(left, "status-bar-leading", true, cx))
        .child(entry_group(right, "status-bar-trailing", false, cx))
}

fn entry_group(
    entries: &[StatusEntry],
    id_prefix: &str,
    leading: bool,
    cx: &App,
) -> impl IntoElement {
    h_flex()
        .items_center()
        .gap_0p5()
        .when(leading, |this| this.flex_1().min_w_0().overflow_hidden())
        .when(!leading, |this| this.flex_shrink_0())
        .children(
            entries
                .iter()
                .enumerate()
                .map(|(index, entry)| entry_chip(format!("{id_prefix}-{index}"), entry, cx)),
        )
}

fn entry_chip(id: String, entry: &StatusEntry, cx: &App) -> impl IntoElement {
    let hover_bg = cx.theme().accent;
    let hover_fg = cx.theme().foreground;
    let on_click = entry.on_click.clone();

    h_flex()
        .id(id)
        .h_6()
        .max_w(rems(CHIP_MAX_WIDTH_SPEC / 16.))
        .flex_shrink_0()
        .items_center()
        .gap_1()
        .rounded(rems(CHIP_RADIUS_SPEC / 16.))
        .px_1p5()
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        .when_some(entry_icon(entry, cx), |this, icon| this.child(icon))
        .child(entry.text.clone())
        .when_some(on_click, |this, handler| {
            this.cursor_pointer()
                .on_click(move |event, window, cx| handler(event, window, cx))
        })
}

fn entry_icon(entry: &StatusEntry, cx: &App) -> Option<AnyElement> {
    let path = entry.icon?;
    let theme = gpui_kit::component::Theme::global(cx);
    Some(
        gpui::svg()
            .path(path)
            .flex_shrink_0()
            .text_color(theme.muted_foreground)
            .w(rems(14. / 16.))
            .h(rems(14. / 16.))
            .into_any_element(),
    )
}
