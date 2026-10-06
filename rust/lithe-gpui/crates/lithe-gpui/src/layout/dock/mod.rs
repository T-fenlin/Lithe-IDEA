mod view;

use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, EventEmitter, Hsla, IntoElement as _, Pixels, Rems, SharedString,
    Window, px,
};
use serde::{Deserialize, Serialize};

use crate::{primitives::IconName, theme::ActiveTheme};

/// How many groups a side holds. Two is what an IDE uses: one tool window above
/// another, each with its own tabs.
pub const GROUPS: usize = 2;

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

/// Where a panel lives: a side's group, or floating at a window position.
///
/// A side holds [`GROUPS`] groups stacked along it, so an editor can put a file
/// tree above an outline on the left, as an IDE does.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Spot {
    Docked { side: DockSide, group: u8 },
    Floating { x: f32, y: f32 },
}

impl Spot {
    /// The first group on `side`.
    pub fn docked(side: DockSide) -> Self {
        Self::Docked { side, group: 0 }
    }
}

/// How far a side.s size may go. `max` of `None` keeps the cap the drag applies
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
    /// Per-panel looks that win over the dock's.
    pub look: PanelLook,
}

/// One panel's appearance, over the dock's defaults. `None` inherits.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PanelLook {
    /// Border thickness; 0 draws none.
    pub border: Option<Pixels>,
    pub border_color: Option<Hsla>,
    pub radius: Option<Rems>,
    pub background: Option<Hsla>,
    /// Draws the tab strip. An application whose own chrome already names the
    /// panel turns it off; the divider is then the only handle.
    pub header: Option<bool>,
}

impl DockPanel {
    /// A panel with the dock's own look.
    pub fn new(id: impl Into<SharedString>, title: PanelTitle, icon: IconName) -> Self {
        Self {
            id: id.into(),
            title,
            icon,
            look: PanelLook::default(),
        }
    }

    /// A panel the application's own chrome already announces: its group draws
    /// no tab strip, so there is no icon and no drag handle — the divider is the
    /// only way to size it, and the application's own control the way to close it.
    pub fn plain(id: impl Into<SharedString>, title: impl Into<PanelTitle>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: IconName::Square,
            look: PanelLook {
                header: Some(false),
                ..PanelLook::default()
            },
        }
    }

    /// Border thickness; 0 draws none.
    pub fn border(mut self, width: Pixels) -> Self {
        self.look.border = Some(width);
        self
    }

    /// Corner radius, on the rem scale like the theme's own.
    pub fn radius(mut self, radius: Rems) -> Self {
        self.look.radius = Some(radius);
        self
    }

    /// The panel's own background, over the dock's.
    pub fn background(mut self, color: Hsla) -> Self {
        self.look.background = Some(color);
        self
    }

    /// The border's color, over the dock's.
    pub fn border_color(mut self, color: Hsla) -> Self {
        self.look.border_color = Some(color);
        self
    }

    /// Whether this panel's group draws its tab strip.
    pub fn header(mut self, on: bool) -> Self {
        self.look.header = Some(on);
        self
    }
}

/// The dock's appearance defaults: the frame around each panel, the divider
/// between groups and the drag handle. A panel's [`PanelLook`] wins per field.
#[derive(Clone, Debug, PartialEq)]
pub struct DockLook {
    /// Border thickness around each panel; 0 draws none.
    pub border: Pixels,
    pub border_color: Hsla,
    pub radius: Rems,
    /// A panel's background, where it does not set its own.
    pub background: Hsla,
    /// Space between a panel and its divider.
    pub gap: Pixels,
    /// The divider's hit area.
    pub handle: Pixels,
    /// The divider's line thickness.
    pub handle_line: Pixels,
    /// Whether every group draws its tab strip, where a panel does not say.
    pub header: bool,
}

impl DockLook {
    /// Defaults from the theme's own tokens: no frame, no rounding, no gap.
    pub fn from_theme(cx: &App) -> Self {
        let theme = cx.theme();
        Self {
            border: px(0.),
            border_color: theme.colors.border,
            radius: Rems::ZERO,
            background: theme.colors.surface,
            gap: px(0.),
            handle: px(6.),
            handle_line: px(1.),
            header: true,
        }
    }
}

/// A panel's look once the dock's defaults have filled its gaps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintedLook {
    pub border: Pixels,
    pub border_color: Hsla,
    pub radius: Rems,
    pub background: Hsla,
    pub header: bool,
}

