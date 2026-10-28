//! REQ-GUI-205 (R-457, R-459 G4): each tilt angle tilts into its own target, `u₁ = cos τ₁ e_a + sin τ₁ e_X`,
//! `u₂ = cos τ₂ e_b + sin τ₂ e_Y`, the defaults the slice normal; the targets and the normal are SimConfig's slice
//! frame, each pick one undoable history entry, re-tilting where its angle is not 0; the normal never lies in the
//! plane, a preset change or a Chart basis edit moving it in the same entry; the compass's readout names both targets.
//! Each check is shown failing on an input it must reject (R-176, through `rejects`).

use eframe::egui::{Key, Modifiers};
use engine::contract::sim_config::{Latent, Plane, SliceFrame};

use super::support::{focus, frame_texts, headless, mock_app, press, rejects};
use crate::app::App;
use crate::explore::compass;
use crate::explore::manifold_view::chart::preset_basis;
use crate::explore::manifold_view::{
    self as mv, basis_edit, frame_edit, slice_axis, target_choices, Touch, View, AXES, PRESETS,
    TILT_X, TILT_X_ENTRIES, TILT_Y, TILT_Y_ENTRIES,
};
use crate::headless::Headless;
use crate::side::MockSide;

const NONE: Modifiers = Modifiers::NONE;
const Z_Q0: usize = 2;
const Z_Q2: usize = 4;
const Z_Q3: usize = 5;
const Z_MU1: usize = 6;
const Z_MU2: usize = 7;

fn index(i: usize) -> u32 {
    u32::try_from(i).expect("a latent index")
}

/// The slice frame `(normal, X, Y)`.
fn frame(normal: usize, x: usize, y: usize) -> SliceFrame {
    SliceFrame {
        normal: index(normal),
        tilt_x: index(x),
        tilt_y: index(y),
    }
}

/// `e_i` scaled by `s`.
fn e(i: usize, s: f64) -> Latent {
    let mut v = [0.0; 8];
    v[i] = s;
    v
}

fn max_diff(a: &Latent, b: &Latent) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// The map before R-457, both angles tilting into the slice normal `e_d`: `u₁ = cos τ₁ e_a + sin τ₁ e_d`,
/// `u₂ = cos τ₂ e_b + sin τ₂ e_d`, `q₁ = σ(cos γ u₁ + sin γ u₂)`, `q₂ = σ(cos γ u₂ − sin γ u₁)`.
fn todays_map(v: &View) -> (Latent, Latent) {
    let (s1, c1) = v.tau1.sin_cos();
    let (s2, c2) = v.tau2.sin_cos();
    let (sg, cg) = v.gamma.sin_cos();
    let mut u1 = e(v.a, c1);
    u1[v.d] += s1;
    let mut u2 = e(v.b, c2);
    u2[v.d] += s2;
    let q1 = std::array::from_fn(|i| v.scale * (cg * u1[i] + sg * u2[i]));
    let q2 = std::array::from_fn(|i| v.scale * (cg * u2[i] - sg * u1[i]));
    (q1, q2)
}

/// Asserts `View::basis` with the default targets is [`todays_map`] at every view of `views`, to 1e-15.
fn check_todays_map(views: &[View]) {
    for v in views {
        let (q1, q2) = v.basis();
        let (w1, w2) = todays_map(&View {
            x: v.d,
            y: v.d,
            ..*v
        });
        let off = max_diff(&q1, &w1).max(max_diff(&q2, &w2));
        assert!(off <= 1e-15, "{v:?}: off by {off}");
    }
}

