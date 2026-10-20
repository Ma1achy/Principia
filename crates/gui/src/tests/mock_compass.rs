//! The compass (render_gui_spec §G2, §G3, §G4, § "The compass: look and views (R-451, R-452)", read with R-409 and
//! R-451): `mock_compass_modes` (REQ-GUI-181, re-checked by TASK-M6-33): it opens in Tilt, its buttons switch the mode
//! and follow the slider touched, a tilt-mode drag anywhere tilts and a slice-mode drag anywhere moves the plane along
//! its drawn normal, one `SetField` per drag, and nothing turns the view, which is the reference's isometric
//! projection; and `mock_compass_keys` (REQ-GUI-183, re-checked): the compass, Tilt and Slice as siblings, Enter on a
//! button setting the mode, Enter on the compass starting interaction, its arrows tilting or slicing by the mode,
//! Shift ×10 and Alt ×0.1. Each check also runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{self, pos2, Key, Modifiers, Rect};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Lock, Plane};
use engine::contract::view_ui::{Focus, KeyMode, Mode};

use super::compass_support::{
    app_at, check_painted, compass_rect, compass_shapes, drag, drawing, off_cube, on_slice,
    painted_edges, plane, setfields, settle, square, view,
};
use super::support::{focus, headless, mock_app, nodes, press, rejects};
use crate::app::App;
use crate::explore::compass::{
    self, after, drag_edit, latent_normal, slice, tilt, Compass, Drag, Iso, DRAG_DEGREES,
    HINT_SLICE, HINT_TILT, MARGIN, SLICE_BUTTON, TILT_BUTTON, TITLE,
};
use crate::explore::manifold_view::slice_tilt::Angles;
use crate::explore::manifold_view::{
    add, basis_edit, dot, lock_edit, norm, z0_edit, Out, Scratch, Touch, View, SLICE_AXIS,
    Z0_HALF_RANGE,
};
use crate::explore::{COMPASS, COMPASS_SLICE, COMPASS_TILT, COMPASS_VIEW};
use crate::headless::Headless;
use crate::keyboard::keymap::{Command, Direction};
use crate::keyboard::scopes::{Adjust, Base, Scope, ScopeTree, StepKind};
use crate::mock::{MOCK_Q1, MOCK_Q2, MOCK_Z0};
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;

/// The square of the compass's tests that need none of the app's.
const SQUARE: Rect = Rect::from_min_max(pos2(100.0, 200.0), pos2(300.0, 400.0));

