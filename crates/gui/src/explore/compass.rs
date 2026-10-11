//! The compass, the nav cube (render_gui_spec §G2, §G3, §G4; GUI_DESIGN_NOTES § "01 Explore", § "08 Lock"; 01_main.png,
//! 08_lock.png, read with R-409): bottom left, under the view controls. The cube is the chart's frame, its axes the
//! preset's two axes and the hidden slice direction; inside it, the slice plane: through the centre, at the slice step
//! along the depth axis, tilted by `τ₁`, `τ₂` and turned by `γ`. It is a fixed orthographic view at its default angle,
//! with no orbit (R-409).
//!
//! It has two modes, Tilt (the default) and Slice, with a button for each beside it, which show the current mode; a
//! click on one switches the mode, and touching a slice slider still switches it to Slice and touching a tilt to Tilt.
//! In tilt mode a drag anywhere on the cube tilts the plane (one `SetField` on the basis); in slice mode it moves the
//! plane along its normal as drawn, the tilt fixed (one `SetField` on `z₀`). Each drag is one edit, made when it ends
//! (R-96), the compass drawing the plane as the drag leaves it meanwhile. Tilting draws the plane before the tilt
//! dashed. Locked, it carries a gold pin at the anchor's point on the plane, the pivot every basis edit turns about
//! ([`super::manifold_view::about_the_pin`]), and a slice-mode drag is an excursion along the normal through the anchor.
//! It reads out the tilt and rotation angles.
//!
//! Its keys (§G3): the compass is one element in navigation mode, beside its Tilt and Slice buttons; Enter on it
//! starts interaction mode, in which the arrows tilt in tilt mode and move the slice along the normal in slice mode,
//! Shift ×10 and Alt ×0.1 as everywhere (R-409 F3).

use eframe::egui::{
    self, pos2, vec2, Color32, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, UiBuilder, Vec2,
};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Lock, Plane, SimConfig};

use crate::explore::lock::GOLD;
use crate::explore::manifold_view::slice_tilt::{Angles, TILT_RANGE};
use crate::explore::manifold_view::{
    about_the_pin, add, axis, basis_edit, chart_coords, z0_edit, Out, Scratch, Touch, View, AXES,
    Z0_HALF_RANGE,
};
use crate::explore::{COMPASS, COMPASS_SLICE, COMPASS_TILT, COMPASS_VIEW};
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::{Adjust, Base, StepKind};
use crate::keyboard::Keyboard;

/// The compass's title.
pub const TITLE: &str = "Compass";
/// The Tilt button's label (a look choice, R-390).
pub const TILT_BUTTON: &str = "Tilt";
/// The Slice button's label (a look choice, R-390).
pub const SLICE_BUTTON: &str = "Slice";
/// The hint under the buttons in tilt mode (a look choice, R-390).
pub const HINT_TILT: &str = "drag to tilt the plane";
/// The hint under the buttons in slice mode (a look choice, R-390).
pub const HINT_SLICE: &str = "drag to move the plane along its normal";
/// The pin's label, as 08_lock.png's.
pub const PINNED: &str = "pinned";
/// The fixed view's angle, yaw and pitch in degrees: the angle the cube opened at before R-409 removed its orbit, as
/// 01_main.png draws the cube (a look choice, R-390; R-409 point 1).
pub const VIEW_ANGLE: (f64, f64) = (-35.0, 25.0);
/// The degrees a point of drag tilts (a look choice, R-390).
pub const DEGREES_PER_POINT: f64 = 0.5;
/// The cube's half-size on the screen, as a fraction of the compass's square (a look choice, R-390).
pub const CUBE_SCALE: f64 = 0.26;
/// The plane's half-size inside the cube's, which is 1 (a look choice, R-390).
pub const PLANE_HALF: f64 = 0.7;
/// The shortest the normal's unit is taken to be on the screen, as a fraction of the cube's half-size, so a slice-mode
/// drag stays finite when the normal points at the viewer (a look choice, R-390).
pub const MIN_NORMAL_ON_SCREEN: f64 = 0.25;
/// The plane's colour: egui's dark theme's selection blue (a look choice, R-390).
pub fn plane_colour(visuals: &egui::Visuals) -> Color32 {
    visuals.selection.stroke.color
}

/// A drag on the compass in progress.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    /// What it does: the mode when it started, tilting or slicing.
    pub mode: Touch,
    /// The pointer's travel since it started, in points.
    pub travel: Vec2,
}

/// The compass's own state: its mode and its drag. The GUI's, never the engine's.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Compass {
    /// Its mode, which its buttons show: Tilt by default.
    pub mode: Touch,
    /// The drag in progress.
    pub drag: Option<Drag>,
}

