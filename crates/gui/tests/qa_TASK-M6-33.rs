//! QA tests for TASK-M6-33, written from the requirements and R-451, not from the implementation:
//!
//! - REQ-GUI-191: the compass's cube is drawn as `principia_nav_cube.html` draws it (R-451): the isometric projection
//!   `P(x,y,z) = (CX + S·√3/2·(x − z), CY + S·(½(x + z) − y))`, `S = min(0.78·w/(2√3), 0.78·h/4)`, centred; the edges
//!   depth-sorted by `x + z`, opacity `0.28 + 0.55·near` and width `0.8 + 1.0·near` px, `near = (x + z + 2)/4` at the
//!   edge's middle; the slice the true cross-section, sorted among the edges by its own mean `x + z`; the labels (slice
//!   mode Width, Depth, Height on the bottom-left, bottom-right and left vertical edges, pushed 20 px outward, with
//!   −1.0 / 1.0 at the corners, one number per corner, 12 px outward; tilt mode the one label "q₁, q₂ → <axis>" on the
//!   bottom-right edge). The cases are B8's four (render_gui_spec § "The compass: look and views (R-451, R-452)").
//! - REQ-GUI-192: the cross-section of the decoder's plane (View::basis's `(q₁, q₂)` in the frame `(e_a, e_b, e_d)`,
//!   through `z₀`'s point over the half-range, unclamped, the offset alone clamped to `±|n|₁`) is a convex ordered
//!   polygon of 3 to 6 vertices, 4 to 6 through the central axis, wherever `s = 2|q₁ × q₂| / (|q₁|² + |q₂|²) ≥ 10⁻³`;
//!   and where `s < 10⁻³` no slice, the pin kept, the readout's "no plane drawn" line (R-451 B2, B11).
//! - REQ-GUI-193: the readout per mode, the drags at the reference's rates (slice mode along the drawn normal, the
//!   cube's full width the whole range, the vertical doing nothing; tilt mode the full width or height 180°, each
//!   clamped to ±90°), one SetField per drag, and the easing `v ← v + (target − v)·(1 − 0.0016^dt)`, `dt` capped at
//!   0.05 s, in the drawing only (R-451 B3 to B5).
//!
//! The decoder's plane is computed here from R-451's own construction, `u₁ = cos τ₁ e_a + sin τ₁ e_d`,
//! `u₂ = cos τ₂ e_b + sin τ₂ e_d`, `q₁ = σ(cos γ u₁ + sin γ u₂)`, `q₂ = σ(cos γ u₂ − sin γ u₁)`, and its cross-section
//! by an edge walk of this file's own, apart from the code under test. Each check runs its control (R-176) through
//! [`rejects`].
// The file name `qa_TASK-M6-33` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect};
use engine::contract::log::Source;
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Lock, Plane};
use engine::contract::snapshot::Snapshot;
use gui::app::App;
use gui::capture::{FORMAT, PIXELS_PER_POINT, SIZE};
use gui::explore::compass::{self, Cut, Drawing, Item};
use gui::explore::lock::GOLD;
use gui::explore::manifold_view::{Touch, View, Z0_HALF_RANGE};
use gui::headless::Headless;
use gui::layout::Layout;
use gui::mock::MockEngine;
use gui::side::MockSide;

type Z = [f64; 8];
type P3 = [f64; 3];

/// The default chart's frame (render_gui_spec §G2: the chart `z_α × z_β`, the depth axis `z_q3`, as 01_main.png's
/// compass names it): `e_a`, `e_b`, `e_d`.
const A: usize = 0;
const B: usize = 1;
const D: usize = 5;
const WIDTH_NAME: &str = "z_α";
const HEIGHT_NAME: &str = "z_β";
const DEPTH_NAME: &str = "z_q3";

/// R-451 B11's drawing cut on `s`.
const CUT: f64 = 1e-3;
/// REQ-GUI-192's tolerance on the plane, the cube and the normal.
const TOL: f64 = 1e-12;

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

// --- vectors ----------------------------------------------------------------------------------------------------------

fn dot3(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn sub3(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn unit3(a: P3) -> P3 {
    let n = dot3(a, a).sqrt();
    a.map(|x| x / n)
}

fn l1(a: P3) -> f64 {
    a.iter().map(|x| x.abs()).sum()
}

// --- the decoder's plane, computed apart ------------------------------------------------------------------------------

/// R-451's construction of `View::basis`, in the cube's frame: `(q₁, q₂)` at `τ₁`, `τ₂`, `γ` in degrees, scale `σ`.
fn decoder_basis(t1: f64, t2: f64, g: f64, sigma: f64) -> (P3, P3) {
    let (t1, t2, g) = (t1.to_radians(), t2.to_radians(), g.to_radians());
    let u1 = [t1.cos(), 0.0, t1.sin()];
    let u2 = [0.0, t2.cos(), t2.sin()];
    let q1 = std::array::from_fn(|i| sigma * (g.cos() * u1[i] + g.sin() * u2[i]));
    let q2 = std::array::from_fn(|i| sigma * (g.cos() * u2[i] - g.sin() * u1[i]));
    (q1, q2)
}

/// R-451 B11's `s`.
fn s_of(q1: P3, q2: P3) -> f64 {
    2.0 * dot3(cross3(q1, q2), cross3(q1, q2)).sqrt() / (dot3(q1, q1) + dot3(q2, q2))
}

/// A latent vector with components `r` on the frame `(e_a, e_b, e_d)` and `rest` elsewhere.
fn lift(r: P3, rest: &Z) -> Z {
    let mut z = *rest;
    z[A] = r[0];
    z[B] = r[1];
    z[D] = r[2];
    z
}

/// The plane at `τ₁`, `τ₂`, `γ` (degrees), scale `σ`, with `z₀`'s point `p` (in half-ranges) on the frame.
fn plane_at(t1: f64, t2: f64, g: f64, sigma: f64, p: P3) -> Plane {
    let (q1, q2) = decoder_basis(t1, t2, g, sigma);
    Plane {
        z0: lift(p.map(|x| x * Z0_HALF_RANGE), &[0.0; 8]),
        q1: lift(q1, &[0.0; 8]),
        q2: lift(q2, &[0.0; 8]),
    }
}

/// The decoder's plane through `p`: the unit normal of `(q₁, q₂)` and the offset `n·p` clamped to `±|n|₁` (R-451 B2).
fn decoder_cut(t1: f64, t2: f64, g: f64, p: P3) -> (P3, f64) {
    let (q1, q2) = decoder_basis(t1, t2, g, 1.0);
    let n = unit3(cross3(q1, q2));
    (n, dot3(n, p).clamp(-l1(n), l1(n)))
}

/// The cube's corners and its twelve edges, the pairs one coordinate apart.
fn corners() -> Vec<P3> {
    let mut c = Vec::new();
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                c.push([x, y, z]);
            }
        }
    }
    c
}

fn cube_edges() -> Vec<(P3, P3)> {
    let c = corners();
    let mut e = Vec::new();
    for i in 0..8 {
        for j in i + 1..8 {
            let differ = (0..3).filter(|k| c[i][*k] != c[j][*k]).count();
            if differ == 1 {
                e.push((c[i], c[j]));
            }
        }
    }
    assert_eq!(e.len(), 12);
    e
}

/// This file's own cross-section of `n·x = c` with the cube: the edges' crossings and the corners on the plane,
/// unordered, one per point.
fn my_section(n: P3, c: f64) -> Vec<P3> {
    let mut pts: Vec<P3> = Vec::new();
    let mut put = |p: P3| {
        if !pts
            .iter()
            .any(|q| (0..3).all(|k| (q[k] - p[k]).abs() < 1e-9))
        {
            pts.push(p);
        }
    };
    for (a, b) in cube_edges() {
        let (da, db) = (dot3(n, a) - c, dot3(n, b) - c);
        if da.abs() <= TOL {
            put(a);
        }
        if db.abs() <= TOL {
            put(b);
        }
        if da.abs() > TOL && db.abs() > TOL && da.signum() != db.signum() {
            let t = da / (da - db);
            put(std::array::from_fn(|k| a[k] + t * (b[k] - a[k])));
        }
    }
    pts
}

fn near3(a: P3, b: P3, tol: f64) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() <= tol)
}

/// `got` and `want` hold the same points, each within `tol`.
fn same_set(got: &[P3], want: &[P3], tol: f64) -> bool {
    got.len() == want.len()
        && want.iter().all(|w| got.iter().any(|g| near3(*g, *w, tol)))
        && got.iter().all(|g| want.iter().any(|w| near3(*g, *w, tol)))
}

/// `got` is the cycle `want`, from any start, either way round, each vertex within `tol`.
fn same_cycle(got: &[P3], want: &[P3], tol: f64) -> bool {
    let n = want.len();
    got.len() == n
        && (0..n).any(|s| {
            (0..n).all(|i| near3(got[i], want[(s + i) % n], tol))
                || (0..n).all(|i| near3(got[i], want[(s + n - i) % n], tol))
        })
}

