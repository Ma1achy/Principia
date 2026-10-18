//! The Trajectory panel (render_gui_spec §G2, §G3; GUI_DESIGN_NOTES § "01 Explore"; 01_main.png; R-390, ORDER item 4):
//! right of the figure, for the IC under the cursor or a kept orbit, by its tabs ("under cursor", `#1`, `#2`, …, `+`).
//! Its frame and controls are real; its content is the mock's plausible fake sample, read from the snapshot. Under the
//! tabs: a one-line summary (fate, `t_end`, FTLE, `E₀`); real space, in the centre-of-mass frame, and the shape sphere
//! side by side, as GUI_DESIGN_NOTES and render_gui_spec §G2 say over 01_main.png's stacked views (applied per R-369);
//! the sphere turns slowly with visible x, y, z axes and switches to an unwrapped (equirectangular) view, its "turn"
//! checkbox stopping it; below them the F₂ word, substeps, minimum separation and `|ΔE/E|`; a playhead for
//! this orbit; **listen** (its sound is TASK-M8-12's, so here it is a state only) and "follow cursor"; **Open full
//! viewer…** and **IC Inspector…**, both of which open the Inspector window on the orbit shown (§G2, §G8; TASK-M6-28); and the kept orbits below, each with its fate and time, removable.
//!
//! **The turn rate (REQ-GUI-157, proposed, R-71; used provisionally until the human confirms it at the M8 gate,
//! R-182):** [`TURN_DEG_PER_S`]. The angle is advanced by the rate times the seconds its time source moved, never by
//! a step per frame, so it turns at the same rate at any frame rate. The seconds are the wall clock's (egui's frame
//! time), the mock's tick counting frames; only capture mode's frozen clock holds them, so a capture is the same every
//! run.
//!
//! Its keys (§G3): Trajectory is big scope 4; Enter goes in, onto its elements in the order drawn, the playhead,
//! listen and the kept orbits among them; Enter on a button acts as its click, on the playhead starts interaction, in
//! which the arrows move it by `Bounded`, 1/100 of the orbit's span. The kept orbits are a scope whose elements come and
//! go with the list: each kept orbit, with its show and remove.
//!
//! **The pin (R-453 D1 to D3, D5):** a primary click on the figure with no drag, or P with the pointer over it, pins
//! the sample under the pointer here: the first tab reads "pinned", shows that sample whatever the pointer does, and
//! an **unpin** button, an element after `+` while it is held, returns it to following the cursor. The pin holds the
//! sample's IC and the inputs its integration read, never a pixel, so a pan, a zoom or a slice edit leaves it; a change
//! of those inputs (the horizon, the integrator, the playhead it is read at) re-integrates the IC through the engine
//! side's `computeIC`. Without a pin the first tab keeps the last sample hovered over the figure, never going blank as
//! the pointer leaves it. `+` and Space keep the first tab's orbit. Like the kept orbits, the pin is the GUI's own
//! state, never undoable, and sends no `SetField`.
//!
//! **The views (R-453 D6 to D8):** real space and the unwrapped view pan and zoom, the sphere orbits and zooms, each
//! tab with its own [`Views`]. On real space and the unwrapped view a drag pans and the wheel zooms about the pointer;
//! on the sphere a drag orbits (yaw about its pole, pitch clamped to ±90°), turning "turn" off, and the wheel zooms; a
//! double-click restores a view's default. The orbit turns the view, never the data, so the sphere and its unwrap
//! agree. From the keyboard each view is an element of the Trajectory scope, the unwrapped view standing in the
//! sphere's place while shown and taking real space's keys: in interaction mode the arrows pan by `Pan` or orbit by
//! `Angle` and + and − zoom by `ZoomLog2`, Shift ×10 and Alt ×0.1.

use eframe::egui::{
    self, pos2, vec2, Align, Button, Color32, Layout as EguiLayout, Pos2, Rect, Response, RichText,
    Sense, Slider, Stroke, Ui, UiBuilder, Vec2,
};
use engine::contract::sample::Sample;
use engine::contract::sim_config::{
    Collision, Horizon, Integrator, KernelVariant, Latent, Quality,
};
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::KeptOrbits;

use crate::keyboard::keymap::{multiplier, Direction};
use crate::keyboard::scopes::{Adjust, Arrows, Base, Scope, ScopeId, ScopeTree, StepKind};
use crate::keyboard::Keyboard;

/// The shape sphere's turn rate, in degrees per second (REQ-GUI-157; proposed, R-71): a full turn in 30 s. Slow enough
/// that a point on the sphere's rim moves at most about a fifth of the sphere's radius a second (12° = 0.21 rad), so
/// the path stays readable as it turns; fast enough to read as turning within a second or two, the rim moving about
/// 20 points a second at the panel's sphere size.
pub const TURN_DEG_PER_S: f64 = 12.0;
/// The sphere's view: its pole tilted this far toward the viewer, in degrees, so all three axes show (a look choice,
/// R-390).
pub const SPHERE_TILT_DEG: f64 = 20.0;
/// The sphere's resting azimuth, in degrees, from which it turns: oblique, as 01_main.png draws it, so neither x nor y
/// lies on z's screen line and all three axes show apart (a look choice, R-390).
pub const REST_ANGLE_DEG: f64 = 35.0;
/// The most orbits kept; keeping one more drops the oldest (a look choice, R-390).
pub const MAX_KEPT: usize = 9;

/// The panel's big scope.
pub const TRAJECTORY: ScopeId = "trajectory";
/// The first tab: "under cursor", or "pinned" while a pin is held (R-453 D2).
pub const UNDER_CURSOR: ScopeId = "traj_under_cursor";
/// The `+` tab: keep the first tab's orbit.
pub const KEEP: ScopeId = "traj_keep";
/// unpin, an element while a pin is held (R-453 D2, D6).
pub const UNPIN: ScopeId = "traj_unpin";
/// The real-space view, an adjustable element (R-453 D6).
pub const REAL_VIEW: ScopeId = "traj_real_view";
/// The sphere's place: the sphere, or the unwrapped view while it is shown there, an adjustable element (R-453 D6).
pub const SPHERE_VIEW: ScopeId = "traj_sphere_view";
/// The "sphere" view.
pub const SPHERE: ScopeId = "traj_sphere";
/// The "unwrapped" view.
pub const UNWRAPPED: ScopeId = "traj_unwrapped";
/// The "turn" checkbox.
pub const TURN: ScopeId = "traj_turn";
/// The playhead's play button.
pub const PLAY: ScopeId = "traj_play";
/// The playhead.
pub const PLAYHEAD: ScopeId = "traj_playhead";
/// listen.
pub const LISTEN: ScopeId = "traj_listen";
/// "follow cursor".
pub const FOLLOW: ScopeId = "traj_follow";
/// Open full viewer…, which opens the Inspector window on the orbit shown, as IC Inspector… does (render_gui_spec §G2:
/// "both open §G8").
pub const VIEWER: ScopeId = "traj_viewer";
/// IC Inspector…, which opens the Inspector on the orbit shown (render_gui_spec §G8).
pub const INSPECTOR: ScopeId = "traj_inspector";
/// The kept orbits.
pub const KEPT: ScopeId = "traj_kept";

