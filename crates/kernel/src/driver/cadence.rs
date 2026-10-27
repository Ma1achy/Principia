//! The wrapper's per-substep cadence (integrator_contract Part 1, "Cadence (pinned)"; Part 2a, Cost 1; R-19): what runs
//! after every internal step, inside the occupant's march, as the callback [`WrapperCadence`] the wrapper passes in.
//!
//! **In order, after each `STEP`** (Part 1's loop): the COM projection ([`project`], TASK-M3-05 fills it), the
//! invariant accumulation on the projected state ([`monitor`], TASK-M3-05), and terminal detection, which sets the
//! `done` flag on collision (TASK-M3-09) or escape (TASK-M3-12) and never breaks ([`detect`]). Until those land each hook leaves the
//! state as it is and detects nothing, so a run ends only at its horizon.
//!
//! **Instrumented** (REQ-INT-009, REQ-INT-022): the cadence counts its own invocations
//! ([`WrapperCadence::invocations`]), so a test compares them with the internal steps an occupant took and with the
//! `total_substeps` it reported.

use super::advance::Cadence;
use crate::occupant::{Geom, State};

/// The wrapper's per-substep callback: the hooks in Part 1's order, the run's `done` flag and the count of its calls. A
/// run starts from [`WrapperCadence::default`]: no call made, not done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WrapperCadence {
    /// The calls so far.
    invocations: u32,
    /// Whether a terminal has fired.
    done: bool,
}

impl WrapperCadence {
    /// The calls made so far, one per internal step taken.
    #[inline]
    pub fn invocations(&self) -> u32 {
        self.invocations
    }
}

impl<R> Cadence<R> for WrapperCadence {
    #[inline]
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.invocations += 1;
        project(state, geom);
        monitor(state, geom);
        self.done = detect(state, geom);
    }

    #[inline]
    fn done(&self) -> bool {
        self.done
    }
}

/// The COM projection after each `STEP` (integrator_contract Part 1, "COM projection and invariant monitoring"): the
/// hook TASK-M3-05 fills; until then the state is left as the step left it. A projection that moves the positions
/// recomputes `geom` from them, which the next step reads as its force evaluation.
#[inline]
pub fn project<R>(_state: &mut State<R>, _geom: &mut Geom<R>) {}

/// The invariant accumulation on the projected state (Part 1): the hook TASK-M3-05 fills.
#[inline]
pub fn monitor<R>(_state: &State<R>, _geom: &Geom<R>) {}

/// Terminal detection on the projected state (Part 7; dd_integrator §3.6): whether a terminal fired, which sets the
/// run's `done` flag. The hook TASK-M3-09 fills for collision and TASK-M3-12 for escape; until then nothing fires.
#[inline]
pub fn detect<R>(_state: &State<R>, _geom: &Geom<R>) -> bool {
    false
}