/// REQ-GUI-192's polygon checks: `poly` has `lo..=hi` vertices, each on `n·x = c` and inside the cube within 1e-12,
/// convex and ordered round the polygon.
fn check_polygon(poly: &[P3], n: P3, c: f64, lo: usize, hi: usize, what: &str) {
    assert!(
        (lo..=hi).contains(&poly.len()),
        "{what}: {} vertices, not {lo} to {hi}: {poly:?}",
        poly.len()
    );
    for v in poly {
        assert!(
            (dot3(n, *v) - c).abs() <= TOL,
            "{what}: the vertex {v:?} is off the plane by {}",
            dot3(n, *v) - c
        );
        assert!(
            v.iter().all(|x| x.abs() <= 1.0 + TOL),
            "{what}: the vertex {v:?} is outside the cube"
        );
    }
    // Convex and ordered: every turn round the polygon is the same way, none straight.
    let k = poly.len();
    let turns: Vec<f64> = (0..k)
        .map(|i| {
            let (a, b, c) = (poly[i], poly[(i + 1) % k], poly[(i + 2) % k]);
            dot3(cross3(sub3(b, a), sub3(c, b)), n)
        })
        .collect();
    assert!(
        turns.iter().all(|t| *t > 1e-12) || turns.iter().all(|t| *t < -1e-12),
        "{what}: not convex and ordered, turns {turns:?}: {poly:?}"
    );
}

/// What the compass draws for `plane` in `mode`, `lock`, in a 172-point square (B8's size).
fn draw(plane: &Plane, mode: Touch, lock: &Lock) -> Drawing {
    let square = Rect::from_min_size(egui::pos2(20.0, 30.0), egui::vec2(172.0, 172.0));
    compass::drawing(square, plane, &View::of(plane), mode, lock)
}

fn free() -> Lock {
    Lock {
        locked: false,
        z_locked: [0.0; 8],
    }
}

/// The drawn normal, signed to face `n`, checked against `n` within 1e-12.
fn check_normal(cut: &Cut, n: P3, what: &str) {
    let sign = dot3(cut.normal, n).signum();
    let off = (0..3)
        .map(|k| (cut.normal[k] * sign - n[k]).abs())
        .fold(0.0, f64::max);
    assert!(
        off <= TOL,
        "{what}: the drawn normal {:?} is {off} from the decoder's {n:?}",
        cut.normal
    );
}

// === REQ-GUI-192: the cross-section and the plane it follows ==========================================================

#[test]
fn qa_compass_cross_section_sweep_is_the_decoders_plane_and_a_convex_polygon() {
    let taus: Vec<f64> = (-12..=12).map(|i| f64::from(i) * 7.5).collect();
    let gammas = [0.0, 30.0, 45.0, -120.0, 180.0];
    let depths = [-1.0, -0.6, -0.25, 0.0, 0.25, 0.6, 1.0];
    let off_axis = [[0.8, -0.7, 0.9], [-0.95, 0.9, -0.1], [0.3, 0.3, -0.99]];
    let (mut drawn, mut skipped) = (0, 0);
    for &t1 in &taus {
        for &t2 in &taus {
            for &g in &gammas {
                let (q1, q2) = decoder_basis(t1, t2, g, 1.0);
                let s = s_of(q1, q2);
                if s < CUT {
                    skipped += 1;
                    continue;
                }
                let n = unit3(cross3(q1, q2));
                let points: Vec<(P3, bool)> = depths
                    .iter()
                    .map(|d| ([0.0, 0.0, *d], true))
                    .chain(off_axis.iter().map(|p| (*p, false)))
                    .collect();
                for (p, on_axis) in points {
                    let what = format!("τ₁ {t1}°, τ₂ {t2}°, γ {g}°, z₀'s point {p:?}");
                    let d = draw(&plane_at(t1, t2, g, 1.0, p), Touch::Tilt, &free());
                    let slice = d
                        .slice
                        .as_ref()
                        .unwrap_or_else(|| panic!("{what}: s = {s} and no slice drawn"));
                    check_normal(&slice.cut, n, &what);
                    let sign = dot3(slice.cut.normal, n).signum();
                    let c = dot3(n, p).clamp(-l1(n), l1(n));
                    assert!(
                        (slice.cut.offset * sign - c).abs() <= TOL,
                        "{what}: the offset {} is not n·p = {c}",
                        slice.cut.offset * sign
                    );
                    let open = on_axis && p[2].abs() < 1.0;
                    let (lo, hi) = if open { (4, 6) } else { (3, 6) };
                    check_polygon(&slice.points, n, c, lo, hi, &what);
                    assert!(
                        same_set(&slice.points, &my_section(n, c), 1e-9),
                        "{what}: the polygon {:?} is not the cross-section {:?}",
                        slice.points,
                        my_section(n, c)
                    );
                    drawn += 1;
                }
            }
        }
    }
    // The sweep reaches the cut: (±90°, ±90°) are skipped, every γ.
    assert_eq!(skipped, 4 * gammas.len(), "the tilts with s < 10⁻³");
    assert!(drawn > 10_000);
    // Controls: a fixed square at the depth under a tilt, and the reference's axis-angle normal at τ₁ = τ₂ = 30°.
    let (n, c) = decoder_cut(30.0, 30.0, 0.0, [0.0, 0.0, 0.25]);
    let square = [
        [-1.0, -1.0, 0.25],
        [1.0, -1.0, 0.25],
        [1.0, 1.0, 0.25],
        [-1.0, 1.0, 0.25],
    ];
    rejects("a fixed square under a tilt", || {
        check_polygon(&square, n, c, 4, 6, "a fixed square")
    });
    let d = draw(
        &plane_at(30.0, 30.0, 0.0, 1.0, [0.0, 0.0, 0.25]),
        Touch::Tilt,
        &free(),
    );
    let m = (30.0_f64.to_radians()).hypot(30.0_f64.to_radians());
    let t = 30.0_f64.to_radians();
    // The reference's map tips the vertical normal toward (τ₁, 0, τ₂); in the cube's frame its y is the vertical.
    let axis_angle = [m.sin() * t / m, m.cos(), m.sin() * t / m];
    let cut = d.slice.expect("drawn").cut;
    rejects("the reference's axis-angle normal", || {
        check_normal(&cut, axis_angle, "the axis-angle normal")
    });
}

#[test]
fn qa_compass_cross_section_untilted_is_the_rectangle_at_the_depth() {
    for d in [-0.75, -0.25, 0.0, 0.25, 0.9] {
        for g in [0.0, 30.0, 90.0] {
            for mode in [Touch::Tilt, Touch::Slice] {
                let drawing = draw(&plane_at(0.0, 0.0, g, 1.0, [0.0, 0.0, d]), mode, &free());
                let poly = drawing.slice.expect("drawn").points;
                let want = [
                    [-1.0, -1.0, d],
                    [1.0, -1.0, d],
                    [1.0, 1.0, d],
                    [-1.0, 1.0, d],
                ];
                assert!(
                    same_cycle(&poly, &want, TOL),
                    "untilted at depth {d}, γ {g}°: {poly:?}"
                );
            }
        }
    }
    // Control: the reference's own tilt-mode plane at τ = 0, the horizontal one.
    let poly = draw(
        &plane_at(0.0, 0.0, 0.0, 1.0, [0.0; 3]),
        Touch::Tilt,
        &free(),
    )
    .slice
    .unwrap()
    .points;
    let horizontal = [
        [-1.0, 0.0, -1.0],
        [1.0, 0.0, -1.0],
        [1.0, 0.0, 1.0],
        [-1.0, 0.0, 1.0],
    ];
    rejects("the horizontal plane at τ = 0", || {
        assert!(same_cycle(&poly, &horizontal, TOL))
    });
}

#[test]
fn qa_compass_cross_section_slice_mode_draws_the_tilted_plane_through_z0s_point() {
    // Both modes draw the decoder's plane (R-451 B2): with a tilt set, slice mode shows it tilted, through z₀'s point,
    // on and off the central axis.
    for p in [[0.0, 0.0, 0.3], [0.4, -0.3, 0.2], [-0.6, 0.5, -0.4]] {
        let plane = plane_at(30.0, -20.0, 15.0, 1.0, p);
        let (n, c) = decoder_cut(30.0, -20.0, 15.0, p);
        let slice_mode = draw(&plane, Touch::Slice, &free()).slice.expect("drawn");
        let tilt_mode = draw(&plane, Touch::Tilt, &free()).slice.expect("drawn");
        check_normal(&slice_mode.cut, n, "slice mode");
        assert_eq!(
            slice_mode.points, tilt_mode.points,
            "the modes draw one plane"
        );
        check_polygon(&slice_mode.points, n, c, 3, 6, "slice mode, tilted");
        // Through z₀'s point: the offset is n·p, unclamped here.
        assert!((c - dot3(n, p)).abs() == 0.0);
        let sign = dot3(slice_mode.cut.normal, n).signum();
        assert!((slice_mode.cut.offset * sign - dot3(n, p)).abs() <= TOL);
        // Control: the untilted plane at the point's depth drawn while τ₁ ≠ 0.
        let flat = [
            [-1.0, -1.0, p[2]],
            [1.0, -1.0, p[2]],
            [1.0, 1.0, p[2]],
            [-1.0, 1.0, p[2]],
        ];
        rejects("an untilted plane in slice mode while τ₁ ≠ 0", || {
            check_polygon(&flat, n, c, 3, 6, "untilted")
        });
    }
}

