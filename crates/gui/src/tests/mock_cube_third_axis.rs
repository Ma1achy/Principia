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

/// Asserts the third axis's text.
fn check_third(got: &str, want: &str, what: &str) {
    assert_eq!(got, want, "{what}");
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
    check_third(&third(&h, &app), "D z_q3", "Tilt, both 0: the normal");
    app.manifold.compass.mode = Touch::Slice;
    follow(&mut h, &mut app);
    check_third(&third(&h, &app), "D z_q3", "Slice: D");
    app.manifold.compass.mode = Touch::Tilt;
    let mut shown = Vec::new();
    for (tau1, tau2, want) in [
        (20.0, 0.0, "X z_q3"),
        (0.0, 15.0, "Y z_μ1"),
        (20.0, 15.0, "X z_q3"),
    ] {
        app.set_field(basis_edit(two(tau1, tau2).basis()));
        follow(&mut h, &mut app);
        let got = third(&h, &app);
        check_third(&got, want, &format!("({tau1}, {tau2})"));
        shown.push(got);
        check_names_both(&drawing(&h, &app).readout, "z_q3", "z_μ1");
    }
    app.manifold.compass.mode = Touch::Slice;
    follow(&mut h, &mut app);
    let sliced = third(&h, &app);
    check_third(&sliced, "D z_q3", "Slice, both set: D");
    check_names_both(&drawing(&h, &app).readout, "z_q3", "z_μ1");
    rejects("Tilt's X kept in Slice mode", || {
        check_third(&shown[2], "D z_q3", "Slice, both set: D")
    });
    rejects("τ₂ alone read as X", || {
        check_third(&shown[1], "X z_q3", "(0, 15)")
    });
    rejects("τ₁ alone read as the normal", || {
        check_third(&sliced, "X z_q3", "(20, 0)")
    });
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
    let with_x = ["H z_α", "X z_q3", "V z_β"];
    let with_d = ["H z_α", "D z_q3", "V z_β"];
    check_labels(&names(20.0, Touch::Tilt), with_x);
    check_labels(&names(20.0, Touch::Slice), with_d);
    check_labels(&names(0.0, Touch::Tilt), with_d);
    rejects("Slice mode's labels in Tilt with τ₁ set", || {
        check_labels(&names(20.0, Touch::Slice), with_x)
    });
    rejects("Tilt with τ₁ set's labels in Slice mode", || {
        check_labels(&names(20.0, Touch::Tilt), with_d)
    });
    rejects("both angles 0 read as X", || {
        check_labels(&names(0.0, Touch::Tilt), with_x)
    });
}

/// Asserts the three labels' texts, H, the third axis and V.
fn check_labels(got: &[String], want: [&str; 3]) {
    assert_eq!(got, want, "the labels");
}

#[test]
fn mock_cube_third_axis_an_angle_is_set_past_rounding() {
    check_set_cut(is_set);
    rejects("an angle set at the cut", || {
        check_set_cut(|a: f64| a.abs() >= ANGLE_SET)
    });
    rejects("only a positive angle set", || {
        check_set_cut(|a: f64| a > ANGLE_SET)
    });
    check_unset_at_cut(|v: &View| third_axis(v, Touch::Tilt, None));
    rejects("a third axis on X at the cut", || {
        check_unset_at_cut(|v: &View| {
            if v.tau1.abs() >= ANGLE_SET {
                Shows::X
            } else {
                Shows::Normal
            }
        })
    });
    rejects("a third axis on Y at the cut", || {
        check_unset_at_cut(|v: &View| {
            if v.tau2.abs() >= ANGLE_SET {
                Shows::Y
            } else {
                Shows::Normal
            }
        })
    });
    check_badge_cut(badge_shown);
    rejects("a badge with one angle set", || {
        check_badge_cut(|v: &View| is_set(v.tau1) || is_set(v.tau2))
    });
    rejects("a badge with X = Y", || {
        check_badge_cut(|v: &View| is_set(v.tau1) && is_set(v.tau2))
    });
    rejects("a badge at the cut", || {
        check_badge_cut(|v: &View| {
            v.tau1.abs() >= ANGLE_SET && v.tau2.abs() >= ANGLE_SET && v.x != v.y
        })
    });
}

/// The view `two(0, 0)` with the tilts `(τ₁, τ₂)` in radians.
fn at_angles(t1: f64, t2: f64) -> View {
    View {
        tau1: t1,
        tau2: t2,
        ..two(0.0, 0.0)
    }
}

