//! `compass_cross_section` (REQ-GUI-192; R-451 B2, B11; render_gui_spec § "The compass: look and views (R-451,
//! R-452)"): the slice the compass draws is the true cross-section of the decoder's plane with the cube, in both modes:
//! `View::basis`'s normal in the frame `(e_a, e_b, e_d)` through `z₀`'s point, unclamped, its offset alone clamped to
//! `±|n|₁`; 3 to 6 vertices, 4 to 6 through the cube's central axis, each on the plane and in the cube, convex and
//! ordered; where `s < 10⁻³`, at and near the four corner points, no slice, the readout's line, and, locked, the pin
//! at the anchor. Each check also runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{pos2, vec2, Rect};
use engine::contract::sim_config::Lock;

use super::compass_support::{
    app_at, check_painted, compass_shapes, drawing, fills, plane, settle, view, z0_at,
};
use super::support::{frame_texts, rejects};
use crate::explore::compass::{
    self, cross_section, cube_s, cut, drawn_normal, frame_basis, lift, plane_basis, point_of, Cut,
    Drawing, Item, DRAW_CUT, P3,
};
use crate::explore::manifold_view::{add, lock_edit, Touch, View};

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
    let l = dot3(a, a).sqrt();
    a.map(|x| x / l)
}

fn near3(a: P3, b: P3, tol: f64) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() <= tol)
}

fn free() -> Lock {
    Lock {
        locked: false,
        z_locked: [0.0; 8],
    }
}

/// The compass's square in the app: 172 points.
fn square172() -> Rect {
    Rect::from_min_size(pos2(0.0, 0.0), vec2(172.0, 172.0))
}

/// The cut of the view `(τ₁, τ₂, γ)` degrees at σ through the frame's point `p`.
fn cut_at(t1: f64, t2: f64, g: f64, sigma: f64, p: P3) -> Option<Cut> {
    let v = view(t1, t2, g, sigma);
    cut(&plane(&v, p).z0, &v)
}

/// Asserts `poly` is a cross-section of the plane `c`: `min..=max` vertices, each on the plane and in the cube within
/// 1e-12, convex and ordered, no three in a line.
fn check_section(poly: &[P3], c: Cut, min: usize, max: usize, what: &str) {
    assert!(
        (min..=max).contains(&poly.len()),
        "{what}: {} vertices, not {min} to {max}: {poly:?}",
        poly.len()
    );
    for p in poly {
        assert!(
            (dot3(c.normal, *p) - c.offset).abs() <= 1e-12,
            "{what}: {p:?} off the plane"
        );
        assert!(
            p.iter().all(|x| x.abs() <= 1.0 + 1e-12),
            "{what}: {p:?} outside the cube"
        );
    }
    let (u, v) = plane_basis(c.normal);
    let flat: Vec<[f64; 2]> = poly.iter().map(|p| [dot3(*p, u), dot3(*p, v)]).collect();
    let n = flat.len();
    let turns: Vec<f64> = (0..n)
        .map(|i| {
            let (a, b, c) = (flat[i], flat[(i + 1) % n], flat[(i + 2) % n]);
            (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])
        })
        .collect();
    assert!(
        turns.iter().all(|t| *t > 1e-9) || turns.iter().all(|t| *t < -1e-9),
        "{what}: not convex and ordered: {turns:?}"
    );
}

/// The sweep's tilts: τ₁, τ₂ over [−90°, 90°] by 15°, with ±89.9° beside the corners and two off the grid.
fn tilts() -> Vec<f64> {
    let mut t: Vec<f64> = (-6..=6).map(|i| f64::from(i) * 15.0).collect();
    t.extend([89.9, -89.9, 44.0, -7.5]);
    t
}