/// Each kept orbit's scopes: the orbit, its show and its remove, and its tab's label.
pub const KEPT_IDS: [(ScopeId, ScopeId, ScopeId, &str); MAX_KEPT] = [
    ("kept_1", "kept_1_show", "kept_1_remove", "#1"),
    ("kept_2", "kept_2_show", "kept_2_remove", "#2"),
    ("kept_3", "kept_3_show", "kept_3_remove", "#3"),
    ("kept_4", "kept_4_show", "kept_4_remove", "#4"),
    ("kept_5", "kept_5_show", "kept_5_remove", "#5"),
    ("kept_6", "kept_6_show", "kept_6_remove", "#6"),
    ("kept_7", "kept_7_show", "kept_7_remove", "#7"),
    ("kept_8", "kept_8_show", "kept_8_remove", "#8"),
    ("kept_9", "kept_9_show", "kept_9_remove", "#9"),
];

/// The kept orbits' tabs, `#1` to `#9`, beside the first tab: elements while their orbits are kept, so every visible
/// tab is reachable by keyboard (R-405, R-409; TASK-M6-32's audit), Enter showing the orbit as a click does.
pub const KEPT_TAB_IDS: [ScopeId; MAX_KEPT] = [
    "traj_tab_1",
    "traj_tab_2",
    "traj_tab_3",
    "traj_tab_4",
    "traj_tab_5",
    "traj_tab_6",
    "traj_tab_7",
    "traj_tab_8",
    "traj_tab_9",
];

/// The title.
pub const TITLE: &str = "Trajectory";
/// What the panel shows before any sample was hovered over the figure.
pub const NO_SAMPLE: &str = "move the pointer over the figure";
/// The first tab's label while a pin is held (R-453 D2).
pub const PINNED: &str = "pinned";
/// The first tab's label without a pin.
pub const FOLLOWING: &str = "under cursor";
/// The unpin button's label (R-453 D2).
pub const UNPIN_LABEL: &str = "unpin";
/// The views' elements' labels, as the breadcrumb reads them: real space, the sphere and the unwrapped view.
pub const VIEW_LABELS: [&str; 3] = ["real-space view", "sphere view", "unwrapped view"];
/// The zoom of real space and of the unwrapped view, in octaves from the default, kept within this range (a look
/// choice, R-390): out to an eighth, in to 64 times.
pub const PAN_ZOOM_RANGE: (f64, f64) = (-3.0, 6.0);
/// The sphere's zoom, in octaves from its default size, kept within this range (a look choice, R-390): out to a
/// quarter, in to eight times.
pub const SPHERE_ZOOM_RANGE: (f64, f64) = (-2.0, 3.0);
/// The sphere's orbit for a drag, in degrees per point dragged (a look choice, R-390): a drag across a sphere of 180
/// points turns it half a turn.
pub const ORBIT_DEG_PER_POINT: f64 = 1.0;
/// The sphere's pitch is kept within ±this many degrees (R-453 D7).
pub const PITCH_LIMIT_DEG: f64 = 90.0;
/// What listen sonifies (`principia_scratchpad_pointer_channels.md` §4: `θ(t), φ(t)` → spectrum; 01_main.png's
/// selector is illustrative, R-68).
pub const LISTEN_MAPPING: &str = "θ(t), φ(t) → spectrum";
/// The panel's margin, in points (a look choice, R-390).
pub const MARGIN: f32 = 12.0;
/// The axes' colours, x, y, z (a look choice, R-390: 01_main.png's red, green and blue).
pub const AXIS_COLOURS: [Color32; 3] = [
    Color32::from_rgb(220, 70, 70),
    Color32::from_rgb(80, 190, 90),
    Color32::from_rgb(80, 130, 230),
];
/// The three bodies' colours in real space (a look choice, R-390).
pub const BODY_COLOURS: [Color32; 3] = [
    Color32::from_rgb(240, 120, 90),
    Color32::from_rgb(100, 170, 240),
    Color32::from_rgb(240, 200, 90),
];
/// The opacity of the sphere's far side (a look choice, R-390).
pub const FAR_SIDE: f32 = 0.3;

/// Joins the panel's elements to `tree`, under its big scope, in the order drawn, with its rows of the `?` overlay.
pub fn register(tree: &mut ScopeTree) {
    tree.register(None, Scope::group(TRAJECTORY, TITLE));
    for (id, label) in [(UNDER_CURSOR, FOLLOWING), (KEEP, "+ keep")] {
        tree.register(Some(TRAJECTORY), Scope::control(id, label));
    }
    tree.register(
        Some(TRAJECTORY),
        Scope::value(REAL_VIEW, VIEW_LABELS[0], StepKind::Pan),
    );
    tree.register(
        Some(TRAJECTORY),
        Scope::value(SPHERE_VIEW, VIEW_LABELS[1], StepKind::Angle),
    );
    for (id, label) in [
        (SPHERE, "sphere"),
        (UNWRAPPED, "unwrapped"),
        (TURN, "turn"),
        (PLAY, "play"),
    ] {
        tree.register(Some(TRAJECTORY), Scope::control(id, label));
    }
    tree.register(
        Some(TRAJECTORY),
        Scope::value(PLAYHEAD, "playhead", StepKind::Bounded),
    );
    tree.register(Some(TRAJECTORY), Scope::control(LISTEN, "listen"));
    tree.register(Some(TRAJECTORY), Scope::control(FOLLOW, "follow cursor"));
    tree.register(
        Some(TRAJECTORY),
        Scope::control(VIEWER, "Open full viewer…"),
    );
    tree.register(Some(TRAJECTORY), Scope::control(INSPECTOR, "IC Inspector…"));
    tree.register(Some(TRAJECTORY), Scope::group(KEPT, "Kept orbits"));
    for (key, action) in SHORTCUTS {
        tree.register_shortcut(key, action);
    }
}

/// The rows the Figure's Space, L and P and the panel add to the `?` overlay (§G3; R-453 D6).
pub const SHORTCUTS: [(&str, &str); 5] = [
    (
        "Figure: Space",
        "keep the orbit under the cursor, or the pinned one, in either mode",
    ),
    ("Figure: L", "listen, in either mode"),
    (
        "Figure: P",
        "pin the sample under the pointer in the Trajectory panel, in either mode",
    ),
    (
        "Trajectory",
        "Enter reaches the tabs, unpin, the two views, the playhead, listen and the kept orbits",
    ),
    (
        "Trajectory: views",
        "after Enter: arrows pan real space and the unwrapped view, orbit the sphere; + / − zoom; Shift ×10, Alt ×0.1",
    ),
];

/// Joins the panel's changing elements to `tree`, again each frame: unpin after `+` while a pin is held, the first
/// tab's label ("pinned" or "under cursor"), and the sphere's place, the sphere orbiting by `Angle` or, while the
/// unwrapped view is shown there, the unwrapped view panning by `Pan`, as real space does (R-453 D2, D6).
pub fn join(tree: &mut ScopeTree, scratch: &Scratch) {
    let pinned = scratch.pin.is_some();
    match (pinned, tree.get(UNPIN).is_some()) {
        (true, false) => tree.insert_after(KEEP, Scope::control(UNPIN, UNPIN_LABEL)),
        (false, true) => tree.remove(UNPIN),
        _ => {}
    }
    if let Some(first) = tree.get_mut(UNDER_CURSOR) {
        first.label = first_tab(pinned);
    }
    if let Some(view) = tree.get_mut(SPHERE_VIEW) {
        let (label, kind) = if scratch.unwrapped {
            (VIEW_LABELS[2], StepKind::Pan)
        } else {
            (VIEW_LABELS[1], StepKind::Angle)
        };
        view.label = label;
        view.arrows = Arrows::Adjust(kind);
    }
}

