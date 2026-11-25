//! Heggie's occupant seam and the carried regularised state (`kernel::regularisation::heggie::Heggie`,
//! `kernel::driver::carried`, the wrapper's `integrate_carrying` and `integrate_resumed_carrying`; integrator_contract
//! Part 2a, Part 2b § "The profile gains a field"; dd_simstate_payload §1, "The carried regularised state"):
//! - `heggie_seam_non_finite_fails`, `heggie_seam_exhausted_stops`: through the wrapper (`integrate_with`), a Heggie ×
//!   RK4 march ended non-finite fails the run, `sim_failed` at the macro-step it ended in, its state the march's
//!   latched one; a spent budget stops the copy, `running` as it stood, never run on. Their control drops the march's
//!   end (the stepper's rule).
//! - `regularised_state_carried`: Heggie × RK4, at f64 and f32, marched across `k` `ADVANCE` calls with the carried
//!   state is one direct march registered once at `t = 0`, bitwise, in the Cartesian state and in `(Q, P, h)` and the
//!   clock, with the direct march's step total and 0 re-registrations. Control: a seam that rebuilds the regularised
//!   state from `(m, r, p)` at each call.
//! - `regularised_state_carried_reboot`: through the wrapper, catch-up (one call from the IC), the live march (one
//!   macro-step per call, resumed) and a re-boot after eviction (the stored `SimState` and buffer read back, then
//!   resumed) agree bitwise, with 0 re-registrations. Control: a re-boot that does not restore the carried state.
//! - `regularised_state_carried_clock`: the interval clock at every call's end is the boundary's `Δ_sync`, bitwise.
//!   Control: a landing with its retakes off, which keeps a clock past the boundary. The test also measures the
//!   control the task asked for, a re-boot that restores `Q`, `P` and `h` but not the clock: it does not differ from
//!   the live march, on the fixture or with a landing planted off the boundary, because every interval restarts its
//!   clock at 0. That is asserted, printed, and reported void in the pull request.
//! - `regularised_state_carried_budget`: the running step total crosses the calls, so a budget that the direct march
//!   spends in a later interval is spent by the seam's march at the same step, the state bitwise. Control: a seam that
//!   starts its step total afresh at each call.
//! - `regularised_state_carried_layout`: the buffer's element is 72 bytes at f32 and 128 at f64, all zero before a
//!   registration, with Heggie's `Q`, `P`, `h` and clock in payload §1's order. Control: `h` and the clock swapped.
//! - `regularised_re_registrations_visible`: the count a consumer reads from the buffer: 0 for Heggie through the
//!   wrapper, one per boundary marched on from after `t = 0` for Aarseth–Zare, and 1 for a Heggie call handed no
//!   carried state after `t = 0`. Control: a seam that does not pass the buffer on.
//! - `regularisation_default`: the default pair is Heggie × RK4 under Eq. 22, measured good, with Eq. 20 selectable,
//!   and its occupant type runs with RK4's profile on a regularised system. Control: a default of Heggie × KDK.
//! - `regularisation_default_pairs`: every stepper × regularisation pair is selected and runs a call through the
//!   seam: all five steppers under `none`, Aarseth–Zare, Heggie (Eq. 20 and Eq. 22) and logH in each time mode, 30
//!   pairs. Control: a builder that refuses a pair.
//!
//! logH's seam and carried state are `tests/logh_seam.rs`.

use kernel::decode::Ic;
#[cfg(feature = "controls")]
use kernel::driver::advance::AdvanceReal;
use kernel::driver::advance::{Advance, AdvanceParams, Advanced, Cadence, Reached};
use kernel::driver::carried::Carried;
use kernel::driver::substep::NsubThresholds;
use kernel::driver::wrapper::{
    integrate_carrying, integrate_resumed_carrying, integrate_with, WrapperReal, WrapperUniforms,
};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{DebugOccupant, Geom, Occupant, Selection, State};
use kernel::payload::{
    sd_detail, sd_state, tm_t_end_step, SimStateBaseOf, STATE_BOUNDED, STATE_RUNNING,
    STATE_SIM_FAILED,
};
use kernel::regularisation::az::{AarsethZare, AzConfig, AzParams, AzReal, End, ExplicitTest};
use kernel::regularisation::form::Form;
use kernel::regularisation::heggie::{
    self as hg, carried_state, Heggie, HgAdvanced, HgConfig, HgP, HgParams, HgQ, DEFAULT_TIME,
};
use kernel::regularisation::logh::{LhConfig, LhParams, LogH};
use kernel::regularisation::pairs::{
    Choice, DefaultOccupant, HeggieTime, LogHMode, Pair, PairTable, Status, STEPPERS,
};
use kernel::regularisation::{Composed, NoRegularisation};
use spirv_std::glam::UVec4;
use validation::negative_control;

// ── Precisions and fixtures ────────────────────────────────────────────────────────────────────────────────────

/// A precision the tests run at.
trait P: WrapperReal + Into<f64> + core::fmt::Debug {
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

/// `x`'s bits, widened exactly.
fn b<R: P>(x: R) -> u64 {
    Into::<f64>::into(x).to_bits()
}

/// Test values for Heggie's step control, as `tests/heggie.rs` takes them: `η = 0.01`, `Δ_sync = 1/8`, the reference's
/// `TINY`, `f = 0.02`, a budget that does not bind, and the landing in force (the clamp, 4 secant retakes, 64
/// halvings), under `time`. Test inputs, not calibrations.
fn values<R: AzReal>(time: HeggieTime) -> HgParams<R> {
    HgParams {
        eta: R::at_width(0.01, 0.01),
        delta_sync: R::at_width(0.125, 0.125),
        tiny: R::at_width(1e-37, 1e-300),
        predictive_f: R::at_width(0.02, 0.02),
        step_budget_rate: R::at_width(1e7, 1e7),
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        time,
    }
}

/// [`values`] under Eq. 22, the default.
struct Eq22;

impl HgConfig for Eq22 {
    fn params<R: AzReal>() -> HgParams<R> {
        values(DEFAULT_TIME)
    }
}

/// [`values`] under Eq. 20.
struct Eq20;

impl HgConfig for Eq20 {
    fn params<R: AzReal>() -> HgParams<R> {
        values(HeggieTime::Eq20)
    }
}

/// [`values`] on a budget of 4 steps, which the first interval's march spends.
struct Spent;

impl HgConfig for Spent {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            n_min: 4,
            step_budget_rate: R::default(),
            ..values(DEFAULT_TIME)
        }
    }
}

/// The step total [`Tight`] allows: more than any one interval of the fixture's march takes (609 at most, measured)
/// and fewer than its eight take together (1627), both of which [`check_budget`] asserts, so that only a total carried
/// across calls spends it. A fixture input, not a threshold.
const TIGHT_STEPS: u32 = 1000;

