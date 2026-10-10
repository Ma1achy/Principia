//! The left panel, "Manifold view": ONE group (render_gui_spec §G2; GUI_DESIGN_NOTES § "01 Explore"; 01_main.png;
//! R-390, ORDER item 3). Chart, Navigate with the depth readout, its precision warning and the lock, Centre z₀ and
//! Slice & tilt, top to bottom, are one thing: how you view the manifold. Every edit is a `SetField` on `z₀`, on the
//! basis `q₁, q₂` or on the lock: navigation is chart construction, with no camera (§G1; chart_decoder_contract
//! Part 4). The panel reads the snapshot and keeps nothing of the state itself.
//!
//! The chart is the affine slice `z(s,t) = z₀ + (2s−1) q₁ + (2t−1) q₂` (chart_decoder_contract Part 3). The panel reads
//! it in a preset's frame: the two axes the preset is named by, `e_a` and `e_b`, and a hidden slice direction `e_d`.
//! In that frame `q₁ = σ (cos γ u₁ + sin γ u₂)` and `q₂ = σ (−sin γ u₁ + cos γ u₂)`, with `u₁ = cos τ₁ e_a + sin τ₁ e_d`
//! and `u₂ = cos τ₂ e_b + sin τ₂ e_d`: the scale `σ` (the zoom is `−log₂ σ`), the tilts `τ₁`, `τ₂` toward the slice
//! direction (Part 4, "Tilt") and the in-plane rotation `γ`. The slice step is `z₀`'s component along `e_d`
//! (Part 4, "Slice"). [`View::of`] reads them off the basis and [`View::basis`] writes them back, so each slider is
//! one basis edit.
//!
//! The panel's layout is fixed: each section takes a share of the panel by its rows ([`sections`]), so the keyboard's
//! ring on a section (TASK-M6-25) and the controls inside it agree at any window size.

pub mod centre;
pub mod chart;
pub mod depth;
pub mod lock;
pub mod navigate;
pub mod slice_tilt;

use eframe::egui::{
    self, pos2, vec2, Align, Layout as EguiLayout, Rect, Response, RichText, Ui, UiBuilder,
};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Lock, Plane, LATENT_DIM};
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::Window;

use crate::explore::compass::{self, Compass};
use crate::keyboard::scopes::{Adjust, Base, Scope, ScopeId, ScopeTree};
use crate::keyboard::Keyboard;

/// The eight controls' short names, in chart_decoder_contract Part 2's block order (render_gui_spec §G2's "Centre
/// z₀: eight sliders").
pub const AXES: [&str; LATENT_DIM] = ["z_α", "z_β", "z_q0", "z_q1", "z_q2", "z_q3", "z_μ1", "z_μ2"];

/// The affine presets, each the pair of axes it is named by (§G2: "named by its axes, never a nickname"). Placeholder
/// content on the mock (R-390): the preset names on the real engine are REQ-GUI-082's, TASK-M8-06's.
pub const PRESETS: [(usize, usize); 5] = [(0, 1), (2, 3), (4, 5), (6, 7), (0, 6)];

/// The hidden direction slicing and tilting move along: `z_q3`, as 01_main.png's compass names its depth axis
/// (applied per R-369: the direction library is TASK-M8-07's), or the first axis the preset leaves hidden when the
/// preset shows `z_q3`.
pub const SLICE_AXIS: usize = 5;

/// The span of a `z₀` slider and of the slice step: `[−1, 1]`, the signed, centred plane of Part 3; locked, the span
/// is re-based to the anchor, `anchor ± 1` (a look choice, R-390).
pub const Z0_HALF_RANGE: f64 = 1.0;
/// The span a basis component's field is stepped across by the keyboard's bounded step (a look choice, R-390).
pub const Q_HALF_RANGE: f64 = 1.0;
/// The zoom slider's span, in log₂ (a look choice, R-390): four octaves out, forty-eight in.
pub const ZOOM_RANGE: (f64, f64) = (-4.0, 48.0);