/// Asserts `set`, whether an angle is set, is false up to the cut either side and true past it.
fn check_set_cut(set: impl Fn(f64) -> bool) {
    assert!(!set(0.0));
    assert!(!set(ANGLE_SET), "at the cut, not set");
    assert!(!set(-ANGLE_SET));
    assert!(set(ANGLE_SET * 1.5));
    assert!(set(-ANGLE_SET * 1.5));
}

/// Asserts `rule`, the third axis in Tilt mode at rest, shows the normal with either angle at the cut.
fn check_unset_at_cut(rule: impl Fn(&View) -> Shows) {
    assert_eq!(rule(&at_angles(ANGLE_SET, 0.0)), Shows::Normal);
    assert_eq!(rule(&at_angles(0.0, ANGLE_SET)), Shows::Normal);
}

/// Asserts `badge` is shown only with both angles past the cut and `X ≠ Y`.
fn check_badge_cut(badge: impl Fn(&View) -> bool) {
    assert!(!badge(&at_angles(1.0, ANGLE_SET)));
    assert!(!badge(&at_angles(ANGLE_SET, 1.0)));
    assert!(badge(&at_angles(1.0, 1.0)));
    assert!(!badge(&View {
        y: Z_Q3,
        ..at_angles(1.0, 1.0)
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
    let at_rest = (app.manifold.compass.slider, third(&h, &app));
    check_third(&at_rest.1, "X z_q3", "at rest: X");
    let rail = app.keyboard.place_of(TAU2).expect("τ₂ placed").rect;
    let from = rail.left_center() + vec2(rail.width() * 0.3, 0.0);
    let _ = h.frame(&mut app, vec![egui::Event::PointerMoved(from)]);
    let _ = h.frame(&mut app, vec![button(from, true)]);
    let _ = h.frame(&mut app, Vec::new());
    let held = (app.manifold.compass.slider, third(&h, &app));
    check_held(&held, Some(Gesture::Tau2), "Y z_μ1");
    let _ = h.frame(
        &mut app,
        vec![egui::Event::PointerMoved(from + vec2(12.0, 0.0))],
    );
    let _ = h.frame(&mut app, Vec::new());
    check_third(&third(&h, &app), "Y z_μ1", "dragged: Y");
    let d = drawing(&h, &app);
    assert_eq!(d.third, Shows::Y);
    rejects("τ₂'s slider held, read at rest", || {
        check_held(&at_rest, Some(Gesture::Tau2), "Y z_μ1")
    });
    rejects("τ₂'s slider held as τ₁'s", || {
        check_held(
            &(Some(Gesture::Tau1), held.1.clone()),
            Some(Gesture::Tau2),
            "Y z_μ1",
        )
    });
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
    let released = (app.manifold.compass.slider, third(&h, &app));
    check_held(&released, None, "X z_q3");
    rejects("the held reading kept when the drag ends", || {
        check_held(&held, None, "X z_q3")
    });
}

/// Asserts the slider `held`, and the third axis's text with it: `(slider, text)`.
fn check_held((slider, text): &(Option<Gesture>, String), gesture: Option<Gesture>, want: &str) {
    assert_eq!(*slider, gesture, "the slider held");
    check_third(text, want, "the third axis while it is");
}

#[test]
fn mock_cube_third_axis_during_a_tau1_slider_drag_reads_x_and_a_held_gamma_nothing() {
    let (mut app, mut h) = app_on(&two(0.0, 15.0));
    check_third(&third(&h, &app), "Y z_μ1", "at rest");
    let mut seen = Vec::new();
    for (id, gesture) in [(TAU1, Some(Gesture::Tau1)), (GAMMA, None)] {
        let rail = app.keyboard.place_of(id).expect("placed").rect;
        let from = rail.left_center() + vec2(rail.width() * 0.3, 0.0);
        let _ = h.frame(&mut app, vec![egui::Event::PointerMoved(from)]);
        let _ = h.frame(&mut app, vec![button(from, true)]);
        let _ = h.frame(&mut app, Vec::new());
        let held = app.manifold.compass.slider;
        check_slider(held, gesture, &format!("{id} held"));
        let _ = h.frame(&mut app, vec![button(from, false)]);
        let _ = h.frame(&mut app, Vec::new());
        // The press set the value where it landed, a real gesture: the rest rule reads the angles next.
        let released = app.manifold.compass.slider;
        check_slider(released, None, &format!("{id} released"));
        let tau1 = two(0.0, 15.0);
        app.set_field(basis_edit(tau1.basis()));
        follow(&mut h, &mut app);
        check_third(&third(&h, &app), "Y z_μ1", &format!("{id}: back at rest"));
        seen.push((held, released));
    }
    rejects("τ₁'s slider held as τ₂'s", || {
        check_slider(Some(Gesture::Tau2), Some(Gesture::Tau1), "τ₁ held")
    });
    rejects("γ's slider held as a tilt", || {
        check_slider(seen[0].0, None, "γ held")
    });
    rejects("τ₁'s slider held as none", || {
        check_slider(seen[1].0, Some(Gesture::Tau1), "τ₁ held")
    });
    rejects("τ₁'s slider still held when released", || {
        check_slider(seen[0].0, None, "τ₁ released")
    });
    rejects("back at rest on X", || {
        check_third("X z_q3", "Y z_μ1", "back at rest")
    });
}

/// Asserts the slider held: `got` is `want`.
fn check_slider(got: Option<Gesture>, want: Option<Gesture>, what: &str) {
    assert_eq!(got, want, "{what}");
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
    let dragging = third(&h, &app);
    check_third(&dragging, "Y z_μ1", "τ₂ moved further");
    assert_eq!(drawing(&h, &app).third.gesture(), Gesture::Tau2);
    let drag = app.manifold.compass.drag.expect("dragging");
    check_drag_moved_tau2(&drag);
    let _ = h.frame(&mut app, vec![button(from + delta, false)]);
    follow(&mut h, &mut app);
    // At rest both are set: X, by the rest rule.
    let rest = third(&h, &app);
    check_third(&rest, "X z_q3", "at rest");
    rejects("the drag's reading kept at rest", || {
        assert_eq!(third(&h, &app), "Y z_μ1")
    });
    rejects("the drag's reading kept at rest, the text", || {
        check_third(&dragging, "X z_q3", "at rest")
    });
    rejects("the drag read by the rest rule", || {
        check_third(&rest, "Y z_μ1", "τ₂ moved further")
    });
    rejects("the travels exchanged, τ₁ moved further", || {
        check_drag_moved_tau2(&Drag {
            tau: (drag.tau.1, drag.tau.0),
            ..drag
        })
    });
    rejects("the drag's travel lost", || {
        check_drag_moved_tau2(&Drag {
            tau: (drag.tau.0, drag.tau.1 - 5.0),
            ..drag
        })
    });
}

/// Asserts the drag of `(+10°, +30°)` from `(0, 0)` is `τ₂`'s, at those targets.
fn check_drag_moved_tau2(drag: &Drag) {
    assert_eq!(drag.gesture(), Some(Gesture::Tau2));
    assert!(
        (drag.tau.0 - 10.0).abs() < 0.5 && (drag.tau.1 - 30.0).abs() < 0.5,
        "{drag:?}"
    );
}

#[test]
fn mock_cube_third_axis_a_drags_gesture_is_the_further_tau1_on_a_tie() {
    let plane = Plane {
        z0: [0.0; 8],
        q1: two(0.0, 0.0).basis().0,
        q2: two(0.0, 0.0).basis().1,
    };
    let f = frame_of(&two(0.0, 0.0));
    let tilting = Drag::start(Touch::Tilt, &plane, &f);
    let slicing = Drag::start(Touch::Slice, &plane, &f);
    check_gesture_rule(&tilting, &slicing, Drag::gesture);
    // Each rule the gesture is not.
    let further = |d: &Drag, tie: Gesture| {
        (d.mode == Touch::Tilt).then(|| {
            let (m1, m2) = ((d.tau.0 - d.from.0).abs(), (d.tau.1 - d.from.1).abs());
            if m2 > m1 {
                Gesture::Tau2
            } else if m1 > m2 {
                Gesture::Tau1
            } else {
                tie
            }
        })
    };
    rejects("τ₂ on a tie", || {
        check_gesture_rule(&tilting, &slicing, |d: &Drag| further(d, Gesture::Tau2))
    });
    rejects("the larger angle, not the larger travel", || {
        check_gesture_rule(&tilting, &slicing, |d: &Drag| {
            (d.mode == Touch::Tilt).then(|| {
                if d.tau.1.abs() > d.tau.0.abs() {
                    Gesture::Tau2
                } else {
                    Gesture::Tau1
                }
            })
        })
    });
    rejects("the signed travel, not its size", || {
        check_gesture_rule(&tilting, &slicing, |d: &Drag| {
            (d.mode == Touch::Tilt).then_some(if d.tau.1 - d.from.1 > d.tau.0 - d.from.0 {
                Gesture::Tau2
            } else {
                Gesture::Tau1
            })
        })
    });
    rejects("a slice read as a tilt", || {
        check_gesture_rule(&tilting, &slicing, |d: &Drag| {
            further(
                &Drag {
                    mode: Touch::Tilt,
                    ..*d
                },
                Gesture::Tau1,
            )
        })
    });
}

/// Asserts `gesture`, the gesture a drag is, on `tilting`, a tilt-mode drag at `(0, 0)`, moved through the ruling's
/// cases, and on `slicing`, a slice-mode one: the angle moved further since it began, `τ₁` on a tie, none for a slice.
fn check_gesture_rule(tilting: &Drag, slicing: &Drag, gesture: impl Fn(&Drag) -> Option<Gesture>) {
    let mut d = *tilting;
    assert_eq!(gesture(&d), Some(Gesture::Tau1), "a tie at the start");
    d.tau = (10.0, -10.0);
    assert_eq!(gesture(&d), Some(Gesture::Tau1), "a tie, opposite signs");
    d.tau = (10.0, -10.5);
    assert_eq!(gesture(&d), Some(Gesture::Tau2));
    d.tau = (-10.5, 10.0);
    assert_eq!(gesture(&d), Some(Gesture::Tau1));
    d.from = (5.0, 5.0);
    d.tau = (5.0, 6.0);
    assert_eq!(gesture(&d), Some(Gesture::Tau2), "since it began");
    d.tau = (7.0, 6.0);
    assert_eq!(gesture(&d), Some(Gesture::Tau1), "since it began");
    // The travel, not the angle: τ₁ far from 0 and still, τ₂ moved a little.
    d.from = (-20.0, 0.0);
    d.tau = (-20.0, 5.0);
    assert_eq!(gesture(&d), Some(Gesture::Tau2), "τ₁ has not moved");
    assert_eq!(gesture(slicing), None, "a slice is no tilt");
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
    check_badge_drawing(&d);
    rejects("the badge case drawn without its badge", || {
        check_badge_drawing(&Drawing {
            badge: false,
            ..d.clone()
        })
    });
    rejects("the badge case's third axis on Y at rest", || {
        check_badge_drawing(&Drawing {
            third: Shows::Y,
            ..d.clone()
        })
    });
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
    let texts = painted(&compass_shapes(&mut h, &mut app));
    check_badge_painted(&texts);
    rejects("no badge painted", || check_badge_painted(&texts[..0]));
    let placed = app.keyboard.place_of(COMPASS_BADGE).expect("placed").rect;
    let sq = square(&h);
    check_badge_placed(placed, sq, &d);
    rejects("a badge in the middle of the drawing", || {
        check_badge_placed(
            egui::Rect::from_center_size(sq.center(), placed.size()),
            sq,
            &d,
        )
    });
    rejects("a badge at the top-left corner", || {
        check_badge_placed(
            egui::Rect::from_min_size(sq.left_top(), placed.size()),
            sq,
            &d,
        )
    });
    click_scope(&mut h, &mut app, COMPASS_BADGE);
    assert_eq!(app.manifold.compass.view, CompassView::Plucker, "the click");
    let q1 = app.snapshot().sim.plane.q1;
    check_edits_nothing(&q1);
    rejects("a badge that left the view", || {
        assert_eq!(app.manifold.compass.view, CompassView::Cube)
    });
    rejects("a badge click that edits the plane", || {
        check_edits_nothing(&two(0.0, 15.0).basis().0)
    });
}

/// Asserts the badge case's drawing: X at rest, the gesture's plane exactly, the badge shown, both targets named.
fn check_badge_drawing(d: &Drawing) {
    assert_eq!(d.third, Shows::X);
    check_polygon(d, &independent_rectangle(20.0));
    assert!(d.badge, "both set, X ≠ Y");
    check_names_both(&d.readout, "z_q3", "z_μ1");
}

/// Asserts the frame's painted texts hold the badge.
fn check_badge_painted(texts: &[(String, egui::Rect, f32, egui::Color32)]) {
    assert!(texts.iter().any(|t| t.0 == BADGE), "painted");
}

/// Asserts the badge's scope `placed` is on its rect in the square `sq`'s top-right corner, clear of every projected
/// cube corner of `d`.
fn check_badge_placed(placed: egui::Rect, sq: egui::Rect, d: &Drawing) {
    assert!((placed.center() - badge_rect(sq).center()).length() <= 1.0);
    assert!(
        (placed.right_top() - sq.right_top()).length() <= 1.0,
        "the top-right corner"
    );
    // Clear of the drawing: of every projected cube corner, and of the labels.
    for corner in compass::CORNERS {
        assert!(!placed.contains(d.iso.at(corner)), "{corner:?} under it");
    }
}

/// Asserts the badge's click left `q₁` the badge case's.
fn check_edits_nothing(q1: &engine::contract::sim_config::Latent) {
    assert_eq!(*q1, two(20.0, 15.0).basis().0, "edits nothing");
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
    let on_badge = (app.manifold.compass.view, owned(&focus(&app)));
    keys_on(&mut h, &mut app);
    press(&mut h, &mut app, Key::Enter, Modifiers::NONE);
    let switched = app.manifold.compass.view;
    let _ = h.frame(&mut app, Vec::new());
    let landed = (switched, owned(&focus(&app)));
    check_switched_onto_plucker(&landed);
    rejects("Enter on the badge that does nothing", || {
        check_switched_onto_plucker(&on_badge)
    });
    rejects(
        "the view switched, the focus left on the gone badge",
        || check_switched_onto_plucker(&(switched, on_badge.1.clone())),
    );
    rejects(
        "the focus on the view button, the view not switched",
        || check_switched_onto_plucker(&(on_badge.0, landed.1.clone())),
    );
}

/// A focus path, owned.
fn owned(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| (*s).to_owned()).collect()
}

/// Asserts Enter on the badge left `(view, focus)`: the Plücker view, the focus on its button.
fn check_switched_onto_plucker((view, path): &(CompassView, Vec<String>)) {
    assert_eq!(*view, CompassView::Plucker);
    assert_eq!(path, &[COMPASS, COMPASS_PLUCKER]);
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
    let unit = n.map(|x| x / len);
    check_unit_normal(&cut.normal, &unit);
    rejects(
        "the τ₁ gesture's plane alone, not the whole plane",
        || check_unit_normal(&[-s1, 0.0, c1], &unit),
    );
    let placed = app.keyboard.place_of(COMPASS_BADGE).map(|p| p.rect);
    let texts = painted(&compass_shapes(&mut h, &mut app));
    check_no_badge(placed, &texts);
    check_names_both(&d.readout, "z_q3", "z_q3");
    rejects("a badge shown with X = Y", || assert!(d.badge));
    // The control: the two-target case's badge, placed and painted.
    let (mut two_targets, mut h2) = app_on(&two(20.0, 15.0));
    let _ = h2.frame(&mut two_targets, Vec::new());
    let shown = two_targets.keyboard.place_of(COMPASS_BADGE).map(|p| p.rect);
    let shown_texts = painted(&compass_shapes(&mut h2, &mut two_targets));
    rejects("a badge placed with X = Y", || {
        check_no_badge(shown, &texts)
    });
    rejects("a badge painted with X = Y", || {
        check_no_badge(placed, &shown_texts)
    });
}

/// Asserts no badge: never placed, `placed` none, and not among the painted `texts`.
fn check_no_badge(placed: Option<egui::Rect>, texts: &[(String, egui::Rect, f32, egui::Color32)]) {
    assert_eq!(placed, None, "never placed");
    assert!(!texts.iter().any(|t| t.0 == BADGE), "not painted");
}

/// Asserts the drawn plane's unit normal `got` is `want`.
fn check_unit_normal(got: &[f64; 3], want: &[f64; 3]) {
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() < 1e-12, "{got:?}");
    }
}