/// A point of the cube's frame, `(w, h, d)` each in `[−1, 1]`, on the screen in `square`, seen from [`VIEW_ANGLE`].
pub fn project(square: Rect, [w, h, d]: [f64; 3]) -> Pos2 {
    let (sy, cy) = VIEW_ANGLE.0.to_radians().sin_cos();
    let (sp, cp) = VIEW_ANGLE.1.to_radians().sin_cos();
    let x = w * cy + d * sy;
    let depth = -w * sy + d * cy;
    let y = h * cp - depth * sp;
    let r = f64::from(square.width()) * CUBE_SCALE;
    square.center() + vec2((x * r) as f32, (-y * r) as f32)
}

/// A plane's frame in the cube: its centre and its two in-plane directions.
pub type Frame = ([f64; 3], [f64; 3], [f64; 3]);

/// The slice plane's centre and in-plane directions in the cube's frame: the centre at the slice step along the depth
/// axis, the directions `u₁`, `u₂` tilted by `τ₁`, `τ₂` and turned by `γ`.
pub fn plane_frame(plane: &Plane, view: &View, tilted: bool) -> Frame {
    let depth = (plane.z0[view.d] / Z0_HALF_RANGE).clamp(-1.0, 1.0);
    let (t1, t2) = if tilted {
        (view.tau1, view.tau2)
    } else {
        (0.0, 0.0)
    };
    let u1 = [t1.cos(), 0.0, t1.sin()];
    let u2 = [0.0, t2.cos(), t2.sin()];
    let (sg, cg) = view.gamma.sin_cos();
    let r1 = std::array::from_fn(|i| cg * u1[i] + sg * u2[i]);
    let r2 = std::array::from_fn(|i| -sg * u1[i] + cg * u2[i]);
    ([0.0, 0.0, depth], r1, r2)
}

/// The unit normal of the plane `(_, r1, r2)` in the cube's frame, `r1 × r2`: untilted, the depth axis.
pub fn normal((_, r1, r2): Frame) -> [f64; 3] {
    let n = [
        r1[1] * r2[2] - r1[2] * r2[1],
        r1[2] * r2[0] - r1[0] * r2[2],
        r1[0] * r2[1] - r1[1] * r2[0],
    ];
    let len = n.iter().map(|x| x * x).sum::<f64>().sqrt();
    n.map(|x| x / len)
}

/// The unit normal of `plane` as the compass draws it, in the latent space: the cube's frame is the preset's
/// `(e_a, e_b, e_d)` (R-409 A3).
pub fn latent_normal(plane: &Plane) -> Latent {
    let view = View::of(plane);
    let n = normal(plane_frame(plane, &view, true));
    add(
        &add(&axis(view.a, n[0]), n[1], &axis(view.b, 1.0)),
        n[2],
        &axis(view.d, 1.0),
    )
}

/// The edit moving `plane`'s centre `z₀` by `amount` along the drawn plane's normal; the basis, so the tilt and `γ`,
/// unchanged. Locked, `z₀` leaves the anchor along that line: an excursion, as the slice step's is.
pub fn slice(plane: &Plane, amount: f64) -> SetField {
    z0_edit(add(&plane.z0, amount, &latent_normal(plane)))
}

/// The plane's four corners on the screen.
pub fn corners(square: Rect, (c, r1, r2): Frame) -> Vec<Pos2> {
    [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)]
        .iter()
        .map(|(a, b)| {
            let p = std::array::from_fn(|i| c[i] + PLANE_HALF * (a * r1[i] + b * r2[i]));
            project(square, p)
        })
        .collect()
}

/// Whether `p` is inside the convex polygon `poly`.
pub fn inside(poly: &[Pos2], p: Pos2) -> bool {
    let n = poly.len();
    let side = |i: usize| {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        (b - a).x * (p - a).y - (b - a).y * (p - a).x
    };
    let signs: Vec<f32> = (0..n).map(side).collect();
    signs.iter().all(|s| *s >= 0.0) || signs.iter().all(|s| *s <= 0.0)
}

/// The tilt edit turning `τ₁` by `d1` and `τ₂` by `d2` degrees, within their span.
pub fn tilt(plane: &Plane, d1: f64, d2: f64) -> SetField {
    let view = View::of(plane);
    let mut angles = Angles::of(&view);
    angles.tau1 = (angles.tau1 + d1).clamp(TILT_RANGE.0, TILT_RANGE.1);
    angles.tau2 = (angles.tau2 + d2).clamp(TILT_RANGE.0, TILT_RANGE.1);
    basis_edit(angles.apply(view).basis())
}

