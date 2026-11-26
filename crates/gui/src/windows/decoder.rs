//! Decoder (render_gui_spec §G7a): the link functions, chosen and tuned. One row per block (Masses, Configuration α,
//! Configuration β, Momenta), each with a dropdown of only the registry entries onto that block's codomain, the default
//! first; the link's tunable parameters on sliders held to their ranges, or "no parameters"; a live preview; and a
//! reset, dimmed at the default. The footer: Reset all to defaults, the warning that a change re-integrates every pixel,
//! and Apply / Revert.
//!
//! Edits are staged, the GUI's own: dragging a parameter re-integrates nothing. Apply sends the whole `Links` as one
//! `SetField(SimField::Links)`, one undo entry; Revert drops what is staged; closing the window keeps it.
//!
//! The previews evaluate the generated link functions through `engine::contract::links`, at f64; the GUI computes no
//! link itself. [`curve`] and [`simplex`] are the previews' data, which the painters only draw.

use eframe::egui::{
    self, pos2, vec2, Align2, Button, Color32, ComboBox, FontId, Rect, RichText, Sense, Shape,
    Slider, Stroke, Ui,
};
use engine::contract::links::{self, Block};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{LinkChoice, Links, Plane};

use crate::keyboard::scopes::{Adjust, Base, Scope, ScopeId, ScopeTree, StepKind};
use crate::windows::{note, section, Keys};

/// The window's scope.
pub const WINDOW: ScopeId = "win_decoder";
/// Its title.
pub const TITLE: &str = "Decoder";

/// Each row's section, in [`Block::ALL`]'s order.
pub const ROWS: [ScopeId; 4] = [
    "decoder_mass",
    "decoder_alpha",
    "decoder_beta",
    "decoder_momenta",
];
/// Each row's link dropdown: Enter moves to the next compatible entry.
pub const LINK: [ScopeId; 4] = [
    "decoder_mass_link",
    "decoder_alpha_link",
    "decoder_beta_link",
    "decoder_momenta_link",
];
/// Each row's parameter slider; a row whose entries take none has its scope but no value to step.
pub const PARAM: [ScopeId; 4] = [
    "decoder_mass_param",
    "decoder_alpha_param",
    "decoder_beta_param",
    "decoder_momenta_param",
];
/// Each row's "reset to default".
pub const RESET: [ScopeId; 4] = [
    "decoder_mass_reset",
    "decoder_alpha_reset",
    "decoder_beta_reset",
    "decoder_momenta_reset",
];
/// The Momenta preview's tabs `q₁ … q₄`: whose range and centre are drawn.
pub const TABS: [ScopeId; 4] = [
    "decoder_tab_q1",
    "decoder_tab_q2",
    "decoder_tab_q3",
    "decoder_tab_q4",
];
/// The footer.
pub const FOOTER: ScopeId = "decoder_footer";
/// Reset all to defaults.
pub const RESET_ALL: ScopeId = "decoder_reset_all";
/// Apply.
pub const APPLY: ScopeId = "decoder_apply";
/// Revert.
pub const REVERT: ScopeId = "decoder_revert";

/// The footer's warning.
pub const REINTEGRATES: &str = "Changing a link re-integrates every pixel";
/// What Apply does today, shown in the window and in the badge's and the Links row's tooltips: the field's decode takes
/// the default links whatever is applied, until the engine selects links at run time.
pub const NOT_DECODED: &str =
    "Applying records the links; the field is still decoded with the default links.";
/// A row's parameters' text when its link takes none.
pub const NO_PARAMETERS: &str = "no parameters";
/// The dropdown's mark on the default entry.
pub const DEFAULT_MARK: &str = "(default)";
/// A preview's size, in points.
pub const PREVIEW: egui::Vec2 = vec2(160.0, 110.0);
/// The samples across a one-dimensional preview's span.
pub const SAMPLES: usize = 97;
/// The least span a one-dimensional preview draws, `[−4, 4]`.
pub const LEAST_SPAN: (f64, f64) = (-4.0, 4.0);
/// The simplex preview's even grid of latent lines: from `−6` to `6` in steps of 1, each sampled at [`LINE_SAMPLES`].
pub const GRID: (f64, f64, usize) = (-6.0, 6.0, 13);
/// The samples along each grid line of the simplex preview.
pub const LINE_SAMPLES: usize = 49;

