//! `mock_compass_steer_physical` (REQ-GUI-200; render_gui_spec § "The compass: look and views (R-451, R-452)"; R-550,
//! R-633): a click on a physical cell steers to a plane whose pushforward lights the cell at ±1; a cell needing the
//! energy shell's normal is refused, logged, with no `SetField`; each steer is one `SetField`, logged, undone by one
//! Ctrl+Z; the Shape view's pole refuses, naming it. Each check also runs on an input it must reject (R-176, through
//! `rejects`).

use eframe::egui::{Event, Key, Modifiers};
use engine::contract::set_field::{Edit, SimField};
use engine::contract::sim_config::{Latent, Links, Lock, Plane, SimConfig, LATENT_DIM};

use super::compass_physical_views::{leaked, linear_a, missing};
use super::compass_support::{app_at, plane, setfields, square, view};
use super::support::{click_scope, press, rejects};
use crate::app::App;
use crate::explore::compass::physical::{
    self, latent_jacobian, pushforward, refusal_line, smaller_singular, steer, Decoder, Inverse,
    Matrix, Refusal, Set, CUT, TOLERANCE,
};
use crate::explore::compass::views::{orthonormal, upper, CompassView, GridPlace};
use crate::explore::manifold_view::{dot, lock_edit, norm, point, View};
use crate::explore::{COMPASS_JACOBI, COMPASS_SHAPE};
use crate::headless::Headless;
use crate::layout::Layout;
use crate::side::MockSide;

/// A tilted, zoomed start plane.
fn start() -> Plane {
    plane(&view(20.0, -15.0, 10.0, 0.8), [0.1, -0.3, 0.25])
}

