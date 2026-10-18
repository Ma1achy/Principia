//! `mock_trajectory_views` (REQ-GUI-202; R-453 D6 to D8; render_gui_spec §G2's Trajectory panel, §G3's Trajectory
//! keys): on real space and the unwrapped view a drag pans and the wheel zooms about the pointer; on the sphere a drag
//! orbits (yaw, pitch clamped to ±90°), turning "turn" off, which ticked again turns on from the view as it stands, and
//! the wheel zooms; a double-click restores each default; the orbit turns the view, never the data; each tab keeps its
//! own views; from the keyboard each view is an element whose interaction mode pans by `Pan` or orbits by `Angle` with
//! the arrows and zooms by `ZoomLog2` with + and −, Shift ×10 and Alt ×0.1, the unwrapped view in the sphere's place
//! taking real space's keys; the Inspector's panes take the same pointer; and nothing sends a `SetField` or enters
//! undo. Each check also runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{self, vec2, Event, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, Rect};
use engine::contract::view_ui::{KeyMode, Window};

use super::support::{click, headless, mock_app, press, rejects, shapes};
use crate::app::App;
use crate::capture::PIXELS_PER_POINT;
use crate::explore::manifold_view::navigate::OCTAVES_PER_LINE;
use crate::explore::trajectory::{
    self, areas, keep, on_sphere, remove, unwrapped_point, view, view_at, zoom_presses, Orbit,
    PanZoom, Scratch, Tab, Views, ORBIT_DEG_PER_POINT, PAN_ZOOM_RANGE, PITCH_LIMIT_DEG, REAL_VIEW,
    SPHERE_TILT_DEG, SPHERE_VIEW, SPHERE_ZOOM_RANGE, TRAJECTORY,
};
use crate::headless::{Headless, FRAME_S};
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::{Adjust, StepKind};
use crate::layout::Layout;
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;

/// Whether `a` and `b` agree to 1e-9.
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// The app with the pointer hovered over the figure, then moved onto the panel's title, the sample kept there (R-453
/// D3), the sphere's turn off so the frames move nothing by themselves.
fn shown() -> (App<MockSide>, Headless, Layout) {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let layout = Layout::new(h.screen(), PIXELS_PER_POINT);
    let hover = crate::capture::hover_point(layout.figure);
    let _ = h.frame(&mut app, vec![Event::PointerMoved(hover)]);
    let _ = h.frame(&mut app, Vec::new());
    let _ = h.frame(
        &mut app,
        vec![Event::PointerMoved(layout.trajectory.center_top())],
    );
    let _ = h.frame(&mut app, Vec::new());
    assert!(app.trajectory.first().is_some(), "no sample shown");
    (app, h, layout)
}

/// The panel's real-space and sphere frames.
fn frames_of(layout: &Layout) -> (Rect, Rect) {
    let a = areas(layout.trajectory);
    (a.real, a.sphere)
}

/// A primary drag from `from` by `by`, then a frame: one move past egui's click distance, so the whole move drags.
fn drag(h: &mut Headless, app: &mut App<MockSide>, from: Pos2, by: egui::Vec2) {
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: NONE,
    };
    let to = from + by;
    for events in [
        vec![Event::PointerMoved(from)],
        vec![button(from, true)],
        vec![Event::PointerMoved(to)],
        vec![button(to, false)],
        Vec::new(),
    ] {
        let _ = h.frame(app, events);
    }
}

/// One notch of the wheel up at `at`, then a frame.
fn wheel(h: &mut Headless, app: &mut App<MockSide>, at: Pos2, notches: f32) {
    let _ = h.frame(app, vec![Event::PointerMoved(at)]);
    let _ = h.frame(
        app,
        vec![Event::MouseWheel {
            unit: MouseWheelUnit::Line,
            delta: vec2(0.0, notches),
            phase: egui::TouchPhase::Move,
            modifiers: NONE,
        }],
    );
    let _ = h.frame(app, Vec::new());
}

/// A double-click at `at`.
fn double_click(h: &mut Headless, app: &mut App<MockSide>, at: Pos2) {
    h.set_frame_step(0.05);
    for _ in 0..2 {
        for events in crate::capture::click(at) {
            let _ = h.frame(app, events);
        }
    }
    h.set_frame_step(FRAME_S);
    let _ = h.frame(app, Vec::new());
}