/// [`values`] on a budget of [`TIGHT_STEPS`] steps at rate 0.
struct Tight;

impl HgConfig for Tight {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            n_min: TIGHT_STEPS,
            step_budget_rate: R::default(),
            ..values(DEFAULT_TIME)
        }
    }
}

/// [`values`] with the landing's retakes off, the secant's and the bisection's: a step that ends the clock past the
/// boundary is kept. Never the occupant's; here it plants a landed clock that is not the boundary's.
struct Overshot;

impl HgConfig for Overshot {
    fn params<R: AzReal>() -> HgParams<R> {
        HgParams {
            secant_tries: 0,
            bisect_tries: 0,
            ..values(DEFAULT_TIME)
        }
    }
}

/// Aarseth–Zare's test values, as `tests/az.rs` takes them.
struct AzValues;

impl AzConfig for AzValues {
    fn params<R: AzReal>() -> AzParams<R> {
        AzParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: R::at_width(0.125, 0.125),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(1e7, 1e7),
            n_min: 1_000_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
        }
    }
}

/// Heggie × RK4 with [`values`] under Eq. 22: the default occupant.
type HgRk4 = DefaultOccupant<Eq22>;

/// Aarseth–Zare + RK4.
type AzRk4 = Composed<Rk4, AarsethZare<AzValues>>;

/// The sync quantum of [`values`].
const DELTA_SYNC: f64 = 0.125;

/// A cadence that counts its calls and never ends the march.
#[derive(Default)]
struct Counting {
    calls: u32,
}

impl<R> Cadence<R> for Counting {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {
        self.calls += 1;
    }
    fn done(&self) -> bool {
        false
    }
}

/// Burrau's three bodies of masses 3, 4, 5 on their triangle, with small momenta summing to zero, in the gauge `M = 1`
/// (masses over 12, lengths over 5), as `tests/az.rs` takes them.
fn burrau<R: P>() -> State<R> {
    let m = [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0];
    let r = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]].map(|x: [f64; 2]| x.map(|c| c / 5.0));
    let p = [[0.03, -0.01], [-0.02, 0.04], [-0.01, -0.03]];
    State {
        m: m.map(R::of),
        r: r.map(|x| x.map(R::of)),
        p: p.map(|x: [f64; 2]| x.map(R::of)),
    }
}

/// The bits of a state's positions and momenta.
fn bits<R: P>(s: &State<R>) -> Vec<u64> {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .map(|&x| b(x))
        .collect()
}

/// The IC of the state `s`.
fn ic_of<R: Copy>(s: &State<R>) -> Ic<R> {
    Ic {
        m: s.m,
        r: s.r,
        p: s.p,
    }
}

/// A sim key for the wrapper's loop: `dt_macro`, `horizon_steps`, and `N_sub = n_max` throughout, so that Heggie's step
/// count is never `N_sub` by accident.
fn key<R: P>(dt_macro: f64, horizon_steps: u32, n_max: u32) -> WrapperUniforms<R> {
    WrapperUniforms {
        dt_macro: R::of(dt_macro),
        horizon_steps,
        n_max,
        thresholds: NsubThresholds {
            rows: [UVec4::splat(f32::MAX.to_bits()); NSUB_TABLE_ROWS],
        },
    }
}

/// The direct march of Burrau's state to boundary `k` under `params`: registered once at `t = 0`.
fn direct<R: P>(k: u32, params: &HgParams<R>) -> (HgAdvanced<R>, u32) {
    let mut cadence = Counting::default();
    let out = hg::march::<Rk4, R, Counting, ExplicitTest>(
        &burrau(),
        k,
        params,
        &mut cadence,
        &mut ExplicitTest,
    );
    (out, cadence.calls)
}

// ── The seam's end, through the wrapper ────────────────────────────────────────────────────────────────────────

/// The wrapper's run by `A`.
fn run_by<A: Advance, R: WrapperReal>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut Counting,
) -> SimStateBaseOf<R> {
    integrate_with::<A, R, Counting>(ic, uniforms, cadence)
}

/// How `A`'s seam call over the first sync interval of `s` ended, as `advance_reaching` reports it with nothing
/// carried: the macro-step from 1 to 2 at `dt_macro = 1/16`.
fn reaching<A: Advance, R: P>(s: &State<R>) -> Reached {
    let params = AdvanceParams {
        dt_macro: R::of(1.0 / 16.0),
        n_sub: 4,
    };
    let geom = Geom::new(&s.m, &s.r);
    A::advance_reaching(s, &geom, 1, 2, &params, &mut Counting::default()).1
}

/// The occupant `A` read by the stepper's rule alone, its end dropped: a macro-step completed when the steps taken are
/// `N_sub`, and no march ever ended (a control's).
#[cfg(feature = "controls")]
struct Dropping<A>(core::marker::PhantomData<A>);

#[cfg(feature = "controls")]
impl<A: Advance> Advance for Dropping<A> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        A::advance(state, geom, t_now, t_target, params, &mut Muted(cadence))
    }
}

/// The cadence with the march's own report of a non-finite end dropped (`Cadence::non_finite`), every other call
/// passed on: [`Dropping`]'s, so no march ever ended.
#[cfg(feature = "controls")]
struct Muted<'a, C>(&'a mut C);

#[cfg(feature = "controls")]
impl<R, C: Cadence<R>> Cadence<R> for Muted<'_, C> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.0.after_step(state, geom);
    }
    fn done(&self) -> bool {
        self.0.done()
    }
    fn sample(&mut self, state: &State<R>, index: u32, quantum: R) {
        self.0.sample(state, index, quantum);
    }
    fn clock(&mut self, completed: u32) {
        self.0.clock(completed);
    }
}

/// Burrau's state with its second body moved onto its first: `dt/dτ` is 0 at registration, so the first interval's
/// entry is not positive and the march ends there, not finite, before any step.
fn coincident<R: P>() -> State<R> {
    let mut s = burrau::<R>();
    s.r[1] = s.r[0];
    s
}

