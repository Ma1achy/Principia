//! logH's occupant seam and its carried state (`kernel::regularisation::logh::LogH`, `kernel::driver::carried`, the
//! wrapper's `integrate_carrying` and `integrate_resumed_carrying`; integrator_contract Part 2a, Part 2b § "The profile
//! gains a field"; dd_simstate_payload §1, "The carried regularised state"). Heggie's are `tests/heggie_seam.rs`; these
//! are the same properties under logH, in each time mode (logH and TTL) and with each stepper logH defines a pair
//! with (KDK, logH's own leapfrog, and RK4), at f64 and f32. Each criterion was written before its first run.
//! - `logh_seam_non_finite_fails`, `logh_seam_exhausted_stops`: through the wrapper (`integrate_with`), a march ended
//!   non-finite fails the run, `sim_failed` at the macro-step it ended in, its state the march's latched one; a spent
//!   budget stops the copy, `running` as it stood, never run on. Their control drops the march's end.
//! - `regularised_state_carried_logh`, `regularised_state_carried_logh_ttl`: marched across `k` `ADVANCE` calls with
//!   the carried state is one direct march registered once at `t = 0`, bitwise, in the Cartesian state and in `B`, `W`
//!   and the clock, with the direct march's step total and 0 re-registrations. Control: a seam that registers again
//!   from `(m, r, p)` at each call, re-freezing `B` and resetting `W`.
//! - `regularised_state_carried_logh_reboot`: through the wrapper, catch-up, the live march (one macro-step per
//!   resumed call) and a re-boot after eviction agree bitwise, with 0 re-registrations. Control: a re-boot that does
//!   not restore the carried state.
//! - `regularised_state_carried_logh_clock`: the interval clock at every call's end is the boundary's `Δ_sync`,
//!   bitwise. Control: a landing with its clamp and retakes off. The test also measures the control the task asked for,
//!   a re-boot that restores `B` and `W` but not the clock: it does not differ from the live march, on the fixture or
//!   with a landing planted off the boundary, because every interval restarts its clock at 0. That is asserted and
//!   printed.
//! - `regularised_state_carried_logh_budget`: the running step total crosses the calls. Control: a seam whose step
//!   total starts afresh at each call.
//! - `regularised_state_carried_logh_layout`: logH's `B`, `W` and clock are the buffer's first three floats, the rest
//!   0. Control: `B` and the clock swapped.
//! - `regularised_re_registrations_visible_logh`: the count a consumer reads from the buffer is 0 for logH through the
//!   wrapper, and 1 for a call handed no carried state after `t = 0`. Control: a seam that registers at every call.
//! - `regularisation_default_logh`: logH is selectable in each time mode with KDK and with RK4, each pair's occupant
//!   type runs a call through the seam with the pair's profile, and the two modes are two marches. Control: the TTL
//!   pair read with the logH mode's profile.

use kernel::decode::Ic;
use kernel::driver::advance::{Advance, AdvanceParams, AdvanceReal, Advanced, Cadence, Reached};
use kernel::driver::carried::Carried;
use kernel::driver::substep::NsubThresholds;
use kernel::driver::wrapper::{
    integrate_carrying, integrate_resumed_carrying, integrate_with, WrapperReal, WrapperUniforms,
};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{CapabilityProfile, Geom, Occupant, Selection, State, Stepper};
use kernel::payload::{
    sd_detail, sd_state, tm_t_end_step, SimStateBaseOf, STATE_BOUNDED, STATE_RUNNING,
    STATE_SIM_FAILED,
};
use kernel::regularisation::az::{AzReal, End, ExplicitTest};
use kernel::regularisation::form::Form;
use kernel::regularisation::logh::{
    self as lh, carried_state, LhAdvanced, LhConfig, LhParams, LhStepper, LogH,
};
use kernel::regularisation::pairs::{Choice, LogHMode, Pair, PairTable, Status};
use kernel::regularisation::Composed;
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

/// The time mode `TTL` names.
const fn mode_of(ttl: bool) -> LogHMode {
    if ttl {
        LogHMode::Ttl
    } else {
        LogHMode::LogH
    }
}

