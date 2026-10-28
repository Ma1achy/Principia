//! QA's tests for TASK-M3-04, written from the requirements it closes and integrator_contract Part 1's loop, not from
//! the implementation. The oracle is this file's own transcription of Part 1's pseudocode (`contract_loop`): `macro`
//! from 0 while `macro < horizon_steps and not done`, `N_sub` from the substep law on the quantised `d²` of the state
//! the macro-step starts from (TASK-M3-03's, a Need), an inner `while s < N_sub and not done` of `STEP` at
//! `dt_macro / N_sub` with the cadence after each. Every occupant (Euler, KDK, Yoshida-4, Yoshida-6, RK4), f64 and f32.
//!
//! - REQ-INT-007 (`qa_same_spec_twice`): the same IC and sim key integrated twice, with other ICs integrated between,
//!   give bit-identical `SimState`s. Control: a sim key one ulp of `dt_macro` away is caught as a different render.
//! - REQ-INT-008, REQ-INT-038, REQ-INT-022 (`qa_wrapper_is_contract_loop`): `integrate` equals the contract loop bit
//!   for bit (state, `total_substeps`, `t_end_step` the completed macro-steps, `bounded`, `saturated`), for every
//!   occupant through the one loop, stopped by `done` or not. `t_end_step` is `step_count`, the completed macro-steps
//!   (integrator_contract Part 5, "Exact step-index timing invariant"; payload §2; generation_root's `t_end_step` row):
//!   a macro-step `done` cut short is begun, not completed. Controls: the oracle run with another occupant is caught,
//!   and so is the begun count in place of the completed one on a run stopped mid-macro-step.
//! - REQ-INT-008 (`qa_seam4_trace_shared`): the wrapper's calls into `ADVANCE`, `(t_now, t_target, dt_macro, N_sub)`,
//!   are identical across occupants where the table forces `N_sub`, while their end states differ (only the stepper's
//!   arithmetic changes). Control: a trace from another horizon is caught.
//! - REQ-INT-009, REQ-INT-022, REQ-INT-053 (`qa_cadence_once_per_step`): steps are counted by the stepper itself, not by
//!   the cadence; the cadence's calls equal them and the reported `total_substeps` equals them. Controls: an occupant
//!   that skips the callback once, and one that over-reports its substeps, are both caught.
//! - REQ-INT-038 (`qa_horizon_integer_count`): at `N_sub = 1`, the run takes exactly `horizon_steps` macro-steps for
//!   horizons up to 65535, whatever `dt_macro`. Control: a count off by one is caught.
//! - REQ-INT-020 (`qa_advance_is_nsub_steps`): for KDK, Yoshida-4 and Yoshida-6 (and the other steppers),
//!   `ADVANCE(t, t+1)` at `N_sub` equals `N_sub` `STEP` calls at `dt_macro / N_sub`, bit for bit, at any grid index `t`.
//!   Control: `N_sub − 1` steps is caught.
//! - REQ-INT-053 (`qa_steppers_do_not_own_time`): every stepper, regularisation none, reports `owns_time_mapping` false.
//! - REQ-INT-025, REQ-SCHED-003 (`qa_seam3_charts_same_kernel`, `qa_visit_order_free`): samples of several charts,
//!   decoded at fuzzed `(u, v)`, integrate to the contract loop's bits on their `(m, r, p)` alone; the entry point and the
//!   `STEP` signature, coerced to function pointers, take no chart and no neighbour state; a batch of ICs integrated
//!   forwards and backwards gives each IC the same bits. Control: two different ICs are caught as different.
//!
//! **Domain** (pitfalls §10): the wrapper's loop with its cadence hooks still empty (TASK-M3-05, -09, -12 fill them), on
//! short horizons from decoded ICs; it shows the loop's and the seam's behaviour, not the physics of a trajectory.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::marker::PhantomData;

use kernel::chart::burrau::Burrau;
use kernel::chart::invariant::WarpReal;
use kernel::chart::{Chart, ChartDecode, Latent, MassSimplex, ShapeMap, ShapeSphere};
use kernel::decode::{self, Ic};
use kernel::driver::advance::{Advance, AdvanceParams, AdvanceReal, Advanced, Cadence};
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::substep::{d2_quantised, n_sub, NsubThresholds};
use kernel::driver::wrapper::{integrate, integrate_with, WrapperReal, WrapperUniforms};
use kernel::generated::links::{Defaults, LinkReal};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{CapabilityProfile, Geom, State, StepReal, Stepper};
use kernel::payload::{
    sd_saturated, sd_state, tm_t_end_step, SimStateBaseOf, STATE_BOUNDED, STATE_RUNNING,
};
use kernel::stage::decode::sample;
use proptest::prelude::*;
use spirv_std::glam::UVec4;
use validation::{negative_control, GAMMA_K, K_MAX, MASS_SIMPLEX_EPS_M};

