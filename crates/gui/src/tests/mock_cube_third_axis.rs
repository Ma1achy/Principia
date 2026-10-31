//! `mock_cube_third_axis` (REQ-GUI-209; render_gui_spec § "The compass: look and views (R-451, R-452)"; R-459 G5 to G8): in
//! Tilt mode the third axis shows what the gesture tilts into, in its colour, the cube drawing that gesture's plane
//! exactly; the readout names both targets; with both angles set into different targets a badge switches to the Plücker
//! view. Each check also runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{self, vec2, Key, Modifiers, Pos2};
use engine::contract::sim_config::{Plane, SliceFrame};

use super::compass_support::{
    app_at, check_painted, compass_shapes, drawing, plane, square, texts as painted, view,
};
use super::support::{click_scope, focus, keys_on, press, rejects};
use crate::app::App;
use crate::explore::compass::views::CompassView;
use crate::explore::compass::{
    self, badge_rect, badge_shown, gesture_view, is_set, label_colour, line_parts, third_axis,
    Drag, Drawing, Gesture, Reading, Shows, ANGLE_SET, BADGE, GESTURE_TAU1, GESTURE_TAU2, P3,
};
use crate::explore::manifold_view::{basis_edit, frame_edit, Touch, View, GAMMA, TAU1, TAU2};
use crate::explore::{COMPASS, COMPASS_BADGE, COMPASS_PLUCKER};
use crate::headless::Headless;
use crate::side::MockSide;

const Z_ALPHA: usize = 0;
const Z_BETA: usize = 1;
const Z_Q2: usize = 4;
const Z_Q3: usize = 5;
const Z_MU1: usize = 6;

/// The view at `(τ₁, τ₂)` degrees into `X = z_q3`, `Y = z_μ1`, the normal `z_q3`.
fn two(tau1: f64, tau2: f64) -> View {
    View {
        y: Z_MU1,
        ..view(tau1, tau2, 0.0, 1.0)
    }
}

/// The slice frame of `v`.
fn frame_of(v: &View) -> SliceFrame {
    v.frame()
}

/// The app at rest on `v`, with `z₀ = 0` on every dimension the cube can draw.
fn app_on(v: &View) -> (App<MockSide>, Headless) {
    let mut p = plane(v, [0.0, 0.0, 0.0]);
    p.z0[v.x] = 0.0;
    p.z0[v.y] = 0.0;
    let (mut app, mut h) = app_at(&p);
    app.set_field(frame_edit(frame_of(v), None));
    follow(&mut h, &mut app);
    (app, h)
}

fn follow(h: &mut Headless, app: &mut App<MockSide>) {
    let _ = h.frame(app, Vec::new());
    super::compass_support::settle(h, app);
}

/// The third axis's text.
fn third(h: &Headless, app: &App<MockSide>) -> String {
    drawing(h, app).labels[1].text.clone()
}

/// Asserts the readout names both targets, each with its angle.
fn check_names_both(lines: &[String], x: &str, y: &str) {
    let all = lines.join(" | ");
    assert!(all.contains(&format!("τ₁ → {x} ")), "τ₁'s target: {all}");
    assert!(all.contains(&format!("τ₂ → {y} ")), "τ₂'s target: {all}");
}

/// Asserts the rule on a view: what the third axis shows, by the table of the ruling's cases.
fn check_rule(rule: impl Fn(&View, Touch, Option<Gesture>) -> Shows) {
    let t = Some(Gesture::Tau1);
    let u = Some(Gesture::Tau2);
    for (tau1, tau2, mode, active, want) in [
        (0.0, 0.0, Touch::Slice, None, Shows::Normal),
        (20.0, 15.0, Touch::Slice, None, Shows::Normal),
        (20.0, 15.0, Touch::Slice, u, Shows::Normal),
        (0.0, 0.0, Touch::Tilt, None, Shows::Normal),
        (20.0, 0.0, Touch::Tilt, None, Shows::X),
        (0.0, 15.0, Touch::Tilt, None, Shows::Y),
        (20.0, 15.0, Touch::Tilt, None, Shows::X),
        (20.0, 15.0, Touch::Tilt, u, Shows::Y),
        (0.0, 15.0, Touch::Tilt, t, Shows::X),
        (0.0, 0.0, Touch::Tilt, u, Shows::Y),
        (20.0, 15.0, Touch::Tilt, Some(Gesture::Slice), Shows::X),
    ] {
        assert_eq!(
            rule(&two(tau1, tau2), mode, active),
            want,
            "({tau1}, {tau2}) {mode:?} {active:?}"
        );
    }
}