#[test]
fn compass_cross_section_sweep() {
    let mut drawn = 0;
    for t1 in tilts() {
        for t2 in tilts() {
            for g in [0.0, 17.0, 45.0, 90.0, -135.0, 180.0] {
                for d in [-1.0, -0.999, -0.6, -0.25, 0.0, 0.3, 0.75, 0.999, 1.0] {
                    let what = format!("({t1}°, {t2}°, γ {g}°) at depth {d}");
                    let v = view(t1, t2, g, 1.0);
                    let (q1, q2) = frame_basis(&v);
                    let Some(c) = cut_at(t1, t2, g, 1.0, [0.0, 0.0, d]) else {
                        assert!(
                            cube_s(q1, q2) < DRAW_CUT,
                            "{what}: no plane drawn at s ≥ 10⁻³"
                        );
                        continue;
                    };
                    assert!(cube_s(q1, q2) >= DRAW_CUT, "{what}");
                    let poly = cross_section(c);
                    let min = if f64::abs(d) < 1.0 { 4 } else { 3 };
                    check_section(&poly, c, min, 6, &what);
                    drawn += 1;
                }
            }
        }
    }
    assert!(drawn > 10_000, "{drawn} planes swept");
    // The control: a fixed square, the old slice, on a tilted plane is not its cross-section.
    let c = cut_at(30.0, 20.0, 0.0, 1.0, [0.0, 0.0, 0.2]).expect("a plane");
    let square: Vec<P3> = [(0.7, 0.7), (-0.7, 0.7), (-0.7, -0.7), (0.7, -0.7)]
        .iter()
        .map(|(a, b)| [*a, *b, 0.2])
        .collect();
    rejects("a fixed-square polygon", || {
        check_section(&square, c, 4, 6, "the square")
    });
}

#[test]
fn compass_cross_section_untilted_is_the_rectangle_at_the_depth() {
    for d in [-0.75, 0.0, 0.25, 1.0] {
        let c = cut_at(0.0, 0.0, 0.0, 1.0, [0.3, -0.4, d]).expect("a plane");
        assert!(near3(c.normal, [0.0, 0.0, 1.0], 0.0) && c.offset == d);
        let poly = cross_section(c);
        let want = [
            [-1.0, -1.0, d],
            [1.0, -1.0, d],
            [1.0, 1.0, d],
            [-1.0, 1.0, d],
        ];
        assert_eq!(poly, want, "the rectangle at {d}");
    }
    let poly = cross_section(cut_at(0.0, 0.0, 0.0, 1.0, [0.0; 3]).expect("a plane"));
    rejects("a fixed square", || {
        assert_eq!(
            poly,
            [
                [-0.7, -0.7, 0.0],
                [0.7, -0.7, 0.0],
                [0.7, 0.7, 0.0],
                [-0.7, 0.7, 0.0]
            ]
        )
    });
}

#[test]
fn compass_cross_section_slice_mode_draws_the_tilted_plane_through_z0() {
    let v = view(30.0, 0.0, 0.0, 1.0);
    let p = plane(&v, [0.2, -0.1, 0.3]);
    let (mut app, mut h) = app_at(&p);
    app.manifold.compass.mode = Touch::Slice;
    settle(&mut h, &mut app);
    let drawn = drawing(&h, &app);
    let slice = drawn.slice.clone().expect("a slice");
    let n = unit([-0.5, 0.0, 3f64.sqrt() / 2.0]);
    assert!(near3(slice.cut.normal, n, 1e-12), "{:?}", slice.cut.normal);
    let through = dot3(n, [0.2, -0.1, 0.3]);
    assert!(
        (slice.cut.offset - through).abs() < 1e-12,
        "through z₀'s point"
    );
    check_painted(&compass_shapes(&mut h, &mut app), &drawn);
    rejects(
        "a slice-mode plane drawn untilted while τ₁ ≠ 0",
        || assert!(near3(slice.cut.normal, [0.0, 0.0, 1.0], 1e-12)),
    );
}