#[test]
fn mock_cube_third_axis_each_gesture_draws_its_own_frame() {
    // τ₂ alone, into Y: the plane in (e_a, e_b, e_μ1).
    let (app, h) = app_on(&two(0.0, 15.0));
    let d = drawing(&h, &app);
    check_y_drawing(&d);
    rejects("τ₂ alone drawn in X's frame", || {
        let mut wrong = d.clone();
        wrong.slice = compass::slice_drawing(&app.snapshot().sim.plane.z0, &two(15.0, 0.0));
        check_y_drawing(&wrong)
    });
    rejects("τ₂ alone read as X", || {
        check_y_drawing(&Drawing {
            third: Shows::X,
            ..d.clone()
        })
    });
    // The rule's arms, with γ kept.
    check_gesture_views(gesture_view);
    rejects("no gesture's frame: the view as it is", || {
        check_gesture_views(|v: &View, _, _| *v)
    });
    rejects("the active gesture ignored", || {
        check_gesture_views(|v: &View, mode, _| gesture_view(v, mode, None))
    });
    rejects("the other angle kept", || {
        check_gesture_views(|v: &View, mode, active| View {
            tau1: v.tau1,
            tau2: v.tau2,
            ..gesture_view(v, mode, active)
        })
    });
    rejects("γ dropped", || {
        check_gesture_views(|v: &View, mode, active| View {
            gamma: 0.0,
            ..gesture_view(v, mode, active)
        })
    });
}