/// Manifold view's sub-scopes (render_gui_spec §G3), in order, with their ids.
pub const CHART: ScopeId = "chart";
/// Navigate.
pub const NAVIGATE: ScopeId = "navigate";
/// Centre z₀.
pub const CENTRE_Z0: ScopeId = "centre_z0";
/// Slice & tilt.
pub const SLICE_TILT: ScopeId = "slice_tilt";

/// The preset selector.
pub const PRESET: ScopeId = "chart_preset";
/// `q₁`'s edit button, its components under it.
pub const Q1_EDIT: ScopeId = "chart_q1";
/// `q₂`'s edit button.
pub const Q2_EDIT: ScopeId = "chart_q2";
/// Chart builder…, drawn disabled until TASK-M6-28 builds its window.
pub const BUILDER: ScopeId = "chart_builder";
/// `q₁`'s eight components, under its edit button.
pub const Q1_COMPONENTS: [ScopeId; LATENT_DIM] = [
    "q1_0", "q1_1", "q1_2", "q1_3", "q1_4", "q1_5", "q1_6", "q1_7",
];
/// `q₂`'s eight components.
pub const Q2_COMPONENTS: [ScopeId; LATENT_DIM] = [
    "q2_0", "q2_1", "q2_2", "q2_3", "q2_4", "q2_5", "q2_6", "q2_7",
];
/// The centre `u`.
pub const CENTRE_U: ScopeId = "centre_u";
/// The centre `v`.
pub const CENTRE_V: ScopeId = "centre_v";
/// The zoom, in log₂.
pub const ZOOM: ScopeId = "zoom";
/// Navigate's eight `z₀` fields.
pub const NAV_Z0: [ScopeId; LATENT_DIM] = [
    "nav_z0_0", "nav_z0_1", "nav_z0_2", "nav_z0_3", "nav_z0_4", "nav_z0_5", "nav_z0_6", "nav_z0_7",
];
/// The lock badge's unlock.
pub const UNLOCK: ScopeId = "lock_unlock";
/// The lock badge's open in Inspector.
pub const OPEN_INSPECTOR: ScopeId = "lock_inspector";
/// Centre z₀'s eight sliders.
pub const SLIDERS: [ScopeId; LATENT_DIM] = [
    "centre_z0_0",
    "centre_z0_1",
    "centre_z0_2",
    "centre_z0_3",
    "centre_z0_4",
    "centre_z0_5",
    "centre_z0_6",
    "centre_z0_7",
];
/// The slice step.
pub const SLICE_STEP: ScopeId = "slice_step";
/// The tilt `τ₁`.
pub const TAU1: ScopeId = "tau1";
/// The tilt `τ₂`.
pub const TAU2: ScopeId = "tau2";
/// The rotation `γ`.
pub const GAMMA: ScopeId = "gamma";

/// The rows each section takes, header included: Chart 5; Navigate 9 (centre, zoom, two of `z₀`, depth, precision,
/// the lock badge and its anchor); Centre z₀ 9; Slice & tilt 5.
pub const SECTION_ROWS: [usize; 4] = [5, 9, 9, 5];
/// The height the Manifold view's title and subtitle take above its sections, in points.
pub const TITLE_H: f32 = 40.0;
/// The gap between the sections, in points.
pub const SECTION_GAP: f32 = 6.0;
/// The panel's margin, in points.
pub const MARGIN: f32 = 12.0;
/// The tallest a row grows, in points, when the panel has room.
pub const MAX_ROW_H: f32 = 22.0;

/// The height of one row in `panel`.
pub fn row_height(panel: Rect) -> f32 {
    let body = panel.shrink(MARGIN);
    let rows: usize = SECTION_ROWS.iter().sum();
    let free = body.height() - TITLE_H - 3.0 * SECTION_GAP;
    (free / rows as f32).clamp(0.0, MAX_ROW_H)
}