#[test]
fn compass_cross_section_normal_is_view_basis() {
    for t1 in tilts() {
        for t2 in tilts() {
            for (g, sigma) in [(0.0, 1.0), (30.0, 0.5), (-120.0, 2.0), (45.0, 0.01)] {
                let v = view(t1, t2, g, sigma);
                let (q1, q2) = v.basis();
                let (a, b, d) = (v.a, v.b, v.d);
                let (r1, r2) = ([q1[a], q1[b], q1[d]], [q2[a], q2[b], q2[d]]);
                let cr = cross(r1, r2);
                let s = 2.0 * dot3(cr, cr).sqrt() / (dot3(r1, r1) + dot3(r2, r2));
                if s < DRAW_CUT {
                    assert_eq!(drawn_normal(&v), None, "({t1}, {t2}, {g}): s = {s}");
                    continue;
                }
                let n = drawn_normal(&v).expect("a plane at s ≥ 10⁻³");
                assert!(
                    near3(n, unit(cr), 1e-12),
                    "({t1}, {t2}, {g}): {n:?} ≠ {:?}",
                    unit(cr)
                );
            }
        }
    }
    // The control: the reference's axis-angle normal at τ₁ = τ₂ = 30°, n = (sin m·τ₁/m, cos m, sin m·τ₂/m) in its
    // (x, y, z), y vertical, is not the decoder's.
    let (t1, t2) = (30f64.to_radians(), 30f64.to_radians());
    let m = t1.hypot(t2);
    let reference = [m.sin() * t1 / m, m.cos(), m.sin() * t2 / m];
    let n = drawn_normal(&view(30.0, 30.0, 0.0, 1.0)).expect("a plane");
    rejects("the reference's axis-angle normal", || {
        assert!(near3(n, reference, 1e-12) || near3(n, reference.map(|x| -x), 1e-12))
    });
}

#[test]
fn compass_cross_section_near_the_corners() {
    // (90°, 89.9°): s = cos 89.9° = 1.75·10⁻³, drawn, the plane span(e_b, e_d).
    let v = view(90.0, 89.9, 0.0, 1.0);
    let (q1, q2) = frame_basis(&v);
    let s = cube_s(q1, q2);
    assert!((s - 1.745e-3).abs() < 1e-6, "s = {s}");
    let n = drawn_normal(&v).expect("drawn at s ≥ 10⁻³");
    assert!(near3(n, [-1.0, 0.0, 0.0], 1e-12), "{n:?}");
    let poly = cross_section(cut_at(90.0, 89.9, 0.0, 1.0, [0.0; 3]).expect("a plane"));
    assert_eq!(poly.len(), 4, "{poly:?}");
    // (90°, 89.95°): s = 8.7·10⁻⁴, none.
    let v = view(90.0, 89.95, 0.0, 1.0);
    let (q1, q2) = frame_basis(&v);
    let s = cube_s(q1, q2);
    assert!((s - 8.7e-4).abs() < 1e-5, "s = {s}");
    assert_eq!(drawn_normal(&v), None);
    rejects("a plane drawn below the cut", || {
        assert!(drawn_normal(&view(90.0, 89.95, 0.0, 1.0)).is_some())
    });
}

/// The four corner points `(τ₁, τ₂)`.
const CORNERS: [(f64, f64); 4] = [(90.0, 90.0), (90.0, -90.0), (-90.0, 90.0), (-90.0, -90.0)];

/// Asserts the compass draws no slice and reads out the corner `(τ₁, τ₂)`.
fn check_no_slice(d: &Drawing, t1: f64, t2: f64) {
    assert!(d.slice.is_none(), "no slice at ({t1}, {t2}): {:?}", d.slice);
    assert!(!d.items.contains(&Item::Slice));
    assert_eq!(d.items.len(), 12, "the twelve edges alone");
    let line = format!(
        "no plane drawn: s < 10⁻³ at τ₁ = {}°, τ₂ = {}° (RQ-340)",
        compass::signed(t1, 1),
        compass::signed(t2, 1)
    );
    assert!(d.readout.contains(&line), "{line:?} in {:?}", d.readout);
}

