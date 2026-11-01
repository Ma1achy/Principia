//! `mock_cube_axis_buttons` (REQ-GUI-208; render_gui_spec § "The compass: look and views (R-451, R-452)", §G3; R-459 G1 to
//! G4, G9): the cube's H, V and D buttons sit on R-451 B1's edges in both modes, each a dropdown of the dimensions not on
//! the other two; a pick is one undoable entry, H and V in step with Chart's rows, D the slice frame's normal, a target
//! on it following and the plane re-tilting; a plane that takes D moves it by the slice-axis rule; the keys reach them.
//! Each check also runs on an input it must reject (R-176, through `rejects`).

use eframe::egui::{Key, Modifiers};
use engine::contract::sim_config::{Latent, Lock, Plane, SliceFrame};
use engine::contract::view_ui::{KeyMode, Mode};

use super::compass_support::{app_at, check_painted, compass_shapes, drawing, plane, square, view};
use super::support::{click_scope, focus, frame_texts, press, rejects};
use crate::app::App;
use crate::explore::compass::views::{CompassView, GridPlace};
use crate::explore::compass::{
    self, choices, edge_label, pick_edit, Edge, Iso, Shows, DEPTH_EDGE, HEIGHT_EDGE, WIDTH_EDGE,
};
use crate::explore::manifold_view::chart::basis_text;
use crate::explore::manifold_view::{
    axis, frame_edit, lock_edit, normal_edit, plane_edit, slice_axis, Touch, View,
};
use crate::explore::{
    COMPASS, COMPASS_BADGE, COMPASS_CUBE, COMPASS_D, COMPASS_D_ENTRIES, COMPASS_H,
    COMPASS_H_ENTRIES, COMPASS_JACOBI, COMPASS_PLUCKER, COMPASS_SHAPE, COMPASS_SLICE, COMPASS_TILT,
    COMPASS_V, COMPASS_VIEW, COMPASS_V_ENTRIES,
};
use crate::headless::Headless;
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;
const Z_ALPHA: usize = 0;
const Z_BETA: usize = 1;
const Z_Q0: usize = 2;
const Z_Q1: usize = 3;
const Z_Q2: usize = 4;
const Z_Q3: usize = 5;
const Z_MU1: usize = 6;

/// The slice frame `(normal, X, Y)`.
fn frame(normal: usize, x: usize, y: usize) -> SliceFrame {
    let index = |i: usize| u32::try_from(i).expect("a latent index");
    SliceFrame {
        normal: index(normal),
        tilt_x: index(x),
        tilt_y: index(y),
    }
}

/// The app at rest on the mock's preset at `(τ₁, τ₂, γ)` degrees, scale `σ`, a frame of the normal and targets given.
fn app_with(
    (tau1, tau2, gamma): (f64, f64, f64),
    sigma: f64,
    f: SliceFrame,
) -> (App<MockSide>, Headless) {
    let v = View {
        d: f.normal as usize,
        x: f.tilt_x as usize,
        y: f.tilt_y as usize,
        ..view(tau1, tau2, gamma, sigma)
    };
    let (mut app, mut h) = app_at(&plane(&v, [0.1, -0.2, 0.2]));
    app.set_field(frame_edit(f, None));
    settle(&mut h, &mut app);
    (app, h)
}

/// Frames for the snapshot and the compass to follow an edit.
fn settle(h: &mut Headless, app: &mut App<MockSide>) {
    let _ = h.frame(app, Vec::new());
    super::compass_support::settle(h, app);
}

/// Ctrl+Z, the contract's undo, then frames to follow it.
fn ctrl_z(h: &mut Headless, app: &mut App<MockSide>) {
    press(h, app, Key::Z, Modifiers::CTRL);
    settle(h, app);
}

/// The state a pick edits, and the undo depth.
fn state(app: &App<MockSide>) -> (u32, Plane, SliceFrame) {
    let s = app.snapshot();
    (s.history.undo_depth, s.sim.plane.clone(), s.sim.slice.frame)
}

/// Asserts the pick from `before` to `after` was one history entry, and `undone` the state before it.
fn check_one_entry(
    before: &(u32, Plane, SliceFrame),
    after: &(u32, Plane, SliceFrame),
    undone: &(u32, Plane, SliceFrame),
) {
    assert_eq!(after.0, before.0 + 1, "one history entry");
    assert_eq!(undone, before, "one Ctrl+Z undoes all of it");
}

/// The buttons' texts, H, third, V.
fn texts(h: &Headless, app: &App<MockSide>) -> [String; 3] {
    let d = drawing(h, app);
    assert_eq!(d.labels.len(), 3, "three labels: {:?}", d.labels);
    [
        d.labels[0].text.clone(),
        d.labels[1].text.clone(),
        d.labels[2].text.clone(),
    ]
}

/// Asserts the buttons read `want`, each centred on its edge's place, the scope placed there.
fn check_on_edges(h: &Headless, app: &App<MockSide>, want: [&str; 3]) {
    let iso = Iso::fit(square(h));
    assert_eq!(texts(h, app), want.map(str::to_owned));
    for (id, edge, text) in [
        (COMPASS_H, WIDTH_EDGE, want[0]),
        (COMPASS_D, DEPTH_EDGE, want[1]),
        (COMPASS_V, HEIGHT_EDGE, want[2]),
    ] {
        let at = edge_label(&iso, edge, text.to_owned(), false).at;
        let placed = app.keyboard.place_of(id).expect("placed").rect;
        assert!(
            (placed.center() - at).length() < 1e-3,
            "{id} at {:?}, its edge's label at {at:?}",
            placed.center()
        );
        assert!(
            placed.width() > 10.0 && placed.height() > 10.0,
            "{id} big enough to press: {placed:?}"
        );
    }
}

