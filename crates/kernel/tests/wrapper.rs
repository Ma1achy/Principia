//! The wrapper, the `ADVANCE` seam and the per-substep callback (`kernel::driver::{wrapper, advance, cadence}`;
//! integrator_contract Part 1, Part 2a, Part 3, Part 5; dd_integrator §4, seams 3 and 4; R-19), over every occupant
//! (Euler, KDK, Yoshida-4, Yoshida-6, RK4) at f64 and f32, on ICs decoded from fuzzed latent controls:
//! - `same_spec_twice`: the same IC and sim key integrated twice give bit-identical `SimState`s (REQ-INT-007).
//! - `seam4_occupant_swap`: the wrapper's trace, each `ADVANCE` call's `(t_now, t_target, N_sub, d²)`, the substeps
//!   reported and the cadence's calls, is the same function of the state for every occupant: each `N_sub` is the law's
//!   on the quantised `d²` of the state the call starts from, the times step 0, 1, … on the integer grid, and where the
//!   table forces `N_sub` the whole trace is identical across occupants, a `done` stop included, while their states
//!   differ (REQ-INT-008).
//! - `cadence_callback_count`: the callback's calls equal the `STEP` calls an occupant made, the run stopped by `done`
//!   or not (REQ-INT-009).
//! - `advance_equals_nsub_steps`: `ADVANCE(t, t + 1)` at `N_sub` is `N_sub` `STEP` calls at `dt_macro / N_sub`, bit for
//!   bit, `t_target = t + 2` is twice that, and a target not after `t` takes no step (REQ-INT-020).
//! - `total_substeps_honest`: the `total_substeps` the run reports is the callback's calls, and differs from `Σ N_sub`
//!   where `done` stopped a macro-step part-way (REQ-INT-022).
//! - `seam3_every_chart`: a sample of every chart of the registry, decoded at fuzzed `(u, v)`, integrates through the
//!   one wrapper to the same bits as its `(m, r, p)` copied out of the decode as bare numbers; the run is labelled
//!   (REQ-INT-025).
//! - `advance_seam_r19`: for every stepper with regularisation `none`, `owns_time_mapping` is false and the callback is
//!   called once per substep (REQ-INT-053's stepper arm).
//! - `wrapper_horizon_counter`: the loop runs `horizon_steps` macro-steps, writes `bounded` and `t_end_step` at the
//!   horizon, `running` when `done` stopped it, and `saturated` where `N_sub` reached `N_max`.
//!
//! **Domain** (pitfalls §10): the wrapper's loop with its cadence hooks empty (TASK-M3-05, -09, -12 fill them), over
//! short horizons; what it shows is the loop's and the seam's, not a trajectory's physics.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::marker::PhantomData;

use kernel::chart::burrau::{Burrau, EuclidPlane};
use kernel::chart::invariant::{Invariant, WarpReal};
use kernel::chart::mixed::{Alpha, ZMu1};
use kernel::chart::{
    BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, ChartDecode, InvariantLE, InvariantLK,
    Latent, MassSimplex, Mixed, ShapeMap, ShapeSphere, REGISTRY,
};
use kernel::decode::{self, Ic};
use kernel::driver::advance::{Advance, AdvanceParams, AdvanceReal, Advanced, Cadence};
use kernel::driver::cadence::{self, WrapperCadence};
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
use kernel::occupant::{CapabilityProfile, Geom, State, Stepper};
use kernel::payload::{
    pa_d_min_is_unset, sd_dmin_pair, sd_saturated, sd_state, tm_t_dmin_step, tm_t_end_step,
    SimStateBaseOf, SD_DMIN_PAIR_SENTINEL, STATE_BOUNDED, STATE_RUNNING,
};
use kernel::stage::decode::sample;
use proptest::prelude::*;
use spirv_std::glam::UVec4;
use validation::{negative_control, GAMMA_K, K_MAX, MASS_SIMPLEX_EPS_M};

// ── Precisions, occupants, sim keys and ICs ─────────────────────────────────────────────────────────────────────

/// A precision the wrapper runs at, with the decoder's bounds, and its bits.
trait P: WrapperReal + LinkReal + WarpReal + From<f32> + Into<f64> + Debug {
    const LABEL: &'static str;
    fn bits(self) -> u64;
    fn from_bits(bits: u64) -> Self;
}

impl P for f32 {
    const LABEL: &'static str = "f32";
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
    fn from_bits(bits: u64) -> Self {
        f32::from_bits(bits as u32)
    }
}

impl P for f64 {
    const LABEL: &'static str = "f64";
    fn bits(self) -> u64 {
        self.to_bits()
    }
    fn from_bits(bits: u64) -> Self {
        f64::from_bits(bits)
    }
}

/// The f32 `x` at `R`, exactly.
fn of<R: P>(x: f32) -> R {
    <R as From<f32>>::from(x)
}

/// Runs `$check::<Occupant, $r>(name, args…)` for every occupant, the debug Euler with the four production steppers.
macro_rules! every_occupant {
    ($check:ident, $r:ty $(, $arg:expr)*) => {{
        $check::<Euler, $r>("Euler" $(, $arg)*);
        $check::<Kdk, $r>("KDK" $(, $arg)*);
        $check::<Yoshida4, $r>("Yoshida-4" $(, $arg)*);
        $check::<Yoshida6, $r>("Yoshida-6" $(, $arg)*);
        $check::<Rk4, $r>("RK4" $(, $arg)*);
    }};
}

