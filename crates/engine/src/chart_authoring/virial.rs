//! The virial axis read by its mapping (design `principia_chart_composition.md` § 3's `Q` row, § 9; chart_reference
//! §2.1): the registry's row is `Q = 2K/|U|` and its scale solve, which is the axis at a held `L_z = 0` and under the
//! linear option. Warped at a held `L_z ≠ 0` the axis stands on the floor (`kernel::chart::physical`), its decode
//! `K = max(Q₀·|U|/2, L_z²/2I) + (Q(t) − Q₀)·|U|/2`, so its coordinate is that decode's inverse,
//! `Q(t) = Q₀ + 2(K − max(Q₀·|U|/2, L_z²/2I))/|U|`, on the state's own `K`, `U`, `L_z` and `I`:
//! - at `Q₀ = 0`, `2(K − L_z²/2I)/|U|`, the excess above the floor, labelled `Q above the floor`;
//! - at `Q₀ > 0`, labelled `Q`: the state's own ratio `2K/|U|` on a column where `Q₀ ≥ Q_min(U) = L_z²/(I·|U|)`, and
//!   that ratio less `Q_min(U) − Q₀` on one where it is not, which is marked.
//!
//! The readout is the state's own ratio on every arm. The row's inverse at a held `L_z ≠ 0` is the chart's decode, the
//! construction at that `L_z`, never the row's scale solve, which moves `L_z` with the momenta.

use kernel::chart::energy::Mapping;
use kernel::chart::physical::{AxisVirial, MixedEnergy};
use kernel::decode::Ic;

use super::registry::{angular_momentum, kinetic, potential};

/// A virial axis's mapping, its asked bottom and the chart's held `L_z`: all its reading needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirialAxis {
    /// The axis's mapping.
    pub mapping: Mapping,
    /// `Q₀ = max(Q_lo, 0)`, the warped axis's bottom.
    pub q0: f64,
    /// The chart's held `L_z`.
    pub held: f64,
}

/// `I = Σ mᵢ‖rᵢ − c‖²` about the centre of mass `c`.
fn inertia(ic: &Ic<f64>) -> f64 {
    let total: f64 = ic.m.iter().sum();
    let c = [0, 1].map(|a| (0..3).map(|j| ic.m[j] * ic.r[j][a]).sum::<f64>() / total);
    (0..3)
        .map(|j| ic.m[j] * ((ic.r[j][0] - c[0]).powi(2) + (ic.r[j][1] - c[1]).powi(2)))
        .sum()
}

/// The state's own floor `L_z²/2I`, the least kinetic energy its spin allows.
pub fn floor(ic: &Ic<f64>) -> f64 {
    let lz = angular_momentum(ic);
    lz * lz / (2.0 * inertia(ic))
}

/// The state's own virial ratio `2K/|U|`.
pub fn ratio(ic: &Ic<f64>) -> f64 {
    2.0 * kinetic(ic) / potential(ic).abs()
}

impl VirialAxis {
    /// The axis of the chart `c`.
    pub fn of<A, S, P: Copy>(c: &MixedEnergy<A, AxisVirial, S, P>) -> Self
    where
        f64: From<P>,
    {
        VirialAxis {
            mapping: c.mapping,
            q0: f64::from(c.bottom).max(0.0),
            held: f64::from(c.held),
        }
    }

    /// Whether the axis stands on the floor: warped at a held `L_z ≠ 0`.
    fn floored(&self) -> bool {
        self.mapping == Mapping::Warped && self.held != 0.0
    }

    /// The axis's coordinate of `ic` (the module's note).
    pub fn forward(&self, ic: &Ic<f64>) -> f64 {
        if !self.floored() {
            return ratio(ic);
        }
        let u = potential(ic).abs();
        self.q0 + 2.0 * (kinetic(ic) - (self.q0 * u / 2.0).max(floor(ic))) / u
    }

    /// The axis's label: `Q above the floor` warped at a held `L_z ≠ 0` with `Q₀ = 0`, else `Q`.
    pub fn label(&self) -> &'static str {
        if self.floored() && self.q0 == 0.0 {
            "Q above the floor"
        } else {
            "Q"
        }
    }

    /// Whether `ic`'s column is marked: `Q₀ > 0` is infeasible there, `Q_min(U) > Q₀`, so its bottom row is the floor.
    // The plan is unclear here: "marked as the floor is" names no existing mark; chose a per-state flag, compared in `K`
    // as the decode's `max` compares, so a marked state is exactly one the floor's side of that `max` decodes.
    pub fn marked(&self, ic: &Ic<f64>) -> bool {
        self.floored() && self.q0 > 0.0 && floor(ic) > self.q0 * potential(ic).abs() / 2.0
    }

    /// The readout: the state's own ratio `2K/|U|`.
    pub fn readout(&self, ic: &Ic<f64>) -> f64 {
        ratio(ic)
    }
}
