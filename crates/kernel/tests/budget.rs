//! The regularised integration floor's step budget (`kernel::regularisation::budget`; integrator_contract Part 6;
//! R-563, R-564, R-603, R-604; payload §2's `budget_bits`):
//! - `budget_undetermined_*` (REQ-INT-101): a Heggie × RK4 copy that runs out of its budget stops `running`, its sticky
//!   `budget_exhausted` bit set and `saturated` with it, its state kept, never a physical outcome; a later dispatch
//!   hands it back word for word, never marching it on; and through `integrate_with` over many macro-steps the budget
//!   binds on the copy's cumulative step total, never on one call's.
//! - `budget_rate_*` (REQ-INT-102): the allowance is `max(N_min, rate · t)`, `N_min` holding an early encounter whose
//!   `rate · t` is still small; and it is checked every step, so a march overruns its allowance by at most one step and
//!   that step's landing retakes, never by the rest of a sync interval.
//!
//! Every pass criterion is written here before the runs it reads. The fixtures are test inputs: Burrau's
//! Pythagorean problem (a deep close-encounter march) at f64 and f32, under step-control values that are not the
//! calibrations (`cargo xtask gate step-budget --propose` proposes those).

use std::sync::atomic::{AtomicU32, Ordering};

use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::advance::{Advance, AdvanceParams, AdvanceReal, Advanced, Cadence, Reached};
use kernel::driver::benettin::{integrate_ftle, BenettinParams};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::carried::Carried;
use kernel::driver::terminal::finished;
use kernel::driver::wrapper::{
    integrate_resumed_with, integrate_with, monitored, WrapperReal, WrapperUniforms,
};
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::payload::{
    bb_budget_exhausted, bb_cap_hits, bb_floor_hits, sd_saturated, sd_state, tm_t_end_step,
    SimStateBaseOf, STATE_RUNNING,
};
use kernel::regularisation::az::{AzCount, AzParams, AzReal, End, ExplicitTest};
use kernel::regularisation::budget::{cap_hits_max, over_allowance};
use kernel::regularisation::heggie::{
    self as hg, Heggie, HgConfig, HgP, HgParams, HgQ, HgWatch, DEFAULT_TIME,
};
use kernel::regularisation::logh::LhParams;
use kernel::regularisation::pairs::LogHMode;
use kernel::regularisation::Composed;
use validation::drift::{sim_key, AsR};
use validation::negative_control;

// ── The fixtures ───────────────────────────────────────────────────────────────────────────────────────────────

/// A precision the tests run at.
trait P: WrapperReal + AzReal + Into<f64> + core::fmt::Debug {}

impl P for f32 {}
impl P for f64 {}

/// A Burrau-like close-encounter state, as `tests/sync_quantum.rs` takes it: the Pythagorean problem's masses and
/// positions scaled by 1/5, with small momenta, so not Burrau's problem itself; a deep encounter march for a budget.
fn burrau<R: P>() -> Ic<R>
where
    f64: AsR<R>,
{
    let m = [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0];
    let r = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]].map(|x: [f64; 2]| x.map(|c| c / 5.0));
    let p = [[0.03, -0.01], [-0.02, 0.04], [-0.01, -0.03]];
    Ic {
        m: m.map(AsR::to_r),
        r: r.map(|x| x.map(AsR::to_r)),
        p: p.map(|x: [f64; 2]| x.map(AsR::to_r)),
    }
}

/// Heggie's test values (`η = 0.01`, `Δ_sync = 1/8`, the reference's `TINY`, `f = 0.02`, the landing in force,
/// Eq. 22) with the budget `rate`, `n_min`. Test inputs.
fn values<R: AzReal>(rate: f64, n_min: u32) -> HgParams<R> {
    HgParams {
        eta: R::at_width(0.01, 0.01),
        delta_sync: R::at_width(0.125, 0.125),
        tiny: R::at_width(1e-37, 1e-300),
        predictive_f: R::at_width(0.02, 0.02),
        step_budget_rate: R::at_width(rate as f32, rate),
        n_min,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        time: DEFAULT_TIME,
    }
}

/// A budget that never binds on these runs.
struct Loose;