#[test]
fn mock_cube_third_axis_follows_the_rule_in_each_case() {
    check_rule(third_axis);
    rejects(
        "a third axis that stays on the slice normal in Tilt mode",
        || check_rule(|_, _, _| Shows::Normal),
    );
    // On the app, with X = z_q3, Y = z_μ1.
    let (mut app, mut h) = app_on(&two(0.0, 0.0));
    assert_eq!(third(&h, &app), "D z_q3", "Tilt, both 0: the normal");
    app.manifold.compass.mode = Touch::Slice;
    follow(&mut h, &mut app);
    assert_eq!(third(&h, &app), "D z_q3", "Slice: D");
    app.manifold.compass.mode = Touch::Tilt;
    for (tau1, tau2, want) in [
        (20.0, 0.0, "X z_q3"),
        (0.0, 15.0, "Y z_μ1"),
        (20.0, 15.0, "X z_q3"),
    ] {
        app.set_field(basis_edit(two(tau1, tau2).basis()));
        follow(&mut h, &mut app);
        assert_eq!(third(&h, &app), want, "({tau1}, {tau2})");
        check_names_both(&drawing(&h, &app).readout, "z_q3", "z_μ1");
    }
    app.manifold.compass.mode = Touch::Slice;
    follow(&mut h, &mut app);
    assert_eq!(third(&h, &app), "D z_q3");
    check_names_both(&drawing(&h, &app).readout, "z_q3", "z_μ1");
    rejects("a readout naming one target", || {
        check_names_both(&["τ₁ → z_q3 +20.0°".to_owned()], "z_q3", "z_μ1")
    });
}

#[test]
fn mock_cube_third_axis_labels_at_rest_name_the_three_edges() {
    let iso = compass::Iso::fit(egui::Rect::from_min_size(
        egui::pos2(0.0, 0.0),
        vec2(172.0, 172.0),
    ));
    let names = |tau1: f64, mode: Touch| -> Vec<String> {
        compass::labels(&iso, &two(tau1, 0.0), mode)
            .into_iter()
            .map(|l| l.text)
            .collect()
    };
    assert_eq!(names(20.0, Touch::Tilt), ["H z_α", "X z_q3", "V z_β"]);
    assert_eq!(names(20.0, Touch::Slice), ["H z_α", "D z_q3", "V z_β"]);
    assert_eq!(names(0.0, Touch::Tilt), ["H z_α", "D z_q3", "V z_β"]);
}

#[test]
fn mock_cube_third_axis_an_angle_is_set_past_rounding() {
    assert!(!is_set(0.0));
    assert!(!is_set(ANGLE_SET), "at the cut, not set");
    assert!(!is_set(-ANGLE_SET));
    assert!(is_set(ANGLE_SET * 1.5));
    assert!(is_set(-ANGLE_SET * 1.5));
    let at = |t1: f64, t2: f64| View {
        tau1: t1,
        tau2: t2,
        ..two(0.0, 0.0)
    };
    assert_eq!(
        third_axis(&at(ANGLE_SET, 0.0), Touch::Tilt, None),
        Shows::Normal
    );
    assert_eq!(
        third_axis(&at(0.0, ANGLE_SET), Touch::Tilt, None),
        Shows::Normal
    );
    assert!(!badge_shown(&at(1.0, ANGLE_SET)));
    assert!(!badge_shown(&at(ANGLE_SET, 1.0)));
    assert!(badge_shown(&at(1.0, 1.0)));
    assert!(!badge_shown(&View {
        y: Z_Q3,
        ..at(1.0, 1.0)
    }));
}

