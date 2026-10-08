mod view;

use std::rc::Rc;

use gpui::{AnyElement, App, Context, EventEmitter, Pixels, Rems, SharedString, Window};
use serde::{Deserialize, Serialize};

use crate::{primitives::IconName, theme::ActiveTheme};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl DockSide {
    const ALL: [DockSide; 4] = [
        DockSide::Left,
        DockSide::Right,
        DockSide::Top,
        DockSide::Bottom,
    ];

    fn index(self) -> usize {
        self as usize
    }
}

/// Where a panel lives: docked on a side, or floating at a window position.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Spot {
    Docked(DockSide),
    Floating { x: f32, y: f32 },
}

/// How far a side's size may go. `max` of `None` keeps the cap the drag applies
/// (a little under half the dock); a side that must not grow past a fixed height
/// — a terminal, say — sets one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SideLimits {
    pub min: Rems,
    pub max: Option<Rems>,
    /// Space the side may not eat, kept for the center: the cap becomes
    /// `extent - reserve` whenever that is tighter than `max`. An editor leaves
    /// the editor column a minimum width this way.
    pub reserve: Option<Rems>,
}

impl SideLimits {
    /// `size` brought inside this range; `cap` is the fallback a `None` max uses.
    pub fn clamp(&self, size: Rems, cap: Rems) -> Rems {
        // A max under the min would make the range empty; the min wins.
        let max = self.max.unwrap_or(cap).0.max(self.min.0);
        Rems(size.0.max(self.min.0).min(max))
    }

    /// `size` brought inside this range for a dock `extent` along the side's
    /// axis — what [`Self::cap`] allows, and never under the minimum.
    pub fn clamp_in(&self, size: Rems, extent: Rems, fallback: Rems, rem: Pixels) -> Rems {
        let cap = self.cap(extent, fallback, rem);
        Rems(size.0.max(self.min.0).min(cap.0))
    }

    /// The cap for a dock `extent` along this side's axis: the tighter of `max`
    /// and what `reserve` leaves. Never under the minimum.
    pub fn cap(&self, extent: Rems, fallback: Rems, rem: Pixels) -> Rems {
        let mut cap = self.max.unwrap_or(fallback);
        if let Some(reserve) = self.reserve {
            let left =
                Rems(f32::from(extent.to_pixels(rem) - reserve.to_pixels(rem)) / f32::from(rem));
            if left.0 < cap.0 {
                cap = left;
            }
        }
        Rems(cap.0.max(self.min.0))
    }
}

/// A panel's heading: fixed text, or a catalogue key resolved as it is drawn.
///
/// The key form follows the shown locale, which is what an application wants for
/// anything a person reads; `Text` is for a name that is already final.
#[derive(Clone, Debug, PartialEq)]
pub enum PanelTitle {
    Text(SharedString),
    Key(SharedString),
}

impl From<SharedString> for PanelTitle {
    fn from(title: SharedString) -> Self {
        Self::Text(title)
    }
}

impl From<&str> for PanelTitle {
    fn from(title: &str) -> Self {
        Self::Text(title.into())
    }
}

