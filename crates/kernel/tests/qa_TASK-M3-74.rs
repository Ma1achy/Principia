//! QA's tests for TASK-M3-74 (REQ-VAL-355): the FTLE shadow's joint step on Heggie × RK4 keeps the sample and its
//! shadow at the same physical time at every renormalisation, as dd_integrator §3.8, "Heggie × RK4's joint step and its
//! displacement", defines it (R-974, R-72).
//!
//! Written from REQ-VAL-355's statement and verify and §3.8's definition, not from the implementation. The reference
//! shadow is an absolute one, marched here on its own by RK4 under its own frozen energy `h + δh` (§3.8, "The
//! displacement"), so the tests read the shadow's physical time from a trajectory, not from the displacement's own
//! bookkeeping:
//! - `qa_m374_same_time_bitwise`: on a joint-step run of each fixture, at f32 and f64, under Eq. 22 (the default,
//!   R-161) and Eq. 20, with a constant `Δτ` (so the clocks move apart by the time transformation alone), renormalised
//!   every 25 steps: at every renormalisation the shadow's elapsed physical time since the previous one equals the
//!   sample's, bitwise (§3.8: the resynchronisation sets `δt ← 0` exactly, so the shadow lands on the sample's `t`),
//!   and the clocks were apart before it (the measurement can fire). Control, REQ-VAL-355's: the shadow advanced by the
//!   sample's `Δτ` alone, an absolute shadow restarted at each renormalisation and marched by the same `Δτ`s; its
//!   elapsed `t` drifts from the sample's.
//! - `qa_m374_same_time_order`: what lands on the sample's `t` is the shadow's state there: at f64, the resynchronised
//!   `J·δy` at each renormalisation against the absolute shadow brought to the sample's `t` (Newton on its clock),
//!   less the sample. §3.8 states the resynchronisation and `J` to first order, dropping second-order terms, so the
//!   relative miss is `O(δ)`: the test asserts the observed order in `δ₀` over a decade, `≥ 0.8` (first order is 1).
//!   Control, REQ-VAL-355's: the reference read from the `Δτ`-alone shadow, not brought to the sample's `t`; its miss is
//!   `ẋ·δt`, first order in `δ` itself, so the relative miss does not fall with `δ₀`.

use kernel::occupant::State;
use kernel::regularisation::az::AzReal;
use kernel::regularisation::form::{Flow, Form};
use kernel::regularisation::heggie::{register, HgP, HgQ, HgRegistered, HgSystem, DEFAULT_TIME};
use kernel::regularisation::heggie_joint::{
    cartesian, joint_step, renormalise, resynchronise, start, HgDisplacement,
};
use kernel::regularisation::pairs::HeggieTime;
use validation::negative_control;

// ── Independent fixtures ─────────────────────────────────────────────────────────────────────────────────────────

/// A precision the tests run at.
trait P: AzReal + Into<f64> {
    fn of(x: f64) -> Self;
}

impl P for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }
}

impl P for f64 {
    fn of(x: f64) -> f64 {
        x
    }
}

/// A xorshift generator, so the draws are fixed.
struct Rng(u64);

impl Rng {
    /// A draw in `[−1, 1)`.
    fn unit(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
    }
}

/// Positions and velocities at masses `m`, moved to the centre of mass and momentum.
fn com_state(m: [f64; 3], r: [[f64; 2]; 3], v: [[f64; 2]; 3]) -> State<f64> {
    let total: f64 = m.iter().sum();
    let rc = [0, 1].map(|c| (0..3).map(|i| m[i] * r[i][c]).sum::<f64>() / total);
    let vc = [0, 1].map(|c| (0..3).map(|i| m[i] * v[i][c]).sum::<f64>() / total);
    State {
        m,
        r: [0, 1, 2].map(|i| [0, 1].map(|c| r[i][c] - rc[c])),
        p: [0, 1, 2].map(|i| [0, 1].map(|c| m[i] * (v[i][c] - vc[c]))),
    }
}

/// The fixtures: a hierarchical triple (an inner binary of unequal masses, a light outer body), a Lagrange equilateral
/// rotating at unequal masses, and a seeded random bound-ish state.
fn fixtures() -> Vec<State<f64>> {
    // Hierarchical: binary (1.0, 0.7) at separation 0.6 on a near-circular orbit, outer 0.3 at distance 3.
    let (m0, m1): (f64, f64) = (1.0, 0.7);
    let a = 0.6;
    let vrel = ((m0 + m1) / a).sqrt();
    let hier = com_state(
        [m0, m1, 0.3],
        [
            [-a * m1 / (m0 + m1), 0.0],
            [a * m0 / (m0 + m1), 0.0],
            [0.0, 3.0],
        ],
        [
            [0.0, -vrel * m1 / (m0 + m1)],
            [0.0, vrel * m0 / (m0 + m1)],
            [-((m0 + m1) / 3.0f64).sqrt() * 0.9, 0.0],
        ],
    );
    // Lagrange: equilateral side 1, rigid rotation at ω² = M (G = 1), slightly perturbed velocities.
    let m = [1.2, 0.9, 0.6];
    let total: f64 = m.iter().sum();
    let tri = [[0.0, 0.0], [1.0, 0.0], [0.5, 3.0f64.sqrt() / 2.0]];
    let rc = [0, 1].map(|c| (0..3).map(|i| m[i] * tri[i][c]).sum::<f64>() / total);
    let w = total.sqrt() * 0.97;
    let lag = com_state(
        m,
        tri,
        [0, 1, 2].map(|i| [-w * (tri[i][1] - rc[1]), w * (tri[i][0] - rc[0])]),
    );
    // Random: positions in [−1, 1)², velocities in [−0.3, 0.3)², masses in [0.5, 1.5).
    let mut g = Rng(0x7a5c_3e91_04d2_b68f);
    let mr = [0, 1, 2].map(|_| 1.0 + 0.5 * g.unit());
    let rr = [0, 1, 2].map(|_| [g.unit(), g.unit()]);
    let vr = [0, 1, 2].map(|_| [0.3 * g.unit(), 0.3 * g.unit()]);
    vec![hier, lag, com_state(mr, rr, vr)]
}

