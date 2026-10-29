//! `mock_compass_slice_follows_pointer` (REQ-GUI-217; R-465; render_gui_spec § "The compass: look and views (R-451,
//! R-452)", the drag): by pointer scripts on the mock, in slice mode, the slice drag reads the 2-D drag projected onto
//! the drawn normal's screen direction `ŝ`, the unit vector along `P(n̂) − P(0)`, so the drawn plane moves with the
//! pointer at every tilt, a drag of half the cube's width `√3·S` along `ŝ` moving `z₀` half the range along the normal
//! and the travel across `ŝ` nothing; near the view axis, where the drawn normal's image is shorter than `0.25·S`, the
//! drag reads its vertical travel, up moving the plane toward the viewer (N3). The plane measured is the target plane
//! the contract holds, unclamped and drawn without easing (N1, N4). The tilt-mode drag keeps REQ-GUI-193's rates,
//! re-run here unchanged.
//!
//! Each check also runs, in its test, on an input it must reject (R-176, through `rejects`; gui has no route to
//! `negative_control!`, `support`'s note): the `z₀` a GUI reading the drag otherwise would send.

use eframe::egui::{self, Vec2};
use engine::contract::sim_config::{Latent, Plane};

use super::compass_support::{app_at, drag, drawing, plane, square, view, FRAME};
use super::support::{rejects, SLICE_FRAME};
use crate::app::App;
use crate::explore::compass::{
    self, cube_width, drawn_normal, latent_normal, normal_on_screen, Iso, Reading, P3, VIEW_DIR,
};
use crate::explore::manifold_view::tilt::Angles;
use crate::explore::manifold_view::{add, dot, Touch, View, Z0_HALF_RANGE};
use crate::headless::Headless;
use crate::side::MockSide;

/// Half the slice's range: the move a drag of half the cube's width makes.
const HALF_RANGE: f64 = Z0_HALF_RANGE;

/// The app on the plane tilted by `(τ₁, τ₂)` through `z₀ = 0` on the frame, at rest, in slice mode.
fn slicing(t1: f64, t2: f64) -> (App<MockSide>, Headless) {
    let (mut app, h) = app_at(&plane(&view(t1, t2, 0.0, 1.0), [0.0; 3]));
    app.manifold.compass.mode = Touch::Slice;
    (app, h)
}

/// The cube's fit in the app on `h`.
fn iso(h: &Headless) -> Iso {
    Iso::fit(square(h))
}

/// The screen image, in points, of the target plane's point nearest the cube's centre: `n̂·p n̂`, `p` the point of
/// `z₀` on the frame, unclamped, the plane the contract holds (R-465 N4).
fn nearest_image(iso: &Iso, plane: &Plane) -> [f64; 2] {
    let v = View::of(plane);
    let n = drawn_normal(&v).expect("a drawn plane");
    let offset = dot3(n, compass::point_of(&plane.z0, &v));
    iso.at64(n.map(|x| x * offset))
}