#[test]
fn tilt_targets_the_defaults_give_todays_map() {
    let degrees = [-80.0, -30.0, 0.0, 12.5, 45.0, 89.0];
    let mut views = Vec::new();
    for (p, &(a, b)) in PRESETS.iter().enumerate() {
        let d = slice_axis(a, b);
        // The defaults: a plane's view with no slice frame picked reads both targets as the slice normal.
        let read = View::of(&Plane {
            z0: [0.0; 8],
            q1: e(a, 1.0),
            q2: e(b, 1.0),
        });
        assert_eq!((read.preset, read.d, read.x, read.y), (Some(p), d, d, d));
        for (k, &tau1) in degrees.iter().enumerate() {
            for &tau2 in &degrees {
                for gamma in [-170.0, -45.0, 0.0, 30.0, 120.0] {
                    let scale = [0.25, 1.0, 2.0][k % 3];
                    views.push(View {
                        tau1: f64::to_radians(tau1),
                        tau2: f64::to_radians(tau2),
                        gamma: f64::to_radians(gamma),
                        scale,
                        ..read
                    });
                }
            }
        }
    }
    check_todays_map(&views);
    rejects(
        "τ₁ tilting into another dimension than the normal",
        || {
            let v = View {
                x: Z_Q2,
                tau1: 0.5,
                ..views[0]
            };
            check_todays_map(&[v])
        },
    );
}

/// Asserts `(q1, q2)` is `σ(cos τ₁ e_a + sin τ₁ e_X)`, `σ(cos τ₂ e_b + sin τ₂ e_Y)`, to 1e-15, and `q₁·q₂ = 0`.
fn check_own_axes(
    (q1, q2): (Latent, Latent),
    (a, x, tau1): (usize, usize, f64),
    (b, y, tau2): (usize, usize, f64),
    scale: f64,
) {
    let mut w1 = e(a, scale * tau1.cos());
    w1[x] += scale * tau1.sin();
    let mut w2 = e(b, scale * tau2.cos());
    w2[y] += scale * tau2.sin();
    assert!(max_diff(&q1, &w1) <= 1e-15, "q₁ {q1:?}, not {w1:?}");
    assert!(max_diff(&q2, &w2) <= 1e-15, "q₂ {q2:?}, not {w2:?}");
    let dot: f64 = q1.iter().zip(&q2).map(|(u, v)| u * v).sum();
    assert!(dot.abs() <= 1e-15, "q₁·q₂ = {dot}");
}

#[test]
fn tilt_targets_each_angle_tilts_into_its_own_target() {
    let (t1, t2, scale) = (30f64.to_radians(), 20f64.to_radians(), 0.7);
    let v = View {
        preset: Some(0),
        a: 0,
        b: 1,
        d: Z_Q3,
        x: Z_Q3,
        y: Z_MU1,
        scale,
        tau1: t1,
        tau2: t2,
        gamma: 0.0,
    };
    check_own_axes(v.basis(), (0, Z_Q3, t1), (1, Z_MU1, t2), scale);
    // Read back with its frame, the view is the same.
    let (q1, q2) = v.basis();
    let plane = Plane {
        z0: [0.0; 8],
        q1,
        q2,
    };
    let back = View::framed(&plane, Some(&v.frame()));
    assert_eq!(
        (back.a, back.b, back.d, back.x, back.y),
        (0, 1, Z_Q3, Z_Q3, Z_MU1)
    );
    assert!((back.tau1 - t1).abs() < 1e-12 && (back.tau2 - t2).abs() < 1e-12);
    rejects("τ₂ tilting into X's target", || {
        check_own_axes(
            View { y: Z_Q3, ..v }.basis(),
            (0, Z_Q3, t1),
            (1, Z_MU1, t2),
            scale,
        )
    });
}

/// Asserts the dimensions `offered` on the plane `(a, b)` are the six not `e_a` or `e_b`, in order.
fn check_offers(offered: &[usize], a: usize, b: usize) {
    assert_eq!(offered.len(), 6, "six: {offered:?}");
    assert!(
        !offered.contains(&a) && !offered.contains(&b),
        "{offered:?} on ({a}, {b})"
    );
    assert!(
        offered.windows(2).all(|w| w[0] < w[1]),
        "in order: {offered:?}"
    );
}

