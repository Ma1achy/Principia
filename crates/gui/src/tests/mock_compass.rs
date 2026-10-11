//! The compass (render_gui_spec §G2, §G3, §G4, read with R-409): its fixed view, the slice plane's frame and normal,
//! its corners and hit test, its tilt and slice edits; `mock_compass_modes` (REQ-GUI-181): it opens in Tilt, its
//! buttons switch the mode and follow the slider touched, a tilt-mode drag anywhere tilts and a slice-mode drag
//! anywhere moves the plane along its drawn normal, one `SetField` per drag, and nothing turns the view; and
//! `mock_compass_keys` (REQ-GUI-183): the compass, Tilt and Slice as siblings, Enter on a button setting the mode, Enter
//! on the compass starting interaction, its arrows tilting or slicing by the mode, Shift ×10 and Alt ×0.1. Each check
//! also runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{self, pos2, Key, Modifiers, Rect, Visuals};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Lock, Plane};
use engine::contract::view_ui::{Focus, KeyMode, Mode};

use super::support::{focus, headless, mock_app, nodes, press, rejects, shapes};
use crate::app::App;
use crate::explore::compass::{
    after, corners, drag_edit, inside, latent_normal, normal, pin, plane_colour, plane_frame,
    project, slice, slice_by_drag, square, tilt, Compass, Drag, CUBE_SCALE, HINT_SLICE, HINT_TILT,
    MARGIN, MIN_NORMAL_ON_SCREEN, PINNED, PLANE_HALF, SLICE_BUTTON, TILT_BUTTON, TITLE, VIEW_ANGLE,
};
use crate::explore::lock::GOLD;
use crate::explore::manifold_view::slice_tilt::Angles;
use crate::explore::manifold_view::{
    add, basis_edit, dot, lock_edit, norm, z0_edit, Out, Scratch, Touch, View, AXES, SLICE_AXIS,
};
use crate::explore::{COMPASS, COMPASS_SLICE, COMPASS_TILT, COMPASS_VIEW};
use crate::headless::Headless;
use crate::keyboard::keymap::{Command, Direction};
use crate::keyboard::scopes::{Adjust, Base, Scope, ScopeTree, StepKind};
use crate::mock::{MOCK_Q1, MOCK_Q2, MOCK_Z0};
use crate::side::MockSide;

const SQUARE: Rect = Rect::from_min_max(pos2(100.0, 200.0), pos2(300.0, 400.0));
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;

fn close(a: egui::Pos2, b: egui::Pos2) -> bool {
    (a - b).length() < 1e-3
}

/// The projection of `(w, h, d)` in `square` seen from `(yaw, pitch)` degrees, composed by hand: yaw about H, then
/// pitch about the screen's x.
fn seen_from(angle: (f64, f64), square: Rect, [w, h, d]: [f64; 3]) -> egui::Pos2 {
    let (yaw, pitch) = (angle.0.to_radians(), angle.1.to_radians());
    let (x, depth) = (
        w * yaw.cos() + d * yaw.sin(),
        -w * yaw.sin() + d * yaw.cos(),
    );
    let y = h * pitch.cos() - depth * pitch.sin();
    let r = f64::from(square.width()) * CUBE_SCALE;
    square.center() + egui::vec2((x * r) as f32, (-y * r) as f32)
}

#[test]
fn mock_compass_projection_is_the_fixed_view() {
    for p in [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.4, -0.6, 0.9],
    ] {
        assert!(close(project(SQUARE, p), seen_from(VIEW_ANGLE, SQUARE, p)));
    }
    assert_eq!(VIEW_ANGLE, (-35.0, 25.0), "the code's default angle");
    rejects("another angle", || {
        assert!(close(
            project(SQUARE, [0.4, -0.6, 0.9]),
            seen_from((0.0, 0.0), SQUARE, [0.4, -0.6, 0.9])
        ))
    });
}

#[test]
fn mock_compass_opens_as_01_main_draws_it() {
    // 01_main.png's cube: W to the right, D towards the lower left, H up.
    let c = SQUARE.center();
    let (w, h, d) = (
        project(SQUARE, [1.0, 0.0, 0.0]),
        project(SQUARE, [0.0, 1.0, 0.0]),
        project(SQUARE, [0.0, 0.0, 1.0]),
    );
    assert!(w.x > c.x, "W to the right: {w:?}");
    assert!(d.x < c.x && d.y > c.y, "D to the lower left: {d:?}");
    assert!(h.y < c.y, "H up: {h:?}");
    let mirrored = seen_from((-VIEW_ANGLE.0, VIEW_ANGLE.1), SQUARE, [0.0, 0.0, 1.0]);
    rejects("the view mirrored", || assert!(mirrored.x < c.x));
    // The plane's colour is the theme's selection blue, drawn.
    let visuals = Visuals::dark();
    assert_eq!(plane_colour(&visuals), visuals.selection.stroke.color);
    assert_ne!(plane_colour(&visuals), egui::Color32::TRANSPARENT);
}

fn plane(z_d: f64) -> Plane {
    let mut z0 = MOCK_Z0;
    z0[SLICE_AXIS] = z_d;
    Plane {
        z0,
        q1: MOCK_Q1,
        q2: MOCK_Q2,
    }
}

fn near(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12)
}

#[test]
fn mock_compass_plane_frame() {
    let p = plane(0.25);
    let view = View::of(&p);
    let (c, r1, r2) = plane_frame(&p, &view, true);
    assert!(near(c, [0.0, 0.0, 0.25]) && near(r1, [1.0, 0.0, 0.0]) && near(r2, [0.0, 1.0, 0.0]));
    // The slice step past the cube stays on its face.
    assert!(near(
        plane_frame(&plane(1.5), &view, true).0,
        [0.0, 0.0, 1.0]
    ));
    assert!(near(
        plane_frame(&plane(-3.0), &view, true).0,
        [0.0, 0.0, -1.0]
    ));
    // Tilts lean the directions into the depth; the rotation turns them in the plane.
    let (s30, c30) = 30f64.to_radians().sin_cos();
    let turned = View {
        tau1: 30f64.to_radians(),
        tau2: -30f64.to_radians(),
        gamma: 0.0,
        ..view
    };
    let (_, r1, r2) = plane_frame(&p, &turned, true);
    assert!(
        near(r1, [c30, 0.0, s30]) && near(r2, [0.0, c30, -s30]),
        "{r1:?} {r2:?}"
    );
    // Untilted, the same view's plane lies flat.
    let (_, r1, r2) = plane_frame(&p, &turned, false);
    assert!(near(r1, [1.0, 0.0, 0.0]) && near(r2, [0.0, 1.0, 0.0]));
    let rotated = View {
        gamma: 90f64.to_radians(),
        ..turned
    };
    let (_, r1, r2) = plane_frame(&p, &rotated, true);
    assert!(
        near(r1, [0.0, c30, -s30]) && near(r2, [-c30, 0.0, -s30]),
        "{r1:?} {r2:?}"
    );
    rejects("the plane untilted when asked tilted", || {
        let (_, r1, _) = plane_frame(&p, &turned, true);
        assert!(near(r1, [1.0, 0.0, 0.0]));
    });
}