impl PanelTitle {
    /// The heading to draw: straight through for text, the catalogue's copy for a key.
    pub fn resolve(&self, cx: &App) -> SharedString {
        match self {
            Self::Text(title) => title.clone(),
            Self::Key(key) => crate::i18n::text(cx, key, &[]),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DockPanel {
    pub id: SharedString,
    pub title: PanelTitle,
    pub icon: IconName,
    /// Whether the side draws this panel's tab strip. An application whose own
    /// chrome already names the panel turns it off; the divider is then the only
    /// handle.
    pub header: bool,
}

impl DockPanel {
    /// A panel that draws its own tab strip.
    pub fn new(id: impl Into<SharedString>, title: PanelTitle, icon: IconName) -> Self {
        Self {
            id: id.into(),
            title,
            icon,
            header: true,
        }
    }

    /// A panel the application's own chrome already announces: its side draws no
    /// tab strip, so the divider is the only way to size it and the application's
    /// own control the way to close it.
    pub fn plain(id: impl Into<SharedString>, title: impl Into<PanelTitle>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: IconName::Square,
            header: false,
        }
    }
}

/// Saved dock arrangement, per side in `DockSide` order. Sizes are in rems.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DockLayout {
    pub spots: Vec<(String, Spot)>,
    /// Per side: the panel that side shows.
    pub active: [Option<String>; 4],
    pub sizes: [f32; 4],
    /// Panels that are closed but still docked, so they can be opened again.
    #[serde(default)]
    pub hidden: Vec<String>,
}

impl EventEmitter<DockEvent> for Dock {}

type Center = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
type Content = Rc<dyn Fn(&SharedString, &mut Window, &mut App) -> AnyElement>;

/// What a dock reports once a person has changed it, so an application can save
/// the arrangement. A programmatic change — `set_size`, `restore` — does not
/// emit: the caller already knows.
#[derive(Clone, Debug, PartialEq)]
pub enum DockEvent {
    /// A divider was dragged: the side is now this many rems.
    Resized { side: DockSide, size: Rems },
}

/// Side docks around a center; panels move between docks, float, hide and show.
pub struct Dock {
    panels: Vec<DockPanel>,
    spots: Vec<Spot>,
    /// Closed but still docked: `hidden[ix]` matches `panels[ix]`.
    hidden: Vec<bool>,
    active: [Option<usize>; 4],
    sizes: [Rems; 4],
    limits: [SideLimits; 4],
    center: Center,
    content: Content,
    dragging: bool,
}

impl Dock {
    pub fn new(
        center: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        content: impl Fn(&SharedString, &mut Window, &mut App) -> AnyElement + 'static,
        cx: &App,
    ) -> Self {
        let theme = cx.theme();
        let (side, strip) = (theme.sidebar_width(false), theme.pane_min());
        let limits = SideLimits {
            min: theme.pane_min(),
            max: None,
            reserve: None,
        };
        Self {
            panels: Vec::new(),
            spots: Vec::new(),
            hidden: Vec::new(),
            active: [None; 4],
            sizes: [side, side, strip, strip],
            limits: [limits; 4],
            center: Rc::new(center),
            content: Rc::new(content),
            dragging: false,
        }
    }

    /// Panics on a repeated id.
    pub fn add(&mut self, panel: DockPanel, side: DockSide) {
        assert!(
            self.find(&panel.id).is_none(),
            "dock panel {} added twice",
            panel.id
        );
        self.panels.push(panel);
        self.spots.push(Spot::Docked(side));
        self.hidden.push(false);
        let slot = &mut self.active[side.index()];
        if slot.is_none() {
            *slot = Some(self.panels.len() - 1);
        }
    }

    fn find(&self, id: &str) -> Option<usize> {
        self.panels.iter().position(|panel| panel.id.as_ref() == id)
    }

    /// The panels docked on `side`, closed ones included.
    fn on_side(&self, side: DockSide) -> Vec<usize> {
        (0..self.panels.len())
            .filter(|ix| self.spots[*ix] == Spot::Docked(side))
            .collect()
    }

    /// The panels a side shows: docked there and not closed.
    fn shown_in(&self, side: DockSide) -> Vec<usize> {
        self.on_side(side)
            .into_iter()
            .filter(|ix| !self.hidden[*ix])
            .collect()
    }

    /// The range a side's size may take. A size already outside it is brought in.
    pub fn set_side_limits(
        &mut self,
        side: DockSide,
        min: Rems,
        max: Option<Rems>,
        cx: &mut Context<Self>,
    ) {
        self.set_side_limits_with(side, min, max, None, cx);
    }

    /// [`Self::set_side_limits`] plus the space this side must leave the center.
    pub fn set_side_limits_with(
        &mut self,
        side: DockSide,
        min: Rems,
        max: Option<Rems>,
        reserve: Option<Rems>,
        cx: &mut Context<Self>,
    ) {
        self.limits[side.index()] = SideLimits { min, max, reserve };
        let cap = Rems(f32::MAX);
        self.sizes[side.index()] = self.limits[side.index()].clamp(self.sizes[side.index()], cap);
        cx.notify();
    }

    /// How wide (or tall) a side is now, in rems.
    pub fn size(&self, side: DockSide) -> Rems {
        self.sizes[side.index()]
    }

    /// Sets a side's size from outside — a window restoring a saved layout, or an
    /// application writing back a clamped value. Brought inside the side's limits;
    /// does not emit [`DockEvent::Resized`], which is for a person's drag.
    pub fn set_size(&mut self, side: DockSide, size: Rems, cx: &mut Context<Self>) {
        let cap = Rems(f32::MAX);
        let next = self.limits[side.index()].clamp(size, cap);
        if next != self.sizes[side.index()] {
            self.sizes[side.index()] = next;
            cx.notify();
        }
    }

    /// The dock's own limits for a side, so a caller can clamp what it stores.
    pub fn side_limits(&self, side: DockSide) -> SideLimits {
        self.limits[side.index()]
    }

