//! QA tests for TASK-M3-68's seam end (PR #502, the `Reached` seam), written from integrator contract Part 2a
//! § "The sync grid is a fixed quantum of physical time (R-565)", Part 2a § "What this preserves, and what it costs",
//! payload §2 (`t_end_step`, `sim_failed` and its detail) and §4 (one Welford sample per completed macro-step), not
//! from the implementation:
//! - `qa_m368_seam_non_finite_latches`, `qa_m368_seam_exhausted_latches`: a terminal "still latches at the step it
//!   fires on, and the copy's march ends there" (R-565's bullet). A scripted occupant, through the wrapper, ends its
//!   march at its fourth call, after three completed calls whose steps were never `N_sub`: the run stops at once (no
//!   further call), `t_end_step = 3`, the Welford co-moments those of exactly the three completed states, the state
//!   the one the occupant latched; non-finite is `sim_failed` with detail 2 (a finite state, payload §2), a spent
//!   budget `running`.
//! - `qa_m368_seam_failed_over_collision`: R-30, `sim_failed` first: a march that ends non-finite on the step a
//!   collision fires stays `sim_failed` through the monitored write, `dmin_pair` its sentinel; the same step reported
//!   short is labelled `collision`, so the fixture does fire one.
//! - `qa_m368_sync_grid_routes`: "live march, catch-up and re-boot after eviction reach the same state at the same grid
//!   time, bitwise", the grid independent of frame timing: Aarseth–Zare + RK4 through the wrapper at macro-steps
//!   `1/16, 1/8, 3/32, 3/16, 0.1, 1/4, 3/8, 1/2` gives the same `r`, `p` and step total at each grid time, at f32 and
//!   f64; every macro-step completed (`t_end_step = horizon_steps`, `bounded`); and at `dt_macro = 3/16` the Welford
//!   samples are the boundary states `t = 1/8` and `t = 3/8`, as the playhead reads them.
//! - `qa_m368_none_bitwise_every_stepper`: `none` composed with each of Euler, KDK, RK4, Yoshida-4 and Yoshida-6 is
//!   bitwise the stepper's own run through the wrapper, at f32 and f64, under a cadence done at every substep of the
//!   first three macro-steps and never, and through the monitored write with and without a collision radius.
//!
//! Each check takes the code under test as a function, so its negative control (R-176) runs the same check on a faulty
//! one.

use std::fmt::Debug;

use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::advance::{Advance, AdvanceParams, AdvanceReal, Advanced, Cadence, Reached};
use kernel::driver::substep::NsubThresholds;
use kernel::driver::welford::{self, Welford};
use kernel::driver::wrapper::{integrate_detecting, integrate_with, WrapperReal, WrapperUniforms};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{CapabilityProfile, Geom, State, Stepper};
use kernel::payload::{
    sd_detail, sd_dmin_pair, sd_state, tm_t_end_step, SimStateBaseOf, SD_DMIN_PAIR_SENTINEL,
    STATE_BOUNDED, STATE_COLLISION, STATE_RUNNING, STATE_SIM_FAILED,
};
use kernel::regularisation::az::{AarsethZare, AzConfig, AzParams, AzReal};
use kernel::regularisation::{Composed, Kind, NoRegularisation, Regularisation};
use kernel::shape::shape;
use spirv_std::glam::UVec4;
use validation::negative_control;

// ── Fixtures ───────────────────────────────────────────────────────────────────────────────────────────────────

/// A precision the tests run at.
trait P: AzReal + WrapperReal + Debug {
    fn of(x: f64) -> Self;
    fn bits(self) -> u64;
}

impl P for f32 {
    fn of(x: f64) -> f32 {
        x as f32
    }
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
}

impl P for f64 {
    fn of(x: f64) -> f64 {
        x
    }
    fn bits(self) -> u64 {
        self.to_bits()
    }
}

/// Burrau's masses 3, 4, 5 on their triangle with small momenta summing to zero, in the gauge `M = 1` (masses over 12,
/// lengths over 5): every pair at least 0.6 apart.
fn burrau<R: P>() -> Ic<R> {
    let m = [3.0 / 12.0, 4.0 / 12.0, 5.0 / 12.0];
    let r = [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]].map(|x: [f64; 2]| x.map(|c| c / 5.0));
    let p = [[0.03, -0.01], [-0.02, 0.04], [-0.01, -0.03]];
    Ic {
        m: m.map(R::of),
        r: r.map(|x| x.map(R::of)),
        p: p.map(|x| x.map(R::of)),
    }
}