#[test]
fn tilt_targets_each_dropdown_offers_the_six_not_in_the_plane() {
    for &(a, b) in &PRESETS {
        check_offers(&target_choices(a, b), a, b);
    }
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    // The keyboard's menus hold the same six, each named by its axis, and follow a preset change.
    for (p, (q1, q2)) in [(0, (e(0, 1.0), e(1, 1.0))), (1, (e(2, 1.0), e(3, 1.0)))] {
        app.set_field(basis_edit((q1, q2)));
        let _ = h.frame(&mut app, Vec::new());
        let (a, b) = PRESETS[p];
        let tree = app.keyboard.tree(engine::contract::view_ui::Mode::Explore);
        for (menu, entries) in [(TILT_X, TILT_X_ENTRIES), (TILT_Y, TILT_Y_ENTRIES)] {
            let offered: Vec<usize> = tree
                .children(Some(menu))
                .iter()
                .map(|id| entries.iter().position(|e| e == id).expect("an entry"))
                .collect();
            check_offers(&offered, a, b);
            assert_eq!(offered, target_choices(a, b), "{menu} on preset {p}");
        }
    }
    // Opened, the τ₁ dropdown draws its six by their names.
    keys(&mut h, &mut app, &to_target(1));
    assert_eq!(app.keyboard.open_menu(), Some(TILT_X));
    let texts = frame_texts(&mut h, &mut app);
    for i in target_choices(2, 3) {
        assert!(texts.contains(&AXES[i].to_owned()), "{} drawn", AXES[i]);
    }
    rejects("a dropdown that offers e_a", || {
        check_offers(&[0, 2, 3, 4, 5, 6], 0, 1)
    });
}

/// Presses `keys` with no modifier.
fn keys(h: &mut Headless, app: &mut App<MockSide>, keys: &[Key]) {
    for key in keys {
        press(h, app, *key, NONE);
    }
}

/// The keys from nothing focused to the Tilt section's child `k` (`τ₁` 0, X's dropdown 1, `τ₂` 2, Y's 3), then Enter.
fn to_target(k: usize) -> Vec<Key> {
    let mut out = vec![Key::Tab, Key::Tab, Key::Enter];
    out.extend([Key::ArrowDown; 3]);
    out.push(Key::Enter);
    out.extend(std::iter::repeat_n(Key::ArrowDown, k));
    out.push(Key::Enter);
    out
}

/// Picks entry `to` of the Tilt section's dropdown `k` (1 X, 3 Y) by the keys: Enter opens it on its first entry,
/// ↓ chooses, Enter picks (R-457 E5); then a frame for the snapshot to follow.
fn pick(h: &mut Headless, app: &mut App<MockSide>, k: usize, to: usize) {
    let view = View::read(&app.snapshot().sim);
    let at = target_choices(view.a, view.b)
        .iter()
        .position(|&i| i == to)
        .expect("offered");
    let mut all = to_target(k);
    all.extend(std::iter::repeat_n(Key::ArrowDown, at));
    all.push(Key::Enter);
    keys(h, app, &all);
    let _ = h.frame(app, Vec::new());
}

/// Ctrl+Z, the contract's undo, then a frame for the snapshot to follow.
fn ctrl_z(h: &mut Headless, app: &mut App<MockSide>) {
    press(h, app, Key::Z, Modifiers::CTRL);
    let _ = h.frame(app, Vec::new());
}

/// The app's undo depth, basis and slice frame.
fn state(app: &App<MockSide>) -> (u32, (Latent, Latent), SliceFrame) {
    let s = app.snapshot();
    (
        s.history.undo_depth,
        (s.sim.plane.q1, s.sim.plane.q2),
        s.sim.slice.frame,
    )
}