/// Test values for logH's step control, those `tests/heggie_seam.rs` takes for Heggie's: `η = 0.01`, `Δ_sync = 1/8`,
/// the reference's `TINY`, `f = 0.02`, a budget that does not bind, and the landing in force (the clamp, 4 secant
/// retakes, 64 halvings), in `mode`. Test inputs, not calibrations.
fn values<R: AzReal>(mode: LogHMode) -> LhParams<R> {
    LhParams {
        eta: R::at_width(0.01, 0.01),
        delta_sync: R::at_width(0.125, 0.125),
        tiny: R::at_width(1e-37, 1e-300),
        predictive_f: R::at_width(0.02, 0.02),
        step_budget_rate: R::at_width(1e7, 1e7),
        n_min: 1_000_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        mode,
    }
}

/// [`values`] in the logH mode (`TTL` false) or the TTL mode (`TTL` true).
struct Values<const TTL: bool>;

impl<const TTL: bool> LhConfig for Values<TTL> {
    fn params<R: AzReal>() -> LhParams<R> {
        values(mode_of(TTL))
    }
}

/// [`values`] on a budget of 4 steps, which the first interval's march spends.
struct Spent<const TTL: bool>;

impl<const TTL: bool> LhConfig for Spent<TTL> {
    fn params<R: AzReal>() -> LhParams<R> {
        LhParams {
            n_min: 4,
            step_budget_rate: R::default(),
            ..values(mode_of(TTL))
        }
    }
}

/// The step total [`Tight`] allows: more than any one interval of the fixture's march takes (599 at most, measured on
/// the direct march before this test was written) and fewer than its eight take together (1583 at least), both of
/// which [`check_budget`] asserts, so that only a total carried across calls spends it. A fixture input, not a
/// threshold.
const TIGHT_STEPS: u32 = 1000;

/// [`values`] on a budget of [`TIGHT_STEPS`] steps at rate 0.
struct Tight<const TTL: bool>;

impl<const TTL: bool> LhConfig for Tight<TTL> {
    fn params<R: AzReal>() -> LhParams<R> {
        LhParams {
            n_min: TIGHT_STEPS,
            step_budget_rate: R::default(),
            ..values(mode_of(TTL))
        }
    }
}

/// [`values`] with the landing off, its clamp and its retakes: each interval's last step is kept where it ends, past
/// the boundary. Never the occupant's; here it plants a landed clock that is not the boundary's. (With the clamp on
/// and only the retakes off, logH's clamped step ends on the boundary by itself on this fixture, measured, so
/// Heggie's plant, the retakes alone, plants nothing here.)
struct Loose<const TTL: bool>;

impl<const TTL: bool> LhConfig for Loose<TTL> {
    fn params<R: AzReal>() -> LhParams<R> {
        LhParams {
            clamp: false,
            secant_tries: 0,
            bisect_tries: 0,
            ..values(mode_of(TTL))
        }
    }
}

/// logH × `S` with the values of `C`, through the seam.
type Seamed<S, C> = Composed<S, LogH<C>>;

/// A stepper logH defines a pair with, and its name.
trait Defined: Stepper + LhStepper {
    const STEPPER: &'static str;
}

impl Defined for Kdk {
    const STEPPER: &'static str = "KDK";
}

impl Defined for Rk4 {
    const STEPPER: &'static str = "RK4";
}

/// The pair's name at `R`: "logH × KDK (f64)", "logH-TTL × RK4 (f32)".
fn name<S: Defined, C: LhConfig, R: P>() -> String {
    let mode = match C::params::<R>().mode {
        LogHMode::LogH => "logH",
        LogHMode::Ttl => "logH-TTL",
    };
    format!("{mode} × {} ({})", S::STEPPER, R::NAME)
}

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
/// (masses over 12, lengths over 5), as `tests/heggie_seam.rs` takes them.
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

/// A sim key for the wrapper's loop: `dt_macro`, `horizon_steps`, and `N_sub = n_max` throughout, so that logH's step
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

/// The direct march of `s` to boundary `k` by logH × `S` with the values of `C`: registered once at `t = 0`.
fn direct_from<S: Defined, C: LhConfig, R: P>(s: &State<R>, k: u32) -> (LhAdvanced<R>, u32) {
    let mut cadence = Counting::default();
    let out = lh::march::<S, R, Counting, ExplicitTest>(
        s,
        k,
        &C::params(),
        &mut cadence,
        &mut ExplicitTest,
    );
    (out, cadence.calls)
}

/// [`direct_from`] Burrau's state.
fn direct<S: Defined, C: LhConfig, R: P>(k: u32) -> (LhAdvanced<R>, u32) {
    direct_from::<S, C, R>(&burrau(), k)
}

// ── The seam's end, through the wrapper ────────────────────────────────────────────────────────────────────────

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
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
struct Dropping<A>(core::marker::PhantomData<A>);