impl HgConfig for Loose {
    fn params<R: AzReal>() -> HgParams<R> {
        values(1e7, 100_000_000)
    }
}

/// The step total [`Tight`] allows, at rate 0: more than any one call of the wrapper's takes on Burrau's march (one
/// sync interval at most, since a macro-step of `T = 50`'s sim key is far shorter than `Δ_sync`), which
/// [`budget_undetermined_binds_on_the_cumulative_count`] measures and asserts, and fewer than the march to `T` takes
/// together. A fixture input, not a threshold.
const TIGHT_STEPS: u32 = 1000;

/// A budget of [`TIGHT_STEPS`] steps at rate 0.
struct Tight;

impl HgConfig for Tight {
    fn params<R: AzReal>() -> HgParams<R> {
        values(0.0, TIGHT_STEPS)
    }
}

type HgLoose = Composed<Rk4, Heggie<Loose>>;
type HgTight = Composed<Rk4, Heggie<Tight>>;

/// The horizon of every run here, the default `T`.
const T: f64 = 50.0;

/// `A`'s run of Burrau at `R` to `T`, through the wrapper and its own cadence, the monitored fields written.
fn run<A: Advance, R: P>() -> SimStateBaseOf<R>
where
    f64: AsR<R>,
{
    let ic = burrau::<R>();
    let key: WrapperUniforms<R> = sim_key(T);
    let mut cadence = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    monitored(integrate_with::<A, R, _>(&ic, &key, &mut cadence), &cadence)
}

/// `s`'s words, its reals by their bits: what "word for word" compares.
fn words<R: P>(s: &SimStateBaseOf<R>) -> String {
    let bits = |x: R| Into::<f64>::into(x).to_bits();
    let mut out = String::new();
    for v in s.r.iter().chain(s.p.iter()) {
        out += &format!("{:x} {:x} ", bits(v[0]), bits(v[1]));
    }
    for x in [s.S, s.theta, s.mean_y, s.C_ty, s.E_0, s.Lz_0, s.closure_min] {
        out += &format!("{:x} ", bits(x));
    }
    out + &format!(
        "{:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x}",
        s.packed_a,
        s.packed_b,
        s.times,
        s.total_substeps,
        s.budget_bits,
        s.dE_reg,
        s.closure_step,
        s._reserved,
        s.coll_floor_bits
    )
}

// ── budget_undetermined (REQ-INT-101) ──────────────────────────────────────────────────────────────────────────

/// Asserts what an exhausted copy stores and how a later dispatch treats it: `stored`, `Tight`'s run, is `running`, its
/// `budget_exhausted` bit set, `saturated` set (its entry-cap signal fired), short of the horizon, its state finite;
/// and `redispatch` of it hands it back word for word.
fn check_undetermined<R: P>(name: &str, redispatch: fn(&SimStateBaseOf<R>) -> SimStateBaseOf<R>)
where
    f64: AsR<R>,
{
    let key: WrapperUniforms<R> = sim_key(T);
    let stored = run::<HgTight, R>();
    let n = tm_t_end_step(stored.times);
    println!(
        "{name}: exhausted {} state {} saturated {} at t_end_step {n} of {}; floor hits {}, cap hits {}",
        bb_budget_exhausted(stored.budget_bits),
        sd_state(stored.packed_a),
        sd_saturated(stored.packed_a),
        key.horizon_steps,
        bb_floor_hits(stored.budget_bits),
        bb_cap_hits(stored.budget_bits)
    );
    assert!(
        bb_budget_exhausted(stored.budget_bits),
        "{name}: the budget did not run out"
    );
    assert_eq!(
        sd_state(stored.packed_a),
        STATE_RUNNING,
        "{name}: an exhausted copy is labelled, not undetermined"
    );
    assert!(
        n < key.horizon_steps,
        "{name}: an exhausted copy reached the horizon"
    );
    assert!(
        sd_saturated(stored.packed_a) && bb_cap_hits(stored.budget_bits) > 0,
        "{name}: saturated is not set with the signals that fired"
    );
    let finite = stored
        .r
        .iter()
        .chain(stored.p.iter())
        .flatten()
        .all(|&x| Into::<f64>::into(x).is_finite());
    assert!(finite, "{name}: the exhausted copy's state was not kept");
    let again = redispatch(&stored);
    assert!(
        words(&again) == words(&stored),
        "{name}: a later dispatch did not hand the exhausted copy back word for word"
    );
    // The loose budget's run of the same IC is the control on the fixture: it does not run out.
    let loose = run::<HgLoose, R>();
    assert!(
        !bb_budget_exhausted(loose.budget_bits),
        "{name}: the loose budget ran out too"
    );
}