/// Asserts the edit from `before` to `after` was one history entry leaving the basis `basis` and the frame `frame`.
fn check_entry(
    before: &(u32, (Latent, Latent), SliceFrame),
    after: &(u32, (Latent, Latent), SliceFrame),
    basis: (Latent, Latent),
    frame: SliceFrame,
) {
    assert_eq!(after.0, before.0 + 1, "one history entry");
    assert!(
        max_diff(&after.1 .0, &basis.0).max(max_diff(&after.1 .1, &basis.1)) <= 1e-15,
        "the basis {:?}, not {basis:?}",
        after.1
    );
    assert_eq!(after.2, frame, "the slice frame");
}

/// Asserts one undo brought `undone` back to `before`.
fn check_undone(
    before: &(u32, (Latent, Latent), SliceFrame),
    undone: &(u32, (Latent, Latent), SliceFrame),
) {
    assert_eq!(
        undone, before,
        "one Ctrl+Z restores the basis and the frame"
    );
}

/// The mock app with its plane at `view`'s basis, its frame `view`'s, settled.
fn app_at(view: &View) -> (App<MockSide>, Headless) {
    let mut app = mock_app();
    let mut h = headless();
    app.set_field(frame_edit(view.frame(), Some(view.basis())));
    let _ = h.frame(&mut app, Vec::new());
    (app, h)
}

#[test]
fn tilt_targets_a_pick_at_an_angle_re_tilts_in_one_entry() {
    let start = View {
        tau1: 30f64.to_radians(),
        ..View::read(&mock_app().snapshot().sim)
    };
    let (mut app, mut h) = app_at(&start);
    let before = state(&app);
    assert_eq!(before.2, frame(Z_Q3, Z_Q3, Z_Q3));
    pick(&mut h, &mut app, 1, Z_Q2);
    assert_eq!(focus(&app).last(), Some(&TILT_X), "back on the dropdown");
    let after = state(&app);
    let into_q2 = View { x: Z_Q2, ..start };
    check_entry(&before, &after, into_q2.basis(), frame(Z_Q3, Z_Q2, Z_Q3));
    let (q1, _) = after.1;
    assert!(
        q1[Z_Q2] > 0.49 && q1[Z_Q3] == 0.0,
        "q₁ tilts into e_q2: {q1:?}"
    );
    let line = &app.console().last().expect("a line").message;
    assert!(
        line.contains("Slice.frame") && line.contains("Plane.q1"),
        "{line}"
    );
    ctrl_z(&mut h, &mut app);
    let undone = state(&app);
    check_undone(&before, &undone);
    // The controls: the pick as two entries, the frame then the basis; and the frame alone, not re-tilted.
    let (mut split, mut sh) = app_at(&start);
    let split_before = state(&split);
    split.set_field(frame_edit(frame(Z_Q3, Z_Q2, Z_Q3), None));
    split.set_field(basis_edit(into_q2.basis()));
    let _ = sh.frame(&mut split, Vec::new());
    let split_after = state(&split);
    rejects("a target change that makes two history entries", || {
        check_entry(
            &split_before,
            &split_after,
            into_q2.basis(),
            frame(Z_Q3, Z_Q2, Z_Q3),
        )
    });
    ctrl_z(&mut sh, &mut split);
    let split_undone = state(&split);
    rejects("an undo that restores the basis but not the target", || {
        check_undone(&split_before, &split_undone)
    });
    let (mut flat, mut fh) = app_at(&start);
    let flat_before = state(&flat);
    flat.set_field(frame_edit(frame(Z_Q3, Z_Q2, Z_Q3), None));
    let _ = fh.frame(&mut flat, Vec::new());
    let flat_after = state(&flat);
    rejects(
        "a target change that does not re-tilt at a non-zero angle",
        || {
            check_entry(
                &flat_before,
                &flat_after,
                into_q2.basis(),
                frame(Z_Q3, Z_Q2, Z_Q3),
            )
        },
    );
}