/// Every occupant's profile, by name.
fn profiles() -> [(&'static str, CapabilityProfile); 5] {
    [
        ("Euler", Euler::profile()),
        ("KDK", Kdk::profile()),
        ("Yoshida-4", Yoshida4::profile()),
        ("Yoshida-6", Yoshida6::profile()),
        ("RK4", Rk4::profile()),
    ]
}

/// Which `N_sub` table a sim key carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Table {
    /// The default sim key's (`r_sub = 0.05`, `γ_sub = 1.5`, `N_max = 64`).
    Default,
    /// Every threshold 0, so no `d²` is below one: `N_sub = 1` throughout.
    One,
    /// Every threshold the largest finite f32, so every finite `d²` is below them all: `N_sub = N_max` throughout.
    Cap(u32),
}

/// Every threshold `x`.
fn flat(x: f32) -> NsubThresholds {
    NsubThresholds {
        rows: [UVec4::splat(x.to_bits()); NSUB_TABLE_ROWS],
    }
}

/// A sim key: `dt_macro`, `horizon_steps` and the table.
fn uniforms<R: P>(dt_macro: f32, horizon_steps: u32, table: Table) -> WrapperUniforms<R> {
    let (thresholds, n_max) = match table {
        Table::Default => (
            thresholds(&NsubConfig::DEFAULT).expect("the default sim key generates"),
            NsubConfig::DEFAULT.n_max,
        ),
        Table::One => (flat(0.0), NsubConfig::DEFAULT.n_max),
        Table::Cap(n_max) => (flat(f32::MAX), n_max),
    };
    WrapperUniforms {
        dt_macro: of::<R>(dt_macro),
        horizon_steps,
        n_max,
        thresholds,
    }
}

/// The IC the default links decode from the latent controls `z`, at `R`.
fn ic<R: P>(z: [f32; 8]) -> Ic<R> {
    decode::decode::<Defaults, R>(&decode::Latent::new(z.map(of::<R>))).ic
}

/// The latent controls, each on `[−1.5, 1.5]`.
fn any_z() -> impl Strategy<Value = [f32; 8]> {
    proptest::array::uniform8(-1.5f32..1.5)
}

/// A sim key: `dt_macro` 1e-3 or 4e-3, a horizon of 1 to 24 macro-steps, and one of the tables.
fn any_key() -> impl Strategy<Value = (f32, u32, Table)> {
    (
        prop_oneof![Just(1e-3f32), Just(4e-3f32)],
        1u32..=24,
        prop_oneof![
            Just(Table::Default),
            Just(Table::One),
            (1u32..=6).prop_map(Table::Cap),
        ],
    )
}

/// The `SimState`'s bits, field by field.
fn bits<R: P>(s: &SimStateBaseOf<R>) -> Vec<u64> {
    let mut out: Vec<u64> =
        s.r.iter()
            .chain(s.p.iter())
            .flatten()
            .map(|x| x.bits())
            .collect();
    out.extend([s.S, s.theta, s.mean_y, s.C_ty, s.E_0, s.Lz_0, s.closure_min].map(P::bits));
    out.extend([s.packed_a, s.packed_b, s.times, s.total_substeps].map(u64::from));
    out.extend([s.closure_step, s._reserved].map(u64::from));
    out
}

/// A state's and its force evaluation's bits.
fn state_bits<R: P>(s: &State<R>, g: &Geom<R>) -> Vec<u64> {
    let mut out: Vec<u64> = s.m.iter().map(|x| x.bits()).collect();
    out.extend(
        s.r.iter()
            .chain(s.p.iter())
            .chain(g.accel.iter())
            .flatten()
            .map(|x| x.bits()),
    );
    out
}

// ── Test cadences and occupants ─────────────────────────────────────────────────────────────────────────────────

/// A cadence that counts its calls, changes nothing, and is done from its `stop`-th call on, if any.
#[derive(Clone, Copy, Debug, Default)]
struct Probe {
    calls: u32,
    stop: Option<u32>,
}

impl Probe {
    fn stopping_at(stop: Option<u32>) -> Self {
        Probe { calls: 0, stop }
    }
}

impl<R> Cadence<R> for Probe {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {
        self.calls += 1;
    }

    fn done(&self) -> bool {
        self.stop.is_some_and(|k| self.calls >= k)
    }
}

thread_local! {
    /// The `STEP` calls [`Counted`] has made on this thread.
    static STEPS: Cell<u32> = const { Cell::new(0) };
    /// The `ADVANCE` calls [`Traced`] has recorded on this thread.
    static TRACE: RefCell<Vec<Record>> = const { RefCell::new(Vec::new()) };
}

/// The stepper `S`, counting its `STEP` calls into [`STEPS`].
struct Counted<S>(PhantomData<S>);

impl<S: Stepper> Stepper for Counted<S> {
    fn profile() -> CapabilityProfile {
        S::profile()
    }

    fn step<R: kernel::occupant::StepReal>(
        s: &State<R>,
        dt: R,
        geom: &Geom<R>,
    ) -> (State<R>, Geom<R>) {
        STEPS.set(STEPS.get() + 1);
        S::step(s, dt, geom)
    }
}

/// One `ADVANCE` call as the wrapper made it: its times, its `N_sub`, the quantised `d²` of the state it started from,
/// and the substeps the occupant reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Event {
    t_now: u32,
    t_target: u32,
    n_sub: u32,
    d2: u32,
    substeps: u32,
}

/// One `ADVANCE` call as [`Traced`] records it: the event with the `d²` not yet formed, and the positions the call
/// started from, at the run's float type.
struct Record {
    event: Event,
    start: Box<dyn Any>,
}

/// The occupant `A`, recording each `ADVANCE` call into [`TRACE`].
struct Traced<A>(PhantomData<A>);