/// The app in the Jacobi view at `p` with the decoder `d`.
fn app_with(p: &Plane, d: Decoder, id: &'static str) -> (App<MockSide>, Headless) {
    let (mut app, mut h) = app_at(p);
    app.manifold.compass.decoder = d;
    click_scope(&mut h, &mut app, id);
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

/// Clicks cell `(i, j)` of the grid, then a frame for the snapshot to follow.
fn click_cell(h: &mut Headless, app: &mut App<MockSide>, i: usize, j: usize) {
    let at = GridPlace::fit(square(h)).cell_rect(i, j).center();
    for events in crate::capture::click(at) {
        let _ = h.frame(app, events);
    }
    let _ = h.frame(app, Vec::new());
}

/// Asserts `after`'s pushforward through `J` at its `z₀` lights `(i, j)` at ±1 and nothing else.
fn check_lit(d: &Decoder, set: Set, after: &Plane, (i, j): (usize, usize), within: f64) {
    let links = Links::defaults();
    let push = pushforward(&latent_jacobian(d, set, &links, &after.z0), after);
    for (a, b) in upper() {
        let want = if (a, b) == (i, j) { 1.0 } else { 0.0 };
        assert!(
            (push.grid[a][b].abs() - want).abs() <= within,
            "steered to ({i}, {j}): cell ({a}, {b}) {}",
            push.grid[a][b]
        );
    }
}

/// Asserts `after` is `before` with an orthogonal pair at the lengths it had, `z₀` kept.
fn check_apply_form(before: &Plane, after: &Plane) {
    assert_eq!(after.z0, before.z0, "z₀ kept");
    assert!(dot(&after.q1, &after.q2).abs() <= 1e-12, "Gram–Schmidt");
    assert!(
        (norm(&after.q1) - norm(&before.q1)).abs() <= 1e-12,
        "|q₁| kept"
    );
    assert!(
        (norm(&after.q2) - norm(&before.q2)).abs() <= 1e-12,
        "|q₂| kept"
    );
}

/// One Ctrl+Z, then a frame.
fn ctrl_z(h: &mut Headless, app: &mut App<MockSide>) {
    press(h, app, Key::Z, Modifiers::CTRL);
    let _ = h.frame(app, Vec::new());
}

#[test]
fn mock_compass_steer_physical_every_cell_on_the_fixture_lights() {
    let d = Decoder::Linear {
        a: linear_a(),
        shell: false,
    };
    let p = start();
    let (mut app, mut h) = app_with(&p, d, COMPASS_JACOBI);
    let before = app.snapshot().sim.plane.clone();
    for (i, j) in upper() {
        let (rows, depth) = (app.console().len(), app.snapshot().history.undo_depth);
        click_cell(&mut h, &mut app, i, j);
        let after = app.snapshot().sim.plane.clone();
        assert_eq!(
            setfields(&app, rows),
            ["basis"],
            "one SetField on the basis"
        );
        assert_eq!(app.snapshot().history.undo_depth, depth + 1);
        assert!(
            app.console()[rows..]
                .iter()
                .any(|e| e.message.starts_with("SetField Plane.q1 Plane.q2 ")),
            "the steer logged in the console"
        );
        check_lit(&d, Set::Jacobi, &after, (i, j), 1e-9);
        check_apply_form(&before, &after);
        // The Manifold view's sliders read the plane, as after Apply.
        assert_eq!(
            View::of(&after).scale,
            View::read(&app.snapshot().sim).scale
        );
        ctrl_z(&mut h, &mut app);
        assert_eq!(app.snapshot().sim.plane, before, "undone by one Ctrl+Z");
        assert_eq!(app.snapshot().history.undo_depth, depth);
    }
}

/// `sim` from the mock at the plane `p`.
fn sim_at(p: &Plane) -> SimConfig {
    let (app, _h) = app_at(p);
    app.snapshot().sim.clone()
}

/// The basis of a steer's edit.
fn basis_of(edit: Result<engine::contract::set_field::SetField, Refusal>) -> (Latent, Latent) {
    match edit.expect("accepted").edit {
        Edit::Sim(SimField::Basis { q1, q2 }) => (q1, q2),
        other => panic!("not a basis: {other:?}"),
    }
}

#[test]
fn mock_compass_steer_physical_is_the_preimage_orthonormalised() {
    let a = linear_a();
    let d = Decoder::Linear { a, shell: false };
    let sim = sim_at(&start());
    let links = Links::defaults();
    let j = latent_jacobian(&d, Set::Shape, &links, &sim.plane.z0);
    let inverse = Inverse::of(&j, CUT);
    assert_eq!(inverse.kept, 8, "nothing dropped: J⁺ = J⁻¹");
    // J J⁺ e_i = e_i.
    for i in 0..LATENT_DIM {
        let back = physical::apply(&j, &inverse.preimage(i));
        for (k, x) in back.iter().enumerate() {
            let want = if k == i { 1.0 } else { 0.0 };
            assert!((x - want).abs() <= 1e-9, "J J⁺ e_{i} at {k}: {x}");
        }
    }
    let (q1, q2) = basis_of(steer(&d, Set::Shape, &sim, 2, 6).expect("a cell"));
    let (e1, e2) = orthonormal(&Plane {
        z0: sim.plane.z0,
        q1: inverse.preimage(2),
        q2: inverse.preimage(6),
    })
    .expect("a plane");
    let (n1, n2) = (norm(&sim.plane.q1), norm(&sim.plane.q2));
    for k in 0..LATENT_DIM {
        assert!((q1[k] - n1 * e1[k]).abs() <= 1e-12, "q₁ ∝ J⁺ e_i");
        assert!((q2[k] - n2 * e2[k]).abs() <= 1e-12, "q₂ by Gram–Schmidt");
    }
    // The diagonal, below it and off the grid: no cell.
    for (i, j) in [(0, 0), (5, 2), (7, 7), (0, LATENT_DIM)] {
        assert_eq!(steer(&d, Set::Jacobi, &sim, i, j), None, "({i}, {j})");
    }
    assert!(steer(&d, Set::Jacobi, &sim, 6, 7).is_some());
    // Control: a steer that skips the Gram–Schmidt step is no orthogonal pair.
    let raw = Plane {
        z0: sim.plane.z0,
        q1: inverse
            .preimage(2)
            .map(|x| x * n1 / norm(&inverse.preimage(2))),
        q2: inverse
            .preimage(6)
            .map(|x| x * n2 / norm(&inverse.preimage(6))),
    };
    rejects("a steer without Gram–Schmidt", || {
        check_apply_form(&sim.plane, &raw)
    });
}

/// The rank-7 fixture standing for an `E*` chart: the shell's normal is `e_row`.
fn shell_fixture(row: usize) -> Decoder {
    Decoder::Linear {
        a: missing(linear_a(), row),
        shell: true,
    }
}

#[test]
fn mock_compass_steer_physical_refuses_the_shell_normal_and_logs_it() {
    let d = shell_fixture(4);
    let (mut app, mut h) = app_with(&start(), d, COMPASS_JACOBI);
    let before = app.snapshot().sim.plane.clone();
    let (rows, depth) = (app.console().len(), app.snapshot().history.undo_depth);
    let warnings = app.counts().warnings;
    for (i, j) in [(4, 6), (0, 4)] {
        click_cell(&mut h, &mut app, i, j);
        let line = refusal_line(Set::Jacobi, i, j, Refusal::EnergyShell);
        assert!(line.contains("normal to the energy shell"), "{line}");
        assert!(
            app.console()[rows..].iter().any(|e| e.message == line),
            "logged: {line}"
        );
    }
    assert_eq!(app.counts().warnings, warnings + 2);
    assert!(setfields(&app, rows).is_empty(), "no SetField");
    assert_eq!(app.snapshot().sim.plane, before);
    assert_eq!(app.snapshot().history.undo_depth, depth);
    // A cell clear of the normal is steered.
    click_cell(&mut h, &mut app, 0, 6);
    assert_eq!(setfields(&app, rows), ["basis"]);
    check_lit(
        &d,
        Set::Jacobi,
        &app.snapshot().sim.plane.clone(),
        (0, 6),
        1e-9,
    );
    // Control: a steer accepted on the refused case fails this test's check.
    let sim = app.snapshot().sim.clone();
    let refused = steer(&d, Set::Jacobi, &sim, 4, 6).expect("a cell");
    assert_eq!(refused, Err(Refusal::EnergyShell));
    let accepted = steer(&d, Set::Jacobi, &sim, 0, 6).expect("a cell");
    rejects("a steer accepted on a refused case", || {
        assert_eq!(accepted, Err(Refusal::EnergyShell))
    });
}

#[test]
fn mock_compass_steer_physical_the_test_is_on_the_projector() {
    // RQ-344's (a2): scale-free. Scaling J moves [J⁺e_i, J⁺e_j]'s singular value, never [P e_i, P e_j]'s.
    let a = linear_a();
    let big: Matrix = a.map(|row| row.map(|x| x * 1e6));
    let (small_inverse, big_inverse) = (Inverse::of(a, CUT), Inverse::of(&big, CUT));
    let (a2_small, a2_big) = (small_inverse.smaller(1, 5), big_inverse.smaller(1, 5));
    assert!((a2_small - 1.0).abs() <= 1e-9, "P = I: {a2_small}");
    assert!((a2_big - 1.0).abs() <= 1e-9, "P = I: {a2_big}");
    assert!(a2_small >= TOLERANCE);
    // Control: the (a1) test, which scales with J, would refuse the same full-rank cell at this scale.
    let a1 = smaller_singular(&big_inverse.preimage(1), &big_inverse.preimage(5));
    rejects("a refusal test on [J⁺e_i, J⁺e_j]", || {
        assert!(a1 >= TOLERANCE, "(a1) {a1}")
    });
}

/// The boundary of the tolerance: a rank-7 fixture whose normal `n` leaves `[P e_0, P e_1]` a smaller singular value `σ`.
fn tilted_normal(sigma: f64) -> Decoder {
    // n = (√(1−σ²), 0, σ, 0, …): the projector's value on (e₀, e₁) is exactly σ.
    let c = (1.0 - sigma * sigma).sqrt();
    let mut m = [[0.0; LATENT_DIM]; LATENT_DIM];
    // x = Σ_k z_k b_k, b_k an orthonormal basis of n's complement as columns: t in the plane of n, the other axes.
    m[0][0] = sigma;
    m[2][0] = -c;
    for (k, r) in [1, 3, 4, 5, 6, 7].into_iter().enumerate() {
        m[r][k + 1] = 1.0;
    }
    Decoder::Linear {
        a: leaked(m),
        shell: true,
    }
}

#[test]
fn mock_compass_steer_physical_refuses_below_the_tolerance_only() {
    let sim = sim_at(&Plane {
        z0: [0.0; LATENT_DIM],
        ..start()
    });
    let links = Links::defaults();
    // Each side of the tolerance. The ranges rescale the rows, moving σ from the fixture's, so the check reads the value.
    let ratio = |d: &Decoder| {
        Inverse::of(&latent_jacobian(d, Set::Jacobi, &links, &sim.plane.z0), CUT).smaller(0, 1)
    };
    // The refusal flips exactly where the value crosses the tolerance.
    let mut seen = (false, false);
    for k in 1..60 {
        let s = f64::from(k) / 60.0;
        let d = tilted_normal(s);
        let value = ratio(&d);
        let verdict = steer(&d, Set::Jacobi, &sim, 0, 1).expect("a cell");
        if value < TOLERANCE {
            assert_eq!(verdict, Err(Refusal::EnergyShell), "σ = {value}");
            seen.0 = true;
        } else {
            assert!(verdict.is_ok(), "σ = {value}");
            seen.1 = true;
        }
    }
    assert_eq!(seen, (true, true), "both sides of the tolerance");
    assert_eq!(TOLERANCE, 0.1, "REQ-GUI-346's proposal");
    assert_eq!(CUT, 1e-9, "REQ-GUI-410's start");
    // The test's value is the light the steer can reach: the steered plane's cell is that smaller singular value.
    let d = tilted_normal(0.5);
    let j = latent_jacobian(&d, Set::Jacobi, &links, &sim.plane.z0);
    let value = Inverse::of(&j, CUT).smaller(0, 1);
    let (q1, q2) = basis_of(steer(&d, Set::Jacobi, &sim, 0, 1).expect("a cell"));
    let push = pushforward(
        &j,
        &Plane {
            z0: sim.plane.z0,
            q1,
            q2,
        },
    );
    assert!(
        (push.grid[0][1].abs() - value).abs() <= 1e-9,
        "the cell lit at {} for σ = {value}",
        push.grid[0][1]
    );
    rejects("a plane lit at ±1 out of J's range", || {
        assert!((push.grid[0][1].abs() - 1.0).abs() <= 1e-9)
    });
}

/// The latent point of the Shape sphere's `+w` pole, `α = π/4`, `β = π/2`, at equal-ish masses and moving.
fn at_pole() -> Latent {
    [0.0, 0.0, 0.3, -0.2, 0.5, 0.1, 0.0, 0.0]
}

/// Whether the frame's readout carries the pole's line, the pointer `off` the figure's centre.
fn pole_line_shown(p: &Plane, off: eframe::egui::Vec2) -> (bool, Latent) {
    let (mut app, mut h) = app_with(p, Decoder::default(), COMPASS_SHAPE);
    let figure = Layout::new(h.screen(), crate::capture::PIXELS_PER_POINT).figure;
    let moved = Event::PointerMoved(figure.center() + off);
    let _ = h.frame(&mut app, vec![moved]);
    let _ = h.frame(&mut app, Vec::new());
    let hover = app.manifold.compass.hover.expect("over the figure");
    let texts = super::support::frame_texts(&mut h, &mut app);
    let shown = texts.contains(&physical::POLE_LINE.to_owned());
    (shown, physical::latent_point(p, Some(hover)))
}

#[test]
fn mock_compass_steer_physical_the_pole_line_reads_the_hovered_point() {
    let (d, links) = (Decoder::default(), Links::defaults());
    let is_pole = |z: &Latent| physical::at_pole(&d, Set::Shape, &links, z);
    let off = eframe::egui::vec2(90.0, -70.0);
    // z₀ at the pole, the pixel under the pointer off it: the grid's point, so no line.
    let mut p = start();
    p.z0 = at_pole();
    let (shown, hovered) = pole_line_shown(&p, off);
    assert!(is_pole(&p.z0) && !is_pole(&hovered), "at {hovered:?}");
    assert!(!shown, "z₀'s pole under a grid taken off it");
    // z₀ off the pole, the pixel under the pointer on it: the line.
    let mut q = start();
    q.z0 = std::array::from_fn(|k| at_pole()[k] - (hovered[k] - p.z0[k]));
    let (shown, hovered) = pole_line_shown(&q, off);
    assert!(!is_pole(&q.z0) && is_pole(&hovered), "z₀ {:?}", q.z0);
    assert!(shown, "the hovered pole named");
    // Control: the line read at z₀ would say the reverse.
    rejects("the line read at z₀", || {
        assert_eq!(is_pole(&q.z0), shown)
    });
}

#[test]
fn mock_compass_steer_physical_at_the_shape_pole_names_it() {
    let mut p = start();
    p.z0 = at_pole();
    let d = Decoder::default();
    let (mut app, mut h) = app_with(&p, d, COMPASS_SHAPE);
    assert_eq!(app.manifold.compass.view, CompassView::Shape);
    let (rows, depth) = (app.console().len(), app.snapshot().history.undo_depth);
    click_cell(&mut h, &mut app, 4, 6);
    let line = refusal_line(Set::Shape, 4, 6, Refusal::Pole);
    assert!(line.contains("the chart's pole"), "{line}");
    assert!(
        app.console()[rows..].iter().any(|e| e.message == line),
        "logged: {:?}",
        &app.console()[rows..]
    );
    assert!(setfields(&app, rows).is_empty());
    assert_eq!(app.snapshot().history.undo_depth, depth);
    // The readout names the pole as the chart's singularity.
    let texts = super::support::frame_texts(&mut h, &mut app);
    assert!(texts.contains(&physical::POLE_LINE.to_owned()), "named");
    // Off the pole the same cell is steered, in the same view.
    let sim = app.snapshot().sim.clone();
    let mut off = sim.clone();
    off.plane.z0[0] = 0.8;
    off.plane.z0[1] = -1.1;
    assert!(steer(&d, Set::Shape, &off, 4, 6).expect("a cell").is_ok());
    // The Jacobi view has no pole there.
    assert!(steer(&d, Set::Jacobi, &sim, 4, 6).expect("a cell").is_ok());
    // A pole on an E* chart names the pole first.
    let shell = Decoder::Real { e_star: Some(0.0) };
    assert_eq!(
        steer(&shell, Set::Shape, &sim, 4, 6),
        Some(Err(Refusal::Pole))
    );
    rejects("a refusal naming the shell at the pole", || {
        assert_eq!(
            steer(&d, Set::Shape, &sim, 4, 6),
            Some(Err(Refusal::EnergyShell))
        )
    });
}

#[test]
fn mock_compass_steer_physical_the_reasons_are_r_550_j6s() {
    assert_eq!(Refusal::EnergyShell.reason(), "normal to the energy shell");
    assert_eq!(Refusal::Pole.reason(), "the chart's pole");
    assert_eq!(Refusal::Saturated.reason(), "a saturated link");
    assert_eq!(
        refusal_line(Set::Jacobi, 0, 2, Refusal::Saturated),
        "steer to α × p_ρx refused: a saturated link"
    );
    assert_eq!(
        refusal_line(Set::Shape, 4, 5, Refusal::EnergyShell),
        "steer to L_z × dR/dt refused: normal to the energy shell"
    );
    rejects("a line that names no reason", || {
        assert!(refusal_line(Set::Shape, 4, 5, Refusal::Pole).contains("saturated"))
    });
}

#[test]
fn mock_compass_steer_physical_locked_turns_about_the_pin() {
    let d = Decoder::Linear {
        a: linear_a(),
        shell: false,
    };
    let (mut app, mut h) = app_with(&start(), d, COMPASS_JACOBI);
    let p = app.snapshot().sim.plane.clone();
    let anchor = point(&p, 0.3, 0.7);
    app.set_field(lock_edit(Lock {
        locked: true,
        z_locked: anchor,
    }));
    let _ = h.frame(&mut app, Vec::new());
    let depth = app.snapshot().history.undo_depth;
    click_cell(&mut h, &mut app, 1, 3);
    let after = app.snapshot().sim.plane.clone();
    let at = point(&after, 0.3, 0.7);
    for k in 0..LATENT_DIM {
        assert!((at[k] - anchor[k]).abs() <= 1e-12, "the pin's point moved");
    }
    assert_eq!(app.snapshot().history.undo_depth, depth + 1, "one entry");
    ctrl_z(&mut h, &mut app);
    assert_eq!(app.snapshot().sim.plane, p);
    rejects("a pin that moved", || {
        let moved = point(&after, 0.2, 0.7);
        for k in 0..LATENT_DIM {
            assert!((moved[k] - anchor[k]).abs() <= 1e-12);
        }
    });
}

#[test]
fn mock_compass_steer_physical_from_the_keyboard() {
    let d = Decoder::Linear {
        a: linear_a(),
        shell: false,
    };
    let (app, _h) = app_with(&start(), d, COMPASS_JACOBI);
    let sim = app.snapshot().sim.clone();
    let mut scratch = crate::explore::manifold_view::Scratch::default();
    scratch.compass.view = CompassView::Jacobi;
    scratch.compass.decoder = d;
    scratch.compass.cursor = Some((1, 2));
    let mut out = crate::explore::manifold_view::Out::default();
    let mut focus = app.view.focus.clone();
    // Navigating, Space does nothing; in interaction mode on the compass it steers to the cursor.
    crate::explore::compass::space(&focus, &sim, &mut scratch, &mut out);
    assert!(out.edits.is_empty(), "navigating");
    focus.path = vec![
        crate::explore::COMPASS.to_owned(),
        crate::explore::COMPASS_VIEW.to_owned(),
    ];
    focus.mode = engine::contract::view_ui::KeyMode::Interaction;
    crate::explore::compass::space(&focus, &sim, &mut scratch, &mut out);
    assert_eq!(out.edits.len(), 1, "one SetField");
    assert_eq!(
        Ok(out.edits[0].clone()),
        steer(&d, Set::Jacobi, &sim, 1, 2).expect("a cell")
    );
    // No cursor, or the Cube view: nothing more.
    scratch.compass.cursor = None;
    crate::explore::compass::space(&focus, &sim, &mut scratch, &mut out);
    assert_eq!(out.edits.len(), 1);
    scratch.compass.cursor = Some((1, 2));
    scratch.compass.view = CompassView::Cube;
    crate::explore::compass::space(&focus, &sim, &mut scratch, &mut out);
    assert_eq!(out.edits.len(), 1);
    // A refused cell logs, by Space too.
    scratch.compass.view = CompassView::Jacobi;
    scratch.compass.decoder = shell_fixture(1);
    crate::explore::compass::space(&focus, &sim, &mut scratch, &mut out);
    assert_eq!(out.edits.len(), 1);
    assert_eq!(
        out.refusals,
        [refusal_line(Set::Jacobi, 1, 2, Refusal::EnergyShell)]
    );
    // A degenerate view draws no cell and still steers, its cells the way out of it.
    let mut degenerate = sim.clone();
    degenerate.plane.q2 = degenerate.plane.q1;
    scratch.compass.decoder = d;
    crate::explore::compass::space(&focus, &degenerate, &mut scratch, &mut out);
    assert_eq!(out.edits.len(), 2);
    // Through the app: Enter into interaction puts the cursor on the dominant cell, the arrows move it.
    let (mut app2, mut h2) = app_with(&start(), d, COMPASS_JACOBI);
    app2.view.focus.path = focus.path.clone();
    app2.view.focus.mode = engine::contract::view_ui::KeyMode::Interaction;
    let _ = h2.frame(&mut app2, Vec::new());
    let cursor = app2.manifold.compass.cursor.expect("the cursor");
    let grid =
        crate::explore::compass::pushed(&app2.snapshot().sim, &app2.manifold.compass, Set::Jacobi)
            .grid;
    assert_eq!(
        Some(cursor),
        crate::explore::compass::views::dominant(&grid)
    );
    app2.manifold.compass.cursor = Some((0, 1));
    let rows = app2.console().len();
    press(&mut h2, &mut app2, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(
        app2.manifold.compass.cursor,
        Some((0, 2)),
        "→ along the row"
    );
    assert!(setfields(&app2, rows).is_empty(), "the arrows edit nothing");
    let moved = app2.manifold.compass.cursor;
    rejects("a cursor that does not move", || {
        assert_eq!(moved, Some((0, 1)))
    });
}
