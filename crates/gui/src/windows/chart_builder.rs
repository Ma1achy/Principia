//! The Chart builder (render_gui_spec §G7; GUI_DESIGN_NOTES § "03 Chart builder"; 03_chartbuilder.png; R-26, R-407,
//! R-408): the presets, saved pairs of axes, editable (Save, Duplicate, Delete); each axis's kind, a latent direction
//! (any mix of the eight `z` components, normalised, with `|q|` shown), a physical quantity or a Burrau dimension; the
//! domain preview, the chart's admissible region in its own coordinates, the forbidden region hatched, the view as a
//! rectangle, the boundary's formula and "forbidden in view: N%"; and a quick render. Both previews are square, as
//! the slice is (§G1: every preview keeps the viewport's aspect). The footer checks `q₁ · q₂ = 0` and lists the hidden
//! directions; Revert and Apply.
//!
//! On the mock (applied per R-369): the window edits a draft of the basis, seeded from the chart when it opens, each
//! axis a unit direction; Apply sends one `SetField` on the basis, each axis scaled back to the length it had, so the
//! zoom is kept, turning about the pin while locked (§G4); Revert seeds the draft again. A physical quantity or a
//! Burrau dimension is a frame: the mock applies latent directions only. The domain function is the mock's own
//! placeholder, and the quick render draws the chart through the engine side's canvas, square. The axis sliders span
//! `[−1, 1]`, a range, so the arrows step them by `Bounded`. The chart builder's requirements are TASK-M8-18's.

use std::sync::Arc;

use eframe::egui::{
    self, pos2, vec2, Button, Color32, Rect, RichText, Sense, Shape, Slider, Stroke, Ui,
};
use eframe::egui_wgpu;
use engine::contract::canvas::Canvas;
use engine::contract::set_field::SetField;
use engine::contract::sim_config::{Latent, Plane, LATENT_DIM};

use crate::explore::manifold_view::{axis, basis_edit, dot, norm, AXES, PRESETS};
use crate::keyboard::scopes::{Adjust, Base, Scope, ScopeId, ScopeTree, StepKind};
use crate::windows::{note, Keys};

/// The window's scope.
pub const WINDOW: ScopeId = "win_chart_builder";
/// Its title, as 03_chartbuilder.png's.
pub const TITLE: &str = "Chart builder";

/// The presets' section.
pub const PRESETS_SECTION: ScopeId = "cb_presets";
/// The horizontal axis's section, `q₁`.
pub const HORIZONTAL: ScopeId = "cb_h";
/// The vertical axis's section, `q₂`.
pub const VERTICAL: ScopeId = "cb_v";
/// Revert and Apply.
pub const APPLY_SECTION: ScopeId = "cb_apply_section";
/// Save.
pub const SAVE: ScopeId = "cb_save";
/// Duplicate.
pub const DUPLICATE: ScopeId = "cb_duplicate";
/// Delete.
pub const DELETE: ScopeId = "cb_delete";
/// Revert.
pub const REVERT: ScopeId = "cb_revert";
/// Apply.
pub const APPLY: ScopeId = "cb_apply";

/// The most presets the list holds (a look choice, R-390).
pub const MAX_PRESETS: usize = 8;
/// The presets' scopes, one per slot.
pub const PRESET_IDS: [ScopeId; MAX_PRESETS] = [
    "cb_preset_0",
    "cb_preset_1",
    "cb_preset_2",
    "cb_preset_3",
    "cb_preset_4",
    "cb_preset_5",
    "cb_preset_6",
    "cb_preset_7",
];
/// The presets' labels in the breadcrumb, by slot (the scope tree's labels are static; the names are the draft's).
pub const PRESET_LABELS: [&str; MAX_PRESETS] = [
    "preset 1", "preset 2", "preset 3", "preset 4", "preset 5", "preset 6", "preset 7", "preset 8",
];

/// Each axis's scopes: its section, its three kinds, its eight components and normalise.
pub struct AxisIds {
    /// The section.
    pub section: ScopeId,
    /// Its heading.
    pub heading: &'static str,
    /// The kinds, in [`KINDS`]' order.
    pub kinds: [ScopeId; 3],
    /// The components, in [`AXES`]' order.
    pub components: [ScopeId; LATENT_DIM],
    /// Normalise.
    pub normalise: ScopeId,
}

