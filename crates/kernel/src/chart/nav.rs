//! Navigation stops at the edge (R-469 P2, A1 to A3, A6; chart_decoder_contract § "The operation table", "Navigation
//! stops at the edge"; REQ-CHART-060): one generic rule over the [`Chart`] trait and `validate`, [`nav_stop`], which
//! no chart writes for itself. The edit path applies it to every navigation edit (REQ-GUI-243).
//!
//! **The centre** (A1). An edit is accepted only while the view's centre passes the chart's `validate`, its layers 2
//! and 3, at the active link parameters ([`Chart::validate_params`]). The centre is the view's centre pixel: on an
//! affine chart `(½, ½)`, where `z(½, ½) = z₀`; on a curved one the centre of the chart's own parameters, with the
//! slice centre's hidden coordinates. The caller names it beside the chart ([`View`]). A centre whose `Φ` fails counts
//! as failing ([`Check::Phi`]): a view that cannot be built at the edited value, or a chart output there that is not
//! finite.
//!
//! **The path** (A3). The edited field moves along the straight line from the accepted value to the requested one,
//! `from + λ·(to − from)` for `λ ∈ [0, 1]`, `to` itself at `λ = 1`, so a component the edit leaves alone keeps its
//! value exactly ([`Field`]). A requested value that passes is taken. One that fails stops the edit at the last valid
//! value along the path, found by bisection on `λ`: each step halves the interval between the two ends' f64 bit
//! patterns read as ordered integers (the f64s of `[0, 1]` in order, fewer than 2⁶²), until the two ends are adjacent
//! f64 values, at most 62 steps, the valid end kept. The stop reports the check its adjacent invalid value fails
//! ([`Check`]): the constraint `validate` names and its layer ([`Constraint::layer`]), or `Φ`. No tolerance is
//! introduced.
//!
//! **A lock** (A2) sets the centre to a chosen point. It has no path: it is accepted if that point passes, and
//! refused, the centre unchanged, if it does not ([`Edit::Lock`]).
//!
//! **Tilt and zoom** are edits like the others. They are anchor-preserving, so the centre they ask for is the one
//! accepted and they pass, unless they come with a centre edit. Nothing here reads which edit it is given.
//!
//! **An accepted value that fails** cannot anchor a bisection (no edit through this stop reaches one): an edit from it
//! is taken where its requested value passes, and refused otherwise, the value unchanged.
//!
//! CPU-side and f64, as `validate` is (R-26): the edit path's, never the kernel's.

use super::{Chart, Constraint, ValidationResult};
use crate::generated::links::LinkParams;

/// A field a navigation edit moves (A3): `z₀` for a pan or a slice on the latent chart, the centre of a curved
/// chart's own parameters, a hidden coordinate's value at the slice centre, a chart parameter, a tilt's angles.
pub trait Field: Copy {
    /// The value at the fraction `lambda` of the straight line from `self` to `to`: `self + λ·(to − self)`, with
    /// `self` itself at `λ = 0` and `to` at `λ = 1`, whatever `to` is (a non-finite `to` would give NaN at `λ = 0`).
    fn along(self, to: Self, lambda: f64) -> Self;
}

impl Field for f64 {
    #[inline]
    fn along(self, to: f64, lambda: f64) -> f64 {
        if lambda == 0.0 {
            self
        } else if lambda == 1.0 {
            to
        } else {
            self + lambda * (to - self)
        }
    }
}

impl<const N: usize> Field for [f64; N] {
    #[inline]
    fn along(self, to: Self, lambda: f64) -> Self {
        core::array::from_fn(|i| self[i].along(to[i], lambda))
    }
}

/// A navigation edit of a field from its accepted value to the one asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Edit<V> {
    /// A pan, slice, zoom, tilt or parameter edit: the straight path from `from` to `to`, stopped at its last valid
    /// value.
    Path { from: V, to: V },
    /// A lock: `to` if it passes, else `from`.
    Lock { from: V, to: V },
}

/// A view at an edited value: the chart and its centre `(u, v)`, the point `validate` reads (A1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View<C> {
    /// The chart, with its slice centre and tilt.
    pub chart: C,
    /// The view's centre: `(½, ½)` on an affine chart, the centre of its own parameters on a curved one.
    pub centre: [f64; 2],
}