/// A pointer press at `at`.
fn button(at: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn mock_cube_third_axis_during_a_slider_drag_reads_its_target_and_back() {
    let (mut app, mut h) = app_on(&two(20.0, 0.0));
    assert_eq!(third(&h, &app), "X z_q3");
    let rail = app.keyboard.place_of(TAU2).expect("τ₂ placed").rect;
    let from = rail.left_center() + vec2(rail.width() * 0.3, 0.0);
    let _ = h.frame(&mut app, vec![egui::Event::PointerMoved(from)]);
    let _ = h.frame(&mut app, vec![button(from, true)]);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.manifold.compass.slider, Some(Gesture::Tau2));
    assert_eq!(third(&h, &app), "Y z_μ1", "held: Y");
    let _ = h.frame(
        &mut app,
        vec![egui::Event::PointerMoved(from + vec2(12.0, 0.0))],
    );
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(third(&h, &app), "Y z_μ1", "dragged: Y");
    let d = drawing(&h, &app);
    assert_eq!(d.third, Shows::Y);
    // Its label's colour is the active gesture's, painted so; H and V the theme's.
    let shapes = compass_shapes(&mut h, &mut app);
    check_painted(&shapes, &drawing(&h, &app));
    let found = painted(&shapes);
    let colour = |text: &str| found.iter().find(|t| t.0 == text).map(|t| t.3);
    assert_eq!(colour("Y z_μ1"), Some(GESTURE_TAU2));
    let text = label_colour(&egui::Visuals::dark(), false);
    assert_eq!(colour("H z_α"), Some(text));
    assert_eq!(colour("V z_β"), Some(text));
    rejects("the label in τ₁'s colour while τ₂ is dragged", || {
        assert_eq!(colour("Y z_μ1"), Some(GESTURE_TAU1))
    });
    let _ = h.frame(&mut app, vec![button(from + vec2(12.0, 0.0), false)]);
    let _ = h.frame(&mut app, Vec::new());
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.manifold.compass.slider, None, "released");
    assert_eq!(third(&h, &app), "X z_q3", "back to X when the drag ends");
}

#[test]
fn mock_cube_third_axis_during_a_tau1_slider_drag_reads_x_and_a_held_gamma_nothing() {
    let (mut app, mut h) = app_on(&two(0.0, 15.0));
    assert_eq!(third(&h, &app), "Y z_μ1");
    for (id, gesture) in [(TAU1, Some(Gesture::Tau1)), (GAMMA, None)] {
        let rail = app.keyboard.place_of(id).expect("placed").rect;
        let from = rail.left_center() + vec2(rail.width() * 0.3, 0.0);
        let _ = h.frame(&mut app, vec![egui::Event::PointerMoved(from)]);
        let _ = h.frame(&mut app, vec![button(from, true)]);
        let _ = h.frame(&mut app, Vec::new());
        assert_eq!(app.manifold.compass.slider, gesture, "{id} held");
        let _ = h.frame(&mut app, vec![button(from, false)]);
        let _ = h.frame(&mut app, Vec::new());
        // The press set the value where it landed, a real gesture: the rest rule reads the angles next.
        assert_eq!(app.manifold.compass.slider, None, "{id} released");
        let tau1 = two(0.0, 15.0);
        app.set_field(basis_edit(tau1.basis()));
        follow(&mut h, &mut app);
        assert_eq!(third(&h, &app), "Y z_μ1", "{id}: back at rest");
    }
}

#[test]
fn mock_cube_third_axis_a_compass_drag_reads_the_angle_moved_further() {
    let (mut app, mut h) = app_on(&two(0.0, 0.0));
    let sq = square(&h);
    let from = sq.center();
    // τ₁ by +10°, τ₂ by +30°: right, and up.
    let delta = vec2(sq.width() * 10.0 / 180.0, -sq.height() * 30.0 / 180.0);
    let _ = h.frame(&mut app, vec![egui::Event::PointerMoved(from)]);
    let _ = h.frame(&mut app, vec![button(from, true)]);
    for i in 1..=4 {
        let _ = h.frame(
            &mut app,
            vec![egui::Event::PointerMoved(from + delta * (i as f32 / 4.0))],
        );
    }
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(third(&h, &app), "Y z_μ1", "τ₂ moved further");
    assert_eq!(drawing(&h, &app).third.gesture(), Gesture::Tau2);
    let drag = app.manifold.compass.drag.expect("dragging");
    assert_eq!(drag.gesture(), Some(Gesture::Tau2));
    assert!(
        (drag.tau.0 - 10.0).abs() < 0.5 && (drag.tau.1 - 30.0).abs() < 0.5,
        "{drag:?}"
    );
    let _ = h.frame(&mut app, vec![button(from + delta, false)]);
    follow(&mut h, &mut app);
    // At rest both are set: X, by the rest rule.
    assert_eq!(third(&h, &app), "X z_q3");
    rejects("the drag's reading kept at rest", || {
        assert_eq!(third(&h, &app), "Y z_μ1")
    });
}