impl<A: Advance> Advance for Traced<A> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        let out = A::advance(state, geom, t_now, t_target, params, cadence);
        TRACE.with_borrow_mut(|t| {
            t.push(Record {
                event: Event {
                    t_now,
                    t_target,
                    n_sub: params.n_sub,
                    d2: 0,
                    substeps: out.substeps,
                },
                start: Box::new(state.r),
            })
        });
        out
    }
}

/// The trace of one run of `A` at `R`, with `cadence`, each event's `d²` the quantised `d²` of the positions its call
/// started from, and the run's `SimState`.
fn traced<A: Advance, R: P, C: Cadence<R>>(
    ic: &Ic<R>,
    u: &WrapperUniforms<R>,
    cadence: &mut C,
) -> (Vec<Event>, SimStateBaseOf<R>) {
    TRACE.with_borrow_mut(Vec::clear);
    let out = integrate_with::<Traced<A>, R, C>(ic, u, cadence);
    let trace = TRACE.with_borrow_mut(std::mem::take);
    let events = trace
        .into_iter()
        .map(|r| {
            let start = r
                .start
                .downcast_ref::<[[R; 2]; 3]>()
                .expect("the run's float type");
            Event {
                d2: d2_quantised(start).to_bits(),
                ..r.event
            }
        })
        .collect();
    (events, out)
}

/// The occupant `S`, calling the cadence after every other step only: an occupant that omits the call.
#[cfg(feature = "controls")]
struct Skipper<S>(PhantomData<S>);

#[cfg(feature = "controls")]
impl<S: Stepper> Advance for Skipper<S> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        _t_now: u32,
        _t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        let dt = params.dt_macro / R::count(params.n_sub);
        let (mut s, mut g) = (*state, *geom);
        let mut taken = 0;
        while taken < params.n_sub && !cadence.done() {
            STEPS.set(STEPS.get() + 1);
            (s, g) = S::step(&s, dt, &g);
            if taken % 2 == 0 {
                cadence.after_step(&mut s, &mut g);
            }
            taken += 1;
        }
        Advanced {
            state: s,
            geom: g,
            substeps: taken,
        }
    }
}

/// The occupant `S`, reporting the `span · N_sub` steps it was asked for rather than those it took.
#[cfg(feature = "controls")]
struct Liar<S>(PhantomData<S>);

#[cfg(feature = "controls")]
impl<S: Stepper> Advance for Liar<S> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        let out = <S as Advance>::advance(state, geom, t_now, t_target, params, cadence);
        Advanced {
            substeps: (t_target - t_now) * params.n_sub,
            ..out
        }
    }
}

/// The occupant `S`, with an occupant-specific branch on the path: one more substep for a fourth-order occupant.
#[cfg(feature = "controls")]
struct Branchy<S>(PhantomData<S>);

#[cfg(feature = "controls")]
impl<S: Stepper> Advance for Branchy<S> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        let extra = u32::from(S::profile().order == 4);
        let params = AdvanceParams {
            n_sub: params.n_sub + extra,
            ..*params
        };
        <S as Advance>::advance(state, geom, t_now, t_target, &params, cadence)
    }
}

/// The occupant `S`, calling the cadence twice after each step.
#[cfg(feature = "controls")]
struct Twice<S>(PhantomData<S>);

#[cfg(feature = "controls")]
impl<S: Stepper> Advance for Twice<S> {
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        let mut inner = Doubled(cadence);
        <S as Advance>::advance(state, geom, t_now, t_target, params, &mut inner)
    }
}

/// A cadence that calls the cadence it wraps twice per call.
#[cfg(feature = "controls")]
struct Doubled<'a, C>(&'a mut C);

#[cfg(feature = "controls")]
impl<R, C: Cadence<R>> Cadence<R> for Doubled<'_, C> {
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.0.after_step(state, geom);
        self.0.after_step(state, geom);
    }

    fn done(&self) -> bool {
        self.0.done()
    }
}

// ── same_spec_twice (REQ-INT-007) ────────────────────────────────────────────────────────────────────────────────

/// `render`, run twice, gives the same bits.
fn check_twice<R: P>(label: &str, mut render: impl FnMut() -> SimStateBaseOf<R>) {
    let (a, b) = (bits(&render()), bits(&render()));
    assert_eq!(
        a, b,
        "{label}: the same spec rendered twice is not bit-identical"
    );
}

fn twice<A: Advance, R: P>(name: &str, z: [f32; 8], key: (f32, u32, Table)) {
    let ic = ic::<R>(z);
    let u = uniforms::<R>(key.0, key.1, key.2);
    check_twice(&format!("{name} at {}", R::LABEL), || {
        integrate::<A, R>(&ic, &u)
    });
}

#[test]
fn same_spec_twice() {
    validation::prop::run(&(any_z(), any_key()), |(z, key)| {
        every_occupant!(twice, f64, z, key);
        every_occupant!(twice, f32, z, key);
        Ok(())
    });
}

negative_control!(
    same_spec_twice,
    "a render whose horizon follows a counter read afresh on each render, as a wall clock would be",
    expected = "is not bit-identical",
    {
        let run = Cell::new(0u32);
        let ic = ic::<f64>([0.3; 8]);
        check_twice("clocked", || {
            run.set(run.get() + 1);
            integrate::<Kdk, f64>(&ic, &uniforms(1e-3, 4 + run.get(), Table::Default))
        });
    }
);

// ── seam4_occupant_swap (REQ-INT-008) ────────────────────────────────────────────────────────────────────────────

/// One occupant's run in the swap: its name, the wrapper's trace, the callback's calls and the final state's bits.
struct Run {
    name: &'static str,
    trace: Vec<Event>,
    calls: u32,
    state: Vec<u64>,
}