// ── Precisions, sim keys, ICs ──────────────────────────────────────────────────────────────────────────────────

trait Q: WrapperReal + LinkReal + WarpReal + From<f32> + Debug {
    fn bits64(self) -> u64;
}

impl Q for f32 {
    fn bits64(self) -> u64 {
        u64::from(self.to_bits())
    }
}

impl Q for f64 {
    fn bits64(self) -> u64 {
        self.to_bits()
    }
}

fn q<R: Q>(x: f32) -> R {
    <R as From<f32>>::from(x)
}

/// The `N_sub` table a sim key carries: the default sim key's, or every threshold one value.
#[derive(Clone, Copy, Debug)]
enum Table {
    Default,
    /// All thresholds +0: no `d²` is below one, so `N_sub = 1`.
    Unit,
    /// All thresholds `f32::MAX`: every finite `d²` is below them, so `N_sub = N_max`, here the given value.
    Forced(u32),
}

fn flat(x: f32) -> NsubThresholds {
    NsubThresholds {
        rows: [UVec4::splat(x.to_bits()); NSUB_TABLE_ROWS],
    }
}

fn key<R: Q>(dt_macro: f32, horizon_steps: u32, table: Table) -> WrapperUniforms<R> {
    let (thresholds, n_max) = match table {
        Table::Default => (
            thresholds(&NsubConfig::DEFAULT).expect("default table"),
            NsubConfig::DEFAULT.n_max,
        ),
        Table::Unit => (flat(0.0), NsubConfig::DEFAULT.n_max),
        Table::Forced(n) => (flat(f32::MAX), n),
    };
    WrapperUniforms {
        dt_macro: q(dt_macro),
        horizon_steps,
        n_max,
        thresholds,
    }
}

fn decoded<R: Q>(z: [f32; 8]) -> Ic<R> {
    decode::decode::<Defaults, R>(&decode::Latent::new(z.map(q::<R>))).ic
}

fn z_strategy() -> impl Strategy<Value = [f32; 8]> {
    proptest::array::uniform8(-1.4f32..1.4)
}

fn key_strategy() -> impl Strategy<Value = (f32, u32, Table)> {
    (
        prop_oneof![Just(1e-3f32), Just(2.5e-3f32), Just(1e-2f32)],
        1u32..=20,
        prop_oneof![
            Just(Table::Default),
            Just(Table::Unit),
            (1u32..=5).prop_map(Table::Forced)
        ],
    )
}

fn sim_bits<R: Q>(s: &SimStateBaseOf<R>) -> Vec<u64> {
    let mut v: Vec<u64> =
        s.r.iter()
            .chain(s.p.iter())
            .flatten()
            .map(|x| x.bits64())
            .collect();
    v.extend([
        u64::from(s.packed_a),
        u64::from(s.times),
        u64::from(s.total_substeps),
    ]);
    v
}

fn state_bits<R: Q>(r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> Vec<u64> {
    r.iter()
        .chain(p.iter())
        .flatten()
        .map(|x| x.bits64())
        .collect()
}

// ── Cadences and instrumented occupants ────────────────────────────────────────────────────────────────────────

/// A cadence that counts its calls and is done after `stop` calls (never, for `None`).
#[derive(Default)]
struct StopAfter {
    calls: u32,
    stop: Option<u32>,
}

impl StopAfter {
    fn new(stop: Option<u32>) -> Self {
        StopAfter { calls: 0, stop }
    }
}

impl<R> Cadence<R> for StopAfter {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {
        self.calls += 1;
    }
    fn done(&self) -> bool {
        self.stop.is_some_and(|k| self.calls >= k)
    }
}

thread_local! {
    /// `STEP` calls made by [`Counted`] steppers on this thread.
    static STEPS: Cell<u32> = const { Cell::new(0) };
    /// The wrapper's calls into [`Spy`]: `(t_now, t_target, dt_macro bits, N_sub)`.
    static TRACE: RefCell<Vec<(u32, u32, u64, u32)>> = const { RefCell::new(Vec::new()) };
}

/// A stepper that counts its own `STEP` calls, independently of any cadence.
struct Counted<S>(PhantomData<S>);

impl<S: Stepper> Stepper for Counted<S> {
    fn profile() -> CapabilityProfile {
        S::profile()
    }
    fn step<R: StepReal>(s: &State<R>, dt: R, geom: &Geom<R>) -> (State<R>, Geom<R>) {
        STEPS.with(|c| c.set(c.get() + 1));
        S::step(s, dt, geom)
    }
}

fn reset_steps() {
    STEPS.with(|c| c.set(0));
}

fn steps_taken() -> u32 {
    STEPS.with(Cell::get)
}

/// The bits of an f32 or f64 the occupant seam's generic float holds.
fn any_bits<R: 'static>(x: &R) -> u64 {
    let x: &dyn Any = x;
    match (x.downcast_ref::<f64>(), x.downcast_ref::<f32>()) {
        (Some(d), _) => d.to_bits(),
        (_, Some(f)) => u64::from(f.to_bits()),
        _ => panic!("a float type other than f32 or f64"),
    }
}

