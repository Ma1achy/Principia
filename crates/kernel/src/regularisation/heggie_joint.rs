//! The FTLE shadow's joint step on the default occupant, Heggie × RK4 (R-974, RQ-793 (a); R-560; dd_integrator §3.8,
//! "Heggie × RK4's joint step and its displacement", TASK-M3-74's definition, R-72; REQ-VAL-355).
//!
//! **The displacement** ([`HgDisplacement`]) is carried in Heggie's own variables, `δy = (δQ, δP, δt)`, with `δh`, the
//! shadow's frozen energy less the sample's: the shadow is `y + δy` marched under `Γ*` at the energy `h + δh`, as
//! Heggie freezes every trajectory's energy at its registration.
//!
//! - [`start`]: the registration's tangent at the sample's registered state, applied to §3.8's Cartesian offset.
//! - [`joint_step`]: the sample one RK4 step on, the step's bit for bit, and `δy` through the same stages
//!   ([`super::form::rk4_joint`]), each stage's rate difference formed plainly (R-974's (a1)).
//! - [`cartesian`]: `δx = J·δy`, `J` the Jacobian of the return to the Cartesian state at the sample's state, the
//!   displacement the estimator reads.
//! - [`resynchronise`]: `δy ← δy − F(y)·δt/g(y)`, then `δt ← 0`, so the shadow's clock is the sample's.
//! - [`rescale`], [`renormalise`]: `δ = ‖J·δy‖`, `δy ← (δ₀/δ)·δy`, `δh` re-formed; [`renormalise`] resynchronises
//!   first, so at every renormalisation the shadow's elapsed physical time equals the sample's, exactly.
//! - [`separation`]: the finalised read's `δ_current`, from a resynchronised copy.
//!
//! The rescaled displacement never leaves Heggie's variables: no inverse map is taken, so no root of the Levi-Civita
//! map is chosen. No precision-dependent constant is in a branch (Part 2c rule 1): `TINY`, the one excepted floor, is
//! the system's.

use super::az::{add, dot, floored, lmul, ltmul, scale, sub, two, AzReal, V};
use super::form::{rk4_joint, Flow};
use super::heggie::{HgP, HgQ, HgSystem};
use crate::occupant::State;

/// The FTLE shadow's displacement under Heggie: the shadow less the sample in Heggie's variables, and in its frozen
/// energy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HgDisplacement<R> {
    /// `δQᵢ`, and `δt`, the shadow's clock less the sample's.
    pub q: HgQ<R>,
    /// `δPᵢ`.
    pub p: HgP<R>,
    /// `δh`, the shadow's frozen energy less the sample's.
    pub h: R,
}

/// The shadow's system: the sample's at the energy `h + δh`, the sum formed at the working precision.
#[inline]
fn shadow_system<R: AzReal>(system: &HgSystem<R>, d: &HgDisplacement<R>) -> HgSystem<R> {
    HgSystem {
        h: system.h + d.h,
        ..*system
    }
}

/// `δh = E(y + δy) − E(y)`: Heggie's Eq. 6 read from the regularised variables ([`HgSystem::energy`]), at the shadow's
/// state formed plainly and at the sample's.
#[inline]
fn energy_offset<R: AzReal>(
    system: &HgSystem<R>,
    q: &HgQ<R>,
    p: &HgP<R>,
    d: &HgDisplacement<R>,
) -> R {
    let one = R::one();
    let shadow_q = HgSystem::q_add(q, one, &d.q);
    let shadow_p = HgSystem::p_add(p, one, &d.p);
    system.energy(&shadow_q, &shadow_p) - system.energy(q, p)
}