#[test]
fn mock_compass_corners_and_hits() {
    let frame = ([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let poly = corners(SQUARE, frame);
    let k = PLANE_HALF;
    let want = [(k, k), (-k, k), (-k, -k), (k, -k)].map(|(a, b)| project(SQUARE, [a, b, 0.0]));
    assert_eq!(poly.len(), 4);
    for (got, want) in poly.iter().zip(want) {
        assert!(close(*got, want), "{got:?} for {want:?}");
    }
    // Off the centre: shifted by the centre's projection.
    let shifted = corners(SQUARE, ([0.5, 0.0, 0.0], frame.1, frame.2));
    assert!(close(shifted[0], project(SQUARE, [0.5 + k, k, 0.0])));
    let c = project(SQUARE, [0.0; 3]);
    assert!(inside(&poly, c), "the centre is on the plane");
    assert!(inside(&poly, want[0]), "a corner is on it");
    let off = SQUARE.min + egui::vec2(1.0, 1.0);
    assert!(!inside(&poly, off), "the square's corner is off it");
    let reversed: Vec<_> = poly.iter().rev().copied().collect();
    assert!(inside(&reversed, c), "either winding");
    assert!(!inside(&reversed, off));
    let triangle = [c, c + egui::vec2(10.0, 0.0), c + egui::vec2(0.0, 10.0)];
    assert!(
        !inside(&triangle, c + egui::vec2(6.0, 6.0)),
        "past the hypotenuse is off"
    );
    assert!(inside(&triangle, c + egui::vec2(4.0, 4.0)));
    rejects("a point off the plane", || assert!(inside(&poly, off)));
}

/// `τ₁`, `τ₂` in degrees of a basis edit made on `p`.
fn angles_of(p: &Plane, edit: &SetField) -> (f64, f64) {
    match &edit.edit {
        Edit::Sim(SimField::Basis { q1, q2 }) => {
            let v = View::of(&Plane {
                z0: p.z0,
                q1: *q1,
                q2: *q2,
            });
            (v.tau1.to_degrees(), v.tau2.to_degrees())
        }
        other => panic!("a basis edit, not {other:?}"),
    }
}

#[test]
fn mock_compass_tilt_edit() {
    let p = plane(0.0);
    let (t1, t2) = angles_of(&p, &tilt(&p, 10.0, -5.0));
    assert!(
        (t1 - 10.0).abs() < 1e-9 && (t2 + 5.0).abs() < 1e-9,
        "({t1}, {t2})"
    );
    let (t1, t2) = angles_of(&p, &tilt(&p, 120.0, -120.0));
    assert!(
        (t1 - 90.0).abs() < 1e-9 && (t2 + 90.0).abs() < 1e-9,
        "clamped: ({t1}, {t2})"
    );
    rejects("a tilt the other way", || {
        let (t1, _) = angles_of(&p, &tilt(&p, 10.0, 0.0));
        assert!((t1 + 10.0).abs() < 1e-9);
    });
}

// --- the normal and the slice edit (REQ-GUI-181; R-409 A3) -----------------------------------------------------------

/// A plane tilted by `τ₁`, `τ₂` and turned by `γ`, in degrees, about the mock's centre with the slice step at 0.2.
fn tilted(t1: f64, t2: f64, g: f64) -> Plane {
    let flat = plane(0.2);
    let (q1, q2) = Angles {
        tau1: t1,
        tau2: t2,
        gamma: g,
    }
    .apply(View::of(&flat))
    .basis();
    Plane {
        z0: flat.z0,
        q1,
        q2,
    }
}

/// The drawn plane's in-plane directions, in the latent space.
fn in_plane_directions(p: &Plane) -> (Latent, Latent) {
    let view = View::of(p);
    let (_, r1, r2) = plane_frame(p, &view, true);
    let lift = |r: [f64; 3]| {
        let mut v = [0.0; 8];
        v[view.a] = r[0];
        v[view.b] = r[1];
        v[view.d] = r[2];
        v
    };
    (lift(r1), lift(r2))
}

/// Asserts that `edit` on `p` sets `z₀` and moves it along the drawn plane's normal, its components along the plane's
/// in-plane directions zero within 1e-12; returns how far, signed.
fn along_normal(p: &Plane, edit: &SetField) -> f64 {
    let Edit::Sim(SimField::Z0(z0)) = &edit.edit else {
        panic!("a SetField on z₀, not {edit:?}")
    };
    let delta = add(z0, -1.0, &p.z0);
    let (d1, d2) = in_plane_directions(p);
    assert!(
        dot(&delta, &d1).abs() <= 1e-12 && dot(&delta, &d2).abs() <= 1e-12,
        "Δz₀ {delta:?} has a part in the plane"
    );
    dot(&delta, &latent_normal(p))
}

/// Asserts [`along_normal`], by `amount` within `tol`.
fn check_along_normal(p: &Plane, edit: &SetField, amount: f64, tol: f64) {
    let got = along_normal(p, edit);
    assert!(
        (got - amount).abs() <= tol,
        "Δz₀ along the normal: {got} for {amount}"
    );
}

#[test]
fn mock_compass_modes_the_normal_is_the_drawn_planes() {
    // Untilted, the normal is the hidden slice direction: the drag moves the slice step.
    let n = latent_normal(&plane(0.2));
    let mut e_d = [0.0; 8];
    e_d[SLICE_AXIS] = 1.0;
    assert!(
        n.iter().zip(e_d).all(|(a, b)| (a - b).abs() < 1e-15),
        "{n:?}"
    );
    for (t1, t2, g) in [
        (30.0, -20.0, 25.0),
        (60.0, 45.0, -170.0),
        (-89.0, 0.0, 90.0),
    ] {
        let p = tilted(t1, t2, g);
        let frame = plane_frame(&p, &View::of(&p), true);
        let n = normal(frame);
        let dot3 = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        assert!((dot3(n, n) - 1.0).abs() < 1e-12, "unit");
        assert!(dot3(n, frame.1).abs() < 1e-12 && dot3(n, frame.2).abs() < 1e-12);
        // The drawn plane's normal is the basis's own: q₁ and q₂ have no part along it.
        let ln = latent_normal(&p);
        assert!(dot(&ln, &p.q1).abs() < 1e-12 && dot(&ln, &p.q2).abs() < 1e-12);
        check_along_normal(&p, &slice(&p, 0.3), 0.3, 1e-12);
        check_along_normal(&p, &slice(&p, -0.05), -0.05, 1e-12);
        rejects("a slice along the depth axis on a tilted plane", || {
            let mut z0 = p.z0;
            z0[SLICE_AXIS] += 0.3;
            along_normal(&p, &z0_edit(z0));
        });
        rejects("a slice-mode edit that tilts", || {
            along_normal(&p, &tilt(&p, 5.0, 0.0));
        });
    }
}

#[test]
fn mock_compass_modes_slice_by_drag_follows_the_pointer_along_the_normal() {
    let p = tilted(20.0, -10.0, 15.0);
    let n = normal(plane_frame(&p, &View::of(&p), true));
    let along = project(SQUARE, n) - project(SQUARE, [0.0; 3]);
    // A drag the normal's own length on the screen moves the plane one unit; across it, nothing.
    assert!((slice_by_drag(&p, SQUARE, along) - 1.0).abs() < 1e-6);
    assert!((slice_by_drag(&p, SQUARE, -0.5 * along) + 0.5).abs() < 1e-6);
    let across = egui::vec2(-along.y, along.x);
    assert!(slice_by_drag(&p, SQUARE, across).abs() < 1e-6);
    rejects(
        "a drag along the normal that moves the plane nowhere",
        || assert!(slice_by_drag(&p, SQUARE, along).abs() < 1e-6),
    );
    // A normal pointing at the viewer is (−sin yaw, tan pitch, cos yaw), at τ₁ −35°, τ₂ ≈ −29.6°. Ten degrees of τ₂
    // off it, the normal's unit on the screen is about 0.6 of the floor: shorter than the floor, which a drag then
    // divides by, and long enough that the floored and unfloored moves differ well past rounding.
    let facing = tilted(
        -35.0,
        10.0 - (25f64.to_radians().tan() / 35f64.to_radians().cos())
            .atan()
            .to_degrees(),
        0.0,
    );
    let n = normal(plane_frame(&facing, &View::of(&facing), true));
    let along = project(SQUARE, n) - project(SQUARE, [0.0; 3]);
    let floor = f64::from(SQUARE.width()) * CUBE_SCALE * MIN_NORMAL_ON_SCREEN;
    let length = f64::from(along.length());
    assert!(0.5 * floor < length && length < floor, "{along:?}");
    let travel = egui::vec2(10.0, -4.0);
    let want = f64::from(travel.dot(along)) / (floor * floor);
    assert!(want.abs() > 0.1, "{want}");
    assert!((slice_by_drag(&facing, SQUARE, travel) / want - 1.0).abs() < 1e-6);
    rejects("no floor under a normal at the viewer", || {
        let unfloored = f64::from(travel.dot(along)) / f64::from(along.length_sq());
        assert!((slice_by_drag(&facing, SQUARE, travel) / unfloored - 1.0).abs() < 1e-6)
    });
}

#[test]
fn mock_compass_modes_a_drag_is_one_edit_by_its_mode() {
    let p = tilted(20.0, -10.0, 15.0);
    let tilt_drag = Drag {
        mode: Touch::Tilt,
        travel: egui::vec2(20.0, 10.0),
    };
    let (t1, t2) = angles_of(&p, &drag_edit(&p, &tilt_drag, SQUARE).expect("an edit"));
    let before = Angles::of(&View::of(&p));
    assert!((t1 - before.tau1 - 10.0).abs() < 1e-9 && (t2 - before.tau2 + 5.0).abs() < 1e-9);
    let slice_drag = Drag {
        mode: Touch::Slice,
        travel: egui::vec2(-6.0, 9.0),
    };
    let edit = drag_edit(&p, &slice_drag, SQUARE).expect("an edit");
    check_along_normal(
        &p,
        &edit,
        slice_by_drag(&p, SQUARE, slice_drag.travel),
        1e-12,
    );
    for mode in [Touch::Tilt, Touch::Slice] {
        let still = Drag {
            mode,
            travel: egui::Vec2::ZERO,
        };
        assert_eq!(drag_edit(&p, &still, SQUARE), None, "no travel, no edit");
    }
    rejects("a slice-mode drag that tilts", || {
        along_normal(&p, &drag_edit(&p, &tilt_drag, SQUARE).expect("an edit"));
    });
    // The plane after the edits, each on what the earlier leave; a lock edit moves no plane.
    let Edit::Sim(SimField::Z0(z0)) = edit.edit else {
        unreachable!("a slice is on z₀")
    };
    let moved = after(&p, &[edit.clone(), basis_edit((MOCK_Q1, MOCK_Q2))]);
    assert_eq!((moved.z0, moved.q1, moved.q2), (z0, MOCK_Q1, MOCK_Q2));
    assert_eq!(after(&p, &[lock_edit(free_lock())]), p);
}

fn free_lock() -> Lock {
    Lock {
        locked: false,
        z_locked: [0.0; 8],
    }
}

// --- the compass as the app draws it ----------------------------------------------------------------------------------

fn compass_rect(h: &Headless) -> Rect {
    crate::layout::Layout::new(h.screen(), crate::capture::PIXELS_PER_POINT).compass
}

/// The text shapes drawn: each text with its bounds.
fn texts(shapes: &[egui::Shape]) -> Vec<(String, Rect)> {
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) => Some((t.galley.text().to_owned(), s.visual_bounding_rect())),
            _ => None,
        })
        .collect()
}