/// The run of `A` on `ic` under `u`, stopped at the `stop`-th callback if any, with each call's law checked: from
/// macro-step `k` to `k + 1`, its `N_sub` the law's on the quantised `d²` of the state it started from.
fn swap_run<A: Advance, R: P>(
    name: &'static str,
    ic: &Ic<R>,
    u: &WrapperUniforms<R>,
    stop: Option<u32>,
) -> Run {
    let mut probe = Probe::stopping_at(stop);
    let (trace, out) = traced::<A, R, Probe>(ic, u, &mut probe);
    for (k, e) in trace.iter().enumerate() {
        let k = k as u32;
        assert_eq!(
            (e.t_now, e.t_target),
            (k, k + 1),
            "{name}: call {k} is not from macro-step {k} to {}",
            k + 1
        );
        assert_eq!(
            e.n_sub,
            n_sub(f32::from_bits(e.d2), &u.thresholds, u.n_max),
            "{name}: call {k}'s N_sub is not the law's on its d²"
        );
    }
    Run {
        name,
        trace,
        calls: probe.calls,
        state: out
            .r
            .iter()
            .chain(out.p.iter())
            .flatten()
            .map(|x| x.bits())
            .collect(),
    }
}

/// A trace's branch decisions: each call's times, its `N_sub` and the substeps taken, without the `d²` its `N_sub`
/// was read from, which is the occupant's arithmetic.
fn decisions(trace: &[Event]) -> Vec<[u32; 4]> {
    trace
        .iter()
        .map(|e| [e.t_now, e.t_target, e.n_sub, e.substeps])
        .collect()
}

/// Every run's branch decisions and calls are the first's, and, where `distinct`, every two runs' final states differ.
fn check_swap(runs: &[Run], distinct: bool) {
    let first = &runs[0];
    for r in &runs[1..] {
        assert_eq!(
            (decisions(&r.trace), r.calls),
            (decisions(&first.trace), first.calls),
            "the wrapper's branch trace differs between {} and {}",
            first.name,
            r.name
        );
    }
    if distinct {
        for (i, a) in runs.iter().enumerate() {
            for b in &runs[i + 1..] {
                assert_ne!(
                    a.state, b.state,
                    "{} and {} reach the same state: the swap changed no arithmetic",
                    a.name, b.name
                );
            }
        }
    }
}

fn swap<R: P>(z: [f32; 8], key: (f32, u32, Table), stop: Option<u32>) {
    let ic = ic::<R>(z);
    let u = uniforms::<R>(key.0, key.1, key.2);
    let runs = vec![
        swap_run::<Euler, R>("Euler", &ic, &u, stop),
        swap_run::<Kdk, R>("KDK", &ic, &u, stop),
        swap_run::<Yoshida4, R>("Yoshida-4", &ic, &u, stop),
        swap_run::<Yoshida6, R>("Yoshida-6", &ic, &u, stop),
        swap_run::<Rk4, R>("RK4", &ic, &u, stop),
    ];
    // Where the table forces `N_sub`, no decision reads the occupant's arithmetic, so the traces must agree; under the
    // default table each occupant's decisions read its own states, and the law was checked call by call above.
    if key.2 != Table::Default {
        check_swap(&runs, R::LABEL == "f64" && stop.is_none());
    }
}

#[test]
fn seam4_occupant_swap() {
    let stop = proptest::option::of(1u32..=60);
    validation::prop::run(&(any_z(), any_key(), stop), |(z, key, stop)| {
        swap::<f64>(z, key, stop);
        swap::<f32>(z, key, stop);
        Ok(())
    });
}

negative_control!(
    seam4_occupant_swap,
    "a branch on the occupant's order in the path the wrapper drives",
    expected = "the wrapper's branch trace differs",
    {
        let ic = ic::<f64>([0.3; 8]);
        let u = uniforms::<f64>(1e-3, 6, Table::Cap(2));
        check_swap(
            &[
                swap_run::<Branchy<Kdk>, f64>("KDK", &ic, &u, None),
                swap_run::<Branchy<Yoshida4>, f64>("Yoshida-4", &ic, &u, None),
            ],
            true,
        );
    }
);

#[test]
fn seam4_occupant_swap_distinct() {
    let ic = ic::<f64>([0.3; 8]);
    let u = uniforms::<f64>(1e-3, 6, Table::Cap(2));
    let runs: Vec<Run> = vec![
        swap_run::<Euler, f64>("Euler", &ic, &u, None),
        swap_run::<Kdk, f64>("KDK", &ic, &u, None),
        swap_run::<Yoshida4, f64>("Yoshida-4", &ic, &u, None),
        swap_run::<Yoshida6, f64>("Yoshida-6", &ic, &u, None),
        swap_run::<Rk4, f64>("RK4", &ic, &u, None),
    ];
    check_swap(&runs, true);
}

negative_control!(
    seam4_occupant_swap_distinct,
    "the same occupant swapped for itself changes no arithmetic",
    expected = "the swap changed no arithmetic",
    {
        let ic = ic::<f64>([0.3; 8]);
        let u = uniforms::<f64>(1e-3, 6, Table::Cap(2));
        check_swap(
            &[
                swap_run::<Kdk, f64>("KDK", &ic, &u, None),
                swap_run::<Kdk, f64>("KDK again", &ic, &u, None),
            ],
            true,
        );
    }
);

// ── cadence_callback_count (REQ-INT-009) ─────────────────────────────────────────────────────────────────────────

fn check_calls(name: &str, calls: u32, steps: u32) {
    assert_eq!(
        calls, steps,
        "{name}: callback invocations {calls} != internal steps taken {steps}"
    );
}