#[test]
fn mock_cube_axis_buttons_sit_on_their_edges_in_both_modes() {
    let (mut app, mut h) = app_at(&plane(&view(0.0, 0.0, 0.0, 1.0), [0.0, 0.0, 0.0]));
    let want = ["H z_α", "D z_q3", "V z_β"];
    check_on_edges(&h, &app, want);
    let shapes = compass_shapes(&mut h, &mut app);
    check_painted(&shapes, &drawing(&h, &app));
    assert!(drawing(&h, &app).numbers.is_empty(), "tilt has no numbers");
    // Each button is its text and the padding round it, the text painted over it.
    let painted = super::compass_support::texts(&shapes);
    for (id, text) in [
        (COMPASS_H, want[0]),
        (COMPASS_D, want[1]),
        (COMPASS_V, want[2]),
    ] {
        let drawn = painted
            .iter()
            .find(|t| t.0 == text)
            .unwrap_or_else(|| panic!("{text} painted"));
        let placed = app.keyboard.place_of(id).expect("placed").rect;
        let pad = 2.0 * compass::AXIS_BUTTON_PAD;
        assert!(
            (placed.width() - (drawn.1.width() + pad)).abs() < 1.0,
            "{id}'s width: {placed:?}"
        );
        assert!(
            (placed.height() - (compass::LABEL_SIZE + pad)).abs() < 0.5,
            "{id}'s height: {placed:?}"
        );
    }
    app.manifold.compass.mode = Touch::Slice;
    settle(&mut h, &mut app);
    check_on_edges(&h, &app, want);
    assert!(!drawing(&h, &app).numbers.is_empty(), "slice keeps them");
    let shapes = compass_shapes(&mut h, &mut app);
    check_painted(&shapes, &drawing(&h, &app));
    rejects("H and V on each other's edges", || {
        check_on_edges(&h, &app, ["V z_β", "D z_q3", "H z_α"])
    });
    rejects("a button without its letter", || {
        check_on_edges(&h, &app, ["z_α", "D z_q3", "V z_β"])
    });
}

/// Asserts each list holds the dimensions not on the other two, in order.
fn check_lists(v: &View, [h_list, v_list, d_list]: [&[usize]; 3]) {
    let all = |skip: [usize; 2]| -> Vec<usize> { (0..8).filter(|i| !skip.contains(i)).collect() };
    assert_eq!(h_list, all([v.b, v.d]), "H's list");
    assert_eq!(v_list, all([v.a, v.d]), "V's list");
    assert_eq!(d_list, all([v.a, v.b]), "D's list");
}

#[test]
fn mock_cube_axis_buttons_each_dropdown_lists_the_dimensions_not_on_the_other_two() {
    let (app, _h) = app_with((0.0, 0.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_Q3));
    let read = View::read(&app.snapshot().sim);
    let (h_list, v_list, d_list) = (
        choices(&read, Edge::H),
        choices(&read, Edge::V),
        choices(&read, Edge::D),
    );
    check_lists(&read, [&h_list, &v_list, &d_list]);
    // The keyboard's menus hold the same entries.
    let tree = app.keyboard.tree(Mode::Explore);
    for (id, entries, list) in [
        (COMPASS_H, COMPASS_H_ENTRIES, &h_list),
        (COMPASS_V, COMPASS_V_ENTRIES, &v_list),
        (COMPASS_D, COMPASS_D_ENTRIES, &d_list),
    ] {
        let want: Vec<&str> = list.iter().map(|&i| entries[i]).collect();
        assert_eq!(tree.children(Some(id)), want, "{id}'s entries");
    }
    // A dimension not offered is no pick; nor is the one it already is.
    let sim = &app.snapshot().sim;
    assert_eq!(pick_edit(sim, Edge::H, Z_BETA, Shows::Normal), None);
    assert_eq!(pick_edit(sim, Edge::H, Z_Q3, Shows::Normal), None);
    assert_eq!(pick_edit(sim, Edge::H, Z_ALPHA, Shows::Normal), None);
    assert_eq!(pick_edit(sim, Edge::V, Z_ALPHA, Shows::Normal), None);
    assert_eq!(pick_edit(sim, Edge::V, Z_Q3, Shows::Normal), None);
    assert_eq!(pick_edit(sim, Edge::V, Z_BETA, Shows::Normal), None);
    assert_eq!(pick_edit(sim, Edge::D, Z_BETA, Shows::Normal), None);
    assert_eq!(pick_edit(sim, Edge::D, Z_Q3, Shows::Normal), None);
    let mut with_v = h_list.clone();
    with_v.insert(1, read.b);
    rejects("a dropdown that offers V's dimension in H's list", || {
        check_lists(&read, [&with_v, &v_list, &d_list])
    });
}

/// Asserts the H pick `to` from `before` to `after`: `q₁ = σ e_to`, `q₂ = σ e_b`, so the tilts and `γ` are 0, `z₀` kept.
fn check_h_picked(before: &Plane, after: &Plane, to: usize, b: usize, sigma: f64) {
    assert_eq!(after.q1, axis(to, sigma), "q₁ ∝ e_to");
    assert_eq!(after.q2, axis(b, sigma), "q₂ on V's");
    assert_eq!(after.z0, before.z0, "z₀ kept");
}

#[test]
fn mock_cube_axis_buttons_picking_h_sets_the_pair_chart_following_and_one_undo() {
    let (mut app, mut h) = app_with((20.0, 10.0, 5.0), 0.5, frame(Z_Q3, Z_Q3, Z_Q3));
    let before = state(&app);
    click_scope(&mut h, &mut app, COMPASS_H);
    let _ = h.frame(&mut app, Vec::new());
    click_scope(&mut h, &mut app, COMPASS_H_ENTRIES[Z_Q0]);
    settle(&mut h, &mut app);
    let after = state(&app);
    check_h_picked(&before.1, &after.1, Z_Q0, Z_BETA, 0.5);
    let read = View::read(&app.snapshot().sim);
    assert_eq!(
        (read.a, read.b, read.tau1, read.tau2, read.gamma),
        (Z_Q0, Z_BETA, 0.0, 0.0, 0.0),
        "the new pair, every angle 0"
    );
    assert_eq!(read.zoom(), 1.0, "the zoom kept");
    assert_eq!(texts(&h, &app)[0], "H z_q0");
    // Chart's horizontal q₁ reads it.
    assert_eq!(basis_text(&after.1.q1), "0.50·z_q0");
    assert!(
        frame_texts(&mut h, &mut app)
            .iter()
            .any(|t| t == "0.50·z_q0"),
        "Chart's row"
    );
    ctrl_z(&mut h, &mut app);
    check_one_entry(&before, &after, &state(&app));
    rejects("a pick that keeps the tilt", || {
        check_h_picked(&before.1, &before.1, Z_Q0, Z_BETA, 0.5)
    });
}

