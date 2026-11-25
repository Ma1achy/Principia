//! The two tilt maps: "Tip as sheet", the plane turned rigidly by `m = √(τ₁² + τ₂²)` about
//! `k = (τ₂ e_a − τ₁ e_b)/m`, the default, and "Lean each axis", each axis a mix of its own coordinate and its target,
//! an option, chosen by the slice frame's `tilt_map`; the angles stored as set beside it. Each test's pass criterion is
//! written above it before it was run, and each property is shown failing on an input it must reject (`rejects`).

use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{Latent, Plane, SimConfig, SliceFrame, TiltMap};
use engine::contract::snapshot::Snapshot;
use engine::contract::view_state;

use super::support::{headless, mock_app, rejects, SLICE_FRAME};
use crate::explore::compass::{self, components, drawn_normal, lean_warnings, mirrored_line};
use crate::explore::manifold_view::tilt::{map_edit, Angles, LEAN_RANGE, TILT_RANGE};
use crate::explore::manifold_view::{
    self as mv, reset, same_plane, Out, Scratch, Touch, View, GAMMA, RESET_TILT, TAU1, TAU2,
    TILT_MAP_LEAN, TILT_MAP_SHEET, TILT_X, TILT_Y,
};
use crate::keyboard::keymap::Direction;
use crate::keyboard::scopes::{Adjust, StepKind};

const Z_ALPHA: usize = 0;
const Z_BETA: usize = 1;
const Z_Q3: usize = 5;
const Z_MU1: usize = 6;

/// The mock's first preset `z_α × z_β` at `(τ₁, τ₂, γ)` degrees, scale 1, targets `x` and `y`, under `map`.
fn view(tau1: f64, tau2: f64, gamma: f64, x: usize, y: usize, map: TiltMap) -> View {
    View {
        preset: Some(0),
        a: Z_ALPHA,
        b: Z_BETA,
        d: Z_Q3,
        x,
        y,
        scale: 1.0,
        tau1: tau1.to_radians(),
        tau2: tau2.to_radians(),
        gamma: gamma.to_radians(),
        map,
    }
}