/// The callback's calls against `A`'s `STEP` calls, counted by `A` itself into [`STEPS`], under a [`Probe`] stopping
/// at `stop` and under the wrapper's own cadence.
fn calls_run<A: Advance, R: P>(name: &str, ic: &Ic<R>, u: &WrapperUniforms<R>, stop: Option<u32>) {
    STEPS.set(0);
    let mut probe = Probe::stopping_at(stop);
    integrate_with::<A, R, Probe>(ic, u, &mut probe);
    check_calls(name, probe.calls, STEPS.get());
    STEPS.set(0);
    let mut own = WrapperCadence::default();
    integrate_with::<A, R, WrapperCadence>(ic, u, &mut own);
    check_calls(name, own.invocations(), STEPS.get());
}

fn callbacks<S: Stepper, R: P>(name: &str, z: [f32; 8], key: (f32, u32, Table), stop: Option<u32>) {
    let ic = ic::<R>(z);
    let u = uniforms::<R>(key.0, key.1, key.2);
    calls_run::<Counted<S>, R>(name, &ic, &u, stop);
    assert!(STEPS.get() >= key.1, "{name}: fewer steps than macro-steps");
}

#[test]
fn cadence_callback_count() {
    let stop = proptest::option::of(1u32..=60);
    validation::prop::run(&(any_z(), any_key(), stop), |(z, key, stop)| {
        every_occupant!(callbacks, f64, z, key, stop);
        every_occupant!(callbacks, f32, z, key, stop);
        Ok(())
    });
}

negative_control!(
    cadence_callback_count,
    "an occupant that calls the cadence after every other step only",
    expected = "callback invocations",
    calls_run::<Skipper<Kdk>, f64>(
        "Skipper",
        &ic([0.3; 8]),
        &uniforms(1e-3, 4, Table::Cap(3)),
        None
    )
);

// ── advance_equals_nsub_steps (REQ-INT-020) ──────────────────────────────────────────────────────────────────────

/// `n` `STEP` calls of `S` at `dt` from `(s, g)`.
fn steps<S: Stepper, R: P>(s: &State<R>, g: &Geom<R>, dt: R, n: u32) -> (State<R>, Geom<R>) {
    (0..n).fold((*s, *g), |(s, g), _| S::step(&s, dt, &g))
}

/// `out` is `want` bit for bit, after `n` steps, with `calls` callbacks.
fn check_advance<R: P>(
    name: &str,
    out: &Advanced<R>,
    want: &(State<R>, Geom<R>),
    n: u32,
    calls: u32,
) {
    assert_eq!(
        state_bits(&out.state, &out.geom),
        state_bits(&want.0, &want.1),
        "{name}: ADVANCE is not {n} STEP calls bit for bit"
    );
    assert_eq!(
        out.substeps, n,
        "{name}: ADVANCE reports {} substeps, not {n}",
        out.substeps
    );
    assert_eq!(calls, n, "{name}: {calls} callbacks for {n} substeps");
}

/// `ADVANCE` of `S` from `t` to `target` at `N_sub = n`, and its callbacks.
fn advance_once<S: Stepper, R: P>(
    s: &State<R>,
    g: &Geom<R>,
    t: u32,
    target: u32,
    dt_macro: R,
    n: u32,
) -> (Advanced<R>, u32) {
    let mut probe = Probe::default();
    let params = AdvanceParams { dt_macro, n_sub: n };
    let out = <S as Advance>::advance(s, g, t, target, &params, &mut probe);
    (out, probe.calls)
}

fn nsub_steps<S: Stepper, R: P>(name: &str, z: [f32; 8], n: u32, t: u32, dt: f32) {
    let name = &format!("{name} at {}, N_sub {n}, t {t}", R::LABEL);
    let ic = ic::<R>(z);
    let s = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let g = Geom::new(&s.m, &s.r);
    let dt_macro = of::<R>(dt);
    let h = dt_macro / of::<R>(n as f32);
    let (out, calls) = advance_once::<S, R>(&s, &g, t, t + 1, dt_macro, n);
    check_advance(name, &out, &steps::<S, R>(&s, &g, h, n), n, calls);
    let (out, calls) = advance_once::<S, R>(&s, &g, t, t + 2, dt_macro, n);
    check_advance(name, &out, &steps::<S, R>(&s, &g, h, 2 * n), 2 * n, calls);
    let (out, calls) = advance_once::<S, R>(&s, &g, t, t, dt_macro, n);
    check_advance(name, &out, &(s, g), 0, calls);
    if t > 0 {
        let (out, calls) = advance_once::<S, R>(&s, &g, t, t - 1, dt_macro, n);
        check_advance(name, &out, &(s, g), 0, calls);
    }
}

#[test]
fn advance_equals_nsub_steps() {
    let dt = prop_oneof![Just(1e-3f32), Just(4e-3f32), Just(0.05f32)];
    validation::prop::run(&(any_z(), 1u32..=64, 0u32..65535, dt), |(z, n, t, dt)| {
        every_occupant!(nsub_steps, f64, z, n, t, dt);
        every_occupant!(nsub_steps, f32, z, n, t, dt);
        Ok(())
    });
}

negative_control!(
    advance_equals_nsub_steps,
    "ADVANCE compared with one STEP too many",
    expected = "STEP calls bit for bit",
    {
        let ic = ic::<f64>([0.3; 8]);
        let s = State {
            m: ic.m,
            r: ic.r,
            p: ic.p,
        };
        let g = Geom::new(&s.m, &s.r);
        let (out, calls) = advance_once::<Kdk, f64>(&s, &g, 5, 6, 1e-3, 4);
        check_advance(
            "KDK",
            &out,
            &steps::<Kdk, f64>(&s, &g, 1e-3 / 4.0, 5),
            4,
            calls,
        );
    }
);

