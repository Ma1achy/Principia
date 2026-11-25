//! Encode under a hold (inverse_encode_contract Part 6, "Under a hold, at step 4"): a view with `η_E`, hold `L_z` or both
//! decodes through its hold `H` after `D`, so step 4 compares against the held decode, and a state off the hold
//! gets a stated outcome. [`enter`] is the entry's half, in the contract's order: the tag test on `C(x)`, then on or off
//! the hold. The class rule on the projection `y`, and the tag test on `y`, are the lookup's (`engine::lookup`), which
//! reads `y`'s classes; the landing test against the held decode is [`super::by_kind::encode_into_held_params`].
//!
//! Every untagged held decode lies on the hold's set, and a tagged one keeps its momenta as decoded, so a tagged state
//! may lie anywhere: it is rejected, never projected.

use crate::chart::LzHold;
use crate::decode::descriptor::describe;
use crate::decode::energy_norm::Outcome;
use crate::decode::hold::{hold_ic, Targets};
use crate::decode::invariant::ExactSign;
use crate::decode::Ic;
use crate::encode::by_kind::invariants;
use crate::payload::PayloadReal;

/// What the hold makes of the canonical state `C(x)` at step 4.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Entry<R> {
    /// No hold applies on this chart: the state goes on as it is.
    Free,
    /// On the hold, to `ε_phys`: the state goes on as it is.
    On,
    /// Off the hold: the lookup projects it, `y = H(C(x))`, and raises `lookup_held` with the gaps `L_z(C(x)) − L*` and
    /// `E(C(x)) − E*`, each 0 where that quantity is not held.
    Projected { y: Ic<R>, lz_gap: R, e_gap: R },
    /// The hold tags the state, `outcome`: rejected. `no_state` is the first reason, no state on the hold at this
    /// configuration (the collision pole, `E* < U`, `2K* < L*²`); else the second, no direction for the hold to choose.
    Tagged { outcome: Outcome, no_state: bool },
}

/// Step 4's entry on the canonical state `canonical` under the holds `t` of a chart with the flags
/// `forbids_energy_normalisation` and `lz_hold`, `norm` the contract's `‖·‖_phys` and `eps_phys` its tolerance.
pub fn enter<R, N>(
    canonical: &Ic<R>,
    t: &Targets<R>,
    forbids_energy_normalisation: bool,
    lz_hold: LzHold,
    norm: N,
    eps_phys: R,
) -> Entry<R>
where
    R: ExactSign + PayloadReal,
    N: Fn(&Ic<R>, &Ic<R>) -> R,
{
    let lz_on = t.lz_star.is_some() & (lz_hold == LzHold::Free);
    let e_on = t.e_star.is_some() & !forbids_energy_normalisation;
    if !(lz_on | e_on) {
        return Entry::Free;
    }
    let (y, outcome) = hold_ic(canonical, t, forbids_energy_normalisation, lz_hold);
    if outcome.tag().is_some() {
        let pole = describe(canonical).r_min_pair_0 == R::zero();
        return Entry::Tagged {
            outcome,
            no_state: pole | (outcome != Outcome::Unscalable),
        };
    }
    if norm(&y, canonical) <= eps_phys {
        return Entry::On;
    }
    let i = invariants(canonical);
    let gap = |on: bool, value: R, target: Option<R>| match target {
        Some(x) if on => value - x,
        _ => R::zero(),
    };
    Entry::Projected {
        y,
        lz_gap: gap(lz_on, i.lz, t.lz_star),
        e_gap: gap(e_on, i.e, t.e_star),
    }
}