/// [`coincident`] through the wrapper at `dt_macro = 1/16` (`Δ_sync = 1/8`), eight macro-steps, `N_sub = 4`: the first
/// macro-step's target is before the first boundary, so it takes no step and is completed; the second's march ends at
/// its entry, not finite. The run is `sim_failed`, latched at the macro-step it ended in, which is not completed
/// (`t_end_step = 1`), its state the direct march's latched finite one, its detail 2 (a non-finite derived quantity on
/// a finite state, payload §2).
fn check_non_finite_fails<A: Advance, R: P>() {
    let s = coincident::<R>();
    let out = run_by::<A, R>(&ic_of(&s), &key(1.0 / 16.0, 8, 4), &mut Counting::default());
    let reported = reaching::<A, R>(&s);
    let latched = hg::march::<Rk4, R, Counting, ExplicitTest>(
        &s,
        1,
        &values(DEFAULT_TIME),
        &mut Counting::default(),
        &mut ExplicitTest,
    );
    assert_eq!(
        (latched.count.end, latched.count.non_finite_entries),
        (End::NonFinite, 1),
        "{}: the fixture's direct march ends not finite at its entry",
        R::NAME
    );
    println!(
        "{}: heggie_seam non-finite: state {} (criterion {STATE_SIM_FAILED}), t_end_step {} (criterion 1), detail {} \
         (criterion 2), total_substeps {} (criterion 0)",
        R::NAME,
        sd_state(out.packed_a),
        tm_t_end_step(out.times),
        sd_detail(out.packed_a),
        out.total_substeps
    );
    assert_eq!(
        sd_state(out.packed_a),
        STATE_SIM_FAILED,
        "a march ended non-finite fails the run: state {}",
        sd_state(out.packed_a)
    );
    assert_eq!(
        tm_t_end_step(out.times),
        1,
        "the run latches at the macro-step its march ended in"
    );
    assert_eq!(
        sd_detail(out.packed_a),
        2,
        "a non-finite derived quantity on a finite state"
    );
    assert_eq!(out.total_substeps, 0, "no step is kept");
    assert_eq!(
        bits(&State {
            m: s.m,
            r: out.r,
            p: out.p
        }),
        bits(&latched.state),
        "the state is the march's latched finite one"
    );
    assert_eq!(
        reported,
        Reached::NonFinite,
        "the seam reports the march's end through advance_reaching"
    );
}

#[test]
fn heggie_seam_non_finite_fails() {
    check_non_finite_fails::<HgRk4, f64>();
    check_non_finite_fails::<HgRk4, f32>();
}

negative_control!(
    heggie_seam_non_finite_fails,
    "a seam that drops the march's end, the run never failed",
    expected = "a march ended non-finite fails the run",
    check_non_finite_fails::<Dropping<HgRk4>, f64>()
);

/// Heggie × RK4 on a budget of 4 steps.
type SpentRk4 = DefaultOccupant<Spent>;

/// Burrau's state through the wrapper at `dt_macro = 1/16`, eight macro-steps, on a budget of 4 steps: the first
/// macro-step takes no step and is completed, the second's march spends its budget, and the copy is never run on:
/// the run stops there, `running` as it stood, at `t_end_step = 1`, with the state and steps of the direct
/// march on that budget.
fn check_exhausted_stops<A: Advance, R: P>() {
    let s = burrau::<R>();
    let mut cadence = Counting::default();
    let out = run_by::<A, R>(&ic_of(&s), &key(1.0 / 16.0, 8, 4), &mut cadence);
    let reported = reaching::<A, R>(&s);
    let (once, _) = direct::<R>(8, &Spent::params());
    assert_eq!(
        (once.count.end, once.boundary),
        (End::Exhausted, 0),
        "{}: the fixture's direct march spends its budget in its first interval",
        R::NAME
    );
    println!(
        "{}: heggie_seam exhausted: state {} (criterion {STATE_RUNNING}), t_end_step {} (criterion 1), \
         total_substeps {} (criterion {}, the direct march's kept steps)",
        R::NAME,
        sd_state(out.packed_a),
        tm_t_end_step(out.times),
        out.total_substeps,
        once.count.accepted
    );
    assert_eq!(
        sd_state(out.packed_a),
        STATE_RUNNING,
        "a spent copy stops as it stood: state {}",
        sd_state(out.packed_a)
    );
    assert_eq!(
        tm_t_end_step(out.times),
        1,
        "the spent macro-step is not completed"
    );
    assert_eq!(
        (out.total_substeps, cadence.calls),
        (once.count.accepted, once.count.accepted),
        "one march, never run on"
    );
    assert_eq!(
        bits(&State {
            m: s.m,
            r: out.r,
            p: out.p
        }),
        bits(&once.state),
        "the state the march latched"
    );
    assert_eq!(
        reported,
        Reached::Exhausted,
        "the seam reports the spent budget through advance_reaching"
    );
}

#[test]
fn heggie_seam_exhausted_stops() {
    check_exhausted_stops::<SpentRk4, f64>();
    check_exhausted_stops::<SpentRk4, f32>();
}

negative_control!(
    heggie_seam_exhausted_stops,
    "a seam that drops the spent budget, the copy run on",
    expected = "a spent copy stops as it stood",
    check_exhausted_stops::<Dropping<SpentRk4>, f64>()
);

// ── regularised_state_carried: the seam against one direct march ───────────────────────────────────────────────

/// One `ADVANCE` call of the occupant under test, from `state` at macro-step `i` to `i + 1`, with the trajectory's
/// buffer.
type SeamCall<R> =
    fn(&State<R>, u32, &AdvanceParams<R>, &mut Counting, &mut Carried<R>) -> (Advanced<R>, Reached);

/// `A`'s call through the seam.
fn seam<A: Advance, R: P>(
    state: &State<R>,
    i: u32,
    params: &AdvanceParams<R>,
    cadence: &mut Counting,
    carried: &mut Carried<R>,
) -> (Advanced<R>, Reached) {
    A::advance_carrying(
        state,
        &Geom::new(&state.m, &state.r),
        i,
        i + 1,
        params,
        cadence,
        carried,
    )
}

/// A seam that rebuilds the regularised state from `(m, r, p)` at each call, Heggie's clock included: the buffer's
/// registration is dropped before the call, its counts kept (a control's).
#[cfg(feature = "controls")]
fn rebuilding<A: Advance, R: P>(
    state: &State<R>,
    i: u32,
    params: &AdvanceParams<R>,
    cadence: &mut Counting,
    carried: &mut Carried<R>,
) -> (Advanced<R>, Reached) {
    carried.registered = 0;
    seam::<A, R>(state, i, params, cadence, carried)
}

/// A seam whose step total starts afresh at each call (a control's).
#[cfg(feature = "controls")]
fn forgetting<A: Advance, R: P>(
    state: &State<R>,
    i: u32,
    params: &AdvanceParams<R>,
    cadence: &mut Counting,
    carried: &mut Carried<R>,
) -> (Advanced<R>, Reached) {
    carried.steps = 0;
    seam::<A, R>(state, i, params, cadence, carried)
}

/// What a run of seam calls left.
struct Seamed<R> {
    state: State<R>,
    carried: Carried<R>,
    /// The steps the cadence followed, summed over the calls.
    substeps: u32,
    /// The cadence's calls.
    calls: u32,
    /// How the last call ended, and its index.
    last: (Reached, u32),
}

