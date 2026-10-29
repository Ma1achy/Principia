//! The left panel, "Manifold view": ONE group (render_gui_spec §G2; GUI_DESIGN_NOTES § "01 Explore"; 01_main.png;
//! R-390, ORDER item 3). Chart, Navigate with the depth readout, its precision warning and the lock, Slice centre and
//! Tilt, top to bottom, are one thing: how you view the manifold (named "Centre z₀" and "Slice & tilt" before R-457).
//! Every edit is a `SetField` on `z₀`, on the basis `q₁, q₂`, on the slice frame or on the lock: navigation is chart
//! construction, with no camera (§G1; chart_decoder_contract Part 4). The panel reads the snapshot and keeps nothing of
//! the state itself.
//!
//! The chart is the affine slice `z(s,t) = z₀ + (2s−1) q₁ + (2t−1) q₂` (chart_decoder_contract Part 3). The panel reads
//! it in a preset's frame: the two axes the preset is named by, `e_a` and `e_b`, and the slice frame's three hidden
//! dimensions (gui_state_contract §2; R-457): the slice normal `e_d`, the dimension the slice moves through, and the
//! targets `e_X` and `e_Y` the tilts turn into. In that frame `q₁ = σ (cos γ u₁ + sin γ u₂)` and
//! `q₂ = σ (−sin γ u₁ + cos γ u₂)`, with `u₁ = cos τ₁ e_a + sin τ₁ e_X` and `u₂ = cos τ₂ e_b + sin τ₂ e_Y`: the scale `σ`
//! (the zoom is `−log₂ σ`), the tilts `τ₁`, `τ₂`, each toward its own target (Part 4, "Tilt"), and the in-plane
//! rotation `γ`. With `X = Y = d`, the frame's defaults, it is the map the panel had before R-457 exactly. The move
//! along the normal is `z₀`'s component on `e_d`, the Slice centre slider of that dimension (Part 4, "Slice").
//! [`View::read`] reads them off the basis and [`View::basis`] writes them back, so each slider is one basis edit.
//!
//! The panel's layout is fixed: each section takes a share of the panel by its rows ([`sections`]), so the keyboard's
//! ring on a section (TASK-M6-25) and the controls inside it agree at any window size.

pub mod centre;
pub mod chart;
pub mod depth;
pub mod lock;
pub mod navigate;
pub mod tilt;

use eframe::egui::{
    self, pos2, vec2, Align, Layout as EguiLayout, Rect, Response, RichText, Ui, UiBuilder,
};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Lock, Plane, SimConfig, SliceFrame, LATENT_DIM};
use engine::contract::snapshot::Snapshot;
use engine::contract::view_ui::Window;

use crate::explore::compass::{self, Compass};
use crate::keyboard::scopes::{Adjust, Base, Scope, ScopeId, ScopeTree};
use crate::keyboard::Keyboard;

/// The eight controls' short names, in chart_decoder_contract Part 2's block order (render_gui_spec §G2's "Slice
/// centre: eight sliders").
pub const AXES: [&str; LATENT_DIM] = ["z_α", "z_β", "z_q0", "z_q1", "z_q2", "z_q3", "z_μ1", "z_μ2"];

/// The affine presets, each the pair of axes it is named by (§G2: "named by its axes, never a nickname"). Placeholder
/// content on the mock (R-390): the preset names on the real engine are REQ-GUI-082's, TASK-M8-06's. The last,
/// `z_q3 × z_μ1`, holds the default slice normal `z_q3`, as `tilt_targets` needs one (applied per R-369, REQ-GUI-205).
pub const PRESETS: [(usize, usize); 6] = [(0, 1), (2, 3), (4, 5), (6, 7), (0, 6), (5, 6)];

/// The default slice normal and tilt target, the slice axis: `z_q3`, as 01_main.png's compass names its depth axis
/// (applied per R-369: the direction library is TASK-M8-07's), or the first axis the preset leaves hidden when the
/// preset shows `z_q3` (gui_state_contract §2's default for the slice frame).
pub const SLICE_AXIS: usize = engine::contract::sim_config::Z_Q3 as usize;