/// `s` at `R`.
fn cast<R: P>(s: &State<f64>) -> State<R> {
    State {
        m: s.m.map(R::of),
        r: s.r.map(|v| v.map(R::of)),
        p: s.p.map(|v| v.map(R::of)),
    }
}

/// A Cartesian offset of norm `delta_0` over the twelve components, in a seeded direction (§3.8's `‖δ₀‖·u`, `u` any
/// unit vector).
fn offset<R: P>(m: [R; 3], delta_0: f64, seed: u64) -> State<R> {
    let mut g = Rng(seed);
    let raw: Vec<f64> = (0..12).map(|_| g.unit()).collect();
    let n = raw.iter().map(|x| x * x).sum::<f64>().sqrt();
    let c = |i: usize| R::of(delta_0 * raw[i] / n);
    State {
        m,
        r: [0, 1, 2].map(|i| [c(2 * i), c(2 * i + 1)]),
        p: [0, 1, 2].map(|i| [c(6 + 2 * i), c(6 + 2 * i + 1)]),
    }
}

/// A state's twelve Cartesian components at f64.
fn flat<R: P>(s: &State<R>) -> Vec<f64> {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .map(|&x| x.into())
        .collect()
}

fn norm(x: &[f64]) -> f64 {
    x.iter().map(|v| v * v).sum::<f64>().sqrt()
}

/// The shadow's system: the sample's at the shadow's own frozen energy `h + δh` (§3.8, "The displacement").
fn shadow_system<R: P>(system: &HgSystem<R>, d: &HgDisplacement<R>) -> HgSystem<R> {
    HgSystem {
        h: system.h + d.h,
        ..*system
    }
}

/// The absolute shadow `y + δy`.
fn absolute<R: P>(q: &HgQ<R>, p: &HgP<R>, d: &HgDisplacement<R>) -> (HgQ<R>, HgP<R>) {
    (
        HgSystem::q_add(q, R::one(), &d.q),
        HgSystem::p_add(p, R::one(), &d.p),
    )
}

/// Renormalisations per run, and joint steps between them.
const RENORMS: usize = 40;
const INTERVAL: usize = 25;
/// The physical step the constant `Δτ` is sized to at the start.
const PHYS_STEP0: f64 = 1e-3;

// ── qa_m374_same_time_bitwise ────────────────────────────────────────────────────────────────────────────────────

/// Whose clock the check reads as the shadow's.
#[derive(Clone, Copy)]
enum Shadow {
    /// The joint step's: the sample's clock plus the renormalised displacement's `δt`.
    Joint,
    /// REQ-VAL-355's control: an absolute shadow restarted at each renormalisation from `y + δy` and advanced by the
    /// sample's `Δτ` alone.
    #[cfg(feature = "controls")]
    DtauAlone,
}

/// At `R`, under `time`, a joint-step run of each fixture, `‖δ₀‖ = 1e-5`, a constant `Δτ`, renormalised every
/// [`INTERVAL`] steps: at each renormalisation the shadow's elapsed physical time since the previous one must be the
/// sample's, bitwise; before the renormalisations the clocks must have moved apart at least once.
fn check_same_time<R: P>(time: HeggieTime, who: Shadow) {
    for (k, s) in fixtures().iter().enumerate() {
        let HgRegistered {
            system,
            mut q,
            mut p,
        } = register(&cast::<R>(s), R::of(1e-30), time);
        let delta_0 = 1e-5;
        let mut d = start(
            &system,
            &q,
            &p,
            &offset(system.m, delta_0, 0x9e37 + k as u64),
        );
        let dtau = R::of(PHYS_STEP0) / system.dt_dtau(&q);
        let (mut sq, mut sp) = absolute(&q, &p, &d);
        let mut shadow = shadow_system(&system, &d);
        let mut t_prev = q.t;
        let mut apart = false;
        for _ in 0..RENORMS {
            for _ in 0..INTERVAL {
                (sq, sp) = Form::Rk4.step(&shadow, &sq, &sp, dtau);
                (q, p, d) = joint_step(&system, &q, &p, &d, dtau);
            }
            apart |= d.q.t != R::default();
            (d, _) = renormalise(&system, &q, &p, &d, R::of(delta_0));
            let shadow_elapsed = match who {
                Shadow::Joint => (q.t + d.q.t) - t_prev,
                #[cfg(feature = "controls")]
                Shadow::DtauAlone => sq.t - t_prev,
            };
            let sample_elapsed = q.t - t_prev;
            assert!(
                shadow_elapsed.into().to_bits() == sample_elapsed.into().to_bits(),
                "fixture {k}: the shadow's elapsed physical time {:e} is not the sample's {:e}",
                shadow_elapsed.into(),
                sample_elapsed.into()
            );
            t_prev = q.t;
            (sq, sp) = absolute(&q, &p, &d);
            shadow = shadow_system(&system, &d);
        }
        assert!(apart, "fixture {k}: the clocks never moved apart");
    }
}

