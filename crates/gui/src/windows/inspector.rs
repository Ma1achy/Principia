//! The Inspector's frame (render_gui_spec §G8; GUI_DESIGN_NOTES § "05 Inspector"; 05_inspectors.png; R-65): one IC,
//! its trajectory, one timeline. The tabs Inspect and Create & locate; the gauge buttons (Reset, Randomise, Burrau-ish,
//! Rotate 15°, Scale × 1.25, Boost →); the three panes, 1 the IC (the bodies 0, 1, 2 with their velocity arrows and
//! ghost markers at the playhead), 2 the canonical representative (the shape sphere, turning with its axes, or
//! unwrapped, with "ghost the gauge transform"), 3 the trajectory in real space; one timeline scrubbing all three, with
//! play, step and listen; the readouts (latent `z`, `α, β`, masses, `E · L_z`, outcome, F₂ word, round trip, gauge,
//! as trajectory_viewing §4's readout row); the energy error and pair separation plots; and Create & locate's lock the
//! view on it, centre a slice through it, mark it, keep it.
//!
//! On the mock (applied per R-369): the IC is the sample the Trajectory panel shows when the window opens (under the
//! cursor, or a kept orbit), copied in; with none, the panes are empty. Rotate, Scale and Boost move pane 1's drawing
//! of the IC and Reset undoes them, the canonical pane not moving: the gauge quotient, shown. Randomise, Burrau-ish,
//! lock the view on it and mark it are frames, the IC's editing and its re-integration (`computeIC`) being
//! TASK-M8-14's onward. Centre a slice through it sends one `SetField` on `z₀`; keep it keeps the orbit, as the
//! Trajectory panel's keep does. The panes' and the timeline's arrows scrub the one timeline by `TimeStep`, in
//! interaction mode; play runs it at one `t` a second, as the Trajectory panel's play does, and step moves it a point of
//! the orbit's path. turn turns the sphere at the panel's rate. Ghost draws, faint over pane 2, the IC as the gauge
//! buttons left it, beside the canonical pane that has not moved. The readouts are the sample's where it has them
//! (`z`, outcome, word, `E₀`, `α, β` as its shape-sphere point at `t = 0`, chart_decoder_contract Part 1), and
//! placeholder content where it does not: the masses as a softmax of `z[6:8]` and a pinned 0, `L_z` from the paths'
//! first step under them, the round trip a hash of the IC under 10⁻¹⁴, and the gauge error 0, the mock's canonical
//! pane ignoring the gauge exactly; the energy error grows to the sample's `|ΔE/E|` as the pairs close, the real
//! readouts being `computeIC`'s (TASK-M8-14).
//!
//! Panes 2 and 3 are the Trajectory panel's view widgets, so they take its pointer (R-453 D8): on pane 3 and on the
//! unwrapped pane 2 a drag pans and the wheel zooms about the pointer, on the sphere a drag orbits, turning "turn" off,
//! and the wheel zooms, and a double-click restores a pane's default; the window's own [`trajectory::Views`].

use eframe::egui::{self, vec2, Button, Pos2, Rect, RichText, Sense, Slider, Stroke, Ui};
use engine::contract::sample::Sample;
use engine::contract::set_field::SetField;
use engine::contract::view_ui::KeptOrbits;

use crate::explore::trajectory::{self, fit, index_at, on_square, turned, BODY_COLOURS};
use crate::keyboard::scopes::{Adjust, Base, Scope, ScopeId, ScopeTree, StepKind};
use crate::windows::{note, Keys};

/// The window's scope.
pub const WINDOW: ScopeId = "win_inspector";
/// Its title, as 05_inspectors.png's.
pub const TITLE: &str = "Inspector · IC & trajectory";

/// The tabs' section.
pub const TABS: ScopeId = "in_tabs";
/// Inspect.
pub const INSPECT: ScopeId = "in_inspect";
/// Create & locate.
pub const CREATE: ScopeId = "in_create";
/// The gauge buttons' section.
pub const GAUGE: ScopeId = "in_gauge";
/// The panes' section.
pub const PANES: ScopeId = "in_panes";
/// The timeline's section.
pub const TIMELINE_SECTION: ScopeId = "in_timeline_section";
/// The timeline.
pub const TIMELINE: ScopeId = "in_timeline";
/// Play, on the timeline.
pub const PLAY: ScopeId = "in_play";
/// Step back a point of the path.
pub const STEP_BACK: ScopeId = "in_step_back";
/// Step on a point of the path.
pub const STEP: ScopeId = "in_step";
/// turn: the sphere turning.
pub const TURN: ScopeId = "in_turn";
/// Ghost the gauge transform.
pub const GHOST: ScopeId = "in_ghost";
/// Listen.
pub const LISTEN: ScopeId = "in_listen";
/// Locate's section.
pub const LOCATE: ScopeId = "in_locate";