fn dot(a: &Latent, b: &Latent) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn max_diff((p1, p2): (Latent, Latent), (q1, q2): (Latent, Latent)) -> f64 {
    p1.iter()
        .zip(&q1)
        .chain(p2.iter().zip(&q2))
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// Tip as sheet written out apart from the code: Rodrigues' rotation by `m` about `k = (τ₂ e_a − τ₁ e_b)/m` in
/// `(e_a, e_b, e_X)`, the angles wrapped in total tip past 180°, `τ·(m − 360)/m`, then `γ` and `σ = 1`.
fn rodrigues(tau1: f64, tau2: f64, gamma: f64, x: usize) -> (Latent, Latent) {
    let m0 = tau1.hypot(tau2);
    let (t1, t2) = if m0 > 180.0 {
        (tau1 * (m0 - 360.0) / m0, tau2 * (m0 - 360.0) / m0)
    } else {
        (tau1, tau2)
    };
    let m = t1.hypot(t2);
    let k = [t2 / m, -t1 / m, 0.0];
    let rotate = |v: [f64; 3]| -> [f64; 3] {
        let (s, c) = m.to_radians().sin_cos();
        let kv = k[0] * v[0] + k[1] * v[1] + k[2] * v[2];
        let cross = [
            k[1] * v[2] - k[2] * v[1],
            k[2] * v[0] - k[0] * v[2],
            k[0] * v[1] - k[1] * v[0],
        ];
        std::array::from_fn(|i| v[i] * c + cross[i] * s + k[i] * kv * (1.0 - c))
    };
    let place = |r: [f64; 3]| -> Latent {
        let mut u = [0.0; 8];
        u[Z_ALPHA] = r[0];
        u[Z_BETA] = r[1];
        u[x] += r[2];
        u
    };
    let (u1, u2) = (
        place(rotate([1.0, 0.0, 0.0])),
        place(rotate([0.0, 1.0, 0.0])),
    );
    let (sg, cg) = gamma.to_radians().sin_cos();
    (
        std::array::from_fn(|i| cg * u1[i] + sg * u2[i]),
        std::array::from_fn(|i| cg * u2[i] - sg * u1[i]),
    )
}

/// The mock's state with its slice frame `frame` and its basis `basis`, `z₀` the mock's.
fn sim_at(frame: SliceFrame, basis: (Latent, Latent)) -> SimConfig {
    let (mut sim, _) = crate::mock::state(0.0);
    sim.slice.frame = frame;
    (sim.plane.q1, sim.plane.q2) = basis;
    sim
}

/// A snapshot holding `sim`.
fn snapshot(sim: &SimConfig) -> Snapshot {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let mut snap = app.snapshot().clone();
    snap.sim = sim.clone();
    snap
}

/// `sim` after the edits of `commands`, written as the store writes them.
fn written(sim: &SimConfig, commands: &[Vec<SetField>]) -> SimConfig {
    let mut out = sim.clone();
    for edit in commands.iter().flatten() {
        match &edit.edit {
            Edit::Sim(SimField::Z0(z0)) => out.plane.z0 = *z0,
            Edit::Sim(SimField::Basis { q1, q2 }) => (out.plane.q1, out.plane.q2) = (*q1, *q2),
            Edit::Sim(SimField::SliceFrame { frame, basis }) => {
                out.slice.frame = *frame;
                if let Some((q1, q2)) = basis {
                    (out.plane.q1, out.plane.q2) = (*q1, *q2);
                }
            }
            Edit::Sim(SimField::Lock(lock)) => out.lock = lock.clone(),
            _ => {}
        }
    }
    out
}

/// The sim the tilt route leaves from `sim` at the angles `to`, as the Tilt section's sliders send it.
fn tilted_to(sim: &SimConfig, to: Angles) -> SimConfig {
    let from = View::read(sim);
    let edit = crate::dispatch::one(&sim.plane, crate::dispatch::Class::Tilt { from, to });
    let mut out = Out::default();
    out.send_edit(edit);
    written(sim, &out.sends(sim))
}

/// One target, `z_q3`, under `map`, untilted.
fn untilted(map: TiltMap) -> SimConfig {
    let frame = SliceFrame {
        tilt_map: map,
        ..SLICE_FRAME
    };
    sim_at(frame, view(0.0, 0.0, 0.0, Z_Q3, Z_Q3, map).basis())
}

// --- tilt_maps -----------------------------------------------------------------------------

/// Criterion: at `τ₁ = τ₂ = 0` both maps give `σ(e_a, e_b)` turned by `γ`, bitwise, for `γ ∈ {0, 30°, −120°}`; a single
/// tilt gives the same basis under both maps, bitwise (one code path), and the kernel's Tip as sheet on that tilt agrees
/// with Lean's closed form to 1e-15.
#[test]
fn tilt_maps_zero_is_the_identity_and_a_single_tilt_is_the_same() {
    for gamma in [0.0, 30.0, -120.0] {
        // The code's own sines: the platform's can differ from them in the last bit.
        let (sg, cg) = engine::exact::sin_cos(f64::to_radians(gamma));
        let mut want1 = [0.0; 8];
        let mut want2 = [0.0; 8];
        (want1[Z_ALPHA], want1[Z_BETA]) = (cg, sg);
        (want2[Z_ALPHA], want2[Z_BETA]) = (-sg, cg);
        for map in [TiltMap::Sheet, TiltMap::Lean] {
            assert_eq!(
                view(0.0, 0.0, gamma, Z_Q3, Z_Q3, map).basis(),
                (want1, want2)
            );
        }
    }
    for t in [-150.0, -90.0, -30.0, 0.5, 45.0, 120.0, 180.0] {
        for (t1, t2) in [(t, 0.0), (0.0, t)] {
            let sheet = view(t1, t2, 10.0, Z_Q3, Z_Q3, TiltMap::Sheet).basis();
            let lean = view(t1, t2, 10.0, Z_Q3, Z_Q3, TiltMap::Lean).basis();
            assert_eq!(
                sheet, lean,
                "a single tilt ({t1}, {t2}) is the same under both maps"
            );
            let frame = SliceFrame {
                tau1: t1,
                tau2: t2,
                ..SLICE_FRAME
            };
            let kernel = frame
                .latent_basis(0, 1, 1.0, f64::to_radians(10.0))
                .expect("targets off the axes");
            assert!(
                max_diff(kernel, sheet) <= 1e-15,
                "the kernel's sheet at ({t1}, {t2})"
            );
        }
    }
}

/// Criterion: with two targets both maps give the same basis bitwise at every `(τ₁, τ₂)` of a grid; with one target and
/// both angles set they differ by more than 0.01 in some component at each of those, and Tip as sheet is Rodrigues'
/// rotation written apart from the code, to 1e-12, wrap included. Controls: a basis that ignored the map, and Rodrigues
/// with `k` reversed, each fail.
#[test]
fn tilt_maps_differ_only_with_one_target_and_both_angles_set() {
    let grid = [-170.0, -95.0, -45.0, 20.0, 60.0, 89.0, 135.0, 179.0];
    let check = |basis: &dyn Fn(&View) -> (Latent, Latent)| {
        for &t1 in &grid {
            for &t2 in &grid {
                for gamma in [0.0, 37.0] {
                    let two_s = view(t1, t2, gamma, Z_Q3, Z_MU1, TiltMap::Sheet);
                    let two_l = view(t1, t2, gamma, Z_Q3, Z_MU1, TiltMap::Lean);
                    assert_eq!(basis(&two_s), basis(&two_l), "two targets at ({t1}, {t2})");
                    let one_s = view(t1, t2, gamma, Z_Q3, Z_Q3, TiltMap::Sheet);
                    let one_l = view(t1, t2, gamma, Z_Q3, Z_Q3, TiltMap::Lean);
                    assert!(
                        max_diff(basis(&one_s), basis(&one_l)) > 0.01,
                        "one target at ({t1}, {t2}): the maps must differ"
                    );
                    let want = rodrigues(t1, t2, gamma, Z_Q3);
                    assert!(
                        max_diff(basis(&one_s), want) <= 1e-12,
                        "Tip as sheet at ({t1}, {t2}, {gamma}) is not Rodrigues' rotation"
                    );
                }
            }
        }
    };
    check(&|v: &View| v.basis());
    rejects("a basis that ignores the map", || {
        check(&|v: &View| {
            View {
                map: TiltMap::Lean,
                ..*v
            }
            .basis()
        })
    });
    rejects("Rodrigues' axis reversed", || {
        check(&|v: &View| {
            if v.sheet() {
                let deg = |r: f64| r.to_degrees();
                rodrigues(-deg(v.tau1), -deg(v.tau2), deg(v.gamma), v.x)
            } else {
                v.basis()
            }
        })
    });
}

/// Criterion: under Tip as sheet `q₁`, `q₂` are orthonormal at every tilt of the grid, `|q₁| = |q₂| = 1` and `q₁·q₂ = 0`
/// to 1e-14, and at 45°/45° `q₁ = 0.722 e_α − 0.278 e_β + 0.634 e_q3` to 1e-3 (the contract's worked example). Control:
/// Lean each axis at 45°/45° is not orthonormal.
#[test]
fn tilt_maps_tip_as_sheet_is_orthonormal() {
    let check = |map: TiltMap| {
        for t1 in [-179.0, -90.0, -31.0, 45.0, 90.0, 150.0] {
            for t2 in [-120.0, -90.0, 45.0, 90.0, 177.0] {
                let (q1, q2) = view(t1, t2, 23.0, Z_Q3, Z_Q3, map).basis();
                let err = (dot(&q1, &q1) - 1.0)
                    .abs()
                    .max((dot(&q2, &q2) - 1.0).abs())
                    .max(dot(&q1, &q2).abs());
                assert!(
                    err <= 1e-14,
                    "({t1}, {t2}) under {map:?}: off orthonormal by {err:e}"
                );
            }
        }
    };
    check(TiltMap::Sheet);
    let (q1, _) = view(45.0, 45.0, 0.0, Z_Q3, Z_Q3, TiltMap::Sheet).basis();
    let want = [0.722, -0.278, 0.0, 0.0, 0.0, 0.634, 0.0, 0.0];
    assert!(
        q1.iter().zip(want).all(|(x, y)| (x - y).abs() <= 1e-3),
        "{q1:?}"
    );
    rejects("Lean each axis, skewed", || check(TiltMap::Lean));
}

/// Criterion: the Tilt section's map selector is two radio options, "Tip as sheet" and "Lean each axis", in the Tilt
/// sub-scope after `γ` and before the reset, Tip as sheet chosen at launch; both are drawn in the panel.
#[test]
fn tilt_maps_the_selector_follows_gamma() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let tree = app.keyboard.tree(engine::contract::view_ui::Mode::Explore);
    let children = tree.children(Some(mv::TILT));
    let want = [
        TAU1,
        TILT_X,
        TAU2,
        TILT_Y,
        GAMMA,
        TILT_MAP_SHEET,
        TILT_MAP_LEAN,
        RESET_TILT,
    ];
    assert_eq!(children, want);
    assert_eq!(app.snapshot().sim.slice.frame.tilt_map, TiltMap::Sheet);
    let texts = super::support::frame_texts(&mut h, &mut app);
    for name in ["Tip as sheet", "Lean each axis"] {
        assert!(texts.iter().any(|t| t == name), "{name} drawn: {texts:?}");
    }
    rejects("the reset before the radios", || {
        let mut order = want.to_vec();
        order.swap(5, 7);
        assert_eq!(children, order);
    });
}