#[test]
fn tilt_targets_a_pick_at_zero_edits_the_frame_alone() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let before = state(&app);
    pick(&mut h, &mut app, 3, Z_MU1);
    let after = state(&app);
    check_entry(&before, &after, before.1, frame(Z_Q3, Z_Q3, Z_MU1));
    let line = &app.console().last().expect("a line").message;
    assert!(
        line.contains("Slice.frame") && !line.contains("Plane"),
        "no basis edit: {line}"
    );
    ctrl_z(&mut h, &mut app);
    check_undone(&before, &state(&app));
    rejects("a pick that moves the basis at τ₂ = 0", || {
        check_entry(
            &before,
            &after,
            (e(0, 1.0), e(1, 0.9)),
            frame(Z_Q3, Z_Q3, Z_MU1),
        )
    });
}

/// The keys from nothing focused to the preset, then Enter: the next preset (§G2).
const NEXT_PRESET: [Key; 5] = [Key::Tab, Key::Tab, Key::Enter, Key::Enter, Key::Enter];

/// Asserts the slice normal of `frame` is off the plane `(a, b)`, and is the slice-axis rule's.
fn check_normal_off(frame: SliceFrame, (a, b): (usize, usize)) {
    let normal = usize::try_from(frame.normal).expect("an index");
    assert!(
        normal != a && normal != b,
        "the normal {normal} in the plane ({a}, {b})"
    );
    assert_eq!(normal, slice_axis(a, b), "the slice-axis rule");
}

#[test]
fn tilt_targets_a_preset_taking_x_resets_it_and_the_angles() {
    let start = View {
        x: Z_Q0,
        tau1: 30f64.to_radians(),
        tau2: -10f64.to_radians(),
        ..View::read(&mock_app().snapshot().sim)
    };
    let (mut app, mut h) = app_at(&start);
    assert_eq!(state(&app).2, frame(Z_Q3, Z_Q0, Z_Q3));
    let before = state(&app);
    keys(&mut h, &mut app, &NEXT_PRESET);
    let _ = h.frame(&mut app, Vec::new());
    let after = state(&app);
    // z_q0 × z_q1 takes X = z_q0: X resets to the normal, z_q3, and the preset's basis has no tilt.
    let view = View::read(&app.snapshot().sim);
    assert_eq!((view.preset, view.a, view.b), (Some(1), 2, 3));
    check_entry(
        &before,
        &after,
        preset_basis(start, 1),
        frame(Z_Q3, Z_Q3, Z_Q3),
    );
    assert_eq!((view.tau1, view.tau2, view.gamma), (0.0, 0.0, 0.0));
    ctrl_z(&mut h, &mut app);
    check_undone(&before, &state(&app));
    rejects("X left in the plane", || {
        check_entry(
            &before,
            &after,
            preset_basis(start, 1),
            frame(Z_Q3, Z_Q0, Z_Q3),
        )
    });
}

#[test]
fn tilt_targets_a_preset_taking_the_normal_moves_it_in_one_entry() {
    // z_α × z_μ1, the normal z_q3, X following it and Y on z_μ2; the next preset is z_q3 × z_μ1.
    let base = View::read(&mock_app().snapshot().sim);
    let mut app = mock_app();
    let mut h = headless();
    app.set_field(frame_edit(
        frame(Z_Q3, Z_Q3, Z_MU2),
        Some(preset_basis(base, 4)),
    ));
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(View::read(&app.snapshot().sim).preset, Some(4));
    let before = state(&app);
    keys(&mut h, &mut app, &NEXT_PRESET);
    let _ = h.frame(&mut app, Vec::new());
    let after = state(&app);
    let view = View::read(&app.snapshot().sim);
    assert_eq!((view.preset, view.a, view.b), (Some(5), Z_Q3, Z_MU1));
    // The normal moves to z_α, the first axis z_q3 × z_μ1 leaves hidden; X follows it; Y keeps z_μ2.
    check_entry(&before, &after, preset_basis(base, 5), frame(0, 0, Z_MU2));
    check_normal_off(after.2, (Z_Q3, Z_MU1));
    let line = &app.console().last().expect("a line").message;
    assert!(
        line.contains("Slice.frame") && line.contains("Plane.q1"),
        "{line}"
    );
    ctrl_z(&mut h, &mut app);
    let undone = state(&app);
    check_undone(&before, &undone);
    assert_eq!(
        View::read(&app.snapshot().sim).preset,
        Some(4),
        "the preset restored"
    );
    // The control: the preset change as a bare basis edit, the normal left in the plane.
    let mut bare = mock_app();
    bare.set_field(frame_edit(
        frame(Z_Q3, Z_Q3, Z_MU2),
        Some(preset_basis(base, 4)),
    ));
    bare.set_field(basis_edit(preset_basis(base, 5)));
    rejects(
        "a preset change that leaves the normal in the plane",
        || check_normal_off(bare.snapshot().sim.slice.frame, (Z_Q3, Z_MU1)),
    );
}