#[test]
fn mock_cube_third_axis_a_drags_gesture_is_the_further_tau1_on_a_tie() {
    let plane = Plane {
        z0: [0.0; 8],
        q1: two(0.0, 0.0).basis().0,
        q2: two(0.0, 0.0).basis().1,
    };
    let f = frame_of(&two(0.0, 0.0));
    let mut d = Drag::start(Touch::Tilt, &plane, &f);
    assert_eq!(d.gesture(), Some(Gesture::Tau1), "a tie at the start");
    d.tau = (10.0, -10.0);
    assert_eq!(d.gesture(), Some(Gesture::Tau1), "a tie, opposite signs");
    d.tau = (10.0, -10.5);
    assert_eq!(d.gesture(), Some(Gesture::Tau2));
    d.tau = (-10.5, 10.0);
    assert_eq!(d.gesture(), Some(Gesture::Tau1));
    d.from = (5.0, 5.0);
    d.tau = (5.0, 6.0);
    assert_eq!(d.gesture(), Some(Gesture::Tau2), "since it began");
    d.tau = (7.0, 6.0);
    assert_eq!(d.gesture(), Some(Gesture::Tau1), "since it began");
    // The travel, not the angle: τ₁ far from 0 and still, τ₂ moved a little.
    d.from = (-20.0, 0.0);
    d.tau = (-20.0, 5.0);
    assert_eq!(d.gesture(), Some(Gesture::Tau2), "τ₁ has not moved");
    let slicing = Drag::start(Touch::Slice, &plane, &f);
    assert_eq!(slicing.gesture(), None, "a slice is no tilt");
}

/// The polygon of `n·x = 0` through the cube, by the plane's own form: `q₁ = (cos τ, 0, sin τ)`, `q₂ = (0, 1, 0)` cuts the
/// cube at `x = ±1, z = ±tan τ`, `y = ±1`, a rectangle; apart from the code under test (R-451 B8).
fn independent_rectangle(tau1_degrees: f64) -> Vec<P3> {
    let t = tau1_degrees.to_radians().tan();
    vec![
        [-1.0, -1.0, -t],
        [1.0, -1.0, t],
        [1.0, 1.0, t],
        [-1.0, 1.0, -t],
    ]
}

/// The same for `q₁ = (1, 0, 0)`, `q₂ = (0, cos τ, sin τ)`.
fn independent_rectangle_v(tau2_degrees: f64) -> Vec<P3> {
    let t = tau2_degrees.to_radians().tan();
    vec![
        [-1.0, -1.0, -t],
        [-1.0, 1.0, t],
        [1.0, 1.0, t],
        [1.0, -1.0, -t],
    ]
}

/// Asserts `drawing`'s polygon holds exactly the points `want`, within 1e-12, in any order round.
fn check_polygon(d: &Drawing, want: &[P3]) {
    let points = &d.slice.as_ref().expect("a slice").points;
    assert_eq!(points.len(), want.len(), "{points:?}");
    for w in want {
        assert!(
            points
                .iter()
                .any(|p| (0..3).all(|k| (p[k] - w[k]).abs() < 1e-12)),
            "{w:?} not in {points:?}"
        );
    }
}