// ── total_substeps_honest (REQ-INT-022) ──────────────────────────────────────────────────────────────────────────

fn check_reported(name: &str, reported: u32, calls: u32) {
    assert_eq!(
        reported, calls,
        "{name}: reported total_substeps {reported} != callback invocations {calls}"
    );
}

/// `A`'s reported `total_substeps` against the callback's calls, under a [`Probe`] stopping at `stop` and under the
/// wrapper's own cadence.
fn reported_run<A: Advance, R: P>(
    name: &str,
    ic: &Ic<R>,
    u: &WrapperUniforms<R>,
    stop: Option<u32>,
) {
    let mut probe = Probe::stopping_at(stop);
    let out = integrate_with::<A, R, Probe>(ic, u, &mut probe);
    check_reported(name, out.total_substeps, probe.calls);
    let mut own = WrapperCadence::default();
    let out = integrate_with::<A, R, WrapperCadence>(ic, u, &mut own);
    check_reported(name, out.total_substeps, own.invocations());
    assert_eq!(
        integrate::<A, R>(ic, u),
        out,
        "{name}: integrate is not integrate_with under the wrapper's cadence"
    );
}

fn honest<A: Advance, R: P>(name: &str, z: [f32; 8], key: (f32, u32, Table), stop: Option<u32>) {
    reported_run::<A, R>(name, &ic::<R>(z), &uniforms::<R>(key.0, key.1, key.2), stop);
}

#[test]
fn total_substeps_honest() {
    let stop = proptest::option::of(1u32..=60);
    validation::prop::run(&(any_z(), any_key(), stop), |(z, key, stop)| {
        every_occupant!(honest, f64, z, key, stop);
        every_occupant!(honest, f32, z, key, stop);
        Ok(())
    });
}

negative_control!(
    total_substeps_honest,
    "an occupant reporting the steps it was asked for, stopped by done part-way through a macro-step",
    expected = "reported total_substeps",
    reported_run::<Liar<Kdk>, f64>(
        "Liar",
        &ic([0.3; 8]),
        &uniforms(1e-3, 4, Table::Cap(3)),
        Some(4)
    )
);

/// Stopped at the 4th callback with `N_sub = 3`: 4 substeps reported, two macro-steps begun, where `Σ N_sub` is 6.
fn check_mid_macro<A: Advance>(name: &str) {
    let mut probe = Probe::stopping_at(Some(4));
    let out = integrate_with::<A, f64, Probe>(
        &ic([0.3; 8]),
        &uniforms(1e-3, 4, Table::Cap(3)),
        &mut probe,
    );
    assert_eq!(
        (out.total_substeps, tm_t_end_step(out.times)),
        (4, 2),
        "{name}: a run stopped at the 4th of 3-substep macro-steps does not report 4 substeps over 2 macro-steps"
    );
}

#[test]
fn total_substeps_honest_mid_macro() {
    check_mid_macro::<Kdk>("KDK");
    check_mid_macro::<Rk4>("RK4");
}

negative_control!(
    total_substeps_honest_mid_macro,
    "an occupant reporting Σ N_sub",
    expected = "does not report 4 substeps",
    check_mid_macro::<Liar<Kdk>>("Liar")
);

// ── seam3_every_chart (REQ-INT-025) ──────────────────────────────────────────────────────────────────────────────

/// A check run on a chart.
trait Visit {
    fn chart<C: ChartDecode + Chart<Param = f32>>(&mut self, chart: &C);
}

/// One chart of each kind the registry names, with parameters as the decode tests take them.
fn every_chart(v: &mut impl Visit) {
    use validation::{
        BURRAU_THETA_LZ_FRACTION, BURRAU_THETA_LZ_K, BURRAU_THETA_MAX, BURRAU_THETA_MIN,
        EUCLID_M_MAX, EUCLID_M_MIN, EUCLID_NU_MAX, EUCLID_NU_MIN,
    };
    let k_max = K_MAX.number() as f32;
    let gamma_k = GAMMA_K.number() as f32;
    let theta_range = [
        BURRAU_THETA_MIN.number() as f32,
        BURRAU_THETA_MAX.number() as f32,
    ];
    let invariant = Invariant {
        z_cfg: [0.4, -0.3],
        z_mass: [0.2, -0.5],
        k_max,
        gamma_k,
    };
    v.chart(&Latent {
        z0: [0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1],
        q1: [0.5, 0.0, 0.2, 0.0, 0.0, 0.1, 0.0, 0.3],
        q2: [0.0, 0.5, 0.0, -0.2, 0.1, 0.0, 0.2, 0.0],
    });
    v.chart(&ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.2,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.2, -0.3],
        z_mom: [0.1, -0.2, 0.3, 0.0],
    });
    v.chart(&Burrau {
        nu_range: [0.05, 0.95],
        k_max,
        gamma_k,
    });
    v.chart(&EuclidPlane {
        m_range: [EUCLID_M_MIN.number() as f32, EUCLID_M_MAX.number() as f32],
        nu_range: [EUCLID_NU_MIN.number() as f32, EUCLID_NU_MAX.number() as f32],
    });
    v.chart(&BurrauThetaK {
        theta_range,
        k_max,
        gamma_k,
    });
    v.chart(&BurrauThetaLz {
        theta_range,
        k: BURRAU_THETA_LZ_K.number() as f32,
        lz_fraction: BURRAU_THETA_LZ_FRACTION.number() as f32,
    });
    v.chart(&BurrauThetaDeltaM {
        theta_range,
        m_target: [0.25, 0.375, 0.375],
    });
    v.chart(&InvariantLE(invariant));
    v.chart(&InvariantLK(invariant));
    v.chart(&MassSimplex {
        nu0: 0.5,
        eps_m: MASS_SIMPLEX_EPS_M.number() as f32,
    });
    v.chart(&Mixed::<ZMu1, Alpha, f32>::new(
        [0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1],
        [-2.0, 2.0],
        [0.1, 1.4],
    ));
}