/// The two axes' scopes.
pub const AXIS_IDS: [AxisIds; 2] = [
    AxisIds {
        section: HORIZONTAL,
        heading: "Horizontal · q₁",
        kinds: ["cb_h_latent", "cb_h_physical", "cb_h_burrau"],
        components: [
            "cb_h_0", "cb_h_1", "cb_h_2", "cb_h_3", "cb_h_4", "cb_h_5", "cb_h_6", "cb_h_7",
        ],
        normalise: "cb_h_normalise",
    },
    AxisIds {
        section: VERTICAL,
        heading: "Vertical · q₂",
        kinds: ["cb_v_latent", "cb_v_physical", "cb_v_burrau"],
        components: [
            "cb_v_0", "cb_v_1", "cb_v_2", "cb_v_3", "cb_v_4", "cb_v_5", "cb_v_6", "cb_v_7",
        ],
        normalise: "cb_v_normalise",
    },
];

/// An axis's kind (§G7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    /// A latent direction: any mix of the eight components, normalised.
    #[default]
    Latent,
    /// A physical quantity: the chart is nonlinear (Φ).
    Physical,
    /// A Burrau dimension.
    Burrau,
}

/// The kinds, in order, with their labels.
pub const KINDS: [(Kind, &str); 3] = [
    (Kind::Latent, "latent direction"),
    (Kind::Physical, "physical quantity"),
    (Kind::Burrau, "Burrau"),
];

/// A component slider's range: `[−1, 1]`.
pub const COMPONENT_RANGE: (f64, f64) = (-1.0, 1.0);
/// The quick render's size and horizon, its caption (§G7's example, "64 × 64 at a short horizon").
pub const QUICK: &str = "quick render 64 × 64 · t 20 · square, as the view";
/// The mock domain's boundary, as its formula reads.
pub const BOUNDARY: &str = "|t − ½| ≤ 0.35 + 0.15 sin(π s)";
/// The orthogonality check's tolerance, on the unit axes' dot product (a look choice, R-390).
pub const ORTHOGONAL: f64 = 1e-6;
/// The side of the grid the forbidden share is counted on (a look choice, R-390).
pub const SHARE_GRID: usize = 64;

/// A preset: its name and its two unit axes.
#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    /// Its name.
    pub name: String,
    /// `q₁`'s direction.
    pub q1: Latent,
    /// `q₂`'s direction.
    pub q2: Latent,
}

/// The built-in presets: the Manifold view's, named by their axes.
pub fn builtin() -> Vec<Preset> {
    PRESETS
        .iter()
        .map(|&(a, b)| Preset {
            name: format!("{} × {}", AXES[a], AXES[b]),
            q1: axis(a, 1.0),
            q2: axis(b, 1.0),
        })
        .collect()
}

/// The builder's draft: the GUI's own (applied per R-369).
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    /// The two axes' directions.
    pub q: [Latent; 2],
    /// The two axes' kinds.
    pub kind: [Kind; 2],
    /// The presets.
    pub presets: Vec<Preset>,
    /// The preset chosen, by index.
    pub preset: Option<usize>,
    /// The two previews' rects as last drawn: the domain preview's and the quick render's.
    pub previews: Option<[Rect; 2]>,
    /// A note from the last Apply, if it applied nothing.
    pub note: Option<&'static str>,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            q: [axis(0, 1.0), axis(1, 1.0)],
            kind: [Kind::Latent; 2],
            presets: builtin(),
            preset: Some(0),
            previews: None,
            note: None,
        }
    }
}

/// `v` normalised; `v` itself when it is zero.
pub fn unit(v: &Latent) -> Latent {
    let n = norm(v);
    if n > 0.0 {
        v.map(|x| x / n)
    } else {
        *v
    }
}

