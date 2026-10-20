//! The compass, the nav cube (render_gui_spec §G2, § "The compass: look and views (R-451, R-452)", §G3, §G4; R-409,
//! R-451): bottom left, under the view controls. The cube `[−1, 1]³` is the chart's frame `(e_a, e_b, e_d)`, Width =
//! chart x on the bottom-left edge, Height = chart y on the left vertical edge and Depth = the slice axis on the
//! bottom-right edge, in units of the slice step's half-range ([`Z0_HALF_RANGE`]). It is drawn as
//! `docs/gui/reference/principia_nav_cube.html` draws its cube, transcribed from the file's code, in the dark theme's
//! colours (R-451): the isometric projection [`Iso`], the twelve edges depth-sorted by `x + z` with the reference's
//! opacity and width laws, and the slice as the true cross-section polygon of the plane with the cube
//! ([`cross_section`]), sorted among the edges by its own depth, Lambert-shaded, with an in-plane grid clipped to it and
//! an outline; the edge labels and corner numbers; and a readout per mode. It has no orbit (R-409).
//!
//! The plane drawn, in both modes, is the decoder's: the normal of `View::basis`'s `(q₁, q₂)` in the cube's frame,
//! through the point of `z₀`'s components on `e_a`, `e_b` and `e_d` over the half-range, unclamped, its offset alone
//! clamped to `±|n|₁` ([`cut`]; R-451 B2). Where `s = 2|q₁ × q₂| / (|q₁|² + |q₂|²) < 10⁻³` ([`DRAW_CUT`], the compass's
//! own cut, R-451 B11) no slice is drawn, the readout says so, and a slice-mode edit is not sent.
//!
//! It has two modes, Tilt (the default) and Slice, with a button for each beside it, which show the current mode; a
//! click on one switches the mode, and touching a slice slider still switches it to Slice and touching a tilt to Tilt
//! (R-409). In tilt mode a drag sets `τ₁` by its horizontal travel and `τ₂` by its vertical, up raising it, the full
//! width or height 180°, each clamped to ±90° as it moves (one `SetField` on the basis); in slice mode its horizontal
//! travel moves the plane along the drawn plane's normal, the full width the whole range, and its vertical travel does
//! nothing (one `SetField` on `z₀`; R-451 B3). Each drag is one edit, made when it ends (R-96). The drawing eases toward
//! its targets by the reference's law ([`ease_factor`]; R-451 B4): the contract holds the target, so the figure, the
//! sliders and every `SetField` see it, never an eased value. Locked, it carries a gold pin at the anchor, the pivot
//! every basis edit turns about ([`super::manifold_view::about_the_pin`]), and a slice-mode drag is an excursion along
//! the normal through the anchor.
//!
//! Its keys (§G3): the compass is one element in navigation mode, beside its Tilt and Slice buttons; Enter on it
//! starts interaction mode, in which the arrows tilt in tilt mode and move the slice along the normal in slice mode,
//! Shift ×10 and Alt ×0.1 as everywhere (R-409 F3).
//!
//! Its views (R-452; [`views`]): four buttons beside Tilt and Slice, Cube (the default), Plücker, Jacobi and Shape, set
//! what it draws, the mode staying what a drag on the Cube does; in a grid view a drag does nothing, and in the Plücker
//! view a click on a cell, or Space on the keyboard's cell cursor, steers the slice to that coordinate plane. The view
//! is its own GUI state, as its mode is. The keys: the view buttons follow Slice as siblings, Enter on one switching the
//! view; in the Plücker view's interaction mode the arrows move the cell cursor ([`adjust`]) and Space steers
//! ([`space`]).

pub mod views;

use eframe::egui::{
    self, pos2, vec2, Color32, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, UiBuilder,
    Vec2,
};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Lock, Plane, SimConfig};
use engine::contract::view_ui::{Focus, KeyMode};

use crate::explore::lock::GOLD;
use crate::explore::manifold_view::slice_tilt::{Angles, TILT_RANGE};
use crate::explore::manifold_view::{
    about_the_pin, add, axis, basis_edit, chart_coords, plane_s, point, z0_edit, Out, Scratch,
    Touch, View, AXES, Z0_HALF_RANGE,
};
use crate::explore::{COMPASS, COMPASS_SLICE, COMPASS_TILT, COMPASS_VIEW};
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::{Adjust, Base, ScopeTree, StepKind};
use crate::keyboard::Keyboard;
use views::CompassView;

/// The compass's title.
pub const TITLE: &str = "Compass";
/// The Tilt button's label (a look choice, R-390).
pub const TILT_BUTTON: &str = "Tilt";
/// The Slice button's label (a look choice, R-390).
pub const SLICE_BUTTON: &str = "Slice";
/// The hint under the buttons in tilt mode, reworded for the reference's drags (a look choice, R-390; R-451 B7).
pub const HINT_TILT: &str = "drag across to tilt q₁, up and down to tilt q₂";
/// The hint under the buttons in slice mode (a look choice, R-390; R-451 B7).
pub const HINT_SLICE: &str = "drag across to move the plane along its normal";
/// The pin's label, as 08_lock.png's.
pub const PINNED: &str = "pinned";

// --- the reference's numbers (R-451 B6), transcribed from principia_nav_cube.html ------------------------------------

/// The fit's margin: `S = min(0.78·w/(2√3), 0.78·h/4)`.
pub const FIT: f64 = 0.78;
/// The isometric projection's `√3/2`.
pub const ISO: f64 = 0.866_025_403_784_438_6;
/// The light, `(0.35, 1.0, 0.55)`, normalised where it is used.
pub const LIGHT: [f64; 3] = [0.35, 1.0, 0.55];
/// The view, `(1, 1, 1)`, normalised where it is used: the face the viewer sees is lit.
pub const VIEW_DIR: [f64; 3] = [1.0, 1.0, 1.0];
/// The ambient term of the shade `0.2 + 0.8·max(0, n·L)`.
pub const AMBIENT: f64 = 0.2;
/// The depth term's span: the shade gains `(near − 0.5)·0.10`, `±0.05`.
pub const DEPTH_SHADE: f64 = 0.10;
/// The fill's opacity.
pub const FILL_OPACITY: f32 = 0.6;
/// An edge's opacity, `0.28 + 0.55·near`.
pub const EDGE_OPACITY: (f64, f64) = (0.28, 0.55);
/// An edge's width in points, `0.8 + 1.0·near`.
pub const EDGE_WIDTH: (f64, f64) = (0.8, 1.0);
/// The in-plane grid's spacing, in the cube's units.
pub const GRID_STEP: f64 = 0.5;
/// How far the grid reaches either side of the plane's foot, in the cube's units.
pub const GRID_REACH: f64 = 1.9;
/// The grid's opacity.
pub const GRID_OPACITY: f32 = 0.26;
/// The grid's width, in points.
pub const GRID_WIDTH: f32 = 0.7;
/// The outline's width, in points.
pub const OUTLINE_WIDTH: f32 = 1.7;
/// The outline's opacity.
pub const OUTLINE_OPACITY: f32 = 0.9;
/// How far an axis name sits out from its edge, in points.
pub const LABEL_OUT: f64 = 20.0;
/// How far a corner number sits out from its corner, away from the cube's centre, in points.
pub const NUMBER_OUT: f64 = 12.0;
/// An axis name's size, in points.
pub const LABEL_SIZE: f32 = 11.0;
/// A corner number's size, in points.
pub const NUMBER_SIZE: f32 = 9.0;
/// The degrees a drag across the cube's full width, or its full height, tilts.
pub const DRAG_DEGREES: f64 = 180.0;
/// The easing's base: each frame `v ← v + (target − v)·(1 − 0.0016^dt)`.
pub const EASE_BASE: f64 = 0.0016;
/// The longest frame the easing takes, in seconds.
pub const EASE_DT_CAP: f64 = 0.05;