#[test]
fn mock_cube_axis_buttons_picking_v_and_a_chart_edit_move_v() {
    let (mut app, mut h) = app_with((0.0, 0.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_Q3));
    let before = state(&app);
    let edit = pick_edit(&app.snapshot().sim, Edge::V, Z_Q2, Shows::Normal).expect("offered");
    app.set_field(edit);
    settle(&mut h, &mut app);
    assert_eq!(app.snapshot().sim.plane.q2, axis(Z_Q2, 1.0));
    assert_eq!(app.snapshot().sim.plane.q1, axis(Z_ALPHA, 1.0));
    assert_eq!(texts(&h, &app)[2], "V z_q2");
    ctrl_z(&mut h, &mut app);
    assert_eq!(state(&app), before);
    // Chart: editing q₂ to e_q1 moves V to z_q1, as H and V are the axes the view reads.
    let sim = app.snapshot().sim.clone();
    app.set_field(plane_edit(&sim, (sim.plane.q1, axis(Z_Q1, 1.0))));
    settle(&mut h, &mut app);
    assert_eq!(texts(&h, &app)[2], "V z_q1");
    assert_eq!(texts(&h, &app)[0], "H z_α");
    rejects("V left behind by a Chart edit", || {
        assert_eq!(texts(&h, &app)[2], "V z_β")
    });
}

#[test]
fn mock_cube_axis_buttons_the_pair_the_other_way_round_reads_as_picked() {
    // H and V swapped by two picks: no turn of the preset whose axes they are, so it reads as picked, every angle 0.
    let (mut app, mut h) = app_with((0.0, 0.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_Q3));
    for (edge, to) in [(Edge::H, Z_Q0), (Edge::V, Z_ALPHA), (Edge::H, Z_BETA)] {
        let edit = pick_edit(&app.snapshot().sim, edge, to, Shows::Normal).expect("offered");
        app.set_field(edit);
        settle(&mut h, &mut app);
    }
    let read = View::read(&app.snapshot().sim);
    assert_eq!((read.a, read.b, read.gamma), (Z_BETA, Z_ALPHA, 0.0));
    assert_eq!(texts(&h, &app)[0], "H z_β");
    assert_eq!(texts(&h, &app)[2], "V z_α");
    // The preset turned by 90°, a real turn, still reads as the preset.
    let turned = Plane {
        z0: [0.0; 8],
        q1: axis(Z_BETA, 1.0),
        q2: axis(Z_ALPHA, -1.0),
    };
    let read = View::framed(&turned, Some(&frame(Z_Q3, Z_Q3, Z_Q3)));
    assert_eq!((read.a, read.b, read.preset), (Z_ALPHA, Z_BETA, Some(0)));
    assert!((read.gamma.to_degrees() - 90.0).abs() < 1e-9);
    // And tilted, the swapped pair keeps its axes, past 45° too.
    let tilted = |tau: f64| {
        let (s, c) = tau.to_radians().sin_cos();
        let mut q1 = axis(Z_BETA, c);
        q1[Z_Q3] = s;
        Plane {
            z0: [0.0; 8],
            q1,
            q2: axis(Z_ALPHA, 1.0),
        }
    };
    for tau in [20.0, 60.0, 89.0] {
        let read = View::framed(&tilted(tau), Some(&frame(Z_Q3, Z_Q3, Z_Q3)));
        assert_eq!((read.a, read.b), (Z_BETA, Z_ALPHA), "τ₁ = {tau}");
        assert!((read.tau1.to_degrees() - tau).abs() < 1e-9, "τ₁ = {tau}");
    }
}

#[test]
fn mock_cube_axis_buttons_a_picked_pair_keeps_its_axes_past_45_degrees() {
    let f = frame(Z_Q3, Z_Q3, Z_MU1);
    let at = |t1: f64, t2: f64| View {
        preset: None,
        a: Z_Q0,
        b: Z_Q2,
        d: Z_Q3,
        x: Z_Q3,
        y: Z_MU1,
        ..view(t1, t2, 0.0, 1.0)
    };
    for (t1, t2) in [
        (0.0, 0.0),
        (44.0, 10.0),
        (60.0, 70.0),
        (89.0, 89.0),
        (-70.0, 50.0),
    ] {
        let v = at(t1, t2);
        let (q1, q2) = v.basis();
        let read = View::framed(
            &Plane {
                z0: [0.0; 8],
                q1,
                q2,
            },
            Some(&f),
        );
        assert_eq!((read.a, read.b), (Z_Q0, Z_Q2), "({t1}, {t2})");
        assert!(
            (read.tau1.to_degrees() - t1).abs() < 1e-9,
            "τ₁ ({t1}, {t2})"
        );
        assert!(
            (read.tau2.to_degrees() - t2).abs() < 1e-9,
            "τ₂ ({t1}, {t2})"
        );
    }
    // A basis leaving the pair and the targets is read by its dominant axes, as before.
    let mut q1: Latent = axis(Z_Q1, 0.8);
    q1[Z_Q2] = 0.2;
    q1[Z_Q0] = 0.1;
    let read = View::framed(
        &Plane {
            z0: [0.0; 8],
            q1,
            q2: axis(Z_MU1, 1.0),
        },
        Some(&f),
    );
    assert_eq!((read.a, read.b), (Z_Q1, Z_MU1), "the dominant axes");
    // A vector all on the targets has nothing off them: the dominant axes too.
    let read = View::framed(
        &Plane {
            z0: [0.0; 8],
            q1: {
                let mut q = axis(Z_Q3, 0.6);
                q[Z_MU1] = 0.8;
                q
            },
            q2: {
                let mut q = axis(Z_Q0, 0.6);
                q[Z_Q2] = 0.8;
                q
            },
        },
        Some(&f),
    );
    assert_eq!((read.a, read.b), (Z_MU1, Z_Q2), "nothing off the targets");
    // And one whose q₂ is all on q₁'s axis and the targets.
    let read = View::framed(
        &Plane {
            z0: [0.0; 8],
            q1: {
                let mut q = axis(Z_Q0, 0.6);
                q[Z_Q3] = 0.8;
                q
            },
            q2: {
                let mut q = axis(Z_Q0, 0.8);
                q[Z_MU1] = 0.6;
                q
            },
        },
        Some(&f),
    );
    assert_eq!(
        read.a, Z_Q0,
        "q₂ has nothing else off the targets: {read:?}"
    );
}