#[test]
fn compass_cross_section_corners_draw_no_slice() {
    for (t1, t2) in CORNERS {
        for g in [0.0, 45.0] {
            for mode in [Touch::Tilt, Touch::Slice] {
                let v = view(t1, t2, g, 1.0);
                let d =
                    compass::drawing(square172(), &plane(&v, [0.1, 0.2, -0.3]), &v, mode, &free());
                check_no_slice(&d, t1, t2);
                assert!(!d.labels.is_empty(), "the labels kept");
            }
        }
        // In the app: nothing of a slice painted, the readout read out.
        let v = view(t1, t2, 45.0, 1.0);
        let (mut app, mut h) = app_at(&plane(&v, [0.0; 3]));
        let d = drawing(&h, &app);
        check_no_slice(&d, t1, t2);
        check_painted(&compass_shapes(&mut h, &mut app), &d);
        let texts = frame_texts(&mut h, &mut app);
        assert!(
            texts
                .iter()
                .any(|t| t.starts_with("no plane drawn: s < 10⁻³")),
            "{texts:?}"
        );
    }
    let v = view(90.0, 90.0, 45.0, 1.0);
    let d = compass::drawing(square172(), &plane(&v, [0.0; 3]), &v, Touch::Tilt, &free());
    assert!(d
        .readout
        .contains(&"no plane drawn: s < 10⁻³ at τ₁ = +90.0°, τ₂ = +90.0° (RQ-340)".to_owned()));
    // The controls: a polygon from the rounding normal of q₁ × q₂, the plane x + y = c; and the sine of the angle
    // between q₁ and q₂ as the test, which passes at a corner with γ = 45°.
    let (q1, q2) = frame_basis(&v);
    let rounding = unit(cross(q1, q2));
    let half = 0.5f64.sqrt();
    assert!(
        near3(rounding.map(f64::abs), [half, half, 0.0], 1e-6),
        "{rounding:?}"
    );
    let from_rounding = cross_section(Cut {
        normal: rounding,
        offset: 0.0,
    });
    assert_eq!(from_rounding.len(), 4, "x + y = 0 cuts the cube");
    rejects("a polygon drawn from the rounding normal", || {
        assert!(from_rounding.is_empty(), "{from_rounding:?}")
    });
    let sine_at = |sigma: f64| {
        let v = view(90.0, 90.0, 45.0, sigma);
        let (q1, q2) = frame_basis(&v);
        let (l1, l2) = (dot3(q1, q1).sqrt(), dot3(q2, q2).sqrt());
        (
            dot3(cross(q1, q2), cross(q1, q2)).sqrt() / (l1 * l2),
            cube_s(q1, q2),
        )
    };
    let sines: Vec<(f64, f64, f64)> = [0.01, 0.1, 0.5, 1.0, 2.0, 4.0]
        .into_iter()
        .map(|sigma| {
            let (sine, s) = sine_at(sigma);
            (sigma, sine, s)
        })
        .collect();
    for (sigma, sine, s) in &sines {
        assert!(
            (0.27..=1.0).contains(sine) && *s < DRAW_CUT,
            "σ {sigma}: the sine {sine}, s {s}"
        );
    }
    let at_one = sine_at(1.0).0;
    assert!(
        (at_one - 0.48).abs() < 0.01,
        "the sine at σ = 1 reads {at_one}"
    );
    rejects("the sine as the test", || {
        assert!(
            sines.iter().all(|(_, sine, _)| *sine < DRAW_CUT),
            "the sine passes the cut: {sines:?}"
        )
    });
}