fn dot3(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// `ŝ` of the plane tilted by `(τ₁, τ₂)`, from the projection: the unit vector along `P(n̂) − P(0)`.
fn s_hat(iso: &Iso, t1: f64, t2: f64) -> [f64; 2] {
    let n = drawn_normal(&View::of(&plane(&view(t1, t2, 0.0, 1.0), [0.0; 3]))).expect("a plane");
    let ([x, y], [cx, cy]) = (iso.at64(n), iso.at64([0.0; 3]));
    let len = (x - cx).hypot(y - cy);
    [(x - cx) / len, (y - cy) / len]
}

/// `k` points along `dir`.
fn along(dir: [f64; 2], k: f64) -> Vec2 {
    egui::vec2((dir[0] * k) as f32, (dir[1] * k) as f32)
}

/// A drag by `delta` on the cube, from its centre, in the app's mode; the plane before and after.
fn dragged(app: &mut App<MockSide>, h: &mut Headless, delta: Vec2) -> (Plane, Plane) {
    let before = app.snapshot().sim.plane.clone();
    let from = square(h).center();
    drag(h, app, from, delta);
    (before, app.snapshot().sim.plane.clone())
}

/// How far `after`'s `z₀` lies from `before`'s along the unit latent `normal`, and how far off that line.
fn moved_along(before: &Plane, after: &Latent, normal: &Latent) -> (f64, f64) {
    let delta = add(after, -1.0, &before.z0);
    let along = dot(&delta, normal);
    let off = add(&delta, -along, normal);
    (along, dot(&off, &off).sqrt())
}

/// That the drag moved `z₀` by `want` along `normal` (both finite), nothing off it, within the pointer's f32 precision.
fn check_moved(before: &Plane, after: &Latent, normal: &Latent, want: f64, what: &str) {
    assert!(
        after.iter().all(|x| x.is_finite()),
        "{what}: z₀ not finite: {after:?}"
    );
    let (along, off) = moved_along(before, after, normal);
    assert!(
        (along - want).abs() < 1e-5 && off < 1e-9,
        "{what}: z₀ moved {along} along the normal (off it {off}), not {want}"
    );
}

/// That the image of the target plane's nearest point moved along `+dir` on the screen, with the pointer: a positive
/// step along it and none across it.
fn check_image_follows(iso: &Iso, before: &Plane, after: &Plane, dir: [f64; 2], what: &str) {
    let ([bx, by], [ax, ay]) = (nearest_image(iso, before), nearest_image(iso, after));
    let (dx, dy) = (ax - bx, ay - by);
    let (on, across) = (dx * dir[0] + dy * dir[1], dx * dir[1] - dy * dir[0]);
    assert!(
        on > 1.0 && across.abs() < 1e-3,
        "{what}: the plane's image moved ({dx}, {dy}), {on} along ŝ and {across} across it"
    );
}

/// The `z₀` a GUI reading the slice drag by `delta` from `before` would send, its travel read by `travel`, at the rate
/// along the normal.
fn read_by(
    iso: &Iso,
    before: &Plane,
    delta: Vec2,
    normal: &Latent,
    travel: impl Fn(f64, f64) -> f64,
) -> Latent {
    let t = travel(f64::from(delta.x), f64::from(delta.y));
    add(&before.z0, t / cube_width(iso.s) * 2.0 * HALF_RANGE, normal)
}

/// R-451 B3 as written: the horizontal travel alone.
fn horizontal(x: f64, _: f64) -> f64 {
    x
}

#[test]
fn mock_compass_slice_follows_pointer_untilted() {
    let (mut app, mut h) = slicing(0.0, 0.0);
    let iso = iso(&h);
    let s = s_hat(&iso, 0.0, 0.0);
    assert!(
        (s[0] + 3f64.sqrt() / 2.0).abs() < 1e-12 && (s[1] - 0.5).abs() < 1e-12,
        "ŝ = (−√3/2, ½): {s:?}"
    );
    let half = 3f64.sqrt() * iso.s;
    let mut e_d: Latent = [0.0; 8];
    e_d[FRAME.2] = 1.0;
    // Along ŝ: half the range along +e_d, the image moving along +ŝ with the pointer.
    let delta = along(s, half);
    let (before, after) = dragged(&mut app, &mut h, delta);
    check_moved(&before, &after.z0, &e_d, HALF_RANGE, "along ŝ");
    check_image_follows(&iso, &before, &after, s, "along ŝ");
    let b3 = read_by(&iso, &before, delta, &e_d, horizontal);
    rejects("the horizontal reading, against the pointer", || {
        check_moved(&before, &b3, &e_d, HALF_RANGE, "along ŝ, read horizontally")
    });
    rejects("the horizontal reading's image", || {
        let mut moved = before.clone();
        moved.z0 = b3;
        check_image_follows(&iso, &before, &moved, s, "along ŝ, read horizontally")
    });
    // Across ŝ, the same length: nothing.
    let (before, after) = dragged(&mut app, &mut h, along([s[1], -s[0]], half));
    check_moved(&before, &after.z0, &e_d, 0.0, "across ŝ");
    // Straight right: along −e_d, the image moving right and up.
    let (before, after) = dragged(&mut app, &mut h, egui::vec2(half as f32, 0.0));
    let (moved, _) = moved_along(&before, &after.z0, &e_d);
    assert!(moved < -0.1, "a drag right moves along −e_d: {moved}");
    let ([bx, by], [ax, ay]) = (nearest_image(&iso, &before), nearest_image(&iso, &after));
    assert!(
        ax > bx && ay < by,
        "the image moves right and up: ({bx}, {by}) → ({ax}, {ay})"
    );
    rejects("a drag right read along +e_d", || {
        check_moved(&before, &after.z0, &e_d, -moved, "right")
    });
}

#[test]
fn mock_compass_slice_follows_pointer_tilted() {
    let (mut app, mut h) = slicing(30.0, 0.0);
    let iso = iso(&h);
    let tilted = app.snapshot().sim.plane.clone();
    // The tilt_rect pose: n̂ ∝ (−½, 0, √3/2), and ŝ along (−1.183, 0.183).
    let n = drawn_normal(&View::of(&tilted)).expect("a plane");
    assert!(
        (n[0] + 0.5).abs() < 1e-12
            && n[1].abs() < 1e-12
            && (n[2] - 3f64.sqrt() / 2.0).abs() < 1e-12,
        "{n:?}"
    );
    let s = s_hat(&iso, 30.0, 0.0);
    let want = {
        let (x, y) = (-1.183, 0.183);
        let len = f64::hypot(x, y);
        [x / len, y / len]
    };
    assert!(
        (s[0] - want[0]).abs() < 1e-3 && (s[1] - want[1]).abs() < 1e-3,
        "ŝ {s:?} not along (−1.183, 0.183)"
    );
    let normal = latent_normal(&tilted, &SLICE_FRAME).expect("a plane");
    let half = 3f64.sqrt() * iso.s;
    let delta = along(s, half);
    let (before, after) = dragged(&mut app, &mut h, delta);
    check_moved(&before, &after.z0, &normal, HALF_RANGE, "tilted, along ŝ");
    check_image_follows(&iso, &before, &after, s, "tilted, along ŝ");
    let b3 = read_by(&iso, &before, delta, &normal, horizontal);
    rejects("the horizontal reading, against the pointer", || {
        check_moved(
            &before,
            &b3,
            &normal,
            HALF_RANGE,
            "tilted, read horizontally",
        )
    });
    // Across ŝ: nothing.
    let across = along([s[1], -s[0]], half);
    let (before, after) = dragged(&mut app, &mut h, across);
    check_moved(&before, &after.z0, &normal, 0.0, "tilted, across ŝ");
    // The untilted direction P(e_d) in place of the tilted ŝ reads the drag across ŝ as a move.
    let untilted = s_hat(&iso, 0.0, 0.0);
    let read_untilted = read_by(&iso, &before, across, &normal, |x, y| {
        x * untilted[0] + y * untilted[1]
    });
    rejects("ŝ taken as the untilted P(e_d)", || {
        check_moved(
            &before,
            &read_untilted,
            &normal,
            0.0,
            "tilted, across ŝ, read along P(e_d)",
        )
    });
}

#[test]
fn mock_compass_slice_follows_pointer_at_the_view_axis() {
    let (mut app, mut h) = slicing(-45.0, -45.0);
    let iso = iso(&h);
    let start = app.snapshot().sim.plane.clone();
    let v = View::of(&start);
    let n = drawn_normal(&v).expect("a plane");
    let [x, y] = normal_on_screen(n);
    assert!(
        x.hypot(y) < compass::MIN_NORMAL_ON_SCREEN,
        "the normal along the view axis"
    );
    assert!(matches!(compass::reading(&v), Reading::Vertical(_)));
    // The side of the normal facing the viewer.
    let facing = if dot3(n, VIEW_DIR) > 0.0 { 1.0 } else { -1.0 };
    let toward = latent_normal(&start, &SLICE_FRAME)
        .expect("a plane")
        .map(|c| c * facing);
    let half = 3f64.sqrt() * iso.s;
    let (before, after) = dragged(&mut app, &mut h, egui::vec2(0.0, -half as f32));
    check_moved(
        &before,
        &after.z0,
        &toward,
        HALF_RANGE,
        "up at the view axis",
    );
    rejects("a GUI that sends no edit", || {
        check_moved(&before, &before.z0, &toward, HALF_RANGE, "no edit")
    });
    rejects("a GUI that sends a non-finite edit", || {
        check_moved(
            &before,
            &[f64::NAN; 8],
            &toward,
            HALF_RANGE,
            "a non-finite edit",
        )
    });
    let (before, after) = dragged(&mut app, &mut h, egui::vec2(half as f32, 0.0));
    check_moved(
        &before,
        &after.z0,
        &toward,
        0.0,
        "horizontal at the view axis",
    );
}

/// The tilt-mode drag's rates, REQ-GUI-193's checks, unchanged by R-465: a drag across the full width sets `τ₁` to the
/// clamp, +90°; a quarter of the height up raises `τ₂` by 45°.
#[test]
fn mock_compass_slice_follows_pointer_tilt_mode_unchanged() {
    let (mut app, mut h) = app_at(&plane(&view(0.0, 0.0, 0.0, 1.0), [0.0; 3]));
    assert_eq!(app.manifold.compass.mode, Touch::Tilt);
    let sq = square(&h);
    let check = |a: Angles, want: (f64, f64), what: &str| {
        assert!(
            (a.tau1 - want.0).abs() < 1e-3 && (a.tau2 - want.1).abs() < 1e-3,
            "{what}: ({}, {}), not {want:?}",
            a.tau1,
            a.tau2
        );
    };
    let (_, after) = dragged(&mut app, &mut h, egui::vec2(sq.width(), 0.0));
    let wide = Angles::of(&View::of(&after));
    check(wide, (90.0, 0.0), "the full width");
    rejects("an unclamped drag", || {
        check(wide, (180.0, 0.0), "the full width")
    });
    let (_, after) = dragged(&mut app, &mut h, egui::vec2(0.0, -sq.height() / 4.0));
    let up = Angles::of(&View::of(&after));
    check(up, (90.0, 45.0), "a quarter of the height up");
    rejects("a drag up read as down", || check(up, (90.0, -45.0), "up"));
    assert!(
        drawing(&h, &app)
            .readout
            .iter()
            .any(|l| l.starts_with("mode tilt")),
        "still tilt mode"
    );
}