// --- tilt_over_the_pole ----------------------------------------------------------------------------

/// Criterion: the basis from angles of total tip `m > 180°` is bitwise the basis from the wrap's own output
/// `τ·(m − 360)/m`, computed as the kernel computes it; the 180° partner `τ·(m − 180)/m` spans the same plane
/// with the normal reversed (mirrored), and differs from the basis by more than 0.5 in some component (never folded).
#[test]
fn tilt_over_the_pole_wraps_at_360_and_never_folds() {
    for (t1, t2) in [(150.0, 150.0), (170.0, -100.0), (-179.0, 60.0)] {
        let frame = |a: f64, b: f64| SliceFrame {
            tau1: a,
            tau2: b,
            ..SLICE_FRAME
        };
        let total = f64::sqrt(t1 * t1 + t2 * t2);
        assert!(total > 180.0);
        let wrap = (total - 360.0) / total;
        let basis = |f: SliceFrame| f.latent_basis(0, 1, 1.0, 0.0).expect("a basis");
        assert_eq!(
            basis(frame(t1, t2)),
            basis(frame(t1 * wrap, t2 * wrap)),
            "the wrap at ({t1}, {t2}) is one path"
        );
        let (p1, p2) = compass::partner(t1, t2).expect("a tip");
        let (q1, q2) = basis(frame(t1, t2));
        let (r1, r2) = basis(frame(p1, p2));
        assert!(
            same_plane((&q1, &q2), (&r1, &r2)),
            "the partner of ({t1}, {t2}) is the same plane"
        );
        let n = |a: &Latent, b: &Latent| a[0] * b[1] - a[1] * b[0];
        assert!(
            n(&q1, &q2) * n(&r1, &r2) < 0.0,
            "the partner's basis is mirrored"
        );
        assert!(
            max_diff((q1, q2), (r1, r2)) > 0.5,
            "never folded onto the partner"
        );
    }
}

/// Criterion: under Tip as sheet a drag of the tilt from 60°/60° to 70°/70° (the tip crossing 90°) moves the basis by
/// at most 0.01 per 0.1° step (continuous, never flipped); the sliders' span is ±180° under Tip as sheet and ±90° under
/// Lean each axis. Control: a basis folded onto its 180° partner past 90° jumps.
#[test]
fn tilt_over_the_pole_carries_on_past_ninety() {
    let sweep = |basis: &dyn Fn(f64) -> (Latent, Latent)| {
        let mut last = basis(60.0);
        for i in 1..=100 {
            let t = 60.0 + 0.1 * f64::from(i);
            let now = basis(t);
            assert!(max_diff(now, last) <= 0.01, "a jump at {t}°/{t}°");
            last = now;
        }
    };
    sweep(&|t| view(t, t, 0.0, Z_Q3, Z_Q3, TiltMap::Sheet).basis());
    rejects("folded onto the partner past 90°", || {
        sweep(&|t| {
            let m = t * std::f64::consts::SQRT_2;
            let (a, b) = if m > 90.0 {
                compass::partner(t, t).expect("a tip")
            } else {
                (t, t)
            };
            view(a, b, 0.0, Z_Q3, Z_Q3, TiltMap::Sheet).basis()
        })
    });
    let mut a = Angles {
        tau1: 89.5,
        tau2: 0.0,
        gamma: 0.0,
    };
    a.turn(TAU1, 1.0, TiltMap::Sheet);
    assert_eq!(a.tau1, 90.5);
    a.turn(TAU1, 500.0, TiltMap::Sheet);
    assert_eq!(a.tau1, TILT_RANGE.1);
    assert_eq!(TILT_RANGE, (-180.0, 180.0));
    a.turn(TAU1, 0.0, TiltMap::Lean);
    assert_eq!(a.tau1, LEAN_RANGE.1);
}