/// How far along the normal, in `z₀`'s units, a slice-mode drag of `travel` points moves the plane in `square`: the
/// plane follows the pointer along the normal as drawn, the normal's unit taken as at least [`MIN_NORMAL_ON_SCREEN`]
/// of the cube's half-size on the screen.
pub fn slice_by_drag(plane: &Plane, square: Rect, travel: Vec2) -> f64 {
    let n = normal(plane_frame(plane, &View::of(plane), true));
    let along = project(square, n) - project(square, [0.0; 3]);
    let floor = f64::from(square.width()) * CUBE_SCALE * MIN_NORMAL_ON_SCREEN;
    let unit = f64::from(along.length_sq()).max(floor * floor);
    f64::from(travel.dot(along)) / unit * Z0_HALF_RANGE
}

/// The one edit a drag of `drag` on `plane` in `square` makes: a tilt, right raising `τ₁` and down lowering `τ₂`, or a
/// slice along the normal; none for no travel.
pub fn drag_edit(plane: &Plane, drag: &Drag, square: Rect) -> Option<SetField> {
    if drag.travel == Vec2::ZERO {
        return None;
    }
    Some(match drag.mode {
        Touch::Tilt => tilt(
            plane,
            f64::from(drag.travel.x) * DEGREES_PER_POINT,
            -f64::from(drag.travel.y) * DEGREES_PER_POINT,
        ),
        Touch::Slice => slice(plane, slice_by_drag(plane, square, drag.travel)),
    })
}

/// `plane` after `edits`, each on the plane the earlier leave.
pub fn after(plane: &Plane, edits: &[SetField]) -> Plane {
    let mut plane = plane.clone();
    for edit in edits {
        match &edit.edit {
            Edit::Sim(SimField::Z0(z0)) => plane.z0 = *z0,
            Edit::Sim(SimField::Basis { q1, q2 }) => (plane.q1, plane.q2) = (*q1, *q2),
            _ => {}
        }
    }
    plane
}

/// The arrows on the compass in interaction mode, by its mode: in tilt mode ← → tilt `τ₁` and ↑ ↓ `τ₂` by the `Tilt`
/// step; in slice mode ↑ and → move the slice along the normal, ↓ and ← back, by the `SliceNormal` step (REQ-GUI-184).
/// Shift's ×10 and Alt's ×0.1 are in `adjust.times`, as everywhere (R-409 F3).
pub fn adjust(adjust: &Adjust, plane: &Plane, scratch: &mut Scratch, out: &mut Out) {
    match scratch.compass.mode {
        Touch::Tilt => {
            let Base::Degrees(degrees) = StepKind::Tilt.base().times(adjust.times) else {
                unreachable!("the compass's tilt step is in degrees")
            };
            let (d1, d2) = match adjust.direction {
                Direction::Left | Direction::Right => (degrees, 0.0),
                Direction::Up | Direction::Down => (0.0, degrees),
            };
            out.edit(tilt(plane, d1, d2), Some(Touch::Tilt), scratch);
        }
        Touch::Slice => {
            let Base::OfRange(fraction) = StepKind::SliceNormal.base().times(adjust.times) else {
                unreachable!("the compass's slice step is a fraction of the slice step's range")
            };
            let amount = fraction * 2.0 * Z0_HALF_RANGE;
            out.edit(slice(plane, amount), Some(Touch::Slice), scratch);
        }
    }
}

/// Enter on the compass's control `id`, as its click: Tilt or Slice sets the mode.
pub fn activate(id: &str, scratch: &mut Scratch) {
    match id {
        COMPASS_TILT => scratch.compass.mode = Touch::Tilt,
        COMPASS_SLICE => scratch.compass.mode = Touch::Slice,
        _ => {}
    }
}

/// The compass's margin, in points (a look choice, R-390).
pub const MARGIN: f32 = 12.0;

/// The cube's square in the compass's `rect`: at its top left inside the margin, as tall as the margin leaves, at most
/// half its width.
pub fn square(rect: Rect) -> Rect {
    let side = (rect.height() - 2.0 * MARGIN).min(rect.width() * 0.5);
    Rect::from_min_size(rect.min + vec2(MARGIN, MARGIN), vec2(side, side))
}