// --- the compass's own choices ---------------------------------------------------------------------------------------

/// The compass's drawing cut on `s` (R-451 B11): below it no slice is drawn. Numerical, set by the 1e-12 normal check
/// (the normal's error is about `ε/s`); not the guards' 10⁻⁶ ([`super::manifold_view::PLANE_S_CUT`]).
pub const DRAW_CUT: f64 = 1e-3;
/// A cube corner within this of the plane is on it (applied per R-369).
pub const ON_PLANE: f64 = 1e-12;
/// Two points of the cross-section this close are one (applied per R-369).
pub const SAME_POINT: f64 = 1e-9;
/// An eased value this close to its target is at it, so a still compass draws its target exactly (applied per
/// R-369).
pub const EASE_SNAP: f64 = 1e-12;

/// A point of the cube's frame, `(x, y, z) = (e_a, e_b, e_d)`, `y` the vertical.
pub type P3 = [f64; 3];

/// The cube's eight corners, in the reference's order: `x` outermost, then `y`, then `z`.
pub const CORNERS: [P3; 8] = [
    [-1.0, -1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, 1.0, 1.0],
    [1.0, -1.0, -1.0],
    [1.0, -1.0, 1.0],
    [1.0, 1.0, -1.0],
    [1.0, 1.0, 1.0],
];

/// The cube's twelve edges, by their corners in [`CORNERS`], in the reference's order: each pair `i < j` one apart.
pub const EDGES: [(usize, usize); 12] = [
    (0, 1),
    (0, 2),
    (0, 4),
    (1, 3),
    (1, 5),
    (2, 3),
    (2, 6),
    (3, 7),
    (4, 5),
    (4, 6),
    (5, 7),
    (6, 7),
];

/// Width, chart x, the bottom-left edge.
pub const WIDTH_EDGE: [P3; 2] = [[-1.0, -1.0, 1.0], [1.0, -1.0, 1.0]];
/// Depth, the slice axis, the bottom-right edge.
pub const DEPTH_EDGE: [P3; 2] = [[1.0, -1.0, -1.0], [1.0, -1.0, 1.0]];
/// Height, chart y, the left vertical edge.
pub const HEIGHT_EDGE: [P3; 2] = [[-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0]];

fn dot3(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: P3) -> P3 {
    let len = dot3(a, a).sqrt();
    a.map(|x| x / len)
}

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// The reference's depth key of a point: `x + z`, larger nearer the viewer.
pub fn key([x, _, z]: P3) -> f64 {
    x + z
}

/// How near a depth key is, in `[0, 1]`: `(key + 2)/4`.
pub fn near(key: f64) -> f64 {
    (key + 2.0) / 4.0
}

/// The reference's isometric projection, fitted to a square: `P(x,y,z) = (CX + S·√3/2·(x − z), CY + S·(½(x + z) −
/// y))`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Iso {
    /// The centre's x, `CX`, in points.
    pub cx: f64,
    /// The centre's y, `CY`, in points.
    pub cy: f64,
    /// The scale `S`, in points per unit.
    pub s: f64,
}

impl Iso {
    /// The projection fitted to `square`, centred: `S = min(0.78·w/(2√3), 0.78·h/4)`.
    pub fn fit(square: Rect) -> Self {
        let (w, h) = (f64::from(square.width()), f64::from(square.height()));
        let centre = square.center();
        Self {
            cx: f64::from(centre.x),
            cy: f64::from(centre.y),
            s: (FIT * w / (4.0 * ISO)).min(FIT * h / 4.0),
        }
    }

    /// Where `p` falls, in points.
    pub fn at(&self, p: P3) -> Pos2 {
        let [x, y] = self.at64(p);
        pos2(x as f32, y as f32)
    }

    /// Where `p` falls, in points, at full precision.
    pub fn at64(&self, [x, y, z]: P3) -> [f64; 2] {
        [
            self.cx + self.s * ISO * (x - z),
            self.cy + self.s * (0.5 * (x + z) - y),
        ]
    }
}

// --- the plane in the cube's frame -----------------------------------------------------------------------------------

/// `v`'s components on the frame `(e_a, e_b, e_d)` of `view`.
pub fn in_frame(v: &Latent, view: &View) -> P3 {
    [v[view.a], v[view.b], v[view.d]]
}

/// The latent vector with components `r` on the frame of `view`.
pub fn lift(r: P3, view: &View) -> Latent {
    add(
        &add(&axis(view.a, r[0]), r[1], &axis(view.b, 1.0)),
        r[2],
        &axis(view.d, 1.0),
    )
}

/// `View::basis`'s `(q₁, q₂)` of `view`, in its cube's frame.
pub fn frame_basis(view: &View) -> (P3, P3) {
    let (q1, q2) = view.basis();
    (in_frame(&q1, view), in_frame(&q2, view))
}

/// R-451 B11's `s = 2|q₁ × q₂| / (|q₁|² + |q₂|²)` of `q₁`, `q₂` in the cube's frame: [`plane_s`] on the three
/// components.
pub fn cube_s(q1: P3, q2: P3) -> f64 {
    let pad = |r: P3| -> Latent { std::array::from_fn(|i| if i < 3 { r[i] } else { 0.0 }) };
    plane_s(&pad(q1), &pad(q2))
}

