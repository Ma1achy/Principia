//! QA tests for TASK-M3-68's terminal on the landing step of a call's own target boundary (PR #502), written from
//! integrator contract Part 2a § "The sync grid is a fixed quantum of physical time (R-565)" ("a terminal event …
//! still latches at the step it fires on, and the copy's march ends there"; "ADVANCE … advances to the latest boundary
//! at or before `t_target`"), Part 2a's exact step-index timing invariant (`t_end_step` the completed-macro-step count,
//! latched at a terminal) and payload §4 (one Welford sample per completed macro-step), not from the implementation:
//! - `qa_m368_terminal_on_target_landing`: Aarseth–Zare + RK4 through the wrapper over eight macro-steps, at f32 and
//!   f64, at `dt_macro = Δ_sync` and `dt_macro = 2·Δ_sync`, with a terminal that fires on the step landing on the
//!   third macro-step's boundary. That step reaches the boundary, so the third macro-step is completed: the run is
//!   bitwise the same wrapper run stopped by its horizon at three macro-steps (`r`, `p`, step total, `t_end_step = 3`
//!   and the Welford co-moments of three samples), the cadence never run again, and the run not failed. A terminal one
//!   step earlier, short of that boundary, completes only two: bitwise the run with a horizon of two macro-steps, its
//!   state that of the step it fired on.
//!
//! The negative control (R-176) runs the same check with a seam that reads every call whose cadence is done as short of
//! its target: the third macro-step is then dropped and the check fails.

use std::fmt::Debug;

use kernel::decode::Ic;
use kernel::driver::advance::{Advance, Cadence};
#[cfg(feature = "controls")]
use kernel::driver::advance::{AdvanceParams, AdvanceReal, Advanced, Reached};
use kernel::driver::substep::NsubThresholds;
use kernel::driver::wrapper::{integrate_with, WrapperReal, WrapperUniforms};
use kernel::generated::nsub_thresholds::NSUB_TABLE_ROWS;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::{Geom, State};
use kernel::payload::{sd_state, tm_t_end_step, SimStateBaseOf, STATE_SIM_FAILED};
use kernel::regularisation::az::{AarsethZare, AzConfig, AzParams, AzReal};
use kernel::regularisation::Composed;
use spirv_std::glam::UVec4;
use validation::negative_control;

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
/// lengths over 5).
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
/// reference's `TINY` (R-953), R-160's `f = 0.02`, a budget that does not bind, the clamp and the landing's retakes on
/// (the secant's 4, the bisection's 64, Part 2b). Changed by TASK-M3-76: the `|R₃|` floor is `TINY` (R-999), and the
/// landing takes no tolerance (R-998); *was* `r3_floor` `0` / `1e-300` and `land_eps_rel` `1e-5` / `1e-14`.
struct Grid;

impl AzConfig for Grid {
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

/// Aarseth–Zare + RK4.
type AzRk4 = Composed<Rk4, AarsethZare<Grid>>;

/// A seam that reads every call whose cadence is done as short of its target (a control's).
#[cfg(feature = "controls")]
struct DoneIsShort<A>(core::marker::PhantomData<A>);

#[cfg(feature = "controls")]
impl<A: Advance> Advance for DoneIsShort<A> {
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
        let (out, reached) = A::advance_reaching(state, geom, t_now, t_target, params, cadence);
        if reached == Reached::Target && cadence.done() {
            (out, Reached::Short)
        } else {
            (out, reached)
        }
    }
}

/// The wrapper's run by `A` at `R`.
fn run<A: Advance, R: P>(
    dt_macro: f64,
    horizon: u32,
    cadence: &mut DoneAfter,
) -> SimStateBaseOf<R> {
    integrate_with::<A, R, DoneAfter>(&burrau::<R>(), &key::<R>(dt_macro, horizon), cadence)
}

/// The bits a run is compared on: `r`, `p`, the step total, `t_end_step` and the Welford co-moments.
fn fingerprint<R: P>(out: &SimStateBaseOf<R>) -> Vec<u64> {
    let mut v: Vec<u64> = out
        .r
        .iter()
        .chain(out.p.iter())
        .flatten()
        .map(|x| x.bits())
        .collect();
    v.push(u64::from(out.total_substeps));
    v.push(u64::from(tm_t_end_step(out.times)));
    v.push(out.mean_y.bits());
    v.push(out.C_ty.bits());
    v
}

/// The check at `R` and `dt_macro`, the seam `A` under test; the reference runs are Aarseth–Zare + RK4's own.
fn check<A: Advance, R: P>(dt_macro: f64) {
    // The reference: the run stopped by its horizon after three macro-steps, no terminal; its cadence calls are the
    // steps up to and including the one landing on the third boundary.
    let mut free3 = DoneAfter::new(u32::MAX);
    let three = run::<AzRk4, R>(dt_macro, 3, &mut free3);
    assert_eq!(
        tm_t_end_step(three.times),
        3,
        "the reference completes three macro-steps"
    );
    let mut free2 = DoneAfter::new(u32::MAX);
    let two = run::<AzRk4, R>(dt_macro, 2, &mut free2);
    let n = free3.calls;
    assert!(
        n > free2.calls + 1,
        "the third macro-step takes more than one step"
    );

    let mut on_landing = DoneAfter::new(n);
    let got = run::<A, R>(dt_macro, 8, &mut on_landing);
    assert_eq!(
        tm_t_end_step(got.times),
        3,
        "a terminal on the landing step of the target boundary completes that macro-step (dt_macro = {dt_macro})"
    );
    assert_eq!(
        fingerprint(&got),
        fingerprint(&three),
        "the run is the three completed macro-steps, bitwise, its third Welford sample the landing state"
    );
    assert_eq!(on_landing.calls, n, "an ended march is never run on");
    assert_ne!(
        sd_state(got.packed_a),
        STATE_SIM_FAILED,
        "a terminal on a landing is not a failure"
    );

    // One step earlier, short of the boundary: two macro-steps completed, the state that of the step fired on.
    let mut early = DoneAfter::new(n - 1);
    let short = run::<A, R>(dt_macro, 8, &mut early);
    assert_eq!(
        tm_t_end_step(short.times),
        2,
        "a terminal short of the target boundary leaves that macro-step uncompleted"
    );
    assert_eq!(
        (short.mean_y.bits(), short.C_ty.bits()),
        (two.mean_y.bits(), two.C_ty.bits()),
        "no Welford sample for the macro-step not completed"
    );
    assert_ne!(
        fingerprint(&short)[..12],
        fingerprint(&three)[..12],
        "the earlier terminal latches a state short of the boundary"
    );
    assert_eq!(early.calls, n - 1, "an ended march is never run on");
}

fn check_all(f32_at: fn(f64), f64_at: fn(f64)) {
    for dt_macro in [0.125, 0.25] {
        f32_at(dt_macro);
        f64_at(dt_macro);
    }
}

#[test]
fn qa_m368_terminal_on_target_landing() {
    check_all(check::<AzRk4, f32>, check::<AzRk4, f64>);
}

negative_control!(
    qa_m368_terminal_on_target_landing,
    "every call whose cadence is done read as short of its target",
    expected = "a terminal on the landing step of the target boundary completes that macro-step",
    check_all(
        check::<DoneIsShort<AzRk4>, f32>,
        check::<DoneIsShort<AzRk4>, f64>,
    )
);
