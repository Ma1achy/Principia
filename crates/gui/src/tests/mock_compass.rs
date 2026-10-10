//! The compass's geometry (render_gui_spec §G2: "a small 3D cube showing the slice plane inside the chart"; §G3: its
//! arrows tilt, Shift+arrows orbit): the cube's projection under its orbit, the slice plane's frame from the view, its
//! corners, the hit test that decides a drag tilts or orbits, and the tilt edit. Each check also runs on an input it
//! must reject (R-176, through `rejects`).

use eframe::egui::{self, pos2, Rect, Visuals};
use engine::contract::set_field::{Edit, SimField};
use engine::contract::sim_config::Plane;

use super::support::rejects;
use crate::explore::compass::{
    corners, inside, plane_colour, plane_frame, tilt, Compass, CUBE_SCALE, ORBIT, PLANE_HALF,
};
use crate::explore::manifold_view::{Touch, View, SLICE_AXIS};
use crate::mock::{MOCK_Q1, MOCK_Q2, MOCK_Z0};

const SQUARE: Rect = Rect::from_min_max(pos2(100.0, 200.0), pos2(300.0, 400.0));

fn close(a: egui::Pos2, b: egui::Pos2) -> bool {
    (a - b).length() < 1e-3
}

fn compass(yaw: f64, pitch: f64) -> Compass {
    Compass {
        yaw,
        pitch,
        ..Compass::default()
    }
}

#[test]
fn mock_compass_projection() {
    let c = SQUARE.center();
    let r = 200.0 * CUBE_SCALE as f32;
    // Head on: W right, H up, D into the screen, at the centre.
    let front = compass(0.0, 0.0);
    assert!(close(
        front.project(SQUARE, [1.0, 0.0, 0.0]),
        c + egui::vec2(r, 0.0)
    ));
    assert!(close(
        front.project(SQUARE, [0.0, 1.0, 0.0]),
        c + egui::vec2(0.0, -r)
    ));
    assert!(close(front.project(SQUARE, [0.0, 0.0, 1.0]), c));
    // A quarter turn of yaw: D comes right, W goes into the screen.
    let side = compass(90.0, 0.0);
    assert!(close(
        side.project(SQUARE, [0.3, 0.5, 0.7]),
        c + egui::vec2(0.7 * r, -0.5 * r)
    ));
    // Looking straight down: D comes down the screen, H goes into it.
    let top = compass(0.0, 90.0);
    assert!(close(
        top.project(SQUARE, [0.3, 0.5, 0.7]),
        c + egui::vec2(0.3 * r, 0.7 * r)
    ));
    // Both: yaw then pitch, W into the screen and then down it.
    let both = compass(90.0, 90.0);
    assert!(close(
        both.project(SQUARE, [0.3, 0.5, 0.7]),
        c + egui::vec2(0.7 * r, -0.3 * r)
    ));
    // A general orbit, against the rotations composed by hand.
    let (yaw, pitch) = (30f64.to_radians(), 20f64.to_radians());
    let [w, h, d] = [0.4, -0.6, 0.9];
    let (x, depth) = (
        w * yaw.cos() + d * yaw.sin(),
        -w * yaw.sin() + d * yaw.cos(),
    );
    let y = h * pitch.cos() - depth * pitch.sin();
    let want = c + egui::vec2((x * f64::from(r)) as f32, (-y * f64::from(r)) as f32);
    assert!(close(compass(30.0, 20.0).project(SQUARE, [w, h, d]), want));
    rejects("the head-on view's H downwards", || {
        assert!(close(
            front.project(SQUARE, [0.0, 1.0, 0.0]),
            c + egui::vec2(0.0, r)
        ));
    });
}

#[test]
fn mock_compass_opens_as_01_main_draws_it() {
    // 01_main.png's cube: W to the right, D towards the lower left, H up.
    let open = Compass::default();
    assert_eq!((open.yaw, open.pitch), ORBIT);
    assert_eq!((open.mode, open.drag), (Touch::Slice, None));
    let c = SQUARE.center();
    let (w, h, d) = (
        open.project(SQUARE, [1.0, 0.0, 0.0]),
        open.project(SQUARE, [0.0, 1.0, 0.0]),
        open.project(SQUARE, [0.0, 0.0, 1.0]),
    );
    assert!(w.x > c.x, "W to the right: {w:?}");
    assert!(d.x < c.x && d.y > c.y, "D to the lower left: {d:?}");
    assert!(h.y < c.y, "H up: {h:?}");
    let mirrored = compass(-ORBIT.0, ORBIT.1).project(SQUARE, [0.0, 0.0, 1.0]);
    rejects("the orbit mirrored", || assert!(mirrored.x < c.x));
    // The plane's colour is the theme's selection blue, drawn.
    let visuals = Visuals::dark();
    assert_eq!(plane_colour(&visuals), visuals.selection.stroke.color);
    assert_ne!(plane_colour(&visuals), egui::Color32::TRANSPARENT);
}