/// The gauge buttons, in order, each with whether the mock acts on it.
pub const GAUGE_IDS: [(ScopeId, &str, bool); 6] = [
    ("in_reset", "Reset", true),
    ("in_randomise", "Randomise", false),
    ("in_burrau", "Burrau-ish", false),
    ("in_rotate", "Rotate 15°", true),
    ("in_scale", "Scale × 1.25", true),
    ("in_boost", "Boost →", true),
];
/// The three panes, in order, with their captions (05_inspectors.png).
pub const PANE_IDS: [(ScopeId, &str); 3] = [
    ("in_pane_ic", "1 · the IC · drag bodies and arrows"),
    ("in_pane_canonical", "2 · canonical · shape sphere"),
    (
        "in_pane_trajectory",
        "3 · its trajectory · real space · CoM frame",
    ),
];
/// The canonical pane's views: the sphere, turning, or unwrapped.
pub const SPHERE: ScopeId = "in_sphere";
/// See [`SPHERE`].
pub const UNWRAPPED: ScopeId = "in_unwrapped";
/// Locate's buttons, in order, each with whether the mock acts on it.
pub const LOCATE_IDS: [(ScopeId, &str, bool); 4] = [
    ("in_lock", "Lock view on this IC", false),
    ("in_centre", "Centre slice through it", true),
    ("in_mark", "Mark it in the figure", false),
    ("in_keep", "Keep", true),
];
/// The line under the timeline (05_inspectors.png).
pub const TIMELINE_NOTE: &str =
    "one timeline for all three · editing the IC re-integrates the orbit live";
/// The panes' line with no IC.
pub const NO_IC: &str = "no IC: open the Inspector from the Trajectory panel with an orbit shown";

/// The gauge steps (§G8's buttons): a rotation, in degrees, a scale and a boost, in pane units.
pub const ROTATE_DEG: f64 = 15.0;
/// See [`ROTATE_DEG`].
pub const SCALE: f64 = 1.25;
/// See [`ROTATE_DEG`].
pub const BOOST: f64 = 0.1;

/// The Inspector's own state: the GUI's.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// The IC inspected and its orbit.
    pub sample: Option<Sample>,
    /// The timeline's time.
    pub t: f64,
    /// Create & locate (`true`) or Inspect.
    pub create: bool,
    /// The canonical pane unwrapped (`true`) or the sphere.
    pub unwrapped: bool,
    /// Listen.
    pub listen: bool,
    /// Pane 1's gauge transform: the rotation, in degrees, the scale and the boost.
    pub gauge: (f64, f64, f64),
    /// turn: the sphere turning.
    pub turn: bool,
    /// The sphere's turn so far, in degrees, in `[0, 360)`.
    pub angle: f64,
    /// Ghost the gauge transform.
    pub ghost: bool,
    /// Whether the timeline plays.
    pub playing: bool,
    /// The time source's seconds at the last frame.
    pub last_seconds: Option<f64>,
    /// Panes 2's and 3's views: real space, the sphere and the unwrapped view (R-453 D8).
    pub views: trajectory::Views,
}

impl Default for State {
    fn default() -> Self {
        Self {
            sample: None,
            t: 0.0,
            create: false,
            unwrapped: false,
            listen: false,
            gauge: (0.0, 1.0, 0.0),
            turn: true,
            angle: 0.0,
            ghost: false,
            playing: false,
            last_seconds: None,
            views: trajectory::Views::default(),
        }
    }
}

impl State {
    /// Seeds the window with `sample`, its timeline at 0 and pane 1's gauge reset.
    pub fn seed(&mut self, sample: Option<&Sample>) {
        *self = Self {
            sample: sample.cloned(),
            ..Self::default()
        };
    }