#[test]
fn mock_cube_third_axis_the_badge_case_draws_the_gestures_plane_and_switches_to_plucker() {
    let (mut app, mut h) = app_on(&two(20.0, 15.0));
    let d = drawing(&h, &app);
    assert_eq!(d.third, Shows::X);
    check_polygon(&d, &independent_rectangle(20.0));
    assert!(d.badge, "both set, X ≠ Y");
    check_names_both(&d.readout, "z_q3", "z_μ1");
    rejects("the whole plane drawn in the one cube", || {
        let whole = compass::slice_drawing(
            &app.snapshot().sim.plane.z0,
            &View {
                y: Z_Q3,
                ..two(20.0, 15.0)
            },
        );
        let mut wrong = d.clone();
        wrong.slice = whole;
        check_polygon(&wrong, &independent_rectangle(20.0))
    });
    let _ = h.frame(&mut app, Vec::new());
    assert!(
        painted(&compass_shapes(&mut h, &mut app))
            .iter()
            .any(|t| t.0 == BADGE),
        "painted"
    );
    let placed = app.keyboard.place_of(COMPASS_BADGE).expect("placed").rect;
    let sq = square(&h);
    assert!((placed.center() - badge_rect(sq).center()).length() <= 1.0);
    assert!(
        (placed.right_top() - sq.right_top()).length() <= 1.0,
        "the top-right corner"
    );
    // Clear of the drawing: of every projected cube corner, and of the labels.
    for corner in compass::CORNERS {
        assert!(!placed.contains(d.iso.at(corner)), "{corner:?} under it");
    }
    click_scope(&mut h, &mut app, COMPASS_BADGE);
    assert_eq!(app.manifold.compass.view, CompassView::Plucker, "the click");
    assert_eq!(
        app.snapshot().sim.plane.q1,
        two(20.0, 15.0).basis().0,
        "edits nothing"
    );
    rejects("a badge that left the view", || {
        assert_eq!(app.manifold.compass.view, CompassView::Cube)
    });
}

#[test]
fn mock_cube_third_axis_enter_on_the_badge_switches_and_lands_on_the_view_button() {
    let (mut app, mut h) = app_on(&two(20.0, 15.0));
    for key in [Key::Tab; 5] {
        press(&mut h, &mut app, key, Modifiers::NONE);
    }
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    for _ in 0..10 {
        press(&mut h, &mut app, Key::ArrowRight, Modifiers::NONE);
    }
    assert_eq!(focus(&app), [COMPASS, COMPASS_BADGE]);
    keys_on(&mut h, &mut app);
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    assert_eq!(app.manifold.compass.view, CompassView::Plucker);
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(focus(&app), [COMPASS, COMPASS_PLUCKER]);
}

#[test]
fn mock_cube_third_axis_with_one_target_no_badge_and_the_whole_plane() {
    let v = view(20.0, 15.0, 0.0, 1.0);
    assert_eq!((v.x, v.y), (Z_Q3, Z_Q3));
    let (mut app, mut h) = app_on(&v);
    let d = drawing(&h, &app);
    assert!(!d.badge);
    assert_eq!(d.third, Shows::X);
    // R-451 B2's plane: the normal of (cos 20, 0, sin 20) × (0, cos 15, sin 15).
    let (s1, c1) = 20.0_f64.to_radians().sin_cos();
    let (s2, c2) = 15.0_f64.to_radians().sin_cos();
    let n = [-s1 * c2, -c1 * s2, c1 * c2];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    let cut = d.slice.as_ref().expect("a slice").cut;
    for (got, want) in cut.normal.iter().zip(n) {
        assert!((got - want / len).abs() < 1e-12, "{:?}", cut.normal);
    }
    assert_eq!(app.keyboard.place_of(COMPASS_BADGE), None, "never placed");
    assert!(
        !painted(&compass_shapes(&mut h, &mut app))
            .iter()
            .any(|t| t.0 == BADGE),
        "not painted"
    );
    check_names_both(&d.readout, "z_q3", "z_q3");
    rejects("a badge shown with X = Y", || assert!(d.badge));
}

