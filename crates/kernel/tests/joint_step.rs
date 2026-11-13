//! The FTLE shadow's joint step (R-974, RQ-793 (a); dd_integrator §3.8; integrator_contract Part 2, Part 2a;
//! `kernel::occupant::Stepper::joint_step`, `kernel::driver::advance::Advance::advance_joint`,
//! `kernel::physics::difference`; TASK-M3-72):
//! - `joint_step_sample_bitwise`: every stepper's joint step leaves the sample's state and force evaluation exactly
//!   its `STEP`'s, bit for bit, in both forms of the force difference, at f32 and f64, on states of several orbits and
//!   seeded random states.
//! - `joint_step_difference`: the displacement a joint step leaves is the absolute shadow's step less the sample's,
//!   `STEP(x + δ⃗) − STEP(x)`, exactly but for rounding (every stage's update is linear in `δ⃗` but the force, whose
//!   difference is formed exactly, not linearised): at f64, at `‖δ⃗‖` of 1e-3 and 1e-4, within the absolute
//!   difference's own rounding, `ε·‖x‖/‖δ⃗‖`, with a wide margin.
//! - `force_difference_forms`: (a2)'s `a(r + δr) − a(r)` keeps its relative precision at f32 however small `δr` is
//!   against the separations, read against the same difference at f64 from the absolute positions; (a1) and (a2) agree
//!   to f64 rounding where `δr` is not small.
//! - `joint_march_replays_sample`: a stepper's joint march, under a cadence that projects the sample as the wrapper's
//!   does, leaves the sample's state that of its `ADVANCE` under the wrapper's cadence, bit for bit, macro-step by
//!   macro-step.
//!
//! Each test has a registered negative control naming the panic it expects (R-176).

use kernel::driver::advance::{Advance, AdvanceParams, JointCadence};
use kernel::driver::cadence::WrapperCadence;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{Geom, State, StepReal, Stepper};
use kernel::physics::difference::{CancellationFree, ForceDifference, Plain};
use kernel::physics::PairGeom;
use kernel::Real;
use validation::measure::benettin::{burrau, figure_eight, lagrange, Orbit};
use validation::negative_control;

/// Bits of a state's positions and momenta.
fn bits<R: Into<f64> + Copy>(s: &State<R>) -> Vec<u64> {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .map(|&x| x.into().to_bits())
        .collect()
}

/// A seeded pseudo-random number in `[−1, 1)` (a 64-bit LCG), so the cases are fixed.
fn next(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
}

/// The fixture states, at f64: three orbits' ICs and eight seeded random states of positive masses, positions in a
/// unit box and momenta of order 0.3.
fn states() -> Vec<State<f64>> {
    let mut out: Vec<State<f64>> = [
        burrau(),
        figure_eight(1.0),
        lagrange("Lagrange", [0.5, 0.3, 0.2], 1.0),
    ]
    .iter()
    .map(|o: &Orbit| State {
        m: o.ic.m,
        r: o.ic.r,
        p: o.ic.p,
    })
    .collect();
    let mut seed = 0x5eed_u64;
    for _ in 0..8 {
        let m = [0, 1, 2].map(|_| 0.2 + 0.4 * (next(&mut seed) + 1.0));
        let r = [0, 1, 2].map(|_| [next(&mut seed), next(&mut seed)]);
        let p = [0, 1, 2].map(|_| [0.3 * next(&mut seed), 0.3 * next(&mut seed)]);
        out.push(State { m, r, p });
    }
    out
}

/// A displacement of size `size` in a seeded direction, positions and momenta.
fn displacement(m: [f64; 3], size: f64, seed: &mut u64) -> State<f64> {
    State {
        m,
        r: [0, 1, 2].map(|_| [size * next(seed), size * next(seed)]),
        p: [0, 1, 2].map(|_| [size * next(seed), size * next(seed)]),
    }
}

/// `s` at `R`.
fn at<R: Real>(s: &State<f64>, f: fn(f64) -> R) -> State<R> {
    State {
        m: s.m.map(f),
        r: s.r.map(|v| v.map(f)),
        p: s.p.map(|v| v.map(f)),
    }
}