/// The shadow's start (§3.8, "Heggie × RK4"): the Cartesian offset `dx` (§3.8's `‖δ₀‖·u`, positions and Cartesian
/// momenta) carried into Heggie's variables by the registration's tangent at the sample's registered `(q, p)`:
/// `δqᵢ = δrⱼ − δrₖ`, `δpᵢ = (δ(mv)ⱼ − δ(mv)ₖ)/3`, `δQᵢ = L(Qᵢ)ᵀδqᵢ/(2Rᵢ)`, `δPᵢ = 2(L(δQᵢ)ᵀpᵢ + L(Qᵢ)ᵀδpᵢ)`, `Rᵢ` floored at
/// `TINY` where it divides, as the return floors it; `δt = 0` and `δh` the energy offset of the shadow so formed.
#[inline]
pub fn start<R: AzReal>(
    system: &HgSystem<R>,
    q: &HgQ<R>,
    p: &HgP<R>,
    dx: &State<R>,
) -> HgDisplacement<R> {
    let two = two::<R>();
    let third = R::one() / (two + R::one());
    let dqv = [
        sub(dx.r[1], dx.r[2]),
        sub(dx.r[2], dx.r[0]),
        sub(dx.r[0], dx.r[1]),
    ];
    let dpv = [
        scale(third, sub(dx.p[1], dx.p[2])),
        scale(third, sub(dx.p[2], dx.p[0])),
        scale(third, sub(dx.p[0], dx.p[1])),
    ];
    let (_, pv) = system.phys(q, p);
    let one = |qi: V<R>, dqi: V<R>, pi: V<R>, dpi: V<R>| {
        let du = scale(
            R::one() / (two * floored(dot(qi, qi), system.tiny)),
            ltmul(qi, dqi),
        );
        (du, scale(two, add(ltmul(du, pi), ltmul(qi, dpi))))
    };
    let (u0, w0) = one(q.q[0], dqv[0], pv[0], dpv[0]);
    let (u1, w1) = one(q.q[1], dqv[1], pv[1], dpv[1]);
    let (u2, w2) = one(q.q[2], dqv[2], pv[2], dpv[2]);
    let zero = R::default();
    let d = HgDisplacement {
        q: HgQ {
            q: [u0, u1, u2],
            t: zero,
        },
        p: HgP { p: [w0, w1, w2] },
        h: zero,
    };
    HgDisplacement {
        h: energy_offset(system, q, p, &d),
        ..d
    }
}

/// Heggie × RK4's joint step (§3.8): the sample `(q, p)` one step `h` in fictitious time on `system`, RK4's step bit
/// for bit, and the displacement `d` through the same stages, the shadow's rates those of `system` at `h + δh`, each
/// difference formed plainly ([`super::form::rk4_joint`]). `δt`'s rate is `g(y + δy) − g(y)`, `g = dt/dτ`, the time
/// component of that difference; `δh` is carried unchanged.
#[inline]
pub fn joint_step<R: AzReal>(
    system: &HgSystem<R>,
    q: &HgQ<R>,
    p: &HgP<R>,
    d: &HgDisplacement<R>,
    h: R,
) -> (HgQ<R>, HgP<R>, HgDisplacement<R>) {
    let shadow = shadow_system(system, d);
    let ((q1, p1), (dq, dp)) = rk4_joint(system, &shadow, q, p, &d.q, &d.p, h);
    (
        q1,
        p1,
        HgDisplacement {
            q: dq,
            p: dp,
            h: d.h,
        },
    )
}

/// `δx = J·δy` (§3.8): the Jacobian of the return to the Cartesian state ([`HgSystem::image`]) at the sample's `(q, p)`
/// applied to the displacement, `δqᵢ = 2L(Qᵢ)δQᵢ`, `δpᵢ = (δWᵢ − 2pᵢ·δR̃ᵢ)/(2R̃ᵢ)` with `δWᵢ = L(δQᵢ)Pᵢ + L(Qᵢ)δPᵢ`,
/// `R̃ᵢ = max(Rᵢ, TINY)` and `δR̃ᵢ` the floor's own derivative (`2Qᵢ·δQᵢ` where `Rᵢ` is not below `TINY`, `0` where it
/// is), then [`HgSystem::cartesian`]. Linear in `δy`, read at the sample's state alone; `δt` and `δh` are not read.
#[inline]
pub fn cartesian<R: AzReal>(
    system: &HgSystem<R>,
    q: &HgQ<R>,
    p: &HgP<R>,
    d: &HgDisplacement<R>,
) -> State<R> {
    let two = two::<R>();
    let (_, pv) = system.phys(q, p);
    let one = |qi: V<R>, pi: V<R>, dqi: V<R>, dpi: V<R>, pvi: V<R>| {
        let ri = dot(qi, qi);
        let dr = if system.tiny > ri {
            R::default()
        } else {
            two * dot(qi, dqi)
        };
        let dw = add(lmul(dqi, pi), lmul(qi, dpi));
        (
            scale(two, lmul(qi, dqi)),
            scale(
                R::one() / (two * floored(ri, system.tiny)),
                sub(dw, scale(two * dr, pvi)),
            ),
        )
    };
    let (q0, p0) = one(q.q[0], p.p[0], d.q.q[0], d.p.p[0], pv[0]);
    let (q1, p1) = one(q.q[1], p.p[1], d.q.q[1], d.p.p[1], pv[1]);
    let (q2, p2) = one(q.q[2], p.p[2], d.q.q[2], d.p.p[2], pv[2]);
    system.cartesian(&[q0, q1, q2], &[p0, p1, p2])
}

