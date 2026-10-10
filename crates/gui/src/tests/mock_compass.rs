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

// --- the compass as the app draws it ----------------------------------------------------------------------------------

use super::support::{headless, mock_app, nodes, shapes};
use crate::explore::compass::{square, HINT, MARGIN, PINNED, SLICING, TILTING, TITLE};
use crate::explore::lock::GOLD;
use crate::explore::manifold_view::{Out, Scratch, AXES};
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::{Adjust, StepKind};

fn compass_rect(h: &crate::headless::Headless) -> Rect {
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

/// Asserts the drawing of `compass`'s cube, plane, labels and (when `locked`) pin in `sq`.
fn check_drawing(shapes: &[egui::Shape], sq: Rect, compass: &Compass, plane: &Plane, locked: bool) {
    let close = |a: egui::Pos2, b: egui::Pos2| (a - b).length() < 1e-2;
    let segments: Vec<[egui::Pos2; 2]> = shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::LineSegment { points, .. } => Some(*points),
            _ => None,
        })
        .collect();
    let corner = |w: f64, h: f64, d: f64| compass.project(sq, [w, h, d]);
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
                            segments.iter().any(|s| close(s[0], a) && close(s[1], b)),
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
    let view = View::of(plane);
    let frame = plane_frame(plane, &view, true);
    let poly = corners(compass, sq, frame);
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
        let at = compass.project(sq, p);
        assert!(
            drawn
                .iter()
                .any(|(t, r)| *t == text && (r.center() - at).length() < 1.0),
            "{text} at {at:?}: {drawn:?}"
        );
    }
    let centre = compass.project(sq, frame.0);
    let head = centre - egui::vec2(0.0, 18.0);
    let pin = segments
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
    check_drawing(&free, sq, &Compass::default(), &plane, false);
    app.set_field(crate::explore::manifold_view::lock_edit(
        engine::contract::sim_config::Lock {
            locked: true,
            z_locked: MOCK_Z0,
        },
    ));
    let _ = h.frame(&mut app, Vec::new());
    let locked = shapes(&mut h, &mut app);
    check_drawing(&locked, sq, &Compass::default(), &plane, true);
    rejects("the free compass read as locked", || {
        check_drawing(&free, sq, &Compass::default(), &plane, true)
    });
    rejects("another orbit", || {
        check_drawing(&free, sq, &compass(0.0, 0.0), &plane, false)
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
    assert!(
        text(&all, &format!("● {SLICING}")).is_some()
            && text(&all, &format!("○ {TILTING}")).is_some()
    );
    for t in [TITLE, HINT, "τ₁ 0.0° · τ₂ 0.0° · γ 0.0°"] {
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
    assert_eq!(
        dashes(&shapes(&mut h, &mut app)),
        0,
        "slicing: no pre-tilt outline"
    );
    app.manifold.compass.mode = Touch::Tilt;
    let all = nodes(&mut h, &mut app);
    assert!(
        text(&all, &format!("○ {SLICING}")).is_some()
            && text(&all, &format!("● {TILTING}")).is_some()
    );
    let tilting = dashes(&shapes(&mut h, &mut app));
    assert!(
        tilting > 4,
        "tilting: the plane before the tilt, dashed ({tilting})"
    );
    rejects("the tilt outline while slicing", || {
        assert!(dashes(&shapes(&mut h, &mut mock_app())) > 4)
    });
}

fn drag(
    h: &mut crate::headless::Headless,
    app: &mut crate::app::App<crate::side::MockSide>,
    from: egui::Pos2,
    delta: egui::Vec2,
) {
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
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

#[test]
fn mock_compass_a_drag_on_the_plane_tilts_and_off_it_orbits() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let sq = square(compass_rect(&h));
    let plane = app.snapshot().sim.plane.clone();
    let on = compass(ORBIT.0, ORBIT.1).project(sq, plane_frame(&plane, &View::of(&plane), true).0);
    drag(&mut h, &mut app, on, egui::vec2(20.0, 10.0));
    let view = View::of(&app.snapshot().sim.plane);
    let (t1, t2) = (view.tau1.to_degrees(), view.tau2.to_degrees());
    assert!(
        (t1 - 10.0).abs() < 1e-6 && (t2 + 5.0).abs() < 1e-6,
        "right tilts τ₁ up, down tilts τ₂ down: ({t1}, {t2})"
    );
    assert_eq!(app.manifold.compass.mode, Touch::Tilt);
    assert_eq!(
        (app.manifold.compass.yaw, app.manifold.compass.pitch),
        ORBIT,
        "no orbit"
    );
    assert_eq!(app.manifold.compass.drag, None, "the drag ended");
    // Off the plane, in the cube's corner: the cube orbits, nothing is edited.
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let depth = app.snapshot().history.undo_depth;
    drag(
        &mut h,
        &mut app,
        sq.min + egui::vec2(4.0, 4.0),
        egui::vec2(20.0, 10.0),
    );
    let c = app.manifold.compass;
    assert_eq!(
        (c.yaw, c.pitch),
        (ORBIT.0 + 10.0 + 360.0, ORBIT.1 + 5.0),
        "half a degree a point"
    );
    assert_eq!(
        app.snapshot().history.undo_depth,
        depth,
        "an orbit edits nothing"
    );
    rejects("a tilt read off the plane", || {
        assert!((t1 + 10.0).abs() < 1e-6)
    });
}

#[test]
fn mock_compass_shift_arrows_orbit_by_the_orbit_step() {
    let snapshot_plane = plane(0.0);
    let run = |direction, times| {
        let mut scratch = Scratch::default();
        let mut out = Out::default();
        let a = Adjust {
            scope: crate::explore::COMPASS,
            kind: StepKind::Tilt,
            times,
            direction,
            shift: true,
        };
        crate::explore::compass::adjust(&a, &snapshot_plane, &mut scratch, &mut out);
        assert!(out.edits.is_empty(), "an orbit edits nothing");
        (
            (scratch.compass.yaw - ORBIT.0).rem_euclid(360.0),
            scratch.compass.pitch - ORBIT.1,
        )
    };
    // Shift's ×10 is the orbit's own step, 5°.
    assert_eq!(run(Direction::Right, 10.0), (5.0, 0.0));
    assert_eq!(run(Direction::Left, -10.0), (355.0, 0.0));
    assert_eq!(run(Direction::Up, 10.0), (0.0, 5.0));
    rejects("the step ten times over", || {
        assert_eq!(run(Direction::Up, 10.0), (0.0, 50.0))
    });
}