/// The unit normal `q₁ × q₂ / |q₁ × q₂|` of `view`'s plane in its cube's frame, `View::basis`'s; `None` where
/// `s < `[`DRAW_CUT`], where the compass draws no plane (R-451 B11). Untilted it is the depth axis.
pub fn drawn_normal(view: &View) -> Option<P3> {
    let (q1, q2) = frame_basis(view);
    (cube_s(q1, q2) >= DRAW_CUT).then(|| unit(cross(q1, q2)))
}

/// The point of `z₀`'s components on `e_a`, `e_b` and `e_d` of `view`, each over the half-range, unclamped.
pub fn point_of(z0: &Latent, view: &View) -> P3 {
    in_frame(z0, view).map(|x| x / Z0_HALF_RANGE)
}

/// A plane of the cube's frame: `normal · x = offset`, the normal a unit vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cut {
    /// The unit normal.
    pub normal: P3,
    /// The offset.
    pub offset: f64,
}

/// The plane the compass draws for `z₀` and `view`: [`drawn_normal`] through [`point_of`] `z₀`, its offset `n·p`
/// clamped to `±|n|₁`, where the plane still meets the cube (R-451 B2); `None` where `s < `[`DRAW_CUT`].
pub fn cut(z0: &Latent, view: &View) -> Option<Cut> {
    let normal = drawn_normal(view)?;
    let reach: f64 = normal.iter().map(|x| x.abs()).sum();
    let offset = dot3(normal, point_of(z0, view)).clamp(-reach, reach);
    Some(Cut { normal, offset })
}

/// The reference's in-plane basis `(u, v)` of the plane with unit normal `n`: `u = (h × n)/|h × n|`, `h` the vertical
/// unless `n` is near it, then `x`; `v = n × u`.
pub fn plane_basis(n: P3) -> (P3, P3) {
    let helper = if n[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let u = unit(cross(helper, n));
    (u, cross(n, u))
}

/// The cross-section of the plane `cut` with the cube `[−1, 1]³`, ordered round its centroid by the reference's
/// angle in [`plane_basis`]: 3 to 6 points for a plane through the cube's interior, a polygon; at `|offset| = |n|₁`
/// the contact, a face (4 points), an edge (2) or a corner (1); none for a plane past the cube. The points are the
/// corners on the plane (within [`ON_PLANE`]) and the crossings of the edges whose ends lie strictly either side.
pub fn cross_section(cut: Cut) -> Vec<P3> {
    let Cut { normal: n, offset } = cut;
    let side = |p: P3| dot3(n, p) - offset;
    let mut points: Vec<P3> = Vec::new();
    let mut push = |p: P3| {
        if !points
            .iter()
            .any(|q| (0..3).all(|i| (q[i] - p[i]).abs() <= SAME_POINT))
        {
            points.push(p);
        }
    };
    for corner in CORNERS {
        if side(corner).abs() <= ON_PLANE {
            push(corner);
        }
    }
    for (i, j) in EDGES {
        let (a, b) = (CORNERS[i], CORNERS[j]);
        let (da, db) = (side(a), side(b));
        if da.abs() <= ON_PLANE || db.abs() <= ON_PLANE || (da > 0.0) == (db > 0.0) {
            continue;
        }
        let t = da / (da - db);
        push(std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t));
    }
    if points.len() < 3 {
        return points;
    }
    let (u, v) = plane_basis(n);
    let count = points.len() as f64;
    let c: P3 = std::array::from_fn(|k| points.iter().map(|p| p[k]).sum::<f64>() / count);
    let angle = |p: &P3| {
        let d = sub(*p, c);
        dot3(d, v).atan2(dot3(d, u))
    };
    points.sort_by(|a, b| angle(a).total_cmp(&angle(b)));
    points
}

/// The part of the segment `a`–`b` inside the convex polygon `poly`, either winding (Cyrus–Beck); `None` when it misses
/// it or the polygon has no area.
pub fn clip(poly: &[[f64; 2]], a: [f64; 2], b: [f64; 2]) -> Option<([f64; 2], [f64; 2])> {
    let n = poly.len();
    let cross2 = |p: [f64; 2], q: [f64; 2]| p[0] * q[1] - p[1] * q[0];
    let minus = |p: [f64; 2], q: [f64; 2]| [p[0] - q[0], p[1] - q[1]];
    let area: f64 = (0..n).map(|i| cross2(poly[i], poly[(i + 1) % n])).sum();
    if area == 0.0 {
        return None;
    }
    let sign = area.signum();
    let d = minus(b, a);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let e = minus(q, p);
        // Inside where `num + t·den ≥ 0`.
        let num = sign * cross2(e, minus(a, p));
        let den = sign * cross2(e, d);
        if den == 0.0 {
            if num < 0.0 {
                return None;
            }
            continue;
        }
        let t = -num / den;
        if den > 0.0 {
            t0 = t0.max(t);
        } else {
            t1 = t1.min(t);
        }
    }
    (t0 < t1).then(|| {
        let at = |t: f64| [a[0] + t * d[0], a[1] + t * d[1]];
        (at(t0), at(t1))
    })
}

/// The reference's two-sided Lambert shade of a plane with normal `n`: `0.2 + 0.8·max(0, n·L)`, `n` flipped to face the
/// view.
pub fn face_shade(n: P3) -> f64 {
    let mut n = unit(n);
    if dot3(n, unit(VIEW_DIR)) < 0.0 {
        n = n.map(|x| -x);
    }
    AMBIENT + (1.0 - AMBIENT) * dot3(n, unit(LIGHT)).max(0.0)
}

/// The fill's shade of a slice with normal `n` at depth key `key`: [`face_shade`] plus the depth term, in `[0, 1]`.
pub fn fill_shade(n: P3, key: f64) -> f64 {
    (face_shade(n) + (near(key) - 0.5) * DEPTH_SHADE).clamp(0.0, 1.0)
}

/// The in-plane grid of the slice `cut` with cross-section `poly` (at least 3 points): lines [`GRID_STEP`] apart along
/// [`plane_basis`]'s `u` and `v`, reaching [`GRID_REACH`] either side of the plane's foot `n·offset`, each clipped to
/// the polygon.
pub fn grid(cut: Cut, poly: &[P3]) -> Vec<[P3; 2]> {
    let (u, v) = plane_basis(cut.normal);
    let foot = cut.normal.map(|x| x * cut.offset);
    let flat: Vec<[f64; 2]> = poly
        .iter()
        .map(|p| {
            let d = sub(*p, foot);
            [dot3(d, u), dot3(d, v)]
        })
        .collect();
    let up = |[a, b]: [f64; 2]| -> P3 { std::array::from_fn(|k| foot[k] + a * u[k] + b * v[k]) };
    let steps = (2.0 * GRID_REACH / GRID_STEP + 1e-9).floor() as usize;
    let mut lines = Vec::new();
    for along_u in [true, false] {
        for k in 0..=steps {
            let c = -GRID_REACH + k as f64 * GRID_STEP;
            let (a, b) = if along_u {
                ([c, -GRID_REACH], [c, GRID_REACH])
            } else {
                ([-GRID_REACH, c], [GRID_REACH, c])
            };
            if let Some((p, q)) = clip(&flat, a, b) {
                lines.push([up(p), up(q)]);
            }
        }
    }
    lines
}

