use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, ClickEvent, Entity, InteractiveElement as _, IntoElement, Length,
    ParentElement as _, Pixels, Point, SharedString, StatefulInteractiveElement as _, Styled as _,
    Window, div, rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_image, idea_icon_mono, idea_icon_svg_px};

use lithe_gpui_shared::tr;
use lithe_ui::buttons::{Button, ButtonVariant};
use lithe_ui::layout::{h_stack, v_stack};
use lithe_ui::theme::{ActiveTheme as _, Size};

const TOOL_WINDOW_HEADER_HEIGHT_SPEC: f32 = 30.;
const TOOL_WINDOW_HEADER_ICON_SPEC: f32 = 13.;
const TOOL_WINDOW_HEADER_TRAILING_INSET_SPEC: f32 = 7.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RightToolWindowView {
    Notifications,
    Maven,
    Spring,
}

impl RightToolWindowView {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Notifications => "notifications",
            Self::Maven => "maven",
            Self::Spring => "spring",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "notifications" => Some(Self::Notifications),
            "maven" => Some(Self::Maven),
            "spring" => Some(Self::Spring),
            _ => None,
        }
    }

    pub const fn from_rail_index(index: usize) -> Option<Self> {
        match index {
            1 => Some(Self::Notifications),
            2 => Some(Self::Maven),
            _ => None,
        }
    }

    fn title(self) -> SharedString {
        match self {
            Self::Notifications => tr("lithe.notifications.title"),
            Self::Maven => tr("lithe.maven.title"),
            Self::Spring => tr("lithe.spring.title"),
        }
    }

    fn icon(self, cx: &App, side: impl Into<Length>) -> AnyElement {
        let side: Length = side.into();
        match self {
            Self::Notifications => idea_icon_svg_px(&idea::BELL_ICON, cx, side)
                .text_color(cx.theme().colors.info)
                .into_any_element(),
            Self::Maven => idea_icon_image(&idea::MAVEN_ICON, cx, side),
            // 本仓图标集里**没有**「Spring 的叶子」那张图形 ⇒ 同尺寸空槽（留位、不画），
            // 不拿组件库的 `IconName::Leaf` 顶替（维护者口径：应用图标只取 `assets/icons/idea/**`，
            // 我们缺的留空）。补图形要走「往 `assets/icons/idea/**` 加一张我们自己的图」。
            Self::Spring => div().flex_none().w(side).h(side).into_any_element(),
        }
    }

    fn empty_title(self) -> SharedString {
        match self {
            Self::Notifications => tr("lithe.notifications.empty"),
            Self::Maven => tr("lithe.maven.notDetected"),
            Self::Spring => tr("lithe.spring.notDetected"),
        }
    }
}