/// An occupant that records each `ADVANCE` call the wrapper makes, then advances as `S`.
struct Spy<S>(PhantomData<S>);

impl<S: Stepper> Advance for Spy<S> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        let dt_bits = any_bits(&params.dt_macro);
        TRACE.with_borrow_mut(|t| t.push((t_now, t_target, dt_bits, params.n_sub)));
        <S as Advance>::advance(state, geom, t_now, t_target, params, cadence)
    }
}

/// A defective occupant (a control's): steps `N_sub` times per macro-step but skips the callback after its last step.
#[cfg(feature = "controls")]
struct Skipper<S>(PhantomData<S>);

#[cfg(feature = "controls")]
impl<S: Stepper> Advance for Skipper<S> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        let n = (t_target - t_now) * params.n_sub;
        let dt = params.dt_macro / R::count(params.n_sub);
        let (mut s, mut g) = (*state, *geom);
        let mut k = 0;
        while k < n {
            let (ns, ng) = Counted::<S>::step(&s, dt, &g);
            s = ns;
            g = ng;
            if k + 1 < n {
                cadence.after_step(&mut s, &mut g);
            }
            k += 1;
        }
        Advanced {
            state: s,
            geom: g,
            substeps: k,
        }
    }
}

/// A defective occupant (a control's): advances as `S` but reports one substep more than it took.
#[cfg(feature = "controls")]
struct Overreport<S>(PhantomData<S>);

#[cfg(feature = "controls")]
impl<S: Stepper> Advance for Overreport<S> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        let mut out =
            <Counted<S> as Advance>::advance(state, geom, t_now, t_target, params, cadence);
        out.substeps += 1;
        out
    }
}

/// Runs `$f::<Stepper, $r>(name, args…)` for each occupant.
macro_rules! each_occupant {
    ($f:ident, $r:ty $(, $a:expr)*) => {{
        $f::<Euler, $r>("Euler" $(, $a)*);
        $f::<Kdk, $r>("KDK" $(, $a)*);
        $f::<Yoshida4, $r>("Yoshida-4" $(, $a)*);
        $f::<Yoshida6, $r>("Yoshida-6" $(, $a)*);
        $f::<Rk4, $r>("RK4" $(, $a)*);
    }};
}

// ── The oracle: integrator_contract Part 1's loop, transcribed ─────────────────────────────────────────────────

struct Oracle<R> {
    state: State<R>,
    steps: u32,
    /// `step_count`: the macro-steps whose `N_sub` substeps were all taken (Part 5; payload §2). The loop's counter
    /// `macro` bounds the loop only: it also counts a macro-step `done` cut short.
    completed: u32,
    saturated: bool,
    done: bool,
}

fn contract_loop<S: Stepper, R: Q, C: Cadence<R>>(
    ic: &Ic<R>,
    u: &WrapperUniforms<R>,
    cadence: &mut C,
) -> Oracle<R> {
    let mut state = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let mut geom = Geom::new(&state.m, &state.r);
    let (mut macros, mut completed, mut steps, mut saturated) = (0u32, 0u32, 0u32, false);
    while macros < u.horizon_steps && !cadence.done() {
        let n = n_sub(d2_quantised(&state.r), &u.thresholds, u.n_max);
        saturated |= n == u.n_max;
        let dt = u.dt_macro / q::<R>(n as f32);
        let mut s = 0;
        while s < n && !cadence.done() {
            let (ns, ng) = S::step(&state, dt, &geom);
            state = ns;
            geom = ng;
            cadence.after_step(&mut state, &mut geom);
            s += 1;
            steps += 1;
        }
        completed += u32::from(s == n);
        macros += 1;
    }
    Oracle {
        state,
        steps,
        completed,
        saturated,
        done: cadence.done(),
    }
}