// --- the drawing ------------------------------------------------------------------------------------------------------

/// One thing the cube draws, in its depth order.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// An edge, its corners, its opacity and its width in points.
    Edge {
        /// Its first corner.
        from: P3,
        /// Its second corner.
        to: P3,
        /// `0.28 + 0.55·near`.
        opacity: f64,
        /// `0.8 + 1.0·near`, in points.
        width: f64,
    },
    /// The slice, [`Drawing::slice`].
    Slice,
}

/// The slice as drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceDrawing {
    /// Its plane.
    pub cut: Cut,
    /// Its cross-section, ordered: 3 to 6 points a polygon, filled; 2 an edge and 1 a corner, outlined only.
    pub points: Vec<P3>,
    /// Its depth key, the points' mean `x + z`.
    pub key: f64,
    /// The fill's shade, [`fill_shade`].
    pub shade: f64,
    /// The in-plane grid, clipped to the polygon.
    pub grid: Vec<[P3; 2]>,
}

/// A label: its text, its centre in points, and whether it is in the accent colour.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    /// The text.
    pub text: String,
    /// Its centre, in points.
    pub at: Pos2,
    /// Whether it takes the accent colour, as the reference's Depth and tilt labels do.
    pub accent: bool,
}

/// What the cube draws, as data: the paint follows it, and the capture records it.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
    /// The projection.
    pub iso: Iso,
    /// The edges and the slice, in drawing order.
    pub items: Vec<Item>,
    /// The slice; `None` where `s < `[`DRAW_CUT`].
    pub slice: Option<SliceDrawing>,
    /// The axis names.
    pub labels: Vec<Label>,
    /// The corner numbers.
    pub numbers: Vec<Label>,
    /// The readout's lines.
    pub readout: Vec<String>,
    /// The pin, in the cube's frame, while locked.
    pub pin: Option<P3>,
}

/// The cube's edges and `slice` in the reference's drawing order: by depth key, the slice pushed last so it draws
/// after the edges of its own key (a stable sort).
pub fn items(slice: Option<&SliceDrawing>) -> Vec<Item> {
    let mut keyed: Vec<(f64, Item)> = EDGES
        .iter()
        .map(|&(i, j)| {
            let (from, to) = (CORNERS[i], CORNERS[j]);
            let k = (key(from) + key(to)) / 2.0;
            let n = near(k);
            (
                k,
                Item::Edge {
                    from,
                    to,
                    opacity: EDGE_OPACITY.0 + EDGE_OPACITY.1 * n,
                    width: EDGE_WIDTH.0 + EDGE_WIDTH.1 * n,
                },
            )
        })
        .collect();
    if let Some(slice) = slice {
        keyed.push((slice.key, Item::Slice));
    }
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    keyed.into_iter().map(|(_, item)| item).collect()
}

/// The slice the compass draws for `z₀` and `view`: [`cut`]'s cross-section, its depth, shade and grid; `None` where
/// `s < `[`DRAW_CUT`] or the plane misses the cube.
pub fn slice_drawing(z0: &Latent, view: &View) -> Option<SliceDrawing> {
    let cut = cut(z0, view)?;
    let points = cross_section(cut);
    if points.is_empty() {
        return None;
    }
    let key = points.iter().map(|p| key(*p)).sum::<f64>() / points.len() as f64;
    let grid = if points.len() >= 3 {
        grid(cut, &points)
    } else {
        Vec::new()
    };
    Some(SliceDrawing {
        cut,
        shade: fill_shade(cut.normal, key),
        points,
        key,
        grid,
    })
}

/// An axis name on the edge `edge`, centred on it and pushed [`LABEL_OUT`] outward from the cube's centre.
pub fn edge_label(iso: &Iso, edge: [P3; 2], text: String, accent: bool) -> Label {
    let ([ax, ay], [bx, by]) = (iso.at64(edge[0]), iso.at64(edge[1]));
    let (dx, dy) = (bx - ax, by - ay);
    let len = dx.hypot(dy);
    let (mx, my) = ((ax + bx) / 2.0, (ay + by) / 2.0);
    let (mut px, mut py) = (-dy / len, dx / len);
    if (mx - iso.cx) * px + (my - iso.cy) * py < 0.0 {
        (px, py) = (-px, -py);
    }
    Label {
        text,
        at: pos2((mx + px * LABEL_OUT) as f32, (my + py * LABEL_OUT) as f32),
        accent,
    }
}

/// The labels of `mode` in `view`'s frame: slice mode names Width, Depth and Height by their axes; tilt mode names only
/// the tilt target, the slice axis, on the bottom-right edge (R-451 B1).
pub fn labels(iso: &Iso, view: &View, mode: Touch) -> Vec<Label> {
    match mode {
        Touch::Slice => vec![
            edge_label(iso, WIDTH_EDGE, AXES[view.a].to_owned(), false),
            edge_label(iso, DEPTH_EDGE, AXES[view.d].to_owned(), true),
            edge_label(iso, HEIGHT_EDGE, AXES[view.b].to_owned(), false),
        ],
        Touch::Tilt => vec![edge_label(iso, DEPTH_EDGE, tilt_label(view), true)],
    }
}

/// Tilt mode's one label: "q₁, q₂ → `<axis>`".
pub fn tilt_label(view: &View) -> String {
    format!("q₁, q₂ → {}", AXES[view.d])
}

/// Slice mode's corner numbers: −1.0 and 1.0 at the ends of Width, Depth and Height, one number per corner, each
/// [`NUMBER_OUT`] out from the cube's centre; tilt mode has none.
pub fn numbers(iso: &Iso, mode: Touch) -> Vec<Label> {
    if mode == Touch::Tilt {
        return Vec::new();
    }
    let mut seen: Vec<((i64, i64), [f64; 2], &str)> = Vec::new();
    for edge in [WIDTH_EDGE, DEPTH_EDGE, HEIGHT_EDGE] {
        for (corner, text) in [(edge[0], "−1.0"), (edge[1], "1.0")] {
            let s = iso.at64(corner);
            let place = (s[0].round() as i64, s[1].round() as i64);
            if !seen.iter().any(|(p, _, _)| *p == place) {
                seen.push((place, s, text));
            }
        }
    }
    seen.into_iter()
        .map(|(_, [x, y], text)| {
            let (ox, oy) = (x - iso.cx, y - iso.cy);
            let len = ox.hypot(oy);
            Label {
                text: text.to_owned(),
                at: pos2(
                    (x + ox / len * NUMBER_OUT) as f32,
                    (y + oy / len * NUMBER_OUT) as f32,
                ),
                accent: false,
            }
        })
        .collect()
}