/// The window's draft: the staged links, `None` while nothing differs from the applied ones; and the Momenta tab.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Draft {
    /// The staged links, the GUI's own until Apply.
    pub staged: Option<Links>,
    /// The Momenta preview's tab, `0` for `q₁`.
    pub tab: usize,
}

impl Draft {
    /// The links the window shows: the staged ones, or the applied ones `active`.
    pub fn working(&self, active: &Links) -> Links {
        self.staged.clone().unwrap_or_else(|| active.clone())
    }

    /// Stages `links`, nothing where they are `active`'s.
    pub fn stage(&mut self, links: Links, active: &Links) {
        self.staged = (links != *active).then_some(links);
    }

    /// Stages `block`'s choice `choice`.
    pub fn stage_block(&mut self, block: Block, choice: LinkChoice, active: &Links) {
        let mut links = self.working(active);
        *block.of_mut(&mut links) = choice;
        self.stage(links, active);
    }

    /// Drops a staged edit the applied links have caught up with, as an undo or another window's edit can.
    pub fn follow(&mut self, active: &Links) {
        if self.staged.as_ref() == Some(active) {
            self.staged = None;
        }
    }
}

/// The edit Apply sends: the whole staged `Links`, one undoable `SetField`; `None` with nothing staged.
pub fn apply_edit(draft: &Draft) -> Option<SetField> {
    draft.staged.clone().map(|links| SetField {
        edit: Edit::Sim(SimField::Links(links)),
        no_history: false,
    })
}

/// The index of `block` in [`Block::ALL`].
fn index(block: Block) -> usize {
    Block::ALL
        .iter()
        .position(|b| *b == block)
        .expect("a block")
}

/// Whether any of `block`'s entries takes a parameter: its row's slider is a value the arrows step.
pub fn tunable(block: Block) -> bool {
    links::entries(block).iter().any(|e| !e.params.is_empty())
}

/// Joins the window's rows and footer to `tree`, in the order drawn: each row's dropdown, parameter, reset (and the
/// Momenta tabs), then the footer.
pub fn register(tree: &mut ScopeTree) {
    for block in Block::ALL {
        let i = index(block);
        tree.register(Some(WINDOW), Scope::group(ROWS[i], block.title()));
        tree.register(Some(ROWS[i]), Scope::control(LINK[i], "link"));
        if tunable(block) {
            tree.register(
                Some(ROWS[i]),
                Scope::value(PARAM[i], "parameter", StepKind::Bounded),
            );
        }
        tree.register(Some(ROWS[i]), Scope::control(RESET[i], "reset to default"));
        if block == Block::Momenta {
            for (tab, name) in TABS.iter().zip(["q₁", "q₂", "q₃", "q₄"]) {
                tree.register(Some(ROWS[i]), Scope::control(tab, name));
            }
        }
    }
    tree.register(Some(WINDOW), Scope::group(FOOTER, "Apply"));
    tree.register(
        Some(FOOTER),
        Scope::control(RESET_ALL, "Reset all to defaults"),
    );
    tree.register(Some(FOOTER), Scope::control(APPLY, "Apply"));
    tree.register(Some(FOOTER), Scope::control(REVERT, "Revert"));
}

/// `choice` switched to the entry after its own in `block`'s list, wrapping.
fn next_entry(block: Block, choice: &LinkChoice) -> LinkChoice {
    let list = links::entries(block);
    let at = list.iter().position(|e| e.id == choice.id).unwrap_or(0);
    links::switched(choice, list[(at + 1) % list.len()].id)
}

/// Enter on control `id`, as its click, against the applied links `active`; Apply's edit is pushed to `edits`.
pub fn activate(id: &str, draft: &mut Draft, active: &Links, edits: &mut Vec<SetField>) {
    let working = draft.working(active);
    for block in Block::ALL {
        let i = index(block);
        if id == LINK[i] {
            draft.stage_block(block, next_entry(block, block.of(&working)), active);
        } else if id == RESET[i] {
            draft.stage_block(block, block.default_choice(), active);
        }
    }
    if let Some(tab) = TABS.iter().position(|t| *t == id) {
        draft.tab = tab;
    }
    match id {
        RESET_ALL => draft.stage(Links::defaults(), active),
        APPLY => {
            edits.extend(apply_edit(draft));
            draft.staged = None;
        }
        REVERT => draft.staged = None,
        _ => {}
    }
}

