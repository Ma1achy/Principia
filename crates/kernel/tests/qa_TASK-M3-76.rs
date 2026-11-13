//! QA tests for TASK-M3-76, written from REQ-INT-119 and REQ-INT-120 and their sources (R-998, R-999, R-998's dated
//! line, and integrator contract Part 2b § "Aarseth–Zare: the equations, the reference body, the Levi-Civita inverse
//! and the step control (transcribed)"), not from the implementation:
//!
//! - REQ-INT-119 (`qa_az_landing_state_with_clock_fuzzed`): over fuzzed three-body states, three step factors and
//!   three sync quanta, at f32 and f64, Aarseth–Zare + RK4 marched over three sync intervals lands on every boundary
//!   with "the state landing on the boundary together with the clock": in every interval the last kept regularised
//!   state's own clock is `Δ_sync`, bitwise, and no kept state's clock is past it (the clock "never set to the boundary
//!   while the stored state's own clock differs from it", R-998's dated line, with no tolerance in which a clock short
//!   of or past `Δ_sync` could land); the march's time is `k·Δ_sync` exactly; and its Cartesian state is, bit for bit,
//!   the image of that last kept state under the last interval's registration. A stepper spy counts the steps that
//!   ended the clock past the boundary, so the fixtures are shown to exercise the landing (an overshooting step), at
//!   each precision.
//! - REQ-INT-119 (`qa_az_landing_state_with_clock_truth`): ground truth from outside the occupant, a Cartesian RK4
//!   integration of the same three bodies in physical time at a step 1/20000 of the sync quantum: after four landed
//!   intervals the occupant's state is nearer the truth than the clock-only landing's (the clamp kept, no retakes, the
//!   clock set to the boundary over the overshot state, what R-998 forbids), whose run is shown to keep a clock past
//!   the boundary.
//! - REQ-INT-120 (`qa_az_r3_floor_tiny`): the equations of motion's `|R₃|` reads `max(|R₃|, TINY)`, `TINY` the working
//!   precision's (`1e-37` at f32, `1e-300` at f64, the reference's values, R-953): on states built with `|R₃|` exactly
//!   `0` and with `0 < |R₃| < TINY` it reads `TINY`; above `TINY` it reads `|R₂ − R₁|` from a Levi-Civita map written
//!   here.
//!
//! Negative controls (R-176): the clock-only landing (the secant's and the bisection's caps both 0, the contract's
//! measurement setting) fails the fuzzed check and the truth check; a floor of 0 at f32 fails the floor check on the
//! state with `|R₃|` below `TINY`.

use std::cell::Cell;

use kernel::driver::advance::Cadence;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{CapabilityProfile, Geom, State};
use kernel::regularisation::az::{
    march, register, AzAdvanced, AzCount, AzP, AzParams, AzQ, AzReal, AzStepper, AzSystem, End,
    ExplicitTest, Watch,
};
use validation::negative_control;

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

/// Test step-control values (inputs, not the calibrations): `η` and `Δ_sync` as given, the reference's `TINY`
/// (`1e-37` / `1e-300`, R-953), R-160's `f = 0.02`, a budget that a stalled landing would spend, the clamp on and the
/// landing's two integer caps as Part 2b writes them (the secant's 4, the bisection's 64).
fn params<R: P>(eta: f64, delta: f64) -> AzParams<R> {
    AzParams {
        eta: R::of(eta),
        delta_sync: R::of(delta),
        tiny: R::at_width(1e-37, 1e-300),
        predictive_f: R::of(0.02),
        step_budget_rate: R::of(1e7),
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
    }
}

/// The landing's retakes as the occupant has them (the settings under test).
fn retakes_on<R>(_p: &mut AzParams<R>) {}

/// The clock-only landing: the clamp kept, the secant's and the bisection's caps both 0, so a step past the boundary
/// is kept and the clock set to the boundary alone (the contract's measurement setting; what R-998 forbids).
#[cfg(feature = "controls")]
fn clock_only<R>(p: &mut AzParams<R>) {
    p.secant_tries = 0;
    p.bisect_tries = 0;
}

/// A cadence that keeps nothing and never ends the march.
struct Quiet;

impl<R> Cadence<R> for Quiet {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {}
    fn done(&self) -> bool {
        false
    }
}

