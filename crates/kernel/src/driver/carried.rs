//! The carried regularised state: what a regularised occupant keeps of its own from one `ADVANCE` call to the next,
//! beside the Cartesian `(m, r, p)` (integrator_contract Part 2a; dd_simstate_payload §1, "The carried regularised
//! state").
//!
//! **Why it exists.** An occupant that registers once (Heggie, logH) maps the Cartesian state to its own variables at
//! `t = 0` and never again: rebuilding them from `(m, r, p)` at a later call is a re-registration, a pass through the
//! Levi-Civita inverse and a re-frozen energy, and the march after it is no longer the march registered once
//! (pitfalls §4). So the occupant's own state crosses the seam in this buffer, and the wrapper passes it into every
//! call ([`super::advance::Advance::advance_carrying`]).
//!
//! **Where it lives.** The plan is unclear here (the carried state "in `SimState` or a buffer beside it"); chose a
//! buffer beside `SimState`, one element per trajectory at the trajectory's own index, as the word buffer is, because
//! it leaves the GPU payload's layout, its ledger rows and its schema version as they are, and a trajectory under a
//! stepper pays nothing for numbers it would not use, and Aarseth–Zare uses only the step total and the count. It is
//! part of the one logical payload `f(IC, sim key, t)`: stored, evicted, recomputed and resumed with the `SimState` it
//! sits beside.
//!
//! **Its layout** ([`Carried`]) is occupant-neutral: fourteen floats at the working precision, whose meaning is the
//! occupant's that wrote them, and four `u32`. Heggie's fourteen are `Q₀, Q₁, Q₂`, `P₀, P₁, P₂`, the frozen energy `h`
//! and its interval clock `t` ([`crate::regularisation::heggie::carried_state`]); logH's are `B`, `W` and its interval
//! clock `t`, the first three, the rest 0 ([`crate::regularisation::logh::carried_state`]). 72 bytes at f32, 128 at
//! f64.
//!
//! **The re-registration count** ([`Carried::re_registrations`]; integrator_contract § "The profile gains a field"):
//! the registrations the trajectory's occupant made at a sync boundary after `t = 0`. It is 0 for an occupant that
//! registers once and whose state is carried, and one per boundary marched on from for Aarseth–Zare. The plan is
//! unclear here (a profile field, a per-trajectory count or both, and whether the registration at `t = 0` counts);
//! chose a per-trajectory count here, because the profile is one static row per pair and cannot hold a number that
//! differs between trajectories, and chose not to count the registration at `t = 0`, because every regularised
//! occupant makes that one, and what sets the methods apart is each later pass of a marched state through the chart.

/// One trajectory's carried regularised state (the module's note). `repr(C)`: its fields in this order with no
/// padding, 72 bytes at f32 and 128 at f64.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Carried<R> {
    /// The occupant's own numbers, in the occupant's order; all zero while unregistered.
    pub x: [R; 14],
    /// The running total of the occupant's steps since `t = 0`, every stepper call and landing retake counted, which
    /// its step budget reads (R-604). An occupant that keeps no running total leaves it 0.
    pub steps: u32,
    /// The registrations made at a sync boundary after `t = 0`.
    pub re_registrations: u32,
    /// 1 once an occupant has registered into `x`, 0 before.
    pub registered: u32,
    /// Unused, 0: it keeps the element a multiple of 8 bytes at both widths.
    pub reserved: u32,
}

impl<R: Copy + Default> Carried<R> {
    /// A trajectory's buffer at `t = 0`, before its occupant has registered: every field 0.
    #[inline]
    pub fn unregistered() -> Self {
        Carried {
            x: [R::default(); 14],
            steps: 0,
            re_registrations: 0,
            registered: 0,
            reserved: 0,
        }
    }
}

impl<R: Copy + Default> Default for Carried<R> {
    #[inline]
    fn default() -> Self {
        Self::unregistered()
    }
}
