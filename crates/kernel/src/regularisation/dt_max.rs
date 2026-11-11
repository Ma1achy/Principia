//! `dt_max`, the largest physical step a trajectory's occupant took, and its "no usable step" sentinel
//! (integrator_contract Part 2c § "Rules the new kernel must hold by construction", rule 4 and its `dt_max` definition;
//! pitfalls §9; REQ-TOOL-039).
//!
//! An occupant diagnostic, outside the payload: the occupant that forms the physical steps feeds [`DtMax::observe`] one
//! step at a time, and whoever runs it reads [`DtMax::report`]. A step is usable when it is a number (`dt == dt`); a NaN
//! is skipped, and every other value, the overflow `2.209e128` and `±∞` among them, is reported as it stands. The fold
//! is reseeded from the first usable step (rule 4), so no seed value takes part, and a trajectory with no usable step
//! reports [`DtMaxReport::NoUsableStep`], never `0.0`.

use crate::Real;

/// What [`DtMax`] reports.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DtMaxReport<R> {
    /// No step was usable: none was taken, or every one was a NaN. The explicit sentinel, not a number.
    NoUsableStep,
    /// The largest usable step.
    Largest(R),
}

/// The fold of one trajectory's physical steps into its `dt_max`.
#[derive(Clone, Copy, Debug)]
pub struct DtMax<R> {
    /// Whether a usable step has been observed; until one has, `largest` holds no step and is not read.
    usable: bool,
    /// The largest usable step observed, once `usable`.
    largest: R,
}

impl<R: Real> DtMax<R> {
    /// The fold before any step: no usable step.
    #[inline]
    pub fn new() -> Self {
        DtMax {
            usable: false,
            largest: R::default(),
        }
    }

    /// The fold with the physical step `dt` observed: skipped if it is a NaN, the maximum if it is the first usable
    /// step, and otherwise the maximum only where it compares greater.
    #[inline]
    pub fn observe(self, dt: R) -> Self {
        // `dt == dt` is the contract's test of a number, false of a NaN alone; `Real` carries no `is_nan`.
        #[allow(clippy::eq_op)]
        let is_number = dt == dt;
        if is_number && (!self.usable || dt > self.largest) {
            DtMax {
                usable: true,
                largest: dt,
            }
        } else {
            self
        }
    }

    /// The largest usable step, or the sentinel where there was none.
    #[inline]
    pub fn report(self) -> DtMaxReport<R> {
        if self.usable {
            DtMaxReport::Largest(self.largest)
        } else {
            DtMaxReport::NoUsableStep
        }
    }
}

impl<R: Real> Default for DtMax<R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