/// Saved dock arrangement, per side in `DockSide` order. Sizes are in rems.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DockLayout {
    pub spots: Vec<(String, Spot)>,
    /// Per side, per group: the panel that group shows.
    pub active: [[Option<String>; GROUPS]; 4],
    pub sizes: [f32; 4],
    /// Per side: the share of its extent group 0 takes, group 1 the rest.
    #[serde(default = "even_split")]
    pub split: [f32; 4],
    /// Panels that are closed but still docked, so they can be opened again.
    #[serde(default)]
    pub hidden: Vec<String>,
}

fn even_split() -> [f32; 4] {
    [0.5; 4]
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

impl EventEmitter<DockEvent> for Dock {}

type Center = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
type Content = Rc<dyn Fn(&SharedString, &mut Window, &mut App) -> AnyElement>;

/// What a dock reports once a person has changed it, so an application can save
/// the arrangement or sync its own chrome. A programmatic change — `hide`,
/// `set_size`, `restore` — does not emit: the caller already knows.
#[derive(Clone, Debug, PartialEq)]
pub enum DockEvent {
    /// A divider was dragged: the side is now this many rems.
    Resized { side: DockSide, size: Rems },
    /// The divider between a side's two groups moved.
    Split { side: DockSide, share: f32 },
    /// A panel was opened or closed from the dock itself.
    VisibilityChanged { id: SharedString, hidden: bool },
    /// A panel moved, docked or floating.
    Moved { id: SharedString, spot: Spot },
}

/// Side docks around a center; panels move between docks, float, hide and show.
pub struct Dock {
    panels: Vec<DockPanel>,
    spots: Vec<Spot>,
    /// Closed but still docked: `hidden[ix]` matches `panels[ix]`.
    hidden: Vec<bool>,
    active: [[Option<usize>; GROUPS]; 4],
    /// The panel filling the dock on its own, until it is restored.
    zoomed: Option<usize>,
    /// The side each panel was last docked to, so a floated panel has a way home.
    last_docked: Vec<DockSide>,
    /// Whether a rail of icons is drawn along the left and right edges, one per
    /// panel docked there; a click opens that panel. An application with its own
    /// activity bar leaves this off.
    rail: bool,
    /// Whether a strip of closed panels is drawn, so they can be opened again.
    /// An application with an activity bar turns this off: a toggle there is the
    /// usual way back, and a second one would be noise.
    closed_strip: bool,
    sizes: [Rems; 4],
    split: [f32; 4],
    limits: [SideLimits; 4],
    look: DockLook,
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
            active: [[None; GROUPS]; 4],
            zoomed: None,
            last_docked: Vec::new(),
            rail: false,
            closed_strip: false,
            sizes: [side, side, strip, strip],
            split: even_split(),
            limits: [limits; 4],
            look: DockLook::from_theme(cx),
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

    /// Draws an icon rail along the left and right edges. Off by default.
    pub fn rail(mut self, on: bool) -> Self {
        self.rail = on;
        self
    }

    /// Draws a strip of closed panels, each a button that opens it again.
    /// Off by default.
    pub fn closed_strip(mut self, on: bool) -> Self {
        self.closed_strip = on;
        self
    }

    /// Replaces the appearance defaults. A panel's [`PanelLook`] wins per field.
    pub fn look(mut self, look: DockLook) -> Self {
        self.look = look;
        self
    }

    /// The divider's hit area and its line thickness.
    pub fn handle(mut self, hit: Pixels, line: Pixels) -> Self {
        self.look.handle = hit;
        self.look.handle_line = line;
        self
    }

    /// Space between a panel and its divider.
    pub fn gap(mut self, gap: Pixels) -> Self {
        self.look.gap = gap;
        self
    }

    /// The border around every panel, where a panel does not set its own.
    pub fn border(mut self, width: Pixels, color: Hsla) -> Self {
        self.look.border = width;
        self.look.border_color = color;
        self
    }

    /// How one panel draws: its own look over the dock's, field by field.
    pub fn painted(&self, panel: &DockPanel) -> PaintedLook {
        PaintedLook {
            border: panel.look.border.unwrap_or(self.look.border),
            border_color: panel.look.border_color.unwrap_or(self.look.border_color),
            radius: panel.look.radius.unwrap_or(self.look.radius),
            background: panel.look.background.unwrap_or(self.look.background),
            header: panel.look.header.unwrap_or(self.look.header),
        }
    }

    /// Panics on a repeated id. The panel joins `side`'s first group.
    pub fn add(&mut self, panel: DockPanel, side: DockSide) {
        self.add_to(panel, side, 0);
    }

    /// Adds a panel to a side's `group`; a group past [`GROUPS`] is the last one.
    /// Panics on a repeated id.
    pub fn add_to(&mut self, panel: DockPanel, side: DockSide, group: u8) {
        assert!(
            self.find(&panel.id).is_none(),
            "dock panel {} added twice",
            panel.id
        );
        let group = usize::from(group).min(GROUPS - 1);
        self.panels.push(panel);
        self.spots.push(Spot::Docked {
            side,
            group: group as u8,
        });
        self.hidden.push(false);
        self.last_docked.push(side);
        let slot = &mut self.active[side.index()][group];
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

    /// How much of `side` its first group takes; the rest goes to the second.
    pub fn set_split(&mut self, side: DockSide, share: f32, cx: &mut Context<Self>) {
        self.split[side.index()] = share.clamp(0.15, 0.85);
        cx.notify();
    }

    fn on_side(&self, side: DockSide, group: usize) -> Vec<usize> {
        (0..self.panels.len())
            .filter(|ix| {
                self.spots[*ix]
                    == Spot::Docked {
                        side,
                        group: group as u8,
                    }
            })
            .collect()
    }

    /// The panels a group shows: docked there and not closed.
    fn shown_in(&self, side: DockSide, group: usize) -> Vec<usize> {
        self.on_side(side, group)
            .into_iter()
            .filter(|ix| !self.hidden[*ix])
            .collect()
    }

    /// Whether `ix` is the panel its own group is showing.
    pub fn is_active_in(&self, ix: usize) -> bool {
        match self.spots[ix] {
            Spot::Docked { side, group } => {
                self.active[side.index()][usize::from(group)] == Some(ix)
            }
            Spot::Floating { .. } => false,
        }
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

    /// Hands a group to another of its panels when `ix` leaves it.
    fn reselect(&mut self, ix: usize) {
        if let Spot::Docked { side, group } = self.spots[ix] {
            let group = usize::from(group);
            if self.active[side.index()][group] == Some(ix) {
                self.active[side.index()][group] = self.shown_in(side, group).first().copied();
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
        if let Spot::Docked { side, group } = self.spots[ix] {
            self.active[side.index()][usize::from(group)] = Some(ix);
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
        if let Spot::Docked { side, group } = self.spots[ix] {
            self.active[side.index()][usize::from(group)] = Some(ix);
            cx.notify();
        }
    }

    /// Moves a panel to `side`'s `group`, or sends it floating.
    pub fn move_to_group(&mut self, id: &str, side: DockSide, group: u8, cx: &mut Context<Self>) {
        let group = usize::from(group).min(GROUPS - 1) as u8;
        self.move_panel(id, Spot::Docked { side, group }, cx);
    }

    pub fn move_panel(&mut self, id: &str, spot: Spot, cx: &mut Context<Self>) {
        let ix = self
            .find(id)
            .unwrap_or_else(|| panic!("no dock panel {id}"));
        let from = self.spots[ix];
        self.spots[ix] = spot;
        // A move shows the panel: dropping a tab somewhere means wanting it there.
        self.hidden[ix] = false;
        self.reselect(ix);
        if let Spot::Docked { side, group } = spot {
            self.last_docked[ix] = side;
            self.active[side.index()][usize::from(group)] = Some(ix);
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

    /// Zooms the panel, or restores the dock when it is the zoomed one.
    pub fn toggle_zoom(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.is_zoomed(id) {
            self.unzoom(cx);
        } else {
            self.zoom(id, cx);
        }
    }

    /// Sends a docked panel afloat, and a floating one back where it was docked.
    pub fn toggle_float(&mut self, id: &str, cx: &mut Context<Self>) {
        let ix = self
            .find(id)
            .unwrap_or_else(|| panic!("no dock panel {id}"));
        let spot = match self.spots[ix] {
            Spot::Docked { .. } => Spot::Floating { x: 160.0, y: 120.0 },
            Spot::Floating { .. } => Spot::Docked {
                side: self.last_docked[ix],
                group: 0,
            },
        };
        self.move_panel(id, spot, cx);
    }

    pub fn layout(&self) -> DockLayout {
        let name = |slot: Option<usize>| slot.map(|ix| self.panels[ix].id.to_string());
        DockLayout {
            spots: self
                .panels
                .iter()
                .zip(&self.spots)
                .map(|(panel, spot)| (panel.id.to_string(), *spot))
                .collect(),
            active: std::array::from_fn(|side| {
                std::array::from_fn(|group| name(self.active[side][group]))
            }),
            sizes: self.sizes.map(|size| size.0),
            split: self.split,
            hidden: self
                .panels
                .iter()
                .zip(&self.hidden)
                .filter(|(_, hidden)| **hidden)
                .map(|(panel, _)| panel.id.to_string())
                .collect(),
        }
    }

    /// All or nothing. A group the layout leaves unset shows its first open panel.
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
            layout.split.iter().all(|share| share.is_finite()),
            "dock split {:?} must be finite",
            layout.split
        );
        anyhow::ensure!(
            layout.spots.iter().all(|(_, spot)| match spot {
                Spot::Docked { group, .. } => usize::from(*group) < GROUPS,
                Spot::Floating { x, y } => x.is_finite() && y.is_finite(),
            }),
            "dock spots name a group past the last one"
        );
        let unknown: Vec<&str> = layout
            .spots
            .iter()
            .map(|(id, _)| id.as_str())
            .chain(layout.active.iter().flatten().flatten().map(String::as_str))
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
            if let Spot::Docked { side, .. } = spot {
                last_docked[ix] = *side;
            }
        }
        let mut hidden = vec![false; self.panels.len()];
        for id in &layout.hidden {
            hidden[self.find(id).expect("checked above")] = true;
        }
        let mut active = [[None; GROUPS]; 4];
        for side in DockSide::ALL {
            for group in 0..GROUPS {
                let chosen = layout.active[side.index()][group]
                    .as_deref()
                    .and_then(|id| self.find(id));
                if let Some(ix) = chosen {
                    anyhow::ensure!(
                        spots[ix]
                            == Spot::Docked {
                                side,
                                group: group as u8
                            },
                        "active panel {} is not docked in that group",
                        self.panels[ix].id
                    );
                }
                // A closed panel cannot be the one a group shows.
                active[side.index()][group] = chosen.filter(|ix| !hidden[*ix]).or_else(|| {
                    (0..spots.len()).find(|ix| {
                        spots[*ix]
                            == Spot::Docked {
                                side,
                                group: group as u8,
                            }
                            && !hidden[*ix]
                    })
                });
            }
        }
        self.spots = spots;
        self.last_docked = last_docked;
        self.hidden = hidden;
        self.active = active;
        self.sizes = layout.sizes.map(gpui::rems);
        self.split = layout.split;
        log::info!("dock: restored {} panel spots", layout.spots.len());
        cx.notify();
        Ok(())
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use gpui::{AnyElement, Entity, TestAppContext, div, prelude::*, rems};

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

    fn at(dock: &Dock, side: DockSide, group: usize) -> Option<usize> {
        dock.active[side.index()][group]
    }

    /// By default a panel draws no frame, no rounding, on the dock's own tone.
    #[gpui::test]
    fn panels_draw_without_a_frame_by_default(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            let plain = panel("files");
            let look = dock.painted(&plain);
            assert_eq!(look.border, px(0.), "no frame");
            assert_eq!(look.radius, Rems::ZERO, "square corners");
            assert_eq!(
                look.background,
                cx.theme().colors.surface,
                "the dock's tone"
            );
        });
    }

    /// A panel's own fields win; the ones it leaves alone stay the dock's.
    #[gpui::test]
    fn a_panel_overrides_the_dock_field_by_field(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            let frame = cx.theme().colors.focus;
            let docked = px(1.);
            dock.look = DockLook {
                border: docked,
                border_color: frame,
                radius: rems(0.25),
                ..dock.look.clone()
            };
            // Sets a radius only: thickness still comes from the dock.
            let own = panel("outline").radius(rems(0.75));
            let look = dock.painted(&own);
            assert_eq!(look.border, docked, "thickness from the dock");
            assert_eq!(look.radius, rems(0.75), "radius from the panel");
            assert_eq!(look.border_color, frame, "colour from the dock");

            // And the dock's own frame reaches a panel that sets nothing.
            let plain = dock.painted(&panel("files"));
            assert_eq!(plain.border, docked, "the dock's frame applies");
            assert_eq!(plain.radius, rems(0.25), "the dock's radius applies");
        });
    }

    /// The divider's hit area and line, and the gap around a panel, are the
    /// dock's own: they are the frame's, not a panel's.
    #[gpui::test]
    fn the_dock_owns_the_divider_and_the_gap(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            let dock = Dock::new(|_, _| blank(), |_, _, _| blank(), cx)
                .handle(px(9.), px(3.))
                .gap(px(4.));
            assert_eq!(dock.look.handle, px(9.), "hit area");
            assert_eq!(dock.look.handle_line, px(3.), "line");
            assert_eq!(dock.look.gap, px(4.), "gap");
        });
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

    /// Two panels on one side can share it: the second group is its own stack.
    #[gpui::test]
    fn a_side_holds_two_groups(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            assert_eq!(at(dock, DockSide::Left, 0), Some(0), "files leads");
            assert_eq!(
                at(dock, DockSide::Left, 1),
                None,
                "the second group is empty"
            );
            dock.move_to_group("outline", DockSide::Left, 1, cx);
            assert_eq!(
                at(dock, DockSide::Left, 0),
                Some(0),
                "files keeps the first group"
            );
            assert_eq!(
                at(dock, DockSide::Left, 1),
                Some(1),
                "outline now leads the second"
            );
            assert_eq!(
                dock.shown_in(DockSide::Left, 0),
                vec![0],
                "a group shows only its own panels"
            );
        });
    }

    /// Closing a group.s own panel hands that group over, not the whole side.
    #[gpui::test]
    fn closing_hands_over_the_group_it_was_in(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            dock.move_to_group("outline", DockSide::Left, 1, cx);
            dock.hide("outline", cx);
            assert_eq!(at(dock, DockSide::Left, 1), None, "that group is empty");
            assert_eq!(
                at(dock, DockSide::Left, 0),
                Some(0),
                "the other is untouched"
            );
        });
    }

    /// Zooming fills the dock with one panel; restoring gives the arrangement back.
    #[gpui::test]
    fn zooming_fills_the_dock_and_restoring_returns_it(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            dock.zoom("terminal", cx);
            assert!(dock.is_zoomed("terminal"));
            dock.toggle_zoom("terminal", cx);
            assert!(!dock.is_zoomed("terminal"));
            assert_eq!(
                at(dock, DockSide::Left, 0),
                Some(0),
                "the arrangement is back"
            );
            dock.hide("outline", cx);
            dock.zoom("outline", cx);
            assert!(dock.is_zoomed("outline"));
            assert!(!dock.is_hidden("outline"), "zooming opens the panel");
        });
    }

    /// The rail marks the panel a group is showing, and a closed panel stops being
    /// the marked one (the rail lists it so a click can bring it back).
    #[gpui::test]
    fn a_panel_is_the_shown_one_only_while_it_is_open(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            dock.move_to_group("outline", DockSide::Left, 1, cx);
            assert!(dock.is_active_in(0), "files leads its group");
            assert!(dock.is_active_in(1), "outline leads its group");
            dock.hide("outline", cx);
            assert!(
                !dock.is_active_in(1),
                "a closed panel is not the one its group shows"
            );
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
            assert_eq!(
                dock.spots[2],
                Spot::Docked {
                    side: DockSide::Bottom,
                    group: 0
                }
            );
            assert_eq!(at(dock, DockSide::Bottom, 0), Some(2));
        });
    }

    /// The layout carries groups, the split and what is closed; a closed panel is
    /// never the one a group shows — including right after restoring.
    #[gpui::test]
    fn a_layout_carries_groups_and_what_is_closed(cx: &mut TestAppContext) {
        let dock = bench(cx);
        let layout = dock.update(cx, |dock, cx| {
            dock.move_to_group("outline", DockSide::Left, 1, cx);
            dock.set_split(DockSide::Left, 0.3, cx);
            dock.hide("outline", cx);
            dock.layout()
        });
        assert_eq!(layout.hidden, ["outline"]);
        assert_eq!(layout.split[DockSide::Left.index()], 0.3);
        dock.update(cx, |dock, cx| {
            dock.restore(&layout, cx).expect("a layout it just wrote");
            assert!(dock.is_hidden("outline"));
            assert_eq!(
                at(dock, DockSide::Left, 1),
                None,
                "a closed panel never leads"
            );
            assert_eq!(
                layout.spots[1].1,
                Spot::Docked {
                    side: DockSide::Left,
                    group: 1
                }
            );
        });
    }

    /// The split stays inside its range, so neither group can be squeezed away.
    #[gpui::test]
    fn the_split_stays_inside_its_range(cx: &mut TestAppContext) {
        let dock = bench(cx);
        dock.update(cx, |dock, cx| {
            dock.set_split(DockSide::Left, 0.0, cx);
            assert_eq!(dock.split[DockSide::Left.index()], 0.15);
            dock.set_split(DockSide::Left, 1.0, cx);
            assert_eq!(dock.split[DockSide::Left.index()], 0.85);
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
            assert_eq!(at(&dock, DockSide::Left, 0), Some(0));
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
            dock.move_panel("terminal", Spot::docked(DockSide::Right), cx);
            assert!(!dock.is_hidden("terminal"));
            assert_eq!(at(dock, DockSide::Right, 0), Some(2));
        });
    }
}
