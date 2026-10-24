//! QA's tests for TASK-M2-23, the lock, written from the requirements it closes (REQ-GUI-004..007, REQ-SYS-012,
//! REQ-CHART-010, REQ-CHART-007) and chart_decoder_contract Part 4, "The lock (projective microscope)", not from the
//! implementation. Each reads a pixel through the kernel's own chart maps and decode, the edits through the real state
//! store, and checks against an answer computed here by other means: the affine formula by hand, the clicked state
//! decoded through the latent decode within `ε_phys` (REQ-ENC-024), the parabola `K ≥ L_z²/2I` with `I` the moment of
//! inertia about the centre of mass computed here, and the tangent pushed forward through the links' `forward` against
//! `γ′(ν₀)` and the softmax's analytic derivative.
//!
//! The random draws are a fixed-seed SplitMix64 over `validation::prop::CASES` cases (REQ-VAL-151): `engine` has no
//! `proptest` dev-dependency, so this file cannot add one.

use engine::chart_authoring::charts::NU_CURVE;
use engine::chart_authoring::{CurveMeta, CurvePoint};
use engine::contract::interface::EngineInterface;
use engine::contract::render_state::{Hover, Overlays, Palette, Playhead, RenderState, StainGraph};
use engine::contract::set_field::{Edit, SetField, SimField};
use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Latent, Links, Lock, Plane, Quality,
    SimConfig, Slice,
};
use engine::contract::store::StateStore;
use engine::lock::{
    curve_tangent, curve_tilt, kernel_chart, move_slider, recentre, set, set_on_chart, slider,
    snap_back, unlock, validate, LockError, Pixel, Remedy, Report, Tag, Window,
};
use engine::lookup::Tolerances;
use engine::nav::directions::hidden;
use engine::nav::{oblique, pan, slice, zoom, BasisVector, ChartConstructor};
use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{Beta, ZQ1};
use kernel::chart::{
    Chart as KernelChart, ChartDecode, Constraint, InvariantLE, InvariantLK, Latent as LatentChart,
    Mixed, ValidationResult,
};
use kernel::decode::descriptor::potential;
use kernel::decode::{decode, Ic};
use kernel::encode::by_kind::ByKind;
use kernel::generated::links::{
    Defaults, Link, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ, StickBreaking,
};
use validation::negative_control;
use validation::prop::CASES;
use validation::roundtrip::phys;

type Sticks =
    Select<StickBreaking, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

// ---- shared ----

/// SplitMix64, uniform in `[0, 1)`.
struct Rng(u64);

impl Rng {
    fn u(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }

    fn in_(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.u()
    }

    fn below(&mut self, n: usize) -> usize {
        ((self.u() * n as f64) as usize).min(n - 1)
    }

    fn latent(&mut self, r: f64) -> Latent {
        std::array::from_fn(|_| self.in_(-r, r))
    }
}