// --- same_plane_readout ------------------------------------------------------------------------------

/// Criterion: past 90° of tip under Tip as sheet the readout says "same plane as (τ₁′, τ₂′), mirrored", τ′ = τ·(m −
/// 180)/m to one decimal (70°/70°: m = 98.99°, τ′ = −57.3°), the partner's basis spanning the same plane; no such line
/// at 60°/60° or under Lean each axis. Drawing frames at that view writes nothing back: the stored angles, the basis and
/// the undo depth are as they were. Control: a state whose angles were replaced by the partner's fails the last check.
#[test]
fn same_plane_readout_names_the_mirrored_partner_and_writes_nothing() {
    let past = view(70.0, 70.0, 0.0, Z_Q3, Z_Q3, TiltMap::Sheet);
    assert_eq!(
        mirrored_line(&past).as_deref(),
        Some("same plane as (-57.3°, -57.3°), mirrored")
    );
    let (p1, p2) = compass::partner(70.0, 70.0).expect("a tip");
    let (q1, q2) = past.basis();
    let (r1, r2) = view(p1, p2, 0.0, Z_Q3, Z_Q3, TiltMap::Sheet).basis();
    assert!(same_plane((&q1, &q2), (&r1, &r2)));
    assert_eq!(
        mirrored_line(&view(60.0, 60.0, 0.0, Z_Q3, Z_Q3, TiltMap::Sheet)),
        None
    );
    assert_eq!(
        mirrored_line(&view(70.0, 70.0, 0.0, Z_Q3, Z_Q3, TiltMap::Lean)),
        None
    );
    let axes = compass::axes_lines(&past, &q1, &q2);
    assert!(
        axes.iter().any(|l| l.starts_with("same plane as")),
        "{axes:?}"
    );
    rejects("a different plane read as the same", || {
        let (o1, o2) = view(40.0, 70.0, 0.0, Z_Q3, Z_Q3, TiltMap::Sheet).basis();
        assert!(same_plane((&q1, &q2), (&o1, &o2)));
    });

    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let sim = tilted_to(
        &app.snapshot().sim,
        Angles {
            tau1: 70.0,
            tau2: 70.0,
            gamma: 0.0,
        },
    );
    app.set_field(mv::frame_edit(
        sim.slice.frame,
        Some((sim.plane.q1, sim.plane.q2)),
    ));
    let _ = h.frame(&mut app, Vec::new());
    let before = app.snapshot().clone();
    for _ in 0..5 {
        let _ = h.frame(&mut app, Vec::new());
    }
    let after = app.snapshot();
    let check = |a: &Snapshot| {
        assert_eq!(
            a.sim.slice.frame, before.sim.slice.frame,
            "the stored angles"
        );
        assert_eq!(a.sim.plane, before.sim.plane, "the basis");
        assert_eq!(
            a.history.undo_depth, before.history.undo_depth,
            "no history entry"
        );
    };
    check(after);
    rejects("the partner written back", || {
        let mut wrote = after.clone();
        wrote.sim.slice.frame.tau1 = p1;
        wrote.sim.slice.frame.tau2 = p2;
        check(&wrote);
    });
}

// --- tilt_map_state -------------------------------------------------------------------------------------

/// Criterion: `tilt_map` is the slice frame's, Tip as sheet by default, serialised `"tilt_map":"sheet"`, a record with
/// no such field read as Lean each axis, a saved record restoring bitwise; choosing a map is one history
/// entry, one Ctrl+Z restoring map, angles and basis together; choosing Lean each axis at τ₁ = 150° clamps it to 90° in
/// that entry, sign kept (K15), and a switch back leaves it; the Tilt reset and Home leave the map (K7).
#[test]
fn tilt_map_state_is_recorded_and_one_entry() {
    let (sim, _) = crate::mock::state(0.0);
    assert_eq!(sim.slice.frame.tilt_map, TiltMap::Sheet);
    let text = view_state::export(&sim).expect("exports");
    assert!(text.contains("\"tilt_map\":\"sheet\""), "{text}");
    let old = text
        .replace(",\"tilt_map\":\"sheet\"", "")
        .replace("\"tau1\":0,\"tau2\":0,", "");
    assert!(!old.contains("tilt_map") && !old.contains("tau1"), "{old}");
    let read = view_state::restore(&old).expect("an older record restores");
    assert_eq!(
        read.slice.frame.tilt_map,
        TiltMap::Lean,
        "absent, Lean each axis"
    );
    assert_eq!((read.slice.frame.tau1, read.slice.frame.tau2), (0.0, 0.0));

    let at = tilted_to(
        &untilted(TiltMap::Sheet),
        Angles {
            tau1: 150.0,
            tau2: -20.0,
            gamma: 5.0,
        },
    );
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    app.set_field(mv::frame_edit(
        at.slice.frame,
        Some((at.plane.q1, at.plane.q2)),
    ));
    let _ = h.frame(&mut app, Vec::new());
    let before = app.snapshot().clone();
    let edit = map_edit(&before.sim, TiltMap::Lean).expect("a change of map");
    app.set_field(edit);
    let _ = h.frame(&mut app, Vec::new());
    let lean = app.snapshot().clone();
    assert_eq!(
        lean.history.undo_depth,
        before.history.undo_depth + 1,
        "one entry"
    );
    assert_eq!(lean.sim.slice.frame.tilt_map, TiltMap::Lean);
    assert_eq!(
        (lean.sim.slice.frame.tau1, lean.sim.slice.frame.tau2),
        (90.0, -20.0),
        "clamped, sign kept"
    );
    assert_ne!(lean.sim.plane, before.sim.plane, "the basis re-sent");
    let back = map_edit(&lean.sim, TiltMap::Sheet).expect("a change of map");
    let back = written(&lean.sim, &[vec![back]]);
    assert_eq!(
        (back.slice.frame.tau1, back.slice.frame.tau2),
        (90.0, -20.0),
        "a switch back leaves them"
    );
    super::support::press(
        &mut h,
        &mut app,
        eframe::egui::Key::Z,
        eframe::egui::Modifiers::CTRL,
    );
    let _ = h.frame(&mut app, Vec::new());
    let undone = app.snapshot();
    assert_eq!(
        undone.sim.slice.frame, before.sim.slice.frame,
        "map and angles undone"
    );
    assert_eq!(
        undone.sim.plane, before.sim.plane,
        "the basis undone with them"
    );

    for command in [
        reset::command(&before.sim, reset::Section::Tilt),
        reset::home(&before.sim),
    ] {
        let after = written(&before.sim, &[command]);
        assert_eq!(
            after.slice.frame.tilt_map,
            TiltMap::Sheet,
            "the resets leave the map"
        );
    }
    rejects("a map change that clamps nothing", || {
        let unclamped = SliceFrame {
            tilt_map: TiltMap::Lean,
            ..before.sim.slice.frame
        };
        assert_eq!((unclamped.tau1, unclamped.tau2), (90.0, -20.0));
    });
}

