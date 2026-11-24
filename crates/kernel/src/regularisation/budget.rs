//! The regularised integration floor's step budget and its two counted signals (integrator_contract Part 6;
//! payload §1, §2's `budget_bits`).
//!
//! **The budget is a rate**: a copy's running total of steps since `t = 0`, every stepper call and
//! landing retake counted, is checked at every step against `max(N_min, step_budget_rate · t)`, `t` its physical time
//! since `t = 0` ([`over_allowance`]). The rate is steps per unit physical time per copy and `N_min`
//! protects a close encounter early in the run, before `rate · t` has grown; both are calibrations,
//! proposed by `cargo xtask gate step-budget --propose`. Each regularised occupant (Aarseth–Zare, Heggie, logH) runs the
//! check in its own march and ends it [`crate::driver::advance::Reached::Exhausted`]; the running total crosses its
//! seam calls in the trajectory's carried buffer ([`crate::driver::carried::Carried::steps`]), so a march split into
//! calls spends its budget at the step one call does.
//!
//! **Exhaustion is undetermined, never an outcome**: the copy stops, its `state` as it stood
//! (`running`), and `budget_bits`'s sticky `budget_exhausted` bit is set. A sample with it set is finished: a later
//! dispatch hands it back word for word and never marches it on ([`crate::driver::terminal::finished`]), since its
//! remedy is finer steps as an explicit re-run, and its footprint is never discarded, treated as resolved or split.
//!
//! **The two counted signals** (Part 6): the separation floor (`TINY` bound a separation of the step's
//! state) and the entry-cap hit (the per-step size asked for more than the interval's entry size), each counted once
//! per step the march sizes and reported through [`crate::driver::advance::Cadence::signals`]. They never end the
//! march. [`Signals`] holds the two counts live and writes them into `budget_bits`; `saturated` (descriptor bit 5) is
//! set when either has fired.
//!
//! **The word's layout** (payload §2): bit 0 `budget_exhausted`, bits 1–15
//! the floor hits, bits 16–30 the entry-cap hits, bit 31 reserved. Each count saturates at its width's greatest value,
//! `2¹⁵ − 1`, which then reads "at least that many". The plan is unclear here (it fixes the word, not the split); chose
//! two 15-bit counts, because a count declared `[0, 2¹⁶ − 1]` is the generator's step index, drawn on
//! `[0, horizon_steps]` (`ledger::gen::numeric::step_index`), and a count is no step; the entry-cap count saturates
//! early on a long run all the same (measured on Heggie × RK4: about half of all steps hit the entry cap on
//! dd_integrator §5's tests 2 and 7), so it reads "at least 32767" there, and the floor's are rare. Saturation is exact across a
//! resume: `min(min(a, M) + b, M) = min(a + b, M)`, so a march split at any persist boundary stores the uncut march's
//! word. The sums are written out by hand, not with `saturating_add`, which rust-gpu does not implement.

use crate::driver::advance::Count;
use crate::payload::{
    bb_budget_exhausted, bb_cap_hits, bb_floor_hits, set_budget_exhausted, set_cap_hits,
    set_floor_hits,
};
use crate::real::IeeeBits;

/// The greatest floor-hit count the word holds, its field's all-ones value: `2¹⁵ − 1` (bits 1–15).
#[inline]
pub fn floor_hits_max() -> u32 {
    bb_floor_hits(u32::MAX)
}

/// The greatest entry-cap-hit count the word holds, its field's all-ones value: `2¹⁵ − 1` (bits 16–30).
#[inline]
pub fn cap_hits_max() -> u32 {
    bb_cap_hits(u32::MAX)
}

/// `max(a, b)` of two floats, a NaN `a` kept: the allowance's floor.
#[inline]
fn floored<R: PartialOrd>(x: R, floor: R) -> R {
    if floor > x {
        floor
    } else {
        x
    }
}

/// The step budget for any regularised occupant: whether the running total `steps` exceeds `max(n_min, rate·t)` at
/// physical time `t`. An allowance that is not a number, read on its bits, spends the budget too, so that the march
/// stays bounded (Part 2c rule 3).
#[inline]
pub fn over_allowance<R: Count + IeeeBits + PartialOrd + core::ops::Mul<Output = R> + Copy>(
    steps: u32,
    t: R,
    rate: R,
    n_min: u32,
) -> bool {
    let allowance = floored(rate * t, R::count(n_min));
    R::count(steps) > allowance || allowance.nan_bits()
}

/// `a + b`, at most `max`: written out for rust-gpu (the module's note). `a ≤ max` is not required.
#[inline]
pub fn add_saturating(a: u32, b: u32, max: u32) -> u32 {
    if a >= max || b >= max - a {
        max
    } else {
        a + b
    }
}

/// The two counted signals of one trajectory, live during its march (the module's note): plain counts, saturated
/// only when written ([`Signals::write`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Signals {
    /// Steps on which the separation floor bound.
    pub floor_hits: u32,
    /// Steps whose per-step size asked for more than the entry size.
    pub cap_hits: u32,
}

impl Signals {
    /// No signal yet: a run from its IC.
    #[inline]
    pub const fn none() -> Self {
        Signals {
            floor_hits: 0,
            cap_hits: 0,
        }
    }

    /// The counts a resumed march goes on from: the stored `budget_bits`'s.
    #[inline]
    pub fn reload(budget_bits: u32) -> Self {
        Signals {
            floor_hits: bb_floor_hits(budget_bits),
            cap_hits: bb_cap_hits(budget_bits),
        }
    }

    /// One sized step's signals counted (each at most its field's greatest value, so the live count never wraps).
    #[inline]
    pub fn observe(&mut self, floored: bool, cap_hit: bool) {
        self.floor_hits = add_saturating(self.floor_hits, u32::from(floored), floor_hits_max());
        self.cap_hits = add_saturating(self.cap_hits, u32::from(cap_hit), cap_hits_max());
    }

    /// Whether either signal has fired: the condition for `saturated`.
    #[inline]
    pub fn fired(&self) -> bool {
        self.floor_hits > 0 || self.cap_hits > 0
    }

    /// `budget_bits` with the two counts written, each saturated at its width, the `budget_exhausted` bit kept.
    #[inline]
    pub fn write(&self, budget_bits: u32) -> u32 {
        let floor = if self.floor_hits > floor_hits_max() {
            floor_hits_max()
        } else {
            self.floor_hits
        };
        let cap = if self.cap_hits > cap_hits_max() {
            cap_hits_max()
        } else {
            self.cap_hits
        };
        set_cap_hits(set_floor_hits(budget_bits, floor), cap)
    }
}

/// `budget_bits` with the sticky `budget_exhausted` bit set where `exhausted`, kept as it was otherwise: never cleared.
#[inline]
pub fn latch_exhausted(budget_bits: u32, exhausted: bool) -> u32 {
    set_budget_exhausted(budget_bits, exhausted || bb_budget_exhausted(budget_bits))
}

/// Whether a stored sample's step budget ran out: its `budget_bits`'s sticky bit (payload §2). Such a sample is
/// undetermined: finished, never a physical outcome, and its footprint never resolved (payload §2, "An
/// unusable copy").
#[inline]
pub fn exhausted(budget_bits: u32) -> bool {
    bb_budget_exhausted(budget_bits)
}