/// Asserts the view on the axes `(a, b)` at these angles, in `f`, reads back on `(a, b)` with its own angles.
fn check_reads_back((a, b): (usize, usize), f: SliceFrame, (t1, t2, g): (f64, f64, f64)) {
    let v = View {
        preset: None,
        a,
        b,
        d: usize::try_from(f.normal).expect("a dimension"),
        x: usize::try_from(f.tilt_x).expect("a dimension"),
        y: usize::try_from(f.tilt_y).expect("a dimension"),
        ..view(t1, t2, g, 1.0)
    };
    let (q1, q2) = v.basis();
    let read = View::framed(
        &Plane {
            z0: [0.0; 8],
            q1,
            q2,
        },
        Some(&f),
    );
    let at = format!("({t1}, {t2}, {g})");
    assert_eq!((read.a, read.b), (a, b), "{at}");
    for (got, want, what) in [
        (read.tau1, t1, "τ₁"),
        (read.tau2, t2, "τ₂"),
        (read.gamma, g, "γ"),
    ] {
        assert!((got.to_degrees() - want).abs() < 1e-9, "{what} {at}");
    }
}

/// R-459 G3: H and V move only with a pick or an edit in Chart, so no angle in the range reads another pair. At ±90° the
/// kept axis is only `cos 90°`, and at a rotation past 45° it is not the largest off the targets.
#[test]
fn mock_cube_axis_buttons_a_pair_keeps_its_axes_at_the_ends_of_the_range_and_under_a_turn() {
    let f = frame(Z_Q3, Z_Q3, Z_MU1);
    let angles = [
        (90.0, 0.0, 0.0),
        (-90.0, 0.0, 0.0),
        (0.0, 90.0, 0.0),
        (90.0, 90.0, 0.0),
        (-90.0, 45.0, 0.0),
        (90.0, -90.0, 30.0),
        (90.0, 0.0, 30.0),
        (30.0, 20.0, 60.0),
        (0.0, 0.0, 60.0),
        (0.0, 0.0, -120.0),
    ];
    for at in angles {
        check_reads_back((Z_Q0, Z_BETA), f, at);
        // A preset's axes, the other way round, are no turn of that preset (R-459 G3): the determinant says so at the
        // ends too, where it is only `cos² 90°`.
        check_reads_back((Z_BETA, Z_ALPHA), f, at);
        // And a preset stays that preset.
        check_reads_back((Z_ALPHA, Z_BETA), f, at);
    }
    // At the smallest zoom's scale too.
    let small = View {
        preset: None,
        a: Z_Q0,
        b: Z_BETA,
        d: Z_Q3,
        x: Z_Q3,
        y: Z_MU1,
        ..view(90.0, 90.0, 30.0, (-48.0_f64).exp2())
    };
    let (q1, q2) = small.basis();
    let read = View::framed(
        &Plane {
            z0: [0.0; 8],
            q1,
            q2,
        },
        Some(&f),
    );
    assert_eq!((read.a, read.b), (Z_Q0, Z_BETA), "zoomed in");
    rejects("a pair read as another's axes", || {
        check_reads_back((Z_Q0, Z_BETA), frame(Z_Q3, Z_Q3, Z_Q0), (90.0, 0.0, 0.0))
    });
}

/// The basis with `q₁` and `q₂` of the components given, each `(axis, value)`, in the frame `(z_q3, z_q3, z_μ1)`.
fn read_of(q1: &[(usize, f64)], q2: &[(usize, f64)]) -> View {
    let vector = |parts: &[(usize, f64)]| -> Latent {
        let mut v = [0.0; 8];
        for &(i, x) in parts {
            v[i] = x;
        }
        v
    };
    View::framed(
        &Plane {
            z0: [0.0; 8],
            q1: vector(q1),
            q2: vector(q2),
        },
        Some(&frame(Z_Q3, Z_Q3, Z_MU1)),
    )
}