/// The sections' rects in `panel`, top to bottom: Chart, Navigate, Centre z₀, Slice & tilt, each as tall as its rows.
pub fn sections(panel: Rect) -> [Rect; 4] {
    let body = panel.shrink(MARGIN);
    let h = row_height(panel);
    let mut y = body.min.y + TITLE_H;
    std::array::from_fn(|i| {
        let rect = Rect::from_min_size(
            pos2(body.min.x, y),
            vec2(body.width(), h * SECTION_ROWS[i] as f32),
        );
        y = rect.max.y + SECTION_GAP;
        rect
    })
}

/// Row `i` of `section`, `h` tall.
pub fn row(section: Rect, h: f32, i: usize) -> Rect {
    Rect::from_min_size(
        pos2(section.min.x, section.min.y + h * i as f32),
        vec2(section.width(), h),
    )
}

/// A child `Ui` laid out left to right in `rect`, its widgets centred on the row.
pub fn row_ui(ui: &mut Ui, rect: Rect) -> Ui {
    ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(EguiLayout::left_to_right(Align::Center)),
    )
}

/// Joins the panel's controls to `tree`, under the four sub-scopes, in the order they are drawn, with the rows the
/// figure's and the compass's own keys add to the `?` overlay. Idempotent: the panel joins when it is first drawn.
pub fn join(tree: &mut ScopeTree) {
    use crate::keyboard::scopes::StepKind::{Angle, Bounded, Pan, ZoomLog2};
    if tree.get(PRESET).is_some() {
        return;
    }
    tree.register(Some(CHART), Scope::control(PRESET, "preset"));
    for (edit, label, components) in [
        (Q1_EDIT, "q₁ edit…", Q1_COMPONENTS),
        (Q2_EDIT, "q₂ edit…", Q2_COMPONENTS),
    ] {
        tree.register(Some(CHART), Scope::control(edit, label));
        for (id, name) in components.into_iter().zip(AXES) {
            tree.register(Some(edit), Scope::value(id, name, Bounded));
        }
    }
    tree.register(Some(CHART), Scope::disabled(BUILDER, "Chart builder…"));
    tree.register(Some(NAVIGATE), Scope::value(CENTRE_U, "centre u", Pan));
    tree.register(Some(NAVIGATE), Scope::value(CENTRE_V, "centre v", Pan));
    tree.register(Some(NAVIGATE), Scope::value(ZOOM, "zoom", ZoomLog2));
    for (id, name) in NAV_Z0.into_iter().zip(AXES) {
        tree.register(Some(NAVIGATE), Scope::value(id, name, Bounded));
    }
    tree.register(Some(NAVIGATE), Scope::control(UNLOCK, "unlock"));
    tree.register(
        Some(NAVIGATE),
        Scope::control(OPEN_INSPECTOR, "open in Inspector"),
    );
    for (id, name) in SLIDERS.into_iter().zip(AXES) {
        tree.register(Some(CENTRE_Z0), Scope::value(id, name, Bounded));
    }
    tree.register(
        Some(SLICE_TILT),
        Scope::value(SLICE_STEP, "slice step", Bounded),
    );
    tree.register(Some(SLICE_TILT), Scope::value(TAU1, "τ₁", Angle));
    tree.register(Some(SLICE_TILT), Scope::value(TAU2, "τ₂", Angle));
    tree.register(Some(SLICE_TILT), Scope::value(GAMMA, "γ", Angle));
    for (key, action) in SHORTCUTS {
        tree.register_shortcut(key, action);
    }
}

/// The rows the Manifold view, the figure and the compass add to the `?` overlay (render_gui_spec §G3's "In scope").
pub const SHORTCUTS: [(&str, &str); 6] = [
    (
        "Enter on a value",
        "arrows: adjust it, or move between its siblings",
    ),
    ("Figure: arrows", "pan"),
    ("Figure: + / −", "zoom"),
    ("K", "lock at the point under the pointer"),
    ("Compass: arrows", "tilt"),
    ("Compass: Shift+arrows", "orbit"),
];