#[test]
fn mock_compass_square_sits_in_the_margin() {
    let rect = Rect::from_min_size(pos2(10.0, 20.0), egui::vec2(400.0, 200.0));
    assert_eq!(
        square(rect),
        Rect::from_min_size(pos2(22.0, 32.0), egui::vec2(176.0, 176.0)),
        "as tall as the margin leaves"
    );
    let narrow = Rect::from_min_size(pos2(10.0, 20.0), egui::vec2(200.0, 400.0));
    assert_eq!(
        square(narrow),
        Rect::from_min_size(pos2(22.0, 32.0), egui::vec2(100.0, 100.0)),
        "at most half the width"
    );
    rejects("a square without its margin", || {
        assert_eq!(square(rect).min, rect.min)
    });
}

/// The line segments drawn.
fn segments(shapes: &[egui::Shape]) -> Vec<[egui::Pos2; 2]> {
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::LineSegment { points, .. } => Some(*points),
            _ => None,
        })
        .collect()
}

/// Asserts the cube's twelve edges as seen from `angle`, in `sq`, and no more.
fn check_cube(shapes: &[egui::Shape], sq: Rect, angle: (f64, f64)) {
    let close = |a: egui::Pos2, b: egui::Pos2| (a - b).length() < 1e-2;
    let drawn = segments(shapes);
    let corner = |w: f64, h: f64, d: f64| seen_from(angle, sq, [w, h, d]);
    let mut edges = 0;
    for w in [-1.0, 1.0] {
        for hh in [-1.0, 1.0] {
            for d in [-1.0, 1.0] {
                for (a, b) in [
                    (corner(w, hh, d), (w < 0.0).then(|| corner(1.0, hh, d))),
                    (corner(w, hh, d), (hh < 0.0).then(|| corner(w, 1.0, d))),
                    (corner(w, hh, d), (d < 0.0).then(|| corner(w, hh, 1.0))),
                ] {
                    if let Some(b) = b {
                        edges += 1;
                        assert!(
                            drawn.iter().any(|s| close(s[0], a) && close(s[1], b)),
                            "no cube edge {a:?} → {b:?}"
                        );
                    }
                }
            }
        }
    }
    assert_eq!(edges, 12);
    let weak = egui::Visuals::dark().weak_text_color();
    let cube = shapes
        .iter()
        .filter(|s| matches!(s, egui::Shape::LineSegment { stroke, .. } if stroke.color == weak))
        .count();
    assert_eq!(cube, 12, "the cube's twelve edges, no more");
}