#[test]
fn mock_cube_axis_buttons_the_frame_reading_cuts_at_their_boundaries() {
    let (s, c) = 60.0_f64.to_radians().sin_cos();
    // No component is rounding: the smallest off the pair and the targets leaves them, to its dominant axes, the first
    // on the target. Its square would read 0, so the test is on the component.
    let least = f64::MIN_POSITIVE;
    let kept = read_of(&[(Z_Q0, c), (Z_Q3, s)], &[(Z_Q2, 1.0)]);
    assert_eq!(
        (kept.a, kept.b),
        (Z_Q0, Z_Q2),
        "on the pair and the targets"
    );
    let left = read_of(&[(Z_Q0, c), (Z_Q3, s), (Z_Q1, least)], &[(Z_Q2, 1.0)]);
    assert_eq!((left.a, left.b), (Z_Q3, Z_Q2), "off them, by the least");
    let left = read_of(&[(Z_Q0, c), (Z_Q3, s)], &[(Z_Q2, 1.0), (Z_Q1, least)]);
    assert_eq!((left.a, left.b), (Z_Q3, Z_Q2), "off them on q₂");
    // A preset's exact determinant: 0 is the preset. The least below it is a typed component, with X != Y no pair the
    // other way round can read back: it reads as at 0 and as the least above it, its axes and its angles not jumping.
    let on = read_of(&[(Z_Q3, 1.0)], &[(Z_BETA, 1.0)]);
    assert_eq!(on.preset, Some(0), "a determinant of 0");
    assert!((on.tau1.to_degrees() - 90.0).abs() < 1e-9, "τ₁ at 0");
    for component in [-least, least] {
        let near = read_of(&[(Z_ALPHA, component), (Z_Q3, 1.0)], &[(Z_BETA, 1.0)]);
        assert_eq!((near.a, near.b), (on.a, on.b), "the axes at {component}");
        for (got, want, what) in [
            (near.tau1, on.tau1, "τ₁"),
            (near.tau2, on.tau2, "τ₂"),
            (near.gamma, on.gamma, "γ"),
        ] {
            assert!((got - want).abs() < 1e-9, "{what} at {component}");
        }
    }
    // A preset's frame, exactly: all of it holds, whatever is on the targets.
    let within = read_of(&[(Z_ALPHA, 1.0)], &[(Z_BETA, 1.0), (Z_MU1, 0.5)]);
    assert_eq!(within.preset, Some(0), "on the targets");
    // Where no exact reading holds, a preset allows noise. Its determinant: 1e-12 of the total of 2 below 0.
    let noise = 1e-12;
    let on = read_of(
        &[(Z_ALPHA, -1.5e-12), (Z_Q3, 1.0), (Z_Q1, noise)],
        &[(Z_BETA, 1.0)],
    );
    assert_eq!(on.preset, Some(0), "within the rounding");
    let off = read_of(
        &[(Z_ALPHA, -2.5e-12), (Z_Q3, 1.0), (Z_Q1, noise)],
        &[(Z_BETA, 1.0)],
    );
    assert_eq!(off.preset, None, "past the rounding");
    // And what leaves it: 1.5e-18 of the total of 2 holds, on either vector, and 2.5e-18 does not.
    let leak = 1.5e-18_f64.sqrt();
    let on = read_of(&[(Z_ALPHA, 1.0)], &[(Z_BETA, 1.0), (Z_Q1, leak)]);
    assert_eq!(on.preset, Some(0), "within the preset's cut, on q₂");
    let on = read_of(&[(Z_ALPHA, 1.0), (Z_Q1, leak)], &[(Z_BETA, 1.0)]);
    assert_eq!(on.preset, Some(0), "within the preset's cut, on q₁");
    let leak = 2.5e-18_f64.sqrt();
    let off = read_of(&[(Z_ALPHA, 1.0)], &[(Z_BETA, 1.0), (Z_Q1, leak)]);
    assert_eq!(off.preset, None, "past the preset's cut");
    // The determinant, not its terms' sum: 0.36 − 0.64 is mirrored.
    let mirrored = read_of(
        &[(Z_ALPHA, 0.6), (Z_BETA, 0.8)],
        &[(Z_ALPHA, 0.8), (Z_BETA, 0.6)],
    );
    assert_eq!(mirrored.preset, None, "mirrored: no turn of the preset");
}

/// The largest difference, on any component, between `read`'s basis and `(q1, q2)`.
fn read_back_off(read: &View, q1: &Latent, q2: &Latent) -> f64 {
    let (p1, p2) = read.basis();
    (0..8)
        .map(|k| (p1[k] - q1[k]).abs().max((p2[k] - q2[k]).abs()))
        .fold(0.0, f64::max)
}