/// A sim key: `dt_macro`, `horizon_steps`, `N_max = 4`, every `N_sub` threshold the largest finite f32.
fn key<R: P>(dt_macro: f64, horizon_steps: u32) -> WrapperUniforms<R> {
    WrapperUniforms {
        dt_macro: R::of(dt_macro),
        horizon_steps,
        n_max: 4,
        thresholds: NsubThresholds {
            rows: [UVec4::splat(f32::MAX.to_bits()); NSUB_TABLE_ROWS],
        },
    }
}

/// The bits of `r` and `p`.
fn rp_bits<R: P>(r: &[[R; 2]; 3], p: &[[R; 2]; 3]) -> Vec<u64> {
    r.iter()
        .chain(p.iter())
        .flatten()
        .map(|x| x.bits())
        .collect()
}

/// A cadence that counts its calls and is done from its `n`-th on (never, at `u32::MAX`).
struct DoneAfter {
    n: u32,
    calls: u32,
}

impl DoneAfter {
    fn new(n: u32) -> Self {
        DoneAfter { n, calls: 0 }
    }
}

impl<R> Cadence<R> for DoneAfter {
    fn after_step(&mut self, _state: &mut State<R>, _geom: &mut Geom<R>) {
        self.calls += 1;
    }
    fn done(&self) -> bool {
        self.calls >= self.n
    }
}

/// Aarseth–Zare test values (test inputs, not the calibrations): `η = 0.01`, `Δ_sync = 1/8` (exact in binary), the
/// reference's `TINY` (R-953), R-160's `f = 0.02`, a budget that does not bind, the clamp and the secant on.
struct Grid;

impl AzConfig for Grid {
    fn params<R: AzReal>() -> AzParams<R> {
        AzParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: R::at_width(0.125, 0.125),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            r3_floor: R::at_width(0.0, 1e-300),
            land_eps_rel: R::at_width(1e-5, 1e-14),
            step_budget_rate: R::at_width(1e7, 1e7),
            n_min: 1_000_000,
            clamp: true,
            secant_tries: 4,
        }
    }
}

/// Aarseth–Zare + RK4.
type AzRk4 = Composed<Rk4, AarsethZare<Grid>>;

/// `A` read by the stepper's rule alone (the default `advance_reaching`): its end dropped (a control's).
#[cfg(feature = "controls")]
struct ByCount<A>(core::marker::PhantomData<A>);

#[cfg(feature = "controls")]
impl<A: Advance> Advance for ByCount<A> {
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

/// `A` with every call reported as its target reached (a control's).
#[cfg(feature = "controls")]
struct AlwaysTarget<A>(core::marker::PhantomData<A>);

#[cfg(feature = "controls")]
impl<A: Advance> Advance for AlwaysTarget<A> {
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
        (
            A::advance(state, geom, t_now, t_target, params, cadence),
            Reached::Target,
        )
    }
}

// ── A scripted occupant that owns its time mapping ─────────────────────────────────────────────────────────────

/// The ends a [`Script`] reports at its firing call.
const SHORT: u8 = 1;
const NON_FINITE: u8 = 2;
const EXHAUSTED: u8 = 3;

fn end_of(end: u8) -> Reached {
    match end {
        SHORT => Reached::Short,
        NON_FINITE => Reached::NonFinite,
        EXHAUSTED => Reached::Exhausted,
        _ => Reached::Target,
    }
}

/// One scripted call's move: body 0 shifted by `1/64` in `x`, which changes the shape.
fn moved<R: AdvanceReal>(s: &State<R>) -> State<R> {
    let mut s = *s;
    s.r[0][0] = s.r[0][0] + R::count(1) / R::count(64);
    s
}

/// A stand-in regularisation (owning its time mapping, `KIND` Aarseth–Zare): every call before `t_now = FIRE` moves the
/// state ([`moved`]), runs the cadence once, reports 3 steps (never the wrapper's `N_sub`, 4 or 1) and its target
/// reached. The call at `FIRE` reports `END`: with `COLLIDE` it moves body 1 to `1/128` from body 0 and runs the
/// cadence there, one step; without, it takes no step and returns its state as it was. Later calls move as before, so
/// a wrapper that ran on past an ended march would show it.
struct Script<const FIRE: u32, const END: u8, const COLLIDE: bool>;