/// Asserts the drawing of `τ₂ = 15°` alone, into Y: Y shown, the plane in `(e_a, e_b, e_μ1)` exactly, no badge.
fn check_y_drawing(d: &Drawing) {
    assert_eq!(d.third, Shows::Y);
    check_polygon(d, &independent_rectangle_v(15.0));
    assert!(!d.badge);
}

/// Asserts `rule`, the view a gesture draws, by the ruling's arms: X or Y with the other angle 0 and `γ` kept, the
/// whole plane in its frame with one target, the normal's frame at rest with both 0, and Slice mode B2's plane but the
/// rest rule's with the badge, whatever is dragged.
fn check_gesture_views(rule: impl Fn(&View, Touch, Option<Gesture>) -> View) {
    let g = View {
        gamma: 0.3,
        ..two(20.0, 15.0)
    };
    assert_eq!(
        rule(&g, Touch::Tilt, None),
        View {
            d: Z_Q3,
            tau2: 0.0,
            ..g
        },
        "X, the other angle 0"
    );
    assert_eq!(
        rule(&g, Touch::Tilt, Some(Gesture::Tau2)),
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
        rule(&one, Touch::Tilt, None),
        View { d: Z_Q2, ..one },
        "X = Y: the whole plane, in its frame"
    );
    assert_eq!(
        rule(&one, Touch::Tilt, Some(Gesture::Tau2)),
        View { d: Z_Q2, ..one },
        "Y = X: the whole plane, in its frame"
    );
    // At rest with both 0, the normal's frame.
    let zero = two(0.0, 0.0);
    assert_eq!(rule(&zero, Touch::Tilt, None), zero);
    // Slice mode: B2's plane in (e_a, e_b, e_D), but with the badge the rule's at rest, whatever is dragged.
    let lone = two(20.0, 0.0);
    assert_eq!(rule(&lone, Touch::Slice, None), lone);
    assert_eq!(
        rule(&g, Touch::Slice, Some(Gesture::Tau2)),
        rule(&g, Touch::Tilt, None),
    );
    let one_slice = View {
        x: Z_Q3,
        y: Z_Q3,
        ..g
    };
    assert_eq!(rule(&one_slice, Touch::Slice, None), one_slice);
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
    check_x_frame_normal(&n, (s, c));
    let mut in_d = [0.0; 8];
    in_d[Z_ALPHA] = -s;
    in_d[Z_Q2] = c;
    rejects("the normal in D's frame", || {
        check_x_frame_normal(&in_d, (s, c))
    });
    let mut leaning = n;
    leaning[Z_BETA] = 1e-6;
    rejects("a normal leaning onto z_β", || {
        check_x_frame_normal(&leaning, (s, c))
    });
    // The drag reads along that normal's screen image, not the (a, b, D) plane's.
    let d = Drag::start(Touch::Slice, &plane, &v.frame());
    check_reads_along(d.reading, (s, c));
    rejects("the reading of the untilted plane's normal", || {
        check_reads_along(compass::reading_of([0.0, 0.0, 1.0]), (s, c))
    });
    rejects("a vertical reading", || {
        check_reads_along(Reading::Vertical(1.0), (s, c))
    });
}