    /// One frame at `seconds`, as the Trajectory panel's: the sphere turns while turn is on and the sphere shown, and
    /// the timeline plays on at one `t` a second, stopping at the orbit's end.
    pub fn advance(&mut self, seconds: f64) {
        let dt = self
            .last_seconds
            .map_or(0.0, |last| (seconds - last).max(0.0));
        self.last_seconds = Some(seconds);
        if self.turn && !self.unwrapped {
            self.angle = turned(self.angle, trajectory::TURN_DEG_PER_S, dt);
        }
        let span = self.sample.as_ref().map_or(0.0, |s| s.path_t);
        if self.playing {
            self.t = (self.t + dt).min(span);
            if self.t >= span {
                self.playing = false;
            }
        }
    }

    /// Whether the window moves by itself, so the app asks for frames: with an IC, the sphere turning or the timeline
    /// playing.
    pub fn moving(&self) -> bool {
        self.sample.is_some() && ((self.turn && !self.unwrapped) || self.playing)
    }

    /// The timeline moved `by` points of the orbit's path, kept on it, and paused.
    pub fn step(&mut self, by: i64) {
        let Some(sample) = &self.sample else {
            return;
        };
        let n = sample.shape.len();
        if n < 2 {
            return;
        }
        let i = index_at(self.t, sample.path_t, n) as i64;
        let j = (i + by).clamp(0, n as i64 - 1);
        self.t = sample.path_t * j as f64 / (n - 1) as f64;
        self.playing = false;
    }
}

/// Joins the window's sections and elements to `tree`, in order.
pub fn register(tree: &mut ScopeTree) {
    tree.register(Some(WINDOW), Scope::group(TABS, "tabs"));
    tree.register(Some(TABS), Scope::control(INSPECT, "Inspect"));
    tree.register(Some(TABS), Scope::control(CREATE, "Create & locate"));
    tree.register(Some(WINDOW), Scope::group(GAUGE, "gauge"));
    for (id, label, acts) in GAUGE_IDS {
        let scope = if acts {
            Scope::control(id, label)
        } else {
            Scope::disabled(id, label)
        };
        tree.register(Some(GAUGE), scope);
    }
    tree.register(Some(WINDOW), Scope::group(PANES, "panes"));
    for (id, label) in PANE_IDS {
        tree.register(Some(PANES), Scope::value(id, label, StepKind::TimeStep));
    }
    tree.register(Some(PANES), Scope::control(SPHERE, "sphere"));
    tree.register(Some(PANES), Scope::control(UNWRAPPED, "unwrapped"));
    tree.register(Some(PANES), Scope::control(TURN, "turn"));
    tree.register(
        Some(PANES),
        Scope::control(GHOST, "ghost the gauge transform"),
    );
    tree.register(Some(WINDOW), Scope::group(TIMELINE_SECTION, "timeline"));
    tree.register(
        Some(TIMELINE_SECTION),
        Scope::value(TIMELINE, "timeline", StepKind::TimeStep),
    );
    tree.register(Some(TIMELINE_SECTION), Scope::control(PLAY, "play"));
    tree.register(
        Some(TIMELINE_SECTION),
        Scope::control(STEP_BACK, "step back"),
    );
    tree.register(Some(TIMELINE_SECTION), Scope::control(STEP, "step"));
    tree.register(Some(TIMELINE_SECTION), Scope::control(LISTEN, "listen"));
    tree.register(Some(WINDOW), Scope::group(LOCATE, "Locate"));
    for (id, label, acts) in LOCATE_IDS {
        let scope = if acts {
            Scope::control(id, label)
        } else {
            Scope::disabled(id, label)
        };
        tree.register(Some(LOCATE), scope);
    }
}

/// Enter on control `id`, as its click: centre pushes its `SetField` to `edits`, keep keeps the orbit in `kept`.
pub fn activate(id: &str, state: &mut State, kept: &mut KeptOrbits, edits: &mut Vec<SetField>) {
    let (angle, scale, boost) = &mut state.gauge;
    match id {
        INSPECT => state.create = false,
        CREATE => state.create = true,
        SPHERE => state.unwrapped = false,
        UNWRAPPED => state.unwrapped = true,
        LISTEN => state.listen = !state.listen,
        TURN => state.turn = !state.turn,
        GHOST => state.ghost = !state.ghost,
        PLAY => {
            if let Some(s) = &state.sample {
                // Play at the end starts over.
                if !state.playing && state.t >= s.path_t {
                    state.t = 0.0;
                }
                state.playing = !state.playing;
            }
        }
        STEP_BACK => state.step(-1),
        STEP => state.step(1),
        "in_reset" => state.gauge = (0.0, 1.0, 0.0),
        "in_rotate" => *angle += ROTATE_DEG,
        "in_scale" => *scale *= SCALE,
        "in_boost" => *boost += BOOST,
        "in_centre" => {
            if let Some(s) = &state.sample {
                edits.push(crate::explore::manifold_view::z0_edit(s.ic));
            }
        }
        "in_keep" => {
            if let Some(s) = &state.sample {
                if !kept.orbits.contains(s) && kept.orbits.len() < trajectory::MAX_KEPT {
                    kept.orbits.push(s.clone());
                }
            }
        }
        _ => {}
    }
}