/// The first tab's label: "pinned" while a pin is held, else "under cursor".
pub fn first_tab(pinned: bool) -> &'static str {
    if pinned {
        PINNED
    } else {
        FOLLOWING
    }
}

/// Joins the kept orbits' scopes to `tree` for `n` kept, again whenever `n` changed: each kept orbit in the list, with
/// its show and remove, and its tab after the first tab, in the order drawn.
pub fn join_kept(tree: &mut ScopeTree, n: usize) {
    if tree.children(Some(KEPT)).len() == n {
        return;
    }
    tree.remove_children(KEPT);
    for (orbit, show, remove, label) in KEPT_IDS.iter().take(n) {
        tree.register(Some(KEPT), Scope::group(orbit, label));
        tree.register(Some(orbit), Scope::control(show, "show"));
        tree.register(Some(orbit), Scope::control(remove, "remove"));
    }
    for tab in KEPT_TAB_IDS {
        tree.remove(tab);
    }
    for i in (0..n).rev() {
        tree.insert_after(UNDER_CURSOR, Scope::control(KEPT_TAB_IDS[i], KEPT_IDS[i].3));
    }
}

/// Which orbit the panel shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    /// The first tab: the pinned IC, or the IC under the cursor, the last one hovered.
    #[default]
    UnderCursor,
    /// Kept orbit `#n`, by its index in the list.
    Kept(usize),
}

impl Tab {
    /// The tab's place among the panel's views: 0 for the first tab, `i + 1` for kept orbit `i`.
    pub fn index(self) -> usize {
        match self {
            Tab::UnderCursor => 0,
            Tab::Kept(i) => i + 1,
        }
    }
}

/// A view that pans and zooms (real space, the unwrapped view; R-453 D7): the point of the drawing at its centre and
/// its zoom. The drawing's points are in `[−1, 1]²` across the view at the default, `x` right and `y` down, the view's
/// half-width 1; the view draws `p` at `(p − centre) · 2^zoom`. The default is the drawing fitted, as before.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PanZoom {
    /// The drawing's point at the view's centre.
    pub centre: [f64; 2],
    /// The zoom, in octaves from the fit, in [`PAN_ZOOM_RANGE`].
    pub zoom: f64,
}

impl PanZoom {
    /// Where the drawing's point `p` falls in the view, in half-widths from its centre.
    pub fn apply(&self, p: [f64; 2]) -> [f64; 2] {
        let k = self.zoom.exp2();
        [(p[0] - self.centre[0]) * k, (p[1] - self.centre[1]) * k]
    }

    /// The drawing's point the view shows at `v`, in half-widths from its centre: the inverse of [`Self::apply`].
    pub fn unapply(&self, v: [f64; 2]) -> [f64; 2] {
        let k = self.zoom.exp2();
        [self.centre[0] + v[0] / k, self.centre[1] + v[1] / k]
    }

    /// A drag by `delta` half-widths: the drawing follows the pointer.
    pub fn drag(&mut self, delta: [f64; 2]) {
        let k = self.zoom.exp2();
        self.centre = [self.centre[0] - delta[0] / k, self.centre[1] - delta[1] / k];
    }

    /// A zoom by `octaves` about the view's point `at`, in half-widths from its centre: the drawing's point there stays
    /// there. The zoom is kept in [`PAN_ZOOM_RANGE`].
    pub fn zoom_about(&mut self, octaves: f64, at: [f64; 2]) {
        let fixed = self.unapply(at);
        self.zoom = (self.zoom + octaves).clamp(PAN_ZOOM_RANGE.0, PAN_ZOOM_RANGE.1);
        let k = self.zoom.exp2();
        self.centre = [fixed[0] - at[0] / k, fixed[1] - at[1] / k];
    }

    /// An arrow's pan by `fraction` of the view's span along its axis (R-404's `Pan`, its sign the arrow's): → and ↑
    /// move the view right and up over the drawing, as the Figure's arrows do.
    pub fn step(&mut self, fraction: f64, direction: Direction) {
        let span = 2.0 / self.zoom.exp2();
        match direction {
            Direction::Left | Direction::Right => self.centre[0] += fraction * span,
            Direction::Up | Direction::Down => self.centre[1] -= fraction * span,
        }
    }
}

/// The sphere's view (R-453 D7, D8): its orbit, a yaw about the pole added to the turn and a pitch of the pole toward
/// the viewer, and its zoom. It turns the view, never the data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    /// The yaw, in degrees, in `[0, 360)`, added to the turn.
    pub yaw: f64,
    /// The pole's pitch toward the viewer, in degrees, in `[−90, 90]`.
    pub pitch: f64,
    /// The zoom, in octaves from the default size, in [`SPHERE_ZOOM_RANGE`].
    pub zoom: f64,
}

impl Default for Orbit {
    /// The default view: at rest, the pole tilted [`SPHERE_TILT_DEG`] toward the viewer, at its default size.
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: SPHERE_TILT_DEG,
            zoom: 0.0,
        }
    }
}

impl Orbit {
    /// Turns the view by `yaw` and `pitch` degrees, the pitch kept within ±[`PITCH_LIMIT_DEG`].
    pub fn turn(&mut self, yaw: f64, pitch: f64) {
        self.yaw = (self.yaw + yaw).rem_euclid(360.0);
        self.pitch = (self.pitch + pitch).clamp(-PITCH_LIMIT_DEG, PITCH_LIMIT_DEG);
    }

    /// Zooms by `octaves`, kept in [`SPHERE_ZOOM_RANGE`].
    pub fn zoom_by(&mut self, octaves: f64) {
        self.zoom = (self.zoom + octaves).clamp(SPHERE_ZOOM_RANGE.0, SPHERE_ZOOM_RANGE.1);
    }
}

/// One tab's views: real space, the sphere and the unwrapped view (R-453 D8).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Views {
    /// Real space.
    pub real: PanZoom,
    /// The sphere.
    pub sphere: Orbit,
    /// The unwrapped view.
    pub unwrapped: PanZoom,
}

/// What a pinned IC's integration read besides the IC: the sim key's settings an IC's orbit depends on, and the
/// playhead its fate is read at, as the hover's sample is. The slice plane is not among them: the pin holds the IC,
/// whatever plane it was picked on (R-453 D1, D5).
#[derive(Clone, Debug, PartialEq)]
pub struct Inputs {
    /// The horizon.
    pub horizon: Horizon,
    /// The integrator.
    pub integrator: Integrator,
    /// The kernel variant.
    pub kernel_variant: KernelVariant,
    /// The collision settings.
    pub collision: Collision,
    /// The quality settings.
    pub quality: Quality,
    /// The playhead's `t`.
    pub playhead: f64,
}

impl Inputs {
    /// The inputs as `snapshot` holds them.
    pub fn of(snapshot: &Snapshot) -> Self {
        let sim = &snapshot.sim;
        Self {
            horizon: sim.horizon.clone(),
            integrator: sim.integrator.clone(),
            kernel_variant: sim.kernel_variant,
            collision: sim.collision.clone(),
            quality: sim.quality.clone(),
            playhead: snapshot.render.playhead.t,
        }
    }
}

/// The pin: the pinned sample, its IC integrated under `inputs` (R-453 D1, D5).
#[derive(Clone, Debug, PartialEq)]
pub struct Pin {
    /// The sample: its IC, and its orbit as last integrated.
    pub sample: Sample,
    /// What that integration read.
    pub inputs: Inputs,
}