/// Asserts the drawing of the cube, the plane, the labels and (when `locked`) the pin in `sq`.
fn check_drawing(shapes: &[egui::Shape], sq: Rect, plane: &Plane, locked: bool) {
    let close = |a: egui::Pos2, b: egui::Pos2| (a - b).length() < 1e-2;
    check_cube(shapes, sq, VIEW_ANGLE);
    let view = View::of(plane);
    let frame = plane_frame(plane, &view, true);
    let poly = corners(sq, frame);
    assert!(
        shapes.iter().any(|s| matches!(s, egui::Shape::Path(p)
            if p.closed && p.points.len() == 4 && p.points.iter().zip(&poly).all(|(a, b)| close(*a, *b)))),
        "the plane at {poly:?}"
    );
    let drawn = texts(shapes);
    for (p, text) in [
        ([1.25, -1.0, -1.0], format!("W {}", AXES[view.a])),
        ([-1.0, 1.2, -1.0], format!("H {}", AXES[view.b])),
        ([-1.0, -1.0, 1.3], format!("D {}", AXES[view.d])),
    ] {
        let at = project(sq, p);
        assert!(
            drawn
                .iter()
                .any(|(t, r)| *t == text && (r.center() - at).length() < 1.0),
            "{text} at {at:?}: {drawn:?}"
        );
    }
    let centre = project(sq, frame.0);
    let head = centre - egui::vec2(0.0, 18.0);
    let pin = segments(shapes)
        .iter()
        .any(|s| close(s[0], centre) && close(s[1], head));
    let label = drawn.iter().any(|(t, r)| {
        t == PINNED && (r.min.x - (head.x + 8.0)).abs() < 1.0 && (r.center().y - head.y).abs() < 1.0
    });
    assert_eq!(
        (pin, label),
        (locked, locked),
        "the pin and its label while locked"
    );
    if locked {
        for (radius, at) in [(3.0, centre), (5.0, head)] {
            assert!(
                shapes.iter().any(|s| matches!(s, egui::Shape::Circle(c)
                if close(c.center, at) && c.radius == radius && c.fill == GOLD)),
                "a gold dot of {radius} at {at:?}"
            );
        }
    }
}

#[test]
fn mock_compass_draws_the_cube_the_plane_and_the_pin() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let sq = square(compass_rect(&h));
    let plane = app.snapshot().sim.plane.clone();
    let free = shapes(&mut h, &mut app);
    check_drawing(&free, sq, &plane, false);
    app.set_field(lock_edit(Lock {
        locked: true,
        z_locked: MOCK_Z0,
    }));
    let _ = h.frame(&mut app, Vec::new());
    let locked = shapes(&mut h, &mut app);
    check_drawing(&locked, sq, &plane, true);
    rejects("the free compass read as locked", || {
        check_drawing(&free, sq, &plane, true)
    });
    rejects("the cube seen from another angle", || {
        check_cube(&free, sq, (0.0, 0.0))
    });
}

/// The dashed segments in the plane's colour.
fn dashes(shapes: &[egui::Shape]) -> usize {
    let blue = plane_colour(&Visuals::dark());
    shapes
        .iter()
        .filter(|s| matches!(s, egui::Shape::LineSegment { stroke, .. } if stroke.color == blue))
        .count()
}

/// Whether each of the Tilt and Slice buttons is drawn selected, from one more frame's AccessKit tree.
fn buttons(h: &mut Headless, app: &mut App<MockSide>) -> (bool, bool) {
    let output = h.frame(app, Vec::new());
    let update = output
        .platform_output
        .accesskit_update
        .expect("an AccessKit tree");
    let selected = |label: &str| {
        let node = update
            .nodes
            .iter()
            .map(|(_, n)| n)
            .find(|n| n.label() == Some(label))
            .unwrap_or_else(|| panic!("no {label} button"));
        node.toggled() == Some(egui::accesskit::Toggled::True)
    };
    (selected(TILT_BUTTON), selected(SLICE_BUTTON))
}

/// Asserts the compass's mode and that its buttons show it.
fn check_mode(h: &mut Headless, app: &mut App<MockSide>, want: Touch, what: &str) {
    assert_eq!(app.manifold.compass.mode, want, "{what}: the mode");
    assert_eq!(
        buttons(h, app),
        (want == Touch::Tilt, want == Touch::Slice),
        "{what}: the buttons (Tilt, Slice)"
    );
}