/// Asserts the latent normal `n` of the badge case's plane at `τ₁` with `(sin τ₁, cos τ₁) = (s, c)` is in the frame of
/// X: `−s e_α + c e_q3`, nothing on `z_q2` or `z_β`.
fn check_x_frame_normal(n: &engine::contract::sim_config::Latent, (s, c): (f64, f64)) {
    assert!((n[Z_ALPHA] + s).abs() < 1e-12, "{n:?}");
    assert!((n[Z_Q3] - c).abs() < 1e-12, "in the frame of X: {n:?}");
    assert_eq!(n[Z_Q2], 0.0, "not the normal's frame: {n:?}");
    assert_eq!(n[Z_BETA], 0.0);
}

/// Asserts a slice drag's `reading` is along the screen image of `(−s, 0, c)`.
fn check_reads_along(reading: Reading, (s, c): (f64, f64)) {
    let want = compass::reading_of([-s, 0.0, c]);
    assert_eq!(reading, want);
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
    check_no_plane_line(&tilt, false);
    let slice = compass::readout(&[0.0; 8], &v, Touch::Slice);
    check_no_plane_line(&slice, true);
    // With the gesture on Y, at the corner of a plane the frame draws, no line either.
    let dragged = compass::readout_active(&[0.0; 8], &v, Touch::Tilt, Some(Gesture::Tau2));
    check_no_plane_line(&dragged, false);
    rejects("D's frame read in Tilt mode", || {
        check_no_plane_line(&slice, false)
    });
    rejects("a plane read in D's frame, where it has none", || {
        check_no_plane_line(&tilt, true)
    });
}