/// The filled circles drawn in `rect` by one frame: the bodies' and the playhead's marks.
fn dots(h: &mut Headless, app: &mut App<MockSide>, rect: Rect) -> Vec<Pos2> {
    shapes(h, app)
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Circle(c)
                if c.fill != egui::Color32::TRANSPARENT && rect.contains(c.center) =>
            {
                Some(c.center)
            }
            _ => None,
        })
        .collect()
}

/// The views of the first tab.
fn first(app: &App<MockSide>) -> Views {
    app.trajectory.views_of(Tab::UnderCursor)
}

/// No SetField but the hover's, and an empty undo.
fn sent_nothing(app: &App<MockSide>) {
    let sent: Vec<&str> = app
        .console()
        .iter()
        .map(|e| e.message.as_str())
        .filter(|m| m.starts_with("SetField") && !m.starts_with("SetField Hover.at"))
        .collect();
    assert!(sent.is_empty(), "{sent:?}");
    assert_eq!(app.snapshot().history.undo_depth, 0);
}

#[test]
fn mock_trajectory_views_pan_and_zoom_state() {
    let mut v = PanZoom::default();
    assert_eq!(v.apply([0.3, -0.2]), [0.3, -0.2], "the default is the fit");
    v.drag([0.5, -0.25]);
    assert_eq!(v.centre, [-0.5, 0.25]);
    assert_eq!(
        v.apply([0.0, 0.0]),
        [0.5, -0.25],
        "the drawing follows the drag"
    );
    // The wheel about a point: the drawing's point there stays there.
    let at = [0.4, -0.6];
    let under = v.unapply(at);
    v.zoom_about(1.0, at);
    assert!(close(v.zoom, 1.0));
    let after = v.apply(under);
    assert!(
        close(after[0], at[0]) && close(after[1], at[1]),
        "{after:?}"
    );
    rejects("a zoom about the centre", || {
        let mut c = PanZoom::default();
        c.drag([0.5, -0.25]);
        c.zoom_about(1.0, [0.0, 0.0]);
        let p = c.apply(under);
        assert!(close(p[0], at[0]) && close(p[1], at[1]));
    });
    // Zoomed, a drag pans by the drag on screen.
    let before = v.apply([0.1, 0.1]);
    v.drag([0.2, 0.0]);
    let moved = v.apply([0.1, 0.1]);
    assert!(close(moved[0] - before[0], 0.2) && close(moved[1], before[1]));
    // The range: kept at its ends, exactly.
    let mut z = PanZoom::default();
    z.zoom_about(PAN_ZOOM_RANGE.1, [0.0, 0.0]);
    assert_eq!(z.zoom, PAN_ZOOM_RANGE.1);
    z.zoom_about(0.25, [0.0, 0.0]);
    assert_eq!(z.zoom, PAN_ZOOM_RANGE.1);
    z.zoom_about(-100.0, [0.0, 0.0]);
    assert_eq!(z.zoom, PAN_ZOOM_RANGE.0);
    // The arrows: 1/20 of the span, → and ↑ moving the view right and up.
    let mut s = PanZoom::default();
    s.step(0.05, Direction::Right);
    assert!(close(s.centre[0], 0.1));
    s.step(0.05, Direction::Up);
    assert!(close(s.centre[1], -0.1));
    s.step(-0.05, Direction::Left);
    s.step(-0.05, Direction::Down);
    assert!(close(s.centre[0], 0.0) && close(s.centre[1], 0.0));
    let mut zoomed = PanZoom {
        zoom: 1.0,
        ..PanZoom::default()
    };
    zoomed.step(0.05, Direction::Right);
    assert!(
        close(zoomed.centre[0], 0.05),
        "half the span zoomed in once"
    );
    // Zoomed in an octave, a point 1.5 half-widths out is 0.75 of the drawing out, and a drag of 1.5 half-widths moves
    // the centre by 0.75 of the drawing, on each axis.
    let z1 = PanZoom {
        centre: [0.1, -0.2],
        zoom: 1.0,
    };
    let u = z1.unapply([1.5, -1.25]);
    assert!(close(u[0], 0.85) && close(u[1], -0.825), "{u:?}");
    let mut d = z1;
    d.drag([1.5, -1.25]);
    assert!(
        close(d.centre[0], -0.65) && close(d.centre[1], 0.425),
        "{:?}",
        d.centre
    );
    rejects("a drag that ignores the zoom", || {
        let mut e = PanZoom::default();
        e.drag([1.5, -1.25]);
        assert!(close(e.centre[0] + 0.1, -0.65))
    });
}