/// The check that stopped an edit (A5): what `validate` names, or `Φ`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// The centre fails `validate` on this constraint.
    Failed(Constraint),
    /// The centre's `Φ` fails.
    Phi,
}

impl Check {
    /// The layer of `validate` that stopped the edit, 2 or 3; none where `Φ` failed.
    pub fn layer(self) -> Option<u8> {
        match self {
            Check::Failed(failed) => Some(failed.layer()),
            Check::Phi => None,
        }
    }
}

impl Constraint {
    /// The layer of inverse_encode_contract § "Chart-aware validation" the constraint belongs to: 3 for decode sanity
    /// (a decoded mass positive, the CoM at the origin, the total momentum zero), 2 for every other.
    pub fn layer(self) -> u8 {
        match self {
            Constraint::DecodedMassPositive
            | Constraint::ComAtOrigin
            | Constraint::MomentumZero => 3,
            Constraint::LzEParabola
            | Constraint::KNonNegative
            | Constraint::KAboveLzBound
            | Constraint::MassPositive
            | Constraint::MassSum
            | Constraint::MassBuffer
            | Constraint::ShapeTheta
            | Constraint::ShapePhi
            | Constraint::BurrauNu
            | Constraint::AxisDomain
            | Constraint::BurrauTheta => 2,
        }
    }
}

/// What [`nav_stop`] accepted: the clamped value, its fraction of the path, the bisection steps taken, and the check
/// that stopped it, none where the edit was taken whole.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stop<V> {
    /// The value accepted: the one asked for, the last valid one along the path, or the accepted one, refused.
    pub value: V,
    /// Its fraction `λ` of the path: 1 where taken whole, 0 where refused.
    pub lambda: f64,
    /// The bisection steps taken, at most 62.
    pub steps: u32,
    /// The check the requested value, or the value adjacent past the stop, fails; none where the edit was taken whole.
    pub stopped: Option<Check>,
}

/// What fails at a view's centre (A1): `Φ`, where there is no view or the chart's output there is not finite, else
/// `validate`'s constraint at `p`; none where the centre passes.
fn centre<C>(view: Option<View<C>>, p: &LinkParams<f64>) -> Option<Check>
where
    C: Chart,
    f64: From<C::Param>,
{
    let Some(View {
        chart,
        centre: [u, v],
    }) = view
    else {
        return Some(Check::Phi);
    };
    if !chart.map::<f64>(u, v).z.z.iter().all(|x| x.is_finite()) {
        return Some(Check::Phi);
    }
    match chart.validate_params(u, v, p) {
        ValidationResult::Valid => None,
        ValidationResult::Project { failed, .. }
        | ValidationResult::Clamp { failed, .. }
        | ValidationResult::Reject { failed } => Some(Check::Failed(failed)),
    }
}

/// The navigation stop (the module's note): `edit` applied to the field whose views `view` builds, accepted while the
/// centre passes `validate` at the link parameters `p`, a path stopped at its last valid value and a lock refused.
pub fn nav_stop<V, C, F>(edit: Edit<V>, view: F, p: &LinkParams<f64>) -> Stop<V>
where
    V: Field,
    C: Chart,
    f64: From<C::Param>,
    F: Fn(V) -> Option<View<C>>,
{
    let check = |x: V| centre(view(x), p);
    let (from, to, lock) = match edit {
        Edit::Path { from, to } => (from, to, false),
        Edit::Lock { from, to } => (from, to, true),
    };
    let Some(mut failed) = check(to) else {
        return Stop {
            value: to,
            lambda: 1.0,
            steps: 0,
            stopped: None,
        };
    };
    if lock || check(from).is_some() {
        return Stop {
            value: from,
            lambda: 0.0,
            steps: 0,
            stopped: Some(failed),
        };
    }
    let (mut lo, mut hi, mut steps) = (0u64, 1f64.to_bits(), 0);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        match check(from.along(to, f64::from_bits(mid))) {
            None => lo = mid,
            Some(at) => {
                hi = mid;
                failed = at;
            }
        }
        steps += 1;
    }
    let lambda = f64::from_bits(lo);
    Stop {
        value: from.along(to, lambda),
        lambda,
        steps,
        stopped: Some(failed),
    }
}