#[test]
fn mock_cube_third_axis_each_gesture_draws_its_own_frame() {
    // τ₂ alone, into Y: the plane in (e_a, e_b, e_μ1).
    let (app, h) = app_on(&two(0.0, 15.0));
    let d = drawing(&h, &app);
    assert_eq!(d.third, Shows::Y);
    check_polygon(&d, &independent_rectangle_v(15.0));
    assert!(!d.badge);
    // The rule's arms, with γ kept.
    let g = View {
        gamma: 0.3,
        ..two(20.0, 15.0)
    };
    assert_eq!(
        gesture_view(&g, Touch::Tilt, None),
        View {
            d: Z_Q3,
            tau2: 0.0,
            ..g
        },
        "X, the other angle 0"
    );
    assert_eq!(
        gesture_view(&g, Touch::Tilt, Some(Gesture::Tau2)),
        View {
            d: Z_MU1,
            tau1: 0.0,
            ..g
        },
        "Y, the other angle 0"
    );
    let one = View {
        x: Z_Q2,
        y: Z_Q2,
        ..g
    };
    assert_eq!(
        gesture_view(&one, Touch::Tilt, None),
        View { d: Z_Q2, ..one },
        "X = Y: the whole plane, in its frame"
    );
    assert_eq!(
        gesture_view(&one, Touch::Tilt, Some(Gesture::Tau2)),
        View { d: Z_Q2, ..one },
        "Y = X: the whole plane, in its frame"
    );
    // At rest with both 0, the normal's frame.
    let zero = two(0.0, 0.0);
    assert_eq!(gesture_view(&zero, Touch::Tilt, None), zero);
    // Slice mode: B2's plane in (e_a, e_b, e_D), but with the badge the rule's at rest, whatever is dragged.
    let lone = two(20.0, 0.0);
    assert_eq!(gesture_view(&lone, Touch::Slice, None), lone);
    assert_eq!(
        gesture_view(&g, Touch::Slice, Some(Gesture::Tau2)),
        gesture_view(&g, Touch::Tilt, None),
    );
    let one_slice = View {
        x: Z_Q3,
        y: Z_Q3,
        ..g
    };
    assert_eq!(gesture_view(&one_slice, Touch::Slice, None), one_slice);
}

#[test]
fn mock_cube_third_axis_a_slice_drag_with_the_badge_moves_along_the_drawn_normal() {
    let v = View {
        d: Z_Q2,
        ..two(20.0, 15.0)
    };
    let (q1, q2) = v.basis();
    let plane = Plane {
        z0: [0.0; 8],
        q1,
        q2,
    };
    let n = compass::latent_normal(&plane, &v.frame()).expect("a plane");
    let (s, c) = 20.0_f64.to_radians().sin_cos();
    assert!((n[Z_ALPHA] + s).abs() < 1e-12, "{n:?}");
    assert!((n[Z_Q3] - c).abs() < 1e-12, "in the frame of X: {n:?}");
    assert_eq!(n[Z_Q2], 0.0, "not the normal's frame: {n:?}");
    assert_eq!(n[Z_BETA], 0.0);
    // The drag reads along that normal's screen image, not the (a, b, D) plane's.
    let d = Drag::start(Touch::Slice, &plane, &v.frame());
    let want = compass::reading_of([-s, 0.0, c]);
    assert_eq!(d.reading, want);
    assert!(matches!(want, Reading::Along(_)));
}

#[test]
fn mock_cube_third_axis_no_plane_is_read_of_the_plane_drawn() {
    // τ₁ = 90° into X ≠ D: the cube draws (e_a, e_b, e_X), where it has a plane; in D's frame it has none.
    let v = View {
        d: Z_Q2,
        ..two(90.0, 0.0)
    };
    let tilt = compass::readout(&[0.0; 8], &v, Touch::Tilt);
    assert!(!tilt.iter().any(|l| l.starts_with("no plane")), "{tilt:?}");
    let slice = compass::readout(&[0.0; 8], &v, Touch::Slice);
    assert!(slice.iter().any(|l| l.starts_with("no plane")), "{slice:?}");
    // With the gesture on Y, at the corner of a plane the frame draws, no line either.
    let dragged = compass::readout_active(&[0.0; 8], &v, Touch::Tilt, Some(Gesture::Tau2));
    assert!(
        !dragged.iter().any(|l| l.starts_with("no plane")),
        "{dragged:?}"
    );
}