/// A later dispatch to the horizon, as the scheduler makes one: the wrapper's resumed march with a fresh cadence.
fn resumed<R: P>(stored: &SimStateBaseOf<R>) -> SimStateBaseOf<R>
where
    f64: AsR<R>,
{
    let ic = burrau::<R>();
    let key: WrapperUniforms<R> = sim_key(T);
    let mut cadence = WrapperCadence::new(&ic.m, &stored.r, &stored.p);
    let out = integrate_resumed_with::<HgTight, R, _>(&ic, &key, &mut cadence, stored, u32::MAX);
    cadence.finish(out)
}

#[test]
fn budget_undetermined_on_exhaustion() {
    check_undetermined::<f64>("Heggie × RK4, f64", resumed::<f64>);
    check_undetermined::<f32>("Heggie × RK4, f32", resumed::<f32>);
}

negative_control!(
    budget_undetermined_on_exhaustion,
    "a dispatch that reads the copy as running, its exhausted bit lost, marches it on",
    expected = "did not hand the exhausted copy back word for word",
    check_undetermined::<f64>("Heggie × RK4, f64, the bit dropped", |s| {
        let lost = SimStateBaseOf {
            budget_bits: 0,
            ..*s
        };
        resumed::<f64>(&lost)
    })
);

/// The FTLE tier's run of the exhausted copy, its shadow beside it: every word the base tier stores of the budget and
/// the regularised drift is the FTLE tier's too, so the copy reads as finished there and is never marched on. `keep`
/// is what the FTLE writer passes on of the base tier's `budget_bits`.
fn check_ftle_exhausted<R: P>(name: &str, keep: fn(u32) -> u32)
where
    f64: AsR<R>,
{
    let key: WrapperUniforms<R> = sim_key(T);
    let base = run::<HgTight, R>();
    let params = BenettinParams {
        delta_0: 1e-8.to_r(),
        n_renorm: 10,
    };
    let ftle =
        integrate_ftle::<HgTight, R>(&burrau::<R>(), &key, CollisionRadius::none(), params, false);
    let budget_bits = keep(ftle.budget_bits);
    println!(
        "{name}: base budget_bits {:#010x} dE_reg {:#010x}; FTLE {budget_bits:#010x} {:#010x}",
        base.budget_bits, base.dE_reg, ftle.dE_reg
    );
    assert!(
        bb_budget_exhausted(base.budget_bits) && base.dE_reg != 0,
        "{name}: the base run's fixture is not an exhausted copy with a drift"
    );
    assert_eq!(
        (budget_bits, ftle.dE_reg),
        (base.budget_bits, base.dE_reg),
        "{name}: the FTLE tier does not store the base tier's budget word and regularised drift"
    );
    assert!(
        finished(
            ftle.packed_a,
            ftle.times,
            budget_bits,
            key.horizon_steps,
            true
        ),
        "{name}: the FTLE tier's exhausted copy reads as unfinished"
    );
}

#[test]
fn budget_undetermined_on_the_ftle_tier() {
    check_ftle_exhausted::<f64>("Heggie × RK4, FTLE, f64", |b| b);
    check_ftle_exhausted::<f32>("Heggie × RK4, FTLE, f32", |b| b);
}

negative_control!(
    budget_undetermined_on_the_ftle_tier,
    "an FTLE writer that drops budget_bits, as it once did, stores 0 and the copy reads as unfinished",
    expected = "does not store the base tier's budget word",
    check_ftle_exhausted::<f64>("Heggie × RK4, FTLE, f64, budget_bits dropped", |_| 0)
);

/// The largest running-step increment one wrapper call made, across the calls since it was reset.
static LARGEST_CALL: AtomicU32 = AtomicU32::new(0);

/// `A`, with each call's increment of the carried running step total recorded in [`LARGEST_CALL`].
struct Measured<A>(core::marker::PhantomData<A>);