/// The arrows on a pane or the timeline, in interaction mode: `TimeStep` steps of `[0, path_t]`, kept on it.
pub fn adjust(adjust: &Adjust, state: &mut State) {
    let scrubs = adjust.scope == TIMELINE || PANE_IDS.iter().any(|(p, _)| *p == adjust.scope);
    let (Some(sample), Base::OfRange(fraction), true) = (&state.sample, adjust.delta(), scrubs)
    else {
        return;
    };
    state.t = (state.t + fraction * sample.path_t).clamp(0.0, sample.path_t);
}

/// Pane 1: the IC, its three bodies at `t = 0` under the gauge transform, joined, each with its velocity arrow and a
/// ghost marker at the playhead's index `i`.
pub(crate) fn ic_pane(ui: &Ui, rect: Rect, sample: &Sample, gauge: (f64, f64, f64), i: usize) {
    let painter = ui.painter_at(rect);
    let to = ic_to(rect, sample, gauge);
    let weak = ui.visuals().weak_text_color();
    let starts: Vec<Pos2> = sample
        .bodies
        .iter()
        .filter_map(|b| b.first().map(&to))
        .collect();
    if starts.len() == 3 {
        for k in 0..3 {
            painter.line_segment([starts[k], starts[(k + 1) % 3]], Stroke::new(1.0, weak));
        }
    }
    for (body, (path, colour)) in sample.bodies.iter().zip(BODY_COLOURS).enumerate() {
        let (Some(a), Some(b)) = (path.first(), path.get(4)) else {
            continue;
        };
        let (a, b) = (to(a), to(b));
        painter.line_segment([a, a + (b - a) * 3.0], Stroke::new(1.5, colour));
        painter.circle_filled(a, 8.0, colour);
        painter.text(
            a,
            egui::Align2::CENTER_CENTER,
            body.to_string(),
            egui::FontId::monospace(9.0),
            egui::Color32::BLACK,
        );
        if let Some(g) = path.get(i) {
            painter.circle_stroke(to(g), 5.0, Stroke::new(1.0, colour));
        }
    }
}

/// Where pane 1 draws a point of `sample`'s paths in `rect` under `gauge`: fitted to the paths, then rotated, scaled
/// and boosted.
fn ic_to(rect: Rect, sample: &Sample, gauge: (f64, f64, f64)) -> impl Fn(&[f64; 2]) -> Pos2 {
    let [x0, y0, x1, y1] = fit(&sample.bodies);
    let (sa, ca) = gauge.0.to_radians().sin_cos();
    move |p: &[f64; 2]| -> Pos2 {
        let x = 2.0 * (p[0] - x0) / (x1 - x0) - 1.0;
        let y = 2.0 * (p[1] - y0) / (y1 - y0) - 1.0;
        let (x, y) = (x * ca - y * sa, x * sa + y * ca);
        on_square(rect, [x * gauge.1 * 0.6 + gauge.2, y * gauge.1 * 0.6])
    }
}

/// The ghost of the gauge transform over pane 2's `rect`: the IC's three bodies at `t = 0` as pane 1 draws them under
/// `gauge`, faint rings joined, so what the gauge buttons did shows beside the canonical pane that does not move.
pub(crate) fn gauge_ghost(ui: &Ui, rect: Rect, sample: &Sample, gauge: (f64, f64, f64)) {
    let painter = ui.painter_at(rect);
    let to = ic_to(rect, sample, gauge);
    let starts: Vec<Pos2> = sample
        .bodies
        .iter()
        .filter_map(|b| b.first().map(&to))
        .collect();
    let faint = ui.visuals().weak_text_color().gamma_multiply(GHOST_ALPHA);
    if starts.len() == 3 {
        for k in 0..3 {
            painter.line_segment([starts[k], starts[(k + 1) % 3]], Stroke::new(1.0, faint));
        }
    }
    for (a, colour) in starts.iter().zip(BODY_COLOURS) {
        painter.circle_stroke(
            *a,
            8.0,
            Stroke::new(1.0, colour.gamma_multiply(GHOST_ALPHA)),
        );
    }
}