#[test]
fn mock_trajectory_views_orbit_state() {
    let o = Orbit::default();
    assert_eq!((o.yaw, o.pitch, o.zoom), (0.0, SPHERE_TILT_DEG, 0.0));
    let mut o = Orbit::default();
    o.turn(-10.0, 0.0);
    assert!(close(o.yaw, 350.0), "yaw kept in [0, 360)");
    o.turn(0.0, PITCH_LIMIT_DEG - SPHERE_TILT_DEG);
    assert_eq!(o.pitch, PITCH_LIMIT_DEG, "to the limit exactly");
    o.turn(0.0, 1.0);
    assert_eq!(o.pitch, PITCH_LIMIT_DEG, "clamped");
    o.turn(0.0, -500.0);
    assert_eq!(o.pitch, -PITCH_LIMIT_DEG);
    rejects("a pitch past the pole", || {
        let mut p = Orbit::default();
        p.turn(0.0, 200.0);
        assert!(p.pitch > PITCH_LIMIT_DEG);
    });
    o.zoom_by(100.0);
    assert_eq!(o.zoom, SPHERE_ZOOM_RANGE.1);
    o.zoom_by(-100.0);
    assert_eq!(o.zoom, SPHERE_ZOOM_RANGE.0);
    // The view at the default pitch is the panel's old view.
    let p = on_sphere([1.0, 1.2]);
    assert_eq!(view(p, 30.0), view_at(p, 30.0, SPHERE_TILT_DEG));
    // Pitched to the pole, the pole is at the centre, nearest.
    let ([x, y], depth) = view_at([0.0, 0.0, 1.0], 0.0, 90.0);
    assert!(x.abs() < 1e-12 && y.abs() < 1e-12 && close(depth, 1.0));
}

#[test]
fn mock_trajectory_views_real_space_drags_and_zooms_about_the_pointer() {
    let (mut app, mut h, layout) = shown();
    let (real, _) = frames_of(&layout);
    let before = dots(&mut h, &mut app, real);
    assert_eq!(before.len(), 3, "three bodies at the playhead");
    let by = vec2(14.0, -9.0);
    drag(&mut h, &mut app, real.center(), by);
    let after = dots(&mut h, &mut app, real);
    for (a, b) in before.iter().zip(&after) {
        assert!(
            (*b - *a - by).length() < 1e-3,
            "{a:?} → {b:?}, not by {by:?}"
        );
    }
    rejects("a view that only paints", || {
        for (a, b) in before.iter().zip(&before) {
            assert!((*b - *a - by).length() < 1e-3);
        }
    });
    assert!(
        app.trajectory.pin.is_none(),
        "a drag on the panel pins nothing"
    );
    // The wheel, one notch up, zooms by an eighth of an octave about the pointer: the dot under it stays.
    let v0 = first(&app).real;
    let at = before[0] + by;
    wheel(&mut h, &mut app, at, 1.0);
    let v1 = first(&app).real;
    assert!(close(v1.zoom - v0.zoom, OCTAVES_PER_LINE));
    let zoomed = dots(&mut h, &mut app, real);
    assert!(
        (zoomed[0] - at).length() < 1e-2,
        "{:?} left the pointer {at:?}",
        zoomed[0]
    );
    assert!((zoomed[1] - after[1]).length() > 1.0, "the others spread");
    // A double-click restores the fit.
    double_click(&mut h, &mut app, real.center());
    assert_eq!(first(&app).real, PanZoom::default());
    let restored = dots(&mut h, &mut app, real);
    for (a, b) in before.iter().zip(&restored) {
        assert!((*b - *a).length() < 1e-3);
    }
    sent_nothing(&app);
}