impl<A: Advance> Advance for Measured<A> {
    const OWN_SAMPLING: bool = A::OWN_SAMPLING;

    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        A::advance(state, geom, t_now, t_target, params, cadence)
    }

    fn advance_carrying<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
        carried: &mut Carried<R>,
    ) -> (Advanced<R>, Reached) {
        let before = carried.steps;
        let out = A::advance_carrying(state, geom, t_now, t_target, params, cadence, carried);
        LARGEST_CALL.fetch_max(carried.steps - before, Ordering::Relaxed);
        out
    }
}

/// `A` with the carried running total set back to 0 before every call: a budget read per call, never cumulatively,
/// the control of [`budget_undetermined_binds_on_the_cumulative_count`] and its rated twin.
#[cfg(feature = "controls")]
struct PerCall<A>(core::marker::PhantomData<A>);

#[cfg(feature = "controls")]
impl<A: Advance> Advance for PerCall<A> {
    const OWN_SAMPLING: bool = true;

    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        A::advance(state, geom, t_now, t_target, params, cadence)
    }

    fn advance_carrying<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
        carried: &mut Carried<R>,
    ) -> (Advanced<R>, Reached) {
        carried.steps = 0;
        A::advance_carrying(state, geom, t_now, t_target, params, cadence, carried)
    }
}

/// Asserts that through `integrate_with`, over the many macro-steps of `T`'s sim key, `A` (`Tight`'s budget) runs out,
/// and that no one call took as many steps as the budget allows, so it ran out on the cumulative count alone.
fn check_cumulative<A: Advance>() {
    LARGEST_CALL.store(0, Ordering::Relaxed);
    let loose = run::<Measured<HgLoose>, f64>();
    let largest = LARGEST_CALL.load(Ordering::Relaxed);
    let key: WrapperUniforms<f64> = sim_key(T);
    println!(
        "the largest one call's steps on the loose march: {largest} (the budget {TIGHT_STEPS}); the march's total \
         {} accepted over {} macro-steps",
        loose.total_substeps, key.horizon_steps
    );
    assert!(
        largest > 0 && largest < TIGHT_STEPS,
        "a single call took {largest} steps, so the budget could bind within one call"
    );
    let out = run::<A, f64>();
    assert!(
        bb_budget_exhausted(out.budget_bits),
        "the budget did not bind on the cumulative count"
    );
    assert!(
        tm_t_end_step(out.times) > 1,
        "the budget bound in the first macro-step"
    );
}

#[test]
fn budget_undetermined_binds_on_the_cumulative_count() {
    check_cumulative::<HgTight>();
}

negative_control!(
    budget_undetermined_binds_on_the_cumulative_count,
    "a budget whose count is reset at every call never binds",
    expected = "the budget did not bind on the cumulative count",
    check_cumulative::<PerCall<HgTight>>()
);

/// The rated budget's floor: above the largest one call's steps (609, measured above), so a count reset per call never
/// binds, and above the loose march's 1 619 steps by `t = 1`, so the floor does not bind first. A fixture input.
const RATED_N_MIN: u32 = 2000;

/// The rated budget's rate, in steps per unit time. The loose march's mean rate `N(t)/t`, measured: 1 619 at `t = 1`,
/// 1 226 at 2, 1 720 at 4, 2 017 at 8, 2 247 at 50, rising after `t = 2`; so the cumulative count outruns `rate · t`
/// near `t = 8`, well after `rate · t` passes the floor. A fixture input.
const RATED_RATE: f64 = 2000.0;

/// A budget of [`RATED_RATE`] per unit time over a floor of [`RATED_N_MIN`].
struct Rated;

impl HgConfig for Rated {
    fn params<R: AzReal>() -> HgParams<R> {
        values(RATED_RATE, RATED_N_MIN)
    }
}

type HgRated = Composed<Rk4, Heggie<Rated>>;

/// The time before which a budget read on the carried time cannot run out, and after which [`RATED_N_MIN`] alone has
/// long bound: the loose march's mean rate `N(t)/t` is under [`RATED_RATE`] up to here (1 720 at `t = 4`, measured), so
/// the count stays within `rate · t`; while its count passes 2 000 between `t = 1` (1 619) and `t = 2` (2 452), so a
/// budget that read each call's own time, whose allowance is the floor alone ([`FloorAlone`]), runs out well before.
const FLOOR_ALONE_BY: f64 = 4.0;