/// The ghost's opacity (a look choice, R-390).
pub const GHOST_ALPHA: f32 = 0.45;

/// The minimum pair separation at each point of `sample`'s paths.
fn closest(sample: &Sample) -> Vec<f64> {
    let b = &sample.bodies;
    let n = b.iter().map(Vec::len).min().unwrap_or(0);
    (0..n)
        .map(|k| {
            [(0, 1), (1, 2), (0, 2)]
                .map(|(p, q): (usize, usize)| {
                    let (a, c) = (b[p][k], b[q][k]);
                    ((a[0] - c[0]).powi(2) + (a[1] - c[1]).powi(2)).sqrt()
                })
                .into_iter()
                .fold(f64::INFINITY, f64::min)
        })
        .collect()
}

/// The energy error over the orbit (placeholder content): growing from 0 to the sample's `|ΔE/E|`, each step by the
/// inverse of the pairs' closest separation there, so it climbs at the close encounters, as an integrator's does.
pub fn energy_error(sample: &Sample) -> Vec<f64> {
    let weights: Vec<f64> = closest(sample).iter().map(|d| 1.0 / d.max(1e-9)).collect();
    let total: f64 = weights.iter().sum::<f64>().max(f64::MIN_POSITIVE);
    let mut sum = 0.0;
    weights
        .iter()
        .map(|w| {
            sum += w;
            sample.energy_drift * sum / total
        })
        .collect()
}

/// The energy error plot: [`energy_error`] over the orbit, against its largest, in the hyperlink colour.
pub(crate) fn energy_plot(ui: &mut Ui, sample: &Sample) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
    let e = energy_error(sample);
    let top = e.iter().copied().fold(f64::MIN_POSITIVE, f64::max);
    let n = e.len();
    let points: Vec<Pos2> = e
        .iter()
        .enumerate()
        .map(|(k, v)| {
            let x = rect.min.x + rect.width() * k as f32 / (n.max(2) - 1) as f32;
            egui::pos2(x, rect.max.y - rect.height() * (v / top) as f32)
        })
        .collect();
    painter.add(egui::Shape::line(
        points,
        Stroke::new(1.0, ui.visuals().hyperlink_color),
    ));
}

/// The masses (placeholder content): a softmax of `z[6]`, `z[7]` and a pinned 0, the mass block's two controls onto the
/// simplex (chart_decoder_contract Part 1, `D_mass`).
pub fn masses(sample: &Sample) -> [f64; 3] {
    let e = [sample.ic[6].exp(), sample.ic[7].exp(), 1.0];
    let sum: f64 = e.iter().sum();
    e.map(|x| x / sum)
}

/// `L_z` (placeholder content): `Σ m (x v_y − y v_x)` over the bodies under [`masses`], their velocities the paths'
/// first step over its `t`; 0 for a path under two points.
pub fn l_z(sample: &Sample) -> f64 {
    let m = masses(sample);
    let n = sample.bodies.iter().map(Vec::len).min().unwrap_or(0);
    if n < 2 {
        return 0.0;
    }
    let dt = sample.path_t / (n - 1) as f64;
    sample
        .bodies
        .iter()
        .zip(m)
        .map(|(path, m)| {
            let (a, b) = (path[0], path[1]);
            let v = [(b[0] - a[0]) / dt, (b[1] - a[1]) / dt];
            m * (a[0] * v[1] - a[1] * v[0])
        })
        .sum()
}