/// R-459 G3: a negative determinant is the pair the other way round only where that order reads the basis back. A typed
/// mirrored basis outside the map's image keeps its tilt, and the next edit built from the view keeps the plane near it.
#[test]
fn mock_cube_axis_buttons_a_mirrored_basis_is_swapped_only_where_that_reads_it_back() {
    let vector = |parts: &[(usize, f64)]| -> Latent {
        let mut v = [0.0; 8];
        for &(i, x) in parts {
            v[i] = x;
        }
        v
    };
    let in_frame = |f: SliceFrame, q1: &[(usize, f64)], q2: &[(usize, f64)]| {
        View::framed(
            &Plane {
                z0: [0.0; 8],
                q1: vector(q1),
                q2: vector(q2),
            },
            Some(&f),
        )
    };
    let (q1, q2) = ([(Z_ALPHA, -0.6), (Z_Q3, 0.8)], [(Z_BETA, 1.0)]);
    // X != Y: the swap would lose z_q3. The tilt is kept, on the axes as typed; only α's sign is lost.
    let tilt = 0.8_f64.atan2(0.6).to_degrees();
    let read = in_frame(frame(Z_Q3, Z_Q3, Z_MU1), &q1, &q2);
    assert_eq!((read.a, read.b), (Z_ALPHA, Z_BETA));
    assert!((read.tau1.to_degrees() - tilt).abs() < 1e-9, "{read:?}");
    assert!(
        read.tau2.abs() < 1e-9 && read.gamma.abs() < 1e-9,
        "{read:?}"
    );
    let (p1, p2) = read.basis();
    assert!((p1[Z_Q3] - 0.8).abs() < 1e-12, "z_q3 kept: {p1:?}");
    assert!((p1[Z_ALPHA] - 0.6).abs() < 1e-12, "the sign is what goes");
    assert!((p2[Z_BETA] - 1.0).abs() < 1e-12, "{p2:?}");
    // The next edit built from the view, a zoom, leaves the axes and the tilt where they were.
    let (p1, p2) = read.with_zoom(1.0).basis();
    let zoomed = View::framed(
        &Plane {
            z0: [0.0; 8],
            q1: p1,
            q2: p2,
        },
        Some(&frame(Z_Q3, Z_Q3, Z_MU1)),
    );
    assert_eq!((zoomed.a, zoomed.b), (Z_ALPHA, Z_BETA), "zoomed");
    assert!((zoomed.tau1.to_degrees() - tilt).abs() < 1e-9, "{zoomed:?}");
    rejects("a swap that loses the tilt", || {
        assert_eq!((read.a, read.b), (Z_BETA, Z_ALPHA))
    });
    // X = Y: the swap holds it, exactly.
    let read = in_frame(frame(Z_Q3, Z_Q3, Z_Q3), &q1, &q2);
    assert_eq!((read.a, read.b), (Z_BETA, Z_ALPHA));
    assert!((read.tau2.to_degrees() + tilt).abs() < 1e-9, "{read:?}");
    assert!((read.gamma.to_degrees() + 90.0).abs() < 1e-9, "{read:?}");
    assert!(read_back_off(&read, &vector(&q1), &vector(&q2)) < 1e-12);
    // X != Y with the tilt on the target the swap can hold: it reads back, so it is swapped.
    let (q1, q2) = ([(Z_ALPHA, -0.6), (Z_MU1, 0.8)], [(Z_BETA, 1.0)]);
    let read = in_frame(frame(Z_Q3, Z_Q3, Z_MU1), &q1, &q2);
    assert_eq!((read.a, read.b), (Z_BETA, Z_ALPHA));
    assert!(read_back_off(&read, &vector(&q1), &vector(&q2)) < 1e-12);
    // The read-back's cut, of a total of 2: 1.5e-18 of it left on z_q3 is the swap's noise, 2.5e-18 is not.
    let at = |left: f64| {
        in_frame(
            frame(Z_Q3, Z_Q3, Z_MU1),
            &[(Z_ALPHA, -1.0), (Z_Q3, left.sqrt())],
            &[(Z_BETA, 1.0)],
        )
    };
    assert_eq!((at(1.5e-18).a, at(1.5e-18).b), (Z_BETA, Z_ALPHA), "within");
    assert_eq!((at(2.5e-18).a, at(2.5e-18).b), (Z_ALPHA, Z_BETA), "past it");
    // Neither order reads it back: the swap only if it keeps more of the tilt. Here it keeps both targets' parts, and
    // the order as typed neither.
    let read = in_frame(
        frame(Z_Q3, Z_Q3, Z_MU1),
        &[(Z_ALPHA, -0.6), (Z_MU1, 0.8)],
        &[(Z_BETA, 1.0), (Z_Q3, 0.3)],
    );
    assert_eq!(
        (read.a, read.b),
        (Z_BETA, Z_ALPHA),
        "keeps more of the tilt"
    );
    // And a tie, no tilt to keep, stays as typed: the unequal lengths are no reading's, so no read-back.
    let read = in_frame(
        frame(Z_Q3, Z_Q3, Z_MU1),
        &[(Z_ALPHA, -0.6), (Z_BETA, 0.1)],
        &[(Z_ALPHA, 0.1), (Z_BETA, 1.0)],
    );
    assert_eq!((read.a, read.b), (Z_ALPHA, Z_BETA), "a tie");
}

#[test]
fn mock_cube_axis_buttons_picking_d_sets_the_normal_and_re_tilts_in_one_entry() {
    let (mut app, mut h) = app_with((20.0, 0.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_Q2));
    assert_eq!(
        texts(&h, &app)[1],
        "X z_q3",
        "τ₁ set: the third axis shows X"
    );
    // The button shows X while τ₁ is set: at rest in Slice mode it is D, the normal's.
    app.manifold.compass.mode = Touch::Slice;
    settle(&mut h, &mut app);
    assert_eq!(texts(&h, &app)[1], "D z_q3");
    let before = state(&app);
    click_scope(&mut h, &mut app, COMPASS_D);
    let _ = h.frame(&mut app, Vec::new());
    click_scope(&mut h, &mut app, COMPASS_D_ENTRIES[Z_MU1]);
    settle(&mut h, &mut app);
    let after = state(&app);
    assert_eq!(after.2, frame(Z_MU1, Z_MU1, Z_Q2), "X followed; Y kept");
    let (s, c) = 20.0_f64.to_radians().sin_cos();
    let mut q1 = axis(Z_ALPHA, c);
    q1[Z_MU1] = s;
    let off = q1
        .iter()
        .zip(&after.1.q1)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    assert!(off < 1e-15, "q₁ re-tilted into e_μ1: {:?}", after.1.q1);
    assert_eq!(after.1.q2, before.1.q2, "τ₂ is 0");
    assert_eq!(after.1.z0, before.1.z0, "z₀ kept");
    assert_eq!(texts(&h, &app)[1], "D z_μ1");
    ctrl_z(&mut h, &mut app);
    let undone = state(&app);
    check_one_entry(&before, &after, &undone);
    assert_eq!(texts(&h, &app)[1], "D z_q3");
    // The control: the same pick as two edits, the frame then the basis, is two entries.
    let d0 = state(&app);
    app.set_field(frame_edit(frame(Z_MU1, Z_MU1, Z_Q2), None));
    app.set_field(frame_edit(
        frame(Z_MU1, Z_MU1, Z_Q2),
        Some((q1, after.1.q2)),
    ));
    settle(&mut h, &mut app);
    let d1 = state(&app);
    ctrl_z(&mut h, &mut app);
    let d2 = state(&app);
    rejects("a D pick that makes two entries", || {
        check_one_entry(&d0, &d1, &d2)
    });
}

