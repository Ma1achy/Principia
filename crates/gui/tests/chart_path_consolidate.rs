//! The gui's navigation routes as thin adapters onto the engine's leaves (TASK-M2-64; R-894, R-919 F1 (a)). Run on the
//! Mac, which has a display; written in the cloud session, where only `cargo check` and `cargo clippy` ran.
//!
//! **`path_adapter_same_leaf`.** Each route's output, the drag's pan, the wheel's zoom, the zoom row, the lock at a
//! point, the compass's tilt step and tilt drag, equals the engine leaf's edits for the same input, built here from
//! `engine::path::leaf` directly, bit for bit (compared by `Debug`, which writes every f64 exactly, `−0` included).
//!
//! **`path_merge_bitwise`.** The routes whose formula was the leaf's already, the drag's pan, the wheel's zoom, the zoom
//! row (`q·(σ′/σ)`, the zoom leaf's factor form) and the lock at a point, against their formulas as they stood on
//! `main`, frozen here: bit for bit.
//!
//! **`path_canonical_conversion`.** The compass's tilt against its old formula, within `8ε·σ` per component
//! (`crates/engine/tests/chart_path_consolidate.rs` derives the bound and measures it on 4,000 tilts).
//!
//! **`path_slice_not_merged`.** The compass's slice, measured not rounding from the slice leaf's, keeps its own bits.
//!
//! **`path_pin_kept`.** Locked off the centre, the compass's tilt and the wheel's zoom, turned about the pin as the app
//! turns them (`about_the_pin`), keep the anchor at its chart point within 10⁻¹² (`mock_manifold_view`'s criterion).
//!
//! **Controls** (`path_control_*`): an adapter holding a formula of its own (the basis rebuilt from the view) is not the
//! leaf's; a pan with one operation reordered is not the old pan; the slice leaf put in the compass's place is not its
//! slice; the edits not turned about the pin move the anchor.

use eframe::egui;
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, Links, Lock, Plane, Quality,
    SimConfig, Slice, SliceFrame, Window, Z_Q3,
};
use engine::path::leaf::{self, Affine, CentreOnPin, OnLatent, Scale};
use engine::path::{stop, LatentTilt, TiltMap, TiltSpec, View as PathView};
use gui::explore::compass;
use gui::explore::manifold_view::tilt::Angles;
use gui::explore::manifold_view::{
    self as mv, lock as mv_lock, navigate, Out, Scratch, Touch, View,
};

// ── Fixtures ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// The affine view of `plane` the leaves edit, as `gui::dispatch` builds it.
fn affine(plane: &Plane) -> PathView {
    PathView::latent(SimConfig {
        chart: Chart {
            window: Window::SQUARE,
            mapping: Default::default(),
        },
        plane: plane.clone(),
        slice: Slice {
            frame: SliceFrame::along(Z_Q3),
        },
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links::defaults(),
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 1.0 },
        collision: Collision {},
        quality: Quality {},
    })
}

fn axis(i: usize, k: f64) -> Latent {
    let mut v = [0.0; 8];
    v[i] = k;
    v
}