/// Clicks the button `label`.
fn click_button(h: &mut Headless, app: &mut App<MockSide>, label: &str) {
    super::support::click(h, app, label);
    let _ = h.frame(app, Vec::new());
}

/// Presses each of `keys`.
fn keys(h: &mut Headless, app: &mut App<MockSide>, keys: &[(Key, Modifiers)]) {
    for (key, modifiers) in keys {
        press(h, app, *key, *modifiers);
    }
}

#[test]
fn mock_compass_modes_open_in_tilt_and_switch_by_buttons_and_sliders() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(
        Compass::default(),
        Compass {
            mode: Touch::Tilt,
            drag: None
        },
        "Tilt by default"
    );
    check_mode(&mut h, &mut app, Touch::Tilt, "opened");
    click_button(&mut h, &mut app, SLICE_BUTTON);
    check_mode(&mut h, &mut app, Touch::Slice, "a click on Slice");
    click_button(&mut h, &mut app, TILT_BUTTON);
    check_mode(&mut h, &mut app, Touch::Tilt, "a click on Tilt");
    // Touching the slice step (Slice & tilt's first slider) shows Slice; touching τ₁ shows Tilt.
    keys(
        &mut h,
        &mut app,
        &[
            (Key::Tab, NONE),
            (Key::Tab, NONE),
            (Key::Enter, NONE),
            (Key::ArrowDown, NONE),
            (Key::ArrowDown, NONE),
            (Key::ArrowDown, NONE),
            (Key::Enter, NONE),
            (Key::Enter, NONE),
            (Key::ArrowUp, NONE),
        ],
    );
    assert_eq!(focus(&app).last(), Some(&"slice_step"));
    check_mode(&mut h, &mut app, Touch::Slice, "the slice step touched");
    keys(
        &mut h,
        &mut app,
        &[
            (Key::Escape, NONE),
            (Key::ArrowDown, NONE),
            (Key::Enter, NONE),
            (Key::ArrowUp, NONE),
        ],
    );
    assert_eq!(focus(&app).last(), Some(&"tau1"));
    check_mode(&mut h, &mut app, Touch::Tilt, "τ₁ touched");
    // A Centre z₀ slider shows Slice too.
    keys(
        &mut h,
        &mut app,
        &[
            (Key::Escape, NONE),
            (Key::Escape, NONE),
            (Key::ArrowUp, NONE),
            (Key::Enter, NONE),
            (Key::Enter, NONE),
            (Key::ArrowUp, NONE),
        ],
    );
    assert_eq!(focus(&app).last(), Some(&"centre_z0_0"));
    check_mode(&mut h, &mut app, Touch::Slice, "a z₀ slider touched");
    // The controls: buttons that do not follow the slider touched, and buttons that show no mode.
    app.manifold.compass.mode = Touch::Tilt;
    rejects("a button that does not follow a slider", || {
        check_mode(&mut h, &mut app, Touch::Slice, "the slider touched")
    });
    rejects("buttons that show no mode", || {
        assert_eq!(buttons(&mut h, &mut app), (false, false))
    });
}

#[test]
fn mock_compass_modes_read_and_draw() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let rect = compass_rect(&h);
    let sq = square(rect);
    let all = nodes(&mut h, &mut app);
    let text =
        |all: &[super::support::Node], t: &str| all.iter().find(|n| n.text == t).map(|n| n.rect);
    for t in [
        TITLE,
        TILT_BUTTON,
        SLICE_BUTTON,
        HINT_TILT,
        "τ₁ 0.0° · τ₂ 0.0° · γ 0.0°",
    ] {
        let r = text(&all, t).unwrap_or_else(|| panic!("no {t:?}"));
        assert!(
            r.min.x >= sq.max.x + MARGIN - 0.5,
            "{t} right of the cube: {r:?}"
        );
        assert!(
            rect.shrink(MARGIN - 0.5).contains_rect(r),
            "{t} inside the margin: {r:?}"
        );
    }
    assert!(text(&all, HINT_SLICE).is_none(), "the tilt hint only");
    let tilting = dashes(&shapes(&mut h, &mut app));
    assert!(
        tilting > 4,
        "tilting: the plane before the tilt, dashed ({tilting})"
    );
    app.manifold.compass.mode = Touch::Slice;
    let all = nodes(&mut h, &mut app);
    assert!(text(&all, HINT_SLICE).is_some() && text(&all, HINT_TILT).is_none());
    assert_eq!(
        dashes(&shapes(&mut h, &mut app)),
        0,
        "slicing: no pre-tilt outline"
    );
    rejects("the tilt outline while slicing", || {
        assert!(dashes(&shapes(&mut h, &mut app)) > 4)
    });
}

fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn drag(h: &mut Headless, app: &mut App<MockSide>, from: egui::Pos2, delta: egui::Vec2) {
    for events in [
        vec![egui::Event::PointerMoved(from)],
        vec![button(from, true)],
        vec![egui::Event::PointerMoved(from + delta / 2.0)],
        vec![egui::Event::PointerMoved(from + delta)],
        vec![button(from + delta, false)],
        Vec::new(),
    ] {
        let _ = h.frame(app, events);
    }
}

/// The `SetField`s the mock logged since row `from` of the console, by their paths.
fn setfields(app: &App<MockSide>, from: usize) -> Vec<&'static str> {
    app.console()[from..]
        .iter()
        .filter_map(|e| {
            let m = e.message.as_str();
            if m.starts_with("SetField Plane.z0 ") {
                Some("z0")
            } else if m.starts_with("SetField Plane.q1 Plane.q2 ") {
                Some("basis")
            } else if m.starts_with("SetField ") {
                Some("other")
            } else {
                None
            }
        })
        .collect()
}

/// Asserts one `SetField`, on `field`.
fn check_one(fields: &[&str], field: &str) {
    assert_eq!(fields, [field], "one SetField on {field}");
}

/// The app with the plane tilted by `τ₁` 30°, `τ₂` −20° and turned by `γ` 25°, read.
fn tilted_app() -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    let p = tilted(30.0, -20.0, 25.0);
    app.set_field(basis_edit((p.q1, p.q2)));
    let _ = h.frame(&mut app, Vec::new());
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