/// The allowance a budget that read each call's own time would have: within a call `t ≤ Δ_sync = 1/8`, so
/// `rate · t ≤ 250 < N_min`, and the allowance is [`RATED_N_MIN`] alone, exactly; the control's config.
#[cfg(feature = "controls")]
struct FloorAlone;

#[cfg(feature = "controls")]
impl HgConfig for FloorAlone {
    fn params<R: AzReal>() -> HgParams<R> {
        values(0.0, RATED_N_MIN)
    }
}

/// Asserts that through `integrate_with`, with `step_budget_rate > 0`, `A` runs out after its first macro-step, and
/// that at that time `rate · t` is above `N_min`, so the rate term, read against the time carried across seam calls,
/// is the one that bound.
fn check_cumulative_rated<A: Advance>() {
    let key: WrapperUniforms<f64> = sim_key(T);
    let out = run::<A, f64>();
    let n = tm_t_end_step(out.times);
    let t = f64::from(n) * key.dt_macro;
    println!(
        "rated budget, rate {RATED_RATE} N_min {RATED_N_MIN}: exhausted {} at t_end_step {n} (t = {t:.4}, rate·t = \
         {:.0}) after {} accepted steps",
        bb_budget_exhausted(out.budget_bits),
        RATED_RATE * t,
        out.total_substeps
    );
    assert!(
        bb_budget_exhausted(out.budget_bits),
        "the rated budget did not bind on the cumulative count"
    );
    assert!(
        n > 1 && RATED_RATE * t > f64::from(RATED_N_MIN),
        "the rated budget bound at t = {t}, where N_min, not the rate, sets the allowance"
    );
    assert!(
        t > FLOOR_ALONE_BY,
        "the rated budget bound at t = {t}, by which the floor alone would bind: the rate was read on call-local time"
    );
}

#[test]
fn budget_undetermined_binds_on_the_cumulative_count_at_a_rate() {
    check_cumulative_rated::<HgRated>();
}

negative_control!(
    budget_undetermined_binds_on_the_cumulative_count_at_a_rate,
    "a rated budget whose count is reset at every call never binds",
    expected = "the rated budget did not bind on the cumulative count",
    check_cumulative_rated::<PerCall<HgRated>>()
);

negative_control!(
    budget_rate_reads_the_carried_time,
    "a rated budget read on each call's own time is the floor alone, and binds by t = 2, before t = 4",
    expected = "the rate was read on call-local time",
    check_cumulative_rated::<Composed<Rk4, Heggie<FloorAlone>>>()
);

// ── budget_rate (REQ-INT-102) ──────────────────────────────────────────────────────────────────────────────────

/// An allowance rule: `(steps, t, rate, n_min)` in, whether the budget is spent out.
type Rule = fn(u32, f64, f64, u32) -> bool;

/// Asserts `rule` is `max(N_min, rate · t)`: at `rate = 100`, `N_min = 50`, early (`t = 0.1`, `rate · t = 10`) the floor
/// holds 50 steps and the 51st spends it; late (`t = 10`, `rate · t = 1000`) the rate holds 1000 and the 1001st spends
/// it; an allowance that is not a number spends it.
fn check_rule(rule: Rule) {
    assert!(
        !rule(50, 0.1, 100.0, 50),
        "N_min does not hold an early encounter's 50 steps"
    );
    assert!(
        rule(51, 0.1, 100.0, 50),
        "the 51st step past N_min = 50 is allowed"
    );
    assert!(
        !rule(1000, 10.0, 100.0, 50),
        "rate · t = 1000 does not hold 1000 steps"
    );
    assert!(
        rule(1001, 10.0, 100.0, 50),
        "the 1001st step past rate · t = 1000 is allowed"
    );
    assert!(
        rule(0, 1.0, f64::NAN, 50),
        "a NaN allowance leaves the march unbounded"
    );
}

#[test]
fn budget_rate_allowance_is_max_of_floor_and_rate() {
    check_rule(over_allowance::<f64>);
    check_rule(|s, t, r, n| over_allowance::<f32>(s, t as f32, r as f32, n));
}