#[test]
fn qa_m374_same_time_bitwise() {
    for time in [DEFAULT_TIME, HeggieTime::Eq20] {
        check_same_time::<f32>(time, Shadow::Joint);
        check_same_time::<f64>(time, Shadow::Joint);
    }
}

negative_control!(
    qa_m374_same_time_bitwise,
    "the shadow advanced by the sample's Δτ alone drifts from the sample's physical time",
    expected = "is not the sample's",
    check_same_time::<f64>(DEFAULT_TIME, Shadow::DtauAlone)
);

// ── qa_m374_same_time_order ──────────────────────────────────────────────────────────────────────────────────────

/// Where the reference separation is read.
#[derive(Clone, Copy)]
enum Reference {
    /// The absolute shadow brought to the sample's physical time.
    SampleTime,
    /// The control: the absolute shadow where the sample's `Δτ`s alone left it.
    #[cfg(feature = "controls")]
    OwnDtau,
}

/// At f64, the default time transformation, `‖δ₀‖ = delta_0`: the largest relative miss, over every renormalisation of
/// a run of each fixture, between the resynchronised `J·δy` and the absolute shadow's Cartesian state less the
/// sample's, the shadow read where `reference` says.
fn worst_miss(delta_0: f64, reference: Reference) -> f64 {
    let mut worst: f64 = 0.0;
    for (k, s) in fixtures().iter().enumerate() {
        let HgRegistered {
            system,
            mut q,
            mut p,
        } = register(s, 1e-30, DEFAULT_TIME);
        let mut d = start(
            &system,
            &q,
            &p,
            &offset(system.m, delta_0, 0x51ed + k as u64),
        );
        let dtau = PHYS_STEP0 / system.dt_dtau(&q);
        for _ in 0..RENORMS / 4 {
            let shadow = shadow_system(&system, &d);
            let (mut sq, mut sp) = absolute(&q, &p, &d);
            for _ in 0..INTERVAL {
                (sq, sp) = Form::Rk4.step(&shadow, &sq, &sp, dtau);
                (q, p, d) = joint_step(&system, &q, &p, &d, dtau);
            }
            if matches!(reference, Reference::SampleTime) {
                // Newton on the shadow's clock: three RK4 steps of `(t − t')/g` land it on `t` to round-off.
                for _ in 0..3 {
                    let back = (q.t - sq.t) / shadow.dt_dtau(&sq);
                    (sq, sp) = Form::Rk4.step(&shadow, &sq, &sp, back);
                }
            }
            let sep: Vec<f64> = flat(&shadow.image(&sq, &sp))
                .iter()
                .zip(flat(&system.image(&q, &p)))
                .map(|(a, b)| a - b)
                .collect();
            let read = flat(&cartesian(
                &system,
                &q,
                &p,
                &resynchronise(&system, &q, &p, &d),
            ));
            let miss: Vec<f64> = read.iter().zip(&sep).map(|(a, b)| a - b).collect();
            worst = worst.max(norm(&miss) / norm(&sep));
            (d, _) = renormalise(&system, &q, &p, &d, delta_0);
        }
    }
    worst
}

/// The observed order of the miss in `δ₀` over a decade, `1e-4` to `1e-5`, must be at least `0.8`: §3.8's rules are
/// first order, the dropped terms second, so the relative miss falls as `δ`.
fn check_order(reference: Reference) {
    let coarse = worst_miss(1e-4, reference);
    let fine = worst_miss(1e-5, reference);
    assert!(fine > 0.0, "the miss is zero: the comparison cannot fire");
    let order = (coarse / fine).log10();
    eprintln!("misses at δ₀ 1e-4, 1e-5: {coarse:.3e}, {fine:.3e}; observed order {order:.3}");
    assert!(
        order >= 0.8,
        "the shadow's separation at the sample's time is not matched to first order: misses {coarse:.3e}, {fine:.3e}, \
         order {order:.3}"
    );
}

#[test]
fn qa_m374_same_time_order() {
    check_order(Reference::SampleTime);
}

negative_control!(
    qa_m374_same_time_order,
    "the reference read where the sample's Δτ alone left the shadow, not at the sample's time",
    expected = "is not matched to first order",
    check_order(Reference::OwnDtau)
);