// --- compass_active_map -------------------------------------------------------------------------------

/// Criterion: the cube draws `View::basis` under the active map: at 45°/45° one target its normal is
/// `∝ (−0.634, −0.634, 0.444)` under Tip as sheet and `∝ (−½, −½, ½)` under Lean each axis, each to 1e-3; at the corner
/// 90°/90° Tip as sheet draws a plane and Lean each axis none (B11 under Lean only). Control: the normal of the other
/// map fails.
#[test]
fn compass_active_map_draws_the_map_chosen() {
    let unit = |v: [f64; 3]| {
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        v.map(|x| x / n)
    };
    let m = f64::to_radians(45.0 * std::f64::consts::SQRT_2);
    let s = m.sin() / std::f64::consts::SQRT_2;
    let sheet = unit([-s, -s, m.cos()]);
    let lean = unit([-0.5, -0.5, 0.5]);
    let check = |map: TiltMap, want: [f64; 3]| {
        let n = drawn_normal(&view(45.0, 45.0, 0.0, Z_Q3, Z_Q3, map)).expect("a plane");
        assert!(
            n.iter().zip(want).all(|(x, y)| (x - y).abs() <= 1e-3),
            "{map:?}: {n:?}, not {want:?}"
        );
    };
    check(TiltMap::Sheet, sheet);
    check(TiltMap::Lean, lean);
    assert!(
        (sheet[0] + 0.634).abs() <= 1e-3 && (sheet[2] - 0.444).abs() <= 1e-3,
        "{sheet:?}"
    );
    assert!(drawn_normal(&view(90.0, 90.0, 0.0, Z_Q3, Z_Q3, TiltMap::Sheet)).is_some());
    assert!(drawn_normal(&view(90.0, 90.0, 0.0, Z_Q3, Z_Q3, TiltMap::Lean)).is_none());
    rejects("the cube drawing Lean each axis under Tip as sheet", || {
        check(TiltMap::Sheet, lean)
    });
}

// --- lean_warnings -----------------------------------------------------------------------------------

/// Criterion: under Lean each axis with one target the readout adds "dead zone: |τ₁| = 90°, τ₂ does not move the plane"
/// at 90°/30° (indices swapped at 30°/−90°), "axes at 60.0°" at 45°/45° (cos θ = sin 45° sin 45°), B11's line citing
/// "(Lean each axis)" at the corner; none of these under Tip as sheet or with two targets. Control: the warnings asked of
/// Tip as sheet fail.
#[test]
fn lean_warnings_only_under_lean_each_axis() {
    let lean = |t1, t2| view(t1, t2, 0.0, Z_Q3, Z_Q3, TiltMap::Lean);
    assert_eq!(
        lean_warnings(&lean(90.0, 30.0)),
        vec![
            "dead zone: |τ₁| = 90°, τ₂ does not move the plane",
            "axes at 60.0°"
        ]
    );
    assert_eq!(
        lean_warnings(&lean(30.0, -90.0)),
        vec![
            "dead zone: |τ₂| = 90°, τ₁ does not move the plane",
            "axes at 120.0°"
        ]
    );
    assert_eq!(lean_warnings(&lean(45.0, 45.0)), vec!["axes at 60.0°"]);
    assert!(lean_warnings(&lean(30.0, 0.0)).is_empty());
    let corner = compass::readout(&[0.0; 8], &lean(90.0, 90.0), Touch::Tilt);
    assert!(
        corner
            .iter()
            .any(|l| l.starts_with("no plane drawn") && l.ends_with("(Lean each axis)")),
        "{corner:?}"
    );
    let check_none = |v: View| {
        assert!(lean_warnings(&v).is_empty(), "{v:?}");
        let lines = compass::readout(&[0.0; 8], &v, Touch::Tilt);
        assert!(
            !lines.iter().any(|l| l.starts_with("no plane drawn")),
            "{lines:?}"
        );
    };
    check_none(view(90.0, 90.0, 0.0, Z_Q3, Z_Q3, TiltMap::Sheet));
    check_none(view(90.0, 30.0, 0.0, Z_Q3, Z_MU1, TiltMap::Lean));
    rejects("Lean's warnings asked of Tip as sheet", || {
        assert_eq!(
            lean_warnings(&view(45.0, 45.0, 0.0, Z_Q3, Z_Q3, TiltMap::Sheet)),
            vec!["axes at 60.0°"]
        );
    });
}

// --- readout_components ------------------------------------------------------------------------------