/// Burrau's state through `n` seam calls at `dt_macro`, from an unregistered buffer, stopping where a call's march
/// ended.
fn seamed<R: P>(call: SeamCall<R>, dt_macro: f64, n: u32) -> Seamed<R> {
    let params = AdvanceParams {
        dt_macro: R::of(dt_macro),
        n_sub: 4,
    };
    let mut state = burrau::<R>();
    let mut carried = Carried::unregistered();
    let mut cadence = Counting::default();
    let mut substeps = 0;
    let mut last = (Reached::Target, 0);
    for i in 0..n {
        if last.0.ended() {
            break;
        }
        let (out, reached) = call(&state, i, &params, &mut cadence, &mut carried);
        state = out.state;
        substeps += out.substeps;
        last = (reached, i);
    }
    Seamed {
        state,
        carried,
        substeps,
        calls: cadence.calls,
        last,
    }
}

/// The bits of Heggie's `(Q, P, h)` and clock, in the buffer's order.
fn regularised_bits<R: P>(q: &HgQ<R>, p: &HgP<R>, h: R) -> Vec<u64> {
    q.q.iter()
        .chain(p.p.iter())
        .flatten()
        .map(|&x| b(x))
        .chain([b(h), b(q.t)])
        .collect()
}

/// Heggie × RK4 to boundary `k` in `n` seam calls at `dt_macro` against one direct march registered once at `t = 0`:
/// the Cartesian state, `(Q, P, h)` and the clock bitwise the direct march's, with its step total, its kept steps and
/// 0 re-registrations. The criterion is bitwise equality: 0 components differing.
fn check_carried<R: P>(call: SeamCall<R>, dt_macro: f64, n: u32, k: u32) {
    let (want, want_calls) = direct::<R>(k, &values(DEFAULT_TIME));
    assert_eq!(
        (want.count.end, want.boundary),
        (End::Marching, k),
        "the direct march reaches boundary {k}"
    );
    let got = seamed::<R>(call, dt_macro, n);
    let (q, p, h) = carried_state(&got.carried);
    let differing = bits(&got.state)
        .iter()
        .zip(bits(&want.state))
        .filter(|(x, y)| **x != *y)
        .count();
    let regularised_differing = regularised_bits(&q, &p, h)
        .iter()
        .zip(regularised_bits(&want.q, &want.p, want.system.h))
        .filter(|(x, y)| **x != *y)
        .count();
    println!(
        "{}: regularised_state_carried, {n} calls at dt_macro {dt_macro} to boundary {k}: Cartesian components \
         differing from the direct march {differing} of 12 (criterion 0), regularised (Q, P, h, clock) differing \
         {regularised_differing} of 14 (criterion 0), re-registrations {} (criterion 0), steps {} (direct march {}), \
         kept steps {} (direct march {})",
        R::NAME,
        got.carried.re_registrations,
        got.carried.steps,
        want.count.steps,
        got.substeps,
        want.count.accepted
    );
    assert_eq!(
        differing,
        0,
        "{}: {n} calls with the carried state are not one direct march registered once at t = 0",
        R::NAME
    );
    assert_eq!(
        regularised_differing, 0,
        "the carried (Q, P, h) and clock are not the direct march's"
    );
    assert_eq!(
        (got.carried.re_registrations, got.carried.registered),
        (0, 1),
        "registered once: no re-registration"
    );
    assert_eq!(
        (got.carried.steps, got.substeps, got.calls),
        (want.count.steps, want.count.accepted, want_calls),
        "the step total, the kept steps and the cadence's calls are the direct march's"
    );
    assert!(want.count.steps > 0, "the march stepped");
    assert_eq!(
        got.last.0,
        Reached::Target,
        "the last call reached its target"
    );
}

/// The arrangements: sixteen calls of half a sync interval each, every second one marching; four calls of two
/// intervals each; three calls of 3/8, which reach boundaries 3, 6 and 9; and twelve calls of 0.1, a macro-step that
/// is no multiple or fraction of the sync quantum, which reach `t = 1.2` and so boundary 9 (`1.125 ≤ 1.2 < 1.25`).
fn check_carried_all<R: P>(call: SeamCall<R>) {
    check_carried::<R>(call, 1.0 / 16.0, 16, 8);
    check_carried::<R>(call, 1.0 / 4.0, 4, 8);
    check_carried::<R>(call, 3.0 / 8.0, 3, 9);
    check_carried::<R>(call, 0.1, 12, 9);
}

#[test]
fn regularised_state_carried() {
    check_carried_all::<f64>(seam::<HgRk4, f64>);
    check_carried_all::<f32>(seam::<HgRk4, f32>);
}

negative_control!(
    regularised_state_carried,
    "a seam that rebuilds the regularised state from (m, r, p) at each call",
    expected = "are not one direct march registered once at t = 0",
    check_carried_all::<f64>(rebuilding::<HgRk4, f64>)
);

// ── regularised_state_carried_budget ───────────────────────────────────────────────────────────────────────────

/// Heggie × RK4 on [`Tight`]'s budget.
type TightRk4 = DefaultOccupant<Tight>;

/// On a budget the direct march spends after its first landing and before its last, the seam's march, one sync
/// interval per call, spends it at the same step: the same call, the state bitwise, the same step total.
fn check_budget<R: P>(call: SeamCall<R>) {
    let (want, _) = direct::<R>(8, &Tight::params());
    let (free, _) = direct::<R>(8, &values(DEFAULT_TIME));
    assert_eq!(
        want.count.end,
        End::Exhausted,
        "the direct march spends its budget"
    );
    assert!(
        want.boundary >= 1 && want.boundary < 8,
        "the fixture spends its budget after a landing and before the last: boundary {}",
        want.boundary
    );
    assert!(
        free.count.largest_interval < TIGHT_STEPS,
        "no one interval spends the budget on its own: {} steps",
        free.count.largest_interval
    );
    let got = seamed::<R>(call, DELTA_SYNC, 8);
    println!(
        "{}: regularised_state_carried_budget: direct march spent at boundary {} after {} steps; the seam's march \
         ended {:?} in call {} after {} steps (criterion: Exhausted, call {}, {} steps); largest interval {} steps \
         against a budget of {TIGHT_STEPS}",
        R::NAME,
        want.boundary,
        want.count.steps,
        got.last.0,
        got.last.1,
        got.carried.steps,
        want.boundary,
        want.count.steps,
        free.count.largest_interval
    );
    assert_eq!(
        got.last,
        (Reached::Exhausted, want.boundary),
        "the running step total is carried: the seam's march spends its budget where the direct march does"
    );
    assert_eq!(got.carried.steps, want.count.steps, "at the same step");
    assert_eq!(
        bits(&got.state),
        bits(&want.state),
        "the state latched, bitwise"
    );
}

#[test]
fn regularised_state_carried_budget() {
    check_budget::<f64>(seam::<TightRk4, f64>);
    check_budget::<f32>(seam::<TightRk4, f32>);
}

negative_control!(
    regularised_state_carried_budget,
    "a seam whose step total starts afresh at each call",
    expected = "the running step total is carried",
    check_budget::<f64>(forgetting::<TightRk4, f64>)
);

// ── Through the wrapper: live march, catch-up and re-boot after eviction ───────────────────────────────────────