impl<const FIRE: u32, const END: u8, const COLLIDE: bool> Regularisation
    for Script<FIRE, END, COLLIDE>
{
    const KIND: Kind = Kind::AarsethZare;

    fn profile<S: Stepper>() -> CapabilityProfile {
        AarsethZare::<Grid>::profile::<S>()
    }

    fn advance<S: Stepper, R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        Self::advance_reaching::<S, R, C>(state, geom, t_now, t_target, params, cadence).0
    }

    fn advance_reaching<S: Stepper, R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        _geom: &Geom<R>,
        t_now: u32,
        _t_target: u32,
        _params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> (Advanced<R>, Reached) {
        if t_now == FIRE {
            if COLLIDE {
                let mut s = *state;
                s.r[1] = [s.r[0][0] + R::count(1) / R::count(128), s.r[0][1]];
                let mut g = Geom::new(&s.m, &s.r);
                cadence.after_step(&mut s, &mut g);
                return (
                    Advanced {
                        state: s,
                        geom: g,
                        substeps: 1,
                    },
                    end_of(END),
                );
            }
            return (
                Advanced {
                    state: *state,
                    geom: Geom::new(&state.m, &state.r),
                    substeps: 0,
                },
                end_of(END),
            );
        }
        let mut s = moved(state);
        let mut g = Geom::new(&s.m, &s.r);
        cadence.after_step(&mut s, &mut g);
        (
            Advanced {
                state: s,
                geom: g,
                substeps: 3,
            },
            Reached::Target,
        )
    }
}

/// The wrapper's run of `ic` under `uniforms` with the cadence `C`.
type Run<C> = fn(&Ic<f64>, &WrapperUniforms<f64>, &mut C) -> SimStateBaseOf<f64>;

fn run_by<A: Advance, C: Cadence<f64>>(
    ic: &Ic<f64>,
    uniforms: &WrapperUniforms<f64>,
    cadence: &mut C,
) -> SimStateBaseOf<f64> {
    integrate_with::<A, f64, C>(ic, uniforms, cadence)
}

/// The script's states after its first `n` calls, and the Welford co-moments of those `n` as completed macro-steps at
/// `dt_macro = 1/16` (payload §4: the sample `y` of the state reached, `n` the completed count).
fn model(n: u32) -> (State<f64>, Welford<f64>) {
    let ic = burrau::<f64>();
    let n_0 = shape(ic.r, ic.m);
    let mut s = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let mut w = Welford::default();
    for k in 1..=n {
        s = moved(&s);
        w = w.update(welford::y(s.r, s.m, n_0), k, 1.0 / 16.0);
    }
    (s, w)
}

/// A march ended at the fourth call (`t_now = 3`) of eight, through the wrapper: the run is `expected`, latched there.
fn check_latch(run: Run<DoneAfter>, expected: u32, what: &str) {
    let ic = burrau::<f64>();
    let mut cadence = DoneAfter::new(u32::MAX);
    let out = run(&ic, &key(1.0 / 16.0, 8), &mut cadence);
    assert_eq!(
        sd_state(out.packed_a),
        expected,
        "{what}: state {}",
        sd_state(out.packed_a)
    );
    assert_eq!(
        tm_t_end_step(out.times),
        3,
        "the run latches at the macro-step its march ended in, which is not completed"
    );
    assert_eq!(cadence.calls, 3, "an ended march is never run on");
    assert_eq!(out.total_substeps, 9, "the steps the occupant reported");
    let (s, w) = model(3);
    assert_eq!(
        rp_bits(&out.r, &out.p),
        rp_bits(&s.r, &s.p),
        "the state is the one the occupant latched"
    );
    assert_eq!(
        (out.mean_y.to_bits(), out.C_ty.to_bits()),
        (w.mean_y.to_bits(), w.c_ty.to_bits()),
        "one Welford sample per completed macro-step, none for the one that ended"
    );
    assert!(w.c_ty != 0.0, "the samples see the shape move");
}

// ── Tests ──────────────────────────────────────────────────────────────────────────────────────────────────────

