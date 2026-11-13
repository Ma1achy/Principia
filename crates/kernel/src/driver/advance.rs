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
//! which own their time mapping, implement `ADVANCE` themselves, as [`crate::regularisation::Composed`].
//!
//! **The joint march** (R-974; [`Advance::advance_joint`]): the FTLE shadow's displacement form marches `(x, δ⃗)`
//! together through the occupant's joint step ([`Stepper::joint_step`]), `x` bit-identical to the occupant's own
//! `ADVANCE` and `δ⃗` through the same stages, with a [`JointCadence`] after each internal step. A stepper's is the
//! blanket impl's; an occupant that owns its time mapping has none until its own is built (Heggie × RK4's is
//! TASK-M3-74's): marching a displacement-form shadow beside it fails to build.
//!
//! **Determinism** (REQ-INT-007, REQ-INT-023): the number and size of the steps are a function of the integer span, the
//! wrapper's `N_sub` and the sim key's `dt_macro` only; nothing reads a clock.

use super::substep::{d2_quantised, Quantise};
use crate::generated::constants::{U32_RANGE_END_F32, U32_TOP_F32, U32_TOP_F64};
use crate::occupant::{Geom, State, StepReal, Stepper};
use crate::payload::{PayloadReal, SimStateBaseOf};
use crate::physics::difference::ForceDifference;
use crate::real::IeeeBits;

/// A step count as the float type it divides: `n` converted, exact for `n ≤ 2²⁴` at f32, and `N_sub ≤ L_max` is far
/// below it. And back: a float truncated to a count, as a regularised occupant finds its sync boundary.
pub trait Count: Sized {
    /// `n` as `Self`.
    fn count(n: u32) -> Self;

    /// `self` truncated toward zero to a `u32`, clamped into range in the float type before the cast
    /// (gpu_determinism_note § "The discipline", rule 3: an out-of-range float-to-unsigned conversion is undefined on
    /// the GPU, so the cast never sees one): a NaN, tested on its bits, is sent to `0` first; a value below `0` gives
    /// `0`; one at or above `2³²` gives `u32::MAX`; and one in between its truncation, which the cast then forms. The
    /// result is these selects', the same on every backend, not the CPU cast's saturation. The bounds are the register's
    /// `u32_range_end` at each width (`U32_RANGE_END_F32`, `U32_TOP_F32`, `U32_TOP_F64`).
    fn whole(self) -> u32;
}

impl Count for f32 {
    #[inline]
    fn count(n: u32) -> Self {
        n as f32
    }

    #[inline]
    fn whole(self) -> u32 {
        let bits = self.to_bits();
        let x = f32::from_bits(if bits & 0x7FFF_FFFF > 0x7F80_0000 {
            0
        } else {
            bits
        });
        let above = x >= U32_RANGE_END_F32;
        let x = if x > 0.0 { x } else { 0.0 };
        let x = if x < U32_TOP_F32 { x } else { U32_TOP_F32 };
        if above {
            u32::MAX
        } else {
            x as u32
        }
    }
}

impl Count for f64 {
    #[inline]
    fn count(n: u32) -> Self {
        n as f64
    }

    #[inline]
    fn whole(self) -> u32 {
        let bits = self.to_bits();
        let x = f64::from_bits(if bits & 0x7FFF_FFFF_FFFF_FFFF > 0x7FF0_0000_0000_0000 {
            0
        } else {
            bits
        });
        let x = if x > 0.0 { x } else { 0.0 };
        let x = if x < U32_TOP_F64 { x } else { U32_TOP_F64 };
        x as u32
    }
}

/// A configuration value given at each width, read at this one: a regularised occupant's configuration states each of
/// its values at f32 and at f64, and the occupant reads its own width's, never one converted from the other.
pub trait Width: Sized {
    /// `single` at f32, `double` at f64.
    fn at_width(single: f32, double: f64) -> Self;
}