#[test]
fn mock_cube_third_axis_the_readouts_line_takes_the_gestures_colour() {
    let on = |line: &str, g: Gesture| -> Vec<String> {
        line_parts(line, g, "z_q3")
            .into_iter()
            .filter(|(_, active)| *active)
            .map(|(part, _)| part)
            .collect()
    };
    let targets = "τ₁ → z_q3 +20.0° · τ₂ → z_μ1 +15.0°";
    assert_eq!(on(targets, Gesture::Tau1), ["τ₁ → z_q3 +20.0° · "]);
    assert_eq!(on(targets, Gesture::Tau2), ["τ₂ → z_μ1 +15.0°"]);
    assert!(on(targets, Gesture::Slice).is_empty());
    assert_eq!(
        on("mode slice · plane ⟂ z_q3", Gesture::Slice),
        ["plane ⟂ z_q3"]
    );
    assert_eq!(
        on("z_q3 = +0.25 of range", Gesture::Slice),
        ["z_q3 = +0.25 of range"]
    );
    assert!(on("z_μ1 = +0.25 of range", Gesture::Slice).is_empty());
    // The parts rebuild the line.
    let whole: String = line_parts(targets, Gesture::Tau1, "z_q3")
        .into_iter()
        .map(|(p, _)| p)
        .collect();
    assert_eq!(whole, targets);
    rejects("a readout line in no colour", || {
        assert!(on(targets, Gesture::Tau1).is_empty())
    });
    // On the frame: the target's part in its gesture's colour, the rest not.
    let (mut app, mut h) = app_on(&two(20.0, 15.0));
    let shapes = compass_shapes(&mut h, &mut app);
    let colours: Vec<egui::Color32> = shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) if t.galley.text().starts_with("τ₁ →") => Some(
                t.galley
                    .job
                    .sections
                    .iter()
                    .map(|s| s.format.color)
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(colours.len(), 2, "two parts: {colours:?}");
    assert_eq!(colours[0], GESTURE_TAU1);
    assert_ne!(colours[1], GESTURE_TAU1);
}

#[test]
fn mock_cube_third_axis_the_capture_cases_set_their_angles_targets_and_mode() {
    use crate::capture::CompassCase;
    let parameters = [
        (CompassCase::Slice, (0.0, 0.0, 0.0), 0.25, Touch::Slice),
        (CompassCase::Tilt0, (0.0, 0.0, 0.0), 0.0, Touch::Tilt),
        (CompassCase::TiltRect, (30.0, 0.0, 0.0), 0.0, Touch::Tilt),
        (CompassCase::TiltHex, (45.0, 45.0, 0.0), 0.0, Touch::Tilt),
        (CompassCase::ThirdAxis, (0.0, 15.0, 0.0), 0.0, Touch::Tilt),
        (CompassCase::Badge, (20.0, 15.0, 0.0), 0.0, Touch::Tilt),
    ];
    for (case, angles, depth, mode) in parameters {
        assert_eq!(case.parameters(), (angles, depth, mode), "{case:?}");
    }
    // Each R-459 case leaves z₀ 0 on Y and picks Y = z_μ1, in the one edit that sets the basis.
    let mut app = super::support::mock_app();
    let mut h = super::support::headless();
    let _ = h.frame(&mut app, Vec::new());
    for (case, want) in [
        (CompassCase::ThirdAxis, two(0.0, 15.0)),
        (CompassCase::Badge, two(20.0, 15.0)),
    ] {
        let depth = app.snapshot().history.undo_depth;
        for edit in case.edits(&app.snapshot().sim) {
            app.set_field(edit);
        }
        follow(&mut h, &mut app);
        let sim = &app.snapshot().sim;
        assert_eq!(sim.slice.frame, want.frame(), "{case:?}'s frame");
        assert_eq!(sim.plane.z0[Z_MU1], 0.0, "{case:?}: z₀ on Y");
        assert_eq!(sim.plane.z0[Z_Q3], 0.0, "{case:?}: z₀ on X");
        let (q1, q2) = want.basis();
        assert_eq!((sim.plane.q1, sim.plane.q2), (q1, q2), "{case:?}'s basis");
        assert_eq!(app.snapshot().history.undo_depth, depth + 2, "{case:?}");
    }
    // An older case leaves the frame the chart's.
    let before = app.snapshot().sim.slice.frame;
    for edit in CompassCase::Tilt0.edits(&app.snapshot().sim) {
        app.set_field(edit);
    }
    follow(&mut h, &mut app);
    assert_eq!(app.snapshot().sim.slice.frame, before);
    rejects("a case that picks no target", || {
        assert_eq!(before, view(0.0, 0.0, 0.0, 1.0).frame())
    });
}
