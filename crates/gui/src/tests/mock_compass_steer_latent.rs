//! `mock_compass_steer_latent` (REQ-GUI-197; render_gui_spec § "The compass: look and views (R-451, R-452)", §G4;
//! R-452 C5): a click on Plücker cell (i, j) sets the slice plane to `z_i × z_j`, the tilts and `γ` 0, the zoom and `z₀`
//! kept, one `SetField` on the basis, logged in the console, the Manifold view showing `z_i × z_j`, undone exactly by
//! one Ctrl+Z; locked, the plane turns about the pin. Each check also runs on an input it must reject (R-176, through
//! `rejects`).

use eframe::egui::{Key, Modifiers};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Lock, Plane, LATENT_DIM};

use super::compass_plucker::plucker_app;
use super::compass_support::{plane, setfields, square, view};
use super::support::{frame_texts, press, rejects};
use crate::app::App;
use crate::explore::compass;
use crate::explore::compass::views::{plucker, steer, upper, GridPlace};
use crate::explore::manifold_view::tilt::Angles;
use crate::explore::manifold_view::SLICE_AXIS;
use crate::explore::manifold_view::{about_the_pin, lock_edit, point, View, AXES};
use crate::headless::Headless;
use crate::side::MockSide;

/// The plane the steer tests start from: tilted, turned and zoomed, off the frame's centre.
fn start() -> Plane {
    plane(&view(20.0, -15.0, 10.0, 0.8), [0.1, -0.3, 0.25])
}

/// Ctrl+Z, the contract's undo (§G3; R-409 G5), then a frame to read the snapshot it changed.
fn ctrl_z(h: &mut Headless, app: &mut App<MockSide>) {
    press(h, app, Key::Z, Modifiers::CTRL);
    let _ = h.frame(app, Vec::new());
}

/// Clicks cell `(i, j)` of the Plücker grid, then one frame for the snapshot to follow.
fn click_cell(h: &mut Headless, app: &mut App<MockSide>, i: usize, j: usize) {
    let at = GridPlace::fit(square(h)).cell_rect(i, j).center();
    for events in crate::capture::click(at) {
        let _ = h.frame(app, events);
    }
    let _ = h.frame(app, Vec::new());
}

/// Asserts `after` is the steer to `(i, j)` from `before`: its grid lights `(i, j)` at ±1, it reads as `z_i × z_j`
/// with `τ₁ = τ₂ = γ = 0` and `before`'s scale, and `z₀` is `before`'s.
fn check_steered(before: &Plane, after: &Plane, (i, j): (usize, usize)) {
    let grid = plucker(after);
    for (a, b) in upper() {
        let want = if (a, b) == (i, j) { 1.0 } else { 0.0 };
        assert!(
            (grid[a][b].abs() - want).abs() <= 1e-12,
            "steered to ({i}, {j}): cell ({a}, {b}) {}",
            grid[a][b]
        );
    }
    let v = View::of(after);
    assert_eq!((v.a, v.b), (i, j), "the chart's axes");
    assert_eq!(v.name(), format!("{} × {}", AXES[i], AXES[j]));
    let angles = Angles::of(&v);
    assert_eq!(
        (angles.tau1, angles.tau2, angles.gamma),
        (0.0, 0.0, 0.0),
        "the tilts reset"
    );
    let was = View::of(before).scale;
    assert!(
        (v.scale - was).abs() <= 1e-15 * was,
        "the zoom kept: {} for {was}",
        v.scale
    );
    assert_eq!(after.z0, before.z0, "z₀ kept");
}