fn sim(plane: Plane) -> SimConfig {
    SimConfig {
        chart: Chart {},
        plane,
        slice: Slice {},
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links: Links {},
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 100.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

fn store(s: SimConfig) -> StateStore {
    StateStore::new(
        s,
        RenderState {
            stain_graph: StainGraph::new(),
            overlays: Overlays::default(),
            hover: Hover::default(),
            palette: Palette {},
            playhead: Playhead { t: 0.0 },
        },
    )
}

fn apply(st: &mut StateStore, edits: impl IntoIterator<Item = SetField>) {
    for e in edits {
        st.set_field(e);
    }
}

fn sim_of(st: &mut StateStore) -> SimConfig {
    st.snapshot().sim
}

/// A random oblique plane: a random centre, an orthonormal pair from two random draws, scaled.
fn random_plane(rng: &mut Rng) -> Plane {
    let z0 = rng.latent(1.5);
    let (a, b) = (rng.latent(1.0), rng.latent(1.0));
    let p = oblique(z0, &a, &b).expect("two random draws span a plane");
    let k = rng.in_(0.05, 1.0);
    Plane {
        z0: p.z0,
        q1: p.q1.map(|x| k * x),
        q2: p.q2.map(|x| k * x),
    }
}

/// The view's pixel `(s, t)` as the affine chart shows it: the kernel's latent chart `Φ` and decode, read here
/// through the chart, not through `engine::lock`.
fn pixel_ic(plane: &Plane, s: f64, t: f64) -> Ic<f64> {
    LatentChart {
        z0: plane.z0,
        q1: plane.q1,
        q2: plane.q2,
    }
    .decode_at::<Defaults, f64>(s, t)
    .ic
}

/// The free decode of the latent `z`.
fn latent_ic(z: &Latent) -> Ic<f64> {
    decode::<Defaults, f64>(&kernel::decode::latent::Latent::new(*z)).ic
}

fn ic_bits(x: &Ic<f64>) -> Vec<u64> {
    x.m.iter()
        .chain(x.r.iter().flatten())
        .chain(x.p.iter().flatten())
        .map(|v| v.to_bits())
        .collect()
}

fn latent_bits(z: &Latent) -> [u64; 8] {
    z.map(f64::to_bits)
}

/// The lock's edits as `(z₀, lock group)`, asserting they are those two `SimConfig` edits in that order.
fn lock_pair(edits: &[SetField; 2]) -> (Latent, Lock) {
    match (&edits[0].edit, &edits[1].edit) {
        (Edit::Sim(SimField::Z0(z)), Edit::Sim(SimField::Lock(l))) => (*z, l.clone()),
        other => {
            panic!("the lock's edits are not the recentring on z₀ then the lock group: {other:?}")
        }
    }
}

/// The register's tolerances (REQ-ENC-024's `ε_phys`, REQ-ENC-027's sanity, REQ-ENC-036's zero band).
fn tol() -> Tolerances {
    Tolerances::register(f64::EPSILON)
}

// ---- REQ-GUI-004: setting the lock ----

/// Locks `plane` at `(s, t)` with `lock`, and checks: `z_locked` is `z₀ + (2s−1)q₁ + (2t−1)q₂` to the rounding of its
/// three products and two sums; the recentring is a `SetField` on `z₀ = z_locked`; after it the centre pixel's IC is
/// the clicked pixel's, bit for bit; the basis is untouched.
fn check_affine(plane: &Plane, s: f64, t: f64, lock: fn(&Plane, f64, f64) -> [SetField; 2]) {
    let clicked = pixel_ic(plane, s, t);
    let edits = lock(plane, s, t);
    let (z0, group) = lock_pair(&edits);
    assert!(group.locked, "the lock group's flag is not set");
    assert_eq!(
        latent_bits(&z0),
        latent_bits(&group.z_locked),
        "z₀ is not recentred on z_locked"
    );
    let (a, b) = (2.0 * s - 1.0, 2.0 * t - 1.0);
    for i in 0..8 {
        let want = plane.z0[i] + a * plane.q1[i] + b * plane.q2[i];
        let scale = plane.z0[i].abs() + (a * plane.q1[i]).abs() + (b * plane.q2[i]).abs();
        assert!(
            (group.z_locked[i] - want).abs() <= 4.0 * f64::EPSILON * scale,
            "z_locked[{i}] = {} is not z₀ + (2s−1)q₁ + (2t−1)q₂ = {want} at ({s}, {t})",
            group.z_locked[i]
        );
    }
    let mut st = store(sim(plane.clone()));
    apply(&mut st, edits);
    let after = sim_of(&mut st);
    assert!(
        ic_bits(&pixel_ic(&after.plane, 0.5, 0.5)) == ic_bits(&clicked),
        "after the lock at ({s}, {t}) the centre pixel's IC is not the clicked pixel's"
    );
    assert_eq!(
        (after.plane.q1, after.plane.q2),
        (plane.q1, plane.q2),
        "the lock moved the basis"
    );
}

#[test]
fn qa_lock_set_affine() {
    let mut rng = Rng(0x51_23_00_04);
    for _ in 0..CASES {
        let plane = random_plane(&mut rng);
        let (s, t) = (rng.in_(-0.5, 1.5), rng.in_(-0.5, 1.5));
        check_affine(&plane, s, t, |p, s, t| set(p, s, t).expect("a lock"));
    }
    // The corners and the centre, exactly.
    let plane = random_plane(&mut rng);
    for (s, t) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (0.5, 0.5)] {
        check_affine(&plane, s, t, |p, s, t| set(p, s, t).expect("a lock"));
    }
}

negative_control!(
    qa_lock_set_affine,
    "a lock at the pixel mirrored through the centre is not the clicked pixel",
    expected = "is not z₀ + (2s−1)q₁ + (2t−1)q₂",
    check_affine(&random_plane(&mut Rng(9)), 0.2, 0.7, |p, s, t| set(
        p,
        1.0 - s,
        1.0 - t
    )
    .expect("a lock"))
);

/// Locks the nonlinear `chart` at the pixel `(s, t)` of `window` and checks: `Φ` and `D` replayed on the CPU, so the
/// lock's state is the chart's own decode at the clicked chart point; the window is re-centred there, its zoom kept,
/// so the centre pixel decodes the clicked state; `z_locked` is the inverse-encode of that state, decoding back to it
/// within `ε_phys` (REQ-ENC-024); the edits are `z₀ = z_locked` then the lock group; and through the store the free
/// latent centre `z(½,½)` is `z_locked` bit for bit.
fn check_chart<C>(chart: &C, window: &Window, s: f64, t: f64, read_at: fn(f64, f64) -> (f64, f64))
where
    C: ByKind<Param = f64> + ChartDecode,
{
    let (ps, pt) = read_at(s, t);
    let u = window.centre[0] + (2.0 * ps - 1.0) * window.half[0];
    let v = window.centre[1] + (2.0 * pt - 1.0) * window.half[1];
    let clicked = chart.decode_at::<Defaults, f64>(u, v).ic;
    let lock = set_on_chart::<Defaults, C, _>(chart, window, s, t, phys, &tol()).expect("a lock");
    assert_eq!(lock.window.half, window.half, "the lock changed the zoom");
    let c = lock.window.at(0.5, 0.5);
    assert!(
        ic_bits(&chart.decode_at::<Defaults, f64>(c[0], c[1]).ic) == ic_bits(&clicked),
        "the re-centred window's centre pixel is not the clicked pixel on {} at ({s}, {t})",
        chart.name()
    );
    assert!(
        ic_bits(&lock.ic) == ic_bits(&clicked),
        "the lock's state is not the clicked one"
    );
    let (z0, group) = lock_pair(&lock.edits);
    assert!(group.locked && latent_bits(&z0) == latent_bits(&group.z_locked));
    let d = phys(&clicked, &latent_ic(&group.z_locked));
    assert!(
        d <= tol().eps_phys,
        "z_locked decodes {d} from the clicked state on {} (ε_phys {})",
        chart.name(),
        tol().eps_phys
    );
    let mut st = store(sim(Plane {
        z0: [0.0; 8],
        q1: unit_e(0),
        q2: unit_e(1),
    }));
    apply(&mut st, lock.edits);
    let after = sim_of(&mut st);
    let centre = LatentChart {
        z0: after.plane.z0,
        q1: after.plane.q1,
        q2: after.plane.q2,
    }
    .map::<f64>(0.5, 0.5)
    .z
    .z;
    assert_eq!(
        latent_bits(&centre),
        latent_bits(&group.z_locked),
        "the free centre is not z_locked"
    );
}

fn unit_e(k: usize) -> Latent {
    let mut v = [0.0; 8];
    v[k] = 1.0;
    v
}

fn lk() -> InvariantLK<f64> {
    InvariantLK(Invariant {
        z_cfg: [-0.2, 0.5],
        z_mass: [0.1, 0.3],
        k_max: 2.0,
        gamma_k: 1.5,
    })
}

fn beta_q1() -> Mixed<Beta, ZQ1, f64> {
    Mixed::new(
        [0.2, 0.1, -0.3, 0.0, 0.25, -0.15, 0.3, -0.2],
        [0.4, 1.1],
        [-0.8, 0.6],
    )
}

/// Pixels whose chart point passes `validate` on `chart`, drawn over `window`.
fn valid_pixels<C: KernelChart>(
    chart: &C,
    window: &Window,
    rng: &mut Rng,
    n: usize,
) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for _ in 0..64 * n {
        let (s, t) = (rng.u(), rng.u());
        let [u, v] = window.at(s, t);
        if chart.validate(u, v) == ValidationResult::Valid {
            out.push((s, t));
        }
        if out.len() == n {
            break;
        }
    }
    assert_eq!(out.len(), n, "too few valid pixels on {}", chart.name());
    out
}