/// The panel's own state: the GUI's, never the engine's, never undoable.
#[derive(Clone, Debug, PartialEq)]
pub struct Scratch {
    /// The tab shown.
    pub tab: Tab,
    /// The unwrapped view, in place of the sphere.
    pub unwrapped: bool,
    /// Whether the sphere turns.
    pub turn: bool,
    /// The sphere's turn so far, in degrees, in `[0, 360)`.
    pub angle: f64,
    /// The time source's seconds at the last frame.
    pub last_seconds: Option<f64>,
    /// listen: on or off (its sound is TASK-M8-12's).
    pub listen: bool,
    /// Whether listen follows the cursor.
    pub follow: bool,
    /// The orbit's playhead, in `t`.
    pub orbit_t: f64,
    /// Whether the orbit's playhead plays.
    pub playing: bool,
    /// The control that asked for the Inspector this frame (IC Inspector… or Open full viewer…, a click or Enter),
    /// for the app to open it on the orbit shown, that control its opener.
    pub open_inspector: Option<ScopeId>,
    /// The pin, while one is held (R-453 D1).
    pub pin: Option<Pin>,
    /// The last sample hovered over the figure, kept when the pointer leaves it (R-453 D3).
    pub last: Option<Sample>,
    /// Each tab's views, by [`Tab::index`]; a tab past the end has the defaults (R-453 D8).
    pub views: Vec<Views>,
}

impl Default for Scratch {
    fn default() -> Self {
        Self {
            tab: Tab::UnderCursor,
            unwrapped: false,
            turn: true,
            angle: 0.0,
            last_seconds: None,
            listen: false,
            follow: true,
            orbit_t: 0.0,
            playing: false,
            open_inspector: None,
            pin: None,
            last: None,
            views: Vec::new(),
        }
    }
}

/// The angle after turning at `rate` degrees per second for `dt` seconds from `angle`, in `[0, 360)`.
pub fn turned(angle: f64, rate: f64, dt: f64) -> f64 {
    (angle + rate * dt).rem_euclid(360.0)
}

impl Scratch {
    /// One frame at `seconds`, the wall clock's or a frozen source's: the sphere turns, while "turn" is on and the sphere shown, by the rate
    /// times the seconds since the last frame, and the orbit's playhead plays on, up to `span`.
    pub fn advance(&mut self, seconds: f64, span: f64) {
        let dt = self
            .last_seconds
            .map_or(0.0, |last| (seconds - last).max(0.0));
        self.last_seconds = Some(seconds);
        if self.turn && !self.unwrapped {
            self.angle = turned(self.angle, TURN_DEG_PER_S, dt);
        }
        if self.playing {
            self.orbit_t = (self.orbit_t + dt).min(span);
            if self.orbit_t >= span {
                self.playing = false;
            }
        }
    }

    /// Whether the panel moves by itself, so the app asks for frames: with an orbit `shown`, the sphere turning or the
    /// playhead playing. With none, nothing on the panel moves, and the app asks for no frames for it.
    pub fn moving(&self, shown: bool) -> bool {
        shown && ((self.turn && !self.unwrapped) || self.playing)
    }

    /// The first tab's orbit: the pinned sample while a pin is held, else the last sample hovered (R-453 D2, D3).
    pub fn first(&self) -> Option<&Sample> {
        self.pin.as_ref().map(|p| &p.sample).or(self.last.as_ref())
    }

    /// The snapshot's sample under the cursor, `sample`, read: kept as the last hovered while there is one, and kept
    /// when there is none, the pointer off the figure (R-453 D3).
    pub fn hovered(&mut self, sample: Option<&Sample>) {
        if let Some(s) = sample {
            if self.last.as_ref() != Some(s) {
                self.last = Some(s.clone());
            }
        }
    }

    /// Pins `sample`, integrated under `inputs`, the first tab shown (R-453 D1).
    pub fn pin(&mut self, sample: Sample, inputs: Inputs) {
        self.pin = Some(Pin { sample, inputs });
        self.tab = Tab::UnderCursor;
    }

    /// Re-integrates the pinned IC through `compute_ic` when the inputs as they stand, `inputs`, differ from those its
    /// orbit read (R-453 D5); the orbit as it was while `compute_ic` gives none.
    pub fn repin(
        &mut self,
        inputs: Inputs,
        compute_ic: impl FnOnce(&Latent, [f64; 2]) -> Option<Sample>,
    ) {
        let Some(pin) = &mut self.pin else {
            return;
        };
        if pin.inputs == inputs {
            return;
        }
        if let Some(sample) = compute_ic(&pin.sample.ic, pin.sample.at) {
            pin.sample = sample;
        }
        pin.inputs = inputs;
    }

    /// The views of `tab`.
    pub fn views_of(&self, tab: Tab) -> Views {
        self.views.get(tab.index()).copied().unwrap_or_default()
    }

    /// The views of `tab`, to change.
    pub fn views_mut(&mut self, tab: Tab) -> &mut Views {
        let i = tab.index();
        if self.views.len() <= i {
            self.views.resize(i + 1, Views::default());
        }
        &mut self.views[i]
    }
}

/// The orbit the panel shows: the first tab's, `first` (the pinned sample, or the last hovered), or a kept one.
pub fn shown<'a>(tab: Tab, first: Option<&'a Sample>, kept: &'a KeptOrbits) -> Option<&'a Sample> {
    match tab {
        Tab::UnderCursor => first,
        Tab::Kept(i) => kept.orbits.get(i),
    }
}

/// Keeps `sample` at the end of `kept`, dropping the oldest past [`MAX_KEPT`] as [`remove`] does, so a tab on a kept
/// orbit stays on that orbit, or goes back to "under cursor" if it was the one dropped.
pub fn keep(sample: &Sample, kept: &mut KeptOrbits, scratch: &mut Scratch) {
    if kept.orbits.len() >= MAX_KEPT {
        remove(0, kept, scratch);
    }
    kept.orbits.push(sample.clone());
}

/// Removes kept orbit `i`, the tab following: a tab on it goes back to "under cursor", one after it moves up one.
pub fn remove(i: usize, kept: &mut KeptOrbits, scratch: &mut Scratch) {
    if i >= kept.orbits.len() {
        return;
    }
    kept.orbits.remove(i);
    // Its views go with it; the later tabs' move up with theirs.
    let views = Tab::Kept(i).index();
    if views < scratch.views.len() {
        scratch.views.remove(views);
    }
    if let Tab::Kept(j) = scratch.tab {
        scratch.tab = match j.cmp(&i) {
            std::cmp::Ordering::Equal => Tab::UnderCursor,
            std::cmp::Ordering::Greater => Tab::Kept(j - 1),
            std::cmp::Ordering::Less => Tab::Kept(j),
        };
    }
}

/// Keeps the first tab's orbit, the pinned one or the last hovered, as `+` and the Figure's Space do (R-453 D2).
pub fn keep_first(kept: &mut KeptOrbits, scratch: &mut Scratch) {
    if let Some(s) = scratch.first().cloned() {
        keep(&s, kept, scratch);
    }
}