#[test]
fn mock_trajectory_views_the_sphere_orbits_and_zooms() {
    let (mut app, mut h, layout) = shown();
    let (_, sphere) = frames_of(&layout);
    assert!(app.trajectory.turn, "turn starts on");
    let data = app.trajectory.first().unwrap().shape.clone();
    drag(&mut h, &mut app, sphere.center(), vec2(30.0, 12.0));
    let o = first(&app).sphere;
    assert!(!app.trajectory.turn, "a drag turned turn off");
    assert!(close(o.yaw, 30.0 * ORBIT_DEG_PER_POINT), "{o:?}");
    assert!(
        close(o.pitch, SPHERE_TILT_DEG + 12.0 * ORBIT_DEG_PER_POINT),
        "{o:?}"
    );
    rejects("a sphere that only paints", || {
        assert!(close(Orbit::default().yaw, 30.0 * ORBIT_DEG_PER_POINT))
    });
    // Its turn holds while off; the data never moves.
    let held = app.trajectory.angle;
    for _ in 0..4 {
        let _ = h.frame(&mut app, Vec::new());
    }
    assert_eq!(app.trajectory.angle, held);
    assert_eq!(
        app.trajectory.first().unwrap().shape,
        data,
        "the orbit moved the data"
    );
    // Pitch clamped at the pole.
    drag(&mut h, &mut app, sphere.center(), vec2(0.0, 150.0));
    assert_eq!(first(&app).sphere.pitch, PITCH_LIMIT_DEG);
    // Ticking turn resumes from the view as it stands.
    let yaw = first(&app).sphere.yaw;
    click(&mut h, &mut app, "turn");
    assert!(app.trajectory.turn);
    let a0 = app.trajectory.angle;
    for _ in 0..4 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let turned = app.trajectory.angle - a0;
    assert!(turned > 0.0 && first(&app).sphere.yaw == yaw, "{turned}");
    // The wheel zooms it.
    wheel(&mut h, &mut app, sphere.center(), 2.0);
    assert!(close(first(&app).sphere.zoom, 2.0 * OCTAVES_PER_LINE));
    // A double-click restores the default view, at rest.
    double_click(&mut h, &mut app, sphere.center());
    assert_eq!(first(&app).sphere, Orbit::default());
    assert!(
        app.trajectory.angle < 1.0,
        "back at rest: {}",
        app.trajectory.angle
    );
    sent_nothing(&app);
}

#[test]
fn mock_trajectory_views_the_orbit_turns_the_view_not_the_data() {
    // Every point of the path keeps its (θ, φ) and its unwrapped place, whatever the orbit; only its view moves.
    let s = crate::mock::sample::sample(
        &mock_app().snapshot().sim.plane.clone(),
        [0.4, 0.6],
        0.0,
        crate::mock::MOCK_T_MAX,
    );
    let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(200.0, 200.0));
    let flat: Vec<Pos2> = s.shape.iter().map(|p| unwrapped_point(rect, *p)).collect();
    let mut o = Orbit::default();
    o.turn(40.0, 30.0);
    let seen: Vec<[f64; 2]> = s
        .shape
        .iter()
        .map(|p| view_at(on_sphere(*p), o.yaw, o.pitch).0)
        .collect();
    let rest: Vec<[f64; 2]> = s.shape.iter().map(|p| view(on_sphere(*p), 0.0).0).collect();
    assert_ne!(seen, rest, "the orbit moved the view");
    // The view of each point is the turned view of the same point: the data is what it was.
    let check = |shape: &[[f64; 2]]| {
        for (p, v) in shape.iter().zip(&seen) {
            let w = view_at(on_sphere(*p), o.yaw, o.pitch).0;
            assert!(close(w[0], v[0]) && close(w[1], v[1]));
        }
        let unwrapped: Vec<Pos2> = shape.iter().map(|p| unwrapped_point(rect, *p)).collect();
        assert_eq!(unwrapped, flat, "the sphere and its unwrap disagree");
    };
    check(&s.shape);
    // An orbit of the data, θ turned by the yaw, fails.
    let rotated: Vec<[f64; 2]> = s
        .shape
        .iter()
        .map(|[t, f]| {
            [
                (t + o.yaw.to_radians()).rem_euclid(std::f64::consts::TAU),
                *f,
            ]
        })
        .collect();
    rejects("an orbit that rotates the data", || check(&rotated));
}