#[test]
fn qa_lock_set_nonlinear() {
    let mut rng = Rng(0x51_23_00_04_02);
    let windows = [
        Window {
            centre: [0.5, 0.5],
            half: [0.5, 0.5],
        },
        Window {
            centre: [0.6, 0.4],
            half: [0.07, 0.11],
        },
    ];
    for w in &windows {
        for (s, t) in valid_pixels(&lk(), w, &mut rng, 16) {
            check_chart(&lk(), w, s, t, |s, t| (s, t));
        }
        for (s, t) in valid_pixels(&beta_q1(), w, &mut rng, 16) {
            check_chart(&beta_q1(), w, s, t, |s, t| (s, t));
        }
    }
}

negative_control!(
    qa_lock_set_nonlinear,
    "the state of another pixel is not the clicked one",
    expected = "is not the clicked pixel",
    check_chart(
        &beta_q1(),
        &Window {
            centre: [0.5, 0.5],
            half: [0.5, 0.5]
        },
        0.3,
        0.6,
        |s, t| (t, s)
    )
);

/// A lock at a chart point the chart's `validate` refuses sets nothing, and one at a point it passes is set, on
/// `(L_z, K)` (R-469); `verdict` is the domain the expectation is read from.
fn check_refusals(verdict: fn(f64, f64) -> ValidationResult) {
    let chart = lk();
    // The window reaches past the chart's square, where the fan's edge refuses points.
    let square = Window {
        centre: [0.5, 0.5],
        half: [0.75, 0.75],
    };
    let mut refused = 0;
    for j in 0..=20 {
        for i in 0..=20 {
            let (s, t) = (i as f64 / 20.0, j as f64 / 20.0);
            let [u, v] = square.at(s, t);
            let got = set_on_chart::<Defaults, _, _>(&chart, &square, s, t, phys, &tol());
            match verdict(u, v) {
                ValidationResult::Valid => {
                    assert!(
                        got.is_ok(),
                        "a lock at the system ({u}, {v}) is refused: {got:?}"
                    )
                }
                bad => {
                    refused += 1;
                    assert_eq!(
                        got,
                        Err(LockError::NotASystem(bad)),
                        "a lock at ({u}, {v}) is set"
                    );
                }
            }
        }
    }
    assert!(
        refused > 0,
        "no chart point of (L_z, K) is refused: the check is vacuous"
    );
}

#[test]
fn qa_lock_set_refuses_infeasible_chart_point() {
    check_refusals(|u, v| lk().validate(u, v));
}

negative_control!(
    qa_lock_set_refuses_infeasible_chart_point,
    "the domain read a step higher in K is not the chart's",
    expected = "a lock at",
    check_refusals(|u, v| lk().validate(u, v - 0.3))
);

// ---- REQ-GUI-005: tilt, zoom, chart switch keep the anchor; the excursion; snap-back ----

/// One random anchor-preserving gesture: a raw tilt of one basis vector, a tilt pair, a zoom, or a chart switch
/// (to an axis-aligned pair or to an oblique plane through the centre).
fn basis_gesture(s: &SimConfig, rng: &mut Rng) -> Option<SetField> {
    let p = &s.plane;
    let h = hidden(p).ok()?;
    match rng.below(5) {
        0 => {
            let which = if rng.u() < 0.5 {
                BasisVector::Q1
            } else {
                BasisVector::Q2
            };
            ChartConstructor::new(p.clone())
                .tilt(p, which, rng.in_(-90.0, 90.0), &h[rng.below(6)].direction)
                .ok()
        }
        1 => ChartConstructor::new(p.clone())
            .tilt_pair(
                p,
                (rng.in_(-90.0, 90.0), &h[rng.below(6)].direction),
                (rng.in_(-90.0, 90.0), &h[rng.below(6)].direction),
            )
            .ok(),
        2 => zoom(p, rng.in_(-2.0, 2.0)).ok(),
        3 => {
            let a = rng.below(8);
            let b = (a + 1 + rng.below(7)) % 8;
            let k = rng.in_(0.1, 3.0);
            Some(SetField {
                edit: Edit::Sim(SimField::Basis {
                    q1: unit_e(a).map(|x| k * x),
                    q2: unit_e(b).map(|x| k * x),
                }),
                no_history: false,
            })
        }
        _ => {
            let o = oblique(p.z0, &rng.latent(1.0), &rng.latent(1.0)).ok()?;
            Some(SetField {
                edit: Edit::Sim(SimField::Basis { q1: o.q1, q2: o.q2 }),
                no_history: false,
            })
        }
    }
}