/// The occupant's own finiteness test, recording every kept state and every registration, in order.
struct Record<R> {
    /// Per interval (opened by its registration), the kept states.
    intervals: Vec<Vec<(AzQ<R>, AzP<R>)>>,
}

impl<R> Record<R> {
    fn new() -> Self {
        Record {
            intervals: Vec::new(),
        }
    }
}

impl<R: AzReal> Watch<R> for Record<R> {
    fn finite(&mut self, q: &AzQ<R>, p: &AzP<R>) -> bool {
        let ok = ExplicitTest.finite(q, p);
        if ok {
            if let Some(last) = self.intervals.last_mut() {
                last.push((*q, *p));
            }
        }
        ok
    }

    fn registered(&mut self, _q: &AzQ<R>) {
        self.intervals.push(Vec::new());
    }
}

thread_local! {
    /// The boundary the spy compares each step's clock with, `(f32, f64)`.
    static SPY_DELTA: Cell<(f32, f64)> = const { Cell::new((0.0, 0.0)) };
    /// The steps whose clock ended past the boundary.
    static SPY_PAST: Cell<u32> = const { Cell::new(0) };
}

/// RK4 in the slot, counting the steps whose clock ends past `SPY_DELTA`.
struct Spy;

impl AzStepper for Spy {
    fn pair_profile() -> CapabilityProfile {
        <Rk4 as AzStepper>::pair_profile()
    }

    fn regularised_step<R: AzReal>(
        system: &AzSystem<R>,
        q: &AzQ<R>,
        p: &AzP<R>,
        h: R,
    ) -> (AzQ<R>, AzP<R>) {
        let out = <Rk4 as AzStepper>::regularised_step(system, q, p, h);
        let (d32, d64) = SPY_DELTA.with(Cell::get);
        if out.0.t > R::at_width(d32, d64) {
            SPY_PAST.with(|c| c.set(c.get() + 1));
        }
        out
    }
}

/// A deterministic generator of fixture values (64-bit LCG, Knuth's constants), uniform in `[lo, hi)`.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, lo: f64, hi: f64) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let u = (self.0 >> 11) as f64 / (1u64 << 53) as f64;
        lo + (hi - lo) * u
    }
}

/// Fuzzed three-body states at f64: masses in `[0.5, 1.5]`, positions in `[−1, 1]²` at least `0.4` apart, velocities
/// in `[−0.5, 0.5]²`, the total momentum removed.
fn fuzzed(n: usize) -> Vec<State<f64>> {
    let mut g = Lcg(0x005e_ed76);
    let mut out = Vec::new();
    while out.len() < n {
        let m = [0, 1, 2].map(|_| g.next(0.5, 1.5));
        let r = [0, 1, 2].map(|_| [g.next(-1.0, 1.0), g.next(-1.0, 1.0)]);
        let far =
            (0..3).all(|i| (i + 1..3).all(|j| (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]) >= 0.4));
        if !far {
            continue;
        }
        let mut p = [0, 1, 2].map(|i| [m[i] * g.next(-0.5, 0.5), m[i] * g.next(-0.5, 0.5)]);
        let total = [0, 1].map(|c| p.iter().map(|x| x[c]).sum::<f64>() / 3.0);
        for x in &mut p {
            x[0] -= total[0];
            x[1] -= total[1];
        }
        out.push(State { m, r, p });
    }
    out
}

fn at<R: P>(s: &State<f64>) -> State<R> {
    State {
        m: s.m.map(R::of),
        r: s.r.map(|x| x.map(R::of)),
        p: s.p.map(|x| x.map(R::of)),
    }
}

fn bits<R: P>(s: &State<R>) -> Vec<u64> {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .map(|&x| Into::<f64>::into(x).to_bits())
        .collect()
}

/// The march from boundary 0 to `k` with the spy in the slot and the record as its watch.
fn spied<R: P>(s: &State<R>, k: u32, prm: &AzParams<R>) -> (AzAdvanced<R>, Record<R>) {
    let mut rec = Record::new();
    let out = march::<Spy, R, Quiet, Record<R>>(s, 0, k, AzCount::new(), prm, &mut Quiet, &mut rec);
    (out, rec)
}

// ── qa_az_landing_state_with_clock_fuzzed ────────────────────────────────────────────────────────────────────────

