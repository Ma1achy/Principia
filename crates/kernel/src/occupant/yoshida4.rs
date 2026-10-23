//! Yoshida-4 (dd_integrator §3.2: order 4, 3 force evaluations, symplectic): the symmetric composition of three KDK
//! steps, `KDK(w₁·dt) ∘ KDK(w₀·dt) ∘ KDK(w₁·dt)`, with drift sequence `d = [w₁, w₀, w₁]` and kick sequence
//! `c = [w₁/2, (w₀+w₁)/2, (w₀+w₁)/2, w₁/2]` ([`super::kdk::sequence3`]).
//!
//! **The coefficients** (R-501, R-515). *Citation:* Yoshida, H. (1990), "Construction of higher order symplectic
//! integrators", *Phys. Lett. A* 150, 262–268: the fourth-order symmetric triple composition, its weights in closed
//! form, `w₁ = 1 / (2 − 2^{1/3})` and `w₀ = −2^{1/3}·w₁`. The shipped digits are dd_integrator §3.2's:
//!
//! ```text
//! w₁ = 1 / (2 − 2^{1/3}) = 1.3512071919596575    (correctly rounded to f64)
//! w₀ = 1 − 2w₁           = −1.702414383919315     (computed in f64; w₀ + 2w₁ = 1 exactly)
//! ```
//!
//! `w₁` is the constants register's `yoshida4_w1`; `w₀` and the kicks are derived from it in f64 and generated into
//! [`StepperCoefficients`](super::StepperCoefficients) (`YOSHIDA4_*`), which this block reads. The negative middle
//! weight `w₀` does not break reversal at a fixed step: the composition is symmetric, so it is step-reversible
//! (R-500; dd_integrator §5, test 3).

use super::kdk::sequence3;
use super::{CapabilityProfile, Geom, State, StepReal, Stepper};

/// Yoshida's fourth-order composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Yoshida4;

/// The drift sequence `[w₁, w₀, w₁]` at `R`.
#[inline]
pub fn drift<R: StepReal>() -> [R; 3] {
    [R::YOSHIDA4_W1, R::YOSHIDA4_W0, R::YOSHIDA4_W1]
}

/// The kick sequence `[w₁/2, (w₀+w₁)/2, (w₀+w₁)/2, w₁/2]` at `R`.
#[inline]
pub fn kick<R: StepReal>() -> [R; 4] {
    [
        R::YOSHIDA4_KICK_1,
        R::YOSHIDA4_KICK_2,
        R::YOSHIDA4_KICK_2,
        R::YOSHIDA4_KICK_1,
    ]
}

impl Stepper for Yoshida4 {
    #[inline]
    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            order: 4,
            force_evals: 3,
            symplectic: true,
            step_reversible: true,
            run_reversible: false,
            owns_time_mapping: false,
        }
    }

    #[inline]
    fn step<R: StepReal>(s: &State<R>, dt: R, geom: &Geom<R>) -> (State<R>, Geom<R>) {
        sequence3(s, dt, geom, &drift(), &kick())
    }
}