// ── joint_step_sample_bitwise ───────────────────────────────────────────────────────────────────────────────────

/// A stepper's `STEP` at `R`.
type Step<R> = fn(&State<R>, R, &Geom<R>) -> (State<R>, Geom<R>);

/// One stepper's joint step against `step`, a `STEP` it must reproduce, at `R`, in the force difference `D`.
fn check_bitwise_at<S: Stepper, D: ForceDifference, R: StepReal + Into<f64> + From<f32>>(
    name: &str,
    step: Step<R>,
    f: fn(f64) -> R,
) {
    let mut seed = 7u64;
    for (k, s64) in states().iter().enumerate() {
        let s = at(s64, f);
        let d = at(&displacement(s64.m, 1e-4, &mut seed), f);
        let g = Geom::new(&s.m, &s.r);
        let dt = f(1e-3);
        let (x, gx, _) = S::joint_step::<R, D>(&s, &d, dt, &g);
        let (y, gy) = step(&s, dt, &g);
        assert!(
            bits(&x) == bits(&y) && gx == gy,
            "{name}, state {k}: the joint step's sample is not its STEP's, bit for bit"
        );
    }
}

/// Every stepper, both differences, both precisions; `steps` the `STEP`s each must reproduce, in the order KDK,
/// Yoshida-4, Yoshida-6, RK4, Euler.
fn check_bitwise(steps32: [Step<f32>; 5], steps64: [Step<f64>; 5]) {
    let f32_of = |x: f64| x as f32;
    let f64_of = |x: f64| x;
    macro_rules! each {
        ($d:ty) => {
            check_bitwise_at::<Kdk, $d, f32>("KDK f32", steps32[0], f32_of);
            check_bitwise_at::<Yoshida4, $d, f32>("Yoshida-4 f32", steps32[1], f32_of);
            check_bitwise_at::<Yoshida6, $d, f32>("Yoshida-6 f32", steps32[2], f32_of);
            check_bitwise_at::<Rk4, $d, f32>("RK4 f32", steps32[3], f32_of);
            check_bitwise_at::<Euler, $d, f32>("Euler f32", steps32[4], f32_of);
            check_bitwise_at::<Kdk, $d, f64>("KDK f64", steps64[0], f64_of);
            check_bitwise_at::<Yoshida4, $d, f64>("Yoshida-4 f64", steps64[1], f64_of);
            check_bitwise_at::<Yoshida6, $d, f64>("Yoshida-6 f64", steps64[2], f64_of);
            check_bitwise_at::<Rk4, $d, f64>("RK4 f64", steps64[3], f64_of);
            check_bitwise_at::<Euler, $d, f64>("Euler f64", steps64[4], f64_of);
        };
    }
    each!(Plain);
    each!(CancellationFree);
}

#[test]
fn joint_step_sample_bitwise() {
    check_bitwise(
        [
            Kdk::step,
            Yoshida4::step,
            Yoshida6::step,
            Rk4::step,
            Euler::step,
        ],
        [
            Kdk::step,
            Yoshida4::step,
            Yoshida6::step,
            Rk4::step,
            Euler::step,
        ],
    );
}

negative_control!(
    joint_step_sample_bitwise,
    "a joint step held against another occupant's STEP must fail",
    expected = "the joint step's sample is not its STEP's",
    check_bitwise(
        [
            Yoshida4::step,
            Yoshida4::step,
            Yoshida6::step,
            Rk4::step,
            Euler::step
        ],
        [
            Kdk::step,
            Yoshida4::step,
            Yoshida6::step,
            Rk4::step,
            Euler::step
        ],
    )
);

// ── joint_step_difference ───────────────────────────────────────────────────────────────────────────────────────

/// A force difference that is always zero: a joint step that ignores the force's change (the control's).
#[cfg(feature = "controls")]
struct Zero;

#[cfg(feature = "controls")]
impl ForceDifference for Zero {
    fn term<R: Real>(_d: [R; 2], _d2: R, _norm: R, _cube: R, _e: [R; 2]) -> [R; 2] {
        [R::default(); 2]
    }
}