/// One case: lock at a random pixel, then `n` gestures from `gesture`, the centre's IC and `z₀` the anchor's bit for
/// bit after each; then an excursion (a slice and a slider move), `δ = z₀ − z_locked` nonzero and ghost-marked, the
/// basis gestures keeping it; then snap-back, `δ → 0` exactly and the centre the anchor's bit for bit.
fn check_anchor(seed: u64, n: usize, gesture: fn(&SimConfig, &mut Rng) -> Option<SetField>) {
    let mut rng = Rng(seed);
    let plane = random_plane(&mut rng);
    let mut st = store(sim(plane.clone()));
    apply(&mut st, set(&plane, rng.u(), rng.u()).expect("a lock"));
    let anchor = sim_of(&mut st).lock.z_locked;
    let anchor_ic = ic_bits(&latent_ic(&anchor));
    let mut applied = 0;
    for _ in 0..n {
        let s = sim_of(&mut st);
        let Some(g) = gesture(&s, &mut rng) else {
            continue;
        };
        apply(&mut st, [g]);
        applied += 1;
        let s = sim_of(&mut st);
        assert!(
            latent_bits(&s.plane.z0) == latent_bits(&anchor)
                && ic_bits(&pixel_ic(&s.plane, 0.5, 0.5)) == anchor_ic,
            "the centre IC moved off the anchor (seed {seed})"
        );
        assert_eq!(
            latent_bits(&s.lock.z_locked),
            latent_bits(&anchor),
            "the stored anchor moved"
        );
    }
    assert!(applied > 0, "no gesture applied (seed {seed})");
    // The excursion.
    let s = sim_of(&mut st);
    let d = hidden(&s.plane).expect("a plane")[rng.below(6)].direction;
    apply(
        &mut st,
        [slice(&s.plane, &d, rng.in_(0.1, 0.5)).expect("a slice")],
    );
    let k = rng.below(8);
    let s = sim_of(&mut st);
    let value = s.plane.z0[k] + rng.in_(0.1, 0.5);
    apply(&mut st, [move_slider(&s, k, value).expect("a slider")]);
    let away = sim_of(&mut st);
    let delta = away
        .lock
        .excursion(&away.plane.z0)
        .expect("locked: δ exists");
    for i in 0..8 {
        assert_eq!(
            delta[i].to_bits(),
            (away.plane.z0[i] - anchor[i]).to_bits(),
            "δ[{i}] is not z₀ − z_locked"
        );
    }
    assert!(delta.iter().any(|&x| x != 0.0), "the excursion left δ = 0");
    let ghost = slider(&away, k).expect("a control");
    assert_eq!(
        ghost.anchor.map(f64::to_bits),
        Some(anchor[k].to_bits()),
        "the anchor is not ghost-marked"
    );
    // A basis gesture during the excursion keeps it.
    if let Some(g) = basis_gesture(&away, &mut rng) {
        apply(&mut st, [g]);
        let s = sim_of(&mut st);
        assert_eq!(
            latent_bits(&s.plane.z0),
            latent_bits(&away.plane.z0),
            "a tilt moved the excursion"
        );
    }
    // Snap-back.
    let s = sim_of(&mut st);
    apply(&mut st, [snap_back(&s).expect("locked")]);
    let back = sim_of(&mut st);
    assert_eq!(
        latent_bits(&back.plane.z0),
        latent_bits(&anchor),
        "snap-back did not restore the anchor bitwise"
    );
    assert!(
        ic_bits(&pixel_ic(&back.plane, 0.5, 0.5)) == anchor_ic,
        "snap-back's centre IC is not the anchor's"
    );
    let zero = back.lock.excursion(&back.plane.z0).expect("still locked");
    assert!(
        zero.iter().all(|&x| x == 0.0),
        "δ is {zero:?} after snap-back, not 0"
    );
    assert!(back.lock.locked, "snap-back unlocked");
}

#[test]
fn qa_lock_anchor_property() {
    for case in 0..CASES as u64 {
        check_anchor(0xA11C_E000 + case, 24, basis_gesture);
    }
}

negative_control!(
    qa_lock_anchor_property,
    "a pan is not anchor-preserving",
    expected = "the centre IC moved off the anchor",
    check_anchor(17, 8, |s, rng| pan(
        &s.plane,
        rng.in_(0.05, 0.3),
        rng.in_(0.05, 0.3)
    )
    .ok())
);

// ---- REQ-GUI-006: re-based sliders; unlock keeps the state ----

/// Locks, then moves random sliders with `mover`: each move changes `z₀[k]` alone, to the value moved to, so `δ`
/// changes along `ê_k` alone; the slider shows that value live, re-based as anchor + δ, with the anchor marked; then
/// unlock emits the lock group alone and leaves `(z₀, q₁, q₂)` the last locked one bit for bit.
fn check_rebased(seed: u64, mover: fn(&SimConfig, usize, f64) -> SetField) {
    let mut rng = Rng(seed);
    let plane = random_plane(&mut rng);
    let mut st = store(sim(plane.clone()));
    apply(&mut st, set(&plane, rng.u(), rng.u()).expect("a lock"));
    let anchor = sim_of(&mut st).lock.z_locked;
    for _ in 0..6 {
        let before = sim_of(&mut st);
        let d_before = before.lock.excursion(&before.plane.z0).expect("locked");
        let k = rng.below(8);
        let value = rng.in_(-3.0, 3.0);
        apply(&mut st, [mover(&before, k, value)]);
        let after = sim_of(&mut st);
        let d = after.lock.excursion(&after.plane.z0).expect("locked");
        for i in 0..8 {
            if i == k {
                assert_eq!(
                    after.plane.z0[i].to_bits(),
                    value.to_bits(),
                    "the slider {k} is frozen, not live"
                );
            } else {
                assert_eq!(
                    d[i].to_bits(),
                    d_before[i].to_bits(),
                    "a move of slider {k} moved δ[{i}]"
                );
            }
        }
        let shown = slider(&after, k).expect("a control");
        assert_eq!(
            shown.value.to_bits(),
            value.to_bits(),
            "the slider does not show z₀ live"
        );
        assert_eq!(
            shown.anchor.map(f64::to_bits),
            Some(anchor[k].to_bits()),
            "the slider is not re-based on the anchor"
        );
        let off = shown.offset.expect("locked: an offset");
        assert!(
            (anchor[k] + off - value).abs() <= f64::EPSILON * value.abs().max(anchor[k].abs()),
            "the slider's anchor + δ is not its value"
        );
    }
    let last = sim_of(&mut st);
    let un = unlock(&last).expect("locked");
    assert!(
        matches!(
            un.edit,
            Edit::Sim(SimField::Lock(Lock { locked: false, .. }))
        ),
        "unlock is {un:?}"
    );
    apply(&mut st, [un]);
    let free = sim_of(&mut st);
    assert!(!free.lock.locked);
    for (a, b) in [
        (free.plane.z0, last.plane.z0),
        (free.plane.q1, last.plane.q1),
        (free.plane.q2, last.plane.q2),
    ] {
        assert_eq!(
            latent_bits(&a),
            latent_bits(&b),
            "unlock did not keep (z₀, q₁, q₂)"
        );
    }
    assert_eq!(
        free.lock.excursion(&free.plane.z0),
        None,
        "a free view has an excursion"
    );
    let s = slider(&free, 3).expect("a control");
    assert_eq!(
        (s.anchor, s.offset),
        (None, None),
        "a free slider is ghost-marked"
    );
}

