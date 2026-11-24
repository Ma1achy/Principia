//! The two drifts as a reader takes them (integrator_contract Part 1; payload §5; render_gui_spec G2,
//! the energy-drift readout): which drift a reader reads, the diagnostics' event filter on the Cartesian one, and the
//! readout's "stopped at event" status for a run that stopped at an event.
//!
//! **The regularised drift is the one read**: the suspect predicates and the drift view read `dE_reg`
//! (`drift_suspect` reads `dE_reg_max`); with regularisation off `dE_reg` holds the Cartesian drift, the two energies
//! being one. **The Cartesian drift is diagnostic only**, and read only from a run that did not stop at an event
//! ([`cartesian_drift_readable`]): at a close approach it is a cancellation of two large terms, wrong exactly where a run
//! stopped at one.
//!
//! **A run that stopped at an event draws in the drift view as now**, its readout saying "stopped at event"
//! ([`DriftReadout`], [`STOPPED_AT_EVENT`]). The events are the two terminals that end the march, escape and collision
//! (Part 7): a run whose budget ran out is undetermined, not stopped at an event, and a failed run is untrusted. The plan
//! is unclear here (it names "a run that stopped at an event" without listing the events); chose the two terminals,
//! because they are what ends a march at an event, and a budget's end or a failure is no event. This is the data and
//! the string; the gui's hover readout shows them (the gui crate's).

use super::{sd_state, STATE_COLLISION, STATE_ESCAPE};

/// The energy-drift readout's words for a run that stopped at an event.
pub const STOPPED_AT_EVENT: &str = "stopped at event";

/// What the energy-drift readout says of a sample beside its drift.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriftReadout {
    /// The run stopped at an event (escape or collision): the drift is drawn as now, the readout says so.
    StoppedAtEvent,
    /// Any other run: the drift alone.
    Drift,
}

impl DriftReadout {
    /// The readout's status for a sample whose descriptor is `packed_a`.
    #[inline]
    pub fn of(packed_a: u32) -> Self {
        if stopped_at_event(packed_a) {
            DriftReadout::StoppedAtEvent
        } else {
            DriftReadout::Drift
        }
    }

    /// The status's words, `None` where the readout shows the drift alone.
    #[inline]
    pub fn words(self) -> Option<&'static str> {
        match self {
            DriftReadout::StoppedAtEvent => Some(STOPPED_AT_EVENT),
            DriftReadout::Drift => None,
        }
    }
}

/// Whether the run of the descriptor `packed_a` stopped at an event: its `state` is escape or collision (Part 7).
#[inline]
pub fn stopped_at_event(packed_a: u32) -> bool {
    let s = sd_state(packed_a);
    s == STATE_ESCAPE || s == STATE_COLLISION
}

/// The diagnostics' event filter: whether the Cartesian drift `dE_max` of the descriptor `packed_a`'s run
/// may be read as a diagnostic: never for a run that stopped at an event, and never for a failed one, whose latches
/// hold the failed-state values (payload §1).
#[inline]
pub fn cartesian_drift_readable(packed_a: u32) -> bool {
    !stopped_at_event(packed_a) && !super::sd_is_failed(packed_a)
}