/// Asserts the readout `lines` has the "no plane" line exactly where `want` says.
fn check_no_plane_line(lines: &[String], want: bool) {
    assert_eq!(
        lines.iter().any(|l| l.starts_with("no plane")),
        want,
        "{lines:?}"
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
    check_active_parts(&on);
    rejects("no part in any colour", || {
        check_active_parts(&|_: &str, _: Gesture| Vec::new())
    });
    rejects("τ₁'s and τ₂'s parts exchanged", || {
        check_active_parts(&|line: &str, g: Gesture| {
            on(
                line,
                match g {
                    Gesture::Tau1 => Gesture::Tau2,
                    Gesture::Tau2 => Gesture::Tau1,
                    Gesture::Slice => Gesture::Slice,
                },
            )
        })
    });
    rejects("the whole line in the gesture's colour", || {
        check_active_parts(&|line: &str, _: Gesture| vec![line.to_owned()])
    });
    // The parts rebuild the line.
    let parts: Vec<String> = line_parts(targets, Gesture::Tau1, "z_q3")
        .into_iter()
        .map(|(p, _)| p)
        .collect();
    check_rebuilds(&parts, targets);
    rejects("parts that drop the line's end", || {
        check_rebuilds(&parts[..parts.len() - 1], targets)
    });
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
    check_line_colours(&colours);
    rejects("the whole line in τ₁'s colour", || {
        check_line_colours(&[GESTURE_TAU1, GESTURE_TAU1])
    });
    rejects("the line in one part", || check_line_colours(&colours[..1]));
    rejects("the target's part in τ₂'s colour", || {
        check_line_colours(&[GESTURE_TAU2, colours[1]])
    });
}

/// Asserts `on`, the parts of a readout line a gesture colours, by the ruling's cases: each tilt its target's part,
/// the slice the normal's part and its own depth line, and nothing else.
fn check_active_parts(on: &dyn Fn(&str, Gesture) -> Vec<String>) {
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
}

/// Asserts the `parts` rebuild the line `whole`.
fn check_rebuilds(parts: &[String], whole: &str) {
    assert_eq!(parts.concat(), whole);
}

/// Asserts the targets' line was painted in two parts, `τ₁`'s in its colour and the rest not.
fn check_line_colours(colours: &[egui::Color32]) {
    assert_eq!(colours.len(), 2, "two parts: {colours:?}");
    assert_eq!(colours[0], GESTURE_TAU1);
    assert_ne!(colours[1], GESTURE_TAU1);
}

#[test]
fn mock_cube_third_axis_the_capture_cases_set_their_angles_targets_and_mode() {
    use crate::capture::CompassCase;
    check_parameters(CompassCase::parameters);
    rejects("the badge case at the third-axis case's angles", || {
        check_parameters(|case: CompassCase| match case {
            CompassCase::Badge => CompassCase::ThirdAxis.parameters(),
            other => other.parameters(),
        })
    });
    rejects("the slice case in Tilt mode", || {
        check_parameters(|case: CompassCase| match case {
            CompassCase::Slice => ((0.0, 0.0, 0.0), 0.25, Touch::Tilt),
            other => other.parameters(),
        })
    });
    // Each R-459 case leaves z₀ 0 on Y and picks Y = z_μ1, in the one edit that sets the basis.
    let mut app = super::support::mock_app();
    let mut h = super::support::headless();
    let _ = h.frame(&mut app, Vec::new());
    let start = (
        app.snapshot().sim.clone(),
        app.snapshot().history.undo_depth,
    );
    let mut set = Vec::new();
    for (case, want) in [
        (CompassCase::ThirdAxis, two(0.0, 15.0)),
        (CompassCase::Badge, two(20.0, 15.0)),
    ] {
        let depth = app.snapshot().history.undo_depth;
        for edit in case.edits(&app.snapshot().sim) {
            app.set_field(edit);
        }
        follow(&mut h, &mut app);
        let got = (
            app.snapshot().sim.clone(),
            app.snapshot().history.undo_depth,
        );
        check_case_set(&got, depth, &want, case);
        set.push((got, depth, want, case));
    }
    for (got, depth, want, case) in &set {
        rejects("a case whose edits were never made", || {
            check_case_set(&start, *depth, want, *case)
        });
        rejects("a case made in one entry", || {
            check_case_set(&(got.0.clone(), got.1 - 1), *depth, want, *case)
        });
    }
    rejects("the badge case's state for the third-axis case", || {
        let (third, badge) = (&set[0], &set[1]);
        check_case_set(&badge.0, third.1, &third.2, third.3)
    });
    // An older case leaves the frame the chart's.
    let before = app.snapshot().sim.slice.frame;
    for edit in CompassCase::Tilt0.edits(&app.snapshot().sim) {
        app.set_field(edit);
    }
    follow(&mut h, &mut app);
    let older = app.snapshot().sim.slice.frame;
    check_frame_kept(older, before);
    rejects("a case that picks no target", || {
        assert_eq!(before, view(0.0, 0.0, 0.0, 1.0).frame())
    });
    rejects("an older case that moves the frame", || {
        check_frame_kept(view(0.0, 0.0, 0.0, 1.0).frame(), before)
    });
}

/// Asserts `parameters`, each capture case's angles, depth and mode, by the table of cases.
fn check_parameters(
    parameters: impl Fn(crate::capture::CompassCase) -> ((f64, f64, f64), f64, Touch),
) {
    use crate::capture::CompassCase;
    for (case, angles, depth, mode) in [
        (CompassCase::Slice, (0.0, 0.0, 0.0), 0.25, Touch::Slice),
        (CompassCase::Tilt0, (0.0, 0.0, 0.0), 0.0, Touch::Tilt),
        (CompassCase::TiltRect, (30.0, 0.0, 0.0), 0.0, Touch::Tilt),
        (CompassCase::TiltHex, (45.0, 45.0, 0.0), 0.0, Touch::Tilt),
        (CompassCase::ThirdAxis, (0.0, 15.0, 0.0), 0.0, Touch::Tilt),
        (CompassCase::Badge, (20.0, 15.0, 0.0), 0.0, Touch::Tilt),
    ] {
        assert_eq!(parameters(case), (angles, depth, mode), "{case:?}");
    }
}

/// Asserts the R-459 case `case`, made from undo depth `depth`, left `(sim, depth after)` on `want`'s frame and basis,
/// `z₀` 0 on X and Y, in two entries.
fn check_case_set(
    (sim, after): &(engine::contract::sim_config::SimConfig, u32),
    depth: u32,
    want: &View,
    case: crate::capture::CompassCase,
) {
    assert_eq!(sim.slice.frame, want.frame(), "{case:?}'s frame");
    assert_eq!(sim.plane.z0[Z_MU1], 0.0, "{case:?}: z₀ on Y");
    assert_eq!(sim.plane.z0[Z_Q3], 0.0, "{case:?}: z₀ on X");
    let (q1, q2) = want.basis();
    assert_eq!((sim.plane.q1, sim.plane.q2), (q1, q2), "{case:?}'s basis");
    assert_eq!(*after, depth + 2, "{case:?}");
}

/// Asserts the older case kept the slice frame: `got` is `before`.
fn check_frame_kept(got: SliceFrame, before: SliceFrame) {
    assert_eq!(got, before);
}
