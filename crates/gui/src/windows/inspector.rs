//! The Inspector's frame (render_gui_spec §G8; GUI_DESIGN_NOTES § "05 Inspector"; 05_inspectors.png; R-65): one IC,
//! its trajectory, one timeline. The tabs Inspect and Create & locate; the gauge buttons (Reset, Randomise, Burrau-ish,
//! Rotate 15°, Scale × 1.25, Boost →); the three panes, 1 the IC (the bodies 0, 1, 2 with their velocity arrows and
//! ghost markers at the playhead), 2 the canonical representative (the shape sphere, turning with its axes, or
//! unwrapped), 3 the trajectory in real space; one timeline scrubbing all three, with listen; the readouts; the pair
//! separations; and Create & locate's lock the view on it, centre a slice through it, mark it, keep it.
//!
//! On the mock (applied per R-369): the IC is the sample the Trajectory panel shows when the window opens (under the
//! cursor, or a kept orbit), copied in; with none, the panes are empty. Rotate, Scale and Boost move pane 1's drawing
//! of the IC and Reset undoes them, the canonical pane not moving: the gauge quotient, shown. Randomise, Burrau-ish,
//! lock the view on it and mark it are frames, the IC's editing and its re-integration (`computeIC`) being
//! TASK-M8-14's onward. Centre a slice through it sends one `SetField` on `z₀`; keep it keeps the orbit, as the
//! Trajectory panel's keep does. The panes' and the timeline's arrows scrub the one timeline by `TimeStep`, in
//! interaction mode.

use eframe::egui::{self, vec2, Button, Pos2, Rect, RichText, Sense, Slider, Stroke, Ui};
use engine::contract::sample::Sample;
use engine::contract::set_field::SetField;
use engine::contract::view_ui::KeptOrbits;

use crate::explore::trajectory::{self, fit, index_at, on_square, BODY_COLOURS};
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
    tree.register(Some(WINDOW), Scope::group(TIMELINE_SECTION, "timeline"));
    tree.register(
        Some(TIMELINE_SECTION),
        Scope::value(TIMELINE, "timeline", StepKind::TimeStep),
    );
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
    let [x0, y0, x1, y1] = fit(&sample.bodies);
    let (sa, ca) = gauge.0.to_radians().sin_cos();
    let to = |p: &[f64; 2]| -> Pos2 {
        let x = 2.0 * (p[0] - x0) / (x1 - x0) - 1.0;
        let y = 2.0 * (p[1] - y0) / (y1 - y0) - 1.0;
        let (x, y) = (x * ca - y * sa, x * sa + y * ca);
        on_square(rect, [x * gauge.1 * 0.6 + gauge.2, y * gauge.1 * 0.6])
    };
    let weak = ui.visuals().weak_text_color();
    let starts: Vec<Pos2> = sample
        .bodies
        .iter()
        .filter_map(|b| b.first().map(to))
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

/// Draws the window's content; its clicks act as Enter does. `turn` is the sphere's angle, in degrees, as the
/// Trajectory panel turns it.
pub fn show(
    ui: &mut Ui,
    keys: &mut Keys<'_>,
    state: &mut State,
    turn: f64,
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
                    let (rect, pane) = ui.allocate_exact_size(vec2(side, side), Sense::click());
                    ui.painter()
                        .rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
                    let pane = pane.on_hover_text(*caption);
                    keys.place(id, &pane);
                    let Some(sample) = &state.sample else {
                        continue;
                    };
                    match (k, state.unwrapped) {
                        (0, _) => ic_pane(ui, rect, sample, state.gauge, i),
                        (1, false) => trajectory::sphere(ui, rect, sample, turn, i),
                        (1, true) => trajectory::unwrapped(ui, rect, sample, i),
                        _ => trajectory::real_space(ui, rect, sample, i),
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
            });
        })
        .response;
    keys.place_rect(ui, PANES, panes.rect);
    if state.sample.is_none() {
        note(ui, NO_IC);
    }
    let span = state.sample.as_ref().map_or(0.0, |s| s.path_t);
    let timeline = ui
        .horizontal(|ui| {
            let scrub = ui.add_enabled(
                state.sample.is_some(),
                Slider::new(&mut state.t, 0.0..=span.max(f64::MIN_POSITIVE)).show_value(false),
            );
            keys.place(TIMELINE, &scrub);
            ui.label(RichText::new(format!("t {:.2} / {span:.2}", state.t)).monospace());
            let listen = ui.add(Button::selectable(state.listen, "listen"));
            if listen.clicked() {
                clicked.push(LISTEN);
            }
            keys.place(LISTEN, &listen);
        })
        .response;
    keys.place_rect(ui, TIMELINE_SECTION, timeline.rect);
    note(ui, TIMELINE_NOTE);
    if let Some(s) = &state.sample {
        egui::Grid::new("inspector readouts").show(ui, |ui| {
            let z: Vec<String> = s.ic.iter().map(|v| format!("{v:+.3}")).collect();
            for (name, value) in [
                // Four to a line: the eight in one line are wider than the window.
                (
                    "latent z",
                    format!("{}\n{}", z[..4].join(" "), z[4..].join(" ")),
                ),
                (
                    "outcome",
                    s.fate.clone().unwrap_or_else(|| "undecided".into()),
                ),
                ("F₂ word", s.f2_word.clone()),
                ("E₀", format!("{:.3}", s.e0)),
                ("|ΔE/E|", format!("{:.1e}", s.energy_drift)),
                ("min separation", format!("{:.3}", s.min_separation)),
            ] {
                ui.label(RichText::new(name).monospace().weak());
                ui.label(RichText::new(value).monospace());
                ui.end_row();
            }
        });
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