// --- the chart in a preset's frame ------------------------------------------------------------------------------------

/// The length of `v`.
pub fn norm(v: &Latent) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// `a · b`.
pub fn dot(a: &Latent, b: &Latent) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// `a + k b`.
pub fn add(a: &Latent, k: f64, b: &Latent) -> Latent {
    std::array::from_fn(|i| a[i] + k * b[i])
}

/// The unit vector along axis `i`, `k` long.
pub fn axis(i: usize, k: f64) -> Latent {
    std::array::from_fn(|j| if j == i { k } else { 0.0 })
}

/// The hidden slice direction of the preset `(a, b)`: [`SLICE_AXIS`], or the first axis neither shows.
pub fn slice_axis(a: usize, b: usize) -> usize {
    if a != SLICE_AXIS && b != SLICE_AXIS {
        return SLICE_AXIS;
    }
    (0..LATENT_DIM)
        .find(|i| *i != a && *i != b)
        .expect("eight axes, two shown")
}

/// The axis `v` leans on most; the first of equals.
pub fn dominant(v: &Latent) -> usize {
    (0..LATENT_DIM).fold(
        0,
        |best, i| if v[i].abs() > v[best].abs() { i } else { best },
    )
}

/// The chart, as the panel reads it in a preset's frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// The preset whose frame holds the basis, by its index in [`PRESETS`]; `None` for a basis edited out of every
    /// preset's frame, read in the frame of its dominant axes.
    pub preset: Option<usize>,
    /// The axis `q₁` is named by.
    pub a: usize,
    /// The axis `q₂` is named by.
    pub b: usize,
    /// The hidden slice and tilt direction.
    pub d: usize,
    /// The scale `σ`.
    pub scale: f64,
    /// The tilt `τ₁`, in radians.
    pub tau1: f64,
    /// The tilt `τ₂`, in radians.
    pub tau2: f64,
    /// The rotation `γ`, in radians.
    pub gamma: f64,
}

impl View {
    /// The view of `plane`'s basis: the first preset whose frame holds it, or its dominant axes' frame.
    pub fn of(plane: &Plane) -> Self {
        let (q1, q2) = (&plane.q1, &plane.q2);
        let total = dot(q1, q1) + dot(q2, q2);
        let outside = |a: usize, b: usize, d: usize| -> f64 {
            (0..LATENT_DIM)
                .filter(|i| ![a, b, d].contains(i))
                .map(|i| q1[i] * q1[i] + q2[i] * q2[i])
                .sum()
        };
        let found = PRESETS.iter().position(|&(a, b)| {
            outside(a, b, slice_axis(a, b)) <= 1e-18 * total.max(f64::MIN_POSITIVE)
        });
        let (a, b) = match found {
            Some(p) => PRESETS[p],
            None => {
                let a = dominant(q1);
                let mut q2_rest = *q2;
                q2_rest[a] = 0.0;
                (a, dominant(&q2_rest))
            }
        };
        Self::in_frame(found, a, b, slice_axis(a, b), q1, q2)
    }

    /// The scale, tilts and rotation of `q1`, `q2` in the frame `(a, b, d)`.
    fn in_frame(
        preset: Option<usize>,
        a: usize,
        b: usize,
        d: usize,
        q1: &Latent,
        q2: &Latent,
    ) -> Self {
        let cos1 = q1[a].hypot(q2[a]);
        let cos2 = q1[b].hypot(q2[b]);
        let gamma = if cos1 >= cos2 {
            (-q2[a]).atan2(q1[a])
        } else {
            q1[b].atan2(q2[b])
        };
        let (sin_g, cos_g) = gamma.sin_cos();
        let sin1 = cos_g * q1[d] - sin_g * q2[d];
        let sin2 = sin_g * q1[d] + cos_g * q2[d];
        // `+ 0.0` reads a −0 angle as 0, as the readouts show it.
        Self {
            preset,
            a,
            b,
            d,
            scale: cos1.hypot(sin1),
            tau1: sin1.atan2(cos1) + 0.0,
            tau2: sin2.atan2(cos2) + 0.0,
            gamma: gamma + 0.0,
        }
    }

