//! The dispatch-time detector pass (integrator_contract Part 5, "t=0 terminals are real outcomes"; payload §2; dd_
//! generation_root §3.1; chart_reference §0.7; R-60, R-13): at dispatch, before the first step, the terminal detectors
//! run on the decoded IC, and [`t0`] gives the label the sample starts with.
//!
//! **A pure function** of the decode's output, its `(m, r, p)` and its `DEGENERATE` tag, and the sim key's threshold:
//! inputs in, a verdict out ([`T0`]). It fills no buffer and reads no device state; how a starting state reaches the
//! integrator is not decided here.
//!
//! **In order:**
//! 1. **`decode_failed` is the decoder's failure only.** A decode output with a non-finite value, tested on the bits
//!    (gpu_determinism_note rule 4), is `decode_failed` with `detail` 0, "non-finite decode output"; else a decode the
//!    decoder tagged `DEGENERATE` is `decode_failed` with the reason's code ([`Degenerate::detail`]; dd_decoder §2).
//!    The non-finite test comes first: a value that is not a number is the more basic failure, and no tag's code is 0
//!    (`crate::decode::degenerate`).
//! 2. **A valid IC already inside `r_coll` is a real outcome**, `collision` with `t_end_step = 0` and `detail` its pair
//!    or 3 (chart_reference §0.7's `COLLISION_T0(pair)` with `t_event = 0`), by the same test, on the same quantised
//!    separations, as every step's ([`crate::detect::collision::classify`]). A looked-up IC takes this path like any
//!    pixel: lookup does not reject bodies within `r_coll` (R-13).
//! 3. **There is no t = 0 escape** (R-60): the escape rule needs a window of history (R-29), so an IC that is already
//!    escaping is `running`, and is classified when its window completes.

use crate::decode::{Degenerate, Ic};
use crate::detect::collision::{classify, CollisionRadius, Separations};
use crate::driver::substep::Quantise;
use crate::payload::{
    set_detail, set_state, DETAIL_DECODE_FAILED_NON_FINITE_DECODE_OUTPUT, STATE_COLLISION,
    STATE_DECODE_FAILED, STATE_RUNNING,
};
use crate::real::IeeeBits;

/// The label a sample starts with: `state` and `detail`, at step 0, where `t_end_step` is 0 (payload §2: initial 0).
/// `running`, with `detail` 0, which nothing reads, where no t = 0 terminal applies, so the march proceeds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct T0 {
    /// The `state` code: `decode_failed`, `collision` or `running`.
    pub state: u32,
    /// The `detail` code under `state`.
    pub detail: u32,
}

impl T0 {
    /// Whether the label is terminal at step 0: anything but `running`.
    #[inline]
    pub fn terminal(&self) -> bool {
        self.state != STATE_RUNNING
    }

    /// `packed_a` with `state` and `detail` written through the generated descriptor setters, every other bit kept.
    #[inline]
    pub fn packed_a(&self, packed_a: u32) -> u32 {
        set_detail(set_state(packed_a, self.state), self.detail)
    }
}

/// Whether any of `ic`'s masses, positions or momenta is NaN or `±∞`, tested on the bits.
#[inline]
fn non_finite<R: IeeeBits>(ic: &Ic<R>) -> bool {
    let mut bad = false;
    let mut i = 0;
    while i < 3 {
        bad = bad
            || ic.m[i].non_finite_bits()
            || ic.r[i][0].non_finite_bits()
            || ic.r[i][1].non_finite_bits()
            || ic.p[i][0].non_finite_bits()
            || ic.p[i][1].non_finite_bits();
        i += 1;
    }
    bad
}

/// The dispatch-time label of the decoded IC `ic`, tagged `degenerate` by the decoder or not, under the threshold
/// `radius` (the module's note).
#[inline]
pub fn t0<R: Quantise + IeeeBits>(
    ic: &Ic<R>,
    degenerate: Option<Degenerate>,
    radius: &CollisionRadius,
) -> T0 {
    let collision = classify(&Separations::of(&ic.r), radius);
    if non_finite(ic) {
        T0 {
            state: STATE_DECODE_FAILED,
            detail: DETAIL_DECODE_FAILED_NON_FINITE_DECODE_OUTPUT,
        }
    } else if let Some(reason) = degenerate {
        let (state, detail) = reason.label();
        T0 { state, detail }
    } else if collision.fired {
        T0 {
            state: STATE_COLLISION,
            detail: collision.detail,
        }
    } else {
        T0 {
            state: STATE_RUNNING,
            detail: 0,
        }
    }
}