/// A trajectory as stored: its `SimState` and the carried buffer beside it.
#[derive(Clone, Copy)]
struct Stored<R: P> {
    sim: SimStateBaseOf<R>,
    carried: Carried<R>,
}

/// Every stored bit of a trajectory: the `SimState`'s fields and the buffer's.
fn stored_bits<R: P>(s: &Stored<R>) -> Vec<u64> {
    let sim = &s.sim;
    let floats = sim
        .r
        .iter()
        .chain(sim.p.iter())
        .flatten()
        .copied()
        .chain([
            sim.S,
            sim.theta,
            sim.mean_y,
            sim.C_ty,
            sim.E_0,
            sim.Lz_0,
            sim.closure_min,
        ])
        .chain(s.carried.x);
    let words = [
        sim.packed_a,
        sim.packed_b,
        sim.times,
        sim.total_substeps,
        u32::from(sim.closure_step),
        u32::from(sim._reserved),
        sim.coll_floor_bits,
        s.carried.steps,
        s.carried.re_registrations,
        s.carried.registered,
        s.carried.reserved,
    ];
    floats
        .map(b)
        .chain(words.into_iter().map(u64::from))
        .collect()
}

/// The macro-step and horizon of the wrapper runs: `dt_macro = 1/16` to `t = 1`, eight sync boundaries.
const DT_MACRO: f64 = 1.0 / 16.0;
const HORIZON: u32 = 16;

/// Catch-up: one call from the IC to the horizon.
fn catch_up<A: Advance, R: P>() -> Stored<R> {
    let mut carried = Carried::unregistered();
    let sim = integrate_carrying::<A, R, Counting>(
        &ic_of(&burrau()),
        &key(DT_MACRO, HORIZON, 4),
        &mut Counting::default(),
        &mut carried,
    );
    Stored { sim, carried }
}

/// The trajectory as the wrapper stores it at `t = 0`: no macro-step taken, the buffer unregistered.
fn at_start<A: Advance, R: P>() -> Stored<R> {
    let mut carried = Carried::unregistered();
    let sim = integrate_carrying::<A, R, Counting>(
        &ic_of(&burrau()),
        &key(DT_MACRO, 0, 4),
        &mut Counting::default(),
        &mut carried,
    );
    Stored { sim, carried }
}

/// `stored` resumed for `steps` more macro-steps.
fn resumed<A: Advance, R: P>(stored: &Stored<R>, steps: u32) -> Stored<R> {
    let mut carried = stored.carried;
    let sim = integrate_resumed_carrying::<A, R, Counting>(
        &ic_of(&burrau()),
        &key(DT_MACRO, HORIZON, 4),
        &mut Counting::default(),
        &stored.sim,
        &mut carried,
        steps,
    );
    Stored { sim, carried }
}

/// The live march: one macro-step per call, each resumed from the last; every call's stored trajectory, from `t = 0`.
fn live<A: Advance, R: P>() -> Vec<Stored<R>> {
    let mut series = vec![at_start::<A, R>()];
    for _ in 0..HORIZON {
        let next = resumed::<A, R>(series.last().expect("the start"), 1);
        series.push(next);
    }
    series
}

/// What a re-boot restores of an evicted trajectory's carried buffer.
#[derive(Clone, Copy, Debug)]
enum Restore {
    /// The buffer, whole.
    Whole,
    /// Nothing: the buffer starts unregistered (a control's).
    #[cfg_attr(not(feature = "controls"), allow(dead_code))]
    Nothing,
    /// `Q`, `P`, `h` and the counts, but not the interval clock, which starts at 0.
    NoClock,
}

/// An evicted trajectory read back: every stored value through its bits, the buffer as `restore` has it.
fn rebooted<R: P>(stored: &Stored<R>, restore: Restore) -> Stored<R> {
    let back = |x: R| R::of(f64::from_bits(b(x)));
    let whole = Carried {
        x: stored.carried.x.map(back),
        ..stored.carried
    };
    let carried = match restore {
        Restore::Whole => whole,
        Restore::Nothing => Carried::unregistered(),
        Restore::NoClock => {
            let mut c = whole;
            c.x[13] = R::default();
            c
        }
    };
    Stored {
        sim: stored.sim,
        carried,
    }
}

/// Where the trajectory is evicted: after 7 macro-steps, three boundaries landed and the next call one that marches.
const EVICTED_AT: u32 = 7;

/// Heggie × RK4 through the wrapper: catch-up is the direct march registered once at `t = 0`; the live march, one
/// macro-step per resumed call, is the catch-up; and a re-boot after eviction at [`EVICTED_AT`], resumed to the
/// horizon, is the live march; each bitwise, over every stored value, with 0 re-registrations.
fn check_reboot<R: P>(restore: Restore) {
    let (want, _) = direct::<R>(8, &values(DEFAULT_TIME));
    let caught = catch_up::<HgRk4, R>();
    let series = live::<HgRk4, R>();
    let last = series.last().expect("the horizon");
    let evicted = &series[EVICTED_AT as usize];
    let back = resumed::<HgRk4, R>(&rebooted(evicted, restore), HORIZON - EVICTED_AT);
    let differing = |x: &Stored<R>, y: &Stored<R>| {
        stored_bits(x)
            .iter()
            .zip(stored_bits(y))
            .filter(|(p, q)| **p != *q)
            .count()
    };
    let total = stored_bits(&caught).len();
    let caught_state = State {
        m: want.state.m,
        r: caught.sim.r,
        p: caught.sim.p,
    };
    println!(
        "{}: regularised_state_carried_reboot ({restore:?}): stored values differing, live against catch-up {} of \
         {total} (criterion 0), re-boot against live {} of {total} (criterion 0); re-registrations catch-up {}, live \
         {}, re-boot {} (criterion 0 each); catch-up state {} t_end_step {} (criterion {STATE_BOUNDED}, {HORIZON})",
        R::NAME,
        differing(last, &caught),
        differing(&back, last),
        caught.carried.re_registrations,
        last.carried.re_registrations,
        back.carried.re_registrations,
        sd_state(caught.sim.packed_a),
        tm_t_end_step(caught.sim.times),
    );
    assert_eq!(
        bits(&caught_state),
        bits(&want.state),
        "catch-up through the wrapper is the direct march registered once at t = 0, bitwise"
    );
    assert_eq!(
        (
            sd_state(caught.sim.packed_a),
            tm_t_end_step(caught.sim.times)
        ),
        (STATE_BOUNDED, HORIZON),
        "every macro-step that reached its boundary is completed"
    );
    assert_eq!(
        caught.sim.total_substeps, want.count.accepted,
        "the steps are the march's"
    );
    assert_eq!(
        stored_bits(last),
        stored_bits(&caught),
        "the live march is the catch-up, bitwise"
    );
    assert!(
        evicted.carried.registered == 1 && evicted.carried.steps > 0,
        "the evicted trajectory has marched"
    );
    assert_eq!(
        stored_bits(&back),
        stored_bits(last),
        "{}: a re-boot after eviction is the live march, bitwise",
        R::NAME
    );
    assert_eq!(
        (
            caught.carried.re_registrations,
            last.carried.re_registrations,
            back.carried.re_registrations
        ),
        (0, 0, 0),
        "no re-registration"
    );
}