#[test]
fn mock_trajectory_views_the_unwrapped_view_pans_and_zooms() {
    let (mut app, mut h, layout) = shown();
    let (_, place) = frames_of(&layout);
    app.trajectory.unwrapped = true;
    let _ = h.frame(&mut app, Vec::new());
    let turn = app.trajectory.turn;
    let before = dots(&mut h, &mut app, place);
    assert_eq!(before.len(), 1, "the playhead's mark");
    let by = vec2(-10.0, 6.0);
    drag(&mut h, &mut app, place.center(), by);
    let after = dots(&mut h, &mut app, place);
    assert!(
        (after[0] - before[0] - by).length() < 1e-3,
        "{before:?} → {after:?}"
    );
    assert_eq!(
        app.trajectory.turn, turn,
        "a pan of the unwrapped view leaves turn"
    );
    assert_eq!(
        first(&app).sphere,
        Orbit::default(),
        "nor orbits the sphere"
    );
    wheel(&mut h, &mut app, after[0], -1.0);
    assert!(close(first(&app).unwrapped.zoom, -OCTAVES_PER_LINE));
    let out = dots(&mut h, &mut app, place);
    assert!(
        (out[0] - after[0]).length() < 1e-2,
        "zoomed about the pointer"
    );
    double_click(&mut h, &mut app, place.center());
    assert_eq!(first(&app).unwrapped, PanZoom::default());
    sent_nothing(&app);
}

/// The focus walked to the Trajectory scope's element `id` and into interaction mode with it.
fn interact_with(h: &mut Headless, app: &mut App<MockSide>, id: &str) {
    app.view.focus.path = vec![TRAJECTORY.to_owned(), id.to_owned()];
    app.view.focus.mode = KeyMode::Navigation;
    press(h, app, Key::Enter, NONE);
    assert_eq!(app.view.focus.mode, KeyMode::Interaction, "Enter on {id}");
}