#[test]
fn mock_compass_orbit() {
    let mut c = Compass::default();
    c.orbit(10.0, 5.0);
    assert_eq!(
        (c.yaw, c.pitch),
        (ORBIT.0 + 10.0 + 360.0, ORBIT.1 + 5.0),
        "yaw wraps into [0, 360)"
    );
    c.orbit(-30.0, -10.0);
    assert_eq!((c.yaw, c.pitch), (305.0, 20.0));
    c.orbit(20.0, 100.0);
    assert_eq!((c.yaw, c.pitch), (325.0, 89.0), "pitch stops at 89");
    c.orbit(0.0, -500.0);
    assert_eq!(c.pitch, -89.0);
    rejects("a pitch past the pole", || {
        let mut c = Compass::default();
        c.orbit(0.0, 100.0);
        assert_eq!(c.pitch, 125.0);
    });
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
    let front = compass(0.0, 0.0);
    let frame = ([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let poly = corners(&front, SQUARE, frame);
    let c = SQUARE.center();
    let k = (200.0 * CUBE_SCALE * PLANE_HALF) as f32;
    let want = [(k, -k), (-k, -k), (-k, k), (k, k)].map(|(x, y)| c + egui::vec2(x, y));
    assert_eq!(poly.len(), 4);
    for (got, want) in poly.iter().zip(want) {
        assert!(close(*got, want), "{got:?} for {want:?}");
    }
    // Off the centre: shifted by the centre's projection.
    let shifted = corners(&front, SQUARE, ([0.5, 0.0, 0.0], frame.1, frame.2));
    assert!(close(
        shifted[0],
        want[0] + egui::vec2(100.0 * CUBE_SCALE as f32, 0.0)
    ));
    assert!(inside(&poly, c), "the centre is on the plane");
    assert!(inside(&poly, want[0]), "a corner is on it");
    assert!(
        !inside(&poly, c + egui::vec2(k + 1.0, 0.0)),
        "right of it is off"
    );
    assert!(
        !inside(&poly, c + egui::vec2(0.0, -k - 1.0)),
        "above it is off"
    );
    let reversed: Vec<_> = poly.iter().rev().copied().collect();
    assert!(inside(&reversed, c), "either winding");
    assert!(!inside(&reversed, c + egui::vec2(-k - 1.0, 0.0)));
    let triangle = [c, c + egui::vec2(10.0, 0.0), c + egui::vec2(0.0, 10.0)];
    assert!(
        !inside(&triangle, c + egui::vec2(6.0, 6.0)),
        "past the hypotenuse is off"
    );
    assert!(inside(&triangle, c + egui::vec2(4.0, 4.0)));
    rejects("a point off the plane", || {
        assert!(inside(&poly, c + egui::vec2(k + 1.0, 0.0)))
    });
}

#[test]
fn mock_compass_tilt_edit() {
    let p = plane(0.0);
    let angles = |edit: engine::contract::set_field::SetField| match edit.edit {
        Edit::Sim(SimField::Basis { q1, q2 }) => {
            let v = View::of(&Plane { z0: p.z0, q1, q2 });
            (v.tau1.to_degrees(), v.tau2.to_degrees())
        }
        other => panic!("a basis edit, not {other:?}"),
    };
    let (t1, t2) = angles(tilt(&p, 10.0, -5.0));
    assert!(
        (t1 - 10.0).abs() < 1e-9 && (t2 + 5.0).abs() < 1e-9,
        "({t1}, {t2})"
    );
    let (t1, t2) = angles(tilt(&p, 120.0, -120.0));
    assert!(
        (t1 - 90.0).abs() < 1e-9 && (t2 + 90.0).abs() < 1e-9,
        "clamped: ({t1}, {t2})"
    );
    rejects("a tilt the other way", || {
        let (t1, _) = angles(tilt(&p, 10.0, 0.0));
        assert!((t1 + 10.0).abs() < 1e-9);
    });
}
