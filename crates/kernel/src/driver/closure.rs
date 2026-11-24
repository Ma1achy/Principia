//! The closure registers and their departure gate (payload §1 and § "Why closure is here, at this width,
//! unconditionally"; dd_generation_root §3.4; dd_validation_orbits §6): two registers and one bit, no
//! array and no history.
//!
//! **What they hold.** `closure_min` is the running minimum of the shape-sphere chord `|n̂(t) − n̂(0)|`, the IC's shape
//! `n̂(0)`, so rotation is quotiented out and relative periodic orbits count; `closure_step` is the completed-step count
//! at which it was taken, the period label, an exact u16 as `t_dmin_step` (payload §2). Each is a register in the march,
//! `if d < closure_min { closure_min = d; closure_step = k }`, compared on squares: the sample's `d²` against the square
//! of the stored minimum, `d² < closure_min²`, strict, so a later chord must come in below the stored one to move the
//! step. The stored minimum is `√(d²)` rounded, so its square may differ from the `d²` it was taken from in the last
//! bit, and a chord that recurs to that last bit can move the step or not; a resumed march compares against the same
//! stored bits, so the split and the uninterrupted march agree.
//!
//! **The departure gate.** The chord is trivially 0 at `t = 0`, so the minimum starts only once the shape has
//! departed: once the chord has first exceeded `δ_dep`, the register's `delta_dep`, a chord on the unit sphere, so a
//! pure number and the rule gauge-covariant. Departure is a latched fact, `sample_descriptor` bit 15, sticky:
//! set at the first sample whose chord exceeds `δ_dep`, never cleared. That sample is the first the minimum takes,
//! unconditionally; the samples before it are ignored, however close to `n̂(0)` they come. Both tests are on the
//! squared chord, `d² > δ_dep²` and `d² < closure_min²`, so no `√` decides either; the stored minimum is `√(d²)`. A
//! sample whose `d²` is not finite (tested on its bits: the shape point broke) is no sample: it neither departs nor
//! moves the minimum.
//!
//! **Before departure** the registers hold the minimum of an empty set: `closure_min` `+∞` and `closure_step` 0, the
//! departed bit clear. The plan is unclear here (it gives no stored value for a sample that has not departed); chose
//! `+∞` because `d_min`'s unset value is `+∞`, and a reader tells the case by the departed bit. The march never forms
//! or compares against that `+∞` (gpu_determinism_note § "The discipline", rule 4): before departure the in-march
//! minimum is a finite placeholder, never read, the minimum is decided by the departed flag until the first departed
//! sample takes it, and the stored `+∞` is formed from its bits where the sample has not departed, when it is stored
//! (`IeeeBits::or_pos_inf`, as `driver::latch` forms `d_min`'s). A resumed march reloads that stored `+∞` with the
//! departed bit clear, and so never reads it either.
//!
//! **Parity tier.** The departed bit and `closure_step` follow from the float chord, so they are Tier N/S with
//! `closure_min`, not Tier B (parity contract § "Tier B"; 17 Nov 2026, `plan/DECISIONS-LOG.md`).

use super::welford::chord2;
use crate::real::IeeeBits;
use crate::Real;
use spirv_std::num_traits::Float;

/// The closure registers and the departed bit of one sample (payload §1, §2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Closure<R> {
    /// `closure_min`: the least chord `|n̂(t) − n̂(0)|` since departure. Before it, unread: a fresh sample's holds the
    /// largest finite `R`, a resumed one's the stored `+∞`, and [`Closure::stored_min`] gives the stored `+∞` (the
    /// module's note).
    pub min: R,
    /// `closure_step`: the completed-step count at which [`Closure::min`] was taken; 0 before departure.
    pub step: u32,
    /// The departed bit: whether the chord has exceeded `δ_dep`.
    pub departed: bool,
}

impl<R: Real + Float + IeeeBits> Closure<R> {
    /// A fresh sample's: not departed, the minimum of an empty set (the module's note), its in-march placeholder the
    /// largest finite `R`, never read.
    #[inline]
    pub fn fresh() -> Self {
        Closure {
            min: R::max_value(),
            step: 0,
            departed: false,
        }
    }

    /// The registers after the sample at the shape point `n`, taken at the completed-step count `step`, against the
    /// IC's shape `n_0` and the threshold `delta_dep`: departure latched where the squared chord exceeds `delta_dep²`;
    /// then the first departed sample is the minimum, and after it a sample whose squared chord is below the square of
    /// the minimum. A non-finite squared chord is no sample.
    #[inline]
    #[must_use]
    pub fn sample(self, n: [R; 3], n_0: [R; 3], step: u32, delta_dep: R) -> Self {
        let d2 = chord2(n, n_0);
        let finite = !d2.non_finite_bits();
        let departed = self.departed || (finite && d2 > delta_dep * delta_dep);
        // The minimum held before departure is unread, a resumed march's the stored `+∞`, so the comparand is chosen
        // before any arithmetic: 0 where none is held, the first departed sample taking the minimum unconditionally.
        let held = if self.departed { self.min } else { R::zero() };
        let below = !self.departed || d2 < held * held;
        if departed && finite && below {
            Closure {
                min: Float::sqrt(d2),
                step,
                departed,
            }
        } else {
            Closure { departed, ..self }
        }
    }

    /// `closure_min` as stored: the minimum once departed, `+∞` before, formed from its bits (the module's note).
    #[inline]
    pub fn stored_min(&self) -> R {
        self.min.or_pos_inf(!self.departed)
    }
}
