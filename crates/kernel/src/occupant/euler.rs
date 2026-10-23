//! Explicit Euler (dd_integrator §3.2): the debug occupant, order 1, 1 force evaluation, not symplectic; it drifts
//! secularly on purpose, so that "energy-drift view + Euler" tests that the diagnostic plumbing works (integrator_contract
//! Part 2). Reachable only from a debug selection ([`super::Selection::Debug`]; REQ-INT-015).
//!
//! ```text
//! r' = r + dt·v ;   v' = v + dt·a(r)
//! ```
//!
//! on the momenta, `v = p/m`: both updates read the state at the start of the step.

use super::kdk::{drift, kick};
use super::{CapabilityProfile, Geom, State, StepReal, Stepper};

/// Explicit Euler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Euler;

impl Stepper for Euler {
    #[inline]
    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            order: 1,
            force_evals: 1,
            symplectic: false,
            step_reversible: false,
            run_reversible: false,
            owns_time_mapping: false,
        }
    }

    #[inline]
    fn step<R: StepReal>(s: &State<R>, dt: R, geom: &Geom<R>) -> (State<R>, Geom<R>) {
        let r = drift(&s.r, &s.m, dt, &s.p);
        let p = kick(&s.p, &s.m, dt, &geom.accel);
        (State { m: s.m, r, p }, Geom::new(&s.m, &r))
    }
}
