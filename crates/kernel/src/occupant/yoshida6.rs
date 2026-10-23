//! Yoshida-6 (dd_integrator §3.2: order 6, 7 force evaluations, symplectic): the symmetric composition of seven KDK
//! steps with drift sequence `d = [w₃, w₂, w₁, w₀, w₁, w₂, w₃]` and kick sequence `cᵢ = (d_{i−1} + dᵢ)/2`, `d₀ = d₈ = 0`
//! ([`super::kdk::sequence7`]).
//!
//! **The coefficients** (R-57, R-501). *Citation:* Yoshida, H. (1990), "Construction of higher order symplectic
//! integrators", *Phys. Lett. A* 150, 262–268, Table 1, solution A (`w₁`, `w₂`, `w₃`; `w₀` from the consistency
//! condition). The shipped digits are dd_integrator §3.2's:
//!
//! ```text
//! w₁ = −1.17767998417887
//! w₂ =  0.235573213359357
//! w₃ =  0.784513610477560
//! w₀ =  1 − 2(w₁ + w₂ + w₃) = 1.31518632068391    (computed in f64)
//! ```
//!
//! `w₁`, `w₂` and `w₃` are the constants register's `yoshida6_w1` to `yoshida6_w3`; `w₀` and the kicks are derived from
//! them in f64 and generated into [`StepperCoefficients`](super::StepperCoefficients) (`YOSHIDA6_*`), which this block
//! reads. The transcription is checked by the convergence-order test and the order-condition check (dd_integrator §5,
//! test 1a; REQ-INT-090), as R-501 puts them in place of a copy of the paper. The negative weight is `w₁` (R-57); the
//! composition is symmetric, so it is step-reversible despite it (R-500).

use super::kdk::sequence7;
use super::{CapabilityProfile, Geom, State, StepReal, Stepper};

/// Yoshida's sixth-order composition, solution A.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Yoshida6;

/// The drift sequence `[w₃, w₂, w₁, w₀, w₁, w₂, w₃]` at `R`.
#[inline]
pub fn drift<R: StepReal>() -> [R; 7] {
    [
        R::YOSHIDA6_W3,
        R::YOSHIDA6_W2,
        R::YOSHIDA6_W1,
        R::YOSHIDA6_W0,
        R::YOSHIDA6_W1,
        R::YOSHIDA6_W2,
        R::YOSHIDA6_W3,
    ]
}

/// The kick sequence `[w₃/2, (w₃+w₂)/2, (w₂+w₁)/2, (w₁+w₀)/2, (w₀+w₁)/2, (w₁+w₂)/2, (w₂+w₃)/2, w₃/2]` at `R`.
#[inline]
pub fn kick<R: StepReal>() -> [R; 8] {
    [
        R::YOSHIDA6_KICK_1,
        R::YOSHIDA6_KICK_2,
        R::YOSHIDA6_KICK_3,
        R::YOSHIDA6_KICK_4,
        R::YOSHIDA6_KICK_4,
        R::YOSHIDA6_KICK_3,
        R::YOSHIDA6_KICK_2,
        R::YOSHIDA6_KICK_1,
    ]
}

impl Stepper for Yoshida6 {
    #[inline]
    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            order: 6,
            force_evals: 7,
            symplectic: true,
            step_reversible: true,
            run_reversible: false,
            owns_time_mapping: false,
        }
    }

    #[inline]
    fn step<R: StepReal>(s: &State<R>, dt: R, geom: &Geom<R>) -> (State<R>, Geom<R>) {
        sequence7(s, dt, geom, &drift(), &kick())
    }
}