/// A signed value as the reference writes it: "+" before a value ≥ 0, `decimals` places.
pub fn signed(value: f64, decimals: usize) -> String {
    let value = value + 0.0;
    let sign = if value >= 0.0 { "+" } else { "" };
    format!("{sign}{value:.decimals$}")
}

/// The tilt as the readout reads it, as the reference's: "q₁→`<axis>` +30.0° · q₂→`<axis>` +0.0°", q₁'s and q₂'s target
/// the slice axis (R-451 B1).
pub fn tilt_readout(view: &View) -> String {
    let a = Angles::of(view);
    let axis = AXES[view.d];
    format!(
        "q₁→{axis} {}° · q₂→{axis} {}°",
        signed(a.tau1, 1),
        signed(a.tau2, 1)
    )
}

/// The rotation as the readout reads it: "γ +0.0°" (§G2: the compass reads out the tilt and rotation angles).
pub fn rotation_readout(view: &View) -> String {
    format!("γ {}°", signed(Angles::of(view).gamma, 1))
}

/// The readout of `mode` for `z₀` and `view` (R-451 B5): slice mode, "mode slice · plane ⟂ `<axis>`" untilted or the
/// tilt with a tilt set, then "`<axis>` = +0.25 of range"; tilt mode, "mode tilt", the tilt and the rotation. Where
/// `s < `[`DRAW_CUT`], a line "no plane drawn: s < 10⁻³ at τ₁ = <τ₁>°, τ₂ = <τ₂>° (RQ-340)" (R-451 B11).
pub fn readout(z0: &Latent, view: &View, mode: Touch) -> Vec<String> {
    let axis = AXES[view.d];
    let tilted = view.tau1 != 0.0 || view.tau2 != 0.0;
    let mut lines = match mode {
        Touch::Slice => vec![
            if tilted {
                format!("mode slice · {}", tilt_readout(view))
            } else {
                format!("mode slice · plane ⟂ {axis}")
            },
            format!(
                "{axis} = {} of range",
                signed(z0[view.d] / Z0_HALF_RANGE, 2)
            ),
        ],
        Touch::Tilt => vec![
            "mode tilt".to_owned(),
            tilt_readout(view),
            rotation_readout(view),
        ],
    };
    if drawn_normal(view).is_none() {
        let a = Angles::of(view);
        lines.push(format!(
            "no plane drawn: s < 10⁻³ at τ₁ = {}°, τ₂ = {}° (RQ-340)",
            signed(a.tau1, 1),
            signed(a.tau2, 1)
        ));
    }
    lines
}

/// Where the pin goes in the cube's frame, while locked: the anchor's chart point `(s, t)` on `plane`, `z(s, t)`'s
/// components over the half-range; where the guards read no plane ([`chart_coords`] is `None`), the anchor's own
/// components. `None` while free.
pub fn pin(plane: &Plane, view: &View, lock: &Lock) -> Option<P3> {
    if !lock.locked {
        return None;
    }
    let at = chart_coords(plane, &lock.z_locked).map_or(lock.z_locked, |(s, t)| point(plane, s, t));
    Some(point_of(&at, view))
}

/// What the cube draws in `square` for the plane `plane`, read in `view`'s frame, in `mode`, with `lock`.
pub fn drawing(square: Rect, plane: &Plane, view: &View, mode: Touch, lock: &Lock) -> Drawing {
    let iso = Iso::fit(square);
    let slice = slice_drawing(&plane.z0, view);
    Drawing {
        iso,
        items: items(slice.as_ref()),
        slice,
        labels: labels(&iso, view, mode),
        numbers: numbers(&iso, mode),
        readout: readout(&plane.z0, view, mode),
        pin: pin(plane, view, lock),
    }
}

// --- the colours (look choices, R-390; R-451 B6) ---------------------------------------------------------------------

/// The edges' colour: the dark theme's stroke colour, its opacity by the reference's law.
pub fn edge_colour(visuals: &egui::Visuals) -> Color32 {
    visuals.widgets.noninteractive.fg_stroke.color
}

/// The fill's two tones, dark to light: the theme's selection fill and its selection stroke.
pub fn fill_tones(visuals: &egui::Visuals) -> (Color32, Color32) {
    (visuals.selection.bg_fill, visuals.selection.stroke.color)
}

/// The fill's colour at `shade`, between [`fill_tones`], opaque; the fill draws it at [`FILL_OPACITY`].
pub fn fill_colour(visuals: &egui::Visuals, shade: f64) -> Color32 {
    let (dark, light) = fill_tones(visuals);
    let mix = |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * shade) as u8;
    Color32::from_rgb(
        mix(dark.r(), light.r()),
        mix(dark.g(), light.g()),
        mix(dark.b(), light.b()),
    )
}

/// The grid's and the outline's colour: the theme's selection stroke.
pub fn plane_colour(visuals: &egui::Visuals) -> Color32 {
    visuals.selection.stroke.color
}

/// The colour of a label: the theme's strong text, or its accent, the hyperlink colour.
pub fn label_colour(visuals: &egui::Visuals, accent: bool) -> Color32 {
    if accent {
        visuals.hyperlink_color
    } else {
        visuals.strong_text_color()
    }
}

/// Paints `drawing` with `painter`.
pub fn paint(painter: &Painter, visuals: &egui::Visuals, drawing: &Drawing) {
    let iso = drawing.iso;
    for item in &drawing.items {
        match item {
            Item::Edge {
                from,
                to,
                opacity,
                width,
            } => {
                painter.line_segment(
                    [iso.at(*from), iso.at(*to)],
                    Stroke::new(
                        *width as f32,
                        edge_colour(visuals).gamma_multiply(*opacity as f32),
                    ),
                );
            }
            Item::Slice => {
                if let Some(slice) = &drawing.slice {
                    paint_slice(painter, visuals, &iso, slice);
                }
            }
        }
    }
    for label in &drawing.labels {
        painter.text(
            label.at,
            egui::Align2::CENTER_CENTER,
            &label.text,
            egui::FontId::proportional(LABEL_SIZE),
            label_colour(visuals, label.accent),
        );
    }
    for number in &drawing.numbers {
        painter.text(
            number.at,
            egui::Align2::CENTER_CENTER,
            &number.text,
            egui::FontId::proportional(NUMBER_SIZE),
            visuals.weak_text_color(),
        );
    }
    if let Some(at) = drawing.pin {
        let centre = iso.at(at);
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
    }
}