#[test]
fn compass_cross_section_locked_at_a_corner_the_pin_is_at_the_anchor() {
    let v = view(90.0, 90.0, 45.0, 1.0);
    let p = plane(&v, [0.0; 3]);
    let anchor = z0_at([0.1, 0.05, 0.2]);
    let (mut app, mut h) = app_at(&p);
    app.set_field(lock_edit(Lock {
        locked: true,
        z_locked: anchor,
    }));
    let _ = h.frame(&mut app, Vec::new());
    settle(&mut h, &mut app);
    let d = drawing(&h, &app);
    assert!(d.slice.is_none(), "no slice at the corner");
    let want = point_of(&anchor, &View::of(&app.snapshot().sim.plane));
    assert!(near3(want, [0.1, 0.05, 0.2], 1e-15));
    assert_eq!(d.pin, Some(want), "the pin at the anchor");
    check_painted(&compass_shapes(&mut h, &mut app), &d);
    rejects("a pin hidden at s < 10⁻³ while locked", || {
        assert_eq!(d.pin, None)
    });
}

/// tilt_rect's rectangle, `z = x tan 30°`.
fn tilt_rect() -> Vec<P3> {
    let t = 30f64.to_radians().tan();
    vec![
        [1.0, 1.0, t],
        [1.0, -1.0, t],
        [-1.0, -1.0, -t],
        [-1.0, 1.0, -t],
    ]
}

/// Whether `got` is the cycle `want` from some start, either way round, within `tol`.
fn same_cycle(got: &[P3], want: &[P3], tol: f64) -> bool {
    let n = want.len();
    got.len() == n
        && (0..n).any(|s| {
            (0..n).all(|i| near3(got[i], want[(s + i) % n], tol))
                || (0..n).all(|i| near3(got[i], want[(s + n - i) % n], tol))
        })
}

#[test]
fn compass_cross_section_a_pan_leaves_the_plane_and_the_offset_clamps() {
    let v = view(30.0, 0.0, 0.0, 1.0);
    let flat = plane(&v, [0.0; 3]);
    // A pan of 2 half-ranges along q₁: z₀'s point (1.732, 0, 1), on the plane.
    let panned = add(&flat.z0, 2.0, &flat.q1);
    let p = point_of(&panned, &v);
    assert!(near3(p, [3f64.sqrt(), 0.0, 1.0], 1e-12), "{p:?}");
    let c = cut(&panned, &v).expect("a plane");
    let poly = cross_section(c);
    assert!(same_cycle(&poly, &tilt_rect(), 1e-12), "{poly:?}");
    // The control: each of the point's components clamped to [−1, 1], (1, 0, 1), shifts the plane to
    // z − 0.577x = 0.423.
    let clamped = Cut {
        offset: dot3(c.normal, p.map(|x| x.clamp(-1.0, 1.0))),
        ..c
    };
    let shifted = cross_section(clamped);
    let tan = 30f64.to_radians().tan();
    assert!(shifted
        .iter()
        .all(|q| (q[2] - tan * q[0] - (1.0 - tan)).abs() < 1e-12));
    rejects("a per-component clamp of z₀'s point", || {
        assert!(same_cycle(&shifted, &tilt_rect(), 1e-12))
    });
    // z₀ moved along the normal past the cube: the offset clamps to ±|n|₁.
    let reach: f64 = c.normal.iter().map(|x| x.abs()).sum();
    let normal = lift(c.normal, &v);
    for (k, want) in [(5.0, reach), (-5.0, -reach), (0.5, 0.5)] {
        let got = cut(&add(&flat.z0, k, &normal), &v).expect("a plane").offset;
        assert!((got - want).abs() < 1e-12, "{k}: {got} for {want}");
    }
    rejects("an offset left unclamped past the cube", || {
        let got = cut(&add(&flat.z0, 5.0, &normal), &v)
            .expect("a plane")
            .offset;
        assert!((got - 5.0).abs() < 1e-12)
    });
}