#[test]
fn qa_lock_sliders_rebased() {
    for case in 0..CASES as u64 {
        check_rebased(0x5_11DE_0000 + case, |s, k, v| {
            move_slider(s, k, v).expect("a slider")
        });
    }
    let s = sim(random_plane(&mut Rng(1)));
    assert_eq!(move_slider(&s, 8, 0.0), Err(LockError::NoSuchControl(8)));
}

negative_control!(
    qa_lock_sliders_rebased,
    "a slider frozen at the anchor while locked is not live",
    expected = "is frozen, not live",
    check_rebased(3, |s, k, _| move_slider(s, k, s.lock.z_locked[k])
        .expect("a slider"))
);

// ---- REQ-GUI-007: SimConfig fields and SetFields, never ViewUI ----

/// Each edit is an undoable `SimConfig` edit of `z₀` or the lock group: none is a render-state (ViewUI) edit.
fn check_sim_only(edits: &[SetField]) {
    for e in edits {
        assert!(!e.no_history, "{e:?} skips the history");
        match &e.edit {
            Edit::Sim(SimField::Z0(_)) | Edit::Sim(SimField::Lock(_)) => {}
            other => panic!("the lock edit {other:?} is not a SimConfig z₀ or lock edit"),
        }
    }
}

#[test]
fn qa_lock_setfields_sim_only() {
    let mut rng = Rng(77);
    let plane = random_plane(&mut rng);
    let mut st = store(sim(plane.clone()));
    let render_before = st.snapshot().render;
    let lock = set(&plane, 0.25, 0.75).expect("a lock");
    check_sim_only(&lock);
    apply(&mut st, lock);
    let locked = sim_of(&mut st);
    let gestures = [
        move_slider(&locked, 2, 0.9).expect("a slider"),
        snap_back(&locked).expect("locked"),
        unlock(&locked).expect("locked"),
    ];
    check_sim_only(&gestures);
    apply(&mut st, gestures);
    check_sim_only(
        &set_on_chart::<Defaults, _, _>(
            &lk(),
            &Window {
                centre: [0.5, 0.5],
                half: [0.5, 0.5],
            },
            0.5,
            0.9,
            phys,
            &tol(),
        )
        .expect("a lock")
        .edits,
    );
    assert_eq!(
        st.snapshot().render,
        render_before,
        "a lock gesture edited the render state"
    );
    // The lock flag and the anchor are SimConfig's own serialised fields; δ is z₀ − z_locked of them.
    let json = serde_json::to_value(&locked).expect("SimConfig serialises");
    assert_eq!(
        json["lock"]["locked"],
        serde_json::json!(true),
        "no lock flag in SimConfig: {json}"
    );
    assert!(
        json["lock"]["z_locked"].is_array(),
        "no anchor in SimConfig: {json}"
    );
    assert!(json["plane"]["z0"].is_array(), "no z₀ in SimConfig: {json}");
}

negative_control!(
    qa_lock_setfields_sim_only,
    "a basis edit is not a lock edit",
    expected = "is not a SimConfig z₀ or lock edit",
    check_sim_only(&[SetField {
        edit: Edit::Sim(SimField::Basis {
            q1: unit_e(0),
            q2: unit_e(1)
        }),
        no_history: false
    }])
);

// ---- REQ-SYS-012: the kernel's chart is the plane's, whatever the lock ----

fn plane_chart_bits(p: &Plane) -> Vec<u32> {
    p.z0.iter()
        .chain(&p.q1)
        .chain(&p.q2)
        .map(|&x| (x as f32).to_bits())
        .collect()
}

fn kernel_bits(s: &SimConfig, kernel: fn(&SimConfig) -> LatentChart<f32>) -> Vec<u32> {
    let c = kernel(s);
    c.z0.iter()
        .chain(&c.q1)
        .chain(&c.q2)
        .map(|x| x.to_bits())
        .collect()
}

/// For random planes and random lock states, through lock, excursion, snap-back and unlock, the kernel's chart is
/// the plane's `(z₀, q₁, q₂)` narrowed to f32, bit for bit, and nothing of the lock.
fn check_kernel_blind(seed: u64, kernel: fn(&SimConfig) -> LatentChart<f32>) {
    let mut rng = Rng(seed);
    let plane = random_plane(&mut rng);
    let mut st = store(sim(plane.clone()));
    let mut seen = Vec::new();
    seen.push(sim_of(&mut st));
    apply(&mut st, set(&plane, rng.u(), rng.u()).expect("a lock"));
    seen.push(sim_of(&mut st));
    let s = sim_of(&mut st);
    apply(
        &mut st,
        [move_slider(&s, rng.below(8), rng.in_(-2.0, 2.0)).expect("a slider")],
    );
    seen.push(sim_of(&mut st));
    let s = sim_of(&mut st);
    apply(&mut st, [snap_back(&s).expect("locked")]);
    seen.push(sim_of(&mut st));
    let s = sim_of(&mut st);
    apply(&mut st, [unlock(&s).expect("locked")]);
    seen.push(sim_of(&mut st));
    for s in &seen {
        assert_eq!(
            kernel_bits(s, kernel),
            plane_chart_bits(&s.plane),
            "the kernel's chart is not the plane's alone"
        );
        let mut other = s.clone();
        other.lock = Lock {
            locked: !s.lock.locked,
            z_locked: rng.latent(5.0),
        };
        assert_eq!(
            kernel_bits(&other, kernel),
            kernel_bits(s, kernel),
            "the lock reached the kernel's chart"
        );
    }
}