type NonFiniteAt3 = Composed<Rk4, Script<3, NON_FINITE, false>>;
type ExhaustedAt3 = Composed<Rk4, Script<3, EXHAUSTED, false>>;

fn check_non_finite(run: Run<DoneAfter>, monitored: fn(&Ic<f64>) -> SimStateBaseOf<f64>) {
    check_latch(
        run,
        STATE_SIM_FAILED,
        "a march ended non-finite fails the run",
    );
    let mut cadence = DoneAfter::new(u32::MAX);
    let out = run(&burrau(), &key(1.0 / 16.0, 8), &mut cadence);
    assert_eq!(
        sd_detail(out.packed_a),
        2,
        "a non-finite end on a finite state is detail 2"
    );
    let full = monitored(&burrau());
    assert_eq!(
        sd_state(full.packed_a),
        STATE_SIM_FAILED,
        "monitored keeps sim_failed"
    );
    assert_eq!(sd_detail(full.packed_a), 2, "monitored keeps the detail");
    assert_eq!(tm_t_end_step(full.times), 3, "monitored keeps t_end_step");
    assert_eq!(
        sd_dmin_pair(full.packed_a),
        SD_DMIN_PAIR_SENTINEL,
        "a failed run's dmin_pair is its sentinel"
    );
}

fn monitored_non_finite(ic: &Ic<f64>) -> SimStateBaseOf<f64> {
    integrate_detecting::<NonFiniteAt3, f64>(ic, &key(1.0 / 16.0, 8), CollisionRadius::none())
}

#[test]
fn qa_m368_seam_non_finite_latches() {
    check_non_finite(run_by::<NonFiniteAt3, DoneAfter>, monitored_non_finite);
}

negative_control!(
    qa_m368_seam_non_finite_latches,
    "the march's end dropped, the occupant read by the stepper's rule",
    expected = "a march ended non-finite fails the run",
    check_non_finite(run_by::<ByCount<NonFiniteAt3>, DoneAfter>, |ic| {
        integrate_detecting::<ByCount<NonFiniteAt3>, f64>(
            ic,
            &key(1.0 / 16.0, 8),
            CollisionRadius::none(),
        )
    })
);

#[test]
fn qa_m368_seam_exhausted_latches() {
    check_latch(
        run_by::<ExhaustedAt3, DoneAfter>,
        STATE_RUNNING,
        "a spent budget stops the copy as it stood",
    );
}

negative_control!(
    qa_m368_seam_exhausted_latches,
    "the march ended one call later than the check expects",
    expected = "the run latches at the macro-step its march ended in",
    check_latch(
        run_by::<Composed<Rk4, Script<4, EXHAUSTED, false>>, DoneAfter>,
        STATE_RUNNING,
        "a spent budget stops the copy as it stood",
    )
);

/// The run of `ic` with the wrapper's own cadence at collision radius `r_coll = 1/20`, its monitored fields written.
type Detecting = fn(&Ic<f64>) -> SimStateBaseOf<f64>;

fn detecting<A: Advance>(ic: &Ic<f64>) -> SimStateBaseOf<f64> {
    integrate_detecting::<A, f64>(ic, &key(1.0 / 16.0, 8), CollisionRadius::from_r_coll(0.05))
}

/// R-30, `sim_failed` first: the march ended non-finite on the step a collision fired stays `sim_failed`.
fn check_failed_over_collision(failing: Detecting) {
    let out = failing(&burrau());
    assert_eq!(
        sd_state(out.packed_a),
        STATE_SIM_FAILED,
        "sim_failed first over a collision (R-30): state {}",
        sd_state(out.packed_a)
    );
    assert_eq!(sd_detail(out.packed_a), 2, "the failure's detail");
    assert_eq!(
        sd_dmin_pair(out.packed_a),
        SD_DMIN_PAIR_SENTINEL,
        "a failed run's dmin_pair is its sentinel"
    );
    assert_eq!(
        tm_t_end_step(out.times),
        3,
        "latched at the step it fired on"
    );
}

#[test]
fn qa_m368_seam_failed_over_collision() {
    let short = detecting::<Composed<Rk4, Script<3, SHORT, true>>>(&burrau());
    assert_eq!(
        sd_state(short.packed_a),
        STATE_COLLISION,
        "the fixture's step fires a collision"
    );
    assert_eq!(tm_t_end_step(short.times), 3);
    check_failed_over_collision(detecting::<Composed<Rk4, Script<3, NON_FINITE, true>>>);
}