/// The boundaries each fuzzed march lands on.
const K: u32 = 3;

/// REQ-INT-119 over the fuzzed states at `R`, with `landing` setting the landing's retakes: see the module's note.
/// Returns the steps that ended the clock past a boundary.
fn check_fuzzed<R: P>(landing: fn(&mut AzParams<R>)) -> u32 {
    let mut past = 0;
    for (i, s64) in fuzzed(16).iter().enumerate() {
        let s = at::<R>(s64);
        for eta in [0.3, 0.05, 0.01] {
            for delta in [0.125, 0.1, 0.3] {
                let mut prm = params::<R>(eta, delta);
                landing(&mut prm);
                let d = prm.delta_sync;
                SPY_DELTA.with(|c| c.set((delta as f32, delta)));
                SPY_PAST.with(|c| c.set(0));
                let (out, rec) = spied(&s, K, &prm);
                past += SPY_PAST.with(Cell::get);
                let at_ = format!("state {i}, η {eta}, Δ_sync {delta}");
                assert_eq!(
                    (out.count.end, out.boundary),
                    (End::Marching, K),
                    "the march lands on every boundary ({at_}): {:?}",
                    out.count
                );
                assert_eq!(
                    rec.intervals.len(),
                    K as usize,
                    "one registration per interval ({at_})"
                );
                for (j, kept) in rec.intervals.iter().enumerate() {
                    let clocks: Vec<f64> = kept.iter().map(|(q, _)| q.t.into()).collect();
                    let (last, _) = kept
                        .last()
                        .expect("an interval keeps its registration's state");
                    assert!(
                        last.t == d && kept.iter().all(|(q, _)| q.t <= d),
                        "the stored state's own clock is the boundary, never past it ({at_}, interval {j}): {clocks:?}"
                    );
                }
                assert!(
                    out.t == R::count(K) * d,
                    "the march's time is the boundary's, k·Δ_sync, exactly ({at_}): {}",
                    Into::<f64>::into(out.t)
                );
                // The last interval's registration: the state the march stored at boundary K − 1.
                let (before, _) = spied(&s, K - 1, &prm);
                let (sys, _, _) = register(&before.state, prm.tiny);
                let (q, p) = rec.intervals[K as usize - 1].last().copied().expect("kept");
                assert_eq!(
                    bits(&out.state),
                    bits(&sys.image(&q, &p)),
                    "the stored state is the image of the state whose clock is the boundary ({at_})"
                );
            }
        }
    }
    past
}

#[test]
fn qa_az_landing_state_with_clock_fuzzed() {
    let past64 = check_fuzzed::<f64>(retakes_on);
    let past32 = check_fuzzed::<f32>(retakes_on);
    eprintln!("qa_az_landing_state_with_clock_fuzzed: steps past the boundary, f64 {past64}, f32 {past32}");
    assert!(
        past64 > 0,
        "the f64 fixtures exercise the landing: some step overshoots"
    );
    assert!(
        past32 > 0,
        "the f32 fixtures exercise the landing: some step overshoots"
    );
}

negative_control!(
    qa_az_landing_state_with_clock_fuzzed,
    "the clock-only landing: the clock set to the boundary over the overshot state",
    expected = "the stored state's own clock is the boundary",
    {
        check_fuzzed::<f64>(clock_only);
    }
);

// ── qa_az_landing_state_with_clock_truth ─────────────────────────────────────────────────────────────────────────

/// The Newtonian rates `(ṙ, ṗ)` of three bodies (`G = 1`, `p` the momenta), written here.
fn newton(m: &[f64; 3], r: &[[f64; 2]; 3], p: &[[f64; 2]; 3]) -> ([[f64; 2]; 3], [[f64; 2]; 3]) {
    let mut f = [[0.0; 2]; 3];
    for i in 0..3 {
        for j in i + 1..3 {
            let d = [r[j][0] - r[i][0], r[j][1] - r[i][1]];
            let n = d[0].hypot(d[1]);
            let k = m[i] * m[j] / (n * n * n);
            for c in 0..2 {
                f[i][c] += k * d[c];
                f[j][c] -= k * d[c];
            }
        }
    }
    let v = [0, 1, 2].map(|i| [p[i][0] / m[i], p[i][1] / m[i]]);
    (v, f)
}