#[test]
fn regularised_state_carried_reboot() {
    check_reboot::<f64>(Restore::Whole);
    check_reboot::<f32>(Restore::Whole);
}

negative_control!(
    regularised_state_carried_reboot,
    "a re-boot that does not restore the carried state, which re-registers from the stored (r, p)",
    expected = "a re-boot after eviction is the live march, bitwise",
    check_reboot::<f64>(Restore::Nothing)
);

// ── regularised_state_carried_clock ────────────────────────────────────────────────────────────────────────────

/// What the live march of `A` shows of its interval clock.
struct Clocks {
    /// The calls that marched, each one landing.
    landings: u32,
    /// Those whose landed clock is not the boundary's `Δ_sync`, bitwise.
    off: u32,
    /// The macro-step the trajectory is evicted at for the clock-less re-boot: the first call's end whose landed clock
    /// is off the boundary, or [`EVICTED_AT`] where there is none.
    evicted_at: u32,
    /// The clock stored at that eviction.
    evicted_clock: f64,
    /// The stored values by which a re-boot there with `Q`, `P` and `h` restored but not the clock differs from the
    /// live march at the horizon.
    clockless_differing: usize,
}

/// The live march of `A` at `R`, its clock read at every call's end.
fn clocks<A: Advance, R: P>() -> Clocks {
    let series = live::<A, R>();
    let (mut landings, mut off) = (0, 0);
    let mut evicted_at = EVICTED_AT;
    for (i, pair) in series.windows(2).enumerate() {
        if pair[1].carried.steps > pair[0].carried.steps {
            landings += 1;
            if b(pair[1].carried.x[13]) != DELTA_SYNC.to_bits() {
                if off == 0 {
                    evicted_at = i as u32 + 1;
                }
                off += 1;
            }
        }
    }
    let evicted = &series[evicted_at as usize];
    let back = resumed::<A, R>(&rebooted(evicted, Restore::NoClock), HORIZON - evicted_at);
    let last = series.last().expect("the horizon");
    Clocks {
        landings,
        off,
        evicted_at,
        evicted_clock: evicted.carried.x[13].into(),
        clockless_differing: stored_bits(&back)
            .iter()
            .zip(stored_bits(last))
            .filter(|(p, q)| **p != *q)
            .count(),
    }
}

/// Heggie × RK4 with the landing's retakes off ([`Overshot`]).
type OvershotRk4 = DefaultOccupant<Overshot>;

/// The interval clock across calls. `own` is the occupant's live march, `planted` one whose landing keeps a step past
/// the boundary.
///
/// The property guarded: the occupant's landed clock is the boundary's at every call's end, bitwise (the landing puts
/// the state on the boundary with its clock; Part 2b, Heggie's step 8).
///
/// The measurement beside it, of the control the task asked for (a re-boot that restores `Q`, `P` and `h` but not the
/// clock must fail, once a landing whose clock differs from the boundary is shown): on the occupant's fixture no
/// landing has such a clock, so one is planted, and with it shown, the clock-less re-boot still does not differ from
/// the live march: every interval restarts its clock at 0 before it reads it (`heggie::march_on`). So that control
/// cannot fail and is void; the test asserts what was measured instead of passing over it.
fn check_clock<R: P>(own: Clocks, planted: Clocks) {
    println!(
        "{}: regularised_state_carried_clock: the occupant's landings {} with a clock off the boundary {} (criterion \
         0); evicted after macro-step {} with clock {:e}, the re-boot without the clock differs from live in {} stored \
         values. Planted (retakes off): landings {} with a clock off the boundary {} (precondition: at least 1); \
         evicted after macro-step {} with clock {:e} (Δ_sync + {:e}), the re-boot without the clock differs from live \
         in {} stored values (measured 0: the control \"restores Q, P, h but not the clock\" is void, every interval \
         restarts its clock at 0)",
        R::NAME,
        own.landings,
        own.off,
        own.evicted_at,
        own.evicted_clock,
        own.clockless_differing,
        planted.landings,
        planted.off,
        planted.evicted_at,
        planted.evicted_clock,
        planted.evicted_clock - DELTA_SYNC,
        planted.clockless_differing
    );
    assert_eq!(own.landings, 8, "the live march lands on eight boundaries");
    assert_eq!(
        own.off,
        0,
        "{}: the landed clock is the boundary's at every call's end",
        R::NAME
    );
    assert!(
        planted.off >= 1 && planted.evicted_clock != DELTA_SYNC && planted.evicted_at < HORIZON,
        "the planted march is evicted at a landing whose clock is not the boundary's, with marching left to do"
    );
    assert_eq!(
        (own.clockless_differing, planted.clockless_differing),
        (0, 0),
        "a re-boot without the interval clock is the live march: no interval reads the clock it starts with"
    );
}

#[test]
fn regularised_state_carried_clock() {
    check_clock::<f64>(clocks::<HgRk4, f64>(), clocks::<OvershotRk4, f64>());
    check_clock::<f32>(clocks::<HgRk4, f32>(), clocks::<OvershotRk4, f32>());
}

negative_control!(
    regularised_state_carried_clock,
    "a landing with its retakes off, which keeps a clock past the boundary",
    expected = "the landed clock is the boundary's at every call's end",
    check_clock::<f64>(clocks::<OvershotRk4, f64>(), clocks::<OvershotRk4, f64>())
);

// ── regularised_state_carried_layout ───────────────────────────────────────────────────────────────────────────

/// Heggie's write into the buffer, as the seam makes it.
type Pack = fn(&mut Carried<f64>, &HgQ<f64>, &HgP<f64>, f64, u32);