negative_control!(
    qa_m368_seam_failed_over_collision,
    "the same colliding step reported short, not non-finite",
    expected = "sim_failed first over a collision",
    check_failed_over_collision(detecting::<Composed<Rk4, Script<3, SHORT, true>>>)
);

// ── R-565 through the wrapper ──────────────────────────────────────────────────────────────────────────────────

/// The wrapper's run by an occupant under a quiet cadence.
type GridRun<R> = fn(&Ic<R>, &WrapperUniforms<R>) -> SimStateBaseOf<R>;

fn quiet<A: Advance, R: P>(ic: &Ic<R>, uniforms: &WrapperUniforms<R>) -> SimStateBaseOf<R> {
    integrate_with::<A, R, DoneAfter>(ic, uniforms, &mut DoneAfter::new(u32::MAX))
}

/// Every `(dt_macro, horizon_steps)` in `routes` reaches, by the march stopping only on the grid, the same grid time:
/// each run's macro-steps all completed and the run `bounded`, and every run's `r`, `p` and step total bitwise the
/// first's.
fn check_routes<R: P>(run: GridRun<R>, routes: &[(f64, u32)]) -> SimStateBaseOf<R> {
    let ic = burrau::<R>();
    let first = run(&ic, &key(routes[0].0, routes[0].1));
    assert!(first.total_substeps > 0, "the march stepped");
    for &(dt, h) in routes {
        let out = run(&ic, &key(dt, h));
        assert_eq!(
            tm_t_end_step(out.times),
            h,
            "every macro-step whose march landed on the grid is completed (dt_macro {dt})"
        );
        assert_eq!(sd_state(out.packed_a), STATE_BOUNDED, "dt_macro {dt}");
        assert_eq!(
            (rp_bits(&out.r, &out.p), out.total_substeps),
            (rp_bits(&first.r, &first.p), first.total_substeps),
            "live march, catch-up and re-boot reach the same state bitwise: dt_macro {dt} × {h} against {:?}",
            routes[0]
        );
    }
    first
}

/// Routes to `t = 3/8`, the boundary `k = 3`: each's last target at or after it and before `t = 1/2`.
const TO_3_8: [(f64, u32); 6] = [
    (0.125, 3),
    (0.0625, 7),
    (0.1875, 2),
    (0.09375, 4),
    (0.1, 4),
    (0.375, 1),
];

/// Routes to `t = 1/2`, the boundary `k = 4`.
const TO_1_2: [(f64, u32); 4] = [(0.125, 4), (0.0625, 8), (0.25, 2), (0.5, 1)];

fn check_grid<R: P>(run: GridRun<R>, to_3_8: &[(f64, u32)]) {
    let at_3_8 = check_routes(run, to_3_8);
    let at_1_2 = check_routes(run, &TO_1_2);
    assert_ne!(
        rp_bits(&at_3_8.r, &at_3_8.p),
        rp_bits(&at_1_2.r, &at_1_2.p),
        "the two grid times differ"
    );
    // At dt_macro = 3/16 the two completed macro-steps sample the boundary states t = 1/8 and t = 3/8.
    let ic = burrau::<R>();
    let n_0 = shape(ic.r, ic.m);
    let b1 = run(&ic, &key(0.125, 1));
    let b3 = run(&ic, &key(0.125, 3));
    let h = R::of(0.1875);
    let w = Welford::default()
        .update(welford::y(b1.r, ic.m, n_0), 1, h)
        .update(welford::y(b3.r, ic.m, n_0), 2, h);
    let out = run(&ic, &key(0.1875, 2));
    assert_eq!(
        (out.mean_y.bits(), out.C_ty.bits()),
        (w.mean_y.bits(), w.c_ty.bits()),
        "one Welford sample per completed macro-step, on the boundary state it reached: {:?} {:?} against {:?} {:?}",
        out.mean_y,
        out.C_ty,
        w.mean_y,
        w.c_ty
    );
}

#[test]
fn qa_m368_sync_grid_routes() {
    check_grid::<f64>(quiet::<AzRk4, f64>, &TO_3_8);
    check_grid::<f32>(quiet::<AzRk4, f32>, &TO_3_8);
}