#[test]
fn qa_compass_cross_section_rect_hex_and_triangle_cases() {
    // tilt_rect: τ₁ = 30°, normal ∝ (−½, 0, √3/2), z = x tan 30°.
    let t = 30.0_f64.to_radians().tan();
    let rect = [
        [1.0, 1.0, t],
        [1.0, -1.0, t],
        [-1.0, -1.0, -t],
        [-1.0, 1.0, -t],
    ];
    let d = draw(
        &plane_at(30.0, 0.0, 0.0, 1.0, [0.0; 3]),
        Touch::Tilt,
        &free(),
    );
    let slice = d.slice.expect("drawn");
    check_normal(&slice.cut, [-0.5, 0.0, 3.0_f64.sqrt() / 2.0], "tilt_rect");
    assert!(
        same_cycle(&slice.points, &rect, TOL),
        "tilt_rect: {:?}",
        slice.points
    );
    // tilt_hex: τ₁ = τ₂ = 45°, z = x + y, the regular hexagon: six vertices through the centre, cutting six faces.
    let hex = [
        [1.0, -1.0, 0.0],
        [1.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [-1.0, 1.0, 0.0],
        [-1.0, 0.0, -1.0],
        [0.0, -1.0, -1.0],
    ];
    let d = draw(
        &plane_at(45.0, 45.0, 0.0, 1.0, [0.0; 3]),
        Touch::Tilt,
        &free(),
    );
    let slice = d.slice.expect("drawn");
    check_normal(&slice.cut, unit3([-0.5, -0.5, 0.5]), "tilt_hex");
    assert!(
        same_cycle(&slice.points, &hex, TOL),
        "tilt_hex: {:?}",
        slice.points
    );
    // Regular: every side the same length.
    let sides: Vec<f64> = (0..6)
        .map(|i| {
            let v = sub3(slice.points[(i + 1) % 6], slice.points[i]);
            dot3(v, v).sqrt()
        })
        .collect();
    assert!(sides.iter().all(|s| (s - 2.0_f64.sqrt()).abs() < TOL));
    // τ₁ = τ₂ = −45°, through (0, 0, 1): x + y + z = 1, the triangle, within the 3-to-6 bound (|d| = 1).
    let tri = [[-1.0, 1.0, 1.0], [1.0, -1.0, 1.0], [1.0, 1.0, -1.0]];
    let d = draw(
        &plane_at(-45.0, -45.0, 0.0, 1.0, [0.0, 0.0, 1.0]),
        Touch::Tilt,
        &free(),
    );
    let slice = d.slice.expect("drawn");
    assert!(
        same_cycle(&slice.points, &tri, TOL),
        "the triangle: {:?}",
        slice.points
    );
    let (n, c) = decoder_cut(-45.0, -45.0, 0.0, [0.0, 0.0, 1.0]);
    check_polygon(&slice.points, n, c, 3, 6, "the triangle");
    rejects("the 4-to-6 bound at |d| = 1", || {
        check_polygon(&slice.points, n, c, 4, 6, "the triangle")
    });
    // Control: the rectangle is not the hexagon.
    rejects("tilt_rect's rectangle at τ₁ = τ₂ = 45°", || {
        assert!(same_cycle(&rect, &hex, TOL))
    });
}

#[test]
fn qa_compass_cross_section_a_pan_leaves_the_drawing_and_the_offset_alone_is_clamped() {
    // τ₁ = 30°, z₀ = 0, a pan of 2 half-ranges along q₁: z₀'s point (1.732, 0, 1), past the half-range.
    let t = 30.0_f64.to_radians();
    let p = [2.0 * t.cos(), 0.0, 2.0 * t.sin()];
    let rect = [
        [1.0, 1.0, t.tan()],
        [1.0, -1.0, t.tan()],
        [-1.0, -1.0, -t.tan()],
        [-1.0, 1.0, -t.tan()],
    ];
    let panned = draw(&plane_at(30.0, 0.0, 0.0, 1.0, p), Touch::Tilt, &free())
        .slice
        .expect("drawn");
    assert!(
        same_cycle(&panned.points, &rect, TOL),
        "after the pan: {:?}",
        panned.points
    );
    // Control: a per-component clamp of the point to [−1, 1], (1, 0, 1), draws z − 0.577x = 0.423.
    let n = [-0.5, 0.0, 3.0_f64.sqrt() / 2.0];
    let clamped = compass::cross_section(Cut {
        normal: n,
        offset: dot3(n, [1.0, 0.0, 1.0]),
    });
    rejects("a per-component clamp of z₀'s point", || {
        assert!(same_cycle(&clamped, &rect, TOL))
    });
    // A z₀ moved along the normal past the cube clamps the offset to ±|n|₁: at +|n|₁ the contact is the edge
    // (−1, −1, 1)–(−1, 1, 1), returned as that segment, outlined with no fill; at −|n|₁ the edge (1, ±1, −1).
    for (k, edge) in [
        (3.0, [[-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0]]),
        (-3.0, [[1.0, -1.0, -1.0], [1.0, 1.0, -1.0]]),
    ] {
        let plane = plane_at(30.0, 0.0, 0.0, 1.0, n.map(|x| x * k));
        let d = draw(&plane, Touch::Slice, &free());
        let slice = d.slice.clone().expect("the contact is drawn");
        let sign = dot3(slice.cut.normal, n).signum();
        assert!(
            (slice.cut.offset * sign - k.signum() * l1(n)).abs() <= TOL,
            "the offset {} is not clamped to ±|n|₁",
            slice.cut.offset
        );
        assert!(
            same_cycle(&slice.points, &edge, TOL),
            "the contact at {k}·n: {:?}",
            slice.points
        );
        assert!(slice.grid.is_empty(), "an edge contact has a grid");
        let record = compass::record(
            &d,
            Rect::from_min_size(egui::pos2(20.0, 30.0), egui::vec2(172.0, 172.0)),
        );
        assert_eq!(record["fill"], false, "an edge contact is filled");
    }
}

#[test]
fn qa_compass_cross_section_the_cut_near_the_corners() {
    // (90°, 89.9°), γ = 0: s = 1.75·10⁻³, drawn, the normal (−1, 0, 0) within 1e-12.
    let (q1, q2) = decoder_basis(90.0, 89.9, 0.0, 1.0);
    assert!((s_of(q1, q2) - 1.745e-3).abs() < 1e-5);
    let d = draw(
        &plane_at(90.0, 89.9, 0.0, 1.0, [0.0; 3]),
        Touch::Tilt,
        &free(),
    );
    let slice = d.slice.as_ref().expect("drawn at s = 1.75·10⁻³");
    check_normal(&slice.cut, [-1.0, 0.0, 0.0], "(90°, 89.9°)");
    assert!(slice.points.len() >= 3);
    assert!(d.items.contains(&Item::Slice));
    // (90°, 89.95°): s = 8.7·10⁻⁴, none drawn.
    let (q1, q2) = decoder_basis(90.0, 89.95, 0.0, 1.0);
    assert!((s_of(q1, q2) - 8.7e-4).abs() < 1e-5);
    let none = draw(
        &plane_at(90.0, 89.95, 0.0, 1.0, [0.0; 3]),
        Touch::Tilt,
        &free(),
    );
    assert!(none.slice.is_none(), "drawn at s = 8.7·10⁻⁴");
    assert!(!none.items.contains(&Item::Slice));
    assert_eq!(
        none.items.len(),
        12,
        "the edges alone where no slice is drawn"
    );
    rejects("a slice drawn at s = 1.75·10⁻³ read as none", || {
        assert!(d.slice.is_none())
    });
}

/// The readout's "no plane drawn" line for `(τ₁, τ₂)`, the angles signed, one decimal (R-451 B11).
fn no_plane_line(readout: &[String], t1: f64, t2: f64) -> bool {
    let sign = |v: f64| if v >= 0.0 { "+" } else { "-" };
    let want = format!(
        "no plane drawn: s < 10⁻³ at τ₁ = {}{:.1}°, τ₂ = {}{:.1}° (RQ-340)",
        sign(t1),
        t1.abs(),
        sign(t2),
        t2.abs()
    );
    readout.iter().any(|l| l.replace('−', "-") == want)
}

#[test]
fn qa_compass_cross_section_at_the_four_corners_no_slice_and_the_readout_says_so() {
    for (t1, t2) in [(90.0, 90.0), (90.0, -90.0), (-90.0, 90.0), (-90.0, -90.0)] {
        for g in [0.0, 45.0] {
            for sigma in [1.0, 0.25] {
                for mode in [Touch::Tilt, Touch::Slice] {
                    let what = format!("({t1}°, {t2}°), γ {g}°, σ {sigma}, {mode:?}");
                    let d = draw(&plane_at(t1, t2, g, sigma, [0.0, 0.0, 0.3]), mode, &free());
                    assert!(d.slice.is_none(), "{what}: a slice drawn");
                    assert!(
                        !d.items.contains(&Item::Slice),
                        "{what}: a slice in the order"
                    );
                    assert_eq!(d.items.len(), 12, "{what}: the cube's edges");
                    assert!(
                        no_plane_line(&d.readout, t1, t2),
                        "{what}: the readout {:?}",
                        d.readout
                    );
                    // The labels stay as in the mode.
                    assert_eq!(
                        d.labels.len(),
                        if mode == Touch::Slice { 3 } else { 1 },
                        "{what}: the labels"
                    );
                }
            }
        }
    }
    // A drawn tilt has no such line.
    let d = draw(
        &plane_at(90.0, 89.9, 0.0, 1.0, [0.0; 3]),
        Touch::Tilt,
        &free(),
    );
    assert!(!d.readout.iter().any(|l| l.contains("no plane drawn")));
    // Controls: the rounding normal of q₁ × q₂ at (90°, 90°) gives a plane x + y = c, which a drawing from it would
    // show; and the sine of the angle between q₁ and q₂ as the test, which at a corner with γ = 45° (σ = 1) is about
    // 0.48 and passes, where s fails.
    let (q1, q2) = decoder_basis(90.0, 90.0, 0.0, 1.0);
    let rounding = unit3(cross3(q1, q2));
    assert!(rounding.iter().all(|x| x.is_finite()));
    let from_rounding = compass::cross_section(Cut {
        normal: rounding,
        offset: 0.0,
    });
    rejects("a polygon from the rounding normal at (90°, 90°)", || {
        assert!(
            from_rounding.is_empty(),
            "a polygon drawn: {from_rounding:?}"
        )
    });
    let (q1, q2) = decoder_basis(90.0, 90.0, 45.0, 1.0);
    let sine = {
        let c = dot3(q1, q2) / (dot3(q1, q1).sqrt() * dot3(q2, q2).sqrt());
        (1.0 - c * c).max(0.0).sqrt()
    };
    assert!(s_of(q1, q2) < CUT, "s at the corner");
    let drawn_by_sine = sine >= CUT;
    rejects("the sine as the test at a corner with γ = 45°", || {
        assert!(!drawn_by_sine, "the sine {sine} would draw a plane")
    });
    rejects("a corner's line with the wrong signs", || {
        let d = draw(
            &plane_at(90.0, -90.0, 0.0, 1.0, [0.0; 3]),
            Touch::Tilt,
            &free(),
        );
        assert!(no_plane_line(&d.readout, 90.0, 90.0))
    });
}

#[test]
fn qa_compass_cross_section_locked_at_a_corner_the_pin_is_at_the_anchor() {
    let anchor_point = [0.1, 0.05, 0.2];
    let plane = plane_at(90.0, 90.0, 45.0, 1.0, [0.0; 3]);
    let lock = Lock {
        locked: true,
        z_locked: lift(anchor_point.map(|x| x * Z0_HALF_RANGE), &plane.z0),
    };
    let d = draw(&plane, Touch::Tilt, &lock);
    assert!(d.slice.is_none());
    let pin = d.pin.expect("the pin, locked at a corner");
    assert!(
        near3(pin, anchor_point, 1e-12),
        "the pin at {pin:?}, the anchor at {anchor_point:?}"
    );
    // And free, no pin.
    assert!(draw(&plane, Touch::Tilt, &free()).pin.is_none());
    // Control: a pin hidden at s < 10⁻³ while locked.
    let mut hidden = d.clone();
    hidden.pin = None;
    rejects("a pin hidden at s < 10⁻³ while locked", || {
        assert!(hidden.pin.is_some())
    });
}

// === the rig, for the drags, the easing and the drawing as painted ===================================================

fn headless() -> Headless {
    Headless::new(SIZE, PIXELS_PER_POINT)
}

fn layout() -> Layout {
    Layout::new(headless().screen(), PIXELS_PER_POINT)
}

struct Rig {
    app: App<MockSide>,
    h: Headless,
}

fn mock() -> Rig {
    let mut rig = Rig {
        app: App::new(MockSide::new(MockEngine::frozen(), None), FORMAT),
        h: headless(),
    };
    rig.idle(2);
    // The default chart is z_α × z_β (§G2): q₁ along e_a, q₂ along e_b.
    let p = rig.snap().sim.plane;
    for (q, k) in [(&p.q1, A), (&p.q2, B)] {
        assert!(
            (0..8).all(|i| i == k || q[i] == 0.0),
            "the default basis is not z_α × z_β: {p:?}"
        );
    }
    rig
}

fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
    match shape {
        egui::Shape::Vec(v) => v.iter().for_each(|s| flatten(s, out)),
        other => out.push(other.clone()),
    }
}