/// A point of the cube's square off the plane: near its top left corner.
fn off_plane(sq: Rect) -> egui::Pos2 {
    sq.min + egui::vec2(4.0, 4.0)
}

/// Where a drag starts: on the drawn plane's centre, or off the plane; checked by the hit test.
fn start(sq: Rect, p: &Plane, on_plane: bool) -> egui::Pos2 {
    let frame = plane_frame(p, &View::of(p), true);
    let from = if on_plane {
        project(sq, frame.0)
    } else {
        off_plane(sq)
    };
    assert_eq!(inside(&corners(sq, frame), from), on_plane);
    from
}

#[test]
fn mock_compass_modes_a_tilt_drag_anywhere_tilts_once() {
    for on_plane in [true, false] {
        let (mut app, mut h) = tilted_app();
        let sq = square(compass_rect(&h));
        let before = app.snapshot().sim.plane.clone();
        let cube = shapes(&mut h, &mut app);
        let rows = app.console().len();
        drag(
            &mut h,
            &mut app,
            start(sq, &before, on_plane),
            egui::vec2(20.0, 10.0),
        );
        let fields = setfields(&app, rows);
        check_one(&fields, "basis");
        let after = app.snapshot().sim.plane.clone();
        assert_eq!(after.z0, before.z0, "none on z₀");
        let (b, a) = (
            Angles::of(&View::of(&before)),
            Angles::of(&View::of(&after)),
        );
        assert!(
            (a.tau1 - b.tau1 - 10.0).abs() < 1e-4 && (a.tau2 - b.tau2 + 5.0).abs() < 1e-4,
            "right tilts τ₁ up, down tilts τ₂ down: {b:?} → {a:?}"
        );
        assert_eq!(app.manifold.compass.mode, Touch::Tilt);
        assert_eq!(app.manifold.compass.drag, None, "the drag ended");
        // The view is fixed: the cube is drawn as before.
        let moved = shapes(&mut h, &mut app);
        check_cube(&cube, sq, VIEW_ANGLE);
        check_cube(&moved, sq, VIEW_ANGLE);
        rejects("a drag that orbits", || {
            check_cube(&moved, sq, (VIEW_ANGLE.0 + 10.0, VIEW_ANGLE.1 + 5.0))
        });
        rejects("a drag of a SetField a frame", || {
            check_one(&["basis", "basis"], "basis")
        });
        rejects("a tilt-mode drag that slices", || check_one(&fields, "z0"));
    }
}

#[test]
fn mock_compass_modes_a_slice_drag_anywhere_moves_along_the_normal() {
    for on_plane in [true, false] {
        let (mut app, mut h) = tilted_app();
        click_button(&mut h, &mut app, SLICE_BUTTON);
        let sq = square(compass_rect(&h));
        let before = app.snapshot().sim.plane.clone();
        let rows = app.console().len();
        let delta = egui::vec2(-12.0, 9.0);
        drag(&mut h, &mut app, start(sq, &before, on_plane), delta);
        check_one(&setfields(&app, rows), "z0");
        let after = app.snapshot().sim.plane.clone();
        assert_eq!(
            (after.q1, after.q2),
            (before.q1, before.q2),
            "the basis unchanged"
        );
        let amount = slice_by_drag(&before, sq, delta);
        assert!(amount.abs() > 1e-3, "it moves: {amount}");
        check_along_normal(&before, &z0_edit(after.z0), amount, 1e-5);
        assert_eq!(app.manifold.compass.mode, Touch::Slice);
        check_cube(&shapes(&mut h, &mut app), sq, VIEW_ANGLE);
        rejects("a slice-mode drag that tilts", || {
            along_normal(&before, &tilt(&before, 5.0, 0.0));
        });
        rejects(
            "a slice move along the depth axis on a tilted plane",
            || {
                let mut z0 = before.z0;
                z0[SLICE_AXIS] += amount;
                along_normal(&before, &z0_edit(z0));
            },
        );
    }
}

#[test]
fn mock_compass_modes_the_drag_draws_its_plane_until_it_ends() {
    let (mut app, mut h) = tilted_app();
    click_button(&mut h, &mut app, SLICE_BUTTON);
    let sq = square(compass_rect(&h));
    let before = app.snapshot().sim.plane.clone();
    let from = off_plane(sq);
    let delta = egui::vec2(-12.0, 9.0);
    for events in [
        vec![egui::Event::PointerMoved(from)],
        vec![button(from, true)],
        vec![egui::Event::PointerMoved(from + delta)],
    ] {
        let _ = h.frame(&mut app, events);
    }
    let depth = app.snapshot().history.undo_depth;
    let mid = shapes(&mut h, &mut app);
    assert_eq!(app.snapshot().history.undo_depth, depth, "no edit yet");
    let shown = after(
        &before,
        &[slice(&before, slice_by_drag(&before, sq, delta))],
    );
    check_drawing(&mid, sq, &shown, false);
    rejects("the plane drawn where the drag began", || {
        check_drawing(&mid, sq, &before, false)
    });
    let _ = h.frame(&mut app, vec![button(from + delta, false)]);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(
        app.snapshot().history.undo_depth,
        depth + 1,
        "one edit, at the end"
    );
}

#[test]
fn mock_compass_modes_locked_a_slice_drag_leaves_the_anchor_along_the_normal() {
    let (mut app, mut h) = tilted_app();
    let anchor = app.snapshot().sim.plane.z0;
    app.set_field(lock_edit(Lock {
        locked: true,
        z_locked: anchor,
    }));
    let _ = h.frame(&mut app, Vec::new());
    click_button(&mut h, &mut app, SLICE_BUTTON);
    let sq = square(compass_rect(&h));
    let before = app.snapshot().sim.plane.clone();
    assert_eq!(before.z0, anchor);
    let rows = app.console().len();
    drag(&mut h, &mut app, off_plane(sq), egui::vec2(8.0, -14.0));
    check_one(&setfields(&app, rows), "z0");
    let snapshot = app.snapshot();
    assert!(snapshot.sim.lock.locked, "still locked");
    assert_eq!(snapshot.sim.lock.z_locked, anchor, "the anchor kept");
    // The excursion is along the normal through the anchor.
    let n = latent_normal(&before);
    let on_line = |z0: &Latent| {
        let offset = add(z0, -1.0, &anchor);
        let along = dot(&offset, &n);
        assert!(along.abs() > 1e-3, "it moves");
        assert!(
            norm(&add(&offset, -along, &n)) < 1e-12,
            "off the line through the anchor: {offset:?}"
        );
    };
    on_line(&snapshot.sim.plane.z0);
    rejects("an excursion along the depth axis", || {
        let mut z0 = anchor;
        z0[SLICE_AXIS] += 0.1;
        on_line(&z0)
    });
}