#[test]
fn qa_kernel_chart_lock_blind() {
    for case in 0..CASES as u64 {
        check_kernel_blind(0x5E5_0120_0000 + case, kernel_chart);
    }
}

negative_control!(
    qa_kernel_chart_lock_blind,
    "a kernel chart centred on the anchor while locked carries the lock",
    expected = "the lock reached the kernel's chart",
    check_kernel_blind(5, |s| {
        let mut c = kernel_chart(s);
        if s.lock.locked {
            c.z0 = s.lock.z_locked.map(|x| x as f32);
        }
        c
    })
);

// ---- REQ-CHART-010: lock validation on a tilted (L_z, E) view ----

fn le(z_cfg: [f64; 2]) -> InvariantLE<f64> {
    InvariantLE(Invariant {
        z_cfg,
        z_mass: [-0.25, 0.4],
        k_max: 1.2,
        gamma_k: 1.0,
    })
}

/// The configuration and masses the frozen controls decode to (the latent decode, momenta's controls 0).
fn frozen_ic(z_cfg: [f64; 2], z_mass: [f64; 2]) -> Ic<f64> {
    latent_ic(&[z_cfg[0], z_cfg[1], 0.0, 0.0, 0.0, 0.0, z_mass[0], z_mass[1]])
}

/// `I`, the moment of inertia about the centre of mass.
fn inertia(ic: &Ic<f64>) -> f64 {
    let m: f64 = ic.m.iter().sum();
    let c = [0, 1].map(|a| (0..3).map(|i| ic.m[i] * ic.r[i][a]).sum::<f64>() / m);
    (0..3)
        .map(|i| ic.m[i] * ((ic.r[i][0] - c[0]).powi(2) + (ic.r[i][1] - c[1]).powi(2)))
        .sum()
}

fn lz_of(ic: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| ic.r[i][0] * ic.p[i][1] - ic.r[i][1] * ic.p[i][0])
        .sum()
}

fn k_of(ic: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| (ic.p[i][0].powi(2) + ic.p[i][1].powi(2)) / (2.0 * ic.m[i]))
        .sum()
}

/// The parabola's verdict, computed here: `Some(true)` where `E − U ≥ L_z²/2I` at the pixel's own configuration,
/// `Some(false)` where not, `None` within the rounding band of the comparison (1024 ε of its largest term).
fn feasible_here(p: &Pixel<InvariantLE<f64>>) -> Option<bool> {
    let ic = frozen_ic(p.chart.0.z_cfg, p.chart.0.z_mass);
    let k = p.y - potential(&ic);
    let bound = p.lz * p.lz / (2.0 * inertia(&ic));
    let band = 1024.0 * f64::EPSILON * k.abs().max(bound).max(p.y.abs()).max(1.0);
    match k - bound {
        x if x > band => Some(true),
        x if x < -band => Some(false),
        _ => None,
    }
}

/// A locked `(L_z, E)` view near the parabola's edge, tilted by `tau` degrees so its horizontal axis carries `z_α`:
/// `n × n` pixels, each its own chart at its own `z_α`. Untilted, its pixels are inside the parabola by a margin of
/// 0.2% of the bound, more than its `L_z` and `E` spans move it; tilted, the potential rises along `+z_α`.
fn tilted(tau: f64) -> Vec<Pixel<InvariantLE<f64>>> {
    let anchor_cfg = [0.3, -0.4];
    let base = frozen_ic(anchor_cfg, le(anchor_cfg).0.z_mass);
    let u0 = potential(&base);
    let i0 = inertia(&base);
    // The anchor: K = 1.002 times the parabola's bound, so the untilted window is inside it.
    let lz0 = 0.6;
    let e0 = u0 + 1.002 * lz0 * lz0 / (2.0 * i0);
    let (sin, cos) = tau.to_radians().sin_cos();
    let n = 11;
    let mut out = Vec::new();
    for j in 0..n {
        for i in 0..n {
            let x = -1.0 + 2.0 * i as f64 / (n - 1) as f64;
            let y = -1.0 + 2.0 * j as f64 / (n - 1) as f64;
            let cfg = [anchor_cfg[0] + sin * x, anchor_cfg[1]];
            out.push(Pixel {
                chart: le(cfg),
                lz: lz0 + 1e-4 * cos * x,
                y: e0 + 1e-4 * y,
            });
        }
    }
    out
}