#[test]
fn compass_cross_section_the_triangle_at_the_top() {
    let c = cut_at(-45.0, -45.0, 0.0, 1.0, [0.0, 0.0, 1.0]).expect("a plane");
    assert!(
        near3(c.normal, unit([1.0, 1.0, 1.0]), 1e-12),
        "x + y + z = 1"
    );
    let poly = cross_section(c);
    let want = [[-1.0, 1.0, 1.0], [1.0, -1.0, 1.0], [1.0, 1.0, -1.0]];
    assert!(same_cycle(&poly, &want, 1e-12), "{poly:?}");
    check_section(&poly, c, 3, 6, "the triangle");
    rejects("4 to 6 at |d| = 1", || {
        check_section(&poly, c, 4, 6, "the triangle")
    });
}

#[test]
fn compass_cross_section_an_edge_contact_is_its_outline() {
    let v = view(30.0, 0.0, 0.0, 1.0);
    let flat = plane(&v, [0.0; 3]);
    let n = drawn_normal(&v).expect("a plane");
    let reach: f64 = n.iter().map(|x| x.abs()).sum();
    let poly = cross_section(Cut {
        normal: n,
        offset: reach,
    });
    let want = [[-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0]];
    assert!(same_cycle(&poly, &want, 0.0), "the edge: {poly:?}");
    // Drawn so in the app, z₀ moved past the cube along the normal: the outline alone, no fill.
    let past = plane(&v, point_of(&add(&flat.z0, 5.0, &lift(n, &v)), &v));
    let (mut app, mut h) = app_at(&past);
    let d = drawing(&h, &app);
    let slice = d.slice.clone().expect("the contact");
    assert!(
        same_cycle(&slice.points, &want, 1e-12),
        "{:?}",
        slice.points
    );
    assert!(slice.grid.is_empty(), "no grid");
    let shapes = compass_shapes(&mut h, &mut app);
    check_painted(&shapes, &d);
    assert!(fills(&shapes).is_empty(), "no fill");
    // A face contact is the polygon; a corner contact one point.
    let face = cross_section(Cut {
        normal: [0.0, 0.0, 1.0],
        offset: 1.0,
    });
    assert_eq!(face.len(), 4, "{face:?}");
    let corner = cross_section(Cut {
        normal: unit([1.0, 1.0, 1.0]),
        offset: unit([1.0, 1.0, 1.0]).iter().sum(),
    });
    assert_eq!(corner, [[1.0, 1.0, 1.0]], "a corner contact");
    let past_the_cube = cross_section(Cut {
        normal: [0.0, 0.0, 1.0],
        offset: 1.5,
    });
    assert!(past_the_cube.is_empty(), "past the cube, nothing");
    rejects("an edge contact drawn filled", || {
        assert!(!fills(&shapes).is_empty())
    });
}

#[test]
fn compass_cross_section_near_a_corner_and_nearly_along_an_edge() {
    // 5e-12 inside the corner (1, 1, 1), not on it: its three edges' crossings lie within 1e-11 of it and are one
    // point.
    let n = unit([1.0, 1.0, 1.0]);
    let reach: f64 = n.iter().sum();
    let near = cross_section(Cut {
        normal: n,
        offset: reach - 5e-12,
    });
    assert_eq!(near.len(), 1, "{near:?}");
    assert!(near3(near[0], [1.0, 1.0, 1.0], 1e-10), "{near:?}");
    // A plane through the top face's edge x = 1, tilted by 1.25e-12: the corners (1, ±1, 1) are on it within 1e-12,
    // the corners (−1, ±1, 1) 2e-12 below it. The contact is that edge; the top face's other edges cross nothing.
    let along = cross_section(Cut {
        normal: [1.25e-12, 0.0, 1.0],
        offset: 1.0 + 0.75e-12,
    });
    assert!(
        same_cycle(&along, &[[1.0, -1.0, 1.0], [1.0, 1.0, 1.0]], 0.0),
        "{along:?}"
    );
    rejects("three points for one corner", || assert_eq!(near.len(), 3));
}
