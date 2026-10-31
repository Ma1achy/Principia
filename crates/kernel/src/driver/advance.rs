//! The occupant seam, `ADVANCE(state, t_now, t_target, params) → state'` (integrator_contract Part 1, Part 2a; R-19;
//! systems_architecture §5, seam 4).
//!
//! **The wrapper owns the target; the occupant owns how it gets there** (Part 2a). The wrapper calls [`Advance::advance`]
//! once per macro-step, from `t_now` to `t_target`, with the `N_sub` its substep law chose ([`AdvanceParams::n_sub`])
//! and its per-substep cadence ([`Cadence`]), and the occupant returns the state at `t_target` with the number of
//! internal steps it took ([`Advanced::substeps`]).
//!
//! **Times are the macro-step grid's** (Part 5; dd_integrator §3.3): `t_now` and `t_target` are integer indices on it,
//! physical time `index · dt_macro`, so no float time is formed, accumulated or compared on the path.
//!
//! **The cadence is passed in, not run after** (Part 2a, Cost 1; R-19). Projection, invariant accumulation and terminal
//! detection run inside the occupant's march, after every internal step, through [`Cadence::after_step`]; an occupant
//! that skips the call skips them, which the callback's count against the steps taken catches (REQ-INT-009). The march
//! ends early only on the cadence's `done` flag ([`Cadence::done`]), read in the loop condition: no `break` (Part 1;
//! gpu_determinism_note § "The discipline", rule 5).
//!
//! **The steppers' `ADVANCE`** (Part 1, Part 2a: "a rename, no behaviour change"). Every [`Stepper`] implements it by the
//! blanket impl: `N_sub` `STEP` calls per macro-step of the span at the fixed `dt_macro / N_sub`, the callback after
//! each, a `while` loop bounded by the count (Part 2c rule 3). The occupant reads only the state `(m, r, p)`, the step
//! `dt`, and `G`, which is 1 in the force (Part 3, Part 5); no chart reaches it (seam 3). The regularised occupants,
//! which own their time mapping, implement `ADVANCE` themselves (TASK-M3-07).
//!
//! **Determinism** (REQ-INT-007, REQ-INT-023): the number and size of the steps are a function of the integer span, the
//! wrapper's `N_sub` and the sim key's `dt_macro` only; nothing reads a clock.

use crate::occupant::{Geom, State, StepReal, Stepper};
use crate::payload::SimStateBaseOf;

/// A step count as the float type it divides: `n` converted, exact for `n ≤ 2²⁴` at f32, and `N_sub ≤ L_max` is far
/// below it.
pub trait Count: Sized {
    /// `n` as `Self`.
    fn count(n: u32) -> Self;
}

impl Count for f32 {
    #[inline]
    fn count(n: u32) -> Self {
        n as f32
    }
}

impl Count for f64 {
    #[inline]
    fn count(n: u32) -> Self {
        n as f64
    }
}

/// The float type an occupant advances at: a stepper's ([`StepReal`]), with step counts converted ([`Count`]).
pub trait AdvanceReal: StepReal + Count {}

impl<R: StepReal + Count> AdvanceReal for R {}

/// The per-substep cadence the wrapper passes into the occupant (integrator_contract Part 2a, Cost 1; R-19): the work
/// that runs after every internal step, and the `done` flag that ends the march.
pub trait Cadence<R> {
    /// Runs after each internal step, on the state the step left and the force evaluation at its positions, either of
    /// which it may replace (the projection does).
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>);

    /// Whether the run is done: a terminal has fired, so no further step is taken (Part 1, R-103).
    fn done(&self) -> bool;

    /// The stored state `out` of the finished run, with what this cadence kept written into it. A cadence that keeps
    /// nothing leaves it; the wrapper's writes its monitored fields ([`super::wrapper::monitored`]).
    #[inline]
    fn finish(&self, out: SimStateBaseOf<R>) -> SimStateBaseOf<R> {
        out
    }
}

/// What the wrapper hands the occupant beside the state and the times (integrator_contract Part 3): the sim key's
/// macro-step and the substep count the wrapper's law chose for this macro-step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdvanceParams<R> {
    /// The macro-step `dt_macro`, fixed for the sim key's horizon (Part 3, Part 5).
    pub dt_macro: R,
    /// `N_sub`, the internal steps per macro-step (dd_integrator §3.3).
    pub n_sub: u32,
}

/// The occupant's answer: the state it reached, the force evaluation at its positions, and the internal steps it took,
/// which the wrapper cannot verify and so takes as reported (Part 2a, Cost 2; REQ-INT-022).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Advanced<R> {
    /// The state at `t_target`, or where the cadence's `done` stopped the march.
    pub state: State<R>,
    /// The force evaluation at `state`'s positions.
    pub geom: Geom<R>,
    /// The internal steps taken, each followed by one call of the cadence.
    pub substeps: u32,
}

/// The occupant seam (integrator_contract Part 2a; R-19).
pub trait Advance {
    /// Advances `state`, whose force evaluation is `geom`, from the macro-step grid's `t_now` to its `t_target`, calling
    /// `cadence` after every internal step and stopping early once it is done.
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R>;
}

/// A stepper's `ADVANCE`: the fixed `N_sub` `STEP` loop (integrator_contract Part 1, Part 2a). Over a span of
/// `t_target − t_now` macro-steps (one, as the wrapper calls it; none where `t_target` is not after `t_now`), it takes
/// `span · N_sub` steps of `dt_macro / N_sub`, the cadence after each, while the cadence is not done.
impl<S: Stepper> Advance for S {
    #[inline]
    fn advance<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> Advanced<R> {
        // The guard is written out, not as `t_target.saturating_sub(t_now)` as the lint suggests: rust-gpu
        // 0.10.0-alpha.1, the backend the kernel builds with (R-169), leaves the `saturating_sub` intrinsic
        // unimplemented, a zombie that fails the SPIR-V build where it is reached (its `builder/intrinsics.rs`).
        #[allow(clippy::implicit_saturating_sub)]
        let span = if t_target > t_now {
            t_target - t_now
        } else {
            0
        };
        let steps = span * params.n_sub;
        let dt = params.dt_macro / R::count(params.n_sub);
        let mut s = *state;
        let mut g = *geom;
        let mut taken = 0;
        while taken < steps && !cadence.done() {
            let (next, next_geom) = S::step(&s, dt, &g);
            s = next;
            g = next_geom;
            cadence.after_step(&mut s, &mut g);
            taken += 1;
        }
        Advanced {
            state: s,
            geom: g,
            substeps: taken,
        }
    }
}