/// The planes: the mock's home, a zoomed preset, a tilted one, one tilted into two targets, and a skew basis.
fn planes() -> Vec<(Plane, SliceFrame)> {
    let home = Plane {
        z0: gui::mock::MOCK_Z0,
        q1: gui::mock::MOCK_Q1,
        q2: gui::mock::MOCK_Q2,
    };
    let tilted = |x: u32, y: u32, tau: [f64; 2], scale: f64| {
        let frame = SliceFrame {
            normal: x,
            tilt_x: x,
            tilt_y: y,
        };
        let view = View::framed(
            &Plane {
                z0: home.z0,
                q1: axis(2, scale),
                q2: axis(3, scale),
            },
            Some(&frame),
        );
        let (q1, q2) = Angles {
            tau1: tau[0],
            tau2: tau[1],
            gamma: 12.5,
        }
        .apply(view)
        .basis();
        (
            Plane {
                z0: home.z0,
                q1,
                q2,
            },
            frame,
        )
    };
    vec![
        (home.clone(), SliceFrame::along(Z_Q3)),
        (
            Plane {
                z0: [0.1, -0.0, 0.25, -0.5, 0.0, 0.125, -0.0, 0.3],
                q1: axis(0, 0.0625),
                q2: axis(1, 0.0625),
            },
            SliceFrame::along(Z_Q3),
        ),
        tilted(5, 5, [23.4, -11.0], 0.5),
        tilted(5, 7, [-40.0, 61.25], 0.25),
        (
            Plane {
                z0: [0.3, -0.0, 0.1, -0.2, 0.05, 0.0, 0.4, -0.25],
                q1: [0.7, 0.2, -0.0, 0.1, 0.0, 0.0, 0.05, 0.0],
                q2: [0.037, 0.333, 0.111, -0.074, 0.0, 0.037, 0.0, -0.0],
            },
            SliceFrame::along(Z_Q3),
        ),
    ]
}

/// Every f64 written exactly: two edit lists are bit for bit the same where their `Debug` texts are.
fn exact(edits: &[SetField]) -> String {
    format!("{edits:?}")
}

fn leaf_edits(l: Result<leaf::Leaf, engine::path::PathError>) -> Vec<SetField> {
    l.expect("the leaf takes it").edits().to_vec()
}

/// The engine's tilt leaf from `view`, read in its frame, to `to`.
fn tilt_leaf(plane: &Plane, view: &View, to: Angles) -> Vec<SetField> {
    let at = Angles::of(view);
    let on = OnLatent {
        from: LatentTilt {
            a: view.a as u32,
            b: view.b as u32,
            scale: view.scale,
            from: [at.tau1, at.tau2, at.gamma],
        },
        gamma: to.gamma,
    };
    let spec = TiltSpec {
        tau1: to.tau1,
        tau2: to.tau2,
        map: TiltMap::Lean,
        x: view.x as u32,
        y: view.y as u32,
    };
    leaf_edits(leaf::tilt(&affine(plane), &on, &spec, &stop::affine()))
}

/// The drags: `(Δx, Δy)` over figures `(w, h)`.
const DRAGS: [([f32; 2], [f32; 2]); 4] = [
    ([10.0, 0.0], [100.0, 50.0]),
    ([-37.5, 12.25], [640.0, 480.0]),
    ([3.0, -211.0], [333.0, 777.0]),
    ([0.0, 0.0], [100.0, 100.0]),
];

// ── path_adapter_same_leaf ────────────────────────────────────────────────────────────────────────────────────────

/// The zoom row's adapter: the route, or the control's adapter holding a formula of its own, the basis rebuilt from the
/// view read at the new scale.
type ZoomRow = fn(&Plane, View, View) -> SetField;

/// `navigate::zoom_to` as it stood: the basis times `σ′/σ`.
fn old_zoom_row(plane: &Plane, view: View, to: View) -> SetField {
    let k = to.scale / view.scale;
    mv::basis_edit((plane.q1.map(|x| x * k), plane.q2.map(|x| x * k)))
}

/// An adapter holding its own formula: the basis rebuilt from the view at the new scale (`View::basis`).
fn rebuilt_zoom_row(_plane: &Plane, _view: View, to: View) -> SetField {
    mv::basis_edit(to.basis())
}