fn check_against_loop<R: Q>(who: &str, got: &SimStateBaseOf<R>, want: &Oracle<R>) {
    assert_eq!(
        state_bits(&got.r, &got.p),
        state_bits(&want.state.r, &want.state.p),
        "{who}: the wrapper's state differs from the contract loop's"
    );
    assert_eq!(
        got.total_substeps, want.steps,
        "{who}: total_substeps differs from the contract loop's step count"
    );
    assert_eq!(
        tm_t_end_step(got.times),
        want.completed,
        "{who}: t_end_step differs from the contract loop's completed macro-steps"
    );
    let label = if want.done {
        STATE_RUNNING
    } else {
        STATE_BOUNDED
    };
    assert_eq!(
        sd_state(got.packed_a),
        label,
        "{who}: the run's state label differs from the contract loop's"
    );
    assert_eq!(
        sd_saturated(got.packed_a),
        want.saturated,
        "{who}: saturated differs from the contract loop's"
    );
}

// ── REQ-INT-007: the same spec twice ───────────────────────────────────────────────────────────────────────────

fn check_same_render<R: Q>(who: &str, a: &SimStateBaseOf<R>, b: &SimStateBaseOf<R>) {
    assert_eq!(
        sim_bits(a),
        sim_bits(b),
        "{who}: the re-render differs from the first render"
    );
}

fn twice<S: Stepper, R: Q>(name: &str, z: [f32; 8], other: [f32; 8], k: (f32, u32, Table)) {
    let u = key::<R>(k.0, k.1, k.2);
    let ic = decoded::<R>(z);
    let first = integrate::<S, R>(&ic, &u);
    let _ = integrate::<S, R>(&decoded::<R>(other), &u);
    let second = integrate::<S, R>(&ic, &u);
    check_same_render(name, &first, &second);
}

#[test]
fn qa_same_spec_twice() {
    validation::prop::run(
        &(z_strategy(), z_strategy(), key_strategy()),
        |(z, other, k)| {
            each_occupant!(twice, f64, z, other, k);
            each_occupant!(twice, f32, z, other, k);
            Ok(())
        },
    );
}

negative_control!(
    qa_same_spec_twice,
    "a sim key one ulp of dt_macro away renders differently",
    expected = "the re-render differs",
    {
        let ic = decoded::<f64>([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1]);
        let u = key::<f64>(1e-2, 8, Table::Unit);
        let mut v = u;
        v.dt_macro = f64::from_bits(u.dt_macro.to_bits() + 1);
        check_same_render(
            "control",
            &integrate::<Kdk, f64>(&ic, &u),
            &integrate::<Kdk, f64>(&ic, &v),
        );
    }
);

// ── REQ-INT-008 / 022 / 038: the wrapper is the contract loop ──────────────────────────────────────────────────

fn against_loop<S: Stepper, R: Q>(
    name: &str,
    z: [f32; 8],
    k: (f32, u32, Table),
    stop: Option<u32>,
) {
    let u = key::<R>(k.0, k.1, k.2);
    let ic = decoded::<R>(z);
    let got = integrate_with::<S, R, _>(&ic, &u, &mut StopAfter::new(stop));
    let want = contract_loop::<S, R, _>(&ic, &u, &mut StopAfter::new(stop));
    check_against_loop(name, &got, &want);
    if stop.is_none() {
        // The wrapper's own cadence, hooks empty, is never done: the same as no stop.
        check_against_loop(name, &integrate::<S, R>(&ic, &u), &want);
    }
}

#[test]
fn qa_wrapper_is_contract_loop() {
    let stop = prop_oneof![Just(None), (1u32..=40).prop_map(Some)];
    validation::prop::run(&(z_strategy(), key_strategy(), stop), |(z, k, stop)| {
        each_occupant!(against_loop, f64, z, k, stop);
        each_occupant!(against_loop, f32, z, k, stop);
        Ok(())
    });
}