/// The span of a `z₀` slider: `[−1, 1]`, the signed, centred plane of Part 3; locked, the span is re-based to the
/// anchor, `anchor ± 1` (a look choice, R-390).
pub const Z0_HALF_RANGE: f64 = 1.0;
/// The span a basis component's field is stepped across by the keyboard's bounded step (a look choice, R-390).
pub const Q_HALF_RANGE: f64 = 1.0;
/// The zoom slider's span, in log₂ (a look choice, R-390): four octaves out, forty-eight in.
pub const ZOOM_RANGE: (f64, f64) = (-4.0, 48.0);

/// Manifold view's sub-scopes (render_gui_spec §G3), in order, with their ids.
pub const CHART: ScopeId = "chart";
/// Navigate.
pub const NAVIGATE: ScopeId = "navigate";
/// Slice centre (named "Centre z₀" before R-457).
pub const SLICE_CENTRE: ScopeId = "slice_centre";
/// Tilt (named "Slice & tilt" before R-457).
pub const TILT: ScopeId = "tilt";

/// The preset selector.
pub const PRESET: ScopeId = "chart_preset";
/// `q₁`'s edit button, its components under it.
pub const Q1_EDIT: ScopeId = "chart_q1";
/// `q₂`'s edit button.
pub const Q2_EDIT: ScopeId = "chart_q2";
/// Chart builder…, which opens its window (render_gui_spec §G7).
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
/// The lock badge's unlock.
pub const UNLOCK: ScopeId = "lock_unlock";
/// The lock badge's open in Inspector.
pub const OPEN_INSPECTOR: ScopeId = "lock_inspector";
/// Slice centre's eight sliders.
pub const SLIDERS: [ScopeId; LATENT_DIM] = [
    "slice_centre_0",
    "slice_centre_1",
    "slice_centre_2",
    "slice_centre_3",
    "slice_centre_4",
    "slice_centre_5",
    "slice_centre_6",
    "slice_centre_7",
];
/// The tilt `τ₁`.
pub const TAU1: ScopeId = "tau1";
/// `τ₁`'s target dropdown, "tilt τ₁ into \[X\]" (R-457 E5): a menu, its entries the dimensions it offers.
pub const TILT_X: ScopeId = "tilt_x";
/// The tilt `τ₂`.
pub const TAU2: ScopeId = "tau2";
/// `τ₂`'s target dropdown, "tilt τ₂ into \[Y\]".
pub const TILT_Y: ScopeId = "tilt_y";
/// The rotation `γ`.
pub const GAMMA: ScopeId = "gamma";
/// `τ₁`'s dropdown's entries, one per latent dimension; it holds the six [`target_choices`] offers.
pub const TILT_X_ENTRIES: [ScopeId; LATENT_DIM] = [
    "tilt_x_0", "tilt_x_1", "tilt_x_2", "tilt_x_3", "tilt_x_4", "tilt_x_5", "tilt_x_6", "tilt_x_7",
];
/// `τ₂`'s dropdown's entries.
pub const TILT_Y_ENTRIES: [ScopeId; LATENT_DIM] = [
    "tilt_y_0", "tilt_y_1", "tilt_y_2", "tilt_y_3", "tilt_y_4", "tilt_y_5", "tilt_y_6", "tilt_y_7",
];

/// The rows each section takes, header included: Chart 5; Navigate 7 (centre, zoom, depth, precision, the lock badge
/// and its anchor); Slice centre 9; Tilt 6 (`τ₁` and its target, `τ₂` and its target, `γ`).
pub const SECTION_ROWS: [usize; 4] = [5, 7, 9, 6];
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

/// The sections' rects in `panel`, top to bottom: Chart, Navigate, Slice centre, Tilt, each as tall as its rows.
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
    tree.register(Some(CHART), Scope::control(BUILDER, "Chart builder…"));
    tree.register(Some(NAVIGATE), Scope::value(CENTRE_U, "centre u", Pan));
    tree.register(Some(NAVIGATE), Scope::value(CENTRE_V, "centre v", Pan));
    tree.register(Some(NAVIGATE), Scope::value(ZOOM, "zoom", ZoomLog2));
    tree.register(Some(NAVIGATE), Scope::control(UNLOCK, "unlock"));
    tree.register(
        Some(NAVIGATE),
        Scope::control(OPEN_INSPECTOR, "open in Inspector"),
    );
    for (id, name) in SLIDERS.into_iter().zip(AXES) {
        tree.register(Some(SLICE_CENTRE), Scope::value(id, name, Bounded));
    }
    tree.register(Some(TILT), Scope::value(TAU1, "τ₁", Angle));
    tree.register(Some(TILT), Scope::menu(TILT_X, "tilt τ₁ into"));
    tree.register(Some(TILT), Scope::value(TAU2, "τ₂", Angle));
    tree.register(Some(TILT), Scope::menu(TILT_Y, "tilt τ₂ into"));
    tree.register(Some(TILT), Scope::value(GAMMA, "γ", Angle));
    for (key, action) in SHORTCUTS {
        tree.register_shortcut(key, action);
    }
}