/// Checks `validator`'s report on `pixels` against the parabola computed here: one tag per pixel in order; feasible
/// pixels `Feasible`, infeasible ones tagged with the parabola; each nearest point keeps the pixel's frozen controls,
/// decodes to a feasible `(L_z, K)` with `L_z`'s sign, and is offered as a re-centre whose edits lock there; the
/// offer ends with the latent chart. Returns the number tagged.
fn check_validation(
    pixels: &[Pixel<InvariantLE<f64>>],
    validator: fn(&[Pixel<InvariantLE<f64>>]) -> Report,
) -> usize {
    let report = validator(pixels);
    assert_eq!(report.tags.len(), pixels.len(), "a pixel was dropped");
    let offer = report.offer();
    let mut tagged = 0;
    let mut recentres = Vec::new();
    for (i, (p, tag)) in pixels.iter().zip(&report.tags).enumerate() {
        match (feasible_here(p), tag) {
            (None, _) => {}
            (Some(true), Tag::Feasible) => {}
            (Some(false), Tag::Infeasible { failed, nearest }) => {
                tagged += 1;
                assert_eq!(
                    *failed,
                    Constraint::LzEParabola,
                    "pixel {i} tagged with {failed:?}"
                );
                if let Some(n) = nearest {
                    recentres.push(i);
                    assert_eq!(
                        [n.z[0], n.z[1], n.z[6], n.z[7]].map(f64::to_bits),
                        [
                            p.chart.0.z_cfg[0],
                            p.chart.0.z_cfg[1],
                            p.chart.0.z_mass[0],
                            p.chart.0.z_mass[1]
                        ]
                        .map(f64::to_bits),
                        "pixel {i}'s nearest point left its configuration or masses"
                    );
                    let ic = latent_ic(&n.z);
                    let (lz, k) = (lz_of(&ic), k_of(&ic));
                    let bound = lz * lz / (2.0 * inertia(&ic));
                    assert!(
                        k >= bound - 1024.0 * f64::EPSILON * bound.max(1.0),
                        "pixel {i}'s nearest point decodes outside the parabola: K {k} < L_z²/2I {bound}"
                    );
                    assert_eq!(
                        lz.signum(),
                        p.lz.signum(),
                        "pixel {i}'s nearest point changed L_z's sign"
                    );
                    assert!(
                        offer.contains(&Remedy::Recentre {
                            pixel: i,
                            nearest: *n
                        }),
                        "pixel {i}'s re-centre is not offered"
                    );
                    let (z0, group) = lock_pair(&recentre(n));
                    assert!(
                        group.locked && latent_bits(&z0) == latent_bits(&n.z),
                        "the re-centre does not lock at the nearest point"
                    );
                }
            }
            (want, got) => panic!(
                "pixel {i}: the parabola says feasible = {want:?}, lock validation says {got:?}"
            ),
        }
    }
    if tagged == 0 {
        assert!(offer.is_empty(), "a feasible view is offered {offer:?}");
    } else {
        assert_eq!(
            offer.last(),
            Some(&Remedy::SwitchToLatent),
            "the latent chart is not offered"
        );
        let offered: Vec<usize> = offer
            .iter()
            .filter_map(|r| match r {
                Remedy::Recentre { pixel, .. } => Some(*pixel),
                Remedy::SwitchToLatent => None,
            })
            .collect();
        assert_eq!(
            offered, recentres,
            "the re-centres offered are not each tagged pixel's, in order"
        );
    }
    tagged
}

#[test]
fn qa_lock_validation_parabola() {
    let flat = tilted(0.0);
    assert_eq!(
        check_validation(&flat, validate::<Defaults, _>),
        0,
        "the untilted view has a tagged pixel"
    );
    let view = tilted(50.0);
    let tagged = check_validation(&view, validate::<Defaults, _>);
    assert!(
        tagged > 0,
        "no pixel was carried past the parabola by the tilt"
    );
    assert!(
        tagged < view.len(),
        "every pixel was tagged: the view has no feasible part"
    );
    assert!(
        validate::<Defaults, _>(&view)
            .offer()
            .iter()
            .any(|r| matches!(r, Remedy::Recentre { .. })),
        "no re-centre is offered: the nearest-point checks are vacuous"
    );
    // In order: reversing the pixels reverses the tags.
    let mut rev = view.clone();
    rev.reverse();
    let mut a = validate::<Defaults, _>(&view).tags;
    a.reverse();
    assert_eq!(
        validate::<Defaults, _>(&rev).tags,
        a,
        "the tags are not in pixel order"
    );
}

negative_control!(
    qa_lock_validation_parabola,
    "a validation that reads every pixel at the anchor's configuration misses the tilt",
    expected = "lock validation says",
    {
        check_validation(&tilted(50.0), |px| {
            let anchor = px[px.len() / 2].chart;
            let moved: Vec<_> = px
                .iter()
                .map(|p| Pixel {
                    chart: anchor,
                    ..*p
                })
                .collect();
            validate::<Defaults, _>(&moved)
        });
    }
);

// ---- REQ-CHART-007: the curve's tangent tilt needs a lock and reads γ′(ν₀) ----

fn softmax(mu: [f64; 2]) -> [f64; 3] {
    let e = [1.0, mu[0].exp(), mu[1].exp()];
    let s: f64 = e.iter().sum();
    e.map(|x| x / s)
}

/// The analytic derivative of `softmax(0, μ₁, μ₂)` along `(dμ₁, dμ₂)`: `dmᵢ = mᵢ(dμᵢ − Σⱼ mⱼdμⱼ)`, `dμ₀ = 0`.
fn dsoftmax(mu: [f64; 2], dmu: [f64; 2]) -> [f64; 3] {
    let m = softmax(mu);
    let d = [0.0, dmu[0], dmu[1]];
    let mean: f64 = (0..3).map(|i| m[i] * d[i]).sum();
    std::array::from_fn(|i| m[i] * (d[i] - mean))
}