/// Paints the slice: a polygon filled, its grid and its outline; an edge or a corner contact outlined only.
fn paint_slice(painter: &Painter, visuals: &egui::Visuals, iso: &Iso, slice: &SliceDrawing) {
    let screen: Vec<Pos2> = slice.points.iter().map(|p| iso.at(*p)).collect();
    let outline = Stroke::new(
        OUTLINE_WIDTH,
        plane_colour(visuals).gamma_multiply(OUTLINE_OPACITY),
    );
    match screen.len() {
        1 => {
            painter.circle_filled(screen[0], OUTLINE_WIDTH, outline.color);
        }
        2 => {
            painter.line_segment([screen[0], screen[1]], outline);
        }
        _ => {
            painter.add(Shape::convex_polygon(
                screen.clone(),
                fill_colour(visuals, slice.shade).gamma_multiply(FILL_OPACITY),
                Stroke::NONE,
            ));
            let grid = Stroke::new(
                GRID_WIDTH,
                plane_colour(visuals).gamma_multiply(GRID_OPACITY),
            );
            for [a, b] in &slice.grid {
                painter.line_segment([iso.at(*a), iso.at(*b)], grid);
            }
            painter.add(Shape::closed_line(screen, outline));
        }
    }
}

// --- the drags, the easing and the edits -----------------------------------------------------------------------------

/// A drag on the compass in progress: its targets, moved as the pointer moves, as the reference's are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    /// What it does: the mode when it started, tilting or slicing.
    pub mode: Touch,
    /// `(τ₁, τ₂)` in degrees when it started.
    pub from: (f64, f64),
    /// The tilt's targets `(τ₁, τ₂)` in degrees, each clamped to ±90° as the pointer moves.
    pub tau: (f64, f64),
    /// The slice's move along the drawn plane's normal, in `z₀`'s units.
    pub amount: f64,
}

impl Drag {
    /// A drag in `mode` starting on `plane`.
    pub fn start(mode: Touch, plane: &Plane) -> Self {
        let a = Angles::of(&View::of(plane));
        Self {
            mode,
            from: (a.tau1, a.tau2),
            tau: (a.tau1, a.tau2),
            amount: 0.0,
        }
    }

    /// The pointer moved by `delta` over the cube's `square`: in tilt mode `τ₁` by the horizontal travel over the
    /// width and `τ₂` by the vertical over the height, up raising it, [`DRAG_DEGREES`] across, each clamped to ±90°;
    /// in slice mode the horizontal travel moves the slice, the full width the whole range `2·`[`Z0_HALF_RANGE`], and
    /// the vertical does nothing (R-451 B3).
    pub fn moved(&mut self, delta: Vec2, square: Rect) {
        let dx = f64::from(delta.x) / f64::from(square.width());
        let dy = f64::from(delta.y) / f64::from(square.height());
        match self.mode {
            Touch::Tilt => {
                let (lo, hi) = TILT_RANGE;
                self.tau.0 = (self.tau.0 + dx * DRAG_DEGREES).clamp(lo, hi);
                self.tau.1 = (self.tau.1 - dy * DRAG_DEGREES).clamp(lo, hi);
            }
            Touch::Slice => self.amount += dx * 2.0 * Z0_HALF_RANGE,
        }
    }
}

/// The one edit `drag` on `plane` makes when it ends: the tilt to its targets, `γ` kept, or the slice along the drawn
/// plane's normal; none when it moved nothing, or for a slice where the compass draws no plane (R-451 B11).
pub fn drag_edit(plane: &Plane, drag: &Drag) -> Option<SetField> {
    match drag.mode {
        Touch::Tilt => {
            if drag.tau == drag.from {
                return None;
            }
            let view = View::of(plane);
            let angles = Angles {
                tau1: drag.tau.0,
                tau2: drag.tau.1,
                gamma: Angles::of(&view).gamma,
            };
            Some(basis_edit(angles.apply(view).basis()))
        }
        Touch::Slice => {
            if drag.amount == 0.0 {
                return None;
            }
            slice(plane, drag.amount)
        }
    }
}

/// The unit normal of `plane` as the compass draws it, in the latent space: [`drawn_normal`] on the preset's
/// `(e_a, e_b, e_d)` (R-409 A3; R-451 B3); `None` where `s < `[`DRAW_CUT`].
pub fn latent_normal(plane: &Plane) -> Option<Latent> {
    let view = View::of(plane);
    drawn_normal(&view).map(|n| lift(n, &view))
}

/// The edit moving `plane`'s centre `z₀` by `amount` along the drawn plane's normal; the basis, so the tilt and `γ`,
/// unchanged. Locked, `z₀` leaves the anchor along that line: an excursion, as the slice step's is. `None` where the
/// compass draws no plane (R-451 B11).
pub fn slice(plane: &Plane, amount: f64) -> Option<SetField> {
    latent_normal(plane).map(|n| z0_edit(add(&plane.z0, amount, &n)))
}

/// The tilt edit turning `τ₁` by `d1` and `τ₂` by `d2` degrees, within their span.
pub fn tilt(plane: &Plane, d1: f64, d2: f64) -> SetField {
    let view = View::of(plane);
    let mut angles = Angles::of(&view);
    angles.tau1 = (angles.tau1 + d1).clamp(TILT_RANGE.0, TILT_RANGE.1);
    angles.tau2 = (angles.tau2 + d2).clamp(TILT_RANGE.0, TILT_RANGE.1);
    basis_edit(angles.apply(view).basis())
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

/// The easing's factor for a frame of `dt` seconds: `1 − 0.0016^dt`, `dt` capped at [`EASE_DT_CAP`].
pub fn ease_factor(dt: f64) -> f64 {
    1.0 - EASE_BASE.powf(dt.clamp(0.0, EASE_DT_CAP))
}

/// `value` eased toward `target` by `k`; at it once within [`EASE_SNAP`].
pub fn ease(value: f64, target: f64, k: f64) -> f64 {
    let next = value + (target - value) * k;
    if (target - next).abs() <= EASE_SNAP {
        target
    } else {
        next
    }
}

/// `a − b` as an angle in `(−π, π]`.
pub fn turn_between(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(std::f64::consts::TAU);
    if d > std::f64::consts::PI {
        d - std::f64::consts::TAU
    } else {
        d
    }
}

/// What the compass shows: the view and the point of `z₀` it draws, eased toward the plane's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shown {
    /// The view: its frame, scale, tilts and rotation.
    pub view: View,
    /// The point of `z₀` in the cube's frame, [`point_of`].
    pub point: P3,
}