impl Draft {
    /// Seeds the draft from `plane`: each axis its direction, latent.
    pub fn seed(&mut self, plane: &Plane) {
        self.q = [unit(&plane.q1), unit(&plane.q2)];
        self.kind = [Kind::Latent; 2];
        self.preset = self
            .presets
            .iter()
            .position(|p| p.q1 == self.q[0] && p.q2 == self.q[1]);
        self.note = None;
    }

    /// The basis edit Apply sends against `plane`: each axis's direction at the length its axis has, so the zoom is
    /// kept; `None` while an axis is not latent or is zero.
    pub fn applied(&self, plane: &Plane) -> Option<SetField> {
        if self.kind != [Kind::Latent; 2] || self.q.iter().any(|q| norm(q) == 0.0) {
            return None;
        }
        let scaled = |q: &Latent, len: f64| unit(q).map(|x| x * len);
        Some(basis_edit((
            scaled(&self.q[0], norm(&plane.q1)),
            scaled(&self.q[1], norm(&plane.q2)),
        )))
    }
}

/// `q₁ · q₂` of the unit axes.
pub fn cross_dot(q: &[Latent; 2]) -> f64 {
    dot(&unit(&q[0]), &unit(&q[1]))
}

/// The check against `q₁ · q₂ = d`: "✔" while the axes are orthogonal, `|d|` within [`ORTHOGONAL`], else "✖" (the
/// heavy marks, which the monospace font draws; its fonts have no light ✓ or ✗).
pub fn mark(d: f64) -> &'static str {
    if d.abs() <= ORTHOGONAL {
        "✔"
    } else {
        "✖"
    }
}

/// The hidden directions: the axes neither `q₁` nor `q₂` touches, by name.
pub fn hidden(q: &[Latent; 2]) -> Vec<&'static str> {
    (0..LATENT_DIM)
        .filter(|&i| q[0][i] == 0.0 && q[1][i] == 0.0)
        .map(|i| AXES[i])
        .collect()
}

/// The mock domain: whether the chart point `(s, t)` is admissible, [`BOUNDARY`] (placeholder content: each chart
/// supplies its own, R-26).
pub fn admissible(s: f64, t: f64) -> bool {
    (t - 0.5).abs() <= 0.35 + 0.15 * (std::f64::consts::PI * s).sin()
}

/// "Forbidden in view": the share of the view `[0, 1]²` the domain forbids, in percent, on a [`SHARE_GRID`]² grid of
/// pixel centres.
pub fn forbidden_share() -> f64 {
    let n = SHARE_GRID;
    let forbidden = (0..n * n)
        .filter(|k| {
            let (i, j) = (k % n, k / n);
            !admissible((i as f64 + 0.5) / n as f64, (j as f64 + 0.5) / n as f64)
        })
        .count();
    100.0 * forbidden as f64 / (n * n) as f64
}

/// Joins the window's sections and elements to `tree`, with `n` presets, again each frame (the presets come and go).
pub fn join_presets(tree: &mut ScopeTree, n: usize) {
    tree.remove_children(PRESETS_SECTION);
    for (id, label) in PRESET_IDS
        .iter()
        .zip(PRESET_LABELS)
        .take(n.min(MAX_PRESETS))
    {
        tree.register(Some(PRESETS_SECTION), Scope::control(id, label));
    }
    for (id, label) in [(SAVE, "Save"), (DUPLICATE, "Duplicate"), (DELETE, "Delete")] {
        tree.register(Some(PRESETS_SECTION), Scope::control(id, label));
    }
}

/// Joins the window's sections and elements to `tree`, in order.
pub fn register(tree: &mut ScopeTree) {
    tree.register(Some(WINDOW), Scope::group(PRESETS_SECTION, "Presets"));
    join_presets(tree, builtin().len());
    for a in &AXIS_IDS {
        tree.register(Some(WINDOW), Scope::group(a.section, a.heading));
        for (id, (_, label)) in a.kinds.iter().zip(KINDS) {
            tree.register(Some(a.section), Scope::control(id, label));
        }
        for (id, label) in a.components.iter().zip(AXES) {
            tree.register(Some(a.section), Scope::value(id, label, StepKind::Bounded));
        }
        tree.register(Some(a.section), Scope::control(a.normalise, "normalise"));
    }
    tree.register(Some(WINDOW), Scope::group(APPLY_SECTION, "Revert · Apply"));
    tree.register(Some(APPLY_SECTION), Scope::control(REVERT, "Revert"));
    tree.register(Some(APPLY_SECTION), Scope::control(APPLY, "Apply"));
}