/// The tangent `dz` at `z(ν₀)` pushed forward through the links' `forward` by a central difference: it must be `γ′(ν₀)`
/// (`dα`, `dβ`, and the softmax's derivative along `(dμ₁, dμ₂)`) to the difference's error, the contract's `ε^{2/3}`
/// order, bounded here by `√ε`; the momenta's tangent 0, the curve a rest start.
fn check_pushforward<S: Selection>(curve: &CurveMeta, nu0: f64, tangent_at: f64) {
    let p: CurvePoint = (curve.embed)(nu0);
    let g: CurvePoint = (curve.tangent)(nu0);
    let dz = curve_tangent::<S>(curve, tangent_at).expect("a tangent");
    assert!(
        dz[2..6].iter().all(|&x| x == 0.0),
        "a rest curve's tangent moves the momenta: {dz:?}"
    );
    let [za] = <S::AlphaLink as Link<1, 1>>::inverse([p.alpha]);
    let [zb] = <S::BetaLink as Link<1, 1>>::inverse([p.beta]);
    let zm = <S::MassLink as Link<2, 3>>::inverse(softmax([p.mu1, p.mu2]));
    let norm = dz.iter().map(|x| x * x).sum::<f64>().sqrt();
    let h = f64::EPSILON.cbrt() / norm.max(1.0);
    let fa = |k: f64| <S::AlphaLink as Link<1, 1>>::forward([za + k * dz[0]])[0];
    let fb = |k: f64| <S::BetaLink as Link<1, 1>>::forward([zb + k * dz[1]])[0];
    let fm = |k: f64| <S::MassLink as Link<2, 3>>::forward([zm[0] + k * dz[6], zm[1] + k * dz[7]]);
    let tol = f64::EPSILON.sqrt();
    let close = |got: f64, want: f64, what: &str| {
        assert!(
            (got - want).abs() <= tol * want.abs().max(1.0),
            "the tangent tilt's {what} pushes forward to {got}, not γ′(ν₀)'s {want} at ν₀ = {nu0}"
        )
    };
    close((fa(h) - fa(-h)) / (2.0 * h), g.alpha, "α");
    close((fb(h) - fb(-h)) / (2.0 * h), g.beta, "β");
    let (hi, lo) = (fm(h), fm(-h));
    let dm = dsoftmax([p.mu1, p.mu2], [g.mu1, g.mu2]);
    for i in 0..3 {
        close((hi[i] - lo[i]) / (2.0 * h), dm[i], "mass");
    }
}

#[test]
fn qa_curve_tilt_needs_lock_and_reads_the_centre() {
    let free = Lock {
        locked: false,
        z_locked: [0.0; 8],
    };
    let mut rng = Rng(0xC0_7E);
    for _ in 0..CASES {
        let nu0 = rng.in_(0.1, 0.9);
        let tau = rng.in_(-90.0, 90.0);
        let target = unit_e(2 + rng.below(4));
        assert_eq!(
            curve_tilt::<Defaults>(&free, &NU_CURVE, nu0, 1.0, tau, &target),
            Err(LockError::NotLocked),
            "a free tangent tilt at ν₀ = {nu0}, τ = {tau} is not refused"
        );
        let locked = Lock {
            locked: true,
            z_locked: rng.latent(1.0),
        };
        let q = curve_tilt::<Defaults>(&locked, &NU_CURVE, nu0, 0.8, tau, &target)
            .expect("a locked tilt");
        let len = q.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!(
            (len - 0.8).abs() <= 8.0 * f64::EPSILON,
            "the tilt's length {len} is not the one asked"
        );
        check_pushforward::<Defaults>(&NU_CURVE, nu0, nu0);
        check_pushforward::<Sticks>(&NU_CURVE, nu0, nu0);
    }
    // At τ = 0 the locked tilt is γ′(ν₀)'s direction.
    let locked = Lock {
        locked: true,
        z_locked: [0.0; 8],
    };
    let nu0 = 0.4;
    let dz = curve_tangent::<Defaults>(&NU_CURVE, nu0).expect("a tangent");
    let n = dz.iter().map(|x| x * x).sum::<f64>().sqrt();
    let q = curve_tilt::<Defaults>(&locked, &NU_CURVE, nu0, 1.0, 0.0, &unit_e(3)).expect("a tilt");
    for i in 0..8 {
        assert!(
            (q[i] - dz[i] / n).abs() <= 8.0 * f64::EPSILON,
            "τ = 0 is not γ′(ν₀)'s direction"
        );
    }
    // The control the requirement names: the tangent at a pixel's ν, not the centre's, drifts.
    let at_pixel = curve_tangent::<Defaults>(&NU_CURVE, 0.55).expect("a tangent");
    let m = at_pixel.iter().map(|x| x * x).sum::<f64>().sqrt();
    let cos: f64 = (0..8).map(|i| dz[i] / n * at_pixel[i] / m).sum();
    assert!(
        cos < 1.0 - f64::EPSILON.sqrt(),
        "the tangent at the pixel is the centre's: no drift, cos {cos}"
    );
}

negative_control!(
    qa_curve_tilt_needs_lock_and_reads_the_centre,
    "the tangent read at a pixel's ν rather than the centre's is not γ′(ν₀)",
    expected = "not γ′(ν₀)'s",
    check_pushforward::<Defaults>(&NU_CURVE, 0.3, 0.55)
);

/// A curve whose mass logits do not move has a mass tangent of exactly +0, under either mass link (the contract's
/// "a curve that moves no mass"), and its other components are still the pullback.
fn check_still(curve: &CurveMeta) {
    for t in [
        curve_tangent::<Defaults>(curve, 0.3).expect("a tangent"),
        curve_tangent::<Sticks>(curve, 0.3).expect("a tangent"),
    ] {
        assert_eq!(
            [t[6].to_bits(), t[7].to_bits()],
            [0u64, 0u64],
            "a still mass has tangent {:?}",
            &t[6..]
        );
        assert!(
            t[0] != 0.0 && t[1] != 0.0,
            "the configuration's tangent is lost: {t:?}"
        );
    }
    check_pushforward::<Defaults>(curve, 0.3, 0.3);
    check_pushforward::<Sticks>(curve, 0.3, 0.3);
}

const STILL: CurveMeta = CurveMeta {
    parameter: "s",
    embed: |s| CurvePoint {
        alpha: 0.4 + 0.05 * s,
        beta: 0.9 - 0.1 * s,
        mu1: -0.3,
        mu2: 0.45,
    },
    tangent: |_| CurvePoint {
        alpha: 0.05,
        beta: -0.1,
        mu1: 0.0,
        mu2: 0.0,
    },
};

#[test]
fn qa_curve_tangent_still_masses() {
    check_still(&STILL);
}

negative_control!(
    qa_curve_tangent_still_masses,
    "the Burrau curve moves its mass, so its mass tangent is not 0",
    expected = "a still mass has tangent",
    check_still(&NU_CURVE)
);