#[test]
fn mock_compass_steer_latent_every_cell_steers_to_its_coordinate_plane() {
    // One app, each steer undone before the next.
    let (mut app, mut h) = plucker_app(&start());
    let before = app.snapshot().sim.plane.clone();
    for (i, j) in upper() {
        assert_eq!(app.snapshot().sim.plane, before);
        let (rows, depth) = (app.console().len(), app.snapshot().history.undo_depth);
        click_cell(&mut h, &mut app, i, j);
        let after = app.snapshot().sim.plane.clone();
        // A plane that takes the slice normal moves it in the same edit, a frame edit carrying the basis (R-459 G4).
        if i == SLICE_AXIS || j == SLICE_AXIS {
            assert_eq!(setfields(&app, rows), ["other"], "one SetField");
        } else {
            check_one_basis(&setfields(&app, rows));
        }
        assert_eq!(app.snapshot().history.undo_depth, depth + 1);
        assert!(
            app.console()[rows..]
                .iter()
                .any(|e| e.message.starts_with("SetField ") && e.message.contains("Plane.q1")),
            "the steer logged in the console"
        );
        check_steered(&before, &after, (i, j));
        // The Manifold view's Chart shows the pair.
        let name = format!("{} × {}", AXES[i], AXES[j]);
        assert!(
            frame_texts(&mut h, &mut app).contains(&name),
            "Chart shows {name}"
        );
        // One Ctrl+Z restores the basis exactly.
        ctrl_z(&mut h, &mut app);
        assert_eq!(app.snapshot().sim.plane, before, "undone by one Ctrl+Z");
        assert_eq!(app.snapshot().history.undo_depth, depth);
    }
    // Controls: a steer that leaves a tilt, and one that emits two SetFields.
    let p = start();
    let mut tilted = plane(&view(10.0, 0.0, 0.0, 0.8), [0.0; 3]);
    tilted.z0 = p.z0;
    rejects("a steer that leaves a tilt", || {
        check_steered(&p, &tilted, (0, 1))
    });
    rejects("a steer of two SetFields", || {
        check_one_basis(&["z0", "basis"])
    });
}

/// Asserts the `SetField`s logged are one, on the basis.
fn check_one_basis(fields: &[&str]) {
    assert_eq!(fields, ["basis"], "one SetField on the basis");
}

#[test]
fn mock_compass_steer_latent_the_diagonal_and_the_lower_triangle_steer_nothing() {
    let p = start();
    for i in 0..LATENT_DIM {
        for j in 0..=i {
            assert_eq!(steer(View::of(&p).scale, i, j), None, "({i}, {j})");
        }
    }
    assert_eq!(
        steer(View::of(&p).scale, 0, LATENT_DIM),
        None,
        "off the grid"
    );
    assert!(steer(View::of(&p).scale, 6, 7).is_some());
    let (mut app, mut h) = plucker_app(&p);
    let (rows, depth) = (app.console().len(), app.snapshot().history.undo_depth);
    for (i, j) in [(0, 0), (3, 3), (7, 7), (5, 2), (7, 0)] {
        click_cell(&mut h, &mut app, i, j);
    }
    assert!(
        setfields(&app, rows).is_empty(),
        "a click on the diagonal or below"
    );
    assert_eq!(app.snapshot().history.undo_depth, depth);
    // Control: the diagonal click's check, on a click that steered.
    click_cell(&mut h, &mut app, 2, 3);
    rejects("a diagonal click that steers", || {
        assert!(setfields(&app, rows).is_empty())
    });
}

/// The basis of a `SetField`.
fn basis_of(edit: &SetField) -> Option<([f64; 8], [f64; 8])> {
    match &edit.edit {
        Edit::Sim(SimField::Basis { q1, q2 }) => Some((*q1, *q2)),
        _ => None,
    }
}

#[test]
fn mock_compass_steer_latent_is_the_basis_alone() {
    let p = start();
    let edit = steer(View::of(&p).scale, 2, 6).expect("a steer");
    let (q1, q2) = basis_of(&edit).expect("a basis edit");
    let s = View::of(&p).scale;
    let mut want1 = [0.0; 8];
    let mut want2 = [0.0; 8];
    want1[2] = s;
    want2[6] = s;
    assert_eq!((q1, q2), (want1, want2));
    assert!(!edit.no_history, "undoable");
}