/// Enter on the panel's control `id`, as its click.
pub fn activate(id: &str, kept: &mut KeptOrbits, scratch: &mut Scratch) {
    match id {
        UNDER_CURSOR => scratch.tab = Tab::UnderCursor,
        KEEP => keep_first(kept, scratch),
        UNPIN => scratch.pin = None,
        SPHERE => scratch.unwrapped = false,
        UNWRAPPED => scratch.unwrapped = true,
        TURN => scratch.turn = !scratch.turn,
        PLAY => scratch.playing = !scratch.playing,
        LISTEN => scratch.listen = !scratch.listen,
        FOLLOW => scratch.follow = !scratch.follow,
        VIEWER => scratch.open_inspector = Some(VIEWER),
        INSPECTOR => scratch.open_inspector = Some(INSPECTOR),
        _ => {
            for (i, (_, show, remove_id, _)) in KEPT_IDS.iter().enumerate() {
                if (id == *show || id == KEPT_TAB_IDS[i]) && i < kept.orbits.len() {
                    scratch.tab = Tab::Kept(i);
                } else if id == *remove_id {
                    remove(i, kept, scratch);
                }
            }
        }
    }
}

/// The arrows in interaction mode: on the playhead, by `Bounded`'s 1/100 of the orbit's span `[0, span]`, kept in it;
/// on the shown tab's real-space view and unwrapped view, a pan by `Pan`'s 1/20 of the view's span; on its sphere, an
/// orbit by `Angle`'s degree, ← → the yaw and ↑ ↓ the pitch, which turns "turn" off as a drag does (R-453 D6).
pub fn adjust(adjust: &Adjust, span: f64, scratch: &mut Scratch) {
    let tab = scratch.tab;
    match (adjust.scope, adjust.delta()) {
        (PLAYHEAD, Base::OfRange(fraction)) => {
            scratch.orbit_t = (scratch.orbit_t + fraction * span).clamp(0.0, span);
        }
        (REAL_VIEW, Base::OfView(fraction)) => {
            scratch.views_mut(tab).real.step(fraction, adjust.direction);
        }
        (SPHERE_VIEW, Base::OfView(fraction)) if scratch.unwrapped => {
            scratch
                .views_mut(tab)
                .unwrapped
                .step(fraction, adjust.direction);
        }
        (SPHERE_VIEW, Base::Degrees(degrees)) if !scratch.unwrapped => {
            let (yaw, pitch) = match adjust.direction {
                Direction::Left | Direction::Right => (degrees, 0.0),
                Direction::Up | Direction::Down => (0.0, degrees),
            };
            scratch.views_mut(tab).sphere.turn(yaw, pitch);
            scratch.turn = false;
        }
        _ => {}
    }
}