/// The components of `v`'s basis.
fn components_of(v: &View) -> [String; 2] {
    let (q1, q2) = v.basis();
    components(&q1, &q2)
}

/// Criterion: the components read the stored basis, the decoder's, not the view's reading of it. Under Tip as sheet a
/// basis typed as Lean each axis's at 20°/15° on one target is no rigid turn, so no angles under the map give it; the
/// compass column shows "H = 0.94 z_α + 0.34 z_q3" and "V = 0.97 z_β + 0.26 z_q3", its own unit axes. Control: the
/// components of the view's basis, which the panel reads, are not these.
#[test]
fn readout_components_are_the_stored_basis() {
    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let (t1, t2) = (20f64.to_radians(), 15f64.to_radians());
    let (mut q1, mut q2) = ([0.0; 8], [0.0; 8]);
    (q1[Z_ALPHA], q1[Z_Q3]) = (t1.cos(), t1.sin());
    (q2[Z_BETA], q2[Z_Q3]) = (t2.cos(), t2.sin());
    app.set_field(mv::basis_edit((q1, q2)));
    let _ = h.frame(&mut app, Vec::new());
    let sim = app.snapshot().sim.clone();
    assert_eq!(sim.slice.frame.tilt_map, TiltMap::Sheet, "the default map");
    let want = ["H = 0.94 z_α + 0.34 z_q3", "V = 0.97 z_β + 0.26 z_q3"];
    let axes = super::compass_support::drawing(&h, &app).axes;
    assert_eq!(axes[axes.len() - 2..], want);
    let read = View::read(&sim);
    rejects("the components of the view's reading", || {
        assert_eq!(components_of(&read), want);
    });
}

/// Criterion: under either map the compass column's last lines (under the hint) are each screen axis's unit components
/// after `γ`, two decimals,
/// terms below 0.005 omitted, named as the cube names them: at 45°/45° one target Tip as sheet reads
/// "H = 0.72 z_α − 0.28 z_β + 0.63 z_q3" and "V = −0.28 z_α + 0.72 z_β + 0.63 z_q3", Lean each axis
/// "H = 0.71 z_α + 0.71 z_q3", "V = 0.71 z_β + 0.71 z_q3"; untilted "H = 1.00 z_α", "V = 1.00 z_β". Control: the other
/// map's lines fail. On the mock's window the app draws them: the default view's "H = 1.00 z_α" and "V = 1.00 z_β" are
/// in the frame's text; control: a line no view reads is not.
#[test]
fn readout_components_show_each_axis() {
    let sheet = view(45.0, 45.0, 0.0, Z_Q3, Z_Q3, TiltMap::Sheet);
    let want_sheet = [
        "H = 0.72 z_α − 0.28 z_β + 0.63 z_q3",
        "V = −0.28 z_α + 0.72 z_β + 0.63 z_q3",
    ];
    assert_eq!(components_of(&sheet), want_sheet);
    let lean = View {
        map: TiltMap::Lean,
        ..sheet
    };
    assert_eq!(
        components_of(&lean),
        ["H = 0.71 z_α + 0.71 z_q3", "V = 0.71 z_β + 0.71 z_q3"]
    );
    assert_eq!(
        components_of(&view(0.0, 0.0, 0.0, Z_Q3, Z_Q3, TiltMap::Sheet)),
        ["H = 1.00 z_α", "V = 1.00 z_β"]
    );
    let (q1, q2) = sheet.basis();
    let lines = compass::axes_lines(&sheet, &q1, &q2);
    assert_eq!(lines[lines.len() - 2..], want_sheet);
    rejects("Lean's components under Tip as sheet", || {
        assert_eq!(components_of(&lean), want_sheet);
    });

    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    let output = h.frame(&mut app, Vec::new());
    let drawn = |wanted: &str| {
        output.shapes.iter().any(|c| match &c.shape {
            eframe::egui::Shape::Text(t) => t.galley.text() == wanted,
            _ => false,
        })
    };
    for line in ["H = 1.00 z_α", "V = 1.00 z_β"] {
        assert!(drawn(line), "{line} drawn");
    }
    rejects("a line no view reads drawn", || {
        assert!(drawn("H = 0.72 z_α − 0.28 z_β + 0.63 z_q3"));
    });
}

// --- reintegration_scope --------------------------------------------------------------------------