impl<A: Advance> Advance for Dropping<A> {
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
}

/// How an occupant is read through the seam: as it is, or with its march's end dropped (the control's).
trait Reading {
    type Of<A: Advance>: Advance;
}

/// The occupant as it is.
struct Whole;

impl Reading for Whole {
    type Of<A: Advance> = A;
}

/// The occupant with its march's end dropped ([`Dropping`]).
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
struct Dropped;

impl Reading for Dropped {
    type Of<A: Advance> = Dropping<A>;
}

/// Burrau's state with its second body moved onto its first. In the logH mode `U` is infinite at registration, so
/// `K + B` is and `dt/ds` is 0: the first interval's entry is not positive and the march ends there, not finite, before
/// any step. In the TTL mode `Ω` floors the zero separation, `W` is finite and the entry passes; the first step's
/// force is not finite and the march ends at the finiteness test after it, the state latched at the registration's.
/// (Both measured on the direct march, and asserted below.)
fn coincident<R: P>() -> State<R> {
    let mut s = burrau::<R>();
    s.r[1] = s.r[0];
    s
}

/// [`coincident`] through the wrapper at `dt_macro = 1/16` (`Δ_sync = 1/8`), eight macro-steps, `N_sub = 4`: the first
/// macro-step's target is before the first boundary, so it takes no step and is completed; the second's march ends not
/// finite. The run is `sim_failed`, latched at the macro-step it ended in, which is not completed (`t_end_step = 1`),
/// its state the direct march's latched finite one, its detail 2 (a non-finite derived quantity on a finite state,
/// payload §2), and no step kept.
fn check_non_finite_fails<S: Defined, const TTL: bool, R: P, D: Reading>() {
    let name = name::<S, Values<TTL>, R>();
    let s = coincident::<R>();
    let out = integrate_with::<D::Of<Seamed<S, Values<TTL>>>, R, Counting>(
        &ic_of(&s),
        &key(1.0 / 16.0, 8, 4),
        &mut Counting::default(),
    );
    let reported = reaching::<D::Of<Seamed<S, Values<TTL>>>, R>(&s);
    let (latched, _) = direct_from::<S, Values<TTL>, R>(&s, 1);
    // The precondition, per mode: at the entry under logH, at the first step under TTL.
    let expected_end = if TTL { (0, 1) } else { (1, 0) };
    assert_eq!(
        (
            latched.count.end,
            (latched.count.non_finite_entries, latched.count.steps),
            latched.count.accepted
        ),
        (End::NonFinite, expected_end, 0),
        "{name}: the fixture's direct march ends not finite, at its entry (logH) or its first step (TTL), no step kept"
    );
    println!(
        "{name}: logh_seam non-finite (direct march: {} non-finite entries, {} steps): state {} (criterion \
         {STATE_SIM_FAILED}), t_end_step {} (criterion 1), detail {} (criterion 2), total_substeps {} (criterion 0)",
        latched.count.non_finite_entries,
        latched.count.steps,
        sd_state(out.packed_a),
        tm_t_end_step(out.times),
        sd_detail(out.packed_a),
        out.total_substeps
    );
    assert_eq!(
        sd_state(out.packed_a),
        STATE_SIM_FAILED,
        "{name}: a march ended non-finite fails the run: state {}",
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

/// `$check::<S, TTL, R, $d>()` over logH's two steppers, its two modes and the two precisions.
macro_rules! every_arm {
    ($check:ident, $d:ty) => {
        $check::<Kdk, false, f64, $d>();
        $check::<Kdk, true, f64, $d>();
        $check::<Rk4, false, f64, $d>();
        $check::<Rk4, true, f64, $d>();
        $check::<Kdk, false, f32, $d>();
        $check::<Kdk, true, f32, $d>();
        $check::<Rk4, false, f32, $d>();
        $check::<Rk4, true, f32, $d>();
    };
}

#[test]
fn logh_seam_non_finite_fails() {
    every_arm!(check_non_finite_fails, Whole);
}

negative_control!(
    logh_seam_non_finite_fails,
    "a seam that drops the march's end, the run never failed",
    expected = "a march ended non-finite fails the run",
    check_non_finite_fails::<Kdk, true, f64, Dropped>()
);

/// Burrau's state through the wrapper at `dt_macro = 1/16`, eight macro-steps, on a budget of 4 steps: the first
/// macro-step takes no step and is completed, the second's march spends its budget, and the copy is never run on: the
/// run stops there, `running` as it stood, at `t_end_step = 1`, with the state and steps of the direct march on that
/// budget.
fn check_exhausted_stops<S: Defined, const TTL: bool, R: P, D: Reading>() {
    let name = name::<S, Spent<TTL>, R>();
    let s = burrau::<R>();
    let mut cadence = Counting::default();
    let out = integrate_with::<D::Of<Seamed<S, Spent<TTL>>>, R, Counting>(
        &ic_of(&s),
        &key(1.0 / 16.0, 8, 4),
        &mut cadence,
    );
    let reported = reaching::<D::Of<Seamed<S, Spent<TTL>>>, R>(&s);
    let (once, _) = direct::<S, Spent<TTL>, R>(8);
    assert_eq!(
        (once.count.end, once.boundary),
        (End::Exhausted, 0),
        "{name}: the fixture's direct march spends its budget in its first interval"
    );
    println!(
        "{name}: logh_seam exhausted: state {} (criterion {STATE_RUNNING}), t_end_step {} (criterion 1), \
         total_substeps {} (criterion {}, the direct march's kept steps)",
        sd_state(out.packed_a),
        tm_t_end_step(out.times),
        out.total_substeps,
        once.count.accepted
    );
    assert_eq!(
        sd_state(out.packed_a),
        STATE_RUNNING,
        "{name}: a spent copy stops as it stood: state {}",
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
fn logh_seam_exhausted_stops() {
    every_arm!(check_exhausted_stops, Whole);
}

negative_control!(
    logh_seam_exhausted_stops,
    "a seam that drops the spent budget, the copy run on",
    expected = "a spent copy stops as it stood",
    check_exhausted_stops::<Kdk, true, f64, Dropped>()
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

/// A seam that registers again from `(m, r, p)` at each call, re-freezing `B` and resetting `W` and the clock: the
/// buffer's registration is dropped before the call, its counts kept (a control's).
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
struct Run<R> {
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
fn seamed<R: P>(call: SeamCall<R>, dt_macro: f64, n: u32) -> Run<R> {
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
    Run {
        state,
        carried,
        substeps,
        calls: cadence.calls,
        last,
    }
}

/// logH × `S` to boundary `k` in `n` seam calls at `dt_macro` against one direct march registered once at `t = 0`: the
/// Cartesian state and `B`, `W` and the clock bitwise the direct march's, with its step total, its kept steps and 0
/// re-registrations. The criterion is bitwise equality: 0 components differing.
fn check_carried<S: Defined, const TTL: bool, R: P>(
    call: SeamCall<R>,
    dt_macro: f64,
    n: u32,
    k: u32,
) {
    let name = name::<S, Values<TTL>, R>();
    let (want, want_calls) = direct::<S, Values<TTL>, R>(k);
    assert_eq!(
        (want.count.end, want.boundary),
        (End::Marching, k),
        "the direct march reaches boundary {k}"
    );
    let got = seamed::<R>(call, dt_macro, n);
    let (b_carried, w, t) = carried_state(&got.carried);
    let differing = bits(&got.state)
        .iter()
        .zip(bits(&want.state))
        .filter(|(x, y)| **x != *y)
        .count();
    let own_differing = [b_carried, w, t]
        .iter()
        .zip([want.system.b, want.p.w, want.q.t])
        .filter(|(x, y)| b(**x) != b(*y))
        .count();
    println!(
        "{name}: regularised_state_carried, {n} calls at dt_macro {dt_macro} to boundary {k}: Cartesian components \
         differing from the direct march {differing} of 12 (criterion 0), logH's own (B, W, clock) differing \
         {own_differing} of 3 (criterion 0), re-registrations {} (criterion 0), steps {} (direct march {}), kept steps \
         {} (direct march {})",
        got.carried.re_registrations,
        got.carried.steps,
        want.count.steps,
        got.substeps,
        want.count.accepted
    );
    assert_eq!(
        differing, 0,
        "{name}: {n} calls with the carried state are not one direct march registered once at t = 0"
    );
    assert_eq!(
        own_differing, 0,
        "{name}: the carried B, W and clock are not the direct march's"
    );
    assert!(
        got.carried.x[3..].iter().all(|&x| b(x) == 0),
        "the buffer's other floats are 0"
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

/// The arrangements, Heggie's: sixteen calls of half a sync interval each, every second one marching; four calls of
/// two intervals each; three calls of 3/8, which reach boundaries 3, 6 and 9; and twelve calls of 0.1, a macro-step
/// that is no multiple or fraction of the sync quantum, which reach `t = 1.2` and so boundary 9.
fn check_carried_all<S: Defined, const TTL: bool, R: P>(call: SeamCall<R>) {
    check_carried::<S, TTL, R>(call, 1.0 / 16.0, 16, 8);
    check_carried::<S, TTL, R>(call, 1.0 / 4.0, 4, 8);
    check_carried::<S, TTL, R>(call, 3.0 / 8.0, 3, 9);
    check_carried::<S, TTL, R>(call, 0.1, 12, 9);
}

/// [`check_carried_all`] in one time mode, with each stepper at each precision.
fn check_carried_mode<const TTL: bool>() {
    check_carried_all::<Kdk, TTL, f64>(seam::<Seamed<Kdk, Values<TTL>>, f64>);
    check_carried_all::<Rk4, TTL, f64>(seam::<Seamed<Rk4, Values<TTL>>, f64>);
    check_carried_all::<Kdk, TTL, f32>(seam::<Seamed<Kdk, Values<TTL>>, f32>);
    check_carried_all::<Rk4, TTL, f32>(seam::<Seamed<Rk4, Values<TTL>>, f32>);
}

#[test]
fn regularised_state_carried_logh() {
    check_carried_mode::<false>();
}

negative_control!(
    regularised_state_carried_logh,
    "a seam that registers again from (m, r, p) at each call, re-freezing B",
    expected = "are not one direct march registered once at t = 0",
    check_carried_all::<Kdk, false, f64>(rebuilding::<Seamed<Kdk, Values<false>>, f64>)
);

#[test]
fn regularised_state_carried_logh_ttl() {
    check_carried_mode::<true>();
}

negative_control!(
    regularised_state_carried_logh_ttl,
    "a seam that registers again from (m, r, p) at each call, resetting W to Ω(r)",
    expected = "are not one direct march registered once at t = 0",
    check_carried_all::<Kdk, true, f64>(rebuilding::<Seamed<Kdk, Values<true>>, f64>)
);

// ── regularised_state_carried_logh_budget ──────────────────────────────────────────────────────────────────────

/// On a budget the direct march spends after its first landing and before its last, the seam's march, one sync
/// interval per call, spends it at the same step: the same call, the state bitwise, the same step total.
fn check_budget<S: Defined, const TTL: bool, R: P>(call: SeamCall<R>) {
    let name = name::<S, Tight<TTL>, R>();
    let (want, _) = direct::<S, Tight<TTL>, R>(8);
    let (free, _) = direct::<S, Values<TTL>, R>(8);
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
        "{name}: regularised_state_carried_budget: direct march spent at boundary {} after {} steps; the seam's march \
         ended {:?} in call {} after {} steps (criterion: Exhausted, call {}, {} steps); largest interval {} steps \
         against a budget of {TIGHT_STEPS}",
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
        "{name}: the running step total is carried: the seam's march spends its budget where the direct march does"
    );
    assert_eq!(got.carried.steps, want.count.steps, "at the same step");
    assert_eq!(
        bits(&got.state),
        bits(&want.state),
        "the state latched, bitwise"
    );
}

/// [`check_budget`] in one time mode, with each stepper at each precision.
fn check_budget_mode<const TTL: bool>() {
    check_budget::<Kdk, TTL, f64>(seam::<Seamed<Kdk, Tight<TTL>>, f64>);
    check_budget::<Rk4, TTL, f64>(seam::<Seamed<Rk4, Tight<TTL>>, f64>);
    check_budget::<Kdk, TTL, f32>(seam::<Seamed<Kdk, Tight<TTL>>, f32>);
    check_budget::<Rk4, TTL, f32>(seam::<Seamed<Rk4, Tight<TTL>>, f32>);
}

#[test]
fn regularised_state_carried_logh_budget() {
    check_budget_mode::<false>();
    check_budget_mode::<true>();
}

negative_control!(
    regularised_state_carried_logh_budget,
    "a seam whose step total starts afresh at each call",
    expected = "the running step total is carried",
    check_budget::<Kdk, true, f64>(forgetting::<Seamed<Kdk, Tight<true>>, f64>)
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
    /// `B`, `W` and the counts, but not the interval clock, which starts at 0.
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
            c.x[2] = R::default();
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

/// logH × `S` through the wrapper: catch-up is the direct march registered once at `t = 0`; the live march, one
/// macro-step per resumed call, is the catch-up; and a re-boot after eviction at [`EVICTED_AT`], resumed to the
/// horizon, is the live march; each bitwise, over every stored value, with 0 re-registrations.
fn check_reboot<S: Defined, const TTL: bool, R: P>(restore: Restore) {
    let name = name::<S, Values<TTL>, R>();
    let (want, _) = direct::<S, Values<TTL>, R>(8);
    let caught = catch_up::<Seamed<S, Values<TTL>>, R>();
    let series = live::<Seamed<S, Values<TTL>>, R>();
    let last = series.last().expect("the horizon");
    let evicted = &series[EVICTED_AT as usize];
    let back =
        resumed::<Seamed<S, Values<TTL>>, R>(&rebooted(evicted, restore), HORIZON - EVICTED_AT);
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
        "{name}: regularised_state_carried_reboot ({restore:?}): stored values differing, live against catch-up {} of \
         {total} (criterion 0), re-boot against live {} of {total} (criterion 0); re-registrations catch-up {}, live \
         {}, re-boot {} (criterion 0 each); catch-up state {} t_end_step {} (criterion {STATE_BOUNDED}, {HORIZON})",
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
        "{name}: catch-up through the wrapper is the direct march registered once at t = 0, bitwise"
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
        "{name}: the live march is the catch-up, bitwise"
    );
    assert!(
        evicted.carried.registered == 1 && evicted.carried.steps > 0,
        "the evicted trajectory has marched"
    );
    assert_eq!(
        stored_bits(&back),
        stored_bits(last),
        "{name}: a re-boot after eviction is the live march, bitwise"
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

/// [`check_reboot`] with the buffer restored whole, over every arm.
fn check_reboot_whole<S: Defined, const TTL: bool, R: P, D>() {
    check_reboot::<S, TTL, R>(Restore::Whole);
}

#[test]
fn regularised_state_carried_logh_reboot() {
    every_arm!(check_reboot_whole, ());
}

negative_control!(
    regularised_state_carried_logh_reboot,
    "a re-boot that does not restore the carried state, which re-freezes B and resets W from the stored (r, p)",
    expected = "a re-boot after eviction is the live march, bitwise",
    {
        // In either mode: under logH the re-frozen B differs, under TTL the reset W does.
        let ttl = std::panic::catch_unwind(|| check_reboot::<Kdk, true, f64>(Restore::Nothing));
        assert!(ttl.is_err(), "the TTL arm's re-boot with nothing restored passed");
        check_reboot::<Kdk, false, f64>(Restore::Nothing)
    }
);

// ── regularised_state_carried_logh_clock ───────────────────────────────────────────────────────────────────────

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
    /// The stored values by which a re-boot there with `B` and `W` restored but not the clock differs from the live
    /// march at the horizon.
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
            if b(pair[1].carried.x[2]) != DELTA_SYNC.to_bits() {
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
        evicted_clock: evicted.carried.x[2].into(),
        clockless_differing: stored_bits(&back)
            .iter()
            .zip(stored_bits(last))
            .filter(|(p, q)| **p != *q)
            .count(),
    }
}

/// The interval clock across calls. `own` is the occupant's live march, `planted` one whose landing is off, so that
/// each interval's last step is kept past the boundary.
///
/// The property guarded: the occupant's landed clock is the boundary's at every call's end, bitwise (Part 2b, logH's
/// step 8).
///
/// The measurement beside it, of the control the task asked for (a re-boot that restores the occupant's state but not
/// its clock must fail, once a landing whose clock differs from the boundary is shown): on the occupant's fixture no
/// landing has such a clock, so one is planted, and with it shown, the clock-less re-boot still does not differ from
/// the live march: every interval restarts its clock at 0 before it reads it (`logh::march_on`). So that control
/// cannot fail and is void, as it is under Heggie; the test asserts what was measured instead of passing over it.
fn check_clock(name: &str, own: Clocks, planted: Clocks) {
    println!(
        "{name}: regularised_state_carried_clock: the occupant's landings {} with a clock off the boundary {} \
         (criterion 0); evicted after macro-step {} with clock {:e}, the re-boot without the clock differs from live in \
         {} stored values. Planted (the landing off): landings {} with a clock off the boundary {} (precondition: at \
         least 1); evicted after macro-step {} with clock {:e} (Δ_sync + {:e}), the re-boot without the clock differs \
         from live in {} stored values (criterion 0: the control \"restores B and W but not the clock\" is void, every \
         interval restarts its clock at 0)",
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
        own.off, 0,
        "{name}: the landed clock is the boundary's at every call's end"
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

/// [`check_clock`] on one arm.
fn check_clock_arm<S: Defined, const TTL: bool, R: P, D>() {
    check_clock(
        &name::<S, Values<TTL>, R>(),
        clocks::<Seamed<S, Values<TTL>>, R>(),
        clocks::<Seamed<S, Loose<TTL>>, R>(),
    );
}

#[test]
fn regularised_state_carried_logh_clock() {
    every_arm!(check_clock_arm, ());
}

negative_control!(
    regularised_state_carried_logh_clock,
    "a landing with its clamp and retakes off, which keeps a clock past the boundary",
    expected = "the landed clock is the boundary's at every call's end",
    check_clock(
        "logH-TTL × KDK (f64), the landing off",
        clocks::<Seamed<Kdk, Loose<true>>, f64>(),
        clocks::<Seamed<Kdk, Loose<true>>, f64>()
    )
);

// ── regularised_state_carried_logh_layout ──────────────────────────────────────────────────────────────────────

/// logH's write into the buffer, as the seam makes it.
type Pack = fn(&mut Carried<f64>, f64, f64, f64, u32);

/// logH's numbers in the buffer's element (payload §1): `B`, `W` and the clock `t` its first three floats, in that
/// order, the other eleven 0; the step total and the registration written, the count left; read back as written.
/// The element's widths and offsets are `regularised_state_carried_layout`'s (`tests/heggie_seam.rs`).
fn check_layout(pack: Pack) {
    let mut carried = Carried::unregistered();
    carried.x = [9.0; 14];
    carried.re_registrations = 5;
    pack(&mut carried, 1.0, 2.0, 3.0, 99);
    println!(
        "regularised_state_carried_logh_layout: the floats {:?} (criterion [1, 2, 3, then eleven 0])",
        carried.x
    );
    assert_eq!(
        carried.x,
        [1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        "logH's numbers in the buffer's order: B, then W, then the clock, the rest 0"
    );
    assert_eq!(
        (carried.steps, carried.registered, carried.re_registrations),
        (99, 1, 5),
        "the step total and the registration are written, the count left"
    );
    assert_eq!(
        carried_state(&carried),
        (1.0, 2.0, 3.0),
        "read back as written"
    );
}

#[test]
fn regularised_state_carried_logh_layout() {
    check_layout(lh::carry::<f64>);
}

negative_control!(
    regularised_state_carried_logh_layout,
    "a write with B and the clock swapped",
    expected = "logH's numbers in the buffer's order",
    check_layout(|carried, b, w, t, steps| lh::carry(carried, t, w, b, steps))
);

// ── regularised_re_registrations_visible_logh ──────────────────────────────────────────────────────────────────

/// The occupant `A` through a seam that drops the buffer's registration before every call, so that every call
/// registers (a control's).
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
struct Registering<A>(core::marker::PhantomData<A>);

impl<A: Advance> Advance for Registering<A> {
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
        carried.registered = 0;
        A::advance_carrying(state, geom, t_now, t_target, params, cadence, carried)
    }
}

/// How the visible count's run reads the occupant: as it is, or registering at every call (the control's).
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
struct Again;

impl Reading for Again {
    type Of<A: Advance> = Registering<A>;
}

/// The count a consumer reads beside the `SimState` after a run to `t = 1`, eight sync intervals, under logH × `S`:
/// 0, logH registering once; and 1 for a call handed an unregistered buffer at a boundary after 0.
fn check_visible<S: Defined, const TTL: bool, R: P, D: Reading>() {
    let name = name::<S, Values<TTL>, R>();
    let mut carried = Carried::unregistered();
    let out = integrate_carrying::<D::Of<Seamed<S, Values<TTL>>>, R, Counting>(
        &ic_of(&burrau()),
        &key(DT_MACRO, HORIZON, 4),
        &mut Counting::default(),
        &mut carried,
    );
    assert_eq!(
        tm_t_end_step(out.times),
        HORIZON,
        "the run reaches the horizon"
    );
    let mut late = Carried::unregistered();
    seam::<Seamed<S, Values<TTL>>, R>(
        &burrau(),
        4,
        &AdvanceParams {
            dt_macro: R::of(DT_MACRO),
            n_sub: 4,
        },
        &mut Counting::default(),
        &mut late,
    );
    println!(
        "{name}: regularised_re_registrations_visible: through the wrapper {} (criterion 0), a call with nothing \
         carried at boundary 2: {} (criterion 1)",
        carried.re_registrations, late.re_registrations
    );
    assert_eq!(
        (carried.re_registrations, carried.registered),
        (0, 1),
        "{name}: logH registers once, and its count reads 0"
    );
    assert_eq!(
        late.re_registrations, 1,
        "a registration after t = 0 is a re-registration, and is counted"
    );
}

#[test]
fn regularised_re_registrations_visible_logh() {
    every_arm!(check_visible, Whole);
}

negative_control!(
    regularised_re_registrations_visible_logh,
    "a seam that registers at every call: seven registrations after t = 0 over eight sync intervals",
    expected = "logH registers once, and its count reads 0",
    check_visible::<Kdk, true, f64, Again>()
);

// ── regularisation_default: logH is selectable ─────────────────────────────────────────────────────────────────

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

/// The profiles the composed occupants report, by stepper and mode: `[KDK, RK4]` in the logH mode, then in TTL.
type Profiles = [[CapabilityProfile; 2]; 2];

/// The composed occupants' own profiles.
fn composed_profiles() -> Profiles {
    [
        [
            Seamed::<Kdk, Values<false>>::profile(),
            Seamed::<Rk4, Values<false>>::profile(),
        ],
        [
            Seamed::<Kdk, Values<true>>::profile(),
            Seamed::<Rk4, Values<true>>::profile(),
        ],
    ]
}

/// logH is selectable in each time mode with each stepper it defines a pair with, KDK and RK4: the pair is selected
/// with no refusal, not measured in the table and carrying no warning, and owns its time mapping; its occupant type
/// reports the pair's profile (the leapfrog's: order 2, one force evaluation, step-reversible, symplectic in the logH
/// mode alone and run-reversible in the TTL mode alone; RK4's form's on a regularised system) and runs a call through
/// the seam; and the two modes are two marches.
fn check_selectable(profiles: Profiles) {
    let table = PairTable::part_2b();
    for (mode, row) in [LogHMode::LogH, LogHMode::Ttl].into_iter().zip(profiles) {
        for (stepper, profile) in [Occupant::Kdk, Occupant::Rk4].into_iter().zip(row) {
            let pair = Pair::select(Selection::Production(stepper), Choice::LogH(mode));
            assert_eq!(pair.regularisation, Choice::LogH(mode));
            assert_eq!(pair.status(&table), Status::NotMeasured);
            assert_eq!(pair.warning(&table), None);
            assert!(pair.owns_time_mapping());
            let want = match stepper {
                Occupant::Kdk => CapabilityProfile {
                    order: 2,
                    force_evals: 1,
                    symplectic: mode == LogHMode::LogH,
                    step_reversible: true,
                    run_reversible: mode == LogHMode::Ttl,
                    owns_time_mapping: true,
                },
                _ => CapabilityProfile {
                    owns_time_mapping: true,
                    ..Form::Rk4.pair_profile()
                },
            };
            println!("regularisation_default_logh: {stepper:?} × {mode:?}: {profile:?}");
            assert_eq!(
                profile, want,
                "{stepper:?} × {mode:?}: the occupant's profile is not the pair's in its time mode"
            );
        }
    }
    let runs = [
        ("logH × KDK", one_call::<Seamed<Kdk, Values<false>>, f64>()),
        ("logH × RK4", one_call::<Seamed<Rk4, Values<false>>, f64>()),
        (
            "logH-TTL × KDK",
            one_call::<Seamed<Kdk, Values<true>>, f64>(),
        ),
        (
            "logH-TTL × RK4",
            one_call::<Seamed<Rk4, Values<true>>, f64>(),
        ),
    ];
    for (name, (out, calls)) in &runs {
        assert!(
            out.substeps > 0 && *calls == out.substeps,
            "{name} runs, the cadence after each kept step"
        );
        assert_ne!(
            bits(&out.state),
            bits(&burrau::<f64>()),
            "{name}: the state moved"
        );
    }
    for (i, j) in [(0, 1), (0, 2), (1, 3), (2, 3)] {
        assert_ne!(
            bits(&runs[i].1 .0.state),
            bits(&runs[j].1 .0.state),
            "{} and {} are two marches",
            runs[i].0,
            runs[j].0
        );
    }
}

#[test]
fn regularisation_default_logh() {
    check_selectable(composed_profiles());
}

negative_control!(
    regularisation_default_logh,
    "the TTL pairs read with the logH mode's profiles",
    expected = "the occupant's profile is not the pair's in its time mode",
    {
        let p = composed_profiles();
        check_selectable([p[0], p[0]])
    }
);
