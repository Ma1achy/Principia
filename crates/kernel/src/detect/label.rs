//! The label writer (integrator_contract Part 7; dd_integrator §3.6; payload §2): the one place a march's `state` and
//! its `detail` are decided and written.
//!
//! **Six labels, mutually exclusive** (payload §2): `escape`, `bounded`, `collision`, `running`, `sim_failed` and
//! `decode_failed`. There is no `timeout` state: a march that reaches the horizon `T` with no terminal is `bounded`,
//! and `T` is part of the sim key. There is no substep-cap terminal either: a macro-step whose `N_sub` is capped at
//! `N_max` advances under-resolved with `saturated` set (dd_integrator §3.3), and the march goes on to an outcome.
//! `decode_failed` is the dispatch-time pass's ([`super::t0`]) and never reaches a march.
//!
//! **Precedence, by time** ([`precedence`]): `sim_failed` first, over anything else that fired; then whichever of
//! collision and escape fired first, a same-time tie going to collision; triple ejection is not a label but escape's
//! `detail` 3. With no event, `bounded` where the march reached the horizon and `running` where it stopped short.
//!
//! **Time** is the cadence's ordinal of the internal step whose state the event was read on
//! ([`crate::driver::cadence::WrapperCadence`]): a collision is tested after every step, escape at the occupant's
//! sampling points, each on the state the last step left. Both read the same ordinal for the same state, so a
//! collision and an escape on one state are a tie, and a later step is a later time. The march ends at the first
//! terminal (the `done` flag, Part 1), so in a run the two meet only on one state: the cadence runs the escape test on
//! a sampling point whose own step fired a collision, so that this function, not the order of the cadence's calls,
//! settles the tie.
//!
//! **One write** ([`Label::write`]): `state` and `detail` together, through the generated setters, every other bit of
//! `packed_a` kept. `detail` is the union keyed by `state` (payload §2): a collision's pair or 3, an escape's escaper or
//! 3, `sim_failed`'s cause, `decode_failed`'s reason; under `bounded` and `running` it is 0, which nothing reads.

use crate::payload::{
    set_detail, set_state, STATE_BOUNDED, STATE_COLLISION, STATE_ESCAPE, STATE_RUNNING,
    STATE_SIM_FAILED,
};

/// One detector's terminal: whether it fired, at which step ordinal, and its `detail`. Where not fired, `at` and
/// `detail` are 0, which nothing reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fired {
    /// Whether the event fired.
    pub fired: bool,
    /// The ordinal of the internal step whose state it fired on, 0 for the IC.
    pub at: u32,
    /// Its `detail` under its own `state`.
    pub detail: u32,
}

impl Fired {
    /// Nothing fired.
    #[inline]
    pub fn none() -> Self {
        Fired {
            fired: false,
            at: 0,
            detail: 0,
        }
    }

    /// Fired at the step ordinal `at` with `detail`.
    #[inline]
    pub fn at(at: u32, detail: u32) -> Self {
        Fired {
            fired: true,
            at,
            detail,
        }
    }
}

/// What a march leaves for its label: the non-finite failure (its `detail` the cause, payload §2), the collision and
/// the escape, and whether the march reached the horizon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// `sim_failed`: the state or a monitored quantity went non-finite. Its `at` is not read: it wins at any time.
    pub failed: Fired,
    /// The collision detector's terminal.
    pub collision: Fired,
    /// The escape detector's terminal; its `detail` the escaper or 3, a triple ejection.
    pub escape: Fired,
    /// Whether the march reached the horizon `T`.
    pub horizon: bool,
}

/// A label: `state` and its `detail`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Label {
    /// The `state` code (payload §2).
    pub state: u32,
    /// The `detail` code under `state`.
    pub detail: u32,
}

impl Label {
    /// `packed_a` with `state` and `detail` written, every other bit kept: the one write of the two (the module's note).
    #[inline]
    pub fn write(self, packed_a: u32) -> u32 {
        set_detail(set_state(packed_a, self.state), self.detail)
    }
}

/// The label of a march's events (the module's note): `sim_failed` first; then the earlier of collision and escape, a
/// tie to collision; then `bounded` at the horizon and `running` short of it.
#[inline]
pub fn precedence(e: &Events) -> Label {
    let collision_first = e.collision.fired && (!e.escape.fired || e.collision.at <= e.escape.at);
    if e.failed.fired {
        Label {
            state: STATE_SIM_FAILED,
            detail: e.failed.detail,
        }
    } else if collision_first {
        Label {
            state: STATE_COLLISION,
            detail: e.collision.detail,
        }
    } else if e.escape.fired {
        Label {
            state: STATE_ESCAPE,
            detail: e.escape.detail,
        }
    } else if e.horizon {
        Label {
            state: STATE_BOUNDED,
            detail: 0,
        }
    } else {
        Label {
            state: STATE_RUNNING,
            detail: 0,
        }
    }
}