    /// The basis `(q₁, q₂)` of this view.
    pub fn basis(&self) -> (Latent, Latent) {
        let (s1, c1) = self.tau1.sin_cos();
        let (s2, c2) = self.tau2.sin_cos();
        let (sg, cg) = self.gamma.sin_cos();
        let u1 = add(&axis(self.a, c1), s1, &axis(self.d, 1.0));
        let u2 = add(&axis(self.b, c2), s2, &axis(self.d, 1.0));
        let q1 = add(&u1.map(|x| x * cg * self.scale), sg * self.scale, &u2);
        let q2 = add(&u2.map(|x| x * cg * self.scale), -sg * self.scale, &u1);
        (q1, q2)
    }

    /// The zoom, in log₂: `−log₂ σ`.
    pub fn zoom(&self) -> f64 {
        -self.scale.log2()
    }

    /// This view at zoom `zoom`.
    pub fn with_zoom(self, zoom: f64) -> Self {
        Self {
            scale: (-zoom).exp2(),
            ..self
        }
    }

    /// The preset's name, by its axes (§G2), or the dominant axes' for an edited basis.
    pub fn name(&self) -> String {
        format!("{} × {}", AXES[self.a], AXES[self.b])
    }
}

/// The in-plane coordinates of `z` along `q̂₁` and `q̂₂`, the unit basis directions: the `(u, v)` with `z`'s projection
/// on the plane's span `u q̂₁ + v q̂₂`. `None` for a degenerate basis.
pub fn in_plane(plane: &Plane, z: &Latent) -> Option<(f64, f64)> {
    let (n1, n2) = (norm(&plane.q1), norm(&plane.q2));
    if n1 == 0.0 || n2 == 0.0 {
        return None;
    }
    let (h1, h2) = (plane.q1.map(|x| x / n1), plane.q2.map(|x| x / n2));
    let g = dot(&h1, &h2);
    let det = 1.0 - g * g;
    if det <= 1e-12 {
        return None;
    }
    let (c1, c2) = (dot(z, &h1), dot(z, &h2));
    Some(((c1 - g * c2) / det, (c2 - g * c1) / det))
}

/// The chart point `z(s, t) = z₀ + (2s−1) q₁ + (2t−1) q₂`, `t` Y-up (chart_decoder_contract Part 3).
pub fn point(plane: &Plane, s: f64, t: f64) -> Latent {
    add(
        &add(&plane.z0, 2.0 * s - 1.0, &plane.q1),
        2.0 * t - 1.0,
        &plane.q2,
    )
}

/// Where `z` falls in the chart, `(s, t)`: the inverse of [`point`] on the plane through `z₀`, `z`'s out-of-plane part
/// dropped. `None` for a degenerate basis.
pub fn chart_coords(plane: &Plane, z: &Latent) -> Option<(f64, f64)> {
    let r = add(z, -1.0, &plane.z0);
    let (g11, g12, g22) = (
        dot(&plane.q1, &plane.q1),
        dot(&plane.q1, &plane.q2),
        dot(&plane.q2, &plane.q2),
    );
    let det = g11 * g22 - g12 * g12;
    if det <= 1e-24 * (g11 * g22).max(f64::MIN_POSITIVE) {
        return None;
    }
    let (c1, c2) = (dot(&r, &plane.q1), dot(&r, &plane.q2));
    let (x, y) = ((g22 * c1 - g12 * c2) / det, (g11 * c2 - g12 * c1) / det);
    Some(((x + 1.0) / 2.0, (y + 1.0) / 2.0))
}

// --- the edits --------------------------------------------------------------------------------------------------------

/// A `SetField` on `z₀`.
pub fn z0_edit(z0: Latent) -> SetField {
    sim(SimField::Z0(z0))
}