/// A force difference that is never finite: a joint step whose displacement is NaN (the non-finite tests' input).
struct NotFinite;

impl ForceDifference for NotFinite {
    fn term<R: Real>(_d: [R; 2], d2: R, _norm: R, _cube: R, _e: [R; 2]) -> [R; 2] {
        // `√(−‖d‖²)`, NaN for every separation that is not zero.
        [(-d2).square_root(); 2]
    }
}

/// The larger of `acc` and `v`, a non-finite `v` counting as infinite: `f64::max` drops a NaN, and no reduction
/// discards a non-finite sample silently (Totality, R-47; R-538).
fn worst(acc: f64, v: f64) -> f64 {
    if v.is_finite() {
        acc.max(v)
    } else {
        f64::INFINITY
    }
}

/// The largest component of `a − b` over the twelve, and the largest of `b`; either is infinite where a component
/// it reads is not finite.
fn residual(a: &State<f64>, b: &State<f64>) -> (f64, f64) {
    let mut res = 0.0f64;
    let mut scale = 0.0f64;
    for i in 0..3 {
        for k in 0..2 {
            res = worst(
                worst(res, (a.r[i][k] - b.r[i][k]).abs()),
                (a.p[i][k] - b.p[i][k]).abs(),
            );
            scale = worst(worst(scale, b.r[i][k].abs()), b.p[i][k].abs());
        }
    }
    (res, scale)
}

/// `STEP(x + δ⃗) − STEP(x)` at f64.
fn absolute_difference<S: Stepper>(x: &State<f64>, d: &State<f64>, dt: f64) -> State<f64> {
    let shadow = State {
        m: x.m,
        r: [0, 1, 2].map(|i| [x.r[i][0] + d.r[i][0], x.r[i][1] + d.r[i][1]]),
        p: [0, 1, 2].map(|i| [x.p[i][0] + d.p[i][0], x.p[i][1] + d.p[i][1]]),
    };
    let (a, _) = S::step(&shadow, dt, &Geom::new(&shadow.m, &shadow.r));
    let (b, _) = S::step(x, dt, &Geom::new(&x.m, &x.r));
    State {
        m: x.m,
        r: [0, 1, 2].map(|i| [a.r[i][0] - b.r[i][0], a.r[i][1] - b.r[i][1]]),
        p: [0, 1, 2].map(|i| [a.p[i][0] - b.p[i][0], a.p[i][1] - b.p[i][1]]),
    }
}

/// The joint step's displacement against the absolute difference, relative to its size, at `δ⃗` of size `size` on
/// every fixture state, the worst over them; infinite where any is not finite.
fn linear_residual<S: Stepper, D: ForceDifference>(size: f64) -> f64 {
    let mut seed = 11u64;
    let dt = 1e-2;
    states()
        .iter()
        .map(|x| {
            let d = displacement(x.m, size, &mut seed);
            let (_, _, joint) = S::joint_step::<f64, D>(x, &d, dt, &Geom::new(&x.m, &x.r));
            let (res, scale) = residual(&joint, &absolute_difference::<S>(x, &d, dt));
            res / scale
        })
        .fold(0.0, worst)
}

/// For one stepper and difference: at each size, the relative residual is within `10⁴·ε/‖δ⃗‖`, the absolute
/// difference's rounding (`ε·‖x‖/‖δ⃗‖`, `‖x‖` of order 1) with a margin of 10⁴; a missing force difference leaves a
/// residual of order `dt·‖∂a‖`, about 1e-2 here.
fn check_linear_one<S: Stepper, D: ForceDifference>(name: &str) {
    for size in [1e-3, 1e-4] {
        let res = linear_residual::<S, D>(size);
        assert!(
            res <= 1e4 * f64::EPSILON / size,
            "{name}: the joint step's displacement is not the absolute difference (relative residual {res:e} at \
             ‖δ⃗‖ = {size:e})"
        );
    }
}

fn check_linear<D: ForceDifference>() {
    check_linear_one::<Kdk, D>("KDK");
    check_linear_one::<Yoshida4, D>("Yoshida-4");
    check_linear_one::<Yoshida6, D>("Yoshida-6");
    check_linear_one::<Rk4, D>("RK4");
    check_linear_one::<Euler, D>("Euler");
}