negative_control!(
    budget_rate_allowance_is_max_of_floor_and_rate,
    "an allowance of rate · t alone, no N_min, spends an early encounter's budget",
    expected = "N_min does not hold an early encounter's 50 steps",
    check_rule(|s, t, r, _| f64::from(s) > r * t)
);

/// The running steps one step can add at most: the stepper's call and its landing's retakes, the secant's 4 and the
/// bisection's 64 (Part 2b, step 6; [`values`]).
const ONE_STEP: f64 = 1.0 + 4.0 + 64.0;

/// What a march stopped by its budget overran it by: `steps − max(n_min, rate · t)`, `t` where it stopped.
fn overrun(steps: u32, t: f64, rate: f64, n_min: u32) -> f64 {
    f64::from(steps) - (rate * t).max(f64::from(n_min))
}

/// Heggie's march of Burrau at f64 under `rate`, `n_min`, to `T`: its running steps, where it stopped and how.
fn march(rate: f64, n_min: u32) -> (u32, f64, bool) {
    let ic = burrau::<f64>();
    let s = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let params = values::<f64>(rate, n_min);
    let out = hg::march::<Rk4, f64, _, _>(
        &s,
        (T / 0.125) as u32,
        &params,
        &mut Quiet,
        &mut ExplicitTest,
    );
    (out.count.steps, out.t, out.count.end == End::Exhausted)
}

// ── The budget and the predictive step limit compose (TASK-M3-31's limit on by default) ────────────────────────────

/// A watch that counts the steps the march sizes, one per step it takes outside a landing's retakes.
#[derive(Default)]
struct Sized(u32);

impl HgWatch<f64> for Sized {
    fn finite(&mut self, q: &HgQ<f64>, p: &HgP<f64>) -> bool {
        ExplicitTest.finite(q, p)
    }
    fn sized(&mut self, _: f64, _: f64, _: f64) {
        self.0 += 1;
    }
}

/// The boundary the composition march runs to: `t = 1`, Burrau's first close encounter included.
const LIMIT_K: u32 = 8;

/// Heggie's march of Burrau at f64 to [`LIMIT_K`] at rate 0 and `n_min`, the landing's retakes off so every step is a
/// sized one: the count, the steps sized, the steps the predictive limit shortened, and whether the budget ran out.
fn limited_march(n_min: u32) -> (hg::HgCount<f64>, u32) {
    let ic = burrau::<f64>();
    let s = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let params = HgParams {
        secant_tries: 0,
        bisect_tries: 0,
        ..values::<f64>(0.0, n_min)
    };
    let mut watch = Sized::default();
    let out = hg::march::<Rk4, f64, _, _>(&s, LIMIT_K, &params, &mut Quiet, &mut watch);
    (out.count, watch.0)
}

/// Asserts the budget counts every step the march takes, those the predictive limit shortened included: with no
/// retakes, `counted` (what the budget is taken to compare) equals the steps sized; the limit bound on some of them;
/// and the march runs out at `N_min` one below that total and not at the total itself.
fn check_limit_composes(counted: fn(&hg::HgCount<f64>) -> u32) {
    let (loose, sized) = limited_march(u32::MAX);
    let n = counted(&loose);
    println!(
        "Burrau-like, Heggie × RK4, f64, to t = 1, retakes off: {} steps counted, {sized} sized, {} shortened by the \
         predictive limit; budget read as {n}",
        loose.steps, loose.predictive_bound
    );
    assert!(
        loose.predictive_bound > 0,
        "the predictive limit never bound: the fixture tests nothing"
    );
    assert_eq!(
        n, sized,
        "the budget does not count every step the march took"
    );
    let (at, _) = limited_march(n);
    let (below, _) = limited_march(n - 1);
    assert!(
        at.end != End::Exhausted && below.end == End::Exhausted,
        "the budget does not run out exactly at the march's step total"
    );
}

#[test]
fn budget_counts_steps_the_predictive_limit_shortens() {
    check_limit_composes(|c| c.steps);
}