    pub fn is_hidden(&self, id: &str) -> bool {
        self.find(id).is_some_and(|ix| self.hidden[ix])
    }

    /// Closes a panel without moving it, so [`Dock::show`] brings it back where it was.
    pub fn hide(&mut self, id: &str, cx: &mut Context<Self>) {
        let ix = self
            .find(id)
            .unwrap_or_else(|| panic!("no dock panel {id}"));
        if self.hidden[ix] {
            return;
        }
        self.hidden[ix] = true;
        self.reselect(ix);
        log::info!("dock: {id} hidden");
        cx.notify();
    }

    /// Hands a side to another of its panels when `ix` leaves it.
    fn reselect(&mut self, ix: usize) {
        if let Spot::Docked(side) = self.spots[ix] {
            if self.active[side.index()] == Some(ix) {
                self.active[side.index()] = self.shown_in(side).first().copied();
            }
        }
    }

    /// Opens a closed panel. Showing a panel that is already open does nothing.
    pub fn show(&mut self, id: &str, cx: &mut Context<Self>) {
        let ix = self
            .find(id)
            .unwrap_or_else(|| panic!("no dock panel {id}"));
        if !self.hidden[ix] {
            return;
        }
        self.hidden[ix] = false;
        if let Spot::Docked(side) = self.spots[ix] {
            self.active[side.index()] = Some(ix);
        }
        log::info!("dock: {id} shown");
        cx.notify();
    }

    /// Opens a closed panel, closes an open one.
    pub fn toggle(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.is_hidden(id) {
            self.show(id, cx);
        } else {
            self.hide(id, cx);
        }
    }

    pub fn activate(&mut self, id: &str, cx: &mut Context<Self>) {
        let ix = self
            .find(id)
            .unwrap_or_else(|| panic!("no dock panel {id}"));
        // Activating a closed panel opens it: a caller asking to see it means see it.
        self.hidden[ix] = false;
        if let Spot::Docked(side) = self.spots[ix] {
            self.active[side.index()] = Some(ix);
            cx.notify();
        }
    }

    /// Moves a panel to a side, or sends it floating.
    pub fn move_panel(&mut self, id: &str, spot: Spot, cx: &mut Context<Self>) {
        let ix = self
            .find(id)
            .unwrap_or_else(|| panic!("no dock panel {id}"));
        let from = self.spots[ix];
        self.spots[ix] = spot;
        // A move shows the panel: dropping a tab somewhere means wanting it there.
        self.hidden[ix] = false;
        self.reselect(ix);
        if let Spot::Docked(side) = spot {
            self.active[side.index()] = Some(ix);
        }
        log::info!("dock: {id} {from:?} -> {spot:?}");
        cx.notify();
    }

    pub fn layout(&self) -> DockLayout {
        DockLayout {
            spots: self
                .panels
                .iter()
                .zip(&self.spots)
                .map(|(panel, spot)| (panel.id.to_string(), *spot))
                .collect(),
            active: std::array::from_fn(|side| {
                self.active[side].map(|ix| self.panels[ix].id.to_string())
            }),
            sizes: self.sizes.map(|size| size.0),
            hidden: self
                .panels
                .iter()
                .zip(&self.hidden)
                .filter(|(_, hidden)| **hidden)
                .map(|(panel, _)| panel.id.to_string())
                .collect(),
        }
    }

