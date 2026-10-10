//! The compass, the nav cube (render_gui_spec §G2, §G4; GUI_DESIGN_NOTES § "01 Explore", § "08 Lock"; 01_main.png,
//! 08_lock.png): bottom left, under the view controls. The cube is the chart's frame, its axes the preset's two axes
//! and the hidden slice direction; inside it, the slice plane: through the centre, at the slice step along the depth
//! axis, tilted by `τ₁`, `τ₂` and turned by `γ`. It switches mode by itself: touching a slice slider shows slicing,
//! touching a tilt shows tilting, and tilting draws the plane before the tilt dashed. Locked, it carries a gold pin at
//! the pivot, the plane's centre, about which the plane turns. Dragging the plane tilts it (one `SetField` on the
//! basis); dragging the cube orbits the cube, the GUI's own view of it, which edits nothing. It reads out the tilt and
//! rotation angles. Its keys (§G3): the arrows tilt, Shift+arrows orbit.

use eframe::egui::{
    self, pos2, vec2, Color32, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, UiBuilder,
};
use engine::contract::sim_config::Plane;

use crate::explore::lock::GOLD;
use crate::explore::manifold_view::slice_tilt::{Angles, TILT_RANGE};
use crate::explore::manifold_view::{basis_edit, Out, Scratch, Touch, View, AXES, Z0_HALF_RANGE};
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::{Adjust, Base, StepKind};
use crate::keyboard::Keyboard;

/// The compass's title.
pub const TITLE: &str = "Compass";
/// The two modes' names, as 01_main.png's.
pub const SLICING: &str = "slicing";
/// See [`SLICING`].
pub const TILTING: &str = "tilting";
/// The hint under the modes, as 01_main.png's.
pub const HINT: &str = "drag the plane to tilt · drag the cube to orbit";
/// The pin's label, as 08_lock.png's.
pub const PINNED: &str = "pinned";
/// The orbit the cube opens at, yaw and pitch in degrees (a look choice, R-390).
pub const ORBIT: (f64, f64) = (-35.0, 25.0);
/// The degrees a point of drag turns a tilt or the orbit (a look choice, R-390).
pub const DEGREES_PER_POINT: f64 = 0.5;
/// The cube's half-size on the screen, as a fraction of the compass's square (a look choice, R-390).
pub const CUBE_SCALE: f64 = 0.26;
/// The plane's half-size inside the cube's, which is 1 (a look choice, R-390).
pub const PLANE_HALF: f64 = 0.7;
/// The plane's colour: egui's dark theme's selection blue (a look choice, R-390).
pub fn plane_colour(visuals: &egui::Visuals) -> Color32 {
    visuals.selection.stroke.color
}

/// What a drag on the compass does: decided where it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drag {
    /// It started on the plane: it tilts.
    Tilt,
    /// It started on the cube off the plane: it orbits.
    Orbit,
}

/// The compass's own state: its mode, its orbit and its drag. The GUI's, never the engine's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Compass {
    /// Which it shows.
    pub mode: Touch,
    /// The orbit's yaw, in degrees.
    pub yaw: f64,
    /// The orbit's pitch, in degrees, within ±89.
    pub pitch: f64,
    /// The drag in progress.
    pub drag: Option<Drag>,
}

impl Default for Compass {
    fn default() -> Self {
        Self {
            mode: Touch::Slice,
            yaw: ORBIT.0,
            pitch: ORBIT.1,
            drag: None,
        }
    }
}

impl Compass {
    /// Orbits by `yaw` and `pitch` degrees.
    pub fn orbit(&mut self, yaw: f64, pitch: f64) {
        self.yaw = (self.yaw + yaw).rem_euclid(360.0);
        self.pitch = (self.pitch + pitch).clamp(-89.0, 89.0);
    }

    /// A point of the cube's frame, `(w, h, d)` each in `[−1, 1]`, on the screen in `square`.
    pub fn project(&self, square: Rect, [w, h, d]: [f64; 3]) -> Pos2 {
        let (sy, cy) = self.yaw.to_radians().sin_cos();
        let (sp, cp) = self.pitch.to_radians().sin_cos();
        let x = w * cy + d * sy;
        let depth = -w * sy + d * cy;
        let y = h * cp - depth * sp;
        let r = f64::from(square.width()) * CUBE_SCALE;
        square.center() + vec2((x * r) as f32, (-y * r) as f32)
    }
}

