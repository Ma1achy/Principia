//! An axis's warp (design `principia_chart_composition.md` § 4): `t ∈ [0,1]` to the axis's value on `[lo, hi]`, one
//! of three written here, the fourth, the axis's own feasibility warp, being its chart's.
//!
//! | Warp | `x(t)` | `t(x)` | Extension |
//! |---|---|---|---|
//! | linear | `lo + (hi − lo)·t` | `(x − lo)/(hi − lo)` | affine |
//! | power | `lo + (hi − lo)·t^γ`, `γ > 0` | `((x − lo)/(hi − lo))^(1/γ)` | bounded below `t = 0` |
//! | log | `lo·(hi/lo)^t`, `lo, hi > 0` | `ln(x/lo)/ln(hi/lo)` | bounded to `[0, 1]` |
//!
//! **A warp is not lowered as a kind.** Its choice is a word, [`WarpKind`], with its exponent beside it, read per
//! axis, uniform over a dispatch as the shape sphere's map is. The linear arm is [`lerp`] itself, the mixed charts'
//! affine expression, so a chart at its linear warp keeps its bits: don't fold the arms
//! into one formula (power at `γ = 1`, or `lo·(1 − t) + hi·t`), which moves them.
//!
//! The f64 path's `pow` and `ln` are the pinned libm's ([`Real`]).

use spirv_std::num_traits::Float;

use super::mixed::lerp;
use crate::real::Real;

/// Which warp an axis takes, its word the index in declaration order (0 is linear, so a zeroed uniform row or saved
/// word reads as today's chart).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum WarpKind {
    /// `lo + (hi − lo)·t`.
    Linear,
    /// `lo + (hi − lo)·t^γ`.
    Power,
    /// `lo·(hi/lo)^t`.
    Log,
    /// Under the `controls` feature only: the linear warp written `lo·(1 − t) + hi·t`, equal in exact arithmetic and
    /// not in bits, so a control can show the bitwise test catches a linear arm that is not today's expression.
    #[cfg(feature = "controls")]
    ReorderedLinear,
}

impl WarpKind {
    /// The kind whose word is `w`: 1 power, 2 log, any other linear.
    #[inline]
    pub fn from_word(w: u32) -> Self {
        match w {
            1 => WarpKind::Power,
            2 => WarpKind::Log,
            #[cfg(feature = "controls")]
            3 => WarpKind::ReorderedLinear,
            _ => WarpKind::Linear,
        }
    }
}

/// One axis's warp: its kind and, for power, its exponent `γ` (0 for the others).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisWarp<P> {
    /// The warp.
    pub kind: WarpKind,
    /// Power's exponent; 0 for the others.
    pub gamma: P,
}

impl<P: Copy + Default> AxisWarp<P> {
    /// The linear warp.
    #[inline]
    pub fn linear() -> Self {
        AxisWarp {
            kind: WarpKind::Linear,
            gamma: P::default(),
        }
    }

    /// The log warp.
    #[inline]
    pub fn log() -> Self {
        AxisWarp {
            kind: WarpKind::Log,
            gamma: P::default(),
        }
    }
}

impl<P: Copy> AxisWarp<P>
where
    f64: From<P>,
{
    /// The power warp `t^γ`; `None` unless `γ` is finite and positive.
    #[inline]
    pub fn power(gamma: P) -> Option<Self> {
        let g = f64::from(gamma);
        (g.is_finite() && g > 0.0).then_some(AxisWarp {
            kind: WarpKind::Power,
            gamma,
        })
    }

    /// Whether the warp is defined on `[lo, hi]`: log needs both ends positive and apart; the others any finite range.
    pub fn fits(&self, range: [P; 2]) -> bool {
        let [lo, hi] = range.map(f64::from);
        let finite = lo.is_finite() && hi.is_finite();
        match self.kind {
            WarpKind::Log => finite && lo > 0.0 && hi > 0.0 && lo != hi,
            _ => finite,
        }
    }

    /// The `t` the warp is defined at nearest `t`: any `t` for linear, `t ≥ 0` for power, `[0, 1]` for log (the
    /// extensions in the module's note).
    #[inline]
    pub fn bound(&self, t: f64) -> f64 {
        match self.kind {
            WarpKind::Power => t.max(0.0),
            WarpKind::Log => t.clamp(0.0, 1.0),
            _ => t,
        }
    }
}

impl<P: Copy> AxisWarp<P> {
    /// The axis's value at `t` on `range`, `x(t)`.
    #[inline]
    pub fn value<R: Real + Float + From<P>>(&self, range: [R; 2], t: R) -> R {
        let [lo, hi] = range;
        match self.kind {
            WarpKind::Linear => lerp(range, t),
            WarpKind::Power => lo + (hi - lo) * t.power(<R as From<P>>::from(self.gamma)),
            WarpKind::Log => lo * (hi / lo).power(t),
            #[cfg(feature = "controls")]
            WarpKind::ReorderedLinear => lo * (R::one() - t) + hi * t,
        }
    }

    /// The `t` at which the axis's value on `range` is `x`, `t(x)`, the warp's inverse: NaN for power below `lo`'s
    /// side and for log at a non-positive ratio, where no `t` reaches `x`.
    #[inline]
    pub fn position<R: Real + Float + From<P>>(&self, range: [R; 2], x: R) -> R {
        let [lo, hi] = range;
        match self.kind {
            WarpKind::Power => {
                ((x - lo) / (hi - lo)).power(R::one() / <R as From<P>>::from(self.gamma))
            }
            WarpKind::Log => (x / lo).logarithm() / (hi / lo).logarithm(),
            _ => (x - lo) / (hi - lo),
        }
    }

    /// `dx/dt` at `t` on `range`: the axis's line element, what an area weight reads.
    #[inline]
    pub fn rate<R: Real + Float + From<P>>(&self, range: [R; 2], t: R) -> R {
        let [lo, hi] = range;
        match self.kind {
            WarpKind::Power => {
                let g = <R as From<P>>::from(self.gamma);
                (hi - lo) * g * t.power(g - R::one())
            }
            WarpKind::Log => self.value(range, t) * (hi / lo).logarithm(),
            _ => hi - lo,
        }
    }
}