impl Rig {
    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.h.frame(&mut self.app, events)
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            let _ = self.frame(Vec::new());
        }
    }

    fn snap(&self) -> Snapshot {
        self.app.snapshot().clone()
    }

    /// The SetFields applied so far: the contract's log entries, less the hover's no-history cursor.
    fn applied(&self) -> usize {
        self.app
            .console()
            .iter()
            .filter(|e| {
                e.source == Source::Contract && !e.message.starts_with("SetField Hover.at ")
            })
            .count()
    }

    fn shapes(&mut self) -> Vec<egui::Shape> {
        let out = self.frame(Vec::new());
        let mut flat = Vec::new();
        for clipped in &out.shapes {
            flatten(&clipped.shape, &mut flat);
        }
        flat
    }

    fn drag(&mut self, from: Pos2, to: Pos2, steps: usize) {
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let _ = self.frame(vec![Event::PointerMoved(from)]);
        let _ = self.frame(vec![button(from, true)]);
        for i in 1..=steps {
            let at = from + (to - from) * (i as f32 / steps as f32);
            let _ = self.frame(vec![Event::PointerMoved(at)]);
        }
        let _ = self.frame(vec![button(to, false)]);
        self.idle(2);
        let away = layout().figure.center();
        let _ = self.frame(vec![Event::PointerMoved(away)]);
        self.idle(1);
    }

    /// Sets the plane through the contract, as the capture's case steps do, and lets the compass come to rest.
    fn set_plane(&mut self, plane: &Plane, mode: Touch) {
        for edit in [
            SimField::Z0(plane.z0),
            SimField::Basis {
                q1: plane.q1,
                q2: plane.q2,
            },
        ] {
            self.app.set_field(SetField {
                edit: Edit::Sim(edit),
                no_history: false,
            });
        }
        self.app.manifold.compass.mode = mode;
        self.settle();
    }

    fn settle(&mut self) {
        self.idle(3);
        for _ in 0..400 {
            if !self.app.manifold.compass.moving {
                break;
            }
            self.idle(1);
        }
        assert!(
            !self.app.manifold.compass.moving,
            "the compass did not settle"
        );
    }

    /// The cube's square in the compass.
    fn square(&self) -> Rect {
        compass::square(layout().compass)
    }
}

/// The default plane's scale and the rest of its z₀.
fn base(rig: &Rig) -> (f64, Z) {
    let p = rig.snap().sim.plane;
    (p.q1[A].abs(), p.z0)
}

/// The plane at `τ₁`, `τ₂`, `γ` with z₀'s point `p`, on the rig's default scale and the rest of its z₀.
fn rig_plane(rig: &Rig, t1: f64, t2: f64, g: f64, p: P3) -> Plane {
    let (sigma, rest) = base(rig);
    let mut plane = plane_at(t1, t2, g, sigma, p);
    plane.z0 = lift(p.map(|x| x * Z0_HALF_RANGE), &rest);
    plane
}

// --- the cube as painted --------------------------------------------------------------------------------------------

/// The isometric projection of R-451, fitted to `square`.
struct Iso {
    cx: f64,
    cy: f64,
    s: f64,
}

impl Iso {
    fn fit(square: Rect) -> Self {
        let (w, h) = (f64::from(square.width()), f64::from(square.height()));
        Self {
            cx: f64::from(square.center().x),
            cy: f64::from(square.center().y),
            s: (0.78 * w / (2.0 * 3.0_f64.sqrt())).min(0.78 * h / 4.0),
        }
    }