/// The arrows on a row's parameter, in interaction mode: `Bounded` steps of its range, kept in it, staged.
pub fn adjust(adjust: &Adjust, draft: &mut Draft, active: &Links) {
    let Some(i) = PARAM.iter().position(|p| *p == adjust.scope) else {
        return;
    };
    let Base::OfRange(fraction) = adjust.delta() else {
        return;
    };
    let block = Block::ALL[i];
    let choice = block.of(&draft.working(active)).clone();
    let Some(p) = links::params(&choice.id).into_iter().next() else {
        return;
    };
    let now = choice.params.get(p.name).copied().unwrap_or(p.default);
    let to = links::with_param(&choice, p.name, now + fraction * (p.hi - p.lo));
    draft.stage_block(block, to, active);
}

// ── The previews' data ─────────────────────────────────────────────────────────────────────────────────────────────

/// A one-dimensional block's preview: latent `z` across, the physical value up.
#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    /// The span of `z` drawn: the visible range widened by half its width each side, never narrower than `[−4, 4]`.
    pub span: (f64, f64),
    /// The link's forward at even samples of the span, `(z, y)`.
    pub points: Vec<[f64; 2]>,
    /// `d(value)/dz`, `exp` of the log-det, at the same samples: where the link spends pixels.
    pub resolution: Vec<[f64; 2]>,
    /// The default link's curve, off the default; `None` at it.
    pub ghost: Option<Vec<[f64; 2]>>,
    /// The codomain's bounds at the active parameters, dashed; `None` on the circle.
    pub bounds: Option<(f64, f64)>,
    /// The window's range of `z` on this block, shaded.
    pub visible: (f64, f64),
    /// The slice centre, `(z, y)`.
    pub centre: [f64; 2],
}

/// The range of latent coordinate `k` the view `plane` shows: `z₀ ± (|q₁| + |q₂|)` on it.
pub fn visible(plane: &Plane, k: usize) -> (f64, f64) {
    let r = plane.q1[k].abs() + plane.q2[k].abs();
    (plane.z0[k] - r, plane.z0[k] + r)
}

/// The span drawn around the visible range `(lo, hi)`: widened by half its width each side, and at least
/// [`LEAST_SPAN`].
pub fn span((lo, hi): (f64, f64)) -> (f64, f64) {
    let w = hi - lo;
    (
        (lo - w / 2.0).min(LEAST_SPAN.0),
        (hi + w / 2.0).max(LEAST_SPAN.1),
    )
}

/// The samples of `f` at [`SAMPLES`] even points of `span`.
fn sampled(span: (f64, f64), f: impl Fn(f64) -> f64) -> Vec<[f64; 2]> {
    (0..SAMPLES)
        .map(|i| {
            let z = span.0 + (span.1 - span.0) * i as f64 / (SAMPLES - 1) as f64;
            [z, f(z)]
        })
        .collect()
}

/// `block`'s preview under `choice`, a one-control link, on latent coordinate `k` of the view `plane`.
pub fn curve(block: Block, choice: &LinkChoice, plane: &Plane, k: usize) -> Curve {
    let fwd = |c: &LinkChoice, z: f64| links::forward(c, z).unwrap_or(f64::NAN);
    let visible = visible(plane, k);
    let span = span(visible);
    let default = block.default_choice();
    let z0 = plane.z0[k];
    Curve {
        span,
        points: sampled(span, |z| fwd(choice, z)),
        resolution: sampled(span, |z| {
            links::log_det(choice, z).map_or(f64::NAN, libm::exp)
        }),
        ghost: (*choice != default).then(|| sampled(span, |z| fwd(&default, z))),
        bounds: links::bounds(block, choice),
        visible,
        centre: [z0, fwd(choice, z0)],
    }
}