/// Enter on control `id`, as its click, against the chart `plane`; Apply pushes its edit to `edits`.
pub fn activate(id: &str, draft: &mut Draft, plane: &Plane, edits: &mut Vec<SetField>) {
    if let Some(i) = PRESET_IDS.iter().position(|p| *p == id) {
        if let Some(p) = draft.presets.get(i) {
            draft.q = [p.q1, p.q2];
            draft.kind = [Kind::Latent; 2];
            draft.preset = Some(i);
        }
        return;
    }
    for (a, ids) in AXIS_IDS.iter().enumerate() {
        if let Some(k) = ids.kinds.iter().position(|k| *k == id) {
            draft.kind[a] = KINDS[k].0;
            return;
        }
        if id == ids.normalise {
            draft.q[a] = unit(&draft.q[a]);
            return;
        }
    }
    let selected = draft.preset.and_then(|i| draft.presets.get(i)).cloned();
    match id {
        SAVE if draft.presets.len() < MAX_PRESETS => {
            let name = selected.map_or_else(|| "my chart".to_owned(), |p| format!("my {}", p.name));
            draft.presets.push(Preset {
                name,
                q1: draft.q[0],
                q2: draft.q[1],
            });
            draft.preset = Some(draft.presets.len() - 1);
        }
        DUPLICATE if draft.presets.len() < MAX_PRESETS => {
            if let Some(p) = selected {
                draft.presets.push(Preset {
                    name: format!("{} (copy)", p.name),
                    ..p
                });
                draft.preset = Some(draft.presets.len() - 1);
            }
        }
        DELETE if draft.presets.len() > 1 => {
            if let Some(i) = draft.preset {
                draft.presets.remove(i);
                draft.preset = None;
            }
        }
        REVERT => draft.seed(plane),
        APPLY => match draft.applied(plane) {
            Some(edit) => {
                draft.note = None;
                edits.push(edit);
            }
            None => draft.note = Some("the mock applies latent directions only"),
        },
        _ => {}
    }
}

/// The arrows on a component, in interaction mode: `Bounded` steps of `[−1, 1]`, kept in it.
pub fn adjust(adjust: &Adjust, draft: &mut Draft) {
    let Base::OfRange(fraction) = adjust.delta() else {
        return;
    };
    for (a, ids) in AXIS_IDS.iter().enumerate() {
        if let Some(i) = ids.components.iter().position(|c| *c == adjust.scope) {
            let (lo, hi) = COMPONENT_RANGE;
            draft.q[a][i] = (draft.q[a][i] + fraction * (hi - lo)).clamp(lo, hi);
            draft.preset = None;
        }
    }
}

/// The cells across the domain preview's hatch grid (a look choice, R-390).
pub const HATCH_CELLS: usize = 48;
/// The samples along each edge of the boundary (a look choice, R-390).
pub const BOUNDARY_SAMPLES: usize = 64;
/// The forbidden region's hatch colour (a look choice, R-390, 03_chartbuilder.png's).
pub const FORBIDDEN: Color32 = Color32::from_rgb(150, 60, 70);

/// Where chart point `(s, t)` falls on the domain preview `rect`: the chart's `[−½, 1½]²` across it, `t` up.
pub fn on_preview(rect: Rect, s: f64, t: f64) -> egui::Pos2 {
    pos2(
        rect.min.x + rect.width() * ((s + 0.5) / 2.0) as f32,
        rect.max.y - rect.height() * ((t + 0.5) / 2.0) as f32,
    )
}

/// The hatch: the centres `(s, t)` of the forbidden cells of a [`HATCH_CELLS`]² grid over `[−½, 1½]²`, every third
/// diagonal, row by row from the bottom.
pub fn hatched() -> Vec<[f64; 2]> {
    let n = HATCH_CELLS;
    (0..n * n)
        .filter_map(|k| {
            let (i, j) = (k % n, k / n);
            let centre = |c: usize| -0.5 + 2.0 * (c as f64 + 0.5) / n as f64;
            let (s, t) = (centre(i), centre(j));
            (!admissible(s, t) && (i + j) % 3 == 0).then_some([s, t])
        })
        .collect()
}