/// The zoom keys' steps among `events`: each press of + (or =) or −, as a signed multiple of the base step, Shift ×10
/// and Alt ×0.1.
pub fn zoom_presses(events: &[egui::Event]) -> Vec<f64> {
    events
        .iter()
        .filter_map(|e| match e {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => match key {
                egui::Key::Plus | egui::Key::Equals => Some(multiplier(*modifiers)),
                egui::Key::Minus => Some(-multiplier(*modifiers)),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// The + or − key on view `id` in interaction mode, `times` the base step: a zoom by `ZoomLog2`'s quarter octave, about
/// the view's centre for real space and the unwrapped view (R-453 D6). Nothing on any other scope.
pub fn zoom_view(id: &str, times: f64, scratch: &mut Scratch) {
    let Base::Log2(base) = StepKind::ZoomLog2.base().times(times) else {
        unreachable!("the zoom's base step is in octaves")
    };
    let (tab, unwrapped) = (scratch.tab, scratch.unwrapped);
    let views = match id {
        REAL_VIEW | SPHERE_VIEW => scratch.views_mut(tab),
        _ => return,
    };
    match (id, unwrapped) {
        (REAL_VIEW, _) => views.real.zoom_about(base, [0.0, 0.0]),
        (_, true) => views.unwrapped.zoom_about(base, [0.0, 0.0]),
        (_, false) => views.sphere.zoom_by(base),
    }
}

/// The pointer at `pos` on a view drawn about `centre` with half-widths `half`, in half-widths from its centre.
pub fn in_view(pos: Pos2, centre: Pos2, half: Vec2) -> [f64; 2] {
    [
        f64::from((pos.x - centre.x) / half.x),
        f64::from((pos.y - centre.y) / half.y),
    ]
}

/// The wheel's zoom, in octaves, while the pointer is over `response`'s view, which takes the wheel: the window below
/// it does not scroll as well.
fn wheel_over(ui: &Ui, response: &Response) -> f64 {
    if !response.hovered() {
        return 0.0;
    }
    ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
    ui.input(|i| crate::explore::manifold_view::navigate::wheel_octaves(&i.events))
}

/// The pointer on a pan-and-zoom view drawn about `centre` with half-widths `half` (real space, the unwrapped view;
/// R-453 D7): a drag pans, the drawing following the pointer; the wheel zooms about the pointer; a double-click
/// restores the default.
pub fn pan_zoom_input(ui: &Ui, response: &Response, centre: Pos2, half: Vec2, view: &mut PanZoom) {
    if response.double_clicked() {
        *view = PanZoom::default();
        return;
    }
    if response.dragged_by(egui::PointerButton::Primary) {
        let d = response.drag_delta();
        if d != Vec2::ZERO {
            view.drag([f64::from(d.x / half.x), f64::from(d.y / half.y)]);
        }
    }
    let octaves = wheel_over(ui, response);
    if octaves != 0.0 {
        if let Some(pos) = response.hover_pos() {
            view.zoom_about(octaves, in_view(pos, centre, half));
        }
    }
}

/// The pointer on the sphere (R-453 D7): a drag orbits it, right turning the yaw up and down the pitch, by
/// [`ORBIT_DEG_PER_POINT`], and turns `turn` off so the view holds; the wheel zooms; a double-click restores the default
/// view, the turn's `angle` back at rest too.
pub fn orbit_input(
    ui: &Ui,
    response: &Response,
    orbit: &mut Orbit,
    turn: &mut bool,
    angle: &mut f64,
) {
    if response.double_clicked() {
        *orbit = Orbit::default();
        *angle = 0.0;
        return;
    }
    if response.dragged_by(egui::PointerButton::Primary) {
        let d = response.drag_delta();
        if d != Vec2::ZERO {
            orbit.turn(
                f64::from(d.x) * ORBIT_DEG_PER_POINT,
                f64::from(d.y) * ORBIT_DEG_PER_POINT,
            );
            *turn = false;
        }
    }
    let octaves = wheel_over(ui, response);
    if octaves != 0.0 {
        orbit.zoom_by(octaves);
    }
}

/// The one-line summary: fate, `t_end`, FTLE and `E₀`, "undecided" for the fate while it is not decided.
pub fn summary(sample: &Sample) -> String {
    let fate = match (&sample.fate, sample.t_end) {
        (Some(fate), Some(t_end)) => format!("{fate} · t_end {t_end:.1}"),
        (Some(fate), None) => fate.clone(),
        (None, _) => "undecided".to_owned(),
    };
    format!("{fate} · FTLE {:.3} · E₀ {:.3}", sample.ftle, sample.e0)
}

/// A kept orbit's row: "#1 body 1 escape · t 8.9", its time its `t_end` once decided, else its path's span.
pub fn kept_row(i: usize, sample: &Sample) -> String {
    let fate = sample.fate.as_deref().unwrap_or("undecided");
    let t = sample.t_end.unwrap_or(sample.path_t);
    format!("{} {fate} · t {t:.1}", KEPT_IDS[i].3)
}

/// The point of the shape sphere at `(θ, φ)`: `n = (sin φ cos θ, sin φ sin θ, cos φ)` (chart_reference §3.3).
pub fn on_sphere([theta, phi]: [f64; 2]) -> [f64; 3] {
    [phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos()]
}

/// `p` of the sphere turned `angle` degrees about its pole past its resting azimuth [`REST_ANGLE_DEG`], and seen with
/// the pole tilted [`SPHERE_TILT_DEG`] toward the viewer: its screen point `(x, y)`, `y` up, each in `[−1, 1]`, and its
/// depth toward the viewer, ≥ 0 on the near side.
pub fn view(p: [f64; 3], angle: f64) -> ([f64; 2], f64) {
    view_at(p, angle, SPHERE_TILT_DEG)
}

/// [`view`] with the pole tilted `pitch` degrees toward the viewer: the orbit's view (R-453 D7). It moves where `p` is
/// drawn, never `p`.
pub fn view_at(p: [f64; 3], angle: f64, pitch: f64) -> ([f64; 2], f64) {
    let (sa, ca) = (angle + REST_ANGLE_DEG).to_radians().sin_cos();
    let (st, ct) = pitch.to_radians().sin_cos();
    let x = p[0] * ca - p[1] * sa;
    let y = p[0] * sa + p[1] * ca;
    ([x, p[2] * ct + y * st], p[2] * st - y * ct)
}

/// `(x, y)`, each in `[−1, 1]`, `y` up, on the square `rect`.
pub fn on_square(rect: Rect, [x, y]: [f64; 2]) -> Pos2 {
    let r = rect.width().min(rect.height()) / 2.0;
    rect.center() + vec2(x as f32 * r, -y as f32 * r)
}

/// `(θ, φ)` on the unwrapped view `rect`: `θ` across from 0 to 2π, `φ` down from the pole.
pub fn unwrapped_point(rect: Rect, [theta, phi]: [f64; 2]) -> Pos2 {
    let x = (theta / std::f64::consts::TAU).clamp(0.0, 1.0) as f32;
    let y = (phi / std::f64::consts::PI).clamp(0.0, 1.0) as f32;
    pos2(
        rect.min.x + x * rect.width(),
        rect.min.y + y * rect.height(),
    )
}

/// The orbit playhead slider's width in a row `available` points wide: the row less 70 points for its play button and
/// time readout, but never under 40 points (a look choice, R-390).
pub fn playhead_width(available: f32) -> f32 {
    (available - 70.0).max(40.0)
}

/// Whether a step from `a` to `b` of the unwrapped path wraps across `θ`'s seam, and so is not drawn.
pub fn wraps(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() > std::f64::consts::PI
}

/// The index of the path's point at `t` of a path of `n` points spanning `[0, span]`.
pub fn index_at(t: f64, span: f64, n: usize) -> usize {
    if n == 0 || span <= 0.0 {
        return 0;
    }
    // `f` is at most 1, so the index is at most `n − 1`.
    let f = (t / span).clamp(0.0, 1.0);
    (f * (n - 1) as f64).round() as usize
}

/// The square `[x0, y0, x1, y1]` holding every point of `paths`, centred on their bounds, a little larger; the unit
/// square about a single point, and `[−1, 1]²` for none.
pub fn fit(paths: &[Vec<[f64; 2]>]) -> [f64; 4] {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in paths.iter().flatten() {
        b = [
            b[0].min(p[0]),
            b[1].min(p[1]),
            b[2].max(p[0]),
            b[3].max(p[1]),
        ];
    }
    if !b[0].is_finite() {
        return [-1.0, -1.0, 1.0, 1.0];
    }
    let half = (b[2] - b[0]).max(b[3] - b[1]) / 2.0;
    // A set of one point has no extent: the unit square about it, so the fit never divides by zero.
    let half = if half > 0.0 { half * 1.1 } else { 1.0 };
    let (cx, cy) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
    [cx - half, cy - half, cx + half, cy + half]
}

/// The panel's regions in `rect`: the title row, the summary, real space and the sphere side by side, two squares, the
/// stats below them, and the rows under those.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Areas {
    /// The title and tabs.
    pub title: Rect,
    /// The summary line.
    pub summary: Rect,
    /// Real space.
    pub real: Rect,
    /// The sphere, or the unwrapped view.
    pub sphere: Rect,
    /// The stats below the two views.
    pub stats: Rect,
    /// Everything below: the view toggles, the playhead, listen, the buttons and the kept orbits.
    pub rows: Rect,
}

/// The heights of the title row and the summary row, in points.
pub const TITLE_H: f32 = 28.0;
/// See [`TITLE_H`].
pub const SUMMARY_H: f32 = 18.0;
/// The side of real space's and the sphere's squares, as a fraction of the body's height, unless the two side by side
/// would be wider than the body (a look choice, R-390).
pub const VIEW_FRACTION: f32 = 0.25;
/// The gap between real space and the sphere, in points (a look choice, R-390).
pub const VIEW_GAP: f32 = 12.0;
/// The stats' height below the views, in points: one row of four, each a name over its value (a look choice, R-390).
pub const STATS_H: f32 = 34.0;

/// The regions of a panel in `rect`.
pub fn areas(rect: Rect) -> Areas {
    let body = rect.shrink(MARGIN);
    let side = (body.height() * VIEW_FRACTION).min((body.width() - VIEW_GAP) / 2.0);
    let title = Rect::from_min_size(body.min, vec2(body.width(), TITLE_H));
    let summary = Rect::from_min_size(title.left_bottom(), vec2(body.width(), SUMMARY_H));
    let real = Rect::from_min_size(summary.left_bottom() + vec2(0.0, 4.0), vec2(side, side));
    let sphere = Rect::from_min_size(real.right_top() + vec2(VIEW_GAP, 0.0), vec2(side, side));
    let stats = Rect::from_min_size(
        real.left_bottom() + vec2(0.0, 6.0),
        vec2(body.width(), STATS_H),
    );
    let rows = Rect::from_min_max(pos2(body.min.x, stats.max.y + 6.0), body.max);
    Areas {
        title,
        summary,
        real,
        sphere,
        stats,
        rows,
    }
}

/// What the panel reads and records into.
pub struct Panel<'a> {
    /// The kept orbits, `ViewUI`'s.
    pub kept: &'a mut KeptOrbits,
    /// The panel's scratch state.
    pub scratch: &'a mut Scratch,
    /// The keyboard layer, for the places of the controls.
    pub keyboard: &'a mut Keyboard,
}

/// Draws the panel in `rect`.
pub fn show(ui: &mut Ui, rect: Rect, panel: &mut Panel<'_>) {
    let visuals = ui.visuals().clone();
    ui.painter().rect_filled(rect, 0.0, visuals.panel_fill);
    ui.painter().rect_stroke(
        rect,
        0.0,
        visuals.widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Inside,
    );
    panel.keyboard.place(TRAJECTORY, rect);
    let a = areas(rect);
    tabs(ui, a.title, panel);
    let orbit = shown(panel.scratch.tab, panel.scratch.first(), panel.kept).cloned();
    let mut line = ui.new_child(
        UiBuilder::new()
            .max_rect(a.summary)
            .layout(EguiLayout::left_to_right(Align::Center)),
    );
    if panel.scratch.pin.is_some() {
        line.with_layout(EguiLayout::right_to_left(Align::Center), |r| {
            let unpin = r.small_button(UNPIN_LABEL);
            panel.keyboard.place_widget(UNPIN, &unpin);
            if unpin.clicked() {
                panel.scratch.pin = None;
            }
            r.with_layout(EguiLayout::left_to_right(Align::Center), |l| {
                summary_line(l, orbit.as_ref());
            });
        });
    } else {
        summary_line(&mut line, orbit.as_ref());
    }
    frame(ui, a.real, "real space · CoM frame");
    frame(ui, a.sphere, "");
    views(ui, &a, panel);
    if let Some(orbit) = &orbit {
        let i = index_at(panel.scratch.orbit_t, orbit.path_t, orbit.shape.len());
        let v = panel.scratch.views_of(panel.scratch.tab);
        real_space(ui, a.real.shrink(8.0), orbit, i, &v.real);
        if panel.scratch.unwrapped {
            unwrapped(ui, a.sphere.shrink(8.0), orbit, i, &v.unwrapped);
        } else {
            let angle = panel.scratch.angle;
            sphere(ui, a.sphere.shrink(8.0), orbit, angle, i, &v.sphere);
        }
        stats(ui, a.stats, orbit);
    }
    rows(ui, a.rows, orbit.as_ref(), panel);
}

/// The summary line: the orbit's, or what to do before any.
fn summary_line(ui: &mut Ui, orbit: Option<&Sample>) {
    let text = orbit.map_or(NO_SAMPLE.to_owned(), summary);
    ui.label(RichText::new(text).monospace().weak().small());
}

/// The views' elements and their pointer (R-453 D6 to D8): real space, and the sphere's place, the sphere or the
/// unwrapped view, each sensing drags, the wheel and double-clicks over its frame, for the shown tab's views.
fn views(ui: &mut Ui, a: &Areas, panel: &mut Panel<'_>) {
    let real = ui.interact(a.real, ui.id().with(REAL_VIEW), Sense::click_and_drag());
    let place = ui.interact(a.sphere, ui.id().with(SPHERE_VIEW), Sense::click_and_drag());
    panel.keyboard.place(REAL_VIEW, a.real);
    panel.keyboard.place(SPHERE_VIEW, a.sphere);
    let scratch = &mut *panel.scratch;
    let tab = scratch.tab;
    let (inner_real, inner_sphere) = (a.real.shrink(8.0), a.sphere.shrink(8.0));
    let r = inner_real.width().min(inner_real.height()) / 2.0;
    pan_zoom_input(
        ui,
        &real,
        inner_real.center(),
        vec2(r, r),
        &mut scratch.views_mut(tab).real,
    );
    if scratch.unwrapped {
        let half = inner_sphere.size() / 2.0;
        let centre = inner_sphere.center();
        pan_zoom_input(
            ui,
            &place,
            centre,
            half,
            &mut scratch.views_mut(tab).unwrapped,
        );
    } else {
        let mut orbit = scratch.views_of(tab).sphere;
        orbit_input(
            ui,
            &place,
            &mut orbit,
            &mut scratch.turn,
            &mut scratch.angle,
        );
        scratch.views_mut(tab).sphere = orbit;
    }
}

/// A view's frame, with its caption at its bottom left.
fn frame(ui: &Ui, rect: Rect, caption: &str) {
    let visuals = ui.visuals();
    ui.painter()
        .rect_filled(rect, 0.0, visuals.extreme_bg_color);
    ui.painter().rect_stroke(
        rect,
        0.0,
        visuals.widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Inside,
    );
    if !caption.is_empty() {
        ui.painter().text(
            rect.left_bottom() + vec2(6.0, -6.0),
            egui::Align2::LEFT_BOTTOM,
            caption,
            egui::FontId::monospace(10.0),
            visuals.weak_text_color(),
        );
    }
}

fn tabs(ui: &mut Ui, rect: Rect, panel: &mut Panel<'_>) {
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(EguiLayout::left_to_right(Align::Center)),
    );
    row.label(RichText::new(TITLE).heading());
    row.with_layout(EguiLayout::right_to_left(Align::Center), |r| {
        let keep_tab = r.button("+");
        panel.keyboard.place_widget(KEEP, &keep_tab);
        if keep_tab.clicked() {
            keep_first(panel.kept, panel.scratch);
        }
        for i in (0..panel.kept.orbits.len()).rev() {
            let tab = r.add(Button::selectable(
                panel.scratch.tab == Tab::Kept(i),
                KEPT_IDS[i].3,
            ));
            panel.keyboard.place_widget(KEPT_TAB_IDS[i], &tab);
            if tab.clicked() {
                panel.scratch.tab = Tab::Kept(i);
            }
        }
        let under = r.add(Button::selectable(
            panel.scratch.tab == Tab::UnderCursor,
            first_tab(panel.scratch.pin.is_some()),
        ));
        panel.keyboard.place_widget(UNDER_CURSOR, &under);
        if under.clicked() {
            panel.scratch.tab = Tab::UnderCursor;
        }
    });
}

/// Real space: the three bodies' paths, fitted to `rect` and seen through `view`, each with its position at the
/// playhead.
pub fn real_space(ui: &Ui, rect: Rect, orbit: &Sample, i: usize, view: &PanZoom) {
    let painter = ui.painter_at(rect);
    let [x0, y0, x1, y1] = fit(&orbit.bodies);
    let to = |p: &[f64; 2]| {
        let x = 2.0 * (p[0] - x0) / (x1 - x0) - 1.0;
        let y = 2.0 * (p[1] - y0) / (y1 - y0) - 1.0;
        // The view's `y` is down; real space's up.
        let [vx, vy] = view.apply([x, -y]);
        on_square(rect, [vx, -vy])
    };
    for (path, colour) in orbit.bodies.iter().zip(BODY_COLOURS) {
        let points: Vec<Pos2> = path.iter().map(to).collect();
        painter.add(egui::Shape::line(
            points,
            Stroke::new(1.2, colour.gamma_multiply(0.8)),
        ));
        if let Some(p) = path.get(i) {
            painter.circle_filled(to(p), 3.5, colour);
        }
    }
}

/// The shape sphere turned `angle` degrees and seen through `orbit`'s view: its outline, its x, y, z axes, and the
/// path, its far side faint. The orbit moves where the path is drawn, never the path (R-453 D8).
pub fn sphere(ui: &Ui, rect: Rect, orbit: &Sample, angle: f64, i: usize, view: &Orbit) {
    let painter = ui.painter_at(rect);
    let weak = ui.visuals().weak_text_color();
    let size = 0.86 * view.zoom.exp2();
    let r = rect.width().min(rect.height()) / 2.0;
    painter.circle_stroke(rect.center(), r * size as f32, Stroke::new(1.0, weak));
    let shrink = |p: [f64; 2]| [p[0] * size, p[1] * size];
    let (yaw, pitch) = (angle + view.yaw, view.pitch);
    for (axis, (colour, name)) in AXIS_COLOURS.iter().zip(["x", "y", "z"]).enumerate() {
        let tip = std::array::from_fn(|k| if k == axis { 1.12 } else { 0.0 });
        let (end, _) = view_at(tip, yaw, pitch);
        let end = on_square(rect, shrink(end));
        painter.line_segment([rect.center(), end], Stroke::new(1.2, *colour));
        painter.text(
            end,
            egui::Align2::CENTER_CENTER,
            name,
            egui::FontId::monospace(10.0),
            *colour,
        );
    }
    let points: Vec<([f64; 2], f64)> = orbit
        .shape
        .iter()
        .map(|p| view_at(on_sphere(*p), yaw, pitch))
        .collect();
    let ink = ui.visuals().text_color();
    for w in points.windows(2) {
        let near = w[0].1 >= 0.0 && w[1].1 >= 0.0;
        let colour = if near {
            ink
        } else {
            ink.gamma_multiply(FAR_SIDE)
        };
        painter.line_segment(
            [
                on_square(rect, shrink(w[0].0)),
                on_square(rect, shrink(w[1].0)),
            ],
            Stroke::new(1.2, colour),
        );
    }
    if let Some((p, _)) = points.get(i) {
        painter.circle_filled(on_square(rect, shrink(*p)), 3.5, AXIS_COLOURS[2]);
    }
}

/// Where `view` draws the point `p` of the unwrapped drawing on `rect`.
fn through(rect: Rect, view: &PanZoom, p: Pos2) -> Pos2 {
    let half = rect.size() / 2.0;
    let [x, y] = view.apply(in_view(p, rect.center(), half));
    rect.center() + vec2(x as f32 * half.x, y as f32 * half.y)
}

/// The unwrapped (equirectangular) view, seen through `view`: `θ` across, `φ` down, the path broken where it wraps.
pub fn unwrapped(ui: &Ui, rect: Rect, orbit: &Sample, i: usize, view: &PanZoom) {
    let painter = ui.painter_at(rect);
    let weak = ui.visuals().weak_text_color();
    let to = |p: [f64; 2]| through(rect, view, unwrapped_point(rect, p));
    let map = Rect::from_two_pos(through(rect, view, rect.min), through(rect, view, rect.max));
    painter.rect_stroke(map, 0.0, Stroke::new(1.0, weak), egui::StrokeKind::Inside);
    let ink = ui.visuals().text_color();
    for w in orbit.shape.windows(2) {
        if !wraps(w[0], w[1]) {
            painter.line_segment([to(w[0]), to(w[1])], Stroke::new(1.2, ink));
        }
    }
    if let Some(p) = orbit.shape.get(i) {
        painter.circle_filled(to(*p), 3.5, AXIS_COLOURS[2]);
    }
    let font = egui::FontId::monospace(10.0);
    painter.text(
        map.left_bottom(),
        egui::Align2::LEFT_BOTTOM,
        "θ 0",
        font.clone(),
        weak,
    );
    painter.text(
        map.right_bottom(),
        egui::Align2::RIGHT_BOTTOM,
        "2π",
        font.clone(),
        weak,
    );
    painter.text(map.left_top(), egui::Align2::LEFT_TOP, "φ 0", font, weak);
}

/// The stats in one row below the two views: the F₂ word, substeps, minimum separation and `|ΔE/E|`, each in its cell.
fn stats(ui: &mut Ui, rect: Rect, orbit: &Sample) {
    for (cell, (name, value)) in stat_cells(rect).into_iter().zip([
        ("F₂ word", orbit.f2_word.clone()),
        ("substeps", crate::explore::top_bar::grouped(orbit.substeps)),
        ("min sep", format!("{:.1e}", orbit.min_separation)),
        ("|ΔE/E|", format!("{:.1e}", orbit.energy_drift)),
    ]) {
        let mut column = ui.new_child(UiBuilder::new().max_rect(cell));
        column.spacing_mut().item_spacing.y = 1.0;
        column.label(RichText::new(name).weak().small().monospace());
        // A word longer than its cell is cut with "…", never drawn over the next.
        column.add(egui::Label::new(RichText::new(value).monospace()).truncate());
    }
}

/// The stats' four cells in `rect`, left to right: the F₂ word's two fifths of the width, as the longest value, then
/// substeps, minimum separation and `|ΔE/E|` a fifth each (a look choice, R-390).
pub fn stat_cells(rect: Rect) -> [Rect; 4] {
    let fifth = rect.width() / 5.0;
    let cell = |from: f32, fifths: f32| {
        Rect::from_min_size(
            rect.min + vec2(from * fifth, 0.0),
            vec2(fifths * fifth, rect.height()),
        )
    };
    [
        cell(0.0, 2.0),
        cell(2.0, 1.0),
        cell(3.0, 1.0),
        cell(4.0, 1.0),
    ]
}

fn rows(ui: &mut Ui, rect: Rect, orbit: Option<&Sample>, panel: &mut Panel<'_>) {
    let mut column = ui.new_child(UiBuilder::new().max_rect(rect));
    column.spacing_mut().item_spacing.y = 6.0;
    let scratch = &mut *panel.scratch;
    let keyboard = &mut *panel.keyboard;
    column.horizontal(|row| {
        let sphere = row.add(Button::selectable(!scratch.unwrapped, "sphere"));
        keyboard.place_widget(SPHERE, &sphere);
        if sphere.clicked() {
            scratch.unwrapped = false;
        }
        let flat = row.add(Button::selectable(scratch.unwrapped, "unwrapped"));
        keyboard.place_widget(UNWRAPPED, &flat);
        if flat.clicked() {
            scratch.unwrapped = true;
        }
        let turn = row.checkbox(&mut scratch.turn, "turn");
        keyboard.place_widget(TURN, &turn);
    });
    let span = orbit.map_or(0.0, |o| o.path_t);
    column.horizontal(|row| {
        let label = if scratch.playing { "⏸" } else { "▶" };
        let play = row.add(Button::selectable(scratch.playing, label));
        keyboard.place_widget(PLAY, &play);
        if play.clicked() {
            scratch.playing = !scratch.playing;
        }
        row.spacing_mut().slider_width = playhead_width(row.available_width());
        let mut t = scratch.orbit_t.min(span);
        let slider =
            row.add(Slider::new(&mut t, 0.0..=span.max(f64::MIN_POSITIVE)).show_value(false));
        keyboard.place_widget(PLAYHEAD, &slider);
        if slider.changed() {
            scratch.orbit_t = t;
        }
        row.label(RichText::new(format!("t {:.1}", scratch.orbit_t.min(span))).monospace());
    });
    column.horizontal(|row| {
        let listen = row.add(Button::selectable(scratch.listen, "listen"));
        keyboard.place_widget(LISTEN, &listen);
        if listen.clicked() {
            scratch.listen = !scratch.listen;
        }
        row.label(RichText::new(LISTEN_MAPPING).weak().small());
        let follow = row.checkbox(&mut scratch.follow, "follow cursor");
        keyboard.place_widget(FOLLOW, &follow);
    });
    column.horizontal(|row| {
        let viewer = row.add(Button::new("Open full viewer…"));
        if viewer.clicked() {
            scratch.open_inspector = Some(VIEWER);
        }
        keyboard.place_widget(VIEWER, &viewer);
        let inspector = row.add(Button::new("IC Inspector…"));
        if inspector.clicked() {
            scratch.open_inspector = Some(INSPECTOR);
        }
        keyboard.place_widget(INSPECTOR, &inspector);
    });
    let header = column.label(RichText::new("Kept orbits").strong());
    let mut list_rect = header.rect;
    let mut removed = None;
    for (i, kept) in panel.kept.orbits.iter().enumerate() {
        let row = column.horizontal(|row| {
            let show = row.add(
                Button::selectable(
                    scratch.tab == Tab::Kept(i),
                    RichText::new(kept_row(i, kept)).monospace(),
                )
                .frame(false),
            );
            if show.clicked() {
                scratch.tab = Tab::Kept(i);
            }
            keyboard.place_widget(KEPT_IDS[i].1, &show);
            row.with_layout(EguiLayout::right_to_left(Align::Center), |r| {
                let x = r.small_button("×");
                keyboard.place_widget(KEPT_IDS[i].2, &x);
                if x.clicked() {
                    removed = Some(i);
                }
            });
        });
        keyboard.place(KEPT_IDS[i].0, row.response.rect);
        list_rect = list_rect.union(row.response.rect);
    }
    keyboard.place(KEPT, list_rect);
    if let Some(i) = removed {
        remove(i, panel.kept, scratch);
    }
}