/// A `SetField` on the basis.
pub fn basis_edit((q1, q2): (Latent, Latent)) -> SetField {
    sim(SimField::Basis { q1, q2 })
}

/// A `SetField` on the lock.
pub fn lock_edit(lock: Lock) -> SetField {
    sim(SimField::Lock(lock))
}

fn sim(field: SimField) -> SetField {
    SetField {
        edit: Edit::Sim(field),
        no_history: false,
    }
}

/// Which the compass shows (render_gui_spec §G2: it "switches mode by itself").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Touch {
    /// A slice slider was touched last: `z₀` or the slice step.
    #[default]
    Slice,
    /// A tilt was touched last: `τ₁`, `τ₂`, `γ`, or the compass's plane.
    Tilt,
}

/// The GUI's own scratch state for the panel and the compass: never the engine's, never undoable (R-69).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scratch {
    /// The compass's mode, its orbit and its drag.
    pub compass: Compass,
    /// Whether `q₁`'s and `q₂`'s editors are open.
    pub editing: [bool; 2],
    /// The figure's point the right-click menu was opened on, in `(s, t)`.
    pub menu_at: Option<(f64, f64)>,
}

/// What the panel, the compass and the figure ask for in one frame.
#[derive(Debug, Default, PartialEq)]
pub struct Out {
    /// The edits, in order.
    pub edits: Vec<SetField>,
    /// The windows to open.
    pub windows: Vec<Window>,
}

impl Out {
    /// An edit touching `touch`, recorded in `scratch`'s compass.
    pub fn edit(&mut self, edit: SetField, touch: Option<Touch>, scratch: &mut Scratch) {
        if let Some(touch) = touch {
            scratch.compass.mode = touch;
        }
        self.edits.push(edit);
    }
}

/// Enter on control `id`, as its click.
pub fn activate(id: &str, snapshot: &Snapshot, scratch: &mut Scratch, out: &mut Out) {
    match id {
        PRESET => {
            let view = View::of(&snapshot.sim.plane);
            let next = view.preset.map_or(0, |p| (p + 1) % PRESETS.len());
            out.edit(chart::preset_basis(view, next), None, scratch);
        }
        UNLOCK => lock::unlock(&snapshot.sim.lock, out),
        OPEN_INSPECTOR => lock::open_inspector(&snapshot.sim.lock, out),
        _ => {}
    }
}

/// An adjustment of the arrows on one of the panel's values, the figure or the compass.
pub fn adjust(adjust: &Adjust, snapshot: &Snapshot, scratch: &mut Scratch, out: &mut Out) {
    let plane = &snapshot.sim.plane;
    let step = adjust.delta();
    let (range, octaves, degrees, view_fraction) = match step {
        Base::OfRange(f) => (f, 0.0, 0.0, 0.0),
        Base::Log2(f) => (0.0, f, 0.0, 0.0),
        Base::Degrees(f) => (0.0, 0.0, f, 0.0),
        Base::OfView(f) => (0.0, 0.0, 0.0, f),
    };
    let id = adjust.scope;
    let span = 2.0 * Z0_HALF_RANGE;
    if let Some(i) = NAV_Z0.iter().chain(&SLIDERS).position(|s| *s == id) {
        let i = i % LATENT_DIM;
        let mut z0 = plane.z0;
        z0[i] += range * span;
        out.edit(z0_edit(z0), Some(Touch::Slice), scratch);
        return;
    }
    for (components, which) in [(Q1_COMPONENTS, 0), (Q2_COMPONENTS, 1)] {
        if let Some(i) = components.iter().position(|s| *s == id) {
            let (mut q1, mut q2) = (plane.q1, plane.q2);
            [&mut q1, &mut q2][which][i] += range * 2.0 * Q_HALF_RANGE;
            out.edit(basis_edit((q1, q2)), None, scratch);
            return;
        }
    }
    let view = View::of(plane);
    match id {
        CENTRE_U => out.edit(
            z0_edit(add(&plane.z0, 2.0 * view_fraction, &plane.q1)),
            None,
            scratch,
        ),
        CENTRE_V => out.edit(
            z0_edit(add(&plane.z0, 2.0 * view_fraction, &plane.q2)),
            None,
            scratch,
        ),
        ZOOM => out.edit(
            basis_edit(view.with_zoom(view.zoom() + octaves).basis()),
            None,
            scratch,
        ),
        SLICE_STEP => {
            let mut z0 = plane.z0;
            z0[view.d] += range * span;
            out.edit(z0_edit(z0), Some(Touch::Slice), scratch);
        }
        TAU1 | TAU2 | GAMMA => {
            let mut angles = slice_tilt::Angles::of(&view);
            angles.turn(id, degrees);
            out.edit(
                basis_edit(angles.apply(view).basis()),
                Some(Touch::Tilt),
                scratch,
            );
        }
        crate::explore::FIGURE => navigate::pan_by_keys(adjust, plane, view_fraction, scratch, out),
        crate::explore::COMPASS => compass::adjust(adjust, plane, scratch, out),
        _ => {}
    }
}