fn check_same_leaf(zoom_row: ZoomRow) {
    let gate = stop::affine();
    for (plane, frame) in planes() {
        let view = View::framed(&plane, Some(&frame));
        for (d, size) in DRAGS {
            let (delta, size) = (egui::vec2(d[0], d[1]), egui::vec2(size[0], size[1]));
            let ds = -f64::from(delta.x) / f64::from(size.x);
            let dt = f64::from(delta.y) / f64::from(size.y);
            assert_eq!(
                exact(&[navigate::pan(&plane, delta, size)]),
                exact(&leaf_edits(leaf::pan(&affine(&plane), ds, dt, &gate))),
                "the drag's pan is not the pan leaf's"
            );
        }
        for octaves in [0.125, -1.0 / 64.0, 1.0, -3.0] {
            assert_eq!(
                exact(&[navigate::zoom_by(&plane, octaves)]),
                exact(&leaf_edits(leaf::zoom(
                    &affine(&plane),
                    Scale::Octaves(octaves),
                    &gate
                ))),
                "the wheel's zoom is not the zoom leaf's"
            );
        }
        for zoom in [-3.0, 0.5, 7.0] {
            let to = view.with_zoom(zoom);
            let k = to.scale / view.scale;
            assert_eq!(
                exact(&[zoom_row(&plane, view, to)]),
                exact(&leaf_edits(leaf::zoom(
                    &affine(&plane),
                    Scale::Factor(k),
                    &gate
                ))),
                "the zoom row is not the zoom leaf's"
            );
        }
        for at in [(0.25, 0.75), (0.5, 0.5), (0.9, 0.1)] {
            for setting in [CentreOnPin::Off, CentreOnPin::On] {
                let mut scratch = Scratch {
                    centre_on_pin: setting,
                    ..Scratch::default()
                };
                let mut out = Out::default();
                mv_lock::lock_at(&plane, at, &mut scratch, &mut out);
                assert_eq!(
                    exact(&out.commands.concat()),
                    exact(&leaf_edits(leaf::lock(
                        &affine(&plane),
                        &Affine,
                        at,
                        setting,
                        &gate
                    ))),
                    "the lock at a point is not the lock leaf's"
                );
            }
        }
        for (d1, d2) in [(5.0, 0.0), (0.0, -2.5), (30.0, 12.0)] {
            let mut to = Angles::of(&view);
            to.tau1 = (to.tau1 + d1).clamp(-90.0, 90.0);
            to.tau2 = (to.tau2 + d2).clamp(-90.0, 90.0);
            assert_eq!(
                exact(&[compass::tilt(&plane, &frame, d1, d2)]),
                exact(&tilt_leaf(&plane, &view, to)),
                "the compass's tilt step is not the tilt leaf's"
            );
        }
        let mut drag = compass::Drag::start(Touch::Tilt, &plane, &frame);
        drag.tau = (drag.tau.0 + 17.0, drag.tau.1 - 4.0);
        let to = Angles {
            tau1: drag.tau.0,
            tau2: drag.tau.1,
            gamma: Angles::of(&view).gamma,
        };
        assert_eq!(
            exact(&[compass::drag_edit(&plane, &frame, &drag).expect("a tilt drag")]),
            exact(&tilt_leaf(&plane, &view, to)),
            "the compass's tilt drag is not the tilt leaf's"
        );
    }
}

#[test]
fn path_adapter_same_leaf() {
    check_same_leaf(navigate::zoom_to);
}

#[test]
#[should_panic(expected = "the zoom row is not the zoom leaf's")]
fn path_control_adapter_holding_a_formula() {
    check_same_leaf(rebuilt_zoom_row);
}

// ── path_merge_bitwise ────────────────────────────────────────────────────────────────────────────────────────────

fn add(a: &Latent, k: f64, b: &Latent) -> Latent {
    std::array::from_fn(|i| a[i] + k * b[i])
}

fn z0_edit(z0: Latent) -> SetField {
    SetField {
        edit: Edit::Sim(SimField::Z0(z0)),
        no_history: false,
    }
}

/// `navigate::pan` as it stood, or the control's with one operation reordered.
fn old_pan(plane: &Plane, ds: f64, dt: f64, reordered: bool) -> SetField {
    z0_edit(match reordered {
        false => add(&add(&plane.z0, 2.0 * ds, &plane.q1), 2.0 * dt, &plane.q2),
        true => add(&add(&plane.z0, 2.0 * dt, &plane.q2), 2.0 * ds, &plane.q1),
    })
}