impl Width for f32 {
    #[inline]
    fn at_width(single: f32, _double: f64) -> Self {
        single
    }
}

impl Width for f64 {
    #[inline]
    fn at_width(_single: f32, double: f64) -> Self {
        double
    }
}

/// The float type an occupant advances at: a stepper's ([`StepReal`]), with step counts converted ([`Count`]), its
/// values' bits read ([`IeeeBits`]), so that a regularised occupant tests its state's finiteness on its bits
/// (integrator_contract Part 2b, the Aarseth–Zare step control's step 7; Part 2c's `dt_max`), and its configuration read
/// at its width ([`Width`]). f32 and f64 are both.
pub trait AdvanceReal: StepReal + Count + IeeeBits + Width {}

impl<R: StepReal + Count + IeeeBits + Width> AdvanceReal for R {}

/// The per-substep cadence the wrapper passes into the occupant (integrator_contract Part 2a, Cost 1; R-19): the work
/// that runs after every internal step, and the `done` flag that ends the march.
pub trait Cadence<R> {
    /// Runs after each internal step, on the state the step left and the force evaluation at its positions, either of
    /// which it may replace (the projection does).
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>);

    /// Whether the run is done: a terminal has fired, so no further step is taken (Part 1, R-103).
    fn done(&self) -> bool;

    /// The position-quantised `d²` of the closest pair at the positions `r`, the state the cadence last saw (its run's
    /// IC, or the state its last [`Cadence::after_step`] left), which the wrapper's `N_sub` bucket reads
    /// (dd_integrator §3.3). A cadence that formed it there returns its own, so the step's one computation serves the
    /// bucket too (integrator_contract Part 7; REQ-EVT-004); one that did not forms it from `r`
    /// ([`d2_quantised`]).
    #[inline]
    fn bucket_d2(&self, r: &[[R; 2]; 3]) -> f32
    where
        R: Quantise,
    {
        d2_quantised(r)
    }

    /// The stored state `out` of the finished run, with what this cadence kept written into it. A cadence that keeps
    /// nothing leaves it; the wrapper's writes its monitored fields ([`super::wrapper::monitored`]).
    #[inline]
    fn finish(&self, out: SimStateBaseOf<R>) -> SimStateBaseOf<R>
    where
        R: PayloadReal,
    {
        out
    }
}

/// The per-substep cadence of a joint march (R-974; [`Advance::advance_joint`]): the work that runs after every internal
/// joint step on the sample's state `x`, its force evaluation and the displacement, and the `done` flag that ends the
/// march.
pub trait JointCadence<R> {
    /// Runs after each internal joint step, on the sample's state `x` the step left, the force evaluation at its
    /// positions, and the displacement `delta`, any of which it may replace (the projections do).
    fn after_step(&mut self, x: &mut State<R>, geom: &mut Geom<R>, delta: &mut State<R>);

    /// Whether the march is done: no further joint step is taken.
    fn done(&self) -> bool;
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

    /// [`Advance::advance`], with how the call ended ([`Reached`]), which the wrapper reads to count the macro-step
    /// completed or to end the run. This default is the stepper's rule: the call reached `t_target` when it took the
    /// `N_sub` steps it was asked for ([`Reached::by_count`]). An occupant whose march can end on a failure, or whose
    /// steps are not `N_sub`'s, reports its own ([`crate::regularisation::Composed`]).
    #[inline]
    fn advance_reaching<R: AdvanceReal, C: Cadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> (Advanced<R>, Reached) {
        let out = Self::advance(state, geom, t_now, t_target, params, cadence);
        let reached = Reached::by_count(out.substeps, params.n_sub);
        (out, reached)
    }