/// The boundary's edge on `sign`'s side of `t = ½` (+1 the top, −1 the bottom), sampled across `[−½, 1½]`.
pub fn boundary(sign: f64) -> Vec<[f64; 2]> {
    (0..=BOUNDARY_SAMPLES)
        .map(|k| {
            let s = -0.5 + 2.0 * k as f64 / BOUNDARY_SAMPLES as f64;
            [
                s,
                0.5 + sign * (0.35 + 0.15 * (std::f64::consts::PI * s).sin()),
            ]
        })
        .collect()
}

/// The domain preview in the square `rect`: the chart's `[−½, 1½]²`, the forbidden region hatched, the boundary
/// stroked, the view `[0, 1]²` dashed.
pub(crate) fn domain(ui: &Ui, rect: Rect) {
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    painter.rect_filled(rect, 0.0, visuals.extreme_bg_color);
    let half = rect.width() / HATCH_CELLS as f32 / 2.0;
    for [s, t] in hatched() {
        let at = on_preview(rect, s, t);
        painter.line_segment(
            [at + vec2(-half, half), at + vec2(half, -half)],
            Stroke::new(1.0, FORBIDDEN),
        );
    }
    let edge = Stroke::new(1.5, visuals.hyperlink_color);
    for sign in [1.0, -1.0] {
        let points = boundary(sign)
            .iter()
            .map(|&[s, t]| on_preview(rect, s, t))
            .collect();
        painter.add(Shape::line(points, edge));
    }
    let view = Rect::from_two_pos(on_preview(rect, 0.0, 0.0), on_preview(rect, 1.0, 1.0));
    let dash = Stroke::new(1.0, visuals.text_color());
    for (a, b) in [
        (view.left_top(), view.right_top()),
        (view.right_top(), view.right_bottom()),
        (view.right_bottom(), view.left_bottom()),
        (view.left_bottom(), view.left_top()),
    ] {
        painter.extend(Shape::dashed_line(&[a, b], dash, 4.0, 3.0));
    }
}

/// What the builder draws with: the chart, and the canvas and target format for its quick render.
pub struct View<'a> {
    /// The chart as it stands.
    pub plane: &'a Plane,
    /// The canvas the quick render is drawn through, and its target format; `None` draws its frame only.
    pub quick: Option<(Arc<dyn Canvas>, wgpu::TextureFormat)>,
}

/// One axis's column.
fn axis_ui(
    ui: &mut Ui,
    keys: &mut Keys<'_>,
    draft: &mut Draft,
    a: usize,
    clicked: &mut Vec<ScopeId>,
) {
    let ids = &AXIS_IDS[a];
    let response = ui
        .scope(|ui| {
            ui.label(RichText::new(ids.heading).strong());
            ui.horizontal_wrapped(|ui| {
                for (id, (kind, label)) in ids.kinds.iter().zip(KINDS) {
                    let button = ui.add(Button::selectable(draft.kind[a] == kind, label));
                    if button.clicked() {
                        clicked.push(id);
                    }
                    keys.place(id, &button);
                }
            });
            match draft.kind[a] {
                Kind::Latent => note(ui, "a unit vector in the 8D chart — any mix of the latent coordinates"),
                Kind::Physical => {
                    note(ui, "a derived quantity as an axis: energy, L_z, virial ratio, mass ratio…");
                    note(ui, "range — · holding fixed: z₀ along the other 7");
                    note(ui, "a physical axis makes this a nonlinear chart Φ: lock replays Φ and the decoder on the CPU");
                }
                Kind::Burrau => note(ui, "Euclid (m, n) → the primitive triple · neighbourhood: mass × momentum"),
            }
            let latent = draft.kind[a] == Kind::Latent;
            for (i, id) in ids.components.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_sized([40.0, 18.0], egui::Label::new(AXES[i]));
                    let (lo, hi) = COMPONENT_RANGE;
                    let mut v = draft.q[a][i];
                    let slider = ui.add_enabled(latent, Slider::new(&mut v, lo..=hi).fixed_decimals(2));
                    if slider.changed() {
                        draft.q[a][i] = v;
                        draft.preset = None;
                    }
                    keys.place(id, &slider);
                });
            }
            ui.horizontal(|ui| {
                let button = ui.button("normalise");
                if button.clicked() {
                    clicked.push(ids.normalise);
                }
                keys.place(ids.normalise, &button);
                ui.label(RichText::new(format!("|q| = {:.3}", norm(&draft.q[a]))).monospace().weak());
            });
        })
        .response;
    keys.place_rect(ui, ids.section, response.rect);
    ui.add_space(8.0);
}