fn check_merge(reordered: bool) {
    for (plane, frame) in planes() {
        for (d, size) in DRAGS {
            let (delta, size) = (egui::vec2(d[0], d[1]), egui::vec2(size[0], size[1]));
            let ds = -f64::from(delta.x) / f64::from(size.x);
            let dt = f64::from(delta.y) / f64::from(size.y);
            assert_eq!(
                exact(&[navigate::pan(&plane, delta, size)]),
                exact(&[old_pan(&plane, ds, dt, reordered)]),
                "the drag's pan moved a bit"
            );
        }
        for octaves in [0.125f64, -1.0 / 64.0, 1.0, -3.0] {
            let k = (-octaves).exp2();
            let old = mv::basis_edit((plane.q1.map(|x| x * k), plane.q2.map(|x| x * k)));
            assert_eq!(exact(&[navigate::zoom_by(&plane, octaves)]), exact(&[old]));
        }
        let view = View::framed(&plane, Some(&frame));
        for zoom in [-3.0, 0.5, 7.0, 20.0] {
            let to = view.with_zoom(zoom);
            assert_eq!(
                exact(&[navigate::zoom_to(&plane, view, to)]),
                exact(&[old_zoom_row(&plane, view, to)]),
                "the zoom row moved a bit"
            );
        }
        for (s, t) in [(0.25, 0.75), (0.9, 0.1)] {
            let z = mv::point(&plane, s, t);
            let old = vec![
                z0_edit(z),
                mv::lock_edit(Lock {
                    locked: true,
                    z_locked: z,
                }),
            ];
            // Main's lock re-centred: the lock with the centre-on-pin setting on, now off by default (TASK-M8-125).
            let mut scratch = Scratch {
                centre_on_pin: CentreOnPin::On,
                ..Scratch::default()
            };
            let mut out = Out::default();
            mv_lock::lock_at(&plane, (s, t), &mut scratch, &mut out);
            assert_eq!(exact(&out.commands.concat()), exact(&old));
        }
    }
}

#[test]
fn path_merge_bitwise() {
    check_merge(false);
}

#[test]
#[should_panic(expected = "the drag's pan moved a bit")]
fn path_control_reordered() {
    check_merge(true);
}

// ── path_canonical_conversion ─────────────────────────────────────────────────────────────────────────────────────

fn basis_of(edit: &SetField) -> (Latent, Latent) {
    match &edit.edit {
        Edit::Sim(SimField::Basis { q1, q2 }) => (*q1, *q2),
        other => panic!("not a basis edit: {other:?}"),
    }
}

fn z0_of(edit: &SetField) -> Latent {
    match &edit.edit {
        Edit::Sim(SimField::Z0(z0)) => *z0,
        other => panic!("not an edit of z₀: {other:?}"),
    }
}

#[test]
fn path_canonical_conversion() {
    let mut worst_tilt = 0.0f64;
    for (plane, frame) in planes() {
        let view = View::framed(&plane, Some(&frame));
        for (d1, d2) in [(5.0, 0.0), (0.0, -2.5), (30.0, 12.0), (-61.0, 44.5)] {
            let mut to = Angles::of(&view);
            to.tau1 = (to.tau1 + d1).clamp(-90.0, 90.0);
            to.tau2 = (to.tau2 + d2).clamp(-90.0, 90.0);
            let w = to.apply(view).basis();
            let n = basis_of(&compass::tilt(&plane, &frame, d1, d2));
            for (a, b) in w.0.iter().chain(&w.1).zip(n.0.iter().chain(&n.1)) {
                let d = (a - b).abs() / (f64::EPSILON * view.scale);
                assert!(
                    d <= 8.0,
                    "the compass's tilt: {a} against {b}, {d} εσ, past 8"
                );
                worst_tilt = worst_tilt.max(d);
            }
        }
    }
    println!("path_canonical_conversion (gui): the tilt {worst_tilt} εσ at most (bound 8)");
}