impl Shown {
    /// What `plane` shows at rest.
    pub fn of(plane: &Plane) -> Self {
        let view = View::of(plane);
        Self {
            view,
            point: point_of(&plane.z0, &view),
        }
    }

    /// The plane shown, on `target`: `View::basis`'s `(q₁, q₂)` of the view, and `target`'s `z₀` with its components
    /// on the frame set to the point's.
    pub fn plane(&self, target: &Plane) -> Plane {
        let (q1, q2) = self.view.basis();
        let mut z0 = target.z0;
        for (i, axis) in [self.view.a, self.view.b, self.view.d]
            .into_iter()
            .enumerate()
        {
            z0[axis] = self.point[i] * Z0_HALF_RANGE;
        }
        Plane { z0, q1, q2 }
    }

    /// `from` eased toward `to` over a frame of `dt` seconds (R-451 B4): the tilts, the rotation the short way round,
    /// and the point; the frame and the scale are `to`'s. From nothing, or across a change of frame, it is `to`.
    pub fn eased(from: Option<Self>, to: Self, dt: f64) -> Self {
        let Some(from) = from else { return to };
        let (f, t) = (from.view, to.view);
        if (f.a, f.b, f.d) != (t.a, t.b, t.d) {
            return to;
        }
        let k = ease_factor(dt);
        let gamma = {
            let next = f.gamma + turn_between(t.gamma, f.gamma) * k;
            if turn_between(t.gamma, next).abs() <= EASE_SNAP {
                t.gamma
            } else {
                next
            }
        };
        Self {
            view: View {
                tau1: ease(f.tau1, t.tau1, k),
                tau2: ease(f.tau2, t.tau2, k),
                gamma,
                ..t
            },
            point: std::array::from_fn(|i| ease(from.point[i], to.point[i], k)),
        }
    }
}

/// The compass's own state: its mode, its view, its drag, what it shows and the keyboard's cell cursor. The GUI's,
/// never the engine's, never undoable (R-409 A5; R-452 C1).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Compass {
    /// Its mode, which its buttons show: Tilt by default.
    pub mode: Touch,
    /// Its view, which its view buttons show: Cube by default (R-452).
    pub view: CompassView,
    /// The keyboard's cell cursor in the Plücker view, `(i, j)` with `i < j`, while the compass is in interaction mode
    /// (R-452 C6).
    pub cursor: Option<(usize, usize)>,
    /// The drag in progress.
    pub drag: Option<Drag>,
    /// What it showed last frame, easing toward the plane.
    pub shown: Option<Shown>,
    /// Whether what it shows is still easing.
    pub moving: bool,
}

/// The arrows on the compass in interaction mode, by its view and mode. In the Cube view, in tilt mode ← → tilt `τ₁`
/// and ↑ ↓ `τ₂` by the `Tilt` step; in slice mode ↑ and → move the slice along the normal, ↓ and ← back, by the
/// `SliceNormal` step (REQ-GUI-184), and nothing where the compass draws no plane (R-451 B11). Shift's ×10 and Alt's
/// ×0.1 are in `adjust.times`, as everywhere (R-409 F3). In the Plücker view they move the cell cursor
/// ([`views::cursor_moved`]; R-452 C6), editing nothing; in the Jacobi and Shape views, which have no grid yet, nothing.
pub fn adjust(adjust: &Adjust, plane: &Plane, scratch: &mut Scratch, out: &mut Out) {
    match scratch.compass.view {
        CompassView::Cube => {}
        CompassView::Plucker => {
            if let Some(at) = scratch.compass.cursor {
                scratch.compass.cursor =
                    Some(views::cursor_moved(at, adjust.direction, adjust.times));
            }
            return;
        }
        CompassView::Jacobi | CompassView::Shape => return,
    }
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
            if let Some(edit) = slice(plane, amount) {
                out.edit(edit, Some(Touch::Slice), scratch);
            }
        }
    }
}

/// Enter on the compass's control `id`, as its click: Tilt or Slice sets the mode, leaving the view; a view button sets
/// the view, leaving the mode (R-452 C1).
pub fn activate(id: &str, scratch: &mut Scratch) {
    match id {
        COMPASS_TILT => scratch.compass.mode = Touch::Tilt,
        COMPASS_SLICE => scratch.compass.mode = Touch::Slice,
        _ => {
            if let Some(view) = CompassView::of_button(id) {
                scratch.compass.view = view;
            }
        }
    }
}

/// Names the compass in the breadcrumb by its view, "Compass › Plücker" in a grid view (R-452 C6), before the keys run.
pub fn join(tree: &mut ScopeTree, compass: &Compass) {
    if let Some(scope) = tree.get_mut(COMPASS_VIEW) {
        scope.label = compass.view.breadcrumb();
    }
}

/// Whether `focus` is on the compass itself in interaction mode.
pub fn interacting(focus: &Focus) -> bool {
    focus.mode == KeyMode::Interaction && focus.path.last().is_some_and(|id| id == COMPASS_VIEW)
}

/// Follows the keyboard after its keys ran: in the Plücker view, interaction mode on the compass puts the cell cursor
/// on the dominant cell, or the first upper cell when none is, and keeps it there as the arrows move it; leaving
/// interaction mode, or the view, takes it away (R-452 C6).
pub fn follow_focus(focus: &Focus, plane: &Plane, compass: &mut Compass) {
    if !interacting(focus) || compass.view != CompassView::Plucker {
        compass.cursor = None;
    } else if compass.cursor.is_none() {
        compass.cursor = Some(views::cursor_start(&views::plucker(plane)));
    }
}

/// Space on the compass in the Plücker view's interaction mode: the steer to the cell cursor, as a click on its cell
/// (R-452 C6). Nothing in navigation mode, or in another view.
pub fn space(focus: &Focus, plane: &Plane, scratch: &mut Scratch, out: &mut Out) {
    if !interacting(focus) || scratch.compass.view != CompassView::Plucker {
        return;
    }
    if let Some(edit) = scratch
        .compass
        .cursor
        .and_then(|(i, j)| views::steer(plane, i, j))
    {
        out.edit(edit, None, scratch);
    }
}

/// The compass's margin, in points (a look choice, R-390).
pub const MARGIN: f32 = 12.0;

/// The cube's square in the compass's `rect`: at its top left inside the margin, as tall as the margin leaves, at most
/// half its width, a whole number of points, so the reference renders at its size (applied per R-369).
pub fn square(rect: Rect) -> Rect {
    let side = (rect.height() - 2.0 * MARGIN)
        .min(rect.width() * 0.5)
        .floor();
    Rect::from_min_size(rect.min + vec2(MARGIN, MARGIN), vec2(side, side))
}