negative_control!(
    budget_counts_steps_the_predictive_limit_shortens,
    "a budget that does not count the steps the limit shortened",
    expected = "the budget does not count every step the march took",
    check_limit_composes(|c| c.steps - c.predictive_bound)
);

/// A cadence that watches nothing.
struct Quiet;

impl<R> Cadence<R> for Quiet {
    fn after_step(&mut self, _: &mut State<R>, _: &mut Geom<R>) {}
    fn done(&self) -> bool {
        false
    }
}

/// The steps at each sync boundary of Heggie's loose march of Burrau, `(t, running steps)`, for the boundary-granular
/// reading of the budget a check only at boundaries would make (the control).
#[cfg(feature = "controls")]
fn boundary_counts() -> Vec<(f64, u32)> {
    let ic = burrau::<f64>();
    let s = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let params = values::<f64>(1e7, 100_000_000);
    let mut out = hg::march::<Rk4, f64, _, _>(&s, 1, &params, &mut Quiet, &mut ExplicitTest);
    let mut counts = vec![(out.t, out.count.steps)];
    for k in 2..=(T / 0.125) as u32 {
        let from = hg::HgStanding {
            q: out.q,
            p: out.p,
            boundary: out.boundary,
            count: out.count,
        };
        out = hg::march_on::<Rk4, f64, _, _>(
            &out.system,
            &out.state,
            from,
            k,
            &params,
            &mut Quiet,
            &mut ExplicitTest,
        );
        counts.push((out.t, out.count.steps));
    }
    counts
}

/// The budget rate whose binding the every-step test reads: half the loose march's mean rate over `T`, so the budget
/// binds mid-run. A fixture input.
fn binding_rate() -> f64 {
    let (steps, t, _) = march(1e7, 100_000_000);
    0.5 * f64::from(steps) / t
}

/// Asserts that a march stopped by its budget overran its allowance by at most [`ONE_STEP`]: `stop` gives where the
/// budget stopped the march, `(steps, t)`, under `rate` and `n_min = 0`.
fn check_every_step(stop: fn(f64) -> (u32, f64)) {
    let rate = binding_rate();
    let (steps, t) = stop(rate);
    let over = overrun(steps, t, rate, 0);
    println!(
        "rate {rate:.1}: stopped at t = {t:.4} after {steps} steps, {over:.1} past its allowance"
    );
    assert!(
        over > 0.0 && over <= ONE_STEP,
        "the march overran its allowance by {over:.1} steps, more than one step's {ONE_STEP}"
    );
}

#[test]
fn budget_rate_checked_every_step() {
    check_every_step(|rate| {
        let (steps, t, exhausted) = march(rate, 0);
        assert!(exhausted, "the budget did not bind at rate {rate}");
        (steps, t)
    });
}

negative_control!(
    budget_rate_checked_every_step,
    "a budget read only at sync boundaries overruns it by the rest of an interval",
    expected = "more than one step's",
    check_every_step(|rate| {
        let first = boundary_counts()
            .into_iter()
            .find(|&(t, steps)| overrun(steps, t, rate, 0) > 0.0)
            .expect("the budget binds at some boundary");
        (first.1, first.0)
    })
);

/// Asserts the entry-cap count `write` stores after `cap_hits_max() + 10` hits saturates at its field's greatest value,
/// never wrapping, and spills into no other field: a long count reads "at least".
fn check_saturate(write: fn(u32) -> u32) {
    let w = write(cap_hits_max() + 10);
    assert_eq!(
        bb_cap_hits(w),
        cap_hits_max(),
        "the entry-cap count wrapped"
    );
    assert_eq!(
        bb_floor_hits(w),
        0,
        "a cap hit spilled into the floor count"
    );
    assert!(!bb_budget_exhausted(w), "a count spilled into the flag");
}

#[test]
fn budget_rate_counts_saturate() {
    check_saturate(|hits| {
        let mut s = kernel::regularisation::budget::Signals::none();
        for _ in 0..hits {
            s.observe(false, true);
        }
        s.write(0)
    });
}

negative_control!(
    budget_rate_counts_saturate,
    "a count written through the field's mask alone wraps",
    expected = "the entry-cap count wrapped",
    check_saturate(|hits| kernel::payload::set_cap_hits(0, hits))
);

// ── A terminal on the step the budget runs out (physics review 6003366749 of PR #32, B1) ───────────────────────────