/// The resynchronisation in Heggie's variables (§3.8): `δy ← δy − F(y)·(δt/g(y))`, `F` the sample's rates in `τ`,
/// `g = dt/dτ` their time component, then `δt ← 0` exactly; `δh` is unchanged. To first order it moves the shadow along
/// its flow to the sample's physical time.
#[inline]
pub fn resynchronise<R: AzReal>(
    system: &HgSystem<R>,
    q: &HgQ<R>,
    p: &HgP<R>,
    d: &HgDisplacement<R>,
) -> HgDisplacement<R> {
    let (kq, kp) = system.rates_of(q, p);
    let s = d.q.t / kq.t;
    let moved = HgSystem::q_add(&d.q, -s, &kq);
    HgDisplacement {
        q: HgQ {
            q: moved.q,
            t: R::default(),
        },
        p: HgSystem::p_add(&d.p, -s, &kp),
        h: d.h,
    }
}

/// `‖x‖` over the twelve components `(r, p)`, in the read side's order (each body's `x² + y²`, the positions' sum, the
/// momenta's, then the root of their sum; payload §5's `benettin_delta`).
#[inline]
fn norm12<R: AzReal>(x: &State<R>) -> R {
    let n = |a: V<R>| dot(a, a);
    let r2 = n(x.r[0]) + n(x.r[1]) + n(x.r[2]);
    let p2 = n(x.p[0]) + n(x.p[1]) + n(x.p[2]);
    (r2 + p2).square_root()
}

/// The renormalisation's rescale (§3.8), on a resynchronised displacement: `δ = ‖J·δy‖` ([`cartesian`]), every
/// component of `δy` times `δ₀/δ` (one rounding each), and `δh` re-formed as at the [`start`], the rescaled shadow
/// frozen at its own energy. Returns the rescaled displacement and `δ`, whose `ln(δ/δ₀)` the caller adds to `S`.
#[inline]
pub fn rescale<R: AzReal>(
    system: &HgSystem<R>,
    q: &HgQ<R>,
    p: &HgP<R>,
    d: &HgDisplacement<R>,
    delta_0: R,
) -> (HgDisplacement<R>, R) {
    let delta = norm12(&cartesian(system, q, p, d));
    let c = delta_0 / delta;
    let v = |a: V<R>| scale(c, a);
    let scaled = HgDisplacement {
        q: HgQ {
            q: [v(d.q.q[0]), v(d.q.q[1]), v(d.q.q[2])],
            t: c * d.q.t,
        },
        p: HgP {
            p: [v(d.p.p[0]), v(d.p.p[1]), v(d.p.p[2])],
        },
        h: d.h,
    };
    (
        HgDisplacement {
            h: energy_offset(system, q, p, &scaled),
            ..scaled
        },
        delta,
    )
}

/// The renormalisation (§3.8): [`resynchronise`], then [`rescale`]. At its end `δt = 0`: the shadow's elapsed physical
/// time is the sample's.
#[inline]
pub fn renormalise<R: AzReal>(
    system: &HgSystem<R>,
    q: &HgQ<R>,
    p: &HgP<R>,
    d: &HgDisplacement<R>,
    delta_0: R,
) -> (HgDisplacement<R>, R) {
    rescale(system, q, p, &resynchronise(system, q, p, d), delta_0)
}

/// The finalised read's `δ_current` (§3.8, "A read resynchronises a copy"): `‖J·δy‖` of a resynchronised copy, the
/// carried displacement left as it is.
#[inline]
pub fn separation<R: AzReal>(
    system: &HgSystem<R>,
    q: &HgQ<R>,
    p: &HgP<R>,
    d: &HgDisplacement<R>,
) -> R {
    norm12(&cartesian(system, q, p, &resynchronise(system, q, p, d)))
}