#[test]
fn mock_compass_the_pin_stands_at_the_anchor_after_an_excursion() {
    let mut app = mock_app();
    let mut h = headless();
    let lock = Lock {
        locked: true,
        z_locked: MOCK_Z0,
    };
    app.set_field(lock_edit(lock.clone()));
    // Off the anchor by +0.4 q₁ − 0.2 q₂: the anchor is at the chart point (0.3, 0.6).
    app.set_field(z0_edit(add(&add(&MOCK_Z0, 0.4, &MOCK_Q1), -0.2, &MOCK_Q2)));
    let _ = h.frame(&mut app, Vec::new());
    let sq = square(compass_rect(&h));
    let plane = app.snapshot().sim.plane.clone();
    let frame = plane_frame(&plane, &View::of(&plane), true);
    let (c, r1, r2) = frame;
    let want: [f64; 3] = std::array::from_fn(|i| c[i] + PLANE_HALF * (-0.4 * r1[i] + 0.2 * r2[i]));
    let at = pin(&plane, &lock, frame).expect("locked");
    assert!(near(at, want), "{at:?} ≠ {want:?}");
    assert_eq!(pin(&plane, &free_lock(), frame), None, "free");
    let drawn = shapes(&mut h, &mut app);
    let dot = |p: [f64; 3]| {
        let at = project(sq, p);
        drawn.iter().any(|s| {
            matches!(s, egui::Shape::Circle(c)
            if close(c.center, at) && c.radius == 3.0 && c.fill == GOLD)
        })
    };
    assert!(dot(want), "the pin at the anchor");
    rejects("the pin at the plane's centre", || assert!(dot(frame.0)));
}

// --- the compass from the keyboard (REQ-GUI-183) ---------------------------------------------------------------------

/// The focus and its mode.
fn at(app: &App<MockSide>) -> (Vec<&str>, KeyMode) {
    (focus(app), app.view.focus.mode)
}

/// Walks the Compass scope on `tree`: Enter onto its first element, then → → → ← ←; the focused id at each.
fn sibling_walk(tree: &ScopeTree) -> Vec<String> {
    let mut f = Focus {
        path: vec![COMPASS.to_owned()],
        mode: KeyMode::Navigation,
    };
    tree.navigate(&mut f, Command::Enter, 1.0);
    let mut seen = vec![f.path.last().cloned().unwrap_or_default()];
    for d in [
        Direction::Right,
        Direction::Right,
        Direction::Right,
        Direction::Left,
        Direction::Left,
    ] {
        tree.navigate(&mut f, Command::Arrow(d), 1.0);
        seen.push(f.path.last().cloned().unwrap_or_default());
    }
    seen
}

fn check_sibling_walk(seen: &[String]) {
    assert_eq!(
        seen,
        [
            COMPASS_VIEW,
            COMPASS_TILT,
            COMPASS_SLICE,
            COMPASS_SLICE,
            COMPASS_TILT,
            COMPASS_VIEW
        ],
        "compass, Tilt, Slice, stopping at Slice, and back"
    );
}

#[test]
fn mock_compass_keys_the_compass_tilt_and_slice_are_siblings() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    keys(&mut h, &mut app, &[(Key::Tab, NONE); 5]);
    assert_eq!(
        at(&app),
        (vec![COMPASS], KeyMode::Navigation),
        "Tab lands on the scope"
    );
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(
        at(&app),
        (vec![COMPASS, COMPASS_VIEW], KeyMode::Navigation),
        "Enter goes in onto the compass, navigating"
    );
    check_sibling_walk(&sibling_walk(app.keyboard.tree(Mode::Explore)));
    let mut wrong = ScopeTree::new();
    wrong.register(None, Scope::group(COMPASS, "Compass"));
    wrong.register(
        Some(COMPASS),
        Scope::value(COMPASS_VIEW, "compass", StepKind::Tilt),
    );
    wrong.register(Some(COMPASS), Scope::control(COMPASS_SLICE, SLICE_BUTTON));
    wrong.register(Some(COMPASS), Scope::control(COMPASS_TILT, TILT_BUTTON));
    rejects("→ from the compass reaching Slice", || {
        check_sibling_walk(&sibling_walk(&wrong))
    });
    // Enter on a button sets the mode, staying in navigation mode on it.
    keys(
        &mut h,
        &mut app,
        &[
            (Key::ArrowRight, NONE),
            (Key::ArrowRight, NONE),
            (Key::Enter, NONE),
        ],
    );
    assert_eq!(
        at(&app),
        (vec![COMPASS, COMPASS_SLICE], KeyMode::Navigation)
    );
    check_mode(&mut h, &mut app, Touch::Slice, "Enter on Slice");
    keys(
        &mut h,
        &mut app,
        &[(Key::ArrowLeft, NONE), (Key::Enter, NONE)],
    );
    assert_eq!(at(&app), (vec![COMPASS, COMPASS_TILT], KeyMode::Navigation));
    check_mode(&mut h, &mut app, Touch::Tilt, "Enter on Tilt");
    // In navigation mode the arrows on the compass move, and edit nothing.
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    let depth = app.snapshot().history.undo_depth;
    let still = |app: &App<MockSide>| {
        assert_eq!(
            app.snapshot().history.undo_depth,
            depth,
            "no edit in navigation mode"
        )
    };
    for key in [Key::ArrowLeft, Key::ArrowUp] {
        press(&mut h, &mut app, key, NONE);
        let _ = h.frame(&mut app, Vec::new());
        still(&app);
    }
    assert_eq!(at(&app), (vec![COMPASS, COMPASS_VIEW], KeyMode::Navigation));
    // Enter on the compass starts interaction; there an arrow tilts.
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(
        at(&app),
        (vec![COMPASS, COMPASS_VIEW], KeyMode::Interaction)
    );
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    let _ = h.frame(&mut app, Vec::new());
    rejects(
        "an arrow on the compass in navigation mode that tilts",
        || still(&app),
    );
    // Esc returns to navigation mode on the compass.
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(at(&app), (vec![COMPASS, COMPASS_VIEW], KeyMode::Navigation));
}