/// Keeps the target dropdowns' entries those [`target_choices`] offers on `view`'s plane, in order, each named by its
/// axis (R-457 E5): re-registered only when the plane's axes change, so a menu the focus is in keeps its entries.
pub fn join_targets(tree: &mut ScopeTree, view: &View) {
    let choices = target_choices(view.a, view.b);
    for (menu, entries) in [(TILT_X, TILT_X_ENTRIES), (TILT_Y, TILT_Y_ENTRIES)] {
        let want: Vec<ScopeId> = choices.iter().map(|&i| entries[i]).collect();
        if tree.children(Some(menu)) != want {
            tree.remove_children(menu);
            for &i in &choices {
                tree.register(Some(menu), Scope::control(entries[i], AXES[i]));
            }
        }
    }
}

/// The rows the Manifold view, the figure and the compass add to the `?` overlay (render_gui_spec §G3's "In scope",
/// read with R-409: the arrows need Enter; the letter shortcuts act in either mode).
pub const SHORTCUTS: [(&str, &str); 9] = [
    (
        "Enter on a value",
        "interact: the arrows adjust it; Enter or Esc, back to navigating",
    ),
    (
        "Enter on a tilt target",
        "open it: ↑ ↓ choose, Enter picks, Esc closes it unchanged",
    ),
    ("Figure: arrows", "pan, after Enter"),
    ("Figure: + / −", "zoom, in either mode"),
    ("K", "lock at the point under the pointer, in either mode"),
    (
        "Compass: arrows",
        "after Enter: tilt in Tilt, move the slice along the normal in Slice; Shift ×10, Alt ×0.1",
    ),
    ("Compass: Tilt / Slice", "Enter on one sets the mode"),
    (
        "Compass: Cube / Plücker / Jacobi / Shape",
        "Enter on one switches the view",
    ),
    (
        "Compass: Plücker, after Enter",
        "arrows move the cell cursor, Shift to the end; Space steers to it",
    ),
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

/// The slice axis of the preset `(a, b)`: [`SLICE_AXIS`], or the first axis neither shows. The slice frame's defaults
/// for a plane along `(a, b)`, and where the normal goes when the plane takes it (R-459 G4).
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

/// The axes `(a, b)` of a basis along two axes alone, `q₁` with no component but on `e_a` and `q₂` none but on `e_b`,
/// `a < b`, as a steer on the compass's Plücker grid sets it (R-452 C5); `None` for any other basis.
pub fn axis_pair(q1: &Latent, q2: &Latent) -> Option<(usize, usize)> {
    let only = |q: &Latent| {
        let mut on = (0..LATENT_DIM).filter(|&i| q[i] != 0.0);
        match (on.next(), on.next()) {
            (Some(i), None) => Some(i),
            _ => None,
        }
    };
    let (a, b) = (only(q1)?, only(q2)?);
    (a < b).then_some((a, b))
}

/// The chart, as the panel reads it in a preset's frame and the slice frame's hidden dimensions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// The preset whose frame holds the basis, by its index in [`PRESETS`]; `None` for a basis edited out of every
    /// preset's frame, read in the frame of its dominant axes.
    pub preset: Option<usize>,
    /// The axis `q₁` is named by.
    pub a: usize,
    /// The axis `q₂` is named by.
    pub b: usize,
    /// The slice normal, the dimension the slice moves through: the compass cube's depth axis.
    pub d: usize,
    /// The target `τ₁` tilts `q₁` into, `e_X` (R-457).
    pub x: usize,
    /// The target `τ₂` tilts `q₂` into, `e_Y`.
    pub y: usize,
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
    /// The view of `plane`'s basis with the slice frame's defaults, each hidden dimension the slice axis of the axes
    /// it is read on ([`slice_axis`]): the frame a plane had before R-457, and has until a target is picked.
    pub fn of(plane: &Plane) -> Self {
        Self::framed(plane, None)
    }

    /// The view of `sim`'s chart: its basis read with its slice frame (gui_state_contract §2).
    pub fn read(sim: &SimConfig) -> Self {
        Self::framed(&sim.plane, Some(&sim.slice.frame))
    }

    /// The view of `plane`'s basis with the slice frame `frame`, or its defaults: the first preset whose frame holds
    /// it, or its dominant axes' frame. A basis along two axes alone, `q₁` along `e_a` and `q₂` along `e_b`, `a < b`,
    /// reads as that pair untilted, the plane a steer on the compass's Plücker grid sets (R-452 C5); a preset's pair
    /// reads as before. On the axes `(a, b)` each of the frame's three is read as the frame holds it, or as the slice
    /// axis where it is one of `a`, `b`, which no frame holds for its own plane (R-459 G4), or no latent dimension; a
    /// preset is found only where the frame holds none of its axes.
    pub fn framed(plane: &Plane, frame: Option<&SliceFrame>) -> Self {
        let (q1, q2) = (&plane.q1, &plane.q2);
        let total = dot(q1, q1) + dot(q2, q2);
        let hidden = |a: usize, b: usize| -> [usize; 3] {
            let axis = slice_axis(a, b);
            let on = |i: Option<u32>| {
                i.and_then(|i| usize::try_from(i).ok())
                    .filter(|&i| i < LATENT_DIM && i != a && i != b)
                    .unwrap_or(axis)
            };
            [
                on(frame.map(|f| f.normal)),
                on(frame.map(|f| f.tilt_x)),
                on(frame.map(|f| f.tilt_y)),
            ]
        };
        let outside = |a: usize, b: usize| -> f64 {
            let [_, x, y] = hidden(a, b);
            (0..LATENT_DIM)
                .filter(|i| ![a, b, x, y].contains(i))
                .map(|i| q1[i] * q1[i] + q2[i] * q2[i])
                .sum()
        };
        if let Some((a, b)) = axis_pair(q1, q2) {
            let preset = PRESETS.iter().position(|&p| p == (a, b));
            return Self::in_frame(preset, a, b, hidden(a, b), q1, q2);
        }
        // A frame holding one of a preset's axes is not that preset's (R-459 G4), so a plane tilted within a 3-space
        // two presets share is read in the preset the frame fits: `z_q3 × z_μ1` into `z_α` is not `z_α × z_μ1` into
        // `z_q3` while the normal is `z_α`.
        let fits = |a: usize, b: usize| {
            frame.is_none_or(|f| {
                [f.normal, f.tilt_x, f.tilt_y]
                    .into_iter()
                    .all(|i| usize::try_from(i).is_ok_and(|i| i != a && i != b))
            })
        };
        let found = PRESETS.iter().position(|&(a, b)| {
            fits(a, b) && outside(a, b) <= 1e-18 * total.max(f64::MIN_POSITIVE)
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
        Self::in_frame(found, a, b, hidden(a, b), q1, q2)
    }

    /// The scale, tilts and rotation of `q1`, `q2` on the axes `(a, b)`, the normal `d` and the targets `x`, `y`.
    fn in_frame(
        preset: Option<usize>,
        a: usize,
        b: usize,
        [d, x, y]: [usize; 3],
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
        let sin1 = cos_g * q1[x] - sin_g * q2[x];
        let sin2 = sin_g * q1[y] + cos_g * q2[y];
        // `+ 0.0` reads a −0 angle as 0, as the readouts show it.
        Self {
            preset,
            a,
            b,
            d,
            x,
            y,
            scale: cos1.hypot(sin1),
            tau1: sin1.atan2(cos1) + 0.0,
            tau2: sin2.atan2(cos2) + 0.0,
            gamma: gamma + 0.0,
        }
    }

    /// The basis `(q₁, q₂)` of this view: `u₁ = cos τ₁ e_a + sin τ₁ e_X`, `u₂ = cos τ₂ e_b + sin τ₂ e_Y`, turned by `γ`
    /// and scaled by `σ` (R-457 E1).
    pub fn basis(&self) -> (Latent, Latent) {
        let (s1, c1) = self.tau1.sin_cos();
        let (s2, c2) = self.tau2.sin_cos();
        let (sg, cg) = self.gamma.sin_cos();
        let u1 = add(&axis(self.a, c1), s1, &axis(self.x, 1.0));
        let u2 = add(&axis(self.b, c2), s2, &axis(self.y, 1.0));
        let q1 = add(&u1.map(|x| x * cg * self.scale), sg * self.scale, &u2);
        let q2 = add(&u2.map(|x| x * cg * self.scale), -sg * self.scale, &u1);
        (q1, q2)
    }

    /// The slice frame this view is read with.
    pub fn frame(&self) -> SliceFrame {
        let index = |i: usize| u32::try_from(i).expect("a latent index");
        SliceFrame {
            normal: index(self.d),
            tilt_x: index(self.x),
            tilt_y: index(self.y),
        }
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

/// The guards' cut on [`plane_s`] (R-451 B11, REQ-GUI-203): below it [`in_plane`] and [`chart_coords`] read the basis
/// as spanning no plane. Numerical, not an R-71 calibration: the computed `s²` is off by up to about `17ε`, so a basis
/// with no plane can read `s` up to `√(17ε) ≈ 6.1·10⁻⁸`, and 10⁻⁶ is the first power of ten at least ten times that
/// floor. It is not the compass's 10⁻³ drawing cut: `s` is small also where the vectors' norms differ (`s ≈ 2r` for
/// orthogonal vectors of norm ratio `r`), and a plane and its pivot exist there.
pub const PLANE_S_CUT: f64 = 1e-6;

/// R-451 B11's scale-free test of the basis `(q₁, q₂)`: `s = 2√(g₁₁g₂₂ − g₁₂²) / (g₁₁ + g₂₂)`, `g` their Gram matrix,
/// `2|q₁ × q₂| / (|q₁|² + |q₂|²)` in the cube's frame. It is 0 only where `q₁`, `q₂` span no plane, is unchanged by
/// scaling both together, and `s ≤ sin θ`, `θ` the angle between them. A negative `g₁₁g₂₂ − g₁₂²`, rounding, reads as 0,
/// and so does the zero basis.
pub fn plane_s(q1: &Latent, q2: &Latent) -> f64 {
    let (g11, g12, g22) = (dot(q1, q1), dot(q1, q2), dot(q2, q2));
    let total = g11 + g22;
    if total == 0.0 {
        return 0.0;
    }
    2.0 * (g11 * g22 - g12 * g12).max(0.0).sqrt() / total
}

/// Whether the guards read `plane`'s basis as spanning no plane by [`plane_s`]: `s < `[`PLANE_S_CUT`]. Each guard keeps
/// its own sine test beside it.
pub(crate) fn spans_no_plane(plane: &Plane) -> bool {
    plane_s(&plane.q1, &plane.q2) < PLANE_S_CUT
}

/// The in-plane coordinates of `z` along `q̂₁` and `q̂₂`, the unit basis directions: the `(u, v)` with `z`'s projection
/// on the plane's span `u q̂₁ + v q̂₂`. `None` for a degenerate basis: where the sine test `1 − (q̂₁·q̂₂)² ≤ 10⁻¹²` fires
/// or `s < `[`PLANE_S_CUT`] (R-451 B11, REQ-GUI-203).
pub fn in_plane(plane: &Plane, z: &Latent) -> Option<(f64, f64)> {
    // A zero basis vector reads `s = 0`, so the cut also keeps the divisions below off a zero norm.
    if spans_no_plane(plane) {
        return None;
    }
    let (n1, n2) = (norm(&plane.q1), norm(&plane.q2));
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
/// dropped. `None` for a degenerate basis: where the sine test `g₁₁g₂₂ − g₁₂² ≤ 10⁻²⁴ g₁₁g₂₂` fires or
/// `s < `[`PLANE_S_CUT`] (R-451 B11, REQ-GUI-203).
pub fn chart_coords(plane: &Plane, z: &Latent) -> Option<(f64, f64)> {
    let r = add(z, -1.0, &plane.z0);
    let (g11, g12, g22) = (
        dot(&plane.q1, &plane.q1),
        dot(&plane.q1, &plane.q2),
        dot(&plane.q2, &plane.q2),
    );
    let det = g11 * g22 - g12 * g12;
    if det <= 1e-24 * (g11 * g22).max(f64::MIN_POSITIVE) || spans_no_plane(plane) {
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

/// A `SetField` on the slice frame, with the basis where the same pick changes it: one history entry (R-457 E3).
pub fn frame_edit(frame: SliceFrame, basis: Option<(Latent, Latent)>) -> SetField {
    sim(SimField::SliceFrame { frame, basis })
}

/// The dimensions a target dropdown offers on the plane `(a, b)`: the six not `e_a` or `e_b`, the slice normal among
/// them, in [`AXES`]'s order (R-457 E5).
pub fn target_choices(a: usize, b: usize) -> Vec<usize> {
    (0..LATENT_DIM).filter(|&i| i != a && i != b).collect()
}

/// Which tilt angle's target: `τ₁`'s, `X`, or `τ₂`'s, `Y`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// "tilt τ₁ into \[X\]".
    X,
    /// "tilt τ₂ into \[Y\]".
    Y,
}

/// The pick of `to` as `which`'s target on `view` (R-457): one `SetField` on the slice frame, carrying the basis
/// re-tilted into `e_to` where that target's angle is not 0, and the slice frame alone where it is. `None` for the
/// target it already is, or a dimension the dropdown does not offer.
pub fn target_edit(view: &View, which: Target, to: usize) -> Option<SetField> {
    if !target_choices(view.a, view.b).contains(&to) {
        return None;
    }
    let (now, angle, picked) = match which {
        Target::X => (view.x, view.tau1, View { x: to, ..*view }),
        Target::Y => (view.y, view.tau2, View { y: to, ..*view }),
    };
    if to == now {
        return None;
    }
    let basis = (angle != 0.0).then(|| picked.basis());
    Some(frame_edit(picked.frame(), basis))
}

/// The slice frame `frame` once the plane's axes are `(a, b)` (R-459 G4, R-457 E4): a normal the plane takes moves by
/// the slice-axis rule ([`slice_axis`]); a target equal to the old normal follows it, and one the plane takes resets to
/// the normal, moved first; the rest keep their own.
pub fn frame_on(frame: SliceFrame, a: usize, b: usize) -> SliceFrame {
    let taken = |i: u32| usize::try_from(i).map_or(true, |i| i >= LATENT_DIM || i == a || i == b);
    let normal = if taken(frame.normal) {
        u32::try_from(slice_axis(a, b)).expect("a latent index")
    } else {
        frame.normal
    };
    let target = |t: u32| {
        if t == frame.normal || taken(t) {
            normal
        } else {
            t
        }
    };
    SliceFrame {
        normal,
        tilt_x: target(frame.tilt_x),
        tilt_y: target(frame.tilt_y),
    }
}

/// The edit moving `sim`'s basis to `basis`, a preset or Chart's basis editor: a `SetField` on the basis where the
/// slice frame still fits the new plane, else one on the slice frame [`frame_on`] the new plane's axes, carrying the
/// basis, so the normal leaves the plane in the same history entry (R-459 G4, REQ-GUI-205).
pub fn plane_edit(sim: &SimConfig, basis: (Latent, Latent)) -> SetField {
    let plane = Plane {
        z0: sim.plane.z0,
        q1: basis.0,
        q2: basis.1,
    };
    let view = View::framed(&plane, Some(&sim.slice.frame));
    let frame = frame_on(sim.slice.frame, view.a, view.b);
    if frame == sim.slice.frame {
        basis_edit(basis)
    } else {
        frame_edit(frame, Some(basis))
    }
}

/// The edits `edits`, made against `sim`, with each basis edit turned about the pin while locked (render_gui_spec §G4:
/// tilt, rotation, zoom and chart changes "all turn about the pin"): the anchor's chart point `(s, t)` is kept, so the
/// basis edit `(q₁, q₂) → (q₁', q₂')` is preceded by the `SetField` on `z₀` that moves the centre to
/// `z₀' = z₀ + (2s−1)(q₁ − q₁') + (2t−1)(q₂ − q₂')` (chart_decoder_contract Part 4: the lock is the pivot). Free, or with
/// the anchor at the centre, or with a degenerate basis, the edits pass as they are. The edits are read in order, each
/// against the state the earlier ones leave.
pub fn about_the_pin(
    edits: Vec<SetField>,
    sim: &engine::contract::sim_config::SimConfig,
) -> Vec<SetField> {
    let (mut plane, mut lock) = (sim.plane.clone(), sim.lock.clone());
    let mut out = Vec::with_capacity(edits.len());
    for edit in edits {
        match &edit.edit {
            Edit::Sim(SimField::Z0(z0)) => plane.z0 = *z0,
            Edit::Sim(SimField::Lock(l)) => lock = l.clone(),
            Edit::Sim(
                SimField::Basis { q1, q2 }
                | SimField::SliceFrame {
                    basis: Some((q1, q2)),
                    ..
                },
            ) => {
                let pivot = lock
                    .locked
                    .then(|| chart_coords(&plane, &lock.z_locked))
                    .flatten();
                if let Some((s, t)) = pivot {
                    let z0 = add(
                        &add(&plane.z0, 2.0 * s - 1.0, &add(&plane.q1, -1.0, q1)),
                        2.0 * t - 1.0,
                        &add(&plane.q2, -1.0, q2),
                    );
                    if z0 != plane.z0 {
                        out.push(z0_edit(z0));
                        plane.z0 = z0;
                    }
                }
                (plane.q1, plane.q2) = (*q1, *q2);
            }
            // The horizon, the window, the slice frame alone and the render fields leave the plane and the lock as they
            // are.
            Edit::Sim(
                SimField::Horizon(_)
                | SimField::Window(_)
                | SimField::SliceFrame { basis: None, .. },
            )
            | Edit::Render(_) => {}
        }
        out.push(edit);
    }
    out
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

/// The compass's mode (render_gui_spec §G2, R-409): set by its Tilt and Slice buttons, and by the slider touched last.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Touch {
    /// Tilt, the default: the Tilt button, or a tilt touched last: `τ₁`, `τ₂`, `γ`, or a tilt on the compass.
    #[default]
    Tilt,
    /// Slice: the Slice button, or a slice slider touched last: a Slice centre slider, or a slice on the compass.
    Slice,
}

/// The GUI's own scratch state for the panel and the compass: never the engine's, never undoable (R-69).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scratch {
    /// The compass's mode and its drag.
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

/// Enter on control `id`, as its click: the panel's, a target dropdown's entry picking that target among them, and the
/// compass's Tilt and Slice.
pub fn activate(id: &str, snapshot: &Snapshot, scratch: &mut Scratch, out: &mut Out) {
    let view = View::read(&snapshot.sim);
    for (which, entries) in [(Target::X, TILT_X_ENTRIES), (Target::Y, TILT_Y_ENTRIES)] {
        if let Some(to) = entries.iter().position(|e| *e == id) {
            if let Some(edit) = target_edit(&view, which, to) {
                out.edit(edit, None, scratch);
            }
            return;
        }
    }
    match id {
        PRESET => {
            let next = view.preset.map_or(0, |p| (p + 1) % PRESETS.len());
            out.edit(chart::preset_edit(&snapshot.sim, view, next), None, scratch);
        }
        UNLOCK => lock::unlock(&snapshot.sim.lock, out),
        OPEN_INSPECTOR => lock::open_inspector(&snapshot.sim.lock, out),
        BUILDER => out.windows.push(Window::ChartBuilder),
        _ => compass::activate(id, scratch),
    }
}

/// An adjustment of the arrows on one of the panel's values, the figure or the compass, in interaction mode.
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
    if let Some(i) = SLIDERS.iter().position(|s| *s == id) {
        let mut z0 = plane.z0;
        z0[i] += range * span;
        out.edit(z0_edit(z0), Some(Touch::Slice), scratch);
        return;
    }
    for (components, which) in [(Q1_COMPONENTS, 0), (Q2_COMPONENTS, 1)] {
        if let Some(i) = components.iter().position(|s| *s == id) {
            let (mut q1, mut q2) = (plane.q1, plane.q2);
            [&mut q1, &mut q2][which][i] += range * 2.0 * Q_HALF_RANGE;
            out.edit(plane_edit(&snapshot.sim, (q1, q2)), None, scratch);
            return;
        }
    }
    let view = View::read(&snapshot.sim);
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
        TAU1 | TAU2 | GAMMA => {
            let mut angles = tilt::Angles::of(&view);
            angles.turn(id, degrees);
            out.edit(
                basis_edit(angles.apply(view).basis()),
                Some(Touch::Tilt),
                scratch,
            );
        }
        crate::explore::FIGURE => navigate::pan_by_keys(adjust, plane, view_fraction, scratch, out),
        crate::explore::COMPASS_VIEW => compass::adjust(adjust, &snapshot.sim, scratch, out),
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
pub const SECTION_TITLES: [&str; 4] = ["Chart", "Navigate", "Slice centre", "Tilt"];

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
    tilt::show(ui, slice_rect, h, clip, panel);
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