/// Draws the compass in `rect`, taking its drags and its buttons' clicks.
pub fn show(
    ui: &mut Ui,
    rect: Rect,
    sim: &SimConfig,
    keyboard: &mut Keyboard,
    scratch: &mut Scratch,
    out: &mut Out,
) {
    let visuals = ui.visuals().clone();
    ui.painter().rect_filled(rect, 0.0, visuals.panel_fill);
    ui.painter().rect_stroke(
        rect,
        0.0,
        visuals.widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Inside,
    );
    keyboard.place(COMPASS, rect);
    let square = square(rect);
    keyboard.place(COMPASS_VIEW, square);
    let plane = &sim.plane;
    let response = ui.interact(square, egui::Id::new("compass cube"), Sense::drag());
    if response.drag_started() {
        scratch.compass.drag = Some(Drag {
            mode: scratch.compass.mode,
            travel: Vec2::ZERO,
        });
    }
    if let Some(drag) = &mut scratch.compass.drag {
        drag.travel += response.drag_delta();
    }
    // The plane as the drag leaves it, turned about the pin while locked, as its edit will be.
    let pending: Vec<SetField> = scratch
        .compass
        .drag
        .and_then(|d| drag_edit(plane, &d, square))
        .map(|edit| about_the_pin(vec![edit], sim))
        .unwrap_or_default();
    if response.drag_stopped() {
        if let Some(drag) = scratch.compass.drag.take() {
            if let Some(edit) = drag_edit(plane, &drag, square) {
                out.edit(edit, Some(drag.mode), scratch);
            }
        }
    }
    let shown = after(plane, &pending);
    let view = View::of(&shown);
    draw(ui, square, &shown, &view, scratch.compass.mode, &sim.lock);
    let text = Rect::from_min_max(
        pos2(square.max.x + MARGIN, rect.min.y + MARGIN),
        rect.max - vec2(MARGIN, MARGIN),
    );
    let mut column = ui.new_child(UiBuilder::new().max_rect(text));
    column.spacing_mut().item_spacing.y = 4.0;
    column.label(RichText::new(TITLE).heading());
    column.horizontal(|ui| {
        for (mode, id, label) in [
            (Touch::Tilt, COMPASS_TILT, TILT_BUTTON),
            (Touch::Slice, COMPASS_SLICE, SLICE_BUTTON),
        ] {
            let button = ui.selectable_label(scratch.compass.mode == mode, label);
            keyboard.place_widget(id, &button);
            if button.clicked() {
                scratch.compass.mode = mode;
            }
        }
    });
    column.label(
        RichText::new(Angles::of(&view).readout())
            .monospace()
            .small(),
    );
    let hint = match scratch.compass.mode {
        Touch::Tilt => HINT_TILT,
        Touch::Slice => HINT_SLICE,
    };
    column.label(RichText::new(hint).weak().small());
}

/// Where the pin goes in the cube's frame: the anchor's chart point `(s, t)` on the tilted plane, whose corners are
/// `s, t ∈ {0, 1}`; `None` while free or for a degenerate basis.
pub fn pin(plane: &Plane, lock: &Lock, (c, r1, r2): Frame) -> Option<[f64; 3]> {
    if !lock.locked {
        return None;
    }
    let (s, t) = chart_coords(plane, &lock.z_locked)?;
    let (a, b) = (PLANE_HALF * (2.0 * s - 1.0), PLANE_HALF * (2.0 * t - 1.0));
    Some(std::array::from_fn(|i| c[i] + a * r1[i] + b * r2[i]))
}

/// Draws the cube, the plane and, locked, the pin at the anchor, in `square`.
fn draw(ui: &Ui, square: Rect, plane: &Plane, view: &View, mode: Touch, lock: &Lock) {
    let painter = ui.painter_at(square);
    let visuals = ui.visuals();
    let edge = Stroke::new(1.0, visuals.weak_text_color());
    let corner = |i: usize| {
        let bit = |b: usize| if i & (1 << b) == 0 { -1.0 } else { 1.0 };
        project(square, [bit(0), bit(1), bit(2)])
    };
    for i in 0..8usize {
        for b in 0..3 {
            if i & (1 << b) == 0 {
                painter.line_segment([corner(i), corner(i | (1 << b))], edge);
            }
        }
    }
    let blue = plane_colour(visuals);
    if mode == Touch::Tilt {
        let mut before = corners(square, plane_frame(plane, view, false));
        before.push(before[0]);
        painter.extend(Shape::dashed_line(
            &before,
            Stroke::new(1.5, blue),
            5.0,
            4.0,
        ));
    }
    let frame = plane_frame(plane, view, true);
    let poly = corners(square, frame);
    painter.add(Shape::convex_polygon(
        poly,
        blue.gamma_multiply(0.35),
        Stroke::new(2.0, blue),
    ));
    let label = |p: [f64; 3], text: String| {
        painter.text(
            project(square, p),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::monospace(10.0),
            visuals.weak_text_color(),
        );
    };
    label([1.25, -1.0, -1.0], format!("W {}", AXES[view.a]));
    label([-1.0, 1.2, -1.0], format!("H {}", AXES[view.b]));
    label([-1.0, -1.0, 1.3], format!("D {}", AXES[view.d]));
    if let Some(at) = pin(plane, lock, frame) {
        let centre = project(square, at);
        let head = centre - vec2(0.0, 18.0);
        painter.line_segment([centre, head], Stroke::new(2.0, GOLD));
        painter.circle_filled(centre, 3.0, GOLD);
        painter.circle_filled(head, 5.0, GOLD);
        painter.text(
            head + vec2(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            PINNED,
            egui::FontId::monospace(10.0),
            GOLD,
        );
    } else {
        painter.circle_filled(project(square, frame.0), 3.0, visuals.text_color());
    }
}