#[test]
fn mock_compass_steer_latent_locked_turns_about_the_pin() {
    // Locked at the centre, as K locks: the steer is the basis alone, the anchor and z₀ kept.
    let (mut app, mut h) = plucker_app(&start());
    let z0 = app.snapshot().sim.plane.z0;
    app.set_field(lock_edit(Lock {
        locked: true,
        z_locked: z0,
    }));
    let _ = h.frame(&mut app, Vec::new());
    let before = app.snapshot().sim.plane.clone();
    let rows = app.console().len();
    click_cell(&mut h, &mut app, 1, 6);
    check_one_basis(&setfields(&app, rows));
    check_steered(&before, &app.snapshot().sim.plane, (1, 6));
    assert_eq!(app.snapshot().sim.lock.z_locked, z0, "the anchor kept");
    assert!(app.snapshot().sim.lock.locked);
    ctrl_z(&mut h, &mut app);
    assert_eq!(app.snapshot().sim.plane, before, "undone by one Ctrl+Z");
    // Locked off the centre (the slice moved since): the pin's chart point keeps its latent point.
    let (mut app, mut h) = plucker_app(&start());
    let p = app.snapshot().sim.plane.clone();
    let anchor = point(&p, 0.3, 0.7);
    app.set_field(lock_edit(Lock {
        locked: true,
        z_locked: anchor,
    }));
    let _ = h.frame(&mut app, Vec::new());
    click_cell(&mut h, &mut app, 0, 4);
    let after = app.snapshot().sim.plane.clone();
    let check_pin = |q: &Plane| {
        let at = point(q, 0.3, 0.7);
        for k in 0..LATENT_DIM {
            assert!(
                (at[k] - anchor[k]).abs() <= 1e-12,
                "the pin's point moved: {at:?}"
            );
        }
    };
    check_pin(&after);
    assert_eq!(app.snapshot().sim.lock.z_locked, anchor, "the anchor kept");
    let v = View::of(&after);
    assert_eq!((v.a, v.b), (0, 4));
    // Control: the steer sent as it is, not about the pin, moves the pin's point.
    let edit = steer(View::of(&p).scale, 0, 4).expect("a steer");
    let free = compass::after(&p, std::slice::from_ref(&edit));
    rejects("a locked steer not about the pin", || check_pin(&free));
    let sim = &app.snapshot().sim;
    let mut locked_sim = sim.clone();
    locked_sim.plane = p.clone();
    assert_eq!(
        about_the_pin(vec![edit], &locked_sim).len(),
        2,
        "z₀ then the basis"
    );
}

#[test]
fn mock_compass_steer_latent_an_axis_pair_reads_as_its_own_chart() {
    use crate::explore::manifold_view::{axis, axis_pair};
    let at = |q1, q2| Plane {
        z0: [0.0; LATENT_DIM],
        q1,
        q2,
    };
    // A steered pair, a preset's or not, reads as itself untilted.
    for (i, j) in upper() {
        let v = View::of(&at(axis(i, 0.5), axis(j, 0.5)));
        assert_eq!((v.a, v.b, v.tau1, v.tau2, v.gamma), (i, j, 0.0, 0.0, 0.0));
    }
    assert_eq!(View::of(&at(axis(0, 1.0), axis(5, 1.0))).preset, None);
    assert_eq!(View::of(&at(axis(4, 1.0), axis(5, 1.0))).preset, Some(2));
    // Only an ascending pair along two axes alone.
    assert_eq!(axis_pair(&axis(2, 1.0), &axis(6, -3.0)), Some((2, 6)));
    assert_eq!(axis_pair(&axis(6, 1.0), &axis(2, 1.0)), None, "descending");
    assert_eq!(axis_pair(&axis(3, 1.0), &axis(3, 2.0)), None, "one axis");
    let mut two = axis(1, 1.0);
    two[4] = 1e-30;
    assert_eq!(axis_pair(&two, &axis(5, 1.0)), None, "two components");
    assert_eq!(axis_pair(&axis(1, 1.0), &two), None, "two components");
    assert_eq!(axis_pair(&[0.0; 8], &axis(5, 1.0)), None, "none");
    // A descending or degenerate pair reads as before: in the first preset's frame that holds it.
    assert_eq!(View::of(&at(axis(5, 1.0), axis(0, 1.0))).preset, Some(0));
    assert_eq!(View::of(&at(axis(0, 1.0), axis(0, 2.0))).preset, Some(0));
    rejects("z_α × z_q3 read as z_α × z_β tilted", || {
        let v = View::of(&at(axis(0, 1.0), axis(5, 1.0)));
        assert_eq!((v.a, v.b), (0, 1));
    });
}