/// One arrow `key` with `modifiers` on the compass in interaction mode, in `mode`, on the tilted plane: the
/// `SetField`s it made, by path, and the plane before and after. No key turns the view.
fn compass_press(mode: Touch, key: Key, modifiers: Modifiers) -> (Vec<&'static str>, Plane, Plane) {
    let (mut app, mut h) = tilted_app();
    app.manifold.compass.mode = mode;
    keys(&mut h, &mut app, &[(Key::Tab, NONE); 5]);
    keys(&mut h, &mut app, &[(Key::Enter, NONE), (Key::Enter, NONE)]);
    assert_eq!(
        at(&app),
        (vec![COMPASS, COMPASS_VIEW], KeyMode::Interaction)
    );
    let before = app.snapshot().sim.plane.clone();
    let rows = app.console().len();
    press(&mut h, &mut app, key, modifiers);
    let _ = h.frame(&mut app, Vec::new());
    check_cube(
        &shapes(&mut h, &mut app),
        square(compass_rect(&h)),
        VIEW_ANGLE,
    );
    assert_eq!(app.manifold.compass.mode, mode, "the mode kept");
    let fields = setfields(&app, rows);
    (fields, before, app.snapshot().sim.plane.clone())
}

/// The tilt `(Δτ₁, Δτ₂)` in degrees between two planes.
fn tilt_between(a: &Plane, b: &Plane) -> (f64, f64) {
    let (x, y) = (Angles::of(&View::of(a)), Angles::of(&View::of(b)));
    (y.tau1 - x.tau1, y.tau2 - x.tau2)
}

/// Asserts a tilt of `want` by one SetField on the basis.
fn check_tilt((fields, a, b): &(Vec<&str>, Plane, Plane), want: (f64, f64)) {
    check_one(fields, "basis");
    let (d1, d2) = tilt_between(a, b);
    assert!(
        (d1 - want.0).abs() < 1e-9 && (d2 - want.1).abs() < 1e-9,
        "({d1}, {d2}) for {want:?}"
    );
}

#[test]
fn mock_compass_keys_tilt_mode_arrows_tilt_shift_x10_alt_x0_1() {
    let Base::Degrees(step) = StepKind::Tilt.base() else {
        panic!("the tilt step is in degrees")
    };
    assert_eq!(step, 1.0, "R-404's Tilt step");
    for (key, unit) in [
        (Key::ArrowRight, (1.0, 0.0)),
        (Key::ArrowLeft, (-1.0, 0.0)),
        (Key::ArrowUp, (0.0, 1.0)),
        (Key::ArrowDown, (0.0, -1.0)),
    ] {
        for (modifiers, times) in [(NONE, 1.0), (SHIFT, 10.0), (ALT, 0.1)] {
            let got = compass_press(Touch::Tilt, key, modifiers);
            check_tilt(&got, (unit.0 * step * times, unit.1 * step * times));
        }
    }
    let shifted = compass_press(Touch::Tilt, Key::ArrowRight, SHIFT);
    rejects(
        "Shift+arrows that give a step smaller than the arrow's",
        || check_tilt(&shifted, (0.1, 0.0)),
    );
    rejects("Shift+arrows that orbit, editing nothing", || {
        check_tilt(
            &(Vec::new(), shifted.1.clone(), shifted.1.clone()),
            (10.0, 0.0),
        )
    });
}

#[test]
fn mock_compass_keys_slice_mode_arrows_move_along_the_normal() {
    let Base::OfRange(fraction) = StepKind::SliceNormal.base() else {
        panic!("the slice step is a fraction of the range")
    };
    // REQ-GUI-184, proposed: a hundredth of the slice step's range, [−1, 1].
    let step = fraction * 2.0;
    assert!((step - 0.02).abs() < 1e-15, "{step}");
    for (key, sign) in [
        (Key::ArrowUp, 1.0),
        (Key::ArrowRight, 1.0),
        (Key::ArrowDown, -1.0),
        (Key::ArrowLeft, -1.0),
    ] {
        for (modifiers, times) in [(NONE, 1.0), (SHIFT, 10.0), (ALT, 0.1)] {
            let (fields, a, b) = compass_press(Touch::Slice, key, modifiers);
            check_one(&fields, "z0");
            assert_eq!((a.q1, a.q2), (b.q1, b.q2), "the tilt unchanged");
            check_along_normal(&a, &z0_edit(b.z0), sign * step * times, 1e-12);
        }
    }
    let (_, a, b) = compass_press(Touch::Slice, Key::ArrowUp, SHIFT);
    rejects(
        "Shift+arrows that give a step smaller than the arrow's",
        || check_along_normal(&a, &z0_edit(b.z0), 0.1 * step, 1e-12),
    );
    let (tilted_instead, _, _) = compass_press(Touch::Tilt, Key::ArrowUp, NONE);
    rejects("a slice-mode arrow that tilts", || {
        check_one(&tilted_instead, "z0")
    });
}

#[test]
fn mock_compass_keys_adjust_steps_by_the_mode() {
    let p = tilted(10.0, 5.0, 0.0);
    let run = |mode, direction, times| {
        let mut scratch = Scratch::default();
        scratch.compass.mode = mode;
        let mut out = Out::default();
        let a = Adjust {
            scope: COMPASS_VIEW,
            kind: StepKind::Tilt,
            times,
            direction,
        };
        crate::explore::compass::adjust(&a, &p, &mut scratch, &mut out);
        assert_eq!(scratch.compass.mode, mode);
        assert_eq!(out.edits.len(), 1);
        out.edits.remove(0)
    };
    let (t1, t2) = angles_of(&p, &run(Touch::Tilt, Direction::Left, -10.0));
    assert!(t1.abs() < 1e-9 && (t2 - 5.0).abs() < 1e-9, "({t1}, {t2})");
    let (t1, t2) = angles_of(&p, &run(Touch::Tilt, Direction::Down, -1.0));
    assert!(
        (t1 - 10.0).abs() < 1e-9 && (t2 - 4.0).abs() < 1e-9,
        "({t1}, {t2})"
    );
    check_along_normal(&p, &run(Touch::Slice, Direction::Down, -0.1), -0.002, 1e-12);
    rejects("the slice's step taken as the tilt's degrees", || {
        check_along_normal(&p, &run(Touch::Slice, Direction::Up, 1.0), 1.0, 1e-12)
    });
}
