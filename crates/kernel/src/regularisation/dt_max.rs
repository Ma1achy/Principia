//! `dt_max`, the largest physical step a trajectory's occupant took, and its "no usable step" sentinel
//! (integrator_contract Part 2c § "Rules the new kernel must hold by construction", rule 4 and its `dt_max` definition;
//! Part 2b, the Aarseth–Zare step control's step 8; pitfalls §9; REQ-TOOL-039).
//!
//! An occupant diagnostic, outside the payload: the occupant that forms the physical steps feeds [`DtMax::observe`] one
//! step at a time, and whoever runs it reads [`DtMax::report`]. A step is usable when it is finite; a NaN or `±∞` is
//! skipped, decided on its bits before any comparison ([`IeeeBits::non_finite_bits`]; R-114, R-538;
//! gpu_determinism_note § "The discipline", rule 4), and every finite step, zero, negative or the overflow `2.209e128`
//! at f64, is reported as it stands. The fold is reseeded from the first usable step (rule 4), so no seed value takes
//! part, and a trajectory with no usable step reports [`DtMaxReport::NoUsableStep`], never `0.0`.

use crate::real::IeeeBits;

/// What [`DtMax`] reports.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DtMaxReport<R> {
    /// No step was usable: none was taken, or every one was a NaN or `±∞`. The explicit sentinel, not a number.
    NoUsableStep,
    /// The largest usable step, finite.
    Largest(R),
}

/// The fold of one trajectory's physical steps into its `dt_max`.
#[derive(Clone, Copy, Debug)]
pub struct DtMax<R> {
    /// Whether a usable step has been observed; until one has, `largest` holds no step and is not read.
    usable: bool,
    /// The largest usable step observed, once `usable`: finite.
    largest: R,
}

impl<R: IeeeBits> DtMax<R> {
    /// The fold before any step: no usable step.
    #[inline]
    pub fn new() -> Self {
        DtMax {
            usable: false,
            largest: R::default(),
        }
    }

    /// The fold with the physical step `dt` observed: skipped if it is not finite, tested on its bits before any
    /// comparison; the maximum if it is the first usable step; and otherwise the maximum only where it compares
    /// greater, a comparison of two finite values.
    #[inline]
    pub fn observe(self, dt: R) -> Self {
        if dt.non_finite_bits() {
            self
        } else if !self.usable || dt > self.largest {
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

impl<R: IeeeBits> Default for DtMax<R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