pub fn resolve_click(
    clicked: RightToolWindowView,
    current: RightToolWindowView,
    current_visible: bool,
) -> (RightToolWindowView, bool) {
    if current_visible && current == clicked {
        (current, false)
    } else {
        (clicked, true)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ProbePoint {
    x: f32,
    y: f32,
}

impl ProbePoint {
    pub(crate) const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<Point<Pixels>> for ProbePoint {
    fn from(point: Point<Pixels>) -> Self {
        Self::new(f32::from(point.x), f32::from(point.y))
    }
}

static RIGHT_PANEL_SEQ: AtomicUsize = AtomicUsize::new(0);

static RIGHT_PANEL_EPOCH: OnceLock<Instant> = OnceLock::new();

fn diagnose_line(
    view: RightToolWindowView,
    visible: bool,
    point: Option<ProbePoint>,
    seq: usize,
    t_ms: u128,
) -> String {
    let mut line = format!(
        "S1_RIGHT_PANEL seq={seq} t_ms={t_ms} view={} visible={visible}",
        view.id()
    );
    if let Some(point) = point {
        line.push_str(&format!(" x={:.1} y={:.1}", point.x, point.y));
    }
    line
}

pub(crate) fn diagnose(view: RightToolWindowView, visible: bool, point: Option<ProbePoint>) {
    let seq = RIGHT_PANEL_SEQ.fetch_add(1, Ordering::Relaxed);
    let t_ms = RIGHT_PANEL_EPOCH
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis();
    eprintln!("{}", diagnose_line(view, visible, point, seq, t_ms));
}

pub(crate) fn diagnose_extensions_page(open: bool, point: Option<ProbePoint>) {
    if let Some(point) = point {
        eprintln!(
            "S1_EXTENSIONS_PAGE open={open} x={:.1} y={:.1}",
            point.x, point.y
        );
    } else {
        eprintln!("S1_EXTENSIONS_PAGE open={open}");
    }
}

pub fn right_tool_window(
    view: RightToolWindowView,
    maven: Option<&crate::maven::MavenProjectView>,
    spring: Option<&crate::spring::SpringIndexView>,
    notifications: &Entity<crate::notifications::NotificationPanel>,
    spring_panel: &Entity<crate::spring_panel::SpringPanel>,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let title = view.title();
    let spring_query = spring_panel.read(cx).query().clone();
    let subtitle = match (view, spring) {
        (RightToolWindowView::Spring, Some(index)) => {
            let count = crate::spring::filter_endpoints(&index.endpoints, &spring_query)
                .len()
                .to_string();
            Some(lithe_gpui_shared::tr_args(
                "lithe.spring.gpui.routeCount",
                &[("count", count.as_str())],
            ))
        }
        _ => None,
    };
    v_stack()
        .id("lithe-right-tool-window")
        .aria_label(title.clone())
        .size_full()
        .child(header(view, title, subtitle, on_close, cx))
        .children(match view {
            RightToolWindowView::Spring => {
                Some(spring_panel.read(cx).filter_row(cx).into_any_element())
            }
            _ => None,
        })
        .child(match (view, maven, spring) {
            (RightToolWindowView::Maven, Some(project), _) => maven_content(project, cx),
            (RightToolWindowView::Spring, _, Some(index)) if !index.is_empty() => {
                spring_content(index, &spring_query, cx)
            }
            (RightToolWindowView::Notifications, _, _) => notifications.clone().into_any_element(),
            _ => empty_state(view, cx),
        })
        .into_any_element()
}

fn spring_content(index: &crate::spring::SpringIndexView, query: &str, cx: &App) -> AnyElement {
    let visible = crate::spring::filter_endpoints(&index.endpoints, query);
    if visible.is_empty() {
        return v_stack()
            .w_full()
            .flex_1()
            .min_h_0()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(tr("lithe.spring.gpui.noEndpoints")),
            )
            .into_any_element();
    }

    let mono = cx.theme().mono_family.clone();
    let mut column = v_stack()
        .id("spring-index")
        .w_full()
        .flex_1()
        .min_h_0()
        .gap(rems(1. / 16.))
        .p(rems(6. / 16.));

    for endpoint in &visible {
        let method_color = match endpoint.http_methods.first().map(String::as_str) {
            Some("GET") => cx.theme().colors.success,
            Some("POST") => cx.theme().colors.info,
            Some("DELETE") => cx.theme().colors.danger,
            _ => cx.theme().colors.warning,
        };
        column = column.child(
            h_stack()
                .w_full()
                .gap(rems(9. / 16.))
                .px(rems(10. / 16.))
                .min_h(rems(30. / 16.))
                .child(
                    div()
                        .flex_shrink_0()
                        .w(rems(58. / 16.))
                        .text_xs()
                        .font_weight(gpui::FontWeight::BOLD)
                        .font_family(mono.clone())
                        .text_color(method_color)
                        .child(endpoint.methods_label()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_sm()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .font_family(mono.clone())
                        .text_color(cx.theme().colors.fg)
                        .child(endpoint.route.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(format!("{}.{}", endpoint.controller, endpoint.method)),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(endpoint.path.clone()),
                ),
        );
    }

    column = column.child(
        div()
            .w_full()
            .mt_2()
            .text_xs()
            .text_color(cx.theme().colors.fg_muted)
            .child(format!(
                "properties {} · values {} · refs {} · injections {} · beans {} · diagnostics {}",
                index.properties,
                index.values,
                index.property_references,
                index.injections,
                index.beans,
                index.diagnostics
            )),
    );

    column.into_any_element()
}

fn maven_content(project: &crate::maven::MavenProjectView, cx: &App) -> AnyElement {
    let mut column = v_stack()
        .id("maven-project")
        .w_full()
        .flex_1()
        .min_h_0()
        .gap_1()
        .p_3();

    column = column.child(
        h_stack()
            .w_full()
            .gap_2()
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(cx.theme().colors.fg)
                    .child(project.artifact_id.clone()),
            )
            .children(project.version.clone().map(|version| {
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(version)
            })),
    );
    column = column.child(
        div()
            .w_full()
            .text_xs()
            .truncate()
            .text_color(cx.theme().colors.fg_muted)
            .child(project.relative_path.clone()),
    );

    for module in &project.modules {
        column = column.child(maven_module_row(module, cx));
    }

    if !project.profiles.is_empty() {
        let active = &project.active_profiles;
        column = column.child(v_stack().w_full().mt_2().gap_1().children(
            project.profiles.iter().map(|id| {
                let is_active = active.contains(id);
                div()
                    .w_full()
                    .truncate()
                    .text_xs()
                    .when(is_active, |this| {
                        this.text_color(cx.theme().colors.info)
                            .font_weight(gpui::FontWeight::MEDIUM)
                    })
                    .when(!is_active, |this| {
                        this.text_color(cx.theme().colors.fg_muted)
                    })
                    .child(id.clone())
            }),
        ));
    }

    column.into_any_element()
}

fn maven_module_row(module: &crate::maven::MavenModuleView, cx: &App) -> AnyElement {
    let mut column = v_stack().w_full().gap_1();
    column = column.child(
        h_stack()
            .w_full()
            .gap_2()
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_xs()
                    .text_color(cx.theme().colors.fg)
                    .child(module.label().to_string()),
            )
            .children(module.version.clone().map(|version| {
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(version)
            })),
    );
    for root in &module.source_roots {
        column = column.child(
            h_stack()
                .w_full()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(root.path.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(root.kind.clone()),
                ),
        );
    }
    if !module.modules.is_empty() {
        column = column.child(
            v_stack().w_full().pl_3().gap_1().children(
                module
                    .modules
                    .iter()
                    .map(|child| maven_module_row(child, cx)),
            ),
        );
    }
    column.into_any_element()
}

fn header(
    view: RightToolWindowView,
    title: SharedString,
    subtitle: Option<SharedString>,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let close_label = tr("lithe.commandPalette.close");
    h_stack()
        .w_full()
        .flex_shrink_0()
        .h(rems(TOOL_WINDOW_HEADER_HEIGHT_SPEC / 16.))
        .gap_2()
        .pl_3()
        .pr(rems(TOOL_WINDOW_HEADER_TRAILING_INSET_SPEC / 16.))
        .border_b_1()
        .border_color(cx.theme().colors.border)
        .child(view.icon(cx, rems(TOOL_WINDOW_HEADER_ICON_SPEC / 16.)))
        .child(
            h_stack()
                .flex_1()
                .min_w_0()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(cx.theme().colors.fg)
                        .child(title),
                )
                .children(subtitle.map(|subtitle| {
                    div()
                        .flex_shrink_0()
                        .truncate()
                        .text_xs()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(cx.theme().colors.fg_muted)
                        .child(subtitle)
                })),
        )
        .child(
            Button::new("right-tool-window-close")
                .variant(ButtonVariant::Ghost)
                // 关闭用的叉来自**本仓**图标集（`icons/idea/**`）。
                .icon_element(idea_icon_mono(&idea::X_ICON, cx.theme().is_dark()))
                // 键盘 Tab 不落在这个关闭键上（原来的 `.tab_stop(false)`）。
                .tab_stop(false)
                // 24pt 方形：库的 `Size::Small` 档就是 24pt（`control_height`）。
                .size(Size::Small)
                .tooltip(close_label.clone())
                // 图标键自己没有可见文字，可访问名只能靠 aria_label 给。
                .aria_label(close_label)
                .on_click(on_close),
        )
        .into_any_element()
}

fn empty_state(view: RightToolWindowView, cx: &App) -> AnyElement {
    lithe_gpui_shared::empty_state::unavailable(
        Some(view.icon(
            cx,
            rems(lithe_gpui_shared::empty_state::UNAVAILABLE_ICON_SIZE_SPEC / 16.),
        )),
        view.empty_title(),
        None,
        None,
        cx,
    )
}

#[cfg(test)]
mod tests {
    use super::{ProbePoint, RightToolWindowView, diagnose_line, resolve_click};

    #[test]
    fn rail_index_maps_to_view() {
        assert_eq!(RightToolWindowView::from_rail_index(0), None);
        assert_eq!(
            RightToolWindowView::from_rail_index(1),
            Some(RightToolWindowView::Notifications)
        );
        assert_eq!(
            RightToolWindowView::from_rail_index(2),
            Some(RightToolWindowView::Maven)
        );
        assert_eq!(RightToolWindowView::from_rail_index(3), None);
    }

    #[test]
    fn view_ids_are_probe_tokens() {
        assert_eq!(RightToolWindowView::Notifications.id(), "notifications");
        assert_eq!(RightToolWindowView::Maven.id(), "maven");
        assert_eq!(RightToolWindowView::Spring.id(), "spring");
    }

    #[test]
    fn id_round_trips_through_from_id() {
        for view in [
            RightToolWindowView::Notifications,
            RightToolWindowView::Maven,
            RightToolWindowView::Spring,
        ] {
            assert_eq!(RightToolWindowView::from_id(view.id()), Some(view));
        }
        assert_eq!(
            RightToolWindowView::from_id("notifications"),
            Some(RightToolWindowView::Notifications)
        );
        assert_eq!(
            RightToolWindowView::from_id("maven"),
            Some(RightToolWindowView::Maven)
        );
        assert_eq!(
            RightToolWindowView::from_id("spring"),
            Some(RightToolWindowView::Spring)
        );
    }

    #[test]
    fn unknown_id_has_no_view() {
        assert_eq!(RightToolWindowView::from_id(""), None);
        assert_eq!(RightToolWindowView::from_id("extensions"), None);
        assert_eq!(RightToolWindowView::from_id("Maven"), None);
        assert_eq!(RightToolWindowView::from_id("outline"), None);
        assert_eq!(RightToolWindowView::from_id("mavne"), None);
        assert_eq!(RightToolWindowView::from_id("spring "), None);
    }

    #[test]
    fn clicking_the_visible_view_closes_it() {
        let (view, visible) =
            resolve_click(RightToolWindowView::Maven, RightToolWindowView::Maven, true);
        assert_eq!(view, RightToolWindowView::Maven);
        assert!(!visible);
    }

    #[test]
    fn clicking_a_hidden_view_shows_it() {
        let (view, visible) = resolve_click(
            RightToolWindowView::Maven,
            RightToolWindowView::Maven,
            false,
        );
        assert_eq!(view, RightToolWindowView::Maven);
        assert!(visible);
    }

    #[test]
    fn clicking_another_view_switches_and_keeps_visible() {
        let (view, visible) = resolve_click(
            RightToolWindowView::Notifications,
            RightToolWindowView::Maven,
            true,
        );
        assert_eq!(view, RightToolWindowView::Notifications);
        assert!(visible);

        let (view, visible) = resolve_click(
            RightToolWindowView::Maven,
            RightToolWindowView::Notifications,
            false,
        );
        assert_eq!(view, RightToolWindowView::Maven);
        assert!(visible);
    }

    #[test]
    fn diagnose_line_carries_seq_timestamp_and_optional_point() {
        assert_eq!(
            diagnose_line(
                RightToolWindowView::Spring,
                true,
                Some(ProbePoint::new(1421.6, 123.2)),
                7,
                8123,
            ),
            "S1_RIGHT_PANEL seq=7 t_ms=8123 view=spring visible=true x=1421.6 y=123.2"
        );
        assert_eq!(
            diagnose_line(RightToolWindowView::Maven, false, None, 8, 9001),
            "S1_RIGHT_PANEL seq=8 t_ms=9001 view=maven visible=false"
        );
    }

    #[test]
    fn probe_point_keeps_logical_pixels_unscaled() {
        use gpui::{Point, px};

        let point = ProbePoint::from(Point {
            x: px(1421.6),
            y: px(123.2),
        });
        assert_eq!(point, ProbePoint::new(1421.6, 123.2));
    }
}