    /// The joint march of the FTLE shadow's displacement form (R-974; the module's note): `state`, whose force
    /// evaluation is `geom`, and the displacement `delta` advanced together from `t_now` to `t_target` with the
    /// force difference `D`, `cadence` after every internal joint step, stopping early once it is done; the sample's
    /// part is [`Advance::advance`]'s, bit for bit, under the same per-step work. Returns the sample's answer and the
    /// displacement. This default is for an occupant with no joint step: it fails to build where it is reached
    /// ([`Advance::NO_JOINT_STEP`]).
    #[inline]
    fn advance_joint<R: AdvanceReal, D: ForceDifference, C: JointCadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        delta: &State<R>,
        _t_now: u32,
        _t_target: u32,
        _params: &AdvanceParams<R>,
        _cadence: &mut C,
    ) -> (Advanced<R>, State<R>) {
        (
            Advanced {
                state: *state,
                geom: *geom,
                substeps: Self::NO_JOINT_STEP,
            },
            *delta,
        )
    }

    /// The build failure of [`Advance::advance_joint`]'s default, read as its step count: evaluated only where an
    /// occupant with no joint step is asked for one, and it fails there (as [`Stepper::FORM`]'s default does).
    const NO_JOINT_STEP: u32 =
        panic!("this occupant has no joint step (Advance::advance_joint; R-974): Heggie × RK4's is TASK-M3-74's");
}

/// How an `ADVANCE` call ended (integrator_contract Part 2a, Part 7): at its target; short of it, the run able to go
/// on; or with the copy's march ended, which latches at the step it fired on and is never run on (Part 2a § "The sync
/// grid is a fixed quantum of physical time (R-565)": "a terminal event (a collision, a non-finite state, the step
/// budget run out, Part 6 and Part 7) still latches at the step it fires on").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reached {
    /// The call reached `t_target`: the macro-step is completed (payload §2, `t_end_step`) and takes its Welford sample
    /// (payload §4).
    Target,
    /// The call stopped short of `t_target`, on the cadence's `done` (a terminal fired, Part 7) or with fewer steps than
    /// asked: the macro-step is not completed.
    Short,
    /// The occupant's state was not finite: the run is `sim_failed` (payload §2), its state the last finite one.
    NonFinite,
    /// The step budget ran out (R-604): the copy stops, undetermined, its `state` as it stood (Part 7; R-603, whose
    /// sticky `budget_exhausted` flag is TASK-M3-41's).
    Exhausted,
}

impl Reached {
    /// The stepper's rule: `Target` where the `substeps` taken are the `n_sub` asked for, else `Short`.
    #[inline]
    pub fn by_count(substeps: u32, n_sub: u32) -> Reached {
        if substeps == n_sub {
            Reached::Target
        } else {
            Reached::Short
        }
    }

    /// Whether the copy's march has ended, never to be run on: [`Reached::NonFinite`] or [`Reached::Exhausted`].
    #[inline]
    pub fn ended(self) -> bool {
        matches!(self, Reached::NonFinite | Reached::Exhausted)
    }
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

    /// The stepper's joint march: [`Advance::advance`]'s loop with [`Stepper::joint_step`] in place of the `STEP`, the
    /// same count and step size, so the sample's part is [`Advance::advance`]'s, bit for bit, under the same per-step
    /// work on `x`.
    #[inline]
    fn advance_joint<R: AdvanceReal, D: ForceDifference, C: JointCadence<R>>(
        state: &State<R>,
        geom: &Geom<R>,
        delta: &State<R>,
        t_now: u32,
        t_target: u32,
        params: &AdvanceParams<R>,
        cadence: &mut C,
    ) -> (Advanced<R>, State<R>) {
        // Written out, as in `advance`, for rust-gpu (R-169).
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
        let mut d = *delta;
        let mut taken = 0;
        while taken < steps && !cadence.done() {
            let (next, next_geom, next_delta) = S::joint_step::<R, D>(&s, &d, dt, &g);
            s = next;
            g = next_geom;
            d = next_delta;
            cadence.after_step(&mut s, &mut g, &mut d);
            taken += 1;
        }
        (
            Advanced {
                state: s,
                geom: g,
                substeps: taken,
            },
            d,
        )
    }
}