/// Draws the window's content; its clicks act as Enter does, Apply's edit pushed to `edits`.
pub fn show(
    ui: &mut Ui,
    keys: &mut Keys<'_>,
    draft: &mut Draft,
    view: &View<'_>,
    edits: &mut Vec<SetField>,
) {
    let mut clicked: Vec<ScopeId> = Vec::new();
    let presets = ui
        .scope(|ui| {
            ui.label(RichText::new("Presets").strong());
            note(ui, "each is just a saved pair of axes");
            for (i, p) in draft.presets.iter().enumerate().take(MAX_PRESETS) {
                let entry = ui.add(Button::selectable(draft.preset == Some(i), &p.name));
                if entry.clicked() {
                    clicked.push(PRESET_IDS[i]);
                }
                keys.place(PRESET_IDS[i], &entry);
            }
            ui.horizontal(|ui| {
                for (id, label) in [(SAVE, "Save"), (DUPLICATE, "Duplicate"), (DELETE, "Delete")] {
                    let button = ui.button(label);
                    if button.clicked() {
                        clicked.push(id);
                    }
                    keys.place(id, &button);
                }
            });
        })
        .response;
    keys.place_rect(ui, PRESETS_SECTION, presets.rect);
    ui.add_space(8.0);
    for a in 0..2 {
        axis_ui(ui, keys, draft, a, &mut clicked);
    }
    ui.label(RichText::new("Domain").strong());
    note(ui, "where this chart decodes to real ICs");
    let side = ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0).max(16.0);
    let (row, _) = ui.allocate_exact_size(
        vec2(2.0 * side + ui.spacing().item_spacing.x, side),
        Sense::hover(),
    );
    let left = Rect::from_min_size(row.min, vec2(side, side));
    let right = Rect::from_min_size(pos2(row.max.x - side, row.min.y), vec2(side, side));
    domain(ui, left);
    match &view.quick {
        Some((canvas, format)) => {
            let callback = egui_wgpu::Callback::new_paint_callback(
                right,
                crate::app::FigureCallback {
                    canvas: canvas.clone(),
                    format: *format,
                },
            );
            ui.painter().add(callback);
        }
        None => {
            ui.painter()
                .rect_filled(right, 0.0, ui.visuals().extreme_bg_color);
        }
    }
    draft.previews = Some([left, right]);
    ui.label(RichText::new(format!("boundary  {BOUNDARY}")).monospace());
    ui.label(RichText::new(format!("forbidden in view  {:.0}%", forbidden_share())).monospace());
    note(ui, QUICK);
    ui.separator();
    let d = cross_dot(&draft.q);
    let check = mark(d);
    ui.label(
        RichText::new(format!(
            "q₁ · q₂ = {d:.3} {check} hidden directions: {}",
            hidden(&draft.q).join(" ")
        ))
        .monospace(),
    );
    let apply = ui
        .scope(|ui| {
            ui.horizontal(|ui| {
                for (id, label) in [(REVERT, "Revert"), (APPLY, "Apply")] {
                    let button = ui.button(label);
                    if button.clicked() {
                        clicked.push(id);
                    }
                    keys.place(id, &button);
                }
            });
            if let Some(text) = draft.note {
                note(ui, text);
            }
        })
        .response;
    keys.place_rect(ui, APPLY_SECTION, apply.rect);
    for id in clicked {
        activate(id, draft, view.plane, edits);
    }
}