    fn at(&self, [x, y, z]: P3) -> [f64; 2] {
        [
            self.cx + self.s * 3.0_f64.sqrt() / 2.0 * (x - z),
            self.cy + self.s * (0.5 * (x + z) - y),
        ]
    }
}

/// The yaw-and-pitch orthographic projection the compass had before R-451 (yaw −35°, pitch 25°), for the control.
fn yaw_pitch(iso: &Iso, [x, y, z]: P3) -> [f64; 2] {
    let (yaw, pitch) = ((-35.0_f64).to_radians(), 25.0_f64.to_radians());
    let (xr, zr) = (x * yaw.cos() - z * yaw.sin(), x * yaw.sin() + z * yaw.cos());
    let yr = y * pitch.cos() - zr * pitch.sin();
    [iso.cx + iso.s * xr, iso.cy - iso.s * yr]
}

fn near2(a: [f64; 2], b: Pos2, tol: f64) -> bool {
    (a[0] - f64::from(b.x)).abs() <= tol && (a[1] - f64::from(b.y)).abs() <= tol
}

/// An edge's width by B6's law, one of five.
fn edge_width(w: f32) -> bool {
    [0.8_f32, 1.05, 1.3, 1.55, 1.8]
        .iter()
        .any(|e| (w - e).abs() < 1e-3)
}

/// The painted edges in the compass, in paint order: ends, width and alpha.
fn painted_edges(shapes: &[egui::Shape]) -> Vec<([Pos2; 2], f32, u8)> {
    let compass = layout().compass;
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::LineSegment { points, stroke }
                if edge_width(stroke.width)
                    && compass.contains(points[0])
                    && compass.contains(points[1]) =>
            {
                Some((*points, stroke.width, stroke.color.a()))
            }
            _ => None,
        })
        .collect()
}

/// REQ-GUI-191's edge checks against the projection `project`: each of the twelve edges painted once at its corners'
/// projections (within 0.02 points), in order of their middles' `x + z`, each `0.8 + 1.0·near` wide and its opacity
/// `0.28 + 0.55·near` (the alpha, kept to 8 bits, proportional to it). Returns each painted edge's key.
fn check_edges(painted: &[([Pos2; 2], f32, u8)], project: &dyn Fn(P3) -> [f64; 2]) -> Vec<f64> {
    assert_eq!(painted.len(), 12, "the twelve edges: {painted:?}");
    let mut keys = Vec::new();
    let mut seen = Vec::new();
    for (ends, width, alpha) in painted {
        let found = cube_edges().into_iter().position(|(a, b)| {
            (near2(project(a), ends[0], 0.02) && near2(project(b), ends[1], 0.02))
                || (near2(project(b), ends[0], 0.02) && near2(project(a), ends[1], 0.02))
        });
        let i = found.unwrap_or_else(|| panic!("the painted edge {ends:?} is no edge of the cube"));
        assert!(!seen.contains(&i), "an edge painted twice");
        seen.push(i);
        let (a, b) = cube_edges()[i];
        let key = (a[0] + a[2] + b[0] + b[2]) / 2.0;
        let near = (key + 2.0) / 4.0;
        assert!(
            (f64::from(*width) - (0.8 + near)).abs() < 1e-4,
            "the edge {a:?}–{b:?} is {width} wide, not {}",
            0.8 + near
        );
        let per = f64::from(*alpha) / (0.28 + 0.55 * near);
        keys.push((key, per));
    }
    // The alpha over the opacity law is one value for every edge, within the 8 bits.
    let pers: Vec<f64> = keys.iter().map(|k| k.1).collect();
    let (lo, hi) = pers
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, h), p| (l.min(*p), h.max(*p)));
    assert!(
        hi - lo <= 2.0 / 0.28,
        "the edges' alpha is not proportional to 0.28 + 0.55·near: {pers:?}"
    );
    let keys: Vec<f64> = keys.iter().map(|k| k.0).collect();
    assert!(
        keys.windows(2).all(|w| w[0] <= w[1]),
        "the edges are not painted in order of x + z: {keys:?}"
    );
    keys
}

/// The filled polygon painted in the compass, if any.
fn painted_fill(shapes: &[egui::Shape]) -> Option<Vec<Pos2>> {
    let compass = layout().compass;
    shapes.iter().find_map(|s| match s {
        egui::Shape::Path(p)
            if p.closed && p.fill.a() > 0 && p.points.iter().all(|q| compass.contains(*q)) =>
        {
            Some(p.points.clone())
        }
        _ => None,
    })
}

/// Any path painted in the compass: a fill or an outline.
fn painted_paths(shapes: &[egui::Shape]) -> usize {
    let compass = layout().compass;
    shapes
        .iter()
        .filter(|s| match s {
            egui::Shape::Path(p) => p.points.iter().all(|q| compass.contains(*q)),
            _ => false,
        })
        .count()
}

/// The texts painted in the compass, with their centres.
fn painted_texts(shapes: &[egui::Shape]) -> Vec<(String, Pos2)> {
    let compass = layout().compass;
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) => {
                let rect = t.galley.rect.translate(t.pos.to_vec2());
                compass
                    .contains(rect.center())
                    .then(|| (t.galley.text().to_owned(), rect.center()))
            }
            _ => None,
        })
        .collect()
}

/// Where an axis name on `edge` goes: centred on it, pushed 20 points outward from the cube's centre.
fn label_at(iso: &Iso, edge: (P3, P3)) -> [f64; 2] {
    let (a, b) = (iso.at(edge.0), iso.at(edge.1));
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = dx.hypot(dy);
    let m = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    let (mut px, mut py) = (-dy / len, dx / len);
    if (m[0] - iso.cx) * px + (m[1] - iso.cy) * py < 0.0 {
        (px, py) = (-px, -py);
    }
    [m[0] + 20.0 * px, m[1] + 20.0 * py]
}

/// A corner number's place: 12 points straight out from the cube's centre.
fn number_at(iso: &Iso, c: P3) -> [f64; 2] {
    let p = iso.at(c);
    let (ox, oy) = (p[0] - iso.cx, p[1] - iso.cy);
    let len = ox.hypot(oy);
    [p[0] + 12.0 * ox / len, p[1] + 12.0 * oy / len]
}

const WIDTH_EDGE: (P3, P3) = ([-1.0, -1.0, 1.0], [1.0, -1.0, 1.0]);
const DEPTH_EDGE: (P3, P3) = ([1.0, -1.0, -1.0], [1.0, -1.0, 1.0]);
const HEIGHT_EDGE: (P3, P3) = ([-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0]);

/// The label `text` is painted on `edge` (within 1 point) and on no other edge's place.
fn check_label_on(texts: &[(String, Pos2)], iso: &Iso, text: &str, edge: (P3, P3)) {
    let at = texts
        .iter()
        .find(|(t, _)| t == text)
        .unwrap_or_else(|| panic!("no label {text:?} painted: {texts:?}"))
        .1;
    let nearest = cube_edges()
        .into_iter()
        .min_by(|a, b| {
            let d = |e| {
                let p = label_at(iso, e);
                (p[0] - f64::from(at.x)).hypot(p[1] - f64::from(at.y))
            };
            d(*a).total_cmp(&d(*b))
        })
        .unwrap();
    let same = |e: (P3, P3)| (e.0 == edge.0 && e.1 == edge.1) || (e.0 == edge.1 && e.1 == edge.0);
    assert!(
        same(nearest),
        "the label {text:?} at {at:?} is on the edge {nearest:?}, not {edge:?}"
    );
    assert!(
        near2(label_at(iso, edge), at, 1.0),
        "the label {text:?} at {at:?}, not at {:?}",
        label_at(iso, edge)
    );
}

/// The cube case `plane` in `mode`, painted: the shapes and the projection.
fn painted_case(t1: f64, t2: f64, depth: f64, mode: Touch) -> (Rig, Vec<egui::Shape>, Iso) {
    let mut rig = mock();
    let plane = rig_plane(&rig, t1, t2, 0.0, [0.0, 0.0, depth]);
    rig.set_plane(&plane, mode);
    let shapes = rig.shapes();
    let iso = Iso::fit(rig.square());
    (rig, shapes, iso)
}

// === REQ-GUI-191: B8's four cases, as painted =========================================================================