/// A cadence that is done from its `at`-th kept step on: a terminal firing on that step.
struct DoneAt {
    at: u32,
    seen: u32,
}

impl<R> Cadence<R> for DoneAt {
    fn after_step(&mut self, _: &mut State<R>, _: &mut Geom<R>) {
        self.seen += 1;
    }
    fn done(&self) -> bool {
        self.seen >= self.at
    }
}

/// How one occupant's march of Burrau at f64 ended under `n_min` steps at rate 0 with a cadence done from its `at`-th
/// kept step (`u32::MAX`: never), and the kept steps it took.
type Ends = fn(u32, u32) -> (End, u32);

fn heggie_ends(n_min: u32, at: u32) -> (End, u32) {
    let ic = burrau::<f64>();
    let s = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let out = hg::march::<Rk4, f64, _, _>(
        &s,
        (T / 0.125) as u32,
        &values::<f64>(0.0, n_min),
        &mut DoneAt { at, seen: 0 },
        &mut ExplicitTest,
    );
    (out.count.end, out.count.accepted)
}

fn az_ends(n_min: u32, at: u32) -> (End, u32) {
    let ic = burrau::<f64>();
    let s = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let v = values::<f64>(0.0, n_min);
    let params = AzParams {
        eta: v.eta,
        delta_sync: v.delta_sync,
        tiny: v.tiny,
        predictive_f: v.predictive_f,
        step_budget_rate: v.step_budget_rate,
        n_min: v.n_min,
        clamp: v.clamp,
        secant_tries: v.secant_tries,
        bisect_tries: v.bisect_tries,
    };
    let out = kernel::regularisation::az::march::<Rk4, f64, _, _>(
        &s,
        0,
        (T / 0.125) as u32,
        AzCount::new(),
        &params,
        &mut DoneAt { at, seen: 0 },
        &mut ExplicitTest,
    );
    (out.count.end, out.count.accepted)
}

fn logh_ends(n_min: u32, at: u32) -> (End, u32) {
    let ic = burrau::<f64>();
    let s = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let v = values::<f64>(0.0, n_min);
    let params = LhParams {
        eta: v.eta,
        delta_sync: v.delta_sync,
        tiny: v.tiny,
        predictive_f: v.predictive_f,
        step_budget_rate: v.step_budget_rate,
        n_min: v.n_min,
        clamp: v.clamp,
        secant_tries: v.secant_tries,
        bisect_tries: v.bisect_tries,
        mode: LogHMode::LogH,
    };
    let out = kernel::regularisation::logh::march::<Rk4, f64, _, _>(
        &s,
        (T / 0.125) as u32,
        &params,
        &mut DoneAt { at, seen: 0 },
        &mut ExplicitTest,
    );
    (out.count.end, out.count.accepted)
}

/// Asserts, for each occupant, that a terminal firing on the very kept step after which the budget runs out ends the
/// march `Done`, the terminal the outcome, never `Exhausted`: the budget's own run with no terminal gives that step,
/// and `shift` places the terminal relative to it (0: on it, the property; 1: one step later, the control).
fn check_terminal_wins(shift: u32) {
    for (name, ends) in [
        ("Heggie", heggie_ends as Ends),
        ("Aarseth–Zare", az_ends as Ends),
        ("logH", logh_ends as Ends),
    ] {
        let (end, kept) = ends(TIGHT_STEPS, u32::MAX);
        assert_eq!(
            end,
            End::Exhausted,
            "{name}: the budget does not run out on the fixture"
        );
        let (end, _) = ends(TIGHT_STEPS, kept + shift);
        println!("{name}: the budget runs out after kept step {kept}; a terminal there ends the march {end:?}");
        assert_eq!(
            end,
            End::Done,
            "{name}: a terminal on the step the budget ran out did not end the march as its outcome"
        );
    }
}

#[test]
fn budget_undetermined_terminal_on_the_last_step_is_the_outcome() {
    check_terminal_wins(0);
}

negative_control!(
    budget_undetermined_terminal_on_the_last_step_is_the_outcome,
    "a terminal one step after the budget ran out is never reached",
    expected = "did not end the march as its outcome",
    check_terminal_wins(1)
);
