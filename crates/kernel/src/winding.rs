//! The frame's winding (`principia_chart_reference.md` §3.3, "The frame's winding"): the net turn, in radians, of
//! `ρ = r₁ − r₀`, the separation of bodies 0 and 1, in the inertial plane, from the IC. It is one of the signed fields
//! the shape sphere's mirror test reads: under `Π` (`y → −y` on every `rᵢ` and `pᵢ`) the run from `Π·x` winds the
//! negation of the run from `x`, bitwise where nonzero and by value where zero. It is not `θ̃`, the shape sphere's
//! unwrapped longitude, which the mirror leaves unchanged (`u` and `v` are mirror-invariant, only `w` turns).
//!
//! **The definition.** From `W = +0` at the IC, each step from the state with separation `ρ_prev` to the next, with
//! `ρ`, adds the turn [`turn`]:
//! - `c = ρ_prev × ρ = ρ_prev,x·ρ_y − ρ_prev,y·ρ_x` and `d = ρ_prev · ρ`, two products and a sum each;
//! - where `c ≠ 0`, `Δ = atan2(c, d)`, in `[−π, π]`, `d`'s zero taken as `+0` (pitfalls §11);
//! - where `c = 0`, of either sign, `Δ = +0`: `ρ` did not turn (`d > 0`), or `ρ_prev` or `ρ` vanished, the bodies 0
//!   and 1 together (`d = 0`), or the step turned `ρ` by exactly `±π` (`d < 0`), the tie, which has no sign of its own.
//!
//! **Why it is odd under `Π`.** `Π` negates both `y`-components. Each product of `c` changes sign exactly, so `c` is
//! negated bitwise where nonzero, and where the two products are equal `c` is `+0` in both runs; `d` is unchanged
//! bitwise. `atan2` is odd in its first operand, so a nonzero `c` gives a negated `Δ`, and a zero `c` gives `+0` in
//! both. A sum of negated terms in the same order is negated, and an exact-zero sum is `+0` in both. So `W(Π·x)` is
//! `−W(x)` bitwise where nonzero and `+0 = W(x)` by value where zero, at every step, with no case excluded: the tie
//! and the negative `x`-axis included. A per-step change of the principal value of `atan2(ρ_y, ρ_x)` is not odd: on the
//! negative `x`-axis, `ρ_y = +0` in both runs, the angle is `+π` in both, and the wrap into `(−π, π]` closes at `+π`
//! (dd_integrator §3.7's `θ̃`), so a mirrored run's deltas differ from the negated ones by `2π` before rounding and in
//! their last bits after; a tie left at `atan2(+0, d < 0) = +π` winds `+π` in both runs.
//!
//! **The cadence** is every state the run's march leaves, each internal step's, the state the wrapper's cadence sees
//! after its projection ([`crate::driver::advance::Cadence::after_step`]), from the IC, and then one closing step to
//! the state the `SimState` stores, which the occupant hands back and can differ from the last state the cadence saw by
//! a rounding. So a run that ended (a collision, an escape) has the winding of its stored, latched state. The sum telescopes: `W` is the
//! net turn of `ρ` over the run, whatever the cadence, provided no step turns `ρ` by `π` or more, which the finest
//! cadence the run has makes least likely. The plan is unclear here (a cadence, and per-step or per-macro-step, is not
//! stated); chose every internal step because the winding is read from the run's states, not stored, so the finest
//! cadence costs nothing and aliases least. Not stored in the `SimState`: it is computed from a run's states.
//!
//! Generic over the kernel's `Real`, no loop and no allocation: [`turn`] and [`Winding::step`] are per-step, as a
//! cadence would call them on the GPU.

use spirv_std::num_traits::Float;

use crate::real::Real;

/// `ρ = r₁ − r₀`, the separation of bodies 0 and 1, the vector whose turn the winding accumulates.
#[inline]
pub fn rho<R: Real + Float>(r: &[[R; 2]; 3]) -> [R; 2] {
    [r[1][0] - r[0][0], r[1][1] - r[0][1]]
}

/// The turn from `prev` to `next` (the module's definition): `atan2(prev × next, prev · next)` where the cross product
/// is nonzero, the dot product's zero taken as `+0`; `+0` where the cross product is zero, of either sign.
#[inline]
pub fn turn<R: Real + Float>(prev: [R; 2], next: [R; 2]) -> R {
    let zero = R::zero();
    let cross = prev[0] * next[1] - prev[1] * next[0];
    let dot = prev[0] * next[0] + prev[1] * next[1];
    let dot = if dot == zero { zero } else { dot };
    if cross == zero {
        zero
    } else {
        Real::arctan2(cross, dot)
    }
}

/// The frame's winding as it accumulates (the module's note): the last separation seen and the sum of the turns since
/// the IC.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Winding<R> {
    /// `ρ` at the last state seen.
    pub prev: [R; 2],
    /// `W`, the sum of the turns, `+0` at the IC.
    pub total: R,
}

impl<R: Real + Float> Winding<R> {
    /// The winding at the IC with positions `r`: `W = +0`.
    #[inline]
    pub fn new(r: &[[R; 2]; 3]) -> Self {
        Winding {
            prev: rho(r),
            total: R::zero(),
        }
    }

    /// One step, to the state with positions `r`: `W += turn(ρ_prev, ρ)`.
    #[inline]
    pub fn step(&mut self, r: &[[R; 2]; 3]) {
        let next = rho(r);
        self.total = self.total + turn(self.prev, next);
        self.prev = next;
    }
}