/// The readouts, in trajectory_viewing §4's order, each `(name, value)`, for the sample's IC.
pub fn readouts(s: &Sample) -> Vec<(&'static str, String)> {
    let z: Vec<String> = s.ic.iter().map(|v| format!("{v:+.3}")).collect();
    let [alpha, beta] = s.shape.first().copied().unwrap_or([f64::NAN; 2]);
    let m = masses(s);
    // A placeholder round trip, under 10⁻¹⁴ (the IC Inspector's verified bound, its scratchpad).
    let round_trip = 1e-15 * (1.0 + 9.0 * crate::mock::sample::hash(s.ic.iter().sum::<f64>()));
    vec![
        // Four to a line: the eight in one line are wider than the window.
        (
            "latent z",
            format!("{}\n{}", z[..4].join(" "), z[4..].join(" ")),
        ),
        ("α, β", format!("{alpha:.3} · {beta:.3}")),
        ("masses", format!("{:.3} · {:.3} · {:.3}", m[0], m[1], m[2])),
        ("E · L_z", format!("{:.3} · {:+.3}", s.e0, l_z(s))),
        (
            "outcome",
            s.fate.clone().unwrap_or_else(|| "undecided".into()),
        ),
        ("F₂ word", s.f2_word.clone()),
        ("round trip", format!("{round_trip:.1e}")),
        ("gauge", format!("{:.1e}", 0.0)),
        ("|ΔE/E|", format!("{:.1e}", s.energy_drift)),
        ("min separation", format!("{:.1e}", s.min_separation)),
    ]
}

/// The pair separations over the orbit, each pair in its first body's colour.
pub(crate) fn separations(ui: &mut Ui, sample: &Sample) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
    let b = &sample.bodies;
    let n = b.iter().map(Vec::len).min().unwrap_or(0);
    let pairs = [(0, 1), (1, 2), (0, 2)];
    let d = |p: usize, q: usize, k: usize| {
        let (a, c) = (b[p][k], b[q][k]);
        ((a[0] - c[0]).powi(2) + (a[1] - c[1]).powi(2)).sqrt()
    };
    let top = (0..n)
        .flat_map(|k| pairs.map(|(p, q)| d(p, q, k)))
        .fold(f64::MIN_POSITIVE, f64::max);
    for (p, q) in pairs {
        let points: Vec<Pos2> = (0..n)
            .map(|k| {
                let x = rect.min.x + rect.width() * k as f32 / (n.max(2) - 1) as f32;
                egui::pos2(x, rect.max.y - rect.height() * (d(p, q, k) / top) as f32)
            })
            .collect();
        painter.add(egui::Shape::line(points, Stroke::new(1.0, BODY_COLOURS[p])));
    }
}

/// A row of buttons, each `(id, label, selected, enabled)`, placed with their section's scope; a click pushes the id to
/// `clicked`.
fn row(
    ui: &mut Ui,
    keys: &mut Keys<'_>,
    section: ScopeId,
    items: &[(ScopeId, &str, bool, bool)],
    clicked: &mut Vec<ScopeId>,
) {
    let response = ui
        .horizontal_wrapped(|ui| {
            for &(id, label, selected, enabled) in items {
                let button = ui.add_enabled(enabled, Button::selectable(selected, label));
                if button.clicked() {
                    clicked.push(id);
                }
                keys.place(id, &button);
            }
        })
        .response;
    keys.place_rect(ui, section, response.rect);
}