#[test]
fn mock_cube_axis_buttons_normal_edit_carries_the_basis_only_where_a_follower_is_tilted() {
    use engine::contract::set_field::{Edit, SimField};
    let carries = |t1: f64, t2: f64, x: usize, y: usize, to: usize| -> Option<bool> {
        let v = View {
            x,
            y,
            ..view(t1, t2, 0.0, 1.0)
        };
        normal_edit(&v, to).map(|e| match e.edit {
            Edit::Sim(SimField::SliceFrame { basis, frame }) => {
                assert_eq!(frame.normal as usize, to);
                basis.is_some()
            }
            other => panic!("not a frame edit: {other:?}"),
        })
    };
    // None for the normal it is and for a dimension in the plane.
    assert_eq!(carries(0.0, 0.0, Z_Q3, Z_Q3, Z_Q3), None);
    assert_eq!(carries(0.0, 0.0, Z_Q3, Z_Q3, Z_ALPHA), None);
    assert_eq!(carries(0.0, 0.0, Z_Q3, Z_Q3, Z_BETA), None);
    // Zero angles: the frame alone.
    assert_eq!(carries(0.0, 0.0, Z_Q3, Z_Q3, Z_MU1), Some(false));
    // X follows, τ₁ set; Y follows, τ₂ set; a target that does not follow keeps the plane.
    assert_eq!(carries(20.0, 0.0, Z_Q3, Z_Q2, Z_MU1), Some(true));
    assert_eq!(carries(0.0, 15.0, Z_Q2, Z_Q3, Z_MU1), Some(true));
    assert_eq!(carries(20.0, 0.0, Z_Q2, Z_Q3, Z_MU1), Some(false));
    assert_eq!(carries(0.0, 15.0, Z_Q3, Z_Q2, Z_MU1), Some(false));
    assert_eq!(carries(20.0, 15.0, Z_Q2, Z_Q0, Z_MU1), Some(false));
}

#[test]
fn mock_cube_axis_buttons_the_third_axis_dropdown_picks_the_target_it_shows() {
    use engine::contract::set_field::{Edit, SimField};
    let (app, _h) = app_with((20.0, 15.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_MU1));
    let sim = &app.snapshot().sim;
    let framed =
        |shows: Shows, to: usize| match pick_edit(sim, Edge::D, to, shows).expect("offered").edit {
            Edit::Sim(SimField::SliceFrame { frame, .. }) => frame,
            other => panic!("not a frame edit: {other:?}"),
        };
    assert_eq!(framed(Shows::X, Z_Q2), frame(Z_Q3, Z_Q2, Z_MU1), "X's");
    assert_eq!(framed(Shows::Y, Z_Q2), frame(Z_Q3, Z_Q3, Z_Q2), "Y's");
    assert_eq!(framed(Shows::Normal, Z_Q2), frame(Z_Q2, Z_Q2, Z_MU1), "D's");
}

#[test]
fn mock_cube_axis_buttons_a_plucker_steer_taking_d_moves_it_by_the_slice_axis_rule() {
    let (mut app, mut h) = app_with((0.0, 0.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_Q3));
    let before = state(&app);
    app.manifold.compass.view = CompassView::Plucker;
    settle(&mut h, &mut app);
    let at = GridPlace::fit(square(&h)).cell_rect(Z_Q2, Z_Q3).center();
    for events in crate::capture::click(at) {
        let _ = h.frame(&mut app, events);
    }
    settle(&mut h, &mut app);
    let after = state(&app);
    let rule = slice_axis(Z_Q2, Z_Q3);
    assert_eq!(rule, Z_ALPHA, "the first axis the plane does not show");
    assert_eq!(
        after.2,
        frame(rule, rule, rule),
        "D moved, a target with it"
    );
    assert_eq!(after.1.q1, axis(Z_Q2, 1.0));
    assert_eq!(after.1.q2, axis(Z_Q3, 1.0));
    app.manifold.compass.view = CompassView::Cube;
    settle(&mut h, &mut app);
    assert_eq!(texts(&h, &app), ["H z_q2", "D z_α", "V z_q3"]);
    ctrl_z(&mut h, &mut app);
    check_one_entry(&before, &after, &state(&app));
    rejects("a steer that leaves D in the plane", || {
        assert_eq!(after.2, frame(Z_Q3, Z_Q3, Z_Q3))
    });
    // From the keyboard, Space on the cell cursor, the same rule.
    assert_eq!(
        compass::views::steer_on(&app.snapshot().sim, Z_Q3, Z_Q2),
        None,
        "the lower triangle is empty"
    );
    assert_eq!(
        compass::views::steer_on(&app.snapshot().sim, Z_Q3, Z_Q3),
        None,
        "the diagonal is empty"
    );
    assert_eq!(
        compass::views::steer_on(&app.snapshot().sim, 7, 8),
        None,
        "past the grid"
    );
    assert!(compass::views::steer_on(&app.snapshot().sim, 6, 7).is_some());
}

#[test]
fn mock_cube_axis_buttons_locked_a_pick_turns_about_the_pin() {
    let (mut app, mut h) = app_with((0.0, 0.0, 0.0), 0.5, frame(Z_Q3, Z_Q3, Z_Q3));
    let sim = app.snapshot().sim.clone();
    let anchor = crate::explore::manifold_view::point(&sim.plane, 0.75, 0.5);
    app.set_field(lock_edit(Lock {
        locked: true,
        z_locked: anchor,
    }));
    settle(&mut h, &mut app);
    click_scope(&mut h, &mut app, COMPASS_H);
    let _ = h.frame(&mut app, Vec::new());
    click_scope(&mut h, &mut app, COMPASS_H_ENTRIES[Z_Q0]);
    settle(&mut h, &mut app);
    let after = app.snapshot().sim.plane.clone();
    assert_eq!(after.q1, axis(Z_Q0, 0.5));
    let (s, t) = crate::explore::manifold_view::chart_coords(&after, &anchor).expect("a plane");
    assert!(
        (s - 0.75).abs() < 1e-12 && (t - 0.5).abs() < 1e-12,
        "the anchor's chart point kept: ({s}, {t})"
    );
    assert_ne!(after.z0, sim.plane.z0, "the centre moved about the pin");
}

/// Tab onto the Compass, Enter in, onto the compass.
fn into_compass(h: &mut Headless, app: &mut App<MockSide>) {
    for key in [Key::Tab; 5] {
        press(h, app, key, NONE);
    }
    press(h, app, Key::Enter, NONE);
    assert_eq!(focus(app), [COMPASS, COMPASS_VIEW]);
}