#[test]
fn tilt_targets_a_chart_basis_edit_taking_the_normal_moves_it() {
    // q₂ = 0.02 e_β + 0.02 e_q3, read as z_α × z_β tilted into z_q3; stepping q₂'s z_β down to 0 leaves q₂ on z_q3,
    // the plane z_α × z_q3, which takes the normal.
    let mut app = mock_app();
    let mut h = headless();
    let mut q2 = e(1, 0.02);
    q2[Z_Q3] = 0.02;
    app.set_field(basis_edit((e(0, 1.0), q2)));
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(View::read(&app.snapshot().sim).preset, Some(0));
    let before = state(&app);
    // To q₂'s editor (Chart's third child), its z_β component, Enter to interact, ↓ one step.
    keys(
        &mut h,
        &mut app,
        &[
            Key::Tab,
            Key::Tab,
            Key::Enter,
            Key::Enter,
            Key::ArrowDown,
            Key::ArrowDown,
            Key::Enter,
            Key::ArrowDown,
            Key::Enter,
            Key::ArrowDown,
        ],
    );
    assert_eq!(focus(&app).last(), Some(&mv::Q2_COMPONENTS[1]));
    let _ = h.frame(&mut app, Vec::new());
    let after = state(&app);
    check_entry(&before, &after, (e(0, 1.0), e(Z_Q3, 0.02)), frame(1, 1, 1));
    check_normal_off(after.2, (0, Z_Q3));
    ctrl_z(&mut h, &mut app);
    check_undone(&before, &state(&app));
    rejects("the normal left on z_q3", || {
        check_normal_off(before.2, (0, Z_Q3))
    });
}

/// Asserts the readout's targets line names `x` and `y` with their angles.
fn check_readout(line: &str, x: &str, tau1: &str, y: &str, tau2: &str) {
    assert_eq!(line, format!("τ₁ → {x} {tau1}° · τ₂ → {y} {tau2}°"));
}

#[test]
fn tilt_targets_the_readout_names_both_targets() {
    let start = View {
        x: Z_Q3,
        y: Z_MU1,
        tau1: 12f64.to_radians(),
        ..View::read(&mock_app().snapshot().sim)
    };
    let z0 = [0.0; 8];
    let tilt = compass::readout(&z0, &start, Touch::Tilt);
    check_readout(&tilt[1], "z_q3", "+12.0", "z_μ1", "0.0");
    let slice = compass::readout(&z0, &start, Touch::Slice);
    assert_eq!(
        slice[0],
        format!("mode slice · {}", tilt[1]),
        "slice mode names them too"
    );
    // The app's compass reads them out, its frame the snapshot's.
    let (mut app, mut h) = app_at(&start);
    let texts = frame_texts(&mut h, &mut app);
    assert!(texts.contains(&tilt[1]), "{texts:?}");
    rejects("the targets swapped", || {
        check_readout(&tilt[1], "z_μ1", "+12.0", "z_q3", "0.0")
    });
}