/// Draws the window's content; its clicks act as Enter does.
pub fn show(
    ui: &mut Ui,
    keys: &mut Keys<'_>,
    state: &mut State,
    (kept, edits): (&mut KeptOrbits, &mut Vec<SetField>),
) {
    let mut clicked: Vec<ScopeId> = Vec::new();
    row(
        ui,
        keys,
        TABS,
        &[
            (INSPECT, "Inspect", !state.create, true),
            (CREATE, "Create & locate", state.create, true),
        ],
        &mut clicked,
    );
    let gauge: Vec<_> = GAUGE_IDS
        .iter()
        .map(|&(id, l, acts)| (id, l, false, acts))
        .collect();
    row(ui, keys, GAUGE, &gauge, &mut clicked);
    let i = state
        .sample
        .as_ref()
        .map_or(0, |s| index_at(state.t, s.path_t, s.shape.len()));
    let panes = ui
        .scope(|ui| {
            let gap = ui.spacing().item_spacing.x;
            let side = ((ui.available_width() - 2.0 * gap) / 3.0).max(16.0);
            ui.horizontal(|ui| {
                for (k, (id, caption)) in PANE_IDS.iter().enumerate() {
                    let (rect, pane) =
                        ui.allocate_exact_size(vec2(side, side), Sense::click_and_drag());
                    ui.painter()
                        .rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
                    let pane = pane.on_hover_text(*caption);
                    keys.place(id, &pane);
                    let Some(sample) = &state.sample else {
                        continue;
                    };
                    let r = rect.width().min(rect.height()) / 2.0;
                    let v = &mut state.views;
                    match (k, state.unwrapped) {
                        (0, _) => ic_pane(ui, rect, sample, state.gauge, i),
                        (1, false) => {
                            let (turn, angle) = (&mut state.turn, &mut state.angle);
                            trajectory::orbit_input(ui, &pane, &mut v.sphere, turn, angle);
                            trajectory::sphere(ui, rect, sample, state.angle, i, &v.sphere);
                        }
                        (1, true) => {
                            let half = rect.size() / 2.0;
                            let unwrapped = &mut v.unwrapped;
                            trajectory::pan_zoom_input(ui, &pane, rect.center(), half, unwrapped);
                            trajectory::unwrapped(ui, rect, sample, i, &v.unwrapped);
                        }
                        _ => {
                            let real = &mut v.real;
                            trajectory::pan_zoom_input(ui, &pane, rect.center(), vec2(r, r), real);
                            trajectory::real_space(ui, rect, sample, i, &v.real);
                        }
                    }
                    if k == 1 && state.ghost {
                        gauge_ghost(ui, rect, sample, state.gauge);
                    }
                }
            });
            ui.horizontal(|ui| {
                for (id, label, on) in [
                    (SPHERE, "sphere", !state.unwrapped),
                    (UNWRAPPED, "unwrapped", state.unwrapped),
                ] {
                    let button = ui.add(Button::selectable(on, label));
                    if button.clicked() {
                        clicked.push(id);
                    }
                    keys.place(id, &button);
                }
                let turn = ui.checkbox(&mut state.turn, "turn");
                keys.place(TURN, &turn);
                let ghost = ui.checkbox(&mut state.ghost, "ghost the gauge transform");
                keys.place(GHOST, &ghost);
            });
        })
        .response;
    keys.place_rect(ui, PANES, panes.rect);
    if state.sample.is_none() {
        note(ui, NO_IC);
    }
    let span = state.sample.as_ref().map_or(0.0, |s| s.path_t);
    let timeline = ui
        .scope(|ui| {
            let has = state.sample.is_some();
            // The scrubber on its own row, across the window; the transport, the time and listen under it.
            ui.spacing_mut().slider_width = ui.available_width();
            let scrub = ui.add_enabled(
                has,
                Slider::new(&mut state.t, 0.0..=span.max(f64::MIN_POSITIVE)).show_value(false),
            );
            keys.place(TIMELINE, &scrub);
            ui.horizontal(|ui| {
                for (id, label, on) in [
                    // ⏵ and ⏴, a pair the font has both of.
                    (PLAY, if state.playing { "⏸" } else { "⏵" }, state.playing),
                    (STEP_BACK, "⏴ step", false),
                    (STEP, "step ⏵", false),
                ] {
                    let button = ui.add_enabled(has, Button::selectable(on, label));
                    if button.clicked() {
                        clicked.push(id);
                    }
                    keys.place(id, &button);
                }
                ui.label(RichText::new(format!("t {:.2} / {span:.2}", state.t)).monospace());
                let listen = ui.add(Button::selectable(state.listen, "listen"));
                if listen.clicked() {
                    clicked.push(LISTEN);
                }
                keys.place(LISTEN, &listen);
            });
        })
        .response;
    keys.place_rect(ui, TIMELINE_SECTION, timeline.rect);
    note(ui, TIMELINE_NOTE);
    if let Some(s) = &state.sample {
        egui::Grid::new("inspector readouts").show(ui, |ui| {
            for (name, value) in readouts(s) {
                ui.label(RichText::new(name).monospace().weak());
                ui.label(RichText::new(value).monospace());
                ui.end_row();
            }
        });
        ui.label(RichText::new("▾ Energy error |ΔE/E|").strong());
        energy_plot(ui, s);
        ui.label(RichText::new("▾ Pair separations").strong());
        separations(ui, s);
    }
    let locate: Vec<_> = LOCATE_IDS
        .iter()
        .map(|&(id, l, acts)| (id, l, false, acts && state.sample.is_some()))
        .collect();
    ui.label(RichText::new("▾ Locate").strong());
    row(ui, keys, LOCATE, &locate, &mut clicked);
    note(
        ui,
        "Create & locate: build an IC here, see where it lands in z, then lock or centre on it",
    );
    for id in clicked {
        activate(id, state, kept, edits);
    }
}
