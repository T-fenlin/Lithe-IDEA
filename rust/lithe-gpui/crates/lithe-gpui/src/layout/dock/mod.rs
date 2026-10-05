mod view;

use std::rc::Rc;

use gpui::{AnyElement, App, Context, IntoElement as _, Rems, SharedString, Window};
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

/// Where a panel lives: a dock, or floating at a window position.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Spot {
    Docked(DockSide),
    Floating { x: f32, y: f32 },
}

/// How far a side.s size may go. `max` of `None` keeps the cap the drag applies
/// (a little under half the dock); a side that must not grow past a fixed height
/// — a terminal, say — sets one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SideLimits {
    pub min: Rems,
    pub max: Option<Rems>,
}

impl SideLimits {
    /// `size` brought inside this range; `cap` is the fallback a `None` max uses.
    pub fn clamp(&self, size: Rems, cap: Rems) -> Rems {
        // A max under the min would make the range empty; the min wins.
        let max = self.max.unwrap_or(cap).0.max(self.min.0);
        Rems(size.0.max(self.min.0).min(max))
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
}

/// Saved dock arrangement, per side in `DockSide` order. Sizes are in rems.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DockLayout {
    pub spots: Vec<(String, Spot)>,
    pub active: [Option<String>; 4],
    pub sizes: [f32; 4],
    /// Panels that are closed but still docked, so they can be opened again.
    #[serde(default)]
    pub hidden: Vec<String>,
}

/// The views a dock shows, by panel id.
///
/// This is what keeps a shell from growing one `match` over every panel it owns:
/// adding a panel becomes one `register` call beside that panel, and the
/// dock asks the registry for whatever it is about to draw.
#[derive(Default)]
pub struct PanelRegistry {
    views: std::collections::HashMap<SharedString, gpui::AnyView>,
}

impl PanelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows `view` for the panel called `id`; a later call for the same id wins.
    pub fn register(&mut self, id: impl Into<SharedString>, view: impl Into<gpui::AnyView>) {
        self.views.insert(id.into(), view.into());
    }

    /// The ids that have a view, for a test or a diagnostic.
    pub fn ids(&self) -> impl Iterator<Item = &SharedString> {
        self.views.keys()
    }

    /// Draws `id`. `None` means the panel is docked but nothing draws it.
    pub fn render(&self, id: &str) -> Option<AnyElement> {
        self.views
            .get(id)
            .map(|view| view.clone().into_any_element())
    }
}

type Center = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
type Content = Rc<dyn Fn(&SharedString, &mut Window, &mut App) -> AnyElement>;