/// What the panel draws from, and what it records into.
pub struct Panel<'a> {
    /// The latest snapshot.
    pub snapshot: &'a Snapshot,
    /// The scratch state.
    pub scratch: &'a mut Scratch,
    /// The keyboard layer, for the places of the controls.
    pub keyboard: &'a mut Keyboard,
    /// The focus path.
    pub focus: &'a [String],
    /// What the controls ask for.
    pub out: &'a mut Out,
}

impl Panel<'_> {
    /// Places control `id` at `response`, its rect cut to the panel's.
    pub fn place(&mut self, id: ScopeId, response: &Response, clip: Rect) {
        let mut placed = response.clone();
        placed.rect = response.rect.intersect(clip);
        self.keyboard.place_widget(id, &placed);
    }

    /// Whether the focus is at `id` or inside it.
    pub fn focus_in(&self, id: &str) -> bool {
        self.focus.iter().any(|f| f == id)
    }
}

/// The panel's title, subtitle and section headers' text.
pub const TITLE: &str = "Manifold view";
/// Under the title, as 01_main.png reads.
pub const SUBTITLE: &str = "chart · navigate · slice · tilt · rotation";
/// The sections' titles, in order.
pub const SECTION_TITLES: [&str; 4] = ["Chart", "Navigate", "Centre z₀", "Slice & tilt"];

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
    let body = rect.shrink(MARGIN);
    let mut title = ui.new_child(UiBuilder::new().max_rect(body));
    title.spacing_mut().item_spacing.y = 2.0;
    title.label(RichText::new(TITLE).heading());
    title.label(RichText::new(SUBTITLE).weak());
    panel.keyboard.place(crate::explore::MANIFOLD_VIEW, rect);
    let h = row_height(rect);
    let [chart_rect, navigate_rect, centre_rect, slice_rect] = sections(rect);
    for ((id, _), section) in crate::explore::MANIFOLD_SECTIONS.iter().zip(sections(rect)) {
        panel.keyboard.place(id, section);
    }
    let clip = rect;
    chart::show(ui, chart_rect, h, clip, panel);
    navigate::show(ui, navigate_rect, h, clip, panel);
    centre::show(ui, centre_rect, h, clip, panel);
    slice_tilt::show(ui, slice_rect, h, clip, panel);
}

/// Draws a section's header in row 0 of `section`: its title, and a weak note beside it.
pub fn header(ui: &mut Ui, section: Rect, h: f32, title: &str, note: &str) {
    let mut row = row_ui(ui, row(section, h, 0));
    row.label(RichText::new(title).strong());
    if !note.is_empty() {
        row.add_space(8.0);
        row.label(RichText::new(note).weak().small());
    }
}

/// A latent value as a field shows it: three decimals.
pub fn value_text(v: f64) -> String {
    format!("{v:.3}")
}