/// The compass's slice is not merged: CI measured it 1.6·10⁻³ from the slice leaf's `z₀ + a·off_plane(n)` on these
/// planes, past the rounding bound `16ε|a| + 1 ulp` (2.3·10⁻¹⁶), so the drawn normal `n` is not normal to the plane in
/// 8D there and the difference is not rounding. It stays the compass's own `z₀ + a·n`, bit for bit, and goes to the
/// owner. This test holds it there and prints the difference from the leaf.
/// The compass's slice as a route: the gui's, or the control's, the slice leaf in its place.
type SliceRoute = fn(&Plane, &SliceFrame, f64) -> SetField;

fn leaf_slice(plane: &Plane, frame: &SliceFrame, amount: f64) -> SetField {
    let n = compass::latent_normal(plane, frame).expect("a drawn plane");
    leaf_edits(leaf::slice(&affine(plane), &n, amount, &stop::affine())).remove(0)
}

fn check_slice(route: SliceRoute) {
    let mut worst = 0.0f64;
    for (plane, frame) in planes() {
        let Some(n) = compass::latent_normal(&plane, &frame) else {
            continue;
        };
        for amount in [0.05, -0.3, 0.5] {
            let got = route(&plane, &frame, amount);
            assert_eq!(
                exact(std::slice::from_ref(&got)),
                exact(&[z0_edit(add(&plane.z0, amount, &n))]),
                "the compass's slice is no longer its own z₀ + a·n"
            );
            let leaf = z0_of(&leaf_slice(&plane, &frame, amount));
            let was = z0_of(&got);
            worst = worst.max(
                leaf.iter()
                    .zip(&was)
                    .map(|(a, b)| (a - b).abs())
                    .fold(0.0, f64::max),
            );
        }
    }
    println!("path_slice_not_merged: the slice leaf differs from the compass's slice by {worst:e} at most");
}

#[test]
fn path_slice_not_merged() {
    check_slice(|p, f, a| compass::slice(p, f, a).expect("a slice"));
}

#[test]
#[should_panic(expected = "no longer its own")]
fn path_control_slice_merged() {
    check_slice(leaf_slice);
}

// ── path_pin_kept ─────────────────────────────────────────────────────────────────────────────────────────────────

/// `plane` after `edits`.
fn applied(plane: &Plane, edits: &[SetField]) -> Plane {
    compass::after(plane, edits)
}

#[test]
fn path_pin_kept() {
    check_pin(true);
}

#[test]
#[should_panic(expected = "the anchor left")]
fn path_control_pin_not_turned() {
    check_pin(false);
}

/// The anchor held through the compass's tilt and the wheel's zoom, each turned about the pin as the app turns it;
/// `turned` false, the control, sends the edits as they are.
fn check_pin(turned: bool) {
    for (plane, frame) in planes() {
        let (s, t) = (0.3, 0.8);
        let z_locked = mv::point(&plane, s, t);
        let mut sim = affine(&plane).sim;
        sim.slice.frame = frame;
        sim.lock = Lock {
            locked: true,
            z_locked,
        };
        let (s, t) = mv::chart_coords(&plane, &z_locked).expect("a chart");
        for (what, edit) in [
            (
                "the compass's tilt",
                compass::tilt(&plane, &frame, 14.0, -6.5),
            ),
            ("the wheel's zoom", navigate::zoom_by(&plane, 0.75)),
        ] {
            let sent = match turned {
                true => mv::about_the_pin(vec![edit], &sim),
                false => vec![edit],
            };
            let after = applied(&plane, &sent);
            let at = mv::point(&after, s, t);
            assert!(
                at.iter()
                    .zip(&z_locked)
                    .all(|(a, b)| (a - b).abs() <= 1e-12),
                "{what}: the anchor left ({s}, {t}): {at:?} against {z_locked:?}"
            );
        }
    }
}