    /// All or nothing. A side the layout leaves unset shows its first open panel.
    pub fn restore(&mut self, layout: &DockLayout, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let mut named: Vec<&str> = layout.spots.iter().map(|(id, _)| id.as_str()).collect();
        named.sort_unstable();
        named.dedup();
        anyhow::ensure!(
            named.len() == layout.spots.len(),
            "layout places a panel twice"
        );
        anyhow::ensure!(
            layout
                .sizes
                .iter()
                .all(|size| size.is_finite() && *size > 0.0),
            "dock sizes {:?} must be finite and positive",
            layout.sizes
        );
        anyhow::ensure!(
            layout.spots.iter().all(|(_, spot)| match spot {
                Spot::Docked(_) => true,
                Spot::Floating { x, y } => x.is_finite() && y.is_finite(),
            }),
            "floating positions must be finite"
        );
        let unknown: Vec<&str> = layout
            .spots
            .iter()
            .map(|(id, _)| id.as_str())
            .chain(layout.active.iter().flatten().map(String::as_str))
            .chain(layout.hidden.iter().map(String::as_str))
            .filter(|id| self.find(id).is_none())
            .collect();
        anyhow::ensure!(
            unknown.is_empty(),
            "layout names unknown panels: {unknown:?}"
        );
        let mut spots = self.spots.clone();
        for (id, spot) in &layout.spots {
            spots[self.find(id).expect("checked above")] = *spot;
        }
        let mut hidden = vec![false; self.panels.len()];
        for id in &layout.hidden {
            hidden[self.find(id).expect("checked above")] = true;
        }
        let mut active = [None; 4];
        for side in DockSide::ALL {
            let chosen = layout.active[side.index()]
                .as_deref()
                .and_then(|id| self.find(id));
            if let Some(ix) = chosen {
                anyhow::ensure!(
                    spots[ix] == Spot::Docked(side),
                    "active panel {} is not docked {side:?}",
                    self.panels[ix].id
                );
            }
            // A closed panel cannot be the one a side shows.
            active[side.index()] = chosen.filter(|ix| !hidden[*ix]).or_else(|| {
                (0..spots.len()).find(|ix| spots[*ix] == Spot::Docked(side) && !hidden[*ix])
            });
        }
        self.spots = spots;
        self.hidden = hidden;
        self.active = active;
        self.sizes = layout.sizes.map(gpui::rems);
        log::info!("dock: restored {} panel spots", layout.spots.len());
        cx.notify();
        Ok(())
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use gpui::{AnyElement, Entity, TestAppContext, div, prelude::*, px, rems};

    use super::*;

    use crate::theme::Theme;

    fn blank() -> AnyElement {
        div().into_any_element()
    }

    fn panel(id: &str) -> DockPanel {
        DockPanel::new(id, PanelTitle::Text(id.into()), IconName::Folder)
    }

    /// `files` and `outline` on the left, `terminal` below.
    fn bench(cx: &mut TestAppContext) -> Entity<Dock> {
        cx.update(|cx| {
            Theme::init(cx);
            cx.new(|cx| {
                let mut dock = Dock::new(|_, _| blank(), |_, _, _| blank(), cx);
                dock.add(panel("files"), DockSide::Left);
                dock.add(panel("outline"), DockSide::Left);
                dock.add(panel("terminal"), DockSide::Bottom);
                dock
            })
        })
    }

    fn at(dock: &Dock, side: DockSide) -> Option<usize> {
        dock.active[side.index()]
    }

    /// A reserve keeps the center's share: the cap follows the extent, and the
    /// minimum still wins when the space left is smaller than it.
    #[gpui::test]
    fn a_reserve_caps_the_side_against_the_extent(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            let rem = px(16.);
            let limits = SideLimits {
                min: rems(3.),
                max: Some(rems(32.)),
                reserve: Some(rems(25.)),
            };
            // A wide dock: the fixed maximum is the tighter of the two.
            assert_eq!(limits.cap(rems(100.), rems(45.), rem), rems(32.));
            // A narrow dock: what the reserve leaves is tighter.
            assert_eq!(limits.cap(rems(40.), rems(45.), rem), rems(15.));
            // Too narrow for either: the minimum wins over an empty range.
            assert_eq!(limits.cap(rems(10.), rems(45.), rem), rems(3.));
        });
    }

    /// A dock draws its center and its panels; a `plain` panel draws no tab
    /// strip, so its body starts at the panel's own top — which is how an editor
    /// shell uses it (its own chrome names the panel).
    #[gpui::test]
    fn a_plain_panel_draws_its_body_without_a_header(cx: &mut TestAppContext) {
        struct Shell {
            dock: Entity<Dock>,
        }

        impl Render for Shell {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                self.dock.clone()
            }
        }

        cx.update(|cx| Theme::init(cx));
        let dock = cx.update(|cx| {
            cx.new(|cx| {
                let mut dock = Dock::new(
                    |_, _| {
                        div()
                            .debug_selector(|| "dock-center-mark".to_string())
                            .into_any_element()
                    },
                    |id, _, _| {
                        div()
                            .debug_selector(move || format!("dock-body-{id}"))
                            .into_any_element()
                    },
                    cx,
                );
                dock.add(DockPanel::plain("plain", "Plain"), DockSide::Left);
                dock.add(
                    DockPanel::new("named", PanelTitle::Text("Named".into()), IconName::Folder),
                    DockSide::Right,
                );
                dock
            })
        });
        let (_view, cx) = cx.add_window_view(|_, _| Shell { dock });
        cx.run_until_parked();