/// The simplex preview: the image of an even grid of `z` lines, the view's footprint, the centre and the equal-mass
/// point, each as masses `(m₀, m₁, m₂)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Simplex {
    /// Each grid line's image, `z₁` lines then `z₂` lines.
    pub lines: Vec<Vec<[f64; 3]>>,
    /// The view's footprint's outline in mass space.
    pub footprint: Vec<[f64; 3]>,
    /// The slice centre's masses.
    pub centre: [f64; 3],
    /// The equal-mass point.
    pub equal: [f64; 3],
}

/// The masses' preview under `choice` for the view `plane`.
pub fn simplex(choice: &LinkChoice, plane: &Plane) -> Simplex {
    let m = |z: [f64; 2]| links::masses(choice, z).unwrap_or([f64::NAN; 3]);
    let (lo, hi, n) = GRID;
    let at = |i: usize, of: usize| lo + (hi - lo) * i as f64 / (of - 1) as f64;
    let mut lines = Vec::with_capacity(2 * n);
    for axis in 0..2 {
        for i in 0..n {
            lines.push(
                (0..LINE_SAMPLES)
                    .map(|j| {
                        let (a, b) = (at(i, n), at(j, LINE_SAMPLES));
                        m(if axis == 0 { [a, b] } else { [b, a] })
                    })
                    .collect(),
            );
        }
    }
    let (k1, k2) = (Block::Mass.z(), Block::Mass.z() + 1);
    let corner = |s: f64, t: f64| {
        [
            plane.z0[k1] + s * plane.q1[k1] + t * plane.q2[k1],
            plane.z0[k2] + s * plane.q1[k2] + t * plane.q2[k2],
        ]
    };
    let edge = |a: (f64, f64), b: (f64, f64)| {
        (0..12).map(move |j| {
            let f = j as f64 / 12.0;
            (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
        })
    };
    let footprint = edge((-1.0, -1.0), (1.0, -1.0))
        .chain(edge((1.0, -1.0), (1.0, 1.0)))
        .chain(edge((1.0, 1.0), (-1.0, 1.0)))
        .chain(edge((-1.0, 1.0), (-1.0, -1.0)))
        .map(|(s, t)| m(corner(s, t)))
        .collect();
    Simplex {
        lines,
        footprint,
        centre: m([plane.z0[k1], plane.z0[k2]]),
        equal: [1.0 / 3.0; 3],
    }
}

// ── Drawing ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// The previews' colours: the line system's presets as egui draws them (a look choice).
const CURVE: Color32 = Color32::from_rgb(120, 180, 255);
const FAINT: Color32 = Color32::from_rgba_premultiplied(90, 90, 90, 120);
const SHADE: Color32 = Color32::from_rgba_premultiplied(60, 80, 110, 70);
const BOUND: Color32 = Color32::from_rgb(200, 160, 80);
const GRID_LINE: Color32 = Color32::from_gray(120);
const DOT: Color32 = Color32::from_rgb(240, 240, 240);

/// Draws `curve` in `rect`.
fn paint_curve(ui: &Ui, rect: Rect, curve: &Curve) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, Color32::from_gray(24));
    let ys = curve
        .points
        .iter()
        .chain(curve.ghost.iter().flatten())
        .map(|p| p[1])
        .chain(curve.bounds.into_iter().flat_map(|(a, b)| [a, b]))
        .filter(|v| v.is_finite());
    let (mut y0, mut y1) = ys.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
        (a.min(v), b.max(v))
    });
    if !(y0.is_finite() && y1.is_finite()) || y1 <= y0 {
        let mid = if y0.is_finite() { y0 } else { 0.0 };
        (y0, y1) = (mid - 1.0, mid + 1.0);
    }
    let pad = (y1 - y0) * 0.08;
    let (y0, y1) = (y0 - pad, y1 + pad);
    let (z0, z1) = curve.span;
    let at = |z: f64, y: f64| {
        pos2(
            rect.min.x + ((z - z0) / (z1 - z0)) as f32 * rect.width(),
            rect.max.y - ((y - y0) / (y1 - y0)) as f32 * rect.height(),
        )
    };
    let (v0, v1) = curve.visible;
    painter.rect_filled(
        Rect::from_x_y_ranges(at(v0, y0).x..=at(v1, y0).x, rect.y_range()),
        0.0,
        SHADE,
    );
    let rmax = curve
        .resolution
        .iter()
        .map(|p| p[1])
        .filter(|v| v.is_finite())
        .fold(0.0, f64::max);
    if rmax > 0.0 {
        let line: Vec<_> = curve
            .resolution
            .iter()
            .map(|p| {
                pos2(
                    at(p[0], y0).x,
                    rect.max.y - (p[1] / rmax) as f32 * rect.height() * 0.4,
                )
            })
            .collect();
        painter.add(Shape::line(line, Stroke::new(1.0, FAINT)));
        painter.text(
            rect.right_bottom() - vec2(2.0, 2.0),
            Align2::RIGHT_BOTTOM,
            "resolution",
            FontId::proportional(9.0),
            FAINT,
        );
    }
    if let Some((a, b)) = curve.bounds {
        for y in [a, b] {
            let (l, r) = (at(z0, y), at(z1, y));
            painter.add(Shape::dashed_line(
                &[l, r],
                Stroke::new(1.0, BOUND),
                4.0,
                3.0,
            ));
        }
    }
    if let Some(ghost) = &curve.ghost {
        let line = ghost.iter().map(|p| at(p[0], p[1])).collect();
        painter.add(Shape::line(line, Stroke::new(1.0, FAINT)));
    }
    let line = curve.points.iter().map(|p| at(p[0], p[1])).collect();
    painter.add(Shape::line(line, Stroke::new(1.5, CURVE)));
    painter.circle_filled(at(curve.centre[0], curve.centre[1]), 2.5, DOT);
}