#[test]
fn mock_cube_axis_buttons_keyboard_tab_enter_right_to_h_enter_down_enter_picks() {
    let (mut app, mut h) = app_with((0.0, 0.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_Q3));
    into_compass(&mut h, &mut app);
    for id in [
        COMPASS_TILT,
        COMPASS_SLICE,
        COMPASS_CUBE,
        COMPASS_PLUCKER,
        COMPASS_JACOBI,
        COMPASS_SHAPE,
        COMPASS_H,
    ] {
        press(&mut h, &mut app, Key::ArrowRight, NONE);
        assert_eq!(focus(&app), [COMPASS, id]);
    }
    let before = state(&app);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(
        focus(&app),
        [COMPASS, COMPASS_H, COMPASS_H_ENTRIES[Z_ALPHA]],
        "Enter opens it, on its first entry"
    );
    // Esc closes it unchanged.
    press(&mut h, &mut app, Key::Escape, NONE);
    assert_eq!(focus(&app), [COMPASS, COMPASS_H]);
    assert_eq!(state(&app), before);
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::ArrowDown, NONE);
    assert_eq!(focus(&app), [COMPASS, COMPASS_H, COMPASS_H_ENTRIES[Z_Q0]]);
    press(&mut h, &mut app, Key::Enter, NONE);
    assert_eq!(focus(&app), [COMPASS, COMPASS_H], "back on the button");
    assert_eq!(app.view.focus.mode, KeyMode::Navigation);
    settle(&mut h, &mut app);
    assert_eq!(app.snapshot().sim.plane.q1, axis(Z_Q0, 1.0), "picked");
    assert_eq!(texts(&h, &app)[0], "H z_q0");
    assert_eq!(state(&app).0, before.0 + 1);
    ctrl_z(&mut h, &mut app);
    assert_eq!(state(&app), before, "one Ctrl+Z");
    // V and D, then nothing more: → stops on the last sibling.
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), [COMPASS, COMPASS_V]);
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), [COMPASS, COMPASS_D]);
    press(&mut h, &mut app, Key::ArrowRight, NONE);
    assert_eq!(focus(&app), [COMPASS, COMPASS_D], "D is last with no badge");
    // A D pick from the keyboard, in Tilt with both angles 0: the normal.
    press(&mut h, &mut app, Key::Enter, NONE);
    for _ in 0..4 {
        press(&mut h, &mut app, Key::ArrowDown, NONE);
    }
    assert_eq!(focus(&app), [COMPASS, COMPASS_D, COMPASS_D_ENTRIES[Z_MU1]]);
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert_eq!(app.snapshot().sim.slice.frame, frame(Z_MU1, Z_MU1, Z_MU1));
    // A V pick too.
    press(&mut h, &mut app, Key::ArrowLeft, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    press(&mut h, &mut app, Key::Enter, NONE);
    settle(&mut h, &mut app);
    assert_eq!(app.snapshot().sim.plane.q2, axis(Z_BETA, 1.0), "z_β kept");
    rejects("the pick by the wrong entry", || {
        assert_eq!(app.snapshot().sim.plane.q1, axis(Z_Q1, 1.0))
    });
}

#[test]
fn mock_cube_axis_buttons_are_the_cube_views_siblings_and_leave_with_it() {
    let (mut app, mut h) = app_with((20.0, 15.0, 0.0), 1.0, frame(Z_Q3, Z_Q3, Z_MU1));
    let siblings = |app: &App<MockSide>| -> Vec<String> {
        app.keyboard
            .tree(Mode::Explore)
            .children(Some(COMPASS))
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    };
    let cube = [
        COMPASS_VIEW,
        COMPASS_TILT,
        COMPASS_SLICE,
        COMPASS_CUBE,
        COMPASS_PLUCKER,
        COMPASS_JACOBI,
        COMPASS_SHAPE,
        COMPASS_H,
        COMPASS_V,
        COMPASS_D,
        COMPASS_BADGE,
    ];
    assert_eq!(siblings(&app), cube, "the badge last while it is shown");
    // The third axis's breadcrumb name follows what it shows.
    assert_eq!(
        app.keyboard
            .tree(Mode::Explore)
            .get(COMPASS_D)
            .map(|s| s.label),
        Some("X")
    );
    // A focus on H when the mouse picks a grid view moves to that view's button.
    app.view.focus.path = vec![COMPASS.to_owned(), COMPASS_H.to_owned()];
    app.manifold.compass.view = CompassView::Jacobi;
    settle(&mut h, &mut app);
    assert_eq!(siblings(&app), cube[..7], "none of them in a grid view");
    assert_eq!(focus(&app), [COMPASS, COMPASS_JACOBI]);
    // Back on the Cube, they come back in order.
    app.manifold.compass.view = CompassView::Cube;
    settle(&mut h, &mut app);
    assert_eq!(siblings(&app), cube);
    // The badge goes when an angle does; a focus on it moves to D.
    app.view.focus.path = vec![COMPASS.to_owned(), COMPASS_BADGE.to_owned()];
    app.view.focus.mode = KeyMode::Interaction;
    let v = View {
        y: Z_MU1,
        ..view(20.0, 0.0, 0.0, 1.0)
    };
    app.set_field(crate::explore::manifold_view::basis_edit(v.basis()));
    settle(&mut h, &mut app);
    assert_eq!(siblings(&app), cube[..10]);
    assert_eq!(focus(&app), [COMPASS, COMPASS_D]);
    assert_eq!(app.view.focus.mode, KeyMode::Navigation);
    // A focus elsewhere, and a whole path in the compass, stay where they are.
    app.view.focus.path = vec![COMPASS.to_owned(), COMPASS_V.to_owned()];
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(focus(&app), [COMPASS, COMPASS_V]);
    app.view.focus.path = vec!["figure".to_owned(), "gone".to_owned()];
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(focus(&app)[0], "figure", "not the compass's to move");
    rejects("the badge kept as a sibling with no badge", || {
        assert_eq!(siblings(&app), cube)
    });
}
