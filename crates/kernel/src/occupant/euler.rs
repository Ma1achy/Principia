//! Explicit Euler (dd_integrator §3.2): the debug occupant, order 1, 1 force evaluation, not symplectic; it drifts
//! secularly on purpose, so that "energy-drift view + Euler" tests that the diagnostic plumbing works (integrator_contract
//! Part 2). Reachable only from a debug selection ([`super::Selection::Debug`]; REQ-INT-015).
//!
//! ```text
//! r' = r + dt·v ;   v' = v + dt·a(r)
//! ```
//!
//! on the momenta, `v = p/m`: both updates read the state at the start of the step.
//!
//! **The joint step** (R-974; [`Stepper::joint_step`]): the displacement takes the same update, linear in it, the
//! force replaced by the force difference at the step's start, `δr' = δr + dt·δp/m`, `δp' = δp + dt·m·Δa(r, δr)`.

use super::kdk::{drift, kick};
use super::{CapabilityProfile, Geom, State, StepReal, Stepper};
use crate::physics::difference::ForceDifference;
use crate::regularisation::form::Form;

/// Explicit Euler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Euler;

impl Stepper for Euler {
    /// The form under a regularisation (integrator_contract Part 2b; [`crate::regularisation::form`]).
    const FORM: Form = Form::Euler;

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

    #[inline]
    fn joint_step<R: StepReal, D: ForceDifference>(
        s: &State<R>,
        delta: &State<R>,
        dt: R,
        geom: &Geom<R>,
    ) -> (State<R>, Geom<R>, State<R>) {
        let da = D::accel(&s.m, &geom.pair, &delta.r);
        let r = drift(&delta.r, &s.m, dt, &delta.p);
        let p = kick(&delta.p, &s.m, dt, &da);
        let (next, g) = Self::step(s, dt, geom);
        (next, g, State { m: delta.m, r, p })
    }
}