/// What drives a decoded IC through the integrator: given the IC and the name of the chart that decoded it.
type Drive<R> = fn(&Ic<R>, &str) -> SimStateBaseOf<R>;

/// The sim key the seam-3 runs take.
fn seam3_key<R: P>() -> WrapperUniforms<R> {
    uniforms(4e-3, 4, Table::Default)
}

/// Each chart decoded at `uv` at `R`, driven by `drive`, and the same `(m, r, p)` as bare numbers driven by the
/// wrapper under `A`: the bits agree, and the run is labelled.
struct Seam3<A, R: P> {
    uv: [f32; 2],
    drive: Drive<R>,
    visited: BTreeSet<String>,
    occupant: PhantomData<A>,
}

impl<A: Advance, R: P> Visit for Seam3<A, R> {
    fn chart<C: ChartDecode + Chart<Param = f32>>(&mut self, chart: &C) {
        let name = chart.name().to_owned();
        let decoded = sample::<C, Defaults, R>(chart, of::<R>(self.uv[0]), of::<R>(self.uv[1])).ic;
        let got = (self.drive)(&decoded, &name);
        let want = integrate::<A, R>(&bare(&decoded), &seam3_key());
        assert_eq!(
            bits(&got),
            bits(&want),
            "chart {name} at {:?}, {}: the integrator's output depends on more than (m, r, p)",
            self.uv,
            R::LABEL
        );
        assert_eq!(
            sd_state(got.packed_a),
            STATE_BOUNDED,
            "chart {name}: the run is not labelled bounded at its horizon"
        );
        self.visited.insert(name);
    }
}

/// `ic`'s fifteen numbers written out as bits and read back into a new IC: nothing of the decode but `(m, r, p)`.
fn bare<R: P>(ic: &Ic<R>) -> Ic<R> {
    let words: Vec<u64> =
        ic.m.iter()
            .chain(ic.r.iter().flatten())
            .chain(ic.p.iter().flatten())
            .map(|x| x.bits())
            .collect();
    let at = |k: usize| R::from_bits(words[k]);
    Ic {
        m: [at(0), at(1), at(2)],
        r: [[at(3), at(4)], [at(5), at(6)], [at(7), at(8)]],
        p: [[at(9), at(10)], [at(11), at(12)], [at(13), at(14)]],
    }
}

/// The wrapper under `A`, which reads the IC alone.
fn wrapper_drive<A: Advance, R: P>(ic: &Ic<R>, _chart: &str) -> SimStateBaseOf<R> {
    integrate::<A, R>(ic, &seam3_key())
}

/// Every chart at `uv` under `A` at `R`, driven by `drive`; the charts visited are the registry.
fn seam3_with<A: Advance, R: P>(uv: [f32; 2], drive: Drive<R>) {
    let mut v = Seam3::<A, R> {
        uv,
        drive,
        visited: BTreeSet::new(),
        occupant: PhantomData,
    };
    every_chart(&mut v);
    let registry: BTreeSet<String> = REGISTRY.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        v.visited, registry,
        "the charts driven are not the registry"
    );
}

fn seam3<A: Advance, R: P>(_name: &str, uv: [f32; 2]) {
    seam3_with::<A, R>(uv, wrapper_drive::<A, R>);
}

#[test]
fn seam3_every_chart() {
    validation::prop::run(&(0f32..=1.0, 0f32..=1.0), |(u, v)| {
        every_occupant!(seam3, f64, [u, v]);
        every_occupant!(seam3, f32, [u, v]);
        Ok(())
    });
}

/// A drive that reads the chart's name: a longer horizon for the Burrau chart.
#[cfg(feature = "controls")]
fn chart_aware(ic: &Ic<f64>, chart: &str) -> SimStateBaseOf<f64> {
    let mut key = seam3_key::<f64>();
    key.horizon_steps += u32::from(chart == "Burrau");
    integrate::<Kdk, f64>(ic, &key)
}

negative_control!(
    seam3_every_chart,
    "an integration that learns the chart's name",
    expected = "depends on more than (m, r, p)",
    seam3_with::<Kdk, f64>([0.4, 0.6], chart_aware)
);

// ── advance_seam_r19 (REQ-INT-053, the stepper arm) ──────────────────────────────────────────────────────────────

/// For a stepper with regularisation `none`, `owns_time_mapping` is false; one `ADVANCE` at `N_sub = n` calls the
/// cadence `n` times, once after each of its `n` substeps.
fn check_r19<A: Advance>(name: &str, profile: CapabilityProfile, n: u32) {
    assert!(
        !profile.owns_time_mapping,
        "{name} with regularisation none reports owns_time_mapping"
    );
    let ic = ic::<f64>([0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.1]);
    let s = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let g = Geom::new(&s.m, &s.r);
    let mut probe = Probe::default();
    let params = AdvanceParams {
        dt_macro: 1e-3,
        n_sub: n,
    };
    let out = A::advance(&s, &g, 7, 8, &params, &mut probe);
    assert_eq!(
        (probe.calls, out.substeps),
        (n, n),
        "{name}: the cadence is not called once per substep"
    );
}

fn r19<S: Stepper, R: P>(name: &str, n: u32) {
    check_r19::<S>(name, S::profile(), n);
}