negative_control!(
    qa_m368_sync_grid_routes,
    "the march's end dropped, a macro-step completed only at N_sub steps",
    expected = "every macro-step whose march landed on the grid is completed",
    check_grid::<f64>(quiet::<ByCount<AzRk4>, f64>, &TO_3_8)
);

#[test]
fn qa_m368_sync_grid_routes_differ() {
    check_grid::<f64>(quiet::<AzRk4, f64>, &TO_3_8);
}

negative_control!(
    qa_m368_sync_grid_routes_differ,
    "a route to t = 1/2 among the routes to t = 3/8",
    expected = "live march, catch-up and re-boot reach the same state bitwise",
    check_grid::<f64>(quiet::<AzRk4, f64>, &[(0.125, 3), (0.0625, 7), (0.0625, 8)])
);

// ── `none` is the stepper ──────────────────────────────────────────────────────────────────────────────────────

type NoneRun<R> = fn(&Ic<R>, &WrapperUniforms<R>, &mut DoneAfter) -> SimStateBaseOf<R>;
type NoneMonitored<R> = fn(&Ic<R>, &WrapperUniforms<R>, CollisionRadius) -> SimStateBaseOf<R>;

fn with<A: Advance, R: P>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    cadence: &mut DoneAfter,
) -> SimStateBaseOf<R> {
    integrate_with::<A, R, DoneAfter>(ic, uniforms, cadence)
}

fn monitored<A: Advance, R: P>(
    ic: &Ic<R>,
    uniforms: &WrapperUniforms<R>,
    radius: CollisionRadius,
) -> SimStateBaseOf<R> {
    integrate_detecting::<A, R>(ic, uniforms, radius)
}

/// The run by `pair` is the run by `own`, bitwise in every field, under a cadence done at its `k`-th call for each `k`
/// over the first three macro-steps' substeps and never, and through the monitored write.
fn check_none<R: P>(
    (pair, own): (NoneRun<R>, NoneRun<R>),
    (pair_m, own_m): (NoneMonitored<R>, NoneMonitored<R>),
    name: &str,
) {
    let ic = burrau::<R>();
    let uniforms = key::<R>(1.0 / 16.0, 8);
    for k in (1..=13).chain([u32::MAX]) {
        let (mut a, mut b) = (DoneAfter::new(k), DoneAfter::new(k));
        let x = pair(&ic, &uniforms, &mut a);
        let y = own(&ic, &uniforms, &mut b);
        assert_eq!(
            (format!("{x:?}"), a.calls),
            (format!("{y:?}"), b.calls),
            "none is bitwise the stepper's own run: {name}, done at call {k}"
        );
    }
    for radius in [CollisionRadius::none(), CollisionRadius::from_r_coll(0.3)] {
        let x = pair_m(&ic, &uniforms, radius);
        let y = own_m(&ic, &uniforms, radius);
        assert_eq!(
            format!("{x:?}"),
            format!("{y:?}"),
            "none is bitwise the stepper's own run: {name}, monitored"
        );
    }
}

macro_rules! none_matches {
    ($s:ty, $r:ty) => {
        check_none::<$r>(
            (with::<Composed<$s, NoRegularisation>, $r>, with::<$s, $r>),
            (
                monitored::<Composed<$s, NoRegularisation>, $r>,
                monitored::<$s, $r>,
            ),
            concat!(stringify!($s), " at ", stringify!($r)),
        )
    };
}

#[test]
fn qa_m368_none_bitwise_every_stepper() {
    none_matches!(Euler, f32);
    none_matches!(Euler, f64);
    none_matches!(Kdk, f32);
    none_matches!(Kdk, f64);
    none_matches!(Rk4, f32);
    none_matches!(Rk4, f64);
    none_matches!(Yoshida4, f32);
    none_matches!(Yoshida4, f64);
    none_matches!(Yoshida6, f32);
    none_matches!(Yoshida6, f64);
}

negative_control!(
    qa_m368_none_bitwise_every_stepper,
    "none read with every call its target reached",
    expected = "none is bitwise the stepper's own run",
    check_none::<f64>(
        (
            with::<AlwaysTarget<Composed<Kdk, NoRegularisation>>, f64>,
            with::<Kdk, f64>,
        ),
        (
            monitored::<Composed<Kdk, NoRegularisation>, f64>,
            monitored::<Kdk, f64>,
        ),
        "Kdk with every call counted",
    )
);
