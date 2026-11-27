//! Clamp: the edge system, the nearest point of the primary range (chart_reference §5.4, §2.2 (v) and (vi)).
//!
//! Two clamps live here. [`to_range`] projects a coordinate onto its range, for the edges whose clamp is the axis
//! itself (the invariant `t < 0` row, the Euclid plane's `m`, a time axis before 0). [`region`] and
//! [`below_floor`] classify a pixel of an invariant axis pair: where the construction is infeasible with `K* > 0` the
//! edge is the unfold's; with `K* ≤ 0` it is the clamp below the floor, whose momenta are the construction's own
//! infeasible arm, the rigid rotation at the pixel's own `L_z` (`a = 0`, no sheet input). Don't build it as a rest
//! start or as a clamp of `E` at fixed `L_z`: the first drops the spin, the second moves it.

use super::{Count, Kind, Resolved, Sheet};
use kernel::decode::invariant::infeasible;

/// An axis's range: `[lo, hi]`, either end open.
pub type Range = (Option<f64>, Option<f64>);

/// `x` projected onto `range`.
pub fn axis(x: f64, range: Range) -> f64 {
    let x = range.0.map_or(x, |lo| x.max(lo));
    range.1.map_or(x, |hi| x.min(hi))
}

/// The pixel at `at` under `kind`, each bounded axis of `ranges` projected onto its range: [`Kind::Clamp`] moves a
/// pixel that lies outside, [`Kind::Blank`] blanks it, anything else leaves it as written.
pub fn to_range(at: [f64; 2], ranges: [Range; 2], kind: Kind) -> Resolved {
    let moved = [axis(at[0], ranges[0]), axis(at[1], ranges[1])];
    if moved == at {
        return Resolved::own(at);
    }
    match kind {
        Kind::Clamp => Resolved {
            at: moved,
            sheet: Sheet::One,
            applied: Some(Kind::Clamp),
            count: Count::Clamped,
        },
        Kind::Blank => Resolved::blank(at),
        _ => Resolved::own(at),
    }
}

/// Where a pixel's `(L_z, K*)` falls against the construction's feasibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    /// Feasible: the construction's own pixel.
    Feasible,
    /// Infeasible with `K* > 0`: `|L_z|` past `L_max`, the unfold's.
    Unfold,
    /// Below the floor with `L_z ≠ 0`: the rigid rotation at the pixel's own `L_z`.
    Floor,
    /// `K* < 0` with `L_z = 0`: the rest start, the apex.
    Rest,
}

/// The pixel's region, from the computed sign of `K*` together with the construction's exact feasibility test
/// ([`kernel::decode::invariant::infeasible`]), so one input takes one branch at each precision.
pub fn region<R: kernel::decode::invariant::ExactSign>(inertia: R, k_star: R, lz: R) -> Region {
    let zero = R::zero();
    if !infeasible(inertia, k_star, lz) {
        Region::Feasible
    } else if k_star > zero {
        Region::Unfold
    } else if lz == zero {
        Region::Rest
    } else {
        Region::Floor
    }
}

/// A clamped pixel's own system: its energy `E' = U + L_z²/(2I)` (the rest start's `E' = U`) and whether that is
/// `≥ 0`, which the clamp's report counts (an unbound system the row's `E < 0` does not show).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Floor {
    /// The drawn system's own energy.
    pub e_prime: f64,
    /// Whether `E' ≥ 0`.
    pub unbound: bool,
}

/// The drawn system's energy for a pixel whose region is [`Region::Floor`] or [`Region::Rest`]; `None` elsewhere.
pub fn below_floor(inertia: f64, k_star: f64, lz: f64, u_pot: f64) -> Option<Floor> {
    let e_prime = match region(inertia, k_star, lz) {
        Region::Floor => u_pot + lz * lz / (2.0 * inertia),
        Region::Rest => u_pot,
        _ => return None,
    };
    Some(Floor {
        e_prime,
        unbound: e_prime >= 0.0,
    })
}