negative_control!(
    qa_wrapper_is_contract_loop,
    "the wrapper under Yoshida-4 against the loop under KDK",
    expected = "the wrapper's state differs from the contract loop's",
    {
        let ic = decoded::<f64>([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1]);
        let u = key::<f64>(1e-2, 6, Table::Default);
        let got = integrate::<Yoshida4, f64>(&ic, &u);
        let want = contract_loop::<Kdk, f64, _>(&ic, &u, &mut StopAfter::new(None));
        check_against_loop("control", &got, &want);
    }
);

mod begun_not_completed {
    use super::*;

    negative_control!(
        qa_wrapper_is_contract_loop,
        "a run stopped at the 4th of 3-substep macro-steps, against the begun count (2) for t_end_step",
        expected = "t_end_step differs from the contract loop's completed macro-steps",
        {
            let ic = decoded::<f64>([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1]);
            let u = key::<f64>(1e-2, 6, Table::Forced(3));
            let got = integrate_with::<Kdk, f64, _>(&ic, &u, &mut StopAfter::new(Some(4)));
            let mut want = contract_loop::<Kdk, f64, _>(&ic, &u, &mut StopAfter::new(Some(4)));
            // 4 substeps of 3-substep macro-steps: one completed, a second begun and cut short.
            assert_eq!(want.completed, 1, "control: the stop is not mid-macro-step");
            want.completed = 2;
            check_against_loop("control", &got, &want);
        }
    );
}

mod horizon_one_longer {
    use super::*;

    negative_control!(
        qa_wrapper_is_contract_loop,
        "the wrapper against the loop at a horizon one longer",
        expected = "differs from the contract loop",
        {
            let ic = decoded::<f64>([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1]);
            let u = key::<f64>(1e-2, 6, Table::Unit);
            let mut longer = u;
            longer.horizon_steps += 1;
            let got = integrate::<Kdk, f64>(&ic, &u);
            let want = contract_loop::<Kdk, f64, _>(&ic, &longer, &mut StopAfter::new(None));
            check_against_loop("control", &got, &want);
        }
    );
}

// ── REQ-INT-008: the seam-4 trace is shared ────────────────────────────────────────────────────────────────────

type Trace = Vec<(u32, u32, u64, u32)>;

fn trace_of<S: Stepper, R: Q>(ic: &Ic<R>, u: &WrapperUniforms<R>) -> (Trace, Vec<u64>) {
    TRACE.with_borrow_mut(Vec::clear);
    let out = integrate::<Spy<S>, R>(ic, u);
    (
        TRACE.with_borrow_mut(std::mem::take),
        state_bits(&out.r, &out.p),
    )
}

fn check_trace(who: &str, trace: &Trace, horizon: u32, dt_bits: u64, n: u32) {
    let want: Trace = (0..horizon).map(|t| (t, t + 1, dt_bits, n)).collect();
    assert_eq!(
        trace, &want,
        "{who}: the wrapper's ADVANCE calls are not the shared trace"
    );
}

fn seam4<R: Q>(z: [f32; 8], dt: f32, horizon: u32, n: u32) {
    let u = key::<R>(dt, horizon, Table::Forced(n));
    let ic = decoded::<R>(z);
    let dt_bits = u.dt_macro.bits64();
    let runs = [
        ("Euler", trace_of::<Euler, R>(&ic, &u)),
        ("KDK", trace_of::<Kdk, R>(&ic, &u)),
        ("Yoshida-4", trace_of::<Yoshida4, R>(&ic, &u)),
        ("Yoshida-6", trace_of::<Yoshida6, R>(&ic, &u)),
        ("RK4", trace_of::<Rk4, R>(&ic, &u)),
    ];
    for (name, (trace, _)) in &runs {
        check_trace(name, trace, horizon, dt_bits, n);
    }
    // Only the arithmetic changes: the occupants' end states are not all one.
    let first = &runs[0].1 .1;
    assert!(
        runs.iter().any(|(_, (_, s))| s != first),
        "every occupant reached the same bits: the occupant is not the swapped piece"
    );
}

#[test]
fn qa_seam4_trace_shared() {
    validation::prop::run(
        &(
            z_strategy(),
            prop_oneof![Just(1e-3f32), Just(1e-2f32)],
            1u32..=12,
            1u32..=6,
        ),
        |(z, dt, h, n)| {
            seam4::<f64>(z, dt, h, n);
            seam4::<f32>(z, dt, h, n);
            Ok(())
        },
    );
}

