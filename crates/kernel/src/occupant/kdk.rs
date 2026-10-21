//! The KDK leapfrog (dd_integrator §3.2: order 2, 1 force evaluation, symplectic), and the KDK compositions Yoshida-4
//! and Yoshida-6 are built from:
//!
//! ```text
//! v½ = v + (dt/2)·a(r) ;   r' = r + dt·v½ ;   v' = v½ + (dt/2)·a(r')
//! ```
//!
//! on the momenta, `v = p/m`: a kick `p ← p + h·m·a(r)` and a drift `r ← r + h·p/m`. A composition with drift
//! sequence `d₁ … dₙ` and kick sequence `c₁ … c_{n+1}` is kick `c₁·dt`, drift `d₁·dt`, kick `c₂·dt`, …, drift `dₙ·dt`,
//! kick `c_{n+1}·dt`; the force is evaluated after each drift, `n` evaluations, the first kick reading the evaluation it
//! is given. KDK is `d = [1]`, `c = [1/2, 1/2]`; [`sequence3`] and [`sequence7`] are the three- and seven-drift
//! compositions, written out stage by stage so that every coefficient is read at a constant index.

use super::{CapabilityProfile, Geom, State, StepReal, Stepper};

/// The KDK leapfrog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kdk;

impl Stepper for Kdk {
    #[inline]
    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            order: 2,
            force_evals: 1,
            symplectic: true,
            step_reversible: true,
            run_reversible: false,
            owns_time_mapping: false,
        }
    }

    #[inline]
    fn step<R: StepReal>(s: &State<R>, dt: R, geom: &Geom<R>) -> (State<R>, Geom<R>) {
        let one = R::one();
        let half = one / (one + one);
        sequence1(s, dt, geom, &[one], &[half, half])
    }
}

/// The kick `p ← p + h·m·a`.
#[inline]
pub fn kick<R: StepReal>(p: &[[R; 2]; 3], m: &[R; 3], h: R, a: &[[R; 2]; 3]) -> [[R; 2]; 3] {
    [
        [
            p[0][0] + h * (m[0] * a[0][0]),
            p[0][1] + h * (m[0] * a[0][1]),
        ],
        [
            p[1][0] + h * (m[1] * a[1][0]),
            p[1][1] + h * (m[1] * a[1][1]),
        ],
        [
            p[2][0] + h * (m[2] * a[2][0]),
            p[2][1] + h * (m[2] * a[2][1]),
        ],
    ]
}

/// The drift `r ← r + h·p/m`.
#[inline]
pub fn drift<R: StepReal>(r: &[[R; 2]; 3], m: &[R; 3], h: R, p: &[[R; 2]; 3]) -> [[R; 2]; 3] {
    [
        [
            r[0][0] + h * (p[0][0] / m[0]),
            r[0][1] + h * (p[0][1] / m[0]),
        ],
        [
            r[1][0] + h * (p[1][0] / m[1]),
            r[1][1] + h * (p[1][1] / m[1]),
        ],
        [
            r[2][0] + h * (p[2][0] / m[2]),
            r[2][1] + h * (p[2][1] / m[2]),
        ],
    ]
}

/// One stage of a composition: kick `c·dt` with the evaluation `g`, drift `d·dt`, then the force evaluation at the
/// drifted positions.
#[inline]
fn stage<R: StepReal>(s: &State<R>, g: &Geom<R>, c: R, d: R, dt: R) -> (State<R>, Geom<R>) {
    let p = kick(&s.p, &s.m, c * dt, &g.accel);
    let r = drift(&s.r, &s.m, d * dt, &p);
    (State { m: s.m, r, p }, Geom::new(&s.m, &r))
}

/// The closing kick `c·dt` with the evaluation `g`, which is returned with the state.
#[inline]
fn close<R: StepReal>(s: &State<R>, g: Geom<R>, c: R, dt: R) -> (State<R>, Geom<R>) {
    let p = kick(&s.p, &s.m, c * dt, &g.accel);
    (State { m: s.m, r: s.r, p }, g)
}

/// The one-drift composition `d`, `c`: one force evaluation.
#[inline]
pub fn sequence1<R: StepReal>(
    s: &State<R>,
    dt: R,
    geom: &Geom<R>,
    d: &[R; 1],
    c: &[R; 2],
) -> (State<R>, Geom<R>) {
    let (s, g) = stage(s, geom, c[0], d[0], dt);
    close(&s, g, c[1], dt)
}

/// The three-drift composition `d`, `c`: three force evaluations.
#[inline]
pub fn sequence3<R: StepReal>(
    s: &State<R>,
    dt: R,
    geom: &Geom<R>,
    d: &[R; 3],
    c: &[R; 4],
) -> (State<R>, Geom<R>) {
    let (s, g) = stage(s, geom, c[0], d[0], dt);
    let (s, g) = stage(&s, &g, c[1], d[1], dt);
    let (s, g) = stage(&s, &g, c[2], d[2], dt);
    close(&s, g, c[3], dt)
}

/// The seven-drift composition `d`, `c`: seven force evaluations.
#[inline]
pub fn sequence7<R: StepReal>(
    s: &State<R>,
    dt: R,
    geom: &Geom<R>,
    d: &[R; 7],
    c: &[R; 8],
) -> (State<R>, Geom<R>) {
    let (s, g) = stage(s, geom, c[0], d[0], dt);
    let (s, g) = stage(&s, &g, c[1], d[1], dt);
    let (s, g) = stage(&s, &g, c[2], d[2], dt);
    let (s, g) = stage(&s, &g, c[3], d[3], dt);
    let (s, g) = stage(&s, &g, c[4], d[4], dt);
    let (s, g) = stage(&s, &g, c[5], d[5], dt);
    let (s, g) = stage(&s, &g, c[6], d[6], dt);
    close(&s, g, c[7], dt)
}