/// A plane tilted by `τ₁`, `τ₂` and turned by `γ`, in degrees, through the frame's point `(0.1, −0.2, 0.2)`.
fn tilted(t1: f64, t2: f64, g: f64) -> Plane {
    plane(&view(t1, t2, g, 1.0), [0.1, -0.2, 0.2])
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

/// Asserts one `SetField`, on `field`.
fn check_one(fields: &[&str], field: &str) {
    assert_eq!(fields, [field], "one SetField on {field}");
}

#[test]
fn mock_compass_tilt_edit() {
    let p = tilted(0.0, 0.0, 0.0);
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

// --- the normal and the slice edit (REQ-GUI-181; R-409 A3; R-451 B3) -------------------------------------------------

/// Asserts that `edit` on `p` sets `z₀` and moves it along the drawn plane's normal, its components along `q₁` and `q₂`
/// zero within 1e-12; returns how far, signed.
fn along_normal(p: &Plane, edit: &SetField) -> f64 {
    let Edit::Sim(SimField::Z0(z0)) = &edit.edit else {
        panic!("a SetField on z₀, not {edit:?}")
    };
    let delta = add(z0, -1.0, &p.z0);
    let unit = |q: &Latent| q.map(|x| x / norm(q));
    assert!(
        dot(&delta, &unit(&p.q1)).abs() <= 1e-12 && dot(&delta, &unit(&p.q2)).abs() <= 1e-12,
        "Δz₀ {delta:?} has a part in the plane"
    );
    dot(&delta, &latent_normal(p).expect("a drawn plane"))
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
    let n = latent_normal(&tilted(0.0, 0.0, 0.0)).expect("a plane");
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
        let ln = latent_normal(&p).expect("a plane");
        assert!((norm(&ln) - 1.0).abs() < 1e-12, "unit");
        // The drawn plane's normal is the basis's own: q₁ and q₂ have no part along it.
        assert!(dot(&ln, &p.q1).abs() < 1e-12 && dot(&ln, &p.q2).abs() < 1e-12);
        check_along_normal(&p, &slice(&p, 0.3).expect("an edit"), 0.3, 1e-12);
        check_along_normal(&p, &slice(&p, -0.05).expect("an edit"), -0.05, 1e-12);
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
fn mock_compass_modes_a_drag_is_one_edit_by_its_mode() {
    let p = tilted(20.0, -10.0, 15.0);
    let before = Angles::of(&View::of(&p));
    let mut tilting = Drag::start(Touch::Tilt, &p);
    tilting.moved(egui::vec2(20.0, 10.0), SQUARE);
    let (t1, t2) = angles_of(&p, &drag_edit(&p, &tilting).expect("an edit"));
    let (w, h) = (f64::from(SQUARE.width()), f64::from(SQUARE.height()));
    assert!(
        (t1 - before.tau1 - 20.0 / w * DRAG_DEGREES).abs() < 1e-9
            && (t2 - before.tau2 + 10.0 / h * DRAG_DEGREES).abs() < 1e-9,
        "right raises τ₁, down lowers τ₂: ({t1}, {t2})"
    );
    let mut slicing = Drag::start(Touch::Slice, &p);
    slicing.moved(egui::vec2(-6.0, 9.0), SQUARE);
    let edit = drag_edit(&p, &slicing).expect("an edit");
    check_along_normal(&p, &edit, -6.0 / w * 2.0 * Z0_HALF_RANGE, 1e-12);
    for mode in [Touch::Tilt, Touch::Slice] {
        let still = Drag::start(mode, &p);
        assert_eq!(drag_edit(&p, &still), None, "no travel, no edit");
    }
    let mut upright = Drag::start(Touch::Slice, &p);
    upright.moved(egui::vec2(0.0, 30.0), SQUARE);
    assert_eq!(
        drag_edit(&p, &upright),
        None,
        "a vertical slice drag, no edit"
    );
    rejects("a slice-mode drag that tilts", || {
        along_normal(&p, &drag_edit(&p, &tilting).expect("an edit"));
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

#[test]
fn mock_compass_square_sits_in_the_margin() {
    let rect = Rect::from_min_size(pos2(10.0, 20.0), egui::vec2(400.0, 200.0));
    assert_eq!(
        compass::square(rect),
        Rect::from_min_size(pos2(22.0, 32.0), egui::vec2(176.0, 176.0)),
        "as tall as the margin leaves"
    );
    let narrow = Rect::from_min_size(pos2(10.0, 20.0), egui::vec2(200.0, 400.0));
    assert_eq!(
        compass::square(narrow),
        Rect::from_min_size(pos2(22.0, 32.0), egui::vec2(100.0, 100.0)),
        "at most half the width"
    );
    // A whole number of points: 01_main's compass leaves 172.67 points, and the square is 172.
    let app_rect = Rect::from_min_size(pos2(0.0, 680.0), egui::vec2(354.0, 196.666_67));
    assert_eq!(compass::square(app_rect).width(), 172.0);
    rejects("a square without its margin", || {
        assert_eq!(compass::square(rect).min, rect.min)
    });
    rejects("a square of a fraction of a point", || {
        assert_eq!(compass::square(app_rect).width(), 172.666_67)
    });
}

#[test]
fn mock_compass_draws_the_cube_the_plane_and_the_pin() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    settle(&mut h, &mut app);
    let free = compass_shapes(&mut h, &mut app);
    let drawn = drawing(&h, &app);
    assert_eq!(drawn.iso, Iso::fit(square(&h)), "the reference's fit");
    assert!(drawn.slice.is_some() && drawn.pin.is_none());
    check_painted(&free, &drawn);
    app.set_field(lock_edit(Lock {
        locked: true,
        z_locked: MOCK_Z0,
    }));
    let _ = h.frame(&mut app, Vec::new());
    settle(&mut h, &mut app);
    let locked = compass_shapes(&mut h, &mut app);
    let pinned = drawing(&h, &app);
    assert!(pinned.pin.is_some(), "locked, a pin");
    check_painted(&locked, &pinned);
    rejects("the free compass read as locked", || {
        check_painted(&free, &pinned)
    });
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
            drag: None,
            shown: None,
            moving: false,
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
    settle(&mut h, &mut app);
    let rect = compass_rect(&h);
    let sq = square(&h);
    let all = nodes(&mut h, &mut app);
    let text =
        |all: &[super::support::Node], t: &str| all.iter().find(|n| n.text == t).map(|n| n.rect);
    let tilt_lines = drawing(&h, &app).readout;
    assert_eq!(
        tilt_lines,
        ["mode tilt", "q₁→z_q3 +0.0° · q₂→z_q3 +0.0°", "γ +0.0°"]
    );
    for t in [TITLE, TILT_BUTTON, SLICE_BUTTON, HINT_TILT]
        .into_iter()
        .chain(tilt_lines.iter().map(String::as_str))
    {
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
    app.manifold.compass.mode = Touch::Slice;
    let all = nodes(&mut h, &mut app);
    assert!(text(&all, HINT_SLICE).is_some() && text(&all, HINT_TILT).is_none());
    let slice_lines = drawing(&h, &app).readout;
    assert_eq!(
        slice_lines,
        ["mode slice · plane ⟂ z_q3", "z_q3 = +0.31 of range"]
    );
    for t in &slice_lines {
        assert!(text(&all, t).is_some(), "{t:?} read out");
    }
    rejects("the tilt readout while slicing", || {
        assert!(text(&all, &tilt_lines[1]).is_some())
    });
}

/// The app with the plane tilted by `τ₁` 30°, `τ₂` −20° and turned by `γ` 25°, at rest.
fn tilted_app() -> (App<MockSide>, Headless) {
    app_at(&tilted(30.0, -20.0, 25.0))
}

/// Asserts the twelve cube edges painted in `shapes` are the reference's, fitted to `sq`, in its order: the view is
/// fixed, whatever the plane.
fn check_cube(shapes: &[egui::Shape], sq: Rect) {
    let still = compass::drawing(
        sq,
        &tilted(0.0, 0.0, 0.0),
        &view(0.0, 0.0, 0.0, 1.0),
        Touch::Tilt,
        &free_lock(),
    );
    let edges: Vec<_> = painted_edges(shapes, &still)
        .into_iter()
        .map(|(e, _)| e)
        .collect();
    let want: Vec<_> = still
        .items
        .iter()
        .filter_map(|i| match i {
            compass::Item::Edge { from, to, .. } => Some([*from, *to]),
            compass::Item::Slice => None,
        })
        .collect();
    assert_eq!(edges, want, "the cube's edges in the reference's view");
}

#[test]
fn mock_compass_modes_a_tilt_drag_anywhere_tilts_once() {
    for on in [true, false] {
        let (mut app, mut h) = tilted_app();
        let sq = square(&h);
        let before = app.snapshot().sim.plane.clone();
        let from = if on {
            on_slice(&drawing(&h, &app))
        } else {
            off_cube(sq)
        };
        let rows = app.console().len();
        drag(&mut h, &mut app, from, egui::vec2(20.0, 10.0));
        let fields = setfields(&app, rows);
        check_one(&fields, "basis");
        let after = app.snapshot().sim.plane.clone();
        assert_eq!(after.z0, before.z0, "none on z₀");
        let (b, a) = (
            Angles::of(&View::of(&before)),
            Angles::of(&View::of(&after)),
        );
        let (w, hgt) = (f64::from(sq.width()), f64::from(sq.height()));
        assert!(
            (a.tau1 - b.tau1 - 20.0 / w * DRAG_DEGREES).abs() < 1e-4
                && (a.tau2 - b.tau2 + 10.0 / hgt * DRAG_DEGREES).abs() < 1e-4,
            "right tilts τ₁ up, down tilts τ₂ down: {b:?} → {a:?}"
        );
        assert_eq!(app.manifold.compass.mode, Touch::Tilt);
        assert_eq!(app.manifold.compass.drag, None, "the drag ended");
        settle(&mut h, &mut app);
        // The view is fixed: the cube is drawn as before.
        check_cube(&compass_shapes(&mut h, &mut app), sq);
        rejects("a drag of a SetField a frame", || {
            check_one(&["basis", "basis"], "basis")
        });
        rejects("a tilt-mode drag that slices", || check_one(&fields, "z0"));
    }
}

#[test]
fn mock_compass_modes_a_slice_drag_anywhere_moves_along_the_normal() {
    for on in [true, false] {
        let (mut app, mut h) = tilted_app();
        click_button(&mut h, &mut app, SLICE_BUTTON);
        let sq = square(&h);
        let before = app.snapshot().sim.plane.clone();
        let from = if on {
            on_slice(&drawing(&h, &app))
        } else {
            off_cube(sq)
        };
        let rows = app.console().len();
        drag(&mut h, &mut app, from, egui::vec2(-12.0, 9.0));
        check_one(&setfields(&app, rows), "z0");
        let after = app.snapshot().sim.plane.clone();
        assert_eq!(
            (after.q1, after.q2),
            (before.q1, before.q2),
            "the basis unchanged"
        );
        let amount = -12.0 / f64::from(sq.width()) * 2.0 * Z0_HALF_RANGE;
        check_along_normal(&before, &z0_edit(after.z0), amount, 1e-6);
        assert_eq!(app.manifold.compass.mode, Touch::Slice);
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
    let sq = square(&h);
    let before = app.snapshot().sim.plane.clone();
    let from = off_cube(sq);
    let delta = egui::vec2(-12.0, 9.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    for events in [
        vec![egui::Event::PointerMoved(from)],
        vec![button(from, true)],
        vec![egui::Event::PointerMoved(from + delta)],
    ] {
        let _ = h.frame(&mut app, events);
    }
    let depth = app.snapshot().history.undo_depth;
    settle(&mut h, &mut app);
    assert_eq!(app.snapshot().history.undo_depth, depth, "no edit yet");
    let mut d = Drag::start(Touch::Slice, &before);
    d.moved(delta, sq);
    let shown = after(&before, &[drag_edit(&before, &d).expect("an edit")]);
    let mid = drawing(&h, &app);
    let want = compass::drawing(sq, &shown, &View::of(&shown), Touch::Slice, &free_lock());
    assert_eq!(mid.slice, want.slice, "the plane the drag leaves");
    let at_start = compass::drawing(sq, &before, &View::of(&before), Touch::Slice, &free_lock());
    rejects("the plane drawn where the drag began", || {
        assert_eq!(mid.slice, at_start.slice)
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
    let sq = square(&h);
    let before = app.snapshot().sim.plane.clone();
    assert_eq!(before.z0, anchor);
    let rows = app.console().len();
    drag(&mut h, &mut app, off_cube(sq), egui::vec2(8.0, -14.0));
    check_one(&setfields(&app, rows), "z0");
    let snapshot = app.snapshot();
    assert!(snapshot.sim.lock.locked, "still locked");
    assert_eq!(snapshot.sim.lock.z_locked, anchor, "the anchor kept");
    // The excursion is along the normal through the anchor.
    let n = latent_normal(&before).expect("a plane");
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
    // Off the anchor by +0.4 q₁ − 0.2 q₂: the anchor's chart point is (0.3, 0.6), where z(0.3, 0.6) is the anchor.
    app.set_field(z0_edit(add(&add(&MOCK_Z0, 0.4, &MOCK_Q1), -0.2, &MOCK_Q2)));
    let _ = h.frame(&mut app, Vec::new());
    settle(&mut h, &mut app);
    let plane = app.snapshot().sim.plane.clone();
    let view = View::of(&plane);
    let want = compass::point_of(&MOCK_Z0, &view);
    let at = compass::pin(&plane, &view, &lock).expect("locked");
    assert!(
        at.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-12),
        "{at:?} ≠ {want:?}"
    );
    assert_eq!(compass::pin(&plane, &view, &free_lock()), None, "free");
    let drawn = drawing(&h, &app);
    assert_eq!(drawn.pin, Some(at));
    check_painted(&compass_shapes(&mut h, &mut app), &drawn);
    rejects("the pin at z₀'s point", || {
        let z0 = compass::point_of(&plane.z0, &view);
        assert!(at.iter().zip(z0).all(|(a, b)| (a - b).abs() < 1e-12))
    });
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
    settle(&mut h, &mut app);
    check_cube(&compass_shapes(&mut h, &mut app), square(&h));
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
    rejects("Shift+arrows that edit nothing", || {
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
    let run = |p: &Plane, mode, direction, times| {
        let mut scratch = Scratch::default();
        scratch.compass.mode = mode;
        let mut out = Out::default();
        let a = Adjust {
            scope: COMPASS_VIEW,
            kind: StepKind::Tilt,
            times,
            direction,
        };
        compass::adjust(&a, p, &mut scratch, &mut out);
        out.edits
    };
    let one = |edits: Vec<SetField>| {
        assert_eq!(edits.len(), 1);
        edits.into_iter().next().expect("one edit")
    };
    let (t1, t2) = angles_of(&p, &one(run(&p, Touch::Tilt, Direction::Left, -10.0)));
    assert!(t1.abs() < 1e-9 && (t2 - 5.0).abs() < 1e-9, "({t1}, {t2})");
    let (t1, t2) = angles_of(&p, &one(run(&p, Touch::Tilt, Direction::Down, -1.0)));
    assert!(
        (t1 - 10.0).abs() < 1e-9 && (t2 - 4.0).abs() < 1e-9,
        "({t1}, {t2})"
    );
    check_along_normal(
        &p,
        &one(run(&p, Touch::Slice, Direction::Down, -0.1)),
        -0.002,
        1e-12,
    );
    // Where the compass draws no plane, a slice-mode arrow sends nothing (R-451 B11); a tilt-mode one still tilts.
    let corner = tilted(90.0, 90.0, 0.0);
    assert!(run(&corner, Touch::Slice, Direction::Up, 1.0).is_empty());
    assert_eq!(run(&corner, Touch::Tilt, Direction::Left, -1.0).len(), 1);
    rejects("the slice's step taken as the tilt's degrees", || {
        check_along_normal(
            &p,
            &one(run(&p, Touch::Slice, Direction::Up, 1.0)),
            1.0,
            1e-12,
        )
    });
    rejects("a slice-mode arrow at a corner that moves z₀", || {
        assert_eq!(run(&corner, Touch::Slice, Direction::Up, 1.0).len(), 1)
    });
}