#[test]
fn advance_seam_r19() {
    for n in [1, 2, 3, 17, 64] {
        every_occupant!(r19, f64, n);
    }
    for (name, profile) in profiles() {
        assert!(!profile.owns_time_mapping, "{name} owns its time mapping");
    }
}

negative_control!(
    advance_seam_r19,
    "an occupant calling the cadence twice per substep",
    expected = "is not called once per substep",
    check_r19::<Twice<Kdk>>("Twice", Kdk::profile(), 3)
);

// ── wrapper_horizon_counter ──────────────────────────────────────────────────────────────────────────────────────

/// A run of `h` macro-steps under `table`, whose `N_sub` is `n`, stopped at the `stop`-th callback if any: the trace,
/// the counter, the label and the payload fields the wrapper writes.
fn check_horizon(h: u32, table: Table, n: u32, stop: Option<u32>) {
    let ic = ic::<f64>([0.3; 8]);
    let u = uniforms::<f64>(1e-3, h, table);
    let mut probe = Probe::stopping_at(stop);
    let (trace, out) = traced::<Kdk, f64, Probe>(&ic, &u, &mut probe);
    let substeps = stop.map_or(h * n, |k| k.min(h * n));
    let begun = substeps.div_ceil(n);
    let label = if stop.is_some_and(|k| k <= h * n) {
        STATE_RUNNING
    } else {
        STATE_BOUNDED
    };
    let what = format!("{h} macro-steps at N_sub {n}, stop {stop:?}");
    assert_eq!(
        trace.len() as u32,
        begun,
        "{what}: {} ADVANCE calls, not {begun}",
        trace.len()
    );
    assert_eq!(
        tm_t_end_step(out.times),
        begun,
        "{what}: t_end_step is not the macro-step counter"
    );
    assert_eq!(
        out.total_substeps, substeps,
        "{what}: total_substeps is not {substeps}"
    );
    assert_eq!(
        sd_state(out.packed_a),
        label,
        "{what}: the run is not labelled {label}"
    );
    assert_eq!(
        sd_saturated(out.packed_a),
        begun > 0 && n == u.n_max,
        "{what}: saturated is not set exactly where N_sub reached N_max"
    );
    assert!(
        pa_d_min_is_unset(out.packed_a),
        "{what}: d_min is not unset"
    );
    assert_eq!(
        sd_dmin_pair(out.packed_a),
        SD_DMIN_PAIR_SENTINEL,
        "{what}: dmin_pair is not its sentinel"
    );
    assert_eq!(
        tm_t_dmin_step(out.times),
        0,
        "{what}: t_dmin_step is written"
    );
    let moved = begun > 0;
    assert_eq!(
        (out.r, out.p) != (ic.r, ic.p),
        moved,
        "{what}: the state written is not where the march stopped"
    );
}

#[test]
fn wrapper_horizon_counter() {
    for h in [0, 1, 2, 7] {
        check_horizon(h, Table::One, 1, None);
        check_horizon(h, Table::Cap(3), 3, None);
        check_horizon(h, Table::Cap(1), 1, None);
    }
    for stop in [1, 2, 3, 4, 5, 6, 7] {
        check_horizon(2, Table::Cap(3), 3, Some(stop));
        check_horizon(5, Table::One, 1, Some(stop));
    }
}

negative_control!(
    wrapper_horizon_counter,
    "a horizon checked for one macro-step more than its counter",
    expected = "ADVANCE calls, not",
    {
        let ic = ic::<f64>([0.3; 8]);
        let u = uniforms::<f64>(1e-3, 3, Table::One);
        let (trace, _) = traced::<Kdk, f64, Probe>(&ic, &u, &mut Probe::default());
        assert_eq!(trace.len(), 4, "{} ADVANCE calls, not 4", trace.len());
    }
);

/// The wrapper's own cadence counts one call per substep and is never done while its hooks detect nothing; its hooks
/// leave the state as the step left it.
fn check_own_cadence(c: &WrapperCadence, substeps: u32) {
    assert!(
        !Cadence::<f64>::done(c),
        "the wrapper's cadence is done with nothing detected"
    );
    assert_eq!(
        c.invocations(),
        substeps,
        "the wrapper's cadence counted {} calls over {substeps} substeps",
        c.invocations()
    );
}

#[test]
fn wrapper_cadence_hooks() {
    let ic = ic::<f64>([0.3; 8]);
    let u = uniforms::<f64>(1e-3, 5, Table::Cap(2));
    let mut own = WrapperCadence::default();
    let a = integrate_with::<Kdk, f64, WrapperCadence>(&ic, &u, &mut own);
    check_own_cadence(&own, 10);
    let b = integrate_with::<Kdk, f64, Probe>(&ic, &u, &mut Probe::default());
    assert_eq!(
        bits(&a),
        bits(&b),
        "the wrapper's empty hooks changed the march"
    );
    let mut s = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let mut g = Geom::new(&s.m, &s.r);
    let before = state_bits(&s, &g);
    cadence::project(&mut s, &mut g);
    cadence::monitor(&s, &g);
    assert!(
        !cadence::detect(&s, &g),
        "detect fired with nothing to detect"
    );
    assert_eq!(
        state_bits(&s, &g),
        before,
        "the empty hooks moved the state"
    );
}

negative_control!(
    wrapper_cadence_hooks,
    "a cadence's count checked against one substep too many",
    expected = "calls over",
    {
        let ic = ic::<f64>([0.3; 8]);
        let u = uniforms::<f64>(1e-3, 5, Table::Cap(2));
        let mut own = WrapperCadence::default();
        integrate_with::<Kdk, f64, WrapperCadence>(&ic, &u, &mut own);
        check_own_cadence(&own, 11);
    }
);