#[test]
fn qa_mock_compass_cube_cases_are_painted_as_the_reference_draws_its_cube() {
    struct Case {
        name: &'static str,
        t: (f64, f64),
        depth: f64,
        mode: Touch,
        polygon: Vec<P3>,
    }
    let t = 30.0_f64.to_radians().tan();
    let cases = [
        Case {
            name: "mock_compass_cube_slice",
            t: (0.0, 0.0),
            depth: 0.25,
            mode: Touch::Slice,
            polygon: vec![
                [-1.0, -1.0, 0.25],
                [1.0, -1.0, 0.25],
                [1.0, 1.0, 0.25],
                [-1.0, 1.0, 0.25],
            ],
        },
        Case {
            name: "mock_compass_cube_tilt_0",
            t: (0.0, 0.0),
            depth: 0.0,
            mode: Touch::Tilt,
            polygon: vec![
                [-1.0, -1.0, 0.0],
                [1.0, -1.0, 0.0],
                [1.0, 1.0, 0.0],
                [-1.0, 1.0, 0.0],
            ],
        },
        Case {
            name: "mock_compass_cube_tilt_rect",
            t: (30.0, 0.0),
            depth: 0.0,
            mode: Touch::Tilt,
            polygon: vec![
                [1.0, 1.0, t],
                [1.0, -1.0, t],
                [-1.0, -1.0, -t],
                [-1.0, 1.0, -t],
            ],
        },
        Case {
            name: "mock_compass_cube_tilt_hex",
            t: (45.0, 45.0),
            depth: 0.0,
            mode: Touch::Tilt,
            polygon: vec![
                [1.0, -1.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [-1.0, 1.0, 0.0],
                [-1.0, 0.0, -1.0],
                [0.0, -1.0, -1.0],
            ],
        },
    ];
    for case in &cases {
        let (_rig, shapes, iso) = painted_case(case.t.0, case.t.1, case.depth, case.mode);
        let edges = painted_edges(&shapes);
        check_edges(&edges, &|p| iso.at(p));
        // The polygon, at its vertices' projections, as a cycle within 0.02 points.
        let fill = painted_fill(&shapes).unwrap_or_else(|| panic!("{}: no fill", case.name));
        let want: Vec<P3> = case
            .polygon
            .iter()
            .map(|p| {
                let s = iso.at(*p);
                [s[0], s[1], 0.0]
            })
            .collect();
        let got: Vec<P3> = fill
            .iter()
            .map(|p| [f64::from(p.x), f64::from(p.y), 0.0])
            .collect();
        assert!(
            same_cycle(&got, &want, 0.02),
            "{}: the fill {got:?}, not {want:?}",
            case.name
        );
        // The labels.
        let texts = painted_texts(&shapes);
        match case.mode {
            Touch::Slice => {
                check_label_on(&texts, &iso, WIDTH_NAME, WIDTH_EDGE);
                check_label_on(&texts, &iso, DEPTH_NAME, DEPTH_EDGE);
                check_label_on(&texts, &iso, HEIGHT_NAME, HEIGHT_EDGE);
                // One number per corner: −1.0 and 1.0 at the ends of Width, Depth and Height.
                let numbers: Vec<&(String, Pos2)> = texts
                    .iter()
                    .filter(|(t, _)| t == "−1.0" || t == "1.0" || t == "-1.0")
                    .collect();
                assert_eq!(
                    numbers.len(),
                    4,
                    "{}: the corner numbers {numbers:?}",
                    case.name
                );
                for (c, text) in [
                    ([-1.0, -1.0, 1.0], "1.0"),
                    ([1.0, -1.0, 1.0], "1.0"),
                    ([1.0, -1.0, -1.0], "1.0"),
                    ([-1.0, 1.0, 1.0], "1.0"),
                ] {
                    let at = number_at(&iso, c);
                    let found = numbers.iter().find(|(_, p)| near2(at, *p, 1.0));
                    let Some((got, _)) = found else {
                        panic!(
                            "{}: no corner number at {c:?} ({at:?}): {numbers:?}",
                            case.name
                        )
                    };
                    let minus = c == [-1.0, -1.0, 1.0] || c == [1.0, -1.0, -1.0];
                    assert_eq!(
                        got.trim_start_matches(['−', '-']),
                        text,
                        "{}: the number at {c:?}",
                        case.name
                    );
                    assert_eq!(
                        got.starts_with(['−', '-']),
                        minus,
                        "{}: the sign at {c:?}",
                        case.name
                    );
                }
                // The readout: the plane's normal axis and z₀'s fraction of the range.
                assert!(
                    texts
                        .iter()
                        .any(|(t, _)| t.contains(&format!("plane ⟂ {DEPTH_NAME}"))),
                    "{}: no \"plane ⟂ {DEPTH_NAME}\" in {texts:?}",
                    case.name
                );
                assert!(
                    texts
                        .iter()
                        .any(|(t, _)| t.contains(&format!("{DEPTH_NAME} = +0.25 of range"))),
                    "{}: no \"{DEPTH_NAME} = +0.25 of range\" in {texts:?}",
                    case.name
                );
            }
            Touch::Tilt => {
                let label = format!("q₁, q₂ → {DEPTH_NAME}");
                check_label_on(&texts, &iso, &label, DEPTH_EDGE);
                for name in [WIDTH_NAME, HEIGHT_NAME] {
                    assert!(
                        !texts.iter().any(|(t, _)| t == name),
                        "{}: tilt mode names {name}",
                        case.name
                    );
                }
                assert!(
                    !texts.iter().any(|(t, _)| t == "1.0" || t == "−1.0"),
                    "{}: tilt mode has corner numbers",
                    case.name
                );
                // Controls: the one label on the bottom-left edge (the reference's q₁ edge), or on any other edge.
                for edge in cube_edges() {
                    let same = |e: (P3, P3)| {
                        (e.0 == DEPTH_EDGE.0 && e.1 == DEPTH_EDGE.1)
                            || (e.0 == DEPTH_EDGE.1 && e.1 == DEPTH_EDGE.0)
                    };
                    if same(edge) {
                        continue;
                    }
                    rejects("the tilt label on another edge", || {
                        check_label_on(&texts, &iso, &label, edge)
                    });
                }
            }
        }
    }
    // Controls: the yaw-and-pitch projection, and a slice-mode label on a wrong edge.
    let (_rig, shapes, iso) = painted_case(0.0, 0.0, 0.25, Touch::Slice);
    let edges = painted_edges(&shapes);
    rejects("a yaw-and-pitch projection", || {
        check_edges(&edges, &|p| yaw_pitch(&iso, p));
    });
    let texts = painted_texts(&shapes);
    for (name, right) in [
        (WIDTH_NAME, WIDTH_EDGE),
        (DEPTH_NAME, DEPTH_EDGE),
        (HEIGHT_NAME, HEIGHT_EDGE),
    ] {
        for edge in cube_edges() {
            if (edge.0 == right.0 && edge.1 == right.1) || (edge.0 == right.1 && edge.1 == right.0)
            {
                continue;
            }
            rejects("a slice-mode label on a wrong edge", || {
                check_label_on(&texts, &iso, name, edge)
            });
        }
    }
    // Control: a fixed square for the hexagon's case.
    let (_rig, shapes, iso) = painted_case(45.0, 45.0, 0.0, Touch::Tilt);
    let fill = painted_fill(&shapes).unwrap();
    rejects("a fixed-square slice in the hexagon's case", || {
        let square: Vec<P3> = [
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ]
        .iter()
        .map(|p| {
            let s = iso.at(*p);
            [s[0], s[1], 0.0]
        })
        .collect();
        let got: Vec<P3> = fill
            .iter()
            .map(|p| [f64::from(p.x), f64::from(p.y), 0.0])
            .collect();
        assert!(same_cycle(&got, &square, 0.02));
    });
}

#[test]
fn qa_mock_compass_cube_the_slice_sits_among_the_edges_by_its_depth() {
    // The slice is painted after every edge whose key is below its own mean x + z and before every edge whose key is
    // above it (R-451; render_gui_spec: edges in front of it draw over it).
    for (t1, depth) in [(0.0, 0.25), (0.0, -0.8), (30.0, 0.0), (0.0, 0.9)] {
        let (_rig, shapes, iso) = painted_case(t1, 0.0, depth, Touch::Slice);
        let compass_rect = layout().compass;
        let mut order: Vec<Option<f64>> = Vec::new();
        for s in &shapes {
            match s {
                egui::Shape::LineSegment { points, stroke }
                    if edge_width(stroke.width)
                        && compass_rect.contains(points[0])
                        && compass_rect.contains(points[1]) =>
                {
                    let i = cube_edges()
                        .into_iter()
                        .position(|(a, b)| {
                            (near2(iso.at(a), points[0], 0.02) && near2(iso.at(b), points[1], 0.02))
                                || (near2(iso.at(b), points[0], 0.02)
                                    && near2(iso.at(a), points[1], 0.02))
                        })
                        .expect("an edge");
                    let (a, b) = cube_edges()[i];
                    order.push(Some((a[0] + a[2] + b[0] + b[2]) / 2.0));
                }
                egui::Shape::Path(p)
                    if p.closed
                        && p.fill.a() > 0
                        && p.points.iter().all(|q| compass_rect.contains(*q)) =>
                {
                    order.push(None)
                }
                _ => {}
            }
        }
        let at = order
            .iter()
            .position(Option::is_none)
            .expect("the fill painted");
        let (n, c) = decoder_cut(t1, 0.0, 0.0, [0.0, 0.0, depth]);
        let poly = my_section(n, c);
        let key = poly.iter().map(|p| p[0] + p[2]).sum::<f64>() / poly.len() as f64;
        for (i, k) in order.iter().enumerate() {
            let Some(k) = k else { continue };
            if *k < key - 1e-9 {
                assert!(
                    i < at,
                    "an edge of key {k} behind the slice ({key}) painted over it"
                );
            }
            if *k > key + 1e-9 {
                assert!(
                    i > at,
                    "an edge of key {k} in front of the slice ({key}) painted under it"
                );
            }
        }
        // Control: the slice painted last, over every edge.
        rejects("the slice painted over the edges in front of it", || {
            for (i, k) in order.iter().enumerate() {
                if let Some(k) = k {
                    if *k > key + 1e-9 {
                        assert!(i > order.len());
                    }
                }
            }
        });
    }
}

#[test]
fn qa_mock_compass_cube_no_slice_painted_at_a_corner_and_the_pin_kept_when_locked() {
    // (90°, 90°), γ = 45°, locked: the cube's edges, its labels and the gold pin at the anchor, and no polygon, fill,
    // grid or outline.
    let mut rig = mock();
    let (_, rest) = base(&rig);
    let anchor_point = [0.1, 0.05, 0.2];
    let anchor = lift(anchor_point.map(|x| x * Z0_HALF_RANGE), &rest);
    rig.app.set_field(SetField {
        edit: Edit::Sim(SimField::Lock(Lock {
            locked: true,
            z_locked: anchor,
        })),
        no_history: false,
    });
    let plane = rig_plane(&rig, 90.0, 90.0, 45.0, [0.0; 3]);
    rig.set_plane(&plane, Touch::Tilt);
    assert!(rig.snap().sim.lock.locked);
    let shapes = rig.shapes();
    let iso = Iso::fit(rig.square());
    check_edges(&painted_edges(&shapes), &|p| iso.at(p));
    assert_eq!(
        painted_paths(&shapes),
        0,
        "a slice path painted at a corner"
    );
    let compass_rect = layout().compass;
    let others: Vec<&egui::Shape> = shapes
        .iter()
        .filter(|s| match s {
            egui::Shape::LineSegment { points, stroke } => {
                compass_rect.contains(points[0])
                    && !edge_width(stroke.width)
                    && stroke.color != GOLD
            }
            _ => false,
        })
        .collect();
    assert!(
        others.is_empty(),
        "a grid or an outline painted: {others:?}"
    );
    let pin = iso.at(anchor_point);
    let gold = shapes.iter().any(|s| match s {
        egui::Shape::Circle(c) => c.fill == GOLD && near2(pin, c.center, 0.05),
        _ => false,
    });
    assert!(gold, "no gold pin at the anchor's {pin:?}");
    let texts = painted_texts(&shapes);
    check_label_on(&texts, &iso, &format!("q₁, q₂ → {DEPTH_NAME}"), DEPTH_EDGE);
    assert!(
        texts
            .iter()
            .any(|(t, _)| t.contains("no plane drawn") && t.contains("(RQ-340)")),
        "the readout does not say no plane is drawn: {texts:?}"
    );
    // Control: the same checks on a drawn plane, (90°, 89.9°), fail.
    let (_rig, drawn, _) = painted_case(90.0, 89.9, 0.0, Touch::Tilt);
    rejects("a slice painted", || assert_eq!(painted_paths(&drawn), 0));
}

// === REQ-GUI-193: the readout, the drags and the easing =============================================================

#[test]
fn qa_mock_compass_cube_readouts_per_mode() {
    let rest = [0.0; 8];
    let readout = |t1: f64, t2: f64, g: f64, depth: f64, mode: Touch| -> Vec<String> {
        let mut plane = plane_at(t1, t2, g, 1.0, [0.0, 0.0, depth]);
        plane.z0 = lift([0.0, 0.0, depth * Z0_HALF_RANGE], &rest);
        draw(&plane, mode, &free()).readout
    };
    // Slice mode, untilted: the normal axis and z₀'s signed fraction of the half-range, two decimals.
    let r = readout(0.0, 0.0, 0.0, 0.25, Touch::Slice);
    assert!(
        r.iter()
            .any(|l| l.contains(&format!("plane ⟂ {DEPTH_NAME}"))),
        "{r:?}"
    );
    assert!(
        r.iter()
            .any(|l| l.contains(&format!("{DEPTH_NAME} = +0.25 of range"))),
        "{r:?}"
    );
    let r = readout(0.0, 0.0, 30.0, -0.5, Touch::Slice);
    assert!(
        r.iter()
            .any(|l| l.contains(&format!("plane ⟂ {DEPTH_NAME}"))),
        "{r:?}"
    );
    assert!(
        r.iter().any(|l| l
            .replace('−', "-")
            .contains(&format!("{DEPTH_NAME} = -0.50 of range"))),
        "{r:?}"
    );
    // Slice mode, tilted: the tilt as tilt mode reads it, not "plane ⟂".
    let r = readout(30.0, -15.0, 0.0, 0.25, Touch::Slice);
    assert!(!r.iter().any(|l| l.contains("plane ⟂")), "{r:?}");
    check_tilt_line(&r, 30.0, -15.0);
    // Tilt mode: τ₁'s and τ₂'s target, the slice axis (the slice frame's defaults), and their angles in degrees, one
    // decimal (R-457 E6 names the angles' targets "τ₁ → z_q3", where R-451 B5 read "q₁→z_q3"; TASK-M6-38's qa commit,
    // R-457 E9).
    let r = readout(30.0, -15.0, 0.0, 0.0, Touch::Tilt);
    check_tilt_line(&r, 30.0, -15.0);
    let r = readout(-72.5, 45.0, 10.0, 0.0, Touch::Tilt);
    check_tilt_line(&r, -72.5, 45.0);
    // Controls: the τ₁ and τ₂ swapped, and the untilted line read with a tilt set.
    rejects("τ₁ and τ₂ swapped", || {
        check_tilt_line(&r, 45.0, -72.5)
    });
    let flat = readout(0.0, 0.0, 0.0, 0.25, Touch::Slice);
    rejects("the untilted line with a tilt set", || {
        check_tilt_line(&flat, 30.0, -15.0)
    });
}

/// The angle written after `q` and its target `→ z_q3` in `lines`, one decimal.
fn tilt_angle(lines: &[String], q: &str) -> Option<(f64, usize)> {
    for l in lines {
        let l = l.replace('−', "-");
        let Some(i) = l.find(&format!("{q} →")) else {
            continue;
        };
        let after = l[i + q.len()..].trim_start();
        let after = after.strip_prefix('→')?.trim_start();
        let after = after.strip_prefix(DEPTH_NAME)?.trim_start();
        let num: String = after
            .chars()
            .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
            .collect();
        if !after[num.len()..].starts_with('°') {
            return None;
        }
        let decimals = num.split('.').nth(1).map_or(0, str::len);
        return num
            .trim_start_matches('+')
            .parse()
            .ok()
            .map(|v| (v, decimals));
    }
    None
}

fn check_tilt_line(lines: &[String], t1: f64, t2: f64) {
    for (q, want) in [("τ₁", t1), ("τ₂", t2)] {
        let (got, decimals) = tilt_angle(lines, q)
            .unwrap_or_else(|| panic!("no {q}→{DEPTH_NAME} angle in {lines:?}"));
        assert!((got - want).abs() < 0.051, "{q} reads {got}°, not {want}°");
        assert_eq!(decimals, 1, "{q}'s angle to one decimal");
    }
}

/// The SetFields `act` applied, with the snapshots before and after.
fn edits(rig: &mut Rig, act: impl FnOnce(&mut Rig)) -> (usize, Snapshot, Snapshot) {
    let (before, n0) = (rig.snap(), rig.applied());
    act(rig);
    (rig.applied() - n0, before, rig.snap())
}

/// τ₁ and τ₂ of a γ = 0 basis in the default frame, in degrees.
fn taus(plane: &Plane) -> (f64, f64) {
    (
        plane.q1[D].atan2(plane.q1[A]).to_degrees(),
        plane.q2[D].atan2(plane.q2[B]).to_degrees(),
    )
}

#[test]
fn qa_mock_compass_cube_slice_mode_drags() {
    // A horizontal drag of half the cube's width in slice mode: one SetField on z₀ of half the range (the half-range,
    // the range being ±1 of it) along the drawn plane's normal, the decoder's; tilted and untilted.
    for (t1, t2) in [(0.0, 0.0), (30.0, -20.0)] {
        let mut rig = mock();
        let plane = rig_plane(&rig, t1, t2, 0.0, [0.0, 0.0, 0.1]);
        rig.set_plane(&plane, Touch::Slice);
        let sq = rig.square();
        let (n, _) = decoder_cut(t1, t2, 0.0, [0.0; 3]);
        let from = sq.center();
        let (count, before, after) = edits(&mut rig, |r| {
            r.drag(from, from + egui::vec2(sq.width() / 2.0, 0.0), 6)
        });
        assert_eq!(count, 1, "τ ({t1}°, {t2}°): {count} SetFields for one drag");
        let (a, b) = (&before.sim.plane, &after.sim.plane);
        assert_eq!((a.q1, a.q2), (b.q1, b.q2), "the drag changed the basis");
        let d: Z = std::array::from_fn(|i| b.z0[i] - a.z0[i]);
        let len = d.iter().map(|x| x * x).sum::<f64>().sqrt();
        // f32 pointer positions over the square's width: 1e-5 of the move.
        assert!(
            (len - Z0_HALF_RANGE).abs() <= 1e-5 * Z0_HALF_RANGE,
            "τ ({t1}°, {t2}°): z₀ moved {len}, not half the range {Z0_HALF_RANGE}"
        );
        let normal = lift(n, &[0.0; 8]);
        let along = d.iter().zip(&normal).map(|(x, y)| x * y).sum::<f64>().abs() / len;
        assert!(
            (along - 1.0).abs() < 1e-9,
            "τ ({t1}°, {t2}°): the move {d:?} is not along the normal {n:?}"
        );
        // A vertical drag: nothing.
        let (count, _, _) = edits(&mut rig, |r| {
            r.drag(from, from + egui::vec2(0.0, -sq.height() / 3.0), 6)
        });
        assert_eq!(count, 0, "a vertical slice-mode drag edited");
        // Control: a vertical drag that moves the slice, as the half-width one did.
        rejects("a slice-mode vertical drag that moves the slice", || {
            let mut rig = mock();
            rig.set_plane(&plane, Touch::Slice);
            let (count, _, _) = edits(&mut rig, |r| {
                r.drag(
                    from,
                    from + egui::vec2(sq.width() / 3.0, -sq.height() / 3.0),
                    6,
                )
            });
            assert_eq!(count, 0);
        });
    }
    // At τ₁ = τ₂ = 90°, γ = 0 (s < 10⁻³), a horizontal drag: nothing.
    let mut rig = mock();
    let plane = rig_plane(&rig, 90.0, 90.0, 0.0, [0.0; 3]);
    rig.set_plane(&plane, Touch::Slice);
    let sq = rig.square();
    let from = sq.center();
    let (count, before, after) = edits(&mut rig, |r| {
        r.drag(from, from + egui::vec2(sq.width() / 2.0, 0.0), 6)
    });
    assert_eq!(count, 0, "a slice drag at a corner edited");
    assert_eq!(before.sim.plane, after.sim.plane);
    // Control: a slice-mode drag at τ₁ = τ₂ = 90° that moves z₀ along the rounding normal (x + y = c).
    let (q1, q2) = decoder_basis(90.0, 90.0, 0.0, 1.0);
    let rounding = lift(unit3(cross3(q1, q2)), &[0.0; 8]);
    let mut moved = after.clone();
    moved.sim.plane.z0 = std::array::from_fn(|i| before.sim.plane.z0[i] + rounding[i]);
    rejects("z₀ moved along the rounding normal at the corner", || {
        assert_eq!(before.sim.plane, moved.sim.plane)
    });
}

#[test]
fn qa_mock_compass_cube_tilt_mode_drags() {
    // A drag across the full width sets τ₁ to the clamp, +90°; a quarter of the height up raises τ₂ by 45°; one
    // SetField on the basis each.
    let mut rig = mock();
    let plane = rig_plane(&rig, 0.0, 0.0, 0.0, [0.0; 3]);
    rig.set_plane(&plane, Touch::Tilt);
    let sq = rig.square();
    let from = sq.center();
    let (count, before, after) = edits(&mut rig, |r| {
        r.drag(from, from + egui::vec2(sq.width(), 0.0), 8)
    });
    assert_eq!(count, 1, "{count} SetFields for one drag");
    assert_eq!(
        before.sim.plane.z0, after.sim.plane.z0,
        "a tilt drag moved z₀"
    );
    let (t1, t2) = taus(&after.sim.plane);
    assert!((t1 - 90.0).abs() < 1e-9, "τ₁ is {t1}°, not the clamp 90°");
    assert!(t2.abs() < 1e-9, "τ₂ moved: {t2}°");
    // Control: an unclamped drag, 180°.
    rejects("an unclamped drag", || {
        let unclamped = (0.0_f64 + 180.0).to_radians();
        let q1 = [unclamped.cos(), unclamped.sin()];
        assert!((q1[1].atan2(q1[0]).to_degrees() - 90.0).abs() < 1e-9);
    });
    let mut rig = mock();
    let plane = rig_plane(&rig, 10.0, -20.0, 0.0, [0.0; 3]);
    rig.set_plane(&plane, Touch::Tilt);
    let sq = rig.square();
    let from = sq.center();
    let (count, _, after) = edits(&mut rig, |r| {
        r.drag(from, from + egui::vec2(0.0, -sq.height() / 4.0), 8)
    });
    assert_eq!(count, 1, "{count} SetFields for one drag");
    let (t1, t2) = taus(&after.sim.plane);
    assert!((t1 - 10.0).abs() < 1e-4, "τ₁ moved: {t1}°");
    // f32 pointer positions over the square's height.
    assert!((t2 - 25.0).abs() < 1e-3, "τ₂ is {t2}°, not −20° + 45°");
    rejects("a drag down read as up", || {
        assert!((t2 - (-65.0)).abs() < 1e-3)
    });
}

#[test]
fn qa_mock_compass_cube_eases_in_the_drawing_only() {
    for (step, dt) in [(1.0 / 60.0, 1.0 / 60.0), (0.2, 0.05)] {
        let mut rig = mock();
        rig.h.set_frame_step(step);
        let start = rig_plane(&rig, 0.0, 0.0, 0.0, [0.0; 3]);
        rig.set_plane(&start, Touch::Tilt);
        let target = rig_plane(&rig, 40.0, -30.0, 0.0, [0.0, 0.0, 0.5]);
        rig.set_plane_now(&target);
        // The contract holds the target from the first frame it is read.
        for _ in 0..10 {
            rig.idle(1);
            if rig.snap().sim.plane == target {
                break;
            }
        }
        assert_eq!(
            rig.snap().sim.plane,
            target,
            "the contract does not hold the target"
        );
        let k = 1.0 - 0.0016_f64.powf(dt);
        let mut last = rig.shown();
        let mut checked = 0;
        for _ in 0..12 {
            rig.idle(1);
            assert_eq!(
                rig.snap().sim.plane,
                target,
                "an eased value sent through the contract"
            );
            let now = rig.shown();
            for (i, (a, b, w)) in [
                (last.0, now.0, 40.0),
                (last.1, now.1, -30.0),
                (last.2, now.2, 0.5),
            ]
            .into_iter()
            .enumerate()
            {
                if (w - a).abs() < 1e-6 {
                    continue;
                }
                let want = a + (w - a) * k;
                assert!(
                    (b - want).abs() <= 1e-6 * (w - a).abs(),
                    "dt {dt}: value {i} went {a} → {b}, the law gives {want}"
                );
                checked += 1;
            }
            last = now;
        }
        assert!(
            checked > 10,
            "dt {dt}: the drawing did not ease ({checked} steps)"
        );
        // Control: an eased value sent through the contract is not the target.
        let eased = 0.0 + (40.0 - 0.0) * k;
        rejects("an eased value sent through the contract", || {
            assert!((eased - 40.0_f64).abs() < 1e-9)
        });
    }
}

impl Rig {
    /// Sets the plane through the contract without waiting for the compass.
    fn set_plane_now(&mut self, plane: &Plane) {
        for edit in [
            SimField::Z0(plane.z0),
            SimField::Basis {
                q1: plane.q1,
                q2: plane.q2,
            },
        ] {
            self.app.set_field(SetField {
                edit: Edit::Sim(edit),
                no_history: false,
            });
        }
    }

    /// What the compass draws now: τ₁, τ₂ in degrees from its slice's normal, and z₀'s point on the depth axis.
    fn shown(&self) -> (f64, f64, f64) {
        let d = compass::current(
            layout().compass,
            &self.snap().sim,
            &self.app.manifold.compass,
        );
        let n = d.slice.expect("drawn").cut.normal;
        // The decoder's normal at γ = 0 is ∝ (−tan τ₁, −tan τ₂, 1).
        let n = n.map(|x| x / n[2]);
        let shown = self.app.manifold.compass.shown.expect("shown");
        (
            (-n[0]).atan().to_degrees(),
            (-n[1]).atan().to_degrees(),
            shown.point[2],
        )
    }
}