/// Side docks around a center; panels move between docks, float, hide and show.
pub struct Dock {
    panels: Vec<DockPanel>,
    spots: Vec<Spot>,
    /// Closed but still docked: `hidden[ix]` matches `panels[ix]`.
    hidden: Vec<bool>,
    active: [Option<usize>; 4],
    /// The panel filling the dock on its own, until it is restored.
    zoomed: Option<usize>,
    /// The side each panel was last docked to, so a floated panel has a way home.
    last_docked: Vec<DockSide>,
    /// Whether a strip of closed panels is drawn, so they can be opened again.
    /// An application with an activity bar turns this off: a toggle there is the
    /// usual way back, and a second one would be noise.
    closed_strip: bool,
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
        };
        Self {
            panels: Vec::new(),
            spots: Vec::new(),
            hidden: Vec::new(),
            active: [None; 4],
            zoomed: None,
            last_docked: Vec::new(),
            closed_strip: false,
            sizes: [side, side, strip, strip],
            limits: [limits; 4],
            center: Rc::new(center),
            content: Rc::new(content),
            dragging: false,
        }
    }

    /// A dock whose panel views come from a registry, so the caller never writes a
    /// match over ids. A panel with no registered view draws empty, and says so once.
    pub fn from_registry(
        registry: PanelRegistry,
        center: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        cx: &App,
    ) -> Self {
        Self::new(
            center,
            move |id, _, _| {
                registry.render(id).unwrap_or_else(|| {
                    log::warn!("dock: no view registered for panel {id}");
                    gpui::div().into_any_element()
                })
            },
            cx,
        )
    }

    /// Draws a strip of closed panels, each a button that opens it again.
    /// Off by default.
    pub fn closed_strip(mut self, on: bool) -> Self {
        self.closed_strip = on;
        self
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
        self.last_docked.push(side);
        let slot = &mut self.active[side.index()];
        if slot.is_none() {
            *slot = Some(self.panels.len() - 1);
        }
    }

    fn find(&self, id: &str) -> Option<usize> {
        self.panels.iter().position(|panel| panel.id.as_ref() == id)
    }

    /// The range a side.s size may take. A size already outside it is brought in.
    pub fn set_side_limits(
        &mut self,
        side: DockSide,
        min: Rems,
        max: Option<Rems>,
        cx: &mut Context<Self>,
    ) {
        self.limits[side.index()] = SideLimits { min, max };
        let cap = Rems(f32::MAX);
        self.sizes[side.index()] = self.limits[side.index()].clamp(self.sizes[side.index()], cap);
        cx.notify();
    }

    fn on_side(&self, side: DockSide) -> Vec<usize> {
        (0..self.panels.len())
            .filter(|ix| self.spots[*ix] == Spot::Docked(side))
            .collect()
    }

    /// The panels a side shows: docked there and not closed.
    fn shown_on_side(&self, side: DockSide) -> Vec<usize> {
        self.on_side(side)
            .into_iter()
            .filter(|ix| !self.hidden[*ix])
            .collect()
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
        if let Spot::Docked(side) = self.spots[ix]
            && self.active[side.index()] == Some(ix)
        {
            self.active[side.index()] = self.shown_on_side(side).first().copied();
        }
        log::info!("dock: {id} hidden");
        cx.notify();
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

    pub fn move_panel(&mut self, id: &str, spot: Spot, cx: &mut Context<Self>) {
        let ix = self
            .find(id)
            .unwrap_or_else(|| panic!("no dock panel {id}"));
        let from = self.spots[ix];
        self.spots[ix] = spot;
        // A move shows the panel: dropping a tab somewhere means wanting it there.
        self.hidden[ix] = false;
        if let Spot::Docked(side) = from
            && self.active[side.index()] == Some(ix)
        {
            self.active[side.index()] = self.shown_on_side(side).first().copied();
        }
        if let Spot::Docked(side) = spot {
            self.last_docked[ix] = side;
            self.active[side.index()] = Some(ix);
        }
        log::info!("dock: {id} {from:?} -> {spot:?}");
        cx.notify();
    }

    /// Whether `id` is the panel filling the dock on its own.
    pub fn is_zoomed(&self, id: &str) -> bool {
        self.zoomed
            .is_some_and(|ix| self.panels[ix].id.as_ref() == id)
    }

    /// Fills the dock with one panel; a closed panel is opened first.
    pub fn zoom(&mut self, id: &str, cx: &mut Context<Self>) {
        let ix = self
            .find(id)
            .unwrap_or_else(|| panic!("no dock panel {id}"));
        self.hidden[ix] = false;
        self.zoomed = Some(ix);
        log::info!("dock: {id} zoomed");
        cx.notify();
    }

    /// Gives the dock back to its arrangement.
    pub fn unzoom(&mut self, cx: &mut Context<Self>) {
        if self.zoomed.take().is_some() {
            log::info!("dock: restored from zoom");
            cx.notify();
        }
    }

    /// Sends a docked panel afloat, and a floating one back where it was docked.
    pub fn toggle_float(&mut self, id: &str, cx: &mut Context<Self>) {
        let ix = self
            .find(id)
            .unwrap_or_else(|| panic!("no dock panel {id}"));
        let spot = match self.spots[ix] {
            Spot::Docked(_) => Spot::Floating { x: 160.0, y: 120.0 },
            Spot::Floating { .. } => Spot::Docked(self.last_docked[ix]),
        };
        self.move_panel(id, spot, cx);
    }

    /// Zooms the panel, or restores the dock when it is the zoomed one.
    pub fn toggle_zoom(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.is_zoomed(id) {
            self.unzoom(cx);
        } else {
            self.zoom(id, cx);
        }
    }

    pub fn layout(&self) -> DockLayout {
        let name = |ix: Option<usize>| ix.map(|ix| self.panels[ix].id.to_string());
        DockLayout {
            spots: self
                .panels
                .iter()
                .zip(&self.spots)
                .map(|(panel, spot)| (panel.id.to_string(), *spot))
                .collect(),
            active: DockSide::ALL.map(|side| name(self.active[side.index()])),
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
        let mut last_docked = vec![DockSide::Left; self.panels.len()];
        for (ix, spot) in spots.iter().enumerate() {
            if let Spot::Docked(side) = spot {
                last_docked[ix] = *side;
            }
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
        self.last_docked = last_docked;
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
    use gpui::{AnyElement, Entity, TestAppContext, div, prelude::*, rems};

    use crate::theme::Theme;

    use super::*;

    fn blank() -> AnyElement {
        div().into_any_element()
    }

    fn panel(id: &str) -> DockPanel {
        DockPanel {
            id: id.into(),
            title: PanelTitle::Text(id.into()),
            icon: IconName::Folder,
        }
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

    /// Closing the leading panel hands the side to the next open one; opening it
    /// takes the side back.
    #[gpui::test]
    fn closing_a_panel_hands_the_side_over_and_opening_takes_it_back(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            assert_eq!(dock.active[DockSide::Left.index()], Some(0), "files leads");
            dock.hide("files", cx);
            assert!(dock.is_hidden("files"));
            assert_eq!(
                dock.active[DockSide::Left.index()],
                Some(1),
                "outline takes over"
            );
            dock.show("files", cx);
            assert!(!dock.is_hidden("files"));
            assert_eq!(
                dock.active[DockSide::Left.index()],
                Some(0),
                "files is back"
            );
        });
    }

    /// A side every panel of which is closed shows nothing, not a stale one.
    #[gpui::test]
    fn closing_every_panel_on_a_side_leaves_it_empty(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            dock.hide("files", cx);
            dock.hide("outline", cx);
            assert_eq!(dock.active[DockSide::Left.index()], None);
            assert!(dock.shown_on_side(DockSide::Left).is_empty());
            dock.toggle("files", cx);
            assert_eq!(dock.active[DockSide::Left.index()], Some(0));
        });
    }

    /// The layout carries what is closed, and a closed panel is never the one a
    /// side shows — including right after restoring.
    #[gpui::test]
    fn a_layout_carries_what_is_closed(cx: &mut TestAppContext) {
        let dock = bench(cx);
        let layout = dock.update(cx, |dock, cx| {
            dock.hide("files", cx);
            dock.layout()
        });
        assert_eq!(layout.hidden, ["files"]);
        dock.update(cx, |dock, cx| {
            dock.show("files", cx);
            assert_eq!(dock.active[DockSide::Left.index()], Some(0));
            dock.restore(&layout, cx).expect("a layout it just wrote");
            assert!(dock.is_hidden("files"));
            assert_eq!(
                dock.active[DockSide::Left.index()],
                Some(1),
                "a closed panel never leads"
            );
        });
    }

    /// A registry holds nothing until a view is registered, and a dock built from
    /// one takes the panels the caller names without a match of its own.
    /// (A registered view needs a window-backed entity to be drawn, so the drawing
    /// side is exercised by the shell rather than here.)
    #[gpui::test]
    fn a_registry_holds_nothing_until_a_view_is_registered(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            let registry = PanelRegistry::new();
            assert!(registry.render("search").is_none(), "nothing registered");
            assert_eq!(registry.ids().count(), 0);
            let mut dock = Dock::from_registry(registry, |_, _| div().into_any_element(), cx);
            dock.add(panel("files"), DockSide::Left);
            assert_eq!(dock.active[DockSide::Left.index()], Some(0));
        });
    }

    /// A floated panel has a way home: it returns to the side it was docked to.
    #[gpui::test]
    fn a_floated_panel_returns_to_the_side_it_was_docked_to(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            dock.toggle_float("terminal", cx);
            assert!(matches!(dock.spots[2], Spot::Floating { .. }));
            dock.toggle_float("terminal", cx);
            assert_eq!(dock.spots[2], Spot::Docked(DockSide::Bottom));
            assert_eq!(dock.active[DockSide::Bottom.index()], Some(2));
        });
    }

    /// Zooming fills the dock with one panel; restoring gives the arrangement back,
    /// and zooming a closed panel opens it.
    #[gpui::test]
    fn zooming_fills_the_dock_and_restoring_returns_it(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            dock.zoom("terminal", cx);
            assert!(dock.is_zoomed("terminal"));
            dock.toggle_zoom("terminal", cx);
            assert!(!dock.is_zoomed("terminal"));
            assert_eq!(
                dock.active[DockSide::Left.index()],
                Some(0),
                "the arrangement is back"
            );
            dock.hide("outline", cx);
            dock.zoom("outline", cx);
            assert!(dock.is_zoomed("outline"));
            assert!(!dock.is_hidden("outline"), "zooming opens the panel");
        });
    }

    /// A side.s limit brings a size that sits outside it back into range, and a
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

    /// Activating and moving both mean "show me this", so a closed panel opens.
    #[gpui::test]
    fn asking_for_a_closed_panel_opens_it(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            dock.hide("terminal", cx);
            dock.activate("terminal", cx);
            assert!(!dock.is_hidden("terminal"));
            dock.hide("terminal", cx);
            dock.move_panel("terminal", Spot::Docked(DockSide::Right), cx);
            assert!(!dock.is_hidden("terminal"));
            assert_eq!(dock.active[DockSide::Right.index()], Some(2));
        });
    }
}