        assert!(
            cx.debug_bounds("dock-center-mark").is_some(),
            "the center is drawn"
        );
        let plain = cx
            .debug_bounds("dock-body-plain")
            .expect("plain body drawn");
        let named = cx
            .debug_bounds("dock-body-named")
            .expect("named body drawn");
        // 两块面板都在 dock 顶部起，所以"body 从多高处开始"就是它上面有没有标签头。
        assert!(
            named.top() > plain.top(),
            "a named panel keeps a tab strip above its body ({named:?} vs {plain:?})"
        );
        assert_eq!(
            plain.top(),
            cx.debug_bounds("dock-center-mark")
                .expect("center drawn")
                .top(),
            "a plain panel's body starts where the center does — no header above it"
        );
    }

    /// Sizes are readable and writable from outside, and a write is clamped —
    /// this is how an application restores a saved arrangement.
    #[gpui::test]
    fn a_size_can_be_written_and_read_back(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            dock.set_side_limits(DockSide::Left, rems(3.), Some(rems(20.)), cx);
            dock.set_size(DockSide::Left, rems(12.), cx);
            assert_eq!(dock.size(DockSide::Left), rems(12.), "written");
            dock.set_size(DockSide::Left, rems(40.), cx);
            assert_eq!(dock.size(DockSide::Left), rems(20.), "clamped to the max");
            dock.set_size(DockSide::Left, rems(1.), cx);
            assert_eq!(dock.size(DockSide::Left), rems(3.), "clamped to the min");
        });
    }

    /// A side's limit brings a size that sits outside it back into range, and a
    /// maximum below the minimum still leaves room for the minimum.
    #[gpui::test]
    fn a_side_limit_brings_the_size_into_range(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            let left = DockSide::Left.index();
            dock.sizes[left] = rems(40.0);
            dock.set_side_limits(DockSide::Left, rems(10.0), Some(rems(20.0)), cx);
            assert_eq!(dock.sizes[left], rems(20.0), "brought down to the maximum");
            dock.set_side_limits(DockSide::Left, rems(30.0), Some(rems(5.0)), cx);
            assert_eq!(dock.sizes[left], rems(30.0), "the minimum wins");
        });
    }

    /// Closing the panel a side shows hands the side to another open one on it,
    /// and showing it again takes the side back.
    #[gpui::test]
    fn hiding_a_panel_hands_the_side_to_another(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            assert_eq!(at(dock, DockSide::Left), Some(0), "files leads");
            dock.hide("files", cx);
            assert!(dock.is_hidden("files"));
            assert_eq!(at(dock, DockSide::Left), Some(1), "outline takes over");
            dock.show("files", cx);
            assert_eq!(at(dock, DockSide::Left), Some(0), "files is back");
        });
    }

    /// Toggling closes an open panel and opens a closed one.
    #[gpui::test]
    fn toggling_flips_a_panel(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            dock.toggle("terminal", cx);
            assert!(dock.is_hidden("terminal"));
            dock.toggle("terminal", cx);
            assert!(!dock.is_hidden("terminal"));
        });
    }

    /// The layout carries what is closed; a closed panel is never the one a side
    /// shows — including right after restoring.
    #[gpui::test]
    fn a_layout_carries_what_is_closed(cx: &mut TestAppContext) {
        let dock = bench(cx);
        let layout = dock.update(cx, |dock, cx| {
            dock.hide("outline", cx);
            dock.layout()
        });
        assert_eq!(layout.hidden, ["outline"]);
        dock.update(cx, |dock, cx| {
            dock.restore(&layout, cx).expect("a layout it just wrote");
            assert!(dock.is_hidden("outline"));
            assert_eq!(
                at(dock, DockSide::Left),
                Some(0),
                "a closed panel never leads"
            );
        });
    }

    /// Activating and moving both mean "show me this", so a closed panel opens.
    #[gpui::test]
    fn asking_for_a_closed_panel_opens_it(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            dock.hide("terminal", cx);
            dock.activate("terminal", cx);
            assert!(!dock.is_hidden("terminal"));
            dock.hide("terminal", cx);
            dock.move_panel("terminal", Spot::Floating { x: 40.0, y: 60.0 }, cx);
            assert!(!dock.is_hidden("terminal"));
            assert!(matches!(dock.spots[2], Spot::Floating { .. }));
            dock.move_panel("terminal", Spot::Docked(DockSide::Bottom), cx);
            assert_eq!(at(dock, DockSide::Bottom), Some(2));
        });
    }
}