/// What the compass in `rect` draws for `sim`, from what `compass` shows: the drawing [`show`] paints and the capture
/// records.
pub fn current(rect: Rect, sim: &SimConfig, compass: &Compass) -> Drawing {
    let shown = compass.shown.unwrap_or_else(|| Shown::of(&sim.plane));
    let plane = shown.plane(&sim.plane);
    drawing(square(rect), &plane, &shown.view, compass.mode, &sim.lock)
}

/// Draws the compass in `rect`, taking its drags and its buttons' clicks: the Cube view's cube and its drags, or the
/// Plücker view's grid, its position readout and its cells' clicks, or the Jacobi and Shape views' note; beside it the
/// title, the Tilt and Slice buttons, the view buttons, the view's readout and its hint.
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
    let view = scratch.compass.view;
    // In a grid view a drag does nothing (R-452 C1): the cube takes none.
    let mut pending: Vec<SetField> = Vec::new();
    if view == CompassView::Cube {
        let response = ui.interact(square, egui::Id::new("compass cube"), Sense::drag());
        if response.drag_started() {
            scratch.compass.drag = Some(Drag::start(scratch.compass.mode, plane));
        }
        if let Some(drag) = &mut scratch.compass.drag {
            drag.moved(response.drag_delta(), square);
        }
        // The plane as the drag leaves it, turned about the pin while locked, as its edit will be.
        pending = scratch
            .compass
            .drag
            .and_then(|d| drag_edit(plane, &d))
            .map(|edit| about_the_pin(vec![edit], sim))
            .unwrap_or_default();
        if response.drag_stopped() {
            if let Some(drag) = scratch.compass.drag.take() {
                if let Some(edit) = drag_edit(plane, &drag) {
                    out.edit(edit, Some(drag.mode), scratch);
                }
            }
        }
    } else {
        scratch.compass.drag = None;
    }
    let target = Shown::of(&after(plane, &pending));
    let dt = ui.input(|i| f64::from(i.unstable_dt));
    let shown = Shown::eased(scratch.compass.shown, target, dt);
    scratch.compass.shown = Some(shown);
    scratch.compass.moving = shown != target;
    if scratch.compass.moving {
        ui.ctx().request_repaint();
    }
    let painter = ui.painter_at(rect);
    let (readout, hint) = match view {
        CompassView::Cube => {
            let drawing = current(rect, sim, &scratch.compass);
            paint(&painter, &visuals, &drawing);
            let hint = match scratch.compass.mode {
                Touch::Tilt => HINT_TILT,
                Touch::Slice => HINT_SLICE,
            };
            (drawing.readout, hint)
        }
        CompassView::Plucker => {
            let drawing = views::plucker_drawing(square, plane, scratch.compass.cursor);
            views::paint_plucker(&painter, &visuals, &drawing);
            cells(ui, &drawing, plane, scratch, out);
            (vec![drawing.readout], views::PLUCKER_HELP)
        }
        CompassView::Jacobi | CompassView::Shape => {
            let galley = painter.layout(
                views::WAITS.to_owned(),
                egui::FontId::proportional(LABEL_SIZE),
                visuals.weak_text_color(),
                square.width(),
            );
            let at = square.center() - galley.size() / 2.0;
            painter.galley(at, galley, visuals.weak_text_color());
            (Vec::new(), "")
        }
    };
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
    column.horizontal_wrapped(|ui| {
        for (view, id, label) in views::VIEWS {
            let button = ui.selectable_label(scratch.compass.view == view, label);
            keyboard.place_widget(id, &button);
            if button.clicked() {
                scratch.compass.view = view;
            }
        }
    });
    for line in &readout {
        column.label(RichText::new(line).monospace().small());
    }
    if !hint.is_empty() {
        column.label(RichText::new(hint).weak().small());
    }
}

/// The Plücker view's upper cells as widgets: each shows its names and value on hover, as egui's tooltip beside the
/// compass (R-452 C2), and a click on one steers the slice to its coordinate plane (C5).
fn cells(
    ui: &mut Ui,
    drawing: &views::PluckerDrawing,
    plane: &Plane,
    scratch: &mut Scratch,
    out: &mut Out,
) {
    for (i, j) in views::upper() {
        let cell = ui
            .interact(
                drawing.place.cell_rect(i, j),
                egui::Id::new(("plucker cell", i, j)),
                Sense::click(),
            )
            .on_hover_text(views::hover_text(&drawing.grid, i, j));
        if cell.clicked() {
            if let Some(edit) = views::steer(plane, i, j) {
                out.edit(edit, None, scratch);
            }
        }
    }
}

/// `drawing` of the compass's `square` as the capture records it (R-451 B8): the square's top left (`origin`) and
/// size in points, then positions in points from its top left, the cube's points in its frame.
pub fn record(drawing: &Drawing, square: Rect) -> serde_json::Value {
    let iso = drawing.iso;
    let (ox, oy) = (f64::from(square.min.x), f64::from(square.min.y));
    let rel = |p: P3| {
        let [x, y] = iso.at64(p);
        [x - ox, y - oy]
    };
    let rel_pos = |p: Pos2| [f64::from(p.x) - ox, f64::from(p.y) - oy];
    let items: Vec<serde_json::Value> = drawing
        .items
        .iter()
        .map(|item| match item {
            Item::Edge {
                from,
                to,
                opacity,
                width,
            } => serde_json::json!({
                "edge": [from, to],
                "screen": [rel(*from), rel(*to)],
                "opacity": opacity,
                "width": width,
            }),
            Item::Slice => serde_json::json!({
                "slice": drawing.slice.as_ref().map(|s| s.key),
            }),
        })
        .collect();
    let labels = |labels: &[Label]| -> Vec<serde_json::Value> {
        labels
            .iter()
            .map(|l| serde_json::json!({ "text": l.text, "at": rel_pos(l.at) }))
            .collect()
    };
    let points = drawing
        .slice
        .as_ref()
        .map(|s| s.points.clone())
        .unwrap_or_default();
    serde_json::json!({
        "origin": [ox, oy],
        "square": [square.width(), square.height()],
        "s": iso.s,
        "centre": [iso.cx - ox, iso.cy - oy],
        "corners": CORNERS.iter().map(|c| rel(*c)).collect::<Vec<_>>(),
        "items": items,
        "polygon": points,
        "polygon_screen": points.iter().map(|p| rel(*p)).collect::<Vec<_>>(),
        "fill": points.len() >= 3,
        "grid": drawing.slice.as_ref().map_or(0, |s| s.grid.len()),
        "labels": labels(&drawing.labels),
        "numbers": labels(&drawing.numbers),
        "readout": drawing.readout,
        "pin": drawing.pin.map(rel),
    })
}