/// The least pairwise separation of three positions.
fn closest(r: &[[f64; 2]; 3]) -> f64 {
    let d = |i: usize, j: usize| (r[i][0] - r[j][0]).hypot(r[i][1] - r[j][1]);
    d(0, 1).min(d(0, 2)).min(d(1, 2))
}

/// The truth: Cartesian RK4 in physical time from `s` to `t` in `n` steps, and the least separation it passed through.
fn truth(s: &State<f64>, t: f64, n: u32) -> (State<f64>, f64) {
    let h = t / f64::from(n);
    let (mut r, mut p) = (s.r, s.p);
    let mut least = closest(&r);
    let add = |a: [[f64; 2]; 3], b: [[f64; 2]; 3], k: f64| {
        [0, 1, 2].map(|i| [a[i][0] + k * b[i][0], a[i][1] + k * b[i][1]])
    };
    for _ in 0..n {
        let (k1r, k1p) = newton(&s.m, &r, &p);
        let (k2r, k2p) = newton(&s.m, &add(r, k1r, h / 2.0), &add(p, k1p, h / 2.0));
        let (k3r, k3p) = newton(&s.m, &add(r, k2r, h / 2.0), &add(p, k2p, h / 2.0));
        let (k4r, k4p) = newton(&s.m, &add(r, k3r, h), &add(p, k3p, h));
        r = add(
            add(add(add(r, k1r, h / 6.0), k2r, h / 3.0), k3r, h / 3.0),
            k4r,
            h / 6.0,
        );
        p = add(
            add(add(add(p, k1p, h / 6.0), k2p, h / 3.0), k3p, h / 3.0),
            k4p,
            h / 6.0,
        );
        least = least.min(closest(&r));
    }
    (State { m: s.m, r, p }, least)
}

/// A state's positions about its centre of mass (the regularised image is centred; the truth keeps the frame it starts
/// in, its total momentum 0, so the centre stays put).
fn centred(s: &State<f64>) -> [[f64; 2]; 3] {
    let m: f64 = s.m.iter().sum();
    let c = [0, 1].map(|k| (0..3).map(|i| s.m[i] * s.r[i][k]).sum::<f64>() / m);
    s.r.map(|x| [x[0] - c[0], x[1] - c[1]])
}