/// The buffer's element as payload §1 states it: 72 bytes aligned to 4 at f32, 128 aligned to 8 at f64; every field 0
/// before a registration; and Heggie's numbers in the order `Q₀, Q₁, Q₂, P₀, P₁, P₂, h, t`, read back as written.
fn check_layout(pack: Pack) {
    use core::mem::{align_of, offset_of, size_of};
    let sizes = (
        size_of::<Carried<f32>>(),
        align_of::<Carried<f32>>(),
        size_of::<Carried<f64>>(),
        align_of::<Carried<f64>>(),
    );
    println!(
        "regularised_state_carried_layout: (size, align) at f32 and f64 {sizes:?} (criterion (72, 4, 128, 8))"
    );
    assert_eq!(sizes, (72, 4, 128, 8), "the element's widths");
    let offsets = |x: usize, steps: usize, count: usize, registered: usize, reserved: usize| {
        [x, steps, count, registered, reserved]
    };
    assert_eq!(
        (
            offsets(
                offset_of!(Carried<f32>, x),
                offset_of!(Carried<f32>, steps),
                offset_of!(Carried<f32>, re_registrations),
                offset_of!(Carried<f32>, registered),
                offset_of!(Carried<f32>, reserved),
            ),
            offsets(
                offset_of!(Carried<f64>, x),
                offset_of!(Carried<f64>, steps),
                offset_of!(Carried<f64>, re_registrations),
                offset_of!(Carried<f64>, registered),
                offset_of!(Carried<f64>, reserved),
            ),
        ),
        ([0, 56, 60, 64, 68], [0, 112, 116, 120, 124]),
        "the fields' offsets are payload §1's table"
    );
    let fresh = Carried::<f64>::unregistered();
    assert_eq!(
        (
            fresh.x,
            fresh.steps,
            fresh.re_registrations,
            fresh.registered,
            fresh.reserved
        ),
        ([0.0; 14], 0, 0, 0, 0),
        "an unregistered buffer is all zero"
    );
    let q = HgQ {
        q: [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]],
        t: 14.0,
    };
    let p = HgP {
        p: [[7.0, 8.0], [9.0, 10.0], [11.0, 12.0]],
    };
    let mut carried = Carried::unregistered();
    pack(&mut carried, &q, &p, 13.0, 99);
    assert_eq!(
        carried.x,
        [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0],
        "Heggie's numbers in the buffer's order: Q, P, then h, then the clock"
    );
    assert_eq!(
        (carried.steps, carried.registered, carried.re_registrations),
        (99, 1, 0),
        "the step total and the registration are written, the count left"
    );
    assert_eq!(
        carried_state(&carried),
        (q, p, 13.0),
        "read back as written"
    );
}

#[test]
fn regularised_state_carried_layout() {
    check_layout(hg::carry::<f64>);
}

negative_control!(
    regularised_state_carried_layout,
    "a write with h and the clock swapped",
    expected = "Heggie's numbers in the buffer's order",
    check_layout(|carried, q, p, h, steps| {
        hg::carry(carried, &HgQ { q: q.q, t: h }, p, q.t, steps)
    })
);

// ── regularised_re_registrations_visible ───────────────────────────────────────────────────────────────────────

/// The wrapper's run from the IC by the occupant under test, the buffer handed back.
type CarryingRun =
    fn(&Ic<f64>, &WrapperUniforms<f64>, &mut Counting, &mut Carried<f64>) -> SimStateBaseOf<f64>;

/// The occupant `A` through a seam that does not pass the buffer on (a control's).
#[cfg(feature = "controls")]
struct Uncounted<A>(core::marker::PhantomData<A>);

#[cfg(feature = "controls")]
impl<A: Advance> Advance for Uncounted<A> {
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

    fn advance_reaching<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> (Advanced<R>, Reached) {
        A::advance_reaching(state, geom, t_now, t_target, params, cadence)
    }
}

/// The count a consumer reads beside the `SimState` after a run to `t = 1`, eight sync intervals: 0 under Heggie × RK4,
/// which registers once; 7 under Aarseth–Zare + RK4, one per boundary it marched on from after `t = 0`; and 1 for a
/// Heggie call handed an unregistered buffer at a boundary after 0.
fn check_visible(heggie: CarryingRun, az: CarryingRun) {
    let ic = ic_of(&burrau::<f64>());
    let uniforms = key::<f64>(DT_MACRO, HORIZON, 4);
    let count = |run: CarryingRun| {
        let mut carried = Carried::unregistered();
        let out = run(&ic, &uniforms, &mut Counting::default(), &mut carried);
        assert_eq!(
            tm_t_end_step(out.times),
            HORIZON,
            "the run reaches the horizon"
        );
        carried.re_registrations
    };
    let (of_heggie, of_az) = (count(heggie), count(az));
    let mut late = Carried::unregistered();
    seam::<HgRk4, f64>(
        &burrau(),
        4,
        &AdvanceParams {
            dt_macro: DT_MACRO,
            n_sub: 4,
        },
        &mut Counting::default(),
        &mut late,
    );
    println!(
        "regularised_re_registrations_visible: Heggie × RK4 {of_heggie} (criterion 0), Aarseth–Zare + RK4 {of_az} \
         (criterion 7), a Heggie call with nothing carried at boundary 2: {} (criterion 1)",
        late.re_registrations
    );
    assert_eq!(of_heggie, 0, "Heggie registers once");
    assert_eq!(
        of_az, 7,
        "Aarseth–Zare's count is one per sync boundary marched on from after t = 0"
    );
    assert_eq!(
        late.re_registrations, 1,
        "a registration after t = 0 is a re-registration, and is counted"
    );
}

#[test]
fn regularised_re_registrations_visible() {
    check_visible(
        integrate_carrying::<HgRk4, f64, Counting>,
        integrate_carrying::<AzRk4, f64, Counting>,
    );
}

negative_control!(
    regularised_re_registrations_visible,
    "a seam that does not pass the buffer on, so Aarseth–Zare's count reads 0",
    expected = "Aarseth–Zare's count is one per sync boundary marched on from after t = 0",
    check_visible(
        integrate_carrying::<HgRk4, f64, Counting>,
        integrate_carrying::<Uncounted<AzRk4>, f64, Counting>,
    )
);

// ── regularisation_default ─────────────────────────────────────────────────────────────────────────────────────

/// One call of `A` through the seam over one sync interval (`dt_macro = 1/8`, `N_sub = 4`) from Burrau's state: the
/// answer and the cadence's calls.
fn one_call<A: Advance, R: P>() -> (Advanced<R>, u32) {
    let s = burrau::<R>();
    let mut cadence = Counting::default();
    let (out, _) = seam::<A, R>(
        &s,
        0,
        &AdvanceParams {
            dt_macro: R::of(DELTA_SYNC),
            n_sub: 4,
        },
        &mut cadence,
        &mut Carried::unregistered(),
    );
    (out, cadence.calls)
}

