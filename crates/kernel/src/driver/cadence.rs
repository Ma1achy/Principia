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
//!
//! **The word** (integrator_contract Part 5, branch-cut crossing detection; payload §3): [`WordCadence`] adds crossing
//! detection to any cadence, after that cadence's own work on the same accepted substep, for a run with a word buffer
//! bound ([`super::wrapper::integrate_word`]). The word buffer stays out of a run without one (REQ-PERF-008's symbolic
//! features off, TASK-M4-05), which is why it is a wrapper around the cadence and not a hook in it.

use super::advance::Cadence;
use crate::generated::links::LinkReal;
use crate::occupant::{Geom, State};
use crate::payload::{sd_last_symbol, set_last_symbol};
use crate::real::IeeeBits;
use crate::word::{state_cuts, CrossingDetector, CrossingHook, WordBuffer};

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

/// A cadence with crossing detection added: after `inner`'s work on each accepted substep, the substep's crossings are
/// appended to the word of the sample at the hook's index ([`CrossingDetector::accept_into`]). The cuts are read from
/// the state `inner` leaves; `d` and the classifier read differences of positions, so the COM projection cannot change
/// them beyond its rounding.
pub struct WordCadence<'a, R, C, B: ?Sized> {
    inner: &'a mut C,
    detector: CrossingDetector<R>,
    hook: CrossingHook,
    words: &'a mut B,
    /// The last crossing's `packed_a`: only `last_symbol` is read from it ([`Self::mirror_last_symbol`]).
    packed_a: u32,
}

impl<'a, R: LinkReal + IeeeBits, C, B: WordBuffer + ?Sized> WordCadence<'a, R, C, B> {
    /// The cadence of a march that starts at positions `start`, whose word is at `index` in `words` and empty.
    #[inline]
    pub fn new(inner: &'a mut C, start: &[[R; 2]; 3], index: usize, words: &'a mut B) -> Self {
        Self {
            inner,
            detector: CrossingDetector::new(state_cuts(start)),
            hook: CrossingHook::new(index),
            words,
            packed_a: 0,
        }
    }

    /// `packed_a` with `last_symbol` as this march's crossings left it, every other bit kept: the wrapper writes the
    /// descriptor at the end of the march, so the mirror reaches it here (payload §3).
    #[inline]
    pub fn mirror_last_symbol(&self, packed_a: u32) -> u32 {
        set_last_symbol(packed_a, sd_last_symbol(self.packed_a))
    }
}

impl<R: LinkReal + IeeeBits, C: Cadence<R>, B: WordBuffer + ?Sized> Cadence<R>
    for WordCadence<'_, R, C, B>
{
    #[inline]
    fn after_step(&mut self, state: &mut State<R>, geom: &mut Geom<R>) {
        self.inner.after_step(state, geom);
        self.packed_a = self.detector.accept_into(
            state_cuts(&state.r),
            &mut self.hook,
            self.words,
            self.packed_a,
        );
    }

    #[inline]
    fn done(&self) -> bool {
        self.inner.done()
    }
}