/// Criterion: a change of map re-sends the basis, so re-integrates, exactly on the views with `X = Y` and both angles
/// set, and where Lean each axis's clamp moves an angle (K15); every other view's plane stays bit for bit and the mock
/// integrates nothing anew. Measured on the mock: after showing span A then span B, a map change and span B
/// again leave A's quads held exactly where nothing re-integrates. Control: a mock that keyed on the slice frame
/// re-integrates on every switch and fails.
#[test]
fn reintegration_scope_is_exactly_the_views_the_maps_differ_on() {
    use engine::contract::interface::EngineInterface;
    let angles = [0.0, 30.0, -60.0, 150.0];
    let mut changed = 0;
    let mut kept = 0;
    for x_y in [(Z_Q3, Z_Q3), (Z_Q3, Z_MU1)] {
        for &t1 in &angles {
            for &t2 in &angles {
                let frame = SliceFrame {
                    tilt_x: x_y.0 as u32,
                    tilt_y: x_y.1 as u32,
                    ..SLICE_FRAME
                };
                let start = sim_at(
                    frame,
                    view(0.0, 0.0, 0.0, x_y.0, x_y.1, TiltMap::Sheet).basis(),
                );
                let sim = tilted_to(
                    &start,
                    Angles {
                        tau1: t1,
                        tau2: t2,
                        gamma: 0.0,
                    },
                );
                assert_eq!((sim.slice.frame.tau1, sim.slice.frame.tau2), (t1, t2));
                let edit = map_edit(&sim, TiltMap::Lean).expect("a change");
                let after = written(&sim, &[vec![edit.clone()]]);
                let differ = x_y.0 == x_y.1 && t1 != 0.0 && t2 != 0.0;
                let clamps = t1.abs() > 90.0 || t2.abs() > 90.0;
                let moved = after.plane != sim.plane;
                assert_eq!(moved, differ || clamps, "({t1}, {t2}) into {x_y:?}");
                if !moved {
                    assert!(matches!(
                        edit.edit,
                        Edit::Sim(SimField::SliceFrame { basis: None, .. })
                    ));
                    let bits = |p: &Plane| [p.q1, p.q2].map(|q| q.map(f64::to_bits));
                    assert_eq!(bits(&after.plane), bits(&sim.plane));
                }
                let mut mock = crate::mock::MockEngine::frozen();
                let span_a = [0.0, 0.0, 0.5, 0.5];
                let span_b = [0.5, 0.5, 1.0, 1.0];
                mock.set_field(mv::frame_edit(
                    sim.slice.frame,
                    Some((sim.plane.q1, sim.plane.q2)),
                ));
                mock.show_span(span_a);
                mock.show_span(span_b);
                let held = mock.integrated().len();
                mock.set_field(edit);
                mock.show_span(span_b);
                let anew = mock.integrated().len() != held;
                assert_eq!(
                    anew, moved,
                    "the mock re-integrates ({t1}, {t2}) into {x_y:?}"
                );
                if moved {
                    changed += 1;
                } else {
                    kept += 1;
                }
            }
        }
    }
    assert!(
        changed > 0 && kept > 0,
        "both kinds measured: {changed} changed, {kept} kept"
    );
    rejects("a mock keyed on the slice frame", || {
        let sim = tilted_to(
            &untilted(TiltMap::Sheet),
            Angles {
                tau1: 30.0,
                tau2: 0.0,
                gamma: 0.0,
            },
        );
        let edit = map_edit(&sim, TiltMap::Lean).expect("a change");
        let after = written(&sim, &[vec![edit]]);
        assert_eq!(after != sim, after.plane != sim.plane);
    });
}

// --- tilt_angles_stored ------------------------------------------------------------------------------

/// Criterion: under Tip as sheet at τ = (150°, 150°) the stored angles read back (150°, 150°) bitwise, not the wrapped
/// (−104.6°, −104.6°) the basis gives; a +1° step on τ₁ gives (151°, 150°) bitwise; one undo restores basis and angles
/// together; a saved view and an export carry the angles bit for bit. Controls: angles read back from the basis, and an
/// undo restoring the basis alone, each fail.
#[test]
fn tilt_angles_stored_as_set() {
    let sim = tilted_to(
        &untilted(TiltMap::Sheet),
        Angles {
            tau1: 150.0,
            tau2: 150.0,
            gamma: 0.0,
        },
    );
    let check = |a: Angles| assert_eq!((a.tau1, a.tau2), (150.0, 150.0));
    check(Angles::read(&sim));
    assert_eq!(degrees_of(&View::read(&sim)), (150.0, 150.0));
    let from_basis = {
        let mut lost = sim.clone();
        (lost.slice.frame.tau1, lost.slice.frame.tau2) = (0.0, 0.0);
        Angles::read(&lost)
    };
    let w = 150.0 * (150.0 * std::f64::consts::SQRT_2 - 360.0) / (150.0 * std::f64::consts::SQRT_2);
    assert!(
        (from_basis.tau1 - w).abs() <= 1e-9 && (from_basis.tau1 + 104.6).abs() < 0.05,
        "{from_basis:?}"
    );
    rejects("the angles read back from the basis", || check(from_basis));

    let snap = snapshot(&sim);
    let mut scratch = Scratch::default();
    let mut out = Out::default();
    mv::adjust(
        &Adjust {
            scope: TAU1,
            kind: StepKind::Angle,
            times: 1.0,
            direction: Direction::Right,
        },
        &snap,
        &mut scratch,
        &mut out,
    );
    let stepped = written(&sim, &out.sends(&sim));
    assert_eq!(
        (
            stepped.slice.frame.tau1.to_bits(),
            stepped.slice.frame.tau2.to_bits()
        ),
        (151.0f64.to_bits(), 150.0f64.to_bits())
    );
    let a = Angles::read(&stepped);
    assert_eq!((a.tau1, a.tau2), (151.0, 150.0));

    let mut app = mock_app();
    let mut h = headless();
    let _ = h.frame(&mut app, Vec::new());
    app.set_field(mv::frame_edit(
        sim.slice.frame,
        Some((sim.plane.q1, sim.plane.q2)),
    ));
    let _ = h.frame(&mut app, Vec::new());
    let before = app.snapshot().sim.clone();
    for edit in out.sends(&before).into_iter().flatten() {
        app.set_field(edit);
    }
    let _ = h.frame(&mut app, Vec::new());
    assert_eq!(app.snapshot().sim.slice.frame.tau1, 151.0);
    super::support::press(
        &mut h,
        &mut app,
        eframe::egui::Key::Z,
        eframe::egui::Modifiers::CTRL,
    );
    let _ = h.frame(&mut app, Vec::new());
    let undone = |s: &SimConfig| {
        assert_eq!(s.plane, before.plane, "the basis undone");
        assert_eq!(
            s.slice.frame, before.slice.frame,
            "the angles undone with it"
        );
    };
    undone(&app.snapshot().sim);
    rejects("an undo restoring the basis alone", || {
        let mut half = app.snapshot().sim.clone();
        half.slice.frame = stepped.slice.frame;
        undone(&half);
    });

    let record = view_state::export(&stepped).expect("exports");
    let back = view_state::restore(&record).expect("restores");
    assert_eq!(back.slice.frame.tau1.to_bits(), 151.0f64.to_bits());
    assert_eq!(back.slice.frame.tau2.to_bits(), 150.0f64.to_bits());
    assert_eq!(
        view_state::export(&back).expect("exports"),
        record,
        "bit for bit"
    );
}