#[test]
fn joint_step_difference() {
    check_linear::<Plain>();
    check_linear::<CancellationFree>();
}

negative_control!(
    joint_step_difference,
    "a joint step that ignores the force's change must fail",
    expected = "not the absolute difference",
    check_linear::<Zero>()
);

/// For one stepper and difference: the relative residual is infinite, a failure of `check_linear_one`, never
/// agreement.
fn check_not_finite_one<S: Stepper, D: ForceDifference>(name: &str) {
    let res = linear_residual::<S, D>(1e-3);
    assert!(
        res == f64::INFINITY,
        "{name}: a non-finite displacement reads as relative residual {res:e}, not as a failure"
    );
}

fn check_not_finite<D: ForceDifference>() {
    check_not_finite_one::<Kdk, D>("KDK");
    check_not_finite_one::<Yoshida4, D>("Yoshida-4");
    check_not_finite_one::<Yoshida6, D>("Yoshida-6");
    check_not_finite_one::<Rk4, D>("RK4");
    check_not_finite_one::<Euler, D>("Euler");
}

/// `joint_step_difference`'s reduction keeps a NaN displacement: it reads as an infinite residual, which fails the
/// check, where `f64::max` would drop it and read agreement.
#[test]
fn joint_step_difference_not_finite() {
    check_not_finite::<NotFinite>();
}

negative_control!(
    joint_step_difference_not_finite,
    "a finite displacement read as non-finite must fail",
    expected = "not as a failure",
    check_not_finite::<Plain>()
);

// ── force_difference_forms ─────────────────────────────────────────────────────────────────────────────────────

/// `a(r + δr) − a(r)` at f64 from the absolute positions: the reference.
fn reference(m: &[f64; 3], r: &[[f64; 2]; 3], dr: &[[f64; 2]; 3]) -> [[f64; 2]; 3] {
    let moved = [0, 1, 2].map(|i| [r[i][0] + dr[i][0], r[i][1] + dr[i][1]]);
    let a = Geom::new(m, &moved).accel;
    let b = Geom::new(m, r).accel;
    [0, 1, 2].map(|i| [a[i][0] - b[i][0], a[i][1] - b[i][1]])
}

/// The largest relative error of `D`'s difference at f32 against the reference, over the fixture states, at
/// displacements of size `size`. The reference's own error, `ε₆₄·‖a‖/‖Δa‖`, is far below f32's at every size used.
fn f32_error<D: ForceDifference>(size: f64) -> f64 {
    let mut seed = 3u64;
    states()
        .iter()
        .map(|x| {
            let d = displacement(x.m, size, &mut seed);
            // The f64 reference is taken at the f32 inputs, so only the arithmetic differs.
            let m32 = x.m.map(|v| v as f32);
            let r32 = x.r.map(|v| v.map(|c| c as f32));
            let d32 = d.r.map(|v| v.map(|c| c as f32));
            let got = D::accel(&m32, &PairGeom::new(&r32), &d32);
            let want = reference(
                &m32.map(f64::from),
                &r32.map(|v| v.map(f64::from)),
                &d32.map(|v| v.map(f64::from)),
            );
            let scale = want.iter().flatten().fold(0.0f64, |a, &v| a.max(v.abs()));
            let err = (0..3)
                .flat_map(|i| (0..2).map(move |k| (i, k)))
                .map(|(i, k)| (f64::from(got[i][k]) - want[i][k]).abs())
                .fold(0.0f64, f64::max);
            err / scale
        })
        .fold(0.0, f64::max)
}

/// (a2) at f32 keeps its relative error within `bound` at every size from 1e-2 to 1e-7.
fn check_cancellation_free<D: ForceDifference>(bound: f64) {
    for size in [1e-2, 1e-4, 1e-5, 1e-6, 1e-7] {
        let e = f32_error::<D>(size);
        assert!(
            e <= bound,
            "at ‖δr‖ ≈ {size:e} the f32 difference's relative error is {e:e}, above {bound:e}: it loses precision as \
             δr shrinks"
        );
    }
}