/// The default pair is Heggie × RK4 under Eq. 22 at n = 3/2, measured good and carrying no warning; Eq. 20 is
/// selectable; and the default occupant's type runs, with the profile of RK4's form on a regularised system, owning
/// its time mapping.
fn check_default(default: Pair) {
    let rk4 = Selection::Production(Occupant::Rk4);
    println!("regularisation_default: the default pair is {default:?}");
    assert_eq!(
        (default.stepper, default.regularisation),
        (rk4, Choice::Heggie(HeggieTime::Eq22)),
        "the default occupant is Heggie × RK4 under Eq. 22"
    );
    assert_eq!(
        DEFAULT_TIME,
        HeggieTime::Eq22,
        "Heggie's default time transformation"
    );
    let table = PairTable::part_2b();
    assert_eq!(default.status(&table), Status::MeasuredGood);
    assert_eq!(default.warning(&table), None);
    assert!(default.owns_time_mapping());
    let eq20 = Pair::select(rk4, Choice::Heggie(HeggieTime::Eq20));
    assert_eq!(eq20.regularisation, Choice::Heggie(HeggieTime::Eq20));
    let profile = DefaultOccupant::<Eq22>::profile();
    assert_eq!(
        profile,
        kernel::occupant::CapabilityProfile {
            owns_time_mapping: true,
            ..Form::Rk4.pair_profile()
        },
        "the default occupant's profile is RK4's on a regularised system"
    );
    for (name, (out, calls)) in [
        ("Eq. 22", one_call::<DefaultOccupant<Eq22>, f64>()),
        ("Eq. 20", one_call::<DefaultOccupant<Eq20>, f64>()),
    ] {
        assert!(
            out.substeps > 0 && calls == out.substeps,
            "Heggie × RK4 under {name} runs, the cadence after each kept step"
        );
        assert_ne!(
            bits(&out.state),
            bits(&burrau::<f64>()),
            "{name}: the state moved"
        );
    }
    assert_ne!(
        bits(&one_call::<DefaultOccupant<Eq22>, f64>().0.state),
        bits(&one_call::<DefaultOccupant<Eq20>, f64>().0.state),
        "the two time transformations are two marches"
    );
}

#[test]
fn regularisation_default() {
    check_default(Pair::DEFAULT);
}

negative_control!(
    regularisation_default,
    "a default of Heggie × KDK, the pair measured not to work",
    expected = "the default occupant is Heggie × RK4 under Eq. 22",
    check_default(Pair::select(
        Selection::Production(Occupant::Kdk),
        Choice::Heggie(DEFAULT_TIME)
    ))
);

/// `$f::<A, R>()` with `A` the stepper `$stepper` selects under the regularisation type `$g`.
macro_rules! by_stepper {
    ($stepper:expr, $g:ty, $r:ty) => {
        match $stepper {
            Selection::Debug(DebugOccupant::Euler) => one_call::<Composed<Euler, $g>, $r>(),
            Selection::Production(Occupant::Kdk) => one_call::<Composed<Kdk, $g>, $r>(),
            Selection::Production(Occupant::Yoshida4) => one_call::<Composed<Yoshida4, $g>, $r>(),
            Selection::Production(Occupant::Yoshida6) => one_call::<Composed<Yoshida6, $g>, $r>(),
            Selection::Production(Occupant::Rk4) => one_call::<Composed<Rk4, $g>, $r>(),
        }
    };
}

/// logH's test values, [`values`]' own, in the logH mode (`TTL` false) or the TTL mode (`TTL` true).
struct LhValues<const TTL: bool>;

impl<const TTL: bool> LhConfig for LhValues<TTL> {
    fn params<R: AzReal>() -> LhParams<R> {
        let v = values::<R>(DEFAULT_TIME);
        LhParams {
            eta: v.eta,
            delta_sync: v.delta_sync,
            tiny: v.tiny,
            predictive_f: v.predictive_f,
            step_budget_rate: v.step_budget_rate,
            n_min: v.n_min,
            clamp: v.clamp,
            secant_tries: v.secant_tries,
            bisect_tries: v.bisect_tries,
            mode: if TTL { LogHMode::Ttl } else { LogHMode::LogH },
        }
    }
}

/// One seam call of the occupant `pair` selects, at `R`. Every pair is defined: under logH, KDK is its own
/// drift–kick–drift leapfrog, Yoshida-4 and Yoshida-6 that leapfrog composed, RK4 and Euler the classical steps on
/// `(r, p, t, W)` (integrator_contract Part 2b, logH's steps).
fn run_pair<R: P>(pair: Pair) -> (Advanced<R>, u32) {
    match pair.regularisation {
        Choice::None => by_stepper!(pair.stepper, NoRegularisation, R),
        Choice::AarsethZare => by_stepper!(pair.stepper, AarsethZare<AzValues>, R),
        Choice::Heggie(HeggieTime::Eq20) => by_stepper!(pair.stepper, Heggie<Eq20>, R),
        Choice::Heggie(HeggieTime::Eq22) => by_stepper!(pair.stepper, Heggie<Eq22>, R),
        Choice::LogH(LogHMode::LogH) => by_stepper!(pair.stepper, LogH<LhValues<false>>, R),
        Choice::LogH(LogHMode::Ttl) => by_stepper!(pair.stepper, LogH<LhValues<true>>, R),
    }
}

/// Every stepper × regularisation pair is selected and runs a call through the seam at `R`: the state moves and stays
/// finite, and the cadence is called once per kept step. The criterion, written before the run: 30 pairs run, the 5
/// steppers × `none`, Aarseth–Zare, Heggie Eq. 20, Heggie Eq. 22, logH and logH-TTL. `select` is the builder under
/// test.
fn check_pairs<R: P>(select: impl Fn(Selection, Choice) -> Option<Pair>) {
    let mut ran = 0;
    for stepper in STEPPERS {
        for choice in Choice::ALL {
            let pair = select(stepper, choice)
                .unwrap_or_else(|| panic!("the builder refused {stepper:?} × {choice:?}"));
            assert_eq!((pair.stepper, pair.regularisation), (stepper, choice));
            let (out, calls) = run_pair::<R>(pair);
            let name = format!("{stepper:?} × {choice:?} ({})", R::NAME);
            assert!(out.substeps > 0, "{name}: no step taken");
            assert_eq!(
                calls, out.substeps,
                "{name}: the cadence after each kept step"
            );
            assert!(
                bits(&out.state)
                    .iter()
                    .all(|&x| f64::from_bits(x).is_finite()),
                "{name}: the state is not finite"
            );
            assert_ne!(
                bits(&out.state),
                bits(&burrau::<R>()),
                "{name}: the state did not move"
            );
            ran += 1;
        }
    }
    println!(
        "{}: regularisation_default_pairs: {ran} pairs ran (criterion 30: 5 steppers × none, Aarseth–Zare, Heggie \
         Eq. 20, Heggie Eq. 22, logH, logH-TTL)",
        R::NAME
    );
    assert_eq!(ran, 30, "every pair runs");
}

#[test]
fn regularisation_default_pairs() {
    check_pairs::<f64>(|s, c| Some(Pair::select(s, c)));
    check_pairs::<f32>(|s, c| Some(Pair::select(s, c)));
}

negative_control!(
    regularisation_default_pairs,
    "a builder that refuses the pair measured not to work, Heggie × KDK",
    expected = "the builder refused",
    check_pairs::<f64>(|s, c| {
        let refused = s == Selection::Production(Occupant::Kdk) && matches!(c, Choice::Heggie(_));
        (!refused).then(|| Pair::select(s, c))
    })
);