fn degrees_of(v: &View) -> (f64, f64) {
    (mv::degrees(v.tau1), mv::degrees(v.tau2))
}

/// The sim after `route` on `sim`, each of the routes that write the basis other than through the angles, as the app
/// sends them.
fn after_route(sim: &SimConfig, route: &str) -> SimConfig {
    let snap = snapshot(sim);
    let mut scratch = Scratch::default();
    let mut out = Out::default();
    match route {
        "zoom" => out.send_edit(mv::navigate::zoom_by(&sim.plane, 0.5)),
        "typed q₁ component" => mv::adjust(
            &Adjust {
                scope: mv::Q1_COMPONENTS[Z_MU1],
                kind: StepKind::Bounded,
                times: 1.0,
                direction: Direction::Right,
            },
            &snap,
            &mut scratch,
            &mut out,
        ),
        "preset" => mv::activate(mv::PRESET, &snap, &mut scratch, &mut out),
        "Tilt reset" => return written(sim, &[reset::command(sim, reset::Section::Tilt)]),
        "Home" => return written(sim, &[reset::home(sim)]),
        "target pick" => mv::activate(mv::TILT_Y_ENTRIES[Z_MU1], &snap, &mut scratch, &mut out),
        "map" => mv::activate(TILT_MAP_LEAN, &snap, &mut scratch, &mut out),
        _ => unreachable!("{route}"),
    }
    written(sim, &out.sends(sim))
}

/// Whether the stored angles and the basis agree: the angles read are the stored ones, bit for bit, and where the basis
/// is any view's, the view at them gives it.
fn agree(sim: &SimConfig) -> Result<(), String> {
    let read = Angles::read(sim);
    let stored = (sim.slice.frame.tau1, sim.slice.frame.tau2);
    if (read.tau1.to_bits(), read.tau2.to_bits()) != (stored.0.to_bits(), stored.1.to_bits()) {
        return Err(format!("stored {stored:?}, read {read:?}"));
    }
    Ok(())
}

/// Criterion: from Tip as sheet at (150°, 150°), each route that writes the basis other than through the angles (zoom, a
/// typed `q₁` component, the preset, the Tilt reset, Home, a target pick, the map) leaves the stored angles those the
/// basis reads, bit for bit; the Tilt reset and Home leave (0°, 0°). Control: the bare basis edit of the Tilt reset,
/// written without the rule, leaves (150°, 150°) beside an untilted basis and fails.
#[test]
fn tilt_angles_stored_each_route_keeps_them_agreeing() {
    let sim = tilted_to(
        &untilted(TiltMap::Sheet),
        Angles {
            tau1: 150.0,
            tau2: 150.0,
            gamma: 0.0,
        },
    );
    agree(&sim).expect("the tilt route agrees");
    for route in [
        "zoom",
        "typed q₁ component",
        "preset",
        "Tilt reset",
        "Home",
        "target pick",
        "map",
    ] {
        let after = after_route(&sim, route);
        agree(&after).unwrap_or_else(|e| panic!("{route}: {e}"));
        if matches!(route, "Tilt reset" | "Home") {
            assert_eq!(
                (after.slice.frame.tau1, after.slice.frame.tau2),
                (0.0, 0.0),
                "{route}"
            );
        }
        if route == "zoom" {
            assert_eq!(
                (after.slice.frame.tau1, after.slice.frame.tau2),
                (150.0, 150.0),
                "a zoom keeps them"
            );
        }
    }
    rejects("the reset's basis written without the rule", || {
        let view = View::read(&sim);
        let flat = View {
            tau1: 0.0,
            tau2: 0.0,
            gamma: 0.0,
            ..view
        }
        .basis();
        let bare = written(&sim, &[vec![mv::basis_edit(flat)]]);
        agree(&bare).expect("agrees");
    });
}

/// Criterion: locked off the centre, a tilt under each map keeps the pin's latent point where it was, to 1e-12: the turn
/// about the pin holds under Tip as sheet as under Lean each axis (render_gui_spec §G4). Control: the same tilt sent
/// without its turn about the pin moves it.
#[test]
fn tilt_maps_turn_about_the_pin_under_both() {
    for map in [TiltMap::Sheet, TiltMap::Lean] {
        let mut sim = tilted_to(
            &untilted(map),
            Angles {
                tau1: 20.0,
                tau2: 35.0,
                gamma: 0.0,
            },
        );
        let pin = mv::point(&sim.plane, 0.3, 0.7);
        sim.lock = engine::contract::sim_config::Lock {
            locked: true,
            z_locked: pin,
        };
        let check = |turn: bool| {
            let from = View::read(&sim);
            let to = Angles {
                tau1: 60.0,
                tau2: 50.0,
                gamma: 10.0,
            };
            let edit = crate::dispatch::one(&sim.plane, crate::dispatch::Class::Tilt { from, to });
            let mut out = Out::default();
            if turn {
                out.send_edit(edit);
            } else {
                out.send_command(vec![edit]);
            }
            let after = written(&sim, &out.sends(&sim));
            let (s, t) = mv::chart_coords(&sim.plane, &pin).expect("a chart point");
            let at = mv::point(&after.plane, s, t);
            let off = at
                .iter()
                .zip(&pin)
                .map(|(x, y)| (x - y).abs())
                .fold(0.0, f64::max);
            assert!(off <= 1e-12, "{map:?}: the pin moved {off:e}");
            assert!(
                max_diff(
                    (after.plane.q1, after.plane.q2),
                    (sim.plane.q1, sim.plane.q2)
                ) > 0.1
            );
        };
        check(true);
        rejects("a tilt not turned about the pin", || check(false));
    }
}