negative_control!(
    qa_seam4_trace_shared,
    "a trace taken at a horizon of 5 checked against a horizon of 6",
    expected = "are not the shared trace",
    {
        let u = key::<f64>(1e-2, 5, Table::Forced(3));
        let ic = decoded::<f64>([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1]);
        let (trace, _) = trace_of::<Kdk, f64>(&ic, &u);
        check_trace("control", &trace, 6, u.dt_macro.to_bits(), 3);
    }
);

// ── REQ-INT-009 / 022 / 053: one callback per internal step, honestly reported ─────────────────────────────────

fn check_cadence(who: &str, taken: u32, calls: u32, reported: u32) {
    assert_eq!(
        calls, taken,
        "{who}: callback invocations ({calls}) differ from the internal steps taken ({taken})"
    );
    assert_eq!(
        reported, taken,
        "{who}: reported total_substeps ({reported}) differs from the internal steps taken ({taken})"
    );
}

fn run_counted<A: Advance, R: Q>(who: &str, ic: &Ic<R>, u: &WrapperUniforms<R>, stop: Option<u32>) {
    reset_steps();
    let mut c = StopAfter::new(stop);
    let out = integrate_with::<A, R, _>(ic, u, &mut c);
    check_cadence(who, steps_taken(), c.calls, out.total_substeps);
}

fn per_step<S: Stepper, R: Q>(name: &str, z: [f32; 8], k: (f32, u32, Table), stop: Option<u32>) {
    let u = key::<R>(k.0, k.1, k.2);
    let ic = decoded::<R>(z);
    run_counted::<Counted<S>, R>(name, &ic, &u, stop);
    assert!(steps_taken() > 0, "{name}: no step was taken");
}

#[test]
fn qa_cadence_once_per_step() {
    let stop = prop_oneof![Just(None), (1u32..=40).prop_map(Some)];
    validation::prop::run(&(z_strategy(), key_strategy(), stop), |(z, k, stop)| {
        each_occupant!(per_step, f64, z, k, stop);
        each_occupant!(per_step, f32, z, k, stop);
        Ok(())
    });
}

negative_control!(
    qa_cadence_once_per_step,
    "an occupant that skips the callback after its last substep",
    expected = "callback invocations",
    {
        let ic = decoded::<f64>([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1]);
        run_counted::<Skipper<Kdk>, f64>("skipper", &ic, &key(1e-2, 4, Table::Forced(3)), None);
    }
);

mod overreport {
    use super::*;

    negative_control!(
        qa_cadence_once_per_step,
        "an occupant that reports one substep more than it took",
        expected = "reported total_substeps",
        {
            let ic = decoded::<f64>([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1]);
            run_counted::<Overreport<Yoshida6>, f64>(
                "liar",
                &ic,
                &key(1e-2, 4, Table::Default),
                None,
            );
        }
    );
}

// ── REQ-INT-038: the horizon is an integer count ───────────────────────────────────────────────────────────────

fn check_macros(who: &str, horizon: u32, t_end: u32, steps: u32) {
    assert_eq!(t_end, horizon, "{who}: t_end_step is not horizon_steps");
    assert_eq!(
        steps, horizon,
        "{who}: the run did not take horizon_steps macro-steps"
    );
}

fn horizon_run<R: Q>(dt: f32, h: u32) {
    let ic = decoded::<R>([0.1, 0.2, -0.3, 0.0, 0.4, -0.1, 0.2, 0.3]);
    reset_steps();
    let out = integrate::<Counted<Euler>, R>(&ic, &key(dt, h, Table::Unit));
    check_macros(
        &format!("dt_macro {dt}, horizon {h}"),
        h,
        tm_t_end_step(out.times),
        steps_taken(),
    );
    assert_eq!(sd_state(out.packed_a), STATE_BOUNDED);
}

#[test]
fn qa_horizon_integer_count() {
    // dt_macro values with no exact binary form, where a float `t += dt` against `T = h·dt` drifts across the edge.
    for dt in [1e-3f32, 0.1, 0.3, 1.0 / 3.0, 7e-3] {
        for h in [1u32, 2, 3, 7, 10, 99, 1000, 4097] {
            horizon_run::<f32>(dt, h);
            horizon_run::<f64>(dt, h);
        }
    }
    horizon_run::<f32>(1e-3, 65535);
}

negative_control!(
    qa_horizon_integer_count,
    "a run one macro-step short of its horizon",
    expected = "t_end_step is not horizon_steps",
    check_macros("control", 10, 9, 9)
);

// ── REQ-INT-020: ADVANCE is N_sub STEP calls ───────────────────────────────────────────────────────────────────