#[test]
fn mock_trajectory_views_the_keys_step_as_named() {
    let (mut app, mut h, _) = shown();
    // Enter reaches the views from the scope, in the order drawn.
    for _ in 0..4 {
        press(&mut h, &mut app, Key::Tab, NONE);
    }
    press(&mut h, &mut app, Key::Enter, NONE);
    for _ in 0..2 {
        press(&mut h, &mut app, Key::ArrowDown, NONE);
    }
    assert_eq!(app.view.focus.path, [TRAJECTORY, REAL_VIEW]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(app.view.focus.mode, KeyMode::Interaction);
    // Real space: → by 1/20 of the span (0.1 of the fitted half-width 1), Shift ×10, Alt ×0.1; ↑ up.
    let x = |app: &App<MockSide>| first(app).real.centre[0];
    let steps: Vec<f64> = [NONE, SHIFT, ALT]
        .into_iter()
        .map(|m| {
            let a = x(&app);
            press(&mut h, &mut app, Key::ArrowRight, m);
            x(&app) - a
        })
        .collect();
    let check = |steps: &[f64]| {
        assert!(
            close(steps[0], 0.1) && close(steps[1], 1.0) && close(steps[2], 0.01),
            "{steps:?}"
        )
    };
    check(&steps);
    rejects("another keyboard step (1/100 of the span)", || {
        check(&[0.02, 0.2, 0.002])
    });
    let y = first(&app).real.centre[1];
    press(&mut h, &mut app, Key::ArrowUp, NONE);
    assert!(
        close(first(&app).real.centre[1] - y, -0.1),
        "↑ moves the view up"
    );
    // + and −: a quarter octave, Shift ×10, Alt ×0.1, about the centre.
    let z = |app: &App<MockSide>| first(app).real.zoom;
    let c = first(&app).real.centre;
    for (key, m, want) in [
        (Key::Plus, NONE, 0.25),
        (Key::Minus, SHIFT, -2.5),
        (Key::Equals, ALT, 0.025),
    ] {
        let a = z(&app);
        press(&mut h, &mut app, key, m);
        assert!(close(z(&app) - a, want), "{key:?} {m:?}: {}", z(&app) - a);
    }
    assert_eq!(first(&app).real.centre, c, "the keys zoom about the centre");
    // Esc back to navigation mode; ↓ to the sphere, Enter: the arrows orbit by a degree, Shift ×10, Alt ×0.1.
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(app.view.focus.mode, KeyMode::Navigation);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    assert_eq!(app.view.focus.path, [TRAJECTORY, SPHERE_VIEW]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert!(app.trajectory.turn);
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    press(&mut h, &mut app, Key::ArrowUp, SHIFT);
    press(&mut h, &mut app, Key::ArrowLeft, ALT);
    let o = first(&app).sphere;
    assert!(
        close(o.yaw, 0.9) && close(o.pitch, SPHERE_TILT_DEG + 10.0),
        "{o:?}"
    );
    assert!(
        !app.trajectory.turn,
        "an orbit from the keys holds the view too"
    );
    press(&mut h, &mut app, Key::Plus, NONE);
    assert!(close(first(&app).sphere.zoom, 0.25));
    // In navigation mode + does nothing on a view.
    press(&mut h, &mut app, Key::Escape, NONE);
    press(&mut h, &mut app, Key::Plus, NONE);
    assert!(close(first(&app).sphere.zoom, 0.25));
    // The unwrapped view in the sphere's place is that element, with real space's keys.
    app.trajectory.unwrapped = true;
    interact_with(&mut h, &mut app, SPHERE_VIEW);
    let u = first(&app).unwrapped.centre[0];
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert!(
        close(first(&app).unwrapped.centre[0] - u, 0.1),
        "the Pan step"
    );
    assert_eq!(first(&app).sphere.yaw, o.yaw, "not the sphere's orbit");
    press(&mut h, &mut app, Key::Minus, NONE);
    assert!(close(first(&app).unwrapped.zoom, -0.25));
    let crumbs = app
        .keyboard
        .tree(engine::contract::view_ui::Mode::Explore)
        .labels(&app.view.focus.path);
    assert_eq!(crumbs, ["Trajectory", "unwrapped view"]);
    sent_nothing(&app);
}

#[test]
fn mock_trajectory_views_adjust_and_zoom_reach_only_the_views() {
    let mut s = Scratch::default();
    let adjust = |scope, kind: StepKind, direction| Adjust {
        scope,
        kind,
        times: direction_sign(direction),
        direction,
    };
    fn direction_sign(d: Direction) -> f64 {
        d.sign()
    }
    trajectory::adjust(
        &adjust(REAL_VIEW, StepKind::Angle, Direction::Right),
        10.0,
        &mut s,
    );
    trajectory::adjust(
        &adjust(SPHERE_VIEW, StepKind::Pan, Direction::Right),
        10.0,
        &mut s,
    );
    trajectory::adjust(
        &adjust(TRAJECTORY, StepKind::Pan, Direction::Right),
        10.0,
        &mut s,
    );
    assert_eq!(
        s.views_of(Tab::UnderCursor),
        Views::default(),
        "a mismatched step moves nothing"
    );
    assert!(s.turn);
    trajectory::zoom_view(TRAJECTORY, 1.0, &mut s);
    assert_eq!(s.views_of(Tab::UnderCursor), Views::default());
    // The unwrapped view shown, an Angle step on the sphere's place orbits nothing: it is the unwrapped view's.
    s.unwrapped = true;
    trajectory::adjust(
        &adjust(SPHERE_VIEW, StepKind::Angle, Direction::Right),
        10.0,
        &mut s,
    );
    assert_eq!(s.views_of(Tab::UnderCursor), Views::default());
    assert!(
        s.turn,
        "an orbit the unwrapped view does not take leaves turn"
    );
    s.unwrapped = false;
    // The zoom keys' presses: + and = in, − out, with the modifiers' multiples, releases ignored.
    let key = |key, pressed, modifiers| Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    };
    assert_eq!(
        zoom_presses(&[
            key(Key::Plus, true, NONE),
            key(Key::Plus, false, NONE),
            key(Key::Equals, true, SHIFT),
            key(Key::Minus, true, ALT),
            key(Key::A, true, NONE),
        ]),
        [1.0, 10.0, -0.1]
    );
}

#[test]
fn mock_trajectory_views_each_tab_keeps_its_own() {
    let mut s = Scratch::default();
    let mut kept = engine::contract::view_ui::KeptOrbits::default();
    let sample = crate::mock::sample::sample(
        &mock_app().snapshot().sim.plane.clone(),
        [0.4, 0.6],
        0.0,
        crate::mock::MOCK_T_MAX,
    );
    for _ in 0..3 {
        keep(&sample, &mut kept, &mut s);
    }
    s.views_mut(Tab::UnderCursor).real.zoom = 1.0;
    s.views_mut(Tab::Kept(1)).real.zoom = 2.0;
    s.views_mut(Tab::Kept(2)).real.zoom = 3.0;
    assert_eq!(s.views_of(Tab::Kept(0)), Views::default());
    assert_eq!(s.views_of(Tab::Kept(1)).real.zoom, 2.0);
    // Removing #2 takes its views; #3's move up with it.
    remove(1, &mut kept, &mut s);
    assert_eq!(s.views_of(Tab::Kept(1)).real.zoom, 3.0);
    assert_eq!(s.views_of(Tab::UnderCursor).real.zoom, 1.0);
    rejects("views left where the orbits moved", || {
        assert_eq!(s.views_of(Tab::Kept(1)).real.zoom, 2.0)
    });
    // A tab past the views has the defaults; removing it leaves the rest.
    remove(5, &mut kept, &mut s);
    let n = s.views.len();
    remove(1, &mut kept, &mut s);
    assert_eq!(s.views.len(), n - 1);
    s.views.truncate(1);
    remove(0, &mut kept, &mut s);
    assert_eq!(s.views.len(), 1, "a kept orbit with no views of its own");
    assert_eq!(Tab::UnderCursor.index(), 0);
    assert_eq!(Tab::Kept(3).index(), 4);
}

#[test]
fn mock_trajectory_views_the_inspector_panes_take_the_pointer() {
    let (mut app, mut h, _) = shown();
    let ctx = h.ctx.clone();
    app.open_window(&ctx, Window::Inspector, None);
    for _ in 0..3 {
        let _ = h.frame(&mut app, Vec::new());
    }
    assert!(
        app.windows.inspector.sample.is_some(),
        "the Inspector shows the panel's orbit"
    );
    let rect_of = |app: &App<MockSide>, k: usize| {
        app.keyboard
            .place_of(crate::windows::inspector::PANE_IDS[k].0)
            .expect("a pane placed")
            .rect
    };
    let (real, sphere) = (rect_of(&app, 2), rect_of(&app, 1));
    drag(&mut h, &mut app, real.center(), vec2(12.0, 0.0));
    // Pane 3 is square: its half-width is half its side.
    let want = -f64::from(12.0 / (real.width() / 2.0));
    let got = app.windows.inspector.views.real.centre;
    assert!(
        close(got[0], want) && got[1] == 0.0,
        "{got:?}, not [{want}, 0]"
    );
    assert!(app.windows.inspector.turn);
    drag(&mut h, &mut app, sphere.center(), vec2(8.0, 0.0));
    assert!(close(
        app.windows.inspector.views.sphere.yaw,
        8.0 * ORBIT_DEG_PER_POINT
    ));
    assert!(
        !app.windows.inspector.turn,
        "a drag on pane 2 turned turn off"
    );
    double_click(&mut h, &mut app, real.center());
    assert_eq!(app.windows.inspector.views.real, PanZoom::default());
    // Pane 1 is the IC: a drag on it moves no view.
    let ic = rect_of(&app, 0);
    drag(&mut h, &mut app, ic.center(), vec2(12.0, 0.0));
    assert_eq!(
        app.windows.inspector.views.real,
        PanZoom::default(),
        "a drag on the IC pane panned the real-space view"
    );
    // Pane 2 unwrapped pans by its own half-widths.
    app.windows.inspector.unwrapped = true;
    let _ = h.frame(&mut app, Vec::new());
    let flat = rect_of(&app, 1);
    drag(&mut h, &mut app, flat.center(), vec2(10.0, -8.0));
    let got = app.windows.inspector.views.unwrapped.centre;
    let (hx, hy) = (flat.width() / 2.0, flat.height() / 2.0);
    assert!(
        close(got[0], -f64::from(10.0 / hx)) && close(got[1], f64::from(8.0 / hy)),
        "{got:?}"
    );
    rejects("a pane that only paints", || {
        assert_eq!(app.windows.inspector.views.sphere, Orbit::default())
    });
    sent_nothing(&app);
}

/// The filled circles among `shapes`, and the stroked ones' radii.
fn circles(shapes: &[egui::Shape]) -> (Vec<Pos2>, Vec<f32>) {
    let mut filled = Vec::new();
    let mut outlines = Vec::new();
    for s in shapes {
        if let egui::Shape::Circle(c) = s {
            if c.fill == egui::Color32::TRANSPARENT {
                outlines.push(c.radius);
            } else {
                filled.push(c.center);
            }
        }
    }
    (filled, outlines)
}

#[test]
fn mock_trajectory_views_draw_through_their_views() {
    use super::mock_windows_drawn::painted;
    let s = crate::mock::sample::sample(
        &mock_app().snapshot().sim.plane.clone(),
        [0.4, 0.6],
        0.0,
        crate::mock::MOCK_T_MAX,
    );
    let rect = Rect::from_min_size(egui::pos2(100.0, 100.0), vec2(200.0, 160.0));
    let r = 80.0_f32;
    let i = 7;
    let near = |a: Pos2, b: Pos2| (a - b).length() < 1e-3;
    // The sphere: its outline 0.86 r at the default, twice that zoomed in an octave; the playhead's point where the
    // orbit's view puts it.
    let orbit = Orbit {
        yaw: 25.0,
        pitch: -40.0,
        zoom: 1.0,
    };
    let (dot, outline) = circles(&painted(|ui| {
        trajectory::sphere(ui, rect, &s, 10.0, i, &orbit)
    }));
    let (_, rest) = circles(&painted(|ui| {
        trajectory::sphere(ui, rect, &s, 10.0, i, &Orbit::default())
    }));
    assert!((rest[0] - 0.86 * r).abs() < 1e-3, "{rest:?}");
    assert!((outline[0] - 2.0 * 0.86 * r).abs() < 1e-3, "{outline:?}");
    let ([x, y], _) = view_at(on_sphere(s.shape[i]), 35.0, -40.0);
    let want = trajectory::on_square(rect, [x * 0.86 * 2.0, y * 0.86 * 2.0]);
    assert!(near(dot[0], want), "{dot:?} {want:?}");
    rejects("a sphere drawn without its orbit", || {
        let ([x, y], _) = view(on_sphere(s.shape[i]), 10.0);
        assert!(near(
            dot[0],
            trajectory::on_square(rect, [x * 0.86, y * 0.86])
        ))
    });
    // Real space: the bodies where the pan and zoom put the fitted points.
    let pan = PanZoom {
        centre: [0.2, -0.1],
        zoom: 0.5,
    };
    let (dots, _) = circles(&painted(|ui| trajectory::real_space(ui, rect, &s, i, &pan)));
    let [x0, y0, x1, y1] = trajectory::fit(&s.bodies);
    for (k, d) in dots.iter().enumerate() {
        let p = s.bodies[k][i];
        let (x, y) = (
            2.0 * (p[0] - x0) / (x1 - x0) - 1.0,
            2.0 * (p[1] - y0) / (y1 - y0) - 1.0,
        );
        let [vx, vy] = pan.apply([x, -y]);
        let want = rect.center() + vec2(vx as f32 * r, vy as f32 * r);
        assert!(near(*d, want), "body {k}: {d:?} {want:?}");
    }
    // The unwrapped view: the playhead's point where the pan and zoom put its place on the map.
    let (dot, _) = circles(&painted(|ui| trajectory::unwrapped(ui, rect, &s, i, &pan)));
    let flat = unwrapped_point(rect, s.shape[i]);
    let half = rect.size() / 2.0;
    let [vx, vy] = pan.apply([
        f64::from((flat.x - rect.center().x) / half.x),
        f64::from((flat.y - rect.center().y) / half.y),
    ]);
    let want = rect.center() + vec2(vx as f32 * half.x, vy as f32 * half.y);
    assert!(near(dot[0], want), "{dot:?} {want:?}");
    rejects("an unwrapped view drawn unpanned", || {
        assert!(near(dot[0], flat))
    });
}