/// `1024·ε₃₂`: rounding over the few dozen operations of a pair's term and the combination, with room for the
/// masses' spread; (a1)'s loss at `‖δr‖ = 1e-6` is `ε₃₂/1e-6 ≈ 0.1`, a thousand times it.
const F32_BOUND: f64 = 1024.0 * f32::EPSILON as f64;

#[test]
fn force_difference_forms() {
    check_cancellation_free::<CancellationFree>(F32_BOUND);
    // At f64 and a displacement that is not small, the two forms are one quantity.
    let mut seed = 5u64;
    for x in states() {
        let d = displacement(x.m, 1e-2, &mut seed);
        let g = PairGeom::new(&x.r);
        let (a1, a2) = (
            Plain::accel(&x.m, &g, &d.r),
            CancellationFree::accel(&x.m, &g, &d.r),
        );
        let scale = a2.iter().flatten().fold(0.0f64, |a, &v| a.max(v.abs()));
        for i in 0..3 {
            for k in 0..2 {
                assert!(
                    (a1[i][k] - a2[i][k]).abs() <= 1e-10 * scale,
                    "(a1) and (a2) differ at f64: {} against {}",
                    a1[i][k],
                    a2[i][k]
                );
            }
        }
    }
}

negative_control!(
    force_difference_forms,
    "(a1), the plain difference, held to (a2)'s bound must fail",
    expected = "it loses precision as δr shrinks",
    check_cancellation_free::<Plain>(F32_BOUND)
);

// ── joint_march_replays_sample ─────────────────────────────────────────────────────────────────────────────────

/// A joint cadence that projects the sample as the wrapper's cadence does (`project`), or not.
struct Replay {
    cadence: WrapperCadence<f64>,
    project: bool,
}

impl JointCadence<f64> for Replay {
    fn after_step(&mut self, x: &mut State<f64>, geom: &mut Geom<f64>, _delta: &mut State<f64>) {
        if self.project {
            self.cadence.project(x, geom);
        }
    }

    fn done(&self) -> bool {
        false
    }
}

/// `macro_steps` macro-steps of `orbit` by `A`'s `ADVANCE` under the wrapper's cadence and by its joint march under
/// [`Replay`], from the same states: the samples are bit for bit the same after every one.
fn check_replay<A: Advance>(name: &str, project: bool) {
    let o = burrau();
    let mut x = State {
        m: o.ic.m,
        r: o.ic.r,
        p: o.ic.p,
    };
    let mut g = Geom::new(&x.m, &x.r);
    let mut d = displacement(x.m, 1e-5, &mut 9u64);
    let mut wrapper = WrapperCadence::new(&x.m, &x.r, &x.p);
    let mut replay = Replay {
        cadence: WrapperCadence::new(&x.m, &x.r, &x.p),
        project,
    };
    for n in 0..200u32 {
        let params = AdvanceParams {
            dt_macro: 1e-3,
            n_sub: 1 + n % 3,
        };
        let plain = A::advance(&x, &g, n, n + 1, &params, &mut wrapper);
        let (joint, delta) =
            A::advance_joint::<f64, Plain, _>(&x, &g, &d, n, n + 1, &params, &mut replay);
        assert!(
            bits(&plain.state) == bits(&joint.state)
                && plain.geom == joint.geom
                && plain.substeps == joint.substeps,
            "{name}, macro-step {n}: the joint march's sample is not its ADVANCE's, bit for bit"
        );
        x = plain.state;
        g = plain.geom;
        d = delta;
    }
}

fn check_replays(project: bool) {
    check_replay::<Kdk>("KDK", project);
    check_replay::<Yoshida4>("Yoshida-4", project);
    check_replay::<Yoshida6>("Yoshida-6", project);
    check_replay::<Rk4>("RK4", project);
}

#[test]
fn joint_march_replays_sample() {
    check_replays(true);
}

negative_control!(
    joint_march_replays_sample,
    "a joint march that skips the sample's projection must fail",
    expected = "the joint march's sample is not its ADVANCE's",
    check_replays(false)
);
