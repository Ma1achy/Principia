//! RK4 (dd_integrator §3.2): order 4, 4 force evaluations, not symplectic, the validation cross-check occupant (the
//! inspector's adaptive reference is the separate RK45, integrator_contract Part 6, not a wrapper occupant). The
//! standard tableau on the full state `y = (r, p)`, `ẏ = f(y) = (p/m, F(r))`, `F = m·a`:
//!
//! ```text
//! k₁ = f(y);  k₂ = f(y + dt/2·k₁);  k₃ = f(y + dt/2·k₂);  k₄ = f(y + dt·k₃)
//! y' = y + (dt/6)(k₁ + 2k₂ + 2k₃ + k₄)
//! ```
//!
//! `k₁` reads the force evaluation it is given; `k₂`, `k₃` and `k₄` take one each, and the end state the fourth.

use super::{CapabilityProfile, Geom, State, StepReal, Stepper};

/// The classical fourth-order Runge–Kutta method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rk4;

/// Three bodies' 2-vectors.
type Vecs<R> = [[R; 2]; 3];

/// `x + h·k`, by component.
#[inline]
fn axpy<R: StepReal>(x: &Vecs<R>, h: R, k: &Vecs<R>) -> Vecs<R> {
    [
        [x[0][0] + h * k[0][0], x[0][1] + h * k[0][1]],
        [x[1][0] + h * k[1][0], x[1][1] + h * k[1][1]],
        [x[2][0] + h * k[2][0], x[2][1] + h * k[2][1]],
    ]
}

/// `ṙ = p/m`.
#[inline]
fn velocity<R: StepReal>(m: &[R; 3], p: &Vecs<R>) -> Vecs<R> {
    [
        [p[0][0] / m[0], p[0][1] / m[0]],
        [p[1][0] / m[1], p[1][1] / m[1]],
        [p[2][0] / m[2], p[2][1] / m[2]],
    ]
}

/// `ṗ = F = m·a`.
#[inline]
fn force<R: StepReal>(m: &[R; 3], a: &Vecs<R>) -> Vecs<R> {
    [
        [m[0] * a[0][0], m[0] * a[0][1]],
        [m[1] * a[1][0], m[1] * a[1][1]],
        [m[2] * a[2][0], m[2] * a[2][1]],
    ]
}

/// `a + 2b + 2c + d`.
#[inline]
fn sum<R: StepReal>(two: R, a: R, b: R, c: R, d: R) -> R {
    a + two * b + two * c + d
}

/// `k₁ + 2k₂ + 2k₃ + k₄`, by component, each read at a constant index.
#[inline]
fn weighted<R: StepReal>(
    two: R,
    k1: &Vecs<R>,
    k2: &Vecs<R>,
    k3: &Vecs<R>,
    k4: &Vecs<R>,
) -> Vecs<R> {
    [
        [
            sum(two, k1[0][0], k2[0][0], k3[0][0], k4[0][0]),
            sum(two, k1[0][1], k2[0][1], k3[0][1], k4[0][1]),
        ],
        [
            sum(two, k1[1][0], k2[1][0], k3[1][0], k4[1][0]),
            sum(two, k1[1][1], k2[1][1], k3[1][1], k4[1][1]),
        ],
        [
            sum(two, k1[2][0], k2[2][0], k3[2][0], k4[2][0]),
            sum(two, k1[2][1], k2[2][1], k3[2][1], k4[2][1]),
        ],
    ]
}

impl Stepper for Rk4 {
    #[inline]
    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            order: 4,
            force_evals: 4,
            symplectic: false,
            step_reversible: false,
            run_reversible: false,
            owns_time_mapping: false,
        }
    }

    #[inline]
    fn step<R: StepReal>(s: &State<R>, dt: R, geom: &Geom<R>) -> (State<R>, Geom<R>) {
        let m = &s.m;
        let one = R::one();
        let two = one + one;
        let half = dt / two;
        let (k1r, k1p) = (velocity(m, &s.p), force(m, &geom.accel));
        let (r2, p2) = (axpy(&s.r, half, &k1r), axpy(&s.p, half, &k1p));
        let (k2r, k2p) = (velocity(m, &p2), force(m, &Geom::new(m, &r2).accel));
        let (r3, p3) = (axpy(&s.r, half, &k2r), axpy(&s.p, half, &k2p));
        let (k3r, k3p) = (velocity(m, &p3), force(m, &Geom::new(m, &r3).accel));
        let (r4, p4) = (axpy(&s.r, dt, &k3r), axpy(&s.p, dt, &k3p));
        let (k4r, k4p) = (velocity(m, &p4), force(m, &Geom::new(m, &r4).accel));
        let sixth = dt / (two + two + two);
        let r = axpy(&s.r, sixth, &weighted(two, &k1r, &k2r, &k3r, &k4r));
        let p = axpy(&s.p, sixth, &weighted(two, &k1p, &k2p, &k3p, &k4p));
        (State { m: *m, r, p }, Geom::new(m, &r))
    }
}