fn check_advance<R: Q>(who: &str, got: &Advanced<R>, want: &(State<R>, Geom<R>), n: u32) {
    assert_eq!(
        state_bits(&got.state.r, &got.state.p),
        state_bits(&want.0.r, &want.0.p),
        "{who}: ADVANCE differs from N_sub STEP calls"
    );
    assert_eq!(
        got.substeps, n,
        "{who}: ADVANCE reports a step count other than N_sub"
    );
}

fn steps_by_hand<S: Stepper, R: Q>(s0: &State<R>, dt: R, n: u32) -> (State<R>, Geom<R>) {
    let mut s = *s0;
    let mut g = Geom::new(&s.m, &s.r);
    for _ in 0..n {
        (s, g) = S::step(&s, dt, &g);
    }
    (s, g)
}

fn advance_vs_steps<S: Stepper, R: Q>(name: &str, z: [f32; 8], n: u32, t: u32, dt: f32) {
    let ic = decoded::<R>(z);
    let s0 = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let g0 = Geom::new(&s0.m, &s0.r);
    let params = AdvanceParams {
        dt_macro: q::<R>(dt),
        n_sub: n,
    };
    let got = <S as Advance>::advance(&s0, &g0, t, t + 1, &params, &mut StopAfter::new(None));
    let want = steps_by_hand::<S, R>(&s0, q::<R>(dt) / q::<R>(n as f32), n);
    check_advance(name, &got, &want, n);
}

#[test]
fn qa_advance_is_nsub_steps() {
    validation::prop::run(
        &(
            z_strategy(),
            1u32..=64,
            0u32..65535,
            prop_oneof![Just(1e-3f32), Just(1e-2f32)],
        ),
        |(z, n, t, dt)| {
            each_occupant!(advance_vs_steps, f64, z, n, t, dt);
            each_occupant!(advance_vs_steps, f32, z, n, t, dt);
            Ok(())
        },
    );
}

negative_control!(
    qa_advance_is_nsub_steps,
    "ADVANCE at N_sub = 4 against three STEP calls",
    expected = "ADVANCE differs from N_sub STEP calls",
    {
        let ic = decoded::<f64>([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1]);
        let s0 = State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        };
        let g0 = Geom::new(&s0.m, &s0.r);
        let params = AdvanceParams {
            dt_macro: 1e-2,
            n_sub: 4,
        };
        let got = <Kdk as Advance>::advance(&s0, &g0, 3, 4, &params, &mut StopAfter::new(None));
        check_advance(
            "control",
            &got,
            &steps_by_hand::<Kdk, f64>(&s0, 1e-2 / 4.0, 3),
            4,
        );
    }
);

// ── REQ-INT-053: steppers, regularisation none, do not own time ────────────────────────────────────────────────

fn check_owns(who: &str, p: CapabilityProfile) {
    assert!(
        !p.owns_time_mapping,
        "{who}: a stepper with regularisation none reports owns_time_mapping"
    );
}

#[test]
fn qa_steppers_do_not_own_time() {
    for (name, p) in [
        ("Euler", Euler::profile()),
        ("KDK", Kdk::profile()),
        ("Yoshida-4", Yoshida4::profile()),
        ("Yoshida-6", Yoshida6::profile()),
        ("RK4", Rk4::profile()),
    ] {
        check_owns(name, p);
    }
}

negative_control!(
    qa_steppers_do_not_own_time,
    "a KDK profile claiming owns_time_mapping",
    expected = "reports owns_time_mapping",
    {
        let mut p = Kdk::profile();
        p.owns_time_mapping = true;
        check_owns("control", p);
    }
);

// ── REQ-INT-025 / REQ-SCHED-003: the IC alone drives the kernel ────────────────────────────────────────────────

/// The entry point and the occupant's `STEP` as function pointers: their types name no chart, neighbour or parent
/// state, only the decoded IC and the sim key's uniforms, and `(m, r, p)`, `dt` and the force evaluation.
#[allow(clippy::type_complexity)]
const _SIGNATURES: (
    fn(&Ic<f64>, &WrapperUniforms<f64>) -> SimStateBaseOf<f64>,
    fn(&Ic<f32>, &WrapperUniforms<f32>) -> SimStateBaseOf<f32>,
    fn(&State<f64>, f64, &Geom<f64>) -> (State<f64>, Geom<f64>),
) = (
    integrate::<Kdk, f64>,
    integrate::<Yoshida6, f32>,
    <Kdk as Stepper>::step::<f64>,
);