/// The slice plane's centre and in-plane directions in the cube's frame: the centre at the slice step along the depth
/// axis, the directions `u₁`, `u₂` tilted by `τ₁`, `τ₂` and turned by `γ`.
pub fn plane_frame(plane: &Plane, view: &View, tilted: bool) -> ([f64; 3], [f64; 3], [f64; 3]) {
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

/// The plane's four corners on the screen.
pub fn corners(
    compass: &Compass,
    square: Rect,
    (c, r1, r2): ([f64; 3], [f64; 3], [f64; 3]),
) -> Vec<Pos2> {
    [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)]
        .iter()
        .map(|(a, b)| {
            let p = std::array::from_fn(|i| c[i] + PLANE_HALF * (a * r1[i] + b * r2[i]));
            compass.project(square, p)
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
pub fn tilt(plane: &Plane, d1: f64, d2: f64) -> engine::contract::set_field::SetField {
    let view = View::of(plane);
    let mut angles = Angles::of(&view);
    angles.tau1 = (angles.tau1 + d1).clamp(TILT_RANGE.0, TILT_RANGE.1);
    angles.tau2 = (angles.tau2 + d2).clamp(TILT_RANGE.0, TILT_RANGE.1);
    basis_edit(angles.apply(view).basis())
}

/// The arrows on the focused compass: ← → tilt `τ₁`, ↑ ↓ tilt `τ₂`; with Shift they orbit the cube instead, Shift's
/// ×10 being the orbit's own ×1.
pub fn adjust(adjust: &Adjust, plane: &Plane, scratch: &mut Scratch, out: &mut Out) {
    let horizontal = matches!(adjust.direction, Direction::Left | Direction::Right);
    if adjust.shift {
        let Base::Degrees(base) = StepKind::Orbit.base() else {
            unreachable!("the orbit's base step is in degrees")
        };
        let degrees = base * adjust.times / 10.0;
        let (yaw, pitch) = if horizontal {
            (degrees, 0.0)
        } else {
            (0.0, degrees)
        };
        scratch.compass.orbit(yaw, pitch);
        return;
    }
    let Base::Degrees(degrees) = adjust.delta() else {
        unreachable!("the compass's tilt step is in degrees")
    };
    let (d1, d2) = if horizontal {
        (degrees, 0.0)
    } else {
        (0.0, degrees)
    };
    out.edit(tilt(plane, d1, d2), Some(Touch::Tilt), scratch);
}

/// The compass's margin, in points (a look choice, R-390).
pub const MARGIN: f32 = 12.0;

/// The cube's square in the compass's `rect`: at its top left inside the margin, as tall as the margin leaves, at most
/// half its width.
pub fn square(rect: Rect) -> Rect {
    let side = (rect.height() - 2.0 * MARGIN).min(rect.width() * 0.5);
    Rect::from_min_size(rect.min + vec2(MARGIN, MARGIN), vec2(side, side))
}

/// Draws the compass in `rect`, taking its drags.
pub fn show(
    ui: &mut Ui,
    rect: Rect,
    plane: &Plane,
    locked: bool,
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
    keyboard.place(crate::explore::COMPASS, rect);
    let square = square(rect);
    let view = View::of(plane);
    let compass = scratch.compass;
    let tilted = plane_frame(plane, &view, true);
    let poly = corners(&compass, square, tilted);
    let response = ui.interact(square, egui::Id::new("compass cube"), Sense::drag());
    if response.drag_started() {
        let on_plane = response
            .interact_pointer_pos()
            .is_some_and(|p| inside(&poly, p));
        scratch.compass.drag = Some(if on_plane { Drag::Tilt } else { Drag::Orbit });
    }
    if response.dragged() {
        let d = response.drag_delta();
        let (dx, dy) = (
            f64::from(d.x) * DEGREES_PER_POINT,
            f64::from(d.y) * DEGREES_PER_POINT,
        );
        if d != egui::Vec2::ZERO {
            match scratch.compass.drag {
                Some(Drag::Tilt) => out.edit(tilt(plane, dx, -dy), Some(Touch::Tilt), scratch),
                Some(Drag::Orbit) | None => scratch.compass.orbit(dx, dy),
            }
        }
    }
    if response.drag_stopped() {
        scratch.compass.drag = None;
    }
    draw(ui, square, plane, &view, &scratch.compass, locked);
    let text = Rect::from_min_max(
        pos2(square.max.x + MARGIN, rect.min.y + MARGIN),
        rect.max - vec2(MARGIN, MARGIN),
    );
    let mut column = ui.new_child(UiBuilder::new().max_rect(text));
    column.spacing_mut().item_spacing.y = 4.0;
    column.label(RichText::new(TITLE).heading());
    column.horizontal(|ui| {
        for (mode, name) in [(Touch::Slice, SLICING), (Touch::Tilt, TILTING)] {
            let on = scratch.compass.mode == mode;
            let dot = if on { "●" } else { "○" };
            let text = RichText::new(format!("{dot} {name}")).monospace();
            ui.label(if on {
                text.color(plane_colour(&visuals))
            } else {
                text.weak()
            });
        }
    });
    column.label(
        RichText::new(Angles::of(&view).readout())
            .monospace()
            .small(),
    );
    column.label(RichText::new(HINT).weak().small());
}

/// Draws the cube, the plane and, locked, the pin, in `square`.
fn draw(ui: &Ui, square: Rect, plane: &Plane, view: &View, compass: &Compass, locked: bool) {
    let painter = ui.painter_at(square);
    let visuals = ui.visuals();
    let edge = Stroke::new(1.0, visuals.weak_text_color());
    let corner = |i: usize| {
        let bit = |b: usize| if i & (1 << b) == 0 { -1.0 } else { 1.0 };
        compass.project(square, [bit(0), bit(1), bit(2)])
    };
    for i in 0..8usize {
        for b in 0..3 {
            if i & (1 << b) == 0 {
                painter.line_segment([corner(i), corner(i | (1 << b))], edge);
            }
        }
    }
    let blue = plane_colour(visuals);
    if compass.mode == Touch::Tilt {
        let mut before = corners(compass, square, plane_frame(plane, view, false));
        before.push(before[0]);
        painter.extend(Shape::dashed_line(
            &before,
            Stroke::new(1.5, blue),
            5.0,
            4.0,
        ));
    }
    let frame = plane_frame(plane, view, true);
    let poly = corners(compass, square, frame);
    painter.add(Shape::convex_polygon(
        poly,
        blue.gamma_multiply(0.35),
        Stroke::new(2.0, blue),
    ));
    let label = |p: [f64; 3], text: String| {
        painter.text(
            compass.project(square, p),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::monospace(10.0),
            visuals.weak_text_color(),
        );
    };
    label([1.25, -1.0, -1.0], format!("W {}", AXES[view.a]));
    label([-1.0, 1.2, -1.0], format!("H {}", AXES[view.b]));
    label([-1.0, -1.0, 1.3], format!("D {}", AXES[view.d]));
    let centre = compass.project(square, frame.0);
    if locked {
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
        painter.circle_filled(centre, 3.0, visuals.text_color());
    }
}