/// Draws `s` in `rect`: the triangle, its corners `m₁`, `m₂`, `m₃`.
fn paint_simplex(ui: &Ui, rect: Rect, s: &Simplex) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, Color32::from_gray(24));
    let inner = rect.shrink2(vec2(18.0, 12.0));
    let corners = [
        pos2(inner.center().x, inner.min.y),
        pos2(inner.min.x, inner.max.y),
        pos2(inner.max.x, inner.max.y),
    ];
    let at = |m: &[f64; 3]| {
        let p = corners[0].to_vec2() * m[0] as f32
            + corners[1].to_vec2() * m[1] as f32
            + corners[2].to_vec2() * m[2] as f32;
        p.to_pos2()
    };
    let fill: Vec<_> = s.footprint.iter().map(at).collect();
    if fill.iter().all(|p| p.is_finite()) {
        painter.add(Shape::convex_polygon(fill.clone(), SHADE, Stroke::NONE));
        painter.add(Shape::closed_line(fill, Stroke::new(1.0, SHADE)));
    }
    for line in &s.lines {
        let pts = line.iter().map(at).collect();
        painter.add(Shape::line(pts, Stroke::new(0.7, GRID_LINE)));
    }
    painter.add(Shape::closed_line(
        corners.to_vec(),
        Stroke::new(1.0, Color32::GRAY),
    ));
    for (c, (name, align, off)) in corners.iter().zip([
        ("m₁", Align2::CENTER_BOTTOM, vec2(0.0, -1.0)),
        ("m₂", Align2::RIGHT_TOP, vec2(-2.0, 1.0)),
        ("m₃", Align2::LEFT_TOP, vec2(2.0, 1.0)),
    ]) {
        painter.text(
            *c + off,
            align,
            name,
            FontId::proportional(10.0),
            Color32::GRAY,
        );
    }
    let e = at(&s.equal);
    painter.line_segment(
        [e - vec2(3.0, 0.0), e + vec2(3.0, 0.0)],
        Stroke::new(1.0, BOUND),
    );
    painter.line_segment(
        [e - vec2(0.0, 3.0), e + vec2(0.0, 3.0)],
        Stroke::new(1.0, BOUND),
    );
    painter.circle_filled(at(&s.centre), 2.5, DOT);
}