fn chart_run<C: ChartDecode + Chart<Param = f32>, R: Q>(chart: &C, uv: [f32; 2]) {
    let ic = sample::<C, Defaults, R>(chart, q::<R>(uv[0]), q::<R>(uv[1])).ic;
    let u = key::<R>(4e-3, 5, Table::Default);
    let got = integrate::<Kdk, R>(&ic, &u);
    let want = contract_loop::<Kdk, R, _>(&ic, &u, &mut StopAfter::new(None));
    check_against_loop(chart.name(), &got, &want);
}

fn charts<R: Q>(uv: [f32; 2]) {
    let k_max = K_MAX.number() as f32;
    let gamma_k = GAMMA_K.number() as f32;
    chart_run::<_, R>(
        &Latent {
            z0: [0.2, -0.1, 0.3, 0.1, -0.2, 0.4, 0.1, 0.0],
            q1: [0.4, 0.0, 0.0, 0.1, 0.0, 0.0, 0.2, 0.0],
            q2: [0.0, 0.3, 0.1, 0.0, 0.0, 0.2, 0.0, 0.1],
        },
        uv,
    );
    chart_run::<_, R>(
        &ShapeSphere {
            map: ShapeMap::Spherical,
            n0: [0.0, 0.0, 1.0],
            e1: [1.0, 0.0, 0.0],
            e2: [0.0, 1.0, 0.0],
            scale: 1.0,
            inertia: 1.0,
            phase: 0.0,
            z_mass: [0.1, 0.2],
            z_mom: [0.0, 0.1, -0.1, 0.2],
        },
        uv,
    );
    chart_run::<_, R>(
        &Burrau {
            nu_range: [0.1, 0.9],
            k_max,
            gamma_k,
        },
        uv,
    );
    chart_run::<_, R>(
        &MassSimplex {
            nu0: 0.4,
            eps_m: MASS_SIMPLEX_EPS_M.number() as f32,
        },
        uv,
    );
}

#[test]
fn qa_seam3_charts_same_kernel() {
    validation::prop::run(&(0f32..=1.0, 0f32..=1.0), |(u, v)| {
        charts::<f64>([u, v]);
        charts::<f32>([u, v]);
        Ok(())
    });
}

negative_control!(
    qa_seam3_charts_same_kernel,
    "a sample's run checked against the loop on another IC",
    expected = "differs from the contract loop",
    {
        let u = key::<f64>(4e-3, 5, Table::Default);
        let got = integrate::<Kdk, f64>(&decoded([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1]), &u);
        let want = contract_loop::<Kdk, f64, _>(
            &decoded([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, 0.1]),
            &u,
            &mut StopAfter::new(None),
        );
        check_against_loop("control", &got, &want);
    }
);

fn check_order(who: &str, forward: &[Vec<u64>], backward: &[Vec<u64>]) {
    assert_eq!(
        forward, backward,
        "{who}: an IC's result depends on the order the batch was visited in"
    );
}

fn visit<R: Q>(zs: &[[f32; 8]]) {
    let u = key::<R>(2.5e-3, 6, Table::Default);
    let ics: Vec<Ic<R>> = zs.iter().map(|z| decoded::<R>(*z)).collect();
    let forward: Vec<Vec<u64>> = ics
        .iter()
        .map(|ic| sim_bits(&integrate::<Yoshida4, R>(ic, &u)))
        .collect();
    let mut backward: Vec<Vec<u64>> = ics
        .iter()
        .rev()
        .map(|ic| sim_bits(&integrate::<Yoshida4, R>(ic, &u)))
        .collect();
    backward.reverse();
    check_order("visit order", &forward, &backward);
}

#[test]
fn qa_visit_order_free() {
    validation::prop::run(&proptest::collection::vec(z_strategy(), 2..6), |zs| {
        visit::<f64>(&zs);
        visit::<f32>(&zs);
        Ok(())
    });
}

negative_control!(
    qa_visit_order_free,
    "two different ICs compared as if one",
    expected = "depends on the order",
    {
        let u = key::<f64>(2.5e-3, 6, Table::Default);
        let a = sim_bits(&integrate::<Kdk, f64>(
            &decoded([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1]),
            &u,
        ));
        let b = sim_bits(&integrate::<Kdk, f64>(
            &decoded([0.1, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1]),
            &u,
        ));
        check_order("control", &[a], &[b]);
    }
);