/// The distance between two states' positions (about their centres of mass) and momenta.
fn gap(a: &State<f64>, b: &State<f64>) -> f64 {
    let (ra, rb) = (centred(a), centred(b));
    ra.iter()
        .chain(a.p.iter())
        .flatten()
        .zip(rb.iter().chain(b.p.iter()).flatten())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// After four intervals of `Δ_sync = 1/8` at `η = 0.05`, at f64, on each of 128 fuzzed states whose truth keeps the
/// fuzzer's separation (`≥ 0.4`, no close encounter) and on which the clock-only landing keeps a clock past the boundary (at least one such state, so the comparison bites), the landing
/// under test (`landing`) leaves the state nearer the Cartesian truth than the clock-only landing does.
fn check_truth(landing: fn(&mut AzParams<f64>)) {
    let delta = 0.125;
    let mut compared = 0;
    for (i, s) in fuzzed(128).iter().enumerate() {
        let mut prm = params::<f64>(0.05, delta);
        landing(&mut prm);
        SPY_DELTA.with(|c| c.set((delta as f32, delta)));
        let (out, _) = spied(s, 4, &prm);
        let mut bad = params::<f64>(0.05, delta);
        bad.secant_tries = 0;
        bad.bisect_tries = 0;
        let (forbidden, rec) = spied(s, 4, &bad);
        if !rec.intervals.iter().flatten().any(|(q, _)| q.t > delta) {
            // The clock-only arm never kept an overshoot here: its state is not the forbidden one.
            continue;
        }
        let (want, least) = truth(s, 4.0 * delta, 80_000);
        if least < 0.4 {
            // A close encounter: the fuzzer's own separation bound (0.4) is left, η = 0.05's truncation there is not
            // the landing's, and a pointwise comparison through it is not one the parity contract allows.
            continue;
        }
        compared += 1;
        assert_eq!(
            (out.boundary, forbidden.boundary),
            (4, 4),
            "both arms land (state {i})"
        );
        let (landed, overshot) = (gap(&out.state, &want), gap(&forbidden.state, &want));
        eprintln!(
            "qa_az_landing_state_with_clock_truth: state {i}: landed {landed:e}, clock-only {overshot:e} from the truth"
        );
        assert!(
            landed < overshot,
            "the landed state is nearer the state at the boundary than the clock-only one (state {i}): {landed:e} vs {overshot:e}"
        );
    }
    assert!(
        compared > 0,
        "the clock-only arm keeps a clock past the boundary on some state, so the comparison bites"
    );
}

#[test]
fn qa_az_landing_state_with_clock_truth() {
    check_truth(retakes_on);
}

negative_control!(
    qa_az_landing_state_with_clock_truth,
    "the clock-only landing compared with itself",
    expected = "the landed state is nearer the state at the boundary",
    check_truth(clock_only)
);

// ── qa_az_r3_floor_tiny ──────────────────────────────────────────────────────────────────────────────────────────

/// The Levi-Civita map `L(u)u = (u₀² − u₁², 2u₀u₁)`, written here at f64.
fn lc64(u: [f64; 2]) -> [f64; 2] {
    [u[0] * u[0] - u[1] * u[1], 2.0 * u[0] * u[1]]
}

/// A registered system at `R` with `TINY = tiny` (any regular state; the floor is the system's).
fn system<R: P>(tiny: R) -> (AzSystem<R>, AzQ<R>) {
    let s = at::<R>(&fuzzed(1)[0]);
    let (sys, q, _) = register(&s, tiny);
    (sys, q)
}

/// REQ-INT-120 at `R`, the system registered with `tiny` and the precision's `TINY` being `want`: `|R₃| = 0` and
/// `|R₃| = x²` (with `u₁ = (x, 0)`, `u₂ = 0`, `x² < TINY`) read `want`; the registered state's `|R₃|`, above `TINY`,
/// reads `|L(u₂)u₂ − L(u₁)u₁|`.
fn check_floor<R: P>(tiny: R, want: f64, x: f64) {
    let (sys, q) = system::<R>(tiny);
    let what = core::any::type_name::<R>();
    let zero = AzQ { u2: q.u1, ..q };
    let below = AzQ {
        u1: [R::of(x), R::of(0.0)],
        u2: [R::of(0.0), R::of(0.0)],
        ..q
    };
    let exact: f64 = {
        let (a, b) = (
            lc64(below.u1.map(Into::<f64>::into)),
            lc64(below.u2.map(Into::<f64>::into)),
        );
        (b[0] - a[0]).hypot(b[1] - a[1])
    };
    assert!(
        exact > 0.0 && exact < want,
        "the fixture's |R₃| ({exact:e}) is above 0 and below TINY ({want:e}) at {what}"
    );
    for (name, q) in [("0", zero), ("below TINY", below)] {
        let got: f64 = sys.r3(&q).into();
        assert_eq!(
            got.to_bits(),
            Into::<f64>::into(R::of(want)).to_bits(),
            "at {what} an |R₃| of {name} reads TINY, {want:e}, not {got:e}"
        );
    }
    let model: f64 = {
        let (a, b) = (
            lc64(q.u1.map(Into::<f64>::into)),
            lc64(q.u2.map(Into::<f64>::into)),
        );
        (b[0] - a[0]).hypot(b[1] - a[1])
    };
    let got: f64 = sys.r3(&q).into();
    let ulps = if core::mem::size_of::<R>() == 4 {
        f64::from(f32::EPSILON)
    } else {
        f64::EPSILON
    };
    assert!(
        (got - model).abs() <= 8.0 * ulps * model,
        "at {what} an |R₃| above TINY reads itself: {got:e} against {model:e}"
    );
}

/// The floor at both precisions: f32's registered with `tiny32`.
fn check_floors(tiny32: f32) {
    check_floor::<f32>(tiny32, 1e-37_f32.into(), 2e-19);
    check_floor::<f64>(1e-300, 1e-300, 1e-151);
}

#[test]
fn qa_az_r3_floor_tiny() {
    check_floors(params::<f32>(0.01, 0.125).tiny);
}

negative_control!(
    qa_az_r3_floor_tiny,
    "a floor of 0 at f32",
    expected = "at f32 an |R₃| of",
    check_floors(0.0)
);