/// One row: the dropdown, the parameter, the reset, the preview.
fn row(
    ui: &mut Ui,
    keys: &mut Keys<'_>,
    block: Block,
    draft: &mut Draft,
    active: &Links,
    plane: &Plane,
) {
    let i = index(block);
    let working = draft.working(active);
    let choice = block.of(&working).clone();
    let list = links::entries(block);
    section(ui, keys, ROWS[i], block.title(), |ui, keys| {
        ui.horizontal(|ui| {
            crate::windows::form_label(ui, "link");
            let mut picked = None;
            let combo = ComboBox::from_id_salt(("decoder link", i))
                .selected_text(links::display(&choice.id))
                .width(150.0)
                .show_ui(ui, |ui| {
                    for (n, e) in list.iter().enumerate() {
                        let text = if n == 0 {
                            format!("{} {DEFAULT_MARK}", e.display)
                        } else {
                            e.display.to_owned()
                        };
                        if ui.selectable_label(e.id == choice.id, text).clicked() {
                            picked = Some(e.id);
                        }
                    }
                });
            keys.place(LINK[i], &combo.response);
            if let Some(id) = picked.filter(|id| *id != choice.id) {
                draft.stage_block(block, links::switched(&choice, id), active);
            }
        });
        let params = links::params(&choice.id);
        ui.horizontal(|ui| {
            crate::windows::form_label(ui, "parameter");
            match params.first() {
                Some(p) => {
                    let mut v = choice.params.get(p.name).copied().unwrap_or(p.default);
                    let slider = ui.add(
                        Slider::new(&mut v, p.lo..=p.hi)
                            .text(links::param_display(p.name))
                            .max_decimals(2),
                    );
                    if tunable(block) {
                        keys.place(PARAM[i], &slider);
                    }
                    ui.label(
                        RichText::new(format!("[{}, {}]", p.lo, p.hi))
                            .weak()
                            .small(),
                    );
                    // Staged only: a drag re-integrates nothing until Apply.
                    if slider.changed() {
                        let to = links::with_param(&choice, p.name, v);
                        draft.stage_block(block, to, active);
                    }
                }
                None => {
                    let label = ui.add_enabled(false, egui::Label::new(NO_PARAMETERS));
                    if tunable(block) {
                        keys.place(PARAM[i], &label);
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            let at_default = choice == block.default_choice();
            let reset = ui.add_enabled(!at_default, Button::new("reset to default").small());
            keys.place(RESET[i], &reset);
            if reset.clicked() {
                draft.stage_block(block, block.default_choice(), active);
            }
            if block == Block::Momenta {
                for (t, (id, name)) in TABS.iter().zip(["q₁", "q₂", "q₃", "q₄"]).enumerate()
                {
                    let tab = ui.add(Button::selectable(draft.tab == t, name).small());
                    keys.place(id, &tab);
                    if tab.clicked() {
                        draft.tab = t;
                    }
                }
            }
        });
        let (rect, _) = ui.allocate_exact_size(PREVIEW, Sense::hover());
        let choice = block.of(&draft.working(active)).clone();
        match block {
            Block::Mass => paint_simplex(ui, rect, &simplex(&choice, plane)),
            _ => {
                let k = block.z()
                    + if block == Block::Momenta {
                        draft.tab
                    } else {
                        0
                    };
                paint_curve(ui, rect, &curve(block, &choice, plane, k));
            }
        }
    });
}

/// Draws the window's content against the applied links `active` and the view `plane`; Apply's edit is pushed to
/// `edits`.
pub fn show(
    ui: &mut Ui,
    keys: &mut Keys<'_>,
    draft: &mut Draft,
    (active, plane): (&Links, &Plane),
    edits: &mut Vec<SetField>,
) {
    draft.follow(active);
    note(ui, "the link functions every chart passes through");
    ui.add_space(6.0);
    for block in Block::ALL {
        row(ui, keys, block, draft, active, plane);
    }
    section(ui, keys, FOOTER, "Apply", |ui, keys| {
        let reset = ui.add_enabled(
            draft.working(active) != Links::defaults(),
            Button::new("Reset all to defaults"),
        );
        keys.place(RESET_ALL, &reset);
        if reset.clicked() {
            activate(RESET_ALL, draft, active, edits);
        }
        note(ui, REINTEGRATES);
        ui.label(
            RichText::new(NOT_DECODED)
                .color(ui.visuals().warn_fg_color)
                .small(),
        );
        ui.horizontal(|ui| {
            let staged = draft.staged.is_some();
            for (id, text) in [(APPLY, "Apply"), (REVERT, "Revert")] {
                let button = ui.add_enabled(staged, Button::new(text));
                keys.place(id, &button);
                if button.clicked() {
                    activate(id, draft, active, edits);
                }
            }
        });
    });
}
